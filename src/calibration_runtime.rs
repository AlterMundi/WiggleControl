//! Session coordination for neutral sampling, hover review and guarded adoption.
use crate::{
    calibration::{Calibration, Control, Profile, Review, Stage},
    profile_store, Event, Point,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    io,
    path::{Path, PathBuf},
};
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Draft {
    version: u32,
    output: [u32; 2],
    bounds: [Option<f64>; 4],
    #[serde(default)]
    diagonal: bool,
}
fn draft_path(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_os_string();
    name.push(".draft");
    PathBuf::from(name)
}
fn verified_path(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_os_string();
    name.push(".verified");
    PathBuf::from(name)
}
/// Explicit operator adoption after finite expiry, without acquisition.
/// The completed verification must match the unchanged confirmed draft.
pub fn accept_saved(path: &Path) -> io::Result<()> {
    let draft: Draft = profile_store::load_value(&draft_path(path))?;
    if draft.version != 1 {
        return Err(io::Error::other("invalid draft version"));
    }
    let cal = if draft.diagonal {
        Calibration::from_diagonal_bounds(draft.bounds)
    } else {
        Calibration::from_bounds(draft.bounds)
    }
    .map_err(io::Error::other)?;
    let candidate = cal.candidate(draft.output).map_err(io::Error::other)?;
    let verified = profile_store::load(&verified_path(path))?;
    if serde_json::to_value(candidate)? != serde_json::to_value(verified)? {
        return Err(io::Error::other(
            "saved verification does not match current draft",
        ));
    }
    profile_store::adopt(path, verified)?;
    std::fs::remove_file(draft_path(path))?;
    std::fs::remove_file(verified_path(path))?;
    Ok(())
}
pub struct Runtime {
    path: PathBuf,
    output: [u32; 2],
    profile: Option<Profile>,
    sampling: Option<Calibration>,
    review: Option<Review>,
    pub last_target: Option<Point>,
    message: String,
    review_saved: bool,
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
        let mut sampling = None;
        let mut review = None;
        let mut message = "Ready for manual calibration".to_string();
        match profile_store::load_value::<Draft>(&draft_path(path)) {
            Ok(draft) => {
                if draft.version != 1 || draft.output != output {
                    return Err(io::Error::other("invalid calibration draft version/output"));
                }
                let cal = if draft.diagonal {
                    Calibration::from_diagonal_bounds(draft.bounds)
                } else {
                    Calibration::from_bounds(draft.bounds)
                }
                .map_err(io::Error::other)?;
                if cal.stage == Stage::Review {
                    review = Some(
                        Review::new(cal.candidate(output).map_err(io::Error::other)?)
                            .map_err(io::Error::other)?,
                    );
                } else {
                    sampling = Some(cal);
                }
                message = "Resumed confirmed bounds; fresh sampling/corner checks required".into();
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => (),
            Err(e) => return Err(e),
        }
        Ok(Self {
            path: path.into(),
            output,
            profile,
            sampling,
            review,
            last_target: None,
            message,
            review_saved: false,
        })
    }
    pub fn verification_saved(&self) -> bool {
        self.review_saved
    }
    pub fn holding_review_target(&self) -> bool {
        self.review.as_ref().is_some_and(Review::holding_target)
    }
    pub fn is_sampling_or_reviewing(&self) -> bool {
        self.sampling.is_some() || self.review.is_some()
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
                Control::Start | Control::StartDiagonal => {
                    self.clear_draft()?;
                    self.sampling = Some(if action == Control::StartDiagonal {
                        Calibration::diagonal()
                    } else {
                        Calibration::default()
                    });
                    self.review = None;
                    self.review_saved = false;
                }
                Control::Next => {
                    let mut cal = self
                        .sampling
                        .as_ref()
                        .ok_or("No sampling stage is active")?
                        .clone();
                    let stage = cal.advance()?;
                    profile_store::atomic_value(
                        &draft_path(&self.path),
                        &Draft {
                            version: 1,
                            output: self.output,
                            bounds: cal.confirmed_bounds(),
                            diagonal: cal.is_diagonal(),
                        },
                    )
                    .map_err(|e| e.to_string())?;
                    if stage == Stage::Review {
                        self.review = Some(Review::new(cal.candidate(self.output)?)?);
                        self.sampling = None;
                    } else {
                        self.sampling = Some(cal);
                    }
                }
                Control::Cancel => {
                    self.clear_draft()?;
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
                    self.clear_draft()?;
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
                    self.clear_draft()?;
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
    fn clear_draft(&self) -> Result<(), String> {
        for path in [draft_path(&self.path), verified_path(&self.path)] {
            match std::fs::remove_file(path) {
                Ok(()) => (),
                Err(e) if e.kind() == io::ErrorKind::NotFound => (),
                Err(e) => return Err(e.to_string()),
            }
        }
        Ok(())
    }
    pub fn frame(
        &mut self,
        raw: Option<Point>,
        pointer: Option<Point>,
        source_us: u64,
        now_us: u64,
        events: Vec<Event>,
    ) -> (Option<Point>, Vec<Event>) {
        self.frame_with_freshness(raw, pointer, (source_us, now_us), events, 150_000)
    }
    pub fn frame_with_freshness(
        &mut self,
        raw: Option<Point>,
        pointer: Option<Point>,
        timing: (u64, u64),
        events: Vec<Event>,
        freshness_us: u64,
    ) -> (Option<Point>, Vec<Event>) {
        let (source_us, now_us) = timing;
        let fresh = source_us <= now_us && now_us - source_us < 150_000;
        let detected = raw.filter(|p| p.valid());
        let raw = detected.filter(|_| fresh);
        if let Some(cal) = &mut self.sampling {
            cal.expire_samples(now_us);
            if let Some(p) = raw {
                cal.sample(p, source_us, now_us);
            } else if detected.is_none() {
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
            if review.verified() && !self.review_saved {
                match profile_store::atomic_value(
                    &verified_path(&self.path),
                    &review.accepted().unwrap(),
                ) {
                    Ok(()) => {
                        self.review_saved = true;
                        self.message =
                            "Corners verified and saved; awaiting explicit acceptance".into();
                    }
                    Err(e) => self.message = format!("Cannot save verification: {e}"),
                }
            }
            return (target, Vec::new());
        }
        let target = if source_us <= now_us && now_us - source_us < freshness_us {
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
                    .filter(|e| {
                        matches!(
                            e,
                            Event::Cancel { .. }
                                | Event::Button { down: false }
                                | Event::NamedButton { down: false, .. }
                        )
                    })
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
    fn active_pointer_delivery_uses_selected_budget_without_loosening_sampling() {
        let base =
            std::env::temp_dir().join(format!("wiggle-active-budget-{}", std::process::id()));
        std::fs::create_dir_all(&base).unwrap();
        let path = base.join("profile.json");
        profile_store::adopt(
            &path,
            Profile {
                version: 1,
                workspace: Default::default(),
                output: [1920, 1080],
            },
        )
        .unwrap();
        let mut r = Runtime::new(&path, [1920, 1080]).unwrap();
        let point = Point { x: 0.5, y: 0.5 };
        let events = vec![Event::Button { down: true }];
        let (target, delivered) = r.frame_with_freshness(
            Some(point),
            Some(point),
            (1, 200_001),
            events.clone(),
            300_000,
        );
        assert!(target.is_some());
        assert_eq!(delivered, events);
        let (target, delivered) = r.frame(Some(point), Some(point), 1, 200_001, events);
        assert!(target.is_none());
        assert!(delivered.is_empty());
        std::fs::remove_dir_all(base).unwrap();
    }
    #[test]
    fn persisted_diagonal_coordinates_reconstruct_the_exact_verified_candidate() {
        for n in 1..1000 {
            let t = n as f64;
            let draft = Draft {
                version: 1,
                output: [1920, 1080],
                diagonal: true,
                bounds: [
                    Some(0.17 + t * 0.00011),
                    Some(0.77 + t * 0.00001),
                    Some(0.21 + t * 0.00013),
                    Some(0.87 + t * 0.00003),
                ],
            };
            let candidate = Calibration::from_diagonal_bounds(draft.bounds)
                .unwrap()
                .candidate(draft.output)
                .unwrap();
            let persisted_draft: Draft =
                serde_json::from_slice(&serde_json::to_vec(&draft).unwrap()).unwrap();
            let persisted_candidate: Profile =
                serde_json::from_slice(&serde_json::to_vec(&candidate).unwrap()).unwrap();
            let reconstructed = Calibration::from_diagonal_bounds(persisted_draft.bounds)
                .unwrap()
                .candidate(persisted_draft.output)
                .unwrap();
            assert_eq!(
                serde_json::to_value(reconstructed).unwrap(),
                serde_json::to_value(persisted_candidate).unwrap(),
                "fictional sample {n}"
            );
        }
    }
    #[test]
    fn diagonal_draft_resumes_second_pose_then_review_without_automatic_adoption() {
        let dir = std::env::temp_dir().join(format!("wiggle-diagonal-test-{}", std::process::id()));
        std::fs::create_dir(&dir).unwrap();
        let path = dir.join("profile.json");
        let mut r = Runtime::new(&path, [1920, 1080]).unwrap();
        r.control(Control::StartDiagonal).unwrap();
        assert_eq!(r.state()["stage"], "upper_left");
        for (i, point) in [Point { x: 0.2, y: 0.3 }, Point { x: 0.8, y: 0.7 }]
            .into_iter()
            .enumerate()
        {
            for n in 0..12 {
                let t = 1_000_000 + i as u64 * 2_000_000 + n * 100_000;
                let (_, events) = r.frame(
                    Some(point),
                    Some(point),
                    t,
                    t,
                    vec![Event::Button { down: true }],
                );
                assert!(events.is_empty());
            }
            r.control(Control::Next).unwrap();
            r = Runtime::new(&path, [1920, 1080]).unwrap();
            assert!(!path.exists());
            assert_eq!(r.state()["samples"], 0);
            assert!(r.control(Control::Accept).is_err());
            assert_eq!(
                r.state()["stage"],
                if i == 0 { "lower_right" } else { "review" }
            );
        }
        assert_eq!(r.state()["corners_verified"], false);
        r.control(Control::Cancel).unwrap();
        assert!(!draft_path(&path).exists());
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn confirmed_steps_resume_without_samples_or_adoption_and_stale_frames_do_not_erase_them() {
        let dir = std::env::temp_dir().join(format!("wiggle-draft-test-{}", std::process::id()));
        std::fs::create_dir(&dir).unwrap();
        let path = dir.join("profile.json");
        let mut runtime = Runtime::new(&path, [1920, 1080]).unwrap();
        runtime.control(Control::Start).unwrap();
        let p = Point { x: 0.2, y: 0.5 };
        for n in 0..12 {
            let at = 1_000_000 + n * 200_000;
            runtime.frame(Some(p), Some(p), at, at + 50_000, vec![]);
            runtime.frame(Some(p), Some(p), at + 10_000, at + 170_000, vec![]);
        }
        assert_eq!(runtime.state()["samples"], 12);
        runtime.control(Control::Next).unwrap();
        assert!(!path.exists());
        assert!(accept_saved(&path).is_err());
        drop(runtime);
        let mut resumed = Runtime::new(&path, [1920, 1080]).unwrap();
        assert_eq!(resumed.state()["stage"], "right");
        assert_eq!(resumed.state()["samples"], 0);
        assert!(resumed.control(Control::Next).is_err());
        assert!(resumed.control(Control::Accept).is_err());
        assert!(Runtime::new(&path, [1280, 720]).is_err());
        let at = 5_000_000;
        resumed.frame(Some(p), Some(p), at, at, vec![]);
        resumed.frame(None, None, at + 100_000, at + 100_000, vec![]);
        assert_eq!(resumed.state()["samples"], 0);
        resumed.control(Control::Cancel).unwrap();
        assert!(!draft_path(&path).exists());
        assert_eq!(
            Runtime::new(&path, [1920, 1080]).unwrap().state()["stage"],
            "uncalibrated"
        );
        std::fs::remove_dir_all(dir).unwrap();
    }
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
        runtime = Runtime::new(&path, [1920, 1080]).unwrap();
        assert_eq!(runtime.state()["stage"], "review");
        assert_eq!(runtime.state()["corners_verified"], false);
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
        assert!(verified_path(&path).exists());
        let original_draft: Draft = profile_store::load_value(&draft_path(&path)).unwrap();
        let changed = Draft {
            version: 1,
            output: original_draft.output,
            bounds: [Some(0.21), Some(0.8), Some(0.3), Some(0.7)],
            diagonal: false,
        };
        profile_store::atomic_value(&draft_path(&path), &changed).unwrap();
        assert!(accept_saved(&path).is_err());
        assert_eq!(profile_store::load(&path).unwrap().workspace.min.x, 0.0);
        profile_store::atomic_value(&draft_path(&path), &original_draft).unwrap();
        // Finite-session expiry does not adopt. Explicit camera-free acceptance
        // uses the exact completed candidate and removes its pending evidence.
        drop(runtime);
        assert_eq!(profile_store::load(&path).unwrap().workspace.min.x, 0.0);
        accept_saved(&path).unwrap();
        assert!(!verified_path(&path).exists());
        let mut runtime = Runtime::new(&path, [1920, 1080]).unwrap();
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
        let right_up = Event::NamedButton {
            button: crate::MouseButton::Right,
            down: false,
        };
        let (target, events) = runtime.frame(
            None,
            None,
            1,
            150_001,
            vec![
                Event::NamedButton {
                    button: crate::MouseButton::Right,
                    down: true,
                },
                right_up.clone(),
            ],
        );
        assert!(target.is_none());
        assert_eq!(events, vec![right_up]);
        runtime.control(Control::Start).unwrap();
        runtime.control(Control::Cancel).unwrap();
        assert_eq!(profile_store::load(&path).unwrap().workspace.min.x, 0.2);
        runtime.control(Control::Rollback).unwrap();
        assert_eq!(profile_store::load(&path).unwrap().workspace.min.x, 0.0);
        assert!(Runtime::new(&path, [1280, 720]).is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
