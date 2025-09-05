include!("../../build.rs");
use std::env;
use std::process::Command;

fn main() {
    let user_dir = Path::new(&env::var("CARGO_MANIFEST_DIR").unwrap()).join("userprog");

    let output = Command::new("make")
        .current_dir(&user_dir)
        .output()
        .expect("Failed to execute make");

    if !output.status.success() {
        panic!("make failed: {}", String::from_utf8_lossy(&output.stderr));
    }

    build_simple_fs("sfs.bin");
}
