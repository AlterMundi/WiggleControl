//! No Media Foundation, camera enumeration or acquisition. Generated JPEG pixels
//! and QPC publication time are explicitly labelled as a fixture in the protocol.
use super::*;
#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Fault {
    None,
    Stall,
    Partial,
    Oversized,
    WrongFormat,
    Future,
    Regress,
    Crash,
    StartupError,
}
pub fn stream(spec: Spec, fault: Fault) -> io::Result<()> {
    spec.validate()?;
    let stop = eof_stop();
    let mut output = io::stdout().lock();
    if matches!(fault, Fault::StartupError) {
        write_message(
            &mut output,
            &Message::Closed {
                run_id: spec.run_id,
                error: Some("fixture camera denied or timestamp unavailable".into()),
            },
        )?;
        return Ok(());
    }
    let mut format = spec.format;
    if matches!(fault, Fault::WrongFormat) {
        format.resolution = [640, 480];
    }
    write_message(
        &mut output,
        &Message::Ready {
            run_id: spec.run_id.clone(),
            native_capture: false,
            format,
        },
    )?;
    let image = image::RgbImage::new(format.resolution[0], format.resolution[1]);
    let mut bytes = Vec::new();
    image::codecs::jpeg::JpegEncoder::new(&mut bytes)
        .encode_image(&image)
        .map_err(io::Error::other)?;
    let mut sequence = 0u32;
    let mut previous = None;
    while !stop.stop.load(Ordering::Relaxed) {
        let mut stamp = monotonic_us()?;
        sequence += 1;
        match fault {
            Fault::Stall if sequence > 1 => loop {
                thread::park();
            },
            Fault::Partial if sequence > 1 => {
                write_message(
                    &mut output,
                    &Message::Frame {
                        sequence,
                        source_us: stamp,
                        length: bytes.len(),
                    },
                )?;
                output.write_all(&bytes[..32])?;
                output.flush()?;
                loop {
                    thread::park();
                }
            }
            Fault::Oversized if sequence > 1 => {
                write_message(
                    &mut output,
                    &Message::Frame {
                        sequence,
                        source_us: stamp,
                        length: MAX_JPEG_BYTES + 1,
                    },
                )?;
                loop {
                    thread::park();
                }
            }
            Fault::Future if sequence > 1 => stamp += 10_000_000,
            Fault::Regress if sequence > 1 => stamp = previous.unwrap(),
            _ => (),
        }
        write_frame(&mut output, sequence, stamp, &bytes)?;
        previous = Some(stamp);
        if matches!(fault, Fault::Crash) {
            std::process::exit(23);
        }
        thread::sleep(Duration::from_millis(30));
    }
    write_message(
        &mut output,
        &Message::Closed {
            run_id: spec.run_id,
            error: None,
        },
    )
}

/// Skip Drop to prove EOF recovery when the owner is gone and its driver hangs.
pub fn parent_exit(executable: &Path, spec: Spec) -> io::Result<()> {
    let capture = Capture::spawn_fixture(executable, spec, Fault::Stall)?;
    let start = Instant::now();
    while capture.status().fresh_frames == 0 {
        if capture.status().error.is_some() || start.elapsed() > Duration::from_secs(5) {
            return Err(invalid("fixture parent did not observe a frame"));
        }
        thread::sleep(Duration::from_millis(5));
    }
    println!("{}", capture.pid());
    io::stdout().flush()?;
    io::stdin().read_exact(&mut [0])?;
    std::process::exit(19);
}
