//! Global and Local Descriptor Table.

use core::arch::asm;

use super::interrupt::{
    AbortDoubleFault, AbortMachineCheck, Handler, HandlerPageFault,
    HandlerWithSegmentSelectorErrorCode, InterruptGateDescriptor,
};

/// Local descriptor table.
pub struct LocalDescriptorTable;

impl LocalDescriptorTable {
    /// Kill the LDT into zero.
    #[inline(always)]
    pub fn kill() {
        unsafe {
            asm!("lldt ax", in("eax") 0_u16, options(nostack));
        }
    }
}

impl DescriptorTable for LocalDescriptorTable {
    #[inline(always)]
    fn load<T>(table: SystemTableRegister<T>) {
        unsafe {
            asm!("lldt [{0}]", in(reg) &table, options(nostack));
        }
    }
}

#[doc(hidden)]
pub trait DescriptorTable {
    fn load<T>(table: SystemTableRegister<T>);
}

/// Global descriptor table.
pub struct GlobalDescriptorTable;

impl DescriptorTable for GlobalDescriptorTable {
    #[inline(always)]
    fn load<T>(table: SystemTableRegister<T>) {
        unsafe {
            asm!("lgdt [{0}]", in(reg) &table, options(nostack));
        }
    }
}

/// Table of interrupt descriptors.
#[repr(C)]
pub struct InterruptDescriptorTable {
    /// Interrupt 0 - Divide Error Exception (#DE)
    pub divide_error: InterruptGateDescriptor<Handler>,
    /// Interrupt 1 - Debug Exception (#DB)
    pub debug: InterruptGateDescriptor<Handler>,
    /// Interrupt 2 - NMI Interrupt
    pub nmi: InterruptGateDescriptor<Handler>,
    /// Interrupt 3 - Breakpoint Exception (#BP)
    pub breakpoint: InterruptGateDescriptor<Handler>,
    /// Interrupt 4 - Overflow Exception (#OF)
    pub overflow_exception: InterruptGateDescriptor<Handler>,
    /// Interrupt 5 - Bound Range Exceeded Exception (#BR)
    pub bound_range_exceeded: InterruptGateDescriptor<Handler>,
    /// Interrupt 6 - Invalid Opcode Exception (#UD)
    pub invalid_opcode: InterruptGateDescriptor<Handler>,
    /// Interrupt 7 - Device Not Available Exception (#NM)
    pub device_not_available: InterruptGateDescriptor<Handler>,
    /// Interrupt 8 - Double Fault Exception (#DF)
    pub double_fault: InterruptGateDescriptor<AbortDoubleFault>,
    /// Interrupt 9 - Coprocessor Segment Overrun
    pub coprocessor_segment_overrun: InterruptGateDescriptor<Handler>,
    /// Interrupt 10 - Invalid TSS Exception (#TS)
    pub invalid_tss: InterruptGateDescriptor<HandlerWithSegmentSelectorErrorCode>,
    /// Interrupt 11 - Segment Not Present (#NP)
    pub segment_not_present: InterruptGateDescriptor<HandlerWithSegmentSelectorErrorCode>,
    /// Interrupt 12 - Stack Fault Exception (#SS)
    pub stack_fault: InterruptGateDescriptor<HandlerWithSegmentSelectorErrorCode>,
    /// Interrupt 13 - General Protection Exception (#GP)
    pub general_protection: InterruptGateDescriptor<HandlerWithSegmentSelectorErrorCode>,
    /// Interrupt 14 - Page-Fault Exception (#PF)
    pub page_fault: InterruptGateDescriptor<HandlerPageFault>,
    /// Interrupt 15 - Reserved.
    _reserved0: InterruptGateDescriptor<core::convert::Infallible>,
    /// Interrupt 16 - x87 FPU Floating-Point Error (#MF)
    pub x87_fpu_floating_point_error: InterruptGateDescriptor<Handler>,
    /// Interrupt 17 - Alignment Check Exception (#AC)
    pub alignment_check_exception: InterruptGateDescriptor<Handler>,
    /// Interrupt 18 - Machine-Check Exception (#MC)
    pub machine_check_exception: InterruptGateDescriptor<AbortMachineCheck>,
    /// Interrupt 19 - SIMD Floating-Point Exception (#XM)
    pub simd_floating_point_exception: InterruptGateDescriptor<Handler>,
    /// Interrupt 20 - Virtualization Exception (#VE)
    pub virtualization_exception: InterruptGateDescriptor<Handler>,
    /// Interrupt 21 - Reserved1.
    _reserved1: InterruptGateDescriptor<core::convert::Infallible>,
    /// Interrupt 22 - Reserved2.
    _reserved2: InterruptGateDescriptor<core::convert::Infallible>,
    /// Interrupt 23 - Reserved3.
    _reserved3: InterruptGateDescriptor<core::convert::Infallible>,
    /// Interrupt 24 - Reserved4.
    _reserved4: InterruptGateDescriptor<core::convert::Infallible>,
    /// Interrupt 25 - Reserved5.
    _reserved5: InterruptGateDescriptor<core::convert::Infallible>,
    /// Interrupt 26 - Reserved6.
    _reserved6: InterruptGateDescriptor<core::convert::Infallible>,
    /// Interrupt 27 - Reserved7.
    _reserved7: InterruptGateDescriptor<core::convert::Infallible>,
    /// Interrupt 28 - Reserved8.
    _reserved8: InterruptGateDescriptor<core::convert::Infallible>,
    /// Interrupt 29 - Reserved9.
    _reserved9: InterruptGateDescriptor<core::convert::Infallible>,
    /// Interrupt 30 - Reserved10.
    _reserved10: InterruptGateDescriptor<core::convert::Infallible>,
    /// Interrupt 31 - Reserved11.
    _reserved11: InterruptGateDescriptor<core::convert::Infallible>,
    /// Interrupt 32 ~ 255 - User Defined Interrupt 0 ~ 223
    pub user_defined: [InterruptGateDescriptor<Handler>; 224],
}

impl DescriptorTable for InterruptDescriptorTable {
    #[inline(always)]
    fn load<T>(table: SystemTableRegister<T>) {
        unsafe {
            asm!("lidt [{0}]", in(reg) &table, options(nostack));
        }
    }
}

impl InterruptDescriptorTable {
    pub const fn empty() -> Self {
        Self {
            divide_error: InterruptGateDescriptor::empty(),
            debug: InterruptGateDescriptor::empty(),
            nmi: InterruptGateDescriptor::empty(),
            breakpoint: InterruptGateDescriptor::empty(),
            overflow_exception: InterruptGateDescriptor::empty(),
            bound_range_exceeded: InterruptGateDescriptor::empty(),
            invalid_opcode: InterruptGateDescriptor::empty(),
            device_not_available: InterruptGateDescriptor::empty(),
            double_fault: InterruptGateDescriptor::empty(),
            coprocessor_segment_overrun: InterruptGateDescriptor::empty(),
            invalid_tss: InterruptGateDescriptor::empty(),
            segment_not_present: InterruptGateDescriptor::empty(),
            stack_fault: InterruptGateDescriptor::empty(),
            general_protection: InterruptGateDescriptor::empty(),
            page_fault: InterruptGateDescriptor::empty(),
            _reserved0: InterruptGateDescriptor::empty(),
            x87_fpu_floating_point_error: InterruptGateDescriptor::empty(),
            alignment_check_exception: InterruptGateDescriptor::empty(),
            machine_check_exception: InterruptGateDescriptor::empty(),
            simd_floating_point_exception: InterruptGateDescriptor::empty(),
            virtualization_exception: InterruptGateDescriptor::empty(),
            _reserved1: InterruptGateDescriptor::empty(),
            _reserved2: InterruptGateDescriptor::empty(),
            _reserved3: InterruptGateDescriptor::empty(),
            _reserved4: InterruptGateDescriptor::empty(),
            _reserved5: InterruptGateDescriptor::empty(),
            _reserved6: InterruptGateDescriptor::empty(),
            _reserved7: InterruptGateDescriptor::empty(),
            _reserved8: InterruptGateDescriptor::empty(),
            _reserved9: InterruptGateDescriptor::empty(),
            _reserved10: InterruptGateDescriptor::empty(),
            _reserved11: InterruptGateDescriptor::empty(),
            user_defined: [InterruptGateDescriptor::empty(); 224],
        }
    }

    pub fn load(&'static self) {
        SystemTableRegister::new(self).load::<Self>();
    }
}

/// X86_64's system table register.
#[repr(C, packed)]
pub struct SystemTableRegister<T> {
    pub size: u16,
    pub address: u64,
    _ty: core::marker::PhantomData<T>,
}

impl<T> SystemTableRegister<T> {
    /// Create a system table register from given table.
    #[inline]
    pub fn new(t: &'static T) -> Self {
        SystemTableRegister {
            size: (core::mem::size_of::<T>() as u16) - 1,
            address: t as *const T as usize as u64,
            _ty: core::marker::PhantomData,
        }
    }

    /// Load the system table register into CPU.
    #[inline]
    pub fn load<V>(self)
    where
        V: DescriptorTable,
    {
        V::load(self)
    }
}
