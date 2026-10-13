//! Direct type-B touchscreen frames, separate from relative mouse semantics.
use evdev::{
    uinput::VirtualDevice, AbsInfo, AbsoluteAxisCode as A, AttributeSet, EventType, InputEvent,
    KeyCode, PropType, UinputAbsSetup,
};
use serde::{Deserialize, Serialize};
use std::{io, time::Duration};
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Contact {
    pub id: u32,
    pub x: f64,
    pub y: f64,
}
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Intent {
    Frame { contacts: Vec<Contact> },
    Heartbeat,
    Stop,
}
#[derive(Default)]
pub struct State {
    slots: [Option<u32>; 2],
    touching: bool,
}
fn abs(axis: A, value: i32) -> InputEvent {
    InputEvent::new(EventType::ABSOLUTE.0, axis.0, value)
}
impl State {
    pub fn frame(&mut self, contacts: &[Contact]) -> io::Result<Vec<InputEvent>> {
        if contacts.len() > 2
            || contacts.iter().any(|p| {
                p.id > i32::MAX as u32
                    || !p.x.is_finite()
                    || !p.y.is_finite()
                    || !(0.0..=1.0).contains(&p.x)
                    || !(0.0..=1.0).contains(&p.y)
            })
            || (contacts.len() == 2 && contacts[0].id == contacts[1].id)
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid/duplicate touch contacts",
            ));
        }
        let mut events = Vec::new();
        for slot in 0..2 {
            if self.slots[slot].is_some_and(|id| !contacts.iter().any(|p| p.id == id)) {
                events.push(abs(A::ABS_MT_SLOT, slot as i32));
                events.push(abs(A::ABS_MT_TRACKING_ID, -1));
                self.slots[slot] = None;
            }
        }
        for p in contacts {
            let (slot, fresh) = if let Some(slot) = self.slots.iter().position(|s| *s == Some(p.id))
            {
                (slot, false)
            } else {
                (
                    self.slots
                        .iter()
                        .position(Option::is_none)
                        .ok_or_else(|| io::Error::other("touch slots exhausted"))?,
                    true,
                )
            };
            events.push(abs(A::ABS_MT_SLOT, slot as i32));
            if fresh {
                events.push(abs(A::ABS_MT_TRACKING_ID, p.id as i32));
                self.slots[slot] = Some(p.id);
            }
            events.push(abs(A::ABS_MT_POSITION_X, (p.x * 65535.0).round() as i32));
            events.push(abs(A::ABS_MT_POSITION_Y, (p.y * 65535.0).round() as i32));
        }
        if let Some(p) = contacts.first() {
            events.push(abs(A::ABS_X, (p.x * 65535.0).round() as i32));
            events.push(abs(A::ABS_Y, (p.y * 65535.0).round() as i32));
        }
        let touching = !contacts.is_empty();
        if touching != self.touching {
            events.push(InputEvent::new(
                EventType::KEY.0,
                KeyCode::BTN_TOUCH.0,
                i32::from(touching),
            ));
            self.touching = touching;
        }
        Ok(events)
    }
    pub fn release(&mut self) -> Vec<InputEvent> {
        self.frame(&[]).unwrap_or_default()
    }
}
fn create() -> io::Result<VirtualDevice> {
    let mut builder = VirtualDevice::builder()?
        .name("WiggleControl Touch")
        .with_keys(&AttributeSet::from_iter([KeyCode::BTN_TOUCH]))?
        .with_properties(&AttributeSet::from_iter([PropType::DIRECT]))?;
    for (code, min, max) in [
        (A::ABS_X, 0, 65535),
        (A::ABS_Y, 0, 65535),
        (A::ABS_MT_SLOT, 0, 1),
        (A::ABS_MT_TRACKING_ID, 0, i32::MAX),
        (A::ABS_MT_POSITION_X, 0, 65535),
        (A::ABS_MT_POSITION_Y, 0, 65535),
    ] {
        builder = builder.with_absolute_axis(&UinputAbsSetup::new(
            code,
            AbsInfo::new(0, min, max, 0, 0, 0),
        ))?;
    }
    builder.build()
}
pub fn check(seconds: u64) -> io::Result<()> {
    if !(1..=120).contains(&seconds) {
        return Err(io::Error::other("duration 1..120 seconds"));
    }
    let mut device = create()?;
    for p in device.enumerate_dev_nodes_blocking()? {
        eprintln!("Created direct touchscreen: {}", p?.display());
    }
    std::thread::sleep(Duration::from_secs(seconds));
    drop(device);
    Ok(())
}
pub fn stream(seconds: u64) -> io::Result<()> {
    use std::{sync::mpsc, time::Instant};
    if !(1..=120).contains(&seconds) {
        return Err(io::Error::other("duration 1..120 seconds"));
    }
    let mut device = create()?;
    let rx = crate::intent::reader::<Intent>();
    let mut state = State::default();
    let start = Instant::now();
    let mut fresh = Instant::now();
    let result = (|| {
        while start.elapsed() < Duration::from_secs(seconds) {
            match rx.recv_timeout(Duration::from_millis(20)) {
                Ok(Ok(Intent::Frame { contacts })) => {
                    device.emit(&state.frame(&contacts)?)?;
                    fresh = Instant::now();
                }
                Ok(Ok(Intent::Heartbeat)) => fresh = Instant::now(),
                Ok(Ok(Intent::Stop)) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
                Ok(Err(e)) => return Err(e),
                Err(mpsc::RecvTimeoutError::Timeout) => (),
            }
            if fresh.elapsed() >= Duration::from_millis(250) {
                let events = state.release();
                if !events.is_empty() {
                    device.emit(&events)?;
                    eprintln!("Touch contacts released: producer watchdog");
                }
            }
        }
        Ok(())
    })();
    let release = device.emit(&state.release());
    drop(device);
    result.and(release)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reordering_preserves_slots_and_loss_ends_only_lost_id() {
        let mut s = State::default();
        let a = Contact {
            id: 7,
            x: 0.3,
            y: 0.4,
        };
        let b = Contact {
            id: 9,
            x: 0.7,
            y: 0.6,
        };
        s.frame(&[a.clone(), b.clone()]).unwrap();
        let events = s.frame(&[b.clone(), a]).unwrap();
        assert!(!events.iter().any(|e| e.code() == A::ABS_MT_TRACKING_ID.0));
        let lost = s.frame(&[b]).unwrap();
        assert!(lost
            .iter()
            .any(|e| e.code() == A::ABS_MT_TRACKING_ID.0 && e.value() == -1));
        assert!(s
            .release()
            .iter()
            .any(|e| e.code() == KeyCode::BTN_TOUCH.0 && e.value() == 0));
        assert!(s.release().is_empty());
    }
    #[test]
    fn duplicate_and_nonfinite_contacts_rejected() {
        let mut s = State::default();
        let a = Contact {
            id: 1,
            x: 0.5,
            y: 0.5,
        };
        assert!(s.frame(&[a.clone(), a]).is_err());
        assert!(s
            .frame(&[Contact {
                id: 2,
                x: f64::NAN,
                y: 0.2
            }])
            .is_err());
    }
}
