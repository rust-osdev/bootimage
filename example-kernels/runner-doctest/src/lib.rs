#![no_std]
#![cfg_attr(test, no_main)]
#![feature(custom_test_frameworks)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]

/// add two numbers
///
/// ```
/// #![no_std]
/// #![no_main]
/// use runner_doctest::{add, exit_qemu, ExitCode};
/// #[export_name = "_start"]
/// extern "C" fn start() {
///     assert_eq!(add(1, 2), 3);
///     unsafe { exit_qemu(ExitCode::Success); }
/// }
/// ```
pub fn add(a: u32, b: u32) -> u32 {
    a + b
}

/// multiply two numbers
///
/// ```
/// #![no_std]
/// #![no_main]
/// use runner_doctest::{mul, exit_qemu, ExitCode};
/// #[export_name = "_start"]
/// extern "C" fn start() {
///     assert_eq!(mul(2, 3), 6);
///     unsafe { exit_qemu(ExitCode::Success); }
/// }
/// ```
pub fn mul(a: u32, b: u32) -> u32 {
    a * b
}

#[cfg(test)]
fn test_runner(tests: &[&dyn Fn()]) {
    for test in tests.iter() {
        test();
    }

    unsafe {
        exit_qemu(ExitCode::Success);
    }
}

pub enum ExitCode {
    Success,
    Failed,
}

impl ExitCode {
    fn code(&self) -> u32 {
        match self {
            ExitCode::Success => 0x10,
            ExitCode::Failed => 0x11,
        }
    }
}

/// exit QEMU (see https://os.phil-opp.com/integration-tests/#shutting-down-qemu)
pub unsafe fn exit_qemu(exit_code: ExitCode) {
    use x86_64::instructions::port::Port;

    let mut port = Port::<u32>::new(0xf4);
    port.write(exit_code.code());
}

#[cfg(test)]
#[no_mangle]
pub extern "C" fn _start() -> ! {
    test_main();

    unsafe {
        exit_qemu(ExitCode::Failed);
    }

    loop {}
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe {
        exit_qemu(ExitCode::Failed);
    }
    loop {}
}
