//! Parses command line arguments.

use crate::{config::Config, Command, ErrorMessage};
use std::path::{Path, PathBuf};
use std::{env, mem};

pub(crate) fn parse_args() -> Result<Command, ErrorMessage> {
    let mut args = env::args();
    let executable_name = args.next().ok_or("no first argument (executable name)")?;
    let first = args.next();
    match first.as_ref().map(|s| s.as_str()) {
        Some("build") => parse_build_args(args),
        Some("bootimage") if executable_name.ends_with("cargo-bootimage") => parse_build_args(args)
            .map(|cmd| match cmd {
                Command::BuildHelp => Command::CargoBootimageHelp,
                cmd => cmd,
            }),
        Some("run") => parse_build_args(args).map(|cmd| match cmd {
            Command::Build(args) => Command::Run(args),
            Command::BuildHelp => Command::RunHelp,
            cmd => cmd,
        }),
        Some("test") => parse_build_args(args).map(|cmd| match cmd {
            Command::Build(args) => {
                assert_eq!(
                    args.bin_name, None,
                    "No `--bin` argument allowed for `bootimage test`"
                );
                Command::Test(args)
            }
            Command::BuildHelp => Command::TestHelp,
            cmd => cmd,
        }),
        Some("runner") => parse_runner_args(args),
        Some("--help") | Some("-h") => Ok(Command::Help),
        Some("--version") => Ok(Command::Version),
        _ => Ok(Command::NoSubcommand),
    }
}

fn parse_build_args<A>(args: A) -> Result<Command, ErrorMessage>
where
    A: Iterator<Item = String>,
{
    let mut manifest_path: Option<PathBuf> = None;
    let mut bin_name: Option<String> = None;
    let mut target: Option<String> = None;
    let mut release: Option<bool> = None;
    let mut cargo_args = Vec::new();
    let mut run_args = Vec::new();
    let mut run_args_started = false;
    let mut quiet = false;
    {
        fn set<T>(arg: &mut Option<T>, value: Option<T>) -> Result<(), ErrorMessage> {
            let previous = mem::replace(arg, value);
            if previous.is_some() {
                Err("multiple arguments of same type provided")?
            }
            Ok(())
        };

        let mut arg_iter = args.into_iter();
        while let Some(arg) = arg_iter.next() {
            if run_args_started {
                run_args.push(arg);
                continue;
            }
            match arg.as_ref() {
                "--help" | "-h" => {
                    return Ok(Command::BuildHelp);
                }
                "--version" => {
                    return Ok(Command::Version);
                }
                "--quiet" => {
                    quiet = true;
                }
                "--bin" => {
                    let next = arg_iter.next();
                    set(&mut bin_name, next.clone())?;
                    cargo_args.push(arg);
                    if let Some(next) = next {
                        cargo_args.push(next);
                    }
                }
                _ if arg.starts_with("--bin=") => {
                    set(
                        &mut bin_name,
                        Some(String::from(arg.trim_start_matches("--bin="))),
                    )?;
                    cargo_args.push(arg);
                }
                "--target" => {
                    let next = arg_iter.next();
                    set(&mut target, next.clone())?;
                    cargo_args.push(arg);
                    if let Some(next) = next {
                        cargo_args.push(next);
                    }
                }
                _ if arg.starts_with("--target=") => {
                    set(
                        &mut target,
                        Some(String::from(arg.trim_start_matches("--target="))),
                    )?;
                    cargo_args.push(arg);
                }
                "--manifest-path" => {
                    let next = arg_iter.next();
                    set(
                        &mut manifest_path,
                        next.as_ref().map(|p| {
                            Path::new(&p)
                                .canonicalize()
                                .expect("--manifest-path invalid")
                        }),
                    )?;
                    cargo_args.push(arg);
                    if let Some(next) = next {
                        cargo_args.push(next);
                    }
                }
                _ if arg.starts_with("--manifest-path=") => {
                    let path = Path::new(arg.trim_start_matches("--manifest-path="))
                        .canonicalize()
                        .expect("--manifest-path invalid");
                    set(&mut manifest_path, Some(path))?;
                    cargo_args.push(arg);
                }
                "--release" => {
                    set(&mut release, Some(true))?;
                    cargo_args.push(arg);
                }
                "--" => {
                    run_args_started = true;
                }
                _ => {
                    cargo_args.push(arg);
                }
            };
        }
    }

    Ok(Command::Build(Args {
        cargo_args,
        run_args,
        bin_name,
        target,
        manifest_path,
        release: release.unwrap_or(false),
        quiet,
    }))
}

#[derive(Debug, Clone)]
pub struct Args {
    /// All arguments that are passed to cargo.
    pub cargo_args: Vec<String>,
    /// All arguments that are passed to the runner.
    pub run_args: Vec<String>,
    /// Suppress any output to stdout.
    pub quiet: bool,
    /// The manifest path (also present in `cargo_args`).
    manifest_path: Option<PathBuf>,
    /// The name of the binary (passed `--bin` argument) (also present in `cargo_args`).
    bin_name: Option<String>,
    /// The target triple (also present in `cargo_args`).
    target: Option<String>,
    /// The release flag (also present in `cargo_args`).
    release: bool,
}

impl Args {
    pub fn manifest_path(&self) -> &Option<PathBuf> {
        &self.manifest_path
    }

    pub fn target(&self) -> &Option<String> {
        &self.target
    }

    pub fn set_target(&mut self, target: String) {
        assert!(self.target.is_none());
        self.target = Some(target.clone());
        self.cargo_args.push("--target".into());
        self.cargo_args.push(target);
    }

    pub fn bin_name(&self) -> Option<&str> {
        self.bin_name.as_ref().map(String::as_str)
    }

    pub fn set_bin_name(&mut self, bin_name: String) {
        assert!(self.bin_name.is_none());
        self.bin_name = Some(bin_name.clone());
        self.cargo_args.push("--bin".into());
        self.cargo_args.push(bin_name);
    }

    pub fn apply_default_target(&mut self, config: &Config, crate_root: &Path) {
        if self.target().is_none() {
            if let Some(ref target) = config.default_target {
                let canonicalized_target = crate_root.join(target);
                self.set_target(canonicalized_target.to_string_lossy().into_owned());
            }
        }
    }
}

fn parse_runner_args<A>(args: A) -> Result<Command, ErrorMessage>
where
    A: Iterator<Item = String>,
{
    let mut executable = None;
    let mut quiet = false;
    let mut runner_args = None;

    let mut arg_iter = args.into_iter().fuse();

    loop {
        if executable.is_some() {
            let args: Vec<_> = arg_iter.collect();
            if args.len() > 0 {
                runner_args = Some(args);
            }
            break;
        }
        let next = match arg_iter.next() {
            Some(next) => next,
            None => break,
        };
        match next.as_str() {
            "--help" | "-h" => {
                return Ok(Command::RunnerHelp);
            }
            "--version" => {
                return Ok(Command::Version);
            }
            "--quiet" => {
                quiet = true;
            }
            exe => {
                let path = Path::new(exe);
                let path_canonicalized = path.canonicalize().map_err(|err| {
                    format!(
                        "Failed to canonicalize executable path `{}`: {}",
                        path.display(),
                        err
                    )
                })?;
                executable = Some(path_canonicalized);
            }
        }
    }

    Ok(Command::Runner(RunnerArgs {
        executable: executable.ok_or("excepted path to kernel executable as first argument")?,
        quiet,
        runner_args,
    }))
}

#[derive(Debug, Clone)]
pub struct RunnerArgs {
    pub executable: PathBuf,
    /// Suppress any output to stdout.
    pub quiet: bool,
    pub runner_args: Option<Vec<String>>,
}
