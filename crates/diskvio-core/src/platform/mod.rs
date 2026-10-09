#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "macos")]
pub(crate) use macos::MacOsBackend;
#[cfg(target_os = "macos")]
pub use macos::{disk_inventory, list_disks};
#[cfg(target_os = "windows")]
pub use windows::{disk_inventory, list_disks};
