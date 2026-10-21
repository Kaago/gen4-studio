fn main() {
    // uxie links `libnitroarc_ffi` dynamically and Cargo places it next to the binaries without
    // an rpath. Add one (same as crates/g4-kernel/build.rs) so the app finds it in target/debug.
    let origin = match std::env::var("CARGO_CFG_TARGET_OS").as_deref() {
        Ok("macos") => Some("@executable_path"),
        Ok("linux") => Some("$ORIGIN"),
        _ => None,
    };
    if let Some(origin) = origin {
        println!("cargo:rustc-link-arg-bins=-Wl,-rpath,{origin}");
    }
    println!("cargo:rerun-if-changed=build.rs");
    tauri_build::build()
}
