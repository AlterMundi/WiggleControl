#![cfg(target_os = "linux")]
use evdev::{AbsoluteAxisCode, KeyCode};
use wigglecontrol::{
    controller::{Controller, Mode, Settings},
    delivery::{Batch, Packet, Publisher, State},
    touch::Contact,
    touch_gestures::Frame,
    Event, Hand, Point,
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
