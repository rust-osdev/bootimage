use super::error::BootloaderError;
use cargo_metadata::{Metadata, Package};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

pub struct BuildConfig {
    manifest_path: PathBuf,
    bootloader_name: String,
    target: PathBuf,
    features: Vec<String>,
    target_dir: PathBuf,
    kernel_bin_path: PathBuf,
    kernel_manifest_path: PathBuf,
}

impl BuildConfig {
    /// Derives the bootloader build config from the project's metadata.
    pub fn from_metadata(
        project_metadata: &Metadata,
        kernel_bin_name: &str,
        kernel_bin_path: &Path,
    ) -> Result<Self, BootloaderError> {
        let kernel_pkg = kernel_package(project_metadata, kernel_bin_name)?;
        let bootloader_pkg = bootloader_package(project_metadata, kernel_pkg)?;
        let bootloader_root =
            bootloader_pkg
                .manifest_path
                .parent()
                .ok_or(BootloaderError::BootloaderInvalid(
                    "bootloader manifest has no target directory".into(),
                ))?;

        let cargo_toml_content = fs::read_to_string(&bootloader_pkg.manifest_path)
            .map_err(|err| format!("bootloader has no valid Cargo.toml: {}", err))
            .map_err(BootloaderError::BootloaderInvalid)?;
        let cargo_toml = cargo_toml_content
            .parse::<toml::Value>()
            .map_err(|e| format!("Failed to parse Cargo.toml of bootloader: {}", e))
            .map_err(BootloaderError::BootloaderInvalid)?;
        let metadata = cargo_toml.get("package").and_then(|t| t.get("metadata"));
        let target = metadata
            .and_then(|t| t.get("bootloader"))
            .and_then(|t| t.get("target"));
        let target_str =
            target
                .and_then(|v| v.as_str())
                .ok_or(BootloaderError::BootloaderInvalid(
                "No `package.metadata.bootloader.target` key found in Cargo.toml of bootloader\n\n\
                 (If you're using the official bootloader crate, you need at least version 0.5.1)"
                    .into(),
            ))?;

        let binary_feature = cargo_toml
            .get("features")
            .and_then(|f| f.get("binary"))
            .is_some();

        let resolve_opt = project_metadata.resolve.as_ref();
        let resolve = resolve_opt.ok_or(BootloaderError::CargoMetadataIncomplete {
            key: "resolve".into(),
        })?;
        let bootloader_resolve = resolve
            .nodes
            .iter()
            .find(|n| n.id == bootloader_pkg.id)
            .ok_or(BootloaderError::CargoMetadataIncomplete {
                key: format!("resolve[\"{}\"]", bootloader_pkg.name),
            })?;
        let mut features = bootloader_resolve.features.clone();
        if binary_feature {
            features.push("binary".into());
        }

        let bootloader_name = &bootloader_pkg.name;
        let target_dir = project_metadata
            .target_directory
            .join("bootimage")
            .join(bootloader_name);

        Ok(BuildConfig {
            manifest_path: bootloader_pkg.manifest_path.clone(),
            target: bootloader_root.join(target_str),
            features,
            bootloader_name: bootloader_name.clone(),
            target_dir,
            kernel_manifest_path: kernel_pkg.manifest_path.clone(),
            kernel_bin_path: kernel_bin_path.to_owned(),
        })
    }

    /// Creates the cargo build command for building the bootloader.
    pub fn build_command(&self) -> Command {
        let cargo = std::env::var("CARGO").unwrap_or("cargo".to_owned());
        let mut cmd = Command::new(&cargo);
        cmd.arg("xbuild");
        cmd.arg("--manifest-path");
        cmd.arg(&self.manifest_path);
        cmd.arg("--bin").arg(&self.bootloader_name);
        cmd.arg("--target-dir").arg(&self.target_dir);
        cmd.arg("--features")
            .arg(self.features.as_slice().join(" "));
        cmd.arg("--target").arg(&self.target);
        cmd.arg("--release");
        cmd.env("KERNEL", &self.kernel_bin_path);
        cmd.env("KERNEL_MANIFEST", &self.kernel_manifest_path);
        cmd.env("RUSTFLAGS", "");
        cmd.env(
            "XBUILD_SYSROOT_PATH",
            self.target_dir.join("bootloader-sysroot"),
        ); // for cargo-xbuild
        cmd
    }
}

/// Returns the package metadata for the kernel crate
fn kernel_package<'a>(
    project_metadata: &'a Metadata,
    kernel_bin_name: &str,
) -> Result<&'a Package, BootloaderError> {
    let contains_bin = |p: &&Package| {
        p.targets
            .iter()
            .any(|t| t.name == kernel_bin_name && t.kind.iter().any(|k| k == "bin"))
    };
    let bin_metadata_opt = project_metadata.packages.iter().find(contains_bin);
    bin_metadata_opt.ok_or_else(|| BootloaderError::KernelBinPackageNotFound {
        bin_name: kernel_bin_name.to_owned(),
    })
}

/// Returns the package metadata for the bootloader crate
fn bootloader_package<'a>(
    project_metadata: &'a Metadata,
    kernel_package: &Package,
) -> Result<&'a Package, BootloaderError> {
    let bootloader_name = {
        let mut dependencies = kernel_package.dependencies.iter();
        let bootloader_package = dependencies
            .find(|p| p.rename.as_ref().unwrap_or(&p.name) == "bootloader")
            .ok_or(BootloaderError::BootloaderNotFound)?;
        bootloader_package.name.clone()
    };

    project_metadata
        .packages
        .iter()
        .find(|p| p.name == bootloader_name)
        .ok_or(BootloaderError::CargoMetadataIncomplete {
            key: format!("packages[name = `{}`", &bootloader_name),
        })
}
