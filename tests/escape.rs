#![cfg(target_os = "linux")]
use evdev::KeyCode;
use wigglecontrol::{
    controller::{Controller, Output, Settings},
    delivery::{Batch, Packet, Publisher, State},
    escape::{self, Pose},
    pointer_buttons,
    touch_gestures::Frame,
    Config, Hand, MouseButton, Point,
};

struct Pipeline {
    controller: Controller,
    publisher: Publisher,
    writer: State,
}
impl Pipeline {
    fn new(enabled: bool) -> Self {
        Self::with_hold(enabled, 700_000)
    }
    fn with_hold(enabled: bool, hold_us: u64) -> Self {
        let mut controller = Controller::new(Settings {
            right_click: true,
            escape: enabled.then_some(escape::Config {
                hold_us,
                ..Default::default()
            }),
            gestures: Config {
                stale_us: 300_000,
                min_confidence: 0.5,
                drag_grace_us: 1_000_000,
                ..Default::default()
            },
            ..Default::default()
        })
        .unwrap();
        let mut publisher = Publisher::default();
        let mut writer = State::default();
        let packet = publisher
            .packet(&controller.start(), 0, None)
            .unwrap()
            .unwrap();
        writer.apply(0, packet, 2000.0, 300_000).unwrap();
        Self {
            controller,
            publisher,
            writer,
        }
    }
    fn interpret(
        &mut self,
        t: u64,
        pose: Option<Pose>,
        button: Option<MouseButton>,
        engaged: bool,
    ) -> Output {
        let hand: Hand = serde_json::from_value(serde_json::json!({"id":7,"confidence":0.99,
            "engaged":engaged,"pinch_eligible":engaged,"cursor":{"x":0.5,"y":0.5},
            "thumb_tip":{"x":0.4,"y":0.5},"index_tip":{"x":0.5,"y":0.5},"palm_width":0.2,
            "pinch_distance_ratio":if button==Some(MouseButton::Left){0.1}else{0.6}}))
        .unwrap();
        let middle = pointer_buttons::Evidence {
            hand_id: 7,
            source_us: t,
            middle_tip: Point { x: 0.5, y: 0.5 },
            pinch_distance_m: None,
            pinch_distance_ratio: Some(if button == Some(MouseButton::Right) {
                0.1
            } else {
                0.6
            }),
            onset_allowed: true,
        };
        self.controller.observe_with_controls(
            t,
            Frame {
                version: 1,
                source_us: t,
                ambiguous: false,
                hands: vec![hand],
            },
            None,
            Some(middle),
            None,
            pose.map(|pose| escape::Evidence {
                hand_id: 7,
                source_us: t,
                pose,
            }),
        )
    }
    fn sample(
        &mut self,
        t: u64,
        pose: Option<Pose>,
        button: Option<MouseButton>,
        engaged: bool,
    ) -> (Output, Batch) {
        let out = self.interpret(t, pose, button, engaged);
        let packet = self
            .publisher
            .absolute_packet(&out, t, Some(t), Default::default())
            .unwrap()
            .unwrap();
        let batch = self.writer.apply(t, packet, 2000.0, 300_000).unwrap();
        (out, batch)
    }
    fn arm(&mut self) {
        for t in [1, 100_001, 200_001] {
            self.sample(t, Some(Pose::Neutral), None, true);
        }
    }
}

#[test]
fn fresh_escape_can_finish_after_pointer_grace_expires_without_extending_the_drag() {
    let mut p = Pipeline::with_hold(true, 1_500_000);
    p.arm();
    p.sample(250_001, Some(Pose::Other), Some(MouseButton::Left), true);
    p.sample(300_001, Some(Pose::Other), Some(MouseButton::Left), true);
    for t in (400_001..1_900_001).step_by(100_000) {
        let (out, batch) = p.sample(t, Some(Pose::ThumbOut), None, false);
        assert!(!out.escape && !batch.escape);
        if t == 1_300_001 {
            assert!(!out.button_down);
            assert!(batch
                .mouse
                .iter()
                .any(|e| e.code() == KeyCode::BTN_LEFT.0 && e.value() == 0));
        }
    }
    assert!(
        p.sample(1_900_001, Some(Pose::ThumbOut), None, false)
            .1
            .escape
    );
}

#[test]
fn escape_cancels_left_or_right_drag_once_and_requires_positive_neutral_rearm() {
    for button in [MouseButton::Left, MouseButton::Right] {
        let mut p = Pipeline::new(true);
        p.arm();
        p.sample(250_001, Some(Pose::Other), Some(button), true);
        let (held, _) = p.sample(300_001, Some(Pose::Other), Some(button), true);
        assert!(held.button_down);
        // In-view alternate geometry while curling cannot refresh drag or arm Esc.
        let (transition, batch) = p.sample(350_001, Some(Pose::Other), None, false);
        let hold = transition.drag_hold.unwrap();
        assert!(hold.in_grace);
        assert_eq!(hold.source_us, 300_001);
        assert_eq!(hold.until_us, 1_300_001);
        assert!(!batch.escape);
        for t in (400_001..1_100_001).step_by(100_000) {
            let (held, batch) = p.sample(t, Some(Pose::ThumbOut), None, false);
            assert!(held.button_down);
            assert!(!batch.escape);
            assert!(batch.mouse.is_empty());
        }
        let (out, batch) = p.sample(1_100_001, Some(Pose::ThumbOut), None, false);
        assert!(out.escape && out.running && out.transition && !out.pointer_ready);
        assert!(!out.button_down);
        assert!(batch.escape);
        let code = if button == MouseButton::Left {
            KeyCode::BTN_LEFT.0
        } else {
            KeyCode::BTN_RIGHT.0
        };
        assert_eq!(
            batch
                .mouse
                .iter()
                .map(|e| (e.code(), e.value()))
                .collect::<Vec<_>>(),
            vec![(code, 0)]
        );
        for t in (1_200_001..=2_000_001).step_by(100_000) {
            assert!(!p.sample(t, Some(Pose::ThumbOut), None, false).1.escape);
        }
        for t in [2_100_001, 2_200_001, 2_300_001] {
            p.sample(t, Some(Pose::Neutral), None, true);
        }
        for t in (2_400_001..3_100_001).step_by(100_000) {
            assert!(!p.sample(t, Some(Pose::ThumbOut), None, false).1.escape);
        }
        assert!(
            p.sample(3_100_001, Some(Pose::ThumbOut), None, false)
                .1
                .escape
        );
    }
}

#[test]
fn missing_evidence_interrupts_hold_and_cannot_rearm_a_still_closed_pose() {
    let mut p = Pipeline::new(true);
    p.arm();
    for t in [300_001, 400_001, 500_001] {
        p.sample(t, Some(Pose::ThumbOut), None, false);
    }
    p.sample(600_001, None, None, false);
    for t in (700_001..=1_700_001).step_by(100_000) {
        let (out, batch) = p.sample(t, Some(Pose::ThumbOut), None, false);
        assert!(!out.escape && !batch.escape);
    }
}

#[test]
fn alternate_ineligible_pose_does_not_bypass_normal_pointer_open_before_press() {
    let mut p = Pipeline::new(true);
    p.arm();
    let (out, _) = p.sample(300_001, Some(Pose::Other), None, false);
    assert!(!out.pointer_ready && !out.escape);
    for t in [400_001, 500_001] {
        let (out, batch) = p.sample(t, Some(Pose::Other), Some(MouseButton::Left), true);
        assert!(!out.pointer_ready && !out.button_down);
        assert!(batch.mouse.iter().all(|e| e.value() != 1));
    }
    assert!(
        p.sample(600_001, Some(Pose::Other), None, true)
            .0
            .pointer_ready
    );
}

#[test]
fn default_off_ignores_pose_evidence_and_late_escape_becomes_neutral_without_a_key() {
    let mut p = Pipeline::new(false);
    p.arm();
    for t in (300_001..=1_200_001).step_by(100_000) {
        assert!(!p.sample(t, Some(Pose::ThumbOut), None, false).1.escape);
    }
    let mut p = Pipeline::new(true);
    p.arm();
    for t in (300_001..1_000_001).step_by(100_000) {
        p.sample(t, Some(Pose::ThumbOut), None, false);
    }
    let out = p.interpret(1_000_001, Some(Pose::ThumbOut), None, false);
    assert!(out.escape);
    let (neutral, discarded) = p.controller.guard_delivery(out, 1_336_168, Some(1_000_001));
    assert!(discarded && neutral.transition && !neutral.escape);
    assert!(matches!(
        p.publisher.packet(&neutral, 1_336_168, None).unwrap(),
        Some(Packet::Transition { .. })
    ));
}

#[test]
fn wrong_source_owner_low_confidence_and_ambiguity_cannot_complete_escape() {
    for case in 0..5 {
        let mut p = Pipeline::new(true);
        p.arm();
        for n in 0..=8 {
            let t = 300_001 + n * 100_000;
            let mut frame:Frame=serde_json::from_value(serde_json::json!({"version":1,"source_us":t,"ambiguous":false,
                "hands":[{"id":7,"confidence":0.99,"engaged":false,"pinch_eligible":false,
                "cursor":{"x":0.5,"y":0.5},"thumb_tip":{"x":0.4,"y":0.5},"index_tip":{"x":0.5,"y":0.5},"palm_width":0.2}]})).unwrap();
            let mut evidence = escape::Evidence {
                hand_id: 7,
                source_us: t,
                pose: Pose::ThumbOut,
            };
            match case {
                0 => evidence.hand_id = 9,
                1 => evidence.source_us -= 1,
                2 => frame.hands[0].confidence = 0.49,
                3 => frame.ambiguous = true,
                _ => frame.hands[0].id = 9,
            }
            let out =
                p.controller
                    .observe_with_controls(t, frame, None, None, None, Some(evidence));
            assert!(!out.escape, "case{case} sample{n}");
        }
    }
}
