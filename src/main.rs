#[macro_use]
extern crate failure;

use args::Args;
use std::{fmt, process};

mod args;
mod config;
mod help;

mod subcommand;

enum Command {
    NoSubcommand,
    Build(Args),
    Run(Args),
    Test(Args),
    Help,
    BuildHelp,
    RunHelp,
    TestHelp,
    Version,
}

pub fn main() {
    if let Err(err) = run() {
        eprintln!("Error: {}", err.display());
        process::exit(1);
    }
}

fn run() -> Result<(), ErrorString> {
    let command = args::parse_args();
    match command {
        Command::Build(args) => subcommand::build::build(args),
        Command::Run(args) => subcommand::run::run(args),
        Command::Test(args) => subcommand::test::test(args),
        Command::NoSubcommand => help::no_subcommand(),
        Command::Help => Ok(help::help()),
        Command::BuildHelp => Ok(help::build_help()),
        Command::RunHelp => Ok(help::run_help()),
        Command::TestHelp => Ok(help::test_help()),
        Command::Version => Ok(println!("bootimage {}", env!("CARGO_PKG_VERSION"))),
    }
}

struct ErrorString(Box<dyn fmt::Display>);

impl ErrorString {
    fn display(&self) -> &dyn fmt::Display {
        &self.0
    }
}

impl fmt::Debug for ErrorString {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.display().fmt(f)
    }
}

impl<T> From<T> for ErrorString
where
    T: fmt::Display + 'static,
{
    fn from(err: T) -> Self {
        ErrorString(Box::new(err))
    }
}
