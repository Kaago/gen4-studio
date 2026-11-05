//! Round trips through the vendored xdelta3, with inputs from a small LCG (no files in the repo).
use g4_xdelta::{XdeltaError, app_header, decode, encode, has_secondary_compression};
use std::process::Command;

const MIB: usize = 1 << 20;
const APP_HEADER: &[u8] = b"neu.nds//alt [abc].nds/";

/// `len` pseudo-random bytes; different seeds give unrelated data.
fn lcg_bytes(seed: u64, len: usize) -> Vec<u8> {
    let mut state = seed;
    (0..len)
        .map(|_| {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            (state >> 56) as u8
        })
        .collect()
}

/// A 1 MiB source and a target with 16 changed bytes and 600 bytes inserted in the middle.
fn source_and_changed_target() -> (Vec<u8>, Vec<u8>) {
    let source = lcg_bytes(1, MIB);
    let mut target = source.clone();
    for i in 0..16 {
        let at = 100_000 + i * 7_919;
        target[at] = !target[at];
    }
    let middle = MIB / 2;
    let inserted = lcg_bytes(2, 600);
    target.splice(middle..middle, inserted);
    (source, target)
}

#[test]
fn round_trips_a_changed_megabyte() {
    let (source, target) = source_and_changed_target();
    let patch = encode(&source, &target, b"").unwrap();
    eprintln!("patch for a changed megabyte: {} bytes", patch.len());
    assert!(patch.len() < 4096, "patch has {} bytes", patch.len());
    assert_eq!(decode(&source, &patch).unwrap(), target);
}

#[test]
fn writes_the_app_header_and_no_secondary_compression() {
    let (source, target) = source_and_changed_target();
    let patch = encode(&source, &target, APP_HEADER).unwrap();
    assert_eq!(app_header(&patch), Some(APP_HEADER));
    assert!(!has_secondary_compression(&patch));
    assert_eq!(decode(&source, &patch).unwrap(), target);
}

#[test]
fn refuses_a_patch_on_the_wrong_source() {
    let (source, target) = source_and_changed_target();
    let patch = encode(&source, &target, b"").unwrap();
    // Offset 10 000 lies in a range the patch copies from the source unchanged.
    let mut wrong = source.clone();
    wrong[10_000] = !wrong[10_000];
    match decode(&wrong, &patch) {
        Ok(out) => panic!("decoded {} bytes", out.len()),
        Err(err) => {
            eprintln!("wrong source refused: {err}");
            assert!(err.to_string().contains("checksum"), "not Adler32: {err}");
        }
    }
}

#[test]
fn refuses_a_corrupted_patch() {
    let (source, target) = source_and_changed_target();
    let mut patch = encode(&source, &target, b"").unwrap();
    let at = patch.len() - patch.len() / 8;
    patch[at] ^= 0x5A;
    match decode(&source, &patch) {
        Ok(out) => panic!("decoded {} bytes", out.len()),
        Err(err) => eprintln!("corrupted patch refused: {err}"),
    }
}

#[test]
fn refuses_input_that_is_not_a_vcdiff_patch() {
    let source = lcg_bytes(5, 4096);
    let not_vcdiff = XdeltaError::Failed("not a VCDIFF patch".to_owned());
    assert_eq!(decode(&source, b""), Err(not_vcdiff.clone()));
    assert_eq!(decode(&source, &source), Err(not_vcdiff));
}

#[test]
fn grows_the_output_for_an_unrelated_target() {
    let source = lcg_bytes(3, MIB);
    let target = lcg_bytes(4, MIB);
    let patch = encode(&source, &target, APP_HEADER).unwrap();
    eprintln!("patch for an unrelated megabyte: {} bytes", patch.len());
    // Larger than the first buffer of max(64 KiB, target / 16).
    assert!(patch.len() > 64 * 1024);
    assert_eq!(decode(&source, &patch).unwrap(), target);
}

#[test]
fn xdelta3_cli_applies_our_patch() {
    if !Command::new("xdelta3")
        .arg("-V")
        .output()
        .is_ok_and(|o| o.status.success())
    {
        eprintln!("skipped: no xdelta3 in PATH");
        return;
    }
    let (source, target) = source_and_changed_target();
    let patch = encode(&source, &target, APP_HEADER).unwrap();

    // A temp folder outside target/: no `.nds` or `.xdelta` may land in the cargo cache of CI.
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let (src, pat, out) = (
        dir.join("alt.nds"),
        dir.join("neu.xdelta"),
        dir.join("neu.nds"),
    );
    std::fs::write(&src, &source).unwrap();
    std::fs::write(&pat, &patch).unwrap();

    let run = Command::new("xdelta3")
        .args(["-d", "-f", "-s"])
        .args([&src, &pat, &out])
        .output()
        .unwrap();
    assert!(
        run.status.success(),
        "xdelta3 -d failed: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert!(std::fs::read(&out).unwrap() == target, "CLI output differs");
}
