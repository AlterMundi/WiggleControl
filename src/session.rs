//! Explicit finite camera sessions with a latest-frame slot and independent writer.
use crate::{intent::monotonic_us, vision::Perception, Engine, Event};
use serde::Serialize;
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
    device::Device,
    io::traits::CaptureStream,
    video::Capture,
};

pub use crate::session_config::{
    controller_config, PalmDeduplication, Pinch2dReference, SecondaryLandmarkModel, SessionConfig,
};
pub(crate) use crate::session_config::{observe_controller, validate_camera_resolution};
impl SessionConfig {
    pub fn perception(&self) -> Result<Perception, Box<dyn Error>> {
        self.replay_perception(5, None)
    }
    pub(crate) fn replay_perception(
        &self,
        rediscovery: u32,
        separation_override: Option<f64>,
    ) -> Result<Perception, Box<dyn Error>> {
        let model = Perception::with_replay_options(
            Path::new(&self.models),
            rediscovery,
            separation_override.unwrap_or(self.pair_separation_widths),
        )?
        .with_palm_deduplication(self.palm_deduplication)
        .with_dual_crop(self.dual_crop);
        if let Some(secondary) = &self.secondary_model {
            model.with_secondary_model(
                Path::new(&secondary.path),
                secondary.min_presence,
                secondary.source_id,
            )
        } else {
            Ok(model)
        }
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
pub(crate) use crate::camera_frame::Frame;
#[derive(Default)]
pub(crate) struct Mailbox {
    pub frame: Option<Frame>,
    pub error: Option<String>,
    pub dropped: u64,
    pub frames: u64,
    pub source_kind: Option<&'static str>,
    pub opened: bool,
}
pub(crate) fn camera(
    config: &SessionConfig,
    stop: Arc<AtomicBool>,
    mailbox: Arc<Mutex<Mailbox>>,
    mut recording: Option<crate::recording::Frames>,
    deadline: Option<Instant>,
) -> thread::JoinHandle<()> {
    let path = config.camera.clone();
    let resolution = config.camera_resolution;
    thread::spawn(move || {
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
            Event::NamedButton { button, down } => {
                if down {
                    presses += 1;
                }
                serde_json::json!({"type":"named_button","button":button,"down":down})
            }
            Event::Scroll {
                vertical,
                horizontal,
            } => {
                serde_json::json!({"type":"scroll","vertical":vertical,"horizontal":horizontal})
            }
            Event::ScrollFine {
                vertical,
                horizontal,
            } => {
                serde_json::json!({"type":"scroll_fine","vertical":vertical,"horizontal":horizontal})
            }
            Event::Cancel { .. } => serde_json::json!({"type":"release"}),
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
pub(crate) fn write_controller(
    output: &mut Option<std::process::ChildStdin>,
    publisher: &mut crate::delivery::Publisher,
    frame: &crate::controller::Output,
    source: Option<u64>,
) -> io::Result<u64> {
    write_controller_mapped(output, publisher, frame, source, None)
}
pub(crate) fn write_controller_mapped(
    output: &mut Option<std::process::ChildStdin>,
    publisher: &mut crate::delivery::Publisher,
    frame: &crate::controller::Output,
    source: Option<u64>,
    workspace: Option<crate::touch_gestures::Workspace>,
) -> io::Result<u64> {
    let now = monotonic_us()?;
    write_controller_mapped_at(output, publisher, frame, source, workspace, now)
}
pub(crate) fn write_controller_mapped_at(
    output: &mut Option<std::process::ChildStdin>,
    publisher: &mut crate::delivery::Publisher,
    frame: &crate::controller::Output,
    source: Option<u64>,
    workspace: Option<crate::touch_gestures::Workspace>,
    now: u64,
) -> io::Result<u64> {
    let packet = match workspace {
        Some(workspace) => publisher.absolute_packet(frame, now, source, workspace)?,
        None => publisher.packet(frame, now, source)?,
    };
    let presses = if matches!(packet, Some(crate::delivery::Packet::Frame { .. })) {
        frame
            .mouse
            .iter()
            .filter(|e| {
                matches!(
                    e,
                    Event::Button { down: true } | Event::NamedButton { down: true, .. }
                )
            })
            .count() as u64
    } else {
        0
    };
    if let (Some(out), Some(packet)) = (output, packet) {
        out.write_all(&crate::recording::json_line(&packet)?)?;
        out.flush()?;
    }
    Ok(presses)
}

pub(crate) fn controller_writer(
    config: &SessionConfig,
    seconds: Option<u64>,
) -> io::Result<std::process::Child> {
    let mut args = match seconds {
        Some(seconds) => vec!["controller-stream".to_owned(), seconds.to_string()],
        None => vec!["controller-stream-until-stop".to_owned()],
    };
    if config.provider_absolute_pointer {
        args[0] = if seconds.is_some() {
            "controller-absolute-stream"
        } else {
            "controller-absolute-stream-until-stop"
        }
        .into();
    }
    args.extend([config.gain.to_string(), config.freshness_ms.to_string()]);
    Command::new(std::env::current_exe()?)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .spawn()
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
    config.validate_recording((!until_stop).then_some(seconds))?;
    if config.pointer_cursor_output.is_some() {
        return Err(
            "cursor output is supported by the explicit camera-worker provider workflow".into(),
        );
    }
    if config.pointer_release_confirmation.is_some() {
        return Err(
            "release confirmation is supported by the explicit camera-worker provider workflow"
                .into(),
        );
    }
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
    let recording = config.recording_directory.as_ref().filter(|_| config.recording_enabled).map(|base|crate::recording::create(Path::new(base),monotonic_us()?,&serde_json::json!({"schema":"wigglecontrol.recording/v1","width":config.camera_resolution[0],"height":config.camera_resolution[1],"camera_fps":30,"processing_fps":config.fps,"freshness_ms":config.freshness_ms,"pinch_onset_max_screen_ratio":config.pinch_onset_max_screen_ratio,"pinch_3d":config.pinch_3d,"pinch_2d_reference":config.pinch_2d_reference,"pinch_close_m":config.pinch_close_m,"pinch_open_m":config.pinch_open_m,"pinch_close_ratio":config.pinch_close_ratio,"pinch_open_ratio":config.pinch_open_ratio,"pair_separation_widths":config.pair_separation_widths,"palm_deduplication":config.palm_deduplication,"dual_crop":config.dual_crop,"secondary_model":config.secondary_model,"gesture_min_confidence":config.gestures().min_confidence,"input_enabled":inject,"human_labels":"separate; prompts and predictions are not ground truth"}))).transpose()?;
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
    if let Some(frames) = &recording_frames {
        frames.metadata("config.json", &fs::read(path)?)?;
    }
    let mut writer = if inject && controller.is_some() {
        Some(controller_writer(
            &config,
            (!until_stop).then_some(writer_seconds),
        )?)
    } else if inject {
        Some(
            Command::new(std::env::current_exe()?)
                .args(if until_stop {
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
        if let (Some(settings), Some(frames)) = (controller_path, &recording_frames) {
            frames.metadata("controller-settings.json", &fs::read(settings)?)?;
        }
    }

    let mailbox = Arc::new(Mutex::new(Mailbox::default()));
    let capture_deadline = (!until_stop).then(|| {
        preview_clock
            .as_ref()
            .map_or(start + Duration::from_secs(seconds), |clock| {
                clock.opened + clock.maximum
            })
    });
    let camera = camera(
        &config,
        stop.clone(),
        mailbox.clone(),
        recording_frames,
        capture_deadline,
    );
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
            limits.max_image_width = Some(config.camera_resolution[0]);
            limits.max_image_height = Some(config.camera_resolution[1]);
            limits.max_alloc = Some(32 * 1024 * 1024);
            reader.limits(limits);
            let image = reader.decode()?.to_rgb8();
            if [image.width(), image.height()] != config.camera_resolution {
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
                let state = observe_controller(
                    &config,
                    controller,
                    now,
                    observation,
                    pose,
                    &detected.hands,
                    [image.width(), image.height()],
                );
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
                    "pinch_ratio":interpreted.as_ref().map(crate::Hand::pinch_ratio),
                    "pinch_state":pinch_state,"presses":summary.button_down_intents,
                    "confirmation_elapsed_ms":state.confirmation_elapsed_us/1000,"confirmation_required_ms":config.gestures().confirm_us/1000,
                    "close_ratio":config.pinch_close_ratio,"open_ratio":config.pinch_open_ratio,"blocked_reason":state.reason,
                    "test_phase":if until_stop {"manual"} else if prepared {"testing"} else {"preparing"},
                    "recording":config.recording_enabled && config.recording_directory.is_some(), "source_us":frame.source_us,
                    "frame_age_ms":now.saturating_sub(frame.source_us) as f64/1000.0,
                    "controller":state, "visual_macros":visual_classifier.is_some(),
                    "visual_pose":visual_state,"classifier_ms":classifier_ms, "pointer_model":"relative",
                    "perception_ambiguity_reason":detected.ambiguity_reason,
                    "pair_separation_widths":config.pair_separation_widths,"palm_deduplication":config.palm_deduplication,
                    "perception":detected.hands.iter().map(|h|serde_json::json!({"id":h.id,"confidence":h.confidence,"landmarks":h.landmarks.iter().map(|p|[p.x,p.y]).collect::<Vec<_>>(),"world_landmarks":h.world_landmarks})).collect::<Vec<_>>(),
                    "detected":!detected.hands.is_empty(), "gesture_state":if state.button_down {"pressed"}else if state.pointer_ready {"open"}else {"open_first"},
                    "press_intents":summary.button_down_intents,"remaining_ms":preview_clock.as_ref().and_then(|c|c.remaining_ms(Instant::now())),"until_stop":until_stop});
                data["pinch_metric"] = config.pinch_metric_name().into();
                if config.dual_crop {
                    data["dual_crop"] = true.into();
                    data["secondary_model"] = serde_json::to_value(&config.secondary_model)?;
                    data["gesture_min_confidence"] = config.gestures().min_confidence.into();
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
                let hand = if config.prayer_enabled || config.dual_crop {
                    let pair = perception.infer_many(&image)?;
                    summary.max_hands = summary.max_hands.max(pair.hands.len());
                    summary.ambiguous_frames += u64::from(pair.ambiguous);
                    let hands: Vec<_> = pair.hands.iter().map(crate::prayer::Hand::from).collect();
                    let request = prayer.observe(
                        config.prayer_enabled,
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
                if let (Some(guide), Some(calibration)) = (&mut guide, &mut calibration) {
                    guide.tracking_feedback(feedback_reason);
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
                    let ratio = interpreted.as_ref().map(crate::Hand::pinch_ratio);
                    let mut data = serde_json::json!({
                        "guided":guide.as_ref().map(|g|g.state(now)),
                        "input_enabled":inject && output_enabled,
                        "prayer":{"enabled":config.prayer_enabled,"available":config.prayer_enabled && preview_clock.as_ref().is_some_and(|c|c.started.is_some()) && calibration.as_ref().is_some_and(|c|c.gesture_entry_available()),"state":prayer.state()},
                        "calibration":calibration.as_ref().map(|c|c.state()),
                        "screen_target":calibration.as_ref().and_then(|c|c.last_target).map(|p|[p.x,p.y]),
                        "source_us":frame.source_us,"consumer_us":now,"recording":config.recording_enabled && config.recording_directory.is_some(),
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
                    data["pinch_metric"] = config.pinch_metric_name().into();
                    if config.dual_crop {
                        data["dual_crop"] = true.into();
                        data["secondary_model"] = serde_json::to_value(&config.secondary_model)?;
                        data["gesture_min_confidence"] = config.gestures().min_confidence.into();
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

pub(crate) fn check_writer_exit(
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
    use crate::landmarks::tests::fictional_hand;
    use crate::Observation;
    use std::os::unix::process::ExitStatusExt;

    #[test]
    fn direct_writer_serialization_cancels_every_named_button_without_stopping() {
        let mut child = std::process::Command::new("cat")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        let mut input = child.stdin.take();
        let presses = write_events(
            &mut input,
            vec![
                Event::Button { down: true },
                Event::NamedButton {
                    button: crate::MouseButton::Right,
                    down: true,
                },
                Event::Cancel {
                    reason: "explicit".into(),
                },
            ],
            true,
        )
        .unwrap();
        drop(input);
        let result = child.wait_with_output().unwrap();
        assert!(result.status.success());
        assert_eq!(presses, 2);
        let lines = String::from_utf8(result.stdout).unwrap();
        let mut state = crate::input::MouseState::default();
        let mut release = Vec::new();
        for line in lines.lines() {
            let intent = serde_json::from_str(line).unwrap();
            let events = state.apply(intent, 1.0).unwrap();
            release.extend(events.into_iter().filter(|e| e.value() == 0));
        }
        assert_eq!(
            release.iter().map(|e| e.code()).collect::<Vec<_>>(),
            vec![evdev::KeyCode::BTN_LEFT.0, evdev::KeyCode::BTN_RIGHT.0]
        );
        assert!(state.release().is_empty());
        assert!(lines.lines().any(|line| line == r#"{"type":"release"}"#));
        assert!(lines.lines().all(|line| line != r#"{"type":"stop"}"#));
    }

    #[test]
    fn raw_landmark_button_bridge_uses_claimed_hand_without_a_classifier_or_model() {
        let config: SessionConfig = serde_json::from_value(serde_json::json!({
            "camera":"/fictional/camera", "models":"/fictional/models",
            "runtime":"/fictional/runtime", "fps":10, "gain":2000,
            "pinch_3d":false, "pinch_2d_reference":"wrist_middle_mcp",
            "pinch_close_ratio":0.38, "pinch_open_ratio":0.40,
            "gesture_min_confidence":0.5
        }))
        .unwrap();
        let mut controller = crate::controller::Controller::new(crate::controller::Settings {
            gestures: config.gestures(),
            right_click: true,
            ..Default::default()
        })
        .unwrap();
        controller.start();
        controller.select_hand(Some(7));
        for (t, closed, right, edge) in [
            (1, false, false, None),
            (20_001, true, true, None),
            (60_001, true, true, Some((crate::MouseButton::Right, true))),
            (70_001, true, true, None),
            (
                80_001,
                false,
                false,
                Some((crate::MouseButton::Right, false)),
            ),
            (100_001, true, false, None),
            (140_001, true, false, Some((crate::MouseButton::Left, true))),
        ] {
            let mut raw = fictional_hand(7, closed && !right, 0.2);
            raw.confidence = 0.5;
            raw.landmarks[9] = crate::vision::XY { x: 200.0, y: 300.0 };
            raw.landmarks[12] = crate::vision::XY {
                x: if right {
                    raw.landmarks[4].x + 2.0
                } else {
                    400.0
                },
                y: raw.landmarks[4].y,
            };
            // A different hand appears first, with the opposite gesture.
            let mut other = raw.clone();
            other.id = 9;
            other.landmarks[12].x = if right {
                400.0
            } else {
                other.landmarks[4].x + 2.0
            };
            let observation = crate::touch_gestures::Frame {
                version: 1,
                source_us: t,
                ambiguous: false,
                hands: vec![
                    config.observation(Some(&other), 640, 480, t).hand.unwrap(),
                    config.observation(Some(&raw), 640, 480, t).hand.unwrap(),
                ],
            };
            let output = observe_controller(
                &config,
                &mut controller,
                t,
                observation,
                None,
                &[other, raw],
                [640, 480],
            );
            let edges: Vec<_> = output
                .mouse
                .iter()
                .filter_map(|e| match e {
                    Event::Button { down } => Some((crate::MouseButton::Left, *down)),
                    Event::NamedButton { button, down } => Some((*button, *down)),
                    _ => None,
                })
                .collect();
            assert_eq!(edges, edge.into_iter().collect::<Vec<_>>(), "source{t}");
        }
    }

    #[test]
    fn raw_detail_bridge_uses_selected_hand_and_positive_extended_rearm() {
        let config: SessionConfig = serde_json::from_value(serde_json::json!({
            "camera":"/fictional/camera", "models":"/fictional/models",
            "runtime":"/fictional/runtime", "fps":10,"gain":2000,
            "pinch_3d":false, "gesture_min_confidence":0.5
        }))
        .unwrap();
        let mut controller = crate::controller::Controller::new(crate::controller::Settings {
            gestures: config.gestures(),
            detail: Some(Default::default()),
            ..Default::default()
        })
        .unwrap();
        controller.start();
        controller.select_hand(Some(7));
        for (t, held, expected) in [
            (1, false, false),
            (100_001, false, false),
            (150_001, true, false),
            (250_001, true, true),
            (300_001, false, true),
            (400_001, false, false),
        ] {
            let mut raw = fictional_hand(7, false, 0.2);
            raw.confidence = 0.5;
            let straight = [
                [0.0, 0.0, 0.0],
                [0.0, 0.01, 0.0],
                [0.0, 0.02, 0.0],
                [0.0, 0.03, 0.0],
            ];
            let folded = [
                [0.0, 0.0, 0.0],
                [0.0, 0.01, 0.0],
                [0.0, 0.01, 0.01],
                [0.0, 0.0, 0.01],
            ];
            for start in [13, 17] {
                raw.world_landmarks[start..start + 4].copy_from_slice(if held {
                    &folded
                } else {
                    &straight
                });
            }
            let mut other = raw.clone();
            other.id = 9;
            for start in [13, 17] {
                other.world_landmarks[start..start + 4].copy_from_slice(if held {
                    &straight
                } else {
                    &folded
                });
            }
            let observation = crate::touch_gestures::Frame {
                version: 1,
                source_us: t,
                ambiguous: false,
                hands: vec![
                    config.observation(Some(&other), 640, 480, t).hand.unwrap(),
                    config.observation(Some(&raw), 640, 480, t).hand.unwrap(),
                ],
            };
            let out = observe_controller(
                &config,
                &mut controller,
                t,
                observation,
                None,
                &[other, raw],
                [640, 480],
            );
            assert_eq!(out.detail.is_some(), expected, "source{t}");
            assert_eq!(out.primary_hand, Some(7));
        }
    }

    #[test]
    fn raw_escape_bridge_uses_claimed_in_view_pose_not_pointer_eligibility() {
        use crate::escape::Pose;
        let config:SessionConfig=serde_json::from_value(serde_json::json!({
            "camera":"/fictional/camera","models":"/fictional/models","runtime":"/fictional/runtime",
            "fps":10,"gain":2000,"pinch_3d":false,"freshness_ms":300,"gesture_min_confidence":0.5
        })).unwrap();
        for obstruct in [false, true] {
            let mut controller = crate::controller::Controller::new(crate::controller::Settings {
                gestures: config.gestures(),
                escape: Some(Default::default()),
                ..Default::default()
            })
            .unwrap();
            controller.start();
            controller.select_hand(Some(7));
            for n in 0..=10 {
                let t = 1 + n * 100_000;
                let pose = if n <= 2 {
                    Pose::Neutral
                } else {
                    Pose::ThumbOut
                };
                let mut raw = fictional_hand(7, false, 0.2);
                raw.confidence = 0.5;
                raw.world_landmarks = crate::escape::tests::world(pose);
                if pose == Pose::ThumbOut {
                    for (tip, pip) in [(8, 6), (12, 10), (16, 14), (20, 18)] {
                        raw.landmarks[pip].y = 280.0;
                        raw.landmarks[tip].y = 350.0;
                    }
                }
                // Out-of-view evidence at the beginning of the hold must disarm;
                // reappearance still curled cannot complete the previous gesture.
                if obstruct && n == 4 {
                    raw.landmarks[0].x = -1.0;
                }
                let mut other = raw.clone();
                other.id = 9;
                other.world_landmarks = crate::escape::tests::world(Pose::Other);
                let observed = config.observation(Some(&raw), 640, 480, t).hand.unwrap();
                if pose == Pose::ThumbOut {
                    assert!(!observed.engaged && !observed.pinch_eligible);
                }
                let frame = crate::touch_gestures::Frame {
                    version: 1,
                    source_us: t,
                    ambiguous: false,
                    hands: vec![
                        config.observation(Some(&other), 640, 480, t).hand.unwrap(),
                        observed,
                    ],
                };
                let out = observe_controller(
                    &config,
                    &mut controller,
                    t,
                    frame,
                    None,
                    &[other, raw],
                    [640, 480],
                );
                assert_eq!(
                    out.escape,
                    n == 10 && !obstruct,
                    "sample{n} obstruct{obstruct}"
                );
            }
        }
    }

    #[test]
    fn projected_palm_metric_preserves_units_and_invalidates_degenerate_geometry() {
        let config: SessionConfig = serde_json::from_value(serde_json::json!({
            "camera":"/fictional/camera", "models":"/fictional/models",
            "runtime":"/fictional/runtime", "fps":10, "gain":2000,
            "pinch_3d":false, "pinch_2d_reference":"wrist_middle_mcp",
            "pinch_close_ratio":0.38, "pinch_open_ratio":0.40
        }))
        .unwrap();
        let mut hand = fictional_hand(1, true, 0.2);
        let converted = config.observation(Some(&hand), 640, 480, 1).hand.unwrap();
        assert!(converted.pinch_eligible);
        assert!(converted.pinch_distance_m.is_none());
        let expected = hand.landmarks[4].distance(hand.landmarks[8])
            / hand.landmarks[0].distance(hand.landmarks[9]);
        assert_eq!(converted.pinch_distance_ratio, Some(expected));
        assert_eq!(config.pinch_metric_name(), "projected_wrist_middle_mcp");
        hand.landmarks[9] = hand.landmarks[0];
        let invalid = config.observation(Some(&hand), 640, 480, 2).hand.unwrap();
        assert!(!invalid.pinch_eligible);
        assert!(invalid.pinch_distance_ratio.is_none());
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
        primary.secondary_source_id = crate::vision::PROXIMAL_PINCH_SOURCE;
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
        assert_eq!(config.pinch_2d_reference, Pinch2dReference::PalmWidth);
        value["pinch_2d_reference"] = serde_json::json!("wrist_middle_mcp");
        fs::write(&path, value.to_string()).unwrap();
        assert_eq!(
            SessionConfig::load(&path).unwrap().pinch_metric_name(),
            "projected_wrist_middle_mcp"
        );
        value["pinch_3d"] = true.into();
        fs::write(&path, value.to_string()).unwrap();
        assert!(SessionConfig::load(&path)
            .err()
            .unwrap()
            .to_string()
            .contains("requires pinch_3d=false"));
        value["pinch_3d"] = false.into();
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
    #[test]
    fn raw_scroll_bridge_uses_in_view_owner_and_mirrored_wrist_for_both_axes() {
        let config:SessionConfig=serde_json::from_value(serde_json::json!({
            "camera":"/fictional/camera","models":"/fictional/models","runtime":"/fictional/runtime",
            "fps":10,"gain":2000,"pinch_3d":false,"freshness_ms":300,"gesture_min_confidence":0.5
        })).unwrap();
        for obstruct in [false, true] {
            let mut controller = crate::controller::Controller::new(crate::controller::Settings {
                gestures: config.gestures(),
                scroll: Some(Default::default()),
                ..Default::default()
            })
            .unwrap();
            controller.start();
            controller.select_hand(Some(7));
            for n in 0..=7 {
                let t = 1 + n * 100_000;
                let fist = n >= 3;
                let mut raw = fictional_hand(7, false, 0.2);
                raw.confidence = 0.5;
                raw.world_landmarks = crate::escape::tests::world(if fist {
                    crate::escape::Pose::ThumbOut
                } else {
                    crate::escape::Pose::Neutral
                });
                if fist {
                    raw.world_landmarks[4] = [0.0, -0.02, 0.0];
                    for (tip, pip) in [(8, 6), (12, 10), (16, 14), (20, 18)] {
                        raw.landmarks[pip].y = 280.0;
                        raw.landmarks[tip].y = 350.0;
                    }
                }
                if n == 7 {
                    raw.landmarks[0].x += 64.0;
                    raw.landmarks[0].y -= 48.0;
                }
                if obstruct && n == 4 {
                    raw.landmarks[1].x = -1.0;
                }
                let observed = config.observation(Some(&raw), 640, 480, t).hand.unwrap();
                if fist {
                    assert!(!observed.pinch_eligible);
                }
                let mut other = raw.clone();
                other.id = 9;
                let frame = crate::touch_gestures::Frame {
                    version: 1,
                    source_us: t,
                    ambiguous: false,
                    hands: vec![
                        config.observation(Some(&other), 640, 480, t).hand.unwrap(),
                        observed,
                    ],
                };
                let out = observe_controller(
                    &config,
                    &mut controller,
                    t,
                    frame,
                    None,
                    &[other, raw],
                    [640, 480],
                );
                assert_eq!(out.primary_hand, Some(7));
                assert_eq!(out.scroll.as_ref().unwrap().active, n >= 6 && !obstruct);
                if n == 7 && !obstruct {
                    assert!(out.mouse.iter().any(|e|matches!(e,Event::ScrollFine{vertical,horizontal} if *vertical<0&&*horizontal>0)));
                } else {
                    assert!(!out
                        .mouse
                        .iter()
                        .any(|e| matches!(e, Event::ScrollFine { .. })));
                }
            }
        }
    }
}
