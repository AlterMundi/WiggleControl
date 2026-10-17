//! Camera-free timed orchestration. No input writer and no implicit adoption.
use crate::{calibration::Control, calibration_runtime::Runtime};
use serde_json::{json, Value};

#[derive(Default)]
pub struct Guide {
    step: usize,
    started: Option<u64>,
    terminal: Option<String>,
    holding_corner: bool,
    tracking_hint: Option<&'static str>,
}
impl Guide {
    pub fn step(&self) -> usize {
        self.step
    }
    pub fn acknowledge(&mut self, now_us: u64) {
        if self.started.is_none() && self.terminal.is_none() {
            self.started = Some(now_us);
        }
    }
    pub fn collecting(&self, now_us: u64) -> bool {
        self.terminal.is_none()
            && self
                .started
                .is_some_and(|s| self.step >= 2 || now_us.saturating_sub(s) >= 15_000_000)
    }
    pub fn finished(&self) -> bool {
        self.terminal.is_some()
    }
    /// Presentation only: sampling and review retain their own validity checks.
    pub fn tracking_feedback(&mut self, reason: Option<&str>) {
        self.tracking_hint = match reason {
            Some("Calibration: hand partly outside camera; move inward") => {
                Some("Hand outside frame: move inward")
            }
            Some("Calibration: no hand detected") => Some("Show your whole hand to the camera"),
            Some("Calibration: low hand confidence") => Some("Hand unclear: reposition in view"),
            Some("Calibration: frame delayed") => Some("Tracking delayed: hold position"),
            _ => None,
        };
    }
    pub fn update(&mut self, runtime: &mut Runtime, now_us: u64) {
        if self.finished() {
            return;
        }
        let Some(start) = self.started else {
            return;
        };
        let state = runtime.state();
        if self.step >= 2 {
            self.holding_corner = runtime.holding_review_target();
            if state["corners_verified"] == true {
                self.terminal = Some(if runtime.verification_saved() {
                    "complete".into()
                } else {
                    "retry: could not save corner verification".into()
                });
                return;
            }
            let expected = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]][self.step - 2];
            if state["corner_target"] != json!(expected) {
                self.step += 1;
                self.started = None;
                self.holding_corner = false;
                return;
            }
        }
        if now_us.saturating_sub(start) >= 20_000_000 {
            if self.step < 2 {
                match runtime.control(Control::Next) {
                    Ok(()) => {
                        self.step += 1;
                        self.started = None;
                    }
                    Err(e) => self.terminal = Some(format!("retry: {e}")),
                }
            } else {
                self.terminal = Some("retry: corner was not verified within 20 seconds".into());
            }
        }
    }
    pub fn state(&self, now_us: u64) -> Value {
        let elapsed = self.started.map(|s| now_us.saturating_sub(s));
        let phase = if let Some(t) = &self.terminal {
            if t == "complete" {
                "complete"
            } else {
                "retry"
            }
        } else if elapsed.is_none() {
            "waiting"
        } else if (self.step < 2 && elapsed.is_some_and(|e| e < 15_000_000))
            || (self.step >= 2 && !self.holding_corner)
            || self.tracking_hint.is_some()
        {
            "move"
        } else {
            "hold"
        };
        let instruction = if phase == "complete" {
            "All corners verified // camera stopping"
        } else if phase == "retry" {
            "Measurement incomplete // camera stopping"
        } else {
            [
                "Move to comfortable UPPER LEFT",
                "Move to comfortable LOWER RIGHT",
                "Reach UPPER LEFT screen corner",
                "Reach UPPER RIGHT screen corner",
                "Reach LOWER RIGHT screen corner",
                "Reach LOWER LEFT screen corner",
            ][self.step]
        };
        let instruction = if !matches!(phase, "retry" | "complete") {
            self.tracking_hint
                .map(|hint| format!("{instruction} // {hint}"))
                .unwrap_or_else(|| instruction.to_string())
        } else {
            instruction.to_string()
        };
        json!({"step":self.step,"phase":phase,"remaining_ms":elapsed.map(|e|(20_000_000_u64.saturating_sub(e))/1000),
            "instruction":instruction,"result":self.terminal})
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::Point;
    #[test]
    fn setup_is_not_timed_and_insufficient_samples_stop_without_adoption() {
        let path =
            std::env::temp_dir().join(format!("wiggle-guide-empty-{}.json", std::process::id()));
        let mut r = Runtime::new(&path, [1920, 1080]).unwrap();
        r.control(Control::StartDiagonal).unwrap();
        let mut g = Guide::default();
        g.update(&mut r, 100_000_000);
        assert_eq!(g.state(100_000_000)["phase"], "waiting");
        assert_eq!(g.state(100_000_000)["remaining_ms"], Value::Null);
        assert!(!g.collecting(100_000_000));
        g.acknowledge(100_000_000);
        assert!(!g.collecting(114_999_999));
        assert!(g.collecting(115_000_000));
        g.tracking_feedback(Some("Calibration: hand partly outside camera; move inward"));
        assert_eq!(g.state(115_000_000)["phase"], "move");
        assert!(g.state(115_000_000)["instruction"]
            .as_str()
            .unwrap()
            .contains("move inward"));
        g.tracking_feedback(Some("Calibration: hand visible; hold steady"));
        assert_eq!(g.state(115_000_000)["phase"], "hold");
        // A clear instruction alone must not create samples or allow advancement.
        g.update(&mut r, 120_000_000);
        assert_eq!(g.state(120_000_000)["phase"], "retry");
        assert!(!path.exists());
    }
    #[test]
    fn each_step_waits_for_paint_and_completes_with_saved_proof_only() {
        let dir = std::env::temp_dir().join(format!("wiggle-guide-full-{}", std::process::id()));
        std::fs::create_dir(&dir).unwrap();
        let path = dir.join("profile.json");
        let mut r = Runtime::new(&path, [1920, 1080]).unwrap();
        r.control(Control::StartDiagonal).unwrap();
        let mut g = Guide::default();
        let mut at = 1_000_000;
        for p in [Point { x: 0.2, y: 0.3 }, Point { x: 0.8, y: 0.7 }] {
            g.acknowledge(at);
            for n in 0..40 {
                let t = at + 15_000_000 + n * 100_000;
                assert!(g.collecting(t));
                r.frame(Some(p), Some(p), t, t, vec![]);
            }
            g.update(&mut r, at + 20_000_000);
            at += 30_000_000;
            assert_eq!(g.state(at)["phase"], "waiting");
        }
        assert_eq!(g.step(), 2);
        for p in [
            Point { x: 0.2, y: 0.3 },
            Point { x: 0.8, y: 0.3 },
            Point { x: 0.8, y: 0.7 },
            Point { x: 0.2, y: 0.7 },
        ] {
            g.acknowledge(at);
            for n in 0..6 {
                let t = at + n * 100_000;
                r.frame(Some(p), Some(p), t, t, vec![]);
                g.update(&mut r, t);
            }
            at += 2_000_000;
        }
        assert_eq!(g.state(at)["phase"], "complete");
        assert!(!path.exists());
        crate::calibration_runtime::accept_saved(&path).unwrap();
        assert!(path.exists());
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn corner_guidance_requests_movement_until_fresh_dwell_then_recovers_from_loss() {
        let dir = std::env::temp_dir().join(format!("wiggle-guide-motion-{}", std::process::id()));
        std::fs::create_dir(&dir).unwrap();
        let path = dir.join("profile.json");
        let mut runtime = Runtime::new(&path, [1920, 1080]).unwrap();
        runtime.control(Control::StartDiagonal).unwrap();
        let upper_left = Point { x: 0.2, y: 0.3 };
        let lower_right = Point { x: 0.8, y: 0.7 };
        let mut guide = Guide::default();
        let mut at = 1_000_000;
        for point in [upper_left, lower_right] {
            guide.acknowledge(at);
            for n in 0..40 {
                let time = at + 15_000_000 + n * 100_000;
                runtime.frame(Some(point), Some(point), time, time, vec![]);
            }
            guide.update(&mut runtime, at + 20_000_000);
            at += 30_000_000;
        }
        assert_eq!(guide.step(), 2);
        assert_eq!(guide.state(at)["phase"], "waiting");
        guide.acknowledge(at);
        // Remaining at the second measurement is not a hold at verification corner 1.
        runtime.frame(Some(lower_right), Some(lower_right), at, at, vec![]);
        guide.update(&mut runtime, at);
        assert_eq!(guide.state(at)["phase"], "move");
        at += 100_000;
        runtime.frame(Some(upper_left), Some(upper_left), at, at, vec![]);
        guide.update(&mut runtime, at);
        assert_eq!(guide.state(at)["phase"], "hold");
        at += 100_000;
        runtime.frame(None, None, at, at, vec![]);
        guide.update(&mut runtime, at);
        assert_eq!(guide.state(at)["phase"], "move");
        for n in 0..6 {
            let time = at + 100_000 + n * 100_000;
            runtime.frame(Some(upper_left), Some(upper_left), time, time, vec![]);
            guide.update(&mut runtime, time);
        }
        assert_eq!(guide.step(), 3);
        assert_eq!(guide.state(at + 700_000)["phase"], "waiting");
        assert!(!runtime.verification_saved());
        assert!(!path.exists());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
