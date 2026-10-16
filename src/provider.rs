//! Raw landmark provider boundary: correspondence/age checks, then existing meaning.
//! Camera/device-free JSONL connection. Array order never supplies an owner ID.
use crate::{
    controller::{self, Controller},
    session::SessionConfig,
    touch_gestures,
    vision::{self, DetectedHand, MultiHands, XY},
};
use serde::Deserialize;
use std::{
    error::Error,
    io::{self, Write},
    path::Path,
};

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Landmarks {
    /// Source image pixels, before the existing mirrored workspace conversion.
    pub landmarks: [[f64; 2]; 21],
    /// Optional source-pixel image Z, scaled by image width, never metric world Z.
    #[serde(default)]
    pub image_depths: Option<[f64; 21]>,
    /// Same-frame measured presence, only from the explicitly selected adapter.
    #[serde(default)]
    pub presence_confidence: Option<f64>,
    pub world_landmarks: [[f64; 3]; 21],
    /// Right-class score in provider coordinates; not presence or participant identity.
    pub right_probability: f64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Frame {
    pub sequence: u64,
    /// Acquisition time and evaluation time must share one monotonic clock.
    pub source_us: u64,
    pub now_us: u64,
    pub width: u32,
    pub height: u32,
    pub hands: Vec<Landmarks>,
}

pub struct Tracker {
    hands: Vec<DetectedHand>,
    next_id: u64,
    watermark: Option<(u64, u64)>,
    last_now: Option<u64>,
    dimensions: Option<(u32, u32)>,
    stale_us: u64,
    pair_separation: f64,
    presence_floor: f64,
    presence_scores: bool,
    absence_retention_us: u64,
    last_visible: Option<u64>,
}
impl Tracker {
    pub fn new(
        stale_us: u64,
        pair_separation: f64,
        presence_floor: f64,
    ) -> Result<Self, &'static str> {
        if stale_us == 0
            || !pair_separation.is_finite()
            || !(0.8..=2.0).contains(&pair_separation)
            || !presence_floor.is_finite()
            || !(0.01..=1.0).contains(&presence_floor)
        {
            return Err("invalid landmark provider policy");
        }
        Ok(Self {
            hands: Vec::new(),
            next_id: 0,
            watermark: None,
            last_now: None,
            dimensions: None,
            stale_us,
            pair_separation,
            presence_floor,
            presence_scores: false,
            absence_retention_us: 0,
            last_visible: None,
        })
    }
    pub fn with_presence_scores(mut self) -> Self {
        self.presence_scores = true;
        self
    }
    pub fn clear(&mut self) {
        self.hands.clear();
        self.last_visible = None;
    }
    pub fn with_absence_retention(mut self, duration_us: u64) -> Result<Self, &'static str> {
        if duration_us < self.stale_us || duration_us > 2_000_000 {
            return Err("absence retention requires source freshness..2000000us");
        }
        self.absence_retention_us = duration_us;
        Ok(self)
    }
    fn retaining_absent_owner(&self, now: u64) -> bool {
        self.absence_retention_us > 0
            && self.hands.len() == 1
            && self
                .last_visible
                .is_some_and(|source| now >= source && now - source < self.absence_retention_us)
    }
    fn unknown(&mut self, reason: &'static str) -> MultiHands {
        self.clear();
        MultiHands {
            hands: Vec::new(),
            ambiguous: true,
            ambiguity_reason: Some(reason),
        }
    }
    pub fn tick(&mut self, now: u64) {
        if self.last_now.is_some_and(|n| now < n)
            || (self.absence_retention_us > 0
                && self
                    .last_visible
                    .is_some_and(|source| now.saturating_sub(source) >= self.absence_retention_us))
            || self
                .watermark
                .is_some_and(|(_, t)| now < t || now - t > self.stale_us)
                && !self.retaining_absent_owner(now)
        {
            self.clear();
        }
        self.last_now = Some(self.last_now.map_or(now, |n| n.max(now)));
    }
    pub fn observe(&mut self, frame: &Frame) -> Result<MultiHands, &'static str> {
        let clock_regressed = self.last_now.is_some_and(|n| frame.now_us < n);
        self.tick(frame.now_us);
        if clock_regressed
            || frame.source_us > frame.now_us
            || frame.now_us - frame.source_us > self.stale_us
        {
            return Ok(self.unknown("provider_age"));
        }
        if self
            .watermark
            .is_some_and(|(s, t)| frame.sequence <= s || frame.source_us <= t)
        {
            return Ok(self.unknown("provider_order"));
        }
        if self
            .watermark
            .is_some_and(|(_, t)| frame.source_us - t > self.stale_us)
            && !self.retaining_absent_owner(frame.now_us)
        {
            self.clear();
        }
        self.watermark = Some((frame.sequence, frame.source_us));
        if !(16..=4096).contains(&frame.width)
            || !(16..=4096).contains(&frame.height)
            || frame.hands.len() > 2
        {
            return Ok(self.unknown("provider_shape"));
        }
        let dimensions = (frame.width, frame.height);
        let changed = self.dimensions.is_some_and(|d| d != dimensions);
        self.dimensions = Some(dimensions);
        if changed {
            return Ok(self.unknown("provider_dimensions"));
        }
        let mut next = Vec::new();
        for raw in &frame.hands {
            if raw.presence_confidence.is_some_and(|score| {
                !self.presence_scores
                    || !score.is_finite()
                    || !(self.presence_floor..=1.0).contains(&score)
            }) || !raw.right_probability.is_finite()
                || !(0.0..=1.0).contains(&raw.right_probability)
                || raw.landmarks.iter().any(|p| {
                    !p[0].is_finite()
                        || !p[1].is_finite()
                        || !(-(frame.width as f64)..=2.0 * frame.width as f64).contains(&p[0])
                        || !(-(frame.height as f64)..=2.0 * frame.height as f64).contains(&p[1])
                })
                || raw.image_depths.as_ref().is_some_and(|depths| {
                    depths
                        .iter()
                        .any(|z| !z.is_finite() || z.abs() > 2.0 * f64::from(frame.width))
                })
                || raw
                    .world_landmarks
                    .iter()
                    .flatten()
                    .any(|v| !v.is_finite() || v.abs() > 2.0)
            {
                return Ok(self.unknown("provider_landmarks"));
            }
            let hand = DetectedHand {
                id: 0,
                landmarks: raw.landmarks.map(|p| XY { x: p[0], y: p[1] }),
                image_depths: raw.image_depths,
                world_landmarks: raw.world_landmarks,
                confidence: raw.presence_confidence.unwrap_or(self.presence_floor),
                handedness: raw.right_probability,
                tracked: false,
                secondary: None,
                secondary_source_id: 0,
            };
            if hand.landmarks[5].distance(hand.landmarks[17]) < 8.0
                || hand.landmarks[0].distance(hand.landmarks[9]) < 8.0
                || hand.world_pinch_distance_m() > 1.0
            {
                return Ok(self.unknown("provider_geometry"));
            }
            next.push(hand);
        }
        if next.len() == 2
            && (vision::duplicate_prediction(&next[0], &next[1])
                || vision::pair_uncertain(&next[0], &next[1], self.pair_separation))
        {
            return Ok(self.unknown("provider_overlap"));
        }
        // At most two hands: enumerate injective partial assignments, maximizing
        // admitted matches first, then minimizing corresponding-palm RMS cost.
        let costs: Vec<Vec<Option<f64>>> = next
            .iter()
            .map(|new| {
                self.hands
                    .iter()
                    .map(|old| {
                        let palm = [0, 5, 9, 13, 17, 1, 2].map(|i| new.landmarks[i]);
                        let scale = old.landmarks[0]
                            .distance(old.landmarks[9])
                            .max(old.landmarks[5].distance(old.landmarks[17]));
                        vision::palm_match_cost(old, &palm).filter(|cost| {
                            *cost < 0.75 && !vision::identity_uncertain(old, new, 2.0 * scale)
                        })
                    })
                    .collect()
            })
            .collect();
        let choices: Vec<_> = std::iter::once(None)
            .chain((0..self.hands.len()).map(Some))
            .collect();
        let mut assignments = Vec::new();
        for first in &choices {
            for second in &choices {
                let selected = [*first, *second];
                if next.len() < 2 && second.is_some() {
                    continue;
                }
                if next.is_empty() && first.is_some() {
                    continue;
                }
                if next.len() == 2 && first.is_some() && first == second {
                    continue;
                }
                let mut count = 0;
                let mut cost = 0.0;
                let mut valid = true;
                for (i, owner) in selected.iter().enumerate().take(next.len()) {
                    if let Some(j) = owner {
                        if let Some(c) = costs[i][*j] {
                            count += 1;
                            cost += c;
                        } else {
                            valid = false;
                        }
                    }
                }
                if valid {
                    assignments.push((count, cost, selected));
                }
            }
        }
        assignments.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.total_cmp(&b.1)));
        let best = assignments.first().ok_or("missing provider assignment")?;
        if assignments
            .get(1)
            .is_some_and(|other| best.0 > 0 && other.0 == best.0 && other.1 - best.1 < 0.25)
        {
            return Ok(self.unknown("provider_assignment"));
        }
        for (i, hand) in next.iter_mut().enumerate() {
            if let Some(j) = best.2[i] {
                hand.id = self.hands[j].id;
                hand.tracked = true;
            } else {
                self.next_id = self
                    .next_id
                    .checked_add(1)
                    .ok_or("provider hand identity exhausted")?;
                hand.id = self.next_id;
            }
        }
        if !next.is_empty() || !self.retaining_absent_owner(frame.now_us) {
            self.hands = next.clone();
            self.last_visible = (!next.is_empty()).then_some(frame.source_us);
        }
        Ok(MultiHands {
            hands: next,
            ambiguous: false,
            ambiguity_reason: None,
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
enum Command {
    Start,
    Stop,
    Tick { now_us: u64 },
    Mode { mode: controller::Mode },
    SelectHand { hand_id: Option<u64> },
    Frame { observation: Frame },
}

/// Intent only. Uses the same bounded JSONL reader as the native device writers.
pub(crate) fn configured(
    session: &Path,
    settings: &Path,
    presence_floor: Option<f64>,
) -> Result<(SessionConfig, Controller, Tracker), Box<dyn Error>> {
    let config = SessionConfig::load(session)?;
    let presence_floor = presence_floor
        .or(config.gesture_min_confidence)
        .ok_or("provider requires an explicit gesture_min_confidence / SDK presence floor")?;
    if config.dual_crop
        || config.secondary_model.is_some()
        || (config.gestures().min_confidence - presence_floor).abs() > f64::EPSILON
    {
        return Err("provider replay requires single estimate and explicit matching presence floor/gesture gate".into());
    }
    let mut tracker = Tracker::new(
        config.gestures().stale_us,
        config.pair_separation_widths,
        presence_floor,
    )?;
    if config.provider_presence_scores {
        tracker = tracker.with_presence_scores();
    }
    if config.pointer_drag_grace_ms > 0 {
        tracker = tracker.with_absence_retention(config.gestures().drag_grace_us)?;
    }
    let controller: Controller = crate::session::controller_config(&config, settings)?;
    if controller.visual_policy().is_some_and(|policy| {
        !config.provider_presence_scores && presence_floor < policy.min_confidence
    }) {
        return Err(
            "provider presence floor must meet the visual macro confidence threshold".into(),
        );
    }
    Ok((config, controller, tracker))
}
pub fn replay(session: &Path, settings: &Path, presence_floor: f64) -> Result<(), Box<dyn Error>> {
    let (config, mut controller, mut tracker) =
        configured(session, settings, Some(presence_floor))?;
    let mut visual_classifier =
        crate::pose_classifier::VisualClassifier::with_runtime(&controller, &config.runtime)?;
    let mut out = io::stdout().lock();
    let publish = |out: &mut std::io::StdoutLock<'_>, value: serde_json::Value| -> io::Result<()> {
        writeln!(out, "{value}")?;
        out.flush()
    };
    for command in crate::intent::reader::<Command>() {
        let mut value = serde_json::json!({"schema":"wigglecontrol.provider-output/v1",
            "camera_opened":false,"system_input":false,"local_semantics_only":true,
            "confidence_kind":if config.provider_presence_scores {"presence_score_or_configured_floor"} else {"configured_presence_threshold_lower_bound"},"presence_floor":presence_floor});
        let state = match command {
            Err(error) => {
                value["controller"] = serde_json::to_value(controller.stop())?;
                publish(&mut out, value)?;
                return Err(error.into());
            }
            Ok(Command::Start) => {
                tracker.clear();
                controller.start()
            }
            Ok(Command::Stop) => {
                tracker.clear();
                controller.stop()
            }
            Ok(Command::Tick { now_us }) => {
                tracker.tick(now_us);
                controller.command(controller::Command::Tick { now_us })
            }
            Ok(Command::Mode { mode }) => controller.command(controller::Command::Mode { mode }),
            Ok(Command::SelectHand { hand_id }) => {
                controller.command(controller::Command::SelectHand { hand_id })
            }
            Ok(Command::Frame { observation: frame }) => {
                let found = match tracker.observe(&frame) {
                    Ok(found) => found,
                    Err(error) => {
                        value["controller"] = serde_json::to_value(controller.stop())?;
                        publish(&mut out, value)?;
                        return Err(error.into());
                    }
                };
                let hands: Vec<_> = found
                    .hands
                    .iter()
                    .filter_map(|h| {
                        config
                            .observation(Some(h), frame.width, frame.height, frame.source_us)
                            .hand
                    })
                    .collect();
                value["sequence"] = frame.sequence.into();
                value["source_us"] = frame.source_us.into();
                value["now_us"] = frame.now_us.into();
                value["width"] = frame.width.into();
                value["height"] = frame.height.into();
                value["ambiguous"] = found.ambiguous.into();
                value["ambiguity_reason"] = found.ambiguity_reason.into();
                value["perception"] = serde_json::json!(found
                    .hands
                    .iter()
                    .zip(frame.hands.iter())
                    .map(|(h, raw)| serde_json::json!({
                    "presence_confidence":raw.presence_confidence,
                    "id":h.id,"tracked":h.tracked,"handedness":h.handedness,
                    "confidence":h.confidence,"landmarks":h.landmarks.map(|p|[p.x,p.y]),
                    "world_landmarks":h.world_landmarks,"image_depths":h.image_depths}))
                    .collect::<Vec<_>>());
                value["hands"] = serde_json::to_value(&hands)?;
                let observation = touch_gestures::Frame {
                    version: 1,
                    source_us: frame.source_us,
                    ambiguous: found.ambiguous,
                    hands,
                };
                let classifier_started = std::time::Instant::now();
                let (pose, visual_state) = if let Some(classifier) = &mut visual_classifier {
                    match classifier.classify(
                        &controller,
                        &observation,
                        &found.hands,
                        frame.width,
                        frame.height,
                    ) {
                        Ok(result) => result,
                        Err(error) => {
                            value["controller"] = serde_json::to_value(controller.stop())?;
                            publish(&mut out, value)?;
                            return Err(error);
                        }
                    }
                } else {
                    (None, None)
                };
                value["visual_pose"] = serde_json::to_value(visual_state)?;
                value["visual_macros"] = visual_classifier.is_some().into();
                value["classifier_ms"] =
                    (classifier_started.elapsed().as_secs_f64() * 1000.0).into();
                controller.observe(frame.now_us, observation, pose)
            }
        };
        value["controller"] = serde_json::to_value(state)?;
        publish(&mut out, value)?;
    }
    publish(
        &mut out,
        serde_json::json!({"schema":"wigglecontrol.provider-output/v1",
        "camera_opened":false,"system_input":false,"local_semantics_only":true,
        "eof":true,"controller":controller.stop()}),
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn hand(shift: f64, gap: f64) -> Landmarks {
        let mut points = [[100.0, 200.0]; 21];
        for (i, p) in [
            (0, [100.0, 300.0]),
            (4, [20.0, 120.0]),
            (5, [75.0, 200.0]),
            (6, [75.0, 170.0]),
            (8, [75.0, 100.0]),
            (9, [100.0, 185.0]),
            (10, [100.0, 150.0]),
            (12, [100.0, 80.0]),
            (13, [115.0, 195.0]),
            (14, [110.0, 170.0]),
            (16, [110.0, 110.0]),
            (17, [125.0, 200.0]),
            (18, [125.0, 180.0]),
            (20, [125.0, 140.0]),
        ] {
            points[i] = p;
        }
        for p in &mut points {
            p[0] += shift;
        }
        let mut world = [[0.0; 3]; 21];
        world[4][0] = gap;
        Landmarks {
            landmarks: points,
            image_depths: None,
            presence_confidence: None,
            world_landmarks: world,
            right_probability: 0.9,
        }
    }
    fn frame(sequence: u64, hands: Vec<Landmarks>) -> Frame {
        Frame {
            sequence,
            source_us: (sequence - 1) * 100_000 + 1,
            now_us: (sequence - 1) * 100_000 + 1,
            width: 640,
            height: 480,
            hands,
        }
    }
    fn tracker() -> Tracker {
        Tracker::new(300_000, 1.2, 0.5).unwrap()
    }
    #[test]
    fn optional_one_hand_absence_retention_preserves_only_matched_unexpired_owners() {
        let mut t = tracker().with_absence_retention(1_000_000).unwrap();
        let original = t.observe(&frame(1, vec![hand(0.0, 0.01)])).unwrap().hands[0].id;
        for i in 2..=8 {
            let absent = t.observe(&frame(i, vec![])).unwrap();
            assert!(absent.hands.is_empty() && !absent.ambiguous);
        }
        let returned = t.observe(&frame(9, vec![hand(5.0, 0.01)])).unwrap();
        assert_eq!(returned.hands[0].id, original);
        assert!(returned.hands[0].tracked);
        for i in 10..=18 {
            assert!(t.observe(&frame(i, vec![])).unwrap().hands.is_empty());
        }
        // Recent empty reports do not refresh last actual visible-hand evidence.
        let expired = t.observe(&frame(19, vec![hand(5.0, 0.01)])).unwrap();
        assert_ne!(expired.hands[0].id, original);
        let current = expired.hands[0].id;
        t.observe(&frame(20, vec![])).unwrap();
        let mut different = hand(5.0, 0.01);
        different.right_probability = 0.01;
        let changed = t.observe(&frame(21, vec![different])).unwrap();
        assert_ne!(changed.hands[0].id, current);

        let mut t = tracker().with_absence_retention(1_000_000).unwrap();
        let first = t.observe(&frame(1, vec![hand(0.0, 0.01)])).unwrap().hands[0].id;
        t.tick(600_001);
        let returned = t.observe(&frame(8, vec![hand(0.0, 0.01)])).unwrap();
        assert_eq!(returned.hands[0].id, first);
        t.tick(1_700_001);
        assert_ne!(
            t.observe(&frame(19, vec![hand(0.0, 0.01)])).unwrap().hands[0].id,
            first
        );
    }
    #[test]
    fn optional_depth_is_not_fabricated_and_invalid_depth_retires_the_owner() {
        let mut t = tracker();
        let legacy = t.observe(&frame(1, vec![hand(0.0, 0.09)])).unwrap();
        assert!(legacy.hands[0].image_depths.is_none());
        let mut raw = hand(0.0, 0.09);
        raw.image_depths = Some(std::array::from_fn(|i| -(i as f64) * 2.0));
        let complete = t.observe(&frame(2, vec![raw.clone()])).unwrap();
        assert_eq!(complete.hands[0].id, legacy.hands[0].id);
        assert_eq!(complete.hands[0].image_depths, raw.image_depths);
        for (sequence, invalid) in [(3, f64::NAN), (4, f64::INFINITY), (5, 1280.01)] {
            raw.image_depths.as_mut().unwrap()[8] = invalid;
            assert_eq!(
                t.observe(&frame(sequence, vec![raw.clone()]))
                    .unwrap()
                    .ambiguity_reason,
                Some("provider_landmarks")
            );
        }
        let returned = t.observe(&frame(6, vec![hand(0.0, 0.09)])).unwrap();
        assert_ne!(returned.hands[0].id, legacy.hands[0].id);
    }

    #[test]
    fn measured_presence_requires_selection_stays_with_its_hand_and_missing_never_upgrades() {
        let mut a = hand(0.0, 0.09);
        a.presence_confidence = Some(0.93);
        let mut disabled = tracker();
        assert_eq!(
            disabled
                .observe(&frame(1, vec![a.clone()]))
                .unwrap()
                .ambiguity_reason,
            Some("provider_landmarks")
        );
        let mut b = hand(240.0, 0.09);
        b.presence_confidence = Some(0.77);
        let mut t = tracker().with_presence_scores();
        let first = t.observe(&frame(1, vec![a.clone(), b.clone()])).unwrap();
        let reordered = t.observe(&frame(2, vec![b, a.clone()])).unwrap();
        assert_eq!(
            reordered
                .hands
                .iter()
                .map(|h| (h.id, h.confidence))
                .collect::<Vec<_>>(),
            [(first.hands[1].id, 0.77), (first.hands[0].id, 0.93)]
        );
        a.presence_confidence = None;
        let missing = t.observe(&frame(3, vec![a.clone()])).unwrap();
        assert_eq!(missing.hands[0].id, first.hands[0].id);
        assert_eq!(missing.hands[0].confidence, 0.5);
        for (sequence, invalid) in [(4, f64::NAN), (5, 0.49), (6, 1.01)] {
            a.presence_confidence = Some(invalid);
            assert_eq!(
                t.observe(&frame(sequence, vec![a.clone()]))
                    .unwrap()
                    .ambiguity_reason,
                Some("provider_landmarks")
            );
        }
    }

    #[test]
    fn visual_profile_requires_matching_real_sdk_floor_without_loading_models() {
        let root = std::env::temp_dir().join(format!(
            "wiggle-provider-visual-policy-{}",
            std::process::id()
        ));
        std::fs::create_dir(&root).unwrap();
        let session = root.join("session.json");
        let settings = root.join("settings.json");
        for floor in [0.5, 0.8] {
            std::fs::write(
                &session,
                serde_json::json!({
                    "camera":"/not-acquired","models":"/not-loaded","runtime":"/not-loaded",
                    "fps":10,"gain":2000,"gesture_min_confidence":floor
                })
                .to_string(),
            )
            .unwrap();
            let config = SessionConfig::load(&session).unwrap();
            std::fs::write(&settings, serde_json::json!({
                "gestures":config.gestures(),
                "macros":{"hold_us":400000,"neutral_us":200000,"cooldown_us":500000,
                    "stale_us":300000,"bindings":[{"pose":"victory","action":"toggle_precision"}]},
                "visual_macros":{"model":"/not-loaded","neutral_pose":"open_palm","min_confidence":0.8}
            }).to_string()).unwrap();
            let result = configured(&session, &settings, None);
            if floor < 0.8 {
                assert!(result.err().unwrap().to_string().contains("must meet"));
                let mut measured: serde_json::Value =
                    serde_json::from_slice(&std::fs::read(&session).unwrap()).unwrap();
                measured["provider_presence_scores"] = true.into();
                std::fs::write(&session, measured.to_string()).unwrap();
                let (_, controller, tracker) = configured(&session, &settings, None).unwrap();
                assert!(tracker.presence_scores);
                assert_eq!(tracker.presence_floor, 0.5);
                assert_eq!(controller.visual_policy().unwrap().min_confidence, 0.8);
            } else {
                let (_, controller, tracker) = result.unwrap();
                assert!(controller.visual_policy().is_some());
                assert_eq!(tracker.presence_floor, 0.8);
                assert!(configured(&session, &settings, Some(0.5))
                    .err()
                    .unwrap()
                    .to_string()
                    .contains("matching presence floor"));
            }
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn reordered_same_handedness_hands_keep_spatial_owners_not_array_indices() {
        let mut t = tracker();
        let first = t
            .observe(&frame(1, vec![hand(0.0, 0.09), hand(240.0, 0.09)]))
            .unwrap();
        let next = t
            .observe(&frame(2, vec![hand(243.0, 0.02), hand(3.0, 0.09)]))
            .unwrap();
        assert_eq!(first.hands.iter().map(|h| h.id).collect::<Vec<_>>(), [1, 2]);
        assert_eq!(next.hands.iter().map(|h| h.id).collect::<Vec<_>>(), [2, 1]);
        assert!(next.hands.iter().all(|h| h.tracked));
        assert!(next.hands.iter().all(|h| h.confidence == 0.5));
    }
    #[test]
    fn ambiguous_assignment_overlap_and_missing_frames_retire_old_ids() {
        let mut t = tracker();
        t.observe(&frame(1, vec![hand(0.0, 0.09), hand(150.0, 0.09)]))
            .unwrap();
        let ambiguous = t.observe(&frame(2, vec![hand(75.0, 0.02)])).unwrap();
        assert_eq!(ambiguous.ambiguity_reason, Some("provider_assignment"));
        assert!(ambiguous.hands.is_empty());
        assert_eq!(
            t.observe(&frame(3, vec![hand(0.0, 0.02)])).unwrap().hands[0].id,
            3
        );
        assert_eq!(
            t.observe(&frame(4, vec![hand(0.0, 0.02), hand(20.0, 0.02)]))
                .unwrap()
                .ambiguity_reason,
            Some("provider_overlap")
        );
        assert_eq!(
            t.observe(&frame(5, vec![hand(0.0, 0.02)])).unwrap().hands[0].id,
            4
        );
        assert!(t.observe(&frame(6, vec![])).unwrap().hands.is_empty());
        assert_eq!(
            t.observe(&frame(7, vec![hand(0.0, 0.02)])).unwrap().hands[0].id,
            5
        );
    }
    #[test]
    fn watchdog_tick_and_strong_handedness_change_do_not_resurrect_an_owner() {
        let mut t = tracker();
        assert_eq!(
            t.observe(&frame(1, vec![hand(0.0, 0.09)])).unwrap().hands[0].id,
            1
        );
        t.tick(300_002);
        assert_eq!(
            t.observe(&frame(5, vec![hand(0.0, 0.02)])).unwrap().hands[0].id,
            2
        );
        let mut swapped = hand(0.0, 0.02);
        swapped.right_probability = 0.1;
        assert_eq!(t.observe(&frame(6, vec![swapped])).unwrap().hands[0].id, 3);
    }
    #[test]
    fn stale_future_regressing_packets_dimensions_and_shape_cannot_keep_an_owner() {
        let mut t = tracker();
        t.observe(&frame(1, vec![hand(0.0, 0.09)])).unwrap();
        let mut stale = frame(2, vec![hand(0.0, 0.02)]);
        stale.now_us = 500_001;
        assert_eq!(
            t.observe(&stale).unwrap().ambiguity_reason,
            Some("provider_age")
        );
        let mut current = frame(7, vec![hand(0.0, 0.02)]);
        assert_eq!(t.observe(&current).unwrap().hands[0].id, 2);
        assert_eq!(
            t.observe(&current).unwrap().ambiguity_reason,
            Some("provider_order")
        );
        current.sequence = 8;
        current.source_us = 700_001;
        current.now_us = 600_002;
        assert_eq!(
            t.observe(&current).unwrap().ambiguity_reason,
            Some("provider_age")
        );
        current.now_us = 700_001;
        current.width = 1280;
        assert_eq!(
            t.observe(&current).unwrap().ambiguity_reason,
            Some("provider_dimensions")
        );
        let mut changed = frame(9, vec![hand(0.0, 0.02)]);
        changed.width = 1280;
        assert_eq!(t.observe(&changed).unwrap().hands[0].id, 3);
        assert_eq!(
            t.observe(&frame(10, vec![hand(0.0, 0.02); 3]))
                .unwrap()
                .ambiguity_reason,
            Some("provider_shape")
        );
        let mut malformed = frame(11, vec![hand(0.0, 0.02)]);
        malformed.width = 1280;
        malformed.hands[0].world_landmarks[4][0] = f64::NAN;
        assert_eq!(
            t.observe(&malformed).unwrap().ambiguity_reason,
            Some("provider_landmarks")
        );
    }
    #[test]
    fn actual_controller_releases_on_loss_and_requires_new_open_after_retirement() {
        let config: SessionConfig = serde_json::from_value(serde_json::json!({
            "camera":"/not-acquired","models":"/unused","runtime":"/unused","fps":10,"gain":2000,
            "freshness_ms":300,"pinch_3d":true,"pinch_close_m":0.045,"pinch_open_m":0.065,
            "gesture_min_confidence":0.5
        }))
        .unwrap();
        let mut controller = Controller::new(controller::Settings {
            initial_mode: controller::Mode::TwoContacts,
            gestures: config.gestures(),
            workspace: config.touch_workspace,
            ..Default::default()
        })
        .unwrap();
        controller.start();
        let mut t = tracker();
        let mut observe = |f: Frame| {
            let found = t.observe(&f).unwrap();
            let hands = found
                .hands
                .iter()
                .filter_map(|h| {
                    config
                        .observation(Some(h), f.width, f.height, f.source_us)
                        .hand
                })
                .collect();
            controller.observe(
                f.now_us,
                touch_gestures::Frame {
                    version: 1,
                    source_us: f.source_us,
                    ambiguous: found.ambiguous,
                    hands,
                },
                None,
            )
        };
        assert!(observe(frame(1, vec![hand(0.0, 0.09)])).contacts.is_empty());
        observe(frame(2, vec![hand(0.0, 0.02)]));
        assert_eq!(observe(frame(3, vec![hand(0.0, 0.02)])).contacts.len(), 1);
        assert!(observe(frame(4, vec![])).contacts.is_empty());
        assert!(observe(frame(5, vec![hand(0.0, 0.02)])).contacts.is_empty());
        assert!(observe(frame(6, vec![hand(0.0, 0.02)])).contacts.is_empty());
        observe(frame(7, vec![hand(0.0, 0.09)]));
        observe(frame(8, vec![hand(0.0, 0.02)]));
        assert_eq!(observe(frame(9, vec![hand(0.0, 0.02)])).contacts.len(), 1);
        assert!(controller.stop().contacts.is_empty());
    }
}
