//! # File state of a process.
//!
//! One of the kernel's primary responsibilities is managing process states.
//! A process is an instance of a program being executed, abstracting a machine
//! by encompassing various states like memory allocation, CPU registers, and
//! the files it operates on. These process states are crucial for the kernel to
//! allocate resources, prioritize tasks, and manage the process lifecycle
//! (including creation, execution, suspension, and termination). The kernel
//! processes system calls by evaluating the current state of the associated
//! processes, checking resource availability, and ensuring that the requested
//! operation is carried out safely and efficiently. Between them, this project
//! focuses on the kernel's interaction with the file system.
//!
//! ## File
//!
//! A **file** primary refers an interface for accessing disk-based data. At its
//! core, a file serves as a sequential stream of bytes. There are two primary
//! types of files in most file systems:
//!
//! - **Regular files**: These contain user or system data, typically organized
//!   as a sequence of bytes. They can store text, binary data, executable code,
//!   and more. Regular files are the most common form of file used by
//!   applications for reading and writing data.
//!
//! - **Directories**: A directory is a special kind of file that contains
//!   mappings from human-readable names (filenames) to other files or
//!   directories. Directories form the backbone of the file system’s
//!   hierarchical structure, allowing files to be organized and accessed via
//!   paths.
//!
//! Processes interact with files through **file descriptors**, which serve
//! as handles to open file objects. File descriptors provide an indirection
//! layer that allows user programs to perform operations like reading, writing,
//! seeking, and closing, without exposing the internal details of file objects.
//! This file descriptor plays a **crucial security role**: actual file objects
//! reside in kernel space, and are never directly accessible from user
//! space. By using descriptors as opaque references, the operating system
//! enforces strict isolation between user and kernel memory, preventing
//! accidental or malicious tampering with sensitive kernel-managed resources.
//!
//! File descriptors are small integer values, typically starting from 0, that
//! index into the process's file descriptor table. This table holds references
//! to open file objects, including metadata like the file's location, access
//! mode (e.g., read or write), and other details necessary for I/O operations.
//! When a process issues a file operation (e.g., reading, writing, or seeking),
//! it provides the appropriate file descriptor as an argument to the system
//! call. The kernel uses this descriptor to access the corresponding entry in
//! the table and perform the requested operation.
//!
//! ## "Everything is a File"
//!
//! Beyond the abstraction about disk, the **file** abstraction is applied
//! uniformly across a wide range of system resources. "Everything is a file" is
//! a Unix-inspired design principle that simplifies system interaction by
//! treating various resources—including devices, sockets, and processes—as
//! files. While not an absolute rule, this philosophy influences many
//! Unix-based systems, encouraging the representation of objects as file
//! descriptors and enabling interaction through standard I/O operations. This
//! approach provides a unified and consistent way to handle different types of
//! system objects.
//!
//! A key aspect of this principle is the existence of **standard file
//! descriptors**:
//! - **Standard Input (stdin) - File Descriptor 0**: Used for reading input
//!   data (e.g., keyboard input or redirected file input).
//! - **Standard Output (stdout) - File Descriptor 1**: Used for writing output
//!   data (e.g., printing to the terminal or redirecting output to a file).
//! - **Standard Error (stderr) - File Descriptor 2**: Used for writing error
//!   messages separately from standard output.
//!
//! Another important mechanism following this design is the **pipe**, which
//! allows interprocess communication by connecting the output of one process to
//! the input of another. Pipes function as a buffer between processes,
//! facilitating seamless data exchange without requiring intermediate storage
//! in a file. For example, executing:
//!
//! ```sh
//! ls | grep "file"
//! ```
//! connects the `ls` command’s output to the `grep` command’s input through a
//! pipe.
//!
//! ## Files in KeOS
//!
//! You need to extend KeOS to support the following system call with a file
//! abstraction:
//! - [`open`]: Open a file.
//! - [`read`]: Read data from a file.
//! - [`write`]: Write data to a file.
//! - [`close`]: Close an open file.
//! - [`seek`]: Set the file pointer to a specific position.
//! - [`tell`]: Get the current position of the file.
//! - [`pipe`]: Create an interprocess communication channel.
//!
//! To manage the state about file, KeOS manages per-process specific state
//! about file called [`FileStruct`], which is corresponding to the Linux
//! kernel's `struct file_struct`. Through this struct, you need to manage file
//! descriptors that represent open files within a process. Since many system
//! interactions are built around file descriptors, understanding this principle
//! will help you design efficient and flexible system call handlers for file
//! operations.
//!
//! You need to implement system call handlers with [`FileStruct`] struct that
//! manages file states for a process. For example, it contains current working
//! directory of a file (cwd), and tables of file descriptors, which map each
//! file descriptor (fd) to a specific [`FileKind`] state. When invoking system
//! calls, you must update these file states accordingly, ensuring the correct
//! file state is used for each operation. To store the mapping between file
//! descriptor and [`FileKind`] state, KeOS utilizes `BTreeMap` provided by the
//! [`alloc::collections`] module. You might refer to [`channel`] and
//! [`teletype`] module for implementing stdio and channel I/O.
//!
//! As mentioned before, kernel requires careful **error handling**. The kernel
//! must properly ensuring that errors are reported in a stable and reliable
//! manner without causing system crashes.
//!
//! ## User Memory Access
//! Kernel **MUST NOT** believe the user input. User might maliciously or
//! mistakenly inject invalid inputs to the system call arguments. If such input
//! represents the invalid memory address or kernel address, directly accessing
//! the address can leads security threats.
//!
//! To safely interact with user-space memory when handling system call, KeOS
//! provides [`uaccess`] module:
//! - [`UserPtrRO`]: A read-only user-space pointer, used for safely retrieving
//!   structured data from user memory.
//! - [`UserPtrWO`]: A write-only user-space pointer, used for safely writing
//!   structured data back to user memory.
//! - [`UserCString`]: Read null-terminated strings from user-space (e.g., file
//!   paths).
//! - [`UserU8SliceRO`]: Read byte slices from user-space (e.g., buffers for
//!   reading files).
//! - [`UserU8SliceWO`]: Write byte slices to user-space (e.g., buffers for
//!   writing files).
//!
//! These types help prevent unsafe memory access and ensure proper bounds
//! checking before performing read/write operations. When error occurs during
//! the check, it returns the `Err` with [`KernelError::BadAddress`]. You can
//! simply combining the `?` operator with the methods to propagate the error to
//! the system call entry. Therefore, **you should never use `unsafe` code
//! directly for accessing user-space memory**. Instead, utilize these safe
//! abstractions, which provide built-in validation and access control, reducing
//! the risk of undefined behavior, security vulnerabilities, and kernel
//! crashes.

//! #### Implementation Requirements
//! You need to implement the followings:
//! - [`FileStruct::install_file`]
//! - [`FileStruct::open`]
//! - [`FileStruct::read`]
//! - [`FileStruct::write`]
//! - [`FileStruct::seek`]
//! - [`FileStruct::tell`]
//! - [`FileStruct::close`]
//! - [`FileStruct::pipe`]
//!
//! This ends the project 1.
//!
//! [`open`]: FileStruct::open
//! [`read`]: FileStruct::read
//! [`write`]: FileStruct::write
//! [`seek`]: FileStruct::seek
//! [`tell`]: FileStruct::tell
//! [`close`]: FileStruct::close
//! [`pipe`]: FileStruct::pipe
//! [`uaccess`]: keos::syscall::uaccess
//! [`UserPtrRO`]: keos::syscall::uaccess::UserPtrRO
//! [`UserPtrWO`]: keos::syscall::uaccess::UserPtrWO
//! [`UserCString`]: keos::syscall::uaccess::UserPtrWO
//! [`UserU8SliceRO`]: keos::syscall::uaccess::UserU8SliceRO
//! [`UserU8SliceWO`]: keos::syscall::uaccess::UserU8SliceWO
//! [`alloc::collections`]: <https://doc.rust-lang.org/alloc/collections/index.html>

use crate::syscall::SyscallAbi;
use alloc::collections::BTreeMap;
use keos::{
    KernelError,
    fs::{Directory, RegularFile},
    syscall::flags::FileMode,
};
#[cfg(doc)]
use keos::{channel, teletype};

/// The type of a file in the filesystem.
///
/// This enum provides a way to distinguish between regular files and special
/// files like standard input (stdin), standard output (stdout), standard error
/// (stderr), and interprocess communication (IPC) channels such as pipes.
/// It allows the system to treat these different types of files accordingly
/// when performing file operations like reading, writing, or seeking.
#[derive(Clone)]
pub enum FileKind {
    /// A regular file on the filesystem.
    RegularFile {
        /// A [`RegularFile`] object, which holds the underlying kernel
        /// structure that represents the actual file in the kernel's
        /// memory. This structure contains additional metadata about
        /// the file, such as its name.
        file: RegularFile,
        /// The current position in the file (offset).
        ///
        /// This field keeps track of the current position of the file pointer
        /// within the file. The position is measured in bytes from the
        /// beginning of the file. It is updated whenever a read or write
        /// operation is performed, allowing the system to track where
        /// the next operation will occur.
        ///
        /// Example: If the file's position is 100, the next read or write
        /// operation will begin at byte 100.
        position: usize,
    },
    /// A directory of the filesystem.
    ///
    /// This variant represents a directory in the filesystem. Unlike regular
    /// files, directories serve as containers for other files and
    /// directories. Operations on directories typically include listing
    /// contents, searching for files, and navigating file structures.
    Directory {
        dir: Directory,
        /// The current position in the directory (offset).
        ///
        /// This field is internally used in readdir() function to track
        /// how much entries
        position: usize,
    },
    /// A special file for standard input/output streams.
    ///
    /// This variant represents standard I/O streams like stdin, stdout, and
    /// stderr. These are not associated with physical files on disk but are
    /// used for interaction between processes and the console or terminal.
    ///
    /// - **Standard Input (`stdin`)**: Used to receive user input.
    /// - **Standard Output (`stdout`)**: Used to display process output.
    /// - **Standard Error (`stderr`)**: Used to display error messages.
    Stdio,
    /// A receive endpoint for interprocess communication (IPC).
    ///
    /// This variant represents a receiving channel in an IPC mechanism,
    /// commonly used for message-passing between processes. It
    /// acts as a read-only endpoint, allowing a process to receive data
    /// from a corresponding [`FileKind::Tx`] (transmit) channel.
    ///
    /// Data sent through the corresponding [`FileKind::Tx`] endpoint is
    /// buffered and can be read asynchronously using this receiver. Once
    /// all [`FileKind::Tx`] handles are closed, reads will return an
    /// end-of-file (EOF) indication.
    ///
    /// This is useful for implementing features like pipes, message queues, or
    /// event notifications.
    Rx(keos::channel::Receiver<u8>),
    /// A transmit endpoint for interprocess communication (IPC).
    ///
    /// This variant represents a sending channel in an IPC mechanism. It serves
    /// as a write-only endpoint, allowing a process to send data to a
    /// corresponding [`FileKind::Rx`] (receive) channel.
    ///
    /// Data written to this [`FileKind::Tx`] endpoint is buffered until it is
    /// read by the corresponding [`FileKind::Rx`] endpoint. If no receiver
    /// exists, writes may block or fail depending on the system's IPC
    /// behavior.
    ///
    /// This is commonly used in pipes, producer-consumer queues, and task
    /// synchronization mechanisms.
    Tx(keos::channel::Sender<u8>),
}

/// The [`File`] struct represents an abstraction over a file descriptor in the
/// operating system.
///
/// This struct encapsulates information about an open file,
/// access mode, and other metadata necessary for performing file operations
/// such as reading, writing, and seeking. It also holds a reference to the
/// kernel's underlying file structure ([`FileKind`]), allowing the operating
/// system to perform actual file operations on the filesystem.
///
/// The [`File`] struct is used to track the state of an open file, ensuring
/// that the correct file operations are executed and resources are managed
/// efficiently.
#[derive(Clone)]
pub struct File {
    /// The access mode of the file (e.g., read, write, read/write).
    ///
    /// [`FileMode`] is used by user program to tell kernel "how" open the file,
    /// and records internally what operation can be done on the file.
    ///
    /// Refer to [`FileMode`] for detail.
    pub mode: FileMode,

    /// The kernel file structure.
    ///
    /// This field contains the underlying representation of the file within the
    /// operating system kernel. It holds the kernel's metadata for the
    /// file, such as its name, permissions, and the actual file object used
    /// to perform system-level file operations.
    ///
    /// The [`FileKind`] enum allows this field to represent either a regular
    /// file ([`FileKind::RegularFile`]) or a special file such as standard
    /// input/output ([`FileKind::Stdio`]).
    pub file: FileKind,
}

/// Represents an index into a process’s file descriptor table.
///
/// In most operating systems, each process maintains a **file descriptor
/// table** that maps small integers (file descriptors) to open file objects.
/// A [`FileDescriptor`] is a wrapper around an `i32` that provides
/// stronger type safety when handling these indices in the kernel.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct FileDescriptor(pub i32);

/// The [`FileStruct`] represents the filesystem state for a specific
/// process, which corresponding to the Linux kernel's `struct files_struct`.
///
/// This struct encapsulates information about the current state of the
/// filesystem for the process, including the management of open
/// files, their positions, and the operations that can be performed on them.
/// The [`FileStruct`] is responsible for keeping track of file descriptors and
/// their associated file states, ensuring that file operations (like
/// reading, writing, seeking, and closing) are executed correctly and
/// efficiently.
///
/// It also provides a mechanism for the operating system to manage and interact
/// with files in a multi-process environment, allowing for
/// process-local filesystem management.
///
/// # Filesystem State
///
/// The filesystem state refers to the set of files currently open for a given
///  process. This includes managing the file descriptors (unique
/// identifiers for open files), file positions, and ensuring that resources are
/// freed once a file is closed.
#[derive(Clone)]
pub struct FileStruct {
    /// The current working directory of the process.
    pub cwd: Directory,
    /// The file descriptor table of the process.
    pub files: BTreeMap<FileDescriptor, File>,
}

impl Default for FileStruct {
    fn default() -> Self {
        Self::new()
    }
}

impl FileStruct {
    /// Creates a new instance of [`FileStruct`].
    ///
    /// This function initializes a new filesystem state, typically when a
    /// process starts or when a fresh file operation is needed.
    ///
    /// # Returns
    ///
    /// Returns a new [`FileStruct`] struct, representing a clean slate for the
    /// filesystem state. The clean state must initialize the STDIN, STDOUT,
    /// STDERR.
    pub fn new() -> Self {
        let mut this = Self {
            cwd: keos::fs::FileSystem::root(),
            files: BTreeMap::new(),
        };
        this.install_file(File {
            mode: FileMode::Read,
            file: FileKind::Stdio,
        })
        .unwrap();
        this.install_file(File {
            mode: FileMode::Write,
            file: FileKind::Stdio,
        })
        .unwrap();
        this.install_file(File {
            mode: FileMode::Write,
            file: FileKind::Stdio,
        })
        .unwrap();
        this
    }

    /// Installs a [`File`] into the process’s file descriptor table.
    ///
    /// This method assigns the lowest available file descriptor number to
    /// `file` and returns it as a [`FileDescriptor`].
    /// The descriptor can then be used by the process to perform I/O operations
    /// such as `read`, `write`, `stat`, or `close`.
    ///
    /// # Errors
    /// - Returns [`KernelError::TooManyOpenFile`] if the process already has
    ///   more than **1024 open files**, meaning no additional descriptors are
    ///   available.
    pub fn install_file(&mut self, file: File) -> Result<FileDescriptor, KernelError> {
        todo!()
    }

    /// Opens a file.
    ///
    /// This function handles the system call for opening a file, including
    /// checking if the file exists, and setting up the file's access mode
    /// (e.g., read, write, or append). It modifies the [`FileStruct`] by
    /// associating the file with the current process and prepares the file
    /// for subsequent operations.
    ///
    /// # Syscall API
    /// ```c
    /// int open(const char *pathname, int flags);
    /// ```
    /// - `pathname`: Path to the file to be opened.
    /// - `flags`: Specifies the access mode. The possible values are:
    ///   - `O_RDONLY` (0): The file is opened for read only.
    ///   - `O_WRONLY` (1): The file is opened for write only.
    ///   - `O_RDWR`   (2): The file is opened for both read and write.
    ///
    /// Returns the corresponding file descriptor number for the opened file.
    pub fn open(&mut self, abi: &SyscallAbi) -> Result<usize, KernelError> {
        todo!()
    }

    /// Reads data from an open file.
    ///
    /// This function implements the system call for reading from an open file.
    /// It reads up to a specified number of bytes from the file and returns
    /// them to the user. The current file position is adjusted accordingly.
    ///
    /// # Syscall API
    /// ```c
    /// ssize_t read(int fd, void *buf, size_t count);
    /// ```
    /// - `fd`: File descriptor of the file to read from.
    /// - `buf`: Buffer to store the data read from the file.
    /// - `count`: Number of bytes to read.
    ///
    /// Returns the actual number of bytes read.
    pub fn read(&mut self, abi: &SyscallAbi) -> Result<usize, KernelError> {
        todo!()
    }

    /// Writes data to an open file.
    ///
    /// This function implements the system call for writing data to a file. It
    /// writes a specified number of bytes to the file, starting from the
    /// current file position. The file's state is updated accordingly.
    ///
    /// # Syscall API
    /// ```c
    /// ssize_t write(int fd, const void *buf, size_t count);
    /// ```
    /// - `fd`: File descriptor of the file to write to.
    /// - `buf`: Buffer containing the data to be written.
    /// - `count`: Number of bytes to write.
    ///
    /// Returns the number of bytes written
    pub fn write(&mut self, abi: &SyscallAbi) -> Result<usize, KernelError> {
        todo!()
    }

    /// Seeks to a new position in the file.
    ///
    /// This function implements the system call for moving the file pointer to
    /// a specified position within the file. The position can be set
    /// relative to the beginning, current position, or end of the file.
    ///
    /// # Syscall API
    /// ```c
    /// off_t seek(int fd, off_t offset, int whence);
    /// ```
    /// - `fd`: File descriptor of the file to seek in.
    /// - `offset`: Number of bytes to move the file pointer.
    /// - `whence`: Specifies how the offset is to be interpreted. Common values
    ///   are:
    ///   - `SEEK_SET` (0): The offset is relative to the beginning of the file.
    ///   - `SEEK_CUR` (1): The offset is relative to the current file position.
    ///   - `SEEK_END` (2): The offset is relative to the end of the file.
    ///
    /// Returns the new position of the file descriptor after moving it.
    pub fn seek(&mut self, abi: &SyscallAbi) -> Result<usize, KernelError> {
        todo!()
    }

    /// Tells the current position in the file.
    ///
    /// This function implements the system call for retrieving the current file
    /// pointer position. It allows the program to know where in the file
    /// the next operation will occur.
    ///
    /// # Syscall API
    /// ```c
    /// off_t tell(int fd);
    /// ```
    /// - `fd`: File descriptor of the file.
    ///
    /// Returns the position of the file descriptor.
    pub fn tell(&mut self, abi: &SyscallAbi) -> Result<usize, KernelError> {
        todo!()
    }

    /// Closes an open file.
    ///
    /// This function implements the system call for closing an open file.
    ///
    /// # Syscall API
    /// ```c
    /// int close(int fd);
    /// ```
    ///  - `fd`: File descriptor to close.
    ///
    /// Returns 0 if success.
    pub fn close(&mut self, abi: &SyscallAbi) -> Result<usize, KernelError> {
        todo!()
    }

    /// Creates an interprocess communication channel between two file
    /// descriptors.
    //
    /// Pipes are unidirectional communication channels commonly used for IPC.
    /// Data written to `pipefd[1]` can be read from `pipefd[0]`.
    ///
    /// A process that read from pipe must wait if there are no bytes to be
    /// read.
    ///
    /// # Syscall API
    /// ```c
    /// int pipe(int pipefd[2]);
    /// ```
    /// - `pipefd`: An array of two file descriptors, where `pipefd[0]` is for
    ///   reading and `pipefd[1]` is for writing.
    ///
    /// Returns 0 if success.
    pub fn pipe(&mut self, abi: &SyscallAbi) -> Result<usize, KernelError> {
        todo!()
    }
}
