//! Windows store ownership and caller-available bytes; shared budgets remain common.
use std::{
    fs::{self, File, OpenOptions},
    io,
    os::windows::{ffi::OsStrExt, fs::OpenOptionsExt},
    path::Path,
};
use windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
pub(crate) struct Directory {
    _guard: File,
    path: Vec<u16>,
}
impl Directory {
    pub fn lock(path: &Path) -> io::Result<Self> {
        let path = path.canonicalize()?;
        // An open, unshared file is the store-wide take lock. Never remove it:
        // its lifetime is the handle, and removing/recreating could split ownership.
        let guard = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .share_mode(0)
            .open(path.join(".wigglecontrol-recording.lock"))?;
        let mut wide: Vec<u16> = path.as_os_str().encode_wide().collect();
        if wide.contains(&0) {
            return Err(io::Error::other("recording path contains NUL"));
        }
        wide.push(0);
        Ok(Self {
            _guard: guard,
            path: wide,
        })
    }
    pub fn available(&self) -> io::Result<u64> {
        let mut available = 0u64;
        // SAFETY: live NUL-terminated absolute directory and writable u64 output.
        if unsafe {
            GetDiskFreeSpaceExW(
                self.path.as_ptr(),
                &mut available,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        } == 0
        {
            return Err(io::Error::last_os_error());
        }
        Ok(available)
    }
}
pub(crate) fn create_dir(path: &Path, recursive: bool) -> io::Result<()> {
    // The app selects user-local state. New directories inherit that user's ACL.
    fs::DirBuilder::new().recursive(recursive).create(path)
}
