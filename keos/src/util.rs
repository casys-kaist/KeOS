//! Debugging Utilities.

use alloc::string::String;
use core::fmt::Write;

use crate::{KernelError, fs::RegularFile};

/// Dumps the bytes in `buf` to the console as hex bytes, arranged 16 per line.
///
/// Each line is buffered into a String and printed in a single operation to
/// avoid console race conditions in multi-threaded environments.
///
/// # Arguments
///
/// * `ofs` - The starting offset for the first byte in `buf`.
/// * `buf` - The slice of bytes to dump.
/// * `ascii` - A flag to enable or disable the ASCII character view.
///
/// # Usage
///
/// ```
/// let my_data: &[u8] = &[0xDE, 0xAD, 0xBE, 0xEF];
/// // This is a safe function and can be called directly.
/// hex_dump_slice(0x100, my_data, true);
/// ```
pub fn hex_dump_slice(ofs: usize, buf: &[u8], ascii: bool) {
    const PER_LINE: usize = 16; // Maximum bytes per line.

    // Create mutable local variables from the immutable arguments to track state.
    let mut current_ofs = ofs;
    let mut current_buf = buf;

    // Loop until all bytes in the buffer have been processed.
    while !current_buf.is_empty() {
        // Create a mutable string to buffer the output for the current line.
        let mut line_buffer = String::new();

        // --- Calculate this line's layout ---

        // `start_col` is the column (0-15) where the first byte of this iteration will
        // be printed.
        let start_col = current_ofs % PER_LINE;

        // Determine how many bytes from the buffer we will print on this line.
        let bytes_on_this_line = (PER_LINE - start_col).min(current_buf.len());

        // `end_col` is the column where our printing will stop.
        let end_col = start_col + bytes_on_this_line;

        // --- Build the line string ---

        // The offset printed at the start of the line is always rounded down.
        // The .unwrap() is safe because writing to a String should not fail.
        write!(&mut line_buffer, "{:016x}  ", current_ofs - start_col).unwrap();

        // --- Append the hex representation ---

        // 1. Append leading spaces for alignment.
        for _ in 0..start_col {
            write!(&mut line_buffer, "   ").unwrap();
        }

        // 2. Append the hex value for each byte.
        for (i, c) in current_buf.iter().enumerate().take(bytes_on_this_line) {
            let current_col = start_col + i;
            write!(&mut line_buffer, "{c:02x}").unwrap();
            if current_col == PER_LINE / 2 - 1 {
                write!(&mut line_buffer, "-").unwrap();
            } else {
                write!(&mut line_buffer, " ").unwrap();
            }
        }

        // --- Append the ASCII representation (if requested) ---
        if ascii {
            // 3. Append trailing spaces to align the ASCII section.
            for _ in end_col..PER_LINE {
                write!(&mut line_buffer, "   ").unwrap();
            }

            write!(&mut line_buffer, "|").unwrap();

            // 1. Append leading spaces for alignment.
            for _ in 0..start_col {
                write!(&mut line_buffer, " ").unwrap();
            }

            // 2. Append the character for each byte.
            for c in current_buf {
                if (0x20..0x7e).contains(c) {
                    write!(&mut line_buffer, "{}", *c as char).unwrap();
                } else {
                    write!(&mut line_buffer, ".").unwrap();
                }
            }

            // 3. Append trailing spaces to align the final bar.
            for _ in end_col..PER_LINE {
                write!(&mut line_buffer, " ").unwrap();
            }
            write!(&mut line_buffer, "|").unwrap();
        }

        // Print the fully constructed line buffer at once.
        println!("{}", line_buffer);

        // --- Update state for the next iteration ---

        // Advance the buffer slice past the bytes we just printed.
        current_buf = &current_buf[bytes_on_this_line..];
        // Increment the master offset.
        current_ofs += bytes_on_this_line;
    }
}

/// Dumps the memory occupied by a value of type `T` to the console.
///
/// This function takes a raw pointer to the data. The size of the data to dump
/// is determined by `core::mem::size_of::<T>()`. This function handles
/// potentially unaligned pointers by first performing an unaligned read.
///
/// # Safety
///
/// The caller must ensure that `ptr` is valid for reads of `size_of::<T>()`
/// bytes for the duration of this function call. The pointer does **not** need
/// to be aligned.
///
/// # Arguments
///
/// * `ofs` - The starting offset for the first byte.
/// * `ptr` - A raw pointer to the data to be dumped.
/// * `ascii` - A flag to enable or disable the ASCII character view.
///
/// # Usage
///
/// ```
/// let my_data: u32 = 0x12345678;
/// // This function is unsafe and must be called within an unsafe block.
/// unsafe {
///     hex_dump(0, &my_data, true);
/// }
/// ```
pub unsafe fn hex_dump<T>(ofs: usize, ptr: *const T, ascii: bool) {
    unsafe {
        // To safely handle potentially unaligned pointers, we first perform an
        // unaligned read into a local variable on the stack. This `value` is
        // guaranteed to have the correct alignment for type T.
        let value: T = core::ptr::read_unaligned(ptr);

        // Now we can safely get a pointer to the local, aligned variable.
        let aligned_ptr: *const T = &value;

        // Determine the size of the data based on its type.
        let size = core::mem::size_of::<T>();

        // Create a byte slice from the aligned pointer and size. This is safe
        // because `aligned_ptr` points to a valid, local variable.
        let slice = core::slice::from_raw_parts(aligned_ptr as *const u8, size);

        // Call the safe, slice-based implementation.
        hex_dump_slice(ofs, slice, ascii)
    }
}

/// Copy a RegularFile's content into another RegularFile.
pub fn copy_file(src: &RegularFile, dest: &RegularFile) -> Result<(), KernelError> {
    let mut buf: [u8; 4096] = [0u8; 4096];
    let size = src.size();

    for i in 0..=(size / 4096) {
        let position = i * 4096;
        let size_to_copy = if (i + 1) * 4096 > size {
            size % 4096
        } else {
            4096
        };

        src.read(position, &mut buf[..size_to_copy])?;
        dest.write(position, &buf[..size_to_copy])?;
    }
    dest.writeback()?;

    Ok(())
}
