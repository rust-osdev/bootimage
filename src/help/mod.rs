const HELP: &str = include_str!("help.txt");
const CARGO_BOOTIMAGE_HELP: &str = include_str!("cargo_bootimage_help.txt");
const RUNNER_HELP: &str = include_str!("runner_help.txt");

/// Prints a general help text.
pub fn print_help() {
    print!("{}", HELP);
}

/// Prints the help for the `cargo bootimage` command.
pub fn print_cargo_bootimage_help() {
    print!("{}", CARGO_BOOTIMAGE_HELP);
}
/// Prints the help for the `bootimage runner` command.
pub fn print_runner_help() {
    print!("{}", RUNNER_HELP);
}

/// Prints the version of this crate.
pub fn print_version() {
    println!("bootimage {}", env!("CARGO_PKG_VERSION"));
}
