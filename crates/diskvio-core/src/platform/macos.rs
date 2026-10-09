use crate::{
    ApfsContainer, ApfsVolume, Device, Disk, DiskInventory, Filesystem, Partition, PartitionScheme,
    PhysicalDisk,
};
use serde::{Deserialize, de::DeserializeOwned};
use std::{
    error::Error,
    process::{Command, Stdio},
};

#[derive(Default, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct DiskList {
    #[serde(default)]
    whole_disks: Vec<String>,
    #[serde(default, rename = "AllDisksAndPartitions")]
    entries: Vec<DiskEntry>,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct DiskEntry {
    device_identifier: String,
    size: Option<u64>,
    content: Option<String>,
    volume_name: Option<String>,
    mount_point: Option<String>,
    #[serde(rename = "DiskUUID")]
    disk_uuid: Option<String>,
    #[serde(default)]
    partitions: Vec<DiskEntry>,
    #[serde(default, rename = "APFSVolumes")]
    apfs_volumes: Vec<SnapshotSource>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct SnapshotSource {
    device_identifier: String,
    #[serde(default)]
    mounted_snapshots: Vec<SnapshotEntry>,
}

#[derive(Deserialize)]
struct SnapshotEntry {
    #[serde(rename = "SnapshotBSD")]
    identifier: String,
    #[serde(rename = "SnapshotName")]
    name: Option<String>,
    #[serde(rename = "SnapshotUUID")]
    uuid: Option<String>,
    #[serde(rename = "SnapshotMountPoint")]
    mount_point: Option<String>,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct DiskInfo {
    device_identifier: Option<String>,
    virtual_or_physical: Option<String>,
    whole_disk: Option<bool>,
    media_name: Option<String>,
    device_model: Option<String>,
    volume_name: Option<String>,
    total_size: Option<u64>,
    size: Option<u64>,
    volume_size: Option<u64>,
    bus_protocol: Option<String>,
    protocol: Option<String>,
    content: Option<String>,
    internal: Option<bool>,
    removable: Option<bool>,
    removable_media: Option<bool>,
    ejectable: Option<bool>,
    mount_point: Option<String>,
    #[serde(rename = "VolumeUUID")]
    volume_uuid: Option<String>,
    #[serde(rename = "DiskUUID")]
    disk_uuid: Option<String>,
    filesystem_type: Option<String>,
    filesystem_name: Option<String>,
    filesystem_user_visible_name: Option<String>,
    volume_free_space: Option<u64>,
    volume_used_space: Option<u64>,
    partition_map_partition_offset: Option<u64>,
    #[serde(rename = "APFSContainerReference")]
    apfs_container_reference: Option<String>,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ApfsList {
    #[serde(default)]
    containers: Vec<ContainerEntry>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ContainerEntry {
    container_reference: String,
    #[serde(rename = "APFSContainerUUID")]
    uuid: Option<String>,
    capacity_ceiling: Option<u64>,
    capacity_free: Option<u64>,
    #[serde(default)]
    physical_stores: Vec<DiskEntry>,
    #[serde(default)]
    volumes: Vec<VolumeEntry>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct VolumeEntry {
    device_identifier: String,
    name: Option<String>,
    #[serde(rename = "APFSVolumeUUID")]
    uuid: Option<String>,
    capacity_in_use: Option<u64>,
    capacity_quota: Option<u64>,
    capacity_reserve: Option<u64>,
    #[serde(default)]
    roles: Vec<String>,
    encryption: Option<bool>,
    locked: Option<bool>,
    mount_point: Option<String>,
}

pub fn list_disks() -> Result<Vec<Disk>, Box<dyn Error>> {
    let list: DiskList = read_plist(&["list", "-plist", "physical"])?;
    let mut disks = Vec::new();
    for identifier in &list.whole_disks {
        let Some(number) = disk_number(identifier) else {
            continue;
        };
        let info: DiskInfo = read_plist(&["info", "-plist", identifier])?;
        let entry = list
            .entries
            .iter()
            .find(|entry| &entry.device_identifier == identifier);
        if let Some(disk) = map_disk(identifier, number, &info, entry) {
            disks.push(disk);
        }
    }
    disks.sort_by_key(|disk| disk.number);
    Ok(disks)
}

pub fn disk_inventory() -> Result<DiskInventory, Box<dyn Error>> {
    let list: DiskList = read_plist(&["list", "-plist", "physical"])?;
    let mut inventory = DiskInventory::default();
    for entry in &list.entries {
        let Some(number) = disk_number(&entry.device_identifier) else {
            continue;
        };
        let info = optional_info(&entry.device_identifier, &mut inventory.warnings);
        if info.virtual_or_physical.as_deref() == Some("Virtual") || info.whole_disk == Some(false)
        {
            continue;
        }
        let mut partitions = Vec::new();
        // Only entries beneath a physical disk are partitions. Synthesized APFS
        // volumes are mapped separately from `apfs list`, never from `list` slices.
        for partition in &entry.partitions {
            let info = optional_info(&partition.device_identifier, &mut inventory.warnings);
            partitions.push(Partition {
                device: map_device(&partition.device_identifier, &info, Some(partition)),
                content_type: info.content.clone().or_else(|| partition.content.clone()),
                offset_bytes: info.partition_map_partition_offset,
                apfs_container_id: info.apfs_container_reference,
            });
        }
        inventory.disks.push(PhysicalDisk {
            device: map_device(&entry.device_identifier, &info, Some(entry)),
            number,
            partition_scheme: PartitionScheme::from_content(
                entry.content.as_deref().or(info.content.as_deref()),
            ),
            connection_type: nonempty(info.bus_protocol.as_deref())
                .or_else(|| nonempty(info.protocol.as_deref()))
                .map(str::to_owned),
            internal: info.internal,
            removable: info.removable.or(info.removable_media),
            ejectable: info.ejectable,
            registry_entry_id: None,
            partitions,
        });
    }
    let apfs: ApfsList = match read_plist(&["apfs", "list", "-plist"]) {
        Ok(apfs) => apfs,
        Err(error) => {
            inventory.warnings.push(format!("APFS discovery: {error}"));
            ApfsList::default()
        }
    };
    for entry in apfs.containers {
        // Ignore containers belonging only to disk images / virtual devices.
        let relevant = entry.physical_stores.iter().any(|store| {
            inventory.disks.iter().any(|disk| {
                disk.device.identifier == store.device_identifier
                    || disk
                        .partitions
                        .iter()
                        .any(|p| p.device.identifier == store.device_identifier)
            })
        });
        if !relevant {
            continue;
        }
        let container_id = entry.container_reference.clone();
        for disk in &mut inventory.disks {
            for partition in &mut disk.partitions {
                if entry
                    .physical_stores
                    .iter()
                    .any(|store| store.device_identifier == partition.device.identifier)
                {
                    partition.apfs_container_id = Some(container_id.clone());
                }
            }
        }
        let volumes = entry
            .volumes
            .into_iter()
            .map(|volume| {
                let info = optional_info(&volume.device_identifier, &mut inventory.warnings);
                map_volume(volume, &entry.capacity_ceiling, &entry.capacity_free, &info)
            })
            .collect();
        inventory.apfs_containers.push(ApfsContainer {
            device: Device {
                identifier: container_id.clone(),
                name: format!("APFS Container {container_id}"),
                size_bytes: entry.capacity_ceiling,
                used_bytes: entry
                    .capacity_ceiling
                    .zip(entry.capacity_free)
                    .and_then(|(total, free)| total.checked_sub(free)),
                available_bytes: entry.capacity_free,
                ..Default::default()
            },
            uuid: entry.uuid,
            physical_store_ids: entry
                .physical_stores
                .into_iter()
                .map(|store| store.device_identifier)
                .collect(),
            volumes,
        });
    }
    inventory.disks.sort_by_key(|disk| disk.number);
    // `list physical` intentionally excludes synthesized snapshots. Enrich only
    // already-discovered APFS volumes using explicit MountedSnapshots ownership.
    // Snapshot BSD devices never become partitions or additional APFS volumes.
    match read_plist::<DiskList>(&["list", "-plist"]) {
        Ok(all) => {
            for source in all.entries.into_iter().flat_map(|entry| entry.apfs_volumes) {
                if let Some(volume) = inventory
                    .apfs_containers
                    .iter_mut()
                    .flat_map(|container| &mut container.volumes)
                    .find(|volume| volume.device.identifier == source.device_identifier)
                {
                    volume.mounted_snapshots = source
                        .mounted_snapshots
                        .into_iter()
                        .map(|snapshot| crate::ApfsSnapshot {
                            identifier: snapshot.identifier,
                            name: snapshot.name,
                            uuid: snapshot.uuid,
                            mount_point: snapshot.mount_point.filter(|path| !path.is_empty()),
                        })
                        .collect();
                }
            }
        }
        Err(error) => inventory
            .warnings
            .push(format!("Mounted snapshot discovery: {error}")),
    }
    match media_registry_ids() {
        Ok(ids) => {
            for disk in &mut inventory.disks {
                disk.registry_entry_id = ids.get(&disk.device.identifier).copied();
            }
        }
        Err(error) => inventory
            .warnings
            .push(format!("Device identity verification: {error}")),
    }
    crate::operations::annotate_operations(&mut inventory);
    Ok(inventory)
}

fn map_volume(
    volume: VolumeEntry,
    total: &Option<u64>,
    free: &Option<u64>,
    info: &DiskInfo,
) -> ApfsVolume {
    let mut device = map_device(&volume.device_identifier, info, None);
    device.name = nonempty(volume.name.as_deref())
        .unwrap_or(&device.name)
        .to_owned();
    device.filesystem = Some(
        info.filesystem_type
            .as_deref()
            .map(|kind| {
                Filesystem::new(
                    kind,
                    info.filesystem_user_visible_name
                        .as_deref()
                        .unwrap_or("APFS"),
                )
            })
            .unwrap_or_else(|| Filesystem::new("apfs", "APFS")),
    );
    // APFS volumes share container capacity, rather than owning fixed partitions.
    let quota = volume.capacity_quota.filter(|quota| *quota > 0);
    device.size_bytes = quota.or(*total);
    device.used_bytes = volume.capacity_in_use;
    device.available_bytes = match (quota, volume.capacity_in_use, *free) {
        (Some(quota), Some(used), Some(free)) => {
            quota.checked_sub(used).map(|remaining| remaining.min(free))
        }
        (Some(_), _, _) => None,
        (None, _, free) => free,
    };
    device.volume_uuid = volume.uuid.or(device.volume_uuid);
    // DiskUUID on APFS volumes is a volume UUID, not a partition UUID.
    device.partition_uuid = None;
    device.mount_point = device
        .mount_point
        .or_else(|| nonempty(volume.mount_point.as_deref()).map(str::to_owned));
    ApfsVolume {
        device,
        mounted_snapshots: vec![],
        roles: volume.roles,
        encrypted: volume.encryption,
        locked: volume.locked,
        quota_bytes: quota,
        reserve_bytes: volume.capacity_reserve.filter(|reserve| *reserve > 0),
    }
}

fn map_device(identifier: &str, info: &DiskInfo, entry: Option<&DiskEntry>) -> Device {
    let filesystem = nonempty(info.filesystem_type.as_deref()).map(|kind| {
        Filesystem::new(
            kind,
            nonempty(info.filesystem_user_visible_name.as_deref())
                .or_else(|| nonempty(info.filesystem_name.as_deref()))
                .unwrap_or(kind),
        )
    });
    let size = info
        .total_size
        .or(info.size)
        .or_else(|| entry.and_then(|entry| entry.size));
    // FreeSpace on a physical disk describes partition-map space, not filesystem
    // availability. Do not derive used filesystem bytes from it.
    let available = filesystem.as_ref().and(info.volume_free_space);
    let used = filesystem.as_ref().and(info.volume_used_space).or_else(|| {
        info.volume_size
            .filter(|size| *size > 0)
            .zip(available)
            .and_then(|(total, free)| total.checked_sub(free))
    });
    Device {
        identifier: identifier.to_owned(),
        name: nonempty(info.volume_name.as_deref())
            .or_else(|| entry.and_then(|e| nonempty(e.volume_name.as_deref())))
            .or_else(|| nonempty(info.media_name.as_deref()))
            .or_else(|| nonempty(info.device_model.as_deref()))
            .unwrap_or(identifier)
            .to_owned(),
        media_name: nonempty(info.media_name.as_deref()).map(str::to_owned),
        size_bytes: size,
        filesystem,
        mount_point: nonempty(info.mount_point.as_deref())
            .or_else(|| entry.and_then(|e| nonempty(e.mount_point.as_deref())))
            .map(str::to_owned),
        volume_uuid: info.volume_uuid.clone(),
        partition_uuid: info
            .disk_uuid
            .clone()
            .or_else(|| entry.and_then(|e| e.disk_uuid.clone())),
        used_bytes: used,
        available_bytes: available,
        ..Default::default()
    }
}

fn optional_info(identifier: &str, warnings: &mut Vec<String>) -> DiskInfo {
    match read_plist::<DiskInfo>(&["info", "-plist", identifier]) {
        Ok(info)
            if info
                .device_identifier
                .as_deref()
                .is_none_or(|actual| actual == identifier) =>
        {
            info
        }
        Ok(_) => {
            warnings.push(format!(
                "{identifier}: device identity changed during discovery"
            ));
            DiskInfo::default()
        }
        Err(error) => {
            warnings.push(format!("{identifier}: {error}"));
            DiskInfo::default()
        }
    }
}

fn read_plist<T: DeserializeOwned>(args: &[&str]) -> Result<T, Box<dyn Error>> {
    plist::from_bytes(&run_diskutil(args)?).map_err(|error| {
        format!("Could not parse diskutil {} plist: {error}", args.join(" ")).into()
    })
}

fn run_diskutil(args: &[&str]) -> Result<Vec<u8>, Box<dyn Error>> {
    let output = Command::new("/usr/sbin/diskutil")
        .args(args)
        .stdin(Stdio::null())
        .output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let detail = if stderr.trim().is_empty() {
            stdout.trim()
        } else {
            stderr.trim()
        };
        return Err(format!(
            "diskutil {} failed ({}): {detail}",
            args.join(" "),
            output.status
        )
        .into());
    }
    Ok(output.stdout)
}

fn disk_number(identifier: &str) -> Option<u32> {
    identifier.strip_prefix("disk")?.parse().ok()
}
fn nonempty(value: Option<&str>) -> Option<&str> {
    value.filter(|value| !value.trim().is_empty())
}

fn map_disk(
    identifier: &str,
    number: u32,
    info: &DiskInfo,
    entry: Option<&DiskEntry>,
) -> Option<Disk> {
    if info.virtual_or_physical.as_deref() == Some("Virtual") || info.whole_disk == Some(false) {
        return None;
    }
    let device = map_device(identifier, info, entry);
    Some(Disk {
        number,
        name: device.name,
        size_bytes: device.size_bytes.unwrap_or(0),
        bus_type: nonempty(info.bus_protocol.as_deref())
            .or_else(|| nonempty(info.protocol.as_deref()))
            .unwrap_or("Unknown")
            .to_owned(),
        partition_style: entry
            .and_then(|entry| entry.content.as_deref())
            .or(info.content.as_deref())
            .filter(|content| content.ends_with("_partition_scheme"))
            .unwrap_or("Unknown")
            .to_owned(),
    })
}

fn media_registry_ids() -> Result<std::collections::HashMap<String, u64>, Box<dyn Error>> {
    let output = Command::new("/usr/sbin/ioreg")
        .args(["-a", "-r", "-c", "IOMedia"])
        .output()?;
    if !output.status.success() {
        return Err("Could not inspect IOMedia registry identity".into());
    }
    let value = plist::Value::from_reader(std::io::Cursor::new(output.stdout))?;
    let mut ids = std::collections::HashMap::new();
    fn visit(value: &plist::Value, ids: &mut std::collections::HashMap<String, u64>) {
        if let Some(array) = value.as_array() {
            for child in array {
                visit(child, ids);
            }
        }
        if let Some(dict) = value.as_dictionary() {
            if let (Some(name), Some(id)) = (
                dict.get("BSD Name").and_then(plist::Value::as_string),
                dict.get("IORegistryEntryID")
                    .and_then(plist::Value::as_unsigned_integer),
            ) {
                ids.insert(name.to_owned(), id);
            }
            if let Some(children) = dict.get("IORegistryEntryChildren") {
                visit(children, ids);
            }
        }
    }
    visit(&value, &mut ids);
    Ok(ids)
}

pub(crate) struct MacOsBackend;
impl crate::DiskBackend for MacOsBackend {
    fn inventory(&self) -> Result<DiskInventory, String> {
        disk_inventory().map_err(|error| error.to_string())
    }
    fn perform(
        &self,
        request: &crate::OperationRequest,
    ) -> Result<crate::OperationOutcome, String> {
        let inventory = self.inventory()?;
        if !inventory.warnings.is_empty() {
            return Err("Device discovery was incomplete. Refresh and resolve discovery errors before performing operations.".into());
        }
        let device = crate::operations::validate_request(&inventory, request)?;
        let current: DiskInfo = read_plist(&["info", "-plist", &request.identifier])
            .map_err(|error| error.to_string())?;
        if current.device_identifier.as_deref() != Some(request.identifier.as_str())
            || current.internal != Some(false)
            || device
                .volume_uuid
                .as_ref()
                .is_some_and(|uuid| current.volume_uuid.as_ref() != Some(uuid))
            || device
                .partition_uuid
                .as_ref()
                .is_some_and(|uuid| current.disk_uuid.as_ref() != Some(uuid))
        {
            return Err(
                "Device identity or location changed before the operation. Refresh and try again."
                    .into(),
            );
        }
        let current_ids = media_registry_ids().map_err(|error| error.to_string())?;
        if inventory
            .disks
            .iter()
            .any(|disk| current_ids.get(&disk.device.identifier).copied() != disk.registry_entry_id)
        {
            return Err(
                "Disk topology changed before the operation. Refresh and try again.".into(),
            );
        }
        let verb = match request.action {
            crate::DiskOperation::Mount => "mount",
            crate::DiskOperation::Unmount => "unmount",
            crate::DiskOperation::Eject => "eject",
        };
        // Absolute executable, separate argv, no shell, no force, no sudo and no
        // authorization helper. diskutil reports OS permission/dissenter failures.
        run_diskutil(&[verb, &request.identifier]).map_err(|error| error.to_string())?;
        Ok(crate::OperationOutcome {
            action: request.action,
            identifier: request.identifier.clone(),
            message: format!(
                "{} {} successfully.",
                match request.action {
                    crate::DiskOperation::Mount => "Mounted",
                    crate::DiskOperation::Unmount => "Unmounted",
                    crate::DiskOperation::Eject => "Ejected",
                },
                request.identifier
            ),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info(xml: &str) -> DiskInfo {
        plist::from_bytes(format!("<plist version=\"1.0\"><dict>{xml}</dict></plist>").as_bytes())
            .unwrap()
    }

    #[test]
    fn maps_physical_disk_and_rejects_virtual_devices() {
        let mut info = info(
            "<key>MediaName</key><string>SSD</string><key>TotalSize</key><integer>500000000000</integer><key>Internal</key><true/>",
        );
        let disk = map_disk("disk0", 0, &info, None).unwrap();
        assert_eq!(disk.name, "SSD");
        assert_eq!(disk.size_bytes, 500_000_000_000);
        assert_eq!(disk_number("disk4s1"), None);
        info.virtual_or_physical = Some("Virtual".into());
        assert!(map_disk("disk0", 0, &info, None).is_none());
    }

    #[test]
    fn partition_types_are_not_filesystems_and_disk_free_space_is_not_volume_space() {
        let info = info(
            "<key>Content</key><string>Apple_APFS</string><key>FreeSpace</key><integer>0</integer><key>DiskUUID</key><string>partition-id</string>",
        );
        let device = map_device("disk0s2", &info, None);
        assert!(device.filesystem.is_none());
        assert!(device.used_bytes.is_none());
        assert!(device.available_bytes.is_none());
        assert_eq!(device.partition_uuid.as_deref(), Some("partition-id"));
    }

    #[test]
    fn maps_apfs_shared_capacity_and_quota_without_partition_uuid() {
        let volume: VolumeEntry = plist::from_bytes(br#"<plist version="1.0"><dict>
            <key>DeviceIdentifier</key><string>disk3s1</string><key>Name</key><string>Data</string>
            <key>APFSVolumeUUID</key><string>volume-id</string><key>CapacityInUse</key><integer>100</integer>
            <key>CapacityQuota</key><integer>200</integer><key>Locked</key><true/>
            </dict></plist>"#).unwrap();
        let mapped = map_volume(volume, &Some(1000), &Some(700), &DiskInfo::default());
        assert_eq!(mapped.device.size_bytes, Some(200));
        assert_eq!(mapped.device.available_bytes, Some(100));
        assert_eq!(mapped.device.used_bytes, Some(100));
        assert_eq!(mapped.device.volume_uuid.as_deref(), Some("volume-id"));
        assert!(mapped.device.partition_uuid.is_none());
        assert_eq!(mapped.locked, Some(true));
    }

    #[test]
    fn parses_nested_physical_partitions_and_multi_store_container() {
        let list: DiskList = plist::from_bytes(br#"<plist version="1.0"><dict><key>AllDisksAndPartitions</key><array><dict>
            <key>DeviceIdentifier</key><string>disk0</string><key>Content</key><string>GUID_partition_scheme</string>
            <key>Partitions</key><array><dict><key>DeviceIdentifier</key><string>disk0s2</string><key>Size</key><integer>1000</integer></dict></array>
            </dict></array></dict></plist>"#).unwrap();
        assert_eq!(list.entries[0].partitions[0].device_identifier, "disk0s2");
        let apfs: ApfsList = plist::from_bytes(br#"<plist version="1.0"><dict><key>Containers</key><array><dict>
            <key>ContainerReference</key><string>disk3</string><key>PhysicalStores</key><array>
            <dict><key>DeviceIdentifier</key><string>disk0s2</string></dict><dict><key>DeviceIdentifier</key><string>disk1s2</string></dict>
            </array><key>Volumes</key><array><dict><key>DeviceIdentifier</key><string>disk3s1</string></dict></array>
            </dict></array></dict></plist>"#).unwrap();
        assert_eq!(apfs.containers[0].physical_stores.len(), 2);
        assert_eq!(apfs.containers[0].volumes[0].device_identifier, "disk3s1");
    }

    #[test]
    fn mounted_snapshots_are_owned_by_source_volume() {
        let list: DiskList = plist::from_bytes(br#"<plist version="1.0"><dict><key>AllDisksAndPartitions</key><array><dict>
            <key>DeviceIdentifier</key><string>disk3</string><key>APFSVolumes</key><array><dict>
            <key>DeviceIdentifier</key><string>disk3s1</string><key>MountedSnapshots</key><array><dict>
            <key>SnapshotBSD</key><string>disk3s1s1</string><key>SnapshotMountPoint</key><string>/</string>
            <key>SnapshotUUID</key><string>snapshot-uuid</string></dict></array></dict></array>
            </dict></array></dict></plist>"#).unwrap();
        assert!(list.entries[0].partitions.is_empty());
        assert_eq!(list.entries[0].apfs_volumes[0].device_identifier, "disk3s1");
        assert_eq!(
            list.entries[0].apfs_volumes[0].mounted_snapshots[0]
                .mount_point
                .as_deref(),
            Some("/")
        );
    }

    #[test]
    fn mounted_filesystem_capacity_and_uuid_are_separate() {
        let info = info(
            "<key>FilesystemType</key><string>exfat</string><key>VolumeSize</key><integer>1000</integer><key>VolumeFreeSpace</key><integer>300</integer><key>MountPoint</key><string>/Volumes/USB</string><key>VolumeUUID</key><string>volume</string><key>DiskUUID</key><string>partition</string>",
        );
        let device = map_device("disk2s1", &info, None);
        assert_eq!(device.used_bytes, Some(700));
        assert_eq!(device.mount_point.as_deref(), Some("/Volumes/USB"));
        assert_eq!(device.volume_uuid.as_deref(), Some("volume"));
        assert_eq!(device.partition_uuid.as_deref(), Some("partition"));
    }
}
