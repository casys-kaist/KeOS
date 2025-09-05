//! # Journaling for Crash Consistency.
//!
//! File systems must ensure data consistency in the presence of crashes or
//! power failures.  When a system crash or power failure occurs, in-progress
//! file operations may leave the file system in an inconsistent state, where
//! metadata and data blocks are only partially updated. This can lead to file
//! corruption, orphaned blocks, or even complete data loss. Thus, modern file
//! systems must guard against these scenarios to ensure durability and
//! recoverability.
//!
//! To address this, modern file systems employ **journaling**. Journaling
//! provides crash-consistency by recording intended changes to a special log
//! (called the journal) before applying them to the main file system. In the
//! event of a crash, the journal can be replayed to recover to a consistent
//! state. This significantly reduces the risk of data corruption and allows
//! faster recovery after unclean shutdowns, without the need for full
//! file system checks.
//!
//! In this approach, all intended updates, such as block allocations, inode
//! changes, or directory modifications, are first written to a special log
//! called the **journal**. Only after the log is safely persisted to disk,
//! the actual file system structures updated. In the event of a crash, the
//! system can replay the journal to restore a consistent state. This method
//! provides a clear "intent before action" protocol, making recovery
//! predictable and bounded.
//!
//! ## Journaling in KeOS
//!
//! To explore the fundamentals of crash-consistent file systems, **KeOS
//! implements a minimal meta-data journaling mechanism** using the well-known
//! technique of **write-ahead logging**. This mechanism ensures that
//! updates to file system structures are made durable and recoverable.
//!
//! The journaling mechanism is anchored by a **journal superblock**, which
//! includes a `commited` flag. This flag indicates whether the journal area
//! currently holds valid, committed journal data that has not yet been
//! checkpointed.
//!
//! Journals in KeOS structured around four key stages: **Metadata updates**,
//! **commit**, **checkpoint**, and **recovery**.
//!
//! ### 1. Metadata Updates
//!
//! In KeOS, journaling is tightly integrated with the [`RunningTransaction`]
//! struct, which acts as the central abstraction for managing write-ahead
//! logging of file system changes. All journaled operations must be serialized
//! through this structure to ensure consistency.
//!
//! Internally, [`RunningTransaction`] is protected by a `SpinLock` on the
//! journal superblock, enforcing **global serialization** of journal writes.
//! This design guarantees that only one transaction may be in progress at any
//! given time, preventing concurrent updates to the same block, which could
//! otherwise result in a corrupted or inconsistent state.
//!
//! Crucially, KeOS uses Rust’s strong type system to enforce this safety at
//! compile time: without access to an active [`RunningTransaction`], it is
//! **impossible** to write metadata blocks. All metadata modifications must be
//! submitted explicitly via the `submit()` method, which stages the changes for
//! journaling.
//!
//! If you forget to submit modified blocks through [`RunningTransaction`], the
//! kernel will **panic** with a clear error message, catching the issue early
//! and avoiding silent corruption. This design provides both safety and
//! transparency, making metadata updates robust and auditable.
//!
//!
//! ### 2. Commit Phase: [`RunningTransaction::commit`]
//!
//! In the commit phase, KeOS records all pending modifications to a dedicated
//! **journal area** before applying them to their actual on-disk locations:
//!
//! A transaction begins with a **`TxBegin` block**, which contains a list of
//! logical block addresses that describe where the updates will eventually be
//! written. This is followed by the **journal data blocks**, which contain the
//! actual contents to be written to the specified logical blocks. Once all data
//! blocks have been written, a **`TxEnd` block** is appended to mark the
//! successful conclusion of the transaction. This write-ahead logging
//! discipline guarantees that no update reaches the main file system until its
//! full intent is safely recorded in the journal.
//!
//! You can write journal blocks with [`JournalWriter`] struct. This structure
//! is marked with a type that represent the stages of commit phase, enforcing
//! you to write journal blocks in a correct order.
//!
//! ### 3. Checkpoint Phase: [`Journal::checkpoint`]
//!
//! After a transaction is fully committed, the system proceeds to
//! **checkpoint** the journal. During checkpointing, the journaled data blocks
//! are copied from the journal area to their final destinations in the main
//! file system (i.e., to the logical block addresses specified in the `TxBegin`
//! block).
//!
//! Once all modified blocks have been written to their final locations, the
//! system clears the journal by resetting the `commited` flag in the journal
//! superblock. This indicates that the journal is no longer recovered when
//! crash.
//!
//! In modern file systems, checkpointing is typically performed
//! **asynchronously** in the background to minimize the latency of system calls
//! like `write()` or `fsync()`. This allows the file system to acknowledge the
//! operation as complete once the journal is committed, without waiting for the
//! final on-disk update.
//!
//! However, for simplicity in this project, **checkpointing is done
//! synchronously**: the file system waits until all journaled updates are
//! copied to their target locations before clearing the journal. This
//! simplifies correctness, avoids the need for background threads or
//! deferred work mechanisms, and reduces work for maintaining consistent view
//! between disk and commited data.
//!
//!
//! ### 4. Recovery: [`Journal::recovery`]
//!
//! If a crash occurs before the checkpointing phase completes, KeOS
//! **recovers** the file system during the next boot. It begins by inspecting
//! the journal superblock to determine whether a committed transaction exists.
//!
//! If the `committed` flag is set and a valid `TxBegin`/`TxEnd` pair is
//! present, this indicates a completed transaction whose changes have not yet
//! been checkpointed. In this case, KeOS retries the **checkpointing**. If the
//! journal is not marked as committed, the system discards the journal
//! entirely. This rollback ensures consistency by ignoring partially written
//! or aborted transactions.
//!
//! This recovery approach is both **bounded** and **idempotent**: it scans only
//! the small, fixed-size journal area, avoiding costly full file system
//! traversal, and it can safely retry recovery without side effects if
//! interrupted again.
//!
//! ## Implementation Requirements
//! You need to implement the followings:
//!   - [`Journal::recovery`]
//!   - [`Journal::checkpoint`]
//!   - [`JournalWriter::<TxBegin>::write_tx_begin`]
//!   - [`JournalWriter::<Block>::write_blocks`]
//!   - [`JournalWriter::<TxEnd>::write_tx_end`]
//!
//! After implement the functionalities, move on to the last [`section`] of the
//! KeOS.
//!
//! [`section`]: mod@crate::advanced_file_structs

use crate::ffs::{
    FastFileSystemInner, JournalIO, LogicalBlockAddress,
    disk_layout::{JournalSb, JournalTxBegin, JournalTxEnd},
};
use alloc::{boxed::Box, vec::Vec};
use core::cell::RefCell;
use keos::{KernelError, sync::SpinLockGuard};

/// A structure representing the journal metadata used for crash consistency.
///
/// Journaling allows the file system to recover from crashes by recording
/// changes in a write-ahead log before committing them to the main file system.
/// This ensures that partially written operations do not corrupt the file
/// system state.
///
/// The `Journal` struct encapsulates the journaling superblock and the total
/// size of the journal region on disk. It is responsible for managing the
/// checkpointing process, which commits durable changes and clears completed
/// transactions.
///
/// # Fields
/// - `sb`: The journal superblock, containing configuration and state of the
///   journal.
/// - `size`: The total number of blocks allocated for the journal region.
pub struct Journal {
    /// Journal superblock.
    pub sb: Box<JournalSb>,
}

impl Journal {
    /// Recovers and commited but not checkpointed transactions from the
    /// journal.
    ///
    /// This function is invoked during file system startup to ensure
    /// metadata consistency in the event of a system crash or power failure.
    /// It scans the on-disk journal area for valid transactions and re-applies
    /// them to the file system metadata.
    ///
    /// If no complete transaction is detected, the journal is left unchanged.
    /// If a partial or corrupt transaction is found, it is safely discarded.
    ///
    /// # Parameters
    /// - `ffs`: A reference to the core file system state, used to apply
    ///   recovered metadata.
    /// - `io`: The journal I/O interface used to read journal blocks and
    ///   perform recovery writes.
    ///
    /// # Returns
    /// - `Ok(())` if recovery completed successfully or no action was needed.
    /// - `Err(KernelError)` if an unrecoverable error occurred during recovery.
    pub fn recovery(
        &mut self,
        ffs: &FastFileSystemInner,
        io: &JournalIO,
    ) -> Result<(), KernelError> {
        todo!()
    }

    /// Commits completed journal transactions to the file system.
    ///
    /// This method performs the **checkpoint** operation: it flushes completed
    /// transactions from the journal into the main file system, ensuring their
    /// effects are permanently recorded.
    ///
    /// # Parameters
    /// - `ffs`: A reference to the file system core (`FastFileSystemInner`),
    ///   needed to apply changes to metadata blocks.
    /// - `io`: An object for performing I/O operations related to the journal.
    /// - `debug_journal`: If true, enables debug logging for checkpointing.
    ///
    /// # Returns
    /// - `Ok(())`: If checkpointing succeeds and all transactions are flushed.
    /// - `Err(KernelError)`: If I/O or consistency errors are encountered.
    pub fn checkpoint(
        &mut self,
        ffs: &FastFileSystemInner,
        io: &JournalIO,
        debug_journal: bool,
    ) -> Result<(), KernelError> {
        if self.sb.commited != 0 {
            let mut block = Box::new([0; 4096]);
            let tx_begin = JournalTxBegin::from_io(io, ffs.journal().start + 1)?;
            if debug_journal {
                println!("[FFS-Journal]: Transaction #{} [", tx_begin.tx_id);
            }
            for (idx, slot) in tx_begin.lbas.iter().enumerate() {
                if let Some(slot) = slot {
                    if debug_journal {
                        println!("[FFS-Journal]:      #{:04}: {:?},", idx, slot);
                    }
                    todo!();
                } else {
                    break;
                }
            }
            if debug_journal {
                println!("[FFS-Journal]: ] Checkpointed.");
            }
            self.sb.commited = 0;
            self.sb.writeback(io, ffs)?;
        }
        Ok(())
    }
}

/// Represents an in-progress file system transaction using write-ahead
/// journaling.
///
/// A `RunningTransaction` buffers metadata updates to disk blocks before they
/// are permanently written, ensuring crash consistency. When a transaction is
/// committed, the buffered blocks are flushed to the journal area first. Once
/// the journal write completes, the updates are applied to the actual metadata
/// locations on disk.
///
/// Transactions are used to group file system changes atomically — either all
/// updates in a transaction are committed, or none are, preventing partial
/// updates.
///
/// # Fields
/// - `tx`: A buffer that stores staged metadata writes as a list of (LBA, data)
///   tuples.
/// - `journal`: A locked handle to the global `Journal`, used during commit.
/// - `tx_id`: Unique identifier for the current transaction.
/// - `io`: The journal I/O interface used for block-level reads/writes.
/// - `debug_journal`: Enables logging of journal operations for debugging.
/// - `ffs`: A reference to the file system's core structure.
pub struct RunningTransaction<'a> {
    tx: RefCell<Vec<(LogicalBlockAddress, Box<[u8; 4096]>)>>,
    journal: Option<SpinLockGuard<'a, Journal>>,
    tx_id: u64,
    io: Option<JournalIO<'a>>,
    debug_journal: bool,
    pub ffs: &'a FastFileSystemInner,
}

impl<'a> RunningTransaction<'a> {
    /// Begins a new journaled transaction.
    ///
    /// Initializes the transaction state and prepares to buffer metadata
    /// writes.
    ///
    /// # Parameters
    /// - `name`: A label for the transaction, useful for debugging.
    /// - `ffs`: The file system core structure.
    /// - `io`: The journal I/O interface for block operations.
    /// - `debug_journal`: Enables verbose logging if set to `true`.
    #[inline]
    pub fn begin(
        name: &str,
        ffs: &'a FastFileSystemInner,
        io: JournalIO<'a>,
        debug_journal: bool,
    ) -> Self {
        let mut journal = ffs.journal.as_ref().map(|journal| journal.lock());
        let tx_id = journal
            .as_mut()
            .map(|j| {
                let tx_id = j.sb.tx_id;
                j.sb.tx_id += 1;
                tx_id
            })
            .unwrap_or(0);
        if debug_journal && journal.is_some() {
            println!("[FFS-Journal]: Transaction #{} \"{}\" [", tx_id, name);
        }
        RunningTransaction {
            tx: RefCell::new(Vec::new()),
            journal,
            io: Some(io),
            tx_id,
            debug_journal,
            ffs,
        }
    }

    /// Buffers a metadata block modification for inclusion in the transaction.
    ///
    /// The actual write is deferred until `commit()` is called.
    ///
    /// # Parameters
    /// - `lba`: The logical block address where the metadata will eventually be
    ///   written.
    /// - `data`: A boxed page of data representing the new metadata contents.
    /// - `ty`: A type string name of the metadata (for debugging).
    #[inline]
    pub fn write_meta(&self, lba: LogicalBlockAddress, data: Box<[u8; 4096]>, ty: &str) {
        if self.debug_journal {
            println!(
                "[FFS-Journal]:      #{:04}: {:20} - {:?},",
                self.tx.borrow_mut().len(),
                ty.split(":").last().unwrap_or("?"),
                lba
            );
        }
        self.tx.borrow_mut().push((lba, data));
    }

    /// Commits the transaction to the journal and applies changes to disk.
    ///
    /// This method performs the following steps:
    /// 1. Writes all staged metadata blocks to the journal region on disk.
    /// 2. Updates the journal superblock.
    /// 3. Checkpoint the journal.
    ///
    /// # Returns
    /// - `Ok(())`: If the transaction was successfully committed and
    ///   checkpointed.
    /// - `Err(KernelError)`: If an I/O or consistency error occurred.
    pub fn commit(mut self) -> Result<(), KernelError> {
        // In real filesystem, there exist more optimizations to reduce disk I/O, such
        // as merging the same LBA in a journal into one block.
        let (io, tx, journal, tx_id, ffs, debug_journal) = (
            self.io.take().unwrap(),
            core::mem::take(&mut *self.tx.borrow_mut()),
            self.journal.take(),
            self.tx_id,
            self.ffs,
            self.debug_journal,
        );

        if let Some(journal) = journal {
            if debug_journal {
                println!("[FFS-Journal]: ] Commited.");
            }
            let (mut journal, io) = JournalWriter::new(tx, journal, io, ffs, tx_id)
                .write_tx_begin()?
                .write_blocks()?
                .write_tx_end()?;

            // In real file system, the checkpointing works asynchronously by the kernel
            // thread.
            //
            // However, to keep the implementation simple, synchronously checkpoints the
            // journaled update right after the commit.
            let result = journal.checkpoint(ffs, &io, debug_journal);
            journal.unlock();
            result
        } else {
            // When a journaling is not supported, write the metadata directly on the
            // locations.
            for (lba, block) in tx.into_iter() {
                io.write_metadata_block(lba, block.as_array().unwrap())?;
            }
            Ok(())
        }
    }
}

impl Drop for RunningTransaction<'_> {
    fn drop(&mut self) {
        if let Some(journal) = self.journal.take() {
            journal.unlock();
        }
    }
}

/// Marker type for the first phase of a journal commit: TxBegin.
///
/// Used with [`JournalWriter`] to enforce commit stage ordering via the type
/// system.
pub struct TxBegin {}

/// Marker type for the second phase of a journal commit: writing the metadata
/// blocks.
///
/// Ensures that [`JournalWriter::write_tx_begin`] must be called before
/// [`JournalWriter::write_blocks`].
pub struct Block {}

/// Marker type for the final phase of a journal commit: TxEnd.
///
/// Ensures that [`JournalWriter::write_blocks`] are completed before finalizing
/// the transaction.
pub struct TxEnd {}

/// A staged writer for committing a transaction to the journal.
///
/// `JournalWriter` uses a type-state pattern to enforce the correct sequence of
/// journal writes:
/// - `JournalWriter<TxBegin>`: Can only call [`JournalWriter::write_tx_begin`].
/// - `JournalWriter<Block>`: Can only call [`JournalWriter::write_blocks`].
/// - `JournalWriter<TxEnd>`: Can only call [`JournalWriter::write_tx_end`].
///
/// This staged API ensures that transactions are written in the correct order
/// and prevents accidental misuse.
pub struct JournalWriter<'a, WriteTarget> {
    /// Staged list of (LBA, data) pairs representing metadata blocks to commit.
    tx: Vec<(LogicalBlockAddress, Box<[u8; 4096]>)>,

    /// A lock-protected handle to the journal structure.
    journal: SpinLockGuard<'a, Journal>,

    /// I/O interface for reading/writing disk blocks.
    io: JournalIO<'a>,

    /// Reference to the filesystem's core state.
    ffs: &'a FastFileSystemInner,

    /// Unique identifier of the transaction.
    tx_id: u64,

    /// Internal index tracking progress through `tx`.
    index: usize,

    /// Phantom data used to track the current commit stage.
    _write_target: core::marker::PhantomData<WriteTarget>,
}

impl<'a> JournalWriter<'a, TxBegin> {
    /// Creates a new `JournalWriter` in the initial `TxBegin` stage.
    ///
    /// This prepares the writer for the staged commit sequence of the given
    /// transaction.
    ///
    /// # Parameters
    /// - `tx`: The list of metadata blocks to be written.
    /// - `journal`: A locked handle to the global journal state.
    /// - `io`: The disk I/O interface.
    /// - `ffs`: A reference to the file system.
    /// - `tx_id`: A unique ID assigned to the transaction.
    ///
    /// # Returns
    /// A `JournalWriter` instance in the `TxBegin` state.
    pub fn new(
        tx: Vec<(LogicalBlockAddress, Box<[u8; 4096]>)>,
        journal: SpinLockGuard<'a, Journal>,
        io: JournalIO<'a>,
        ffs: &'a FastFileSystemInner,
        tx_id: u64,
    ) -> Self {
        Self {
            tx,
            journal,
            io,
            ffs,
            tx_id,
            index: 0,
            _write_target: core::marker::PhantomData,
        }
    }

    /// Writes the `TxBegin` marker to the journal.
    ///
    /// This signals the start of a journaled transaction. Must be called before
    /// writing the data blocks.
    ///
    /// # Returns
    /// A `JournalWriter` in the `Block` stage.
    pub fn write_tx_begin(mut self) -> Result<JournalWriter<'a, Block>, KernelError> {
        let mut tx_begin = JournalTxBegin::new(self.tx_id);
        todo!();
        Ok(JournalWriter {
            tx: self.tx,
            journal: self.journal,
            ffs: self.ffs,
            io: self.io,
            tx_id: self.tx_id,
            index: self.index,
            _write_target: core::marker::PhantomData,
        })
    }
}

impl<'a> JournalWriter<'a, Block> {
    /// Writes all staged metadata blocks to the journal.
    ///
    /// Each block is written sequentially to a dedicated journal area.
    /// This must be called after `write_tx_begin()` and before finalizing with
    /// `write_tx_end()`.
    ///
    /// # Returns
    /// A `JournalWriter` in the `TxEnd` stage.
    pub fn write_blocks(mut self) -> Result<JournalWriter<'a, TxEnd>, KernelError> {
        todo!();
        Ok(JournalWriter {
            tx: self.tx,
            journal: self.journal,
            ffs: self.ffs,
            io: self.io,
            tx_id: self.tx_id,
            index: self.index,
            _write_target: core::marker::PhantomData,
        })
    }
}

impl<'a> JournalWriter<'a, TxEnd> {
    /// Writes the `TxEnd` and completes the transaction by updating journal
    /// superblock.
    ///
    /// This signals a successfully completed transaction and allows recovery
    /// mechanisms to apply the journal contents to the actual file system
    /// metadata.
    ///
    /// # Returns
    /// - The locked journal and I/O handle, to checkpoint the journal.
    /// - `Err(KernelError)` if the final commit stage fails.
    pub fn write_tx_end(
        mut self,
    ) -> Result<(SpinLockGuard<'a, Journal>, JournalIO<'a>), KernelError> {
        let tx_end = JournalTxEnd::new(self.tx_id);
        // In the real-file system, this TxEnd block usally omitted to reduce the disk
        // I/O.
        todo!();

        // Mark the Transaction is commited to the JournalSb.
        let Self {
            mut journal,
            io,
            ffs,
            ..
        } = self;
        journal.sb.commited = 1;
        match journal.sb.writeback(&io, ffs) {
            Ok(_) => Ok((journal, io)),
            Err(e) => {
                journal.unlock();
                Err(e)
            }
        }
    }
}
