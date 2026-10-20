use crate::{PlatformError, partition_type, text};
use diskvio_core::{DeviceKind, Disk, Partition, Volume};
use serde_json::Value;

fn string(v: &Value, key: &str) -> Option<String> {
    text(v[key].as_str().map(str::to_owned))
}
fn integer(v: &Value, key: &str) -> Option<u64> {
    v[key].as_u64().or_else(|| v[key].as_str()?.parse().ok())
}
fn flag(v: &Value, key: &str) -> Option<bool> {
    v[key]
        .as_bool()
        .or_else(|| integer(v, key).filter(|n| *n <= 1).map(|n| n == 1))
}
fn children(v: &Value) -> &[Value] {
    v["children"].as_array().map(Vec::as_slice).unwrap_or(&[])
}
fn mounts(v: &Value) -> Vec<String> {
    v["mountpoints"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|v| v.as_str().map(str::to_owned))
        .collect()
}
fn volume(v: &Value, shared: bool) -> Volume {
    Volume {
        id: string(v, "uuid")
            .or_else(|| string(v, "path"))
            .unwrap_or_default(),
        label: string(v, "label"),
        filesystem: string(v, "fstype"),
        mount_points: mounts(v),
        capacity_bytes: integer(v, "fssize"),
        used_bytes: integer(v, "fsused"),
        free_bytes: integer(v, "fsavail"),
        shared_space: shared || matches!(v["fstype"].as_str(), Some("btrfs" | "zfs")),
    }
}
fn volumes(v: &Value) -> Vec<Volume> {
    let mut vs = Vec::new();
    if string(v, "fstype").is_some()
        && !matches!(
            v["fstype"].as_str(),
            Some("LVM2_member" | "crypto_LUKS" | "linux_raid_member")
        )
    {
        vs.push(volume(v, false));
    }
    for child in children(v) {
        let mut nested = volumes(child);
        for v in &mut nested {
            v.shared_space = true;
        }
        vs.extend(nested);
    }
    vs
}
fn is_system(v: &Value) -> bool {
    mounts(v)
        .iter()
        .any(|s| s == "/" || s == "/boot" || s == "/boot/efi")
        || children(v).iter().any(is_system)
}

pub(crate) fn map(
    root: &Value,
    physical: impl Fn(&str) -> bool,
) -> Result<Vec<Disk>, PlatformError> {
    let nodes = root["blockdevices"]
        .as_array()
        .ok_or_else(|| PlatformError::invalid("lsblk returned no blockdevices array."))?;
    let mut disks = Vec::new();
    for d in nodes {
        if d["type"] != "disk" {
            continue;
        }
        let kname = string(d, "kname")
            .ok_or_else(|| PlatformError::invalid("Missing kernel disk name."))?;
        if !physical(&kname) {
            continue;
        }
        let device_path =
            string(d, "path").ok_or_else(|| PlatformError::invalid("Missing disk path."))?;
        let size = integer(d, "size")
            .ok_or_else(|| PlatformError::invalid("Missing physical disk capacity."))?;
        let serial = string(d, "serial");
        let model = string(d, "model");
        let id = format!(
            "linux:{}",
            string(d, "wwn")
                .or_else(|| serial
                    .as_ref()
                    .map(|s| format!("{}:{s}", model.as_deref().unwrap_or(""))))
                .unwrap_or_else(|| device_path.clone())
        );
        let mut partitions = Vec::new();
        let mut limitations = Vec::new();
        let mut complete = true;
        for p in children(d).iter().filter(|p| p["type"] == "part") {
            // The kernel/lsblk START is in 512-byte sectors, independent of logical sector size.
            let offset = integer(p, "start").and_then(|s| s.checked_mul(512));
            let length = integer(p, "size");
            let (Some(offset_bytes), Some(capacity_bytes)) = (offset, length) else {
                complete = false;
                continue;
            };
            let number = string(p, "kname")
                .and_then(|p| {
                    p.strip_prefix(&kname)
                        .map(|s| s.trim_start_matches('p').to_owned())
                })
                .and_then(|s| s.parse().ok());
            partitions.push(Partition {
                id: string(p, "partuuid")
                    .or_else(|| string(p, "path"))
                    .ok_or_else(|| PlatformError::invalid("Missing partition identity."))?,
                number,
                name: string(p, "partlabel"),
                partition_type: partition_type(string(p, "parttype").as_deref()),
                offset_bytes,
                capacity_bytes,
                volumes: volumes(p),
            });
        }
        if string(d, "fstype").is_some() {
            complete = false;
            limitations.push("Whole-device filesystem; no partition map is available.".into());
        }
        if !complete {
            limitations.push("Exact partition geometry is unavailable.".into());
        }
        partitions.sort_by_key(|p| p.offset_bytes);
        let connection = string(d, "tran").map(|s| s.to_uppercase());
        let removable = flag(d, "rm");
        disks.push(Disk {
            id,
            device_path,
            name: model.clone().unwrap_or(kname),
            manufacturer: string(d, "vendor"),
            model,
            serial,
            kind: if connection.as_deref() == Some("USB") {
                DeviceKind::External
            } else if removable == Some(true) {
                DeviceKind::Removable
            } else if matches!(connection.as_deref(), Some("NVME" | "SATA" | "ATA")) {
                DeviceKind::Internal
            } else {
                DeviceKind::Unknown
            },
            removable,
            connection,
            partition_scheme: string(d, "pttype").map(|s| match s.as_str() {
                "dos" => "MBR".into(),
                _ => s.to_uppercase(),
            }),
            online: None,
            read_only: flag(d, "ro"),
            system: Some(is_system(d)),
            capacity_bytes: size,
            partitions: complete.then_some(partitions),
            limitations,
        });
    }
    Ok(disks)
}

#[cfg(target_os = "linux")]
pub fn discover() -> Result<Vec<Disk>, PlatformError> {
    // A missing/inaccessible topology provider must not masquerade as an empty inventory.
    let topology = std::fs::read_dir("/sys/class/block").map_err(|_| {
        crate::PlatformError::new(
            crate::ErrorCode::DiscoveryFailed,
            "Linux sysfs block topology is unavailable.",
        )
    })?;
    let mut physical = std::collections::HashSet::new();
    for entry in topology {
        let entry =
            entry.map_err(|_| PlatformError::invalid("Cannot read Linux block topology."))?;
        let path = std::fs::canonicalize(entry.path()).map_err(|_| {
            PlatformError::invalid(
                "Linux block topology changed or became inaccessible. Refresh to retry.",
            )
        })?;
        if !path.starts_with("/sys/devices/virtual") {
            physical.insert(entry.file_name().to_string_lossy().into_owned());
        }
    }
    let path = if std::path::Path::new("/usr/bin/lsblk").exists() {
        "/usr/bin/lsblk"
    } else {
        "/bin/lsblk"
    };
    let output = crate::utility::run(
        path,
        &[
            "--json",
            "--bytes",
            "--tree",
            "--output",
            "NAME,KNAME,PATH,TYPE,SIZE,START,MODEL,SERIAL,VENDOR,TRAN,RM,RO,PTTYPE,PARTTYPE,PARTLABEL,PARTUUID,FSTYPE,LABEL,UUID,MOUNTPOINTS,FSSIZE,FSUSED,FSAVAIL,WWN",
        ],
    )?;
    let root = serde_json::from_slice(&output)
        .map_err(|_| PlatformError::invalid("lsblk returned invalid JSON."))?;
    map(&root, |name| physical.contains(name))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn physical_mapping_preserves_bytes_and_missing_usage() {
        let root = serde_json::json!({"blockdevices":[{"type":"disk","kname":"nvme0n1","path":"/dev/nvme0n1","size":10000,"tran":"nvme","children":[{"type":"part","kname":"nvme0n1p1","path":"/dev/nvme0n1p1","size":4096,"start":1,"fstype":"ext4","label":"Data","mountpoints":["/"]}]},{"type":"loop"},{"type":"disk","kname":"virtual"}]});
        let ds = map(&root, |s| s != "virtual").unwrap();
        assert_eq!(ds.len(), 1);
        let p = &ds[0].partitions.as_ref().unwrap()[0];
        assert_eq!(p.offset_bytes, 512);
        assert_eq!(p.number, Some(1));
        assert_eq!(p.volumes[0].used_bytes, None);
        assert_eq!(ds[0].system, Some(true));
        diskvio_core::validate_inventory(&ds).unwrap();
    }
    #[test]
    fn malformed_empty_and_unavailable_geometry() {
        assert!(map(&Value::Null, |_| true).is_err());
        assert!(
            map(&serde_json::json!({"blockdevices":[]}), |_| true)
                .unwrap()
                .is_empty()
        );
        let root = serde_json::json!({"blockdevices":[{"type":"disk","kname":"sda","path":"/dev/sda","size":"10000","children":[{"type":"part","size":2000}]}]});
        assert!(map(&root, |_| true).unwrap()[0].partitions.is_none());
    }
}
