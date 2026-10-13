use std::io::{self, BufRead, Write};
use wigglecontrol::{replay, Config, Engine, ReplayCommand, MAX_MESSAGE_BYTES};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    #[cfg(target_os = "linux")]
    match args.first().map(String::as_str) {
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
        eprintln!("Usage: wigglecontrol replay < observations.jsonl\n       wigglecontrol device-check SECONDS\n       wigglecontrol device-stream SECONDS GAIN\nReplay opens no camera/device. Device-stream injects mouse input.");
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
