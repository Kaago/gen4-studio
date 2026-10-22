//! Overworld sprites of object events.

use g4_kernel::sprites::Facing;
use serde::Serialize;
use tauri::State;

use super::{lock, session};
use crate::errors::CmdError;
use crate::state::AppState;

/// A decoded sprite: RGBA8, row-major, row 0 is the top.
#[derive(Debug, Serialize)]
pub struct SpriteDto {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// The sprite of the object with `graphics_id` facing `facing` (0 up, 1 down, 2 left, 3 right,
/// anything else down); `None` when the object has none (no table entry, a 3D overworld, an
/// unsupported format), so the scene keeps its marker. The cache is per session and in memory
/// after the first read, so the state lock is held only briefly.
pub fn get_object_sprite_impl(
    state: &AppState,
    graphics_id: u16,
    facing: u8,
) -> Result<Option<SpriteDto>, CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    let sprite = session.sprites.object_sprite(
        &session.project,
        graphics_id,
        Facing::from_dir(i16::from(facing)),
    )?;
    Ok(sprite.map(|s| SpriteDto {
        width: s.width,
        height: s.height,
        rgba: s.pixels.clone(),
    }))
}

#[tauri::command]
pub fn get_object_sprite(
    state: State<'_, AppState>,
    graphics_id: u16,
    facing: u8,
) -> Result<Option<SpriteDto>, CmdError> {
    get_object_sprite_impl(&state, graphics_id, facing)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::path::Path;

    use g4_formats::narc::Narc;
    use g4_formats::test_support::btx0;

    use super::*;
    use crate::commands::open_synthetic;

    fn write(root: &Path, rel: &str, bytes: &[u8]) {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, bytes).unwrap();
    }

    /// Gives the synthetic Platinum project an overworld table (`0 -> 1`, `3 -> 0`) in
    /// overlay 5 and a sprite NARC of a model (member 0) and a 16 x 8 sprite (member 1; two
    /// frames, the second texel of the first row opaque).
    fn add_sprites(root: &Path) {
        const OV5_CPUE: usize = 0x2BC34;
        let table: Vec<u8> = [(0u32, 1u32), (3, 0), (0xFFFF, 0)]
            .iter()
            .flat_map(|&(e, s)| [e.to_le_bytes(), s.to_le_bytes()].concat())
            .collect();
        let overlays = format!(
            "table_signed: false\noverlays:\n  - id: 5\n    base_address: 35774464\n    \
             code_size: {}\n    bss_size: 0\n    ctor_start: 35774464\n    ctor_end: 35774464\n    \
             file_id: 5\n    compressed: true\n    signed: false\n    file_name: ov005.bin\n",
            OV5_CPUE + table.len()
        );
        write(root, "arm9_overlays/overlays.yaml", overlays.as_bytes());
        let mut ov5 = vec![0u8; OV5_CPUE];
        ov5.extend_from_slice(&table);
        write(root, "arm9_overlays/ov005.bin", &ov5);
        let mut texels = vec![0u8; 16 * 8 / 2];
        texels[0] = 0x10;
        let palette: Vec<u16> = (0..16).map(|c| 0x7FFF - c).collect();
        let btx = btx0(
            &[(16, 8, 3, false, &texels), (16, 8, 3, false, &texels)],
            &[&palette],
        );
        let narc = Narc {
            members: vec![[b"BMD0".as_slice(), &[0; 8]].concat(), btx],
        };
        write(root, "files/data/mmodel/mmodel.narc", &narc.write());
    }

    #[test]
    fn get_object_sprite_returns_null_for_unknown_id() {
        let (fixture, state) = open_synthetic();
        add_sprites(fixture.dir.path());
        assert!(get_object_sprite_impl(&state, 77, 1).unwrap().is_none());
        // Member 0 is a model, not a BTX0.
        assert!(get_object_sprite_impl(&state, 3, 1).unwrap().is_none());
    }

    #[test]
    fn get_object_sprite_decodes_known_id() {
        let (fixture, state) = open_synthetic();
        add_sprites(fixture.dir.path());
        let sprite = get_object_sprite_impl(&state, 0, 1).unwrap().unwrap();
        assert_eq!((sprite.width, sprite.height), (16, 8));
        assert_eq!(sprite.rgba.len(), 16 * 8 * 4);
        // Index 0 is transparent, index 1 opaque; row 0 comes first.
        assert_eq!(sprite.rgba[3], 0);
        assert_eq!(sprite.rgba[7], 255);
        // A facing the table cannot name is treated as down.
        let other = get_object_sprite_impl(&state, 0, 200).unwrap().unwrap();
        assert_eq!(other.rgba, sprite.rgba);
    }

    #[test]
    fn get_object_sprite_reports_a_missing_table_as_an_error() {
        let (_fixture, state) = open_synthetic();
        // The synthetic project has no overlays: the table cannot be read.
        let err = get_object_sprite_impl(&state, 0, 1).unwrap_err();
        assert_eq!(err.code, "error");
        assert!(get_object_sprite_impl(&AppState::default(), 0, 1).is_err());
    }
}
