// Build script: hand the linker our script, and embed the ring-3 userland
// program (built first by `make`) into the kernel image if it exists.
use std::{env, fs, path::PathBuf};

fn main() {
    let dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    println!("cargo:rustc-link-arg-bins=-T{}", dir.join("linker.ld").display());
    println!("cargo:rerun-if-changed=linker.ld");

    let profile = env::var("PROFILE").unwrap_or_else(|_| "release".into());
    let user = dir.join("../target/x86_64-unknown-none").join(&profile).join("nexos-userland");
    println!("cargo:rerun-if-changed={}", user.display());
    let out = PathBuf::from(env::var("OUT_DIR").unwrap()).join("userland.elf");
    match fs::read(&user) {
        Ok(bytes) => fs::write(&out, bytes).unwrap(),
        Err(_) => fs::write(&out, b"").unwrap(),
    }
}
