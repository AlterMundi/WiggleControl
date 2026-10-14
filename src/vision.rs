// SPDX-License-Identifier: Apache-2.0
//! Model coordinate conventions derived from OpenCV Zoo MediaPipe adapters,
//! commit 47534e27, Apache-2.0; licenses retained under models/. Rust inference,
//! tracking/state plumbing is new. No private Barehands code is incorporated.
use image::RgbImage;
use ort::{
    session::{builder::GraphOptimizationLevel, Session},
    value::Tensor,
};
use std::{error::Error, path::Path};

#[derive(Clone, Copy, Debug, Default)]
pub struct XY {
    pub x: f64,
    pub y: f64,
}
impl XY {
    fn distance(self, b: Self) -> f64 {
        (self.x - b.x).hypot(self.y - b.y)
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Roi {
    pub center: XY,
    pub side: f64,
    pub angle: f64,
}
impl Roi {
    /// Unit crop location -> source pixels. Canonical crop fingers point up.
    pub fn source(self, u: f64, v: f64) -> XY {
        let (s, c) = self.angle.sin_cos();
        let x = (u - 0.5) * self.side;
        let y = (v - 0.5) * self.side;
        XY {
            x: self.center.x + c * x - s * y,
            y: self.center.y + s * x + c * y,
        }
    }
    fn local(self, p: XY) -> XY {
        let (s, c) = self.angle.sin_cos();
        let x = p.x - self.center.x;
        let y = p.y - self.center.y;
        XY {
            x: c * x + s * y,
            y: -s * x + c * y,
        }
    }
}
fn bounds(points: &[XY]) -> Option<(XY, XY)> {
    if points.is_empty() || points.iter().any(|p| !p.x.is_finite() || !p.y.is_finite()) {
        return None;
    }
    let lo = XY {
        x: points.iter().map(|p| p.x).fold(f64::INFINITY, f64::min),
        y: points.iter().map(|p| p.y).fold(f64::INFINITY, f64::min),
    };
    let hi = XY {
        x: points.iter().map(|p| p.x).fold(f64::NEG_INFINITY, f64::max),
        y: points.iter().map(|p| p.y).fold(f64::NEG_INFINITY, f64::max),
    };
    Some((lo, hi))
}
fn roi_from(points: &[XY], wrist: XY, middle: XY, scale: f64, shift: f64) -> Option<Roi> {
    if wrist.distance(middle) < 2.0 {
        return None;
    }
    // Rotate the wrist->middle base vector onto the canonical negative Y axis.
    let angle = (middle.y - wrist.y).atan2(middle.x - wrist.x) + std::f64::consts::FRAC_PI_2;
    let frame = Roi {
        center: XY::default(),
        side: 1.0,
        angle,
    };
    let local: Vec<_> = points.iter().map(|p| frame.local(*p)).collect();
    let (lo, hi) = bounds(&local)?;
    let width = hi.x - lo.x;
    let height = hi.y - lo.y;
    let side = width.max(height) * scale;
    if side < 8.0 || !side.is_finite() {
        return None;
    }
    let center = XY {
        x: (lo.x + hi.x) / 2.0,
        y: (lo.y + hi.y) / 2.0 + shift * height,
    };
    let center = frame.source(center.x + 0.5, center.y + 0.5);
    Some(Roi {
        center,
        side,
        angle,
    })
}

/// MediaPipe's hand-landmark ROI recipe uses proximal joints rather than
/// fingertip extrema. See the pinned source links in POSITION-BIAS.md.
/// Kept experimental until recorded-input comparisons qualify the change.
fn proximal_tracking_roi(hand: &DetectedHand) -> Option<Roi> {
    let p = &hand.landmarks;
    let direction = XY {
        x: (p[6].x + p[14].x) / 4.0 + p[10].x / 2.0,
        y: (p[6].y + p[14].y) / 4.0 + p[10].y / 2.0,
    };
    let points: Vec<_> = [0, 1, 2, 3, 5, 6, 9, 10, 13, 14, 17, 18]
        .iter()
        .map(|i| p[*i])
        .collect();
    roi_from(&points, p[0], direction, 2.0, -0.1)
}
pub fn anchors() -> Vec<XY> {
    let mut result = Vec::with_capacity(2016);
    for (side, repeats) in [(24, 2), (12, 6)] {
        for y in 0..side {
            for x in 0..side {
                for _ in 0..repeats {
                    result.push(XY {
                        x: (x as f64 + 0.5) / side as f64,
                        y: (y as f64 + 0.5) / side as f64,
                    });
                }
            }
        }
    }
    result
}
fn crop(image: &RgbImage, roi: Roi, side: usize) -> Vec<f32> {
    let mut output = vec![0.0; side * side * 3];
    for y in 0..side {
        for x in 0..side {
            let p = roi.source(
                (x as f64 + 0.5) / side as f64,
                (y as f64 + 0.5) / side as f64,
            );
            let ix = p.x - 0.5;
            let iy = p.y - 0.5;
            let x0 = ix.floor() as i64;
            let y0 = iy.floor() as i64;
            let tx = ix - x0 as f64;
            let ty = iy - y0 as f64;
            for (xx, yy, weight) in [
                (x0, y0, (1.0 - tx) * (1.0 - ty)),
                (x0 + 1, y0, tx * (1.0 - ty)),
                (x0, y0 + 1, (1.0 - tx) * ty),
                (x0 + 1, y0 + 1, tx * ty),
            ] {
                if xx >= 0 && yy >= 0 && xx < image.width() as i64 && yy < image.height() as i64 {
                    let pixel = image.get_pixel(xx as u32, yy as u32);
                    for c in 0..3 {
                        output[(y * side + x) * 3 + c] += pixel[c] as f32 * (weight / 255.0) as f32;
                    }
                }
            }
        }
    }
    output
}
#[derive(Clone, Debug)]
pub struct DetectedHand {
    pub id: u64,
    pub landmarks: [XY; 21],
    pub world_landmarks: [[f64; 3]; 21],
    pub confidence: f64,
    pub handedness: f64,
    pub tracked: bool,
}
#[derive(Clone)]
struct TrackedHand {
    roi: Roi,
    hand: DetectedHand,
}
pub struct MultiHands {
    pub hands: Vec<DetectedHand>,
    pub ambiguous: bool,
}
pub struct Perception {
    palm: Session,
    hand: Session,
    anchors: Vec<XY>,
    roi: Option<Roi>,
    id: u64,
    tracks: Vec<TrackedHand>,
    search_frames: u32,
    pub palm_score: Option<f64>,
    pub hand_score: Option<f64>,
    pub side_score: Option<f64>,
    pub last_roi: Option<Roi>,
    pub palm_rois: Vec<Roi>,
    pub skipped_rois: Vec<Roi>,
}
fn iou(a: [f64; 4], b: [f64; 4]) -> f64 {
    let w = (a[0] + a[2]).min(b[0] + b[2]) - a[0].max(b[0]);
    let h = (a[1] + a[3]).min(b[1] + b[3]) - a[1].max(b[1]);
    let intersection = w.max(0.0) * h.max(0.0);
    intersection / (a[2] * a[3] + b[2] * b[3] - intersection).max(f64::EPSILON)
}
fn center(hand: &DetectedHand) -> XY {
    XY {
        x: (hand.landmarks[5].x + hand.landmarks[17].x) / 2.0,
        y: (hand.landmarks[5].y + hand.landmarks[17].y) / 2.0,
    }
}
fn pair_uncertain(a: &DetectedHand, b: &DetectedHand) -> bool {
    let width = a.landmarks[5]
        .distance(a.landmarks[17])
        .max(b.landmarks[5].distance(b.landmarks[17]));
    center(a).distance(center(b)) < 0.8 * width
}
fn identity_uncertain(a: &DetectedHand, b: &DetectedHand, side: f64) -> bool {
    center(a).distance(center(b)) > 0.5 * side
        || (a.handedness < 0.2 && b.handedness > 0.8)
        || (a.handedness > 0.8 && b.handedness < 0.2)
}
fn session(path: &Path) -> Result<Session, Box<dyn Error>> {
    Ok(Session::builder()?
        .with_intra_threads(1)?
        .with_optimization_level(GraphOptimizationLevel::Level3)?
        .commit_from_file(path)?)
}
impl Perception {
    pub fn new(models: &Path) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            palm: session(&models.join("palm_detection_mediapipe_2023feb.onnx"))?,
            hand: session(&models.join("handpose_estimation_mediapipe_2023feb.onnx"))?,
            anchors: anchors(),
            roi: None,
            id: 0,
            tracks: Vec::new(),
            search_frames: 0,
            palm_score: None,
            hand_score: None,
            side_score: None,
            last_roi: None,
            palm_rois: Vec::new(),
            skipped_rois: Vec::new(),
        })
    }
    fn detect_palms(&mut self, image: &RgbImage) -> Result<Vec<Roi>, Box<dyn Error>> {
        let side = image.width().max(image.height()) as f64;
        let square = Roi {
            center: XY {
                x: image.width() as f64 / 2.0,
                y: image.height() as f64 / 2.0,
            },
            side,
            angle: 0.0,
        };
        let input = Tensor::from_array(([1usize, 192, 192, 3], crop(image, square, 192)))?;
        let results = self.palm.run(ort::inputs![input])?;
        let (shape, boxes) = results[0].try_extract_tensor::<f32>()?;
        let (scores_shape, scores) = results[1].try_extract_tensor::<f32>()?;
        if shape.as_ref() != [1, 2016, 18] || scores_shape.as_ref() != [1, 2016, 1] {
            return Err("unexpected palm model output contract".into());
        }
        let mut candidates: Vec<_> = scores
            .iter()
            .enumerate()
            .filter_map(|(i, s)| {
                let score = 1.0 / (1.0 + (-f64::from(*s)).exp());
                let raw = &boxes[i * 18..(i + 1) * 18];
                if !score.is_finite()
                    || score < 0.5
                    || raw.iter().any(|p| !p.is_finite())
                    || raw[2] <= 0.0
                    || raw[3] <= 0.0
                {
                    return None;
                }
                let anchor = self.anchors[i];
                let center = square.source(
                    anchor.x + f64::from(raw[0]) / 192.0,
                    anchor.y + f64::from(raw[1]) / 192.0,
                );
                let rect = [
                    center.x - f64::from(raw[2]) * side / 384.0,
                    center.y - f64::from(raw[3]) * side / 384.0,
                    f64::from(raw[2]) * side / 192.0,
                    f64::from(raw[3]) * side / 192.0,
                ];
                Some((i, score, rect))
            })
            .collect();
        candidates.sort_by(|a, b| b.1.total_cmp(&a.1));
        self.palm_score = candidates.first().map(|c| c.1);
        let mut selected: Vec<[f64; 4]> = Vec::new();
        let mut rois = Vec::new();
        for (index, _, rect) in candidates {
            if selected.iter().any(|r| iou(*r, rect) > 0.3) {
                continue;
            }
            let raw = &boxes[index * 18..(index + 1) * 18];
            let anchor = self.anchors[index];
            let mut points = [XY::default(); 7];
            for i in 0..7 {
                points[i] = square.source(
                    anchor.x + f64::from(raw[4 + i * 2]) / 192.0,
                    anchor.y + f64::from(raw[5 + i * 2]) / 192.0,
                );
            }
            if let Some(roi) = roi_from(&points, points[0], points[2], 3.0, -0.4) {
                selected.push(rect);
                rois.push(roi);
            }
            if rois.len() == 2 {
                break;
            }
        }
        self.palm_rois = rois.clone();
        Ok(rois)
    }
    pub fn infer(&mut self, image: &RgbImage) -> Result<Option<DetectedHand>, Box<dyn Error>> {
        let tracked = self.roi.is_some();
        let roi = if let Some(roi) = self.roi {
            roi
        } else {
            let Some(roi) = self.detect_palms(image)?.into_iter().next() else {
                return Ok(None);
            };
            self.id = self.id.checked_add(1).ok_or("hand identity exhausted")?;
            roi
        };
        let detected = self.infer_roi(image, roi, self.id, tracked)?;
        self.roi = detected
            .as_ref()
            .and_then(|h| roi_from(&h.landmarks, h.landmarks[0], h.landmarks[9], 1.65, -0.1));
        Ok(detected)
    }
    fn infer_roi(
        &mut self,
        image: &RgbImage,
        roi: Roi,
        id: u64,
        tracked: bool,
    ) -> Result<Option<DetectedHand>, Box<dyn Error>> {
        self.last_roi = Some(roi);
        let input = Tensor::from_array(([1usize, 224, 224, 3], crop(image, roi, 224)))?;
        let outputs = self.hand.run(ort::inputs![input])?;
        let (_, landmarks) = outputs[0].try_extract_tensor::<f32>()?;
        let (_, confidence) = outputs[1].try_extract_tensor::<f32>()?;
        let (_, handedness) = outputs[2].try_extract_tensor::<f32>()?;
        let (_, world) = outputs[3].try_extract_tensor::<f32>()?;
        self.hand_score = confidence.first().map(|n| f64::from(*n));
        self.side_score = handedness.first().map(|n| f64::from(*n));
        if landmarks.len() != 63
            || world.len() != 63
            || world.iter().any(|n| !n.is_finite())
            || confidence.len() != 1
            || handedness.len() != 1
        {
            return Err("unexpected hand model output contract".into());
        }
        if !handedness[0].is_finite()
            || !(0.0..=1.0).contains(&handedness[0])
            || !confidence[0].is_finite()
            || confidence[0] < 0.8
            || landmarks.iter().any(|p| !p.is_finite())
        {
            return Ok(None);
        }
        let mut points = [XY::default(); 21];
        for i in 0..21 {
            points[i] = roi.source(
                landmarks[i * 3] as f64 / 224.0,
                landmarks[i * 3 + 1] as f64 / 224.0,
            );
        }
        Ok(Some(DetectedHand {
            id,
            landmarks: points,
            world_landmarks: std::array::from_fn(|i| {
                std::array::from_fn(|j| f64::from(world[i * 3 + j]))
            }),
            confidence: confidence[0] as f64,
            handedness: handedness[0] as f64,
            tracked,
        }))
    }
    pub fn infer_many(&mut self, image: &RgbImage) -> Result<MultiHands, Box<dyn Error>> {
        self.skipped_rois.clear();
        let previous = std::mem::take(&mut self.tracks);
        let mut next = Vec::new();
        for track in previous {
            if let Some(hand) = self.infer_roi(image, track.roi, track.hand.id, true)? {
                if identity_uncertain(&track.hand, &hand, track.roi.side) {
                    return Ok(MultiHands {
                        hands: Vec::new(),
                        ambiguous: true,
                    });
                }
                if let Some(roi) = roi_from(
                    &hand.landmarks,
                    hand.landmarks[0],
                    hand.landmarks[9],
                    1.65,
                    -0.1,
                ) {
                    next.push(TrackedHand { roi, hand });
                }
            }
        }
        self.search_frames = self.search_frames.wrapping_add(1);
        // Find a missing/new hand periodically; existing ROI identities do not follow detector order.
        if next.len() < 2 && (next.is_empty() || self.search_frames % 5 == 0) {
            for roi in self.detect_palms(image)? {
                if next
                    .iter()
                    .any(|t| t.roi.center.distance(roi.center) < 0.4 * t.roi.side.max(roi.side))
                {
                    self.skipped_rois.push(roi);
                    continue;
                }
                self.id = self.id.checked_add(1).ok_or("hand identity exhausted")?;
                if let Some(hand) = self.infer_roi(image, roi, self.id, false)? {
                    if let Some(roi) = roi_from(
                        &hand.landmarks,
                        hand.landmarks[0],
                        hand.landmarks[9],
                        1.65,
                        -0.1,
                    ) {
                        next.push(TrackedHand { roi, hand });
                    }
                }
                if next.len() == 2 {
                    break;
                }
            }
        }
        if next.len() == 2 && pair_uncertain(&next[0].hand, &next[1].hand) {
            return Ok(MultiHands {
                hands: Vec::new(),
                ambiguous: true,
            });
        }
        let hands = next.iter().map(|t| t.hand.clone()).collect();
        self.tracks = next;
        Ok(MultiHands {
            hands,
            ambiguous: false,
        })
    }
}
impl DetectedHand {
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
        let eligible = in_view && (extended(8, 6) || others >= 2);
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
            }),
        }
    }
}

/// Controlled image translation, not a source of known physical fingertip distances.
/// The fixed-ROI branch isolates preprocessing/model invariance; cold detection
/// followed by ten identical frames additionally exercises detection and tracking.
/// No camera, device, live configuration or production inference path is changed.
pub fn position_sweep(
    models: &Path,
    image: &RgbImage,
    directory: &Path,
) -> Result<serde_json::Value, Box<dyn Error>> {
    use serde_json::json;
    let mut model = Perception::new(models)?;
    let reference = model.infer(image)?.ok_or("reference hand not detected")?;
    let roi = model.last_roi.ok_or("reference ROI missing")?;
    let corners = [
        roi.source(0., 0.),
        roi.source(0., 1.),
        roi.source(1., 0.),
        roi.source(1., 1.),
    ];
    let (lo, hi) = bounds(&corners).ok_or("invalid reference ROI")?;
    // Keep interpolation support inside the copied patch, without clipping/padding
    // the reference hand differently at different sweep locations.
    let left = lo.x.floor() as i64 - 2;
    let top = lo.y.floor() as i64 - 2;
    let right = hi.x.ceil() as i64 + 2;
    let bottom = hi.y.ceil() as i64 + 2;
    if left < 0 || top < 0 || right > i64::from(image.width()) || bottom > i64::from(image.height())
    {
        return Err("reference crop touches image boundary; choose a fully visible hand".into());
    }
    let patch = image::imageops::crop_imm(
        image,
        left as u32,
        top as u32,
        (right - left) as u32,
        (bottom - top) as u32,
    )
    .to_image();
    std::fs::create_dir(directory)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(directory, std::fs::Permissions::from_mode(0o700))?;
    }
    let baseline_crop = crop(image, roi, 224);
    let metrics = |h: &DetectedHand| {
        let distance = |a: usize, b: usize| {
            h.world_landmarks[a]
                .iter()
                .zip(h.world_landmarks[b])
                .map(|(x, y)| (x - y).powi(2))
                .sum::<f64>()
                .sqrt()
        };
        json!({"gap_m":h.world_pinch_distance_m(),
            "gap_xyz_m":std::array::from_fn::<_,3,_>(|i|h.world_landmarks[4][i]-h.world_landmarks[8][i]),
            "palm_length_m":distance(0,9),"palm_width_m":distance(5,17),
            "confidence":h.confidence,
            "world_landmarks":h.world_landmarks,
            "landmarks":h.landmarks.iter().map(|p|[p.x,p.y]).collect::<Vec<_>>()})
    };
    let mut samples = Vec::new();
    for gy in 0..3 {
        for gx in 0..3 {
            let x = gx * (image.width() - patch.width()) / 2;
            let y = gy * (image.height() - patch.height()) / 2;
            let shifted = translated_patch(&patch, image.width(), image.height(), x, y);
            let dx = f64::from(x) - left as f64;
            let dy = f64::from(y) - top as f64;
            let shifted_roi = Roi {
                center: XY {
                    x: roi.center.x + dx,
                    y: roi.center.y + dy,
                },
                ..roi
            };
            let input = crop(&shifted, shifted_roi, 224);
            let max_input_delta = input
                .iter()
                .zip(&baseline_crop)
                .map(|(a, b)| f64::from((a - b).abs()))
                .fold(0.0, f64::max);
            let fixed = model.infer_roi(&shifted, shifted_roi, 1, false)?;
            let fixed = fixed.as_ref().map(metrics);
            // Reset tracking for every location; do not leak a preceding ROI or
            // make translation invariance tautological in the end-to-end branch.
            model.roi = None;
            let mut tracked = Vec::new();
            for _ in 0..10 {
                let hand = model.infer(&shifted)?;
                tracked.push(hand.as_ref().map(metrics));
            }
            model.roi = None;
            let mut proximal = Vec::new();
            for _ in 0..10 {
                let hand = model.infer(&shifted)?;
                model.roi = hand.as_ref().and_then(proximal_tracking_roi);
                proximal.push(hand.as_ref().map(metrics));
            }
            let file = format!("position-{gx}-{gy}.png");
            shifted.save(directory.join(&file))?;
            samples.push(json!({"grid":[gx,gy],"translation_px":[dx,dy],
                "image":file,"fixed_crop_max_input_delta":max_input_delta,
                "fixed_roi":fixed,"detection_and_tracking":tracked,
                "experimental_proximal_tracking":proximal}));
        }
    }
    let report = json!({"schema":"wigglecontrol.position-sweep/v1",
        "control":"identical hand pixels, orientation and scale; integer translation only",
        "physical_distance_ground_truth":false,"camera_opened":false,
        "image_size":[image.width(),image.height()],
        "patch_size":[patch.width(),patch.height()],"reference":metrics(&reference),"samples":samples});
    std::fs::write(
        directory.join("report.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    Ok(report)
}

fn translated_patch(patch: &RgbImage, width: u32, height: u32, x: u32, y: u32) -> RgbImage {
    let mut canvas = RgbImage::new(width, height);
    image::imageops::replace(&mut canvas, patch, i64::from(x), i64::from(y));
    canvas
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn controlled_translation_preserves_pixels_and_rotated_model_input() {
        let patch = RgbImage::from_fn(64, 64, |x, y| image::Rgb([x as u8, y as u8, (x + y) as u8]));
        let original = translated_patch(&patch, 200, 160, 20, 30);
        let shifted = translated_patch(&patch, 200, 160, 100, 70);
        let roi = Roi {
            center: XY { x: 52., y: 62. },
            side: 40.,
            angle: 0.4,
        };
        let moved = Roi {
            center: XY { x: 132., y: 102. },
            ..roi
        };
        let a = crop(&original, roi, 224);
        let b = crop(&shifted, moved, 224);
        assert!(a.iter().zip(b).all(|(a, b)| (a - b).abs() < 1e-6));
        for y in 0..64 {
            for x in 0..64 {
                assert_eq!(
                    original.get_pixel(20 + x, 30 + y),
                    shifted.get_pixel(100 + x, 70 + y)
                );
            }
        }
    }
    #[test]
    fn suppression_and_ambiguous_identity_geometry() {
        assert!((iou([0.0, 0.0, 10.0, 10.0], [0.0, 0.0, 10.0, 10.0]) - 1.0).abs() < 1e-8);
        assert_eq!(iou([0.0, 0.0, 10.0, 10.0], [20.0, 0.0, 10.0, 10.0]), 0.0);
        let mut a = DetectedHand {
            id: 1,
            landmarks: [XY { x: 100.0, y: 100.0 }; 21],
            world_landmarks: [[0.0; 3]; 21],
            confidence: 0.99,
            handedness: 0.9,
            tracked: true,
        };
        a.landmarks[5].x = 80.0;
        a.landmarks[17].x = 120.0;
        let mut b = a.clone();
        b.id = 2;
        assert!(pair_uncertain(&a, &b));
        for p in &mut b.landmarks {
            p.x += 100.0;
        }
        assert!(!pair_uncertain(&a, &b));
        assert!(!identity_uncertain(&a, &b, 300.0));
        assert!(identity_uncertain(&a, &b, 100.0));
        b = a.clone();
        b.handedness = 0.1;
        assert!(identity_uncertain(&a, &b, 300.0));
    }
    #[test]
    fn anchor_count_and_order_match_model() {
        let a = anchors();
        assert_eq!(a.len(), 2016);
        assert_eq!(a[0].x, a[1].x);
        assert!((a[1152].x - 1.0 / 24.0).abs() < 1e-8);
        assert!((a[2015].y - 23.0 / 24.0).abs() < 1e-8);
    }
    #[test]
    fn roi_rotation_round_trip_and_finger_orientation() {
        let r = Roi {
            center: XY { x: 300.0, y: 200.0 },
            side: 100.0,
            angle: 0.7,
        };
        let p = r.source(0.2, 0.8);
        let local = r.local(p);
        assert!((local.x + 30.0).abs() < 1e-8);
        assert!((local.y - 30.0).abs() < 1e-8);
        let points = [
            XY { x: 40.0, y: 80.0 },
            XY { x: 30.0, y: 50.0 },
            XY { x: 40.0, y: 40.0 },
            XY { x: 55.0, y: 55.0 },
        ];
        let roi = roi_from(&points, points[0], points[2], 3.0, -0.4).unwrap();
        assert!(roi.local(points[2]).y < roi.local(points[0]).y);
    }
    #[test]
    fn fist_or_out_of_view_hand_cannot_engage_and_pinch_metric_is_square() {
        let mut hand = DetectedHand {
            id: 1,
            landmarks: [XY { x: 200.0, y: 300.0 }; 21],
            world_landmarks: [[0.0; 3]; 21],
            confidence: 0.99,
            handedness: 0.9,
            tracked: false,
        };
        hand.landmarks[0] = XY { x: 200.0, y: 400.0 };
        hand.landmarks[5] = XY { x: 180.0, y: 340.0 };
        hand.landmarks[17] = XY { x: 240.0, y: 340.0 };
        assert!(!hand.observation(640, 480, 1).hand.unwrap().engaged);
        hand.landmarks[6] = XY { x: 180.0, y: 280.0 };
        hand.landmarks[8] = XY { x: 180.0, y: 180.0 };
        hand.landmarks[4] = XY { x: 186.0, y: 180.0 };
        hand.world_landmarks[4] = [0.0, 0.0, 0.01];
        hand.world_landmarks[8] = [0.0, 0.0, 0.035];
        assert!((hand.world_pinch_distance_m() - 0.025).abs() < 1e-12);
        hand.world_landmarks[4] = [0.01, 0.0, 0.0];
        hand.world_landmarks[8] = [0.035, 0.0, 0.0];
        assert!((hand.world_pinch_distance_m() - 0.025).abs() < 1e-12);
        assert!(hand.in_view(640, 480));
        let observed = hand.observation(640, 480, 1).hand.unwrap();
        assert!(observed.engaged);
        assert!(
            (observed.thumb_tip.distance(observed.index_tip) / observed.palm_width - 0.1).abs()
                < 1e-8
        );
        hand.landmarks[8].x = -1.0;
        assert!(!hand.in_view(640, 480));
        assert!(!hand.observation(640, 480, 1).hand.unwrap().pinch_eligible);
    }
    #[test]
    fn interpolation_and_black_padding_are_bounded() {
        let image = RgbImage::from_pixel(4, 4, image::Rgb([255, 128, 0]));
        let data = crop(
            &image,
            Roi {
                center: XY { x: 2.0, y: 2.0 },
                side: 4.0,
                angle: 0.0,
            },
            4,
        );
        assert_eq!(data[0], 1.0);
        assert!((data[1] - 128.0 / 255.0).abs() < 1e-6);
        let zero = crop(
            &image,
            Roi {
                center: XY { x: -20.0, y: -20.0 },
                side: 4.0,
                angle: 0.0,
            },
            4,
        );
        assert!(zero.iter().all(|v| *v == 0.0));
    }
}
