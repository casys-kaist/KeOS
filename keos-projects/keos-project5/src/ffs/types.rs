//! Core type definitions for the filesystem.
//!
//! This module defines fundamental data types used throughout the
//! filesystem implementation. These types encapsulate low-level
//! representations of on-disk metadata and provide utility methods
//! for interpreting and manipulating filesystem structures.
//!
//! Most types here are simple wrappers around integers or enums, often with
//! utility methods to convert between disk layout positions and internal
//! structures.
use super::FastFileSystemInner;
use core::{iter::Step, num::NonZeroU64};
use keos::{KernelError, fs::Sector};

/// Represents the type of a file in the filesystem.
///
/// This enum is used to distinguish between different kinds of inodes,
/// such as regular files and directories. It is stored on disk as part of
/// the inode metadata to identify how the data associated with the inode
/// should be interpreted.
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
#[repr(u32)]
pub enum FileType {
    /// A regular file, containing user data.
    ///
    /// This type represents standard files used to store arbitrary user
    /// content (e.g., text, binaries, etc.). The file’s data blocks are
    /// directly mapped to its contents.
    RegularFile = 0,

    /// A directory, which stores a list of file entries.
    ///
    /// A directory maps file names to inode numbers. Its contents are
    /// typically a structured list of directory entries that allow for
    /// hierarchical navigation within the filesystem.
    Directory = 1,
}

impl TryFrom<u32> for FileType {
    type Error = KernelError;
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::RegularFile),
            1 => Ok(Self::Directory),
            _ => Err(KernelError::FilesystemCorrupted("Invalid inode type")),
        }
    }
}

/// Represents a logical block address (LBA) on disk.
///
/// This structure stores the block index used to locate data blocks
/// on a physical storage device. The actual size of a block depends
/// on the filesystem's configuration.
#[repr(transparent)]
#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub struct LogicalBlockAddress(NonZeroU64);
const_assert!(core::mem::size_of::<Option<LogicalBlockAddress>>() == 8);

impl LogicalBlockAddress {
    /// Creates a [`LogicalBlockAddress`] if the given value is not zero.
    pub const fn new(n: u64) -> Option<Self> {
        if let Some(v) = NonZeroU64::new(n) {
            Some(Self(v))
        } else {
            None
        }
    }

    /// Converts a logical block address (LBA) to a sector number.
    ///
    /// In the Fast File System (FFS), logical blocks are 4 KiB (`0x1000`) in
    /// size, while sectors are 512 bytes. Since LBA numbering starts at `1`
    /// (with `0` being invalid), this function calculates the corresponding
    /// sector.
    ///
    /// # Returns
    /// - A [`Sector`] corresponding to the given logical block.
    pub const fn into_sector(self) -> Sector {
        // LBA is starting from 1. Zero represents the invalid LBA.
        Sector(((self.0.get() - 1) * (0x1000 / 512)) as usize)
    }

    /// Converts this logical block address into the corresponding location in
    /// the block allocation bitmap.
    ///
    /// # Arguments
    /// - `fs`: Reference to the file system's internal metadata layout.
    ///
    /// # Returns
    /// - `Some((lba, index))`: if the block address is valid, which includes:
    ///   - `lba`: the logical block address of the bitmap block that tracks
    ///     this data block.
    ///   - `index`: the index within the bitmap block where the relevant bit
    ///     resides.
    /// - `None`: if the logical block address is out of bounds or outside the
    ///   data region.
    pub fn into_bitmap_lba_offset(
        self,
        ffs: &FastFileSystemInner,
    ) -> Option<(LogicalBlockAddress, usize)> {
        if self >= ffs.data_block_start() && self < ffs.data_block_start() + ffs.block_count {
            Some((
                ffs.block_bitmap().start + (self.0.get() as usize / 0x8000),
                self.0.get() as usize % 0x8000,
            ))
        } else {
            None
        }
    }

    /// Returns the contained value as a u64.
    #[inline]
    pub fn into_u64(&self) -> u64 {
        self.0.get()
    }
}

// Sugars for LBA and FBN.
impl Step for LogicalBlockAddress {
    fn steps_between(start: &Self, end: &Self) -> (usize, Option<usize>) {
        if start.0 <= end.0 {
            let steps = (end.0.get() - start.0.get()) as usize;
            (steps, Some(steps))
        } else {
            (0, None)
        }
    }

    fn forward_checked(start: Self, count: usize) -> Option<Self> {
        start.0.get().checked_add(count as u64).and_then(Self::new)
    }

    fn backward_checked(start: Self, count: usize) -> Option<Self> {
        start.0.get().checked_sub(count as u64).and_then(Self::new)
    }
}

impl core::ops::Add<usize> for LogicalBlockAddress {
    type Output = Self;

    fn add(self, rhs: usize) -> Self {
        Self(NonZeroU64::new(self.0.get() + rhs as u64).unwrap())
    }
}
