use args::{Args, RunnerArgs, TesterArgs};
use std::{fmt, process};

pub mod args;
pub mod builder;
pub mod config;
pub mod help;

mod subcommand;

enum Command {
    NoSubcommand,
    Build(Args),
    Run(Args),
    Test(Args),
    Runner(RunnerArgs),
    Tester(TesterArgs),
    Help,
    BuildHelp,
    RunHelp,
    TestHelp,
    CargoBootimageHelp,
    RunnerHelp,
    TesterHelp,
    Version,
}

pub fn lib_main() {
    match run() {
        Err(err) => {
            eprintln!("Error: {}", err.message);
            process::exit(err.exit_code);
        }
        Ok(Some(exit_code)) => {
            process::exit(exit_code);
        }
        Ok(None) => {}
    }
}

pub fn run() -> Result<Option<i32>, ErrorString> {
    let command = args::parse_args()?;
    let none = |()| None;
    match command {
        Command::Build(args) => subcommand::build::build(args).map(none),
        Command::Run(args) => subcommand::run::run(args).map(Some),
        Command::Test(args) => subcommand::test::test(args).map(none),
        Command::Runner(args) => subcommand::runner::runner(args).map(Some),
        Command::Tester(args) => subcommand::tester::tester(args).map(none),
        Command::NoSubcommand => help::no_subcommand(),
        Command::Help => Ok(help::help()).map(none),
        Command::BuildHelp => Ok(help::build_help()).map(none),
        Command::RunHelp => Ok(help::run_help()).map(none),
        Command::TestHelp => Ok(help::test_help()).map(none),
        Command::Version => Ok(println!("bootimage {}", env!("CARGO_PKG_VERSION"))).map(none),
        Command::RunnerHelp | Command::TesterHelp | Command::CargoBootimageHelp => unimplemented!(),
    }
}

pub struct ErrorString {
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
