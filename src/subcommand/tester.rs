use crate::{args::TesterArgs, builder::Builder, config, ErrorString};
use std::{
    fs,
    io::{self, Write},
    path::Path,
    process,
    process::Command,
    time::Duration,
};
use wait_timeout::ChildExt;

pub(crate) fn tester(args: TesterArgs) -> Result<(), ErrorString> {
    let builder = Builder::new(None)?;
    let config = config::read_config(builder.kernel_manifest_path().to_owned())?;

    let test_name = args
        .test_path
        .file_stem()
        .expect("no file stem")
        .to_os_string()
        .into_string()
        .expect("test name invalid utf8");

    let kernel_manifest_path = locate_cargo_manifest::locate_manifest().unwrap_or(
        Path::new("Cargo.toml")
            .canonicalize()
            .expect("failed to canonicalize manifest path"),
    );
    let kernel_root_path = kernel_manifest_path
        .parent()
        .expect("kernel manifest path has no parent");
    let kernel_manifest_content =
        fs::read_to_string(&kernel_manifest_path).expect("failed to read kernel manifest");
    let kernel_manifest: toml::Value = kernel_manifest_content
        .parse()
        .expect("failed to parse Cargo.toml");

    let kernel_name = kernel_manifest
        .get("package")
        .and_then(|p| p.get("name"))
        .expect("no package.name found in Cargo.toml")
        .as_str()
        .expect("package name must be a string");
    let dependency_table = {
        let mut table = toml::value::Table::new();
        let mut dependencies = kernel_manifest
            .get("dependencies")
            .map(|v| {
                v.as_table()
                    .expect("`dependencies` must be a table in Cargo.toml")
                    .clone()
            })
            .unwrap_or(toml::value::Table::new());
        dependencies.insert(
            kernel_name.to_owned(),
            toml::from_str(&format!(r#"path = "{}""#, kernel_root_path.display())).unwrap(),
        );
        for (key, entry) in kernel_manifest
            .get("dev-dependencies")
            .map(|v| {
                v.as_table()
                    .expect("`dev-dependencies` must be a table in Cargo.toml")
                    .clone()
            })
            .unwrap_or(toml::value::Table::new())
        {
            dependencies.insert(key, entry);
        }
        table.insert("dependencies".to_owned(), toml::Value::Table(dependencies));
        toml::Value::Table(table)
    };
    let kernel_target_dir = Path::new("target")
        .canonicalize()
        .expect("failed to canonicalize target dir"); // TODO
    let kernel_target_json = Path::new("x86_64-blog_os.json")
        .canonicalize()
        .expect("failed to canonicalize target.json"); // TODO
    let kernel_target_json_triple = kernel_target_json
        .file_stem()
        .expect("kernel target json has no valid file stem");

    let out_dir = kernel_target_dir.join("integration-tests").join(&test_name);
    fs::create_dir_all(&out_dir).expect("failed to create out dir");

    let manifest_path = out_dir.join("Cargo.toml");
    let manifest_content = format!(
        r#"
[package]
authors = ["Bootimage Tester <bootimage@example.com>"]
name = "{test_name}"
version = "0.0.0"
edition = "2018"

[[bin]]
name = "{test_name}"
path = "{test_path}"

{dependency_table}
"#,
        test_name = test_name,
        test_path = args.test_path.display(),
        dependency_table = dependency_table
    );

    fs::write(&manifest_path, manifest_content)?;

    let cargo = std::env::var("CARGO").unwrap_or("cargo".to_owned());
    let mut cmd = Command::new(cargo);
    cmd.arg("xbuild");
    cmd.arg("--manifest-path").arg(&manifest_path);
    cmd.arg("--target-dir").arg(&kernel_target_dir);
    cmd.env("SYSROOT_DIR", &kernel_target_dir.join("sysroot")); // for cargo-xbuild
    cmd.arg("--target").arg(&kernel_target_json); // TODO remove when default targets are canonicalized properly
    let output = cmd.output().expect("failed to run cargo xbuild");
    if !output.status.success() {
        io::stderr()
            .write_all(&output.stderr)
            .expect("failed to write to stderr");
        process::exit(1);
    }

    let executable = kernel_target_dir
        .join(&kernel_target_json_triple)
        .join("debug")
        .join(&test_name);
    let bootimage_bin_path = out_dir.join(format!("bootimage-{}.bin", test_name));

    builder.create_bootimage(&executable, &bootimage_bin_path, true)?;

    let run_cmd = args.run_command.clone().unwrap_or(
        [
            "qemu-system-x86_64",
            "-drive",
            "format=raw,file={bootimage}",
            "-device",
            "isa-debug-exit,iobase=0xf4,iosize=0x04",
            "-display",
            "none",
            "-serial",
            "file:{output_file}",
        ]
        .into_iter()
        .map(|&s| String::from(s))
        .collect(),
    );

    let output_file = out_dir.join(format!("output-{}.txt", test_name));

    let mut command = process::Command::new(&run_cmd[0]);
    for arg in &run_cmd[1..] {
        command.arg(
            arg.replace("{bootimage}", &format!("{}", bootimage_bin_path.display()))
                .replace("{output_file}", &format!("{}", output_file.display())),
        );
    }
    command.stderr(process::Stdio::null());
    let mut child = command
        .spawn()
        .map_err(|e| format!("Failed to launch QEMU: {:?}\n{}", command, e))?;
    let timeout = Duration::from_secs(config.test_timeout.into());
    let (exit_status, output) = match child
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
            Err("Timed Out")
        }
        Some(exit_status) => {
            let output = fs::read_to_string(&output_file).map_err(|e| {
                format!(
                    "Failed to read test output file {}: {}",
                    output_file.display(),
                    e
                )
            })?;
            Ok((exit_status, output))
        }
    }?;

    match exit_status.code() {
        None => Err("No QEMU Exit Code")?,
        Some(5) => {} // 2 << 1 | 1
        Some(7) => {
            // 3 << 1 | 1
            let fail_index = output.rfind("bootimage:stderr\n");
            if let Some(index) = fail_index {
                Err(format!("Test Failed:\n{}", &output[index..]))?
            } else {
                Err("Test Failed")?
            }
        }
        Some(c) => Err(format!("Test returned with unexpected exit code {}", c))?,
    }
    Ok(())
}
