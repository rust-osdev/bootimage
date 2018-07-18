use args::Args;
use build;
use failure::{Error, ResultExt};
use std::io::Write;
use std::path::Path;
use std::time::Duration;
use std::{fs, io, process};
use wait_timeout::ChildExt;

pub(crate) fn test(args: Args) -> Result<(), Error> {
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

    let mut tests = Vec::new();

    let crate_metadata = metadata
        .packages
        .iter()
        .find(|p| Path::new(&p.manifest_path) == config.manifest_path)
        .expect("Could not read crate name from cargo metadata");
    let target_iter = crate_metadata.targets.iter();
    for target in target_iter.filter(|t| t.kind == ["bin"] && t.name.starts_with("test-")) {
        println!("{}", target.name);

        let mut target_args = test_args.clone();
        target_args.set_bin_name(target.name.clone());
        let test_path = build::build_impl(
            &target_args,
            &test_config,
            &metadata,
            &root_dir,
            &out_dir,
            false,
        )?;

        let test_result;
        let output_file = format!("{}-output.txt", test_path.display());

        let mut command = process::Command::new("qemu-system-x86_64");
        command.arg("-drive");
        command.arg(format!("format=raw,file={}", test_path.display()));
        command.arg("-device");
        command.arg("isa-debug-exit,iobase=0xf4,iosize=0x04");
        command.arg("-display");
        command.arg("none");
        command.arg("-serial");
        command.arg(format!("file:{}", output_file));
        command.stderr(process::Stdio::null());
        let mut child = command
            .spawn()
            .context(format_err!("Failed to launch QEMU: {:?}", command))?;
        let timeout = Duration::from_secs(60);
        match child
            .wait_timeout(timeout)
            .context("Failed to wait with timeout")?
        {
            None => {
                child.kill().context("Failed to kill QEMU")?;
                child.wait().context("Failed to wait for QEMU process")?;
                test_result = TestResult::TimedOut;
                writeln!(io::stderr(), "Timed Out")?;
            }
            Some(_) => {
                let output = fs::read_to_string(&output_file).context(format_err!(
                    "Failed to read test output file {}",
                    output_file
                ))?;
                if output.starts_with("ok\n") {
                    test_result = TestResult::Ok;
                    println!("Ok");
                } else if output.starts_with("failed\n") {
                    test_result = TestResult::Failed;
                    writeln!(io::stderr(), "Failed:")?;
                    for line in output[7..].lines() {
                        writeln!(io::stderr(), "    {}", line)?;
                    }
                } else {
                    test_result = TestResult::Invalid;
                    writeln!(io::stderr(), "Failed: Invalid Output:")?;
                    for line in output.lines() {
                        writeln!(io::stderr(), "    {}", line)?;
                    }
                }
            }
        }
        println!("");

        tests.push((target.name.clone(), test_result))
    }

    if tests.iter().all(|t| t.1 == TestResult::Ok) {
        println!("All tests succeeded.");
        Ok(())
    } else {
        writeln!(io::stderr(), "The following tests failed:")?;
        for test in tests.iter().filter(|t| t.1 != TestResult::Ok) {
            writeln!(io::stderr(), "    {}: {:?}", test.0, test.1)?;
        }
        process::exit(1);
    }
}

#[derive(Debug, PartialEq, Eq)]
enum TestResult {
    Ok,
    Failed,
    TimedOut,
    Invalid,
}
