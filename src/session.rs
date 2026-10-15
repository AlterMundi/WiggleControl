//! Explicit finite camera sessions with a latest-frame slot and independent writer.
use crate::{intent::monotonic_us, vision::Perception, Config, Engine, Event, Observation};
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
    #[serde(default = "default_freshness_ms")]
    pub freshness_ms: u64,
    #[serde(default)]
    pub pinch_3d: bool,
    /// Experimental veto of new presses using pixel tip-gap / wrist-middle-MCP.
    /// The raw pinch metric still owns confirmation, release and open rearm.
    #[serde(default)]
    pub pinch_onset_max_screen_ratio: Option<f64>,
    #[serde(default = "default_close_m")]
    pub pinch_close_m: f64,
    #[serde(default = "default_open_m")]
    pub pinch_open_m: f64,
    #[serde(default)]
    pub recording_directory: Option<String>,
    #[serde(default)]
    pub pointer_output: Option<[u32; 2]>,
    #[serde(default)]
    pub prayer_enabled: bool,
    #[serde(default)]
    pub touch_workspace: crate::touch_gestures::Workspace,
    #[serde(default = "default_close_ratio")]
    pub pinch_close_ratio: f64,
    #[serde(default = "default_open_ratio")]
    pub pinch_open_ratio: f64,
    #[serde(default)]
    pub adaptive_smoothing: Option<crate::filter::OneEuroConfig>,
    #[serde(default = "default_pair_separation_widths")]
    pub pair_separation_widths: f64,
    #[serde(default)]
    pub palm_deduplication: crate::vision::PalmDeduplication,
    #[serde(default)]
    pub dual_crop: bool,
}
fn default_pair_separation_widths() -> f64 {
    0.8
}
fn default_close_m() -> f64 {
    0.02
}
fn default_open_m() -> f64 {
    0.03
}
fn default_freshness_ms() -> u64 {
    150
}
fn default_close_ratio() -> f64 {
    Config::default().close_ratio
}
fn default_open_ratio() -> f64 {
    Config::default().open_ratio
}
impl SessionConfig {
    pub fn perception(&self) -> Result<Perception, Box<dyn Error>> {
        self.replay_perception(5, None)
    }
    pub(crate) fn replay_perception(
        &self,
        rediscovery: u32,
        separation_override: Option<f64>,
    ) -> Result<Perception, Box<dyn Error>> {
        Ok(Perception::with_replay_options(
            Path::new(&self.models),
            rediscovery,
            separation_override.unwrap_or(self.pair_separation_widths),
        )?
        .with_palm_deduplication(self.palm_deduplication)
        .with_dual_crop(self.dual_crop))
    }
    pub fn gestures(&self) -> Config {
        Config {
            stale_us: self.freshness_ms * 1000,
            close_m: self.pinch_close_m,
            open_m: self.pinch_open_m,
            close_ratio: self.pinch_close_ratio,
            open_ratio: self.pinch_open_ratio,
            adaptive_smoothing: self.adaptive_smoothing,
            ..Config::default()
        }
    }
    /// Apply the selected pinch interpretation identically in pointer, touch and replay.
    pub(crate) fn observation(
        &self,
        detected: Option<&crate::vision::DetectedHand>,
        width: u32,
        height: u32,
        source_us: u64,
    ) -> Observation {
        let mut observation = detected
            .map(|hand| hand.observation(width, height, source_us))
            .unwrap_or(Observation {
                version: 1,
                source_us,
                hand: None,
            });
        if self.pinch_3d {
            if let (Some(hand), Some(detected)) = (&mut observation.hand, detected) {
                hand.pinch_distance_m = Some(detected.world_pinch_distance_m());
                // The legacy 2D scroll chord must not override a 3D pinch.
                hand.scrolling = false;
            }
        }
        if let (Some(limit), Some(hand), Some(detected)) = (
            self.pinch_onset_max_screen_ratio,
            &mut observation.hand,
            detected,
        ) {
            let points = &detected.landmarks;
            let gap = (points[4].x - points[8].x).hypot(points[4].y - points[8].y);
            let palm = (points[0].x - points[9].x).hypot(points[0].y - points[9].y);
            let ratio = gap / palm;
            if !ratio.is_finite() || palm <= 1e-6 {
                // Missing geometry is observation loss, not a valid open/hold.
                hand.pinch_eligible = false;
            }
            hand.pinch_onset_allowed = ratio.is_finite() && palm > 1e-6 && ratio <= limit;
        }
        if self.dual_crop && self.pinch_3d {
            if let (Some(hand), Some(secondary)) = (
                &mut observation.hand,
                detected.and_then(|h| h.secondary.as_deref()),
            ) {
                let gap = secondary.world_pinch_distance_m();
                if gap.is_finite()
                    && (0.0..=1.0).contains(&gap)
                    && secondary.confidence.is_finite()
                    && (0.0..=1.0).contains(&secondary.confidence)
                {
                    let mut eligible = secondary
                        .observation(width, height, source_us)
                        .hand
                        .is_some_and(|h| h.pinch_eligible);
                    let allowed = if let Some(limit) = self.pinch_onset_max_screen_ratio {
                        let p = &secondary.landmarks;
                        let palm = (p[0].x - p[9].x).hypot(p[0].y - p[9].y);
                        let ratio = (p[4].x - p[8].x).hypot(p[4].y - p[8].y) / palm;
                        if palm <= 1e-6 || !ratio.is_finite() {
                            eligible = false;
                        }
                        palm > 1e-6 && ratio.is_finite() && ratio <= limit
                    } else {
                        true
                    };
                    hand.secondary_pinch = Some(crate::PinchEvidence {
                        source_id: crate::vision::PROXIMAL_PINCH_SOURCE,
                        source_us,
                        confidence: secondary.confidence,
                        eligible,
                        distance_m: gap,
                        onset_allowed: allowed,
                    });
                }
            }
        }
        observation
    }
    pub fn load(path: &Path) -> Result<Self, Box<dyn Error>> {
        let data = fs::read(path)?;
        if data.len() > 8192 {
            return Err("session config too large".into());
        }
        let config: Self = serde_json::from_slice(&data)?;
        if config.dual_crop && !config.pinch_3d {
            return Err("dual crop requires the explicit3D pinch metric".into());
        }
        if config
            .pinch_onset_max_screen_ratio
            .is_some_and(|r| !r.is_finite() || r <= 0.0 || r > 2.0)
        {
            return Err("pinch onset screen ratio must be finite and0 < ratio <= 2".into());
        }
        if !(0.8..=2.0).contains(&config.pair_separation_widths) {
            return Err("pair separation must be finite and0.8..2.0 palm widths".into());
        }
        config.touch_workspace.validate()?;
        if let Some(output) = config.pointer_output {
            crate::calibration::Profile {
                version: 1,
                workspace: Default::default(),
                output,
            }
            .validate()?;
        }
        if !(50..=1000).contains(&config.freshness_ms) {
            return Err("freshness_ms must be 50..1000".into());
        }
        Engine::new(config.gestures())?;
        if config.pinch_open_ratio > 1.0 {
            return Err("pinch ratios must satisfy 0 < close < open <= 1".into());
        }
        if !(10..=20).contains(&config.fps)
            || !config.gain.is_finite()
            || !(1.0..=10000.0).contains(&config.gain)
        {
            return Err("fps 10..20; gain 1..10000".into());
        }
        if config
            .recording_directory
            .as_ref()
            .is_some_and(|p| !Path::new(p).is_absolute())
        {
            return Err("recording_directory must be absolute".into());
        }
        for p in [&config.camera, &config.models, &config.runtime] {
            if !Path::new(p).is_absolute() {
                return Err("receiving paths must be absolute".into());
            }
        }
        Ok(config)
    }
}
// Preview preparation and measured trial share a hard 120-second capture bound.
struct PreviewClock {
    opened: Instant,
    duration: Duration,
    started: Option<Instant>,
    maximum: Duration,
}
impl PreviewClock {
    fn new(opened: Instant, seconds: u64) -> Self {
        Self {
            opened,
            duration: Duration::from_secs(seconds),
            started: None,
            maximum: Duration::from_secs(120),
        }
    }
    fn deadline(&self) -> Instant {
        self.started
            .map(|started| started + self.duration)
            .unwrap_or(self.opened + Duration::from_secs(30))
            .min(self.opened + self.maximum)
    }
    fn update(&mut self, ready: bool, now: Instant) -> bool {
        if self.started.is_none() && ready && now < self.deadline() {
            self.started = Some(now);
            true
        } else {
            false
        }
    }
    fn remaining_ms(&self, now: Instant) -> Option<u64> {
        self.started
            .map(|_| self.deadline().saturating_duration_since(now).as_millis() as u64)
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
fn camera(
    config: &SessionConfig,
    stop: Arc<AtomicBool>,
    mailbox: Arc<Mutex<Mailbox>>,
    mut recording: Option<crate::recording::Frames>,
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
                if let Some(recording) = &mut recording {
                    recording.append(&frame.bytes, frame.source_us, frame.sequence)?;
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
#[derive(Serialize, Default)]
struct Summary {
    captured: u64,
    processed: u64,
    tracked: u64,
    detected: u64,
    button_down_intents: u64,
    gesture_state_frames: std::collections::BTreeMap<String, u64>,
    gesture_block_frames: std::collections::BTreeMap<String, u64>,
    invalid_hand_fields: std::collections::BTreeMap<String, u64>,
    dropped: u64,
    stale: u64,
    elapsed_seconds: f64,
    source_kind: Option<String>,
    timestamp_to_intent_p50_ms: Option<f64>,
    timestamp_to_intent_p95_ms: Option<f64>,
    injected: bool,
    touch_mode: bool,
    controller_modes: bool,
    ambiguous_frames: u64,
    max_hands: usize,
    fresh_processed: u64,
    fresh_processed_fps: f64,
    resources: Option<crate::metrics::Resources>,
    preview_test_started: bool,
    preview_test_press_intents: Option<u64>,
    preview_test_elapsed_seconds: Option<f64>,
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
#[derive(Default)]
struct Options<'a> {
    seconds: Option<u64>,
    inject: bool,
    touch: bool,
    preview_port: Option<u16>,
    profile_path: Option<&'a Path>,
    guided: bool,
    controller_path: Option<&'a Path>,
}
pub fn run(path: &Path, seconds: u64, inject: bool) -> Result<(), Box<dyn Error>> {
    run_mode(
        path,
        Options {
            seconds: Some(seconds),
            inject,
            ..Default::default()
        },
    )
}
pub fn run_touch(path: &Path, seconds: u64, inject: bool) -> Result<(), Box<dyn Error>> {
    run_mode(
        path,
        Options {
            seconds: Some(seconds),
            inject,
            touch: true,
            ..Default::default()
        },
    )
}
pub fn preview(path: &Path, seconds: u64, port: u16) -> Result<(), Box<dyn Error>> {
    run_mode(
        path,
        Options {
            seconds: Some(seconds),
            preview_port: Some(port),
            ..Default::default()
        },
    )
}
pub fn feedback(path: &Path, seconds: u64, port: u16) -> Result<(), Box<dyn Error>> {
    run_mode(
        path,
        Options {
            seconds: Some(seconds),
            inject: true,
            preview_port: Some(port),
            ..Default::default()
        },
    )
}
pub fn calibrated(
    path: &Path,
    seconds: u64,
    port: u16,
    profile: &Path,
) -> Result<(), Box<dyn Error>> {
    run_mode(
        path,
        Options {
            seconds: Some(seconds),
            inject: true,
            preview_port: Some(port),
            profile_path: Some(profile),
            ..Default::default()
        },
    )
}
pub fn calibrated_until_stop(path: &Path, port: u16, profile: &Path) -> Result<(), Box<dyn Error>> {
    run_mode(
        path,
        Options {
            inject: true,
            preview_port: Some(port),
            profile_path: Some(profile),
            ..Default::default()
        },
    )
}
pub fn guided(path: &Path, port: u16, profile: &Path) -> Result<(), Box<dyn Error>> {
    run_mode(
        path,
        Options {
            seconds: Some(120),
            preview_port: Some(port),
            profile_path: Some(profile),
            guided: true,
            ..Default::default()
        },
    )
}
pub fn controller(
    path: &Path,
    settings: &Path,
    seconds: Option<u64>,
    port: u16,
    inject: bool,
) -> Result<(), Box<dyn Error>> {
    run_mode(
        path,
        Options {
            seconds,
            inject,
            preview_port: Some(port),
            controller_path: Some(settings),
            ..Default::default()
        },
    )
}
/// Shared pre-acquisition validation; incompatible gesture authorities are refused.
pub fn controller_config(
    config: &SessionConfig,
    path: &Path,
) -> Result<crate::controller::Controller, Box<dyn Error>> {
    let settings = crate::controller::Settings::load(path)?;
    if config.prayer_enabled {
        return Err("prayer requires its separate calibrated workflow".into());
    }
    if serde_json::to_value(settings.gestures)? != serde_json::to_value(config.gestures())? {
        return Err("controller gesture settings must match the session".into());
    }
    if serde_json::to_value(settings.workspace)? != serde_json::to_value(config.touch_workspace)? {
        return Err("controller workspace settings must match the session".into());
    }
    Ok(crate::controller::Controller::new(settings)?)
}
fn write_controller(
    output: &mut Option<std::process::ChildStdin>,
    publisher: &mut crate::delivery::Publisher,
    frame: &crate::controller::Output,
    source: Option<u64>,
) -> io::Result<u64> {
    let packet = publisher.packet(frame, monotonic_us()?, source)?;
    let presses = if matches!(packet, Some(crate::delivery::Packet::Frame { .. })) {
        frame
            .mouse
            .iter()
            .filter(|e| matches!(e, Event::Button { down: true }))
            .count() as u64
    } else {
        0
    };
    if let (Some(out), Some(packet)) = (output, packet) {
        serde_json::to_writer(&mut *out, &packet)?;
        writeln!(out)?;
        out.flush()?;
    }
    Ok(presses)
}
fn write_absolute(
    output: &mut Option<std::process::ChildStdin>,
    target: crate::Point,
) -> io::Result<()> {
    if let Some(out) = output {
        serde_json::to_writer(
            &mut *out,
            &serde_json::json!({"type":"absolute","x":target.x,"y":target.y}),
        )?;
        writeln!(out)?;
        out.flush()?;
    }
    Ok(())
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
fn run_mode(path: &Path, options: Options<'_>) -> Result<(), Box<dyn Error>> {
    let Options {
        seconds,
        inject,
        touch,
        preview_port,
        profile_path,
        guided,
        controller_path,
    } = options;
    let until_stop = seconds.is_none();
    let seconds = seconds.unwrap_or(0);
    if !until_stop && !(1..=120).contains(&seconds) {
        return Err("session duration must be 1..120 seconds".into());
    }
    if preview_port.is_some() && seconds > 90 && !guided {
        return Err(
            "preview test duration must be 1..90 seconds (up to 30 seconds preparation)".into(),
        );
    }
    let meter = crate::metrics::Meter::start();
    let config = SessionConfig::load(path)?;
    if controller_path.is_some() && (touch || profile_path.is_some() || guided) {
        return Err("controller and legacy calibration/touch workflows are separate".into());
    }
    let mut controller = controller_path
        .map(|settings| controller_config(&config, settings))
        .transpose()?;

    let mut calibration = profile_path
        .map(|profile| {
            crate::calibration_runtime::Runtime::new(
                profile,
                config.pointer_output.ok_or_else(|| {
                    io::Error::other("calibrated mode requires pointer_output dimensions")
                })?,
            )
        })
        .transpose()?;
    if config.prayer_enabled && calibration.is_none() {
        return Err("prayer entry requires run-calibrated mode".into());
    }
    if guided {
        if inject || touch || config.prayer_enabled {
            return Err("guided calibration requires camera-only single-hand mode".into());
        }
        calibration
            .as_mut()
            .ok_or("guided mode requires a profile")?
            .control(crate::calibration::Control::StartDiagonal)
            .map_err(io::Error::other)?;
    }
    let mut guide = guided.then(crate::guided_calibration::Guide::default);
    ort::init_from(&config.runtime).commit()?;
    let mut perception = config.perception()?;
    let mut visual_classifier = controller
        .as_ref()
        .map(crate::pose_classifier::VisualClassifier::configured)
        .transpose()?
        .flatten();
    let stop = Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(signal_hook::consts::SIGINT, stop.clone())?;
    signal_hook::flag::register(signal_hook::consts::SIGTERM, stop.clone())?;
    let preview = preview_port
        .map(|port| crate::preview::Preview::start(port, stop.clone()))
        .transpose()?;
    if calibration.is_some() {
        if let Some(preview) = &preview {
            if guided {
                preview.enable_guided();
            } else {
                preview.enable_calibration_controls();
            }
        }
    }
    // Start the session deadline before spawning its independently finite writer.
    let start = Instant::now();
    let mut preview_clock = preview
        .as_ref()
        .filter(|_| !until_stop)
        .map(|_| PreviewClock::new(start, seconds));
    if guided {
        preview_clock.as_mut().unwrap().maximum = Duration::from_secs(150);
    }
    let mut preview_press_baseline = 0;
    let writer_seconds = if preview.is_some() {
        seconds + 30
    } else {
        seconds
    };
    let recording = config.recording_directory.as_ref().map(|base|crate::recording::create(Path::new(base),monotonic_us()?,&serde_json::json!({"schema":"wigglecontrol.recording/v1","width":640,"height":480,"camera_fps":30,"processing_fps":config.fps,"freshness_ms":config.freshness_ms,"pinch_onset_max_screen_ratio":config.pinch_onset_max_screen_ratio,"pinch_3d":config.pinch_3d,"pinch_close_m":config.pinch_close_m,"pinch_open_m":config.pinch_open_m,"pinch_close_ratio":config.pinch_close_ratio,"pinch_open_ratio":config.pinch_open_ratio,"pair_separation_widths":config.pair_separation_widths,"palm_deduplication":config.palm_deduplication,"dual_crop":config.dual_crop,"input_enabled":inject,"human_labels":"separate; prompts and predictions are not ground truth"}))).transpose()?;
    let (recording_frames, mut recording_observations) =
        if let Some((frames, observations, dir)) = recording {
            fs::write(
                Path::new(config.recording_directory.as_ref().unwrap()).join("last-recording.path"),
                dir.as_os_str().as_encoded_bytes(),
            )?;
            (Some(frames), Some(observations))
        } else {
            (None, None)
        };
    if let Some(base) = &config.recording_directory {
        let marker = fs::read_to_string(Path::new(base).join("last-recording.path"))?;
        crate::recording::private_file(&Path::new(marker.trim()).join("config.json"))?
            .write_all(&fs::read(path)?)?;
    }
    let mut writer = if inject {
        Some(
            Command::new(std::env::current_exe()?)
                .args(if controller.is_some() {
                    if until_stop {
                        vec![
                            "controller-stream-until-stop".into(),
                            config.gain.to_string(),
                            config.freshness_ms.to_string(),
                        ]
                    } else {
                        vec![
                            "controller-stream".into(),
                            writer_seconds.to_string(),
                            config.gain.to_string(),
                            config.freshness_ms.to_string(),
                        ]
                    }
                } else if until_stop {
                    vec![
                        "absolute-stream-until-stop".into(),
                        config.freshness_ms.to_string(),
                    ]
                } else if touch {
                    vec![
                        "touch-stream".into(),
                        writer_seconds.to_string(),
                        config.freshness_ms.to_string(),
                    ]
                } else if calibration.is_some() {
                    vec![
                        "absolute-stream".into(),
                        writer_seconds.to_string(),
                        config.freshness_ms.to_string(),
                    ]
                } else {
                    vec![
                        "device-stream".into(),
                        writer_seconds.to_string(),
                        config.gain.to_string(),
                        config.freshness_ms.to_string(),
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
    let mut publisher = crate::delivery::Publisher::default();
    if let Some(controller) = &mut controller {
        let initial = controller.start_at(monotonic_us()?);
        if until_stop || preview.is_none() {
            write_controller(&mut output, &mut publisher, &initial, None)?;
        }
        if let Some(preview) = &preview {
            preview.enable_controller_controls();
        }
        if let (Some(settings), Some(base)) = (controller_path, &config.recording_directory) {
            let marker = fs::read_to_string(Path::new(base).join("last-recording.path"))?;
            crate::recording::private_file(
                &Path::new(marker.trim()).join("controller-settings.json"),
            )?
            .write_all(&fs::read(settings)?)?;
        }
    }
    let mailbox = Arc::new(Mutex::new(Mailbox::default()));
    let camera = camera(&config, stop.clone(), mailbox.clone(), recording_frames);
    let period = Duration::from_secs_f64(1.0 / config.fps as f64);
    let mut due = Instant::now();
    let mut engine = Engine::new(config.gestures())?;
    engine.start();
    let mut touches =
        crate::touch_gestures::Gestures::new(config.gestures(), config.touch_workspace)?;
    touches.start();
    let mut summary = Summary {
        injected: inject,
        touch_mode: touch,
        controller_modes: controller.is_some(),
        ..Default::default()
    };
    let mut latency = Vec::with_capacity(2400);
    let mut prayer = crate::prayer::Detector::default();
    let mut primary_hand = None;
    let result = (|| -> Result<(), Box<dyn Error>> {
        while (until_stop
            || Instant::now()
                < preview_clock
                    .as_ref()
                    .map(PreviewClock::deadline)
                    .unwrap_or(start + Duration::from_secs(seconds)))
            && !stop.load(Ordering::Relaxed)
        {
            if let (Some(preview), Some(clock)) = (&preview, &mut preview_clock) {
                if clock.update(
                    preview.view_ready() && (!guided || preview.guided_ready(0)),
                    Instant::now(),
                ) {
                    engine.stop();
                    engine.start();
                    if let Some(controller) = controller.as_mut().filter(|c| c.is_running()) {
                        write_controller(
                            &mut output,
                            &mut publisher,
                            &controller.start_at(monotonic_us()?),
                            None,
                        )?;
                    }
                    preview_press_baseline = summary.button_down_intents;
                }
            }
            if let (Some(preview), Some(calibration)) = (&preview, &mut calibration) {
                if let Some(action) = preview.take_control() {
                    write_events(&mut output, engine.stop(), false)?;
                    engine.start();
                    let _ = calibration.control(action);
                }
            }
            if let (Some(preview), Some(controller)) = (&preview, &mut controller) {
                if let Some(control) = preview.take_controller_control() {
                    let control_time = monotonic_us()?;
                    let command = serde_json::to_value(&control)?;
                    let frame = match control {
                        crate::preview::ControllerControl::Mode(mode) => {
                            controller.set_mode_at(mode, control_time)
                        }
                        crate::preview::ControllerControl::SelectHand(id) => {
                            controller.select_hand_at(id, control_time)
                        }
                        crate::preview::ControllerControl::Pause => controller.stop(),
                        crate::preview::ControllerControl::Resume => {
                            controller.start_at(control_time)
                        }
                    };
                    if let Some(file) = &mut recording_observations {
                        serde_json::to_writer(
                            &mut *file,
                            &serde_json::json!({"schema":"wigglecontrol.controller-control/v1","source":"preview_control","issued_us":control_time,"command":command,"controller":frame}),
                        )?;
                        writeln!(file)?;
                    }
                    if until_stop || preview_clock.as_ref().is_some_and(|c| c.started.is_some()) {
                        write_controller(&mut output, &mut publisher, &frame, None)?;
                    }
                }
            }
            if let Some(writer) = writer.as_mut() {
                if let Some(status) = writer.try_wait()? {
                    if status.success()
                        && Instant::now()
                            >= preview_clock
                                .as_ref()
                                .map(PreviewClock::deadline)
                                .unwrap_or(start + Duration::from_secs(seconds))
                    {
                        break;
                    }
                    return Err("device writer exited".into());
                }
            }
            if let Some(controller) = &mut controller {
                let frame = controller.tick(monotonic_us()?);
                if until_stop || preview_clock.as_ref().is_some_and(|c| c.started.is_some()) {
                    write_controller(&mut output, &mut publisher, &frame, None)?;
                }
            } else if touch {
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
            let decode_started = Instant::now();
            let mut reader =
                image::ImageReader::new(Cursor::new(&frame.bytes)).with_guessed_format()?;
            let mut limits = image::Limits::default();
            limits.max_image_width = Some(640);
            limits.max_image_height = Some(480);
            limits.max_alloc = Some(8 * 1024 * 1024);
            reader.limits(limits);
            let image = reader.decode()?.to_rgb8();
            if image.width() != 640 || image.height() != 480 {
                return Err("decoded camera dimensions changed".into());
            }
            let decode_ms = decode_started.elapsed().as_secs_f64() * 1000.0;
            let now;
            if let Some(controller) = &mut controller {
                let inference_started = Instant::now();
                let detected = perception.infer_many(&image)?;
                let inference_ms = inference_started.elapsed().as_secs_f64() * 1000.0;
                if !until_stop
                    && Instant::now()
                        >= preview_clock
                            .as_ref()
                            .map(PreviewClock::deadline)
                            .unwrap_or(start + Duration::from_secs(seconds))
                {
                    break;
                }
                summary.processed += 1;
                summary.detected += detected.hands.len() as u64;
                summary.max_hands = summary.max_hands.max(detected.hands.len());
                summary.tracked += detected.hands.iter().filter(|h| h.tracked).count() as u64;
                summary.ambiguous_frames += u64::from(detected.ambiguous);
                let hands = detected
                    .hands
                    .iter()
                    .filter_map(|h| {
                        config
                            .observation(Some(h), image.width(), image.height(), frame.source_us)
                            .hand
                    })
                    .collect();
                let observation = crate::touch_gestures::Frame {
                    version: 1,
                    source_us: frame.source_us,
                    ambiguous: detected.ambiguous,
                    hands,
                };
                let classifier_started = Instant::now();
                let (pose, visual_state) = if let Some(classifier) = &mut visual_classifier {
                    classifier.classify(
                        controller,
                        &observation,
                        &detected.hands,
                        image.width(),
                        image.height(),
                    )?
                } else {
                    (None, None)
                };
                let classifier_ms = classifier_started.elapsed().as_secs_f64() * 1000.0;
                now = monotonic_us()?;
                let state = controller.observe(now, observation, pose);
                let prepared =
                    until_stop || preview_clock.as_ref().is_some_and(|c| c.started.is_some());
                if prepared {
                    summary.button_down_intents += write_controller(
                        &mut output,
                        &mut publisher,
                        &state,
                        Some(frame.source_us),
                    )?;
                }
                let primary = state
                    .primary_hand
                    .and_then(|id| detected.hands.iter().find(|h| h.id == id))
                    .or_else(|| detected.hands.first());
                let interpreted = primary.and_then(|h| {
                    config
                        .observation(Some(h), image.width(), image.height(), frame.source_us)
                        .hand
                });
                let pinch_state = if matches!(
                    state.mode,
                    crate::controller::Mode::OneContact | crate::controller::Mode::TwoContacts
                ) {
                    if state.contacts.is_empty() {
                        "touch"
                    } else {
                        "pressed"
                    }
                } else {
                    state.pointer_state
                };
                let mut data = serde_json::json!({"active":true,"input_enabled":inject && prepared && state.running,
                    "input_available":inject, "width":image.width(),"height":image.height(),"sequence":frame.sequence,
                    "age_ms":now.saturating_sub(frame.source_us) as f64/1000.0, "fresh":frame.source_us<=now && now-frame.source_us<config.gestures().stale_us,
                    "freshness_ms":config.freshness_ms, "decode_ms":decode_ms,"inference_ms":inference_ms,
                    "landmarks":primary.map(|h|h.landmarks.iter().map(|p|[p.x,p.y]).collect::<Vec<_>>()),
                    "cursor":state.pointer_cursor,"confidence":primary.map(|h|h.confidence),
                    "eligible":interpreted.as_ref().is_some_and(|h|h.engaged && h.pinch_eligible),
                    "pinch_ratio":interpreted.as_ref().map(|h|h.thumb_tip.distance(h.index_tip)/h.palm_width),
                    "pinch_state":pinch_state,"presses":summary.button_down_intents,
                    "confirmation_elapsed_ms":state.confirmation_elapsed_us/1000,"confirmation_required_ms":config.gestures().confirm_us/1000,
                    "close_ratio":config.pinch_close_ratio,"open_ratio":config.pinch_open_ratio,"blocked_reason":state.reason,
                    "test_phase":if until_stop {"manual"} else if prepared {"testing"} else {"preparing"},
                    "recording":config.recording_directory.is_some(), "source_us":frame.source_us,
                    "frame_age_ms":now.saturating_sub(frame.source_us) as f64/1000.0,
                    "controller":state, "visual_macros":visual_classifier.is_some(),
                    "visual_pose":visual_state,"classifier_ms":classifier_ms, "pointer_model":"relative",
                    "perception_ambiguity_reason":detected.ambiguity_reason,
                    "pair_separation_widths":config.pair_separation_widths,"palm_deduplication":config.palm_deduplication,
                    "perception":detected.hands.iter().map(|h|serde_json::json!({"id":h.id,"confidence":h.confidence,"landmarks":h.landmarks.iter().map(|p|[p.x,p.y]).collect::<Vec<_>>(),"world_landmarks":h.world_landmarks})).collect::<Vec<_>>(),
                    "detected":!detected.hands.is_empty(), "gesture_state":if state.button_down {"pressed"}else if state.pointer_ready {"open"}else {"open_first"},
                    "press_intents":summary.button_down_intents,"remaining_ms":preview_clock.as_ref().and_then(|c|c.remaining_ms(Instant::now())),"until_stop":until_stop});
                if config.dual_crop {
                    data["dual_crop"] = true.into();
                    data["secondary_pinch"] = interpreted
                        .as_ref()
                        .and_then(|h| h.secondary_pinch.as_ref())
                        .map(serde_json::to_value)
                        .transpose()?
                        .into();
                    data["secondary_perception"] = detected
                        .hands
                        .iter()
                        .map(|h| serde_json::json!({"hand_id":h.id,"evidence":h.secondary_debug()}))
                        .collect::<Vec<_>>()
                        .into();
                }
                if let Some(file) = &mut recording_observations {
                    serde_json::to_writer(&mut *file, &data)?;
                    writeln!(file)?;
                }
                if let Some(preview) = &preview {
                    preview.publish(frame.bytes, data)?;
                }
            } else if touch {
                let detected = perception.infer_many(&image)?;
                if !until_stop
                    && Instant::now()
                        >= preview_clock
                            .as_ref()
                            .map(PreviewClock::deadline)
                            .unwrap_or(start + Duration::from_secs(seconds))
                {
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
                        config
                            .observation(Some(&h), image.width(), image.height(), frame.source_us)
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
                let inference_started = Instant::now();
                let hand = if config.prayer_enabled {
                    let pair = perception.infer_many(&image)?;
                    summary.max_hands = summary.max_hands.max(pair.hands.len());
                    summary.ambiguous_frames += u64::from(pair.ambiguous);
                    let hands: Vec<_> = pair.hands.iter().map(crate::prayer::Hand::from).collect();
                    let request = prayer.observe(
                        true,
                        pair.ambiguous,
                        &hands,
                        frame.source_us,
                        monotonic_us()?,
                    );
                    if request
                        && preview_clock.as_ref().is_some_and(|clock| {
                            clock.started.is_some() && Instant::now() < clock.deadline()
                        })
                        && calibration
                            .as_ref()
                            .is_some_and(|c| c.gesture_entry_available())
                    {
                        write_events(&mut output, engine.stop(), false)?;
                        engine.start();
                        if let Some(calibration) = &mut calibration {
                            calibration.prayer_requested().map_err(io::Error::other)?;
                        }
                    }
                    let selected = primary_hand
                        .and_then(|id| pair.hands.iter().find(|h| h.id == id))
                        .or_else(|| pair.hands.first())
                        .cloned();
                    if let Some(hand) = &selected {
                        primary_hand = Some(hand.id);
                    }
                    selected
                } else {
                    perception.infer(&image)?
                };
                let inference_ms = inference_started.elapsed().as_secs_f64() * 1000.0;
                if !until_stop
                    && Instant::now()
                        >= preview_clock
                            .as_ref()
                            .map(PreviewClock::deadline)
                            .unwrap_or(start + Duration::from_secs(seconds))
                {
                    break;
                }
                summary.processed += 1;
                if let Some(hand) = &hand {
                    summary.detected += 1;
                    summary.max_hands = summary.max_hands.max(1);
                    if hand.tracked {
                        summary.tracked += 1;
                    }
                }
                let observation = config.observation(
                    hand.as_ref(),
                    image.width(),
                    image.height(),
                    frame.source_us,
                );
                now = monotonic_us()?;
                let interpreted = observation.hand.clone();
                if let Some(reason) = interpreted.as_ref().and_then(|h| h.invalid_reason()) {
                    *summary
                        .invalid_hand_fields
                        .entry(reason.into())
                        .or_default() += 1;
                }
                let events = engine.observe(now, observation);
                *summary
                    .gesture_state_frames
                    .entry(engine.pinch_state().into())
                    .or_default() += 1;
                if let Some(reason) = engine.pinch_block_reason() {
                    *summary
                        .gesture_block_frames
                        .entry(reason.into())
                        .or_default() += 1;
                }
                if let (Some(guide), Some(preview)) = (&mut guide, &preview) {
                    if preview.guided_ready(guide.step()) {
                        guide.acknowledge(now);
                    }
                }
                let collecting = guide.as_ref().is_none_or(|g| g.collecting(now));
                let (target, events) = if let Some(calibration) = &mut calibration {
                    calibration.frame_with_freshness(
                        interpreted
                            .as_ref()
                            .filter(|h| {
                                h.confidence >= config.gestures().min_confidence
                                    && hand.as_ref().is_some_and(|detected| {
                                        detected.in_view(image.width(), image.height())
                                    })
                            })
                            .filter(|_| collecting)
                            .map(|h| h.cursor),
                        engine.cursor_position(),
                        (frame.source_us, now),
                        events,
                        config.gestures().stale_us,
                    )
                } else {
                    (None, events)
                };
                if let (Some(guide), Some(calibration)) = (&mut guide, &mut calibration) {
                    guide.update(calibration, now);
                }
                let output_enabled = preview_clock.as_ref().is_none_or(|c| c.started.is_some());
                summary.button_down_intents += if output_enabled {
                    if let Some(target) = target {
                        write_absolute(&mut output, target)?;
                        if let Some(calibration) = &mut calibration {
                            calibration.last_target = Some(target);
                        }
                    }
                    write_events(&mut output, events, true)?
                } else {
                    write_events(&mut output, Vec::new(), true)?;
                    write_events(&mut None, events, false)?
                };
                if let Some(preview) = &preview {
                    let ratio = interpreted
                        .as_ref()
                        .map(|h| h.thumb_tip.distance(h.index_tip) / h.palm_width);
                    let feedback_reason = if calibration
                        .as_ref()
                        .is_some_and(|c| c.is_sampling_or_reviewing())
                    {
                        Some(if now.saturating_sub(frame.source_us) >= 150_000 {
                            "Calibration: frame delayed"
                        } else if let Some(detected) = &hand {
                            if detected.confidence < config.gestures().min_confidence {
                                "Calibration: low hand confidence"
                            } else if !detected.in_view(image.width(), image.height()) {
                                "Calibration: hand partly outside camera; move inward"
                            } else {
                                "Calibration: hand visible; hold steady"
                            }
                        } else {
                            "Calibration: no hand detected"
                        })
                    } else {
                        engine.pinch_block_reason()
                    };
                    let mut data = serde_json::json!({
                        "guided":guide.as_ref().map(|g|g.state(now)),
                        "input_enabled":inject && output_enabled,
                        "prayer":{"enabled":config.prayer_enabled,"available":config.prayer_enabled && preview_clock.as_ref().is_some_and(|c|c.started.is_some()) && calibration.as_ref().is_some_and(|c|c.gesture_entry_available()),"state":prayer.state()},
                        "calibration":calibration.as_ref().map(|c|c.state()),
                        "screen_target":calibration.as_ref().and_then(|c|c.last_target).map(|p|[p.x,p.y]),
                        "source_us":frame.source_us,"consumer_us":now,"recording":config.recording_directory.is_some(),
                        "world_landmarks":hand.as_ref().map(|h|h.world_landmarks),
                        "world_pinch_distance_m":hand.as_ref().map(|h|h.world_pinch_distance_m()),
                        "sequence": frame.sequence, "width":image.width(), "height":image.height(),
                        "landmarks":hand.as_ref().map(|h|h.landmarks.iter().map(|p|[p.x,p.y]).collect::<Vec<_>>()),
                        "confidence":hand.as_ref().map(|h|h.confidence),
                        "handedness_score":hand.as_ref().map(|h|h.handedness),
                        "hand_id":hand.as_ref().map(|h|h.id),
                        "invalid_hand_field":interpreted.as_ref().and_then(|h|h.invalid_reason()),
                        "hand_in_view":hand.as_ref().is_some_and(|h|h.in_view(image.width(),image.height())),
                        "eligible":interpreted.as_ref().is_some_and(|h|h.engaged && h.pinch_eligible),
                        "cursor":interpreted.as_ref().map(|h|h.cursor), "pinch_ratio":ratio,
                        "pinch_distance_m":interpreted.as_ref().and_then(|h|h.pinch_distance_m),
                        "pinch_3d":config.pinch_3d,"pinch_close_m":config.pinch_close_m,"pinch_open_m":config.pinch_open_m,
                        "pinch_state":engine.pinch_state(), "close_ratio":config.pinch_close_ratio,
                        "blocked_reason":feedback_reason,
                        "confirmation_elapsed_ms":engine.confirmation_elapsed_us()/1000,
                        "confirmation_required_ms":config.gestures().confirm_us/1000,
                        "open_ratio":config.pinch_open_ratio,
                        "freshness_ms":config.freshness_ms,
                        "fresh":now.saturating_sub(frame.source_us)<config.gestures().stale_us,
                        "age_ms":now.saturating_sub(frame.source_us) as f64/1000.0,
                        "decode_ms":decode_ms,"inference_ms":inference_ms,
                        "presses":if until_stop || preview_clock.as_ref().is_some_and(|c|c.started.is_some()) {summary.button_down_intents.saturating_sub(preview_press_baseline)} else {0},
                        "test_phase":if until_stop || preview_clock.as_ref().is_some_and(|c|c.started.is_some()) {"testing"} else {"preparing"},
                        "test_duration_seconds":seconds,
                        "remaining_ms":if guide.as_ref().is_some_and(|g|g.finished()) {Some(0)} else {preview_clock.as_ref().and_then(|c|c.remaining_ms(Instant::now()))}
                    });
                    data["until_stop"] = until_stop.into();
                    data["pinch_onset_max_screen_ratio"] =
                        config.pinch_onset_max_screen_ratio.into();
                    data["pinch_onset_allowed"] =
                        interpreted.as_ref().map(|h| h.pinch_onset_allowed).into();
                    if config.dual_crop {
                        data["dual_crop"] = true.into();
                        data["secondary_pinch"] = interpreted
                            .as_ref()
                            .and_then(|h| h.secondary_pinch.as_ref())
                            .map(serde_json::to_value)
                            .transpose()?
                            .into();
                        data["secondary_perception"] =
                            hand.as_ref().and_then(|h| h.secondary_debug()).into();
                    }
                    if let Some(file) = &mut recording_observations {
                        serde_json::to_writer(&mut *file, &data)?;
                        writeln!(file)?;
                    }
                    preview.publish(frame.bytes, data)?;
                }
                if guide.as_ref().is_some_and(|g| g.finished()) {
                    // Give the metadata-only UI a bounded opportunity to paint
                    // terminal evidence before closing acquisition/transport.
                    thread::sleep(Duration::from_millis(200));
                    stop.store(true, Ordering::Relaxed);
                }
            }
            let age = now.saturating_sub(frame.source_us);
            if age >= config.gestures().stale_us {
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
    let release = if let Some(controller) = &mut controller {
        write_controller(&mut output, &mut publisher, &controller.stop(), None).map(|_| ())
    } else if touch {
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
    let release = if Instant::now()
        >= preview_clock
            .as_ref()
            .map(PreviewClock::deadline)
            .unwrap_or(start + Duration::from_secs(seconds))
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
    if let Some(clock) = &preview_clock {
        summary.preview_test_started = clock.started.is_some();
        summary.preview_test_press_intents = clock.started.map(|_| {
            summary
                .button_down_intents
                .saturating_sub(preview_press_baseline)
        });
        summary.preview_test_elapsed_seconds =
            clock.started.map(|began| began.elapsed().as_secs_f64());
    }
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

    fn fictional_hand(id: u64, pixel_closed: bool, world_gap: f64) -> crate::vision::DetectedHand {
        use crate::vision::{DetectedHand, XY};
        let mut hand = DetectedHand {
            id,
            landmarks: [XY { x: 200.0, y: 300.0 }; 21],
            image_depths: [0.0; 21],
            world_landmarks: [[0.0; 3]; 21],
            confidence: 0.99,
            handedness: 0.9,
            tracked: true,
            secondary: None,
        };
        hand.landmarks[0] = XY { x: 200.0, y: 400.0 };
        hand.landmarks[5] = XY { x: 180.0, y: 340.0 };
        hand.landmarks[17] = XY { x: 240.0, y: 340.0 };
        hand.landmarks[6] = XY { x: 180.0, y: 280.0 };
        hand.landmarks[8] = XY { x: 180.0, y: 180.0 };
        hand.landmarks[4] = XY {
            x: if pixel_closed { 186.0 } else { 300.0 },
            y: 180.0,
        };
        hand.world_landmarks[8][2] = world_gap;
        hand
    }

    #[test]
    fn configured_onset_veto_keeps_raw_confirmation_and_rejects_missing_pixel_geometry() {
        use crate::vision::XY;
        let config: SessionConfig = serde_json::from_value(serde_json::json!({
            "camera":"/fictional/camera", "models":"/fictional/models",
            "runtime":"/fictional/runtime", "fps":10, "gain":2000,
            "pinch_3d":true, "pinch_close_m":0.045, "pinch_open_m":0.065,
            "pinch_onset_max_screen_ratio":0.38
        }))
        .unwrap();
        let mut e = Engine::new(config.gestures()).unwrap();
        e.start();
        for (t, pixel_closed, gap, pressed) in [
            (1, false, 0.08, false),
            (20_001, false, 0.02, false),
            (60_001, false, 0.02, false),
            (70_001, true, 0.02, true),
        ] {
            let mut hand = fictional_hand(7, pixel_closed, gap);
            hand.landmarks[0] = XY { x: 200.0, y: 400.0 };
            hand.landmarks[9] = XY { x: 200.0, y: 300.0 };
            e.observe(t, config.observation(Some(&hand), 640, 480, t));
            assert_eq!(e.pinch_state() == "pressed", pressed);
        }
        let mut degenerate = fictional_hand(7, true, 0.02);
        degenerate.landmarks[9] = degenerate.landmarks[0];
        let observation = config.observation(Some(&degenerate), 640, 480, 80_001);
        assert!(!observation.hand.as_ref().unwrap().pinch_eligible);
        assert!(e
            .observe(80_001, observation)
            .contains(&Event::Button { down: false }));
        assert_eq!(e.pinch_state(), "not_eligible");
    }

    #[test]
    fn configured_pinch_metric_drives_both_contacts_when_2d_and_3d_disagree() {
        use crate::touch_gestures::{Frame, Gestures};
        for pinch_3d in [false, true] {
            let config: SessionConfig = serde_json::from_value(serde_json::json!({
                "camera":"/fictional/camera", "models":"/fictional/models",
                "runtime":"/fictional/runtime", "fps":10, "gain":2000,
                "pinch_3d":pinch_3d, "pinch_close_m":0.045, "pinch_open_m":0.065,
            }))
            .unwrap();
            let mut touches = Gestures::new(config.gestures(), config.touch_workspace).unwrap();
            touches.start();
            let mut pointer = Engine::new(config.gestures()).unwrap();
            pointer.start();
            for (time, closed, expected) in [
                (1, false, 0),
                (20_001, true, 0),
                (60_001, true, 2),
                (80_001, false, 0),
            ] {
                // The unused metric always contradicts the configured one.
                let pixel_closed = if pinch_3d { !closed } else { closed };
                let world_closed = if pinch_3d { closed } else { !closed };
                let gap = if world_closed { 0.025 } else { 0.08 };
                let observations: Vec<_> = [7, 9]
                    .into_iter()
                    .map(|id| {
                        config
                            .observation(
                                Some(&fictional_hand(id, pixel_closed, gap)),
                                640,
                                480,
                                time,
                            )
                            .hand
                            .unwrap()
                    })
                    .collect();
                assert!(observations.iter().all(|h| h.engaged));
                pointer.observe(
                    time,
                    Observation {
                        version: 1,
                        source_us: time,
                        hand: Some(observations[0].clone()),
                    },
                );
                let contacts = touches.observe(
                    time,
                    Frame {
                        version: 1,
                        source_us: time,
                        ambiguous: false,
                        hands: observations,
                    },
                );
                assert_eq!(contacts.len(), expected, "3D={pinch_3d} at {time}");
                assert_eq!(pointer.pinch_state() == "pressed", expected == 2);
            }
            assert!(config.observation(None, 640, 480, 100_001).hand.is_none());
        }
    }
    #[test]
    fn native_secondary_keeps_its_own_metric_gate_and_exact_frame_and_is_opt_in() {
        let mut config:SessionConfig=serde_json::from_value(serde_json::json!({
            "camera":"/fictional/camera","models":"/fictional/models","runtime":"/fictional/runtime",
            "fps":10,"gain":2000,"pinch_3d":true,"dual_crop":true,"pinch_onset_max_screen_ratio":0.38
        })).unwrap();
        let mut primary = fictional_hand(7, false, 0.08);
        primary.landmarks[0] = crate::vision::XY { x: 200.0, y: 400.0 };
        primary.landmarks[9] = crate::vision::XY { x: 200.0, y: 300.0 };
        let mut secondary = fictional_hand(7, true, 0.02);
        secondary.landmarks[0] = primary.landmarks[0];
        secondary.landmarks[9] = primary.landmarks[9];
        primary.secondary = Some(Box::new(secondary));
        let h = config
            .observation(Some(&primary), 640, 480, 42)
            .hand
            .unwrap();
        assert_eq!(h.pinch_distance_m, Some(0.08));
        assert!(!h.pinch_onset_allowed);
        let s = h.secondary_pinch.unwrap();
        assert_eq!(s.source_us, 42);
        assert_eq!(s.source_id, crate::vision::PROXIMAL_PINCH_SOURCE);
        assert!((s.distance_m - 0.02).abs() < 1e-9);
        assert!(s.eligible && s.onset_allowed);
        config.dual_crop = false;
        assert!(config
            .observation(Some(&primary), 640, 480, 43)
            .hand
            .unwrap()
            .secondary_pinch
            .is_none());
    }
    #[test]
    fn overlap_policy_is_validated_without_opening_models_or_camera() {
        let path =
            std::env::temp_dir().join(format!("wigglecontrol-policy-{}.json", std::process::id()));
        let mut value = serde_json::json!({"camera":"/fictional/camera", "models":"/fictional/models",
            "runtime":"/fictional/runtime", "fps":10, "gain":2000});
        fs::write(&path, value.to_string()).unwrap();
        assert_eq!(
            SessionConfig::load(&path).unwrap().pair_separation_widths,
            0.8
        );
        assert_eq!(
            SessionConfig::load(&path).unwrap().palm_deduplication,
            crate::vision::PalmDeduplication::Roi
        );
        value["palm_deduplication"] = serde_json::json!("palm_landmarks");
        fs::write(&path, value.to_string()).unwrap();
        assert_eq!(
            SessionConfig::load(&path).unwrap().palm_deduplication,
            crate::vision::PalmDeduplication::PalmLandmarks
        );
        value["palm_deduplication"] = serde_json::json!("unqualified");
        fs::write(&path, value.to_string()).unwrap();
        assert!(SessionConfig::load(&path).is_err());
        value["palm_deduplication"] = serde_json::json!("roi");
        value["pair_separation_widths"] = serde_json::json!(1.2);
        fs::write(&path, value.to_string()).unwrap();
        assert_eq!(
            SessionConfig::load(&path).unwrap().pair_separation_widths,
            1.2
        );
        for invalid in [0.7, 2.1] {
            value["pair_separation_widths"] = serde_json::json!(invalid);
            fs::write(&path, value.to_string()).unwrap();
            assert!(SessionConfig::load(&path)
                .err()
                .unwrap()
                .to_string()
                .contains("pair separation"));
        }
        fs::remove_file(path).unwrap();
    }
    #[test]
    fn preview_clock_starts_on_visible_frame_and_has_a_finite_preparation_bound() {
        let now = Instant::now();
        let mut clock = PreviewClock::new(now, 30);
        assert_eq!(clock.remaining_ms(now + Duration::from_secs(10)), None);
        assert!(clock.update(true, now + Duration::from_secs(10)));
        assert_eq!(
            clock.remaining_ms(now + Duration::from_secs(10)),
            Some(30_000)
        );
        assert_eq!(
            clock.remaining_ms(now + Duration::from_secs(20)),
            Some(20_000)
        );
        assert!(!clock.update(true, now + Duration::from_secs(25)));
        assert_eq!(clock.remaining_ms(now + Duration::from_secs(40)), Some(0));
        let mut absent = PreviewClock::new(now, 30);
        assert_eq!(absent.deadline(), now + Duration::from_secs(30));
        assert!(!absent.update(true, now + Duration::from_secs(31)));
        let mut longest = PreviewClock::new(now, 90);
        assert!(longest.update(true, now + Duration::from_secs(29)));
        assert!(longest.deadline() <= now + Duration::from_secs(120));
    }
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
