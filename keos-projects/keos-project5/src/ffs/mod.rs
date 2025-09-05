//! # Fast File System (FFS).
//!
//! In KeOS, the kernel utilizes multiple layers to implement file operations.
//! Higher layers provide convenient abstractions such as buffered I/O, while
//! the lowest layer is responsible for managing
//! the on-disk layout through the File System.
//!
//! ```text
//! ┌─────────────────────────────┐
//! │ keos::fs::RegularFile       │
//! │ - Top-level API for File    │
//! └─────────────┬───────────────┘
//!               │
//! ┌─────────────▼───────────────────┐
//! │ keos::fs::traits::RegularFile   │
//! │ - Block-based buffer            │
//! │ - Block-level R/W interface     │
//! └─────────────┬───────────────────┘
//!               │
//! ┌─────────────▼─────────────────────────┐
//! │ pagecache::overlaying::RegularFile    │
//! │ - Cache-managed file access           │
//! └─────────────┬─────────────────────────┘
//!               │
//! ┌─────────────▼─────────────────────────┐
//! │ Filesystem-specific RegularFile       │
//! │ - Primitive block ops on filesystem   │
//! └─────────────┬─────────────────────────┘
//!               │
//! ┌─────────────▼─────────────────────────┐
//! │ Inode                                 │
//! │ - Conduct the disk-level operation    │
//! └───────────────────────────────────────┘
//! ```
//!
//! This project focuses on implementing the File System, the lowest level in
//! the hierarchy. Here, you will build the abstractions that directly manage
//! on-disk inodes and blocks, forming the foundation of all higher-level file
//! operations.
//!
//! A **file system** is a fundamental component of any operating system,
//! responsible for managing how data is stored, organized, and accessed on
//! persistent storage devices such as hard drives or solid-state drives. It
//! provides a hierarchical abstraction of files and directories, enabling users
//! and programs to interact with stored data through well-defined interfaces
//! while abstracting away low-level hardware details. The file system ensures
//! data integrity, persistence, and consistency, even in the face of system
//! crashes or concurrent access.
//!
//! While simple file systems may achieve functional correctness, they often
//! suffer from performance limitations, such as poor spatial locality,
//! excessive fragmentation, and inefficient handling of metadata. These issues
//! become increasingly problematic as file systems scale in size and
//! complexity.
//!
//! The **Fast File System (FFS)** was developed to address these
//! shortcomings. It incorporates layout and allocation strategies aimed at
//! improving throughput, minimizing seek time, and optimizing disk utilization.
//! Key features include.
//!
//! In this project, you will implement the simplified version of Fast File
//! System: **No error handling** and **I/O optimizations**.
//!
//! ## Overview of the KeOS Fast File System
//! The following diagram depicts the disk layout of the FFS:
//! ```text
//!            +────────────────────+
//!            │ Superblock         │
//!            +────────────────────+
//!            │ Inode Bitmap       │
//!            +────────────────────+
//!            │ Block Bitmap       │
//! Journal ──-+────────────────────+
//!   │        │ JournalSB          │
//!   │        +────────────────────+
//!   │        │ TxBegin            │
//!   │        │ Journal Block[0]   │
//!   │        │ Journal Block[1]   │
//!   │        │   ...              │
//!   │        │ TxEnd              │
//!   └─────── +────────────────────+
//!            │ Inodes             │
//!            +────────────────────+
//!            │ Data Blocks        │
//!            │ ...                │
//!            +────────────────────+
//! ```
//! The following list describes each parts of the FFS disk layout:
//! - **Superblock** – Global filesystem metadata: block size, counts, offsets,
//!   and identifiers.
//! - **Inode Bitmap** – Tracks which inodes are allocated or free.
//! - **Block Bitmap** – Tracks which data blocks are allocated or free.
//! - **Journal** – Transactional log ensuring crash consistency.
//! - **Inodes** – The inode table storing metadata and block pointers for files
//!   and directories.
//! - **Data Blocks** – The actual contents of files and directories.
//!
//! Now, start with the implementation of [`inode`].
use crate::lru::LRUCache;
use access_control::{BlockPointsTo, MetaData, TrackedInode};
use alloc::{
    boxed::Box,
    collections::btree_map::{BTreeMap, Entry},
    sync::Arc,
};
use core::ops::Range;
use disk_layout::{InodeArray, InodeBitmap, JournalSb};
use fs_objects::Directory;
use inode::Inode;
use journal::{Journal, RunningTransaction};
use keos::{
    KernelError,
    fs::{Disk, FileBlockNumber, InodeNumber},
    sync::{RwLock, SpinLock},
};
use types::LogicalBlockAddress;

pub mod access_control;
pub mod disk_layout;
pub mod fs_objects;
pub mod inode;
pub mod journal;
pub mod types;

/// A handle for performing journal I/O operations.
///
/// The journal is used to provide crash consistency by recording intended
/// changes before they are committed to the main file system. This structure
/// provides an interface for reading from and writing to the journal region
/// of the disk.
pub struct JournalIO<'a> {
    /// A reference to the file system.
    pub ffs: &'a FastFileSystemInner,
}

impl JournalIO<'_> {
    /// Writes a metadata block into the journal.
    ///
    /// This function records a 4 KiB block of metadata at the specified
    /// logical block address (LBA) in the journal. The block is stored
    /// as part of the write-ahead log, ensuring that metadata updates
    /// can be safely replayed in case of a crash.
    pub fn write_metadata_block(
        &self,
        lba: LogicalBlockAddress,
        block: &[u8; 4096],
    ) -> Result<(), KernelError> {
        for i in 0..8 {
            self.ffs.disk.write(
                lba.into_sector() + i,
                block[512 * i..512 * (i + 1)].as_array().unwrap(),
            )?;
        }
        Ok(())
    }

    /// Reads a journal block from the disk.
    ///
    /// This function retrieves a 4 KiB block from the journal at the given
    /// logical block address (LBA) and copies it into the provided buffer.
    /// It is primarily used during recovery to replay logged operations
    /// and restore the file system to a consistent state.
    pub fn read_journal(
        &self,
        lba: LogicalBlockAddress,
        b: &mut [u8; 4096],
    ) -> Result<(), KernelError> {
        for i in 0..8 {
            self.ffs.disk.read(
                lba.into_sector() + i,
                b[512 * i..512 * (i + 1)].as_mut_array().unwrap(),
            )?;
        }

        Ok(())
    }
}

/// Represents the internal structure of a Fast File System (FFS).
///
/// This structure encapsulates the core components of the FFS implementation,
/// including metadata loaded from disk, in-memory caches, and journal state.
/// It provides the foundational layer for all low-level filesystem operations,
/// such as reading/writing blocks and accessing inode metadata.
///
/// This type is used internally by [`FastFileSystem`] and is not intended
/// to be accessed directly by external code.
pub struct FastFileSystemInner {
    /// The underlying disk device used by the filesystem.
    pub(crate) disk: Disk,

    /// Total number of blocks available in the filesystem.
    pub block_count: usize,

    /// Total number of inodes supported by the filesystem.
    pub inode_count: usize,

    /// Indicate this file system support journaling.
    pub has_journal: usize,

    /// The expected view of the disk after all journaled changes are
    /// checkpointed.
    ///
    /// This in-memory map reflects the filesystem state after applying
    /// journaled updates, but may differ from the actual disk contents if a
    /// crash occurred before checkpointing.
    pub blocks: SpinLock<LRUCache<LogicalBlockAddress, Arc<SpinLock<[u8; 4096]>>, 512>>,

    /// On-disk superblock structure, wrapped in metadata-aware
    /// block access.
    pub sb: BlockPointsTo<disk_layout::SuperBlock>,

    /// In-memory table mapping inode numbers to their live representations.
    pub inodes: SpinLock<BTreeMap<InodeNumber, Arc<RwLock<Inode>>>>,

    /// The current state of the journal (if present), wrapped in a
    /// lock to allow mutable access during journal operations.
    pub journal: Option<SpinLock<Journal>>,

    /// Whether trace the transactions for debugging purpose.
    pub debug_journal: bool,
}

impl FastFileSystemInner {
    /// Constructs a new file system instance from an on-disk superblock.
    ///
    /// This function initializes a [`FastFileSystemInner`] by reading the
    /// provided superblock and setting up access to the underlying disk.
    pub fn from_raw_sb(
        sb: BlockPointsTo<disk_layout::SuperBlock>,
        disk: Disk,
        debug_journal: bool,
        disable_journal: bool,
    ) -> Result<Self, KernelError> {
        let guard = sb.read();
        if &guard.magic == b"KeOSFFS\0" {
            let block_count = guard.block_count as usize;
            let inode_count = guard.inode_count as usize;
            let has_journal = guard.has_journal as usize;
            drop(guard);

            let mut this = FastFileSystemInner {
                disk,
                block_count,
                inode_count,
                has_journal,
                blocks: SpinLock::new(LRUCache::new()),
                sb,
                inodes: SpinLock::new(BTreeMap::new()),
                journal: None,
                debug_journal,
            };

            if this.has_journal > 0 && !disable_journal {
                this.journal = Some(SpinLock::new(Journal {
                    sb: JournalSb::from_disk(&this.disk, this.journal().start)?,
                }));
                let mut guard = this.journal.as_ref().unwrap().lock();
                let result = guard.recovery(&this, &JournalIO { ffs: &this });
                guard.unlock();
                result?;
                this.sb.reload(&this.disk)?;
            }

            println!("[FFS] Mounted with superblock: ");
            println!(
                "  - Inodes: {:?} / {:?}",
                this.sb.read().inode_count_inused,
                this.inode_count
            );
            println!(
                "  - Blocks: {:?} / {:?}",
                this.sb.read().block_count_inused,
                this.block_count
            );
            println!("  - InodeBitmap: {:?}", this.inode_bitmap());
            println!("  - BlockBitmap: {:?}", this.block_bitmap());
            println!("  - Journal: {:?}", this.journal());
            println!("  - InodeArray: {:?}", this.inode());
            println!("  - DataBlock: {:?} ~", this.data_block_start());
            Ok(this)
        } else {
            Err(KernelError::FilesystemCorrupted("Invalid Superblock Magic"))
        }
    }

    /// Returns the range of block address of the inode bitmap.
    ///
    /// The inode bitmap is located immediately after the inode table.
    #[inline]
    pub fn inode_bitmap(&self) -> Range<LogicalBlockAddress> {
        let begin = LogicalBlockAddress::new(2).unwrap();
        begin..begin + self.inode_count.div_ceil(8).div_ceil(0x1000)
    }

    /// Returns the range of block address of the block bitmap.
    ///
    /// The block bitmap follows the inode bitmap.
    #[inline]
    pub fn block_bitmap(&self) -> Range<LogicalBlockAddress> {
        let begin = self.inode_bitmap().end;
        begin..begin + self.block_count.div_ceil(8).div_ceil(0x1000)
    }

    /// Returns the range of block address of the journal.
    ///
    /// The data block region follows the bitmaps.
    #[inline]
    pub fn journal(&self) -> Range<LogicalBlockAddress> {
        let begin = self.block_bitmap().end;
        if self.has_journal != 0 {
            begin
                ..begin + 1 /* Journal SB */ + 1 /* TxBegin */ + 4095 /* Maximum number of data blocks */ + 1 /* TxEnd */
        } else {
            begin..begin
        }
    }

    /// Returns the range of the block address of the Inode[] startpoint.
    ///
    /// The data block region follows the bitmaps.
    #[inline]
    pub fn inode(&self) -> Range<LogicalBlockAddress> {
        let begin = self.journal().end;
        begin
            ..begin
                + (core::mem::size_of::<disk_layout::Inode>() * self.inode_count).div_ceil(0x1000)
    }

    /// Returns the starting block address of the data blocks.
    ///
    /// The data block region follows the bitmaps and journal.
    #[inline]
    pub fn data_block_start(&self) -> LogicalBlockAddress {
        self.inode().end
    }

    /// Opens a new running transaction.
    ///
    /// This function begins a transaction on the file system, allowing
    /// multiple operations to be grouped together atomically. Transactions
    /// ensure crash consistency by recording updates in the journal before
    /// they are committed to the main file system.
    pub fn open_transaction(&self, name: &str) -> RunningTransaction<'_> {
        RunningTransaction::begin(name, self, JournalIO { ffs: self }, self.debug_journal)
    }

    /// Reads a data block from disk.
    ///
    /// This function retrieves the 4 KiB block located at the specified
    /// logical block address (LBA) from the underlying disk. It is used for
    /// reading file data on disk.
    pub fn read_data_block(
        &self,
        lba: LogicalBlockAddress,
    ) -> Result<Box<[u8; 4096]>, KernelError> {
        assert!(
            self.data_block_start() <= lba,
            "[FFS-ERROR] You must cannot directly read the metadata. Use `MetaData::load` or `JournalIO`."
        );
        let mut b = Box::new([0u8; 0x1000]);
        for i in 0..8 {
            self.disk.read(
                lba.into_sector() + i,
                b[512 * i..512 * (i + 1)].as_mut_array().unwrap(),
            )?;
        }

        Ok(b)
    }

    /// Writes a 4 KiB data block to disk.
    ///
    /// This function stores the given buffer at the specified logical block
    /// address (LBA) on the underlying disk. It is typically used for writing
    /// file contents.
    pub fn write_data_block(
        &self,
        lba: LogicalBlockAddress,
        b: &[u8; 4096],
    ) -> Result<(), KernelError> {
        assert!(
            self.data_block_start() <= lba,
            "[FFS-ERROR] You must cannot directly write to the metadata ({lba:?}). Use `MetaData::load` or `JournalIO`.",
        );
        for i in 0..8 {
            self.disk.write(
                lba.into_sector() + i,
                b[512 * i..512 * (i + 1)].as_array().unwrap(),
            )?;
        }

        Ok(())
    }

    /// Converts this inode number into the corresponding location in the inode
    /// bitmap.
    ///
    /// # Arguments
    /// - `fs`: Reference to the file system's internal metadata layout.
    ///
    /// # Returns
    /// - `Some((lba, offset)`: if the inode number is valid, which includes:
    ///   - `lba`: the logical block address of the inode bitmap block that
    ///     contains this inode.
    ///   - `offset`: the bit offset within that bitmap block corresponding to
    ///     this inode.
    /// - `None`: if the inode number is out of bounds.
    pub fn get_inode_bitmap_lba_index(
        &self,
        ino: InodeNumber,
    ) -> Option<(LogicalBlockAddress, usize)> {
        if (ino.into_u32() as usize) < self.inode_count {
            let index = (ino.into_u32() - 1) as usize;

            Some((self.inode_bitmap().start + index / 0x1000, index % 0x8000))
        } else {
            None
        }
    }

    /// Converts this inode number into the corresponding location in the inode
    /// array.
    ///
    /// # Returns
    /// - `Some((lba, offset))`: if the inode number is valid, which includes:
    ///   - `lba`: the logical block address where the containing [`InodeArray`]
    ///     is located.
    ///   - `offset`:the index within that array where this specific inode
    ///     resides.
    /// - `None`: if the inode number is out of bounds.
    ///
    /// [`InodeArray`]: crate::ffs::disk_layout::InodeArray
    pub fn get_inode_array_lba_index(
        &self,
        ino: InodeNumber,
    ) -> Option<(LogicalBlockAddress, usize)> {
        if (ino.into_u32() as usize) < self.inode_count {
            let index = (ino.into_u32() - 1) as usize;
            let inode_per_block = 0x1000 / core::mem::size_of::<disk_layout::Inode>();

            Some((
                self.inode().start + index / inode_per_block,
                index % inode_per_block,
            ))
        } else {
            None
        }
    }

    /// Reads a metadata block from disk.
    ///
    /// This function retrieves a block at the given logical block address
    /// and wraps it in a [`SpinLock`] for safe concurrent access. Metadata
    /// blocks include structures such as inodes, directories, and allocation
    /// maps that are frequently shared between threads.
    ///
    /// THIS IS INTERNAL API. DO NOT USE THIS FUNCTION.
    #[doc(hidden)]
    pub fn read_meta(
        &self,
        lba: LogicalBlockAddress,
    ) -> Result<Arc<SpinLock<[u8; 4096]>>, KernelError> {
        let mut guard = self.blocks.lock();
        let result = guard
            .get_or_insert_with(lba, || {
                let b = Arc::new(SpinLock::new([0; 4096]));
                {
                    let mut guard = b.lock();
                    for i in 0..8 {
                        self.disk.read(
                            lba.into_sector() + i,
                            guard[512 * i..512 * (i + 1)].as_mut_array().unwrap(),
                        )?;
                    }
                    guard.unlock();
                }
                Ok(b)
            })
            .map(|b| b.clone());
        guard.unlock();
        result
    }

    /// Allocates a new inode in the file system.
    ///
    /// This function creates a new inode on disk and returns both its
    /// inode number and a [`TrackedInode`] handle for in-memory access.
    /// The allocation is recorded in the given transaction for crash
    /// consistency.
    pub fn allocate_inode(
        self: &Arc<Self>,
        is_dir: bool,
        tx: &RunningTransaction,
    ) -> Result<(InodeNumber, TrackedInode), KernelError> {
        for (i, lba) in self.inode_bitmap().enumerate() {
            let bitmap = InodeBitmap::load(self, lba)?;
            let mut guard = bitmap.write(tx);
            for pos in 0..4096 * 8 {
                if guard.try_allocate(pos) {
                    guard.submit();
                    let ino = InodeNumber::new((pos + i * 4096 * 8 + 1) as u32).unwrap();
                    let mut guard = self.inodes.lock();
                    let result = match guard.entry(ino) {
                        Entry::Occupied(_) => Err(KernelError::FilesystemCorrupted(
                            "Allocate to existing inode.",
                        )),
                        Entry::Vacant(en) => {
                            // Lookup inode bitmap.
                            let (lba, index) = self.get_inode_array_lba_index(ino).unwrap();
                            let inode_arr = InodeArray::load(self, lba)?;
                            let inode = Inode::new(ino, is_dir);
                            let mut guard = inode_arr.write(tx);
                            guard[index] = inode.into_disk_format();
                            guard.submit();
                            let mut sb = self.sb.write(tx);
                            sb.inode_count_inused += 1;
                            sb.submit();
                            let inode = Arc::new(RwLock::new(inode));
                            en.insert(inode.clone());
                            Ok((ino, TrackedInode::new(inode, Arc::downgrade(self))))
                        }
                    };
                    guard.unlock();
                    return result;
                }
            }
            guard.forget();
        }
        Err(KernelError::NoSpace)
    }

    /// Allocates a new data block on disk.
    ///
    /// This function reserves a free block for use in the file system,
    /// recording the allocation in the active transaction. The block is
    /// marked as used in the allocation bitmap and returned to the caller.
    pub fn allocate_block(
        &self,
        tx: &RunningTransaction,
    ) -> Result<LogicalBlockAddress, KernelError> {
        for (i, lba) in self.block_bitmap().enumerate() {
            let bitmap = disk_layout::BlockBitmap::load(self, lba)?;
            let mut bitmap = bitmap.write(tx);
            for pos in 0..4096 * 8 {
                if bitmap.try_allocate(pos) {
                    bitmap.submit();
                    let mut sb = self.sb.write(tx);
                    sb.block_count_inused += 1;
                    sb.submit();
                    return Ok(LogicalBlockAddress::new((pos + i * 4096 * 8) as u64).unwrap());
                }
            }
            bitmap.forget();
        }
        Err(KernelError::NoSpace)
    }

    /// Retrieves an inode from disk or cache.
    ///
    /// This function returns a [`TrackedInode`] corresponding to the given
    /// inode number. If the inode is cached in memory, it is returned
    /// directly; otherwise, it is read from disk and added to the cache.
    ///
    /// This method manages a "unique view" of a single inode.
    pub fn get_inode(self: &Arc<Self>, ino: InodeNumber) -> Result<TrackedInode, KernelError> {
        let mut guard = self.inodes.lock();
        let result = match guard.entry(ino) {
            Entry::Occupied(en) => Ok(TrackedInode::new(en.get().clone(), Arc::downgrade(self))),
            Entry::Vacant(en) => {
                // Lookup inode bitmap.
                let (lba, offset) = self.get_inode_bitmap_lba_index(ino).unwrap();
                let bitmap_block = InodeBitmap::load(self, lba)?;
                if !bitmap_block.read().is_allocated(offset) {
                    return Err(KernelError::NoSuchEntry);
                }

                let (lba, index) = self.get_inode_array_lba_index(ino).unwrap();
                let inodes = InodeArray::load(self, lba)?;
                let inode = Inode::from_disk_layout(&inodes.read()[index])?;
                if inode.ino != ino {
                    return Err(KernelError::FilesystemCorrupted("Inode number mismatch"));
                }

                let inode = Arc::new(RwLock::new(inode));
                en.insert(inode.clone());
                Ok(TrackedInode::new(inode, Arc::downgrade(self)))
            }
        };
        guard.unlock();
        result
    }

    /// Removes an inode from the in-memory inode table.
    ///
    /// This function evicts the given inode from the inode cache maintained
    /// by the file system. It does not remove the inode’s contents on disk,
    /// only its in-memory representation.
    pub fn remove_inode(&self, ino: InodeNumber) -> Option<Arc<RwLock<Inode>>> {
        let mut guard = self.inodes.lock();
        if let Entry::Occupied(en) = guard.entry(ino)
            && Arc::strong_count(en.get()) == 2
        {
            let result = en.remove();
            guard.unlock();
            // This means the caller only has the sole reference to the inode.
            return Some(result);
        }
        guard.unlock();
        None
    }
}

/// A reference-counted wrapper around [`FastFileSystemInner`].
///
/// This structure provides access to a Fast File System instance
/// while ensuring safe concurrent access through an [`Arc`].
#[derive(Clone)]
pub struct FastFileSystem(pub Arc<FastFileSystemInner>);

impl FastFileSystem {
    /// The inode number of the root directory (`/`).
    pub const ROOT_INODE_NUMBER: InodeNumber = InodeNumber::new(1).unwrap();

    /// Loads a Fast File System from a given disk.
    ///
    /// This function attempts to read the superblock from the disk.
    /// If the disk contains a valid FFS superblock, it returns an
    /// initialized [`FastFileSystem`] instance; otherwise, it returns `None`.
    ///
    /// # Parameters
    /// - `disk`: The disk device containing the filesystem.
    ///
    /// # Returns
    /// - `Some(Self)`: If the filesystem is successfully loaded.
    /// - `None`: If the disk does not contain a valid Fast File System.
    pub fn from_disk(
        disk: Disk,
        debug_journal: bool,
        disable_journal: bool,
    ) -> Result<Self, KernelError> {
        let sb = disk_layout::SuperBlock::from_disk(&disk)?;
        Ok(Self(Arc::new(FastFileSystemInner::from_raw_sb(
            sb,
            disk,
            debug_journal,
            disable_journal,
        )?)))
    }

    /// Retrieves an in-memory representation of the inode identified by `ino`.
    ///
    /// This function looks up the inode in the Fast File System and returns a
    /// [`TrackedInode`].
    pub fn get_inode(&self, ino: InodeNumber) -> Result<TrackedInode, KernelError> {
        self.0.get_inode(ino)
    }
}

impl keos::fs::traits::FileSystem for FastFileSystem {
    fn root(&self) -> Option<keos::fs::Directory> {
        Some(keos::fs::Directory(Arc::new(Directory::new(
            self.get_inode(Self::ROOT_INODE_NUMBER).unwrap(),
            Arc::downgrade(&self.0),
        )?)))
    }
}
