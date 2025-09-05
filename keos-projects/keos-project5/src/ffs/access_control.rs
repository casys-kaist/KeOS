//! Metadata access and synchronization primitives for the filesystem.
//!
//! This module defines core abstractions for accessing and modifying
//! filesystem metadata such as inodes and other block-based structures.
//! These types ensure safe, concurrent access to on-disk metadata and provide
//! mechanisms for **enforcing** transactional updates with journaling support.
//!
//! It forms the backbone of safe, transactional filesystem operations.
//! See [`disk_layout`] module for its usage.
//!
//! [`disk_layout`]: super::disk_layout
use crate::ffs::{
    FastFileSystemInner, LogicalBlockAddress, RunningTransaction,
    disk_layout::{InodeArray, Private, SuperBlock},
    inode::Inode,
};
use alloc::{
    boxed::Box,
    sync::{Arc, Weak},
};
use keos::{
    KernelError,
    fs::{Disk, Sector},
    sync::{RwLock, RwLockReadGuard, RwLockWriteGuard, SpinLock, SpinLockGuard},
};

/// Trait for file system metadata types that can be loaded from disk.
///
/// Types implementing [`MetaData`] represent metadata structures in the file
/// system (e.g., superblock, inode bitmap, block bitmap, inode array) that are
/// stored on disk in a fixed layout. This trait provides a standard interface
/// for reading these structures from their on-disk representation.
///
/// The trait requires implementors to be `Default` and support loading from a
/// logical block address (LBA) using the file system’s internal
/// I/O abstraction.
pub trait MetaData: Sized + Default {
    #[doc(hidden)]
    const P: Private;

    /// Loads a metadata structure from disk at the specified logical block
    /// address.
    ///
    /// # Parameters
    /// - `ffs`: Reference to the internal file system state.
    /// - `lba`: Logical block address where the metadata is stored.
    ///
    /// # Returns
    /// - `Ok(metadata)`: An `Arc<SpinLock<T>>` wrapping the loaded metadata.
    /// - `Err(KernelError)`: If the metadata could not be read.
    fn load(
        ffs: &FastFileSystemInner,
        lba: LogicalBlockAddress,
    ) -> Result<BlockPointsTo<Self>, KernelError> {
        Ok(BlockPointsTo {
            lba,
            b: ffs.read_meta(lba)?,
            _m: core::marker::PhantomData,
        })
    }
}

impl SuperBlock {
    /// Loads the superblock structure from disk.
    ///
    /// This function reads the first 8 sectors (4096 bytes) from the disk.
    /// It is the first step when mounting a file system, as the superblock
    /// contains metadata such as layout information, and journals.
    ///
    /// ### Parameters
    /// - `disk`: A reference to the block device implementing the [`Disk`]
    ///   trait.
    ///
    /// ### Returns
    /// - `Ok(Box<SuperBlock>)`: If the superblock is successfully read.
    /// - `Err(KernelError)`: If any sector read fails.
    pub fn from_disk(disk: &Disk) -> Result<BlockPointsTo<Self>, KernelError> {
        let b = Arc::new(SpinLock::new([0; 4096]));
        {
            let mut guard = b.lock();
            for i in 0..8 {
                disk.read(
                    Sector(i),
                    guard[512 * i..512 * (i + 1)].as_mut_array().unwrap(),
                )?;
            }
            guard.unlock();
        }
        Ok(BlockPointsTo {
            lba: LogicalBlockAddress::new(1).unwrap(),
            b,
            _m: core::marker::PhantomData,
        })
    }
}

/// A wrapper around a metadata block that resides at a specific logical block
/// address (LBA).
///
/// `BlockPointsTo` provides safe, synchronized access to a disk-backed
/// 4096-byte block, and associates the block with a specific metadata type `M`
/// implementing the [`MetaData`] trait. Internally, it uses a [`SpinLock`] to
/// protect concurrent access and associate with its metadata type without
/// affecting layout.
///
/// This abstraction allows safe and typed access to the underlying bytes as
/// metadata structures, while supporting transactional read/write operations.
///
/// # Type Parameters
/// - `M`: The type of metadata this block contains. Must implement
///   [`MetaData`].
#[derive(Clone)]
pub struct BlockPointsTo<M: MetaData> {
    /// Logical block address (LBA) where this block resides on disk.
    lba: LogicalBlockAddress,

    /// The in-memory contents of the block, protected by a spinlock for
    /// concurrency.
    b: Arc<SpinLock<[u8; 4096]>>,

    /// Marker to associate this block with metadata type `M`.
    _m: core::marker::PhantomData<M>,
}

impl<M: MetaData> BlockPointsTo<M> {
    /// Acquires a read-only guard to the underlying block contents.
    ///
    /// # Returns
    /// - [`BlockPointsToReadGuard`]: A read guard providing immutable access to
    ///   the block's contents, typed as metadata `M`.
    ///
    /// This method locks the internal spinlock and returns a guard for safe,
    /// read-only access to the raw bytes of the metadata block.
    pub fn read(&self) -> BlockPointsToReadGuard<'_, M> {
        BlockPointsToReadGuard {
            b: Some(self.b.lock()),
            _m: core::marker::PhantomData,
        }
    }

    /// Acquires a write guard to the block, registering it with the given
    /// transaction.
    ///
    /// # Arguments
    /// - `tx`: The currently running transaction used to log changes for
    ///   durability and crash recovery.
    ///
    /// # Returns
    /// - [`BlockPointsToWriteGuard`]: A write guard that allows mutation of the
    ///   block’s contents and records the modification in the transaction.
    ///
    /// This method is intended for use in metadata updates. The block is locked
    /// for exclusive access, and the transaction ensures write-ahead
    /// logging or journaling semantics for filesystem consistency.
    pub fn write<'a, 'b, 'c>(
        &'a self,
        tx: &'b RunningTransaction<'c>,
    ) -> BlockPointsToWriteGuard<'a, 'b, 'c, M> {
        BlockPointsToWriteGuard {
            lba: self.lba,
            b: Some(self.b.lock()),
            tx,
            _m: core::marker::PhantomData,
        }
    }

    /// Reload in-memory structure to synchronize with on-disk structure
    pub fn reload(&self, disk: &Disk) -> Result<(), KernelError> {
        let mut guard = self.b.lock();
        for i in 0..8 {
            disk.read(
                self.lba.into_sector() + i,
                guard[512 * i..512 * (i + 1)].as_mut_array().unwrap(),
            )?;
        }
        guard.unlock();
        Ok(())
    }
}

/// A read-only guard that provides typed access to a metadata block.
///
/// `BlockPointsToReadGuard` is returned by [`BlockPointsTo::read`] and allows
/// immutable access to the contents of a 4096-byte disk block as a value of
/// type `M`, which implements the [`MetaData`] trait.
///
/// Internally, it wraps a locked reference to the block’s byte array using a
/// [`SpinLockGuard`] and casts it to the metadata type using `unsafe` pointer
/// casting. The guard ensures that the block cannot be modified while borrowed
/// immutably.
///
/// # Use Case
/// Use this when you want to inspect metadata structures (like an inode table,
/// superblock, or bitmap) without modifying them.
pub struct BlockPointsToReadGuard<'a, M: MetaData> {
    /// Spinlock guard for the raw block data.
    b: Option<SpinLockGuard<'a, [u8; 4096]>>,

    /// Marker to associate the block with its metadata type.
    _m: core::marker::PhantomData<M>,
}

impl<M: MetaData> core::ops::Deref for BlockPointsToReadGuard<'_, M> {
    type Target = M;

    fn deref(&self) -> &Self::Target {
        unsafe { &*(self.b.as_ref().unwrap().as_ptr() as *const M) }
    }
}

impl<M: MetaData> Drop for BlockPointsToReadGuard<'_, M> {
    /// Ensures the spinlock is released when the guard goes out of scope.
    fn drop(&mut self) {
        self.b.take().unwrap().unlock();
    }
}

/// A mutable guard for modifying metadata loaded from a block on disk,
/// paired with a transaction context for journaling or rollback.
///
/// This guard is returned by [`BlockPointsTo::write`] and provides exclusive,
/// mutable access to in-memory metadata of type `M`, along with a reference to
/// the current transaction. Any modifications made through this guard must be
/// explicitly committed via [`submit`] to ensure they are persisted and
/// journaled properly.
///
/// # Use Case
/// Use this when modifying metadata that needs to be tracked in a
/// [`RunningTransaction`], such as updating inode entries, marking blocks
/// as allocated, or changing filesystem state.
///
/// # Safety and Enforcement
/// The implementation panics if this guard is dropped without calling
/// [`submit`], enforcing that all metadata updates must go through the
/// transaction system to maintain consistency.
///
/// [`submit`]: Self::submit
pub struct BlockPointsToWriteGuard<'a, 'b, 'c, M: MetaData> {
    /// Logical block address of the block being modified.
    lba: LogicalBlockAddress,

    /// Spinlock guard protecting the block contents.
    b: Option<SpinLockGuard<'a, [u8; 4096]>>,

    /// Mutable reference to the ongoing transaction.
    tx: &'b RunningTransaction<'c>,

    /// Marker to associate the block with its metadata type.
    _m: core::marker::PhantomData<M>,
}

impl<M: MetaData> BlockPointsToWriteGuard<'_, '_, '_, M> {
    /// Submits the modified metadata block to the [`RunningTransaction`].
    ///
    /// This function marks the block as dirty and ensures it will be written
    /// to disk as part of the journal. After calling `submit`, the guard is
    /// consumed.
    pub fn submit(mut self) {
        self.tx.write_meta(
            self.lba,
            Box::new(**self.b.as_ref().unwrap()),
            core::any::type_name::<M>(),
        );
        let _ = unsafe { core::ptr::read(&self.b.as_ref().unwrap()) };
        self.b.take().unwrap().unlock();
        let _ = core::mem::ManuallyDrop::new(self);
    }

    /// Explictly drops the modified metadata block to the
    /// [`RunningTransaction`].
    ///
    /// This function marks the block as intact and ensures it will never be
    /// written to disk as part of the journal. After calling `forget`, the
    /// guard is consumed.
    pub fn forget(self) {
        let _ = unsafe { core::ptr::read(&self.b) };
        let _ = core::mem::ManuallyDrop::new(self);
    }
}

impl<M: MetaData> core::ops::Deref for BlockPointsToWriteGuard<'_, '_, '_, M> {
    type Target = M;
    fn deref(&self) -> &Self::Target {
        unsafe { &*(self.b.as_ref().unwrap().as_ptr() as *const M) }
    }
}

impl<M: MetaData> core::ops::DerefMut for BlockPointsToWriteGuard<'_, '_, '_, M> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *(self.b.as_mut().unwrap().as_mut_ptr() as *mut M) }
    }
}
impl<M: MetaData> Drop for BlockPointsToWriteGuard<'_, '_, '_, M> {
    /// Panics if the guard is dropped without calling `submit`.
    ///
    /// This ensures that all metadata changes are either explicitly recorded
    /// in a transaction or clearly rejected, helping prevent silent data loss.
    fn drop(&mut self) {
        panic!(
            "You are not calling `submit()` on `BlockPointsToWriteGuard`.

** All metadata modifications must be explicitly submitted to the transaction.
** To apply changes safely, call `submit()` before the guard is dropped."
        );
    }
}

/// A reference-counted, thread-safe wrapper around an in-memory [`Inode`],
/// enabling synchronized read and transactional write access.
///
/// This type provides read access through a shared guard and write access via
/// a transactional context, ensuring consistency between the in-memory and
/// on-disk representations of the inode.
#[derive(Clone)]
pub struct TrackedInode(Arc<RwLock<Inode>>, Weak<FastFileSystemInner>);

impl Drop for TrackedInode {
    fn drop(&mut self) {
        if let Some(ffs) = self.1.upgrade() {
            let ino = self.read().ino;
            if let Some(inode) = ffs.remove_inode(ino) {
                // There is no way to access this file.
                let mem_layout = inode.write();
                if mem_layout.link_count == 0 {
                    let tx = ffs.open_transaction("File::remove");
                    if let Some((lba, index)) = tx.ffs.get_inode_array_lba_index(mem_layout.ino)
                        && let Ok(inode_array) = InodeArray::load(tx.ffs, lba)
                    {
                        let disk_layout = inode_array.write(&tx);
                        let mut guard = TrackedInodeWriteGuard {
                            mem_layout,
                            disk_layout,
                            index,
                        };

                        Inode::zeroify(&mut guard, &tx, &ffs);

                        let mut sb = ffs.sb.write(&tx);
                        sb.inode_count_inused -= 1;
                        sb.submit();

                        let ino = guard.ino;

                        guard.disk_layout[index].ino = None;
                        guard.do_submit();

                        let bitmap_no = (ino.into_u32() - 1) as usize;
                        let bitmap_lba = ffs.inode_bitmap().start + (bitmap_no / 0x8000);
                        let bitmap =
                            crate::ffs::disk_layout::InodeBitmap::load(&ffs, bitmap_lba).unwrap();
                        let mut guard = bitmap.write(&tx);

                        assert!(guard.deallocate(bitmap_no % 0x8000));
                        guard.submit();

                        let _ = tx.commit();
                    }
                }
            }
        }
    }
}

impl TrackedInode {
    /// Create a new [`TrackedInode`] reference.
    pub fn new(inner: Arc<RwLock<Inode>>, ffs: Weak<FastFileSystemInner>) -> Self {
        Self(inner, ffs)
    }

    /// Acquires a shared read lock on the in-memory inode.
    ///
    /// # Returns
    /// A [`TrackedInodeReadGuard`] which provides read-only access to the
    /// current state of the inode.
    ///
    /// # Use Case
    /// Use this when you need to inspect an inode without modifying it.
    #[inline]
    pub fn read(&self) -> TrackedInodeReadGuard<'_> {
        TrackedInodeReadGuard(self.0.read())
    }

    /// Acquires an exclusive write lock on both the in-memory inode and the
    /// corresponding on-disk inode for transactional modification.
    ///
    /// You **must** submit the changes by calling the [`submit`] method.
    ///
    /// # Arguments
    /// - `tx`: A reference to the current [`RunningTransaction`] used to track
    ///   and commit filesystem changes.
    /// - `f`: A closure that performs modifications using the provided
    ///   [`TrackedInodeWriteGuard`], which contains both in-memory and on-disk
    ///   representations of the inode.
    ///
    /// # Returns
    /// - `Ok(R)`: If the closure `f` returns successfully.
    /// - `Err(KernelError)`: If an error occurs while resolving the inode
    ///   layout or during execution of `f`.
    ///
    /// # Use Case
    /// Use this when updating inode state (e.g., growing a file, updating
    /// metadata) and ensuring consistency between memory and disk through
    /// the transaction.
    ///
    /// # Example
    /// ```rust
    /// tracked_inode.write_with(tx, |mut guard| {
    ///     guard.mem_layout.size += 1;
    ///     guard.disk_layout[guard.index].size = guard.mem_layout.size;
    ///     guard.submit();
    ///     Ok(())
    /// })?;
    /// ```
    ///
    /// [`submit`]: TrackedInodeWriteGuard::submit
    #[inline]
    pub fn write_with<R>(
        &self,
        tx: &RunningTransaction,
        f: impl FnOnce(TrackedInodeWriteGuard) -> Result<R, KernelError>,
    ) -> Result<R, KernelError> {
        let mem_layout = self.0.write();

        let (lba, index) = tx
            .ffs
            .get_inode_array_lba_index(mem_layout.ino)
            .ok_or(KernelError::FilesystemCorrupted("Invalid Inode number."))?;
        let inode_array = InodeArray::load(tx.ffs, lba)?;
        let disk_layout = inode_array.write(tx);
        f(TrackedInodeWriteGuard {
            mem_layout,
            disk_layout,
            index,
        })
    }
}

/// A guard that provides read-only access to the in-memory [`Inode`] structure.
///
/// This guard is acquired via [`TrackedInode::read`] and ensures shared
/// access to the inode, preventing concurrent writes.
///
/// # Use Case
/// Use this guard when inspecting the state of an inode without making any
/// modifications.
///
/// # Example
/// ```rust
/// let guard = tracked_inode.read();
/// println!("inode size: {}", guard.size);
/// ```
pub struct TrackedInodeReadGuard<'a>(RwLockReadGuard<'a, Inode>);

impl core::ops::Deref for TrackedInodeReadGuard<'_> {
    type Target = Inode;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// A guard that provides mutable access to the in-memory within a transactional
/// context.
///
/// This guard is constructed internally by [`TrackedInode::write_with`]
/// and ensures that any modifications are performed consistently across both
/// memory and disk. The changes must be explicitly committed using the
/// associated transaction.
///
/// # Panics
/// If dropped without submitting the change to the transaction, this guard will
/// panic to ensure no silent inconsistency between in-memory and on-disk state.
///
/// # Example
/// ```rust
/// tracked_inode.write_with(tx, |mut guard| {
///     guard.mem_layout.size += 1;
///     guard.disk_layout[guard.index].size = guard.mem_layout.size;
///     guard.submit();
///     Ok(())
/// })?;
/// ```
pub struct TrackedInodeWriteGuard<'a, 'b, 'c, 'd> {
    /// Write guard protecting the in-memory inode.
    mem_layout: RwLockWriteGuard<'a, Inode>,

    /// Write guard protecting the on-disk inode array.
    disk_layout: BlockPointsToWriteGuard<'b, 'c, 'd, InodeArray>,

    /// Index to the inode.
    index: usize,
}

impl core::ops::Deref for TrackedInodeWriteGuard<'_, '_, '_, '_> {
    type Target = Inode;
    fn deref(&self) -> &Self::Target {
        &self.mem_layout
    }
}

impl core::ops::DerefMut for TrackedInodeWriteGuard<'_, '_, '_, '_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.mem_layout
    }
}

impl TrackedInodeWriteGuard<'_, '_, '_, '_> {
    /// Submits the modified metadata block to the [`RunningTransaction`].
    ///
    /// This function marks the block as dirty and ensures it will be written
    /// to disk as part of the journal. After calling `submit`, the guard is
    /// consumed.
    pub fn submit(mut self) {
        self.disk_layout[self.index] = self.mem_layout.into_disk_format();
        self.do_submit();
    }

    fn do_submit(self) {
        let _mem = unsafe { core::ptr::read(&self.mem_layout) };
        let disk = unsafe { core::ptr::read(&self.disk_layout) };

        disk.submit();
        core::mem::forget(self);
    }
}

impl Drop for TrackedInodeWriteGuard<'_, '_, '_, '_> {
    /// Panics if the guard is dropped without calling `submit`.
    ///
    /// This ensures that all metadata changes are either explicitly recorded
    /// in a transaction or clearly rejected, helping prevent silent data loss.
    fn drop(&mut self) {
        panic!(
            "You are not calling `submit()` on `TrackedInodeWriteGuard`.

** All inode modifications must be explicitly submitted to the transaction.
** To apply changes safely, call `submit()` before the guard is dropped."
        );
    }
}
