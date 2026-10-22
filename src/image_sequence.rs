//! Bounded camera-free source manifest shared by native and browser replay.
use serde::Deserialize;
use std::{
    error::Error,
    fs,
    path::{Component, Path},
};
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Sequence {
    version: u32,
    pub frames: Vec<ImageFrame>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ImageFrame {
    pub source_us: u64,
    /// None is explicit missing acquisition, not an invented blank hand image.
    pub image: Option<String>,
}
impl Sequence {
    pub(crate) fn load(path: &Path) -> Result<Self, Box<dyn Error>> {
        if fs::metadata(path)?.len() > 1_048_576 {
            return Err("sequence exceeds 1 MiB".into());
        }
        let s: Self = serde_json::from_slice(&fs::read(path)?)?;
        if s.version != 1 || s.frames.is_empty() || s.frames.len() > 10_000 {
            return Err("sequence version 1 requires 1..10000 frames".into());
        }
        let mut last = None;
        for frame in &s.frames {
            if last.is_some_and(|t| frame.source_us <= t) {
                return Err("sequence source time must increase".into());
            }
            last = Some(frame.source_us);
            if let Some(name) = &frame.image {
                if name.is_empty()
                    || Path::new(name)
                        .components()
                        .any(|p| !matches!(p, Component::Normal(_)))
                {
                    return Err(
                        "sequence image must be a relative path without dot/parent components"
                            .into(),
                    );
                }
            }
        }
        Ok(s)
    }
}
