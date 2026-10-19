#![cfg(target_os = "linux")]
use evdev::{EventType, RelativeAxisCode};
use wigglecontrol::{
    controller::{Controller, Controls, Mode, Output, Settings},
    delivery::{Batch, Packet, Publisher, State},
    pointer_buttons,
    scroll::{self, Pose},
    touch_gestures::Frame,
    Config, Hand, Point,
};

struct Pipeline {
    controller: Controller,
    publisher: Publisher,
    writer: State,
}
impl Pipeline {
    fn new(cursor: bool) -> Self {
        Self::with_config(cursor, Default::default())
    }
    fn with_config(cursor: bool, scroll: scroll::Config) -> Self {
        let mut controller = Controller::new(Settings {
            right_click: true,
            detail: Some(Default::default()),
            scroll: Some(scroll),
            escape: Some(Default::default()),
            gestures: Config {
                min_confidence: 0.5,
                stale_us: 300_000,
                drag_grace_us: 1_000_000,
                ..Default::default()
            },
            ..Default::default()
        })
        .unwrap();
        let mut publisher =
            Publisher::with_cursor_output(cursor.then_some(Default::default()), 300_000).unwrap();
        let mut writer = State::default();
        writer
            .apply(
                0,
                publisher
                    .packet(&controller.start(), 0, None)
                    .unwrap()
                    .unwrap(),
                2000.0,
                300_000,
            )
            .unwrap();
        Self {
            controller,
            publisher,
            writer,
        }
    }
    fn interpret(&mut self, t: u64, pose: Option<Pose>, x: f64, y: f64) -> Output {
        let escape = pose.map(|p| {
            if p == Pose::Neutral {
                wigglecontrol::escape::Pose::Neutral
            } else {
                wigglecontrol::escape::Pose::Other
            }
        });
        self.interpret_escape(t, pose, x, y, escape)
    }
    fn interpret_escape(
        &mut self,
        t: u64,
        pose: Option<Pose>,
        x: f64,
        y: f64,
        escape: Option<wigglecontrol::escape::Pose>,
    ) -> Output {
        let eligible = pose != Some(Pose::Fist);
        let hand: Hand = serde_json::from_value(serde_json::json!({"id":7,"confidence":0.5,
            "engaged":eligible,"pinch_eligible":eligible,"cursor":{"x":x,"y":y},
            "thumb_tip":{"x":0.4,"y":0.5},"index_tip":{"x":0.5,"y":0.5},"palm_width":0.2,
            "pinch_distance_ratio":0.6}))
        .unwrap();
        self.controller.observe_with_evidence(
            t,
            Frame {
                version: 1,
                source_us: t,
                ambiguous: false,
                hands: vec![hand],
            },
            None,
            Controls {
                escape: escape.map(|pose| wigglecontrol::escape::Evidence {
                    hand_id: 7,
                    source_us: t,
                    pose,
                }),
                middle_pinch: Some(pointer_buttons::Evidence {
                    hand_id: 7,
                    source_us: t,
                    middle_tip: Point { x: 0.5, y: 0.5 },
                    pinch_distance_m: None,
                    pinch_distance_ratio: Some(0.6),
                    onset_allowed: true,
                }),
                scroll: pose.map(|pose| scroll::Evidence {
                    hand_id: 7,
                    source_us: t,
                    pose,
                    wrist: Point { x, y },
                }),
                ..Default::default()
            },
        )
    }
    fn sample(&mut self, t: u64, pose: Option<Pose>, x: f64, y: f64) -> (Output, Packet, Batch) {
        let out = self.interpret(t, pose, x, y);
        let packet = self
            .publisher
            .absolute_packet(&out, t, Some(t), Default::default())
            .unwrap()
            .unwrap();
        let batch = self
            .writer
            .apply(
                t,
                serde_json::from_value(serde_json::to_value(&packet).unwrap()).unwrap(),
                2000.0,
                300_000,
            )
            .unwrap();
        (out, packet, batch)
    }
    fn arm(&mut self) {
        for t in [1, 100_001, 200_001] {
            self.sample(t, Some(Pose::Neutral), 0.5, 0.5);
        }
    }
    fn enter(&mut self) {
        for t in [300_001, 400_001, 500_001, 600_001] {
            self.sample(t, Some(Pose::Fist), 0.5, 0.5);
        }
    }
}

#[test]
fn diagonal_wheel_freezes_pointer_and_returns_without_jumps_with_or_without_reconstruction() {
    for cursor in [false, true] {
        let mut p = Pipeline::new(cursor);
        p.arm();
        p.enter();
        let (out, packet, batch) = p.sample(700_001, Some(Pose::Fist), 0.4, 0.4);
        assert_eq!(out.mode, Mode::Scroll);
        assert!(out.scroll.unwrap().active);
        assert!(matches!(packet, Packet::Frame { absolute: None, .. }));
        assert!(batch
            .mouse
            .iter()
            .any(|e| e.event_type() == EventType::RELATIVE
                && e.code() == RelativeAxisCode::REL_WHEEL.0
                && e.value() < 0));
        assert!(batch
            .mouse
            .iter()
            .any(|e| e.event_type() == EventType::RELATIVE
                && e.code() == RelativeAxisCode::REL_HWHEEL.0
                && e.value() > 0));
        assert!(batch
            .mouse
            .iter()
            .all(|e| e.event_type() != EventType::ABSOLUTE && e.event_type() != EventType::KEY));
        let tick = p.controller.tick(730_001);
        assert!(p
            .publisher
            .absolute_packet(&tick, 730_001, None, Default::default())
            .unwrap()
            .is_none());
        let (out, packet, _) = p.sample(800_001, Some(Pose::Neutral), 0.8, 0.8);
        assert_eq!(out.mode, Mode::Pointer);
        assert_eq!(out.reason, Some("scroll_released"));
        assert!(matches!(
            packet,
            Packet::Transition {
                mode: Mode::Pointer,
                enabled: true,
                ..
            }
        ));
        let (_, packet, _) = p.sample(900_001, Some(Pose::Neutral), 0.8, 0.8);
        let Packet::Frame {
            absolute: Some(point),
            ..
        } = packet
        else {
            panic!("missing anchored resume")
        };
        assert_eq!((point.x, point.y), (0.5, 0.5));
        let (out, _, _) = p.sample(1_000_001, Some(Pose::Neutral), 0.8, 0.8);
        assert!(out.pointer_ready && !out.button_down);
    }
}

#[test]
fn loss_and_owner_changes_leave_scroll_with_neutral_delivery_and_require_open() {
    for failure in 0..4 {
        let mut p = Pipeline::new(true);
        p.arm();
        p.enter();
        let mut out = p.interpret(700_001, Some(Pose::Fist), 0.4, 0.4);
        assert_eq!(out.mode, Mode::Scroll);
        let t = 800_001;
        if failure == 3 {
            out = p.controller.tick(1_000_001); // deadline, no fabricated observation
        } else {
            let hand:Hand=serde_json::from_value(serde_json::json!({"id":if failure==1{8}else{7},"confidence":if failure==2{0.4}else{0.5},
                "engaged":false,"pinch_eligible":false,"cursor":{"x":0.3,"y":0.3},"thumb_tip":{"x":0.4,"y":0.5},"index_tip":{"x":0.5,"y":0.5},"palm_width":0.2})).unwrap();
            out = p.controller.observe_with_evidence(
                t,
                Frame {
                    version: 1,
                    source_us: t,
                    ambiguous: false,
                    hands: if failure == 0 { vec![] } else { vec![hand] },
                },
                None,
                Controls {
                    scroll: Some(scroll::Evidence {
                        hand_id: if failure == 1 { 8 } else { 7 },
                        source_us: t,
                        pose: Pose::Fist,
                        wrist: Point { x: 0.3, y: 0.3 },
                    }),
                    ..Default::default()
                },
            );
        }
        assert_eq!(out.mode, Mode::Pointer);
        assert!(out.transition && !out.scroll.unwrap().captures_pointer());
        assert!(!out.mouse.iter().any(|e| matches!(
            e,
            wigglecontrol::Event::Scroll { .. } | wigglecontrol::Event::ScrollFine { .. }
        )));
    }
}

#[test]
fn default_off_conflict_validation_and_legacy_vertical_serialization_stay_explicit() {
    let mut c = Controller::new(Settings::default()).unwrap();
    assert!(c.start().scroll.is_none());
    assert!(Controller::new(Settings {
        scroll: Some(Default::default()),
        escape: Some(wigglecontrol::escape::Config {
            thumb_clearance_widths: 0.1,
            ..Default::default()
        }),
        ..Default::default()
    })
    .is_err());
    let legacy: wigglecontrol::Event =
        serde_json::from_str(r#"{"type":"scroll","vertical":-2}"#).unwrap();
    assert_eq!(
        serde_json::to_string(&legacy).unwrap(),
        r#"{"type":"scroll","vertical":-2}"#
    );
    let diagonal: wigglecontrol::Event =
        serde_json::from_str(r#"{"type":"scroll","vertical":-2,"horizontal":3}"#).unwrap();
    assert!(matches!(
        diagonal,
        wigglecontrol::Event::Scroll {
            vertical: -2,
            horizontal: 3
        }
    ));
}

#[test]
fn positive_open_flick_keeps_exclusive_wheel_mode_until_decay_or_new_gesture() {
    for interrupt in [false, true] {
        let mut p = Pipeline::with_config(
            true,
            scroll::Config {
                inertia: Some(Default::default()),
                ..Default::default()
            },
        );
        p.arm();
        p.enter();
        for (t, q) in [(650_001, 0.47), (700_001, 0.44), (750_001, 0.41)] {
            p.sample(t, Some(Pose::Fist), q, q);
        }
        let (out, _, _) = p.sample(800_001, Some(Pose::Neutral), 0.41, 0.41);
        assert!(out.scroll.unwrap().inertia);
        assert_eq!(out.mode, Mode::Scroll);
        let mut wheels = 0;
        let mut ended = false;
        for n in 1..=20 {
            let (out, _, batch) = p.sample(
                800_001 + n * 50_000,
                Some(if interrupt && n == 2 {
                    Pose::Other
                } else {
                    Pose::Neutral
                }),
                0.41,
                0.41,
            );
            wheels += batch
                .mouse
                .iter()
                .filter(|e| e.code() == RelativeAxisCode::REL_WHEEL_HI_RES.0)
                .count();
            if out.scroll.as_ref().unwrap().inertia {
                assert!(batch
                    .mouse
                    .iter()
                    .all(|e| e.event_type() != EventType::KEY
                        && e.event_type() != EventType::ABSOLUTE));
            } else {
                assert_eq!(out.mode, Mode::Pointer);
                ended = true;
                break;
            }
        }
        assert!(wheels > 0 && ended);
    }
}

#[test]
fn escape_during_scroll_first_rearms_the_writer_then_emits_one_key() {
    let mut p = Pipeline::new(true);
    p.arm();
    p.enter();
    let mut fired = 0;
    for n in 0..=7 {
        let t = 700_001 + n * 100_000;
        let out = p.interpret_escape(
            t,
            Some(Pose::Other),
            0.5,
            0.5,
            Some(wigglecontrol::escape::Pose::ThumbOut),
        );
        let packet = p
            .publisher
            .absolute_packet(&out, t, Some(t), Default::default())
            .unwrap()
            .unwrap();
        let batch = p.writer.apply(t, packet, 2000.0, 300_000).unwrap();
        fired += usize::from(batch.escape);
        assert_eq!(out.mode, Mode::Pointer);
    }
    assert_eq!(fired, 1);
}

fn pinch_observation(p: &mut Pipeline, t: u64, right: bool) -> Output {
    let hand: Hand = serde_json::from_value(serde_json::json!({
        "id":7,"confidence":0.5,"engaged":true,"pinch_eligible":true,
        "cursor":{"x":0.5,"y":0.5},"thumb_tip":{"x":0.4,"y":0.5},
        "index_tip":{"x":0.5,"y":0.5},"palm_width":0.2,
        "pinch_distance_ratio":if right{0.6}else{0.1}
    }))
    .unwrap();
    p.controller.observe_with_evidence(
        t,
        Frame {
            version: 1,
            source_us: t,
            ambiguous: false,
            hands: vec![hand],
        },
        None,
        Controls {
            middle_pinch: Some(pointer_buttons::Evidence {
                hand_id: 7,
                source_us: t,
                middle_tip: Point { x: 0.5, y: 0.5 },
                pinch_distance_m: None,
                pinch_distance_ratio: Some(if right { 0.1 } else { 0.6 }),
                onset_allowed: true,
            }),
            scroll: Some(scroll::Evidence {
                hand_id: 7,
                source_us: t,
                pose: Pose::Other,
                wrist: Point { x: 0.5, y: 0.5 },
            }),
            escape: Some(wigglecontrol::escape::Evidence {
                hand_id: 7,
                source_us: t,
                pose: wigglecontrol::escape::Pose::Other,
            }),
            ..Default::default()
        },
    )
}

#[test]
fn positive_fist_can_preempt_an_uncommitted_pinch_without_a_click() {
    for right in [false, true] {
        let mut p = Pipeline::new(true);
        p.arm();
        let pending = pinch_observation(&mut p, 250_001, right);
        assert_eq!(pending.pointer_state, "closing");
        assert!(!pending.button_down);
        for t in [300_001, 400_001, 500_001, 600_001] {
            let (out, _, batch) = p.sample(t, Some(Pose::Fist), 0.5, 0.5);
            assert_eq!(
                out.mode,
                Mode::Scroll,
                "uncommitted {} pinch at {t}",
                if right { "right" } else { "left" }
            );
            assert!(!out.button_down);
            assert!(batch.mouse.iter().all(|e| e.event_type() != EventType::KEY));
        }
        let (out, _, batch) = p.sample(700_001, Some(Pose::Fist), 0.4, 0.4);
        assert!(out.scroll.unwrap().active);
        assert!(batch
            .mouse
            .iter()
            .any(|e| e.code() == RelativeAxisCode::REL_WHEEL_HI_RES.0));
    }
}

#[test]
fn an_established_press_is_not_converted_to_scroll_by_a_fist() {
    for right in [false, true] {
        let mut p = Pipeline::new(true);
        p.arm();
        pinch_observation(&mut p, 250_001, right);
        let pressed = pinch_observation(&mut p, 300_001, right);
        assert!(pressed.button_down);
        for t in [400_001, 500_001, 600_001] {
            let out = p.interpret(t, Some(Pose::Fist), 0.5, 0.5);
            assert_eq!(out.mode, Mode::Pointer);
            assert!(out.button_down);
            assert!(!out.scroll.unwrap().captures_pointer());
            assert!(!out
                .mouse
                .iter()
                .any(|e| matches!(e, wigglecontrol::Event::ScrollFine { .. })));
        }
    }
}
