//! Shared bounded JSONL reader for independent finite device writers.
use serde::de::DeserializeOwned;
use std::{
    io::{self, BufRead},
    sync::mpsc::{self, Receiver},
};
pub fn reader<T: DeserializeOwned + Send + 'static>() -> Receiver<io::Result<T>> {
    let (tx, rx) = mpsc::sync_channel(1);
    std::thread::spawn(move || {
        let mut input = io::stdin().lock();
        let mut line = Vec::with_capacity(crate::MAX_MESSAGE_BYTES);
        let parse = |line: &[u8]| {
            serde_json::from_slice(line).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
        };
        loop {
            let buf = match input.fill_buf() {
                Ok(b) => b,
                Err(e) => {
                    let _ = tx.send(Err(e));
                    return;
                }
            };
            if buf.is_empty() {
                if !line.is_empty() {
                    let _ = tx.send(parse(&line));
                }
                return;
            }
            let n = buf
                .iter()
                .position(|b| *b == b'\n')
                .map_or(buf.len(), |i| i + 1);
            if line.len() + n > crate::MAX_MESSAGE_BYTES {
                let _ = tx.send(Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "device intent exceeds 16 KiB",
                )));
                return;
            }
            line.extend_from_slice(&buf[..n]);
            input.consume(n);
            if line.last() == Some(&b'\n') {
                if tx.send(parse(&line)).is_err() {
                    return;
                }
                line.clear();
            }
        }
    });
    rx
}

/// Shared Linux monotonic time for same-host producer/device age checks.
pub fn monotonic_us() -> io::Result<u64> {
    let mut ts = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    if unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut ts) } != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(ts.tv_sec as u64 * 1_000_000 + ts.tv_nsec as u64 / 1000)
}
