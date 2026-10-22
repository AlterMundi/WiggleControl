// SPDX-License-Identifier: Apache-2.0
//! Shared landmark geometry and controller interpretation; no model/camera or OS API.
//! Extracted from the existing OpenCV-Zoo-coordinate vision/session adapters.
use crate::Observation;
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, Default)]
pub struct XY {
    pub x: f64,
    pub y: f64,
}
impl XY {
    pub(crate) fn distance(self, b: Self) -> f64 {
        (self.x - b.x).hypot(self.y - b.y)
    }
}
#[derive(Clone, Debug)]
pub struct DetectedHand {
    pub id: u64,
    pub landmarks: [XY; 21],
    /// Relative image depth in source pixels (same scale as X), before mirroring.
    /// This is a model image coordinate, not metric world depth. None means unavailable.
    pub image_depths: Option<[f64; 21]>,
    pub world_landmarks: [[f64; 3]; 21],
    pub confidence: f64,
    pub handedness: f64,
    pub tracked: bool,
    /// Bounded same-hand crop alternative; constructors leave its own alternative empty.
    pub secondary: Option<Box<DetectedHand>>,
    pub secondary_source_id: u64,
}
pub struct MultiHands {
    pub hands: Vec<DetectedHand>,
    pub ambiguous: bool,
    /// Diagnostic cause of an explicitly rejected identity set.
    pub ambiguity_reason: Option<&'static str>,
}
pub const PROXIMAL_PINCH_SOURCE: u64 = 1;

impl DetectedHand {
    pub fn secondary_debug(&self) -> Option<serde_json::Value> {
        self.secondary.as_ref().map(|h| {
            serde_json::json!({
                "source_id":self.secondary_source_id,"confidence":h.confidence,
                "world_pinch_distance_m":h.world_pinch_distance_m(),
                "landmarks":h.landmarks.map(|p|[p.x,p.y]),"world_landmarks":h.world_landmarks,
            })
        })
    }
    pub fn world_pinch_distance_m(&self) -> f64 {
        self.world_landmarks[4]
            .iter()
            .zip(self.world_landmarks[8])
            .map(|(a, b)| (a - b).powi(2))
            .sum::<f64>()
            .sqrt()
    }

    pub fn in_view(&self, width: u32, height: u32) -> bool {
        self.landmarks
            .iter()
            .all(|p| p.x >= 0.0 && p.y >= 0.0 && p.x < width as f64 && p.y < height as f64)
    }
    pub fn observation(&self, width: u32, height: u32, source_us: u64) -> crate::Observation {
        self.observation_with_posture(width, height, source_us, false)
    }
    /// A contextual detail posture can replace only the open-finger gate.
    /// Framing and all later metric/confidence/source checks still apply.
    pub(crate) fn observation_with_posture(
        &self,
        width: u32,
        height: u32,
        source_us: u64,
        detail_pinch_posture: bool,
    ) -> crate::Observation {
        let points = &self.landmarks;
        let metric = width.max(height) as f64;
        let in_view = self.in_view(width, height);
        let extended = |tip: usize, pip: usize| {
            points[0].distance(points[tip]) > points[0].distance(points[pip]) * 1.05
        };
        let others = [(12, 10), (16, 14), (20, 18)]
            .iter()
            .filter(|(tip, pip)| extended(*tip, *pip))
            .count();
        let eligible = in_view && (extended(8, 6) || others >= 2 || detail_pinch_posture);
        let center = XY {
            x: (points[5].x + points[17].x) / 2.0,
            y: (points[5].y + points[17].y) / 2.0,
        };
        let normalized = |p: XY| crate::Point {
            x: (1.0 - p.x / metric).clamp(0.0, 1.0),
            y: (p.y / metric).clamp(0.0, 1.0),
        };
        crate::Observation {
            version: 1,
            source_us,
            hand: Some(crate::Hand {
                id: self.id,
                confidence: self.confidence,
                engaged: eligible,
                pinch_eligible: eligible,
                pinch_onset_allowed: true,
                scrolling: points[4].distance(points[12]) / points[5].distance(points[17]) < 0.25
                    && points[4].distance(points[8]) / points[5].distance(points[17]) > 0.4,
                cursor: crate::Point {
                    x: (1.0 - center.x / width as f64).clamp(0.0, 1.0),
                    y: (center.y / height as f64).clamp(0.0, 1.0),
                },
                thumb_tip: normalized(points[4]),
                index_tip: normalized(points[8]),
                palm_width: points[5].distance(points[17]) / metric,
                pinch_distance_m: None,
                pinch_distance_ratio: None,
                secondary_pinch: None,
            }),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Pinch2dReference {
    #[default]
    PalmWidth,
    WristMiddleMcp,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Interpretation {
    pub pinch_3d: bool,
    pub pinch_2d_reference: Pinch2dReference,
    pub pinch_onset_max_screen_ratio: Option<f64>,
    pub dual_crop: bool,
}
impl Interpretation {
    /// Apply the selected pinch interpretation identically in pointer, touch and replay.
    pub fn observation(
        &self,
        detected: Option<&DetectedHand>,
        width: u32,
        height: u32,
        source_us: u64,
    ) -> Observation {
        self.observation_with_posture(detected, width, height, source_us, false)
    }
    fn observation_with_posture(
        &self,
        detected: Option<&DetectedHand>,
        width: u32,
        height: u32,
        source_us: u64,
        detail_pinch_posture: bool,
    ) -> Observation {
        let mut observation = detected
            .map(|hand| {
                hand.observation_with_posture(width, height, source_us, detail_pinch_posture)
            })
            .unwrap_or(Observation {
                version: 1,
                source_us,
                hand: None,
            });
        if self.pinch_3d {
            if let (Some(hand), Some(detected)) = (&mut observation.hand, detected) {
                hand.pinch_distance_m = Some(detected.world_pinch_distance_m());
                // The legacy 2D scroll chord must not override a 3D pinch.
                hand.scrolling = false;
            }
        } else if self.pinch_2d_reference == Pinch2dReference::WristMiddleMcp {
            if let (Some(hand), Some(detected)) = (&mut observation.hand, detected) {
                let points = &detected.landmarks;
                let palm = points[0].distance(points[9]);
                let ratio = points[4].distance(points[8]) / palm;
                if !palm.is_finite() || palm <= 1e-6 || !ratio.is_finite() {
                    hand.pinch_eligible = false;
                } else {
                    hand.pinch_distance_ratio = Some(ratio);
                }
                hand.scrolling = false;
            }
        }
        if let (Some(limit), Some(hand), Some(detected)) = (
            self.pinch_onset_max_screen_ratio,
            &mut observation.hand,
            detected,
        ) {
            let points = &detected.landmarks;
            let gap = (points[4].x - points[8].x).hypot(points[4].y - points[8].y);
            let palm = (points[0].x - points[9].x).hypot(points[0].y - points[9].y);
            let ratio = gap / palm;
            if !ratio.is_finite() || palm <= 1e-6 {
                // Missing geometry is observation loss, not a valid open/hold.
                hand.pinch_eligible = false;
            }
            hand.pinch_onset_allowed = ratio.is_finite() && palm > 1e-6 && ratio <= limit;
        }
        if self.dual_crop && self.pinch_3d {
            if let (Some(hand), Some(secondary)) = (
                &mut observation.hand,
                detected.and_then(|h| h.secondary.as_deref()),
            ) {
                let gap = secondary.world_pinch_distance_m();
                if gap.is_finite()
                    && (0.0..=1.0).contains(&gap)
                    && secondary.confidence.is_finite()
                    && (0.0..=1.0).contains(&secondary.confidence)
                {
                    let mut eligible = secondary
                        .observation(width, height, source_us)
                        .hand
                        .is_some_and(|h| h.pinch_eligible);
                    let allowed = if let Some(limit) = self.pinch_onset_max_screen_ratio {
                        let p = &secondary.landmarks;
                        let palm = (p[0].x - p[9].x).hypot(p[0].y - p[9].y);
                        let ratio = (p[4].x - p[8].x).hypot(p[4].y - p[8].y) / palm;
                        if palm <= 1e-6 || !ratio.is_finite() {
                            eligible = false;
                        }
                        palm > 1e-6 && ratio.is_finite() && ratio <= limit
                    } else {
                        true
                    };
                    hand.secondary_pinch = Some(crate::PinchEvidence {
                        source_id: detected
                            .expect("secondary belongs to this detected hand")
                            .secondary_source_id,
                        source_us,
                        confidence: secondary.confidence,
                        eligible,
                        distance_m: gap,
                        onset_allowed: allowed,
                    });
                }
            }
        }
        observation
    }
}

/// Share the exact pinch metric/normalization between index and middle fingers.
pub fn observe_controller(
    config: &Interpretation,
    controller: &mut crate::controller::Controller,
    now: u64,
    mut observation: crate::touch_gestures::Frame,
    pose: Option<crate::macros::HandPose>,
    detected: &[DetectedHand],
    size: [u32; 2],
) -> crate::controller::Output {
    let selected = controller.selected_pointer_hand(&observation);
    let detail_pinch_posture = controller.active_detail_config().is_some_and(|settings| {
        detected
            .iter()
            .find(|h| Some(h.id) == selected)
            .is_some_and(|h| crate::detail::pinch_posture(settings, &h.world_landmarks))
    });
    if detail_pinch_posture {
        if let Some(raw) = detected.iter().find(|h| Some(h.id) == selected) {
            if let Some(interpreted) = observation.hands.iter_mut().find(|h| h.id == raw.id) {
                *interpreted = config
                    .observation_with_posture(
                        Some(raw),
                        size[0],
                        size[1],
                        observation.source_us,
                        true,
                    )
                    .hand
                    .expect("raw hand remains present");
            }
        }
    }
    let evidence = controller
        .right_click_enabled()
        .then(|| {
            let id = controller.selected_pointer_hand(&observation)?;
            let hand = detected.iter().find(|h| h.id == id)?;
            let mut middle = hand.clone();
            middle.landmarks[8] = hand.landmarks[12];
            middle.world_landmarks[8] = hand.world_landmarks[12];
            let interpreted = config
                .observation_with_posture(
                    Some(&middle),
                    size[0],
                    size[1],
                    observation.source_us,
                    detail_pinch_posture,
                )
                .hand?;
            Some(crate::pointer_buttons::Evidence {
                hand_id: id,
                source_us: observation.source_us,
                middle_tip: interpreted.index_tip,
                pinch_distance_m: interpreted.pinch_distance_m,
                pinch_distance_ratio: interpreted.pinch_distance_ratio,
                onset_allowed: interpreted.pinch_onset_allowed,
            })
        })
        .flatten();
    let detail = controller.detail_config().and_then(|settings| {
        let id = controller.selected_pointer_hand(&observation)?;
        let hand = detected.iter().find(|h| h.id == id)?;
        let held = crate::detail::folded(
            settings,
            hand.world_landmarks[13..17].try_into().ok()?,
            hand.world_landmarks[17..21].try_into().ok()?,
        )?;
        Some(crate::detail::Evidence {
            hand_id: id,
            source_us: observation.source_us,
            held,
        })
    });
    let escape = controller.escape_config().and_then(|settings| {
        let id = controller.selected_pointer_hand(&observation)?;
        let hand = detected
            .iter()
            .find(|h| h.id == id && h.in_view(size[0], size[1]))?;
        Some(crate::escape::Evidence {
            hand_id: id,
            source_us: observation.source_us,
            pose: crate::escape::classify(settings, &hand.world_landmarks)?,
        })
    });
    let scroll = controller.scroll_config().and_then(|settings| {
        let id = controller.selected_pointer_hand(&observation)?;
        let hand = detected
            .iter()
            .find(|h| h.id == id && h.in_view(size[0], size[1]))?;
        Some(crate::scroll::Evidence {
            hand_id: id,
            source_us: observation.source_us,
            pose: crate::scroll::classify(settings, &hand.world_landmarks)?,
            wrist: crate::Point {
                x: 1.0 - hand.landmarks[0].x / size[0] as f64,
                y: hand.landmarks[0].y / size[1] as f64,
            },
        })
    });
    controller.observe_with_evidence(
        now,
        observation,
        pose,
        crate::controller::Controls {
            middle_pinch: evidence,
            detail,
            escape,
            scroll,
        },
    )
}

// OpenCV Zoo's pinned PALM_LANDMARK_IDS maps the first five palm points
// to wrist/index/middle/ring/pinky MCP. Thumb articulation is not a scale cue.
pub(crate) fn palm_match_cost(hand: &DetectedHand, palm: &[XY; 7]) -> Option<f64> {
    let scale = hand.landmarks[5]
        .distance(hand.landmarks[17])
        .max(hand.landmarks[0].distance(hand.landmarks[9]));
    if !scale.is_finite() || scale < 8.0 {
        return None;
    }
    let error = [0, 5, 9, 13, 17]
        .into_iter()
        .enumerate()
        .map(|(i, j)| hand.landmarks[j].distance(palm[i]).powi(2))
        .sum::<f64>();
    let cost = (error / 5.0).sqrt() / scale;
    cost.is_finite().then_some(cost)
}
/// Duplicate image predictions are not independent observations of a second hand.
/// This matches model output geometry, never participant identity or hidden anatomy.
pub(crate) fn duplicate_prediction(a: &DetectedHand, b: &DetectedHand) -> bool {
    let scale = [a, b]
        .into_iter()
        .map(|h| {
            h.landmarks[5]
                .distance(h.landmarks[17])
                .max(h.landmarks[0].distance(h.landmarks[9]))
        })
        .fold(0.0_f64, f64::max);
    // Finger predictions can differ with the crop during a pinch. Ownership
    // correspondence must use the same proximal palm points as proposal matching.
    let palm = [0, 5, 9, 13, 17];
    let error = palm
        .into_iter()
        .map(|i| a.landmarks[i].distance(b.landmarks[i]).powi(2))
        .sum::<f64>();
    scale >= 8.0
        && scale.is_finite()
        && error.is_finite()
        && (error / palm.len() as f64).sqrt() / scale < 0.10
        && (a.handedness - b.handedness).abs() < 0.2
}
fn center(hand: &DetectedHand) -> XY {
    XY {
        x: (hand.landmarks[5].x + hand.landmarks[17].x) / 2.0,
        y: (hand.landmarks[5].y + hand.landmarks[17].y) / 2.0,
    }
}
pub(crate) fn pair_uncertain(a: &DetectedHand, b: &DetectedHand, minimum_widths: f64) -> bool {
    let width = a.landmarks[5]
        .distance(a.landmarks[17])
        .max(b.landmarks[5].distance(b.landmarks[17]));
    center(a).distance(center(b)) < minimum_widths * width
}
pub(crate) fn identity_uncertain(a: &DetectedHand, b: &DetectedHand, side: f64) -> bool {
    center(a).distance(center(b)) > 0.5 * side
        || (a.handedness < 0.2 && b.handedness > 0.8)
        || (a.handedness > 0.8 && b.handedness < 0.2)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    pub(crate) fn fictional_hand(
        id: u64,
        pixel_closed: bool,
        world_gap: f64,
    ) -> crate::landmarks::DetectedHand {
        use crate::landmarks::{DetectedHand, XY};
        let mut hand = DetectedHand {
            id,
            landmarks: [XY { x: 200.0, y: 300.0 }; 21],
            image_depths: Some([0.0; 21]),
            world_landmarks: [[0.0; 3]; 21],
            confidence: 0.99,
            handedness: 0.9,
            tracked: true,
            secondary: None,
            secondary_source_id: 0,
        };
        hand.landmarks[0] = XY { x: 200.0, y: 400.0 };
        hand.landmarks[5] = XY { x: 180.0, y: 340.0 };
        hand.landmarks[17] = XY { x: 240.0, y: 340.0 };
        hand.landmarks[6] = XY { x: 180.0, y: 280.0 };
        hand.landmarks[8] = XY { x: 180.0, y: 180.0 };
        hand.landmarks[4] = XY {
            x: if pixel_closed { 186.0 } else { 300.0 },
            y: 180.0,
        };
        hand.world_landmarks[8][2] = world_gap;
        hand
    }

    #[test]
    fn detail_pinching_posture_preserves_raw_drag_without_admitting_a_fist() {
        use crate::escape::Pose;
        let config = Interpretation::default();
        let mut controller = crate::controller::Controller::new(crate::controller::Settings {
            gestures: crate::Config {
                min_confidence: 0.5,
                drag_grace_us: 1_000_000,
                confirm_us: 40_000,
                ..Default::default()
            },
            detail: Some(Default::default()),
            right_click: true,
            escape: Some(Default::default()),
            ..Default::default()
        })
        .unwrap();
        controller.start();
        controller.select_hand(Some(7));
        let mut frame = |t, folded, closed, fist, outside| {
            let mut raw = fictional_hand(7, closed, 0.02);
            raw.world_landmarks = crate::escape::tests::world(if fist {
                Pose::ThumbOut
            } else if folded {
                Pose::Other
            } else {
                Pose::Neutral
            });
            if closed {
                // A fictional projected pinch shortens wrist-to-tip distance.
                // Its nondegenerate index chain is bent, not a fully curled fist.
                raw.landmarks[6].y = 170.0;
                if !fist {
                    raw.world_landmarks[5..9].copy_from_slice(&[
                        [-0.03, 0.0, 0.0],
                        [-0.03, 0.01, 0.0],
                        [-0.03, 0.02, 0.01],
                        [-0.03, 0.015, 0.02],
                    ]);
                }
            }
            if outside {
                raw.landmarks[20].x = -1.0;
            }
            let observation = crate::touch_gestures::Frame {
                version: 1,
                source_us: t,
                ambiguous: false,
                hands: vec![config.observation(Some(&raw), 640, 480, t).hand.unwrap()],
            };
            observe_controller(
                &config,
                &mut controller,
                t,
                observation,
                None,
                &[raw],
                [640, 480],
            )
        };
        frame(1, false, false, false, false);
        frame(100_001, false, false, false, false);
        frame(150_001, true, false, false, false);
        assert!(frame(250_001, true, false, false, false).detail.is_some());
        frame(300_001, true, true, false, false);
        let pressed = frame(350_001, true, true, false, false);
        assert!(pressed.button_down, "detail pinch must establish a button");
        for t in (400_001..=3_000_001).step_by(50_000) {
            let held = frame(t, true, true, false, false);
            assert!(held.button_down, "raw detail drag lost at {t}: {held:?}");
            assert_ne!(held.reason, Some("escape_holding"));
        }
        assert!(!frame(3_050_001, true, false, false, false).button_down);
        assert!(!frame(3_100_001, true, true, true, false).button_down);
        frame(3_150_001, true, false, false, false);
        frame(3_200_001, true, true, false, false);
        assert!(frame(3_250_001, true, true, false, false).button_down);
        assert!(!frame(3_300_001, true, true, false, true).button_down);
    }
}
