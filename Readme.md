# bootimage

Creates a bootable disk image from a Rust OS kernel.

## Installation

```
> cargo install bootimage
```

## Usage

To build the kernel project and create a bootable disk image from it, run:

```
> bootimage build --target your_custom_target [other_args]
```

The command will invoke [`cargo xbuild`](https://github.com/rust-osdev/cargo-xbuild), forwarding all passed options. Then it will download and build a bootloader, by default the [rust-osdev/bootloader](https://github.com/rust-osdev/bootloader). Finally, it combines the kernel and the bootloader into a bootable disk image.

## Configuration

Configuration is done through a through a `[package.metadata.bootimage]` table in the `Cargo.toml`. The following options are available:

```toml
    [package.metadata.bootimage]
    default-target = ""         # This target is used if no `--target` is passed
    output = "bootimage.bin"    # The output file name
    minimum-image-size = 0      # The minimum output file size (in MiB)
    # The command invoked on `bootimage run`
    # (the "{}" will be replaced with the path to the bootable disk image)
    run-command = ["qemu-system-x86_64", "-drive", "format=raw,file={}"]

    [package.metadata.bootimage.bootloader]
    name = "bootloader"             # The bootloader crate name
    version = ""                    # The bootloader version that should be used
    git = ""                        # Use the bootloader from this git repository
    branch = ""                     # The git branch to use (defaults to master)
    path = ""                       # Use the bootloader from this local path
    precompiled = false             # Whether the bootloader crate is precompiled
    target = "x86_64-bootloader"    # Target triple for compiling the bootloader
```

If no `[package.metadata.bootimage.bootloader]` sub-table is specified, it defaults to:

```toml
name = "bootloader_precompiled"
precompiled = true
```

## License
Dual-licensed under MIT or the Apache License (Version 2.0).
