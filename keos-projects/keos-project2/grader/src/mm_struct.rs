use keos::{
    KernelError,
    addressing::Va,
    mm::page_table::{Permission, Pml4e},
};
use keos_project2::{eager_pager::EagerPager, mm_struct::MmStruct};

pub fn do_mmap() {
    let mut mm: MmStruct<EagerPager> = MmStruct::new();
    let pml4e_array = unsafe {
        (mm.page_table.pa().into_kva().into_usize() as *const [Pml4e; 512])
            .as_ref()
            .unwrap()
    };

    let small_va = Va::new(0x1000).unwrap();
    let big_va = Va::new(0x0000_7FFF_4746_0000).unwrap();

    assert_eq!(pml4e_array[0].0, 0);
    assert_eq!(pml4e_array[0xff].0, 0);

    assert_eq!(
        mm.do_mmap(small_va, 0x1000, Permission::READ, None, 0),
        Ok(0x1000),
        "mmap() to valid Virtual Address should succeed"
    );

    assert_ne!(
        pml4e_array[0].0, 0,
        "By the result of mmap(), PML4e entry should have been created"
    );

    assert_eq!(
        mm.do_mmap(big_va, 0x2000, Permission::READ, None, 0),
        Ok(0x0000_7FFF_4746_0000),
        "mmap() to valid Virtual Address should succeed"
    );

    assert_ne!(
        pml4e_array[0xff].0, 0,
        "By the result of mmap(), PML4e entry should have been created"
    );
}

pub fn bad_addr_0() {
    let mut mm: MmStruct<EagerPager> = MmStruct::new();
    let null_va = Va::new(0).unwrap();
    let small_va = Va::new(0x1000).unwrap();
    let misaligned = Va::new(0x1337).unwrap();
    let kern_percpu = Va::new(0xFFFF_FF00_0090_0000).unwrap();

    assert_eq!(
        mm.do_mmap(null_va, 0x1000, Permission::READ, None, 0),
        Err(KernelError::InvalidArgument),
        "mmap() to NULL should result in InvalidAccess"
    );

    assert_eq!(
        mm.do_mmap(kern_percpu, 0x1000, Permission::READ, None, 0),
        Err(KernelError::InvalidArgument),
        "mmap() to Kernel Virtual Address should result in InvalidAccess"
    );

    assert_eq!(
        mm.do_mmap(small_va, -0x2000isize as usize, Permission::READ, None, 0),
        Err(KernelError::InvalidArgument),
        "mmap() to Kernel Virtual Address should result in InvalidAccess"
    );

    assert_eq!(
        mm.do_mmap(misaligned, 0x1000, Permission::READ, None, 0),
        Err(KernelError::InvalidArgument),
        "Misaligned mmap() should result in InvalidArgument"
    );
}

pub fn access_ok_normal() {
    let mut mm: MmStruct<EagerPager> = MmStruct::new();
    let ro = Va::new(0x1000).unwrap();
    let rw = Va::new(0x2000).unwrap();

    assert_eq!(
        mm.do_mmap(ro, 0x1000, Permission::READ, None, 0),
        Ok(0x1000),
        "mmap() to valid Virtual Address should succeed"
    );

    assert!(
        mm.access_ok(ro..ro + 0xfff, false),
        "access_ok() with allocated memory area should return true"
    );

    assert_eq!(
        mm.do_mmap(rw, 0x1000, Permission::READ | Permission::WRITE, None, 0),
        Ok(0x2000),
        "mmap() to valid Virtual Address should succeed"
    );

    assert!(
        mm.access_ok(rw..rw + 0xfff, true),
        "access_ok() with write attempt to read-write memory area should return true"
    );
}

pub fn access_ok_invalid() {
    let mut mm: MmStruct<EagerPager> = MmStruct::new();
    let null_va = Va::new(0).unwrap();
    let misaligned = Va::new(0x42).unwrap();
    let kern_percpu = Va::new(0xFFFF_FF00_0090_0000).unwrap();

    assert!(
        !mm.access_ok(kern_percpu..kern_percpu + 0xfff, false),
        "access_ok() with Kernel Virtual Address should return false"
    );

    assert!(
        !mm.access_ok(null_va..null_va + 0xfff, false),
        "access_ok() with NULL pointer should return false"
    );

    assert!(
        !mm.access_ok(misaligned..misaligned + 1, false),
        "access_ok() with 0th page should return false"
    );

    let non_allocated = Va::new(0xDEADBEEF).unwrap();
    assert!(
        !mm.access_ok(non_allocated..non_allocated + 1, false),
        "access_ok() with unallocated memory area should return false"
    );

    let ro = Va::new(0x1000).unwrap();

    assert_eq!(
        mm.do_mmap(ro, 0x1000, Permission::READ, None, 0),
        Ok(0x1000),
        "mmap() to valid Virtual Address should succeed"
    );

    assert!(
        !mm.access_ok(ro..ro + 0xfff, true),
        "access_ok() with write attempt to read-only memory area should return false"
    );
}
