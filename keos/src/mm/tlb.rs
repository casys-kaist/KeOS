//! TLB Shootdown helper.
use crate::{
    mm::page_table::ACTIVE_PAGE_TABLES,
    sync::{RwLock, atomic::AtomicUsize},
};
use abyss::{
    addressing::Va,
    boot::ONLINE_CPU,
    dev::x86_64::apic::{IPIDest, Mode},
    interrupt::Registers,
    spinlock::SpinLock,
    x86_64::Cr3,
};
use core::sync::atomic::Ordering;

#[doc(hidden)]
static IN_PROGRESS: SpinLock<()> = SpinLock::new(());

#[doc(hidden)]
static REQUEST: RwLock<Option<TlbIpi>> = RwLock::new(None);

/// Struct for TLB request
pub struct TlbIpi {
    /// Destination Cr3
    cr3: Cr3,

    /// If va is Some, invalidate only that page. Otherwise, shutdown the whole
    /// TLB.
    va: Option<Va>,

    /// Count of CPUs that processed this request.
    processed: AtomicUsize,
}

impl TlbIpi {
    /// Send the request and wait until the request is done for all CPUs
    pub fn send(cr3: Cr3, va: Option<Va>) {
        let cr3_pa = cr3.0 as usize;
        let guard = IN_PROGRESS.lock();

        // Publish the requests.
        {
            let mut request = REQUEST.write();

            assert!(
                request.is_none(),
                "Before sending TLB Shootdown request, the request queue must be empty."
            );

            *request = Some(Self {
                cr3,
                va,
                processed: AtomicUsize::new(0),
            });
        }

        // Waiting the requests
        {
            let request = REQUEST.read();
            let self_id = abyss::x86_64::intrinsics::cpuid();

            let online_cpu_cnt = ONLINE_CPU
                .iter()
                .enumerate()
                .filter(|(i, cpu)| {
                    cpu.load(Ordering::SeqCst)
                        && *i != self_id
                        && ACTIVE_PAGE_TABLES[*i].load() == cr3_pa
                })
                .map(|(core_id, _)| unsafe {
                    abyss::dev::x86_64::apic::send_ipi(IPIDest::Cpu(core_id), Mode::Fixed(0x7e));
                })
                .count();

            let request_ref = request.as_ref().unwrap();

            while request_ref.processed.load() < online_cpu_cnt {
                core::hint::spin_loop();
            }
        }

        // Clean up the request
        {
            let mut request = REQUEST.write();
            *request = None;
        }

        guard.unlock();
    }

    fn handle() {
        let request = REQUEST.read();

        if let Some(request) = &*request {
            if request.cr3 == Cr3::current() {
                match request.va {
                    Some(va) => unsafe {
                        core::arch::asm!(
                            "invlpg [{0}]",
                            in(reg) va.into_usize(),
                            options(nostack)
                        )
                    },
                    _ => unsafe {
                        core::arch::asm! {
                            "mov rax, cr3",
                            "mov cr3, rax",
                            out("rax") _,
                            options(nostack)
                        }
                    },
                }
            }
            request.processed.fetch_add(1);
        }
    }
}

/// Event handler for TLB Shootdown request
pub fn handler(_regs: &mut Registers) {
    TlbIpi::handle();
}
