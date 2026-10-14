//! Finite camera-free local recorded-frame player and separate human interval labels.
use crate::recording::{FrameIndex, MAX_BYTES};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    io::{self, Read, Seek, SeekFrom, Write},
    net::{TcpListener, TcpStream},
    path::Path,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Interval {
    pub start_ms: f64,
    pub end_ms: f64,
}
fn valid_labels(labels: &[Interval], duration: f64) -> bool {
    labels.len() <= 256
        && labels.iter().all(|i| {
            i.start_ms.is_finite()
                && i.end_ms.is_finite()
                && i.start_ms >= 0.0
                && i.end_ms > i.start_ms
                && i.end_ms <= duration
        })
}
fn respond(s: &mut TcpStream, status: &str, mime: &str, body: &[u8]) -> io::Result<()> {
    write!(s,"HTTP/1.1 {status}\r\nContent-Type: {mime}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\nX-Content-Type-Options: nosniff\r\n\r\n",body.len())?;
    s.write_all(body)
}
pub fn serve(dir: &Path, port: u16, seconds: u64) -> io::Result<()> {
    if port == 0 || !(1..=3600).contains(&seconds) {
        return Err(io::Error::other(
            "review requires port and duration1..3600 seconds",
        ));
    }
    let raw = fs::read(dir.join("frames.jsonl"))?;
    if raw.len() > 1024 * 1024 {
        return Err(io::Error::other("index too large"));
    }
    let rows: Vec<FrameIndex> = String::from_utf8(raw)
        .map_err(io::Error::other)?
        .lines()
        .map(serde_json::from_str)
        .collect::<Result<_, _>>()?;
    let mut video = File::open(dir.join("video.mjpeg"))?;
    let size = video.metadata()?.len();
    if rows.is_empty()
        || rows.len() > 5000
        || size > MAX_BYTES
        || rows.iter().any(|r| {
            r.length == 0
                || r.length > 2 * 1024 * 1024
                || r.offset
                    .checked_add(r.length as u64)
                    .is_none_or(|n| n > size)
        })
        || rows.windows(2).any(|r| r[1].source_us <= r[0].source_us)
    {
        return Err(io::Error::other("invalid recording index"));
    }
    let times: Vec<f64> = rows
        .iter()
        .map(|r| (r.source_us - rows[0].source_us) as f64 / 1000.0)
        .collect();
    let duration = *times.last().unwrap();
    let manifest = serde_json::to_vec(
        &serde_json::json!({"schema":"wigglecontrol.review/v1","times_ms":times,"duration_ms":duration,"recording":false}),
    )?;
    let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port))?;
    listener.set_nonblocking(true)?;
    let until = Instant::now() + Duration::from_secs(seconds);
    while Instant::now() < until {
        let (mut stream, _) = match listener.accept() {
            Ok(s) => s,
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(10));
                continue;
            }
            Err(e) => return Err(e),
        };
        stream.set_read_timeout(Some(Duration::from_millis(500)))?;
        stream.set_write_timeout(Some(Duration::from_secs(2)))?;
        let result = (|| -> io::Result<()> {
            let mut request = Vec::new();
            let mut chunk = [0; 4096];
            while !request.windows(4).any(|w| w == b"\r\n\r\n") {
                let n = stream.read(&mut chunk)?;
                if n == 0 {
                    return Ok(());
                }
                request.extend_from_slice(&chunk[..n]);
                if request.len() > 8192 {
                    return Err(io::Error::other("headers too large"));
                }
            }
            let split = request.windows(4).position(|w| w == b"\r\n\r\n").unwrap() + 4;
            let header = std::str::from_utf8(&request[..split])
                .map_err(io::Error::other)?
                .to_owned();
            let mut lines = header.lines();
            let first = lines.next().unwrap_or("");
            let host = format!("127.0.0.1:{port}");
            let origin = format!("http://{host}");
            let headers: Vec<_> = lines.filter_map(|s| s.split_once(':')).collect();
            if !headers
                .iter()
                .any(|(k, v)| k.eq_ignore_ascii_case("host") && v.trim() == host)
            {
                return respond(
                    &mut stream,
                    "403 Forbidden",
                    "text/plain",
                    b"Local host required",
                );
            }
            if first == "GET / HTTP/1.1" {
                return respond(
                    &mut stream,
                    "200 OK",
                    "text/html; charset=utf-8",
                    include_bytes!("../tools/review.html"),
                );
            }
            if first == "GET /manifest HTTP/1.1" {
                return respond(&mut stream, "200 OK", "application/json", &manifest);
            }
            if first == "GET /labels HTTP/1.1" {
                let body = fs::read(dir.join("labels.json")).unwrap_or_else(|_| b"[]".to_vec());
                return respond(&mut stream, "200 OK", "application/json", &body);
            }
            if let Some(id) = first
                .strip_prefix("GET /frame/")
                .and_then(|s| s.strip_suffix(" HTTP/1.1"))
                .and_then(|s| s.parse::<usize>().ok())
            {
                if let Some(row) = rows.get(id) {
                    let mut bytes = vec![0; row.length];
                    video.seek(SeekFrom::Start(row.offset))?;
                    video.read_exact(&mut bytes)?;
                    return respond(&mut stream, "200 OK", "image/jpeg", &bytes);
                }
            }
            if first == "POST /labels HTTP/1.1" {
                if !headers
                    .iter()
                    .any(|(k, v)| k.eq_ignore_ascii_case("origin") && v.trim() == origin)
                {
                    return respond(
                        &mut stream,
                        "403 Forbidden",
                        "text/plain",
                        b"Same origin required",
                    );
                }
                let len = headers
                    .iter()
                    .find(|(k, _)| k.eq_ignore_ascii_case("content-length"))
                    .and_then(|(_, v)| v.trim().parse::<usize>().ok())
                    .filter(|n| *n <= 65536)
                    .ok_or_else(|| io::Error::other("invalid body length"))?;
                while request.len() < split + len {
                    let n = stream.read(&mut chunk)?;
                    if n == 0 {
                        return Err(io::Error::other("incomplete labels"));
                    }
                    request.extend_from_slice(&chunk[..n]);
                }
                let labels: Vec<Interval> = serde_json::from_slice(&request[split..split + len])?;
                if !valid_labels(&labels, duration) {
                    return respond(
                        &mut stream,
                        "400 Bad Request",
                        "text/plain",
                        b"Invalid intervals",
                    );
                }
                let bytes = serde_json::to_vec_pretty(&labels)?;
                let stamp = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map_err(io::Error::other)?
                    .as_nanos();
                crate::recording::private_file(&dir.join(format!("labels-{stamp}.json")))?
                    .write_all(&bytes)?;
                let temporary = dir.join(format!("labels-{stamp}.new"));
                crate::recording::private_file(&temporary)?.write_all(&bytes)?;
                fs::rename(temporary, dir.join("labels.json"))?;
                return respond(
                    &mut stream,
                    "200 OK",
                    "application/json",
                    b"{\"saved\":true}",
                );
            }
            respond(&mut stream, "404 Not Found", "text/plain", b"Not found")
        })();
        if result.is_err() {
            let _ = respond(
                &mut stream,
                "400 Bad Request",
                "text/plain",
                b"Invalid request",
            );
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn labels_reject_reversed_nonfinite_and_outside_video() {
        assert!(valid_labels(
            &[Interval {
                start_ms: 100.,
                end_ms: 300.
            }],
            500.
        ));
        assert!(!valid_labels(
            &[Interval {
                start_ms: 300.,
                end_ms: 100.
            }],
            500.
        ));
        assert!(!valid_labels(
            &[Interval {
                start_ms: 0.,
                end_ms: 600.
            }],
            500.
        ));
        assert!(!valid_labels(
            &[Interval {
                start_ms: f64::NAN,
                end_ms: 300.
            }],
            500.
        ));
    }
}
