//! Pure read-only inventory contract. Byte counts serialize as decimal strings.
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, fmt};

pub mod bytes {
    use serde::{Deserialize, Deserializer, Serializer};
    pub fn serialize<S: Serializer>(value: &u64, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&value.to_string())
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<u64, D::Error> {
        String::deserialize(d)?
            .parse()
            .map_err(serde::de::Error::custom)
    }
}
pub mod optional_bytes {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    pub fn serialize<S: Serializer>(value: &Option<u64>, s: S) -> Result<S::Ok, S::Error> {
        value.map(|v| v.to_string()).serialize(s)
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<u64>, D::Error> {
        Option::<String>::deserialize(d)?
            .map(|v| v.parse().map_err(serde::de::Error::custom))
            .transpose()
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DeviceKind {
    Internal,
    External,
    Removable,
    Unknown,
}

/// Filesystem/logical volume, which need not have exclusive physical geometry (APFS).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Volume {
    pub id: String,
    pub label: Option<String>,
    pub filesystem: Option<String>,
    pub mount_points: Vec<String>,
    #[serde(with = "optional_bytes")]
    pub capacity_bytes: Option<u64>,
    #[serde(with = "optional_bytes")]
    pub used_bytes: Option<u64>,
    #[serde(with = "optional_bytes")]
    pub free_bytes: Option<u64>,
    pub shared_space: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Partition {
    pub id: String,
    pub number: Option<u32>,
    pub name: Option<String>,
    pub partition_type: Option<String>,
    #[serde(with = "bytes")]
    pub offset_bytes: u64,
    #[serde(with = "bytes")]
    pub capacity_bytes: u64,
    pub volumes: Vec<Volume>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Disk {
    /// Stable OS identity when available; path fallback is only for read-only selection.
    pub id: String,
    pub device_path: String,
    pub name: String,
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub serial: Option<String>,
    pub kind: DeviceKind,
    pub removable: Option<bool>,
    pub connection: Option<String>,
    pub partition_scheme: Option<String>,
    pub online: Option<bool>,
    pub read_only: Option<bool>,
    pub system: Option<bool>,
    #[serde(with = "bytes")]
    pub capacity_bytes: u64,
    /// None means unavailable; Some(empty) means a known empty layout.
    pub partitions: Option<Vec<Partition>>,
    pub limitations: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    MissingIdentity,
    ZeroCapacity,
    DuplicateDiskId,
    DuplicatePartitionId,
    DuplicateVolumeId,
    UsageExceedsCapacity,
    OverlapOrOutOfBounds,
    IdentityChanged,
}
impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for ValidationError {}
pub fn validate_inventory(disks: &[Disk]) -> Result<(), ValidationError> {
    let mut ids = HashSet::new();
    for disk in disks {
        if !ids.insert(&disk.id) {
            return Err(ValidationError::DuplicateDiskId);
        }
        validate_disk(disk)?;
    }
    Ok(())
}
pub fn validate_disk(disk: &Disk) -> Result<(), ValidationError> {
    if disk.id.trim().is_empty() || disk.device_path.trim().is_empty() {
        return Err(ValidationError::MissingIdentity);
    }
    if disk.capacity_bytes == 0 {
        return Err(ValidationError::ZeroCapacity);
    }
    let mut end = 0;
    let mut ids = HashSet::new();
    for p in disk.partitions.iter().flatten() {
        // Shared logical volumes may legitimately reference multiple physical stores.
        let mut volumes = HashSet::new();
        if p.id.trim().is_empty() {
            return Err(ValidationError::MissingIdentity);
        }
        if !ids.insert(&p.id) {
            return Err(ValidationError::DuplicatePartitionId);
        }
        if p.capacity_bytes == 0 {
            return Err(ValidationError::ZeroCapacity);
        }
        let next = p
            .offset_bytes
            .checked_add(p.capacity_bytes)
            .ok_or(ValidationError::OverlapOrOutOfBounds)?;
        if p.offset_bytes < end || next > disk.capacity_bytes {
            return Err(ValidationError::OverlapOrOutOfBounds);
        }
        end = next;
        for v in &p.volumes {
            if v.id.trim().is_empty() {
                return Err(ValidationError::MissingIdentity);
            }
            if !volumes.insert(&v.id) {
                return Err(ValidationError::DuplicateVolumeId);
            }
            if v.capacity_bytes.is_some_and(|cap| {
                v.used_bytes.is_some_and(|u| u > cap) || v.free_bytes.is_some_and(|f| f > cap)
            }) {
                return Err(ValidationError::UsageExceedsCapacity);
            }
            if !v.shared_space && v.capacity_bytes.is_some_and(|cap| cap > p.capacity_bytes) {
                return Err(ValidationError::UsageExceedsCapacity);
            }
            if let (Some(cap), Some(used), Some(free)) =
                (v.capacity_bytes, v.used_bytes, v.free_bytes)
                && used.checked_add(free).is_none_or(|sum| sum > cap)
            {
                return Err(ValidationError::UsageExceedsCapacity);
            }
        }
    }
    Ok(())
}
/// Display continuity only. Missing serial is legitimate, and this never authorizes writes.
pub fn validate_identity(expected: &Disk, current: &Disk) -> Result<(), ValidationError> {
    validate_disk(expected)?;
    validate_disk(current)?;
    if expected.id != current.id
        || expected.serial != current.serial
        || expected.model != current.model
        || expected.capacity_bytes != current.capacity_bytes
    {
        return Err(ValidationError::IdentityChanged);
    }
    Ok(())
}
pub fn format_capacity(bytes: u64) -> String {
    const UNITS: [&str; 7] = ["B", "KB", "MB", "GB", "TB", "PB", "EB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1000.0 && unit < UNITS.len() - 1 {
        value /= 1000.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Disk {
        serde_json::from_str::<Vec<Disk>>(include_str!(
            "../../../apps/desktop/src/fixtures/disks.json"
        ))
        .unwrap()
        .remove(0)
    }
    #[test]
    fn fixtures_validate() {
        let disks: Vec<Disk> = serde_json::from_str(include_str!(
            "../../../apps/desktop/src/fixtures/disks.json"
        ))
        .unwrap();
        assert_eq!(disks.len(), 3);
        assert_eq!(validate_inventory(&disks), Ok(()));
        assert_eq!(validate_inventory(&[]), Ok(()));
        assert_eq!(
            validate_inventory(&[fixture(), fixture()]),
            Err(ValidationError::DuplicateDiskId)
        );
    }
    #[test]
    fn optional_metadata_is_valid() {
        let mut d = fixture();
        d.serial = None;
        d.model = None;
        d.partitions = None;
        assert_eq!(validate_disk(&d), Ok(()));
        d.capacity_bytes = 0;
        assert_eq!(validate_disk(&d), Err(ValidationError::ZeroCapacity));
        d.id.clear();
        assert_eq!(validate_disk(&d), Err(ValidationError::MissingIdentity));
    }
    #[test]
    fn invalid_layouts() {
        let mut d = fixture();
        d.partitions.as_mut().unwrap()[0].offset_bytes = u64::MAX;
        assert_eq!(
            validate_disk(&d),
            Err(ValidationError::OverlapOrOutOfBounds)
        );
        d = fixture();
        d.partitions.as_mut().unwrap()[1].offset_bytes = 0;
        assert_eq!(
            validate_disk(&d),
            Err(ValidationError::OverlapOrOutOfBounds)
        );
        d = fixture();
        let ps = d.partitions.as_mut().unwrap();
        ps[1].id = ps[0].id.clone();
        assert_eq!(
            validate_disk(&d),
            Err(ValidationError::DuplicatePartitionId)
        );
        d = fixture();
        d.partitions.as_mut().unwrap()[0].capacity_bytes = d.capacity_bytes + 1;
        assert_eq!(
            validate_disk(&d),
            Err(ValidationError::OverlapOrOutOfBounds)
        );
    }
    #[test]
    fn usage_and_identity() {
        let mut d = fixture();
        d.partitions.as_mut().unwrap()[0].volumes[0].used_bytes = Some(u64::MAX);
        assert_eq!(
            validate_disk(&d),
            Err(ValidationError::UsageExceedsCapacity)
        );
        let a = fixture();
        let mut b = a.clone();
        assert_eq!(validate_identity(&a, &b), Ok(()));
        b.serial = Some("replacement".into());
        assert_eq!(
            validate_identity(&a, &b),
            Err(ValidationError::IdentityChanged)
        );
    }
    #[test]
    fn exact_string_transport() {
        let mut d = fixture();
        d.capacity_bytes = u64::MAX;
        let json = serde_json::to_value(&d).unwrap();
        assert_eq!(json["capacityBytes"], u64::MAX.to_string());
        assert_eq!(serde_json::from_value::<Disk>(json).unwrap(), d);
        assert_eq!(format_capacity(u64::MAX), "18.4 EB");
        assert_eq!(format_capacity(1000), "1.0 KB");
    }
}
