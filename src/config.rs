use std::path::{Path, PathBuf};
use Error;
use toml::Value;

pub struct Config {
    pub manifest_path: PathBuf,
    pub default_target: Option<String>,
    pub bootloader: BootloaderConfig,
}

pub struct BootloaderConfig {
    pub name: String,
    pub precompiled: bool,
    pub target: String,
    pub version: Option<String>,
    pub git: Option<String>,
    pub path: Option<PathBuf>,
}

pub(crate) fn read_config(manifest_path: PathBuf) -> Result<Config, Error> {
    use std::{fs::File, io::Read};
    let cargo_toml: Value = {
        let mut content = String::new();
        File::open(&manifest_path)?.read_to_string(&mut content)?;
        content.parse()?
    };

    let metadata = cargo_toml.get("package")
        .and_then(|table| table.get("metadata"))
        .and_then(|table| table.get("bootimage"));
    let metadata = match metadata {
        None => {
            return Ok(ConfigBuilder {
               manifest_path: Some(manifest_path),
               ..Default::default()
            }.into())
        }
        Some(metadata) => metadata.as_table().ok_or(Error::Config(
            format!("Bootimage configuration invalid: {:?}", metadata)
        ))?,
    };

    let mut config = ConfigBuilder {
        manifest_path: Some(manifest_path),
        ..Default::default()
    };

    for (key, value) in metadata {
        match (key.as_str(), value.clone()) {
            ("default-target", Value::String(s)) => config.default_target = From::from(s),
            ("bootloader", Value::Table(t)) => {
                let mut bootloader_config = BootloaderConfigBuilder::default();
                for (key, value) in t {
                    match (key.as_str(), value) {
                        ("name", Value::String(s)) => bootloader_config.name = From::from(s),
                        ("precompiled", Value::Boolean(b)) => {
                            bootloader_config.precompiled = From::from(b)
                        },
                        ("version", Value::String(s)) => bootloader_config.version = From::from(s),
                        ("git", Value::String(s)) => bootloader_config.git = From::from(s),
                        ("path", Value::String(s)) => {
                            bootloader_config.path = Some(Path::new(&s).canonicalize()?);
                        }
                        (key, value) => {
                            Err(Error::Config(format!("unexpected \
                                `package.metadata.bootimage.bootloader` key `{}` with value `{}`",
                                key, value)))?
                        }
                    }
                }
                config.bootloader = Some(bootloader_config);
            }
            (key, value) => {
                Err(Error::Config(format!("unexpected `package.metadata.bootimage` \
                    key `{}` with value `{}`", key, value)))?
            }
        }
    }
    Ok(config.into())
}

#[derive(Default)]
struct ConfigBuilder {
    manifest_path: Option<PathBuf>,
    default_target: Option<String>,
    bootloader: Option<BootloaderConfigBuilder>,
}

#[derive(Default)]
struct BootloaderConfigBuilder {
    name: Option<String>,
    precompiled: Option<bool>,
    target: Option<String>,
    version: Option<String>,
    git: Option<String>,
    path: Option<PathBuf>,
}

impl Into<Config> for ConfigBuilder {
    fn into(self) -> Config {
        let default_bootloader_config = BootloaderConfigBuilder {
            precompiled: Some(true),
            ..Default::default()
        };
        Config {
            manifest_path: self.manifest_path.expect("manifest path must be set"),
            default_target: self.default_target,
            bootloader: self.bootloader.unwrap_or(default_bootloader_config).into(),
        }
    }
}

impl Into<BootloaderConfig> for BootloaderConfigBuilder {
    fn into(self) -> BootloaderConfig {
        let precompiled = self.precompiled.unwrap_or(false);
        let default_name = if precompiled { "bootloader_precompiled" } else { "bootloader" };
        BootloaderConfig {
            name: self.name.unwrap_or(default_name.into()),
            precompiled,
            target: self.target.unwrap_or("x86_64-bootloader".into()),
            version: self.version,
            git: self.git,
            path: self.path,
        }
    }
}
