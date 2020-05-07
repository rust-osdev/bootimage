/// Executable for `bootimage runner`.
use anyhow::{anyhow, Context, Result};
use bootimage::{
    args::{RunnerArgs, RunnerCommand},
    builder::Builder,
    config, help,
};
use std::{env, path::Path};
use std::{process, time::Duration};
use wait_timeout::ChildExt;

pub fn main() -> Result<()> {
    let mut raw_args = env::args();

    let executable_name = raw_args
        .next()
        .ok_or_else(|| anyhow!("no first argument (executable name)"))?;
    let file_stem = Path::new(&executable_name)
        .file_stem()
        .and_then(|s| s.to_str());
    if file_stem != Some("bootimage") {
        return Err(anyhow!(
            "Unexpected executable name: expected `bootimage`, got: `{:?}`",
            file_stem
        ));
    }
    match raw_args.next().as_deref() {
        Some("runner") => {},
        Some("--help") | Some("-h") => {
            help::print_help();
            return Ok(())
        }
        Some(other) => return Err(anyhow!(
            "Unsupported subcommand `{:?}`. See `bootimage --help` for an overview of supported subcommands.", other
        )),
        None => return Err(anyhow!(
            "Please invoke bootimage with a subcommand. See `bootimage --help` for more information."
        )),
    }

    let exit_code = match RunnerCommand::parse_args(raw_args)? {
        RunnerCommand::Runner(args) => Some(runner(args)?),
        RunnerCommand::Version => {
            help::print_version();
            None
        }
        RunnerCommand::Help => {
            help::print_runner_help();
            None
        }
    };

    if let Some(code) = exit_code {
        process::exit(code);
    }

    Ok(())
}

pub(crate) fn runner(args: RunnerArgs) -> Result<i32> {
    let builder = Builder::new(None)?;
    let config = config::read_config(builder.manifest_path())?;
    let exe_parent = args
        .executable
        .parent()
        .ok_or_else(|| anyhow!("kernel executable has no parent"))?;
    let is_doctest = exe_parent
        .file_name()
        .ok_or_else(|| anyhow!("kernel executable's parent has no file name"))?
        .to_str()
        .ok_or_else(|| anyhow!(
            "kernel executable's parent file name is not valid UTF-8"
        ))?
        .starts_with("rustdoctest");
    let is_test = is_doctest || exe_parent.ends_with("deps");

    let bin_name = args
        .executable
        .file_stem()
        .ok_or_else(|| anyhow!("kernel executable has no file stem"))?
        .to_str()
        .ok_or_else(|| anyhow!("kernel executable file stem is not valid UTF-8"))?;

    let output_bin_path = exe_parent.join(format!("bootimage-{}.bin", bin_name));
    let executable_canonicalized = args.executable.canonicalize().with_context(|| {
        format!(
            "failed to canonicalize executable path `{}`",
            args.executable.display(),
        )
    })?;
    builder.create_bootimage(
        bin_name,
        &executable_canonicalized,
        &output_bin_path,
        args.quiet,
    )?;

    let mut run_command: Vec<_> = config
        .run_command
        .iter()
        .map(|arg| arg.replace("{}", &format!("{}", output_bin_path.display())))
        .collect();
    if is_test {
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
        let mut child = command
            .spawn()
            .with_context(|| format!("Failed to launch QEMU: {:?}", command))?;
        let timeout = Duration::from_secs(config.test_timeout.into());
        match child
            .wait_timeout(timeout)
            .context("Failed to wait with timeout")?
        {
            None => {
                child.kill().context("Failed to kill QEMU")?;
                child.wait().context("Failed to wait for QEMU process")?;
                return Err(anyhow!("Timed Out"));
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
                let qemu_exit_code = exit_status
                    .code()
                    .ok_or_else(|| anyhow!("Failed to read QEMU exit code"))?;
                match config.test_success_exit_code {
                    Some(code) if qemu_exit_code == code => 0,
                    _ => qemu_exit_code,
                }
            }
        }
    } else {
        let status = command
            .status()
            .with_context(|| format!("Failed to execute `{:?}`", command))?;
        status.code().unwrap_or(1)
    };

    Ok(exit_code)
}
