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
}
impl SessionConfig {
    pub fn load(path: &Path) -> Result<Self, Box<dyn Error>> {
        let data = fs::read(path)?;
        if data.len() > 8192 {
            return Err("session config too large".into());
        }
        let config: Self = serde_json::from_slice(&data)?;
        if !(5..=20).contains(&config.fps)
            || !config.gain.is_finite()
            || !(1.0..=10000.0).contains(&config.gain)
        {
            return Err("fps 5..20; gain 1..10000".into());
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
    if !(1..=120).contains(&seconds) {
        return Err("session duration must be 1..120 seconds".into());
    }
    let config = SessionConfig::load(path)?;
    ort::init_from(&config.runtime).commit()?;
    let mut perception = Perception::new(Path::new(&config.models))?;
    let stop = Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(signal_hook::consts::SIGINT, stop.clone())?;
    signal_hook::flag::register(signal_hook::consts::SIGTERM, stop.clone())?;
    let mut writer = if inject {
        Some(
            Command::new(std::env::current_exe()?)
                .args([
                    "device-stream",
                    &seconds.to_string(),
                    &config.gain.to_string(),
                ])
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
    let start = Instant::now();
    let period = Duration::from_secs_f64(1.0 / config.fps as f64);
    let mut due = Instant::now();
    let mut engine = Engine::new(Config::default())?;
    engine.start();
    let mut summary = Summary {
        injected: inject,
        ..Default::default()
    };
    let mut latency = Vec::with_capacity(2400);
    let result = (|| -> Result<(), Box<dyn Error>> {
        while start.elapsed() < Duration::from_secs(seconds) && !stop.load(Ordering::Relaxed) {
            if let Some(writer) = writer.as_mut() {
                if writer.try_wait()?.is_some() {
                    return Err("device writer exited".into());
                }
            }
            write_events(&mut output, engine.tick(monotonic_us()?), false)?;
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
            let hand = perception.infer(&image)?;
            summary.processed += 1;
            if let Some(hand) = &hand {
                summary.detected += 1;
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
            let now = monotonic_us()?;
            let age = now.saturating_sub(frame.source_us);
            if age >= 150_000 {
                summary.stale += 1;
            }
            summary.button_down_intents +=
                write_events(&mut output, engine.observe(now, observation), true)?;
            if latency.len() < 2400 {
                latency.push(age as f64 / 1000.0);
            }
        }
        Ok(())
    })();
    stop.store(true, Ordering::Relaxed);
    let release = write_events(&mut output, engine.stop(), false);
    drop(output);
    if let Some(mut writer) = writer {
        let _ = writer.wait();
    }
    let _ = camera.join();
    let slot = mailbox.lock().unwrap();
    summary.captured = slot.frames;
    summary.dropped = slot.dropped;
    summary.source_kind = slot.source_kind.map(String::from);
    summary.elapsed_seconds = start.elapsed().as_secs_f64();
    summary.timestamp_to_intent_p50_ms = percentile(&mut latency, 50);
    summary.timestamp_to_intent_p95_ms = percentile(&mut latency, 95);
    println!("{}", serde_json::to_string(&summary)?);
    result?;
    release?;
    Ok(())
}
