#![cfg(all(target_os = "linux", feature = "native-vision"))]
use std::{fs, path::Path};
use wigglecontrol::session::SessionConfig;

fn load(
    path: &Path,
    value: &serde_json::Value,
) -> Result<SessionConfig, Box<dyn std::error::Error>> {
    fs::write(path, serde_json::to_vec(value)?)?;
    SessionConfig::load(path)
}

#[test]
fn secondary_model_preflight_requires_explicit_compatible_policy_without_opening_resources() {
    let path = std::env::temp_dir().join(format!(
        "wigglecontrol-secondary-{}.json",
        std::process::id()
    ));
    let mut value = serde_json::json!({
        "camera":"/fictional/camera","models":"/fictional/models","runtime":"/fictional/runtime",
        "fps":10,"gain":2000,"pinch_3d":true
    });
    let old = load(&path, &value).unwrap();
    assert_eq!(old.gestures().min_confidence, 0.7);
    assert!(!old.dual_crop && old.secondary_model.is_none());
    value["dual_crop"] = true.into();
    value["secondary_model"] =
        serde_json::json!({"path":"/fictional/converted.onnx","min_presence":0.5,"source_id":3});
    assert!(load(&path, &value).is_err()); //0.7 cannot admit the selected provider's0.5 policy.
    value["gesture_min_confidence"] = 0.5.into();
    let candidate = load(&path, &value).unwrap();
    assert_eq!(candidate.gestures().min_confidence, 0.5);
    assert_eq!(candidate.secondary_model.unwrap().source_id, 3);
    for (key, invalid) in [
        ("path", serde_json::json!("relative.onnx")),
        ("min_presence", serde_json::json!(1.1)),
        ("source_id", serde_json::json!(1)),
    ] {
        let mut bad = value.clone();
        bad["secondary_model"][key] = invalid;
        assert!(load(&path, &bad).is_err(), "{key}");
    }
    for key in ["dual_crop", "pinch_3d"] {
        let mut bad = value.clone();
        bad[key] = false.into();
        assert!(load(&path, &bad).is_err(), "{key}");
    }
    fs::remove_file(path).unwrap();
}
