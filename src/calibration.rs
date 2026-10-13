//! Manual calibration state. No sensor, device or filesystem side effects.
use crate::{touch_gestures::Workspace, Point};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    pub version: u32,
    pub workspace: Workspace,
    /// Qualified single output dimensions; multiple-output routing is separate.
    pub output: [u32; 2],
}
impl Profile {
    pub fn validate(self) -> Result<(), &'static str> {
        if self.version != 1 || self.output.iter().any(|n| !(1..=8192).contains(n)) {
            return Err("invalid calibration version/output");
        }
        self.workspace.validate()
    }
    pub fn target(self, point: Point) -> Result<Point, &'static str> {
        self.validate()?;
        if !point.valid() {
            return Err("invalid calibrated observation");
        }
        Ok(self.workspace.map(point))
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Stage {
    Left,
    Right,
    Top,
    Bottom,
    Review,
    Cancelled,
}
/// Bounded stable observations at each comfortable extreme. Median rejects
/// isolated extreme outliers; the previous adopted profile is owned by the caller.
pub struct Calibration {
    pub stage: Stage,
    samples: VecDeque<(Point, u64)>,
    bounds: [Option<f64>; 4],
    last_source: Option<u64>,
}
impl Default for Calibration {
    fn default() -> Self {
        Self {
            stage: Stage::Left,
            samples: VecDeque::new(),
            bounds: [None; 4],
            last_source: None,
        }
    }
}
impl Calibration {
    pub fn sample(&mut self, point: Point, source_us: u64, now_us: u64) -> bool {
        if matches!(self.stage, Stage::Review | Stage::Cancelled)
            || !point.valid()
            || source_us > now_us
            || now_us - source_us >= 150_000
            || self.last_source.is_some_and(|last| source_us <= last)
        {
            return false;
        }
        self.last_source = Some(source_us);
        if self.samples.len() == 24 {
            self.samples.pop_front();
        }
        self.samples.push_back((point, source_us));
        true
    }
    pub fn tracking_lost(&mut self) {
        self.samples.clear();
    }
    pub fn sample_count(&self) -> usize {
        self.samples.len()
    }
    pub fn advance(&mut self) -> Result<Stage, &'static str> {
        let axis = match self.stage {
            Stage::Left | Stage::Right => 0,
            Stage::Top | Stage::Bottom => 1,
            _ => return Err("calibration is not sampling"),
        };
        if self.samples.len() < 10
            || self.samples.back().unwrap().1 - self.samples.front().unwrap().1 < 750_000
        {
            return Err("hold a stable extreme for at least ten fresh samples and 750 ms");
        }
        let mut values: Vec<_> = self
            .samples
            .iter()
            .map(|(p, _)| if axis == 0 { p.x } else { p.y })
            .collect();
        values.sort_by(f64::total_cmp);
        let n = values.len();
        // Require a stable central population; isolated end outliers do not veto it.
        if values[n * 3 / 4] - values[n / 4] > 0.04 {
            return Err("extreme position is not stable");
        }
        let median = (values[(n - 1) / 2] + values[n / 2]) / 2.0;
        let index = match self.stage {
            Stage::Left => 0,
            Stage::Right => 1,
            Stage::Top => 2,
            Stage::Bottom => 3,
            _ => unreachable!(),
        };
        // Do not leave a successful final sampling stage with invalid bounds.
        if index == 1 && median - self.bounds[0].unwrap() < 0.05
            || index == 3 && median - self.bounds[2].unwrap() < 0.05
        {
            return Err("comfortable bounds are too narrow or reversed");
        }
        self.bounds[index] = Some(median);
        self.samples.clear();
        self.stage = match self.stage {
            Stage::Left => Stage::Right,
            Stage::Right => Stage::Top,
            Stage::Top => Stage::Bottom,
            Stage::Bottom => Stage::Review,
            _ => unreachable!(),
        };
        Ok(self.stage)
    }
    pub fn candidate(&self, output: [u32; 2]) -> Result<Profile, &'static str> {
        if self.stage != Stage::Review {
            return Err("calibration requires corner review before adoption");
        }
        let profile = Profile {
            version: 1,
            workspace: Workspace {
                min: Point {
                    x: self.bounds[0].unwrap(),
                    y: self.bounds[2].unwrap(),
                },
                max: Point {
                    x: self.bounds[1].unwrap(),
                    y: self.bounds[3].unwrap(),
                },
            },
            output,
        };
        profile.validate()?;
        Ok(profile)
    }
    pub fn cancel(&mut self) {
        self.stage = Stage::Cancelled;
        self.samples.clear();
        self.bounds = [None; 4];
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn feed(cal: &mut Calibration, point: Point, start: u64) {
        for n in 0..12 {
            assert!(cal.sample(point, start + n * 100_000, start + n * 100_000 + 50_000));
        }
    }
    #[test]
    fn shared_target_reaches_calibrated_edges_and_rejects_invalid_profile() {
        let mut c = Calibration::default();
        for (i, p) in [
            Point { x: 0.2, y: 0.5 },
            Point { x: 0.8, y: 0.5 },
            Point { x: 0.5, y: 0.3 },
            Point { x: 0.5, y: 0.7 },
        ]
        .into_iter()
        .enumerate()
        {
            feed(&mut c, p, i as u64 * 2_000_000);
            c.advance().unwrap();
        }
        let p = c.candidate([1920, 1080]).unwrap();
        assert_eq!(p.target(Point { x: 0.2, y: 0.3 }).unwrap().x, 0.0);
        assert_eq!(p.target(Point { x: 0.8, y: 0.7 }).unwrap().y, 1.0);
        assert_eq!(p.target(Point { x: 0.0, y: 1.0 }).unwrap().x, 0.0);
        assert!(Profile { version: 2, ..p }.validate().is_err());
        assert!(Profile {
            output: [0, 1080],
            ..p
        }
        .validate()
        .is_err());
    }
    #[test]
    fn cancelled_unstable_stale_and_short_samples_cannot_adopt() {
        let mut c = Calibration::default();
        let p = Point { x: 0.2, y: 0.5 };
        assert!(!c.sample(p, 1, 150_001));
        assert!(!c.sample(p, 2, 1));
        assert!(c.candidate([1920, 1080]).is_err());
        feed(&mut c, p, 1_000_000);
        assert!(!c.sample(p, 1_000_000, 1_000_000));
        c.tracking_lost();
        assert!(c.advance().is_err());
        for n in 0..12 {
            c.sample(
                Point {
                    x: if n % 2 == 0 { 0.1 } else { 0.5 },
                    y: 0.5,
                },
                3_000_000 + n * 100_000,
                3_000_000 + n * 100_000,
            );
        }
        assert!(c.advance().is_err());
        c.cancel();
        assert!(!c.sample(p, 5_000_000, 5_000_000));
        assert!(c.candidate([1920, 1080]).is_err());
    }
    #[test]
    fn median_ignores_one_outlier_and_sample_storage_is_bounded() {
        let mut c = Calibration::default();
        for n in 0..30 {
            c.sample(
                Point {
                    x: if n == 29 { 0.9 } else { 0.2 },
                    y: 0.5,
                },
                n * 100_000,
                n * 100_000,
            );
        }
        assert_eq!(c.sample_count(), 24);
        assert_eq!(c.advance().unwrap(), Stage::Right);
        feed(&mut c, Point { x: 0.21, y: 0.5 }, 4_000_000);
        assert!(c.advance().is_err());
        assert_eq!(c.stage, Stage::Right);
    }
}
