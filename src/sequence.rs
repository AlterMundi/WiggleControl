//! Camera-free image sequence replay through perception and direct-touch meaning.
use crate::{
    session::SessionConfig,
    touch_gestures::{Frame, Gestures},
};
use std::{
    error::Error,
    fs,
    io::{self, Write},
    path::Path,
    time::Instant,
};

pub(crate) use crate::image_sequence::Sequence;
fn read_image(root: &Path, name: &str) -> Result<image::RgbImage, Box<dyn Error>> {
    let path = root.join(name).canonicalize()?;
    if !path.starts_with(root) {
        return Err("sequence image symlink escapes root".into());
    }
    let mut reader = image::ImageReader::open(path)?.with_guessed_format()?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(4096);
    limits.max_image_height = Some(4096);
    reader.limits(limits);
    Ok(reader.decode()?.to_rgb8())
}
pub fn check(
    manifest: &Path,
    config: &Path,
    rediscovery_interval: u32,
    pair_separation_override: Option<f64>,
    gesture_model: Option<&Path>,
) -> Result<(), Box<dyn Error>> {
    if !(1..=30).contains(&rediscovery_interval) {
        return Err("rediscovery interval must be 1..30 frames".into());
    }
    if pair_separation_override.is_some_and(|v| !(0.8..=2.0).contains(&v)) {
        return Err("pair separation must be finite and0.8..2.0 palm widths".into());
    }
    let sequence = Sequence::load(manifest)?;
    let config = SessionConfig::load(config)?;
    let pair_separation_widths = pair_separation_override.unwrap_or(config.pair_separation_widths);
    ort::init_from(&config.runtime).commit()?;
    let root = manifest
        .canonicalize()?
        .parent()
        .ok_or("missing sequence root")?
        .to_path_buf();
    let mut model = config.replay_perception(rediscovery_interval, pair_separation_override)?;
    model.enable_tracking_diagnostics();
    let mut classifier = gesture_model
        .map(crate::pose_classifier::Classifier::new)
        .transpose()?;
    let mut gestures = Gestures::new(config.gestures(), config.touch_workspace)?;
    gestures.start();
    let mut out = io::stdout().lock();
    for (index, frame) in sequence.frames.iter().enumerate() {
        let started = Instant::now();
        let (hands, ambiguous, raw, ambiguity_reason) = if let Some(name) = &frame.image {
            let image = read_image(&root, name)?;
            let found = model.infer_many(&image)?;
            let mut raw = Vec::new();
            for hand in &found.hands {
                let mut value = serde_json::json!({
                    "id":hand.id,"confidence":hand.confidence,"handedness":hand.handedness,"tracked":hand.tracked,
                    "landmarks":hand.landmarks.iter().map(|p|[p.x,p.y]).collect::<Vec<_>>(),
                    "world_landmarks":hand.world_landmarks
                });
                if let Some(secondary) = hand.secondary_debug() {
                    value["secondary_pinch"] = secondary;
                }
                if let Some(classifier) = &mut classifier {
                    value["pose_prediction"] = serde_json::to_value(classifier.predict_hand(
                        hand,
                        image.width(),
                        image.height(),
                    )?)?;
                }
                raw.push(value);
            }
            let hands: Vec<_> = found
                .hands
                .iter()
                .filter_map(|h| {
                    config
                        .observation(Some(h), image.width(), image.height(), frame.source_us)
                        .hand
                })
                .collect();
            (hands, found.ambiguous, raw, found.ambiguity_reason)
        } else {
            (Vec::new(), false, Vec::new(), None)
        };
        let observation = Frame {
            version: 1,
            source_us: frame.source_us,
            ambiguous,
            hands: hands.clone(),
        };
        let contacts = gestures.observe(frame.source_us, observation);
        writeln!(
            out,
            "{}",
            serde_json::json!({"schema":"wigglecontrol.sequence-frame/v1","frame":index,
            "source_us":frame.source_us,"image":frame.image,"camera_opened":false,"system_input":false,
            "offline_now_equals_source":true,"processing_ms":started.elapsed().as_secs_f64()*1000.0,
            "rediscovery_interval":rediscovery_interval,
            "pair_separation_widths":pair_separation_widths,
            "palm_deduplication":config.palm_deduplication,"dual_crop":config.dual_crop,"secondary_model":config.secondary_model,"gesture_min_confidence":config.gestures().min_confidence,
            "tracking_diagnostics":frame.image.as_ref().map(|_|&model.multi_diagnostics),
            "ambiguous":ambiguous,"ambiguity_reason":ambiguity_reason,
            "perception":raw,"hands":hands,"contacts":contacts})
        )?;
        out.flush()?;
    }
    writeln!(
        out,
        "{}",
        serde_json::json!({"type":"stop","contacts":gestures.stop()})
    )?;
    Ok(())
}

/// The actual native pose/controller path over finite files, without devices.
pub fn check_controller(
    manifest: &Path,
    config_path: &Path,
    settings: &Path,
) -> Result<(), Box<dyn Error>> {
    let sequence = Sequence::load(manifest)?;
    let config = SessionConfig::load(config_path)?;
    let mut controller = crate::session::controller_config(&config, settings)?;
    ort::init_from(&config.runtime).commit()?;
    let mut perception = config.perception()?;
    perception.enable_tracking_diagnostics();
    let mut classifier = crate::pose_classifier::VisualClassifier::configured(&controller)?;
    let root = manifest
        .canonicalize()?
        .parent()
        .ok_or("missing sequence root")?
        .to_path_buf();
    let mut out = io::stdout().lock();
    controller.start();
    for (index, frame) in sequence.frames.iter().enumerate() {
        let started = Instant::now();
        let image = frame
            .image
            .as_ref()
            .map(|name| read_image(&root, name))
            .transpose()?;
        let detected = if let Some(image) = &image {
            perception.infer_many(image)?
        } else {
            crate::vision::MultiHands {
                hands: Vec::new(),
                ambiguous: false,
                ambiguity_reason: None,
            }
        };
        let (width, height) = image
            .as_ref()
            .map_or((0, 0), |image| (image.width(), image.height()));
        let hands: Vec<_> = detected
            .hands
            .iter()
            .filter_map(|h| {
                config
                    .observation(Some(h), width, height, frame.source_us)
                    .hand
            })
            .collect();
        let observation = Frame {
            version: 1,
            source_us: frame.source_us,
            ambiguous: detected.ambiguous,
            hands: hands.clone(),
        };
        let (pose, visual_pose) = if let Some(classifier) = &mut classifier {
            classifier.classify(&controller, &observation, &detected.hands, width, height)?
        } else {
            (None, None)
        };
        let state = crate::session::observe_controller(
            &config,
            &mut controller,
            frame.source_us,
            observation,
            pose,
            &detected.hands,
            [width, height],
        );
        let raw: Vec<_> = detected.hands.iter().map(|h| {
            let mut raw = serde_json::json!({"id":h.id,"confidence":h.confidence,"tracked":h.tracked,
                "handedness":h.handedness,"landmarks":h.landmarks.iter().map(|p|[p.x,p.y]).collect::<Vec<_>>(),
                "world_landmarks":h.world_landmarks});
            if let Some(secondary) = h.secondary_debug() {raw["secondary_pinch"]=secondary;}
            if let Some(visual) = visual_pose.as_ref().filter(|v| v.hand_id==h.id) {
                raw["pose_prediction"]=serde_json::to_value(&visual.prediction).expect("finite validated prediction");
            }
            raw
        }).collect();
        writeln!(
            out,
            "{}",
            serde_json::json!({"schema":"wigglecontrol.controller-sequence/v1",
            "frame":index,"source_us":frame.source_us,"image":frame.image,"width":width,"height":height,
            "processing_ms":started.elapsed().as_secs_f64()*1000.0,"offline_now_equals_source":true,
            "pair_separation_widths":config.pair_separation_widths,
            "palm_deduplication":config.palm_deduplication,"dual_crop":config.dual_crop,"secondary_model":config.secondary_model,"gesture_min_confidence":config.gestures().min_confidence,
            "tracking_diagnostics":frame.image.as_ref().map(|_|&perception.multi_diagnostics),
            "visual_macros":classifier.is_some(),"visual_pose":visual_pose,
            "perception":raw,"hands":hands,"contacts":state.contacts,"controller":state,
            "ambiguous":detected.ambiguous,"ambiguity_reason":detected.ambiguity_reason,
            "camera_opened":false,"system_input":false,"local_semantics_only":true})
        )?;
        out.flush()?;
    }
    writeln!(
        out,
        "{}",
        serde_json::json!({"type":"stop","controller":controller.stop(),
        "camera_opened":false,"system_input":false,"local_semantics_only":true})
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn replay_manifest_rejects_ambiguous_paths_and_regressing_time_before_loading_models() {
        let path = std::env::temp_dir().join(format!(
            "wigglecontrol-sequence-{}.json",
            std::process::id()
        ));
        for frames in [
            serde_json::json!([]),
            serde_json::json!([{"source_us":2,"image":"../frame.png"}]),
            serde_json::json!([{"source_us":2,"image":null},{"source_us":2,"image":null}]),
        ] {
            fs::write(
                &path,
                serde_json::json!({"version":1,"frames":frames}).to_string(),
            )
            .unwrap();
            assert!(Sequence::load(&path).is_err());
        }
        fs::write(&path,r#"{"version":1,"frames":[{"source_us":1,"image":"frames/one.png"},{"source_us":2,"image":null}]}"#).unwrap();
        assert!(Sequence::load(&path).is_ok());
        fs::remove_file(path).unwrap();
    }
}
