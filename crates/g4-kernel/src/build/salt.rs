//! Tool salts: which version of each external tool a build step ran with. `build.rs` reads
//! the workspace `Cargo.lock` and hands the kernel one line per tool (`"<name> <version>
//! <source>"`, or `"unknown"` when the lock file or the package is missing). A step's salt is
//! that line plus the step's own revision, so a kernel update that keeps the tools keeps the
//! build state, and a new tool (or a new step revision) builds that step fully once.

// The step runner (phase 4.2, task 2) is the first caller outside the tests.
#![cfg_attr(
    not(test),
    expect(dead_code, reason = "the build graph's steps use the salts")
)]

/// Format version of `.g4/build/state.json`. A file with another version reads as empty.
pub(crate) const STATE_VERSION: u32 = 1;

/// The external tools whose version decides whether a step's recorded result still holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Tool {
    DsRom,
    Rotom,
    Chatot,
    Uxie,
}

/// The `Cargo.lock` line of `tool`: `"<name> <version> <source>"`, or `"unknown"`.
pub(crate) fn lock(tool: Tool) -> &'static str {
    match tool {
        Tool::DsRom => env!("G4_LOCK_DS_ROM"),
        Tool::Rotom => env!("G4_LOCK_ROTOM"),
        Tool::Chatot => env!("G4_LOCK_CHATOT"),
        Tool::Uxie => env!("G4_LOCK_UXIE"),
    }
}

/// The salt of a step that runs `tool` in revision `rev` of its own code: the lock line and
/// `#<rev>`.
pub(crate) fn step_salt(tool: Tool, rev: u32) -> String {
    format!("{}#{rev}", lock(tool))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `"<name> <version> <source>"` of `name` in the workspace lock file, read here
    /// independently of `build.rs`.
    fn lock_line(lock: &str, name: &str) -> Option<String> {
        for section in lock.split("[[package]]").skip(1) {
            let field = |key: &str| {
                section.lines().find_map(|l| {
                    l.strip_prefix(key)
                        .and_then(|rest| rest.trim().strip_prefix('='))
                        .map(|v| v.trim().trim_matches('"').to_owned())
                })
            };
            if field("name ").as_deref() == Some(name) {
                return Some(format!(
                    "{name} {} {}",
                    field("version ")?,
                    field("source ").unwrap_or_default()
                ));
            }
        }
        None
    }

    #[test]
    fn salts_match_the_lockfile() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Cargo.lock");
        let text = std::fs::read_to_string(path).unwrap();
        for (tool, name) in [
            (Tool::DsRom, "ds-rom"),
            (Tool::Rotom, "rotom"),
            (Tool::Chatot, "chatot"),
            (Tool::Uxie, "uxie"),
        ] {
            assert_eq!(
                Some(lock(tool).to_owned()),
                lock_line(&text, name),
                "{name}"
            );
        }
        assert!(
            lock(Tool::DsRom).contains("8a9c151"),
            "{}",
            lock(Tool::DsRom)
        );
        assert!(
            lock(Tool::Rotom).contains("v0.7.0"),
            "{}",
            lock(Tool::Rotom)
        );
    }

    #[test]
    fn step_salt_differs_by_revision() {
        let one = step_salt(Tool::Chatot, 1);
        let two = step_salt(Tool::Chatot, 2);
        assert_ne!(one, two);
        assert_eq!(one, format!("{}#1", lock(Tool::Chatot)));
        assert_ne!(step_salt(Tool::Rotom, 1), step_salt(Tool::Uxie, 1));
        assert_eq!(STATE_VERSION, 1);
    }
}
