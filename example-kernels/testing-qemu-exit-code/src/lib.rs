#![cfg_attr(not(test), no_std)]
#![feature(abi_x86_interrupt)]

#[repr(u32)]
pub enum ExitCode {
    Success = 2,
    Failure = 3,
}

pub unsafe fn exit_qemu(exit_code: ExitCode) {
    use x86_64::instructions::port::Port;

    let mut port = Port::<u32>::new(0xf4);
    port.write(exit_code as u32);
}

pub fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}
