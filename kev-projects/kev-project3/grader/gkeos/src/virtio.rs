use crate::simple_virtio::VirtIoDisk;
use alloc::vec;
use core::str::from_utf8;
use keos::fs::{BlockOps, Sector};

const DISK_CONTENT: &str = "Welcome to the KeV project.\n\n\
            Virtualization is an increasingly ubiquitous feature of modern computer systems, and a rapidly evolving part of the system stack. Hardware vendors are adding new features to support more efficient virtualization, OS designs are adapting to perform better in VMs, and VMs are an essential component in cloud computing. Thus, understanding how VMs work is essential to a complete education in computer systems.\n\n\
            In this project, you will skim through the basic components that runs on real virtual machine monitor like KVM. From what you learn, you will build your own type 2 hypervisor and finally extend the hypervisor as an open-ended course project.\n\n\
            In KeV project, we will not bother you from the time-consuming edge case handling and the hidden test cases. The score that you see when run the grading scripts is your final score. We want to keep this project as easy as possible. If you have suggestions on how we can reduce the unnecessary overhead of assignments, cutting them down to the important underlying issues, please let us know.\n";

// Get virtual disk size
fn get_disk_size(disk: &VirtIoDisk) -> usize {
    let mut read_buf = [0; 512];
    let mut check1 = [0xff; 512];
    let mut check2 = [0xee; 512];

    let mut idx = 0;
    loop {
        read_buf.fill(0xff);
        assert!(disk.read(Sector(idx), &mut read_buf));
        if read_buf.eq(&check1) {
            read_buf.fill(0xee);
            assert!(disk.read(Sector(idx), &mut read_buf));
            if read_buf.eq(&check2) {
                break;
            }
        }
        idx += 1;
    }
    idx -= 1;
    assert!(idx >= 0);
    assert!(disk.read(Sector(idx), &mut check1));
    assert!(disk.read(Sector(idx), &mut check2));
    let mut off = 0;
    for i in 0..512 {
        if check1[i] != check2[i] {
            off = i;
            break;
        }
    }
    idx * 512 + off
}

pub fn check_blockio() {
    let mut read_buf = [0 as u8; 512];
    let mut write_buf = [0 as u8; 512];
    let mut disk = VirtIoDisk::new().unwrap();

    // Test virtio read operation.
    for (idx, off) in (0..DISK_CONTENT.len()).step_by(512).enumerate() {
        let start = off;
        let end = (off + 512).min(DISK_CONTENT.len());
        read_buf.fill(0);

        assert!(disk.read(Sector(idx), &mut read_buf));
        assert_eq!(
            &from_utf8(&read_buf).unwrap()[..(end - start)],
            &DISK_CONTENT[start..end]
        );
    }

    // Test virtio write operation.
    write_buf.fill(77);
    read_buf.fill(0);

    assert!(disk.write(Sector(1), &mut write_buf));
    assert!(disk.read(Sector(1), &mut read_buf));
    assert_eq!(
        from_utf8(&read_buf).unwrap(),
        from_utf8(&write_buf).unwrap()
    );

    // Check that other sectors are not corrupted
    for (idx, off) in (0..DISK_CONTENT.len()).step_by(512).enumerate() {
        if idx == 1 {
            continue;
        }
        let start = off;
        let end = (off + 512).min(DISK_CONTENT.len());
        read_buf.fill(0);

        assert!(disk.read(Sector(idx), &mut read_buf));
        assert_eq!(
            &from_utf8(&read_buf).unwrap()[..(end - start)],
            &DISK_CONTENT[start..end]
        );
    }

    // Restore disk contents
    assert!(disk.write(
        Sector(1),
        &DISK_CONTENT[512..1024].as_bytes().try_into().unwrap()
    ));

    disk.finish();
}

pub fn check_blockio_batching() {
    let mut disk = VirtIoDisk::new().unwrap();
    let disk_len = get_disk_size(&disk);
    let mut read_buf = vec![0; (disk_len + 511) / 512 * 512];
    let mut read_buf1 = vec![0; (disk_len + 511) / 512 * 512];
    let write_buf = vec![77; (disk_len + 511) / 512 * 512];

    // Test virtio read batch.
    assert!(disk.read_many(Sector(0), &mut read_buf).is_ok());
    assert_eq!(
        &from_utf8(&read_buf).unwrap()[..DISK_CONTENT.len()],
        DISK_CONTENT
    );

    // Test virtio write batch
    assert!(disk.write_many(Sector(0), &write_buf).is_ok());
    assert!(disk.read_many(Sector(0), &mut read_buf1).is_ok());
    assert_eq!(
        from_utf8(&read_buf1[..disk_len]).unwrap(),
        from_utf8(&write_buf[..disk_len]).unwrap()
    );

    // Restore contents
    assert!(disk.write_many(Sector(0), &mut read_buf).is_ok());
    disk.finish();
}
