//! Provides functions to create a bootable OS image from a kernel binary.
//!
//! This crate is mainly built as a binary tool. Run `cargo install bootimage` to install it.

#![warn(missing_docs)]

use args::{Args, RunnerArgs};
use std::{fmt, process};

mod args;
pub mod builder;
pub mod config;
mod help;

mod subcommand;

enum Command {
    NoSubcommand,
    Build(Args),
    Run(Args),
    Test(Args),
    Runner(RunnerArgs),
    Help,
    BuildHelp,
    RunHelp,
    TestHelp,
    CargoBootimageHelp,
    RunnerHelp,
    Version,
}

/// The entry point for the binaries.
///
/// We support two binaries, `bootimage` and `cargo-bootimage` that both just
/// call into this function.
///
/// This function is just a small wrapper around [`run`] that prints error messages
/// and exits with the correct exit code.
pub fn lib_main() {
    match run() {
        Err(err) => {
            eprintln!("Error: {}", err.message);
            process::exit(1);
        }
        Ok(Some(exit_code)) => {
            process::exit(exit_code);
        }
        Ok(None) => {}
    }
}

/// Run the invoked command.
///
/// This function parses the arguments and invokes the chosen subcommand.
///
/// On success, it optionally returns an exit code. This feature is used by the
/// `run` and `runner` subcommand to pass through the exit code of the invoked
/// run command.
pub fn run() -> Result<Option<i32>, ErrorMessage> {
    let command = args::parse_args()?;
    let none = |()| None;
    match command {
        Command::Build(args) => subcommand::build::build(args).map(none),
        Command::Run(args) => subcommand::run::run(args).map(Some),
        Command::Test(args) => subcommand::test::test(args).map(none),
        Command::Runner(args) => subcommand::runner::runner(args).map(Some),
        Command::NoSubcommand => help::no_subcommand(),
        Command::Help => Ok(help::help()).map(none),
        Command::BuildHelp => Ok(help::build_help()).map(none),
        Command::RunHelp => Ok(help::run_help()).map(none),
        Command::TestHelp => Ok(help::test_help()).map(none),
        Command::Version => Ok(println!("bootimage {}", env!("CARGO_PKG_VERSION"))).map(none),
        Command::RunnerHelp | Command::CargoBootimageHelp => unimplemented!(),
    }
}

/// A simple error message that can be created from every type that implements `fmt::Display`.
///
/// We use this error type for the CLI interface, where text based, human readable error messages
/// make sense. For the library part of this crate, we use custom error enums.
pub struct ErrorMessage {
    /// The actual error message
    pub message: Box<dyn fmt::Display + Send>,
}

impl fmt::Debug for ErrorMessage {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.message.fmt(f)
    }
}

impl<T> From<T> for ErrorMessage
where
    T: fmt::Display + Send + 'static,
{
    fn from(err: T) -> Self {
        ErrorMessage {
            message: Box::new(err),
        }
    }
}
