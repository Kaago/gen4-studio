//! Lookup of the bundled helper executables (`apicula`, `rotom-lsp`), shared by both searches.

use std::env::consts::EXE_SUFFIX;
use std::path::{Path, PathBuf};

/// File name of the sidecar `stem` on this platform: `apicula` or `apicula.exe`.
pub(crate) fn sidecar_name(stem: &str) -> String {
    format!("{stem}{EXE_SUFFIX}")
}

/// True for a non-empty regular file. An empty file is the build-time placeholder for a
/// bundled sidecar (see build.rs), never the real thing.
pub(crate) fn is_real_file(path: &Path) -> bool {
    path.metadata()
        .map(|m| m.is_file() && m.len() > 0)
        .unwrap_or(false)
}

/// First real file called `name` in one of `dirs`.
fn find_in(dirs: &[PathBuf], name: &str) -> Option<PathBuf> {
    dirs.iter()
        .map(|dir| dir.join(name))
        .find(|p| is_real_file(p))
}

/// Finds a sidecar: the path in `env_var`, then `stem` next to the running binary, then (debug
/// builds only) `<repo>/<debug_rel>`. Both file names carry the platform's executable suffix. A
/// set `env_var` always wins, even when it points nowhere, so a typo is reported instead of
/// silently ignored. An empty file never counts.
pub(crate) fn find_sidecar(env_var: &str, stem: &str, debug_rel: &str) -> Option<PathBuf> {
    if let Some(env) = std::env::var_os(env_var).filter(|v| !v.is_empty()) {
        return Some(PathBuf::from(env));
    }
    let name = sidecar_name(stem);
    let beside_exe: Vec<PathBuf> = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf))
        .into_iter()
        .collect();
    if let Some(found) = find_in(&beside_exe, &name) {
        return Some(found);
    }
    if cfg!(debug_assertions) {
        let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        return find_in(&[repo], &format!("{debug_rel}{EXE_SUFFIX}"));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env::consts::EXE_SUFFIX;

    /// A fresh empty directory under the system temp dir, removed by the caller.
    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("g4-sidecar-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn sidecar_name_has_exe_suffix_on_windows() {
        assert_eq!(sidecar_name("apicula"), format!("apicula{}", EXE_SUFFIX));
    }

    #[test]
    fn placeholder_is_not_found() {
        let dir = scratch("placeholder");
        let name = sidecar_name("apicula");
        std::fs::write(dir.join(&name), b"").unwrap();
        assert_eq!(find_in(std::slice::from_ref(&dir), &name), None);
        std::fs::write(dir.join(&name), b"x").unwrap();
        assert_eq!(
            find_in(std::slice::from_ref(&dir), &name),
            Some(dir.join(&name))
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn env_wins_even_if_missing() {
        // A variable of its own: tests share the process environment.
        let var = "G4_SIDECAR_TEST_ENV_WINS";
        let missing = std::env::temp_dir().join("g4-sidecar-does-not-exist");
        // SAFETY: only this test touches this variable.
        unsafe { std::env::set_var(var, &missing) };
        let found = find_sidecar(var, "apicula", "tools/apicula/bin/apicula");
        unsafe { std::env::remove_var(var) };
        assert_eq!(found, Some(missing));
    }

    #[test]
    fn empty_env_var_is_ignored() {
        let var = "G4_SIDECAR_TEST_EMPTY_ENV";
        // SAFETY: only this test touches this variable.
        unsafe { std::env::set_var(var, "") };
        let found = find_sidecar(var, "g4-no-such-sidecar", "tools/g4-no-such-sidecar");
        unsafe { std::env::remove_var(var) };
        assert_eq!(found, None);
    }
}
