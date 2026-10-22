//! Operating-system primitives; portable interaction policy lives outside this layer.
#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(target_os = "windows")]
pub mod windows;
#[cfg(target_os = "linux")]
pub use linux::monotonic_us;
#[cfg(target_os = "linux")]
pub(crate) use linux::profile_files;
#[cfg(target_os = "linux")]
pub(crate) use linux::validate_camera_identity;
#[cfg(target_os = "windows")]
pub use windows::monotonic_us;
#[cfg(target_os = "windows")]
pub(crate) use windows::profile_files;
#[cfg(target_os = "windows")]
pub(crate) use windows::validate_camera_identity;

#[cfg(all(target_os = "linux", feature = "browser-provider"))]
pub(crate) use linux::camera::camera;
#[cfg(target_os = "linux")]
pub(crate) use linux::recording_store;
#[cfg(all(target_os = "windows", feature = "browser-provider"))]
pub(crate) use windows::provider_camera::camera;
#[cfg(all(target_os = "windows", feature = "browser-provider"))]
pub(crate) use windows::recording_store;
