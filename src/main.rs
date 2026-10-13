use std::io::{self, BufRead, Write};
use wigglecontrol::{replay, Config, Engine, ReplayCommand, MAX_MESSAGE_BYTES};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::args().skip(1).collect::<Vec<_>>() != ["replay"] {
        eprintln!("Usage: wigglecontrol replay < observations.jsonl\nReplay emits JSON events only; it opens no camera or input device.");
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
