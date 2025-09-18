//! ## File system objects.
//!
//! Every on-disk object is represented by an **inode**. This inode is
//! diverge into multiple file types like a **regular file** or a **directory**
//! based on the [`Inode::ftype`] field. In this project, you will support only
//! two primary types of file:
//! - [`RegularFile`]: A **regular file** is the most common file type. It
//!   represents a sequence of bytes, typically used to store application data,
//!   logs, executables, or any other form of persistent information. The
//!   operating system exposes system calls such as `read()`, and `write()` to
//!   allow user programs to interact with regular files. Inodes for regular
//!   files point to data blocks on disk that store the actual file content.
//! - [`Directory`]: A **directory** is a special type of file that acts as a
//!   container for other files. Internally, a directory consists of a list of
//!   entries that map human-readable file names to inode numbers. This mapping
//!   allows the kernel to resolve file paths and traverse the hierarchical file
//!   system structure. Directory inodes point to data blocks containing
//!   serialized directory entries rather than arbitrary user data.
//!
//! ## Directory Internals
//! A directory is represented as a collection of directory entries
//! stored within the **data blocks** of the directory inode. Each directory
//! block contains an array of [`DirectoryBlockEntry`] structures, which provide
//! the mapping between human-readable names and their corresponding inode
//! numbers. All directory operations, such as adding, removing, or searching
//! for files and subdirectories, are performed by manipulating this collection
//! of entries. The directory **MUST** start with two entries: "." and "..",
//! which points to itself and the parent directory respectively.
//!
//! ## Implementation Requirements
//! You need to implement the followings:
//! - [`RegularFile::read`]
//! - [`RegularFile::write`]
//! - [`Directory::read_dir`]
//! - [`Directory::find`]
//! - [`Directory::open_entry`]
//! - [`Directory::create_entry`]
//!
//! At this point, you have implemented the core abstractions for managing
//! inodes in the filesystem, supporting both regular files and directories.
//! These abstractions provide safe and structured access to both on-disk and
//! in-memory inode data, allowing higher-level components to interact with
//! files without dealing directly with low-level disk structures.
//!
//! This module gives you insight into the internal foundations of the file
//! abstraction in the filesystem. The next [`section`] will focus on
//! **maintaining crash consistency in the filesystem**..
//!
//! [`section`]: mod@crate::ffs::journal
#[cfg(doc)]
use crate::ffs::inode::Inode;
use crate::ffs::{
    FastFileSystemInner, FileBlockNumber, InodeNumber,
    access_control::{MetaData, TrackedInode},
    disk_layout::{DirectoryBlock, DirectoryBlockEntry},
    journal::RunningTransaction,
    types::FileType,
};
use alloc::{
    string::String,
    sync::{Arc, Weak},
    vec::Vec,
};
#[cfg(doc)]
use keos::fs::traits::{Directory as _Directory, RegularFile as _RegularFile};
use keos::{KernelError, sync::atomic::AtomicBool};

/// A handle to a regular file in the filesystem.
///
/// This struct represents a low-level kernel handle to a regular file,
/// associated with a specific [`TrackedInode`] and the backing
/// [`FastFileSystemInner`] instance.
pub struct RegularFile {
    /// Weak reference to the [`FastFileSystemInner`].
    ffs: Weak<FastFileSystemInner>,
    /// The inode associated with this file.
    inode: TrackedInode,
}

impl RegularFile {
    /// Creates a new [`RegularFile`] from a given inode and filesystem
    /// reference.
    ///
    /// Returns `None` if the provided inode does not represent a regular file.
    ///
    /// # Parameters
    /// - `inode`: A tracked reference to the file’s inode.
    /// - `ffs`: A weak reference to the filesystem context.
    ///
    /// # Returns
    /// - `Some(RegularFile)` if the inode is valid and represents a regular
    ///   file.
    /// - `None` if the inode is invalid or not a regular file.
    pub fn new(inode: TrackedInode, ffs: Weak<FastFileSystemInner>) -> Option<Self> {
        if inode.read().ftype == FileType::RegularFile {
            Some(Self { inode, ffs })
        } else {
            None
        }
    }
}

impl keos::fs::traits::RegularFile for RegularFile {
    /// Inode number of the file.
    fn ino(&self) -> InodeNumber {
        self.inode.read().ino
    }

    /// Returns the size of the file in bytes.
    fn size(&self) -> usize {
        self.inode.read().size
    }

    /// Reads data from the file into the provided buffer.
    ///
    /// # Parameters
    /// - `ofs`: The `FileBlockNumber` which to read.
    /// - `buf`: A mutable array where the file content will be stored.
    ///
    /// # Returns
    /// - `Ok(true)`: If the read success.
    /// - `Ok(false)`: If the read failed.
    /// - `Err(Error)`: An error occured while the read operation.
    fn read(&self, fba: FileBlockNumber, buf: &mut [u8; 4096]) -> Result<bool, keos::KernelError> {
        let ffs = self.ffs.upgrade().unwrap();
        let inode = self.inode.read();
        match inode.get(&ffs, fba)? {
            Some(lba) => {
                todo!();
            }
            None => Ok(false),
        }
    }

    /// Writes a 4096-byte data into the specified file block.
    ///
    /// This method writes the contents of `buf` to the file block indicated by
    /// `fba`. If the target block lies beyond the current end of the file,
    /// the file may be extended up to `new_size` bytes to accommodate the
    /// write.
    ///
    /// However, if the target block lies beyond the current file size **and**
    /// `new_size` is insufficient to reach it, the write will fail.
    ///
    /// # Parameters
    /// - `fba`: The `FileBlockNumber` indicating the block to write to.
    /// - `buf`: A buffer containing exactly 4096 bytes of data to write.
    /// - `new_size`: The desired minimum file size (in bytes) after the write.
    ///   If this value is less than or equal to the current file size, no
    ///   growth occurs.
    ///
    /// # Returns
    /// - `Ok(())` if the write is successful.
    /// - `Err(KernelError)` if the operation fails (e.g., out-of-bounds write,
    ///   I/O error).
    fn write(
        &self,
        fba: FileBlockNumber,
        buf: &[u8; 4096],
        min_size: usize,
    ) -> Result<(), keos::KernelError> {
        let ffs = self.ffs.upgrade().unwrap();
        let tx = ffs.open_transaction("RegularFile::write");
        self.inode.write_with(&tx, |mut inode| {
            // Hint: Must conduct the following step
            // 1: Grow.
            // 2: Update the field `size`.
            // 3: Write to the data block.
            // 4: Submit change of the inode.
            todo!();
        })?;
        tx.commit()?;

        Ok(())
    }

    fn writeback(&self) -> Result<(), keos::KernelError> {
        Ok(())
    }
}

/// Represents a directory, which contains multiple directory entries.
///
/// This structure provides access to the directory's inode, which stores
/// information about the directory's metadata and contents.
pub struct Directory {
    /// Weak reference to the file system reference.
    pub ffs: Weak<FastFileSystemInner>,
    /// The inode associated with this directory.
    pub inode: TrackedInode,
    /// Whether the directory is removed,
    pub removed: AtomicBool,
}

impl Directory {
    /// Creates a new `Directory` from the given inode.
    ///
    /// # Arguments
    /// - `inode`: The tracked inode representing this directory.
    /// - `ffs`: A weak reference to the owning `FastFileSystemInner`.
    ///
    /// # Returns
    /// - `Some(Directory)`: if the provided inode is valid and represents a
    ///   directory.
    /// - `None`: if the inode is not of type `File::Directory`.
    pub fn new(inode: TrackedInode, ffs: Weak<FastFileSystemInner>) -> Option<Self> {
        if inode.read().ftype == FileType::Directory {
            Some(Self {
                inode,
                ffs,
                removed: AtomicBool::new(false),
            })
        } else {
            None
        }
    }

    /// Reads the contents of the directory.
    ///
    /// This function lists all the entries within the directory as a [`Vec`].
    ///
    /// A single entry is a tuple that consists of inode number and file name,
    /// that is `(InodeNumber, String)`.
    ///
    /// # Returns
    /// - `Ok(())`: If the directory was successfully read.
    /// - `Err(Error)`: An error if the read operation fails.
    pub fn read_dir(
        &self,
        ffs: &FastFileSystemInner,
    ) -> Result<Vec<(InodeNumber, String)>, keos::KernelError> {
        let mut output = Vec::new();
        let inode = self.inode.read();
        for fba in (0..inode.size.div_ceil(4096)).map(FileBlockNumber) {
            todo!()
        }
        Ok(output)
    }

    /// Finds the inode number corresponding to a directory entry by name.
    ///
    /// # Arguments
    /// - `ffs`: Reference to the file system’s internal structure.
    /// - `entry`: The name of the directory entry to search for.
    ///
    /// # Returns
    /// - `Ok(inode_number)`: if the entry is found in the directory.
    /// - `Err(KernelError)`: if the entry is not found or other errors occurs.
    pub fn find(&self, ffs: &FastFileSystemInner, entry: &str) -> Result<InodeNumber, KernelError> {
        todo!()
    }

    /// Adds a new entry to the directory.
    ///
    /// # Arguments
    /// - `ffs`: Reference to the file system’s internal structure.
    /// - `entry`: The name of the new directory entry.
    /// - `is_dir`: Whether the entry is a subdirectory (`true`) or a regular
    ///   file (`false`).
    /// - `tx`: A running transaction used to persist metadata changes.
    ///
    /// # Returns
    /// - `Ok(())`: if the entry is successfully added.
    /// - `Err(KernelError)`: if an error occurs (e.g., entry already exists or
    ///   out of space).
    pub fn add_entry(
        &self,
        ffs: &Arc<FastFileSystemInner>,
        entry: &str,
        ino: InodeNumber,
        tx: &RunningTransaction,
    ) -> Result<(), KernelError> {
        if self.removed.load() {
            return Err(KernelError::NoSuchEntry);
        }
        let en = DirectoryBlockEntry::from_ino_name(ino, entry).ok_or(KernelError::NameTooLong)?;
        // Read path
        {
            let inode = self.inode.read();
            // Find reusable entry.
            for fba in (0..inode.size.div_ceil(4096)).map(FileBlockNumber) {
                let lba = inode.get(ffs, fba)?;
                let blk = DirectoryBlock::load(ffs, lba.unwrap())?;
                let mut fit = None;
                {
                    let guard = blk.read();
                    for (i, en) in guard.iter().enumerate() {
                        if en.inode.is_none() {
                            fit = Some(i);
                            break;
                        }
                    }
                }
                if let Some(fit) = fit {
                    let mut guard = blk.write(tx);
                    guard[fit] = en;
                    guard.submit();
                    drop(inode);
                    return ffs.get_inode(ino).unwrap().write_with(tx, |mut inode| {
                        inode.link_count += 1;
                        inode.submit();
                        Ok(())
                    });
                }
            }
        }

        self.inode.write_with(tx, |mut inode| {
            // Grow the directory if no available space.
            let until = FileBlockNumber(inode.size.div_ceil(0x1000));
            inode.grow(ffs, until, tx)?;
            inode.size += 0x1000;

            // Fill the entry.
            let lba = inode.get(ffs, until)?;
            let blk = DirectoryBlock::load(ffs, lba.unwrap())?;
            let mut guard = blk.write(tx);
            guard[0] = en;
            guard.submit();
            inode.submit();
            ffs.get_inode(ino).unwrap().write_with(tx, |mut inode| {
                inode.link_count += 1;
                inode.submit();
                Ok(())
            })
        })
    }

    /// Takes an existing entry out of the directory.
    ///
    /// # Arguments
    /// - `ffs`: Reference to the file system’s internal structure.
    /// - `entry`: The name of the entry to remove.
    /// - `tx`: A running transaction used to persist metadata changes.
    ///
    /// # Returns
    /// - `Ok(())`: if the entry is successfully removed.
    /// - `Err(KernelError)`: if the entry does not exist or an I/O error
    ///   occurs.
    pub fn take_entry(
        &self,
        ffs: &Arc<FastFileSystemInner>,
        entry: &str,
        tx: &RunningTransaction,
    ) -> Result<TrackedInode, KernelError> {
        let guard = self.inode.read();
        for fba in (0..guard.size.div_ceil(4096)).map(FileBlockNumber) {
            let lba = guard.get(ffs, fba)?;
            let blk = DirectoryBlock::load(ffs, lba.unwrap())?;
            let mut fit = None;
            {
                let guard = blk.read();
                for (i, en) in guard.iter().enumerate() {
                    if en.name() == Some(entry) {
                        fit = Some(i);
                        break;
                    }
                }
            }
            if let Some(fit) = fit {
                let mut guard = blk.write(tx);
                let ino = guard[fit].inode.take();
                guard.submit();
                return ffs
                    .get_inode(ino.ok_or(KernelError::FilesystemCorrupted("DirectoryEntry"))?);
            }
        }
        Err(KernelError::NoSuchEntry)
    }
}

impl keos::fs::traits::Directory for Directory {
    /// Inode number of the directory.
    fn ino(&self) -> InodeNumber {
        self.inode.read().ino
    }

    /// Returns the size of the file in bytes.
    #[inline]
    fn size(&self) -> usize {
        self.inode.read().size
    }

    /// Link count of the directory.
    fn link_count(&self) -> usize {
        self.inode.read().link_count
    }

    /// Opens an entry by name.
    ///
    /// # Parameters
    /// - `entry`: The name of the entry to open.
    ///
    /// # Returns
    /// - `Ok(File)`: The enumerate of the file (e.g., regular file, directory).
    /// - `Err(Error)`: An error if the entry cannot be found or accessed.
    fn open_entry(&self, entry: &str) -> Result<keos::fs::File, keos::KernelError> {
        // Get the filesystem from the weak reference.
        let ffs = self
            .ffs
            .upgrade()
            .ok_or(KernelError::FilesystemCorrupted("File system closed."))?;
        // Find the inode corresponding to the entry from the directory.
        let ino = self.find(&ffs, entry)?;
        let inode = ffs.get_inode(ino)?;
        todo!()
    }

    /// Add an entry by name.
    ///
    /// # Parameters
    /// - `entry`: The name of the entry to add.
    /// - `is_dir`: Indicate whether the entry is directory or not.
    ///
    /// # Returns
    /// - `Ok(())`: If the entry was successfully added.
    /// - `Err(Error)`: An error if the add fails.
    fn create_entry(&self, entry: &str, is_dir: bool) -> Result<keos::fs::File, keos::KernelError> {
        // Get the filesystem from the weak reference.
        let ffs = self
            .ffs
            .upgrade()
            .ok_or(KernelError::FilesystemCorrupted("File system closed."))?;
        // Find whether the duplicated entry exists.
        match self.find(&ffs, entry) {
            Err(KernelError::NoSuchEntry) => {
                // If not exist, add the entry to the directory.
                let tx = ffs.open_transaction("Directory::add_entry");
                let parent_ino = self.inode.read().ino;
                let (ino, inode) = ffs.allocate_inode(is_dir, &tx)?;
                todo!()
            }
            Ok(_) => Err(KernelError::FileExist),
            Err(e) => Err(e),
        }
    }

    /// Removes a directory entry by name.
    ///
    /// # Errors
    /// - Returns [`KernelError::Busy`] when entry's Inode number is `1`
    ///  as it means it's a root directory.
    /// - Returns [`KernelError::DirectoryNotEmpty`] when the entry is a
    ///  non-empty directory
    /// - Returns [`KernelError::NoSuchEntry`] if specified entry does
    ///  not exists.
    ///
    /// # Parameters
    /// - `entry`: The name of the entry to remove.
    ///
    /// # Returns
    /// - `Ok(())`: If the entry was successfully removed.
    /// - `Err(Error)`: An error if the removal fails.
    fn unlink_entry(&self, entry: &str) -> Result<(), keos::KernelError> {
        // Get the filesystem from the weak reference.
        let ffs = self
            .ffs
            .upgrade()
            .ok_or(KernelError::FilesystemCorrupted("File system closed."))?;

        let tx = ffs.open_transaction("Directory::remove_entry");
        let inode = self.open_entry(entry)?;

        if inode.ino() == InodeNumber::new(1).unwrap() {
            return Err(KernelError::Busy);
        }

        let links_to_dec = match inode {
            keos::fs::File::Directory(d) => {
                if d.read_dir()?.len() != 2 {
                    Err(KernelError::DirectoryNotEmpty)
                } else {
                    match d.removed()?.swap(true) {
                        true => Err(KernelError::NoSuchEntry),
                        false => Ok(2),
                    }
                }
            }
            _ => Ok(1),
        }?;
        let inode = self.take_entry(&ffs, entry, &tx)?;
        inode.write_with(&tx, |mut ino| {
            ino.link_count -= links_to_dec;
            ino.submit();
            Ok(())
        })?;
        tx.commit()
    }

    /// Reads the contents of the directory.
    ///
    /// This function lists all the entries within the directory.
    ///
    /// # Returns
    /// - `Ok(())`: If the directory was successfully read.
    /// - `Err(Error)`: An error if the read operation fails.
    fn read_dir(&self) -> Result<Vec<(InodeNumber, String)>, keos::KernelError> {
        // Get the filesystem from the weak reference.
        let ffs = self
            .ffs
            .upgrade()
            .ok_or(KernelError::FilesystemCorrupted("File system closed."))?;
        self.read_dir(&ffs)
    }

    /// Returns [`AtomicBool`] which contains whether directory is removed.
    ///
    /// This is important because directory operations against the removed
    /// directory will result in undesirable behavior (e.g. unreachable file).
    ///
    /// # Returns
    /// - `Ok(())`: If the directory was successfully read.
    /// - `Err(Error)`: An error if the operation fails.
    fn removed(&self) -> Result<&AtomicBool, KernelError> {
        Ok(&self.removed)
    }
}
