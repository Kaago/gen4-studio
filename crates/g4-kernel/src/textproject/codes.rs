//! Every `textproject.*` code of a [`Problem`](crate::text::Problem) message, in one place;
//! `docs/formats/text_project.md` lists them with their meaning. A message is
//! `<code>: <sentence>`, and for an entity file `<code>: <path>: <sentence>` with the JSON
//! path of the field.

/// A file that does not match its schema: no JSON, an unknown or misspelt key, a wrong type,
/// a value out of range, a gap in the numbering.
pub(crate) const SCHEMA_CODE: &str = "textproject.schema";
/// An id no table or register knows.
pub(crate) const UNKNOWN_ID_CODE: &str = "textproject.unknown-id";
/// An entity's `id` that is not the id of its number (the import never renames).
pub(crate) const ID_MISMATCH_CODE: &str = "textproject.id-mismatch";
/// A side file `{"file"}` that is missing or unreadable.
pub(crate) const SIDE_FILE_CODE: &str = "textproject.side-file";
/// A `base_sha1` part whose bytes neither the current member nor the base layer has (or the
/// base layer is not there).
pub(crate) const BASE_MISSING_CODE: &str = "textproject.base-missing";
/// A `base_sha1` part whose base layer part has other bytes.
pub(crate) const BASE_MISMATCH_CODE: &str = "textproject.base-mismatch";
/// A map whose `source` is not `binary`.
pub(crate) const MAP_SOURCE_CODE: &str = "textproject.map-source";
/// A number of headers the project cannot take (without the Dynamic Headers patch any
/// change, with it more than the most), a gap or a duplicate number.
pub(crate) const HEADER_COUNT_CODE: &str = "textproject.header-count";
/// A member (or header) without its JSON: the import never deletes, that is 3d.
pub(crate) const MEMBER_WITHOUT_JSON_CODE: &str = "textproject.member-without-json";
/// The header table cannot be located or read.
pub(crate) const HEADER_TABLE_CODE: &str = "textproject.header-table";
/// A key of `data/rotom-quirks.json` that is no plain path of an existing source below
/// `expanded/scripts`.
pub(crate) const QUIRK_KEY_CODE: &str = "textproject.quirk-key";
/// A blob under `data/raw/` outside the range of the blobs or at a path an entity owns.
pub(crate) const RAW_PATH_CODE: &str = "textproject.raw-path";
/// Hint: a folder under `data/` that is no known art; export and import leave it alone.
pub(crate) const UNKNOWN_ART_CODE: &str = "textproject.unknown-art";
/// Hint: `schemas` or `plugins` in the manifest are not empty; phase 4 keeps them unread.
pub(crate) const RESERVED_CODE: &str = "textproject.reserved";
/// Hint: a register entry whose number no longer exists; it stays (frozen).
pub(crate) const ID_DANGLING_CODE: &str = "textproject.id-dangling";
/// Hint: an id left out of the id files for rotom, because rotom already resolves the name to
/// another number (or a kind before has it); in scripts the name means rotom's number.
pub(crate) const ID_SHADOWED_CODE: &str = "textproject.id-shadowed";
/// Hint: binary members without a file under `data/<art>/` could not be compared with the
/// base layer (no layer and no base ROM); they stay as they are.
pub(crate) const BASE_UNCHECKED_CODE: &str = "textproject.base-unchecked";
/// Build step, warning: a member changed since the last build while its JSON did not; the
/// member stays.
pub(crate) const MEMBER_NEWER_CODE: &str = "textproject.member-newer";
/// Build step: the JSON and the member of a unit both changed since the last build; nothing
/// is written.
pub(crate) const CONFLICT_CODE: &str = "textproject.conflict";
/// Build step, full run: a member differs from its JSON and from the base layer; nothing is
/// written.
pub(crate) const UNSYNCED_CODE: &str = "textproject.unsynced";

/// Every code, for the documentation test.
#[cfg(test)]
const ALL: [&str; 20] = [
    SCHEMA_CODE,
    UNKNOWN_ID_CODE,
    ID_MISMATCH_CODE,
    SIDE_FILE_CODE,
    BASE_MISSING_CODE,
    BASE_MISMATCH_CODE,
    MAP_SOURCE_CODE,
    HEADER_COUNT_CODE,
    MEMBER_WITHOUT_JSON_CODE,
    HEADER_TABLE_CODE,
    QUIRK_KEY_CODE,
    RAW_PATH_CODE,
    UNKNOWN_ART_CODE,
    RESERVED_CODE,
    ID_DANGLING_CODE,
    ID_SHADOWED_CODE,
    BASE_UNCHECKED_CODE,
    MEMBER_NEWER_CODE,
    CONFLICT_CODE,
    UNSYNCED_CODE,
];

#[cfg(test)]
mod tests {
    use super::ALL;

    #[test]
    fn the_format_document_lists_every_code() {
        let doc = include_str!("../../../../docs/formats/text_project.md");
        for code in ALL {
            assert!(
                doc.contains(&format!("`{code}`")),
                "{code} is not documented"
            );
        }
    }
}
