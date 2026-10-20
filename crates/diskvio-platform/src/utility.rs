//! Private fixed-utility runner; never exposed to IPC and never invokes a shell.
use crate::{ErrorCode, PlatformError};
use std::{
    io::Read,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

pub fn run(path: &str, args: &[&str]) -> Result<Vec<u8>, PlatformError> {
    let mut child = Command::new(path)
        .args(args)
        .env("LC_ALL", "C")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| {
            PlatformError::new(
                if e.kind() == std::io::ErrorKind::PermissionDenied {
                    ErrorCode::PermissionDenied
                } else {
                    ErrorCode::DiscoveryFailed
                },
                format!("Cannot start the read-only storage utility: {e}"),
            )
        })?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| PlatformError::invalid("Missing discovery output pipe."))?;
    let reader = thread::spawn(move || {
        let mut data = Vec::new();
        stdout
            .take(16 * 1024 * 1024 + 1)
            .read_to_end(&mut data)
            .map(|_| data)
    });
    let start = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if start.elapsed() < Duration::from_secs(20) => {
                thread::sleep(Duration::from_millis(25))
            }
            other => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = reader.join();
                return Err(PlatformError::new(
                    if other.is_err() {
                        ErrorCode::DiscoveryFailed
                    } else {
                        ErrorCode::Timeout
                    },
                    "The read-only storage utility did not complete.",
                ));
            }
        }
    };
    let output = reader
        .join()
        .map_err(|_| PlatformError::invalid("Discovery output reader failed."))?
        .map_err(|_| PlatformError::invalid("Cannot read discovery output."))?;
    if !status.success() {
        return Err(PlatformError::new(
            ErrorCode::DiscoveryFailed,
            "The read-only storage utility failed. It may be unavailable or access may be restricted.",
        ));
    }
    if output.len() > 16 * 1024 * 1024 {
        return Err(PlatformError::invalid(
            "Discovery output exceeded the size limit.",
        ));
    }
    Ok(output)
}
