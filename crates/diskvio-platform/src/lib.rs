//! Unprivileged, read-only discovery. No storage mutation interface is exposed.
use diskvio_core::Disk;
use serde::Serialize;

#[cfg(any(target_os = "linux", test))]
mod linux;
#[cfg(any(target_os = "macos", test))]
mod macos;
#[cfg(any(target_os = "linux", target_os = "macos"))]
mod utility;
#[cfg(any(target_os = "windows", test))]
mod windows;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Capabilities {
    pub discovery: bool,
    pub partition_management: bool,
    pub usb_writing: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformError {
    pub code: ErrorCode,
    pub message: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ErrorCode {
    Unsupported,
    PermissionDenied,
    DiscoveryFailed,
    InvalidData,
    Timeout,
}
impl PlatformError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
    pub fn invalid(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::InvalidData, message)
    }
}
impl std::fmt::Display for PlatformError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for PlatformError {}

pub trait DiskPlatform {
    fn capabilities(&self) -> Capabilities;
    fn discover(&self) -> Result<Vec<Disk>, PlatformError>;
}

pub struct UnavailablePlatform;
impl DiskPlatform for UnavailablePlatform {
    fn capabilities(&self) -> Capabilities {
        Capabilities {
            discovery: false,
            partition_management: false,
            usb_writing: false,
        }
    }
    fn discover(&self) -> Result<Vec<Disk>, PlatformError> {
        Err(PlatformError::new(
            ErrorCode::Unsupported,
            "Read-only discovery is unsupported on this operating system.",
        ))
    }
}

pub fn platform_name() -> &'static str {
    std::env::consts::OS
}

pub struct NativePlatform;
impl DiskPlatform for NativePlatform {
    fn capabilities(&self) -> Capabilities {
        Capabilities {
            discovery: cfg!(any(
                target_os = "windows",
                target_os = "macos",
                target_os = "linux"
            )),
            partition_management: false,
            usb_writing: false,
        }
    }
    fn discover(&self) -> Result<Vec<Disk>, PlatformError> {
        #[cfg(target_os = "windows")]
        let result = windows::discover();
        #[cfg(target_os = "macos")]
        let result = macos::discover();
        #[cfg(target_os = "linux")]
        let result = linux::discover();
        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        let result = UnavailablePlatform.discover();
        let disks = result?;
        diskvio_core::validate_inventory(&disks).map_err(|e| {
            PlatformError::invalid(format!("Discovery returned an invalid inventory: {e}"))
        })?;
        Ok(disks)
    }
}

fn text(value: Option<String>) -> Option<String> {
    value.map(|v| v.trim().to_owned()).filter(|v| !v.is_empty())
}

#[cfg(any(target_os = "windows", target_os = "linux", test))]
fn partition_type(value: Option<&str>) -> Option<String> {
    value.map(
        |v| match v.trim_matches(['{', '}']).to_ascii_lowercase().as_str() {
            "c12a7328-f81f-11d2-ba4b-00a0c93ec93b" | "efi" | "0xef" => "EFI system".into(),
            "e3c9e316-0b5c-4db8-817d-f92df00215ae" => "Microsoft reserved".into(),
            "de94bba4-06d1-4d40-a16a-bfd50179d6ac" | "0x27" => "Recovery".into(),
            "ebd0a0a2-b9e5-4433-87c0-68b6b72699c7" | "0x7" => "Data".into(),
            _ => v.to_owned(),
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_capability_defaults_to_denied() {
        let caps = UnavailablePlatform.capabilities();
        assert!(!caps.discovery && !caps.partition_management && !caps.usb_writing);
    }
    #[test]
    fn discovery_does_not_pretend_no_devices_exist() {
        assert_eq!(
            UnavailablePlatform.discover().unwrap_err().code,
            ErrorCode::Unsupported
        );
        assert!(!NativePlatform.capabilities().usb_writing);
        assert!(!NativePlatform.capabilities().partition_management);
    }
}
