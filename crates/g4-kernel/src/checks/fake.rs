//! A [`Source`] of plain values for the unit tests of the world and data checks, with
//! builders for the fixtures they share.
#![allow(clippy::unwrap_used)]

use std::collections::HashMap;

use g4_formats::area_data::AreaData;
use g4_formats::data::encounter::Encounters;
use g4_formats::data::trainer::{PartyFormat, PartyMon};
use g4_formats::data::trainer_doc::TrainerDoc;
use g4_formats::game::Family;
use g4_formats::map::{Cell, MapFile};
use g4_formats::matrix::Matrix;
use g4_formats::test_support::{bmd0, mdl0_block};
use uxie::event_file::{BgEvent, CoordEvent, EventFile, ObjectEvent, WarpEvent};
use uxie::map_header::{MapHeader, MapHeaderHGSS, MapHeaderPt};

use super::{Finding, Limits, Source, SourceCheck, Unreadable};
use crate::project::{AssetId, AssetKind};

/// Every value a check reads; a kind's count is the length of its list unless `counts`
/// names another.
pub(crate) struct Fake {
    pub family: Family,
    pub headers: Option<Vec<MapHeader>>,
    pub counts: HashMap<AssetKind, usize>,
    pub events: Vec<Option<EventFile>>,
    pub matrices: Vec<Option<Matrix>>,
    pub maps: Vec<Option<MapFile>>,
    pub areas: Vec<Option<AreaData>>,
    pub trainers: Vec<Option<TrainerDoc>>,
    pub encounters: Vec<Option<Encounters>>,
    pub banks: HashMap<&'static str, usize>,
    pub items: usize,
    pub unreadable: Vec<Unreadable>,
    pub locked: Vec<(AssetId, String)>,
}

impl Fake {
    /// A Platinum project without anything.
    pub fn pt() -> Fake {
        Fake {
            family: Family::Pt,
            headers: Some(Vec::new()),
            counts: HashMap::new(),
            events: Vec::new(),
            matrices: Vec::new(),
            maps: Vec::new(),
            areas: Vec::new(),
            trainers: Vec::new(),
            encounters: Vec::new(),
            banks: HashMap::from([("species", 496), ("trainer_classes", 105)]),
            items: 468,
            unreadable: Vec::new(),
            locked: Vec::new(),
        }
    }

    /// A Platinum world: header 0 and 1 on matrix 0 (2 x 1 cells, maps 0 and 1) with event
    /// files 0 and 1; every tile plain ground (behaviour 0, collision 0).
    pub fn world() -> Fake {
        let mut f = Fake::pt();
        f.headers = Some(vec![pt_header(0, 0), pt_header(0, 1)]);
        f.matrices = vec![Some(matrix(2, 1, vec![0, 1], None))];
        f.maps = vec![Some(map()), Some(map())];
        f.events = vec![Some(events()), Some(events())];
        f
    }

    /// Runs `check` and returns its findings (panics when it skips).
    pub fn run(&self, check: &impl SourceCheck) -> Vec<Finding> {
        let mut out = Vec::new();
        check
            .check(self, Limits::get(), &mut out)
            .unwrap_or_else(|s| panic!("skipped: {}", s.why));
        for f in &out {
            assert_eq!(f.code, check_code(check));
        }
        out
    }

    pub fn event_mut(&mut self, i: usize) -> &mut EventFile {
        self.events[i].as_mut().unwrap()
    }

    pub fn map_mut(&mut self, i: usize) -> &mut MapFile {
        self.maps[i].as_mut().unwrap()
    }

    pub fn header_mut(&mut self, i: usize) -> &mut MapHeader {
        &mut self.headers.as_mut().unwrap()[i]
    }
}

fn check_code<T: SourceCheck>(_: &T) -> &'static str {
    T::CODE
}

fn at<T>(list: &[Option<T>], index: u16) -> Option<&T> {
    list.get(usize::from(index))?.as_ref()
}

impl Source for Fake {
    fn family(&self) -> Family {
        self.family
    }
    fn headers(&self) -> Option<&[MapHeader]> {
        self.headers.as_deref()
    }
    fn count(&self, kind: AssetKind) -> usize {
        if let Some(&n) = self.counts.get(&kind) {
            return n;
        }
        match kind {
            AssetKind::Event => self.events.len(),
            AssetKind::Matrix => self.matrices.len(),
            AssetKind::Map => self.maps.len(),
            AssetKind::AreaData => self.areas.len(),
            AssetKind::Trainer => self.trainers.len(),
            AssetKind::Encounter => self.encounters.len(),
            AssetKind::HeaderTable => 1,
            _ => 0,
        }
    }
    fn event(&self, index: u16) -> Option<&EventFile> {
        at(&self.events, index)
    }
    fn matrix(&self, index: u16) -> Option<&Matrix> {
        at(&self.matrices, index)
    }
    fn map(&self, index: u16) -> Option<&MapFile> {
        at(&self.maps, index)
    }
    fn area_data(&self, index: u16) -> Option<&AreaData> {
        at(&self.areas, index)
    }
    fn trainer(&self, index: u16) -> Option<&TrainerDoc> {
        at(&self.trainers, index)
    }
    fn encounter(&self, index: u16) -> Option<&Encounters> {
        at(&self.encounters, index)
    }
    fn bank_len(&self, bank: &str) -> Option<usize> {
        self.banks.get(bank).copied()
    }
    fn item_count(&self) -> usize {
        self.items
    }
    fn unreadable_all(&self) -> (Vec<Unreadable>, Vec<(AssetId, String)>) {
        (self.unreadable.clone(), self.locked.clone())
    }
}

/// A Platinum header on `matrix` with `event_file`, script file 0, level script 1, text
/// archive 0, area data 0 and no encounters.
pub(crate) fn pt_header(matrix: u16, event_file: u16) -> MapHeader {
    MapHeader::Pt(MapHeaderPt {
        matrix_id: matrix,
        event_file_id: event_file,
        level_script_id: 1,
        wild_pokemon: 0xFFFF,
        ..MapHeaderPt::default()
    })
}

/// A HeartGold header on `matrix` with `event_file` and no encounters.
pub(crate) fn hgss_header(matrix: u16, event_file: u16) -> MapHeader {
    MapHeader::HGSS(MapHeaderHGSS {
        matrix_id: matrix,
        event_file_id: event_file,
        level_script_id: 1,
        wild_pokemon: 0xFF,
        ..MapHeaderHGSS::default()
    })
}

pub(crate) fn matrix(width: u8, height: u8, maps: Vec<u16>, headers: Option<Vec<u16>>) -> Matrix {
    Matrix {
        name: "m".into(),
        width,
        height,
        headers,
        altitudes: None,
        maps,
    }
}

/// A valid model of one model block with `models` models.
pub(crate) fn model(models: &[&str]) -> Vec<u8> {
    bmd0(&[mdl0_block(models, &["t0"], &["p0"])])
}

/// A BDHC with `points` points and nothing else.
pub(crate) fn bdhc(points: u16) -> Vec<u8> {
    let mut b = b"BDHC".to_vec();
    b.extend_from_slice(&points.to_le_bytes());
    b.resize(16 + usize::from(points) * 8, 0);
    b
}

/// A Platinum map of plain ground with a valid model and terrain and no buildings.
pub(crate) fn map() -> MapFile {
    MapFile {
        bgs: None,
        permissions: vec![Cell::default(); 1024],
        buildings: Vec::new(),
        buildings_tail: Vec::new(),
        model: model(&["m"]),
        bdhc: bdhc(1),
    }
}

pub(crate) fn events() -> EventFile {
    EventFile {
        bg_events: Vec::new(),
        object_events: Vec::new(),
        warp_events: Vec::new(),
        coord_events: Vec::new(),
    }
}

pub(crate) fn warp(x: u16, z: u16, dest_header_id: u16, dest_warp_id: u16) -> WarpEvent {
    WarpEvent {
        x,
        z,
        dest_header_id,
        dest_warp_id,
        height: 0,
    }
}

/// An object at (`x`, `z`) with `script`, trainer type 1 and no hidden flag.
pub(crate) fn object(x: u16, z: u16, script: u16) -> ObjectEvent {
    ObjectEvent {
        local_id: 0,
        graphics_id: 1,
        movement_type: 0,
        trainer_type: 1,
        hidden_flag: 0,
        script,
        dir: 0,
        data: [0; 3],
        movement_range_x: 0,
        movement_range_z: 0,
        x,
        z,
        y: 0,
    }
}

pub(crate) fn sign(x: i32, z: i32, script: u16) -> BgEvent {
    BgEvent {
        script,
        event_type: 0,
        x,
        z,
        y: 0,
        player_facing_dir: 0,
    }
}

pub(crate) fn trigger(x: i16, z: i16, script: u16) -> CoordEvent {
    CoordEvent {
        script,
        x,
        z,
        width: 1,
        length: 1,
        y: 0,
        value: 0,
        var: 0x4000,
    }
}

/// A Platinum party member of `species` at level 5.
pub(crate) fn mon(species: u16) -> PartyMon {
    PartyMon {
        iv_or_difficulty: 0,
        byte1: 0,
        level: 5,
        species_word: species,
        item: None,
        moves: None,
        ball_capsule: Some(0),
    }
}

/// A single battle trainer of class 1 with `party`, without items or moves.
pub(crate) fn trainer(party: Vec<PartyMon>) -> TrainerDoc {
    TrainerDoc {
        class: 1,
        format: PartyFormat {
            moves: false,
            items: false,
        },
        flags_rest: 0,
        unknown: 0,
        items: [0; 4],
        ai: 0,
        battle_type: 0,
        party,
    }
}
