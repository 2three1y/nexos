// Build script: hand the linker our script, and embed the ring-3 userland
// programs (built first by `make`) into the kernel image if they exist.
use std::{env, fs, path::PathBuf};

fn main() {
    let dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    println!("cargo:rustc-link-arg-bins=-T{}", dir.join("linker.ld").display());
    println!("cargo:rerun-if-changed=linker.ld");
    println!("cargo:rerun-if-changed=catalog");

    let profile = env::var("PROFILE").unwrap_or_else(|_| "release".into());
    // x86_64-unknown-none, or i686-nexos for the 32-bit build (targets/i686-nexos.json)
    let target = env::var("TARGET").unwrap_or_else(|_| "x86_64-unknown-none".into());
    let bin_dir = dir.join("../target").join(&target).join(&profile);
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    // (binary built from userland/, file name that user.rs embeds)
    for (bin, embed) in [("nexos-userland", "userland.elf"), ("guess", "guess.elf")] {
        let src = bin_dir.join(bin);
        println!("cargo:rerun-if-changed={}", src.display());
        let bytes = fs::read(&src).unwrap_or_default();
        fs::write(out_dir.join(embed), bytes).unwrap();
    }
}
