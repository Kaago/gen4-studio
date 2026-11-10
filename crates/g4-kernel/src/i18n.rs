//! The language of the messages: which locale is active, how a process without a UI picks it,
//! and the check that the two catalogs of a crate (`locales/en.yml`, `locales/de.yml`) agree.
//!
//! The locale is process-wide (`rust_i18n::set_locale`), so the kernel, the Tauri crate and
//! `g4k` share it. English is the default and the fallback; German is complete.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde_yaml::Value;

/// The default locale, and the one a missing translation falls back to.
pub const DEFAULT: &str = "en";

/// Sets the locale of this process: `de` (also `de-AT`, `de_CH`, `DE`, `de_DE.UTF-8`) selects
/// German, anything else English.
pub fn set_locale(lang: &str) {
    rust_i18n::set_locale(normalize(lang));
}

/// The current locale: `"en"` until [`set_locale`] or [`init_from_env`] chose another one.
pub fn locale() -> String {
    rust_i18n::locale().to_string()
}

/// Picks the locale at the start of a process without a UI (`studio mcp`, `studio-bridge`,
/// `g4k`): `G4_LANG` if it is set (and not empty), else `LANG`, German when it begins with `de`
/// and English otherwise.
pub fn init_from_env() {
    let g4_lang = std::env::var("G4_LANG").ok();
    let lang = std::env::var("LANG").ok();
    rust_i18n::set_locale(choose(g4_lang.as_deref(), lang.as_deref()));
}

/// The locale for the values of `G4_LANG` and `LANG`: `G4_LANG` wins over `LANG`, an empty
/// `G4_LANG` counts as unset, nothing set is English.
fn choose(g4_lang: Option<&str>, lang: Option<&str>) -> &'static str {
    let chosen = g4_lang.filter(|v| !v.trim().is_empty()).or(lang);
    normalize(chosen.unwrap_or(DEFAULT))
}

/// `"de"` when the language of `lang` is German, else [`DEFAULT`]. The language is what comes
/// before the first separator of `de`, `de-AT`, `de_CH`, `de_DE.UTF-8`, `de@euro`.
fn normalize(lang: &str) -> &'static str {
    let language = lang
        .trim()
        .split(['-', '_', '.', '@'])
        .next()
        .unwrap_or_default();
    if language.eq_ignore_ascii_case("de") {
        "de"
    } else {
        DEFAULT
    }
}

/// A catalog with its nested maps flattened: dotted key to value.
type Catalog = BTreeMap<String, String>;

/// Checks the catalogs `en.yml` and `de.yml` in `dir`: both have the same keys (nested maps
/// count by their dotted path, a top-level `_version` is no key), each key has the same
/// `%{name}` placeholders on both sides and every value is a non-empty string. `Ok` is the
/// number of keys; `Err` lists every problem, each naming the key and the file it is in.
pub fn check_catalogs(dir: &Path) -> Result<usize, Vec<String>> {
    let mut errors = Vec::new();
    let en = load("en.yml", dir, &mut errors);
    let de = load("de.yml", dir, &mut errors);
    let (Some(en), Some(de)) = (en, de) else {
        return Err(errors);
    };
    for key in en.keys().filter(|key| !de.contains_key(*key)) {
        errors.push(format!("de.yml: missing key `{key}` (present in en.yml)"));
    }
    for key in de.keys().filter(|key| !en.contains_key(*key)) {
        errors.push(format!("en.yml: missing key `{key}` (present in de.yml)"));
    }
    for (key, en_text) in &en {
        let Some(de_text) = de.get(key) else { continue };
        let (in_en, in_de) = (placeholders(en_text), placeholders(de_text));
        if in_en != in_de {
            errors.push(format!(
                "key `{key}`: placeholders differ (en.yml: {}, de.yml: {})",
                names(&in_en),
                names(&in_de)
            ));
        }
    }
    if errors.is_empty() {
        Ok(en.len())
    } else {
        Err(errors)
    }
}

/// Reads and flattens `dir/file`; a problem with the file or its values goes to `errors`
/// (`None` when the file cannot be used at all).
fn load(file: &str, dir: &Path, errors: &mut Vec<String>) -> Option<Catalog> {
    let path = dir.join(file);
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) => {
            errors.push(format!("{file}: cannot read {}: {e}", path.display()));
            return None;
        }
    };
    let root = match serde_yaml::from_str::<Value>(&text) {
        Ok(root) => root,
        Err(e) => {
            errors.push(format!("{file}: invalid YAML: {e}"));
            return None;
        }
    };
    let Value::Mapping(top) = root else {
        errors.push(format!("{file}: the top level is not a mapping"));
        return None;
    };
    let mut catalog = Catalog::new();
    for (key, value) in &top {
        match key.as_str() {
            Some("_version") => {}
            Some(key) => flatten(file, key, value, &mut catalog, errors),
            None => errors.push(format!("{file}: a top-level key is not a string")),
        }
    }
    Some(catalog)
}

/// Adds `value` under `key` to `catalog`, descending into maps. A value that is not a string
/// or is empty is a problem, but its key still counts, so it is not reported as missing too.
fn flatten(file: &str, key: &str, value: &Value, catalog: &mut Catalog, errors: &mut Vec<String>) {
    let text = match value {
        Value::Mapping(map) => {
            for (name, child) in map {
                match name.as_str() {
                    Some(name) => flatten(file, &format!("{key}.{name}"), child, catalog, errors),
                    None => errors.push(format!("{file}: a key below `{key}` is not a string")),
                }
            }
            return;
        }
        Value::String(text) => {
            if text.trim().is_empty() {
                errors.push(format!("{file}: empty value for key `{key}`"));
            }
            text.clone()
        }
        _ => {
            errors.push(format!("{file}: the value of key `{key}` is not a string"));
            String::new()
        }
    };
    if catalog.insert(key.to_owned(), text).is_some() {
        errors.push(format!("{file}: key `{key}` is defined twice"));
    }
}

/// The names of the `%{name}` placeholders in `text`.
fn placeholders(text: &str) -> BTreeSet<&str> {
    let mut found = BTreeSet::new();
    let mut rest = text;
    while let Some(open) = rest.find("%{") {
        let after = &rest[open + 2..];
        let Some(close) = after.find('}') else { break };
        found.insert(&after[..close]);
        rest = &after[close + 1..];
    }
    found
}

/// `%{a}, %{b}` for the error messages, `none` for no placeholder.
fn names(set: &BTreeSet<&str>) -> String {
    if set.is_empty() {
        return "none".to_owned();
    }
    set.iter()
        .map(|name| format!("%{{{name}}}"))
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn german_in_any_spelling_is_german_the_rest_english() {
        for lang in [
            "de",
            "DE",
            "de-AT",
            "de_CH",
            "de_DE.UTF-8",
            "de@euro",
            " de ",
        ] {
            assert_eq!(normalize(lang), "de", "{lang}");
        }
        for lang in [
            "",
            "en",
            "en_US.UTF-8",
            "fr",
            "deu",
            "dex",
            "xx-de",
            "C",
            "POSIX",
        ] {
            assert_eq!(normalize(lang), "en", "{lang}");
        }
    }

    #[test]
    fn g4_lang_wins_over_lang_and_nothing_is_english() {
        assert_eq!(choose(Some("de"), Some("en_US.UTF-8")), "de");
        assert_eq!(choose(Some("en"), Some("de_DE.UTF-8")), "en");
        assert_eq!(choose(Some("xx"), Some("de_DE.UTF-8")), "en");
        assert_eq!(choose(None, Some("de_DE.UTF-8")), "de");
        assert_eq!(choose(None, Some("C")), "en");
        assert_eq!(choose(None, None), "en");
        // An empty `G4_LANG` is no choice.
        assert_eq!(choose(Some(""), Some("de_AT.UTF-8")), "de");
    }

    #[test]
    fn placeholders_are_the_names_between_the_braces() {
        assert_eq!(placeholders("none"), BTreeSet::new());
        assert_eq!(
            placeholders("%{n} of %{total}, %{n} again"),
            BTreeSet::from(["n", "total"])
        );
        assert_eq!(placeholders("open %{name"), BTreeSet::new());
        assert_eq!(names(&BTreeSet::from(["a", "b"])), "%{a}, %{b}");
    }
}
