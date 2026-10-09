use serde::{Deserialize, Serialize};

mod model;
mod operations;
pub use model::*;
pub use operations::*;

mod platform;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Disk {
    pub number: u32,
    pub name: String,
    pub size_bytes: u64,
    pub bus_type: String,
    pub partition_style: String,
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
pub use platform::{disk_inventory, list_disks};

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub fn list_disks() -> Result<Vec<Disk>, Box<dyn std::error::Error>> {
    Err("Disk discovery is not implemented on this platform yet".into())
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub fn disk_inventory() -> Result<DiskInventory, Box<dyn std::error::Error>> {
    Err("Disk discovery is not implemented on this platform yet".into())
}
