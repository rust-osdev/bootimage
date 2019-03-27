use crate::{args::Args, builder::Builder, cargo_config, config, ErrorString};
use std::{
    path::{Path, PathBuf},
    process,
};

pub(crate) fn build(mut args: Args) -> Result<(), ErrorString> {
    let builder = Builder::new(args.manifest_path().clone())?;
    let config = config::read_config(builder.kernel_manifest_path().to_owned())?;
    args.apply_default_target(&config, builder.kernel_root());

    build_impl(&builder, &mut args, false).map(|_| ())
}

pub(crate) fn build_impl(
    builder: &Builder,
    args: &Args,
    quiet: bool,
) -> Result<PathBuf, ErrorString> {
    run_cargo_fetch(&args);

    builder.build_kernel(&args.cargo_args, quiet)?;

    let out_dir = out_dir(&args, &builder)?;
    let kernel_package = builder
        .kernel_package()
        .map_err(|key| format!("Kernel package not found in cargo metadata (`{}`)", key))?;
    let kernel_bin_name = args.bin_name().as_ref().unwrap_or(&kernel_package.name);
    let kernel_path = out_dir.join(kernel_bin_name);

    let bootimage_path = out_dir.join(format!("bootimage-{}.bin", kernel_bin_name));
    builder.create_bootimage(&kernel_path, &bootimage_path, quiet)?;
    Ok(bootimage_path)
}

fn out_dir(args: &Args, builder: &Builder) -> Result<PathBuf, ErrorString> {
    let target_dir = PathBuf::from(&builder.kernel_metadata().target_directory);
    let mut out_dir = target_dir;
    if let &Some(ref target) = args.target() {
        out_dir.push(Path::new(target).file_stem().unwrap().to_str().unwrap());
    } else {
        let default_triple = cargo_config::default_target_triple(builder.kernel_root(), true)?;
        if let Some(triple) = default_triple {
            out_dir.push(triple);
        }
    }
    if args.release() {
        out_dir.push("release");
    } else {
        out_dir.push("debug");
    }
    Ok(out_dir)
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
