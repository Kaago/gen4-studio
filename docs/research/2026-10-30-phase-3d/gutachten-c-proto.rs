//! Prototyp für Gutachten C (3d): Kampfsprites (NCGR Modus 1, verschlüsselt) entschlüsseln und
//! mit `g4_formats::ncgr::Ncgr::read` und `g4_formats::nclr::Nclr::read` lesen. Gibt nur Zahlen
//! aus (Größe, Tiefe, Palettenindizes, Lage im Bild), keine Pixel, keine Bilder.
//!
//! Entschlüsselung (gemessen mit gutachten-c-sprites-krypto.py): 16-Bit-Wörter, Schlüssel
//! k' = k * 0x41C64E6D + 0x6073, XOR mit k & 0xFFFF; Startwert das erste Wort der Richtung.
//! Pokémon (Pt, HGSS) und Trainer Pt: vorwärts ab dem ersten Wort; Trainer HGSS: rückwärts ab
//! dem letzten.

use std::path::Path;
use std::time::Instant;

use g4_formats::ncgr::{Ncgr, render_rgba};
use g4_formats::nclr::Nclr;

const MUL: u32 = 0x41C6_4E6D;
const ADD: u32 = 0x6073;

fn decrypt(tiles: &mut [u8], backward: bool) {
    let n = tiles.len() / 2;
    let word = |t: &[u8], i: usize| u16::from_le_bytes([t[2 * i], t[2 * i + 1]]);
    let order: Vec<usize> = if backward { (0..n).rev().collect() } else { (0..n).collect() };
    let mut key = u32::from(word(tiles, order[0]));
    for i in order {
        let w = word(tiles, i) ^ (key as u16);
        tiles[2 * i..2 * i + 2].copy_from_slice(&w.to_le_bytes());
        key = key.wrapping_mul(MUL).wrapping_add(ADD);
    }
}

/// Palettenindex des Pixels (x, y) bei linearer Ablage (Modus 1, „scanned“), 4 bpp.
fn scanned(nc: &Ncgr, x: usize, y: usize) -> u8 {
    let n = y * nc.width + x;
    (nc.tiles[n / 2] >> (4 * (n % 2))) & 0x0F
}

/// Palettenindex bei gekachelter Ablage, wie `render_rgba` sie liest.
fn tiled(nc: &Ncgr, x: usize, y: usize) -> u8 {
    let across = nc.width / 8;
    let tile = (y / 8) * across + x / 8;
    let at = (y % 8) * 8 + x % 8;
    (nc.tiles[tile * 32 + at / 2] >> (4 * (at % 2))) & 0x0F
}

/// Anteil gleicher waagrechter Nachbarn im linken 80x80-Bild (Kohärenz der Ablage).
fn coherence(nc: &Ncgr, f: fn(&Ncgr, usize, usize) -> u8) -> f64 {
    let (mut same, mut all) = (0u32, 0u32);
    for y in 0..80 {
        for x in 0..79 {
            same += u32::from(f(nc, x, y) == f(nc, x + 1, y));
            all += 1;
        }
    }
    f64::from(same) / f64::from(all)
}

/// Zusammenhängende Flächen deckender Pixel (4er-Nachbarschaft) im linken 80x80-Bild: ein
/// richtig gelesenes Sprite hat wenige, eine falsche Ablage zerreißt es in viele.
fn components(nc: &Ncgr, f: fn(&Ncgr, usize, usize) -> u8) -> usize {
    let mut seen = [[false; 80]; 80];
    let mut count = 0;
    for sy in 0..80 {
        for sx in 0..80 {
            if seen[sy][sx] || f(nc, sx, sy) == 0 {
                continue;
            }
            count += 1;
            let mut stack = vec![(sx, sy)];
            seen[sy][sx] = true;
            while let Some((x, y)) = stack.pop() {
                let mut push = |x: usize, y: usize| {
                    if !seen[y][x] && f(nc, x, y) != 0 {
                        seen[y][x] = true;
                        stack.push((x, y));
                    }
                };
                if x > 0 { push(x - 1, y) }
                if x < 79 { push(x + 1, y) }
                if y > 0 { push(x, y - 1) }
                if y < 79 { push(x, y + 1) }
            }
        }
    }
    count
}

/// Rahmen der nicht durchsichtigen Pixel im linken 80x80-Bild: (x0, y0, x1, y1).
fn bbox(nc: &Ncgr) -> Option<(usize, usize, usize, usize)> {
    let mut b: Option<(usize, usize, usize, usize)> = None;
    for y in 0..80 {
        for x in 0..80 {
            if scanned(nc, x, y) != 0 {
                b = Some(match b {
                    None => (x, y, x, y),
                    Some((a, c, d, e)) => (a.min(x), c.min(y), d.max(x), e.max(y)),
                });
            }
        }
    }
    b
}

fn members(dir: &Path) -> Vec<Vec<u8>> {
    let mut names: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| !p.file_name().unwrap().to_string_lossy().starts_with('.'))
        .collect();
    names.sort();
    names.iter().map(|p| std::fs::read(p).unwrap()).collect()
}

struct Tally {
    read_ok: usize,
    sizes: std::collections::BTreeMap<(usize, usize, u8), usize>,
    coh_scanned: f64,
    coh_tiled: f64,
    colours_min: usize,
    colours_max: usize,
    colours_sum: usize,
    bottom_touch: usize,
    empty_frame: usize,
    comp_scanned: usize,
    comp_tiled: usize,
    n: usize,
}

impl Tally {
    fn new() -> Tally {
        Tally {
            read_ok: 0,
            sizes: Default::default(),
            coh_scanned: 0.0,
            coh_tiled: 0.0,
            colours_min: 99,
            colours_max: 0,
            colours_sum: 0,
            bottom_touch: 0,
            empty_frame: 0,
            comp_scanned: 0,
            comp_tiled: 0,
            n: 0,
        }
    }
    fn add(&mut self, bytes: &[u8], backward: bool) {
        let Ok(mut nc) = Ncgr::read(bytes) else { return };
        self.read_ok += 1;
        *self.sizes.entry((nc.width, nc.height, nc.bpp)).or_default() += 1;
        decrypt(&mut nc.tiles, backward);
        self.coh_scanned += coherence(&nc, scanned);
        self.coh_tiled += coherence(&nc, tiled);
        self.comp_scanned += components(&nc, scanned);
        self.comp_tiled += components(&nc, tiled);
        let mut used = [false; 16];
        for b in &nc.tiles {
            used[usize::from(b & 15)] = true;
            used[usize::from(b >> 4)] = true;
        }
        let c = used.iter().filter(|u| **u).count();
        self.colours_min = self.colours_min.min(c);
        self.colours_max = self.colours_max.max(c);
        self.colours_sum += c;
        match bbox(&nc) {
            Some((_, _, _, y1)) if y1 == 79 => self.bottom_touch += 1,
            Some(_) => {}
            None => self.empty_frame += 1,
        }
        self.n += 1;
    }
    fn print(&self, label: &str) {
        let n = self.n.max(1) as f64;
        println!(
            "{label:34} gelesen {:5}  Maße {:?}  Kohärenz linear {:.2} / gekachelt {:.2}  \
             Flächen linear {:.1} / gekachelt {:.1}  Farben je Bild {}..{} (Mittel {:.1})  \
             unten angeschnitten {:5}  leeres Bild {}",
            self.read_ok,
            self.sizes,
            self.coh_scanned / n,
            self.coh_tiled / n,
            self.comp_scanned as f64 / n,
            self.comp_tiled as f64 / n,
            self.colours_min,
            self.colours_max,
            self.colours_sum as f64 / n,
            self.bottom_touch,
            self.empty_frame
        );
    }
}

fn main() {
    let root = std::path::PathBuf::from(std::env::args().nth(1).expect("DSPRE-Ordner"));
    let hgss = root.join("unpacked/trainerGraphics").read_dir().unwrap().count() > 600;
    let t0 = Instant::now();
    // Pokémon: je Art sechs Member; 0..=3 Bilder, 4 und 5 Paletten.
    let mons = members(&root.join("unpacked/pokemonBattleSprites"));
    let mut slots: Vec<Tally> = (0..4).map(|_| Tally::new()).collect();
    let (mut pal_ok, mut pal_16, mut pal_same) = (0, 0, 0);
    for (i, m) in mons.iter().enumerate() {
        match i % 6 {
            s @ 0..=3 if !m.is_empty() => slots[s].add(m, false),
            4 | 5 => {
                if let Ok(p) = Nclr::read(m) {
                    pal_ok += 1;
                    pal_16 += usize::from(p.palettes.len() == 1);
                }
                if i % 6 == 5 && mons[i - 1] == *m {
                    pal_same += 1;
                }
            }
            _ => {}
        }
    }
    for (s, t) in slots.iter().enumerate() {
        t.print(&format!("Pokémon Platz {s}"));
    }
    println!(
        "Pokémon Paletten: {pal_ok} gelesen, {pal_16} mit genau einer Palette zu 16 Farben, \
         normal = schillernd bei {pal_same} Arten"
    );
    let dt_mon = t0.elapsed();
    // Ein Bild ganz: entschlüsseln, mit render_rgba (gekachelt) und linear zählen.
    let one = &mons[6 + 3];
    let mut nc = Ncgr::read(one).unwrap();
    decrypt(&mut nc.tiles, false);
    let pal = Nclr::read(&mons[6 + 4]).unwrap().palettes[0];
    let t1 = Instant::now();
    let rgba = render_rgba(&nc, &pal);
    let dt_one = t1.elapsed();
    let opaque = rgba.chunks_exact(4).filter(|p| p[3] == 255).count();
    println!(
        "Art 1, Platz 3: {} x {} Pixel, {} bpp, RGBA {} Byte, deckend {} Pixel ({:.0} %), render {:?}",
        nc.width,
        nc.height,
        nc.bpp,
        rgba.len(),
        opaque,
        100.0 * opaque as f64 / (nc.width * nc.height) as f64,
        dt_one
    );
    // Trainer: je Klasse fünf Member; 4 ist das lineare Bild (Modus 1), 1 die Palette.
    let trs = members(&root.join("unpacked/trainerGraphics"));
    let mut tr = Tally::new();
    let mut tr_pal = (0, 0);
    for (i, m) in trs.iter().enumerate() {
        match i % 5 {
            4 => tr.add(m, hgss),
            1 => {
                if let Ok(p) = Nclr::read(m) {
                    tr_pal.0 += 1;
                    tr_pal.1 = p.palettes.len();
                }
            }
            _ => {}
        }
    }
    tr.print(if hgss { "Trainer (rückwärts)" } else { "Trainer (vorwärts)" });
    println!("Trainer Paletten: {} gelesen, je {} Paletten zu 16", tr_pal.0, tr_pal.1);
    // Formen und Eier
    let other = members(&root.join("unpacked/otherPokemonBattleSprites"));
    let mut ot = Tally::new();
    for m in &other {
        if m.starts_with(b"RGCN") {
            ot.add(m, false);
        }
    }
    ot.print("Formen (otherPokemonBattleSprites)");
    println!("Zeit: Pokémon {:?}, gesamt {:?}", dt_mon, t0.elapsed());
}
