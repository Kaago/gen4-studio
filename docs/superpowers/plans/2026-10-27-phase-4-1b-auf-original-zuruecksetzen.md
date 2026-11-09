# Phase 4.1b „Auf Original zurücksetzen“ Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Jede Entität der entschiedenen Arten (Karte, Eventdatei, Textarchiv je Nachricht und ganz, Trainer, Pokémon-Daten, Begegnungen, Header Feld für Feld, Scripts, Texturen und Modelle) lässt sich über „Auf Original zurücksetzen…“ im Menü „Aktionen“ ihres Kopfs, im Kontextmenü ihrer Liste und über MCP mit einer Vorher-Nachher-Rückfrage als ein Undo-Schritt auf den Stand der Basis-ROM bringen, so dass ihr Member danach byte-gleich zum Basis-Member ist; Technik › Änderungen listet aus den Hashes des Build-Graphs, was vom Original abweicht.

**Architecture:** Das neue Kernel-Modul `original` kennt die Ziele (`ResetTarget`), liest das Original über einen kleinen Trait `Original` (umgesetzt von `base::BaseRom`, in Tests von den gepackten NARCs einer unberührten Projektkopie), plant den Schritt mit deutschen Vorher-Nachher-Zeilen und führt ihn aus. Store-Arten laufen über eine neue Transaktionsoperation `Tx::replace` (ganze Asset-Bytes), Header über `Tx::edit_header` Feld für Feld über serde; Scripts, Texturen und Modelle sind keine Store-Werte und werden sofort geschrieben, aber als „Dateischritt“ (`FileStep`) auf denselben Undo-Stapel gelegt. Die Liste „geändert gegenüber Original“ vergleicht xxh3 je Basis-Member (einmal je SHA-1 im App-Cache) mit den Member-Fingerabdrücken aus `.g4/build/state.json` (4.2) und trägt fehlende Hashes einmal nach; Header vergleicht sie über den Codec. Die App bekommt vier Befehle (Tauri, Brücke, MCP), einen Dialog, ein Kontextmenü an `ListRow`/`TreeRow` und die Technik-Fläche „Änderungen“.

**Tech Stack:** Rust (g4-kernel, uxie `MapHeader` mit serde, rotom `fb11503` für das Dekompilieren, xxhash-rust xxh3, serde_json), Tauri-Befehle mit `Channel` für den Fortschritt, MCP (Befehlstabelle, Katalog), React 19 mit den Bausteinen aus `apps/studio/src/ui/` (Base UI `ContextMenu`), vitest.

**Spec:** [entscheidungen.md](../../research/2026-10-27-phase-4/entscheidungen.md): „Phasenschnitt und Reihenfolge“, im Abschnitt „Basis-ROM und Patch (Gutachten B, 4.1)“ die Zeilen „Auf Original zurücksetzen (4.1b)“ und „Liste „geändert gegenüber Original““ (die Spalte „Entscheidung“ gilt wörtlich), „Fehlende Basis-ROM“, „Nicht in Phase 4“; im Abschnitt „Git“ die Zeile „Ort in der App“ (Technik › Änderungen); [gutachten-b-build-graph.md](../../research/2026-10-27-phase-4/gutachten-b-build-graph.md) §3.3, §3.4, §6 (Gate-Vorschlag 4.1b); [ROADMAP.md](../../ROADMAP.md) „Phase 4“, Punkt 4.1b mit Gate; der Plan [4.2](2026-10-27-phase-4-2-build-graph.md) (Abschnitt „Offen“, Member-Hashes) und das Format [build_state.md](../../formats/build_state.md) (auf main nach dem Merge von 4.2).

Stand 27.10.2026: `main` bei `256105e` (4.0 und 4.1a gemergt); **4.2 wird vor dem Start von 4.1b auf main gemergt**, der Plan setzt diesen Stand voraus (Abschnitt „Voraussetzungen aus 4.2“). Arbeit im eigenen Worktree (Branch `phase-4-1b-auf-original`), **parallel zu 4.3a** in einem zweiten Worktree. Kernel `crates/g4-kernel/src`, Tauri `apps/studio/src-tauri/src`, Frontend `apps/studio/src`.

## Voraussetzungen aus 4.2

Vor Task 1 auf dem gemergten main prüfen (`grep` genügt); fehlt etwas, steht dabei, was 4.1b selbst ergänzt:

| Schnittstelle | erwartet | gebraucht in |
|---|---|---|
| `build::state::BuildState` | `pub fn load(root) -> BuildState`, `pub fn update(root, f: impl FnOnce(&mut BuildState)) -> Result<(), KernelError>`, Feld `steps: BTreeMap<String, StepRecord>` | Task 5 |
| `build::state::StepRecord` | `scanned_at_ns: i128`, `inputs: BTreeMap<String, Fp>`, `outputs: BTreeMap<String, Fp>`; ein Ordner unter den Eingaben steht **je Datei** in `inputs` (Schlüssel `unpacked/<Ordner>/NNNN`, `/` als Trenner) | Task 5 |
| `build::Fp` | `{ size, mtime_ns, ctime_ns, ino, xxh3: Option<u64> }`, serde als Array | Task 5 |
| `build::fingerprint` | `pub(crate) fn stat(path) -> Result<Option<Fp>, KernelError>`, `pub(crate) fn is_racy(fp, scanned_at_ns) -> bool`; **Invariante:** ein racy aufgezeichneter Eintrag trägt immer `xxh3` | Task 5 (Regel für das Nachtragen) |
| Schritt-IDs | `pack:<Ordner unter unpacked/>` mit dem gepackten NARC (`files/…narc`) als einziger Ausgabe; auch die Pack-Einträge der DSPRE-Ordner aus `archives.yaml` | Task 5 |
| Liste der Pack-Einträge | gibt es keine öffentliche Funktion, legt Task 5 in `build/steps.rs` `pub(crate) fn pack_entries(project: &Project) -> Result<Vec<(String, PathBuf)>, KernelError>` an (Ordnername, projektrelativer Pfad des NARC), aus derselben Quelle wie die Pack-Schritte | Task 5 |
| Texte nach Fingerabdrücken | `text_winner` ist durch `text::TextSync` ersetzt; ein Speichern im Store schreibt JSON und Member und zeichnet den Abgleich in `state.json` auf | Task 2 (Textarchiv), Task 5 |
| „Neu bauen (ohne Cache)“ | `BuildState::clear_keeping_texts` löscht die Schritt-Einträge; die Liste hasht danach die Ordner neu (Zustand `unbuilt`) | Task 5 |

Offen laut 4.2: `state.json` trägt xxh3 nur für gehashte Einträge. **4.1b trägt die fehlenden Hashes beim ersten Vergleich nach** (einmal 4 bis 25 s warm, mit Fortschritt) und schreibt sie in die vorhandenen Einträge zurück (Regel in Task 5). Ein Hashen im Hintergrund nach jedem ersten Build (die andere Möglichkeit) bremst „Hier spielen“ und belastet den Rechner auch für Nutzer, die die Liste nie öffnen.

## Entscheidungen (wo Roadmap und Gutachten offen lassen)

| Frage | Entscheidung | Grund |
|---|---|---|
| Undo für Store-Arten | `Tx::replace(id, bytes)` setzt ein Asset auf den aus `bytes` dekodierten Wert, in der Byte-Form des Assets (Textarchiv: kanonische JSON; Trainer: `u32 len(props) \| props \| party`); ein Ziel = eine Transaktion mit dem Label `Auf Original zurückgesetzt: <Name>` | derselbe Weg wie jede Bearbeitung: dirty bis „Speichern“, ⌘Z/⌘⇧Z, Herkunft (Agent) im Verlauf |
| Undo für Scripts, Texturen, Modelle | sofort geschrieben, als **Dateischritt** auf dem Undo-Stapel des Stores (`FileStep`: Asset, Dateien mit Bytes vorher und nachher, bei Scripts der Eintrag in rotoms `compile-state.json`); ⌘Z schreibt die alten Bytes zurück | sie sind keine Store-Werte: der Script-Editor speichert sofort, die 3D-Ansicht und rotom lesen die Dateien; „Undo-Schritt im Store“ bleibt wörtlich erfüllt |
| Datei seit dem Schritt außen geändert | ⌘Z schreibt nichts, der Dateischritt fällt aus dem Verlauf, die Meldung sagt es; ältere Schritte bleiben rücknehmbar. Speichert die App eine Scriptdatei, vergisst der Store deren Dateischritte (`forget_asset_history`) | ein stehengebliebener Schritt würde jedes ältere ⌘Z sperren |
| Pokémon-Daten | Ziel `species`: Personal, Lernset und Entwicklung desselben Index in einer Transaktion | der Pokédex zeigt die drei zusammen |
| Trainer | das Paar Eigenschaften + Team (`AssetKind::Trainer`); Name und Texte in den Textbänken bleiben | B §3.4 „zwei Member, ganz“; Texte sind Textarchive |
| Textarchiv ganz | Nachrichten wie in der Basis (Anzahl der Basis, neue fallen weg), `key` der Basis; für vorhandene Indizes bleiben `id` und `other` der aktuellen JSON | der Member wird byte-gleich, die DSPRE-Ids in der JSON bleiben stabil |
| Textarchiv je Nachricht | nur `text` der Nachricht; Index ≥ Anzahl der Basis: `Neue Nachricht, kein Original.` | B §3.4 |
| Header | Feld für Feld über serde (`{"Pt": {…}}`), Vergleich auf den dekodierten Feldern; der Dialog zeigt die abweichenden Felder mit Häkchen (alle an); `reset_to_original` nimmt optional `fields` | Entscheidung „Feld für Feld über den Codec“; mit Dynamic-Headers-Patch liegt die Basis im ARM9, das Projekt im NARC |
| Scripts | Basis-Member schreiben, im Projektkontext dekompilieren (rotom-Quelle oder Level-Script-JSON), Eintrag in `compile-state.json` als „dekompiliert“ mit den Quirks setzen, damit der nächste Build den Member nicht neu kompiliert; dekompiliert rotom ihn nicht, wird der Member allein geschrieben und eine Quelle entfernt; ohne `rotom.toml` nur der Member | Entscheidung „Basis dekompilieren“ und Gate „Member byte-gleich“ |
| Texturen und Modelle | Ziele `map_texture`, `building_texture`, `building_model`, `interior_building_model`, ganz, binär, ohne Vorschau des Unterschieds (Größe und xxh3) | Entscheidung „(ganz)“; „Nicht in Phase 4“: kein Bildvergleich |
| Andere Arten | Matrix, Gebietsdaten, Attacken, Items, Icons und die übrigen DSPRE-Ordner erscheinen in der Liste als geändert, aber ohne Zurücksetzen (`Für diese Art gibt es kein Zurücksetzen.`); ARM9-Patches, Overlays, Overlay-Tabellen nie | die Entscheidung zählt die Arten auf; „Nicht in Phase 4“ |
| Neue Entitäten | verfügbar nur für Index < Anzahl in der Basis; sonst Eintrag gesperrt mit `Kein Original`; der Kernel lehnt mit `NO_ORIGINAL` ab | Entscheidung „nicht für neue Entitäten“ |
| Gesperrte Assets | (Länge falsch, Platzhalter-Trainer, nicht byte-treu lesbar) abgelehnt mit dem Sperrgrund, nichts aufgezeichnet | `Tx` lehnt gesperrte Assets ab; ein Undo-Teil kann keinen rohen Member halten |
| Basis-ROM lesen | je Aufruf: `BaseRoms::locate` (hasht nur bei abweichender Größe/mtime), dann `BaseRom::open_unchecked(path, sha1)` ohne zweite SHA-1 | die SHA-1 kostet 0,12 s und fräße das Budget von 0,2 s je Reset; Entscheidung „je Aufruf lesen, nicht im Speicher halten“ |
| Verfügbarkeit in der Oberfläche | `original_status` liefert einmal je Projekt die Anzahl je Zielart; das Frontend sperrt `index ≥ Anzahl`; Nachrichten prüft erst der Plan | kein Kernel-Aufruf je Auswahl |
| Vorher-Nachher | deutsche Zeilen aus dem Kernel (Regeln in Task 2), im Dialog mit `BeforeAfter` je Änderung | eine Quelle für App und MCP, wie die Texte der Prüfung in 4.1a |
| Ort der Liste | **Technik › Änderungen** (neuer Eintrag nach „Veröffentlichen“, Symbol `changes`) | die Entscheidung zu 4.4 legt die Liste dorthin (mit Umschalter Basis/Commit); 4.4 erweitert sie, statt sie umzuziehen; Veröffentlichen bleibt beim Patch |
| Inhalt der Liste | gespeicherter Stand (Dateien), nicht ungespeicherte Store-Werte; Zeilen eines dirty Assets tragen „Ungespeichert“ | die Hashes beschreiben Dateien |
| Basis-Hashes | `<App-Cache>/base/<sha1>.json` (`app_cache_dir`; in der Brücke `<app_data>/cache`), je gepacktem NARC-Pfad die xxh3 der Member; kopflos ohne Cache (im Speicher, < 1 s) | Entscheidung „einmal je SHA-1 im App-Cache“ |
| Ordner ohne Build-Eintrag | wird gehasht (mit Fortschritt), nichts gespeichert, Name in `unbuilt`; Archiv ohne `unpacked/`-Ordner: das NARC des Projekts im Speicher | erfundene Schritt-Einträge würden den Build falsch überspringen lassen |
| MCP | Werkzeuge `changed_from_original` und `plan_reset` (`Read`), `reset_to_original` (`Write`, Undo-Schritt); Befehl `original_status` (`Read`) ohne eigenes Werkzeug; `reset_to_original` mit einem Dateiziel (Script, Textur, Modell) nicht im Batch | Vorgabe „Zugriffsart `Write`, da Undo-Schritt“; Batch fasst Schritte mit `squash_since` zusammen, Dateischritte schreiben sofort |
| Kopflos | kein `g4k`-Unterbefehl; kopflos geht es über `studio mcp --project … --base-rom …` | MCP deckt es ab; `g4k.rs` ändert 4.3a parallel |
| Kontextmenü | neue Eigenschaft `contextMenu?: readonly MenuEntry[]` an `ListRow` und `TreeRow` (Base UI `ContextMenu`, wie `Reference`); Shift+F10 und die Kontextmenü-Taste öffnen es | Rechtsklick ist ohne Tastaturweg nicht barrierefrei; ein Baustein statt zehn eigener Menüs |
| Karte in Listen | Karten haben keine Liste (Raster der Matrix); Zurücksetzen nur im Kopf des Zellen-Inspectors | – |
| Script mit ungespeichertem Editor-Inhalt | der Dialog warnt „Ungespeicherte Änderungen an dieser Scriptdatei gehen verloren.“; nach dem Schritt liest der Editor die Datei neu | der Editor-Puffer ist kein Store-Wert |

## Global Constraints

- Commits enden mit `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>`; vor jedem cargo-Aufruf `export PATH="$HOME/.cargo/bin:/opt/homebrew/opt/rustup/bin:$PATH"`.
- Originale unter `$HOME/Projekte/lab/editor/Roms` nie von Hand öffnen oder ändern; Testprojekte nur als frische Kopien unter `Roms/_testkopien/4-1b-<zweck>` (Ordner `4-1b-<zweck>_DSPRE_contents` und `4-1b-<zweck>.nds` direkt nebeneinander). Gated Tests lesen `G4_TEST_PROJECT` (von `scripts/gate.sh` je Projekt gesetzt), arbeiten auf `vanilla_copy()` und finden die Basis-ROM über `test_support::vanilla_base_rom()` (= `suggest_base(vanilla_project())`), nur lesend. Aus einem Worktree `G4_ROMS_DIR` absolut und den `tools`-Symlink setzen; `target/agent/gate-*` vor `git worktree remove` sichern.
- Keine ROM-Daten, keine `.nds`, keine Basis-Hashes und keine Member ins Repo; Fixtures nur synthetisch.
- Fehlende Basis-ROM sperrt nie den Build; Zurücksetzen und die Liste sind dann aus und nennen den Grund (`patch::NO_MARK`, `base::missing_text`, Grund einer unlesbaren Marke). Ersatz nur mit gleicher SHA-1 (4.1a, unverändert).
- Gate-Zahlen: nach dem Zurücksetzen ist der Member byte-gleich zum Basis-Member; ein Reset dauert ≤ 0,2 s (vom Öffnen der Basis-ROM bis zum aufgezeichneten Schritt, warm); ⌘Z stellt die Änderung wieder her.
- Nicht zurücksetzbar: neue Entitäten, ARM9-Patches, Overlays, Overlay-Tabellen. Keine Plakette „geändert“ im Inspector-Kopf (Nicht in Phase 4).
- UI: nur Bausteine und Tokens aus `apps/studio/src/ui/`, keine Hex-Farben, keine Inline-Styles (`bash scripts/check-styles.sh`, `bash scripts/check-tokens.sh`), deutsche Sätze, Bezeichner englisch; höchstens eine primäre Taste je Dialog und Karte.
- Parallel zu 4.3a: in `dispatch.rs` (`COMMANDS`, Arme), `lib.rs` (`generate_handler!`), `access.rs` (`COMMAND_KINDS`), `catalog.rs`, `tools.rs` und `crates/g4-kernel/src/lib.rs` nur **anhängen**; `g4k.rs` bleibt unberührt.
- Vor jedem Commit: `cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, die Tests der berührten Crates; bei Frontend-Änderungen zusätzlich `cd apps/studio && pnpm test && pnpm build`, `bash scripts/check-styles.sh`, `bash scripts/check-tokens.sh`.

## Review Focus

- Ein Asset mit ungespeicherter Bearbeitung wird zurückgesetzt: ein Schritt über der Bearbeitung, ⌘Z führt zum ungespeicherten Wert (nicht zum gespeicherten), ⌘Z noch einmal zum gespeicherten (Task 1 `replace_over_an_unsaved_edit_undoes_to_the_unsaved_value`).
- DSPRE (oder ein Editor außerhalb) schreibt eine Scriptdatei oder Textur, nachdem sie zurückgesetzt wurde: ⌘Z schreibt nichts darüber, sagt warum, der Dateischritt verschwindet und ein älterer Store-Schritt ist danach rücknehmbar (Task 1 `file_step_undo_of_a_changed_file_writes_nothing_and_drops_the_step`).
- Ein Member wurde nach dem letzten Build geändert, ohne dass gebaut wurde (gleiche Größe, andere mtime): die Liste hasht ihn jetzt und zeigt ihn als geändert, und `state.json` bekommt für diese Datei **keinen** Hash, sonst übersprünge der nächste Build das Packen (Task 5 `changed_list_never_records_a_hash_for_a_file_changed_since_the_build`).
- Die Basis-ROM verschwindet zwischen dem Öffnen des Dialogs und „Zurücksetzen“ (umbenannt): Antwort `base_missing` mit dem Grund, kein Schritt, der Dialog zeigt die Meldung (Task 6 `reset_without_a_found_base_answers_base_missing_and_records_nothing`, Task 7 `a_failed_reset_shows_the_reason_and_keeps_the_dialog`).
- Ein gesperrtes Asset (Item-Länge falsch, Platzhalter-Trainer des Sicht-Scripts): abgelehnt mit dem Sperrgrund, nichts aufgezeichnet, die Oberfläche zeigt den Grund statt eines leeren Dialogs (Task 1 `replace_of_a_locked_asset_is_refused_and_records_nothing`, Task 2 `reset_of_the_placeholder_trainer_is_refused_with_its_reason`).

---

### Task 1: Store: ganze Assets ersetzen und Dateischritte

**Files:**
- Modify: `crates/g4-kernel/src/store.rs` (`Tx::replace`, `trainer_pair_bytes`, `FileChange`, `FileStep`, `RotomEntryChange`, Feld `files` in `Changeset`, `record_file_step`, `forget_asset_history`, `undo`/`redo`/`forget_history`/`squash_since`; Tests im Testmodul der Datei), `crates/g4-kernel/src/lib.rs` (`pub mod original;` am Ende)
- Create: `crates/g4-kernel/src/original.rs` (zunächst nur `pub fn asset_label(id: AssetId) -> String`, Wörter wie in Task 2)

**Interfaces:**
- Produces:

```rust
impl Tx<'_> {
    /// Stages asset `id` as decoded from `bytes` in the asset's own byte form (text archive: its
    /// canonical JSON; trainer: `trainer_pair_bytes`). Refuses like `edit` (locked, edited twice);
    /// bytes that do not decode fail the transaction with the decode error.
    pub fn replace(&mut self, id: AssetId, bytes: &[u8]) -> Result<(), KernelError>;
}
pub fn trainer_pair_bytes(props: &[u8], party: &[u8]) -> Vec<u8>;           // = join_pair
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileChange { pub path: PathBuf, pub before: Option<Vec<u8>>, pub after: Option<Vec<u8>> } // None: no file
#[derive(Debug, Clone, PartialEq)]
pub struct RotomEntryChange { pub source: PathBuf, pub before: Option<serde_json::Value>, pub after: Option<serde_json::Value> }
#[derive(Debug, Clone, PartialEq)]
pub struct FileStep { pub asset: AssetId, pub files: Vec<FileChange>, pub rotom_entry: Option<RotomEntryChange> }
impl Store {
    /// Records a step whose files the caller has already written (`after` is on disk). Undo checks
    /// every file against `after`, then writes `before` (None removes the file) and restores the
    /// rotom entry; redo the other way. Notes a change of `step.asset` (`changes_since`).
    pub fn record_file_step(&mut self, label: &str, step: FileStep) -> ChangeId;
    /// Drops every part and file step of `id` from both stacks (the existing `forget_history`, public).
    pub fn forget_asset_history(&mut self, id: AssetId);
}
```

- Undo/Redo eines Changesets mit Dateischritten: erst alle Store-Teile dekodieren (wie heute), dann jede Datei gegen die erwartete Seite prüfen. Weicht eine ab: das Changeset verlässt den Stapel, nichts wird geschrieben, `Err(KernelError::Refused(format!("{} wurde seit dem Zurücksetzen außerhalb des Studios geändert; ⌘Z nimmt den Schritt nicht zurück, er fällt aus dem Verlauf.", crate::original::asset_label(asset))))`. Sonst Dateien über `write_atomic` bzw. `remove_file`, dann die Store-Teile. `rotom_entry` trägt dieser Task nur mit (die Tests nutzen `None`); Task 4 ruft beim Undo und Redo nach den Dateien `crate::scripts::set_state_entry` mit der passenden Seite. Rückgabe wie heute plus die Assets der Dateischritte. `squash_since` hängt die Dateischritte der zusammengefassten Schritte in ihrer Reihenfolge an; `forget_history(id)` entfernt auch Dateischritte von `id`, ein Changeset ohne Teile und ohne Dateischritte fällt weg (Barriere wie heute).

- [ ] **Step 1: Scheiternde Tests** (synthetisches Projekt, `Store::new`):
  - `replace_sets_the_asset_from_bytes_as_one_undoable_step` (Karte 0: Bytes einer geänderten Karte über `replace`; `current_bytes` = neue Bytes, `history().0.last() == Some("Test")`; `undo` → alte Bytes, `redo` → neue).
  - `replace_with_the_same_bytes_records_no_step` (Rückgabe `None`, Stapel leer).
  - `replace_over_an_unsaved_edit_undoes_to_the_unsaved_value` (Review Focus: `edit_map` ohne Speichern, dann `replace` mit den gespeicherten Bytes; `undo` → bearbeiteter Wert, `undo` → gespeicherter).
  - `replace_of_a_locked_asset_is_refused_and_records_nothing` (Review Focus: Item mit falscher Länge im Ordner → `Err(ReadOnly { reason, .. })`, Stapel leer).
  - `replace_of_a_trainer_takes_the_pair_bytes` (`trainer_pair_bytes(props, party)` eines anderen Trainers; nach `save` beide Member-Dateien gleich den Quellen).
  - `file_step_undo_writes_before_and_redo_writes_after` (zwei Dateien, eine vorher nicht vorhanden: `undo` entfernt sie und schreibt die andere zurück; `redo` umgekehrt; Rückgabe enthält das Asset).
  - `file_step_undo_of_a_changed_file_writes_nothing_and_drops_the_step` (Review Focus: vorher ein `edit_map`-Schritt; Datei nach dem Schritt überschrieben → `undo` ist `Refused` mit „außerhalb des Studios geändert“, Datei unverändert, `history().0` hat nur noch den Kartenschritt, das nächste `undo` nimmt ihn zurück).
  - `forget_asset_history_drops_its_file_steps`, `squash_since_keeps_file_steps_in_order`, `file_step_counts_as_a_change_for_changes_since`.
- [ ] **Step 2: Lauf** `cargo test -p g4-kernel --all-features store::` → FAIL (Funktionen fehlen).
- [ ] **Step 3: Umsetzen** wie unter Interfaces; `Tx::replace` ruft `self.edit(id, |loaded| { *loaded = Loaded::from_bytes(id.kind, bytes, family, lang)?; Ok(()) })`.
- [ ] **Step 4: Lauf** → PASS; dazu `cargo test -p g4-kernel --all-features` (bestehende Store-Tests grün).
- [ ] **Step 5: Commit** `feat(store): replace a whole asset in a transaction, file steps on the undo stack` (mit Trailer).

### Task 2: Kernel-Modul `original`: Ziele, Original, Store-Arten

**Files:**
- Create: `crates/g4-kernel/src/original/preview.rs`, `crates/g4-kernel/tests/original.rs` (gated)
- Modify: `crates/g4-kernel/src/original.rs` (aus Task 1; + Tests im Modul), `crates/g4-kernel/src/base.rs` (`open_unchecked`, `impl Original for BaseRom`), `crates/g4-kernel/src/test_support.rs` (`vanilla_base_rom`, `ProjectOriginal`), `docs/formats/project_base.md` (Abschnitt „Auf Original zurücksetzen“)

**Interfaces:**
- Consumes: Task 1 `Tx::replace`, `trainer_pair_bytes`; `Store::transaction`, `Store::{map, event, text_archive, trainer, personal, learnset, evolution, encounter}`; `text::TextArchiveDoc::{from_member, to_json}`.
- Produces:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ResetTarget {
    Map { index: u16 }, EventFile { index: u16 }, TextArchive { index: u16 },
    TextMessage { archive: u16, message: u16 }, Trainer { index: u16 }, Species { index: u16 },
    Encounter { index: u16 }, Header { id: u16 }, Script { file: u16 },
    MapTexture { index: u16 }, BuildingTexture { index: u16 }, BuildingModel { index: u16 },
    InteriorBuildingModel { index: u16 },
}
impl ResetTarget {
    pub fn assets(&self) -> Vec<AssetId>;   // Species: Personal, Learnset, Evolution; Header: HEADER_TABLE
    pub fn label(&self) -> String;          // "Karte 0003", "Nachricht 2 in Textarchiv 0005", "Header 0012"
    pub fn writes_files(&self) -> bool;     // Script und die vier Textur/Modell-Ziele
}
pub trait Original {
    fn sha1(&self) -> &str;
    fn member(&self, kind: AssetKind, index: u16) -> Result<Option<Vec<u8>>, KernelError>;
    fn count(&self, kind: AssetKind) -> Result<Option<usize>, KernelError>;  // None: kein Archiv
    fn header_table(&self) -> Result<Vec<MapHeader>, KernelError>;
    fn narc_at(&self, packed: &Path) -> Result<Option<Narc>, KernelError>;   // projektrelativ `files/…`
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)] pub struct ResetChange { pub label: String, pub before: String, pub after: String }
#[derive(Debug, Clone, PartialEq, Eq, Serialize)] pub struct HeaderFieldDiff { pub field: String, pub before: String, pub after: String }
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ResetPlan { pub target: ResetTarget, pub label: String, pub changes: Vec<ResetChange>,
                       pub fields: Vec<HeaderFieldDiff>, pub writes_files: bool, pub same: bool }
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ResetResult { pub changed: Vec<AssetId>, pub recorded: bool, pub ms: u64 }
pub const NO_ORIGINAL: &str = "Kein Original: diesen Eintrag gibt es in der Basis-ROM nicht.";
pub const NEW_MESSAGE: &str = "Neue Nachricht, kein Original.";
pub const NOT_RESETTABLE: &str = "Für diese Art gibt es kein Zurücksetzen.";
pub fn asset_label(id: AssetId) -> String;
pub fn step_label(target: &ResetTarget) -> String;               // "Auf Original zurückgesetzt: Karte 0003"
pub fn base_counts(original: &dyn Original, project: &Project) -> Result<BTreeMap<&'static str, usize>, KernelError>;
pub fn availability(target: &ResetTarget, original: &dyn Original, project: &Project) -> Result<Option<String>, KernelError>;
pub fn plan_reset(store: &mut Store, original: &dyn Original, target: &ResetTarget) -> Result<ResetPlan, KernelError>;
pub fn reset(store: &mut Store, original: &dyn Original, target: &ResetTarget, fields: Option<&[String]>) -> Result<ResetResult, KernelError>;
// base.rs
impl BaseRom { pub fn open_unchecked(path: &Path, sha1: &str) -> Result<BaseRom, KernelError>; }  // liest, hasht nicht
impl Original for BaseRom { /* narc_at: Pfad ohne `files/` an `file` */ }
// test_support.rs (feature test-support)
pub fn vanilla_base_rom() -> Option<PathBuf>;
pub struct ProjectOriginal { /* Project einer unberührten Kopie; member/count aus `read_packed`, header_table aus `read_headers` */ }
impl ProjectOriginal { pub fn new(root: &Path) -> ProjectOriginal; }
```

- Wörter (`asset_label`, `label`, Schlüssel von `base_counts` = `kind` des Ziels): `map` Karte, `event_file` Eventdatei, `text_archive` Textarchiv, Nachricht `{m}` in Textarchiv, `trainer` Trainer, `species` Pokémon, `encounter` Begegnungsdatei, `header` Header, `script` Scriptdatei, `map_texture` Kartentextur, `building_texture` Gebäudetextur, `building_model` Gebäudemodell, `interior_building_model` Innenraum-Gebäudemodell; Nummern vierstellig, die Nachricht ohne Nullen. `base_counts` lässt Arten ohne Archiv weg; `trainer` ist das Minimum beider Archive, `species` die Anzahl von Personal.
- `availability`: `Some(NO_ORIGINAL)` für Index ≥ Anzahl, `Some(NEW_MESSAGE)` für eine Nachricht ≥ Nachrichten der Basis, `None` sonst. `plan_reset` und `reset` prüfen sie zuerst (`Refused(<Grund>)`). `reset` misst `ms` vom Eintritt bis zum aufgezeichneten Schritt; `recorded == false`, wenn schon alles dem Original gleicht. Header, Script, Texturen, Modelle antworten in diesem Task mit `Refused("noch nicht")`; Task 3 und 4 ersetzen das.
- Byte-Form des Originals je Ziel: Karte, Eventdatei, Begegnung, Personal/Lernset/Entwicklung = Member; Trainer = `trainer_pair_bytes(props, party)` aus beiden Archiven; Textarchiv = `TextArchiveDoc::from_member(member, lang)` mit `id`/`other` der aktuellen Nachrichten gleichen Index, dann `to_json`; Nachricht = aktuelles Dokument, nur `messages[m].text` aus der Basis.
- Vorschau-Zeilen (`preview.rs`, `pub(crate) fn lines(target, bytes) -> String`, eine Zeile je Eintrag, `\n` dazwischen; dieselbe Funktion für beide Seiten):

| Ziel | Zeilen |
|---|---|
| Karte | `Bewegungsrechte {xxh3 8 Stellen}`, `Gebäude {Anzahl}`, `Modell {Bytes} Byte {xxh3 8}`, `BDHC {Bytes} Byte {xxh3 8}`; `ResetChange.label` nennt zusätzlich die Zahl der abweichenden Zellen („Karte 0003: 37 Zellen der Bewegungsrechte anders“) |
| Eventdatei | je Event `{Figur|Tür|Auslöser|Schild} {i}: {serde_json kompakt des uxie-Events}` |
| Textarchiv | je Nachricht `{i}: {text}` |
| Nachricht | der Text der Nachricht (eine Zeile, `\n`-Escapes wie in der JSON) |
| Trainer, Begegnung, Personal | je Feld aus `RawRecord::FIELDS` (Trainer: Eigenschaften, dann je Teammitglied mit Präfix `Team {n} `) `{name}: {Wert}`, `Bytes` als Hex |
| Lernset | je Eintrag `Lv {level}: Attacke {move}` |
| Entwicklung | je Eintrag `{methode} {parameter} → {ziel}` (Zahlen) |

  Pokémon ergibt bis zu drei `ResetChange` (Personal, Lernset, Entwicklung), nur die abweichenden.

- [ ] **Step 1: Scheiternde Tests** (synthetisches Projekt; Original = `ProjectOriginal::new` auf einer Kopie vor der Bearbeitung):
  - `target_json_is_tagged_snake_case` (`{"kind":"text_message","archive":5,"message":2}` rund).
  - `labels_name_the_entity_in_german` (je Ziel der Text aus der Liste oben; `step_label`).
  - `each_store_kind_resets_to_the_original_and_undoes` (je Ziel Karte 0, Eventdatei 0, Textarchiv 0, Trainer 1, Pokémon 1, Begegnung 0: bearbeiten und speichern, `plan_reset` → `same == false`, `changes` nicht leer; `reset` → nach `save` jeder Member gleich dem Original; `undo` → `current_bytes` gleich dem bearbeiteten Stand).
  - `text_message_reset_changes_only_that_message` (Nachricht 1 und 2 geändert, Nachricht 1 zurück: 2 bleibt).
  - `text_archive_reset_drops_new_messages_and_keeps_ids` (angehängte Nachricht fällt weg; `id` der vorhandenen bleibt).
  - `a_new_entity_has_no_original` (Index = Anzahl der Basis → `availability == Some(NO_ORIGINAL)`, `reset` → `Refused(NO_ORIGINAL)`, Stapel leer); `a_new_message_has_no_original`.
  - `an_unchanged_entity_plans_same_and_records_nothing`.
  - `reset_of_the_placeholder_trainer_is_refused_with_its_reason` (Review Focus; Platin-Familie, `placeholder_trainer()`).
  - `base_counts_name_each_target_kind`.
- [ ] **Step 2: Gated Test** `tests/original.rs` `reset_of_each_store_kind_gives_the_base_member_back`: `vanilla_copy()`, Basis über `vanilla_base_rom()` und `BaseRom::open(path, None)` (einmal, für die Prüfung), Store auf der Kopie; je Ziel (Karte 3: Byte einer Bewegungsrecht-Zelle über `edit_map`; Eventdatei 3: erstes Objekt `x + 1`; Textarchiv 0 und Nachricht 0 von Archiv 1: `" Studio"` angehängt; Trainer 1: Level des ersten Teammitglieds `+ 1`; Pokémon 1: Basis-KP `+ 1`; Begegnungsdatei 1: erste Rate `+ 1`) bearbeiten, speichern, Member ≠ Basis prüfen; dann `BaseRom::open_unchecked(path, sha1)` + `reset` mit gemessener Zeit, speichern; Member-Dateien (Trainer beide, Pokémon drei, Textarchiv der Member unter `unpacked/`) **gleich `base.member(kind, i)`**; `ms ≤ 200` (Zeit inklusive `open_unchecked`); `undo` → `current_bytes` gleich dem bearbeiteten Stand. Druckt `reset <game_code> <ziel> ms=<n>` je Ziel.
- [ ] **Step 3: Lauf** `cargo test -p g4-kernel --all-features original` → FAIL.
- [ ] **Step 4: Umsetzen** (`original.rs`, `preview.rs`, `base.rs`, `test_support.rs`).
- [ ] **Step 5: Lauf** → PASS; gated einmal auf Platin DE: `G4_TEST_PROJECT="<Roms>/Pokemon - Platin-Edition (Germany)_DSPRE_contents" cargo test -p g4-kernel --all-features --test original -- --nocapture --test-threads 1` → alle Ziele gleich, Zeiten ≤ 200 ms.
- [ ] **Step 6: Doku** `docs/formats/project_base.md`, neuer Abschnitt „Auf Original zurücksetzen“: Ziele und Byte-Form je Ziel, was nicht geht (neu, gesperrt, ARM9-Patches, Overlays), Undo-Schritt und Label, Basis je Aufruf ohne zweite SHA-1.
- [ ] **Step 7: Commit** `feat(kernel): reset maps, event files, texts, trainers, species and encounters to the base ROM as one undo step` (mit Trailer).

### Task 3: Header Feld für Feld

**Files:**
- Modify: `crates/g4-kernel/src/original.rs` (Header-Arm, `header_fields`), `crates/g4-kernel/tests/original.rs` (gated)

**Interfaces:**
- Consumes: Task 2; `Store::header_table`, `Tx::edit_header`, `Original::header_table`.
- Produces:

```rust
/// The fields of `header` as serde names them inside its family variant, with their JSON values.
pub fn header_fields(header: &MapHeader) -> Vec<(String, serde_json::Value)>;
```

- `plan_reset` für `Header { id }`: `fields` = die abweichenden Felder (`field` = serde-Name, `before`/`after` = JSON-Wert als Text), `changes` = eine Änderung mit Zeilen `{field}: {wert}` beider Seiten. `reset(…, fields)`: `None` = alle abweichenden; ein Name, der nicht abweicht oder nicht existiert → `Refused(format!("Unbekanntes oder unverändertes Feld: {name}"))`; das neue Objekt = aktuelles mit den gewählten Werten der Basis, über serde zurück zu `MapHeader`, dann `tx.edit_header(id, |h| *h = neu)`. Id ≥ Anzahl der Basis-Header → `NO_ORIGINAL`.

- [ ] **Step 1: Scheiternde Tests:** `header_fields_follow_serde_names` (Platin: `music_day_id`, `weather_id`, … ohne Variantenhülle), `header_plan_lists_only_the_differing_fields`, `header_reset_of_chosen_fields_keeps_the_others`, `header_reset_without_fields_takes_all`, `header_reset_refuses_an_unknown_field`, `header_beyond_the_base_has_no_original`.
- [ ] **Step 2: Gated Test** `header_reset_field_by_field_gives_the_base_record_back`: Header 3, `music_day_id` und `weather_id` geändert, gespeichert; nur `weather_id` zurück → gespeicherter Datensatz unterscheidet sich nur noch in `music_day_id`; Rest zurück → 24 Byte des Datensatzes (ARM9-Ausschnitt) gleich `write_map_header_to_bytes(&base.header_table()?[3])`. Zweiter Teil auf einer zweiten Kopie, nur wenn `store.dynamic_headers_applied()` falsch ist und die Familie den Patch kennt: `apply_dynamic_headers`, Header 3 ändern, speichern, zurücksetzen, speichern → `unpacked/dynamicHeaders/0003` gleich dem Basis-Datensatz. Druckt `reset <game_code> header ms=<n>`.
- [ ] **Step 3: Lauf** → FAIL; **Step 4: Umsetzen**; **Step 5: Lauf** → PASS (gated einmal auf HeartGold DE).
- [ ] **Step 6: Commit** `feat(kernel): reset a map header field by field through the codec` (mit Trailer).

### Task 4: Scripts, Texturen und Modelle als Dateischritt

**Files:**
- Modify: `crates/g4-kernel/src/scripts.rs` (`decompile_member`, `state_entry`, `set_state_entry`, `reset_script`), `crates/g4-kernel/src/original.rs` (Arme für Script und die vier Datei-Ziele, Vorschau), `crates/g4-kernel/src/store.rs` (Aufruf von `set_state_entry`), `crates/g4-kernel/tests/original.rs` (gated)

**Interfaces:**
- Consumes: Task 1 `FileStep`, `FileChange`, `RotomEntryChange`, `record_file_step`; Task 2.
- Produces:

```rust
// scripts.rs
/// Decompiles `bytes` as script file `file` in the project's context (database, constants, header
/// table) without writing; `None` when rotom cannot decompile it. Also the quirks rotom recorded.
pub fn decompile_member(project: &Project, file: u16, bytes: &[u8]) -> Result<Option<(ScriptSource, rotom::BinaryQuirk)>, KernelError>;
pub(crate) fn state_entry(project: &Project, source: &Path) -> Result<Option<serde_json::Value>, KernelError>;
pub(crate) fn set_state_entry(project: &Project, source: &Path, entry: Option<&serde_json::Value>) -> Result<(), KernelError>;
/// Writes `base_member` as member `file`, its decompiled source (removing a source of the other kind,
/// or every source when it does not decompile) and the compile state entry ("decompiled", quirks);
/// without rotom.toml only the member. Returns the step for `Store::record_file_step`.
pub fn reset_script(project: &Project, file: u16, base_member: &[u8]) -> Result<FileStep, KernelError>;
```

- Dateien eines Script-Schritts: Member, `expanded/scripts/NNNN.rotom`, `expanded/scripts/NNNN.json` (je mit `before`/`after`, `None` = keine Datei); `rotom_entry.source` = der projektrelative Pfad der Quelle, `before` = alter Eintrag, `after` = neuer. Der neue Eintrag: Status „decompiled“, `source_hash` und `output_hash` mit derselben Hashfunktion, die rotoms `compile_project` für die Prüfung „veraltet“ benutzt (in `rotom/src/project/compile.rs` nachsehen), Quirks aus `decompile_member`.
- Datei-Ziele (Texturen, Modelle): `project.ensure_unpacked(kind)`, Member-Datei schreiben (`write_atomic`), ein `FileStep` mit dieser einen Datei. `plan_reset` liefert für sie `Größe: {n} Byte` und `xxh3: {16 Stellen}` je Seite, für Scripts die Quelle als Text (Level-Script: JSON) bzw. `Binär, {n} Byte`; `writes_files == true`.
- `reset` für Datei-Ziele: Basis-Member gleich Datei → `recorded == false`, nichts geschrieben; sonst schreiben, `store.record_file_step(&step_label(target), step)`.

- [ ] **Step 1: Scheiternde Tests** (synthetisches Projekt, Texturen und Gebäudemodelle sind dort gepackt): `texture_reset_writes_the_member_and_undo_restores_it`, `model_reset_writes_the_member_and_undo_restores_it`, `an_unchanged_texture_writes_nothing`; im Testmodul von `scripts.rs` mit einer kleinen adoptierten Fixture (wie die bestehenden rotom-Tests): `reset_script_writes_member_source_and_a_fresh_state_entry` (danach kompiliert `compile` die Datei nicht: `!report.compiled.contains(&file)`), `reset_script_of_an_undecompilable_member_removes_the_source`, `undo_of_a_script_reset_restores_source_member_and_entry`.
- [ ] **Step 2: Gated Test** `script_texture_and_model_reset_give_the_base_member_back`: auf der Kopie (ist sie nicht adoptiert, zuerst `scripts::adopt`, Dauer drucken) an die erste Datei mit `SourceKind::Script` ein Script `script Studio #99:\n\tEnd\n` anhängen, `scripts::compile`, Member ≠ Basis; `reset` → Member gleich `base.member(Script, file)`; `scripts::compile` danach kompiliert die Datei nicht und der Member bleibt gleich; `undo` → Quelle und Member wie vor dem Reset. Kartentextur 0 und Gebäudemodell 0: ein Byte in der Mitte gekippt, `reset` → gleich der Basis, `undo` → gekippt. Druckt `reset <game_code> <ziel> ms=<n>`, Grenze 200 ms je Reset.
- [ ] **Step 3: Lauf** → FAIL; **Step 4: Umsetzen**; **Step 5: Lauf** → PASS (gated einmal auf Platin DE).
- [ ] **Step 6: Commit** `feat(kernel): reset scripts, textures and models to the base ROM as file steps undo writes back` (mit Trailer).

### Task 5: Liste „geändert gegenüber Original“ aus den Hashes des Graphs

**Files:**
- Create: `crates/g4-kernel/src/original/base_hashes.rs`, `crates/g4-kernel/src/original/changed.rs` (+ Tests je Datei)
- Modify: `crates/g4-kernel/src/original.rs` (Untermodule, Re-Exporte), `crates/g4-kernel/src/build/steps.rs` (nur wenn nötig: `pack_entries`, siehe Voraussetzungen), `crates/g4-kernel/tests/original.rs` (gated), `docs/formats/build_state.md` (Absatz „Nachgetragene Hashes“)

**Interfaces:**
- Consumes: 4.2 `BuildState`, `StepRecord`, `Fp`, `fingerprint::{stat, is_racy}`, `pack_entries`; Task 2 `Original`, `ResetTarget`; `scripts::numbered_files`; `Project::read_packed`, `Project::read_headers`.
- Produces:

```rust
pub const BASE_HASHES_VERSION: u32 = 1;
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BaseHashes { pub version: u32, pub sha1: String, pub archives: BTreeMap<String, Vec<String>> } // Pfad `files/…` mit `/` → xxh3 je Member, 16 Hex
pub fn base_hashes(original: &dyn Original, packed: &[PathBuf], cache: Option<&Path>) -> Result<BaseHashes, KernelError>;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)] #[serde(rename_all = "snake_case")]
pub enum ChangeState { Changed, New, Missing }
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ChangedEntry { pub group: String, pub label: String, pub state: ChangeState,
                          pub target: Option<ResetTarget>, pub assets: Vec<AssetId>, pub folder: Option<String> }
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ChangedReport { pub entries: Vec<ChangedEntry>, pub files: u32, pub hashed: u32, pub recorded: u32,
                           pub unbuilt: Vec<String>, pub ms: u64 }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)] pub struct ChangedProgress { pub done: u32, pub total: u32 }
pub fn changed_from_original(project: &Project, original: &dyn Original, cache: Option<&Path>,
                             progress: &mut dyn FnMut(ChangedProgress)) -> Result<ChangedReport, KernelError>;
```

- `base_hashes`: liest `<cache>/base/<sha1>.json` (fehlt, kaputt, andere Version oder SHA-1 → neu), rechnet fehlende Archive aus `original.narc_at`, schreibt atomar (Temp-Datei im selben Ordner, `rename`), wenn etwas dazukam.
- Projektseite je Pack-Eintrag `(Ordner, NARC)`:
  1. `unpacked/<Ordner>/` fehlt: NARC des Projekts lesen, Member im Speicher hashen.
  2. Eintrag `steps["pack:<Ordner>"]` fehlt: jede Datei `NNNN` hashen (Fortschritt), Ordner in `unbuilt`, nichts speichern.
  3. Sonst je Datei `stat`: gleicher `stat` wie aufgezeichnet und `xxh3` vorhanden → aufgezeichneter Hash; gleicher `stat`, kein `xxh3` und `!is_racy(aufgezeichnet, scanned_at_ns)` → hashen **und vormerken**; jeder andere Fall (anderer `stat`, neue Datei) → hashen, **nicht** vormerken.
  4. Vorgemerkte Hashes in **einem** `BuildState::update`; innerhalb der Closure nur setzen, wo der Eintrag noch genau den gesehenen `stat` trägt und `xxh3` noch `None` ist. `recorded` = Zahl der gesetzten Hashes.
- Einträge: Index < Basis-Anzahl und Hash ungleich → `Changed`; Index ≥ Basis-Anzahl → `New` ohne `target`; Index fehlt im Projekt, aber in der Basis → `Missing` ohne `target`. Zusammenfassen: Trainer-Eigenschaften und -Team → ein Eintrag `Trainer`; Personal, Lernset, Entwicklung → ein Eintrag `Pokémon`. Ordner → Art über `ArchiveMap::unpacked_name`; Ordner ohne Ziel-Art → `target: None`, `label` `"{Gruppe} {NNNN}"`. Header: `project.read_headers()` gegen `original.header_table()` über `==`, abweichend → `Header`, darüber hinaus → `New`.
- Gruppen in dieser Reihenfolge: Karten, Eventdateien, Textarchive, Trainer, Pokémon, Begegnungen, Header, Scriptdateien, Kartentexturen, Gebäudetexturen, Gebäudemodelle, Innenraum-Gebäudemodelle, dann Matrizen, Gebietsdaten, Attacken, Items und die übrigen Ordner unter ihrem Namen; innerhalb nach Index. Fortschritt alle 256 gehashten Dateien und am Ende.

- [ ] **Step 1: Scheiternde Tests** (synthetisches Projekt; Original = `ProjectOriginal` einer Kopie; Build-Einträge schreibt der Test selbst über `BuildState::update` mit `fingerprint::stat`, mtimes über `File::set_modified` drei Sekunden in die Vergangenheit, damit sie nicht racy sind):
  - `base_hashes_are_cached_per_sha1` (zweiter Aufruf liest die Datei; kaputte Datei → neu gerechnet und ersetzt).
  - `an_unchanged_project_lists_nothing`.
  - `a_changed_map_member_is_listed_with_its_target`.
  - `trainer_parts_make_one_entry`, `species_parts_make_one_entry`.
  - `a_member_beyond_the_base_is_new_without_target`.
  - `a_header_differing_by_codec_is_listed`.
  - `a_changed_matrix_is_listed_without_reset`.
  - `missing_hashes_are_filled_into_state_json_once` (erster Lauf `recorded > 0`, zweiter `hashed == 0`).
  - `changed_list_never_records_a_hash_for_a_file_changed_since_the_build` (Review Focus: Datei gleicher Größe neu geschrieben, mtime neu → als geändert gelistet; der Eintrag in `state.json` behält `xxh3: None` und den alten `stat`).
  - `an_archive_without_a_build_record_is_hashed_and_named_unbuilt`.
  - `progress_reaches_the_total`.
- [ ] **Step 2: Gated Test** `changed_list_after_edits_names_them_and_after_reset_not`: Kopie, `build::build` (Zeit drucken), `changed_from_original` → leer (Vanilla), Zeit `first_ms`; zweiter Lauf `second_ms ≤ 2000` und `hashed == 0`; Karte 3 und Trainer 1 ändern, speichern → beide gelistet (ohne erneuten Build: der `stat` weicht ab); Karte 3 zurücksetzen, speichern → nur noch Trainer 1. Druckt `changed <game_code> first_ms=<n> second_ms=<n> files=<n> recorded=<n>`.
- [ ] **Step 3: Lauf** → FAIL; **Step 4: Umsetzen**; **Step 5: Lauf** → PASS (gated einmal auf HeartGold US).
- [ ] **Step 6: Doku** `docs/formats/build_state.md`, Absatz „Nachgetragene Hashes (4.1b)“: wer `xxh3` in vorhandene Einträge schreibt, die Regel aus Schritt 3/4, dass das die Entscheidungen des Builds nicht ändert.
- [ ] **Step 7: Commit** `feat(kernel): list what differs from the base ROM from member hashes of the build graph` (mit Trailer).

### Task 6: App-Befehle, Brücke und MCP

**Files:**
- Create: `apps/studio/src-tauri/src/commands/original.rs` (+ Tests)
- Modify: `src/commands/mod.rs`, `src/commands/base.rs` (`pub(crate) fn found_base(state, root) -> Result<(BaseMark, Located), CmdError>` über `base_of`, `pub(crate) fn base_missing`), `src/base_roms.rs` (`configure_cache`, `cache_dir`), `src/lib.rs` (`setup_agents`: `state.base_roms.configure_cache(app.path().app_cache_dir().ok())`; vier Handler am Ende von `generate_handler!`), `src/bridge/http.rs` (`config.app_data` → Cache `<app_data>/cache`), `src/commands/scripts.rs` (`save_source` und `save_level_script`: `store.forget_asset_history(AssetId { kind: Script, index: file })`), `src/commands/dispatch.rs` (anhängen), `src/mcp/access.rs`, `src/mcp/catalog.rs`, `src/mcp/tools.rs`, `docs/mcp.md`

**Interfaces:**
- Consumes: Task 2 bis 5; 4.1a `base_of`, `BaseRoms::locate`, `NO_MARK`.
- Produces:

```rust
#[derive(Debug, Serialize)] #[serde(tag = "state", rename_all = "snake_case")]
pub enum OriginalStatusDto {
    Ready { label: String, sha1: String, counts: BTreeMap<&'static str, usize> },
    Unavailable { reason: String },
}
pub fn original_status_impl(state: &AppState) -> Result<OriginalStatusDto, CmdError>;
pub fn plan_reset_impl(state: &AppState, target: ResetTarget) -> Result<ResetPlan, CmdError>;
pub fn reset_to_original_impl(state: &AppState, target: ResetTarget, fields: Option<Vec<String>>) -> Result<ResetResult, CmdError>;
pub fn changed_from_original_impl(state: &AppState, progress: &mut dyn FnMut(ChangedProgress)) -> Result<ChangedReport, CmdError>;
impl BaseRoms { pub fn configure_cache(&self, dir: Option<PathBuf>); pub fn cache_dir(&self) -> Option<PathBuf>; }
```

- Ohne gefundene Basis: `original_status` → `Unavailable { reason }` (`NO_MARK`, `missing_text`, Grund der Marke); die drei anderen → `CmdError { code: "base_missing", message: <derselbe Grund> }`. Nicht verfügbares Ziel → `CmdError { code: "no_original", message: <NO_ORIGINAL oder NEW_MESSAGE> }`. Gesperrtes Asset → `read_only` wie heute.
- `reset_to_original_impl`: Basis finden (`found_base`), `BaseRom::open_unchecked`, unter dem Session-Lock `original::reset`; danach `session.after_write()`, bei `target.writes_files()` zusätzlich `session.files_written()`. `changed_from_original_impl` hält den Session-Lock nicht während des Hashens (Projekt-`Arc` holen, Lock freigeben, rechnen). Tauri: `plan_reset`, `reset_to_original`, `original_status` und `changed_from_original` sind `async` mit `spawn_blocking`; `changed_from_original` nimmt `progress: Channel<ChangedProgress>`.
- Zugriff (anhängen): `("original_status", Read)`, `("plan_reset", Read)`, `("reset_to_original", Write)`, `("changed_from_original", Read)`. Katalog (englisch, drei Werkzeuge): `changed_from_original` („List what differs from the base ROM“, ohne Argumente, nennt den ersten langsamen Lauf), `plan_reset` („Plan reset to original“, `{ target }`, `data_note!()`), `reset_to_original` („Reset to original“, `{ target, fields? }`, „one undo step; scripts, textures and models are written at once“). Schema von `target`: Objekt mit `kind` (Enum der 13 Werte) und den Zahlenfeldern `index`, `archive`, `message`, `id`, `file` (je nach Art Pflicht, im Text beschrieben). `tools.rs`: `not_in_batch("reset_to_original", args)` ist wahr für `kind` ∈ {`script`, `map_texture`, `building_texture`, `building_model`, `interior_building_model`}.

- [ ] **Step 1: Scheiternde Tests** (`commands/original.rs`, synthetisches Projekt wie in `commands/base.rs`):
  - `status_without_a_mark_is_unavailable_with_no_mark`, `status_with_a_missing_rom_names_label_and_sha1`.
  - `reset_without_a_found_base_answers_base_missing_and_records_nothing` (Review Focus; Marke da, ROM verschoben; `history` leer).
  - `changed_without_a_base_answers_base_missing`.
  - `save_script_forgets_the_file_steps_of_that_script` (Dateischritt per `record_file_step` anlegen, `save_script_impl`, `history` ohne ihn).
  - gated (übersprungen ohne `G4_TEST_PROJECT`): `reset_over_the_command_path_on_the_vanilla_copy` (Kopie öffnen, `register_base_rom_impl`/`assign_base_impl` mit `vanilla_base_rom()`, Trainer 1 über `data_edit` ändern, `save`, `reset_to_original_impl` → `recorded`, `save`, Member gleich Basis; `undo_impl` gibt das Asset zurück).
  - `access.rs`: die vier Zeilen; `catalog.rs`: `the_curated_set_is_the_decided_one` mit `plan_reset` und `changed_from_original` unter den lesenden, `reset_to_original` unter den schreibenden; `tool_count_is_about_thirty` bleibt grün (49); `tools.rs`: `batch_refuses_reset_to_original_of_a_script_and_takes_one_of_a_map`; `dispatch.rs`: Beispielargumente im Test aller Befehle; `bridge/http.rs`: `app_data_configures_the_base_hash_cache`.
- [ ] **Step 2: Lauf** `cargo test -p studio --all-features` → FAIL; **Step 3: Umsetzen**; **Step 4: Lauf** → PASS, dazu `cargo test -p studio --test mcp_stdio`.
- [ ] **Step 5: Doku** `docs/mcp.md`: drei Werkzeuge in der Werkzeugtabelle (Zugriff, Undo, Batch-Ausnahme), `original_status` unter den Befehlen, Hinweis „ohne Basis-ROM `base_missing` mit Grund“, Cache nur in der App.
- [ ] **Step 6: Commit** `feat(studio): original status, plan and reset to original, changed list as commands and MCP tools` (mit Trailer).

### Task 7: Frontend-Bausteine: Kontextmenü, API, Zustand, Dialog

**Files:**
- Create: `apps/studio/src/api.original.ts` (+ `api.original.test.ts`), `apps/studio/src/original/useOriginal.ts` (+ test), `apps/studio/src/original/OriginalContext.tsx` (Provider, `useReset`), `apps/studio/src/original/ResetDialog.tsx` (+ `ResetDialog.module.css`, test), `apps/studio/src/original/resetEntry.ts` (+ test)
- Modify: `apps/studio/src/api.ts` (`export * from "./api.original";`), `apps/studio/src/ui/primitives/ListRow.tsx` und `TreeRow.tsx` (+ Tests: `contextMenu`), `apps/studio/src/ui/primitives/Menu.tsx` (gemeinsamer Teil für das Kontextmenü, Muster `Reference`), `apps/studio/src/ui/gallery/pages/ContentPage.tsx` (Beispiel mit Kontextmenü), `apps/studio/src/App.tsx` (Provider; nach einem Reset `reloadChanged(result.changed, true)`; nach einem `agent-event` mit `reset_to_original` wie bei den anderen Werkzeugen)

**Interfaces:**
- Consumes: Task 6 (Befehle, DTOs); `Dialog`, `BeforeAfter`, `Notice`, `Badge`, `Checkbox`, `Menu`/`MenuEntry`; `headerFieldSpecs(family)` aus `workbench/inspector/headerSections.ts` für die Feldnamen; `scriptDirtyStore` für ungespeicherte Scripts.
- Produces (TS):

```ts
export type ResetTarget =
  | { kind: "map" | "event_file" | "text_archive" | "trainer" | "species" | "encounter" | "map_texture" | "building_texture" | "building_model" | "interior_building_model"; index: number }
  | { kind: "text_message"; archive: number; message: number } | { kind: "header"; id: number } | { kind: "script"; file: number };
export interface ResetChange { label: string; before: string; after: string }
export interface HeaderFieldDiff { field: string; before: string; after: string }
export interface ResetPlan { target: ResetTarget; label: string; changes: ResetChange[]; fields: HeaderFieldDiff[]; writes_files: boolean; same: boolean }
export interface ResetResult { changed: AssetId[]; recorded: boolean; ms: number }
export type OriginalStatus = { state: "ready"; label: string; sha1: string; counts: Record<string, number> } | { state: "unavailable"; reason: string };
export type ChangeState = "changed" | "new" | "missing";
export interface ChangedEntry { group: string; label: string; state: ChangeState; target: ResetTarget | null; assets: AssetId[]; folder: string | null }
export interface ChangedReport { entries: ChangedEntry[]; files: number; hashed: number; recorded: number; unbuilt: string[]; ms: number }
export interface ChangedProgress { done: number; total: number }
export const originalStatus: () => Promise<OriginalStatus>;
export const planReset: (target: ResetTarget) => Promise<ResetPlan>;
export const resetToOriginal: (target: ResetTarget, fields?: string[]) => Promise<ResetResult>;
export const changedFromOriginal: (onProgress: (p: ChangedProgress) => void) => Promise<ChangedReport>;
export function targetIndex(target: ResetTarget): number;          // index, archive, id oder file
// original/useOriginal.ts: liest `original_status` einmal je Projekt und nach base-Änderung (assign, register)
export function useOriginal(): { status: OriginalStatus | null; refresh: () => Promise<void> };
// original/OriginalContext.tsx
export function OriginalProvider(props: { children: ReactNode; onReset: (result: ResetResult) => void | Promise<void> }): JSX.Element;
export function useReset(): { status: OriginalStatus | null; open: (target: ResetTarget, opts?: { scriptDirty?: boolean }) => void };
// original/resetEntry.ts
export function resetEntry(target: ResetTarget, status: OriginalStatus | null, open: (t: ResetTarget) => void, label?: string): MenuItemSpec;
// ui/primitives
export type ListRowProps = { /* wie heute */ contextMenu?: readonly MenuEntry[] };   // TreeRowProps erbt es
```

- `resetEntry`: Label Standard „Auf Original zurücksetzen…“ (eigenes Label, wo zwei Ziele nebeneinander stehen), Symbol `undo`; gesperrt mit `detail` „Basis-ROM fehlt“ (`unavailable`), „Kein Original“ (`targetIndex ≥ counts[kind]`, für `text_message` die Anzahl der Archive), „Wird gelesen …“ (`status === null`).
- Kontextmenü: `contextMenu` gesetzt → die Zeile ist `ContextMenu.Trigger`; Shift+F10 und die Kontextmenü-Taste auf der fokussierten Zeile öffnen es an ihrer Unterkante (wie `Deck.tsx` es für Tabs tut); ohne `contextMenu` bleibt die Zeile, wie sie ist.
- Texte des Dialogs wörtlich:

| Stelle | Text |
|---|---|
| Titel | `{label} auf Original zurücksetzen?` (z. B. „Karte 0003 auf Original zurücksetzen?“) |
| Beschreibung | `Stellt den Stand der Basis-ROM {status.label} wieder her. ⌘Z nimmt es zurück.` |
| Laden | `Lade Vergleich …` (`role="status"`) |
| je Änderung | `BeforeAfter` mit `label` = `change.label` |
| Header | je abweichendes Feld eine `Checkbox` (an) mit dem Feldnamen aus `headerFieldSpecs`, daneben `{before} → {after}` in Mono; ohne Häkchen ist „Zurücksetzen“ gesperrt |
| Dateiziel | `Notice` Info „Wird sofort geschrieben; ⌘Z nimmt es zurück.“ |
| ungespeichertes Script | `Notice` Warnung „Ungespeicherte Änderungen an dieser Scriptdatei gehen verloren.“ |
| ungespeichertes Asset (dirty) | `Notice` Info „Auch ungespeicherte Änderungen werden ersetzt; ⌘Z stellt sie wieder her.“ |
| schon Original (`same`) | `Notice` Info „Schon im Stand der Basis-ROM.“, nur „Schließen“ |
| läuft | `Setze zurück …` |
| Fehler | `Notice` Fehler mit `errorText(e)` (bei `base_missing`/`no_original` der Satz des Kernels), Dialog bleibt offen |
| Tasten | „Abbrechen“, „Zurücksetzen“ (primär) |
| danach | App-Meldung `Auf Original zurückgesetzt: {label}.` |

- [ ] **Step 1: Scheiternde Tests:** `api.original.test.ts`: `original_commands_send_the_documented_arguments`, `target_index_reads_the_number_of_each_kind`; `resetEntry.test.ts`: `entry_is_disabled_without_a_base_and_says_why`, `entry_is_disabled_for_a_new_index_with_kein_original`, `entry_opens_the_dialog_with_its_target`; `useOriginal.test.tsx`: `reads_the_status_once_per_project`, `reads_again_after_the_base_changed`; `ResetDialog.test.tsx`: `shows_before_and_after_per_change_then_resets_on_confirm`, `header_fields_can_be_unchecked_and_only_checked_ones_are_sent`, `a_file_target_says_it_is_written_at_once`, `an_unsaved_script_warns`, `same_offers_only_schliessen`, `a_failed_reset_shows_the_reason_and_keeps_the_dialog` (Review Focus: `base_missing`); `ListRow.test.tsx`/`TreeRow.test.tsx`: `context_menu_opens_on_right_click_and_shift_f10`, `without_context_menu_nothing_changes`.
- [ ] **Step 2: Lauf** `cd apps/studio && pnpm test` → FAIL; **Step 3: Umsetzen** (CSS-Modul nur mit Tokens); **Step 4: Lauf** → PASS, `pnpm build`, `bash scripts/check-styles.sh`, `bash scripts/check-tokens.sh` grün.
- [ ] **Step 5: Commit** `feat(ui): context menu on list and tree rows, reset dialog with before and after` (mit Trailer).

### Task 8: Einträge in den Köpfen und Listen

**Files:**
- Modify: `apps/studio/src/workbench/inspector/MatrixCellEditor.tsx` (Karte), `EventEditor.tsx` (Eventdatei der gezeigten Datei), `HeaderEditor.tsx` (Header), `AreaEditor.tsx` (Kartentextur und Gebäudetextur des Area-Data, Labels „Kartentextur auf Original zurücksetzen…“, „Gebäudetextur auf Original zurücksetzen…“), `BuildingEditor.tsx` („Gebäudemodell auf Original zurücksetzen…“, Innenraum-Modell in HGSS wie der Editor es wählt), `workbench/SceneHierarchy.tsx` (Kontextmenü: Header-Zeile „Header auf Original zurücksetzen…“, Event-Gruppen und Events „Eventdatei auf Original zurücksetzen…“, Script-Zeile „Scriptdatei auf Original zurücksetzen…“, Gebäude „Gebäudemodell auf Original zurücksetzen…“), `workbench/WorldTree.tsx` und `HeaderList.tsx` (Header-Zeilen, Script-Zeilen), `workbench/text/TextList.tsx` (Archiv-Zeilen), `workbench/text/TextHand.tsx` (Menü „Aktionen“ im Kopf: „Nachricht auf Original zurücksetzen…“, „Ganzes Textarchiv auf Original zurücksetzen…“), `data/trainer/TrainerView.tsx` und `TrainerList.tsx`, `data/pokedex/PokedexView.tsx` und `data/DataList.tsx` (`DataListRows.contextMenu?: (row: DataRowDto) => MenuEntry[]`), `data/encounters/EncounterView.tsx` und `EncounterList.tsx`, `workbench/script/ScriptPanel.tsx` (Menü „Aktionen“ im Kopf, gesperrt ohne Dokument), `reload.ts` (+ test: `MapTexture`/`BuildingTexture` → `{ kind: "models" }`, `BuildingModel`/`InteriorBuildingModel` → `{ kind: "buildings" }` und `{ kind: "models" }`)
- Test: je Datei der bestehende Test um die genannten Fälle

**Interfaces:**
- Consumes: Task 7 `useReset`, `resetEntry`, `contextMenu`; `InspectorHead.actions`; `Menu` mit `label="Aktionen"` und `trigger={<Icon name="more" />}` für die Köpfe ohne `InspectorHead` (Trainer, Pokédex, Begegnungen, Text-Hand, Script) im Muster des Inspector-Kopfs.

- [ ] **Step 1: Scheiternde Tests** (vorhandene Test-Helfer, `headTesting.ts` für den Kopf; der Status kommt aus einem Test-Provider):
  - `MatrixCellEditor.test.tsx` `head_offers_reset_of_the_map`; `EventEditor.test.tsx` `head_offers_reset_of_the_event_file`; `HeaderEditor.test.tsx` `head_offers_reset_of_the_header_and_locks_a_new_one` („Kein Original“ für eine Id ≥ Anzahl); `AreaEditor.test.tsx` `head_offers_both_textures`; `BuildingEditor.test.tsx` `head_offers_the_model`.
  - `SceneHierarchy.test.tsx` `rows_have_reset_in_their_context_menu`; `WorldTree.test.tsx` `header_and_script_rows_have_reset`; `TextList.test.tsx` `archive_rows_have_reset`; `TextHand.test.tsx` `menu_offers_message_and_archive_and_locks_a_new_message_in_the_plan`.
  - `TrainerView.test.tsx`/`TrainerList.test.tsx`, `PokedexView.test.tsx`/`PokedexList.test.tsx`, `EncounterView.test.tsx`/`EncounterList.test.tsx`: `offers_reset_in_head_and_list`, `without_a_base_the_entry_says_basis_rom_fehlt`.
  - `ScriptPanel.test.tsx` `menu_offers_reset_of_the_open_file_and_warns_when_unsaved`.
  - `reload.test.ts` `textures_and_models_reload_scene_parts`.
  - `App.test.tsx` `a_reset_reloads_what_changed_and_says_so` (Dialog bestätigen → `resetToOriginal`, dann Neuladen der Assets und Meldung).
- [ ] **Step 2: Lauf** → FAIL; **Step 3: Umsetzen**; **Step 4: Lauf** → PASS, `pnpm build`, Stil- und Token-Prüfung grün.
- [ ] **Step 5: Blick in der Test-Umgebung** (`docs/agent-gate.md`, „Je Lauf“, Kopie `_testkopien/4-1b-ui_DSPRE_contents` mit `4-1b-ui.nds`, Basis zugeordnet): Menü im Kopf, Kontextmenü einer Liste, Dialog Karte, Dialog Header, Dialog Script in Hell, Dunkel, Kompakt; Screenshots nach `target/agent/gate-<datum>/4-1b/` (nicht ins Repo).
- [ ] **Step 6: Commit** `feat(studio): reset to original in the heads and context menus of every decided kind` (mit Trailer).

### Task 9: Technik › Änderungen

**Files:**
- Create: `apps/studio/src/technik/ChangesSurface.tsx` (+ `ChangesSurface.module.css`, `ChangesSurface.test.tsx`), `apps/studio/src/technik/useChanges.ts` (+ test)
- Modify: `apps/studio/src/technik/TechnikArea.tsx` (+ test: Eintrag `aenderungen` nach `veroeffentlichen`, Untertitel, Weiche), `apps/studio/src/ui/primitives/icons.ts` (`changes: GitDiff`)

**Interfaces:**
- Consumes: Task 7 `changedFromOriginal`, `useReset`, `useOriginal`; die Liste der dirty Assets der Session (wie die Inspector-Köpfe sie bekommen); `Section`, `ListRow`, `Badge`, `Button`, `Notice`, `EmptyState`.
- Produces:

```ts
export interface ChangesApi { report: ChangedReport | null; progress: ChangedProgress | null; error: string | null; busy: boolean; compare: () => Promise<void> }
export function useChanges(): ChangesApi;        // vergleicht beim ersten Zeigen je Projekt und nach jedem Speichern und Reset, wenn Technik › Änderungen offen ist
export function ChangesSurface(props: { changes: ChangesApi | null; status: OriginalStatus | null; dirty: AssetId[]; onOpenProject: () => void }): JSX.Element;
```

- `TechnikEntry` bekommt `"aenderungen"`; Liste: „Projekt“, „Prüfen und Bauen“, „Veröffentlichen“, „Änderungen“ (Symbol `changes`), „Patches“, „Emulator“, „Werkzeuge“. Fläche `data-technik="aenderungen"`, je Gruppe eine `Section` (`data-changes-group=<group>`), je Eintrag eine `ListRow` (`data-change=<label>`).
- Texte wörtlich:

| Stelle | Text |
|---|---|
| Untertitel der Liste | ohne Projekt „Kein Projekt geöffnet“; `unavailable` „Basis-ROM fehlt“; läuft „Vergleiche …“; danach „Keine Abweichung“ bzw. „{n} Einträge weichen ab“ (1: „1 Eintrag weicht ab“) |
| ohne Projekt | `EmptyState` „Kein Projekt geöffnet.“ / „Öffne ein Projekt, dann zeigt Technik hier, was vom Original abweicht.“ / „Projekt öffnen“ |
| ohne Basis | `Notice` Warnung mit `status.reason`, darunter „Die Basis-ROM ordnest du in Technik › Veröffentlichen zu.“ |
| Kopf der Fläche | Titel „Änderungen“, Satz „Gegenüber der Basis-ROM {label}. Gezählt wird der gespeicherte Stand.“, Taste „Neu vergleichen“ (sekundär) |
| Fortschritt | „Vergleiche {done} von {total} Dateien …“ (`role="status"`); beim ersten Mal darunter „Das erste Mal dauert länger; danach geht es schnell.“ |
| `unbuilt` nicht leer | `Notice` Info „Noch nicht gebaut: {Ordner, kommagetrennt}. Nach dem nächsten Build geht der Vergleich schneller.“ |
| leer | „Alles wie in der Basis-ROM.“ |
| Zeile | Label; `Badge` „Geändert“ (Warnung), „Neu“ (Info), „Fehlt“ (Fehler); „Ungespeichert“ (`unsaved`), wenn ein Asset der Zeile dirty ist; Taste „Auf Original zurücksetzen…“ (sekundär, klein), gesperrt mit `title` „Kein Original“ (neu, fehlt) bzw. „Für diese Art gibt es kein Zurücksetzen.“ (`target === null`) |
| Fehler | `Notice` Fehler mit `errorText(e)` |

- [ ] **Step 1: Scheiternde Tests:** `useChanges.test.tsx`: `compares_once_when_shown_and_reports_progress`, `compares_again_after_save_and_reset`; `ChangesSurface.test.tsx`: `without_a_project_offers_to_open_one`, `without_a_base_names_the_reason_and_points_to_veroeffentlichen`, `groups_entries_with_badges_and_counts`, `a_row_without_target_has_a_locked_button_with_its_reason`, `an_unsaved_asset_is_marked`, `reset_from_a_row_opens_the_dialog`, `unbuilt_folders_are_named`; `TechnikArea.test.tsx`: `lists_seven_entries_with_aenderungen_after_veroeffentlichen`, `aenderungen_subtitle_counts_the_entries`.
- [ ] **Step 2: Lauf** → FAIL; **Step 3: Umsetzen**; **Step 4: Lauf** → PASS, `pnpm build`, Stil- und Token-Prüfung grün.
- [ ] **Step 5: Blick in der Test-Umgebung** (wie Task 8): Technik › Änderungen mit Fortschritt, mit zwei Einträgen, leer, ohne Basis; Hell, Dunkel, Kompakt.
- [ ] **Step 6: Commit** `feat(technik): Änderungen lists what differs from the base ROM with reset per row` (mit Trailer).

### Task 10: Design-System fortschreiben

**Files:**
- Create: `design-system/openpencil/beispiele/auf-original.body.js`, `design-system/openpencil/NACHTRAG-4-1b.md`
- Modify: `design-system/README.md` (Abschnitt „Nachtrag 4.1b“ nach „Nachtrag 4.1a“), `design-system/Audit-Design-System.md` (Zeilen „Kontextmenü“ und „Änderungen“ bzw. die Zeile zum Basis-ROM-Vergleich aus „Aus dem Design-System, später“: teilweise, 4.1b)

**Interfaces:**
- Consumes: die Texte und Zustände aus Task 7 bis 9, die Screenshots aus Task 8 Step 5 und Task 9 Step 5.

- [ ] **Step 1: README** „Nachtrag 4.1b“ nach dem Muster von „Nachtrag 4.1a“: Quelle (`original/*`, `technik/ChangesSurface.tsx`, `useChanges.ts`, `ui/primitives/ListRow.tsx`/`TreeRow.tsx`/`Menu.tsx`, die Köpfe aus Task 8), Gate-Verweis `docs/phase-4-1b-gate.md`, Vergleichsbilder unter `target/agent/gate-<datum>/4-1b/` (Namen `blick-<fläche>-<zustand>-<thema>.png`), neues Symbol `changes` (Phosphor `GitDiff`), keine neuen Tokens; Baustein-Erweiterung „Listenzeile und Baumzeile mit Kontextmenü“ (Rechtsklick, Shift+F10, Kontextmenü-Taste, gleiche Einträge wie das Menü „Aktionen“); der Menüeintrag „Auf Original zurücksetzen…“ (Symbol `undo`, gesperrt mit leiser Notiz „Kein Original“ / „Basis-ROM fehlt“); der Dialog mit allen Zuständen der Tabelle aus Task 7; die Fläche „Änderungen“ mit allen Texten aus Task 9; Abweichungen: keine Plakette „geändert“ im Kopf (Nicht in Phase 4), kein Bildvergleich bei Texturen und Modellen, kein „Öffnen“ und kein Umschalter in der Liste (4.4), Karten nur im Kopf des Zellen-Inspectors; die `.fig` kennt das noch nicht.
- [ ] **Step 2: Vorlage** `beispiele/auf-original.body.js` nach dem Muster von `veroeffentlichen.body.js` mit den Bausteinen aus `jsx-lib-app.js` und `v('token')`-Farben, `MODE`: `'karte'` (Frame „App: Welt, Auf Original zurücksetzen“: Zellen-Inspector mit offenem Menü „Aktionen“ und darüber der Dialog „Karte 0003 auf Original zurücksetzen?“ mit `BeforeAfter`), `'header'` (Frame „App: Welt, Header zurücksetzen“: Dialog mit drei Feldern und Häkchen), `'aenderungen'` (Frame „App: Technik, Änderungen“: sieben Listeneinträge, „Änderungen“ gewählt, Gruppen „Karten“ (1), „Trainer“ (1), „Matrizen“ (1, ohne Zurücksetzen), eine Zeile „Ungespeichert“); Projekt „Sinnoh-Remix“, Platin DE.
- [ ] **Step 3: Anleitung** `NACHTRAG-4-1b.md` nach dem Muster von `NACHTRAG-4-1a.md`: „Die Datei wurde nicht angefasst“; drei Frames (Seiten „App: Welt …“ und „App: Texte, Technik, Dialoge“), Lage (nächste freie Spalte, x = Spalte × 1520, y = Reihe × 1000, vorher `get_page_tree`), Bauen (`bash ../build-screen.sh auf-original`, `MODE` je Frame, `op.mjs render`, Vergleich per `export_image` mit den Screenshots, erzeugte `.jsx` nicht einchecken), danach `fix_fig_text.mjs --rgb555` und README-Satz; was nicht gezeichnet werden muss (Laden, Fehler, Dateiziel-Hinweis, Kontextmenü der übrigen Listen, Fläche ohne Projekt).
- [ ] **Step 4: Audit** wie unter Files; `bash scripts/check-tokens.sh` grün.
- [ ] **Step 5: Commit** `docs(design-system): addendum 4.1b reset to original, context menu and Technik › Änderungen` (mit Trailer).

### Task 11: Gate 4.1b

**Files:**
- Create: `docs/phase-4-1b-gate.md`
- Modify: `docs/ROADMAP.md` (Übersicht und Punkt 4.1b mit Stand), `docs/agent-gate.md` (Gate-Datei in der Liste oben, Abschnitt „Phase 4.1b“ mit Befehlen und `data-*`), `docs/dev-setup.md` (gated Test `tests/original.rs` in der Tabelle `G4_TEST_PROJECT`)

**Interfaces:**
- Consumes: alles Vorige; die Ausgaben `reset <game> <ziel> ms=…` und `changed <game> …` der gated Tests.

- [ ] **Step 1: Gate-Dokument** `docs/phase-4-1b-gate.md` nach dem Muster von `docs/phase-4-1a-gate.md`: Kopf mit Stand und dem Gate-Text der Roadmap als Zitat; Legende „Wer“ (Agent, Agent + Blick, Hand) mit Verweis auf `docs/agent-gate.md`; Namen in der Oberfläche (Menü „Aktionen“, Einträge „… auf Original zurücksetzen…“, Dialogtitel, `[data-technik=aenderungen]`, `[data-changes-group]`, `[data-change]`); Vorbereitung (Kopien `_testkopien/4-1b-{pt,hgde,hgus}`, `4-1b-web`, `4-1b-mcp`, `4-1b-ui`, je mit `.nds` daneben; `cargo build --workspace --all-features`; `G4_BASE_ROM_DIR=<Roms>/_testkopien`; Worktree-Hinweise).
  - **(1) Automatik:** `scripts/gate.sh` grün; die gated Tests `reset_of_each_store_kind_gives_the_base_member_back`, `header_reset_field_by_field_gives_the_base_record_back`, `script_texture_and_model_reset_give_the_base_member_back`, `changed_list_after_edits_names_them_and_after_reset_not` und der gated Studio-Test `reset_over_the_command_path_on_the_vanilla_copy` auf Platin DE, HeartGold DE, HeartGold US, einzeln je Projekt mit `--nocapture --test-threads 1` für die Zeiten.
  - **(2) Szenario kopflos** (MCP über `studio mcp --project <Kopie> --base-rom <nds>`, je Spiel): `changed_from_original` leer; Trainer 1 über `data_edit`, Header 3 über `set_header`, Nachricht 0 von Archiv 1 über `edit_text`, `save`; `changed_from_original` nennt die drei; `plan_reset` je Ziel zeigt Vorher und Nachher; `reset_to_original` je Ziel (`ms` notieren), `save`; `changed_from_original` wieder leer; `g4k extract <nds> <tmp>` und `cmp` der drei Member gegen die extrahierte Basis; `undo` stellt die letzte Änderung wieder her; ein Ziel mit Index = Anzahl der Basis → `no_original`; `g4_batch` mit `reset_to_original` eines Scripts → abgelehnt.
  - **(3) Browser-Teil** (Test-Umgebung, `4-1b-web`, Basis zugeordnet): je Art aus der Entscheidung ändern (Karte, Eventdatei, Nachricht und ganzes Textarchiv, Trainer, Pokémon, Begegnung, Header mit einem abgewählten Feld, Script, Kartentextur, Gebäudemodell), „Auf Original zurücksetzen…“ über den Kopf oder das Kontextmenü (beides mindestens einmal, das Kontextmenü auch über Shift+F10), Dialog mit Vorher-Nachher, bestätigen, speichern, Technik › Änderungen ohne den Eintrag; ⌘Z stellt die Änderung wieder her; eine neue Entität (neuer Trainer) → Eintrag gesperrt mit „Kein Original“; die `.nds` umbenennen → Einträge gesperrt mit „Basis-ROM fehlt“, Technik › Änderungen nennt den Grund; MCP über die Brücke (`4-1b-mcp`, Freigabe „Lesen und bearbeiten“): `mcp.sh call reset_to_original '{"target":{"kind":"trainer","index":1}}'` ohne Rückfrage, der Verlauf zeigt den Agenten, die Oberfläche lädt den Trainer neu.
  - **(4) Hand und Blick beim Nutzer:** Kontextmenü mit Trackpad (Zwei-Finger-Klick) und Kontextmenü-Taste; Blick auf Dialog (Karte, Header, Script) und Technik › Änderungen in Hell, Dunkel, Kompakt (Screenshots aus Task 8 und 9).
  - **Ergebnis:** Tabelle Punkt / Wer / Ergebnis / Datum und die Zeittabelle:

    | Spiel | Karte | Eventdatei | Textarchiv | Nachricht | Trainer | Pokémon | Begegnung | Header | Script | Textur | Modell | Liste erst / warm (ms) |
    |---|---|---|---|---|---|---|---|---|---|---|---|---|
    | Platin DE | | | | | | | | | | | | |
    | HeartGold DE | | | | | | | | | | | | |
    | HeartGold US | | | | | | | | | | | | |

- [ ] **Step 2: Gate abarbeiten** (Agent: (1) bis (3)); Werte und Befunde eintragen; Befunde beheben (eigene Commits `fix(4.1b): …`), bis alles grün ist.
- [ ] **Step 3: Roadmap** `docs/ROADMAP.md`: Übersicht („4.1b Gate vom Agenten bestanden <Datum>, Blick beim Nutzer offen“) und am Ende des Punkts 4.1b „**Stand <Datum>:** Gate vom Agenten bestanden: je Art Member byte-gleich zur Basis auf allen drei Projekten, Reset <min>–<max> ms, Liste erst <…> s / warm <…> ms; Branch `phase-4-1b-auf-original`; [docs/phase-4-1b-gate.md](phase-4-1b-gate.md); beim Nutzer offen: Kontextmenü mit Trackpad, Blick“. `docs/agent-gate.md` und `docs/dev-setup.md` wie unter Files.
- [ ] **Step 4: Commit** `docs(4.1b): gate of reset to original, roadmap status` (mit Trailer).

---

## Offen für die Umsetzung

- **rotom im Projektkontext:** ob `decompile_member` mit der öffentlichen rotom-API (`ProjectContext`, `DecompileContext::for_project`, `decompile_to_ir`, `ir_to_source`) genau den Text erzeugt, den `decompile_project_files` schreiben würde, und welche Hashfunktion rotoms Zustand für `source_hash`/`output_hash` benutzt. Geht es nicht ohne Schreiben, dekompiliert `reset_script` mit `decompile_project_files(root, config, &[member])` an Ort und Stelle (die Bytes vorher sind ja gesichert) und die Vorschau zeigt die Quelle einer Dekompilierung in `.g4/tmp/`; der Gate-Test bleibt derselbe.
- **Quirks nach ⌘Z:** der Dateischritt stellt den alten Zustandseintrag wieder her; ob ein vom Nutzer **danach** geänderter Quelltext mit den alten Quirks kompiliert, prüft nur der Gate-Test indirekt.
- **Zeitbudget:** `BaseRom::open_unchecked` liest 128 MiB (14–56 ms gemessen); bleibt ein Reset unter Last über 0,2 s, ist die erste Wahl ein `mmap`-freies Lesen nur des FAT-Bereichs und der gebrauchten Datei (eigene kleine Funktion in `base.rs`), nicht ein Cache im Speicher (Entscheidung „nicht im Speicher halten“).
- **Textarchive mit neuerer JSON:** ist nach 4.2 die JSON eines Archivs neuer als sein Member (DSPRE hat die JSON geändert, noch nicht gebaut), vergleicht die Liste den alten Member und zeigt das Archiv womöglich nicht; der Reset selbst arbeitet auf dem Store-Wert und ist richtig. Ob die Liste für Textarchive die JSON einbeziehen soll, entscheidet 4.4 (Diff je Nachricht).
- **Weitere Arten:** Matrix, Gebietsdaten, Attacken und Items wären mit `Tx::replace` je eine Zeile; die Entscheidung nennt sie nicht, darum stehen sie nur in der Liste. Ein Nachtrag braucht eine Nutzerentscheidung.
- **Kontextmenü:** ob Shift+F10 in WebKit (Tauri auf macOS) als Tastenkombination ankommt oder vom System abgefangen wird, zeigt erst die Hand (4); die Kontextmenü-Taste fehlt auf Mac-Tastaturen, Shift+F10 ist der dokumentierte Weg.
- **„Öffnen“ aus der Liste** und der Umschalter „gegen Basis / gegen letzten Commit“ kommen mit 4.4.
- **Konflikte mit 4.3a** (parallel, legt `textproject` an und ändert `g4k.rs`, `access.rs`, `catalog.rs`, `dispatch.rs`):
  - `crates/g4-kernel/src/lib.rs`: beide hängen ein `pub mod` an.
  - `access.rs` (`COMMAND_KINDS` und der Test der App-only-/unumkehrbaren Gruppen), `dispatch.rs` (`COMMANDS`, Arme, Beispielargumente im Test aller Befehle), `lib.rs` (`generate_handler!`, `use commands::…`): beide hängen an; wer später landet, setzt seine Zeilen hinter die des anderen.
  - `catalog.rs`: die Liste in `the_curated_set_is_the_decided_one` und `tool_count_is_about_thirty` (mit 4.1b 49 Werkzeuge; bringt 4.3a Werkzeuge mit, hebt der Spätere die Grenze und begründet es im Commit).
  - `build/steps.rs`: 4.3a fügt die Stufe „Entität → Member“ an; eine neue `pack_entries` aus Task 5 steht am Ende der Datei, damit der Merge nur anhängt.
  - `test_support.rs`: beide ergänzen Helfer (`vanilla_base_rom`, `ProjectOriginal` hier); `store.rs`, falls 4.3a dort liest oder schreibt (Task 1 ändert `Changeset`, `undo`, `redo`).
  - `g4k.rs` ändert 4.1b nicht; `docs/mcp.md`, `docs/agent-gate.md`, `docs/dev-setup.md`, `docs/ROADMAP.md` (Statuszeilen) ändern beide.
