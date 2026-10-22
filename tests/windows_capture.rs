#![cfg(all(target_os = "windows", feature = "windows-camera"))]
//! Real process/pipe lifecycle with generated JPEGs. Never enumerate/activate
//! a camera, load a hand model, render a preview or send system input.
use std::{
    path::Path,
    thread,
    time::{Duration, Instant},
};
use wigglecontrol::platform::windows::camera::{fixture::Fault, Capture, Format, Spec};
fn spec() -> Spec {
    Spec {
        camera: "fictional-camera".into(),
        format: Format {
            resolution: [1920, 1080],
            frame_rate: [30, 1],
        },
        freshness_us: 500_000,
        run_id: "fixture".into(),
    }
}
fn executable() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_wigglecontrol"))
}
fn wait(mut condition: impl FnMut() -> bool) {
    let start = Instant::now();
    while !condition() {
        assert!(
            start.elapsed() < Duration::from_secs(8),
            "capture fixture deadline expired"
        );
        thread::sleep(Duration::from_millis(5));
    }
}
#[test]
fn manual_capture_retains_native_aspect_latest_frame_and_confirmed_eof_close() {
    let capture = Capture::spawn_fixture(executable(), spec(), Fault::None).unwrap();
    assert!(!capture.status().native_capture);
    wait(|| capture.status().fresh_frames >= 3);
    let frame = capture.take_frame().unwrap().unwrap();
    assert!(frame.source_us > 0 && frame.sequence >= 2);
    let dimensions = image::ImageReader::new(std::io::Cursor::new(&frame.bytes))
        .with_guessed_format()
        .unwrap()
        .into_dimensions()
        .unwrap();
    assert_eq!(dimensions, (1920, 1080));
    assert_eq!(capture.status().negotiated, Some(spec().format));
    let status = capture.status();
    assert!(status.received >= status.fresh_frames);
    assert!(
        capture.status().dropped >= 1,
        "unconsumed frames are replaced, never queued without bounds"
    );
    let final_state = capture.stop();
    assert!(final_state.process_exited, "{final_state:?}");
    assert_eq!(final_state.camera_opened(), Some(false));
    assert!(final_state.error.is_none(), "{final_state:?}");
}
#[test]
fn blocked_or_partial_driver_stream_is_terminated_and_reported_after_process_exit() {
    for fault in [Fault::Stall, Fault::Partial] {
        let capture = Capture::spawn_fixture(executable(), spec(), fault).unwrap();
        wait(|| capture.status().process_exited);
        let status = capture.stop();
        assert!(
            status.process_exited && status.error.is_some(),
            "{status:?}"
        );
        assert_eq!(status.camera_opened(), Some(false));
        assert!(status.error.unwrap().contains("watchdog"));
    }
}
#[test]
fn invalid_transport_or_clock_never_returns_another_frame_after_failure() {
    for fault in [Fault::Oversized, Fault::Future, Fault::Regress] {
        let capture = Capture::spawn_fixture(executable(), spec(), fault).unwrap();
        wait(|| capture.status().process_exited);
        assert!(capture.take_frame().is_err());
        let status = capture.stop();
        assert!(status.process_exited && status.error.is_some());
        assert_eq!(status.camera_opened(), Some(false));
    }
}
#[test]
fn wrong_format_startup_error_and_crash_do_not_look_like_a_ready_camera() {
    assert!(Capture::spawn_fixture(executable(), spec(), Fault::WrongFormat).is_err());
    let denied = Capture::spawn_fixture(executable(), spec(), Fault::StartupError);
    assert!(denied
        .err()
        .unwrap()
        .to_string()
        .contains("denied or timestamp"));
    // Exit may race the startup handshake; both paths must retain a failure.
    if let Ok(capture) = Capture::spawn_fixture(executable(), spec(), Fault::Crash) {
        wait(|| capture.status().process_exited);
        let status = capture.stop();
        assert!(status.error.is_some());
        assert_eq!(status.camera_opened(), Some(false));
    }
}

#[test]
fn hard_parent_exit_also_ends_a_camera_child_blocked_in_the_driver() {
    use std::{
        io::{BufRead, BufReader, Write},
        process::{Command, Stdio},
    };
    use windows::Win32::{
        Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0, WAIT_TIMEOUT},
        System::Threading::{OpenProcess, WaitForSingleObject, PROCESS_SYNCHRONIZE},
    };
    struct Handle(HANDLE);
    impl Drop for Handle {
        fn drop(&mut self) {
            unsafe {
                let _ = CloseHandle(self.0);
            }
        }
    }
    let mut parent = Command::new(executable())
        .arg("windows-camera-parent-fixture")
        .arg(serde_json::to_string(&spec()).unwrap())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdout = BufReader::new(parent.stdout.take().unwrap());
    let mut line = String::new();
    stdout.read_line(&mut line).unwrap();
    let pid = line.trim().parse::<u32>().unwrap();
    // Hold the original process handle before asking its parent to hard-exit;
    // no PID-reuse assumption or absence of an entry is treated as proof.
    let handle = Handle(unsafe { OpenProcess(PROCESS_SYNCHRONIZE, false, pid).unwrap() });
    assert_eq!(unsafe { WaitForSingleObject(handle.0, 0) }, WAIT_TIMEOUT);
    parent.stdin.take().unwrap().write_all(b"x").unwrap();
    assert_eq!(parent.wait_with_output().unwrap().status.code(), Some(19));
    assert_eq!(
        unsafe { WaitForSingleObject(handle.0, 4000) },
        WAIT_OBJECT_0
    );
}
