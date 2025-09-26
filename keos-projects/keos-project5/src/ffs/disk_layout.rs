//! **on-disk layout** of the file system's metadata structures.
//!
//! This module defines the **on-disk layout** of the file system's core
//! metadata structures. These types represent raw disk-resident data such as
//! the superblock, block/inode allocation bitmaps, and inode table. Each struct
//! in this module is tightly packed and designed to match the exact binary
//! layout used by the file system when persisting and loading from disk.
//!
//! This module defines [`MetaData`] trait, and support [`MetaData::load`]
//! method, allowing them to be generically loaded from a logical block address
//! (LBA).
//!
//! ## Examples of accessing metadata.
//! ```
//! use crate::ffs::disk_layout::{MetaData, BlockBitmap};
//!
//! let block_bitmap = BlockBitmap::load(ffs, lba)?; // read BlockPointsTo<BlockBitMap> from lba.
//! { // READ
//!     let guard = block_bitmap.read(); // reading metadata does not requires transaction.
//!     assert!(guard.is_allocated(0));
//! }
//! { // WRITE
//!     let tx = ffs.open_transaction(); // Open a new transaction. Only a single transaction can be existed.
//!     let mut guard = block_bitmap.write(&tx); // writing metadata requires transaction.
//!     guard.try_allocate(1);
//!     guard.submit(); // Submit the modified change to the transaction.
//! }
//! ```
use crate::ffs::{
    FastFileSystemInner, InodeNumber, JournalIO, LogicalBlockAddress, access_control::MetaData,
};
use alloc::boxed::Box;
use core::fmt::Debug;
use keos::{KernelError, fs::Disk};

/// A struct for denying implementing [`MetaData`] from outside of this module.
#[doc(hidden)]
pub struct Private {
    _p: (),
}

/// On-disk representation of the superblock of the Fast File System (FFS).
///
/// The superblock contains essential metadata about the filesystem,
/// including the total number of blocks and inodes, as well as the
/// size of the journal.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SuperBlock {
    /// File system magic: "KeOSFFS\0".
    pub magic: [u8; 8],
    /// Total number of blocks in the filesystem.
    pub block_count: u64,
    /// In-used count of blocks in the filesystem.
    pub block_count_inused: u64,
    /// Total number of inodes in the filesystem.
    pub inode_count: u64,
    /// In-used count of inodes in the filesystem.
    pub inode_count_inused: u64,
    /// A indicator that this filesystem have journaling feature.
    pub has_journal: u64,
    /// Padding to align to Block size.
    pub _pad: [u8; 4096 - core::mem::size_of::<u64>() * 5 - 8],
}

impl Default for SuperBlock {
    fn default() -> Self {
        Self {
            magic: [0; 8],
            block_count: 0,
            block_count_inused: 0,
            inode_count: 0,
            inode_count_inused: 0,
            has_journal: 0,
            _pad: [0; 4096 - 48],
        }
    }
}

impl Debug for SuperBlock {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SuperBlock")
            .field("magic", &self.magic)
            .field("block_count", &self.block_count)
            .field("block_count_inused", &self.block_count_inused)
            .field("inode_count", &self.inode_count)
            .field("inode_count_used", &self.inode_count_inused)
            .field("has_journal", &(self.has_journal != 0))
            .finish()
    }
}

impl MetaData for SuperBlock {
    const P: Private = Private { _p: () };
    fn load(
        _ffs: &FastFileSystemInner,
        _lba: LogicalBlockAddress,
    ) -> Result<super::access_control::BlockPointsTo<Self>, KernelError> {
        unreachable!()
    }
}
const_assert!(core::mem::size_of::<SuperBlock>() == 4096);

/// Represents the on-disk block allocation bitmap for the file system.
///
/// Each bit in the [`BlockBitmap`] corresponds to a single block on disk.
/// A bit value of `1` indicates that the block is in use, while `0`
/// means the block is free and available for allocation.
#[repr(C, packed)]
pub struct BlockBitmap {
    bits: [u64; 4096 / 8],
}

impl Default for BlockBitmap {
    fn default() -> Self {
        BlockBitmap {
            bits: [0; 4096 / 8],
        }
    }
}

impl BlockBitmap {
    /// Checks whether a block at the given position is allocated.
    ///
    /// # Parameters
    /// - `pos`: The index of the block to check.
    ///
    /// # Returns
    /// - `true` if the block is currently marked as allocated (bit is 1).
    /// - `false` if the block is free (bit is 0).
    ///
    /// This method is used to determine the allocation status of a block
    /// in the file system's block bitmap.
    pub fn is_allocated(&self, pos: usize) -> bool {
        let (pos, off) = (pos / 64, pos % 64);
        self.bits[pos] & (1 << off) != 0
    }

    /// Attempts to allocate a block at the given position.
    ///
    /// # Parameters
    /// - `pos`: The index of the block to allocate.
    ///
    /// # Returns
    /// - `true` if the block was previously free and is now marked as
    ///   allocated.
    /// - `false` if the block was already allocated (no change).
    ///
    /// This method is used during block allocation to claim a free block.
    /// If the block is already allocated, it fails without modifying the
    /// bitmap.
    pub fn try_allocate(&mut self, pos: usize) -> bool {
        let (pos, off) = (pos / 64, pos % 64);
        if self.bits[pos] & (1 << off) == 0 {
            self.bits[pos] |= 1 << off;
            true
        } else {
            false
        }
    }

    /// deallocate a block at the given position.
    ///
    /// # Parameters
    /// - `pos`: The index of the block to deallocate.
    ///
    /// # Returns
    /// - `true` if the block was previously free and is now marked as
    ///   allocated.
    /// - `false` if the block was already allocated (no change).
    ///
    /// This method is used during block allocation to claim a free block.
    /// If the block is already allocated, it fails without modifying the
    /// bitmap.
    pub fn deallocate(&mut self, pos: usize) -> bool {
        let (pos, off) = (pos / 64, pos % 64);
        if self.bits[pos] & (1 << off) != 0 {
            self.bits[pos] &= !(1 << off);
            true
        } else {
            false
        }
    }
}

impl MetaData for BlockBitmap {
    const P: Private = Private { _p: () };
}
const_assert!(core::mem::size_of::<BlockBitmap>() == 4096);

/// Represents the on-disk inode allocation bitmap for the file system.
///
/// Each bit in the [`InodeBitmap`] corresponds to a single inode on disk.
/// A bit value of `1` indicates that the inode is in use, while `0`
/// means the inode is free and available for allocation.
#[repr(C, packed)]
pub struct InodeBitmap {
    bits: [u64; 4096 / 8],
}

impl Default for InodeBitmap {
    fn default() -> Self {
        InodeBitmap {
            bits: [0; 4096 / 8],
        }
    }
}

impl InodeBitmap {
    /// Checks whether a inode at the given position is allocated.
    ///
    /// # Parameters
    /// - `pos`: The index of the inode to check.
    ///
    /// # Returns
    /// - `true` if the inode is currently marked as allocated (bit is 1).
    /// - `false` if the inode is free (bit is 0).
    ///
    /// This method is used to determine the allocation status of a inode
    /// in the file system's inode bitmap.
    pub fn is_allocated(&self, pos: usize) -> bool {
        let (pos, off) = (pos / 64, pos % 64);
        self.bits[pos] & (1 << off) != 0
    }

    /// Attempts to allocate a inode at the given position.
    ///
    /// # Parameters
    /// - `pos`: The index of the inode to allocate.
    ///
    /// # Returns
    /// - `true` if the inode was previously free and is now marked as
    ///   allocated.
    /// - `false` if the inode was already allocated (no change).
    ///
    /// This method is used during inode allocation to claim a free inode.
    /// If the inode is already allocated, it fails without modifying the
    /// bitmap.
    pub fn try_allocate(&mut self, pos: usize) -> bool {
        let (pos, off) = (pos / 64, pos % 64);
        if self.bits[pos] & (1 << off) == 0 {
            self.bits[pos] |= 1 << off;
            true
        } else {
            false
        }
    }

    pub fn deallocate(&mut self, pos: usize) -> bool {
        let (pos, off) = (pos / 64, pos % 64);
        if self.bits[pos] & (1 << off) != 0 {
            self.bits[pos] &= !(1 << off);
            true
        } else {
            false
        }
    }
}

impl MetaData for InodeBitmap {
    const P: Private = Private { _p: () };
}
const_assert!(core::mem::size_of::<InodeBitmap>() == 4096);

/// Represent a single inode within a inode array.
#[derive(Debug)]
#[repr(C, packed)]
pub struct Inode {
    /// File system magic: "KeOSFFSI".
    pub magic: [u8; 8],
    /// The unique inode number assigned to this file or directory.
    pub ino: Option<InodeNumber>,
    /// The type of the file (e.g., regular file, directory, symbolic link).
    ///
    /// Uses a `u32` to store values corresponding to [`FileType`].
    ///
    /// [`FileType`]: crate::ffs::types::FileType
    pub ftype: u32,
    /// The total size of the file in bytes.
    pub size: u64,
    /// The number of links alive in the file system.
    pub link_count: u64,
    /// Directly mapped data blocks.
    ///
    /// These 12 blocks store the first portions of a file's data, allowing
    /// for efficient access to small files without requiring indirect blocks.
    pub dblocks: [Option<LogicalBlockAddress>; 12],
    /// An indirect block, which contains pointers to additional data
    /// blocks.
    ///
    /// This extends the file size capability beyond direct blocks by storing
    /// an array of logical block addresses in a separate block.
    pub iblock: Option<LogicalBlockAddress>,
    /// A doubly indirect block, which contains pointers to indirect
    /// blocks.
    ///
    /// This allows for even larger file sizes by introducing an extra level
    /// of indirection.
    pub diblock: Option<LogicalBlockAddress>,
    /// A padding to align to the power of two.
    pub _pad: [u8; 112],
}

impl Default for Inode {
    fn default() -> Self {
        Self {
            magic: [0; 8],
            ino: None,
            ftype: 0,
            size: 0,
            link_count: 0,
            dblocks: [None; 12],
            iblock: None,
            diblock: None,
            _pad: [0; 112],
        }
    }
}
const_assert!(core::mem::size_of::<Inode>() == 256);

/// Represents a fixed-size array of inodes loaded from a block.
///
/// Each [`InodeArray`] holds a collection of [`Inode`] structures that are
/// packed into a single 4KB block. The number of inodes it contains is computed
/// based on the size of the [`Inode`] type.
#[derive(Default)]
#[repr(C, packed)]
pub struct InodeArray {
    inodes: [Inode; 4096 / core::mem::size_of::<Inode>()],
}

impl core::ops::Deref for InodeArray {
    type Target = [Inode; 4096 / core::mem::size_of::<Inode>()];
    fn deref(&self) -> &Self::Target {
        &self.inodes
    }
}

impl core::ops::DerefMut for InodeArray {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inodes
    }
}

impl MetaData for InodeArray {
    const P: Private = Private { _p: () };
}
const_assert!(core::mem::size_of::<InodeArray>() == 4096);

/// Represents an indirect block in the filesystem.
///
/// An indirect block is used to extend the number of data blocks that an inode
/// can reference. Instead of storing data directly, it contains a list of
/// logical block addresses (LBAs), each pointing to a separate data block on
/// disk.
///
/// This structure allows files to grow beyond the direct block limit imposed by
/// the inode structure.
///
/// # Usage
/// Typically used as part of indirect, or double-indirectblock addressing
/// schemes to support large file sizes.
#[repr(C)]
pub struct IndirectBlock {
    lbas: [Option<LogicalBlockAddress>; 512],
}

impl Default for IndirectBlock {
    fn default() -> Self {
        Self { lbas: [None; 512] }
    }
}

impl core::ops::Deref for IndirectBlock {
    type Target = [Option<LogicalBlockAddress>; 512];
    fn deref(&self) -> &Self::Target {
        &self.lbas
    }
}

impl core::ops::DerefMut for IndirectBlock {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.lbas
    }
}

impl MetaData for IndirectBlock {
    const P: Private = Private { _p: () };
}

const_assert!(core::mem::size_of::<IndirectBlock>() == 4096);

/// Represent a single directory entry within a directory block.
///
/// Each entry stores metadata for necessary to locate a file or subdirectory.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct DirectoryBlockEntry {
    /// The inode associated with this directory entry.
    /// - `Some(inode)`: a valid file or directory.
    /// - `None`: an unused or deleted entry.
    pub inode: Option<InodeNumber>,
    /// The length of the file or directory name stored in `name`.
    /// This indicates how many bytes in the `name` array are valid.
    pub name_len: u8,
    /// The name of the file or directory.
    /// Only the first `name_len` bytes are meaningful.
    pub name: [u8; 251],
}

impl Default for DirectoryBlockEntry {
    fn default() -> Self {
        Self {
            inode: None,
            name_len: 0,
            name: [0; 251],
        }
    }
}

impl DirectoryBlockEntry {
    /// Constructs a new directory entry from an inode number and name.
    ///
    /// Returns `None` if the name is too long to fit in the directory entry.
    ///
    /// # Arguments
    /// - `ino`: The inode number associated with the entry.
    /// - `name`: The name of the file or directory.
    ///
    /// # Returns
    /// - `Some(Self)`: A valid directory entry.
    /// - `None`: If the name is too long to fit.
    pub fn from_ino_name(ino: InodeNumber, name: &str) -> Option<Self> {
        let name_len = name.len();
        if name_len <= 251 {
            let mut out = DirectoryBlockEntry {
                inode: Some(ino),
                name_len: name_len as u8,
                name: [0; 251],
            };
            out.name[..name_len].copy_from_slice(name.as_bytes());
            Some(out)
        } else {
            None
        }
    }

    /// Returns the name of the directory entry as a string slice.
    ///
    /// # Returns
    /// - `Some(&str)`: If the name is valid UTF-8.
    /// - `None`: If the name contains invalid UTF-8 bytes.
    pub fn name(&self) -> Option<&str> {
        self.inode
            .and_then(|_| core::str::from_utf8(&self.name[..self.name_len as usize]).ok())
    }
}

const_assert!(core::mem::size_of::<DirectoryBlockEntry>() == 256);

/// Represents a block that contains multiple directory entries.
///
/// A directory is composed of one or more of these blocks, each
/// containing a fixed-size array of directory entries.
#[repr(C)]
pub struct DirectoryBlock {
    entries: [DirectoryBlockEntry; 4096 / core::mem::size_of::<DirectoryBlockEntry>()],
}

impl Default for DirectoryBlock {
    fn default() -> Self {
        Self {
            entries: [DirectoryBlockEntry::default();
                4096 / core::mem::size_of::<DirectoryBlockEntry>()],
        }
    }
}

impl core::ops::Deref for DirectoryBlock {
    type Target = [DirectoryBlockEntry; 4096 / core::mem::size_of::<DirectoryBlockEntry>()];
    fn deref(&self) -> &Self::Target {
        &self.entries
    }
}

impl core::ops::DerefMut for DirectoryBlock {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.entries
    }
}

impl MetaData for DirectoryBlock {
    const P: Private = Private { _p: () };
}

const_assert!(core::mem::size_of::<DirectoryBlock>() == 4096);

/// Represents the on-disk metadata for the journal superblock.
#[repr(C, packed)]
pub struct JournalSb {
    /// Journal magic: "KeOSJOUR".
    pub magic: [u8; 8],
    /// Indicate journal has been commited.
    pub commited: u64,
    /// Transaction id.
    pub tx_id: u64,
    /// Padding to fill a full block (4096 bytes).
    _pad: [u8; 4096 - 24],
}

impl Default for JournalSb {
    fn default() -> Self {
        Self {
            magic: [0; 8],
            commited: 0,
            tx_id: 0,
            _pad: [0; 4096 - 24],
        }
    }
}

impl JournalSb {
    /// Loads the journal superblock from disk.
    ///
    /// # Arguments
    /// - `disk`: The underlying disk interface.
    /// - `lba`: The logical block address where the journal superblock is
    ///   located.
    ///
    /// # Returns
    /// - `Ok(Box<Self>)`: A parsed journal superblock.
    /// - `Err(KernelError)`: If the block could not be read or parsed.
    pub fn from_disk(disk: &Disk, lba: LogicalBlockAddress) -> Result<Box<Self>, KernelError> {
        let mut b = Box::new(JournalSb::default());
        {
            let inner =
                unsafe { core::slice::from_raw_parts_mut(&mut *b as *mut _ as *mut u8, 4096) };
            for i in 0..8 {
                disk.read(
                    lba.into_sector() + i,
                    inner[512 * i..512 * (i + 1)].as_mut_array().unwrap(),
                )?;
            }
        }
        Ok(b)
    }

    /// Writes the current journal superblock to disk.
    ///
    /// This updates the on-disk metadata for the journal ring buffer.
    ///
    /// # Arguments
    /// - `io`: The I/O interface to the journal.
    /// - `ffs`: Reference to the file system's metadata layout.
    ///
    /// # Returns
    /// - `Ok(())`: If the write succeeds.
    /// - `Err(KernelError)`: If the write fails.
    pub fn writeback(&self, io: &JournalIO, ffs: &FastFileSystemInner) -> Result<(), KernelError> {
        let lba = ffs.journal().start;
        {
            let inner = unsafe { core::slice::from_raw_parts(self as *const _ as *const u8, 4096) };
            io.write_metadata_block(lba, inner.as_array().unwrap())?;
        }
        Ok(())
    }
}

const_assert!(core::mem::size_of::<JournalSb>() == 4096);

/// Represents a journal transaction header used to track modified blocks.
///
/// This structure is used during transaction commit to list all
/// blocks that are affected by the transaction.
pub struct JournalTxBegin {
    /// Transaction id.
    pub tx_id: u64,
    /// Array of logical block addresses involved in the transaction.
    /// - `Some(lba)`: a block to be committed.
    /// - `None`: an unused slot.
    pub lbas: [Option<LogicalBlockAddress>; 511],
}

impl JournalTxBegin {
    /// Creates a new, empty journal `TxBegin` block.
    pub fn new(tx_id: u64) -> Box<Self> {
        Box::new(Self {
            tx_id,
            lbas: [None; 511],
        })
    }

    /// Loads a journal `TxBegin` block from disk at the specified LBA.
    ///
    /// # Arguments
    /// - `io`: Interface for reading blocks from disk.
    /// - `lba`: Logical block address of the journal transaction block to load.
    ///
    /// # Returns
    /// - `Ok(Box<Self>)`: The loaded journal transaction block.
    /// - `Err(KernelError)`: If the block could not be read or parsed.
    pub fn from_io(io: &JournalIO, lba: LogicalBlockAddress) -> Result<Box<Self>, KernelError> {
        let mut b = Self::new(0);
        {
            let inner =
                unsafe { core::slice::from_raw_parts_mut(&mut *b as *mut _ as *mut u8, 4096) };
            io.read_journal(lba, inner.as_mut_array().unwrap())?;
        }
        Ok(b)
    }

    /// Converts this transaction into a 4096-byte block suitable for writing to
    /// disk.
    ///
    /// This is typically used during transaction commit.
    pub fn into_block(self: Box<Self>) -> Box<[u8; 4096]> {
        unsafe { Box::from_raw(Box::into_raw(self) as *mut [u8; 4096]) }
    }
}

const_assert!(core::mem::size_of::<JournalTxBegin>() == 4096);

/// Represents a journal transaction is ended.
///
/// This structure is used during transaction commit to mark the end of
/// transaction.
pub struct JournalTxEnd {
    /// Transaction id.
    pub tx_id: u64,
    _pad: [u8; 4088],
}

impl JournalTxEnd {
    /// Creates a new, empty journal `TxEnd` block.
    pub fn new(tx_id: u64) -> Box<Self> {
        Box::new(Self {
            tx_id,
            _pad: [0; 4088],
        })
    }

    /// Loads a journal `TxEnd` block from disk at the specified LBA.
    ///
    /// # Arguments
    /// - `io`: Interface for reading blocks from disk.
    /// - `lba`: Logical block address of the journal transaction block to load.
    ///
    /// # Returns
    /// - `Ok(Box<Self>)`: The loaded journal transaction block.
    /// - `Err(KernelError)`: If the block could not be read or parsed.
    pub fn from_io(io: &JournalIO, lba: LogicalBlockAddress) -> Result<Box<Self>, KernelError> {
        let mut b = Self::new(0);
        {
            let inner =
                unsafe { core::slice::from_raw_parts_mut(&mut *b as *mut _ as *mut u8, 4096) };
            io.read_journal(lba, inner.as_mut_array().unwrap())?;
        }
        Ok(b)
    }

    /// Converts this transaction into a 4096-byte block suitable for writing to
    /// disk.
    ///
    /// This is typically used during transaction commit.
    pub fn into_block(self: Box<Self>) -> Box<[u8; 4096]> {
        unsafe { Box::from_raw(Box::into_raw(self) as *mut [u8; 4096]) }
    }
}

const_assert!(core::mem::size_of::<JournalTxEnd>() == 4096);
