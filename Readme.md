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
bootloader = "0.5.0"
```

**Note**: At least bootloader version `0.5.0` is required.

If you want to use a custom bootloader with a different name, you can use Cargo's [rename functionality](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#renaming-dependencies-in-cargotoml).

### Building

Now you can build the kernel project and create a bootable disk image from it by running:

```
bootimage build --target your_custom_target.json [other_args]
```

The command will invoke [`cargo xbuild`](https://github.com/rust-osdev/cargo-xbuild), forwarding all passed options. Then it will build the specified bootloader together with the kernel to create a bootable disk image.

If you prefer a cargo subcommand, you can use the equivalent `cargo bootimage` command:

```
cargo bootimage --target your_custom_target.json [other_args]
```

### Running

To run your kernel in QEMU, you can use `bootimage run`:

```
bootimage run --target your_custom_target.json [other_args] -- [qemu args]
```

All arguments after `--` are passed to QEMU. If you want to use a custom run command, see the _Configuration_ section below.

If you prefer working directly with cargo, you can use `bootimage runner` as a custom runner in your `.cargo/config`:

```toml
[target.'cfg(target_os = "none")']
runner = "bootimage runner"
```

Now you can run your kernel through `cargo xrun --target […]`.

## Configuration

Configuration is done through a through a `[package.metadata.bootimage]` table in the `Cargo.toml` of your kernel. The following options are available:

```toml
    [package.metadata.bootimage]
    # This target is used if no `--target` is passed
    default-target = ""

    # The command invoked on `bootimage run` or `bootimage runner`
    # (the "{}" will be replaced with the path to the bootable disk image)
    run-command = ["qemu-system-x86_64", "-drive", "format=raw,file={}"]

    # Additional arguments passed to the runner on `bootimage run` or `bootimage runner`
    # (this is useful when you want to add some arguments to the default QEMU command)
    run-args = []

    # The timeout for running an integration test through `bootimage test` in seconds
    test-timeout = 300
```

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
