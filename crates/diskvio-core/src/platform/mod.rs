#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "macos")]
pub use macos::list_disks;
#[cfg(target_os = "windows")]
pub use windows::list_disks;
