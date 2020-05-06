use super::error::DiskImageError;
use std::{path::Path, process::Command};

pub fn create_disk_image(
    bootloader_elf_path: &Path,
    output_bin_path: &Path,
) -> Result<(), DiskImageError> {
    let llvm_tools = llvm_tools::LlvmTools::new()?;
    let objcopy = llvm_tools
        .tool(&llvm_tools::exe("llvm-objcopy"))
        .ok_or(DiskImageError::LlvmObjcopyNotFound)?;

    // convert bootloader to binary
    let mut cmd = Command::new(objcopy);
    cmd.arg("-I").arg("elf64-x86-64");
    cmd.arg("-O").arg("binary");
    cmd.arg("--binary-architecture=i386:x86-64");
    cmd.arg(bootloader_elf_path);
    cmd.arg(output_bin_path);
    let output = cmd.output().map_err(|err| DiskImageError::Io {
        message: "failed to execute llvm-objcopy command",
        error: err,
    })?;
    if !output.status.success() {
        return Err(DiskImageError::ObjcopyFailed {
            stderr: output.stderr,
        });
    }

    pad_to_nearest_block_size(output_bin_path)?;
    Ok(())
}

fn pad_to_nearest_block_size(output_bin_path: &Path) -> Result<(), DiskImageError> {
    const BLOCK_SIZE: u64 = 512;
    use std::fs::OpenOptions;
    let file = OpenOptions::new()
        .write(true)
        .open(&output_bin_path)
        .map_err(|err| DiskImageError::Io {
            message: "failed to open boot image",
            error: err,
        })?;
    let file_size = file
        .metadata()
        .map_err(|err| DiskImageError::Io {
            message: "failed to get size of boot image",
            error: err,
        })?
        .len();
    let remainder = file_size % BLOCK_SIZE;
    let padding = if remainder > 0 {
        BLOCK_SIZE - remainder
    } else {
        0
    };
    file.set_len(file_size + padding)
        .map_err(|err| DiskImageError::Io {
            message: "failed to pad boot image to a multiple of the block size",
            error: err,
        })
}
