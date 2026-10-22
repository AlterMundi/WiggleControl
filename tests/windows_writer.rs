#![cfg(target_os = "windows")]
//! Real process/pipe failures with fake native emission only. No desktop reads,
//! SendInput, sensors or human interaction, including in the child processes.
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::atomic::{AtomicU64, Ordering},
    thread,
    time::{Duration, Instant},
};
use wigglecontrol::{
    controller::Mode,
    output::Packet,
    platform::windows::{
        display::{Desktop, Monitor, Rect},
        monotonic_us,
        writer::{
            fixture::{self, Fault, Trace},
            Config, Receipt, Writer,
        },
    },
    Event, MAX_MESSAGE_BYTES,
};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "wiggle-writer-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn fixture(&self, fault: Fault) -> fixture::Config {
        fixture::Config {
            trace: self.0.join("events.jsonl"),
            fault,
        }
    }
    fn events(&self) -> Vec<Trace> {
        fs::read_to_string(self.0.join("events.jsonl"))
            .unwrap_or_default()
            .lines()
            .filter_map(|line| serde_json::from_str(line).ok())
            .collect()
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn executable() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_wigglecontrol"))
}
fn config() -> Config {
    let bounds = Rect {
        left: -1920,
        top: 0,
        width: 1920,
        height: 1080,
    };
    Config {
        run_id: "fixture-only".into(),
        desktop: Desktop {
            bounds,
            monitors: vec![Monitor {
                id: "fictional-monitor".into(),
                bounds,
                primary: true,
            }],
        },
        target: "fictional-monitor".into(),
        watchdog_us: 500_000,
    }
}
fn transition(seq: u64) -> Packet {
    Packet::Transition {
        sequence: seq,
        issued_us: monotonic_us().unwrap(),
        mode: Mode::Pointer,
        enabled: true,
    }
}
fn frame(seq: u64, down: bool) -> Packet {
    let issued_us = monotonic_us().unwrap();
    Packet::Frame {
        sequence: seq,
        issued_us,
        source_us: issued_us,
        mode: Mode::Pointer,
        mouse: vec![Event::Button { down }],
        absolute: None,
        contacts: vec![],
    }
}
fn wait_until(mut check: impl FnMut() -> bool) {
    let start = Instant::now();
    while !check() {
        assert!(
            start.elapsed() < Duration::from_secs(8),
            "fixture deadline expired"
        );
        thread::sleep(Duration::from_millis(5));
    }
}
fn started(scratch: &Scratch, fault: Fault) -> Writer {
    Writer::spawn_fixture(executable(), config(), scratch.fixture(fault)).unwrap()
}
#[test]
fn manual_start_is_neutral_and_eof_releases_only_owned_input() {
    let scratch = Scratch::new();
    let mut writer = started(&scratch, Fault::None);
    let receipt = writer.receipt().unwrap();
    assert!(!receipt.native_input && !receipt.armed && !receipt.stopped);
    assert_eq!(receipt.held_mask, 0);
    assert!(scratch.events().is_empty());
    writer.send(&transition(1)).unwrap();
    writer.send(&frame(2, true)).unwrap();
    wait_until(|| writer.receipt().is_some_and(|r| r.held_mask == 1));
    let outcome = writer.stop();
    assert!(outcome.neutral && !outcome.fallback_used, "{outcome:?}");
    assert!(outcome.error.is_none(), "{outcome:?}");
    let events = scratch.events();
    assert_eq!(
        events.iter().map(|e| e.flags).collect::<Vec<_>>(),
        [MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP]
    );
    assert!(events.iter().all(|e| e.role == "child"));
}
#[test]
fn watchdog_releases_then_requires_positive_rearm() {
    let scratch = Scratch::new();
    let mut writer = started(&scratch, Fault::None);
    writer.send(&transition(1)).unwrap();
    writer.send(&frame(2, true)).unwrap();
    wait_until(|| writer.receipt().is_some_and(|r| r.held_mask == 1));
    wait_until(|| {
        writer
            .receipt()
            .is_some_and(|r| r.sequence == Some(2) && !r.armed && r.held_mask == 0)
    });
    // A fresh press without another transition is terminal, never a re-press.
    writer.send(&frame(3, true)).unwrap();
    wait_until(|| writer.outcome().is_some());
    let outcome = writer.stop();
    assert!(outcome.neutral && !outcome.fallback_used);
    assert!(outcome.error.as_ref().unwrap().contains("unarmed"));
    assert_eq!(
        scratch.events().iter().map(|e| e.flags).collect::<Vec<_>>(),
        [MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP]
    );
}
#[test]
fn crash_after_press_before_ack_uses_pending_ownership_for_scoped_recovery() {
    let scratch = Scratch::new();
    let mut writer = started(&scratch, Fault::CrashAfterPress);
    writer.send(&transition(1)).unwrap();
    writer.send(&frame(2, true)).unwrap();
    wait_until(|| writer.outcome().is_some());
    assert!(writer.send(&transition(3)).is_err());
    let outcome = writer.stop();
    assert!(
        outcome.neutral && outcome.fallback_used && outcome.error.is_some(),
        "{outcome:?}"
    );
    let events = scratch.events();
    assert_eq!(events.len(), 2);
    assert_eq!(
        (events[0].role.as_str(), events[0].flags),
        ("child", MOUSEEVENTF_LEFTDOWN)
    );
    assert_eq!(
        (events[1].role.as_str(), events[1].flags),
        ("fallback", MOUSEEVENTF_LEFTUP)
    );
    assert_ne!(events[0].pid, events[1].pid);
}
#[test]
fn stalled_writer_is_terminated_before_parent_release() {
    let scratch = Scratch::new();
    let mut writer = started(&scratch, Fault::StallAfterPress);
    writer.send(&transition(1)).unwrap();
    writer.send(&frame(2, true)).unwrap();
    wait_until(|| writer.outcome().is_some());
    let outcome = writer.stop();
    assert!(outcome.neutral && outcome.fallback_used, "{outcome:?}");
    assert!(outcome.error.unwrap().contains("receipt watchdog"));
    let events = scratch.events();
    assert_eq!(
        events.iter().map(|e| e.flags).collect::<Vec<_>>(),
        [MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP]
    );
    assert_eq!(events[1].role, "fallback");
}
#[test]
fn parent_hard_exit_does_not_depend_on_rust_destructors() {
    let scratch = Scratch::new();
    let child = Command::new(executable())
        .arg("windows-writer-parent-fixture")
        .arg(serde_json::to_string(&config()).unwrap())
        .arg(serde_json::to_string(&scratch.fixture(Fault::None)).unwrap())
        .output()
        .unwrap();
    assert_eq!(
        child.status.code(),
        Some(19),
        "{}",
        String::from_utf8_lossy(&child.stderr)
    );
    wait_until(|| {
        scratch
            .events()
            .iter()
            .any(|e| e.flags == MOUSEEVENTF_LEFTUP)
    });
    let events = scratch.events();
    assert_eq!(events.len(), 2);
    assert!(events.iter().all(|e| e.role == "child"));
}
#[test]
fn explicit_stop_without_pending_press_cannot_release_physical_buttons() {
    let scratch = Scratch::new();
    let mut writer = started(&scratch, Fault::None);
    writer.send(&Packet::Stop).unwrap();
    wait_until(|| writer.outcome().is_some());
    let outcome = writer.stop();
    assert!(
        outcome.neutral && !outcome.fallback_used && outcome.error.is_none(),
        "{outcome:?}"
    );
    assert!(scratch.events().is_empty());
}
#[test]
fn malformed_oversized_and_stale_streams_close_neutral() {
    for tail in [b"garbage\n".to_vec(), vec![b'x'; MAX_MESSAGE_BYTES + 1],
        b"{\"type\":\"transition\",\"sequence\":3,\"issued_us\":0,\"mode\":\"pointer\",\"enabled\":true}\n".to_vec()] {
        let scratch = Scratch::new();
        let mut child = Command::new(executable()).arg("windows-writer-fixture")
            .arg(serde_json::to_string(&config()).unwrap())
            .arg(serde_json::to_string(&scratch.fixture(Fault::None)).unwrap())
            .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn().unwrap();
        let mut stdin = child.stdin.take().unwrap();
        for packet in [transition(1), frame(2, true)] {
            writeln!(stdin, "{}", serde_json::to_string(&packet).unwrap()).unwrap();
        }
        stdin.write_all(&tail).unwrap();
        drop(stdin);
        let output = child.wait_with_output().unwrap();
        assert!(!output.status.success());
        let receipts: Vec<Receipt> = String::from_utf8(output.stdout).unwrap().lines().map(|line| serde_json::from_str(line).unwrap()).collect();
        let last = receipts.last().unwrap();
        assert!(last.stopped && !last.armed && !last.native_input && last.error.is_some());
        assert_eq!(last.held_mask, 0);
        assert_eq!(scratch.events().iter().map(|e| e.flags).collect::<Vec<_>>(), [MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP]);
    }
}
