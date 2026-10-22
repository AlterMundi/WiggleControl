//! Exclusive interaction modes over the existing pointer/contact/macro engines.
//! Intent only: acquiring sensors and delivering OS input remain separate adapters.
use crate::{
    contacts::Contact, macros, pointer_buttons, touch_gestures, Config, Engine, Event, Hand,
    MouseButton, Observation,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    #[default]
    Pointer,
    Precision,
    Scroll,
    OneContact,
    TwoContacts,
}
impl Mode {
    fn touch(self) -> bool {
        matches!(self, Self::OneContact | Self::TwoContacts)
    }
    fn next(self) -> Self {
        match self {
            Self::Pointer => Self::OneContact,
            Self::OneContact => Self::TwoContacts,
            _ => Self::Pointer,
        }
    }
}
#[derive(Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Settings {
    pub version: u32,
    pub initial_mode: Mode,
    pub precision_gain: f64,
    pub gestures: Config,
    pub workspace: touch_gestures::Workspace,
    pub cursor_output: Option<crate::cursor::Config>,
    pub macros: Option<macros::Config>,
    pub visual_macros: Option<macros::VisualPolicy>,
    pub right_click: bool,
    pub right_click_dominance_ratio: f64,
    pub detail: Option<crate::detail::Config>,
    pub escape: Option<crate::escape::Config>,
    pub scroll: Option<crate::scroll::Config>,
}
impl Settings {
    pub fn load(path: &std::path::Path) -> std::io::Result<Self> {
        use std::{fs, io};
        if fs::metadata(path)?.len() > 8192 {
            return Err(io::Error::other("controller config exceeds8KiB"));
        }
        Ok(serde_json::from_slice(&fs::read(path)?)?)
    }
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            version: 1,
            initial_mode: Mode::Pointer,
            precision_gain: 0.25,
            gestures: Config::default(),
            workspace: Default::default(),
            cursor_output: None,
            macros: None,
            visual_macros: None,
            right_click: false,
            right_click_dominance_ratio: 0.5,
            detail: None,
            escape: None,
            scroll: None,
        }
    }
}
#[derive(Debug, Serialize)]
pub struct ActionResult {
    pub action: macros::Action,
    /// Applied inside this semantic controller, never proof of an OS effect.
    pub handled: bool,
}
#[derive(Clone, Copy, Debug, Serialize)]
pub struct MacroReceipt {
    pub action: macros::Action,
    pub handled: bool,
    pub source_us: u64,
    pub mode: Mode,
    pub running: bool,
}
#[derive(Debug, Serialize)]
pub struct Output {
    /// One-shot command for this observation only; never repeated by a tick.
    pub escape: bool,
    pub escape_feedback: Option<macros::Feedback>,
    pub scroll: Option<crate::scroll::Feedback>,
    pub mode: Mode,
    pub right_click: bool,
    /// Active held modifier; separate from destructive exclusive mode transitions.
    pub detail: Option<crate::detail::Config>,
    pub running: bool,
    pub transition: bool,
    pub reason: Option<&'static str>,
    pub primary_hand: Option<u64>,
    pub claimed_hand: Option<u64>,
    pub pointer_cursor: Option<crate::Point>,
    pub pointer_state: &'static str,
    pub confirmation_elapsed_us: u64,
    pub pointer_ready: bool,
    pub button_down: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pointer_button: Option<MouseButton>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub drag_hold: Option<crate::DragHold>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub release_pending: Option<crate::ReleasePending>,
    pub mouse: Vec<Event>,
    /// Complete current contact set, including an empty release frame.
    pub contacts: Vec<Contact>,
    pub awaiting_open_hands: Vec<u64>,
    pub action: Option<ActionResult>,
    pub macro_feedback: Option<macros::Feedback>,
    /// Bounded historical receipt, separate from this frame's action/output.
    pub last_macro_action: Option<MacroReceipt>,
}
/// Optional raw controls share controller-owned hand/source admission.
#[derive(Default)]
pub struct Controls {
    pub middle_pinch: Option<pointer_buttons::Evidence>,
    pub detail: Option<crate::detail::Evidence>,
    pub escape: Option<crate::escape::Evidence>,
    pub scroll: Option<crate::scroll::Evidence>,
}
pub struct Controller {
    mode: Mode,
    running: bool,
    config: Config,
    precision_gain: f64,
    pointer: Engine,
    touches: touch_gestures::Gestures,
    macros: Option<macros::Engine>,
    visual_policy: Option<macros::VisualPolicy>,
    last_macro_action: Option<MacroReceipt>,
    claimed: Option<u64>,
    primary: Option<u64>,
    pointer_ready: bool,
    button_candidate: Option<(MouseButton, u64)>,
    active_button: Option<MouseButton>,
    right_click: bool,
    right_click_dominance_ratio: f64,
    detail_config: Option<crate::detail::Config>,
    detail: crate::detail::Held,
    escape_config: Option<crate::escape::Config>,
    escape_engine: Option<macros::Engine>,
    scroll_config: Option<crate::scroll::Config>,
    scroll: crate::scroll::Held,
    scroll_captured: bool,
    contacts: Vec<Contact>,
    watermark: Option<u64>,
    last_now: Option<u64>,
}
impl Controller {
    pub fn new(settings: Settings) -> Result<Self, &'static str> {
        if let Some(config) = settings.scroll {
            config.validate()?;
            if settings
                .escape
                .is_some_and(|e| config.thumb_clearance_max >= e.thumb_clearance_widths)
            {
                return Err("scroll and Escape thumb regions must not overlap");
            }
        }
        if let Some(config) = settings.detail {
            config.validate()?;
        }
        if let Some(config) = settings.cursor_output {
            config.validate()?;
        }
        if settings.version != 1
            || !settings.precision_gain.is_finite()
            || !(0.05..=1.0).contains(&settings.precision_gain)
        {
            return Err("invalid controller version or precision gain");
        }
        if !settings.right_click_dominance_ratio.is_finite()
            || !(0.0..1.0).contains(&settings.right_click_dominance_ratio)
        {
            return Err("right-click dominance ratio must be finite and 0 <= ratio < 1");
        }
        if let Some(policy) = &settings.visual_macros {
            policy.validate(settings.macros.as_ref())?;
        }
        Ok(Self {
            mode: settings.initial_mode,
            running: false,
            config: settings.gestures,
            precision_gain: settings.precision_gain,
            pointer: Engine::new(settings.gestures)?,
            touches: touch_gestures::Gestures::new(
                Config {
                    drag_grace_us: 0,
                    release_confirmation: None,
                    ..settings.gestures
                },
                settings.workspace,
            )?,
            macros: settings.macros.map(macros::Engine::new).transpose()?,
            visual_policy: settings.visual_macros,
            last_macro_action: None,
            claimed: None,
            primary: None,
            pointer_ready: false,
            button_candidate: None,
            active_button: None,
            right_click: settings.right_click,
            right_click_dominance_ratio: settings.right_click_dominance_ratio,
            detail_config: settings.detail,
            detail: Default::default(),
            escape_config: settings.escape,
            scroll_config: settings.scroll,
            scroll: Default::default(),
            scroll_captured: false,
            escape_engine: settings
                .escape
                .map(|c| c.engine(settings.gestures.stale_us, settings.gestures.min_confidence))
                .transpose()?,
            contacts: Vec::new(),
            watermark: None,
            last_now: None,
        })
    }
    fn output(&self, mouse: Vec<Event>, transition: bool) -> Output {
        Output {
            escape: false,
            escape_feedback: self.escape_engine.as_ref().map(macros::Engine::feedback),
            scroll: self.scroll_config.map(|c| self.scroll.feedback(c)),
            mode: if self.scroll_captured {
                Mode::Scroll
            } else {
                self.mode
            },
            right_click: self.right_click,
            detail: self.detail_config.filter(|_| self.detail.active()),
            running: self.running,
            transition,
            reason: None,
            primary_hand: self.primary,
            claimed_hand: self.claimed,
            pointer_cursor: self.pointer.cursor_position(),
            pointer_state: self.pointer.pinch_state(),
            confirmation_elapsed_us: self.pointer.confirmation_elapsed_us(),
            pointer_ready: self.pointer_ready,
            button_down: self.pointer.pinch_state() == "pressed",
            pointer_button: self.active_button.or(self.button_candidate.map(|(b, _)| b)),
            drag_hold: self.pointer.drag_hold(),
            release_pending: self.pointer.release_pending(),
            mouse,
            contacts: self.contacts.clone(),
            awaiting_open_hands: self.touches.awaiting_open_hands(),
            action: None,
            macro_feedback: self.macros.as_ref().map(macros::Engine::feedback),
            last_macro_action: self.last_macro_action.filter(|r| {
                self.last_now
                    .is_some_and(|now| r.source_us <= now && now - r.source_us < 2_000_000)
            }),
        }
    }
    pub fn detail_config(&self) -> Option<crate::detail::Config> {
        self.detail_config
    }
    pub(crate) fn active_detail_config(&self) -> Option<crate::detail::Config> {
        self.detail_config
            .filter(|_| self.mode == Mode::Pointer && self.detail.active())
    }
    pub fn scroll_config(&self) -> Option<crate::scroll::Config> {
        self.scroll_config
    }
    pub fn escape_config(&self) -> Option<crate::escape::Config> {
        self.escape_config
    }
    pub fn visual_policy(&self) -> Option<&macros::VisualPolicy> {
        self.visual_policy.as_ref()
    }
    pub fn right_click_enabled(&self) -> bool {
        self.right_click
    }
    pub fn right_click_dominance_ratio(&self) -> f64 {
        self.right_click_dominance_ratio
    }
    pub(crate) fn selected_pointer_hand(&self, frame: &touch_gestures::Frame) -> Option<u64> {
        if !self.running || frame.ambiguous || self.mode.touch() {
            None
        } else {
            self.selected_hand(frame)
        }
    }
    /// Route both edges to the button chosen at press, including tick/Stop release.
    fn pointer_events(&mut self, events: Vec<Event>) -> Vec<Event> {
        events
            .into_iter()
            .map(|event| match event {
                Event::Button { down } => {
                    let button = if down {
                        let button = self.button_candidate.map_or(MouseButton::Left, |(b, _)| b);
                        self.active_button = Some(button);
                        button
                    } else {
                        self.active_button.take().unwrap_or(MouseButton::Left)
                    };
                    if button == MouseButton::Left {
                        event
                    } else {
                        Event::NamedButton { button, down }
                    }
                }
                event => event,
            })
            .collect()
    }
    fn selected_hand(&self, frame: &touch_gestures::Frame) -> Option<u64> {
        if self.mode == Mode::TwoContacts {
            None
        } else {
            self.claimed
                .or(self
                    .primary
                    .filter(|id| frame.hands.iter().any(|h| h.id == *id)))
                .or_else(|| frame.hands.iter().map(|h| h.id).min())
        }
    }
    /// The classifier follows existing controller ownership, never detector order.
    pub fn macro_hand(&self, frame: &touch_gestures::Frame) -> Option<u64> {
        if !self.running || frame.ambiguous {
            return None;
        }
        if self.mode != Mode::TwoContacts {
            return self.selected_hand(frame);
        }
        // The one-hand claim is inactive in two-contact mode, like its disabled
        // UI selector. Do not strand macros on a hidden prior one-hand claim.
        self.macros
            .as_ref()
            .and_then(macros::Engine::owner_hand)
            .filter(|id| frame.hands.iter().any(|h| h.id == *id))
            .or_else(|| frame.hands.iter().map(|h| h.id).min())
    }
    fn neutralize(&mut self, reason: &'static str) -> Output {
        let events = self.pointer.stop();
        let events = self.pointer_events(events);
        self.button_candidate = None;
        self.detail = Default::default();
        self.scroll.reset();
        self.scroll_captured = false;
        self.contacts = self.touches.stop();
        self.primary = None;
        self.pointer_ready = false;
        if let Some(macros) = &mut self.macros {
            macros.stop();
        }
        if let Some(escape) = &mut self.escape_engine {
            escape.stop();
        }
        if self.running {
            if self.mode.touch() {
                self.touches.start();
            } else {
                self.pointer.start();
            }
            if let Some(macros) = &mut self.macros {
                macros.start();
            }
            if let Some(escape) = &mut self.escape_engine {
                escape.start();
            }
        }
        let mut output = self.output(events, true);
        output.reason = Some(reason);
        output
    }
    fn transition_at(&mut self, now: u64, change: impl FnOnce(&mut Self) -> Output) -> Output {
        let previous = self.tick(now);
        // Manual controls are wall-clock events: a queued pre-control camera
        // frame must not supply the new mode's open-before-press evidence.
        self.watermark = Some(
            self.watermark
                .unwrap_or(now)
                .max(now)
                .max(self.last_now.unwrap_or(now)),
        );
        let mut output = change(self);
        output.mouse.splice(0..0, previous.mouse);
        output
    }
    pub fn start_at(&mut self, now: u64) -> Output {
        self.transition_at(now, Self::start)
    }
    pub fn set_mode_at(&mut self, mode: Mode, now: u64) -> Output {
        if mode == self.mode {
            return self.output(Vec::new(), false);
        }
        self.transition_at(now, |this| this.set_mode(mode))
    }
    pub fn select_hand_at(&mut self, id: Option<u64>, now: u64) -> Output {
        if id == self.claimed {
            return self.output(Vec::new(), false);
        }
        self.transition_at(now, |this| this.select_hand(id))
    }
    pub fn is_running(&self) -> bool {
        self.running
    }
    pub fn start(&mut self) -> Output {
        self.running = true;
        self.neutralize("started")
    }
    pub fn stop(&mut self) -> Output {
        self.running = false;
        self.neutralize("stopped")
    }
    pub fn set_mode(&mut self, mode: Mode) -> Output {
        if self.mode == mode {
            return self.output(Vec::new(), false);
        }
        self.mode = mode;
        self.neutralize("mode_changed")
    }
    /// An explicit track claim is not participant identification or handedness.
    pub fn select_hand(&mut self, id: Option<u64>) -> Output {
        if self.claimed == id {
            return self.output(Vec::new(), false);
        }
        self.claimed = id;
        self.neutralize("hand_selected")
    }
    pub fn command(&mut self, command: Command) -> Output {
        match command {
            Command::Start => self.start(),
            Command::Stop => self.stop(),
            Command::Mode { mode } => self.set_mode(mode),
            Command::SelectHand { hand_id } => self.select_hand(hand_id),
            Command::Action { action } => self.action(action),
            Command::Tick { now_us } => self.tick(now_us),
            Command::Frame {
                now_us,
                observation,
                pose,
                middle_pinch,
                detail,
                escape,
                scroll,
            } => self.observe_with_evidence(
                now_us,
                observation,
                pose,
                Controls {
                    middle_pinch,
                    detail,
                    escape,
                    scroll: scroll.map(|e| *e),
                },
            ),
        }
    }
    pub fn action(&mut self, action: macros::Action) -> Output {
        let (mut output, handled) = match action {
            macros::Action::TogglePrecision => (
                self.set_mode(if self.mode == Mode::Precision {
                    Mode::Pointer
                } else {
                    Mode::Precision
                }),
                true,
            ),
            macros::Action::ToggleScroll => (
                self.set_mode(if self.mode == Mode::Scroll {
                    Mode::Pointer
                } else {
                    Mode::Scroll
                }),
                true,
            ),
            macros::Action::CycleMode => (self.set_mode(self.mode.next()), true),
            macros::Action::PauseController => (self.stop(), true),
            _ => (self.output(Vec::new(), false), false),
        };
        output.action = Some(ActionResult { action, handled });
        output
    }
    /// Recheck observation age immediately before publishing to the device writer.
    /// Interpretation and recording can consume the remaining freshness budget.
    /// A late frame must rearm through neutral rather than terminate the session
    /// or deliver a stale press. Existing bounded loss holds keep their deadline.
    pub fn guard_delivery(
        &mut self,
        output: Output,
        now: u64,
        source: Option<u64>,
    ) -> (Output, bool) {
        let reserve = 20_000.min(self.config.stale_us / 4);
        if output.running
            && (!output.transition || output.escape)
            && !output.drag_hold.is_some_and(|hold| hold.in_grace)
            && source.is_some_and(|t| {
                now.saturating_sub(t) >= self.config.stale_us.saturating_sub(reserve)
            })
        {
            // Do not advance the observation watermark to wall time: a later
            // queued, still-fresh open observation may legitimately rearm.
            self.last_now = Some(self.last_now.unwrap_or(now).max(now));
            return (self.neutralize("delivery_source_expired"), true);
        }
        (output, false)
    }
    pub fn tick(&mut self, now: u64) -> Output {
        if self.last_now.is_some_and(|t| now < t) {
            return self.neutralize("consumer_clock_regression");
        }
        self.last_now = Some(now);
        if let Some(escape) = &mut self.escape_engine {
            escape.tick(now);
        }
        if self
            .watermark
            .is_some_and(|t| now.saturating_sub(t) >= self.config.stale_us)
            && self
                .pointer
                .drag_hold()
                .is_none_or(|hold| now >= hold.until_us)
        {
            return self.neutralize("stale_input");
        }
        let mouse = self.pointer.tick(now);
        let mouse = self.pointer_events(mouse);
        // Tick-only releases have no new observation and need a delivery transition.
        if !mouse.is_empty() {
            // Pointer expiry must release on schedule without discarding a
            // separately fresh cancellation-pose hold. Missing/stale pose, owner,
            // mode and Stop still reset its engine through their normal paths.
            let escape = self
                .escape_engine
                .take()
                .map(|engine| (engine, self.primary));
            let mut output = self.neutralize("pointer_state_expired");
            if let Some((engine, owner)) = escape {
                self.escape_engine = Some(engine);
                self.primary = owner;
                output.primary_hand = owner;
                output.escape_feedback = self.escape_engine.as_ref().map(macros::Engine::feedback);
            }
            output.mouse.splice(0..0, mouse);
            return output;
        }
        self.contacts = self.touches.tick(now);
        if let Some(macros) = &mut self.macros {
            macros.tick(now);
        }
        self.output(mouse, false)
    }
    fn open(&self, hand: &Hand) -> bool {
        hand.open(&self.config, false)
    }
    fn fully_open(&self, hand: &Hand) -> bool {
        hand.open(&self.config, true)
    }
    pub fn observe(
        &mut self,
        now: u64,
        frame: touch_gestures::Frame,
        pose: Option<macros::HandPose>,
    ) -> Output {
        self.observe_with_buttons(now, frame, pose, None)
    }
    pub fn observe_with_buttons(
        &mut self,
        now: u64,
        frame: touch_gestures::Frame,
        pose: Option<macros::HandPose>,
        middle_pinch: Option<pointer_buttons::Evidence>,
    ) -> Output {
        self.observe_with_modifiers(now, frame, pose, middle_pinch, None)
    }
    pub fn observe_with_modifiers(
        &mut self,
        now: u64,
        frame: touch_gestures::Frame,
        pose: Option<macros::HandPose>,
        middle_pinch: Option<pointer_buttons::Evidence>,
        detail: Option<crate::detail::Evidence>,
    ) -> Output {
        self.observe_with_controls(now, frame, pose, middle_pinch, detail, None)
    }
    pub fn observe_with_controls(
        &mut self,
        now: u64,
        frame: touch_gestures::Frame,
        pose: Option<macros::HandPose>,
        middle_pinch: Option<pointer_buttons::Evidence>,
        detail: Option<crate::detail::Evidence>,
        escape: Option<crate::escape::Evidence>,
    ) -> Output {
        self.observe_with_evidence(
            now,
            frame,
            pose,
            Controls {
                middle_pinch,
                detail,
                escape,
                scroll: None,
            },
        )
    }
    pub fn observe_with_evidence(
        &mut self,
        now: u64,
        frame: touch_gestures::Frame,
        pose: Option<macros::HandPose>,
        controls: Controls,
    ) -> Output {
        let Controls {
            middle_pinch,
            detail,
            escape,
            scroll,
        } = controls;
        let regressed = self.last_now.is_some_and(|t| now < t);
        let tick = self.tick(now);
        if !self.running {
            return tick;
        }
        if regressed
            || frame.version != 1
            || frame.source_us > now
            || now - frame.source_us >= self.config.stale_us
            || self.watermark.is_some_and(|t| frame.source_us <= t)
            || frame.hands.len() > 2
            || frame.hands.iter().any(|h| !h.valid_for(frame.source_us))
            || (frame.hands.len() == 2 && frame.hands[0].id == frame.hands[1].id)
            || (self.right_click && frame.hands.iter().any(|h| h.secondary_pinch.is_some()))
        {
            let mut released = self.neutralize("invalid_observation");
            released.mouse.splice(0..0, tick.mouse);
            return released;
        }
        self.watermark = Some(frame.source_us);
        if frame.ambiguous {
            let mut released = self.neutralize("ambiguous_identity");
            released.mouse.splice(0..0, tick.mouse);
            return released;
        }
        let selected = if frame.hands.is_empty() && !self.mode.touch() {
            self.pointer
                .drag_hold()
                .map(|h| h.hand_id)
                .or_else(|| self.selected_hand(&frame))
        } else {
            self.selected_hand(&frame)
        };
        if selected != self.primary {
            self.scroll.reset();
            self.detail = Default::default();
            self.pointer_ready = false;
            self.button_candidate = None;
            if let Some(macros) = &mut self.macros {
                macros.start();
            }
            if let Some(escape) = &mut self.escape_engine {
                escape.start();
            }
        }
        self.primary = selected;
        // Escape uses positive in-view raw pose evidence, not pointer eligibility:
        // four curled fingers deliberately fail the normal pointer's open-hand gate.
        let escape_pose = escape
            .filter(|e| {
                e.source_us == frame.source_us && Some(e.hand_id) == selected && !self.mode.touch()
            })
            .and_then(|e| {
                let hand = frame.hands.iter().find(|h| h.id == e.hand_id)?;
                if hand.confidence < self.config.min_confidence {
                    return None;
                }
                let neutral = self.fully_open(hand)
                    && (!self.right_click
                        || middle_pinch
                            .and_then(|m| m.middle_hand(hand, frame.source_us))
                            .is_some_and(|m| self.fully_open(&m)));
                let pose = match e.pose {
                    crate::escape::Pose::Neutral if neutral => None,
                    crate::escape::Pose::ThumbOut => Some(macros::Pose::ThumbOut),
                    _ => Some(macros::Pose::Other),
                };
                Some(macros::HandPose {
                    id: hand.id,
                    confidence: hand.confidence,
                    pose,
                })
            });
        let escape_pose_transition = escape_pose.is_some()
            && frame
                .hands
                .iter()
                .any(|h| Some(h.id) == selected && (!h.engaged || !h.pinch_eligible));
        let (escape_fired, escape_holding) = if let Some(engine) = &mut self.escape_engine {
            let fired = engine.observe(
                now,
                macros::Frame {
                    version: 1,
                    source_us: frame.source_us,
                    ambiguous: false,
                    contacts_active: false,
                    hand: escape_pose,
                },
            ) == Some(macros::Action::Escape);
            let feedback = engine.feedback();
            (
                fired,
                feedback.candidate == Some(macros::Pose::ThumbOut)
                    || (feedback.armed
                        && escape_pose_transition
                        && self.pointer.pinch_state() == "pressed"),
            )
        } else {
            (false, false)
        };
        if escape_fired {
            let mut output = self.neutralize("escape");
            output.escape = true;
            let receipt = MacroReceipt {
                action: macros::Action::Escape,
                handled: true,
                source_us: frame.source_us,
                mode: self.mode,
                running: self.running,
            };
            self.last_macro_action = Some(receipt);
            output.last_macro_action = Some(receipt);
            return output;
        }
        if escape_holding {
            let scroll_released = self.scroll_captured;
            if scroll_released {
                self.scroll.reset();
                self.scroll_captured = false;
            }
            // Suppress pinch guesses and motion during a confirmed-owner candidate.
            // Preserve only the existing same-source bounded drag grace, never
            // extend it with these cancellation-pose observations.
            let hand = frame
                .hands
                .iter()
                .find(|h| Some(h.id) == selected)
                .cloned()
                .map(|mut h| {
                    h.engaged = true;
                    h.pinch_eligible = false;
                    h
                });
            let events = self.pointer.observe(
                now,
                Observation {
                    version: 1,
                    source_us: frame.source_us,
                    hand,
                },
            );
            let events = self.pointer_events(events);
            let mut output = self.output(events, tick.transition || scroll_released);
            output.mouse.splice(0..0, tick.mouse);
            output.reason = Some(if scroll_released {
                "scroll_released"
            } else {
                "escape_holding"
            });
            return output;
        }
        let mut scroll_released = false;
        if let Some(config) = self.scroll_config.filter(|_| self.mode == Mode::Pointer) {
            let was_captured = self.scroll_captured;
            let evidence = scroll
                .filter(|e| e.source_us == frame.source_us && Some(e.hand_id) == selected)
                .and_then(|mut e| {
                    let hand = frame.hands.iter().find(|h| h.id == e.hand_id)?;
                    // A committed press keeps its existing drag/release rules.
                    // A pending pinch may yield to a positive fist before any
                    // button has been emitted; curling fingers can pass through it.
                    if hand.confidence < self.config.min_confidence
                        || self.pointer.pinch_state() == "pressed"
                    {
                        return None;
                    }
                    let neutral = self.fully_open(hand)
                        && (!self.right_click
                            || middle_pinch
                                .and_then(|m| m.middle_hand(hand, frame.source_us))
                                .is_some_and(|m| self.fully_open(&m)));
                    if e.pose == crate::scroll::Pose::Neutral && !neutral {
                        e.pose = crate::scroll::Pose::Other;
                    }
                    Some(e)
                });
            let motion =
                self.scroll
                    .observe(config, frame.source_us, evidence, self.config.stale_us);
            self.scroll_captured = self.scroll.feedback(config).captures_pointer();
            scroll_released = was_captured && !self.scroll_captured;
            if self.scroll_captured {
                let mut events = tick.mouse;
                if !was_captured {
                    let releases = self.pointer.stop();
                    events.extend(self.pointer_events(releases));
                    self.pointer.start();
                    self.pointer_ready = false;
                    self.button_candidate = None;
                }
                if motion.vertical != 0 || motion.horizontal != 0 {
                    events.push(Event::ScrollFine {
                        vertical: motion.vertical,
                        horizontal: motion.horizontal,
                    });
                }
                let mut output = self.output(events, tick.transition || !was_captured);
                output.reason = Some("held_scroll");
                return output;
            }
        }
        if let Some(hold) = self.pointer.drag_hold() {
            let owner_changed = selected.is_some_and(|id| id != hold.hand_id);
            let disengaged = frame
                .hands
                .iter()
                .any(|h| h.id == hold.hand_id && !h.engaged);
            if owner_changed || disengaged {
                let mut output = self.neutralize(if owner_changed {
                    "drag_owner_changed"
                } else {
                    "hand_not_engaged"
                });
                output.mouse.splice(0..0, tick.mouse);
                return output;
            }
        }
        let mut mouse = tick.mouse;
        if let Some(config) = self.detail_config.filter(|_| self.mode == Mode::Pointer) {
            let held = detail
                .filter(|e| e.source_us == frame.source_us && Some(e.hand_id) == selected)
                .and_then(|e| {
                    frame
                        .hands
                        .iter()
                        .find(|h| h.id == e.hand_id)
                        .filter(|h| {
                            h.engaged
                                && h.pinch_eligible
                                && h.confidence >= self.config.min_confidence
                        })
                        .map(|_| e.held)
                });
            self.detail
                .observe(config, frame.source_us, held, self.config.stale_us);
        }
        let mut buttons_neutral = !self.right_click;
        if self.mode.touch() {
            let hands = frame
                .hands
                .iter()
                .filter(|h| self.mode == Mode::TwoContacts || Some(h.id) == selected)
                .cloned()
                .collect();
            self.contacts = self.touches.observe(
                now,
                touch_gestures::Frame {
                    version: 1,
                    source_us: frame.source_us,
                    ambiguous: false,
                    hands,
                },
            );
        } else {
            let mut hand = frame.hands.iter().find(|h| Some(h.id) == selected).cloned();
            if self.right_click {
                let middle = hand
                    .as_ref()
                    .and_then(|h| middle_pinch.and_then(|e| e.middle_hand(h, frame.source_us)));
                buttons_neutral = hand.as_ref().is_some_and(|h| self.fully_open(h))
                    && middle.as_ref().is_some_and(|h| self.fully_open(h));
                let choice = self.active_button.or_else(|| {
                    hand.as_ref()
                        .zip(middle.as_ref())
                        .and_then(|(index, middle)| {
                            pointer_buttons::selection(
                                index,
                                middle,
                                &self.config,
                                self.right_click_dominance_ratio,
                            )
                        })
                });
                self.button_candidate = choice.map(|button| {
                    self.button_candidate
                        .filter(|(previous, _)| *previous == button)
                        .unwrap_or((button, frame.source_us))
                });
                if choice == Some(MouseButton::Right) {
                    hand = middle.or_else(|| {
                        hand.map(|mut h| {
                            // Lost middle-finger evidence is unknown input for an
                            // existing right drag, never an index-based release.
                            h.pinch_eligible = false;
                            h
                        })
                    });
                }
                if let Some(hand) = &mut hand {
                    hand.pinch_onset_allowed &= self.button_candidate.is_some_and(|(_, since)| {
                        frame.source_us.saturating_sub(since) >= self.config.confirm_us
                    });
                }
            }
            if hand.as_ref().is_none_or(|h| {
                !h.engaged || !h.pinch_eligible || h.confidence < self.config.min_confidence
            }) && !self.pointer.drag_hold().is_some_and(|hold| {
                now < hold.until_us
                    && selected == Some(hold.hand_id)
                    && hand.as_ref().is_none_or(|h| h.engaged)
            }) {
                self.pointer_ready = false;
            }
            if !self.pointer_ready {
                self.pointer_ready = if self.right_click {
                    buttons_neutral
                } else {
                    hand.as_ref().is_some_and(|h| self.open(h))
                };
            }
            if !self.pointer_ready {
                hand = None;
            }
            if let Some(hand) = &mut hand {
                hand.scrolling = self.mode == Mode::Scroll;
            }
            let events = self.pointer.observe(
                now,
                Observation {
                    version: 1,
                    source_us: frame.source_us,
                    hand,
                },
            );
            let events = self.pointer_events(events);
            if self.right_click
                && !buttons_neutral
                && self.pointer.pinch_state() != "pressed"
                && events.iter().any(|e| {
                    matches!(
                        e,
                        Event::Button { down: false } | Event::NamedButton { down: false, .. }
                    )
                })
            {
                // Opening only the active pinch cannot arm an already closed
                // different finger. Require explicit neutral across both.
                self.pointer_ready = false;
            }
            mouse.extend(events.into_iter().map(|event| match event {
                Event::Move { dx, dy } if self.mode == Mode::Precision => Event::Move {
                    dx: dx * self.precision_gain,
                    dy: dy * self.precision_gain,
                },
                event => event,
            }));
        }
        let busy = (self.right_click && !self.mode.touch() && !buttons_neutral)
            || !self.contacts.is_empty()
            || matches!(self.pointer.pinch_state(), "closing" | "pressed")
            || frame.hands.iter().any(|h| {
                (self.mode == Mode::TwoContacts || Some(h.id) == selected)
                    && h.engaged
                    && h.pinch_eligible
                    && h.confidence >= self.config.min_confidence
                    && !self.fully_open(h)
            });
        // A classifier cannot claim a different/missing hand or override observed confidence.
        let macro_hand = self.macro_hand(&frame);
        let pose = pose.and_then(|mut p| {
            if !p.confidence.is_finite() {
                return None;
            }
            let observed = frame
                .hands
                .iter()
                .find(|h| h.id == p.id && Some(h.id) == macro_hand)?;
            // A recognized neutral class must also be positively open for input.
            // Fist/ineligible/out-of-view evidence cannot silently rearm macros.
            if p.pose.is_none() && !self.fully_open(observed) {
                return None;
            }
            p.confidence = p.confidence.min(observed.confidence);
            if self
                .visual_policy
                .as_ref()
                .is_some_and(|policy| p.confidence < policy.min_confidence)
            {
                return None;
            }
            Some(p)
        });
        let action = self.macros.as_mut().and_then(|macros| {
            macros.observe(
                now,
                macros::Frame {
                    version: 1,
                    source_us: frame.source_us,
                    ambiguous: false,
                    contacts_active: busy,
                    hand: pose,
                },
            )
        });
        if let Some(action) = action {
            let mut output = self.action(action);
            let receipt = MacroReceipt {
                action,
                handled: output.action.as_ref().is_some_and(|a| a.handled),
                source_us: frame.source_us,
                mode: output.mode,
                running: output.running,
            };
            self.last_macro_action = Some(receipt);
            output.last_macro_action = Some(receipt);
            if output.action.as_ref().is_some_and(|a| a.handled) {
                return output;
            }
            let mut output = self.output(mouse, tick.transition);
            output.reason = tick.reason;
            output.action = Some(ActionResult {
                action,
                handled: false,
            });
            output.last_macro_action = Some(receipt);
            return output;
        }
        let mut output = self.output(mouse, tick.transition || scroll_released);
        output.reason = if scroll_released {
            Some("scroll_released")
        } else {
            tick.reason
        };
        output
    }
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Command {
    Start,
    Stop,
    Mode {
        mode: Mode,
    },
    SelectHand {
        hand_id: Option<u64>,
    },
    Action {
        action: macros::Action,
    },
    Tick {
        now_us: u64,
    },
    Frame {
        now_us: u64,
        observation: touch_gestures::Frame,
        #[serde(default)]
        pose: Option<macros::HandPose>,
        #[serde(default)]
        middle_pinch: Option<pointer_buttons::Evidence>,
        #[serde(default)]
        detail: Option<crate::detail::Evidence>,
        #[serde(default)]
        escape: Option<crate::escape::Evidence>,
        #[serde(default)]
        // Box only this optional replay payload; raw live observations use Controls directly.
        scroll: Option<Box<crate::scroll::Evidence>>,
    },
}
pub fn replay_stdin(config: &std::path::Path) -> std::io::Result<()> {
    use std::io::{self, Write};
    let mut controller = Controller::new(Settings::load(config)?).map_err(io::Error::other)?;
    let mut out = io::stdout().lock();
    let mut publish = |frame: Output| -> io::Result<()> {
        writeln!(
            out,
            "{}",
            serde_json::json!({"schema":"wigglecontrol.controller-output/v1",
            "camera_opened":false,"system_input":false,"output":frame})
        )?;
        out.flush()
    };
    for command in crate::intent::reader::<Command>() {
        let frame = match command {
            Ok(command) => controller.command(command),
            Err(error) => {
                publish(controller.stop())?;
                return Err(error);
            }
        };
        publish(frame)?;
    }
    publish(controller.stop())
}
