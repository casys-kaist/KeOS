use alloc::sync::Arc;
use keos::{
    KernelError,
    fs::{Disk, Sector, traits::FileSystem},
    sync::atomic::{AtomicBool, AtomicI32},
    thread::{Current, ThreadBuilder},
};
use keos_project5::ffs;

pub fn recovery() {
    static WRITE_COUNTER: AtomicI32 = AtomicI32::new(0);
    static IS_JOURNAL_SB_ALTERNATED: AtomicBool = AtomicBool::new(false);
    static COMMITTED: AtomicBool = AtomicBool::new(false);
    static DEST_WRITE_COUNTER: AtomicI32 = AtomicI32::new(0);

    WRITE_COUNTER.store(0);

    let limit_wc = Arc::new(|sector: Sector, data: &[u8; 512], write: bool| {
        if sector.0.is_multiple_of(8) && write {
            let lba = sector.0 / 8 + 1;
            if WRITE_COUNTER.fetch_add(1) == DEST_WRITE_COUNTER.load() {
                return Err(KernelError::IOError);
            }

            if lba == 11 {
                // FIXME: fix this to any method to directly knowing journal sb
                let committed = u64::from_le_bytes(data[8..16].try_into().unwrap());
                IS_JOURNAL_SB_ALTERNATED.store(true);
                COMMITTED.store(committed != 0);
            }
        }
        Ok(())
    });

    DEST_WRITE_COUNTER.store(0);
    loop {
        WRITE_COUNTER.store(0);
        let wc = DEST_WRITE_COUNTER.fetch_add(1) + 1;

        let cloned_limit_wc = limit_wc.clone();
        let writer = ThreadBuilder::new("writer").spawn(move || {
            let ffs =
                ffs::FastFileSystem::from_disk(Disk::new(2).hook(cloned_limit_wc), true, false)
                    .unwrap();
            let fs: &dyn keos::fs::traits::FileSystem = &ffs;
            let root = fs.root().unwrap();

            if let Err(e) = root.create("journal__test_file", false) {
                Current::exit(e.into_usize() as i32)
            } else {
                Current::exit(0)
            }
        });
        let writer_result = writer.join();
        keos::debug!(
            "create() with write count limit {} test: {:?}",
            wc,
            TryInto::<KernelError>::try_into(writer_result as isize)
        );
        if writer_result == 0 {
            break;
        }

        let verifier = ThreadBuilder::new("verifier").spawn(move || {
            let ffs = ffs::FastFileSystem::from_disk(Disk::new(2), true, false).unwrap();
            let root = ffs.root().unwrap();

            if let Err(e) = root.open("journal__test_file") {
                Current::exit(e.into_usize() as i32)
            }

            root.unlink("journal__test_file").unwrap();
            Current::exit(0)
        });

        let verifier_result = verifier.join();

        if COMMITTED.load() {
            // Recovery Test
            assert_eq!(verifier_result, 0);
        } else {
            // Discard Test
            assert_eq!(
                verifier_result,
                KernelError::NoSuchEntry.into_usize() as i32
            );
        }

        keos::debug!(
            "{} test pass for write count {}",
            if COMMITTED.load() {
                "recovery"
            } else {
                "discard"
            },
            wc
        );
    }

    assert!(IS_JOURNAL_SB_ALTERNATED.load());

    let final_verifier = ThreadBuilder::new("final_verifier").spawn(move || {
        let ffs = ffs::FastFileSystem::from_disk(Disk::new(2).ro(), true, true).unwrap();
        let root = ffs.root().unwrap();

        if let Err(e) = root.open("journal__test_file") {
            Current::exit(e.into_usize() as i32)
        }

        Current::exit(0)
    });
    assert_eq!(final_verifier.join(), 0);
}
