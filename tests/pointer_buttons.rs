use wigglecontrol::{pointer_buttons::Evidence, Hand, MouseButton, Point};

fn hand(id: u64, closed: bool) -> Hand {
    Hand {
        id,
        confidence: 0.99,
        engaged: true,
        pinch_eligible: true,
        pinch_onset_allowed: true,
        scrolling: false,
        cursor: Point { x: 0.3, y: 0.6 },
        thumb_tip: Point { x: 0.4, y: 0.5 },
        index_tip: Point {
            x: if closed { 0.42 } else { 0.56 },
            y: 0.5,
        },
        palm_width: 0.2,
        pinch_distance_m: None,
        pinch_distance_ratio: None,
        secondary_pinch: None,
    }
}
fn middle(t: u64, closed: bool) -> Evidence {
    Evidence {
        hand_id: 7,
        source_us: t,
        middle_tip: Point {
            x: if closed { 0.42 } else { 0.56 },
            y: 0.5,
        },
        pinch_distance_m: None,
        pinch_distance_ratio: None,
        onset_allowed: true,
    }
}

#[test]
fn finger_evidence_preserves_confidence_and_rejects_wrong_owner_frame_metric_or_invalid_data() {
    let original = hand(7, false);
    let observed = middle(20, true).middle_hand(&original, 20).unwrap();
    assert_eq!(observed.confidence, original.confidence);
    assert_eq!(observed.engaged, original.engaged);
    assert_eq!(observed.pinch_eligible, original.pinch_eligible);
    assert_eq!(observed.index_tip.x, 0.42);
    assert_eq!(original.index_tip.x, 0.56);
    for case in 0..6 {
        let mut evidence = middle(20, true);
        match case {
            0 => evidence.hand_id = 9,
            1 => evidence.source_us = 19,
            2 => evidence.pinch_distance_m = Some(0.02),
            3 => evidence.pinch_distance_ratio = Some(0.1),
            4 => evidence.middle_tip.x = f64::NAN,
            _ => evidence.middle_tip.y = 1.1,
        }
        assert!(evidence.middle_hand(&original, 20).is_none(), "case{case}");
    }
    let mut ratio = original.clone();
    ratio.pinch_distance_ratio = Some(0.6);
    let mut evidence = middle(20, true);
    evidence.pinch_distance_ratio = Some(0.1);
    assert!(evidence.middle_hand(&ratio, 20).is_some());
    evidence.pinch_distance_ratio = Some(f64::NAN);
    assert!(evidence.middle_hand(&ratio, 20).is_none());
    let mut world = original;
    world.pinch_distance_m = Some(0.08);
    let mut evidence = middle(20, true);
    evidence.pinch_distance_m = Some(0.02);
    assert!(evidence.middle_hand(&world, 20).is_some());
    evidence.pinch_distance_m = Some(-0.01);
    assert!(evidence.middle_hand(&world, 20).is_none());
}

#[cfg(target_os = "linux")]
mod linux {
    use super::*;
    use evdev::{AbsoluteAxisCode, EventType, KeyCode};
    use wigglecontrol::{
        controller::{Controller, Mode, Output, Settings},
        delivery::{Batch, Publisher, State},
        touch_gestures::Frame,
        Event,
    };
    struct Pipeline {
        controller: Controller,
        publisher: Publisher,
        receiver: State,
    }
    impl Pipeline {
        fn new(enabled: bool, grace: bool) -> Self {
            let settings = Settings {
                right_click: enabled,
                gestures: wigglecontrol::Config {
                    drag_grace_us: if grace { 1_000_000 } else { 0 },
                    ..Default::default()
                },
                ..Default::default()
            };
            let mut this = Self {
                controller: Controller::new(settings).unwrap(),
                publisher: Publisher::default(),
                receiver: State::default(),
            };
            let output = this.controller.start();
            this.deliver(0, None, &output);
            this
        }
        fn deliver(&mut self, now: u64, source: Option<u64>, output: &Output) -> Batch {
            let packet = self
                .publisher
                .absolute_packet(output, now, source, Default::default())
                .unwrap();
            packet
                .map(|p| self.receiver.apply(now, p, 2000.0, 150_000).unwrap())
                .unwrap_or_default()
        }
        fn frame(
            &mut self,
            t: u64,
            hands: Vec<Hand>,
            evidence: Option<Evidence>,
        ) -> (Output, Batch) {
            let output = self.controller.observe_with_buttons(
                t,
                Frame {
                    version: 1,
                    source_us: t,
                    ambiguous: false,
                    hands,
                },
                None,
                evidence,
            );
            let batch = self.deliver(t, Some(t), &output);
            (output, batch)
        }
        fn sample(&mut self, t: u64, closed: bool, button: MouseButton) -> (Output, Batch) {
            self.frame(
                t,
                vec![hand(7, closed && button == MouseButton::Left)],
                Some(middle(t, closed && button == MouseButton::Right)),
            )
        }
    }
    fn edges(batch: &Batch) -> Vec<(u16, i32)> {
        batch
            .mouse
            .iter()
            .filter(|e| e.event_type() == EventType::KEY)
            .map(|e| (e.code(), e.value()))
            .collect()
    }
    fn right_press(p: &mut Pipeline) {
        p.sample(1, false, MouseButton::Left);
        p.sample(20_001, true, MouseButton::Right);
        assert_eq!(
            edges(&p.sample(60_001, true, MouseButton::Right).1),
            vec![(KeyCode::BTN_RIGHT.0, 1)]
        );
    }

    #[test]
    fn middle_only_pinch_reaches_absolute_writer_and_cannot_transfer_to_closed_index() {
        let mut p = Pipeline::new(true, false);
        p.sample(1, false, MouseButton::Left);
        p.sample(20_001, true, MouseButton::Right);
        let (held, batch) = p.sample(60_001, true, MouseButton::Right);
        assert_eq!(held.pointer_button, Some(MouseButton::Right));
        assert_eq!(edges(&batch), vec![(KeyCode::BTN_RIGHT.0, 1)]);
        let position = batch
            .mouse
            .iter()
            .position(|e| e.code() == AbsoluteAxisCode::ABS_Y.0)
            .unwrap();
        let press = batch
            .mouse
            .iter()
            .position(|e| e.event_type() == EventType::KEY)
            .unwrap();
        assert!(position < press);
        // Closing index during an established middle drag must not change its button.
        let (held, batch) = p.frame(70_001, vec![hand(7, true)], Some(middle(70_001, true)));
        assert_eq!(held.pointer_button, Some(MouseButton::Right));
        assert!(edges(&batch).is_empty());
        // Releasing middle while index remains closed only releases right.
        assert_eq!(
            edges(
                &p.frame(80_001, vec![hand(7, true)], Some(middle(80_001, false)))
                    .1
            ),
            vec![(KeyCode::BTN_RIGHT.0, 0)]
        );
        assert!(edges(&p.sample(100_001, true, MouseButton::Left).1).is_empty());
        assert!(edges(&p.sample(140_001, true, MouseButton::Left).1).is_empty());
        p.sample(150_001, false, MouseButton::Left);
        p.sample(170_001, true, MouseButton::Left);
        assert_eq!(
            edges(&p.sample(210_001, true, MouseButton::Left).1),
            vec![(KeyCode::BTN_LEFT.0, 1)]
        );
    }

    #[test]
    fn two_pinches_together_are_not_a_new_three_finger_chord() {
        let mut p = Pipeline::new(true, false);
        p.sample(1, false, MouseButton::Left);
        for t in [20_001, 60_001, 100_001] {
            assert!(edges(&p.frame(t, vec![hand(7, true)], Some(middle(t, true))).1).is_empty());
        }
        p.sample(120_001, true, MouseButton::Right);
        assert_eq!(
            edges(&p.sample(160_001, true, MouseButton::Right).1),
            vec![(KeyCode::BTN_RIGHT.0, 1)]
        );
    }

    #[test]
    fn finger_choice_changes_must_confirm_before_pressing() {
        let mut p = Pipeline::new(true, false);
        p.sample(1, false, MouseButton::Left);
        p.sample(20_001, true, MouseButton::Right);
        assert!(edges(&p.sample(60_001, true, MouseButton::Left).1).is_empty());
        assert!(edges(&p.sample(80_001, true, MouseButton::Right).1).is_empty());
        assert_eq!(
            edges(&p.sample(120_001, true, MouseButton::Right).1),
            vec![(KeyCode::BTN_RIGHT.0, 1)]
        );
    }

    #[test]
    fn missing_wrong_owner_or_wrong_frame_middle_evidence_cannot_fall_back_to_left() {
        for case in 0..3 {
            let mut p = Pipeline::new(true, false);
            p.sample(1, false, MouseButton::Left);
            for t in [20_001, 60_001, 100_001] {
                let mut evidence = middle(t, false);
                if case == 1 {
                    evidence.hand_id = 9;
                }
                if case == 2 {
                    evidence.source_us -= 1;
                }
                let (output, batch) =
                    p.frame(t, vec![hand(7, true)], (case != 0).then_some(evidence));
                assert!(!output.button_down, "case{case}");
                assert!(edges(&batch).is_empty(), "case{case}");
            }
        }
    }

    #[test]
    fn right_drag_uses_its_finger_after_dropout_and_missing_middle_is_unknown_not_open() {
        let mut p = Pipeline::new(true, true);
        right_press(&mut p);
        let (lost, batch) = p.frame(80_001, vec![], None);
        assert!(lost.drag_hold.unwrap().in_grace);
        assert!(edges(&batch).is_empty());
        let (unclear, batch) = p.frame(100_001, vec![hand(7, false)], None);
        assert!(unclear.button_down);
        assert_eq!(unclear.pointer_button, Some(MouseButton::Right));
        assert!(edges(&batch).is_empty());
        let (returned, batch) = p.sample(120_001, true, MouseButton::Right);
        assert!(returned.button_down);
        assert!(edges(&batch).is_empty());
        assert_eq!(
            edges(&p.sample(140_001, false, MouseButton::Left).1),
            vec![(KeyCode::BTN_RIGHT.0, 0)]
        );
    }

    #[test]
    fn tick_stop_and_mode_transition_neutralize_the_actual_right_button() {
        for case in 0..3 {
            let mut p = Pipeline::new(true, false);
            right_press(&mut p);
            let (now, output) = match case {
                0 => (210_001, p.controller.tick(210_001)),
                1 => (70_001, p.controller.stop()),
                _ => (70_001, p.controller.set_mode(Mode::Scroll)),
            };
            assert!(output.mouse.contains(&Event::NamedButton {
                button: MouseButton::Right,
                down: false
            }));
            let batch = p.deliver(now, None, &output);
            assert_eq!(edges(&batch), vec![(KeyCode::BTN_RIGHT.0, 0)]);
            assert!(p.receiver.release().mouse.is_empty());
        }
    }

    #[test]
    fn closed_reacquisition_cannot_press_right_and_disabled_config_keeps_left_only() {
        let mut p = Pipeline::new(true, false);
        for t in [1, 60_001, 100_001] {
            assert!(edges(&p.sample(t, true, MouseButton::Right).1).is_empty());
        }
        p.sample(120_001, false, MouseButton::Right);
        p.sample(140_001, true, MouseButton::Right);
        assert_eq!(
            edges(&p.sample(180_001, true, MouseButton::Right).1),
            vec![(KeyCode::BTN_RIGHT.0, 1)]
        );
        let mut p = Pipeline::new(false, false);
        p.sample(1, false, MouseButton::Left);
        p.sample(20_001, true, MouseButton::Right);
        assert!(edges(&p.sample(60_001, true, MouseButton::Right).1).is_empty());
        p.sample(80_001, true, MouseButton::Left);
        assert_eq!(
            edges(&p.sample(120_001, true, MouseButton::Left).1),
            vec![(KeyCode::BTN_LEFT.0, 1)]
        );
    }
}
