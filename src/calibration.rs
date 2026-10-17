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
    UpperLeft,
    LowerRight,
    Review,
    Cancelled,
}
/// Bounded stable observations at each comfortable extreme. Median rejects
/// isolated extreme outliers; the previous adopted profile is owned by the caller.
#[derive(Clone)]
pub struct Calibration {
    pub stage: Stage,
    samples: VecDeque<(Point, u64)>,
    bounds: [Option<f64>; 4],
    last_source: Option<u64>,
    diagonal: bool,
}
impl Default for Calibration {
    fn default() -> Self {
        Self {
            diagonal: false,
            stage: Stage::Left,
            samples: VecDeque::new(),
            bounds: [None; 4],
            last_source: None,
        }
    }
}
impl Calibration {
    pub fn diagonal() -> Self {
        Self {
            stage: Stage::UpperLeft,
            diagonal: true,
            ..Self::default()
        }
    }
    pub fn is_diagonal(&self) -> bool {
        self.diagonal
    }
    pub fn from_diagonal_bounds(bounds: [Option<f64>; 4]) -> Result<Self, &'static str> {
        let valid_value =
            |v: Option<f64>| v.is_some_and(|n| n.is_finite() && (0.0..=1.0).contains(&n));
        let stage = match bounds {
            [None, None, None, None] => Stage::UpperLeft,
            [Some(_), None, Some(_), None] if valid_value(bounds[0]) && valid_value(bounds[2]) => {
                Stage::LowerRight
            }
            [Some(_), Some(_), Some(_), Some(_)] => {
                Self::from_bounds(bounds)?;
                Stage::Review
            }
            _ => return Err("invalid diagonal calibration bounds"),
        };
        Ok(Self {
            stage,
            bounds,
            ..Self::diagonal()
        })
    }
    /// Persist only explicitly confirmed bounds, never pending observations.
    pub fn confirmed_bounds(&self) -> [Option<f64>; 4] {
        self.bounds
    }
    pub fn from_bounds(bounds: [Option<f64>; 4]) -> Result<Self, &'static str> {
        let mut count = 0;
        let mut gap = false;
        for bound in bounds {
            match bound {
                Some(value) if !gap && value.is_finite() && (0.0..=1.0).contains(&value) => {
                    count += 1
                }
                None => gap = true,
                _ => return Err("invalid confirmed calibration bounds"),
            }
        }
        if count >= 2 && bounds[1].unwrap() - bounds[0].unwrap() < 0.05
            || count == 4 && bounds[3].unwrap() - bounds[2].unwrap() < 0.05
        {
            return Err("confirmed calibration bounds are narrow or reversed");
        }
        Ok(Self {
            stage: [
                Stage::Left,
                Stage::Right,
                Stage::Top,
                Stage::Bottom,
                Stage::Review,
            ][count],
            bounds,
            ..Self::default()
        })
    }

    pub fn sample(&mut self, point: Point, source_us: u64, now_us: u64) -> bool {
        if matches!(self.stage, Stage::Review | Stage::Cancelled)
            || !point.valid()
            || source_us > now_us
            || now_us - source_us >= 150_000
            || self.last_source.is_some_and(|last| source_us <= last)
        {
            return false;
        }
        // Reject old observations without discarding an otherwise stable window.
        // A real gap still requires a new window; input freshness is unchanged.
        if self
            .last_source
            .is_some_and(|last| source_us - last >= 500_000)
        {
            self.samples.clear();
        }
        self.last_source = Some(source_us);
        if self.samples.len() == 24 {
            self.samples.pop_front();
        }
        self.samples.push_back((point, source_us));
        true
    }
    pub fn expire_samples(&mut self, now_us: u64) {
        if self
            .last_source
            .is_some_and(|last| now_us.saturating_sub(last) >= 500_000)
        {
            self.samples.clear();
        }
    }
    pub fn tracking_lost(&mut self) {
        self.samples.clear();
    }
    pub fn sample_count(&self) -> usize {
        self.samples.len()
    }
    fn stable_median(&self, axis: usize) -> Result<f64, &'static str> {
        if self.samples.len() < 10
            || self.samples.back().unwrap().1 - self.samples.front().unwrap().1 < 750_000
        {
            return Err("hold the position for ten fresh samples and 750 ms");
        }
        let mut values: Vec<_> = self
            .samples
            .iter()
            .map(|(p, _)| if axis == 0 { p.x } else { p.y })
            .collect();
        values.sort_by(f64::total_cmp);
        let n = values.len();
        if values[n * 3 / 4] - values[n / 4] > 0.04 {
            return Err("position is not stable");
        }
        Ok((values[(n - 1) / 2] + values[n / 2]) / 2.0)
    }
    pub fn advance(&mut self) -> Result<Stage, &'static str> {
        if matches!(self.stage, Stage::UpperLeft | Stage::LowerRight) {
            let x = self.stable_median(0)?;
            let y = self.stable_median(1)?;
            if self.stage == Stage::UpperLeft {
                self.bounds = [Some(x), None, Some(y), None];
                self.stage = Stage::LowerRight;
            } else {
                // Leave at least 0.05 usable width/height after a 5% margin per side.
                if (x - self.bounds[0].unwrap()) * 0.9 < 0.05
                    || (y - self.bounds[2].unwrap()) * 0.9 < 0.05
                {
                    return Err("diagonal positions are too close or reversed");
                }
                self.bounds[1] = Some(x);
                self.bounds[3] = Some(y);
                self.stage = Stage::Review;
            }
            self.samples.clear();
            return Ok(self.stage);
        }
        let axis = match self.stage {
            Stage::Left | Stage::Right => 0,
            Stage::Top | Stage::Bottom => 1,
            _ => return Err("calibration is not sampling"),
        };
        let median = self.stable_median(axis)?;
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
        let mut profile = Profile {
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
        if self.diagonal {
            // Reach the screen edges before the exact sampled endpoints.
            let dx = (profile.workspace.max.x - profile.workspace.min.x) * 0.05;
            let dy = (profile.workspace.max.y - profile.workspace.min.y) * 0.05;
            profile.workspace.min.x += dx;
            profile.workspace.max.x -= dx;
            profile.workspace.min.y += dy;
            profile.workspace.max.y -= dy;
        }
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
    fn diagonal_positions_require_both_stable_axes_and_map_interior_edges() {
        let mut c = Calibration::diagonal();
        feed(&mut c, Point { x: 0.2, y: 0.3 }, 1_000_000);
        assert_eq!(c.advance().unwrap(), Stage::LowerRight);
        let restored = Calibration::from_diagonal_bounds(c.confirmed_bounds()).unwrap();
        assert_eq!(restored.stage, Stage::LowerRight);
        assert_eq!(restored.sample_count(), 0);
        for n in 0..12 {
            c.sample(
                Point {
                    x: 0.8,
                    y: if n % 2 == 0 { 0.5 } else { 0.9 },
                },
                3_000_000 + n * 100_000,
                3_000_000 + n * 100_000,
            );
        }
        assert!(c.advance().is_err());
        c.tracking_lost();
        feed(&mut c, Point { x: 0.8, y: 0.7 }, 5_000_000);
        assert_eq!(c.advance().unwrap(), Stage::Review);
        let profile = c.candidate([1920, 1080]).unwrap();
        assert!((profile.workspace.min.x - 0.23).abs() < 1e-9);
        assert!((profile.workspace.max.y - 0.68).abs() < 1e-9);
        assert_eq!(profile.target(Point { x: 0.2, y: 0.3 }).unwrap().x, 0.0);
        assert_eq!(profile.target(Point { x: 0.8, y: 0.7 }).unwrap().y, 1.0);
        let restored = Calibration::from_diagonal_bounds(c.confirmed_bounds()).unwrap();
        assert_eq!(
            restored.candidate([1920, 1080]).unwrap().workspace.min.x,
            profile.workspace.min.x
        );
        assert!(Calibration::from_diagonal_bounds([Some(0.2), None, None, None]).is_err());
        let mut reversed = Calibration::diagonal();
        feed(&mut reversed, Point { x: 0.6, y: 0.5 }, 1_000_000);
        reversed.advance().unwrap();
        feed(&mut reversed, Point { x: 0.4, y: 0.8 }, 3_000_000);
        assert!(reversed.advance().is_err());
        assert_eq!(reversed.stage, Stage::LowerRight);
    }
    #[test]
    fn restored_bounds_are_ordered_and_long_sampling_gaps_reset_pending_samples() {
        assert!(Calibration::from_bounds([None, Some(0.8), None, None]).is_err());
        assert!(Calibration::from_bounds([Some(f64::NAN), None, None, None]).is_err());
        assert!(Calibration::from_bounds([Some(0.8), Some(0.2), None, None]).is_err());
        let c = Calibration::from_bounds([Some(0.2), Some(0.8), Some(0.3), Some(0.7)]).unwrap();
        assert_eq!(c.stage, Stage::Review);
        assert!(c.candidate([1920, 1080]).is_ok());
        let mut c = Calibration::default();
        let p = Point { x: 0.2, y: 0.5 };
        feed(&mut c, p, 1_000_000);
        c.sample(p, 3_000_000, 3_000_000);
        assert_eq!(c.sample_count(), 1);
        assert!(c.advance().is_err());
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

/// Candidate corner verification is independent of pinch. Sampling/hover review
/// never authorizes a button press or adoption by itself.
pub struct Review {
    profile: Profile,
    next: usize,
    held_since: Option<u64>,
    last_source: Option<u64>,
}
impl Review {
    pub fn new(profile: Profile) -> Result<Self, &'static str> {
        profile.validate()?;
        Ok(Self {
            profile,
            next: 0,
            held_since: None,
            last_source: None,
        })
    }
    pub fn target(&self) -> Option<Point> {
        [
            Point { x: 0.0, y: 0.0 },
            Point { x: 1.0, y: 0.0 },
            Point { x: 1.0, y: 1.0 },
            Point { x: 0.0, y: 1.0 },
        ]
        .get(self.next)
        .copied()
    }
    /// Fresh in-target evidence has started the current corner's dwell.
    /// This is not verification; loss, stale input and movement reset it.
    pub fn holding_target(&self) -> bool {
        self.held_since.is_some()
    }
    pub fn observe(
        &mut self,
        point: Point,
        source_us: u64,
        now_us: u64,
    ) -> Result<Point, &'static str> {
        if source_us > now_us
            || now_us - source_us >= 150_000
            || self.last_source.is_some_and(|s| source_us <= s)
        {
            self.held_since = None;
            return Err("corner observation is stale or out of order");
        }
        if self.last_source.is_some_and(|s| source_us - s >= 150_000) {
            self.held_since = None;
        }
        self.last_source = Some(source_us);
        let mapped = match self.profile.target(point) {
            Ok(mapped) => mapped,
            Err(error) => {
                self.held_since = None;
                return Err(error);
            }
        };
        if let Some(target) = self.target() {
            if (mapped.x - target.x).abs() <= 0.05 && (mapped.y - target.y).abs() <= 0.05 {
                let since = *self.held_since.get_or_insert(source_us);
                if source_us - since >= 500_000 {
                    self.next += 1;
                    self.held_since = None;
                }
            } else {
                self.held_since = None;
            }
        }
        Ok(mapped)
    }
    pub fn tracking_lost(&mut self) {
        self.held_since = None;
    }
    pub fn verified(&self) -> bool {
        self.next == 4
    }
    pub fn accepted(&self) -> Result<Profile, &'static str> {
        if !self.verified() {
            return Err("verify all four corners before adoption");
        }
        Ok(self.profile)
    }
}

#[cfg(test)]
mod review_tests {
    use super::*;
    fn profile() -> Profile {
        Profile {
            version: 1,
            workspace: Workspace::default(),
            output: [1920, 1080],
        }
    }
    #[test]
    fn all_four_corners_require_fresh_continuous_dwell_before_acceptance() {
        let mut review = Review::new(profile()).unwrap();
        assert!(review.accepted().is_err());
        let mut at = 1_000_000;
        for _ in 0..4 {
            let target = review.target().unwrap();
            for _ in 0..6 {
                review.observe(target, at, at + 50_000).unwrap();
                at += 100_000;
            }
        }
        assert!(review.verified());
        assert!(review.accepted().is_ok());
        assert!(review.target().is_none());
    }
    #[test]
    fn stale_loss_invalid_and_jump_cannot_complete_a_corner() {
        let mut review = Review::new(profile()).unwrap();
        let p = review.target().unwrap();
        for n in 0..3 {
            review
                .observe(p, 1_000_000 + n * 100_000, 1_000_000 + n * 100_000)
                .unwrap();
        }
        review.tracking_lost();
        review.observe(p, 1_300_000, 1_300_000).unwrap();
        assert!(review.observe(p, 1_400_000, 1_550_000).is_err());
        assert!(review
            .observe(
                Point {
                    x: f64::NAN,
                    y: 0.0
                },
                1_500_000,
                1_500_000
            )
            .is_err());
        review.observe(p, 3_000_000, 3_000_000).unwrap();
        assert_eq!(review.target().unwrap().x, 0.0);
        assert!(!review.verified());
        assert!(review.observe(p, 2_000_000, 3_000_000).is_err());
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Control {
    Start,
    StartDiagonal,
    Next,
    Cancel,
    Accept,
    Rollback,
}
