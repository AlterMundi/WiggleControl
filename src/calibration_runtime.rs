//! Session coordination for neutral sampling, hover review and guarded adoption.
use crate::{
    calibration::{Calibration, Control, Profile, Review, Stage},
    profile_store, Event, Point,
};
use serde_json::{json, Value};
use std::{
    io,
    path::{Path, PathBuf},
};
pub struct Runtime {
    path: PathBuf,
    output: [u32; 2],
    profile: Option<Profile>,
    sampling: Option<Calibration>,
    review: Option<Review>,
    pub last_target: Option<Point>,
    message: String,
}
impl Runtime {
    pub fn new(path: &Path, output: [u32; 2]) -> io::Result<Self> {
        Profile {
            version: 1,
            workspace: Default::default(),
            output,
        }
        .validate()
        .map_err(io::Error::other)?;
        let profile = match profile_store::load(path) {
            Ok(p) => {
                if p.output != output {
                    return Err(io::Error::other(
                        "calibration output differs from configured output",
                    ));
                }
                Some(p)
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => None,
            Err(e) => return Err(e),
        };
        Ok(Self {
            path: path.into(),
            output,
            profile,
            sampling: None,
            review: None,
            last_target: None,
            message: "Ready for manual calibration".into(),
        })
    }
    pub fn gesture_entry_available(&self) -> bool {
        self.profile.is_some() && self.sampling.is_none() && self.review.is_none()
    }
    pub fn prayer_requested(&mut self) -> Result<(), String> {
        if !self.gesture_entry_available() {
            return Err("Prayer entry requires an active accepted profile".into());
        }
        self.control(Control::Start)?;
        self.message = "Prayer requested calibration; input neutral".into();
        Ok(())
    }
    /// Caller must release/rearm its gesture engine and writer for every command,
    /// including rejected commands, before processing new observations.
    pub fn control(&mut self, action: Control) -> Result<(), String> {
        let result = (|| {
            match action {
                Control::Start => {
                    self.sampling = Some(Calibration::default());
                    self.review = None;
                }
                Control::Next => {
                    let cal = self
                        .sampling
                        .as_mut()
                        .ok_or("No sampling stage is active")?;
                    let stage = cal.advance()?;
                    if stage == Stage::Review {
                        self.review = Some(Review::new(cal.candidate(self.output)?)?);
                        self.sampling = None;
                    }
                }
                Control::Cancel => {
                    self.sampling = None;
                    self.review = None;
                }
                Control::Accept => {
                    let profile = self
                        .review
                        .as_ref()
                        .ok_or("No corner review is active")?
                        .accepted()?;
                    profile_store::adopt(&self.path, profile).map_err(|e| e.to_string())?;
                    self.profile = Some(profile);
                    self.review = None;
                }
                Control::Rollback => {
                    let profile =
                        profile_store::previous_profile(&self.path).map_err(|e| e.to_string())?;
                    if profile.output != self.output {
                        return Err("Previous profile belongs to another output".into());
                    }
                    profile_store::adopt(&self.path, profile).map_err(|e| e.to_string())?;
                    self.profile = Some(profile);
                    self.sampling = None;
                    self.review = None;
                }
            }
            Ok::<(), String>(())
        })();
        self.message = match &result {
            Ok(_) => format!("{action:?} applied"),
            Err(e) => e.clone(),
        };
        result
    }
    pub fn frame(
        &mut self,
        raw: Option<Point>,
        pointer: Option<Point>,
        source_us: u64,
        now_us: u64,
        events: Vec<Event>,
    ) -> (Option<Point>, Vec<Event>) {
        let fresh = source_us <= now_us && now_us - source_us < 150_000;
        let raw = raw.filter(|p| p.valid() && fresh);
        if let Some(cal) = &mut self.sampling {
            if let Some(p) = raw {
                cal.sample(p, source_us, now_us);
            } else {
                cal.tracking_lost();
            }
            return (None, Vec::new());
        }
        if let Some(review) = &mut self.review {
            let target = if let Some(p) = raw {
                review.observe(p, source_us, now_us).ok()
            } else {
                review.tracking_lost();
                None
            };
            return (target, Vec::new());
        }
        let target = if fresh {
            self.profile
                .and_then(|p| pointer.and_then(|point| p.target(point).ok()))
        } else {
            None
        };
        if target.is_none() {
            return (
                None,
                events
                    .into_iter()
                    .filter(|e| matches!(e, Event::Cancel { .. } | Event::Button { down: false }))
                    .collect(),
            );
        }
        (
            target,
            events
                .into_iter()
                .filter(|e| !matches!(e, Event::Move { .. }))
                .collect(),
        )
    }
    pub fn state(&self) -> Value {
        let stage = self
            .sampling
            .as_ref()
            .map(|s| serde_json::to_value(s.stage).unwrap())
            .unwrap_or_else(|| {
                json!(if self.review.is_some() {
                    "review"
                } else if self.profile.is_some() {
                    "active"
                } else {
                    "uncalibrated"
                })
            });
        json!({"stage":stage,"samples":self.sampling.as_ref().map(|s|s.sample_count()).unwrap_or(0),"corner_target":self.review.as_ref().and_then(|r|r.target()).map(|p|[p.x,p.y]),"corners_verified":self.review.as_ref().is_some_and(|r|r.verified()),"message":self.message,"output":self.output})
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sampling_review_adoption_cancel_and_rollback_preserve_neutral_contract() {
        let dir = std::env::temp_dir().join(format!("wiggle-runtime-test-{}", std::process::id()));
        std::fs::create_dir(&dir).unwrap();
        let path = dir.join("profile.json");
        let mut empty = Runtime::new(&path, [1920, 1080]).unwrap();
        assert!(!empty.gesture_entry_available());
        assert!(empty.prayer_requested().is_err());
        assert_eq!(empty.state()["stage"], "uncalibrated");
        let old = Profile {
            version: 1,
            workspace: Default::default(),
            output: [1920, 1080],
        };
        profile_store::adopt(&path, old).unwrap();
        let mut runtime = Runtime::new(&path, [1920, 1080]).unwrap();
        assert!(runtime.gesture_entry_available());
        runtime.prayer_requested().unwrap();
        assert!(!runtime.gesture_entry_available());
        assert!(runtime.prayer_requested().is_err());
        assert!(runtime.control(Control::Accept).is_err());
        let mut at = 1_000_000;
        for p in [
            Point { x: 0.2, y: 0.5 },
            Point { x: 0.8, y: 0.5 },
            Point { x: 0.5, y: 0.3 },
            Point { x: 0.5, y: 0.7 },
        ] {
            for _ in 0..12 {
                let (target, events) = runtime.frame(
                    Some(p),
                    Some(p),
                    at,
                    at + 50_000,
                    vec![Event::Button { down: true }],
                );
                assert!(target.is_none());
                assert!(events.is_empty());
                at += 100_000;
            }
            runtime.control(Control::Next).unwrap();
        }
        assert!(runtime.control(Control::Accept).is_err());
        for p in [
            Point { x: 0.2, y: 0.3 },
            Point { x: 0.8, y: 0.3 },
            Point { x: 0.8, y: 0.7 },
            Point { x: 0.2, y: 0.7 },
        ] {
            for _ in 0..6 {
                let (target, events) = runtime.frame(
                    Some(p),
                    Some(p),
                    at,
                    at + 50_000,
                    vec![Event::Button { down: true }, Event::Scroll { vertical: 1 }],
                );
                assert!(target.is_some());
                assert!(events.is_empty());
                at += 100_000;
            }
        }
        assert_eq!(runtime.state()["corners_verified"], true);
        runtime.control(Control::Accept).unwrap();
        assert_eq!(profile_store::load(&path).unwrap().workspace.min.x, 0.2);
        let p = Point { x: 0.8, y: 0.7 };
        let (target, events) = runtime.frame(
            Some(p),
            Some(p),
            at,
            at,
            vec![
                Event::Move { dx: 0.1, dy: 0.1 },
                Event::Button { down: true },
            ],
        );
        assert_eq!(target.unwrap().x, 1.0);
        assert_eq!(events, vec![Event::Button { down: true }]);
        let (target, events) = runtime.frame(
            None,
            None,
            at,
            at + 200_000,
            vec![Event::Button { down: true }, Event::Button { down: false }],
        );
        assert!(target.is_none());
        assert_eq!(events, vec![Event::Button { down: false }]);
        runtime.control(Control::Start).unwrap();
        runtime.control(Control::Cancel).unwrap();
        assert_eq!(profile_store::load(&path).unwrap().workspace.min.x, 0.2);
        runtime.control(Control::Rollback).unwrap();
        assert_eq!(profile_store::load(&path).unwrap().workspace.min.x, 0.0);
        assert!(Runtime::new(&path, [1280, 720]).is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
