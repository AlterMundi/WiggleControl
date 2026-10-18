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
        #[serde(default, skip_serializing_if = "Option::is_none")]
        absolute: Option<crate::Point>,
        contacts: Vec<Contact>,
    },
    /// Estimator liveness plus a fixed last-observation deadline; no motion/press.
    Hold {
        sequence: u64,
        issued_us: u64,
        source_us: u64,
        until_us: u64,
        mode: Mode,
    },
    /// Derived motion anchored to the last delivered absolute observation.
    Cursor {
        sequence: u64,
        issued_us: u64,
        source_us: u64,
        mode: Mode,
        absolute: crate::Point,
    },
    /// Fresh explicit one-shot cancellation: release owned input, then tap Esc.
    Escape {
        sequence: u64,
        issued_us: u64,
        source_us: u64,
        mode: Mode,
    },
    Stop,
}
#[derive(Default)]
pub struct Publisher {
    sequence: u64,
    cursor: Option<crate::cursor::Cursor>,
    cursor_owner: Option<u64>,
    cursor_held: bool,
    cursor_freshness_us: u64,
    detail_map: crate::detail::Mapping,
    mapping_owner: Option<u64>,
}
impl Publisher {
    pub fn with_cursor_output(
        config: Option<crate::cursor::Config>,
        freshness_us: u64,
    ) -> io::Result<Self> {
        if config.is_some() && !(50_000..=1_000_000).contains(&freshness_us) {
            return Err(invalid(
                "cursor output requires the writer source-freshness budget",
            ));
        }
        Ok(Self {
            cursor: config
                .map(crate::cursor::Cursor::new)
                .transpose()
                .map_err(io::Error::other)?,
            cursor_freshness_us: freshness_us,
            ..Self::default()
        })
    }
    pub fn cursor_feedback(&self) -> Option<crate::cursor::Sample> {
        self.cursor
            .as_ref()
            .and_then(crate::cursor::Cursor::feedback)
    }
    pub fn packet(
        &mut self,
        output: &Output,
        now: u64,
        source: Option<u64>,
    ) -> io::Result<Option<Packet>> {
        let hold = output
            .drag_hold
            .filter(|h| output.button_down && (h.in_grace || source.is_none()));
        if !output.transition && (!output.running || (source.is_none() && hold.is_none())) {
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
        } else if let Some(hold) = hold {
            Packet::Hold {
                sequence: self.sequence,
                issued_us: now,
                source_us: hold.source_us,
                until_us: hold.until_us,
                mode: output.mode,
            }
        } else {
            Packet::Frame {
                sequence: self.sequence,
                issued_us: now,
                source_us: source.unwrap(),
                mode: output.mode,
                mouse: output.mouse.clone(),
                absolute: None,
                contacts: output.contacts.clone(),
            }
        }))
    }
    /// Reuse the source-age/transition contract for a calibrated absolute mouse.
    pub fn absolute_packet(
        &mut self,
        output: &Output,
        now: u64,
        source: Option<u64>,
        workspace: crate::touch_gestures::Workspace,
    ) -> io::Result<Option<Packet>> {
        workspace.validate().map_err(io::Error::other)?;
        if !matches!(output.mode, Mode::Pointer | Mode::Scroll) {
            return Err(invalid(
                "absolute pointer supports pointer/scroll modes only",
            ));
        }
        let mut packet = self.packet(output, now, source)?;
        if output.transition
            || !output.running
            || output.mode != Mode::Pointer
            || self.mapping_owner != output.primary_hand
        {
            self.detail_map.reset();
            self.mapping_owner = output.primary_hand;
        }
        let mapped = if output.mode == Mode::Pointer && matches!(packet, Some(Packet::Frame { .. }))
        {
            output
                .pointer_cursor
                .map(|point| {
                    self.detail_map
                        .observe(
                            workspace.map(point),
                            output.detail,
                            self.cursor
                                .as_ref()
                                .and_then(|c| c.feedback())
                                .map(|s| s.estimated),
                        )
                        .map_err(io::Error::other)
                })
                .transpose()?
        } else {
            None
        };
        if let Some(cursor) = &mut self.cursor {
            let was_held = self.cursor_held;
            self.cursor_held = output.running && output.button_down;
            if output.transition || !output.running || output.mode != Mode::Pointer {
                cursor.reset();
                self.cursor_owner = None;
            } else if output.drag_hold.is_some_and(|h| h.in_grace) {
                cursor.freeze(now).map_err(io::Error::other)?;
            } else if source.is_none()
                && cursor.feedback().is_some_and(|sample| {
                    now.saturating_sub(sample.source_us)
                        >= self.cursor_freshness_us.saturating_sub(20_000)
                })
            {
                // Reserve one writer poll interval for derived-motion transport.
                // Existing hold liveness may continue, but it must not move stale data.
                cursor.freeze(now).map_err(io::Error::other)?;
            } else if let Some(Packet::Frame {
                absolute, mouse, ..
            }) = &mut packet
            {
                mouse.retain(|e| !matches!(e, Event::Move { .. }));
                if let Some(point) = output.pointer_cursor {
                    if self.cursor_owner != output.primary_hand {
                        cursor.reset();
                        if self.cursor_owner.is_some() && was_held {
                            self.cursor_owner = output.primary_hand;
                            if output.button_down {
                                return Err(invalid("cursor owner changed during held output"));
                            }
                            // Deliver release at the old visible point before moving a new owner.
                            return Ok(packet);
                        }
                        self.cursor_owner = output.primary_hand;
                    }
                    let (point, changed) = mapped.unwrap_or((workspace.map(point), false));
                    if changed {
                        cursor.reanchor(point, now).map_err(io::Error::other)?;
                    }
                    cursor
                        .observe(
                            source.unwrap(),
                            now,
                            point,
                            output.pointer_state == "closing",
                            output.button_down,
                        )
                        .map_err(io::Error::other)?;
                    *absolute = cursor
                        .sample(now, true)
                        .map_err(io::Error::other)?
                        .map(|s| s.estimated);
                } else {
                    cursor.reset();
                    self.cursor_owner = None;
                }
                return Ok(packet);
            } else if let Some(sample) = cursor.sample(now, false).map_err(io::Error::other)? {
                if packet.is_none() {
                    self.sequence = self
                        .sequence
                        .checked_add(1)
                        .ok_or_else(|| io::Error::other("delivery sequence exhausted"))?;
                }
                return Ok(Some(Packet::Cursor {
                    sequence: self.sequence,
                    issued_us: now,
                    source_us: sample.source_us,
                    mode: Mode::Pointer,
                    absolute: sample.estimated,
                }));
            }
            return Ok(packet);
        }
        if let Some(Packet::Frame {
            mouse, absolute, ..
        }) = &mut packet
        {
            mouse.retain(|e| !matches!(e, Event::Move { .. }));
            if output.mode == Mode::Pointer {
                *absolute = mapped.map(|(point, _)| point);
            }
        }
        Ok(packet)
    }
}
#[derive(Default)]
pub struct Batch {
    pub mouse: Vec<InputEvent>,
    pub touch: Vec<InputEvent>,
    pub escape: bool,
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
    hold_until: Option<u64>,
    absolute_source: bool,
}
fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}
impl State {
    pub fn release(&mut self) -> Batch {
        self.ready = false;
        self.hold_until = None;
        self.absolute_source = false;
        Batch {
            mouse: self.mouse.release(),
            touch: self.touch.release(),
            escape: false,
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
                || self.hold_until.is_some_and(|deadline| now >= deadline)
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
        let previously_received = self.received;
        let previously_armed = self.ready;
        self.now = Some(now);
        if self
            .received
            .is_some_and(|t| now.saturating_sub(t) >= watchdog_us)
            || self.hold_until.is_some_and(|deadline| now >= deadline)
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
            }
            | Packet::Hold {
                sequence,
                issued_us,
                ..
            }
            | Packet::Cursor {
                sequence,
                issued_us,
                ..
            }
            | Packet::Escape {
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
            Packet::Escape {
                source_us, mode, ..
            } => {
                if !self.ready
                    || self.mode != Some(mode)
                    || source_us > issued
                    || now.saturating_sub(source_us) >= watchdog_us
                    || self.source.is_some_and(|previous| source_us <= previous)
                {
                    return Err(invalid(
                        "Escape requires fresh increasing source and an armed mode",
                    ));
                }
                let mut batch = self.release();
                self.ready = true;
                self.source = Some(source_us);
                batch.escape = true;
                Ok(batch)
            }
            Packet::Cursor {
                source_us,
                mode,
                absolute,
                ..
            } => {
                if !self.ready
                    || !self.absolute_source
                    || mode != Mode::Pointer
                    || self.mode != Some(mode)
                    || self.source != Some(source_us)
                    || source_us > issued
                    || now - source_us >= watchdog_us
                    || !absolute.valid()
                {
                    return Err(invalid("derived cursor requires the last fresh absolute source and an armed pointer"));
                }
                Ok(Batch {
                    mouse: self.mouse.apply(
                        MouseIntent::Absolute {
                            x: absolute.x,
                            y: absolute.y,
                        },
                        gain,
                    )?,
                    touch: Vec::new(),
                    escape: false,
                })
            }
            Packet::Hold {
                source_us,
                until_us,
                mode,
                ..
            } => {
                if !self.ready
                    || !self.mouse.is_held()
                    || !matches!(mode, Mode::Pointer | Mode::Precision)
                    || self.mode != Some(mode)
                    || self.source != Some(source_us)
                    || source_us > issued
                    || until_us <= now
                    || until_us > source_us.saturating_add(2_000_000)
                    || self.hold_until.is_some_and(|deadline| until_us > deadline)
                {
                    return Err(invalid(
                        "hold requires an existing drag and its bounded original source deadline",
                    ));
                }
                self.hold_until = Some(until_us);
                Ok(Batch::default())
            }
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
                absolute,
                contacts,
                ..
            } => {
                let rejected = [
                    ("unarmed", !self.ready),
                    ("mode_mismatch", self.mode != Some(mode)),
                    ("future_source", source_us > issued),
                    (
                        "source_expired",
                        now.saturating_sub(source_us) >= watchdog_us,
                    ),
                    ("source_order", self.source.is_some_and(|t| source_us <= t)),
                    ("mouse_limit", mouse.len() > 8),
                    ("contact_limit", contacts.len() > 2),
                ];
                if rejected.iter().any(|(_, failed)| *failed) {
                    // Retain the exact failed fences and both ages. A frame may
                    // be fresh when published but expire before its receiver runs.
                    // This is exceptional-path diagnostics, not a clock/arm reset.
                    let reasons: Vec<_> = rejected
                        .iter()
                        .filter_map(|(reason, failed)| failed.then_some(*reason))
                        .collect();
                    let evidence = serde_json::json!({
                        "schema":"wigglecontrol.delivery-rejection/v1",
                        "reasons":reasons,"now_us":now,"issued_us":issued,
                        "source_us":source_us,"source_age_us":now.saturating_sub(source_us),
                        "issued_age_us":now-issued,"watchdog_us":watchdog_us,
                        "previous_source_us":self.source,
                        "receive_gap_us":previously_received.map(|t| now.saturating_sub(t)),
                        "armed_before":previously_armed,"armed_after":self.ready,
                        "mode":mode,"previous_mode":self.mode,"hold_until_us":self.hold_until,
                        "sequence":sequence,"mouse_count":mouse.len(),"contact_count":contacts.len()
                    });
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!(
                            "delivery requires fresh source and an explicit armed mode; {evidence}"
                        ),
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
                                        | Event::NamedButton { down: false, .. }
                                )
                            })
                    }
                    Mode::Pointer | Mode::Precision => {
                        contacts.is_empty()
                            && mouse.iter().all(|e| !matches!(e, Event::Scroll { .. }))
                    }
                };
                if !allowed
                    || absolute.is_some_and(|p| !p.valid() || mode != Mode::Pointer)
                    || (absolute.is_some() && mouse.iter().any(|e| matches!(e, Event::Move { .. })))
                {
                    return Err(invalid("competing or incompatible mode output"));
                }
                self.source = Some(source_us);
                self.absolute_source = absolute.is_some();
                self.hold_until = None;
                let mut batch = Batch::default();
                if let Some(p) = absolute {
                    batch.mouse = self
                        .mouse
                        .apply(MouseIntent::Absolute { x: p.x, y: p.y }, gain)?;
                }
                for event in mouse {
                    let events = match event {
                        Event::Move { dx, dy } => {
                            self.mouse.apply(MouseIntent::Move { dx, dy }, gain)?
                        }
                        Event::Button { down } => {
                            self.mouse.apply(MouseIntent::Button { down }, gain)?
                        }
                        Event::NamedButton { button, down } => self
                            .mouse
                            .apply(MouseIntent::NamedButton { button, down }, gain)?,
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
