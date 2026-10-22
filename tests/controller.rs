mod support;
use wigglecontrol::{
    controller::{Controller, Mode, Output, Settings},
    macros,
    touch_gestures::Frame,
    Config, Event, Hand, Point,
};
fn hand(id: u64, closed: bool, x: f64, y: f64) -> Hand {
    Hand {
        id,
        confidence: 0.99,
        engaged: true,
        pinch_eligible: true,
        pinch_onset_allowed: true,
        scrolling: false,
        cursor: Point { x, y },
        thumb_tip: Point { x: 0.4, y: 0.5 },
        index_tip: Point {
            x: if closed { 0.42 } else { 0.52 },
            y: 0.5,
        },
        palm_width: 0.2,
        pinch_distance_m: None,
        pinch_distance_ratio: None,
        secondary_pinch: None,
    }
}
fn frame(t: u64, hands: Vec<Hand>) -> Frame {
    Frame {
        version: 1,
        source_us: t,
        ambiguous: false,
        hands,
    }
}

#[test]
fn projected_ratio_rearms_and_releases_in_pointer_and_both_contact_modes() {
    for mode in [Mode::Pointer, Mode::OneContact, Mode::TwoContacts] {
        let mut c = controller(mode);
        let hands = |ratio| {
            let mut a = hand(1, false, 0.3, 0.5);
            a.pinch_distance_ratio = Some(ratio);
            let mut b = hand(2, false, 0.7, 0.5);
            b.pinch_distance_ratio = Some(ratio);
            vec![a, b]
        };
        c.observe(1, frame(1, hands(0.6)), None);
        c.observe(20_001, frame(20_001, hands(0.1)), None);
        let held = c.observe(60_001, frame(60_001, hands(0.1)), None);
        match mode {
            Mode::Pointer => assert!(held.button_down),
            Mode::OneContact => assert_eq!(held.contacts.len(), 1),
            Mode::TwoContacts => assert_eq!(held.contacts.len(), 2),
            _ => unreachable!(),
        }
        let opened = c.observe(70_001, frame(70_001, hands(0.6)), None);
        assert!(!opened.button_down && opened.contacts.is_empty());
    }
}
fn controller(mode: Mode) -> Controller {
    let mut c = Controller::new(Settings {
        initial_mode: mode,
        gestures: Config {
            smoothing_us: 0.0,
            ..Default::default()
        },
        ..Default::default()
    })
    .unwrap();
    c.start();
    c
}
fn observe(c: &mut Controller, t: u64, closed: bool) -> Output {
    c.observe(t, frame(t, vec![hand(7, closed, 0.5, 0.5)]), None)
}
fn down(o: &Output) -> bool {
    o.mouse.contains(&Event::Button { down: true })
}
fn up(o: &Output) -> bool {
    o.mouse.contains(&Event::Button { down: false })
}
fn press(c: &mut Controller) {
    observe(c, 1, false);
    observe(c, 20_001, true);
    let o = observe(c, 60_001, true);
    assert!(o.button_down || !o.contacts.is_empty());
}

#[test]
fn delivery_guard_discards_an_observation_that_expired_after_interpretation() {
    use support::output::State;
    use wigglecontrol::output::Publisher;
    let mut c = Controller::new(Settings {
        gestures: Config {
            stale_us: 300_000,
            ..Default::default()
        },
        ..Default::default()
    })
    .unwrap();
    let mut publisher = Publisher::default();
    let mut receiver = State::default();
    let start = publisher.packet(&c.start(), 0, None).unwrap().unwrap();
    receiver.apply(0, start, 2000.0, 300_000).unwrap();
    observe(&mut c, 1, false);
    observe(&mut c, 20_001, true);
    let late = observe(&mut c, 60_001, true);
    assert!(down(&late));
    // Actual retained failure: source 336251us old, publication 84us old.
    let handoff = 60_001 + 336_251 - 84;
    let unsafe_packet = publisher
        .packet(&late, handoff, Some(60_001))
        .unwrap()
        .unwrap();
    let error = receiver
        .apply(handoff + 84, unsafe_packet, 2000.0, 300_000)
        .err()
        .expect("expired source must still be rejected");
    assert!(error.to_string().contains("source_expired"));
    let (neutral, discarded) = c.guard_delivery(late, handoff, Some(60_001));
    assert!(discarded && neutral.transition && neutral.running);
    assert_eq!(neutral.reason, Some("delivery_source_expired"));
    assert!(!neutral.button_down && !neutral.pointer_ready);
    assert!(!down(&neutral));
    let packet = publisher
        .packet(&neutral, handoff, Some(60_001))
        .unwrap()
        .unwrap();
    receiver
        .apply(handoff + 84, packet, 2000.0, 300_000)
        .unwrap();
    // Closed reacquisition cannot invent the discarded click.
    let closed = c.observe(
        handoff + 100,
        frame(handoff - 100, vec![hand(7, true, 0.5, 0.5)]),
        None,
    );
    assert!(!down(&closed));
    // Queued but fresh open evidence need not be newer than the wall-clock guard.
    let opened = c.observe(
        handoff + 200,
        frame(handoff - 1, vec![hand(7, false, 0.5, 0.5)]),
        None,
    );
    assert!(opened.pointer_ready);
    observe(&mut c, handoff + 20_000, true);
    let pressed = observe(&mut c, handoff + 60_000, true);
    assert!(down(&pressed));
    let packet = publisher
        .packet(&pressed, handoff + 60_000, Some(handoff + 60_000))
        .unwrap()
        .unwrap();
    receiver
        .apply(handoff + 60_084, packet, 2000.0, 300_000)
        .unwrap();
}

#[test]
fn delivery_guard_keeps_fresh_frames_ticks_and_bounded_drag_grace() {
    let mut c = Controller::new(Settings {
        gestures: Config {
            stale_us: 300_000,
            drag_grace_us: 1_000_000,
            ..Default::default()
        },
        ..Default::default()
    })
    .unwrap();
    c.start();
    let fresh = observe(&mut c, 1, false);
    let (fresh, discarded) = c.guard_delivery(fresh, 80_001, Some(1));
    assert!(!discarded && fresh.pointer_ready);
    observe(&mut c, 100_001, true);
    let held = observe(&mut c, 140_001, true);
    assert!(held.button_down);
    let tick = c.tick(150_001);
    assert!(!c.guard_delivery(tick, 150_001, None).1);
    let lost = c.observe(160_001, frame(160_001, vec![]), None);
    let deadline = lost.drag_hold.unwrap().until_us;
    assert!(lost.drag_hold.unwrap().in_grace);
    let (hold, discarded) = c.guard_delivery(lost, 500_001, Some(160_001));
    assert!(!discarded && hold.button_down);
    assert_eq!(hold.drag_hold.unwrap().until_us, deadline);
}

#[test]
fn pointer_drag_switch_releases_and_closed_hand_cannot_move_or_press_until_open() {
    let mut c = controller(Mode::Pointer);
    press(&mut c);
    let switched = c.set_mode(Mode::Precision);
    assert!(up(&switched));
    assert!(!switched.button_down);
    assert!(switched.contacts.is_empty());
    for t in [80_001, 100_001] {
        let o = c.observe(t, frame(t, vec![hand(7, true, 0.8, 0.8)]), None);
        assert!(!o.pointer_ready);
        assert!(!down(&o));
        assert!(!o.mouse.iter().any(|e| matches!(e, Event::Move { .. })));
    }
    assert!(observe(&mut c, 120_001, false).pointer_ready);
    observe(&mut c, 140_001, true);
    assert!(down(&observe(&mut c, 180_001, true)));
}
#[test]
fn touch_switch_ends_both_contacts_and_reentry_requires_open() {
    let mut c = controller(Mode::TwoContacts);
    for (t, closed) in [(1, false), (20_001, true), (60_001, true)] {
        let o = c.observe(
            t,
            frame(
                t,
                vec![hand(7, closed, 0.3, 0.5), hand(9, closed, 0.7, 0.5)],
            ),
            None,
        );
        assert_eq!(o.contacts.len(), if t == 60_001 { 2 } else { 0 });
        assert!(o.mouse.is_empty());
    }
    let stopped = c.set_mode(Mode::OneContact);
    assert!(stopped.contacts.is_empty());
    assert!(stopped.mouse.is_empty());
    assert!(observe(&mut c, 80_001, true).contacts.is_empty());
    observe(&mut c, 100_001, false);
    observe(&mut c, 120_001, true);
    let one = observe(&mut c, 160_001, true);
    assert_eq!(one.contacts.len(), 1);
    assert!(one.contacts[0].id >= 2);
    assert!(c.set_mode(Mode::Pointer).contacts.is_empty());
    assert!(!down(&observe(&mut c, 180_001, true)));
}
#[test]
fn touch_rearm_feedback_follows_each_real_engine_after_identity_loss() {
    let mut c = controller(Mode::TwoContacts);
    let o = c.observe(
        1,
        frame(1, vec![hand(7, true, 0.3, 0.5), hand(9, true, 0.7, 0.5)]),
        None,
    );
    assert_eq!(o.awaiting_open_hands, [7, 9]);
    assert!(o.contacts.is_empty());
    let o = c.observe(
        50_001,
        frame(
            50_001,
            vec![hand(7, false, 0.3, 0.5), hand(9, true, 0.7, 0.5)],
        ),
        None,
    );
    assert_eq!(o.awaiting_open_hands, [9]);
    c.observe(
        70_001,
        frame(
            70_001,
            vec![hand(7, true, 0.3, 0.5), hand(9, true, 0.7, 0.5)],
        ),
        None,
    );
    let held = c.observe(
        110_001,
        frame(
            110_001,
            vec![hand(7, true, 0.3, 0.5), hand(9, true, 0.7, 0.5)],
        ),
        None,
    );
    assert_eq!(held.contacts.len(), 1);
    assert_eq!(held.awaiting_open_hands, [9]);
    let mut lost = frame(130_001, vec![]);
    lost.ambiguous = true;
    assert!(c.observe(130_001, lost, None).contacts.is_empty());
    let returning = c.observe(
        150_001,
        frame(
            150_001,
            vec![hand(17, true, 0.3, 0.5), hand(19, true, 0.7, 0.5)],
        ),
        None,
    );
    assert!(returning.contacts.is_empty());
    assert_eq!(returning.awaiting_open_hands, [17, 19]);
    let reopened = c.observe(
        170_001,
        frame(
            170_001,
            vec![hand(17, false, 0.3, 0.5), hand(19, false, 0.7, 0.5)],
        ),
        None,
    );
    assert!(reopened.awaiting_open_hands.is_empty());
    assert!(reopened.contacts.is_empty());
}
#[test]
fn precision_scales_motion_and_scroll_emits_only_wheel() {
    let mut normal = controller(Mode::Pointer);
    let mut fine = controller(Mode::Precision);
    for c in [&mut normal, &mut fine] {
        c.observe(1, frame(1, vec![hand(7, false, 0.4, 0.4)]), None);
    }
    let a = normal.observe(50_001, frame(50_001, vec![hand(7, false, 0.6, 0.7)]), None);
    let b = fine.observe(50_001, frame(50_001, vec![hand(7, false, 0.6, 0.7)]), None);
    let delta = |o: &Output| {
        o.mouse
            .iter()
            .find_map(|e| {
                if let Event::Move { dx, dy } = e {
                    Some((*dx, *dy))
                } else {
                    None
                }
            })
            .unwrap()
    };
    let (a, b) = (delta(&a), delta(&b));
    assert!((b.0 / a.0 - 0.25).abs() < 1e-12);
    assert!((b.1 / a.1 - 0.25).abs() < 1e-12);
    let mut scroll = controller(Mode::Scroll);
    observe(&mut scroll, 1, false);
    let wheel = scroll.observe(50_001, frame(50_001, vec![hand(7, true, 0.7, 0.8)]), None);
    assert!(wheel
        .mouse
        .iter()
        .any(|e| matches!(e,Event::Scroll{vertical, ..}if *vertical<0)));
    assert!(!wheel
        .mouse
        .iter()
        .any(|e| matches!(e, Event::Button { .. } | Event::Move { .. })));
    assert!(wheel.contacts.is_empty());
}
#[test]
fn clock_regression_and_invalid_frame_keep_the_actual_release_event() {
    for case in 0..5 {
        let mut c = controller(Mode::Pointer);
        press(&mut c);
        let mut f = frame(80_001, vec![hand(7, true, 0.5, 0.5)]);
        let now = match case {
            0 => 50_001,
            1 => {
                f.source_us = 90_001;
                80_001
            }
            2 => {
                f.ambiguous = true;
                80_001
            }
            3 => {
                f.hands.push(f.hands[0].clone());
                80_001
            }
            _ => 300_001,
        };
        let released = c.observe(now, f, None);
        assert!(up(&released), "case{case}");
        assert!(!released.button_down);
        assert!(released.contacts.is_empty());
        assert!(!down(&observe(&mut c, 320_001, true)));
        assert!(c.stop().contacts.is_empty());
    }
}
#[test]
fn explicit_claim_does_not_fall_back_to_another_participant_or_array_position() {
    let mut c = controller(Mode::Pointer);
    c.select_hand(Some(9));
    let o = c.observe(
        1,
        frame(1, vec![hand(9, false, 0.7, 0.5), hand(7, false, 0.3, 0.5)]),
        None,
    );
    assert_eq!(o.primary_hand, Some(9));
    let o = c.observe(
        50_001,
        frame(
            50_001,
            vec![hand(7, false, 0.8, 0.5), hand(9, false, 0.7, 0.5)],
        ),
        None,
    );
    assert!(o.mouse.is_empty());
    let missing = c.observe(
        100_001,
        frame(100_001, vec![hand(7, false, 0.8, 0.5)]),
        None,
    );
    assert!(!missing.pointer_ready);
    assert!(!down(&missing));
    let returned = c.observe(150_001, frame(150_001, vec![hand(9, true, 0.8, 0.5)]), None);
    assert!(!returned.pointer_ready);
}
fn macros_controller(mode: Mode) -> Controller {
    let config = macros::Config {
        hold_us: 200_000,
        neutral_us: 100_000,
        cooldown_us: 200_000,
        stale_us: 150_000,
        bindings: vec![macros::Binding {
            pose: macros::Pose::Victory,
            action: macros::Action::TogglePrecision,
        }],
    };
    let mut c = Controller::new(Settings {
        initial_mode: mode,
        macros: Some(config),
        ..Default::default()
    })
    .unwrap();
    c.start();
    c
}
#[test]
fn unarmed_closed_secondary_blocks_macro_without_blocking_primary_hover() {
    let mut c = macros_controller(Mode::Pointer);
    for i in 0..15 {
        let t = 1 + i * 50_000;
        let mut h = hand(7, false, 0.3 + i as f64 * 0.01, 0.5);
        h.secondary_pinch = Some(wigglecontrol::PinchEvidence {
            source_id: if i < 3 { 1 } else { 2 },
            source_us: t,
            confidence: 0.99,
            eligible: true,
            distance_m: if i < 3 { 0.08 } else { 0.01 },
            onset_allowed: true,
        });
        let pose = macros::HandPose {
            id: 7,
            confidence: 0.99,
            pose: if i < 3 {
                None
            } else {
                Some(macros::Pose::Victory)
            },
        };
        let o = c.observe(t, frame(t, vec![h]), Some(pose));
        assert!(o.action.is_none());
        assert_eq!(o.mode, Mode::Pointer);
        assert!(!o.button_down);
        assert!(o.pointer_ready);
        if i >= 3 {
            assert_eq!(o.macro_feedback.unwrap().blocked_reason, Some("input_busy"));
        }
    }
}
#[test]
fn held_macro_changes_mode_once_and_discards_same_frame_movement() {
    let mut c = macros_controller(Mode::Pointer);
    let mut actions = 0;
    for i in 0..15 {
        let t = 1 + i * 50_000;
        let pose = macros::HandPose {
            id: 7,
            confidence: 0.99,
            pose: if i < 3 {
                None
            } else {
                Some(macros::Pose::Victory)
            },
        };
        let o = c.observe(
            t,
            frame(t, vec![hand(7, false, 0.3 + i as f64 * 0.01, 0.5)]),
            Some(pose),
        );
        if o.action.is_some() {
            actions += 1;
            assert_eq!(o.mode, Mode::Precision);
            assert!(o.transition);
            assert!(!o.mouse.iter().any(|e| matches!(
                e,
                Event::Move { .. } | Event::Scroll { .. } | Event::Button { down: true }
            )));
            assert!(!o.pointer_ready);
        }
    }
    assert_eq!(actions, 1);
    assert!(!c.action(macros::Action::PlayPause).action.unwrap().handled);
    assert!(!c.action(macros::Action::PauseController).running);
    assert!(!observe(&mut c, 800_001, false).running);
}
#[test]
fn touch_pinch_candidate_and_missing_pose_cannot_complete_a_macro() {
    for mode in [Mode::Pointer, Mode::OneContact, Mode::TwoContacts] {
        let mut c = macros_controller(mode);
        for i in 0..9 {
            let t = 1 + i * 50_000;
            let closing = i >= 6;
            let pose = macros::HandPose {
                id: 7,
                confidence: 0.99,
                pose: if i < 3 {
                    None
                } else {
                    Some(macros::Pose::Victory)
                },
            };
            let o = c.observe(t, frame(t, vec![hand(7, closing, 0.5, 0.5)]), Some(pose));
            assert!(o.action.is_none(), "{mode:?}/{i}");
        }
    }
    let mut c = macros_controller(Mode::Pointer);
    for i in 0..12 {
        let t = 1 + i * 50_000;
        let p = (i >= 3).then_some(macros::HandPose {
            id: 7,
            confidence: 0.99,
            pose: Some(macros::Pose::Victory),
        });
        assert!(c
            .observe(t, frame(t, vec![hand(7, false, 0.5, 0.5)]), p)
            .action
            .is_none());
    }
}

#[test]
fn changing_mode_never_resets_the_source_watermark_or_accepts_invalid_settings() {
    let mut c = controller(Mode::Pointer);
    observe(&mut c, 1, false);
    c.set_mode(Mode::OneContact);
    let mut repeated = frame(1, vec![hand(7, false, 0.5, 0.5)]);
    repeated.source_us = 1;
    assert!(c.observe(10_001, repeated, None).transition);
    let defaults: Settings = serde_json::from_str("{}").unwrap();
    assert!(Controller::new(defaults).is_ok());
    for value in [
        serde_json::json!({"version":2}),
        serde_json::json!({"precision_gain":0}),
        serde_json::json!({"gestures":{"stale_us":1}}),
    ] {
        assert!(Controller::new(serde_json::from_value(value).unwrap()).is_err());
    }
    assert!(serde_json::from_str::<Settings>(r#"{"gestures":{"typo":1}}"#).is_err());
}

#[test]
fn pose_evidence_cannot_invent_a_neutral_hand_or_upgrade_its_confidence() {
    for fault in 0..4 {
        let mut c = macros_controller(Mode::Pointer);
        for i in 0..9 {
            let t = 1 + i * 50_000;
            let neutral = i < 3;
            let mut p = macros::HandPose {
                id: 7,
                confidence: 0.99,
                pose: if neutral {
                    None
                } else {
                    Some(macros::Pose::Victory)
                },
            };
            let mut hands = vec![hand(7, false, 0.5, 0.5)];
            if neutral {
                match fault {
                    0 => p.confidence = f64::NAN,
                    1 => p.id = 9,
                    2 => hands[0].confidence = 0.6,
                    _ => hands.clear(),
                }
            }
            assert!(
                c.observe(t, frame(t, hands), Some(p)).action.is_none(),
                "fault{fault}/{i}"
            );
        }
    }
}

#[test]
fn queued_open_frame_from_before_manual_mode_change_cannot_rearm_a_closed_hand() {
    let mut c = controller(Mode::Pointer);
    press(&mut c);
    assert!(up(&c.set_mode_at(Mode::OneContact, 80_001)));
    let queued = c.observe(90_001, frame(70_001, vec![hand(7, false, 0.5, 0.5)]), None);
    assert!(queued.contacts.is_empty());
    assert_eq!(queued.reason, Some("invalid_observation"));
    for (source, now) in [(100_001, 110_001), (140_001, 150_001)] {
        assert!(c
            .observe(now, frame(source, vec![hand(7, true, 0.5, 0.5)]), None)
            .contacts
            .is_empty());
    }
    c.observe(
        170_001,
        frame(160_001, vec![hand(7, false, 0.5, 0.5)]),
        None,
    );
    c.observe(190_001, frame(180_001, vec![hand(7, true, 0.5, 0.5)]), None);
    assert_eq!(
        c.observe(230_001, frame(220_001, vec![hand(7, true, 0.5, 0.5)]), None)
            .contacts
            .len(),
        1
    );
}

#[test]
fn visual_unknown_low_score_or_ineligible_neutral_cannot_arm_a_macro() {
    let policy = macros::VisualPolicy {
        model: "/fictional/model.onnx".into(),
        neutral_pose: macros::Pose::OpenPalm,
        min_confidence: 0.8,
    };
    for fault in 0..3 {
        let mut c = macros_controller(Mode::Pointer);
        for i in 0..15 {
            let t = 1 + i * 50_000;
            let mut h = hand(7, false, 0.5, 0.5);
            let neutral = i < 3;
            let evidence = if neutral {
                match fault {
                    0 => policy.evidence(7, 0.99, None),
                    1 => policy.evidence(7, 0.79, Some(macros::Pose::OpenPalm)),
                    _ => {
                        h.engaged = false;
                        h.pinch_eligible = false;
                        policy.evidence(7, 0.99, Some(macros::Pose::OpenPalm))
                    }
                }
            } else {
                policy.evidence(7, 0.99, Some(macros::Pose::Victory))
            };
            let o = c.observe(t, frame(t, vec![h]), evidence);
            assert!(o.action.is_none(), "fault{fault}/{i}");
            assert!(!o.macro_feedback.unwrap().armed);
        }
    }
}

#[test]
fn macro_progress_uses_observations_and_receipt_does_not_repeat_the_action() {
    let mut c = macros_controller(Mode::Pointer);
    for i in 0..3 {
        let t = 1 + i * 50_000;
        c.observe(
            t,
            frame(t, vec![hand(7, false, 0.5, 0.5)]),
            Some(macros::HandPose {
                id: 7,
                confidence: 0.99,
                pose: None,
            }),
        );
    }
    let mut fired = 0;
    let mut action_source = 0;
    for i in 3..49 {
        let t = 1 + i * 50_000;
        let o = c.observe(
            t,
            frame(t, vec![hand(7, false, 0.5, 0.5)]),
            Some(macros::HandPose {
                id: 7,
                confidence: 0.99,
                pose: Some(macros::Pose::Victory),
            }),
        );
        if i == 4 {
            assert_eq!(o.macro_feedback.unwrap().hold_elapsed_us, 50_000);
            assert_eq!(
                c.tick(t + 25_000).macro_feedback.unwrap().hold_elapsed_us,
                50_000
            );
        }
        if o.action.is_some() {
            fired += 1;
            action_source = t;
            assert!(o.transition);
        } else if fired > 0 {
            assert!(!o.transition);
        }
        if fired > 0 {
            assert_eq!(o.mode, Mode::Precision);
            if t - action_source < 2_000_000 {
                let receipt = o.last_macro_action.unwrap();
                assert_eq!(receipt.action, macros::Action::TogglePrecision);
                assert_eq!(receipt.source_us, action_source);
                assert!(receipt.handled);
            } else {
                assert!(o.last_macro_action.is_none());
            }
        }
    }
    assert_eq!(fired, 1);
}

#[test]
fn macro_owner_survives_array_order_but_a_new_hand_must_rearm() {
    for replacement in [false, true] {
        let mut c = macros_controller(Mode::TwoContacts);
        let mut fired = 0;
        for i in 0..14 {
            let t = 1 + i * 50_000;
            let mut hands = if replacement && i >= 5 {
                vec![hand(7, false, 0.3, 0.5)]
            } else if i >= 5 {
                vec![hand(7, false, 0.3, 0.5), hand(9, false, 0.7, 0.5)]
            } else {
                vec![hand(9, false, 0.7, 0.5)]
            };
            if i % 2 == 0 {
                hands.reverse();
            }
            let f = frame(t, hands);
            let id = c.macro_hand(&f).unwrap();
            if fired == 0 || replacement {
                assert_eq!(id, if replacement && i >= 5 { 7 } else { 9 });
            }
            let o = c.observe(
                t,
                f,
                Some(macros::HandPose {
                    id,
                    confidence: 0.99,
                    pose: if i < 3 {
                        None
                    } else {
                        Some(macros::Pose::Victory)
                    },
                }),
            );
            fired += usize::from(o.action.is_some());
        }
        assert_eq!(fired, if replacement { 0 } else { 1 });
    }
}

#[test]
fn one_hand_claim_cannot_strand_two_contact_macro_selection() {
    let mut c = macros_controller(Mode::Pointer);
    c.select_hand(Some(99));
    let f = frame(1, vec![hand(7, false, 0.3, 0.5), hand(9, false, 0.7, 0.5)]);
    assert_eq!(c.macro_hand(&f), Some(99));
    c.set_mode(Mode::TwoContacts);
    assert_eq!(c.macro_hand(&f), Some(7));
    c.set_mode(Mode::Pointer);
    assert_eq!(c.macro_hand(&f), Some(99));
}
