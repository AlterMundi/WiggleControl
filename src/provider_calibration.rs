//! Guided provider calibration. Reuses the native sampling/review/store engines;
//! owns neither acquisition nor an input writer. The caller confirms camera closure.
use crate::{calibration::Control, calibration_runtime::Runtime, guided_calibration::Guide, Point};
use serde_json::{json, Value};
use std::{io, path::Path};

pub struct Run {
    runtime: Runtime,
    guide: Option<Guide>,
    last_source: Option<u64>,
    target: Option<Point>,
    camera_closed: bool,
    cancelled: bool,
    saved: bool,
    failure: Option<String>,
}
impl Run {
    pub fn new(profile: &Path, output: [u32; 2]) -> io::Result<Self> {
        if !profile.is_absolute() {
            return Err(io::Error::other(
                "choose an absolute calibration profile path",
            ));
        }
        Ok(Self {
            runtime: Runtime::new(profile, output)?,
            guide: None,
            last_source: None,
            target: None,
            camera_closed: true,
            cancelled: false,
            saved: false,
            failure: None,
        })
    }
    pub fn start(&mut self) -> Result<(), String> {
        if self.guide.is_some() || self.cancelled || self.saved {
            return Err("Retry or reopen calibration before Start".into());
        }
        self.runtime.control(Control::StartDiagonal)?;
        self.guide = Some(Guide::default());
        self.camera_closed = false;
        Ok(())
    }
    pub fn running(&self) -> bool {
        self.guide.is_some() && !self.finished()
    }
    pub fn finished(&self) -> bool {
        self.cancelled
            || self.saved
            || self.failure.is_some()
            || self.guide.as_ref().is_some_and(Guide::finished)
    }
    pub fn done(&self) -> bool {
        self.cancelled || self.saved
    }
    pub fn camera_closed(&self) -> bool {
        self.camera_closed
    }
    pub fn acknowledge(&mut self, step: usize, now: u64) -> Result<(), String> {
        let guide = self.guide.as_mut().ok_or("Start calibration first")?;
        if guide.finished()
            || self.failure.is_some()
            || self.last_source.is_none()
            || guide.step() != step
            || self.camera_closed
            || self.cancelled
        {
            return Err("Calibration step is not ready for display acknowledgment".into());
        }
        guide.acknowledge(now);
        Ok(())
    }
    pub fn observe(&mut self, point: Option<Point>, source: u64, now: u64, reason: &str) {
        if !self.running() {
            return;
        }
        let guide = self.guide.as_mut().unwrap();
        self.last_source = Some(source);
        let raw = point.filter(|_| guide.collecting(now));
        self.target = self.runtime.frame(raw, raw, source, now, vec![]).0;
        guide.tracking_feedback(Some(reason));
        guide.update(&mut self.runtime, now);
    }
    pub fn tick(&mut self, now: u64) {
        if !self.running() {
            return;
        }
        if self
            .last_source
            .is_some_and(|s| now.saturating_sub(s) >= 150_000)
        {
            self.runtime.frame(None, None, now, now, vec![]);
            self.target = None;
            self.guide
                .as_mut()
                .unwrap()
                .tracking_feedback(Some("Calibration: frame delayed"));
        }
        self.guide.as_mut().unwrap().update(&mut self.runtime, now);
    }
    pub fn limit_reached(&mut self) {
        if self.running() {
            self.failure = Some("Calibration capture limit reached; retry when ready".into());
        }
    }
    /// Called only after the producer has joined and its camera is closed.
    pub fn confirm_camera_closed(&mut self) {
        self.camera_closed = true;
    }
    pub fn save(&mut self) -> Result<(), String> {
        if !self.camera_closed
            || self.failure.is_some()
            || self.cancelled
            || self.saved
            || !self.runtime.verification_saved()
            || !self.guide.as_ref().is_some_and(Guide::finished)
        {
            return Err("Verify all four corners and wait for camera closure before Save".into());
        }
        self.runtime.control(Control::Accept)?;
        self.saved = true;
        Ok(())
    }
    pub fn cancel(&mut self) -> Result<(), String> {
        if self.saved {
            return Err("Calibration already saved".into());
        }
        if self.guide.is_some() {
            self.runtime.control(Control::Cancel)?;
        }
        self.cancelled = true;
        Ok(())
    }
    pub fn retry(&mut self) -> Result<(), String> {
        if !self.camera_closed || !self.finished() || self.done() {
            return Err("Wait for camera closure before Retry".into());
        }
        self.runtime.control(Control::Cancel)?;
        self.guide = None;
        self.failure = None;
        self.last_source = None;
        self.target = None;
        Ok(())
    }
    pub fn state(&self, now: u64) -> Value {
        let mut guide = self.guide.as_ref().map(|g| g.state(now));
        let phase = if self.saved {
            "saved"
        } else if self.cancelled {
            "cancelled"
        } else if self.failure.is_some() {
            "retry"
        } else if let Some(guide) = &guide {
            guide["phase"].as_str().unwrap_or("waiting")
        } else {
            "idle"
        }
        .to_string();
        if let (Some(guide), Some(message)) = (&mut guide, &self.failure) {
            guide["instruction"] = json!(message);
        }
        if self.camera_closed {
            if let Some(guide) = &mut guide {
                if phase == "complete" {
                    guide["instruction"] = json!("All four corners verified");
                } else if phase == "retry" && self.failure.is_none() {
                    guide["instruction"] = json!("Measurement incomplete; retry when ready");
                }
            }
        }
        json!({"phase":phase,"guide":guide,"review":self.runtime.state(),
            "target":self.target.map(|p|[p.x,p.y]),"has_frame":self.last_source.is_some(),
            "camera_closed":self.camera_closed,"saved":self.saved})
    }
}
