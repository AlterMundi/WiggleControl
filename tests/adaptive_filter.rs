use wigglecontrol::{filter::OneEuroConfig, Config, Engine, Event, Hand, Observation, Point};

fn config() -> Config {
    Config {
        adaptive_smoothing: Some(OneEuroConfig {
            min_cutoff_hz: 1.0,
            beta: 10.0,
            derivative_cutoff_hz: 1.0,
            speed_floor: 0.0,
        }),
        ..Config::default()
    }
}
fn observation(t: u64, id: u64, x: f64, closed: bool) -> Observation {
    Observation {
        version: 1,
        source_us: t,
        hand: Some(Hand {
            id,
            confidence: 0.99,
            engaged: true,
            pinch_eligible: true,
            pinch_onset_allowed: true,
            scrolling: false,
            cursor: Point { x, y: 0.5 },
            thumb_tip: Point { x: 0.4, y: 0.5 },
            index_tip: Point {
                x: if closed { 0.42 } else { 0.52 },
                y: 0.5,
            },
            palm_width: 0.2,
            pinch_distance_m: None,
            pinch_distance_ratio: None,
            secondary_pinch: None,
        }),
    }
}
#[test]
fn adaptive_filter_balances_rest_noise_and_motion_without_changing_press_release() {
    let mut adaptive = Engine::new(config()).unwrap();
    let mut fixed = Engine::new(Config {
        smoothing_us: 100_000.0,
        ..Config::default()
    })
    .unwrap();
    adaptive.start();
    fixed.start();
    let mut raw_energy = 0.0;
    let mut filtered_energy = 0.0;
    for i in 0..40 {
        let t = 1 + i * 50_000;
        let x = 0.5 + if i % 2 == 0 { 0.008 } else { -0.008 };
        let closed = (20..30).contains(&i);
        assert_eq!(
            adaptive
                .observe(t, observation(t, 1, x, closed))
                .iter()
                .filter(|e| matches!(e, Event::Button { .. }))
                .collect::<Vec<_>>(),
            fixed
                .observe(t, observation(t, 1, x, closed))
                .iter()
                .filter(|e| matches!(e, Event::Button { .. }))
                .collect::<Vec<_>>()
        );
        if i >= 10 {
            raw_energy += (x - 0.5).powi(2);
            filtered_energy += (adaptive.cursor_position().unwrap().x - 0.5).powi(2);
        }
    }
    assert!(filtered_energy < raw_energy * 0.25);
    // Compare identical fast movement after both filters have settled.
    let mut a_error = 0.0;
    let mut f_error = 0.0;
    for i in 40..50 {
        let t = 1 + i * 50_000;
        let x = 0.5 + (i - 39) as f64 * 0.03;
        adaptive.observe(t, observation(t, 1, x, false));
        fixed.observe(t, observation(t, 1, x, false));
        a_error += x - adaptive.cursor_position().unwrap().x;
        f_error += x - fixed.cursor_position().unwrap().x;
    }
    assert!(a_error < f_error);
}
#[test]
fn loss_identity_change_and_scroll_transition_reset_derivative_and_never_jump_or_press() {
    let mut engine = Engine::new(config()).unwrap();
    engine.start();
    for (t, x, closed) in [(1, 0.2, false), (50_001, 0.3, true), (100_001, 0.4, true)] {
        engine.observe(t, observation(t, 1, x, closed));
    }
    assert_eq!(engine.pinch_state(), "pressed");
    let events = engine.observe(150_001, observation(150_001, 2, 0.9, true));
    assert!(events.contains(&Event::Button { down: false }));
    assert!(!events
        .iter()
        .any(|e| matches!(e, Event::Move { .. } | Event::Button { down: true })));
    assert_eq!(engine.cursor_position().unwrap().x, 0.9);
    engine.observe(
        200_001,
        Observation {
            version: 1,
            source_us: 200_001,
            hand: None,
        },
    );
    assert!(engine
        .observe(250_001, observation(250_001, 2, 0.1, true))
        .is_empty());
    assert_eq!(engine.cursor_position().unwrap().x, 0.1);
    let mut scroll = observation(300_001, 2, 0.8, false);
    scroll.hand.as_mut().unwrap().scrolling = true;
    assert!(engine.observe(300_001, scroll).is_empty());
    assert_eq!(engine.cursor_position().unwrap().x, 0.8);
}
#[test]
fn invalid_nonfinite_or_extreme_filter_settings_are_rejected() {
    for value in [f64::NAN, f64::INFINITY, -1.0, 1001.0] {
        let mut c = config();
        c.adaptive_smoothing.as_mut().unwrap().beta = value;
        assert!(Engine::new(c).is_err());
    }
    for value in [f64::NAN, f64::INFINITY, -0.1, 1.1] {
        let mut c = config();
        c.adaptive_smoothing.as_mut().unwrap().speed_floor = value;
        assert!(Engine::new(c).is_err());
    }
}

#[test]
fn adaptive_speed_floor_never_becomes_a_spatial_dead_zone() {
    let mut c = config();
    c.adaptive_smoothing = Some(OneEuroConfig {
        min_cutoff_hz: 0.3,
        beta: 40.0,
        derivative_cutoff_hz: 1.0,
        speed_floor: 0.05,
    });
    let mut engine = Engine::new(c).unwrap();
    engine.start();
    engine.observe(1, observation(1, 7, 0.5, false));
    for n in 1..21 {
        let t = 1 + n * 50_000;
        engine.observe(t, observation(t, 7, 0.5 + n as f64 * 0.0001, false));
    }
    let p = engine.cursor_position().unwrap();
    assert!(p.x > 0.501 && p.x < 0.502);
}
