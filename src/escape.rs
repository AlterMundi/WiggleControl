//! Orientation-independent thumb-out candidate; source evidence, not human intent.
//! Reuses the one-shot macro engine and the existing finger geometry measurement.
use crate::{detail::finger_ratio, macros};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub hold_us: u64,
    pub neutral_us: u64,
    pub folded_ratio: f64,
    pub extended_ratio: f64,
    pub thumb_clearance_widths: f64,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            hold_us: 700_000,
            neutral_us: 200_000,
            folded_ratio: 0.55,
            extended_ratio: 0.8,
            thumb_clearance_widths: 0.35,
        }
    }
}
impl Config {
    pub fn validate(self) -> Result<(), &'static str> {
        if !(200_000..=2_000_000).contains(&self.hold_us)
            || !(100_000..=1_000_000).contains(&self.neutral_us)
            || !self.folded_ratio.is_finite()
            || !self.extended_ratio.is_finite()
            || !(0.1..=0.8).contains(&self.folded_ratio)
            || !(self.folded_ratio + 0.1..=1.0).contains(&self.extended_ratio)
            || !self.thumb_clearance_widths.is_finite()
            || !(0.1..=1.5).contains(&self.thumb_clearance_widths)
        {
            return Err("invalid Escape hold, rearm or finger geometry");
        }
        Ok(())
    }
    pub(crate) fn engine(
        self,
        stale_us: u64,
        min_confidence: f64,
    ) -> Result<macros::Engine, &'static str> {
        self.validate()?;
        macros::Engine::with_min_confidence(
            macros::Config {
                hold_us: self.hold_us,
                neutral_us: self.neutral_us,
                cooldown_us: self.hold_us,
                stale_us,
                bindings: vec![macros::Binding {
                    pose: macros::Pose::ThumbOut,
                    action: macros::Action::Escape,
                }],
            },
            min_confidence,
        )
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Pose {
    Neutral,
    ThumbOut,
    Other,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Evidence {
    pub hand_id: u64,
    pub source_us: u64,
    /// A positive raw pose from a fully in-view hand; missing is not Other/Neutral.
    pub pose: Pose,
}
pub fn classify(config: Config, world: &[[f64; 3]; 21]) -> Option<Pose> {
    if world.iter().flatten().any(|v| !v.is_finite()) {
        return None;
    }
    let mut ratios = [0.0; 4];
    for (i, start) in [5, 9, 13, 17].into_iter().enumerate() {
        ratios[i] = finger_ratio(world[start..start + 4].try_into().ok()?)?;
    }
    let thumb = finger_ratio(world[1..5].try_into().ok()?)?;
    let across = std::array::from_fn::<_, 3, _>(|i| world[5][i] - world[17][i]);
    let width_squared = across.iter().map(|v| v * v).sum::<f64>();
    if !width_squared.is_finite()
        || width_squared < 1e-12
        || !thumb.is_finite()
        || ratios.iter().any(|r| !r.is_finite())
    {
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
        && thumb >= config.extended_ratio
        && clearance >= config.thumb_clearance_widths
    {
        Some(Pose::ThumbOut)
    } else {
        Some(Pose::Other)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    /// Controlled geometry for contract tests, not an anatomical/model fixture.
    pub(crate) fn world(pose: Pose) -> [[f64; 3]; 21] {
        let mut points = [[0.0; 3]; 21];
        for (i, start) in [5, 9, 13, 17].into_iter().enumerate() {
            let x = -0.03 + i as f64 * 0.02;
            let folded = pose == Pose::ThumbOut || (pose == Pose::Other && i >= 2);
            let chain = if folded {
                [
                    [x, 0.0, 0.0],
                    [x, 0.015, 0.0],
                    [x, 0.015, 0.01],
                    [x, 0.0, 0.01],
                ]
            } else {
                [
                    [x, 0.0, 0.0],
                    [x, 0.01, 0.0],
                    [x, 0.02, 0.0],
                    [x, 0.03, 0.0],
                ]
            };
            points[start..start + 4].copy_from_slice(&chain);
        }
        for (i, p) in points[1..5].iter_mut().enumerate() {
            *p = [-0.03 - i as f64 * 0.015, -0.02, 0.0];
        }
        points
    }
    #[test]
    fn four_finger_thumb_out_is_distinct_from_neutral_and_detail() {
        for pose in [Pose::Neutral, Pose::Other, Pose::ThumbOut] {
            let original = world(pose);
            assert_eq!(classify(Config::default(), &original), Some(pose));
            let transformed =
                original.map(|p| [2.0 * p[2] + 0.4, 2.0 * p[0] - 0.3, 2.0 * p[1] + 0.1]);
            assert_eq!(classify(Config::default(), &transformed), Some(pose));
        }
        let mut tucked = world(Pose::ThumbOut);
        tucked[4][0] = 0.0;
        assert_eq!(classify(Config::default(), &tucked), Some(Pose::Other));
        let mut invalid = world(Pose::ThumbOut);
        invalid[12][0] = f64::NAN;
        assert_eq!(classify(Config::default(), &invalid), None);
        assert_eq!(classify(Config::default(), &[[0.0; 3]; 21]), None);
        let overflow = world(Pose::ThumbOut).map(|p| p.map(|v| v * 1e308));
        assert_eq!(classify(Config::default(), &overflow), None);
    }
    #[test]
    fn other_pose_cannot_be_bound_as_a_deliberate_action() {
        let config = Config::default();
        assert!(config.engine(300_000, 0.5).is_ok());
        assert!(Config {
            hold_us: 100_000,
            ..config
        }
        .engine(300_000, 0.5)
        .is_err());
        assert!(macros::Engine::new(macros::Config {
            hold_us: 700_000,
            neutral_us: 200_000,
            cooldown_us: 700_000,
            stale_us: 300_000,
            bindings: vec![macros::Binding {
                pose: macros::Pose::Other,
                action: macros::Action::Escape
            }]
        })
        .is_err());
    }
}
