#![cfg(target_os = "linux")]
use wigglecontrol::{
    controller::{Controller, Output, Settings},
    cursor,
    delivery::{Packet, Publisher, State},
    detail,
    touch_gestures::{Frame, Workspace},
    Config, Event, Hand, Point,
};
fn hand(x: f64, closed: bool) -> Hand {
    serde_json::from_value(serde_json::json!({
        "id":7,"confidence":0.99,"engaged":true,"pinch_eligible":true,
        "cursor":{"x":x,"y":0.5},"thumb_tip":{"x":0.4,"y":0.5},
        "index_tip":{"x":0.5,"y":0.5},"palm_width":0.2,
        "pinch_distance_ratio":if closed {0.2} else {0.6}
    }))
    .unwrap()
}
struct Pipeline {
    controller: Controller,
    publisher: Publisher,
    writer: State,
    last: Option<Point>,
}
impl Pipeline {
    fn new(smoothing: bool, enabled: bool) -> Self {
        let config = cursor::Config {
            smoothing_us: if smoothing { 30_000.0 } else { 0.0 },
            prediction_us: if smoothing { 80_000 } else { 0 },
            max_speed_per_s: 10.0,
            ..Default::default()
        };
        let mut controller = Controller::new(Settings {
            detail: enabled.then(detail::Config::default),
            cursor_output: Some(config),
            gestures: Config {
                smoothing_us: 0.0,
                drag_grace_us: 1_000_000,
                ..Default::default()
            },
            ..Default::default()
        })
        .unwrap();
        let mut publisher = Publisher::with_cursor_output(Some(config), 300_000).unwrap();
        let mut writer = State::default();
        let start = controller.start();
        let packet = publisher
            .absolute_packet(&start, 0, None, Workspace::default())
            .unwrap()
            .unwrap();
        writer.apply(0, packet, 2000.0, 300_000).unwrap();
        Self {
            controller,
            publisher,
            writer,
            last: None,
        }
    }
    fn frame(
        &mut self,
        t: u64,
        x: f64,
        closed: bool,
        evidence: Option<detail::Evidence>,
    ) -> Output {
        let out = self.controller.observe_with_modifiers(
            t,
            Frame {
                version: 1,
                source_us: t,
                ambiguous: false,
                hands: vec![hand(x, closed)],
            },
            None,
            None,
            evidence,
        );
        let packet = self
            .publisher
            .absolute_packet(&out, t, Some(t), Workspace::default())
            .unwrap()
            .unwrap();
        if let Packet::Frame {
            absolute: Some(point),
            ..
        } = &packet
        {
            self.last = Some(*point);
        }
        self.writer.apply(t, packet, 2000.0, 300_000).unwrap();
        out
    }
    fn pose(&mut self, t: u64, x: f64, closed: bool, held: bool) -> Output {
        self.frame(
            t,
            x,
            closed,
            Some(detail::Evidence {
                hand_id: 7,
                source_us: t,
                held,
            }),
        )
    }
    fn arm(&mut self) {
        self.pose(1, 0.5, false, false);
        self.pose(100_001, 0.5, false, false);
    }
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-9, "{a} != {b}");
}
#[test]
fn held_detail_preserves_drag_and_release_and_has_no_transition_jump() {
    for smoothing in [false, true] {
        let mut p = Pipeline::new(smoothing, true);
        p.arm();
        p.pose(150_001, 0.55, true, true);
        let out = p.pose(200_001, 0.6, true, true);
        assert!(out.button_down);
        let before = p.last.unwrap();
        let out = p.pose(250_001, 0.65, true, true);
        assert!(out.detail.is_some());
        assert!(out.button_down);
        assert!(!out.transition);
        assert!(out
            .mouse
            .iter()
            .all(|e| !matches!(e, Event::Button { .. } | Event::NamedButton { .. })));
        close(p.last.unwrap().x, before.x);
        p.pose(300_001, 0.75, true, true);
        assert!(p.last.unwrap().x > before.x);
        p.pose(350_001, 0.8, true, false);
        p.pose(400_001, 0.85, true, false);
        let before = p.last.unwrap();
        let out = p.pose(450_001, 0.9, true, false);
        assert!(out.detail.is_none());
        assert!(out.button_down);
        assert!(!out.transition);
        close(p.last.unwrap().x, before.x);
        let out = p.pose(500_001, 0.9, false, false);
        assert!(!out.button_down);
        assert!(out
            .mouse
            .iter()
            .any(|e| matches!(e, Event::Button { down: false })));
        for i in 0..20 {
            p.pose(550_001 + i * 50_000, 0.0, false, false);
        }
        assert!(p.last.unwrap().x < 0.000001);
        // With smoothing enabled, output approaches the full-screen endpoint.
        for i in 0..20 {
            p.pose(1_550_001 + i * 50_000, 1.0, false, false);
        }
        assert!(p.last.unwrap().x > 0.999999);
    }
}
#[test]
fn rest_is_stable_and_slow_subpixel_motion_accumulates_without_edge_windup() {
    let mut p = Pipeline::new(false, true);
    p.arm();
    p.pose(150_001, 0.5, false, true);
    p.pose(250_001, 0.5, false, true);
    let rest = p.last.unwrap();
    for i in 0..100 {
        let jitter = if i % 2 == 0 { 0.0004 } else { -0.0004 };
        p.pose(300_001 + i * 20_000, 0.5 + jitter, false, true);
        close(p.last.unwrap().x, rest.x);
    }
    for i in 1..=20 {
        p.pose(2_300_001 + i * 20_000, 0.5 + i as f64 * 0.0001, false, true);
    }
    close(p.last.unwrap().x, rest.x + 0.00025);
    p.pose(2_800_001, 1.0, false, true);
    let out = p.pose(2_850_001, 1.0, false, true);
    assert!(out.detail.is_some());
    let edge = p.last.unwrap().x;
    p.pose(2_900_001, 0.99, false, true);
    assert!(p.last.unwrap().x < edge);
}
#[test]
fn missing_wrong_owner_or_clock_and_low_confidence_are_not_modifier_release() {
    let mut p = Pipeline::new(false, true);
    p.arm();
    p.pose(150_001, 0.5, true, true);
    p.pose(250_001, 0.5, true, true);
    assert!(p.frame(300_001, 0.5, true, None).detail.is_some());
    assert!(p
        .frame(
            350_001,
            0.5,
            true,
            Some(detail::Evidence {
                hand_id: 9,
                source_us: 350_001,
                held: false
            })
        )
        .detail
        .is_some());
    assert!(p
        .frame(
            400_001,
            0.5,
            true,
            Some(detail::Evidence {
                hand_id: 7,
                source_us: 399_999,
                held: false
            })
        )
        .detail
        .is_some());
    let mut low = hand(0.5, true);
    low.confidence = 0.1;
    let out = p.controller.observe_with_modifiers(
        425_001,
        Frame {
            version: 1,
            source_us: 425_001,
            ambiguous: false,
            hands: vec![low],
        },
        None,
        None,
        Some(detail::Evidence {
            hand_id: 7,
            source_us: 425_001,
            held: false,
        }),
    );
    assert!(out.detail.is_some());
    assert!(out.button_down);
    // Missing evidence also breaks a partially confirmed exit.
    p.pose(450_001, 0.5, true, false);
    p.frame(500_001, 0.5, true, None);
    assert!(p.pose(550_001, 0.5, true, false).detail.is_some());
    assert!(p.pose(650_001, 0.5, true, false).detail.is_none());
    let out = p.controller.stop();
    assert!(out.detail.is_none());
    assert!(!out.button_down);
}
#[test]
fn no_detail_on_closed_acquisition_and_disabled_settings_preserve_pointer() {
    let mut p = Pipeline::new(false, true);
    for t in [1, 100_001, 200_001] {
        assert!(p.pose(t, 0.5, false, true).detail.is_none());
    }
    p.pose(250_001, 0.5, false, false);
    p.pose(350_001, 0.5, false, false);
    p.pose(400_001, 0.5, false, true);
    assert!(p.pose(500_001, 0.5, false, true).detail.is_some());
    let mut p = Pipeline::new(false, false);
    for i in 0..10 {
        let x = 0.2 + i as f64 * 0.04;
        assert!(p.pose(1 + i * 50_000, x, false, true).detail.is_none());
        close(p.last.unwrap().x, x);
    }
}
#[test]
fn folded_finger_evidence_is_rigid_transform_invariant_and_unknown_is_not_open() {
    let straight = [
        [0.0, 0.0, 0.0],
        [0.0, 0.01, 0.0],
        [0.0, 0.02, 0.0],
        [0.0, 0.03, 0.0],
    ];
    let folded = [
        [0.0, 0.0, 0.0],
        [0.0, 0.01, 0.0],
        [0.0, 0.01, 0.01],
        [0.0, 0.0, 0.01],
    ];
    let c = detail::Config::default();
    assert_eq!(detail::folded(c, straight, straight), Some(false));
    assert_eq!(detail::folded(c, folded, folded), Some(true));
    assert_eq!(detail::folded(c, straight, folded), None);
    assert_eq!(detail::folded(c, [[0.0; 3]; 4], straight), None);
    let transformed = folded.map(|p| [0.3 + 2.0 * p[1], 0.4 - 2.0 * p[0], 0.5 + 2.0 * p[2]]);
    close(
        detail::finger_ratio(transformed).unwrap(),
        detail::finger_ratio(folded).unwrap(),
    );
    let mut invalid = folded;
    invalid[2][0] = f64::NAN;
    assert!(detail::finger_ratio(invalid).is_none());
    for config in [
        detail::Config { gain: 0.0, ..c },
        detail::Config {
            rest_radius: f64::NAN,
            ..c
        },
        detail::Config { confirm_us: 0, ..c },
        detail::Config {
            folded_ratio: 0.8,
            extended_ratio: 0.7,
            ..c
        },
    ] {
        assert!(Controller::new(Settings {
            detail: Some(config),
            ..Default::default()
        })
        .is_err());
    }
}

#[test]
fn detail_preserves_named_right_drag_through_missing_frames_and_never_confirms_across_a_gap() {
    use evdev::{EventType, KeyCode};
    use wigglecontrol::{pointer_buttons, MouseButton};
    let mut controller = Controller::new(Settings {
        right_click: true,
        detail: Some(Default::default()),
        gestures: Config {
            drag_grace_us: 1_000_000,
            ..Default::default()
        },
        ..Default::default()
    })
    .unwrap();
    let mut publisher = Publisher::default();
    let mut writer = State::default();
    let start = controller.start();
    writer
        .apply(
            0,
            publisher
                .absolute_packet(&start, 0, None, Workspace::default())
                .unwrap()
                .unwrap(),
            2000.0,
            300_000,
        )
        .unwrap();
    for (t, closed, modifier, missing, active) in [
        (1, false, false, false, false),
        (100_001, false, false, false, false),
        (150_001, true, true, false, false),
        (200_001, true, true, false, false),
        (250_001, true, true, false, true),
        (300_001, true, false, true, true),
        // A long source gap cannot itself satisfy the modifier's release hold.
        (650_001, true, false, false, true),
        (750_001, true, false, false, false),
        (800_001, false, false, false, false),
    ] {
        if t == 650_001 {
            // The independent writer still needs estimator liveness while the
            // camera is missing. Without these holds its watchdog must release.
            for tick in [450_001, 550_001] {
                let output = controller.tick(tick);
                let packet = publisher
                    .absolute_packet(&output, tick, None, Workspace::default())
                    .unwrap()
                    .unwrap();
                assert!(matches!(
                    packet,
                    Packet::Hold {
                        source_us: 250_001,
                        ..
                    }
                ));
                let batch = writer.apply(tick, packet, 2000.0, 300_000).unwrap();
                assert!(batch.mouse.is_empty());
            }
        }
        let output = controller.observe_with_modifiers(
            t,
            Frame {
                version: 1,
                source_us: t,
                ambiguous: false,
                hands: if missing {
                    vec![]
                } else {
                    vec![hand(0.5, false)]
                },
            },
            None,
            (!missing).then_some(pointer_buttons::Evidence {
                hand_id: 7,
                source_us: t,
                middle_tip: Point { x: 0.5, y: 0.5 },
                pinch_distance_m: None,
                pinch_distance_ratio: Some(if closed { 0.2 } else { 0.6 }),
                onset_allowed: true,
            }),
            (!missing).then_some(detail::Evidence {
                hand_id: 7,
                source_us: t,
                held: modifier,
            }),
        );
        assert_eq!(output.detail.is_some(), active, "source{t}");
        let batch = writer
            .apply(
                t,
                publisher
                    .absolute_packet(&output, t, Some(t), Workspace::default())
                    .unwrap()
                    .unwrap(),
                2000.0,
                300_000,
            )
            .unwrap();
        let edges: Vec<_> = batch
            .mouse
            .iter()
            .filter(|e| e.event_type() == EventType::KEY)
            .map(|e| (e.code(), e.value()))
            .collect();
        let expected = match t {
            200_001 => vec![(KeyCode::BTN_RIGHT.0, 1)],
            800_001 => vec![(KeyCode::BTN_RIGHT.0, 0)],
            _ => vec![],
        };
        assert_eq!(edges, expected, "source{t}");
        if (200_001..800_001).contains(&t) {
            assert!(output.button_down);
            assert_eq!(output.pointer_button, Some(MouseButton::Right));
        }
    }
}
