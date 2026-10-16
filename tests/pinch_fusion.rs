#![cfg(target_os = "linux")]
use wigglecontrol::{Config, Engine, Event, Hand, Observation, PinchEvidence, Point};

fn config() -> Config {
    Config {
        close_m: 0.045,
        open_m: 0.065,
        stale_us: 300_000,
        smoothing_us: 0.0,
        ..Default::default()
    }
}
fn hand(t: u64, id: u64, primary: f64, secondary: Option<(u64, f64)>) -> Hand {
    Hand {
        id,
        confidence: 0.99,
        engaged: true,
        pinch_eligible: true,
        pinch_onset_allowed: true,
        scrolling: false,
        cursor: Point {
            x: if id == 7 { 0.25 } else { 0.75 },
            y: 0.5,
        },
        thumb_tip: Point { x: 0.4, y: 0.5 },
        index_tip: Point { x: 0.42, y: 0.5 },
        palm_width: 0.2,
        pinch_distance_m: Some(primary),
        pinch_distance_ratio: None,
        secondary_pinch: secondary.map(|(source_id, distance_m)| PinchEvidence {
            source_id,
            source_us: t,
            confidence: 0.99,
            eligible: true,
            distance_m,
            onset_allowed: true,
        }),
    }
}
fn observe(e: &mut Engine, t: u64, hand: Hand) -> Vec<Event> {
    e.observe(
        t,
        Observation {
            version: 1,
            source_us: t,
            hand: Some(hand),
        },
    )
}
fn engine() -> Engine {
    let mut e = Engine::new(config()).unwrap();
    e.start();
    e
}
fn button(events: &[Event], down: bool) -> bool {
    events.contains(&Event::Button { down })
}
fn secondary_press(e: &mut Engine) {
    observe(e, 1, hand(1, 7, 0.08, Some((1, 0.08))));
    observe(e, 20_001, hand(20_001, 7, 0.08, Some((1, 0.02))));
    assert!(button(
        &observe(e, 60_001, hand(60_001, 7, 0.08, Some((1, 0.02)))),
        true
    ));
}

#[test]
fn alternating_hypotheses_cannot_share_a_confirmation_clock() {
    let mut e = engine();
    observe(&mut e, 1, hand(1, 7, 0.08, Some((1, 0.08))));
    for i in 1..12 {
        let t = i * 20_000 + 1;
        let (a, b) = if i % 2 == 0 {
            (0.08, 0.02)
        } else {
            (0.02, 0.08)
        };
        assert!(!button(
            &observe(&mut e, t, hand(t, 7, a, Some((1, b)))),
            true
        ));
    }
}

#[test]
fn overlap_handoff_preserves_one_press_and_one_release_and_primary_cursor() {
    let mut e = engine();
    observe(&mut e, 1, hand(1, 7, 0.08, Some((1, 0.08))));
    observe(&mut e, 20_001, hand(20_001, 7, 0.02, Some((1, 0.08))));
    assert!(button(
        &observe(&mut e, 60_001, hand(60_001, 7, 0.02, Some((1, 0.08)))),
        true
    ));
    observe(&mut e, 80_001, hand(80_001, 7, 0.02, Some((1, 0.02))));
    let mut moved = hand(120_001, 7, 0.08, Some((1, 0.02)));
    moved.cursor.x = 0.35;
    let events = observe(&mut e, 120_001, moved);
    assert!(!button(&events, true) && !button(&events, false));
    assert!(events
        .iter()
        .any(|e| matches!(e,Event::Move{dx,..}if *dx>0.0)));
    assert_eq!(e.pinch_state(), "pressed");
    assert!(button(
        &observe(&mut e, 140_001, hand(140_001, 7, 0.08, Some((1, 0.08)))),
        false
    ));
}

#[test]
fn new_estimator_closed_first_cannot_inherit_the_old_estimators_open_or_hold() {
    let mut e = engine();
    secondary_press(&mut e);
    assert!(button(
        &observe(&mut e, 80_001, hand(80_001, 7, 0.08, Some((2, 0.02)))),
        false
    ));
    assert!(!button(
        &observe(&mut e, 140_001, hand(140_001, 7, 0.08, Some((2, 0.02)))),
        true
    ));
    observe(&mut e, 160_001, hand(160_001, 7, 0.08, Some((2, 0.08))));
    observe(&mut e, 180_001, hand(180_001, 7, 0.08, Some((2, 0.02))));
    assert!(button(
        &observe(&mut e, 220_001, hand(220_001, 7, 0.08, Some((2, 0.02)))),
        true
    ));
}

#[test]
fn missing_weak_or_ineligible_secondary_releases_and_requires_fresh_open() {
    for loss in 0..3 {
        let mut e = engine();
        secondary_press(&mut e);
        let mut h = hand(80_001, 7, 0.08, Some((1, 0.02)));
        match loss {
            0 => h.secondary_pinch = None,
            1 => h.secondary_pinch.as_mut().unwrap().confidence = 0.6,
            _ => h.secondary_pinch.as_mut().unwrap().eligible = false,
        }
        assert!(button(&observe(&mut e, 80_001, h), false));
        for t in [100_001, 160_001] {
            assert!(!button(
                &observe(&mut e, t, hand(t, 7, 0.08, Some((1, 0.02)))),
                true
            ));
        }
    }
}

#[test]
fn wrong_frame_or_invalid_secondary_cancels_both_and_cannot_repress_closed() {
    for fault in 0..3 {
        let mut e = engine();
        secondary_press(&mut e);
        let mut h = hand(80_001, 7, 0.02, Some((1, 0.02)));
        let secondary = h.secondary_pinch.as_mut().unwrap();
        match fault {
            0 => secondary.source_us -= 1,
            1 => secondary.source_us += 1,
            _ => secondary.distance_m = f64::NAN,
        }
        assert!(button(&observe(&mut e, 80_001, h), false));
        for t in [100_001, 160_001] {
            assert!(!button(
                &observe(&mut e, t, hand(t, 7, 0.02, Some((1, 0.02)))),
                true
            ));
        }
        assert_eq!(e.pinch_state(), "open_first");
    }
}

#[test]
fn stop_and_stale_release_secondary_owned_hold_once() {
    let mut e = engine();
    secondary_press(&mut e);
    assert!(button(&e.stop(), false));
    assert!(!button(&e.stop(), false));
    let mut e = engine();
    secondary_press(&mut e);
    assert!(button(&e.tick(360_001), false));
    assert!(!button(&e.tick(360_002), false));
}

#[test]
fn four_hypotheses_still_make_two_contacts_and_keep_ids_through_handoff() {
    use wigglecontrol::touch_gestures::{Frame, Gestures, Workspace};
    let mut g = Gestures::new(config(), Workspace::default()).unwrap();
    g.start();
    let frame = |t, closed| Frame {
        version: 1,
        source_us: t,
        ambiguous: false,
        hands: vec![
            hand(
                t,
                7,
                if closed { 0.02 } else { 0.08 },
                Some((1, if closed { 0.02 } else { 0.08 })),
            ),
            hand(t, 8, 0.08, Some((1, if closed { 0.02 } else { 0.08 }))),
        ],
    };
    g.observe(1, frame(1, false));
    g.observe(20_001, frame(20_001, true));
    let contacts = g.observe(60_001, frame(60_001, true));
    assert_eq!(contacts.len(), 2);
    let mut f = frame(80_001, true);
    f.hands[0].pinch_distance_m = Some(0.08);
    f.hands.reverse();
    let held = g.observe(80_001, f);
    assert_eq!(held.len(), 2);
    assert!(held
        .iter()
        .zip(&contacts)
        .all(|(a, b)| a.id == b.id && a.x == b.x));
    assert!(g.stop().is_empty());
}
