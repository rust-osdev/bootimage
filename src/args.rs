use std::{env, mem};
use std::path::PathBuf;
use Command;

pub(crate) fn parse_args() -> Command {
    let mut args = env::args().skip(1);
    let first = args.next();
    match first.as_ref().map(|s| s.as_str()) {
        Some("build") => parse_build_args(args),
        Some("run") => match parse_build_args(args) {
            Command::Build(args) => Command::Run(args),
            Command::BuildHelp => Command::RunHelp,
            cmd => cmd,
        },
        Some("--help") | Some("-h") => Command::Help,
        Some("--version") => Command::Version,
        _ => Command::NoSubcommand,
    }
}

fn parse_build_args<A>(args: A) -> Command
where
    A: Iterator<Item = String>,
{
    let mut manifest_path: Option<PathBuf> = None;
    let mut target: Option<String> = None;
    let mut release: Option<bool> = None;
    let mut update_bootloader: Option<bool> = None;
    let mut cargo_args = Vec::new();
    let mut run_args = Vec::new();
    let mut run_args_started = false;
    {
        fn set<T>(arg: &mut Option<T>, value: Option<T>) {
            let previous = mem::replace(arg, value);
            assert!(
                previous.is_none(),
                "multiple arguments of same type provided"
            )
        };

        let mut arg_iter = args.into_iter();
        while let Some(arg) = arg_iter.next() {
            if run_args_started {
                run_args.push(arg);
                continue;
            }
            match arg.as_ref() {
                "--help" | "-h" => {
                    return Command::BuildHelp;
                }
                "--version" => {
                    return Command::Version;
                }
                "--target" => {
                    let next = arg_iter.next();
                    set(&mut target, next.clone());
                    cargo_args.push(arg);
                    if let Some(next) = next {
                        cargo_args.push(next);
                    }
                }
                _ if arg.starts_with("--target=") => {
                    set(
                        &mut target,
                        Some(String::from(arg.trim_left_matches("--target="))),
                    );
                    cargo_args.push(arg);
                }
                "--manifest-path" => {
                    let next = arg_iter.next();
                    set(&mut manifest_path, next.as_ref().map(|p| PathBuf::from(&p)));
                    cargo_args.push(arg);
                    if let Some(next) = next {
                        cargo_args.push(next);
                    }
                }
                _ if arg.starts_with("--manifest-path=") => {
                    let path = PathBuf::from(arg.trim_left_matches("--manifest-path="));
                    set(&mut manifest_path, Some(path));
                    cargo_args.push(arg);
                }
                "--release" => {
                    set(&mut release, Some(true));
                    cargo_args.push(arg);
                }
                "--update-bootloader" => {
                    set(&mut update_bootloader, Some(true));
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

    Command::Build(Args {
        cargo_args,
        run_args,
        target,
        manifest_path,
        release: release.unwrap_or(false),
        update_bootloader: update_bootloader.unwrap_or(false),
    })
}

pub struct Args {
    /// All arguments that are passed to cargo.
    pub cargo_args: Vec<String>,
    /// All arguments that are passed to the runner.
    pub run_args: Vec<String>,
    /// The manifest path (also present in `cargo_args`).
    manifest_path: Option<PathBuf>,
    /// The target triple (also present in `cargo_args`).
    target: Option<String>,
    /// The release flag (also present in `cargo_args`).
    release: bool,
    /// Whether the bootloader should be updated (not present in `cargo_args`).
    update_bootloader: bool,
}

impl Args {
    pub fn manifest_path(&self) -> &Option<PathBuf> {
        &self.manifest_path
    }

    pub fn target(&self) -> &Option<String> {
        &self.target
    }

    pub fn release(&self) -> bool {
        self.release
    }

    pub fn update_bootloader(&self) -> bool {
        self.update_bootloader
    }

    pub fn set_target(&mut self, target: String) {
        assert!(self.target.is_none());
        self.target = Some(target.clone());
        self.cargo_args.push("--target".into());
        self.cargo_args.push(target);
    }
}
