#![cfg(target_os = "linux")]
use std::{
    io::Write,
    process::{Command, Stdio},
};
fn run(input: &[u8]) -> std::process::Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_wigglecontrol"))
        .arg("touch-replay")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(input).unwrap();
    child.wait_with_output().unwrap()
}
#[test]
fn fixture_preserves_two_contacts_and_eof_neutralizes() {
    let fixture = include_str!("fixtures/two-hand-touch.jsonl");
    // End while both contacts are active, without a final newline.
    let lines: Vec<_> = fixture.lines().take(5).collect();
    let result = run(lines.join("\n").as_bytes());
    assert!(result.status.success());
    let frames: Vec<serde_json::Value> = String::from_utf8(result.stdout)
        .unwrap()
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect();
    assert_eq!(frames[3]["contacts"].as_array().unwrap().len(), 2);
    assert_eq!(
        frames[3]["contacts"][0]["id"],
        frames[4]["contacts"][0]["id"]
    );
    assert!(frames.last().unwrap()["contacts"]
        .as_array()
        .unwrap()
        .is_empty());
}
#[test]
fn malformed_touch_replay_releases_and_reports_failure() {
    let mut input = include_str!("fixtures/two-hand-touch.jsonl")
        .lines()
        .take(4)
        .collect::<Vec<_>>()
        .join("\n");
    input.push_str("\n{bad}\n");
    let result = run(input.as_bytes());
    assert!(!result.status.success());
    let output = String::from_utf8(result.stdout).unwrap();
    assert_eq!(
        output.lines().last(),
        Some("{\"type\":\"frame\",\"contacts\":[]}")
    );
}
