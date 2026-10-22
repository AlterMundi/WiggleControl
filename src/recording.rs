//! Explicit local trial evidence. Raw camera bytes and human labels never enter Git.
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    io::{self, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
pub const MAX_BYTES: u64 = 512 * 1024 * 1024;

/// Dataset samples are explicit, finite and small; ordinary input never records.
pub const MAX_TAKE_BYTES: u64 = 256 * 1024 * 1024;
pub const MAX_DIRECTORY_BYTES: u64 = 2 * 1024 * 1024 * 1024;
pub const MIN_FREE_BYTES: u64 = 2 * 1024 * 1024 * 1024;
struct Budget {
    used: u64,
    remaining: u64,
    began: Instant,
    directory: crate::platform::recording_store::Directory,
    exhausted: bool,
}
impl Budget {
    fn check(&mut self, requested: u64, available: u64) -> io::Result<()> {
        if self.exhausted
            || requested > self.remaining
            || self.used.saturating_add(requested) > MAX_TAKE_BYTES
            || available < MIN_FREE_BYTES.saturating_add(requested)
            || self.began.elapsed() >= Duration::from_secs(30)
        {
            self.exhausted = true;
            return Err(io::Error::other("dataset recording stopped: duration, storage budget or free-space reserve reached; retained data preserved"));
        }
        Ok(())
    }
    fn available(&self) -> io::Result<u64> {
        self.directory.available()
    }
}
/// Shares the take/directory budget across video, frame index and observations.
pub struct Output {
    file: File,
    budget: Arc<Mutex<Budget>>,
}
impl Write for Output {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let mut budget = self
            .budget
            .lock()
            .map_err(|_| io::Error::other("recording budget unavailable"))?;
        let available = budget.available()?;
        budget.check(bytes.len() as u64, available)?;
        let written = self.file.write(bytes)?;
        budget.used += written as u64;
        budget.remaining -= written as u64;
        Ok(written)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }
}
fn retained_bytes(root: &Path) -> io::Result<u64> {
    let mut bytes = 0u64;
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        for item in fs::read_dir(dir)? {
            let item = item?;
            let metadata = item.path().symlink_metadata()?;
            if metadata.is_dir() {
                pending.push(item.path());
            } else if metadata.is_file() {
                bytes = bytes.saturating_add(metadata.len());
            } else {
                return Err(io::Error::other(
                    "recording store contains unsupported entry",
                ));
            }
            if bytes >= MAX_DIRECTORY_BYTES {
                return Ok(bytes);
            }
        }
    }
    Ok(bytes)
}

/// Encode before touching an unbuffered file/pipe: serde's streaming serializer
/// otherwise turns the JSON fields into many separate writes on the hot path.
pub(crate) fn json_line(value: &impl Serialize) -> io::Result<Vec<u8>> {
    let mut line = serde_json::to_vec(value)?;
    line.push(b'\n');
    Ok(line)
}
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
    video: Output,
    index: Output,
    offset: u64,
    last: Option<u64>,
}
pub fn private_file(path: &Path) -> io::Result<File> {
    crate::platform::profile_files::create_new(path)
}
pub fn create(
    base: &Path,
    stamp: u64,
    manifest: &serde_json::Value,
) -> io::Result<(Frames, Output, PathBuf)> {
    crate::platform::recording_store::create_dir(base, true)?;
    let directory = crate::platform::recording_store::Directory::lock(base)?;
    let existing = retained_bytes(base)?;
    let mut budget = Budget {
        used: 0,
        remaining: MAX_DIRECTORY_BYTES.saturating_sub(existing),
        began: Instant::now(),
        directory,
        exhausted: false,
    };
    let available = budget.available()?;
    budget.check(1, available)?;
    let budget = Arc::new(Mutex::new(budget));
    let output = |path: &Path| -> io::Result<Output> {
        Ok(Output {
            file: private_file(path)?,
            budget: budget.clone(),
        })
    };
    let dir = base.join(format!("trial-{stamp}-{}", std::process::id()));
    crate::platform::recording_store::create_dir(&dir, false)?;
    let video = output(&dir.join("video.mjpeg"))?;
    let index = output(&dir.join("frames.jsonl"))?;
    let observations = output(&dir.join("observations.jsonl"))?;
    serde_json::to_writer_pretty(output(&dir.join("manifest.json"))?, manifest)?;
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
    pub fn metadata(&self, name: &str, bytes: &[u8]) -> io::Result<()> {
        if !matches!(name, "config.json" | "controller-settings.json") || bytes.len() > 64 * 1024 {
            return Err(io::Error::other("invalid recording metadata"));
        }
        Output {
            file: private_file(&self.dir.join(name))?,
            budget: self.video.budget.clone(),
        }
        .write_all(bytes)
    }
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
            self.video = Output {
                file: private_file(&self.dir.join(format!("video-{}.mjpeg", self.segment)))?,
                budget: self.video.budget.clone(),
            };
            self.offset = 0;
        }
        self.video.write_all(bytes)?;
        self.index.write_all(&json_line(&FrameIndex {
            segment: self.segment,
            source_us,
            sequence,
            offset: self.offset,
            length: bytes.len(),
        })?)?;
        self.offset += bytes.len() as u64;
        self.last = Some(source_us);
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::OpenOptions;

    #[test]
    fn dataset_budget_is_shared_latched_and_checks_free_reserve_and_duration() {
        let base = std::env::temp_dir().join(format!("wiggle-budget-{}", std::process::id()));
        let (mut frames, mut observations, dir) =
            create(&base, 31, &serde_json::json!({})).unwrap();
        let budget = frames.video.budget.clone();
        budget.lock().unwrap().used = MAX_TAKE_BYTES - 2;
        let before = fs::read(dir.join("observations.jsonl")).unwrap();
        assert!(observations.write_all(b"too much").is_err());
        assert!(frames.append(b"x", 100, 1).is_err());
        assert_eq!(fs::read(dir.join("observations.jsonl")).unwrap(), before);
        assert_eq!(fs::metadata(dir.join("video.mjpeg")).unwrap().len(), 0);
        assert!(create(&base, 32, &serde_json::json!({})).is_err());
        let mut guard = budget.lock().unwrap();
        guard.exhausted = false;
        guard.used = 0;
        assert!(guard.check(2, MIN_FREE_BYTES + 1).is_err());
        guard.exhausted = false;
        guard.began = Instant::now() - Duration::from_secs(31);
        assert!(guard.check(1, u64::MAX).is_err());
        drop(guard);
        drop(budget);
        drop(frames);
        drop(observations);
        fs::remove_dir_all(base).unwrap();
    }
    #[test]
    fn full_dataset_store_refuses_new_take_without_deleting_retained_evidence() {
        let base = std::env::temp_dir().join(format!("wiggle-store-budget-{}", std::process::id()));
        fs::create_dir_all(&base).unwrap();
        let retained = base.join("retained.mjpeg");
        let file = private_file(&retained).unwrap();
        file.set_len(MAX_DIRECTORY_BYTES).unwrap();
        assert!(create(&base, 33, &serde_json::json!({})).is_err());
        assert_eq!(fs::metadata(retained).unwrap().len(), MAX_DIRECTORY_BYTES);
        #[cfg(target_os = "linux")]
        assert_eq!(fs::read_dir(&base).unwrap().count(), 1);
        fs::remove_dir_all(base).unwrap();
    }

    struct Counting<W> {
        inner: W,
        writes: usize,
    }
    impl<W: Write> Write for Counting<W> {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.writes += 1;
            self.inner.write(bytes)
        }
        fn flush(&mut self) -> io::Result<()> {
            self.inner.flush()
        }
    }
    fn sample() -> serde_json::Value {
        serde_json::json!({"event":"frame","source_us":1000001,"now_us":1100001,
            "controller":{"button_down":true,"cursor":{"x":0.23456789,"y":0.87654321}},
            "perception":[{"landmarks":vec![[0.1234,0.5678];21],"world_landmarks":vec![[0.01,-0.02,0.03];21]}],
            "message":"unicode → and escaped\ntext"})
    }
    #[test]
    fn json_lines_preserve_wire_bytes_and_coalesce_unbuffered_writes() {
        let value = sample();
        let mut previous = Counting {
            inner: Vec::new(),
            writes: 0,
        };
        writeln!(previous, "{value}").unwrap();
        let mut current = Counting {
            inner: Vec::new(),
            writes: 0,
        };
        current.write_all(&json_line(&value).unwrap()).unwrap();
        assert_eq!(previous.inner, current.inner);
        assert!(previous.writes > 100);
        assert_eq!(current.writes, 1);
        let row = FrameIndex {
            segment: 3,
            source_us: 123456,
            sequence: 4,
            offset: 1024,
            length: 2048,
        };
        let mut previous = Counting {
            inner: Vec::new(),
            writes: 0,
        };
        serde_json::to_writer(&mut previous, &row).unwrap();
        writeln!(previous).unwrap();
        assert_eq!(previous.inner, json_line(&row).unwrap());
        assert!(previous.writes > 1);
    }

    #[test]
    #[ignore = "explicit finite camera-free serialization/write-call benchmark"]
    fn write_call_benchmark() {
        let value = sample();
        let events = 1000;
        for paired_run in 0..3 {
            for coalesced in if paired_run % 2 == 0 {
                [false, true]
            } else {
                [true, false]
            } {
                let mut sink = Counting {
                    inner: OpenOptions::new()
                        .write(true)
                        .open(if cfg!(target_os = "windows") {
                            "NUL"
                        } else {
                            "/dev/null"
                        })
                        .unwrap(),
                    writes: 0,
                };
                let began = std::time::Instant::now();
                for _ in 0..events {
                    if coalesced {
                        let line = json_line(&value).unwrap();
                        sink.write_all(&line).unwrap();
                        sink.write_all(&line).unwrap();
                    } else {
                        writeln!(sink, "{value}").unwrap();
                        writeln!(sink, "{value}").unwrap();
                    }
                }
                println!("{{\"paired_run\":{paired_run},\"coalesced\":{coalesced},\"events\":{events},\"writes\":{},\"elapsed_us\":{}}}",sink.writes,began.elapsed().as_micros());
            }
        }
    }
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
        assert!(Reader::open(&dir).is_err()); // Missing indexed segment on both OSes.
        #[cfg(target_os = "linux")]
        {
            std::os::unix::fs::symlink(base.join("outside.mjpeg"), dir.join("video.mjpeg"))
                .unwrap();
            assert!(Reader::open(&dir).is_err());
        }
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
