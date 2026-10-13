use std::io::Write;
use std::process::{Command, Stdio};
use wigglecontrol::Event;
fn run(input: &[u8]) -> std::process::Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_wigglecontrol"))
        .arg("replay")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(input).unwrap();
    child.wait_with_output().unwrap()
}
#[test]
fn fixture_drags_and_releases_on_eof_without_final_newline() {
    let fixture = include_str!("fixtures/click-drag.jsonl");
    let truncated = fixture
        .split("{\"type\": \"stop\"}")
        .next()
        .unwrap()
        .trim_end();
    let output = run(truncated.as_bytes());
    assert!(output.status.success());
    let events: Vec<Event> = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(
        events
            .iter()
            .filter(|e| **e == Event::Button { down: true })
            .count(),
        1
    );
    assert_eq!(
        events
            .iter()
            .filter(|e| **e == Event::Button { down: false })
            .count(),
        1
    );
    assert!(events
        .iter()
        .any(|e| matches!(e, Event::Move { dx, .. } if *dx > 0.0)));
}
#[test]
fn oversized_message_is_bounded_and_fails() {
    let output = run(&vec![b' '; 16 * 1024 + 1]);
    assert!(!output.status.success());
    assert!(String::from_utf8(output.stderr)
        .unwrap()
        .contains("exceeds 16 KiB"));
}
#[test]
fn malformed_input_releases_a_held_button() {
    let fixture = include_str!("fixtures/click-drag.jsonl");
    let mut first = fixture.lines().take(4).collect::<Vec<_>>().join("\n");
    first.push('\n');
    let output = run(format!("{first}{{broken\n").as_bytes());
    let events: Vec<Event> = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert!(events.contains(&Event::Button { down: true }));
    assert!(events.contains(&Event::Button { down: false }));
    assert!(events.contains(&Event::Cancel {
        reason: "malformed_input".into()
    }));
}
