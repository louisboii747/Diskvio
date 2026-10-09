use crate::Disk;
use std::process::Command;
use std::{os::windows::process::CommandExt, time::Duration};

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

    // A desktop refresh must not open a console window. Use the system copy
    // rather than resolving an executable from the application's directory.
    let system_root = std::env::var_os("SystemRoot").ok_or("SystemRoot is not set")?;
    let powershell = std::path::PathBuf::from(system_root)
        .join("System32/WindowsPowerShell/v1.0/powershell.exe");
    let mut child = Command::new(powershell)
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .creation_flags(0x0800_0000) // CREATE_NO_WINDOW
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|error| format!("Could not run Windows disk discovery: {error}"))?;

    // Drain both pipes while Get-Disk runs to avoid pipe-buffer deadlocks.
    let stdout = child.stdout.take().ok_or("Missing discovery stdout")?;
    let stderr = child.stderr.take().ok_or("Missing discovery stderr")?;
    let read_pipe = |mut pipe: Box<dyn std::io::Read + Send>| {
        let mut bytes = Vec::new();
        pipe.read_to_end(&mut bytes).map(|_| bytes)
    };
    let stdout_reader = std::thread::spawn(move || read_pipe(Box::new(stdout)));
    let stderr_reader = std::thread::spawn(move || read_pipe(Box::new(stderr)));
    let started = std::time::Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if started.elapsed() >= Duration::from_secs(30) {
            let _ = child.kill();
            let _ = child.wait();
            let _ = stdout_reader.join();
            let _ = stderr_reader.join();
            return Err("Windows disk discovery timed out after 30 seconds".into());
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    let stdout = stdout_reader
        .join()
        .map_err(|_| "Discovery stdout reader failed")??;
    let stderr = stderr_reader
        .join()
        .map_err(|_| "Discovery stderr reader failed")??;

    if !status.success() {
        return Err(format!(
            "Disk discovery failed: {}",
            String::from_utf8_lossy(&stderr).trim()
        )
        .into());
    }

    let disks: Vec<Disk> = serde_json::from_slice(&stdout)?;

    Ok(disks)
}
