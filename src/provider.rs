//! Raw landmark provider boundary: correspondence/age checks, then existing meaning.
//! Camera/device-free JSONL connection. Array order never supplies an owner ID.
use crate::{
    controller::{self, Controller},
    session::SessionConfig,
    touch_gestures,
};
use serde::Deserialize;
use std::{
    error::Error,
    io::{self, Write},
    path::Path,
};

pub use crate::tracking::{Frame, Landmarks, Tracker};

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
    let config = SessionConfig::load_provider(session)?;
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
    if controller.visual_policy().is_some() && config.runtime.is_empty() {
        return Err("provider visual macros require an explicit native runtime path".into());
    }
    if controller.visual_policy().is_some_and(|policy| {
        !config.provider_presence_scores && presence_floor < policy.min_confidence
    }) {
        return Err(
            "provider presence floor must meet the visual macro confidence threshold".into(),
        );
    }
    Ok((config, controller, tracker))
}
/// Validate standalone browser-provider settings without assets, inference,
/// sensors, a listener or system input. Reports configuration, not host readiness.
pub fn check_config(session: &Path, settings: &Path) -> Result<(), Box<dyn Error>> {
    let (config, controller, _) = configured(session, settings, None)?;
    println!(
        "{}",
        serde_json::json!({"schema":"wigglecontrol.provider-config/v1","valid":true,
            "camera_opened":false,"system_input":false,"inference_initialized":false,
            "absolute_pointer":config.provider_absolute_pointer,
            "recording_configured":config.recording_enabled && config.recording_directory.is_some(),
            "onnx_runtime_required":controller.visual_policy().is_some(),
            "presence_floor":config.gestures().min_confidence,
            "right_click":controller.right_click_enabled(),
            "detail":controller.detail_config(),"escape":controller.escape_config(),
            "scroll":controller.scroll_config()})
    );
    Ok(())
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
                crate::session::observe_controller(
                    &config,
                    &mut controller,
                    frame.now_us,
                    observation,
                    pose,
                    &found.hands,
                    [frame.width, frame.height],
                )
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
    use crate::tracking::tests::{frame, hand, tracker};
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
                let (_, controller, mut tracker) = configured(&session, &settings, None).unwrap();
                let mut raw = hand(0.0, 0.09);
                raw.presence_confidence = Some(0.6);
                let observed = tracker.observe(&frame(1, vec![raw])).unwrap();
                assert_eq!(observed.hands.len(), 1);
                assert_eq!(observed.hands[0].confidence, 0.6);
                assert_eq!(controller.visual_policy().unwrap().min_confidence, 0.8);
            } else {
                let (_, controller, mut tracker) = result.unwrap();
                assert!(controller.visual_policy().is_some());
                let observed = tracker.observe(&frame(1, vec![hand(0.0, 0.09)])).unwrap();
                assert_eq!(observed.hands[0].confidence, 0.8);
                assert!(configured(&session, &settings, Some(0.5))
                    .err()
                    .unwrap()
                    .to_string()
                    .contains("matching presence floor"));
            }
        }
        // Browser landmarks do not need native hand models, but an opted-in
        // ONNX pose classifier still needs its explicit runtime before preflight.
        let mut value: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&session).unwrap()).unwrap();
        value.as_object_mut().unwrap().remove("models");
        value.as_object_mut().unwrap().remove("runtime");
        std::fs::write(&session, value.to_string()).unwrap();
        assert!(configured(&session, &settings, None)
            .err()
            .unwrap()
            .to_string()
            .contains("visual macros require an explicit native runtime path"));
        value["runtime"] = "/not-loaded".into();
        std::fs::write(&session, value.to_string()).unwrap();
        assert!(configured(&session, &settings, None).is_ok());
        std::fs::remove_dir_all(root).unwrap();
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
