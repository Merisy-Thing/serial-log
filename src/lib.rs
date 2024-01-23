#![no_std]


#[cfg(not(feature = "print-mute"))]
#[macro_export]
macro_rules! println {
    ($($arg:tt)*) => {{
        {
            use core::fmt::Write;
            writeln!($crate::Printer, $($arg)*).ok();
        }
    }};
}

#[cfg(not(feature = "print-mute"))]
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {{
        {
            use core::fmt::Write;
            write!($crate::Printer, $($arg)*).ok();
        }
    }};
}

#[cfg(feature = "print-mute")]
#[macro_export]
macro_rules! println {
    ($($arg:tt)*) => {{}};
}

#[cfg(feature = "print-mute")]
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {{}};
}

#[cfg(feature = "defmt-log")]
mod defmt_log {
    use super::Printer;

    //static mut ENCODER: defmt::Encoder = defmt::Encoder::new();

    #[defmt::global_logger]
    pub struct Logger;
    
    unsafe impl defmt::Logger for Logger {
        fn acquire() {
            
        }

        unsafe fn release() {
            
        }

        unsafe fn flush() {
            Printer.flush();
        }

        unsafe fn write(data: &[u8]) {
            // ENCODER.write(data, |bytes: &[u8]| {
            //     Printer.write_bytes_assume_cs(bytes);
            // });
            Printer.write_bytes_assume_cs(data);
        }
    }
}

pub struct Printer;

impl core::fmt::Write for Printer {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        Printer.write_bytes(s.as_bytes());
        Ok(())
    }
}

impl Printer {
    pub fn write_bytes(&mut self, bytes: &[u8]) {
        self.write_bytes_assume_cs(bytes);
        self.flush();
    }
}

#[cfg(feature = "device-esp32c3")]
mod uart_printer {
    trait Functions {
        const TX_ONE_CHAR: usize;
        const CHUNK_SIZE: usize = 32;

        fn tx_byte(b: u8) {
            unsafe {
                let tx_one_char: unsafe extern "C" fn(u8) -> i32 =
                    core::mem::transmute(Self::TX_ONE_CHAR);
                tx_one_char(b);
            }
        }

        fn flush();
    }

    struct Device;

    impl Functions for Device {
        const TX_ONE_CHAR: usize = 0x4000_0068;

        fn flush() {
            unsafe {
                const TX_FLUSH: usize = 0x4000_0080;
                const GET_CHANNEL: usize = 0x4000_058C;
                let tx_flush: unsafe extern "C" fn(u8) = core::mem::transmute(TX_FLUSH);
                let get_channel: unsafe extern "C" fn() -> u8 = core::mem::transmute(GET_CHANNEL);

                const G_USB_PRINT_ADDR: usize = 0x3FCD_FFD0;
                let g_usb_print = G_USB_PRINT_ADDR as *mut bool;

                let channel = if *g_usb_print {
                    // Flush USB-JTAG
                    3
                } else {
                    get_channel()
                };
                tx_flush(channel);
            }
        }
    }

    impl super::Printer {
        pub fn write_bytes_assume_cs(&mut self, bytes: &[u8]) {
            for chunk in bytes.chunks(Device::CHUNK_SIZE) {
                for &b in chunk {
                    Device::tx_byte(b);
                }

                Device::flush();
            }
        }

        pub fn flush(&mut self) {}
    }
}

#[cfg(any(feature = "device-hk32f0301mxxc-uart1", feature = "device-hk32f0301mxxc-uart2"))]
mod uart_printer {
    use core::ptr;

    struct Device;
    #[cfg(feature = "device-hk32f0301mxxc-uart1")]
    const UART_BASE: u32 = 0x40013800;

    #[cfg(feature = "device-hk32f0301mxxc-uart2")]
    const UART_BASE: u32 = 0x40013C00;

    const OFFSET_ISR: u32 = 0x1C;
    const OFFSET_TDR: u32 = 0x28;
    const P_UART_ISR: *const u32 = (UART_BASE + OFFSET_ISR) as *const u32;
    const P_UART_TDA: *mut u32 = (UART_BASE + OFFSET_TDR) as *mut u32;
    const UART_ISR_TXE: u32 = 0x01_u32 << 7;

    impl Device {

        fn tx_byte(b: u8) {
            loop {
                //Wait transmit data register empty before write
                let isr = unsafe { ptr::read_volatile( P_UART_ISR ) };

                if (isr & UART_ISR_TXE) != 0 { 
                    unsafe { ptr::write_volatile(P_UART_TDA, b as u32) };
                    break;
                }
            }
        }

        fn flush() {

        }
    }

    impl super::Printer {
        pub fn write_bytes_assume_cs(&mut self, bytes: &[u8]) {
            for byte in bytes {
                Device::tx_byte(*byte);
                Device::flush();
            }
        }

        pub fn flush(&mut self) {}
    }
}
