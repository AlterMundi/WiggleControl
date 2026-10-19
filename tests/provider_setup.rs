#![cfg(feature = "native-vision")]
use serde_json::{json, Value};
use std::{
    fs,
    os::unix::fs::{symlink, PermissionsExt},
    path::Path,
    process::{Command, Output},
};

fn prepare(root: &Path, output: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_wigglecontrol"))
        .arg("prepare-provider-config")
        .arg(root.join("session.json"))
        .arg(root.join("settings.json"))
        .arg(root.join("profile.json"))
        .arg(output)
        .output()
        .unwrap()
}
fn fixture(name: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!(
        "wiggle-provider-setup-{name}-{}",
        std::process::id()
    ));
    fs::create_dir(&root).unwrap();
    let session = json!({"camera":"/fictional/unavailable-camera","fps":10,"gain":2000,
        "gesture_min_confidence":0.5,"provider_absolute_pointer":true,
        "recording_directory":"/fictional/uncreated-recordings"});
    let config: wigglecontrol::session::SessionConfig =
        serde_json::from_value(session.clone()).unwrap();
    fs::write(root.join("session.json"), session.to_string()).unwrap();
    fs::write(
        root.join("settings.json"),
        json!({"gestures":config.gestures(),
        "right_click":true,"detail":{},"escape":{},"scroll":{"acceleration_max":2.5,"inertia":{}}})
        .to_string(),
    )
    .unwrap();
    fs::write(
        root.join("profile.json"),
        json!({"version":1,"output":[1920,1080],
        "workspace":{"min":{"x":0.23,"y":0.32},"max":{"x":0.77,"y":0.68}}})
        .to_string(),
    )
    .unwrap();
    root
}
fn value(path: &Path) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}

#[test]
fn imported_profile_produces_a_matching_private_pair_without_changing_sources_or_loading_resources()
{
    let root = fixture("accepted");
    let output = root.join("new-config");
    let originals: Vec<_> = ["session.json", "settings.json", "profile.json"]
        .iter()
        .map(|name| (*name, fs::read(root.join(name)).unwrap()))
        .collect();
    let result = prepare(&root, &output);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let report: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(report["prepared"], true);
    for field in [
        "camera_opened",
        "system_input",
        "inference_initialized",
        "installed",
    ] {
        assert_eq!(report[field], false);
    }
    assert!(!String::from_utf8_lossy(&result.stdout).contains("/fictional"));
    let session = value(&output.join("session.json"));
    let settings = value(&output.join("settings.json"));
    let profile = value(&output.join("calibration.json"));
    assert_eq!(session["touch_workspace"], profile["workspace"]);
    assert_eq!(settings["workspace"], profile["workspace"]);
    assert_eq!(session["pointer_output"], profile["output"]);
    let mut source_session = value(&root.join("session.json"));
    source_session["touch_workspace"] = profile["workspace"].clone();
    source_session["pointer_output"] = profile["output"].clone();
    assert_eq!(
        session, source_session,
        "Only the explicitly imported mapping changes"
    );
    let mut source_settings = value(&root.join("settings.json"));
    source_settings["workspace"] = profile["workspace"].clone();
    assert_eq!(
        settings, source_settings,
        "Keep gestures, cursor policy and optional actions"
    );
    assert_eq!(
        fs::metadata(&output).unwrap().permissions().mode() & 0o777,
        0o700
    );
    for name in [
        "session.json",
        "settings.json",
        "calibration.json",
        "setup.json",
    ] {
        assert_eq!(
            fs::metadata(output.join(name))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
    }
    for (name, bytes) in &originals {
        assert_eq!(&fs::read(root.join(name)).unwrap(), bytes);
    }
    let checked = Command::new(env!("CARGO_BIN_EXE_wigglecontrol"))
        .arg("config-check-provider")
        .arg(output.join("session.json"))
        .arg(output.join("settings.json"))
        .output()
        .unwrap();
    assert!(
        checked.status.success(),
        "{}",
        String::from_utf8_lossy(&checked.stderr)
    );
    assert_eq!(value(&output.join("setup.json")), report);
    assert!(
        !prepare(&root, &output).status.success(),
        "Never overwrite an existing setup"
    );
    for (name, bytes) in originals {
        assert_eq!(fs::read(root.join(name)).unwrap(), bytes);
    }
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn rejected_imports_preserve_the_previous_profile_and_leave_no_partial_configuration() {
    let root = fixture("refused");
    let output = root.join("new-config");
    let session = value(&root.join("session.json"));
    let profile = fs::read(root.join("profile.json")).unwrap();
    for (field, invalid, message) in [
        (
            "pointer_output",
            json!([1280, 720]),
            "calibration output differs",
        ),
        (
            "provider_absolute_pointer",
            json!(false),
            "held detail requires",
        ),
        ("freshness_ms", json!(301), "gesture settings must match"),
    ] {
        let mut changed = session.clone();
        changed[field] = invalid;
        fs::write(root.join("session.json"), changed.to_string()).unwrap();
        let rejected = prepare(&root, &output);
        assert!(!rejected.status.success());
        assert!(
            String::from_utf8_lossy(&rejected.stderr).contains(message),
            "{}",
            String::from_utf8_lossy(&rejected.stderr)
        );
        assert!(!output.exists());
        assert_eq!(fs::read(root.join("profile.json")).unwrap(), profile);
    }
    fs::write(root.join("session.json"), session.to_string()).unwrap();
    fs::rename(root.join("profile.json"), root.join("saved.json")).unwrap();
    symlink(root.join("saved.json"), root.join("profile.json")).unwrap();
    assert!(
        !prepare(&root, &output).status.success(),
        "Profile-store ownership/no-symlink admission applies"
    );
    assert!(!output.exists());
    fs::remove_file(root.join("profile.json")).unwrap();
    fs::write(
        root.join("profile.json"),
        br#"{"version":1,"output":[1920,1080],"bounds":[0.2,null,0.3,null]}"#,
    )
    .unwrap();
    assert!(
        !prepare(&root, &output).status.success(),
        "An unfinished draft is not an adopted profile"
    );
    assert!(!output.exists());
    assert_eq!(fs::read(root.join("saved.json")).unwrap(), profile);
    assert_eq!(
        fs::read_dir(&root).unwrap().count(),
        4,
        "No staging directories remain"
    );
    fs::remove_dir_all(root).unwrap();
}
