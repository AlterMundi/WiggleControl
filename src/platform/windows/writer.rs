//! Independently polled output child, acknowledged ownership and parent recovery.
//! No construction or spawn arms input. Only explicit fresh common packets can.
use super::{
    display::Desktop,
    input, monotonic_us,
    process::{terminate, CREATE_NO_WINDOW},
};
use crate::{controller::Mode, output::Packet, Event, MouseButton, MAX_MESSAGE_BYTES};
use serde::{Deserialize, Serialize};
use std::{
    collections::VecDeque,
    io::{self, BufRead, BufReader, Write},
    os::windows::process::CommandExt,
    path::Path,
    process::{Child, ChildStdin, Command, Stdio},
    sync::{mpsc, Arc, Condvar, Mutex},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

#[doc(hidden)]
pub mod fixture;
const POLL: Duration = Duration::from_millis(20);
const STARTUP: Duration = Duration::from_secs(5);
const CLOSE_GRACE: Duration = Duration::from_millis(400);

fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}
fn short(error: impl std::fmt::Display) -> String {
    error.to_string().chars().take(1024).collect()
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub run_id: String,
    pub desktop: Desktop,
    pub target: String,
    pub watchdog_us: u64,
}
impl Config {
    pub fn validate(&self) -> io::Result<()> {
        if self.run_id.is_empty()
            || self.run_id.len() > 64
            || !self
                .run_id
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'-')
            || !(50_000..=1_000_000).contains(&self.watchdog_us)
            || self.desktop.monitors.is_empty()
            || self.desktop.monitors.len() > 32
        {
            return Err(invalid("invalid Windows writer configuration"));
        }
        let mut ids = std::collections::HashSet::new();
        for monitor in &self.desktop.monitors {
            if monitor.id.is_empty() || monitor.id.len() > 128 || !ids.insert(&monitor.id) {
                return Err(invalid("invalid or duplicate monitor ID"));
            }
            super::display::Mapping::new(self.desktop.bounds, monitor.bounds)?;
        }
        self.desktop.mapping(&self.target)?;
        Ok(())
    }
    fn check_display(&self) -> io::Result<()> {
        let mut actual = Desktop::read()?;
        let mut expected = self.desktop.clone();
        actual.monitors.sort_by(|a, b| a.id.cmp(&b.id));
        expected.monitors.sort_by(|a, b| a.id.cmp(&b.id));
        if actual != expected {
            return Err(invalid(
                "display layout changed; stop and select/calibrate the output again",
            ));
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Receipt {
    pub run_id: String,
    pub native_input: bool,
    pub sequence: Option<u64>,
    pub held_mask: u8,
    pub armed: bool,
    pub stopped: bool,
    pub error: Option<String>,
}
/// Terminal status distinguishes clean closure from a failure recovered to neutral.
#[derive(Clone, Debug)]
pub struct Outcome {
    pub process_exited: bool,
    pub unconfirmed_mask: u8,
    pub neutral: bool,
    pub fallback_used: bool,
    pub error: Option<String>,
}
fn sequence(packet: &Packet) -> Option<u64> {
    match packet {
        Packet::Transition { sequence, .. }
        | Packet::Frame { sequence, .. }
        | Packet::Hold { sequence, .. }
        | Packet::Cursor { sequence, .. }
        | Packet::Escape { sequence, .. } => Some(*sequence),
        Packet::Stop => None,
    }
}
fn possible_press(packet: &Packet) -> u8 {
    match packet {
        Packet::Escape { .. } => 8,
        Packet::Frame { mouse, .. } => mouse.iter().fold(0, |mask, event| {
            mask | match event {
                Event::Button { down: true } => 1,
                Event::NamedButton { button, down: true } => match button {
                    MouseButton::Left => 1,
                    MouseButton::Right => 2,
                    MouseButton::Middle => 4,
                },
                _ => 0,
            }
        }),
        _ => 0,
    }
}
fn check_packet(packet: &Packet) -> io::Result<()> {
    match packet {
        Packet::Transition { mode, .. }
        | Packet::Frame { mode, .. }
        | Packet::Hold { mode, .. }
        | Packet::Cursor { mode, .. }
        | Packet::Escape { mode, .. }
            if !matches!(mode, Mode::Pointer | Mode::Scroll) =>
        {
            Err(invalid("unsupported Windows absolute mouse mode"))
        }
        Packet::Frame {
            mouse, contacts, ..
        } if !contacts.is_empty() || mouse.iter().any(|e| matches!(e, Event::Move { .. })) => Err(
            invalid("Windows preview requires absolute mouse packets without touch"),
        ),
        _ => Ok(()),
    }
}
fn json_line<T: Serialize>(value: &T) -> io::Result<Vec<u8>> {
    let mut bytes = serde_json::to_vec(value)?;
    bytes.push(b'\n');
    if bytes.len() > MAX_MESSAGE_BYTES {
        return Err(invalid("writer message exceeds 16 KiB"));
    }
    Ok(bytes)
}
fn read_receipt(reader: &mut impl BufRead) -> io::Result<Option<Receipt>> {
    let mut bytes = Vec::new();
    let count =
        std::io::Read::take(reader, MAX_MESSAGE_BYTES as u64 + 1).read_until(b'\n', &mut bytes)?;
    if count == 0 {
        return Ok(None);
    }
    if count > MAX_MESSAGE_BYTES {
        return Err(invalid("writer receipt exceeds 16 KiB"));
    }
    Ok(Some(serde_json::from_slice(&bytes)?))
}

/// Only called by the explicit Windows writer command, never on app launch.
pub fn stream(config: Config) -> io::Result<()> {
    config.validate()?;
    config.check_display()?;
    stream_with(
        config,
        input::Injector::default(),
        true,
        fixture::Fault::None,
    )
}
fn stream_with<S: input::Sender>(
    config: Config,
    mut injector: input::Injector<S>,
    native_input: bool,
    fault: fixture::Fault,
) -> io::Result<()> {
    config.validate()?;
    let mut state = input::State::with_backends(
        input::Mouse::configured(config.desktop.mapping(&config.target)?),
        input::NoTouch,
    );
    let receiver = crate::intent::reader::<Packet>();
    let mut stdout = io::stdout().lock();
    let mut last_sequence = None;
    let mut display_check = Instant::now();
    let receipt = |sink: &mut dyn Write,
                   state: &input::State,
                   injector: &input::Injector<S>,
                   seq,
                   stopped,
                   error| {
        let value = Receipt {
            run_id: config.run_id.clone(),
            native_input,
            sequence: seq,
            held_mask: injector.held_mask(),
            armed: state.is_armed(),
            stopped,
            error,
        };
        sink.write_all(&json_line(&value)?)?;
        sink.flush()
    };
    let result: io::Result<()> = (|| {
        receipt(&mut stdout, &state, &injector, None, false, None)?;
        loop {
            // Tick even under a busy packet producer; no dependence on inference.
            injector.write(state.tick(monotonic_us()?, config.watchdog_us)?)?;
            if native_input && display_check.elapsed() >= Duration::from_millis(250) {
                config.check_display()?;
                display_check = Instant::now();
            }
            match receiver.recv_timeout(POLL) {
                Ok(Ok(packet)) => {
                    check_packet(&packet)?;
                    let seq = sequence(&packet);
                    let stop = matches!(packet, Packet::Stop);
                    injector.write(state.apply(
                        monotonic_us()?,
                        packet,
                        1.0,
                        config.watchdog_us,
                    )?)?;
                    if injector.held_mask() != 0 {
                        match fault {
                            fixture::Fault::CrashAfterPress => std::process::exit(17),
                            fixture::Fault::StallAfterPress => {
                                receipt(&mut stdout, &state, &injector, seq, false, None)?;
                                loop {
                                    thread::park();
                                }
                            }
                            fixture::Fault::None | fixture::Fault::RejectRelease => (),
                        }
                    }
                    last_sequence = seq.or(last_sequence);
                    if stop {
                        break;
                    }
                }
                Ok(Err(error)) => return Err(error),
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
                Err(mpsc::RecvTimeoutError::Timeout) => (),
            }
            receipt(&mut stdout, &state, &injector, last_sequence, false, None)?;
        }
        Ok(())
    })();
    // Cleanup precedes stdout, including broken-pipe/protocol/API failures.
    let cleanup = injector
        .write(state.release())
        .and_then(|_| injector.release());
    let error = match (&result, &cleanup) {
        (Err(error), Err(cleanup)) => {
            Some(short(format!("{error}; cleanup unconfirmed: {cleanup}")))
        }
        (Err(error), _) | (_, Err(error)) => Some(short(error)),
        _ => None,
    };
    let terminal = receipt(&mut stdout, &state, &injector, last_sequence, true, error);
    result.and(cleanup).and(terminal)
}

struct Ledger {
    last_posted: Option<u64>,
    pending: VecDeque<(u64, u8)>,
    receipt: Option<Receipt>,
    last_ack: Instant,
    fault: Option<String>,
    closing: Option<Instant>,
    outcome: Option<Outcome>,
}
impl Ledger {
    fn possible(&self) -> u8 {
        self.pending.iter().fold(
            self.receipt.as_ref().map_or(0, |r| r.held_mask),
            |mask, (_, bits)| mask | bits,
        )
    }
    fn acknowledge(
        &mut self,
        receipt: Receipt,
        config: &Config,
        native_input: bool,
    ) -> io::Result<()> {
        if receipt.run_id != config.run_id
            || receipt.native_input != native_input
            || receipt.held_mask & !15 != 0
            || (receipt.stopped && receipt.armed)
            || receipt.sequence > self.last_posted
            || self
                .receipt
                .as_ref()
                .is_some_and(|old| receipt.sequence < old.sequence || old.stopped)
        {
            return Err(invalid("invalid/regressing Windows writer receipt"));
        }
        if receipt.stopped {
            // No further native downs can follow a terminal receipt.
            self.pending.clear();
        } else {
            while self
                .pending
                .front()
                .is_some_and(|(seq, _)| Some(*seq) <= receipt.sequence)
            {
                self.pending.pop_front();
            }
        }
        self.last_ack = Instant::now();
        self.receipt = Some(receipt);
        Ok(())
    }
}
type Shared = Arc<(Mutex<Ledger>, Condvar)>;
/// Independent supervisor survives a blocked producer/send and never restarts input.
pub struct Writer {
    input: Option<ChildStdin>,
    shared: Shared,
    monitor: Option<JoinHandle<()>>,
    pid: u32,
}
impl Writer {
    /// Explicit user Start only. `executable` is this package's wigglecontrol binary.
    pub fn spawn(executable: &Path, config: Config) -> io::Result<Self> {
        config.validate()?;
        config.check_display()?;
        Self::start(executable, config, None)
    }
    #[doc(hidden)]
    pub fn spawn_fixture(
        executable: &Path,
        config: Config,
        fixture: fixture::Config,
    ) -> io::Result<Self> {
        Self::start(executable, config, Some(fixture))
    }
    fn start(
        executable: &Path,
        config: Config,
        fixture: Option<fixture::Config>,
    ) -> io::Result<Self> {
        config.validate()?;
        let mut command = Command::new(executable);
        command
            .arg(if fixture.is_some() {
                "windows-writer-fixture"
            } else {
                "windows-writer"
            })
            .arg(String::from_utf8(json_line(&config)?).map_err(|error| invalid(short(error)))?)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .creation_flags(CREATE_NO_WINDOW);
        if let Some(fixture) = &fixture {
            command.arg(
                String::from_utf8(json_line(fixture)?).map_err(|error| invalid(short(error)))?,
            );
        }
        let mut child = command.spawn()?;
        let pid = child.id();
        let input = child.stdin.take();
        let stdout = child.stdout.take().expect("piped child stdout");
        let shared = Arc::new((
            Mutex::new(Ledger {
                last_posted: None,
                pending: VecDeque::new(),
                receipt: None,
                last_ack: Instant::now(),
                fault: None,
                closing: None,
                outcome: None,
            }),
            Condvar::new(),
        ));
        let reader_shared = shared.clone();
        let reader_config = config.clone();
        let native_input = fixture.is_none();
        let reader = thread::spawn(move || {
            let mut input = BufReader::new(stdout);
            let result: io::Result<()> = (|| {
                while let Some(receipt) = read_receipt(&mut input)? {
                    let terminal = receipt.stopped;
                    let (lock, changed) = &*reader_shared;
                    lock.lock()
                        .unwrap()
                        .acknowledge(receipt, &reader_config, native_input)?;
                    changed.notify_all();
                    if terminal {
                        return Ok(());
                    }
                }
                Ok(())
            })();
            if let Err(error) = result {
                reader_shared
                    .0
                    .lock()
                    .unwrap()
                    .fault
                    .get_or_insert(short(error));
                reader_shared.1.notify_all();
            }
        });
        let monitor_shared = shared.clone();
        let monitor =
            thread::spawn(move || supervise(child, reader, monitor_shared, config, fixture));
        let writer = Self {
            input,
            shared,
            monitor: Some(monitor),
            pid,
        };
        let (lock, changed) = &*writer.shared;
        let state = lock.lock().unwrap();
        let (state, _) = changed
            .wait_timeout_while(state, STARTUP + Duration::from_secs(1), |s| {
                s.receipt.is_none() && s.outcome.is_none()
            })
            .unwrap();
        if state.receipt.as_ref().is_none_or(|r| r.stopped) || state.outcome.is_some() {
            let error = state
                .outcome
                .as_ref()
                .and_then(|o| o.error.clone())
                .or_else(|| state.receipt.as_ref().and_then(|r| r.error.clone()))
                .unwrap_or_else(|| "Windows writer did not become ready".into());
            drop(state);
            return Err(invalid(error)); // Drop closes and joins this owned writer.
        }
        drop(state);
        Ok(writer)
    }
    pub fn pid(&self) -> u32 {
        self.pid
    }
    pub fn receipt(&self) -> Option<Receipt> {
        self.shared.0.lock().unwrap().receipt.clone()
    }
    pub fn outcome(&self) -> Option<Outcome> {
        self.shared.0.lock().unwrap().outcome.clone()
    }
    pub fn send(&mut self, packet: &Packet) -> io::Result<()> {
        check_packet(packet)?;
        let bytes = json_line(packet)?;
        {
            let mut state = self.shared.0.lock().unwrap();
            if state.outcome.is_some()
                || state.fault.is_some()
                || state.closing.is_some()
                || state.receipt.as_ref().is_some_and(|r| r.stopped)
            {
                return Err(invalid(
                    "Windows writer stopped; explicit new Start required",
                ));
            }
            if let Some(seq) = sequence(packet) {
                if Some(seq) <= state.last_posted || state.pending.len() >= 64 {
                    return Err(invalid(
                        "writer sequence regressed or pending queue reached64",
                    ));
                }
                // Register before writing: a child can press then die before ACK.
                state.pending.push_back((seq, possible_press(packet)));
                state.last_posted = Some(seq);
            }
        }
        let result = self
            .input
            .as_mut()
            .ok_or_else(|| invalid("writer input closed"))?
            .write_all(&bytes);
        if let Err(error) = &result {
            self.shared
                .0
                .lock()
                .unwrap()
                .fault
                .get_or_insert(short(error));
        }
        result
    }
    /// Closing stdin requests normal EOF cleanup; a stalled child is terminated
    /// by its own supervisor before scoped possible-owned button-up recovery.
    pub fn stop(mut self) -> Outcome {
        self.close();
        self.outcome()
            .expect("joined writer has a terminal outcome")
    }
    fn close(&mut self) {
        self.input.take();
        self.shared
            .0
            .lock()
            .unwrap()
            .closing
            .get_or_insert_with(Instant::now);
        if let Some(monitor) = self.monitor.take() {
            let _ = monitor.join();
        }
    }
}
impl Drop for Writer {
    fn drop(&mut self) {
        self.close();
    }
}
fn supervise(
    mut child: Child,
    reader: JoinHandle<()>,
    shared: Shared,
    config: Config,
    fixture: Option<fixture::Config>,
) {
    let mut failure = None;
    let mut terminal_wait = None;
    let exited = loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    failure.get_or_insert_with(|| format!("writer exited: {status}"));
                }
                break true;
            }
            Err(error) => {
                failure = Some(short(error));
                match terminate(&mut child) {
                    Ok(()) => break true,
                    Err(error) => {
                        failure = Some(format!("writer termination unconfirmed: {error}"));
                        break false;
                    }
                }
            }
            Ok(None) => (),
        }
        let mut state = shared.0.lock().unwrap();
        if state.receipt.as_ref().is_some_and(|r| r.stopped) {
            terminal_wait.get_or_insert_with(Instant::now);
        }
        let overdue = if state.receipt.is_none() {
            state.last_ack.elapsed() >= STARTUP
        } else {
            state.last_ack.elapsed() >= Duration::from_micros(config.watchdog_us)
        };
        let closing = state.closing.is_some_and(|t| t.elapsed() >= CLOSE_GRACE)
            || terminal_wait.is_some_and(|t| t.elapsed() >= CLOSE_GRACE);
        if state.fault.is_some() || overdue || closing {
            failure = Some(state.fault.clone().unwrap_or_else(|| {
                if closing {
                    "writer failed to close".into()
                } else {
                    "writer receipt watchdog expired".into()
                }
            }));
            state.fault = failure.clone();
            drop(state);
            // Only this owned child. Never fallback while it could still press.
            match terminate(&mut child) {
                Ok(()) => break true,
                Err(error) => {
                    failure = Some(format!("writer termination unconfirmed: {error}"));
                    break false;
                }
            }
        }
        drop(state);
        thread::sleep(POLL);
    };
    if exited {
        let _ = reader.join();
    }
    let mut state = shared.0.lock().unwrap();
    // Do not allow producer writes after the process has exited but before cleanup.
    state.closing.get_or_insert_with(Instant::now);
    let mask = state.possible();
    let terminal = state.receipt.as_ref().is_some_and(|r| r.stopped);
    if !terminal {
        failure.get_or_insert_with(|| "writer exited without terminal cleanup receipt".into());
    }
    if let Some(error) = state.receipt.as_ref().and_then(|r| r.error.clone()) {
        failure = Some(match failure {
            Some(exit) => format!("{error}; {exit}"),
            None => error,
        });
    }
    if let Some(error) = &state.fault {
        failure.get_or_insert_with(|| error.clone());
    }
    drop(state);
    let recovered = if exited && mask != 0 {
        match fixture {
            Some(fixture) => fixture::recover(&fixture, mask),
            None => input::Injector::default().release_possible(mask),
        }
    } else {
        Ok(())
    };
    if let Err(error) = &recovered {
        failure = Some(format!(
            "{}; recovery unconfirmed: {error}",
            failure.unwrap_or_default()
        ));
    }
    let mut state = shared.0.lock().unwrap();
    state.outcome = Some(Outcome {
        process_exited: exited,
        unconfirmed_mask: if exited && recovered.is_ok() { 0 } else { mask },
        neutral: exited && recovered.is_ok(),
        fallback_used: exited && mask != 0,
        error: failure,
    });
    shared.1.notify_all();
}

#[cfg(test)]
mod tests {
    use super::super::display::{Monitor, Rect};
    use super::*;
    fn config() -> Config {
        let bounds = Rect {
            left: 0,
            top: 0,
            width: 1920,
            height: 1080,
        };
        Config {
            run_id: "fixture".into(),
            target: "fictional".into(),
            watchdog_us: 150_000,
            desktop: Desktop {
                bounds,
                monitors: vec![Monitor {
                    id: "fictional".into(),
                    bounds,
                    primary: true,
                }],
            },
        }
    }
    fn ledger() -> Ledger {
        Ledger {
            last_posted: Some(3),
            pending: VecDeque::from([(1, 0), (2, 1), (3, 2)]),
            receipt: None,
            last_ack: Instant::now(),
            fault: None,
            closing: None,
            outcome: None,
        }
    }
    fn ack(sequence: u64, mask: u8, stopped: bool) -> Receipt {
        Receipt {
            run_id: "fixture".into(),
            native_input: false,
            sequence: Some(sequence),
            held_mask: mask,
            armed: !stopped,
            stopped,
            error: None,
        }
    }
    #[test]
    fn acknowledgements_cannot_drop_a_later_pending_press() {
        let mut ledger = ledger();
        ledger
            .acknowledge(ack(1, 0, false), &config(), false)
            .unwrap();
        assert_eq!(ledger.possible(), 3);
        ledger
            .acknowledge(ack(2, 1, false), &config(), false)
            .unwrap();
        assert_eq!(ledger.possible(), 3);
        ledger
            .acknowledge(ack(3, 0, false), &config(), false)
            .unwrap();
        assert_eq!(ledger.possible(), 0);
        assert!(ledger
            .acknowledge(ack(2, 1, false), &config(), false)
            .is_err());
        assert!(ledger
            .acknowledge(ack(4, 1, false), &config(), false)
            .is_err());
        assert_eq!(ledger.possible(), 0);
    }
    #[test]
    fn terminal_receipt_retires_unprocessed_packets_but_keeps_failed_native_releases() {
        let mut ledger = ledger();
        ledger
            .acknowledge(ack(1, 1, true), &config(), false)
            .unwrap();
        assert!(ledger.pending.is_empty());
        assert_eq!(ledger.possible(), 1);
        assert!(ledger
            .acknowledge(ack(2, 0, false), &config(), false)
            .is_err());
        assert_eq!(ledger.possible(), 1);
    }
    #[test]
    fn receipts_bind_the_run_emitter_kind_and_known_button_bits() {
        for value in [
            Receipt {
                run_id: "other-run".into(),
                ..ack(1, 0, false)
            },
            Receipt {
                native_input: true,
                ..ack(1, 0, false)
            },
            ack(1, 16, false),
            Receipt {
                armed: true,
                ..ack(1, 0, true)
            },
        ] {
            let mut ledger = ledger();
            assert!(ledger.acknowledge(value, &config(), false).is_err());
            assert_eq!(ledger.possible(), 3);
        }
    }
    #[test]
    fn receipt_parser_is_bounded_and_does_not_consume_the_next_line() {
        let mut bytes = json_line(&ack(1, 0, false)).unwrap();
        bytes.extend(json_line(&ack(2, 1, false)).unwrap());
        let mut reader = io::Cursor::new(bytes);
        assert_eq!(
            read_receipt(&mut reader).unwrap().unwrap().sequence,
            Some(1)
        );
        assert_eq!(
            read_receipt(&mut reader).unwrap().unwrap().sequence,
            Some(2)
        );
        assert!(read_receipt(&mut reader).unwrap().is_none());
        assert!(read_receipt(&mut io::Cursor::new(vec![b'x'; MAX_MESSAGE_BYTES + 1])).is_err());
    }
}
