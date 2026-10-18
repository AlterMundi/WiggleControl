//! Independent middle-finger pinch evidence from the same hand/frame as index.
use crate::{Config, Hand, MouseButton, Point};
use serde::{Deserialize, Serialize};

/// A second finger measurement, not an alternative estimate of the index pinch.
/// Metric kind/scale must match the primary hand; no confidence can be promoted.
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Evidence {
    pub hand_id: u64,
    pub source_us: u64,
    pub middle_tip: Point,
    #[serde(default)]
    pub pinch_distance_m: Option<f64>,
    #[serde(default)]
    pub pinch_distance_ratio: Option<f64>,
    #[serde(default = "crate::default_onset_allowed")]
    pub onset_allowed: bool,
}
impl Evidence {
    pub fn middle_hand(self, hand: &Hand, source_us: u64) -> Option<Hand> {
        if self.hand_id != hand.id
            || self.source_us != source_us
            || self.pinch_distance_m.is_some() != hand.pinch_distance_m.is_some()
            || self.pinch_distance_ratio.is_some() != hand.pinch_distance_ratio.is_some()
        {
            return None;
        }
        let mut middle = hand.clone();
        middle.index_tip = self.middle_tip;
        middle.pinch_distance_m = self.pinch_distance_m;
        middle.pinch_distance_ratio = self.pinch_distance_ratio;
        middle.pinch_onset_allowed = self.onset_allowed;
        // A second index-crop estimate cannot become middle-finger evidence.
        middle.secondary_pinch = None;
        middle.valid_for(source_us).then_some(middle)
    }
}

/// Clear finger dominance can resolve a nearby non-contact tip estimate.
/// Similarly closed pinches are unknown, not a third chord.
/// During a drag the controller uses its already selected finger until release.
pub(crate) fn selection(
    index: &Hand,
    middle: &Hand,
    config: &Config,
    dominance: f64,
) -> Option<MouseButton> {
    let (index_gap, index_close, _) = index.primary_metric(config);
    let (middle_gap, middle_close, _) = middle.primary_metric(config);
    if (index.open(config, true) && middle.open(config, true))
        || (index_gap <= index_close
            && (middle.open(config, true) || index_gap < middle_gap * dominance))
    {
        Some(MouseButton::Left)
    } else if middle_gap <= middle_close
        && (index.open(config, true) || middle_gap < index_gap * dominance)
    {
        Some(MouseButton::Right)
    } else {
        None
    }
}
