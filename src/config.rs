use failure::{Error, ResultExt};
use std::path::PathBuf;
use toml::Value;

#[derive(Debug, Clone)]
pub struct Config {
    pub manifest_path: PathBuf,
    pub default_target: Option<String>,
    pub output: Option<PathBuf>,
    pub bootloader: BootloaderConfig,
    pub minimum_image_size: Option<u64>,
    pub run_command: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct BootloaderConfig {
    pub name: String,
    pub target: PathBuf,
}

pub(crate) fn read_config(manifest_path: PathBuf) -> Result<Config, Error> {
    use std::{fs::File, io::Read};
    let cargo_toml: Value = {
        let mut content = String::new();
        File::open(&manifest_path)
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
            return Ok(ConfigBuilder {
                manifest_path: Some(manifest_path),
                ..Default::default()
            }.into())
        }
        Some(metadata) => metadata.as_table().ok_or(format_err!(
            "Bootimage configuration invalid: {:?}",
            metadata
        ))?,
    };

    let mut config = ConfigBuilder {
        manifest_path: Some(manifest_path),
        ..Default::default()
    };

    for (key, value) in metadata {
        match (key.as_str(), value.clone()) {
            ("default-target", Value::String(s)) => config.default_target = From::from(s),
            ("output", Value::String(s)) => config.output = Some(PathBuf::from(s)),
            ("bootloader", Value::Table(t)) => {
                let mut bootloader_config = BootloaderConfigBuilder::default();
                for (key, value) in t {
                    match (key.as_str(), value) {
                        ("name", Value::String(s)) => bootloader_config.name = From::from(s),
                        ("target", Value::String(s)) => {
                            bootloader_config.target = Some(PathBuf::from(s))
                        }
                        (k @ "precompiled", _)
                        | (k @ "version", _)
                        | (k @ "git", _)
                        | (k @ "branch", _)
                        | (k @ "path", _) => Err(format_err!(
                            "the \
                             `package.metadata.bootimage.bootloader` key `{}` was deprecated",
                            k
                        ))?,
                        (key, value) => Err(format_err!(
                            "unexpected \
                             `package.metadata.bootimage.bootloader` key `{}` with value `{}`",
                            key,
                            value
                        ))?,
                    }
                }
                config.bootloader = Some(bootloader_config);
            }
            ("minimum-image-size", Value::Integer(x)) => {
                if x >= 0 {
                    config.minimum_image_size = Some((x * 1024 * 1024) as u64); // MiB -> Byte
                } else {
                    Err(format_err!(
                        "unexpected `package.metadata.bootimage` \
                         key `minimum-image-size` with negative value `{}`",
                        value
                    ))?
                }
            }
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
    output: Option<PathBuf>,
    bootloader: Option<BootloaderConfigBuilder>,
    minimum_image_size: Option<u64>,
    run_command: Option<Vec<String>>,
}

#[derive(Default)]
struct BootloaderConfigBuilder {
    name: Option<String>,
    target: Option<PathBuf>,
}

impl Into<Config> for ConfigBuilder {
    fn into(self) -> Config {
        Config {
            manifest_path: self.manifest_path.expect("manifest path must be set"),
            default_target: self.default_target,
            output: self.output,
            bootloader: self.bootloader.unwrap_or_default().into(),
            minimum_image_size: self.minimum_image_size,
            run_command: self.run_command.unwrap_or(vec![
                "qemu-system-x86_64".into(),
                "-drive".into(),
                "format=raw,file={}".into(),
            ]),
        }
    }
}

impl Into<BootloaderConfig> for BootloaderConfigBuilder {
    fn into(self) -> BootloaderConfig {
        BootloaderConfig {
            name: self.name.unwrap_or("bootloader".into()),
            target: self.target
                .unwrap_or(PathBuf::from("x86_64-bootloader.json")),
        }
    }
}
