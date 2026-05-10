use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let target = env::var("TARGET").unwrap();
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    // Copy memory layout file for linker
    if target.starts_with("thumb") {
        let memory_x = PathBuf::from("memory.x");
        if memory_x.exists() {
            fs::copy(&memory_x, out_dir.join("memory.x")).unwrap();
            println!("cargo:rustc-link-search={}", out_dir.display());
        }
    }

    // Rebuild if configuration changes
    println!("cargo:rerun-if-changed=memory.x");
    println!("cargo:rerun-if-changed=link.x");
}
