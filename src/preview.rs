//! Finite loopback-only live feedback; frames exist only in the latest memory slot.
use crate::calibration::Control;
use serde_json::Value;
#[derive(Default)]
struct Controls {
    enabled: bool,
    pending: Option<Control>,
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
}
impl Snapshot {
    fn metadata_at(&self, now: Instant) -> io::Result<String> {
        if let Some(expires) = self.expires {
            let mut metadata: Value = serde_json::from_str(&self.metadata)?;
            metadata["remaining_ms"] =
                Value::from(expires.saturating_duration_since(now).as_millis() as u64);
            Ok(serde_json::to_string(&metadata)?)
        } else {
            Ok(self.metadata.clone())
        }
    }
}
pub struct Preview {
    slot: Arc<Mutex<Option<Arc<Snapshot>>>>,
    stop: Arc<AtomicBool>,
    worker: Option<thread::JoinHandle<()>>,
    ready: Arc<AtomicBool>,
    pub port: u16,
    controls: Arc<Mutex<Controls>>,
}
impl Preview {
    pub fn start(port: u16, stop: Arc<AtomicBool>) -> io::Result<Self> {
        let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port))?;
        listener.set_nonblocking(true)?;
        let port = listener.local_addr()?.port();
        let slot = Arc::new(Mutex::new(None));
        let shared = slot.clone();
        let stopped = stop.clone();
        let ready = Arc::new(AtomicBool::new(false));
        let acknowledged = ready.clone();
        let controls = Arc::new(Mutex::new(Controls::default()));
        let commands = controls.clone();
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
                        );
                    }
                    Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(10))
                    }
                    Err(_) => break,
                }
            }
            *shared.lock().unwrap() = None;
        });
        Ok(Self {
            slot,
            stop,
            worker: Some(worker),
            port,
            ready,
            controls,
        })
    }
    pub fn enable_calibration_controls(&self) {
        self.controls.lock().unwrap().enabled = true;
    }
    pub fn take_control(&self) -> Option<Control> {
        self.controls.lock().unwrap().pending.take()
    }
    pub fn view_ready(&self) -> bool {
        self.ready.load(Ordering::Relaxed)
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
        *self.slot.lock().unwrap() = Some(Arc::new(Snapshot {
            jpeg,
            metadata,
            expires,
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
        *self.slot.lock().unwrap() = None;
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
    slot: &Mutex<Option<Arc<Snapshot>>>,
    stop: &AtomicBool,
    ready: &AtomicBool,
    controls: &Mutex<Controls>,
    port: u16,
) -> io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_millis(250)))?;
    stream.set_write_timeout(Some(Duration::from_millis(250)))?;
    let mut request = Vec::new();
    let mut chunk = [0; 1024];
    while !request.windows(4).any(|w| w == b"\r\n\r\n") {
        let count = stream.read(&mut chunk)?;
        if count == 0 {
            return Ok(());
        }
        request.extend_from_slice(&chunk[..count]);
        if request.len() > 4096 {
            return respond(
                stream,
                "431 Request Header Fields Too Large",
                "text/plain",
                b"Too large",
                "",
            );
        }
    }
    let request =
        std::str::from_utf8(&request).map_err(|_| io::Error::other("invalid HTTP request"))?;
    let mut lines = request.split("\r\n");
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
            let frame = slot.lock().unwrap().clone();
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
            let frame = slot.lock().unwrap().clone();
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
        "POST /stop HTTP/1.1"
        | "POST /ready HTTP/1.1"
        | "POST /calibration/start HTTP/1.1"
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
            let action = match first {
                "POST /calibration/start HTTP/1.1" => Some(Control::Start),
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
    fn cached_frame_does_not_extend_the_session_deadline() {
        let now = Instant::now();
        let snapshot = Snapshot {
            jpeg: vec![],
            metadata: "{\"remaining_ms\":1000}".into(),
            expires: Some(now + Duration::from_secs(1)),
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
        let port = preview.port;
        drop(preview);
        assert!(TcpStream::connect((std::net::Ipv4Addr::LOCALHOST, port)).is_err());
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
