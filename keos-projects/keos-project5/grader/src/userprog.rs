use crate::Thread;
use alloc::boxed::Box;
use grading::*;
use keos::{fs::Disk, thread::ThreadBuilder};
use keos_project2::{loader::LoadContext, mm_struct::MmStruct};
use keos_project5::{ffs, page_cache::PageCache};

pub fn run_elf(name: &str) -> i32 {
    run_elf_with_arg(name, &[name])
}

pub fn run_elf_with_arg(name: &str, args: &[&str]) -> i32 {
    let LoadContext { mm_struct, regs } = LoadContext {
        mm_struct: MmStruct::new(),
        regs: keos::syscall::Registers::new(),
    }
    .load(
        &keos::fs::FileSystem::root()
            .open(name)
            .unwrap()
            .into_regular_file()
            .unwrap(),
        args,
    )
    .unwrap_or_else(|e| panic!("Failed to load elf: {}. reason: {:?}", name, e));

    let thread_build = ThreadBuilder::new(name);
    let tid = thread_build.get_tid();
    thread_build
        .attach_task(Box::new(Thread::from_mm_struct(mm_struct, tid)))
        .spawn(move || regs.launch())
        .join()
}

#[stdin(b"")]
#[assert_output(
    b"36d54439d71e9745235b21bca81f2e346aeecd6bd86b2b90e26524d8a115781d  os-release
"
)]
pub fn sha256sum() {
    let fs = ffs::FastFileSystem::from_disk(Disk::new(2), false, false).unwrap();
    keos::fs::FileSystem::register(PageCache::new(fs));
    let root = keos::fs::FileSystem::root();

    let simple_fs = simple_fs::FileSystem::load(1).unwrap();
    let simple_fs: &dyn keos::fs::traits::FileSystem = &simple_fs;
    let simple_fs_root = simple_fs.root().unwrap();

    let org_sha256sum = simple_fs_root
        .open("sha256sum")
        .unwrap()
        .into_regular_file()
        .unwrap();
    let new_sha256sum = root
        .create("sha256sum", false)
        .unwrap()
        .into_regular_file()
        .unwrap();
    keos::util::copy_file(&org_sha256sum, &new_sha256sum).unwrap();

    let org_os_release = simple_fs_root
        .open("os-release")
        .unwrap()
        .into_regular_file()
        .unwrap();
    let new_os_release = root
        .create("os-release", false)
        .unwrap()
        .into_regular_file()
        .unwrap();
    keos::util::copy_file(&org_os_release, &new_os_release).unwrap();

    assert_eq!(
        run_elf_with_arg("sha256sum", &["/bin/sha256sum", "os-release"]),
        0
    );
}

#[stdin(b"")]
#[assert_output(
    b"total 16
drwxrwxrwx    0    0        4096 Jan  1 00:00 .
drwxrwxrwx    0    0        4096 Jan  1 00:00 ..
-rwxrwxrwx    0    0         284 Jan  1 00:00 os-release
drwxrwxrwx    0    0        4096 Jan  1 00:00 skel
"
)]
pub fn ls() {
    let fs = ffs::FastFileSystem::from_disk(Disk::new(2), false, false).unwrap();
    keos::fs::FileSystem::register(PageCache::new(fs));
    let root = keos::fs::FileSystem::root();

    let simple_fs = simple_fs::FileSystem::load(1).unwrap();
    let simple_fs: &dyn keos::fs::traits::FileSystem = &simple_fs;
    let simple_fs_root = simple_fs.root().unwrap();

    let org_ls = simple_fs_root
        .open("ls")
        .unwrap()
        .into_regular_file()
        .unwrap();
    let new_ls = root
        .create("ls", false)
        .unwrap()
        .into_regular_file()
        .unwrap();
    keos::util::copy_file(&org_ls, &new_ls).unwrap();

    let dir = root
        .create("ls__dir", true)
        .unwrap()
        .into_directory()
        .unwrap()
        .create("etc", true)
        .unwrap()
        .into_directory()
        .unwrap();

    dir.create("skel", true).unwrap();
    let org_os_release = simple_fs_root
        .open("os-release")
        .unwrap()
        .into_regular_file()
        .unwrap();
    let new_os_release = dir
        .create("os-release", false)
        .unwrap()
        .into_regular_file()
        .unwrap();
    keos::util::copy_file(&org_os_release, &new_os_release).unwrap();

    assert_eq!(
        run_elf_with_arg("ls", &["/bin/ls", "-al", "/ls__dir/etc"]),
        0
    );
}

#[stdin(b"")]
#[assert_output(
    b"Extracting: simple_fs/ (size: 0)
Extracting: simple_fs/Cargo.lock (size: 1924)
Extracting: simple_fs/src/ (size: 0)
Extracting: simple_fs/src/lib.rs (size: 15334)
Extracting: simple_fs/src/keos_binder.rs (size: 4586)
Extracting: simple_fs/rust-toolchain (size: 126)
Extracting: simple_fs/Cargo.toml (size: 284)
Extraction complete.
793b473f24c2b110c39be7dfbd786a85600e738279dc25ce87aee2e4d49a6180  simple_fs/Cargo.lock
045759815f76da667489cff5d268fb5e788d5e631bafe02d3feac0f537eaf725  simple_fs/Cargo.toml
17254ea93d72683ba444ae3d75558ae289d55dc81e04234e63b9084019237218  simple_fs/rust-toolchain
754ae51ca63747c11101d796b7d0ec3f0ca564a0b93390ceaf63ad9e7c40b68f  simple_fs/src/keos_binder.rs
1a36d5613257a402ccb982f3a8826f8c8f447e6f4e8d6d22407be59f37d3eae7  simple_fs/src/lib.rs
"
)]
pub fn tar() {
    let fs = ffs::FastFileSystem::from_disk(Disk::new(2), false, false).unwrap();
    keos::fs::FileSystem::register(PageCache::new(fs));
    let root = keos::fs::FileSystem::root();

    let simple_fs = simple_fs::FileSystem::load(1).unwrap();
    let simple_fs: &dyn keos::fs::traits::FileSystem = &simple_fs;
    let simple_fs_root = simple_fs.root().unwrap();

    let org_archive = simple_fs_root
        .open("simple_fs.tar")
        .unwrap()
        .into_regular_file()
        .unwrap();
    let new_archive = root
        .create("simple_fs.tar", false)
        .unwrap()
        .into_regular_file()
        .unwrap();
    keos::util::copy_file(&org_archive, &new_archive).unwrap();

    let org_tar = simple_fs_root
        .open("tar")
        .unwrap()
        .into_regular_file()
        .unwrap();
    let new_tar = root
        .create("tar", false)
        .unwrap()
        .into_regular_file()
        .unwrap();
    keos::util::copy_file(&org_tar, &new_tar).unwrap();

    assert_eq!(
        run_elf_with_arg("tar", &["/bin/tar", "-x", "simple_fs.tar"]),
        0
    );

    let org_sha256sum: keos::fs::RegularFile = simple_fs_root
        .open("sha256sum")
        .unwrap()
        .into_regular_file()
        .unwrap();
    let new_sha256sum = root
        .create("tar__sha256sum", false)
        .unwrap()
        .into_regular_file()
        .unwrap();
    keos::util::copy_file(&org_sha256sum, &new_sha256sum).unwrap();

    let files = [
        "simple_fs/Cargo.lock",
        "simple_fs/Cargo.toml",
        "simple_fs/rust-toolchain",
        "simple_fs/src/keos_binder.rs",
        "simple_fs/src/lib.rs",
    ];

    for file in files {
        assert_eq!(
            run_elf_with_arg("tar__sha256sum", &["/bin/sha256sum", file]),
            0
        );
    }
}

#[stdin(b"")]
#[assert_output(
    b"Archiving: tar_gen__dir
Archiving: tar_gen__dir/etc
Archiving: tar_gen__dir/etc/skel
Archiving: tar_gen__dir/etc/os-release
Archiving: tar_gen__dir/bin
Archiving: tar_gen__dir/bin/ls
Archiving: tar_gen__dir/bin/sha256sum
Archiving: tar_gen__dir/bin/tar
Archiving complete.
"
)]
pub fn tar_gen() {
    let fs = ffs::FastFileSystem::from_disk(Disk::new(2), true, false).unwrap();
    keos::fs::FileSystem::register(PageCache::new(fs));
    let root = keos::fs::FileSystem::root();

    let simple_fs = simple_fs::FileSystem::load(1).unwrap();
    let simple_fs: &dyn keos::fs::traits::FileSystem = &simple_fs;
    let simple_fs_root = simple_fs.root().unwrap();

    let org_tar = simple_fs_root
        .open("tar")
        .unwrap()
        .into_regular_file()
        .unwrap();
    let new_tar = root
        .create("tar_gen__tar", false)
        .unwrap()
        .into_regular_file()
        .unwrap();
    keos::util::copy_file(&org_tar, &new_tar).unwrap();

    let dir = root
        .create("tar_gen__dir", true)
        .unwrap()
        .into_directory()
        .unwrap();

    let etc = dir.create("etc", true).unwrap().into_directory().unwrap();

    let bin = dir.create("bin", true).unwrap().into_directory().unwrap();

    etc.create("skel", true).unwrap();
    let org_os_release = simple_fs_root
        .open("os-release")
        .unwrap()
        .into_regular_file()
        .unwrap();
    let new_os_release = etc
        .create("os-release", false)
        .unwrap()
        .into_regular_file()
        .unwrap();
    keos::util::copy_file(&org_os_release, &new_os_release).unwrap();

    let org_ls = simple_fs_root
        .open("ls")
        .unwrap()
        .into_regular_file()
        .unwrap();
    let bin_ls = bin
        .create("ls", false)
        .unwrap()
        .into_regular_file()
        .unwrap();
    keos::util::copy_file(&org_ls, &bin_ls).unwrap();

    let org_sha256sum = simple_fs_root
        .open("sha256sum")
        .unwrap()
        .into_regular_file()
        .unwrap();
    let bin_sha256sum = bin
        .create("sha256sum", false)
        .unwrap()
        .into_regular_file()
        .unwrap();
    keos::util::copy_file(&org_sha256sum, &bin_sha256sum).unwrap();

    let bin_tar = bin
        .create("tar", false)
        .unwrap()
        .into_regular_file()
        .unwrap();
    keos::util::copy_file(&org_tar, &bin_tar).unwrap();

    assert_eq!(
        run_elf_with_arg(
            "tar_gen__tar",
            &["/bin/tar", "-c", "generated.tar", "tar_gen__dir"]
        ),
        0
    );
}
