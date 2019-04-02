use crate::{args::RunnerArgs, builder::Builder, config, ErrorString};
use std::{fs, process};

pub(crate) fn runner(args: RunnerArgs) -> Result<i32, ErrorString> {
    let builder = Builder::new(None)?;
    let config = config::read_config(builder.kernel_manifest_path().to_owned())?;

    let bootimage_bin = {
        let kernel_target_dir = &builder.kernel_metadata().target_directory;
        let bootimage_target_dir = kernel_target_dir.join("bootimage").join("runner");

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
        let sub_path = parent.strip_prefix(kernel_target_dir).map_err(|err| {
            format!(
                "kernel executable does not live in kernel target directory: {}",
                err
            )
        })?;

        let out_dir = bootimage_target_dir.join(sub_path);
        fs::create_dir_all(&out_dir).map_err(|err| {
            format!(
                "failed to create output directory {}: {}",
                out_dir.display(),
                err
            )
        })?;
        out_dir.join(format!("bootimage-{}.bin", file_stem))
    };

    builder.create_bootimage(&args.executable, &bootimage_bin, false)?;

    let mut command = process::Command::new(&config.run_command[0]);
    for arg in &config.run_command[1..] {
        command.arg(arg.replace("{}", &format!("{}", bootimage_bin.display())));
    }
    if let Some(run_args) = config.run_args {
        command.args(run_args);
    }

    println!("Running: {:?}", command);

    let output = command
        .output()
        .map_err(|e| format!("Failed to execute `{:?}`: {}", command, e))?;

    Ok(output.status.code().unwrap_or(1))
}
