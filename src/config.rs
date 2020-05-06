//! Parses the `package.metadata.bootimage` configuration table

use anyhow::{anyhow, Context, Result};
use std::path::Path;
use toml::Value;

/// Represents the `package.metadata.bootimage` configuration table
///
/// The bootimage crate can be configured through a `package.metadata.bootimage` table
/// in the `Cargo.toml` file of the kernel. This struct represents the parsed configuration
/// options.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Config {
    /// The run command that is invoked on `bootimage run` or `bootimage runner`
    ///
    /// The substring "{}" will be replaced with the path to the bootable disk image.
    pub run_command: Vec<String>,
    /// Additional arguments passed to the runner for not-test binaries
    ///
    /// Applies to `bootimage run` and `bootimage runner`.
    pub run_args: Option<Vec<String>>,
    /// Additional arguments passed to the runner for test binaries
    ///
    /// Applies to `bootimage runner`.
    pub test_args: Option<Vec<String>>,
    /// The timeout for running an test through `bootimage test` or `bootimage runner` in seconds
    pub test_timeout: u32,
    /// An exit code that should be considered as success for test executables (applies to
    /// `bootimage runner`)
    pub test_success_exit_code: Option<i32>,
}

/// Reads the configuration from a `package.metadata.bootimage` in the given Cargo.toml.
pub fn read_config(manifest_path: &Path) -> Result<Config> {
    read_config_inner(manifest_path).context("Failed to read bootimage configuration")
}

fn read_config_inner(manifest_path: &Path) -> Result<Config> {
    use std::{fs::File, io::Read};
    let cargo_toml: Value = {
        let mut content = String::new();
        File::open(manifest_path)
            .context("Failed to open Cargo.toml")?
            .read_to_string(&mut content)
            .context("Failed to read Cargo.toml")?;
        content
            .parse::<Value>()
            .context("Failed to parse Cargo.toml")?
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
            .ok_or(anyhow!("Bootimage configuration invalid: {:?}", metadata))?,
    };

    let mut config = ConfigBuilder::default();

    for (key, value) in metadata {
        match (key.as_str(), value.clone()) {
            ("test-timeout", Value::Integer(timeout)) if timeout.is_negative() => {
                Err(anyhow!("test-timeout must not be negative"))?
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
                        _ => Err(anyhow!("run-command must be a list of strings"))?,
                    }
                }
                config.run_command = Some(command);
            }
            ("run-args", Value::Array(array)) => {
                let mut args = Vec::new();
                for value in array {
                    match value {
                        Value::String(s) => args.push(s),
                        _ => Err(anyhow!("run-args must be a list of strings"))?,
                    }
                }
                config.run_args = Some(args);
            }
            ("test-args", Value::Array(array)) => {
                let mut args = Vec::new();
                for value in array {
                    match value {
                        Value::String(s) => args.push(s),
                        _ => Err(anyhow!("test-args must be a list of strings"))?,
                    }
                }
                config.test_args = Some(args);
            }
            (key, value) => Err(anyhow!(
                "unexpected `package.metadata.bootimage` \
                 key `{}` with value `{}`",
                key,
                value
            ))?,
        }
    }
    Ok(config.into())
}

#[derive(Default)]
struct ConfigBuilder {
    run_command: Option<Vec<String>>,
    run_args: Option<Vec<String>>,
    test_args: Option<Vec<String>>,
    test_timeout: Option<u32>,
    test_success_exit_code: Option<i32>,
}

impl Into<Config> for ConfigBuilder {
    fn into(self) -> Config {
        Config {
            run_command: self.run_command.unwrap_or(vec![
                "qemu-system-x86_64".into(),
                "-drive".into(),
                "format=raw,file={}".into(),
            ]),
            run_args: self.run_args,
            test_args: self.test_args,
            test_timeout: self.test_timeout.unwrap_or(60 * 5),
            test_success_exit_code: self.test_success_exit_code,
        }
    }
}
