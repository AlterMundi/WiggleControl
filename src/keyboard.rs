//! Fixed Escape tap output, separate from mouse classification and held buttons.
use evdev::{uinput::VirtualDevice, AttributeSet, EventType, InputEvent, KeyCode};
use std::io;

pub(crate) fn create() -> io::Result<VirtualDevice> {
    VirtualDevice::builder()?
        .name("WiggleControl Escape Keyboard")
        .with_keys(&AttributeSet::from_iter([KeyCode::KEY_ESC]))?
        .build()
}

pub fn escape_events() -> [InputEvent; 2] {
    [1, 0].map(|value| InputEvent::new(EventType::KEY.0, KeyCode::KEY_ESC.0, value))
}

/// Each edge has its own SYN_REPORT. Attempt release even if pressing fails.
pub(crate) fn tap(mut emit: impl FnMut(&[InputEvent]) -> io::Result<()>) -> io::Result<()> {
    let [down, up] = escape_events();
    let press = emit(&[down]);
    let release = emit(&[up]);
    press.and(release)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failed_press_still_attempts_release_in_a_separate_frame() {
        let mut frames = Vec::new();
        let result = tap(|events| {
            frames.push(events.to_vec());
            if frames.len() == 1 {
                Err(io::Error::other("test write failure"))
            } else {
                Ok(())
            }
        });
        assert!(result.is_err());
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0].len(), 1);
        assert_eq!(frames[1].len(), 1);
        assert_eq!(frames[0][0].value(), 1);
        assert_eq!(frames[1][0].value(), 0);
        assert!(frames
            .iter()
            .flatten()
            .all(|e| e.code() == KeyCode::KEY_ESC.0));
    }
}
