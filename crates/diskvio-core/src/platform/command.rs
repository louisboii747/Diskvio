use crate::{OperationError, OperationErrorKind};
use std::process::{Command, Stdio};

pub(crate) struct CommandOutput {
    pub success: bool,
    pub code: Option<i32>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

impl CommandOutput {
    pub fn checked(self, program: &str) -> Result<Vec<u8>, OperationError> {
        if self.success {
            return Ok(self.stdout);
        }
        let stderr = String::from_utf8_lossy(&self.stderr);
        let stdout = String::from_utf8_lossy(&self.stdout);
        let detail = if stderr.trim().is_empty() {
            stdout.trim()
        } else {
            stderr.trim()
        };
        let lower = detail.to_ascii_lowercase();
        let code = if lower.contains("permission")
            || lower.contains("access is denied")
            || lower.contains("not authorized")
            || lower.contains("administrator")
            || lower.contains("privilege")
        {
            OperationErrorKind::PermissionDenied
        } else if lower.contains("busy")
            || lower.contains("in use")
            || lower.contains("dissenter")
            || lower.contains("could not be unmounted")
        {
            OperationErrorKind::Busy
        } else {
            OperationErrorKind::CommandFailed
        };
        Err(OperationError {
            code,
            message: format!("{program} failed: {detail}"),
            platform_code: self.code.map(i64::from),
        })
    }
}

pub(crate) trait CommandRunner {
    fn run(&self, program: &str, args: &[&str]) -> Result<CommandOutput, OperationError>;
}

#[derive(Default)]
pub(crate) struct SystemCommandRunner;

impl CommandRunner for SystemCommandRunner {
    fn run(&self, program: &str, args: &[&str]) -> Result<CommandOutput, OperationError> {
        let mut command = Command::new(program);
        command
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x0800_0000);
        }
        #[cfg(target_os = "windows")]
        let output = run_windows_command(command);
        #[cfg(not(target_os = "windows"))]
        let output = command.output();
        let output = output.map_err(|error| {
            OperationError::new(
                if error.kind() == std::io::ErrorKind::PermissionDenied {
                    OperationErrorKind::PermissionDenied
                } else {
                    OperationErrorKind::Io
                },
                format!("Could not run {program}: {error}"),
            )
        })?;
        Ok(CommandOutput {
            success: output.status.success(),
            code: output.status.code(),
            stdout: output.stdout,
            stderr: output.stderr,
        })
    }
}

#[cfg(target_os = "windows")]
fn run_windows_command(mut command: Command) -> std::io::Result<std::process::Output> {
    use std::{
        io::Read,
        time::{Duration, Instant},
    };
    let mut child = command.spawn()?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| std::io::Error::other("Missing command stdout"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| std::io::Error::other("Missing command stderr"))?;
    let read = |mut pipe: Box<dyn Read + Send>| {
        let mut bytes = Vec::new();
        pipe.read_to_end(&mut bytes).map(|_| bytes)
    };
    let stdout = std::thread::spawn(move || read(Box::new(stdout)));
    let stderr = std::thread::spawn(move || read(Box::new(stderr)));
    let started = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Ok(status),
            Ok(None) if started.elapsed() < Duration::from_secs(30) => {
                std::thread::sleep(Duration::from_millis(50))
            }
            Ok(None) => {
                break Err(std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    "Windows Storage discovery timed out after 30 seconds",
                ));
            }
            Err(error) => break Err(error),
        }
    };
    if status.is_err() {
        let _ = child.kill();
        let _ = child.wait();
    }
    let stdout = stdout
        .join()
        .map_err(|_| std::io::Error::other("Command stdout reader failed"))?;
    let stderr = stderr
        .join()
        .map_err(|_| std::io::Error::other("Command stderr reader failed"))?;
    Ok(std::process::Output {
        status: status?,
        stdout: stdout?,
        stderr: stderr?,
    })
}

#[cfg(test)]
type FixtureStep = (String, Vec<String>, Result<CommandOutput, OperationError>);

#[cfg(test)]
pub(crate) struct FixtureRunner {
    steps: std::cell::RefCell<std::collections::VecDeque<FixtureStep>>,
}

#[cfg(test)]
impl FixtureRunner {
    pub fn new(steps: Vec<FixtureStep>) -> Self {
        Self {
            steps: std::cell::RefCell::new(steps.into()),
        }
    }

    pub fn assert_finished(&self) {
        assert!(self.steps.borrow().is_empty());
    }
}

#[cfg(test)]
impl CommandRunner for FixtureRunner {
    fn run(&self, program: &str, args: &[&str]) -> Result<CommandOutput, OperationError> {
        let (expected_program, expected_args, output) = self
            .steps
            .borrow_mut()
            .pop_front()
            .expect("Unexpected command execution");
        assert_eq!(program, expected_program);
        assert_eq!(args, expected_args);
        output
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_failures_preserve_detail_and_code() {
        for (detail, kind) in [
            ("Access is denied", OperationErrorKind::PermissionDenied),
            ("Volume is busy", OperationErrorKind::Busy),
            ("Unexpected failure", OperationErrorKind::CommandFailed),
        ] {
            let error = CommandOutput {
                success: false,
                code: Some(5),
                stdout: vec![],
                stderr: detail.as_bytes().to_vec(),
            }
            .checked("test command")
            .unwrap_err();
            assert_eq!(error.code, kind);
            assert_eq!(error.platform_code, Some(5));
            assert!(error.message.contains(detail));
        }
    }
}
