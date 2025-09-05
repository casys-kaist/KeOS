//! Serial device driver.
use crate::spinlock::SpinLock;

use crate::x86_64::pio::Pio;

/// Initialize a serial.
pub unsafe fn init() {
    Pio::new(0x3f8 + 2).write_u8(0);
    Pio::new(0x3f8 + 3).write_u8(0x80);
    Pio::new(0x3f8).write_u8((115200 / 9600) as u8);
    Pio::new(0x3f8 + 1).write_u8(0);
    Pio::new(0x3f8 + 3).write_u8(0x3 & !0x80);
    Pio::new(0x3f8 + 4).write_u8(0);
    Pio::new(0x3f8 + 1).write_u8(1);
    Pio::new(0x3f8 + 2).read_u8();
    Pio::new(0x3f8).read_u8();
}

pub(crate) fn write_str(s: &str) {
    for b in s.as_bytes() {
        for _ in 0..12800 {
            if Pio::new(0x3f8 + 5).read_u8() & 0x20 != 0 {
                break;
            }
            // delay
            Pio::new(0x84).read_u8();
            Pio::new(0x84).read_u8();
            Pio::new(0x84).read_u8();
            Pio::new(0x84).read_u8();
        }
        Pio::new(0x3f8).write_u8(*b);
    }
}

static RB_SPINLOCK: SpinLock<()> = SpinLock::new(());
pub fn read_bytes_busywait(buffer: &mut [u8]) -> Option<usize> {
    if let Ok(rb_lock) = RB_SPINLOCK.try_lock() {
        let mut count = 0;

        // Continue reading bytes until the buffer is full or no more data is available.
        while count < buffer.len()
        /* && (Pio::new(0x3f8 + 5).read_u8() & 0x01 != 0) */
        {
            while Pio::new(0x3f8 + 5).read_u8() & 0x01 == 0 {}

            let byte = Pio::new(0x3f8).read_u8();
            if byte != 0x7F && byte != 0x04 {
                // Ctrl+D = EOT. Stop reading immediately.
                print!("{}", byte as char);
                buffer[count] = byte;
                count += 1;
            } else if byte == 0x7F {
                count -= 1;
                buffer[count] = 0;
                print!("{} {}", 0x8 as char, 0x8 as char);
            } else {
                break;
            }

            if byte == 0x0A || byte == 0x0D {
                // newline = Save newline and stop reading.
                break;
            }
        }

        rb_lock.unlock();
        Some(count)
    } else {
        None
    }
}

pub struct Com1Sink {
    _p: (),
}

impl Com1Sink {
    /// Create a new serial device interface.
    pub const fn new() -> Self {
        Com1Sink { _p: () }
    }
}

impl core::fmt::Write for Com1Sink {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        write_str(s);
        Ok(())
    }
}

impl Default for Com1Sink {
    fn default() -> Self {
        Self::new()
    }
}
