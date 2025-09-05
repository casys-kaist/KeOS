use alloc::boxed::Box;
use grading::syscall;
use keos::{KernelError, addressing::Va, fs::FileSystem};
use keos_project1::file_struct::FileStruct;
use keos_project5::{ACCESS_CHECK_BYPASS_LIST, SyscallNumber};

struct AccessCheckBypasser<T> {
    inner: *const T,
    count: usize,
}

impl<T> AccessCheckBypasser<T> {
    fn new(inner: *const T, count: usize) -> Option<Self> {
        for offset in 0..count {
            let mut guard = ACCESS_CHECK_BYPASS_LIST.lock();
            let va = match Va::new(inner as *const () as usize + offset * core::mem::size_of::<T>())
            {
                Some(va) => va,
                None => {
                    guard.unlock();
                    return None;
                }
            };

            guard.insert(va);
            guard.unlock();
        }
        Some(Self { inner, count })
    }

    fn as_ptr(&self) -> *const T {
        self.inner
    }
}

impl<T> Drop for AccessCheckBypasser<T> {
    fn drop(&mut self) {
        for offset in 0..self.count {
            let mut guard = ACCESS_CHECK_BYPASS_LIST.lock();
            guard.remove(
                &Va::new(self.inner as *const () as usize + offset * core::mem::size_of::<T>())
                    .unwrap(),
            );
            guard.unlock();
        }
    }
}

pub fn open_dir() {
    let root = FileSystem::root();

    root.create("open_dir", true).unwrap();

    let fd = syscall!(
        SyscallNumber::Open as usize,
        AccessCheckBypasser::new(c"open_dir".as_ptr(), 9)
            .unwrap()
            .as_ptr(),
        0
    );

    assert!(fd >= 3, "Opening the directory must succeed.");
}

pub fn dir_rw() {
    let root = FileSystem::root();

    root.create("dir_rw", true).unwrap();

    let fd = syscall!(
        SyscallNumber::Open as usize,
        AccessCheckBypasser::new(c"dir_rw".as_ptr(), 7)
            .unwrap()
            .as_ptr(),
        2
    );

    assert!(fd >= 3, "Opening the directory must succeed.");

    let buf = Box::new([0u8; 4096]);

    assert_eq!(
        syscall!(
            SyscallNumber::Read as usize,
            fd,
            AccessCheckBypasser::new(&*buf, 1).unwrap().as_ptr(),
            0x1000
        )
        .try_into(),
        Ok(KernelError::IsDirectory),
    );

    assert_eq!(
        syscall!(
            SyscallNumber::Write as usize,
            fd,
            AccessCheckBypasser::new(&*buf, 1).unwrap().as_ptr(),
            0x1000
        )
        .try_into(),
        Ok(KernelError::IsDirectory),
    );
}

pub fn dir_seek() {
    let root = FileSystem::root();

    root.create("dir_seek", true).unwrap();

    let fd = syscall!(
        SyscallNumber::Open as usize,
        AccessCheckBypasser::new(c"dir_seek".as_ptr(), 9)
            .unwrap()
            .as_ptr(),
        0
    );

    assert!(fd >= 3, "Opening the directory must succeed.");

    for i in 1..=6 {
        assert_eq!(
            syscall!(SyscallNumber::Seek as usize, fd, 1234 * (i % 2), i / 2).try_into(),
            Ok(KernelError::InvalidArgument),
        );
    }

    assert_eq!(
        syscall!(SyscallNumber::Seek as usize, fd, 0, 0).try_into(),
        Ok(KernelError::InvalidArgument),
    );
}

pub fn create() {
    let root = FileSystem::root();

    root.create("create__exists", false).unwrap();

    assert_eq!(
        syscall!(
            SyscallNumber::Create as usize,
            AccessCheckBypasser::new(c"create__exists".as_ptr(), 15)
                .unwrap()
                .as_ptr()
        )
        .try_into(),
        Ok(KernelError::FileExist),
    );

    assert_eq!(
        syscall!(
            SyscallNumber::Create as usize,
            AccessCheckBypasser::new(c"create__syscall".as_ptr(), 17)
                .unwrap()
                .as_ptr()
        ),
        0
    );

    root.open("create__syscall")
        .expect("File created by create() syscall must present.")
        .into_regular_file()
        .expect("File created by create() syscall must be a RegularFile");
}

pub fn mkdir() {
    let root = FileSystem::root();

    root.create("mkdir__exists", true).unwrap();

    assert_eq!(
        syscall!(
            SyscallNumber::Mkdir as usize,
            AccessCheckBypasser::new(c"mkdir__exists".as_ptr(), 14)
                .unwrap()
                .as_ptr()
        )
        .try_into(),
        Ok(KernelError::FileExist),
    );

    assert_eq!(
        syscall!(
            SyscallNumber::Mkdir as usize,
            AccessCheckBypasser::new(c"mkdir__syscall".as_ptr(), 15)
                .unwrap()
                .as_ptr()
        ),
        0
    );

    root.open("mkdir__syscall")
        .expect("Directory created by mkdir() syscall must present.")
        .into_directory()
        .expect("Directory created by mkdir() syscall must be a Directory");
}

pub fn unlink() {
    let root = FileSystem::root();

    for this in [c".", c".."] {
        assert_eq!(
            syscall!(
                SyscallNumber::Unlink as usize,
                AccessCheckBypasser::new(this.as_ptr(), this.count_bytes() + 1)
                    .unwrap()
                    .as_ptr()
            )
            .try_into(),
            Ok(KernelError::Busy)
        );
    }

    assert_eq!(
        syscall!(
            SyscallNumber::Unlink as usize,
            AccessCheckBypasser::new(c"unlink__absent".as_ptr(), 15)
                .unwrap()
                .as_ptr()
        )
        .try_into(),
        Ok(KernelError::NoSuchEntry)
    );

    root.create("unlink__exists", true).unwrap();

    assert_eq!(
        syscall!(
            SyscallNumber::Unlink as usize,
            AccessCheckBypasser::new(c"unlink__exists".as_ptr(), 15)
                .unwrap()
                .as_ptr()
        ),
        0
    );

    assert!(
        root.open("unlink__exists").is_err(),
        "Directory `unlink__exists` must be removed by unlink() system call."
    );
}

pub fn chdir() {
    let root = FileSystem::root();

    let file_struct = unsafe {
        &*(syscall!(SyscallNumber::GetPhys as usize, 0, 0x80041337u32 as i32) as usize
            as *const FileStruct)
    };

    assert_eq!(
        root.ino(),
        file_struct.cwd.ino(),
        "Initially, Current Working Directory must be the root."
    );

    let dir = root
        .create("chdir__dir", true)
        .unwrap()
        .into_directory()
        .unwrap();

    assert_eq!(
        syscall!(
            SyscallNumber::Chdir as usize,
            AccessCheckBypasser::new(c"chdir__dir".as_ptr(), 11)
                .unwrap()
                .as_ptr()
        ),
        0
    );

    assert_eq!(
        dir.ino(),
        file_struct.cwd.ino(),
        "After chdir() to the directory `chdir__dir', cwd must be `chdir__dir'."
    );
}
