//! Useful tips for implementing KeOS Projects.
//!
//! ## Implementation Strategy
//!
//! We recommend using a **"TODO-driven" approach** to implement project
//! systematically. This method ensures an **incremental and structured**
//! development process:
//!
//! 1. **Run the code** and identify `todo!()` placeholders that cause panics.
//! 2. **Implement the missing functionality**, ensuring it aligns with the
//!    expected behavior described in the project requirements.
//! 3. **Repeat** steps 1 and 2 until all test cases pass and the system behaves
//!    correctly.
//!
//! This approach allows you to build your project **one step at a time**,
//! making debugging and understanding the system easier.
//!
//! ## Stopping an execution
//!
//! When KeOS got stuck in deadlock or does not automatically shut down after
//! it panicked, you may need to forcibly shut down the QEMU.
//!
//! For execution in `cargo grade` or `cargo run` without argument in project 5,
//! press **Ctrl-C** to stop execution.
//!
//! Otherwise, such as running KeOS by `cargo run` in project 1-4, press
//! **Ctrl-A**, then press **X** to stop execution.
//!
//! ## Passing tests != bug-free correctness
//!
//! * The provided tests check basic functionality and semantics only. Passing
//! them does not guarantee your code is bug-free.
//! * Later components that build on your code may stress it in ways the tests
//!   don’t cover, including edge cases.
//! * This also applies across project milestones: a bug you see now may come
//!   from a misimplementation in an earlier project—even if you received full
//!   points at the time.
//!
//! ## Do not assign large variables on stack
//!
//! In **KeOS**, each process/thread is assigned a fixed execution stack of
//! `STACK_SIZE` bytes. While KeOS attempts to detect stack overflows, its
//! detection is not perfect. **A stack overflow may lead to mysterious kernel
//! panics.** To avoid this:
//! - **Avoid declaring large data structures on the stack.**
//! ```rust
//! let v: [u8; 0x200000]; // ERROR: This may cause a stack overflow
//! ```
//!
//! - **Instead, allocate large data structures on the heap using `Box`.**
//! ```rust
//! let v = Box::new([0u8; 0x200000]); // OK: Allocates on the heap
//! ```
