use crate::Disk;
use std::process::Command;

pub fn list_disks() -> Result<Vec<Disk>, Box<dyn std::error::Error>> {
    let script = r#"
        $ErrorActionPreference = 'Stop'
        [Console]::OutputEncoding = New-Object System.Text.UTF8Encoding($false)

        $disks = @(
            Get-Disk | Sort-Object Number | ForEach-Object {
                [PSCustomObject]@{
                    number = [uint32]$_.Number
                    name = [string]$_.FriendlyName
                    size_bytes = [uint64]$_.Size
                    bus_type = [string]$_.BusType
                    partition_style = [string]$_.PartitionStyle
                }
            }
        )

        ConvertTo-Json -InputObject $disks -Depth 3 -Compress
    "#;

    let output = Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .output()?;

    if !output.status.success() {
        return Err(format!(
            "Disk discovery failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    let disks: Vec<Disk> = serde_json::from_slice(&output.stdout)?;

    Ok(disks)
}

pub fn disk_inventory() -> Result<crate::DiskInventory, Box<dyn std::error::Error>> {
    let disks = list_disks()?
        .into_iter()
        .map(|disk| crate::PhysicalDisk {
            device: crate::Device {
                identifier: format!("PhysicalDrive{}", disk.number),
                name: disk.name,
                size_bytes: Some(disk.size_bytes),
                ..Default::default()
            },
            number: disk.number,
            partition_scheme: crate::PartitionScheme::from_content(Some(&disk.partition_style)),
            connection_type: Some(disk.bus_type),
            internal: None,
            removable: None,
            ejectable: None,
            registry_entry_id: None,
            partitions: vec![],
        })
        .collect();
    Ok(crate::DiskInventory {
        disks,
        apfs_containers: vec![],
        warnings: vec![
            "Partition and filesystem exploration is not implemented on Windows yet".into(),
        ],
    })
}
