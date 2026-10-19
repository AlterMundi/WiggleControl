//! Held precision evidence and continuous calibrated-screen mapping.
//! No buttons, clocks, sensors or OS output are synthesized here.
use crate::Point;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub gain: f64,
    /// Radius in calibrated hand-workspace units, before applying detail gain.
    pub rest_radius: f64,
    pub confirm_us: u64,
    pub folded_ratio: f64,
    pub extended_ratio: f64,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            gain: 0.25,
            rest_radius: 0.001,
            confirm_us: 100_000,
            folded_ratio: 0.55,
            extended_ratio: 0.8,
        }
    }
}
impl Config {
    pub fn validate(self) -> Result<(), &'static str> {
        if !self.gain.is_finite()
            || !(0.05..=1.0).contains(&self.gain)
            || !self.rest_radius.is_finite()
            || !(0.0..=0.01).contains(&self.rest_radius)
            || !(20_000..=500_000).contains(&self.confirm_us)
            || !self.folded_ratio.is_finite()
            || !self.extended_ratio.is_finite()
            || !(0.1..=0.8).contains(&self.folded_ratio)
            || !(self.folded_ratio + 0.1..=1.0).contains(&self.extended_ratio)
        {
            return Err("invalid held-detail gain, rest radius, confirmation or finger ratios");
        }
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Evidence {
    pub hand_id: u64,
    pub source_us: u64,
    pub held: bool,
}
#[derive(Default)]
pub(crate) struct Held {
    active: bool,
    armed: bool,
    candidate: Option<(bool, u64)>,
    last_source: Option<u64>,
}
impl Held {
    pub fn active(&self) -> bool {
        self.active
    }
    pub fn observe(&mut self, config: Config, source: u64, held: Option<bool>, max_gap_us: u64) {
        if self
            .last_source
            .is_some_and(|previous| source.saturating_sub(previous) >= max_gap_us)
        {
            self.candidate = None;
        }
        self.last_source = Some(source);
        let Some(held) = held else {
            self.candidate = None;
            return;
        };
        if held == self.active && (self.armed || held) {
            self.candidate = None;
            return;
        }
        let since = self
            .candidate
            .filter(|(v, _)| *v == held)
            .map_or(source, |(_, t)| t);
        self.candidate = Some((held, since));
        if source.saturating_sub(since) >= config.confirm_us {
            // Positively extended ring/little fingers must first establish screen mode.
            if !held {
                self.armed = true;
            }
            if self.armed {
                self.active = held;
            }
            self.candidate = None;
        }
    }
}
/// Chord-to-path ratio: insensitive to uniform scale and rigid rotation.
/// Unknown/degenerate bones never establish a finger pose.
pub fn finger_ratio(points: [[f64; 3]; 4]) -> Option<f64> {
    if points.iter().flatten().any(|v| !v.is_finite()) {
        return None;
    }
    let distance = |a: [f64; 3], b: [f64; 3]| (a[0] - b[0]).hypot(a[1] - b[1]).hypot(a[2] - b[2]);
    let mut path = 0.0;
    for bone in points.windows(2) {
        let length = distance(bone[0], bone[1]);
        if length < 1e-6 {
            return None;
        }
        path += length;
    }
    Some((distance(points[0], points[3]) / path).clamp(0.0, 1.0))
}
pub fn folded(config: Config, ring: [[f64; 3]; 4], little: [[f64; 3]; 4]) -> Option<bool> {
    let (ring, little) = (finger_ratio(ring)?, finger_ratio(little)?);
    if ring <= config.folded_ratio && little <= config.folded_ratio {
        Some(true)
    } else if ring >= config.extended_ratio && little >= config.extended_ratio {
        Some(false)
    } else {
        None
    }
}

#[derive(Default)]
pub(crate) struct Mapping {
    active: Option<Config>,
    input: Option<Point>,
    output: Option<Point>,
    screen_anchor: Option<(Point, Point)>,
}
fn screen_axis(p: f64, anchor: f64, output: f64) -> f64 {
    if p == anchor {
        output
    } else if p < anchor {
        output * p / anchor
    } else {
        output + (1.0 - output) * (p - anchor) / (1.0 - anchor)
    }
}
impl Mapping {
    pub fn reset(&mut self) {
        *self = Self::default();
    }
    /// Resume screen/detail at the point that stayed visible while scrolling.
    pub fn anchor(&mut self, input: Point, detail: Option<Config>, visible: Point) {
        self.active = detail;
        self.input = Some(input);
        self.output = Some(visible);
        self.screen_anchor = Some((input, visible));
    }
    pub fn observe(
        &mut self,
        point: Point,
        detail: Option<Config>,
        visible: Option<Point>,
    ) -> Result<(Point, bool), &'static str> {
        if !point.valid() || visible.is_some_and(|p| !p.valid()) {
            return Err("detail mapping requires normalized finite positions");
        }
        if let Some(config) = detail {
            config.validate()?;
        }
        let changed = self.active != detail;
        if changed {
            let output = visible.or(self.output).unwrap_or(point);
            self.active = detail;
            self.input = Some(point);
            self.output = Some(output);
            self.screen_anchor = Some((point, output));
            return Ok((output, true));
        }
        let output = if let Some(config) = detail {
            let input = self.input.unwrap_or(point);
            let output = self.output.unwrap_or(point);
            let dx = point.x - input.x;
            let dy = point.y - input.y;
            let distance = dx.hypot(dy);
            // Radial play retains accumulated tiny deliberate displacement. It
            // cannot integrate bounded rest noise into creep or accumulate edge windup.
            let fraction = if distance > config.rest_radius {
                (distance - config.rest_radius) / distance
            } else {
                0.0
            };
            self.input = Some(Point {
                x: input.x + dx * fraction,
                y: input.y + dy * fraction,
            });
            Point {
                x: (output.x + dx * fraction * config.gain).clamp(0.0, 1.0),
                y: (output.y + dy * fraction * config.gain).clamp(0.0, 1.0),
            }
        } else if let Some((anchor, output)) = self.screen_anchor {
            Point {
                x: screen_axis(point.x, anchor.x, output.x),
                y: screen_axis(point.y, anchor.y, output.y),
            }
        } else {
            point
        };
        // Exiting exactly at an input boundary cannot both preserve a different
        // visible point and map that same boundary to the screen edge. Reanchor
        // that axis on the first inward move, so returning reaches the endpoint.
        if detail.is_none() {
            if let Some((anchor, at_anchor)) = &mut self.screen_anchor {
                if (anchor.x == 0.0 || anchor.x == 1.0) && point.x > 0.0 && point.x < 1.0 {
                    anchor.x = point.x;
                    at_anchor.x = output.x;
                }
                if (anchor.y == 0.0 || anchor.y == 1.0) && point.y > 0.0 && point.y < 1.0 {
                    anchor.y = point.y;
                    at_anchor.y = output.y;
                }
            }
        }
        self.input = if detail.is_some() {
            self.input
        } else {
            Some(point)
        };
        self.output = Some(output);
        Ok((output, false))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn p(x: f64, y: f64) -> Point {
        Point { x, y }
    }
    #[test]
    fn transitions_follow_visible_position_and_screen_recovers_every_edge() {
        for input in [0.0, 0.1, 0.5, 0.9, 1.0] {
            for visible in [0.0, 0.2, 0.8, 1.0] {
                let mut map = Mapping::default();
                let cfg = Some(Config::default());
                map.observe(p(0.5, 0.5), None, None).unwrap();
                let (entry, changed) = map
                    .observe(p(input, input), cfg, Some(p(visible, visible)))
                    .unwrap();
                assert!(changed);
                assert_eq!(entry.x, visible);
                let (exit, changed) = map
                    .observe(p(input, input), None, Some(p(visible, visible)))
                    .unwrap();
                assert!(changed);
                assert_eq!(exit.x, visible);
                // One inward step resolves the unavoidable boundary-anchor case.
                map.observe(p(0.5, 0.5), None, None).unwrap();
                let (lo, _) = map.observe(p(0.0, 0.0), None, None).unwrap();
                let (hi, _) = map.observe(p(1.0, 1.0), None, None).unwrap();
                assert_eq!((lo.x, lo.y, hi.x, hi.y), (0.0, 0.0, 1.0, 1.0));
            }
        }
    }
    #[test]
    fn saturation_has_no_windup_and_zero_radius_is_finite() {
        let mut map = Mapping::default();
        let cfg = Some(Config {
            rest_radius: 0.0,
            ..Default::default()
        });
        map.observe(p(0.5, 0.5), cfg, Some(p(0.99, 0.01))).unwrap();
        let (edge, _) = map.observe(p(1.0, 0.0), cfg, None).unwrap();
        assert_eq!((edge.x, edge.y), (1.0, 0.0));
        let (back, _) = map.observe(p(0.99, 0.01), cfg, None).unwrap();
        assert!((back.x - 0.9975).abs() < 1e-12);
        assert!((back.y - 0.0025).abs() < 1e-12);
        let (still, _) = map.observe(p(0.99, 0.01), cfg, None).unwrap();
        assert_eq!((still.x, still.y), (back.x, back.y));
        assert!(map.observe(p(f64::NAN, 0.5), cfg, None).is_err());
        map.reset();
        let (point, _) = map.observe(p(0.2, 0.7), None, None).unwrap();
        assert_eq!((point.x, point.y), (0.2, 0.7));
    }
}
