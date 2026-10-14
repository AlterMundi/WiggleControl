//! Explicit local trial evidence. Raw camera bytes and human labels never enter Git.
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Write},
    os::unix::fs::{DirBuilderExt, OpenOptionsExt},
    path::{Path, PathBuf},
};
pub const MAX_BYTES: u64 = 512 * 1024 * 1024;
#[derive(Clone, Serialize, Deserialize)]
pub struct FrameIndex {
    pub source_us: u64,
    pub sequence: u32,
    pub offset: u64,
    pub length: usize,
}
pub struct Frames {
    video: File,
    index: File,
    offset: u64,
    last: Option<u64>,
}
pub fn private_file(path: &Path) -> io::Result<File> {
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
}
pub fn create(
    base: &Path,
    stamp: u64,
    manifest: &serde_json::Value,
) -> io::Result<(Frames, File, PathBuf)> {
    fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(base)?;
    let dir = base.join(format!("trial-{stamp}-{}", std::process::id()));
    fs::DirBuilder::new().mode(0o700).create(&dir)?;
    let video = private_file(&dir.join("video.mjpeg"))?;
    let index = private_file(&dir.join("frames.jsonl"))?;
    let observations = private_file(&dir.join("observations.jsonl"))?;
    serde_json::to_writer_pretty(private_file(&dir.join("manifest.json"))?, manifest)?;
    Ok((
        Frames {
            video,
            index,
            offset: 0,
            last: None,
        },
        observations,
        dir,
    ))
}
impl Frames {
    pub fn append(&mut self, bytes: &[u8], source_us: u64, sequence: u32) -> io::Result<()> {
        if bytes.is_empty()
            || bytes.len() > 2 * 1024 * 1024
            || self.offset + bytes.len() as u64 > MAX_BYTES
            || self.last.is_some_and(|s| source_us <= s)
        {
            return Err(io::Error::other("invalid or exhausted trial recording"));
        }
        self.video.write_all(bytes)?;
        serde_json::to_writer(
            &mut self.index,
            &FrameIndex {
                source_us,
                sequence,
                offset: self.offset,
                length: bytes.len(),
            },
        )?;
        writeln!(self.index)?;
        self.offset += bytes.len() as u64;
        self.last = Some(source_us);
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn recording_retains_dropped_frame_evidence_and_rejects_overwrite_or_clock_regression() {
        let base = std::env::temp_dir().join(format!("wiggle-recording-{}", std::process::id()));
        let (mut frames, _, dir) = create(
            &base,
            1,
            &serde_json::json!({"schema":"wigglecontrol.recording/v1"}),
        )
        .unwrap();
        frames.append(b"first", 100, 1).unwrap();
        frames.append(b"second", 200, 3).unwrap();
        assert!(frames.append(b"bad", 200, 4).is_err());
        assert!(create(&base, 1, &serde_json::json!({})).is_err());
        drop(frames);
        assert_eq!(fs::read(dir.join("video.mjpeg")).unwrap(), b"firstsecond");
        let rows: Vec<FrameIndex> = fs::read_to_string(dir.join("frames.jsonl"))
            .unwrap()
            .lines()
            .map(|s| serde_json::from_str(s).unwrap())
            .collect();
        assert_eq!(rows[1].offset, 5);
        assert_eq!(rows[1].sequence, 3);
        fs::remove_dir_all(base).unwrap();
    }
}
