//! Linux mouse output. No camera, compositor control or privileged inference.
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
    Move { dx: f64, dy: f64 },
    Absolute { x: f64, y: f64 },
    Button { down: bool },
    Scroll { vertical: i32, horizontal: i32 },
    Heartbeat,
    Stop,
}

#[derive(Default)]
pub struct MouseState {
    held: bool,
    residual: [f64; 2],
}
impl MouseState {
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
            MouseIntent::Button { down } => {
                if down != self.held {
                    events.push(InputEvent::new(
                        EventType::KEY.0,
                        KeyCode::BTN_LEFT.0,
                        i32::from(down),
                    ));
                }
                self.held = down;
            }
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
                for (value, code) in [
                    (vertical, RelativeAxisCode::REL_WHEEL),
                    (horizontal, RelativeAxisCode::REL_HWHEEL),
                ] {
                    if value != 0 {
                        events.push(InputEvent::new(EventType::RELATIVE.0, code.0, value));
                    }
                }
            }
            MouseIntent::Heartbeat => (),
            MouseIntent::Stop => return Ok(self.release()),
        }
        Ok(events)
    }
    pub fn release(&mut self) -> Vec<InputEvent> {
        self.residual = [0.0; 2];
        if std::mem::take(&mut self.held) {
            vec![InputEvent::new(EventType::KEY.0, KeyCode::BTN_LEFT.0, 0)]
        } else {
            Vec::new()
        }
    }
}
fn create(absolute: bool) -> io::Result<VirtualDevice> {
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
            ]))?
            .build()
    } else {
        builder
            .name("WiggleControl Mouse")
            .with_keys(&AttributeSet::from_iter([KeyCode::BTN_LEFT]))?
            .with_relative_axes(&AttributeSet::from_iter([
                RelativeAxisCode::REL_X,
                RelativeAxisCode::REL_Y,
                RelativeAxisCode::REL_WHEEL,
                RelativeAxisCode::REL_HWHEEL,
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
    stream_mode(seconds, gain, false)
}
pub fn absolute_stream(seconds: u64) -> io::Result<()> {
    stream_mode(seconds, 1.0, true)
}
fn stream_mode(seconds: u64, gain: f64, absolute: bool) -> io::Result<()> {
    let limit = duration(seconds)?;
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
            if fresh.elapsed() >= Duration::from_millis(150) {
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
