//! Explicit finite camera sessions with a latest-frame slot and independent writer.
use crate::{vision::Perception, Config, Engine, Event, Observation};
use serde::{Deserialize, Serialize};
use std::{
    error::Error,
    fs,
    io::{self, Cursor, Write},
    path::Path,
    process::{Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
    time::{Duration, Instant},
};
use v4l::{
    buffer::{Flags, Type},
    io::traits::CaptureStream,
    prelude::*,
    video::Capture,
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionConfig {
    pub camera: String,
    pub models: String,
    pub runtime: String,
    pub fps: u32,
    pub gain: f64,
    #[serde(default)]
    pub touch_workspace: crate::touch_gestures::Workspace,
}
impl SessionConfig {
    pub fn load(path: &Path) -> Result<Self, Box<dyn Error>> {
        let data = fs::read(path)?;
        if data.len() > 8192 {
            return Err("session config too large".into());
        }
        let config: Self = serde_json::from_slice(&data)?;
        config.touch_workspace.validate()?;
        if !(10..=20).contains(&config.fps)
            || !config.gain.is_finite()
            || !(1.0..=10000.0).contains(&config.gain)
        {
            return Err("fps 10..20 (150 ms freshness budget); gain 1..10000".into());
        }
        for p in [&config.camera, &config.models, &config.runtime] {
            if !Path::new(p).is_absolute() {
                return Err("receiving paths must be absolute".into());
            }
        }
        Ok(config)
    }
}
struct Frame {
    bytes: Vec<u8>,
    source_us: u64,
    sequence: u32,
}
#[derive(Default)]
struct Mailbox {
    frame: Option<Frame>,
    error: Option<String>,
    dropped: u64,
    frames: u64,
    source_kind: Option<&'static str>,
}
fn monotonic_us() -> io::Result<u64> {
    let mut ts = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    if unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut ts) } != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(ts.tv_sec as u64 * 1_000_000 + ts.tv_nsec as u64 / 1000)
}
fn camera(
    config: &SessionConfig,
    stop: Arc<AtomicBool>,
    mailbox: Arc<Mutex<Mailbox>>,
) -> thread::JoinHandle<()> {
    let path = config.camera.clone();
    thread::spawn(move || {
        let work = (|| -> Result<(), Box<dyn Error>> {
            let device = Device::with_path(path)?;
            let mut format = device.format()?;
            format.width = 640;
            format.height = 480;
            format.fourcc = v4l::FourCC::new(b"MJPG");
            let actual = device.set_format(&format)?;
            if actual.width != 640 || actual.height != 480 || actual.fourcc != format.fourcc {
                return Err("camera did not accept 640x480 MJPEG".into());
            }
            device.set_params(&v4l::video::capture::Parameters::with_fps(30))?;
            let mut stream = MmapStream::with_buffers(&device, Type::VideoCapture, 2)?;
            stream.set_timeout(Duration::from_millis(250));
            while !stop.load(Ordering::Relaxed) {
                let (buffer, metadata) = match stream.next() {
                    Ok(v) => v,
                    Err(e) if e.kind() == io::ErrorKind::TimedOut => continue,
                    Err(e) => return Err(e.into()),
                };
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
#[derive(Serialize, Default)]
struct Summary {
    captured: u64,
    processed: u64,
    tracked: u64,
    detected: u64,
    button_down_intents: u64,
    dropped: u64,
    stale: u64,
    elapsed_seconds: f64,
    source_kind: Option<String>,
    timestamp_to_intent_p50_ms: Option<f64>,
    timestamp_to_intent_p95_ms: Option<f64>,
    injected: bool,
    touch_mode: bool,
    ambiguous_frames: u64,
    max_hands: usize,
    fresh_processed: u64,
    fresh_processed_fps: f64,
    resources: Option<crate::metrics::Resources>,
}
fn percentile(values: &mut [f64], percent: usize) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(f64::total_cmp);
    Some(
        values[(values.len() * percent)
            .div_ceil(100)
            .saturating_sub(1)
            .min(values.len() - 1)],
    )
}
fn write_events(
    output: &mut Option<std::process::ChildStdin>,
    events: Vec<Event>,
    heartbeat: bool,
) -> io::Result<u64> {
    let mut presses = 0;
    for event in events {
        let value = match event {
            Event::Move { dx, dy } => serde_json::json!({"type":"move","dx":dx,"dy":dy}),
            Event::Button { down } => {
                if down {
                    presses += 1;
                }
                serde_json::json!({"type":"button","down":down})
            }
            Event::Scroll { vertical } => {
                serde_json::json!({"type":"scroll","vertical":vertical,"horizontal":0})
            }
            Event::Cancel { .. } => serde_json::json!({"type":"button","down":false}),
        };
        if let Some(output) = output {
            serde_json::to_writer(&mut *output, &value)?;
            writeln!(output)?;
        }
    }
    if let Some(output) = output {
        if heartbeat {
            writeln!(output, "{{\"type\":\"heartbeat\"}}")?;
        }
        output.flush()?;
    }
    Ok(presses)
}
pub fn run(path: &Path, seconds: u64, inject: bool) -> Result<(), Box<dyn Error>> {
    run_mode(path, seconds, inject, false)
}
pub fn run_touch(path: &Path, seconds: u64, inject: bool) -> Result<(), Box<dyn Error>> {
    run_mode(path, seconds, inject, true)
}
fn write_contacts(
    output: &mut Option<std::process::ChildStdin>,
    contacts: Vec<crate::touch::Contact>,
) -> io::Result<()> {
    if let Some(output) = output {
        serde_json::to_writer(
            &mut *output,
            &serde_json::json!({"type":"frame","contacts":contacts}),
        )?;
        writeln!(output)?;
        output.flush()?;
    }
    Ok(())
}
fn run_mode(path: &Path, seconds: u64, inject: bool, touch: bool) -> Result<(), Box<dyn Error>> {
    if !(1..=120).contains(&seconds) {
        return Err("session duration must be 1..120 seconds".into());
    }
    let meter = crate::metrics::Meter::start();
    let config = SessionConfig::load(path)?;
    ort::init_from(&config.runtime).commit()?;
    let mut perception = Perception::new(Path::new(&config.models))?;
    let stop = Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(signal_hook::consts::SIGINT, stop.clone())?;
    signal_hook::flag::register(signal_hook::consts::SIGTERM, stop.clone())?;
    // Start the session deadline before spawning its independently finite writer.
    let start = Instant::now();
    let mut writer = if inject {
        Some(
            Command::new(std::env::current_exe()?)
                .args(if touch {
                    vec!["touch-stream".into(), seconds.to_string()]
                } else {
                    vec![
                        "device-stream".into(),
                        seconds.to_string(),
                        config.gain.to_string(),
                    ]
                })
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .spawn()?,
        )
    } else {
        None
    };
    let mut output = writer.as_mut().and_then(|w| w.stdin.take());
    let mailbox = Arc::new(Mutex::new(Mailbox::default()));
    let camera = camera(&config, stop.clone(), mailbox.clone());
    let period = Duration::from_secs_f64(1.0 / config.fps as f64);
    let mut due = Instant::now();
    let mut engine = Engine::new(Config::default())?;
    engine.start();
    let mut touches =
        crate::touch_gestures::Gestures::new(Config::default(), config.touch_workspace)?;
    touches.start();
    let mut summary = Summary {
        injected: inject,
        touch_mode: touch,
        ..Default::default()
    };
    let mut latency = Vec::with_capacity(2400);
    let result = (|| -> Result<(), Box<dyn Error>> {
        while start.elapsed() < Duration::from_secs(seconds) && !stop.load(Ordering::Relaxed) {
            if let Some(writer) = writer.as_mut() {
                if let Some(status) = writer.try_wait()? {
                    if status.success() && start.elapsed() >= Duration::from_secs(seconds) {
                        break;
                    }
                    return Err("device writer exited".into());
                }
            }
            if touch {
                write_contacts(&mut output, touches.tick(monotonic_us()?))?;
            } else {
                write_events(&mut output, engine.tick(monotonic_us()?), false)?;
            }
            let error = mailbox.lock().unwrap().error.clone();
            if let Some(error) = error {
                return Err(error.into());
            }
            if Instant::now() < due {
                thread::sleep(Duration::from_millis(10));
                continue;
            }
            let Some(frame) = mailbox.lock().unwrap().frame.take() else {
                thread::sleep(Duration::from_millis(10));
                continue;
            };
            let _sequence = frame.sequence;
            due = Instant::now() + period;
            let mut reader =
                image::ImageReader::new(Cursor::new(frame.bytes)).with_guessed_format()?;
            let mut limits = image::Limits::default();
            limits.max_image_width = Some(640);
            limits.max_image_height = Some(480);
            limits.max_alloc = Some(8 * 1024 * 1024);
            reader.limits(limits);
            let image = reader.decode()?.to_rgb8();
            if image.width() != 640 || image.height() != 480 {
                return Err("decoded camera dimensions changed".into());
            }
            let now;
            if touch {
                let detected = perception.infer_many(&image)?;
                if start.elapsed() >= Duration::from_secs(seconds) {
                    break;
                }
                now = monotonic_us()?;
                summary.processed += 1;
                summary.detected += detected.hands.len() as u64;
                summary.max_hands = summary.max_hands.max(detected.hands.len());
                summary.tracked += detected.hands.iter().filter(|h| h.tracked).count() as u64;
                summary.ambiguous_frames += u64::from(detected.ambiguous);
                let hands = detected
                    .hands
                    .into_iter()
                    .filter_map(|h| {
                        h.observation(image.width(), image.height(), frame.source_us)
                            .hand
                    })
                    .collect();
                write_contacts(
                    &mut output,
                    touches.observe(
                        now,
                        crate::touch_gestures::Frame {
                            version: 1,
                            source_us: frame.source_us,
                            ambiguous: detected.ambiguous,
                            hands,
                        },
                    ),
                )?;
            } else {
                let hand = perception.infer(&image)?;
                if start.elapsed() >= Duration::from_secs(seconds) {
                    break;
                }
                summary.processed += 1;
                if let Some(hand) = &hand {
                    summary.detected += 1;
                    summary.max_hands = 1;
                    if hand.tracked {
                        summary.tracked += 1;
                    }
                }
                let observation = hand
                    .map(|h| h.observation(image.width(), image.height(), frame.source_us))
                    .unwrap_or(Observation {
                        version: 1,
                        source_us: frame.source_us,
                        hand: None,
                    });
                now = monotonic_us()?;
                summary.button_down_intents +=
                    write_events(&mut output, engine.observe(now, observation), true)?;
            }
            let age = now.saturating_sub(frame.source_us);
            if age >= 150_000 {
                summary.stale += 1;
            } else if frame.source_us <= now {
                summary.fresh_processed += 1;
            }
            if latency.len() < 2400 {
                latency.push(age as f64 / 1000.0);
            }
        }
        Ok(())
    })();
    let interrupted = stop.load(Ordering::Relaxed);
    stop.store(true, Ordering::Relaxed);
    let release = if touch {
        write_contacts(&mut output, touches.stop())
    } else {
        write_events(&mut output, engine.stop(), false).map(|_| ())
    };
    drop(output);
    // Keep wait errors until the camera thread has also been joined.
    let writer_status = writer.map(|mut writer| writer.wait()).transpose();
    let writer_ok = writer_status
        .as_ref()
        .is_ok_and(|status| status.is_some_and(|status| status.success()));
    // Its finite deadline can expire while the final inference is completing.
    // Successful device teardown is already neutral; a crashed/early writer is not.
    let release = if start.elapsed() >= Duration::from_secs(seconds)
        && writer_ok
        && release
            .as_ref()
            .is_err_and(|error| error.kind() == io::ErrorKind::BrokenPipe)
    {
        Ok(())
    } else {
        release
    };
    let camera_joined = camera.join();
    let slot = mailbox.lock().unwrap();
    summary.captured = slot.frames;
    summary.dropped = slot.dropped;
    summary.source_kind = slot.source_kind.map(String::from);
    summary.elapsed_seconds = start.elapsed().as_secs_f64();
    summary.fresh_processed_fps = summary.fresh_processed as f64 / summary.elapsed_seconds;
    summary.resources = Some(meter.finish());
    summary.timestamp_to_intent_p50_ms = percentile(&mut latency, 50);
    summary.timestamp_to_intent_p95_ms = percentile(&mut latency, 95);
    println!("{}", serde_json::to_string(&summary)?);
    result?;
    check_writer_exit(writer_status?, interrupted)?;
    camera_joined.map_err(|_| "camera thread panicked")?;
    release?;
    Ok(())
}

fn check_writer_exit(
    status: Option<std::process::ExitStatus>,
    interrupted: bool,
) -> io::Result<()> {
    use std::os::unix::process::ExitStatusExt;
    match status {
        None => Ok(()),
        Some(status)
            if status.success()
                || (interrupted
                    && matches!(status.signal(), Some(libc::SIGINT | libc::SIGTERM))) =>
        {
            Ok(())
        }
        Some(status) => Err(io::Error::other(format!("device writer exited: {status}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::process::ExitStatusExt;

    #[test]
    fn writer_failure_at_teardown_is_not_success_or_operator_stop() {
        assert!(check_writer_exit(None, false).is_ok());
        assert!(check_writer_exit(Some(std::process::ExitStatus::from_raw(0)), false).is_ok());
        for interrupted in [false, true] {
            let failed = Command::new("sh").args(["-c", "exit 7"]).status().unwrap();
            assert!(check_writer_exit(Some(failed), interrupted).is_err());
            assert!(check_writer_exit(
                Some(std::process::ExitStatus::from_raw(libc::SIGKILL)),
                interrupted
            )
            .is_err());
        }
        for signal in [libc::SIGINT, libc::SIGTERM] {
            let signalled = std::process::ExitStatus::from_raw(signal);
            assert!(check_writer_exit(Some(signalled), false).is_err());
            assert!(check_writer_exit(Some(signalled), true).is_ok());
        }
    }

    #[test]
    fn session_config_loads_old_defaults_and_rejects_invalid_calibration() {
        let path =
            std::env::temp_dir().join(format!("wigglecontrol-config-{}.json", std::process::id()));
        let mut value = serde_json::json!({
            "camera":"/fictional/camera", "models":"/fictional/models",
            "runtime":"/fictional/runtime", "fps":10, "gain":2000
        });
        fs::write(&path, value.to_string()).unwrap();
        let config = SessionConfig::load(&path).unwrap();
        assert_eq!(config.touch_workspace.min.x, 0.0);
        assert_eq!(config.touch_workspace.max.y, 1.0);
        value["touch_workspace"] = serde_json::json!({
            "min":{"x":0.2,"y":0.2}, "max":{"x":0.8,"y":0.8}
        });
        fs::write(&path, value.to_string()).unwrap();
        assert_eq!(
            SessionConfig::load(&path).unwrap().touch_workspace.min.x,
            0.2
        );
        for invalid in [
            serde_json::json!({"min":{"x":0.8,"y":0.2},"max":{"x":0.2,"y":0.8}}),
            serde_json::json!({"min":{"x":0.2,"y":0.2},"max":{"x":0.21,"y":0.8}}),
            serde_json::json!({"min":{"x":-0.2,"y":0.2},"max":{"x":0.8,"y":0.8}}),
            serde_json::json!({"min":{"x":0.2,"y":0.2},"max":{"x":0.8,"y":0.8},"typo":true}),
        ] {
            value["touch_workspace"] = invalid;
            fs::write(&path, value.to_string()).unwrap();
            assert!(SessionConfig::load(&path).is_err());
        }
        fs::remove_file(path).unwrap();
    }
}
