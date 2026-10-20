use crate::{PlatformError, partition_type, text};
use diskvio_core::{DeviceKind, Disk, Partition, Volume};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct NativeDisk {
    number: u32,
    path: Option<String>,
    unique_id: Option<String>,
    friendly_name: Option<String>,
    manufacturer: Option<String>,
    model: Option<String>,
    serial_number: Option<String>,
    size: u64,
    bus_type: Option<u16>,
    partition_style: Option<u16>,
    number_of_partitions: Option<u32>,
    is_offline: Option<bool>,
    is_read_only: Option<bool>,
    is_system: Option<bool>,
    is_boot: Option<bool>,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct NativePartition {
    disk_number: u32,
    partition_number: u32,
    offset: u64,
    size: u64,
    guid: Option<String>,
    gpt_type: Option<String>,
    mbr_type: Option<u16>,
    access_paths: Option<Vec<String>>,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct NativeVolume {
    path: Option<String>,
    file_system: Option<String>,
    file_system_label: Option<String>,
    size: Option<u64>,
    size_remaining: Option<u64>,
}
fn path_key(s: &str) -> String {
    s.trim_end_matches('\\').to_ascii_lowercase()
}

pub(crate) fn map(
    disks: Vec<NativeDisk>,
    partitions: Result<Vec<NativePartition>, PlatformError>,
    volumes: Result<Vec<NativeVolume>, PlatformError>,
) -> Result<Vec<Disk>, PlatformError> {
    let mut result = Vec::new();
    for d in disks {
        // Windows explicitly identifies virtual, file-backed and Storage Spaces devices.
        if matches!(d.bus_type, Some(14..=16)) {
            continue;
        }
        let device_path =
            text(d.path).unwrap_or_else(|| format!("\\\\.\\PhysicalDrive{}", d.number));
        let serial = text(d.serial_number);
        let model = text(d.model);
        let id = format!(
            "windows:{}",
            text(d.unique_id).unwrap_or_else(|| format!(
                "{}|{}|{}|{}",
                device_path,
                serial.as_deref().unwrap_or(""),
                model.as_deref().unwrap_or(""),
                d.size
            ))
        );
        let mut limitations = Vec::new();
        let layout = match &partitions {
            Err(_) => {
                limitations.push(
                    "Partition information is unavailable (provider or permission limitation)."
                        .into(),
                );
                None
            }
            Ok(ps) => {
                let mut found: Vec<_> = ps.iter().filter(|p| p.disk_number == d.number).collect();
                found.sort_by_key(|p| p.offset);
                if d.number_of_partitions
                    .is_none_or(|count| count as usize != found.len())
                {
                    limitations
                        .push("The partition layout is incomplete; refresh to retry.".into());
                    None
                } else {
                    Some(
                        found
                            .into_iter()
                            .map(|p| {
                                let paths = p.access_paths.clone().unwrap_or_default();
                                let mut vs: Vec<Volume> = volumes
                                    .as_ref()
                                    .ok()
                                    .into_iter()
                                    .flatten()
                                    .filter(|v| {
                                        v.path.as_ref().is_some_and(|vp| {
                                            paths.iter().any(|p| path_key(p) == path_key(vp))
                                        })
                                    })
                                    .map(|v| Volume {
                                        id: v.path.clone().unwrap_or_default(),
                                        label: text(v.file_system_label.clone()),
                                        filesystem: text(v.file_system.clone()),
                                        mount_points: paths
                                            .iter()
                                            .filter(|s| !s.starts_with("\\\\?\\"))
                                            .cloned()
                                            .collect(),
                                        capacity_bytes: v.size.filter(|s| *s > 0),
                                        used_bytes: v
                                            .size
                                            .zip(v.size_remaining)
                                            .and_then(|(s, f)| s.checked_sub(f))
                                            .filter(|_| v.size != Some(0)),
                                        free_bytes: v
                                            .size_remaining
                                            .filter(|_| v.size.is_some_and(|s| s > 0)),
                                        shared_space: false,
                                    })
                                    .collect();
                                if vs.is_empty() && !paths.is_empty() {
                                    vs.push(Volume {
                                        id: paths[0].clone(),
                                        label: None,
                                        filesystem: None,
                                        mount_points: paths
                                            .iter()
                                            .filter(|s| !s.starts_with("\\\\?\\"))
                                            .cloned()
                                            .collect(),
                                        capacity_bytes: None,
                                        used_bytes: None,
                                        free_bytes: None,
                                        shared_space: false,
                                    });
                                }
                                Partition {
                                    id: text(p.guid.clone()).unwrap_or_else(|| {
                                        format!("{id}:{}:{}", p.partition_number, p.offset)
                                    }),
                                    number: Some(p.partition_number),
                                    name: None,
                                    partition_type: partition_type(p.gpt_type.as_deref())
                                        .or_else(|| p.mbr_type.map(|t| format!("MBR 0x{t:x}"))),
                                    offset_bytes: p.offset,
                                    capacity_bytes: p.size,
                                    volumes: vs,
                                }
                            })
                            .collect(),
                    )
                }
            }
        };
        if volumes.is_err() {
            limitations.push("Volume metadata and filesystem usage are unavailable.".into());
        }
        result.push(Disk {
            id,
            device_path,
            name: text(d.friendly_name)
                .or_else(|| model.clone())
                .unwrap_or_else(|| format!("Disk {}", d.number)),
            manufacturer: text(d.manufacturer),
            model,
            serial,
            kind: match d.bus_type {
                Some(7 | 4) => DeviceKind::External,
                Some(3 | 11 | 17) => DeviceKind::Internal,
                _ => DeviceKind::Unknown,
            },
            removable: None,
            connection: d.bus_type.and_then(|b| {
                match b {
                    1 => Some("SCSI"),
                    2 => Some("ATAPI"),
                    3 => Some("ATA"),
                    4 => Some("IEEE 1394"),
                    6 => Some("Fibre Channel"),
                    7 => Some("USB"),
                    8 => Some("RAID"),
                    9 => Some("iSCSI"),
                    10 => Some("SAS"),
                    11 => Some("SATA"),
                    12 => Some("SD"),
                    13 => Some("MMC"),
                    17 => Some("NVMe"),
                    18 => Some("SCM"),
                    19 => Some("UFS"),
                    _ => None,
                }
                .map(str::to_owned)
            }),
            partition_scheme: match d.partition_style {
                Some(1) => Some("MBR".into()),
                Some(2) => Some("GPT".into()),
                Some(0) => Some("RAW".into()),
                _ => None,
            },
            online: d.is_offline.map(|v| !v),
            read_only: d.is_read_only,
            system: match (d.is_system, d.is_boot) {
                (Some(a), Some(b)) => Some(a || b),
                (Some(true), _) | (_, Some(true)) => Some(true),
                _ => None,
            },
            capacity_bytes: d.size,
            partitions: layout,
            limitations,
        });
    }
    Ok(result)
}

#[cfg(target_os = "windows")]
pub fn discover() -> Result<Vec<Disk>, PlatformError> {
    use crate::ErrorCode;
    use wmi::WMIConnection;
    fn error(e: wmi::WMIError) -> PlatformError {
        let detail = e.to_string();
        let denied = detail.contains("80041003")
            || detail.contains("80070005")
            || detail.to_ascii_lowercase().contains("access denied");
        PlatformError::new(
            if denied {
                ErrorCode::PermissionDenied
            } else {
                ErrorCode::DiscoveryFailed
            },
            format!("Windows storage discovery failed: {detail}"),
        )
    }
    let modern = || {
        let con = WMIConnection::with_namespace_path("ROOT\\Microsoft\\Windows\\Storage")
            .map_err(error)?;
        let disks = con.raw_query("SELECT Number,Path,UniqueId,FriendlyName,Manufacturer,Model,SerialNumber,Size,BusType,PartitionStyle,NumberOfPartitions,IsOffline,IsReadOnly,IsSystem,IsBoot FROM MSFT_Disk").map_err(error)?;
        let partitions = con.raw_query("SELECT DiskNumber,PartitionNumber,Offset,Size,Guid,GptType,MbrType,AccessPaths FROM MSFT_Partition").map_err(error);
        let volumes = con
            .raw_query("SELECT Path,FileSystem,FileSystemLabel,Size,SizeRemaining FROM MSFT_Volume")
            .map_err(error);
        map(disks, partitions, volumes)
    };
    match modern() {
        Ok(disks) => Ok(disks),
        Err(_) => {
            let con = WMIConnection::new().map_err(error)?;
            let ds=con.raw_query("SELECT Index,DeviceID,PNPDeviceID,Model,Manufacturer,SerialNumber,Size,InterfaceType,MediaType,Partitions,Signature FROM Win32_DiskDrive").map_err(error)?;
            let ps=con.raw_query("SELECT DiskIndex,Index,DeviceID,StartingOffset,Size,Type,BootPartition FROM Win32_DiskPartition").map_err(error);
            let vs = con
                .raw_query(
                    "SELECT DeviceID,VolumeName,FileSystem,Size,FreeSpace FROM Win32_LogicalDisk",
                )
                .map_err(error);
            let links = con
                .raw_query("SELECT Antecedent,Dependent FROM Win32_LogicalDiskToPartition")
                .map_err(error);
            map_legacy(
                ds,
                ps,
                vs,
                links,
                std::env::var("SystemDrive").ok().as_deref(),
            )
        }
    }
}

#[derive(Deserialize)]
pub(crate) struct LegacyDisk {
    #[serde(rename = "Index")]
    index: u32,
    #[serde(rename = "DeviceID")]
    device_id: String,
    #[serde(rename = "PNPDeviceID")]
    pnp_device_id: Option<String>,
    #[serde(rename = "Model")]
    model: Option<String>,
    #[serde(rename = "Manufacturer")]
    manufacturer: Option<String>,
    #[serde(rename = "SerialNumber")]
    serial: Option<String>,
    #[serde(rename = "Size")]
    size: Option<u64>,
    #[serde(rename = "InterfaceType")]
    interface: Option<String>,
    #[serde(rename = "MediaType")]
    media_type: Option<String>,
    #[serde(rename = "Partitions")]
    partitions: Option<u32>,
    #[serde(rename = "Signature")]
    signature: Option<u32>,
}
#[derive(Deserialize)]
pub(crate) struct LegacyPartition {
    #[serde(rename = "DiskIndex")]
    disk_index: u32,
    #[serde(rename = "Index")]
    index: u32,
    #[serde(rename = "DeviceID")]
    device_id: String,
    #[serde(rename = "StartingOffset")]
    offset: u64,
    #[serde(rename = "Size")]
    size: u64,
    #[serde(rename = "Type")]
    kind: Option<String>,
    #[serde(rename = "BootPartition")]
    boot: Option<bool>,
}
#[derive(Deserialize)]
pub(crate) struct LegacyVolume {
    #[serde(rename = "DeviceID")]
    device_id: String,
    #[serde(rename = "VolumeName")]
    label: Option<String>,
    #[serde(rename = "FileSystem")]
    filesystem: Option<String>,
    #[serde(rename = "Size")]
    size: Option<u64>,
    #[serde(rename = "FreeSpace")]
    free: Option<u64>,
}
#[derive(Deserialize)]
pub(crate) struct LegacyLink {
    #[serde(rename = "Antecedent")]
    antecedent: String,
    #[serde(rename = "Dependent")]
    dependent: String,
}
fn reference_key(s: &str) -> Option<String> {
    let (_, value) = s.rsplit_once("DeviceID=\"")?;
    Some(
        value
            .strip_suffix('"')?
            .replace("\\\\", "\\")
            .replace("\\\"", "\""),
    )
}
pub(crate) fn map_legacy(
    disks: Vec<LegacyDisk>,
    parts: Result<Vec<LegacyPartition>, PlatformError>,
    volumes: Result<Vec<LegacyVolume>, PlatformError>,
    links: Result<Vec<LegacyLink>, PlatformError>,
    system_drive: Option<&str>,
) -> Result<Vec<Disk>, PlatformError> {
    let mut result = Vec::new();
    for d in disks {
        // Empty card readers have no media/capacity, so are not physical storage inventories.
        let Some(capacity_bytes) = d.size.filter(|s| *s > 0) else {
            continue;
        };
        let pnp = text(d.pnp_device_id);
        let model = text(d.model);
        let serial = text(d.serial);
        if pnp
            .as_ref()
            .is_some_and(|s| s.to_uppercase().starts_with("ROOT\\"))
        {
            continue;
        }
        let id = format!(
            "windows:{}",
            pnp.clone().unwrap_or_else(|| format!(
                "{}|{}|{}|{capacity_bytes}",
                d.device_id,
                model.as_deref().unwrap_or(""),
                serial.as_deref().unwrap_or("")
            ))
        );
        let usb = d.interface.as_deref() == Some("USB")
            || pnp
                .as_ref()
                .is_some_and(|p| p.to_uppercase().starts_with("USBSTOR\\"));
        let removable = d
            .media_type
            .as_ref()
            .map(|s| s.to_lowercase().contains("removable"));
        let mut limitations=vec!["Using the unprivileged Windows compatibility provider. Online, read-only and some storage-bus metadata are unavailable.".into()];
        let selected: Vec<_> = parts
            .as_ref()
            .ok()
            .into_iter()
            .flatten()
            .filter(|p| p.disk_index == d.index)
            .collect();
        let scheme = if selected
            .iter()
            .any(|p| p.kind.as_ref().is_some_and(|k| k.starts_with("GPT:")))
        {
            Some("GPT".into())
        } else if d.signature.is_some_and(|s| s != 0) {
            Some("MBR".into())
        } else {
            None
        };
        let complete = parts.is_ok()
            && d.partitions.is_some_and(|n| n as usize == selected.len())
            && !selected.iter().any(|p| {
                p.kind
                    .as_ref()
                    .is_some_and(|k| k.to_lowercase().contains("extended"))
            });
        let mut system = if volumes.is_ok() && links.is_ok() && system_drive.is_some() && complete {
            Some(false)
        } else {
            None
        };
        let mut ps = Vec::new();
        for p in selected {
            let vs: Vec<_> = volumes
                .as_ref()
                .ok()
                .into_iter()
                .flatten()
                .filter(|v| {
                    links.as_ref().is_ok_and(|ls| {
                        ls.iter().any(|l| {
                            reference_key(&l.antecedent).as_deref() == Some(&p.device_id)
                                && reference_key(&l.dependent).as_deref() == Some(&v.device_id)
                        })
                    })
                })
                .map(|v| {
                    if system_drive.is_some_and(|s| s.eq_ignore_ascii_case(&v.device_id)) {
                        system = Some(true);
                    }
                    Volume {
                        id: v.device_id.clone(),
                        label: text(v.label.clone()),
                        filesystem: text(v.filesystem.clone()),
                        mount_points: vec![format!("{}\\", v.device_id)],
                        capacity_bytes: v.size.filter(|s| *s > 0),
                        used_bytes: v
                            .size
                            .zip(v.free)
                            .and_then(|(s, f)| s.checked_sub(f))
                            .filter(|_| v.size != Some(0)),
                        free_bytes: v.free.filter(|_| v.size.is_some_and(|s| s > 0)),
                        shared_space: false,
                    }
                })
                .collect();
            if p.boot == Some(true) {
                system = Some(true);
            }
            ps.push(Partition {
                id: format!("{id}:{}:{}", p.index, p.offset),
                number: Some(p.index + 1),
                name: None,
                partition_type: text(p.kind.clone()).map(|k| {
                    if k == "GPT: System" {
                        "EFI system".into()
                    } else {
                        k
                    }
                }),
                offset_bytes: p.offset,
                capacity_bytes: p.size,
                volumes: vs,
            });
        }
        ps.sort_by_key(|p| p.offset_bytes);
        if !complete {
            limitations.push("Exact partition information is unavailable or uses an unsupported extended layout.".into());
        }
        if volumes.is_err() || links.is_err() {
            limitations.push("Filesystem labels, mounts and usage are unavailable.".into());
        }
        result.push(Disk {
            id,
            device_path: d.device_id,
            name: model.clone().unwrap_or_else(|| format!("Disk {}", d.index)),
            manufacturer: text(d.manufacturer),
            model,
            serial,
            kind: if usb {
                DeviceKind::External
            } else if removable == Some(true) {
                DeviceKind::Removable
            } else {
                DeviceKind::Unknown
            },
            removable,
            connection: if usb {
                Some("USB".into())
            } else {
                text(d.interface)
            },
            partition_scheme: scheme,
            online: None,
            read_only: None,
            system,
            capacity_bytes,
            partitions: complete.then_some(ps),
            limitations,
        });
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn compatibility_provider_maps_associations_and_system_protection() {
        let ds=serde_json::from_str(r#"[{"Index":0,"DeviceID":"physical0","PNPDeviceID":"USBSTOR\\real-device","Model":"USB","Size":10000,"InterfaceType":"USB","MediaType":"Removable Media","Partitions":1}]"#).unwrap();
        let ps=serde_json::from_str(r#"[{"DiskIndex":0,"Index":0,"DeviceID":"Disk #0, Partition #0","StartingOffset":512,"Size":9000,"Type":"GPT: Basic Data","BootPartition":false}]"#).unwrap();
        let vs=serde_json::from_str(r#"[{"DeviceID":"C:","VolumeName":"System","FileSystem":"NTFS","Size":8000,"FreeSpace":1000}]"#).unwrap();
        let ls=serde_json::from_str(r#"[{"Antecedent":"Win32_DiskPartition.DeviceID=\"Disk #0, Partition #0\"","Dependent":"Win32_LogicalDisk.DeviceID=\"C:\""}]"#).unwrap();
        let disks = map_legacy(ds, Ok(ps), Ok(vs), Ok(ls), Some("C:")).unwrap();
        assert_eq!(disks[0].system, Some(true));
        assert_eq!(disks[0].connection.as_deref(), Some("USB"));
        assert_eq!(disks[0].partition_scheme.as_deref(), Some("GPT"));
        assert_eq!(
            disks[0].partitions.as_ref().unwrap()[0].volumes[0].used_bytes,
            Some(7000)
        );
        diskvio_core::validate_inventory(&disks).unwrap();
    }
    fn disks() -> Vec<NativeDisk> {
        serde_json::from_str(r#"[{"Number":0,"Path":"physical0","UniqueId":"stable","FriendlyName":"NVMe","Size":100000,"BusType":17,"PartitionStyle":2,"NumberOfPartitions":1,"IsSystem":true,"IsBoot":true}]"#).unwrap()
    }
    #[test]
    fn maps_exact_geometry_and_metadata() {
        let ps = serde_json::from_str(r#"[{"DiskNumber":0,"PartitionNumber":1,"Offset":512,"Size":99000,"Guid":"guid","GptType":"c12a7328-f81f-11d2-ba4b-00a0c93ec93b","AccessPaths":["C:\\","volume"]}]"#).unwrap();
        let vs = serde_json::from_str(r#"[{"Path":"volume","FileSystem":"NTFS","FileSystemLabel":"Real label","Size":98000,"SizeRemaining":8000}]"#).unwrap();
        let ds = map(disks(), Ok(ps), Ok(vs)).unwrap();
        diskvio_core::validate_inventory(&ds).unwrap();
        let p = &ds[0].partitions.as_ref().unwrap()[0];
        assert_eq!(p.offset_bytes, 512);
        assert_eq!(p.volumes[0].used_bytes, Some(90000));
        assert_eq!(p.partition_type.as_deref(), Some("EFI system"));
        assert_eq!(ds[0].serial, None);
    }
    #[test]
    fn inaccessible_layout_is_not_unallocated() {
        let ds = map(
            disks(),
            Err(PlatformError::new(
                crate::ErrorCode::PermissionDenied,
                "denied",
            )),
            Ok(vec![]),
        )
        .unwrap();
        assert!(ds[0].partitions.is_none());
        assert!(!ds[0].limitations.is_empty());
        assert!(
            map(disks(), Ok(vec![]), Ok(vec![])).unwrap()[0]
                .partitions
                .is_none()
        );
    }
    #[test]
    fn stable_ids_and_virtual_filter() {
        let a = map(disks(), Ok(vec![]), Ok(vec![])).unwrap();
        let b = map(disks(), Ok(vec![]), Ok(vec![])).unwrap();
        assert_eq!(a[0].id, b[0].id);
        let mut ds = disks();
        ds[0].bus_type = Some(15);
        assert!(map(ds, Ok(vec![]), Ok(vec![])).unwrap().is_empty());
        assert!(map(vec![], Ok(vec![]), Ok(vec![])).unwrap().is_empty());
    }
}
