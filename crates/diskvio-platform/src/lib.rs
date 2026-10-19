//! Default-deny platform boundary. No OS storage APIs are called in Milestone 1.
use diskvio_core::Disk;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Capabilities {
    pub discovery: bool,
    pub partition_management: bool,
    pub usb_writing: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub enum PlatformError {
    NotYetAvailable,
}

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
        Err(PlatformError::NotYetAvailable)
    }
}

pub fn platform_name() -> &'static str {
    std::env::consts::OS
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
            UnavailablePlatform.discover(),
            Err(PlatformError::NotYetAvailable)
        );
    }
}
