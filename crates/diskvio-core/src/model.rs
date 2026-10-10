use serde::{Deserialize, Serialize};

/// A normalized topology: containers are stored once, including multi-store APFS.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DiskInventory {
    pub disks: Vec<PhysicalDisk>,
    pub apfs_containers: Vec<ApfsContainer>,
    /// Partial discovery errors are visible to callers, never replaced by fake data.
    pub warnings: Vec<String>,
    #[serde(default)]
    pub unattached_volumes: Vec<Volume>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Device {
    pub identifier: String,
    pub name: String,
    /// Filesystem label, distinct from a media or synthesized device name.
    #[serde(default)]
    pub volume_label: Option<String>,
    pub media_name: Option<String>,
    pub size_bytes: Option<u64>,
    pub filesystem: Option<Filesystem>,
    pub mount_point: Option<String>,
    pub volume_uuid: Option<String>,
    pub partition_uuid: Option<String>,
    pub used_bytes: Option<u64>,
    pub available_bytes: Option<u64>,
    #[serde(default)]
    pub identity_token: Option<String>,
    #[serde(default)]
    pub actions: Vec<crate::DiskOperation>,
    #[serde(default)]
    pub stable_id: Option<String>,
    #[serde(default)]
    pub drive_letter: Option<String>,
    #[serde(default)]
    pub mount_points: Vec<String>,
    #[serde(default)]
    pub safety: DeviceSafety,
    #[serde(default)]
    pub health_status: Option<String>,
    #[serde(default)]
    pub operational_status: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DeviceSafety {
    pub system: Option<bool>,
    pub boot: Option<bool>,
    pub recovery: Option<bool>,
    pub hidden: Option<bool>,
    pub read_only: Option<bool>,
    pub offline: Option<bool>,
    #[serde(default)]
    pub page_file: Option<bool>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PhysicalDisk {
    pub device: Device,
    pub number: u32,
    pub partition_scheme: PartitionScheme,
    pub connection_type: Option<String>,
    pub internal: Option<bool>,
    pub removable: Option<bool>,
    pub ejectable: Option<bool>,
    pub registry_entry_id: Option<u64>,
    pub partitions: Vec<Partition>,
    #[serde(default)]
    pub disk_uuid: Option<String>,
    #[serde(default)]
    pub mbr_signature: Option<u32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Partition {
    pub device: Device,
    #[serde(default)]
    pub role: Option<PartitionRole>,
    /// Partition type is distinct from the filesystem that may occupy it.
    pub content_type: Option<String>,
    pub offset_bytes: Option<u64>,
    pub apfs_container_id: Option<String>,
    #[serde(default)]
    pub number: Option<u32>,
    #[serde(default)]
    pub gpt_type: Option<String>,
    #[serde(default)]
    pub mbr_type: Option<u16>,
    #[serde(default)]
    pub active: Option<bool>,
    #[serde(default)]
    pub shadow_copy: Option<bool>,
    #[serde(default)]
    pub no_default_drive_letter: Option<bool>,
    #[serde(default)]
    pub volumes: Vec<Volume>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PartitionRole {
    EfiSystem,
    Recovery,
    MicrosoftReserved,
    BasicData,
    Unknown,
}

impl PartitionRole {
    pub fn from_windows_name(name: Option<&str>) -> Self {
        match name.map(str::to_ascii_lowercase).as_deref() {
            Some("recovery") => Self::Recovery,
            Some("reserved") => Self::MicrosoftReserved,
            Some("basic" | "ifs") => Self::BasicData,
            _ => Self::Unknown,
        }
    }

    pub fn windows(gpt: Option<&str>, mbr: Option<u16>) -> Self {
        if let Some(gpt) = gpt {
            return match gpt.trim_matches(['{', '}']).to_ascii_lowercase().as_str() {
                "c12a7328-f81f-11d2-ba4b-00a0c93ec93b" => Self::EfiSystem,
                "de94bba4-06d1-4d40-a16a-bfd50179d6ac" => Self::Recovery,
                "e3c9e316-0b5c-4db8-817d-f92df00215ae" => Self::MicrosoftReserved,
                "ebd0a0a2-b9e5-4433-87c0-68b6b72699c7" => Self::BasicData,
                _ => Self::Unknown,
            };
        }
        match mbr {
            Some(0xef) => Self::EfiSystem,
            Some(0x27) => Self::Recovery,
            Some(1 | 4 | 6 | 7 | 11 | 12 | 14) => Self::BasicData,
            _ => Self::Unknown,
        }
    }

    pub fn friendly_name(self) -> Option<&'static str> {
        match self {
            Self::EfiSystem => Some("EFI System Partition"),
            Self::Recovery => Some("Recovery Partition"),
            Self::MicrosoftReserved => Some("Microsoft Reserved"),
            Self::BasicData => Some("Basic Data"),
            Self::Unknown => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Volume {
    pub device: Device,
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApfsContainer {
    pub device: Device,
    pub uuid: Option<String>,
    pub physical_store_ids: Vec<String>,
    pub volumes: Vec<ApfsVolume>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApfsVolume {
    pub device: Device,
    #[serde(default)]
    pub mounted_snapshots: Vec<ApfsSnapshot>,
    pub roles: Vec<String>,
    pub encrypted: Option<bool>,
    pub locked: Option<bool>,
    pub quota_bytes: Option<u64>,
    pub reserve_bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApfsSnapshot {
    pub identifier: String,
    pub name: Option<String>,
    pub uuid: Option<String>,
    pub mount_point: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PartitionScheme {
    Gpt,
    Mbr,
    ApplePartitionMap,
    None,
    #[default]
    Unknown,
}

impl PartitionScheme {
    pub fn from_content(content: Option<&str>) -> Self {
        match content {
            Some("GUID_partition_scheme" | "GPT") => Self::Gpt,
            Some("FDisk_partition_scheme" | "MBR") => Self::Mbr,
            Some("Apple_partition_scheme") => Self::ApplePartitionMap,
            Some("" | "None" | "RAW") => Self::None,
            _ => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Filesystem {
    /// Preserve the platform's precise name (including case-sensitive variants).
    pub name: String,
    pub kind: FilesystemKind,
}

impl Filesystem {
    pub fn new(platform_type: &str, name: &str) -> Self {
        let kind = match platform_type.to_ascii_lowercase().as_str() {
            "apfs" => FilesystemKind::Apfs,
            "hfs" | "hfs+" => FilesystemKind::Hfs,
            "exfat" => FilesystemKind::Exfat,
            "msdos" | "fat" | "fat32" | "fat16" => FilesystemKind::Fat,
            "ntfs" => FilesystemKind::Ntfs,
            "udf" => FilesystemKind::Udf,
            "cd9660" | "iso9660" => FilesystemKind::Iso9660,
            _ => FilesystemKind::Other,
        };
        Self {
            name: name.to_owned(),
            kind,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FilesystemKind {
    Apfs,
    Hfs,
    Exfat,
    Fat,
    Ntfs,
    Udf,
    Iso9660,
    Other,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_partition_schemes_without_guessing() {
        assert_eq!(
            PartitionScheme::from_content(Some("GUID_partition_scheme")),
            PartitionScheme::Gpt
        );
        assert_eq!(
            PartitionScheme::from_content(Some("FDisk_partition_scheme")),
            PartitionScheme::Mbr
        );
        assert_eq!(
            PartitionScheme::from_content(Some("RAW")),
            PartitionScheme::None
        );
        assert_eq!(
            PartitionScheme::from_content(None),
            PartitionScheme::Unknown
        );
        assert_eq!(
            PartitionScheme::from_content(Some("Apple_APFS")),
            PartitionScheme::Unknown
        );
    }

    #[test]
    fn filesystem_preserves_variants_and_unknown_formats() {
        assert_eq!(
            Filesystem::new("APFS", "APFS (Case-sensitive)").kind,
            FilesystemKind::Apfs
        );
        assert_eq!(
            Filesystem::new("msdos", "MS-DOS FAT32").kind,
            FilesystemKind::Fat
        );
        assert_eq!(
            Filesystem::new("exfat", "ExFAT").kind,
            FilesystemKind::Exfat
        );
        assert_eq!(
            Filesystem::new("hfs", "Mac OS Extended (Journaled)").name,
            "Mac OS Extended (Journaled)"
        );
        assert_eq!(
            Filesystem::new("custom", "Custom FS").kind,
            FilesystemKind::Other
        );
    }

    #[test]
    fn unknown_capacity_is_not_zero() {
        let device = Device::default();
        let json = serde_json::to_value(device).unwrap();
        assert!(json["size_bytes"].is_null());
        assert!(json["available_bytes"].is_null());
    }
}
