//! Camera-free timed orchestration. No input writer and no implicit adoption.
use crate::{calibration::Control, calibration_runtime::Runtime};
use serde_json::{json, Value};

#[derive(Default)]
pub struct Guide {
    step: usize,
    started: Option<u64>,
    terminal: Option<String>,
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
    pub fn update(&mut self, runtime: &mut Runtime, now_us: u64) {
        if self.finished() {
            return;
        }
        let Some(start) = self.started else {
            return;
        };
        let state = runtime.state();
        if self.step >= 2 {
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
        } else if self.step < 2 && elapsed.is_some_and(|e| e < 15_000_000) {
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
}
