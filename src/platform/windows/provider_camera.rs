//! Connect the supervised native camera to the common latest-frame session slot.
use crate::{
    platform::{
        monotonic_us,
        windows::camera::{Capture, Format, Spec},
    },
    session_config::SessionConfig,
    session_io::Mailbox,
};
use std::{
    io,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
    time::{Duration, Instant},
};
pub(crate) fn camera(
    config: &SessionConfig,
    stop: Arc<AtomicBool>,
    mailbox: Arc<Mutex<Mailbox>>,
    mut recording: Option<crate::recording::Frames>,
    deadline: Option<Instant>,
) -> thread::JoinHandle<()> {
    mailbox.lock().unwrap().unclosed = true;
    let camera = config.camera.clone();
    let resolution = config.camera_resolution;
    let freshness_us = config.gestures().stale_us;
    thread::spawn(move || {
        let work = (|| -> io::Result<()> {
            let spec = Spec {
                camera,
                format: Format {
                    resolution,
                    frame_rate: [30, 1],
                },
                freshness_us,
                run_id: format!("capture-{}-{}", std::process::id(), monotonic_us()?),
            };
            let capture = match Capture::spawn(&std::env::current_exe()?, spec) {
                Ok(capture) => capture,
                Err(error) => {
                    mailbox.lock().unwrap().unclosed =
                        crate::platform::windows::camera::failed_start_unclosed(&error);
                    return Err(error);
                }
            };
            let mut forwarded_drops = 0u64;
            let result = (|| -> io::Result<()> {
                while !stop.load(Ordering::Relaxed)
                    && deadline.is_none_or(|end| Instant::now() < end)
                {
                    let status = capture.status();
                    {
                        let mut slot = mailbox.lock().unwrap();
                        slot.opened |= status.negotiated.is_some();
                        slot.frames = status.fresh_frames;
                        slot.dropped = status.dropped.saturating_add(forwarded_drops);
                        slot.source_kind = Some("driver_qpc_timestamp");
                    }
                    if let Some(frame) = capture.take_frame()? {
                        if let Some(recording) = &mut recording {
                            recording.append(&frame.bytes, frame.source_us, frame.sequence)?;
                        }
                        let mut slot = mailbox.lock().unwrap();
                        if slot.frame.replace(frame).is_some() {
                            forwarded_drops = forwarded_drops.saturating_add(1);
                            slot.dropped = slot.dropped.saturating_add(1);
                        }
                    }
                    if status.process_exited {
                        return Err(io::Error::other("camera exited before session Stop"));
                    }
                    thread::sleep(Duration::from_millis(2));
                }
                Ok(())
            })();
            let status = capture.stop();
            mailbox.lock().unwrap().unclosed = !status.process_exited;
            result?;
            if !status.process_exited {
                return Err(io::Error::other("camera closure unconfirmed"));
            }
            if let Some(error) = status.error {
                return Err(io::Error::other(error));
            }
            Ok(())
        })();
        if let Err(error) = work {
            mailbox.lock().unwrap().error = Some(error.to_string());
        }
    })
}
