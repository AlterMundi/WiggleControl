//! Operating-system primitives; portable interaction policy lives outside this layer.
#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(target_os = "windows")]
pub mod windows;
#[cfg(target_os = "linux")]
pub use linux::monotonic_us;
#[cfg(target_os = "linux")]
pub(crate) use linux::profile_files;
#[cfg(target_os = "windows")]
pub use windows::monotonic_us;
#[cfg(target_os = "windows")]
pub(crate) use windows::profile_files;
