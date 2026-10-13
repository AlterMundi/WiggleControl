use std::io::{self, BufRead, Write};
use wigglecontrol::{replay, Config, Engine, ReplayCommand, MAX_MESSAGE_BYTES};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    #[cfg(feature = "native-vision")]
    match args.first().map(String::as_str) {
        Some("image-check") if args.len() == 3 || args.len() == 4 => {
            let config =
                wigglecontrol::session::SessionConfig::load(std::path::Path::new(&args[2]))?;
            ort::init_from(&config.runtime).commit()?;
            let image = image::open(&args[1])?.to_rgb8();
            // Optional 0..9 tile selection for the upstream 5x2 reference grid.
            let image = if let Some(tile) = args.get(3) {
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
            let mut model =
                wigglecontrol::vision::Perception::new(std::path::Path::new(&config.models))?;
            let detected = model.infer(&image)?;
            println!(
                "{}",
                serde_json::json!({"detected":detected.is_some(),"confidence":detected.as_ref().map(|h|h.confidence),"landmarks":detected.as_ref().map(|h|h.landmarks.iter().map(|p|[p.x,p.y]).collect::<Vec<_>>()),"handedness":detected.as_ref().map(|h|h.handedness),"width":image.width(),"height":image.height(),"palm_score":model.palm_score,"hand_score":model.hand_score,"side_score":model.side_score,"roi":model.last_roi.map(|r|serde_json::json!({"x":r.center.x,"y":r.center.y,"side":r.side,"angle":r.angle}))})
            );
            return Ok(());
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
        Some("touch-check") if args.len() == 2 => {
            return Ok(wigglecontrol::touch::check(args[1].parse()?)?)
        }
        Some("touch-stream") if args.len() == 2 => {
            return Ok(wigglecontrol::touch::stream(args[1].parse()?)?)
        }
        Some("device-check") if args.len() == 2 => {
            return Ok(wigglecontrol::input::check(args[1].parse()?)?)
        }
        Some("device-stream") if args.len() == 3 => {
            return Ok(wigglecontrol::input::stream(
                args[1].parse()?,
                args[2].parse()?,
            )?)
        }
        _ => (),
    }
    if args != ["replay"] {
        eprintln!("Usage: wigglecontrol replay < observations.jsonl\n       wigglecontrol device-check SECONDS\n       wigglecontrol device-stream SECONDS GAIN\n       wigglecontrol touch-check SECONDS\n       wigglecontrol touch-stream SECONDS\nReplay opens no camera/device. Device-stream and touch-stream inject system input.");
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
