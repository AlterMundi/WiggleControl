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
    assert!(report["adaptive_smoothing"].is_null());
    assert_eq!(report["pair_separation_widths"], 0.8);
    config["pair_separation_widths"] = serde_json::json!(1.2);
    fs::write(&path, config.to_string()).unwrap();
    let selected = check();
    assert!(selected.status.success());
    let selected: serde_json::Value = serde_json::from_slice(&selected.stdout).unwrap();
    assert_eq!(selected["pair_separation_widths"], 1.2);
    config["adaptive_smoothing"] =
        serde_json::json!({"min_cutoff_hz":0.5,"beta":20.0,"derivative_cutoff_hz":1.0});
    fs::write(&path, config.to_string()).unwrap();
    let result = check();
    assert!(result.status.success());
    let report: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(report["adaptive_smoothing"]["beta"], 20.0);
    config["adaptive_smoothing"]["beta"] = (-1).into();
    fs::write(&path, config.to_string()).unwrap();
    assert!(!check().status.success());
    config.as_object_mut().unwrap().remove("adaptive_smoothing");
    config["freshness_ms"] = serde_json::json!(300);
    fs::write(&path, config.to_string()).unwrap();
    assert!(check().status.success());
    config["freshness_ms"] = serde_json::json!(1001);
    fs::write(&path, config.to_string()).unwrap();
    assert!(!check().status.success());
    config["freshness_ms"] = serde_json::json!(300);
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

#[test]
fn controller_preflight_rejects_divergent_settings_without_acquiring_any_receiving_path() {
    let root = std::env::temp_dir().join(format!(
        "wigglecontrol-controller-config-{}",
        std::process::id()
    ));
    fs::create_dir(&root).unwrap();
    let session = root.join("session.json");
    let modes = root.join("modes.json");
    fs::write(&session,serde_json::json!({"camera":"/fictional/unavailable-camera","models":"/fictional/unavailable-models","runtime":"/fictional/unavailable-runtime","fps":10,"gain":2000}).to_string()).unwrap();
    fs::write(&modes, "{}").unwrap();
    let check = || {
        Command::new(env!("CARGO_BIN_EXE_wigglecontrol"))
            .arg("config-check-controller")
            .arg(&session)
            .arg(&modes)
            .output()
            .unwrap()
    };
    let result = check();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(report["valid"], true);
    assert_eq!(report["camera_opened"], false);
    assert_eq!(report["system_input"], false);
    let visual = serde_json::json!({"macros":{
        "hold_us":500000,"neutral_us":200000,"cooldown_us":1000000,"stale_us":150000,
        "bindings":[{"pose":"victory","action":"toggle_precision"}]},
        "visual_macros":{"model":"/fictional/unavailable-pose-model.onnx","neutral_pose":"open_palm"}});
    fs::write(&modes, visual.to_string()).unwrap();
    let enabled = check();
    assert!(
        enabled.status.success(),
        "{}",
        String::from_utf8_lossy(&enabled.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&enabled.stdout).unwrap();
    assert_eq!(report["visual_macros"], true);
    assert_eq!(report["neutral_pose"], "open_palm");
    assert!(report.get("model").is_none());
    for failure in 0..3 {
        let mut invalid = visual.clone();
        match failure {
            0 => invalid["visual_macros"]["neutral_pose"] = "victory".into(),
            1 => invalid["visual_macros"]["min_confidence"] = 0.79.into(),
            _ => {
                invalid.as_object_mut().unwrap().remove("macros");
            }
        }
        fs::write(&modes, invalid.to_string()).unwrap();
        assert!(!check().status.success());
    }
    fs::write(&modes, r#"{"gestures":{"close_m":0.045,"open_m":0.065}}"#).unwrap();
    let result = check();
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("must match the session"));

    let mut source = serde_json::json!({"camera":"/fictional/unavailable-camera",
        "models":"/fictional/unavailable-models","runtime":"/fictional/unavailable-runtime",
        "fps":10,"gain":2000,"provider_absolute_pointer":true,"pointer_cursor_output":{}});
    fs::write(&session, source.to_string()).unwrap();
    fs::write(&modes, "{}").unwrap();
    assert!(String::from_utf8_lossy(&check().stderr).contains("cursor output settings must match"));
    fs::write(&modes, r#"{"cursor_output":{}}"#).unwrap();
    let selected = check();
    assert!(
        selected.status.success(),
        "{}",
        String::from_utf8_lossy(&selected.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&selected.stdout).unwrap();
    assert_eq!(report["camera_opened"], false);
    assert_eq!(report["system_input"], false);
    let unsupported = Command::new(env!("CARGO_BIN_EXE_wigglecontrol"))
        .args(["camera-check", "1"])
        .arg(&session)
        .output()
        .unwrap();
    assert!(!unsupported.status.success());
    assert!(String::from_utf8_lossy(&unsupported.stderr)
        .contains("explicit camera-worker provider workflow"));
    source["press_confirm_us"] = 0.into();
    fs::write(&session, source.to_string()).unwrap();
    assert!(String::from_utf8_lossy(&check().stderr).contains("gesture settings must match"));
    fs::write(
        &modes,
        r#"{"cursor_output":{},"gestures":{"confirm_us":0}}"#,
    )
    .unwrap();
    assert!(check().status.success());
    source["pointer_release_confirmation"] =
        serde_json::json!({"confirm_us":40000,"immediate_open_factor":1.25});
    fs::write(&session, source.to_string()).unwrap();
    assert!(String::from_utf8_lossy(&check().stderr).contains("gesture settings must match"));
    fs::write(&modes, r#"{"cursor_output":{},"gestures":{"confirm_us":0,"release_confirmation":{"confirm_us":40000,"immediate_open_factor":1.25}}}"#).unwrap();
    assert!(check().status.success());
    source["pointer_cursor_output"] = serde_json::Value::Null;
    fs::write(&session, source.to_string()).unwrap();
    let unsupported = Command::new(env!("CARGO_BIN_EXE_wigglecontrol"))
        .args(["camera-check", "1"])
        .arg(&session)
        .output()
        .unwrap();
    assert!(!unsupported.status.success());
    assert!(String::from_utf8_lossy(&unsupported.stderr)
        .contains("release confirmation is supported by the explicit camera-worker"));
    source["pointer_cursor_output"] = serde_json::json!({});
    source["pointer_release_confirmation"]["confirm_us"] = 300_000.into();
    fs::write(&session, source.to_string()).unwrap();
    assert!(String::from_utf8_lossy(&check().stderr).contains("release confirmation requires"));
    source
        .as_object_mut()
        .unwrap()
        .remove("pointer_release_confirmation");
    fs::write(
        &modes,
        r#"{"cursor_output":{},"gestures":{"confirm_us":0}}"#,
    )
    .unwrap();
    source["pointer_cursor_output"]["click_stabilization"] = serde_json::json!({"radius":0});
    fs::write(&session, source.to_string()).unwrap();
    assert!(String::from_utf8_lossy(&check().stderr).contains("click stabilization requires"));
    source["pointer_cursor_output"]
        .as_object_mut()
        .unwrap()
        .remove("click_stabilization");
    source["pointer_cursor_output"]["prediction_us"] = 100_001.into();
    fs::write(&session, source.to_string()).unwrap();
    assert!(String::from_utf8_lossy(&check().stderr).contains("prediction0..100ms"));
    fs::remove_dir_all(root).unwrap();
}
