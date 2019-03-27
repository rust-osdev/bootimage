use args::{Args, RunnerArgs};
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
    Runner(RunnerArgs),
    Help,
    BuildHelp,
    RunHelp,
    TestHelp,
    RunnerHelp,
    Version,
}

pub fn main() {
    if let Err(err) = run() {
        eprintln!("Error: {}", err.message);
        process::exit(err.exit_code);
    }
}

fn run() -> Result<(), ErrorString> {
    let command = args::parse_args();
    match command {
        Command::Build(args) => subcommand::build::build(args),
        Command::Run(args) => subcommand::run::run(args),
        Command::Test(args) => subcommand::test::test(args),
        Command::Runner(args) => subcommand::runner::runner(args),
        Command::NoSubcommand => help::no_subcommand(),
        Command::Help => Ok(help::help()),
        Command::BuildHelp => Ok(help::build_help()),
        Command::RunHelp => Ok(help::run_help()),
        Command::TestHelp => Ok(help::test_help()),
        Command::RunnerHelp => unimplemented!(),
        Command::Version => Ok(println!("bootimage {}", env!("CARGO_PKG_VERSION"))),
    }
}

struct ErrorString {
    pub message: Box<dyn fmt::Display + Send>,
    pub exit_code: i32,
}

impl fmt::Debug for ErrorString {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.message.fmt(f)
    }
}

impl<T> From<T> for ErrorString
where
    T: fmt::Display + Send + 'static,
{
    fn from(err: T) -> Self {
        ErrorString {
            message: Box::new(err),
            exit_code: 1,
        }
    }
}
