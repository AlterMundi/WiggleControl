//! Owned encoded acquisition payload; no camera or file is opened by this type.
pub struct Frame {
    pub bytes: Vec<u8>,
    /// Same-host monotonic microseconds, retained from the acquisition driver.
    pub source_us: u64,
    pub sequence: u32,
}
