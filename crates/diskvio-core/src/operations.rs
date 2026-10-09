use crate::{Device, DiskInventory, PhysicalDisk};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiskOperation {
    Mount,
    Unmount,
    Eject,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationRequest {
    pub action: DiskOperation,
    pub identifier: String,
    pub expected_identity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationOutcome {
    pub action: DiskOperation,
    pub identifier: String,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationErrorKind {
    InvalidRequest,
    MissingTarget,
    AmbiguousTarget,
    IdentityChanged,
    ProtectedDevice,
    UnsupportedOperation,
    InvalidState,
    IncompleteDiscovery,
    PermissionDenied,
    Busy,
    CommandFailed,
    Io,
    UnsupportedPlatform,
    OperationInProgress,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperationError {
    pub code: OperationErrorKind,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform_code: Option<i64>,
}

impl OperationError {
    pub fn new(code: OperationErrorKind, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            platform_code: None,
        }
    }

    #[cfg(any(target_os = "macos", target_os = "windows", test))]
    pub(crate) fn discovery(error: impl fmt::Display) -> Self {
        Self::new(OperationErrorKind::IncompleteDiscovery, error.to_string())
    }
}

impl fmt::Display for OperationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for OperationError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceKind {
    PhysicalDisk,
    Partition,
    ApfsContainer,
    ApfsVolume,
    Volume,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationCapabilities {
    pub identifier: String,
    pub device_kind: DeviceKind,
    pub identity_token: Option<String>,
    pub actions: Vec<DiskOperation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationValidation {
    pub valid: bool,
    pub action: DiskOperation,
    pub identifier: String,
    pub expected_identity: String,
}

pub trait DiskBackend {
    fn inventory(&self) -> Result<DiskInventory, OperationError>;
    fn execute(
        &self,
        request: &OperationRequest,
        device: &Device,
    ) -> Result<OperationOutcome, OperationError>;

    fn perform(&self, request: &OperationRequest) -> Result<OperationOutcome, OperationError> {
        let device = validate_request(&self.inventory()?, request)?;
        self.execute(request, &device)
    }
}

pub fn perform_operation(request: &OperationRequest) -> Result<OperationOutcome, OperationError> {
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    {
        static OPERATION_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
        let _guard = OPERATION_LOCK.try_lock().map_err(|_| {
            OperationError::new(
                OperationErrorKind::OperationInProgress,
                "Another disk operation is running. Try again when it finishes.",
            )
        })?;
        crate::platform::backend().perform(request)
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = request;
        Err(OperationError::new(
            OperationErrorKind::UnsupportedPlatform,
            "Disk operations are unsupported on this platform",
        ))
    }
}

fn operation_inventory() -> Result<DiskInventory, OperationError> {
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    {
        crate::platform::backend().inventory()
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        Err(OperationError::new(
            OperationErrorKind::UnsupportedPlatform,
            "Disk operations are unsupported on this platform",
        ))
    }
}

pub fn supported_operations(identifier: &str) -> Result<OperationCapabilities, OperationError> {
    check_identifier(identifier)?;
    supported_operations_in(&operation_inventory()?, identifier)
}

pub fn validate_operation(
    request: &OperationRequest,
) -> Result<OperationValidation, OperationError> {
    check_identifier(&request.identifier)?;
    validate_operation_in(&operation_inventory()?, request)
}

pub fn supported_operations_in(
    inventory: &DiskInventory,
    identifier: &str,
) -> Result<OperationCapabilities, OperationError> {
    check_identifier(identifier)?;
    let mut inventory = inventory.clone();
    annotate_operations(&mut inventory);
    let (device, kind) = unique_target(&inventory, identifier)?;
    Ok(OperationCapabilities {
        identifier: device.identifier.clone(),
        device_kind: kind,
        identity_token: device.identity_token.clone(),
        actions: device.actions.clone(),
    })
}

pub fn validate_operation_in(
    inventory: &DiskInventory,
    request: &OperationRequest,
) -> Result<OperationValidation, OperationError> {
    let device = validate_request(inventory, request)?;
    Ok(OperationValidation {
        valid: true,
        action: request.action,
        identifier: device.identifier,
        expected_identity: request.expected_identity.clone(),
    })
}

fn protected_path(path: &str) -> bool {
    path == "/" || path == "/System" || path.starts_with("/System/")
}

fn protected(device: &Device) -> bool {
    device.safety.system == Some(true)
        || device.safety.boot == Some(true)
        || device.safety.recovery == Some(true)
        || device.mount_point.as_deref().is_some_and(protected_path)
        || device.mount_points.iter().any(|path| protected_path(path))
}

fn owns(disk: &PhysicalDisk, identifier: &str) -> bool {
    disk.device.identifier == identifier
        || disk
            .partitions
            .iter()
            .any(|partition| partition.device.identifier == identifier)
}

fn protected_role(role: &str) -> bool {
    ["system", "data", "recovery", "preboot", "vm"]
        .iter()
        .any(|value| role.eq_ignore_ascii_case(value))
}

fn protected_disk(inventory: &DiskInventory, disk: &PhysicalDisk) -> bool {
    protected(&disk.device)
        || disk.partitions.iter().any(|partition| {
            protected(&partition.device)
                || partition
                    .volumes
                    .iter()
                    .any(|volume| protected(&volume.device))
        })
        || inventory
            .apfs_containers
            .iter()
            .filter(|container| container.physical_store_ids.iter().any(|id| owns(disk, id)))
            .any(|container| {
                container.volumes.iter().any(|volume| {
                    protected(&volume.device)
                        || volume.roles.iter().any(|role| protected_role(role))
                        || volume.mounted_snapshots.iter().any(|snapshot| {
                            snapshot.mount_point.as_deref().is_some_and(protected_path)
                        })
                })
            })
}

fn protected_target(inventory: &DiskInventory, device: &Device) -> bool {
    protected(device)
        || inventory.disks.iter().any(|disk| {
            let owns_volume = disk.partitions.iter().any(|partition| {
                partition
                    .volumes
                    .iter()
                    .any(|volume| volume.device.identifier == device.identifier)
            }) || inventory.apfs_containers.iter().any(|container| {
                container
                    .volumes
                    .iter()
                    .any(|volume| volume.device.identifier == device.identifier)
                    && container.physical_store_ids.iter().any(|id| owns(disk, id))
            });
            (owns(disk, &device.identifier) || owns_volume) && protected_disk(inventory, disk)
        })
}

fn external(disk: &PhysicalDisk) -> bool {
    disk.internal == Some(false)
        && (disk.registry_entry_id.is_some()
            || disk
                .device
                .stable_id
                .as_ref()
                .is_some_and(|id| !id.is_empty()))
}

fn manageable(inventory: &DiskInventory, disk: &PhysicalDisk) -> bool {
    external(disk)
        && !protected_disk(inventory, disk)
        && disk.device.safety.offline != Some(true)
        && disk.device.safety.read_only != Some(true)
        && !disk.partitions.iter().any(|partition| {
            partition
                .content_type
                .as_deref()
                .is_some_and(|content| content.contains("CoreStorage") || content.contains("RAID"))
        })
}

fn identity(device: &Device, parents: &[&PhysicalDisk]) -> Option<String> {
    if parents.is_empty() || parents.iter().any(|disk| !external(disk)) {
        return None;
    }
    let mut parents: Vec<_> = parents
        .iter()
        .map(|disk| {
            (
                disk.registry_entry_id,
                &disk.device.stable_id,
                disk.device.size_bytes,
            )
        })
        .collect();
    parents.sort_unstable();
    let target = device
        .stable_id
        .as_deref()
        .or(device.volume_uuid.as_deref())
        .or(device.partition_uuid.as_deref())
        .unwrap_or(&device.identifier);
    serde_json::to_string(&(
        target,
        &device.volume_uuid,
        &device.partition_uuid,
        device.size_bytes,
        parents,
    ))
    .ok()
}

fn volume_actions(device: &Device, locked: Option<bool>) -> Vec<DiskOperation> {
    if device.identity_token.is_none()
        || device.filesystem.is_none()
        || locked == Some(true)
        || protected(device)
        || device.safety.hidden == Some(true)
        || device.safety.offline == Some(true)
        || device.safety.read_only == Some(true)
    {
        return vec![];
    }
    vec![
        if device.mount_point.is_some() || !device.mount_points.is_empty() {
            DiskOperation::Unmount
        } else {
            DiskOperation::Mount
        },
    ]
}

pub(crate) fn valid_volume_path(path: &str) -> bool {
    let Some(guid) = path
        .strip_prefix(r"\\?\Volume{")
        .and_then(|value| value.strip_suffix("}\\"))
    else {
        return false;
    };
    valid_uuid(guid)
}

pub(crate) fn valid_uuid(value: &str) -> bool {
    value.len() == 36
        && value.bytes().enumerate().all(|(index, byte)| {
            if [8, 13, 18, 23].contains(&index) {
                byte == b'-'
            } else {
                byte.is_ascii_hexdigit()
            }
        })
}

pub(crate) fn valid_drive_path(path: &str) -> bool {
    let bytes = path.as_bytes();
    bytes.len() == 3 && bytes[0].is_ascii_uppercase() && bytes[1] == b':' && bytes[2] == b'\\'
}

fn windows_volume_supported(
    gpt_type: Option<&str>,
    mbr_type: Option<u16>,
    device: &Device,
) -> bool {
    let basic = gpt_type.is_some_and(|kind| {
        kind.trim_matches(['{', '}'])
            .eq_ignore_ascii_case("ebd0a0a2-b9e5-4433-87c0-68b6b72699c7")
    }) || (gpt_type.is_none()
        && mbr_type.is_some_and(|kind| [1, 4, 6, 7, 11, 12, 14].contains(&kind)));
    basic
        && device.stable_id.as_deref().is_some_and(valid_volume_path)
        && device.filesystem.as_ref().is_some_and(|fs| {
            matches!(
                fs.kind,
                crate::FilesystemKind::Ntfs
                    | crate::FilesystemKind::Exfat
                    | crate::FilesystemKind::Fat
            )
        })
        && device.safety.system == Some(false)
        && device.safety.boot == Some(false)
        && device.safety.recovery == Some(false)
        && device.safety.hidden == Some(false)
        && device.safety.read_only == Some(false)
        && device.safety.offline == Some(false)
        && device.mount_points.len() <= 1
        && device
            .mount_points
            .iter()
            .all(|path| valid_drive_path(path))
        && device
            .mount_point
            .as_ref()
            .is_none_or(|path| device.mount_points.contains(path))
}

pub(crate) fn annotate_operations(inventory: &mut DiskInventory) {
    for volume in &mut inventory.unattached_volumes {
        volume.device.actions.clear();
        volume.device.identity_token = None;
    }
    let snapshot = inventory.clone();
    let parents = &snapshot.disks;
    for (disk, parent) in inventory.disks.iter_mut().zip(parents) {
        disk.device.actions.clear();
        disk.device.identity_token = identity(&disk.device, &[parent]);
        let allowed = snapshot.warnings.is_empty() && manageable(&snapshot, parent);
        let safe_pool = snapshot
            .apfs_containers
            .iter()
            .filter(|container| {
                container
                    .physical_store_ids
                    .iter()
                    .any(|id| owns(parent, id))
            })
            .all(|container| {
                !container.physical_store_ids.is_empty()
                    && container.physical_store_ids.iter().all(|id| {
                        let backing: Vec<_> =
                            parents.iter().filter(|disk| owns(disk, id)).collect();
                        backing.len() == 1 && manageable(&snapshot, backing[0])
                    })
            });
        if allowed && safe_pool {
            if disk.registry_entry_id.is_some()
                && (disk.removable == Some(true) || disk.ejectable == Some(true))
            {
                disk.device.actions.push(DiskOperation::Eject);
            }
            if !snapshot.apfs_containers.iter().any(|container| {
                container
                    .physical_store_ids
                    .contains(&disk.device.identifier)
            }) {
                disk.device
                    .actions
                    .extend(volume_actions(&disk.device, None));
            }
        }
        for partition in &mut disk.partitions {
            partition.device.actions.clear();
            partition.device.identity_token = identity(&partition.device, &[parent]);
            if allowed
                && safe_pool
                && partition.apfs_container_id.is_none()
                && partition.volumes.is_empty()
            {
                partition.device.actions = volume_actions(&partition.device, None);
            }
            for volume in &mut partition.volumes {
                volume.device.identity_token = identity(&volume.device, &[parent]);
                volume.device.actions.clear();
                if allowed
                    && safe_pool
                    && windows_volume_supported(
                        partition.gpt_type.as_deref(),
                        partition.mbr_type,
                        &volume.device,
                    )
                {
                    volume.device.actions = volume_actions(&volume.device, None);
                }
            }
        }
    }
    for container in &mut inventory.apfs_containers {
        container.device.actions.clear();
        container.device.identity_token = None;
        let mut backing = Vec::new();
        let all_resolved = !container.physical_store_ids.is_empty()
            && container.physical_store_ids.iter().all(|id| {
                let disks: Vec<_> = parents.iter().filter(|disk| owns(disk, id)).collect();
                if disks.len() != 1 || !manageable(&snapshot, disks[0]) {
                    return false;
                }
                backing.push(disks[0]);
                true
            });
        backing.sort_by_key(|disk| disk.number);
        backing.dedup_by_key(|disk| disk.number);
        for volume in &mut container.volumes {
            volume.device.actions.clear();
            volume.device.identity_token = None;
            if snapshot.warnings.is_empty()
                && all_resolved
                && volume.locked == Some(false)
                && volume.mounted_snapshots.is_empty()
                && !volume.roles.iter().any(|role| protected_role(role))
            {
                volume.device.identity_token = identity(&volume.device, &backing);
                volume.device.actions = volume_actions(&volume.device, volume.locked);
            }
        }
    }
    let current = inventory.clone();
    for (device, kind) in targets_mut(inventory) {
        if ambiguous_identity(&current, device, kind) {
            device.actions.clear();
        }
    }
}

fn targets(inventory: &DiskInventory) -> impl Iterator<Item = (&Device, DeviceKind)> {
    inventory
        .unattached_volumes
        .iter()
        .map(|volume| (&volume.device, DeviceKind::Volume))
        .chain(
            inventory
                .disks
                .iter()
                .flat_map(|disk| {
                    std::iter::once((&disk.device, DeviceKind::PhysicalDisk)).chain(
                        disk.partitions.iter().flat_map(|partition| {
                            std::iter::once((&partition.device, DeviceKind::Partition)).chain(
                                partition
                                    .volumes
                                    .iter()
                                    .map(|volume| (&volume.device, DeviceKind::Volume)),
                            )
                        }),
                    )
                })
                .chain(inventory.apfs_containers.iter().flat_map(|container| {
                    std::iter::once((&container.device, DeviceKind::ApfsContainer)).chain(
                        container
                            .volumes
                            .iter()
                            .map(|volume| (&volume.device, DeviceKind::ApfsVolume)),
                    )
                })),
        )
}

fn ambiguous_identity(inventory: &DiskInventory, device: &Device, kind: DeviceKind) -> bool {
    targets(inventory)
        .filter(|(other, other_kind)| {
            *other_kind == kind
                && (other.identifier == device.identifier
                    || device.stable_id.as_ref().is_some_and(|id| {
                        other
                            .stable_id
                            .as_ref()
                            .is_some_and(|other_id| id.eq_ignore_ascii_case(other_id))
                    })
                    || device.volume_uuid.as_ref().is_some_and(|id| {
                        other
                            .volume_uuid
                            .as_ref()
                            .is_some_and(|other_id| id.eq_ignore_ascii_case(other_id))
                    })
                    || (device.volume_uuid.is_none()
                        && device.partition_uuid.as_ref().is_some_and(|id| {
                            other
                                .partition_uuid
                                .as_ref()
                                .is_some_and(|other_id| id.eq_ignore_ascii_case(other_id))
                        })))
        })
        .count()
        > 1
}

fn targets_mut(inventory: &mut DiskInventory) -> impl Iterator<Item = (&mut Device, DeviceKind)> {
    inventory
        .unattached_volumes
        .iter_mut()
        .map(|volume| (&mut volume.device, DeviceKind::Volume))
        .chain(
            inventory
                .disks
                .iter_mut()
                .flat_map(|disk| {
                    std::iter::once((&mut disk.device, DeviceKind::PhysicalDisk)).chain(
                        disk.partitions.iter_mut().flat_map(|partition| {
                            std::iter::once((&mut partition.device, DeviceKind::Partition)).chain(
                                partition
                                    .volumes
                                    .iter_mut()
                                    .map(|volume| (&mut volume.device, DeviceKind::Volume)),
                            )
                        }),
                    )
                })
                .chain(inventory.apfs_containers.iter_mut().flat_map(|container| {
                    std::iter::once((&mut container.device, DeviceKind::ApfsContainer)).chain(
                        container
                            .volumes
                            .iter_mut()
                            .map(|volume| (&mut volume.device, DeviceKind::ApfsVolume)),
                    )
                })),
        )
}

fn unique_target<'a>(
    inventory: &'a DiskInventory,
    identifier: &str,
) -> Result<(&'a Device, DeviceKind), OperationError> {
    let mut matches = targets(inventory).filter(|(device, _)| device.identifier == identifier);
    let target = matches.next().ok_or_else(|| {
        OperationError::new(
            OperationErrorKind::MissingTarget,
            "The device is missing. Refresh before trying again.",
        )
    })?;
    if matches.next().is_some() {
        return Err(OperationError::new(
            OperationErrorKind::AmbiguousTarget,
            "More than one device has this identifier. Refresh before trying again.",
        ));
    }
    Ok(target)
}

pub(crate) fn validate_request(
    inventory: &DiskInventory,
    request: &OperationRequest,
) -> Result<Device, OperationError> {
    check_identifier(&request.identifier)?;
    if !inventory.warnings.is_empty() {
        return Err(OperationError::new(
            OperationErrorKind::IncompleteDiscovery,
            "Device discovery was incomplete. Resolve discovery warnings before performing operations.",
        ));
    }
    if request.expected_identity.is_empty() {
        return Err(OperationError::new(
            OperationErrorKind::IdentityChanged,
            "A current device identity token is required.",
        ));
    }
    let mut inventory = inventory.clone();
    annotate_operations(&mut inventory);
    let original = unique_target(&inventory, &request.identifier);
    if original
        .as_ref()
        .is_err_and(|error| error.code == OperationErrorKind::AmbiguousTarget)
    {
        return Err(original.unwrap_err());
    }
    let mut matches = targets(&inventory).filter(|(device, _)| {
        device.identity_token.as_deref() == Some(request.expected_identity.as_str())
    });
    let target = matches.next();
    if matches.next().is_some() {
        return Err(OperationError::new(
            OperationErrorKind::AmbiguousTarget,
            "The stable device identity resolves to multiple targets.",
        ));
    }
    let (device, kind) = match target {
        Some(target) => target,
        None => {
            let (device, kind) = original?;
            if kind == DeviceKind::ApfsContainer {
                return Err(OperationError::new(
                    OperationErrorKind::UnsupportedOperation,
                    "APFS containers are storage pools, not mountable filesystems.",
                ));
            }
            if protected_target(&inventory, device) {
                return Err(OperationError::new(
                    OperationErrorKind::ProtectedDevice,
                    "System, boot and recovery devices cannot be managed.",
                ));
            }
            return Err(OperationError::new(
                OperationErrorKind::IdentityChanged,
                "The device identity changed or could not be verified. Refresh before trying again.",
            ));
        }
    };
    unique_target(&inventory, &device.identifier)?;
    if ambiguous_identity(&inventory, device, kind) {
        return Err(OperationError::new(
            OperationErrorKind::AmbiguousTarget,
            "The stable identifier or UUID belongs to multiple devices. Refresh and resolve the ambiguity.",
        ));
    }
    if !device.actions.contains(&request.action) {
        if matches!(
            request.action,
            DiskOperation::Mount | DiskOperation::Unmount
        ) && device
            .actions
            .iter()
            .any(|action| matches!(action, DiskOperation::Mount | DiskOperation::Unmount))
        {
            return Err(OperationError::new(
                OperationErrorKind::InvalidState,
                "The volume mount state does not permit this operation. Refresh before trying again.",
            ));
        }

        let code = if protected_target(&inventory, device) {
            OperationErrorKind::ProtectedDevice
        } else {
            OperationErrorKind::UnsupportedOperation
        };
        return Err(OperationError::new(
            code,
            format!(
                "{:?} is unavailable for this {:?}. Protected, internal, locked, unknown or unsupported devices are inspection-only.",
                request.action, kind
            ),
        ));
    }
    Ok(device.clone())
}

fn check_identifier(identifier: &str) -> Result<(), OperationError> {
    if identifier.is_empty()
        || identifier.len() > 512
        || identifier.chars().any(|ch| ch.is_control())
    {
        Err(OperationError::new(
            OperationErrorKind::InvalidRequest,
            "Invalid device identifier",
        ))
    } else {
        Ok(())
    }
}

#[cfg(any(target_os = "macos", test))]
pub(crate) fn valid_identifier(identifier: &str) -> bool {
    let Some(rest) = identifier.strip_prefix("disk") else {
        return false;
    };
    rest.split('s')
        .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ApfsContainer, ApfsVolume, Filesystem, Partition, PartitionScheme};

    fn disk(internal: Option<bool>, registry: Option<u64>) -> PhysicalDisk {
        PhysicalDisk {
            device: Device {
                identifier: "disk2".into(),
                name: "USB".into(),
                ..Default::default()
            },
            number: 2,
            internal,
            registry_entry_id: registry,
            removable: Some(false),
            ejectable: Some(true),
            connection_type: Some("USB".into()),
            partition_scheme: PartitionScheme::Gpt,
            partitions: vec![Partition {
                device: Device {
                    identifier: "disk2s1".into(),
                    partition_uuid: Some("part".into()),
                    filesystem: Some(Filesystem::new("exfat", "ExFAT")),
                    ..Default::default()
                },
                content_type: None,
                offset_bytes: None,
                apfs_container_id: None,
                ..Default::default()
            }],
            ..Default::default()
        }
    }
    fn inventory(internal: Option<bool>, registry: Option<u64>) -> DiskInventory {
        DiskInventory {
            disks: vec![disk(internal, registry)],
            ..Default::default()
        }
    }

    #[test]
    fn internal_unknown_and_unidentified_disks_have_no_actions() {
        for (internal, registry) in [(Some(true), Some(1)), (None, Some(1)), (Some(false), None)] {
            let mut inventory = inventory(internal, registry);
            annotate_operations(&mut inventory);
            assert!(inventory.disks[0].device.actions.is_empty());
            assert!(inventory.disks[0].partitions[0].device.actions.is_empty());
        }
    }

    #[test]
    fn stale_identity_cannot_target_reused_disk_number() {
        let mut inventory = inventory(Some(false), Some(1));
        annotate_operations(&mut inventory);
        let request = OperationRequest {
            action: DiskOperation::Eject,
            identifier: "disk2".into(),
            expected_identity: inventory.disks[0].device.identity_token.clone().unwrap(),
        };
        assert!(validate_request(&inventory, &request).is_ok());
        inventory.disks[0].registry_entry_id = Some(2);
        annotate_operations(&mut inventory);
        assert!(validate_request(&inventory, &request).is_err());
    }

    #[test]
    fn mount_state_and_system_paths_restrict_actions() {
        let mut inventory = inventory(Some(false), Some(1));
        annotate_operations(&mut inventory);
        assert_eq!(
            inventory.disks[0].partitions[0].device.actions,
            [DiskOperation::Mount]
        );
        inventory.disks[0].partitions[0].device.mount_point = Some("/Volumes/USB".into());
        annotate_operations(&mut inventory);
        assert_eq!(
            inventory.disks[0].partitions[0].device.actions,
            [DiskOperation::Unmount]
        );
        let request = OperationRequest {
            action: DiskOperation::Mount,
            identifier: "disk2s1".into(),
            expected_identity: inventory.disks[0].partitions[0]
                .device
                .identity_token
                .clone()
                .unwrap(),
        };
        assert_eq!(
            validate_request(&inventory, &request).unwrap_err().code,
            OperationErrorKind::InvalidState
        );
        inventory.disks[0].partitions[0].device.mount_point = Some("/".into());
        annotate_operations(&mut inventory);
        assert!(inventory.disks[0].partitions[0].device.actions.is_empty());
    }

    #[test]
    fn mixed_internal_external_and_unresolved_apfs_stores_are_protected() {
        let mut inventory = inventory(Some(false), Some(1));
        inventory.apfs_containers.push(ApfsContainer {
            device: Device::default(),
            uuid: None,
            physical_store_ids: vec!["disk2s1".into(), "disk0s2".into()],
            volumes: vec![ApfsVolume {
                device: Device {
                    identifier: "disk3s1".into(),
                    filesystem: Some(Filesystem::new("apfs", "APFS")),
                    ..Default::default()
                },
                mounted_snapshots: vec![],
                roles: vec![],
                encrypted: Some(false),
                locked: Some(false),
                quota_bytes: None,
                reserve_bytes: None,
            }],
        });
        annotate_operations(&mut inventory);
        assert!(
            inventory.apfs_containers[0].volumes[0]
                .device
                .actions
                .is_empty()
        );
    }

    #[test]
    fn mounted_system_snapshot_blocks_external_disk_ejection() {
        let mut inventory = inventory(Some(false), Some(1));
        inventory.apfs_containers.push(ApfsContainer {
            device: Device::default(),
            uuid: None,
            physical_store_ids: vec!["disk2s1".into()],
            volumes: vec![ApfsVolume {
                device: Device {
                    identifier: "disk3s1".into(),
                    filesystem: Some(Filesystem::new("apfs", "APFS")),
                    ..Default::default()
                },
                mounted_snapshots: vec![crate::ApfsSnapshot {
                    identifier: "disk3s1s1".into(),
                    name: None,
                    uuid: None,
                    mount_point: Some("/".into()),
                }],
                roles: vec!["System".into()],
                encrypted: Some(false),
                locked: Some(false),
                quota_bytes: None,
                reserve_bytes: None,
            }],
        });
        annotate_operations(&mut inventory);
        assert!(inventory.disks[0].device.actions.is_empty());
        assert!(
            inventory.apfs_containers[0].volumes[0]
                .device
                .actions
                .is_empty()
        );
    }

    #[test]
    fn internal_apfs_member_blocks_ejecting_external_member() {
        let mut inventory = inventory(Some(false), Some(1));
        let mut internal = disk(Some(true), Some(2));
        internal.device.identifier = "disk0".into();
        internal.partitions[0].device.identifier = "disk0s2".into();
        inventory.disks.push(internal);
        inventory.apfs_containers.push(ApfsContainer {
            device: Device::default(),
            uuid: None,
            physical_store_ids: vec!["disk2s1".into(), "disk0s2".into()],
            volumes: vec![],
        });
        annotate_operations(&mut inventory);
        assert!(inventory.disks[0].device.actions.is_empty());
    }

    #[test]
    fn supports_apfs_volumes_but_never_containers_or_physical_stores() {
        let mut inventory = inventory(Some(false), Some(1));
        inventory.disks[0].partitions[0].apfs_container_id = Some("disk3".into());
        inventory.apfs_containers.push(ApfsContainer {
            device: Device {
                identifier: "disk3".into(),
                filesystem: Some(Filesystem::new("apfs", "APFS")),
                ..Default::default()
            },
            uuid: Some("container".into()),
            physical_store_ids: vec!["disk2s1".into()],
            volumes: vec![ApfsVolume {
                device: Device {
                    identifier: "disk3s1".into(),
                    volume_uuid: Some("volume".into()),
                    filesystem: Some(Filesystem::new("apfs", "APFS")),
                    ..Default::default()
                },
                roles: vec![],
                locked: Some(false),
                encrypted: Some(false),
                mounted_snapshots: vec![],
                quota_bytes: None,
                reserve_bytes: None,
            }],
        });
        annotate_operations(&mut inventory);
        assert!(inventory.disks[0].partitions[0].device.actions.is_empty());
        assert!(
            supported_operations_in(&inventory, "disk3")
                .unwrap()
                .actions
                .is_empty()
        );
        let volume = &inventory.apfs_containers[0].volumes[0].device;
        let request = OperationRequest {
            action: DiskOperation::Mount,
            identifier: volume.identifier.clone(),
            expected_identity: volume.identity_token.clone().unwrap(),
        };
        assert!(validate_operation_in(&inventory, &request).unwrap().valid);
        let container = OperationRequest {
            identifier: "disk3".into(),
            expected_identity: "not-a-filesystem".into(),
            ..request.clone()
        };
        assert_eq!(
            validate_operation_in(&inventory, &container)
                .unwrap_err()
                .code,
            OperationErrorKind::UnsupportedOperation
        );
        inventory.apfs_containers[0].volumes[0].roles = vec!["Recovery".into()];
        assert_eq!(
            validate_operation_in(&inventory, &request)
                .unwrap_err()
                .code,
            OperationErrorKind::ProtectedDevice
        );
    }

    #[test]
    fn validates_using_fresh_policy_instead_of_advertised_actions() {
        let mut inventory = inventory(Some(true), Some(1));
        inventory.disks[0].device.actions = vec![DiskOperation::Eject];
        inventory.disks[0].device.identity_token = Some("forged".into());
        let request = OperationRequest {
            action: DiskOperation::Eject,
            identifier: "disk2".into(),
            expected_identity: "forged".into(),
        };
        assert!(validate_request(&inventory, &request).is_err());
        assert!(
            supported_operations_in(&inventory, "disk2")
                .unwrap()
                .actions
                .is_empty()
        );
    }

    #[test]
    fn resolves_stable_volume_identity_after_bsd_name_changes() {
        let mut inventory = inventory(Some(false), Some(1));
        annotate_operations(&mut inventory);
        let request = OperationRequest {
            action: DiskOperation::Mount,
            identifier: "disk2s1".into(),
            expected_identity: inventory.disks[0].partitions[0]
                .device
                .identity_token
                .clone()
                .unwrap(),
        };
        inventory.disks[0].partitions[0].device.identifier = "disk4s1".into();
        assert_eq!(
            validate_request(&inventory, &request).unwrap().identifier,
            "disk4s1"
        );
        let duplicate = inventory.disks[0].partitions[0].clone();
        inventory.disks[0].partitions.push(duplicate);
        assert_eq!(
            validate_request(&inventory, &request).unwrap_err().code,
            OperationErrorKind::AmbiguousTarget
        );
    }

    #[test]
    fn cloned_volume_uuids_on_different_disks_are_ambiguous() {
        let mut inventory = inventory(Some(false), Some(1));
        annotate_operations(&mut inventory);
        let request = OperationRequest {
            action: DiskOperation::Mount,
            identifier: "disk2s1".into(),
            expected_identity: inventory.disks[0].partitions[0]
                .device
                .identity_token
                .clone()
                .unwrap(),
        };
        let mut other = disk(Some(false), Some(2));
        other.number = 4;
        other.device.identifier = "disk4".into();
        other.partitions[0].device.identifier = "disk4s1".into();
        inventory.disks.push(other);
        assert_eq!(
            validate_request(&inventory, &request).unwrap_err().code,
            OperationErrorKind::AmbiguousTarget
        );
        assert!(
            supported_operations_in(&inventory, "disk2s1")
                .unwrap()
                .actions
                .is_empty()
        );
    }

    #[test]
    fn rejects_missing_targets_empty_identity_and_unsupported_actions() {
        let mut inventory = inventory(Some(false), Some(1));
        annotate_operations(&mut inventory);
        let device = &inventory.disks[0].partitions[0].device;
        let mut request = OperationRequest {
            action: DiskOperation::Eject,
            identifier: device.identifier.clone(),
            expected_identity: device.identity_token.clone().unwrap(),
        };
        assert_eq!(
            validate_request(&inventory, &request).unwrap_err().code,
            OperationErrorKind::UnsupportedOperation
        );
        request.expected_identity.clear();
        assert_eq!(
            validate_request(&inventory, &request).unwrap_err().code,
            OperationErrorKind::IdentityChanged
        );
        request.identifier = "disk99".into();
        request.expected_identity = "missing".into();
        assert_eq!(
            validate_request(&inventory, &request).unwrap_err().code,
            OperationErrorKind::MissingTarget
        );
    }

    #[test]
    fn rejects_paths_and_command_arguments() {
        for invalid in [
            "/dev/disk2",
            "disk2;id",
            "disk2s",
            "disk",
            "-force",
            "disk2 s1",
        ] {
            assert!(!valid_identifier(invalid));
        }
        assert!(valid_identifier("disk2s1"));
        assert!(valid_identifier("disk2"));
    }
}
