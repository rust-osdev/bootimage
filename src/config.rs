//! Parses the `package.metadata.bootimage` configuration table

use crate::ErrorMessage;
use std::path::Path;
use toml::Value;

/// Represents the `package.metadata.bootimage` configuration table
///
/// The bootimage crate can be configured through a `package.metadata.bootimage` table
/// in the `Cargo.toml` file of the kernel. This struct represents the parsed configuration
/// options.
#[derive(Debug, Clone)]
pub struct Config {
    /// This target is used if no `--target` argument is passed
    pub default_target: Option<String>,
    /// The run command that is invoked on `bootimage run` or `bootimage runner`
    ///
    /// The substring "{}" will be replaced with the path to the bootable disk image.
    pub run_command: Vec<String>,
    /// Additional arguments passed to the runner for not-test binaries
    ///
    /// Applies to `bootimage run` and `bootimage runner`.
    pub run_args: Option<Vec<String>>,
    /// The offset used for mapping physical memory when the `map_physical_memory` feature of the
    /// bootloader is enabled. If not provided, the bootloader dynamically finds a free virtual
    /// address region.
    pub physical_memory_offset: Option<u64>,
    /// The address of the kernel stack. If not provided, the bootloader dynamically finds a free
    /// virtual address region.
    pub kernel_stack_address: Option<u64>,
    /// The size of the kernel stack, in number of 4KiB pages. Defaults to 512.
    pub kernel_stack_size: Option<u64>,
    /// Additional arguments passed to the runner for test binaries
    ///
    /// Applies to `bootimage runner`.
    pub test_args: Option<Vec<String>>,
    /// The timeout for running an test through `bootimage test` or `bootimage runner` in seconds
    pub test_timeout: u32,
    /// An exit code that should be considered as success for test executables (applies to
    /// `bootimage runner`)
    pub test_success_exit_code: Option<i32>,
    non_exhaustive: (),
}

pub(crate) fn read_config(manifest_path: &Path) -> Result<Config, ErrorMessage> {
    let config = read_config_inner(manifest_path)
        .map_err(|err| format!("Failed to read bootimage configuration: {:?}", err))?;
    Ok(config)
}

pub(crate) fn read_config_inner(manifest_path: &Path) -> Result<Config, ErrorMessage> {
    use std::{fs::File, io::Read};
    let cargo_toml: Value = {
        let mut content = String::new();
        File::open(manifest_path)
            .map_err(|e| format!("Failed to open Cargo.toml: {}", e))?
            .read_to_string(&mut content)
            .map_err(|e| format!("Failed to read Cargo.toml: {}", e))?;
        content
            .parse::<Value>()
            .map_err(|e| format!("Failed to parse Cargo.toml: {}", e))?
    };

    let metadata = cargo_toml
        .get("package")
        .and_then(|table| table.get("metadata"))
        .and_then(|table| table.get("bootimage"));
    let metadata = match metadata {
        None => {
            return Ok(ConfigBuilder::default().into());
        }
        Some(metadata) => metadata
            .as_table()
            .ok_or(format!("Bootimage configuration invalid: {:?}", metadata))?,
    };

    let mut config = ConfigBuilder::default();

    for (key, value) in metadata {
        match (key.as_str(), value.clone()) {
            ("default-target", Value::String(s)) => config.default_target = From::from(s),
            ("test-timeout", Value::Integer(timeout)) if timeout.is_negative() => {
                Err(format!("test-timeout must not be negative"))?
            }
            ("test-timeout", Value::Integer(timeout)) => {
                config.test_timeout = Some(timeout as u32);
            }
            ("test-success-exit-code", Value::Integer(exit_code)) => {
                config.test_success_exit_code = Some(exit_code as i32);
            }
            ("run-command", Value::Array(array)) => {
                let mut command = Vec::new();
                for value in array {
                    match value {
                        Value::String(s) => command.push(s),
                        _ => Err(format!("run-command must be a list of strings"))?,
                    }
                }
                config.run_command = Some(command);
            }
            ("run-args", Value::Array(array)) => {
                let mut args = Vec::new();
                for value in array {
                    match value {
                        Value::String(s) => args.push(s),
                        _ => Err(format!("run-args must be a list of strings"))?,
                    }
                }
                config.run_args = Some(args);
            }

            ("physical-memory-offset", Value::Integer(_)) => {
                Err(format!("physical-memory-offset must be given as a string, as TOML does not support \
                unsigned 64-bit integers"))?
            }
            ("kernel-stack-address", Value::Integer(_)) => {
                Err(format!("kernel-stack-address must be given as a string, as TOML does not support \
                unsigned 64-bit integers"))?
            }
            ("physical-memory-offset", Value::String(s)) => {
                let offset = if s.starts_with("0x") {
                    u64::from_str_radix(&s[2..], 16)
                } else {
                    u64::from_str_radix(&s, 10)
                };
                let offset = offset.map_err(|e| format!("failed to parse physical-memory-offset as u64: {} (was \"{}\")", e, s))?;
                config.physical_memory_offset = Some(offset);
            }
            ("kernel-stack-address", Value::String(s)) => {
                let address = if s.starts_with("0x") {
                    u64::from_str_radix(&s[2..], 16)
                } else {
                    u64::from_str_radix(&s, 10)
                };
                let address = address.map_err(|e| format!("failed to parse kernel-stack-address as u64: {} (was \"{}\")", e, s))?;
                config.kernel_stack_address = Some(address);
            }
            ("kernel-stack-size", Value::Integer(size)) => {
                if size.is_negative() {
                    Err(format!("kernel-stack-size must not be negative"))?
                } else {
                    config.kernel_stack_size = Some(size as u64);
                }
            }
            ("test-args", Value::Array(array)) => {
                let mut args = Vec::new();
                for value in array {
                    match value {
                        Value::String(s) => args.push(s),
                        _ => Err(format!("test-args must be a list of strings"))?,
                    }
                }
                config.test_args = Some(args);
            }
            (key, value) => Err(format!(
                "unexpected `package.metadata.bootimage` \
                 key `{}` with value `{}`",
                key, value
            ))?,
        }
    }
    Ok(config.into())
}

#[derive(Default)]
struct ConfigBuilder {
    default_target: Option<String>,
    run_command: Option<Vec<String>>,
    run_args: Option<Vec<String>>,
    physical_memory_offset: Option<u64>,
    kernel_stack_address: Option<u64>,
    kernel_stack_size: Option<u64>,
    test_args: Option<Vec<String>>,
    test_timeout: Option<u32>,
    test_success_exit_code: Option<i32>,
}

impl Into<Config> for ConfigBuilder {
    fn into(self) -> Config {
        Config {
            default_target: self.default_target,
            run_command: self.run_command.unwrap_or(vec![
                "qemu-system-x86_64".into(),
                "-drive".into(),
                "format=raw,file={}".into(),
            ]),
            run_args: self.run_args,
            physical_memory_offset: self.physical_memory_offset,
            kernel_stack_address: self.kernel_stack_address,
            kernel_stack_size: self.kernel_stack_size,
            test_args: self.test_args,
            test_timeout: self.test_timeout.unwrap_or(60 * 5),
            test_success_exit_code: self.test_success_exit_code,
            non_exhaustive: (),
        }
    }
}
