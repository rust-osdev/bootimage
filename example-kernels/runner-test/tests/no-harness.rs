#![no_std]
#![no_main]

use runner_test::{exit_qemu, ExitCode};
use core::panic::PanicInfo;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    unsafe {
        exit_qemu(ExitCode::Success);
    }
    loop {}
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    unsafe {
        exit_qemu(ExitCode::Failed);
    }
    loop {}
}
