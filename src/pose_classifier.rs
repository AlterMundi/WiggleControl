// SPDX-License-Identifier: Apache-2.0
// Portions derived from MediaPipe. Copyright 2025 The MediaPipe Authors.
//! Opt-in pretrained pose inference over existing hand landmarks; no sensors/actions.
//! Preprocessing follows the Apache-2.0 MediaPipe LandmarksToMatrix calculator at
//! 7dd8fe00c17e7732818ea1406c4fb64beb237e6c, for full images with rotation zero.
//! See tools/synthetic/export_gesture_classifier.py for exact model provenance.
use ort::{
    session::{builder::GraphOptimizationLevel, Session},
    value::Tensor,
};
use serde::{Deserialize, Serialize};
use std::{error::Error, io::Write, path::Path};

pub const LABELS: [&str; 8] = [
    "None",
    "Closed_Fist",
    "Open_Palm",
    "Pointing_Up",
    "Thumb_Down",
    "Thumb_Up",
    "Victory",
    "ILoveYou",
];

#[derive(Debug, Serialize)]
pub struct Prediction {
    pub category: &'static str,
    pub confidence: f32,
    pub probabilities: [f32; 8],
}
impl Prediction {
    pub fn pose(&self) -> Option<crate::macros::Pose> {
        use crate::macros::Pose;
        match self.category {
            "Closed_Fist" => Some(Pose::ClosedFist),
            "Open_Palm" => Some(Pose::OpenPalm),
            "Pointing_Up" => Some(Pose::PointingUp),
            "Thumb_Down" => Some(Pose::ThumbsDown),
            "Thumb_Up" => Some(Pose::ThumbsUp),
            "Victory" => Some(Pose::Victory),
            _ => None, // Unknown and unsupported ILoveYou are not neutral.
        }
    }
}

#[derive(Debug, Serialize)]
pub struct VisualState {
    pub hand_id: u64,
    pub prediction: Prediction,
    pub pose_evidence_available: bool,
}

/// Shared native/offline adapter: same ownership, preprocessing and policy.
pub struct VisualClassifier {
    classifier: Classifier,
    policy: crate::macros::VisualPolicy,
}
impl VisualClassifier {
    /// Provider paths have no native detector to initialize ORT. Disabled profiles
    /// must remain model/runtime-free, including when their paths do not exist.
    pub fn with_runtime(
        controller: &crate::controller::Controller,
        runtime: &str,
    ) -> Result<Option<Self>, Box<dyn Error>> {
        if controller.visual_policy().is_none() {
            return Ok(None);
        }
        ort::init_from(runtime).commit()?;
        Self::configured(controller)
    }

    pub fn configured(
        controller: &crate::controller::Controller,
    ) -> Result<Option<Self>, Box<dyn Error>> {
        controller
            .visual_policy()
            .map(|policy| {
                Ok(Self {
                    classifier: Classifier::new(Path::new(&policy.model))?,
                    policy: policy.clone(),
                })
            })
            .transpose()
    }
    pub fn classify(
        &mut self,
        controller: &crate::controller::Controller,
        observation: &crate::touch_gestures::Frame,
        detected: &[crate::landmarks::DetectedHand],
        width: u32,
        height: u32,
    ) -> Result<(Option<crate::macros::HandPose>, Option<VisualState>), Box<dyn Error>> {
        let Some(hand) = controller
            .macro_hand(observation)
            .and_then(|id| detected.iter().find(|h| h.id == id))
        else {
            return Ok((None, None));
        };
        if hand.image_depths.is_none() {
            return Ok((None, None));
        }
        let prediction = self.classifier.predict_hand(hand, width, height)?;
        let evidence = self.policy.evidence(
            hand.id,
            f64::from(prediction.confidence).min(hand.confidence),
            prediction.pose(),
        );
        let state = VisualState {
            hand_id: hand.id,
            pose_evidence_available: evidence.is_some(),
            prediction,
        };
        Ok((evidence, Some(state)))
    }
}

/// Image XYZ in the provider's original, unmirrored normalized coordinates.
/// Unknown classes are predictions, never positively observed neutral evidence.
pub fn preprocess(
    points: &[[f32; 3]; 21],
    width: u32,
    height: u32,
) -> Result<Vec<f32>, &'static str> {
    if width == 0 || height == 0 || points.iter().flatten().any(|v| !v.is_finite()) {
        return Err("pose preprocessing requires finite image XYZ and positive dimensions");
    }
    let max = width.max(height) as f32;
    let mut points = *points;
    for point in &mut points {
        point[0] = (point[0] - 0.5) * (width as f32 / max) + 0.5;
        point[1] = (point[1] - 0.5) * (height as f32 / max) + 0.5;
    }
    let origin = points[0];
    let mut lo = [f32::MAX; 2];
    // Preserve the pinned upstream calculator's positive numeric_limits::min().
    let mut hi = [f32::MIN_POSITIVE; 2];
    for point in &mut points {
        for axis in 0..3 {
            point[axis] -= origin[axis];
            if axis < 2 {
                lo[axis] = lo[axis].min(point[axis]);
                hi[axis] = hi[axis].max(point[axis]);
            }
        }
    }
    let scale = (hi[0] - lo[0]).max(hi[1] - lo[1]) + 1e-5;
    let result: Vec<f32> = points.iter().flatten().map(|value| value / scale).collect();
    if result.iter().any(|v| !v.is_finite()) {
        return Err("pose preprocessing overflow");
    }
    Ok(result)
}

pub struct Classifier {
    model: Session,
}
impl Classifier {
    pub fn new(model: &Path) -> Result<Self, Box<dyn Error>> {
        if std::fs::metadata(model)?.len() > 2 * 1024 * 1024 {
            return Err("gesture model exceeds2MiB".into());
        }
        let model = Session::builder()?
            .with_intra_threads(1)?
            .with_optimization_level(GraphOptimizationLevel::Level3)?
            .commit_from_file(model)?;
        if model.inputs.len() != 1
            || model.inputs[0].name != "hand"
            || model.outputs.len() != 2
            || model.outputs[0].name != "probabilities"
            || model.outputs[1].name != "embedding"
        {
            return Err("unexpected exported gesture model contract".into());
        }
        Ok(Self { model })
    }
    pub fn predict(
        &mut self,
        points: &[[f32; 3]; 21],
        width: u32,
        height: u32,
    ) -> Result<Prediction, Box<dyn Error>> {
        let input = Tensor::from_array(([1usize, 21, 3], preprocess(points, width, height)?))?;
        let results = self.model.run(ort::inputs![input])?;
        let (shape, scores) = results[0].try_extract_tensor::<f32>()?;
        if shape.as_ref() != [1, 8]
            || scores.len() != 8
            || scores
                .iter()
                .any(|s| !s.is_finite() || !(0.0..=1.0).contains(s))
            || (scores.iter().sum::<f32>() - 1.0).abs() > 1e-4
        {
            return Err("unexpected gesture probability output".into());
        }
        // Match the provider's first-index argmax rule, including exact ties.
        let best = (1..8).fold(0, |best, i| if scores[i] > scores[best] { i } else { best });
        Ok(Prediction {
            category: LABELS[best],
            confidence: scores[best],
            probabilities: scores.try_into()?,
        })
    }
    pub fn predict_hand(
        &mut self,
        hand: &crate::landmarks::DetectedHand,
        width: u32,
        height: u32,
    ) -> Result<Prediction, Box<dyn Error>> {
        let depths = hand
            .image_depths
            .as_ref()
            .ok_or("visual classification requires image depth")?;
        let points = std::array::from_fn(|i| {
            [
                (hand.landmarks[i].x / f64::from(width)) as f32,
                (hand.landmarks[i].y / f64::from(height)) as f32,
                (depths[i] / f64::from(width)) as f32,
            ]
        });
        self.predict(&points, width, height)
    }
}

/// Bounded, camera-free provider-landmark comparison using the real Rust backend.
pub fn replay(model: &Path, runtime: &str) -> Result<(), Box<dyn Error>> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Sample {
        width: u32,
        height: u32,
        landmarks: [[f32; 3]; 21],
    }
    ort::init_from(runtime).commit()?;
    let mut classifier = Classifier::new(model)?;
    let mut output = std::io::stdout().lock();
    for sample in crate::intent::reader::<Sample>() {
        let sample = sample?;
        let prediction = classifier.predict(&sample.landmarks, sample.width, sample.height)?;
        writeln!(
            output,
            "{}",
            serde_json::json!({
                "schema":"wigglecontrol.gesture-prediction/v1", "prediction":prediction,
                "camera_opened":false,"system_input":false,"actions_executed":false
            })
        )?;
        output.flush()?;
    }
    Ok(())
}

#[cfg(feature = "native-vision")]
pub fn check_image(image: &Path, config: &Path, model: &Path) -> Result<(), Box<dyn Error>> {
    let config = crate::session::SessionConfig::load(config)?;
    ort::init_from(&config.runtime).commit()?;
    let mut reader = image::ImageReader::open(image)?.with_guessed_format()?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(4096);
    limits.max_image_height = Some(4096);
    reader.limits(limits);
    let image = reader.decode()?.to_rgb8();
    let mut perception = config.perception()?;
    let mut classifier = Classifier::new(model)?;
    let found = perception.infer_many(&image)?;
    let mut hands = Vec::new();
    for hand in found.hands {
        let started = std::time::Instant::now();
        let prediction = classifier.predict_hand(&hand, image.width(), image.height())?;
        hands.push(
            serde_json::json!({"id":hand.id,"hand_confidence":hand.confidence,
            "world_pinch_distance_m":hand.world_pinch_distance_m(),"prediction":prediction,
            "classifier_ms":started.elapsed().as_secs_f64()*1000.0}),
        );
    }
    println!(
        "{}",
        serde_json::json!({"schema":"wigglecontrol.gesture-image/v1",
        "hands":hands,"ambiguous":found.ambiguous,"ambiguity_reason":found.ambiguity_reason,
        "camera_opened":false,"system_input":false,"actions_executed":false})
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn disabled_visual_profile_does_not_load_an_invalid_runtime() {
        let controller = crate::controller::Controller::new(Default::default()).unwrap();
        assert!(VisualClassifier::with_runtime(&controller, "/not-loaded")
            .unwrap()
            .is_none());
    }
    #[test]
    fn image_aspect_and_depth_are_normalized_in_the_provider_coordinate_system() {
        let mut points = [[0.5, 0.5, 0.0]; 21];
        points[1] = [0.75, 0.5, 0.25];
        points[2] = [0.5, 0.75, 0.0];
        let normalized = preprocess(&points, 640, 480).unwrap();
        let scale = 0.25 + 1e-5;
        assert_eq!(&normalized[..3], &[0.0, 0.0, 0.0]);
        assert!((normalized[3] - 0.25 / scale).abs() < 1e-6);
        assert!((normalized[5] - 0.25 / scale).abs() < 1e-6);
        assert!((normalized[7] - 0.1875 / scale).abs() < 1e-6);
        assert!(preprocess(&points, 0, 480).is_err());
        points[1][2] = f32::NAN;
        assert!(preprocess(&points, 640, 480).is_err());
    }
}
