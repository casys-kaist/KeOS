//! Flags and MSRs for VMX capabilities.

// VMX Capalibility MSRs
/// MSR - IA32_VMX_BASIC
pub const IA32_VMX_BASIC: usize = 0x480;
/// MSR - IA32_VMX_PINBASED_CTLS.
pub const IA32_VMX_PINBASED_CTLS: usize = 0x481;
/// MSR - IA32_VMX_PROC_BASED_CTLS.
pub const IA32_VMX_PROC_BASED_CTLS: usize = 0x482;
/// MSR - IA32_VMX_PROC_BASED_CTLS2.
pub const IA32_VMX_PROC_BASED_CTLS2: usize = 0x48B;
/// MSR - IA32_VMX_EXIT_CTLS.
pub const IA32_VMX_EXIT_CTLS: usize = 0x483;
/// MSR - IA32_VMX_ENTRY_CTLS.
pub const IA32_VMX_ENTRY_CTLS: usize = 0x484;
/// MSR - IA32_VMX_MISC.
pub const IA32_VMX_MISC: usize = 0x485;
/// MSR - IA32_VMX_CR0_FIXED0.
pub const IA32_VMX_CR0_FIXED0: usize = 0x486;
/// MSR - IA32_VMX_CR0_FIXED1.
pub const IA32_VMX_CR0_FIXED1: usize = 0x487;
/// MSR - IA32_VMX_CR4_FIXED0.
pub const IA32_VMX_CR4_FIXED0: usize = 0x488;
/// MSR - IA32_VMX_CR4_FIXED1.
pub const IA32_VMX_CR4_FIXED1: usize = 0x489;
/// MSR - IA32_VMX_VMCS_ENUM.
pub const IA32_VMX_VMCS_ENUM: usize = 0x48A;
/// MSR - IA32_VMX_EPT_VPID_CAP.
pub const IA32_VMX_EPT_VPID_CAP: usize = 0x48C;
/// MSR - IA32_FEATURE_CONTROL.
pub const IA32_FEATURE_CONTROL: usize = 0x03A;

bitflags::bitflags! {
    /// Table 24-5. Definitions of Pin-Based VM-Execution Controls.
    pub struct VmcsPinBasedVmexecCtl: u32 {
        /// If this control is 1, external interrupts cause VM exits.
        /// Otherwise, they are delivered normally through the guest interrupt-descriptor table (IDT).
        /// If this control is 1, the value of RFLAGS. IF does not affect interrupt blocking.
        const EXTERNAL_INTERRUPT_EXITING = 1 << 0;
        /// If this control is 1, non-maskable interrupts (NMIs) cause VM exits.
        /// Otherwise, they are delivered normally using descriptor 2 of the IDT.
        /// This control also determines interactions between IRET and blocking by NMI (see Section 25.3).
        const NMI_EXITING = 1 << 3;
        /// If this control is 1, NMIs are never blocked and the “blocking by NMI” bit (bit 3) in the
        /// interruptibility-state field indicates “virtual-NMI blocking” (see Table 24-3).
        /// This control also interacts with the “NMI-window exiting” VM-execution control (see Section 24.6.2)
        const VIRTUAL_NMIS = 1 << 5;
        /// If this control is 1, the VMX-preemption timer counts down in VMX non-root operation;
        /// see Section 25.5.1. A VM exit occurs when the timer counts down to zero; see Section 25.2.
        const ACTIVE_VMX_PREEMPTION_TIMER = 1 << 6;
        /// If this control is 1, the processor treats interrupts with the posted-interrupt notification vector
        /// (see Section 24.6.8) specially, updating the virtual-APIC page with posted-interrupt requests (see Section 29.6).
        const PROCESS_POSTED_INTERRUPT = 1 << 7;
    }
}

bitflags::bitflags! {
    /// Table 24-6. Definitions of Primary Processor-Based VM-Execution Controls.
    pub struct VmcsProcBasedVmexecCtl: u32 {
        /// If this control is 1, a VM exit occurs at the beginning of any instruction if RFLAGS.IF = 1 and
        /// there are no other blocking of interrupts (see Section 24.4.2).
        const INTRWINEXIT = 1 << 2;
        /// This control determines whether executions of RDTSC, executions of RDTSCP, and executions
        /// of RDMSR that read from the IA32_TIME_STAMP_COUNTER MSR return a value modified by
        /// the TSC offset field (see Section 24.6.5 and Section 25.3).
        const USETSCOFF	= 1 << 3;
        /// This control determines whether executions of HLT cause VM exits.
        const HLT_EXITING = 1 << 7;
        /// This determines whether executions of INVLPG cause VM exits.
        const INVLPGEXIT = 1 << 9;
        /// This control determines whether executions of MWAIT cause VM exits
        const MWAITEXIT	= 1 << 10;
        /// This control determines whether executions of RDPMC cause VM exits.
        const RDPMCEXIT	= 1 << 11;
        /// This control determines whether executions of RDTSC and RDTSCP cause VM exits.
        const RDTSCEXIT	= 1 << 12;
        /// In conjunction with the CR3-target controls (see Section 24.6.7),
        /// this control determines whether executions of MOV to CR3 cause VM exits. See Section 25.1.3.
        ///
        /// The first processors to support the
        /// virtual-machine extensions supported only the 1-setting of this control.
        const CR3LOADEXIT = 1 << 15;
        /// This control determines whether executions of MOV from CR3 cause VM exits.
        ///
        /// The first processors to support the virtual-machine extensions supported only the 1-setting of this control.
        const CR3STOREXIT = 1 << 16;
        /// This control determines whether the tertiary processor-based VM-execution controls are used.
        /// If this control is 0, the logical processor operates as if all the tertiary processor-based
        /// VM-execution controls were also 0.
        const ACTIVETETCTL = 1 << 17;
        /// This control determines whether executions of MOV to CR8 cause VM exits.
        const CR8LOADEXIT = 1 << 19;
        /// This control determines whether executions of MOV from CR8 cause VM exits.
        const CR8STOREEXIT = 1 << 20;
        /// Setting this control to 1 enables TPR virtualization and other APIC-virtualization features. See Chapter 29.
        const USETPRSHADOW = 1 << 21;
        /// If this control is 1, a VM exit occurs at the beginning of any instruction if there is no virtual NMI blocking (see Section 24.4.2)
        const NMIWINEXIT = 1 << 22;
        /// This control determines whether executions of MOV DR cause VM exits.
        const MOVDREXIT	= 1 << 23;
        /// This control determines whether executions of I/O instructions
        /// (IN, INS/INSB/INSW/INSD, OUT, and OUTS/OUTSB/OUTSW/OUTSD) cause VM exits.
        const UNCONDIOEXIT = 1 << 24;
        /// This control determines whether I/O bitmaps are used to restrict executions
        /// of I/O instructions (see Section 24.6.4 and Section 25.1.3).
        ///
        /// For this control, “0” means “do not use I/O bitmaps” and “1” means “use I/O bitmaps.”
        /// If the I/O bitmaps are used, the setting of the “unconditional I/O exiting” control is ignored.
        const USEIOBMP = 1 << 25;
        /// If this control is 1, the monitor trap flag debugging feature is enabled. See Section 25.5.2.
        const MTF = 1 << 27;
        /// This control determines whether MSR bitmaps are used to control execution of the RDMSR and
        /// WRMSR instructions (see Section 24.6.9 and Section 25.1.3).
        ///
        /// For this control, “0” means “do not use MSR bitmaps” and “1” means “use MSR bitmaps.”
        /// If the MSR bitmaps are not used, all executions of the RDMSR and WRMSR instructions cause VM exits.
        const USEMSRBMP	= 1 << 28;
        /// This control determines whether executions of MONITOR cause VM exits.
        const MONITOREXIT = 1 << 29;
        /// This control determines whether executions of PAUSE cause VM exits.
        const PAUSEEXIT	= 1 << 30;
        /// This control determines whether the secondary processor-based VM-execution controls are used.
        /// If this control is 0, the logical processor operates as if all the secondary processor-based
        /// VM-execution controls were also 0.
        const ACTIVATE_SECONDARY_CTL = 1 << 31;
    }
}

bitflags::bitflags! {
    /// Table 24-7. Definitions of Secondary Processor-Based VM-Execution Controls.
    pub struct VmcsProcBasedSecondaryVmexecCtl: u32 {
        /// If this control is 1, the logical processor treats specially accesses to the page with the APICaccess address. See Section 29.4.
        const VIRTUALIZE_APIC_ACCESSES = 1 << 0;
        /// If this control is 1, extended page tables (EPT) are enabled. See Section 28.3.
        const ENABLE_EPT = 1 << 1;
        /// This control determines whether executions of LGDT, LIDT, LLDT, LTR, SGDT, SIDT, SLDT, and STR cause VM exits.
        const DESCRIPTOR_TABLE_EXITING = 1 << 2;
        /// If this control is 0, any execution of RDTSCP causes an invalid-opcode exception (#UD).
        const ENABLE_RDTSCP = 1 << 3;
        /// If this control is 1, the logical processor treats specially RDMSR and WRMSR to APIC MSRs (in the range 800H–8FFH). See Section 29.5.
        const VIRTUALIZED_X2APIC_MODE = 1 << 4;
        /// If this control is 1, cached translations of linear addresses are associated with a virtualprocessor identifier (VPID). See Section 28.1.
        const EANBLE_VPID = 1 << 5;
        /// This control determines whether executions of WBINVD and WBNOINVD cause VM exits.
        const WBINVD_EXITING = 1 << 6;
        /// This control determines whether guest software may run in unpaged protected mode or in realaddress mode.
        const UNRESTRICTED_GUEST = 1 << 7;
        /// If this control is 1, the logical processor virtualizes certain APIC accesses. See Section 29.4 and Section 29.5.
        const APIC_REGISTER_VIRTUALIZATION = 1 << 8;
        /// This controls enables the evaluation and delivery of pending virtual interrupts as well as the
        /// emulation of writes to the APIC registers that control interrupt prioritization.
        const VIRTUAL_INTERRUPT_DELIVERY = 1 << 9;
        /// This control determines whether a series of executions of PAUSE can cause a VM exit
        /// (see Section 24.6.13 and Section 25.1.3).
        const PAUSE_LOOP_EXITING = 1 << 10;
        /// This control determines whether executions of RDRAND cause VM exits.
        const RDRAND_EXITING = 1 << 1;
        /// If this control is 0, any execution of INVPCID causes a #UD.
        const ENABLE_INVPCID = 1 << 12;
        /// Setting this control to 1 enables use of the VMFUNC instruction in VMX non-root operation. See Section 25.5.6.
        const ENABLE_VM_FUNCTIONS = 1 << 13;
        /// If this control is 1, executions of VMREAD and VMWRITE in VMX non-root operation may access
        /// a shadow VMCS (instead of causing VM exits). See Section 24.10 and Section 30.3.
        const VMCS_SHADOWING = 1 << 14;
        /// If this control is 1, executions of ENCLS consult the ENCLS-exiting bitmap to determine whether
        /// the instruction causes a VM exit. See Section 24.6.16 and Section 25.1.3.
        const ENABLE_ENCLS_EXITING = 1 << 15;
        /// This control determines whether executions of RDSEED cause VM exits.
        const RDSEED_EXITING = 1 << 16;
        /// If this control is 1, an access to a guest-physical address that sets an EPT dirty bit first adds an
        /// entry to the page-modification log. See Section 28.3.6.
        const ENABLE_PML = 1 << 17;
        /// If this control is 1, EPT violations may cause virtualization exceptions (#VE) instead of VM exits.
        /// See Section 25.5.7.
        const EPT_VIOLATION_VE = 1 << 18;
        /// If this control is 1, Intel Processor Trace suppresses from PIPs an indication that the processor
        /// was in VMX non-root operation and omits a VMCS packet from any PSB+ produced in VMX nonroot operation (see Chapter 32).
        const CONCEAL_VMX_FROM_PT = 1 << 19;
        /// If this control is 0, any execution of XSAVES or XRSTORS causes a #UD.
        const ENABLE_XSAVES_XRSTORS = 1 << 20;
        /// If this control is 1, EPT execute permissions are based on whether the linear address being
        /// accessed is supervisor mode or user mode. See Chapter 28.
        const MODE_BASED_EXEC_CTL_FOR_EPT = 1 << 22;
        /// If this control is 1, EPT write permissions may be specified at the granularity of 128 bytes. See
        /// Section 28.3.4.
        const SUBPAGE_WRITE_PERM_FOR_EPT = 1 << 23;
        /// If this control is 1, all output addresses used by Intel Processor Trace are treated as guest
        /// physical addresses and translated using EPT. See Section 25.5.4.
        const INTEL_PT_USES_GPA = 1 << 24;
        /// This control determines whether executions of RDTSC, executions of RDTSCP, and executions
        /// of RDMSR that read from the IA32_TIME_STAMP_COUNTER MSR return a value modified by the
        /// TSC multiplier field (see Section 24.6.5 and Section 25.3).
        const USE_TSC_SCALING = 1 << 25;
        /// If this control is 0, any execution of TPAUSE, UMONITOR, or UMWAIT causes a #UD.
        const ENABLE_UWAIT_PAUSE = 1 << 26;
        /// If this control is 0, any execution of PCONFIG causes a #UD.
        const ENABLE_PCONFIG = 1 << 27;
        /// If this control is 1, executions of ENCLV consult the ENCLV-exiting bitmap to
        const ENABLE_ENCLV_EXITING = 1 << 28;

    }
}

bitflags::bitflags! {
    /// Table 24-15. Definitions of VM-Entry Controls.
    pub struct VmcsEntryCtl: u32 {
        /// This control determines whether DR7 and the IA32_DEBUGCTL MSR are loaded on VM entry.
        ///
        /// The first processors to support the virtual-machine extensions supported only the 1-setting of this control.
        const LOAD_DEBUG_CTL = 1 << 2;
        /// On processors that support Intel 64 architecture, this control determines
        /// whether the logical processor is in IA-32e mode after VM entry.
        /// Its value is loaded into IA32_EFER.LMA as part of VM entry.1
        ///
        /// This control must be 0 on processors that do not support Intel 64 architecture.
        const IA32E_MODE_GUEST = 1 << 9;
        /// This control determines whether the logical processor is in system-management mode (SMM) after VM entry.
        /// This control must be 0 for any VM entry from outside SMM.
        const ENTRY_TO_SMM = 1 << 10;
        /// If set to 1, the default treatment of SMIs and SMM is in effect after
        /// the VM entry (see Section 31.15.7). This control must be 0 for any VM entry from outside SMM.
        const DEACTIVATE_DUAL_MONITOR_TREATMENT = 1 << 11;
        /// This control determines whether the IA32_PERF_GLOBAL_CTRL MSR is loaded on VM entry.
        const LOAD_IA32_PERF_GLOBAL_CTRL = 1 << 13;
        ///  This control determines whether the IA32_PAT MSR is loaded on VM entry
        const LOAD_IA32_PAT = 1 << 14;
        /// This control determines whether the IA32_EFER MSR is loaded on VM entry.
        const LOAD_IA32_EFER = 1 << 15;
        /// This control determines whether the IA32_BNDCFGS MSR is loaded on VM entry.
        const LOAD_IA32_BNDCFGS = 1 << 16;
        /// If this control is 1, Intel Processor Trace does not produce a paging information packet (PIP)
        /// on a VM entry or a VMCS packet on a VM entry that returns from SMM (see Chapter 32).
        const CONCEAL_VMX_FROM_PT = 1 << 17;
        /// This control determines whether the IA32_RTIT_CTL MSR is loaded on VM entry.
        const LOAD_IA32_RTIT_CTL = 1 << 18;
        /// This control determines whether CET-related MSRs and SPP are loaded on VM entry
        const LOAD_CET_STATE = 1 << 20;
        /// This control determines whether the IA32_LBR_CTL MSR is loaded on VM entry.
        const LOAD_GUEST_IA32_LBR_CTL = 1 << 21;
        /// This control determines whether the IA32_PKRS MSR is loaded on VM entry.
        const LOAD_PKRS = 1 << 22;
    }
}

bitflags::bitflags! {
    /// Table 24-13. Definitions of Primary VM-Exit Controls.
    pub struct VmcsExitCtl: u32 {
        /// This control determines whether DR7 and the IA32_DEBUGCTL MSR are saved on
        /// VM exit.
        ///
        /// The first processors to support the virtual-machine extensions supported only the 1-setting of this control.
        const SAVE_DEBUG_CTLS = 1 << 2;
        /// On processors that support Intel 64 architecture, this control determines whether a logical processor
        /// is in 64-bit mode after the next VM exit. Its value is loaded into CS.L, IA32_EFER.LME, and IA32_EFER.LMA on every VM exit.1
        /// This control must be 0 on processors that do not support Intel 64 architecture.
        const HOST_ADDRESS_SPACE_SIZE = 1 << 9;
        /// This control determines whether the IA32_PERF_GLOBAL_CTRL MSR is loaded on VM exit.
        const LOAD_IA32_PERF_GLOBAL_CTRL = 1 << 12;
        /// This control affects VM exits due to external interrupts:
        ///
        /// - If such a VM exit occurs and this control is 1, the logical processor acknowledges
        /// the interrupt controller, acquiring the interrupt’s vector. The vector is stored in
        /// the VM-exit interruption-information field, which is marked valid.
        /// - If such a VM exit occurs and this control is 0, the interrupt is not acknowledged and
        /// the VM-exit interruption-information field is marked invalid.
        const ACK_INTR_ON_EXIT = 1 << 15;
        /// This control determines whether the IA32_PAT MSR is saved on VM exit.
        const SAVE_IA32_PAT = 1 << 18;
        /// This control determines whether the IA32_PAT MSR is loaded on VM exit.
        const LOAD_IA32_PAT = 1 << 19;
        /// This control determines whether the IA32_EFER MSR is saved on VM exit.
        const SAVE_IA32_EFER = 1 << 20;
        /// This control determines whether the IA32_EFER MSR is loaded on VM exit.
        const LOAD_IA32_EFER = 1 << 20;
        /// This control determines whether the value of the VMX-preemption timer is saved on VM exit.
        const SAVE_VMX_PREEMPTION_TIMER_VALUE = 1 << 22;
        /// This control determines whether the IA32_BNDCFGS MSR is cleared on VM exit.
        const CLEAR_IA32_BNDCFGS = 1 << 23;
        /// If this control is 1, Intel Processor Trace does not produce a paging information packet (PIP)
        /// on a VM exit or a VMCS packet on an SMM VM exit (see Chapter 32).
        const CONCEAL_VMX_FROM_PT = 1 << 24;
        /// This control determines whether the IA32_RTIT_CTL MSR is cleared on VM exit.
        const CLEAR_IA32_RTIT_CTL = 1 << 25;
        /// This control determines whether the IA32_LBR_CTL MSR is cleared on VM exit.
        const CLEAR_LBR_CTL = 1 << 26;
        /// This control determines whether CET-related MSRs and SPP are loaded on VM exit.
        const LOAD_CET_STATE = 1 << 28;
        /// This control determines whether the IA32_PKRS MSR is loaded on VM exit.
        const LOAD_PKRS = 1 << 29;
        /// This control determines whether the IA32_PERF_GLOBAL_CTL MSR is saved on VM exit.
        const SAVE_IA32_PERF_GLOBAL_CTL = 1 << 30;
        /// This control determines whether the secondary VM-exit controls are used.
        /// If this control is 0, the logical processor operates as if all the secondary VM-exit controls were also 0.
        const ACTIVATE_SECONDARY_CTL = 1 << 31;
    }
}
