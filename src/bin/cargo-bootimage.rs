use anyhow::{anyhow, Result};
use bootimage::{
    args::{BuildArgs, BuildCommand},
    builder::Builder,
    help,
};
use std::{env, path::{PathBuf, Path}};

pub fn main() -> Result<()> {
    let mut raw_args = env::args();

    let executable_name = raw_args
        .next()
        .ok_or(anyhow!("no first argument (executable name)"))?;
    let file_stem = Path::new(&executable_name)
        .file_stem()
        .and_then(|s| s.to_str());
    if file_stem != Some("cargo-bootimage") {
        return Err(anyhow!(
            "Unexpected executable name: expected `cargo-bootimage`, got: `{:?}`",
            file_stem
        ));
    }
    if raw_args.next().as_deref() != Some("bootimage") {
        return Err(anyhow!("Please invoke this as `cargo bootimage`"));
    }

    match BuildCommand::parse_args(raw_args)? {
        BuildCommand::Build(args) => build(args),
        BuildCommand::Version => Ok(help::print_version()),
        BuildCommand::Help => Ok(help::print_cargo_bootimage_help()),
    }
}

fn build(args: BuildArgs) -> Result<()> {
    let builder = Builder::new(args.manifest_path().map(PathBuf::from))?;
    let quiet = args.quiet();

    let executables = builder.build_kernel(&args.cargo_args(), quiet)?;
    if executables.len() == 0 {
        return Err(anyhow!("no executables built"));
    }

    for executable in executables {
        let out_dir = executable
            .parent()
            .ok_or(anyhow!("executable has no parent path"))?;
        let bin_name = &executable
            .file_stem()
            .ok_or(anyhow!("executable has no file stem"))?
            .to_str()
            .ok_or(anyhow!("executable file stem not valid utf8"))?;

        let bootimage_path = out_dir.join(format!("bootimage-{}.bin", bin_name));
        builder.create_bootimage(bin_name, &executable, &bootimage_path, quiet)?;
        if !args.quiet() {
            println!(
                "Created bootimage for `{}` at `{}`",
                bin_name,
                bootimage_path.display()
            );
        }
    }

    Ok(())
}
