#![cfg(target_os = "linux")]
use evdev::{AbsoluteAxisCode, KeyCode};
use wigglecontrol::{
    controller::{Controller, Mode, Settings},
    delivery::{Batch, Packet, Publisher, State},
    touch::Contact,
    touch_gestures::Frame,
    Event, Hand, MouseButton, Point,
};
fn transition(sequence: u64, issued_us: u64, mode: Mode) -> Packet {
    Packet::Transition {
        sequence,
        issued_us,
        mode,
        enabled: true,
    }
}
fn packet(
    sequence: u64,
    now: u64,
    mode: Mode,
    mouse: Vec<Event>,
    contacts: Vec<Contact>,
) -> Packet {
    Packet::Frame {
        sequence,
        issued_us: now,
        source_us: now,
        mode,
        mouse,
        contacts,
        absolute: None,
    }
}
fn apply(s: &mut State, t: u64, p: Packet) -> Batch {
    s.apply(t, p, 2000.0, 150_000).unwrap()
}
fn button(batch: &Batch, value: i32) -> bool {
    batch
        .mouse
        .iter()
        .any(|e| e.code() == KeyCode::BTN_LEFT.0 && e.value() == value)
}
fn hand(id: u64, closed: bool) -> Hand {
    Hand {
        id,
        confidence: 0.99,
        engaged: true,
        pinch_eligible: true,
        pinch_onset_allowed: true,
        scrolling: false,
        cursor: Point { x: 0.5, y: 0.5 },
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
#[test]
fn mode_transition_releases_real_mouse_and_both_touch_slots_before_rearming() {
    let mut s = State::default();
    apply(&mut s, 1, transition(1, 1, Mode::Pointer));
    assert!(button(
        &apply(
            &mut s,
            2,
            packet(
                2,
                2,
                Mode::Pointer,
                vec![Event::Button { down: true }],
                vec![]
            )
        ),
        1
    ));
    assert!(button(
        &apply(&mut s, 3, transition(3, 3, Mode::TwoContacts)),
        0
    ));
    let down = apply(
        &mut s,
        4,
        packet(
            4,
            4,
            Mode::TwoContacts,
            vec![],
            vec![
                Contact {
                    id: 7,
                    x: 0.3,
                    y: 0.5,
                },
                Contact {
                    id: 9,
                    x: 0.7,
                    y: 0.5,
                },
            ],
        ),
    );
    assert_eq!(
        down.touch
            .iter()
            .filter(|e| e.code() == AbsoluteAxisCode::ABS_MT_TRACKING_ID.0 && e.value() >= 0)
            .count(),
        2
    );
    let release = apply(&mut s, 5, transition(5, 5, Mode::Precision));
    assert_eq!(
        release
            .touch
            .iter()
            .filter(|e| e.code() == AbsoluteAxisCode::ABS_MT_TRACKING_ID.0 && e.value() == -1)
            .count(),
        2
    );
    assert!(release
        .touch
        .iter()
        .any(|e| e.code() == KeyCode::BTN_TOUCH.0 && e.value() == 0));
    assert!(s.release().mouse.is_empty());
    assert!(s.release().touch.is_empty());
}
#[test]
fn watchdog_latches_and_old_held_frame_cannot_repress_without_transition() {
    let mut s = State::default();
    apply(&mut s, 1, transition(1, 1, Mode::Pointer));
    apply(
        &mut s,
        2,
        packet(
            2,
            2,
            Mode::Pointer,
            vec![Event::Button { down: true }],
            vec![],
        ),
    );
    assert!(button(&s.tick(150_002, 150_000).unwrap(), 0));
    assert!(s
        .apply(
            150_003,
            packet(
                3,
                150_003,
                Mode::Pointer,
                vec![Event::Button { down: true }],
                vec![]
            ),
            2000.0,
            150_000
        )
        .is_err());
    assert!(s.release().mouse.is_empty());
    apply(&mut s, 150_004, transition(4, 150_004, Mode::Pointer));
    assert!(button(
        &apply(
            &mut s,
            150_005,
            packet(
                5,
                150_005,
                Mode::Pointer,
                vec![Event::Button { down: true }],
                vec![]
            )
        ),
        1
    ));
}
#[test]
fn rejection_distinguishes_source_expiry_in_transit_from_writer_watchdog_loss() {
    let mut state = State::default();
    apply(&mut state, 1, transition(1, 1, Mode::Pointer));
    apply(
        &mut state,
        2,
        packet(
            2,
            2,
            Mode::Pointer,
            vec![Event::Button { down: true }],
            vec![],
        ),
    );
    apply(
        &mut state,
        140_000,
        Packet::Hold {
            sequence: 3,
            issued_us: 140_000,
            source_us: 2,
            until_us: 1_000_002,
            mode: Mode::Pointer,
        },
    );
    // This new source is fresh at publication (148999us), but a1002us
    // transit interval carries it across the150000us receiver fence.
    let error = state
        .apply(
            150_004,
            Packet::Frame {
                sequence: 4,
                issued_us: 149_002,
                source_us: 3,
                mode: Mode::Pointer,
                mouse: vec![],
                absolute: None,
                contacts: vec![],
            },
            2000.0,
            150_000,
        )
        .err()
        .unwrap();
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
    let message = error.to_string();
    let (_, json) = message.split_once("; ").unwrap();
    let evidence: serde_json::Value = serde_json::from_str(json).unwrap();
    assert_eq!(evidence["reasons"], serde_json::json!(["source_expired"]));
    assert_eq!(evidence["source_age_us"], 150_001);
    assert_eq!(evidence["issued_age_us"], 1_002);
    assert_eq!(evidence["receive_gap_us"], 10_004);
    assert_eq!(evidence["armed_before"], true);
    assert_eq!(evidence["armed_after"], true);
    assert!(button(&state.release(), 0));
    assert!(state.release().mouse.is_empty());

    let mut state = State::default();
    apply(&mut state, 1, transition(1, 1, Mode::Pointer));
    let error = state
        .apply(
            150_002,
            packet(2, 150_002, Mode::Pointer, vec![], vec![]),
            2000.0,
            150_000,
        )
        .err()
        .unwrap();
    let message = error.to_string();
    let evidence: serde_json::Value =
        serde_json::from_str(message.split_once("; ").unwrap().1).unwrap();
    assert_eq!(evidence["reasons"], serde_json::json!(["unarmed"]));
    assert_eq!(evidence["source_age_us"], 0);
    assert_eq!(evidence["receive_gap_us"], 150_001);
    assert_eq!(evidence["armed_before"], true);
    assert_eq!(evidence["armed_after"], false);
}

#[test]
fn invalid_source_or_competing_output_is_terminal_and_cleanup_preserves_release() {
    for case in 0..6 {
        let mut s = State::default();
        apply(&mut s, 1, transition(1, 1, Mode::Pointer));
        apply(
            &mut s,
            2,
            packet(
                2,
                2,
                Mode::Pointer,
                vec![Event::Button { down: true }],
                vec![],
            ),
        );
        let p = match case {
            0 => packet(
                3,
                3,
                Mode::Pointer,
                vec![],
                vec![Contact {
                    id: 1,
                    x: 0.5,
                    y: 0.5,
                }],
            ),
            1 => packet(3, 3, Mode::TwoContacts, vec![], vec![]),
            2 => packet(2, 3, Mode::Pointer, vec![], vec![]),
            3 => Packet::Frame {
                absolute: None,
                sequence: 3,
                issued_us: 3,
                source_us: 2,
                mode: Mode::Pointer,
                mouse: vec![],
                contacts: vec![],
            },
            4 => packet(3, 4, Mode::Pointer, vec![], vec![]),
            _ => packet(
                3,
                3,
                Mode::Pointer,
                vec![Event::Move {
                    dx: f64::NAN,
                    dy: 0.0,
                }],
                vec![],
            ),
        };
        assert!(s.apply(3, p, 2000.0, 150_000).is_err(), "case{case}");
        assert!(button(&s.release(), 0));
    }
}
#[test]
fn controller_recovers_from_gap_with_a_neutral_transition_not_replayed_contacts() {
    let mut c = Controller::new(Settings {
        initial_mode: Mode::TwoContacts,
        ..Default::default()
    })
    .unwrap();
    let mut publisher = Publisher::default();
    let mut s = State::default();
    apply(
        &mut s,
        1,
        publisher.packet(&c.start(), 1, None).unwrap().unwrap(),
    );
    for (t, closed) in [(2, false), (20_002, true), (60_002, true)] {
        let output = c.observe(
            t,
            Frame {
                version: 1,
                source_us: t,
                ambiguous: false,
                hands: vec![hand(7, closed), hand(9, closed)],
            },
            None,
        );
        apply(
            &mut s,
            t,
            publisher.packet(&output, t, Some(t)).unwrap().unwrap(),
        );
    }
    let expired = s.tick(300_002, 150_000).unwrap();
    assert!(expired
        .touch
        .iter()
        .any(|e| e.code() == KeyCode::BTN_TOUCH.0 && e.value() == 0));
    let recovery = c.observe(
        300_003,
        Frame {
            version: 1,
            source_us: 300_003,
            ambiguous: false,
            hands: vec![hand(7, true), hand(9, true)],
        },
        None,
    );
    assert!(recovery.transition);
    assert!(recovery.contacts.is_empty());
    let p = publisher
        .packet(&recovery, 300_003, Some(300_003))
        .unwrap()
        .unwrap();
    assert!(matches!(p, Packet::Transition { .. }));
    assert!(apply(&mut s, 300_003, p).touch.is_empty());
}

#[test]
fn calibrated_absolute_packets_keep_source_fences_order_buttons_and_release_without_motion() {
    let mut controller = Controller::new(Settings::default()).unwrap();
    let mut publisher = Publisher::default();
    let mut consumer = State::default();
    let workspace = wigglecontrol::touch_gestures::Workspace {
        min: Point { x: 0.2, y: 0.3 },
        max: Point { x: 0.8, y: 0.9 },
    };
    let initial = controller.start_at(1);
    let start = publisher
        .absolute_packet(&initial, 1, None, workspace)
        .unwrap()
        .unwrap();
    apply(&mut consumer, 1, start);
    let mut selected = hand(7, false);
    selected.cursor = Point { x: 0.8, y: 0.3 };
    let observe = |controller: &mut Controller, now, h: Hand| {
        controller.observe(
            now,
            Frame {
                version: 1,
                source_us: now,
                ambiguous: false,
                hands: vec![h],
            },
            None,
        )
    };
    let open = observe(&mut controller, 10, selected.clone());
    let packet = publisher
        .absolute_packet(&open, 10, Some(10), workspace)
        .unwrap()
        .unwrap();
    let events = apply(&mut consumer, 10, packet);
    assert!(events.mouse.iter().any(|e| e.event_type().0 == 3
        && e.code() == AbsoluteAxisCode::ABS_X.0
        && e.value() == 65535));
    assert!(events.mouse.iter().all(|e| e.event_type().0 != 2));
    selected.index_tip.x = 0.42;
    let closing = observe(&mut controller, 20_000, selected.clone());
    let p = publisher
        .absolute_packet(&closing, 20_000, Some(20_000), workspace)
        .unwrap()
        .unwrap();
    apply(&mut consumer, 20_000, p);
    let held = observe(&mut controller, 100_000, selected);
    assert!(held.button_down);
    let p = publisher
        .absolute_packet(&held, 100_000, Some(100_000), workspace)
        .unwrap()
        .unwrap();
    let press = apply(&mut consumer, 100_000, p);
    assert!(button(&press, 1));
    let abs_index = press
        .mouse
        .iter()
        .position(|e| e.event_type().0 == 3)
        .unwrap();
    let down_index = press
        .mouse
        .iter()
        .position(|e| e.event_type().0 == 1)
        .unwrap();
    assert!(abs_index < down_index);
    assert!(button(&consumer.tick(250_001, 150_000).unwrap(), 0));
    // A stale held packet cannot extend the watchdog or move/press the cursor.
    let late = publisher
        .absolute_packet(&held, 250_002, Some(100_000), workspace)
        .unwrap()
        .unwrap();
    assert!(consumer.apply(250_002, late, 2000.0, 150_000).is_err());
    let stopped = controller.stop();
    let p = publisher
        .absolute_packet(&stopped, 250_003, None, workspace)
        .unwrap()
        .unwrap();
    let released = apply(&mut consumer, 250_003, p);
    assert!(released.mouse.iter().all(|e| e.event_type().0 != 3));
}

#[test]
fn named_right_drag_orders_position_before_press_and_releases_on_all_terminal_paths() {
    for termination in [
        "stop",
        "cancel",
        "transition",
        "deadline",
        "watchdog",
        "rejection",
    ] {
        let mut state = State::default();
        apply(&mut state, 1, transition(1, 1, Mode::Pointer));
        let batch = apply(
            &mut state,
            2,
            Packet::Frame {
                sequence: 2,
                issued_us: 2,
                source_us: 2,
                mode: Mode::Pointer,
                absolute: Some(Point { x: 0.3, y: 0.7 }),
                mouse: vec![Event::NamedButton {
                    button: MouseButton::Right,
                    down: true,
                }],
                contacts: vec![],
            },
        );
        assert_eq!(batch.mouse.len(), 3);
        assert_eq!(batch.mouse[0].code(), AbsoluteAxisCode::ABS_X.0);
        assert_eq!(batch.mouse[1].code(), AbsoluteAxisCode::ABS_Y.0);
        assert_eq!(
            (batch.mouse[2].code(), batch.mouse[2].value()),
            (KeyCode::BTN_RIGHT.0, 1)
        );
        // A delayed-but-alive estimator may retain this established right drag,
        // but only the original source/deadline and existing watchdog can do so.
        let hold = apply(
            &mut state,
            3,
            Packet::Hold {
                sequence: 3,
                issued_us: 3,
                source_us: 2,
                until_us: if termination == "watchdog" {
                    1_000_002
                } else {
                    100_002
                },
                mode: Mode::Pointer,
            },
        );
        assert!(hold.mouse.is_empty());
        let release = match termination {
            "stop" => apply(&mut state, 4, Packet::Stop),
            "cancel" => apply(
                &mut state,
                4,
                packet(
                    4,
                    4,
                    Mode::Pointer,
                    vec![Event::Cancel {
                        reason: "explicit".into(),
                    }],
                    vec![],
                ),
            ),
            "transition" => apply(&mut state, 4, transition(4, 4, Mode::Scroll)),
            "deadline" => state.tick(100_002, 150_000).unwrap(),
            "watchdog" => state.tick(150_003, 150_000).unwrap(),
            _ => {
                assert!(state
                    .apply(
                        4,
                        packet(4, 4, Mode::Scroll, vec![], vec![]),
                        2000.0,
                        150_000
                    )
                    .is_err());
                state.release()
            }
        };
        assert_eq!(release.mouse.len(), 1, "{termination}");
        assert_eq!(
            (release.mouse[0].code(), release.mouse[0].value()),
            (KeyCode::BTN_RIGHT.0, 0)
        );
        assert!(state.release().mouse.is_empty());
    }
}

#[test]
fn scroll_can_release_named_buttons_but_cannot_press_them() {
    let mut state = State::default();
    apply(&mut state, 1, transition(1, 1, Mode::Scroll));
    let release = apply(
        &mut state,
        2,
        packet(
            2,
            2,
            Mode::Scroll,
            vec![Event::NamedButton {
                button: MouseButton::Right,
                down: false,
            }],
            vec![],
        ),
    );
    assert!(release.mouse.is_empty());
    assert!(state
        .apply(
            3,
            packet(
                3,
                3,
                Mode::Scroll,
                vec![Event::NamedButton {
                    button: MouseButton::Right,
                    down: true
                }],
                vec![]
            ),
            2000.0,
            150_000
        )
        .is_err());
    assert!(state.release().mouse.is_empty());
}
