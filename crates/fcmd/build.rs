//! The icon of `fcmd.exe`.
//!
//! On Windows the program's icon is a resource linked into the executable. It is compiled
//! once, from `assets/fcmd.ico` through `packaging/windows/fcmd.rc`, into the two forms the
//! two Windows linkers take — `fcmd.res` for MSVC's, `fcmd-res.o` for MinGW's — and kept in
//! the repository, so that building needs no resource compiler and no crate to drive one.
//! Every other target builds as if this file were not here.

use std::env;
use std::path::PathBuf;

fn main() {
    let res = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("cargo sets it"))
        .join("../../packaging/windows");
    println!("cargo:rerun-if-changed={}", res.join("fcmd.res").display());
    println!(
        "cargo:rerun-if-changed={}",
        res.join("fcmd-res.o").display()
    );
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let file = match (
        env::var("CARGO_CFG_TARGET_ENV").as_deref(),
        env::var("CARGO_CFG_TARGET_ARCH").as_deref(),
    ) {
        // A `.res` file is the same for every processor: MSVC's linker converts it itself.
        (Ok("msvc"), _) => "fcmd.res",
        // MinGW's linker takes an object file, which is the processor's own.
        (Ok("gnu"), Ok("x86_64")) => "fcmd-res.o",
        _ => return,
    };
    println!("cargo:rustc-link-arg-bins={}", res.join(file).display());
}
