//! Real shared delivery policy with a device-free event sink, also run on Windows.
use std::io;
use wigglecontrol::{
    contacts::Contact,
    controller::Mode,
    output::{self, MouseBackend, MouseIntent, Packet, TouchBackend},
    Event, MouseButton,
};
#[derive(Debug, PartialEq)]
enum Edge {
    Button(MouseButton, bool),
    Position(f64, f64),
}
#[derive(Default)]
struct Mouse {
    held: Vec<MouseButton>,
}
impl MouseBackend for Mouse {
    type Event = Edge;
    fn is_held(&self) -> bool {
        !self.held.is_empty()
    }
    fn release(&mut self) -> Vec<Edge> {
        self.held
            .drain(..)
            .map(|b| Edge::Button(b, false))
            .collect()
    }
    fn apply(&mut self, intent: MouseIntent, _gain: f64) -> io::Result<Vec<Edge>> {
        match intent {
            MouseIntent::Button { down } => self.apply(
                MouseIntent::NamedButton {
                    button: MouseButton::Left,
                    down,
                },
                1.0,
            ),
            MouseIntent::NamedButton { button, down } => {
                if self.held.contains(&button) == down {
                    return Ok(vec![]);
                }
                if down {
                    self.held.push(button);
                } else {
                    self.held.retain(|b| *b != button);
                }
                Ok(vec![Edge::Button(button, down)])
            }
            MouseIntent::Absolute { x, y } => Ok(vec![Edge::Position(x, y)]),
            MouseIntent::Release | MouseIntent::Stop => Ok(self.release()),
            _ => Ok(vec![]),
        }
    }
}
#[derive(Default)]
struct Contacts;
impl TouchBackend for Contacts {
    type Event = ();
    fn frame(&mut self, contacts: &[Contact]) -> io::Result<Vec<()>> {
        if !contacts.is_empty() {
            return Err(io::Error::other("this test sink has no touch device"));
        }
        Ok(vec![])
    }
    fn release(&mut self) -> Vec<()> {
        vec![]
    }
}
type State = output::State<Mouse, Contacts>;
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
