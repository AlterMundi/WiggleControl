//! Windows 11 x64 primitives; these functions never acquire a sensor or inject input.
pub(crate) mod profile_files;
use std::io;
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
