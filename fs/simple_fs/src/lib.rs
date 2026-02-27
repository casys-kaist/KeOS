//! A very simple fs.
//!
//! This file system have following structure:
//! ```
//! [Offset 0]
//! "SIMPLEFS" (8 bytes)
//! disk_size (8 bytes)
//! [Offset 512]
//! file_name_len_0 (8 bytes)
//! file_size_0 (8 bytes)
//! file_name_0 (file_name_len_byte_0 bytes)
//! pad_to_512
//! [Offset 1024]
//! File_contents
//! [Offset align_up(file_size_0 + 512, 512)]
//! file_name_len_1 (8 bytes)
//! file_size_1 (8 bytes)
//! file_name_1 (file_name_len_byte_1 bytes)
//! pad_to_512
//! [Offset 1536]
//! File_contents
//! ...
//! ```
//!
//! This file system only have file abstraction (**NO DIRECTORY!!**) and the
//! file can only be read, overwrite.

#![allow(unexpected_cfgs)]
#![feature(iter_array_chunks)]
#![cfg_attr(all(not(feature = "std"), not(test)), no_std)]
#[cfg(feature = "keos_binder")]
mod keos_binder;
#[cfg(feature = "keos_binder")]
pub use keos_binder::FileSystem;

extern crate alloc;
use alloc::{boxed::Box, string::String, sync::Arc};

pub const FS_NAME: &str = "simple_fs";

/// A utilties to read/write bytes to u8 slice.
#[doc(hidden)]
pub struct ByteRw<'a> {
    b: &'a mut [u8],
}

impl<'a> ByteRw<'a> {
    /// Create a new ByteRw object.
    pub fn new(b: &'a mut [u8]) -> Self {
        Self { b }
    }
    /// Read u8 from position `p`.
    #[inline]
    pub fn read_u8(&self, p: usize) -> u8 {
        self.b.as_ref()[p]
    }
    /// Read u16 from position `p`.
    #[inline]
    pub fn read_u16(&self, p: usize) -> u16 {
        u16::from_le_bytes(self.b.as_ref()[p..p + 2].try_into().unwrap())
    }
    /// Read u32 from position `p`.
    #[inline]
    pub fn read_u32(&self, p: usize) -> u32 {
        u32::from_le_bytes(self.b.as_ref()[p..p + 4].try_into().unwrap())
    }
    /// Read u64 from position `p`.
    #[inline]
    pub fn read_u64(&self, p: usize) -> u64 {
        u64::from_le_bytes(self.b.as_ref()[p..p + 8].try_into().unwrap())
    }
    /// Write u8 from position `p`.
    #[inline]
    pub fn write_u8(&mut self, p: usize, v: u8) {
        self.b.as_mut()[p] = v;
    }
    /// Write u16 from position `p`.
    #[inline]
    pub fn write_u16(&mut self, p: usize, v: u16) {
        self.b.as_mut()[p..p + 2].copy_from_slice(&u16::to_le_bytes(v))
    }
    /// Write u32 from position `p`.
    #[inline]
    pub fn write_u32(&mut self, p: usize, v: u32) {
        self.b.as_mut()[p..p + 4].copy_from_slice(&u32::to_le_bytes(v))
    }
    /// Write u64 from position `p`.
    #[inline]
    pub fn write_u64(&mut self, p: usize, v: u64) {
        self.b.as_mut()[p..p + 8].copy_from_slice(&u64::to_le_bytes(v))
    }
    /// Get underlying buffer as reference.
    #[inline]
    pub fn inner(&self) -> &[u8] {
        &self.b
    }
    /// Get underlying buffer as mutable reference.
    #[inline]
    pub fn inner_mut(&mut self) -> &mut [u8] {
        &mut self.b
    }
}

/// Sector.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct Sector(pub usize);

impl Sector {
    /// Get offset that represented by the sector.
    #[inline]
    pub fn into_offset(self) -> usize {
        self.0 * 512
    }

    /// Cast into usize.
    #[inline]
    pub fn into_usize(self) -> usize {
        self.0
    }
}

/// Possible error kinds.
#[derive(Debug)]
pub enum Error {
    /// Disk operation has an error.
    DiskError,
    /// File system operation has an error.
    FsError,
}

/// A device that has byte sink.
pub trait Disk {
    /// Read 512 bytes from disk starting from sector.
    fn read(&self, sector: Sector, buf: &mut [u8; 512]) -> Result<(), Error>;
    /// Write 512 bytes to disk starting from sector.
    fn write(&self, sector: Sector, buf: &[u8; 512]) -> Result<(), Error>;
}

/// The root file system
#[derive(Debug)]
pub struct SimpleFs<T: Disk> {
    t: T,
    size: usize,
}

impl<T: Disk> SimpleFs<T> {
    /// Create a new file system to disk.
    pub fn new(t: T, size: usize) -> Result<Self, Error> {
        // Set header
        let mut buf = Box::new([0; 512]);
        let mut rw = ByteRw::new(buf.as_mut());
        rw.inner_mut()[0..8].copy_from_slice(b"SIMPLEFS");
        rw.write_u64(8, size as u64);
        drop(rw);
        t.write(Sector(0), buf.as_ref())?;

        let this = Self { t, size };
        this.write_file_header(Sector(1), "", size - 512 * 2)?;

        // Cleanup buf
        buf.fill(0);
        for i in 2..(size / 512) {
            this.t.write(Sector(i), buf.as_ref())?;
        }
        Ok(this)
    }
    /// Load a file system from disk
    pub fn load(t: T) -> Result<Self, Error> {
        let mut buf = Box::new([0; 512]);
        t.read(Sector(0), buf.as_mut())?;
        let mut rw = ByteRw::new(buf.as_mut());
        if &rw.inner_mut()[0..8] != b"SIMPLEFS" {
            return Err(Error::FsError);
        }
        let size = rw.read_u64(8) as usize;
        Ok(Self { t, size })
    }

    fn write_file_header(&self, sector: Sector, name: &str, size: usize) -> Result<(), Error> {
        let mut buf = Box::new([0; 512]);
        let mut rw = ByteRw::new(buf.as_mut());
        let name_len = name.len();
        if name_len > 512 - 16 {
            return Err(Error::FsError);
        }
        rw.write_u64(0, name_len as u64);
        rw.write_u64(8, size as u64);
        rw.inner_mut()[16..16 + name_len].copy_from_slice(name.as_bytes());
        drop(rw);

        self.t.write(sector, buf.as_ref())
    }

    /// Open a file with `name`.
    pub fn open(self: &Arc<Self>, name: &str) -> Option<File<T>> {
        if name.len() == 0 {
            return None;
        }
        let mut buf = Box::new([0; 512]);
        let mut pos = 1;
        while pos < self.size / 512 {
            self.t.read(Sector(pos), buf.as_mut()).ok()?;
            let rw = ByteRw::new(buf.as_mut());
            let len = rw.read_u64(0) as usize;
            let fname = core::str::from_utf8(&rw.inner()[16..16 + len]).ok()?;
            if fname == name {
                return Some(File {
                    name: String::from(name),
                    size: rw.read_u64(8) as usize,
                    start_sector: Sector(pos),
                    fs: self.clone(),
                });
            }
            let this_segment_size = ((rw.read_u64(8) + 511) & !511) as usize;
            pos += 1 + this_segment_size / 512;
        }
        None
    }

    /// Create a file that contains `contents`.
    pub fn create(&mut self, name: &str, contents: &[u8]) -> Result<(), Error> {
        if name.len() == 0 {
            return Err(Error::FsError);
        }
        let file_size = contents.len();
        let required = (file_size + 511) & !511;
        let mut buf = Box::new([0; 512]);
        let mut pos = 1;
        while pos < self.size / 512 {
            self.t.read(Sector(pos), buf.as_mut())?;
            let rw = ByteRw::new(buf.as_mut());
            let this_segment_size = ((rw.read_u64(8) + 511) & !511) as usize;
            if rw.read_u64(0) == 0 && this_segment_size >= required {
                if this_segment_size != required {
                    // split
                    let nseg_size = this_segment_size - required - 512;
                    self.write_file_header(Sector(pos + 1 + required / 512), "", nseg_size)?;
                }

                let mut content_pos = pos + 1;
                let mut chunks = contents.chunks_exact(512);
                while let Some(chunk) = chunks.next() {
                    self.t
                        .write(Sector(content_pos), chunk.try_into().unwrap())?;
                    content_pos += 1;
                }
                let remainder = chunks.remainder();
                if remainder.len() != 0 {
                    buf[..remainder.len()].copy_from_slice(remainder);
                    buf[remainder.len()..].fill(0);
                    self.t.write(Sector(content_pos), buf.as_ref())?;
                    content_pos += 1;
                }
                self.write_file_header(Sector(pos), name, file_size)?;
                assert_eq!(content_pos, pos + 1 + required / 512);
                return Ok(());
            } else {
                pos += 1 + this_segment_size / 512;
            }
        }
        Err(Error::FsError)
    }

    /// Close this filesystem.
    #[inline]
    pub fn close(self) -> T {
        self.t
    }
}

/// The file.
#[derive(Debug)]
pub struct File<T: Disk> {
    name: String,
    size: usize,
    start_sector: Sector,
    fs: Arc<SimpleFs<T>>,
}

impl<T: Disk> File<T> {
    /// Get name of this file.
    #[inline]
    pub fn name(&self) -> &str {
        self.name.as_ref()
    }
    /// Get size of this file.
    #[inline]
    pub fn size(&self) -> usize {
        self.size
    }

    /// Read from file starting from `ofs` to `contents`.
    pub fn read(&self, ofs: usize, contents: &mut [u8]) -> Result<usize, Error> {
        let len = contents.len().min(self.size.saturating_sub(ofs));
        let contents = &mut contents[..len];
        let mut buf = Box::new([0; 512]);
        let mut pos = self.start_sector.0 + 1 + ofs / 512;
        let sofs = if ofs % 512 != 0 { 512 - ofs % 512 } else { 0 };
        if len == 0 {
            return Ok(0);
        }

        // First unaligned
        if ofs % 512 != 0 {
            self.fs.t.read(Sector(pos), buf.as_mut())?;
            let this_len = sofs.min(len);
            contents[..this_len].copy_from_slice(&buf[ofs % 512..ofs % 512 + this_len]);
            pos += 1;
        }

        if len > sofs {
            let mut chunks = contents[sofs..len].chunks_exact_mut(512);
            while let Some(chunk) = chunks.next() {
                self.fs.t.read(Sector(pos), buf.as_mut())?;
                chunk.copy_from_slice(buf.as_ref());
                pos += 1;
            }
            let remainder = chunks.into_remainder();
            if remainder.len() != 0 {
                self.fs.t.read(Sector(pos), buf.as_mut())?;
                remainder.copy_from_slice(&buf.as_ref()[..remainder.len()]);
            }
        }

        Ok(len)
    }

    /// Write to file starting from `ofs` from `contents`.
    pub fn write(&self, ofs: usize, contents: &[u8]) -> Result<usize, Error> {
        let len = contents.len().min(self.size.saturating_sub(ofs));
        let contents = &contents[..len];
        let mut buf = Box::new([0; 512]);
        let mut pos = self.start_sector.0 + 1 + ofs / 512;
        let sofs = if ofs % 512 != 0 { 512 - ofs % 512 } else { 0 };
        // First unaligned
        if ofs % 512 != 0 {
            self.fs.t.read(Sector(pos), buf.as_mut())?;
            let this_len = sofs.min(len);
            buf[ofs % 512..ofs % 512 + this_len].copy_from_slice(&contents[..this_len]);
            self.fs.t.write(Sector(pos), buf.as_ref())?;
            pos += 1;
        }

        if len > sofs {
            let mut chunks = contents[sofs..len].chunks_exact(512);
            while let Some(chunk) = chunks.next() {
                buf.copy_from_slice(chunk);
                self.fs.t.write(Sector(pos), buf.as_ref())?;
                pos += 1;
            }
            let remainder = chunks.remainder();
            if remainder.len() != 0 {
                buf[..remainder.len()].copy_from_slice(remainder);
                self.fs.t.write(Sector(pos), buf.as_ref())?;
            }
        }
        Ok(len)
    }
}

#[cfg(not(all(not(feature = "std"), not(test))))]
mod tests {
    use super::*;
    use rand::distributions::Alphanumeric;
    use rand::{Rng, thread_rng};
    use std::fs::OpenOptions;
    use std::os::unix::fs::FileExt;

    impl Disk for FileDisk {
        fn read(&self, sector: Sector, buf: &mut [u8; 512]) -> Result<(), Error> {
            self.file
                .read_at(buf.as_mut(), sector.into_offset() as u64)
                .map_err(|_| Error::DiskError)
                .map(|_| ())
        }
        fn write(&self, sector: Sector, buf: &[u8; 512]) -> Result<(), Error> {
            self.file
                .write_at(buf.as_ref(), sector.into_offset() as u64)
                .map_err(|_| Error::DiskError)
                .map(|_| ())
        }
    }

    struct FileDisk {
        file: std::fs::File,
        fname: std::path::PathBuf,
    }

    impl FileDisk {
        fn new() -> Self {
            let mut fname = std::path::PathBuf::new();
            fname.push(r"/tmp");
            fname.push(format!(
                "{}.disk",
                thread_rng()
                    .sample_iter(&Alphanumeric)
                    .take(8)
                    .map(char::from)
                    .collect::<String>()
            ));
            let file = OpenOptions::new()
                .read(true)
                .write(true)
                .create_new(true)
                .open(fname.as_path())
                .expect("Failed to create file.");
            Self { file, fname }
        }
    }

    impl Drop for FileDisk {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.fname);
        }
    }

    #[test]
    fn test_simple() {
        let mut fs = SimpleFs::new(FileDisk::new(), 512 * 0x1000).unwrap();
        // create test
        let content = (0..0x3ff).map(|i| i as u8).collect::<Vec<_>>();
        assert!(fs.create("a", content.as_ref()).is_ok());
        assert!(fs.create("b", content.as_ref()).is_ok());

        let fs = SimpleFs::load(fs.close()).unwrap();
        // Read test
        let mut readbuf = vec![0; 0x3ff];

        let a = fs.open("a").unwrap();
        for i in 0..content.len() {
            for j in 1..content.len() - i {
                a.read(i, &mut readbuf[..j]).unwrap();
                assert_eq!(&readbuf[..j], &content[i..i + j]);
            }
        }

        let b = fs.open("b").unwrap();
        for i in 0..content.len() {
            for j in 1..content.len() - i {
                b.read(i, &mut readbuf[..j]).unwrap();
                assert_eq!(&readbuf[..j], &content[i..i + j]);
            }
        }
        // Write test
        let content = (0..0x3ff).map(|i| (0x3ff - i) as u8).collect::<Vec<_>>();
        let a = fs.open("a").unwrap();
        for i in 0..content.len() {
            for j in 1..content.len() - i {
                a.write(i, &content[i..i + j]).unwrap();
                a.read(i, &mut readbuf[..j]).unwrap();
                assert_eq!(&readbuf[..j], &content[i..i + j]);
            }
        }

        let b = fs.open("b").unwrap();
        for i in 0..content.len() {
            for j in 1..content.len() - i {
                b.write(i, &content[i..i + j]).unwrap();
                b.read(i, &mut readbuf[..j]).unwrap();
                assert_eq!(&readbuf[..j], &content[i..i + j]);
            }
        }
        let fs = SimpleFs::load(fs.close()).unwrap();
        // Read test - persistent
        let mut readbuf = vec![0; 0x3ff];

        let a = fs.open("a").unwrap();
        for i in 0..content.len() {
            for j in 1..content.len() - i {
                a.read(i, &mut readbuf[..j]).unwrap();
                assert_eq!(&readbuf[..j], &content[i..i + j]);
            }
        }

        let b = fs.open("b").unwrap();
        for i in 0..content.len() {
            for j in 1..content.len() - i {
                b.read(i, &mut readbuf[..j]).unwrap();
                assert_eq!(&readbuf[..j], &content[i..i + j]);
            }
        }
    }
}
