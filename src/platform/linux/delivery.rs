//! Linux independent device writer and replay adapter over the common output core.
pub use crate::output::{Packet, Publisher};
use crate::{
    controller::{Mode, Output},
    input::{MouseIntent, MouseState},
    touch::{Contact, State as TouchState},
    Event,
};
use evdev::InputEvent;
use serde::Deserialize;
use std::{
    io::{self, Write},
    sync::mpsc,
    time::{Duration, Instant},
};
pub type State = crate::output::State<MouseState, TouchState>;
pub type Batch = crate::output::Batch<MouseState, TouchState>;
fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}
impl crate::output::MouseBackend for MouseState {
    type Event = InputEvent;
    fn apply(&mut self, intent: MouseIntent, gain: f64) -> io::Result<Vec<InputEvent>> {
        MouseState::apply(self, intent, gain)
    }
    fn release(&mut self) -> Vec<InputEvent> {
        MouseState::release(self)
    }
    fn is_held(&self) -> bool {
        MouseState::is_held(self)
    }
}
impl crate::output::TouchBackend for TouchState {
    type Event = InputEvent;
    fn frame(&mut self, contacts: &[Contact]) -> io::Result<Vec<InputEvent>> {
        TouchState::frame(self, contacts)
    }
    fn release(&mut self) -> Vec<InputEvent> {
        TouchState::release(self)
    }
}
/// Manually invoked device writer. None is an explicit until-stop session.
pub fn stream(seconds: Option<u64>, gain: f64, watchdog_ms: u64) -> io::Result<()> {
    stream_mode(seconds, gain, watchdog_ms, false)
}
pub fn absolute_stream(seconds: Option<u64>, gain: f64, watchdog_ms: u64) -> io::Result<()> {
    stream_mode(seconds, gain, watchdog_ms, true)
}
fn stream_mode(
    seconds: Option<u64>,
    gain: f64,
    watchdog_ms: u64,
    absolute: bool,
) -> io::Result<()> {
    if seconds.is_some_and(|n| !(1..=120).contains(&n))
        || !(50..=1000).contains(&watchdog_ms)
        || !gain.is_finite()
        || !(1.0..=10_000.0).contains(&gain)
    {
        return Err(invalid("invalid delivery duration/gain/watchdog"));
    }
    let mut mouse = crate::input::create(absolute)?;
    // Create before gesture acquisition so the compositor can discover it.
    // Receiving installation must classify this fixed-key device as a keyboard.
    let mut keyboard = crate::keyboard::create()?;
    let mut touch = if absolute {
        None
    } else {
        Some(crate::touch::create()?)
    };
    let mut state = State::default();
    let rx = crate::intent::reader::<Packet>();
    let start = Instant::now();
    let limit = seconds.map(Duration::from_secs).unwrap_or(Duration::MAX);
    let emit = |batch: Batch,
                mouse: &mut evdev::uinput::VirtualDevice,
                touch: &mut Option<evdev::uinput::VirtualDevice>,
                keyboard: &mut evdev::uinput::VirtualDevice|
     -> io::Result<()> {
        if !batch.mouse.is_empty() {
            mouse.emit(&batch.mouse)?;
        }
        if !batch.touch.is_empty() {
            touch
                .as_mut()
                .ok_or_else(|| invalid("touch output in absolute mouse session"))?
                .emit(&batch.touch)?;
        }
        if batch.escape {
            crate::keyboard::tap(|events| keyboard.emit(events))?;
        }
        Ok(())
    };
    let result = (|| {
        while start.elapsed() < limit {
            match rx
                .recv_timeout(Duration::from_millis(20).min(limit.saturating_sub(start.elapsed())))
            {
                Ok(Ok(packet)) => {
                    if let Packet::Transition { mode, .. }
                    | Packet::Frame { mode, .. }
                    | Packet::Hold { mode, .. }
                    | Packet::Cursor { mode, .. }
                    | Packet::Escape { mode, .. } = &packet
                    {
                        if absolute && !matches!(mode, Mode::Pointer | Mode::Scroll) {
                            return Err(invalid("unsupported absolute mouse mode"));
                        }
                    }
                    if let Packet::Frame {
                        absolute: target,
                        mouse,
                        ..
                    } = &packet
                    {
                        if (!absolute && target.is_some())
                            || (absolute && mouse.iter().any(|e| matches!(e, Event::Move { .. })))
                        {
                            return Err(invalid("pointer packet does not match device"));
                        }
                    }
                    let stop = matches!(packet, Packet::Stop);
                    emit(
                        state.apply(
                            crate::intent::monotonic_us()?,
                            packet,
                            gain,
                            watchdog_ms * 1000,
                        )?,
                        &mut mouse,
                        &mut touch,
                        &mut keyboard,
                    )?;
                    if stop {
                        break;
                    }
                }
                Ok(Err(e)) => return Err(e),
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
                Err(mpsc::RecvTimeoutError::Timeout) => emit(
                    state.tick(crate::intent::monotonic_us()?, watchdog_ms * 1000)?,
                    &mut mouse,
                    &mut touch,
                    &mut keyboard,
                )?,
            }
        }
        Ok(())
    })();
    let release = emit(state.release(), &mut mouse, &mut touch, &mut keyboard);
    let key_release = keyboard.emit(&[crate::keyboard::escape_events()[1]]);
    drop(mouse);
    drop(touch);
    drop(keyboard);
    result.and(release).and(key_release)
}

/// Device-free protocol verification with independent synthetic consumer time.
pub fn replay_stdin(gain: f64, watchdog_ms: u64) -> io::Result<()> {
    #[derive(Deserialize)]
    #[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
    enum Replay {
        Packet { now_us: u64, packet: Packet },
        Tick { now_us: u64 },
    }
    let mut state = State::default();
    let mut out = io::stdout().lock();
    let mut publish = |batch: Batch| -> io::Result<()> {
        let values = |events: Vec<InputEvent>| {
            events.into_iter().map(|e|serde_json::json!({"kind":e.event_type().0,"code":e.code(),"value":e.value()})).collect::<Vec<_>>()
        };
        writeln!(
            out,
            "{}",
            serde_json::json!({"schema":"wigglecontrol.delivery-output/v1","camera_opened":false,"system_input":false,"mouse":values(batch.mouse),"touch":values(batch.touch),
                "keyboard":if batch.escape {values(crate::keyboard::escape_events().to_vec())} else {Vec::new()}})
        )?;
        out.flush()
    };
    if !(50..=1000).contains(&watchdog_ms) || !gain.is_finite() || !(1.0..=10_000.0).contains(&gain)
    {
        return Err(invalid("invalid replay gain/watchdog"));
    }
    for command in crate::intent::reader::<Replay>() {
        let result = match command {
            Ok(Replay::Packet { now_us, packet }) => {
                state.apply(now_us, packet, gain, watchdog_ms * 1000)
            }
            Ok(Replay::Tick { now_us }) => state.tick(now_us, watchdog_ms * 1000),
            Err(e) => Err(e),
        };
        match result {
            Ok(batch) => publish(batch)?,
            Err(error) => {
                publish(state.release())?;
                return Err(error);
            }
        }
    }
    publish(state.release())
}

/// Full controller-to-device-event pipeline, with replay clocks and no device fds.
pub fn pipeline_replay_stdin(
    settings: &std::path::Path,
    gain: f64,
    watchdog_ms: u64,
) -> io::Result<()> {
    pipeline_replay(settings, gain, watchdog_ms, false)
}

/// Exercise the same calibrated absolute publisher as Beta1, without device fds.
pub fn absolute_pipeline_replay_stdin(
    settings: &std::path::Path,
    gain: f64,
    watchdog_ms: u64,
) -> io::Result<()> {
    pipeline_replay(settings, gain, watchdog_ms, true)
}

fn pipeline_replay(
    settings: &std::path::Path,
    gain: f64,
    watchdog_ms: u64,
    absolute: bool,
) -> io::Result<()> {
    use crate::controller::{Command, Controller, Settings};
    if !(50..=1000).contains(&watchdog_ms) || !gain.is_finite() || !(1.0..=10_000.0).contains(&gain)
    {
        return Err(invalid("invalid pipeline gain/watchdog"));
    }
    let settings = Settings::load(settings)?;
    let workspace = settings.workspace;
    let cursor_output = settings.cursor_output;
    let freshness_us = settings.gestures.stale_us.min(watchdog_ms * 1000);
    if cursor_output.is_some() && !absolute {
        return Err(invalid("cursor output replay requires --absolute"));
    }
    let mut controller = Controller::new(settings).map_err(io::Error::other)?;
    let mut publisher = Publisher::with_cursor_output(cursor_output, freshness_us)?;
    let mut state = State::default();
    let mut now = 0;
    let mut out = io::stdout().lock();
    let mut publish = |output: Output, packet: Option<Packet>, batch: Batch| -> io::Result<()> {
        let values = |events: Vec<InputEvent>| {
            events.into_iter().map(|e|serde_json::json!({"kind":e.event_type().0,"code":e.code(),"value":e.value()})).collect::<Vec<_>>()
        };
        writeln!(
            out,
            "{}",
            serde_json::json!({"schema":"wigglecontrol.pipeline-output/v1","camera_opened":false,"system_input":false,"absolute_pointer":absolute,
            "controller":output,"packet":packet,"mouse":values(batch.mouse),"touch":values(batch.touch),
            "keyboard":if batch.escape {values(crate::keyboard::escape_events().to_vec())} else {Vec::new()}})
        )?;
        out.flush()
    };
    for command in crate::intent::reader::<Command>() {
        let command = match command {
            Ok(c) => c,
            Err(error) => {
                publish(controller.stop(), None, state.release())?;
                return Err(error);
            }
        };
        let source = match &command {
            Command::Frame {
                now_us,
                observation,
                ..
            } => {
                now = *now_us;
                Some(observation.source_us)
            }
            Command::Tick { now_us } => {
                now = *now_us;
                None
            }
            _ => None,
        };
        let output = controller.command(command);
        // Match the live provider's final freshness fence before publication.
        // Replay's supplied clock is logical; this is not measured IPC latency.
        let (output, _) = controller.guard_delivery(output, now, source);
        let mut batch = match state.tick(now, watchdog_ms * 1000) {
            Ok(batch) => batch,
            Err(error) => {
                publish(controller.stop(), None, state.release())?;
                return Err(error);
            }
        };
        let packet = if absolute {
            publisher.absolute_packet(&output, now, source, workspace)
        } else {
            publisher.packet(&output, now, source)
        };
        let packet = match packet {
            Ok(packet) => packet,
            Err(error) => {
                publish(controller.stop(), None, state.release())?;
                return Err(error);
            }
        };
        if let Some(packet) = &packet {
            let decoded = serde_json::from_slice(&serde_json::to_vec(packet)?)?;
            match state.apply(now, decoded, gain, watchdog_ms * 1000) {
                Ok(next) => {
                    batch.mouse.extend(next.mouse);
                    batch.touch.extend(next.touch);
                }
                Err(error) => {
                    let released = state.release();
                    batch.mouse.extend(released.mouse);
                    batch.touch.extend(released.touch);
                    publish(controller.stop(), None, batch)?;
                    return Err(error);
                }
            }
        }
        publish(output, packet, batch)?;
    }
    publish(controller.stop(), None, state.release())
}
