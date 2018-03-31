extern crate byteorder;
extern crate xmas_elf;
extern crate toml;
extern crate cargo_metadata;

use std::{io, process};
use args::Args;

mod args;
mod config;
mod build;
mod help;

enum Command {
    Build(Args),
    Help(bool),
    Version,
}

pub fn main() {
    use std::io::Write;
    if let Err(err) = run() {
        writeln!(io::stderr(), "Error: {:?}", err).unwrap();
        process::exit(1);
    }
}

#[derive(Debug)]
pub enum Error {
    Config(String),
    Bootloader(String, io::Error),
    Io(io::Error),
    Toml(toml::de::Error),
    CargoMetadata(cargo_metadata::Error),
}

impl From<io::Error> for Error {
    fn from(other: io::Error) -> Self {
        Error::Io(other)
    }
}

impl From<toml::de::Error> for Error {
    fn from(other: toml::de::Error) -> Self {
        Error::Toml(other)
    }
}

impl From<cargo_metadata::Error> for Error {
    fn from(other: cargo_metadata::Error) -> Self {
        Error::CargoMetadata(other)
    }
}

fn run() -> Result<(), Error> {
    let command = args::parse_args();
    match command {
        Command::Build(args) => build::build(args),
        Command::Help(explicitly_invoked) => help::help(explicitly_invoked),
        Command::Version => Ok(println!("bootimage {}", env!("CARGO_PKG_VERSION"))),
    }

}
