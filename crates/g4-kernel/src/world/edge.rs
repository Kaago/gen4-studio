//! The edge sets of a tileset (phase 6.4b): `Tileset.edge_sets`, each a set of tiles for the
//! 15 places of the Wang corners of two colours, **inside** (the painted area) and
//! **outside**. A cell's place is the mask of its four corners ([`NW`], [`NE`], [`SW`], [`SE`],
//! bit set = inside); mask 0 is the set's `outside` tile, masks 1 to 15 are the places
//! ([`Place`]): the 13 places of the gate ([`Place::GATE13`]) and the two diagonals. Every
//! entry of a place is a full tile token with turn and flip (`"0007r1"`), so a tile may fill
//! several places turned; the same token (compared after [`parse_tile`]) stands only once in a
//! set, `outside` included, so a token names one place. A place with several entries picks one
//! per cell by [`variant`] (rule „Zufall“). Sets may be incomplete (empty places left out).
//!
//! `tileset_edit` puts and removes sets ([`EdgeSetOp`]) as one file step
//! ([`super::tileset::tileset_edit`]); [`edge_set_problems`] are the checks of one set, which
//! [`super::tileset::tileset_problems`] runs for every set. The corner and fill logic is
//! written for gen4-studio; the Wang-corner idea follows Tiled's terrain brush (no code).

use serde::{Deserialize, Serialize};

use super::grid::{TileRef, parse_tile, tile_token};
use super::tileset::tile_outside;

/// The corner bits of a place's mask (bit set = the corner is inside).
pub const NW: u8 = 1;
pub const NE: u8 = 2;
pub const SW: u8 = 4;
pub const SE: u8 = 8;

/// The 15 places of an edge set, one per mask ≠ 0 (table „Plätze und Masken“): edges by the
/// side that is outside, outer corners by their position at the area, inner corners by the
/// corner that is outside.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Place {
    Center,
    N,
    E,
    S,
    W,
    OuterNw,
    OuterNe,
    OuterSw,
    OuterSe,
    InnerNw,
    InnerNe,
    InnerSw,
    InnerSe,
    DiagNwse,
    DiagNesw,
}

impl Place {
    /// Every place in the order of the file.
    pub const ALL: [Place; 15] = [
        Place::Center,
        Place::N,
        Place::E,
        Place::S,
        Place::W,
        Place::OuterNw,
        Place::OuterNe,
        Place::OuterSw,
        Place::OuterSe,
        Place::InnerNw,
        Place::InnerNe,
        Place::InnerSw,
        Place::InnerSe,
        Place::DiagNwse,
        Place::DiagNesw,
    ];

    /// The 13 places of the gate: [`Place::ALL`] without the diagonals.
    pub const GATE13: [Place; 13] = [
        Place::Center,
        Place::N,
        Place::E,
        Place::S,
        Place::W,
        Place::OuterNw,
        Place::OuterNe,
        Place::OuterSw,
        Place::OuterSe,
        Place::InnerNw,
        Place::InnerNe,
        Place::InnerSw,
        Place::InnerSe,
    ];

    /// The corner mask of the place (bit set = inside).
    pub fn mask(self) -> u8 {
        match self {
            Place::Center => NW | NE | SW | SE,
            Place::N => SW | SE,
            Place::E => NW | SW,
            Place::S => NW | NE,
            Place::W => NE | SE,
            Place::OuterNw => SE,
            Place::OuterNe => SW,
            Place::OuterSw => NE,
            Place::OuterSe => NW,
            Place::InnerNw => NE | SW | SE,
            Place::InnerNe => NW | SW | SE,
            Place::InnerSw => NW | NE | SE,
            Place::InnerSe => NW | NE | SW,
            Place::DiagNwse => NW | SE,
            Place::DiagNesw => NE | SW,
        }
    }

    /// The place of `mask`; `None` for 0 (the outside tile) and past 15.
    pub fn of_mask(mask: u8) -> Option<Place> {
        Place::ALL.into_iter().find(|p| p.mask() == mask)
    }

    /// The key of the place in the file (`outer_nw`).
    pub fn key(self) -> &'static str {
        match self {
            Place::Center => "center",
            Place::N => "n",
            Place::E => "e",
            Place::S => "s",
            Place::W => "w",
            Place::OuterNw => "outer_nw",
            Place::OuterNe => "outer_ne",
            Place::OuterSw => "outer_sw",
            Place::OuterSe => "outer_se",
            Place::InnerNw => "inner_nw",
            Place::InnerNe => "inner_ne",
            Place::InnerSw => "inner_sw",
            Place::InnerSe => "inner_se",
            Place::DiagNwse => "diag_nwse",
            Place::DiagNesw => "diag_nesw",
        }
    }

    /// The name of the place in the UI, in the current locale („Outer corner NW“).
    pub fn label(self) -> String {
        let key = match self {
            Place::Center => "world.place_center",
            Place::N => "world.place_n",
            Place::E => "world.place_e",
            Place::S => "world.place_s",
            Place::W => "world.place_w",
            Place::OuterNw => "world.place_outer_nw",
            Place::OuterNe => "world.place_outer_ne",
            Place::OuterSw => "world.place_outer_sw",
            Place::OuterSe => "world.place_outer_se",
            Place::InnerNw => "world.place_inner_nw",
            Place::InnerNe => "world.place_inner_ne",
            Place::InnerSw => "world.place_inner_sw",
            Place::InnerSe => "world.place_inner_se",
            Place::DiagNwse => "world.place_diag_nwse",
            Place::DiagNesw => "world.place_diag_nesw",
        };
        t!(key).into_owned()
    }
}

/// Which cell of a cliff edge drawn with the set blocks (rule „Kollision der Klippe, Regel je
/// Satz“): the upper one (default, left out of the file) or the lower one.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeBlock {
    #[default]
    Upper,
    Lower,
}

fn is_upper(block: &EdgeBlock) -> bool {
    *block == EdgeBlock::Upper
}

/// The tile tokens of the 15 places, each field named by [`Place::key`], in file order;
/// empty places are left out.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EdgePlaces {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub center: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub n: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub e: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub s: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub w: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub outer_nw: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub outer_ne: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub outer_sw: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub outer_se: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub inner_nw: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub inner_ne: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub inner_sw: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub inner_se: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diag_nwse: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diag_nesw: Vec<String>,
}

impl EdgePlaces {
    /// The entries of place `p`.
    pub fn get(&self, p: Place) -> &[String] {
        match p {
            Place::Center => &self.center,
            Place::N => &self.n,
            Place::E => &self.e,
            Place::S => &self.s,
            Place::W => &self.w,
            Place::OuterNw => &self.outer_nw,
            Place::OuterNe => &self.outer_ne,
            Place::OuterSw => &self.outer_sw,
            Place::OuterSe => &self.outer_se,
            Place::InnerNw => &self.inner_nw,
            Place::InnerNe => &self.inner_ne,
            Place::InnerSw => &self.inner_sw,
            Place::InnerSe => &self.inner_se,
            Place::DiagNwse => &self.diag_nwse,
            Place::DiagNesw => &self.diag_nesw,
        }
    }

    /// The entries of place `p`, to change.
    pub fn get_mut(&mut self, p: Place) -> &mut Vec<String> {
        match p {
            Place::Center => &mut self.center,
            Place::N => &mut self.n,
            Place::E => &mut self.e,
            Place::S => &mut self.s,
            Place::W => &mut self.w,
            Place::OuterNw => &mut self.outer_nw,
            Place::OuterNe => &mut self.outer_ne,
            Place::OuterSw => &mut self.outer_sw,
            Place::OuterSe => &mut self.outer_se,
            Place::InnerNw => &mut self.inner_nw,
            Place::InnerNe => &mut self.inner_ne,
            Place::InnerSw => &mut self.inner_sw,
            Place::InnerSe => &mut self.inner_se,
            Place::DiagNwse => &mut self.diag_nwse,
            Place::DiagNesw => &mut self.diag_nesw,
        }
    }
}

/// One edge set of a tileset: its name, the tile of the pure outside (mask 0), which cell of
/// a cliff edge blocks, and the places.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EdgeSetJson {
    pub name: String,
    pub outside: String,
    #[serde(default, skip_serializing_if = "is_upper")]
    pub block: EdgeBlock,
    pub places: EdgePlaces,
}

/// Problems of set `index` (paths relative to `edge_sets[index]`) against a tileset of
/// `tiles` tiles, each `<path>: <text>` in file order: an empty name („Ein Kantensatz braucht
/// einen Namen.“), a token [`parse_tile`] refuses (its sentence), `.` („`.` ist keine Kachel
/// eines Platzes.“), a tile past the tileset ([`tile_outside`]), a token that stands twice in
/// the set („Kachel 0007r1 steht zweimal im Satz („n“ und „e“).“, `outside` named
/// „outside“).
pub fn edge_set_problems(
    set: &EdgeSetJson,
    index: usize,
    package: u16,
    tiles: usize,
) -> Vec<String> {
    let base = format!("edge_sets[{index}]");
    let mut problems = Vec::new();
    if set.name.trim().is_empty() {
        problems.push(t!("world.edge_set_needs_name", path = base).into_owned());
    }
    // The tokens read so far, each with the key it stands under.
    let mut seen: Vec<(TileRef, &str)> = Vec::new();
    let mut entry = |path: String, token: &str, key: &'static str, problems: &mut Vec<String>| {
        let tile = match parse_tile(token) {
            Err(why) => return problems.push(format!("{path}: {why}")),
            Ok(None) => {
                return problems.push(t!("world.edge_set_dot", path = path).into_owned());
            }
            Ok(Some(tile)) => tile,
        };
        if usize::from(tile.tile) >= tiles {
            return problems.push(tile_outside(&path, tile.tile, package, tiles));
        }
        if let Some((_, first)) = seen.iter().find(|(t, _)| *t == tile) {
            return problems.push(
                t!(
                    "world.edge_set_twice",
                    path = path,
                    tile = tile_token(Some(tile)),
                    first = first,
                    key = key
                )
                .into_owned(),
            );
        }
        seen.push((tile, key));
    };
    entry(
        format!("{base}.outside"),
        &set.outside,
        "outside",
        &mut problems,
    );
    for place in Place::ALL {
        let key = place.key();
        for (i, token) in set.places.get(place).iter().enumerate() {
            entry(
                format!("{base}.places.{key}[{i}]"),
                token,
                key,
                &mut problems,
            );
        }
    }
    problems
}

/// The variant of a place with `n` entries for cell `cell` (z · 32 + x) and stroke seed `seed`
/// (rule „Zufall“): splitmix64 of `seed << 32 | cell`, modulo `n`; 0 for `n` 0 or 1.
pub fn variant(seed: u32, cell: usize, n: usize) -> usize {
    if n <= 1 {
        return 0;
    }
    let mut z = ((u64::from(seed) << 32) | cell as u64).wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    (z % n as u64) as usize
}

/// One operation of `tileset_edit` on the edge sets: `{"put": {…}}` replaces the set of the
/// same name in its place or appends it, `{"remove": {"name": …}}` removes it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
// A request holds a handful of operations for one call; boxing would only change the
// interface the brushes and the editor build on.
#[allow(clippy::large_enum_variant)]
pub enum EdgeSetOp {
    Put(EdgeSetJson),
    Remove { name: String },
}

impl EdgeSetOp {
    /// The name of the set the operation is about.
    pub fn name(&self) -> &str {
        match self {
            EdgeSetOp::Put(set) => &set.name,
            EdgeSetOp::Remove { name } => name,
        }
    }
}

/// The label of a `tileset_edit` with sets, after its first operation `op`: „Edge set “Path”
/// added: Tileset 0006“ (a put of a new set), „… changed: …“ (a put of a set that
/// `existed`), „… removed: …“.
pub fn edge_set_label(op: &EdgeSetOp, existed: bool, package: u16) -> String {
    let key = match op {
        EdgeSetOp::Put(_) if existed => "world.edge_set_changed_label",
        EdgeSetOp::Put(_) => "world.edge_set_added_label",
        EdgeSetOp::Remove { .. } => "world.edge_set_removed_label",
    };
    t!(key, name = op.name(), package = package : {:04}).into_owned()
}

/// „Edge set “Path” does not exist in tileset 0006.“
pub fn missing_set(name: &str, package: u16) -> String {
    t!("world.edge_set_missing", name = name, package = package : {:04}).into_owned()
}

/// The edge sets of the synthetic tileset [`super::tileset::Tileset::synthetic_edges`].
#[cfg(any(test, feature = "test-support"))]
pub mod fixtures {
    use super::{EdgeBlock, EdgePlaces, EdgeSetJson};

    fn tokens(t: &[&str]) -> Vec<String> {
        t.iter().map(|s| (*s).to_owned()).collect()
    }

    /// „Weg“: the 13 places of the gate on tiles `0006` (center) to `0012`, outside `0000`.
    pub fn weg13() -> EdgeSetJson {
        EdgeSetJson {
            name: "Weg".to_owned(),
            outside: "0000".to_owned(),
            block: EdgeBlock::Upper,
            places: EdgePlaces {
                center: tokens(&["0006"]),
                n: tokens(&["0007"]),
                e: tokens(&["0008"]),
                s: tokens(&["0009"]),
                w: tokens(&["000a"]),
                outer_nw: tokens(&["000b"]),
                outer_ne: tokens(&["000c"]),
                outer_sw: tokens(&["000d"]),
                outer_se: tokens(&["000e"]),
                inner_nw: tokens(&["000f"]),
                inner_ne: tokens(&["0010"]),
                inner_sw: tokens(&["0011"]),
                inner_se: tokens(&["0012"]),
                ..EdgePlaces::default()
            },
        }
    }

    /// „Weg15“: [`weg13`] with the diagonals `0013` (NW–SO) and `0014` (NO–SW) and the center
    /// with the variant `0015`.
    pub fn weg15() -> EdgeSetJson {
        let mut set = weg13();
        set.name = "Weg15".to_owned();
        set.places.diag_nwse = tokens(&["0013"]);
        set.places.diag_nesw = tokens(&["0014"]);
        set.places.center = tokens(&["0006", "0015"]);
        set
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::fixtures::{weg13, weg15};
    use super::*;
    use crate::textproject::canon::to_canonical;

    fn keys(v: &Value) -> Vec<&str> {
        v.as_object().unwrap().keys().map(String::as_str).collect()
    }

    #[test]
    fn masks_are_the_fifteen_non_zero_ones() {
        let mut seen = [false; 16];
        for p in Place::ALL {
            let m = p.mask();
            assert!((1..=15).contains(&m), "{p:?}");
            assert!(!seen[usize::from(m)], "{p:?} twice");
            seen[usize::from(m)] = true;
            assert_eq!(Place::of_mask(m), Some(p));
        }
        assert!(seen[1..].iter().all(|&s| s));
        for m in 1..=15u8 {
            assert_eq!(Place::of_mask(m).unwrap().mask(), m);
        }
        assert_eq!(Place::of_mask(0), None);
        assert_eq!(Place::of_mask(16), None);
        assert_eq!(Place::of_mask(255), None);
        // The table „Plätze und Masken“.
        let masks: Vec<u8> = Place::ALL.iter().map(|p| p.mask()).collect();
        assert_eq!(masks, [15, 12, 5, 3, 10, 8, 4, 2, 1, 14, 13, 11, 7, 9, 6]);
        assert_eq!((NW, NE, SW, SE), (1, 2, 4, 8));
        assert_eq!(Place::N.mask(), SW | SE);
        assert_eq!(Place::OuterNw.mask(), SE);
        assert_eq!(Place::InnerNw.mask(), NE | SW | SE);
    }

    #[test]
    fn gate13_has_no_diagonal() {
        assert_eq!(Place::GATE13.len(), 13);
        assert!(!Place::GATE13.contains(&Place::DiagNwse));
        assert!(!Place::GATE13.contains(&Place::DiagNesw));
        assert_eq!(Place::GATE13[..], Place::ALL[..13]);
    }

    #[test]
    fn keys_and_labels_in_file_order() {
        let keys: Vec<&str> = Place::ALL.iter().map(|p| p.key()).collect();
        assert_eq!(
            keys,
            [
                "center",
                "n",
                "e",
                "s",
                "w",
                "outer_nw",
                "outer_ne",
                "outer_sw",
                "outer_se",
                "inner_nw",
                "inner_ne",
                "inner_sw",
                "inner_se",
                "diag_nwse",
                "diag_nesw"
            ]
        );
        let labels: Vec<String> = Place::ALL.iter().map(|p| p.label()).collect();
        assert_eq!(
            labels,
            [
                "Center",
                "Edge N",
                "Edge E",
                "Edge S",
                "Edge W",
                "Outer corner NW",
                "Outer corner NE",
                "Outer corner SW",
                "Outer corner SE",
                "Inner corner NW",
                "Inner corner NE",
                "Inner corner SW",
                "Inner corner SE",
                "Diagonal NW–SE",
                "Diagonal NE–SW"
            ]
        );
        // `get` reads the field named by the key.
        let set = weg15();
        let v = serde_json::to_value(&set.places).unwrap();
        for p in Place::ALL {
            let want: Vec<String> = serde_json::from_value(v[p.key()].clone()).unwrap();
            assert_eq!(set.places.get(p), want.as_slice(), "{p:?}");
        }
        let mut places = EdgePlaces::default();
        places.get_mut(Place::InnerSe).push("0012".to_owned());
        assert_eq!(
            serde_json::to_value(&places).unwrap(),
            json!({"inner_se": ["0012"]})
        );
    }

    #[test]
    fn a_set_round_trips_in_key_order() {
        let set = weg13();
        let bytes = to_canonical(&set);
        let v: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(keys(&v), ["name", "outside", "places"]);
        assert_eq!(
            keys(&v["places"]),
            [
                "center", "n", "e", "s", "w", "outer_nw", "outer_ne", "outer_sw", "outer_se",
                "inner_nw", "inner_ne", "inner_sw", "inner_se"
            ]
        );
        assert_eq!(v["places"]["center"], json!(["0006"]));
        assert_eq!(v["places"]["inner_se"], json!(["0012"]));
        let back: EdgeSetJson = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(back, set);
        assert_eq!(to_canonical(&back), bytes);

        let w15: Value = serde_json::to_value(weg15()).unwrap();
        assert_eq!(keys(&w15["places"]).len(), 15);
        assert_eq!(w15["places"]["center"], json!(["0006", "0015"]));

        let lower = EdgeSetJson {
            block: EdgeBlock::Lower,
            ..weg13()
        };
        let v = serde_json::to_value(&lower).unwrap();
        assert_eq!(keys(&v), ["name", "outside", "block", "places"]);
        assert_eq!(v["block"], "lower");
        assert_eq!(serde_json::from_value::<EdgeSetJson>(v).unwrap(), lower);

        // `block` left out reads as upper; unknown keys are refused, also in `places`.
        let mut v = serde_json::to_value(weg13()).unwrap();
        assert_eq!(
            serde_json::from_value::<EdgeSetJson>(v.clone())
                .unwrap()
                .block,
            EdgeBlock::Upper
        );
        v["places"]["middle"] = json!([]);
        let e = serde_json::from_value::<EdgeSetJson>(v).unwrap_err();
        assert!(e.to_string().contains("unknown field `middle`"), "{e}");
        let mut v = serde_json::to_value(weg13()).unwrap();
        v["weight"] = json!(1);
        assert!(serde_json::from_value::<EdgeSetJson>(v).is_err());

        // The operations: {"put": {…}} and {"remove": {"name": …}}.
        let put: EdgeSetOp =
            serde_json::from_value(json!({"put": serde_json::to_value(weg13()).unwrap()})).unwrap();
        assert_eq!(put, EdgeSetOp::Put(weg13()));
        let remove: EdgeSetOp = serde_json::from_value(json!({"remove": {"name": "Weg"}})).unwrap();
        assert_eq!(
            remove,
            EdgeSetOp::Remove {
                name: "Weg".to_owned()
            }
        );
        assert_eq!(
            serde_json::to_value(&remove).unwrap(),
            json!({"remove": {"name": "Weg"}})
        );
        assert!(
            serde_json::from_value::<EdgeSetOp>(json!({"remove": {"name": "Weg", "x": 1}}))
                .is_err()
        );
        assert!(serde_json::from_value::<EdgeSetOp>(json!({"drop": {"name": "Weg"}})).is_err());
    }

    #[test]
    fn variant_is_stable_and_spread() {
        assert_eq!(variant(7, 100, 3), variant(7, 100, 3));
        for seed in [0u32, 1] {
            let mut hits = [0usize; 2];
            for cell in 0..1024 {
                let v = variant(seed, cell, 2);
                assert!(v < 2);
                hits[v] += 1;
            }
            for h in hits {
                assert!((400..=624).contains(&h), "seed {seed}: {hits:?}");
            }
        }
        for cell in 0..1024 {
            assert_eq!(variant(5, cell, 1), 0);
        }
        // Seeds give different choices somewhere.
        assert!((0..1024).any(|c| variant(0, c, 2) != variant(1, c, 2)));
        // splitmix64 of (seed << 32 | cell), the constants of the rule „Zufall“.
        assert_eq!(
            variant(0, 0, u32::MAX as usize),
            (0xe220_a839_7b1d_cdaf_u64 % 0xffff_ffff) as usize
        );
    }

    #[test]
    fn edge_set_problems_name_each_case() {
        let problems = |change: &dyn Fn(&mut EdgeSetJson)| {
            let mut set = weg13();
            change(&mut set);
            edge_set_problems(&set, 0, 0, 22)
        };
        let s = |v: &[&str]| v.iter().map(|t| (*t).to_owned()).collect::<Vec<String>>();
        assert_eq!(edge_set_problems(&weg13(), 0, 0, 22), Vec::<String>::new());
        assert_eq!(edge_set_problems(&weg15(), 0, 0, 22), Vec::<String>::new());
        // A tile may fill several places turned.
        assert_eq!(
            problems(&|set| *set.places.get_mut(Place::E) = s(&["0007r1"])),
            Vec::<String>::new()
        );
        assert_eq!(
            problems(&|set| set.name = String::new()),
            ["edge_sets[0].name: An edge set needs a name."]
        );
        assert_eq!(
            problems(&|set| set.name = "  ".to_owned()),
            ["edge_sets[0].name: An edge set needs a name."]
        );
        let mut empty = weg13();
        empty.name = String::new();
        assert_eq!(
            edge_set_problems(&empty, 2, 6, 22),
            ["edge_sets[2].name: An edge set needs a name."]
        );
        assert_eq!(
            problems(&|set| *set.places.get_mut(Place::N) = s(&["0007", "0g07"])),
            [
                "edge_sets[0].places.n[1]: `0g07` is not a tile (`.` or four hex digits, followed by `r1` to `r3` and `f`)."
            ]
        );
        assert_eq!(
            problems(&|set| set.outside = "zz".to_owned()),
            [
                "edge_sets[0].outside: `zz` is not a tile (`.` or four hex digits, followed by `r1` to `r3` and `f`)."
            ]
        );
        assert_eq!(
            problems(&|set| *set.places.get_mut(Place::N) = s(&["0007", "."])),
            ["edge_sets[0].places.n[1]: `.` is not a tile of a place."]
        );
        assert_eq!(
            problems(&|set| set.outside = ".".to_owned()),
            ["edge_sets[0].outside: `.` is not a tile of a place."]
        );
        assert_eq!(
            problems(&|set| *set.places.get_mut(Place::N) = s(&["0016"])),
            ["edge_sets[0].places.n[0]: Tile 0x0016 does not exist in tileset 0000 (22 tiles)."]
        );
        assert_eq!(
            problems(&|set| set.outside = "0100".to_owned()),
            ["edge_sets[0].outside: Tile 0x0100 does not exist in tileset 0000 (22 tiles)."]
        );
        assert_eq!(
            problems(&|set| {
                *set.places.get_mut(Place::N) = s(&["0007r1"]);
                *set.places.get_mut(Place::E) = s(&["0007r1"]);
            }),
            ["edge_sets[0].places.e[0]: Tile 0007r1 stands twice in the set (“n” and “e”)."]
        );
        // Compared after `parse_tile`: upper case is the same token.
        assert_eq!(
            problems(&|set| {
                *set.places.get_mut(Place::N) = s(&["0007r1"]);
                *set.places.get_mut(Place::E) = s(&["0007R1"]);
            }),
            ["edge_sets[0].places.e[0]: Tile 0007r1 stands twice in the set (“n” and “e”)."]
        );
        assert_eq!(
            problems(&|set| *set.places.get_mut(Place::Center) = s(&["0006", "0006"])),
            [
                "edge_sets[0].places.center[1]: Tile 0006 stands twice in the set (“center” and “center”)."
            ]
        );
        assert_eq!(
            problems(&|set| set.places.get_mut(Place::InnerSe).push("0000".to_owned())),
            [
                "edge_sets[0].places.inner_se[1]: Tile 0000 stands twice in the set (“outside” and “inner_se”)."
            ]
        );
        // Several problems: all of them, in file order.
        assert_eq!(
            problems(&|set| {
                set.name = String::new();
                set.outside = ".".to_owned();
                *set.places.get_mut(Place::W) = s(&["0006"]);
                *set.places.get_mut(Place::DiagNesw) = s(&["0099"]);
            }),
            [
                "edge_sets[0].name: An edge set needs a name.",
                "edge_sets[0].outside: `.` is not a tile of a place.",
                "edge_sets[0].places.w[0]: Tile 0006 stands twice in the set (“center” and “w”).",
                "edge_sets[0].places.diag_nesw[0]: Tile 0x0099 does not exist in tileset 0000 (22 tiles).",
            ]
        );
    }

    #[test]
    fn labels_say_what_happened() {
        let put = EdgeSetOp::Put(weg13());
        assert_eq!(
            edge_set_label(&put, false, 6),
            "Edge set “Weg” added: Tileset 0006"
        );
        assert_eq!(
            edge_set_label(&put, true, 6),
            "Edge set “Weg” changed: Tileset 0006"
        );
        let remove = EdgeSetOp::Remove {
            name: "Weg".to_owned(),
        };
        assert_eq!(
            edge_set_label(&remove, true, 6),
            "Edge set “Weg” removed: Tileset 0006"
        );
        assert_eq!(
            missing_set("Weg", 6),
            "Edge set “Weg” does not exist in tileset 0006."
        );
    }
}
