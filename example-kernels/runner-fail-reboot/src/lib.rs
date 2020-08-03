#![no_std]
#![cfg_attr(test, no_main)]
#![feature(custom_test_frameworks)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate rlibc;

pub fn test_runner(tests: &[&dyn Fn()]) {
    for test in tests.iter() {
        test();
    }

    unsafe {
        exit_qemu(ExitCode::Success);
    }
}

#[test_case]
fn should_reboot() {
    // this overflows the stack which leads to a triple fault
    // the as-if rule might allow this to get optimized away on release builds
    #[allow(unconditional_recursion)]
    fn stack_overflow() {
        stack_overflow()
    }
    stack_overflow()
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

#[cfg(test)]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe {
        exit_qemu(ExitCode::Failed);
    }
    loop {}
}
