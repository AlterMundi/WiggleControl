//! Common configured observation/gesture policy; loading never opens sensors or models.
use crate::{Config, Engine};
use serde::{Deserialize, Serialize};
use std::{error::Error, fs, path::Path};

#[derive(Clone, Copy, Default, PartialEq, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PalmDeduplication {
    #[default]
    Roi,
    PalmLandmarks,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SecondaryLandmarkModel {
    pub path: String,
    pub min_presence: f64,
    pub source_id: u64,
}

pub use crate::landmarks::Pinch2dReference;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionConfig {
    pub camera: String,
    /// Native MJPEG capture size; provider geometry and recording retain these pixels.
    #[serde(default = "default_camera_resolution")]
    pub camera_resolution: [u32; 2],
    /// Required by native inference; a browser provider need not install ONNX.
    #[serde(default)]
    pub models: String,
    #[serde(default)]
    pub runtime: String,
    pub fps: u32,
    pub gain: f64,
    #[serde(default = "default_freshness_ms")]
    pub freshness_ms: u64,
    /// Zero explicitly selects single-observation onset; default remains temporal.
    #[serde(default = "default_press_confirm_us")]
    pub press_confirm_us: u64,
    #[serde(default)]
    pub pinch_3d: bool,
    /// Explicit projected metric experiment. Never an estimate in meters.
    #[serde(default)]
    pub pinch_2d_reference: Pinch2dReference,
    /// Experimental veto of new presses using pixel tip-gap / wrist-middle-MCP.
    /// The raw pinch metric still owns confirmation, release and open rearm.
    #[serde(default)]
    pub pinch_onset_max_screen_ratio: Option<f64>,
    #[serde(default = "default_close_m")]
    pub pinch_close_m: f64,
    #[serde(default = "default_open_m")]
    pub pinch_open_m: f64,
    #[serde(default)]
    pub recording_directory: Option<String>,
    /// Explicit finite dataset capture; a directory alone never enables recording.
    #[serde(default)]
    pub recording_enabled: bool,
    #[serde(default)]
    pub pointer_output: Option<[u32; 2]>,
    #[serde(default)]
    pub prayer_enabled: bool,
    #[serde(default)]
    pub touch_workspace: crate::touch_gestures::Workspace,
    #[serde(default = "default_close_ratio")]
    pub pinch_close_ratio: f64,
    #[serde(default = "default_open_ratio")]
    pub pinch_open_ratio: f64,
    #[serde(default)]
    pub adaptive_smoothing: Option<crate::filter::OneEuroConfig>,
    #[serde(default = "default_pair_separation_widths")]
    pub pair_separation_widths: f64,
    #[serde(default)]
    pub palm_deduplication: PalmDeduplication,
    #[serde(default)]
    pub dual_crop: bool,
    #[serde(default)]
    pub secondary_model: Option<SecondaryLandmarkModel>,
    #[serde(default)]
    pub gesture_min_confidence: Option<f64>,
    /// Opt-in raw-provider quality export; native inference already has presence scores.
    #[serde(default)]
    pub provider_presence_scores: bool,
    /// Calibrated one-hand provider mouse; pointer/scroll only.
    #[serde(default)]
    pub provider_absolute_pointer: bool,
    /// Experimental one-hand drag continuity; independent of source freshness.
    #[serde(default)]
    pub pointer_drag_grace_ms: u64,
    #[serde(default)]
    pub pointer_release_confirmation: Option<crate::ReleaseConfig>,
    #[serde(default)]
    pub pointer_cursor_output: Option<crate::cursor::Config>,
}
fn default_camera_resolution() -> [u32; 2] {
    [640, 480]
}
pub(crate) fn validate_camera_resolution(size: [u32; 2]) -> Result<(), &'static str> {
    if !(16..=1920).contains(&size[0]) || !(16..=1080).contains(&size[1]) {
        return Err("camera_resolution must be within16x16..1920x1080");
    }
    Ok(())
}
fn default_pair_separation_widths() -> f64 {
    0.8
}
fn default_close_m() -> f64 {
    0.02
}
fn default_open_m() -> f64 {
    0.03
}
fn default_freshness_ms() -> u64 {
    150
}
fn default_press_confirm_us() -> u64 {
    Config::default().confirm_us
}
fn default_close_ratio() -> f64 {
    Config::default().close_ratio
}
fn default_open_ratio() -> f64 {
    Config::default().open_ratio
}
impl SessionConfig {
    pub fn validate_recording(&self, seconds: Option<u64>) -> Result<(), Box<dyn Error>> {
        if self.recording_enabled
            && (self.recording_directory.is_none()
                || seconds.is_none_or(|s| !(1..=30).contains(&s)))
        {
            return Err("dataset recording requires an explicit directory and finite 1..30 second trial; ordinary/until-stop use never records".into());
        }
        Ok(())
    }

    pub fn pinch_metric_name(&self) -> &'static str {
        if self.pinch_3d {
            "estimated_world_meters"
        } else if self.pinch_2d_reference == Pinch2dReference::WristMiddleMcp {
            "projected_wrist_middle_mcp"
        } else {
            "projected_palm_width"
        }
    }
    pub fn gestures(&self) -> Config {
        Config {
            min_confidence: self
                .gesture_min_confidence
                .unwrap_or(Config::default().min_confidence),
            stale_us: self.freshness_ms * 1000,
            close_m: self.pinch_close_m,
            open_m: self.pinch_open_m,
            close_ratio: self.pinch_close_ratio,
            open_ratio: self.pinch_open_ratio,
            adaptive_smoothing: self.adaptive_smoothing,
            drag_grace_us: self.pointer_drag_grace_ms.saturating_mul(1000),
            confirm_us: self.press_confirm_us,
            release_confirmation: self.pointer_release_confirmation,
            ..Config::default()
        }
    }
    pub fn interpretation(&self) -> crate::landmarks::Interpretation {
        crate::landmarks::Interpretation {
            pinch_3d: self.pinch_3d,
            pinch_2d_reference: self.pinch_2d_reference,
            pinch_onset_max_screen_ratio: self.pinch_onset_max_screen_ratio,
            dual_crop: self.dual_crop,
        }
    }
    #[cfg(feature = "provider-runtime")]
    pub(crate) fn observation(
        &self,
        detected: Option<&crate::landmarks::DetectedHand>,
        width: u32,
        height: u32,
        source_us: u64,
    ) -> crate::Observation {
        self.interpretation()
            .observation(detected, width, height, source_us)
    }
    pub fn load(path: &Path) -> Result<Self, Box<dyn Error>> {
        Self::load_backend(path, false)
    }
    /// Browser-supplied landmarks use the same camera/gesture configuration but
    /// do not require native hand models or ONNX Runtime. Optional visual macros
    /// validate their separate runtime requirement in provider::configured.
    #[cfg(feature = "provider-runtime")]
    pub(crate) fn load_provider(path: &Path) -> Result<Self, Box<dyn Error>> {
        Self::load_backend(path, true)
    }
    fn load_backend(path: &Path, provider: bool) -> Result<Self, Box<dyn Error>> {
        let data = fs::read(path)?;
        if data.len() > 8192 {
            return Err("session config too large".into());
        }
        let config: Self = serde_json::from_slice(&data)?;
        validate_camera_resolution(config.camera_resolution)?;
        if let Some(cursor) = config.pointer_cursor_output {
            cursor.validate()?;
            if !config.provider_absolute_pointer {
                return Err("cursor output requires the explicit absolute provider".into());
            }
        }
        if config.pointer_drag_grace_ms > 0 && !config.provider_absolute_pointer {
            return Err("pointer drag grace requires the explicit absolute provider".into());
        }
        if config.pointer_release_confirmation.is_some() && !config.provider_absolute_pointer {
            return Err("release confirmation requires the explicit absolute provider".into());
        }
        if config.pinch_3d && config.pinch_2d_reference != Pinch2dReference::PalmWidth {
            return Err("projected pinch reference requires pinch_3d=false".into());
        }
        if config.dual_crop && !config.pinch_3d {
            return Err("dual crop requires the explicit3D pinch metric".into());
        }
        if let Some(secondary) = &config.secondary_model {
            if !config.dual_crop
                || !Path::new(&secondary.path).is_absolute()
                || !secondary.min_presence.is_finite()
                || !(0.0..=1.0).contains(&secondary.min_presence)
                || secondary.source_id <= crate::landmarks::PROXIMAL_PINCH_SOURCE
                || config.gestures().min_confidence > secondary.min_presence
            {
                return Err("secondary model requires dual3D, absolute path, distinct source ID and an explicit compatible gesture confidence threshold".into());
            }
        }
        if config
            .pinch_onset_max_screen_ratio
            .is_some_and(|r| !r.is_finite() || r <= 0.0 || r > 2.0)
        {
            return Err("pinch onset screen ratio must be finite and0 < ratio <= 2".into());
        }
        if !(0.8..=2.0).contains(&config.pair_separation_widths) {
            return Err("pair separation must be finite and0.8..2.0 palm widths".into());
        }
        config.touch_workspace.validate()?;
        if let Some(output) = config.pointer_output {
            crate::calibration::Profile {
                version: 1,
                workspace: Default::default(),
                output,
            }
            .validate()?;
        }
        if !(50..=1000).contains(&config.freshness_ms) {
            return Err("freshness_ms must be 50..1000".into());
        }
        Engine::new(config.gestures())?;
        if config.pinch_open_ratio > 1.0 {
            return Err("pinch ratios must satisfy 0 < close < open <= 1".into());
        }
        if !(10..=20).contains(&config.fps)
            || !config.gain.is_finite()
            || !(1.0..=10000.0).contains(&config.gain)
        {
            return Err("fps 10..20; gain 1..10000".into());
        }
        if config
            .recording_directory
            .as_ref()
            .is_some_and(|p| !Path::new(p).is_absolute())
        {
            return Err("recording_directory must be absolute".into());
        }
        if !Path::new(&config.camera).is_absolute() {
            return Err("camera path must be absolute".into());
        }
        for p in [&config.models, &config.runtime] {
            if p.is_empty() {
                if !provider {
                    return Err(
                        "native inference requires explicit models and runtime paths".into(),
                    );
                }
            } else if !Path::new(p).is_absolute() {
                return Err("receiving paths must be absolute".into());
            }
        }
        Ok(config)
    }
}
/// Shared pre-acquisition validation; incompatible gesture authorities are refused.
pub fn controller_config(
    config: &SessionConfig,
    path: &Path,
) -> Result<crate::controller::Controller, Box<dyn Error>> {
    let settings = crate::controller::Settings::load(path)?;
    if serde_json::to_value(settings.cursor_output)?
        != serde_json::to_value(config.pointer_cursor_output)?
    {
        return Err("controller cursor output settings must match the session".into());
    }
    if settings.detail.is_some() && !config.provider_absolute_pointer {
        return Err("held detail requires calibrated absolute provider output".into());
    }
    if settings.right_click && (config.dual_crop || config.secondary_model.is_some()) {
        return Err("right-click gestures require a single estimate per finger".into());
    }
    if config.prayer_enabled {
        return Err("prayer requires its separate calibrated workflow".into());
    }
    if serde_json::to_value(settings.gestures)? != serde_json::to_value(config.gestures())? {
        return Err("controller gesture settings must match the session".into());
    }
    if serde_json::to_value(settings.workspace)? != serde_json::to_value(config.touch_workspace)? {
        return Err("controller workspace settings must match the session".into());
    }
    if config.provider_absolute_pointer
        && (!matches!(
            settings.initial_mode,
            crate::controller::Mode::Pointer | crate::controller::Mode::Scroll
        ) || settings.macros.is_some()
            || settings.visual_macros.is_some())
    {
        return Err("absolute provider requires pointer/scroll without macros".into());
    }
    Ok(crate::controller::Controller::new(settings)?)
}
/// Compatibility entry for native/provider/sequence callers; interpretation is portable.
#[cfg(feature = "provider-runtime")]
pub(crate) fn observe_controller(
    config: &SessionConfig,
    controller: &mut crate::controller::Controller,
    now: u64,
    observation: crate::touch_gestures::Frame,
    pose: Option<crate::macros::HandPose>,
    detected: &[crate::landmarks::DetectedHand],
    size: [u32; 2],
) -> crate::controller::Output {
    crate::landmarks::observe_controller(
        &config.interpretation(),
        controller,
        now,
        observation,
        pose,
        detected,
        size,
    )
}
