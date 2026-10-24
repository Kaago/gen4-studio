//! Fixtures for tests in this crate and downstream crates (feature `test-support`).
#![allow(clippy::unwrap_used)]

/// One test texture: width, height, format, color0 flag, texel bytes.
pub type Tex<'a> = (u32, u32, u8, bool, &'a [u8]);

fn put_u16(out: &mut Vec<u8>, v: u16) {
    out.extend_from_slice(&v.to_le_bytes());
}

fn put_u32(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_le_bytes());
}

fn name16(prefix: &str, i: usize) -> [u8; 16] {
    let mut n = [0u8; 16];
    let s = format!("{prefix}{i}");
    n[..s.len()].copy_from_slice(s.as_bytes());
    n
}

/// Info block: `count` entries of `datum` bytes (already encoded) plus names.
fn info_block(prefix: &str, data: &[Vec<u8>], datum_size: u16) -> Vec<u8> {
    let n = data.len();
    let mut out = vec![0u8, n as u8];
    put_u16(&mut out, (16 + 4 * n) as u16);
    put_u16(&mut out, 8);
    put_u16(&mut out, (4 + 4 * n) as u16);
    put_u32(&mut out, 0x017F);
    for i in 0..n {
        put_u32(&mut out, i as u32);
    }
    put_u16(&mut out, datum_size);
    put_u16(&mut out, (4 + datum_size as usize * n) as u16);
    for d in data {
        out.extend_from_slice(d);
    }
    for i in 0..n {
        out.extend_from_slice(&name16(prefix, i));
    }
    out
}

/// Builds a minimal BTX0 with a single TEX0 section (apicula layout).
pub fn btx0(textures: &[Tex], palettes: &[&[u16]]) -> Vec<u8> {
    const TEX0_HEADER: usize = 60;
    // Texel data block and texture info.
    let mut tex_block = Vec::new();
    let mut tex_entries = Vec::new();
    for &(w, h, format, color0, data) in textures {
        let off = tex_block.len();
        tex_block.extend_from_slice(data);
        while tex_block.len() % 8 != 0 {
            tex_block.push(0);
        }
        let wlog = (w / 8).trailing_zeros();
        let hlog = (h / 8).trailing_zeros();
        let params = (off as u32 >> 3)
            | (wlog << 20)
            | (hlog << 23)
            | ((format as u32) << 26)
            | ((color0 as u32) << 29);
        let mut e = Vec::new();
        put_u32(&mut e, params);
        put_u32(&mut e, 0);
        tex_entries.push(e);
    }
    // Palette data block and palette info.
    let mut pal_block = Vec::new();
    let mut pal_entries = Vec::new();
    for pal in palettes {
        let off = pal_block.len();
        for c in *pal {
            put_u16(&mut pal_block, *c);
        }
        while pal_block.len() % 8 != 0 {
            pal_block.push(0);
        }
        let mut e = Vec::new();
        put_u16(&mut e, (off >> 3) as u16);
        put_u16(&mut e, 0);
        pal_entries.push(e);
    }
    let tex_info = info_block("tex", &tex_entries, 8);
    let pal_info = info_block("pal", &pal_entries, 4);

    let tex_info_off = TEX0_HEADER;
    let pal_info_off = tex_info_off + tex_info.len();
    let tex_block_off = pal_info_off + pal_info.len();
    let pal_block_off = tex_block_off + tex_block.len();
    let tex0_size = pal_block_off + pal_block.len();

    let mut t = Vec::new();
    t.extend_from_slice(b"TEX0");
    put_u32(&mut t, tex0_size as u32);
    put_u32(&mut t, 0);
    put_u16(&mut t, (tex_block.len() >> 3) as u16);
    put_u16(&mut t, tex_info_off as u16);
    put_u32(&mut t, 0);
    put_u32(&mut t, tex_block_off as u32);
    put_u32(&mut t, 0);
    put_u16(&mut t, 0); // compressed block length
    put_u16(&mut t, 0); // compressed info offset
    put_u32(&mut t, 0);
    put_u32(&mut t, tex0_size as u32); // compressed1 offset
    put_u32(&mut t, tex0_size as u32); // compressed2 offset
    put_u32(&mut t, 0);
    put_u16(&mut t, (pal_block.len() >> 3) as u16);
    put_u16(&mut t, 0);
    put_u32(&mut t, pal_info_off as u32);
    put_u32(&mut t, pal_block_off as u32);
    assert_eq!(t.len(), TEX0_HEADER);
    t.extend_from_slice(&tex_info);
    t.extend_from_slice(&pal_info);
    t.extend_from_slice(&tex_block);
    t.extend_from_slice(&pal_block);

    let mut out = Vec::new();
    out.extend_from_slice(b"BTX0");
    put_u16(&mut out, 0xFEFF);
    put_u16(&mut out, 2);
    put_u32(&mut out, (20 + t.len()) as u32);
    put_u16(&mut out, 16);
    put_u16(&mut out, 1);
    put_u32(&mut out, 20);
    out.extend_from_slice(&t);
    out
}

/// A synthetic NCLR: header, one TTLP block with `depth` and `colours` (16-bit BGR555).
pub fn nclr_bytes(depth: u32, colours: &[u16]) -> Vec<u8> {
    let data = colours.len() * 2;
    let mut b = Vec::new();
    b.extend_from_slice(b"RLCN");
    b.extend_from_slice(&0xFEFFu16.to_le_bytes());
    b.extend_from_slice(&0x0100u16.to_le_bytes());
    b.extend_from_slice(&((0x28 + data) as u32).to_le_bytes());
    b.extend_from_slice(&0x10u16.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(b"TTLP");
    b.extend_from_slice(&((0x18 + data) as u32).to_le_bytes());
    b.extend_from_slice(&depth.to_le_bytes());
    b.extend_from_slice(&0u32.to_le_bytes());
    b.extend_from_slice(&(data as u32).to_le_bytes());
    b.extend_from_slice(&0x10u32.to_le_bytes());
    for c in colours {
        b.extend_from_slice(&c.to_le_bytes());
    }
    b
}

/// A synthetic NCGR with `tiles_x` by `tiles_y` tiles in the header (`0xFFFF` when unknown) and
/// the raw tile bytes `data`.
pub fn ncgr_bytes(tiles_x: u16, tiles_y: u16, depth: u32, data: &[u8]) -> Vec<u8> {
    let mut b = Vec::new();
    b.extend_from_slice(b"RGCN");
    b.extend_from_slice(&0xFEFFu16.to_le_bytes());
    b.extend_from_slice(&0x0101u16.to_le_bytes());
    b.extend_from_slice(&((0x30 + data.len()) as u32).to_le_bytes());
    b.extend_from_slice(&0x10u16.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(b"RAHC");
    b.extend_from_slice(&((0x20 + data.len()) as u32).to_le_bytes());
    b.extend_from_slice(&tiles_y.to_le_bytes());
    b.extend_from_slice(&tiles_x.to_le_bytes());
    b.extend_from_slice(&depth.to_le_bytes());
    b.extend_from_slice(&0x10u32.to_le_bytes()); // mapping
    b.extend_from_slice(&0u32.to_le_bytes()); // 0: tiled
    b.extend_from_slice(&(data.len() as u32).to_le_bytes());
    b.extend_from_slice(&0x18u32.to_le_bytes());
    b.extend_from_slice(data);
    b
}

/// Nitro resource dictionary with `names` and `unit`-byte data entries (the matching value
/// of `data`, else the entry index). The tree nodes are zero.
fn nitro_dict(names: &[&str], unit: u16, data: &[u32]) -> Vec<u8> {
    let n = names.len();
    let entry = 8 + 4 * (n + 1);
    let mut out = vec![0u8, n as u8];
    let total = entry + 4 + n * unit as usize + 16 * n;
    put_u16(&mut out, total as u16);
    put_u16(&mut out, 8);
    put_u16(&mut out, entry as u16);
    out.resize(entry, 0);
    put_u16(&mut out, unit);
    put_u16(&mut out, (4 + n * unit as usize) as u16);
    for i in 0..n {
        let v = data.get(i).copied().unwrap_or(i as u32);
        out.extend_from_slice(&v.to_le_bytes()[..unit as usize]);
    }
    for name in names {
        let mut rec = [0u8; 16];
        rec[..name.len()].copy_from_slice(name.as_bytes());
        out.extend_from_slice(&rec);
    }
    out
}

/// A minimal `MDL0` block: one model entry per name (all pointing at the same model),
/// whose material section binds `textures` and `palettes`. No geometry.
pub fn mdl0_block(models: &[&str], textures: &[&str], palettes: &[&str]) -> Vec<u8> {
    let dict_len = nitro_dict(models, 4, &[]).len();
    let model_off = 8 + dict_len; // relative to the block start
    let dict = nitro_dict(models, 4, &vec![model_off as u32; models.len()]);
    let mats = nitro_dict(&["mat"], 4, &[]);
    let tex = nitro_dict(textures, 4, &[]);
    let pal = nitro_dict(palettes, 4, &[]);
    let mut material = Vec::new();
    put_u16(&mut material, (4 + mats.len()) as u16);
    put_u16(&mut material, (4 + mats.len() + tex.len()) as u16);
    material.extend_from_slice(&mats);
    material.extend_from_slice(&tex);
    material.extend_from_slice(&pal);
    // Model header: size, ofs_sbc, ofs_mat, ofs_shp, ofs_evp.
    let model_len = 20 + material.len();
    let mut model = Vec::new();
    for v in [model_len as u32, 20, 20, model_len as u32, model_len as u32] {
        put_u32(&mut model, v);
    }
    model.extend_from_slice(&material);
    let mut out = b"MDL0".to_vec();
    put_u32(&mut out, 0);
    out.extend_from_slice(&dict);
    out.extend_from_slice(&model);
    while !out.len().is_multiple_of(4) {
        out.push(0);
    }
    let len = out.len() as u32;
    out[4..8].copy_from_slice(&len.to_le_bytes());
    out
}

/// A BMD0 container of `blocks` (each starting with its tag and size), version 2.
pub fn bmd0(blocks: &[Vec<u8>]) -> Vec<u8> {
    let table = 16 + 4 * blocks.len();
    let total = table + blocks.iter().map(Vec::len).sum::<usize>();
    let mut out = b"BMD0".to_vec();
    put_u16(&mut out, 0xFEFF);
    put_u16(&mut out, 2);
    put_u32(&mut out, total as u32);
    put_u16(&mut out, 16);
    put_u16(&mut out, blocks.len() as u16);
    let mut off = table;
    for b in blocks {
        put_u32(&mut out, off as u32);
        off += b.len();
    }
    for b in blocks {
        out.extend_from_slice(b);
    }
    out
}
