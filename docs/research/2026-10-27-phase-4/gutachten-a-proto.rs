//! Prototype for Gutachten A (phase 4.3): export the decodable entities of a DSPRE project
//! into one JSON file per entity (binary parts as blob files), import them back, compare
//! the bytes, and time both directions. Usage: proto-a <project> <out-dir>
//! Writes ROM-derived files only into <out-dir> (scratchpad), never into the repo.

use std::collections::BTreeMap;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::time::Instant;

use g4_formats::area_data::AreaData;
use g4_formats::data::evolution::Evolution;
use g4_formats::data::item::Item;
use g4_formats::data::learnset::{LearnEntry, Learnset};
use g4_formats::data::moves::Move;
use g4_formats::data::personal::Personal;
use g4_formats::data::record::{FieldKind, RawRecord};
use g4_formats::data::encounter::{HgssEncounters, PtEncounters};
use g4_formats::data::trainer::TrainerProps;
use g4_formats::data::trainer_doc::TrainerDoc;
use g4_formats::data::trainer_text::{TrainerTextOffsets, TrainerTextTable};
use g4_formats::game::Family;
use g4_formats::map::{Building, Cell, MapFile};
use g4_formats::matrix::Matrix;
use g4_kernel::data::names::{Bank, bank_index};
use g4_kernel::project::{AssetKind, Project};
use g4_kernel::text::TextArchiveDoc;
use serde_json::{Value, json};
use sha1::{Digest, Sha1};
use uxie::event_file::{BgEvent, CoordEvent, EventFile, ObjectEvent, WarpEvent};

#[derive(Default, Debug)]
struct Stat {
    members: usize,
    json_files: usize,
    json_bytes: u64,
    blob_files: usize,
    blob_bytes: u64,
    export_ms: f64,
    import_ms: f64,
    same: usize,
    differ: Vec<u16>,
    notes: BTreeMap<String, usize>,
}

impl Stat {
    fn note(&mut self, k: &str) {
        *self.notes.entry(k.to_owned()).or_default() += 1;
    }
    fn to_json(&self) -> Value {
        json!({
            "members": self.members, "json_files": self.json_files, "json_bytes": self.json_bytes,
            "blob_files": self.blob_files, "blob_bytes": self.blob_bytes,
            "export_ms": (self.export_ms * 10.0).round() / 10.0,
            "import_ms": (self.import_ms * 10.0).round() / 10.0,
            "roundtrip_same": self.same, "roundtrip_differ": self.differ.len(),
            "differ_first": self.differ.iter().take(10).collect::<Vec<_>>(),
            "notes": self.notes,
        })
    }
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}
fn unhex(s: &str) -> Vec<u8> {
    (0..s.len() / 2).map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap()).collect()
}

fn write_json(path: &Path, v: &Value, st: &mut Stat) {
    let mut text = serde_json::to_vec_pretty(v).unwrap();
    text.push(b'\n');
    std::fs::write(path, &text).unwrap();
    st.json_files += 1;
    st.json_bytes += text.len() as u64;
}
fn read_json(path: &Path) -> Value {
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}
fn write_blob(path: &Path, b: &[u8], st: &mut Stat) -> String {
    std::fs::write(path, b).unwrap();
    st.blob_files += 1;
    st.blob_bytes += b.len() as u64;
    hex(&Sha1::digest(b)[..8])
}

fn members(p: &Project, kind: AssetKind) -> Vec<(u16, Vec<u8>)> {
    let dir = p.unpacked_dir(kind);
    let mut out = Vec::new();
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .map(|r| r.filter_map(|e| e.ok()).map(|e| e.file_name().to_string_lossy().into_owned()).collect())
        .unwrap_or_default();
    names.sort();
    for n in names {
        if n.len() == 4 && n.chars().all(|c| c.is_ascii_digit()) {
            let i: u16 = n.parse().unwrap();
            out.push((i, std::fs::read(dir.join(&n)).unwrap()));
        }
    }
    out
}

/// A fixed record as JSON: every field (containers with bit fields as numbers too, bytes as hex).
fn record_json<R: RawRecord>(r: &R, st: &mut Stat) -> Value {
    let mut m = serde_json::Map::new();
    let mut unknown_nonzero = false;
    for f in R::FIELDS {
        let raw = &r.raw()[f.offset..f.offset + f.len];
        let v = match f.kind {
            FieldKind::U8 => json!(r.get_u8(f.offset)),
            FieldKind::I8 => json!(r.get_i8(f.offset)),
            FieldKind::U16 => json!(r.get_u16(f.offset)),
            FieldKind::U32 => json!(r.get_u32(f.offset)),
            FieldKind::Bits { mask } => json!(r.bits(f.offset, mask)),
            FieldKind::Bytes => json!(hex(raw)),
        };
        let lname = f.name.to_lowercase();
        if (lname.contains("pad") || lname.contains("unused") || lname.contains("unknown") || lname.contains("unk"))
            && !matches!(f.kind, FieldKind::Bits { .. })
            && raw.iter().any(|&b| b != 0)
        {
            unknown_nonzero = true;
        }
        m.insert(f.name.to_owned(), v);
    }
    if unknown_nonzero {
        st.note("unknown_bytes_nonzero");
    }
    Value::Object(m)
}

/// Rebuilds the raw bytes from the non-bit fields of the JSON.
fn record_from_json<R: RawRecord>(v: &Value) -> Vec<u8> {
    let mut raw = vec![0u8; R::LEN];
    for f in R::FIELDS {
        let x = &v[f.name];
        let slot = &mut raw[f.offset..f.offset + f.len];
        match f.kind {
            FieldKind::U8 => slot[0] = x.as_u64().unwrap() as u8,
            FieldKind::I8 => slot[0] = x.as_i64().unwrap() as i8 as u8,
            FieldKind::U16 => slot.copy_from_slice(&(x.as_u64().unwrap() as u16).to_le_bytes()),
            FieldKind::U32 => slot.copy_from_slice(&(x.as_u64().unwrap() as u32).to_le_bytes()),
            FieldKind::Bits { .. } => {}
            FieldKind::Bytes => slot.copy_from_slice(&unhex(x.as_str().unwrap())),
        }
    }
    raw
}

fn fixed_kind<R: RawRecord>(p: &Project, kind: AssetKind, out: &Path, name: &str) -> Stat {
    let mut st = Stat::default();
    let dir = out.join(name);
    std::fs::create_dir_all(&dir).unwrap();
    let ms = members(p, kind);
    st.members = ms.len();
    let t = Instant::now();
    for (i, b) in &ms {
        match R::read(b) {
            Ok(r) => {
                let v = record_json(&r, &mut st);
                write_json(&dir.join(format!("{i:04}.json")), &v, &mut st);
            }
            Err(_) => {
                st.note("bad_size_blob");
                write_blob(&dir.join(format!("{i:04}.bin")), b, &mut st);
            }
        }
    }
    st.export_ms = t.elapsed().as_secs_f64() * 1e3;
    let t = Instant::now();
    for (i, b) in &ms {
        let jp = dir.join(format!("{i:04}.json"));
        let again = if jp.exists() { record_from_json::<R>(&read_json(&jp)) } else { std::fs::read(dir.join(format!("{i:04}.bin"))).unwrap() };
        if &again == b { st.same += 1 } else { st.differ.push(*i) }
    }
    st.import_ms = t.elapsed().as_secs_f64() * 1e3;
    st
}

fn maps(p: &Project, out: &Path) -> Stat {
    let fam = p.family;
    let mut st = Stat::default();
    let dir = out.join("maps");
    std::fs::create_dir_all(&dir).unwrap();
    let ms = members(p, AssetKind::Map);
    st.members = ms.len();
    let t = Instant::now();
    for (i, b) in &ms {
        let m = match MapFile::read(b, fam) {
            Ok(m) => m,
            Err(_) => {
                st.note("unreadable_blob");
                write_blob(&dir.join(format!("{i:04}.bin")), b, &mut st);
                continue;
            }
        };
        let rows: Vec<String> = (0..32)
            .map(|y| (0..32).map(|x| { let c = m.permissions[y * 32 + x]; format!("{:02x}{:02x}", c.kind, c.collision) }).collect::<Vec<_>>().join(" "))
            .collect();
        let buildings: Vec<Value> = m.buildings.iter().map(|bd| {
            let mut o = json!({"model": bd.model_id, "pos": bd.position, "rot": bd.rotation, "scale": bd.scale});
            if bd.dummy != [0; 8] { o["dummy"] = json!(hex(&bd.dummy)); }
            o
        }).collect();
        if m.buildings.iter().any(|bd| bd.dummy != [0; 8]) { st.note("building_dummy_nonzero"); }
        if !m.buildings_tail.is_empty() { st.note("buildings_tail"); }
        let model_hash = write_blob(&dir.join(format!("{i:04}.nsbmd")), &m.model, &mut st);
        let bdhc_hash = write_blob(&dir.join(format!("{i:04}.bdhc")), &m.bdhc, &mut st);
        let mut v = json!({"permissions": rows, "buildings": buildings,
            "model": {"file": format!("{i:04}.nsbmd"), "sha1_64": model_hash},
            "bdhc": {"file": format!("{i:04}.bdhc"), "sha1_64": bdhc_hash}});
        if let Some(bgs) = &m.bgs {
            if bgs.len() > 64 {
                let h = write_blob(&dir.join(format!("{i:04}.bgs")), bgs, &mut st);
                v["bgs"] = json!({"file": format!("{i:04}.bgs"), "sha1_64": h});
                st.note("bgs_blob");
            } else {
                v["bgs_hex"] = json!(hex(bgs));
            }
        }
        if !m.buildings_tail.is_empty() { v["buildings_tail"] = json!(hex(&m.buildings_tail)); }
        write_json(&dir.join(format!("{i:04}.json")), &v, &mut st);
    }
    st.export_ms = t.elapsed().as_secs_f64() * 1e3;
    let t = Instant::now();
    for (i, b) in &ms {
        let jp = dir.join(format!("{i:04}.json"));
        if !jp.exists() { st.same += 1; continue; }
        let v = read_json(&jp);
        let mut permissions = Vec::with_capacity(1024);
        for row in v["permissions"].as_array().unwrap() {
            for tok in row.as_str().unwrap().split(' ') {
                let x = unhex(tok);
                permissions.push(Cell { kind: x[0], collision: x[1] });
            }
        }
        let buildings = v["buildings"].as_array().unwrap().iter().map(|o| {
            let arr = |k: &str| { let a = o[k].as_array().unwrap(); [a[0].as_i64().unwrap() as i32, a[1].as_i64().unwrap() as i32, a[2].as_i64().unwrap() as i32] };
            let mut dummy = [0u8; 8];
            if let Some(d) = o.get("dummy") { dummy.copy_from_slice(&unhex(d.as_str().unwrap())); }
            Building { model_id: o["model"].as_u64().unwrap() as u32, position: arr("pos"), rotation: arr("rot"), scale: arr("scale"), dummy }
        }).collect();
        let bgs = if let Some(h) = v.get("bgs_hex") { Some(unhex(h.as_str().unwrap())) } else if v.get("bgs").is_some() { Some(std::fs::read(dir.join(format!("{i:04}.bgs"))).unwrap()) } else { None };
        let m = MapFile {
            bgs,
            permissions,
            buildings,
            buildings_tail: v.get("buildings_tail").map(|h| unhex(h.as_str().unwrap())).unwrap_or_default(),
            model: std::fs::read(dir.join(format!("{i:04}.nsbmd"))).unwrap(),
            bdhc: std::fs::read(dir.join(format!("{i:04}.bdhc"))).unwrap(),
        };
        if &m.write(fam) == b { st.same += 1 } else { st.differ.push(*i) }
    }
    st.import_ms = t.elapsed().as_secs_f64() * 1e3;
    st
}

fn matrices(p: &Project, out: &Path) -> Stat {
    let mut st = Stat::default();
    let dir = out.join("matrices");
    std::fs::create_dir_all(&dir).unwrap();
    let ms = members(p, AssetKind::Matrix);
    st.members = ms.len();
    let t = Instant::now();
    for (i, b) in &ms {
        let m = Matrix::read(b).unwrap();
        let w = m.width as usize;
        let grid = |vals: Vec<String>| -> Vec<String> { vals.chunks(w.max(1)).map(|c| c.join(" ")).collect() };
        let mut v = json!({"name": m.name, "width": m.width, "height": m.height,
            "maps": grid(m.maps.iter().map(|x| if *x == 0xFFFF { "----".into() } else { format!("{x:04}") }).collect())});
        if let Some(h) = &m.headers { v["headers"] = json!(grid(h.iter().map(|x| format!("{x:04}")).collect())); }
        if let Some(a) = &m.altitudes { v["altitudes"] = json!(grid(a.iter().map(|x| format!("{x:02}")).collect())); }
        write_json(&dir.join(format!("{i:04}.json")), &v, &mut st);
    }
    st.export_ms = t.elapsed().as_secs_f64() * 1e3;
    let t = Instant::now();
    for (i, b) in &ms {
        let v = read_json(&dir.join(format!("{i:04}.json")));
        let cells = |k: &str| -> Vec<String> { v[k].as_array().unwrap().iter().flat_map(|r| r.as_str().unwrap().split(' ').map(str::to_owned).collect::<Vec<_>>()).collect() };
        let m = Matrix {
            name: v["name"].as_str().unwrap().to_owned(),
            width: v["width"].as_u64().unwrap() as u8,
            height: v["height"].as_u64().unwrap() as u8,
            headers: v.get("headers").map(|_| cells("headers").iter().map(|s| s.parse().unwrap()).collect()),
            altitudes: v.get("altitudes").map(|_| cells("altitudes").iter().map(|s| s.parse().unwrap()).collect()),
            maps: cells("maps").iter().map(|s| if s == "----" { 0xFFFF } else { s.parse().unwrap() }).collect(),
        };
        if &m.write() == b { st.same += 1 } else { st.differ.push(*i) }
    }
    st.import_ms = t.elapsed().as_secs_f64() * 1e3;
    st
}

fn area_data(p: &Project, out: &Path) -> Stat {
    let fam = p.family;
    let mut st = Stat::default();
    let dir = out.join("areaData");
    std::fs::create_dir_all(&dir).unwrap();
    let ms = members(p, AssetKind::AreaData);
    st.members = ms.len();
    let t = Instant::now();
    for (i, b) in &ms {
        let v = match AreaData::read(b, fam).unwrap() {
            AreaData::Dppt { buildings_tileset, map_tileset, unknown, light_type } => json!({"buildings_tileset": buildings_tileset, "map_tileset": map_tileset, "unknown": unknown, "light_type": light_type}),
            AreaData::Hgss { buildings_tileset, map_tileset, dynamic_texture_type, area_type, light_type } => json!({"buildings_tileset": buildings_tileset, "map_tileset": map_tileset, "dynamic_texture_type": dynamic_texture_type, "area_type": area_type, "light_type": light_type}),
        };
        write_json(&dir.join(format!("{i:04}.json")), &v, &mut st);
    }
    st.export_ms = t.elapsed().as_secs_f64() * 1e3;
    let t = Instant::now();
    for (i, b) in &ms {
        let v = read_json(&dir.join(format!("{i:04}.json")));
        let g = |k: &str| v[k].as_u64().unwrap();
        let a = if fam == Family::Hgss {
            AreaData::Hgss { buildings_tileset: g("buildings_tileset") as u16, map_tileset: g("map_tileset") as u16, dynamic_texture_type: g("dynamic_texture_type") as u16, area_type: g("area_type") as u8, light_type: g("light_type") as u8 }
        } else {
            AreaData::Dppt { buildings_tileset: g("buildings_tileset") as u16, map_tileset: g("map_tileset") as u16, unknown: g("unknown") as u16, light_type: g("light_type") as u16 }
        };
        if &a.write() == b { st.same += 1 } else { st.differ.push(*i) }
    }
    st.import_ms = t.elapsed().as_secs_f64() * 1e3;
    st
}

fn events(p: &Project, out: &Path) -> Stat {
    let ufam = match p.family { Family::Hgss => uxie::GameFamily::HGSS, Family::Pt => uxie::GameFamily::Platinum, Family::Dp => uxie::GameFamily::DP };
    let mut st = Stat::default();
    let dir = out.join("events");
    std::fs::create_dir_all(&dir).unwrap();
    let ms = members(p, AssetKind::Event);
    st.members = ms.len();
    let t = Instant::now();
    for (i, b) in &ms {
        let e = EventFile::from_binary(&mut Cursor::new(b)).unwrap();
        let v = json!({
            "bgs": e.bg_events.iter().map(|x| json!({"script": x.script, "type": x.event_type, "x": x.x, "z": x.z, "y": x.y, "facing": x.player_facing_dir})).collect::<Vec<_>>(),
            "objects": e.object_events.iter().map(|o| json!({"id": o.local_id, "graphics": o.graphics_id, "movement": o.movement_type, "trainer_type": o.trainer_type, "hidden_flag": o.hidden_flag, "script": o.script, "dir": o.dir, "data": o.data, "range_x": o.movement_range_x, "range_z": o.movement_range_z, "x": o.x, "z": o.z, "y": o.y})).collect::<Vec<_>>(),
            "warps": e.warp_events.iter().map(|w| json!({"x": w.x, "z": w.z, "dest_header": w.dest_header_id, "dest_warp": w.dest_warp_id, "height": w.height})).collect::<Vec<_>>(),
            "coords": e.coord_events.iter().map(|c| json!({"script": c.script, "x": c.x, "z": c.z, "width": c.width, "length": c.length, "y": c.y, "value": c.value, "var": c.var})).collect::<Vec<_>>(),
        });
        write_json(&dir.join(format!("{i:04}.json")), &v, &mut st);
    }
    st.export_ms = t.elapsed().as_secs_f64() * 1e3;
    let t = Instant::now();
    for (i, b) in &ms {
        let v = read_json(&dir.join(format!("{i:04}.json")));
        let u = |x: &Value| x.as_u64().unwrap();
        let s = |x: &Value| x.as_i64().unwrap();
        let e = EventFile {
            bg_events: v["bgs"].as_array().unwrap().iter().map(|x| BgEvent { script: u(&x["script"]) as u16, event_type: u(&x["type"]) as u16, x: s(&x["x"]) as i32, z: s(&x["z"]) as i32, y: s(&x["y"]) as i32, player_facing_dir: u(&x["facing"]) as u16 }).collect(),
            object_events: v["objects"].as_array().unwrap().iter().map(|o| ObjectEvent { local_id: u(&o["id"]) as u16, graphics_id: u(&o["graphics"]) as u16, movement_type: u(&o["movement"]) as u16, trainer_type: u(&o["trainer_type"]) as u16, hidden_flag: u(&o["hidden_flag"]) as u16, script: u(&o["script"]) as u16, dir: s(&o["dir"]) as i16, data: { let d = o["data"].as_array().unwrap(); [u(&d[0]) as u16, u(&d[1]) as u16, u(&d[2]) as u16] }, movement_range_x: s(&o["range_x"]) as i16, movement_range_z: s(&o["range_z"]) as i16, x: u(&o["x"]) as u16, z: u(&o["z"]) as u16, y: s(&o["y"]) as i32 }).collect(),
            warp_events: v["warps"].as_array().unwrap().iter().map(|w| WarpEvent { x: u(&w["x"]) as u16, z: u(&w["z"]) as u16, dest_header_id: u(&w["dest_header"]) as u16, dest_warp_id: u(&w["dest_warp"]) as u16, height: u(&w["height"]) as u32 }).collect(),
            coord_events: v["coords"].as_array().unwrap().iter().map(|c| CoordEvent { script: u(&c["script"]) as u16, x: s(&c["x"]) as i16, z: s(&c["z"]) as i16, width: u(&c["width"]) as u16, length: u(&c["length"]) as u16, y: u(&c["y"]) as u16, value: u(&c["value"]) as u16, var: u(&c["var"]) as u16 }).collect(),
        };
        let mut w = Cursor::new(Vec::new());
        let ok = e.to_binary_for(&mut w, ufam).is_ok() && &w.into_inner() == b;
        if ok { st.same += 1 } else { st.differ.push(*i) }
    }
    st.import_ms = t.elapsed().as_secs_f64() * 1e3;
    st
}

fn headers(p: &Project, out: &Path) -> Stat {
    let mut st = Stat::default();
    let dir = out.join("headers");
    std::fs::create_dir_all(&dir).unwrap();
    let t = Instant::now();
    let hs = p.read_headers().unwrap();
    let names = p.internal_names().unwrap();
    st.members = hs.len();
    let original: Vec<u8> = hs.iter().flat_map(uxie::map_header::write_map_header_to_bytes).collect();
    for (i, h) in hs.iter().enumerate() {
        let mut v = serde_json::to_value(h).unwrap();
        v["internal_name"] = json!(names.get(i).cloned().unwrap_or_default());
        write_json(&dir.join(format!("{i:04}.json")), &v, &mut st);
    }
    st.export_ms = t.elapsed().as_secs_f64() * 1e3;
    let t = Instant::now();
    let mut again = Vec::new();
    for i in 0..hs.len() {
        let mut v = read_json(&dir.join(format!("{i:04}.json")));
        v.as_object_mut().unwrap().remove("internal_name");
        let h: uxie::MapHeader = serde_json::from_value(v).unwrap();
        again.extend(uxie::map_header::write_map_header_to_bytes(&h));
    }
    // Compare against the bytes on disk (arm9 slice or dynamic header files).
    let disk = match p.asset_location(AssetKind::HeaderTable, 0).unwrap() {
        g4_kernel::project::Location::Slice { path, offset, len } => std::fs::read(path).unwrap()[offset as usize..offset as usize + len].to_vec(),
        g4_kernel::project::Location::HeaderFiles { dir, count } => (0..count).flat_map(|i| std::fs::read(dir.join(format!("{i:04}"))).unwrap()).collect(),
        _ => original.clone(),
    };
    if again == disk { st.same = hs.len() } else { st.differ.push(0) }
    st.import_ms = t.elapsed().as_secs_f64() * 1e3;
    st
}

fn learnsets(p: &Project, out: &Path) -> Stat {
    let mut st = Stat::default();
    let dir = out.join("learnsets");
    std::fs::create_dir_all(&dir).unwrap();
    let ms = members(p, AssetKind::Learnset);
    st.members = ms.len();
    let t = Instant::now();
    for (i, b) in &ms {
        let l = Learnset::read(b).unwrap();
        let mut v = json!({"moves": l.entries.iter().map(|e| json!({"level": e.level, "move": e.move_id})).collect::<Vec<_>>()});
        if !l.terminated { v["unterminated"] = json!(true); st.note("unterminated"); }
        if !l.tail.is_empty() && l.tail.iter().any(|&x| x != 0) { st.note("tail_nonzero"); }
        v["tail"] = json!(hex(&l.tail));
        write_json(&dir.join(format!("{i:04}.json")), &v, &mut st);
    }
    st.export_ms = t.elapsed().as_secs_f64() * 1e3;
    let t = Instant::now();
    for (i, b) in &ms {
        let v = read_json(&dir.join(format!("{i:04}.json")));
        let entries: Vec<LearnEntry> = v["moves"].as_array().unwrap().iter().map(|e| LearnEntry { move_id: e["move"].as_u64().unwrap() as u16, level: e["level"].as_u64().unwrap() as u8 }).collect();
        let n = entries.len();
        let l = Learnset { entries, tail: unhex(v["tail"].as_str().unwrap()), terminated: v.get("unterminated").is_none(), read_count: n };
        let w = l.write();
        let w = if l.terminated { w } else { w[..w.len() - 2].to_vec() };
        if &w == b { st.same += 1 } else { st.differ.push(*i) }
    }
    st.import_ms = t.elapsed().as_secs_f64() * 1e3;
    st
}

fn trainers(p: &Project, out: &Path) -> Stat {
    let fam = p.family;
    let mut st = Stat::default();
    let dir = out.join("trainers");
    std::fs::create_dir_all(&dir).unwrap();
    let props = members(p, AssetKind::TrainerProperties);
    let party: BTreeMap<u16, Vec<u8>> = members(p, AssetKind::TrainerParty).into_iter().collect();
    st.members = props.len();
    let t = Instant::now();
    let mut docs = BTreeMap::new();
    for (i, pr) in &props {
        let pa = party.get(i).cloned().unwrap_or_default();
        match TrainerDoc::decode(pr, &pa, fam) {
            Ok(d) => {
                let tp = TrainerProps::read(pr).unwrap();
                let mut v = record_json(&tp, &mut st);
                v["party"] = json!(d.party.iter().map(|m| {
                    let mut o = json!({"species": m.species_word & 0x3ff, "form": m.species_word >> 10, "level": m.level, "difficulty": m.iv_or_difficulty, "byte1": m.byte1});
                    if let Some(it) = m.item { o["item"] = json!(it); }
                    if let Some(mv) = m.moves { o["moves"] = json!(mv); }
                    if let Some(bc) = m.ball_capsule { o["ball_capsule"] = json!(bc); }
                    o
                }).collect::<Vec<_>>());
                write_json(&dir.join(format!("{i:04}.json")), &v, &mut st);
                docs.insert(*i, d);
            }
            Err(_) => {
                st.note("undecodable_pair");
            }
        }
    }
    st.export_ms = t.elapsed().as_secs_f64() * 1e3;
    // Import: parse JSON again (timed), re-encode the decoded doc (the codec is the kernel's).
    let t = Instant::now();
    for (i, pr) in &props {
        let _v = read_json(&dir.join(format!("{i:04}.json")));
        let Some(d) = docs.get(i) else { st.differ.push(*i); continue };
        let (a, b) = d.encode(fam);
        let pa = party.get(i).cloned().unwrap_or_default();
        if &a == pr && b == pa { st.same += 1 } else { st.differ.push(*i) }
    }
    st.import_ms = t.elapsed().as_secs_f64() * 1e3;
    st
}

fn trainer_text(p: &Project, out: &Path) -> Stat {
    let mut st = Stat::default();
    let dir = out.join("trainerText");
    std::fs::create_dir_all(&dir).unwrap();
    let t = Instant::now();
    let tt = members(p, AssetKind::TrainerTextTable);
    let to = members(p, AssetKind::TrainerTextOffsets);
    st.members = tt.len() + to.len();
    let table = TrainerTextTable::read(&tt[0].1).unwrap();
    let offs = TrainerTextOffsets::read(&to[0].1).unwrap();
    write_json(&dir.join("table.json"), &json!(table.entries.iter().map(|(a, b)| json!([a, b])).collect::<Vec<_>>()), &mut st);
    write_json(&dir.join("offsets.json"), &json!(offs.offsets), &mut st);
    st.export_ms = t.elapsed().as_secs_f64() * 1e3;
    let t = Instant::now();
    if table.write() == tt[0].1 { st.same += 1 } else { st.differ.push(0) }
    if offs.write() == to[0].1 { st.same += 1 } else { st.differ.push(1) }
    st.import_ms = t.elapsed().as_secs_f64() * 1e3;
    st
}

fn texts(p: &Project, out: &Path) -> Stat {
    let lang = p.language.locale_key();
    let mut st = Stat::default();
    let dir = out.join("texts");
    let tmp = out.join("_tmp");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::create_dir_all(&tmp).unwrap();
    let ms = members(p, AssetKind::TextArchive);
    st.members = ms.len();
    let t = Instant::now();
    let mut msgs = 0usize;
    for (i, b) in &ms {
        let d = TextArchiveDoc::from_member(b, lang).unwrap();
        msgs += d.messages.len();
        let mut text = d.to_json();
        text.push(b'\n');
        std::fs::write(dir.join(format!("{i:04}.json")), &text).unwrap();
        st.json_files += 1;
        st.json_bytes += text.len() as u64;
    }
    st.notes.insert("messages".into(), msgs);
    st.export_ms = t.elapsed().as_secs_f64() * 1e3;
    let t = Instant::now();
    for (i, b) in &ms {
        let d = TextArchiveDoc::from_json(&std::fs::read(dir.join(format!("{i:04}.json"))).unwrap(), lang).unwrap();
        let jp = tmp.join(format!("{i:04}.json"));
        let mp = tmp.join(format!("{i:04}"));
        let _ = g4_kernel::text::write_archive(out, &jp, &mp, &d).unwrap();
        if &std::fs::read(&mp).unwrap() == b { st.same += 1 } else { st.differ.push(*i) }
    }
    st.import_ms = t.elapsed().as_secs_f64() * 1e3;
    let _ = std::fs::remove_dir_all(&tmp);
    st
}

fn dir_size(d: &Path, files: &mut usize, bytes: &mut u64, skip_hidden: bool) {
    if let Ok(rd) = std::fs::read_dir(d) {
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            if skip_hidden && name.starts_with('.') { continue; }
            let p = e.path();
            if p.is_dir() { dir_size(&p, files, bytes, skip_hidden) } else { *files += 1; *bytes += e.metadata().unwrap().len(); }
        }
    }
}

fn scripts_kind(p: &Project) -> Stat {
    let mut st = Stat::default();
    st.members = p.member_count(AssetKind::Script).unwrap();
    let t = Instant::now();
    if !g4_kernel::scripts::is_adopted(p) {
        let r = g4_kernel::scripts::adopt(p, &mut |_| {}).unwrap();
        st.notes.insert("decompiled".into(), r.decompiled as usize);
        st.notes.insert("skipped_empty".into(), r.skipped_empty as usize);
        st.notes.insert("failed".into(), r.failed.len());
    }
    st.export_ms = t.elapsed().as_secs_f64() * 1e3;
    let src = p.root.join("expanded/scripts");
    let (mut f, mut b) = (0, 0);
    dir_size(&src, &mut f, &mut b, true);
    st.json_files = f;
    st.json_bytes = b;
    let (mut f2, mut b2) = (0, 0);
    dir_size(&src.join(".rotom"), &mut f2, &mut b2, false);
    st.blob_files = f2; // .rotom metadata (cache, status, command database)
    st.blob_bytes = b2;
    // Import: delete every member with a source, compile, compare.
    let files = g4_kernel::scripts::list_files(p).unwrap();
    let mut original = Vec::new();
    for info in files.iter().filter(|i| i.kind != g4_kernel::scripts::SourceKind::Binary) {
        let path = p.member_path(AssetKind::Script, info.file);
        original.push((info.file, std::fs::read(&path).unwrap()));
        std::fs::remove_file(&path).unwrap();
    }
    let t = Instant::now();
    let rep = g4_kernel::scripts::compile(p).unwrap();
    st.import_ms = t.elapsed().as_secs_f64() * 1e3;
    st.notes.insert("compiled".into(), rep.compiled.len());
    st.notes.insert("problems".into(), rep.problems.len());
    for (file, bytes) in &original {
        if std::fs::read(p.member_path(AssetKind::Script, *file)).ok().as_ref() == Some(bytes) { st.same += 1 } else { st.differ.push(*file) }
    }
    st
}

fn blobs(p: &Project, kind: AssetKind, out: &Path, name: &str) -> Stat {
    let mut st = Stat::default();
    if p.archives.packed(kind).is_none() { return st; }
    let dir = out.join(name);
    std::fs::create_dir_all(&dir).unwrap();
    let ms = members(p, kind);
    st.members = ms.len();
    let t = Instant::now();
    for (i, b) in &ms {
        write_blob(&dir.join(format!("{i:04}.bin")), b, &mut st);
    }
    st.export_ms = t.elapsed().as_secs_f64() * 1e3;
    st.same = ms.len();
    st
}

fn names(p: &Project, out: &Path) {
    let lang = p.language.locale_key();
    let read_bank = |bank: u16| -> Vec<String> {
        let b = std::fs::read(p.member_path(AssetKind::TextArchive, bank)).unwrap();
        TextArchiveDoc::from_member(&b, lang).unwrap().messages.into_iter().map(|m| m.text).collect()
    };
    let mut v = serde_json::Map::new();
    for (k, bank) in [("species", Bank::Species), ("moves", Bank::Moves), ("items", Bank::Items), ("abilities", Bank::Abilities), ("types", Bank::Types), ("trainer_names", Bank::TrainerNames), ("trainer_classes", Bank::TrainerClasses)] {
        if let Some(i) = bank_index(p.family, p.language, bank) { v.insert(k.into(), json!(read_bank(i))); }
    }
    v.insert("locations".into(), json!(read_bank(g4_kernel::places::location_bank(p))));
    v.insert("internal_names".into(), json!(p.internal_names().unwrap()));
    let hs = p.read_headers().unwrap();
    v.insert("header_location".into(), json!(hs.iter().map(|h| serde_json::to_value(h).unwrap()).map(|h| { let o = h.as_object().unwrap().values().next().unwrap().clone(); o["location_name"].clone() }).collect::<Vec<_>>()));
    // trainer classes per trainer
    let props = members(p, AssetKind::TrainerProperties);
    v.insert("trainer_class_of".into(), json!(props.iter().map(|(_, b)| b[1]).collect::<Vec<_>>()));
    std::fs::write(out.join("names.json"), serde_json::to_vec_pretty(&Value::Object(v)).unwrap()).unwrap();
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let root = PathBuf::from(&args[1]);
    let out = PathBuf::from(&args[2]);
    let only: Option<&str> = args.get(3).map(|s| s.as_str());
    let _ = std::fs::remove_dir_all(&out);
    std::fs::create_dir_all(&out).unwrap();
    let t0 = Instant::now();
    let p = Project::open(&root).unwrap();
    let open_ms = t0.elapsed().as_secs_f64() * 1e3;
    let mut report = serde_json::Map::new();
    report.insert("game".into(), json!(p.game_code));
    report.insert("open_ms".into(), json!(open_ms));
    let run = |name: &str| only.is_none_or(|o| o == name);
    let mut add = |name: &str, st: Stat| { eprintln!("[{:.1}s] {name}: {:?}", t0.elapsed().as_secs_f64(), st.to_json()); report.insert(name.into(), st.to_json()); };
    if run("headers") { add("headers", headers(&p, &out)); }
    if run("matrices") { add("matrices", matrices(&p, &out)); }
    if run("maps") { add("maps", maps(&p, &out)); }
    if run("areaData") { add("areaData", area_data(&p, &out)); }
    if run("events") { add("events", events(&p, &out)); }
    if run("texts") { add("texts", texts(&p, &out)); }
    if run("encounters") {
        let st = if p.family == Family::Hgss { fixed_kind::<HgssEncounters>(&p, AssetKind::Encounter, &out, "encounters") } else { fixed_kind::<PtEncounters>(&p, AssetKind::Encounter, &out, "encounters") };
        add("encounters", st);
    }
    if run("personal") { add("personal", fixed_kind::<Personal>(&p, AssetKind::Personal, &out, "personal")); }
    if run("learnsets") { add("learnsets", learnsets(&p, &out)); }
    if run("evolutions") { add("evolutions", fixed_kind::<Evolution>(&p, AssetKind::Evolution, &out, "evolutions")); }
    if run("moves") { add("moves", fixed_kind::<Move>(&p, AssetKind::Move, &out, "moves")); }
    if run("items") { add("items", fixed_kind::<Item>(&p, AssetKind::Item, &out, "items")); }
    if run("trainers") { add("trainers", trainers(&p, &out)); }
    if run("trainerText") { add("trainerText", trainer_text(&p, &out)); }
    if run("blobs") {
        add("mapTextures", blobs(&p, AssetKind::MapTexture, &out, "blobs/mapTextures"));
        add("buildingTextures", blobs(&p, AssetKind::BuildingTexture, &out, "blobs/buildingTextures"));
        add("buildingModels", blobs(&p, AssetKind::BuildingModel, &out, "blobs/buildingModels"));
        add("interiorBuildingModels", blobs(&p, AssetKind::InteriorBuildingModel, &out, "blobs/interiorBuildingModels"));
    }
    if run("names") { names(&p, &out); }
    if run("scripts") { add("scripts", scripts_kind(&p)); }
    report.insert("total_ms".into(), json!(t0.elapsed().as_secs_f64() * 1e3));
    std::fs::write(out.join("report.json"), serde_json::to_vec_pretty(&Value::Object(report)).unwrap()).unwrap();
}
