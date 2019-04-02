use crate::ErrorMessage;
use std::path::Path;
use toml::Value;

#[derive(Debug, Clone)]
pub struct Config {
    pub default_target: Option<String>,
    pub run_command: Vec<String>,
    pub run_args: Option<Vec<String>>,
    pub test_timeout: u32,
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
    test_timeout: Option<u32>,
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
            test_timeout: self.test_timeout.unwrap_or(60 * 5),
            non_exhaustive: (),
        }
    }
}
