//! Replayable gesture meaning, independent of camera and operating-system grants.
use serde::{Deserialize, Serialize};

pub mod acquisition;
pub mod camera_frame;
pub mod contacts;
pub mod cursor;
pub mod detail;
pub mod escape;
pub mod filter;
pub mod landmarks;
pub mod macros;
pub mod output;
pub mod platform;
pub mod pointer_buttons;
pub mod scroll;
pub mod session_config;
pub mod tracking;

pub mod controller;

#[cfg(target_os = "linux")]
pub use platform::linux::delivery;

#[cfg(target_os = "linux")]
pub use platform::linux::mouse as input;

#[cfg(target_os = "linux")]
pub use platform::linux::keyboard;

#[cfg(target_os = "linux")]
pub mod recording;
#[cfg(target_os = "linux")]
pub mod review;

#[cfg(target_os = "linux")]
pub use platform::linux::touch;

pub mod touch_gestures;

pub mod calibration;

pub mod calibration_runtime;

pub mod guided_calibration;

pub mod provider_calibration;

pub mod prayer;

pub mod profile_store;

#[cfg(target_os = "linux")]
pub mod metrics;

mod intent;

#[cfg(feature = "native-vision")]
pub mod vision;

#[cfg(feature = "provider-runtime")]
pub mod pose_classifier;

#[cfg(feature = "native-vision")]
pub mod sequence;

#[cfg(feature = "provider-runtime")]
pub mod provider;

#[cfg(feature = "native-vision")]
pub mod provider_setup;

#[cfg(feature = "native-vision")]
pub mod provider_fixture;

#[cfg(feature = "native-vision")]
mod camera_mmap;

#[cfg(feature = "native-vision")]
pub mod session;

#[cfg(feature = "native-vision")]
pub mod preview;

pub const SCHEMA_VERSION: u32 = 1;
pub const MAX_MESSAGE_BYTES: usize = 16 * 1024;

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}
impl Point {
    fn valid(self) -> bool {
        self.x.is_finite()
            && self.y.is_finite()
            && (0.0..=1.0).contains(&self.x)
            && (0.0..=1.0).contains(&self.y)
    }
    fn distance(self, other: Self) -> f64 {
        (self.x - other.x).hypot(self.y - other.y)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PinchEvidence {
    /// Stable estimator/crop identity, not a hand, participant or authority.
    pub source_id: u64,
    /// Must identify this exact observation frame, never a cached estimate.
    pub source_us: u64,
    pub confidence: f64,
    pub eligible: bool,
    pub distance_m: f64,
    #[serde(default = "default_onset_allowed", skip_serializing_if = "is_true")]
    pub onset_allowed: bool,
}
impl PinchEvidence {
    fn valid(&self) -> bool {
        self.confidence.is_finite()
            && (0.0..=1.0).contains(&self.confidence)
            && self.distance_m.is_finite()
            && (0.0..=1.0).contains(&self.distance_m)
    }
    fn active(&self, config: &Config) -> bool {
        self.eligible && self.confidence >= config.min_confidence
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Hand {
    pub id: u64,
    pub confidence: f64,
    /// Engagement is deliberate, not implied by detection.
    pub engaged: bool,
    /// Producer must reject fists, ambiguous landmarks and occluded fingers.
    pub pinch_eligible: bool,
    /// Optional producer veto of a NEW press. Does not cancel open rearm,
    /// temporal confirmation or an existing hold; loss still uses eligibility.
    #[serde(default = "default_onset_allowed", skip_serializing_if = "is_true")]
    pub pinch_onset_allowed: bool,
    #[serde(default)]
    pub scrolling: bool,
    pub cursor: Point,
    pub thumb_tip: Point,
    pub index_tip: Point,
    pub palm_width: f64,
    #[serde(default)]
    pub pinch_distance_m: Option<f64>,
    /// Explicit dimensionless pinch metric; mutually exclusive with meters.
    /// Omitted observations retain the legacy tip-gap / palm-width metric.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pinch_distance_ratio: Option<f64>,
    /// One matched same-hand/frame hypothesis; never an additional contact.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secondary_pinch: Option<PinchEvidence>,
}
fn default_onset_allowed() -> bool {
    true
}
fn is_true(value: &bool) -> bool {
    *value
}
fn is_zero(value: &u64) -> bool {
    *value == 0
}
impl Hand {
    pub(crate) fn invalid_reason(&self) -> Option<&'static str> {
        if !self.confidence.is_finite() || !(0.0..=1.0).contains(&self.confidence) {
            Some("confidence")
        } else if !self.cursor.valid() {
            Some("cursor")
        } else if !self.thumb_tip.valid() {
            Some("thumb_tip")
        } else if !self.index_tip.valid() {
            Some("index_tip")
        } else if self
            .pinch_distance_m
            .is_some_and(|d| !d.is_finite() || !(0.0..=1.0).contains(&d))
        {
            Some("pinch_distance_m")
        } else if self
            .pinch_distance_ratio
            .is_some_and(|r| !r.is_finite() || r < 0.0)
        {
            Some("pinch_distance_ratio")
        } else if self.pinch_distance_m.is_some() && self.pinch_distance_ratio.is_some() {
            Some("pinch_metric_conflict")
        } else if self.pinch_distance_m.is_none()
            && self.pinch_distance_ratio.is_none()
            && (!self.palm_width.is_finite() || !(0.01..=1.0).contains(&self.palm_width))
        {
            Some("palm_width")
        } else if self.secondary_pinch.as_ref().is_some_and(|p| !p.valid()) {
            Some("secondary_pinch")
        } else {
            None
        }
    }
    fn valid(&self) -> bool {
        self.invalid_reason().is_none()
    }
    fn valid_for(&self, source_us: u64) -> bool {
        self.valid()
            && self
                .secondary_pinch
                .as_ref()
                .is_none_or(|p| p.source_us == source_us)
    }
    pub(crate) fn pinch_ratio(&self) -> f64 {
        self.pinch_distance_ratio
            .unwrap_or_else(|| self.thumb_tip.distance(self.index_tip) / self.palm_width)
    }
    fn primary_metric(&self, config: &Config) -> (f64, f64, f64) {
        self.pinch_distance_m
            .map(|d| (d, config.close_m, config.open_m))
            .unwrap_or_else(|| (self.pinch_ratio(), config.close_ratio, config.open_ratio))
    }
    fn primary_open(&self, config: &Config) -> bool {
        let (distance, _, open) = self.primary_metric(config);
        distance >= open
    }
    fn open(&self, config: &Config, all: bool) -> bool {
        if !self.engaged || !self.pinch_eligible || self.confidence < config.min_confidence {
            return false;
        }
        let primary = self.primary_open(config);
        let secondary = self.secondary_pinch.as_ref().filter(|p| p.active(config));
        if all {
            primary && secondary.is_none_or(|p| p.distance_m >= config.open_m)
        } else {
            primary || secondary.is_some_and(|p| p.distance_m >= config.open_m)
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Observation {
    pub version: u32,
    /// Same monotonic microsecond epoch as the consumer's now_us.
    pub source_us: u64,
    pub hand: Option<Hand>,
}

/// Logical mouse buttons, independent of platform event codes.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

fn wheel_zero(value: &i32) -> bool {
    *value == 0
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Event {
    /// Normalized displacement intent; adapter applies calibrated gain.
    Move {
        dx: f64,
        dy: f64,
    },
    /// Existing pinch event; always the left button.
    Button {
        down: bool,
    },
    /// Explicit button selection; legacy `button` remains left-only.
    NamedButton {
        button: MouseButton,
        down: bool,
    },
    Scroll {
        vertical: i32,
        #[serde(default, skip_serializing_if = "wheel_zero")]
        horizontal: i32,
    },
    /// High-resolution wheel units:120 units equal one detent.
    ScrollFine {
        vertical: i32,
        horizontal: i32,
    },
    Cancel {
        reason: String,
    },
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReleaseConfig {
    /// Successive eligible openings without intervening measured closure/deadband.
    pub confirm_us: u64,
    /// A wider opening bypasses confirmation. Relative to the selected open metric.
    pub immediate_open_factor: f64,
}
impl ReleaseConfig {
    fn validate(self, stale_us: u64) -> Result<(), &'static str> {
        if !(1_000..=200_000).contains(&self.confirm_us)
            || self.confirm_us >= stale_us
            || !self.immediate_open_factor.is_finite()
            || !(1.0..=3.0).contains(&self.immediate_open_factor)
        {
            return Err(
                "release confirmation requires1..200ms below freshness and open factor1..3",
            );
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub min_confidence: f64,
    pub close_ratio: f64,
    pub open_ratio: f64,
    pub close_m: f64,
    pub open_m: f64,
    pub confirm_us: u64,
    /// Opt-in opening evidence filter; omission preserves immediate release.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub release_confirmation: Option<ReleaseConfig>,
    pub stale_us: u64,
    /// Opt-in bounded continuity of an established drag; never source freshness.
    #[serde(skip_serializing_if = "is_zero")]
    pub drag_grace_us: u64,
    pub smoothing_us: f64,
    /// Opt-in experiment; None preserves the original exponential filter.
    pub adaptive_smoothing: Option<filter::OneEuroConfig>,
    pub scroll_steps_per_unit: f64,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            min_confidence: 0.7,
            close_ratio: 0.25,
            open_ratio: 0.4,
            close_m: 0.02,
            open_m: 0.03,
            confirm_us: 40_000,
            release_confirmation: None,
            stale_us: 150_000,
            drag_grace_us: 0,
            smoothing_us: 30_000.0,
            adaptive_smoothing: None,
            scroll_steps_per_unit: 12.0,
        }
    }
}

#[derive(Default)]
struct PinchTrack {
    armed: bool,
    held: bool,
    candidate: Option<u64>,
    opening: Option<u64>,
    vetoed: bool,
}
impl PinchTrack {
    fn observe(
        &mut self,
        source_us: u64,
        distance: f64,
        close: f64,
        open: f64,
        config: &Config,
        allowed: bool,
    ) {
        self.vetoed = false;
        if distance >= open {
            if self.held {
                if let Some(release) = config.release_confirmation {
                    if distance < open * release.immediate_open_factor {
                        let since = *self.opening.get_or_insert(source_us);
                        if source_us - since < release.confirm_us {
                            return;
                        }
                    }
                }
            }
            self.held = false;
            self.armed = true;
            self.candidate = None;
            self.opening = None;
            return;
        }
        self.opening = None;
        if distance <= close && self.armed && !self.held {
            let since = *self.candidate.get_or_insert(source_us);
            if source_us - since >= config.confirm_us {
                if allowed {
                    self.held = true;
                    self.candidate = None;
                } else {
                    self.vetoed = true;
                }
            }
        } else if !self.held {
            self.candidate = None;
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct DragHold {
    pub hand_id: u64,
    pub source_us: u64,
    pub until_us: u64,
    pub in_grace: bool,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct ReleasePending {
    pub since_source_us: u64,
    pub elapsed_us: u64,
    pub required_us: u64,
}

pub struct Engine {
    config: Config,
    running: bool,
    watermark: Option<u64>,
    last_now: Option<u64>,
    last_source: Option<u64>,
    last_received: Option<u64>,
    hand_id: Option<u64>,
    cursor: Option<Point>,
    cursor_filter: filter::OneEuro,
    held: bool,
    drag_gap: bool,
    pinches: [PinchTrack; 2],
    secondary_source: Option<u64>,
    scrolling: bool,
    scroll_remainder: f64,
    last_rejection: Option<&'static str>,
}
impl Engine {
    pub fn new(config: Config) -> Result<Self, &'static str> {
        if let Some(release) = config.release_confirmation {
            release.validate(config.stale_us)?;
        }
        if let Some(filter) = config.adaptive_smoothing {
            filter.validate()?;
        }
        if !config.min_confidence.is_finite()
            || !(0.0..=1.0).contains(&config.min_confidence)
            || !config.close_ratio.is_finite()
            || !config.open_ratio.is_finite()
            || config.close_ratio <= 0.0
            || config.open_ratio <= config.close_ratio
            || !config.close_m.is_finite()
            || !config.open_m.is_finite()
            || config.close_m <= 0.0
            || config.open_m <= config.close_m
            || config.open_m > 1.0
            || config.stale_us <= config.confirm_us
            || config.drag_grace_us > 2_000_000
            || (config.drag_grace_us != 0 && config.drag_grace_us < config.stale_us)
            || !config.smoothing_us.is_finite()
            || config.smoothing_us < 0.0
            || !config.scroll_steps_per_unit.is_finite()
            || !(1.0..=20.0).contains(&config.scroll_steps_per_unit)
        {
            return Err("invalid gesture configuration");
        }
        Ok(Self {
            config,
            running: false,
            watermark: None,
            last_now: None,
            last_source: None,
            last_received: None,
            hand_id: None,
            cursor: None,
            cursor_filter: filter::OneEuro::default(),
            held: false,
            drag_gap: false,
            pinches: Default::default(),
            secondary_source: None,
            scrolling: false,
            scroll_remainder: 0.0,
            last_rejection: None,
        })
    }
    pub fn start(&mut self) -> Vec<Event> {
        let events = self.cancel("restart");
        self.running = true;
        events
    }
    pub fn stop(&mut self) -> Vec<Event> {
        self.running = false;
        self.cancel("stop")
    }
    /// Read-only diagnostic state; does not modify gesture decisions.
    pub fn cursor_position(&self) -> Option<Point> {
        self.cursor
    }
    pub fn drag_hold(&self) -> Option<DragHold> {
        if !self.held || self.config.drag_grace_us == 0 || self.scrolling {
            return None;
        }
        let source_us = self.last_source?;
        Some(DragHold {
            hand_id: self.hand_id?,
            source_us,
            until_us: source_us.saturating_add(self.config.drag_grace_us),
            in_grace: self.drag_gap
                || self
                    .last_now
                    .is_some_and(|now| now.saturating_sub(source_us) >= self.config.stale_us),
        })
    }
    fn retain_drag(&mut self, now: u64) -> bool {
        if self.drag_hold().is_some_and(|h| now < h.until_us) {
            self.drag_gap = true;
            true
        } else {
            false
        }
    }
    pub fn release_pending(&self) -> Option<ReleasePending> {
        let required_us = self.config.release_confirmation?.confirm_us;
        let since_source_us = self.pinches.iter().filter_map(|p| p.opening).min()?;
        Some(ReleasePending {
            since_source_us,
            elapsed_us: self.last_source?.saturating_sub(since_source_us),
            required_us,
        })
    }
    pub fn pinch_state(&self) -> &'static str {
        if !self.running {
            "stopped"
        } else if self.hand_id.is_none() {
            "not_eligible"
        } else if self.scrolling {
            "scrolling"
        } else if self.held {
            "pressed"
        } else if self.pinches.iter().any(|p| p.candidate.is_some()) {
            "closing"
        } else if self.pinches.iter().any(|p| p.armed) {
            "open"
        } else {
            "open_first"
        }
    }
    pub fn pinch_block_reason(&self) -> Option<&'static str> {
        match self.pinch_state() {
            "pressed" => None,
            "closing" if self.last_rejection == Some("pinch_onset_veto") => self.last_rejection,
            "closing" => Some("confirming"),
            "open_first" => Some("open_before_press"),
            "open" => Some("close_threshold"),
            "scrolling" => Some("scroll_gesture"),
            _ => self.last_rejection,
        }
    }
    pub fn confirmation_elapsed_us(&self) -> u64 {
        self.pinches
            .iter()
            .filter_map(|p| p.candidate)
            .filter_map(|first| self.last_source.map(|last| last.saturating_sub(first)))
            .max()
            .unwrap_or(0)
    }
    pub fn reject(&mut self) -> Vec<Event> {
        self.cancel("malformed_input")
    }
    pub fn tick(&mut self, now_us: u64) -> Vec<Event> {
        if self.last_now.is_some_and(|last| now_us < last) {
            return self.cancel("consumer_clock_regression");
        }
        self.last_now = Some(now_us);
        if self.drag_hold().is_some_and(|h| now_us >= h.until_us) {
            return self.cancel("drag_grace_expired");
        }
        if self
            .last_received
            .is_some_and(|last| now_us.saturating_sub(last) >= self.config.stale_us)
        {
            if self.retain_drag(now_us) {
                return Vec::new();
            }
            return self.cancel("stale_input");
        }
        Vec::new()
    }
    fn cancel(&mut self, reason: &'static str) -> Vec<Event> {
        self.last_rejection = Some(reason);
        let mut events = Vec::new();
        if self.held {
            events.push(Event::Button { down: false });
        }
        if self.hand_id.is_some() {
            events.push(Event::Cancel {
                reason: reason.into(),
            });
        }
        self.held = false;
        self.drag_gap = false;
        self.pinches = Default::default();
        self.secondary_source = None;
        self.hand_id = None;
        self.cursor = None;
        self.last_source = None;
        self.last_received = None;
        self.scrolling = false;
        self.scroll_remainder = 0.0;
        events
    }
    pub fn observe(&mut self, now_us: u64, observation: Observation) -> Vec<Event> {
        if self.last_now.is_some_and(|last| now_us < last) {
            return self.cancel("consumer_clock_regression");
        }
        let mut events = self.tick(now_us);
        if !self.running {
            return events;
        }
        if observation.version != SCHEMA_VERSION
            || observation.source_us > now_us
            || now_us - observation.source_us >= self.config.stale_us
            || self
                .watermark
                .is_some_and(|last| observation.source_us <= last)
            || observation
                .hand
                .as_ref()
                .is_some_and(|h| !h.valid_for(observation.source_us))
        {
            events.extend(self.cancel("invalid_observation"));
            self.last_rejection = Some(if observation.version != SCHEMA_VERSION {
                "unsupported_version"
            } else if observation.source_us > now_us {
                "future_frame"
            } else if now_us.saturating_sub(observation.source_us) >= self.config.stale_us {
                "stale_frame"
            } else if self
                .watermark
                .is_some_and(|last| observation.source_us <= last)
            {
                "old_frame"
            } else {
                "invalid_hand"
            });
            return events;
        }
        self.watermark = Some(observation.source_us);
        let Some(hand) = observation.hand else {
            if self.retain_drag(now_us) {
                return events;
            }
            events.extend(self.cancel("tracking_lost"));
            return events;
        };
        if !hand.engaged || hand.confidence < self.config.min_confidence || !hand.pinch_eligible {
            if hand.engaged && self.hand_id == Some(hand.id) && self.retain_drag(now_us) {
                return events;
            }
            events.extend(self.cancel("hand_not_eligible"));
            self.last_rejection = Some(if !hand.engaged {
                "hand_not_engaged"
            } else if hand.confidence < self.config.min_confidence {
                "low_confidence"
            } else {
                "pinch_not_eligible"
            });
            return events;
        }
        self.last_rejection = None;
        self.drag_gap = false;
        if self.hand_id != Some(hand.id) {
            events.extend(self.cancel("hand_changed"));
            self.hand_id = Some(hand.id);
            self.cursor = Some(hand.cursor);
            self.cursor_filter.reset(hand.cursor);
        }
        if self.scrolling != hand.scrolling {
            if self.held {
                events.push(Event::Button { down: false });
            }
            self.held = false;
            self.pinches = Default::default();
            self.secondary_source = None;
            self.scrolling = hand.scrolling;
            self.scroll_remainder = 0.0;
            self.cursor = Some(hand.cursor);
            self.cursor_filter.reset(hand.cursor);
        }
        if let (Some(previous), Some(last)) = (self.cursor, self.last_source) {
            let dt = (observation.source_us - last) as f64;
            let alpha = if self.config.smoothing_us == 0.0 {
                1.0
            } else {
                1.0 - (-dt / self.config.smoothing_us).exp()
            };
            let next = if let Some(filter) = self.config.adaptive_smoothing {
                self.cursor_filter.apply(
                    filter,
                    previous,
                    hand.cursor,
                    observation.source_us - last,
                )
            } else {
                Point {
                    x: previous.x + alpha * (hand.cursor.x - previous.x),
                    y: previous.y + alpha * (hand.cursor.y - previous.y),
                }
            };
            let (dx, dy) = (next.x - previous.x, next.y - previous.y);
            if hand.scrolling {
                self.scroll_remainder -= dy * self.config.scroll_steps_per_unit;
                let steps = (self.scroll_remainder.trunc() as i32).clamp(-10, 10);
                self.scroll_remainder -= f64::from(steps);
                if steps != 0 {
                    events.push(Event::Scroll {
                        vertical: steps,
                        horizontal: 0,
                    });
                }
            } else if dx != 0.0 || dy != 0.0 {
                events.push(Event::Move { dx, dy });
            }
            self.cursor = Some(next);
        }
        self.last_source = Some(observation.source_us);
        self.last_received = Some(now_us);
        self.last_rejection = None;
        if hand.scrolling {
            self.pinches = Default::default();
            self.secondary_source = None;
            return events;
        }
        let (distance, close, open) = hand.primary_metric(&self.config);
        self.pinches[0].observe(
            observation.source_us,
            distance,
            close,
            open,
            &self.config,
            hand.pinch_onset_allowed,
        );
        if let Some(secondary) = hand
            .secondary_pinch
            .as_ref()
            .filter(|p| p.active(&self.config))
        {
            if self.secondary_source != Some(secondary.source_id) {
                self.pinches[1] = PinchTrack::default();
            }
            self.secondary_source = Some(secondary.source_id);
            self.pinches[1].observe(
                observation.source_us,
                secondary.distance_m,
                self.config.close_m,
                self.config.open_m,
                &self.config,
                secondary.onset_allowed,
            );
        } else {
            self.pinches[1] = PinchTrack::default();
            self.secondary_source = None;
        }
        let held = self.pinches.iter().any(|p| p.held);
        if held != self.held {
            events.push(Event::Button { down: held });
            self.held = held;
        }
        if !held && self.pinches.iter().any(|p| p.vetoed) {
            self.last_rejection = Some("pinch_onset_veto");
        }
        events
    }
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum ReplayCommand {
    Start,
    Stop,
    Tick {
        now_us: u64,
    },
    Frame {
        now_us: u64,
        observation: Observation,
    },
}
pub fn replay(engine: &mut Engine, command: ReplayCommand) -> Vec<Event> {
    match command {
        ReplayCommand::Start => engine.start(),
        ReplayCommand::Stop => engine.stop(),
        ReplayCommand::Tick { now_us } => engine.tick(now_us),
        ReplayCommand::Frame {
            now_us,
            observation,
        } => engine.observe(now_us, observation),
    }
}
