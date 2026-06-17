use std::process::Command;
use std::path::{Path, PathBuf};
use std::env;

fn get_libs_dir() -> PathBuf {
    let home = env::var("HOME").expect("HOME not set");
    Path::new(&home).join(".lopy/libs")
}

fn compile_c_file(src_path: &Path, out_path: &Path, libs: &[&str], defines: &[&str]) {
    if !src_path.exists() {
        println!("⚠️  Файл {} не найден, пропускаем", src_path.display());
        return;
    }

    println!("cargo:rerun-if-changed={}", src_path.display());

    let mut cmd = Command::new("gcc");
    cmd.arg("-c");
    cmd.arg(src_path.to_str().unwrap());
    cmd.arg("-o");
    cmd.arg(out_path.to_str().unwrap());
    cmd.arg("-fPIC");

    for def in defines {
        cmd.arg(def);
    }

    for lib in libs {
        cmd.arg(lib);
    }

    let status = cmd.status().expect(&format!("Failed to compile {}", src_path.display()));
    if !status.success() {
        panic!("Failed to compile {}", src_path.display());
    }
}

fn main() {
    let libs_dir = get_libs_dir();

    let llgui_src = libs_dir.join("llgui/lib/llgui.c");
    let llgui_o = libs_dir.join("llgui/lib/llgui.o");

    #[cfg(target_os = "linux")]
    let llgui_defines = &["-D_GLFW_X11", "-D_GNU_SOURCE"];

    #[cfg(target_os = "windows")]
    let llgui_defines = &[];

    #[cfg(target_os = "macos")]
    let llgui_defines = &[];

    #[cfg(target_os = "linux")]
    let llgui_libs = &["-lX11", "-lXrandr", "-lGL", "-lGLX"];

    #[cfg(target_os = "windows")]
    let llgui_libs = &["-lgdi32", "-lopengl32", "-lwinmm"];

    #[cfg(target_os = "macos")]
    let llgui_libs = &["-framework Cocoa", "-framework OpenGL"];

    compile_c_file(&llgui_src, &llgui_o, llgui_libs, llgui_defines);

    let llstd_src = libs_dir.join("llstd/lib/llstd.c");
    let llstd_o = libs_dir.join("llstd/lib/llstd.o");

    #[cfg(target_os = "linux")]
    let llstd_defines = &[];

    #[cfg(target_os = "windows")]
    let llstd_defines = &[];

    #[cfg(target_os = "macos")]
    let llstd_defines = &[];

    #[cfg(target_os = "linux")]
    let llstd_libs = &["-lX11", "-lXrandr", "-lGL"];

    #[cfg(target_os = "windows")]
    let llstd_libs = &["-lgdi32", "-lopengl32"];

    #[cfg(target_os = "macos")]
    let llstd_libs = &["-framework Cocoa", "-framework OpenGL"];

    compile_c_file(&llstd_src, &llstd_o, llstd_libs, llstd_defines);

    println!("cargo:rustc-link-search=native={}", libs_dir.join("llgui/lib").display());
    println!("cargo:rustc-link-search=native={}", libs_dir.join("llstd/lib").display());
}