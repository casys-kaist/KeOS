//! Interrupt management.
use crate::{sync::SpinLock, thread::with_current};
use abyss::{
    x86_64::{Cr2, PrivilegeLevel, interrupt::PFErrorCode},
    {addressing::Va, interrupt::Registers},
};
use alloc::sync::Arc;

type Handler = Option<Arc<dyn Fn(&mut Registers) + Send + Sync>>;
#[allow(clippy::declare_interior_mutable_const)]
const INIT: SpinLock<Handler> = SpinLock::new(None);
static HANDLERS: [SpinLock<Handler>; 224] = [INIT; 224];

#[doc(hidden)]
#[unsafe(no_mangle)]
pub fn do_handle_interrupt(idx: usize, frame: &mut Registers) {
    let guard = HANDLERS.get(idx).unwrap().lock();
    let handler = guard.clone();
    guard.unlock();

    match &handler {
        Some(handler) => handler(frame),
        _ => {
            panic!("Unknown interrupt #{}", idx + 32);
        }
    }

    if frame.interrupt_stack_frame.cs.dpl() == PrivilegeLevel::Ring3 {
        crate::thread::__check_for_signal();
    }
}

/// Register the interrupt handler
pub fn register(vec: usize, handler: impl Fn(&mut Registers) + Send + Sync + 'static) {
    let mut guard = HANDLERS.get(vec - 32).expect("Invalid index").lock();
    *guard = Some(Arc::new(handler));
    guard.unlock();
}

/// The entry points of the page fault.
#[doc(hidden)]
#[unsafe(no_mangle)]
pub extern "C" fn handle_page_fault(frame: &mut Registers, ec: PFErrorCode) {
    with_current(|th| match th.task.as_mut() {
        Some(task) => {
            let cr2 = Va::new(Cr2::current().into_usize()).unwrap();
            // Enable interrupt after resolving the faulting address.
            unsafe {
                core::arch::asm!("sti");
            }
            task.page_fault(ec, cr2);
        }
        _ => {
            panic!("Unexpected page fault: {:?} {:#?}", ec, frame);
        }
    });
}
