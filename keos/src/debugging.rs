//! This modules explains debugging tips for KeOS.
//!
//! ## Selectively run tests
//!
//! In KeOS, you can run one or more specific test cases by passing their names
//! as arguments to the test runner. For example:
//!
//! ```bash
//! $ cargo run -- syscall::pipe_normal syscall::pipe_partial
//! ```
//!
//! This command runs exactly the listed test cases, `syscall::pipe_normal` and
//! `syscall::pipe_partial`. You may specify a single test case or multiple test
//! cases, depending on your needs.
//!
//!
//! ## Debugging with `print!`
//! `print!` is a simple yet effective debugging tool in KeOS. It allows you to
//! output messages directly to the console, helping you trace the execution.
//! Use `print!` generously while debugging.
//!
//! ### Example: Workflow to debug why `keos_project2::userprog::arg_parse` has no output.
//!
//! There are two possibilities:
//! 1. The user program executed **but never reached the print**.
//!     * This implies the program’s initial state was wrong.
//!     * In the grader directory, you can check that `arg_parse` is launched
//!       via `run_elf_with_arg`.
//!     * Add logging around that path to dump the final state before entering
//!       user mode:
//!         * Print a summary of `mm_struct` and the register set (`regs`).
//!         * Because the existing `mm_struct`/page-table walk helpers may only
//!           return a **final PTE**, write small debug helpers to dump
//!           **intermediate** page-table levels and the final PTE (include
//!           addresses and flags).
//!
//! 2. The user program executed, reached the print, **but failed to print**.
//!    * User–kernel interactions go through the syscall interface, so if it
//!      executed at all, it likely issued a syscall.
//!         * Add a log in `Process::syscall` to print the syscall
//!           **number/args** and the **return_val**.
//!         * If you see this log, the program executed, made a syscall, and the
//!           kernel handled it without panicking.
//!         * Check whether `return_val` is what you expect. If not, dig deeper:
//!    * Additional symptom: syscall number is `Write`, but the result is
//!      `KernelError::BadAddress`
//!         * Add a log inside `Write` to find where `KernelError::BadAddress`
//!           originates.
//!         * If `UserU8SliceRO::get` returns the error, the kernel thinks the
//!           user buffer address is invalid.
//!         * If you believe the address should be valid, continue down the call
//!           chain: **`UserU8SliceRO::get`** → **`Task::access_ok`** →
//!           **`MmStruct::access_ok`** (your implementation). Review these
//!           carefully.
//!
//!
//!
//! ### Trust, but verify, your debug prints
//! * Your logging may call code you wrote, which might itself be buggy. That
//!   can mislead you.
//!   * For example, if you use `get_user_page` (also your code) to dump user
//!     memory. If it’s wrong, you could read the wrong memory and still “see”
//!     plausible contents. In this case, use GDB to inspect the real memory
//!     state during execution. Treat that as ground truth.
//!
//! -----
//!
//! ## Debugging with GDB
//!
//! KeOS supports debugging with **GDB** and **QEMU**. This section provides
//! step-by-step instructions on how to set up and use GDB for effective
//! debugging.
//!
//! ### Running GDB
//!
//! To launch KeOS in debug mode, run the following command **inside each grader
//! directory**:
//! ```bash
//! $ GDB=1 cargo run <TESTCASE>
//! ```
//! You must specify a single test case to debug with a GDB.
//!
//! This starts **QEMU** and waits for a GDB connection on TCP **port 1234**.
//! A `.gdbinit` script will also be generated to automate the debugging setup.
//!
//! In a **separate terminal**, start GDB using:
//! ```bash
//! $ rust-gdb keos_kernel
//! ```
//!
//! We recommend to use `rust-gdb`, as it provides better support for
//! Rust-specific data structures and improves debugging readability.
//!
//! #### One-time setup
//!
//! Before using `rust-gdb`, you may need to modify your **`~/.gdbinit`** file
//! to allow script execution. Add the following line:
//! ```bash
//! set auto-load safe-path /
//! ```
//!
//! After launching `rust-gdb`, the execution will halt at the startup stage,
//! showing output similar to this:
//! ```bash
//! $ rust-gdb
//! warning: No executable has been specified and target does not support
//! determining executable automatically. Try using the "file" command.
//! 0x000000000000fff0 in ?? ()
//! (gdb)
//! ```
//!
//! Now, you can continue execution by typing:
//!
//! ### Inspect Each Core
//!
//! In **QEMU**, each **CPU core** is treated as a **separate thread**.
//! When debugging multi-core execution, be aware that **some cores may panic
//! while others continue running.**
//!
//! To inspect all active cores, use:
//! ```bash
//! (gdb) info threads
//! ```
//!
//! This will display the state of each thread, including which CPU core it
//! belongs to and its current stack frame. For example:
//! ```text
//! (gdb) info threads
//! Id   Target Id         Frame
//! * 1    Thread 1 (CPU#0 [running]) 0x000000000000fff0 in ?? ()
//! 2    Thread 2 (CPU#1 [running]) 0x000000000000fff0 in ?? ()
//! 3    Thread 3 (CPU#2 [running]) 0x000000000000fff0 in ?? ()
//! 4    Thread 4 (CPU#3 [running]) 0x000000000000fff0 in ?? ()
//! ```
//!
//! To switch to a specific **CPU core (thread)**, use:
//! ```bash
//! (gdb) thread {thread_id}
//! ```
//!
//! This allows you to inspect registers, call stacks, and execution state
//! per core.
//!
//! ### Analyzing Execution State
//!
//! #### Viewing the Call Stack (Backtrace)
//!
//! Use `backtrace` (or `bt`) to display the **call stack** of the current
//! thread:
//! ```bash
//! (gdb) bt
//! ```
//!
//! Each function call in the stack is represented as a **frame**.
//! To switch to a specific frame, use:
//! ```bash
//! (gdb) frame {frame_id}
//! ```
//!
//! Once inside a frame, you can inspect variables:
//! ```bash
//! (gdb) info args
//! (gdb) info locals
//! (gdb) i r
//! ```
//!
//! **Debugging a Panic:**
//! If you encounter a **kernel panic** during a test, use:
//! 1. `info threads` to locate the crashing core
//! 2. `bt` to examine the backtrace
//! 3. `frame {frame_id}` to inspect function parameters
//!
//! ### Setting Breakpoints
//!
//! Breakpoints help stop execution at specific points. However, in **multi-core
//! debugging**, regular breakpoints may not always work correctly.
//! Instead, use **hardware breakpoints**:
//! ```bash
//! (gdb) hb * {address_of_breakpoint}
//! ```
//!
//! To view the source code that the current CPU is executing, use:
//! ```bash
//! (gdb) layout asm
//! (gdb) layout src
//! ```
//!
//! #### Examples
//!
//! Here are some examples of how to set breakpoints in GDB:
//! ```text
//! (gdb) hbreak function_name  # Example: hbreak keos::fs::Directory::open
//! (gdb) hbreak *address       # Example: hbreak *0x1000
//! (gdb) hbreak (file:)line    # Example: hbreak syscall.rs:164
//! ```
//!
//! #### Example 1
//!
//! To debug the `syscall::read_normal` test case in project 1, and set a
//! breakpoint at `syscall.rs:150`, use:
//! ```bash
//! (gdb) hbreak syscall.rs:150
//! ```
//!
//! Alternatively, you can set a breakpoint by the test case's name:
//! ```bash
//! (gdb) hbreak project1_grader::syscall::read_normal
//! ```
//!
//! You can even set a breakpoint on the closure entry, for instance, to set a
//! on a closure of `sync::semaphore::sema_0` test case in project 3:
//! ```bash
//! (gdb) hbreak project3_grader::sync::semaphore::{{closure}}
//! ```
//!
//! To limit debugging to one core, use `thread apply`:
//! ```bash
//! (gdb) thread apply 1 hbreak syscall.rs:150
//! (gdb) c
//! ```
//!
//! #### Example 2
//!
//! To stop at a breakpoint only when a specific condition is met (e.g., when a
//! parameter is `0xcafe0000`), use:
//! ```bash
//! (gdb) hbreak walk if va.__0 == 0xcafe0000
//! ```
//!
//! This approach allows you to focus on specific conditions and skip over
//! unrelated calls.
