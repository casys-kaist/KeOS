use alloc::vec::Vec;
use keos::{
    addressing::{Pa, PAGE_SHIFT},
    mm::{
        page_table::{Pde, PdeFlags, Pdpe, PdpeFlags, Pml4e, Pml4eFlags},
        Page,
    },
    thread::Thread,
};
use kev::{
    vm::Gva,
    vmcs::{Field, Vmcs},
    {vm::Gpa, Probe},
};
use kev_project2::ept::{EptMappingError, EptPteFlags, ExtendedPageTable, Permission};

fn check_insert_one(pgtbl: &mut ExtendedPageTable, gpa: usize, permission: Permission) {
    let gpa = Gpa::new(gpa).unwrap();
    let pg = Page::new();
    let pa = pg.pa();
    assert!(pgtbl.map(gpa, pg, permission).is_ok());
    let pte = pgtbl.walk(gpa);
    assert!(pte.is_ok());
    let pte = pte.unwrap();
    assert_eq!(pte.pa().unwrap(), pa);
    assert_eq!(
        pte.flags().intersection(EptPteFlags::FULL),
        EptPteFlags::from_bits_truncate(permission.bits())
    );
}

fn check_remove_one(pgtbl: &mut ExtendedPageTable, gpa: usize) {
    let gpa = Gpa::new(gpa).unwrap();
    assert!(pgtbl.unmap(gpa).is_ok());
    assert!(matches!(pgtbl.walk(gpa), Err(EptMappingError::NotExist)));
}

pub fn simple() {
    let mut pgtbl = ExtendedPageTable::new();
    assert!(pgtbl
        .map(
            Gpa::new(0x1234000).unwrap(),
            Page::new(),
            Permission::READ,
        )
        .is_ok());
    assert_eq!(
        pgtbl.map(
            Gpa::new(0x1234000).unwrap(),
            Page::new(),
            Permission::READ,
        ),
        Err(EptMappingError::Duplicated)
    );
    assert_eq!(
        pgtbl.map(
            Gpa::new(0x1234123).unwrap(),
            Page::new(),
            Permission::READ,
        ),
        Err(EptMappingError::Unaligned)
    );
    assert_eq!(
        pgtbl.unmap(Gpa::new(0x1235000).unwrap()).map(|_| ()),
        Err(EptMappingError::NotExist)
    );
    assert!(pgtbl.unmap(Gpa::new(0x1234000).unwrap()).is_ok());
}

pub fn complicate() {
    let mut pgtbl = ExtendedPageTable::new();

    let addr = 0x1234000;
    // Check combination of permissions
    for i in 1..8 {
        check_insert_one(&mut pgtbl, addr, Permission::from_bits_truncate(i));
        check_remove_one(&mut pgtbl, addr);
    }

    let permission = Permission::READ | Permission::EXECUTABLE;
    let mut addrs: [usize; 5] = [0xeeee_ffff_ffff_f000; 5];
    for (i, p) in addrs.iter_mut().enumerate() {
        if i == 0 {
            continue;
        }
        *p ^= 1 << (PAGE_SHIFT + 9 * (i - 1));
        // 0xeeee_ffff_ffff_f000
        // 0xeeee_ffff_ffff_e000
        // 0xeeee_ffff_ffdf_f000
        // 0xeeee_ffff_bfff_f000
        // 0xeeee_ff7f_ffff_f000
    }

    for (i, addr) in addrs.iter().enumerate() {
        check_insert_one(&mut pgtbl, *addr, permission);
        if i != 0 {
            // Check the previous map not to be forgotten if additional mapping created
            assert!(pgtbl.walk(Gpa::new(addrs[i - 1]).unwrap()).is_ok());
        }
    }
    for (i, addr) in addrs.iter().enumerate() {
        if i == 0 {
            continue;
        };
        check_remove_one(&mut pgtbl, *addr);
        // Check the first map not to be forgotten if other mapping removed
        assert!(pgtbl.walk(Gpa::new(addrs[0]).unwrap()).is_ok());
    }
    check_remove_one(&mut pgtbl, addrs[0]);
}

pub fn check_huge_translation() {
    let _p = Thread::pin();
    let mut ept = ExtendedPageTable::new();
    let vmcs = Vmcs::activate(&mut Vmcs::new()).unwrap();

    vmcs.write(Field::GuestCr3, 0x1000).unwrap();
    let pml4_page = Page::new();
    let pml4 = unsafe {
        (pml4_page.kva().into_usize() as *mut [Pml4e; 512])
            .as_mut()
            .unwrap()
    };
    pml4[0]
        .set_pa(Pa::new(0x2000).unwrap())
        .unwrap()
        .set_flags(Pml4eFlags::P | Pml4eFlags::RW);
    assert!(ept
        .map(Gpa::new(0x1000).unwrap(), pml4_page, Permission::all())
        .is_ok());

    let pdp_page = Page::new();
    let pdp = unsafe {
        (pdp_page.kva().into_usize() as *mut [Pdpe; 512])
            .as_mut()
            .unwrap()
    };
    pdp[0]
        .set_pa(Pa::new(0x3000).unwrap())
        .unwrap()
        .set_flags(PdpeFlags::P | PdpeFlags::RW);
    assert!(ept
        .map(Gpa::new(0x2000).unwrap(), pdp_page, Permission::all())
        .is_ok());

    let pd_page = Page::new();
    let pd = unsafe {
        (pd_page.kva().into_usize() as *mut [Pde; 512])
            .as_mut()
            .unwrap()
    };
    pd[1]
        .set_pa(Pa::new(0x200000).unwrap())
        .unwrap()
        .set_flags(PdeFlags::P | PdeFlags::RW | PdeFlags::PS);
    assert!(ept
        .map(Gpa::new(0x3000).unwrap(), pd_page, Permission::all())
        .is_ok());

    let mut pgs = (0..512)
        .map(|_| Page::new())
        .collect::<Vec<Page>>();
    let mut pas = pgs.iter().map(|pg| pg.pa()).collect::<Vec<Pa>>();
    for i in (0x200_000..0x400_000).step_by(0x1000) {
        assert!(ept
            .map(Gpa::new(i).unwrap(), pgs.pop().unwrap(), Permission::all())
            .is_ok());
    }

    for i in (0x200_000..0x400_000).step_by(0x1000) {
        let o = ept.gva2hpa(&vmcs, Gva::new(i).unwrap());
        assert!(o.is_some());
        assert_eq!(o.unwrap(), pas.pop().unwrap());
    }
}
