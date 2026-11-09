# Open Source und Release v1.0.0: Design-Spezifikation

Stand: 2026-11-09. Gilt nach dem Abschluss von Phase 6 (`scripts/gate.sh` auf main grün). Drei Arbeitspakete in fester Reihenfolge: (1) Open-Source-Reife des Repositories, (2) Zweisprachigkeit der App, der Rust-Seite und der Nutzer-Docs, (3) Release v1.0.0, Umschalten auf public und Launch. Plan: [2026-11-09-open-source-release-v1.md](../plans/2026-11-09-open-source-release-v1.md).

## 1. Ziel

Gen4 Studio erscheint als v1.0.0 öffentlich unter MIT auf `github.com/Kaago/gen4-studio`, auf Englisch und Deutsch bedienbar, mit einem README, das in der Gen-4-Hacking-Community und bei Google gefunden wird, und mit allen Dateien, die ein Open-Source-Projekt für Nutzer und Contributor braucht.

## 2. Entscheidungen (mit dem Nutzer getroffen, 2026-11-09)

| Frage | Entscheidung |
|---|---|
| Sprache | Komplett zweisprachig vor dem Release: UI, Rust-Meldungen, CLI, MCP-Antworten, Nutzer- und Agenten-Docs. Englisch ist die Standard- und Fallback-Sprache, Deutsch gleichwertig. |
| Version | v1.0.0. Die rc-Tags `v0.1.0-rc.1`, `v0.1.0-rc.2` und ihre Draft-Releases werden gelöscht. SemVer gilt ab 1.0.0 für Projektformat (`schemaVersion`), `g4k` und den MCP-Werkzeugsatz `g4.v1`. |
| Interne Docs | Bleiben öffentlich (Specs, Pläne, Gates, Research), eingeordnet durch `docs/README.md`; absolute Pfade werden entfernt. |
| Rust-Texte | Katalog mit `rust-i18n` (YAML `de`/`en`) in `g4-kernel` und `studio`; die App setzt die Locale, kopflose Binaries lesen `G4_LANG` (Standard `en`). |
| Docs-Umfang | Englisch: README (en + de), CHANGELOG, CONTRIBUTING, `docs/en/` mit `mcp.md`, `dev-setup.md`, `release.md`. `docs/formats/*.md` bleiben deutsch mit englischem Kopfabsatz; vollständige Übersetzung ist v1.1-Ziel. |
| DSPRE | Ausdrücklicher Verweis auf das Ursprungsprojekt im README und in der Kommunikation: Dank, Formatwissen, Testorakel, Projektkompatibilität, kein geteilter Code (DSPRE ist AGPL-3.0, Gen4 Studio MIT). Nie „based on“ oder „fork of“. |

Nicht in diesem Vorhaben (Roadmap nach 1.0.0): Landingpage `gen4studio.dev` (Domain hat keine NS-Einträge, vermutlich frei), Linux-Build, Developer-ID-Signatur und Notarisierung, Git LFS für die `.fig`, `docs/formats` auf Englisch, Übersetzung der Galerie-Seiten (`apps/studio/src/ui/gallery`, Entwicklungswerkzeug) und der OpenPencil-Datei.

## 3. Arbeitspaket 1: Open-Source-Reife

Branch `oss-readiness`, keine Änderung an App-Code. Alle neuen Dateien auf Englisch, außer `README.de.md`.

### 3.1 README

`README.md` (Englisch) und `README.de.md` (Deutsch, inhaltsgleich), Reihenfolge der Abschnitte:

1. Marke (`design-system/marke/gen4-studio-marke.svg`), Titel, Ein-Satz-Pitch: „Gen4 Studio is a modern, cross-platform editor for Pokémon Diamond, Pearl, Platinum, HeartGold and SoulSilver ROM hacks. Build worlds, stories, scripts and data from a text-based project, then compile to a ROM or xdelta patch. Works with AI agents via MCP.“ Badges: License MIT, Latest release, CI.
2. Hero-Screenshot `docs/media/hero-welt-bauen.png`, darunter zwei weitere (`docs/media/erzaehlen.png`, `docs/media/technik-aenderungen.png`). Die Bilder liefert der Maintainer aus der gepackten App (Hell, Komfort, 1440×900, Platin oder HeartGold).
3. What it does: Welt, Erzählen, Scripts und Texte, Daten, Prüfen, Build zu ROM oder xdelta, Textprojekt mit Git, MCP für Agenten, DSPRE-Projekte öffnen.
4. Supported games and platforms: Tabelle Spiele (Diamant, Perl, Platin, HeartGold, SoulSilver; Regionen laut `docs/formats/project_base.md`) und Plattformen (macOS 13+ arm64 DMG, Windows 10+ x64 NSIS; Spielmodus live nur macOS).
5. Install: Download-Links auf die Release-Seite, Gatekeeper-Hinweis (Rechtsklick, Öffnen; `xattr -dr com.apple.quarantine`), SmartScreen-Hinweis.
6. Quickstart: „New game from ROM…“ mit eigener Basis-ROM, Vorlage, erste Route, „Play here“, „Publish“ als xdelta.
7. Agents (MCP): zwei Absätze und der `claude mcp add`-Block aus `docs/en/mcp.md`.
8. Project format: ein Absatz zum Textprojekt, Link auf `docs/formats/text_project.md`.
9. Relationship to DSPRE: der Text aus Abschnitt 2 dieser Spec, ausformuliert (siehe Plan, Task 1).
10. Acknowledgements: DSPRE, ds-rom, uxie und nitroarc, rotom und rotom-lsp, chatot, apicula, pret (pokeheartgold, pokeplatinum), hg-engine, PokeAPI, melonDS, DS Pokémon Hacking Wiki, Kingdom of DS ROM Hacking.
11. Legal: „Gen4 Studio contains no ROM data, no game assets and no copyrighted material from Nintendo, Game Freak or The Pokémon Company. You need your own legally obtained copy of the game. Pokémon and all related names are trademarks of their respective owners. This project is not affiliated with or endorsed by them.“
12. Building from source: Verweis auf `docs/en/dev-setup.md`. Contributing: Verweis auf `CONTRIBUTING.md`. License: MIT.

Sprachumschalter oben: „English · [Deutsch](README.de.md)“.

### 3.2 Community-Dateien

- `CONTRIBUTING.md`: Dev-Setup (Link), Branch von `main`, der hermetische Teil des Gates (`G4_GATE_HERMETIC=1 bash scripts/gate.sh`) ist Pflicht vor einem PR, der ROM-gebundene Teil läuft beim Maintainer; Commit-Konvention (Conventional Commits, wie bisher `feat:`, `fix:`, `docs(…)`); Sprache: Code-Kommentare und neue Docs Englisch, UI-Texte über die Kataloge in beiden Sprachen (Arbeitspaket 2); keine ROM-Daten, keine Savegames in Issues oder PRs.
- `CODE_OF_CONDUCT.md`: Contributor Covenant 2.1, Kontakt: die E-Mail aus der Git-Historie des Maintainers.
- `SECURITY.md`: unterstützte Version (nur die aktuelle Minor), Meldung über GitHub Private Vulnerability Reporting, Antwort binnen 14 Tagen.
- `.github/ISSUE_TEMPLATE/bug.yml` (Felder: Version, Plattform, Spiel und Region, Schritte, Erwartet, Beobachtet, Logs; Checkbox „I have not attached a ROM, save game or any file derived from one“), `feature.yml`, `question.yml`, `config.yml` (blank issues aus, Link auf Discussions).
- `.github/PULL_REQUEST_TEMPLATE.md`: Was, Warum, hermetisches Gate grün, Kataloge de und en gepflegt, Docs angepasst.
- `.github/dependabot.yml`: `cargo`, `npm` (Verzeichnis `/apps/studio`), `github-actions`, wöchentlich, gruppiert (`minor-and-patch`).

### 3.3 Docs einordnen

- `docs/README.md` (Englisch): Landkarte in drei Teilen. Für Nutzer und Agenten: `docs/en/*`, `docs/formats/*` (deutsch, englischer Kopfabsatz), `docs/mcp.md`. Für Contributor: `dev-setup`, `release`, Gate-Skripte. Entwicklungsgeschichte: `ROADMAP.md`, `superpowers/specs`, `superpowers/plans`, `phase-*-gate.md`, `research/`, mit einem Absatz zur Methodik (Phasen, Gates, agentische Umsetzung mit unabhängiger Abnahme).
- Vorspann in `docs/research/2026-10-20-dspre-und-suite-analyse.md`: interne Architekturbewertung vom Oktober 2026 als Entscheidungsgrundlage, keine Kritik am Projekt; DSPRE hat seither weitere Releases.
- `design-system/openpencil/BRIEFING-APP-SCREENS.md`: absolute Pfade in das Home-Verzeichnis des Maintainers (Ordner `Projekte/lab/editor/gen4-studio/`) werden zu Pfaden relativ zur Repo-Wurzel; in den übrigen Dokumenten (Gates, Pläne, Research, Szenarien) steht statt des Home-Verzeichnisses `$HOME`.

### 3.4 Repository und CI

- Repo-Metadaten per `gh repo edit`: Description „Cross-platform editor for Pokémon Gen 4 (DPPt, HGSS) ROM hacks: worlds, stories, scripts, data, text-based project with Git, build to ROM or xdelta, MCP for AI agents. DSPRE-compatible.“, Homepage `https://github.com/Kaago/gen4-studio#readme` (bis eine Landingpage existiert), Topics: `pokemon`, `pokemon-rom-hack`, `rom-hacking`, `romhacking`, `nintendo-ds`, `heartgold`, `soulsilver`, `platinum`, `map-editor`, `level-editor`, `game-tools`, `mcp`, `mcp-server`, `tauri`, `rust`, `react`, `typescript`, `threejs`, `desktop-app`, `dspre`. Discussions an, Wiki aus, Projects aus.
- `ci.yml` bekommt neben `workflow_dispatch` die Auslöser `pull_request` (auf `main`) und `push` (Branch `main`), jeweils mit `paths-ignore` für `**.md`, `docs/**`, `design-system/**`. Der Job `gate-macos` läuft so bei jedem PR. Begründung: Für öffentliche Repositories sind Standard-Runner von GitHub Actions kostenlos, auch macOS; die bisherige Kostenbegrenzung entfällt mit dem Umschalten auf public. `gate-windows` bleibt manuell.
- Branch Protection auf `main` nach dem Umschalten (Arbeitspaket 3): PR erforderlich, Status-Check `gate-macos`, keine Force-Pushes.

## 4. Arbeitspaket 2: Zweisprachigkeit

Branch `i18n`. Englisch ist Standard und Fallback; Deutsch ist vollständig. Die Sprache folgt dem System (`navigator.language`, `de*` → `de`, sonst `en`) und lässt sich in den Einstellungen festlegen (System, Deutsch, English), gespeichert wie Thema und Dichte in `localStorage` unter `g4.lang`.

### 4.1 Bestand

| Bereich | Umfang (Zeilen mit deutschem Text, ohne Tests) |
|---|---|
| Frontend `apps/studio/src` | rund 1.800 in 520 Dateien; Schwerpunkte `workbench/` (550), `ui/` (230), `data/` (200), `technik/` (115), `app/` (100), Wurzeldateien (280) |
| `crates/g4-kernel/src` | rund 490; Schwerpunkte `textproject`, `world`, `original`, `recipes`, `stories`, `git`, `scripts` |
| `apps/studio/src-tauri/src` | rund 320; Schwerpunkte `mcp/` (90, Antworten und Freigabetexte; der Werkzeugkatalog ist bereits englisch), `commands/data/labels.rs` (Feldnamen), `commands/dispatch.rs`, `bridge/` |
| Tests mit deutschen Zusicherungen | Frontend 204 Testdateien, Rust rund 150 Literale |
| `crates/g4-formats` | 4 Literale, alle in Tests; keine Übersetzung |
| `g4k` | 7 Literale; folgt dem Kernel-Katalog |

Bereits zentral: `apps/studio/src/checks/messages.ts` (`CHECK_MESSAGES`, Codes mit Titel und Satz), `apps/studio/src/workbench/terms.ts` (`TERMS`), `apps/studio/src/words.ts` (`count`, `NBSP`, `WJ`), `apps/studio/src-tauri/src/commands/data/labels.rs`, die Meldungsmodule `previewMessages.ts`, `startMessages.ts`, `dockMessages.ts`.

### 4.2 Frontend

- Bibliotheken: `i18next` 27.0.1, `react-i18next` 18.0.1 (beide MIT, bestehen `scripts/check-licenses-npm.sh`). Keine Sprach-Erkennungs-Bibliothek; die Erkennung ist eine Funktion.
- Modul `apps/studio/src/i18n/`:
  - `index.ts`: `init()` (einmal beim Start, vor dem ersten Render), `resources` (`as const`), `defaultNS = "common"`, Typ `Language = "system" | "de" | "en"`, `readLanguage(): Language`, `applyLanguage(l: Language): void` (schreibt `localStorage`, ruft `i18next.changeLanguage`, setzt `document.documentElement.lang`, ruft den Tauri-Befehl `set_locale`), `effectiveLanguage(): "de" | "en"`, `detectLanguage(nav: string): "de" | "en"`.
  - `i18next.d.ts`: `CustomTypeOptions` mit `defaultNS` und `resources` aus `index.ts` (typisierte Schlüssel; ein Tippfehler im Schlüssel ist ein Compile-Fehler).
  - `locales/de/<ns>.json` und `locales/en/<ns>.json`, ein Namespace je Ordner von `apps/studio/src`: `common` (Wurzeldateien, `words.ts`), `app`, `ui`, `workbench`, `world`, `data`, `stories`, `flags`, `technik`, `project`, `checks`, `git`, `publish`, `textproject`, `original`, `delete`, `platform`, `recipes`, `terms`. Schlüssel in `camelCase`, verschachtelt nach Komponente (`"newArea": { "title": "…" }`), Platzhalter `{{name}}`, Plural über `_one` und `_other` mit `count`.
  - `catalogs.test.ts`: für jeden Namespace sind die Schlüsselmengen von `de` und `en` gleich, jeder Wert ist ein nicht leerer String, die Platzhaltermengen je Schlüssel sind gleich, jeder Schlüssel mit `_one` hat `_other` und umgekehrt.
- Verwendung: Komponenten über `useTranslation("<ns>")`, Module ohne React über `i18next.t("<ns>:<key>")`. `words.count` wird durch Plural-Schlüssel ersetzt; `NBSP` und `WJ` stehen in den Katalogwerten. `terms.ts` behält `TermKey` und `tech`, holt `label`, `plural`, `short` aus `terms.json`. `CHECK_MESSAGES` behält `group` und `severity`; `title`, `text`, `one`, `building` kommen aus `checks.json` unter `<code>.title`, `<code>.text`, `<code>.one`, `<code>.building.title`, `<code>.building.text`.
- Tests: `vitest.setup.ts` initialisiert i18n mit `lng: "de"`. Die vorhandenen Tests behalten ihre deutschen Zusicherungen; die Prüfung der englischen Texte liegt in `catalogs.test.ts` und in je einem Test pro Bereich, der nach `changeLanguage("en")` eine Komponente rendert.
- `index.html`: `lang` wird beim Start gesetzt, nicht mehr statisch `de`.
- Einstellungen: `SettingsArea` bekommt neben Thema und Dichte den Schalter „Language“ (System, Deutsch, English) als `Segmented`.
- Rust-Test `commands::checks::messages_ts` liest statt `messages.ts` beide `checks.json` (serde_json) und hält Codes, Reihenfolge, `group` und `severity` wie bisher gegen `Registry::all()`; `group` und `severity` bleiben in `messages.ts`.

### 4.3 Rust

- `rust-i18n = "4.2.4"` als Workspace-Abhängigkeit (MIT). `g4-kernel` und `studio` rufen `i18n!("locales", fallback = "en")` in `lib.rs`; Kataloge `crates/g4-kernel/locales/{en,de}.yml` und `apps/studio/src-tauri/locales/{en,de}.yml`. Schlüssel `module.snake_case`, Platzhalter `%{name}`.
- `g4_kernel::i18n`: `pub const DEFAULT: &str = "en"`, `pub fn set_locale(l: &str)` (nimmt `de` oder `en`, alles andere wird `en`), `pub fn locale() -> String`, `pub fn init_from_env()` (liest `G4_LANG`, dann `LANG`-Präfix `de`, sonst `en`), `pub fn check_catalogs(dir: &Path) -> Result<(), Vec<String>>` (Schlüsselmengen und `%{…}`-Mengen von `en.yml` und `de.yml` gleich, keine leeren Werte) für die Tests beider Crates.
- Alle nutzerseitigen Texte des Kernels und der Tauri-Seite (Fehler über `CmdError.message`, MCP-Antworten und Freigabetexte, Feldnamen in `labels.rs`, Meldungen von `g4k`) werden `t!("…")`-Aufrufe. `const … &str`-Meldungen werden Funktionen oder Aufrufe am Ort. Codes (`CmdError.code`, Prüfcodes, Fehlerarten) bleiben unverändert und sind kein Teil der Kataloge.
- Locale setzen: Tauri-Befehl `set_locale(lang: String)` (vom Frontend in `applyLanguage` und beim Start). `studio mcp`, `studio-bridge` und `g4k` rufen `init_from_env()` als Erstes in `main`.
- MCP: `glossary.md` wird `glossary.en.md` und `glossary.de.md`, `prompts.rs` liefert die Texte der aktuellen Locale; der Werkzeugkatalog bleibt englisch.
- Tests: Rust-Tests sichern englische Texte zu (Standard-Locale `en`); deutsche Zusicherungen werden übersetzt. Je ein Test pro Crate ruft `check_catalogs`. Ein Test setzt `de`, prüft eine Meldung, setzt zurück.
- `g4-formats` bleibt ohne Katalog.

### 4.4 Docs

- `CHANGELOG.md` wird englisch (Keep a Changelog, bestehende Einträge übersetzt), neuer Abschnitt `## [1.0.0] - <Datum>` mit den Phasen 4.5, 3d und 6 und der Zweisprachigkeit. `scripts/release/changelog-section.sh` bleibt.
- `docs/en/mcp.md`, `docs/en/dev-setup.md`, `docs/en/release.md`: Übersetzungen, inhaltsgleich; die deutschen Originale verweisen oben auf sie und umgekehrt.
- `docs/formats/*.md`: erster Absatz je Datei auf Englisch („In English: …“, drei bis sechs Sätze: Zweck, Dateien, wo der Code liegt).
- `docs/ROADMAP.md`: Abschnitt „Nach 1.0.0“ mit den Punkten aus Abschnitt 2 dieser Spec.

## 5. Arbeitspaket 3: Release und Launch

Auf `main`, nach dem Merge von `oss-readiness` und `i18n` und einem grünen `scripts/gate.sh`.

1. `bash scripts/version.sh set 1.0.0`, Datum im Changelog, Commit `release: 1.0.0`.
2. rc-Reste entfernen: `gh release delete v0.1.0-rc.1 --yes --cleanup-tag`, dito rc.2.
3. Umschalten: `gh repo edit Kaago/gen4-studio --visibility public --accept-visibility-change-consequences`, dann Discussions, Branch Protection, Dependabot-Alerts und Private Vulnerability Reporting einschalten.
4. `bash scripts/release-local.sh v1.0.0 --dry-run`, dann ohne `--dry-run`; `gh workflow run release.yml -f tag=v1.0.0 -f dry_run=false`; nach dem Lauf: `SHA256SUMS.txt` hat zwei Zeilen (DMG und Installer), `gh release edit v1.0.0 --draft=false --latest`.
5. Launch-Texte liegen vorbereitet in `docs/launch/` (Englisch): `announcement.md` (Langfassung für PokéCommunity und Project Pokémon), `discord.md` (KoDSH, kurz, beginnt mit dem Dank an das DSPRE-Team), `reddit.md` (r/PokemonROMhacks), `show-hn.md` (Winkel: textbasiertes ROM-Projekt mit Build-Pipeline und MCP), `wiki-pr.md` (Eintrag für ds-pokemon-hacking.github.io, Getting Started und Mapping Tools), `awesome-lists.md` (Zeilen für Awesome-Rom-Hacking, awesome-romhacking, awesome-mcp-servers, awesome-tauri). Das Veröffentlichen übernimmt der Maintainer in dieser Reihenfolge: Discord, Wiki-PR, Foren, Reddit, HN, Listen.

## 6. Gate

- Arbeitspaket 1: `README.md` und `README.de.md` haben dieselben Überschriften in derselben Reihenfolge (Test `scripts/tests/readme.test.sh`); alle Links im README und in `docs/README.md` zeigen auf existierende Dateien oder Anker; `git grep -n '/Users/POLT[E]P'` (die eckige Klammer verhindert den Treffer in dieser Zeile) findet nichts; `gh repo view` zeigt Description, Homepage, 20 Topics, Discussions an, Wiki aus; `ci.yml` besteht `scripts/check-actions.sh`.
- Arbeitspaket 2: `pnpm test`, `cargo test --workspace` und der hermetische Gate grün; `catalogs.test.ts` und `check_catalogs` grün; `git grep -nE '"[^"]*[äöüÄÖÜß][^"]*"' -- 'apps/studio/src/**/*.tsx' 'apps/studio/src/**/*.ts' ':!**/*.test.*' ':!apps/studio/src/i18n/**' ':!apps/studio/src/ui/gallery/**'` findet nichts; dasselbe für `crates/g4-kernel/src` und `apps/studio/src-tauri/src` ohne `locales/` und ohne `#[cfg(test)]`-Module; die App startet mit `G4_LANG=en` bzw. englischem System komplett englisch (Blick beim Nutzer: Start, Welt bauen, Erzählen, Technik, Einstellungen, ein Fehler-Toast, der Assistent „New game from ROM…“); `studio mcp` antwortet mit `G4_LANG=de` deutsch und ohne Variable englisch.
- Arbeitspaket 3: Release `v1.0.0` ist veröffentlicht, nicht Draft, mit DMG, Installer und `SHA256SUMS.txt` (zwei Zeilen); `gh repo view --json visibility` ist `PUBLIC`; `check-no-rom-data.sh --history v0.1.0-rc.1..v1.0.0` und auf beiden Paketen grün; `docs/launch/` vollständig.
