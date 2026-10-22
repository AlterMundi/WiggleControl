//! Existing Linux MJPEG producer, shared by native and browser sessions.
use crate::{camera_frame::Frame, session_config::SessionConfig, session_io::Mailbox};
use std::{
    error::Error,
    io,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
    time::{Duration, Instant},
};
use v4l::{
    buffer::{Flags, Type},
    device::Device,
    io::traits::CaptureStream,
    video::Capture,
};
struct Closure(Arc<Mutex<Mailbox>>);
impl Drop for Closure {
    fn drop(&mut self) {
        if let Ok(mut slot) = self.0.lock() {
            slot.unclosed = false;
        }
    }
}
pub(crate) fn camera(
    config: &SessionConfig,
    stop: Arc<AtomicBool>,
    mailbox: Arc<Mutex<Mailbox>>,
    mut recording: Option<crate::recording::Frames>,
    deadline: Option<Instant>,
) -> thread::JoinHandle<()> {
    mailbox.lock().unwrap().unclosed = true;
    let path = config.camera.clone();
    let resolution = config.camera_resolution;
    thread::spawn(move || {
        let _closure = Closure(mailbox.clone());
        let work = (|| -> Result<(), Box<dyn Error>> {
            let device = Device::with_path(path)
                .map_err(|e| io::Error::new(e.kind(), format!("camera open: {e}")))?;
            mailbox.lock().unwrap().opened = true;
            let mut format = device
                .format()
                .map_err(|e| io::Error::new(e.kind(), format!("camera get format: {e}")))?;
            let [width, height] = resolution;
            format.width = width;
            format.height = height;
            format.fourcc = v4l::FourCC::new(b"MJPG");
            let actual = device.set_format(&format).map_err(|e| {
                io::Error::new(e.kind(), format!("camera set {width}x{height} MJPEG: {e}"))
            })?;
            if actual.width != width || actual.height != height || actual.fourcc != format.fourcc {
                return Err(format!("camera did not accept {width}x{height} MJPEG").into());
            }
            device
                .set_params(&v4l::video::capture::Parameters::with_fps(30))
                .map_err(|e| io::Error::new(e.kind(), format!("camera set 30 FPS: {e}")))?;
            let mut stream =
                crate::camera_mmap::Stream::with_buffers(&device, Type::VideoCapture, 2)
                    .map_err(|e| io::Error::new(e.kind(), format!("camera map buffers: {e}")))?;
            stream.set_timeout(Duration::from_millis(250));
            while !stop.load(Ordering::Relaxed) && deadline.is_none_or(|end| Instant::now() < end) {
                let (buffer, metadata) = match stream.next() {
                    Ok(v) => v,
                    Err(e) if e.kind() == io::ErrorKind::TimedOut => continue,
                    Err(e) => {
                        return Err(
                            io::Error::new(e.kind(), format!("camera stream next: {e}")).into()
                        )
                    }
                };
                if stop.load(Ordering::Relaxed) || deadline.is_some_and(|end| Instant::now() >= end)
                {
                    break;
                }
                if metadata.flags.contains(Flags::ERROR) {
                    continue;
                }
                if (metadata.flags & Flags::TIMESTAMP_MASK) != Flags::TIMESTAMP_MONOTONIC
                    || metadata.timestamp.sec < 0
                    || !(0..1_000_000).contains(&metadata.timestamp.usec)
                {
                    return Err("driver monotonic frame timestamp unavailable".into());
                }
                let size = metadata.bytesused as usize;
                if size > 2 * 1024 * 1024 || size > buffer.len() {
                    return Err("camera buffer exceeds bounded payload".into());
                }
                let frame = Frame {
                    bytes: buffer[..size].to_vec(),
                    source_us: metadata.timestamp.sec as u64 * 1_000_000
                        + metadata.timestamp.usec as u64,
                    sequence: metadata.sequence,
                };
                if let Some(recording) = &mut recording {
                    recording
                        .append(&frame.bytes, frame.source_us, frame.sequence)
                        .map_err(|e| {
                            io::Error::new(e.kind(), format!("camera record frame: {e}"))
                        })?;
                }
                let mut slot = mailbox.lock().unwrap();
                slot.frames += 1;
                slot.source_kind = Some(if metadata.flags.contains(Flags::TSTAMP_SRC_SOE) {
                    "monotonic_start_of_exposure"
                } else {
                    "monotonic_end_of_frame"
                });
                if slot.frame.replace(frame).is_some() {
                    slot.dropped += 1;
                }
            }
            Ok(())
        })();
        if let Err(error) = work {
            mailbox.lock().unwrap().error = Some(error.to_string());
        }
    })
}
