//! Process-failure fixtures. Every emission goes to a bounded test trace, never
//! SendInput, and no fixture reads the actual display or opens a camera.
use super::{input, invalid, json_line, Config as WriterConfig, Packet, Writer};
use crate::{controller::Mode, platform::windows::monotonic_us, Event};
use serde::{Deserialize, Serialize};
use std::{
    fs::{File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    thread,
    time::{Duration, Instant},
};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_KEYBOARD, INPUT_MOUSE, KEYEVENTF_KEYUP, MOUSEEVENTF_LEFTUP, MOUSEEVENTF_MIDDLEUP,
    MOUSEEVENTF_RIGHTUP,
};
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Fault {
    None,
    CrashAfterPress,
    StallAfterPress,
    RejectRelease,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub trace: PathBuf,
    pub fault: Fault,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct Trace {
    pub role: String,
    pub pid: u32,
    pub kind: u32,
    pub flags: u32,
    pub data: u32,
    pub accepted: bool,
}
struct Fake {
    file: File,
    role: &'static str,
    events: u32,
    reject_release: bool,
}
impl input::Sender for Fake {
    fn send(&mut self, event: &INPUT) -> io::Result<()> {
        if self.events >= 256 {
            return Err(invalid("fixture trace exhausted"));
        }
        self.events += 1;
        // SAFETY: the prepared event constructors set the corresponding SDK tag.
        let (flags, data) = match event.r#type {
            INPUT_MOUSE => {
                let mouse = unsafe { event.Anonymous.mi };
                (mouse.dwFlags, mouse.mouseData)
            }
            INPUT_KEYBOARD => {
                let key = unsafe { event.Anonymous.ki };
                (key.dwFlags, u32::from(key.wVk))
            }
            _ => return Err(invalid("unexpected fixture event tag")),
        };
        let accepted = !self.reject_release
            || if event.r#type == INPUT_MOUSE {
                flags & (MOUSEEVENTF_LEFTUP | MOUSEEVENTF_RIGHTUP | MOUSEEVENTF_MIDDLEUP) == 0
            } else {
                flags & KEYEVENTF_KEYUP == 0
            };
        self.file.write_all(&json_line(&Trace {
            role: self.role.into(),
            pid: std::process::id(),
            kind: event.r#type,
            flags,
            data,
            accepted,
        })?)?;
        self.file.flush()?;
        if accepted {
            Ok(())
        } else {
            Err(invalid("fixture release rejected"))
        }
    }
}
pub fn stream(config: WriterConfig, fixture: Config) -> io::Result<()> {
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&fixture.trace)?;
    super::stream_with(
        config,
        input::Injector::with_sender(Fake {
            file,
            role: "child",
            events: 0,
            reject_release: matches!(fixture.fault, Fault::RejectRelease),
        }),
        false,
        fixture.fault,
    )
}
pub(super) fn recover(fixture: &Config, mask: u8) -> io::Result<()> {
    let file = OpenOptions::new().append(true).open(&fixture.trace)?;
    input::Injector::with_sender(Fake {
        file,
        role: "fallback",
        events: 0,
        reject_release: matches!(fixture.fault, Fault::RejectRelease),
    })
    .release_possible(mask)
}
/// Deliberately exits without Rust Drop to exercise the orphan child's EOF path.
pub fn parent_exit(executable: &Path, config: WriterConfig, fixture: Config) -> io::Result<()> {
    let mut writer = Writer::spawn_fixture(executable, config, fixture)?;
    let issued_us = monotonic_us()?;
    writer.send(&Packet::Transition {
        sequence: 1,
        issued_us,
        mode: Mode::Pointer,
        enabled: true,
    })?;
    let issued_us = monotonic_us()?;
    writer.send(&Packet::Frame {
        sequence: 2,
        issued_us,
        source_us: issued_us,
        mode: Mode::Pointer,
        mouse: vec![Event::Button { down: true }],
        absolute: None,
        contacts: vec![],
    })?;
    let start = Instant::now();
    while !writer.receipt().is_some_and(|r| r.held_mask == 1) {
        if start.elapsed() > Duration::from_secs(3) || writer.outcome().is_some() {
            return Err(invalid("fixture parent did not observe its press"));
        }
        thread::sleep(Duration::from_millis(5));
    }
    println!("{}", writer.pid());
    io::stdout().flush()?;
    std::process::exit(19);
}
