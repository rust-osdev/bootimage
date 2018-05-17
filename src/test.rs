use std::{io, process};
use Error;
use args::Args;
use build;

pub(crate) fn test(args: Args) -> Result<(), Error> {
    run_cargo_test()?;
    run_integration_tests(args)?;
    Ok(())
}

fn run_cargo_test() -> io::Result<process::ExitStatus> {
    let mut command = process::Command::new("cargo");
    command.arg("test");
    command.status()
}

fn run_integration_tests(args: Args) -> Result<(), Error> {
    let (args, config, metadata, root_dir, out_dir) = build::common_setup(args)?;

    let test_args = args.clone();
    let test_run_command = vec![
        "qemu-system-x86_64".into(),
        "-drive".into(),
        "format=raw,file={}".into(),
        "-device".into(),
        "isa-debug-exit,iobase=0xf4,iosize=0x04".into(),
        "-display".into(),
        "none".into(),
        "-serial".into(),
        "file:{}-output.txt".into(),
    ];
    let test_config = {
        let mut test_config = config.clone();
        test_config.output = None;
        test_config.run_command = test_run_command;
        test_config
    };

    let mut test_paths = Vec::new();

    assert_eq!(metadata.packages.len(), 1, "Only crates with one package are supported");
    let target_iter = metadata.packages[0].targets.iter();
    for target in target_iter.filter(|t| t.kind == ["bin"] && t.name.starts_with("test-")) {
        println!("{}", target.name);
        let mut target_args = test_args.clone();
        target_args.set_bin_name(target.name.clone());
        let path = build::build_impl(&target_args, &test_config, &metadata, &root_dir, &out_dir)?;
        test_paths.push(path);
    }

    for test_path in test_paths {
        println!("{}", test_path.display());
        let mut command = process::Command::new("qemu-system-x86_64");
        command.arg("-drive");
        command.arg(format!("format=raw,file={}", test_path.display()));
        command.arg("-device");
        command.arg("isa-debug-exit,iobase=0xf4,iosize=0x04");
        command.arg("-display");
        command.arg("none");
        command.arg("-serial");
        command.arg(format!("file:{}-output.txt", test_path.display()));
        command.status()?; // TODO timeout
    }

    Ok(())
}
