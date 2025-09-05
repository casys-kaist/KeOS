//! Utility to parsing ELF file.
//!
//! The Executable and Linkable Format (ELF, formerly named
//! Extensible Linking Format) is a common standard file format for executable
//! files, object code, shared libraries, and core dumps.
//!
//! ![ELF format](https://raw.githubusercontent.com/casys-kaist/KeOS/79a689838dc34de607bb8d1beb89aa5535cd4af2/images/image.png)
//! An ELF file has the program header, and section headers. The program headers
//! show the segments used at run time, whereas the section header lists the
//! set of sections.
use alloc::vec::Vec;
use core::convert::TryInto;
use keos::{KernelError, fs::RegularFile, mm::page_table::Permission};

/// Represents the ELF file header.
///
/// This structure contains metadata about the ELF file, such as its type,
/// architecture, entry point, and various offsets for program and section
/// headers.
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ELFHeader {
    /// The ELF magic number (`0x7F` followed by `ELF` in ASCII).
    pub magic: [u8; 4],
    /// Indicates 32-bit or 64-bit format.
    pub class: u8,
    /// Specifies little-endian or big-endian encoding.
    pub data: u8,
    /// ELF version (set to `1` for the original and current version).
    pub version: u8,
    /// Identifies the target operating system ABI.
    pub abi: u8,
    /// Further specifies the ABI version.
    pub abi_version: u8,
    /// Unused padding bytes (must be zero).
    pub pad: [u8; 7],
    /// Object file type (e.g., executable, shared object, relocatable).
    pub e_type: u16,
    /// Target instruction set architecture.
    pub e_machine: u16,
    /// ELF version (should be `1`).
    pub e_version: u32,
    /// Memory address of the entry point where execution starts.
    pub e_entry: u64,
    /// Offset of the program header table in bytes.
    pub e_phoff: u64,
    /// Offset of the section header table in bytes.
    pub e_shoff: u64,
    /// Processor-specific flags.
    pub e_flags: u32,
    /// Size of this header in bytes.
    pub e_ehsize: u16,
    /// Size of a program header table entry in bytes.
    pub e_phentsize: u16,
    /// Number of entries in the program header table.
    pub e_phnum: u16,
    /// Size of a section header table entry in bytes.
    pub e_shentsize: u16,
    /// Number of entries in the section header table.
    pub e_shnum: u16,
    /// Index of the section header table entry that contains section names.
    pub e_shstrndx: u16,
}

/// Represents an ELF file in memory.
///
/// This struct provides access to ELF metadata and program headers.
pub struct Elf<'a> {
    /// A parsed ELF header
    pub header: ELFHeader,
    /// Reference to the backing file containing ELF data.
    pub file: &'a RegularFile,
}

impl<'a, 'b> Elf<'a> {
    /// Attempts to create an [`Elf`] object from a [`RegularFile`].
    ///
    /// Returns `Some(Elf)` if the file is a valid ELF binary, otherwise `None`.
    ///
    /// # Validity Checks
    /// - Must have the correct ELF magic bytes (`0x7F ELF`).
    /// - Must be little-endian (`Endian::Little`).
    /// - Must be 64-bit (`Bit::Bit64`).
    /// - Must target the x86-64 architecture (`EMachine::Amd64`).
    pub fn from_file(file: &'a RegularFile) -> Option<Self> {
        union HeaderUnion {
            _raw: [u8; 4096],
            header: ELFHeader,
        }
        let header = unsafe {
            let mut _u = HeaderUnion { _raw: [0; 4096] };
            file.read(0, &mut _u._raw).ok()?;
            _u.header
        };

        if &header.magic == b"\x7FELF"
            && /* Little Endian */ header.data == 1
            && /* Bit64 */ header.class == 2
            && /* Amd64 */ header.e_machine == 0x3E
            && /* Executable file. */ header.e_type == 2
        {
            Some(Self { header, file })
        } else {
            None
        }
    }

    /// Returns an iterator over the program headers.
    pub fn phdrs(&'b self) -> Result<PhdrIterator<'a, 'b>, KernelError> {
        let (base, size) = (self.header.e_phoff.try_into().unwrap(), self.header.e_phnum);
        let mut buffer = alloc::vec![0; size as usize * 0x38];
        self.file.read(base, buffer.as_mut())?;
        Ok(PhdrIterator {
            cursor: 0,
            buffer,
            elf: self,
        })
    }
}

/// Iterator over program headers in an ELF binary.
///
/// This iterator is created using [`Elf::phdrs`].
pub struct PhdrIterator<'a, 'b> {
    cursor: u16,
    elf: &'a Elf<'b>,
    buffer: Vec<u8>,
}

impl<'a, 'b> core::iter::Iterator for PhdrIterator<'a, 'b> {
    type Item = Phdr;
    fn next(&mut self) -> Option<Self::Item> {
        union Reader {
            phdr: Phdr,
            _raw: [u8; 0x38],
        }

        if self.cursor as usize * 0x38 < self.buffer.len() {
            unsafe {
                let ofs = self.cursor as usize * 0x38;
                let mut inner = Reader { _raw: [0; 0x38] };
                inner._raw.copy_from_slice(&self.buffer[ofs..ofs + 0x38]);
                self.cursor += 1;
                Some(inner.phdr)
            }
        } else {
            None
        }
    }
}

/// ELF program header type.
///
/// This enum represents different segment types in an ELF binary.
#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[allow(dead_code)]
pub enum PType {
    /// Unused segment.
    Null = 0x0,
    /// Loadable segment.
    Load = 0x1,
    /// Dynamic linking information.
    Dynamic = 0x2,
    /// Interpreter path (for dynamically linked executables).
    Interp = 0x3,
    /// Auxiliary information.
    Note = 0x4,
    /// Reserved.
    Shlib = 0x5,
    /// Program header table itself.
    Phdr = 0x6,
    /// Thread-Local Storage (TLS) template.
    Tls = 0x7,
    /// GNU-specific: Exception handling information.
    GnuEhFrame = 0x6474e550,
    /// GNU-specific: Stack segment flags.
    GnuStack = 0x6474e551,
    /// GNU-specific: Read-only after relocation.
    GnuRelro = 0x6474e552,
    /// GNU-specific.
    GnuProperty = 0x6474e553,
}

bitflags::bitflags! {
    /// Segment permission flags for ELF program headers.
    ///
    /// These flags specify whether a segment is readable, writable, or executable.
    pub struct PFlags: u32 {
        /// Segment is readable.
        const READ = 1 << 2;
        /// Segment is writable.
        const WRITE = 1 << 1;
        /// Segment is executable.
        const EXECUTABLE = 1 << 0;
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
/// ELF program header for 64-bit binaries.
///
/// Each `Phdr` entry describes a segment or other information needed for
/// execution.
pub struct Phdr {
    /// Segment type.
    pub type_: PType,
    /// Segment permissions.
    pub p_flags: PFlags,
    /// Offset in the file where the segment starts.
    pub p_offset: u64,
    /// Virtual address where the segment should be mapped in memory.
    pub p_vaddr: u64,
    /// Physical address (not commonly used in modern OSes).
    pub p_paddr: u64,
    /// Size of the segment in the file.
    pub p_filesz: u64,
    /// Size of the segment in memory.
    pub p_memsz: u64,
    /// Alignment of the segment (must be a power of two).
    pub p_align: u64,
}

impl Phdr {
    /// Get a ELF segment permissions ([`PFlags`]) of this Phdr in forms of
    /// memory permissions ([`Permission`]).
    ///
    /// This function translates the permission flags of phdr into the
    /// corresponding memory protection flags used by the system and return it.
    /// The conversion ensures that the memory is properly set up according
    /// to the ELF segment's requirements.
    ///
    ///  # Returns
    /// - A `Permission` value representing the phdr's memory permissions.
    pub fn permission(&self) -> Permission {
        let mut permission = Permission::USER;
        todo!()
    }
}
