//! Speed-adaptive cursor filtering. Coordinates are normalized camera units.
//! Independent implementation of Casiez et al., CHI 2012 (https://gery.casiez.net/1euro/).
use crate::Point;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OneEuroConfig {
    pub min_cutoff_hz: f64,
    /// Cutoff increase per normalized coordinate unit/second.
    pub beta: f64,
    pub derivative_cutoff_hz: f64,
    /// Ignore low estimated speed when increasing cutoff, without freezing position.
    #[serde(default)]
    pub speed_floor: f64,
}
impl OneEuroConfig {
    pub fn validate(self) -> Result<(), &'static str> {
        if !self.min_cutoff_hz.is_finite()
            || !(0.01..=120.0).contains(&self.min_cutoff_hz)
            || !self.derivative_cutoff_hz.is_finite()
            || !(0.01..=120.0).contains(&self.derivative_cutoff_hz)
            || !self.beta.is_finite()
            || !(0.0..=1000.0).contains(&self.beta)
            || !self.speed_floor.is_finite()
            || !(0.0..=1.0).contains(&self.speed_floor)
        {
            return Err(
                "adaptive smoothing requires cutoff 0.01..120 Hz, beta 0..1000, speed_floor 0..1",
            );
        }
        Ok(())
    }
}
pub(crate) struct OneEuro {
    raw: Option<Point>,
    derivative: Point,
}
impl Default for OneEuro {
    fn default() -> Self {
        Self {
            raw: None,
            derivative: Point { x: 0.0, y: 0.0 },
        }
    }
}
impl OneEuro {
    pub fn reset(&mut self, raw: Point) {
        self.raw = Some(raw);
        self.derivative = Point { x: 0.0, y: 0.0 };
    }
    pub fn apply(
        &mut self,
        config: OneEuroConfig,
        previous: Point,
        raw: Point,
        dt_us: u64,
    ) -> Point {
        // Engine validates strictly increasing source timestamps before this call.
        let dt = dt_us as f64 / 1_000_000.0;
        let alpha = |hz: f64| 1.0 / (1.0 + 1.0 / (std::f64::consts::TAU * hz * dt));
        let last_raw = self.raw.unwrap_or(raw);
        let derivative_alpha = alpha(config.derivative_cutoff_hz);
        self.derivative.x += derivative_alpha * ((raw.x - last_raw.x) / dt - self.derivative.x);
        self.derivative.y += derivative_alpha * ((raw.y - last_raw.y) / dt - self.derivative.y);
        self.raw = Some(raw);
        // One shared speed-dependent cutoff avoids axis-dependent response on diagonals.
        let speed = (self.derivative.x.hypot(self.derivative.y) - config.speed_floor).max(0.0);
        let a = alpha(config.min_cutoff_hz + config.beta * speed);
        Point {
            x: previous.x + a * (raw.x - previous.x),
            y: previous.y + a * (raw.y - previous.y),
        }
    }
}
