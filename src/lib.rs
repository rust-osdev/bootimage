use std::{
    fmt, fs, io,
    path::{Path, PathBuf},
    process::{self, Command},
};

pub struct Builder {
    kernel_manifest_path: PathBuf,
    kernel_metadata: cargo_metadata::Metadata,
}

impl Builder {
    pub fn new(manifest_path: Option<PathBuf>) -> Result<Self, BuilderError> {
        let kernel_manifest_path =
            manifest_path.unwrap_or(locate_cargo_manifest::locate_manifest()?);
        let kernel_metadata = cargo_metadata::MetadataCommand::new()
            .manifest_path(&kernel_manifest_path)
            .exec()?;
        Ok(Builder {
            kernel_manifest_path,
            kernel_metadata,
        })
    }

    pub fn kernel_manifest_path(&self) -> &Path {
        &self.kernel_manifest_path
    }

    pub fn kernel_root(&self) -> &Path {
        self.kernel_manifest_path
            .parent()
            .expect("kernel manifest has no parent directory")
    }

    pub fn kernel_metadata(&self) -> &cargo_metadata::Metadata {
        &self.kernel_metadata
    }

    pub fn kernel_package(&self) -> Result<&cargo_metadata::Package, String> {
        let mut packages = self.kernel_metadata.packages.iter();
        let kernel_package = packages.find(|p| &p.manifest_path == &self.kernel_manifest_path);
        kernel_package.ok_or(format!(
            "packages[manifest_path = `{}`]",
            &self.kernel_manifest_path.display()
        ))
    }

    pub fn build_kernel(&self, args: &[String], quiet: bool) -> Result<(), BuildKernelError> {
        if !quiet {
            println!("Building kernel");
        }

        let cargo = std::env::var("CARGO").unwrap_or("cargo".to_owned());
        let mut cmd = process::Command::new(cargo);
        cmd.arg("xbuild");
        cmd.args(args);
        if !quiet {
            cmd.stdout(process::Stdio::inherit());
            cmd.stderr(process::Stdio::inherit());
        }
        let output = cmd.output().map_err(|err| BuildKernelError::Io {
            message: "failed to execute kernel build",
            error: err,
        })?;;
        if !output.status.success() {
            let mut help_command = process::Command::new("cargo");
            help_command.arg("xbuild").arg("--help");
            help_command.stdout(process::Stdio::null());
            help_command.stderr(process::Stdio::null());
            if let Ok(help_exit_status) = help_command.status() {
                if !help_exit_status.success() {
                    return Err(BuildKernelError::XbuildNotFound);
                }
            }
            return Err(BuildKernelError::XbuildFailed {
                stderr: output.stderr,
            });
        }

        Ok(())
    }

    pub fn create_bootimage(
        &self,
        kernel_bin_path: &Path,
        output_bin_path: &Path,
        quiet: bool,
    ) -> Result<(), CreateBootimageError> {
        let metadata = self.kernel_metadata();

        let bootloader_name = {
            let kernel_package = self
                .kernel_package()
                .map_err(|key| CreateBootimageError::CargoMetadataIncomplete { key })?;
            let mut dependencies = kernel_package.dependencies.iter();
            let bootloader_package = dependencies
                .find(|p| p.rename.as_ref().unwrap_or(&p.name) == "bootloader")
                .ok_or(CreateBootimageError::BootloaderNotFound)?;
            bootloader_package.name.clone()
        };
        let target_dir = metadata
            .target_directory
            .join("bootimage")
            .join(&bootloader_name);

        let bootloader_pkg = metadata
            .packages
            .iter()
            .find(|p| p.name == bootloader_name)
            .ok_or(CreateBootimageError::CargoMetadataIncomplete {
                key: format!("packages[name = `{}`", &bootloader_name),
            })?;
        let bootloader_root = bootloader_pkg.manifest_path.parent().ok_or(
            CreateBootimageError::BootloaderInvalid(
                "bootloader manifest has no target directory".into(),
            ),
        )?;
        let bootloader_features =
            {
                let resolve = metadata.resolve.as_ref().ok_or(
                    CreateBootimageError::CargoMetadataIncomplete {
                        key: "resolve".into(),
                    },
                )?;
                let bootloader_resolve = resolve
                    .nodes
                    .iter()
                    .find(|n| n.id == bootloader_pkg.id)
                    .ok_or(CreateBootimageError::CargoMetadataIncomplete {
                    key: format!("resolve[\"{}\"]", bootloader_name),
                })?;
                bootloader_resolve.features.clone()
            };
        let bootloader_target_triple =
            default_target_triple_from_cargo_config(&bootloader_root, false)
                .map_err(CreateBootimageError::BootloaderInvalid)?
                .ok_or(CreateBootimageError::BootloaderInvalid(format!(
                    "bootloader must have a default target"
                )))?;

        // build bootloader
        if !quiet {
            println!("Building bootloader");
        }

        let cargo = std::env::var("CARGO").unwrap_or("cargo".to_owned());
        let mut cmd = process::Command::new(cargo);
        cmd.arg("xbuild");
        cmd.arg("--manifest-path");
        cmd.arg(&bootloader_pkg.manifest_path);
        cmd.arg("--target-dir").arg(&target_dir);
        cmd.arg("--features")
            .arg(bootloader_features.as_slice().join(" "));
        cmd.arg("--release");
        cmd.current_dir(bootloader_root);
        cmd.env("KERNEL", kernel_bin_path);
        cmd.env_remove("RUSTFLAGS");
        if !quiet {
            cmd.stdout(process::Stdio::inherit());
            cmd.stderr(process::Stdio::inherit());
        }
        let output = cmd.output().map_err(|err| CreateBootimageError::Io {
            message: "failed to execute bootloader build command",
            error: err,
        })?;
        if !output.status.success() {
            return Err(CreateBootimageError::BootloaderBuildFailed {
                stderr: output.stderr,
            });
        }

        let bootloader_elf_path = target_dir
            .join(&bootloader_target_triple)
            .join("release")
            .join(&bootloader_name);

        let llvm_tools = llvm_tools::LlvmTools::new()?;
        let objcopy = llvm_tools
            .tool(&llvm_tools::exe("llvm-objcopy"))
            .ok_or(CreateBootimageError::LlvmObjcopyNotFound)?;

        // convert bootloader to binary
        let mut cmd = Command::new(objcopy);
        cmd.arg("-I").arg("elf64-x86-64");
        cmd.arg("-O").arg("binary");
        cmd.arg("--binary-architecture=i386:x86-64");
        cmd.arg(&bootloader_elf_path);
        cmd.arg(&output_bin_path);
        let output = cmd.output().map_err(|err| CreateBootimageError::Io {
            message: "failed to execute llvm-objcopy command",
            error: err,
        })?;
        if !output.status.success() {
            return Err(CreateBootimageError::ObjcopyFailed {
                stderr: output.stderr,
            });
        }

        Ok(())
    }
}

pub fn default_target_triple_from_cargo_config(
    crate_root: &Path,
    walk_up: bool,
) -> Result<Option<String>, String> {
    let default_triple = default_target_from_cargo_config(crate_root, walk_up)?;
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

fn default_target_from_cargo_config(
    crate_root: &Path,
    walk_up: bool,
) -> Result<Option<(String, PathBuf)>, String> {
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

#[derive(Debug)]
pub enum BuilderError {
    /// Failed to locate cargo manifest
    LocateCargoManifest(locate_cargo_manifest::LocateManifestError),
    /// Error while running `cargo metadata`
    CargoMetadata(cargo_metadata::Error),
}

impl fmt::Display for BuilderError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            BuilderError::LocateCargoManifest(err) => writeln!(
                f,
                "Could not find Cargo.toml file starting from current folder: {:?}",
                err
            ),
            BuilderError::CargoMetadata(err) => writeln!(
                f,
                "Error while running `cargo metadata` for current project: {:?}",
                err
            ),
        }
    }
}

#[derive(Debug)]
pub enum BuildKernelError {
    /// Could not find kernel package in cargo metadata, required for retrieving kernel crate name
    KernelPackageNotFound,
    /// An unexpected I/O error occurred
    Io {
        /// Desciption of the failed I/O operation
        message: &'static str,
        /// The I/O error that occured
        error: io::Error,
    },
    XbuildNotFound,
    XbuildFailed {
        stderr: Vec<u8>,
    },
    CargoConfigInvalid {
        path: PathBuf,
        error: String,
    },
}

impl fmt::Display for BuildKernelError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            BuildKernelError::KernelPackageNotFound => {
                writeln!(f, "Could not find kernel package in cargo metadata, required for retrieving kernel crate name")
            }
            BuildKernelError::Io {message, error} => {
                writeln!(f, "I/O error: {}: {}", message, error)
            }
            BuildKernelError::XbuildNotFound => {
                writeln!(f, "Failed to run `cargo xbuild`. Perhaps it is not installed?\n\
                    Run `cargo install cargo-xbuild` to install it.")
            }
            BuildKernelError::XbuildFailed{stderr} => {
                writeln!(f, "Kernel build failed: {}", String::from_utf8_lossy(stderr))
            }
            BuildKernelError::CargoConfigInvalid{path,error} => {
                writeln!(f, "Failed to read cargo config at {}: {}", path.display(), error)
            },
        }
    }
}

#[derive(Debug)]
pub enum CreateBootimageError {
    /// Could not find some required information in the `cargo metadata` output
    CargoMetadataIncomplete {
        /// The required key that was not found
        key: String,
    },
    /// Bootloader dependency not found
    BootloaderNotFound,
    /// Bootloader dependency has not the right format
    BootloaderInvalid(String),
    BootloaderBuildFailed {
        stderr: Vec<u8>,
    },
    /// An unexpected I/O error occurred
    Io {
        /// Desciption of the failed I/O operation
        message: &'static str,
        /// The I/O error that occured
        error: io::Error,
    },
    /// There was a problem retrieving the `llvm-tools-preview` rustup component
    LlvmTools(llvm_tools::Error),
    /// The llvm-tools component did not contain the required `llvm-objcopy` executable
    LlvmObjcopyNotFound,
    /// The `llvm-objcopy` command failed
    ObjcopyFailed {
        stderr: Vec<u8>,
    },
}

impl fmt::Display for CreateBootimageError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CreateBootimageError::CargoMetadataIncomplete { key } => writeln!(
                f,
                "Could not find required key `{}` in cargo metadata output",
                key
            ),
            CreateBootimageError::BootloaderNotFound => {
                writeln!(f, "Bootloader dependency not found\n\n\
                    You need to add a dependency on a crate named `bootloader` in your Cargo.toml.")
            }
            CreateBootimageError::BootloaderInvalid(err) => writeln!(
                f,
                "The `bootloader` dependency has not the right format: {}",
                err
            ),
            CreateBootimageError::BootloaderBuildFailed { stderr } => writeln!(
                f,
                "Bootloader build failed:\n\n{}",
                String::from_utf8_lossy(stderr)
            ),
            CreateBootimageError::Io { message, error } => {
                writeln!(f, "I/O error: {}: {}", message, error)
            }
            CreateBootimageError::LlvmTools(err) => match err {
                llvm_tools::Error::NotFound => writeln!(
                    f,
                    "Could not find the `llvm-tools-preview` rustup component.\n\n\
                     You can install by executing `rustup component add llvm-tools-preview`."
                ),
                err => writeln!(
                    f,
                    "Failed to locate the `llvm-tools-preview` rustup component: {:?}",
                    err
                ),
            },
            CreateBootimageError::LlvmObjcopyNotFound => writeln!(
                f,
                "Could not find `llvm-objcopy` in the `llvm-tools-preview` rustup component."
            ),
            CreateBootimageError::ObjcopyFailed { stderr } => writeln!(
                f,
                "Failed to run `llvm-objcopy`: {}",
                String::from_utf8_lossy(stderr)
            ),
        }
    }
}

// from implementations

impl From<locate_cargo_manifest::LocateManifestError> for BuilderError {
    fn from(err: locate_cargo_manifest::LocateManifestError) -> Self {
        BuilderError::LocateCargoManifest(err)
    }
}

impl From<cargo_metadata::Error> for BuilderError {
    fn from(err: cargo_metadata::Error) -> Self {
        BuilderError::CargoMetadata(err)
    }
}

impl From<llvm_tools::Error> for CreateBootimageError {
    fn from(err: llvm_tools::Error) -> Self {
        CreateBootimageError::LlvmTools(err)
    }
}
