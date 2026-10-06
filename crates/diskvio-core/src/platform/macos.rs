use crate::Disk;
use serde::Deserialize;
use std::{error::Error, process::Command};

#[derive(Deserialize)]
struct DiskList {
    #[serde(rename = "WholeDisks")]
    whole_disks: Vec<String>,
    #[serde(rename = "AllDisksAndPartitions", default)]
    entries: Vec<DiskEntry>,
}

#[derive(Deserialize)]
struct DiskEntry {
    #[serde(rename = "DeviceIdentifier")]
    identifier: Option<String>,
    #[serde(rename = "Size")]
    size: Option<u64>,
    #[serde(rename = "Content")]
    content: Option<String>,
}

#[derive(Deserialize)]
struct DiskInfo {
    #[serde(rename = "VirtualOrPhysical")]
    virtual_or_physical: Option<String>,
    #[serde(rename = "WholeDisk")]
    whole_disk: Option<bool>,
    #[serde(rename = "MediaName")]
    media_name: Option<String>,
    #[serde(rename = "DeviceModel")]
    device_model: Option<String>,
    #[serde(rename = "TotalSize")]
    total_size: Option<u64>,
    #[serde(rename = "Size")]
    size: Option<u64>,
    #[serde(rename = "BusProtocol")]
    bus_protocol: Option<String>,
    #[serde(rename = "Protocol")]
    protocol: Option<String>,
    #[serde(rename = "Content")]
    content: Option<String>,
}

pub fn list_disks() -> Result<Vec<Disk>, Box<dyn Error>> {
    // The physical filter excludes synthesized APFS containers and disk images.
    let output = run_diskutil(&["list", "-plist", "physical"])?;
    let list: DiskList = plist::from_bytes(&output)
        .map_err(|error| format!("Could not parse diskutil list plist: {error}"))?;

    let mut disks = Vec::new();
    for identifier in &list.whole_disks {
        let Some(number) = disk_number(identifier) else {
            continue;
        };
        let device = format!("/dev/{identifier}");
        let output = run_diskutil(&["info", "-plist", &device])?;
        let info: DiskInfo = plist::from_bytes(&output).map_err(|error| {
            format!("Could not parse diskutil info plist for {device}: {error}")
        })?;
        let entry = list
            .entries
            .iter()
            .find(|entry| entry.identifier.as_deref() == Some(identifier));
        if let Some(disk) = map_disk(identifier, number, &info, entry) {
            disks.push(disk);
        }
    }
    disks.sort_by_key(|disk| disk.number);
    Ok(disks)
}

fn run_diskutil(args: &[&str]) -> Result<Vec<u8>, Box<dyn Error>> {
    let command = format!("diskutil {}", args.join(" "));
    let output = Command::new("diskutil")
        .args(args)
        .output()
        .map_err(|error| format!("Could not run {command}: {error}"))?;
    if !output.status.success() {
        let detail = String::from_utf8_lossy(&output.stderr);
        let detail = detail.trim();
        let detail = if detail.is_empty() {
            format!("exited with {}", output.status)
        } else {
            detail.to_owned()
        };
        return Err(format!("{command} failed: {detail}").into());
    }
    Ok(output.stdout)
}

fn disk_number(identifier: &str) -> Option<u32> {
    identifier.strip_prefix("disk")?.parse::<u32>().ok()
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

    let name = nonempty(info.media_name.as_deref())
        .or_else(|| nonempty(info.device_model.as_deref()))
        .unwrap_or(identifier);
    let bus_type = nonempty(info.bus_protocol.as_deref())
        .or_else(|| nonempty(info.protocol.as_deref()))
        .unwrap_or("Unknown");
    let partition_style = entry
        .and_then(|entry| entry.content.as_deref())
        .or(info.content.as_deref())
        .filter(|content| content.ends_with("_partition_scheme"))
        .unwrap_or("Unknown");

    Some(Disk {
        number,
        name: name.to_owned(),
        size_bytes: info
            .total_size
            .or(info.size)
            .or_else(|| entry.and_then(|entry| entry.size))
            .unwrap_or(0),
        bus_type: bus_type.to_owned(),
        partition_style: partition_style.to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIST: &[u8] = br#"<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0"><dict>
  <key>WholeDisks</key><array><string>disk0</string><string>disk4</string></array>
  <key>AllDisksAndPartitions</key><array>
    <dict><key>DeviceIdentifier</key><string>disk0</string><key>Size</key><integer>500000000000</integer><key>Content</key><string>GUID_partition_scheme</string></dict>
  </array>
</dict></plist>"#;

    const INFO: &[u8] = br#"<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0"><dict>
  <key>VirtualOrPhysical</key><string>Physical</string>
  <key>WholeDisk</key><true/>
  <key>MediaName</key><string>Example SSD</string>
  <key>TotalSize</key><integer>500000000000</integer>
  <key>BusProtocol</key><string>PCI-Express</string>
</dict></plist>"#;

    #[test]
    fn maps_physical_disk_from_plists() {
        let list: DiskList = plist::from_bytes(LIST).unwrap();
        let info: DiskInfo = plist::from_bytes(INFO).unwrap();
        let disk = map_disk("disk0", 0, &info, list.entries.first()).unwrap();

        assert_eq!(list.whole_disks, ["disk0", "disk4"]);
        assert_eq!(disk.number, 0);
        assert_eq!(disk.name, "Example SSD");
        assert_eq!(disk.size_bytes, 500_000_000_000);
        assert_eq!(disk.bus_type, "PCI-Express");
        assert_eq!(disk.partition_style, "GUID_partition_scheme");
    }

    #[test]
    fn skips_virtual_disks_and_handles_missing_fields() {
        let mut info: DiskInfo = plist::from_bytes(INFO).unwrap();
        info.virtual_or_physical = Some("Virtual".to_owned());
        assert!(map_disk("disk0", 0, &info, None).is_none());

        info.virtual_or_physical = Some("Unknown".to_owned());
        assert!(map_disk("disk0", 0, &info, None).is_some());

        let info: DiskInfo = plist::from_bytes(b"<plist version=\"1.0\"><dict/></plist>").unwrap();
        let disk = map_disk("disk4", 4, &info, None).unwrap();
        assert_eq!(disk.name, "disk4");
        assert_eq!(disk.size_bytes, 0);
        assert_eq!(disk.bus_type, "Unknown");
        assert_eq!(disk.partition_style, "Unknown");
        assert_eq!(disk_number("disk4s1"), None);
    }
}
