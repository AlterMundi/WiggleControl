#![cfg(target_os = "linux")]
use evdev::{EventType, KeyCode};
use wigglecontrol::{
    controller::{Command, Controller, Mode, Output, Settings},
    delivery::{Batch, Packet, Publisher, State},
    touch_gestures::{Frame, Workspace},
    Config, Hand, Point,
};

const WATCHDOG: u64 = 300_000;

fn hand(id: u64, closed: bool) -> Hand {
    serde_json::from_value(serde_json::json!({
        "id":id,"confidence":0.99,"engaged":true,"pinch_eligible":true,
        "cursor":{"x":0.5,"y":0.5},"thumb_tip":{"x":0.4,"y":0.5},
        "index_tip":{"x":0.5,"y":0.5},"palm_width":0.2,
        "pinch_distance_ratio":if closed {0.2} else {0.6}
    }))
    .unwrap()
}
fn frame(t: u64, hands: Vec<Hand>) -> Command {
    Command::Frame {
        now_us: t,
        observation: Frame {
            version: 1,
            source_us: t,
            ambiguous: false,
            hands,
        },
        pose: None,
        middle_pinch: None,
    }
}
fn button(batch: &Batch, value: i32) -> bool {
    batch.mouse.iter().any(|e| {
        e.event_type() == EventType::KEY && e.code() == KeyCode::BTN_LEFT.0 && e.value() == value
    })
}
struct Pipeline {
    controller: Controller,
    publisher: Publisher,
    delivery: State,
}
impl Pipeline {
    fn new() -> Self {
        Self::with_release(None)
    }
    fn with_release(release_confirmation: Option<wigglecontrol::ReleaseConfig>) -> Self {
        let controller = Controller::new(Settings {
            gestures: Config {
                release_confirmation,
                stale_us: WATCHDOG,
                drag_grace_us: 1_000_000,
                ..Config::default()
            },
            ..Settings::default()
        })
        .unwrap();
        let mut this = Self {
            controller,
            publisher: Publisher::default(),
            delivery: State::default(),
        };
        this.step(0, Command::Start);
        this.step(1, frame(1, vec![hand(7, false)]));
        this.step(20_001, frame(20_001, vec![hand(7, true)]));
        assert!(button(
            &this.step(60_001, frame(60_001, vec![hand(7, true)])).2,
            1
        ));
        this
    }
    fn step(&mut self, now: u64, command: Command) -> (Output, Option<Packet>, Batch) {
        let source = match &command {
            Command::Frame { observation, .. } => Some(observation.source_us),
            _ => None,
        };
        let output = self.controller.command(command);
        let mut batch = self.delivery.tick(now, WATCHDOG).unwrap();
        let packet = self
            .publisher
            .absolute_packet(&output, now, source, Workspace::default())
            .unwrap();
        if let Some(packet) = &packet {
            // Use the actual serde wire boundary too, not shared mutable state.
            let decoded = serde_json::from_slice(&serde_json::to_vec(packet).unwrap()).unwrap();
            let next = self.delivery.apply(now, decoded, 2000.0, WATCHDOG).unwrap();
            batch.mouse.extend(next.mouse);
            batch.touch.extend(next.touch);
        }
        (output, packet, batch)
    }
}

#[test]
fn pending_weak_release_remains_held_through_the_wire_but_never_blocks_stop_or_watchdog() {
    let config = || {
        Some(wigglecontrol::ReleaseConfig {
            confirm_us: 40_000,
            immediate_open_factor: 1.25,
        })
    };
    let weak = || {
        let mut h = hand(7, true);
        h.pinch_distance_ratio = Some(0.42);
        h
    };
    let mut p = Pipeline::with_release(config());
    let (out, packet, batch) = p.step(100_001, frame(100_001, vec![weak()]));
    assert!(out.button_down && out.release_pending.is_some());
    assert!(matches!(
        packet,
        Some(Packet::Frame {
            source_us: 100_001,
            ..
        })
    ));
    assert!(!button(&batch, 0));
    let (out, _, batch) = p.step(168_001, frame(168_001, vec![hand(7, true)]));
    assert!(out.button_down && out.release_pending.is_none());
    assert!(!button(&batch, 0) && !button(&batch, 1));
    p.step(236_001, frame(236_001, vec![weak()]));
    let (out, _, batch) = p.step(304_001, frame(304_001, vec![weak()]));
    assert!(!out.button_down && out.release_pending.is_none());
    assert!(button(&batch, 0));
    for stop in [false, true] {
        let mut p = Pipeline::with_release(config());
        p.step(100_001, frame(100_001, vec![weak()]));
        if stop {
            let (out, _, batch) = p.step(100_002, Command::Stop);
            assert!(button(&batch, 0) && out.release_pending.is_none());
        } else {
            assert!(button(&p.delivery.tick(400_001, WATCHDOG).unwrap(), 0));
        }
    }
}

#[test]
fn missing_samples_freeze_motion_and_same_owner_returns_without_releasing_or_repressing() {
    let mut p = Pipeline::new();
    let (out, packet, batch) = p.step(100_001, frame(100_001, vec![]));
    assert!(out.button_down && out.drag_hold.unwrap().in_grace);
    assert!(batch.mouse.is_empty());
    assert!(matches!(
        packet,
        Some(Packet::Hold {
            source_us: 60_001,
            until_us: 1_060_001,
            ..
        })
    ));
    for now in (150_001..=850_001).step_by(50_000) {
        let (out, packet, batch) = p.step(now, Command::Tick { now_us: now });
        assert!(out.button_down);
        assert!(matches!(
            packet,
            Some(Packet::Hold {
                source_us: 60_001,
                until_us: 1_060_001,
                ..
            })
        ));
        assert!(batch.mouse.is_empty());
    }
    let (out, _, batch) = p.step(900_001, frame(900_001, vec![hand(7, true)]));
    assert!(out.button_down && !out.drag_hold.unwrap().in_grace);
    assert!(!button(&batch, 0) && !button(&batch, 1));
    let (out, _, batch) = p.step(950_001, frame(950_001, vec![hand(7, false)]));
    assert!(!out.button_down);
    assert!(button(&batch, 0)); // Measured opening wins immediately over grace.
}

#[test]
fn absence_reports_cannot_extend_deadline_and_closed_return_after_expiry_cannot_repress() {
    let mut p = Pipeline::new();
    for now in (100_001..=1_000_001).step_by(50_000) {
        let (out, _, batch) = p.step(now, frame(now, vec![]));
        assert!(out.button_down);
        assert!(!button(&batch, 0));
    }
    let (out, _, batch) = p.step(1_060_001, Command::Tick { now_us: 1_060_001 });
    assert!(out.transition && !out.button_down);
    assert!(button(&batch, 0));
    for now in [1_100_001, 1_150_001] {
        let (out, _, batch) = p.step(now, frame(now, vec![hand(7, true)]));
        assert!(!out.button_down && !out.pointer_ready);
        assert!(!button(&batch, 1));
    }
}

#[test]
fn no_sample_gap_keeps_truthful_source_but_writer_process_watchdog_still_releases() {
    let mut p = Pipeline::new();
    for now in (100_001..=600_001).step_by(50_000) {
        let (out, packet, batch) = p.step(now, Command::Tick { now_us: now });
        assert!(out.button_down);
        assert!(matches!(
            packet,
            Some(Packet::Hold {
                source_us: 60_001,
                ..
            })
        ));
        assert!(!button(&batch, 0));
    }
    // Producer dies: no more controller/hold packets, although grace is not over.
    assert!(button(&p.delivery.tick(900_001, WATCHDOG).unwrap(), 0));
    let forged = Packet::Hold {
        sequence: 999,
        issued_us: 900_002,
        source_us: 60_001,
        until_us: 1_060_001,
        mode: Mode::Pointer,
    };
    assert!(p.delivery.apply(900_002, forged, 2000.0, WATCHDOG).is_err());
}

#[test]
fn changed_owner_disengagement_ambiguity_stop_and_mode_change_release_without_motion() {
    for case in 0..5 {
        let mut p = Pipeline::new();
        p.step(100_001, frame(100_001, vec![]));
        let mut disengaged = hand(7, true);
        disengaged.engaged = false;
        let command = match case {
            0 => frame(150_001, vec![hand(9, false)]),
            1 => frame(150_001, vec![disengaged]),
            2 => Command::Frame {
                now_us: 150_001,
                observation: Frame {
                    version: 1,
                    source_us: 150_001,
                    ambiguous: true,
                    hands: vec![],
                },
                pose: None,
                middle_pinch: None,
            },
            3 => Command::Stop,
            _ => Command::Mode { mode: Mode::Scroll },
        };
        let (out, _, batch) = p.step(150_001, command);
        assert!(!out.button_down, "case {case}");
        assert!(button(&batch, 0), "case {case}");
        assert!(!batch
            .mouse
            .iter()
            .any(|e| e.event_type() == EventType::ABSOLUTE));
    }
}

#[test]
fn ineligible_same_owner_is_unknown_but_invalid_frames_are_not_new_evidence() {
    let mut p = Pipeline::new();
    let mut uncertain = hand(7, true);
    uncertain.pinch_eligible = false;
    uncertain.cursor = Point { x: 0.9, y: 0.1 };
    let (out, _, batch) = p.step(100_001, frame(100_001, vec![uncertain]));
    assert!(out.button_down && out.drag_hold.unwrap().in_grace);
    assert_eq!(out.pointer_cursor.unwrap().x, 0.5);
    assert!(batch.mouse.is_empty());
    let (out, _, batch) = p.step(150_001, frame(90_001, vec![hand(7, true)]));
    assert!(!out.button_down);
    assert!(button(&batch, 0));
}

#[test]
fn writer_rejects_source_substitution_deadline_extension_and_holds_without_existing_press() {
    for case in 0..4 {
        let mut p = Pipeline::new();
        p.step(100_001, Command::Tick { now_us: 100_001 });
        let (source_us, until_us) = match case {
            0 => (60_000, 1_060_001),
            1 => (60_001, 1_060_002),
            2 => (60_001, 2_060_002),
            _ => (60_001, 150_001),
        };
        let packet = Packet::Hold {
            sequence: 999,
            issued_us: 150_001,
            source_us,
            until_us,
            mode: Mode::Pointer,
        };
        assert!(p.delivery.apply(150_001, packet, 2000.0, WATCHDOG).is_err());
        assert!(button(&p.delivery.release(), 0));
    }
    let mut state = State::default();
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
            WATCHDOG,
        )
        .unwrap();
    assert!(state
        .apply(
            2,
            Packet::Hold {
                sequence: 2,
                issued_us: 2,
                source_us: 1,
                until_us: 1_000_001,
                mode: Mode::Pointer
            },
            2000.0,
            WATCHDOG
        )
        .is_err());
}

#[test]
fn writer_fixed_deadline_releases_even_when_heartbeat_stays_alive() {
    let mut p = Pipeline::new();
    for now in (100_001..=1_050_001).step_by(50_000) {
        p.step(now, Command::Tick { now_us: now });
    }
    assert!(button(&p.delivery.tick(1_060_001, WATCHDOG).unwrap(), 0));
}

#[test]
fn drag_grace_cannot_keep_touch_contacts_or_an_unestablished_mouse_press_alive() {
    let settings = || Settings {
        gestures: Config {
            stale_us: WATCHDOG,
            drag_grace_us: 1_000_000,
            release_confirmation: Some(wigglecontrol::ReleaseConfig {
                confirm_us: 40_000,
                immediate_open_factor: 1.25,
            }),
            ..Config::default()
        },
        ..Settings::default()
    };
    let mut pointer = Controller::new(settings()).unwrap();
    pointer.start();
    pointer.command(frame(1, vec![hand(7, false)]));
    pointer.command(frame(20_001, vec![hand(7, true)])); // Not yet confirmed.
    assert!(pointer.command(frame(30_001, vec![])).drag_hold.is_none());
    for now in [60_001, 100_001] {
        assert!(!pointer.command(frame(now, vec![hand(7, true)])).button_down);
    }
    let mut touch = Controller::new(Settings {
        initial_mode: Mode::OneContact,
        ..settings()
    })
    .unwrap();
    touch.start();
    touch.command(frame(1, vec![hand(7, false)]));
    touch.command(frame(20_001, vec![hand(7, true)]));
    assert_eq!(
        touch
            .command(frame(60_001, vec![hand(7, true)]))
            .contacts
            .len(),
        1
    );
    let mut weak_open = hand(7, false);
    weak_open.pinch_distance_ratio = Some(0.42);
    assert!(touch
        .command(frame(100_001, vec![weak_open]))
        .contacts
        .is_empty());
    touch.command(frame(120_001, vec![hand(7, true)]));
    assert_eq!(
        touch
            .command(frame(160_001, vec![hand(7, true)]))
            .contacts
            .len(),
        1
    );
    assert!(touch.command(frame(200_001, vec![])).contacts.is_empty());
}
