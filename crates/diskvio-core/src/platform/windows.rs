use super::command::CommandRunner;
#[cfg(target_os = "windows")]
use super::command::SystemCommandRunner;
#[cfg(target_os = "windows")]
use crate::Disk;
use crate::{
    Device, DeviceSafety, DiskBackend, DiskInventory, Filesystem, OperationError,
    OperationErrorKind, OperationOutcome, OperationRequest, Partition, PartitionScheme,
    PhysicalDisk, Volume,
};
use serde::Deserialize;

const INVENTORY_SCRIPT: &str = include_str!("windows_inventory.ps1");

#[derive(Deserialize)]
struct StorageInventory {
    disks: Vec<StorageDisk>,
    #[serde(default)]
    unattached_volumes: Vec<StorageVolume>,
    #[serde(default)]
    warnings: Vec<String>,
}

#[derive(Deserialize)]
struct StorageDisk {
    number: u32,
    name: String,
    size_bytes: u64,
    bus_type: String,
    partition_style: String,
    unique_id: Option<String>,
    serial_number: Option<String>,
    path: Option<String>,
    guid: Option<String>,
    signature: Option<u32>,
    system: Option<bool>,
    boot: Option<bool>,
    offline: Option<bool>,
    read_only: Option<bool>,
    clustered: Option<bool>,
    partitions: Vec<StoragePartition>,
    health_status: Option<String>,
    #[serde(default)]
    operational_status: Vec<String>,
}

#[derive(Deserialize)]
struct StoragePartition {
    number: u32,
    guid: Option<String>,
    gpt_type: Option<String>,
    mbr_type: Option<u16>,
    #[serde(rename = "type")]
    kind: Option<String>,
    offset_bytes: u64,
    size_bytes: u64,
    #[serde(default)]
    access_paths: Vec<String>,
    system: Option<bool>,
    boot: Option<bool>,
    active: Option<bool>,
    hidden: Option<bool>,
    read_only: Option<bool>,
    offline: Option<bool>,
    shadow_copy: Option<bool>,
    no_default_drive_letter: Option<bool>,
    volumes: Vec<StorageVolume>,
    page_file: Option<bool>,
}

#[derive(Deserialize)]
struct StorageVolume {
    path: String,
    unique_id: Option<String>,
    drive_letter: Option<String>,
    label: Option<String>,
    filesystem: Option<String>,
    size_bytes: Option<u64>,
    available_bytes: Option<u64>,
    health_status: Option<String>,
    #[serde(default)]
    operational_status: Vec<String>,
}

fn nonempty(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn map_volume(volume: StorageVolume, safety: DeviceSafety, access_paths: &[String]) -> Volume {
    let label = nonempty(volume.label);
    let drive_letter = nonempty(volume.drive_letter)
        .filter(|letter| letter.len() == 1 && letter.as_bytes()[0].is_ascii_uppercase());
    let mut mount_points: Vec<_> = access_paths
        .iter()
        .filter(|path| !crate::operations::valid_volume_path(path))
        .cloned()
        .collect();
    if let Some(letter) = &drive_letter {
        let path = format!("{letter}:\\");
        if !mount_points.contains(&path) {
            mount_points.push(path);
        }
    }
    mount_points.sort();
    mount_points.dedup();
    let filesystem = nonempty(volume.filesystem)
        .filter(|fs| !fs.eq_ignore_ascii_case("unknown") && !fs.eq_ignore_ascii_case("raw"))
        .map(|fs| Filesystem::new(&fs, &fs));
    let stable_id = crate::operations::valid_volume_path(&volume.path).then(|| volume.path.clone());
    let available_bytes = filesystem.as_ref().and(volume.available_bytes);
    Volume {
        device: Device {
            identifier: volume.path.clone(),
            name: label.clone().unwrap_or_else(|| {
                drive_letter
                    .as_ref()
                    .map(|letter| format!("Volume ({letter}:)"))
                    .unwrap_or_else(|| "Unlabelled volume".into())
            }),
            volume_uuid: stable_id
                .as_ref()
                .and_then(|path| {
                    path.strip_prefix(r"\\?\Volume{")
                        .and_then(|value| value.strip_suffix("}\\"))
                })
                .map(str::to_owned),
            stable_id: stable_id.or_else(|| nonempty(volume.unique_id)),
            drive_letter,
            mount_point: mount_points.first().cloned(),
            mount_points,
            filesystem,
            size_bytes: volume.size_bytes,
            available_bytes,
            used_bytes: volume
                .size_bytes
                .zip(available_bytes)
                .and_then(|(size, free)| size.checked_sub(free)),
            safety,
            health_status: nonempty(volume.health_status),
            operational_status: volume.operational_status,
            ..Default::default()
        },
        label,
    }
}

fn parse_inventory(bytes: &[u8]) -> Result<DiskInventory, OperationError> {
    let storage: StorageInventory =
        serde_json::from_slice(bytes).map_err(OperationError::discovery)?;
    let mut inventory = DiskInventory {
        warnings: storage.warnings,
        ..Default::default()
    };
    let mut disk_numbers = std::collections::HashSet::new();
    for disk in storage.disks {
        if !disk_numbers.insert(disk.number) {
            return Err(OperationError::new(
                OperationErrorKind::AmbiguousTarget,
                "Windows returned duplicate disk numbers",
            ));
        }
        let stable_id = nonempty(disk.unique_id)
            .zip(nonempty(disk.path))
            .map(|(id, path)| {
                serde_json::json!([id, nonempty(disk.serial_number), path]).to_string()
            });
        let external = disk.bus_type.eq_ignore_ascii_case("USB")
            && disk.system == Some(false)
            && disk.boot == Some(false)
            && disk.clustered == Some(false)
            && disk.offline == Some(false)
            && disk.read_only == Some(false);
        let mut partition_numbers = std::collections::HashSet::new();
        let mut partitions = Vec::new();
        for partition in disk.partitions {
            if !partition_numbers.insert(partition.number) {
                return Err(OperationError::new(
                    OperationErrorKind::AmbiguousTarget,
                    "Windows returned duplicate partition numbers",
                ));
            }
            let gpt_type = nonempty(partition.gpt_type);
            let mut role = crate::PartitionRole::windows(gpt_type.as_deref(), partition.mbr_type);
            if role == crate::PartitionRole::Unknown {
                role = crate::PartitionRole::from_windows_name(partition.kind.as_deref());
            }
            let recovery = partition
                .kind
                .as_deref()
                .is_some_and(|kind| kind.eq_ignore_ascii_case("Recovery"))
                || gpt_type.as_deref().is_some_and(|kind| {
                    kind.trim_matches(['{', '}'])
                        .eq_ignore_ascii_case("de94bba4-06d1-4d40-a16a-bfd50179d6ac")
                })
                || partition.mbr_type == Some(0x27);
            let system = partition.system.map(|system| {
                system
                    || gpt_type.as_deref().is_some_and(|kind| {
                        kind.trim_matches(['{', '}'])
                            .eq_ignore_ascii_case("c12a7328-f81f-11d2-ba4b-00a0c93ec93b")
                    })
            });
            let safety = DeviceSafety {
                system,
                boot: partition
                    .boot
                    .zip(partition.active)
                    .map(|(boot, active)| boot || active),
                recovery: Some(recovery),
                hidden: partition
                    .hidden
                    .zip(partition.shadow_copy)
                    .map(|(hidden, shadow)| hidden || shadow),
                read_only: partition.read_only,
                offline: partition.offline,
                page_file: partition.page_file,
            };
            let mut volumes: Vec<_> = partition
                .volumes
                .into_iter()
                .map(|volume| map_volume(volume, safety.clone(), &partition.access_paths))
                .collect();
            if partition.no_default_drive_letter == Some(true)
                && volumes
                    .iter()
                    .all(|volume| volume.device.mount_points.is_empty())
            {
                for volume in &mut volumes {
                    volume.device.safety.hidden = Some(true);
                }
            }
            let uuid = nonempty(partition.guid);
            for volume in &mut volumes {
                volume.device.partition_uuid = uuid.clone();
            }
            partitions.push(Partition {
                device: Device {
                    identifier: format!(
                        "PhysicalDrive{}Partition{}",
                        disk.number, partition.number
                    ),
                    name: volumes
                        .iter()
                        .find_map(|v| v.label.clone())
                        .or_else(|| role.friendly_name().map(str::to_owned))
                        .unwrap_or_else(|| format!("Partition {}", partition.number)),
                    partition_uuid: uuid,
                    size_bytes: Some(partition.size_bytes),
                    safety,
                    ..Default::default()
                },
                content_type: nonempty(partition.kind),
                role: Some(role),
                offset_bytes: Some(partition.offset_bytes),
                number: Some(partition.number),
                gpt_type,
                mbr_type: partition.mbr_type,
                active: partition.active,
                shadow_copy: partition.shadow_copy,
                no_default_drive_letter: partition.no_default_drive_letter,
                volumes,
                ..Default::default()
            });
        }
        inventory.disks.push(PhysicalDisk {
            device: Device {
                identifier: format!("PhysicalDrive{}", disk.number),
                name: disk.name,
                size_bytes: Some(disk.size_bytes),
                stable_id,
                health_status: nonempty(disk.health_status),
                operational_status: disk.operational_status,
                safety: DeviceSafety {
                    system: disk.system,
                    boot: disk.boot,
                    offline: disk.offline,
                    read_only: disk.read_only,
                    ..Default::default()
                },
                ..Default::default()
            },
            number: disk.number,
            partition_scheme: PartitionScheme::from_content(Some(&disk.partition_style)),
            connection_type: Some(disk.bus_type),
            internal: external.then_some(false),
            ejectable: Some(false),
            disk_uuid: nonempty(disk.guid),
            mbr_signature: disk.signature,
            partitions,
            ..Default::default()
        });
    }
    inventory.unattached_volumes = storage
        .unattached_volumes
        .into_iter()
        .map(|volume| map_volume(volume, DeviceSafety::default(), &[]))
        .collect();
    crate::operations::annotate_operations(&mut inventory);
    Ok(inventory)
}

fn powershell_path() -> Result<String, OperationError> {
    #[cfg(target_os = "windows")]
    let powershell = {
        let root = std::env::var_os("SystemRoot")
            .ok_or_else(|| OperationError::discovery("SystemRoot is not set"))?;
        let path =
            std::path::PathBuf::from(root).join("System32/WindowsPowerShell/v1.0/powershell.exe");
        path.to_str()
            .ok_or_else(|| OperationError::discovery("Invalid system PowerShell path"))?
            .to_owned()
    };
    #[cfg(not(target_os = "windows"))]
    let powershell = "powershell.exe".to_owned();
    Ok(powershell)
}

fn inventory_with(runner: &impl CommandRunner) -> Result<DiskInventory, OperationError> {
    let powershell = powershell_path()?;
    let output = runner.run(
        &powershell,
        &[
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            INVENTORY_SCRIPT,
        ],
    )?;
    parse_inventory(&output.checked("Windows Storage discovery")?)
}

#[cfg(target_os = "windows")]
pub fn disk_inventory() -> Result<DiskInventory, Box<dyn std::error::Error>> {
    inventory_with(&SystemCommandRunner).map_err(Into::into)
}

#[cfg(target_os = "windows")]
pub fn list_disks() -> Result<Vec<Disk>, Box<dyn std::error::Error>> {
    let output = SystemCommandRunner.run(
        &powershell_path()?,
        &[
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            include_str!("windows_disks.ps1"),
        ],
    )?;
    Ok(serde_json::from_slice(
        &output.checked("Windows disk discovery")?,
    )?)
}

pub(crate) trait VolumeManager {
    fn mount(&self, device: &Device) -> Result<String, OperationError>;
    fn unmount(&self, device: &Device) -> Result<(), OperationError>;
    fn rename(&self, device: &Device, label: &str) -> Result<(), OperationError>;
    fn set_drive_letter(&self, device: &Device, letter: &str) -> Result<(), OperationError>;
}

#[cfg(any(target_os = "windows", test))]
#[path = "windows_volume.rs"]
mod native;

#[cfg(target_os = "windows")]
#[derive(Default)]
pub(crate) struct WindowsBackend<R = SystemCommandRunner, V = native::NativeVolumeManager> {
    runner: R,
    volumes: V,
}

#[cfg(not(target_os = "windows"))]
pub(crate) struct WindowsBackend<R, V> {
    runner: R,
    volumes: V,
}

impl<R: CommandRunner, V: VolumeManager> DiskBackend for WindowsBackend<R, V> {
    fn inventory(&self) -> Result<DiskInventory, OperationError> {
        inventory_with(&self.runner)
    }

    fn execute(
        &self,
        request: &OperationRequest,
        device: &Device,
    ) -> Result<OperationOutcome, OperationError> {
        let fresh = crate::operations::validate_request(&self.inventory()?, request)?;
        if fresh.identity_token != device.identity_token
            || fresh.mount_points != device.mount_points
        {
            return Err(OperationError::new(
                OperationErrorKind::IdentityChanged,
                "Device identity or access paths changed before execution. Refresh and try again.",
            ));
        }
        let message = match request.action {
            crate::DiskOperation::Mount => format!("Mounted at {}", self.volumes.mount(&fresh)?),
            crate::DiskOperation::Unmount => {
                self.volumes.unmount(&fresh)?;
                "Volume unmounted and drive letter removed.".into()
            }
            crate::DiskOperation::Eject => {
                return Err(OperationError::new(
                    OperationErrorKind::UnsupportedOperation,
                    "Windows device ejection is not implemented",
                ));
            }
            crate::DiskOperation::RenameVolume => {
                self.volumes
                    .rename(&fresh, request.volume_label.as_deref().unwrap())?;
                "Volume label updated.".into()
            }
            crate::DiskOperation::SetDriveLetter => {
                let letter = request.drive_letter.as_deref().unwrap();
                self.volumes.set_drive_letter(&fresh, letter)?;
                format!("Drive letter updated to {letter}:")
            }
        };
        Ok(OperationOutcome {
            action: request.action,
            identifier: fresh.identifier,
            message,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::command::{CommandOutput, FixtureRunner};
    use super::*;
    use crate::{DiskOperation, supported_operations_in, validate_operation_in};
    use std::cell::RefCell;

    const FIXTURE: &[u8] = include_bytes!("../../tests/fixtures/windows_inventory.json");

    fn fixture() -> serde_json::Value {
        serde_json::from_slice(FIXTURE).unwrap()
    }
    fn parse(value: serde_json::Value) -> DiskInventory {
        parse_inventory(&serde_json::to_vec(&value).unwrap()).unwrap()
    }
    fn volume(inventory: &DiskInventory) -> &Device {
        &inventory.disks[1].partitions[0].volumes[0].device
    }
    fn request(inventory: &DiskInventory) -> OperationRequest {
        OperationRequest {
            expected_mount_points: None,
            volume_label: None,
            drive_letter: None,
            action: DiskOperation::Unmount,
            identifier: volume(inventory).identifier.clone(),
            expected_identity: volume(inventory).identity_token.clone().unwrap(),
        }
    }

    #[test]
    fn parses_gpt_partitions_and_volumes_without_losing_capacity_or_unicode() {
        let inventory = parse_inventory(FIXTURE).unwrap();
        let disk = &inventory.disks[1];
        assert_eq!(disk.partition_scheme, PartitionScheme::Gpt);
        assert_eq!(disk.partitions[0].number, Some(1));
        assert_eq!(disk.partitions[0].offset_bytes, Some(1048576));
        assert!(disk.partitions[0].device.filesystem.is_none());
        let device = volume(&inventory);
        assert_eq!(device.name, "Données");
        assert_eq!(device.drive_letter.as_deref(), Some("E"));
        assert_eq!(device.size_bytes, Some(63900000000));
        assert_eq!(device.available_bytes, Some(20000000000));
        assert_eq!(device.used_bytes, Some(43900000000));
        assert_eq!(
            device.actions,
            [
                DiskOperation::Unmount,
                DiskOperation::RenameVolume,
                DiskOperation::SetDriveLetter
            ]
        );
        assert_eq!(disk.partitions[0].device.name, "Données");
        assert_eq!(
            disk.partitions[0].role,
            Some(crate::PartitionRole::BasicData)
        );
        assert!(
            inventory.disks[0].partitions[0].volumes[0]
                .device
                .actions
                .is_empty()
        );
        assert!(inventory.unattached_volumes[0].device.actions.is_empty());
        assert!(!disk.device.actions.contains(&DiskOperation::Eject));
    }

    #[test]
    fn parses_mbr_metadata_and_unmounted_volume() {
        let mut value = fixture();
        let disk = &mut value["disks"][1];
        disk["partition_style"] = "MBR".into();
        disk["guid"] = serde_json::Value::Null;
        disk["signature"] = 12345.into();
        let partition = &mut disk["partitions"][0];
        partition["guid"] = serde_json::Value::Null;
        partition["gpt_type"] = serde_json::Value::Null;
        partition["mbr_type"] = 7.into();
        partition["access_paths"] = serde_json::json!([]);
        partition["volumes"][0]["drive_letter"] = serde_json::Value::Null;
        partition["volumes"][0]["filesystem"] = "exFAT".into();
        let inventory = parse(value);
        assert_eq!(inventory.disks[1].partition_scheme, PartitionScheme::Mbr);
        assert_eq!(inventory.disks[1].mbr_signature, Some(12345));
        assert_eq!(inventory.disks[1].partitions[0].mbr_type, Some(7));
        assert_eq!(
            volume(&inventory).actions,
            [
                DiskOperation::Mount,
                DiskOperation::RenameVolume,
                DiskOperation::SetDriveLetter
            ]
        );
        assert!(volume(&inventory).mount_point.is_none());
    }

    #[test]
    fn recovery_hidden_boot_offline_and_unknown_flags_block_management() {
        for (field, value) in [
            ("boot", true.into()),
            ("system", true.into()),
            ("active", true.into()),
            ("hidden", true.into()),
            ("offline", true.into()),
            ("read_only", true.into()),
            ("shadow_copy", true.into()),
            ("boot", serde_json::Value::Null),
            ("page_file", true.into()),
            ("page_file", serde_json::Value::Null),
            ("active", serde_json::Value::Null),
            ("shadow_copy", serde_json::Value::Null),
        ] {
            let mut fixture = fixture();
            fixture["disks"][1]["partitions"][0][field] = value;
            assert!(volume(&parse(fixture)).actions.is_empty(), "{field}");
        }
        for gpt in [
            "de94bba4-06d1-4d40-a16a-bfd50179d6ac",
            "c12a7328-f81f-11d2-ba4b-00a0c93ec93b",
            "e3c9e316-0b5c-4db8-817d-f92df00215ae",
        ] {
            let mut value = fixture();
            value["disks"][1]["partitions"][0]["gpt_type"] = gpt.into();
            assert!(volume(&parse(value)).actions.is_empty());
        }
    }

    #[test]
    fn incomplete_ambiguous_and_changed_identity_are_rejected() {
        let inventory = parse_inventory(FIXTURE).unwrap();
        let request = request(&inventory);
        let mut value = fixture();
        value["disks"][1]["serial_number"] = "replacement".into();
        assert_eq!(
            validate_operation_in(&parse(value), &request)
                .unwrap_err()
                .code,
            OperationErrorKind::IdentityChanged
        );
        let mut duplicate = inventory.clone();
        duplicate.disks.push(duplicate.disks[1].clone());
        assert_eq!(
            validate_operation_in(&duplicate, &request)
                .unwrap_err()
                .code,
            OperationErrorKind::AmbiguousTarget
        );
        let mut incomplete = inventory;
        incomplete
            .warnings
            .push("missing partition information".into());
        assert_eq!(
            validate_operation_in(&incomplete, &request)
                .unwrap_err()
                .code,
            OperationErrorKind::IncompleteDiscovery
        );
        let mut duplicate = fixture();
        duplicate["disks"][1]["number"] = 0.into();
        assert_eq!(
            parse_inventory(&serde_json::to_vec(&duplicate).unwrap())
                .unwrap_err()
                .code,
            OperationErrorKind::AmbiguousTarget
        );
    }

    #[test]
    fn unsupported_directory_mounts_and_filesystems_have_no_actions() {
        for fs in ["ReFS", "RAW", "Unknown"] {
            let mut value = fixture();
            value["disks"][1]["partitions"][0]["volumes"][0]["filesystem"] = fs.into();
            assert!(volume(&parse(value)).actions.is_empty());
        }
        let mut value = fixture();
        value["disks"][1]["partitions"][0]["access_paths"] =
            serde_json::json!(["E:\\", "C:\\Data\\"]);
        let inventory = parse(value);
        assert!(
            supported_operations_in(&inventory, &volume(&inventory).identifier)
                .unwrap()
                .actions
                .is_empty()
        );
    }

    #[test]
    fn roles_and_friendly_names_preserve_labels_and_hide_raw_identifiers() {
        for (guid, role, name) in [
            (
                "{C12A7328-F81F-11D2-BA4B-00A0C93EC93B}",
                crate::PartitionRole::EfiSystem,
                "EFI System Partition",
            ),
            (
                "de94bba4-06d1-4d40-a16a-bfd50179d6ac",
                crate::PartitionRole::Recovery,
                "Recovery Partition",
            ),
            (
                "e3c9e316-0b5c-4db8-817d-f92df00215ae",
                crate::PartitionRole::MicrosoftReserved,
                "Microsoft Reserved",
            ),
            ("unknown", crate::PartitionRole::Unknown, "Partition 1"),
        ] {
            let mut value = fixture();
            let p = &mut value["disks"][1]["partitions"][0];
            p["gpt_type"] = guid.into();
            if role == crate::PartitionRole::Unknown {
                p["type"] = serde_json::Value::Null;
            }
            p["volumes"][0]["label"] = serde_json::Value::Null;
            let inventory = parse(value);
            let p = &inventory.disks[1].partitions[0];
            assert_eq!(p.role, Some(role));
            assert_eq!(p.device.name, name);
            assert!(volume(&inventory).actions.is_empty());
        }
    }

    #[test]
    fn management_parameters_are_validated_by_backend_not_advertised_actions() {
        let inventory = parse_inventory(FIXTURE).unwrap();
        let mut request = request(&inventory);
        request.expected_mount_points = Some(volume(&inventory).mount_points.clone());
        request.action = DiskOperation::RenameVolume;
        request.volume_label = Some("Données USB".into());
        assert!(validate_operation_in(&inventory, &request).is_ok());
        let caps = supported_operations_in(&inventory, &request.identifier).unwrap();
        assert_eq!(caps.label_max_length, Some(32));
        assert!(!caps.limitations.is_empty());
        for invalid in [
            "x".repeat(33),
            "bad\0label".into(),
            "trailing ".into(),
            "bad/label".into(),
        ] {
            request.volume_label = Some(invalid);
            assert_eq!(
                validate_operation_in(&inventory, &request)
                    .unwrap_err()
                    .code,
                OperationErrorKind::InvalidRequest
            );
        }
        request.volume_label = Some(String::new()); // Explicit label removal is valid.
        assert!(validate_operation_in(&inventory, &request).is_ok());
        request.action = DiskOperation::SetDriveLetter;
        request.volume_label = None;
        for invalid in ["C", "A", "e", "F:\\", "FF", ""] {
            request.drive_letter = Some(invalid.into());
            assert_eq!(
                validate_operation_in(&inventory, &request)
                    .unwrap_err()
                    .code,
                OperationErrorKind::InvalidRequest
            );
        }
        request.drive_letter = Some("E".into());
        assert_eq!(
            validate_operation_in(&inventory, &request)
                .unwrap_err()
                .code,
            OperationErrorKind::InvalidState
        );
        request.drive_letter = Some("F".into());
        assert!(validate_operation_in(&inventory, &request).is_ok());
        request.expected_mount_points = Some(vec!["H:\\".into()]);
        assert_eq!(
            validate_operation_in(&inventory, &request)
                .unwrap_err()
                .code,
            OperationErrorKind::IdentityChanged
        );
        request.expected_mount_points = None;
        assert_eq!(
            validate_operation_in(&inventory, &request)
                .unwrap_err()
                .code,
            OperationErrorKind::IdentityChanged
        );
        request.expected_mount_points = Some(volume(&inventory).mount_points.clone());
        let mut incomplete = inventory.clone();
        incomplete.warnings.push("Partial".into());
        assert_eq!(
            validate_operation_in(&incomplete, &request)
                .unwrap_err()
                .code,
            OperationErrorKind::IncompleteDiscovery
        );
    }

    #[test]
    fn fat_label_limit_and_protected_management_are_enforced() {
        let mut value = fixture();
        value["disks"][1]["partitions"][0]["volumes"][0]["filesystem"] = "exFAT".into();
        let inventory = parse(value);
        let mut request = request(&inventory);
        request.expected_mount_points = Some(volume(&inventory).mount_points.clone());
        request.action = DiskOperation::RenameVolume;
        request.volume_label = Some("x".repeat(12));
        assert_eq!(
            validate_operation_in(&inventory, &request)
                .unwrap_err()
                .code,
            OperationErrorKind::InvalidRequest
        );
        for action in [DiskOperation::RenameVolume, DiskOperation::SetDriveLetter] {
            let mut value = fixture();
            value["disks"][1]["partitions"][0]["page_file"] = true.into();
            let protected = parse(value);
            request.action = action;
            assert_eq!(
                validate_operation_in(&protected, &request)
                    .unwrap_err()
                    .code,
                OperationErrorKind::ProtectedDevice
            );
            assert!(
                supported_operations_in(&protected, &request.identifier)
                    .unwrap()
                    .unsupported_reason
                    .is_some()
            );
        }
    }

    struct FakeVolumes {
        calls: RefCell<Vec<DiskOperation>>,
        failure: bool,
    }
    impl VolumeManager for FakeVolumes {
        fn rename(&self, _: &Device, _: &str) -> Result<(), OperationError> {
            self.calls.borrow_mut().push(DiskOperation::RenameVolume);
            Ok(())
        }
        fn set_drive_letter(&self, _: &Device, _: &str) -> Result<(), OperationError> {
            self.calls.borrow_mut().push(DiskOperation::SetDriveLetter);
            Ok(())
        }
        fn mount(&self, _: &Device) -> Result<String, OperationError> {
            self.calls.borrow_mut().push(DiskOperation::Mount);
            Ok("F:\\".into())
        }
        fn unmount(&self, _: &Device) -> Result<(), OperationError> {
            self.calls.borrow_mut().push(DiskOperation::Unmount);
            if self.failure {
                Err(OperationError::new(
                    OperationErrorKind::PermissionDenied,
                    "Access denied",
                ))
            } else {
                Ok(())
            }
        }
    }
    fn step(bytes: &[u8]) -> (String, Vec<String>, Result<CommandOutput, OperationError>) {
        (
            powershell_path().unwrap(),
            vec![
                "-NoProfile".into(),
                "-NonInteractive".into(),
                "-Command".into(),
                INVENTORY_SCRIPT.into(),
            ],
            Ok(CommandOutput {
                success: true,
                code: Some(0),
                stdout: bytes.to_vec(),
                stderr: vec![],
            }),
        )
    }
    #[test]
    fn revalidates_before_native_execution_and_propagates_permission_errors() {
        let inventory = parse_inventory(FIXTURE).unwrap();
        let request = request(&inventory);
        let mut changed = fixture();
        changed["disks"][1]["unique_id"] = "replacement".into();
        let backend = WindowsBackend {
            runner: FixtureRunner::new(vec![
                step(FIXTURE),
                step(&serde_json::to_vec(&changed).unwrap()),
            ]),
            volumes: FakeVolumes {
                calls: RefCell::default(),
                failure: false,
            },
        };
        assert_eq!(
            backend.perform(&request).unwrap_err().code,
            OperationErrorKind::IdentityChanged
        );
        assert!(backend.volumes.calls.borrow().is_empty());
        let backend = WindowsBackend {
            runner: FixtureRunner::new(vec![step(FIXTURE), step(FIXTURE)]),
            volumes: FakeVolumes {
                calls: RefCell::default(),
                failure: true,
            },
        };
        assert_eq!(
            backend.perform(&request).unwrap_err().code,
            OperationErrorKind::PermissionDenied
        );
        assert_eq!(*backend.volumes.calls.borrow(), [DiskOperation::Unmount]);
        backend.runner.assert_finished();
    }

    #[test]
    fn new_management_dispatch_is_revalidated_before_any_native_call() {
        let inventory = parse_inventory(FIXTURE).unwrap();
        for action in [DiskOperation::RenameVolume, DiskOperation::SetDriveLetter] {
            let mut request = request(&inventory);
            request.action = action;
            request.expected_mount_points = Some(volume(&inventory).mount_points.clone());
            if action == DiskOperation::RenameVolume {
                request.volume_label = Some("New label".into());
            } else {
                request.drive_letter = Some("H".into());
            }
            let backend = WindowsBackend {
                runner: FixtureRunner::new(vec![step(FIXTURE), step(FIXTURE)]),
                volumes: FakeVolumes {
                    calls: RefCell::default(),
                    failure: false,
                },
            };
            assert_eq!(backend.perform(&request).unwrap().action, action);
            assert_eq!(*backend.volumes.calls.borrow(), [action]);
            backend.runner.assert_finished();
            let mut changed = fixture();
            changed["disks"][1]["partitions"][0]["page_file"] = true.into();
            let backend = WindowsBackend {
                runner: FixtureRunner::new(vec![
                    step(FIXTURE),
                    step(&serde_json::to_vec(&changed).unwrap()),
                ]),
                volumes: FakeVolumes {
                    calls: RefCell::default(),
                    failure: false,
                },
            };
            assert_eq!(
                backend.perform(&request).unwrap_err().code,
                OperationErrorKind::ProtectedDevice
            );
            assert!(backend.volumes.calls.borrow().is_empty());
        }
    }
}
