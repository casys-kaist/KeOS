use keos::{
    fs::{Disk, FileBlockNumber, RegularFile, traits::FileSystem},
    println,
};
use keos_project5::{
    ffs,
    page_cache::{PageCache, PageCacheState},
};

fn cache_exists(
    page_cache_state: &mut PageCacheState,
    file: RegularFile,
    fba: FileBlockNumber,
) -> bool {
    page_cache_state.get((file.0.ino(), fba)).is_some()
}

pub fn readahead() {
    println!();

    let fs = PageCache::new(simple_fs::FileSystem::load(1).unwrap());

    let root = fs.root().expect("Root directory must be present");

    let f: RegularFile = root
        .open("sha256sum")
        .expect("file `sha256sum' must be present on the root directory.")
        .into_regular_file()
        .expect("file `sha256sum' must be a RegularFile");

    let mut buffer: [u8; 4096] = [0u8; 4096];

    f.read(0, &mut buffer)
        .expect("Reading file `sha256sum' must succeed");

    let mut prime_count = 0;
    for num in 2..1000000 {
        let mut is_prime = true;

        let mut i = 2;
        while i * i <= num {
            if num % i == 0 {
                is_prime = false;
                break;
            }
            i += 1;
        }

        if is_prime {
            prime_count += 1;
        }
    }

    println!(
        "Waiting for read ahead. Number of primes found: {}",
        prime_count
    );

    let mut guard = fs.0.inner.lock();
    assert!(
        cache_exists(&mut guard, f, FileBlockNumber(1)),
        "File block 1 should be cached by
    read-ahead after reading block 0"
    );
    guard.unlock();

    // Prevent fs drop after the test finish
    keos::fs::FileSystem::register(fs);
}

pub fn readahead_ffs() {
    println!();
    let ffs = ffs::FastFileSystem::from_disk(Disk::new(2), false, false).unwrap();
    let fs: &dyn keos::fs::traits::FileSystem = &ffs;

    let root = fs.root().expect("Root directory must be present");

    let f: RegularFile = root
        .create("page_cache__readahead", false)
        .unwrap()
        .into_regular_file()
        .unwrap();

    let mut buffer: [u8; 4096] = [0u8; 4096];
    f.write(2 * 0x1000, &buffer).unwrap();
    f.writeback().unwrap();
    drop(f);

    let page_cache = PageCache::new(ffs);
    let fs: &dyn keos::fs::traits::FileSystem = &page_cache;

    let root = fs.root().expect("Root directory must be present");

    let f: RegularFile = root
        .open("page_cache__readahead")
        .expect("Created file `page_cache__readahead' must be present on the root directory.")
        .into_regular_file()
        .expect("Created file `page_cache__readahead' must be a RegularFile");

    f.read(0, &mut buffer)
        .expect("Reading file `page_cache__readahead' must succeed");

    let mut prime_count = 0;
    for num in 2..1000000 {
        let mut is_prime = true;

        let mut i = 2;
        while i * i <= num {
            if num % i == 0 {
                is_prime = false;
                break;
            }
            i += 1;
        }

        if is_prime {
            prime_count += 1;
        }
    }

    println!(
        "Waiting for read ahead. Number of primes found: {}",
        prime_count
    );

    let mut guard = page_cache.0.inner.lock();
    assert!(
        cache_exists(&mut guard, f, FileBlockNumber(1)),
        "File block 1 should be cached by
    read-ahead after reading block 0"
    );
    guard.unlock();

    // Prevent fs drop after the test finish
    keos::fs::FileSystem::register(page_cache);
}

pub fn fastfilesystem() {
    println!();
    let ffs = ffs::FastFileSystem::from_disk(Disk::new(2), false, false).unwrap();
    let fs: &dyn keos::fs::traits::FileSystem = &ffs;

    let root = fs.root().expect("Root directory must be present");

    let f: RegularFile = root
        .create("page_cache__fastfilesystem", false)
        .unwrap()
        .into_regular_file()
        .unwrap();

    let mut buffer: [u8; 4096] = [0u8; 4096];
    f.write(0, &buffer).unwrap();
    f.writeback().unwrap();
    drop(f);

    let page_cache = PageCache::new(ffs);
    let fs: &dyn keos::fs::traits::FileSystem = &page_cache;

    let root = fs.root().expect("Root directory must be present");
    let f: RegularFile = root
        .open("page_cache__fastfilesystem")
        .expect("Created file `page_cache__fastfilesystem' must be present on the root directory.")
        .into_regular_file()
        .expect("Created file `page_cache__fastfilesystem' must be a RegularFile");

    f.read(0, &mut buffer)
        .expect("Reading file `file' must succeed");

    let mut guard = page_cache.0.inner.lock();
    assert!(
        cache_exists(&mut guard, f, FileBlockNumber(0)),
        "File block must be cached after reading it"
    );
    guard.unlock();

    // Prevent fs drop after the test finish
    keos::fs::FileSystem::register(page_cache);
}

pub fn simplefs() {
    println!();
    let fs = simple_fs::FileSystem::load(1).unwrap();

    let page_cache = PageCache::new(fs);
    let fs: &dyn keos::fs::traits::FileSystem = &page_cache;

    let root = fs.root().expect("Root directory must be present");

    let f: RegularFile = root
        .open("os-release")
        .expect("file `os-release' must be present on the root directory.")
        .into_regular_file()
        .expect("file `os-release' must be a RegularFile");

    let mut buffer: [u8; 4096] = [0u8; 4096];
    f.read(0, &mut buffer)
        .expect("Reading file `os-release' must succeed");

    let mut guard = page_cache.0.inner.lock();
    assert!(
        cache_exists(&mut guard, f, FileBlockNumber(0)),
        "File block must be cached after reading it"
    );
    guard.unlock();

    // Prevent fs drop after the test finish
    keos::fs::FileSystem::register(page_cache);
}

pub fn writeback() {
    let ffs = ffs::FastFileSystem::from_disk(Disk::new(2), false, false).unwrap();
    let page_cache = PageCache::new(ffs.clone());
    let fs: &dyn keos::fs::traits::FileSystem = &page_cache;

    let root = fs.root().unwrap();

    let file = root
        .create("fsync__file", false)
        .unwrap()
        .into_regular_file()
        .unwrap();

    let mut buf = [0u8; 4096];
    buf[..18].copy_from_slice(b"Enjoying KeOS FFS?");
    file.write(0, &buf).unwrap();
    file.writeback().unwrap();

    file.read(0, &mut buf[..]).unwrap();
    buf[..18].copy_from_slice(b"Is this reflected?");
    file.write(0, &buf).unwrap();

    let inode = ffs.get_inode(file.ino()).unwrap();
    let lba = inode
        .read()
        .get(&ffs.0, FileBlockNumber(0))
        .unwrap()
        .unwrap();
    let mut buf = [0u8; 512];
    Disk::new(2).read(lba.into_sector(), &mut buf).unwrap();

    assert_ne!(
        &buf[..18],
        b"Is this reflected?",
        "Before writeback of page cache, the disk content should be left intact"
    );
    file.writeback().unwrap();

    Disk::new(2).read(lba.into_sector(), &mut buf).unwrap();

    assert_eq!(
        &buf[..18],
        b"Is this reflected?",
        "After writeback of page cache, the disk content should be reflected"
    );
}
