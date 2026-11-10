//! The `.gitignore` and `.gitattributes` of a project, from the templates under
//! `metadata/git/` (one `.gitignore` per layout, one `.gitattributes` for both). The whole
//! template stands in one block between [`BLOCK_START`] and [`BLOCK_END`]: a file without the
//! block gets it at its end (the user's lines stay byte for byte, the later line wins in both
//! files), a file with it gets it replaced and moved to the end unless it is the same and there
//! already, and a block without its end is refused. Format: `docs/formats/project_git.md`.

use std::path::Path;

use serde::Serialize;

use crate::error::{KernelError, io_at};
use crate::fsutil::write_atomic;
use crate::project::{Project, SourceMode};

const DSPRE_GITIGNORE: &str = include_str!("../../metadata/git/dspre.gitignore");
const TEXT_GITIGNORE: &str = include_str!("../../metadata/git/text.gitignore");
const GITATTRIBUTES: &str = include_str!("../../metadata/git/gitattributes");

/// Where the DSPRE template lists the archives the build packs.
const PACKED_ARCHIVES: &str = "{{packed_archives}}";
/// Where the text template lists the DSPRE part.
const DSPRE_PART_SLOT: &str = "{{dspre_part}}";

const GITIGNORE: &str = ".gitignore";
const GITATTRIBUTES_FILE: &str = ".gitattributes";

/// The first line of the block Gen4 Studio keeps in `.gitignore` and `.gitattributes`.
pub const BLOCK_START: &str = "# >>> Gen4 Studio (dieser Block wird von Gen4 Studio gepflegt) >>>";
/// The last line of the block.
pub const BLOCK_END: &str = "# <<< Gen4 Studio <<<";

/// The refusal for a block whose start has no end; the file name stands before it
/// (`.gitignore: …`).
pub fn broken_block() -> String {
    t!("git.broken_block").into_owned()
}

/// Which `.gitignore` template a project gets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Layout {
    /// The DSPRE folder is the source; the archives the build packs from `unpacked/` are
    /// ignored.
    Dspre,
    /// The text project is the source; the DSPRE part (generated from the base ROM and the
    /// text project) is ignored, `expanded/` stays.
    TextProject,
}

/// What `build::extract_rom` writes at the root, ignored in a text project (with `unpacked`):
/// every name ds-rom's `Rom::save` can write with the configuration `Rom::extract` makes (the
/// overlay folders and `multiboot_signature.yaml` only when the ROM has them). A folder ends
/// with `/`. Checked against an extracted ROM by a gated test.
pub const DSPRE_PART: &[&str] = &[
    "arm7/",
    "arm7_overlays/",
    "arm9/",
    "arm9_overlays/",
    "banner/",
    "files/",
    "config.yaml",
    "header.yaml",
    "header_logo.png",
    "multiboot_signature.yaml",
    "path_order.txt",
];

/// What writing the block did to one file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TemplateAction {
    /// The file did not exist; it holds the block alone.
    Created,
    /// The file had no block; the block stands at its end.
    Appended,
    /// The file's block differed or was not at the end; it was taken out and the block
    /// appended.
    Updated,
    /// The file's block was the same; nothing was written.
    Unchanged,
}

/// One file `write_templates` looked at, relative to the project root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TemplateFile {
    pub path: String,
    pub action: TemplateAction,
}

/// What `write_templates` did: the layout of the `.gitignore`, both files (`.gitignore`
/// first), and how many packed archives the `.gitignore` names (DSPRE layout; 0 in a text
/// project).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TemplateReport {
    pub layout: Layout,
    pub files: Vec<TemplateFile>,
    pub ignored_archives: u32,
}

/// The layout of the project's `.gitignore`: a text project of any schema (also one Gen4
/// Studio only reads) is [`Layout::TextProject`], a DSPRE folder and a manifest that does not
/// read are [`Layout::Dspre`].
pub fn layout_of(project: &Project) -> Layout {
    match project.source_mode() {
        SourceMode::TextProject | SourceMode::Older { .. } | SourceMode::Newer { .. } => {
            Layout::TextProject
        }
        SourceMode::Dspre | SourceMode::Broken { .. } => Layout::Dspre,
    }
}

/// The `.gitignore` block of the project's layout, with its markers.
pub fn gitignore_block(project: &Project) -> Result<String, KernelError> {
    gitignore(project).map(|(block, _)| block)
}

/// The `.gitattributes` block (the same for both layouts), with its markers.
pub fn gitattributes_block() -> String {
    with_markers(GITATTRIBUTES)
}

/// `existing` with `block` merged in; the block always ends up at the end of the file, so its
/// lines win over the user's in both files. Without a file (`None`) the block alone
/// ([`TemplateAction::Created`]). A file without the block gets it at its end, after one blank
/// line, its own text byte for byte before it ([`TemplateAction::Appended`]). A file with the
/// block (a line equal to [`BLOCK_START`] up to the next equal to [`BLOCK_END`]) is returned as
/// it is when it has exactly one, that block is the same and only blank space follows it
/// ([`TemplateAction::Unchanged`]; a block that differs only in `\r\n` line ends counts as the
/// same); else every block is taken out (a pasted second one too; the user's lines before,
/// between and after them stay byte for byte and in order) and the block appended like to a
/// file without one ([`TemplateAction::Updated`]). A start without an end, or a start inside an
/// open block, is `Err` with [`broken_block`].
pub fn merge_block(
    existing: Option<&str>,
    block: &str,
) -> Result<(String, TemplateAction), String> {
    let Some(text) = existing else {
        return Ok((block.to_owned(), TemplateAction::Created));
    };
    // Every complete block out, the text around them kept in order.
    let mut rest = String::with_capacity(text.len());
    let mut blocks = Vec::new();
    let mut at = 0;
    while let Some(start) = find_line(text, at, BLOCK_START) {
        let inside = line_end(text, start);
        let end = find_line(text, inside, BLOCK_END).ok_or_else(broken_block)?;
        // A start inside an open block is broken too.
        if find_line(text, inside, BLOCK_START).is_some_and(|next| next < end) {
            return Err(broken_block());
        }
        let end = line_end(text, end);
        rest.push_str(&text[at..start]);
        blocks.push(&text[start..end]);
        at = end;
    }
    let after = &text[at..];
    rest.push_str(after);
    match blocks.as_slice() {
        [] => Ok((append(text, block), TemplateAction::Appended)),
        [current]
            if (*current == block || current.replace("\r\n", "\n") == block)
                && after.trim().is_empty() =>
        {
            Ok((text.to_owned(), TemplateAction::Unchanged))
        }
        _ => Ok((append(&rest, block), TemplateAction::Updated)),
    }
}

/// The offset after the line end of the line starting at `start` (the text's end for its last
/// line).
fn line_end(text: &str, start: usize) -> usize {
    text[start..]
        .find('\n')
        .map_or(text.len(), |n| start + n + 1)
}

/// `text` byte for byte, then (unless it is empty) a line end if it lacks one and one blank
/// line if it does not end in one, then `block`.
fn append(text: &str, block: &str) -> String {
    let mut merged = text.to_owned();
    if !merged.is_empty() {
        if !merged.ends_with('\n') {
            merged.push('\n');
        }
        if !merged.ends_with("\n\n") {
            merged.push('\n');
        }
    }
    merged.push_str(block);
    merged
}

/// Writes the block of the project's layout into `.gitignore` and the `.gitattributes` block
/// into `.gitattributes` ([`merge_block`]), each atomically. Both files are merged before
/// either is written: a broken block in one ([`KernelError::Refused`] with the file name before
/// [`broken_block`]) leaves both as they are. An unchanged file is not written.
pub fn write_templates(project: &Project) -> Result<TemplateReport, KernelError> {
    let (ignore, ignored_archives) = gitignore(project)?;
    let mut merged = Vec::new();
    for (name, block) in [
        (GITIGNORE, ignore),
        (GITATTRIBUTES_FILE, gitattributes_block()),
    ] {
        let path = project.root.join(name);
        let existing =
            match std::fs::read(&path) {
                Ok(bytes) => Some(String::from_utf8(bytes).map_err(|_| {
                    KernelError::Refused(t!("git.not_utf8", name = name).into_owned())
                })?),
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => None,
                Err(err) => return Err(io_at(&path)(err)),
            };
        let (text, action) = merge_block(existing.as_deref(), &block)
            .map_err(|message| KernelError::Refused(format!("{name}: {message}")))?;
        merged.push((name, path, text, action));
    }
    let mut files = Vec::new();
    for (name, path, text, action) in merged {
        if action != TemplateAction::Unchanged {
            write_atomic(&project.root, &path, text.as_bytes())?;
        }
        files.push(TemplateFile {
            path: name.to_owned(),
            action,
        });
    }
    Ok(TemplateReport {
        layout: layout_of(project),
        files,
        ignored_archives,
    })
}

/// The `.gitignore` block and how many packed archives it names.
fn gitignore(project: &Project) -> Result<(String, u32), KernelError> {
    match layout_of(project) {
        Layout::Dspre => {
            let archives = packed_archives(project)?;
            let lines: String = archives.iter().map(|a| format!("/{a}\n")).collect();
            let count = u32::try_from(archives.len()).unwrap_or(u32::MAX);
            Ok((fill(DSPRE_GITIGNORE, PACKED_ARCHIVES, &lines), count))
        }
        Layout::TextProject => {
            let mut lines: String = DSPRE_PART.iter().map(|n| format!("/{n}\n")).collect();
            lines.push_str("/unpacked/\n");
            Ok((fill(TEXT_GITIGNORE, DSPRE_PART_SLOT, &lines), 0))
        }
    }
}

/// The packed path (`/`-separated, sorted) of every pack entry whose `unpacked/<folder>/`
/// exists: the build packs exactly these, so they are generated. An archive not unpacked is
/// still the source and stays versioned.
fn packed_archives(project: &Project) -> Result<Vec<String>, KernelError> {
    let unpacked = project.root.join("unpacked");
    let mut archives: Vec<String> = crate::build::pack_entries(project)?
        .into_iter()
        .filter(|(name, _)| unpacked.join(name).is_dir())
        .map(|(_, packed)| slash(&packed))
        .collect();
    archives.sort();
    archives.dedup();
    Ok(archives)
}

/// `path` with `/` between its parts.
fn slash(path: &Path) -> String {
    let parts: Vec<_> = path.iter().map(|p| p.to_string_lossy()).collect();
    parts.join("/")
}

/// `template` with its placeholder line replaced by `lines` (each ending in `\n`; none drops
/// the line), between the markers.
fn fill(template: &str, placeholder: &str, lines: &str) -> String {
    with_markers(&template.replace(&format!("{placeholder}\n"), lines))
}

fn with_markers(body: &str) -> String {
    format!("{BLOCK_START}\n{body}{BLOCK_END}\n")
}

/// The byte offset of the first line at or after `from` (a line start) that is `line`, with or
/// without a `\r` before its line end.
fn find_line(text: &str, from: usize, line: &str) -> Option<usize> {
    let mut at = from;
    for raw in text[from..].split_inclusive('\n') {
        let content = raw.strip_suffix('\n').unwrap_or(raw);
        let content = content.strip_suffix('\r').unwrap_or(content);
        if content == line {
            return Some(at);
        }
        at += raw.len();
    }
    None
}

#[cfg(all(test, feature = "test-support"))]
mod tests {
    #![allow(clippy::unwrap_used)]

    use std::collections::BTreeSet;
    use std::path::Path;

    use super::*;
    use crate::git::repo::{GitRepo, PathState};
    use crate::project::AssetKind;
    use crate::test_support::{GitFixture, TextFixture, synthetic_project};

    fn block(body: &str) -> String {
        format!("{BLOCK_START}\n{body}{BLOCK_END}\n")
    }

    fn read(root: &Path, rel: &str) -> String {
        std::fs::read_to_string(root.join(rel)).unwrap()
    }

    fn actions(report: &TemplateReport) -> Vec<(&str, TemplateAction)> {
        report
            .files
            .iter()
            .map(|f| (f.path.as_str(), f.action))
            .collect()
    }

    /// Unpacks every archive of the project that can be: what "Git einrichten…" does first in
    /// the DSPRE layout. The header container of the Dynamic Headers patch is store-only (its
    /// folder appears with the patch).
    fn unpack_every_kind(project: &Project) {
        let kinds: Vec<AssetKind> = project.archives.kinds().collect();
        for kind in kinds {
            if kind == AssetKind::DynamicHeaders {
                continue;
            }
            if let Some(packed) = project.archives.packed(kind)
                && project.root.join(packed).is_file()
            {
                project.ensure_unpacked(kind).unwrap();
            }
        }
    }

    /// The untracked paths of the project's repository, ignored ones left out.
    fn untracked(root: &Path) -> BTreeSet<String> {
        let repo = GitRepo::discover(root).unwrap().unwrap();
        repo.status()
            .unwrap()
            .paths
            .into_iter()
            .filter(|p| p.state == PathState::Untracked)
            .map(|p| p.path)
            .collect()
    }

    /// Every file below `rel` (relative to `root`, `/`-separated).
    fn files_below(root: &Path, rel: &str) -> Vec<String> {
        let mut out = Vec::new();
        let mut dirs = vec![root.join(rel)];
        while let Some(dir) = dirs.pop() {
            for entry in std::fs::read_dir(&dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    dirs.push(path);
                } else {
                    let rel = path.strip_prefix(root).unwrap();
                    let parts: Vec<_> = rel.iter().map(|p| p.to_str().unwrap()).collect();
                    out.push(parts.join("/"));
                }
            }
        }
        out
    }

    /// The `/`-separated packed paths of the pack entries whose `unpacked/` folder exists.
    fn packed_with_folder(project: &Project) -> Vec<String> {
        crate::build::pack_entries(project)
            .unwrap()
            .into_iter()
            .filter(|(name, _)| project.root.join("unpacked").join(name).is_dir())
            .map(|(_, packed)| {
                let parts: Vec<_> = packed.iter().map(|p| p.to_str().unwrap()).collect();
                parts.join("/")
            })
            .collect()
    }

    #[test]
    fn merge_creates_appends_updates_and_keeps_user_lines() {
        let ours = block("*.nds\n");
        assert_eq!(
            merge_block(None, &ours).unwrap(),
            (ours.clone(), TemplateAction::Created)
        );

        // The user's text stays byte for byte, one blank line, then the block at the end.
        for user in ["notes/\n*.tmp", "notes/\n*.tmp\n"] {
            let (merged, action) = merge_block(Some(user), &ours).unwrap();
            assert_eq!(action, TemplateAction::Appended);
            assert_eq!(merged, format!("notes/\n*.tmp\n\n{ours}"), "{user:?}");
            assert!(merged.starts_with(user));
        }
        // An empty file, or one ending in a blank line, gets no second blank line.
        assert_eq!(
            merge_block(Some(""), &ours).unwrap(),
            (ours.clone(), TemplateAction::Appended)
        );
        assert_eq!(
            merge_block(Some("notes/\n\n"), &ours).unwrap().0,
            format!("notes/\n\n{ours}")
        );

        // Another block replaces ours at the end.
        let (merged, _) = merge_block(Some("notes/\n"), &ours).unwrap();
        let theirs = block("*.nds\n*.srl\n");
        let (updated, action) = merge_block(Some(&merged), &theirs).unwrap();
        assert_eq!(action, TemplateAction::Updated);
        assert_eq!(updated, format!("notes/\n\n{theirs}"));
        // A user line after an old block: the block moves to the end, the user's lines stay
        // byte for byte and in order before it; the same block moves too.
        let around = format!("notes/\n\n{ours}mine/\n");
        for new in [&theirs, &ours] {
            assert_eq!(
                merge_block(Some(&around), new).unwrap(),
                (format!("notes/\n\nmine/\n\n{new}"), TemplateAction::Updated)
            );
        }
        // Blank space after the block still counts as the end.
        let trailing = format!("{updated}\n");
        assert_eq!(
            merge_block(Some(&trailing), &theirs).unwrap(),
            (trailing.clone(), TemplateAction::Unchanged)
        );

        // The same block: nothing to write, also with Windows line ends.
        assert_eq!(
            merge_block(Some(&updated), &theirs).unwrap(),
            (updated.clone(), TemplateAction::Unchanged)
        );
        let crlf = updated.replace('\n', "\r\n");
        assert_eq!(
            merge_block(Some(&crlf), &theirs).unwrap(),
            (crlf.clone(), TemplateAction::Unchanged)
        );

        // A start without an end.
        let broken = format!("notes/\n{BLOCK_START}\n*.nds\n");
        assert_eq!(merge_block(Some(&broken), &ours), Err(broken_block()));
    }

    #[test]
    fn an_existing_file_keeps_its_lines_and_gets_the_block_once() {
        let s = synthetic_project();
        let root = s.dir.path();
        std::fs::write(root.join(".gitattributes"), "* text=auto\n").unwrap();
        std::fs::write(root.join(".gitignore"), "notes/\n").unwrap();
        let project = Project::open(root).unwrap();

        let first = write_templates(&project).unwrap();
        assert_eq!(first.layout, Layout::Dspre);
        assert_eq!(
            actions(&first),
            [
                (".gitignore", TemplateAction::Appended),
                (".gitattributes", TemplateAction::Appended),
            ]
        );
        let ignore = read(root, ".gitignore");
        let attributes = read(root, ".gitattributes");
        assert!(ignore.starts_with("notes/\n\n"), "{ignore}");
        assert!(attributes.starts_with("* text=auto\n\n"), "{attributes}");
        assert_eq!(
            attributes,
            format!("* text=auto\n\n{}", gitattributes_block())
        );
        for text in [&ignore, &attributes] {
            assert_eq!(text.matches(BLOCK_START).count(), 1, "{text}");
            assert!(text.ends_with(&format!("{BLOCK_END}\n")), "{text}");
        }

        let second = write_templates(&project).unwrap();
        assert_eq!(
            actions(&second),
            [
                (".gitignore", TemplateAction::Unchanged),
                (".gitattributes", TemplateAction::Unchanged),
            ]
        );
        assert_eq!(read(root, ".gitignore"), ignore);
        assert_eq!(read(root, ".gitattributes"), attributes);
    }

    #[test]
    fn a_broken_block_is_refused_and_nothing_written() {
        let s = synthetic_project();
        let root = s.dir.path();
        let broken = format!("notes/\n{BLOCK_START}\n*.nds\n");
        std::fs::write(root.join(".gitignore"), &broken).unwrap();
        let project = Project::open(root).unwrap();

        let err = write_templates(&project).unwrap_err();
        let KernelError::Refused(message) = err else {
            panic!("not a refusal: {err:?}");
        };
        assert_eq!(message, format!(".gitignore: {}", broken_block()));
        assert_eq!(read(root, ".gitignore"), broken);
        assert!(!root.join(".gitattributes").exists());

        // A broken `.gitattributes` beside a `.gitignore` that would be written: neither is.
        std::fs::write(root.join(".gitignore"), "notes/\n").unwrap();
        std::fs::write(root.join(".gitattributes"), &broken).unwrap();
        let err = write_templates(&project).unwrap_err();
        let KernelError::Refused(message) = err else {
            panic!("not a refusal: {err:?}");
        };
        assert_eq!(message, format!(".gitattributes: {}", broken_block()));
        assert_eq!(read(root, ".gitignore"), "notes/\n");
        assert_eq!(read(root, ".gitattributes"), broken);

        // Without any `.gitignore`: none is created.
        std::fs::remove_file(root.join(".gitignore")).unwrap();
        write_templates(&project).unwrap_err();
        assert!(!root.join(".gitignore").exists());
    }

    #[test]
    fn every_block_is_taken_out_and_one_settles_at_the_end() {
        let ours = block("*.nds\n");
        // Two identical blocks (pasted twice) with the user's lines around them.
        let twice = format!("a/\n{ours}b/\n{ours}c/\n");
        let (merged, action) = merge_block(Some(&twice), &ours).unwrap();
        assert_eq!(action, TemplateAction::Updated);
        assert_eq!(merged, format!("a/\nb/\nc/\n\n{ours}"));
        assert_eq!(merged.matches(BLOCK_START).count(), 1);
        assert_eq!(
            merge_block(Some(&merged), &ours).unwrap(),
            (merged.clone(), TemplateAction::Unchanged)
        );
        // Two blocks, the last one at the end and the same: still one too many.
        let tail = format!("a/\n{ours}\n{ours}");
        assert_eq!(
            merge_block(Some(&tail), &ours).unwrap(),
            (format!("a/\n\n{ours}"), TemplateAction::Updated)
        );
    }

    #[test]
    fn a_start_inside_an_open_block_is_broken() {
        let ours = block("*.nds\n");
        let nested = format!("{BLOCK_START}\nmine/\n{BLOCK_START}\n*.nds\n{BLOCK_END}\n");
        assert_eq!(merge_block(Some(&nested), &ours), Err(broken_block()));
        // A complete block, then a start without an end.
        let open = format!("{ours}x/\n{BLOCK_START}\n");
        assert_eq!(merge_block(Some(&open), &ours), Err(broken_block()));
    }

    #[test]
    fn dspre_block_lists_packed_archives_only_with_their_unpacked_folder() {
        let s = synthetic_project();
        let root = s.dir.path();
        let project = Project::open(root).unwrap();
        // Nothing unpacked yet: no archive line.
        let report = write_templates(&project).unwrap();
        assert_eq!(report.ignored_archives, 0);

        unpack_every_kind(&project);
        let lines = packed_with_folder(&project);
        assert!(lines.len() >= 10, "{lines:?}");
        let report = write_templates(&project).unwrap();
        assert_eq!(report.ignored_archives as usize, lines.len());
        assert_eq!(report.files[0].action, TemplateAction::Updated);
        let ignore = read(root, ".gitignore");
        for line in &lines {
            assert!(
                ignore.lines().any(|l| l == format!("/{line}")),
                "{line}:\n{ignore}"
            );
        }
        let matrix = "/files/fielddata/mapmatrix/map_matrix.narc";
        assert!(ignore.lines().any(|l| l == matrix), "{ignore}");
        // The lines stand sorted, in their group.
        let block = gitignore_block(&project).unwrap();
        let listed: Vec<&str> = block.lines().filter(|l| l.starts_with("/files/")).collect();
        let mut sorted = listed.clone();
        sorted.sort();
        assert_eq!(listed, sorted);
        assert!(!block.contains("{{"), "{block}");

        std::fs::remove_dir_all(project.unpacked_dir(AssetKind::Matrix)).unwrap();
        let report = write_templates(&project).unwrap();
        assert_eq!(report.ignored_archives as usize, lines.len() - 1);
        assert_eq!(report.files[0].action, TemplateAction::Updated);
        let ignore = read(root, ".gitignore");
        assert!(!ignore.lines().any(|l| l == matrix), "{ignore}");
    }

    #[test]
    fn the_templates_keep_releases_json_versioned() {
        for (fx, layout) in [
            (TextFixture::dspre(), Layout::Dspre),
            (TextFixture::exported(), Layout::TextProject),
        ] {
            let root = fx.root();
            let project = fx.project();
            assert_eq!(layout_of(&project), layout);
            write_templates(&project).unwrap();
            fx.write(".g4/releases.json", b"{}");
            fx.write(".g4/cache/x", b"x");
            GitFixture::init(root);

            let untracked = untracked(root);
            assert!(untracked.contains(".g4/releases.json"), "{untracked:?}");
            assert!(!untracked.contains(".g4/cache/x"), "{untracked:?}");
            assert!(!untracked.contains(".g4/cache/"), "{untracked:?}");
        }
    }

    #[test]
    fn dspre_template_leaves_no_generated_archive_untracked() {
        let fx = TextFixture::dspre();
        let root = fx.root();
        let project = fx.project();
        unpack_every_kind(&project);
        // What the build and the Studio leave behind.
        fx.write(".g4/build/state.json", b"{}");
        fx.write(".g4/tmp/x.saving", b"x");
        fx.write("out.nds", b"rom");
        fx.write("out.nds.sav", b"sav");
        fx.write(".rotom/cache/x", b"x");
        fx.write(".rotom/command_database/.g4-ids-trainers.json", b"{}");
        fx.write(".rotom/command_database/commands.json", b"{}");

        let report = write_templates(&project).unwrap();
        assert_eq!(report.layout, Layout::Dspre);
        GitFixture::init(root);
        let untracked = untracked(root);

        for packed in packed_with_folder(&project) {
            assert!(!untracked.contains(&packed), "{packed} is untracked");
        }
        // Without its folder an archive is the source and stays: the vanilla header container.
        assert!(!root.join("unpacked/dynamicHeaders").exists());
        assert!(untracked.contains("files/debug/cb_edit/d_test.narc"));
        let in_g4: Vec<_> = untracked.iter().filter(|p| p.starts_with(".g4/")).collect();
        assert_eq!(in_g4, [".g4/base.json"]);
        for kept in [
            ".gitignore",
            ".gitattributes",
            "header.yaml",
            "arm9/arm9.bin",
            ".rotom/command_database/commands.json",
        ] {
            assert!(untracked.contains(kept), "{kept} is not untracked");
        }
        for ignored in [
            "out.nds",
            "out.nds.sav",
            ".rotom/cache/x",
            ".rotom/command_database/.g4-ids-trainers.json",
        ] {
            assert!(!untracked.contains(ignored), "{ignored} is untracked");
        }
        let unpacked = files_below(root, "unpacked");
        assert!(unpacked.len() > 100, "{}", unpacked.len());
        for file in unpacked {
            assert!(untracked.contains(&file), "{file} is not untracked");
        }
    }

    #[test]
    fn text_template_ignores_the_dspre_part_and_keeps_the_sources() {
        let fx = TextFixture::exported();
        let root = fx.root();
        let project = fx.project();
        assert_eq!(layout_of(&project), Layout::TextProject);

        // A generated id file for rotom: ignored, its folder's other files are sources.
        fx.write(".rotom/command_database/.g4-ids-trainers.json", b"{}");
        let report = write_templates(&project).unwrap();
        assert_eq!(report.layout, Layout::TextProject);
        assert_eq!(report.ignored_archives, 0);
        GitFixture::init(root);
        let untracked = untracked(root);

        let sources: Vec<String> = crate::textproject::sources(&project)
            .unwrap()
            .iter()
            .map(|p| p.to_str().unwrap().replace('\\', "/"))
            .collect();
        assert!(sources.len() > 50, "{}", sources.len());
        for source in &sources {
            assert!(untracked.contains(source), "{source} is not untracked");
        }
        assert!(
            sources
                .iter()
                .any(|p| p.starts_with(".rotom/command_database/")),
            "{sources:?}"
        );
        assert!(!untracked.contains(".rotom/command_database/.g4-ids-trainers.json"));
        for path in &untracked {
            let expected = sources.contains(path)
                || path == ".gitignore"
                || path == ".gitattributes"
                || path.starts_with("expanded/");
            assert!(expected, "{path} is untracked");
        }
    }

    #[test]
    fn a_changed_layout_replaces_the_block() {
        let fx = TextFixture::dspre();
        let root = fx.root();
        let report = write_templates(&fx.project()).unwrap();
        assert_eq!(report.layout, Layout::Dspre);
        assert!(report.ignored_archives > 0);
        assert!(read(root, ".gitignore").contains(".narc\n"));

        crate::textproject::export(
            &fx.project(),
            &fx.base(),
            &crate::textproject::ExportOptions {
                adopt_scripts: true,
                force: false,
            },
        )
        .unwrap();
        let report = write_templates(&fx.project()).unwrap();
        assert_eq!(report.layout, Layout::TextProject);
        assert_eq!(report.ignored_archives, 0);
        assert_eq!(
            actions(&report),
            [
                (".gitignore", TemplateAction::Updated),
                (".gitattributes", TemplateAction::Unchanged),
            ]
        );
        let ignore = read(root, ".gitignore");
        assert_eq!(ignore, gitignore_block(&fx.project()).unwrap());
        assert!(ignore.lines().any(|l| l == "/unpacked/"), "{ignore}");
        assert!(!ignore.contains(".narc"), "{ignore}");
    }

    /// Gated: ds-rom cannot extract the synthetic ROM (it has no file system), so this
    /// extracts the vanilla base ROM (`G4_TEST_PROJECT`'s sibling `.nds`, only read).
    #[test]
    fn the_dspre_part_matches_what_extract_writes() {
        let Some(rom) = crate::test_support::vanilla_base_rom() else {
            eprintln!("skipped: no vanilla base ROM (G4_TEST_PROJECT)");
            return;
        };
        let tmp = tempfile::tempdir().unwrap();
        let dest = tmp.path().join("project");
        crate::build::extract_rom(&rom, &dest).unwrap();
        let mut names = Vec::new();
        for entry in std::fs::read_dir(&dest).unwrap() {
            let entry = entry.unwrap();
            let mut name = entry.file_name().to_str().unwrap().to_owned();
            if entry.file_type().unwrap().is_dir() {
                name.push('/');
            }
            names.push(name);
        }
        names.sort();
        eprintln!("extracted root: {names:?}");
        assert!(names.len() >= 5, "{names:?}");
        for name in &names {
            assert!(
                DSPRE_PART.contains(&name.as_str()),
                "{name} is not in DSPRE_PART"
            );
        }
        assert!(!DSPRE_PART.contains(&"unpacked/"));
        assert!(!DSPRE_PART.contains(&"expanded/"));
    }

    #[test]
    fn the_text_block_lists_the_dspre_part_then_unpacked() {
        let fx = TextFixture::exported();
        let block = gitignore_block(&fx.project()).unwrap();
        let mut want: Vec<String> = DSPRE_PART.iter().map(|n| format!("/{n}")).collect();
        want.push("/unpacked/".to_owned());
        let start = block
            .lines()
            .position(|l| l.starts_with("# Spieldateien:"))
            .unwrap();
        let listed: Vec<&str> = block
            .lines()
            .skip(start + 1)
            .take_while(|l| !l.is_empty())
            .collect();
        assert_eq!(listed, want);
        assert!(block.starts_with(&format!("{BLOCK_START}\n# Gen4 Studio, Textprojekt.")));
        assert!(block.ends_with(&format!("Thumbs.db\n{BLOCK_END}\n")));
        assert!(!block.contains("{{"), "{block}");
    }

    #[test]
    fn attributes_mark_everything_binary_and_text_sources_lf() {
        let block = gitattributes_block();
        assert!(block.starts_with(&format!("{BLOCK_START}\n")));
        assert!(block.ends_with(&format!("{BLOCK_END}\n")));
        let rules: Vec<&str> = block
            .lines()
            .filter(|l| !l.starts_with('#') && !l.is_empty())
            .collect();
        assert_eq!(
            rules,
            [
                "* -text",
                "*.json text eol=lf",
                "*.rotom text eol=lf",
                "*.yaml text eol=lf",
                "*.yml text eol=lf",
                "*.toml text eol=lf",
                "*.md text eol=lf",
                "*.png binary",
                "*.glb binary",
            ]
        );
        assert!(block.contains(
            "# Beifiles der Phase 6 (Modelle, Bilder): ohne Diff und Merge; LFS erst ab 1 MB je \
             Datei, etwa: *.glb filter=lfs"
        ));
    }

    #[test]
    fn the_project_attributes_mark_png_and_glb_binary() {
        let block = gitattributes_block();
        let lines: Vec<&str> = block.lines().collect();
        for rule in ["*.png binary", "*.glb binary"] {
            let at = lines.iter().position(|l| *l == rule);
            // After `* -text`: the later rule wins in Git.
            assert!(at > lines.iter().position(|l| *l == "* -text"), "{rule}");
        }
        assert!(!block.contains("Platz für große Binärquellen"));
        // The repository marks them the same way.
        let repo = include_str!("../../../../.gitattributes");
        assert!(repo.lines().any(|l| l == "*.png binary"));
        assert!(repo.lines().any(|l| l == "*.glb binary"));
    }
}
