use crate::{PlatformError, text};
use diskvio_core::{DeviceKind, Disk, Partition, Volume};
use plist::Value;

fn get<'a>(v: &'a Value, k: &str) -> Option<&'a Value> {
    v.as_dictionary()?.get(k)
}
fn string(v: &Value, k: &str) -> Option<String> {
    text(get(v, k)?.as_string().map(str::to_owned))
}
fn int(v: &Value, k: &str) -> Option<u64> {
    get(v, k)?.as_unsigned_integer()
}
fn flag(v: &Value, k: &str) -> Option<bool> {
    get(v, k)?.as_boolean()
}
fn array<'a>(v: &'a Value, k: &str) -> &'a [Value] {
    get(v, k)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}
fn device_id(s: &str) -> bool {
    let Some(rest) = s.strip_prefix("disk") else {
        return false;
    };
    let mut parts = rest.split('s');
    parts.all(|s| !s.is_empty() && s.bytes().all(|c| c.is_ascii_digit()))
}
fn volume(v: &Value, shared: bool) -> Option<Volume> {
    let id = string(v, "VolumeUUID")
        .or_else(|| string(v, "APFSVolumeUUID"))
        .or_else(|| string(v, "DeviceIdentifier"))?;
    let capacity = int(v, "TotalSize");
    let free = int(v, "VolumeFreeSpace");
    Some(Volume {
        id,
        label: string(v, "VolumeName").or_else(|| string(v, "Name")),
        filesystem: string(v, "FilesystemType").or_else(|| shared.then(|| "APFS".into())),
        mount_points: string(v, "MountPoint").into_iter().collect(),
        capacity_bytes: capacity,
        used_bytes: if shared {
            int(v, "CapacityInUse")
        } else {
            capacity.zip(free).and_then(|(s, f)| s.checked_sub(f))
        },
        free_bytes: if shared { None } else { free },
        shared_space: shared,
    })
}

pub(crate) fn map(
    list: &Value,
    apfs: Option<&Value>,
    mut info: impl FnMut(&str) -> Result<Value, PlatformError>,
) -> Result<Vec<Disk>, PlatformError> {
    let entries = get(list, "AllDisksAndPartitions")
        .and_then(Value::as_array)
        .ok_or_else(|| PlatformError::invalid("diskutil returned no physical disk array."))?;
    let mut disks = Vec::new();
    for d in entries {
        let name = string(d, "DeviceIdentifier")
            .filter(|s| device_id(s))
            .ok_or_else(|| PlatformError::invalid("Invalid physical BSD disk identifier."))?;
        let di = info(&name)?;
        if string(&di, "VirtualOrPhysical").as_deref() == Some("Virtual") {
            continue;
        }
        let capacity = int(&di, "TotalSize")
            .or_else(|| int(d, "Size"))
            .ok_or_else(|| PlatformError::invalid("Missing physical disk capacity."))?;
        let serial = string(&di, "SerialNumber");
        let model = string(&di, "MediaName").or_else(|| string(&di, "DeviceModel"));
        let id = format!(
            "macos:{}",
            string(&di, "DiskUUID")
                .or_else(|| serial
                    .as_ref()
                    .map(|s| format!("{}:{s}", model.as_deref().unwrap_or(""))))
                .unwrap_or_else(|| format!("{name}|{}|{capacity}", model.as_deref().unwrap_or("")))
        );
        let mut ps = Vec::new();
        let mut complete = true;
        let mut limitations = Vec::new();
        for p in array(d, "Partitions") {
            let pid = string(p, "DeviceIdentifier")
                .filter(|s| device_id(s))
                .ok_or_else(|| PlatformError::invalid("Invalid partition BSD identifier."))?;
            let Ok(pi) = info(&pid) else {
                complete = false;
                continue;
            };
            let offset =
                int(&pi, "PartitionMapPartitionOffset").or_else(|| int(&pi, "PartitionOffset"));
            let size = int(&pi, "TotalSize").or_else(|| int(p, "Size"));
            let (Some(offset_bytes), Some(capacity_bytes)) = (offset, size) else {
                complete = false;
                continue;
            };
            let mut vs = Vec::new();
            if string(&pi, "FilesystemType").is_some()
                && let Some(v) = volume(&pi, false)
            {
                vs.push(v);
            }
            if let Some(apfs) = apfs {
                for container in array(apfs, "Containers") {
                    if array(container, "PhysicalStores")
                        .iter()
                        .any(|s| string(s, "DeviceIdentifier").as_deref() == Some(&pid))
                    {
                        for v in array(container, "Volumes") {
                            if let Some(v) = volume(v, true) {
                                vs.push(v);
                            }
                        }
                        limitations.push("APFS volumes share container space; their usage is not summed as physical partition usage.".into());
                    }
                }
            }
            ps.push(Partition {
                id: string(&pi, "DiskUUID").unwrap_or(pid.clone()),
                number: pid.rsplit('s').next().and_then(|s| s.parse().ok()),
                name: string(&pi, "PartitionName"),
                partition_type: string(p, "Content"),
                offset_bytes,
                capacity_bytes,
                volumes: vs,
            });
        }
        ps.sort_by_key(|p| p.offset_bytes);
        if string(&di, "FilesystemType").is_some() && ps.is_empty() {
            complete = false;
            limitations.push("Whole-device filesystem; no partition map is available.".into());
        }
        if !complete {
            limitations.push("Exact partition geometry is unavailable.".into());
        }
        if apfs.is_none() {
            limitations.push("APFS logical volume information is unavailable.".into());
        }
        let internal = flag(&di, "Internal");
        let removable = flag(&di, "RemovableMedia");
        disks.push(Disk {
            id,
            device_path: format!("/dev/{name}"),
            name: model.clone().unwrap_or(name),
            manufacturer: None,
            model,
            serial,
            kind: match internal {
                Some(true) => DeviceKind::Internal,
                Some(false) => DeviceKind::External,
                None => DeviceKind::Unknown,
            },
            removable,
            connection: string(&di, "BusProtocol"),
            partition_scheme: string(d, "Content").and_then(|s| match s.as_str() {
                "GUID_partition_scheme" => Some("GPT".into()),
                "FDisk_partition_scheme" => Some("MBR".into()),
                "Apple_partition_scheme" => Some("APM".into()),
                _ => None,
            }),
            online: None,
            read_only: flag(&di, "WritableMedia")
                .or_else(|| flag(&di, "Writable"))
                .map(|v| !v),
            system: None,
            capacity_bytes: capacity,
            partitions: complete.then_some(ps),
            limitations,
        });
    }
    Ok(disks)
}

#[cfg(target_os = "macos")]
pub fn discover() -> Result<Vec<Disk>, PlatformError> {
    fn read(args: &[&str]) -> Result<Value, PlatformError> {
        Value::from_reader(std::io::Cursor::new(crate::utility::run(
            "/usr/sbin/diskutil",
            args,
        )?))
        .map_err(|_| PlatformError::invalid("diskutil returned an invalid plist."))
    }
    let list = read(&["list", "-plist", "physical"])?;
    let apfs = read(&["apfs", "list", "-plist"]).ok();
    map(&list, apfs.as_ref(), |id| {
        if !device_id(id) {
            return Err(PlatformError::invalid("Invalid BSD identifier."));
        }
        read(&["info", "-plist", id])
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn plist(json: &str) -> Value {
        let value: serde_json::Value = serde_json::from_str(json).unwrap();
        plist::to_value(&value).unwrap()
    }
    #[test]
    fn apfs_physical_relationships_and_geometry() {
        let list = plist(
            r#"{"AllDisksAndPartitions":[{"DeviceIdentifier":"disk0","Content":"GUID_partition_scheme","Partitions":[{"DeviceIdentifier":"disk0s2","Size":8000,"Content":"Apple_APFS"}]}]}"#,
        );
        let apfs = plist(
            r#"{"Containers":[{"PhysicalStores":[{"DeviceIdentifier":"disk0s2"}],"Volumes":[{"DeviceIdentifier":"disk3s1","Name":"Macintosh HD","CapacityInUse":4000}]}]}"#,
        );
        let ds = map(&list, Some(&apfs), |id| {
            Ok(if id == "disk0" {
                plist(r#"{"TotalSize":10000,"Internal":true,"MediaName":"Apple SSD"}"#)
            } else {
                plist(r#"{"TotalSize":8000,"PartitionMapPartitionOffset":512}"#)
            })
        })
        .unwrap();
        assert_eq!(ds.len(), 1);
        let p = &ds[0].partitions.as_ref().unwrap()[0];
        assert_eq!(p.offset_bytes, 512);
        assert_eq!(p.volumes[0].label.as_deref(), Some("Macintosh HD"));
        assert!(p.volumes[0].shared_space);
        diskvio_core::validate_inventory(&ds).unwrap();
    }
    #[test]
    fn invalid_identifiers_and_discovery_errors() {
        assert!(device_id("disk12s3"));
        assert!(!device_id("disk0;erase"));
        assert!(!device_id("../disk0"));
        assert!(map(&Value::String("bad".into()), None, |_| unreachable!()).is_err());
    }
}
