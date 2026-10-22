//! Session orchestration seams; gesture and delivery policy stay in the shared core.
use crate::{
    camera_frame::Frame,
    controller,
    output::{Packet, Publisher},
    platform::monotonic_us,
    session_config::SessionConfig,
    touch_gestures::Workspace,
    Event,
};
use std::io;
#[cfg(target_os = "linux")]
use std::{
    io::Write,
    process::{Child, ChildStdin},
};

#[derive(Default)]
pub(crate) struct Mailbox {
    pub frame: Option<Frame>,
    pub error: Option<String>,
    pub dropped: u64,
    pub frames: u64,
    pub source_kind: Option<&'static str>,
    /// Retained acquisition history, not current activity.
    pub opened: bool,
    /// Set before starting a native producer; only confirmed cleanup clears it.
    pub unclosed: bool,
}
pub(crate) struct Writer {
    #[cfg(target_os = "linux")]
    child: Child,
    #[cfg(target_os = "linux")]
    input: Option<ChildStdin>,
    #[cfg(target_os = "windows")]
    inner: crate::platform::windows::writer::Writer,
}
pub(crate) struct Closed {
    pub exited: bool,
    pub neutral: bool,
    pub error: Option<String>,
}
impl Writer {
    pub fn start(
        config: &SessionConfig,
        seconds: Option<u64>,
        target: Option<&str>,
    ) -> io::Result<Self> {
        #[cfg(target_os = "linux")]
        {
            if target.is_some() {
                return Err(io::Error::other(
                    "Windows monitor selection is unavailable on Linux",
                ));
            }
            let mut child =
                crate::platform::linux::provider_output::controller_writer(config, seconds)?;
            let input = child.stdin.take();
            Ok(Self { child, input })
        }
        #[cfg(target_os = "windows")]
        {
            let _ = seconds; // The common session deadline ends this supervised writer.
            let target = target
                .ok_or_else(|| io::Error::other("select an explicit Windows output monitor"))?;
            let desktop = validate_output(config, target)?;
            let inner = crate::platform::windows::writer::Writer::spawn(
                &std::env::current_exe()?,
                crate::platform::windows::writer::Config {
                    run_id: format!("output-{}-{}", std::process::id(), monotonic_us()?),
                    desktop,
                    target: target.to_owned(),
                    watchdog_us: config.gestures().stale_us,
                },
            )?;
            Ok(Self { inner })
        }
    }
    pub fn check(&mut self) -> io::Result<()> {
        #[cfg(target_os = "linux")]
        if let Some(status) = self.child.try_wait()? {
            return Err(io::Error::other(format!(
                "controller writer exited during session: {status}"
            )));
        }
        #[cfg(target_os = "windows")]
        if let Some(status) = self.inner.outcome() {
            return Err(io::Error::other(status.error.unwrap_or_else(|| {
                "controller writer exited during session".into()
            })));
        }
        Ok(())
    }
    pub fn send(&mut self, packet: &Packet) -> io::Result<()> {
        #[cfg(target_os = "linux")]
        {
            let input = self
                .input
                .as_mut()
                .ok_or_else(|| io::Error::other("controller writer pipe is closed"))?;
            input.write_all(&crate::recording::json_line(packet)?)?;
            input.flush()
        }
        #[cfg(target_os = "windows")]
        {
            self.inner.send(packet)
        }
    }
    pub fn stop(self, interrupted: bool) -> Closed {
        #[cfg(target_os = "linux")]
        {
            let Self { mut child, input } = self;
            drop(input);
            match child.wait() {
                Ok(status) => {
                    let result = crate::platform::linux::provider_output::check_writer_exit(
                        Some(status),
                        interrupted,
                    );
                    Closed {
                        exited: true,
                        neutral: result.is_ok(),
                        error: result.err().map(|e| e.to_string()),
                    }
                }
                Err(error) => Closed {
                    exited: false,
                    neutral: false,
                    error: Some(error.to_string()),
                },
            }
        }
        #[cfg(target_os = "windows")]
        {
            let _ = interrupted;
            let result = self.inner.stop();
            Closed {
                exited: result.process_exited,
                neutral: result.neutral,
                error: result.error,
            }
        }
    }
}
#[cfg(target_os = "windows")]
pub(crate) fn validate_output(
    config: &SessionConfig,
    target: &str,
) -> io::Result<crate::platform::windows::display::Desktop> {
    if !config.provider_absolute_pointer {
        return Err(io::Error::other(
            "Windows preview requires the shared absolute one-hand controller",
        ));
    }
    let desktop = crate::platform::windows::display::Desktop::read()?;
    if config.pointer_output != Some(desktop.mapping(target)?.output_size()) {
        return Err(io::Error::other(
            "selected monitor dimensions differ from calibration output",
        ));
    }
    Ok(desktop)
}
pub(crate) fn publish(
    writer: &mut Option<Writer>,
    publisher: &mut Publisher,
    frame: &controller::Output,
    source: Option<u64>,
    workspace: Option<Workspace>,
) -> io::Result<u64> {
    publish_at(writer, publisher, frame, source, workspace, monotonic_us()?)
}
pub(crate) fn publish_at(
    writer: &mut Option<Writer>,
    publisher: &mut Publisher,
    frame: &controller::Output,
    source: Option<u64>,
    workspace: Option<Workspace>,
    now: u64,
) -> io::Result<u64> {
    let (packet, presses) = mapped_packet(publisher, frame, source, workspace, now)?;
    if let (Some(writer), Some(packet)) = (writer, packet) {
        writer.send(&packet)?;
    }
    Ok(presses)
}

pub(crate) fn mapped_packet(
    publisher: &mut Publisher,
    frame: &controller::Output,
    source: Option<u64>,
    workspace: Option<Workspace>,
    now: u64,
) -> io::Result<(Option<Packet>, u64)> {
    let packet = match workspace {
        Some(workspace) => publisher.absolute_packet(frame, now, source, workspace)?,
        None => publisher.packet(frame, now, source)?,
    };
    let presses = if matches!(packet, Some(Packet::Frame { .. })) {
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
    Ok((packet, presses))
}
