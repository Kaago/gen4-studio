# Phase 4.1a „Basis-ROM und Patch“ Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ein geöffneter DSPRE-Ordner bekommt über „Basis-ROM zuordnen…“ (App, MCP, `g4k base assign`) eine Basismarke v2 mit `origin: "dspre"` und baut danach byte-gleich zu seiner `.nds`; das Studio findet die Basis-ROM über die SHA-1 (App-Einstellung `base-roms.json`, `--base-rom`, `G4_BASE_ROM_DIR`), liest sie mit dem Kernel-Modul `base` und schreibt in Technik › Veröffentlichen › „Patch erstellen…“ einen selbstgeprüften xdelta-Patch `build/<Projekt>-<Spielcode>.xdelta` mit Begleitdatei; `cargo deny check licenses advisories` wird Gate-Schritt.

**Architecture:** xdelta3.c (3.0.12, Apache-2.0) liegt unverändert in einem eigenen Workspace-Crate `g4-xdelta`; eine kleine C-Datei bindet sie als eine Übersetzungseinheit ein und bietet drei Funktionen, die Rust über eine schmale `extern "C"`-Schicht ohne bindgen aufruft. Im Kernel erweitert `start.rs` die Marke und bekommt Prüfen, Zuordnen und Vorschlag; das neue Modul `base` sucht die Basis-ROM (ausdrücklich, gemerkter Eintrag, Nachbar-`.nds`, Ordner) und liest Dateien, NARCs, Member, ARM9 und die Header-Tabelle aus ihr; das neue Modul `patch` erzeugt, prüft und schreibt den Patch. Die App merkt sich Basis-ROMs in `<App-Daten>/base-roms.json` (Muster `agents.json`), schreibt beim Zuordnen die Agenten-Freigabe auf die neue `base_id` um und zeigt alles in der neuen Fläche Technik › Veröffentlichen.

**Tech Stack:** Rust (g4-kernel, neues Crate g4-xdelta mit `cc`, ds-rom `raw::Rom`/`FileSystem`, sha1), Tauri-Befehle, MCP (rmcp, Befehlstabelle), React 19 mit den Bausteinen aus `apps/studio/src/ui/`, vitest, cargo-deny 0.20.2, xdelta3-CLI (nur Gegenprobe).

**Spec:** [entscheidungen.md](../../research/2026-10-27-phase-4/entscheidungen.md) Abschnitte „Phasenschnitt und Reihenfolge“, „Basis-ROM und Patch (Gutachten B, 4.1)“ (die Spalte „Entscheidung“ gilt wörtlich; die Zeilen „Auf Original zurücksetzen (4.1b)“ und „Liste „geändert gegenüber Original““ gehören nicht hierher), „Nicht in Phase 4“, „Nebenaufgaben 4.0 und das Projektformat“; [gutachten-b-build-graph.md](../../research/2026-10-27-phase-4/gutachten-b-build-graph.md) §1.4, §3.1, §3.2, §4, §6 (Gate-Vorschlag 4.1a) und [`gutachten-b-messung/`](../../research/2026-10-27-phase-4/gutachten-b-messung/) (`deny.toml`, `mark_dspre.py`, `xdelta-cli.sh`); [ROADMAP.md](../../ROADMAP.md) „Phase 4“, Punkt 4.1a mit Gate.

Stand 27.10.2026, `main` bei `4d87ee0`. Arbeit im eigenen Worktree (Branch `phase-4-1a`), **parallel zu 4.2** in einem zweiten Worktree. Kernel `crates/g4-kernel/src`, Tauri `apps/studio/src-tauri/src`, Frontend `apps/studio/src`.

## Entscheidungen (wo Roadmap und Gutachten offen lassen)

| Frage | Entscheidung | Grund |
|---|---|---|
| Wo liegt xdelta3.c | eigenes Crate `crates/g4-xdelta` (Lizenz `MIT AND Apache-2.0`, `publish = false`, einzige Abhängigkeit die Build-Abhängigkeit `cc`), Quelle unter `vendor/xdelta3/`, `NOTICE` im Crate | der C-Bau und alles `unsafe` bleiben aus dem Kernel heraus; `g4-kernel` hat kein `build.rs`, und 4.2 arbeitet parallel im Kernel |
| Herkunft der Quelle | xdelta 3.0.12 aus dem Tarball des Crates `xdelta3` 0.1.5 (Verzeichnis `xdelta3/xdelta3/`, liegt im Cargo-Register unter `~/.cargo/registry/src/*/xdelta3-0.1.5/`, Upstream jmacd/xdelta; jede Datei trägt „Copyright 2016 Joshua MacDonald, Licensed under the Apache License, Version 2.0“); kopiert werden `xdelta3.c`, `xdelta3.h`, alle `xdelta3-*.h`, `LICENSE`, **unverändert** | dieselbe C-Quelle, mit der Gutachten B gemessen hat (4,8 bis 8,3 KB); das Crate selbst wird nicht verwendet (bindgen 0.52 mit vier RustSec-Befunden) |
| FFI-Form | `csrc/g4_xdelta.c` beginnt mit `#include "xdelta3.c"` (eine Übersetzungseinheit, nur diese Datei kompiliert `cc`) und nutzt deren statische Helfer wie `xd3_process_memory` es tut; Rust sieht nur drei Funktionen mit Puffern und Längen, nie `xd3_stream` | ein von Hand gespiegeltes `xd3_stream`/`xd3_config` in Rust wäre ohne bindgen brüchig; App-Header und Adler32 brauchen die Stream-API, die `xd3_encode_memory` nicht anbietet |
| Encoder-Einstellung | wie `xd3_process_memory` (ganze Quelle als ein Block, `max_winsize = source_size`, `winsize = min(Ziel, XD3_DEFAULT_WINSIZE)`), Flags genau `XD3_ADLER32`, kein `XD3_SEC_*`, App-Header über `xd3_set_appheader`; C immer mit `opt_level(2)` | entspricht der Messung (Crate-Pfad) plus Adler32; der Debug-Build der Tests soll nicht zehnmal langsamer kodieren |
| App-Header | genau `<Name>.nds//<Basislabel> [<SHA-1, 40 Stellen>].nds/` (xdelta-Form `ziel/zielkompr/quelle/quellkompr` mit leeren Kompressionsfeldern) | Entscheidung „`<ziel>//<quelle [sha1]>/`“; die volle SHA-1, weil das Gutachten-Beispiel nur zur Anzeige kürzt |
| Name | `<Name>` = `<Projekt>-<Spielcode>`; `<Projekt>` ist der Ordnername ohne `_DSPRE_contents`, die Zeichen `/ \ : * ? " < > \|` und Steuerzeichen werden `_`, Leerzeichen und Punkte an den Enden fallen weg, leer wird `Projekt` | Ziel `build/<Projekt>-<Spielcode>.xdelta`; DSPRE-Ordner heißen sonst „…_DSPRE_contents-CPUD“ |
| Gebaute ROM des Patches | der Export baut nach `build/<Name>.nds` und lässt sie dort liegen | die Begleitdatei nennt ihre SHA-1, der Nutzer kann genau sie spielen; `build/` und `*.nds` ignoriert die Git-Vorlage (4.4) |
| Schreiben | Patch, dann Begleitdatei, je über eine Temp-Datei `.<Datei>.tmp` im selben Ordner und `rename`; scheitert die Selbstprüfung, wird nichts geschrieben, ein alter Patch bleibt | kein halber Patch neben einer passenden Begleitdatei |
| Begleitdatei | deutsch, fester Text (Task 5), `\n` | die App ist deutsch; Felder wie entschieden (Spiel, beide SHA-1, Studio-Version, ein Satz Anleitung) |
| Marke v2 | `BASE_MARK_VERSION = 2` für **jede** neue Marke (auch `create_project`); `template` und `assigned` sind `Option` mit `skip_serializing_if`; `created` = Zeitpunkt der ersten Zuordnung, `assigned` = Zeitpunkt der letzten; v1 liest weiter | ein Schreiber, eine Version; `base_id = <sha1>/<created>` bleibt stabil, solange dieselbe ROM zugeordnet ist |
| Erneut zuordnen | gleiche SHA-1: nichts schreiben, Ergebnis wie beim ersten Mal; andere SHA-1 bei `origin: "dspre"`: neue Marke (neues `created`); bei `origin: "extracted"`: abgelehnt; unlesbare oder neuere Marke: abgelehnt mit Grund | die Basis eines angelegten Projekts steht fest; eine Marke einer neueren Fassung darf nicht still überschrieben werden |
| Texte der Prüfung | deutsch im Kernel (`BaseWarning.message`, Ablehnungen), wörtlich in Task 3; dieselben Sätze in App, MCP und `g4k` | eine Quelle; Vorbild `DP_LOCKED` |
| ARM7 und Banner | nur wenn beide Seiten lesbar sind: `arm7/arm7.bin` gegen den ARM7 der ROM (Bytes), Banner-Titel aus `banner/banner.yaml` gegen die der ROM; sonst keine Warnung | Gutachten §3.1 Punkt 5 „optional und billig“; die synthetische Test-ROM hat keinen ARM7 |
| Vorschlag | `suggest_base(root)`: der Ordnername endet auf `_DSPRE_contents`, daneben liegt `<Name>.nds`, deren Kopf (0x200 Byte, kein Hash) denselben Spielcode trägt wie `header.yaml` | Entscheidung „liegt `<Name>.nds` neben `<Name>_DSPRE_contents`, schlägt die App sie vor“; billig beim Öffnen von Technik |
| Suche der Basis-ROM | `base::locate` in dieser Reihenfolge: ausdrücklich (`--base-rom`; andere SHA-1 ist ein Fehler) → gemerkter Eintrag (Größe und mtime gleich: ohne Hash, sonst neu hashen) → Nachbar-`.nds` (Vorschlag) → `G4_BASE_ROM_DIR` (nicht rekursiv, `*.nds`, erst Größe, dann SHA-1) → nichts; der Aufrufer kann Pfade ausschließen | Entscheidung „Auflösen über Größe und mtime, sonst neu hashen“; der Nachbar macht einen über `g4k` zugeordneten Ordner auch in der App sofort patchbar |
| Originale | ist `G4_ROMS_DIR` gesetzt, schließt die App Basis-ROMs unter den Originalen (Regel `is_original` der `UserPolicy`) von der Suche aus | Entwicklungsregel „Originale nie öffnen“, auch für die App im Entwicklungsbetrieb |
| `base-roms.json` | `{version: 1, roms: [{sha1, path, size, mtime_ms, game_code, label, verified}]}`, ein Eintrag je SHA-1, atomar und 0600 über `mcp::settings::write_private`; kopflos (`studio mcp --project`, `g4k`) wird sie nicht gelesen | Entscheidung „Muster `agents.json`“; kopflos gelten `--base-rom` und `G4_BASE_ROM_DIR` (Gutachten §3.2) |
| Freigabe umschreiben | nach dem Zuordnen wandert die Freigabe mit dem alten Schlüssel (Pfad, alte `base_id`, also meist `None`) auf die neue `base_id`; andere Freigaben bleiben | Entscheidung „schreibt den Eintrag `(pfad, None)` auf die neue `base_id` um“, auch nach einem Wechsel der ROM |
| MCP | Werkzeuge `base_status` und `check_base` (`Read`), `assign_base` (`Irreversible("Basis-ROM zuordnen")`), `create_patch` (`Irreversible("Patch erstellen")`, Ziel fest `build/<Name>.xdelta`); `register_base_rom` ist `Unavailable("Nur in der App")`; `assign_base` und `create_patch` stehen in `NOT_IN_BATCH` | Entscheidung „über MCP unumkehrbar mit Rückfrage“; App-Einstellungen ändert kein Agent |
| Pfad der `.nds` über MCP | `check_base`/`assign_base` lesen `nds` über `policy.read_path`; Ausnahme: der Pfad ist genau der Vorschlag `suggest_base(projekt)` (und kein Original) | die Nachbar-`.nds` liegt außerhalb des Projektordners; ohne die Ausnahme könnte ein Agent der App nie zuordnen |
| Ort in der App | neuer Eintrag „Veröffentlichen“ (Symbol `publish` = Phosphor `UploadSimple`, wie in `jsx-lib.js` der Titelleiste) an zweiter Stelle der Technik-Liste; die Fläche hat die Karten „Basis-ROM“ und „Patch“; eigene Datei `technik/PublishSurface.tsx` | Design-System „Veröffentlichen“; eigene Datei hält den Merge mit 4.2 (ändert „Prüfen und Bauen“ in `TechnikArea.tsx`) klein |
| Kein Modaldialog beim Öffnen | der Vorschlag erscheint nur in Technik › Veröffentlichen (Karte und Untertitel der Liste); keine Meldung beim Öffnen, keine Taste in der Titelleiste | Entscheidung „kein Modaldialog beim Öffnen“; der Rest ist unter „Offen“ |
| `cargo deny` | `deny.toml` aus der Vorlage des Gutachtens mit `[graph] all-features = true`; Ausnahmen nur je RUSTSEC-ID mit Grund; Gate-Schritt nach clippy | `gate.sh` baut mit `--all-features`, also prüft deny denselben Baum |
| `G4_BASE_ROM_DIR` im Gate | zeigt auf `Roms/_testkopien`, nicht auf `G4_ROMS_DIR` (Abweichung von Gutachten §3.2) | Originale werden von Hand nie geöffnet; die gated Tests finden die Basis wie heute `G4_TEST_ROM` neben dem Original (nur gelesen) |

## Global Constraints

- Commits enden mit `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>`; vor jedem cargo-Aufruf `export PATH="$HOME/.cargo/bin:/opt/homebrew/opt/rustup/bin:$PATH"`.
- Originale unter `/Users/POLTEP/Projekte/lab/editor/Roms` nie von Hand öffnen oder ändern; Testprojekte nur als frische Kopien unter `Roms/_testkopien/4-1a-<zweck>` (Ordner `4-1a-<zweck>_DSPRE_contents` und `.nds` `4-1a-<zweck>.nds` direkt unter `_testkopien`). Gated Tests lesen wie bisher `G4_TEST_PROJECT` (von `scripts/gate.sh` gesetzt) und arbeiten auf `vanilla_copy()`; die Basis-ROM finden sie über `suggest_base(vanilla_project())` und lesen sie nur.
- Keine ROM-Daten, keine `.nds`, keine Patches gebauter ROMs ins Repo. Die xdelta3-Quelle darf ins Repo (Apache-2.0 mit `NOTICE`).
- Nur ein falscher Spielcode gegen `header.yaml` sperrt das Zuordnen; Titel, Version, unbekannte SHA-1, ARM7 und Banner warnen.
- Fehlende Basis-ROM sperrt nie den Build; Export (in 4.1a das einzige Werkzeug, das sie braucht) ist aus und nennt den Grund; Ersatz nur mit gleicher SHA-1.
- Patch ohne Zweitkompression, mit Adler32, Basis-SHA-1 im App-Header und in `<Name>.xdelta.txt`, Selbstprüfung vor dem Schreiben, Ziel `build/<Projekt>-<Spielcode>.xdelta`.
- Gate-Zahlen: Patch nach einer Kartenänderung ≤ 1 KiB, nach Karte, Script und Text ≤ 16 KiB, Erzeugen ≤ 1 s; Rebuild-SHA-1 gleich der `.nds`: Platin DE `d40f19ac24d67c721a4ab25e1c013154443d75b4`, HeartGold DE `13b6853aa93121069df1d17446d3ecb36872aecf`, HeartGold US `4fcded0e2713dc03929845de631d0932ea2b5a37`.
- UI: nur Bausteine und Tokens aus `apps/studio/src/ui/`, keine Hex-Farben, keine Inline-Styles (`scripts/check-styles.sh`, `scripts/check-tokens.sh`), deutsche Sätze, Bezeichner englisch; höchstens eine primäre Taste je Karte.
- Parallel zu 4.2: `crates/g4-kernel/src/build.rs`, `BuildReport` und „Prüfen und Bauen“ bleiben unberührt; in `dispatch.rs` (`COMMANDS`), `lib.rs` (`generate_handler!`), `access.rs` (`COMMAND_KINDS`), `catalog.rs` und `g4k.rs` nur **anhängen**. `export.rs` (Modell-Export über apicula) bleibt unverändert, der Patch bekommt das Modul `patch`.
- Vor jedem Commit: `cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, die Tests der berührten Crates; bei Frontend-Änderungen zusätzlich `cd apps/studio && pnpm test && pnpm build`, `bash scripts/check-styles.sh`, `bash scripts/check-tokens.sh`.
- Nicht in 4.1a: „Auf Original zurücksetzen“, die Liste „geändert gegenüber Original“, Diff gegen die Basis, Plakette „geändert“, Git, No-Intro-Prüfsummen.

## Review Focus

- Ein DSPRE-Ordner, dessen `.g4/base.json` kaputt ist oder aus einer neueren Fassung stammt (`version: 3`): Zuordnen lehnt mit Grund ab und überschreibt nichts, Technik zeigt „Nicht lesbar“ mit Grund, der Build läuft weiter (Task 3 `assign_base_refuses_a_newer_or_broken_mark`, Task 8 `status_of_a_broken_mark_is_unreadable_and_build_still_runs`).
- Die gemerkte Basis-ROM wurde an Ort und Stelle durch eine andere ROM gleicher Größe ersetzt (mtime neu): die Suche hasht neu, verwirft den Eintrag und meldet „Basis-ROM nicht gefunden“, statt einen Patch gegen die falsche ROM zu schreiben (Task 4 `locate_drops_a_known_entry_whose_file_changed`, Task 7 `locate_updates_a_rehashed_entry_and_drops_a_stale_one`).
- Die Selbstprüfung scheitert, während in `build/` noch ein Patch vom letzten Mal liegt: der alte Patch und seine Begleitdatei bleiben Byte für Byte, keine `.tmp`-Datei bleibt liegen (Task 5 `a_failed_self_check_writes_nothing`).
- Ein Agent ordnet über MCP zu (nach der Rückfrage): sein nächster Aufruf fragt nicht noch einmal nach der Freigabe, die Freigabe eines zweiten Projekts in derselben `agents.json` bleibt (Task 7 `move_grant_leaves_other_projects_and_other_base_ids_alone`, Task 8 `assign_writes_the_mark_registers_the_rom_and_moves_the_grant`).
- Projektordner mit `_DSPRE_contents`, Schrägstrich oder Doppelpunkt im Namen, oder nur aus Punkten: Patchname und App-Header bleiben gültige Dateinamen (Task 5 `patch_name_strips_the_dspre_suffix_and_unsafe_characters`).

---

### Task 1: `deny.toml` und Gate-Schritt `cargo deny`

**Files:**
- Create: `deny.toml`
- Modify: `Cargo.lock` (Updates), `scripts/gate.sh` (Schritt nach clippy), `docs/dev-setup.md` (cargo-deny installieren, Gate-Schritt)

**Interfaces:**
- Produces: `deny.toml` im Repo-Wurzelordner; Gate-Schritt mit dem Label `cargo deny (licenses, advisories)`, Befehl `cargo deny --locked check licenses advisories`. Task 2 fügt ein Crate hinzu und lässt diesen Schritt grün.

- [ ] **Step 1: Werkzeug.** `cargo deny --version` muss `0.20.2` zeigen. Fehlt es: den Nutzer um sein OK für den Download bitten, dann `cargo install --locked cargo-deny@0.20.2`.
- [ ] **Step 2: Scheiternde Prüfung anlegen.** `deny.toml` aus `docs/research/2026-10-27-phase-4/gutachten-b-messung/deny.toml` übernehmen (Kopfkommentar auf „Gen4 Studio: license allow list and advisory exceptions, checked by scripts/gate.sh“ ändern), dazu `[graph] all-features = true`. Lauf: `cargo deny --locked check licenses advisories`. Erwartet: licenses ok, advisories FAIL (Gutachten §1.4: acht Befunde, darunter `zip` dreimal über `rotom 0.7.0`, `tokio`, `quick-xml` zweimal über `uxie`/`plist`, `proc-macro-error` über `gtk3-macros`/`glib-macros`, `unsafe-libyaml` über `serde_yaml`). Die tatsächliche Liste mit RUSTSEC-IDs und Kette (`cargo deny check advisories` zeigt sie) notieren.
- [ ] **Step 3: Erst Updates.** Je Befund: gibt es eine semver-verträgliche Fassung mit Fix, dann `cargo update -p <crate>@<alte Version>`; sonst prüfen, ob der direkte Elternteil eine verträgliche neuere Fassung hat, die den Fix zieht (`cargo update -p uxie`, `cargo update -p plist`, `cargo update -p tauri`). Nicht in diesem Task: Major-Sprünge direkter Abhängigkeiten, ein anderer Git-Tag von `rotom` oder `ds-rom`.
- [ ] **Step 4: Rest als begründete Ausnahme.** Für jede übrige ID ein Eintrag `[advisories] ignore = [{ id = "RUSTSEC-JJJJ-NNNN", reason = "…" }, …]`; der Grund nennt Crate und Version, die Kette, warum kein Update geht und warum es hinnehmbar ist (etwa „nur Linux-Ziel (Best-Effort) über tauri → gtk 0.18; unmaintained, keine Schwachstelle“ oder „rotom v0.7.0 pinnt zip x.y; rotom-Update ist ein eigener Schritt, rotom liest keine fremden ZIP-Dateien“). Keine pauschalen Schalter (`unmaintained = "none"` o. ä.).
- [ ] **Step 5: Grün.** `cargo deny --locked check licenses advisories` → `advisories ok, licenses ok`; danach `env -u G4_TEST_PROJECT -u G4_TEST_ROM cargo test --workspace --all-features` grün (die Updates ändern nichts am Verhalten).
- [ ] **Step 6: Gate-Schritt.** In `scripts/gate.sh` nach „cargo clippy“: fehlt `cargo-deny` (`command -v cargo-deny`), `record "cargo deny (licenses, advisories)" "-" "FAILED (cargo-deny fehlt)" 0`, `FAILED=1`, `exit 1` mit dem Installationsbefehl auf stderr; sonst `run_step "cargo deny (licenses, advisories)" "" cargo deny --locked check licenses advisories`. Kopfkommentar des Skripts ergänzen. In `docs/dev-setup.md` die Installation und den Schritt nennen.
- [ ] **Step 7: Commit**

```bash
git add deny.toml Cargo.lock scripts/gate.sh docs/dev-setup.md
git commit -m "build(deny): license and advisory gate with cargo-deny, advisories fixed by updates or excepted with reasons

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

### Task 2: Crate `g4-xdelta` (xdelta3.c mit eigener FFI)

**Files:**
- Create: `crates/g4-xdelta/Cargo.toml`, `crates/g4-xdelta/build.rs`, `crates/g4-xdelta/NOTICE`, `crates/g4-xdelta/vendor/xdelta3/{LICENSE,xdelta3.c,xdelta3.h,xdelta3-*.h}`, `crates/g4-xdelta/csrc/g4_xdelta.c`, `crates/g4-xdelta/csrc/g4_xdelta.h`, `crates/g4-xdelta/src/lib.rs`, `crates/g4-xdelta/tests/roundtrip.rs`
- Modify: `Cargo.toml` (Workspace-Mitglied), `Cargo.lock`

**Interfaces:**
- Produces (C, `csrc/g4_xdelta.h`; Rückgabe 0 = ok, 1 = Ausgabepuffer zu klein, 2 = Fehler mit Text in `*msg`, auch für Längen über `UINT32_MAX`, weil `usize_t` in 3.0.12 `uint32_t` ist):

```c
int g4_xd3_encode(const uint8_t *source, size_t source_len, const uint8_t *target, size_t target_len,
                  const uint8_t *app_header, size_t app_header_len,
                  uint8_t *out, size_t out_cap, size_t *out_len, const char **msg);
int g4_xd3_decode(const uint8_t *source, size_t source_len, const uint8_t *patch, size_t patch_len,
                  uint8_t *out, size_t out_cap, size_t *out_len, const char **msg);
```

- Produces (Rust, `g4_xdelta`):

```rust
pub fn encode(source: &[u8], target: &[u8], app_header: &[u8]) -> Result<Vec<u8>, XdeltaError>;
pub fn decode(source: &[u8], patch: &[u8]) -> Result<Vec<u8>, XdeltaError>;
/// The application header of a VCDIFF patch (header indicator bit 0x04), parsed in Rust.
pub fn app_header(patch: &[u8]) -> Option<&[u8]>;
/// Whether the header announces secondary compression (indicator bit 0x01).
pub fn has_secondary_compression(patch: &[u8]) -> bool;
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XdeltaError { TooLarge, Failed(String) } // Display + std::error::Error von Hand, keine Abhängigkeit
```

- [ ] **Step 1: Quelle und NOTICE.** Die Dateien aus `~/.cargo/registry/src/*/xdelta3-0.1.5/xdelta3/xdelta3/` (siehe Entscheidungen) nach `vendor/xdelta3/` kopieren, unverändert, nur `xdelta3.c`, `xdelta3.h`, `xdelta3-*.h`, `LICENSE`. `NOTICE`: Name „Xdelta 3, version 3.0.12“, „Copyright 2016 Joshua MacDonald“, „Licensed under the Apache License, Version 2.0 (vendor/xdelta3/LICENSE)“, Herkunft (Crate `xdelta3` 0.1.5 von crates.io, VCS-Commit `67be0d3f6498cfc79cbddf7eb42d4f6f95726f02` aus dessen `.cargo_vcs_info.json`, Upstream https://github.com/jmacd/xdelta), „Modifications: none; csrc/g4_xdelta.c is ours (MIT) and includes xdelta3.c“.
- [ ] **Step 2: Scheiternde Tests** in `tests/roundtrip.rs` (Eingaben aus einem kleinen LCG, keine Dateien im Repo):
  - `round_trips_a_changed_megabyte`: 1 MiB Quelle; Ziel = Quelle mit 16 geänderten Bytes und 600 eingefügten Bytes in der Mitte; `decode(src, encode(src, dst, b"")) == dst`, Patch < 4096 Byte.
  - `writes_the_app_header_and_no_secondary_compression`: `app_header(&p) == Some(&b"neu.nds//alt [abc].nds/"[..])`, `!has_secondary_compression(&p)`.
  - `refuses_a_patch_on_the_wrong_source`: Quelle im kopierten Bereich geändert → `decode` ist `Err` (Adler32).
  - `refuses_a_corrupted_patch`: ein Byte im letzten Viertel des Patches gekippt → `Err`.
  - `grows_the_output_for_an_unrelated_target`: 1 MiB Ziel ohne Bezug zur Quelle (Patch größer als der erste Puffer) → Rundweg gleich.
  - `xdelta3_cli_applies_our_patch`: ohne `xdelta3` im PATH (`xdelta3 -V` scheitert) `eprintln!("skipped: …")` und Ende; sonst Dateien in ein `tempdir`, `xdelta3 -d -f -s <src> <patch> <out>`, `out == dst` (Patch mit App-Header).
- [ ] **Step 3: Lauf** `cargo test -p g4-xdelta` → FAIL (Crate fehlt).
- [ ] **Step 4: `build.rs`.** `cc::Build` kompiliert nur `csrc/g4_xdelta.c` mit `include("vendor/xdelta3")`, `opt_level(2)`, `warnings(false)` und den Defines `XD3_ENCODER=1`, `XD3_MAIN=0`, `SECONDARY_DJW=0`, `SECONDARY_FGK=0`, `SECONDARY_LZMA=0`, `EXTERNAL_COMPRESSION=0`, `SHELL_TESTS=0`, `REGRESSION_TEST=0`, `XD3_DEBUG=0`, `XD3_USE_LARGEFILE64=1`, `SIZEOF_SIZE_T` (aus `CARGO_CFG_TARGET_POINTER_WIDTH`), `SIZEOF_UNSIGNED_INT=4`, `SIZEOF_UNSIGNED_LONG` (4 unter Windows, sonst Zeigerbreite), `SIZEOF_UNSIGNED_LONG_LONG=8`, unter Windows `XD3_WIN32=1`; `cargo:rerun-if-changed` für `csrc/` und `vendor/`. Kompiliert eine `SECONDARY_*=0`-Kombination nicht, die Defines des Crates nehmen (`SECONDARY_DJW=1`, `SECONDARY_FGK=1`); die Patches bleiben ohne Zweitkompression, weil kein `XD3_SEC_*`-Flag gesetzt ist (Test `writes_the_app_header_and_no_secondary_compression`).
- [ ] **Step 5: `csrc/g4_xdelta.c`** nach dem Vorbild von `xd3_process_memory`: Kodieren mit `config.flags = XD3_ADLER32`, `winsize`/`sprevsz` wie dort, nach `xd3_config_stream` `xd3_set_appheader` (der Zeiger bleibt bis zum Ende gültig), Quelle als ein Block (`blksize = onblk = max_winsize = source_len`), dann `xd3_process_stream`; `ENOSPC` → 1; Dekodieren mit Flags 0 (Adler32 wird geprüft, der App-Header übersprungen). `xd3_free_stream` auf jedem Weg.
- [ ] **Step 6: `src/lib.rs`.** `extern "C"` für die zwei Funktionen; `encode` beginnt mit einem Puffer von `max(64 KiB, target.len() / 16)` und verdoppelt bei 1 bis `2 × target.len() + 1 MiB`, `decode` mit `source.len() + 1 MiB` bis 1 GiB; Längen über `u32::MAX` → `TooLarge` vor dem C-Aufruf; Rückgabe 2 → `Failed(msg)`. `app_header`/`has_secondary_compression` lesen den VCDIFF-Kopf (`D6 C3 C4 00`, Indikator, optional Sekundär-ID, Code-Tabelle mit Varint-Länge, App-Header mit Varint-Länge). Jede `unsafe`-Stelle mit `// SAFETY:`.
- [ ] **Step 7: Lauf** `cargo test -p g4-xdelta` → PASS; `cargo deny --locked check licenses advisories` bleibt grün (`cc` ist MIT/Apache-2.0, das Crate `MIT AND Apache-2.0`); clippy ohne Warnung.
- [ ] **Step 8: Commit**

```bash
git add Cargo.toml Cargo.lock crates/g4-xdelta
git commit -m "feat(xdelta): vendored xdelta3 3.0.12 (Apache-2.0) behind a small C shim and a safe Rust API

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

### Task 3: Basismarke v2, Prüfen, Zuordnen und Vorschlag (`start.rs`)

**Files:**
- Modify: `crates/g4-kernel/src/start.rs` (Marke, `mark_state`, `check_base`, `assign_base`, `suggest_base`, `rfc3339_utc` wird `pub`, Tests im Modul `create_tests`), `apps/studio/src-tauri/src/mcp/settings.rs` (Test-Literal der Marke), `docs/formats/project_base.md`
- Create: `crates/g4-kernel/tests/base.rs` (gated)

**Interfaces:**
- Produces:

```rust
pub const BASE_MARK_VERSION: u32 = 2;
pub const ORIGIN_EXTRACTED: &str = "extracted";
pub const ORIGIN_DSPRE: &str = "dspre";
pub struct BaseMark {            // Serialize, Deserialize; Felder wie heute, dazu:
    pub template: Option<Template>,   // #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assigned: Option<String>,     // RFC 3339, dito
    /* version, origin, sha1, game_code, title, revision, size, created, studio_version,
       secure_area_crc, secure_area_sha1 unverändert */
}
pub enum MarkState { Missing, Unreadable(String), Newer(u32), Present(BaseMark) }
pub fn mark_state(root: &Path) -> MarkState;
pub fn read_base_mark(root: &Path) -> Option<BaseMark>;        // nur `Present`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)] #[serde(rename_all = "snake_case")]
pub enum BaseWarningKind { TitleDiffers, RevisionDiffers, UnknownDump, Arm7Differs, BannerDiffers }
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BaseWarning { pub kind: BaseWarningKind, pub message: String }
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BaseCheck { pub info: RomInfo, pub warnings: Vec<BaseWarning>, pub refused: Option<String> }
pub fn check_base(root: &Path, nds: &Path) -> Result<BaseCheck, KernelError>;          // schreibt nichts
pub fn assign_base(root: &Path, nds: &Path) -> Result<(BaseCheck, BaseMark), KernelError>; // Refused bei `refused`
pub fn suggest_base(root: &Path) -> Option<PathBuf>;
pub fn rfc3339_utc(secs: u64) -> String;
```

- Texte wörtlich (`{…}` eingesetzt):
  - Ablehnung Spielcode: `Falsches Spiel: die ROM ist {rom}, das Projekt {project}.`
  - `TitleDiffers`: `Der Titel der ROM („{rom}“) weicht vom Projekt ab („{project}“).`
  - `RevisionDiffers`: `Die Version der ROM ({rom}) weicht vom Projekt ab ({project}).` (nur wenn `header.yaml` `rom_version` hat)
  - `UnknownDump`: `Unbekannte oder veränderte ROM; Patches gelten nur für genau diese Datei.`
  - `Arm7Differs`: `Der ARM7-Code der ROM weicht vom Projekt ab; vielleicht ist es die falsche ROM.`
  - `BannerDiffers`: `Das Banner der ROM weicht vom Projekt ab; vielleicht ist es die falsche ROM.`
  - angelegtes Projekt, andere ROM: `Das Projekt wurde aus einer ROM angelegt; seine Basis-ROM steht fest.`
  - kaputte Marke: `Die Basismarke .g4/base.json ist nicht lesbar ({fehler}); entferne sie, dann ordne neu zu.`
  - neuere Marke: `Die Basismarke stammt aus einer neueren Fassung von Gen4 Studio (Version {v}).`

- [ ] **Step 1: Scheiternde Tests** (synthetisch, `test_support::synthetic_project`/`synthetic_rom`/`with_game_code`, beide `CPUE`, Titel „POKEMON PL“):
  - `reads_a_v1_mark_with_its_template` (JSON-Literal v1 → `template == Some(Original)`, `assigned == None`), `reads_a_v2_dspre_mark_without_template`, `mark_state_tells_missing_broken_and_newer`.
  - `create_project_writes_a_v2_mark` (der heutige `create_project_writes_the_base_mark` mit `version == 2`, `template == Some(…)`; `data_only_is_recorded_in_the_mark` und `empty_region_is_named_in_the_mark` vergleichen mit `Some`).
  - `check_base_refuses_another_game_and_writes_nothing` (`with_game_code(rom, "IPKE")` → `refused == Some("Falsches Spiel: die ROM ist IPKE, das Projekt CPUE.")`, kein `.g4/`).
  - `check_base_warns_about_an_unknown_dump_and_a_different_title` (Titel in `header.yaml` geändert → Warnungen `TitleDiffers`, `UnknownDump`, in dieser Reihenfolge, kein ARM7/Banner).
  - `assign_base_writes_a_v2_dspre_mark_with_the_secure_area` (`version 2`, `origin "dspre"`, `template None`, `assigned == Some(created)`, `sha1`, `secure_area_crc`/`_sha1` wie `read_secure_area`; die Datei enthält kein `"template"`).
  - `assign_base_again_with_the_same_rom_keeps_the_mark` (Bytes von `base.json` unverändert).
  - `assign_base_replaces_a_dspre_mark_from_another_rom` (ein Byte der ROM hinter `0x8000` angehängt → andere SHA-1, neues `created`).
  - `assign_base_refuses_another_rom_for_an_extracted_project`.
  - `assign_base_refuses_a_newer_or_broken_mark` (Review Focus: `version: 3` und `{kaputt` → `Refused` mit den Texten, Datei unverändert).
  - `suggest_base_finds_the_sibling_nds_with_the_same_game_code` (Kopie des Projekts als `<tmp>/X_DSPRE_contents`, ROM als `<tmp>/X.nds`), `suggest_base_ignores_another_game_or_a_folder_without_the_suffix`.
  - `restore_header_crcs_works_with_a_dspre_mark` (zuordnen, CRC in einer Kopie der ROM auf 0 setzen, `restore_header_crcs` → `true`, CRC wie im Original).
- [ ] **Step 2: Gated Test** `tests/base.rs`: `base_assign_on_the_vanilla_folder_has_no_warning_and_rebuilds_the_nds`: `vanilla_project()` und `suggest_base(&orig)` (fehlt eins: `eprintln!("skipped: …")`), `vanilla_copy()`, `assign_base(copy, nds)` → `warnings` leer, `info.known_base` gesetzt; `build::build` nach `<tmp>/out.nds`; `inspect_rom(out).sha1 == check.info.sha1`; druckt `rebuild <game_code> sha1=<…> equal`.
- [ ] **Step 3: Lauf** `cargo test -p g4-kernel --all-features start::` → FAIL.
- [ ] **Step 4: Umsetzen** in `start.rs`. `check_base`: `inspect_rom`, Spielcode gegen `Project::open(root)?.game_code`, Titel/`rom_version` aus `header.yaml`, `known_base`, dann ARM7 (`arm7/arm7.bin` gegen `raw::Rom::from_file(nds)` ARM7-Bytes) und Banner-Titel (`banner/banner.yaml` gegen den Banner der ROM), beides nur, wenn beide Seiten lesbar sind. `assign_base`: `mark_state` zuerst (Ablehnungen wie oben), dann `check_base`, dann `write_base_mark` mit Secure Area aus `read_secure_area`. `suggest_base` liest nur die ersten 0x200 Byte.
- [ ] **Step 5: Lauf** → PASS, auch `cargo test -p studio` (Literal in `mcp/settings.rs`). Gated einmal auf Platin DE: `G4_TEST_PROJECT="<Roms>/Pokemon - Platin-Edition (Germany)_DSPRE_contents" cargo test -p g4-kernel --all-features --test base -- --nocapture` → `equal`.
- [ ] **Step 6: Doku** `docs/formats/project_base.md`: Version 2, `origin` `"dspre"`, `template` optional, `assigned`; Abschnitt „Zuordnen (DSPRE-Ordner)“ mit Regeln, Warnungen, Ablehnungen, Vorschlag; im Abschnitt „Byte-gleicher Neubau“ die DSPRE-Ordner mit Marke.
- [ ] **Step 7: Commit** `feat(kernel): base mark v2 with origin dspre, check and assign a base ROM to a DSPRE folder` (mit Trailer).

### Task 4: Kernel-Modul `base` (Basis-ROM finden und lesen)

**Files:**
- Create: `crates/g4-kernel/src/base.rs` (+ Tests im Modul)
- Modify: `crates/g4-kernel/src/lib.rs` (`pub mod base;`), `crates/g4-kernel/src/project.rs` (`header_table_offset` zusätzlich als freie `pub(crate) fn header_table_offset_for(game_code: &str) -> Result<u64, KernelError>`), `Cargo.toml` (`[profile.dev.package.sha1] opt-level = 3`, wie bei ds-rom), `crates/g4-kernel/tests/base.rs` (gated)

**Interfaces:**
- Consumes: Task 3 `BaseMark`, `suggest_base`, `inspect_rom`.
- Produces:

```rust
pub const BASE_ROM_DIR_VAR: &str = "G4_BASE_ROM_DIR";
pub fn env_base_dir() -> Option<PathBuf>;
pub fn mtime_ms(meta: &std::fs::Metadata) -> u64;
#[derive(Debug, Clone, PartialEq, Eq)] pub struct KnownRom { pub path: PathBuf, pub size: u64, pub mtime_ms: u64 }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)] #[serde(rename_all = "snake_case")]
pub enum BaseSource { Explicit, Known, Sibling, Dir }
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Located { pub path: PathBuf, pub source: BaseSource, pub size: u64, pub mtime_ms: u64, pub hashed: bool }
pub struct Search<'a> {
    pub explicit: Option<&'a Path>, pub known: Option<&'a KnownRom>, pub root: Option<&'a Path>,
    pub dir: Option<&'a Path>, pub skip: &'a dyn Fn(&Path) -> bool,
}
pub fn locate(mark: &BaseMark, search: &Search<'_>) -> Result<Option<Located>, KernelError>;
pub fn base_label(mark: &BaseMark) -> String;  // Label aus base_roms.yaml, sonst "<title> (<game_code>)"
pub fn other_rom(found_sha1: &str, mark: &BaseMark) -> String;
// "Das ist eine andere ROM (SHA-1 {found8}…), gesucht ist {label} ({wanted8}…)."
pub fn missing_text(mark: &BaseMark) -> String;
// "Basis-ROM nicht gefunden: {label} ({sha8}…)." — dieselbe Meldung in g4k (Task 6) und App (Task 8)
pub struct BaseRom { /* raw::Rom, sha1, game_code, ArchiveMap; je Aufruf gelesen, nie gecacht */ }
impl BaseRom {
    pub fn open(path: &Path, expected_sha1: Option<&str>) -> Result<BaseRom, KernelError>; // Refused(other_rom)
    pub fn sha1(&self) -> &str;
    pub fn game_code(&self) -> &str;
    pub fn bytes(&self) -> &[u8];
    pub fn file(&self, inside: &str) -> Result<Option<Vec<u8>>, KernelError>; // Pfad ohne "files/"
    pub fn narc(&self, kind: AssetKind) -> Result<Option<Narc>, KernelError>; // None: Spiel hat kein Archiv
    pub fn member(&self, kind: AssetKind, index: u16) -> Result<Option<Vec<u8>>, KernelError>;
    pub fn arm9(&self) -> Result<Vec<u8>, KernelError>;                 // entpackt (HGSS: BLZ)
    pub fn header_table(&self) -> Result<Vec<uxie::MapHeader>, KernelError>;
}
```

- `locate`: (1) `explicit`: muss existieren, wird gehasht; andere SHA-1 → `Err(Refused(other_rom))`. (2) `known`: Größe = `mark.size` und mtime und Größe wie gemerkt → `Located { source: Known, hashed: false }`; sonst bei gleicher Größe hashen, gleich → `hashed: true`, ungleich → weiter. (3) `root` gesetzt und `suggest_base(root)` vorhanden, gleiche Größe und SHA-1 → `Sibling`. (4) `dir`: `read_dir` nicht rekursiv, Endung `.nds` ohne Groß/klein, Größe gleich, nach Namen sortiert hashen → `Dir`. (5) `Ok(None)`. `skip(path)` gilt für jeden Kandidaten außer `explicit`. `BaseRom::file` parst `FileSystem` je Aufruf neu (3 ms) statt einer selbstreferenziellen Struktur; `header_table` nimmt Offset aus `header_table_offset_for` und die Anzahl aus der Länge der `mapname.bin` der Basis (derselbe Pfad wie `Project::mapname_path`, ohne `files/`), gelesen aus `arm9()`.

- [ ] **Step 1: Scheiternde Tests** (synthetische ROMs in `tempdir`, Varianten durch ein geändertes Byte hinter `0x8000`): `locate_takes_the_explicit_rom_and_refuses_another_one`, `locate_trusts_a_known_entry_with_the_same_size_and_mtime` (`hashed == false`), `locate_hashes_a_known_entry_whose_mtime_changed` (Datei neu geschrieben, gleicher Inhalt → `hashed == true`), `locate_drops_a_known_entry_whose_file_changed` (Review Focus: gleiche Größe, anderer Inhalt → `None`), `locate_finds_the_sibling_nds`, `locate_scans_the_dir_by_size_and_sha1` (drei Dateien: andere Größe, gleiche Größe und andere SHA-1, Treffer), `locate_skips_what_the_caller_excludes`, `base_rom_open_refuses_another_sha1`, `base_label_falls_back_to_title_and_game_code`.
- [ ] **Step 2: Gated Tests** in `tests/base.rs`: `base_rom_members_equal_the_vanilla_folders` (je Art aus `archives.kinds()` mit vorhandenem `unpacked/`-Ordner: `base.narc(kind)?.members == build::pack_dir(dir)?.members`), `base_rom_header_table_equals_the_vanilla_project` (`== Project::read_headers()`); Basis über `suggest_base(vanilla_project())`.
- [ ] **Step 3: Lauf** `cargo test -p g4-kernel --all-features base` → FAIL.
- [ ] **Step 4: Umsetzen** wie oben.
- [ ] **Step 5: Lauf** → PASS; gated einmal auf HeartGold DE (komprimiertes ARM9).
- [ ] **Step 6: Commit** `feat(kernel): module base finds the base ROM by SHA-1 and reads files, members, ARM9 and headers from it` (mit Trailer).

### Task 5: Kernel-Modul `patch`

**Files:**
- Create: `crates/g4-kernel/src/patch.rs` (+ Tests im Modul), `docs/formats/xdelta_patch.md`
- Modify: `crates/g4-kernel/src/lib.rs` (`pub mod patch;`), `crates/g4-kernel/Cargo.toml` (`g4-xdelta = { path = "../g4-xdelta" }`), `crates/g4-kernel/tests/base.rs` (gated)

**Interfaces:**
- Consumes: Task 2 `g4_xdelta::{encode, decode}`; Task 3 `read_base_mark`; Task 4 `BaseRom`, `base_label`; `build::build` und `BuildReport` (unverändert). `build_and_create` liest die Marke selbst (ohne Marke `KernelError::Refused`).
- Produces:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchMeta { pub name: String, pub game_code: String, pub base_label: String, pub studio_version: String }
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchTargets { pub rom: PathBuf, pub patch: PathBuf }   // build/<Name>.nds, build/<Name>.xdelta
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PatchReport {
    pub patch: PathBuf, pub notes: PathBuf, pub rom: PathBuf, pub size: u64,
    pub base_sha1: String, pub target_sha1: String, pub encode_ms: u64, pub verify_ms: u64,
}
pub fn patch_name(project: &Project) -> String;
pub fn default_targets(project: &Project) -> PatchTargets;
pub fn meta(project: &Project, base: &BaseMark) -> PatchMeta;   // studio_version = CARGO_PKG_VERSION
pub fn notes_path(patch: &Path) -> PathBuf;                     // "<Name>.xdelta" → "<Name>.xdelta.txt"
pub fn app_header(meta: &PatchMeta, base_sha1: &str) -> String;
pub fn notes(meta: &PatchMeta, base_sha1: &str, target_sha1: &str) -> String;
pub fn create_patch(base: &BaseRom, built_rom: &Path, out: &Path, meta: &PatchMeta) -> Result<PatchReport, KernelError>;
pub(crate) fn create_patch_with(base: &BaseRom, built_rom: &Path, out: &Path, meta: &PatchMeta,
    verify: &dyn Fn(&[u8], &[u8]) -> Result<Vec<u8>, KernelError>) -> Result<PatchReport, KernelError>;
/// For g4k and the gated tests: builds to `default_targets().rom`, then `create_patch` to `out` or the default.
pub fn build_and_create(project: &Project, base: &BaseRom, out: Option<&Path>) -> Result<(BuildReport, PatchReport), KernelError>;
pub const SELF_CHECK_FAILED: &str =
    "Selbstprüfung fehlgeschlagen: der Patch ergibt auf der Basis-ROM nicht die gebaute ROM; nichts geschrieben.";
```

- App-Header: `format!("{name}.nds//{label} [{sha1}].nds/")`, `label` mit derselben Bereinigung wie `<Projekt>`. Begleitdatei wörtlich:

```text
Gen4 Studio: Patch für {base_label}
Spiel: {game_code}
Basis-ROM SHA-1: {base_sha1}
Ziel-ROM SHA-1: {target_sha1}
Studio-Version: {studio_version}

Anwenden: mit xdelta3 („xdelta3 -d -s <Basis-ROM> {name}.xdelta {name}.nds“) oder mit Rom Patcher JS, auf genau die Basis-ROM mit dieser SHA-1.
```

- `create_patch`: gebaute ROM lesen, SHA-1; `encode(base.bytes(), ziel, app_header)` (Zeit → `encode_ms`); `verify` = `decode` + SHA-1-Vergleich (Zeit → `verify_ms`), ungleich → `KernelError::Build(SELF_CHECK_FAILED)` ohne zu schreiben; dann Patch und Begleitdatei je über `.<Datei>.tmp` und `rename`.

- [ ] **Step 1: Scheiternde Tests** (synthetische ROM als Basis, eine geänderte Kopie als „gebaute ROM“):
  - `patch_name_strips_the_dspre_suffix_and_unsafe_characters` (Review Focus: `Mein/Hack:Neu_DSPRE_contents` → `Mein_Hack_Neu-CPUE`; ` .Rom. ` → `Rom-CPUE`; `...` → `Projekt-CPUE`; `Pokemon - Platin-Edition (Germany)_DSPRE_contents` → `Pokemon - Platin-Edition (Germany)-CPUE`).
  - `app_header_names_target_and_base_with_the_full_sha1`.
  - `notes_carry_both_sha1_and_the_instruction` (genau der Text oben).
  - `create_patch_writes_patch_and_notes_that_apply` (`decode(basis, Datei) == gebaute ROM`, `report.size` = Dateigröße, `g4_xdelta::app_header` = `app_header(...)`, keine Zweitkompression).
  - `a_failed_self_check_writes_nothing` (Review Focus: alter Patch und alte Begleitdatei vorher angelegt; `create_patch_with` mit `verify`, das andere Bytes liefert → `Err(Build(SELF_CHECK_FAILED))`, beide Dateien Byte für Byte wie vorher, keine `.tmp`-Datei im Ordner).
  - `create_patch_replaces_an_old_patch` (neuer Inhalt, keine `.tmp`-Datei).
- [ ] **Step 2: Gated Test** in `tests/base.rs`: `patch_after_map_and_after_map_script_text_is_small_and_applies`: `vanilla_copy()`, Basis über `suggest_base(vanilla_project())`, `assign_base`; Kartenänderung (Byte `0x20` von `member_path(AssetKind::Map, 3)` mit `0x80` gekippt, wie Gutachten `edit_map`) → `build_and_create` → `size ≤ 1024`, `encode_ms ≤ 1000`; dann Text (Archiv 0: `TextArchiveDoc::from_member`, Nachricht 0 + `" Studio"`, nach `expanded/textArchives/0000.json` mit `to_json`, wie Gutachten `edit_text`) und Script (an `member_path(AssetKind::Script, 0)` die Bytes `[0x02, 0x00]` anhängen) → `build_and_create` → `size ≤ 16384`, `encode_ms ≤ 1000`; nach jedem Patch, wenn `xdelta3` im PATH ist, `xdelta3 -d -f -s <nds> <patch> <out>` und SHA-1 gleich `report.target_sha1`. Druckt `patch-sizes <game_code> map=<n> all=<n> encode_ms=<a>/<b> verify_ms=<a>/<b>` für die Tabelle im Gate.
- [ ] **Step 3: Lauf** → FAIL; **Step 4: Umsetzen**; **Step 5: Lauf** → PASS; gated einmal auf Platin DE mit `--nocapture`.
- [ ] **Step 6: Doku** `docs/formats/xdelta_patch.md`: Ziel und Name, VCDIFF ohne Zweitkompression mit Adler32, App-Header, Begleitdatei, Selbstprüfung, atomares Schreiben, Gegenprobe mit `xdelta3 -d -s`, Stolperfalle `xdelta3 -d` ohne `-s` (sucht die Quelle aus dem App-Header).
- [ ] **Step 7: Commit** `feat(kernel): xdelta patch against the base ROM with app header, notes and a self-check before writing` (mit Trailer).

### Task 6: `g4k base assign`, `g4k base check`, `g4k patch create`

**Files:**
- Modify: `crates/g4-kernel/src/bin/g4k.rs` (Unterbefehle `Base` und `Patch` am Ende von `Command`), `crates/g4-kernel/tests/cli.rs`

**Interfaces:**
- Consumes: Task 3 (`check_base`, `assign_base`, `suggest_base`, `mark_state`), Task 4 (`locate`, `env_base_dir`, `BaseRom`), Task 5 (`build_and_create`, `meta`).
- Produces (Befehlszeile):
  - `g4k base check <project> [--rom <nds>] [--json]`: ohne `--rom` die Zeilen `mark=none|extracted|dspre|unreadable|newer`, `sha1=<…>` (mit Marke), `suggestion=<pfad>` (ohne Marke, wenn vorhanden), `base=<pfad> (<explicit|sibling|dir>)` oder `base=missing` (Suche mit `dir = env_base_dir()`, `root = project`); mit `--rom` das `BaseCheck` (je Warnung `warning: <message>`, `--json` als JSON), Exit 1 bei `refused`.
  - `g4k base assign <project> <nds>`: `warning: <message>` je Warnung, dann `assigned sha1=<…> label=<…>`; Ablehnung: Meldung auf stderr, Exit 1.
  - `g4k patch create <project> [--base-rom <nds>] [--out <datei>] [--json]`: ohne Marke Exit 1 `no base mark: run g4k base assign first`; Basis über `--base-rom`, Nachbar, `G4_BASE_ROM_DIR`, sonst Exit 1 mit `base::missing_text(&mark)`; dann `build_and_create`; Ausgabe `patch=… size=… encode_ms=… verify_ms=… base=… target=…` oder `PatchReport` als JSON.

- [ ] **Step 1: Scheiternde Tests** (`assert_cmd`, synthetisch): `cli_base_assign_writes_a_dspre_mark`, `cli_base_assign_refuses_another_game_with_exit_1` (stderr enthält „Falsches Spiel“), `cli_base_check_names_the_suggestion` (Ordner `X_DSPRE_contents` neben `X.nds`), `cli_base_check_with_rom_prints_the_warnings_as_json`, `cli_patch_create_without_a_mark_fails`, `cli_patch_create_with_a_missing_base_names_it` (Marke da, keine ROM, `G4_BASE_ROM_DIR` leer gesetzt).
- [ ] **Step 2: Lauf** `cargo test -p g4-kernel --all-features --test cli cli_base cli_patch` → FAIL; **Step 3: Umsetzen**; **Step 4: Lauf** → PASS.
- [ ] **Step 5: Commit** `feat(g4k): base assign, base check and patch create` (mit Trailer).

### Task 7: App-Einstellung `base-roms.json`, Konfiguration, Freigabe umschreiben

**Files:**
- Create: `apps/studio/src-tauri/src/base_roms.rs` (+ Tests)
- Modify: `apps/studio/src-tauri/src/state.rs` (`pub base_roms: BaseRoms` in `AppState`), `src/lib.rs` (`mod base_roms;`, in `setup_agents` daneben `state.base_roms.configure(dir)` mit demselben App-Datenordner), `src/bridge/http.rs` (`config.app_data` konfiguriert auch `base_roms`), `src/mcp/mod.rs` (`--base-rom <PATH>`, nur mit `--project`, in `headless_flag`), die Stelle, an der `Toolbox::open` den `AppState` anlegt (`set_explicit`), `src/mcp/settings.rs` (`AgentSettings::move_grant`), `src/mcp/hub.rs` (`AgentHub::move_grant`), `src/commands/policy.rs` (`is_original` wird `pub(crate)`, neu `pub(crate) fn originals_dir() -> Option<PathBuf>`, kanonisches `G4_ROMS_DIR`), `src/commands/start.rs` und `commands/dispatch.rs` (`create_project_impl(state: &AppState, …)` trägt die ROM nach Erfolg ein)

**Interfaces:**
- Consumes: Task 4 `locate`, `Search`, `KnownRom`, `Located`, `BaseSource`, `env_base_dir`, `mtime_ms`; Task 3 `rfc3339_utc`, `RomInfo`.
- Produces:

```rust
pub const BASE_ROMS_FILE: &str = "base-roms.json";
pub const BASE_ROMS_VERSION: u32 = 1;
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BaseRomEntry { pub sha1: String, pub path: String, pub size: u64, pub mtime_ms: u64,
                          pub game_code: String, pub label: Option<String>, pub verified: String }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)] #[serde(default)]
pub struct BaseRomSettings { pub version: u32, pub roms: Vec<BaseRomEntry> }
impl BaseRomSettings {
    pub fn load(dir: &Path) -> BaseRomSettings;                // fehlend oder kaputt: leer
    pub fn save(&self, dir: &Path) -> Result<(), String>;      // mcp::settings::write_private
    pub fn find(&self, sha1: &str) -> Option<&BaseRomEntry>;
    pub fn upsert(&mut self, entry: BaseRomEntry);             // ersetzt den Eintrag derselben SHA-1
    pub fn remove(&mut self, sha1: &str);
}
#[derive(Default)] pub struct BaseRoms { /* Mutex<Inner { dir, settings, explicit, search_dir, originals }> */ }
impl BaseRoms {
    pub fn configure(&self, dir: Option<PathBuf>);  // lädt die Datei, liest G4_BASE_ROM_DIR und originals_dir()
    pub(crate) fn configure_with(&self, dir: Option<PathBuf>, search_dir: Option<PathBuf>, originals: Option<PathBuf>);
    pub fn set_explicit(&self, path: Option<PathBuf>);
    pub fn register(&self, path: &Path, info: &RomInfo) -> Result<(), String>;
    /// Searches as `base::locate` (explicit, known entry, sibling of `root`, search dir; originals skipped);
    /// a rehashed known entry gets its new mtime, a known entry that no longer matches is removed,
    /// a hit from sibling or dir is registered. Saves when anything changed.
    pub fn locate(&self, root: &Path, mark: &BaseMark) -> Result<Option<Located>, KernelError>;
    pub fn settings(&self) -> BaseRomSettings;
}
impl AgentSettings { pub fn move_grant(&mut self, from: &ProjectKey, to: &ProjectKey) -> bool; }
impl AgentHub { pub fn move_grant(&self, from: &ProjectKey, to: ProjectKey) -> Result<(), String>; }
```

- Datei, Beispiel:

```json
{
  "version": 1,
  "roms": [
    { "sha1": "d40f19ac24d67c721a4ab25e1c013154443d75b4", "path": "/…/Pokemon - Platin-Edition (Germany).nds",
      "size": 134217728, "mtime_ms": 1761400000000, "game_code": "CPUD",
      "label": "Platin DE (Testprojekt)", "verified": "2026-10-27T10:00:00Z" }
  ]
}
```

- [ ] **Step 1: Scheiternde Tests:** `base_roms.rs`: `missing_or_broken_file_gives_the_defaults`, `save_is_private_and_leaves_no_temp_file` (Modus 0600), `upsert_replaces_the_entry_of_the_same_sha1`, `register_writes_size_mtime_and_label`, `locate_updates_a_rehashed_entry_and_drops_a_stale_one` (Review Focus), `locate_registers_a_sibling_hit`, `locate_skips_an_original_under_g4_roms_dir` (`configure_with(…, originals = Some(<tmp>/Roms))`, Eintrag zeigt auf `<tmp>/Roms/x.nds` → `None`; unter `<tmp>/Roms/_testkopien/` → gefunden); `settings.rs`: `move_grant_rewrites_the_grant_of_the_old_key`, `move_grant_leaves_other_projects_and_other_base_ids_alone` (Review Focus), `move_grant_without_a_grant_changes_nothing`; `mcp/mod.rs`: `base_rom_flag_needs_project`; `bridge/http.rs`: `app_data_configures_the_base_roms`.
- [ ] **Step 2: Lauf** `cargo test -p studio --all-features base_roms move_grant base_rom_flag` → FAIL; **Step 3: Umsetzen**; **Step 4: Lauf** → PASS.
- [ ] **Step 5: Commit** `feat(studio): base-roms.json in the app data, --base-rom and G4_BASE_ROM_DIR headless, grants move with the base id` (mit Trailer).

### Task 8: Befehle `base_status`, `check_base`, `assign_base`, `register_base_rom`, `create_patch` und MCP

**Files:**
- Create: `apps/studio/src-tauri/src/commands/base.rs` (+ Tests)
- Modify: `src/commands/mod.rs`, `src/lib.rs` (fünf Handler am Ende von `generate_handler!`), `src/commands/dispatch.rs` (`COMMANDS` anhängen, Arme, Beispielargumente im Test aller Befehle), `src/mcp/access.rs` (`COMMAND_KINDS` anhängen, `irreversible_ones_are_the_decided_groups`, Liste der App-only-Befehle), `src/mcp/catalog.rs` (vier Werkzeuge + Tests), `src/mcp/tools.rs` (`NOT_IN_BATCH`), `docs/mcp.md` (Werkzeuge, `--base-rom`, `G4_BASE_ROM_DIR`)

**Interfaces:**
- Consumes: Task 7 `AppState::base_roms`, `AgentHub::move_grant`, `ProjectKey::of`; Task 3/4/5; `build_rom_impl` (unverändert) und `BuildDto`.
- Produces:

```rust
#[derive(Debug, Serialize)] #[serde(tag = "state", rename_all = "snake_case")]
pub enum BaseStatusDto {
    Unassigned { suggestion: Option<String> },
    Missing { mark: BaseMark, label: String, reason: String },
    Found { mark: BaseMark, label: String, path: String, source: BaseSource },
    Unreadable { reason: String },
}
#[derive(Debug, Serialize)] pub struct PatchDto { #[serde(flatten)] pub report: PatchReport, pub build: BuildDto }
pub fn base_status_impl(state: &AppState) -> Result<BaseStatusDto, CmdError>;
pub fn check_base_impl(state: &AppState, nds: &str) -> Result<BaseCheck, CmdError>;
pub fn assign_base_impl(state: &AppState, nds: &str) -> Result<BaseStatusDto, CmdError>;   // Marke, Freigabe, Eintrag
pub fn register_base_rom_impl(state: &AppState, path: &str) -> Result<BaseStatusDto, CmdError>; // nur gleiche SHA-1
pub fn create_patch_impl(state: &AppState) -> Result<PatchDto, CmdError>;
pub const NO_MARK: &str = "Diesem Projekt ist keine Basis-ROM zugeordnet.";
// `Missing.reason` ist `base::missing_text(&mark)` (Task 4).
```

- Tauri-Befehle (`base_status`, `create_patch` und `assign_base` hashen oder bauen: `async` mit `spawn_blocking`), Argumente `{ nds }` bzw. `{ path }`. `create_patch_impl`: Status muss `Found` sein, sonst `CmdError { code: "base_missing", message: <reason oder NO_MARK> }`; dann `build_rom_impl(state, default_targets.rom)`, `BaseRom::open(path, Some(&mark.sha1))`, `create_patch`. `assign_base_impl`: alter `ProjectKey`, `start::assign_base`, neuer Schlüssel, `agents.move_grant`, `base_roms.register`, neuer Status. Dispatch: `check_base`/`assign_base` erlauben den Pfad, der kanonisch gleich `suggest_base(projekt)` ist und kein Original, sonst `policy.read_path(&a.nds)`; `create_patch` ruft vorher `policy.check_output` für das Patch-Ziel.
- Zugriff: `("base_status", Read)`, `("check_base", Read)`, `("assign_base", Irreversible("Basis-ROM zuordnen"))`, `("register_base_rom", APP_ONLY)`, `("create_patch", Irreversible("Patch erstellen"))`. Werkzeuge (englisch): `base_status` („Base ROM status“), `check_base` und `assign_base` mit Schema `{ nds: text("path of the .nds; relative to the project folder; headless only inside the project, an --allow-dir folder or the .nds next to a *_DSPRE_contents folder") }` (Pflicht), `create_patch` ohne Argumente; die Beschreibungen der zwei unumkehrbaren nennen `--allow-irreversible` und `needs_confirmation`.

- [ ] **Step 1: Scheiternde Tests** (`commands/base.rs`, synthetisches Projekt als `<tmp>/X_DSPRE_contents` mit `<tmp>/X.nds`, geöffnet mit `open_project_impl`):
  - `status_without_a_mark_suggests_the_sibling_nds`
  - `check_refuses_another_game_without_writing`
  - `assign_writes_the_mark_registers_the_rom_and_moves_the_grant` (Review Focus: Freigabe `(pfad, None) = Write` vorher, nachher `grant_for(&ProjectKey::of(root)) == Some(Write)`, Freigabe eines zweiten Projekts unverändert, `base_roms.settings().find(sha1)` gesetzt, Status `Found`)
  - `register_refuses_another_rom_and_keeps_the_entry` (Meldung aus `base::other_rom`)
  - `status_names_a_missing_base_with_label_and_sha1` (Marke da, ROM verschoben)
  - `status_of_a_broken_mark_is_unreadable_and_build_still_runs` (Review Focus; `build_rom_impl` antwortet wie ohne Marke, nicht mit einem Basis-Fehler)
  - `create_patch_without_a_base_answers_base_missing`
  - `access.rs`: die zwei neuen unumkehrbaren in `irreversible_ones_are_the_decided_groups`, `register_base_rom` unter den App-only; `catalog.rs`: Lese- und Schreibwerkzeuge der Entscheidung, `irreversible_tools_name_the_headless_switch` mit `assign_base` und `create_patch`; `tools.rs`: `batch_refuses_assign_base_and_create_patch`; `dispatch.rs`: `assign_base_allows_the_suggested_sibling_and_refuses_other_outside_paths`.
- [ ] **Step 2: Lauf** `cargo test -p studio --all-features` → FAIL; **Step 3: Umsetzen**; **Step 4: Lauf** → PASS, dazu `cargo test -p studio --test mcp_stdio`.
- [ ] **Step 5: Doku** `docs/mcp.md`: Zeilen der vier Werkzeuge in der Werkzeugtabelle, `--base-rom` in der Schaltertabelle, `G4_BASE_ROM_DIR` unter Umgebung, Rückfrage „Basis-ROM zuordnen“ und „Patch erstellen“.
- [ ] **Step 6: Commit** `feat(studio): base status, assign base ROM and create patch as commands and MCP tools with confirmation` (mit Trailer).

### Task 9: Technik › Veröffentlichen und Design-System

**Files:**
- Create: `apps/studio/src/api.base.ts` (+ `api.base.test.ts`), `apps/studio/src/technik/useBase.ts` (+ `useBase.test.tsx`), `apps/studio/src/technik/PublishSurface.tsx` (+ `PublishSurface.module.css`, `PublishSurface.test.tsx`), `design-system/openpencil/beispiele/veroeffentlichen.body.js`, `design-system/openpencil/NACHTRAG-4-1a.md`
- Modify: `apps/studio/src/api.ts` (`export * from "./api.base";`), `apps/studio/src/ui/primitives/icons.ts` (`publish: UploadSimple`), `apps/studio/src/technik/TechnikArea.tsx` (+ Test: Eintrag, Weiche, Untertitel; `WithPatch` wird `WithProject` mit `useHeaderPatch` und `useBase`), `design-system/README.md` (Abschnitt „Nachtrag 4.1a“), `design-system/Audit-Design-System.md` (Zeile „Veröffentlichen“: teilweise, 4.1a)

**Interfaces:**
- Consumes: Task 8 (Befehle und DTOs), `platform/dialogs.ts` `openPath`, `Dialog`, `Notice`, `Badge`, `Button`, `EmptyState` aus `ui/primitives`, `Surface`/Karten-Klassen wie `PatchSurface`.
- Produces (TS):

```ts
export interface BaseMarkDto { version: number; origin: "extracted" | "dspre"; sha1: string; game_code: string; title: string;
  revision: number; size: number; template?: Template; created: string; assigned?: string; studio_version: string;
  secure_area_crc: number; secure_area_sha1: string }
export type BaseSource = "explicit" | "known" | "sibling" | "dir";
export type BaseStatus =
  | { state: "unassigned"; suggestion: string | null }
  | { state: "missing"; mark: BaseMarkDto; label: string; reason: string }
  | { state: "found"; mark: BaseMarkDto; label: string; path: string; source: BaseSource }
  | { state: "unreadable"; reason: string };
export interface BaseWarning { kind: "title_differs" | "revision_differs" | "unknown_dump" | "arm7_differs" | "banner_differs"; message: string }
export interface BaseCheck { info: RomInfo; warnings: BaseWarning[]; refused: string | null }
export interface PatchDto { patch: string; notes: string; rom: string; size: number; base_sha1: string; target_sha1: string;
  encode_ms: number; verify_ms: number; build: BuildReport }
export const baseStatus: () => Promise<BaseStatus>;
export const checkBase: (nds: string) => Promise<BaseCheck>;
export const assignBase: (nds: string) => Promise<BaseStatus>;
export const registerBaseRom: (path: string) => Promise<BaseStatus>;
export const createPatch: () => Promise<PatchDto>;
export function formatBytes(n: number): string; // "397 Byte", "4,7 KiB", "1,2 MiB" (Komma, eine Stelle)
// technik/useBase.ts
export interface BaseApi { status: BaseStatus | null; error: string | null; busy: "check" | "assign" | "register" | "patch" | null;
  check: (nds: string) => Promise<BaseCheck | null>; assign: (nds: string) => Promise<void>;
  register: (path: string) => Promise<void>; createPatch: () => Promise<void>; lastPatch: PatchDto | null; refresh: () => Promise<void> }
export function useBase(): BaseApi;   // liest den Status einmal je Projekt, wenn Technik sich zeigt
export function PublishSurface(props: { base: BaseApi | null; onOpenProject: () => void }): JSX.Element;
```

- `TechnikEntry` wird `"bauen" | "veroeffentlichen" | "patches" | "emulator" | "werkzeuge"`; Liste: „Prüfen und Bauen“, „Veröffentlichen“ (Symbol `publish`), „Patches“, „Emulator“, „Werkzeuge“. Die Fläche trägt `data-technik="veroeffentlichen"`, die Karten `data-base-state=<state>` und `data-patch-card`.
- Texte wörtlich:

| Stelle | Text |
|---|---|
| Untertitel der Liste | ohne Projekt „Kein Projekt geöffnet“; lädt „Wird gelesen …“; `unassigned` „Keine Basis-ROM zugeordnet“; `missing` „Basis-ROM fehlt“; `unreadable` „Basismarke nicht lesbar“; `found` „Bereit für einen Patch“, nach einem Patch „Patch erstellt“ |
| ohne Projekt | `EmptyState` „Kein Projekt geöffnet.“ / „Öffne ein Projekt, dann erstellt Technik hier seinen Patch.“ / „Projekt öffnen“ |
| Karte „Basis-ROM“, Satz | „Ein Patch beschreibt den Unterschied zu deiner Basis-ROM. Die ROM bleibt außerhalb des Projekts; Gen4 Studio merkt sich nur, wo sie liegt.“ |
| Plakette | `found` Erfolg „Gefunden“; `missing` Warnung „Fehlt“; `unassigned` neutral „Nicht zugeordnet“; `unreadable` Fehler „Nicht lesbar“ |
| `found`, Angaben | „ROM“ = Label; „SHA-1“ = 8 Stellen in Mono, die volle als `title`; „Datei“ = Pfad in Mono, gekürzt, voll als `title` |
| `unassigned` mit Vorschlag | `Notice` Info „Neben dem Projekt liegt „{Dateiname}“.“, primär „Zuordnen…“, sekundär „Andere Datei wählen…“ |
| `unassigned` ohne Vorschlag | „Diesem Projekt ist keine Basis-ROM zugeordnet.“, primär „Basis-ROM zuordnen…“ |
| `missing` | `Notice` Warnung mit `reason`, primär „Basis-ROM suchen…“ |
| `unreadable` | `Notice` Fehler mit `reason` |
| Dateidialog | Titel „Basis-ROM wählen“, Filter `{ name: "DS-ROM", extensions: ["nds"] }` |
| Dialog Zuordnen | Titel „Basis-ROM zuordnen?“; Beschreibung „{title} · {game_code} · SHA-1 {sha8}“; darunter „Bekannte Basis-ROM: {known_base}“, wenn gesetzt; je Warnung ein `Notice` Warnung; Satz „Schreibt die Basismarke .g4/base.json ins Projekt.“ mit `Badge kind="irreversible"`; Tasten „Abbrechen“, „Zuordnen“ (primär); bei `refused` statt Warnungen ein `Notice` Fehler mit dem Satz und nur „Schließen“ |
| nach dem Zuordnen | `Notice` Erfolg „Basis-ROM zugeordnet: {label}.“ |
| Karte „Patch“, Satz | „Der Patch enthält nur deine Änderungen. Wer dieselbe Basis-ROM hat, wendet ihn mit xdelta3 oder Rom Patcher JS an.“ |
| Taste | primär „Patch erstellen…“, gesperrt ohne `found`; daneben als Notiz der Grund (`reason`, bei `unassigned` „Erst eine Basis-ROM zuordnen.“) |
| Dialog Patch | Titel „Patch erstellen?“; Beschreibung „Speichert, baut die ROM und schreibt build/{name}.xdelta und build/{name}.xdelta.txt. Ein älterer Patch dort wird ersetzt.“; Tasten „Abbrechen“, „Patch erstellen“ (primär) |
| läuft | Notiz „Baue und erstelle den Patch …“ |
| Ergebnis | `Notice` Erfolg „Patch erstellt: {formatBytes(size)}“, darunter der Pfad in Mono und „Selbstprüfung bestanden: auf die Basis-ROM angewendet ergibt der Patch genau die gebaute ROM.“ |
| Fehler | `Notice` Fehler mit `errorText(e)` |

- [ ] **Step 1: Scheiternde Tests:** `api.base.test.ts`: `base_commands_send_the_documented_arguments`, `format_bytes_uses_byte_kib_and_mib_with_a_comma`; `useBase.test.tsx`: `reads_the_status_once_per_project`, `assigns_and_reads_the_status_again`, `keeps_the_last_patch`, `a_refused_register_keeps_the_status_and_says_why`; `PublishSurface.test.tsx`: `without_a_project_offers_to_open_one`, `unassigned_with_a_suggestion_offers_zuordnen`, `unassigned_without_a_suggestion_asks_for_a_file`, `the_check_dialog_lists_the_warnings_and_assigns_on_confirm`, `a_wrong_game_shows_the_reason_and_only_schliessen`, `missing_base_disables_the_patch_and_offers_basis_rom_suchen`, `found_base_shows_label_short_sha1_and_path`, `create_patch_asks_first_then_shows_size_path_and_self_check`; `TechnikArea.test.tsx`: `lists_four_entries_…` wird `lists_five_entries_with_veroeffentlichen_second`, neu `veroeffentlichen_subtitle_follows_the_base_state`.
- [ ] **Step 2: Lauf** `cd apps/studio && pnpm test` → FAIL; **Step 3: Umsetzen** (CSS-Modul nur mit Tokens, Raster wie `TechnikArea.module.css`); **Step 4: Lauf** → PASS, `pnpm build`, `bash scripts/check-styles.sh`, `bash scripts/check-tokens.sh` grün.
- [ ] **Step 5: Blick in der Test-Umgebung** (`docs/agent-gate.md`, „Je Lauf“, Kopie `_testkopien/4-1a-ui_DSPRE_contents` mit `4-1a-ui.nds`): Hell, Dunkel, Kompakt; Screenshots nach `target/agent/gate-<datum>/4-1a-ui/` (nicht ins Repo).
- [ ] **Step 6: Design-System fortschreiben.** `design-system/README.md` neuer Abschnitt „Nachtrag 4.1a“ nach dem Muster „Nachtrag 3h“: Quelle (`technik/PublishSurface.tsx`, `useBase.ts`, `TechnikArea.tsx`), Gate-Verweis `docs/phase-4-1a-gate.md`, neues Symbol `publish` (Phosphor `UploadSimple`), keine neuen Tokens, die Zustände und Texte der Tabelle oben, Abweichungen (keine Taste „Veröffentlichen“ in der Titelleiste, kein Hinweis beim Öffnen, Zielbild „Veröffentlichen: Patch erstellen“ nur teilweise), und dass die `.fig` die Zustände noch nicht kennt (Vorlage und Anleitung in `openpencil/`). `beispiele/veroeffentlichen.body.js` nach dem Muster von `welt-leeren.body.js` mit den Bausteinen aus `jsx-lib-app.js` (`Window`, `ListPane`, `ListRow`, `PanelHead`, `Badge`, `Btn`, `Notice`, `Mono`, `T`) und `v('token')`-Farben: `MODE` `'gefunden'` (Frame „App: Technik, Veröffentlichen“: Liste mit fünf Einträgen, „Veröffentlichen“ gewählt, Basis-ROM gefunden „Platin DE (Testprojekt)“, `d40f19ac`, Patch erstellt „4,7 KiB“), `'fehlt'` („App: Technik, Basis-ROM fehlt“: Warnung, „Basis-ROM suchen…“, gesperrte Taste mit Grund), `'zuordnen'` („App: Technik, Basis-ROM zuordnen“: Vorschlag und offener Dialog mit einer Warnung); Projekt „Sinnoh-Remix“, Platin DE. `NACHTRAG-4-1a.md` nach dem Muster von `NACHTRAG-3h.md`: „Die Datei wurde nicht angefasst“, was neu zu zeichnen ist (drei Frames auf der Seite „App: Texte, Technik, Dialoge“), Lage (nächste freie Spalte, x = Spalte × 1520, y = Reihe × 1000, vorher `get_page_tree`), Einspielen (`bash ../build-screen.sh veroeffentlichen`, `op.mjs render`, Vergleich per `export_image` mit den Screenshots aus Step 5, erzeugte `.jsx` nicht einchecken, danach `fix_fig_text.mjs --rgb555`, README-Satz, Marke über dem Zielbild „Veröffentlichen: Patch erstellen“ auf „Teilweise umgesetzt“), was nicht gezeichnet werden muss (Ladezustand, Fehlermeldung, „Andere Datei wählen…“). `Audit-Design-System.md` Zeile 26 „Veröffentlichen“: „teilweise (4.1a: Technik › Veröffentlichen)“.
- [ ] **Step 7: Commit** `feat(technik): Veröffentlichen with base ROM assignment and patch creation; design system addendum 4.1a` (mit Trailer).

### Task 10: Gate 4.1a

**Files:**
- Create: `docs/phase-4-1a-gate.md`
- Modify: `docs/ROADMAP.md` (Übersicht Zeile 4 und Punkt 4.1a mit Stand), `docs/agent-gate.md` (Gate-Datei in der Liste oben, Abschnitt „Phase 4.1a“ mit den Befehlen), `docs/dev-setup.md` (gated Tests der Tabelle `G4_TEST_PROJECT`, `G4_BASE_ROM_DIR`, `xdelta3` optional)

**Interfaces:**
- Consumes: alles Vorige; Messwerte aus den gated Tests (`patch-sizes …`, `rebuild … equal`) und dem Szenario.

- [ ] **Step 1: Gate-Dokument** `docs/phase-4-1a-gate.md` nach dem Muster von `docs/phase-3b4-gate.md`: Kopf mit Stand und dem Gate-Text der Roadmap als Zitat; Legende „Wer“ (Agent, Agent + Blick, Hand) mit Verweis auf `docs/agent-gate.md`; Namen in der Oberfläche (`[data-technik=veroeffentlichen]`, `[data-base-state]`, `[data-patch-card]`, Dialogtitel); Vorbereitung (Kopien `_testkopien/4-1a-{pt,hgde,hgus}_DSPRE_contents` mit `4-1a-{pt,hgde,hgus}.nds`, `4-1a-web…`, `4-1a-mcp…`; `cargo build --workspace --all-features` und `cargo build --release -p g4-kernel --bin g4k`; aus einem Worktree `G4_ROMS_DIR` absolut und der `tools`-Symlink, `target/agent/gate-*` vor `git worktree remove` sichern).
  - **(1) Automatik:** `scripts/gate.sh` grün mit dem Schritt `cargo deny (licenses, advisories)`; die gated Tests `base_assign_on_the_vanilla_folder_has_no_warning_and_rebuilds_the_nds`, `base_rom_members_equal_the_vanilla_folders`, `base_rom_header_table_equals_the_vanilla_project`, `patch_after_map_and_after_map_script_text_is_small_and_applies` (mit `xdelta3` im PATH) auf Platin DE, HeartGold DE, HeartGold US; der Einzelaufruf je Projekt mit `--nocapture --test-threads 1`.
  - **(2) Szenario kopflos** (Release-`g4k`, je Spiel auf seiner Kopie): `g4k base check` nennt die Nachbar-`.nds`; `g4k base assign` ohne Warnung; `g4k build --out build/rebuild.nds` und `shasum` gleich der `.nds`; fremder Spielcode `g4k base assign <pt-Kopie> <hgde-Kopie>.nds` → Exit 1 „Falsches Spiel …“; unbekannte SHA-1 (Kopie der `.nds` mit geändertem letztem Byte, `4-1a-pt-fremd.nds`) → `g4k base check --rom` warnt „Unbekannte oder veränderte ROM …“, Exit 0; `g4k map set-perm` auf Karte 3, `G4_BASE_ROM_DIR=<Roms>/_testkopien g4k patch create --json` → Größe ≤ 1 KiB, `encode_ms` ≤ 1000; `xdelta3 -d -s <nds> <patch> out.nds` und `shasum` gleich `build/<Name>.nds`; `xdelta3 printhdr` zeigt den App-Header und keine Zweitkompression.
  - **(3) Browser-Teil** (Test-Umgebung, `4-1a-web`): Technik › Veröffentlichen zeigt den Vorschlag; „Zuordnen…“ → Dialog ohne Warnung → „Basis-ROM zugeordnet: Platin DE (Testprojekt).“; „Patch erstellen…“ → Ergebnis mit Größe und Pfad; die `.nds` umbenennen, Technik neu zeigen → „Basis-ROM fehlt“ mit Grund, „Patch erstellen…“ gesperrt; „Basis-ROM suchen…“ mit der HeartGold-Kopie → „Das ist eine andere ROM …“, mit der umbenannten Datei → „Gefunden“. MCP über die Brücke (`4-1a-mcp`, Freigabe „Lesen und bearbeiten“): `scripts/agent/mcp.sh call assign_base '{"nds":"<Nachbar>.nds"}'` im Hintergrund → Dialog „Agent möchte etwas Unumkehrbares tun“ mit „Basis-ROM zuordnen“, `call.sh agent_answer … allow`; der nächste `mcp.sh call base_status` fragt nicht nach der Freigabe; `create_patch` ebenso mit „Patch erstellen“.
  - **(4) Hand und Blick beim Nutzer:** Rom Patcher JS im Browser mit der eigenen Basis-ROM und dem Patch aus (2) → SHA-1 des Ergebnisses gleich der Begleitdatei (Fassung notieren); Blick auf Veröffentlichen in Hell, Dunkel, Kompakt (Screenshots aus Task 9); xdelta UI unter Windows offen (kein Windows).
  - **Ergebnis:** Tabelle Punkt / Wer / Ergebnis / Datum und die Tabelle der Patch-Größen je Spiel:

    | Spiel | Rebuild-SHA-1 gleich | Patch Karte | Patch Karte + Script + Text | Erzeugen (ms) | Selbstprüfung (ms) | `xdelta3 -d` gleich |
    |---|---|---|---|---|---|---|
    | Platin DE | | | | | | |
    | HeartGold DE | | | | | | |
    | HeartGold US | | | | | | |

- [ ] **Step 2: Gate abarbeiten** (Agent: (1) bis (3)); Werte und Befunde eintragen; Befunde beheben (eigene Commits `fix(4.1a): …`), bis alles grün ist.
- [ ] **Step 3: Roadmap** `docs/ROADMAP.md`: Übersicht Zeile 4 („4.1a Gate vom Agenten bestanden <Datum>, Rom Patcher JS und Blick beim Nutzer offen“) und am Ende des Punkts 4.1a „(Gate vom Agenten bestanden <Datum>: Rebuild byte-gleich auf allen drei Projekten, Patch-Größen <…>; beim Nutzer offen: Rom Patcher JS, Blick)“. `docs/agent-gate.md` und `docs/dev-setup.md` wie unter Files.
- [ ] **Step 4: Commit** `docs(4.1a): gate of base ROM and patch, roadmap status` (mit Trailer).

---

## Offen für die Umsetzung

- **Advisories:** welche Befunde die Advisory-Datenbank am Tag der Umsetzung meldet und welche sich durch Updates beheben lassen, steht erst nach Task 1 Step 2 fest; die Ausnahmen und ihre Gründe entscheidet der Implementierer nach den Regeln von Task 1 (rotom- oder Tauri-Sprünge sind kein Teil von 4.1a).
- **C-Bau:** ob xdelta3.c mit `SECONDARY_*=0` und unter dem globalen `CFLAGS=-D_POSIX_C_SOURCE=200809L` aus `.cargo/config.toml` auf macOS kompiliert (sonst die Crate-Defines bzw. nur für dieses Crate zusätzlich `_DARWIN_C_SOURCE`); der Bau unter Windows (MSVC, `XD3_WIN32`) ist ungeprüft, es gibt kein Windows.
- **Banner-Vergleich:** ob die ds-rom-Fassung `8a9c151` den Banner aus `banner/banner.yaml` und aus der ROM so liefert, dass sich die Titel vergleichen lassen; geht das nicht billig, entfällt nur die Warnung `BannerDiffers` (im Gate vermerken). Ebenso, ob DSPREs `header.yaml` das Feld `rom_version` trägt (ohne es keine Versionswarnung).
- **Rom Patcher JS und xdelta UI:** welche Fassungen gelten (Hand des Nutzers); Windows nicht geprüft.
- **Lizenztext im App-Bundle:** Apache-2.0 §4 verlangt den Lizenztext bei Weitergabe von Binärdateien; ob `docs/release.md` und das Bundle (`LICENSE`/`NOTICE` von xdelta3 in den Ressourcen) das beim nächsten Release nachziehen, entscheidet der Release-Schritt, nicht 4.1a.
- **Weitere Einstiege:** eine Taste „Veröffentlichen“ in der Titelleiste (Zielbild, Audit: „bewusst Phase 4“) und ein Hinweis beim Öffnen eines DSPRE-Ordners mit Nachbar-`.nds` sind nicht entschieden; 4.1a zeigt beides nur in Technik.
- **`build/<Name>.nds`:** ob die gebaute ROM des Patches nach dem Export liegen bleiben soll (128 MiB je Projekt) oder in `.g4/tmp/` gehört; der Plan lässt sie liegen.
- **Kopflose Basis-ROMs:** ob `studio mcp --project` später auch `base-roms.json` der App lesen soll (heute nur `--base-rom` und `G4_BASE_ROM_DIR`).
- **No-Intro-Prüfsummen** in `metadata/base_roms.yaml`: weiterhin nur die drei Testprojekt-Dumps; bei echten Nutzer-Dumps warnt „Unbekannte oder veränderte ROM“ öfter als nötig.
- **Merge mit 4.2:** Konflikte in `TechnikArea.tsx` (Liste und „Prüfen und Bauen“), `dispatch.rs`/`lib.rs`/`access.rs`/`catalog.rs` (angehängte Einträge) und `g4k.rs` (neue Unterbefehle neben `--timings`); wer zuerst landet, bestimmt die Basis, der andere zieht nach. Ruft 4.2 `restore_header_crcs` anders auf, muss der gated Rebuild-Test aus Task 3 danach weiter grün sein.
