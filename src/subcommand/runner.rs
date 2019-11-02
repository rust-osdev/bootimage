use crate::{args::RunnerArgs, builder::Builder, config, ErrorMessage};
use std::{process, time::Duration};
use wait_timeout::ChildExt;

pub(crate) fn runner(args: RunnerArgs) -> Result<i32, ErrorMessage> {
    let builder = Builder::new(None)?;
    let config = config::read_config(builder.kernel_manifest_path())?;
    let exe_parent = args
        .executable
        .parent()
        .ok_or("kernel executable has no parent")?;
    let is_test = exe_parent.ends_with("deps");

    let bootimage_bin = {
        let file_stem = args
            .executable
            .file_stem()
            .ok_or("kernel executable has no file stem")?
            .to_str()
            .ok_or("kernel executable file stem is not valid UTF-8")?;
        exe_parent.join(format!("bootimage-{}.bin", file_stem))
    };

    let executable_canonicalized = args.executable.canonicalize().map_err(|err| {
        format!(
            "failed to canonicalize executable path `{}`: {}",
            args.executable.display(),
            err
        )
    })?;
    builder.create_bootimage(&executable_canonicalized, &bootimage_bin, args.quiet)?;

    let mut run_command: Vec<_> = config
        .run_command
        .iter()
        .map(|arg| arg.replace("{}", &format!("{}", bootimage_bin.display())))
        .collect();
    if is_test {
        if let Some(args) = config.test_args {
            run_command.extend(args);
        }
    } else {
        if let Some(args) = config.run_args {
            run_command.extend(args);
        }
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
        let mut child = command
            .spawn()
            .map_err(|e| format!("Failed to launch QEMU: {:?}\n{}", command, e))?;
        let timeout = Duration::from_secs(config.test_timeout.into());
        match child
            .wait_timeout(timeout)
            .map_err(|e| format!("Failed to wait with timeout: {}", e))?
        {
            None => {
                child
                    .kill()
                    .map_err(|e| format!("Failed to kill QEMU: {}", e))?;
                child
                    .wait()
                    .map_err(|e| format!("Failed to wait for QEMU process: {}", e))?;
                return Err(ErrorMessage::from("Timed Out"));
            }
            Some(exit_status) => {
                let qemu_exit_code = exit_status.code().ok_or("Failed to read QEMU exit code")?;
                match config.test_success_exit_code {
                    Some(code) if qemu_exit_code == code => 0,
                    _ => qemu_exit_code,
                }
            }
        }
    } else {
        let status = command
            .status()
            .map_err(|e| format!("Failed to execute `{:?}`: {}", command, e))?;
        status.code().unwrap_or(1)
    };

    Ok(exit_code)
}
