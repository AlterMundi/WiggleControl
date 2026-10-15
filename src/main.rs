use std::io::{self, BufRead, Write};
use wigglecontrol::{replay, Config, Engine, ReplayCommand, MAX_MESSAGE_BYTES};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args == ["--help"] || args == ["-h"] {
        print_help();
        return Ok(());
    }
    if args.first().is_some_and(|a| a == "accept-calibration") && args.len() == 2 {
        wigglecontrol::calibration_runtime::accept_saved(std::path::Path::new(&args[1]))?;
        println!("Saved corner verification accepted; no acquisition started");
        return Ok(());
    }
    #[cfg(feature = "native-vision")]
    match args.first().map(String::as_str) {
        Some("provider-fixture") if args.len() == 6 || args.len() == 7 => {
            return wigglecontrol::provider_fixture::run(
                std::path::Path::new(&args[1]),
                std::path::Path::new(&args[2]),
                std::path::Path::new(&args[3]),
                std::path::Path::new(&args[4]),
                args[5].parse()?,
                args.get(6).map(|s| s.parse()).transpose()?.unwrap_or(2),
            );
        }
        Some("provider-replay") if args.len() == 4 => {
            return wigglecontrol::provider::replay(
                std::path::Path::new(&args[1]),
                std::path::Path::new(&args[2]),
                args[3].parse()?,
            );
        }
        Some("controller-sequence") if args.len() == 4 => {
            return wigglecontrol::sequence::check_controller(
                std::path::Path::new(&args[1]),
                std::path::Path::new(&args[2]),
                std::path::Path::new(&args[3]),
            );
        }
        Some("gesture-check") if args.len() == 4 => {
            return wigglecontrol::pose_classifier::check_image(
                std::path::Path::new(&args[1]),
                std::path::Path::new(&args[2]),
                std::path::Path::new(&args[3]),
            );
        }
        Some("gesture-landmarks") if args.len() == 3 => {
            return wigglecontrol::pose_classifier::replay(
                std::path::Path::new(&args[1]),
                &args[2],
            );
        }
        Some("sequence-check") if args.len() >= 3 => {
            let mut rediscovery = None;
            let mut separation = None;
            let mut gesture_model = None;
            let mut options = args[3..].chunks_exact(2);
            for option in options.by_ref() {
                match option[0].as_str() {
                    "--rediscovery" if rediscovery.is_none() => {
                        rediscovery = Some(option[1].parse()?)
                    }
                    "--pair-separation" if separation.is_none() => {
                        separation = Some(option[1].parse()?)
                    }
                    "--gesture-model" if gesture_model.is_none() => {
                        gesture_model = Some(std::path::Path::new(&option[1]));
                    }
                    _ => return Err("unknown or duplicate sequence-check option".into()),
                }
            }
            if !options.remainder().is_empty() {
                return Err("sequence-check option requires a value".into());
            }
            return wigglecontrol::sequence::check(
                std::path::Path::new(&args[1]),
                std::path::Path::new(&args[2]),
                rediscovery.unwrap_or(5),
                separation,
                gesture_model,
            );
        }
        Some("crop-check") if args.len() == 4 || args.len() == 5 => {
            let config =
                wigglecontrol::session::SessionConfig::load(std::path::Path::new(&args[2]))?;
            let roi = if let Some(file) = args.get(4) {
                if std::fs::metadata(file)?.len() > 1024 {
                    return Err("crop-check ROI exceeds1KiB".into());
                }
                Some(serde_json::from_slice::<[f64; 4]>(&std::fs::read(file)?)?)
            } else {
                None
            };
            ort::init_from(&config.runtime).commit()?;
            let image = image::open(&args[1])?.to_rgb8();
            let report = wigglecontrol::vision::crop_check(
                std::path::Path::new(&config.models),
                &image,
                roi,
                std::path::Path::new(&args[3]),
            )?;
            println!("{}", serde_json::to_string(&report)?);
            return Ok(());
        }
        Some("position-sweep") if args.len() == 4 => {
            let config =
                wigglecontrol::session::SessionConfig::load(std::path::Path::new(&args[2]))?;
            ort::init_from(&config.runtime).commit()?;
            let image = image::open(&args[1])?.to_rgb8();
            let report = wigglecontrol::vision::position_sweep(
                std::path::Path::new(&config.models),
                &image,
                std::path::Path::new(&args[3]),
            )?;
            println!("{}", serde_json::to_string(&report)?);
            return Ok(());
        }
        Some("calibrate-guided") if args.len() == 4 => {
            return wigglecontrol::session::guided(
                std::path::Path::new(&args[1]),
                args[2].parse()?,
                std::path::Path::new(&args[3]),
            );
        }
        Some("run-calibrated-until-stop") if args.len() == 4 => {
            return wigglecontrol::session::calibrated_until_stop(
                std::path::Path::new(&args[1]),
                args[2].parse()?,
                std::path::Path::new(&args[3]),
            );
        }
        Some("run-calibrated") if args.len() == 5 => {
            return wigglecontrol::session::calibrated(
                std::path::Path::new(&args[2]),
                args[1].parse()?,
                args[3].parse()?,
                std::path::Path::new(&args[4]),
            );
        }
        Some("run-feedback") if args.len() == 4 => {
            return wigglecontrol::session::feedback(
                std::path::Path::new(&args[2]),
                args[1].parse()?,
                args[3].parse()?,
            );
        }
        Some("camera-preview") if args.len() == 4 => {
            return wigglecontrol::session::preview(
                std::path::Path::new(&args[2]),
                args[1].parse()?,
                args[3].parse()?,
            );
        }
        Some("review-recording") if args.len() == 4 => {
            return wigglecontrol::review::serve(
                std::path::Path::new(&args[1]),
                args[2].parse()?,
                args[3].parse()?,
            )
            .map_err(Into::into);
        }
        Some("config-check-controller") if args.len() == 3 => {
            let config =
                wigglecontrol::session::SessionConfig::load(std::path::Path::new(&args[1]))?;
            let controller =
                wigglecontrol::session::controller_config(&config, std::path::Path::new(&args[2]))?;
            println!(
                "{}",
                serde_json::json!({"valid":true,"camera_opened":false,"system_input":false,"pointer_model":"relative","visual_macros":controller.visual_policy().is_some(),
                    "neutral_pose":controller.visual_policy().map(|p|p.neutral_pose),
                    "macro_min_confidence":controller.visual_policy().map(|p|p.min_confidence),
                    "pair_separation_widths":config.pair_separation_widths,
                    "palm_deduplication":config.palm_deduplication})
            );
            return Ok(());
        }
        Some("run-controller") | Some("camera-check-controller") if args.len() == 5 => {
            return wigglecontrol::session::controller(
                std::path::Path::new(&args[2]),
                std::path::Path::new(&args[3]),
                Some(args[1].parse()?),
                args[4].parse()?,
                args[0] == "run-controller",
            );
        }
        Some("run-controller-until-stop") if args.len() == 4 => {
            return wigglecontrol::session::controller(
                std::path::Path::new(&args[1]),
                std::path::Path::new(&args[2]),
                None,
                args[3].parse()?,
                true,
            );
        }
        Some("config-check") if args.len() == 2 => {
            let config =
                wigglecontrol::session::SessionConfig::load(std::path::Path::new(&args[1]))?;
            println!(
                "{}",
                serde_json::json!({
                    "valid": true, "fps": config.fps, "gain": config.gain, "pointer_output":config.pointer_output, "prayer_enabled":config.prayer_enabled,
                    "adaptive_smoothing":config.adaptive_smoothing,"pair_separation_widths":config.pair_separation_widths,
                    "palm_deduplication":config.palm_deduplication,
                "pinch_close_ratio":config.pinch_close_ratio,
                "pinch_open_ratio":config.pinch_open_ratio,
                "pinch_3d":config.pinch_3d,"pinch_close_m":config.pinch_close_m,"pinch_open_m":config.pinch_open_m,"recording_enabled":config.recording_directory.is_some(),"freshness_ms":config.freshness_ms,
                    "touch_workspace": {
                        "min": config.touch_workspace.min, "max": config.touch_workspace.max
                    }
                })
            );
            return Ok(());
        }
        Some("image-check-touch")
            if args.len() == 3
                || ((args.len() == 4 || args.len() == 5)
                    && ["pair", "single"].contains(&args[3].as_str())) =>
        {
            let config =
                wigglecontrol::session::SessionConfig::load(std::path::Path::new(&args[2]))?;
            ort::init_from(&config.runtime).commit()?;
            let image = image::open(&args[1])?.to_rgb8();
            // Deterministic two-hand reference fixture in memory, not a camera image.
            let image = if args.len() >= 4 && args[3] == "pair" {
                let mut pair = image::RgbImage::new(image.width() * 2, image.height());
                image::imageops::replace(&mut pair, &image, 0, 0);
                image::imageops::replace(
                    &mut pair,
                    &image::imageops::flip_horizontal(&image),
                    i64::from(image.width()),
                    0,
                );
                pair
            } else {
                image
            };
            let mut model = config.perception()?;
            let iterations: usize = args.get(4).map(|s| s.parse()).transpose()?.unwrap_or(3);
            if !(1..=60).contains(&iterations) {
                return Err("reference iterations must be 1..60".into());
            }
            for _ in 0..iterations {
                let start = std::time::Instant::now();
                let result = model.infer_many(&image)?;
                println!(
                    "{}",
                    serde_json::json!({"inference_ms":start.elapsed().as_secs_f64()*1000.0,"palm_rois":model.palm_rois.iter().map(|r|[r.center.x,r.center.y,r.side]).collect::<Vec<_>>(),"skipped_rois":model.skipped_rois.iter().map(|r|[r.center.x,r.center.y,r.side]).collect::<Vec<_>>(),"last_hand_score":model.hand_score,"last_roi":model.last_roi.map(|r|serde_json::json!({"x":r.center.x,"y":r.center.y,"side":r.side})),"ambiguous":result.ambiguous,"hands":result.hands.iter().map(|h|serde_json::json!({"id":h.id,"confidence":h.confidence,"tracked":h.tracked,"landmarks":h.landmarks.iter().map(|p|[p.x,p.y]).collect::<Vec<_>>()})).collect::<Vec<_>>() })
                );
            }
            return Ok(());
        }
        Some("image-check")
            if args.len() == 3 || args.len() == 4 || (args.len() == 5 && args[3] == "--repeat") =>
        {
            let iterations: usize = if args.len() == 5 { args[4].parse()? } else { 1 };
            if !(1..=60).contains(&iterations) {
                return Err("image-check repetitions must be1..60".into());
            }
            let config =
                wigglecontrol::session::SessionConfig::load(std::path::Path::new(&args[2]))?;
            ort::init_from(&config.runtime).commit()?;
            let image = image::open(&args[1])?.to_rgb8();
            // Optional 0..9 tile selection for the upstream 5x2 reference grid.
            let image = if args.len() == 4 {
                let tile = &args[3];
                let tile: u32 = tile.parse()?;
                if tile >= 10 {
                    return Err("reference tile must be 0..9".into());
                }
                image::imageops::crop_imm(
                    &image,
                    tile % 5 * (image.width() / 5),
                    tile / 5 * (image.height() / 2),
                    image.width() / 5,
                    image.height() / 2,
                )
                .to_image()
            } else {
                image
            };
            let mut model = config.perception()?;
            for iteration in 0..iterations {
                let detected = model.infer(&image)?;
                println!(
                    "{}",
                    serde_json::json!({"iteration":iteration,"detected":detected.is_some(),"tracked":detected.as_ref().map(|h|h.tracked),"confidence":detected.as_ref().map(|h|h.confidence),"landmarks":detected.as_ref().map(|h|h.landmarks.iter().map(|p|[p.x,p.y]).collect::<Vec<_>>()),"handedness":detected.as_ref().map(|h|h.handedness),"world_pinch_distance_m":detected.as_ref().map(|h|h.world_pinch_distance_m()),"world_landmarks":detected.as_ref().map(|h|h.world_landmarks),"width":image.width(),"height":image.height(),"palm_score":model.palm_score,"hand_score":model.hand_score,"side_score":model.side_score,"roi":model.last_roi.map(|r|serde_json::json!({"x":r.center.x,"y":r.center.y,"side":r.side,"angle":r.angle}))})
                );
            }
            return Ok(());
        }
        Some("camera-check-touch") | Some("run-touch") if args.len() == 3 => {
            return wigglecontrol::session::run_touch(
                std::path::Path::new(&args[2]),
                args[1].parse()?,
                args[0] == "run-touch",
            );
        }
        Some("camera-check") | Some("run") if args.len() == 3 => {
            return wigglecontrol::session::run(
                std::path::Path::new(&args[2]),
                args[1].parse()?,
                args[0] == "run",
            );
        }
        _ => (),
    }
    #[cfg(target_os = "linux")]
    match args.first().map(String::as_str) {
        Some("touch-replay") if args.len() == 1 => {
            return Ok(wigglecontrol::touch_gestures::replay_stdin()?)
        }
        Some("controller-stream") if args.len() == 4 => {
            return Ok(wigglecontrol::delivery::stream(
                Some(args[1].parse()?),
                args[2].parse()?,
                args[3].parse()?,
            )?);
        }
        Some("controller-stream-until-stop") if args.len() == 3 => {
            return Ok(wigglecontrol::delivery::stream(
                None,
                args[1].parse()?,
                args[2].parse()?,
            )?);
        }
        Some("pipeline-replay") if args.len() == 4 => {
            return Ok(wigglecontrol::delivery::pipeline_replay_stdin(
                std::path::Path::new(&args[1]),
                args[2].parse()?,
                args[3].parse()?,
            )?);
        }
        Some("delivery-replay") if args.len() == 3 => {
            return Ok(wigglecontrol::delivery::replay_stdin(
                args[1].parse()?,
                args[2].parse()?,
            )?);
        }
        Some("mode-replay") if args.len() == 2 => {
            return Ok(wigglecontrol::controller::replay_stdin(
                std::path::Path::new(&args[1]),
            )?);
        }
        Some("macro-replay") if args.len() == 2 => {
            return Ok(wigglecontrol::macros::replay_stdin(std::path::Path::new(
                &args[1],
            ))?);
        }
        Some("touch-check") if args.len() == 2 => {
            return Ok(wigglecontrol::touch::check(args[1].parse()?)?)
        }
        Some("touch-stream") if args.len() == 2 || args.len() == 3 => {
            return Ok(wigglecontrol::touch::stream_with_watchdog(
                args[1].parse()?,
                args.get(2).map(|s| s.parse()).transpose()?.unwrap_or(250),
            )?)
        }
        Some("device-check") if args.len() == 2 => {
            return Ok(wigglecontrol::input::check(args[1].parse()?)?)
        }
        Some("absolute-stream-until-stop") if args.len() == 2 => {
            return wigglecontrol::input::absolute_until_stop(args[1].parse()?).map_err(Into::into);
        }
        Some("absolute-stream") if args.len() == 2 || args.len() == 3 => {
            return wigglecontrol::input::stream_with_watchdog(
                args[1].parse()?,
                1.0,
                true,
                args.get(2).map(|s| s.parse()).transpose()?.unwrap_or(150),
            )
            .map_err(Into::into);
        }
        Some("device-stream") if args.len() == 3 || args.len() == 4 => {
            return Ok(wigglecontrol::input::stream_with_watchdog(
                args[1].parse()?,
                args[2].parse()?,
                false,
                args.get(3).map(|s| s.parse()).transpose()?.unwrap_or(150),
            )?)
        }
        _ => (),
    }
    if args != ["replay"] {
        print_help();
        std::process::exit(2);
    }
    let mut engine = Engine::new(Config::default())?;
    let mut input = io::stdin().lock();
    let mut output = io::stdout().lock();
    // Read bounded chunks rather than allocating an arbitrarily long line.
    let mut line = Vec::with_capacity(MAX_MESSAGE_BYTES);
    loop {
        let buffer = input.fill_buf()?;
        if buffer.is_empty() {
            break;
        }
        let n = buffer
            .iter()
            .position(|b| *b == b'\n')
            .map_or(buffer.len(), |i| i + 1);
        if line.len() + n > MAX_MESSAGE_BYTES {
            for event in engine.reject() {
                serde_json::to_writer(&mut output, &event)?;
                writeln!(output)?;
            }
            output.flush()?;
            return Err("input message exceeds 16 KiB; replay stopped".into());
        }
        line.extend_from_slice(&buffer[..n]);
        input.consume(n);
        if line.last() == Some(&b'\n') {
            let events = match serde_json::from_slice::<ReplayCommand>(&line) {
                Ok(command) => replay(&mut engine, command),
                Err(error) => {
                    eprintln!("Rejected replay input: {error}");
                    engine.reject()
                }
            };
            for event in events {
                serde_json::to_writer(&mut output, &event)?;
                writeln!(output)?;
            }
            output.flush()?;
            line.clear();
        }
    }
    if !line.is_empty() {
        let events = match serde_json::from_slice::<ReplayCommand>(&line) {
            Ok(command) => replay(&mut engine, command),
            Err(error) => {
                eprintln!("Rejected replay input: {error}");
                engine.reject()
            }
        };
        for event in events {
            serde_json::to_writer(&mut output, &event)?;
            writeln!(output)?;
        }
    }
    for event in engine.stop() {
        serde_json::to_writer(&mut output, &event)?;
        writeln!(output)?;
    }
    output.flush()?;
    Ok(())
}

fn print_help() {
    println!("Usage: wigglecontrol replay < observations.jsonl");
    #[cfg(feature = "native-vision")]
    println!("       wigglecontrol provider-fixture SEQUENCE SESSION SETTINGS ASSET_ROOT PORT [MAX_HANDS] (real-clock JPEG/worker/meaning; no camera/devices)");
    #[cfg(feature = "native-vision")]
    println!("       wigglecontrol provider-replay SESSION SETTINGS PRESENCE_FLOOR < raw-landmarks.jsonl (no acquisition/models/devices)");
    #[cfg(target_os = "linux")]
    println!("       wigglecontrol touch-replay < observations.jsonl\n       wigglecontrol macro-replay CONFIG < named-poses.jsonl (no actions executed)\n       wigglecontrol mode-replay CONFIG < controller.jsonl (intent only)\n       wigglecontrol pipeline-replay SETTINGS GAIN WATCHDOG_MS < controller.jsonl (device-free)\n       wigglecontrol delivery-replay GAIN WATCHDOG_MS < packets.jsonl (device-free)\n       wigglecontrol controller-stream SECONDS GAIN WATCHDOG_MS\n       wigglecontrol controller-stream-until-stop GAIN WATCHDOG_MS\n       wigglecontrol device-check SECONDS\n       wigglecontrol device-stream SECONDS GAIN [WATCHDOG_MS]\n       wigglecontrol absolute-stream SECONDS [WATCHDOG_MS]\n       wigglecontrol touch-check SECONDS\n       wigglecontrol touch-stream SECONDS [WATCHDOG_MS]\nDevice commands open virtual devices; stream commands inject system input.");
    #[cfg(feature = "native-vision")]
    println!("       wigglecontrol config-check-controller CONFIG SETTINGS (no acquisition)\n       wigglecontrol camera-check-controller SECONDS CONFIG SETTINGS PORT\n       wigglecontrol run-controller SECONDS CONFIG SETTINGS PORT\n       wigglecontrol run-controller-until-stop CONFIG SETTINGS PORT\n       wigglecontrol run-feedback SECONDS CONFIG PORT\n       wigglecontrol calibrate-guided CONFIG PORT PROFILE (camera-only, at most 150 seconds)\n       wigglecontrol run-calibrated-until-stop CONFIG PORT PROFILE (explicit live input)\n       wigglecontrol run-calibrated SECONDS CONFIG PORT PROFILE\n       wigglecontrol review-recording DIRECTORY PORT SECONDS (camera-free human labels)\n       wigglecontrol accept-calibration PROFILE (camera-free, requires saved verification)\n       wigglecontrol camera-preview SECONDS CONFIG PORT\n       wigglecontrol config-check CONFIG\n       wigglecontrol gesture-check IMAGE CONFIG MODEL.onnx (camera-free pose inference)\n       wigglecontrol gesture-landmarks MODEL.onnx RUNTIME < normalized-image-xyz.jsonl\n       wigglecontrol controller-sequence SEQUENCE CONFIG SETTINGS (camera/device-free full controller)\n       wigglecontrol sequence-check SEQUENCE CONFIG (camera-free temporal vision)\n       wigglecontrol crop-check IMAGE CONFIG NEW_DIRECTORY [ROI.json] (camera-free exact tensor/model probe)\n       wigglecontrol position-sweep IMAGE CONFIG NEW_DIRECTORY (camera-free controlled input)\n       wigglecontrol image-check IMAGE CONFIG [TILE|--repeat N]\n       wigglecontrol image-check-touch IMAGE CONFIG [single|pair [ITERATIONS]]\n       wigglecontrol camera-check SECONDS CONFIG\n       wigglecontrol camera-check-touch SECONDS CONFIG\n       wigglecontrol run SECONDS CONFIG\n       wigglecontrol run-touch SECONDS CONFIG\nCamera-check commands open the camera; run commands also inject system input.\nPreview tests last 1..90 seconds after the first visible frame, plus bounded preparation.\nConfig-check validates JSON/settings only; it does not test resource availability.");
    println!(
        "Replay and help open no camera/device or models. Sessions are finite (1..120 seconds)."
    );
}
