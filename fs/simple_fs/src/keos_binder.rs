use crate::{Disk, Error, Sector};
use alloc::{collections::BTreeMap, string::String, sync::Arc, vec::Vec};
use keos::{
    fs::{Directory, File, FileBlockNumber, InodeNumber, RegularFile},
    sync::{atomic::AtomicU32, spinlock::SpinLock},
};

/// The filesystem disk.
#[derive(Debug)]
pub struct FsDisk(usize);

impl Disk for FsDisk {
    fn read(&self, sector: Sector, buf: &mut [u8; 512]) -> Result<(), Error> {
        let dev = abyss::dev::get_bdev(self.0).ok_or(Error::DiskError)?;
        dev.read_bios(&mut Some((512 * sector.into_usize(), buf.as_mut())).into_iter())
            .map_err(|_| Error::DiskError)
    }
    fn write(&self, sector: Sector, buf: &[u8; 512]) -> Result<(), Error> {
        let dev = abyss::dev::get_bdev(self.0).ok_or(Error::DiskError)?;
        dev.write_bios(&mut Some((512 * sector.into_usize(), buf.as_ref())).into_iter())
            .map_err(|_| Error::DiskError)
    }
}

#[derive(Clone)]
pub struct FileSystem(Arc<super::SimpleFs<FsDisk>>);

impl FileSystem {
    pub fn load(slot_idx: usize) -> Result<Self, super::Error> {
        abyss::dev::get_bdev(slot_idx).ok_or(Error::DiskError)?;
        super::SimpleFs::load(FsDisk(slot_idx)).map(|o| FileSystem(Arc::new(o)))
    }
}

// RegularFile Inode starts from 2.
// XXX: it enforces no two simple_fs mounted simultaneously.
static GLOBAL_SIMPLEFS_INO_COUNTER: AtomicU32 = AtomicU32::new(2);
static GLOBAL_SIMPLEFS_INO_TABLE: SpinLock<BTreeMap<Sector, InodeNumber>> =
    SpinLock::new(BTreeMap::new());

/// The root directory of simple fs.
pub struct Root {
    fs: FileSystem,
}

impl keos::fs::traits::FileSystem for FileSystem {
    fn root(&self) -> Option<Directory> {
        Some(Directory::new(Root { fs: self.clone() }))
    }
}

impl keos::fs::traits::RegularFile for super::File<FsDisk> {
    fn ino(&self) -> InodeNumber {
        let mut ino_table = GLOBAL_SIMPLEFS_INO_TABLE.lock();
        let result = if let Some(ino) = ino_table.get(&self.start_sector) {
            *ino
        } else {
            let ino = InodeNumber::new(GLOBAL_SIMPLEFS_INO_COUNTER.fetch_add(1)).unwrap();

            ino_table.insert(self.start_sector, ino);
            ino
        };
        ino_table.unlock();
        result
    }

    fn size(&self) -> usize {
        self.size
    }

    fn read(&self, fba: FileBlockNumber, buf: &mut [u8; 4096]) -> Result<bool, keos::KernelError> {
        self.read(fba.0 * 4096, buf)
            .map_err(|e| match e {
                Error::DiskError => keos::KernelError::IOError,
                Error::FsError => {
                    keos::KernelError::FilesystemCorrupted("SimpleFS is in invalid state.")
                }
            })
            .map(|size| if size == 0 { false } else { true })
    }

    fn write(
        &self,
        fba: FileBlockNumber,
        buf: &[u8; 4096],
        _min_size: usize,
    ) -> Result<(), keos::KernelError> {
        self.write(fba.0 * 4096, buf)
            .map_err(|e| match e {
                Error::DiskError => keos::KernelError::IOError,
                Error::FsError => {
                    keos::KernelError::FilesystemCorrupted("SimpleFS is in invalid state.")
                }
            })
            .map(|_| ())
    }

    fn writeback(&self) -> Result<(), keos::KernelError> {
        Ok(())
    }
}

impl Drop for FsDisk {
    fn drop(&mut self) {
        let mut guard = GLOBAL_SIMPLEFS_INO_TABLE.lock();
        guard.clear();
        guard.unlock();
    }
}

impl keos::fs::traits::Directory for Root {
    fn ino(&self) -> InodeNumber {
        InodeNumber::new(1).unwrap()
    }

    fn size(&self) -> usize {
        0x1000
    }

    fn link_count(&self) -> usize {
        2
    }

    fn open_entry(&self, entry: &str) -> Result<File, keos::KernelError> {
        let fs = self.fs.clone();
        fs.0.open(entry)
            .map(|file| File::RegularFile(RegularFile::new(file)))
            .ok_or(keos::KernelError::NoSuchEntry)
    }

    fn create_entry(
        &self,
        _entry: &str,
        _is_dir: bool,
    ) -> Result<keos::fs::File, keos::KernelError> {
        Err(keos::KernelError::NotSupportedOperation)
    }

    fn unlink_entry(&self, _entry: &str) -> Result<(), keos::KernelError> {
        Err(keos::KernelError::NotSupportedOperation)
    }

    fn read_dir(&self) -> Result<Vec<(InodeNumber, String)>, keos::KernelError> {
        Err(keos::KernelError::NotSupportedOperation)
    }

    fn removed(&self) -> Result<&keos::sync::atomic::AtomicBool, keos::KernelError> {
        Err(keos::KernelError::NotSupportedOperation)
    }
}
