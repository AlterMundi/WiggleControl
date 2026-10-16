//! Explicit local trial evidence. Raw camera bytes and human labels never enter Git.
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Seek, SeekFrom, Write},
    os::unix::fs::{DirBuilderExt, OpenOptionsExt},
    path::{Path, PathBuf},
};
pub const MAX_BYTES: u64 = 512 * 1024 * 1024;
#[derive(Clone, Serialize, Deserialize)]
pub struct FrameIndex {
    #[serde(default)]
    pub segment: u32,
    pub source_us: u64,
    pub sequence: u32,
    pub offset: u64,
    pub length: usize,
}

/// Read-only indexed access to retained bytes, shared by review and inference replay.
pub struct Reader {
    rows: Vec<FrameIndex>,
    videos: std::collections::BTreeMap<u32, File>,
}
impl Reader {
    pub fn open(dir: &Path) -> io::Result<Self> {
        let root = dir.canonicalize()?;
        let open = |name: &str, maximum: u64| -> io::Result<File> {
            let path = root.join(name).canonicalize()?;
            if !path.starts_with(&root) {
                return Err(io::Error::other(
                    "recording file escapes selected directory",
                ));
            }
            let file = File::open(path)?;
            let metadata = file.metadata()?;
            if !metadata.is_file() || metadata.len() > maximum {
                return Err(io::Error::other("invalid or oversized recording file"));
            }
            Ok(file)
        };
        let maximum = 16 * 1024 * 1024;
        let mut raw = Vec::new();
        open("frames.jsonl", maximum)?
            .take(maximum + 1)
            .read_to_end(&mut raw)?;
        if raw.len() as u64 > maximum {
            return Err(io::Error::other("recording index too large"));
        }
        let rows: Vec<FrameIndex> = String::from_utf8(raw)
            .map_err(io::Error::other)?
            .lines()
            .map(serde_json::from_str)
            .collect::<Result<_, _>>()?;
        if rows.is_empty()
            || rows.len() > 100_000
            || rows
                .windows(2)
                .any(|pair| pair[1].source_us <= pair[0].source_us)
        {
            return Err(io::Error::other("invalid recording index"));
        }
        let mut videos = std::collections::BTreeMap::new();
        for row in &rows {
            if row.segment > 1000 {
                return Err(io::Error::other("invalid recording segment"));
            }
            if let std::collections::btree_map::Entry::Vacant(entry) = videos.entry(row.segment) {
                let name = if row.segment == 0 {
                    "video.mjpeg".into()
                } else {
                    format!("video-{}.mjpeg", row.segment)
                };
                entry.insert(open(&name, MAX_BYTES)?);
            }
            if row.length == 0
                || row.length > 2 * 1024 * 1024
                || row.offset.checked_add(row.length as u64).is_none_or(|end| {
                    end > videos[&row.segment]
                        .metadata()
                        .map(|m| m.len())
                        .unwrap_or(0)
                })
            {
                return Err(io::Error::other("recording frame exceeds segment bounds"));
            }
        }
        Ok(Self { rows, videos })
    }
    pub fn rows(&self) -> &[FrameIndex] {
        &self.rows
    }
    pub fn frame(&mut self, index: usize) -> io::Result<Vec<u8>> {
        let row = self
            .rows
            .get(index)
            .ok_or_else(|| io::Error::other("recording frame absent"))?;
        let video = self.videos.get_mut(&row.segment).unwrap();
        let mut bytes = vec![0; row.length];
        video.seek(SeekFrom::Start(row.offset))?;
        video.read_exact(&mut bytes)?;
        Ok(bytes)
    }
}
pub struct Frames {
    dir: PathBuf,
    segment: u32,
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
            dir: dir.clone(),
            segment: 0,
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
            || self.last.is_some_and(|s| source_us <= s)
        {
            return Err(io::Error::other("invalid or exhausted trial recording"));
        }
        if self.offset + bytes.len() as u64 > MAX_BYTES {
            self.segment = self
                .segment
                .checked_add(1)
                .ok_or_else(|| io::Error::other("recording segment overflow"))?;
            self.video = private_file(&self.dir.join(format!("video-{}.mjpeg", self.segment)))?;
            self.offset = 0;
        }
        self.video.write_all(bytes)?;
        serde_json::to_writer(
            &mut self.index,
            &FrameIndex {
                segment: self.segment,
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
    fn indexed_reader_preserves_rotated_bytes_timestamps_and_kernel_sequences() {
        let base = std::env::temp_dir().join(format!("wiggle-reader-{}", std::process::id()));
        let (mut frames, observations, dir) = create(&base, 11, &serde_json::json!({})).unwrap();
        frames.append(b"first", 1_000, 15).unwrap();
        frames.offset = MAX_BYTES;
        frames.append(b"second", 68_000, 19).unwrap();
        drop(frames);
        drop(observations);
        let original = fs::read(dir.join("frames.jsonl")).unwrap();
        let mut reader = Reader::open(&dir).unwrap();
        assert_eq!(reader.frame(0).unwrap(), b"first");
        assert_eq!(reader.frame(1).unwrap(), b"second");
        assert_eq!(
            (
                reader.rows()[1].source_us,
                reader.rows()[1].sequence,
                reader.rows()[1].segment
            ),
            (68_000, 19, 1)
        );
        assert!(reader.frame(2).is_err());
        assert_eq!(fs::read(dir.join("frames.jsonl")).unwrap(), original);
        drop(reader);
        fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn indexed_reader_rejects_out_of_range_bytes_and_external_segment_symlinks() {
        let base =
            std::env::temp_dir().join(format!("wiggle-reader-bounds-{}", std::process::id()));
        let (mut frames, observations, dir) = create(&base, 12, &serde_json::json!({})).unwrap();
        frames.append(b"data", 1_000, 1).unwrap();
        drop(frames);
        drop(observations);
        let original = fs::read(dir.join("frames.jsonl")).unwrap();
        let row = FrameIndex {
            segment: 0,
            source_us: 1_000,
            sequence: 1,
            offset: u64::MAX,
            length: 4,
        };
        fs::write(dir.join("frames.jsonl"), serde_json::to_vec(&row).unwrap()).unwrap();
        assert!(Reader::open(&dir).is_err());
        fs::write(dir.join("frames.jsonl"), original).unwrap();
        fs::rename(dir.join("video.mjpeg"), base.join("outside.mjpeg")).unwrap();
        std::os::unix::fs::symlink(base.join("outside.mjpeg"), dir.join("video.mjpeg")).unwrap();
        assert!(Reader::open(&dir).is_err());
        fs::remove_dir_all(base).unwrap();
    }
    #[test]
    fn continued_recording_rotates_without_overwriting_or_stopping() {
        let base = std::env::temp_dir().join(format!("wiggle-rotate-{}", std::process::id()));
        let (mut frames, _, dir) = create(&base, 2, &serde_json::json!({})).unwrap();
        frames.append(b"first", 100, 1).unwrap();
        frames.offset = MAX_BYTES;
        frames.append(b"second", 200, 2).unwrap();
        drop(frames);
        assert_eq!(fs::read(dir.join("video.mjpeg")).unwrap(), b"first");
        assert_eq!(fs::read(dir.join("video-1.mjpeg")).unwrap(), b"second");
        let rows: Vec<FrameIndex> = fs::read_to_string(dir.join("frames.jsonl"))
            .unwrap()
            .lines()
            .map(|s| serde_json::from_str(s).unwrap())
            .collect();
        assert_eq!(rows[1].segment, 1);
        assert_eq!(rows[1].offset, 0);
        fs::remove_dir_all(base).unwrap();
    }
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
