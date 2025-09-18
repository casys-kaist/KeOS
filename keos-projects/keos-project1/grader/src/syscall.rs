use keos::{KernelError, fs::FileSystem, syscall::flags::FileMode};
use keos_project1::{SyscallNumber, syscall::SyscallAbi};

#[derive(Default)]
pub struct SyscallAbiValidator {}
impl keos::task::Task for SyscallAbiValidator {
    fn syscall(&mut self, registers: &mut keos::syscall::Registers) {
        let abi = SyscallAbi::from_registers(registers); // Extract ABI from the registers.
        let return_val = match abi.sysno {
            0x10000 => Ok(abi.arg1),
            0x10001 => Ok(abi.arg2),
            0x10002 => Ok(abi.arg3),
            0x10003 => Ok(abi.arg4),
            0x10004 => Ok(abi.arg5),
            0x10005 => Ok(abi.arg6),
            0x10006 => Err(KernelError::InvalidArgument),
            o => Ok(o),
        };
        abi.set_return_value(return_val);
    }
}

pub fn syscall_abi() {
    assert_eq!(
        syscall!(0x1234, 0x31331, 0x31332, 0x31333, 0x31334, 0x31335, 0x31336),
        0x1234,
        "sysno != 0x1234."
    );
    assert_eq!(
        syscall!(
            0x10000, 0x31331, 0x31332, 0x31333, 0x31334, 0x31335, 0x31336
        ),
        0x31331,
        "arg1 != 0x31331."
    );
    assert_eq!(
        syscall!(
            0x10001, 0x31331, 0x31332, 0x31333, 0x31334, 0x31335, 0x31336
        ),
        0x31332,
        "arg2 != 0x31332."
    );
    assert_eq!(
        syscall!(
            0x10002, 0x31331, 0x31332, 0x31333, 0x31334, 0x31335, 0x31336
        ),
        0x31333,
        "arg3 != 0x31333."
    );
    assert_eq!(
        syscall!(
            0x10003, 0x31331, 0x31332, 0x31333, 0x31334, 0x31335, 0x31336
        ),
        0x31334,
        "arg4 != 0x31334."
    );
    assert_eq!(
        syscall!(
            0x10004, 0x31331, 0x31332, 0x31333, 0x31334, 0x31335, 0x31336
        ),
        0x31335,
        "arg5 != 0x31335."
    );
    assert_eq!(
        syscall!(
            0x10005, 0x31331, 0x31332, 0x31333, 0x31334, 0x31335, 0x31336
        ),
        0x31336,
        "arg6 != 0x31336."
    );
    assert_eq!(
        syscall!(
            0x10006, 0x31331, 0x31332, 0x31333, 0x31334, 0x31335, 0x31336
        ),
        -22,
        "retval != KernelError::InvalidArgument."
    );
}

/// Tests normal `SYS_OPEN` system call operations.
///
/// This test verifies the correct behavior of opening existing files
/// with different modes.
pub fn open_normal() {
    // Attempt to open an existing file "hello" with read mode.
    // File descriptors typically start from 3 (0, 1, and 2 are reserved for
    // standard input, output, and error).
    let fd1 = syscall!(SyscallNumber::Open as usize, c"hello".as_ptr(), 0);
    assert!(
        fd1 >= 0,
        "Opening an existing file should return a valid file descriptor."
    );

    // Attempt to open an existing file "hello" with write mode.
    // File descriptors typically start from 3 (0, 1, and 2 are reserved for
    // standard input, output, and error).
    let fd2 = syscall!(SyscallNumber::Open as usize, c"hello".as_ptr(), 1);
    assert!(
        fd2 >= 0,
        "Opening an existing file should return a valid file descriptor."
    );
    assert_ne!(fd1, fd2, "File descriptor should be different.");

    // Attempt to open an existing file "hello" with read_write mode.
    // File descriptors typically start from 3 (0, 1, and 2 are reserved for
    // standard input, output, and error).
    let fd3 = syscall!(SyscallNumber::Open as usize, c"hello".as_ptr(), 2);
    assert!(
        fd3 >= 0,
        "Opening an existing file should return a valid file descriptor."
    );
    assert_ne!(fd1, fd3, "File descriptor should be different.");
    assert_ne!(fd2, fd3, "File descriptor should be different.");
}

/// Tests invalid `SYS_OPEN` system call operations.
///
/// This test verifies error handling for invalid open operations including
/// non-existent files, invalid pointers, and invalid modes.
pub fn open_invalid() {
    // Attempt to open a non-existent file "nonexistant" with read mode.
    // It should return `KernelError::NoSuchEntry`.
    assert_eq!(
        syscall!(SyscallNumber::Open as usize, c"nonexistant".as_ptr(), 0).try_into(),
        Ok(KernelError::NoSuchEntry),
        "Opening a non-existent file should return NoSuchEntry error."
    );

    // Attempt to open a file with an invalid pointer (e.g., NULL).
    // This should return `KernelError::BadAddress`.
    assert_eq!(
        syscall!(SyscallNumber::Open as usize, core::ptr::null::<u8>(), 0).try_into(),
        Ok(KernelError::BadAddress),
        "Opening a file with a null pointer should return BadAddress error."
    );

    // Attempt to open a file with an invalid mode (e.g., an undefined file mode).
    // This should also return `KernelError::InvalidArgument`.
    assert_eq!(
        syscall!(SyscallNumber::Open as usize, c"hello".as_ptr(), 9999).try_into(),
        Ok(KernelError::InvalidArgument),
        "Opening a file with an invalid mode should return InvalidArgument error."
    );
}

/// Tests normal reading from a file using `SYS_READ`.
///
/// This test ensures that opening and reading from a file works correctly
/// for sequential read operations.
pub fn read_normal() {
    let mut buf = [0u8; 24]; // Buffer to store read data.

    // Open the file in read mode.
    let fd = syscall!(SyscallNumber::Open as usize, c"hello".as_ptr(), 0);
    assert!(
        fd >= 0,
        "File descriptor should be a valid number (>= 0) when opening a file."
    );

    // Read 24 bytes from the file and check if the return value is correct.
    assert_eq!(
        syscall!(SyscallNumber::Read as usize, fd, buf.as_mut_ptr(), 24),
        24,
        "Reading 24 bytes should return 24."
    );

    // Verify that the read content matches the expected data.
    assert_eq!(
        &buf[..24],
        b"Welcome to KeOS Project!",
        "File contents should match the expected string."
    );

    // Attempt to read another 8 bytes from the file.
    assert_eq!(
        syscall!(SyscallNumber::Read as usize, fd, buf.as_mut_ptr(), 8),
        8,
        "Reading additional 8 bytes should return 8."
    );

    // Validate that the buffer contains new data, ensuring correct file read
    // behavior.
    assert_eq!(
        &buf[..24],
        b"\n\nEven tto KeOS Project!",
        "Buffer content should match expected data after second read."
    );
}

/// Tests reading beyond file size using `SYS_READ`.
///
/// This test verifies behavior when reading more data than available
/// and handling EOF correctly.
pub fn read_truncate() {
    let mut buf = [0u8; 180]; // Buffer to store read data.

    // Open the file in read mode.
    let fd = syscall!(SyscallNumber::Open as usize, c"hello".as_ptr(), 0);
    assert!(
        fd >= 0,
        "File descriptor should be a valid number (>= 0) when opening a file."
    );

    // Attempt to read 180 bytes, but only 140 bytes are available in the file.
    // The syscall should return 140, indicating the number of bytes actually read.
    assert_eq!(
        syscall!(SyscallNumber::Read as usize, fd, buf.as_mut_ptr(), 180),
        140,
        "Reading beyond the available data should return the remaining bytes (140), not EOF immediately."
    );

    // Attempt to read when no more data should be available.
    assert_eq!(
        syscall!(SyscallNumber::Read as usize, fd, buf.as_mut_ptr(), 180),
        0,
        "Reading beyond the available data should return 0 (EOF)."
    );
}

/// Tests read error with invalid file descriptor.
pub fn read_error_bad_fd() {
    let mut buf = [0u8; 24];

    // Attempt to read using an invalid file descriptor (-1).
    // This should return the error `BadFileDescriptor`.
    assert_eq!(
        syscall!(SyscallNumber::Read as usize, -1, buf.as_mut_ptr(), 10).try_into(),
        Ok(KernelError::BadFileDescriptor),
        "Reading with an invalid file descriptor should return an error."
    );
}

/// Tests read error when reading from write-only file.
pub fn read_error_bad_mode() {
    let mut buf = [0u8; 24];

    // Open the file "hello" in write-only mode (mode = 1).
    let fd = syscall!(SyscallNumber::Open as usize, c"hello".as_ptr(), 1);
    assert!(
        fd >= 0,
        "File descriptor should be a valid number (>= 0) when opening a file."
    );

    // Attempt to read from a file that was opened in write-only mode.
    // This should return the error `InvalidArgument`, as reading is not permitted.
    assert_eq!(
        syscall!(SyscallNumber::Read as usize, fd, buf.as_mut_ptr(), 24).try_into(),
        Ok(KernelError::InvalidArgument),
        "Reading from a write-only file should return an InvalidArgument error."
    );
}

/// Tests read error with null buffer pointer.
pub fn read_error_bad_address() {
    // Open the file "hello" in read-only mode (mode = 0).
    let fd = syscall!(SyscallNumber::Open as usize, c"hello".as_ptr(), 0);
    assert!(
        fd >= 0,
        "File descriptor should be a valid number (>= 0) when opening a file."
    );

    // Attempt to read using a null buffer pointer.
    // This should return `BadAddress` since the buffer must be valid.
    assert_eq!(
        syscall!(
            SyscallNumber::Read as usize,
            fd,
            core::ptr::null_mut::<u8>(),
            10
        )
        .try_into(),
        Ok(KernelError::BadAddress),
        "Reading with a null buffer should return an BadAddress error."
    );
}

/// Tests normal writing to a file using `SYS_WRITE`.
///
/// This test verifies basic writing functionality including writing data
/// and reading it back to confirm the write operation.
pub fn write_normal() {
    let mut buf = [0u8; 24];

    // Open the file in read-write mode.
    let fd = syscall!(SyscallNumber::Open as usize, c"hello2".as_ptr(), 2);
    assert!(fd >= 0, "File descriptor should be a valid number (>= 0).");

    // Read the first 7 bytes and verify the initial content.
    assert_eq!(
        syscall!(SyscallNumber::Read as usize, fd, buf.as_mut_ptr(), 7),
        7
    );
    assert_eq!(
        &buf[0..7],
        b"Welcome",
        "Initial file content should be 'Welcome'."
    );

    // Seek to the beginning of the file.
    assert_eq!(syscall!(SyscallNumber::Seek as usize, fd, 0, 0), 0);

    // Write "Testing" to the file.
    assert_eq!(
        syscall!(SyscallNumber::Write as usize, fd, "Testing".as_ptr(), 7),
        7,
        "Writing 7 bytes should return 7."
    );

    // Seek back to the beginning to read what we wrote.
    assert_eq!(syscall!(SyscallNumber::Seek as usize, fd, 0, 0), 0);

    // Read the data back and verify it matches what we wrote.
    assert_eq!(
        syscall!(SyscallNumber::Read as usize, fd, buf.as_mut_ptr(), 7),
        7
    );
    assert_eq!(
        &buf[0..7],
        b"Testing",
        "File content should match what was written."
    );

    // Seek back to the beginning to read what we wrote.
    assert_eq!(syscall!(SyscallNumber::Seek as usize, fd, 0, 0), 0);

    // Write "Welcome" to the file.
    assert_eq!(
        syscall!(SyscallNumber::Write as usize, fd, "Welcome".as_ptr(), 7),
        7,
        "Writing 7 bytes should return 7."
    );

    // Close the file descriptor.
    assert_eq!(syscall!(SyscallNumber::Close as usize, fd), 0);
}

/// Tests writing and data synchronization across multiple file descriptors.
///
/// This test verifies writing behavior including overwriting existing data
/// and ensuring data consistency across multiple file descriptors.
pub fn write_sync() {
    let mut buf = [0u8; 24];

    // Open the file in read-write mode.
    let fd1 = syscall!(SyscallNumber::Open as usize, c"hello2".as_ptr(), 2);
    assert!(fd1 >= 0, "File descriptor should be a valid number (>= 0).");

    // Read the first 7 bytes and verify the initial content.
    assert_eq!(
        syscall!(SyscallNumber::Read as usize, fd1, buf.as_mut_ptr(), 7),
        7
    );
    assert_eq!(
        &buf[0..7],
        b"Welcome",
        "Initial file content should be 'Welcome'."
    );

    // Seek to the beginning of the file.
    assert_eq!(syscall!(SyscallNumber::Seek as usize, fd1, 0, 0), 0);

    // Open the same file again, obtaining a second file descriptor.
    let fd2 = syscall!(SyscallNumber::Open as usize, c"hello2".as_ptr(), 0);
    assert!(fd2 >= 0, "Second file descriptor should be valid.");

    // Write "Awesome" using the first file descriptor.
    assert_eq!(
        syscall!(SyscallNumber::Write as usize, fd1, "Awesome".as_ptr(), 7),
        7
    );

    // Read from the second file descriptor without seeking to ensure changes are
    // reflected.
    assert_eq!(syscall!(SyscallNumber::Seek as usize, fd2, 0, 0), 0);
    assert_eq!(
        syscall!(SyscallNumber::Read as usize, fd2, buf.as_mut_ptr(), 7),
        7
    );
    assert_eq!(
        &buf[0..7],
        b"Awesome",
        "File content should be 'Awesome' when read from the second FD."
    );

    // Close both file descriptors.
    assert_eq!(syscall!(SyscallNumber::Close as usize, fd1), 0);
    assert_eq!(syscall!(SyscallNumber::Close as usize, fd2), 0);
}

/// Tests data persistence after write, close, reopen, and read operations.
///
/// This test verifies that written data persists across file close/reopen
/// cycles.
pub fn write_persistence() {
    let mut buf = [0u8; 24];

    // Open the file in read-write mode.
    let fd = syscall!(SyscallNumber::Open as usize, c"hello3".as_ptr(), 2);
    assert!(fd >= 0, "File descriptor should be a valid number (>= 0).");

    // Read the first 7 bytes and verify the initial content.
    assert_eq!(
        syscall!(SyscallNumber::Read as usize, fd, buf.as_mut_ptr(), 7),
        7
    );
    assert_eq!(
        &buf[0..7],
        b"Welcome",
        "Initial file content should be 'Welcome'."
    );

    // Seek to the beginning of the file.
    assert_eq!(syscall!(SyscallNumber::Seek as usize, fd, 0, 0), 0);

    // Write "Testing" to the file.
    assert_eq!(
        syscall!(SyscallNumber::Write as usize, fd, "Awesome".as_ptr(), 7),
        7,
        "Writing 7 bytes should return 7."
    );

    // Close the file descriptor.
    assert_eq!(syscall!(SyscallNumber::Close as usize, fd), 0);

    // Reopen the file in read-write mode to ensure persistence.
    let fd = syscall!(SyscallNumber::Open as usize, c"hello3".as_ptr(), 2);
    assert!(fd >= 0, "Reopened file descriptor should be valid.");

    // Read the first 7 bytes again and ensure they match the updated content.
    assert_eq!(
        syscall!(SyscallNumber::Read as usize, fd, buf.as_mut_ptr(), 7),
        7
    );
    assert_eq!(
        &buf[0..7],
        b"Awesome",
        "File content should persist after reopening."
    );

    // Seek to the beginning of the file again.
    assert_eq!(syscall!(SyscallNumber::Seek as usize, fd, 0, 0), 0);

    // Write using a buffer.
    assert_eq!(
        syscall!(SyscallNumber::Write as usize, fd, c"Welcome".as_ptr(), 7),
        7
    );

    // Close the reopened file descriptor.
    assert_eq!(syscall!(SyscallNumber::Close as usize, fd), 0);
}

/// Tests write error with invalid file descriptor.
pub fn write_error_bad_fd() {
    let mut buf = [0u8; 24];

    // Attempt to write using an invalid file descriptor (-1).
    // This should return the error `BadFileDescriptor`.
    assert_eq!(
        syscall!(SyscallNumber::Write as usize, -1, buf.as_mut_ptr(), 10).try_into(),
        Ok(KernelError::BadFileDescriptor),
        "Writing to an invalid file descriptor should return an error."
    );
}

/// Tests write error when writing to read-only file.
pub fn write_error_bad_mode() {
    let mut buf = [0u8; 24];

    // Open the file "hello" in read-only mode (mode = 0).
    let fd = syscall!(SyscallNumber::Open as usize, c"hello".as_ptr(), 0);
    assert!(
        fd >= 0,
        "File descriptor should be a valid number (>= 0) when opening a file."
    );

    // Attempt to write to a file that was opened in read-only mode.
    // This should return the error `InvalidArgument`, as writing is not permitted.
    assert_eq!(
        syscall!(SyscallNumber::Write as usize, fd, buf.as_mut_ptr(), 24).try_into(),
        Ok(KernelError::InvalidArgument),
        "Writing to a read-only file should return an InvalidArgument error."
    );
}

/// Tests write error with null buffer pointer.
pub fn write_error_bad_address() {
    // Open the file "hello" in write-only mode (mode = 1).
    let fd = syscall!(SyscallNumber::Open as usize, c"hello".as_ptr(), 1);
    assert!(
        fd >= 0,
        "File descriptor should be a valid number (>= 0) when opening a file."
    );

    // Attempt to write using a null buffer pointer.
    // This should return `BadAddress` since the buffer must be valid.
    assert_eq!(
        syscall!(
            SyscallNumber::Write as usize,
            fd,
            core::ptr::null_mut::<u8>(),
            10
        )
        .try_into(),
        Ok(KernelError::BadAddress),
        "Writing with a null buffer should return an BadAddress error."
    );
}

/// Tests seeking to the beginning of a file using `SYS_SEEK`.
///
/// This test verifies seeking to the start of a file and reading from the
/// beginning.
pub fn seek_begin() {
    let mut buf = [0u8; 24];

    // Open the file "hello" in read mode.
    let fd = syscall!(
        SyscallNumber::Open as usize,
        c"hello".as_ptr(),
        FileMode::Read as usize
    );
    assert!(fd >= 0, "File descriptor should be valid (>= 0).");

    // Seek to the beginning of the file (offset = 0, from start).
    assert_eq!(
        syscall!(SyscallNumber::Seek as usize, fd, 0, 0),
        0,
        "Seeking to the beginning should return 0."
    );

    // Read the first 24 bytes and verify the contents.
    assert_eq!(
        syscall!(SyscallNumber::Read as usize, fd, buf.as_mut_ptr(), 24),
        24
    );
    assert_eq!(
        &buf, b"Welcome to KeOS Project!",
        "File content mismatch after seeking to start."
    );

    // Close the file.
    assert_eq!(
        syscall!(SyscallNumber::Close as usize, fd),
        0,
        "Closing the file should return 0."
    );
}

/// Tests seeking relative to current position using `SYS_SEEK`.
///
/// This test verifies seeking forward and backward from the current position.
pub fn seek_current() {
    let mut buf = [0u8; 24];

    // Open the file "hello" in read mode.
    let fd = syscall!(
        SyscallNumber::Open as usize,
        c"hello".as_ptr(),
        FileMode::Read as usize
    );
    assert!(fd >= 0, "File descriptor should be valid (>= 0).");

    // Read 24 bytes to advance position
    assert_eq!(
        syscall!(SyscallNumber::Read as usize, fd, buf.as_mut_ptr(), 24),
        24
    );

    // Seek to the current.
    assert_eq!(
        syscall!(SyscallNumber::Seek as usize, fd, 0, 1),
        24,
        "Seeking to the current with offset 0 should hold the current offset."
    );

    // Seek 13 bytes backward from the current position.
    assert_eq!(
        syscall!(SyscallNumber::Seek as usize, fd, -13isize, 1),
        11,
        "Seeking backward by 13 bytes should move to offset 11."
    );

    // Read the next 4 bytes after seeking backward.
    assert_eq!(
        syscall!(SyscallNumber::Read as usize, fd, buf.as_mut_ptr(), 4),
        4
    );
    assert_eq!(
        &buf, b"KeOSome to KeOS Project!",
        "File content mismatch after seeking backwards."
    );

    // Close the file.
    assert_eq!(
        syscall!(SyscallNumber::Close as usize, fd),
        0,
        "Closing the file should return 0."
    );
}

/// Tests seeking relative to end of file using `SYS_SEEK`.
///
/// This test verifies seeking from the end of the file.
pub fn seek_end() {
    // Open the file "hello" in read mode.
    let fd = syscall!(
        SyscallNumber::Open as usize,
        c"hello".as_ptr(),
        FileMode::Read as usize
    );
    assert!(fd >= 0, "File descriptor should be valid (>= 0).");

    // Seek 16 bytes backward from the end of the file.
    let size = syscall!(SyscallNumber::Seek as usize, fd, -0x10isize, 2);

    // Verify that the seek operation was correct by checking the file size.
    let keos::fs::File::RegularFile(file) = FileSystem::root().open("hello").unwrap() else {
        panic!("Invalid test configuration: `hello` is not a regular file.");
    };
    assert_eq!(
        file.size() - 0x10,
        size as usize,
        "Seeking 16 bytes backward from the end should match expected file size."
    );

    // Close the file.
    assert_eq!(
        syscall!(SyscallNumber::Close as usize, fd),
        0,
        "Closing the file should return 0."
    );
}

/// Tests seeking beyond EOF using `SYS_SEEK`.
///
/// This test verifies behavior when seeking beyond the end of file.
pub fn seek_beyond_eof() {
    // Open the file "hello" in read mode.
    let fd = syscall!(
        SyscallNumber::Open as usize,
        c"hello".as_ptr(),
        FileMode::Read as usize
    );
    assert!(fd >= 0, "File descriptor should be valid (>= 0).");

    // Seeking beyond the EOF
    assert_eq!(
        syscall!(SyscallNumber::Seek as usize, fd, 3000isize, 0),
        3000,
        "Seeking beyond EOF should return the requested offset"
    );

    let buf = [0u8; 180]; // Buffer to store read data.

    // reading after seeking beyond EOF
    assert_eq!(
        syscall!(SyscallNumber::Read as usize, fd, buf.as_ptr(), 10),
        0,
        "Reading after seeking beyond EOF should read 0 bytes"
    );

    // Close the file.
    assert_eq!(
        syscall!(SyscallNumber::Close as usize, fd),
        0,
        "Closing the file should return 0."
    );
}

/// Tests seek error on standard I/O streams.
pub fn seek_error_stdio() {
    // Attempt to seek using stdin (fd = 0).
    assert_eq!(
        syscall!(SyscallNumber::Seek as usize, 0, 0, 0).try_into(),
        Ok(KernelError::InvalidArgument),
        "Seeking on stdin should return InvalidArgument."
    );

    // Attempt to seek using stdout (fd = 1).
    assert_eq!(
        syscall!(SyscallNumber::Seek as usize, 1, 0, 0).try_into(),
        Ok(KernelError::InvalidArgument),
        "Seeking on stdout should return InvalidArgument."
    );

    // Attempt to seek using stderr (fd = 2).
    assert_eq!(
        syscall!(SyscallNumber::Seek as usize, 2, 0, 0).try_into(),
        Ok(KernelError::InvalidArgument),
        "Seeking on stderr should return InvalidArgument."
    );
}

/// Tests seek error with invalid file descriptor.
pub fn seek_error_bad_fd() {
    // Attempt to seek using an invalid file descriptor (-1).
    assert_eq!(
        syscall!(SyscallNumber::Seek as usize, -1, 0, 0).try_into(),
        Ok(KernelError::BadFileDescriptor),
        "Seeking with an invalid file descriptor should return BadFileDescriptor."
    );

    // Attempt to seek using an invalid file descriptor (-1).
    assert_eq!(
        syscall!(SyscallNumber::Seek as usize, -1, 0, 1).try_into(),
        Ok(KernelError::BadFileDescriptor),
        "Seeking with an invalid file descriptor should return BadFileDescriptor."
    );

    // Attempt to seek using an invalid file descriptor (-1).
    assert_eq!(
        syscall!(SyscallNumber::Seek as usize, -1, 0, 2).try_into(),
        Ok(KernelError::BadFileDescriptor),
        "Seeking with an invalid file descriptor should return BadFileDescriptor."
    );
}

/// Tests seek error with invalid whence parameter.
pub fn seek_error_bad_whence() {
    // Open the file "hello" in read mode.
    let fd = syscall!(
        SyscallNumber::Open as usize,
        c"hello".as_ptr(),
        FileMode::Read as usize
    );
    assert!(fd >= 0, "File descriptor should be valid (>= 0).");

    // Attempt to seek using an invalid whence argument (arg3 = 3).
    assert_eq!(
        syscall!(SyscallNumber::Seek as usize, fd, 0, 3).try_into(),
        Ok(KernelError::InvalidArgument),
        "Seeking with an invalid whence argument (not 0, 1, or 2) should return InvalidArgument."
    );
}

/// Tests the `SYS_TELL` system call for reporting file offset after basic
/// operations.
///
/// This test validates basic tell operations including reading and seeking.
pub fn tell_basic() {
    let mut buf = [0u8; 24];

    // Open the file in read/write mode.
    let fd = syscall!(SyscallNumber::Open as usize, c"hello4".as_ptr(), 2);
    assert!(fd >= 0, "File descriptor should be a valid number (>= 0).");

    // The initial file offset should be 0.
    assert_eq!(
        syscall!(SyscallNumber::Tell as usize, fd),
        0,
        "Newly opened file should have an offset of 0."
    );

    // Read 7 bytes from the file.
    assert_eq!(
        syscall!(SyscallNumber::Read as usize, fd, buf.as_mut_ptr(), 7),
        7
    );
    assert_eq!(
        &buf[0..7],
        b"Welcome",
        "Read content should match expected file data."
    );

    // File offset should now be at 7.
    assert_eq!(
        syscall!(SyscallNumber::Tell as usize, fd),
        7,
        "File offset should be 7 after reading 7 bytes."
    );

    // Seek back to position 0.
    assert_eq!(syscall!(SyscallNumber::Seek as usize, fd, 0, 0), 0);
    assert_eq!(
        syscall!(SyscallNumber::Tell as usize, fd),
        0,
        "File offset should be 0 after seeking back to the start."
    );
}

/// Tests the `SYS_TELL` system call for reporting file offset after write
/// operations.
///
/// This test validates tell operations after writing and seeking.
pub fn tell_write() {
    // Open the file in read/write mode.
    let fd = syscall!(SyscallNumber::Open as usize, c"hello4".as_ptr(), 2);
    assert!(fd >= 0, "File descriptor should be a valid number (>= 0).");

    // Write new data to the file.
    assert_eq!(
        syscall!(SyscallNumber::Write as usize, fd, b"Awesome".as_ptr(), 7),
        7
    );

    // The file offset should now be at 7 after writing.
    assert_eq!(
        syscall!(SyscallNumber::Tell as usize, fd),
        7,
        "File offset should be 7 after writing 7 bytes."
    );

    // Seek to the end of the file.
    assert_eq!(
        syscall!(SyscallNumber::Seek as usize, fd, 0, 2),
        140,
        "Seeking to end should return an 140.",
    );
    assert_eq!(
        syscall!(SyscallNumber::Tell as usize, fd),
        140,
        "File offset should be at end of file after seeking."
    );
}

/// Tests tell error on standard I/O streams.
pub fn tell_error_stdio() {
    // `SYS_TELL` should return `InvalidArgument` for stdin (fd=0).
    assert_eq!(
        syscall!(SyscallNumber::Tell as usize, 0).try_into(),
        Ok(KernelError::InvalidArgument),
        "SYS_TELL on stdin (fd=0) should return InvalidArgument."
    );

    // `SYS_TELL` should return `InvalidArgument` for stdout (fd=1).
    assert_eq!(
        syscall!(SyscallNumber::Tell as usize, 1).try_into(),
        Ok(KernelError::InvalidArgument),
        "SYS_TELL on stdout (fd=1) should return InvalidArgument."
    );

    // `SYS_TELL` should return `InvalidArgument` for stderr (fd=2).
    assert_eq!(
        syscall!(SyscallNumber::Tell as usize, 2).try_into(),
        Ok(KernelError::InvalidArgument),
        "SYS_TELL on stderr (fd=2) should return InvalidArgument."
    );
}

/// Tests tell error with invalid file descriptor.
pub fn tell_error_bad_fd() {
    // `SYS_TELL` should return `BadFileDescriptor` for an invalid file
    // descriptor (e.g., 3123).
    assert_eq!(
        syscall!(SyscallNumber::Tell as usize, 3123).try_into(),
        Ok(KernelError::BadFileDescriptor),
        "SYS_TELL on an invalid file descriptor should return BadFileDescriptor."
    );
}

/// Test case for normal standard input operations.
///
/// This test verifies that reading from stdin works correctly, including:
/// - Successfully reading a predefined string.
/// - Handling end-of-input correctly.
/// - Ensuring the buffer remains unchanged when no data is read.
#[stdin(b"KeOS is fun!")]
#[assert_output(b"")]
pub fn stdio_normal() {
    // Allocate a buffer of 12 bytes initialized to zero.
    let mut buf = [0u8; 12];

    // Attempt to read 12 bytes from stdin (file descriptor 0).
    // This should return 12 bytes and populate `buf` with the expected string.
    assert_eq!(
        syscall!(SyscallNumber::Read as usize, 0, buf.as_mut_ptr(), 12),
        12
    );

    // Verify that the buffer contains the expected string "KeOS is fun!".
    assert_eq!(&buf, b"KeOS is fun!");

    // Attempt to read using an invalid pointer (e.g., NULL).
    // This should return `KernelError::BadAddress`.
    assert_eq!(
        syscall!(SyscallNumber::Read as usize, 0, core::ptr::null::<u8>(), 12).try_into(),
        Ok(KernelError::BadAddress),
        "Reading from stdin with a null pointer should return BadAddress error."
    );

    // Attempt to write to standard input (fd = 0).
    // This should return `KernelError::InvalidArgument`.
    assert_eq!(
        syscall!(SyscallNumber::Write as usize, 0, [0u8; 12].as_ptr(), 12).try_into(),
        Ok(KernelError::InvalidArgument),
        "Writing to stdin should return InvalidArgument error."
    );

    // Fill the buffer with 0xff to check for any unexpected changes in later reads.
    buf.fill(0xff);

    // Attempt to read 8 more bytes from stdin.
    // Since all input has already been consumed, this should return 0 (indicating
    // EOF).
    assert_eq!(
        syscall!(SyscallNumber::Read as usize, 0, buf.as_mut_ptr(), 8),
        0
    );

    // Verify that the buffer remains unchanged after the failed read.
    assert_eq!(&buf, &[0xff; 12]);
}

/// Test case for partial reads from standard input.
///
/// This test verifies that reading data from standard input (stdin) in chunks
/// works correctly. It ensures that:
/// - Partial reads correctly retrieve the requested number of bytes.
/// - Consecutive reads continue from where the last read left off.
/// - EOF is correctly handled when no more data is available.
#[stdin(b"Hello, World")]
#[assert_output(b"")]
pub fn stdio_partial() {
    // Allocate a small buffer to read only part of the input.
    let mut small_buf = [0u8; 5];

    // Read the first 5 bytes, expecting "Hello".
    assert_eq!(
        syscall!(SyscallNumber::Read as usize, 0, small_buf.as_mut_ptr(), 5),
        5
    );
    assert_eq!(&small_buf, b"Hello");

    // Allocate a new buffer to read the next portion of the input.
    let mut next_buf = [0u8; 7];

    // Read the next 7 bytes, expecting ", World".
    assert_eq!(
        syscall!(SyscallNumber::Read as usize, 0, next_buf.as_mut_ptr(), 7),
        7
    );
    assert_eq!(&next_buf, b", World");

    // Attempt to read beyond the available data.
    // Since "Hello, World!" is 13 bytes long, this should return 0 (EOF).
    assert_eq!(
        syscall!(SyscallNumber::Read as usize, 0, small_buf.as_mut_ptr(), 4),
        0
    );
}

/// Tests normal writing to standard output.
///
/// This test verifies that writing to `stdout` (file descriptor 1) correctly
/// outputs the expected data.
#[stdin(b"")]
#[assert_output(b"Hello, keos!")]
pub fn stdout_normal() {
    // Attempt to read from standard input (fd = 1).
    // This should return `KernelError::InvalidArgument`.
    assert_eq!(
        syscall!(SyscallNumber::Read as usize, 1, [0u8; 12].as_ptr(), 12).try_into(),
        Ok(KernelError::InvalidArgument),
        "Reading from stderr should return InvalidArgument error."
    );

    // Write "Hello, keos!" to standard output (fd = 1).
    // The return value should be equal to the number of bytes written (12).
    assert_eq!(
        syscall!(
            SyscallNumber::Write as usize,
            1,
            "Hello, keos!".as_ptr(),
            12
        ),
        12,
        "Writing to stdout should return the number of bytes written."
    );
}

/// Tests writing empty string to standard output.
///
/// This test verifies that writing an empty string to stdout works correctly.
#[stdin(b"")]
#[assert_output(b"")]
pub fn stdout_empty() {
    // Attempt to write an empty string to stdout, which should succeed and return
    // 0.
    assert_eq!(
        syscall!(SyscallNumber::Write as usize, 1, "".as_ptr(), 0),
        0,
        "Writing an empty string to stdout should return 0."
    );
}

/// Tests invalid operations on standard output.
///
/// This test verifies error handling for invalid stdout operations.
#[stdin(b"")]
#[assert_output(b"")]
pub fn stdout_invalid() {
    // Attempt to write using an invalid pointer (e.g., NULL).
    // This should return `KernelError::BadAddress`.
    assert_eq!(
        syscall!(
            SyscallNumber::Write as usize,
            1,
            core::ptr::null::<u8>(),
            12
        )
        .try_into(),
        Ok(KernelError::BadAddress),
        "Writing to stdout with a null pointer should return BadAddress error."
    );
}

/// Tests normal writing to standard error.
///
/// This test verifies that writing to `stderr` (file descriptor 2) correctly
/// outputs the expected data.
#[stdin(b"")]
#[assert_output(b"Hello, keos!")]
pub fn stderr_normal() {
    // Attempt to read from standard error (fd = 2).
    // This should return `KernelError::InvalidArgument`.
    assert_eq!(
        syscall!(SyscallNumber::Read as usize, 2, [0u8; 12].as_ptr(), 12).try_into(),
        Ok(KernelError::InvalidArgument),
        "Reading from stderr should return InvalidArgument error."
    );

    // Write "Hello, keos!" to standard error (fd = 2).
    // The return value should be equal to the number of bytes written (12).
    assert_eq!(
        syscall!(
            SyscallNumber::Write as usize,
            2,
            "Hello, keos!".as_ptr(),
            12
        ),
        12,
        "Writing to stderr should return the number of bytes written."
    );
}

/// Tests writing empty string to standard error.
///
/// This test verifies that writing an empty string to stderr works correctly.
#[stdin(b"")]
#[assert_output(b"")]
pub fn stderr_empty() {
    // Attempt to write an empty string to stderr, which should succeed and return
    // 0.
    assert_eq!(
        syscall!(SyscallNumber::Write as usize, 2, "".as_ptr(), 0),
        0,
        "Writing an empty string to stderr should return 0."
    );
}

/// Tests invalid operations on standard error.
///
/// This test verifies error handling for invalid stderr operations.
#[stdin(b"")]
#[assert_output(b"")]
pub fn stderr_invalid() {
    // Attempt to write using an invalid pointer (e.g., NULL).
    // This should return `KernelError::BadAddress`.
    assert_eq!(
        syscall!(
            SyscallNumber::Write as usize,
            2,
            core::ptr::null::<u8>(),
            12
        )
        .try_into(),
        Ok(KernelError::BadAddress),
        "Writing to stderr with a null pointer should return BadAddress error."
    );
}

/// Tests the `SYS_CLOSE` system call.
///
/// This test ensures that closing a file descriptor:
/// - Successfully closes an open file.
/// - Prevents further read/write operations on the closed descriptor.
/// - Returns an error when trying to close an invalid or already closed
///   descriptor.
/// - Handles closing standard input (stdin), output (stdout), and error
///   (stderr) properly.
#[stdin(b"Awesome or not?")]
#[assert_output(b"Awesome")]
pub fn close() {
    let mut buf = [0u8; 24];

    // Open a file with write access.
    let fd = syscall!(SyscallNumber::Open as usize, c"hello".as_ptr(), 2);
    assert!(fd >= 0, "File descriptor should be a valid number (>= 0).");

    // Close the file descriptor and ensure it succeeds.
    assert_eq!(
        syscall!(SyscallNumber::Close as usize, fd),
        0,
        "Closing an open file descriptor should return success."
    );

    // Attempt to read from the closed file descriptor, which should fail.
    assert_eq!(
        syscall!(SyscallNumber::Read as usize, fd, buf.as_mut_ptr(), 24).try_into(),
        Ok(KernelError::BadFileDescriptor),
        "Reading from a closed file descriptor should return BadFileDescriptor."
    );

    // Attempt to write to the closed file descriptor, which should fail.
    assert_eq!(
        syscall!(SyscallNumber::Write as usize, fd, buf.as_mut_ptr(), 24).try_into(),
        Ok(KernelError::BadFileDescriptor),
        "Writing to a closed file descriptor should return BadFileDescriptor."
    );

    // Attempt to close an invalid file descriptor (e.g., 9222), which should fail.
    assert_eq!(
        syscall!(SyscallNumber::Close as usize, 9222).try_into(),
        Ok(KernelError::BadFileDescriptor),
        "Closing an invalid file descriptor should return BadFileDescriptor."
    );

    // Read 7 bytes from stdin before closing it.
    assert_eq!(
        syscall!(SyscallNumber::Read as usize, 0, buf.as_mut_ptr(), 7),
        7
    );

    // Close stdin and ensure it succeeds.
    assert_eq!(
        syscall!(SyscallNumber::Close as usize, 0),
        0,
        "Closing stdin should return success."
    );

    // Attempt to read from closed stdin, which should fail.
    assert_eq!(
        syscall!(SyscallNumber::Read as usize, 0, buf.as_mut_ptr(), 7).try_into(),
        Ok(KernelError::BadFileDescriptor),
        "Reading from a closed stdin should return BadFileDescriptor."
    );

    // Close stdout and ensure it succeeds.
    assert_eq!(
        syscall!(SyscallNumber::Close as usize, 1),
        0,
        "Closing stdout should return success."
    );

    // Attempt to write to closed stdout, which should fail.
    assert_eq!(
        syscall!(SyscallNumber::Write as usize, 1, buf.as_ptr(), 7).try_into(),
        Ok(KernelError::BadFileDescriptor),
        "Writing to a closed stdout should return BadFileDescriptor."
    );

    // Write to stderr before closing it (should succeed).
    assert_eq!(
        syscall!(SyscallNumber::Write as usize, 2, buf.as_ptr(), 7),
        7,
        "Writing to stderr before closing should succeed."
    );

    // Close stderr and ensure it succeeds.
    assert_eq!(
        syscall!(SyscallNumber::Close as usize, 2),
        0,
        "Closing stderr should return success."
    );

    // Attempt to write to closed stderr, which should fail.
    assert_eq!(
        syscall!(SyscallNumber::Write as usize, 2, buf.as_ptr(), 7).try_into(),
        Ok(KernelError::BadFileDescriptor),
        "Writing to a closed stderr should return BadFileDescriptor."
    );
}

/// Tests normal pipe operations.
///
/// This test verifies basic pipe functionality including creating a pipe,
/// writing to it, and reading from it.
pub fn pipe_normal() {
    let mut fds = [0i32; 2];
    let mut buf = [0u8; 12];

    assert_eq!(
        syscall!(SyscallNumber::Pipe as usize, fds.as_mut_ptr()),
        0,
        "Creating a pipe should return success."
    );

    assert!(
        fds[0] >= 0,
        "File descriptor 0 should be a valid number (>= 0)."
    );
    assert!(
        fds[1] >= 0,
        "File descriptor 1 should be a valid number (>= 0)."
    );
    assert_ne!(
        fds[0], fds[1],
        "File descriptor 1 must not be same with File Descriptor 2."
    );

    assert_eq!(
        syscall!(
            SyscallNumber::Write as usize,
            fds[1],
            c"Hello, keos!".as_ptr(),
            12
        ),
        12,
        "Writing to the tx fd should return success."
    );

    assert_eq!(
        syscall!(SyscallNumber::Read as usize, fds[0], buf.as_mut_ptr(), 12),
        12,
        "Reading from the rx fd should return success."
    );

    assert_eq!(
        &buf[..12],
        b"Hello, keos!",
        "File content mismatch to what was written to tx fd."
    );
}

/// Tests partial pipe operations with closing and broken pipe handling.
///
/// This test verifies pipe behavior when the write end is closed and
/// data is read in chunks.
pub fn pipe_partial() {
    let mut fds = [0i32; 2];
    let mut buf = [0u8; 12];

    assert_eq!(
        syscall!(SyscallNumber::Pipe as usize, fds.as_mut_ptr()),
        0,
        "Creating a pipe should return success."
    );

    assert!(
        fds[0] >= 0,
        "File descriptor 0 should be a valid number (>= 0)."
    );
    assert!(
        fds[1] >= 0,
        "File descriptor 1 should be a valid number (>= 0)."
    );
    assert_ne!(
        fds[0], fds[1],
        "File descriptor 1 must not be same with File Descriptor 2."
    );

    assert_eq!(
        syscall!(
            SyscallNumber::Write as usize,
            fds[1],
            c"Hello, keos!".as_ptr(),
            12
        ),
        12,
        "Writing to the tx fd should return success."
    );

    assert_eq!(
        syscall!(SyscallNumber::Close as usize, fds[1]),
        0,
        "Closing the tx should return success.",
    );
    assert_eq!(
        syscall!(SyscallNumber::Read as usize, fds[0], buf.as_mut_ptr(), 7),
        7,
        "Reading from the rx fd should return success."
    );

    assert_eq!(
        &buf[..7],
        b"Hello, ",
        "File content mismatch to what was written to tx fd."
    );
    assert_eq!(
        syscall!(SyscallNumber::Read as usize, fds[0], buf.as_mut_ptr(), 7),
        5,
        "Reading from the rx fd should return success."
    );
    assert_eq!(
        &buf[..5],
        b"keos!",
        "File content mismatch to what was written to tx fd."
    );
    assert_eq!(
        syscall!(SyscallNumber::Read as usize, fds[0], buf.as_mut_ptr(), 12).try_into(),
        Ok(KernelError::BrokenPipe),
        "Reading beyond the available data after closing the tx should return BrokenPipe Error."
    );
}

/// Tests pipe error with invalid directions.
pub fn pipe_error_bad_direction() {
    let mut fds = [0i32; 2];
    let mut buf = [0u8; 12];

    assert_eq!(
        syscall!(SyscallNumber::Pipe as usize, fds.as_mut_ptr()),
        0,
        "Creating a pipe should return success."
    );

    assert!(
        fds[0] >= 0,
        "File descriptor 0 should be a valid number (>= 0)."
    );
    assert!(
        fds[1] >= 0,
        "File descriptor 1 should be a valid number (>= 0)."
    );
    assert_ne!(
        fds[0], fds[1],
        "File descriptor 1 must not be same with File Descriptor 2."
    );
    assert_eq!(
        syscall!(SyscallNumber::Write as usize, fds[0], buf.as_mut_ptr(), 8).try_into(),
        Ok(KernelError::InvalidArgument),
        "Writing to the rx fd should return an InvalidArgument error"
    );

    assert_eq!(
        syscall!(SyscallNumber::Read as usize, fds[1], buf.as_mut_ptr(), 8).try_into(),
        Ok(KernelError::InvalidArgument),
        "Reading from the tx fd should return an InvalidArgument error"
    );
}

/// Tests pipe error with bad address.
pub fn pipe_error_bad_address() {
    assert_eq!(
        syscall!(SyscallNumber::Pipe as usize, core::ptr::null_mut::<u8>()).try_into(),
        Ok(KernelError::BadAddress),
        "Creating a pipe with a null pointer should return BadAddress error."
    );
}
