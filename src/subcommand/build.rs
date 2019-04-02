use crate::{args::Args, builder::Builder, config, ErrorMessage};
use std::{path::PathBuf, process};

pub(crate) fn build(mut args: Args) -> Result<(), ErrorMessage> {
    let builder = Builder::new(args.manifest_path().clone())?;
    let config = config::read_config(builder.kernel_manifest_path())?;
    args.apply_default_target(&config, builder.kernel_root());

    let quiet = args.quiet;
    build_impl(&builder, &mut args, quiet).map(|_| ())
}

pub(crate) fn build_impl(
    builder: &Builder,
    args: &Args,
    quiet: bool,
) -> Result<Vec<PathBuf>, ErrorMessage> {
    run_cargo_fetch(&args);

    let executables = builder.build_kernel(&args.cargo_args, quiet)?;
    if executables.len() == 0 {
        Err("no executables built")?;
    }

    let mut bootimages = Vec::new();

    for executable in executables {
        let out_dir = executable.parent().ok_or("executable has no parent path")?;
        let file_stem = executable
            .file_stem()
            .ok_or("executable has no file stem")?
            .to_str()
            .ok_or("executable file stem not valid utf8")?;

        let bootimage_path = out_dir.join(format!("bootimage-{}.bin", file_stem));
        builder.create_bootimage(&executable, &bootimage_path, quiet)?;
        bootimages.push(bootimage_path);
    }

    Ok(bootimages)
}

fn run_cargo_fetch(args: &Args) {
    let mut command = process::Command::new("cargo");
    command.arg("fetch");
    if let Some(manifest_path) = args.manifest_path() {
        command.arg("--manifest-path");
        command.arg(manifest_path);
    }
    if !command.status().map(|s| s.success()).unwrap_or(false) {
        process::exit(1);
    }
}
