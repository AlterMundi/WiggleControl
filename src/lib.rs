//! Replayable gesture meaning, independent of camera and operating-system grants.
use serde::{Deserialize, Serialize};

pub mod filter;
pub mod macros;

#[cfg(target_os = "linux")]
pub mod controller;

#[cfg(target_os = "linux")]
pub mod delivery;

#[cfg(target_os = "linux")]
pub mod input;

#[cfg(target_os = "linux")]
pub mod recording;
#[cfg(target_os = "linux")]
pub mod review;

#[cfg(target_os = "linux")]
pub mod touch;

#[cfg(target_os = "linux")]
pub mod touch_gestures;

#[cfg(target_os = "linux")]
pub mod calibration;

#[cfg(target_os = "linux")]
pub mod calibration_runtime;

#[cfg(target_os = "linux")]
pub mod guided_calibration;

pub mod prayer;

#[cfg(target_os = "linux")]
pub mod profile_store;

#[cfg(target_os = "linux")]
pub mod metrics;

#[cfg(target_os = "linux")]
mod intent;

#[cfg(feature = "native-vision")]
pub mod vision;

#[cfg(feature = "native-vision")]
pub mod pose_classifier;

#[cfg(feature = "native-vision")]
pub mod sequence;

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
pub struct Hand {
    pub id: u64,
    pub confidence: f64,
    /// Engagement is deliberate, not implied by detection.
    pub engaged: bool,
    /// Producer must reject fists, ambiguous landmarks and occluded fingers.
    pub pinch_eligible: bool,
    #[serde(default)]
    pub scrolling: bool,
    pub cursor: Point,
    pub thumb_tip: Point,
    pub index_tip: Point,
    pub palm_width: f64,
    #[serde(default)]
    pub pinch_distance_m: Option<f64>,
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
        } else if self.pinch_distance_m.is_none()
            && (!self.palm_width.is_finite() || !(0.01..=1.0).contains(&self.palm_width))
        {
            Some("palm_width")
        } else {
            None
        }
    }
    fn valid(&self) -> bool {
        self.invalid_reason().is_none()
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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    /// Normalized displacement intent; adapter applies calibrated gain.
    Move {
        dx: f64,
        dy: f64,
    },
    Button {
        down: bool,
    },
    Scroll {
        vertical: i32,
    },
    Cancel {
        reason: String,
    },
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
    pub stale_us: u64,
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
            stale_us: 150_000,
            smoothing_us: 30_000.0,
            adaptive_smoothing: None,
            scroll_steps_per_unit: 12.0,
        }
    }
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
    armed: bool,
    held: bool,
    candidate: Option<u64>,
    scrolling: bool,
    scroll_remainder: f64,
    last_rejection: Option<&'static str>,
}
impl Engine {
    pub fn new(config: Config) -> Result<Self, &'static str> {
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
            || config.confirm_us == 0
            || config.stale_us <= config.confirm_us
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
            armed: false,
            held: false,
            candidate: None,
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
    pub fn pinch_state(&self) -> &'static str {
        if !self.running {
            "stopped"
        } else if self.hand_id.is_none() {
            "not_eligible"
        } else if self.scrolling {
            "scrolling"
        } else if self.held {
            "pressed"
        } else if self.candidate.is_some() {
            "closing"
        } else if self.armed {
            "open"
        } else {
            "open_first"
        }
    }
    pub fn pinch_block_reason(&self) -> Option<&'static str> {
        match self.pinch_state() {
            "pressed" => None,
            "closing" => Some("confirming"),
            "open_first" => Some("open_before_press"),
            "open" => Some("close_threshold"),
            "scrolling" => Some("scroll_gesture"),
            _ => self.last_rejection,
        }
    }
    pub fn confirmation_elapsed_us(&self) -> u64 {
        self.last_source
            .zip(self.candidate)
            .map(|(last, first)| last.saturating_sub(first))
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
        if self
            .last_received
            .is_some_and(|last| now_us.saturating_sub(last) >= self.config.stale_us)
        {
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
        self.armed = false;
        self.candidate = None;
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
            || observation.hand.as_ref().is_some_and(|h| !h.valid())
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
            events.extend(self.cancel("tracking_lost"));
            return events;
        };
        if !hand.engaged || hand.confidence < self.config.min_confidence || !hand.pinch_eligible {
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
            self.armed = false;
            self.candidate = None;
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
                    events.push(Event::Scroll { vertical: steps });
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
            self.armed = false;
            self.candidate = None;
            return events;
        }
        let (distance, close, open) = hand
            .pinch_distance_m
            .map(|d| (d, self.config.close_m, self.config.open_m))
            .unwrap_or_else(|| {
                (
                    hand.thumb_tip.distance(hand.index_tip) / hand.palm_width,
                    self.config.close_ratio,
                    self.config.open_ratio,
                )
            });
        if distance >= open {
            if self.held {
                events.push(Event::Button { down: false });
            }
            self.held = false;
            self.armed = true;
            self.candidate = None;
        } else if distance <= close && self.armed && !self.held {
            let since = *self.candidate.get_or_insert(observation.source_us);
            if observation.source_us - since >= self.config.confirm_us {
                self.held = true;
                self.candidate = None;
                events.push(Event::Button { down: true });
            }
        } else if !self.held {
            self.candidate = None;
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
