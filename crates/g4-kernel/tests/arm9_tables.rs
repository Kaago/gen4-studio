//! Gated (`G4_TEST_PROJECT`, read only): the arm9 tables of the vanilla projects are found and
//! plausible. Needs no crate feature, so `cargo test -p g4-kernel --test arm9_tables` runs it.

use std::path::PathBuf;

use g4_kernel::data::arm9::arm9_layout;
use g4_kernel::project::Project;

fn vanilla_project() -> Option<PathBuf> {
    match std::env::var_os("G4_TEST_PROJECT") {
        Some(p) => Some(PathBuf::from(p)),
        None => {
            eprintln!("skipped: G4_TEST_PROJECT not set");
            None
        }
    }
}

#[test]
fn vanilla_arm9_tables_are_plausible() {
    let Some(root) = vanilla_project() else {
        return;
    };
    let p = Project::open(&root).unwrap();
    let layout = arm9_layout(&p.game_code)
        .unwrap_or_else(|| panic!("{}: no arm9_tables.yaml entry", p.game_code));
    assert!(!layout.unverified, "{} is a checked game", p.game_code);

    let items = p.item_table().expect("item table available");
    assert_eq!(Some(items.entries.len()), layout.item_count);
    // Item 17 (Potion) uses data member 17, icon 24, palette 25 in all three games.
    let potion = items.entries[17];
    assert_eq!((potion.data, potion.icon, potion.palette), (17, 24, 25));

    let machines = p.machine_table().expect("machine table available");
    assert_eq!(machines.moves.len(), 100);
    assert_eq!(machines.moves[0], 264, "TM01 Focus Punch");

    let palettes = p.mon_icon_palettes().expect("icon palettes available");
    assert_eq!(Some(palettes.values.len()), layout.mon_icon_palettes_len);
    assert!(palettes.values.iter().all(|v| *v <= 2));
    // Bulbasaur 1, Charmander 0, Squirtle 0 (Task 0 rendering check).
    assert_eq!(
        [palettes.values[1], palettes.values[4], palettes.values[7]],
        [1, 0, 0]
    );
}
