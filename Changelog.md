
- New features for `bootimage runner`
    - Pass additional arguments to the run command (e.g. QEMU)
    - Consider all binaries in the `target/deps` folder as test executables
    - Apply `test-timeout` config key when running tests in `bootimage runner`
    - Don't apply `run-args` for test executables
    - Add a new `test-args` config key for test arguments
    - Add a new `test-success-exit-code` config key for interpreting an exit code as success
        - This is useful when the `isa-debug-exit` QEMU device is used.
    - Improve printing of the run command (print string instead of array, print non-canonicalized executable path, respect `--quiet`)

# 0.7.1

- Fix for backwards compatibility: Ignore `test-` executables for `bootimage run`.
    - This ensures that `bootimage run` still works without the need for a `--bin` argument if all other executables are integration tests.
    - This only changes the default, you can still run test executables by passing `--bin test-.`

# 0.7.0

## Breaking

- Rewrite for new bootloader build system
  - Compatible with bootloader 0.5.1+
- Remove the following config options: `output`, `bootloader.*`, `minimum_image_size`, and `package_filepath`
  - The bootloader is now fully controlled through cargo dependencies.
  - For using a bootloader crate with name different than `bootloader` use [cargo's rename feature](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#renaming-dependencies-in-cargotoml).
- Remove support for `bootloader_precompiled`
  - The `bootloader` crate compiles fine on all architectures for some time and should be prefered
- Require the `llvm-tools-preview` rustup component
- Pass the QEMU exit code in `bootimage run`

## Other

- Add support for default targets declared in `.cargo/config` files
- Add a `cargo-bootimage` executable that is equivalent to `bootimage build` and can be used as cargo subcommand (`cargo bootimage`)
- Add a new `bootimage runner` subcommand that can be used as `target.[…].runner` in `.cargo/config` files
- Make test timeout configurable and increase default to 5 minutes
- Move crate to 2018 edition
- Refactor and cleanup the code
- Remove the dependency on `failure`
    - Use a custom `ErrorMessage` type instead
- Add a new `run-args` config key
- Add a new `--quiet` argument to suppress output

# 0.6.6

- Update dependencies

# 0.6.5

- You can now mark integration tests as success/failure by setting the exit code in the QEMU `isa-debug-exit` device. See [#32](https://github.com/rust-osdev/bootimage/issues/32) for more information.

# 0.6.4

- Canonicalize paths before comparing them when invoking `bootimage test`
  - This caused an error on Windows where the path in the cargo metadata is not fully canonicalized
- Improve CI infrastructure

# 0.6.3

- Canonicalize paths before comparing them when invoking `bootimage build`
  - This caused an error on Windows where the path in the cargo metadata is not fully canonicalized

# 0.6.2

- Fix build on Windows (don't use the `.` directory)

# 0.6.1

- Fix: bootimage should now work correctly with `--manifest-path`

# 0.6.0

(Yanked from crates.io because of a bug fixed in 0.6.1.)

**Breaking**:

- When no `--manifest-path` argument is passed, `bootimage` defaults to the `Cargo.toml` in the current directory instead of the workspace root.
  - This fixes compilation of projects that are part of a workspace
