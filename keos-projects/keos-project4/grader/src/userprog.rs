use crate::Thread;
use alloc::boxed::Box;
use keos::thread::ThreadBuilder;
use keos_project2::{loader::LoadContext, mm_struct::MmStruct};

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
    b"argc: 4
argv[0] = /bin/ls (0x4747ffed)
argv[1] = -l (0x4747fff5)
argv[2] = foo (0x4747fff8)
argv[3] = bar (0x4747fffc)
"
)]
pub fn arg_parse() {
    run_elf_with_arg("arg_parse", &["/bin/ls", "-l", "foo", "bar"]);
}

#[stdin(b"")]
#[assert_output(b"success ")]
pub fn sys_open() {
    run_elf("sys_open");
}

#[stdin(b"")]
#[assert_output(b"success ")]
pub fn sys_read() {
    run_elf("sys_read");
}

#[stdin(b"")]
#[assert_output(b"success ")]
pub fn sys_read_error() {
    run_elf("sys_read_error");
}

#[stdin(b"")]
#[assert_output(b"success ")]
pub fn sys_write() {
    run_elf("sys_write");
}

#[stdin(b"")]
#[assert_output(b"success ")]
pub fn sys_write_error() {
    run_elf("sys_write_error");
}

#[stdin(b"")]
#[assert_output(b"success ")]
pub fn sys_seek() {
    run_elf("sys_seek");
}

#[stdin(b"")]
#[assert_output(b"success ")]
pub fn sys_seek_error() {
    run_elf("sys_seek_error");
}

#[stdin(b"")]
#[assert_output(b"success ")]
pub fn sys_tell() {
    run_elf("sys_tell");
}

#[stdin(b"")]
#[assert_output(b"success ")]
pub fn sys_tell_error() {
    run_elf("sys_tell_error");
}

#[stdin(b"KeOS is fun!")]
#[assert_output(b"success ")]
pub fn sys_stdio_1() {
    run_elf("sys_stdio_1");
}

#[stdin(b"Hello, World")]
#[assert_output(b"success ")]
pub fn sys_stdio_2() {
    run_elf("sys_stdio_2");
}

#[stdin(b"")]
#[assert_output(b"Hello, keos!success ")]
pub fn sys_stdout() {
    run_elf("sys_stdout");
}

#[stdin(b"")]
#[assert_output(b"Hello, keos!success ")]
pub fn sys_stderr() {
    run_elf("sys_stderr");
}

#[stdin(b"")]
#[assert_output(b"success ")]
pub fn sys_close() {
    run_elf("sys_close");
}

#[stdin(b"")]
#[assert_output(b"success ")]
pub fn sys_pipe() {
    run_elf("sys_pipe");
}

#[stdin(b"")]
#[assert_output(b"success ")]
pub fn mm_mmap() {
    run_elf("mm_mmap");
}

pub fn mm_mmap_error_protection() {
    assert_eq!(run_elf("mm_mmap_error_protection"), -1);
}

pub fn mm_mmap_error_protection_exec() {
    assert_eq!(run_elf("mm_mmap_error_protection_exec"), -1);
}

#[stdin(b"")]
#[assert_output(b"success ")]
pub fn mm_munmap() {
    run_elf("mm_munmap");
}

#[stdin(b"")]
#[assert_output(b"success ")]
pub fn mm_munmap_error() {
    run_elf("mm_munmap_error");
}

pub fn mm_exit_cleanup_stress() {
    for _ in 0..24 {
        assert_eq!(run_elf("mm_exit_cleanup"), 0);
    }
}

#[stdin(b"")]
#[assert_output(b"success ")]
pub fn bad_addr_1() {
    run_elf("bad_addr_1");
}

#[stdin(b"KeOS is fun!")]
pub fn bad_code_write() {
    assert_eq!(run_elf("bad_code_write"), -1);
}

#[stdin(b"")]
#[assert_output(b"Hello from thread!: deadbeef\nChild thread exited with code 0\n")]
pub fn thread_create() {
    run_elf("thread_create");
}

#[stdin(b"")]
#[assert_output(b"success ")]
pub fn thread_join_err() {
    run_elf("thread_join_err");
}

#[stdin(b"")]
#[assert_output(b"Found 664579 primes\nThread 2 exited with code 2\nThread 1 exited with code 1\n")]
pub fn thread_join_chain() {
    run_elf("thread_join_chain");
}

#[stdin(b"")]
#[assert_output(b"Found 664579 primes\nOnly 1 join succeeded\n")]
pub fn thread_join_complex() {
    run_elf("thread_join_complex");
}

#[stdin(b"")]
#[assert_output(b"Found 664579 primes\n")]
pub fn thread_mm_shared() {
    run_elf("thread_mm_shared");
}
