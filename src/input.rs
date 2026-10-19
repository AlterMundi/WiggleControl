//! Linux mouse output. No camera, compositor control or privileged inference.
use crate::MouseButton;
use evdev::{
    uinput::VirtualDevice, AbsInfo, AbsoluteAxisCode, AttributeSet, EventType, InputEvent, KeyCode,
    PropType, RelativeAxisCode, UinputAbsSetup,
};
use serde::Deserialize;
use std::{
    io,
    sync::mpsc,
    time::{Duration, Instant},
};

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum MouseIntent {
    Move {
        dx: f64,
        dy: f64,
    },
    Absolute {
        x: f64,
        y: f64,
    },
    Button {
        down: bool,
    },
    NamedButton {
        button: MouseButton,
        down: bool,
    },
    Scroll {
        vertical: i32,
        horizontal: i32,
    },
    /// High-resolution wheel units;120 equals one detent.
    ScrollFine {
        vertical: i32,
        horizontal: i32,
    },
    /// Neutralize every held button without stopping the writer.
    Release,
    Heartbeat,
    Stop,
}

#[derive(Default)]
pub struct MouseState {
    held: [bool; 3],
    residual: [f64; 2],
    wheel_residual: [i32; 2],
}
impl MouseState {
    pub(crate) fn is_held(&self) -> bool {
        self.held.iter().any(|held| *held)
    }
    fn button(&mut self, button: MouseButton, down: bool) -> Vec<InputEvent> {
        let (index, code) = match button {
            MouseButton::Left => (0, KeyCode::BTN_LEFT),
            MouseButton::Right => (1, KeyCode::BTN_RIGHT),
            MouseButton::Middle => (2, KeyCode::BTN_MIDDLE),
        };
        if std::mem::replace(&mut self.held[index], down) == down {
            Vec::new()
        } else {
            vec![InputEvent::new(EventType::KEY.0, code.0, i32::from(down))]
        }
    }
    pub fn apply(&mut self, intent: MouseIntent, gain: f64) -> io::Result<Vec<InputEvent>> {
        let mut events = Vec::new();
        match intent {
            MouseIntent::Move { dx, dy } => {
                if !gain.is_finite()
                    || !(1.0..=10_000.0).contains(&gain)
                    || !dx.is_finite()
                    || !dy.is_finite()
                    || dx.abs() > 1.0
                    || dy.abs() > 1.0
                {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "invalid movement/gain",
                    ));
                }
                for (i, delta, code) in [
                    (0, dx, RelativeAxisCode::REL_X),
                    (1, dy, RelativeAxisCode::REL_Y),
                ] {
                    self.residual[i] += delta * gain;
                    let integer = self.residual[i].trunc() as i32;
                    self.residual[i] -= f64::from(integer);
                    if integer != 0 {
                        events.push(InputEvent::new(EventType::RELATIVE.0, code.0, integer));
                    }
                }
            }
            MouseIntent::Absolute { x, y } => {
                if !x.is_finite()
                    || !y.is_finite()
                    || !(0.0..=1.0).contains(&x)
                    || !(0.0..=1.0).contains(&y)
                {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "invalid absolute pointer position",
                    ));
                }
                for (value, code) in [(x, AbsoluteAxisCode::ABS_X), (y, AbsoluteAxisCode::ABS_Y)] {
                    events.push(InputEvent::new(
                        EventType::ABSOLUTE.0,
                        code.0,
                        (value * 65535.0).round() as i32,
                    ));
                }
            }
            MouseIntent::Button { down } => return Ok(self.button(MouseButton::Left, down)),
            MouseIntent::NamedButton { button, down } => return Ok(self.button(button, down)),
            MouseIntent::Scroll {
                vertical,
                horizontal,
            } => {
                if vertical.abs_diff(0) > 10 || horizontal.abs_diff(0) > 10 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "wheel intent exceeds 10 steps",
                    ));
                }
                for (value, code, fine) in [
                    (
                        vertical,
                        RelativeAxisCode::REL_WHEEL,
                        RelativeAxisCode::REL_WHEEL_HI_RES,
                    ),
                    (
                        horizontal,
                        RelativeAxisCode::REL_HWHEEL,
                        RelativeAxisCode::REL_HWHEEL_HI_RES,
                    ),
                ] {
                    if value != 0 {
                        events.push(InputEvent::new(EventType::RELATIVE.0, fine.0, value * 120));
                        events.push(InputEvent::new(EventType::RELATIVE.0, code.0, value));
                    }
                }
            }
            MouseIntent::ScrollFine {
                vertical,
                horizontal,
            } => {
                if vertical.abs_diff(0) > 1200 || horizontal.abs_diff(0) > 1200 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "fine wheel intent exceeds ten detents",
                    ));
                }
                for (axis, value, code, fine) in [
                    (
                        0,
                        horizontal,
                        RelativeAxisCode::REL_HWHEEL,
                        RelativeAxisCode::REL_HWHEEL_HI_RES,
                    ),
                    (
                        1,
                        vertical,
                        RelativeAxisCode::REL_WHEEL,
                        RelativeAxisCode::REL_WHEEL_HI_RES,
                    ),
                ] {
                    if value != 0 {
                        events.push(InputEvent::new(EventType::RELATIVE.0, fine.0, value));
                        self.wheel_residual[axis] += value;
                        let detents = self.wheel_residual[axis] / 120;
                        self.wheel_residual[axis] -= detents * 120;
                        if detents != 0 {
                            events.push(InputEvent::new(EventType::RELATIVE.0, code.0, detents));
                        }
                    }
                }
            }
            MouseIntent::Heartbeat => (),
            MouseIntent::Release | MouseIntent::Stop => return Ok(self.release()),
        }
        Ok(events)
    }
    pub fn release(&mut self) -> Vec<InputEvent> {
        self.residual = [0.0; 2];
        self.wheel_residual = [0; 2];
        [MouseButton::Left, MouseButton::Right, MouseButton::Middle]
            .into_iter()
            .flat_map(|button| self.button(button, false))
            .collect()
    }
}
pub(crate) fn create(absolute: bool) -> io::Result<VirtualDevice> {
    let builder = VirtualDevice::builder()?;
    if absolute {
        builder
            .name("WiggleControl Absolute Mouse")
            .with_keys(&AttributeSet::from_iter([
                KeyCode::BTN_LEFT,
                KeyCode::BTN_RIGHT,
                KeyCode::BTN_MIDDLE,
            ]))?
            .with_properties(&AttributeSet::from_iter([PropType::POINTER]))?
            .with_absolute_axis(&UinputAbsSetup::new(
                AbsoluteAxisCode::ABS_X,
                AbsInfo::new(0, 0, 65535, 0, 0, 0),
            ))?
            .with_absolute_axis(&UinputAbsSetup::new(
                AbsoluteAxisCode::ABS_Y,
                AbsInfo::new(0, 0, 65535, 0, 0, 0),
            ))?
            .with_relative_axes(&AttributeSet::from_iter([
                RelativeAxisCode::REL_WHEEL,
                RelativeAxisCode::REL_HWHEEL,
                RelativeAxisCode::REL_WHEEL_HI_RES,
                RelativeAxisCode::REL_HWHEEL_HI_RES,
            ]))?
            .build()
    } else {
        builder
            .name("WiggleControl Mouse")
            .with_keys(&AttributeSet::from_iter([
                KeyCode::BTN_LEFT,
                KeyCode::BTN_RIGHT,
                KeyCode::BTN_MIDDLE,
            ]))?
            .with_relative_axes(&AttributeSet::from_iter([
                RelativeAxisCode::REL_X,
                RelativeAxisCode::REL_Y,
                RelativeAxisCode::REL_WHEEL,
                RelativeAxisCode::REL_HWHEEL,
                RelativeAxisCode::REL_WHEEL_HI_RES,
                RelativeAxisCode::REL_HWHEEL_HI_RES,
            ]))?
            .build()
    }
}
fn duration(seconds: u64) -> io::Result<Duration> {
    if !(1..=120).contains(&seconds) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "duration must be 1..120 seconds",
        ));
    }
    Ok(Duration::from_secs(seconds))
}
pub fn check(seconds: u64) -> io::Result<()> {
    let duration = duration(seconds)?;
    let mut device = create(false)?;
    for path in device.enumerate_dev_nodes_blocking()? {
        eprintln!("Created WiggleControl mouse: {}", path?.display());
    }
    eprintln!("Device check only: no movement/button/wheel events. Expires after {seconds}s.");
    std::thread::sleep(duration);
    drop(device);
    Ok(())
}
pub fn stream(seconds: u64, gain: f64) -> io::Result<()> {
    duration(seconds)?;
    stream_mode(seconds, gain, false, 150)
}
pub fn absolute_stream(seconds: u64) -> io::Result<()> {
    duration(seconds)?;
    stream_mode(seconds, 1.0, true, 150)
}
pub fn stream_with_watchdog(
    seconds: u64,
    gain: f64,
    absolute: bool,
    watchdog_ms: u64,
) -> io::Result<()> {
    duration(seconds)?;
    stream_mode(seconds, gain, absolute, watchdog_ms)
}
pub fn absolute_until_stop(watchdog_ms: u64) -> io::Result<()> {
    stream_mode(0, 1.0, true, watchdog_ms)
}
fn stream_mode(seconds: u64, gain: f64, absolute: bool, watchdog_ms: u64) -> io::Result<()> {
    if !(50..=1000).contains(&watchdog_ms) {
        return Err(io::Error::other("watchdog_ms must be 50..1000"));
    }
    let limit = if seconds == 0 {
        Duration::MAX
    } else {
        duration(seconds)?
    };
    if !gain.is_finite() || !(1.0..=10_000.0).contains(&gain) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "gain must be 1..10000",
        ));
    }
    let mut device = create(absolute)?;
    let rx = crate::intent::reader::<MouseIntent>();
    let start = Instant::now();
    let mut fresh = Instant::now();
    let mut state = MouseState::default();
    let result = (|| {
        while start.elapsed() < limit {
            match rx
                .recv_timeout(Duration::from_millis(20).min(limit.saturating_sub(start.elapsed())))
            {
                Ok(Ok(intent)) => {
                    if matches!(intent, MouseIntent::Absolute { .. }) && !absolute
                        || matches!(intent, MouseIntent::Move { .. }) && absolute
                    {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "pointer intent does not match device mode",
                        ));
                    }
                    let stop = matches!(intent, MouseIntent::Stop);
                    device.emit(&state.apply(intent, gain)?)?;
                    fresh = Instant::now();
                    if stop {
                        break;
                    }
                }
                Ok(Err(e)) => return Err(e),
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
                Err(mpsc::RecvTimeoutError::Timeout) => (),
            }
            if fresh.elapsed() >= Duration::from_millis(watchdog_ms) {
                let release = state.release();
                if !release.is_empty() {
                    device.emit(&release)?;
                    eprintln!("Released held button: producer watchdog");
                }
            }
        }
        Ok(())
    })();
    let release = device.emit(&state.release());
    // Closing the sole uinput fd removes this virtual device, including on errors.
    drop(device);
    result.and(release)
}
