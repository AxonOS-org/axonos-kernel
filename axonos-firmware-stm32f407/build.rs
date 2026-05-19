// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright (c) 2026 Denis Yermakou <denis@axonos.org>
// Part of the AxonOS project — https://github.com/AxonOS-org

use std::env;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

fn main() {
    // Put memory.x in OUT_DIR for cortex-m-rt to find.
    let out = &PathBuf::from(env::var_os("OUT_DIR").unwrap());
    File::create(out.join("memory.x"))
        .unwrap()
        .write_all(include_bytes!("memory.x"))
        .unwrap();
    println!("cargo:rustc-link-search={}", out.display());

    // Re-run if memory.x changes.
    println!("cargo:rerun-if-changed=memory.x");
    println!("cargo:rerun-if-changed=build.rs");
}
