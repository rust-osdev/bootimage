use crate::{args::Args, builder::Builder, config, subcommand::build, ErrorMessage};
use rayon::prelude::*;
use std::{fs, io, io::Write, process, time::Duration};
use wait_timeout::ChildExt;

pub(crate) fn test(mut args: Args) -> Result<(), ErrorMessage> {
    let builder = Builder::new(args.manifest_path().clone())?;
    let config = config::read_config(builder.kernel_manifest_path())?;
    args.apply_default_target(&config, builder.kernel_root());

    let test_args = args.clone();

    let kernel_package = builder
        .kernel_package()
        .map_err(|key| format!("Kernel package not found it cargo metadata (`{}`)", key))?;
    let test_target_iter = kernel_package
        .targets
        .iter()
        .filter(|t| t.kind == ["bin"] && t.name.starts_with("test-"));

    let mut test_targets = Vec::new();
    for target in test_target_iter {
        println!("BUILD: {}", target.name);

        let mut target_args = test_args.clone();
        target_args.set_bin_name(target.name.clone());
        let executables = build::build_impl(&builder, &mut target_args, true)
            .expect(&format!("Failed to build test: {}", target.name));
        let test_bin_path = executables
            .first()
            .ok_or("no test executable built")?
            .to_owned();
        if executables.len() > 1 {
            Err("more than one test executables built")?;
        }

        test_targets.push((target, test_bin_path));
    }

    let tests = test_targets
        .par_iter()
        .map(|(target, test_path)| {
            println!("RUN: {}", target.name);

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
                .map_err(|e| format!("Failed to launch QEMU: {:?}\n{}", command, e))?;
            let timeout = Duration::from_secs(config.test_timeout.into());
            match child
                .wait_timeout(timeout)
                .map_err(|e| format!("Failed to wait with timeout: {}", e))?
            {
                None => {
                    child
                        .kill()
                        .map_err(|e| format!("Failed to kill QEMU: {}", e))?;
                    child
                        .wait()
                        .map_err(|e| format!("Failed to wait for QEMU process: {}", e))?;
                    test_result = TestResult::TimedOut;
                    writeln!(io::stderr(), "Timed Out")?;
                }
                Some(exit_status) => {
                    let output = fs::read_to_string(&output_file).map_err(|e| {
                        format!("Failed to read test output file {}: {}", output_file, e)
                    })?;
                    test_result = handle_exit_status(exit_status, &output, &target.name)?;
                }
            }

            Ok((target.name.clone(), test_result))
        })
        .collect::<Result<Vec<(String, TestResult)>, ErrorMessage>>()?;

    println!("");
    if tests.iter().all(|t| t.1 == TestResult::Ok) {
        println!("All tests succeeded.");
        Ok(())
    } else {
        writeln!(io::stderr(), "The following tests failed:")?;
        for test in tests.iter().filter(|t| t.1 != TestResult::Ok) {
            writeln!(io::stderr(), "    {}: {:?}", test.0, test.1)?;
        }
        Err("Some tests failed".into())
    }
}

fn handle_exit_status(
    exit_status: process::ExitStatus,
    output: &str,
    target_name: &str,
) -> Result<TestResult, ErrorMessage> {
    match exit_status.code() {
        None => {
            writeln!(io::stderr(), "FAIL: No Exit Code.")?;
            for line in output.lines() {
                writeln!(io::stderr(), "    {}", line)?;
            }
            Ok(TestResult::Invalid)
        }
        Some(code) => {
            match code {
                // 0 << 1 | 1
                1 => {
                    if output.starts_with("ok\n") {
                        println!("OK: {}", target_name);
                        Ok(TestResult::Ok)
                    } else if output.starts_with("failed\n") {
                        writeln!(io::stderr(), "FAIL:")?;
                        for line in output[7..].lines() {
                            writeln!(io::stderr(), "    {}", line)?;
                        }
                        Ok(TestResult::Failed)
                    } else {
                        writeln!(io::stderr(), "FAIL: Invalid Output:")?;
                        for line in output.lines() {
                            writeln!(io::stderr(), "    {}", line)?;
                        }
                        Ok(TestResult::Invalid)
                    }
                }

                // 2 << 1 | 1
                5 => {
                    println!("OK: {}", target_name);
                    Ok(TestResult::Ok)
                }

                // 3 << 1 | 1
                7 => {
                    let fail_index = output.find("failed\n");
                    if fail_index.is_some() {
                        writeln!(io::stderr(), "FAIL:")?;
                        let fail_output = output.split_at(fail_index.unwrap()).1;
                        for line in fail_output[7..].lines() {
                            writeln!(io::stderr(), "    {}", line)?;
                        }
                    } else {
                        writeln!(io::stderr(), "FAIL: {}", target_name)?;
                    }
                    Ok(TestResult::Failed)
                }

                _ => {
                    writeln!(io::stderr(), "FAIL: Invalid Exit Code {}:", code)?;
                    for line in output.lines() {
                        writeln!(io::stderr(), "    {}", line)?;
                    }
                    Ok(TestResult::Invalid)
                }
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum TestResult {
    Ok,
    Failed,
    TimedOut,
    Invalid,
}
