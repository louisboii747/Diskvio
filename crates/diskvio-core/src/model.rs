use serde::{Deserialize, Serialize};

/// A normalized topology: containers are stored once, including multi-store APFS.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DiskInventory {
    pub disks: Vec<PhysicalDisk>,
    pub apfs_containers: Vec<ApfsContainer>,
    /// Partial discovery errors are visible to callers, never replaced by fake data.
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Device {
    pub identifier: String,
    pub name: String,
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Partition {
    pub device: Device,
    /// Partition type is distinct from the filesystem that may occupy it.
    pub content_type: Option<String>,
    pub offset_bytes: Option<u64>,
    pub apfs_container_id: Option<String>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PartitionScheme {
    Gpt,
    Mbr,
    ApplePartitionMap,
    None,
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
