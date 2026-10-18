//! Finite loopback-only live feedback; frames exist only in the latest memory slot.
use crate::calibration::Control;
use serde_json::Value;
#[derive(Debug, serde::Serialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum ControllerControl {
    Mode(crate::controller::Mode),
    SelectHand(Option<u64>),
    Pause,
    Resume,
}
#[derive(Default)]
struct Controls {
    enabled: bool,
    pending: Option<Control>,
    guided: bool,
    painted_step: Option<usize>,
    controller_enabled: bool,
    pending_controller: Option<ControllerControl>,
}
use std::{
    io::{self, Read, Write},
    net::{TcpListener, TcpStream},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
    time::{Duration, Instant},
};
struct Snapshot {
    jpeg: Vec<u8>,
    metadata: String,
    expires: Option<Instant>,
    guided_expires: Option<Instant>,
    published_at: Instant,
}
#[derive(Default)]
struct Slot {
    frame: Option<Arc<Snapshot>>,
    requested: Option<Instant>,
}
impl Slot {
    fn read(&mut self) -> Option<Arc<Snapshot>> {
        self.requested = Some(Instant::now());
        self.frame.clone()
    }
    fn requested_recently(&self) -> bool {
        self.requested
            .is_some_and(|t| t.elapsed() < Duration::from_millis(500))
    }
}
/// Optional same-origin endpoint owned by an explicitly selected session.
pub(crate) trait Extension: Send + Sync {
    fn handles(&self, path: &str) -> bool;
    fn respond(&self, method: &str, path: &str, body: &[u8]) -> io::Result<ExtensionResponse>;
}
pub(crate) struct ExtensionResponse {
    pub status: &'static str,
    pub mime: &'static str,
    pub body: Vec<u8>,
    pub headers: String,
}
impl Snapshot {
    fn metadata_at(&self, now: Instant) -> io::Result<String> {
        let mut metadata: Value = serde_json::from_str(&self.metadata)?;
        if let Some(expires) = self.expires {
            metadata["remaining_ms"] =
                Value::from(expires.saturating_duration_since(now).as_millis() as u64);
        }
        if let Some(expires) = self.guided_expires {
            metadata["guided"]["remaining_ms"] =
                Value::from(expires.saturating_duration_since(now).as_millis() as u64);
        }
        let elapsed_ms = now
            .saturating_duration_since(self.published_at)
            .as_secs_f64()
            * 1000.0;
        for key in ["age_ms", "frame_age_ms"] {
            if let Some(age) = metadata.get(key).and_then(Value::as_f64) {
                metadata[key] = Value::from(age + elapsed_ms);
            }
        }
        if let (Some(fresh), Some(age), Some(limit)) = (
            metadata.get("fresh").and_then(Value::as_bool),
            metadata.get("age_ms").and_then(Value::as_f64),
            metadata.get("freshness_ms").and_then(Value::as_f64),
        ) {
            metadata["fresh"] = Value::from(fresh && age < limit);
        }
        Ok(serde_json::to_string(&metadata)?)
    }
}
pub struct Preview {
    slot: Arc<Mutex<Slot>>,
    stop: Arc<AtomicBool>,
    worker: Option<thread::JoinHandle<()>>,
    ready: Arc<AtomicBool>,
    pub port: u16,
    controls: Arc<Mutex<Controls>>,
    extension: Arc<Mutex<Option<Arc<dyn Extension>>>>,
}
impl Preview {
    pub fn start(port: u16, stop: Arc<AtomicBool>) -> io::Result<Self> {
        let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port))?;
        listener.set_nonblocking(true)?;
        let port = listener.local_addr()?.port();
        let slot = Arc::new(Mutex::new(Slot::default()));
        let shared = slot.clone();
        let stopped = stop.clone();
        let ready = Arc::new(AtomicBool::new(false));
        let acknowledged = ready.clone();
        let controls = Arc::new(Mutex::new(Controls::default()));
        let commands = controls.clone();
        let extension = Arc::new(Mutex::new(None::<Arc<dyn Extension>>));
        let extra = extension.clone();
        let worker = thread::spawn(move || {
            while !stopped.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        let _ = serve(
                            &mut stream,
                            &shared,
                            &stopped,
                            &acknowledged,
                            &commands,
                            port,
                            &extra,
                        );
                    }
                    Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(if extra.lock().unwrap().is_some() {
                            2
                        } else {
                            10
                        }))
                    }
                    Err(_) => break,
                }
            }
            shared.lock().unwrap().frame = None;
        });
        Ok(Self {
            slot,
            stop,
            worker: Some(worker),
            port,
            ready,
            controls,
            extension,
        })
    }
    pub fn enable_calibration_controls(&self) {
        self.controls.lock().unwrap().enabled = true;
    }
    pub(crate) fn enable_extension(&self, extension: Arc<dyn Extension>) {
        *self.extension.lock().unwrap() = Some(extension);
    }
    pub fn enable_guided(&self) {
        self.controls.lock().unwrap().guided = true;
    }
    pub fn guided_ready(&self, step: usize) -> bool {
        self.controls.lock().unwrap().painted_step == Some(step)
    }
    pub fn take_control(&self) -> Option<Control> {
        self.controls.lock().unwrap().pending.take()
    }
    pub fn enable_controller_controls(&self) {
        self.controls.lock().unwrap().controller_enabled = true;
    }
    pub fn take_controller_control(&self) -> Option<ControllerControl> {
        self.controls.lock().unwrap().pending_controller.take()
    }
    pub fn view_ready(&self) -> bool {
        self.ready.load(Ordering::Relaxed)
    }
    /// Independent of inference/controls: only actual image/state consumers
    /// request the expensive feedback snapshot, including after reopening a view.
    pub(crate) fn requested_recently(&self) -> bool {
        self.slot.lock().unwrap().requested_recently()
    }
    pub fn publish(&self, jpeg: Vec<u8>, data: Value) -> io::Result<()> {
        if jpeg.len() > 2 * 1024 * 1024 {
            return Err(io::Error::other("preview frame too large"));
        }
        let metadata = serde_json::to_string(&data)?;
        let expires = data
            .get("remaining_ms")
            .and_then(Value::as_u64)
            .map(|remaining| Instant::now() + Duration::from_millis(remaining.min(120_000)));
        if metadata.len() > 8192 || metadata.contains(['\r', '\n']) {
            return Err(io::Error::other("preview metadata too large or invalid"));
        }
        self.slot.lock().unwrap().frame = Some(Arc::new(Snapshot {
            jpeg,
            metadata,
            expires,
            published_at: Instant::now(),
            guided_expires: data
                .get("guided")
                .and_then(|g| g.get("remaining_ms"))
                .and_then(Value::as_u64)
                .map(|ms| Instant::now() + Duration::from_millis(ms.min(20_000))),
        }));
        Ok(())
    }
}
impl Drop for Preview {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        self.slot.lock().unwrap().frame = None;
    }
}
fn respond(
    stream: &mut TcpStream,
    status: &str,
    mime: &str,
    data: &[u8],
    extra: &str,
) -> io::Result<()> {
    write!(stream,"HTTP/1.1 {status}\r\nContent-Type: {mime}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nConnection: close\r\n{extra}\r\n",data.len())?;
    stream.write_all(data)
}
fn serve(
    stream: &mut TcpStream,
    slot: &Mutex<Slot>,
    stop: &AtomicBool,
    ready: &AtomicBool,
    controls: &Mutex<Controls>,
    port: u16,
    extension: &Mutex<Option<Arc<dyn Extension>>>,
) -> io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_millis(250)))?;
    stream.set_write_timeout(Some(Duration::from_millis(250)))?;
    let read_deadline = Instant::now() + Duration::from_millis(250);
    let mut request = Vec::new();
    let mut chunk = [0; 1024];
    let header_end = loop {
        if let Some(end) = request.windows(4).position(|w| w == b"\r\n\r\n") {
            if end + 4 <= 4096 {
                break end + 4;
            }
        }
        if request.len() >= 4096 {
            return respond(
                stream,
                "431 Request Header Fields Too Large",
                "text/plain",
                b"Too large",
                "",
            );
        }
        let remaining = read_deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "HTTP read deadline",
            ));
        }
        stream.set_read_timeout(Some(remaining))?;
        let count = stream.read(&mut chunk)?;
        if count == 0 {
            return Ok(());
        }
        request.extend_from_slice(&chunk[..count]);
    };
    let head = std::str::from_utf8(&request[..header_end])
        .map_err(|_| io::Error::other("invalid HTTP request"))?;
    let mut lines = head.split("\r\n");
    let first = lines.next().unwrap_or_default();
    let headers: Vec<_> = lines.filter_map(|line| line.split_once(':')).collect();
    let host = format!("127.0.0.1:{port}");
    if !headers
        .iter()
        .any(|(k, v)| k.eq_ignore_ascii_case("host") && v.trim() == host)
    {
        return respond(
            stream,
            "403 Forbidden",
            "text/plain",
            b"Local host required",
            "",
        );
    }
    let parts: Vec<_> = first.split_whitespace().collect();
    let endpoint = extension.lock().unwrap().clone();
    if let (Some(endpoint), [method, path, "HTTP/1.1"]) = (endpoint, parts.as_slice()) {
        if endpoint.handles(path) {
            if !["GET", "POST"].contains(method) {
                return respond(
                    stream,
                    "405 Method Not Allowed",
                    "text/plain",
                    b"Unsupported method",
                    "",
                );
            }
            if *method == "POST"
                && !headers.iter().any(|(k, v)| {
                    k.eq_ignore_ascii_case("origin") && v.trim() == format!("http://{host}")
                })
            {
                return respond(
                    stream,
                    "403 Forbidden",
                    "text/plain",
                    b"Same origin required",
                    "",
                );
            }
            let lengths: Vec<_> = headers
                .iter()
                .filter(|(k, _)| k.eq_ignore_ascii_case("content-length"))
                .collect();
            if lengths.len() > 1
                || headers
                    .iter()
                    .any(|(k, _)| k.eq_ignore_ascii_case("transfer-encoding"))
            {
                return respond(
                    stream,
                    "400 Bad Request",
                    "text/plain",
                    b"Invalid body framing",
                    "",
                );
            }
            let length = match lengths
                .first()
                .map(|(_, v)| v.trim().parse::<usize>())
                .transpose()
            {
                Ok(n) => n.unwrap_or(0),
                Err(_) => {
                    return respond(
                        stream,
                        "400 Bad Request",
                        "text/plain",
                        b"Invalid body length",
                        "",
                    )
                }
            };
            if length > crate::MAX_MESSAGE_BYTES {
                return respond(
                    stream,
                    "413 Content Too Large",
                    "text/plain",
                    b"Provider body exceeds16KiB",
                    "",
                );
            }
            // Copy the parsed route before extending the request buffer.
            let (method, path) = (method.to_string(), path.to_string());
            let mut body = request[header_end..].to_vec();
            if body.len() > length {
                return respond(
                    stream,
                    "400 Bad Request",
                    "text/plain",
                    b"Unexpected request body",
                    "",
                );
            }
            body.resize(length, 0);
            let already = request.len() - header_end;
            let mut offset = already;
            while offset < length {
                let remaining = read_deadline.saturating_duration_since(Instant::now());
                if remaining.is_zero() {
                    return Err(io::Error::new(
                        io::ErrorKind::TimedOut,
                        "HTTP body deadline",
                    ));
                }
                stream.set_read_timeout(Some(remaining))?;
                let n = stream.read(&mut body[offset..])?;
                if n == 0 {
                    return Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "incomplete HTTP body",
                    ));
                }
                offset += n;
            }
            let response = endpoint.respond(&method, &path, &body)?;
            return respond(
                stream,
                response.status,
                response.mime,
                &response.body,
                &response.headers,
            );
        }
    }
    match first {
        "GET / HTTP/1.1" => respond(
            stream,
            "200 OK",
            "text/html; charset=utf-8",
            include_bytes!("../tools/preview.html"),
            "",
        ),
        "GET /qualification HTTP/1.1" => respond(
            stream,
            "200 OK",
            "text/html; charset=utf-8",
            include_bytes!("../tools/qualification.html"),
            "",
        ),
        "GET /state HTTP/1.1" => {
            let frame = slot.lock().unwrap().read();
            if let Some(frame) = frame {
                respond(
                    stream,
                    "200 OK",
                    "application/json",
                    frame.metadata_at(Instant::now())?.as_bytes(),
                    "",
                )
            } else {
                respond(stream, "204 No Content", "text/plain", b"", "")
            }
        }
        "GET /frame HTTP/1.1" => {
            let frame = slot.lock().unwrap().read();
            if let Some(frame) = frame.as_ref() {
                respond(
                    stream,
                    "200 OK",
                    "image/jpeg",
                    &frame.jpeg,
                    &format!("X-Wiggle-State: {}\r\n", frame.metadata_at(Instant::now())?),
                )
            } else {
                respond(stream, "204 No Content", "text/plain", b"", "")
            }
        }
        path if path.starts_with("POST /controller/") && path.ends_with(" HTTP/1.1") => {
            let origin = format!("http://{host}");
            if !headers
                .iter()
                .any(|(k, v)| k.eq_ignore_ascii_case("origin") && v.trim() == origin)
            {
                return respond(
                    stream,
                    "403 Forbidden",
                    "text/plain",
                    b"Same origin required",
                    "",
                );
            }
            let mut controls = controls.lock().unwrap();
            if !controls.controller_enabled {
                return respond(
                    stream,
                    "403 Forbidden",
                    "text/plain",
                    b"Controller mode not enabled",
                    "",
                );
            }
            if controls.pending_controller.is_some() {
                return respond(
                    stream,
                    "409 Conflict",
                    "text/plain",
                    b"A controller control is pending",
                    "",
                );
            }
            let route = path
                .strip_prefix("POST /controller/")
                .unwrap()
                .strip_suffix(" HTTP/1.1")
                .unwrap();
            let action = if let Some(mode) = route.strip_prefix("mode/") {
                serde_json::from_value::<crate::controller::Mode>(Value::String(mode.to_owned()))
                    .ok()
                    .map(ControllerControl::Mode)
            } else if let Some(id) = route.strip_prefix("hand/") {
                if id == "auto" {
                    Some(ControllerControl::SelectHand(None))
                } else {
                    id.parse()
                        .ok()
                        .map(|id| ControllerControl::SelectHand(Some(id)))
                }
            } else {
                match route {
                    "pause" => Some(ControllerControl::Pause),
                    "resume" => Some(ControllerControl::Resume),
                    _ => None,
                }
            };
            let Some(action) = action else {
                return respond(
                    stream,
                    "400 Bad Request",
                    "text/plain",
                    b"Unknown controller control",
                    "",
                );
            };
            controls.pending_controller = Some(action);
            respond(
                stream,
                "200 OK",
                "text/plain",
                b"Queued; await observed controller state",
                "",
            )
        }
        "POST /stop HTTP/1.1"
        | "POST /ready HTTP/1.1"
        | "POST /guided-ready/0 HTTP/1.1"
        | "POST /guided-ready/1 HTTP/1.1"
        | "POST /guided-ready/2 HTTP/1.1"
        | "POST /guided-ready/3 HTTP/1.1"
        | "POST /guided-ready/4 HTTP/1.1"
        | "POST /guided-ready/5 HTTP/1.1"
        | "POST /calibration/start HTTP/1.1"
        | "POST /calibration/diagonal HTTP/1.1"
        | "POST /calibration/next HTTP/1.1"
        | "POST /calibration/cancel HTTP/1.1"
        | "POST /calibration/accept HTTP/1.1"
        | "POST /calibration/rollback HTTP/1.1" => {
            let origin = format!("http://{host}");
            if !headers
                .iter()
                .any(|(k, v)| k.eq_ignore_ascii_case("origin") && v.trim() == origin)
            {
                return respond(
                    stream,
                    "403 Forbidden",
                    "text/plain",
                    b"Same origin required",
                    "",
                );
            }
            if let Some(tail) = first.strip_prefix("POST /guided-ready/") {
                let step: usize = tail
                    .split(' ')
                    .next()
                    .unwrap()
                    .parse()
                    .map_err(io::Error::other)?;
                let snapshot = slot.lock().unwrap().frame.clone();
                let matches = snapshot
                    .as_ref()
                    .and_then(|s| serde_json::from_str::<Value>(&s.metadata).ok())
                    .is_some_and(|s| {
                        s["guided"]["step"] == serde_json::json!(step)
                            && s["guided"]["phase"] == "waiting"
                    });
                let mut control = controls.lock().unwrap();
                if !control.guided || !matches {
                    return respond(
                        stream,
                        "409 Conflict",
                        "text/plain",
                        b"Guided step is not awaiting paint",
                        "",
                    );
                }
                control.painted_step = Some(step);
                return respond(
                    stream,
                    "200 OK",
                    "text/plain",
                    b"Step paint acknowledged",
                    "",
                );
            }
            let action = match first {
                "POST /calibration/start HTTP/1.1" => Some(Control::Start),
                "POST /calibration/diagonal HTTP/1.1" => Some(Control::StartDiagonal),
                "POST /calibration/next HTTP/1.1" => Some(Control::Next),
                "POST /calibration/cancel HTTP/1.1" => Some(Control::Cancel),
                "POST /calibration/accept HTTP/1.1" => Some(Control::Accept),
                "POST /calibration/rollback HTTP/1.1" => Some(Control::Rollback),
                _ => None,
            };
            if let Some(action) = action {
                let mut controls = controls.lock().unwrap();
                if !controls.enabled {
                    return respond(
                        stream,
                        "403 Forbidden",
                        "text/plain",
                        b"Calibration mode not enabled",
                        "",
                    );
                }
                if controls.pending.is_some() {
                    return respond(
                        stream,
                        "409 Conflict",
                        "text/plain",
                        b"A control is already pending",
                        "",
                    );
                }
                controls.pending = Some(action);
                return respond(
                    stream,
                    "200 OK",
                    "text/plain",
                    b"Queued; await producer calibration state",
                    "",
                );
            }
            if first == "POST /ready HTTP/1.1" {
                ready.store(true, Ordering::Relaxed);
                respond(stream, "200 OK", "text/plain", b"View ready", "")
            } else {
                stop.store(true, Ordering::Relaxed);
                respond(stream, "200 OK", "text/plain", b"Stopping", "")
            }
        }
        _ => respond(stream, "404 Not Found", "text/plain", b"Not found", ""),
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feedback_demand_starts_from_a_read_expires_and_recovers_without_capture() {
        let preview = Preview::start(0, Arc::new(AtomicBool::new(false))).unwrap();
        assert!(!preview.requested_recently());
        let _ = request(preview.port, "GET /", false);
        assert!(!preview.requested_recently());
        // Even an empty first image request must enable the next publication.
        let response = request(preview.port, "GET /frame", false);
        assert!(String::from_utf8_lossy(&response).starts_with("HTTP/1.1 204"));
        assert!(preview.requested_recently());
        preview
            .publish(
                b"retained image".to_vec(),
                serde_json::json!({"active":true}),
            )
            .unwrap();
        preview.slot.lock().unwrap().requested = Some(Instant::now() - Duration::from_secs(1));
        assert!(!preview.requested_recently());
        let response = request(preview.port, "GET /state", false);
        assert!(String::from_utf8_lossy(&response).contains("\"active\":true"));
        assert!(preview.requested_recently());
    }
    struct Echo;
    impl Extension for Echo {
        fn handles(&self, path: &str) -> bool {
            path == "/provider/echo"
        }
        fn respond(&self, _: &str, _: &str, body: &[u8]) -> io::Result<ExtensionResponse> {
            Ok(ExtensionResponse {
                status: "200 OK",
                mime: "application/octet-stream",
                body: body.to_vec(),
                headers: String::new(),
            })
        }
    }
    #[test]
    fn provider_extension_keeps_header_body_bounds_origin_and_partial_body_separate() {
        let preview = Preview::start(0, Arc::new(AtomicBool::new(false))).unwrap();
        preview.enable_extension(Arc::new(Echo));
        let host = format!("127.0.0.1:{}", preview.port);
        let exchange = |headers: String, body: &[u8]| {
            let mut stream = TcpStream::connect(&host).unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            stream.write_all(headers.as_bytes()).unwrap();
            for chunk in body.chunks(3072) {
                stream.write_all(chunk).unwrap();
            }
            let mut out = Vec::new();
            let _ = stream.read_to_end(&mut out);
            out
        };
        let body = vec![b'x'; 14_000];
        let response=exchange(format!("POST /provider/echo HTTP/1.1\r\nHost: {host}\r\nOrigin: http://{host}\r\nContent-Length: {}\r\n\r\n",body.len()),&body);
        assert!(response.starts_with(b"HTTP/1.1 200"));
        assert!(response.ends_with(&body));
        for (extra, status) in [
            ("Content-Length: 16385\r\n", "413"),
            ("Content-Length: 1\r\nContent-Length: 1\r\n", "400"),
            ("Transfer-Encoding: chunked\r\n", "400"),
        ] {
            let result=exchange(format!("POST /provider/echo HTTP/1.1\r\nHost: {host}\r\nOrigin: http://{host}\r\n{extra}\r\n"),&[]);
            assert!(String::from_utf8_lossy(&result).starts_with(&format!("HTTP/1.1 {status}")));
        }
        assert!(exchange(
            format!("POST /provider/echo HTTP/1.1\r\nHost: {host}\r\n\r\n"),
            &[]
        )
        .starts_with(b"HTTP/1.1 403"));
        let mut incomplete = TcpStream::connect(&host).unwrap();
        incomplete.write_all(format!("POST /provider/echo HTTP/1.1\r\nHost: {host}\r\nOrigin: http://{host}\r\nContent-Length: 100\r\n\r\nx").as_bytes()).unwrap();
        thread::sleep(Duration::from_millis(320));
        assert!(exchange(
            format!("GET /provider/echo HTTP/1.1\r\nHost: {host}\r\n\r\n"),
            &[]
        )
        .starts_with(b"HTTP/1.1 200"));
    }
    #[test]
    fn guided_paint_ack_requires_selected_current_step_and_same_origin() {
        let preview = Preview::start(0, Arc::new(AtomicBool::new(false))).unwrap();
        preview
            .publish(
                vec![],
                serde_json::json!({"guided":{"step":0,"phase":"waiting"}}),
            )
            .unwrap();
        assert!(
            String::from_utf8_lossy(&request(preview.port, "POST /guided-ready/0", true))
                .starts_with("HTTP/1.1 409")
        );
        preview.enable_guided();
        assert!(
            String::from_utf8_lossy(&request(preview.port, "POST /guided-ready/0", false))
                .starts_with("HTTP/1.1 403")
        );
        assert!(!preview.guided_ready(0));
        assert!(
            String::from_utf8_lossy(&request(preview.port, "POST /guided-ready/0", true))
                .starts_with("HTTP/1.1 200")
        );
        assert!(preview.guided_ready(0));
        assert!(!preview.guided_ready(1));
        preview
            .publish(
                vec![],
                serde_json::json!({"guided":{"step":1,"phase":"waiting"}}),
            )
            .unwrap();
        assert!(
            String::from_utf8_lossy(&request(preview.port, "POST /guided-ready/0", true))
                .starts_with("HTTP/1.1 409")
        );
        assert!(
            String::from_utf8_lossy(&request(preview.port, "POST /calibration/next", true))
                .starts_with("HTTP/1.1 403")
        );
    }
    #[test]
    fn cached_guided_countdown_ages_without_new_frames() {
        let at = Instant::now();
        let snapshot = Snapshot {
            jpeg: vec![],
            metadata: "{\"guided\":{\"remaining_ms\":20000}}".into(),
            expires: None,
            guided_expires: Some(at + Duration::from_secs(20)),
            published_at: at,
        };
        let state: Value =
            serde_json::from_str(&snapshot.metadata_at(at + Duration::from_secs(3)).unwrap())
                .unwrap();
        assert_eq!(state["guided"]["remaining_ms"], 17000);
    }
    #[test]
    fn manual_cached_frame_ages_and_loses_freshness_without_new_capture() {
        let at = Instant::now();
        let snapshot = Snapshot {
            jpeg: vec![],
            metadata: r#"{"age_ms":20,"frame_age_ms":20,"fresh":true,"freshness_ms":100}"#.into(),
            expires: None,
            guided_expires: None,
            published_at: at,
        };
        let state: Value = serde_json::from_str(
            &snapshot
                .metadata_at(at + Duration::from_millis(90))
                .unwrap(),
        )
        .unwrap();
        assert_eq!(state["age_ms"].as_f64(), Some(110.0));
        assert_eq!(state["frame_age_ms"].as_f64(), Some(110.0));
        assert_eq!(state["fresh"], false);
    }
    #[test]
    fn cached_frame_does_not_extend_the_session_deadline() {
        let now = Instant::now();
        let snapshot = Snapshot {
            jpeg: vec![],
            metadata: "{\"remaining_ms\":1000}".into(),
            published_at: now,
            expires: Some(now + Duration::from_secs(1)),
            guided_expires: None,
        };
        let remaining = |at| {
            serde_json::from_str::<Value>(&snapshot.metadata_at(at).unwrap()).unwrap()
                ["remaining_ms"]
                .as_u64()
                .unwrap()
        };
        assert_eq!(remaining(now + Duration::from_millis(700)), 300);
        assert_eq!(remaining(now + Duration::from_secs(2)), 0);
    }
    fn request(port: u16, route: &str, origin: bool) -> Vec<u8> {
        let mut stream = TcpStream::connect((std::net::Ipv4Addr::LOCALHOST, port)).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        write!(
            stream,
            "{route} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n{}\r\n",
            if origin {
                format!("Origin: http://127.0.0.1:{port}\r\n")
            } else {
                String::new()
            }
        )
        .unwrap();
        let mut result = Vec::new();
        stream.read_to_end(&mut result).unwrap();
        result
    }
    #[test]
    fn latest_image_and_metadata_are_atomic_and_stop_is_scoped() {
        let stop = Arc::new(AtomicBool::new(false));
        let preview = Preview::start(0, stop.clone()).unwrap();
        preview
            .publish(vec![1, 2, 3], serde_json::json!({"sequence":7}))
            .unwrap();
        preview
            .publish(vec![4, 5, 6], serde_json::json!({"sequence":8}))
            .unwrap();
        let response = request(preview.port, "GET /frame", false);
        assert!(String::from_utf8_lossy(&response).contains("X-Wiggle-State: {\"sequence\":8}"));
        assert!(response.ends_with(&[4, 5, 6]));
        let state = request(preview.port, "GET /state", false);
        assert!(String::from_utf8_lossy(&state).contains("Content-Type: application/json"));
        assert!(state.ends_with(b"{\"sequence\":8}"));
        assert!(!preview.view_ready());
        assert!(
            String::from_utf8_lossy(&request(preview.port, "POST /ready", false))
                .starts_with("HTTP/1.1 403")
        );
        assert!(!preview.view_ready());
        assert!(
            String::from_utf8_lossy(&request(preview.port, "POST /ready", true))
                .starts_with("HTTP/1.1 200")
        );
        assert!(preview.view_ready());
        assert!(!stop.load(Ordering::Relaxed));
        assert!(
            String::from_utf8_lossy(&request(preview.port, "POST /stop", false))
                .starts_with("HTTP/1.1 403")
        );
        assert!(!stop.load(Ordering::Relaxed));
        assert!(
            String::from_utf8_lossy(&request(preview.port, "POST /stop", true))
                .starts_with("HTTP/1.1 200")
        );
        let owned_slot = preview.slot.clone();
        drop(preview);
        assert!(stop.load(Ordering::Relaxed));
        assert!(owned_slot.lock().unwrap().frame.is_none());
        // Other parallel previews may reuse a released ephemeral port. Verify
        // this preview's worker/owner clones are gone, not global port ownership.
        assert_eq!(Arc::strong_count(&owned_slot), 1);
    }
    #[test]
    fn controller_controls_require_enabled_same_origin_and_one_pending_command() {
        let preview = Preview::start(0, Arc::new(AtomicBool::new(false))).unwrap();
        let status = |route, origin| {
            String::from_utf8_lossy(&request(preview.port, route, origin))
                .split_whitespace()
                .nth(1)
                .unwrap()
                .to_owned()
        };
        assert_eq!(status("POST /controller/mode/precision", true), "403");
        preview.enable_controller_controls();
        assert_eq!(status("POST /controller/mode/precision", false), "403");
        assert_eq!(status("POST /controller/mode/unknown", true), "400");
        assert_eq!(status("POST /controller/mode/precision", true), "200");
        assert_eq!(status("POST /controller/pause", true), "409");
        assert!(matches!(
            preview.take_controller_control(),
            Some(ControllerControl::Mode(crate::controller::Mode::Precision))
        ));
        assert_eq!(status("POST /controller/hand/9", true), "200");
        assert!(matches!(
            preview.take_controller_control(),
            Some(ControllerControl::SelectHand(Some(9)))
        ));
        assert_eq!(status("POST /controller/pause", true), "200");
        assert!(matches!(
            preview.take_controller_control(),
            Some(ControllerControl::Pause)
        ));
        assert_eq!(status("POST /controller/resume", true), "200");
        assert!(matches!(
            preview.take_controller_control(),
            Some(ControllerControl::Resume)
        ));
        assert!(!preview.stop.load(Ordering::Relaxed));
    }
    #[test]
    fn calibration_control_requires_selected_mode_origin_and_bounded_consumption() {
        let preview = Preview::start(0, Arc::new(AtomicBool::new(false))).unwrap();
        assert!(
            String::from_utf8_lossy(&request(preview.port, "POST /calibration/start", true))
                .starts_with("HTTP/1.1 403")
        );
        preview.enable_calibration_controls();
        assert!(
            String::from_utf8_lossy(&request(preview.port, "POST /calibration/start", false))
                .starts_with("HTTP/1.1 403")
        );
        assert_eq!(preview.take_control(), None);
        assert!(
            String::from_utf8_lossy(&request(preview.port, "POST /calibration/start", true))
                .starts_with("HTTP/1.1 200")
        );
        assert!(
            String::from_utf8_lossy(&request(preview.port, "POST /calibration/accept", true))
                .starts_with("HTTP/1.1 409")
        );
        assert_eq!(preview.take_control(), Some(Control::Start));
        assert_eq!(preview.take_control(), None);
        for (path, action) in [
            ("diagonal", Control::StartDiagonal),
            ("next", Control::Next),
            ("cancel", Control::Cancel),
            ("accept", Control::Accept),
            ("rollback", Control::Rollback),
        ] {
            assert!(String::from_utf8_lossy(&request(
                preview.port,
                &format!("POST /calibration/{path}"),
                true
            ))
            .starts_with("HTTP/1.1 200"));
            assert_eq!(preview.take_control(), Some(action));
        }
    }
}
