use std::path::Path;

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
    // The rpath `@executable_path/../Frameworks` for the .app bundle (dylib in Contents/Frameworks)
    // is added by tauri-build itself, because `bundle.macOS.frameworks` is set. Adding it here as
    // well only makes the linker warn about a duplicate -rpath (`scripts/check-bundle.sh` asserts it).
    ensure_bundle_placeholders();
    println!("cargo:rerun-if-changed=build.rs");
    tauri_build::build()
}

/// `bundle.externalBin` and, on macOS targets, `bundle.macOS.frameworks` make tauri-build require
/// `binaries/apicula-<target triple>` and `binaries/rotom-lsp-<target triple>` (both with `.exe`
/// on Windows) plus `frameworks/libnitroarc_ffi.dylib` (macOS targets only) for every build, also
/// `cargo test`/`clippy`. The files are only real after `scripts/prepare-bundle.sh`; until then
/// empty placeholders keep normal builds working. `find_apicula` and `find_rotom_lsp` ignore an
/// empty file and `scripts/check-bundle.sh` fails on empty files, so a placeholder can never pass
/// as the real thing.
///
/// There is deliberately no placeholder for `windows/nitroarc_ffi.dll`: that resource is only
/// declared in `tauri.bundle.windows.conf.json` (used by `scripts/bundle.sh`), because tauri-build
/// copies every `bundle.resources` entry next to the binary on each build and an empty file would
/// overwrite the real DLL that uxie's build script stages there.
fn ensure_bundle_placeholders() {
    let Ok(triple) = std::env::var("TARGET") else {
        return;
    };
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    // tauri-build looks for the sidecars with `.exe` on Windows targets.
    let exe = if target_os == "windows" { ".exe" } else { "" };
    let mut paths = vec![
        format!("binaries/apicula-{triple}{exe}"),
        format!("binaries/rotom-lsp-{triple}{exe}"),
    ];
    // nitroarc stays a separate file: a framework on macOS (see the note above for Windows).
    if target_os == "macos" {
        paths.push("frameworks/libnitroarc_ffi.dylib".to_string());
    }
    for path in paths {
        let path = Path::new(&path);
        if !path.exists() {
            if let Some(dir) = path.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            let _ = std::fs::write(path, b"");
        }
    }
}
