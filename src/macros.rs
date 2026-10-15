//! Deliberate one-shot macro meaning. No shell execution, key injection or sensors.
//! A perception classifier supplies named pose evidence; missing tracking is not neutral.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Pose {
    OpenPalm,
    Victory,
    ThumbsUp,
    ThumbsDown,
    PointingUp,
    ClosedFist,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    Recenter,
    TogglePrecision,
    ToggleScroll,
    CycleMode,
    PlayPause,
    VolumeUp,
    VolumeDown,
    OpenLauncher,
    Calibrate,
    PauseController,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    pub pose: Pose,
    pub action: Action,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub hold_us: u64,
    pub neutral_us: u64,
    pub cooldown_us: u64,
    pub stale_us: u64,
    pub bindings: Vec<Binding>,
}
/// Explicit native classifier configuration. Config checks do not load its model.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VisualPolicy {
    pub model: String,
    pub neutral_pose: Pose,
    #[serde(default = "visual_confidence")]
    pub min_confidence: f64,
}
fn visual_confidence() -> f64 {
    0.8
}
impl VisualPolicy {
    pub fn validate(&self, macros: Option<&Config>) -> Result<(), &'static str> {
        let macros = macros.ok_or("visual macros require configured macro bindings")?;
        if self.model.trim().is_empty()
            || self.model.len() > 4096
            || self.model.contains('\0')
            || !(0.8..=1.0).contains(&self.min_confidence)
            || macros.bindings.iter().any(|b| b.pose == self.neutral_pose)
        {
            return Err("invalid visual model/confidence or neutral pose also bound to an action");
        }
        Ok(())
    }
    /// Unknown/unsupported labels remain absent, never positively observed neutral.
    pub fn evidence(&self, id: u64, confidence: f64, pose: Option<Pose>) -> Option<HandPose> {
        let pose = pose?;
        if !confidence.is_finite() || !(self.min_confidence..=1.0).contains(&confidence) {
            return None;
        }
        Some(HandPose {
            id,
            confidence,
            pose: (pose != self.neutral_pose).then_some(pose),
        })
    }
}
#[derive(Debug, Serialize)]
pub struct Feedback {
    pub enabled: bool,
    pub armed: bool,
    pub owner_hand: Option<u64>,
    pub candidate: Option<Pose>,
    /// Progress is derived from actual observations, never a UI dwell timer.
    pub hold_elapsed_us: u64,
    pub hold_required_us: u64,
    pub neutral_elapsed_us: u64,
    pub neutral_required_us: u64,
    pub cooldown_remaining_us: u64,
    pub blocked_reason: Option<&'static str>,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HandPose {
    pub id: u64,
    pub confidence: f64,
    /// None means a confidently observed neutral pose, never missing tracking.
    pub pose: Option<Pose>,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Frame {
    pub version: u32,
    pub source_us: u64,
    pub ambiguous: bool,
    pub contacts_active: bool,
    pub hand: Option<HandPose>,
}
pub struct Engine {
    config: Config,
    enabled: bool,
    last_now: Option<u64>,
    watermark: Option<u64>,
    last_source: Option<u64>,
    hand: Option<u64>,
    neutral_since: Option<u64>,
    candidate: Option<(Pose, u64)>,
    armed: bool,
    last_action: Option<u64>,
    blocked_reason: Option<&'static str>,
}
impl Engine {
    pub fn new(config: Config) -> Result<Self, &'static str> {
        if !(200_000..=3_000_000).contains(&config.hold_us)
            || !(100_000..=2_000_000).contains(&config.neutral_us)
            || !(200_000..=5_000_000).contains(&config.cooldown_us)
            || !(50_000..=1_000_000).contains(&config.stale_us)
            || config.bindings.is_empty()
            || config.bindings.len() > 16
            || config
                .bindings
                .iter()
                .enumerate()
                .any(|(i, b)| config.bindings[..i].iter().any(|a| a.pose == b.pose))
        {
            return Err("invalid macro timing or duplicate/empty pose bindings");
        }
        Ok(Self {
            config,
            enabled: false,
            last_now: None,
            watermark: None,
            last_source: None,
            hand: None,
            neutral_since: None,
            candidate: None,
            armed: false,
            last_action: None,
            blocked_reason: Some("stopped"),
        })
    }
    fn neutralize(&mut self, reason: &'static str) {
        self.armed = false;
        self.neutral_since = None;
        self.candidate = None;
        self.blocked_reason = Some(reason);
    }
    pub fn start(&mut self) {
        self.neutralize("neutral_required");
        self.enabled = true;
    }
    pub fn stop(&mut self) {
        self.enabled = false;
        self.neutralize("stopped");
    }
    pub fn owner_hand(&self) -> Option<u64> {
        self.hand
    }
    pub fn feedback(&self) -> Feedback {
        let source = self.last_source.unwrap_or(0);
        Feedback {
            enabled: self.enabled,
            armed: self.armed,
            owner_hand: self.hand,
            candidate: self.candidate.map(|(pose, _)| pose),
            hold_elapsed_us: self.candidate.map_or(0, |(_, since)| {
                source.saturating_sub(since).min(self.config.hold_us)
            }),
            hold_required_us: self.config.hold_us,
            neutral_elapsed_us: self.neutral_since.map_or(0, |since| {
                source.saturating_sub(since).min(self.config.neutral_us)
            }),
            neutral_required_us: self.config.neutral_us,
            cooldown_remaining_us: self.last_action.map_or(0, |t| {
                self.config
                    .cooldown_us
                    .saturating_sub(self.last_now.unwrap_or(0).saturating_sub(t))
            }),
            blocked_reason: self.blocked_reason,
        }
    }
    pub fn tick(&mut self, now: u64) {
        if self.last_now.is_some_and(|t| now < t)
            || self
                .last_source
                .is_some_and(|t| now.saturating_sub(t) >= self.config.stale_us)
        {
            self.neutralize("stale_or_regressing_clock");
        }
        self.last_now = Some(now);
    }
    pub fn observe(&mut self, now: u64, frame: Frame) -> Option<Action> {
        let regressed = self.last_now.is_some_and(|t| now < t);
        self.tick(now);
        if !self.enabled {
            return None;
        }
        if regressed
            || frame.version != 1
            || frame.source_us > now
            || now - frame.source_us >= self.config.stale_us
            || self.watermark.is_some_and(|t| frame.source_us <= t)
        {
            self.neutralize("invalid_observation");
            return None;
        }
        self.watermark = Some(frame.source_us);
        let continuous = self
            .last_source
            .is_none_or(|t| frame.source_us - t < self.config.stale_us);
        self.last_source = Some(frame.source_us);
        if frame.ambiguous || frame.contacts_active {
            self.neutralize(if frame.ambiguous {
                "ambiguous_identity"
            } else {
                "input_busy"
            });
            return None;
        }
        let Some(hand) = frame.hand else {
            self.neutralize("missing_pose");
            return None;
        };
        if !hand.confidence.is_finite() || !(0.8..=1.0).contains(&hand.confidence) {
            self.neutralize("low_confidence");
            return None;
        }
        if !continuous || self.hand != Some(hand.id) {
            self.neutralize("identity_or_continuity_changed");
        }
        self.hand = Some(hand.id);
        let Some(pose) = hand.pose else {
            self.candidate = None;
            let since = *self.neutral_since.get_or_insert(frame.source_us);
            if frame.source_us - since >= self.config.neutral_us {
                self.armed = true;
            }
            self.blocked_reason = (!self.armed).then_some("neutral_hold");
            return None;
        };
        self.neutral_since = None;
        if !self.armed {
            self.blocked_reason = Some("neutral_required");
            return None;
        }
        let Some(binding) = self.config.bindings.iter().find(|b| b.pose == pose) else {
            self.candidate = None;
            self.blocked_reason = Some("unbound_pose");
            return None;
        };
        let action = binding.action;
        let since = match self.candidate {
            Some((previous, since)) if previous == pose => since,
            _ => {
                self.candidate = Some((pose, frame.source_us));
                frame.source_us
            }
        };
        self.blocked_reason = None;
        if frame.source_us - since < self.config.hold_us
            || self
                .last_action
                .is_some_and(|t| frame.source_us.saturating_sub(t) < self.config.cooldown_us)
        {
            if self
                .last_action
                .is_some_and(|t| frame.source_us.saturating_sub(t) < self.config.cooldown_us)
            {
                self.blocked_reason = Some("cooldown");
            }
            return None;
        }
        self.last_action = Some(frame.source_us);
        self.neutralize("neutral_required");
        Some(action)
    }
}

#[cfg(target_os = "linux")]
pub fn replay_stdin(config_path: &std::path::Path) -> std::io::Result<()> {
    use std::{
        fs,
        io::{self, Write},
    };
    #[derive(Deserialize)]
    #[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
    enum Command {
        Start,
        Stop,
        Tick { now_us: u64 },
        Frame { now_us: u64, observation: Frame },
    }
    if fs::metadata(config_path)?.len() > 8192 {
        return Err(io::Error::other("macro config exceeds8KiB"));
    }
    let config = serde_json::from_slice(&fs::read(config_path)?)?;
    let mut engine = Engine::new(config).map_err(io::Error::other)?;
    let mut output = io::stdout().lock();
    for command in crate::intent::reader::<Command>() {
        let action = match command? {
            Command::Start => {
                engine.start();
                None
            }
            Command::Stop => {
                engine.stop();
                None
            }
            Command::Tick { now_us } => {
                engine.tick(now_us);
                None
            }
            Command::Frame {
                now_us,
                observation,
            } => engine.observe(now_us, observation),
        };
        if let Some(action) = action {
            writeln!(
                output,
                "{}",
                serde_json::json!({"type":"macro","action":action,"executed":false})
            )?;
            output.flush()?;
        }
    }
    engine.stop();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn engine() -> Engine {
        let mut e = Engine::new(Config {
            hold_us: 500_000,
            neutral_us: 200_000,
            cooldown_us: 800_000,
            stale_us: 150_000,
            bindings: vec![Binding {
                pose: Pose::Victory,
                action: Action::TogglePrecision,
            }],
        })
        .unwrap();
        e.start();
        e
    }
    fn frame(t: u64, pose: Option<Pose>) -> Frame {
        Frame {
            version: 1,
            source_us: t,
            ambiguous: false,
            contacts_active: false,
            hand: Some(HandPose {
                id: 7,
                confidence: 0.99,
                pose,
            }),
        }
    }
    #[test]
    fn neutral_then_hold_fires_once_and_requires_positive_neutral_to_rearm() {
        let mut e = engine();
        let mut actions = Vec::new();
        for n in 0..50 {
            if let Some(a) = e.observe(
                n * 100_000,
                frame(n * 100_000, if n < 4 { None } else { Some(Pose::Victory) }),
            ) {
                actions.push(a)
            }
        }
        assert_eq!(actions, vec![Action::TogglePrecision]);
        for n in 50..54 {
            assert!(e.observe(n * 100_000, frame(n * 100_000, None)).is_none());
        }
        assert_eq!(
            (54..65)
                .filter_map(|n| e.observe(n * 100_000, frame(n * 100_000, Some(Pose::Victory))))
                .count(),
            1
        );
    }
    #[test]
    fn dropout_contact_ambiguity_identity_stale_or_clock_failure_cannot_complete_hold() {
        for failure in 0..8 {
            let mut e = engine();
            for n in 0..7 {
                e.observe(
                    n * 100_000,
                    frame(n * 100_000, if n < 3 { None } else { Some(Pose::Victory) }),
                );
            }
            let mut f = frame(700_000, Some(Pose::Victory));
            let mut now = 700_000;
            match failure {
                0 => f.hand = None,
                1 => f.contacts_active = true,
                2 => f.ambiguous = true,
                3 => f.hand.as_mut().unwrap().id = 8,
                4 => now = 900_000,
                5 => now = 500_000,
                6 => f.hand.as_mut().unwrap().confidence = 0.5,
                _ => f.source_us = 600_000,
            }
            assert!(e.observe(now, f).is_none());
            for n in 10..25 {
                assert!(
                    e.observe(n * 100_000, frame(n * 100_000, Some(Pose::Victory)))
                        .is_none(),
                    "failure{failure}"
                );
            }
        }
    }
}
