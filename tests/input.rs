#![cfg(target_os = "linux")]
use evdev::{EventType, KeyCode, RelativeAxisCode};
use wigglecontrol::input::{MouseIntent, MouseState};
use wigglecontrol::MouseButton;
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

#[test]
fn named_buttons_are_independent_and_legacy_left_shares_the_same_state() {
    let mut state = MouseState::default();
    state
        .apply(MouseIntent::Button { down: true }, 1.0)
        .unwrap();
    assert!(state
        .apply(
            MouseIntent::NamedButton {
                button: MouseButton::Left,
                down: true,
            },
            1.0,
        )
        .unwrap()
        .is_empty());
    for (button, code) in [
        (MouseButton::Right, KeyCode::BTN_RIGHT),
        (MouseButton::Middle, KeyCode::BTN_MIDDLE),
    ] {
        let events = state
            .apply(MouseIntent::NamedButton { button, down: true }, 1.0)
            .unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0],
            evdev::InputEvent::new(EventType::KEY.0, code.0, 1)
        );
        assert!(state
            .apply(MouseIntent::NamedButton { button, down: true }, 1.0)
            .unwrap()
            .is_empty());
    }
    let left_up = state
        .apply(MouseIntent::Button { down: false }, 1.0)
        .unwrap();
    assert_eq!(
        left_up,
        vec![evdev::InputEvent::new(
            EventType::KEY.0,
            KeyCode::BTN_LEFT.0,
            0
        )]
    );
    let release = state.apply(MouseIntent::Release, 1.0).unwrap();
    assert_eq!(
        release
            .iter()
            .map(|e| (e.code(), e.value()))
            .collect::<Vec<_>>(),
        vec![(KeyCode::BTN_RIGHT.0, 0), (KeyCode::BTN_MIDDLE.0, 0)]
    );
    assert!(state.release().is_empty());
    // Release neutralizes input without permanently disabling the writer.
    assert_eq!(
        state
            .apply(MouseIntent::Button { down: true }, 1.0)
            .unwrap()
            .len(),
        1
    );
    assert_eq!(state.apply(MouseIntent::Stop, 1.0).unwrap().len(), 1);
}

#[test]
fn named_button_protocol_rejects_unknown_button_without_reinterpreting_it_as_left() {
    let intent: MouseIntent =
        serde_json::from_str(r#"{"type":"named_button","button":"right","down":true}"#).unwrap();
    let mut state = MouseState::default();
    let events = state.apply(intent, 1.0).unwrap();
    assert_eq!(events[0].code(), KeyCode::BTN_RIGHT.0);
    for text in [
        r#"{"type":"named_button","button":"unknown","down":true}"#,
        r#"{"type":"named_button","down":true}"#,
        r#"{"type":"button","button":"right","down":true}"#,
    ] {
        assert!(serde_json::from_str::<MouseIntent>(text).is_err());
    }
    assert_eq!(state.release()[0].code(), KeyCode::BTN_RIGHT.0);
}

#[test]
fn fine_wheel_preserves_subdetents_both_axes_and_clears_residual_on_release() {
    let mut s = MouseState::default();
    for n in 1..=4 {
        let events = s
            .apply(
                MouseIntent::ScrollFine {
                    vertical: -30,
                    horizontal: 30,
                },
                1.0,
            )
            .unwrap();
        assert!(events
            .iter()
            .any(|e| e.code() == RelativeAxisCode::REL_WHEEL_HI_RES.0 && e.value() == -30));
        assert!(events
            .iter()
            .any(|e| e.code() == RelativeAxisCode::REL_HWHEEL_HI_RES.0 && e.value() == 30));
        assert_eq!(
            events
                .iter()
                .any(|e| e.code() == RelativeAxisCode::REL_WHEEL.0),
            n == 4
        );
        assert_eq!(
            events
                .iter()
                .any(|e| e.code() == RelativeAxisCode::REL_HWHEEL.0),
            n == 4
        );
    }
    s.apply(
        MouseIntent::ScrollFine {
            vertical: 90,
            horizontal: -90,
        },
        1.0,
    )
    .unwrap();
    s.release();
    let after = s
        .apply(
            MouseIntent::ScrollFine {
                vertical: 30,
                horizontal: -30,
            },
            1.0,
        )
        .unwrap();
    assert!(after
        .iter()
        .all(|e| e.code() != RelativeAxisCode::REL_WHEEL.0
            && e.code() != RelativeAxisCode::REL_HWHEEL.0));
    let legacy = s
        .apply(
            MouseIntent::Scroll {
                vertical: 2,
                horizontal: -1,
            },
            1.0,
        )
        .unwrap();
    for (code, value) in [
        (RelativeAxisCode::REL_WHEEL_HI_RES, 240),
        (RelativeAxisCode::REL_WHEEL, 2),
        (RelativeAxisCode::REL_HWHEEL_HI_RES, -120),
        (RelativeAxisCode::REL_HWHEEL, -1),
    ] {
        assert!(legacy
            .iter()
            .any(|e| e.code() == code.0 && e.value() == value));
    }
    assert!(s
        .apply(
            MouseIntent::ScrollFine {
                vertical: i32::MIN,
                horizontal: 0
            },
            1.0
        )
        .is_err());
}
