#![cfg(feature = "provider-runtime")]
//! Actual configured CLI chain, with fictional raw frames and no browser, camera,
//! input, hand model assets or ORT initialization when visual macros are disabled.
use serde_json::{json, Value};
use std::{
    fs,
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
    sync::atomic::{AtomicU64, Ordering},
};
use wigglecontrol::session_config::SessionConfig;
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture {
    root: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "wiggle-provider-runtime-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        let config = json!({"camera":root.join("not-acquired"),"camera_resolution":[1920,1080],
            "fps":10,"gain":2000,"freshness_ms":300,"press_confirm_us":40000,
            "pinch_3d":true,"pinch_close_m":0.045,"pinch_open_m":0.065,
            "gesture_min_confidence":0.5,"provider_absolute_pointer":true,"pointer_output":[1920,1080]});
        fs::write(root.join("session.json"), config.to_string()).unwrap();
        let config: SessionConfig = serde_json::from_value(config).unwrap();
        fs::write(
            root.join("settings.json"),
            json!({"version":1,"initial_mode":"pointer",
            "workspace":config.touch_workspace,"gestures":config.gestures()})
            .to_string(),
        )
        .unwrap();
        Self { root }
    }
    fn command(&self, action: &str) -> Command {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_wigglecontrol"));
        cmd.arg(action)
            .arg(self.root.join("session.json"))
            .arg(self.root.join("settings.json"));
        cmd
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
fn raw_frame(sequence: u64, gap: f64) -> Value {
    // Same fictional geometry as tracking's established correspondence fixture.
    let mut points = [[100.0, 200.0]; 21];
    for (i, p) in [
        (0, [100., 300.]),
        (4, [20., 120.]),
        (5, [75., 200.]),
        (6, [75., 170.]),
        (8, [75., 100.]),
        (9, [100., 185.]),
        (10, [100., 150.]),
        (12, [100., 80.]),
        (13, [115., 195.]),
        (14, [110., 170.]),
        (16, [110., 110.]),
        (17, [125., 200.]),
        (18, [125., 180.]),
        (20, [125., 140.]),
    ] {
        points[i] = p;
    }
    let mut world = [[0.; 3]; 21];
    world[4][0] = gap;
    let now = (sequence - 1) * 100_000 + 1;
    json!({"type":"frame","observation":{"sequence":sequence,"source_us":now,"now_us":now,
        "width":1920,"height":1080,"hands":[{"landmarks":points,"world_landmarks":world,"right_probability":0.9}]}})
}
#[test]
fn configured_raw_replay_has_real_button_edges_and_neutral_terminal_on_eof_or_rejection() {
    let fixture = Fixture::new();
    let stale =
        json!({"type":"frame","observation":{"sequence":10,"source_us":1_000_001,"now_us":1_500_001,
        "width":1920,"height":1080,"hands":[]}})
        .to_string() + "\n";
    let old =
        json!({"type":"frame","observation":{"sequence":10,"source_us":800_001,"now_us":900_001,
        "width":1920,"height":1080,"hands":[]}})
        .to_string()
            + "\n";
    for (tail, success, reason) in [
        (None, true, None),
        (Some("{broken\n".to_owned()), false, None),
        (Some(" ".repeat(16 * 1024 + 1)), false, None),
        (Some(stale), true, Some("provider_age")),
        (Some(old), true, Some("provider_order")),
    ] {
        let mut bytes = b"{\"type\":\"start\"}\n".to_vec();
        for (i, gap) in [0.09, 0.09, 0.02, 0.02, 0.02, 0.09, 0.09, 0.02, 0.02]
            .into_iter()
            .enumerate()
        {
            writeln!(&mut bytes, "{}", raw_frame(i as u64 + 1, gap)).unwrap();
        }
        if let Some(tail) = tail {
            bytes.extend(tail.as_bytes());
        }
        if reason.is_some() {
            writeln!(&mut bytes, "{}", raw_frame(17, 0.02)).unwrap();
        }
        let mut child = fixture
            .command("provider-replay")
            .arg("0.5")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let mut input = child.stdin.take().unwrap();
        let feed = std::thread::spawn(move || input.write_all(&bytes));
        let result = child.wait_with_output().unwrap();
        feed.join().unwrap().unwrap();
        assert_eq!(
            result.status.success(),
            success,
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        let rows: Vec<Value> = String::from_utf8(result.stdout)
            .unwrap()
            .lines()
            .map(|s| serde_json::from_str(s).unwrap())
            .collect();
        assert!(
            rows.iter().any(|r| r["controller"]["button_down"] == true),
            "{rows:?}"
        );
        assert!(rows
            .iter()
            .any(|r| r["controller"]["mouse"].as_array().is_some_and(|m| m
                .iter()
                .any(|e| e["type"] == "button" && e["down"] == false))));
        if let Some(reason) = reason {
            // Invalid observation evidence is loss, not a malformed stream. It
            // releases before EOF and must not press on closed-hand reacquisition.
            let loss = rows.iter().find(|r| r["sequence"] == 10).unwrap();
            assert_eq!(loss["ambiguous"], true);
            assert_eq!(loss["ambiguity_reason"], reason);
            assert_eq!(loss["controller"]["button_down"], false);
            assert_eq!(
                rows.iter().find(|r| r["sequence"] == 17).unwrap()["controller"]["button_down"],
                false
            );
        }
        let last = rows.last().unwrap();
        assert_eq!(last["controller"]["running"], false);
        assert_eq!(last["controller"]["button_down"], false);
        assert!(rows.iter().all(|r| r["camera_opened"] == false
            && r["system_input"] == false
            && r["local_semantics_only"] == true));
        assert!(rows.iter().all(|r| r["visual_macros"] != true));
    }
    assert_eq!(
        fs::read_dir(&fixture.root).unwrap().count(),
        2,
        "No model, recording, profile or camera output was created"
    );
}
#[test]
fn preflight_works_without_models_and_rejects_mismatched_controller_authority() {
    let fixture = Fixture::new();
    let output = fixture.command("config-check-provider").output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    for key in [
        "camera_opened",
        "system_input",
        "inference_initialized",
        "onnx_runtime_required",
        "recording_configured",
    ] {
        assert_eq!(value[key], false, "{key}");
    }
    assert_eq!(value["absolute_pointer"], true);
    let path = fixture.root.join("settings.json");
    let original = fs::read(&path).unwrap();
    let mut changed: Value = serde_json::from_slice(&original).unwrap();
    changed["gestures"]["stale_us"] = 999999.into();
    fs::write(&path, changed.to_string()).unwrap();
    let rejected = fixture.command("config-check-provider").output().unwrap();
    assert!(!rejected.status.success());
    assert!(String::from_utf8_lossy(&rejected.stderr).contains("gesture settings must match"));
    fs::write(&path, original).unwrap();
    let mut session: Value =
        serde_json::from_slice(&fs::read(fixture.root.join("session.json")).unwrap()).unwrap();
    session["camera_resolution"] = json!([1921, 1080]);
    fs::write(fixture.root.join("session.json"), session.to_string()).unwrap();
    let rejected = fixture.command("config-check-provider").output().unwrap();
    assert!(!rejected.status.success());
    assert!(String::from_utf8_lossy(&rejected.stderr).contains("camera_resolution"));
}
