use std::process;

const HELP: &str = include_str!("help.txt");
const BUILD_HELP: &str = include_str!("build_help.txt");
const CARGO_BOOTIMAGE_HELP: &str = include_str!("cargo_bootimage_help.txt");
const RUN_HELP: &str = include_str!("run_help.txt");
const RUNNER_HELP: &str = include_str!("runner_help.txt");
const TEST_HELP: &str = include_str!("test_help.txt");

pub(crate) fn help() {
    print!("{}", HELP);
}

pub(crate) fn build_help() {
    print!("{}", BUILD_HELP);
}

pub(crate) fn cargo_bootimage_help() {
    print!("{}", CARGO_BOOTIMAGE_HELP);
}

pub(crate) fn run_help() {
    print!("{}", RUN_HELP);
}

pub(crate) fn runner_help() {
    print!("{}", RUNNER_HELP);
}

pub(crate) fn test_help() {
    print!("{}", TEST_HELP);
}

pub(crate) fn no_subcommand() -> ! {
    println!("Please invoke `bootimage` with a subcommand (e.g. `bootimage build`).");
    println!();
    println!("See `bootimage --help` for more information.");
    process::exit(1);
}
