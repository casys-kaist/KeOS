use alloc::{borrow::ToOwned, boxed::Box};
use keos::{
    KernelError,
    fs::{Disk, FileSystem, InodeNumber, RegularFile},
    println,
};
use keos_project2::loader::LoadContext;
use keos_project5::{ffs, page_cache::PageCache};

pub fn root() {
    // The only requirement is not to panic.
    let fs = ffs::FastFileSystem::from_disk(Disk::new(2), true, true).unwrap();
    FileSystem::register(PageCache::new(fs));
    let _ = FileSystem::root();
}

pub fn root_open_self() {
    let fs = ffs::FastFileSystem::from_disk(Disk::new(2), true, true).unwrap();
    FileSystem::register(PageCache::new(fs));

    let root = FileSystem::root();

    root.open(".")
        .expect("Opening `.' must succeed as it means itself.")
        .into_directory()
        .expect("`.` should be a directory.");

    root.open("..")
        .expect("Opening `..' must succeed as root's parent is itself.")
        .into_directory()
        .expect("`..` should be a directory.");
}

pub fn root_open_absent() {
    let fs = ffs::FastFileSystem::from_disk(Disk::new(2), true, true).unwrap();
    FileSystem::register(PageCache::new(fs));
    let root = FileSystem::root();

    assert_eq!(
        root.open("nonexistant").map(|_| ()),
        Err(KernelError::NoSuchEntry),
        "Opening `nonexistant' must be fail as it was never created."
    );
}

pub fn add_file() {
    println!();
    let fs = ffs::FastFileSystem::from_disk(Disk::new(2), true, true).unwrap();
    FileSystem::register(PageCache::new(fs));
    let root = FileSystem::root();

    keos::info!("Creating the file `add_file_with_journal' on the root directory");
    let created = root.create("add_file_with_journal", false).unwrap();

    let idempotent = root
        .open(".")
        .expect("Opening `.' must succeed as it means itself.")
        .into_directory()
        .unwrap();

    let f = idempotent
        .open("add_file_with_journal")
        .expect("Created file `add_file_with_journal' must be present on the root directory.");

    let f = f
        .into_regular_file()
        .expect("Created file `add_file_with_journal' must be a RegularFile");

    assert_eq!(created.ino(), f.ino(),);
}

pub fn ib() {
    println!();
    let fs = ffs::FastFileSystem::from_disk(Disk::new(2), true, true).unwrap();
    FileSystem::register(PageCache::new(fs));
    let root = FileSystem::root();

    let mut buf: [u8; 4096] = [0u8; 4096];
    buf[..104].copy_from_slice(b"After using 12 direct blocks, the indirect block is used to cover until 524th block that is about 2 MiB.");

    let file = root
        .create("ib_with_journal", false)
        .unwrap()
        .into_regular_file()
        .unwrap();

    keos::info!(
        "Extending the size of `ib_with_journal' to 48 KiB which uses all of direct blocks"
    );
    file.write(11 * 0x1000, &buf).unwrap();

    keos::info!(
        "Extending the size of `ib_with_journal' to 52 KiB which uses first entry of indirect blocks"
    );
    file.write(12 * 0x1000, &buf).unwrap();

    keos::info!(
        "Extending the size of `ib_with_journal' to 56 KiB which uses second entry of indirect blocks"
    );
    file.write(13 * 0x1000, &buf).unwrap();

    file.writeback().unwrap();

    for fbn in 12..=13 {
        let mut read_buf: [u8; 4096] = [0u8; 4096];

        assert!(file.read(fbn * 0x1000, &mut read_buf).is_ok());

        assert_eq!(
            &read_buf[..104],
            b"After using 12 direct blocks, the indirect block is used to cover until 524th block that is about 2 MiB."
        );
    }
}

pub fn dib() {
    println!();
    let fs = ffs::FastFileSystem::from_disk(Disk::new(2), true, true).unwrap();
    FileSystem::register(PageCache::new(fs));
    let root = FileSystem::root();

    let mut buf: [u8; 4096] = [0u8; 4096];
    buf[..169].copy_from_slice(b"After using 12 direct blocks and 524th indirect block, the doubly indirect block is used to cover until 262668th block that is maximum block of a single file of KeOSFFS.");

    let file = root
        .create("dib_with_journal", false)
        .unwrap()
        .into_regular_file()
        .unwrap();

    keos::info!(
        "Extending the size of `dib_with_journal' to 2092 KiB which uses all of direct blocks and indirect blocks"
    );
    for fbn in (12..524).step_by(64) {
        file.write(fbn * 0x1000, &buf).unwrap();
    }

    keos::info!(
        "Extending the size of `dib_with_journal' to 2096 KiB which uses first entry of doubly indirect blocks"
    );
    file.write(524 * 0x1000, &buf).unwrap();
    file.writeback().unwrap();

    for fbn in 524..=524 {
        let mut read_buf: [u8; 4096] = [0u8; 4096];

        assert!(file.read(fbn * 0x1000, &mut read_buf).is_ok(),);

        assert_eq!(
            &read_buf[..169],
            b"After using 12 direct blocks and 524th indirect block, the doubly indirect block is used to cover until 262668th block that is maximum block of a single file of KeOSFFS."
        );
    }
}

pub fn add_directory() {
    println!();
    let fs = ffs::FastFileSystem::from_disk(Disk::new(2), true, true).unwrap();
    FileSystem::register(PageCache::new(fs));
    let root = FileSystem::root();

    keos::info!("Creating the directory `add_directory_with_journal' on the root directory");
    let created = root.create("add_directory_with_journal", true).unwrap();

    let idempotent = root
        .open(".")
        .expect("Reading `.' must succeed")
        .into_directory()
        .unwrap();

    let f = idempotent
        .open("add_directory_with_journal")
        .expect("Created file `add_directory_with_journal' must be present on the directory.");

    let d = f
        .into_directory()
        .expect("Created file `add_directory_with_journal' must be a Directory");

    assert_eq!(created.ino(), d.ino(),);
}

pub fn file_in_dir() {
    println!();
    let fs = ffs::FastFileSystem::from_disk(Disk::new(2), true, true).unwrap();
    FileSystem::register(PageCache::new(fs));
    let root = FileSystem::root();
    let mut buffer: [u8; 4096] = [0u8; 4096];

    let root_file = root
        .create("file_in_dir__file_with_journal", false)
        .unwrap()
        .into_regular_file()
        .unwrap();

    buffer[..33].copy_from_slice(b"I am a file in the root directory");
    root_file.write(0, &buffer).unwrap();

    let dir = root
        .create("file_in_dir__directory_with_journal", true)
        .unwrap()
        .into_directory()
        .unwrap();

    assert_eq!(
        dir.open("file_in_dir__file_with_journal").map(|_| ()),
        Err(KernelError::NoSuchEntry),
        "Opening `file_in_dir__file_with_journal' in `file_in_dir__directory_with_journal' must be fail as it was never created."
    );

    dir.create("file_in_dir__file_with_journal", false)
        .expect("Creating `file_in_dir__file_with_journal' in `file_in_dir__directory_with_journal' must be succeed");

    let dir_file = dir
        .open("file_in_dir__file_with_journal")
        .expect("Created file `file_in_dir__file_with_journal' must be present on `file_in_dir__directory_with_journal'.")
        .into_regular_file()
        .unwrap();

    buffer[..33].copy_from_slice(b"This is not a file of root folder");
    dir_file.write(0, &buffer).unwrap();

    root_file.read(0, &mut buffer).unwrap();

    assert_eq!(&buffer[..33], b"I am a file in the root directory");

    dir_file.read(0, &mut buffer).unwrap();

    assert_eq!(&buffer[..33], b"This is not a file of root folder");
}

pub fn remove_file() {
    let mut buffer: [u8; 4096] = [0u8; 4096];
    let fs = ffs::FastFileSystem::from_disk(Disk::new(2), true, true).unwrap();
    FileSystem::register(PageCache::new(fs));
    let root = FileSystem::root();

    let file = root
        .create("remove_file_with_journal", false)
        .unwrap()
        .into_regular_file()
        .unwrap();
    file.write(0, b"Enjoying KeOS FFS?").unwrap();
    file.writeback().unwrap();
    keos::info!("Created file `remove_file_with_journal` into the root directory.");

    root.unlink("remove_file_with_journal").unwrap();
    keos::info!("Removed file `remove_file_with_journal' from the root directory.");
    file.read(0, &mut buffer)
        .expect("File must be readable even if it is unlinked");

    assert_eq!(
        &buffer[..18],
        b"Enjoying KeOS FFS?",
        "Read data must keep consist."
    );

    assert!(
        root.open("remove_file_with_journal").is_err(),
        "`file' is supposed to be absent after remove"
    );
}

pub fn read_dir() {
    let fs = ffs::FastFileSystem::from_disk(Disk::new(2), true, true).unwrap();
    keos::fs::FileSystem::register(PageCache::new(fs));
    let root = FileSystem::root();
    let dir = root
        .create("read_dir_with_journal", true)
        .unwrap()
        .into_directory()
        .unwrap();

    let mut expected_entries = alloc::vec![
        (dir.ino(), ".".to_owned()),
        (InodeNumber::new(1).unwrap(), "..".to_owned())
    ];

    assert_eq!(dir.read_dir().unwrap(), expected_entries);

    let temp_file = dir
        .create("temp", false)
        .unwrap()
        .into_regular_file()
        .unwrap();
    expected_entries.push((temp_file.ino(), "temp".to_owned()));

    assert_eq!(dir.read_dir().unwrap(), expected_entries);

    dir.unlink("temp").unwrap();
    expected_entries.pop();

    assert_eq!(dir.read_dir().unwrap(), expected_entries);
}

pub fn remove_root() {
    println!();
    let fs = ffs::FastFileSystem::from_disk(Disk::new(2), true, true).unwrap();
    FileSystem::register(PageCache::new(fs));
    let root = FileSystem::root();

    assert_eq!(
        root.unlink("."),
        Err(KernelError::Busy),
        "Removing `.` of root directory must fail."
    );

    assert_eq!(
        root.unlink(".."),
        Err(KernelError::Busy),
        "Removing `..` of root directory must fail."
    );
}

pub fn remove_dir() {
    let fs = ffs::FastFileSystem::from_disk(Disk::new(2), true, true).unwrap();
    FileSystem::register(PageCache::new(fs));
    let root = FileSystem::root();

    let dir = root
        .create("remove_dir_with_journal", true)
        .unwrap()
        .into_directory()
        .unwrap();

    dir.create("file", false).unwrap();
    drop(dir);
    keos::info!("Created directory `remove_dir_with_journal` with `file` inside.");

    assert_eq!(
        root.unlink("remove_dir_with_journal").map(|_| ()),
        Err(KernelError::DirectoryNotEmpty),
        "Removing `remove_dir_with_journal' without removing all entities in the directory must fail."
    );

    let dir = root
        .open("remove_dir_with_journal")
        .unwrap()
        .into_directory()
        .unwrap();
    dir.unlink("file").unwrap();

    drop(dir);

    root.unlink("remove_dir_with_journal")
        .expect("Deleting empty directory `remove_dir_with_journal' must succeed.");
}

pub fn simple_elf() {
    pub fn run_elf_regularfile(elf: &RegularFile, name: &str) -> i32 {
        let LoadContext { mm_struct, regs } = LoadContext {
            mm_struct: keos_project2::mm_struct::MmStruct::new(),
            regs: keos::syscall::Registers::new(),
        }
        .load(elf, &[name])
        .unwrap_or_else(|e| panic!("Failed to load elf: {:?}", e));

        let thread_build = keos::thread::ThreadBuilder::new(name);
        let tid = thread_build.get_tid();
        thread_build
            .attach_task(Box::new(keos_project4::Thread::from_mm_struct(
                mm_struct, tid,
            )))
            .spawn(move || regs.launch())
            .join()
    }

    println!();
    let fs = ffs::FastFileSystem::from_disk(Disk::new(2), true, true).unwrap();
    FileSystem::register(PageCache::new(fs));
    let root = FileSystem::root();

    let mut simple_elf: [u8; 4096] = [0u8; 4096];
    simple_elf[..128].copy_from_slice(&[
        0x7f, 0x45, 0x4c, 0x46, 0x02, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x02, 0x00, 0x3e, 0x00, 0x01, 0x00, 0x00, 0x00, 0x78, 0x00, 0x40, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x40, 0x00, 0x38, 0x00, 0x01, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x07, 0x00, 0x00, 0x00, 0x78, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x78, 0x00, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x08,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x48, 0x31, 0xc0, 0x6a, 0x42, 0x5f, 0x0f, 0x05,
    ]);

    let file = root
        .create("the_answer_with_journal", false)
        .unwrap()
        .into_regular_file()
        .unwrap();

    file.write(0, &simple_elf).unwrap();

    assert_eq!(
        run_elf_regularfile(&file, "the_answer_with_journal"),
        0x42,
        "The user program must return 0x42"
    );

    drop(file);
    root.unlink("the_answer_with_journal").unwrap();
}
