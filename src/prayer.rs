//! Camera-free prayer-like pose candidate. This 2-D heuristic does not establish
//! physical palm contact, and does not activate sensors or emit system input.
#[derive(Clone, Debug)]
pub struct Hand {
    pub id: u64,
    pub confidence: f64,
    pub handedness: f64,
    /// Image pixels, matching the native landmark projection (not normalized XY).
    pub points: [[f64; 2]; 21],
}
#[derive(Default)]
pub struct Detector {
    pair: Option<[u64; 2]>,
    candidate_since: Option<u64>,
    separated_since: Option<u64>,
    last_source: Option<u64>,
    latched: bool,
}
fn distance(a: [f64; 2], b: [f64; 2]) -> f64 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}
fn palm(h: &Hand) -> [f64; 2] {
    [
        (h.points[5][0] + h.points[17][0]) / 2.0,
        (h.points[5][1] + h.points[17][1]) / 2.0,
    ]
}
fn width(h: &Hand) -> f64 {
    distance(h.points[5], h.points[17])
}
fn valid(h: &Hand) -> bool {
    h.handedness.is_finite()
        && (0.0..=1.0).contains(&h.handedness)
        && h.confidence.is_finite()
        && (0.7..=1.0).contains(&h.confidence)
        && h.points.iter().all(|p| {
            p.iter()
                .all(|v| v.is_finite() && (0.0..=8192.0).contains(v))
        })
        && width(h) >= 8.0
}
fn upright(h: &Hand) -> bool {
    let scale = width(h);
    let center = palm(h);
    let wrist = h.points[0];
    wrist[1] - center[1] > scale * 0.4
        && (wrist[0] - center[0]).abs() < scale * 0.8
        && [(5, 8), (9, 12), (13, 16), (17, 20)]
            .iter()
            .all(|(base, tip)| h.points[*base][1] - h.points[*tip][1] > scale * 0.35)
}
fn pose(a: &Hand, b: &Hand) -> bool {
    let scale = (width(a) + width(b)) / 2.0;
    let ac = palm(a);
    let bc = palm(b);
    ((a.handedness <= 0.2 && b.handedness >= 0.8) || (b.handedness <= 0.2 && a.handedness >= 0.8))
        && upright(a)
        && upright(b)
        && width(a).max(width(b)) / width(a).min(width(b)) <= 1.6
        && distance(ac, bc) <= scale * 1.2
        && (ac[1] - bc[1]).abs() <= scale * 0.45
        && distance(a.points[12], b.points[12]) <= scale * 1.2
}
impl Detector {
    /// A single activation requires 1 second of fresh, unambiguous, stable-ID
    /// candidate observations. Only explicit separation rearms a latched pose;
    /// dropout/occlusion never counts as separation.
    pub fn observe(
        &mut self,
        enabled: bool,
        ambiguous: bool,
        hands: &[Hand],
        source_us: u64,
        now_us: u64,
    ) -> bool {
        if !enabled {
            *self = Self::default();
            return false;
        }
        let fresh = source_us <= now_us && now_us - source_us < 150_000;
        let ordered = self.last_source.is_none_or(|last| source_us > last);
        let continuous = self
            .last_source
            .is_none_or(|last| source_us.saturating_sub(last) < 150_000);
        if !fresh
            || !ordered
            || ambiguous
            || hands.len() != 2
            || hands[0].id == hands[1].id
            || !hands.iter().all(valid)
        {
            self.candidate_since = None;
            self.separated_since = None;
            return false;
        }
        let mut pair = [hands[0].id, hands[1].id];
        pair.sort();
        if !continuous || self.pair != Some(pair) {
            self.candidate_since = None;
            self.separated_since = None;
        }
        self.pair = Some(pair);
        self.last_source = Some(source_us);
        let scale = (width(&hands[0]) + width(&hands[1])) / 2.0;
        let separated = distance(palm(&hands[0]), palm(&hands[1])) >= scale * 1.8;
        if self.latched {
            if separated {
                let since = *self.separated_since.get_or_insert(source_us);
                if source_us - since >= 400_000 {
                    self.latched = false;
                    self.separated_since = None;
                }
            } else {
                self.separated_since = None;
            }
            return false;
        }
        if !pose(&hands[0], &hands[1]) {
            self.candidate_since = None;
            return false;
        }
        let since = *self.candidate_since.get_or_insert(source_us);
        if source_us - since >= 1_000_000 {
            self.latched = true;
            self.candidate_since = None;
            return true;
        }
        false
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn hand(id: u64, x: f64) -> Hand {
        let mut points = [[x, 300.0]; 21];
        points[0] = [x, 360.0];
        for (base, tip, offset) in [(5, 8, -20.0), (9, 12, -7.0), (13, 16, 7.0), (17, 20, 20.0)] {
            points[base] = [x + offset, 300.0];
            points[tip] = [x + offset, 230.0];
        }
        Hand {
            id,
            confidence: 0.9,
            handedness: if id == 2 { 1.0 } else { 0.0 },
            points,
        }
    }
    fn pair() -> [Hand; 2] {
        [hand(1, 300.0), hand(2, 340.0)]
    }
    #[test]
    fn held_candidate_activates_once_and_requires_visible_separation() {
        let mut d = Detector::default();
        let pair = pair();
        let mut activations = 0;
        for n in 0..25 {
            activations += usize::from(d.observe(true, false, &pair, n * 100_000, n * 100_000));
        }
        assert_eq!(activations, 1);
        assert!(!d.observe(true, true, &pair, 2_500_000, 2_500_000));
        for n in 26..40 {
            assert!(!d.observe(true, false, &pair, n * 100_000, n * 100_000));
        }
        let apart = [hand(1, 200.0), hand(2, 440.0)];
        for n in 40..46 {
            assert!(!d.observe(true, false, &apart, n * 100_000, n * 100_000));
        }
        let mut activations = 0;
        for n in 46..58 {
            activations += usize::from(d.observe(true, false, &pair, n * 100_000, n * 100_000));
        }
        assert_eq!(activations, 1);
    }
    #[test]
    fn disabled_stale_occluded_low_confidence_fist_and_replaced_identity_do_not_arm() {
        let mut d = Detector::default();
        let mut hands = pair();
        for n in 0..12 {
            assert!(!d.observe(false, false, &hands, n * 100_000, n * 100_000));
        }
        for n in 12..24 {
            assert!(!d.observe(true, false, &hands, n * 100_000, n * 100_000 + 150_000));
        }
        hands[0].confidence = 0.5;
        for n in 24..36 {
            assert!(!d.observe(true, false, &hands, n * 100_000, n * 100_000));
        }
        hands = pair();
        hands[0].points[12][1] = 310.0;
        for n in 36..48 {
            assert!(!d.observe(true, false, &hands, n * 100_000, n * 100_000));
        }
        hands = pair();
        for n in 48..60 {
            hands[0].id = n;
            assert!(!d.observe(true, false, &hands, n * 100_000, n * 100_000));
        }
        assert!(!d.observe(true, false, &hands[..1], 6_000_000, 6_000_000));
        assert!(!d.observe(true, false, &hands, 6_100_000, 6_000_000));
        hands[0].points[0][0] = f64::NAN;
        assert!(!d.observe(true, false, &hands, 6_200_000, 6_200_000));
    }
    #[test]
    fn reversed_array_order_is_stable_but_indeterminate_sides_do_not_activate() {
        let mut detector = Detector::default();
        let hands = pair();
        let mut activations = 0;
        for n in 0..12 {
            let ordered = if n % 2 == 0 {
                [hands[0].clone(), hands[1].clone()]
            } else {
                [hands[1].clone(), hands[0].clone()]
            };
            activations +=
                usize::from(detector.observe(true, false, &ordered, n * 100_000, n * 100_000));
        }
        assert_eq!(activations, 1);
        for side in [0.0, 0.5] {
            let mut detector = Detector::default();
            let mut hands = pair();
            hands[1].handedness = side;
            for n in 0..12 {
                assert!(!detector.observe(true, false, &hands, n * 100_000, n * 100_000));
            }
        }
    }
}
