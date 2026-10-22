//! Explicit Windows native-MJPEG capture, bounded latest-frame transport and
//! driver-clock validation. No construction, enumeration or test starts input.
use super::{
    monotonic_us,
    process::{terminate, CREATE_NO_WINDOW},
};
pub use crate::camera_frame::Frame;
use serde::{Deserialize, Serialize};
use std::{
    io::{self, BufRead, BufReader, Read, Write},
    os::windows::process::CommandExt,
    path::Path,
    process::{Child, ChildStdin, Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Condvar, Mutex,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};
#[doc(hidden)]
pub mod fixture;
mod native;
pub use native::{devices, Device};
const MAX_HEADER: usize = 8192;
const MAX_JPEG_BYTES: usize = 2 * 1024 * 1024;
const STARTUP: Duration = Duration::from_secs(5);
const SILENCE: Duration = Duration::from_secs(1);
const POLL: Duration = Duration::from_millis(20);
fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}
fn short(value: impl std::fmt::Display) -> String {
    value.to_string().chars().take(1024).collect()
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Format {
    pub resolution: [u32; 2],
    pub frame_rate: [u32; 2],
}
impl Format {
    pub fn validate(&self) -> io::Result<()> {
        let [n, d] = self.frame_rate;
        if !(16..=1920).contains(&self.resolution[0])
            || !(16..=1080).contains(&self.resolution[1])
            || d == 0
            || u64::from(n) < u64::from(d)
            || u64::from(n) > 120 * u64::from(d)
        {
            return Err(invalid("invalid native camera dimensions/frame rate"));
        }
        Ok(())
    }
    fn matches(&self, other: &Self) -> bool {
        self.resolution == other.resolution
            && u64::from(self.frame_rate[0]) * u64::from(other.frame_rate[1])
                == u64::from(other.frame_rate[0]) * u64::from(self.frame_rate[1])
    }
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Spec {
    pub camera: String,
    pub format: Format,
    pub freshness_us: u64,
    pub run_id: String,
}
impl Spec {
    pub fn validate(&self) -> io::Result<()> {
        self.format.validate()?;
        if self.camera.is_empty()
            || self.camera.len() > 16_000
            || self.camera.contains('\0')
            || self.run_id.is_empty()
            || self.run_id.len() > 64
            || !self
                .run_id
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'-')
            || !(50_000..=1_000_000).contains(&self.freshness_us)
        {
            return Err(invalid("invalid camera selection/freshness/run"));
        }
        Ok(())
    }
}
#[derive(Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Message {
    Ready {
        run_id: String,
        native_capture: bool,
        format: Format,
    },
    Frame {
        sequence: u32,
        source_us: u64,
        length: usize,
    },
    Closed {
        run_id: String,
        error: Option<String>,
    },
}
fn encode(value: &impl Serialize) -> io::Result<Vec<u8>> {
    let mut bytes = serde_json::to_vec(value)?;
    bytes.push(b'\n');
    if bytes.len() > MAX_HEADER {
        return Err(invalid("camera header exceeds8KiB"));
    }
    Ok(bytes)
}
fn read_message(input: &mut impl BufRead) -> io::Result<Option<Message>> {
    let mut bytes = Vec::new();
    let count = Read::take(input, MAX_HEADER as u64 + 1).read_until(b'\n', &mut bytes)?;
    if count == 0 {
        return Ok(None);
    }
    if count > MAX_HEADER {
        return Err(invalid("camera header exceeds8KiB"));
    }
    Ok(Some(serde_json::from_slice(&bytes)?))
}
fn write_message(output: &mut impl Write, message: &Message) -> io::Result<()> {
    output.write_all(&encode(message)?)?;
    output.flush()
}
fn write_frame(
    output: &mut impl Write,
    sequence: u32,
    source_us: u64,
    bytes: &[u8],
) -> io::Result<()> {
    if !(4..=MAX_JPEG_BYTES).contains(&bytes.len()) {
        return Err(invalid("invalid encoded camera frame length"));
    }
    output.write_all(&encode(&Message::Frame {
        sequence,
        source_us,
        length: bytes.len(),
    })?)?;
    output.write_all(bytes)?;
    output.flush()
}
fn jpeg(bytes: &[u8], format: Format) -> io::Result<()> {
    if !bytes.starts_with(&[0xff, 0xd8, 0xff]) {
        return Err(invalid("camera payload is not native JPEG"));
    }
    let actual = image::ImageReader::new(io::Cursor::new(bytes))
        .with_guessed_format()?
        .into_dimensions()
        .map_err(io::Error::other)?;
    if [actual.0, actual.1] != format.resolution {
        return Err(invalid(
            "camera JPEG dimensions differ from negotiated native format",
        ));
    }
    Ok(())
}
/// QPC-epoch driver timestamp,100ns -> microseconds. No stream-relative fallback.
fn driver_us(timestamp: u64) -> io::Result<u64> {
    if timestamp < 10 {
        return Err(invalid("missing/zero driver timestamp"));
    }
    Ok(timestamp / 10)
}
#[derive(Default)]
struct Guard {
    last: Option<(u32, u64)>,
}
impl Guard {
    fn fresh(&mut self, sequence: u32, source: u64, now: u64, budget: u64) -> io::Result<bool> {
        if sequence == 0
            || source == 0
            || source > now
            || self.last.is_some_and(|(s, t)| sequence <= s || source <= t)
        {
            return Err(invalid("future, duplicate or regressing camera frame"));
        }
        self.last = Some((sequence, source));
        Ok(now - source < budget)
    }
}
struct EofControl {
    stop: Arc<AtomicBool>,
    done: Arc<AtomicBool>,
}
impl Drop for EofControl {
    fn drop(&mut self) {
        self.done.store(true, Ordering::Release);
    }
}
fn eof_stop() -> EofControl {
    let stop = Arc::new(AtomicBool::new(false));
    let done = Arc::new(AtomicBool::new(false));
    let signal = stop.clone();
    let finished = done.clone();
    thread::spawn(move || {
        let _ = io::stdin().read(&mut [0]);
        signal.store(true, Ordering::Relaxed);
        // Survives a hard parent exit even when a driver never returns to check
        // the stop flag. This is the dedicated camera child, never the UI owner.
        thread::sleep(Duration::from_millis(600));
        if !finished.load(Ordering::Acquire) {
            // ExitProcess may deadlock in DLL detach when another thread owns
            // a driver/DLL lock. Terminate only this dedicated capture worker.
            use ::windows::Win32::System::Threading::{GetCurrentProcess, TerminateProcess};
            unsafe {
                let _ = TerminateProcess(GetCurrentProcess(), 24);
            }
            std::process::abort(); // The successful self-termination never returns.
        }
    });
    EofControl { stop, done }
}
/// Internal manually invoked capture child. No input, recording or renderer.
pub fn stream(spec: Spec) -> io::Result<()> {
    spec.validate()?;
    let stop = eof_stop();
    let mut output = io::stdout().lock();
    let mut source = None;
    let result = (|| -> io::Result<()> {
        source = Some(native::Reader::open(&spec)?);
        let source = source.as_mut().unwrap();
        write_message(
            &mut output,
            &Message::Ready {
                run_id: spec.run_id.clone(),
                native_capture: true,
                format: source.format,
            },
        )?;
        let mut sequence = 0u32;
        while !stop.stop.load(Ordering::Relaxed) {
            if let Some((stamp, bytes)) = source.next()? {
                let stamp = driver_us(stamp)?;
                sequence = sequence
                    .checked_add(1)
                    .ok_or_else(|| invalid("capture sequence exhausted"))?;
                write_frame(&mut output, sequence, stamp, &bytes)?;
            }
        }
        Ok(())
    })();
    let cleanup = source.as_ref().map_or(Ok(()), native::Reader::close);
    let error = match (&result, &cleanup) {
        (Err(e), Err(close)) => Some(short(format!("{e}; camera close failed: {close}"))),
        (Err(e), _) | (_, Err(e)) => Some(short(e)),
        _ => None,
    };
    let terminal = write_message(
        &mut output,
        &Message::Closed {
            run_id: spec.run_id,
            error,
        },
    );
    result.and(cleanup).and(terminal)
}
#[derive(Clone, Debug)]
pub struct Status {
    pub negotiated: Option<Format>,
    pub native_capture: bool,
    pub received: u64,
    pub fresh_frames: u64,
    pub dropped: u64,
    pub process_exited: bool,
    pub error: Option<String>,
}
impl Status {
    /// None means opening or unconfirmed closure, not an invented off state.
    pub fn camera_opened(&self) -> Option<bool> {
        if self.process_exited {
            Some(false)
        } else if self.negotiated.is_some() && self.error.is_none() {
            Some(true)
        } else {
            None
        }
    }
}
struct Slot {
    status: Status,
    frame: Option<Frame>,
    last_fresh: Instant,
    ready: bool,
    closing: Option<Instant>,
    terminal: bool,
}
type Shared = Arc<(Mutex<Slot>, Condvar)>;
pub struct Capture {
    pid: u32,
    input: Option<ChildStdin>,
    shared: Shared,
    monitor: Option<JoinHandle<()>>,
    budget: u64,
}
impl Capture {
    /// Called on explicit Start after model warmup, never merely for app launch.
    pub fn spawn(executable: &Path, spec: Spec) -> io::Result<Self> {
        Self::start(executable, spec, None)
    }
    #[doc(hidden)]
    pub fn spawn_fixture(executable: &Path, spec: Spec, fault: fixture::Fault) -> io::Result<Self> {
        Self::start(executable, spec, Some(fault))
    }
    fn start(executable: &Path, spec: Spec, fault: Option<fixture::Fault>) -> io::Result<Self> {
        spec.validate()?;
        let mut command = Command::new(executable);
        command
            .arg(if fault.is_some() {
                "windows-camera-fixture"
            } else {
                "windows-camera-stream"
            })
            .arg(String::from_utf8(encode(&spec)?).map_err(io::Error::other)?)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .creation_flags(CREATE_NO_WINDOW);
        if let Some(fault) = fault {
            command.arg(serde_json::to_string(&fault)?);
        }
        let mut child = command.spawn()?;
        let pid = child.id();
        let input = child.stdin.take();
        let output = child.stdout.take().expect("piped camera output");
        let shared = Arc::new((
            Mutex::new(Slot {
                status: Status {
                    negotiated: None,
                    native_capture: fault.is_none(),
                    received: 0,
                    fresh_frames: 0,
                    dropped: 0,
                    process_exited: false,
                    error: None,
                },
                frame: None,
                last_fresh: Instant::now(),
                ready: false,
                closing: None,
                terminal: false,
            }),
            Condvar::new(),
        ));
        let reader_shared = shared.clone();
        let reader_spec = spec.clone();
        let reader = thread::spawn(move || {
            if let Err(error) = receive(
                BufReader::new(output),
                &reader_spec,
                fault.is_none(),
                &reader_shared,
            ) {
                reader_shared
                    .0
                    .lock()
                    .unwrap()
                    .status
                    .error
                    .get_or_insert(short(error));
                reader_shared.1.notify_all();
            }
        });
        let monitor_shared = shared.clone();
        let monitor = thread::spawn(move || supervise(child, reader, monitor_shared));
        let capture = Self {
            pid,
            input,
            shared,
            monitor: Some(monitor),
            budget: spec.freshness_us,
        };
        let state = capture.shared.0.lock().unwrap();
        let (state, _) = capture
            .shared
            .1
            .wait_timeout_while(state, STARTUP + Duration::from_secs(1), |s| {
                !s.ready && s.status.error.is_none() && !s.status.process_exited
            })
            .unwrap();
        if !state.ready || state.status.error.is_some() || state.status.process_exited {
            let error = state
                .status
                .error
                .clone()
                .unwrap_or_else(|| "camera did not become ready".into());
            drop(state);
            return Err(invalid(error));
        }
        drop(state);
        Ok(capture)
    }
    pub fn pid(&self) -> u32 {
        self.pid
    }
    pub fn status(&self) -> Status {
        self.shared.0.lock().unwrap().status.clone()
    }
    pub fn take_frame(&self) -> io::Result<Option<Frame>> {
        let now = monotonic_us()?;
        let mut state = self.shared.0.lock().unwrap();
        if let Some(error) = &state.status.error {
            return Err(invalid(error.clone()));
        }
        if state.status.process_exited {
            return Ok(None);
        }
        if let Some(frame) = state.frame.take() {
            if now >= frame.source_us && now - frame.source_us < self.budget {
                return Ok(Some(frame));
            }
            state.status.dropped = state.status.dropped.saturating_add(1);
        }
        Ok(None)
    }
    pub fn stop(mut self) -> Status {
        self.close();
        self.status()
    }
    fn close(&mut self) {
        self.input.take();
        self.shared
            .0
            .lock()
            .unwrap()
            .closing
            .get_or_insert_with(Instant::now);
        if let Some(monitor) = self.monitor.take() {
            if monitor.join().is_err() {
                self.shared.0.lock().unwrap().status.error =
                    Some("camera supervisor panicked; closure unconfirmed".into());
            }
        }
    }
}
impl Drop for Capture {
    fn drop(&mut self) {
        self.close();
    }
}
fn receive(mut input: impl BufRead, spec: &Spec, native: bool, shared: &Shared) -> io::Result<()> {
    let mut guard = Guard::default();
    while let Some(message) = read_message(&mut input)? {
        match message {
            Message::Ready {
                run_id,
                native_capture,
                format,
            } => {
                format.validate()?;
                let mut state = shared.0.lock().unwrap();
                if state.ready
                    || run_id != spec.run_id
                    || native_capture != native
                    || !format.matches(&spec.format)
                {
                    return Err(invalid("unexpected camera readiness/format"));
                }
                state.ready = true;
                state.status.negotiated = Some(format);
                state.last_fresh = Instant::now();
                shared.1.notify_all();
            }
            Message::Frame {
                sequence,
                source_us,
                length,
            } => {
                if !(4..=MAX_JPEG_BYTES).contains(&length) || !shared.0.lock().unwrap().ready {
                    return Err(invalid("unprepared or oversized camera frame"));
                }
                let mut bytes = vec![0; length];
                input.read_exact(&mut bytes)?;
                jpeg(&bytes, spec.format)?;
                let fresh = guard.fresh(sequence, source_us, monotonic_us()?, spec.freshness_us);
                let mut state = shared.0.lock().unwrap();
                state.status.received = state.status.received.saturating_add(1);
                let fresh = fresh?;
                if !fresh {
                    state.status.dropped = state.status.dropped.saturating_add(1);
                    continue;
                }
                if state.frame.is_some() {
                    state.status.dropped = state.status.dropped.saturating_add(1);
                }
                state.frame = Some(Frame {
                    bytes,
                    source_us,
                    sequence,
                });
                state.status.fresh_frames = state.status.fresh_frames.saturating_add(1);
                state.last_fresh = Instant::now();
            }
            Message::Closed { run_id, error } => {
                if run_id != spec.run_id {
                    return Err(invalid("camera close belongs to another run"));
                }
                let mut state = shared.0.lock().unwrap();
                state.terminal = true;
                if let Some(error) = error {
                    state.status.error.get_or_insert(short(error));
                }
                shared.1.notify_all();
                return Ok(());
            }
        }
    }
    Err(invalid("camera pipe ended without a close receipt"))
}
fn supervise(mut child: Child, reader: JoinHandle<()>, shared: Shared) {
    let mut failure = None;
    let mut terminal_at = None;
    let exited = loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    failure = Some(format!("camera child exited: {status}"));
                }
                break true;
            }
            Err(e) => {
                failure = Some(short(e));
                break terminate(&mut child).is_ok();
            }
            Ok(None) => (),
        }
        let state = shared.0.lock().unwrap();
        if state.terminal {
            terminal_at.get_or_insert_with(Instant::now);
        }
        let close = state
            .closing
            .is_some_and(|t| t.elapsed() >= Duration::from_millis(400))
            || terminal_at.is_some_and(|t| t.elapsed() >= Duration::from_millis(400));
        let stale = state.last_fresh.elapsed()
            >= if state.ready && state.status.fresh_frames > 0 {
                SILENCE
            } else {
                STARTUP
            };
        if state.status.error.is_some() || close || stale {
            failure = Some(state.status.error.clone().unwrap_or_else(|| {
                if close {
                    "camera did not close within grace".into()
                } else {
                    "camera fresh-frame watchdog expired".into()
                }
            }));
            drop(state);
            match terminate(&mut child) {
                Ok(()) => break true,
                Err(e) => {
                    failure = Some(format!("camera termination unconfirmed: {e}"));
                    break false;
                }
            }
        }
        drop(state);
        thread::sleep(POLL);
    };
    if exited && reader.join().is_err() {
        failure = Some("camera reader panicked".into());
    }
    let mut state = shared.0.lock().unwrap();
    if !state.terminal {
        failure.get_or_insert_with(|| "camera exited without a close receipt".into());
    }
    if let Some(failure) = failure {
        state.status.error = Some(match state.status.error.take() {
            Some(original) => format!("{original}; {failure}"),
            None => failure,
        });
    }
    state.status.process_exited = exited;
    state.frame = None;
    shared.1.notify_all();
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn driver_timestamps_use_qpc_units_without_relabeling_arrival() {
        assert_eq!(driver_us(12_345_678).unwrap(), 1_234_567);
        assert!(driver_us(0).is_err());
        let mut guard = Guard::default();
        assert!(guard.fresh(1, 1_000_000, 1_020_000, 150_000).unwrap());
        assert!(!guard.fresh(2, 1_030_000, 1_180_000, 150_000).unwrap());
        assert!(guard.fresh(3, 1_190_000, 1_200_000, 150_000).unwrap());
        assert!(guard.fresh(4, 1_200_001, 1_200_000, 150_000).is_err());
        assert!(guard.fresh(3, 1_200_000, 1_200_000, 150_000).is_err());
        assert!(guard.fresh(4, 1_190_000, 1_200_000, 150_000).is_err());
    }
    #[test]
    fn native_format_comparison_is_exact_in_dimensions_and_rational_rate() {
        let full = Format {
            resolution: [1920, 1080],
            frame_rate: [30, 1],
        };
        assert!(full.matches(&Format {
            frame_rate: [60, 2],
            ..full
        }));
        assert!(!full.matches(&Format {
            frame_rate: [30000, 1001],
            ..full
        }));
        assert!(!full.matches(&Format {
            resolution: [1080, 1080],
            ..full
        }));
        for format in [
            Format {
                frame_rate: [30, 0],
                ..full
            },
            Format {
                resolution: [1921, 1080],
                ..full
            },
            Format {
                frame_rate: [121, 1],
                ..full
            },
        ] {
            assert!(format.validate().is_err());
        }
    }
    #[test]
    fn packet_headers_and_payload_lengths_are_bounded_without_allocating_video() {
        assert!(read_message(&mut io::Cursor::new(vec![b'x'; MAX_HEADER + 1])).is_err());
        let ready = Message::Ready {
            run_id: "fixture".into(),
            native_capture: false,
            format: Format {
                resolution: [1920, 1080],
                frame_rate: [30, 1],
            },
        };
        let mut bytes = encode(&ready).unwrap();
        bytes.extend(
            encode(&Message::Frame {
                sequence: 1,
                source_us: 1,
                length: 8,
            })
            .unwrap(),
        );
        bytes.extend(b"jpegdata");
        let mut input = io::Cursor::new(bytes);
        assert!(matches!(
            read_message(&mut input).unwrap(),
            Some(Message::Ready { .. })
        ));
        assert!(matches!(
            read_message(&mut input).unwrap(),
            Some(Message::Frame { length: 8, .. })
        ));
        let mut payload = [0; 8];
        input.read_exact(&mut payload).unwrap();
        assert_eq!(&payload, b"jpegdata");
        assert!(read_message(&mut input).unwrap().is_none());
        assert!(write_frame(&mut Vec::new(), 1, 1, &[0; 3]).is_err());
    }
}
