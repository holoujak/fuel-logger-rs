use std::process::Command;

fn main() {
    // Re-run build script when web source files change
    println!("cargo:rerun-if-changed=web/src/");
    println!("cargo:rerun-if-changed=web/index.html");
    println!("cargo:rerun-if-changed=web/package.json");

    let install = Command::new("npm")
        .args(["ci"])
        .current_dir("web")
        .status()
        .expect("Failed to run npm. Is npm installed?");

    let status = Command::new("npm")
        .args(["run", "build"])
        .current_dir("web")
        .status()
        .expect("Failed to run npm. Is npm installed?");

    if !install.success() {
        panic!("npm install failed");
    }

    if !status.success() {
        panic!("Web build failed");
    }
}
