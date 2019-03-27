use crate::ErrorString;
use failure::{Error, ResultExt};
use std::path::PathBuf;
use toml::Value;

#[derive(Debug, Clone)]
pub struct Config {
    pub manifest_path: PathBuf,
    pub default_target: Option<String>,
    pub run_command: Vec<String>,
}

pub(crate) fn read_config(manifest_path: PathBuf) -> Result<Config, ErrorString> {
    let config = read_config_inner(manifest_path)
        .map_err(|err| format!("Failed to read bootimage configuration: {:?}", err))?;
    Ok(config)
}

pub(crate) fn read_config_inner(manifest_path: PathBuf) -> Result<Config, Error> {
    use std::{fs::File, io::Read};
    let cargo_toml: Value = {
        let mut content = String::new();
        File::open(&manifest_path)
            .with_context(|e| format!("Failed to open Cargo.toml: {}", e))?
            .read_to_string(&mut content)
            .with_context(|e| format!("Failed to read Cargo.toml: {}", e))?;
        content
            .parse::<Value>()
            .with_context(|e| format!("Failed to parse Cargo.toml: {}", e))?
    };

    let metadata = cargo_toml
        .get("package")
        .and_then(|table| table.get("metadata"))
        .and_then(|table| table.get("bootimage"));
    let metadata = match metadata {
        None => {
            return Ok(ConfigBuilder {
                manifest_path: Some(manifest_path),
                ..Default::default()
            }
            .into());
        }
        Some(metadata) => metadata.as_table().ok_or(format_err!(
            "Bootimage configuration invalid: {:?}",
            metadata
        ))?,
    };

    /*
     * The user shouldn't specify any features if they're using a precompiled bootloader, as we
     * don't actually compile it.
     */
    if cargo_toml
        .get("dependencies")
        .and_then(|table| table.get("bootloader_precompiled"))
        .and_then(|table| {
            table
                .get("features")
                .or_else(|| table.get("default-features"))
        })
        .is_some()
    {
        return Err(format_err!(
            "Can't change features of precompiled bootloader!"
        ));
    }

    let mut config = ConfigBuilder {
        manifest_path: Some(manifest_path),
        ..Default::default()
    };

    for (key, value) in metadata {
        match (key.as_str(), value.clone()) {
            ("default-target", Value::String(s)) => config.default_target = From::from(s),
            ("run-command", Value::Array(array)) => {
                let mut command = Vec::new();
                for value in array {
                    match value {
                        Value::String(s) => command.push(s),
                        _ => Err(format_err!("run-command must be a list of strings"))?,
                    }
                }
                config.run_command = Some(command);
            }
            (key, value) => Err(format_err!(
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
    manifest_path: Option<PathBuf>,
    default_target: Option<String>,
    run_command: Option<Vec<String>>,
}

impl Into<Config> for ConfigBuilder {
    fn into(self) -> Config {
        Config {
            manifest_path: self.manifest_path.expect("manifest path must be set"),
            default_target: self.default_target,
            run_command: self.run_command.unwrap_or(vec![
                "qemu-system-x86_64".into(),
                "-drive".into(),
                "format=raw,file={}".into(),
            ]),
        }
    }
}
