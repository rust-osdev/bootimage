use std::{
    fs,
    path::{Path, PathBuf},
};

pub fn default_target_triple(crate_root: &Path, walk_up: bool) -> Result<Option<String>, String> {
    let default_triple = default_target(crate_root, walk_up)?;
    default_triple
        .map(|(target, crate_root)| {
            if target.ends_with(".json") {
                crate_root
                    .join(target)
                    .file_stem()
                    .ok_or(String::from(
                        "The target path specfied in `build.target` has no file stem",
                    ))?
                    .to_os_string()
                    .into_string()
                    .map_err(|err| format!("Default target triple not valid UTF-8: {:?}", err))
            } else {
                Ok(target)
            }
        })
        .transpose()
}

fn default_target(crate_root: &Path, walk_up: bool) -> Result<Option<(String, PathBuf)>, String> {
    let mut parent_dir = crate_root;

    loop {
        let config_path = parent_dir.join(".cargo/config");
        if config_path.exists() {
            let config_content = fs::read_to_string(config_path).map_err(|err| {
                format!("Failed to read `.cargo/config` file of crate: {:?}", err)
            })?;
            let config = config_content.parse::<toml::Value>().map_err(|err| {
                format!(
                    "Failed to parse `.cargo/config` of crate as toml: {:?}",
                    err
                )
            })?;
            let target = config
                .get("build")
                .and_then(|v| v.get("target"))
                .and_then(|v| v.as_str())
                .map(String::from);
            if let Some(target) = target {
                return Ok(Some((target, parent_dir.to_owned())));
            }
        }
        if walk_up {
            parent_dir = match parent_dir.parent() {
                Some(parent) => parent,
                None => break,
            }
        } else {
            break;
        }
    }
    Ok(None)
}
