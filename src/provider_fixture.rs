//! Shared finite worker transport; explicit synthetic and manually enabled camera sources.
use crate::acquisition::prepare_acquisition;
use crate::{
    camera_frame::Frame,
    controller,
    platform::monotonic_us,
    preview::{Extension, ExtensionResponse, Preview},
    provider::{self, Landmarks},
    session_config::SessionConfig,
    touch_gestures,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    error::Error,
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Condvar, Mutex,
    },
    thread,
    time::{Duration, Instant},
};

const MODEL: &str = "fbc2a30080c3c557093b5ddfc334698132eb341044ccee322ccf8bcf3607cde1";
#[derive(Clone)]
struct Lease {
    sequence: u64,
    issued_us: u64,
    frame: Arc<Frame>,
    recorded: Option<crate::recording::FrameIndex>,
}
struct Submission {
    lease: Lease,
    hands: Vec<Landmarks>,
    received_us: u64,
}
#[derive(Default)]
struct State {
    preparing: Option<Instant>,
    ready: bool,
    cancelled: bool,
    worker_error: Option<String>,
    stop_requested: bool,
    calibration_retry_requested: bool,
    closed: bool,
    finishing: bool,
    terminal_visible: bool,
    camera_opened: bool,
    camera_closure_unconfirmed: bool,
    recording_active: bool,
    input_started: bool,
    writer_reaped: bool,
    eof: bool,
    error: Option<String>,
    captured: u64,
    issued: u64,
    last_issued: u64,
    skipped: u64,
    expired: u64,
    captures: Vec<Value>,
    latest: Option<Lease>,
    lease: Option<Lease>,
    submission: Option<Submission>,
    receipt: Option<(u64, Value)>,
    worker_last_seen: Option<Instant>,
    acquisition_started: Option<Instant>,
    control_mode: Option<controller::Mode>,
    controller_running: bool,
    detail_active: bool,
    escape_feedback: Option<crate::macros::Feedback>,
    scroll_feedback: Option<crate::scroll::Feedback>,
}
impl State {
    fn camera_active(&self) -> Option<bool> {
        (!self.camera_closure_unconfirmed).then_some(self.camera_opened)
    }
    fn remaining_ms(&self, seconds: Option<u64>, now: Instant) -> Option<u64> {
        let (seconds, started) = (seconds?, self.acquisition_started?);
        Some(if self.eof {
            0
        } else {
            Duration::from_secs(seconds)
                .saturating_sub(now.saturating_duration_since(started))
                .as_millis() as u64
        })
    }
    fn control_output(&mut self, output: &controller::Output) {
        self.control_mode = Some(output.mode);
        self.controller_running = output.running;
        self.detail_active = output.detail.is_some();
        self.escape_feedback = output.escape_feedback.clone();
        self.scroll_feedback = output.scroll.clone();
    }
    fn worker_expired(&self, now: Instant) -> bool {
        self.ready
            && !self.eof
            && self
                .worker_last_seen
                .is_some_and(|last| now.saturating_duration_since(last) >= Duration::from_secs(5))
    }

    /// Acquisition history is retained separately from confirmed current input.
    fn system_input(&self) -> Option<bool> {
        if !self.input_started || self.writer_reaped {
            Some(false)
        } else if self.eof {
            None // A failed child wait does not prove that its device is gone.
        } else {
            Some(true)
        }
    }
}
struct Bridge {
    calibration: Option<Arc<Mutex<crate::provider_calibration::Run>>>,
    camera_source: bool,
    dimensions: [u32; 2],
    recorded_source: bool,
    input_requested: bool,
    warmup: Vec<u8>,
    max_hands: usize,
    presence_floor: f64,
    presence_scores: bool,
    absolute_pointer: bool,
    cursor_output: Option<crate::cursor::Config>,
    detail_configured: bool,
    escape_configured: bool,
    scroll_configured: bool,
    right_click: bool,
    manual_start: bool,
    trial_seconds: Option<u64>,
    run_id: String,
    state: Mutex<State>,
    changed: Condvar,
    assets: BTreeMap<String, PathBuf>,
    stale_us: u64,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Returned {
    run_id: String,
    sequence: u64,
    hands: Vec<Landmarks>,
}

fn reply(status: &'static str, body: Value) -> ExtensionResponse {
    ExtensionResponse {
        status,
        mime: "application/json",
        body: if status.starts_with("204") {
            Vec::new()
        } else {
            body.to_string().into_bytes()
        },
        headers: String::new(),
    }
}
impl Bridge {
    fn new(
        assets: BTreeMap<String, PathBuf>,
        stale_us: u64,
        run_id: String,
        max_hands: usize,
    ) -> Self {
        Self {
            calibration: None,
            camera_source: false,
            dimensions: [640, 480],
            recorded_source: false,
            input_requested: false,
            warmup: Vec::new(),
            max_hands,
            presence_floor: 0.5,
            presence_scores: false,
            absolute_pointer: false,
            cursor_output: None,
            detail_configured: false,
            escape_configured: false,
            scroll_configured: false,
            right_click: false,
            manual_start: false,
            trial_seconds: None,
            run_id,
            state: Mutex::new(State::default()),
            changed: Condvar::new(),
            assets,
            stale_us,
        }
    }
    fn publish(&self, frame: Frame) -> io::Result<u64> {
        self.publish_recorded(frame, None)
    }
    fn publish_recorded(
        &self,
        frame: Frame,
        recorded: Option<crate::recording::FrameIndex>,
    ) -> io::Result<u64> {
        let mut s = self.state.lock().unwrap();
        s.captured = s
            .captured
            .checked_add(1)
            .ok_or_else(|| io::Error::other("frame sequence exhausted"))?;
        let sequence = s.captured;
        let camera_opened = s.camera_active();
        let system_input = s.system_input();
        s.captures
            .push(json!({"event":"capture","sequence":sequence,
            "capture_sequence":frame.sequence,"source_us":frame.source_us,
            "camera_opened":camera_opened,"system_input":system_input,"recorded_frame":recorded}));
        s.latest = Some(Lease {
            sequence,
            issued_us: 0,
            frame: Arc::new(frame),
            recorded,
        });
        self.changed.notify_all();
        Ok(sequence)
    }
    fn finish(&self, sequence: u64, receipt: Value) {
        let mut s = self.state.lock().unwrap();
        s.lease = None;
        s.receipt = Some((sequence, receipt));
        self.changed.notify_all();
    }
    fn close_calibration_capture(&self) {
        let mut s = self.state.lock().unwrap();
        s.camera_opened = false;
        s.latest = None;
        s.lease = None;
        s.submission = None;
        s.preparing = None;
        s.last_issued = s.captured;
        self.changed.notify_all();
    }
    fn retire_calibration_frames(&self) {
        let mut s = self.state.lock().unwrap();
        // End HTTP leases before joining a capture thread that may still be
        // polling. Its receipt deadline must not turn normal closure into503.
        s.latest = None;
        s.lease = None;
        s.submission = None;
        s.last_issued = s.captured;
        self.changed.notify_all();
    }
}
impl Extension for Bridge {
    fn handles(&self, path: &str) -> bool {
        path.starts_with("/provider/")
            || path == "/task_hand_worker.js"
            || self.assets.contains_key(path)
    }
    fn respond(&self, method: &str, path: &str, body: &[u8]) -> io::Result<ExtensionResponse> {
        if method == "GET" {
            if let Some(file) = self.assets.get(path) {
                let bytes = fs::read(file)?;
                let mime = if path.ends_with(".wasm") {
                    "application/wasm"
                } else if path.ends_with(".js") || path.ends_with(".mjs") {
                    "text/javascript"
                } else {
                    "application/octet-stream"
                };
                return Ok(ExtensionResponse {
                    status: "200 OK",
                    mime,
                    body: bytes,
                    headers: String::new(),
                });
            }
        }
        match (method, path) {
            ("GET", "/provider/view") => {
                let page = include_str!("../tools/preview.html").replace(
                    "</html>",
                    "<script type=\"module\" src=\"/provider/client.mjs\"></script></html>",
                );
                Ok(ExtensionResponse {
                    status: "200 OK",
                    mime: "text/html; charset=utf-8",
                    body: page.into_bytes(),
                    headers: String::new(),
                })
            }
            ("GET", "/provider/feedback") => Ok(ExtensionResponse {
                status: "200 OK",
                mime: "text/html; charset=utf-8",
                body: include_bytes!("../tools/preview.html").to_vec(),
                headers: String::new(),
            }),
            ("GET", "/provider/worker") => Ok(ExtensionResponse {
                status: "200 OK",
                mime: "text/html; charset=utf-8",
                body: include_bytes!("../tools/provider_worker.html").to_vec(),
                headers: String::new(),
            }),
            ("GET", "/provider/control") => Ok(ExtensionResponse {
                status: "200 OK",
                mime: "text/html; charset=utf-8",
                body: include_bytes!("../tools/provider_control.html").to_vec(),
                headers: String::new(),
            }),
            ("GET", "/provider/calibration") if self.calibration.is_some() => {
                Ok(ExtensionResponse {
                    status: "200 OK",
                    mime: "text/html; charset=utf-8",
                    body: include_bytes!("../tools/provider_calibration.html").to_vec(),
                    headers: String::new(),
                })
            }
            ("GET", "/provider/client.mjs") => Ok(ExtensionResponse {
                status: "200 OK",
                mime: "text/javascript",
                body: include_bytes!("../tools/synthetic/provider_fixture_client.mjs").to_vec(),
                headers: String::new(),
            }),
            ("GET", "/provider/worker-tools.mjs") => Ok(ExtensionResponse {
                status: "200 OK",
                mime: "text/javascript",
                body: include_bytes!("../tools/synthetic/browser_hand_sequence.mjs").to_vec(),
                headers: String::new(),
            }),
            ("GET", "/provider/presence-tools.mjs") => Ok(ExtensionResponse {
                status: "200 OK",
                mime: "text/javascript",
                body: include_bytes!("../tools/synthetic/provider_presence.mjs").to_vec(),
                headers: String::new(),
            }),
            ("GET", "/task_hand_worker.js") => Ok(ExtensionResponse {
                status: "200 OK",
                mime: "text/javascript",
                body: include_bytes!("../tools/synthetic/task_hand_worker.js").to_vec(),
                headers: String::new(),
            }),
            ("GET", "/provider/config") => {
                let state = self.state.lock().unwrap();
                Ok(reply(
                    "200 OK",
                    json!({"model_sha256":MODEL,"delegate":"GPU",
                "run_id":self.run_id,"max_hands":self.max_hands,"presence_floor":self.presence_floor,"presence_scores":self.presence_scores,"absolute_pointer":self.absolute_pointer,"manual_start":self.manual_start,"trial_seconds":self.trial_seconds,"package_version":"1.1.1",
                "width":self.dimensions[0],"height":self.dimensions[1],"camera_source":self.camera_source,"recorded_source":self.recorded_source,"camera_opened":state.camera_active(),"recording_active":state.recording_active,
                "input_requested":self.input_requested,"system_input":state.system_input(),"cursor_output":self.cursor_output,
                "prepared":state.preparing.is_some(),"worker_ready":state.ready,"active":!state.eof,"error":state.error,
                "terminal_ready":state.finishing,
                "calibration":self.calibration.as_ref().map(|c|c.lock().unwrap().state(monotonic_us().unwrap_or(0))),
                "remaining_ms":state.remaining_ms(self.trial_seconds,Instant::now()),
                "mode":state.control_mode,"controller_running":state.controller_running&&!state.eof,
                "detail_configured":self.detail_configured,"detail_active":state.detail_active&&!state.eof,
                "right_click":self.right_click,"escape_configured":self.escape_configured,
                "escape_feedback":if state.eof {None}else{state.escape_feedback.as_ref()},
                "scroll_configured":self.scroll_configured,"scroll_feedback":if state.eof {None}else{state.scroll_feedback.as_ref()}}),
                ))
            }
            ("GET", "/provider/warmup") if !self.warmup.is_empty() => Ok(ExtensionResponse {
                status: "200 OK",
                mime: "image/jpeg",
                body: self.warmup.clone(),
                headers: "X-Wiggle-Synthetic: 1\r\n".into(),
            }),
            ("POST", "/provider/prepare")
            | ("POST", "/provider/ready")
            | ("POST", "/provider/stop")
            | ("POST", "/provider/terminal-visible") => {
                #[derive(Deserialize)]
                #[serde(deny_unknown_fields)]
                struct Run {
                    run_id: String,
                    #[serde(default)]
                    graceful: bool,
                    worker_error: Option<String>,
                }
                let run: Run = match serde_json::from_slice(body) {
                    Ok(v) => v,
                    Err(_) => {
                        return Ok(reply("400 Bad Request", json!({"error":"run ID required"})))
                    }
                };
                if run.run_id != self.run_id {
                    return Ok(reply(
                        "409 Conflict",
                        json!({"error":"different provider run"}),
                    ));
                }
                if run.graceful && path != "/provider/stop" {
                    return Ok(reply(
                        "400 Bad Request",
                        json!({"error":"graceful applies only to Stop"}),
                    ));
                }
                if run.worker_error.as_ref().is_some_and(|error| {
                    path != "/provider/stop"
                        || run.graceful
                        || error.is_empty()
                        || error.chars().count() > 512
                }) {
                    return Ok(reply(
                        "400 Bad Request",
                        json!({"error":"bounded worker failure applies only to cancellation"}),
                    ));
                }
                if path == "/provider/prepare" {
                    let mut state = self.state.lock().unwrap();
                    if state.eof
                        || state.preparing.is_some()
                        || (state.ready && self.calibration.is_none())
                    {
                        return Ok(reply(
                            "409 Conflict",
                            json!({"error":"preparation already requested"}),
                        ));
                    }
                    if let Some(calibration) = &self.calibration {
                        if let Err(error) = calibration.lock().unwrap().start() {
                            return Ok(reply("409 Conflict", json!({"error":error})));
                        }
                    }
                    state.preparing = Some(Instant::now());
                } else if path == "/provider/ready" {
                    let mut state = self.state.lock().unwrap();
                    if self.manual_start && state.preparing.is_none() {
                        return Ok(reply(
                            "409 Conflict",
                            json!({"error":"manual Start required"}),
                        ));
                    }
                    state.ready = true;
                    state.worker_last_seen = Some(Instant::now());
                } else if path == "/provider/terminal-visible" {
                    let mut state = self.state.lock().unwrap();
                    if !state.finishing {
                        return Ok(reply(
                            "409 Conflict",
                            json!({"error":"terminal feedback not published"}),
                        ));
                    }
                    state.terminal_visible = true;
                } else if run.graceful {
                    self.state.lock().unwrap().stop_requested = true;
                } else {
                    let mut state = self.state.lock().unwrap();
                    state.cancelled = true;
                    if state.worker_error.is_none() {
                        state.worker_error = run.worker_error;
                    }
                }
                self.changed.notify_all();
                Ok(reply("200 OK", json!({"accepted":true})))
            }
            ("POST", path) if path.starts_with("/provider/calibration/") => {
                #[derive(Deserialize)]
                #[serde(deny_unknown_fields)]
                struct Action {
                    run_id: String,
                    step: Option<usize>,
                }
                let action: Action = match serde_json::from_slice(body) {
                    Ok(action) => action,
                    Err(_) => {
                        return Ok(reply("400 Bad Request", json!({"error":"run ID required"})))
                    }
                };
                if action.run_id != self.run_id {
                    return Ok(reply(
                        "409 Conflict",
                        json!({"error":"different provider run"}),
                    ));
                }
                let Some(calibration) = &self.calibration else {
                    return Ok(reply(
                        "404 Not Found",
                        json!({"error":"calibration not selected"}),
                    ));
                };
                let mut state = self.state.lock().unwrap();
                if state.eof {
                    return Ok(reply("410 Gone", json!({"error":"session ended"})));
                }
                let mut calibration = calibration.lock().unwrap();
                let result = match (path, action.step) {
                    ("/provider/calibration/step", Some(step)) => {
                        calibration.acknowledge(step, monotonic_us()?)
                    }
                    ("/provider/calibration/save", None) => calibration.save(),
                    ("/provider/calibration/retry", None) => calibration.retry().map(|()| {
                        state.calibration_retry_requested = true;
                    }),
                    ("/provider/calibration/cancel", None) => calibration.cancel(),
                    _ => {
                        return Ok(reply(
                            "400 Bad Request",
                            json!({"error":"unknown calibration action/step"}),
                        ))
                    }
                };
                match result {
                    Ok(()) => Ok(reply("200 OK", json!({"accepted":true}))),
                    Err(error) => Ok(reply("409 Conflict", json!({"error":error}))),
                }
            }
            ("POST", "/provider/closed") => {
                #[derive(Deserialize)]
                #[serde(deny_unknown_fields)]
                struct Closed {
                    run_id: String,
                    acknowledged: bool,
                }
                let closed: Closed = match serde_json::from_slice(body) {
                    Ok(v) => v,
                    Err(_) => {
                        return Ok(reply(
                            "400 Bad Request",
                            json!({"error":"close acknowledgment required"}),
                        ))
                    }
                };
                if closed.run_id != self.run_id {
                    return Ok(reply(
                        "409 Conflict",
                        json!({"error":"different provider run"}),
                    ));
                }
                self.state.lock().unwrap().closed = closed.acknowledged;
                self.changed.notify_all();
                Ok(reply("200 OK", json!({"closed":closed.acknowledged})))
            }
            ("GET", "/provider/frame") => {
                let mut s = self.state.lock().unwrap();
                if !s.ready {
                    return Ok(reply("409 Conflict", json!({"error":"worker not ready"})));
                }
                if s.lease.is_some() {
                    return Ok(reply(
                        "409 Conflict",
                        json!({"error":"one frame already in flight"}),
                    ));
                }
                s.worker_last_seen = Some(Instant::now());
                let current = s.latest.clone().filter(|f| f.sequence > s.last_issued);
                if let Some(mut lease) = current {
                    let now = monotonic_us()?;
                    if now < lease.frame.source_us || now - lease.frame.source_us > self.stale_us {
                        s.skipped += lease.sequence.saturating_sub(s.last_issued);
                        s.last_issued = lease.sequence;
                        return Ok(reply(
                            if s.eof { "410 Gone" } else { "204 No Content" },
                            json!({"expired":true,"error":s.error}),
                        ));
                    }
                    s.skipped += lease.sequence.saturating_sub(s.last_issued + 1);
                    s.last_issued = lease.sequence;
                    s.issued += 1;
                    lease.issued_us = now;
                    s.lease = Some(lease.clone());
                    let ticket = json!({"run_id":self.run_id,"sequence":lease.sequence,"source_us":lease.frame.source_us,"issued_us":lease.issued_us,
                        "width":self.dimensions[0],"height":self.dimensions[1],"capture_sequence":lease.frame.sequence});
                    Ok(ExtensionResponse {
                        status: "200 OK",
                        mime: "image/jpeg",
                        body: lease.frame.bytes.clone(),
                        headers: format!("X-Wiggle-Source: {ticket}\r\n"),
                    })
                } else if s.eof {
                    Ok(reply("410 Gone", json!({"eof":true,"error":s.error})))
                } else {
                    Ok(reply("204 No Content", json!(null)))
                }
            }
            ("POST", "/provider/result") => {
                let returned: Returned = match serde_json::from_slice(body) {
                    Ok(v) => v,
                    Err(_) => {
                        return Ok(reply(
                            "400 Bad Request",
                            json!({"error":"invalid raw landmarks"}),
                        ))
                    }
                };
                if returned.run_id != self.run_id {
                    return Ok(reply(
                        "409 Conflict",
                        json!({"error":"different provider run"}),
                    ));
                }
                if returned.hands.len() > self.max_hands {
                    return Ok(reply(
                        "400 Bad Request",
                        json!({"error":"configured provider hand limit"}),
                    ));
                }
                let now = monotonic_us()?;
                let mut s = self.state.lock().unwrap();
                let Some(lease) = s.lease.clone().filter(|f| f.sequence == returned.sequence)
                else {
                    return Ok(reply(
                        "410 Gone",
                        json!({"error":"frame lease absent or expired"}),
                    ));
                };
                if s.submission.is_some() {
                    return Ok(reply(
                        "409 Conflict",
                        json!({"error":"result already pending"}),
                    ));
                }
                s.worker_last_seen = Some(Instant::now());
                if now < lease.frame.source_us || now - lease.frame.source_us > self.stale_us {
                    s.lease = None;
                    s.expired += 1;
                    return Ok(reply("410 Gone", json!({"error":"frame lease expired"})));
                }
                s.submission = Some(Submission {
                    lease,
                    hands: returned.hands,
                    received_us: now,
                });
                self.changed.notify_all();
                let until = Instant::now() + Duration::from_millis(200);
                loop {
                    if let Some((seq, result)) = &s.receipt {
                        if *seq == returned.sequence {
                            return Ok(reply("200 OK", result.clone()));
                        }
                    }
                    if s.lease
                        .as_ref()
                        .is_none_or(|lease| lease.sequence != returned.sequence)
                    {
                        return Ok(reply(
                            "410 Gone",
                            json!({"error":"frame lease closed before meaning receipt"}),
                        ));
                    }
                    let remaining = until.saturating_duration_since(Instant::now());
                    if remaining.is_zero() {
                        return Ok(reply(
                            "503 Service Unavailable",
                            json!({"error":"meaning receipt delayed"}),
                        ));
                    }
                    s = self.changed.wait_timeout(s, remaining).unwrap().0;
                }
            }
            _ => Ok(reply(
                "404 Not Found",
                json!({"error":"unknown provider route"}),
            )),
        }
    }
}

/// A verified manifest limits serving to selected public SDK/model files.
fn assets(root: &Path) -> Result<BTreeMap<String, PathBuf>, Box<dyn Error>> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Manifest {
        package_version: String,
        files: BTreeMap<String, String>,
    }
    let root = root.canonicalize()?;
    let manifest = root.join("provider-assets.json");
    if fs::metadata(&manifest)?.len() > 16384 {
        return Err("asset manifest exceeds16KiB".into());
    }
    let m: Manifest = serde_json::from_slice(&fs::read(manifest)?)?;
    if m.package_version != "1.1.1"
        || m.files.len() > 32
        || m.files.get("hand_landmarker.task").map(String::as_str) != Some(MODEL)
    {
        return Err("expected pinned Tasks1.1.1 model/asset manifest".into());
    }
    let mut files = BTreeMap::new();
    for (name, sha) in m.files {
        if !((name.starts_with("package/wasm/") && !name[13..].contains('/'))
            || name == "package/vision_bundle.mjs"
            || name == "hand_landmarker.task")
            || name.contains("..")
            || sha.len() != 64
            || !sha.bytes().all(|b| b.is_ascii_hexdigit())
        {
            return Err("unsupported asset name/digest".into());
        }
        let path = root.join(&name).canonicalize()?;
        // Explicitly selected public assets may be receiving-local symlinks.
        if !fs::metadata(&path)?.is_file() || fs::metadata(&path)?.len() > 64 * 1024 * 1024 {
            return Err("invalid provider asset".into());
        }
        use sha2::{Digest, Sha256};
        let mut file = fs::File::open(&path)?;
        let mut digest = Sha256::new();
        io::copy(&mut file, &mut digest)?;
        if format!("{:x}", digest.finalize()) != sha.to_ascii_lowercase() {
            return Err("provider asset digest differs".into());
        }
        files.insert(format!("/{name}"), path);
    }
    if !files.contains_key("/package/vision_bundle.mjs") {
        return Err("missing provider SDK".into());
    }
    Ok(files)
}

fn jpeg(path: &Path) -> Result<Vec<u8>, Box<dyn Error>> {
    if !fs::metadata(path)?.is_file() || fs::metadata(path)?.len() > 2 * 1024 * 1024 {
        return Err("expected bounded regular JPEG".into());
    }
    jpeg_bytes(fs::read(path)?)
}
fn jpeg_bytes(data: Vec<u8>) -> Result<Vec<u8>, Box<dyn Error>> {
    jpeg_bytes_for(data, [640, 480])
}
fn jpeg_bytes_for(data: Vec<u8>, dimensions: [u32; 2]) -> Result<Vec<u8>, Box<dyn Error>> {
    if !data.starts_with(&[0xff, 0xd8, 0xff]) {
        return Err("fixture requires prepared JPEG images".into());
    }
    if image::ImageReader::new(io::Cursor::new(&data))
        .with_guessed_format()?
        .into_dimensions()?
        != (dimensions[0], dimensions[1])
    {
        return Err("JPEG dimensions differ from selected source".into());
    }
    Ok(data)
}

pub fn run(
    manifest: &Path,
    session: &Path,
    settings: &Path,
    asset_root: &Path,
    port: u16,
    max_hands: usize,
) -> Result<(), Box<dyn Error>> {
    if !(1..=2).contains(&max_hands) {
        return Err("provider max hands must be1 or2".into());
    }
    let sequence = crate::image_sequence::Sequence::load(manifest)?;
    let first = sequence.frames.first().unwrap().source_us;
    let duration = sequence.frames.last().unwrap().source_us - first;
    if duration > 90_000_000 {
        return Err("fixture duration exceeds90seconds".into());
    }
    let root = manifest
        .canonicalize()?
        .parent()
        .ok_or("fixture root missing")?
        .to_path_buf();
    let mut frames = Vec::new();
    for f in sequence.frames {
        let path = f
            .image
            .map(|name| root.join(name).canonicalize())
            .transpose()?;
        if let Some(p) = &path {
            if !p.starts_with(&root) {
                return Err("fixture image escapes root".into());
            }
            jpeg(p)?;
        }
        frames.push((f.source_us - first, path));
    }
    let initial = frames
        .iter()
        .find_map(|(_, p)| p.as_ref())
        .ok_or("fixture has no images")?;
    let initial_jpeg = jpeg(initial)?;
    run_engine(RunOptions {
        output_target: None,
        calibration_profile: None,
        session,
        settings,
        asset_root,
        port,
        max_hands,
        initial_jpeg,
        source: Source::Fixture { frames, duration },
        preflight: false,
    })
}

/// Starts acquisition only after this manually launched session's worker is ready.
/// Automatic trials require fresh human readiness before launch. Manual trials
/// remain idle until Start; obtain fresh human readiness before that action.
pub struct CameraOptions<'a> {
    pub session: &'a Path,
    pub settings: &'a Path,
    pub asset_root: &'a Path,
    pub synthetic_warmup: &'a Path,
    pub seconds: u64,
    pub port: u16,
    pub max_hands: usize,
    pub inject: bool,
    pub preflight: bool,
    pub manual_start: bool,
}

pub fn run_camera(options: CameraOptions<'_>) -> Result<(), Box<dyn Error>> {
    if options.seconds > 90 {
        return Err("camera trial duration must be0(until-stop)..90 seconds".into());
    }
    run_engine(RunOptions {
        output_target: None,
        calibration_profile: None,
        session: options.session,
        settings: options.settings,
        asset_root: options.asset_root,
        port: options.port,
        max_hands: options.max_hands,
        initial_jpeg: jpeg(options.synthetic_warmup)?,
        source: Source::Camera {
            seconds: options.seconds,
            inject: options.inject,
            manual_start: options.manual_start || options.seconds == 0,
        },
        preflight: options.preflight,
    })
}

/// Windows uses the same service, worker, calibration and gesture loop. Selection
/// is explicit and checked before preflight; construction does not acquire IO.
#[cfg(target_os = "windows")]
pub fn run_windows(
    options: CameraOptions<'_>,
    output_target: &str,
    calibration_profile: Option<&Path>,
) -> Result<(), Box<dyn Error>> {
    if options.seconds > 90 || options.max_hands != 1 {
        return Err("Windows preview requires one hand and 0(until-Stop)..90 seconds".into());
    }
    if calibration_profile.is_some() && options.inject {
        return Err("calibration is camera-only".into());
    }
    run_engine(RunOptions {
        output_target: Some(output_target),
        calibration_profile,
        session: options.session,
        settings: options.settings,
        asset_root: options.asset_root,
        port: options.port,
        max_hands: 1,
        initial_jpeg: jpeg(options.synthetic_warmup)?,
        source: Source::Camera {
            seconds: if calibration_profile.is_some() {
                150
            } else {
                options.seconds
            },
            inject: options.inject,
            manual_start: true,
        },
        preflight: options.preflight,
    })
}

/// Independent guided calibration, manual Start, camera-only and bounded to150s
/// per attempt. Completion closes capture before explicit Save/Retry/Cancel.
pub fn calibrate(
    session: &Path,
    settings: &Path,
    asset_root: &Path,
    warmup: &Path,
    profile: &Path,
    port: u16,
    preflight: bool,
) -> Result<(), Box<dyn Error>> {
    run_engine(RunOptions {
        output_target: None,
        calibration_profile: Some(profile),
        session,
        settings,
        asset_root,
        port,
        max_hands: 1,
        initial_jpeg: jpeg(warmup)?,
        preflight,
        source: Source::Camera {
            seconds: 150,
            inject: false,
            manual_start: true,
        },
    })
}

pub struct RecordingOptions<'a> {
    pub recording: &'a Path,
    pub session: &'a Path,
    pub settings: &'a Path,
    pub asset_root: &'a Path,
    pub synthetic_warmup: &'a Path,
    pub start_ms: u64,
    pub duration_ms: u64,
    pub port: u16,
    pub max_hands: usize,
}
pub fn run_recording(options: RecordingOptions<'_>) -> Result<(), Box<dyn Error>> {
    if !(1..=90_000).contains(&options.duration_ms) {
        return Err("recorded replay duration must be1..90000ms".into());
    }
    let manifest = options.recording.join("manifest.json");
    if fs::metadata(&manifest)?.len() > 16 * 1024 {
        return Err("recording manifest exceeds16KiB".into());
    }
    let metadata: Value = serde_json::from_slice(&fs::read(manifest)?)?;
    if metadata["schema"] != "wigglecontrol.recording/v1" {
        return Err("recorded replay requires recording/v1".into());
    }
    let dimensions: [u32; 2] =
        serde_json::from_value(json!([metadata["width"], metadata["height"]]))?;
    crate::session_config::validate_camera_resolution(dimensions)?;
    let reader = crate::recording::Reader::open(options.recording)?;
    let epoch = reader.rows()[0].source_us;
    let start = options
        .start_ms
        .checked_mul(1000)
        .and_then(|n| epoch.checked_add(n))
        .ok_or("recording start overflow")?;
    let end = start
        .checked_add(options.duration_ms * 1000)
        .ok_or("recording end overflow")?;
    let indices: Vec<_> = reader
        .rows()
        .iter()
        .enumerate()
        .filter_map(|(i, r)| (r.source_us >= start && r.source_us < end).then_some(i))
        .collect();
    let first = *indices
        .first()
        .ok_or("recording window contains no frames")?;
    let last = *indices.last().unwrap();
    let duration = reader.rows()[last].source_us - reader.rows()[first].source_us;
    run_engine(RunOptions {
        output_target: None,
        calibration_profile: None,
        session: options.session,
        settings: options.settings,
        asset_root: options.asset_root,
        port: options.port,
        max_hands: options.max_hands,
        initial_jpeg: jpeg(options.synthetic_warmup)?,
        source: Source::Recorded {
            reader,
            indices,
            duration,
            dimensions,
        },
        preflight: false,
    })
}

enum Source {
    Fixture {
        frames: Vec<(u64, Option<PathBuf>)>,
        duration: u64,
    },
    Camera {
        seconds: u64,
        inject: bool,
        manual_start: bool,
    },
    Recorded {
        reader: crate::recording::Reader,
        indices: Vec<usize>,
        duration: u64,
        dimensions: [u32; 2],
    },
}
struct RunOptions<'a> {
    calibration_profile: Option<&'a Path>,
    session: &'a Path,
    settings: &'a Path,
    asset_root: &'a Path,
    port: u16,
    max_hands: usize,
    initial_jpeg: Vec<u8>,
    source: Source,
    preflight: bool,
    output_target: Option<&'a str>,
}
/// Keep startup factories camera-free in tests; the returned writer is owned by
/// the normal runtime cleanup, independently of an optional dataset recorder.
struct Journal<'a> {
    verbose: bool,
    stdout: std::io::StdoutLock<'a>,
    recording: Option<crate::recording::Output>,
}
impl Journal<'_> {
    fn emit(&mut self, value: Value) -> io::Result<()> {
        let line = crate::recording::json_line(&value)?;
        if let Some(file) = &mut self.recording {
            file.write_all(&line)?;
        }
        if self.verbose
            || matches!(
                value["event"].as_str(),
                Some("listening" | "stop" | "calibration_capture_closed")
            )
        {
            self.stdout.write_all(&line)?;
            self.stdout.flush()?;
        }
        Ok(())
    }
}

fn run_engine(options: RunOptions<'_>) -> Result<(), Box<dyn Error>> {
    run_engine_with_assets(options, assets)
}

fn run_engine_with_assets(
    options: RunOptions<'_>,
    verify_assets: impl FnOnce(&Path) -> Result<BTreeMap<String, PathBuf>, Box<dyn Error>>,
) -> Result<(), Box<dyn Error>> {
    let RunOptions {
        output_target,
        calibration_profile,
        session,
        settings,
        asset_root,
        port,
        max_hands,
        initial_jpeg,
        source,
        preflight,
    } = options;
    if !(1..=2).contains(&max_hands) {
        return Err("provider max hands must be1 or2".into());
    }
    let (config, mut controller, mut tracker) = provider::configured(session, settings, None)?;
    let presence_floor = config.gestures().min_confidence;
    if config.provider_absolute_pointer && max_hands != 1 {
        return Err("absolute provider requires exactly one hand".into());
    }
    let absolute_workspace = config
        .provider_absolute_pointer
        .then_some(config.touch_workspace);
    let camera_source = matches!(&source, Source::Camera { .. });
    let recorded_source = matches!(&source, Source::Recorded { .. });
    let manual_start = matches!(
        &source,
        Source::Camera {
            manual_start: true,
            ..
        }
    );
    let inject = matches!(&source, Source::Camera { inject: true, .. });
    let calibration = if let Some(profile) = calibration_profile {
        if !camera_source || !manual_start || inject || max_hands != 1 {
            return Err("provider calibration requires manual camera-only single-hand mode".into());
        }
        Some(Arc::new(Mutex::new(crate::provider_calibration::Run::new(
            profile,
            config
                .pointer_output
                .ok_or("calibration requires explicit pointer_output dimensions")?,
        )?)))
    } else {
        None
    };
    let recording_epoch = match &source {
        Source::Recorded { reader, .. } => Some(reader.rows()[0].source_us),
        _ => None,
    };
    let (mut frames, duration, mut playback, dimensions) = match source {
        Source::Fixture { frames, duration } => (frames, duration, None, [640, 480]),
        Source::Camera { seconds, .. } => (
            Vec::new(),
            seconds * 1_000_000,
            None,
            config.camera_resolution,
        ),
        Source::Recorded {
            reader,
            indices,
            duration,
            dimensions,
        } => (Vec::new(), duration, Some((reader, indices)), dimensions),
    };
    let until_stop = camera_source && duration == 0;
    if camera_source {
        config.validate_recording((!until_stop).then_some(duration / 1_000_000))?;
    }
    #[cfg(target_os = "windows")]
    if matches!(&source, Source::Camera { .. }) {
        crate::session_io::validate_output(
            &config,
            output_target.ok_or("select an explicit Windows output monitor")?,
        )?;
        if !matches!(
            &source,
            Source::Camera {
                manual_start: true,
                ..
            }
        ) {
            return Err("Windows camera sessions require manual Start".into());
        }
        if config.recording_enabled {
            return Err(
                "Windows preview calibration/mouse use must have recording disabled".into(),
            );
        }
    }
    let mut bridge = Bridge::new(
        verify_assets(asset_root)?,
        config.gestures().stale_us,
        format!("{}-{}", std::process::id(), monotonic_us()?),
        max_hands,
    );
    bridge.calibration = calibration.clone();
    bridge.dimensions = dimensions;
    bridge.presence_floor = presence_floor;
    bridge.presence_scores = config.provider_presence_scores;
    bridge.absolute_pointer = config.provider_absolute_pointer;
    bridge.cursor_output = config.pointer_cursor_output;
    bridge.detail_configured = controller.detail_config().is_some();
    bridge.escape_configured = controller.escape_config().is_some();
    bridge.scroll_configured = controller.scroll_config().is_some();
    bridge.right_click = controller.right_click_enabled();
    bridge.manual_start = manual_start;
    bridge.trial_seconds = (camera_source && !until_stop).then_some(duration / 1_000_000);
    bridge.camera_source = camera_source;
    bridge.recorded_source = recorded_source;
    bridge.input_requested = inject;
    bridge.warmup = initial_jpeg.clone();
    if preflight {
        println!(
            "{}",
            json!({"preflight":"ok","camera_resolution":dimensions,"camera_opened":false,"system_input":false,"calibration":calibration.is_some(),
            "until_stop":until_stop,"manual_start":manual_start,"trial_seconds":bridge.trial_seconds,"absolute_pointer":config.provider_absolute_pointer,"recording_configured":config.recording_enabled && config.recording_directory.is_some(),"max_hands":max_hands,
            "model_sha256":MODEL,"presence_floor":presence_floor,"presence_scores":config.provider_presence_scores,"detail":controller.detail_config(),"escape":controller.escape_config(),"scroll":controller.scroll_config(),"right_click":controller.right_click_enabled(),"right_click_dominance_ratio":controller.right_click_dominance_ratio(),"visual_macros":controller.visual_policy().is_some(),"input_requested":inject,"acquisition_started":false})
        );
        return Ok(());
    }
    let mut visual_classifier =
        crate::pose_classifier::VisualClassifier::with_runtime(&controller, &config.runtime)?;
    let bridge = Arc::new(bridge);
    let stop = Arc::new(AtomicBool::new(false));
    #[cfg(target_os = "linux")]
    {
        signal_hook::flag::register(signal_hook::consts::SIGTERM, stop.clone())?;
        signal_hook::flag::register(signal_hook::consts::SIGINT, stop.clone())?;
    }
    let preview = Preview::start(port, stop.clone())?;
    preview.enable_extension(bridge.clone());
    preview.enable_controller_controls();
    let mut journal = Journal {
        verbose: cfg!(target_os = "linux"),
        stdout: io::stdout().lock(),
        recording: None,
    };
    let capture_stop = Arc::new(AtomicBool::new(false));
    let camera_mailbox = Arc::new(Mutex::new(crate::session_io::Mailbox::default()));
    let mut writer = None;
    let mut publisher = crate::output::Publisher::with_cursor_output(
        config.pointer_cursor_output,
        config.gestures().stale_us,
    )?;
    journal.emit(
        json!({"event":"listening","port":preview.port,"schema":if camera_source {"wigglecontrol.provider-camera/v1"}else if recorded_source {"wigglecontrol.provider-recording/v1"}else{"wigglecontrol.provider-fixture/v1"},
        "camera_opened":false,"system_input":false,"input_requested":inject,"presence_scores":config.provider_presence_scores,"presence_floor":presence_floor,"source_kind":if camera_source {"kernel_monotonic_camera"}else if recorded_source {"recorded_frame_republication"}else{"synthetic_encoded_frame_publish"}}),
    )?;
    let mut feedback = Feedback {
        dimensions,
        jpeg: initial_jpeg,
        source_us: 0,
        raw: Vec::new(),
        sequence: 0,
        meaning_us: 0,
        recorded_time_ms: None,
        pinch_ratio: None,
        eligible: false,
        visual_macros: visual_classifier.is_some(),
        visual_pose: None,
        classifier_ms: 0.0,
    };
    let mut previous_active = (Vec::new(), false);
    let prepared = Instant::now();
    let mut started = None;
    let mut ended = None;
    let mut producer: Option<thread::JoinHandle<()>> = None;
    let mut calibration_closed_at: Option<Instant> = None;
    let mut last_publish = Instant::now() - Duration::from_secs(1);
    let mut feedback_snapshots = 0_u64;
    let outcome = (|| -> Result<(), Box<dyn Error>> {
        while !stop.load(Ordering::Relaxed) {
            {
                let state = bridge.state.lock().unwrap();
                if state.cancelled {
                    return Err(match &state.worker_error {
                        Some(error) => format!("provider worker requested cancellation: {error}"),
                        None => "provider worker requested cancellation".into(),
                    }
                    .into());
                }
                if state.stop_requested {
                    break;
                }
            }
            let mut now = monotonic_us()?;
            if let Some(calibration) = &calibration {
                if std::mem::take(&mut bridge.state.lock().unwrap().calibration_retry_requested) {
                    started = None;
                    calibration_closed_at = None;
                    capture_stop.store(false, Ordering::Relaxed);
                    *camera_mailbox.lock().unwrap() = Default::default();
                    bridge.state.lock().unwrap().acquisition_started = None;
                    tracker.clear();
                }
                {
                    let mut c = calibration.lock().unwrap();
                    c.tick(now);
                    if started
                        .is_some_and(|s: Instant| s.elapsed() >= Duration::from_micros(duration))
                    {
                        c.limit_reached();
                    }
                }
                let close_camera = {
                    let c = calibration.lock().unwrap();
                    c.finished() && !c.camera_closed()
                };
                if close_camera {
                    capture_stop.store(true, Ordering::Relaxed);
                    bridge.retire_calibration_frames();
                    if let Some(p) = producer.take() {
                        p.join()
                            .map_err(|_| "calibration camera producer panicked")?;
                    }
                    if let Some(error) = &camera_mailbox.lock().unwrap().error {
                        return Err(error.clone().into());
                    }
                    if camera_mailbox.lock().unwrap().unclosed {
                        return Err("camera closure unconfirmed".into());
                    }
                    bridge.close_calibration_capture();
                    calibration.lock().unwrap().confirm_camera_closed();
                    calibration_closed_at = Some(Instant::now());
                    journal.emit(json!({"event":"calibration_capture_closed","now_us":monotonic_us()?,
                        "camera_opened":false,"system_input":false,"calibration":calibration.lock().unwrap().state(monotonic_us()?)}))?;
                }
                if calibration.lock().unwrap().done()
                    || calibration_closed_at
                        .is_some_and(|at| at.elapsed() >= Duration::from_secs(600))
                {
                    break;
                }
            }
            if bridge.state.lock().unwrap().ready
                && started.is_none()
                && calibration
                    .as_ref()
                    .is_none_or(|c| c.lock().unwrap().running())
            {
                let (recording_frames, input_writer) = prepare_acquisition(
                    camera_source && config.recording_enabled,
                    camera_source && inject,
                    || -> Result<_, Box<dyn Error>> {
                        let base = Path::new(config.recording_directory.as_ref().unwrap());
                        let (frames, observations, dir) = crate::recording::create(
                            base,
                            monotonic_us()?,
                            &json!({"schema":"wigglecontrol.recording/v1","perception_backend":"tasks_gpu_worker",
                            "model_sha256":MODEL,"package_version":"1.1.1","max_hands":max_hands,
                            "presence_floor":presence_floor,"presence_scores":config.provider_presence_scores,"visual_macros":visual_classifier.is_some(),"confidence_kind":if config.provider_presence_scores {"presence_score_or_configured_floor"}else{"configured_threshold_lower_bound"},
                            "width":dimensions[0],"height":dimensions[1],"camera_fps":30,"freshness_ms":config.freshness_ms,
                            "input_requested":inject,"human_labels":"separate; predictions are not ground truth"}),
                        )?;
                        for (name, path) in [
                            ("config.json", session),
                            ("controller-settings.json", settings),
                        ] {
                            frames.metadata(name, &fs::read(path)?)?;
                        }
                        fs::write(
                            base.join("last-recording.path"),
                            dir.as_os_str().as_encoded_bytes(),
                        )?;
                        journal.recording = Some(observations);
                        Ok(frames)
                    },
                    || {
                        Ok(crate::session_io::Writer::start(
                            &config,
                            (!until_stop).then_some(duration / 1_000_000 + 3),
                            output_target,
                        )?)
                    },
                )?;
                writer = input_writer;
                if writer.is_some() {
                    bridge.state.lock().unwrap().input_started = true;
                }
                started = Some(Instant::now());
                {
                    let mut state = bridge.state.lock().unwrap();
                    state.acquisition_started = started;
                    state.recording_active = journal.recording.is_some();
                }
                let initial = controller.start_at(monotonic_us()?);
                crate::session_io::publish(
                    &mut writer,
                    &mut publisher,
                    &initial,
                    None,
                    absolute_workspace,
                )?;
                if camera_source {
                    producer = Some(crate::platform::camera(
                        &config,
                        capture_stop.clone(),
                        camera_mailbox.clone(),
                        recording_frames,
                        (!until_stop).then(|| started.unwrap() + Duration::from_micros(duration)),
                    ));
                } else {
                    let (shared, stopped) = (bridge.clone(), capture_stop.clone());
                    let source_frames = std::mem::take(&mut frames);
                    let mut recorded = playback.take();
                    producer = Some(thread::spawn(move || {
                        let begin = Instant::now();
                        let count = recorded
                            .as_ref()
                            .map_or(source_frames.len(), |(_, ids)| ids.len());
                        let recorded_first = recorded
                            .as_ref()
                            .map(|(reader, ids)| reader.rows()[ids[0]].source_us);
                        for i in 0..count {
                            let offset = recorded.as_ref().map_or_else(
                                || source_frames[i].0,
                                |(reader, ids)| {
                                    reader.rows()[ids[i]].source_us - recorded_first.unwrap()
                                },
                            );
                            while begin.elapsed().as_micros() < offset as u128
                                && !stopped.load(Ordering::Relaxed)
                            {
                                thread::sleep(Duration::from_millis(1));
                            }
                            if stopped.load(Ordering::Relaxed) {
                                break;
                            }
                            let next = (|| -> Result<(), Box<dyn Error>> {
                                if let Some((reader, ids)) = &mut recorded {
                                    let index = ids[i];
                                    let original = reader.rows()[index].clone();
                                    let bytes =
                                        jpeg_bytes_for(reader.frame(index)?, shared.dimensions)?;
                                    shared.publish_recorded(
                                        Frame {
                                            bytes,
                                            source_us: monotonic_us()?,
                                            sequence: index as u32,
                                        },
                                        Some(original),
                                    )?;
                                } else if let Some(path) = &source_frames[i].1 {
                                    let bytes = jpeg(path)?;
                                    shared.publish(Frame {
                                        bytes,
                                        source_us: monotonic_us()?,
                                        sequence: i as u32,
                                    })?;
                                }
                                Ok(())
                            })();
                            if let Err(error) = next {
                                shared.state.lock().unwrap().error = Some(error.to_string());
                                break;
                            }
                        }
                        shared.state.lock().unwrap().eof = true;
                        shared.changed.notify_all();
                    }));
                }
            }
            if camera_source
                && !until_stop
                && calibration.is_none()
                && started.is_some_and(|s| s.elapsed() >= Duration::from_micros(duration))
            {
                break;
            }
            if let Some(writer) = &mut writer {
                writer.check()?;
            }
            let camera_opened = if calibration_closed_at.is_some() {
                false
            } else {
                let mut mailbox = camera_mailbox.lock().unwrap();
                {
                    let mut state = bridge.state.lock().unwrap();
                    state.camera_opened = mailbox.opened && mailbox.unclosed;
                    state.camera_closure_unconfirmed = mailbox.unclosed && !mailbox.opened;
                }
                if let Some(error) = &mailbox.error {
                    return Err(error.clone().into());
                }
                if let Some(frame) = mailbox.frame.take() {
                    bridge.publish(frame)?;
                }
                mailbox.opened
            };
            let preparation_expired = if manual_start {
                bridge
                    .state
                    .lock()
                    .unwrap()
                    .preparing
                    .is_some_and(|t| t.elapsed() > Duration::from_secs(30))
            } else {
                prepared.elapsed() > Duration::from_secs(30)
            };
            if started.is_none() && preparation_expired {
                return Err("worker preparation timeout".into());
            }
            if !until_stop
                && calibration.is_none()
                && started.is_some_and(|s| {
                    s.elapsed() > Duration::from_micros(duration) + Duration::from_secs(5)
                })
            {
                return Err("fixture completion timeout".into());
            }
            if let Some(error) = bridge.state.lock().unwrap().error.clone() {
                return Err(error.into());
            }
            if bridge.state.lock().unwrap().worker_expired(Instant::now()) {
                return Err("worker disconnected: no frame requests/results for 5 seconds".into());
            }
            tracker.tick(now);
            for capture in std::mem::take(&mut bridge.state.lock().unwrap().captures) {
                journal.emit(capture)?;
            }
            let submission = bridge.state.lock().unwrap().submission.take();
            // Tick-only releases carry no new source. Reusing the previous source
            // would violate the native writer's strictly increasing frame contract.
            let delivery_source = submission.as_ref().map(|s| s.lease.frame.source_us);
            now = monotonic_us()?;
            let output = if let Some(submission) = submission {
                let lease = submission.lease;
                let packet = provider::Frame {
                    sequence: lease.sequence,
                    source_us: lease.frame.source_us,
                    now_us: now,
                    width: dimensions[0],
                    height: dimensions[1],
                    hands: submission.hands,
                };
                let found = tracker.observe(&packet)?;
                if let Some(calibration) = &calibration {
                    let calibration_now = monotonic_us()?;
                    let hand =
                        (!found.ambiguous && found.hands.len() == 1).then(|| &found.hands[0]);
                    let reason = if calibration_now.saturating_sub(packet.source_us) >= 150_000 {
                        "Calibration: frame delayed"
                    } else if let Some(hand) = hand {
                        if hand.confidence < presence_floor {
                            "Calibration: low hand confidence"
                        } else if !hand.in_view(dimensions[0], dimensions[1]) {
                            "Calibration: hand partly outside camera; move inward"
                        } else {
                            "Calibration: hand visible; hold steady"
                        }
                    } else {
                        "Calibration: no hand detected"
                    };
                    let point = hand
                        .filter(|_| reason == "Calibration: hand visible; hold steady")
                        .and_then(|h| {
                            config
                                .observation(
                                    Some(h),
                                    dimensions[0],
                                    dimensions[1],
                                    packet.source_us,
                                )
                                .hand
                                .map(|h| h.cursor)
                        });
                    calibration.lock().unwrap().observe(
                        point,
                        packet.source_us,
                        calibration_now,
                        reason,
                    );
                }

                let hands: Vec<_> = found
                    .hands
                    .iter()
                    .filter_map(|h| {
                        config
                            .observation(Some(h), dimensions[0], dimensions[1], packet.source_us)
                            .hand
                    })
                    .collect();
                let observation = touch_gestures::Frame {
                    version: 1,
                    source_us: packet.source_us,
                    ambiguous: found.ambiguous,
                    hands: hands.clone(),
                };
                let classifier_started = Instant::now();
                let (pose, visual_pose) = if let Some(classifier) = &mut visual_classifier {
                    classifier.classify(
                        &controller,
                        &observation,
                        &found.hands,
                        dimensions[0],
                        dimensions[1],
                    )?
                } else {
                    (None, None)
                };
                feedback.classifier_ms = classifier_started.elapsed().as_secs_f64() * 1000.0;
                feedback.visual_pose = visual_pose;
                // Classification is part of meaning latency and cannot extend freshness.
                now = monotonic_us()?;
                let state = crate::session_config::observe_controller(
                    &config,
                    &mut controller,
                    now,
                    observation,
                    pose,
                    &found.hands,
                    dimensions,
                );
                feedback.recorded_time_ms = lease.recorded.as_ref().and_then(|r| {
                    recording_epoch.map(|first| (r.source_us - first) as f64 / 1000.0)
                });
                feedback.source_us = packet.source_us;
                feedback.jpeg = lease.frame.bytes.clone();
                feedback.sequence = lease.sequence;
                feedback.meaning_us = monotonic_us()?;
                feedback.pinch_ratio = hands.first().map(crate::Hand::pinch_ratio);
                feedback.eligible = hands.iter().any(|h| h.engaged && h.pinch_eligible);
                feedback.raw=found.hands.iter().zip(packet.hands.iter()).map(|(h,raw)|json!({"presence_confidence":raw.presence_confidence,"id":h.id,"confidence":h.confidence,
                    "handedness":h.handedness,"landmarks":h.landmarks.map(|p|[p.x,p.y]),"world_landmarks":h.world_landmarks,"image_depths":h.image_depths})).collect();
                let receipt = json!({"sequence":lease.sequence,"source_us":packet.source_us,"now_us":now,
                    "age_ms":(feedback.meaning_us-packet.source_us)as f64/1000.0,"contacts":state.contacts,"mode":state.mode});
                bridge.finish(lease.sequence, receipt);
                journal.emit(
                    json!({"event":"frame","sequence":lease.sequence,"capture_sequence":lease.frame.sequence,
                    "source_us":packet.source_us,"issued_us":lease.issued_us,"received_us":submission.received_us,"now_us":now,"meaning_us":feedback.meaning_us,
                    "ambiguous":found.ambiguous,"ambiguity_reason":found.ambiguity_reason,"hands":hands,
                    "perception":feedback.raw,"visual_pose":feedback.visual_pose,"classifier_ms":feedback.classifier_ms,"visual_macros":feedback.visual_macros,"controller":state,"camera_opened":camera_opened,"system_input":inject,"recorded_frame":lease.recorded}),
                )?;
                state
            } else {
                controller.tick(now)
            };
            let handoff_us = monotonic_us()?;
            let (output, discarded) =
                controller.guard_delivery(output, handoff_us, delivery_source);
            crate::session_io::publish_at(
                &mut writer,
                &mut publisher,
                &output,
                delivery_source,
                absolute_workspace,
                handoff_us,
            )?;
            if discarded {
                journal.emit(json!({"event":"delivery_source_discarded",
                    "sequence":feedback.sequence,"source_us":delivery_source,
                    "now_us":handoff_us,"controller":output}))?;
            }
            let active = (
                output.contacts.iter().map(|c| c.id).collect::<Vec<_>>(),
                output.button_down,
            );
            if active != previous_active {
                journal.emit(
                    json!({"event":"intent_transition","now_us":now,"last_source_us":feedback.source_us,
                    "controller":output,"camera_opened":camera_opened,"system_input":inject}),
                )?;
                previous_active = active;
            }
            {
                let mut s = bridge.state.lock().unwrap();
                s.control_output(&output);
                if s.lease
                    .as_ref()
                    .is_some_and(|l| now.saturating_sub(l.frame.source_us) > bridge.stale_us)
                    && s.submission.is_none()
                {
                    let lease = s.lease.take().unwrap();
                    s.expired += 1;
                    journal.emit(json!({"event":"lease_expired","sequence":lease.sequence,
                        "source_us":lease.frame.source_us,"now_us":now,"camera_opened":camera_opened,"system_input":inject}))?;
                }
                if s.eof
                    && s.lease.is_none()
                    && s.latest
                        .as_ref()
                        .is_none_or(|f| f.sequence <= s.last_issued)
                    && ended.is_none()
                {
                    ended = Some(Instant::now());
                }
            }
            if last_publish.elapsed() > Duration::from_millis(40) && preview.requested_recently() {
                let remain = started.map_or(30000, |s| {
                    (duration / 1000 + if camera_source { 0 } else { 2000 })
                        .saturating_sub(s.elapsed().as_millis() as u64)
                });
                let mut data =
                    metadata(&config, &output, &feedback, now, remain, started.is_some());
                data["awaiting_start"] =
                    (manual_start && bridge.state.lock().unwrap().preparing.is_none()).into();
                data["until_stop"] = (until_stop && started.is_some()).into();
                data["trial_seconds"] = bridge.trial_seconds.into();
                data["ui_fixture"] = (!camera_source).into();
                data["synthetic_frame"] = (feedback.source_us == 0).into();
                data["camera_source"] = camera_source.into();
                data["recorded_source"] = recorded_source.into();
                data["recorded_time_ms"] = feedback.recorded_time_ms.into();
                data["camera_opened"] = camera_opened.into();
                data["provider_run_id"] = bridge.run_id.clone().into();
                data["input_available"] = (inject && writer.is_some()).into();
                data["input_enabled"] = (inject && writer.is_some() && output.running).into();
                data["recording"] = journal.recording.is_some().into();
                if let Some(cursor) = publisher.cursor_feedback() {
                    data["observed_screen_target"] = data["screen_target"].clone();
                    data["screen_target"] = serde_json::to_value(cursor.estimated)?;
                    data["cursor_output"] = serde_json::to_value(cursor)?;
                }
                preview.publish(feedback.jpeg.clone(), data)?;
                feedback_snapshots += 1;
                last_publish = Instant::now();
            }
            if ended.is_some_and(|s| s.elapsed() > Duration::from_secs(1)) {
                break;
            }
            if let Some(control) = preview.take_controller_control() {
                let control_time = monotonic_us()?;
                let result = match control {
                    crate::preview::ControllerControl::Mode(mode) => {
                        if config.provider_absolute_pointer
                            && !matches!(
                                mode,
                                crate::controller::Mode::Pointer | crate::controller::Mode::Scroll
                            )
                        {
                            continue;
                        }
                        controller.set_mode_at(mode, control_time)
                    }
                    crate::preview::ControllerControl::SelectHand(hand_id) => {
                        controller.select_hand_at(hand_id, control_time)
                    }
                    crate::preview::ControllerControl::Pause => controller.stop(),
                    crate::preview::ControllerControl::Resume => controller.start_at(control_time),
                };
                crate::session_io::publish(
                    &mut writer,
                    &mut publisher,
                    &result,
                    None,
                    absolute_workspace,
                )?;
                journal.emit(
                    json!({"event":"control","now_us":control_time,"controller":result,"camera_opened":camera_opened,"system_input":inject}),
                )?;
            }
            thread::sleep(Duration::from_millis(5));
        }
        Ok(())
    })();
    capture_stop.store(true, Ordering::Relaxed);
    let final_state = controller.stop();
    let release = crate::session_io::publish(
        &mut writer,
        &mut publisher,
        &final_state,
        None,
        absolute_workspace,
    );
    let writer_exit = writer.map(|writer| writer.stop(stop.load(Ordering::Relaxed)));
    let writer_reaped = writer_exit
        .as_ref()
        .is_none_or(|status| status.exited && status.neutral);
    let joined = producer
        .map(|p| p.join())
        .transpose()
        .map_err(|_| "frame producer panicked");
    let camera_closed = !camera_mailbox.lock().unwrap().unclosed;
    if camera_closed {
        if let Some(calibration) = &calibration {
            calibration.lock().unwrap().confirm_camera_closed();
        }
    }
    let outcome = outcome.and_then(|_| {
        joined?;
        if !camera_closed {
            return Err("camera closure unconfirmed".into());
        }
        release?;
        if let Some(status) = writer_exit {
            if let Some(error) = status.error {
                return Err(error.into());
            }
            if !status.exited || !status.neutral {
                return Err("input cleanup unconfirmed".into());
            }
        }
        Ok(())
    });
    {
        let mut s = bridge.state.lock().unwrap();
        s.eof = true;
        s.error = outcome.as_ref().err().map(ToString::to_string);
        if s.lease.take().is_some() {
            s.expired += 1;
        }
        s.submission = None;
        s.latest = None;
        s.skipped += s.captured.saturating_sub(s.last_issued);
        s.last_issued = s.captured;
        s.camera_opened = false;
        s.camera_closure_unconfirmed = !camera_closed;
        s.recording_active = false;
        s.writer_reaped = writer_reaped;
    }
    // Capture is joined and input released. Keep terminal diagnostics available
    // on stdout even when the dataset budget (or its filesystem) is exhausted.
    journal.recording = None;
    let mut terminal = metadata(&config, &final_state, &feedback, monotonic_us()?, 0, true);
    terminal["active"] = false.into();
    terminal["trial_seconds"] = bridge.trial_seconds.into();
    terminal["ui_fixture"] = (!camera_source).into();
    terminal["synthetic_frame"] = (feedback.source_us == 0).into();
    terminal["camera_source"] = camera_source.into();
    terminal["recorded_source"] = recorded_source.into();
    terminal["recorded_time_ms"] = feedback.recorded_time_ms.into();
    terminal["camera_opened"] = bridge.state.lock().unwrap().camera_active().into();
    terminal["test_phase"] = if outcome.is_ok() { "complete" } else { "error" }.into();
    terminal["input_available"] = false.into();
    terminal["recording"] = false.into();
    terminal["session_error"] = outcome.as_ref().err().map(ToString::to_string).into();
    terminal["provider_run_id"] = bridge.run_id.clone().into();
    // Keep the listener available for the browser's real EOF/worker-close receipt.
    let terminal_publish = preview.publish(feedback.jpeg.clone(), terminal);
    bridge.state.lock().unwrap().finishing = terminal_publish.is_ok();
    let outcome = outcome.and(terminal_publish.map_err(Into::into));
    let until = Instant::now() + Duration::from_millis(2000);
    while !stop.load(Ordering::Relaxed) && Instant::now() < until {
        let ready = {
            let state = bridge.state.lock().unwrap();
            state.closed && state.terminal_visible
        };
        if ready {
            break;
        }
        thread::sleep(Duration::from_millis(5));
    }
    stop.store(true, Ordering::Relaxed);
    let s = bridge.state.lock().unwrap();
    for capture in &s.captures {
        journal.emit(capture.clone())?;
    }
    let camera = camera_mailbox.lock().unwrap();
    journal.emit(
        json!({"event":"stop","controller":final_state,"captured":s.captured,
        "issued":s.issued,"skipped":s.skipped,"expired":s.expired,"worker_closed":s.closed,"terminal_visible":s.terminal_visible,
        "error":outcome.as_ref().err().map(ToString::to_string),"stop_requested":s.stop_requested,"camera_opened":s.camera_active(),
        "camera_captured":camera.frames,"camera_mailbox_dropped":camera.dropped,
        "source_kind":camera.source_kind,"system_input":s.system_input(),
        "camera_was_opened":camera.opened,"input_was_started":s.input_started,
        "writer_reaped":s.writer_reaped,"feedback_snapshots":feedback_snapshots,"calibration":calibration.as_ref().map(|c|c.lock().unwrap().state(monotonic_us().unwrap_or(0)))}),
    )?;
    outcome
}

struct Feedback {
    dimensions: [u32; 2],
    jpeg: Vec<u8>,
    source_us: u64,
    raw: Vec<Value>,
    sequence: u64,
    meaning_us: u64,
    recorded_time_ms: Option<f64>,
    pinch_ratio: Option<f64>,
    eligible: bool,
    visual_macros: bool,
    visual_pose: Option<crate::pose_classifier::VisualState>,
    classifier_ms: f64,
}

fn metadata(
    config: &SessionConfig,
    state: &controller::Output,
    feedback: &Feedback,
    now: u64,
    remaining: u64,
    started: bool,
) -> Value {
    let (raw, source, sequence, meaning, eligible) = (
        &feedback.raw,
        feedback.source_us,
        feedback.sequence,
        feedback.meaning_us,
        feedback.eligible,
    );
    let first = raw.first();
    let age = now.saturating_sub(source) as f64 / 1000.0;
    let mut perception = raw.to_vec();
    for hand in &mut perception {
        hand.as_object_mut().unwrap().remove("world_landmarks");
    }
    let mut data = json!({"active":true,"ui_fixture":true,"input_enabled":false,"input_available":false,"recording":false,
        "width":feedback.dimensions[0],"height":feedback.dimensions[1],"source_us":source,"sequence":sequence,"age_ms":age,"frame_age_ms":age,
        "fresh":source>0&&age<(config.freshness_ms as f64),"freshness_ms":config.freshness_ms,"remaining_ms":remaining,
        "test_phase":if started {"testing"}else{"preparing"},"decode_ms":0,"inference_ms":0,
        "provider_transport":true,"presence_scores":config.provider_presence_scores,"confidence_kind":if config.provider_presence_scores {"presence_score_or_configured_floor"}else{"configured_presence_threshold_lower_bound"},"frame_to_meaning_ms":meaning.saturating_sub(source)as f64/1000.0,
        "landmarks":first.map(|h|&h["landmarks"]),"confidence":first.map(|h|&h["confidence"]),
        "perception":perception,"controller":state,"visual_macros":feedback.visual_macros,"visual_pose":feedback.visual_pose,"classifier_ms":feedback.classifier_ms,"cursor":state.pointer_cursor,
        "eligible":eligible,"pinch_ratio":feedback.pinch_ratio,"pinch_metric":config.pinch_metric_name(),"presses":0,"pinch_state":if matches!(state.mode,crate::controller::Mode::OneContact|crate::controller::Mode::TwoContacts) {if state.contacts.is_empty(){"touch"}else{"pressed"}}else{state.pointer_state},
        "confirmation_elapsed_ms":state.confirmation_elapsed_us/1000,"confirmation_required_ms":config.gestures().confirm_us/1000,
        "close_ratio":config.pinch_close_ratio,"open_ratio":config.pinch_open_ratio,"blocked_reason":state.reason});
    data["beta_one_hand"] = config.provider_absolute_pointer.into();
    data["pointer_model"] = if config.provider_absolute_pointer {
        "absolute_workspace"
    } else {
        "relative"
    }
    .into();
    data["screen_target"] = if config.provider_absolute_pointer {
        serde_json::to_value(state.pointer_cursor.map(|p| config.touch_workspace.map(p))).unwrap()
    } else {
        Value::Null
    };
    data
}

#[cfg(test)]
mod tests {
    #[test]
    fn unconfirmed_native_camera_closure_is_unknown_in_http_status() {
        let b = bridge();
        {
            let mut state = b.state.lock().unwrap();
            state.eof = true;
            state.camera_closure_unconfirmed = true;
        }
        let status: Value =
            serde_json::from_slice(&b.respond("GET", "/provider/config", b"").unwrap().body)
                .unwrap();
        assert!(status["camera_opened"].is_null());
        assert_eq!(status["system_input"], false);
    }

    /// Actual HTTP + session + source republication + tracking/meaning + Stop.
    /// The worker is emulated; this exercises no browser, model, camera or input.
    #[test]
    fn full_service_consumes_worker_leases_and_closes_without_recording_or_devices() {
        use std::{
            io::{Read, Write},
            net::{TcpListener, TcpStream},
        };
        fn http(
            port: u16,
            method: &str,
            path: &str,
            body: &Value,
        ) -> io::Result<(String, String, Vec<u8>)> {
            let mut connection = TcpStream::connect((std::net::Ipv4Addr::LOCALHOST, port))?;
            connection.set_read_timeout(Some(Duration::from_secs(3)))?;
            let body = if method == "POST" {
                serde_json::to_vec(body)?
            } else {
                Vec::new()
            };
            write!(connection,"{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nOrigin: http://127.0.0.1:{port}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",body.len())?;
            connection.write_all(&body)?;
            let mut reply = Vec::new();
            connection.read_to_end(&mut reply)?;
            let boundary = reply
                .windows(4)
                .position(|v| v == b"\r\n\r\n")
                .ok_or_else(|| io::Error::other("HTTP headers missing"))?;
            let headers =
                String::from_utf8(reply[..boundary].to_vec()).map_err(io::Error::other)?;
            Ok((
                headers.lines().next().unwrap().to_owned(),
                headers,
                reply[boundary + 4..].to_vec(),
            ))
        }
        let root = std::env::temp_dir().join(format!("wiggle-full-service-{}", std::process::id()));
        fs::create_dir(&root).unwrap();
        let session = root.join("session.json");
        let settings = root.join("settings.json");
        fs::write(&session,json!({"camera":root.join("never-opened"),"fps":10,"gain":2000,"gesture_min_confidence":0.5}).to_string()).unwrap();
        let config = SessionConfig::load_provider(&session).unwrap();
        fs::write(
            &settings,
            json!({"version":1,"gestures":config.gestures()}).to_string(),
        )
        .unwrap();
        let mut jpeg = Vec::new();
        image::codecs::jpeg::JpegEncoder::new(&mut jpeg)
            .encode_image(&image::RgbImage::new(640, 480))
            .unwrap();
        let image = root.join("fixture.jpg");
        fs::write(&image, &jpeg).unwrap();
        let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        let worker = thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(10);
            let config = loop {
                if let Ok((status, _, body)) = http(port, "GET", "/provider/config", &Value::Null) {
                    assert!(status.contains("200"));
                    break serde_json::from_slice::<Value>(&body).unwrap();
                }
                assert!(Instant::now() < deadline, "service did not listen");
                thread::sleep(Duration::from_millis(10));
            };
            let run_id = &config["run_id"];
            assert_eq!(config["camera_opened"], false);
            assert_eq!(config["system_input"], false);
            assert_eq!(config["recording_active"], false);
            assert!(http(port, "GET", "/provider/frame", &Value::Null)
                .unwrap()
                .0
                .contains("409"));
            for path in ["/provider/prepare", "/provider/ready"] {
                assert!(http(port, "POST", path, &json!({"run_id":run_id}))
                    .unwrap()
                    .0
                    .contains("200"));
            }
            let mut receipts = 0;
            let mut previous = 0;
            while receipts < 2 {
                assert!(
                    Instant::now() < deadline,
                    "fixture did not return meaning receipts"
                );
                let (status, headers, bytes) =
                    http(port, "GET", "/provider/frame", &Value::Null).unwrap();
                if status.contains("204") {
                    thread::sleep(Duration::from_millis(5));
                    continue;
                }
                assert!(status.contains("200"), "{status}");
                assert!(bytes.starts_with(&[0xff, 0xd8]));
                let ticket: Value = serde_json::from_str(
                    headers
                        .lines()
                        .find_map(|line| line.strip_prefix("X-Wiggle-Source: "))
                        .unwrap(),
                )
                .unwrap();
                let source = ticket["source_us"].as_u64().unwrap();
                assert!(source > previous);
                previous = source;
                let (status, _, body) = http(
                    port,
                    "POST",
                    "/provider/result",
                    &json!({"run_id":run_id,"sequence":ticket["sequence"],"hands":[]}),
                )
                .unwrap();
                assert!(
                    status.contains("200"),
                    "{status}: {}",
                    String::from_utf8_lossy(&body)
                );
                let receipt: Value = serde_json::from_slice(&body).unwrap();
                assert_eq!(receipt["source_us"], source);
                assert_eq!(receipt["contacts"], json!([]));
                receipts += 1;
            }
            assert!(http(
                port,
                "POST",
                "/provider/stop",
                &json!({"run_id":run_id,"graceful":true})
            )
            .unwrap()
            .0
            .contains("200"));
            loop {
                let (_, _, body) = http(port, "GET", "/provider/config", &Value::Null).unwrap();
                let state: Value = serde_json::from_slice(&body).unwrap();
                if state["terminal_ready"] == true {
                    assert_eq!(state["camera_opened"], false);
                    assert_eq!(state["system_input"], false);
                    assert_eq!(state["recording_active"], false);
                    break;
                }
                assert!(Instant::now() < deadline);
                thread::sleep(Duration::from_millis(5));
            }
            for (path, body) in [
                (
                    "/provider/closed",
                    json!({"run_id":run_id,"acknowledged":true}),
                ),
                ("/provider/terminal-visible", json!({"run_id":run_id})),
            ] {
                assert!(http(port, "POST", path, &body).unwrap().0.contains("200"));
            }
        });
        let result = run_engine_with_assets(
            RunOptions {
                output_target: None,
                calibration_profile: None,
                session: &session,
                settings: &settings,
                asset_root: &root,
                port,
                max_hands: 1,
                initial_jpeg: jpeg,
                preflight: false,
                source: Source::Fixture {
                    frames: vec![
                        (0, Some(image.clone())),
                        (400_000, Some(image.clone())),
                        (900_000, Some(image)),
                    ],
                    duration: 900_000,
                },
            },
            |_| Ok(BTreeMap::new()),
        );
        let received = worker.join();
        result.unwrap();
        received.unwrap();
        assert_eq!(
            fs::read_dir(&root).unwrap().count(),
            3,
            "ordinary sessions must create no recording files"
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn recording_status_reports_the_live_sink_separately_from_camera_and_eof() {
        let bridge = Bridge::new(BTreeMap::new(), 300_000, "record-status".into(), 1);
        for (camera, recording, eof) in [
            (false, false, false),
            (true, false, false),
            (true, true, false),
            (true, true, true),
            (false, false, true),
        ] {
            {
                let mut state = bridge.state.lock().unwrap();
                state.camera_opened = camera;
                state.recording_active = recording;
                state.eof = eof;
            }
            let value: Value = serde_json::from_slice(
                &bridge.respond("GET", "/provider/config", b"").unwrap().body,
            )
            .unwrap();
            assert_eq!(value["recording_active"], recording);
        }
    }

    #[test]
    fn full_hd_jpeg_and_frame_lease_keep_native_dimensions() {
        let mut bytes = Vec::new();
        image::codecs::jpeg::JpegEncoder::new(&mut bytes)
            .encode_image(&image::RgbImage::new(1920, 1080))
            .unwrap();
        assert!(jpeg_bytes_for(bytes.clone(), [1920, 1080]).is_ok());
        assert!(jpeg_bytes_for(bytes.clone(), [640, 480]).is_err());
        let mut b = Bridge::new(BTreeMap::new(), 300_000, "hd".into(), 1);
        b.dimensions = [1920, 1080];
        b.state.lock().unwrap().ready = true;
        b.publish(Frame {
            bytes,
            source_us: monotonic_us().unwrap(),
            sequence: 1,
        })
        .unwrap();
        let config: Value =
            serde_json::from_slice(&b.respond("GET", "/provider/config", b"").unwrap().body)
                .unwrap();
        assert_eq!(
            (config["width"].as_u64(), config["height"].as_u64()),
            (Some(1920), Some(1080))
        );
        let lease = b.respond("GET", "/provider/frame", b"").unwrap();
        assert_eq!(lease.status, "200 OK");
        let ticket: Value = serde_json::from_str(
            lease
                .headers
                .trim()
                .strip_prefix("X-Wiggle-Source: ")
                .unwrap(),
        )
        .unwrap();
        assert_eq!(
            (ticket["width"].as_u64(), ticket["height"].as_u64()),
            (Some(1920), Some(1080))
        );
        assert!(jpeg_bytes_for(lease.body, [1920, 1080]).is_ok());
    }

    use super::*;
    #[test]
    fn calibration_routes_are_run_bound_manual_camera_only_and_cannot_save_before_review() {
        let root =
            std::env::temp_dir().join(format!("wiggle-calibration-routes-{}", std::process::id()));
        fs::create_dir(&root).unwrap();
        let path = root.join("profile.json");
        let mut b = Bridge::new(BTreeMap::new(), 300_000, "calibration".into(), 1);
        b.manual_start = true;
        b.calibration = Some(Arc::new(Mutex::new(
            crate::provider_calibration::Run::new(&path, [1920, 1080]).unwrap(),
        )));
        for (route, body, status) in [
            (
                "/provider/calibration/save",
                br#"{"run_id":"other"}"#.as_slice(),
                "409 Conflict",
            ),
            (
                "/provider/calibration/save",
                br#"{"run_id":"calibration"}"#.as_slice(),
                "409 Conflict",
            ),
            (
                "/provider/calibration/step",
                br#"{"run_id":"calibration","step":0}"#.as_slice(),
                "409 Conflict",
            ),
            (
                "/provider/prepare",
                br#"{"run_id":"calibration"}"#.as_slice(),
                "200 OK",
            ),
            (
                "/provider/prepare",
                br#"{"run_id":"calibration"}"#.as_slice(),
                "409 Conflict",
            ),
            (
                "/provider/calibration/save",
                br#"{"run_id":"calibration","step":0}"#.as_slice(),
                "400 Bad Request",
            ),
            (
                "/provider/calibration/cancel",
                br#"{"run_id":"calibration"}"#.as_slice(),
                "200 OK",
            ),
        ] {
            assert_eq!(
                b.respond("POST", route, body).unwrap().status,
                status,
                "{route}"
            );
        }
        let state = b.state.lock().unwrap();
        assert!(!state.ready && !state.camera_opened && !state.input_started && !state.cancelled);
        assert!(!path.exists());
        drop(state);
        assert!(b.calibration.as_ref().unwrap().lock().unwrap().done());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn worker_loss_is_bounded_without_timing_out_manual_standby_or_missing_hands() {
        let bridge = Bridge::new(BTreeMap::new(), 300_000, "run".into(), 1);
        let now = Instant::now();
        {
            let mut s = bridge.state.lock().unwrap();
            assert!(!s.worker_expired(now)); // Manual standby is indefinite.
            s.ready = true;
            s.worker_last_seen = Some(now);
            assert!(!s.worker_expired(now + Duration::from_millis(4999)));
            assert!(s.worker_expired(now + Duration::from_secs(5)));
            s.worker_last_seen = Some(now - Duration::from_secs(6));
        }
        // An alive worker polling with no frame/hand still keeps its connection.
        assert_eq!(
            bridge
                .respond("GET", "/provider/frame", b"")
                .unwrap()
                .status,
            "204 No Content"
        );
        assert!(!bridge.state.lock().unwrap().worker_expired(Instant::now()));
        let mut s = bridge.state.lock().unwrap();
        s.eof = true;
        assert!(!s.worker_expired(now + Duration::from_secs(60)));
    }

    #[test]
    fn optional_feedback_and_controls_do_not_own_the_detector() {
        let bridge = Bridge::new(BTreeMap::new(), 300_000, "run".into(), 1);
        for path in ["/provider/feedback", "/provider/control"] {
            let response = bridge.respond("GET", path, b"").unwrap();
            assert_eq!(response.status, "200 OK");
            let body = String::from_utf8(response.body).unwrap();
            assert!(!body.contains("/provider/client.mjs"));
        }
        let response = bridge.respond("GET", "/provider/worker", b"").unwrap();
        let body = String::from_utf8(response.body).unwrap();
        assert!(body.contains("/provider/client.mjs"));
        assert!(!body.contains("<canvas"));
        assert!(!body.contains("/frame"));
        assert!(!body.contains("preview.html"));
        let state = bridge.state.lock().unwrap();
        assert!(!state.ready && !state.camera_opened && !state.input_started);
    }
    #[test]
    fn terminal_activity_distinguishes_acquisition_history_from_reaped_or_unknown_input() {
        let bridge = Bridge::new(BTreeMap::new(), 300_000, "teardown".into(), 1);
        {
            let mut state = bridge.state.lock().unwrap();
            state.input_started = true;
            assert_eq!(state.system_input(), Some(true));
            state.eof = true;
            assert_eq!(state.system_input(), None);
            state.writer_reaped = true;
            assert_eq!(state.system_input(), Some(false));
            assert!(state.input_started); // Preserve the historical acquisition.
        }
        let response = bridge.respond("GET", "/provider/config", b"").unwrap();
        let value: Value = serde_json::from_slice(&response.body).unwrap();
        assert_eq!(value["system_input"], false);
        assert_eq!(value["camera_opened"], false);
    }
    #[test]
    fn graceful_control_stop_is_distinct_from_worker_failure_and_cannot_prepare() {
        let b = Bridge::new(BTreeMap::new(), 300_000, "selected".into(), 1);
        for (path, body, status) in [
            (
                "/provider/stop",
                br#"{"run_id":"other","graceful":true}"#.as_slice(),
                "409 Conflict",
            ),
            (
                "/provider/prepare",
                br#"{"run_id":"selected","graceful":true}"#.as_slice(),
                "400 Bad Request",
            ),
        ] {
            assert_eq!(b.respond("POST", path, body).unwrap().status, status);
            let s = b.state.lock().unwrap();
            assert!(!s.stop_requested && !s.cancelled && s.preparing.is_none() && !s.ready);
        }
        assert_eq!(
            b.respond(
                "POST",
                "/provider/stop",
                br#"{"run_id":"selected","graceful":true}"#
            )
            .unwrap()
            .status,
            "200 OK"
        );
        {
            let s = b.state.lock().unwrap();
            assert!(s.stop_requested && !s.cancelled && s.preparing.is_none() && !s.ready);
        }
        let failed = Bridge::new(BTreeMap::new(), 300_000, "worker".into(), 1);
        assert_eq!(
            failed
                .respond("POST", "/provider/stop", br#"{"run_id":"worker"}"#)
                .unwrap()
                .status,
            "200 OK"
        );
        let s = failed.state.lock().unwrap();
        assert!(s.cancelled && !s.stop_requested);
    }
    #[test]
    fn worker_failure_detail_is_bounded_run_scoped_and_retained() {
        let b = Bridge::new(BTreeMap::new(), 300_000, "failed".into(), 1);
        for (path, body, status) in [
            (
                "/provider/stop",
                json!({"run_id":"other","worker_error":"wrong run"}),
                "409 Conflict",
            ),
            (
                "/provider/ready",
                json!({"run_id":"failed","worker_error":"wrong action"}),
                "400 Bad Request",
            ),
            (
                "/provider/stop",
                json!({"run_id":"failed","graceful":true,"worker_error":"wrong reason"}),
                "400 Bad Request",
            ),
            (
                "/provider/stop",
                json!({"run_id":"failed","worker_error":"x".repeat(513)}),
                "400 Bad Request",
            ),
        ] {
            assert_eq!(
                b.respond("POST", path, body.to_string().as_bytes())
                    .unwrap()
                    .status,
                status
            );
            assert!(!b.state.lock().unwrap().cancelled);
        }
        let detail = "Meaning receipt rejected: meaning receipt delayed";
        assert_eq!(
            b.respond(
                "POST",
                "/provider/stop",
                json!({"run_id":"failed","worker_error":detail})
                    .to_string()
                    .as_bytes()
            )
            .unwrap()
            .status,
            "200 OK"
        );
        b.respond("POST", "/provider/stop", br#"{"run_id":"failed"}"#)
            .unwrap();
        let state = b.state.lock().unwrap();
        assert!(state.cancelled && !state.stop_requested);
        assert_eq!(state.worker_error.as_deref(), Some(detail));
    }
    #[test]
    fn lightweight_control_status_has_no_preparation_countdown_and_retains_terminal_truth() {
        let mut bridge = Bridge::new(BTreeMap::new(), 300_000, "control".into(), 1);
        bridge.manual_start = true;
        bridge.trial_seconds = Some(90);
        bridge.detail_configured = true;
        bridge.escape_configured = true;
        bridge.scroll_configured = true;
        bridge.right_click = true;
        let mut controller = controller::Controller::new(controller::Settings {
            escape: Some(Default::default()),
            scroll: Some(Default::default()),
            ..Default::default()
        })
        .unwrap();
        let initial = controller.start();
        let now = Instant::now();
        {
            let mut s = bridge.state.lock().unwrap();
            s.control_output(&initial);
            s.preparing = Some(now - Duration::from_secs(10));
            assert_eq!(s.remaining_ms(Some(90), now), None);
            s.acquisition_started = Some(now - Duration::from_secs(2));
            assert_eq!(s.remaining_ms(Some(90), now), Some(88_000));
            assert_eq!(s.remaining_ms(None, now), None);
            assert_eq!(
                s.remaining_ms(Some(90), now + Duration::from_secs(100)),
                Some(0)
            );
            s.detail_active = true;
            s.controller_running = true;
            s.control_mode = Some(controller::Mode::Pointer);
        }
        let response = bridge.respond("GET", "/provider/config", b"").unwrap();
        let live: Value = serde_json::from_slice(&response.body).unwrap();
        assert_eq!(live["detail_configured"], true);
        assert_eq!(live["detail_active"], true);
        assert_eq!(live["right_click"], true);
        assert_eq!(live["scroll_configured"], true);
        assert_eq!(live["scroll_feedback"]["active"], false);
        assert_eq!(live["escape_configured"], true);
        assert_eq!(live["escape_feedback"]["enabled"], true);
        assert_eq!(live["mode"], "pointer");
        assert_eq!(live["controller_running"], true);
        assert!(live["remaining_ms"].as_u64().unwrap() <= 88_000);
        assert_eq!(live["camera_opened"], false);
        assert_eq!(live["system_input"], false);
        assert_eq!(live["terminal_ready"], false);
        {
            let mut s = bridge.state.lock().unwrap();
            s.eof = true;
            assert_eq!(s.remaining_ms(Some(90), now), Some(0));
        }
        let end: Value =
            serde_json::from_slice(&bridge.respond("GET", "/provider/config", b"").unwrap().body)
                .unwrap();
        assert_eq!(end["detail_active"], false);
        assert_eq!(end["controller_running"], false);
        assert!(end["escape_feedback"].is_null());
        assert!(end["scroll_feedback"].is_null());
        assert_eq!(end["remaining_ms"], 0);
        assert_eq!(end["terminal_ready"], false);
        bridge.state.lock().unwrap().finishing = true;
        let ready: Value =
            serde_json::from_slice(&bridge.respond("GET", "/provider/config", b"").unwrap().body)
                .unwrap();
        assert_eq!(ready["terminal_ready"], true);
    }

    #[test]
    fn manual_standby_requires_start_before_ready_and_preserves_run_identity() {
        let mut bridge = Bridge::new(BTreeMap::new(), 300_000, "beta".into(), 1);
        bridge.manual_start = true;
        bridge.trial_seconds = Some(45);
        let config: Value =
            serde_json::from_slice(&bridge.respond("GET", "/provider/config", b"").unwrap().body)
                .unwrap();
        assert_eq!(config["manual_start"], true);
        assert_eq!(config["trial_seconds"], 45);
        let body = br#"{"run_id":"beta"}"#;
        assert_eq!(
            bridge
                .respond("POST", "/provider/ready", body)
                .unwrap()
                .status,
            "409 Conflict"
        );
        assert!(!bridge.state.lock().unwrap().ready);
        assert!(bridge.state.lock().unwrap().preparing.is_none());
        assert_eq!(
            bridge
                .respond("POST", "/provider/prepare", br#"{"run_id":"old"}"#)
                .unwrap()
                .status,
            "409 Conflict"
        );
        assert_eq!(
            bridge
                .respond("POST", "/provider/prepare", body)
                .unwrap()
                .status,
            "200 OK"
        );
        assert_eq!(
            bridge
                .respond("POST", "/provider/prepare", body)
                .unwrap()
                .status,
            "409 Conflict"
        );
        assert_eq!(
            bridge
                .respond("POST", "/provider/ready", body)
                .unwrap()
                .status,
            "200 OK"
        );
        let state = bridge.state.lock().unwrap();
        assert!(state.ready && state.preparing.is_some());
        assert!(!state.camera_opened && !state.input_started && state.captured == 0);
    }

    #[test]
    fn retained_frame_provenance_never_replaces_the_new_host_clock() {
        let mut b = Bridge::new(BTreeMap::new(), 300_000, "recorded".into(), 1);
        b.recorded_source = true;
        let now = monotonic_us().unwrap();
        b.publish_recorded(
            Frame {
                bytes: vec![1, 2, 3],
                source_us: now,
                sequence: 77,
            },
            Some(crate::recording::FrameIndex {
                segment: 1,
                source_us: 1_000,
                sequence: 91,
                offset: 0,
                length: 3,
            }),
        )
        .unwrap();
        let state = b.state.lock().unwrap();
        assert_eq!(state.captures[0]["source_us"], now);
        assert_eq!(state.captures[0]["capture_sequence"], 77);
        assert_eq!(state.captures[0]["recorded_frame"]["source_us"], 1_000);
        assert_eq!(state.captures[0]["recorded_frame"]["sequence"], 91);
        assert_eq!(state.captures[0]["camera_opened"], false);
        assert_eq!(
            state
                .latest
                .as_ref()
                .unwrap()
                .recorded
                .as_ref()
                .unwrap()
                .segment,
            1
        );
    }
    #[test]
    fn camera_configuration_does_not_claim_acquisition_or_input_before_start() {
        let mut b = Bridge::new(BTreeMap::new(), 300_000, "camera".into(), 1);
        b.camera_source = true;
        b.input_requested = true;
        b.presence_floor = 0.8;
        let response = b.respond("GET", "/provider/config", &[]).unwrap();
        let config: Value = serde_json::from_slice(&response.body).unwrap();
        assert_eq!(config["presence_floor"], 0.8);
        assert_eq!(config["camera_source"], true);
        assert_eq!(config["input_requested"], true);
        assert_eq!(config["camera_opened"], false);
        assert_eq!(config["system_input"], false);
    }

    #[test]
    fn producer_failure_is_retained_in_the_worker_eof_response() {
        let b = bridge();
        {
            let mut state = b.state.lock().unwrap();
            state.ready = true;
            state.eof = true;
            state.error = Some("camera unavailable".into());
        }
        let response = b.respond("GET", "/provider/frame", &[]).unwrap();
        assert_eq!(response.status, "410 Gone");
        assert_eq!(
            serde_json::from_slice::<Value>(&response.body).unwrap()["error"],
            "camera unavailable"
        );
    }

    #[test]
    fn terminal_view_receipt_is_scoped_to_the_current_published_end_state() {
        let b = bridge();
        let current = br#"{"run_id":"test-1"}"#;
        assert_eq!(
            b.respond("POST", "/provider/terminal-visible", current)
                .unwrap()
                .status,
            "409 Conflict"
        );
        b.state.lock().unwrap().finishing = true;
        assert_eq!(
            b.respond("POST", "/provider/terminal-visible", br#"{"run_id":"old"}"#)
                .unwrap()
                .status,
            "409 Conflict"
        );
        assert!(!b.state.lock().unwrap().terminal_visible);
        assert_eq!(
            b.respond("POST", "/provider/terminal-visible", current)
                .unwrap()
                .status,
            "200 OK"
        );
        assert!(b.state.lock().unwrap().terminal_visible);
    }

    #[test]
    fn warmup_stays_on_the_selected_synthetic_image_after_camera_frames_arrive() {
        let mut b = Bridge::new(BTreeMap::new(), 300_000, "warmup".into(), 1);
        b.camera_source = true;
        b.warmup = b"explicit synthetic warmup".to_vec();
        b.publish(Frame {
            bytes: b"different camera bytes".to_vec(),
            source_us: monotonic_us().unwrap(),
            sequence: 1,
        })
        .unwrap();
        let response = b.respond("GET", "/provider/warmup", &[]).unwrap();
        assert_eq!(response.body, b"explicit synthetic warmup");
        assert_eq!(response.headers, "X-Wiggle-Synthetic: 1\r\n");
        assert_eq!(b.state.lock().unwrap().last_issued, 0);
    }

    #[test]
    fn old_directory_only_config_does_not_enable_recording_and_opt_in_is_finite() {
        let mut config: SessionConfig = serde_json::from_value(json!({"camera":"/not-acquired",
            "fps":10,"gain":2000,"recording_directory":"/not-created"}))
        .unwrap();
        assert!(!config.recording_enabled);
        assert!(config.validate_recording(None).is_ok());
        config.recording_enabled = true;
        assert!(config.validate_recording(None).is_err());
        assert!(config.validate_recording(Some(31)).is_err());
        assert!(config.validate_recording(Some(30)).is_ok());
    }
    #[test]
    fn explicit_dataset_rejects_missing_directory_before_loading_assets_or_starting_io() {
        let root =
            std::env::temp_dir().join(format!("wiggle-camera-policy-{}", std::process::id()));
        fs::create_dir(&root).unwrap();
        let session = root.join("session.json");
        let settings = root.join("settings.json");
        fs::write(&session, json!({"camera":root.join("not-acquired"),"models":root.join("not-loaded"),"runtime":root.join("not-loaded"),"fps":10,"gain":2000,"gesture_min_confidence":0.5,"recording_enabled":true}).to_string()).unwrap();
        let config = SessionConfig::load(&session).unwrap();
        fs::write(
            &settings,
            json!({"version":1,"gestures":config.gestures()}).to_string(),
        )
        .unwrap();
        let result = run_engine(RunOptions {
            output_target: None,
            calibration_profile: None,
            session: &session,
            settings: &settings,
            asset_root: Path::new("/not-loaded"),
            port: 0,
            max_hands: 1,
            initial_jpeg: Vec::new(),
            source: Source::Camera {
                seconds: 1,
                inject: true,
                manual_start: false,
            },
            preflight: true,
        });
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("dataset recording requires"));
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn fixture_view_bootstraps_client_when_the_existing_html_omits_body_end_tag() {
        let b = bridge();
        let page = b.respond("GET", "/provider/view", &[]).unwrap();
        assert_eq!(page.mime, "text/html; charset=utf-8");
        assert!(String::from_utf8(page.body)
            .unwrap()
            .contains("<script type=\"module\" src=\"/provider/client.mjs\">"));
        assert!(
            String::from_utf8(b.respond("GET", "/provider/client.mjs", &[]).unwrap().body)
                .unwrap()
                .contains("/provider/ready")
        );
        assert!(b.handles("/task_hand_worker.js"));
        assert_eq!(
            b.respond("GET", "/task_hand_worker.js", &[])
                .unwrap()
                .status,
            "200 OK"
        );
    }
    #[test]
    fn one_hand_graph_capacity_is_explicit_and_does_not_admit_two_returned_hands() {
        let b = Bridge::new(BTreeMap::new(), 1_000_000, "one".into(), 1);
        let config = b.respond("GET", "/provider/config", &[]).unwrap();
        assert_eq!(
            serde_json::from_slice::<Value>(&config.body).unwrap()["max_hands"],
            1
        );
        let points = [[0.0; 2]; 21];
        let world = [[0.0; 3]; 21];
        let hand = json!({"landmarks":points,"world_landmarks":world,"right_probability":0.9});
        let body = json!({"run_id":"one","sequence":1,"hands":[hand.clone(),hand]}).to_string();
        assert_eq!(
            b.respond("POST", "/provider/result", body.as_bytes())
                .unwrap()
                .status,
            "400 Bad Request"
        );
        assert!(b.state.lock().unwrap().submission.is_none());
    }
    fn bridge() -> Arc<Bridge> {
        Arc::new(Bridge::new(BTreeMap::new(), 1_000_000, "test-1".into(), 2))
    }
    fn publish(b: &Bridge, sequence: u32) -> u64 {
        b.publish(Frame {
            bytes: vec![1, 2, 3],
            source_us: monotonic_us().unwrap(),
            sequence,
        })
        .unwrap()
    }
    #[test]
    fn latest_slot_leases_host_time_and_does_not_accept_client_clock_or_second_inflight() {
        let b = bridge();
        publish(&b, 0);
        publish(&b, 1);
        publish(&b, 2);
        assert_eq!(
            b.respond("GET", "/provider/frame", &[]).unwrap().status,
            "409 Conflict"
        );
        b.respond("POST", "/provider/ready", br#"{"run_id":"test-1"}"#)
            .unwrap();
        let first = b.respond("GET", "/provider/frame", &[]).unwrap();
        assert_eq!(first.status, "200 OK");
        let state = b.state.lock().unwrap();
        assert_eq!((state.captured, state.issued, state.skipped), (3, 1, 2));
        assert_eq!(state.lease.as_ref().unwrap().sequence, 3);
        assert!(first
            .headers
            .contains(&state.lease.as_ref().unwrap().frame.source_us.to_string()));
        drop(state);
        assert_eq!(
            b.respond("GET", "/provider/frame", &[]).unwrap().status,
            "409 Conflict"
        );
        assert_eq!(
            b.respond(
                "POST",
                "/provider/result",
                br#"{"run_id":"test-1","sequence":3,"source_us":1,"now_us":1,"hands":[]}"#
            )
            .unwrap()
            .status,
            "400 Bad Request"
        );
        assert!(b.state.lock().unwrap().submission.is_none());
    }
    #[test]
    fn calibration_closure_expires_an_inflight_result_without_failing_the_worker() {
        let b = bridge();
        b.state.lock().unwrap().camera_opened = true;
        b.respond("POST", "/provider/ready", br#"{"run_id":"test-1"}"#)
            .unwrap();
        publish(&b, 8);
        b.respond("GET", "/provider/frame", &[]).unwrap();
        let worker = b.clone();
        let response = thread::spawn(move || {
            worker
                .respond(
                    "POST",
                    "/provider/result",
                    br#"{"run_id":"test-1","sequence":1,"hands":[]}"#,
                )
                .unwrap()
        });
        let deadline = Instant::now() + Duration::from_millis(150);
        while b.state.lock().unwrap().submission.is_none() {
            assert!(Instant::now() < deadline);
            thread::sleep(Duration::from_millis(1));
        }
        b.retire_calibration_frames();
        assert!(b.state.lock().unwrap().camera_opened);
        // The capture thread can need longer to join than the HTTP receipt
        // deadline. In-flight work must already be expired while it closes.
        thread::sleep(Duration::from_millis(250));
        assert_eq!(response.join().unwrap().status, "410 Gone");
        assert_eq!(
            b.respond("GET", "/provider/frame", &[]).unwrap().status,
            "204 No Content"
        );
        b.close_calibration_capture();
        assert!(!b.state.lock().unwrap().camera_opened);
        let s = b.state.lock().unwrap();
        assert!(
            s.ready,
            "The same worker must remain available for manual Retry"
        );
        assert!(!s.cancelled);
        assert!(!s.eof);
    }
    #[test]
    fn result_is_tied_to_leased_frame_and_waits_for_actual_meaning_receipt() {
        let b = bridge();
        b.respond("POST", "/provider/ready", br#"{"run_id":"test-1"}"#)
            .unwrap();
        publish(&b, 8);
        b.respond("GET", "/provider/frame", &[]).unwrap();
        assert_eq!(
            b.respond(
                "POST",
                "/provider/result",
                br#"{"run_id":"test-1","sequence":99,"hands":[]}"#
            )
            .unwrap()
            .status,
            "410 Gone"
        );
        assert!(b.state.lock().unwrap().submission.is_none());
        let worker = b.clone();
        let response = thread::spawn(move || {
            worker
                .respond(
                    "POST",
                    "/provider/result",
                    br#"{"run_id":"test-1","sequence":1,"hands":[]}"#,
                )
                .unwrap()
        });
        let deadline = Instant::now() + Duration::from_millis(150);
        let submitted = loop {
            if let Some(p) = b.state.lock().unwrap().submission.take() {
                break p;
            }
            assert!(Instant::now() < deadline);
            thread::sleep(Duration::from_millis(1));
        };
        assert_eq!(submitted.lease.frame.sequence, 8);
        assert!(submitted.received_us >= submitted.lease.frame.source_us);
        b.finish(1, json!({"sequence":1,"contacts":[]}));
        let response = response.join().unwrap();
        assert_eq!(response.status, "200 OK");
        assert_eq!(
            serde_json::from_slice::<Value>(&response.body).unwrap()["sequence"],
            1
        );
        assert_eq!(
            b.respond(
                "POST",
                "/provider/result",
                br#"{"run_id":"test-1","sequence":1,"hands":[]}"#
            )
            .unwrap()
            .status,
            "410 Gone"
        );
    }
    #[test]
    fn expired_source_is_not_dispatched_and_close_requires_acknowledgment() {
        let b = bridge();
        b.respond("POST", "/provider/ready", br#"{"run_id":"test-1"}"#)
            .unwrap();
        b.publish(Frame {
            bytes: vec![],
            source_us: monotonic_us().unwrap() - 2_000_000,
            sequence: 0,
        })
        .unwrap();
        b.state.lock().unwrap().eof = true;
        assert_eq!(
            b.respond("GET", "/provider/frame", &[]).unwrap().status,
            "410 Gone"
        );
        assert!(b.state.lock().unwrap().lease.is_none());
        assert_eq!(
            b.respond("POST", "/provider/closed", b"").unwrap().status,
            "400 Bad Request"
        );
        b.respond(
            "POST",
            "/provider/closed",
            br#"{"run_id":"test-1","acknowledged":false}"#,
        )
        .unwrap();
        assert!(!b.state.lock().unwrap().closed);
        b.respond(
            "POST",
            "/provider/closed",
            br#"{"run_id":"test-1","acknowledged":true}"#,
        )
        .unwrap();
        assert!(b.state.lock().unwrap().closed);
    }
    #[test]
    fn an_old_browser_run_cannot_ready_cancel_close_or_complete_the_next_run() {
        let b = bridge();
        for route in ["/provider/ready", "/provider/stop"] {
            assert_eq!(
                b.respond("POST", route, br#"{"run_id":"old"}"#)
                    .unwrap()
                    .status,
                "409 Conflict"
            );
        }
        {
            let state = b.state.lock().unwrap();
            assert!(!state.ready && !state.cancelled);
        }
        assert_eq!(
            b.respond(
                "POST",
                "/provider/closed",
                br#"{"run_id":"old","acknowledged":true}"#
            )
            .unwrap()
            .status,
            "409 Conflict"
        );
        assert_eq!(
            b.respond(
                "POST",
                "/provider/result",
                br#"{"run_id":"old","sequence":1,"hands":[]}"#
            )
            .unwrap()
            .status,
            "409 Conflict"
        );
        {
            let state = b.state.lock().unwrap();
            assert!(!state.closed && state.submission.is_none());
        }
    }
}
