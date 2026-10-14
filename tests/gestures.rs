use wigglecontrol::{Config, Engine, Event, Hand, Observation, Point};
fn frame(t: u64, id: u64, ratio: f64, x: f64) -> Observation {
    Observation {
        version: 1,
        source_us: t,
        hand: Some(Hand {
            id,
            confidence: 0.99,
            engaged: true,
            pinch_eligible: true,
            scrolling: false,
            cursor: Point { x, y: 0.5 },
            thumb_tip: Point { x: 0.4, y: 0.5 },
            index_tip: Point {
                x: 0.4 + ratio * 0.2,
                y: 0.5,
            },
            palm_width: 0.2,
            pinch_distance_m: None,
        }),
    }
}
fn engine() -> Engine {
    let mut e = Engine::new(Config::default()).unwrap();
    e.start();
    e
}
fn press(e: &mut Engine) {
    e.observe(1, frame(1, 1, 0.6, 0.5));
    assert_eq!(e.pinch_state(), "open");
    assert!(!e
        .observe(20_001, frame(20_001, 1, 0.1, 0.5))
        .contains(&Event::Button { down: true }));
    assert_eq!(e.pinch_state(), "closing");
    assert!(e
        .observe(60_001, frame(60_001, 1, 0.1, 0.5))
        .contains(&Event::Button { down: true }));
    assert_eq!(e.pinch_state(), "pressed");
}
#[test]
fn click_drag_hysteresis_release() {
    let mut e = engine();
    press(&mut e);
    let drag = e.observe(80_001, frame(80_001, 1, 0.3, 0.8));
    assert!(drag
        .iter()
        .any(|v| matches!(v, Event::Move { dx, .. } if *dx > 0.0)));
    assert!(!drag.contains(&Event::Button { down: false }));
    assert!(e
        .observe(100_001, frame(100_001, 1, 0.6, 0.8))
        .contains(&Event::Button { down: false }));
    assert!(!e.stop().contains(&Event::Button { down: false }));
    assert_eq!(e.pinch_state(), "stopped");
}
#[test]
fn closed_reacquisition_does_not_click() {
    let mut e = engine();
    press(&mut e);
    assert!(e
        .observe(
            70_001,
            Observation {
                version: 1,
                source_us: 70_001,
                hand: None
            }
        )
        .contains(&Event::Button { down: false }));
    for t in [80_001, 130_001, 180_001] {
        assert!(!e
            .observe(t, frame(t, 1, 0.1, 0.7))
            .contains(&Event::Button { down: true }));
    }
}
#[test]
fn hand_replacement_releases_and_recenters() {
    let mut e = engine();
    press(&mut e);
    let events = e.observe(80_001, frame(80_001, 2, 0.1, 0.9));
    assert!(events.contains(&Event::Button { down: false }));
    assert!(!events
        .iter()
        .any(|v| matches!(v, Event::Move { .. } | Event::Button { down: true })));
}
#[test]
fn loss_staleness_stop_and_ineligibility_release() {
    for cause in 0..6 {
        let mut e = engine();
        press(&mut e);
        let events = match cause {
            0 => e.tick(210_001),
            1 => e.stop(),
            2 => e.reject(),
            _ => {
                let mut f = frame(80_001, 1, 0.1, 0.5);
                let h = f.hand.as_mut().unwrap();
                match cause {
                    3 => h.engaged = false,
                    4 => h.confidence = 0.1,
                    _ => h.pinch_eligible = false,
                }
                e.observe(80_001, f)
            }
        };
        assert!(
            events.contains(&Event::Button { down: false }),
            "cause {cause}"
        );
        assert!(!e.stop().contains(&Event::Button { down: false }));
    }
}
#[test]
fn malformed_future_old_and_duplicate_frames_release() {
    for cause in 0..7 {
        let mut e = engine();
        press(&mut e);
        let mut f = frame(80_001, 1, 0.1, 0.5);
        match cause {
            0 => f.version = 2,
            1 => f.source_us = 90_000,
            2 => f.source_us = 60_001,
            3 => f.hand.as_mut().unwrap().cursor.x = f64::NAN,
            4 => f.hand.as_mut().unwrap().palm_width = 0.0,
            5 => f.hand.as_mut().unwrap().index_tip.y = 1.1,
            _ => f.source_us = 1,
        }
        assert!(
            e.observe(80_001, f)
                .contains(&Event::Button { down: false }),
            "cause {cause}"
        );
    }
}
#[test]
fn consumer_clock_regression_cannot_repress() {
    let mut e = engine();
    press(&mut e);
    let events = e.observe(50_001, frame(50_001, 1, 0.1, 0.5));
    assert!(events.contains(&Event::Button { down: false }));
    assert!(!events.contains(&Event::Button { down: true }));
}
#[test]
fn jitter_is_smoothed_and_pinches_require_confirmation() {
    let mut e = engine();
    e.observe(1, frame(1, 1, 0.6, 0.5));
    let events = e.observe(10_001, frame(10_001, 1, 0.1, 0.51));
    assert!(events
        .iter()
        .any(|v| matches!(v, Event::Move { dx, .. } if *dx > 0.0 && *dx < 0.004)));
    e.observe(20_001, frame(20_001, 1, 0.3, 0.5));
    assert!(!e
        .observe(50_001, frame(50_001, 1, 0.1, 0.5))
        .contains(&Event::Button { down: true }));
}
#[test]
fn stopped_engine_ignores_observations_and_invalid_config_is_rejected() {
    let mut e = Engine::new(Config::default()).unwrap();
    assert!(e.observe(1, frame(1, 1, 0.6, 0.5)).is_empty());
    assert!(Engine::new(Config {
        close_ratio: 0.5,
        open_ratio: 0.4,
        ..Config::default()
    })
    .is_err());
}

#[test]
fn fresh_delayed_frames_preserve_drag_but_stale_frames_do_not() {
    let mut e = engine();
    e.observe(80_001, frame(1, 1, 0.6, 0.5));
    e.observe(100_001, frame(20_001, 1, 0.1, 0.5));
    assert!(e
        .observe(140_001, frame(60_001, 1, 0.1, 0.5))
        .contains(&Event::Button { down: true }));
    let fresh = e.observe(240_001, frame(160_001, 1, 0.1, 0.7));
    assert!(!fresh.contains(&Event::Button { down: false }));
    assert!(e
        .observe(400_001, frame(200_001, 1, 0.1, 0.7))
        .contains(&Event::Button { down: false }));
}

#[test]
fn scroll_releases_drag_freezes_pointer_and_resets_baseline() {
    let mut e = engine();
    press(&mut e);
    let mut first = frame(80_001, 1, 0.6, 0.5);
    first.hand.as_mut().unwrap().scrolling = true;
    assert!(e
        .observe(80_001, first)
        .contains(&Event::Button { down: false }));
    let mut next = frame(180_001, 1, 0.6, 0.5);
    let hand = next.hand.as_mut().unwrap();
    hand.scrolling = true;
    hand.cursor.y = 0.2;
    let events = e.observe(180_001, next);
    assert!(events
        .iter()
        .any(|e| matches!(e,Event::Scroll{vertical} if *vertical>0)));
    assert!(!events.iter().any(|e| matches!(e, Event::Move { .. })));
    let exited = e.observe(200_001, frame(200_001, 1, 0.6, 0.5));
    assert!(!exited
        .iter()
        .any(|e| matches!(e, Event::Move { .. } | Event::Scroll { .. })));
}

#[test]
fn wider_pinch_tolerance_keeps_confirmation_hysteresis_and_open_before_press() {
    let mut e = Engine::new(Config {
        close_ratio: 0.35,
        open_ratio: 0.5,
        ..Config::default()
    })
    .unwrap();
    e.start();
    // A closed initial hand never creates a click.
    assert!(!e
        .observe(1, frame(1, 1, 0.3, 0.5))
        .contains(&Event::Button { down: true }));
    assert_eq!(e.pinch_state(), "open_first");
    e.observe(10_001, frame(10_001, 1, 0.6, 0.5));
    assert!(!e
        .observe(20_001, frame(20_001, 1, 0.3, 0.5))
        .contains(&Event::Button { down: true }));
    assert!(e
        .observe(60_001, frame(60_001, 1, 0.3, 0.5))
        .contains(&Event::Button { down: true }));
    assert!(!e
        .observe(80_001, frame(80_001, 1, 0.45, 0.5))
        .contains(&Event::Button { down: false }));
    assert!(e
        .observe(100_001, frame(100_001, 1, 0.6, 0.5))
        .contains(&Event::Button { down: false }));
    assert_eq!(e.pinch_state(), "open");
}

#[test]
fn diagnostics_distinguish_open_first_confirmation_and_stale_frame() {
    let mut e = engine();
    e.observe(1, frame(1, 1, 0.1, 0.5));
    assert_eq!(e.pinch_block_reason(), Some("open_before_press"));
    e.observe(10_001, frame(10_001, 1, 0.6, 0.5));
    e.observe(20_001, frame(20_001, 1, 0.1, 0.5));
    assert_eq!(e.pinch_block_reason(), Some("confirming"));
    assert_eq!(e.confirmation_elapsed_us(), 0);
    e.observe(40_001, frame(40_001, 1, 0.1, 0.5));
    assert_eq!(e.confirmation_elapsed_us(), 20_000);
    e.observe(60_001, frame(60_001, 1, 0.1, 0.5));
    assert_eq!(e.pinch_block_reason(), None);
    e.observe(250_001, frame(60_001, 1, 0.1, 0.5));
    assert_eq!(e.pinch_block_reason(), Some("stale_frame"));
}

#[test]
fn relaxed_release_uses_finger_separation_without_full_hand_reopening() {
    let mut e = Engine::new(Config {
        close_ratio: 0.35,
        open_ratio: 0.45,
        ..Config::default()
    })
    .unwrap();
    e.start();
    e.observe(1, frame(1, 1, 0.6, 0.5));
    e.observe(20_001, frame(20_001, 1, 0.3, 0.5));
    assert!(e
        .observe(60_001, frame(60_001, 1, 0.3, 0.5))
        .contains(&Event::Button { down: true }));
    assert!(e
        .observe(80_001, frame(80_001, 1, 0.46, 0.5))
        .contains(&Event::Button { down: false }));
    assert_eq!(e.pinch_state(), "open");
}

#[test]
fn selected_300ms_window_preserves_delayed_hold_and_releases_at_deadline() {
    let mut e = Engine::new(Config {
        stale_us: 300_000,
        ..Config::default()
    })
    .unwrap();
    e.start();
    e.observe(200_001, frame(1, 1, 0.6, 0.5));
    e.observe(240_001, frame(40_001, 1, 0.1, 0.5));
    assert!(e
        .observe(280_001, frame(80_001, 1, 0.1, 0.5))
        .contains(&Event::Button { down: true }));
    assert!(!e
        .observe(480_001, frame(280_001, 1, 0.1, 0.6))
        .contains(&Event::Button { down: false }));
    assert!(!e.tick(780_000).contains(&Event::Button { down: false }));
    assert!(e.tick(780_001).contains(&Event::Button { down: false }));
    assert!(!e
        .observe(800_001, frame(700_001, 1, 0.1, 0.6))
        .contains(&Event::Button { down: true }));
}

#[test]
fn metric_pinch_uses_depth_distance_not_projected_palm_and_preserves_hold_hysteresis() {
    let mut e = engine();
    let metric = |t, d| {
        let mut f = frame(t, 1, 0.1, 0.5);
        let h = f.hand.as_mut().unwrap();
        h.pinch_distance_m = Some(d);
        h.palm_width = 0.001;
        f
    };
    e.observe(1, metric(1, 0.04));
    e.observe(50_001, metric(50_001, 0.01));
    assert!(e
        .observe(100_001, metric(100_001, 0.01))
        .contains(&Event::Button { down: true }));
    assert!(!e
        .observe(150_001, metric(150_001, 0.025))
        .contains(&Event::Button { down: false }));
    assert!(e
        .observe(200_001, metric(200_001, 0.031))
        .contains(&Event::Button { down: false }));
    let mut invalid = metric(250_001, 0.01);
    invalid.hand.as_mut().unwrap().pinch_distance_m = Some(f64::NAN);
    e.observe(250_001, invalid);
    assert_eq!(e.pinch_block_reason(), Some("invalid_hand"));
}
