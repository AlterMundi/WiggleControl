//! Camera-free image sequence replay through perception and direct-touch meaning.
use crate::{
    session::SessionConfig,
    touch_gestures::{Frame, Gestures},
};
use serde::Deserialize;
use std::{
    error::Error,
    fs,
    io::{self, Write},
    path::{Component, Path},
    time::Instant,
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Sequence {
    version: u32,
    frames: Vec<ImageFrame>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ImageFrame {
    source_us: u64,
    /// None is explicit missing acquisition, not an invented blank hand image.
    image: Option<String>,
}
impl Sequence {
    fn load(path: &Path) -> Result<Self, Box<dyn Error>> {
        if fs::metadata(path)?.len() > 1_048_576 {
            return Err("sequence exceeds 1 MiB".into());
        }
        let s: Self = serde_json::from_slice(&fs::read(path)?)?;
        if s.version != 1 || s.frames.is_empty() || s.frames.len() > 10_000 {
            return Err("sequence version 1 requires 1..10000 frames".into());
        }
        let mut last = None;
        for frame in &s.frames {
            if last.is_some_and(|t| frame.source_us <= t) {
                return Err("sequence source time must increase".into());
            }
            last = Some(frame.source_us);
            if let Some(name) = &frame.image {
                if name.is_empty()
                    || Path::new(name)
                        .components()
                        .any(|p| !matches!(p, Component::Normal(_)))
                {
                    return Err(
                        "sequence image must be a relative path without dot/parent components"
                            .into(),
                    );
                }
            }
        }
        Ok(s)
    }
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
    let mut classifier = gesture_model
        .map(crate::pose_classifier::Classifier::new)
        .transpose()?;
    let mut gestures = Gestures::new(config.gestures(), config.touch_workspace)?;
    gestures.start();
    let mut out = io::stdout().lock();
    for (index, frame) in sequence.frames.iter().enumerate() {
        let started = Instant::now();
        let (hands, ambiguous, raw, ambiguity_reason) = if let Some(name) = &frame.image {
            let path = root.join(name).canonicalize()?;
            if !path.starts_with(&root) {
                return Err("sequence image symlink escapes root".into());
            }
            let reader = image::ImageReader::open(path)?.with_guessed_format()?;
            let mut limits = image::Limits::default();
            limits.max_image_width = Some(4096);
            limits.max_image_height = Some(4096);
            let mut reader = reader;
            reader.limits(limits);
            let image = reader.decode()?.to_rgb8();
            let found = model.infer_many(&image)?;
            let mut raw = Vec::new();
            for hand in &found.hands {
                let mut value = serde_json::json!({
                    "id":hand.id,"confidence":hand.confidence,"handedness":hand.handedness,"tracked":hand.tracked,
                    "landmarks":hand.landmarks.iter().map(|p|[p.x,p.y]).collect::<Vec<_>>(),
                    "world_landmarks":hand.world_landmarks
                });
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
