use crate::userprog::run_elf;

#[stdin(b"")]
#[assert_output(
    b"Hello, parent!
Hello, child!
"
)]
pub fn fork() {
    assert_eq!(run_elf("sys_fork"), 0);
}

#[stdin(b"")]
#[assert_output(
    b"Hello, parent!
Child edited successfully!
Hello, child!
"
)]
pub fn fork2() {
    assert_eq!(run_elf("sys_fork2"), -1);
}

pub fn cow() {
    assert_eq!(run_elf("mm_cow"), 0);
}

pub fn cow_perm() {
    assert_eq!(run_elf("mm_cow_perm"), 0);
}

pub fn cow_sys() {
    assert_eq!(run_elf("mm_cow_sys"), 0);
}

pub fn cow_cleanup_stress() {
    for _ in 0..12 {
        assert_eq!(run_elf("fork_cow_cleanup"), 0);
    }
}
