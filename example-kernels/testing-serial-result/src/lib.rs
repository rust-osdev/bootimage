#![cfg_attr(not(test), no_std)]
#![feature(abi_x86_interrupt)]

pub unsafe fn exit_qemu() {
    use x86_64::instructions::port::Port;

    let mut port = Port::<u32>::new(0xf4);
    port.write(0);
}

pub fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}

pub mod serial {
    use lazy_static::lazy_static;
    use spin::Mutex;
    use uart_16550::SerialPort;

    lazy_static! {
        pub static ref SERIAL1: Mutex<SerialPort> = {
            let mut serial_port = SerialPort::new(0x3F8);
            serial_port.init();
            Mutex::new(serial_port)
        };
    }

    #[doc(hidden)]
    pub fn _print(args: ::core::fmt::Arguments) {
        use core::fmt::Write;
        use x86_64::instructions::interrupts;

        interrupts::without_interrupts(|| {
            SERIAL1
                .lock()
                .write_fmt(args)
                .expect("Printing to serial failed");
        });
    }

    /// Prints to the host through the serial interface.
    #[macro_export]
    macro_rules! serial_print {
        ($($arg:tt)*) => {
            $crate::serial::_print(format_args!($($arg)*));
        };
    }

    /// Prints to the host through the serial interface, appending a newline.
    #[macro_export]
    macro_rules! serial_println {
        () => ($crate::serial_print!("\n"));
        ($fmt:expr) => ($crate::serial_print!(concat!($fmt, "\n")));
        ($fmt:expr, $($arg:tt)*) => ($crate::serial_print!(concat!($fmt, "\n"), $($arg)*));
    }
}