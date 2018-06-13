extern crate byteorder;
extern crate cargo_metadata;
extern crate tempdir;
extern crate toml;
extern crate xmas_elf;
extern crate wait_timeout;
#[macro_use]
extern crate failure;

use std::{io, process};
use args::Args;

mod args;
mod config;
mod build;
mod test;
mod help;

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
    use std::io::Write;
    if let Err(err) = run() {
        writeln!(io::stderr(), "Error: {:?}", err).unwrap();
        process::exit(1);
    }
}

fn run() -> Result<(), failure::Error> {
    let command = args::parse_args();
    match command {
        Command::NoSubcommand => help::no_subcommand(),
        Command::Build(args) => build::build(args),
        Command::Run(args) => build::run(args),
        Command::Test(args) => test::test(args),
        Command::Help => Ok(help::help()),
        Command::BuildHelp => Ok(help::build_help()),
        Command::RunHelp => Ok(help::run_help()),
        Command::TestHelp => Ok(help::test_help()),
        Command::Version => Ok(println!("bootimage {}", env!("CARGO_PKG_VERSION"))),
    }
}
