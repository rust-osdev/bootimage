use std::{env, mem};
use std::path::PathBuf;
use Command;

pub(crate) fn parse_args() -> Command {
    let mut args = env::args().skip(1).peekable();

    match args.peek().map(|s| s.as_str()) {
        Some("build") => parse_build_args(args.skip(1).collect()),
        Some("--help") | Some("-h") => Command::Help,
        Some("--version") => Command::Version,
        _ => Command::NoSubcommand,
    }
}

fn parse_build_args(mut args: Vec<String>) -> Command {
    let mut manifest_path: Option<PathBuf> = None;
    let mut target: Option<String> = None;
    let mut release: Option<bool> = None;
    {
        fn set<T>(arg: &mut Option<T>, value: Option<T>) {
            let previous = mem::replace(arg, value);
            assert!(previous.is_none(), "multiple arguments of same type provided")
        };

        let mut arg_iter = args.iter_mut();
        while let Some(arg) = arg_iter.next() {
            match arg.as_ref() {
                "--help" | "-h" => {
                    return Command::Help;
                }
                "--version" => {
                    return Command::Version;
                }
                "--target" => {
                    set(&mut target, arg_iter.next().map(|s| s.clone()));
                }
                _ if arg.starts_with("--target=") => {
                    set(&mut target, Some(String::from(arg.trim_left_matches("--target="))));
                }
                "--manifest-path" => {
                    set(&mut manifest_path, arg_iter.next().map(|p| PathBuf::from(&p)));
                }
                _ if arg.starts_with("--manifest-path=") => {
                    let path = PathBuf::from(arg.trim_left_matches("--manifest-path="));
                    set(&mut manifest_path, Some(path));
                }
                "--release" => set(&mut release, Some(true)),
                _ => {},
            }
        }
    }

    Command::Build(Args {
        all_cargo: args,
        target,
        manifest_path,
        release: release.unwrap_or(false),
    })
}

pub struct Args {
    /// All arguments that are passed to cargo.
    pub all_cargo: Vec<String>,
    /// The manifest path (also present in `all_cargo`).
    manifest_path: Option<PathBuf>,
    /// The target triple (also present in `all_cargo`).
    target: Option<String>,
    /// The release flag (also present in `all_cargo`).
    release: bool,
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


    pub fn set_target(&mut self, target: String) {
        assert!(self.target.is_none());
        self.target = Some(target.clone());
        self.all_cargo.push("--target".into());
        self.all_cargo.push(target);
    }
}
