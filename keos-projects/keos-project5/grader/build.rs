use core::ops::Range;
use std::env;
use std::process::Command;
//use std::fs::OpenOptions;
//use std::os::unix::fs::FileExt;

include!("../../build.rs");

const M: u64 = 1024 * 1024;
pub struct Sector(pub usize);

#[repr(C)]
#[derive(Debug)]
pub struct SuperBlock {
    pub block_count: usize,
    pub block_count_inused: usize,
    pub inode_count: usize,
    pub inode_count_inused: usize,
    pub has_journal: usize,
}

#[repr(transparent)]
#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub struct InodeNumber(u32);

impl InodeNumber {
    pub fn into_lba_offset(self, sb: &SuperBlock) -> Option<(LogicalBlockAddress, usize)> {
        if self.0 != 0 && (self.0 as usize) < sb.inode_count {
            let index = (self.0 - 1) as usize;
            let inode_size = 128;

            debug_assert_eq!(inode_size, 128);
            let inode_per_block = 0x1000 / inode_size;

            Some((
                sb.inode().start + LogicalBlockAddress(index / inode_per_block),
                inode_size * (index % inode_per_block),
            ))
        } else {
            None
        }
    }

    /// # Returns
    /// Return the tuple of (LBA of Bitmap, byte-level offset, bit-level offset)
    pub fn into_inode_bitmap_offset(
        self,
        sb: &SuperBlock,
    ) -> Option<(LogicalBlockAddress, usize, usize)> {
        if self.0 != 0 && (self.0 as usize) < sb.inode_count {
            let index = (self.0 - 1) as usize;

            Some((
                sb.inode_bitmap().start + LogicalBlockAddress(index / 0x1000),
                index % 0x1000 / 8,
                index % 0x1000 % 8,
            ))
        } else {
            None
        }
    }
}

impl SuperBlock {
    pub const ROOT_INODE_NUMBER: InodeNumber = InodeNumber(1);

    #[inline]
    pub fn inode_bitmap(&self) -> Range<LogicalBlockAddress> {
        let begin = LogicalBlockAddress(2);
        begin..begin + LogicalBlockAddress(self.inode_count.div_ceil(8).div_ceil(0x1000))
    }

    #[inline]
    pub fn block_bitmap(&self) -> Range<LogicalBlockAddress> {
        let begin = self.inode_bitmap().end;
        begin..begin + LogicalBlockAddress(self.block_count.div_ceil(8).div_ceil(0x1000))
    }

    #[inline]
    pub fn journal(&self) -> Range<LogicalBlockAddress> {
        let begin = self.block_bitmap().end;
        begin..begin + LogicalBlockAddress(3 + 4095)
    }

    #[inline]
    pub fn inode(&self) -> Range<LogicalBlockAddress> {
        let begin = self.journal().end;
        begin..begin + LogicalBlockAddress((256 * self.inode_count).div_ceil(0x1000))
    }

    #[inline]
    pub fn data_block_start(&self) -> LogicalBlockAddress {
        self.inode().end
    }
}

/* KeOS FFS Configuration */
const FFS_MAGIC: &[u8; 8] = b"KeOSFFS\0";
const JOUR_MAGIC: &[u8; 8] = b"KeOSJOUR";

const SUPERBLOCK_TO_WRT: SuperBlock = SuperBlock {
    block_count: 250000,
    block_count_inused: 1,
    inode_count: 32768,
    inode_count_inused: 1,
    has_journal: 1,
};

#[repr(transparent)]
#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub struct LogicalBlockAddress(pub usize);

impl LogicalBlockAddress {
    pub const fn into_sector(self) -> Sector {
        // LBA is starting from 1. Zero represents the invalid LBA.
        Sector((self.0 - 1) * (0x1000 / 512))
    }
}

impl core::ops::Add for LogicalBlockAddress {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self(self.0 + rhs.0)
    }
}

const FFSI_MAGIC: &[u8; 8] = b"KeOSFFSI";

fn main() {
    let disk = "ffs.bin";
    let _ = std::fs::remove_file(disk);

    let size: u64 = 1024 * 1024 * 1024; // XXX: INTERIM!!!

    let disk_size = (size.div_ceil(M) + 1) * M;
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .open(disk)
        .expect("Failed to create file.");
    file.set_len(disk_size).unwrap();

    // Writing Superblock
    file.write_at(FFS_MAGIC, 0).unwrap();
    file.write_at(&SUPERBLOCK_TO_WRT.block_count.to_le_bytes(), 8)
        .unwrap();
    file.write_at(&SUPERBLOCK_TO_WRT.block_count_inused.to_le_bytes(), 16)
        .unwrap();
    file.write_at(&SUPERBLOCK_TO_WRT.inode_count.to_le_bytes(), 24)
        .unwrap();
    file.write_at(&SUPERBLOCK_TO_WRT.inode_count_inused.to_le_bytes(), 32)
        .unwrap();
    file.write_at(&SUPERBLOCK_TO_WRT.has_journal.to_le_bytes(), 40)
        .unwrap();

    // Writing Journal Superblock
    file.write_at(
        JOUR_MAGIC,
        (512 * SUPERBLOCK_TO_WRT.journal().start.into_sector().0) as u64,
    )
    .unwrap();

    // Fill Root Directory Inode (inode 1)
    let offset: (LogicalBlockAddress, usize) = SuperBlock::ROOT_INODE_NUMBER
        .into_lba_offset(&SUPERBLOCK_TO_WRT)
        .unwrap();
    file.write_at(FFSI_MAGIC, (512 * offset.0.into_sector().0) as u64)
        .unwrap();
    file.write_at(
        &1_u32.to_le_bytes(), // ino
        (512 * offset.0.into_sector().0 + 8) as u64,
    )
    .unwrap();
    file.write_at(
        &1_u32.to_le_bytes(), // filetype
        (512 * offset.0.into_sector().0 + 12) as u64,
    )
    .unwrap();
    file.write_at(
        &0x1000_usize.to_le_bytes(), // size
        (512 * offset.0.into_sector().0 + 16) as u64,
    )
    .unwrap();
    file.write_at(
        &0x2_usize.to_le_bytes(), // link_count,
        (512 * offset.0.into_sector().0 + 24) as u64,
    )
    .unwrap();
    file.write_at(
        &SUPERBLOCK_TO_WRT.data_block_start().0.to_le_bytes(), // LBA of dblocks[0]
        (512 * offset.0.into_sector().0 + 32) as u64,
    )
    .unwrap();

    // Fill Root Directory Entry (inode 1)
    let inode_bitmap = SuperBlock::ROOT_INODE_NUMBER
        .into_inode_bitmap_offset(&SUPERBLOCK_TO_WRT)
        .unwrap();
    let mut bitmap_data: [u8; 0x1000] = [0u8; 0x1000];
    bitmap_data[inode_bitmap.1] = 1 << inode_bitmap.2;
    file.write_at(&bitmap_data, (512 * inode_bitmap.0.into_sector().0) as u64)
        .unwrap();

    let mut block_bitmap = SUPERBLOCK_TO_WRT.block_bitmap().start;
    let mut bitmap_data = [0u8; 0x1000];
    for i in 0..=SUPERBLOCK_TO_WRT.data_block_start().0 {
        if i != 0 && i % 0x8000 == 0 {
            file.write_at(&bitmap_data, (512 * block_bitmap.into_sector().0) as u64)
                .unwrap();
            block_bitmap = block_bitmap + LogicalBlockAddress(1);
        }
        bitmap_data[i / 8] |= 1 << (i % 8);
    }
    file.write_at(&bitmap_data, (512 * block_bitmap.into_sector().0) as u64)
        .unwrap();
    let root_data_blk = SUPERBLOCK_TO_WRT.data_block_start();
    // -> "."
    file.write_at(
        &1_u32.to_le_bytes(), // ino = Some(1)
        root_data_blk.into_sector().0 as u64 * 512,
    )
    .unwrap();
    file.write_at(
        &1_u8.to_le_bytes(), // file_name_length = 1
        root_data_blk.into_sector().0 as u64 * 512 + 4,
    )
    .unwrap();
    file.write_at(
        b".", // file_name = "."
        root_data_blk.into_sector().0 as u64 * 512 + 5,
    )
    .unwrap();

    // -> ".."
    file.write_at(
        &1_u32.to_le_bytes(), // ino = Some(1)
        root_data_blk.into_sector().0 as u64 * 512 + 256,
    )
    .unwrap();
    file.write_at(
        &2_u8.to_le_bytes(), // file_name_length = 2
        root_data_blk.into_sector().0 as u64 * 512 + 260,
    )
    .unwrap();
    file.write_at(
        b"..", // file_name = ".."
        root_data_blk.into_sector().0 as u64 * 512 + 261,
    )
    .unwrap();

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=sfs.bin");
    println!("cargo:rerun-if-changed=ffs.bin");
    println!("cargo:rerun-if-changed=rootfs");

    let user_dir = Path::new(&env::var("CARGO_MANIFEST_DIR").unwrap()).join("userprog");

    let output = Command::new("make")
        .current_dir(&user_dir)
        .output()
        .expect("Failed to execute make");

    if !output.status.success() {
        panic!("make failed: {}", String::from_utf8_lossy(&output.stderr));
    }

    build_simple_fs("sfs.bin");
}
