use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=libs/llgui_lib/window.c");

    let status = Command::new("gcc")
        .args(&["-c", "libs/llgui_lib/window.c", "-o", "libs/llgui_lib/window.o", "-lX11"])
        .status()
        .expect("Failed to compile window.c");

    if !status.success() {
        panic!("Failed to compile window.c");
    }
    
    println!("cargo:rustc-env=LLGUI_LIB_PATH=libs/llgui_lib");
}