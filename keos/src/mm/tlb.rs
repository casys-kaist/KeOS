//! TLB Shootdown helper.
use crate::sync::{
    RwLock,
    atomic::{AtomicBool, AtomicUsize},
};
use abyss::{
    MAX_CPU,
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

#[doc(hidden)]
#[allow(clippy::declare_interior_mutable_const)]
const PER_CORE_STATUS: AtomicBool = AtomicBool::new(false);

#[doc(hidden)]
static HAVE_REQUEST: [AtomicBool; MAX_CPU] = [PER_CORE_STATUS; MAX_CPU];

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
        let guard = loop {
            if let Ok(guard) = IN_PROGRESS.try_lock() {
                break guard;
            }

            TlbIpi::handle();
        };

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
                    if *i == self_id {
                        true
                    } else if cpu.load(Ordering::SeqCst) {
                        HAVE_REQUEST[*i].store(true);
                        true
                    } else {
                        false
                    }
                })
                .count();

            unsafe {
                abyss::dev::x86_64::apic::send_ipi(IPIDest::AllExcludingSelf, Mode::Fixed(0x7e));
            }

            let request_ref = request.as_ref().unwrap();

            while request_ref.processed.load() < online_cpu_cnt - 1 {
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
        let core_id = abyss::x86_64::intrinsics::cpuid();
        if HAVE_REQUEST[core_id].load() {
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
                HAVE_REQUEST[core_id].store(false);
            }
        }
    }
}

/// Event handler for TLB Shootdown request
pub fn handler(_regs: &mut Registers) {
    TlbIpi::handle();
}
