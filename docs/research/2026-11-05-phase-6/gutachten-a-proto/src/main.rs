//! Gutachten A (Phase 6): Prototyp eines NSBTX-Writers (PNG/RGBA -> BTX0) mit Quantisierung,
//! Patricia-Baum der Wörterbücher und Rückprüfung über den Leser von `g4-formats`.
//!
//! Befehle (Ausgabe nur Zahlen und Namen):
//!   synth <aus.nsbtx>             synthetische Testbilder (keine ROM-Daten) quantisieren,
//!                                 schreiben, mit g4-formats zurücklesen und vergleichen
//!   png <aus.nsbtx> <a.png>...    eigene PNGs ebenso (Name = Dateistamm, höchstens 16 Byte)
//!   cmp <apicula.png> <x.nsbtx> <textur>
//!                                 PNG, das apicula aus dem NSBTX dekodiert hat, gegen die
//!                                 Dekodierung von g4-formats (Pixel für Pixel)
//!   dicts <ordner>                Wörterbücher (Texturen, Paletten) aller BTX0 im Ordner mit
//!                                 diesem Writer neu schreiben und byte-genau vergleichen
//!
//! Layout und Bitfelder: docs/formats/nsbtx.md, apicula (0BSD) als Layout-Referenz, eigene
//! Messungen (gutachten-a-nitro.py). Kein Code aus anderen Werkzeugen.

use std::path::Path;

use g4_formats::nsbtx::Nsbtx;

// ------------------------------------------------------------------------------------------
// Wörterbuch mit Patricia-Baum

type Name = [u8; 16];

fn name16(s: &str) -> Name {
    let mut n = [0u8; 16];
    let b = s.as_bytes();
    assert!(b.len() <= 16, "name longer than 16 bytes: {s}");
    n[..b.len()].copy_from_slice(b);
    n
}

fn bit(name: &Name, b: u8) -> u8 {
    let i = (b as usize >> 5) * 4;
    let w = u32::from_le_bytes([name[i], name[i + 1], name[i + 2], name[i + 3]]);
    ((w >> (b & 31)) & 1) as u8
}

/// Knoten (refBit, links, rechts, Eintrag). Knoten 0 ist die Wurzel (refBit 127); Einfügen in
/// Eintragsreihenfolge, danach Nummerierung in Vorordnung über die Abwärtskanten (gemessen:
/// so liegen alle Vanilla-Wörterbücher vor).
fn build_tree(names: &[Name]) -> Vec<[u8; 4]> {
    let mut nodes: Vec<[u8; 4]> = vec![[127, 0, 0, 0]];
    let zero = [0u8; 16];
    for (ei, key) in names.iter().enumerate() {
        let key_of = |nodes: &Vec<[u8; 4]>, i: usize| -> Name {
            if i == 0 { zero } else { names[nodes[i][3] as usize] }
        };
        let (mut p, mut x) = (0usize, nodes[0][1] as usize);
        while nodes[p][0] > nodes[x][0] {
            p = x;
            x = if bit(key, nodes[x][0]) == 1 { nodes[x][2] } else { nodes[x][1] } as usize;
        }
        let other = key_of(&nodes, x);
        let diff = (0..=127u8).rev().find(|&b| bit(key, b) != bit(&other, b)).expect("duplicate name");
        let (mut p, mut x) = (0usize, nodes[0][1] as usize);
        while nodes[p][0] > nodes[x][0] && nodes[x][0] > diff {
            p = x;
            x = if bit(key, nodes[x][0]) == 1 { nodes[x][2] } else { nodes[x][1] } as usize;
        }
        let t = nodes.len() as u8;
        if bit(key, diff) == 1 {
            nodes.push([diff, x as u8, t, ei as u8]);
        } else {
            nodes.push([diff, t, x as u8, ei as u8]);
        }
        if p == 0 {
            nodes[0][1] = t;
        } else if bit(key, nodes[p][0]) == 1 {
            nodes[p][2] = t;
        } else {
            nodes[p][1] = t;
        }
    }
    let mut order = vec![0usize];
    fn visit(nodes: &[[u8; 4]], i: usize, order: &mut Vec<usize>) {
        order.push(i);
        for c in [nodes[i][1] as usize, nodes[i][2] as usize] {
            if c != 0 && nodes[c][0] < nodes[i][0] {
                visit(nodes, c, order);
            }
        }
    }
    if !names.is_empty() {
        visit(&nodes, nodes[0][1] as usize, &mut order);
    }
    let mut new_of = vec![0u8; nodes.len()];
    for (new, &old) in order.iter().enumerate() {
        new_of[old] = new as u8;
    }
    order
        .iter()
        .map(|&old| {
            let [r, l, rt, e] = nodes[old];
            [r, new_of[l as usize], new_of[rt as usize], e]
        })
        .collect()
}

fn write_dict(entries: &[Vec<u8>], names: &[Name], unit: u16) -> Vec<u8> {
    let n = names.len();
    assert!(n < 256 && entries.len() == n);
    let mut out = vec![0u8, n as u8, 0, 0, 8, 0, 0, 0];
    for node in build_tree(names) {
        out.extend_from_slice(&node);
    }
    let ofs_entry = out.len() as u16;
    out[6..8].copy_from_slice(&ofs_entry.to_le_bytes());
    out.extend_from_slice(&unit.to_le_bytes());
    out.extend_from_slice(&(4 + unit as usize * n).to_le_bytes()[..2]);
    for e in entries {
        assert_eq!(e.len(), unit as usize);
        out.extend_from_slice(e);
    }
    for nm in names {
        out.extend_from_slice(nm);
    }
    let size = out.len() as u16;
    out[2..4].copy_from_slice(&size.to_le_bytes());
    out
}

// ------------------------------------------------------------------------------------------
// Quantisierung

#[derive(Clone)]
struct Img {
    name: String,
    w: u32,
    h: u32,
    rgba: Vec<u8>,
}

fn to555(c: [u8; 3]) -> u16 {
    (c[0] as u16 >> 3) | ((c[1] as u16 >> 3) << 5) | ((c[2] as u16 >> 3) << 10)
}

fn from555(v: u16) -> [u8; 3] {
    let e = |x: u16| -> u8 { ((x << 3) | (x >> 2)) as u8 };
    [e(v & 31), e((v >> 5) & 31), e((v >> 10) & 31)]
}

/// Median-Cut über die deckenden Pixel (in BGR555 gerundet), höchstens `k` Farben.
fn median_cut(px: &[[u8; 3]], k: usize) -> Vec<u16> {
    let mut uniq: Vec<[u8; 3]> = px.iter().map(|&c| from555(to555(c))).collect();
    uniq.sort();
    uniq.dedup();
    if uniq.len() <= k {
        return uniq.iter().map(|&c| to555(c)).collect();
    }
    let mut boxes: Vec<Vec<[u8; 3]>> = vec![px.to_vec()];
    while boxes.len() < k {
        // größte Spannweite teilen
        let (bi, ch, _) = boxes
            .iter()
            .enumerate()
            .flat_map(|(i, b)| {
                (0..3).map(move |c| {
                    let lo = b.iter().map(|p| p[c]).min().unwrap_or(0);
                    let hi = b.iter().map(|p| p[c]).max().unwrap_or(0);
                    (i, c, hi - lo)
                })
            })
            .max_by_key(|&(_, _, r)| r)
            .unwrap();
        let mut b = boxes.swap_remove(bi);
        if b.len() < 2 {
            boxes.push(b);
            break;
        }
        b.sort_by_key(|p| p[ch]);
        let tail = b.split_off(b.len() / 2);
        boxes.push(b);
        boxes.push(tail);
    }
    let mut pal: Vec<u16> = boxes
        .iter()
        .filter(|b| !b.is_empty())
        .map(|b| {
            let n = b.len() as u32;
            let s = b.iter().fold([0u32; 3], |a, p| [a[0] + p[0] as u32, a[1] + p[1] as u32, a[2] + p[2] as u32]);
            to555([(s[0] / n) as u8, (s[1] / n) as u8, (s[2] / n) as u8])
        })
        .collect();
    pal.sort();
    pal.dedup();
    pal
}

fn nearest(pal: &[u16], c: [u8; 3]) -> usize {
    pal.iter()
        .enumerate()
        .min_by_key(|&(_, &p)| {
            let q = from555(p);
            (0..3).map(|i| (q[i] as i32 - c[i] as i32).pow(2)).sum::<i32>()
        })
        .map(|(i, _)| i)
        .unwrap_or(0)
}

struct Quantized {
    fmt: u8,
    color0: bool,
    pal: Vec<u16>,
    idx: Vec<u8>,
    err: f64,
}

enum Method {
    MedianCut,
    NeuQuant,
}

/// Palettentextur: 16 Farben (Format 3) oder, wenn `four`, 4 Farben (Format 2). Pixel mit
/// Alpha < 128 bekommen Index 0, die Textur dann `color0` (Index 0 durchsichtig).
fn quantize(img: &Img, four: bool, method: &Method) -> Quantized {
    let colors = if four { 4 } else { 16 };
    let n = (img.w * img.h) as usize;
    let transparent = (0..n).any(|i| img.rgba[4 * i + 3] < 128);
    let opaque: Vec<[u8; 3]> = (0..n)
        .filter(|&i| img.rgba[4 * i + 3] >= 128)
        .map(|i| [img.rgba[4 * i], img.rgba[4 * i + 1], img.rgba[4 * i + 2]])
        .collect();
    let k = colors - transparent as usize;
    let mut pal = match method {
        Method::MedianCut => median_cut(&opaque, k),
        Method::NeuQuant => {
            let mut flat = Vec::with_capacity(opaque.len() * 4);
            for c in &opaque {
                flat.extend_from_slice(&[c[0], c[1], c[2], 255]);
            }
            let nq = color_quant::NeuQuant::new(10, k.max(1), &flat);
            let mut p: Vec<u16> = nq.color_map_rgb().chunks(3).map(|c| to555([c[0], c[1], c[2]])).collect();
            p.sort();
            p.dedup();
            p
        }
    };
    if transparent {
        pal.insert(0, 0);
    }
    let base = transparent as usize;
    let mut idx = vec![0u8; n];
    let mut err = 0f64;
    for i in 0..n {
        if img.rgba[4 * i + 3] < 128 {
            continue;
        }
        let c = [img.rgba[4 * i], img.rgba[4 * i + 1], img.rgba[4 * i + 2]];
        let j = base + nearest(&pal[base..], c);
        idx[i] = j as u8;
        let q = from555(pal[j]);
        err += (0..3).map(|k| (q[k] as f64 - c[k] as f64).abs()).sum::<f64>() / 3.0;
    }
    while pal.len() < colors {
        pal.push(0);
    }
    Quantized { fmt: if four { 2 } else { 3 }, color0: transparent, pal, idx, err: err / opaque.len().max(1) as f64 }
}

fn pack_texels(q: &Quantized) -> Vec<u8> {
    let bpp = if q.fmt == 2 { 2 } else { 4 };
    let per = 8 / bpp;
    q.idx
        .chunks(per)
        .map(|c| c.iter().enumerate().fold(0u8, |a, (i, &v)| a | (v << (i * bpp))))
        .collect()
}

// ------------------------------------------------------------------------------------------
// Writer

struct TexOut {
    name: Name,
    w: u32,
    h: u32,
    fmt: u8,
    color0: bool,
    texels: Vec<u8>,
}

struct PalOut {
    name: Name,
    colors: Vec<u16>,
    four: bool,
}

fn log2_size(v: u32) -> u32 {
    assert!(v.is_power_of_two() && (8..=1024).contains(&v), "texture side {v}");
    v.trailing_zeros() - 3
}

/// BTX0 mit einem TEX0, Layout wie Vanilla: Kopf 0x3C, Texturen-Wörterbuch, Paletten-
/// Wörterbuch, Texelblock, (leere 4x4-Blöcke), Palettenblock.
fn write_btx0(texs: &[TexOut], pals: &[PalOut]) -> Vec<u8> {
    let mut texel = Vec::new();
    let mut tex_entries = Vec::new();
    for t in texs {
        while texel.len() % 8 != 0 {
            texel.push(0);
        }
        let off8 = (texel.len() / 8) as u32;
        assert!(off8 <= 0xFFFF);
        let params = off8 | log2_size(t.w) << 20 | log2_size(t.h) << 23 | (t.fmt as u32) << 26 | (t.color0 as u32) << 29;
        let extra = t.w | t.h << 11 | 1 << 31;
        tex_entries.push([params.to_le_bytes(), extra.to_le_bytes()].concat());
        texel.extend_from_slice(&t.texels);
    }
    while texel.len() % 8 != 0 {
        texel.push(0);
    }
    let mut palb = Vec::new();
    let mut pal_entries = Vec::new();
    for p in pals {
        while palb.len() % 8 != 0 {
            palb.push(0);
        }
        let off8 = (palb.len() / 8) as u16;
        pal_entries.push([off8.to_le_bytes(), (p.four as u16).to_le_bytes()].concat());
        for c in &p.colors {
            palb.extend_from_slice(&c.to_le_bytes());
        }
    }
    while palb.len() % 8 != 0 {
        palb.push(0);
    }
    let tnames: Vec<Name> = texs.iter().map(|t| t.name).collect();
    let pnames: Vec<Name> = pals.iter().map(|p| p.name).collect();
    let tdict = write_dict(&tex_entries, &tnames, 8);
    let pdict = write_dict(&pal_entries, &pnames, 4);
    let tex_info = 0x3Cu32;
    let pal_info = tex_info + tdict.len() as u32;
    let tex_data = pal_info + pdict.len() as u32;
    let after = tex_data + texel.len() as u32;
    let pal_data = after;
    let size = pal_data + palb.len() as u32;
    let mut h = vec![0u8; 0x3C];
    let put32 = |h: &mut Vec<u8>, o: usize, v: u32| h[o..o + 4].copy_from_slice(&v.to_le_bytes());
    let put16 = |h: &mut Vec<u8>, o: usize, v: u16| h[o..o + 2].copy_from_slice(&v.to_le_bytes());
    h[0..4].copy_from_slice(b"TEX0");
    put32(&mut h, 4, size);
    put16(&mut h, 0x0C, (texel.len() / 8) as u16);
    put16(&mut h, 0x0E, tex_info as u16);
    put32(&mut h, 0x14, tex_data);
    put16(&mut h, 0x1E, tex_info as u16);
    put32(&mut h, 0x24, after);
    put32(&mut h, 0x28, after);
    put16(&mut h, 0x30, (palb.len() / 8) as u16);
    put16(&mut h, 0x32, if pals.iter().any(|p| p.four) { 0x8000 } else { 0 });
    put32(&mut h, 0x34, pal_info);
    put32(&mut h, 0x38, pal_data);
    let mut out = b"BTX0".to_vec();
    out.extend_from_slice(&0xFEFFu16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&0x14u32.to_le_bytes());
    out.extend_from_slice(&h);
    out.extend_from_slice(&tdict);
    out.extend_from_slice(&pdict);
    out.extend_from_slice(&texel);
    out.extend_from_slice(&palb);
    let n = out.len() as u32;
    out[8..12].copy_from_slice(&n.to_le_bytes());
    out
}

// ------------------------------------------------------------------------------------------
// Testbilder (synthetisch, keine ROM-Daten)

fn synth_images() -> Vec<(Img, bool)> {
    let mut v = Vec::new();
    let mk = |name: &str, w: u32, h: u32, f: &dyn Fn(u32, u32) -> [u8; 4]| {
        let mut rgba = Vec::with_capacity((w * h * 4) as usize);
        for y in 0..h {
            for x in 0..w {
                rgba.extend_from_slice(&f(x, y));
            }
        }
        Img { name: name.into(), w, h, rgba }
    };
    v.push((mk("syn_gras", 32, 32, &|x, y| {
        let n = ((x * 7 + y * 13) ^ (x * y)) % 23;
        [40 + (n * 3) as u8, 120 + (y * 3) as u8, 30 + (x * 2) as u8, 255]
    }), false));
    v.push((mk("syn_zaun", 16, 16, &|x, y| {
        if (x + y) % 5 == 0 || x == 0 || x == 15 { [150, 90, 40, 255] } else if y < 4 { [0, 0, 0, 0] } else { [200, 160, 100 + (y * 4) as u8, 255] }
    }), false));
    v.push((mk("syn_weg", 8, 8, &|x, y| {
        let c = [[90, 80, 70], [130, 120, 100], [180, 170, 150], [60, 50, 40]][((x / 2 + y) % 4) as usize];
        [c[0], c[1], c[2], 255]
    }), true));
    v.push((mk("syn_himmel", 64, 64, &|x, y| [((x * 4) % 256) as u8, ((y * 4) % 256) as u8, 200, 255]), false));
    v
}

fn build(images: &[(Img, bool)], method: &Method) -> (Vec<u8>, Vec<Quantized>) {
    let mut texs = Vec::new();
    let mut pals = Vec::new();
    let mut qs = Vec::new();
    for (img, four) in images {
        let q = quantize(img, *four, method);
        texs.push(TexOut { name: name16(&img.name), w: img.w, h: img.h, fmt: q.fmt, color0: q.color0, texels: pack_texels(&q) });
        pals.push(PalOut { name: name16(&format!("{}_pl", img.name)), colors: q.pal.clone(), four: *four });
        qs.push(q);
    }
    (write_btx0(&texs, &pals), qs)
}

/// Rücklesen mit g4-formats und Pixelvergleich gegen die Quantisierung.
fn verify(bytes: &[u8], images: &[(Img, bool)], qs: &[Quantized]) -> Result<usize, String> {
    let n = Nsbtx::read(bytes).map_err(|e| format!("g4-formats read: {e}"))?;
    let mut bad = 0;
    for (ti, ((img, _), q)) in images.iter().zip(qs).enumerate() {
        let t = &n.textures[ti];
        if t.name != img.name || t.width != img.w || t.height != img.h || t.format != q.fmt || t.color0_transparent != q.color0 {
            return Err(format!("texture info differs for {}", img.name));
        }
        let pi = n.palettes.iter().position(|p| p.name == format!("{}_pl", img.name)).ok_or("palette missing")?;
        let rgba = n.decode(ti, pi).map_err(|e| format!("decode: {e}"))?;
        for (i, &ix) in q.idx.iter().enumerate() {
            let px = &rgba.pixels[4 * i..4 * i + 4];
            let want_a = if q.color0 && ix == 0 { 0 } else { 255 };
            let want = from555(q.pal[ix as usize]);
            if px[3] != want_a || (want_a == 255 && px[..3] != want) {
                bad += 1;
            }
        }
    }
    Ok(bad)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("synth") | Some("png") => {
            let out = &args[2];
            let images: Vec<(Img, bool)> = if args[1] == "synth" {
                synth_images()
            } else {
                args[3..]
                    .iter()
                    .map(|p| {
                        let im = image::open(p).expect("png").to_rgba8();
                        let name = Path::new(p).file_stem().unwrap().to_string_lossy().into_owned();
                        (Img { name, w: im.width(), h: im.height(), rgba: im.into_raw() }, false)
                    })
                    .collect()
            };
            for (label, method) in [("median_cut", Method::MedianCut), ("neuquant", Method::NeuQuant)] {
                let (bytes, qs) = build(&images, &method);
                let bad = verify(&bytes, &images, &qs);
                println!(
                    "{label}: {} Byte, Texturen {}, Fehler je Kanal (Mittel) {:?}, Farben {:?}, Pixel falsch nach Rücklesen {:?}",
                    bytes.len(),
                    qs.len(),
                    qs.iter().map(|q| (q.err * 10.0).round() / 10.0).collect::<Vec<_>>(),
                    qs.iter().map(|q| q.pal.iter().collect::<std::collections::BTreeSet<_>>().len()).collect::<Vec<_>>(),
                    bad
                );
                if label == "median_cut" {
                    std::fs::write(out, &bytes).expect("write");
                }
            }
        }
        Some("cmp") => {
            let png = image::open(&args[2]).expect("png").to_rgba8();
            let bytes = std::fs::read(&args[3]).expect("nsbtx");
            let n = Nsbtx::read(&bytes).expect("read");
            let ti = n.textures.iter().position(|t| t.name == args[4]).expect("texture");
            let pi = n.palettes.iter().position(|p| p.name == format!("{}_pl", args[4])).expect("palette");
            let d = n.decode(ti, pi).expect("decode");
            let mut bad = 0;
            for (a, b) in png.as_raw().chunks(4).zip(d.pixels.chunks(4)) {
                if a[3] != b[3] || (a[3] != 0 && a[..3] != b[..3]) {
                    bad += 1;
                }
            }
            println!("cmp {}: {}x{} apicula vs {}x{} g4-formats, abweichende Pixel {bad}", args[4], png.width(), png.height(), d.width, d.height);
        }
        Some("dicts") => {
            let (mut total, mut equal, mut files) = (0, 0, 0);
            let mut paths: Vec<_> = std::fs::read_dir(&args[2]).expect("dir").map(|e| e.unwrap().path()).collect();
            paths.sort();
            for p in paths {
                let b = std::fs::read(&p).unwrap();
                if b.len() < 0x14 + 0x3C || &b[0..4] != b"BTX0" {
                    continue;
                }
                files += 1;
                let t = u32::from_le_bytes(b[0x10..0x14].try_into().unwrap()) as usize;
                let u16a = |o: usize| u16::from_le_bytes([b[o], b[o + 1]]) as usize;
                let u32a = |o: usize| u32::from_le_bytes(b[o..o + 4].try_into().unwrap()) as usize;
                for (o, unit) in [(t + u16a(t + 0x0E), 8usize), (t + u32a(t + 0x34), 4usize)] {
                    let n = b[o + 1] as usize;
                    let e = o + u16a(o + 6);
                    let ofs_name = u16a(e + 2);
                    let entries: Vec<Vec<u8>> = (0..n).map(|k| b[e + 4 + unit * k..e + 4 + unit * (k + 1)].to_vec()).collect();
                    let names: Vec<Name> = (0..n).map(|k| b[e + ofs_name + 16 * k..e + ofs_name + 16 * (k + 1)].try_into().unwrap()).collect();
                    let raw = &b[o..e + ofs_name + 16 * n];
                    total += 1;
                    if write_dict(&entries, &names, unit as u16) == raw {
                        equal += 1;
                    }
                }
            }
            println!("dicts: {files} BTX0, Wörterbücher {total}, byte-gleich {equal}");
        }
        _ => eprintln!("usage: synth <out> | png <out> <png>... | cmp <png> <nsbtx> <tex> | dicts <dir>"),
    }
}
