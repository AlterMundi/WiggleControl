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
            cursor: Point { x, y: 0.5 },
            thumb_tip: Point { x: 0.4, y: 0.5 },
            index_tip: Point {
                x: 0.4 + ratio * 0.2,
                y: 0.5,
            },
            palm_width: 0.2,
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
    assert!(!e
        .observe(20_001, frame(20_001, 1, 0.1, 0.5))
        .contains(&Event::Button { down: true }));
    assert!(e
        .observe(60_001, frame(60_001, 1, 0.1, 0.5))
        .contains(&Event::Button { down: true }));
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
