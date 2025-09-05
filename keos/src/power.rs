//! Modules for system power operations.
/// Restart the machine.
pub fn restart() -> ! {
    unsafe {
        abyss::x86_64::power_control::restart();
    }
}

/// Shutdown the machine.
pub fn shutdown() -> ! {
    unsafe {
        abyss::x86_64::power_control::power_off();
    }
}
