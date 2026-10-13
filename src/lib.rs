//! Replayable gesture meaning, independent of camera and operating-system grants.
use serde::{Deserialize, Serialize};

#[cfg(target_os = "linux")]
pub mod input;

#[cfg(target_os = "linux")]
pub mod touch;

#[cfg(target_os = "linux")]
pub mod touch_gestures;

#[cfg(target_os = "linux")]
pub mod metrics;

#[cfg(target_os = "linux")]
mod intent;

#[cfg(feature = "native-vision")]
pub mod vision;

#[cfg(feature = "native-vision")]
pub mod session;

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
}
impl Hand {
    fn valid(&self) -> bool {
        self.confidence.is_finite()
            && (0.0..=1.0).contains(&self.confidence)
            && self.cursor.valid()
            && self.thumb_tip.valid()
            && self.index_tip.valid()
            && self.palm_width.is_finite()
            && (0.01..=1.0).contains(&self.palm_width)
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

#[derive(Clone, Copy, Debug)]
pub struct Config {
    pub min_confidence: f64,
    pub close_ratio: f64,
    pub open_ratio: f64,
    pub confirm_us: u64,
    pub stale_us: u64,
    pub smoothing_us: f64,
    pub scroll_steps_per_unit: f64,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            min_confidence: 0.7,
            close_ratio: 0.25,
            open_ratio: 0.4,
            confirm_us: 40_000,
            stale_us: 150_000,
            smoothing_us: 30_000.0,
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
    armed: bool,
    held: bool,
    candidate: Option<u64>,
    scrolling: bool,
    scroll_remainder: f64,
}
impl Engine {
    pub fn new(config: Config) -> Result<Self, &'static str> {
        if !config.min_confidence.is_finite()
            || !(0.0..=1.0).contains(&config.min_confidence)
            || !config.close_ratio.is_finite()
            || !config.open_ratio.is_finite()
            || config.close_ratio <= 0.0
            || config.open_ratio <= config.close_ratio
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
            armed: false,
            held: false,
            candidate: None,
            scrolling: false,
            scroll_remainder: 0.0,
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
    fn cancel(&mut self, reason: &str) -> Vec<Event> {
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
            return events;
        }
        self.watermark = Some(observation.source_us);
        let Some(hand) = observation.hand else {
            events.extend(self.cancel("tracking_lost"));
            return events;
        };
        if !hand.engaged || hand.confidence < self.config.min_confidence || !hand.pinch_eligible {
            events.extend(self.cancel("hand_not_eligible"));
            return events;
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
        }
        if self.hand_id != Some(hand.id) {
            events.extend(self.cancel("hand_changed"));
            self.hand_id = Some(hand.id);
            self.cursor = Some(hand.cursor);
        } else if let (Some(previous), Some(last)) = (self.cursor, self.last_source) {
            let dt = (observation.source_us - last) as f64;
            let alpha = if self.config.smoothing_us == 0.0 {
                1.0
            } else {
                1.0 - (-dt / self.config.smoothing_us).exp()
            };
            let next = Point {
                x: previous.x + alpha * (hand.cursor.x - previous.x),
                y: previous.y + alpha * (hand.cursor.y - previous.y),
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
        if hand.scrolling {
            self.armed = false;
            self.candidate = None;
            return events;
        }
        let ratio = hand.thumb_tip.distance(hand.index_tip) / hand.palm_width;
        if ratio >= self.config.open_ratio {
            if self.held {
                events.push(Event::Button { down: false });
            }
            self.held = false;
            self.armed = true;
            self.candidate = None;
        } else if ratio <= self.config.close_ratio && self.armed && !self.held {
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
