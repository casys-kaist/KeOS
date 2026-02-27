//! # Page Cache.
//!
//! The **page cache** is a fundamental component of the operating system’s file
//! system infrastructure. It functions as a high-speed, memory-resident buffer
//! that caches frequently accessed file data, thereby significantly reducing
//! the overhead of disk I/O operations.
//!
//! When a process reads from a file, the kernel first checks whether the
//! requested data is present in the page cache. If it is, the data is served
//! directly from memory, bypassing the much slower disk access path. If not,
//! the system fetches the data from disk, inserts it into the cache, and then
//! returns it to the process. This mechanism greatly enhances system
//! responsiveness and throughput, especially in workloads involving repeated or
//! sequential file access patterns.
//!
//! Because disk I/O is orders of magnitude slower than memory access, the page
//! cache is essential for delivering high performance in modern operating
//! systems. Without it, every file read or write would necessitate direct disk
//! access, resulting in substantial latency and performance degradation.
//!
//! Once the page cache is introduced, file data can be directly mapped into a
//! process’s virtual memory space through `mmap()`. Instead of allocating
//! memory and copying file contents manually, `mmap()` establishes a direct
//! mapping to pages backed by the page cache. This integration enables
//! efficient, demand-paged file I/O: page faults during memory access are
//! resolved by loading data from the page cache. If the page is not yet cached,
//! the kernel fetches it from disk into the cache, then maps it into the
//! process. When mapped pages are modified, they are marked as *dirty*, and
//! later written back to disk as part of the page cache’s write-back policy.
//! This mechanism ensures consistency while minimizing redundant I/O
//! operations.
//!
//! To maintain consistency and durability, the page cache uses a **write-back**
//! policy. Modifications to cached file data are initially applied in memory,
//! and the affected pages are marked as dirty. These pages are later flushed to
//! disk, either explicitly via the `fsync()` system call or automatically by
//! background kernel threads. This approach optimizes performance by deferring
//! costly write operations, while still ensuring data persistence.
//!
//! Since memory is a limited resource, the kernel must eventually evict
//! pages from the page cache to make room for new data. This requires a cache
//! eviction policy that decides which pages to reclaim based on usage
//! patterns—commonly using heuristics such as Least Recently Used (LRU).
//! Eviction is a critical aspect of page cache design, as it balances memory
//! pressure with the goal of retaining useful data in cache to maximize
//! performance.
//!
//! The page cache also enables **readahead**, a performance optimization that
//! preemptively loads file pages into memory based on access patterns. When the
//! kernel detects sequential file access, it predicts future reads and
//! asynchronously fetches additional pages into the cache. This significantly
//! reduces the number of page faults and improves I/O latency, making readahead
//! particularly effective for streaming large files or reading large datasets.
//!
//! Implementing a page cache is a critical step toward building a modern,
//! high-performance operating system. It not only enhances file system
//! efficiency but also provides insight into how the OS bridges the performance
//! gap between fast volatile memory and slow persistent storage.
//!
//! ## Page Cache in KeOS
//!
//! Your goal is to extend KeOS to support a page cache to bridge file I/O
//! operations with memory-backed caching. The template provides three major
//! abstraction:
//!
//! - [`PageCacheInner`]: An internal wrapper that coordinates low-level
//!   interactions between the cache, and storage layer.
//!
//! - [`PageCacheState`]: This is the high-level cache manager. It embeds an
//!   [`LRUCache`] keyed by `(InodeNumber, FileBlockNumber)` and manages up to
//!   512 slots (~2MiB). It is the central entry point for page-cache-aware file
//!   I/O.
//!
//! - [`Slot`]: A [`Slot`] represents a single cached file block. It contains
//!   the owning [`RegularFile`], the corresponding file block number (`FBA`),
//!   the backing [`Page`], and metadata such as the write-back size. Dirty
//!   slots track modifications and are eventually flushed back to disk.
//!
//! ### Readahead Policy
//!
//! KeOS employs a simple readahead policy: when a file block is read, the cache
//! preemptively loads up to 16 subsequent blocks. This heuristic is designed to
//! optimize sequential access workloads (e.g., file scans or streaming),
//! reducing future read latency and improving throughput. Random workloads
//! remain unaffected, since readahead is limited and opportunistic.
//!
//! ### Cache Replacement: LRU
//!
//! [`PageCacheState`] relies on an Least-Recently-Used (LRU) policy to manage
//! memory pressure. When the cache reaches capacity, the least recently used
//! slot is evicted. If the slot is dirty, its contents are flushed back to disk
//! before eviction. This policy balances simplicity and efficiency by retaining
//! hot (recently accessed) pages while discarding cold ones. All these
//! functionalities are provided by the [`LRUCache`] struct.
//!
//! ### Workflow
//!
//! 1. **Read**: On a read request, the cache checks for an existing slot. If
//!    present, data is served from memory; otherwise, the block is loaded from
//!    disk, inserted into the cache, and readahead is triggered.
//!
//! 2. **Write**: Writes update the cached slot in place. The slot is marked
//!    dirty and write-back occurs lazily, either via explicit sync or eviction.
//!
//! 3. **mmap**: Pages can be directly mapped into user space from the page
//!    cache. Faults are resolved by pulling in the corresponding slot.
//!
//! 4. **Unlink**: When a file is deleted, all its slots are invalidated without
//!    flushing, ensuring consistency with the file system state.
//!
//! 5. **Writeback**: Dirty slots are flushed either explicitly (via `fsync`) or
//!    opportunistically during eviction. This ensures persistence while
//!    reducing redundant disk I/O.
//!
//! The following diagram depicts the work-flow of the page cache subsystem of
//! the KeOS.
//! ```text
//!            +-------------------------------+
//!            |       Process issues          |
//!            |  read(), write(), or mmap()   |
//!            +---------------+---------------+
//!                            |
//!                            v
//!                 +-------------------+
//!                 |  Check Slot in    |
//!                 |  PageCacheState   |
//!                 +--------+----------+
//!                  Hit            |   Miss
//!                   |             |
//!                   v             v
//!            +------------+   +----------------------+
//!            | Serve data |   | Load block from disk |
//!            | from cache |   +-----------+----------+
//!            +-----+------+               |
//!                  |                      v
//!                  |             +----------------------+
//!                  |             | Insert block as Slot |
//!                  |             |  into LRUCache       |
//!                  |             |  Trigger readahead   |
//!                  |             +-----------+----------+
//!                  |                         |
//!                  +-------------------------+
//!                  |
//!                  v
//!           +-------------------+
//!           | Is this a write?  |
//!           +--------+----------+
//!                  Yes
//!                   |
//!                   v
//!     +---------------+---------+
//!     | Mark Slot dirty (defer  |
//!     | writeback to disk)      |
//!     +-----------+-------------+
//! ```
//!
//! ## Implementation Requirements
//! You need to implement the followings:
//! - [`PageCacheState::readahead`]
//! - [`PageCacheState::do_read`]
//! - [`PageCacheState::do_write`]
//! - [`PageCacheState::do_mmap`]
//! - [`Slot::read_page`]
//! - [`Slot::write_page`]
//! - [`Slot::writeback`]
//! - [`Slot::drop`]
//! - [`PageCache::read`]
//!
//! After implement the functionalities, move on to the next [`section`].
//!
//! [`section`]: mod@crate::ffs
use crate::lru::LRUCache;
use alloc::{string::ToString, sync::Arc};
use core::ops::{Deref, DerefMut};
use keos::{
    KernelError,
    channel::{Sender, channel},
    fs::{FileBlockNumber, InodeNumber, RegularFile, traits::FileSystem},
    mm::Page,
    thread::{JoinHandle, ThreadBuilder},
};
use keos_project4::sync::mutex::Mutex;

pub mod overlaying;

/// A single entry in the page cache.
///
/// A [`Slot`] corresponds to one file block stored in memory.
pub struct Slot {
    /// The file this slot belongs to.
    pub file: RegularFile,
    /// The file block number this slot represents.
    pub fba: FileBlockNumber,
    /// The backing page containing the block’s data.
    pub page: Page,
    /// Size to be write-backed if dirtied. If the slot is clean, this will be
    /// `None`.
    pub writeback_size: Option<usize>,
}

impl Slot {
    /// Create a new slot for the given file, block, and backing page.
    ///
    /// By default, the slot is clean (i.e., `writeback_size` is `None`).
    pub fn new(file: keos::fs::RegularFile, fba: FileBlockNumber, page: Page) -> Self {
        Slot {
            file,
            fba,
            page,
            writeback_size: None,
        }
    }

    /// Copy the page contents into the provided buffer.
    ///
    /// The buffer must be exactly 4096 bytes long, representing a full
    /// page. This method does not trigger I/O.
    pub fn read_page(&self, buf: &mut [u8; 4096]) {
        todo!()
    }

    /// Update the page contents from the provided buffer.
    ///
    /// Marks the slot as dirty with a write-back of at least `min_size` bytes.
    /// The buffer must be exactly 4096 bytes long.
    pub fn write_page(&mut self, buf: &[u8; 4096], min_size: usize) {
        todo!()
    }

    /// Write back the dirty portion of the page to the underlying file.
    ///
    /// - If `writeback_size` is `Some(size)`, representing slot is dirty,
    ///   marked with the desired minimum file size (in bytes) after write-back.
    /// - On success, clears the `writeback_size` to `None`.
    ///
    /// If the slot is clean, this does not trigger the I/O.
    pub fn writeback(&mut self) -> Result<(), keos::KernelError> {
       todo!() 
    }
}

impl Drop for Slot {
    fn drop(&mut self) {
        // Called on eviction.
        todo!()
    }
}

/// The global page cache state.
///
/// [`PageCacheState`] wraps an [`LRUCache`] mapping `(InodeNumber,
/// FileBlockNumber)` to [`Slot`] entries. It enforces a bounded capacity of 512
/// slots, corresponding to 2 MiB of cached file data.
///
/// This state is protected by a [`Mutex`] inside [`PageCacheInner`], allowing
/// concurrent access from multiple threads with safe eviction.
#[repr(transparent)]
pub struct PageCacheState(
    LRUCache<(InodeNumber, FileBlockNumber), Slot, 512>, // 2MiB
);

impl Deref for PageCacheState {
    type Target = LRUCache<(InodeNumber, FileBlockNumber), Slot, 512>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for PageCacheState {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl PageCacheState {
    /// Perform readahead on sequential file blocks.
    ///
    /// Reads up to **16 consecutive blocks** after the given `fba`
    /// (file block address) into the cache.
    ///
    /// Existing cached slots are not overwritten.
    pub fn readahead(&mut self, file: keos::fs::RegularFile, fba: FileBlockNumber) {
        todo!()
    }

    /// Insert a new [`Slot`] into the page cache.
    ///
    /// Associates the given `(inode, fba)` pair with the slot.
    /// If the cache is at capacity, the least-recently-used slot
    /// will be automatically evicted (writing back its contents if dirty).
    pub fn insert(&mut self, id: (InodeNumber, FileBlockNumber), slot: Slot) {
        self.0.put(id, slot);
    }

    /// Read a file block into the provided buffer.
    ///
    /// - If the block is cached, copies directly from the page cache.
    /// - If the block is not cached, loads it **synchronously** from the file
    ///   system, inserts it into the cache, and copies it into the buffer.
    ///
    /// This method does not triggers the read-ahead requests.
    ///
    /// Returns Ok(true) if there exists any byte read.
    pub fn do_read(
        &mut self,
        file: keos::fs::RegularFile,
        fba: FileBlockNumber,
        buf: &mut [u8; 4096],
    ) -> Result<bool, keos::KernelError> {
        todo!()
    }

    /// Write a file block through the page cache.
    ///
    /// Updates (or inserts) the slot corresponding to the `(file, fba)`
    /// pair with the provided buffer. The slot is marked dirty, and at least
    /// `min_size` bytes are scheduled for write-back.
    ///
    /// This method does not immediately flush to disk; explicit
    /// [`PageCacheState::do_writeback`] or eviction is required for
    /// persistence.
    pub fn do_write(
        &mut self,
        file: keos::fs::RegularFile,
        fba: FileBlockNumber,
        buf: &[u8; 4096],
        min_size: usize,
    ) -> Result<(), keos::KernelError> {
        todo!()
    }

    /// Provide a memory-mapped page for the given file block.
    ///
    /// - If the block is cached, returns a clone of the backing [`Page`].
    /// - If not, loads the block into the cache and returns the new [`Page`].
    ///
    /// This allows direct access to the cached page memory.
    pub fn do_mmap(
        &mut self,
        file: keos::fs::RegularFile,
        fba: FileBlockNumber,
    ) -> Result<Page, keos::KernelError> {
        todo!()
    }

    /// Remove all slots associated with a given file.
    ///
    /// Slots are dropped without flushing dirty data back to the file system.
    /// This is typically used during file unlink (deletion), where data
    /// persistence is no longer required.
    pub fn do_unlink(&mut self, file: keos::fs::RegularFile) {
        let ino = file.0.ino();
        // Remove all slots associated with this file without writeback
        self.0.retain(|(id_ino, _), v| {
            if *id_ino == ino {
                v.writeback_size = None;
                false
            } else {
                true
            }
        });
    }

    /// Write back all dirty slots belonging to the given file.
    ///
    /// Ensures that all cached modifications to the file are persisted
    /// to the underlying file system.
    pub fn do_writeback(&mut self, file: keos::fs::RegularFile) -> Result<(), keos::KernelError> {
        let ino = file.0.ino();
        // Write back all slots associated with this file
        self.0
            .iter_mut()
            .filter(|((id_ino, _), _)| *id_ino == ino)
            .for_each(|(_, slot)| {
                let _ = slot.writeback();
            });

        Ok(())
    }
}

/// Internal representation of a [`PageCache`].
pub struct PageCacheInner<FS: FileSystem> {
    /// The file system that the page cache operates on.
    pub fs: FS,
    /// The shared state of the page cache.
    pub inner: Arc<Mutex<PageCacheState>>,
    /// Channel for sending read-ahead requests to the background thread.
    pub request: Sender<(keos::fs::RegularFile, FileBlockNumber)>,
    /// Join handle for the read-ahead thread.
    _readahead_thread: JoinHandle,
}

/// A reference-counted handle to the page cache.
///
/// [`PageCache`] wraps an [`Arc`] around [`PageCacheInner`], allowing
/// safe sharing across multiple threads. It provides methods to
/// construct a cache and to read pages with read-ahead support.
pub struct PageCache<FS: FileSystem>(pub Arc<PageCacheInner<FS>>);

impl<FS: FileSystem> Clone for PageCache<FS> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<FS: FileSystem> PageCache<FS> {
    /// Create a new page cache associated with the given file system.
    ///
    /// Spawns a background thread to service read-ahead requests.
    pub fn new(fs: FS) -> Self {
        info!("Mounting {} to PageCache.", core::any::type_name::<FS>());
        let (request, rx) = channel(100);
        let inner = Arc::new(Mutex::new(PageCacheState(LRUCache::new())));
        let cloned_inner = inner.clone();
        let _readahead_thread = ThreadBuilder::new("[Readahead]".to_string()).spawn(move || {
            println!(
                "Start [Readahead] (TID: {})",
                keos::thread::Current::get_tid()
            );
            while let Ok((file, fba)) = rx.recv() {
                let mut guard = cloned_inner.lock();
                guard.readahead(file, fba);
                guard.unlock();
            }
        });
        PageCache(Arc::new(PageCacheInner {
            fs,
            inner,
            request,
            _readahead_thread,
        }))
    }

    /// Read a page from the cache or underlying file system.
    ///
    /// A read-ahead request for subsequent pages is issued to the
    /// background thread.
    pub fn read(
        &self,
        file: &keos::fs::RegularFile,
        fba: FileBlockNumber,
        buf: &mut [u8; 4096],
    ) -> Result<bool, KernelError> {
        // TODO:
        // 1. read the requested file synchronously.
        // 2. send a read-ahead request to the readahead thread.
        todo!()
    }
}

impl<FS: FileSystem> Drop for PageCacheInner<FS> {
    fn drop(&mut self) {
        if keos::PANIC_DEPTH.load(core::sync::atomic::Ordering::SeqCst) == 0 {
            let readahead_tid = self._readahead_thread.tid;
            println!(
                "Stop [Readahead] (TID: {}) / success: {}",
                readahead_tid,
                keos::thread::kill_by_tid(readahead_tid, 0).is_ok()
            );
        }
    }
}
