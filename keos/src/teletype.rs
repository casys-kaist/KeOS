//! A teletype (TTY) interface for character-based I/O.
//!
//! This module provides a trait [`Teletype`] that defines an interface for
//! reading from and writing to a teletype device, such as a serial port.
//! The [`Serial`] struct implements this interface for x86_64 systems.

use crate::{KernelError, spinlock::SpinLock, thread::with_current};

/// The `Teletype` trait represents a generic character-based input/output
/// device.
///
/// Implementations of this trait define methods for:
/// - Writing data to the teletype (`write`)
/// - Reading data from the teletype (`read`)
///
/// This abstraction allows for different kinds of terminal or serial interfaces
/// to implement their own communication methods.
pub trait Teletype {
    /// Writes data to the teletype.
    ///
    /// # Arguments
    /// - `data`: A byte slice containing the data to be written.
    ///
    /// # Returns
    /// - `Ok(usize)`: The number of bytes successfully written.
    /// - `Err(KernelError)`: If the write operation failed.
    fn write(&mut self, data: &[u8]) -> Result<usize, KernelError>;

    /// Reads data from the teletype.
    ///
    /// # Arguments
    /// - `data`: A mutable byte slice where the read data will be stored.
    ///
    /// # Returns
    /// - `Ok(usize)`: The number of bytes successfully read.
    /// - `Err(KernelError)`: If the read operation failed.
    fn read(&mut self, data: &mut [u8]) -> Result<usize, KernelError>;
}

/// A serial teletype interface for x86_64 systems.
///
/// This struct provides a basic implementation of a serial TTY using the
/// **COM1** serial port. It implements the [`Teletype`] trait to allow
/// read and write operations over a serial interface.
pub struct Serial {
    _p: (),
}

impl Serial {
    /// Creates a new **COM1** serial interface instance.
    ///
    /// This function initializes a serial TTY for performing character-based
    /// I/O operations. The actual hardware interaction is handled via the
    /// [`Teletype`] trait methods (`write` and `read`).
    ///
    /// # Returns
    /// - A new instance of `Serial`, representing a COM1 serial interface.
    pub const fn new() -> Self {
        Self { _p: () }
    }
}

impl Default for Serial {
    fn default() -> Self {
        Self::new()
    }
}

/// A global serial device protected by a spinlock.
///
/// This static instance of [`Serial`] ensures safe concurrent access to the
/// serial port. It is wrapped in a [`SpinLock`] to provide mutual exclusion,
/// preventing race conditions when multiple threads attempt to write to or read
/// from the serial device.
///
/// The [`Serial`] struct typically represents a UART (Universal Asynchronous
/// Receiver-Transmitter) device used for debugging, logging, or kernel output.
///
/// # Safety
/// - Accessing this global requires acquiring the spinlock before modifying the
///   serial state.
/// - Since [`SpinLock`] is used instead of [`Mutex`], it should **only be used
///   in environments without preemption**, such as kernel mode, to avoid
///   deadlocks.
///
/// [`Mutex`]: struct.Mutex.html
static SERIAL: SpinLock<Serial> = SpinLock::new(Serial::new());

/// Returns a reference to the global serial device.
///
/// This function provides safe access to the global serial interface wrapped in
/// a [`SpinLock`]. Users must lock the spinlock before performing any
/// operations on the [`Serial`] instance.
///
/// # Example
/// ```
/// use keos::teletype::Teletype;
///
/// let serial = serial().lock();
/// serial.write("Hello, serial output!").expect("Failed to write tty");
/// ```
///
/// # Safety
/// - Since this returns a reference to a global [`SpinLock`], the caller must
///   **ensure proper locking** before accessing the [`Serial`] device.
///
/// # Returns
/// A reference to the [`SpinLock`] wrapping the global [`Serial`] instance.
pub fn serial() -> &'static SpinLock<Serial> {
    &SERIAL
}

impl Teletype for Serial {
    /// Writes data to the serial teletype (COM1).
    ///
    /// This function attempts to convert the input byte slice into a UTF-8
    /// string. If the conversion is successful, it prints the string to
    /// the console. If the data is aligned to a **16-byte boundary**, it
    /// is printed as a single string; otherwise, it is printed byte by byte.
    ///
    /// # Arguments
    /// - `data`: The byte slice to be written.
    ///
    /// # Returns
    /// - `Ok(usize)`: The number of bytes written.
    /// - `Err`: If the input data is not valid UTF-8.
    fn write(&mut self, data: &[u8]) -> Result<usize, KernelError> {
        with_current(|th| {
            let b = if data.as_ptr().is_aligned_to(8) {
                if let Ok(b) = core::str::from_utf8(data) {
                    print!("{}", b);
                    Ok(data.len())
                } else {
                    Err(KernelError::InvalidArgument)
                }
            } else {
                for b in data {
                    print!("{}", b);
                }
                Ok(data.len())
            };
            let mut tty_hook = th.tty_hook.lock();
            let val = match tty_hook.as_mut() {
                Some(ttyhook) => {
                    let mut guard = ttyhook.lock();
                    let val = guard.write(data);
                    guard.unlock();
                    val
                }
                _ => b,
            };
            tty_hook.unlock();
            val
        })
    }

    /// Reads data from the serial teletype (COM1).
    ///
    /// This function retrieves data from the serial interface and stores it
    /// in the provided mutable buffer.
    ///
    /// # Arguments
    /// - `data`: A mutable byte slice where the read data will be stored.
    ///
    /// # Returns
    /// - `Ok(usize)`: The number of bytes successfully read.
    /// - `Err`: If the read operation failed.
    fn read(&mut self, data: &mut [u8]) -> Result<usize, KernelError> {
        with_current(|th| {
            let mut tty_guard = th.tty_hook.lock();

            let val = match tty_guard.as_mut() {
                Some(ttyhook) => {
                    let mut guard = ttyhook.lock();
                    let val = guard.read(data);
                    guard.unlock();
                    val
                }
                _ => abyss::dev::x86_64::serial::read_bytes_busywait(data)
                    .ok_or(KernelError::IOError),
            };
            tty_guard.unlock();
            val
        })
    }
}
