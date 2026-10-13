#![cfg(feature = "native-vision")]
use std::{fs, process::Command};

#[test]
fn config_check_needs_no_camera_models_or_runtime_and_rejects_invalid_bounds() {
    let path = std::env::temp_dir().join(format!(
        "wigglecontrol-cli-config-{}.json",
        std::process::id()
    ));
    let mut config = serde_json::json!({
        "camera":"/fictional/unavailable-camera",
        "models":"/fictional/unavailable-models",
        "runtime":"/fictional/unavailable-runtime", "fps":10, "gain":2000
    });
    let check = || {
        Command::new(env!("CARGO_BIN_EXE_wigglecontrol"))
            .arg("config-check")
            .arg(&path)
            .output()
            .unwrap()
    };
    fs::write(&path, config.to_string()).unwrap();
    let result = check();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(report["valid"], true);
    assert_eq!(report["prayer_enabled"], false);
    assert_eq!(report["touch_workspace"]["max"]["x"], 1.0);
    assert!(report.get("camera").is_none());
    assert_eq!(report["pinch_close_ratio"], 0.25);
    assert_eq!(report["pinch_open_ratio"], 0.4);
    config["prayer_enabled"] = true.into();
    fs::write(&path, config.to_string()).unwrap();
    let rejected = Command::new(env!("CARGO_BIN_EXE_wigglecontrol"))
        .args(["camera-check", "1"])
        .arg(&path)
        .output()
        .unwrap();
    assert!(!rejected.status.success());
    assert!(String::from_utf8_lossy(&rejected.stderr)
        .contains("prayer entry requires run-calibrated mode"));
    config["prayer_enabled"] = false.into();
    config["pinch_close_ratio"] = serde_json::json!(0.35);
    config["pinch_open_ratio"] = serde_json::json!(0.5);
    fs::write(&path, config.to_string()).unwrap();
    assert!(check().status.success());
    config["pinch_open_ratio"] = serde_json::json!(0.3);
    fs::write(&path, config.to_string()).unwrap();
    assert!(!check().status.success());
    config["pinch_open_ratio"] = serde_json::json!(0.5);
    config["touch_workspace"] = serde_json::json!({
        "min":{"x":0.8,"y":0.2}, "max":{"x":0.2,"y":0.8}
    });
    fs::write(&path, config.to_string()).unwrap();
    let result = check();
    assert!(!result.status.success());
    assert!(result.stdout.is_empty());
    assert!(String::from_utf8_lossy(&result.stderr).contains("touch workspace"));
    fs::remove_file(path).unwrap();
}
