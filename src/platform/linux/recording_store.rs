//! Filesystem mechanics only; the shared recorder owns all retention budgets.
use std::{
    fs::{self, File},
    io,
    os::{fd::AsRawFd, unix::fs::DirBuilderExt},
    path::Path,
};
pub(crate) struct Directory(File);
impl Directory {
    pub fn lock(path: &Path) -> io::Result<Self> {
        let directory = File::open(path)?;
        // Held for the entire take, matching the existing Linux store contract.
        if unsafe { libc::flock(directory.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
            return Err(io::Error::other(
                "recording store already has an active take",
            ));
        }
        Ok(Self(directory))
    }
    pub fn available(&self) -> io::Result<u64> {
        let mut stat = std::mem::MaybeUninit::<libc::statvfs>::uninit();
        if unsafe { libc::fstatvfs(self.0.as_raw_fd(), stat.as_mut_ptr()) } != 0 {
            return Err(io::Error::last_os_error());
        }
        let stat = unsafe { stat.assume_init() };
        u64::try_from(u128::from(stat.f_bavail) * u128::from(stat.f_frsize))
            .map_err(io::Error::other)
    }
}
pub(crate) fn create_dir(path: &Path, recursive: bool) -> io::Result<()> {
    fs::DirBuilder::new()
        .recursive(recursive)
        .mode(0o700)
        .create(path)
}
