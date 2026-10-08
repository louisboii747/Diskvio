use crate::DiskInventory;
#[cfg(any(target_os = "macos", test))]
use crate::{Device, PhysicalDisk};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiskOperation {
    Mount,
    Unmount,
    Eject,
}

#[derive(Debug, Deserialize)]
pub struct OperationRequest {
    pub action: DiskOperation,
    pub identifier: String,
    /// An opaque snapshot identity, revalidated against fresh platform metadata.
    pub expected_identity: String,
}

#[derive(Debug, Serialize)]
pub struct OperationOutcome {
    pub action: DiskOperation,
    pub identifier: String,
    pub message: String,
}

/// Platform implementations own discovery and execution; shared policy is below.
pub trait DiskBackend {
    fn inventory(&self) -> Result<DiskInventory, String>;
    fn perform(&self, request: &OperationRequest) -> Result<OperationOutcome, String>;
}

pub fn perform_operation(request: &OperationRequest) -> Result<OperationOutcome, String> {
    #[cfg(target_os = "macos")]
    {
        static OPERATION_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
        let _guard = OPERATION_LOCK.try_lock().map_err(
            |_| "Another disk operation is already running. Try again when it finishes.",
        )?;
        crate::platform::MacOsBackend.perform(request)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = request;
        Err("Disk operations are not implemented on this platform".into())
    }
}

#[cfg(any(target_os = "macos", test))]
fn protected_path(path: &str) -> bool {
    path == "/" || path == "/System" || path.starts_with("/System/")
}

#[cfg(any(target_os = "macos", test))]
fn external(disk: &PhysicalDisk) -> bool {
    disk.internal == Some(false) && disk.registry_entry_id.is_some()
}

#[cfg(any(target_os = "macos", test))]
fn identity(device: &Device, parents: &[&PhysicalDisk]) -> Option<String> {
    if parents.is_empty() || parents.iter().any(|disk| !external(disk)) {
        return None;
    }
    let mut parents: Vec<_> = parents
        .iter()
        .map(|disk| (disk.device.identifier.as_str(), disk.registry_entry_id))
        .collect();
    parents.sort_unstable();
    serde_json::to_string(&(
        device.identifier.as_str(),
        &device.volume_uuid,
        &device.partition_uuid,
        parents,
    ))
    .ok()
}

#[cfg(any(target_os = "macos", test))]
fn volume_actions(device: &Device, locked: Option<bool>) -> Vec<DiskOperation> {
    if device.identity_token.is_none() || device.filesystem.is_none() || locked == Some(true) {
        return vec![];
    }
    if device.mount_point.as_deref().is_some_and(protected_path) {
        return vec![];
    }
    vec![if device.mount_point.is_some() {
        DiskOperation::Unmount
    } else {
        DiskOperation::Mount
    }]
}

/// Advertise only operations that the shared policy will accept. No action is
/// executed here, and caller-provided capabilities are never trusted.
#[cfg(any(target_os = "macos", test))]
pub(crate) fn annotate_operations(inventory: &mut DiskInventory) {
    let parents = inventory.disks.clone();
    for (disk, parent) in inventory.disks.iter_mut().zip(&parents) {
        disk.device.actions.clear();
        disk.device.identity_token = identity(&disk.device, &[disk]);
        let protects_system = disk
            .device
            .mount_point
            .as_deref()
            .is_some_and(protected_path)
            || disk
                .partitions
                .iter()
                .any(|p| p.device.mount_point.as_deref().is_some_and(protected_path))
            || inventory
                .apfs_containers
                .iter()
                .filter(|container| {
                    container.physical_store_ids.iter().any(|id| {
                        disk.device.identifier == *id
                            || disk.partitions.iter().any(|p| p.device.identifier == *id)
                    })
                })
                .any(|container| {
                    container.volumes.iter().any(|volume| {
                        volume
                            .device
                            .mount_point
                            .as_deref()
                            .is_some_and(protected_path)
                            || volume.mounted_snapshots.iter().any(|snapshot| {
                                snapshot.mount_point.as_deref().is_some_and(protected_path)
                            })
                    })
                });
        let unsupported_pool = disk.partitions.iter().any(|partition| {
            partition
                .content_type
                .as_deref()
                .is_some_and(|content| content.contains("CoreStorage") || content.contains("RAID"))
        });
        if !unsupported_pool
            && !protects_system
            && disk.device.identity_token.is_some()
            && (disk.removable == Some(true) || disk.ejectable == Some(true))
            && inventory
                .apfs_containers
                .iter()
                .filter(|container| {
                    container.physical_store_ids.iter().any(|id| {
                        disk.device.identifier == *id
                            || disk.partitions.iter().any(|p| p.device.identifier == *id)
                    })
                })
                .all(|container| {
                    container.physical_store_ids.iter().all(|id| {
                        parents.iter().any(|backing| {
                            external(backing)
                                && (backing.device.identifier == *id
                                    || backing
                                        .partitions
                                        .iter()
                                        .any(|p| p.device.identifier == *id))
                        })
                    })
                })
        {
            disk.device.actions.push(DiskOperation::Eject);
        }
        // A superfloppy can have a filesystem on the whole physical disk.
        disk.device
            .actions
            .extend(volume_actions(&disk.device, None));
        for partition in &mut disk.partitions {
            partition.device.actions.clear();
            partition.device.identity_token = identity(&partition.device, &[parent]);
            if partition.apfs_container_id.is_none() {
                partition.device.actions = volume_actions(&partition.device, None);
            }
        }
    }
    for container in &mut inventory.apfs_containers {
        let backing: Vec<_> = parents
            .iter()
            .filter(|disk| {
                container.physical_store_ids.iter().any(|id| {
                    disk.device.identifier == *id
                        || disk.partitions.iter().any(|p| p.device.identifier == *id)
                })
            })
            .collect();
        // Every store must resolve to a known physical disk; unknown or internal
        // members of a Fusion/multi-store container make all its volumes read-only.
        let all_resolved = container.physical_store_ids.iter().all(|id| {
            backing.iter().any(|disk| {
                disk.device.identifier == *id
                    || disk.partitions.iter().any(|p| p.device.identifier == *id)
            })
        });
        for volume in &mut container.volumes {
            volume.device.actions.clear();
            volume.device.identity_token = None;
            if all_resolved && volume.locked == Some(false) && volume.mounted_snapshots.is_empty() {
                volume.device.identity_token = identity(&volume.device, &backing);
                volume.device.actions = volume_actions(&volume.device, volume.locked);
            }
        }
    }
}

#[cfg(any(target_os = "macos", test))]
pub(crate) fn validate_request<'a>(
    inventory: &'a DiskInventory,
    request: &OperationRequest,
) -> Result<&'a Device, String> {
    if !valid_identifier(&request.identifier) {
        return Err("Invalid device identifier".into());
    }
    let device = inventory
        .disks
        .iter()
        .flat_map(|disk| {
            std::iter::once(&disk.device)
                .chain(disk.partitions.iter().map(|partition| &partition.device))
        })
        .chain(
            inventory
                .apfs_containers
                .iter()
                .flat_map(|container| container.volumes.iter().map(|volume| &volume.device)),
        )
        .find(|device| device.identifier == request.identifier)
        .ok_or("The device was removed. Refresh before trying again.")?;
    if device.identity_token.as_deref() != Some(request.expected_identity.as_str())
        || request.expected_identity.is_empty()
    {
        return Err(
            "The device identity changed or could not be verified. Refresh before trying again."
                .into(),
        );
    }
    if !device.actions.contains(&request.action) {
        return Err("This operation is unavailable. Internal disks, protected system volumes, locked volumes and unknown device locations cannot be managed.".into());
    }
    Ok(device)
}

#[cfg(any(target_os = "macos", test))]
pub(crate) fn valid_identifier(identifier: &str) -> bool {
    let Some(rest) = identifier.strip_prefix("disk") else {
        return false;
    };
    let mut parts = rest.split('s');
    parts.all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
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
            }],
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
