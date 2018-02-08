#![feature(termination_trait)]
#![feature(try_from)]

extern crate byteorder;
extern crate xmas_elf;
extern crate lapp;
extern crate toml;
extern crate curl;

use std::io;
use std::io::prelude::*;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::convert::TryFrom;
use byteorder::{ByteOrder, LittleEndian};
use curl::easy::Easy;

const ARGS: &str = "
A tool for appending a x86 bootloader to a Rust kernel.

    -t,--target (default '')
    -o,--output (default 'bootimage.bin')
    -g,--git (default 'https://github.com/phil-opp/bootloader')
    --release           Compile kernel in release mode
    --build-bootloader  Build the bootloader instead of downloading it
    --only-bootloader   Only build the bootloader without appending the kernel
";

#[derive(Debug)]
struct Opt {
    release: bool,
    output: String,
    target: Option<String>,
    build_bootloader: bool,
    only_bootloader: bool,
    bootloader_git: String,
}

fn main() -> io::Result<()> {
    let args = lapp::parse_args(ARGS);
    let target = args.get_string("target");
    let target = if target == "" { None } else { Some(target) };
    let opt = Opt {
        release: args.get_bool("release"),
        output: args.get_string("output"),
        target: target,
        build_bootloader: args.get_bool("build-bootloader"),
        only_bootloader: args.get_bool("only-bootloader"),
        bootloader_git: args.get_string("git"),
    };
    let pwd = std::env::current_dir()?;

    if opt.only_bootloader {
        build_bootloader(&opt, &pwd)?;
        return Ok(());
    }

    let (mut kernel, out_dir) = build_kernel(&opt, &pwd)?;

    let kernel_size = kernel.metadata()?.len();
    let kernel_size = u32::try_from(kernel_size).expect("kernel too big");
    let mut kernel_interface_block = [0u8; 512];
    LittleEndian::write_u32(&mut kernel_interface_block[0..4], kernel_size);

    let bootloader_path = build_bootloader(&opt, &out_dir)?;

    let mut bootloader_data = Vec::new();
    File::open(&bootloader_path)?.read_to_end(&mut bootloader_data)?;

    println!("Creating disk image at {}", opt.output);
    let mut output = File::create(&opt.output)?;
    output.write_all(&bootloader_data)?;
    output.write_all(&kernel_interface_block)?;

    // write out kernel elf file
    let mut buffer = [0u8; 1024];
    loop {
        let (n, interrupted) = match kernel.read(&mut buffer) {
            Ok(0) => break,
            Ok(n) => (n, false),
            Err(ref e) if e.kind() == io::ErrorKind::Interrupted => (0, true),
            Err(e) => return Err(e),
        };
        if !interrupted {
            output.write_all(&buffer[..n])?
        }
    }

    Ok(())
}

fn build_kernel(opt: &Opt, pwd: &Path) -> io::Result<(File, PathBuf)> {
    let mut crate_root = pwd.to_path_buf();
    // try to find Cargo.toml to find root dir and read crate name
    let crate_name = loop {
        let mut cargo_toml_path = crate_root.clone();
        cargo_toml_path.push("Cargo.toml");
        match File::open(cargo_toml_path) {
            Err(ref e) if e.kind() == io::ErrorKind::NotFound => {
                if !crate_root.pop() {
                    panic!("Cargo.toml not found");
                }
            }
            Err(e) => return Err(e),
            Ok(mut file) => {
                // Cargo.toml found! pwd is already the root path, so we just need
                // to set the crate name
                let mut content = String::new();
                file.read_to_string(&mut content)?;
                let toml = content.parse::<toml::Value>().ok();
                if let Some(toml) = toml {
                    if let Some(crate_name) = toml.get("package")
                        .and_then(|package| package.get("name")).and_then(|n| n.as_str())
                    {
                        break String::from(crate_name);
                    }
                }
                panic!("Cargo.toml invalid");
            }
        }
    };

    let kernel_target = opt.target.as_ref().map(String::as_str);

    // compile kernel
    let exit_status = run_xargo_build(&pwd, kernel_target, opt.release)?;
    if !exit_status.success() { std::process::exit(1) }

    let mut out_dir = pwd.to_path_buf();
    out_dir.push("target");
    if let Some(target) = kernel_target {
        out_dir.push(target);
    }
    if opt.release {
        out_dir.push("release");
    } else {
        out_dir.push("debug");
    }

    let mut kernel_path = out_dir.clone();
    kernel_path.push(crate_name);
    let kernel = File::open(kernel_path)?;
    Ok((kernel, out_dir))
}

fn build_bootloader(opt: &Opt, out_dir: &Path) -> io::Result<PathBuf> {
    let bootloader_target = "x86_64-bootloader";

    let mut bootloader_path = out_dir.to_path_buf();
    bootloader_path.push("bootloader.bin");

    if !bootloader_path.exists() {
        if opt.build_bootloader {
            let mut bootloader_dir = out_dir.to_path_buf();
            bootloader_dir.push("bootloader");

            if !bootloader_dir.exists() {
                // download bootloader from github repo
                let url = &opt.bootloader_git;
                println!("Cloning bootloader from {}", url);
                let mut command = Command::new("git");
                command.current_dir(out_dir);
                command.arg("clone");
                command.arg(url);
                if !command.status()?.success() {
                    write!(std::io::stderr(), "Error: git clone failed")?;
                    std::process::exit(1);
                }
            }

            // compile bootloader
            println!("Compiling bootloader...");
            let exit_status = run_xargo_build(&bootloader_dir, Some(bootloader_target), true)?;
            if !exit_status.success() { std::process::exit(1) }

            let mut bootloader_elf_path = bootloader_dir.to_path_buf();
            bootloader_elf_path.push("target");
            bootloader_elf_path.push(bootloader_target);
            bootloader_elf_path.push("release/bootloader");

            let mut bootloader_elf_bytes = Vec::new();
            File::open(bootloader_elf_path)?.read_to_end(&mut bootloader_elf_bytes)?;

            // copy bootloader section of ELF file to bootloader_path
            let elf_file = xmas_elf::ElfFile::new(&bootloader_elf_bytes).unwrap();
            xmas_elf::header::sanity_check(&elf_file).unwrap();
            let bootloader_section = elf_file.find_section_by_name(".bootloader")
                .expect("bootloader must have a .bootloader section");

            File::create(&bootloader_path)?.write_all(bootloader_section.raw_data(&elf_file))?;
        } else {
            println!("Downloading bootloader...");
            let mut bootloader = File::create(&bootloader_path)?;
            // download bootloader release
            let url = "https://github.com/phil-opp/bootloader/releases/download/latest/bootimage.bin";
            let mut handle = Easy::new();
            handle.url(url)?;
            handle.follow_location(true)?;
            let mut transfer = handle.transfer();
            transfer.write_function(|data| {
                bootloader.write_all(data).expect("Error writing bootloader to file");
                Ok(data.len())
            })?;
            transfer.perform().expect("Downloading bootloader failed");
        }
    }
    Ok(bootloader_path)
}

fn run_xargo_build(pwd: &Path, target: Option<&str>, release: bool) -> io::Result<std::process::ExitStatus> {
    let mut command = Command::new("xargo");
    command.current_dir(pwd).env("RUST_TARGET_PATH", pwd);
    command.arg("build");
    if let Some(target) = target {
        command.arg("--target").arg(target);
    }
    if release {
        command.arg("--release");
    }
    command.status()
}
