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
    pub confidence: f64,
    pub handedness: f64,
    pub tracked: bool,
}
pub struct Perception {
    palm: Session,
    hand: Session,
    anchors: Vec<XY>,
    roi: Option<Roi>,
    id: u64,
    pub palm_score: Option<f64>,
    pub hand_score: Option<f64>,
    pub side_score: Option<f64>,
    pub last_roi: Option<Roi>,
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
            palm_score: None,
            hand_score: None,
            side_score: None,
            last_roi: None,
        })
    }
    fn detect_palm(&mut self, image: &RgbImage) -> Result<Option<Roi>, Box<dyn Error>> {
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
        let best = scores
            .iter()
            .enumerate()
            .filter(|(_, score)| score.is_finite())
            .map(|(i, s)| (i, 1.0 / (1.0 + (-(*s as f64)).exp())))
            .max_by(|a, b| a.1.total_cmp(&b.1));
        self.palm_score = best.map(|(_, score)| score);
        let best = best.filter(|(_, score)| *score >= 0.5);
        let Some((index, _)) = best else {
            return Ok(None);
        };
        let raw = &boxes[index * 18..(index + 1) * 18];
        if raw.iter().any(|p| !p.is_finite()) {
            return Ok(None);
        }
        let anchor = self.anchors[index];
        let mut points = [XY::default(); 7];
        for i in 0..7 {
            points[i] = square.source(
                anchor.x + raw[4 + i * 2] as f64 / 192.0,
                anchor.y + raw[5 + i * 2] as f64 / 192.0,
            );
        }
        Ok(roi_from(&points, points[0], points[2], 3.0, -0.4))
    }
    pub fn infer(&mut self, image: &RgbImage) -> Result<Option<DetectedHand>, Box<dyn Error>> {
        let tracked = self.roi.is_some();
        let roi = if let Some(roi) = self.roi {
            roi
        } else {
            let Some(roi) = self.detect_palm(image)? else {
                return Ok(None);
            };
            self.id = self.id.checked_add(1).ok_or("hand identity exhausted")?;
            roi
        };
        self.last_roi = Some(roi);
        let input = Tensor::from_array(([1usize, 224, 224, 3], crop(image, roi, 224)))?;
        let outputs = self.hand.run(ort::inputs![input])?;
        let (_, landmarks) = outputs[0].try_extract_tensor::<f32>()?;
        let (_, confidence) = outputs[1].try_extract_tensor::<f32>()?;
        let (_, handedness) = outputs[2].try_extract_tensor::<f32>()?;
        self.hand_score = confidence.first().map(|n| f64::from(*n));
        self.side_score = handedness.first().map(|n| f64::from(*n));
        if landmarks.len() != 63 || confidence.len() != 1 || handedness.len() != 1 {
            self.roi = None;
            return Err("unexpected hand model output contract".into());
        }
        if !confidence[0].is_finite()
            || confidence[0] < 0.8
            || landmarks.iter().any(|p| !p.is_finite())
        {
            self.roi = None;
            return Ok(None);
        }
        let mut points = [XY::default(); 21];
        for i in 0..21 {
            points[i] = roi.source(
                landmarks[i * 3] as f64 / 224.0,
                landmarks[i * 3 + 1] as f64 / 224.0,
            );
        }
        self.roi = roi_from(&points, points[0], points[9], 1.65, -0.1);
        Ok(Some(DetectedHand {
            id: self.id,
            landmarks: points,
            confidence: confidence[0] as f64,
            handedness: handedness[0] as f64,
            tracked,
        }))
    }
}
impl DetectedHand {
    pub fn observation(&self, width: u32, height: u32, source_us: u64) -> crate::Observation {
        let points = &self.landmarks;
        let metric = width.max(height) as f64;
        let in_view = points
            .iter()
            .all(|p| p.x >= 0.0 && p.y >= 0.0 && p.x < width as f64 && p.y < height as f64);
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
            }),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
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
        let observed = hand.observation(640, 480, 1).hand.unwrap();
        assert!(observed.engaged);
        assert!(
            (observed.thumb_tip.distance(observed.index_tip) / observed.palm_width - 0.1).abs()
                < 1e-8
        );
        hand.landmarks[8].x = -1.0;
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
