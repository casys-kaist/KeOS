//! [`StackBuilder`], a utility for constructing a user-space stack layout.
use crate::{mm_struct::MmStruct, pager::Pager};
use keos::{KernelError, addressing::Va, mm::page_table::Permission};

/// A utility for constructing a user-space stack layout.
///
/// [`StackBuilder`] provides methods to allocate, align, and push data onto
/// a stack before mapping it into a user process. It is primarily used to
/// prepare the initial stack for a new process, including setting up `argv`
/// and other necessary data.
///
/// The stack starts at virtual address `0x4748_0000` and grows downward.
///
/// # Fields
/// - `sp`: The current stack pointer, representing the top of the stack.
/// - `pages`: A list of allocated pages that will back the stack.
///
/// # Usage
/// 1. **Create a new stack** using [`StackBuilder::new`].
/// 2. **Push data** (e.g., arguments, environment variables) onto the stack.
/// 3. **Align the stack** for proper memory layout.
/// 4. **Finalize the stack** using [`StackBuilder::finish`] to map it into the
///    process's address space.
pub struct StackBuilder<'a, P: Pager> {
    sp: Va,
    mm_state: &'a mut MmStruct<P>,
}

impl<'a, P: Pager> StackBuilder<'a, P> {
    /// Creates a new [`StackBuilder`] instance for building a user-space stack.
    ///
    /// The stack is initialized at virtual address `0x4748_0000` and grows
    /// downward as data is pushed onto it.
    ///
    /// # Returns
    /// A new [`StackBuilder`] with an empty stack and no allocated pages.
    pub fn new(mm_state: &'a mut MmStruct<P>) -> Result<Self, KernelError> {
        mm_state
            .do_mmap(
                Va::new(0x4748_0000 - 0x10000).unwrap(),
                0x10000,
                Permission::READ | Permission::WRITE | Permission::USER,
                None,
                0,
            )
            .map(|_| Self {
                sp: Va::new(0x4748_0000).unwrap(),
                mm_state,
            })
    }

    /// Consume the [`StackBuilder`] and return the stack pointer.
    ///
    ///
    /// # Returns
    /// - `Ok(Va)`: The final stack pointer after mapping.
    /// - `Err(KernelError)`: If the stack mapping fails.
    pub fn finish(self) -> Va {
        self.sp
    }

    /// Returns the current stack pointer.
    ///
    /// The stack pointer (`sp`) indicates the top of the stack, where the next
    /// value would be pushed. The stack grows downward, meaning the pointer
    /// decreases as more data is pushed onto it.
    ///
    /// # Returns
    /// - The current stack pointer as a virtual address ([`Va`]).
    #[inline]
    pub fn sp(&self) -> Va {
        self.sp
    }

    /// Aligns the stack pointer to the given alignment.
    ///
    /// This function ensures that the stack pointer is aligned to the specified
    /// byte boundary, which is useful for maintaining proper data alignment
    /// when pushing values.
    ///
    /// # Parameters
    /// - `align`: The byte alignment requirement.
    ///
    /// # Behavior
    /// - If the stack pointer is not already aligned, it is adjusted downward
    ///   to meet the alignment requirement.
    #[inline]
    pub fn align(&mut self, align: usize) {
        while !self.sp.into_usize().is_multiple_of(align) {
            self.sp -= 1;
        }
    }

    /// Pushes a byte array onto the stack.
    ///
    /// This function decreases the stack pointer to allocate space for the
    /// value and stores it at the new top of the stack.
    ///
    /// # Parameters
    /// - `v`: The `[u8]` value to be pushed onto the stack.
    ///
    /// # Returns
    /// - The updated stack pointer after pushing the value.
    pub fn push_bytes(&mut self, mut bytes: &[u8]) -> Va {
        // HINT: use `get_user_page_and`
        todo!();
        self.sp()
    }

    /// Pushes a `usize` value onto the stack.
    ///
    /// This function decreases the stack pointer to allocate space for the
    /// value and stores it at the new top of the stack.
    ///
    /// # Parameters
    /// - `v`: The `usize` value to be pushed onto the stack.
    ///
    /// # Returns
    /// - The updated stack pointer after pushing the value.
    pub fn push_usize(&mut self, v: usize) -> Va {
        self.push_bytes(&v.to_ne_bytes())
    }

    /// Pushes a string onto the stack as a C-style string (null-terminated).
    ///
    /// This function copies the given string onto the stack, appends a null
    /// terminator (`\0`), and returns the virtual address ([`Va`]) where the
    /// string is stored.
    ///
    /// # Parameters
    /// - `s`: The string to push onto the stack.
    ///
    /// # Returns
    /// - The virtual address ([`Va`]) pointing to the beginning of the stored
    ///   string in memory.
    ///
    /// # Behavior
    /// - The stack pointer is adjusted downward to allocate space for the
    ///   string.
    /// - The string is stored in memory in a null-terminated format, making it
    ///   compatible with C-style APIs.
    #[inline]
    pub fn push_str(&mut self, s: &str) -> Va {
        // Make a space for null bytes ('\0').
        self.sp -= 1;
        // Push the string slice.
        self.push_bytes(s.as_bytes())
    }
}
