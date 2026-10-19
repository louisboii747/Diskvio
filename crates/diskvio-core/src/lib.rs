//! Pure, unprivileged domain types. No operating system access.
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DeviceKind {
    Internal,
    Removable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Partition {
    pub id: String,
    pub name: String,
    pub offset_bytes: u64,
    pub capacity_bytes: u64,
    pub used_bytes: u64,
    pub filesystem: Option<String>,
    pub mount_point: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Disk {
    pub id: String,
    pub name: String,
    pub model: String,
    pub serial: String,
    pub kind: DeviceKind,
    pub connection: String,
    pub partition_scheme: String,
    pub capacity_bytes: u64,
    pub partitions: Vec<Partition>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    MissingIdentity,
    ZeroCapacity,
    DuplicatePartitionId,
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

/// Validates a sorted layout using checked arithmetic, including unallocated gaps.
pub fn validate_disk(disk: &Disk) -> Result<(), ValidationError> {
    if disk.id.trim().is_empty() || disk.serial.trim().is_empty() {
        return Err(ValidationError::MissingIdentity);
    }
    if disk.capacity_bytes == 0 {
        return Err(ValidationError::ZeroCapacity);
    }
    let mut end = 0;
    let mut ids = HashSet::new();
    for partition in &disk.partitions {
        if partition.id.trim().is_empty() {
            return Err(ValidationError::MissingIdentity);
        }
        if !ids.insert(&partition.id) {
            return Err(ValidationError::DuplicatePartitionId);
        }
        if partition.capacity_bytes == 0 {
            return Err(ValidationError::ZeroCapacity);
        }
        if partition.used_bytes > partition.capacity_bytes {
            return Err(ValidationError::UsageExceedsCapacity);
        }
        let next = partition
            .offset_bytes
            .checked_add(partition.capacity_bytes)
            .ok_or(ValidationError::OverlapOrOutOfBounds)?;
        if partition.offset_bytes < end || next > disk.capacity_bytes {
            return Err(ValidationError::OverlapOrOutOfBounds);
        }
        end = next;
    }
    Ok(())
}

/// A preliminary identity check, NOT sufficient authorization for future writes.
pub fn validate_identity(expected: &Disk, current: &Disk) -> Result<(), ValidationError> {
    validate_disk(expected)?;
    validate_disk(current)?;
    if expected.id != current.id
        || expected.serial != current.serial
        || expected.capacity_bytes != current.capacity_bytes
        || expected.model != current.model
    {
        return Err(ValidationError::IdentityChanged);
    }
    Ok(())
}

/// Decimal SI units, matching manufacturers' disk capacities.
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
        Disk {
            id: "demo-0".into(),
            name: "System".into(),
            model: "Example".into(),
            serial: "DEMO-0".into(),
            kind: DeviceKind::Internal,
            connection: "NVMe".into(),
            partition_scheme: "GPT".into(),
            capacity_bytes: 1000,
            partitions: vec![Partition {
                id: "p1".into(),
                name: "Data".into(),
                offset_bytes: 0,
                capacity_bytes: 800,
                used_bytes: 200,
                filesystem: Some("NTFS".into()),
                mount_point: Some("C:".into()),
            }],
        }
    }
    #[test]
    fn valid_layout_with_unallocated_space() {
        assert_eq!(validate_disk(&fixture()), Ok(()));
    }
    #[test]
    fn rejects_missing_identity_and_zero_capacity() {
        let mut d = fixture();
        d.serial.clear();
        assert_eq!(validate_disk(&d), Err(ValidationError::MissingIdentity));
        d.serial = "serial".into();
        d.capacity_bytes = 0;
        assert_eq!(validate_disk(&d), Err(ValidationError::ZeroCapacity));
    }
    #[test]
    fn rejects_overused_partition() {
        let mut d = fixture();
        d.partitions[0].used_bytes = 801;
        assert_eq!(
            validate_disk(&d),
            Err(ValidationError::UsageExceedsCapacity)
        );
    }
    #[test]
    fn rejects_overlap_and_duplicate_ids() {
        let mut d = fixture();
        d.partitions.push(d.partitions[0].clone());
        assert_eq!(
            validate_disk(&d),
            Err(ValidationError::DuplicatePartitionId)
        );
        d.partitions[1].id = "p2".into();
        assert_eq!(
            validate_disk(&d),
            Err(ValidationError::OverlapOrOutOfBounds)
        );
    }
    #[test]
    fn rejects_out_of_bounds_and_integer_overflow() {
        let mut d = fixture();
        d.partitions[0].offset_bytes = 999;
        assert_eq!(
            validate_disk(&d),
            Err(ValidationError::OverlapOrOutOfBounds)
        );
        d.partitions[0].offset_bytes = u64::MAX;
        assert_eq!(
            validate_disk(&d),
            Err(ValidationError::OverlapOrOutOfBounds)
        );
    }
    #[test]
    fn detects_replaced_device() {
        let d = fixture();
        let mut replacement = d.clone();
        replacement.serial = "different".into();
        assert_eq!(
            validate_identity(&d, &replacement),
            Err(ValidationError::IdentityChanged)
        );
        assert_eq!(validate_identity(&d, &d), Ok(()));
    }
    #[test]
    fn capacity_units_are_decimal() {
        assert_eq!(format_capacity(0), "0 B");
        assert_eq!(format_capacity(999), "999 B");
        assert_eq!(format_capacity(1000), "1.0 KB");
        assert_eq!(format_capacity(512_000_000_000), "512.0 GB");
        assert_eq!(format_capacity(u64::MAX), "18.4 EB");
    }
    #[test]
    fn serializable_contract_roundtrips() {
        let disk = fixture();
        let json = serde_json::to_value(&disk).unwrap();
        assert_eq!(json["capacityBytes"], 1000);
        assert_eq!(json["kind"], "internal");
        assert_eq!(serde_json::from_value::<Disk>(json).unwrap(), disk);
    }
    #[test]
    fn frontend_fixtures_match_rust_contract_and_validate() {
        let disks: Vec<Disk> = serde_json::from_str(include_str!(
            "../../../apps/desktop/src/fixtures/disks.json"
        ))
        .unwrap();
        assert_eq!(disks.len(), 3);
        let mut ids = HashSet::new();
        for disk in disks {
            assert!(disk.id.starts_with("demo-"));
            assert!(ids.insert(disk.id.clone()));
            validate_disk(&disk).unwrap();
        }
    }
}
