#![no_std] // don't link the Rust standard library
#![no_main] // disable all Rust-level entry points

#![feature(custom_test_frameworks)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::panic::PanicInfo;
use runner_test::{exit_qemu, ExitCode};

#[no_mangle] // don't mangle the name of this function
pub extern "C" fn _start() -> ! {
    test_main();

    unsafe { exit_qemu(ExitCode::Failed); }

    loop {}
}

pub fn test_runner(tests: &[&dyn Fn()]) {
    for test in tests.iter() {
        test();
    }
}

#[test_case]
fn should_panic() {
    assert_eq!(1, 2);
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    unsafe { exit_qemu(ExitCode::Success); }
    loop {}
}
