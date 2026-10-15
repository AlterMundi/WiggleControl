#![cfg(target_os = "linux")]
use serde_json::Value;
use std::{
    fs,
    io::Write,
    process::{Command, Stdio},
};
#[test]
fn bounded_mode_replay_releases_on_switch_eof_and_malformed_input_without_devices() {
    let path =
        std::env::temp_dir().join(format!("wigglecontrol-modes-{}.json", std::process::id()));
    fs::write(&path, r#"{"version":1,"initial_mode":"two_contacts"}"#).unwrap();
    let fixture = include_str!("fixtures/two-hand-touch.jsonl")
        .lines()
        .take(4)
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    for (tail, success) in [
        ("".to_owned(), true),
        ("{broken\n".to_owned(), false),
        (" ".repeat(16 * 1024 + 1), false),
        (
            "{\"type\":\"mode\",\"mode\":\"pointer\"}\n".to_owned(),
            true,
        ),
    ] {
        let mut child = Command::new(env!("CARGO_BIN_EXE_wigglecontrol"))
            .arg("mode-replay")
            .arg(&path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(format!("{fixture}{tail}").as_bytes())
            .unwrap();
        let result = child.wait_with_output().unwrap();
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
        assert!(rows
            .iter()
            .any(|r| r["output"]["contacts"].as_array().unwrap().len() == 2));
        assert!(rows.last().unwrap()["output"]["contacts"]
            .as_array()
            .unwrap()
            .is_empty());
        assert_eq!(rows.last().unwrap()["output"]["running"], false);
        assert!(rows
            .iter()
            .all(|r| r["camera_opened"] == false && r["system_input"] == false));
    }
    fs::remove_file(path).unwrap();
}
