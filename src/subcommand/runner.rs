use crate::{args::RunnerArgs, builder::Builder, ErrorString};
use std::process;

pub(crate) fn runner(args: RunnerArgs) -> Result<(), ErrorString> {
    let out_dir = tempdir::TempDir::new("bootimage-runner")?;
    let bootimage_bin = out_dir.path().join("bootimage.bin");

    let builder = Builder::new(None)?;
    builder.create_bootimage(&args.executable, &bootimage_bin, false)?;

    let run_cmd = args.run_command.unwrap_or(vec![
        "qemu-system-x86_64".into(),
        "-drive".into(),
        "format=raw,file={bootimage}".into(),
    ]);

    println!("Running {:?}", run_cmd);

    let mut command = process::Command::new(&run_cmd[0]);
    for arg in &run_cmd[1..] {
        command.arg(arg.replace("{bootimage}", &format!("{}", bootimage_bin.display())));
    }
    if let Some(run_args) = args.run_args {
        command.args(run_args);
    }
    let output = command
        .output()
        .map_err(|e| format!("Failed to execute `{:?}`: {}", command, e))?;

    if !output.status.success() {
        return Err(ErrorString {
            exit_code: output.status.code().unwrap_or(1),
            message: Box::new(format!(
                "Command `{:?}` failed:\n{}",
                command,
                String::from_utf8_lossy(&output.stderr)
            )),
        });
    }

    drop(bootimage_bin);
    out_dir.close()?;

    Ok(())
}
