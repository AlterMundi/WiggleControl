//! Held fist scroll meaning over existing raw landmarks. No inference or OS I/O.
use crate::{detail::finger_ratio, Point};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Inertia {
    pub decay_us: u64,
    pub max_duration_us: u64,
    pub release_window_us: u64,
    pub min_speed: f64,
    pub max_distance: f64,
}
impl Default for Inertia {
    fn default() -> Self {
        Self {
            decay_us: 180_000,
            max_duration_us: 700_000,
            release_window_us: 150_000,
            min_speed: 0.15,
            max_distance: 0.18,
        }
    }
}
impl Inertia {
    fn validate(self) -> Result<(), &'static str> {
        if !(50_000..=400_000).contains(&self.decay_us)
            || !(100_000..=1_000_000).contains(&self.max_duration_us)
            || !(50_000..=250_000).contains(&self.release_window_us)
            || !self.min_speed.is_finite()
            || !(0.05..=1.0).contains(&self.min_speed)
            || !self.max_distance.is_finite()
            || !(0.02..=0.5).contains(&self.max_distance)
        {
            return Err("invalid scroll inertia limits");
        }
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub inertia: Option<Inertia>,
    pub acceleration_max: f64,
    pub acceleration_speed: f64,
    pub velocity_smoothing_us: f64,
    pub hold_us: u64,
    pub neutral_us: u64,
    pub folded_ratio: f64,
    pub extended_ratio: f64,
    /// Thumb tip must stay within the palm, along little-to-index MCP axis.
    pub thumb_clearance_max: f64,
    pub steps_per_unit: f64,
    pub rest_radius: f64,
    pub max_step: f64,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            inertia: None,
            acceleration_max: 1.0,
            acceleration_speed: 0.8,
            velocity_smoothing_us: 80_000.0,
            hold_us: 300_000,
            neutral_us: 200_000,
            folded_ratio: 0.55,
            extended_ratio: 0.8,
            thumb_clearance_max: 0.1,
            steps_per_unit: 30.0,
            rest_radius: 0.004,
            max_step: 0.2,
        }
    }
}
impl Config {
    pub fn validate(self) -> Result<(), &'static str> {
        if let Some(inertia) = self.inertia {
            inertia.validate()?;
        }
        if !self.acceleration_max.is_finite()
            || !(1.0..=4.0).contains(&self.acceleration_max)
            || !self.acceleration_speed.is_finite()
            || !(0.1..=3.0).contains(&self.acceleration_speed)
            || !self.velocity_smoothing_us.is_finite()
            || !(0.0..=200_000.0).contains(&self.velocity_smoothing_us)
            || !(100_000..=1_000_000).contains(&self.hold_us)
            || !(100_000..=1_000_000).contains(&self.neutral_us)
            || !self.folded_ratio.is_finite()
            || !self.extended_ratio.is_finite()
            || !(0.1..=0.8).contains(&self.folded_ratio)
            || !(self.folded_ratio + 0.1..=1.0).contains(&self.extended_ratio)
            || !self.thumb_clearance_max.is_finite()
            || !(-0.5..=0.2).contains(&self.thumb_clearance_max)
            || !self.steps_per_unit.is_finite()
            || !(1.0..=100.0).contains(&self.steps_per_unit)
            || !self.rest_radius.is_finite()
            || !(0.0..=0.02).contains(&self.rest_radius)
            || !self.max_step.is_finite()
            || !(0.03..=0.5).contains(&self.max_step)
        {
            return Err("invalid held-scroll geometry, timing or wheel mapping");
        }
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Pose {
    Neutral,
    Fist,
    Other,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Evidence {
    pub hand_id: u64,
    pub source_us: u64,
    pub pose: Pose,
    /// Normalized image-space wrist, independent of fingertip articulation.
    pub wrist: Point,
}
#[derive(Clone, Debug, Default, Serialize)]
pub struct Feedback {
    pub armed: bool,
    pub active: bool,
    pub candidate: bool,
    pub inertia: bool,
    pub releasing: bool,
    pub hold_elapsed_us: u64,
    pub hold_required_us: u64,
    pub neutral_elapsed_us: u64,
    pub neutral_required_us: u64,
}
impl Feedback {
    pub fn captures_pointer(&self) -> bool {
        self.active || self.candidate || self.inertia || self.releasing
    }
}
pub fn classify(config: Config, world: &[[f64; 3]; 21]) -> Option<Pose> {
    if world.iter().flatten().any(|v| !v.is_finite()) {
        return None;
    }
    let mut ratios = [0.0; 4];
    for (i, start) in [5, 9, 13, 17].into_iter().enumerate() {
        ratios[i] = finger_ratio(world[start..start + 4].try_into().ok()?)?;
    }
    let across = std::array::from_fn::<_, 3, _>(|i| world[5][i] - world[17][i]);
    let width_squared = across.iter().map(|v| v * v).sum::<f64>();
    if !width_squared.is_finite() || width_squared < 1e-12 {
        return None;
    }
    let clearance = (0..3)
        .map(|i| (world[4][i] - world[5][i]) * across[i])
        .sum::<f64>()
        / width_squared;
    if !clearance.is_finite() {
        return None;
    }
    if ratios.iter().all(|r| *r >= config.extended_ratio) {
        Some(Pose::Neutral)
    } else if ratios.iter().all(|r| *r <= config.folded_ratio)
        && (-1.2..=config.thumb_clearance_max).contains(&clearance)
    {
        Some(Pose::Fist)
    } else {
        Some(Pose::Other)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Motion {
    pub vertical: i32,
    pub horizontal: i32,
}
struct Coast {
    since: u64,
    last: u64,
    velocity: [f64; 2],
    distance: f64,
    finished: bool,
}
/// Controller validates source age, owner, confidence and in-view raw evidence.
/// This layer independently prevents temporal gaps and missing evidence from
/// becoming neutral or initiating inertia. Coasting needs fresh positive open evidence.
#[derive(Default)]
pub(crate) struct Held {
    armed: bool,
    active: bool,
    owner: Option<u64>,
    last_source: Option<u64>,
    neutral_since: Option<u64>,
    fist_since: Option<u64>,
    source: u64,
    wrist: Option<Point>,
    remainder: [f64; 2],
    speed: f64,
    velocity: [f64; 2],
    motion_source: Option<u64>,
    release_pending: Option<u64>,
    coast: Option<Coast>,
}
impl Held {
    pub fn feedback(&self, config: Config) -> Feedback {
        Feedback {
            armed: self.armed,
            active: self.active,
            inertia: self.coast.is_some(),
            releasing: self.release_pending.is_some(),
            candidate: self.fist_since.is_some() && !self.active,
            hold_elapsed_us: self.fist_since.map_or(0, |s| self.source.saturating_sub(s)),
            hold_required_us: config.hold_us,
            neutral_elapsed_us: self
                .neutral_since
                .map_or(0, |s| self.source.saturating_sub(s)),
            neutral_required_us: config.neutral_us,
        }
    }
    pub fn reset(&mut self) {
        *self = Self::default();
    }
    fn wheel(&mut self, config: Config, motion: [f64; 2]) -> Motion {
        let signs = [-1.0, 1.0];
        let steps = std::array::from_fn::<_, 2, _>(|axis| {
            self.remainder[axis] += signs[axis] * motion[axis] * config.steps_per_unit * 120.0;
            let whole = self.remainder[axis].trunc();
            self.remainder[axis] -= whole;
            (whole as i32).clamp(-1200, 1200)
        });
        Motion {
            horizontal: steps[0],
            vertical: steps[1],
        }
    }
    fn can_release(&self, config: Inertia, source: u64) -> bool {
        self.motion_source
            .is_some_and(|t| source.saturating_sub(t) <= config.release_window_us)
            && self.velocity[0].hypot(self.velocity[1]) >= config.min_speed
    }
    fn coast(&mut self, config: Config, source: u64) -> Motion {
        let inertia = config.inertia.expect("coast requires inertia config");
        let coast = self.coast.as_mut().expect("coast state");
        if coast.finished {
            self.coast = None;
            self.remainder = [0.0; 2];
            return Motion::default();
        }
        let until = source.min(coast.since.saturating_add(inertia.max_duration_us));
        let integral = inertia.decay_us as f64 / 1_000_000.0
            * ((-(coast.last.saturating_sub(coast.since) as f64) / inertia.decay_us as f64).exp()
                - (-(until.saturating_sub(coast.since) as f64) / inertia.decay_us as f64).exp());
        let mut motion = coast.velocity.map(|v| v * integral);
        let length = motion[0].hypot(motion[1]);
        let remaining = (inertia.max_distance - coast.distance).max(0.0);
        if length > remaining && length > 0.0 {
            motion = motion.map(|v| v * remaining / length);
        }
        coast.distance += motion[0].hypot(motion[1]);
        coast.last = until;
        let speed = coast.velocity[0].hypot(coast.velocity[1])
            * (-(until.saturating_sub(coast.since) as f64) / inertia.decay_us as f64).exp();
        coast.finished = source.saturating_sub(coast.since) >= inertia.max_duration_us
            || coast.distance >= inertia.max_distance
            || speed < inertia.min_speed * 0.2;
        self.wheel(config, motion)
    }
    pub fn observe(
        &mut self,
        config: Config,
        source: u64,
        evidence: Option<Evidence>,
        stale_us: u64,
    ) -> Motion {
        let Some(evidence) = evidence.filter(|e| e.source_us == source && e.wrist.valid()) else {
            self.reset();
            return Motion::default();
        };
        if self.owner != Some(evidence.hand_id)
            || self
                .last_source
                .is_some_and(|t| source <= t || source.saturating_sub(t) >= stale_us)
        {
            self.reset();
        }
        let dt = source.saturating_sub(self.last_source.unwrap_or(source));
        self.owner = Some(evidence.hand_id);
        self.last_source = Some(source);
        self.source = source;
        match evidence.pose {
            Pose::Neutral => {
                if self.coast.is_some() {
                    return self.coast(config, source);
                }
                if (self.active || self.release_pending.is_some())
                    && config.inertia.is_some_and(|i| self.can_release(i, source))
                {
                    self.coast = Some(Coast {
                        since: source,
                        last: source,
                        velocity: self.velocity,
                        distance: 0.0,
                        finished: false,
                    });
                }
                if self.active || self.fist_since.is_some() || self.release_pending.is_some() {
                    self.armed = false;
                }
                self.active = false;
                self.fist_since = None;
                self.release_pending = None;
                self.wrist = None;
                self.velocity = [0.0; 2];
                self.speed = 0.0;
                if self.coast.is_none() {
                    self.remainder = [0.0; 2];
                }
                let since = *self.neutral_since.get_or_insert(source);
                if source.saturating_sub(since) >= config.neutral_us {
                    self.armed = true;
                }
            }
            Pose::Other => {
                let could_release =
                    self.active && config.inertia.is_some_and(|i| self.can_release(i, source));
                if could_release {
                    self.release_pending = self.motion_source;
                }
                if self.release_pending.is_some_and(|t| {
                    config
                        .inertia
                        .is_none_or(|i| source.saturating_sub(t) > i.release_window_us)
                }) {
                    self.release_pending = None;
                }
                if self.active || self.fist_since.is_some() || self.coast.is_some() {
                    self.armed = false;
                }
                self.active = false;
                self.neutral_since = None;
                self.fist_since = None;
                self.wrist = None;
                self.coast = None;
                if self.release_pending.is_none() {
                    self.remainder = [0.0; 2];
                    self.speed = 0.0;
                    self.velocity = [0.0; 2];
                }
            }
            Pose::Fist => {
                self.neutral_since = None;
                if self.coast.take().is_some() {
                    // Regrabbing a moving page arrests inertia at this sample.
                    self.active = true;
                    self.armed = true;
                    self.fist_since = Some(source);
                    self.wrist = Some(evidence.wrist);
                    self.velocity = [0.0; 2];
                    self.speed = 0.0;
                    self.remainder = [0.0; 2];
                    return Motion::default();
                }
                self.release_pending = None;
                if !self.armed {
                    return Motion::default();
                }
                let since = *self.fist_since.get_or_insert(source);
                if !self.active {
                    // Articulation/candidate motion is never a first wheel step.
                    self.wrist = Some(evidence.wrist);
                    self.remainder = [0.0; 2];
                    self.speed = 0.0;
                    self.velocity = [0.0; 2];
                    self.active = source.saturating_sub(since) >= config.hold_us;
                    return Motion::default();
                }
                let previous = self.wrist.unwrap_or(evidence.wrist);
                let delta = [evidence.wrist.x - previous.x, evidence.wrist.y - previous.y];
                if delta[0].hypot(delta[1]) > config.max_step {
                    // A discontinuity reanchors both axes, with no wheel backlog.
                    self.wrist = Some(evidence.wrist);
                    self.remainder = [0.0; 2];
                    self.speed = 0.0;
                    self.velocity = [0.0; 2];
                    return Motion::default();
                }
                let motion = delta.map(|d| d.signum() * (d.abs() - config.rest_radius).max(0.0));
                self.wrist = Some(Point {
                    x: previous.x + motion[0],
                    y: previous.y + motion[1],
                });
                let speed = motion[0].hypot(motion[1]) * 1_000_000.0 / dt.max(1) as f64;
                let alpha = if config.velocity_smoothing_us == 0.0 {
                    1.0
                } else {
                    1.0 - (-(dt as f64) / config.velocity_smoothing_us).exp()
                };
                self.speed += alpha * (speed - self.speed);
                let x = (self.speed / config.acceleration_speed).clamp(0.0, 1.0);
                let gain = 1.0 + (config.acceleration_max - 1.0) * x * x * (3.0 - 2.0 * x);
                let amplified = motion.map(|d| d * gain);
                let instantaneous =
                    amplified[0].hypot(amplified[1]) * 1_000_000.0 / dt.max(1) as f64;
                let rest_speed = config.inertia.map_or(0.02, |i| i.min_speed * 0.25);
                if instantaneous < rest_speed {
                    // Stop velocity at rest, including floating-point boundary
                    // noise: EMA history must not turn a quiet opening into a flick.
                    self.velocity = [0.0; 2];
                    self.motion_source = None;
                } else {
                    if self.velocity[0] * amplified[0] + self.velocity[1] * amplified[1] < 0.0 {
                        self.velocity = [0.0; 2];
                    }
                    for (velocity, movement) in self.velocity.iter_mut().zip(amplified) {
                        *velocity +=
                            alpha * (movement * 1_000_000.0 / dt.max(1) as f64 - *velocity);
                    }
                    self.motion_source = Some(source);
                }
                return self.wheel(config, amplified);
            }
        }
        Motion::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sample(h: &mut Held, source: u64, pose: Pose, x: f64, y: f64) -> Motion {
        h.observe(
            Config::default(),
            source,
            Some(Evidence {
                hand_id: 7,
                source_us: source,
                pose,
                wrist: Point { x, y },
            }),
            300_000,
        )
    }
    fn arm(h: &mut Held) {
        for t in [1, 100_001, 200_001] {
            sample(h, t, Pose::Neutral, 0.5, 0.5);
        }
    }
    #[test]
    fn direct_content_diagonal_rest_and_rearm_have_no_wheel_backlog() {
        let c = Config::default();
        let mut h = Held::default();
        arm(&mut h);
        for t in [300_001, 400_001, 500_001, 600_001] {
            assert_eq!(sample(&mut h, t, Pose::Fist, 0.5, 0.5), Motion::default());
        }
        assert!(h.feedback(c).active);
        let up_left = sample(&mut h, 700_001, Pose::Fist, 0.4, 0.4);
        assert!(up_left.vertical < 0 && up_left.horizontal > 0);
        let down_right = sample(&mut h, 800_001, Pose::Fist, 0.5, 0.5);
        assert!(down_right.vertical > 0 && down_right.horizontal < 0);
        let mut settling = 0;
        for t in (900_001..2_900_001).step_by(100_000) {
            let noise = if t / 100_000 % 2 == 0 { 0.001 } else { -0.001 };
            let m = sample(&mut h, t, Pose::Fist, 0.5 + noise, 0.5 - noise);
            settling += m.horizontal.abs() + m.vertical.abs();
            if t > 1_100_001 {
                assert_eq!(m, Motion::default());
            }
        }
        // One bounded settling displacement after a real reversal is allowed;
        // cycling rest noise cannot accumulate into continuing scroll.
        assert!(settling <= 8);
        // A jump cannot become ten steps now and a hidden queued burst later.
        assert_eq!(
            sample(&mut h, 2_900_001, Pose::Fist, 0.9, 0.9),
            Motion::default()
        );
        assert_eq!(
            sample(&mut h, 3_000_001, Pose::Fist, 0.9, 0.9),
            Motion::default()
        );
        sample(&mut h, 3_100_001, Pose::Neutral, 0.5, 0.5);
        assert!(!h.feedback(c).active && !h.feedback(c).armed);
        sample(&mut h, 3_200_001, Pose::Fist, 0.5, 0.5);
        assert!(!h.feedback(c).captures_pointer());
    }
    #[test]
    fn loss_owner_gap_and_duplicate_never_reacquire_a_held_fist() {
        let c = Config::default();
        for failure in 0..4 {
            let mut h = Held::default();
            arm(&mut h);
            for t in [300_001, 400_001, 500_001, 600_001] {
                sample(&mut h, t, Pose::Fist, 0.5, 0.5);
            }
            let t = if failure == 2 {
                1_000_001
            } else if failure == 3 {
                600_001
            } else {
                700_001
            };
            let e = (failure != 0).then_some(Evidence {
                hand_id: if failure == 1 { 8 } else { 7 },
                source_us: t,
                pose: Pose::Fist,
                wrist: Point { x: 0.4, y: 0.4 },
            });
            assert_eq!(h.observe(c, t, e, 300_000), Motion::default());
            assert!(!h.feedback(c).captures_pointer());
            assert_eq!(
                sample(&mut h, t + 100_000, Pose::Fist, 0.3, 0.3),
                Motion::default()
            );
            assert!(!h.feedback(c).captures_pointer());
        }
    }
    #[test]
    fn geometry_separates_thumb_out_detail_and_folded_fist_under_transforms() {
        let c = Config::default();
        for pose in [
            crate::escape::Pose::Neutral,
            crate::escape::Pose::Other,
            crate::escape::Pose::ThumbOut,
        ] {
            let raw = crate::escape::tests::world(pose);
            let expected = if pose == crate::escape::Pose::Neutral {
                Pose::Neutral
            } else {
                Pose::Other
            };
            assert_eq!(classify(c, &raw), Some(expected));
        }
        let mut fist = crate::escape::tests::world(crate::escape::Pose::ThumbOut);
        fist[4] = [0.0, -0.02, 0.0];
        for raw in [
            fist,
            fist.map(|p| [p[2] * 2.0 + 0.1, -p[0] * 2.0, p[1] * 2.0 - 0.2]),
        ] {
            assert_eq!(classify(c, &raw), Some(Pose::Fist));
            assert_eq!(
                crate::escape::classify(Default::default(), &raw),
                Some(crate::escape::Pose::Other)
            );
        }
        fist[12][1] = f64::NAN;
        assert_eq!(classify(c, &fist), None);
        assert_eq!(classify(c, &[[0.0; 3]; 21]), None);
        assert!(Config {
            rest_radius: f64::NAN,
            ..c
        }
        .validate()
        .is_err());
        assert!(Config {
            steps_per_unit: f64::INFINITY,
            ..c
        }
        .validate()
        .is_err());
    }
    fn enabled_motion(config: Config, stationary: bool) -> (Held, u64) {
        let mut h = Held::default();
        arm(&mut h);
        for t in [300_001, 400_001, 500_001, 600_001] {
            h.observe(
                config,
                t,
                Some(Evidence {
                    hand_id: 7,
                    source_us: t,
                    pose: Pose::Fist,
                    wrist: Point { x: 0.5, y: 0.5 },
                }),
                300_000,
            );
        }
        let mut t = 600_001;
        for n in 1..=3 {
            t += 50_000;
            h.observe(
                config,
                t,
                Some(Evidence {
                    hand_id: 7,
                    source_us: t,
                    pose: Pose::Fist,
                    wrist: Point {
                        x: 0.5 - n as f64 * 0.03,
                        y: 0.5 - n as f64 * 0.03,
                    },
                }),
                300_000,
            );
        }
        if stationary {
            t += 50_000;
            h.observe(
                config,
                t,
                Some(Evidence {
                    hand_id: 7,
                    source_us: t,
                    pose: Pose::Fist,
                    wrist: Point { x: 0.41, y: 0.41 },
                }),
                300_000,
            );
        }
        (h, t)
    }
    #[test]
    fn inertia_requires_positive_moving_release_and_resting_release_has_none() {
        let config = Config {
            inertia: Some(Default::default()),
            ..Default::default()
        };
        for stationary in [false, true] {
            let (mut h, t) = enabled_motion(config, stationary);
            let mut t = t + 50_000;
            h.observe(
                config,
                t,
                Some(Evidence {
                    hand_id: 7,
                    source_us: t,
                    pose: Pose::Other,
                    wrist: Point { x: 0.41, y: 0.41 },
                }),
                300_000,
            );
            assert_eq!(h.feedback(config).releasing, !stationary);
            t += 50_000;
            h.observe(
                config,
                t,
                Some(Evidence {
                    hand_id: 7,
                    source_us: t,
                    pose: Pose::Neutral,
                    wrist: Point { x: 0.41, y: 0.41 },
                }),
                300_000,
            );
            assert_eq!(h.feedback(config).inertia, !stationary);
            let mut sum = 0;
            let mut previous = i32::MAX;
            for _ in 0..20 {
                t += 50_000;
                let m = h.observe(
                    config,
                    t,
                    Some(Evidence {
                        hand_id: 7,
                        source_us: t,
                        pose: Pose::Neutral,
                        wrist: Point { x: 0.41, y: 0.41 },
                    }),
                    300_000,
                );
                if stationary {
                    assert_eq!(m, Motion::default());
                } else {
                    assert!(m.horizontal >= 0 && m.vertical <= 0);
                    assert!(m.horizontal <= previous);
                    previous = m.horizontal;
                    sum += m.horizontal;
                }
            }
            assert!(!h.feedback(config).inertia);
            if !stationary {
                assert!(
                    sum > 0
                        && sum as f64
                            <= config.inertia.unwrap().max_distance * config.steps_per_unit * 120.0
                                + 1.0
                );
            }
        }
    }
    #[test]
    fn inertia_is_cut_by_loss_and_regrab_and_acceleration_increases_fast_travel() {
        let config = Config {
            inertia: Some(Default::default()),
            ..Default::default()
        };
        for regrab in [false, true] {
            let (mut h, t) = enabled_motion(config, false);
            let t = t + 50_000;
            h.observe(
                config,
                t,
                Some(Evidence {
                    hand_id: 7,
                    source_us: t,
                    pose: Pose::Neutral,
                    wrist: Point { x: 0.4, y: 0.4 },
                }),
                300_000,
            );
            assert!(h.feedback(config).inertia);
            let t = t + 50_000;
            let m = h.observe(
                config,
                t,
                regrab.then_some(Evidence {
                    hand_id: 7,
                    source_us: t,
                    pose: Pose::Fist,
                    wrist: Point { x: 0.4, y: 0.4 },
                }),
                300_000,
            );
            assert_eq!(m, Motion::default());
            assert!(!h.feedback(config).inertia);
            assert_eq!(h.feedback(config).active, regrab);
        }
        let total = |gain| {
            let c = Config {
                acceleration_max: gain,
                ..Default::default()
            };
            let (mut h, t) = enabled_motion(c, false);
            h.observe(
                c,
                t + 50_000,
                Some(Evidence {
                    hand_id: 7,
                    source_us: t + 50_000,
                    pose: Pose::Fist,
                    wrist: Point { x: 0.38, y: 0.38 },
                }),
                300_000,
            )
            .horizontal
        };
        assert!(total(2.5) > total(1.0));
    }
}
