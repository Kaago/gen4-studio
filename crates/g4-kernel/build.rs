//! uxie's nitroarc links `libnitroarc_ffi` dynamically and Cargo places the library next to
//! the binaries without an rpath. Add one so `g4k` also runs outside `cargo run`/`cargo test`.

fn main() {
    let origin = match std::env::var("CARGO_CFG_TARGET_OS").as_deref() {
        Ok("macos") => "@executable_path",
        Ok("linux") => "$ORIGIN",
        _ => return,
    };
    println!("cargo:rustc-link-arg-bins=-Wl,-rpath,{origin}");
    println!("cargo:rerun-if-changed=build.rs");
}
