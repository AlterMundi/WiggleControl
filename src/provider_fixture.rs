//! Finite real-clock encoded-frame/worker/meaning qualification. No camera or device.
use crate::{
    controller,
    intent::monotonic_us,
    preview::{Extension, ExtensionResponse, Preview},
    provider::{self, Landmarks},
    session::{Frame, SessionConfig},
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
    process::Command,
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
}
struct Submission {
    lease: Lease,
    hands: Vec<Landmarks>,
    received_us: u64,
}
#[derive(Default)]
struct State {
    ready: bool,
    cancelled: bool,
    closed: bool,
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
}
struct Bridge {
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
    fn new(assets: BTreeMap<String, PathBuf>, stale_us: u64, run_id: String) -> Self {
        Self {
            run_id,
            state: Mutex::new(State::default()),
            changed: Condvar::new(),
            assets,
            stale_us,
        }
    }
    fn publish(&self, frame: Frame) -> io::Result<u64> {
        let mut s = self.state.lock().unwrap();
        s.captured = s
            .captured
            .checked_add(1)
            .ok_or_else(|| io::Error::other("frame sequence exhausted"))?;
        let sequence = s.captured;
        s.captures
            .push(json!({"event":"capture","sequence":sequence,
            "capture_sequence":frame.sequence,"source_us":frame.source_us,
            "camera_opened":false,"system_input":false}));
        s.latest = Some(Lease {
            sequence,
            issued_us: 0,
            frame: Arc::new(frame),
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
            ("GET", "/task_hand_worker.js") => Ok(ExtensionResponse {
                status: "200 OK",
                mime: "text/javascript",
                body: include_bytes!("../tools/synthetic/task_hand_worker.js").to_vec(),
                headers: String::new(),
            }),
            ("GET", "/provider/config") => Ok(reply(
                "200 OK",
                json!({"model_sha256":MODEL,"delegate":"GPU",
                "run_id":self.run_id,"package_version":"1.1.1","camera_opened":false,"system_input":false}),
            )),
            ("POST", "/provider/ready") | ("POST", "/provider/stop") => {
                #[derive(Deserialize)]
                #[serde(deny_unknown_fields)]
                struct Run {
                    run_id: String,
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
                if path == "/provider/ready" {
                    self.state.lock().unwrap().ready = true;
                } else {
                    self.state.lock().unwrap().cancelled = true;
                }
                self.changed.notify_all();
                Ok(reply("200 OK", json!({"accepted":true})))
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
                let current = s.latest.clone().filter(|f| f.sequence > s.last_issued);
                if let Some(mut lease) = current {
                    let now = monotonic_us()?;
                    if now < lease.frame.source_us || now - lease.frame.source_us > self.stale_us {
                        s.skipped += lease.sequence.saturating_sub(s.last_issued);
                        s.last_issued = lease.sequence;
                        return Ok(reply(
                            if s.eof { "410 Gone" } else { "204 No Content" },
                            json!({"expired":true}),
                        ));
                    }
                    s.skipped += lease.sequence.saturating_sub(s.last_issued + 1);
                    s.last_issued = lease.sequence;
                    s.issued += 1;
                    lease.issued_us = now;
                    s.lease = Some(lease.clone());
                    let ticket = json!({"run_id":self.run_id,"sequence":lease.sequence,"source_us":lease.frame.source_us,"issued_us":lease.issued_us,
                        "width":640,"height":480,"capture_sequence":lease.frame.sequence});
                    Ok(ExtensionResponse {
                        status: "200 OK",
                        mime: "image/jpeg",
                        body: lease.frame.bytes.clone(),
                        headers: format!("X-Wiggle-Source: {ticket}\r\n"),
                    })
                } else if s.eof {
                    Ok(reply("410 Gone", json!({"eof":true})))
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
                if returned.hands.len() > 2 {
                    return Ok(reply(
                        "400 Bad Request",
                        json!({"error":"at most two hands"}),
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
        let output = Command::new("sha256sum").arg("--").arg(&path).output()?;
        let actual = String::from_utf8(output.stdout)?;
        if !output.status.success() || actual.split_whitespace().next() != Some(sha.as_str()) {
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
    let data = fs::read(path)?;
    if !data.starts_with(&[0xff, 0xd8, 0xff]) {
        return Err("fixture requires prepared JPEG images".into());
    }
    if image::ImageReader::new(io::Cursor::new(&data))
        .with_guessed_format()?
        .into_dimensions()?
        != (640, 480)
    {
        return Err("fixture requires640x480 JPEG".into());
    }
    Ok(data)
}

pub fn run(
    manifest: &Path,
    session: &Path,
    settings: &Path,
    asset_root: &Path,
    port: u16,
) -> Result<(), Box<dyn Error>> {
    let (config, mut controller, mut tracker) = provider::configured(session, settings, 0.5)?;
    let sequence = crate::sequence::Sequence::load(manifest)?;
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
    let bridge = Arc::new(Bridge::new(
        assets(asset_root)?,
        config.gestures().stale_us,
        format!("{}-{}", std::process::id(), monotonic_us()?),
    ));
    let stop = Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(signal_hook::consts::SIGTERM, stop.clone())?;
    signal_hook::flag::register(signal_hook::consts::SIGINT, stop.clone())?;
    let preview = Preview::start(port, stop.clone())?;
    preview.enable_extension(bridge.clone());
    preview.enable_controller_controls();
    let mut log = io::stdout().lock();
    let mut emit = |value: Value| -> io::Result<()> {
        writeln!(log, "{value}")?;
        log.flush()
    };
    emit(
        json!({"event":"listening","port":preview.port,"schema":"wigglecontrol.provider-fixture/v1",
        "camera_opened":false,"system_input":false,"source_kind":"synthetic_encoded_frame_publish"}),
    )?;
    let mut feedback = Feedback {
        jpeg: initial_jpeg,
        source_us: 0,
        raw: Vec::new(),
        sequence: 0,
        meaning_us: 0,
        eligible: false,
    };
    let mut previous_active = (Vec::new(), false);
    let prepared = Instant::now();
    let mut started = None;
    let mut ended = None;
    let mut producer = None;
    let mut last_publish = Instant::now() - Duration::from_secs(1);
    let outcome = (|| -> Result<(), Box<dyn Error>> {
        while !stop.load(Ordering::Relaxed) {
            if bridge.state.lock().unwrap().cancelled {
                break;
            }
            let mut now = monotonic_us()?;
            if bridge.state.lock().unwrap().ready && started.is_none() {
                started = Some(Instant::now());
                controller.start_at(now);
                let (shared, stopped) = (bridge.clone(), stop.clone());
                let source_frames = std::mem::take(&mut frames);
                producer = Some(thread::spawn(move || {
                    let begin = Instant::now();
                    for (i, (offset, path)) in source_frames.into_iter().enumerate() {
                        while begin.elapsed().as_micros() < (offset as u128)
                            && !stopped.load(Ordering::Relaxed)
                        {
                            thread::sleep(Duration::from_millis(1));
                        }
                        if stopped.load(Ordering::Relaxed) {
                            break;
                        }
                        if let Some(path) = path {
                            let next = (|| -> Result<(), Box<dyn Error>> {
                                let bytes = jpeg(&path)?;
                                let source_us = monotonic_us()?;
                                shared.publish(Frame {
                                    bytes,
                                    source_us,
                                    sequence: i as u32,
                                })?;
                                Ok(())
                            })();
                            if let Err(error) = next {
                                shared.state.lock().unwrap().error = Some(error.to_string());
                                break;
                            }
                        }
                    }
                    shared.state.lock().unwrap().eof = true;
                    shared.changed.notify_all();
                }));
            }
            if started.is_none() && prepared.elapsed() > Duration::from_secs(30) {
                return Err("worker preparation timeout".into());
            }
            if started.is_some_and(|s| {
                s.elapsed() > Duration::from_micros(duration) + Duration::from_secs(5)
            }) {
                return Err("fixture completion timeout".into());
            }
            if let Some(error) = bridge.state.lock().unwrap().error.clone() {
                return Err(error.into());
            }
            tracker.tick(now);
            for capture in std::mem::take(&mut bridge.state.lock().unwrap().captures) {
                emit(capture)?;
            }
            let submission = bridge.state.lock().unwrap().submission.take();
            now = monotonic_us()?;
            let output = if let Some(submission) = submission {
                let lease = submission.lease;
                let packet = provider::Frame {
                    sequence: lease.sequence,
                    source_us: lease.frame.source_us,
                    now_us: now,
                    width: 640,
                    height: 480,
                    hands: submission.hands,
                };
                let found = tracker.observe(&packet)?;
                let hands: Vec<_> = found
                    .hands
                    .iter()
                    .filter_map(|h| config.observation(Some(h), 640, 480, packet.source_us).hand)
                    .collect();
                let state = controller.observe(
                    now,
                    touch_gestures::Frame {
                        version: 1,
                        source_us: packet.source_us,
                        ambiguous: found.ambiguous,
                        hands: hands.clone(),
                    },
                    None,
                );
                feedback.source_us = packet.source_us;
                feedback.jpeg = lease.frame.bytes.clone();
                feedback.sequence = lease.sequence;
                feedback.meaning_us = monotonic_us()?;
                feedback.eligible = hands.iter().any(|h| h.engaged && h.pinch_eligible);
                feedback.raw=found.hands.iter().map(|h|json!({"id":h.id,"confidence":h.confidence,
                    "handedness":h.handedness,"landmarks":h.landmarks.map(|p|[p.x,p.y]),"world_landmarks":h.world_landmarks})).collect();
                let receipt = json!({"sequence":lease.sequence,"source_us":packet.source_us,"now_us":now,
                    "age_ms":(feedback.meaning_us-packet.source_us)as f64/1000.0,"contacts":state.contacts,"mode":state.mode});
                bridge.finish(lease.sequence, receipt);
                emit(
                    json!({"event":"frame","sequence":lease.sequence,"capture_sequence":lease.frame.sequence,
                    "source_us":packet.source_us,"issued_us":lease.issued_us,"received_us":submission.received_us,"now_us":now,"meaning_us":feedback.meaning_us,
                    "ambiguous":found.ambiguous,"ambiguity_reason":found.ambiguity_reason,"hands":hands,
                    "perception":feedback.raw,"controller":state,"camera_opened":false,"system_input":false}),
                )?;
                state
            } else {
                controller.tick(now)
            };
            let active = (
                output.contacts.iter().map(|c| c.id).collect::<Vec<_>>(),
                output.button_down,
            );
            if active != previous_active {
                emit(
                    json!({"event":"intent_transition","now_us":now,"last_source_us":feedback.source_us,
                    "controller":output,"camera_opened":false,"system_input":false}),
                )?;
                previous_active = active;
            }
            {
                let mut s = bridge.state.lock().unwrap();
                if s.lease
                    .as_ref()
                    .is_some_and(|l| now.saturating_sub(l.frame.source_us) > bridge.stale_us)
                    && s.submission.is_none()
                {
                    let lease = s.lease.take().unwrap();
                    s.expired += 1;
                    emit(json!({"event":"lease_expired","sequence":lease.sequence,
                        "source_us":lease.frame.source_us,"now_us":now,"camera_opened":false,"system_input":false}))?;
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
            if last_publish.elapsed() > Duration::from_millis(40) {
                let remain = started.map_or(30000, |s| {
                    (duration / 1000 + 2000).saturating_sub(s.elapsed().as_millis() as u64)
                });
                preview.publish(
                    feedback.jpeg.clone(),
                    metadata(&config, &output, &feedback, now, remain, started.is_some()),
                )?;
                last_publish = Instant::now();
            }
            if ended.is_some_and(|s| s.elapsed() > Duration::from_secs(1)) {
                break;
            }
            if let Some(control) = preview.take_controller_control() {
                let control_time = monotonic_us()?;
                let result = match control {
                    crate::preview::ControllerControl::Mode(mode) => {
                        controller.set_mode_at(mode, control_time)
                    }
                    crate::preview::ControllerControl::SelectHand(hand_id) => {
                        controller.select_hand_at(hand_id, control_time)
                    }
                    crate::preview::ControllerControl::Pause => controller.stop(),
                    crate::preview::ControllerControl::Resume => controller.start_at(control_time),
                };
                emit(
                    json!({"event":"control","now_us":control_time,"controller":result,"camera_opened":false,"system_input":false}),
                )?;
            }
            thread::sleep(Duration::from_millis(5));
        }
        Ok(())
    })();
    stop.store(true, Ordering::Relaxed);
    if let Some(p) = producer {
        let _ = p.join();
    }
    let final_state = controller.stop();
    let s = bridge.state.lock().unwrap();
    emit(
        json!({"event":"stop","controller":final_state,"captured":s.captured,"issued":s.issued,"skipped":s.skipped,"expired":s.expired,
        "worker_closed":s.closed,"error":outcome.as_ref().err().map(ToString::to_string),"camera_opened":false,"system_input":false}),
    )?;
    outcome
}

struct Feedback {
    jpeg: Vec<u8>,
    source_us: u64,
    raw: Vec<Value>,
    sequence: u64,
    meaning_us: u64,
    eligible: bool,
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
    json!({"active":true,"ui_fixture":true,"input_enabled":false,"input_available":false,"recording":false,
        "width":640,"height":480,"source_us":source,"sequence":sequence,"age_ms":age,"frame_age_ms":age,
        "fresh":source>0&&age<(config.freshness_ms as f64),"freshness_ms":config.freshness_ms,"remaining_ms":remaining,
        "test_phase":if started {"testing"}else{"preparing"},"decode_ms":0,"inference_ms":0,
        "provider_transport":true,"frame_to_meaning_ms":meaning.saturating_sub(source)as f64/1000.0,
        "landmarks":first.map(|h|&h["landmarks"]),"confidence":first.map(|h|&h["confidence"]),
        "perception":perception,"controller":state,"visual_macros":false,"cursor":state.pointer_cursor,
        "eligible":eligible,"pinch_ratio":null,"presses":0,"pinch_state":if state.contacts.is_empty(){"touch"}else{"pressed"},
        "confirmation_elapsed_ms":state.confirmation_elapsed_us/1000,"confirmation_required_ms":config.gestures().confirm_us/1000,
        "close_ratio":config.pinch_close_ratio,"open_ratio":config.pinch_open_ratio,"blocked_reason":state.reason})
}

#[cfg(test)]
mod tests {
    use super::*;
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
    fn bridge() -> Arc<Bridge> {
        Arc::new(Bridge::new(BTreeMap::new(), 1_000_000, "test-1".into()))
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
