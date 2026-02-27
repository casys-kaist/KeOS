//! MMIO based simple printer device.
//!
//! ## Background
//! Memory-mapped I/O uses the same address space for both main memory and I/O
//! devices. Unlike PIO, which uses dedicated instructions, in MMIO, one can
//! think of the memory and register of I/O devices as if they are mapped in a
//! specific address of main memory. This method allows for the convenient and
//! fast definition of I/O device behavior and concise implementation of the CPU
//! and I/O devices.
//!
//! In memory-mapped I/O, the same memory address space is used for accessing
//! both main memory (RAM) and I/O devices. Commands for accessing main memory
//! (such as load and store) are also used to access devices, reading from and
//! writing to memory instead. For instance, if there exists memory-mapped I/O
//! (MMIO) mapping between a console device and CPU at address 0x00ff, writing
//! 'a' to the 0x00ff will print a letter 'a' to the console device.
//!
//! The specific area of memory for these operations can be temporarily agreed
//! upon between a CPU and device or it can be a permanently assigned area.
//! As an example, modern PCIe devices negotiate the locations of the MMIO
//! sections with a CPU by using a base address register (BAR) in the
//! initialization process.
//!
//! On modern CPUs, MMIO is performed by the memory management unit (MMU), which
//! is a hardware component responsible for managing memory access.
//! When a program running on the CPU performs an I/O operation on a
//! memory-mapped location, the MMU intercepts the memory access and sends it to
//! the appropriate hardware device, which performs the requested operation.
//! Each I/O device monitors the CPU's address bus and when the memory is
//! accessed, the device executes the command and writes the result to a
//! specific memory location or performs the command.
//!
//! Memory-mapped I/O is generally faster than Programmed I/O (PIO) because it
//! avoids the overhead of the processor having to manage the I/O operations
//! directly. In PIO, the CPU should execute IN and OUT operations to
//! communicate with peripheral devices. Which causes delays and limits overall
//! system performance. In contrast, MMIO allows I/O devices to be directly
//! mapped to memory and managed by MMU. This makes I/O operations can be
//! performed quickly and efficiently, without requiring the CPU to spend a lot
//! of time managing the I/O operations directly.
//!
//!
//! ## Tasks
//! In this project, you are requested to implement simple virtual MMIO control
//! for the purpose of the printing text on the host console. In our MMIO
//! PrinterDev specifications, we dictate the usage of 0xcafe0000 guest physical
//! address for the buffer, which contains an array of utf8 strings. The length
//! of the array can be founded at the 0xcafe0008, while the doorbell is located
//! at 0xcafe0010. The doorbell in MMIO typically refers to a deginated memory
//! location used to trigger device operations. The PrinterDev virtual device
//! lauches an operation when a write occurs to the doorbell address, and
//! subsequently fetches the address and length of the string buffer.
//! It then parses the string data from the buffer and outputs given text to the
//! host console. In summary:
//! * 0xcafe0000: Guest physical address for the utf8 string buffer
//! * 0xcafe0008: Length of the utf8 string buffer
//! * 0xcafe0010: The doorbell which notifies VMM to print the registered string
//!   to the console
//!
//! Your device SHOULD parses the string data from the buffer and outputs the
//! given text to the teletype device. You can translate a raw pointer to slice
//! by using [`core::slice::from_raw_parts`].
//!
//! [`core::slice::from_utf8`]: https://doc.rust-lang.org/beta/core/str/fn.from_utf8.html
//! [`core::slice::from_raw_parts`]: https://doc.rust-lang.org/std/slice/fn.from_raw_parts.html

use crate::keos::teletype::Teletype;
use crate::vmexit::mmio::{self, MmioInfo, MmioRegion};
use kev::{
    Probe, VmError,
    vcpu::{GenericVCpuState, VmexitResult},
    vm::Gpa,
};

#[derive(Default)]
pub struct PrinterDev {}

impl mmio::MmioHandler for PrinterDev {
    fn region(&self) -> MmioRegion {
        MmioRegion {
            start: Gpa::new(0xcafe0000).unwrap(),
            end: Gpa::new(0xcafe0018).unwrap(),
        }
    }

    fn handle(
        &mut self,
        p: &dyn Probe,
        info: MmioInfo,
        GenericVCpuState { vmcs, .. }: &mut GenericVCpuState,
    ) -> Result<VmexitResult, VmError> {
        // Mmio register:
        // - 0xcafe0000 (8 bytes): physical address of the buffer.
        // - 0xcafe0008 (8 bytes): length of the buffer.
        // - 0xcafe0010 (8 bytes): the doorbell.
        //
        // - To print out the contents, you required to do following steps:
        // 1. Write the buffer address to pa 0xcafe0000.
        // 2. Write the buffer length to pa 0xcafe0008.
        // 3. Write any value to 0xcafe0010 to ring the doorbell.
        //
        // Hint:
        //   - If io size is invalid, ignore the request.
        //   - You should reflect the change on the mmio area.
        todo!()
    }
}
