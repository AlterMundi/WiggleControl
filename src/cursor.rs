//! Optional absolute cursor reconstruction; never creates observations or buttons.
use crate::Point;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct ClickConfig {
    pub radius: f64,
    pub closing_us: u64,
    pub pressed_us: u64,
}
impl Default for ClickConfig {
    fn default() -> Self {
        Self {
            radius: 0.008,
            closing_us: 200_000,
            pressed_us: 150_000,
        }
    }
}
impl ClickConfig {
    fn validate(self) -> Result<(), &'static str> {
        if !self.radius.is_finite()
            || !(0.0001..=0.05).contains(&self.radius)
            || !(20_000..=500_000).contains(&self.closing_us)
            || !(20_000..=500_000).contains(&self.pressed_us)
        {
            return Err(
                "click stabilization requires radius0.0001..0.05 and closing/pressed20..500ms",
            );
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub interval_us: u64,
    pub smoothing_us: f64,
    pub prediction_us: u64,
    /// Additional bound in normalized screen distance, independent of time.
    pub max_prediction_distance: f64,
    pub consistent_prediction: bool,
    pub velocity_smoothing_us: f64,
    /// Euclidean speed in normalized screen coordinates/second.
    pub max_speed_per_s: f64,
    pub click_stabilization: Option<ClickConfig>,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            interval_us: 16_667,
            smoothing_us: 30_000.0,
            prediction_us: 80_000,
            max_prediction_distance: 0.01,
            consistent_prediction: true,
            velocity_smoothing_us: 80_000.0,
            max_speed_per_s: 2.0,
            click_stabilization: None,
        }
    }
}
impl Config {
    pub fn validate(self) -> Result<(), &'static str> {
        if let Some(click) = self.click_stabilization {
            click.validate()?;
        }
        if !(4_000..=50_000).contains(&self.interval_us)
            || !self.smoothing_us.is_finite()
            || !(0.0..=200_000.0).contains(&self.smoothing_us)
            || self.prediction_us > 100_000
            || !self.max_prediction_distance.is_finite()
            || !(0.0..=1.0).contains(&self.max_prediction_distance)
            || !self.velocity_smoothing_us.is_finite()
            || !(0.0..=200_000.0).contains(&self.velocity_smoothing_us)
            || !self.max_speed_per_s.is_finite()
            || !(0.05..=10.0).contains(&self.max_speed_per_s)
        {
            return Err("cursor output requires interval4..50ms, smoothing0..200ms, prediction0..100ms, lead0..1, speed0.05..10/s");
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct Sample {
    pub source_us: u64,
    pub estimated_us: u64,
    pub observed: Point,
    pub predicted: Point,
    pub estimated: Point,
    pub prediction_us: u64,
    pub frozen: bool,
    pub click_locked: bool,
}

#[derive(Clone, Copy, Default)]
enum Click {
    #[default]
    Idle,
    Locked {
        observed: Point,
        output: Point,
        started: u64,
        pressed_at: Option<u64>,
    },
    Bypass,
    Release,
}
impl Click {
    fn deadline(self, config: ClickConfig) -> Option<u64> {
        match self {
            Self::Locked {
                started,
                pressed_at,
                ..
            } => Some(pressed_at.map_or_else(
                || started.saturating_add(config.closing_us),
                |t| t.saturating_add(config.pressed_us),
            )),
            _ => None,
        }
    }
    fn observe(
        &mut self,
        config: ClickConfig,
        now: u64,
        observed: Point,
        active: bool,
        pressed: bool,
        previous: Option<Sample>,
    ) -> Option<Point> {
        if !active {
            let released = match *self {
                Self::Locked {
                    output,
                    pressed_at: Some(_),
                    ..
                } => Some(output),
                _ => None,
            };
            *self = if released.is_some() {
                Self::Release
            } else {
                Self::Idle
            };
            return released;
        }
        if matches!(self, Self::Idle | Self::Release) {
            let anchor = previous
                .map(|s| (s.observed, s.estimated))
                .unwrap_or((observed, observed));
            *self = Self::Locked {
                observed: anchor.0,
                output: anchor.1,
                started: now,
                pressed_at: pressed.then_some(now),
            };
        }
        if self
            .deadline(config)
            .is_some_and(|deadline| now >= deadline)
        {
            *self = Self::Bypass;
            return None;
        }
        if let Self::Locked {
            observed: anchor,
            output,
            pressed_at,
            ..
        } = self
        {
            if observed.distance(*anchor) > config.radius {
                *self = Self::Bypass;
                return None;
            }
            if pressed && pressed_at.is_none() {
                *pressed_at = Some(now);
            }
            return Some(*output);
        }
        None
    }
}

pub(crate) struct Cursor {
    config: Config,
    sample: Option<Sample>,
    velocity: Point,
    next_emit_us: u64,
    consistent_intervals: u8,
    click: Click,
}
fn alpha(dt_us: u64, tau_us: f64) -> f64 {
    if tau_us == 0.0 {
        1.0
    } else {
        1.0 - (-(dt_us as f64) / tau_us).exp()
    }
}
fn limit(delta: Point, maximum: f64) -> Point {
    let length = delta.x.hypot(delta.y);
    let scale = if length > maximum {
        maximum / length
    } else {
        1.0
    };
    Point {
        x: delta.x * scale,
        y: delta.y * scale,
    }
}
impl Cursor {
    pub fn new(config: Config) -> Result<Self, &'static str> {
        config.validate()?;
        Ok(Self {
            config,
            sample: None,
            velocity: Point { x: 0.0, y: 0.0 },
            next_emit_us: 0,
            consistent_intervals: 0,
            click: Click::Idle,
        })
    }
    pub fn reset(&mut self) {
        self.sample = None;
        self.velocity = Point { x: 0.0, y: 0.0 };
        self.next_emit_us = 0;
        self.consistent_intervals = 0;
        self.click = Click::Idle;
    }
    pub fn feedback(&self) -> Option<Sample> {
        self.sample
    }
    pub fn observe(
        &mut self,
        source_us: u64,
        now: u64,
        observed: Point,
        closing: bool,
        pressed: bool,
    ) -> Result<(), &'static str> {
        if source_us > now
            || !observed.valid()
            || self
                .sample
                .is_some_and(|s| source_us <= s.source_us || now < s.estimated_us)
        {
            return Err("cursor observation requires ordered, finite source evidence");
        }
        let lock = self.config.click_stabilization.and_then(|cfg| {
            self.click
                .observe(cfg, now, observed, closing || pressed, pressed, self.sample)
        });
        if let Some(sample) = &mut self.sample {
            if sample.frozen || sample.click_locked {
                if sample.click_locked {
                    sample.estimated_us = now;
                }
                self.velocity = Point { x: 0.0, y: 0.0 };
                self.consistent_intervals = 0;
            } else {
                let dt = source_us - sample.source_us;
                let v = limit(
                    Point {
                        x: (observed.x - sample.observed.x) * 1_000_000.0 / dt as f64,
                        y: (observed.y - sample.observed.y) * 1_000_000.0 / dt as f64,
                    },
                    self.config.max_speed_per_s,
                );
                let speed = v.x.hypot(v.y);
                let previous_speed = self.velocity.x.hypot(self.velocity.y);
                let consistent = previous_speed > 1e-9
                    && speed >= 0.5 * previous_speed
                    && speed <= 2.0 * previous_speed
                    && v.x * self.velocity.x + v.y * self.velocity.y > 0.9 * speed * previous_speed;
                self.consistent_intervals = if consistent {
                    self.consistent_intervals.saturating_add(1).min(3)
                } else {
                    0
                };
                let a = alpha(dt, self.config.velocity_smoothing_us);
                self.velocity.x += a * (v.x - self.velocity.x);
                self.velocity.y += a * (v.y - self.velocity.y);
            }
            sample.observed = observed;
            sample.source_us = source_us;
            sample.frozen = false;
            sample.click_locked = false;
        } else {
            self.sample = Some(Sample {
                source_us,
                estimated_us: now,
                observed,
                predicted: observed,
                estimated: observed,
                prediction_us: 0,
                frozen: false,
                click_locked: false,
            });
            self.next_emit_us = now.saturating_add(self.config.interval_us);
        }
        if let Some(point) = lock {
            let sample = self.sample.as_mut().unwrap();
            sample.estimated = point;
            sample.predicted = point;
            sample.estimated_us = now;
            sample.prediction_us = 0;
            sample.click_locked = true;
            self.velocity = Point { x: 0.0, y: 0.0 };
            self.consistent_intervals = 0;
        }
        Ok(())
    }
    pub fn freeze(&mut self, now: u64) -> Result<(), &'static str> {
        if let Some(sample) = &mut self.sample {
            if now < sample.estimated_us {
                return Err("cursor clock regressed");
            }
            sample.estimated_us = now;
            sample.prediction_us = 0;
            sample.frozen = true;
            sample.predicted = sample.estimated;
        }
        self.velocity = Point { x: 0.0, y: 0.0 };
        self.consistent_intervals = 0;
        self.next_emit_us = now.saturating_add(self.config.interval_us);
        Ok(())
    }
    pub fn sample(&mut self, now: u64, force: bool) -> Result<Option<Sample>, &'static str> {
        if self.config.click_stabilization.is_some_and(|cfg| {
            self.click
                .deadline(cfg)
                .is_some_and(|deadline| now >= deadline)
        }) {
            self.click = Click::Bypass;
            if let Some(sample) = &mut self.sample {
                sample.click_locked = false;
            }
            self.velocity = Point { x: 0.0, y: 0.0 };
            self.consistent_intervals = 0;
        }
        let Some(sample) = &mut self.sample else {
            return Ok(None);
        };
        if now < sample.estimated_us {
            return Err("cursor clock regressed");
        }
        if sample.click_locked {
            if force {
                sample.estimated_us = now;
                let locked = *sample;
                if matches!(self.click, Click::Release) {
                    self.click = Click::Idle;
                    sample.click_locked = false;
                }
                return Ok(Some(locked));
            }
            return Ok(None);
        }
        if sample.frozen || (!force && now < self.next_emit_us) {
            return Ok(None);
        }
        let dt = now - sample.estimated_us;
        let mut prediction = now
            .saturating_sub(sample.source_us)
            .min(self.config.prediction_us);
        let speed = self.velocity.x.hypot(self.velocity.y);
        if speed < 1e-9 || (self.config.consistent_prediction && self.consistent_intervals < 2) {
            prediction = 0;
        } else {
            prediction =
                prediction.min((self.config.max_prediction_distance / speed * 1_000_000.0) as u64);
        }
        let seconds = prediction as f64 / 1_000_000.0;
        let target = Point {
            x: (sample.observed.x + self.velocity.x * seconds).clamp(0.0, 1.0),
            y: (sample.observed.y + self.velocity.y * seconds).clamp(0.0, 1.0),
        };
        sample.predicted = target;
        let a = alpha(dt, self.config.smoothing_us);
        let change = limit(
            Point {
                x: (target.x - sample.estimated.x) * a,
                y: (target.y - sample.estimated.y) * a,
            },
            self.config.max_speed_per_s * dt as f64 / 1_000_000.0,
        );
        sample.estimated.x = (sample.estimated.x + change.x).clamp(0.0, 1.0);
        sample.estimated.y = (sample.estimated.y + change.y).clamp(0.0, 1.0);
        sample.estimated_us = now;
        sample.prediction_us = prediction;
        if now >= self.next_emit_us {
            let missed = (now - self.next_emit_us) / self.config.interval_us;
            self.next_emit_us = self.next_emit_us.saturating_add(
                missed
                    .saturating_add(1)
                    .saturating_mul(self.config.interval_us),
            );
        }
        Ok(Some(*sample))
    }
}
