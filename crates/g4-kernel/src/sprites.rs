//! Overworld sprites: the game's overworld table (event object `graphics_id` -> sprite
//! member of the sprite NARC) read from the decompressed overlays, and a per-session cache
//! that decodes object sprites. See `docs/formats/overworld_table.md`.

use std::collections::HashMap;
use std::sync::Arc;

use g4_formats::game::Family;
use g4_formats::narc::Narc;
use g4_formats::nsbtx::{Nsbtx, Rgba};
use serde::Deserialize;

use crate::error::{KernelError, io_at};
use crate::project::Project;

const OVERWORLD_TABLES_YAML: &str = include_str!("../metadata/overworld_tables.yaml");

/// `entry` value that ends the table.
const TABLE_END: u32 = 0xFFFF;
/// Diamond/Pearl and Platinum entries: `u32 entry, u32 sprite`.
const PT_ENTRY_SIZE: usize = 8;
/// HeartGold/SoulSilver entries: `u16 entry, u16 sprite, u16 props`.
const HGSS_ENTRY_SIZE: usize = 6;

#[derive(Debug, Deserialize)]
struct TablesYaml {
    overlay5_offset: HashMap<String, u32>,
    overlay1_pointer: HashMap<String, u32>,
    sprites: HashMap<String, String>,
}

fn tables_yaml() -> Result<TablesYaml, KernelError> {
    serde_yaml::from_str(OVERWORLD_TABLES_YAML)
        .map_err(|e| KernelError::InvalidProject(format!("overworld_tables.yaml: {e}")))
}

fn invalid(message: String) -> KernelError {
    KernelError::InvalidProject(message)
}

/// The game's overworld table: event object `graphics_id` to sprite member.
#[derive(Debug, Clone)]
pub struct OverworldTable {
    entries: HashMap<u16, u16>,
}

impl OverworldTable {
    /// Reads the table from overlay 5 (Diamond/Pearl, Platinum) or through the pointer in
    /// overlay 1 (HeartGold/SoulSilver). Overlays are decompressed on disk, so no BLZ.
    pub fn load(project: &Project) -> Result<OverworldTable, KernelError> {
        let tables = tables_yaml()?;
        let code = project.game_code.as_str();
        let unknown = || KernelError::UnknownGame(code.to_string());
        let (bytes, offset, entry_size) = match project.family {
            Family::Dp | Family::Pt => {
                let offset = *tables.overlay5_offset.get(code).ok_or_else(unknown)?;
                let overlay = project.overlay(5)?;
                let bytes = std::fs::read(&overlay.path).map_err(io_at(&overlay.path))?;
                (bytes, offset as usize, PT_ENTRY_SIZE)
            }
            Family::Hgss => {
                let pointer_at = *tables.overlay1_pointer.get(code).ok_or_else(unknown)?;
                let overlay = project.overlay(1)?;
                let bytes = std::fs::read(&overlay.path).map_err(io_at(&overlay.path))?;
                let end = u64::from(overlay.base_address) + u64::from(overlay.code_size);
                let outside = |address: u32, what: &str| {
                    invalid(format!(
                        "the sprite table {what} {address:#010x} lies outside overlay 1 \
                         ({:#010x}..{end:#010x}); synthetic overlays and ov131 patches are not \
                         supported",
                        overlay.base_address
                    ))
                };
                let inside =
                    |address: u32| address >= overlay.base_address && u64::from(address) < end;
                if !inside(pointer_at) {
                    return Err(outside(pointer_at, "pointer at"));
                }
                let at = (pointer_at - overlay.base_address) as usize;
                let pointer = read_u32(&bytes, at).ok_or_else(|| {
                    invalid(format!(
                        "{} ends before the table pointer at {at:#x}",
                        overlay.path.display()
                    ))
                })?;
                if !inside(pointer) {
                    return Err(outside(pointer, "pointer"));
                }
                (
                    bytes,
                    (pointer - overlay.base_address) as usize,
                    HGSS_ENTRY_SIZE,
                )
            }
        };
        let entries = parse_entries(&bytes, offset, entry_size)?;
        Ok(OverworldTable { entries })
    }

    /// Sprite member for `graphics_id`.
    pub fn sprite_for(&self, graphics_id: u16) -> Option<u16> {
        self.entries.get(&graphics_id).copied()
    }

    /// Number of distinct graphics ids in the table.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

fn read_u32(bytes: &[u8], at: usize) -> Option<u32> {
    let end = at.checked_add(4)?;
    Some(u32::from_le_bytes(bytes.get(at..end)?.try_into().ok()?))
}

fn read_u16(bytes: &[u8], at: usize) -> Option<u16> {
    let end = at.checked_add(2)?;
    Some(u16::from_le_bytes(bytes.get(at..end)?.try_into().ok()?))
}

/// Entries from `offset` up to the `0xFFFF` terminator. The first entry of a graphics id
/// wins: the game looks ids up linearly.
fn parse_entries(
    bytes: &[u8],
    offset: usize,
    entry_size: usize,
) -> Result<HashMap<u16, u16>, KernelError> {
    let mut entries = HashMap::new();
    let mut at = offset;
    loop {
        let (entry, sprite) = read_entry(bytes, at, entry_size).ok_or_else(|| {
            invalid(format!(
                "the overworld table at {offset:#x} has no {TABLE_END:#x} terminator before the \
                 end of the overlay ({} bytes)",
                bytes.len()
            ))
        })?;
        if entry == TABLE_END {
            return Ok(entries);
        }
        let entry = u16::try_from(entry)
            .map_err(|_| invalid(format!("overworld table entry {entry:#x} at {at:#x}")))?;
        let sprite = u16::try_from(sprite).map_err(|_| {
            invalid(format!(
                "overworld table sprite {sprite:#x} of entry {entry} at {at:#x}"
            ))
        })?;
        entries.entry(entry).or_insert(sprite);
        at += entry_size;
    }
}

/// `(entry, sprite)` of the record at `at`, or `None` when it does not fit in `bytes`.
fn read_entry(bytes: &[u8], at: usize, entry_size: usize) -> Option<(u32, u32)> {
    if entry_size == PT_ENTRY_SIZE {
        Some((read_u32(bytes, at)?, read_u32(bytes, at.checked_add(4)?)?))
    } else {
        let end = at.checked_add(entry_size)?;
        bytes.get(at..end)?;
        Some((
            u32::from(read_u16(bytes, at)?),
            u32::from(read_u16(bytes, at + 2)?),
        ))
    }
}

/// Direction of an object event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Facing {
    Up,
    Down,
    Left,
    Right,
}

impl Facing {
    /// From the event's `dir`: 0 up, 1 down, 2 left, 3 right; anything else faces down.
    pub fn from_dir(dir: i16) -> Facing {
        match dir {
            0 => Facing::Up,
            2 => Facing::Left,
            3 => Facing::Right,
            _ => Facing::Down,
        }
    }
}

/// Texture shown for `facing` by a sprite with `texture_count` textures (DSPRE's choice,
/// spec 4.1).
pub fn frame_index(texture_count: usize, facing: Facing) -> usize {
    let [up, down, left, right] = match texture_count {
        0..=1 => [0, 0, 0, 0],
        2..=4 => [0, 1, 2, 3],
        5..=8 => [0, 2, 4, 6],
        9..=16 => [0, 11, 2, 4],
        _ => [0, 27, 2, 4],
    };
    match facing {
        Facing::Up => up,
        Facing::Down => down,
        Facing::Left => left,
        Facing::Right => right,
    }
}

/// DSPRE treats these Diamond/Pearl/Platinum ids (signs, variable-dependent characters) as
/// 3D overworlds without a sprite.
pub fn is_3d_overworld(family: Family, graphics_id: u16) -> bool {
    matches!(family, Family::Dp | Family::Pt) && matches!(graphics_id, 91..=96 | 101..=116)
}

/// Overworld table, sprite NARC and decoded sprites of one session. Everything is read at
/// most once: failures of the table or the NARC are remembered and returned on every call.
pub struct SpriteCache {
    table: Option<Result<OverworldTable, String>>,
    narc: Option<Result<Narc, String>>,
    sprites: HashMap<(u16, u8), Option<Arc<Rgba>>>,
}

impl Default for SpriteCache {
    fn default() -> Self {
        Self::new()
    }
}

impl SpriteCache {
    pub fn new() -> Self {
        SpriteCache {
            table: None,
            narc: None,
            sprites: HashMap::new(),
        }
    }

    /// The sprite of the object with `graphics_id` facing `facing`, decoded with palette 0
    /// and palette index 0 transparent.
    ///
    /// `Ok(None)` means "no sprite, show a marker": no table entry, a 3D overworld, a sprite
    /// member that is missing or not a BTX0 (the NARCs also hold BMD0 models), or a sprite
    /// that cannot be read or decoded (broken data, format 5). Errors are for the table and
    /// the NARC themselves.
    pub fn object_sprite(
        &mut self,
        project: &Project,
        graphics_id: u16,
        facing: Facing,
    ) -> Result<Option<Arc<Rgba>>, KernelError> {
        if is_3d_overworld(project.family, graphics_id) {
            return Ok(None);
        }
        let key = (graphics_id, facing as u8);
        if let Some(cached) = self.sprites.get(&key) {
            return Ok(cached.clone());
        }
        let table = self
            .table
            .get_or_insert_with(|| OverworldTable::load(project).map_err(|e| e.to_string()))
            .as_ref()
            .map_err(|e| KernelError::Remembered(e.clone()))?;
        let Some(member) = table.sprite_for(graphics_id) else {
            self.sprites.insert(key, None);
            return Ok(None);
        };
        let narc = self
            .narc
            .get_or_insert_with(|| read_sprite_narc(project).map_err(|e| e.to_string()))
            .as_ref()
            .map_err(|e| KernelError::Remembered(e.clone()))?;
        let sprite = decode_sprite(narc, member, facing).map(Arc::new);
        self.sprites.insert(key, sprite.clone());
        Ok(sprite)
    }
}

fn read_sprite_narc(project: &Project) -> Result<Narc, KernelError> {
    let tables = tables_yaml()?;
    let key = match project.family {
        Family::Dp => "dp",
        Family::Pt => "pt",
        Family::Hgss => "hgss",
    };
    let rel = tables
        .sprites
        .get(key)
        .ok_or_else(|| invalid(format!("overworld_tables.yaml: no sprite NARC for {key}")))?;
    let path = project.root.join(rel);
    let bytes = std::fs::read(&path).map_err(io_at(&path))?;
    Ok(Narc::read(&bytes)?)
}

/// The frame of sprite `member` for `facing`; `None` when the member does not exist, is not
/// a readable BTX0 or does not decode. A frame index past the last texture falls back to
/// frame 0.
fn decode_sprite(narc: &Narc, member: u16, facing: Facing) -> Option<Rgba> {
    let bytes = narc.members.get(usize::from(member))?;
    let btx = Nsbtx::read(bytes).ok()?;
    let count = btx.textures.len();
    if count == 0 {
        return None;
    }
    let frame = match frame_index(count, facing) {
        f if f < count => f,
        _ => 0,
    };
    btx.decode_with(frame, 0, true).ok()
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::path::Path;

    use g4_formats::narc::Narc;
    use g4_formats::test_support::btx0;

    use super::*;
    use crate::test_support::synthetic_project;

    const OV5_CPUE: usize = 0x2BC34;
    const OV1_BASE: u32 = 0x021E_5900;
    const OV1_SIZE: u32 = 0x24260;
    /// RAM address of the table pointer of IPKE and its file offset in `ov001.bin`.
    const OV1_POINTER_IPKE: u32 = 0x021F_92FC;
    const OV1_TABLE: u32 = 0x21BA8;

    fn write(root: &Path, rel: &str, bytes: &[u8]) {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, bytes).unwrap();
    }

    fn overlays_yaml(root: &Path, overlays: &[(u16, u32, u32)]) {
        let mut text = String::from("table_signed: false\noverlays:\n");
        for &(id, base, size) in overlays {
            text.push_str(&format!(
                "  - id: {id}\n    base_address: {base}\n    code_size: {size}\n    bss_size: 0\n    \
                 ctor_start: {base}\n    ctor_end: {base}\n    file_id: {id}\n    compressed: true\n    \
                 signed: false\n    file_name: ov{id:03}.bin\n"
            ));
        }
        write(root, "arm9_overlays/overlays.yaml", text.as_bytes());
    }

    fn pt_entries(entries: &[(u32, u32)]) -> Vec<u8> {
        entries
            .iter()
            .flat_map(|&(e, s)| [e.to_le_bytes(), s.to_le_bytes()].concat())
            .collect()
    }

    fn hgss_entries(entries: &[(u16, u16, u16)]) -> Vec<u8> {
        entries
            .iter()
            .flat_map(|&(e, s, p)| [e.to_le_bytes(), s.to_le_bytes(), p.to_le_bytes()].concat())
            .collect()
    }

    /// Platinum US project whose overlay 5 holds `table` at the CPUE offset.
    fn pt_project(table: &[u8]) -> (tempfile::TempDir, Project) {
        let synth = synthetic_project();
        let root = synth.dir.path();
        overlays_yaml(root, &[(5, 0x0221_E000, (OV5_CPUE + table.len()) as u32)]);
        let mut ov5 = vec![0u8; OV5_CPUE];
        ov5.extend_from_slice(table);
        write(root, "arm9_overlays/ov005.bin", &ov5);
        let project = Project::open(root).unwrap();
        (synth.dir, project)
    }

    /// HeartGold US project whose overlay 1 holds `table` at `OV1_TABLE` and a pointer to
    /// `pointer` at the IPKE pointer address.
    fn hgss_project(pointer: u32, table: &[u8]) -> (tempfile::TempDir, Project) {
        let synth = synthetic_project();
        let root = synth.dir.path();
        std::fs::write(
            root.join("header.yaml"),
            "game_title: \"POKEMON HG\"\ngamecode: IPKE\nmaker_code: \"01\"\n",
        )
        .unwrap();
        overlays_yaml(root, &[(1, OV1_BASE, OV1_SIZE)]);
        let mut ov1 = vec![0u8; OV1_SIZE as usize];
        let at = (OV1_POINTER_IPKE - OV1_BASE) as usize;
        ov1[at..at + 4].copy_from_slice(&pointer.to_le_bytes());
        let t = OV1_TABLE as usize;
        ov1[t..t + table.len()].copy_from_slice(table);
        write(root, "arm9_overlays/ov001.bin", &ov1);
        let project = Project::open(root).unwrap();
        (synth.dir, project)
    }

    #[test]
    fn platinum_table_is_read_from_overlay_5() {
        let (_dir, project) = pt_project(&pt_entries(&[(0, 90), (7, 12), (9, 1), (0xFFFF, 0)]));
        let table = OverworldTable::load(&project).unwrap();
        assert_eq!(table.len(), 3);
        assert_eq!(table.sprite_for(0), Some(90));
        assert_eq!(table.sprite_for(7), Some(12));
        assert_eq!(table.sprite_for(1), None);
    }

    #[test]
    fn heartgold_table_is_read_through_the_overlay_1_pointer() {
        let table = hgss_entries(&[(1, 0, 0), (2, 5, 0x1234), (0xFFFF, 0, 0)]);
        let (_dir, project) = hgss_project(OV1_BASE + OV1_TABLE, &table);
        let table = OverworldTable::load(&project).unwrap();
        assert_eq!(table.len(), 2);
        assert_eq!(table.sprite_for(1), Some(0));
        assert_eq!(table.sprite_for(2), Some(5));
        assert_eq!(table.sprite_for(0), None);
    }

    #[test]
    fn a_pointer_outside_overlay_1_is_an_error() {
        let (_dir, project) = hgss_project(0x023C_8000, &hgss_entries(&[(0xFFFF, 0, 0)]));
        let err = OverworldTable::load(&project).unwrap_err().to_string();
        assert!(err.contains("overlay 1"), "{err}");
    }

    #[test]
    fn an_unterminated_table_is_an_error() {
        // Platinum: the overlay ends after two entries without the 0xFFFF terminator.
        let (_dir, project) = pt_project(&pt_entries(&[(0, 90), (1, 91)]));
        let err = OverworldTable::load(&project).unwrap_err().to_string();
        assert!(err.contains("terminator"), "{err}");
    }

    #[test]
    fn a_missing_overlay_is_an_error() {
        let synth = synthetic_project();
        let project = Project::open(synth.dir.path()).unwrap();
        assert!(OverworldTable::load(&project).is_err());
        overlays_yaml(synth.dir.path(), &[(1, OV1_BASE, OV1_SIZE)]);
        let err = OverworldTable::load(&project).unwrap_err().to_string();
        assert!(err.contains("no overlay 5"), "{err}");
    }

    #[test]
    fn the_first_entry_of_a_duplicate_graphics_id_wins() {
        let (_dir, project) = pt_project(&pt_entries(&[(4, 10), (4, 11), (0xFFFF, 0)]));
        assert_eq!(
            OverworldTable::load(&project).unwrap().sprite_for(4),
            Some(10)
        );
    }

    #[test]
    fn overlay_reads_base_size_and_path() {
        let synth = synthetic_project();
        overlays_yaml(synth.dir.path(), &[(1, OV1_BASE, OV1_SIZE), (5, 7, 9)]);
        let project = Project::open(synth.dir.path()).unwrap();
        let ov = project.overlay(5).unwrap();
        assert_eq!((ov.base_address, ov.code_size), (7, 9));
        assert_eq!(ov.path, synth.dir.path().join("arm9_overlays/ov005.bin"));
        assert!(project.overlay(2).is_err());
    }

    #[test]
    fn frame_index_follows_the_texture_count() {
        use Facing::*;
        assert_eq!(frame_index(16, Down), 11);
        assert_eq!(frame_index(4, Right), 3);
        assert_eq!(frame_index(1, Left), 0);
        assert_eq!(frame_index(20, Down), 27);
        assert_eq!(frame_index(0, Down), 0);
        assert_eq!(frame_index(8, Right), 6);
        assert_eq!(frame_index(8, Left), 4);
        assert_eq!(frame_index(16, Right), 4);
        assert_eq!(frame_index(17, Left), 2);
        for n in [1, 4, 8, 16, 40] {
            assert_eq!(frame_index(n, Up), 0);
        }
    }

    #[test]
    fn facing_from_dir() {
        assert_eq!(Facing::from_dir(0), Facing::Up);
        assert_eq!(Facing::from_dir(1), Facing::Down);
        assert_eq!(Facing::from_dir(2), Facing::Left);
        assert_eq!(Facing::from_dir(3), Facing::Right);
        assert_eq!(Facing::from_dir(-1), Facing::Down);
        assert_eq!(Facing::from_dir(9), Facing::Down);
    }

    #[test]
    fn only_dp_and_platinum_have_3d_overworlds() {
        assert!(is_3d_overworld(Family::Pt, 94));
        assert!(is_3d_overworld(Family::Dp, 91));
        assert!(is_3d_overworld(Family::Dp, 116));
        assert!(!is_3d_overworld(Family::Hgss, 94));
        assert!(!is_3d_overworld(Family::Pt, 90));
        assert!(!is_3d_overworld(Family::Pt, 97));
        assert!(!is_3d_overworld(Family::Pt, 100));
        assert!(!is_3d_overworld(Family::Pt, 117));
    }

    // ---- sprite cache ------------------------------------------------------------------

    /// A BTX0 with one 16-colour texture per entry of `sizes` (all texels index 0 except
    /// the second of the first row, which is index 1) and one palette.
    fn sprite_btx(sizes: &[(u32, u32)]) -> Vec<u8> {
        let data: Vec<Vec<u8>> = sizes
            .iter()
            .map(|&(w, h)| {
                let mut d = vec![0u8; (w * h / 2) as usize];
                d[0] = 0x10;
                d
            })
            .collect();
        let textures: Vec<_> = sizes
            .iter()
            .zip(&data)
            .map(|(&(w, h), d)| (w, h, 3, false, d.as_slice()))
            .collect();
        let palette: Vec<u16> = (0..16).map(|c| 0x7FFF - c).collect();
        btx0(&textures, &[&palette])
    }

    const SPRITE_NARC: &str = "files/data/mmodel/mmodel.narc";

    /// Platinum project with the table `0 -> 1, 3 -> 0, 4 -> 9, 94 -> 1` and a sprite NARC of
    /// a model (member 0) and a four-frame sprite (member 1; frame `i` is 8 x 8 * 2^i).
    fn sprite_project() -> (tempfile::TempDir, Project) {
        let (dir, _) = pt_project(&pt_entries(&[(0, 1), (3, 0), (4, 9), (94, 1), (0xFFFF, 0)]));
        let narc = Narc {
            members: vec![
                [b"BMD0".as_slice(), &[0; 8]].concat(),
                sprite_btx(&[(8, 8), (8, 16), (8, 32), (8, 64)]),
            ],
        };
        write(dir.path(), SPRITE_NARC, &narc.write());
        let project = Project::open(dir.path()).unwrap();
        (dir, project)
    }

    #[test]
    fn object_sprite_decodes_the_frame_of_the_facing() {
        let (_dir, project) = sprite_project();
        let mut cache = SpriteCache::new();
        let down = cache
            .object_sprite(&project, 0, Facing::Down)
            .unwrap()
            .unwrap();
        assert_eq!((down.width, down.height), (8, 16));
        let right = cache
            .object_sprite(&project, 0, Facing::Right)
            .unwrap()
            .unwrap();
        assert_eq!((right.width, right.height), (8, 64));
        // Palette index 0 is transparent although the texture's color0 flag is clear;
        // index 1 is opaque.
        assert_eq!(down.pixels[3], 0);
        assert_eq!(down.pixels[7], 255);
    }

    #[test]
    fn object_sprite_is_none_without_a_sprite() {
        let (_dir, project) = sprite_project();
        let mut cache = SpriteCache::new();
        // Not in the table.
        assert!(
            cache
                .object_sprite(&project, 77, Facing::Down)
                .unwrap()
                .is_none()
        );
        // Member 0 is a model, not a BTX0.
        assert!(
            cache
                .object_sprite(&project, 3, Facing::Down)
                .unwrap()
                .is_none()
        );
        // Member 9 does not exist.
        assert!(
            cache
                .object_sprite(&project, 4, Facing::Down)
                .unwrap()
                .is_none()
        );
        // 3D overworld id: has a table entry but no sprite.
        assert!(
            cache
                .object_sprite(&project, 94, Facing::Down)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn a_second_call_reads_no_file() {
        let (dir, project) = sprite_project();
        let mut cache = SpriteCache::new();
        let first = cache
            .object_sprite(&project, 0, Facing::Down)
            .unwrap()
            .unwrap();
        std::fs::remove_file(dir.path().join(SPRITE_NARC)).unwrap();
        std::fs::remove_file(dir.path().join("arm9_overlays/ov005.bin")).unwrap();
        let again = cache
            .object_sprite(&project, 0, Facing::Down)
            .unwrap()
            .unwrap();
        assert!(Arc::ptr_eq(&first, &again));
        // Another facing of the same sprite needs the NARC, which is cached too.
        let left = cache
            .object_sprite(&project, 0, Facing::Left)
            .unwrap()
            .unwrap();
        assert_eq!((left.width, left.height), (8, 32));
        // Misses are cached, and an unknown id needs the cached table only.
        assert!(
            cache
                .object_sprite(&project, 77, Facing::Down)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn a_table_error_is_remembered_with_its_original_message() {
        let synth = synthetic_project();
        let project = Project::open(synth.dir.path()).unwrap();
        let original = OverworldTable::load(&project).unwrap_err().to_string();
        assert!(original.contains("overlays.yaml"), "{original}");
        let mut cache = SpriteCache::new();
        let first = cache
            .object_sprite(&project, 0, Facing::Down)
            .unwrap_err()
            .to_string();
        // The overlays appear afterwards; the cache does not look again.
        overlays_yaml(synth.dir.path(), &[(5, 0x0221_E000, 0x2BC34 + 16)]);
        let mut ov5 = vec![0u8; OV5_CPUE];
        ov5.extend_from_slice(&pt_entries(&[(0xFFFF, 0)]));
        write(synth.dir.path(), "arm9_overlays/ov005.bin", &ov5);
        let second = cache
            .object_sprite(&project, 0, Facing::Down)
            .unwrap_err()
            .to_string();
        // No prefix is added, on the first call or on later ones.
        assert_eq!(first, original);
        assert_eq!(second, original);
    }

    #[test]
    fn a_nested_table_error_keeps_its_exact_message() {
        // overlays.yaml exists but has no overlay 5: a metadata error, which used to get a
        // second "invalid project metadata" prefix.
        let synth = synthetic_project();
        overlays_yaml(synth.dir.path(), &[(1, OV1_BASE, OV1_SIZE)]);
        let project = Project::open(synth.dir.path()).unwrap();
        let original = OverworldTable::load(&project).unwrap_err().to_string();
        assert_eq!(original.matches("invalid project metadata").count(), 1);
        let mut cache = SpriteCache::new();
        for _ in 0..2 {
            let e = cache
                .object_sprite(&project, 0, Facing::Down)
                .unwrap_err()
                .to_string();
            assert_eq!(e, original);
        }
    }

    #[test]
    fn a_narc_error_is_remembered_with_its_original_message() {
        let (dir, _) = pt_project(&pt_entries(&[(0, 0), (0xFFFF, 0)]));
        let project = Project::open(dir.path()).unwrap();
        let original = read_sprite_narc(&project).unwrap_err().to_string();
        assert!(original.contains("mmodel.narc"), "{original}");
        let mut cache = SpriteCache::new();
        let first = cache
            .object_sprite(&project, 0, Facing::Down)
            .unwrap_err()
            .to_string();
        // The NARC appears afterwards; the cache does not look again.
        write(
            dir.path(),
            SPRITE_NARC,
            &Narc {
                members: vec![sprite_btx(&[(8, 8)])],
            }
            .write(),
        );
        let second = cache
            .object_sprite(&project, 0, Facing::Down)
            .unwrap_err()
            .to_string();
        assert_eq!(first, original);
        assert_eq!(second, original);
    }

    #[test]
    fn a_broken_sprite_is_none_and_cached() {
        let (dir, _) = pt_project(&pt_entries(&[(0, 0), (0xFFFF, 0)]));
        // A BTX0 header cut off in the middle.
        let mut btx = sprite_btx(&[(8, 8)]);
        btx.truncate(30);
        let narc = Narc { members: vec![btx] };
        write(dir.path(), SPRITE_NARC, &narc.write());
        let project = Project::open(dir.path()).unwrap();
        let mut cache = SpriteCache::new();
        assert!(
            cache
                .object_sprite(&project, 0, Facing::Down)
                .unwrap()
                .is_none()
        );
    }
}
