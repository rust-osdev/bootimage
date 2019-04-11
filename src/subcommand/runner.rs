use crate::{args::RunnerArgs, builder::Builder, config, ErrorMessage};
use std::process;

pub(crate) fn runner(args: RunnerArgs) -> Result<i32, ErrorMessage> {
    let builder = Builder::new(None)?;
    let config = config::read_config(builder.kernel_manifest_path())?;

    let bootimage_bin = {
        let parent = args
            .executable
            .parent()
            .ok_or("kernel executable has no parent")?;
        let file_stem = args
            .executable
            .file_stem()
            .ok_or("kernel executable has no file stem")?
            .to_str()
            .ok_or("kernel executable file stem is not valid UTF-8")?;
        parent.join(format!("bootimage-{}.bin", file_stem))
    };

    builder.create_bootimage(&args.executable, &bootimage_bin, args.quiet)?;

    let mut command = process::Command::new(&config.run_command[0]);
    for arg in &config.run_command[1..] {
        command.arg(arg.replace("{}", &format!("{}", bootimage_bin.display())));
    }
    if let Some(run_args) = config.run_args {
        command.args(run_args);
    }
    if let Some(args) = args.runner_args {
        command.args(args);
    }

    println!("Running: {:?}", command);

    let output = command
        .output()
        .map_err(|e| format!("Failed to execute `{:?}`: {}", command, e))?;

    Ok(output.status.code().unwrap_or(1))
}
