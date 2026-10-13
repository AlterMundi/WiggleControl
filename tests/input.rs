#![cfg(target_os = "linux")]
use evdev::{EventType, KeyCode, RelativeAxisCode};
use wigglecontrol::input::{MouseIntent, MouseState};
#[test]
fn real_button_codes_and_idempotent_release() {
    let mut s = MouseState::default();
    let down = s.apply(MouseIntent::Button { down: true }, 1000.0).unwrap();
    assert_eq!(
        (down[0].event_type(), down[0].code(), down[0].value()),
        (EventType::KEY, KeyCode::BTN_LEFT.0, 1)
    );
    assert!(s
        .apply(MouseIntent::Button { down: true }, 1000.0)
        .unwrap()
        .is_empty());
    let up = s.release();
    assert_eq!(up[0].value(), 0);
    assert!(s.release().is_empty());
}
#[test]
fn fractional_motion_accumulates_without_rounding_bias() {
    let mut s = MouseState::default();
    for _ in 0..4 {
        assert!(s
            .apply(
                MouseIntent::Move {
                    dx: 0.0002,
                    dy: 0.0
                },
                1000.0
            )
            .unwrap()
            .is_empty());
    }
    let events = s
        .apply(
            MouseIntent::Move {
                dx: 0.0002,
                dy: 0.0,
            },
            1000.0,
        )
        .unwrap();
    assert_eq!(
        (events[0].code(), events[0].value()),
        (RelativeAxisCode::REL_X.0, 1)
    );
}
#[test]
fn rejects_nonfinite_large_move_and_wheel() {
    let mut s = MouseState::default();
    assert!(s
        .apply(
            MouseIntent::Move {
                dx: f64::NAN,
                dy: 0.0
            },
            1000.0
        )
        .is_err());
    assert!(s
        .apply(MouseIntent::Move { dx: 2.0, dy: 0.0 }, 1000.0)
        .is_err());
    assert!(s
        .apply(
            MouseIntent::Scroll {
                vertical: i32::MIN,
                horizontal: 0
            },
            1000.0
        )
        .is_err());
}

#[test]
fn absolute_pointer_frames_are_bounded_and_share_button_release() {
    let mut state = MouseState::default();
    let xy = state
        .apply(MouseIntent::Absolute { x: 0.25, y: 0.75 }, 1.0)
        .unwrap();
    assert_eq!(
        xy.iter()
            .map(|e| (e.event_type(), e.code(), e.value()))
            .collect::<Vec<_>>(),
        vec![
            (EventType::ABSOLUTE, evdev::AbsoluteAxisCode::ABS_X.0, 16384),
            (EventType::ABSOLUTE, evdev::AbsoluteAxisCode::ABS_Y.0, 49151),
        ]
    );
    for (x, y) in [
        (f64::NAN, 0.0),
        (0.0, f64::INFINITY),
        (-0.01, 0.0),
        (1.01, 0.0),
    ] {
        assert!(state.apply(MouseIntent::Absolute { x, y }, 1.0).is_err());
    }
    state
        .apply(MouseIntent::Button { down: true }, 1.0)
        .unwrap();
    assert_eq!(state.release()[0].value(), 0);
    assert!(state.release().is_empty());
}
