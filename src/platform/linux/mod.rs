//! Linux device adapters and operating-system primitives.
use std::io;
pub mod delivery;
pub mod keyboard;
pub mod mouse;
pub(crate) mod profile_files;
pub mod touch;
pub(crate) fn validate_camera_identity(camera: &str) -> io::Result<()> {
    if !std::path::Path::new(camera).is_absolute() {
        return Err(io::Error::other("camera path must be absolute"));
    }
    Ok(())
}
/// Shared Linux monotonic time for same-host producer/device age checks.
pub fn monotonic_us() -> io::Result<u64> {
    let mut ts = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    if unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut ts) } != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(ts.tv_sec as u64 * 1_000_000 + ts.tv_nsec as u64 / 1000)
}

#[cfg(feature = "browser-provider")]
pub(crate) mod camera;
#[cfg(feature = "browser-provider")]
pub(crate) mod provider_output;
pub(crate) mod recording_store;
