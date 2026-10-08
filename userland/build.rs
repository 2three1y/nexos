use std::{env, path::PathBuf};
fn main() {
    let dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    println!("cargo:rustc-link-arg-bins=-T{}", dir.join("user.ld").display());
    println!("cargo:rerun-if-changed=user.ld");
}
