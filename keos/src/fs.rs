//! Filesystem abstraction.

/// Defines traits for file system operations.
pub mod traits {
    use alloc::{string::String, vec::Vec};

    use super::{File, FileBlockNumber, InodeNumber};
    use crate::{KernelError, mm::Page, sync::atomic::AtomicBool};

    /// Trait representing a filesystem.
    ///
    /// This trait provides access to the root directory of the filesystem,
    /// allowing operations on files and directories.
    pub trait FileSystem
    where
        Self: Sync + Send,
    {
        /// Retrieves the root directory of the filesystem.
        ///
        /// # Returns
        /// - `Some(Directory)`: A reference to the root directory if available.
        /// - `None`: If the root directory is inaccessible or the filesystem is
        ///   uninitialized.
        fn root(&self) -> Option<super::Directory>;
    }

    /// Trait representing a regular file in the filesystem.
    ///
    /// A regular file contains user data and supports basic read and write
    /// operations.
    pub trait RegularFile
    where
        Self: Send + Sync,
    {
        /// Returns the inode number of the file.
        fn ino(&self) -> InodeNumber;

        /// Returns the size of the file in bytes.
        fn size(&self) -> usize;

        /// Reads data from the file into the provided buffer.
        ///
        /// # Parameters
        /// - `fba`: The `FileBlockNumber` which to read.
        /// - `buf`: A mutable array where the file content will be stored.
        ///
        /// # Returns
        /// - `Ok(true)`: If the read success.
        /// - `Ok(false)`: If the read success.
        /// - `Err(Error)`: An error occured while the read operation.
        fn read(&self, fba: FileBlockNumber, buf: &mut [u8; 4096]) -> Result<bool, KernelError>;

        /// Writes a 4096-byte page of data into the specified file block.
        ///
        /// This method writes the contents of `buf` to the file block indicated
        /// by `fba`. If the target block lies beyond the current end of
        /// the file, the file may be extended up to `new_size` bytes to
        /// accommodate the write.
        ///
        /// However, if the target block lies beyond the current file size
        /// **and** `min_size` is insufficient to reach it, the write
        /// will fail.
        ///
        /// # Parameters
        /// - `fba`: The `FileBlockNumber` indicating the block to write to.
        /// - `buf`: A buffer containing exactly 4096 bytes of data to write.
        /// - `min_size`: The desired minimum file size (in bytes) after the
        ///   write.   If this value is less than or equal to the current file
        ///   size, no growth occurs.
        ///
        /// # Returns
        /// - `Ok(())` if the write is successful.
        /// - `Err(KernelError)` if the operation fails (e.g., out-of-bounds
        ///   write, I/O error).
        fn write(
            &self,
            fba: FileBlockNumber,
            buf: &[u8; 4096],
            min_size: usize,
        ) -> Result<(), KernelError>;

        /// Maps a file block into memory.
        ///
        /// This method retrieves the contents of the file at the specified file
        /// block number (`fba`) and returns it as a [`Page`] containing the
        /// contents of the file block.
        ///
        /// The memory-mapped page reflects the current contents of the file at
        /// the requested block, and it can be used for reading or
        /// modifying file data at page granularity.
        ///
        /// # Parameters
        /// - `fba`: The file block number to map into memory. This is a logical
        ///   offset into the file, measured in fixed-size blocks (not bytes).
        ///
        /// # Returns
        /// - `Ok(Page)`: A reference-counted, in-memory page containing the
        ///   file block's data.
        /// - `Err(KernelError)`: If the file block cannot be found or loaded
        ///   (e.g., out-of-bounds access).
        fn mmap(&self, fba: FileBlockNumber) -> Result<Page, KernelError> {
            let mut page = Page::new();
            self.read(fba, page.inner_mut().as_mut_array().unwrap())?;
            Ok(page)
        }

        /// Write back the file to disk.
        fn writeback(&self) -> Result<(), KernelError>;
    }

    /// Trait representing a directory in the filesystem.
    ///
    /// A directory contains entries that reference other files or directories.
    pub trait Directory
    where
        Self: Send + Sync,
    {
        /// Returns the inode number of the directory.
        fn ino(&self) -> InodeNumber;

        /// Returns the size of the file in bytes.
        fn size(&self) -> usize;

        /// Returns the link count of the directory.
        fn link_count(&self) -> usize;

        /// Opens an entry by name.
        ///
        /// # Parameters
        /// - `entry`: The name of the entry to open.
        ///
        /// # Returns
        /// - `Ok(File)`: The enumerate of the file (e.g., regular file,
        ///   directory).
        /// - `Err(Error)`: An error if the entry cannot be found or accessed.
        fn open_entry(&self, entry: &str) -> Result<File, KernelError>;

        /// Create an entry by name.
        ///
        /// # Parameters
        /// - `entry`: The name of the entry to add.
        /// - `is_dir`: Indicate whether the entry is directory or not.
        ///
        /// # Returns
        /// - `Ok(())`: If the entry was successfully added.
        /// - `Err(Error)`: An error if the add fails.
        fn create_entry(&self, entry: &str, is_dir: bool) -> Result<File, KernelError>;

        /// Unlinks a directory entry by name.
        ///
        /// # Parameters
        /// - `entry`: The name of the entry to remove.
        ///
        /// # Returns
        /// - `Ok(())`: If the entry was successfully removed.
        /// - `Err(Error)`: An error if the removal fails.
        fn unlink_entry(&self, entry: &str) -> Result<(), KernelError>;

        /// Reads the contents of the directory.
        ///
        /// This function lists all the entries within the directory.
        ///
        /// # Returns
        /// - `Ok(())`: If the directory was successfully read.
        /// - `Err(Error)`: An error if the read operation fails.
        fn read_dir(&self) -> Result<Vec<(InodeNumber, String)>, KernelError>;

        /// Returns a reference of [`AtomicBool`] which contains whether
        /// directory is removed.
        ///
        /// This is important because directory operations against the removed
        /// directory will result in undesirable behavior (e.g. unreachable
        /// file).
        ///
        /// # Returns
        /// - `Ok(())`: If the directory was successfully read.
        /// - `Err(Error)`: An error if the operation fails.
        fn removed(&self) -> Result<&AtomicBool, KernelError>;
    }
}

use crate::{KernelError, mm::Page, sync::atomic::AtomicBool};
use alloc::{boxed::Box, string::String, sync::Arc, vec::Vec};
use core::{iter::Step, num::NonZeroU32};

/// A global file system abstraction.
///
/// The `FileSystem` struct provides an interface for interacting with the file
/// system, including operations such as opening and creating files.
///
/// # Example
/// ```
/// let fs = file_system();
/// if let Some(file) = fs.open("example.txt") {
///     println!("Opened file: {}", file.name());
/// }
/// ```
pub struct FileSystem {
    _p: (),
}

static mut FS: Option<Box<dyn traits::FileSystem>> = None;

impl FileSystem {
    /// Retrieves the root directory of the filesystem.
    ///
    /// # Returns
    /// - `Directory`: A reference to the root directory.
    pub fn root() -> Directory {
        unsafe { FS.as_ref() }
            .and_then(|fs| fs.root())
            .expect("Filesystem is not available.")
    }

    /// Register the global file system.
    pub fn register(fs: impl traits::FileSystem + 'static) {
        unsafe {
            FS = Some(Box::new(fs));
        }
    }
}

/// A handle to a regular file.
///
/// This struct provides a reference-counted handle to a file that supports
/// reading and writing operations at the kernel level.
#[derive(Clone)]
pub struct RegularFile(pub Arc<dyn traits::RegularFile>);

impl RegularFile {
    /// Inode number of the file.
    pub fn ino(&self) -> InodeNumber {
        self.0.ino()
    }

    /// Creates a new [`RegularFile`] handle from a given implementation of
    /// [`traits::RegularFile`].
    ///
    /// This function takes an instance of any type that implements the
    /// [`traits::RegularFile`] trait, wraps it in a reference-counted
    /// [`Arc`], and returns a [`RegularFile`] handle.
    ///
    /// # Parameters
    /// - `r`: An instance of a type that implements [`traits::RegularFile`].
    ///
    /// # Returns
    /// A [`RegularFile`] handle that enables reference-counted access to the
    /// underlying file.
    pub fn new(r: impl traits::RegularFile + 'static) -> Self {
        Self(Arc::new(r))
    }

    /// Returns the size of the file in bytes.
    #[inline]
    pub fn size(&self) -> usize {
        self.0.size()
    }

    /// Reads data from the file into the provided buffer.
    ///
    /// # Parameters
    /// - `fba`: The `FileBlockNumber` which to read.
    /// - `buf`: A mutable slice where the file content will be stored.
    ///
    /// # Returns
    /// - `Ok(usize)`: The number of bytes read.
    /// - `Err(Error)`: An error if the read operation fails.
    #[inline]
    pub fn read(&self, mut position: usize, buf: &mut [u8]) -> Result<usize, KernelError> {
        let mut bounce_buffer = alloc::boxed::Box::new([0; 4096]);
        let max_read = self
            .size()
            .min(position + buf.len())
            .saturating_sub(position);
        let mut read_bytes = 0;
        let first_segment = position & 0xfff;
        if first_segment != 0 {
            self.0.read(
                FileBlockNumber::from_offset(position & !0xfff),
                &mut bounce_buffer,
            )?;
            read_bytes += (0x1000 - first_segment).min(max_read);
            buf[..read_bytes]
                .copy_from_slice(&bounce_buffer[first_segment..first_segment + read_bytes]);
            position += read_bytes;
        }

        for i in (read_bytes..max_read).step_by(0x1000) {
            self.0
                .read(FileBlockNumber::from_offset(position), &mut bounce_buffer)?;
            let remainder = (max_read - i).min(0x1000);
            buf[i..i + remainder].copy_from_slice(&bounce_buffer[..remainder]);
            position += remainder;
            read_bytes += remainder;
        }
        Ok(read_bytes)
    }

    /// Writes data from the buffer into the file.
    ///
    /// If the write position is beyond the current file size, file will be
    /// extended to minimum size required to reflect the update.
    ///
    /// # Parameters
    /// - `fba`: The `FileBlockNumber` which to write.
    /// - `buf`: An slice containing the data to write.
    ///
    /// # Returns
    /// - `Ok(usize)`: The number of bytes written.
    /// - `Err(Error)`: An error if the write operation fails.
    #[inline]
    pub fn write(&self, mut position: usize, buf: &[u8]) -> Result<usize, KernelError> {
        let mut bounce_buffer = alloc::boxed::Box::new([0; 4096]);
        let mut write_bytes = 0;
        let first_segment = position & 0xfff;
        if first_segment != 0 {
            let r = self.0.read(
                FileBlockNumber::from_offset(position & !0xfff),
                &mut bounce_buffer,
            );
            if matches!(
                r,
                Err(KernelError::IOError) | Err(KernelError::FilesystemCorrupted(_))
            ) {
                return r.map(|_| 0);
            }
            write_bytes += (0x1000 - first_segment).min(buf.len());
            bounce_buffer[first_segment..first_segment + write_bytes]
                .copy_from_slice(&buf[..write_bytes]);
            self.0.write(
                FileBlockNumber::from_offset(position & !0xfff),
                &bounce_buffer,
                position + write_bytes,
            )?;
            position += write_bytes;
        }
        for i in (write_bytes..buf.len()).step_by(0x1000) {
            if buf.len() - i < 0x1000 {
                break;
            }
            self.0.write(
                FileBlockNumber::from_offset(position),
                buf[i..i + 0x1000].as_array().unwrap(),
                position + 0x1000,
            )?;
            position += 0x1000;
            write_bytes += 0x1000;
        }
        if write_bytes != buf.len() {
            let r = self
                .0
                .read(FileBlockNumber::from_offset(position), &mut bounce_buffer);
            if matches!(
                r,
                Err(KernelError::IOError) | Err(KernelError::FilesystemCorrupted(_))
            ) {
                return r.map(|_| 0);
            }
            let remainder = buf.len() - write_bytes;
            assert!(remainder < 0x1000);
            bounce_buffer[..remainder].copy_from_slice(&buf[write_bytes..]);
            self.0.write(
                FileBlockNumber::from_offset(position),
                &bounce_buffer,
                position + remainder,
            )?;
            write_bytes += remainder;
        }
        Ok(write_bytes)
    }

    /// Maps a file block into memory.
    ///
    /// This method retrieves the contents of the file at the specified file
    /// block number (`fba`) and returns it as a [`Page`] of the file
    /// block.
    ///
    /// The memory-mapped page reflects the current contents of the file at
    /// the requested block, and it can be used for reading or
    /// modifying file data at page granularity.
    ///
    /// # Parameters
    /// - `fba`: The file block number to map into memory. This is a logical
    ///   offset into the file, measured in fixed-size blocks (not bytes).
    ///
    /// # Returns
    /// - `Ok(Page)`: A reference-counted, in-memory page containing the file
    ///   block's data.
    /// - `Err(KernelError)`: If the file block cannot be found or loaded (e.g.,
    ///   out-of-bounds access).
    #[inline]
    pub fn mmap(&self, fba: FileBlockNumber) -> Result<Page, KernelError> {
        self.0.mmap(fba)
    }

    /// Write back the file to disk.
    pub fn writeback(&self) -> Result<(), KernelError> {
        self.0.writeback()
    }
}

/// A handle to a directory.
///
/// This struct represents a reference-counted directory that supports
/// file entry management, including opening and removing entries.
#[derive(Clone)]
pub struct Directory(pub Arc<dyn traits::Directory>);

impl Directory {
    /// Inode number of the directory.
    pub fn ino(&self) -> InodeNumber {
        self.0.ino()
    }

    /// Returns the size of the file in bytes.
    #[inline]
    pub fn size(&self) -> usize {
        self.0.size()
    }

    /// Link count of the directory.
    pub fn link_count(&self) -> usize {
        self.0.link_count()
    }

    /// Creates a new [`Directory`] handle from a given implementation of
    /// [`traits::Directory`].
    ///
    /// This function takes an instance of any type that implements the
    /// [`traits::Directory`] trait, wraps it in a reference-counted
    /// [`Arc`], and returns a [`Directory`] handle.
    ///
    /// # Parameters
    /// - `r`: An instance of a type that implements [`traits::Directory`].
    ///
    /// # Returns
    /// A [`Directory`] handle that enables reference-counted access to the
    /// underlying file.
    pub fn new(r: impl traits::Directory + 'static) -> Self {
        Self(Arc::new(r))
    }

    /// Opens a path from the directory.
    ///
    /// # Parameters
    /// - `path`: The path to the entry.
    ///
    /// # Returns
    /// - `Ok(File)`: The type of the file (e.g., regular file, directory).
    /// - `Err(Error)`: An error if the entry cannot be found or accessed.
    #[inline]
    pub fn open(&self, mut path: &str) -> Result<File, KernelError> {
        let mut ret = File::Directory(if path.starts_with("/") {
            path = &path[1..];
            FileSystem::root()
        } else {
            self.clone()
        });

        for part in path.split("/").filter(|&s| !s.is_empty()) {
            match ret {
                File::Directory(d) => ret = d.0.open_entry(part)?,
                File::RegularFile(_) => return Err(KernelError::NotDirectory),
            }
        }
        Ok(ret)
    }

    /// Create an entry in the directory.
    ///
    /// # Parameters
    /// - `path`: The path to the entry.
    /// - `is_dir`: Indicate whether the entry is directory or not.
    ///
    /// # Returns
    /// - `Ok(())`: If the entry was successfully added.
    /// - `Err(Error)`: An error if the add fails.
    #[inline]
    pub fn create(&self, mut path: &str, is_dir: bool) -> Result<File, KernelError> {
        let mut dstdir = if path.starts_with("/") {
            path = &path[1..];
            FileSystem::root()
        } else {
            self.clone()
        };

        let mut list: Vec<&str> = path.split("/").filter(|&s| !s.is_empty()).collect();
        let entry = list.pop().ok_or(KernelError::InvalidArgument)?;

        for part in list {
            dstdir = dstdir
                .0
                .open_entry(part)?
                .into_directory()
                .ok_or(KernelError::NoSuchEntry)?;
        }

        dstdir.0.create_entry(entry, is_dir)
    }

    /// Unlink an entry in the directory.
    ///
    /// # Parameters
    /// - `path`: The path to the entry.
    ///
    /// # Returns
    /// - `Ok(())`: If the entry was successfully added.
    /// - `Err(Error)`: An error if the add fails.
    #[inline]
    pub fn unlink(&self, mut path: &str) -> Result<(), KernelError> {
        let mut dstdir = if path.starts_with("/") {
            path = &path[1..];
            FileSystem::root()
        } else {
            self.clone()
        };

        let mut list: Vec<&str> = path.split("/").filter(|&s| !s.is_empty()).collect();
        let entry = list.pop().ok_or(KernelError::InvalidArgument)?;

        for part in list {
            dstdir = dstdir
                .0
                .open_entry(part)?
                .into_directory()
                .ok_or(KernelError::NoSuchEntry)?;
        }

        dstdir.0.unlink_entry(entry)
    }

    /// Reads the contents of the directory.
    ///
    /// This function lists all the entries within the directory.
    ///
    /// # Returns
    /// - `Ok(())`: If the directory was successfully read.
    /// - `Err(Error)`: An error if the read operation fails.
    #[inline]
    pub fn read_dir(&self) -> Result<Vec<(InodeNumber, String)>, KernelError> {
        self.0.read_dir()
    }

    /// Returns [`AtomicBool`] which contains whether directory is removed.
    ///
    /// This is important because directory operations against the removed
    /// directory will result in undesirable behavior (e.g. unreachable file).
    ///
    /// # Returns
    /// - `Ok(())`: If the directory was successfully read.
    /// - `Err(Error)`: An error if the operation fails.
    #[inline]
    pub fn removed(&self) -> Result<&AtomicBool, KernelError> {
        self.0.removed()
    }
}

/// Represents a file system entry, which can be either a file or a directory.
///
/// This enum allows distinguishing between regular files and directories within
/// the filesystem. It provides flexibility for handling different file system
/// objects in a unified manner.
#[derive(Clone)]
pub enum File {
    /// A regular file.
    ///
    /// This variant represents a standard file in the filesystem, which can be
    /// read from or written to.
    RegularFile(RegularFile),

    /// A directory.
    ///
    /// This variant represents a directory in the filesystem, which can contain
    /// other files or directories.
    Directory(Directory),
}

impl File {
    /// Converts the [`File`] into a [`RegularFile`], if it is one.
    ///
    /// # Returns
    ///
    /// - `Some(RegularFile)` if `self` is a [`RegularFile`].
    /// - `None` if `self` is not a `RegularFile`.
    ///
    /// This function allows extracting the [`RegularFile`] from [`File`]
    /// safely.
    pub fn into_regular_file(self) -> Option<RegularFile> {
        if let File::RegularFile(r) = self {
            Some(r)
        } else {
            None
        }
    }

    /// Converts the `File` into a `Directory`, if it is one.
    ///
    /// # Returns
    ///
    /// - `Some(Directory)` if `self` is a `Directory`.
    /// - `None` if `self` is not a `Directory`.
    ///
    /// This function allows extracting the `Directory` from `File` safely.
    ///
    /// # Example
    ///
    /// ```
    /// let dir = File::Directory(directory);
    /// assert!(dir.into_directory().is_some());
    ///
    /// let file = File::RegularFile(regular_file);
    /// assert!(file.into_directory().is_none());
    /// ```
    pub fn into_directory(self) -> Option<Directory> {
        if let File::Directory(d) = self {
            Some(d)
        } else {
            None
        }
    }

    /// Get [`InodeNumber`] of this [`File`] regardless of its inner type.
    pub fn ino(&self) -> InodeNumber {
        match self {
            File::RegularFile(r) => r.ino(),
            File::Directory(d) => d.ino(),
        }
    }

    /// Get size of this [`File`] regardless of its inner type.
    pub fn size(&self) -> u64 {
        match self {
            File::RegularFile(r) => r.size() as u64,
            File::Directory(d) => d.size() as u64,
        }
    }
}

/// Sector, an access granuality for the disk.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct Sector(pub usize);

impl Sector {
    /// Get offset that represented by the sector.
    #[inline]
    pub fn into_offset(self) -> usize {
        self.0 * 512
    }

    /// Cast into usize.
    #[inline]
    pub fn into_usize(self) -> usize {
        self.0
    }
}

impl core::ops::Add<usize> for Sector {
    type Output = Self;

    fn add(self, rhs: usize) -> Self {
        Self(self.0 + rhs)
    }
}

/// Represents a unique identifier for an inode in the filesystem.
///
/// An inode number uniquely identifies a file or directory within a filesystem.
/// It is typically used to reference file metadata rather than file names.
#[repr(transparent)]
#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub struct InodeNumber(NonZeroU32);

impl InodeNumber {
    /// Creates a [`InodeNumber`] if the given value is not zero.
    pub const fn new(n: u32) -> Option<Self> {
        if let Some(v) = NonZeroU32::new(n) {
            Some(Self(v))
        } else {
            None
        }
    }

    /// Returns the contained value as a u32.
    #[inline]
    pub fn into_u32(&self) -> u32 {
        self.0.get()
    }
}

/// Represents a file block number within a file.
///
/// This number refers to the position of a block within a specific file.
/// Each block contains 4096 bytes of contents.
/// It helps in translating file-relative offsets to actual storage locations.
#[repr(transparent)]
#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub struct FileBlockNumber(pub usize);

impl FileBlockNumber {
    /// Computes the file block number from a byte offset within a file.
    ///
    /// In KeOS, each file is divided into file blocks of `0x1000` bytes (4
    /// KiB). This function calculates the file block index corresponding to a
    /// given byte offset.
    ///
    /// # Parameters
    /// - `offset`: The byte offset within the file.
    ///
    /// # Returns
    /// - The [`FileBlockNumber`] that corresponds to the given offset.
    pub const fn from_offset(offset: usize) -> Self {
        Self(offset / 0x1000)
    }
}

impl Step for FileBlockNumber {
    fn steps_between(start: &Self, end: &Self) -> (usize, Option<usize>) {
        if start.0 <= end.0 {
            let steps = end.0 - start.0;
            (steps, Some(steps))
        } else {
            (0, None)
        }
    }

    fn forward_checked(start: Self, count: usize) -> Option<Self> {
        start.0.checked_add(count).map(Self)
    }

    fn backward_checked(start: Self, count: usize) -> Option<Self> {
        start.0.checked_sub(count).map(Self)
    }
}

impl core::ops::Add<usize> for FileBlockNumber {
    type Output = Self;

    fn add(self, rhs: usize) -> Self {
        Self(self.0 + rhs)
    }
}

// The type for disk hooking.
#[doc(hidden)]
pub type Hook =
    Arc<dyn Fn(Sector, &[u8; 512], bool) -> Result<(), KernelError> + Send + Sync + 'static>;

/// The disk, a device that has byte sink.
///
/// It gets slot number as its field.
pub struct Disk {
    index: usize,
    is_ro: bool,
    hook: Option<Hook>,
}

impl Disk {
    /// Create a new FsDisk from the index.
    pub fn new(index: usize) -> Self {
        Self {
            index,
            is_ro: false,
            hook: None,
        }
    }

    /// Make the disk read-only.
    pub fn ro(self) -> Self {
        Self {
            index: self.index,
            is_ro: true,
            hook: self.hook,
        }
    }

    /// Add a hook for the disk.
    pub fn hook(self, hook: Hook) -> Self {
        Self {
            index: self.index,
            is_ro: self.is_ro,
            hook: Some(hook),
        }
    }

    /// Read 512 bytes from disk starting from sector.
    pub fn read(&self, sector: Sector, buf: &mut [u8; 512]) -> Result<(), KernelError> {
        let dev = abyss::dev::get_bdev(self.index).ok_or(KernelError::IOError)?;
        if let Some(hook) = self.hook.as_ref() {
            hook(sector, buf, false)?;
        }
        dev.read_bios(&mut Some((512 * sector.into_usize(), buf.as_mut())).into_iter())
            .map_err(|_| KernelError::IOError)
    }

    /// Write 512 bytes to disk starting from sector.
    pub fn write(&self, sector: Sector, buf: &[u8; 512]) -> Result<(), KernelError> {
        let dev = abyss::dev::get_bdev(self.index).ok_or(KernelError::IOError)?;
        if self.is_ro {
            Err(KernelError::NotSupportedOperation)
        } else {
            if let Some(hook) = self.hook.as_ref() {
                hook(sector, buf, true)?;
            }
            dev.write_bios(&mut Some((512 * sector.into_usize(), buf.as_ref())).into_iter())
                .map_err(|_| KernelError::IOError)
        }
    }
}
