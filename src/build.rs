use std::fs::{self, File};
use std::{env, process, io};
use std::path::{Path, PathBuf};
use byteorder::{ByteOrder, LittleEndian};
use args::{self, Args};
use config::{self, Config};
use cargo_metadata::{self, Metadata as CargoMetadata, Package as CrateMetadata};
use Error;
use xmas_elf;

const BLOCK_SIZE: usize = 512;
type KernelInfoBlock = [u8; BLOCK_SIZE];

pub(crate) fn build(mut args: Args) -> Result<(), Error> {
    let metadata = read_cargo_metadata(&args)?;
    let crate_root = PathBuf::from(&metadata.workspace_root);
    let manifest_path = args.manifest_path().as_ref().map(Clone::clone).unwrap_or({
        let mut path = crate_root.clone();
        path.push("Cargo.toml");
        path
    });
    let config = config::read_config(manifest_path)?;

    if args.target().is_none() {
        if let Some(ref target) = config.default_target {
            args.set_target(target.clone());
        }
    }

    let (kernel, out_dir) = build_kernel(&args, &config, &metadata)?;

    let kernel_size = kernel.metadata()?.len();
    let kernel_info_block = create_kernel_info_block(kernel_size);

    let bootloader = build_bootloader(&out_dir, &config)?;

    create_disk_image(&config, kernel, kernel_info_block, &bootloader)?;

    Ok(())
}

fn read_cargo_metadata(args: &Args) -> Result<CargoMetadata, cargo_metadata::Error> {
    cargo_metadata::metadata(args.manifest_path().as_ref().map(PathBuf::as_path))
}

fn build_kernel(args: &args::Args, config: &Config, metadata: &CargoMetadata) -> Result<(File, PathBuf), Error> {
    let crate_ = metadata.packages.iter()
        .find(|p| Path::new(&p.manifest_path) == config.manifest_path)
        .expect("Could not read crate name from cargo metadata");
    let crate_name = &crate_.name;

    let target_dir = PathBuf::from(&metadata.target_directory);

    // compile kernel
    println!("Building kernel");
    let exit_status = run_xargo_build(&env::current_dir()?, &args.all_cargo)?;
    if !exit_status.success() { process::exit(1) }

    let mut out_dir = target_dir;
    if let &Some(ref target) = args.target() {
        out_dir.push(target);
    }
    if args.release() {
        out_dir.push("release");
    } else {
        out_dir.push("debug");
    }

    let mut kernel_path = out_dir.clone();
    kernel_path.push(crate_name);
    let kernel = File::open(kernel_path)?;
    Ok((kernel, out_dir))
}

fn run_xargo_build(pwd: &Path, args: &[String]) -> io::Result<process::ExitStatus> {
    let mut command = process::Command::new("xargo");
    command.arg("build");
    command.current_dir(pwd).env("RUST_TARGET_PATH", pwd);
    command.args(args);
    command.status()
}

fn create_kernel_info_block(kernel_size: u64) -> KernelInfoBlock {
    let kernel_size = if kernel_size <= u64::from(u32::max_value()) {
        kernel_size as u32
    } else {
        panic!("Kernel can't be loaded by BIOS bootloader because is too big")
    };

    let mut kernel_info_block = [0u8; BLOCK_SIZE];
    LittleEndian::write_u32(&mut kernel_info_block[0..4], kernel_size);

    kernel_info_block
}

fn download_bootloader(out_dir: &Path, config: &Config) -> Result<CrateMetadata, Error> {
    use std::io::Write;

    let bootloader_dir = {
        let mut dir = PathBuf::from(out_dir);
        dir.push("bootloader");
        dir
    };

    let cargo_toml = {
        let mut dir = bootloader_dir.clone();
        dir.push("Cargo.toml");
        dir
    };
    let src_lib = {
        let mut dir = bootloader_dir.clone();
        dir.push("src");
        fs::create_dir_all(dir.as_path())?;
        dir.push("lib.rs");
        dir
    };

    {
        let mut cargo_toml_file = File::create(&cargo_toml)?;
        cargo_toml_file.write_all(r#"
            [package]
            authors = ["author@example.com>"]
            name = "bootloader_download_helper"
            version = "0.0.0"

        "#.as_bytes())?;
        cargo_toml_file.write_all(format!(r#"
            [dependencies.{}]
        "#, config.bootloader.name).as_bytes())?;
        if let &Some(ref version) = &config.bootloader.version {
            cargo_toml_file.write_all(format!(r#"
                    version = "{}"
            "#, version).as_bytes())?;
        }
        if let &Some(ref git) = &config.bootloader.git {
            cargo_toml_file.write_all(format!(r#"
                    git = "{}"
            "#, git).as_bytes())?;
        }
        if let &Some(ref branch) = &config.bootloader.branch {
            cargo_toml_file.write_all(format!(r#"
                    branch = "{}"
            "#, branch).as_bytes())?;
        }
        if let &Some(ref path) = &config.bootloader.path {
            cargo_toml_file.write_all(format!(r#"
                    path = "{}"
            "#, path.display()).as_bytes())?;
        }

        File::create(src_lib)?.write_all(r#"
            #![no_std]
        "#.as_bytes())?;
    }

    let mut command = process::Command::new("cargo");
    command.arg("fetch");
    command.current_dir(bootloader_dir);
    assert!(command.status()?.success(), "Bootloader download failed.");

    let metadata = cargo_metadata::metadata_deps(Some(&cargo_toml), true)?;
    let bootloader = metadata.packages.iter().find(|p| p.name == config.bootloader.name)
            .expect(&format!("Could not find crate named “{}”", config.bootloader.name));

    Ok(bootloader.clone())
}

fn build_bootloader(out_dir: &Path, config: &Config) -> Result<Box<[u8]>, Error> {
    use std::io::Read;

    let bootloader_metadata = download_bootloader(out_dir, config)?;
    let bootloader_dir = Path::new(&bootloader_metadata.manifest_path).parent().unwrap();


    let bootloader_elf_path = if !config.bootloader.precompiled {
        let args = &[
            String::from("--target"),
            config.bootloader.target.clone(),
            String::from("--release"),
        ];

        println!("Building bootloader");
        let exit_status = run_xargo_build(bootloader_dir, args)?;
        if !exit_status.success() { process::exit(1) }

        let mut bootloader_elf_path = bootloader_dir.to_path_buf();
        bootloader_elf_path.push("target");
        bootloader_elf_path.push(&config.bootloader.target);
        bootloader_elf_path.push("release");
        bootloader_elf_path.push("bootloader");
        bootloader_elf_path
    } else {
        let mut bootloader_elf_path = bootloader_dir.to_path_buf();
        bootloader_elf_path.push("bootloader");
        bootloader_elf_path
    };

    let mut bootloader_elf_bytes = Vec::new();
    let mut bootloader = File::open(&bootloader_elf_path).map_err(|err| {
        Error::Bootloader(format!("Could not open bootloader at {}",
            bootloader_elf_path.display()), err)
    })?;
    bootloader.read_to_end(&mut bootloader_elf_bytes)?;

    // copy bootloader section of ELF file to bootloader_path
    let elf_file = xmas_elf::ElfFile::new(&bootloader_elf_bytes).unwrap();
    xmas_elf::header::sanity_check(&elf_file).unwrap();
    let bootloader_section = elf_file.find_section_by_name(".bootloader")
        .expect("bootloader must have a .bootloader section");

    Ok(Vec::from(bootloader_section.raw_data(&elf_file)).into_boxed_slice())
}

fn create_disk_image(config: &Config, mut kernel: File, kernel_info_block: KernelInfoBlock,
    bootloader_data: &[u8]) -> Result<(), Error>
{
    use std::io::{Read, Write};

    println!("Creating disk image at {}", config.output.display());
    let mut output = File::create(&config.output)?;
    output.write_all(&bootloader_data)?;
    output.write_all(&kernel_info_block)?;

    // write out kernel elf file
    let kernel_size = kernel.metadata()?.len();
    let mut buffer = [0u8; 1024];
    loop {
        let (n, interrupted) = match kernel.read(&mut buffer) {
            Ok(0) => break,
            Ok(n) => (n, false),
            Err(ref e) if e.kind() == io::ErrorKind::Interrupted => (0, true),
            Err(e) => Err(e)?,
        };
        if !interrupted {
            output.write_all(&buffer[..n])?
        }
    }

    let padding_size = ((512 - (kernel_size % 512)) % 512) as usize;
    let padding = [0u8; 512];
    output.write_all(&padding[..padding_size])?;

    Ok(())
}
