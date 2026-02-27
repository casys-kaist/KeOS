//! Lazy loader and pager for guest virtual machine.
//!
//! ## Background
//! Lazy loading is a technique that delays resource allocations until the
//! resource is required. Lazy paging in project 3 implements the principle of
//! lazy loading, which involves delaying the reading from file and loading of
//! guest OS's memory (such as text section of guest OS) into the host machine
//! until a specific physical page of the guest is required.
//!
//! Lazy paging in project 3 enhances the performance of the initial booting
//! process since there is no need to read and load all the pages guest OS at
//! once. This approache also helps the reducing the memory usage of the guest
//! OS by avoiding the allocation of memory which is not used
//!
//! In KeV projects, we will launch as KeOS as a guest operting system for the
//! simplicity. The `build.rs` automatically build the guest KeOS (gKeOS) from
//! the `guests/project3/` and build the file system from the `rootfs/` with the
//! compiled gKeOS.
//!
//! KeOS's file system is rough. It only supports file without grow.
//! There is no more abstraction than file such as directory or symbolic link.
//!
//! ### Executable and Linkage Format
//! The Elf (Executable and Linkable Format) is a file format for executable
//! programs in Unix-based operating systems.
//!
//! The Elf header contains information about the Elf metadata, type of file
//! (executable, shared library, object file), the architecture (x86_64, ARM,
//! ...), the entry point (where execution should begin), and others.
//! In summary, Elf header contains the following sections:
//! * e_type: Type of tile
//! * e_machine: Set of the machine instrutions (SPARC, x86_64, ARM, MIPS...)
//! * e_version: Version of the elf (default is 1)
//! * e_entry: Start entry of the program
//! * e_phoff: Start p_offsetf the program header table
//! * e_phnum: Entry size of the program header
//!
//! The Elf Program header (PHDR) contains the information about the program's
//! memory segments (sections of memory allocated for different parts of the
//! program), including their virtual address, physical address, file p_offset
//! sizes, and access permissions. In summary, the Program header contains the
//! following sections:
//! * p_type: Type of the program header
//! * p_offset: Offset in file
//! * p_vaddr: Virtual address to be loaded
//! * p_paddr: Physical address to be loaded
//! * p_filesz: Size on file
//! * p_memsz: Size in memory
//! * p_flags: Flags for Read, Write, Execute
//!
//! The operating system (Hypervisor) loads and executes the program (Guest OS)
//! by parsing the Elf format reading from a file. The operating system should
//! parse the Elf headers to locate and load the program's memory, setup the
//! program's execution environment, and begin executing the program from its
//! entry point.
//!
//! ## Tasks
//! ### Translate kernel entry into Physical Address
//! The initial step to enable the lazy pager is to parse the kernel and
//! populate the [`KernelVmPager`] struct. Unlike the user level Elf program,
//! kernel loading operates on physical address. In this task, you must have to
//! find the entry point of the kernel to be used for initial entry point for
//! the guest kernel. The physical address of the kernel entry point can be
//! obtained by subtracting the virtual address of the [`Phdr`] from the kernel
//! entry address [`Elf::entry()`].
//!
//! ### Load phdr to loader
//! The next step to have to implement is the `load_phdr` to enable the
//! registeration of loaders that map physicall address in the [`Phdr`] to the
//! pager. The implementation requires reading from the kernel image file
//! through `kernel.peeker().file` starting from the specified page offset
//! to page p_offset size. Page offset and the size can be obtained from the
//! [`Phdr`]. See the [`File`] for the apis to operate with the file system.
//!
//! ### Load page to extended page table
//! Lastly, you have to implement `load_page` that called on EPT violation.
//! `load_page` maps a page to the extended page table with permission set to
//! READ, WRITE, and EXECUTABLE. You MUST consider the case that multiple cores
//! trigger EPT violations on the same physical page.
//!
//! [`File`]: keos::fs::File
//! [`Elf`]: crate::keos_vm::elf::Elf
//! [`Phdr`]: crate::keos_vm::elf::Phdr
//! [`Elf::entry()`]: crate::keos_vm::elf::Elf::entry
//! [`map_page`]: KernelVmPager::map_page

use crate::{
    ept::{EptMappingError, EptPteFlags, ExtendedPageTable, Permission},
    keos_vm::elf::{Elf, PType, Phdr},
};
use alloc::{collections::BTreeMap, sync::Arc, vec::Vec};
use keos::{
    addressing::{PAGE_MASK, Pa},
    fs::RegularFile,
    mm::Page,
    sync::SpinLock,
};
use kev::{
    VmError,
    vcpu::VmexitResult,
    vm::{Gpa, Gva},
    vmcs::{ActiveVmcs, ExitReason},
};

pub type PageLoader = Arc<dyn Fn(&mut Page) -> bool + Send + Sync>;

/// Vm Pager of the kernel.
pub struct KernelVmPager {
    ept: ExtendedPageTable,
    pub loaders: BTreeMap<Gpa, PageLoader>,
    entry: usize,
}

impl KernelVmPager {
    /// Create a new vm pager from the kernel image.
    pub fn from_image(kernel: RegularFile, ram_in_kb: usize) -> Option<Self> {
        let kernel = Arc::new(Elf::from_file(&kernel)?);
        let mut pager = Self {
            ept: ExtendedPageTable::new(),
            loaders: BTreeMap::new(),
            entry: 0,
        };

        for p in kernel.phdrs().ok()? {
            if p.type_ == PType::Load {
                pager.load_phdr(p, &kernel).then_some(())?;
            }
        }
        // parse the entry.
        let entry = kernel.header.e_entry;
        pager.entry = todo!();

        // Fill usable mems.
        let empty_pager = Arc::new(|_: &mut Page| true);
        let mut remainder = (ram_in_kb * 1024) / 4096;
        let (kernel_start, kernel_end) = (
            pager.loaders.keys().next().unwrap().into_usize(),
            pager.loaders.keys().last().unwrap().into_usize() + 0x1000,
        );
        remainder -= (kernel_end - kernel_start) / 0x1000;

        for gpa in (0..kernel_start).step_by(0x1000) {
            if remainder == 0 {
                break;
            }
            pager
                .map_page(Gpa::new(gpa).unwrap(), empty_pager.clone())
                .then_some(())?;
            remainder -= 1;
        }

        let mut gpa = kernel_end;
        while remainder > 0 {
            if gpa == 0xbffda000 {
                // Hole for mmio.
                gpa = 0x1_0000_0000;
                continue;
            }
            pager
                .map_page(Gpa::new(gpa).unwrap(), empty_pager.clone())
                .then_some(())?;
            remainder -= 1;
            gpa += 0x1000;
        }

        Some(pager)
    }

    /// Setup the page for mbinfo.
    pub fn finalize_mem(&mut self) -> Option<usize> {
        let mut section_start = self.loaders.keys().next().unwrap();
        let mut section_end = section_start;
        let mut sections = Vec::new();
        for gpa in self.loaders.keys() {
            if *gpa == *section_end + 0x1000 {
                section_end = gpa;
            } else {
                if section_start != section_end {
                    sections.push(*section_start..*section_end + 0x1000);
                }
                section_start = gpa;
                section_end = gpa;
            }
        }
        if section_start != section_end {
            sections.push(*section_start..*section_end + 0x1000);
        }
        assert!(self.loaders.remove(&Gpa::new(0).unwrap()).is_some());

        pub struct MbiWriter {
            page: Page,
            pos: usize,
        }

        impl MbiWriter {
            fn new() -> Option<Self> {
                Some(Self {
                    page: Page::new(),
                    pos: 4,
                })
            }
            fn write_u32(&mut self, b: u32) -> &mut Self {
                self.page.inner_mut()[self.pos..self.pos + 4].copy_from_slice(&b.to_le_bytes());
                self.pos += 4;
                self
            }
            fn write_u64(&mut self, b: u64) -> &mut Self {
                self.page.inner_mut()[self.pos..self.pos + 8].copy_from_slice(&b.to_le_bytes());
                self.pos += 8;
                self
            }
            fn write_memory_info_head(&mut self, entry_cnt: u32) -> &mut Self {
                self.write_u32(6) // Field.ty
                    .write_u32(16 + entry_cnt * 24) // Field.size
                    .write_u32(24) // stride
                    .write_u32(0) // version
            }
            fn write_memory_info(&mut self, base_addr: u64, length: u64, ty: u32) -> &mut Self {
                self.write_u64(base_addr)
                    .write_u64(length)
                    .write_u32(ty)
                    .write_u32(0)
            }
            fn finalize(self) -> Page {
                let Self { mut page, pos } = self;
                page.inner_mut()[0..4].copy_from_slice(&(pos as u32).to_le_bytes());
                page
            }
        }

        let mut writer = MbiWriter::new()?;
        writer
            .write_u32(0) // MutiBootInfo2._rev
            .write_memory_info_head(sections.len() as u32);
        for s in sections.into_iter() {
            writer.write_memory_info(
                s.start.into_usize() as u64,
                (s.end.into_usize() - s.start.into_usize()) as u64,
                1, // Usable.
            );
        }
        self.ept
            .map(Gpa::new(0).unwrap(), writer.finalize(), Permission::all())
            .expect("Failed to insert page for multiboot info");
        Some(0)
    }

    // Register loaders of the PAs in the phdr to the pager.
    fn load_phdr(&mut self, phdr: Phdr, kernel: &Arc<Elf>) -> bool {
        todo!()
    }

    /// Get a entry point of the this kernel.
    #[inline]
    pub fn entry(&self) -> usize {
        self.entry
    }

    /// Attach a mmio page at `gpa`.
    #[inline]
    pub fn map_mmio_page(&mut self, gpa: Gpa, page: Page) -> Result<(), EptMappingError> {
        self.ept
            .map(gpa, page, Permission::READ | Permission::EXECUTABLE)
    }

    /// Attach a page at `gpa`.
    #[inline]
    pub fn map_page(&mut self, gpa: Gpa, loader: PageLoader) -> bool {
        assert_eq!(gpa.into_usize() & 0xfff, 0);
        assert!(self.loaders.insert(gpa, loader).is_none());
        true
    }

    /// Get ept ptr of the pager.
    #[inline]
    pub fn ept_ptr(&self) -> Pa {
        self.ept.pa()
    }

    fn load_page(&mut self, gpa: Gpa) -> bool {
        todo!()
    }

    /// Handle the ept violation and load the corresponding page.
    pub fn try_lazy_paging(&mut self, reason: ExitReason) -> Result<VmexitResult, VmError> {
        if let kev::vmcs::BasicExitReason::EptViolation { fault_addr, .. } =
            reason.get_basic_reason()
            && let Some(gpa) = fault_addr
        {
            let gpa = Gpa::new(gpa.into_usize() & !PAGE_MASK).unwrap();
            if self.load_page(gpa) {
                return Ok(VmexitResult::Ok);
            }
        }
        Err(VmError::HandleVmexitFailed(reason))
    }
}

impl kev::Probe for KernelVmPager {
    fn gpa2hpa(&self, vmcs: &ActiveVmcs, gpa: Gpa) -> Option<Pa> {
        self.ept.gpa2hpa(vmcs, gpa)
    }
    fn gva2hpa(&self, vmcs: &ActiveVmcs, gva: Gva) -> Option<Pa> {
        self.ept.gva2hpa(vmcs, gva)
    }
}

pub struct Probe<'a> {
    pub inner: &'a SpinLock<KernelVmPager>,
}

impl<'a> kev::Probe for Probe<'a> {
    fn gpa2hpa(&self, vmcs: &ActiveVmcs, gpa: Gpa) -> Option<Pa> {
        let guard = self.inner.lock();
        let ret = guard.gpa2hpa(vmcs, gpa);
        guard.unlock();
        ret
    }

    fn gva2hpa(&self, vmcs: &ActiveVmcs, gva: Gva) -> Option<Pa> {
        let guard = self.inner.lock();
        let ret = guard.gva2hpa(vmcs, gva);
        guard.unlock();
        ret
    }
}
