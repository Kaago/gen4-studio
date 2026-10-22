#![cfg(feature = "test-support")]
mod common;

use g4_kernel::project::Project;
use g4_kernel::sprites::{Facing, OverworldTable, SpriteCache};

/// Opens the project named by `G4_TEST_PROJECT` (read only) if its game code is one of
/// `codes`; the gate script runs this once per vanilla project.
fn project_of(codes: &[&str]) -> Option<Project> {
    let root = common::vanilla_project()?;
    let project = Project::open(root).unwrap();
    if codes.contains(&project.game_code.as_str()) {
        Some(project)
    } else {
        eprintln!("skipped: {} is not one of {codes:?}", project.game_code);
        None
    }
}

#[test]
fn platinum_german_table_has_440_entries() {
    let Some(project) = project_of(&["CPUD"]) else {
        return;
    };
    let table = OverworldTable::load(&project).unwrap();
    assert_eq!(table.len(), 440);
    assert_eq!(table.sprite_for(0), Some(90));
}

#[test]
fn heartgold_german_table_has_901_entries() {
    let Some(project) = project_of(&["IPKD"]) else {
        return;
    };
    let table = OverworldTable::load(&project).unwrap();
    assert_eq!(table.len(), 901);
    assert_eq!(table.sprite_for(1), Some(0));
}

#[test]
fn heartgold_english_table_loads() {
    let Some(project) = project_of(&["IPKE"]) else {
        return;
    };
    let table = OverworldTable::load(&project).unwrap();
    assert!(!table.is_empty());
}

/// Graphics id 1 is a regular overworld sprite in Platinum, HeartGold and HeartGold US.
#[test]
fn object_sprite_decodes_graphics_id_1() {
    let Some(project) = project_of(&["CPUD", "IPKD", "IPKE"]) else {
        return;
    };
    let mut cache = SpriteCache::new();
    for facing in [Facing::Up, Facing::Down, Facing::Left, Facing::Right] {
        let sprite = cache
            .object_sprite(&project, 1, facing)
            .unwrap()
            .unwrap_or_else(|| panic!("{}: no sprite for graphics id 1", project.game_code));
        assert!(sprite.width > 0 && sprite.height > 0);
        assert_eq!(
            sprite.pixels.len(),
            (sprite.width * sprite.height * 4) as usize
        );
        assert!(
            sprite.pixels.chunks(4).any(|p| p[3] == 255),
            "{}: sprite is fully transparent",
            project.game_code
        );
    }
}
