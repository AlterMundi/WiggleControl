#![cfg(target_os = "linux")]
use wigglecontrol::{
    touch_gestures::{Frame, Gestures, Workspace},
    Config, Hand, Point,
};
fn hand(id: u64, x: f64, closed: bool) -> Hand {
    Hand {
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
        secondary_pinch: None,
    }
}
fn frame(t: u64, hands: Vec<Hand>) -> Frame {
    Frame {
        version: 1,
        source_us: t,
        ambiguous: false,
        hands,
    }
}
fn engine() -> Gestures {
    let mut g = Gestures::new(Config::default(), Workspace::default()).unwrap();
    g.start();
    g
}

#[test]
fn onset_veto_is_independent_per_contact_and_loss_still_requires_open() {
    let mut e = engine();
    e.observe(1, frame(1, vec![hand(7, 0.3, false), hand(8, 0.7, false)]));
    let closed = |veto: bool| {
        let mut first = hand(7, 0.3, true);
        first.pinch_onset_allowed = !veto;
        vec![first, hand(8, 0.7, true)]
    };
    e.observe(20_001, frame(20_001, closed(true)));
    let first = e.observe(60_001, frame(60_001, closed(true)));
    assert_eq!(first.len(), 1);
    assert!(first[0].x > 0.5);
    let both = e.observe(70_001, frame(70_001, closed(false)));
    assert_eq!(both.len(), 2);
    assert!(both.iter().any(|p| p.id == first[0].id));
    assert_eq!(e.observe(80_001, frame(80_001, closed(true))).len(), 2);
    assert!(e.observe(90_001, frame(90_001, vec![])).is_empty());
    assert!(e.observe(100_001, frame(100_001, closed(false))).is_empty());
    assert!(e.observe(150_001, frame(150_001, closed(false))).is_empty());
    assert_eq!(e.awaiting_open_hands(), vec![7, 8]);
}
fn press(g: &mut Gestures) -> Vec<wigglecontrol::touch::Contact> {
    assert!(g
        .observe(1, frame(1, vec![hand(7, 0.3, false), hand(9, 0.7, false)]))
        .is_empty());
    assert!(g
        .observe(
            20_001,
            frame(20_001, vec![hand(7, 0.3, true), hand(9, 0.7, true)])
        )
        .is_empty());
    g.observe(
        60_001,
        frame(60_001, vec![hand(7, 0.3, true), hand(9, 0.7, true)]),
    )
}
#[test]
fn independent_contacts_keep_ownership_on_reorder_and_single_loss() {
    let mut g = engine();
    let initial = press(&mut g);
    assert_eq!(initial.len(), 2);
    let moved = g.observe(
        80_001,
        frame(80_001, vec![hand(9, 0.8, true), hand(7, 0.2, true)]),
    );
    assert_eq!(
        initial.iter().map(|p| p.id).collect::<Vec<_>>(),
        moved.iter().map(|p| p.id).collect::<Vec<_>>()
    );
    assert!(moved[0].x < initial[0].x && moved[1].x > initial[1].x);
    let remaining = g.observe(100_001, frame(100_001, vec![hand(9, 0.8, true)]));
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].id, initial[1].id);
    assert_eq!(
        g.observe(
            120_001,
            frame(120_001, vec![hand(7, 0.2, true), hand(9, 0.8, true)])
        )
        .len(),
        1
    );
}
#[test]
fn ambiguous_crossing_releases_and_requires_open_before_repress() {
    let mut g = engine();
    let initial = press(&mut g);
    let mut uncertain = frame(80_001, vec![hand(7, 0.5, true), hand(9, 0.5, true)]);
    uncertain.ambiguous = true;
    assert!(g.observe(80_001, uncertain).is_empty());
    for t in [100_001, 160_001] {
        assert!(g
            .observe(t, frame(t, vec![hand(7, 0.3, true), hand(9, 0.7, true)]))
            .is_empty());
    }
    g.observe(
        180_001,
        frame(180_001, vec![hand(7, 0.3, false), hand(9, 0.7, false)]),
    );
    g.observe(
        200_001,
        frame(200_001, vec![hand(7, 0.3, true), hand(9, 0.7, true)]),
    );
    let next = g.observe(
        240_001,
        frame(240_001, vec![hand(7, 0.3, true), hand(9, 0.7, true)]),
    );
    assert_eq!(next.len(), 2);
    assert!(next.iter().all(|p| initial.iter().all(|q| q.id != p.id)));
}
#[test]
fn malformed_future_duplicate_and_clock_regression_neutralize() {
    for failure in 0..5 {
        let mut g = engine();
        assert_eq!(press(&mut g).len(), 2);
        let mut f = frame(80_001, vec![hand(7, 0.3, true), hand(9, 0.7, true)]);
        match failure {
            0 => f.source_us = 90_001,
            1 => f.source_us = 60_001,
            2 => f.hands[1].id = 7,
            3 => f.hands[0].cursor.x = f64::NAN,
            _ => f.version = 2,
        }
        assert!(g.observe(80_001, f).is_empty());
        assert!(g
            .observe(100_001, frame(100_001, vec![hand(7, 0.3, true)]))
            .is_empty());
    }
    let mut g = engine();
    press(&mut g);
    assert!(g.tick(50_001).is_empty());
}
#[test]
fn stale_and_stop_release_without_closed_reacquisition() {
    let mut g = engine();
    press(&mut g);
    assert_eq!(g.tick(200_000).len(), 2);
    assert!(g.tick(210_001).is_empty());
    assert!(g
        .observe(220_001, frame(220_001, vec![hand(7, 0.3, true)]))
        .is_empty());
    g.stop();
    assert!(g
        .observe(240_001, frame(240_001, vec![hand(7, 0.3, false)]))
        .is_empty());
}
#[test]
fn workspace_maps_absolute_contacts_and_rejects_degenerate_rectangles() {
    assert!(Gestures::new(
        Config::default(),
        Workspace {
            min: Point { x: 0.5, y: 0.0 },
            max: Point { x: 0.5, y: 1.0 }
        }
    )
    .is_err());
    let mut g = Gestures::new(
        Config::default(),
        Workspace {
            min: Point { x: 0.2, y: 0.2 },
            max: Point { x: 0.8, y: 0.8 },
        },
    )
    .unwrap();
    g.start();
    let points = press(&mut g);
    assert!((points[0].x - 1.0 / 6.0).abs() < 1e-8);
    assert!((points[1].x - 5.0 / 6.0).abs() < 1e-8);
}

#[test]
fn calibrated_workspace_clamps_edges_without_changing_pinch_ownership() {
    let mut g = Gestures::new(
        Config {
            smoothing_us: 0.0,
            ..Config::default()
        },
        Workspace {
            min: Point { x: 0.4, y: 0.2 },
            max: Point { x: 0.6, y: 0.8 },
        },
    )
    .unwrap();
    g.start();
    let points = press(&mut g);
    assert_eq!(points[0].x, 0.0);
    assert_eq!(points[1].x, 1.0);
    let next = g.observe(
        80_001,
        frame(80_001, vec![hand(9, 0.5, true), hand(7, 0.5, true)]),
    );
    assert_eq!(
        points.iter().map(|p| p.id).collect::<Vec<_>>(),
        next.iter().map(|p| p.id).collect::<Vec<_>>()
    );
    assert!(next.iter().all(|p| (p.x - 0.5).abs() < 1e-8));
}
