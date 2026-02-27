//! Virtual machine interface.
use crate::{
    VmError,
    vcpu::{GenericVCpuState, VCpu, VCpuOps, VCpuState},
    vmcs::Field,
};
use abyss::dev::x86_64::apic::{IPIDest, Mode, send_ipi};
use alloc::{boxed::Box, sync::Arc, vec::Vec};
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use keos::{
    sync::SpinLock,
    syscall::Registers,
    thread::{Current, JoinHandle, ParkHandle, ThreadBuilder},
};

/// Guest virtual address
#[repr(transparent)]
#[derive(Clone, Copy, Eq, PartialEq, PartialOrd, Ord)]
pub struct Gva(usize);

impl Gva {
    /// Create a new virtual address with a check.
    #[inline(always)]
    pub const fn new(addr: usize) -> Option<Self> {
        match addr & 0xffff_8000_0000_0000 {
            m if m == 0xffff_8000_0000_0000 || m == 0 => Some(Self(addr)),
            _ => None,
        }
    }

    /// Cast into usize.
    #[inline]
    pub const fn into_usize(self) -> usize {
        self.0
    }
}

/// Guest physical address
#[repr(transparent)]
#[derive(Clone, Copy, Eq, PartialEq, PartialOrd, Ord)]
pub struct Gpa(usize);
impl Gpa {
    /// Create a new physical address with a check.
    #[inline]
    pub const fn new(addr: usize) -> Option<Self> {
        if addr < 0xffff_0000_0000_0000 {
            Some(Self(addr))
        } else {
            None
        }
    }

    /// Cast into usize.
    #[inline]
    pub const fn into_usize(self) -> usize {
        self.0
    }
}

macro_rules! impl_arith {
    ($t: ty) => {
        impl core::ops::Add<usize> for $t {
            type Output = Self;

            fn add(self, other: usize) -> Self::Output {
                Self(self.0 + other)
            }
        }
        impl core::ops::AddAssign<usize> for $t {
            fn add_assign(&mut self, other: usize) {
                self.0 = self.0 + other
            }
        }
        impl core::ops::Sub<usize> for $t {
            type Output = Self;

            fn sub(self, other: usize) -> Self::Output {
                Self(self.0 - other)
            }
        }
        impl core::ops::SubAssign<usize> for $t {
            fn sub_assign(&mut self, other: usize) {
                self.0 = self.0 - other
            }
        }
        impl core::ops::BitOr<usize> for $t {
            type Output = Self;

            fn bitor(self, other: usize) -> Self {
                Self(self.0 | other)
            }
        }
        impl core::ops::BitOrAssign<usize> for $t {
            fn bitor_assign(&mut self, other: usize) {
                self.0 = self.0 | other;
            }
        }
        impl core::ops::BitAnd<usize> for $t {
            type Output = Self;

            fn bitand(self, other: usize) -> Self {
                Self(self.0 & other)
            }
        }
        impl core::ops::BitAndAssign<usize> for $t {
            fn bitand_assign(&mut self, other: usize) {
                self.0 = self.0 & other;
            }
        }
    };
}

impl_arith!(Gva);
impl_arith!(Gpa);

impl core::fmt::Debug for Gva {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Gva(0x{:x})", self.0)
    }
}
impl core::fmt::Display for Gva {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Gva(0x{:x})", self.0)
    }
}

impl core::fmt::Debug for Gpa {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Gpa(0x{:x})", self.0)
    }
}
impl core::fmt::Display for Gpa {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Gpa(0x{:x})", self.0)
    }
}

/// Per-vm private state.
pub trait VmState
where
    Self: Sync + Send,
{
    /// Per-Vcpu private state
    type VcpuState: VCpuState;
    /// Error Kind for setup_bsp_vcpu.
    type Error;

    /// Create per-vcpu private state.
    fn vcpu_state(&self) -> Self::VcpuState;
    /// Setup the virtual bootstrap processor (bsp) state.
    fn setup_vbsp(
        &self,
        vbsp_generic_state: &mut GenericVCpuState,
        vbsp_vcpu_state: &mut Self::VcpuState,
    ) -> Result<(), Self::Error>;
    /// Setup the ap state.
    fn setup_ap(
        &self,
        _ap_generic_state: &mut GenericVCpuState,
        _ap_vcpu_state: &mut Self::VcpuState,
    ) -> Result<(), Self::Error> {
        panic!("ap is not supported.");
    }
}

/// MSR entries that can be passed through VMEXIT_MSR_LOAD_ADDR or
/// VMEXIT_MSR_STORE_ADDR.
#[repr(C, packed)]
pub struct MsrEntry {
    /// MSR index
    pub msr_index: u32,
    #[doc(hidden)]
    _rsv: u32,
    /// MSR value
    pub msr_value: u64,
}

#[doc(hidden)]
pub enum VCpuRunningState {
    Halted,
    Running {
        handle: JoinHandle,
        have_kicked: Arc<AtomicBool>,
    },
    Kicked(ParkHandle),
}

/// The virtual machine.
pub struct Vm<S: VmState + 'static> {
    vcpu: Vec<Arc<SpinLock<VCpu<S>>>>,
    pub(crate) state: S,
    pub(crate) exit_code: AtomicU64,
    vcpu_states: Vec<Arc<SpinLock<VCpuRunningState>>>,
}

/// Handle for maintaining a VM.
pub struct VmHandle<S: VmState + 'static> {
    vm: Arc<Vm<S>>,
    /// VCPU threads.
    pub vcpu_threads: Vec<Arc<SpinLock<VCpuRunningState>>>,
}

impl<S: VmState + 'static> VmHandle<S> {
    pub(crate) fn new(vcpu: usize, state: S) -> Result<Self, S::Error> {
        let vm = Arc::new(Vm {
            vcpu: Vec::new(),
            state,
            exit_code: AtomicU64::new(0),
            vcpu_states: (0..vcpu)
                .map(|_| Arc::new(SpinLock::new(VCpuRunningState::Halted)))
                .collect(),
        });
        let mut this = VmHandle {
            vcpu_threads: vm.vcpu_states.to_vec(),
            vm,
        };
        let mut vcpu_vec = Vec::new();
        for id in 0..vcpu {
            vcpu_vec.push(Arc::new(SpinLock::new(VCpu::new(
                id,
                this.vm.state.vcpu_state(),
                Arc::downgrade(&this.vm),
            ))))
        }
        // SAFETY:
        // vcpu is not running.
        unsafe {
            Arc::get_mut_unchecked(&mut this.vm).vcpu = vcpu_vec;
        }

        {
            let mut guard = this.vm.vcpu[0].lock();
            let mut activated = guard.unpack_activate().expect("Failed to activate vcpu.");
            this.vm
                .state
                .setup_vbsp(&mut activated.generic_state, activated.vcpu_state)?;
            drop(activated);
            guard.unlock();
        }
        for vcpu in this.vm.vcpu.iter().skip(1) {
            let mut guard = vcpu.lock();
            let mut activated = guard.unpack_activate().expect("Failed to activate vcpu.");
            this.vm
                .state
                .setup_ap(&mut activated.generic_state, activated.vcpu_state)?;
            drop(activated);
            guard.unlock();
        }
        Ok(this)
    }

    /// Get vcpu #idx.
    #[inline]
    pub fn vcpu(&self, idx: usize) -> Option<&Arc<SpinLock<VCpu<S>>>> {
        self.vm.vcpu.get(idx)
    }

    /// Join the vm.
    pub fn join(&self) -> i32 {
        loop {
            let v = self.vm.exit_code.load(Ordering::SeqCst);
            if v >= 0x8000_0000_0000_0000 {
                break v as i32;
            }
        }
    }

    /// Start this vm's bsp.
    #[inline]
    pub fn start_bsp(&self) -> Result<(), VmError> {
        self.vm.start_vcpu(0, |_| {})
    }
}

impl<S: VmState + 'static> Drop for Vm<S> {
    fn drop(&mut self) {
        /*
        for th in vcpu_threads.into_iter() {
            loop {
                if let Ok(mut state) = th.try_lock() {
                    match &*state {
                        VCpuRunningState::Running { handle, .. } => {
                            handle.join();
                        }
                        VCpuRunningState::Kicked(p) => {
                            *state = VCpuRunningState::Halted;
                        }
                        VCpuRunningState::Halted => (),
                    }
                    break;
                }
            }
        }
        */
    }
}

impl<S: VmState + 'static> Vm<S> {
    /// The main loop of a VCpu.
    pub fn vcpu_thread_work(
        vcpu: Arc<SpinLock<VCpu<S>>>,
        state: Arc<SpinLock<VCpuRunningState>>,
        init: impl FnOnce(&SpinLock<VCpu<S>>),
    ) {
        use crate::vcpu::VmexitResult;

        init(&vcpu);

        let guard = state.lock();
        let have_kicked = match &*guard {
            VCpuRunningState::Running { have_kicked, .. } => have_kicked.clone(),
            _ => {
                unreachable!()
            }
        };
        guard.unlock();
        let exit_code = loop {
            {
                unsafe { abyss::interrupt::InterruptState::disable() };
                let mut vcpu_guard = vcpu.lock();
                let loop_result = vcpu_guard
                    .unpack_activate()
                    .expect("Failed to activate vcpu")
                    .vcpu_loop(&have_kicked)
                    .expect("Vm exited");
                vcpu_guard.unlock();

                match loop_result {
                    VmexitResult::Exited(exit_code) => {
                        unsafe { abyss::interrupt::InterruptState::enable() };
                        break exit_code;
                    }
                    VmexitResult::ExtInt(vec) => {
                        abyss::interrupt::do_handle_irq(&mut Registers::default(), vec as usize);
                        // IPI_KICK
                        if vec != 100 {
                            unsafe { abyss::interrupt::InterruptState::enable() };
                            continue;
                        }
                    }
                    VmexitResult::Kicked => (),
                    VmexitResult::Ok => unreachable!(),
                }
                unsafe { abyss::interrupt::InterruptState::enable() };
            }
            // Kicked.
            {
                let mut guard = state.lock();
                if have_kicked.fetch_and(false, Ordering::SeqCst) {
                    match core::mem::replace(&mut *guard, VCpuRunningState::Halted) {
                        VCpuRunningState::Running {
                            handle,
                            have_kicked,
                        } => {
                            Current::park_with(move |hdl| {
                                *guard = VCpuRunningState::Kicked(hdl);
                                guard.unlock();
                            });
                            let mut guard = state.lock();
                            *guard = VCpuRunningState::Running {
                                handle,
                                have_kicked,
                            };
                            guard.unlock();
                        }
                        _ => {
                            unreachable!()
                        }
                    }
                }
            }
        };
        Current::exit(exit_code);
    }

    fn start_vcpu(
        &self,
        id: usize,
        init: impl FnOnce(&SpinLock<VCpu<S>>) + Send + 'static,
    ) -> Result<(), VmError> {
        let vcpu = self
            .vcpu
            .get(id)
            .cloned()
            .ok_or(VmError::VCpuError(Box::new("VCpu not exists.")))?;

        let mut vcpu_slot = self.vcpu_states[id].lock();
        let slot = self.vcpu_states[id].clone();
        let have_kicked = Arc::new(AtomicBool::new(false));
        let ret = if matches!(&*vcpu_slot, VCpuRunningState::Halted) {
            *vcpu_slot = VCpuRunningState::Running {
                handle: ThreadBuilder::new(alloc::format!("vcpu#{}", id))
                    .spawn(move || Self::vcpu_thread_work(vcpu, slot, init)),
                have_kicked,
            };
            Ok(())
        } else {
            Err(VmError::VCpuError(Box::new("VCpu is already started.")))
        };
        vcpu_slot.unlock();
        ret
    }
}

/// VmState neutral Vm operations.
pub trait VmOps
where
    Self: Send + Sync,
{
    /// Kick the vcpu.
    fn kick_vcpu(&self, id: usize) -> Result<(), VmError>;
    /// Exit this vm.
    fn exit(&self, exit_code: i32);
    /// Start the vcpu.
    fn start_vcpu(&self, id: usize, ip: u16) -> Result<(), VmError>;
    /// Get the VCpuOps from the id of the VCpu.
    fn get_vcpu(&self, id: usize) -> Option<&dyn VCpuOps>;
    /// Resum the vcpu.
    fn resume_vcpu(&self, id: usize);
}

impl<S: VmState + 'static> VmOps for Vm<S> {
    fn kick_vcpu(&self, id: usize) -> Result<(), VmError> {
        if let Some(vcpu) = self.vcpu_states.get(id) {
            {
                let guard = vcpu.lock();
                match &*guard {
                    VCpuRunningState::Running {
                        handle,
                        have_kicked,
                    } => {
                        have_kicked.store(true, Ordering::SeqCst);
                        if let Some(cpuid) = handle.try_get_running_cpu() {
                            unsafe {
                                send_ipi(IPIDest::Cpu(cpuid), Mode::Fixed(100));
                            }
                        }
                    }
                    VCpuRunningState::Halted => {
                        warning!("kicking halted thread");
                        return Ok(());
                    }
                    VCpuRunningState::Kicked(_) => {
                        warning!("kicking kicked thread");
                        return Ok(());
                    }
                }
                guard.unlock();
            }
            loop {
                let guard = vcpu.lock();
                if matches!(&*guard, VCpuRunningState::Running { .. }) {
                    guard.unlock();
                    keos::thread::scheduler::scheduler().reschedule();
                } else {
                    guard.unlock();
                    break;
                }
            }
            Ok(())
        } else {
            Err(VmError::VCpuError(Box::new(alloc::format!(
                "vcpu#{id:} not exists"
            ))))
        }
    }
    fn resume_vcpu(&self, id: usize) {
        if let Some(vcpu) = self.vcpu_states.get(id) {
            let mut guard = vcpu.lock();
            match core::mem::replace(&mut *guard, VCpuRunningState::Halted) {
                VCpuRunningState::Kicked(handle) => {
                    handle.unpark();
                }
                _ => {
                    unreachable!()
                }
            }
            guard.unlock();
        }
    }

    fn exit(&self, exit_code: i32) {
        self.exit_code
            .store(0x8000_0000_0000_0000 | (exit_code as u64), Ordering::SeqCst);
    }

    fn start_vcpu(&self, id: usize, ip: u16) -> Result<(), VmError> {
        self.start_vcpu(id, move |vcpu| {
            let mut guard = vcpu.lock();
            guard
                .unpack_activate()
                .expect("Failed to activate vcpu")
                .generic_state
                .vmcs
                .write(Field::GuestRip, ip as u64)
                .expect("Faild to update vcpu.");
            guard.unlock();
        })
    }

    fn get_vcpu(&self, id: usize) -> Option<&dyn VCpuOps> {
        self.vcpu.get(id).map(|cpu| &**cpu as &dyn VCpuOps)
    }
}

impl<S: VmState> core::ops::Deref for Vm<S> {
    type Target = S;
    fn deref(&self) -> &Self::Target {
        &self.state
    }
}

/// Builder factory to build a virtual machine.
pub struct VmBuilder<S: VmState + 'static> {
    pub(crate) vm_handle: VmHandle<S>,
    exception_bitmap: u32,
}

impl<S: VmState + 'static> VmBuilder<S> {
    /// Get a builder object to create a new vm.
    ///
    /// The vm has `vcpu` numbers of virtual CPU.
    pub fn new(vmstate: S, vcpu: usize) -> Result<VmBuilder<S>, S::Error> {
        assert!(vcpu > 0);

        VmHandle::new(vcpu, vmstate).map(|vm| VmBuilder {
            vm_handle: vm,
            exception_bitmap: 0,
        })
    }

    /// Add a exception bitmap to the builder.
    #[inline]
    pub fn exception_bitmap(mut self, en: u32) -> Self {
        self.exception_bitmap = en;
        self
    }

    /// Finalize this builder.
    #[inline]
    pub fn finalize(self) -> Result<VmHandle<S>, VmError> {
        let Self {
            vm_handle,
            exception_bitmap,
        } = self;
        for vcpu in vm_handle.vm.vcpu.iter() {
            unsafe {
                let mut guard = vcpu.lock();
                guard.unpack_activate()?.init_vcpu(exception_bitmap)?;
                guard.unlock();
            }
        }
        Ok(vm_handle)
    }
}

impl<S: VmState> core::ops::Deref for VmBuilder<S> {
    type Target = S;
    fn deref(&self) -> &Self::Target {
        &self.vm_handle.vm.state
    }
}
