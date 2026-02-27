//! Project 5: Final project
//!
//! You have learned the basic components of the virtulaization.
//! In the final project, you will do an open-ended final project based on what
//! you have learned. For the final project, you should add a significant piece
//! of functionality to the kev, as described below. The project must (1) have a
//! significant implementation component, specified by deliverables (2) concern
//! operating systems and (3) be challenging and potentially research-worthy.
//! You can modify any source code of the keos and kev infrastructure.
//! For all projects, you must write a proposal that is accepted by the course
//! staff.
//!
//! ## Candidates for final project
//! Here is a list of possible tasks for a final project. Some are small enough
//! that multiple options should be combined, especially if you have a larger
//! group. You are most welcome to come up with other ideas. The actual project
//! is up to you. Please pick something manageable. It is far better to complete
//! your project to spec than it is to take on something too big and not have
//! anything to show for it except excellent intentions (also true in the real
//! world).
//!
//! - Port the kev to work on AMD SVM (the rough equivalent of Intel's VMX).
//! - Implement support for an IOMMU to allow the guest to directly access
//!   hardware.
//! - Incorporate APICv support into kev.
//! - Use a binary translator to support trap-and-emulate semantics on an x86
//!   CPU without VMX or SVM support.
//! - Implement network driver and work it on the kev.
//! - Implement the nested virtualization to run kev on kev.
//! - Support a smp in keos on kev.
//!
//! The work of students that do a particularly good project may be incorporated
//! into future assignments (a good way to get your name on the credits page!).
//! The project you choose must have a significant virtualization component. For
//! example, you shouldn't simply port a user-level application that requires
//! little or no kernel modification. You should email a proposal to the
//! instructor by the notified deadline. The proposal must include: (1) The
//! names of your group members; (2) What you want to do; and (3) What you are
//! expecting to present (a list of deliverables). Please keep it short (no more
//! than several paragraphs).

#![no_std]
#![no_main]
#![allow(clippy::missing_safety_doc)]

extern crate alloc;
extern crate keos;
extern crate keos_project2;
extern crate keos_project4;

extern crate kev;
extern crate kev_project1;
extern crate kev_project2;
extern crate kev_project3;

use keos::SystemConfigurationBuilder;
use keos_project4::round_robin::RoundRobin;

#[unsafe(no_mangle)]
pub unsafe fn main(config_builder: SystemConfigurationBuilder) {
    unsafe { kev::start_vmx_on_cpu().expect("Failed to initialize VMX.") }
    config_builder.set_scheduler(RoundRobin::new());

    if let Ok(fs) = simple_fs::FileSystem::load(1) {
        keos::info!("Filesystem: use `SimpleFS`.");
        keos::fs::FileSystem::register(fs)
    }
    todo!()
}

#[unsafe(no_mangle)]
pub unsafe fn ap_main() {
    unsafe { kev::start_vmx_on_cpu().expect("Failed to initialize VMX.") }
}
