//! Shared logical output, cursor publication, source-age/hold and watchdog policy.
use crate::{
    contacts::Contact,
    controller::{Mode, Output},
    Event, MouseButton,
};
use serde::{Deserialize, Serialize};
use std::io;

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

/// Platform event conversion and per-button ownership. Gesture/freshness policy stays here.
pub trait MouseBackend: Default {
    type Event;
    fn apply(&mut self, intent: MouseIntent, gain: f64) -> io::Result<Vec<Self::Event>>;
    fn release(&mut self) -> Vec<Self::Event>;
    fn is_held(&self) -> bool;
}
/// Contact conversion is separate from the shared logical contact/gesture state.
pub trait TouchBackend: Default {
    type Event;
    fn frame(&mut self, contacts: &[Contact]) -> io::Result<Vec<Self::Event>>;
    fn release(&mut self) -> Vec<Self::Event>;
}

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
    scroll_anchor: Option<crate::Point>,
    last_absolute: Option<crate::Point>,
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
        Ok(Some(if output.escape {
            let source_us =
                source.ok_or_else(|| invalid("Escape requires its observation source"))?;
            if !output.running
                || !output.transition
                || output.button_down
                || !output.contacts.is_empty()
            {
                return Err(invalid("Escape requires a running neutralized controller"));
            }
            Packet::Escape {
                sequence: self.sequence,
                issued_us: now,
                source_us,
                mode: output.mode,
            }
        } else if output.transition {
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
        if output.running && output.scroll.as_ref().is_some_and(|s| s.captures_pointer()) {
            if self.scroll_anchor.is_none() {
                self.scroll_anchor = self
                    .cursor
                    .as_ref()
                    .and_then(|c| c.feedback())
                    .map(|s| s.estimated)
                    .or(self.last_absolute);
            }
            if let Some(cursor) = &mut self.cursor {
                cursor.freeze(now).map_err(io::Error::other)?;
            }
            if let Some(Packet::Frame {
                mouse, absolute, ..
            }) = &mut packet
            {
                mouse.retain(|e| {
                    matches!(
                        e,
                        Event::Scroll { .. } | Event::ScrollFine { .. } | Event::Cancel { .. }
                    )
                });
                *absolute = None;
            }
            return Ok(packet);
        }
        if output.reason == Some("scroll_released")
            && output.transition
            && self.scroll_anchor.is_some()
            && self.mapping_owner == output.primary_hand
        {
            if let Some(cursor) = &mut self.cursor {
                cursor.freeze(now).map_err(io::Error::other)?;
            }
            return Ok(packet);
        }
        if output.transition
            || !output.running
            || output.mode != Mode::Pointer
            || self.mapping_owner != output.primary_hand
        {
            self.detail_map.reset();
            self.mapping_owner = output.primary_hand;
            self.scroll_anchor = None;
            self.last_absolute = None;
        }
        let mapped = if output.mode == Mode::Pointer && matches!(packet, Some(Packet::Frame { .. }))
        {
            output
                .pointer_cursor
                .map(|point| {
                    if let Some(anchor) = self.scroll_anchor.take() {
                        self.detail_map
                            .anchor(workspace.map(point), output.detail, anchor);
                        return Ok((anchor, true));
                    }
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
                    self.last_absolute = *absolute;
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
                if absolute.is_some() {
                    self.last_absolute = *absolute;
                }
            }
        }
        Ok(packet)
    }
}
pub struct Batch<M: MouseBackend, T: TouchBackend> {
    pub mouse: Vec<M::Event>,
    pub touch: Vec<T::Event>,
    pub escape: bool,
}
impl<M: MouseBackend, T: TouchBackend> Default for Batch<M, T> {
    fn default() -> Self {
        Self {
            mouse: Vec::new(),
            touch: Vec::new(),
            escape: false,
        }
    }
}

#[derive(Default)]
pub struct State<M: MouseBackend, T: TouchBackend> {
    mouse: M,
    touch: T,
    mode: Option<Mode>,
    ready: bool,
    sequence: Option<u64>,
    source: Option<u64>,
    now: Option<u64>,
    received: Option<u64>,
    hold_until: Option<u64>,
    expired_hold: Option<(u64, u64, Mode)>,
    absolute_source: bool,
}
fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}
impl<M: MouseBackend, T: TouchBackend> State<M, T> {
    /// Bind configured platform converters without arming or emitting input.
    pub fn with_backends(mouse: M, touch: T) -> Self {
        Self {
            mouse,
            touch,
            ..Self::default()
        }
    }
    /// Delivery readiness, not evidence of application input receipt.
    pub fn is_armed(&self) -> bool {
        self.ready
    }
    pub fn release(&mut self) -> Batch<M, T> {
        self.ready = false;
        self.hold_until = None;
        self.expired_hold = None;
        self.absolute_source = false;
        Batch {
            mouse: self.mouse.release(),
            touch: self.touch.release(),
            escape: false,
        }
    }
    pub fn tick(&mut self, now: u64, watchdog_us: u64) -> io::Result<Batch<M, T>> {
        if self.now.is_some_and(|t| now < t) {
            return Err(invalid("delivery clock regressed"));
        }
        self.now = Some(now);
        let watchdog = self
            .received
            .is_some_and(|t| now.saturating_sub(t) >= watchdog_us);
        if watchdog || self.hold_until.is_some_and(|deadline| now >= deadline) {
            // Retain only this established drag's expired deadline, so an
            // already-published Hold can be drained without reopening a press.
            // An independent watchdog release deliberately retains no such proof.
            let expired = (!watchdog && self.mouse.is_held())
                .then(|| {
                    self.source
                        .zip(self.hold_until)
                        .zip(self.mode)
                        .map(|((source, until), mode)| (source, until, mode))
                })
                .flatten();
            let batch = self.release();
            self.expired_hold = expired;
            Ok(batch)
        } else {
            Ok(Batch::default())
        }
    }
    pub fn apply(
        &mut self,
        now: u64,
        packet: Packet,
        gain: f64,
        watchdog_us: u64,
    ) -> io::Result<Batch<M, T>> {
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
                let retired = self.expired_hold.filter(|(source, _, previous_mode)| {
                    *source == source_us && *previous_mode == mode
                });
                let original_deadline = self.hold_until.or(retired.map(|(_, until, _)| until));
                if until_us <= now
                    && issued < until_us
                    && source_us <= issued
                    && until_us <= source_us.saturating_add(2_000_000)
                    && original_deadline.is_none_or(|deadline| until_us <= deadline)
                    && matches!(mode, Mode::Pointer | Mode::Precision)
                    && self.mode == Some(mode)
                    && self.source == Some(source_us)
                    && previously_received.is_some_and(|t| now.saturating_sub(t) < watchdog_us)
                    && ((previously_armed && self.mouse.is_held()) || retired.is_some())
                {
                    // Receipt crossed the deadline, despite publication before
                    // it. Release exactly once and stay unarmed; the producer's
                    // normal neutral transition can continue this same stream.
                    let batch = self.release();
                    self.expired_hold = Some((source_us, until_us, mode));
                    return Ok(batch);
                }
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
                                        | Event::ScrollFine { .. }
                                        | Event::Cancel { .. }
                                        | Event::Button { down: false }
                                        | Event::NamedButton { down: false, .. }
                                )
                            })
                    }
                    Mode::Pointer | Mode::Precision => {
                        contacts.is_empty()
                            && mouse.iter().all(|e| {
                                !matches!(e, Event::Scroll { .. } | Event::ScrollFine { .. })
                            })
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
                        Event::Scroll {
                            vertical,
                            horizontal,
                        } => self.mouse.apply(
                            MouseIntent::Scroll {
                                vertical,
                                horizontal,
                            },
                            gain,
                        )?,
                        Event::ScrollFine {
                            vertical,
                            horizontal,
                        } => self.mouse.apply(
                            MouseIntent::ScrollFine {
                                vertical,
                                horizontal,
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
