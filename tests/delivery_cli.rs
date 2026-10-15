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
