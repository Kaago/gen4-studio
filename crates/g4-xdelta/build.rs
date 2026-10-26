//! Compiles csrc/g4_xdelta.c, which includes the vendored xdelta3.c as one translation unit.
use std::env;

fn main() {
    let pointer_bytes = env::var("CARGO_CFG_TARGET_POINTER_WIDTH")
        .expect("cargo sets CARGO_CFG_TARGET_POINTER_WIDTH")
        .parse::<u32>()
        .expect("pointer width is a number")
        / 8;
    let windows = env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows");
    let sizeof_size_t = pointer_bytes.to_string();
    let sizeof_unsigned_long = if windows {
        "4".to_owned()
    } else {
        sizeof_size_t.clone()
    };

    let mut build = cc::Build::new();
    build
        .file("csrc/g4_xdelta.c")
        .include("vendor/xdelta3")
        .opt_level(2)
        .warnings(false)
        .define("XD3_ENCODER", "1")
        .define("XD3_MAIN", "0")
        .define("SECONDARY_DJW", "0")
        .define("SECONDARY_FGK", "0")
        .define("SECONDARY_LZMA", "0")
        .define("EXTERNAL_COMPRESSION", "0")
        .define("SHELL_TESTS", "0")
        .define("REGRESSION_TEST", "0")
        .define("XD3_DEBUG", "0")
        .define("XD3_USE_LARGEFILE64", "1")
        .define("SIZEOF_SIZE_T", sizeof_size_t.as_str())
        .define("SIZEOF_UNSIGNED_INT", "4")
        .define("SIZEOF_UNSIGNED_LONG", sizeof_unsigned_long.as_str())
        .define("SIZEOF_UNSIGNED_LONG_LONG", "8");
    if windows {
        build.define("XD3_WIN32", "1");
    }
    build.compile("g4_xdelta");

    println!("cargo:rerun-if-changed=csrc");
    println!("cargo:rerun-if-changed=vendor");
}
