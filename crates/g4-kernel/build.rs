//! uxie's nitroarc links `libnitroarc_ffi` dynamically and Cargo places the library next to
//! the binaries without an rpath. Add one so `g4k` also runs outside `cargo run`/`cargo test`.
//! Before that, [`emit_lock_salts`] hands the kernel the locked versions of its tools.

fn main() {
    emit_lock_salts();
    let origin = match std::env::var("CARGO_CFG_TARGET_OS").as_deref() {
        Ok("macos") => "@executable_path",
        Ok("linux") => "$ORIGIN",
        _ => return,
    };
    println!("cargo:rustc-link-arg-bins=-Wl,-rpath,{origin}");
    println!("cargo:rerun-if-changed=build.rs");
}

/// Sets `G4_LOCK_DS_ROM`, `G4_LOCK_ROTOM`, `G4_LOCK_CHATOT` and `G4_LOCK_UXIE` to
/// `"<name> <version> <source>"` from the workspace `Cargo.lock` (the build steps' tool salts,
/// `src/build/salt.rs`), or to `"unknown"` when the lock file or the package is missing. The
/// lock file is read line by line: `[[package]]` starts a section, `name`, `version` and
/// `source` are its fields (a package without `source` is a workspace member).
fn emit_lock_salts() {
    const TOOLS: [(&str, &str); 4] = [
        ("G4_LOCK_DS_ROM", "ds-rom"),
        ("G4_LOCK_ROTOM", "rotom"),
        ("G4_LOCK_CHATOT", "chatot"),
        ("G4_LOCK_UXIE", "uxie"),
    ];
    let lock = std::fs::read_to_string("../../Cargo.lock").unwrap_or_default();
    let mut packages: Vec<[String; 3]> = Vec::new();
    for line in lock.lines() {
        let line = line.trim();
        if line == "[[package]]" {
            packages.push(Default::default());
            continue;
        }
        if line.starts_with('[') {
            // Another table (`[metadata]` in old lock files) ends the package section.
            packages.push(Default::default());
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let slot = match key.trim() {
            "name" => 0,
            "version" => 1,
            "source" => 2,
            _ => continue,
        };
        if let Some(package) = packages.last_mut() {
            package[slot] = value.trim().trim_matches('"').to_owned();
        }
    }
    for (var, name) in TOOLS {
        let value = packages
            .iter()
            .find(|[n, v, _]| n == name && !v.is_empty())
            .map(|[n, v, s]| format!("{n} {v} {s}"))
            .unwrap_or_else(|| "unknown".to_owned());
        println!("cargo:rustc-env={var}={value}");
    }
    println!("cargo:rerun-if-changed=../../Cargo.lock");
}
