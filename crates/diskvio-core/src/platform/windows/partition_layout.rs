use super::{StorageDisk, StorageInventory, nonempty, powershell_path};
use crate::platform::command::CommandRunner;
use crate::*;

const PLANNING_SCRIPT: &str = concat!(
    "$DiskvioPartitionPlanning = $true\n",
    include_str!("../windows_inventory.ps1")
);

#[derive(Debug, Clone, PartialEq, Eq)]
struct NativePartition {
    number: u32,
    id: String,
    kind: String,
    offset: u64,
    size: u64,
    attributes: u64,
    service: bool,
    name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NativeLayout {
    geometry: GptGeometry,
    partitions: Vec<NativePartition>,
    serial: String,
    unique_ids: Vec<(u16, String)>,
    number: u32,
    usb: bool,
    removable: bool,
}

trait LayoutQuery {
    fn query(&self, disk: &StorageDisk) -> Result<NativeLayout, PlanningIssue>;
}

fn error(code: PlanningErrorCode, message: impl Into<String>) -> PlanningIssue {
    PlanningIssue::new(code, message)
}

fn persistent_identity(disk: &StorageDisk) -> Option<PhysicalDiskIdentity> {
    if disk.unique_id.is_none() && disk.serial_number.is_none() {
        return None;
    }
    Some(PhysicalDiskIdentity {
        unique_id: nonempty(disk.unique_id.clone()).unwrap_or_default(),
        unique_id_format: disk.unique_id_format.unwrap_or(0),
        serial_number: nonempty(disk.serial_number.clone()).unwrap_or_default(),
    })
}

fn storage_with(runner: &impl CommandRunner) -> Result<StorageInventory, PlanningIssue> {
    let read = || -> Result<StorageInventory, OperationError> {
        let output = runner.run(
            &powershell_path()?,
            &["-NoProfile", "-NonInteractive", "-Command", PLANNING_SCRIPT],
        )?;
        serde_json::from_slice(&output.checked("Windows partition safety discovery")?)
            .map_err(OperationError::discovery)
    };
    read().map_err(|e| PlanningIssue {
        code: if e.code == OperationErrorKind::PermissionDenied {
            PlanningErrorCode::PermissionDenied
        } else {
            PlanningErrorCode::DiscoveryFailed
        },
        message: e.message,
        platform_code: e.platform_code,
    })
}

fn source_issues(storage: &StorageInventory) -> Vec<PlanningIssue> {
    storage
        .warnings
        .iter()
        .chain(&storage.planning_warnings)
        .map(|warning| error(PlanningErrorCode::MissingMetadata, warning))
        .collect()
}

fn collision_issues(storage: &StorageInventory, target: &StorageDisk) -> Vec<PlanningIssue> {
    let disk_guid = target
        .guid
        .as_deref()
        .map(crate::partition_planning::canonical_guid);
    let partition_ids: Vec<_> = target
        .partitions
        .iter()
        .filter_map(|p| p.guid.as_deref())
        .map(crate::partition_planning::canonical_guid)
        .collect();
    let collisions = storage
        .disks
        .iter()
        .filter(|disk| {
            disk.number == target.number
                || disk_guid.as_ref().is_some_and(|id| {
                    disk.guid
                        .as_deref()
                        .map(crate::partition_planning::canonical_guid)
                        .as_ref()
                        == Some(id)
                })
        })
        .count()
        > 1
        || storage
            .disks
            .iter()
            .filter(|disk| disk.number != target.number)
            .any(|disk| {
                disk.partitions
                    .iter()
                    .filter_map(|p| p.guid.as_deref())
                    .map(crate::partition_planning::canonical_guid)
                    .any(|id| partition_ids.contains(&id))
            });
    if collisions {
        vec![error(
            PlanningErrorCode::AmbiguousIdentity,
            "Disk number, GPT disk GUID or partition GUID collides with another disk",
        )]
    } else {
        vec![]
    }
}

fn planning_disk(disk: &StorageDisk, native: Option<&NativeLayout>) -> PlanningDisk {
    let mut result = PlanningDisk {
        identifier: format!("PhysicalDrive{}", disk.number),
        name: disk.name.clone(),
        identity: persistent_identity(disk),
        device_path: nonempty(disk.path.clone()),
        partition_table: PartitionScheme::from_content(Some(&disk.partition_style)),
        safety: PartitionDiskSafety {
            external: native
                .map(|n| n.usb && n.removable && disk.bus_type.eq_ignore_ascii_case("USB")),
            system: disk.system,
            boot: disk.boot,
            read_only: disk.read_only,
            offline: disk.offline,
            clustered: disk.clustered,
            boot_from_disk: disk.planning.as_ref().and_then(|p| p.boot_from_disk),
            storage_pool: disk.planning.as_ref().and_then(|p| p.storage_pool),
            dynamic: native.map(|n| {
                n.partitions.iter().any(|p| {
                    matches!(
                        p.kind.as_str(),
                        "5808c8aa-7e8f-42e0-85d2-e1e90434cfb3"
                            | "af9b60a0-1431-4f62-bc68-3311714a69ad"
                    )
                })
            }),
        },
        ..Default::default()
    };
    let Some(native) = native else {
        return result;
    };
    result.geometry = Some(native.geometry.clone());
    let id_matches = result.identity.as_ref().is_some_and(|id| {
        native.serial.eq_ignore_ascii_case(&id.serial_number)
            && native.unique_ids.iter().any(|(format, value)| {
                *format == id.unique_id_format && value.eq_ignore_ascii_case(&id.unique_id)
            })
    });
    if !id_matches || native.number != disk.number {
        result.discovery_issues.push(error(
            PlanningErrorCode::StaleDeviceIdentity,
            "Native handle VPD identity, serial or disk number disagrees with Storage inventory",
        ));
    }
    if disk.size_bytes != native.geometry.disk_size_bytes
        || disk
            .guid
            .as_deref()
            .map(crate::partition_planning::canonical_guid)
            != Some(native.geometry.disk_guid.clone())
        || disk.planning.as_ref().and_then(|p| p.logical_sector_bytes)
            != Some(native.geometry.logical_sector_bytes)
        || disk.planning.as_ref().and_then(|p| p.physical_sector_bytes)
            != Some(native.geometry.physical_sector_bytes)
        || disk.planning.as_ref().and_then(|p| p.partition_count)
            != Some(native.partitions.len() as u32)
        || disk.partitions.len() != native.partitions.len()
    {
        result.discovery_issues.push(error(PlanningErrorCode::InvalidGeometry, "Storage inventory and native GPT geometry/count disagree or required metadata is absent"));
    }
    for p in &native.partitions {
        let matches: Vec<_> = disk
            .partitions
            .iter()
            .filter(|s| s.number == p.number)
            .collect();
        let mut partition = PlanningPartition {
            partition_id: Some(p.id.clone()),
            number: Some(p.number),
            offset_bytes: Some(p.offset),
            size_bytes: Some(p.size),
            gpt_type: Some(p.kind.clone()),
            attributes: Some(p.attributes),
            service_partition: Some(p.service),
            name: Some(p.name.clone()),
            ..Default::default()
        };
        if matches.len() != 1 {
            result.discovery_issues.push(error(
                PlanningErrorCode::InvalidGeometry,
                "Native and Storage partition associations are missing or ambiguous",
            ));
        } else {
            let s = matches[0];
            if s.offset_bytes != p.offset
                || s.size_bytes != p.size
                || s.guid
                    .as_deref()
                    .map(crate::partition_planning::canonical_guid)
                    != Some(p.id.clone())
                || s.gpt_type
                    .as_deref()
                    .map(crate::partition_planning::canonical_guid)
                    != Some(p.kind.clone())
            {
                result.discovery_issues.push(error(
                    PlanningErrorCode::InvalidGeometry,
                    "Native and Storage partition extents/GUIDs/types disagree",
                ));
            }
            partition.safety = PartitionSafety {
                flags: DeviceSafety {
                    system: s.system,
                    boot: s.boot,
                    recovery: Some(
                        crate::PartitionRole::windows(Some(&p.kind), None)
                            == crate::PartitionRole::Recovery,
                    ),
                    hidden: s.hidden,
                    read_only: s.read_only,
                    offline: s.offline,
                    page_file: s.page_file,
                },
                active: s.active,
                shadow_copy: s.shadow_copy,
                encrypted: s.planning.as_ref().and_then(|p| p.encrypted),
                hosts_windows: s.planning.as_ref().and_then(|p| p.hosts_windows),
                volumes_complete: s.planning.as_ref().and_then(|p| p.volumes_complete),
            };
            partition.volumes = s
                .volumes
                .iter()
                .map(|v| PartitionVolumeReference {
                    identifier: v.path.clone(),
                    mount_points: s
                        .access_paths
                        .iter()
                        .filter(|path| !crate::operations::valid_volume_path(path))
                        .cloned()
                        .collect(),
                })
                .collect();
            if partition
                .volumes
                .iter()
                .any(|v| !crate::operations::valid_volume_path(&v.identifier))
            {
                result.discovery_issues.push(error(
                    PlanningErrorCode::MissingMetadata,
                    "Associated volume lacks a valid GUID identity",
                ));
            }
        }
        result.partitions.push(partition);
    }
    result
}

fn preview_with(
    runner: &impl CommandRunner,
    api: &impl LayoutQuery,
    request: &PartitionPlanningRequest,
) -> PartitionPlanningResponse {
    let run = || -> Result<PartitionPlanningResponse, PlanningIssue> {
        let first = storage_with(runner)?;
        let expected = match request {
            PartitionPlanningRequest::Create {
                expected_identity, ..
            }
            | PartitionPlanningRequest::Delete {
                expected_identity, ..
            } => Some(expected_identity),
            _ => None,
        };
        let candidates: Vec<_> = first
            .disks
            .iter()
            .filter(|d| {
                expected.map_or_else(
                    || format!("PhysicalDrive{}", d.number) == request.identifier(),
                    |expected| persistent_identity(d).is_some_and(|id| id.matches(expected)),
                )
            })
            .collect();
        if candidates.len() != 1 {
            return Err(error(
                if candidates.len() > 1 {
                    PlanningErrorCode::AmbiguousIdentity
                } else if first
                    .disks
                    .iter()
                    .any(|d| format!("PhysicalDrive{}", d.number) == request.identifier())
                {
                    PlanningErrorCode::StaleDeviceIdentity
                } else {
                    PlanningErrorCode::DeviceDisappeared
                },
                "Physical target is missing, replaced or ambiguous",
            ));
        }
        let target = candidates[0];
        let mut inventory = PartitionPlanningInventory {
            disks: first.disks.iter().map(|d| planning_disk(d, None)).collect(),
            issues: source_issues(&first),
        };
        inventory.issues.extend(collision_issues(&first, target));
        let native = match api.query(target) {
            Ok(native) => native,
            Err(issue) => {
                let current = inventory
                    .disks
                    .iter_mut()
                    .find(|d| d.identifier == format!("PhysicalDrive{}", target.number))
                    .unwrap();
                current.discovery_issues.push(issue);
                return Ok(plan_partition_in(&inventory, request));
            }
        };
        let second = storage_with(runner)?;
        let current: Vec<_> = second
            .disks
            .iter()
            .filter(|d| {
                persistent_identity(d)
                    .zip(persistent_identity(target))
                    .is_some_and(|(a, b)| a.matches(&b))
            })
            .collect();
        if current.len() != 1 {
            return Err(error(
                if current.len() > 1 {
                    PlanningErrorCode::AmbiguousIdentity
                } else {
                    PlanningErrorCode::DeviceDisappeared
                },
                "Physical target changed during discovery",
            ));
        }
        let again = api.query(current[0])?;
        let before = planning_disk(target, Some(&native));
        let after = planning_disk(current[0], Some(&again));
        if native != again
            || serde_json::to_value(&before).ok() != serde_json::to_value(&after).ok()
        {
            return Err(error(
                PlanningErrorCode::ChangedPartitionLayout,
                "Native layout, identity or safety metadata changed during discovery",
            ));
        }
        inventory.disks = second
            .disks
            .iter()
            .map(|d| planning_disk(d, (d.number == current[0].number).then_some(&again)))
            .collect();
        inventory.issues.extend(source_issues(&second));
        inventory
            .issues
            .extend(collision_issues(&second, current[0]));
        Ok(plan_partition_in(&inventory, request))
    };
    run().unwrap_or_else(PartitionPlanningResponse::rejected)
}

#[cfg(target_os = "windows")]
pub(crate) fn preview(request: &PartitionPlanningRequest) -> PartitionPlanningResponse {
    preview_with(
        &super::super::command::SystemCommandRunner,
        &win32::NativeLayoutQuery,
        request,
    )
}

fn bytes<const N: usize>(data: &[u8], offset: usize) -> Result<[u8; N], PlanningIssue> {
    data.get(offset..offset.checked_add(N).ok_or_else(malformed)?)
        .and_then(|slice| slice.try_into().ok())
        .ok_or_else(malformed)
}
fn u32_at(data: &[u8], offset: usize) -> Result<u32, PlanningIssue> {
    Ok(u32::from_le_bytes(bytes(data, offset)?))
}
fn u64_at(data: &[u8], offset: usize) -> Result<u64, PlanningIssue> {
    Ok(u64::from_le_bytes(bytes(data, offset)?))
}
fn positive_at(data: &[u8], offset: usize) -> Result<u64, PlanningIssue> {
    let value = i64::from_le_bytes(bytes(data, offset)?);
    u64::try_from(value).map_err(|_| malformed())
}
fn malformed() -> PlanningIssue {
    error(
        PlanningErrorCode::InvalidGeometry,
        "Truncated, invalid or unsupported native storage metadata",
    )
}
fn guid_at(data: &[u8], offset: usize) -> Result<String, PlanningIssue> {
    let b: [u8; 16] = bytes(data, offset)?;
    Ok(format!(
        "{:08x}-{:04x}-{:04x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        u32::from_le_bytes(b[0..4].try_into().unwrap()),
        u16::from_le_bytes(b[4..6].try_into().unwrap()),
        u16::from_le_bytes(b[6..8].try_into().unwrap()),
        b[8],
        b[9],
        b[10],
        b[11],
        b[12],
        b[13],
        b[14],
        b[15]
    ))
}

fn parse_layout(
    data: &[u8],
    length: &[u8],
    alignment: &[u8],
) -> Result<(GptGeometry, Vec<NativePartition>), PlanningIssue> {
    if u32_at(data, 0)? != 1 {
        return Err(error(
            PlanningErrorCode::UnsupportedOperation,
            "Native layout is MBR, RAW or unknown; only GPT is supported",
        ));
    }
    let count = u32_at(data, 4)? as usize;
    if count > 4096
        || data.len() < 48 + count * 144
        || u32_at(alignment, 0)? < 28
        || u32_at(alignment, 4)? < 28
        || u32_at(alignment, 4)? as usize > alignment.len()
    {
        return Err(malformed());
    }
    let start = positive_at(data, 24)?;
    let g = GptGeometry {
        disk_guid: guid_at(data, 8)?,
        disk_size_bytes: positive_at(length, 0)?,
        usable_start_bytes: start,
        usable_end_bytes: start
            .checked_add(positive_at(data, 32)?)
            .ok_or_else(malformed)?,
        logical_sector_bytes: u32_at(alignment, 16)?,
        physical_sector_bytes: u32_at(alignment, 20)?,
        sector_alignment_offset_bytes: u32_at(alignment, 24)?,
        max_partition_count: u32_at(data, 40)?,
    };
    if count > g.max_partition_count as usize {
        return Err(malformed());
    }
    let mut partitions = Vec::new();
    for entry in data[48..48 + count * 144].as_chunks::<144>().0 {
        if u32_at(entry, 0)? != 1 {
            return Err(malformed());
        }
        let kind = guid_at(entry, 32)?;
        if kind == "00000000-0000-0000-0000-000000000000" {
            if entry[8..28].iter().chain(&entry[48..144]).any(|b| *b != 0) {
                return Err(malformed());
            }
            continue;
        }
        let units: Vec<_> = entry[72..144]
            .as_chunks::<2>()
            .0
            .iter()
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
            .take_while(|c| *c != 0)
            .collect();
        partitions.push(NativePartition {
            number: u32_at(entry, 24)?,
            id: guid_at(entry, 48)?,
            kind,
            offset: positive_at(entry, 8)?,
            size: positive_at(entry, 16)?,
            attributes: u64_at(entry, 64)?,
            service: entry[29] != 0,
            name: String::from_utf16(&units).map_err(|_| malformed())?,
        });
    }
    partitions.sort_by_key(|p| (p.offset, p.number));
    Ok((g, partitions))
}

fn descriptor(data: &[u8], minimum: usize) -> Result<&[u8], PlanningIssue> {
    let size = u32_at(data, 4)? as usize;
    if u32_at(data, 0)? < minimum as u32 || size < minimum {
        return Err(malformed());
    }
    data.get(..size).ok_or_else(malformed)
}

fn parse_device(data: &[u8]) -> Result<(String, bool, bool), PlanningIssue> {
    let data = descriptor(data, 36)?;
    if data[8] != 0 {
        return Err(error(
            PlanningErrorCode::UnsupportedOperation,
            "Only direct-access disk devices are supported",
        ));
    }
    let offset = u32_at(data, 24)? as usize;
    if offset < 36 {
        return Err(error(
            PlanningErrorCode::MissingMetadata,
            "Native storage serial is missing",
        ));
    }
    let serial = data.get(offset..).ok_or_else(malformed)?;
    let end = serial.iter().position(|b| *b == 0).ok_or_else(malformed)?;
    let serial = std::str::from_utf8(&serial[..end])
        .map_err(|_| malformed())?
        .trim()
        .to_owned();
    Ok((serial, u32_at(data, 28)? == 7, data[10] != 0))
}

fn parse_ids(data: &[u8]) -> Result<Vec<(u16, String)>, PlanningIssue> {
    let data = descriptor(data, 12)?;
    let count = u32_at(data, 8)?;
    if count > 1024 {
        return Err(malformed());
    }
    let mut position = 12usize;
    let mut ids = Vec::new();
    for index in 0..count {
        let entry = data.get(position..).ok_or_else(malformed)?;
        let code_set = u32_at(entry, 0)?;
        let kind = u32_at(entry, 4)?;
        let size = u16::from_le_bytes(bytes(entry, 8)?) as usize;
        let next = u16::from_le_bytes(bytes(entry, 10)?) as usize;
        let association = u32_at(entry, 12)?;
        let id = entry.get(16..16 + size).ok_or_else(malformed)?;
        if association == 0 {
            match (kind, code_set) {
                (2 | 3, 1) if !id.is_empty() => {
                    ids.push((kind as u16, id.iter().map(|b| format!("{b:02x}")).collect()))
                }
                (8, 2 | 3) if !id.is_empty() => ids.push((
                    8,
                    std::str::from_utf8(id)
                        .map_err(|_| malformed())?
                        .trim_end_matches('\0')
                        .trim()
                        .to_owned(),
                )),
                _ => {}
            }
        }
        if index + 1 < count {
            if next < 16 + size {
                return Err(malformed());
            }
            position = position.checked_add(next).ok_or_else(malformed)?;
        } else if next != 0 {
            return Err(malformed());
        }
    }
    ids.sort();
    ids.dedup();
    Ok(ids)
}

#[cfg(target_os = "windows")]
mod win32 {
    use super::*;
    use std::{ffi::c_void, ptr};
    type Handle = *mut c_void;
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn CreateFileW(
            name: *const u16,
            access: u32,
            share: u32,
            security: *const c_void,
            creation: u32,
            flags: u32,
            template: Handle,
        ) -> Handle;
        fn CloseHandle(handle: Handle) -> i32;
        fn DeviceIoControl(
            handle: Handle,
            code: u32,
            input: *const c_void,
            input_size: u32,
            output: *mut c_void,
            output_size: u32,
            returned: *mut u32,
            overlapped: *mut c_void,
        ) -> i32;
    }
    struct ReadOnlyDisk(Handle);
    impl Drop for ReadOnlyDisk {
        fn drop(&mut self) {
            unsafe {
                CloseHandle(self.0);
            }
        }
    }

    fn native_error(name: &str) -> PlanningIssue {
        let e = std::io::Error::last_os_error();
        PlanningIssue {
            code: match e.raw_os_error() {
                Some(5 | 1314) => PlanningErrorCode::PermissionDenied,
                Some(2 | 3 | 1167) => PlanningErrorCode::DeviceDisappeared,
                _ => PlanningErrorCode::DiscoveryFailed,
            },
            message: format!(
                "{name}: {e}. Read-only query failed; Diskvio does not request elevation."
            ),
            platform_code: e.raw_os_error().map(i64::from),
        }
    }

    impl ReadOnlyDisk {
        fn query(
            &self,
            code: u32,
            input: &[u8],
            initial: usize,
            maximum: usize,
        ) -> Result<Vec<u8>, PlanningIssue> {
            let mut capacity = initial;
            loop {
                let mut output = vec![0u8; capacity];
                let mut returned = 0;
                let success = unsafe {
                    DeviceIoControl(
                        self.0,
                        code,
                        if input.is_empty() {
                            ptr::null()
                        } else {
                            input.as_ptr().cast()
                        },
                        input.len() as u32,
                        output.as_mut_ptr().cast(),
                        output.len() as u32,
                        &mut returned,
                        ptr::null_mut(),
                    )
                };
                if success != 0 {
                    if returned as usize > output.len() {
                        return Err(malformed());
                    }
                    output.truncate(returned as usize);
                    return Ok(output);
                }
                if matches!(
                    std::io::Error::last_os_error().raw_os_error(),
                    Some(122 | 234)
                ) && capacity < maximum
                {
                    capacity = (capacity * 2).min(maximum);
                } else {
                    return Err(native_error("Native read-only layout query"));
                }
            }
        }
        fn property(&self, id: u32) -> Result<Vec<u8>, PlanningIssue> {
            let mut query = [0u8; 12];
            query[..4].copy_from_slice(&id.to_le_bytes());
            let header = self.query(0x002d_1400, &query, 8, 8)?;
            let size = u32_at(&header, 4)? as usize;
            if !(8..=65_536).contains(&size) {
                return Err(malformed());
            }
            self.query(0x002d_1400, &query, size, size)
        }
    }
    pub(super) struct NativeLayoutQuery;
    impl LayoutQuery for NativeLayoutQuery {
        fn query(&self, disk: &StorageDisk) -> Result<NativeLayout, PlanningIssue> {
            let path = disk
                .path
                .as_deref()
                .filter(|path| path.starts_with(r"\\?\") && !path.contains('\0'))
                .ok_or_else(|| {
                    error(
                        PlanningErrorCode::MissingMetadata,
                        "A current Windows disk-interface path is required",
                    )
                })?;
            let wide: Vec<_> = path.encode_utf16().chain(std::iter::once(0)).collect();
            let raw = unsafe {
                CreateFileW(
                    wide.as_ptr(),
                    0x8000_0000,
                    3,
                    ptr::null(),
                    3,
                    0,
                    ptr::null_mut(),
                )
            };
            if raw as isize == -1 {
                return Err(native_error("Open disk for read-only layout query"));
            }
            let handle = ReadOnlyDisk(raw);
            let device = handle.property(0)?;
            let (serial, usb, removable) = parse_device(&device)?;
            let unique_ids = parse_ids(&handle.property(2)?)?;
            let number = handle.query(0x002d_1080, &[], 12, 12)?;
            if u32_at(&number, 0)? != 7 || ![0, u32::MAX].contains(&u32_at(&number, 8)?) {
                return Err(malformed());
            }
            let length = handle.query(0x0007_405c, &[], 8, 8)?;
            let alignment = handle.property(6)?;
            let layout = handle.query(0x0007_0050, &[], 48 + 144 * 128, 48 + 144 * 4096)?;
            let (geometry, partitions) = parse_layout(&layout, &length, &alignment)?;
            Ok(NativeLayout {
                geometry,
                partitions,
                serial,
                unique_ids,
                number: u32_at(&number, 4)?,
                usb,
                removable,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::command::{CommandOutput, FixtureRunner};
    use std::cell::RefCell;

    fn fixture() -> StorageInventory {
        let mut value: serde_json::Value = serde_json::from_slice(include_bytes!(
            "../../../tests/fixtures/windows_inventory.json"
        ))
        .unwrap();
        let mut disk = value["disks"][1].clone();
        disk["unique_id_format"] = 8.into();
        disk["partitions"] = serde_json::json!([]);
        disk["planning"] = serde_json::json!({"boot_from_disk":false,"storage_pool":false,"logical_sector_bytes":512,"physical_sector_bytes":4096,"partition_count":0});
        value["disks"] = serde_json::json!([disk]);
        serde_json::from_value(value).unwrap()
    }
    fn native() -> NativeLayout {
        NativeLayout {
            geometry: GptGeometry {
                disk_guid: "dddddddd-1111-2222-3333-444444444444".into(),
                disk_size_bytes: 64_000_000_000,
                usable_start_bytes: 34 * 512,
                usable_end_bytes: 64_000_000_000 - 33 * 512,
                logical_sector_bytes: 512,
                physical_sector_bytes: 4096,
                sector_alignment_offset_bytes: 0,
                max_partition_count: 128,
            },
            partitions: vec![],
            serial: "usb-serial".into(),
            unique_ids: vec![(8, "usb-id".into())],
            number: 2,
            usb: true,
            removable: true,
        }
    }
    fn runner(storage: Vec<StorageInventory>) -> FixtureRunner {
        FixtureRunner::new(
            storage
                .into_iter()
                .map(|s| {
                    (
                        powershell_path().unwrap(),
                        vec![
                            "-NoProfile".into(),
                            "-NonInteractive".into(),
                            "-Command".into(),
                            PLANNING_SCRIPT.into(),
                        ],
                        Ok(CommandOutput {
                            success: true,
                            code: Some(0),
                            stdout: serde_json::to_vec(&s).unwrap(),
                            stderr: vec![],
                        }),
                    )
                })
                .collect(),
        )
    }
    struct MockQuery(RefCell<std::collections::VecDeque<Result<NativeLayout, PlanningIssue>>>);
    impl MockQuery {
        fn new(values: Vec<Result<NativeLayout, PlanningIssue>>) -> Self {
            Self(RefCell::new(values.into()))
        }
    }
    impl LayoutQuery for MockQuery {
        fn query(&self, _: &StorageDisk) -> Result<NativeLayout, PlanningIssue> {
            self.0
                .borrow_mut()
                .pop_front()
                .expect("Unexpected native query")
        }
    }
    fn request() -> PartitionPlanningRequest {
        PartitionPlanningRequest::Discover {
            identifier: "PhysicalDrive2".into(),
        }
    }
    fn rejected(response: &PartitionPlanningResponse, code: PlanningErrorCode) {
        assert!(!response.validation.valid);
        assert!(
            response.validation.issues.iter().any(|i| i.code == code),
            "{:?}",
            response.validation.issues
        );
    }

    #[test]
    fn queries_are_read_only_and_two_snapshots_must_agree() {
        let runner = runner(vec![fixture(), fixture()]);
        let api = MockQuery::new(vec![Ok(native()), Ok(native())]);
        let response = preview_with(&runner, &api, &request());
        assert!(
            response.validation.valid,
            "{:?}",
            response.validation.issues
        );
        assert!(!response.execution_supported);
        assert!(!response.discovery.unwrap().regions.is_empty());
        runner.assert_finished();
        assert!(api.0.borrow().is_empty());
    }
    #[test]
    fn native_permission_failure_retains_code_without_retry_or_elevation() {
        let mut denied = error(PlanningErrorCode::PermissionDenied, "Access denied");
        denied.platform_code = Some(5);
        let runner = runner(vec![fixture()]);
        let api = MockQuery::new(vec![Err(denied)]);
        let response = preview_with(&runner, &api, &request());
        rejected(&response, PlanningErrorCode::PermissionDenied);
        assert!(
            response
                .validation
                .issues
                .iter()
                .any(|i| i.platform_code == Some(5))
        );
        runner.assert_finished();
        assert!(api.0.borrow().is_empty());
    }
    #[test]
    fn disappearance_and_replacement_during_inventory_are_rejected() {
        for replaced in [false, true] {
            let mut changed = fixture();
            if replaced {
                changed.disks[0].serial_number = Some("replacement".into());
            } else {
                changed.disks.clear();
            }
            let runner = runner(vec![fixture(), changed]);
            let api = MockQuery::new(vec![Ok(native())]);
            rejected(
                &preview_with(&runner, &api, &request()),
                PlanningErrorCode::DeviceDisappeared,
            );
            runner.assert_finished();
        }
    }
    #[test]
    fn changed_native_layout_or_storage_safety_is_rejected() {
        let mut changed = native();
        changed.geometry.usable_end_bytes -= 512;
        rejected(
            &preview_with(
                &runner(vec![fixture(), fixture()]),
                &MockQuery::new(vec![Ok(native()), Ok(changed)]),
                &request(),
            ),
            PlanningErrorCode::ChangedPartitionLayout,
        );
        let mut changed = fixture();
        changed.disks[0].read_only = Some(true);
        rejected(
            &preview_with(
                &runner(vec![fixture(), changed]),
                &MockQuery::new(vec![Ok(native()), Ok(native())]),
                &request(),
            ),
            PlanningErrorCode::ChangedPartitionLayout,
        );
    }
    #[test]
    fn native_and_storage_identity_and_geometry_must_agree() {
        for field in ["serial", "id", "number", "size", "guid", "sector"] {
            let mut n = native();
            match field {
                "serial" => n.serial = "OTHER".into(),
                "id" => n.unique_ids.clear(),
                "number" => n.number = 9,
                "size" => n.geometry.disk_size_bytes += 512,
                "guid" => n.geometry.disk_guid = "aaaaaaaa-1111-2222-3333-444444444444".into(),
                _ => n.geometry.physical_sector_bytes = 512,
            }
            let response = preview_with(
                &runner(vec![fixture(), fixture()]),
                &MockQuery::new(vec![Ok(n.clone()), Ok(n)]),
                &request(),
            );
            rejected(
                &response,
                if ["serial", "id", "number"].contains(&field) {
                    PlanningErrorCode::StaleDeviceIdentity
                } else {
                    PlanningErrorCode::InvalidGeometry
                },
            );
            assert!(response.discovery.unwrap().regions.is_empty());
        }
    }
    #[test]
    fn usb_without_removable_evidence_and_partial_safety_are_not_authorized() {
        let mut n = native();
        n.removable = false;
        rejected(
            &preview_with(
                &runner(vec![fixture(), fixture()]),
                &MockQuery::new(vec![Ok(n.clone()), Ok(n)]),
                &request(),
            ),
            PlanningErrorCode::ProtectedTarget,
        );
        let mut s = fixture();
        s.disks[0].planning.as_mut().unwrap().storage_pool = None;
        rejected(
            &preview_with(
                &runner(vec![s.clone(), s]),
                &MockQuery::new(vec![Ok(native()), Ok(native())]),
                &request(),
            ),
            PlanningErrorCode::MissingMetadata,
        );
    }
    #[test]
    fn incomplete_other_identity_still_detects_serial_collisions() {
        let mut s = fixture();
        let mut duplicate = s.disks[0].clone();
        duplicate.number = 8;
        duplicate.unique_id = None;
        duplicate.unique_id_format = None;
        s.disks.push(duplicate);
        rejected(
            &preview_with(
                &runner(vec![s.clone(), s]),
                &MockQuery::new(vec![Ok(native()), Ok(native())]),
                &request(),
            ),
            PlanningErrorCode::AmbiguousIdentity,
        );
    }
    fn put32(data: &mut [u8], offset: usize, value: u32) {
        data[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }
    fn put64(data: &mut [u8], offset: usize, value: u64) {
        data[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
    }
    fn buffers(count: usize) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
        let mut layout = vec![0; 48 + count * 144];
        put32(&mut layout, 0, 1);
        put32(&mut layout, 4, count as u32);
        layout[8..24].fill(0x11);
        put64(&mut layout, 24, 34 * 512);
        put64(&mut layout, 32, 100 * PARTITION_ALIGNMENT_BYTES - 67 * 512);
        put32(&mut layout, 40, 128);
        for n in 0..count {
            let e = &mut layout[48 + n * 144..48 + (n + 1) * 144];
            put32(e, 0, 1);
            put64(e, 8, (n as u64 + 1) * 10 * PARTITION_ALIGNMENT_BYTES);
            put64(e, 16, PARTITION_ALIGNMENT_BYTES);
            put32(e, 24, n as u32 + 1);
            e[32..48].fill(0x22);
            e[48..64].fill(n as u8 + 1);
            put64(e, 64, 0);
        }
        let length = (100 * PARTITION_ALIGNMENT_BYTES).to_le_bytes().to_vec();
        let mut alignment = vec![0; 28];
        put32(&mut alignment, 0, 28);
        put32(&mut alignment, 4, 28);
        put32(&mut alignment, 16, 512);
        put32(&mut alignment, 20, 4096);
        (layout, length, alignment)
    }
    #[test]
    fn native_gpt_buffer_parsing_supports_empty_and_multiple_partitions() {
        for count in [0, 1, 4] {
            let (layout, length, alignment) = buffers(count);
            let (g, p) = parse_layout(&layout, &length, &alignment).unwrap();
            assert_eq!(g.usable_start_bytes, 34 * 512);
            assert_eq!(p.len(), count);
            if count > 0 {
                assert_eq!(p[0].number, 1);
                assert_eq!(p[0].offset, 10 * PARTITION_ALIGNMENT_BYTES);
            }
        }
    }
    #[test]
    fn every_truncated_native_buffer_and_negative_extent_is_rejected() {
        let (layout, length, alignment) = buffers(2);
        for len in 0..layout.len() {
            assert!(
                parse_layout(&layout[..len], &length, &alignment).is_err(),
                "{len}"
            );
        }
        for len in 0..8 {
            assert!(parse_layout(&layout, &length[..len], &alignment).is_err());
        }
        for len in 0..28 {
            assert!(parse_layout(&layout, &length, &alignment[..len]).is_err());
        }
        for offset in [24, 32, 48 + 8, 48 + 16] {
            let mut invalid = layout.clone();
            put64(&mut invalid, offset, u64::MAX);
            assert!(parse_layout(&invalid, &length, &alignment).is_err());
        }
        let mut invalid = layout.clone();
        put32(&mut invalid, 4, u32::MAX);
        assert!(parse_layout(&invalid, &length, &alignment).is_err());
        for style in [0, 2, u32::MAX] {
            let mut invalid = layout.clone();
            put32(&mut invalid, 0, style);
            assert_eq!(
                parse_layout(&invalid, &length, &alignment)
                    .unwrap_err()
                    .code,
                PlanningErrorCode::UnsupportedOperation
            );
        }
    }
    #[test]
    fn native_unused_entries_require_empty_extents_and_guid() {
        let (mut layout, length, alignment) = buffers(1);
        layout[48 + 8..48 + 28].fill(0);
        layout[48 + 32..].fill(0);
        assert!(
            parse_layout(&layout, &length, &alignment)
                .unwrap()
                .1
                .is_empty()
        );
        layout[48 + 8] = 1;
        assert!(parse_layout(&layout, &length, &alignment).is_err());
    }
    #[test]
    fn native_device_descriptor_requires_bounded_serial_and_usb_evidence() {
        let mut device = vec![0; 48];
        put32(&mut device, 0, 40);
        put32(&mut device, 4, 48);
        put32(&mut device, 24, 40);
        put32(&mut device, 28, 7);
        device[10] = 1;
        device[40..47].copy_from_slice(b"TEST001");
        assert_eq!(
            parse_device(&device).unwrap(),
            ("TEST001".into(), true, true)
        );
        for len in 0..48 {
            assert!(parse_device(&device[..len]).is_err());
        }
        put32(&mut device, 24, 0);
        assert_eq!(
            parse_device(&device).unwrap_err().code,
            PlanningErrorCode::MissingMetadata
        );
        put32(&mut device, 24, u32::MAX);
        assert!(parse_device(&device).is_err());
    }
    #[test]
    fn native_vpd_identifiers_validate_chain_offsets_and_association() {
        let mut data = vec![0; 36];
        put32(&mut data, 0, 16);
        put32(&mut data, 4, 36);
        put32(&mut data, 8, 1);
        put32(&mut data, 12, 1);
        put32(&mut data, 16, 3);
        data[20..22].copy_from_slice(&8u16.to_le_bytes());
        data[28..36].copy_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8]);
        assert_eq!(
            parse_ids(&data).unwrap(),
            vec![(3, "0102030405060708".into())]
        );
        for len in 0..36 {
            assert!(parse_ids(&data[..len]).is_err());
        }
        put32(&mut data, 24, 1);
        assert!(parse_ids(&data).unwrap().is_empty());
        put32(&mut data, 8, 2);
        assert!(parse_ids(&data).is_err());
    }

    fn populated_fixture() -> (StorageInventory, NativeLayout) {
        let source: StorageInventory = serde_json::from_slice(include_bytes!(
            "../../../tests/fixtures/windows_inventory.json"
        ))
        .unwrap();
        let mut storage = fixture();
        let mut p = source.disks[1].partitions[0].clone();
        p.size_bytes = 4 * PARTITION_ALIGNMENT_BYTES;
        p.planning = Some(super::super::StoragePlanningPartition {
            encrypted: Some(false),
            hosts_windows: Some(false),
            volumes_complete: Some(true),
        });
        storage.disks[0].partitions = vec![p];
        storage.disks[0].planning.as_mut().unwrap().partition_count = Some(1);
        let mut n = native();
        n.partitions = vec![NativePartition {
            number: 1,
            id: "eeeeeeee-1111-2222-3333-444444444444".into(),
            kind: BASIC_DATA_GPT_TYPE.into(),
            offset: PARTITION_ALIGNMENT_BYTES,
            size: 4 * PARTITION_ALIGNMENT_BYTES,
            attributes: 0,
            service: false,
            name: "".into(),
        }];
        (storage, n)
    }

    #[test]
    fn native_partition_associations_support_readonly_deletion_previews() {
        let (s, n) = populated_fixture();
        let response = preview_with(
            &runner(vec![s.clone(), s.clone()]),
            &MockQuery::new(vec![Ok(n.clone()), Ok(n.clone())]),
            &request(),
        );
        assert!(
            response.validation.valid,
            "{:?}",
            response.validation.issues
        );
        let d = response.discovery.unwrap();
        let p = &d.partitions[0].partition;
        assert_eq!(p.volumes[0].mount_points, ["E:\\"]);
        let request = PartitionPlanningRequest::Delete {
            identifier: d.identifier,
            expected_identity: d.physical_identity.clone().unwrap(),
            expected_layout: d.layout_snapshot.unwrap(),
            test_disk: TestDiskDesignation {
                identity: d.physical_identity.unwrap(),
                purpose: TestDiskPurpose::ExpendableExternalTestDisk,
            },
            partition: PartitionTarget {
                partition_id: p.partition_id.clone().unwrap(),
                number: 1,
                offset_bytes: PARTITION_ALIGNMENT_BYTES,
                size_bytes: 4 * PARTITION_ALIGNMENT_BYTES,
                gpt_type: BASIC_DATA_GPT_TYPE.into(),
            },
        };
        let response = preview_with(
            &runner(vec![s.clone(), s]),
            &MockQuery::new(vec![Ok(n.clone()), Ok(n)]),
            &request,
        );
        assert!(response.validation.valid);
        assert!(response.delete_preview.unwrap().eligible);
    }

    #[test]
    fn unresolved_encryption_windows_installations_and_partial_volumes_block_planning() {
        for field in ["encrypted", "hosts_windows", "volumes_complete"] {
            let (s, n) = populated_fixture();
            let mut json = serde_json::to_value(s).unwrap();
            json["disks"][0]["partitions"][0]["planning"][field] = serde_json::Value::Null;
            let s: StorageInventory = serde_json::from_value(json).unwrap();
            rejected(
                &preview_with(
                    &runner(vec![s.clone(), s]),
                    &MockQuery::new(vec![Ok(n.clone()), Ok(n)]),
                    &request(),
                ),
                PlanningErrorCode::MissingMetadata,
            );
        }
        let (mut s, n) = populated_fixture();
        s.disks[0].partitions[0]
            .planning
            .as_mut()
            .unwrap()
            .hosts_windows = Some(true);
        rejected(
            &preview_with(
                &runner(vec![s.clone(), s]),
                &MockQuery::new(vec![Ok(n.clone()), Ok(n)]),
                &request(),
            ),
            PlanningErrorCode::ProtectedTarget,
        );
    }

    #[test]
    fn native_partition_mismatches_and_cloned_guids_are_rejected() {
        let (mut s, n) = populated_fixture();
        s.disks[0].partitions[0].offset_bytes += 512;
        rejected(
            &preview_with(
                &runner(vec![s.clone(), s]),
                &MockQuery::new(vec![Ok(n.clone()), Ok(n)]),
                &request(),
            ),
            PlanningErrorCode::InvalidGeometry,
        );
        let (mut s, n) = populated_fixture();
        let mut clone = s.disks[0].clone();
        clone.number = 7;
        clone.unique_id = Some("other-id".into());
        clone.serial_number = Some("other-serial".into());
        s.disks.push(clone);
        rejected(
            &preview_with(
                &runner(vec![s.clone(), s]),
                &MockQuery::new(vec![Ok(n.clone()), Ok(n)]),
                &request(),
            ),
            PlanningErrorCode::AmbiguousIdentity,
        );
    }
}
