//! Kernel print utilities.

use crate::dev::x86_64::serial::Com1Sink;
use crate::spinlock::SpinLock;
use core::fmt::Write;

// Only mutated when force unlocking is required (i.e. panicking)
static mut SERIAL: SpinLock<Com1Sink> = SpinLock::new(Com1Sink::new());

#[doc(hidden)]
#[unsafe(no_mangle)]
/// Safety: Serial only mutated when force unlocking is required (i.e.
/// panicking)
pub fn _print(fmt: core::fmt::Arguments<'_>) {
    let mut guard = unsafe { SERIAL.lock() };
    let _ = write!(&mut *guard, "{fmt}");
    guard.unlock();
}

/// Force Unlocking Serial.
///
/// Do NOT use this API.
#[doc(hidden)]
pub unsafe fn force_unlock_serial() {
    unsafe {
        SERIAL = SpinLock::new(Com1Sink::new());
    }
}

/// Prints out the message.
///
/// Use the format! syntax to write data to the standard output.
/// This first holds the lock for console device.
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::kprint::_print(format_args!($($arg)*)));
}

/// Prints out the message with a newline.
///
/// Use the format! syntax to write data to the standard output.
/// This first holds the lock for console device.
#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

/// Display an information message.
///
/// Use the format! syntax to write data to the standard output.
/// This first holds the lock for console device.
#[macro_export]
macro_rules! info {
    () => (if !$crate::QUITE.load(core::sync::atomic::Ordering::SeqCst) { $crate::print!("[INFO]\n") });
    ($($arg:tt)*) => (if !$crate::QUITE.load(core::sync::atomic::Ordering::SeqCst) { $crate::print!("[INFO] {}\n", format_args!($($arg)*)) });
}

/// Display a warning message.
///
/// Use the format! syntax to write data to the standard output.
/// This first holds the lock for console device.
#[macro_export]
macro_rules! warning {
    () => (if !$crate::QUITE.load(core::sync::atomic::Ordering::SeqCst) { $crate::print!("[WARN]\n") });
    ($($arg:tt)*) => (if !$crate::QUITE.load(core::sync::atomic::Ordering::SeqCst) { $crate::print!("[WARN] {}\n", format_args!($($arg)*)) });
}

/// Display a debug message.
///
/// Use the format! syntax to write data to the standard output.
/// This first holds the lock for console device.
#[macro_export]
macro_rules! debug {
    () => (if !$crate::QUITE.load(core::sync::atomic::Ordering::SeqCst) { $crate::print!("[DEBUG]\n") });
    ($($arg:tt)*) => (if !$crate::QUITE.load(core::sync::atomic::Ordering::SeqCst) { $crate::print!("[DEBUG] {}\n", format_args!($($arg)*))} );
}
