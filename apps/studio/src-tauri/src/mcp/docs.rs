//! The texts the server serves without a project: the glossary, `docs/mcp.md` and the format
//! descriptions `docs/formats/*.md`, embedded when the app is built. The glossary and the MCP
//! guide come in the language of the server (`g4_kernel::i18n::locale()`: German for `de`,
//! English otherwise); the format descriptions have an English lead and are one file each.

/// The glossary in German (`glossary.de.md`).
const GLOSSARY_DE: &str = include_str!("glossary.de.md");
/// The glossary in English (`glossary.en.md`).
const GLOSSARY_EN: &str = include_str!("glossary.en.md");

/// The MCP guide in German (`docs/mcp.md`).
const MCP_GUIDE_DE: &str = include_str!("../../../../../docs/mcp.md");
/// The MCP guide in English (`docs/en/mcp.md`).
const MCP_GUIDE_EN: &str = include_str!("../../../../../docs/en/mcp.md");

/// The text for the language of the server: `de` is German, any other locale English.
fn in_locale(de: &'static str, en: &'static str) -> &'static str {
    if g4_kernel::i18n::locale() == "de" {
        de
    } else {
        en
    }
}

/// The glossary of the interface words (`g4://glossary`) in the language of the server.
pub fn glossary() -> &'static str {
    in_locale(GLOSSARY_DE, GLOSSARY_EN)
}

/// The MCP guide (`g4://docs/mcp`) in the language of the server.
pub fn mcp_guide() -> &'static str {
    in_locale(MCP_GUIDE_DE, MCP_GUIDE_EN)
}

/// The format descriptions (`g4://docs/formats/{name}`): file stem and text. A test checks
/// that the table lists every file of `docs/formats`.
pub static FORMATS: &[(&str, &str)] = &[
    (
        "area_data",
        include_str!("../../../../../docs/formats/area_data.md"),
    ),
    ("bdhc", include_str!("../../../../../docs/formats/bdhc.md")),
    (
        "build_state",
        include_str!("../../../../../docs/formats/build_state.md"),
    ),
    (
        "check_limits",
        include_str!("../../../../../docs/formats/check_limits.md"),
    ),
    (
        "dynamic_headers",
        include_str!("../../../../../docs/formats/dynamic_headers.md"),
    ),
    (
        "egg_moves",
        include_str!("../../../../../docs/formats/egg_moves.md"),
    ),
    (
        "encounter",
        include_str!("../../../../../docs/formats/encounter.md"),
    ),
    (
        "evolution",
        include_str!("../../../../../docs/formats/evolution.md"),
    ),
    (
        "flags_vars",
        include_str!("../../../../../docs/formats/flags_vars.md"),
    ),
    ("item", include_str!("../../../../../docs/formats/item.md")),
    (
        "learnset",
        include_str!("../../../../../docs/formats/learnset.md"),
    ),
    ("map", include_str!("../../../../../docs/formats/map.md")),
    (
        "map_header",
        include_str!("../../../../../docs/formats/map_header.md"),
    ),
    (
        "matrix",
        include_str!("../../../../../docs/formats/matrix.md"),
    ),
    (
        "model_import",
        include_str!("../../../../../docs/formats/model_import.md"),
    ),
    ("move", include_str!("../../../../../docs/formats/move.md")),
    ("narc", include_str!("../../../../../docs/formats/narc.md")),
    (
        "ncgr_nclr",
        include_str!("../../../../../docs/formats/ncgr_nclr.md"),
    ),
    (
        "nsbmd",
        include_str!("../../../../../docs/formats/nsbmd.md"),
    ),
    (
        "nsbmd_header",
        include_str!("../../../../../docs/formats/nsbmd_header.md"),
    ),
    (
        "nsbtx",
        include_str!("../../../../../docs/formats/nsbtx.md"),
    ),
    (
        "overworld_table",
        include_str!("../../../../../docs/formats/overworld_table.md"),
    ),
    (
        "personal",
        include_str!("../../../../../docs/formats/personal.md"),
    ),
    (
        "project_base",
        include_str!("../../../../../docs/formats/project_base.md"),
    ),
    (
        "project_git",
        include_str!("../../../../../docs/formats/project_git.md"),
    ),
    (
        "recipes",
        include_str!("../../../../../docs/formats/recipes.md"),
    ),
    (
        "rotom_project",
        include_str!("../../../../../docs/formats/rotom_project.md"),
    ),
    (
        "script_file",
        include_str!("../../../../../docs/formats/script_file.md"),
    ),
    (
        "stories",
        include_str!("../../../../../docs/formats/stories.md"),
    ),
    (
        "templates",
        include_str!("../../../../../docs/formats/templates.md"),
    ),
    (
        "terrain",
        include_str!("../../../../../docs/formats/terrain.md"),
    ),
    (
        "text_archive",
        include_str!("../../../../../docs/formats/text_archive.md"),
    ),
    (
        "text_project",
        include_str!("../../../../../docs/formats/text_project.md"),
    ),
    (
        "tileset",
        include_str!("../../../../../docs/formats/tileset.md"),
    ),
    ("tmj", include_str!("../../../../../docs/formats/tmj.md")),
    (
        "trainer",
        include_str!("../../../../../docs/formats/trainer.md"),
    ),
    (
        "trainer_scripts",
        include_str!("../../../../../docs/formats/trainer_scripts.md"),
    ),
    (
        "trainer_text",
        include_str!("../../../../../docs/formats/trainer_text.md"),
    ),
    (
        "world_map",
        include_str!("../../../../../docs/formats/world_map.md"),
    ),
    (
        "xdelta_patch",
        include_str!("../../../../../docs/formats/xdelta_patch.md"),
    ),
];

/// The text of the format description `name`.
pub fn format(name: &str) -> Option<&'static str> {
    FORMATS
        .iter()
        .find(|(stem, _)| *stem == name)
        .map(|(_, text)| *text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_covers_every_format_file() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../docs/formats");
        let mut files: Vec<String> = std::fs::read_dir(&dir)
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| p.extension().is_some_and(|e| e == "md"))
            .map(|p| p.file_stem().unwrap().to_string_lossy().into_owned())
            .collect();
        files.sort();
        let table: Vec<&str> = FORMATS.iter().map(|(stem, _)| *stem).collect();
        assert_eq!(table, files, "docs/formats changed: update FORMATS");
        for (stem, text) in FORMATS {
            assert!(text.starts_with('#'), "{stem}");
        }
    }
}
