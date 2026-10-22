//! Windows native event conversion and per-button API-confirmed ownership.
//! No camera or session is started by constructing a backend or injector.
use super::display::Mapping;
use crate::{
    contacts::Contact,
    output::{self, MouseBackend, MouseIntent, TouchBackend},
    MouseButton,
};
use std::{io, mem::size_of};
use windows_sys::Win32::{
    Foundation::{GetLastError, SetLastError},
    UI::Input::KeyboardAndMouse::*,
};

#[derive(Clone, Copy)]
enum Effect {
    None,
    Button(MouseButton, bool),
    Escape(bool),
}
#[derive(Clone, Copy)]
pub struct PreparedEvent {
    raw: INPUT,
    effect: Effect,
}
fn mouse(flags: u32, x: i32, y: i32, data: i32, effect: Effect) -> PreparedEvent {
    PreparedEvent {
        raw: INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: x,
                    dy: y,
                    mouseData: data as u32,
                    dwFlags: flags,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
        effect,
    }
}
fn escape(down: bool) -> PreparedEvent {
    PreparedEvent {
        raw: INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VK_ESCAPE,
                    wScan: 0,
                    dwFlags: if down { 0 } else { KEYEVENTF_KEYUP },
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
        effect: Effect::Escape(down),
    }
}
fn index(button: MouseButton) -> usize {
    match button {
        MouseButton::Left => 0,
        MouseButton::Right => 1,
        MouseButton::Middle => 2,
    }
}
fn button(button: MouseButton, down: bool) -> PreparedEvent {
    mouse(
        match (button, down) {
            (MouseButton::Left, true) => MOUSEEVENTF_LEFTDOWN,
            (MouseButton::Left, false) => MOUSEEVENTF_LEFTUP,
            (MouseButton::Right, true) => MOUSEEVENTF_RIGHTDOWN,
            (MouseButton::Right, false) => MOUSEEVENTF_RIGHTUP,
            (MouseButton::Middle, true) => MOUSEEVENTF_MIDDLEDOWN,
            (MouseButton::Middle, false) => MOUSEEVENTF_MIDDLEUP,
        },
        0,
        0,
        0,
        Effect::Button(button, down),
    )
}
#[derive(Default)]
pub struct Mouse {
    mapping: Option<Mapping>,
    held: [bool; 3],
}
impl Mouse {
    pub fn configured(mapping: Mapping) -> Self {
        Self {
            mapping: Some(mapping),
            ..Default::default()
        }
    }
    fn button(&mut self, button_id: MouseButton, down: bool) -> Vec<PreparedEvent> {
        if std::mem::replace(&mut self.held[index(button_id)], down) == down {
            vec![]
        } else {
            vec![button(button_id, down)]
        }
    }
}
impl MouseBackend for Mouse {
    type Event = PreparedEvent;
    fn is_held(&self) -> bool {
        self.held.iter().any(|v| *v)
    }
    fn release(&mut self) -> Vec<PreparedEvent> {
        [MouseButton::Left, MouseButton::Right, MouseButton::Middle]
            .into_iter()
            .flat_map(|b| self.button(b, false))
            .collect()
    }
    fn apply(&mut self, intent: MouseIntent, _gain: f64) -> io::Result<Vec<PreparedEvent>> {
        match intent {
            MouseIntent::Absolute { x, y } => {
                let (x, y) = self
                    .mapping
                    .ok_or_else(|| io::Error::other("select a Windows output before input"))?
                    .absolute(crate::Point { x, y })?;
                Ok(vec![mouse(
                    MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_VIRTUALDESK,
                    x,
                    y,
                    0,
                    Effect::None,
                )])
            }
            MouseIntent::Move { .. } => Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "Windows preview requires calibrated absolute pointing",
            )),
            MouseIntent::Button { down } => Ok(self.button(MouseButton::Left, down)),
            MouseIntent::NamedButton { button, down } => Ok(self.button(button, down)),
            MouseIntent::Scroll {
                vertical,
                horizontal,
            } => {
                if vertical.abs_diff(0) > 10 || horizontal.abs_diff(0) > 10 {
                    return Err(io::Error::other("wheel intent exceeds ten steps"));
                }
                self.apply(
                    MouseIntent::ScrollFine {
                        vertical: vertical * 120,
                        horizontal: horizontal * 120,
                    },
                    1.0,
                )
            }
            MouseIntent::ScrollFine {
                vertical,
                horizontal,
            } => {
                if vertical.abs_diff(0) > 1200 || horizontal.abs_diff(0) > 1200 {
                    return Err(io::Error::other("fine wheel intent exceeds ten detents"));
                }
                Ok([
                    (vertical, MOUSEEVENTF_WHEEL),
                    (horizontal, MOUSEEVENTF_HWHEEL),
                ]
                .into_iter()
                .filter(|(v, _)| *v != 0)
                .map(|(v, f)| mouse(f, 0, 0, v, Effect::None))
                .collect())
            }
            MouseIntent::Heartbeat => Ok(vec![]),
            MouseIntent::Release | MouseIntent::Stop => Ok(self.release()),
        }
    }
}
#[derive(Default)]
pub struct NoTouch;
impl TouchBackend for NoTouch {
    type Event = PreparedEvent;
    fn frame(&mut self, contacts: &[Contact]) -> io::Result<Vec<PreparedEvent>> {
        if !contacts.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "Windows preview has no touch device",
            ));
        }
        Ok(vec![])
    }
    fn release(&mut self) -> Vec<PreparedEvent> {
        vec![]
    }
}
pub type State = output::State<Mouse, NoTouch>;
pub type Batch = output::Batch<Mouse, NoTouch>;
/// Actual API and device-free test sinks share only the final emission seam.
pub trait Sender {
    fn send(&mut self, event: &INPUT) -> io::Result<()>;
}
#[derive(Default)]
pub struct NativeApi;
impl Sender for NativeApi {
    fn send(&mut self, event: &INPUT) -> io::Result<()> {
        // SAFETY: one live SDK INPUT with its correct ABI size. No retained pointer.
        unsafe {
            SetLastError(0);
        }
        let accepted = unsafe { SendInput(1, event, size_of::<INPUT>() as i32) };
        if accepted != 1 {
            let code = unsafe { GetLastError() };
            return Err(io::Error::other(format!("SendInput accepted {accepted}/1 events; native_error={code}; rejection cause is not established")));
        }
        Ok(())
    }
}
/// Native ownership is retained until a successful API release, independently
/// of logical batch preparation. API acceptance is not an application receipt.
pub struct Injector<S: Sender = NativeApi> {
    sender: S,
    held: [bool; 3],
    escape_held: bool,
}
impl Default for Injector<NativeApi> {
    fn default() -> Self {
        Self::with_sender(NativeApi)
    }
}
impl<S: Sender> Injector<S> {
    pub fn with_sender(sender: S) -> Self {
        Self {
            sender,
            held: [false; 3],
            escape_held: false,
        }
    }
    pub fn held_mask(&self) -> u8 {
        self.held
            .iter()
            .enumerate()
            .fold(0, |v, (i, h)| v | if *h { 1 << i } else { 0 })
            | if self.escape_held { 8 } else { 0 }
    }
    fn event(&mut self, event: PreparedEvent) -> io::Result<()> {
        // A logical cleanup can follow an already-confirmed native cleanup.
        // Do not emit another button-up for an input this injector no longer owns.
        if matches!(event.effect, Effect::Button(button, false) if !self.held[index(button)]) {
            return Ok(());
        }
        self.sender.send(&event.raw)?;
        match event.effect {
            Effect::Button(b, down) => self.held[index(b)] = down,
            Effect::Escape(down) => self.escape_held = down,
            Effect::None => (),
        }
        Ok(())
    }
    pub fn release(&mut self) -> io::Result<()> {
        let mut error = None;
        for b in [MouseButton::Left, MouseButton::Right, MouseButton::Middle] {
            if self.held[index(b)] {
                if let Err(e) = self.event(button(b, false)) {
                    error.get_or_insert(e);
                }
            }
        }
        if self.escape_held {
            if let Err(e) = self.event(escape(false)) {
                error.get_or_insert(e);
            }
        }
        error.map_or(Ok(()), Err)
    }
    pub fn write(&mut self, batch: Batch) -> io::Result<()> {
        let result = (|| {
            if !batch.touch.is_empty() {
                return Err(io::Error::other("touch output in Windows mouse session"));
            }
            for event in batch.mouse {
                self.event(event)?;
            }
            if batch.escape {
                self.release()?;
                let press = self.event(escape(true));
                let release = self.event(escape(false));
                press.and(release)?;
            }
            Ok(())
        })();
        if let Err(error) = result {
            return match self.release() {
                Ok(()) => Err(error),
                Err(cleanup) => Err(io::Error::other(format!(
                    "{error}; input cleanup unconfirmed: {cleanup}"
                ))),
            };
        }
        Ok(())
    }
}
impl<S: Sender> Drop for Injector<S> {
    fn drop(&mut self) {
        let _ = self.release();
    }
}

#[cfg(test)]
mod tests {
    use super::super::display::Rect;
    use super::*;
    use std::{cell::RefCell, rc::Rc};
    #[derive(Clone, Copy, Debug, PartialEq)]
    struct Attempt(u32, u32, i32);
    struct Fake {
        log: Rc<RefCell<Vec<Attempt>>>,
        fail: Vec<usize>,
    }
    impl Sender for Fake {
        fn send(&mut self, event: &INPUT) -> io::Result<()> {
            // SAFETY: our prepared constructors set the matching union tag.
            let item = if event.r#type == INPUT_MOUSE {
                let m = unsafe { event.Anonymous.mi };
                Attempt(INPUT_MOUSE, m.dwFlags, m.mouseData as i32)
            } else {
                let k = unsafe { event.Anonymous.ki };
                assert_eq!(k.wVk, VK_ESCAPE);
                Attempt(INPUT_KEYBOARD, k.dwFlags, 0)
            };
            let mut log = self.log.borrow_mut();
            log.push(item);
            if self.fail.contains(&log.len()) {
                Err(io::Error::other("fixture API rejection"))
            } else {
                Ok(())
            }
        }
    }
    fn mapping() -> Mapping {
        let r = Rect {
            left: 0,
            top: 0,
            width: 1920,
            height: 1080,
        };
        Mapping::new(r, r).unwrap()
    }
    fn mock(fail: Vec<usize>) -> (Injector<Fake>, Rc<RefCell<Vec<Attempt>>>) {
        let log = Rc::new(RefCell::new(Vec::new()));
        (
            Injector::with_sender(Fake {
                log: log.clone(),
                fail,
            }),
            log,
        )
    }
    #[test]
    fn codec_preserves_buttons_absolute_mapping_and_signed_two_axis_wheel() {
        let mut mouse = Mouse::configured(mapping());
        let position = mouse
            .apply(MouseIntent::Absolute { x: 1.0, y: 0.0 }, 1.0)
            .unwrap();
        let raw = unsafe { position[0].raw.Anonymous.mi };
        assert_eq!((raw.dx, raw.dy), (65535, 0));
        assert_eq!(
            raw.dwFlags,
            MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_VIRTUALDESK
        );
        let (mut sink, log) = mock(vec![]);
        let events = mouse
            .apply(
                MouseIntent::NamedButton {
                    button: MouseButton::Right,
                    down: true,
                },
                1.0,
            )
            .unwrap();
        sink.write(Batch {
            mouse: events,
            ..Default::default()
        })
        .unwrap();
        assert_eq!(sink.held_mask(), 2);
        assert!(mouse
            .apply(
                MouseIntent::NamedButton {
                    button: MouseButton::Right,
                    down: true
                },
                1.0
            )
            .unwrap()
            .is_empty());
        let events = mouse
            .apply(
                MouseIntent::ScrollFine {
                    vertical: 37,
                    horizontal: -53,
                },
                1.0,
            )
            .unwrap();
        sink.write(Batch {
            mouse: events,
            ..Default::default()
        })
        .unwrap();
        sink.release().unwrap();
        assert_eq!(
            *log.borrow(),
            vec![
                Attempt(INPUT_MOUSE, MOUSEEVENTF_RIGHTDOWN, 0),
                Attempt(INPUT_MOUSE, MOUSEEVENTF_WHEEL, 37),
                Attempt(INPUT_MOUSE, MOUSEEVENTF_HWHEEL, -53),
                Attempt(INPUT_MOUSE, MOUSEEVENTF_RIGHTUP, 0)
            ]
        );
        assert_eq!(sink.held_mask(), 0);
        assert!(Mouse::default()
            .apply(MouseIntent::Absolute { x: 0.5, y: 0.5 }, 1.0)
            .is_err());
        assert!(mouse
            .apply(
                MouseIntent::ScrollFine {
                    vertical: i32::MIN,
                    horizontal: 0
                },
                1.0
            )
            .is_err());
        assert!(mouse
            .apply(MouseIntent::Move { dx: 0.1, dy: 0.1 }, 1.0)
            .is_err());
    }
    #[test]
    fn failed_partial_batch_releases_only_api_accepted_owned_buttons() {
        let (mut sink, log) = mock(vec![2]);
        let batch = Batch {
            mouse: vec![
                button(MouseButton::Left, true),
                button(MouseButton::Right, true),
            ],
            ..Default::default()
        };
        assert!(sink.write(batch).is_err());
        assert_eq!(sink.held_mask(), 0);
        assert_eq!(
            *log.borrow(),
            vec![
                Attempt(INPUT_MOUSE, MOUSEEVENTF_LEFTDOWN, 0),
                Attempt(INPUT_MOUSE, MOUSEEVENTF_RIGHTDOWN, 0),
                Attempt(INPUT_MOUSE, MOUSEEVENTF_LEFTUP, 0)
            ]
        );
    }
    #[test]
    fn escape_releases_mouse_before_tap_and_retries_failed_key_release_without_a_new_press() {
        let (mut sink, log) = mock(vec![4]);
        sink.write(Batch {
            mouse: vec![button(MouseButton::Left, true)],
            ..Default::default()
        })
        .unwrap();
        assert!(sink
            .write(Batch {
                mouse: vec![button(MouseButton::Left, false)],
                escape: true,
                ..Default::default()
            })
            .is_err());
        assert_eq!(
            *log.borrow(),
            vec![
                Attempt(INPUT_MOUSE, MOUSEEVENTF_LEFTDOWN, 0),
                Attempt(INPUT_MOUSE, MOUSEEVENTF_LEFTUP, 0),
                Attempt(INPUT_KEYBOARD, 0, 0),
                Attempt(INPUT_KEYBOARD, KEYEVENTF_KEYUP, 0),
                Attempt(INPUT_KEYBOARD, KEYEVENTF_KEYUP, 0)
            ]
        );
        assert_eq!(sink.held_mask(), 0);
    }
    #[test]
    fn unconfirmed_cleanup_retains_ownership_and_drop_attempts_release() {
        let (mut sink, log) = mock(vec![2]);
        sink.write(Batch {
            mouse: vec![button(MouseButton::Middle, true)],
            ..Default::default()
        })
        .unwrap();
        assert!(sink.release().is_err());
        assert_eq!(sink.held_mask(), 4);
        drop(sink);
        assert_eq!(
            log.borrow().last(),
            Some(&Attempt(INPUT_MOUSE, MOUSEEVENTF_MIDDLEUP, 0))
        );
        assert_eq!(log.borrow().len(), 3);
    }
    #[test]
    fn shared_writer_watchdog_delivers_windows_release_and_never_represses_after_expiry() {
        use crate::{controller::Mode, output::Packet, Event, Point};
        let mut state = State::with_backends(Mouse::configured(mapping()), NoTouch);
        let (mut sink, log) = mock(vec![]);
        assert!(state.tick(0, 150_000).unwrap().mouse.is_empty());
        sink.write(
            state
                .apply(
                    1,
                    Packet::Transition {
                        sequence: 1,
                        issued_us: 1,
                        mode: Mode::Pointer,
                        enabled: true,
                    },
                    2000.0,
                    150_000,
                )
                .unwrap(),
        )
        .unwrap();
        assert!(log.borrow().is_empty());
        sink.write(
            state
                .apply(
                    2,
                    Packet::Frame {
                        sequence: 2,
                        issued_us: 2,
                        source_us: 2,
                        mode: Mode::Pointer,
                        mouse: vec![Event::NamedButton {
                            button: MouseButton::Right,
                            down: true,
                        }],
                        absolute: Some(Point { x: 0.5, y: 0.5 }),
                        contacts: vec![],
                    },
                    2000.0,
                    150_000,
                )
                .unwrap(),
        )
        .unwrap();
        assert_eq!(sink.held_mask(), 2);
        sink.write(
            state
                .apply(
                    100_000,
                    Packet::Hold {
                        sequence: 3,
                        issued_us: 100_000,
                        source_us: 2,
                        until_us: 350_000,
                        mode: Mode::Pointer,
                    },
                    2000.0,
                    150_000,
                )
                .unwrap(),
        )
        .unwrap();
        assert!(state.tick(249_999, 150_000).unwrap().mouse.is_empty());
        sink.write(state.tick(250_000, 150_000).unwrap()).unwrap();
        assert_eq!(sink.held_mask(), 0);
        assert_eq!(
            log.borrow().last(),
            Some(&Attempt(INPUT_MOUSE, MOUSEEVENTF_RIGHTUP, 0))
        );
        assert!(state
            .apply(
                250_001,
                Packet::Frame {
                    sequence: 4,
                    issued_us: 250_001,
                    source_us: 250_001,
                    mode: Mode::Pointer,
                    mouse: vec![Event::Button { down: true }],
                    absolute: None,
                    contacts: vec![]
                },
                2000.0,
                150_000
            )
            .is_err());
    }
    #[test]
    fn repeated_logical_cleanup_never_releases_an_unowned_button() {
        let (mut sink, log) = mock(vec![]);
        sink.write(Batch {
            mouse: vec![button(MouseButton::Left, true)],
            ..Default::default()
        })
        .unwrap();
        sink.release().unwrap();
        sink.write(Batch {
            mouse: vec![
                button(MouseButton::Left, false),
                button(MouseButton::Right, false),
            ],
            ..Default::default()
        })
        .unwrap();
        assert_eq!(
            *log.borrow(),
            vec![
                Attempt(INPUT_MOUSE, MOUSEEVENTF_LEFTDOWN, 0),
                Attempt(INPUT_MOUSE, MOUSEEVENTF_LEFTUP, 0)
            ]
        );
        sink.write(Batch {
            mouse: vec![button(MouseButton::Middle, true)],
            ..Default::default()
        })
        .unwrap();
        sink.write(Batch {
            escape: true,
            ..Default::default()
        })
        .unwrap();
        assert_eq!(
            &log.borrow()[2..],
            &[
                Attempt(INPUT_MOUSE, MOUSEEVENTF_MIDDLEDOWN, 0),
                Attempt(INPUT_MOUSE, MOUSEEVENTF_MIDDLEUP, 0),
                Attempt(INPUT_KEYBOARD, 0, 0),
                Attempt(INPUT_KEYBOARD, KEYEVENTF_KEYUP, 0)
            ]
        );
    }
}
