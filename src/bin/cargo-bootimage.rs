use std::process;

pub fn main() {
    if let Err(err) = bootimage::run() {
        eprintln!("Error: {}", err.message);
        process::exit(err.exit_code);
    }
}
