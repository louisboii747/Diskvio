use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Disk {
    pub number: u32,
    pub name: String,
    pub size_bytes: u64,
    pub bus_type: String,
    pub partition_style: String,
}

#[cfg(target_os = "windows")]
pub fn list_disks() -> Result<Vec<Disk>, Box<dyn std::error::Error>> {
    use std::process::Command;

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

#[cfg(not(target_os = "windows"))]
pub fn list_disks() -> Result<Vec<Disk>, Box<dyn std::error::Error>> {
    Err("Disk discovery is not implemented on this platform yet".into())
}
