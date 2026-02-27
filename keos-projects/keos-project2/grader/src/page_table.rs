use keos::{
    addressing::Va,
    mm::page_table::{get_current_pt_pa, load_pt},
    mm::{
        Page,
        page_table::{PageTableMappingError, PageTableRoot, Permission, PteFlags},
    },
};
use keos_project2::page_table::PageTable;

/// Insert an entry with `va` and `permission` into the page table, and verifies
/// the operation.
///
/// This function ensures that a virtual address (`va`) is successfully mapped
/// to a newly allocated physical page with the expected permissions. It
/// validates that the mapping operation succeeds and that the resulting page
/// table entry (PTE) matches the expected physical address and permission
/// flags.
///
/// It ensures that:
/// - The mapping success.
/// - `PageTable::walk()` finds a valid a page table entry.
/// - Physical address in the entry matches the allocated page.
/// - Permission flags matches the given permission.
fn check_insert_one(pgtbl: &mut PageTable, va: usize, permission: Permission) {
    let va = Va::new(va).unwrap();
    let pg = Page::new();
    let pa = pg.pa();

    // Attempt to map the virtual address (`va`) to the allocated physical page with
    // the given permissions.
    assert!(pgtbl.map(va, pg, permission).is_ok());

    // Retrieve the page table entry (PTE) for the given virtual address.
    let pte = pgtbl.walk(va).expect("PageTable::walk() failed.");

    // Ensure that the physical address stored in the PTE matches the allocated
    // physical page.
    assert_eq!(pte.pa().unwrap(), pa);

    let mut expected = PteFlags::empty();
    // If any permission is provided, the `Present (P)` flag should be set.
    if !(permission & !Permission::USER).is_empty() {
        // In amd64, if page presents then page is readble.
        expected |= PteFlags::P;
        // If the page is writable, set the `Read/Write (RW)` flag.
        if permission.contains(Permission::WRITE) {
            expected |= PteFlags::RW;
        }
        // If the page is accessible by user mode, set the `User (US)` flag.
        if permission.contains(Permission::USER) {
            expected |= PteFlags::US;
        }
        // If the page is **not** executable, set the `Executable Disable (XD)` flag.
        if !permission.contains(Permission::EXECUTABLE) {
            expected |= PteFlags::XD;
        }
    }

    // Ensure that the permission flags in the PTE match the expected configuration.
    assert_eq!(pte.flags(), expected);
}

/// Remove an entry from page table and verify.
///
/// This function ensures that a previously mapped virtual address (`va`) is
/// successfully unmapped. After the removal, it checks that the page table no
/// longer contains an entry for that address.
fn check_remove_one(pgtbl: &mut PageTable, va: usize) {
    let va = Va::new(va).unwrap();

    // Attempt to unmap the virtual address from the page table.
    assert!(pgtbl.unmap(va).is_ok());

    // Ensure that the page table no longer contains an entry for `va`.
    // `walk()` should return `Err(PageTableMappingError::NotExist)`, confirming
    // removal.
    assert!(matches!(
        pgtbl.walk(va),
        Err(PageTableMappingError::NotExist)
    ));
}

/// A simple test to verify basic mapping and unmapping operations.
///
/// This function tests a single virtual address mapping and unmapping
/// with read-write permissions to ensure basic functionality works correctly.
pub fn simple() {
    // Create a new page table with a boxed root.
    let mut pgtbl = PageTable(PageTableRoot::new_boxed());

    // Test with read-write permission
    let perm = Permission::READ | Permission::WRITE;
    let va = Va::new(0x1000).unwrap();

    keos::println!("Testing single page mapping with READ|WRITE permission");

    // Map the virtual address to a newly allocated page
    assert!(pgtbl.map(va, Page::new(), perm).is_ok());

    // Verify the mapping exists
    assert!(pgtbl.walk(va).is_ok());

    // Unmap the virtual address
    assert!(pgtbl.unmap(va).is_ok());

    // Verify the mapping no longer exists
    assert!(matches!(
        pgtbl.walk(va),
        Err(PageTableMappingError::NotExist)
    ));
}

/// A comprehensive test to verify that mapping and unmapping operations work
/// correctly across all permission combinations.
///
/// This function tests whether virtual addresses can be mapped to physical
/// pages and later successfully unmapped from the page table.
pub fn simple2() {
    // Create a new page table with a boxed root.
    let mut pgtbl = PageTable(PageTableRoot::new_boxed());

    // Iterate over all possible permissions.
    for perm in Permission::ALL_CASES {
        keos::println!("Testing Permission: {perm:?}");
        let map_expect = if perm.is_empty() || !perm.contains(Permission::READ) {
            Err(PageTableMappingError::InvalidPermission)
        } else {
            Ok(())
        };
        // Map a range of virtual addresses to newly allocated pages.
        // Each virtual address is converted from an index multiplied by the page size
        // (0x1000).
        for va in (0x1234..0x4567).map(|i| Va::new(i * 0x1000)) {
            assert_eq!(pgtbl.map(va.unwrap(), Page::new(), perm), map_expect)
        }

        // Unmap the virtual addresses in reverse order to ensure correctness.
        for va in (0x1234..0x4567).rev().map(|i| Va::new(i * 0x1000)) {
            assert_eq!(
                pgtbl.unmap(va.unwrap()).map(|_| ()),
                if map_expect.is_err() {
                    Err(PageTableMappingError::NotExist)
                } else {
                    Ok(())
                }
            );
        }
    }
}

/// A test to verify whether the page table permission bits are correctly set
/// for x86 architecture.
///
/// This function tests a single virtual address mapping with a specific
/// permission has proper permission bits on page table.
pub fn x86_permission() {
    // Take a current kernel page table.
    let prev_cr3 = get_current_pt_pa();
    let mut pgtbl = PageTable::new();

    let perm = Permission::READ | Permission::WRITE | Permission::EXECUTABLE;
    let va = Va::new(0x1000).unwrap();

    let mut page = Page::new();
    page.inner_mut()[0] = 0x7;
    assert!(pgtbl.map(va, page, perm).is_ok());

    // Verify the mapping exists
    assert!(pgtbl.walk(va).is_ok());

    load_pt(pgtbl.pa());

    // Test whether the read permission is actually set
    keos::println!("Testing read...");
    unsafe {
        let val = core::ptr::read(0x1000 as *mut u64);
        assert_eq!(val, 0x7);
    }

    // Test whether the write permission is actually set
    keos::println!("Testing write...");
    unsafe {
        core::ptr::write(0x1000 as *mut u8, 0x89); // mov eax, edi
        core::ptr::write(0x1001 as *mut u8, 0xf8);
        core::ptr::write(0x1002 as *mut u8, 0x01); // add eax, esi
        core::ptr::write(0x1003 as *mut u8, 0xf0);
        core::ptr::write(0x1004 as *mut u8, 0xc3); // ret
    }

    // Test whether the executable permission is actually set
    keos::println!("Testing execute...");
    let func = unsafe { core::mem::transmute::<usize, extern "C" fn(u32, u32) -> u32>(0x1000) };
    let res = func(10, 32);
    assert_eq!(res, 42);

    // Unmap the virtual address
    assert!(pgtbl.unmap(va).is_ok());

    load_pt(prev_cr3);
}

/// A more advanced test to verify whether the page table permission bits are
/// correctly set for x86 architecture.
///
/// This function tests two virtual address mappings with a specific
/// permission have proper permission bits on page table.
pub fn x86_permission_advanced() {
    // Take a current kernel page table.
    let prev_cr3 = get_current_pt_pa();
    let mut pgtbl = PageTable::new();

    let perm = Permission::READ | Permission::WRITE | Permission::EXECUTABLE;
    let va = Va::new(0x1000).unwrap();

    let perm_2 = Permission::READ | Permission::EXECUTABLE;
    let va_2 = Va::new(0x2000).unwrap();

    let perm_3 = Permission::READ | Permission::WRITE;
    let va_3 = Va::new(0x3000).unwrap();

    let mut page = Page::new();
    page.inner_mut()[0] = 0x7;

    assert!(pgtbl.map(va, page, perm).is_ok());
    assert!(pgtbl.map(va_2, Page::new(), perm_2).is_ok());
    assert!(pgtbl.map(va_3, Page::new(), perm_3).is_ok());

    load_pt(pgtbl.pa());

    // Test whether the read permission is actually set
    keos::println!("Testing read...");
    unsafe {
        let val = core::ptr::read(0x1000 as *mut u64);
        assert_eq!(val, 0x7);
    }

    // Test whether the write permission is actually set
    keos::println!("Testing write...");
    unsafe {
        core::ptr::write(0x1000 as *mut u8, 0x89); // mov eax, edi
        core::ptr::write(0x1001 as *mut u8, 0xf8);
        core::ptr::write(0x1002 as *mut u8, 0x01); // add eax, esi
        core::ptr::write(0x1003 as *mut u8, 0xf0);
        core::ptr::write(0x1004 as *mut u8, 0xc3); // ret
    }

    // Test whether the executable permission is actually set
    keos::println!("Testing execute...");
    let func = unsafe { core::mem::transmute::<usize, extern "C" fn(u32, u32) -> u32>(0x1000) };
    let res = func(10, 32);
    assert_eq!(res, 42);

    // Unmap the virtual address
    assert!(pgtbl.unmap(va).is_ok());
    assert!(pgtbl.unmap(va_2).is_ok());
    assert!(pgtbl.unmap(va_3).is_ok());

    load_pt(prev_cr3);
}

/// Tests that all allocated pages are freed when the page table is dropped.
///
/// This function creates a new page table, maps a range of virtual addresses to
/// physical pages, and relies on `#[validate_alloc]` to ensure that all
/// allocations are properly freed when `PageTable` is dropped. If any
/// allocation is not freed, `#[validate_alloc]` will trigger a failure.
#[validate_alloc]
pub fn free() {
    // Create a new page table with a dynamically allocated root.
    let mut pgtbl = PageTable(PageTableRoot::new_boxed());

    // Map a range of virtual addresses to newly allocated physical pages with read
    // permission.
    for va in (0x1234..0x4567).map(|i| Va::new(i * 0x1000)) {
        assert!(
            pgtbl
                .map(va.unwrap(), Page::new(), Permission::READ)
                .is_ok()
        );
    }
    // No explicit unmap is performed here—`#[validate_alloc]` ensures all pages
    // are freed at drop.
}

/// Tests error handling for invalid page table operations.
///
/// This function verifies that the page table correctly rejects duplicate
/// mappings, unaligned virtual addresses, and attempts to unmap nonexistent
/// entries. The `#[validate_alloc]` attribute ensures that all allocated pages
/// are freed when `PageTable` is dropped.
#[validate_alloc]
pub fn error() {
    let mut pgtbl = PageTable(PageTableRoot::new_boxed());

    // Map a valid virtual address to a newly allocated physical page with read
    // permission.
    assert!(
        pgtbl
            .map(Va::new(0x1234000).unwrap(), Page::new(), Permission::READ)
            .is_ok()
    );

    // Attempt to map the same virtual address again; should fail with `Duplicated`
    // error.
    assert_eq!(
        pgtbl.map(Va::new(0x1234000).unwrap(), Page::new(), Permission::READ),
        Err(PageTableMappingError::Duplicated)
    );

    // Attempt to map an unaligned virtual address; should fail with `Unaligned`
    // error.
    assert_eq!(
        pgtbl.map(Va::new(0x1234123).unwrap(), Page::new(), Permission::READ),
        Err(PageTableMappingError::Unaligned)
    );

    // Attempt to unmap a virtual address that has not been mapped; should fail with
    // `NotExist` error.
    assert_eq!(
        pgtbl.unmap(Va::new(0x1235000).unwrap()).map(|_| ()),
        Err(PageTableMappingError::NotExist)
    );

    // No explicit unmap is performed—`#[validate_alloc]` ensures all pages are
    // freed at drop.
}

/// Tests various complex scenarios for page table mappings.
///
/// This function verifies that mappings persist correctly when new entries are
/// added or removed. The `#[validate_alloc]` attribute ensures that all
/// allocated pages are freed when `PageTable` is dropped.
#[validate_alloc]
pub fn complicate() {
    let mut pgtbl = PageTable(PageTableRoot::new_boxed());

    // Define a set of high-memory addresses to test mapping persistence.
    let permission = Permission::READ | Permission::EXECUTABLE;
    let addrs = [
        0xfffffff000,
        0xffffffe000,
        0xffffdff000,
        0xffbffff000,
        0x7ffffff000,
    ];

    // Insert multiple pages and verify that earlier mappings remain intact.
    for (i, &addr) in addrs.iter().enumerate() {
        check_insert_one(&mut pgtbl, addr, permission);
        if i != 0 {
            // Ensure previous mappings are still valid after adding a new one.
            assert!(pgtbl.walk(Va::new(addrs[i - 1]).unwrap()).is_ok());
        }
    }

    // Remove pages one by one and verify that earlier mappings are unaffected.
    for (i, &addr) in addrs.iter().enumerate() {
        if i == 0 {
            continue;
        }
        check_remove_one(&mut pgtbl, addr);
        // Ensure the first mapped page still exists after removing others.
        assert!(pgtbl.walk(Va::new(addrs[0]).unwrap()).is_ok());
    }

    // Remove the first mapped page.
    check_remove_one(&mut pgtbl, addrs[0]);

    // No explicit unmap is performed here—`#[validate_alloc]` ensures all pages
    // are freed at drop.
}
