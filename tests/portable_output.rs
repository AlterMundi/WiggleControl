//! Real shared delivery policy with a device-free event sink, also run on Windows.
mod support;
use support::output::{Edge, State};
use wigglecontrol::{controller::Mode, output::Packet, Event, MouseButton};
fn arm(state: &mut State) {
    state
        .apply(
            1,
            Packet::Transition {
                sequence: 1,
                issued_us: 1,
                mode: Mode::Pointer,
                enabled: true,
            },
            2000.0,
            150_000,
        )
        .unwrap();
}
fn frame(sequence: u64, source: u64, mouse: Vec<Event>) -> Packet {
    Packet::Frame {
        sequence,
        issued_us: source,
        source_us: source,
        mode: Mode::Pointer,
        mouse,
        absolute: Some(wigglecontrol::Point { x: 0.5, y: 0.5 }),
        contacts: vec![],
    }
}
#[test]
fn independent_watchdog_releases_every_owned_button_and_requires_rearm() {
    let mut state = State::default();
    arm(&mut state);
    state
        .apply(
            2,
            frame(
                2,
                2,
                vec![
                    Event::Button { down: true },
                    Event::NamedButton {
                        button: MouseButton::Right,
                        down: true,
                    },
                ],
            ),
            2000.0,
            150_000,
        )
        .unwrap();
    let released = state.tick(150_002, 150_000).unwrap();
    assert_eq!(
        released.mouse,
        vec![
            Edge::Button(MouseButton::Left, false),
            Edge::Button(MouseButton::Right, false)
        ]
    );
    assert!(state.tick(150_003, 150_000).unwrap().mouse.is_empty());
    assert!(state
        .apply(
            150_004,
            frame(3, 150_004, vec![Event::Button { down: true }]),
            2000.0,
            150_000
        )
        .is_err());
    assert!(state.release().mouse.is_empty());
}
#[test]
fn drag_hold_deadline_cannot_be_extended_by_repeated_liveness() {
    let mut state = State::default();
    arm(&mut state);
    state
        .apply(
            2,
            frame(2, 2, vec![Event::Button { down: true }]),
            2000.0,
            150_000,
        )
        .unwrap();
    for (sequence, now) in [(3, 100_000), (4, 200_000), (5, 300_000)] {
        state
            .apply(
                now,
                Packet::Hold {
                    sequence,
                    issued_us: now,
                    source_us: 2,
                    until_us: 350_000,
                    mode: Mode::Pointer,
                },
                2000.0,
                150_000,
            )
            .unwrap();
    }
    assert!(state.tick(349_999, 150_000).unwrap().mouse.is_empty());
    assert_eq!(
        state.tick(350_000, 150_000).unwrap().mouse,
        vec![Edge::Button(MouseButton::Left, false)]
    );
    assert!(state
        .apply(
            350_001,
            Packet::Hold {
                sequence: 6,
                issued_us: 350_001,
                source_us: 2,
                until_us: 450_000,
                mode: Mode::Pointer
            },
            2000.0,
            150_000
        )
        .is_err());
    assert!(state.release().mouse.is_empty());
}
#[test]
fn stale_derived_cursor_cannot_move_and_escape_neutralizes_before_key_tap() {
    let mut state = State::default();
    arm(&mut state);
    state
        .apply(
            2,
            frame(
                2,
                2,
                vec![Event::NamedButton {
                    button: MouseButton::Right,
                    down: true,
                }],
            ),
            2000.0,
            150_000,
        )
        .unwrap();
    assert!(state
        .apply(
            150_002,
            Packet::Cursor {
                sequence: 3,
                issued_us: 150_002,
                source_us: 2,
                mode: Mode::Pointer,
                absolute: wigglecontrol::Point { x: 0.9, y: 0.9 }
            },
            2000.0,
            150_000
        )
        .is_err());
    assert_eq!(
        state.release().mouse,
        vec![Edge::Button(MouseButton::Right, false)]
    );
    let mut state = State::default();
    arm(&mut state);
    state
        .apply(
            2,
            frame(
                2,
                2,
                vec![Event::NamedButton {
                    button: MouseButton::Right,
                    down: true,
                }],
            ),
            2000.0,
            150_000,
        )
        .unwrap();
    let escaped = state
        .apply(
            3,
            Packet::Escape {
                sequence: 3,
                issued_us: 3,
                source_us: 3,
                mode: Mode::Pointer,
            },
            2000.0,
            150_000,
        )
        .unwrap();
    assert!(escaped.escape);
    assert_eq!(escaped.mouse, vec![Edge::Button(MouseButton::Right, false)]);
    assert!(!state.tick(4, 150_000).unwrap().escape);
    assert!(state
        .apply(
            4,
            Packet::Escape {
                sequence: 4,
                issued_us: 4,
                source_us: 3,
                mode: Mode::Pointer
            },
            2000.0,
            150_000
        )
        .is_err());
}
