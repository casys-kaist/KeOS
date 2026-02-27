use kev::vm::VmBuilder;
use kev_project2::keos_vm::VmState;

pub fn run_keos() {
    // VM with 256 MiB memory.
    let vm = VmBuilder::new(
        VmState::new(256 * 1024).expect("Failed to crate vmstate"),
        1,
    )
    .expect("Failed to create vmbuilder.")
    .finalize()
    .expect("Failed to create vm.");
    vm.start_bsp().expect("Failed to start bsp.");
    vm.join();
}
