use crate::{args::Args, builder::Builder, config, ErrorMessage};
use std::process;

pub(crate) fn run(mut args: Args) -> Result<i32, ErrorMessage> {
    use crate::subcommand::build;

    let builder = Builder::new(args.manifest_path().clone())?;
    let config = config::read_config(builder.kernel_manifest_path().to_owned())?;
    args.apply_default_target(&config, builder.kernel_root());

    let quiet = args.quiet;
    let bootimages = build::build_impl(&builder, &mut args, quiet)?;
    let bootimage_path = bootimages.first().ok_or("no bootimages created")?;
    if bootimages.len() > 1 {
        Err("more than one bootimage created")?;
    }

    let command = &config.run_command[0];
    let mut command = process::Command::new(command);
    for arg in &config.run_command[1..] {
        command.arg(
            arg.replace(
                "{}",
                bootimage_path
                    .to_str()
                    .ok_or(ErrorMessage::from("bootimage path is not valid unicode"))?,
            ),
        );
    }
    if let Some(run_args) = config.run_args {
        command.args(run_args);
    }
    command.args(&args.run_args);
    let exit_status = command.status().map_err(|err| {
        ErrorMessage::from(format!(
            "Failed to execute run command `{:?}`: {}",
            command, err
        ))
    })?;
    Ok(exit_status.code().unwrap_or(1))
}
