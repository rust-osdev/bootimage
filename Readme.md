# bootimage

Creates a bootable disk image from a Rust OS kernel.

## Installation

```
> cargo install bootimage
```

## Usage

First you need to add a dependency on the [`bootloader`](https://github.com/rust-osdev/bootloader) crate:

```toml
# in your Cargo.toml

[dependencies]
bootloader = "0.6.4"
```

**Note**: At least bootloader version `0.5.1` is required since `bootimage 0.7.0`. For earlier bootloader versions, use `bootimage 0.6.6`.

If you want to use a custom bootloader with a different name, you can use Cargo's [rename functionality](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#renaming-dependencies-in-cargotoml).

### Building

Now you can build the kernel project and create a bootable disk image from it by running:

```
cargo bootimage --target your_custom_target.json [other_args]
```

The command will invoke `cargo build`, forwarding all passed options. Then it will build the specified bootloader together with the kernel to create a bootable disk image.

### Running

To run your kernel in QEMU, you can set a `bootimage runner` as a custom runner in a `.cargo/config` file:

```toml
[target.'cfg(target_os = "none")']
runner = "bootimage runner"
```

Then you can run your kernel through:

```
cargo xrun --target your_custom_target.json [other_args] -- [qemu args]
```

All arguments after `--` are passed to QEMU. If you want to use a custom run command, see the _Configuration_ section below.

### Testing

The `bootimage` has built-in support for running unit and integration tests of your kernel. For this, you need to use the `custom_tests_framework` feature of Rust as described [here](https://os.phil-opp.com/testing/#custom-test-frameworks).

## Configuration

Configuration is done through a through a `[package.metadata.bootimage]` table in the `Cargo.toml` of your kernel. The following options are available:

```toml
[package.metadata.bootimage]
# The cargo subcommand that will be used for building the kernel.
#
# For building using the `cargo-xbuild` crate, set this to `xbuild`.
build-command = ["build"]
# The command invoked with the created bootimage (the "{}" will be replaced
# with the path to the bootable disk image)
# Applies to `bootimage run` and `bootimage runner`
run-command = ["qemu-system-x86_64", "-drive", "format=raw,file={}"]

# Additional arguments passed to the run command for non-test executables
# Applies to `bootimage run` and `bootimage runner`
run-args = []

# Additional arguments passed to the run command for test executables
# Applies to `bootimage runner`
test-args = []

# An exit code that should be considered as success for test executables
test-success-exit-code = {integer}

# The timeout for running a test through `bootimage test` or `bootimage runner` (in seconds)
test-timeout = 300

# Whether the `-no-reboot` flag should be passed to test executables
test-no-reboot = true
```

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
