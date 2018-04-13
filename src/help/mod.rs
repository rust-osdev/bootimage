use std::process;

const HELP: &str = include_str!("help.txt");
const BUILD_HELP: &str = include_str!("build_help.txt");

pub(crate) fn help() {
    print!("{}", HELP);
}

pub(crate) fn build_help() {
    print!("{}", BUILD_HELP);
}

pub(crate) fn no_subcommand() -> ! {
    println!("Please invoke `bootimage` with a subcommand (e.g. `bootimage build`).");
    println!();
    println!("See `bootimage --help` for more information.");
    process::exit(1);
}
