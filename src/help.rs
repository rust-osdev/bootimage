use Error;
use std::process;

const HELP: &str = include_str!("help.txt");

pub(crate) fn help(explicitly_invoked: bool) -> Result<(), Error> {
    print!("{}", HELP);
    process::exit(if explicitly_invoked { 0 } else { 1 });
}
