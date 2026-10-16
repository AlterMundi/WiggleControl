#![cfg(target_os = "linux")]
use evdev::KeyCode;
use serde_json::{json, Value};
use std::{
    io::Write,
    process::{Command, Stdio},
};
#[test]
fn delivery_protocol_replay_is_device_free_and_releases_before_reporting_malformed_input() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_wigglecontrol"))
        .args(["delivery-replay", "2000", "150"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let packets = [
        json!({"type":"packet","now_us":1,"packet":{"type":"transition","sequence":1,"issued_us":1,"mode":"pointer","enabled":true}}),
        json!({"type":"packet","now_us":2,"packet":{"type":"frame","sequence":2,"issued_us":2,"source_us":2,"mode":"pointer","mouse":[{"type":"button","down":true}],"contacts":[]}}),
    ];
    let input = packets
        .iter()
        .map(Value::to_string)
        .collect::<Vec<_>>()
        .join("\n")
        + "\n{broken\n";
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    let result = child.wait_with_output().unwrap();
    assert!(!result.status.success());
    let rows: Vec<Value> = String::from_utf8(result.stdout)
        .unwrap()
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect();
    assert!(rows
        .iter()
        .all(|r| r["camera_opened"] == false && r["system_input"] == false));
    assert!(rows.iter().any(|r| r["mouse"]
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["code"] == KeyCode::BTN_LEFT.0 && e["value"] == 1)));
    assert!(rows.last().unwrap()["mouse"]
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["code"] == KeyCode::BTN_LEFT.0 && e["value"] == 0));
}

#[test]
fn full_pipeline_encodes_two_contacts_and_eof_release_without_uinput() {
    use std::fs;
    let path = std::env::temp_dir().join(format!(
        "wigglecontrol-pipeline-{}.json",
        std::process::id()
    ));
    fs::write(&path, r#"{"initial_mode":"two_contacts"}"#).unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_wigglecontrol"))
        .arg("pipeline-replay")
        .arg(&path)
        .args(["2000", "150"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let input = include_str!("fixtures/two-hand-touch.jsonl")
        .lines()
        .take(4)
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    let result = child.wait_with_output().unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let rows: Vec<Value> = String::from_utf8(result.stdout)
        .unwrap()
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect();
    let tracking = evdev::AbsoluteAxisCode::ABS_MT_TRACKING_ID.0;
    assert_eq!(
        rows.iter()
            .flat_map(|r| r["touch"].as_array().unwrap())
            .filter(|e| e["code"] == tracking && e["value"].as_i64().unwrap() >= 0)
            .count(),
        2
    );
    assert_eq!(
        rows.last().unwrap()["touch"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|e| e["code"] == tracking && e["value"] == -1)
            .count(),
        2
    );
    assert_eq!(rows.last().unwrap()["controller"]["running"], false);
    assert!(rows.iter().all(|r| r["system_input"] == false));
    fs::remove_file(path).unwrap();
}

#[test]
fn absolute_beta_pipeline_preserves_calibration_button_order_and_eof_release_without_devices() {
    use std::fs;
    let path = std::env::temp_dir().join(format!(
        "wigglecontrol-absolute-pipeline-{}.json",
        std::process::id()
    ));
    fs::write(
        &path,
        r#"{"workspace":{"min":{"x":0.2,"y":0.2},"max":{"x":0.8,"y":0.8}}}"#,
    )
    .unwrap();
    let frame = |t, ratio| {
        json!({"type":"frame","now_us":t,"observation":{"version":1,"source_us":t,"ambiguous":false,"hands":[{
            "id":1,"confidence":0.99,"engaged":true,"pinch_eligible":true,
            "cursor":{"x":0.65,"y":0.35},"thumb_tip":{"x":0.4,"y":0.5},
            "index_tip":{"x":0.5,"y":0.5},"palm_width":0.2,"pinch_distance_ratio":ratio
        }]}})
    };
    let input = [
        json!({"type":"start"}),
        frame(1, 0.6),
        frame(20_001, 0.1),
        frame(60_001, 0.1),
    ]
    .iter()
    .map(Value::to_string)
    .collect::<Vec<_>>()
    .join("\n")
        + "\n";
    for (tail, expected_success) in [
        ("", true),
        ("{\"type\":\"mode\",\"mode\":\"one_contact\"}\n", false),
    ] {
        let mut child = Command::new(env!("CARGO_BIN_EXE_wigglecontrol"))
            .arg("pipeline-replay")
            .arg(&path)
            .args(["2000", "150", "--absolute"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(format!("{input}{tail}").as_bytes())
            .unwrap();
        let result = child.wait_with_output().unwrap();
        assert!(
            result.status.success() == expected_success,
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        let rows: Vec<Value> = String::from_utf8(result.stdout)
            .unwrap()
            .lines()
            .map(|s| serde_json::from_str(s).unwrap())
            .collect();
        assert!(rows.iter().all(|r| r["camera_opened"] == false
            && r["system_input"] == false
            && r["absolute_pointer"] == true
            && r["touch"].as_array().unwrap().is_empty()));
        let press = rows
            .iter()
            .find(|r| r["controller"]["button_down"] == true)
            .unwrap();
        let events = press["mouse"].as_array().unwrap();
        assert_eq!(events[0], json!({"kind":3,"code":0,"value":49151}));
        assert_eq!(events[1], json!({"kind":3,"code":1,"value":16384}));
        assert_eq!(
            events[2],
            json!({"kind":1,"code":KeyCode::BTN_LEFT.0,"value":1})
        );
        assert!((press["packet"]["absolute"]["x"].as_f64().unwrap() - 0.75).abs() < 1e-12);
        assert!((press["packet"]["absolute"]["y"].as_f64().unwrap() - 0.25).abs() < 1e-12);
        assert!(rows.last().unwrap()["mouse"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["kind"] == 1 && e["code"] == KeyCode::BTN_LEFT.0 && e["value"] == 0));
    }
    fs::remove_file(path).unwrap();
}
