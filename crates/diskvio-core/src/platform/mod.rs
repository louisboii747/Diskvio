#[cfg(any(target_os = "macos", target_os = "windows", test))]
mod command;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(any(target_os = "windows", test))]
mod windows;

#[cfg(target_os = "macos")]
pub(crate) fn backend() -> impl crate::DiskBackend {
    macos::MacOsBackend::<command::SystemCommandRunner>::default()
}
#[cfg(target_os = "windows")]
pub(crate) fn backend() -> impl crate::DiskBackend {
    windows::WindowsBackend::<command::SystemCommandRunner>::default()
}
#[cfg(target_os = "macos")]
pub use macos::{disk_inventory, list_disks};
#[cfg(target_os = "windows")]
pub use windows::{disk_inventory, list_disks};
