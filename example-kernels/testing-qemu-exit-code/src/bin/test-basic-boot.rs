#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)] // disable all Rust-level entry points
#![cfg_attr(test, allow(unused_imports))]

use testing_qemu_exit_code::{exit_qemu, ExitCode};
use core::panic::PanicInfo;

/// This function is the entry point, since the linker looks for a function
/// named `_start` by default.
#[cfg(not(test))]
#[no_mangle] // don't mangle the name of this function
pub extern "C" fn _start() -> ! {
    unsafe {
        exit_qemu(ExitCode::Success);
    }
    loop {}
}

/// This function is called on panic.
#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    unsafe {
        exit_qemu(ExitCode::Failure);
    }
    loop {}
}
