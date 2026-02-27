//! Pio handlers to test pio instructions correctly implemented.
use crate::vmexit::pio::{Direction, PioHandler};
use alloc::{boxed::Box, collections::LinkedList};
use keos::sync::SpinLock;
use kev::{
    Probe, VmError,
    vcpu::{GenericVCpuState, VmexitResult},
};

/// emulation of the device that prints the port & direction.
pub struct PioHandlerDummy;
impl PioHandler for PioHandlerDummy {
    fn handle(
        &self,
        port: u16,
        direction: Direction,
        _p: &dyn Probe,
        _generic_vcpu_state: &mut GenericVCpuState,
    ) -> Result<VmexitResult, VmError> {
        println!("port {} direction {:?}", port, direction);
        Ok(VmexitResult::Ok)
    }
}

/// emulation of the device that print the character of the operand in Out
/// instruction.
pub struct PioHandlerPrint;
impl PioHandler for PioHandlerPrint {
    fn handle(
        &self,
        _port: u16,
        direction: Direction,
        _p: &dyn Probe,
        _generic_vcpu_state: &mut GenericVCpuState,
    ) -> Result<VmexitResult, VmError> {
        let char = match direction {
            Direction::Outb(byte) => byte,
            _ => unreachable!(),
        };
        let b = core::char::from_u32(char as u32).unwrap();
        println!("{}", b);
        Ok(VmexitResult::Ok)
    }
}

/// emulation of device that tests all In/Out/Ins/Outs instruction family with
/// three queues.
#[derive(Default)]
pub struct PioHandlerQueue {
    byte_queue: SpinLock<LinkedList<u8>>,
    word_queue: SpinLock<LinkedList<u16>>,
    dword_queue: SpinLock<LinkedList<u32>>,
}

impl PioHandlerQueue {
    /// Create a new PioHandlerQueue.
    pub fn new() -> Self {
        Self {
            byte_queue: SpinLock::new(LinkedList::new()),
            word_queue: SpinLock::new(LinkedList::new()),
            dword_queue: SpinLock::new(LinkedList::new()),
        }
    }
}
impl PioHandler for PioHandlerQueue {
    fn handle(
        &self,
        _port: u16,
        direction: Direction,
        p: &dyn Probe,
        GenericVCpuState { vmcs, gprs, .. }: &mut GenericVCpuState,
    ) -> Result<VmexitResult, VmError> {
        match direction {
            Direction::InbAl => {
                let mut guard = self.byte_queue.lock();
                let b = guard.pop_front();
                guard.unlock();
                if let Some(byte) = b {
                    gprs.rax = byte as usize;
                } else {
                    return Err(VmError::ControllerError(Box::new("Empty byte queue")));
                }
            }
            Direction::InwAx => {
                let mut guard = self.word_queue.lock();
                let b = guard.pop_front();
                guard.unlock();
                if let Some(word) = b {
                    gprs.rax = word as usize;
                } else {
                    return Err(VmError::ControllerError(Box::new("Empty word queue")));
                }
            }
            Direction::IndEax => {
                let mut guard = self.dword_queue.lock();
                let b = guard.pop_front();
                guard.unlock();
                if let Some(dword) = b {
                    gprs.rax = dword as usize;
                } else {
                    return Err(VmError::ControllerError(Box::new("Empty dword queue")));
                }
            }
            Direction::Inbm(gva) => {
                let mut guard = self.byte_queue.lock();
                let b = guard.pop_front();
                guard.unlock();
                if let Some(byte) = b {
                    unsafe {
                        core::ptr::write_unaligned(
                            p.gva2hva(vmcs, gva).unwrap().into_usize() as *mut u8,
                            byte,
                        );
                    }
                } else {
                    return Err(VmError::ControllerError(Box::new("Empty byte queue")));
                }
            }
            Direction::Inwm(gva) => {
                let mut guard = self.word_queue.lock();
                let b = guard.pop_front();
                guard.unlock();
                if let Some(word) = b {
                    unsafe {
                        core::ptr::write_unaligned(
                            p.gva2hva(vmcs, gva).unwrap().into_usize() as *mut u16,
                            word,
                        );
                    }
                } else {
                    return Err(VmError::ControllerError(Box::new("Empty word queue")));
                }
            }
            Direction::Indm(gva) => {
                let mut guard = self.dword_queue.lock();
                let b = guard.pop_front();
                guard.unlock();
                if let Some(dword) = b {
                    unsafe {
                        core::ptr::write_unaligned(
                            p.gva2hva(vmcs, gva).unwrap().into_usize() as *mut u32,
                            dword,
                        );
                    }
                } else {
                    return Err(VmError::ControllerError(Box::new("Empty dword queue")));
                }
            }
            Direction::Outb(byte) => {
                let mut q = self.byte_queue.lock();
                q.push_back(byte);
                q.unlock();
            }
            Direction::Outw(word) => {
                let mut q = self.word_queue.lock();
                q.push_back(word);
                q.unlock();
            }
            Direction::Outd(dword) => {
                let mut q = self.dword_queue.lock();
                q.push_back(dword);
                q.unlock();
            }
        }
        Ok(VmexitResult::Ok)
    }
}
