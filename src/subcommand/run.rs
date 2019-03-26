use crate::{args::Args, config, ErrorString};
use std::process;

pub(crate) fn run(mut args: Args) -> Result<(), ErrorString> {
    use crate::subcommand::build;

    let builder = bootimage::Builder::new(args.manifest_path().clone())?;
    let config = config::read_config(builder.kernel_manifest_path().to_owned())?;
    args.apply_default_target(&config, builder.kernel_root());

    let bootimage_path = build::build_impl(&builder, &mut args, false)?;

    let command = &config.run_command[0];
    let mut command = process::Command::new(command);
    for arg in &config.run_command[1..] {
        command.arg(
            arg.replace(
                "{}",
                bootimage_path
                    .to_str()
                    .ok_or(ErrorString::from("bootimage path is not valid unicode"))?,
            ),
        );
    }
    command.args(&args.run_args);
    command.status().map_err(|err| {
        ErrorString::from(format!(
            "Failed to execute run command `{:?}`: {}",
            command, err
        ))
    })?;
    Ok(())
}
