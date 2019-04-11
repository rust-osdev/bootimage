#![no_std]
#![no_main]

#![feature(custom_test_frameworks)]
#![test_runner(runner_test::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::panic::PanicInfo;
use runner_test::{exit_qemu, ExitCode};

#[no_mangle]
pub extern "C" fn _start() -> ! {
    #[cfg(test)]
    test_main();

    unsafe { exit_qemu(ExitCode::Failed); }

    loop {}
}

#[test_case]
fn test1() {
    assert_eq!(0, 0);
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    unsafe { exit_qemu(ExitCode::Failed); }
    loop {}
}
