//! # Extension of the [`FileStruct`].
//!
//! Up to this point, you have implemented a fully functional file system.
//! Now you will extend the basic [`FileStruct`] interface to provide
//! higher-level functionality. The [`AdvancedFileStructs`] trait builds on
//! the existing interface by introducing additional operations that are
//! essential for a complete and usable file system. These include support
//! for creating and removing files (`create`, `unlink`), managing directories
//! (`mkdir`, `chdir`), enumerating directory entries (`readdir`), retrieving
//! file metadata (`stat`), and ensuring persistence with `fsync`.
//!
//! ## Implementation Requirements
//! You need to implement the followings:
//! - [`FileStruct::create`]
//! - [`FileStruct::mkdir`]
//! - [`FileStruct::unlink`]
//! - [`FileStruct::chdir`]
//! - [`FileStruct::readdir`]
//! - [`FileStruct::stat`]
//! - [`FileStruct::fsync`]
//!
//! # Final Remarks
//! 🎉 Congratulations! By completing this section, you have successfully
//! finished the entire **KeOS** project. You have built, from the ground up, a
//! **minimal yet fully functional operating system** capable of running
//! multi-threaded user processes, managing virtual memory, and supporting a
//! journaling file system.
//!
//! Through this journey, you've gained hands-on experience with core OS
//! subsystems - process scheduling, memory protection, system calls, page
//! tables, file abstraction, and crash-consistent storage. This experience
//! provides deep insight into the **“invisible” responsibilities** of the
//! operating system to support software. Whatever you work with software
//! engineering or pursue low-level engineering, the knowledge and skills you’ve
//! developed here form a strong foundation to understand how your program works
//! on the computer.

use keos::{KernelError, fs::File};
use keos_project1::{file_struct::FileStruct, syscall::SyscallAbi};

/// Represents a directory entry as visible to user-space programs.
///
/// This struct contains the basic information about a directory entry
/// that user programs can observe, including the inode number and name of the
/// record.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct Dentry {
    /// The inode number corresponding to the file or directory.
    pub ino: u64,
    /// The name of entry in null-terminated string.
    pub name: [u8; 256],
}

/// Represents the basic metadata of a file or directory exposed to user-space.
///
/// This struct is typically returned by `stat()` to provide information about a
/// file.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct Stat {
    /// The inode number of the file or directory.
    pub inode: u64,
    /// The type of the file:
    /// - `0` = regular file
    /// - `1` = directory
    pub ty: u32,
    /// The size of the file in bytes.
    pub size: u64,
    #[doc(hidden)]
    pub __must_be_zero: u32,
}

impl Stat {
    /// Create a [`Stat`] struct for the file.
    pub fn new(file: &File) -> Self {
        Self {
            inode: file.ino().into_u32() as u64,
            ty: if matches!(file, File::RegularFile(_)) {
                0
            } else {
                1
            },
            size: file.size(),
            __must_be_zero: 0,
        }
    }
}

/// A trait for extending file operation functionality.
///
/// This trait provides implementations for file system-related system calls
/// that operate on files and directories. Each method corresponds to a
/// specific system call, handling user-space arguments via [`SyscallAbi`]
/// and returning either success or a [`KernelError`] on failure.
pub trait AdvancedFileStructs {
    /// Creates a new empty file in the current directory.
    ///
    /// # Syscall API
    /// ```c
    /// int create(const char *pathname);
    /// ```
    /// - `pathname`: Path of the new file to create.
    ///
    /// Returns `0` on success.
    fn create(&mut self, abi: &SyscallAbi) -> Result<usize, KernelError>;

    /// Creates a new directory in the current working directory.
    ///
    /// # Syscall API
    /// ```c
    /// int mkdir(const char *pathname);
    /// ```
    /// - `pathname`: Path of the new directory to create.
    ///
    /// Returns `0` on success.
    fn mkdir(&mut self, abi: &SyscallAbi) -> Result<usize, KernelError>;

    /// Removes a file from the file system.
    ///
    /// # Syscall API
    /// ```c
    /// int unlink(const char *pathname);
    /// ```
    /// - `pathname`: Path of the file to remove.
    ///
    /// Returns `0` on success.
    fn unlink(&mut self, abi: &SyscallAbi) -> Result<usize, KernelError>;

    /// Changes the current working directory.
    ///
    /// # Syscall API
    /// ```c
    /// int chdir(const char *pathname);
    /// ```
    /// - `pathname`: Path of the directory to change to.
    ///
    /// Returns `0` on success.
    fn chdir(&mut self, abi: &SyscallAbi) -> Result<usize, KernelError>;

    /// Reads directory entries from the current directory.
    ///
    /// # Syscall API
    /// ```c
    /// ssize_t readdir(int fd, struct dentry *buf, size_t count);
    /// ```
    /// - `fd`: File descriptor of the directory to read from.
    /// - `buf`: a pointer to the array of the dentries.
    /// - `count`: the number of entries in the array.
    ///
    /// Returns the number of entries read into the buffer.
    fn readdir(&mut self, abi: &SyscallAbi) -> Result<usize, KernelError>;

    /// Retrieves file metadata.
    ///
    /// # Syscall API
    /// ```c
    /// int stat(const char *pathname, struct stat *buf);
    /// ```
    /// - `pathname`: Path of the file or directory.
    /// - `buf`: Buffer to store the metadata.
    ///
    /// Returns `0` on success.
    fn stat(&mut self, abi: &SyscallAbi) -> Result<usize, KernelError>;

    /// Synchronizes in-memory file contents to disk.
    ///
    /// # Syscall API
    /// ```c
    /// int fsync(int fd);
    /// ```
    /// - `fd`: File descriptor of the file to synchronize.
    ///
    /// Returns `0` on success.
    fn fsync(&mut self, abi: &SyscallAbi) -> Result<usize, KernelError>;
}

impl AdvancedFileStructs for FileStruct {
    /// Creates a new empty file in the current directory.
    ///
    /// # Syscall API
    /// ```c
    /// int create(const char *pathname);
    /// ```
    /// - `pathname`: Path of the new file to create.
    ///
    /// Returns `0` on success.
    fn create(&mut self, abi: &SyscallAbi) -> Result<usize, KernelError> {
        todo!()
    }

    /// Creates a new directory in the current working directory.
    ///
    /// # Syscall API
    /// ```c
    /// int mkdir(const char *pathname);
    /// ```
    /// - `pathname`: Path of the new directory to create.
    ///
    /// Returns `0` on success.
    fn mkdir(&mut self, abi: &SyscallAbi) -> Result<usize, KernelError> {
        todo!()
    }

    /// Removes a file from the file system.
    ///
    /// # Syscall API
    /// ```c
    /// int unlink(const char *pathname);
    /// ```
    /// - `pathname`: Path of the file to remove.
    ///
    /// Returns `0` on success.
    fn unlink(&mut self, abi: &SyscallAbi) -> Result<usize, KernelError> {
        todo!()
    }

    /// Changes the current working directory.
    ///
    /// # Syscall API
    /// ```c
    /// int chdir(const char *pathname);
    /// ```
    /// - `pathname`: Path of the directory to change to.
    ///
    /// Returns `0` on success.
    fn chdir(&mut self, abi: &SyscallAbi) -> Result<usize, KernelError> {
        todo!()
    }

    /// Reads directory entries from the current directory.
    ///
    /// # Syscall API
    /// ```c
    /// ssize_t readdir(int fd, struct dentry *buf, size_t count);
    /// ```
    /// - `fd`: File descriptor of the directory to read from.
    /// - `buf`: a pointer to the array of the dentries.
    /// - `count`: the number of entries in the array.
    ///
    /// Returns the number of entries read into the buffer.
    fn readdir(&mut self, abi: &SyscallAbi) -> Result<usize, KernelError> {
        todo!()
    }

    /// Retrieves file metadata.
    ///
    /// # Syscall API
    /// ```c
    /// int stat(const char *pathname, struct stat *buf);
    /// ```
    /// - `pathname`: Path of the file or directory.
    /// - `buf`: Buffer to store the metadata.
    ///
    /// Returns `0` on success.
    fn stat(&mut self, abi: &SyscallAbi) -> Result<usize, KernelError> {
        todo!()
    }

    /// Synchronizes in-memory file contents to disk.
    ///
    /// # Syscall API
    /// ```c
    /// int fsync(int fd);
    /// ```
    /// - `fd`: File descriptor of the file to synchronize.
    ///
    /// Returns `0` on success.
    fn fsync(&mut self, abi: &SyscallAbi) -> Result<usize, KernelError> {
        todo!()
    }
}
