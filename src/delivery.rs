//! Independent, age-checked writer for exclusive controller output.
use crate::{
    controller::{Mode, Output},
    input::{MouseIntent, MouseState},
    touch::{Contact, State as TouchState},
    Event,
};
use evdev::InputEvent;
use serde::{Deserialize, Serialize};
use std::{
    io::{self, Write},
    sync::mpsc,
    time::{Duration, Instant},
};

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Packet {
    Transition {
        sequence: u64,
        issued_us: u64,
        mode: Mode,
        enabled: bool,
    },
    Frame {
        sequence: u64,
        issued_us: u64,
        source_us: u64,
        mode: Mode,
        mouse: Vec<Event>,
        contacts: Vec<Contact>,
    },
    Stop,
}
#[derive(Default)]
pub struct Publisher {
    sequence: u64,
}
impl Publisher {
    pub fn packet(
        &mut self,
        output: &Output,
        now: u64,
        source: Option<u64>,
    ) -> io::Result<Option<Packet>> {
        if !output.transition && (!output.running || source.is_none()) {
            return Ok(None);
        }
        self.sequence = self
            .sequence
            .checked_add(1)
            .ok_or_else(|| io::Error::other("delivery sequence exhausted"))?;
        Ok(Some(if output.transition {
            Packet::Transition {
                sequence: self.sequence,
                issued_us: now,
                mode: output.mode,
                enabled: output.running,
            }
        } else {
            Packet::Frame {
                sequence: self.sequence,
                issued_us: now,
                source_us: source.unwrap(),
                mode: output.mode,
                mouse: output.mouse.clone(),
                contacts: output.contacts.clone(),
            }
        }))
    }
}
#[derive(Default)]
pub struct Batch {
    pub mouse: Vec<InputEvent>,
    pub touch: Vec<InputEvent>,
}
#[derive(Default)]
pub struct State {
    mouse: MouseState,
    touch: TouchState,
    mode: Option<Mode>,
    ready: bool,
    sequence: Option<u64>,
    source: Option<u64>,
    now: Option<u64>,
    received: Option<u64>,
}
fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}
impl State {
    pub fn release(&mut self) -> Batch {
        self.ready = false;
        Batch {
            mouse: self.mouse.release(),
            touch: self.touch.release(),
        }
    }
    pub fn tick(&mut self, now: u64, watchdog_us: u64) -> io::Result<Batch> {
        if self.now.is_some_and(|t| now < t) {
            return Err(invalid("delivery clock regressed"));
        }
        self.now = Some(now);
        Ok(
            if self
                .received
                .is_some_and(|t| now.saturating_sub(t) >= watchdog_us)
            {
                self.release()
            } else {
                Batch::default()
            },
        )
    }
    pub fn apply(
        &mut self,
        now: u64,
        packet: Packet,
        gain: f64,
        watchdog_us: u64,
    ) -> io::Result<Batch> {
        if !gain.is_finite()
            || !(1.0..=10_000.0).contains(&gain)
            || !(50_000..=1_000_000).contains(&watchdog_us)
        {
            return Err(invalid("invalid delivery gain/watchdog"));
        }
        // A watchdog release is observable even if the next packet is invalid.
        // The stream treats any rejection as terminal and also releases on exit.
        if self.now.is_some_and(|t| now < t) {
            return Err(invalid("delivery clock regressed"));
        }
        self.now = Some(now);
        if self
            .received
            .is_some_and(|t| now.saturating_sub(t) >= watchdog_us)
        {
            self.ready = false;
        }
        let (sequence, issued) = match &packet {
            Packet::Transition {
                sequence,
                issued_us,
                ..
            }
            | Packet::Frame {
                sequence,
                issued_us,
                ..
            } => (*sequence, *issued_us),
            Packet::Stop => return Ok(self.release()),
        };
        if self.sequence.is_some_and(|last| sequence <= last)
            || issued > now
            || now - issued >= watchdog_us
        {
            return Err(invalid("old, future or stale delivery packet"));
        }
        self.sequence = Some(sequence);
        self.received = Some(now);
        match packet {
            Packet::Transition { mode, enabled, .. } => {
                let batch = self.release();
                self.mode = Some(mode);
                self.ready = enabled;
                Ok(batch)
            }
            Packet::Frame {
                source_us,
                mode,
                mouse,
                contacts,
                ..
            } => {
                if !self.ready
                    || self.mode != Some(mode)
                    || source_us > issued
                    || now - source_us >= watchdog_us
                    || self.source.is_some_and(|t| source_us <= t)
                    || mouse.len() > 8
                    || contacts.len() > 2
                {
                    return Err(invalid(
                        "delivery requires fresh source and an explicit armed mode",
                    ));
                }
                let allowed = match mode {
                    Mode::OneContact => mouse.is_empty() && contacts.len() <= 1,
                    Mode::TwoContacts => mouse.is_empty(),
                    Mode::Scroll => {
                        contacts.is_empty()
                            && mouse.iter().all(|e| {
                                matches!(
                                    e,
                                    Event::Scroll { .. }
                                        | Event::Cancel { .. }
                                        | Event::Button { down: false }
                                )
                            })
                    }
                    Mode::Pointer | Mode::Precision => {
                        contacts.is_empty()
                            && mouse.iter().all(|e| !matches!(e, Event::Scroll { .. }))
                    }
                };
                if !allowed {
                    return Err(invalid("competing or incompatible mode output"));
                }
                self.source = Some(source_us);
                let mut batch = Batch::default();
                for event in mouse {
                    let events = match event {
                        Event::Move { dx, dy } => {
                            self.mouse.apply(MouseIntent::Move { dx, dy }, gain)?
                        }
                        Event::Button { down } => {
                            self.mouse.apply(MouseIntent::Button { down }, gain)?
                        }
                        Event::Scroll { vertical } => self.mouse.apply(
                            MouseIntent::Scroll {
                                vertical,
                                horizontal: 0,
                            },
                            gain,
                        )?,
                        Event::Cancel { .. } => self.mouse.release(),
                    };
                    batch.mouse.extend(events);
                }
                batch.touch = self.touch.frame(&contacts)?;
                Ok(batch)
            }
            Packet::Stop => unreachable!(),
        }
    }
}

/// Manually invoked device writer. None is an explicit until-stop session.
pub fn stream(seconds: Option<u64>, gain: f64, watchdog_ms: u64) -> io::Result<()> {
    if seconds.is_some_and(|n| !(1..=120).contains(&n))
        || !(50..=1000).contains(&watchdog_ms)
        || !gain.is_finite()
        || !(1.0..=10_000.0).contains(&gain)
    {
        return Err(invalid("invalid delivery duration/gain/watchdog"));
    }
    let mut mouse = crate::input::create(false)?;
    let mut touch = crate::touch::create()?;
    let mut state = State::default();
    let rx = crate::intent::reader::<Packet>();
    let start = Instant::now();
    let limit = seconds.map(Duration::from_secs).unwrap_or(Duration::MAX);
    let emit = |batch: Batch,
                mouse: &mut evdev::uinput::VirtualDevice,
                touch: &mut evdev::uinput::VirtualDevice|
     -> io::Result<()> {
        if !batch.mouse.is_empty() {
            mouse.emit(&batch.mouse)?;
        }
        if !batch.touch.is_empty() {
            touch.emit(&batch.touch)?;
        }
        Ok(())
    };
    let result = (|| {
        while start.elapsed() < limit {
            match rx
                .recv_timeout(Duration::from_millis(20).min(limit.saturating_sub(start.elapsed())))
            {
                Ok(Ok(packet)) => {
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
                )?,
            }
        }
        Ok(())
    })();
    let release = emit(state.release(), &mut mouse, &mut touch);
    drop(mouse);
    drop(touch);
    result.and(release)
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
            serde_json::json!({"schema":"wigglecontrol.delivery-output/v1","camera_opened":false,"system_input":false,"mouse":values(batch.mouse),"touch":values(batch.touch)})
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
    use crate::controller::{Command, Controller, Settings};
    if !(50..=1000).contains(&watchdog_ms) || !gain.is_finite() || !(1.0..=10_000.0).contains(&gain)
    {
        return Err(invalid("invalid pipeline gain/watchdog"));
    }
    let mut controller = Controller::new(Settings::load(settings)?).map_err(io::Error::other)?;
    let mut publisher = Publisher::default();
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
            serde_json::json!({"schema":"wigglecontrol.pipeline-output/v1","camera_opened":false,"system_input":false,
            "controller":output,"packet":packet,"mouse":values(batch.mouse),"touch":values(batch.touch)})
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
        let mut batch = match state.tick(now, watchdog_ms * 1000) {
            Ok(batch) => batch,
            Err(error) => {
                publish(controller.stop(), None, state.release())?;
                return Err(error);
            }
        };
        let packet = publisher.packet(&output, now, source)?;
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
