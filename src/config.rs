use crate::ErrorString;
use failure::{Error, ResultExt};
use std::path::PathBuf;
use toml::Value;

#[derive(Debug, Clone)]
pub struct Config {
    pub manifest_path: PathBuf,
    pub default_target: Option<String>,
    pub output: Option<PathBuf>,         // remove
    pub bootloader: BootloaderConfig,    // remove
    pub minimum_image_size: Option<u64>, // remove
    pub run_command: Vec<String>,
    pub package_filepath: Option<PathBuf>, // remove
}

#[derive(Debug, Clone)]
pub struct BootloaderConfig {
    pub name: Option<String>,
    pub target: PathBuf,
    pub default_features: bool,
    pub features: Vec<String>,
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

    let bootloader_dependency = cargo_toml
        .get("dependencies")
        .and_then(|table| table.get("bootloader"));
    let bootloader_default_features =
        match bootloader_dependency.and_then(|table| table.get("default-features")) {
            None => None,
            Some(Value::Boolean(default_features)) => Some(*default_features),
            Some(_) => {
                return Err(format_err!(
                    "Bootloader 'default-features' field should be a bool!"
                ));
            }
        };

    let bootloader_features = match cargo_toml
        .get("dependencies")
        .and_then(|table| table.get("bootloader"))
        .and_then(|table| table.get("features"))
    {
        None => None,
        Some(Value::Array(array)) => {
            let mut features = Vec::new();

            for feature_string in array {
                match feature_string {
                    Value::String(feature) => features.push(feature.clone()),
                    _ => return Err(format_err!("Bootloader features are malformed!")),
                }
            }

            Some(features)
        }
        Some(_) => return Err(format_err!("Bootloader features are malformed!")),
    };

    let mut config = ConfigBuilder {
        manifest_path: Some(manifest_path),
        bootloader: BootloaderConfigBuilder {
            features: bootloader_features,
            default_features: bootloader_default_features,
            ..Default::default()
        },
        ..Default::default()
    };

    for (key, value) in metadata {
        match (key.as_str(), value.clone()) {
            ("default-target", Value::String(s)) => config.default_target = From::from(s),
            ("output", Value::String(s)) => config.output = Some(PathBuf::from(s)),
            ("bootloader", Value::Table(t)) => {
                for (key, value) in t {
                    match (key.as_str(), value) {
                        ("name", Value::String(s)) => config.bootloader.name = From::from(s),
                        ("target", Value::String(s)) => {
                            config.bootloader.target = Some(PathBuf::from(s))
                        }
                        (k @ "precompiled", _)
                        | (k @ "version", _)
                        | (k @ "git", _)
                        | (k @ "branch", _)
                        | (k @ "path", _) => Err(format_err!(
                            "the \
                             `package.metadata.bootimage.bootloader` key `{}` was deprecated\n\n\
                             In case you just updated bootimage from an earlier version, \
                             check out the migration guide at \
                             https://github.com/rust-osdev/bootimage/pull/16.",
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
            ("package-file", Value::String(path)) => {
                config.package_filepath = Some(PathBuf::from(path));
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
    bootloader: BootloaderConfigBuilder,
    minimum_image_size: Option<u64>,
    run_command: Option<Vec<String>>,
    package_filepath: Option<PathBuf>,
}

#[derive(Default)]
struct BootloaderConfigBuilder {
    name: Option<String>,
    target: Option<PathBuf>,
    features: Option<Vec<String>>,
    default_features: Option<bool>,
}

impl Into<Config> for ConfigBuilder {
    fn into(self) -> Config {
        Config {
            manifest_path: self.manifest_path.expect("manifest path must be set"),
            default_target: self.default_target,
            output: self.output,
            bootloader: self.bootloader.into(),
            minimum_image_size: self.minimum_image_size,
            run_command: self.run_command.unwrap_or(vec![
                "qemu-system-x86_64".into(),
                "-drive".into(),
                "format=raw,file={}".into(),
            ]),
            package_filepath: self.package_filepath,
        }
    }
}

impl Into<BootloaderConfig> for BootloaderConfigBuilder {
    fn into(self) -> BootloaderConfig {
        BootloaderConfig {
            name: self.name,
            target: self
                .target
                .unwrap_or(PathBuf::from("x86_64-bootloader.json")),
            features: self.features.unwrap_or(Vec::with_capacity(0)),
            default_features: self.default_features.unwrap_or(true),
        }
    }
}
