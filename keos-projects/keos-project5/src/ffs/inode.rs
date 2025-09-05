//! ## Inode abstraction.
//!
//! In a Unix-like file system, every object in a file system is represented by
//! a data structure known as an **inode**. An **inode** (index node) stores
//! metadata about a file system object, including its size, permissions,
//! timestamps, and pointers to the underlying data blocks.
//!
//! ### Inode Internals
//! An `Inode` is the authoritative metadata structure for a file or
//! directory. It stores essential information such as the object’s type, size,
//! and pointers to data blocks, serving as the central handle for locating and
//! managing file data.
//!
//! At its core, an inode functions as an **indexing structure**: it maps a
//! [`FileBlockNumber`] (a block’s position relative to the file) to a
//! [`LogicalBlockAddress`] (the actual block location on disk). This mapping
//! enables the file system to translate file-relative accesses into physical
//! disk operations, bridging the logical view of a file with the underlying
//! storage layout.
//!
//! While the on-disk inode structure provides persistence,
//! the kernel maintains in-memory inodes to speed up access and coordinate
//! concurrent operations. To ensure this, KeOS provides the
//! [`FastFileSystemInner::get_inode`] API to maintains **single, global view**
//! of each inode inside the kernel.
//!
//! The function looks up an inode number ([`InodeNumber`]) and returns a
//! [`TrackedInode`], which is a consistent view for a reference-counted,
//! thread-safe wrapper around the in-memory inode. All kernel subsystems
//! interact with inodes through this wrapper, ensuring proper synchronization.
//!
//! A [`TrackedInode`] provides two key capabilities:
//!
//! - **Read access** via [`TrackedInode::read`], which acquires a shared guard
//!   for inspecting inode state (e.g., file size, permissions) without
//!   modification.
//! - **Write access** via [`TrackedInode::write_with`], which locks both the
//!   in-memory and on-disk inode. Writes are performed inside a closure that
//!   receives a [`TrackedInodeWriteGuard`]. Changes must be explicitly
//!   finalized with [`TrackedInodeWriteGuard::submit`], ensuring that memory
//!   and disk stay consistent.
//!
//! Together, `get_inode` and `TrackedInode` enforce a disciplined access model:
//! there is exactly one in-memory representation of each inode, guarded by
//! lock. This design keeps inode state consistent across memory and disk, even
//! under concurrent file system activity.
//!
//! ### Inode Indexing in FFS
//! An inode does not store file data directly. Instead, it contains pointers
//! to **data blocks** that hold the file’s contents. To balance efficiency
//! for small files with scalability for large files, FFS adopts a tiered
//! indexing scheme as follow:
//! ```text
//!              ┌───────────────────────────┐
//!              │         Inode             │
//!              ├───────────────────────────┤
//!              │ dblocks[0] → Data blk 0   │
//!              │ dblocks[1] → Data blk 1   │
//!              │ ...                       │
//!              │ dblocks[11] → Data blk11  │
//!              │                           │
//!              │ iblock 0 ───────────────┐ │
//!              │                         │ │
//!              │ diblock ─────────────┐  │ │
//!              └──────────────────────┬──┬─┘
//!                                     │  │
//!          ┌──────────────────────────┘  │
//!   ┌──────▼───────┐                 ┌───▼──────────┐
//!   │ Double ind.  │                 │ Indirect     │
//!   ├──────────────┤                 ├──────────────┤
//!   │ → iblock 1   │                 │ → Data blk12 │
//!   │ → iblock 2   │─┐               │ → Data blk13 │
//!   │ ..           │ │               │ ...          │
//!   │ → iblock 512 │ │               │ → Data blk523│
//!   └──────────────┘ │               └──────────────┘
//!                    │
//!     ┌──────────────┘
//! ┌───▼──────────┐
//! │ Indirect blk │
//! ├──────────────┤
//! │ → Data blk X │
//! │ → Data blk Y │
//! │ ...          │
//! └──────────────┘
//! ```
//!
//! - **Direct blocks (`dblocks`)** The first 12 pointers directly reference
//!   data blocks. This makes access to small files very fast, as no additional
//!   lookup is required. Small files can therefore be served entirely from
//!   these direct entries.
//
//! - **Indirect block (`iblock`)** When the file grows beyond the direct
//!   blocks, the inode refers to a single **indirect block**. This block is
//!   itself an array of data block pointers, extending the maximum file size
//!   significantly.
//
//! - **Double indirect block (`diblock`)** For even larger files, the inode
//!   uses a **double indirection**. The inode points to a block that contains
//!   pointers to *indirect blocks*, each of which then contains pointers to
//!   data blocks. This extra level of indirection allows extremely large files
//!   to be addressed.
//!
//! Together, these three levels form a hierarchical mapping from a
//! [`FileBlockNumber`] (position within a file) to a
//! [`LogicalBlockAddress`] (actual block on disk).
//!
//! Your task is to implement the two core file access functions based on the
//! indexing structure: [`Inode::get`] and [`Inode::grow`].
//!
//! - [`Inode::get`] retrieves the disk location of a specific file block. It
//!   traverses the inode’s indexing structure and returns the corresponding
//!   disk block address, or `None` if the block has not been allocated.
//!
//! - [`Inode::grow`] ensures that the inode can cover a target file block
//!   number. If needed, it allocates new blocks and updates the inode’s
//!   indexing structure. All modifications are performed transactionally to
//!   guarantee consistency.
//!
//! These functions use [`MetaData::load`] to access or create
//! [`IndirectBlock`]s, and they update these blocks via the transaction API
//! (using [`BlockPointsTo::write`] and
//! [`BlockPointsToWriteGuard::submit`]). For reference on using these APIs, you
//! may consult the documentation or implementation of the followings:
//! - [`access_control`]
//! - [`Directory::add_entry`]
//!
//! While implementing the requirements, you may encounter
//! [`RunningTransaction`] struct. You do not need to understand it until the
//! implementation of the journaling; for now, simply pass a reference to the
//! required methods.
//!
//! ## Implementation Requirements
//! You need to implement the followings:
//! - [`Inode::get`]
//! - [`Inode::grow`]
//!
//! After implement the functionalities, move on to the next [`section`].
//!
//! [`section`]: mod@crate::ffs::fs_objects
//! [`IndirectBlock`]: crate::ffs::disk_layout::IndirectBlock
use super::access_control::TrackedInodeWriteGuard;
use crate::ffs::{
    FastFileSystemInner, FileBlockNumber, InodeNumber, LogicalBlockAddress,
    access_control::MetaData, disk_layout, journal::RunningTransaction, types::FileType,
};
#[cfg(doc)]
use crate::ffs::{
    access_control::{self, BlockPointsTo, BlockPointsToWriteGuard, TrackedInode},
    fs_objects::Directory,
};
use keos::KernelError;
#[cfg(doc)]
use keos::fs::traits::Directory as _Directory;

/// Represents an inode in memory, the metadata structure for a file or
/// directory.
///
/// An inode stores essential information about a file, including its size,
/// type, and the locations of its data blocks.
#[derive(Debug)]
pub struct Inode {
    /// The unique inode number assigned to this file or directory.
    pub ino: InodeNumber,
    /// The type of the file (e.g., regular file, directory, symbolic link).
    ///
    /// Uses a `u32` to store values corresponding to [`FileType`].
    pub ftype: FileType,
    /// The total size of the file in bytes.
    pub size: usize,
    /// Number of links alive in the filesystem.
    pub link_count: usize,
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
}

impl Inode {
    /// Constructs an in-memory [`Inode`] from its on-disk representation.
    ///
    /// # Arguments
    /// - `inode`: A reference to the [`disk_layout::Inode`] structure loaded
    ///   from disk.
    ///
    /// # Returns
    /// - `Ok(Self)`: If the conversion succeeds.
    /// - `Err(KernelError)`: If the inode contains invalid or inconsistent
    ///   data.
    ///
    /// This function performs necessary validation and translation between the
    /// raw on-disk layout and the structured, in-memory format used by the
    /// filesystem.
    pub(crate) fn from_disk_layout(inode: &disk_layout::Inode) -> Result<Self, KernelError> {
        if &inode.magic != b"KeOSFFSI" {
            return Err(KernelError::FilesystemCorrupted("Inode Magic Mismatch"));
        }
        Ok(Self {
            ino: inode
                .ino
                .ok_or(KernelError::FilesystemCorrupted("Inode number is zero"))?,
            ftype: FileType::try_from(inode.ftype)?,
            size: inode.size as usize,
            link_count: inode.link_count as usize,
            dblocks: inode.dblocks,
            iblock: inode.iblock,
            diblock: inode.diblock,
        })
    }

    /// Converts the in-memory [`Inode`] structure into its on-disk
    /// representation.
    ///
    /// This is used when persisting an inode to disk, typically during
    /// checkpointing or journal submission. The returned [`disk_layout::Inode`]
    /// struct matches the format expected by the on-disk inode array.
    pub fn into_disk_format(&self) -> disk_layout::Inode {
        disk_layout::Inode {
            magic: *b"KeOSFFSI",
            ino: Some(self.ino),
            ftype: match self.ftype {
                FileType::RegularFile => 0,
                FileType::Directory => 1,
            },
            size: self.size as u64,
            link_count: self.link_count as u64,
            dblocks: self.dblocks,
            iblock: self.iblock,
            diblock: self.diblock,
            _pad: [0; 112],
        }
    }

    /// Creates a new in-memory [`Inode`] instance.
    ///
    /// This function is used to initialize a fresh inode in memory before it is
    /// ever written to disk. It sets the inode number and whether the inode
    /// represents a directory.
    ///
    /// # Parameters
    /// - `ino`: The inode number.
    /// - `is_dir`: Whether this inode represents a directory (`true`) or a file
    ///   (`false`).
    ///
    /// # Returns
    /// A new [`Inode`] instance ready to be inserted into the inode table.
    pub(crate) fn new(ino: InodeNumber, is_dir: bool) -> Self {
        Self {
            ino,
            ftype: if is_dir {
                FileType::Directory
            } else {
                FileType::RegularFile
            },
            size: 0,
            link_count: 0,
            dblocks: [None; 12],
            iblock: None,
            diblock: None,
        }
    }

    /// Retrieves the logical block address (LBA) corresponding to a file block.
    ///
    /// # Arguments
    /// - `ffs`: Reference to the file system.
    /// - `fba`: [`FileBlockNumber`], relative to the beginning of the file.
    ///
    /// # Returns
    /// - `Ok(lba)`: The logical block address where the specified file block is
    ///   stored.
    /// - `Err(KernelError)`: If the block is not allocated or the block number
    ///   is out of bounds.
    pub fn get(
        &self,
        ffs: &FastFileSystemInner,
        fba: FileBlockNumber,
    ) -> Result<Option<LogicalBlockAddress>, KernelError> {
        if 0x1000 * fba.0 >= self.size {
            return Ok(None);
        }
        todo!()
    }

    /// Grows the inode to include at least the given number of file blocks.
    ///
    /// # Arguments
    /// - `ffs`: Reference to the file system.
    /// - `until`: The target [`FileBlockNumber`] (inclusive) that the inode
    ///   should grow to cover.
    /// - `tx`: The running transaction used to log allocation changes.
    ///
    /// # Returns
    /// - `Ok(())`: If the inode was successfully extended.
    /// - `Err(KernelError)`: If allocation fails or the inode cannot be grown.
    ///
    /// This function ensures that all blocks up to `until` are allocated,
    /// performing allocation of direct and indirect blocks as needed. The
    /// transaction log is updated to support crash consistency.
    pub fn grow(
        &mut self,
        ffs: &FastFileSystemInner,
        until: FileBlockNumber,
        tx: &RunningTransaction,
    ) -> Result<(), KernelError> {
        // Hint: use [`FastFileSystemInner::allocate_block`] to allocate an free block.
        todo!()
    }

    /// Deallocate inner blocks and set the inode's size to zero.
    ///
    /// Note that submitting the InodeWriteGuard is the caller's responsibility.
    pub fn zeroify(
        ino: &mut TrackedInodeWriteGuard,
        tx: &RunningTransaction,
        ffs: &FastFileSystemInner,
    ) {
        let mut sb = ffs.sb.write(tx);
        for fba in 0..(ino.size.div_ceil(0x1000)) {
            let lba = ino.get(ffs, FileBlockNumber(fba)).unwrap().unwrap();

            let (b_lba, offset) = lba.into_bitmap_lba_offset(ffs).unwrap();
            let bitmap = disk_layout::BlockBitmap::load(ffs, b_lba).unwrap();

            let mut guard = bitmap.write(tx);
            assert!(guard.deallocate(offset));
            guard.submit();

            sb.block_count_inused -= 1;
        }

        sb.submit();
        ino.size = 0;
    }
}
