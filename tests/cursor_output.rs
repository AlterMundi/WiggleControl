#![cfg(target_os = "linux")]
use evdev::{EventType, KeyCode};
use wigglecontrol::{
    controller::{Command, Controller, Mode, Output, Settings},
    cursor,
    delivery::{Batch, Packet, Publisher, State},
    touch_gestures::{Frame, Workspace},
    Config, Hand, Point,
};

const WATCHDOG: u64 = 300_000;
fn hand(id: u64, x: f64, closed: bool) -> Hand {
    serde_json::from_value(serde_json::json!({
        "id":id,"confidence":0.99,"engaged":true,"pinch_eligible":true,
        "cursor":{"x":x,"y":0.5},"thumb_tip":{"x":0.4,"y":0.5},
        "index_tip":{"x":0.5,"y":0.5},"palm_width":0.2,
        "pinch_distance_ratio":if closed {0.2} else {0.6}
    }))
    .unwrap()
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
    now: u64,
}
impl Pipeline {
    fn new(grace: u64) -> Self {
        Self::with_config(grace, cursor::Config::default())
    }
    fn with_config(grace: u64, config: cursor::Config) -> Self {
        let controller = Controller::new(Settings {
            cursor_output: Some(config),
            gestures: Config {
                stale_us: WATCHDOG,
                drag_grace_us: grace,
                ..Config::default()
            },
            ..Settings::default()
        })
        .unwrap();
        let mut p = Self {
            controller,
            publisher: Publisher::with_cursor_output(Some(config), WATCHDOG).unwrap(),
            delivery: State::default(),
            now: 0,
        };
        p.command(0, Command::Start, None);
        p
    }
    fn command(
        &mut self,
        now: u64,
        command: Command,
        source: Option<u64>,
    ) -> (Output, Option<Packet>, Batch) {
        self.now = now;
        let output = self.controller.command(command);
        let mut batch = self.delivery.tick(now, WATCHDOG).unwrap();
        let packet = self
            .publisher
            .absolute_packet(&output, now, source, Workspace::default())
            .unwrap();
        if let Some(packet) = &packet {
            let decoded = serde_json::from_slice(&serde_json::to_vec(packet).unwrap()).unwrap();
            let next = self.delivery.apply(now, decoded, 2000.0, WATCHDOG).unwrap();
            batch.mouse.extend(next.mouse);
            batch.touch.extend(next.touch);
        }
        (output, packet, batch)
    }
    fn until(&mut self, now: u64) {
        while self.now + 8_000 < now {
            let t = self.now + 8_000;
            self.command(t, Command::Tick { now_us: t }, None);
        }
    }
    fn frame(&mut self, source: u64, hands: Vec<Hand>) -> (Output, Option<Packet>, Batch) {
        let now = source + 65_000;
        self.until(now);
        self.command(
            now,
            Command::Frame {
                now_us: now,
                observation: Frame {
                    version: 1,
                    source_us: source,
                    ambiguous: false,
                    hands,
                },
                pose: None,
                middle_pinch: None,
                detail: None,
                escape: None,
                scroll: None,
            },
            Some(source),
        )
    }
}

#[test]
fn cursor_ticks_reconstruct_motion_without_refreshing_source_or_generating_buttons() {
    let mut p = Pipeline::new(1_000_000);
    p.frame(1, vec![hand(7, 0.3, false)]);
    p.frame(68_001, vec![hand(7, 0.34, false)]);
    let initial = p.publisher.cursor_feedback().unwrap();
    assert!(initial.estimated.x < initial.observed.x);
    let mut samples = Vec::new();
    for now in (141_001..=197_001).step_by(8_000) {
        let (_, packet, batch) = p.command(now, Command::Tick { now_us: now }, None);
        if let Some(Packet::Cursor {
            source_us,
            absolute,
            ..
        }) = packet
        {
            assert_eq!(source_us, 68_001);
            assert!(absolute.x >= initial.estimated.x);
            samples.push((now, absolute));
        }
        assert!(!button(&batch, 0) && !button(&batch, 1));
    }
    assert!(samples.len() >= 3);
    for pair in samples.windows(2) {
        assert!(
            (pair[1].1.x - pair[0].1.x).abs()
                <= 2.0 * (pair[1].0 - pair[0].0) as f64 / 1_000_000.0 + 1e-12
        );
    }
    let feedback = p.publisher.cursor_feedback().unwrap();
    assert_eq!(feedback.source_us, 68_001);
    assert_eq!(feedback.prediction_us, 0); // One direction interval is insufficient.
    assert!(feedback.estimated.x > initial.estimated.x);
    p.frame(136_001, vec![hand(7, 0.38, false)]);
    p.frame(204_001, vec![hand(7, 0.42, false)]);
    let feedback = p.publisher.cursor_feedback().unwrap();
    assert!(feedback.prediction_us > 0 && feedback.prediction_us <= 80_000);
}

#[test]
fn prediction_requires_consistent_motion_and_stops_after_an_observed_reversal() {
    let mut p = Pipeline::new(0);
    for (source, x) in [(1, 0.3), (68_001, 0.34), (136_001, 0.38), (204_001, 0.42)] {
        p.frame(source, vec![hand(7, x, false)]);
        let sample = p.publisher.cursor_feedback().unwrap();
        assert!(
            (sample.predicted.x - sample.observed.x).hypot(sample.predicted.y - sample.observed.y)
                <= 0.01 + 1e-12
        );
    }
    assert!(p.publisher.cursor_feedback().unwrap().prediction_us > 0);
    p.frame(272_001, vec![hand(7, 0.38, false)]);
    assert_eq!(p.publisher.cursor_feedback().unwrap().prediction_us, 0);
}

#[test]
fn uncertain_drag_freezes_and_recovery_is_speed_bounded_without_a_new_press() {
    let mut p = Pipeline::new(1_000_000);
    p.frame(1, vec![hand(7, 0.3, false)]);
    p.frame(68_001, vec![hand(7, 0.31, true)]);
    assert!(button(&p.frame(136_001, vec![hand(7, 0.32, true)]).2, 1));
    let (_, packet, batch) = p.frame(204_001, vec![]);
    assert!(matches!(packet, Some(Packet::Hold { .. })));
    assert!(batch.mouse.is_empty());
    let frozen = p.publisher.cursor_feedback().unwrap();
    assert!(frozen.frozen && frozen.prediction_us == 0);
    p.until(800_000);
    assert_eq!(
        p.publisher.cursor_feedback().unwrap().estimated.x,
        frozen.estimated.x
    );
    let (_, _, batch) = p.frame(816_001, vec![hand(7, 0.7, true)]);
    let resumed = p.publisher.cursor_feedback().unwrap();
    assert!(!resumed.frozen);
    assert!(resumed.estimated.x < 0.7);
    assert!((resumed.estimated.x - frozen.estimated.x).abs() <= 2.0 * 0.008 + 1e-12);
    assert!(!button(&batch, 0) && !button(&batch, 1));
    let (_, _, batch) = p.command(p.now + 1, Command::Stop, None);
    assert!(button(&batch, 0));
    assert!(!batch
        .mouse
        .iter()
        .any(|e| e.event_type() == EventType::ABSOLUTE));
    assert!(p.publisher.cursor_feedback().is_none());
}

#[test]
fn derived_motion_cannot_outlive_source_freshness_or_resume_after_stop() {
    let mut p = Pipeline::new(0);
    p.frame(1, vec![hand(7, 0.3, false)]);
    p.frame(68_001, vec![hand(7, 0.5, false)]);
    p.until(368_001);
    let (_, packet, _) = p.command(368_001, Command::Tick { now_us: 368_001 }, None);
    assert!(matches!(packet, Some(Packet::Transition { .. })));
    assert!(p.publisher.cursor_feedback().is_none());
    assert!(p
        .delivery
        .apply(
            368_002,
            Packet::Cursor {
                sequence: 999,
                issued_us: 368_002,
                source_us: 68_001,
                mode: Mode::Pointer,
                absolute: Point { x: 0.9, y: 0.9 }
            },
            2000.0,
            WATCHDOG
        )
        .is_err());
}

#[test]
fn owner_handoff_releases_at_old_point_before_initializing_new_cursor() {
    let mut p = Pipeline::new(0);
    p.frame(1, vec![hand(7, 0.3, false)]);
    p.frame(68_001, vec![hand(7, 0.3, true)]);
    assert!(button(&p.frame(136_001, vec![hand(7, 0.3, true)]).2, 1));
    let (_, packet, batch) = p.frame(204_001, vec![hand(9, 0.9, false)]);
    assert!(button(&batch, 0));
    assert!(matches!(packet, Some(Packet::Frame { absolute: None, .. })));
    assert!(!batch
        .mouse
        .iter()
        .any(|e| e.event_type() == EventType::ABSOLUTE));
    assert!(p.publisher.cursor_feedback().is_none());
    assert!(p
        .frame(272_001, vec![hand(9, 0.9, false)])
        .2
        .mouse
        .iter()
        .any(|e| e.event_type() == EventType::ABSOLUTE));
}

#[test]
fn writer_rejects_derived_motion_without_absolute_evidence_or_from_another_source() {
    for absolute in [None, Some(Point { x: 0.5, y: 0.5 })] {
        for source in [1, 2] {
            let mut state = State::default();
            state
                .apply(
                    0,
                    Packet::Transition {
                        sequence: 1,
                        issued_us: 0,
                        mode: Mode::Pointer,
                        enabled: true,
                    },
                    2000.0,
                    WATCHDOG,
                )
                .unwrap();
            state
                .apply(
                    1,
                    Packet::Frame {
                        sequence: 2,
                        issued_us: 1,
                        source_us: 1,
                        mode: Mode::Pointer,
                        absolute,
                        mouse: vec![],
                        contacts: vec![],
                    },
                    2000.0,
                    WATCHDOG,
                )
                .unwrap();
            let result = state.apply(
                3,
                Packet::Cursor {
                    sequence: 3,
                    issued_us: 3,
                    source_us: source,
                    mode: Mode::Pointer,
                    absolute: Point { x: 0.6, y: 0.5 },
                },
                2000.0,
                WATCHDOG,
            );
            assert_eq!(result.is_ok(), absolute.is_some() && source == 1);
        }
    }
}

#[test]
fn derived_motion_reserves_transport_time_before_the_source_fence() {
    let mut p = Pipeline::new(1_000_000);
    p.frame(1, vec![hand(7, 0.3, false)]);
    p.frame(68_001, vec![hand(7, 0.31, true)]);
    p.frame(136_001, vec![hand(7, 0.32, true)]);
    p.until(410_000);
    // The controller evaluated just before expiry, but publication crosses it.
    let out = p.controller.command(Command::Tick { now_us: 435_000 });
    let packet = p
        .publisher
        .absolute_packet(&out, 436_001, None, Workspace::default())
        .unwrap();
    assert!(matches!(
        &packet,
        Some(Packet::Hold {
            source_us: 136_001,
            ..
        })
    ));
    assert!(p.publisher.cursor_feedback().unwrap().frozen);
    let delivered = p
        .delivery
        .apply(437_001, packet.unwrap(), 2000.0, WATCHDOG)
        .unwrap();
    assert!(delivered.mouse.is_empty());
}

#[test]
fn click_stabilization_keeps_real_observations_and_down_up_at_the_visible_point() {
    let mut p = Pipeline::with_config(
        1_000_000,
        cursor::Config {
            click_stabilization: Some(cursor::ClickConfig::default()),
            ..cursor::Config::default()
        },
    );
    p.frame(1, vec![hand(7, 0.5, false)]);
    p.frame(68_001, vec![hand(7, 0.504, true)]);
    let candidate = p.publisher.cursor_feedback().unwrap();
    assert!(candidate.click_locked);
    assert!(candidate.observed.x > candidate.estimated.x);
    let (_, _, down) = p.frame(136_001, vec![hand(7, 0.504, true)]);
    assert!(button(&down, 1));
    assert_eq!(p.publisher.cursor_feedback().unwrap().estimated.x, 0.5);
    let (_, _, up) = p.frame(204_001, vec![hand(7, 0.501, false)]);
    assert!(button(&up, 0));
    assert!(up
        .mouse
        .iter()
        .filter(|e| e.event_type() == EventType::ABSOLUTE && e.code() == 0)
        .all(|e| e.value() == 32768));
    assert!(!p.publisher.cursor_feedback().unwrap().click_locked);
    p.until(285_000);
    assert!(p.publisher.cursor_feedback().unwrap().estimated.x > 0.5);
}

#[test]
fn click_stabilization_yields_to_drag_motion_and_expires_for_fine_motion() {
    for move_outside in [false, true] {
        let mut p = Pipeline::with_config(
            1_000_000,
            cursor::Config {
                click_stabilization: Some(cursor::ClickConfig {
                    pressed_us: 100_000,
                    ..cursor::ClickConfig::default()
                }),
                ..cursor::Config::default()
            },
        );
        p.frame(1, vec![hand(7, 0.5, false)]);
        p.frame(68_001, vec![hand(7, 0.501, true)]);
        p.frame(136_001, vec![hand(7, 0.501, true)]);
        let (_, _, batch) = p.frame(
            204_001,
            vec![hand(7, if move_outside { 0.55 } else { 0.503 }, true)],
        );
        assert!(!button(&batch, 0) && !button(&batch, 1));
        if move_outside {
            assert!(!p.publisher.cursor_feedback().unwrap().click_locked);
            assert_eq!(p.publisher.cursor_feedback().unwrap().estimated.x, 0.5);
        }
        p.until(330_000);
        assert!(!p.publisher.cursor_feedback().unwrap().click_locked);
        assert!(p.publisher.cursor_feedback().unwrap().estimated.x > 0.5);
    }
}

#[test]
fn unconfirmed_pinch_cannot_lock_hover_indefinitely_and_stop_clears_click_state() {
    let mut p = Pipeline::with_config(
        1_000_000,
        cursor::Config {
            click_stabilization: Some(cursor::ClickConfig {
                closing_us: 80_000,
                ..cursor::ClickConfig::default()
            }),
            ..cursor::Config::default()
        },
    );
    p.frame(1, vec![hand(7, 0.5, false)]);
    let mut veto = hand(7, 0.502, true);
    veto.pinch_onset_allowed = false;
    p.frame(68_001, vec![veto.clone()]);
    assert!(p.publisher.cursor_feedback().unwrap().click_locked);
    p.frame(136_001, vec![veto.clone()]);
    p.until(220_000);
    p.frame(204_001, vec![veto]);
    assert!(!p.publisher.cursor_feedback().unwrap().click_locked);
    let (out, _, batch) = p.command(p.now + 1, Command::Stop, None);
    assert!(!out.button_down && !button(&batch, 1));
    assert!(p.publisher.cursor_feedback().is_none());
}
