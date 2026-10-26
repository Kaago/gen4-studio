//! xdelta3 (C, via the xdelta3 crate): encode base -> new, decode back, time and size.
use std::time::Instant;
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let src = std::fs::read(&a[1]).unwrap();
    let dst = std::fs::read(&a[2]).unwrap();
    let t = Instant::now();
    let patch = xdelta3::encode(&dst, &src);
    let enc = t.elapsed().as_millis();
    match patch {
        Some(p) => {
            std::fs::write(&a[3], &p).unwrap();
            let t = Instant::now();
            let back = xdelta3::decode(&p, &src);
            let dec = t.elapsed().as_millis();
            println!("{{\"encode_ms\":{enc},\"decode_ms\":{dec},\"patch_bytes\":{},\"roundtrip_ok\":{}}}", p.len(), back.as_deref() == Some(&dst[..]));
        }
        None => println!("{{\"encode_ms\":{enc},\"error\":\"encode returned None\"}}"),
    }
}
