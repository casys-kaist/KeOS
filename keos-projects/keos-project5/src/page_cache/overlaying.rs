//! An overlaying mechanism for appling page cache to any file system.

use super::PageCache;
use alloc::{string::String, vec::Vec};
use keos::{
    fs::{FileBlockNumber, InodeNumber, traits::FileSystem},
    mm::Page,
    sync::atomic::AtomicUsize,
};

/// An overlay on the Directory.
pub struct Directory<FS: FileSystem + 'static>(keos::fs::Directory, PageCache<FS>);

impl<FS: FileSystem> keos::fs::traits::Directory for Directory<FS> {
    fn ino(&self) -> InodeNumber {
        self.0.ino()
    }

    fn size(&self) -> usize {
        self.0.size()
    }

    fn link_count(&self) -> usize {
        self.0.link_count()
    }

    fn open_entry(&self, entry: &str) -> Result<keos::fs::File, keos::KernelError> {
        self.0.open(entry).map(|en| match en {
            keos::fs::File::RegularFile(r) => {
                keos::fs::File::RegularFile(keos::fs::RegularFile::new(RegularFile {
                    size: AtomicUsize::new(r.size()),
                    file: r,
                    cache: self.1.clone(),
                }))
            }
            keos::fs::File::Directory(d) => {
                keos::fs::File::Directory(keos::fs::Directory::new(Directory(d, self.1.clone())))
            }
        })
    }

    fn create_entry(&self, entry: &str, is_dir: bool) -> Result<keos::fs::File, keos::KernelError> {
        self.0.create(entry, is_dir).map(|en| match en {
            keos::fs::File::RegularFile(r) => {
                keos::fs::File::RegularFile(keos::fs::RegularFile::new(RegularFile {
                    size: AtomicUsize::new(r.size()),
                    file: r,
                    cache: self.1.clone(),
                }))
            }
            keos::fs::File::Directory(d) => {
                keos::fs::File::Directory(keos::fs::Directory::new(Directory(d, self.1.clone())))
            }
        })
    }

    fn unlink_entry(&self, entry: &str) -> Result<(), keos::KernelError> {
        self.0.open(entry).map(|en| {
            if let keos::fs::File::RegularFile(r) = en {
                // Remove the slot from the cache
                let mut guard = self.1.0.inner.lock();
                guard.do_unlink(r);
                guard.unlock();
            }
        })?;

        self.0.unlink(entry)
    }

    fn read_dir(&self) -> Result<Vec<(InodeNumber, String)>, keos::KernelError> {
        self.0.read_dir()
    }

    fn removed(&self) -> Result<&keos::sync::atomic::AtomicBool, keos::KernelError> {
        self.0.removed()
    }
}

/// An overlay on the RegularFile.
pub struct RegularFile<FS: FileSystem> {
    file: keos::fs::RegularFile,
    size: AtomicUsize,
    cache: PageCache<FS>,
}

impl<FS: FileSystem> keos::fs::traits::RegularFile for RegularFile<FS> {
    fn ino(&self) -> InodeNumber {
        self.file.0.ino()
    }

    fn size(&self) -> usize {
        self.size.load()
    }

    fn read(&self, fba: FileBlockNumber, buf: &mut [u8; 4096]) -> Result<bool, keos::KernelError> {
        self.cache.read(&self.file, fba, buf)
    }

    fn write(
        &self,
        fba: FileBlockNumber,
        buf: &[u8; 4096],
        min_size: usize,
    ) -> Result<(), keos::KernelError> {
        if self.size() < min_size {
            self.size.store(min_size);
            let mut guard = self.cache.0.inner.lock();
            let result = guard.do_write(self.file.clone(), fba, buf, min_size);
            guard.unlock();
            result
        } else {
            let mut guard = self.cache.0.inner.lock();
            let result = guard.do_write(self.file.clone(), fba, buf, self.size.load());
            guard.unlock();
            result
        }
    }

    fn writeback(&self) -> Result<(), keos::KernelError> {
        let mut guard = self.cache.0.inner.lock();
        let result = guard.do_writeback(self.file.clone());
        guard.unlock();
        result
    }

    fn mmap(&self, fba: FileBlockNumber) -> Result<Page, keos::KernelError> {
        let mut guard = self.cache.0.inner.lock();
        let result = guard.do_mmap(self.file.clone(), fba);
        guard.unlock();
        result
    }
}

impl<FS: FileSystem + 'static> FileSystem for PageCache<FS> {
    fn root(&self) -> Option<keos::fs::Directory> {
        self.0
            .fs
            .root()
            .map(|n| keos::fs::Directory::new(Directory(n, Self(self.0.clone()))))
    }
}
