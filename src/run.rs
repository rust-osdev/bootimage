//! Provides a function for running a disk image in QEMU.

use crate::{args::RunnerArgs, config::Config};
use std::{io, path::Path, process, time::Duration};
use thiserror::Error;
use wait_timeout::ChildExt;

/// Run the given disk image in QEMU.
///
/// Automatically takes into account the runner arguments and the run/test
/// commands defined in the given `Config`. Since test executables are treated
/// differently (run with a timeout and match exit status), the caller needs to
/// specify whether the given disk image is a test or not.
pub fn run(
    config: Config,
    args: RunnerArgs,
    image_path: &Path,
    is_test: bool,
) -> Result<i32, RunError> {
    let mut run_command: Vec<_> = config
        .run_command
        .iter()
        .map(|arg| arg.replace("{}", &format!("{}", image_path.display())))
        .collect();
    if is_test {
        if config.test_no_reboot {
            run_command.push("-no-reboot".to_owned());
        }
        if let Some(args) = config.test_args {
            run_command.extend(args);
        }
    } else if let Some(args) = config.run_args {
        run_command.extend(args);
    }
    if let Some(args) = args.runner_args {
        run_command.extend(args);
    }

    if !args.quiet {
        println!("Running: `{}`", run_command.join(" "));
    }
    let mut command = process::Command::new(&run_command[0]);
    command.args(&run_command[1..]);

    let exit_code = if is_test {
        let mut child = command.spawn().map_err(|error| RunError::Io {
            context: IoErrorContext::QemuTestCommand {
                command: format!("{:?}", command),
            },
            error,
        })?;
        let timeout = Duration::from_secs(config.test_timeout.into());
        match child
            .wait_timeout(timeout)
            .map_err(context(IoErrorContext::WaitWithTimeout))?
        {
            None => {
                child.kill().map_err(context(IoErrorContext::KillQemu))?;
                child.wait().map_err(context(IoErrorContext::WaitForQemu))?;
                return Err(RunError::TestTimedOut);
            }
            Some(exit_status) => {
                #[cfg(unix)]
                {
                    if exit_status.code().is_none() {
                        use std::os::unix::process::ExitStatusExt;
                        if let Some(signal) = exit_status.signal() {
                            eprintln!("QEMU process was terminated by signal {}", signal);
                        }
                    }
                }
                let qemu_exit_code = exit_status.code().ok_or(RunError::NoQemuExitCode)?;
                match config.test_success_exit_code {
                    Some(code) if qemu_exit_code == code => 0,
                    Some(_) if qemu_exit_code == 0 => 1,
                    _ => qemu_exit_code,
                }
            }
        }
    } else {
        let status = command.status().map_err(|error| RunError::Io {
            context: IoErrorContext::QemuRunCommand {
                command: format!("{:?}", command),
            },
            error,
        })?;
        status.code().unwrap_or(1)
    };

    Ok(exit_code)
}

/// Running the disk image failed.
#[derive(Debug, Error)]
pub enum RunError {
    /// Test timed out
    #[error("Test timed out")]
    TestTimedOut,

    /// Failed to read QEMU exit code
    #[error("Failed to read QEMU exit code")]
    NoQemuExitCode,

    /// An I/O error occured
    #[error("{context}: An I/O error occured: {error}")]
    Io {
        /// The operation that caused the I/O error.
        context: IoErrorContext,
        /// The I/O error that occured.
        error: io::Error,
    },
}

/// An I/O error occured while trying to run the disk image.
#[derive(Debug, Error)]
pub enum IoErrorContext {
    /// QEMU command for non-test failed
    #[error("Failed to execute QEMU run command `{command}`")]
    QemuRunCommand {
        /// The QEMU command that was executed
        command: String,
    },

    /// QEMU command for test failed
    #[error("Failed to execute QEMU test command `{command}`")]
    QemuTestCommand {
        /// The QEMU command that was executed
        command: String,
    },

    /// Waiting for test with timeout failed
    #[error("Failed to wait with timeout")]
    WaitWithTimeout,

    /// Failed to kill QEMU
    #[error("Failed to kill QEMU")]
    KillQemu,

    /// Failed to wait for QEMU process
    #[error("Failed to wait for QEMU process")]
    WaitForQemu,
}

/// Helper function for IO error construction
fn context(context: IoErrorContext) -> impl FnOnce(io::Error) -> RunError {
    |error| RunError::Io { context, error }
}
