//! Windows 11 x64 primitives and adapters; construction never starts a session.
#[cfg(feature = "windows-camera")]
pub mod camera;
pub mod display;
pub mod input;
pub(crate) mod process;
pub(crate) mod profile_files;
pub mod writer;
use std::io;
pub(crate) fn validate_camera_identity(camera: &str) -> io::Result<()> {
    // Media Foundation symbolic links are opaque device identifiers, not files.
    // The native adapter resolves an exact match from its enumerated devices.
    if camera.is_empty() || camera.len() > 16_000 || camera.contains('\0') {
        return Err(io::Error::other("invalid Windows camera selection"));
    }
    Ok(())
}
use windows_sys::Win32::System::Performance::{QueryPerformanceCounter, QueryPerformanceFrequency};
/// Same-host QPC epoch, shared by producer and independent writer processes.
pub fn monotonic_us() -> io::Result<u64> {
    let mut count = 0i64;
    let mut frequency = 0i64;
    // SAFETY: both APIs receive valid writable i64 pointers; no retained pointers.
    if unsafe { QueryPerformanceCounter(&mut count) } == 0
        || unsafe { QueryPerformanceFrequency(&mut frequency) } == 0
    {
        return Err(io::Error::last_os_error());
    }
    if count < 0 || frequency <= 0 {
        return Err(io::Error::other("invalid QPC clock"));
    }
    u64::try_from(count as u128 * 1_000_000 / frequency as u128)
        .map_err(|_| io::Error::other("QPC clock overflow"))
}

#[cfg(feature = "browser-provider")]
pub(crate) mod provider_camera;
#[cfg(feature = "browser-provider")]
pub(crate) mod recording_store;
