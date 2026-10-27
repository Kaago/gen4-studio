# Phase 4.3a „Textprojekt im Kernel“ Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ein DSPRE-Ordner mit Basismarke wird kopflos (`g4k text export`) zum Textprojekt: Manifest `g4-project.json` mit `schemaVersion`, eine kanonische JSON je Entität unter `data/<art>/NNNN.json` mit symbolischen IDs (feste englische Tabellen aus PokeAPI, Taufe in der Projektsprache mit Register `data/ids/`), Binäres nur bei Abweichung von der Basis, rotom-Quirks als Projektdatei; `g4k text import` und die neue Build-Stufe „Entität → Member“ machen daraus wieder Member, und auf allen drei Testprojekten ergibt Export → Member löschen → Import → Build die SHA-1 der Basis.

**Architecture:** Neues Kernel-Modul `crates/g4-kernel/src/textproject/` mit eigenen serde-Typen je Entität (`deny_unknown_fields`, `g4-formats` bleibt ohne serde), einem generischen Abbilder über `RawRecord::FIELDS` für alle `fixed_record`-Arten, Symboltabellen und Register, einer Migrationskette auf dem Dateibaum und einer „Basisschicht“ (die extrahierte Basis-ROM im Cache) als Maßstab für Beifiles und Blobs. Die Stufe „Entität → Member“ ist ein `Step` im Build-Graphen aus 4.2 und läuft vor `rotom` und `chatot`, nur wenn das Manifest da ist. Kopflos über `g4k text export|import|check|sources|migrate`; über MCP `text_check` (lesen) und `text_export` (mit Rückfrage). Keine Oberfläche.

**Tech Stack:** Rust (g4-kernel: serde, serde_json, serde_yaml, sha1, xxhash, uxie 0.9, chatot 0.8, rotom v0.7.0, ds-rom), proptest, assert_cmd; Python 3 (nur Standardbibliothek) für den Generator der Symboltabellen; Tauri-Befehle und MCP (Befehlstabelle) in `apps/studio/src-tauri`.

**Spec:** [entscheidungen.md](../../research/2026-10-27-phase-4/entscheidungen.md) Abschnitte „Phasenschnitt und Reihenfolge“, „Textprojekt (Gutachten A, 4.3)“ (die Spalte „Entscheidung“ gilt wörtlich; 4.3a ist der Kernel-Teil, die Zeilen „DSPRE parallel“, „Umstellung eines DSPRE-Ordners“ und „Referenzindex und Scripts“ sowie „ID umbenennen“ gehören zu 4.3b), „Einordnung von 3d und Vorsorge für Phase 5 und 6“, „Nicht in Phase 4“; [gutachten-a-textprojekt.md](../../research/2026-10-27-phase-4/gutachten-a-textprojekt.md) mit [`gutachten-a-proto.rs`](../../research/2026-10-27-phase-4/gutachten-a-proto.rs) (Struktur von Export, Import und Byte-Vergleich je Art, Gitter als Textzeilen) und [`gutachten-a-namen.py`](../../research/2026-10-27-phase-4/gutachten-a-namen.py) (Normalisierung der Namen); [ROADMAP.md](../../ROADMAP.md) „Phase 4“, Punkt 4.3a mit Gate.

Stand 27.10.2026, `main` bei `256105e` (4.0 und 4.1a gemergt). **4.2 (Branch `phase-4-2-build-graph`) wird vor dem Start gemergt; der Plan setzt den gemergten Stand voraus** (Abschnitt „Voraussetzungen aus 4.2“). Arbeit im eigenen Worktree (Branch `phase-4-3a`), **parallel zu 4.1b** in einem zweiten Worktree. Kernel `crates/g4-kernel/src`, Tauri `apps/studio/src-tauri/src`.

## Entscheidungen (wo Roadmap und Gutachten offen lassen)

| Frage | Entscheidung | Grund |
|---|---|---|
| Befehle | `g4k text export`, `import`, `check`, `sources`, `migrate`; `check` ersetzt das `status` der Roadmap (prüft und zeigt den Stand, schreibt nie) | `sources` listet die Dateien des Textprojekts (für `textproject_from_base`, das Gate-Szenario mit Git und die `.gitignore`-Vorlage aus 4.4, die „der Kernel aus dem Layout erzeugt“); `migrate` ist der ausdrückliche Schritt mit Schnappschuss, den die App in 4.3b als Hinweis beim Öffnen anbietet |
| Arten und Ordner | `data/<art>/NNNN.json` mit `art` ∈ `headers`, `matrices`, `maps`, `area_data`, `events`, `encounters`, `species`, `moves`, `items`, `trainers`, `trainer_text_table`, `trainer_text_offsets`; reine Binärarten `map_textures`, `building_textures`, `building_models`, `interior_building_models` haben **keine** JSON, nur bei Abweichung `data/<art>/NNNN.nsbtx` bzw. `.nsbmd` | Arten sind im Format Ordnernamen, kein Enum (Phase 5); eine JSON je unveränderter Textur wäre eine Liste von SHA-1 ohne Nutzen; „Binäres nur bei Abweichung“ |
| Basisschicht | `extract_rom(Basis-ROM)` plus alle Archive entpackt (Asset-Arten und `pack_only` aus 4.2) in `.g4/cache/base-layer/<sha1>/`, einmal je Basis, jederzeit löschbar; sie ist der Maßstab für „weicht ab“ und der Startpunkt von `textproject_from_base`. Ohne Marke oder ohne gefundene Basis-ROM lehnt der Export ab (`base::missing_text`) | „nach einem Klon entsteht der DSPRE-Teil aus der eigenen ROM“: genau dieser Baum ist die Basis, also ist er auch der Vergleich; die Zeit zum Anlegen wird getrennt gemessen, Grenzwerte gelten für den warmen Export |
| Teile (Binäres in einer Entität) | `Part` in drei Formen: `{"hex": "…"}` für ≤ 64 Byte; `{"base_sha1": "<40>"}`, wenn die Bytes dem gleichen Teil der Basisschicht gleichen; sonst Beifile `{"file": "NNNN.<ext>"}` mit gleichem Stamm daneben. Ein Beifile gewinnt, seine SHA-1 steht nicht in der JSON | Entscheidung „Beifiles mit gleichem Stamm, sonst Verweis per SHA-1“; die kleine HG-BGS-Form des Prototyps wird allgemein; Phase 6 ersetzt Beifiles von Hand, ohne eine Prüfsumme nachzuziehen |
| Gesperrtes | jedes Member, das nicht decodiert oder sich nicht byte-gleich zurückschreibt, wird `{"locked": "<Grund>", "raw": Part}` an der Stelle seines Teils (Spezies und Trainer je Member) | Entscheidung „was der Store sperrt, wird Blob mit `"locked"`“, hier allgemein über Encode(Decode(x)) = x statt über die privaten Store-Funktionen |
| Feldnamen | wie im Codec: `RawRecord::FIELDS`-Namen für `fixed_record`-Arten, die Rust-Feldnamen von uxie und `g4-formats` für die übrigen (`dest_header_id`, `model_id`, …); Container mit `Bits`-Feldern erscheinen nur als ihre Bit-Felder, übrige Bits als `<container>_rest` nur, wenn ≠ 0; `Bytes` als Kleinbuchstaben-Hex | eine Quelle der Namen; kein Feld doppelt (Container und Bits), damit eine Bearbeitung nicht still verloren geht |
| Verweise | Felder, die eine Spezies, Attacke, Item, Fähigkeit, einen Typ, Header (`MAP_*`), Ort (`PLACE_*`) oder eine Trainerklasse nennen, schreibt der Export als ID, wenn der **ganze** Feldwert eine Nummer der Tabelle bzw. des Registers ist, sonst als Zahl; Lesen nimmt ID **oder** Zahl; Gitter der Matrizen bleiben Zahlen | „überall gilt ID oder Nummer, der Export schreibt die ID“; Gitter als Textzeilen wären mit IDs unlesbar |
| `id` in der Datei | Spezies, Attacken, Items, Trainer, Header tragen `"id"`; Import prüft sie gegen Tabelle bzw. Register (`textproject.id-mismatch`), ändert nichts | Umbenennen der ID ist 4.3b |
| Items | eine Datei je Item-ID (`data/items/NNNN.json`, NNNN = Item-ID), Member aus der ARM9-Item-Tabelle; teilen sich IDs einen Member, trägt nur die erste die Daten, jede weitere `{"id": …, "same_member_as": "<ID>"}`; Member, auf die keine ID zeigt, laufen wie Nicht-Abgebildetes (Blob bei Abweichung) | Pt hat 468 Item-Namen, aber 446 Member, HG 537/514: geteilte Member gibt es sicher; Task 4 misst die Zahl |
| Header | eine Datei je Header (`id` `MAP_<interner Name>`, `internal_name`, `internal_name_bytes` als Hex nur, wenn die 16 Byte von `mapname.bin` nicht „Name + NUL-Füllung“ sind, dazu alle Felder der Familie); Import schreibt den Ausschnitt in `arm9.bin` bzw. die Dateien von `unpacked/dynamicHeaders` und `mapname.bin`; eine andere Zahl von Headern ohne Dynamic Headers lehnt er ab | Header = Eintrag der Tabelle und interner Name (Gutachten §3.1); die Füllbytes sind ungeprüft |
| Karten | `"source": "binary"` (einziger Wert in 4.3a), `permissions` als 32 Zeilen `kkcc` wie im Prototyp, `buildings`, `buildings_tail` (Hex, nur wenn da), `model`, `bdhc`, `bgs` (HGSS) als `Part` | Vorsorge Phase 6 („Kartenmanifest nennt seine Quelle“) |
| Symboltabellen | `metadata/symbols/{species,forms,moves,items,abilities,types}.yaml`, erzeugt von `scripts/symbols/generate_symbols.py` aus den CSV von PokeAPI an einem festen Commit; eigene Einträge nur für `*_NONE` (0), `SPECIES_EGG` (494), `SPECIES_BAD_EGG` (495) und, falls PokeAPI keinen Gen-4-Index hat, `TYPE_MYSTERY` (9); Rückfall immer `PREFIX_NNNN` (vier Stellen, auch für Nummern über der Tabelle) | Entscheidung „feste englische Tabellen im Repo“; Netz nur beim Erzeugen |
| Taufe | Header `MAP_` + interner Name; Trainer `TRAINER_` + Klasse und Name; Klassen `TRAINER_CLASS_`; Orte `PLACE_`; Normalisierung wie `gutachten-a-namen.py` (Markup weg, `♂/♀` → `_M/_F`, deutsche Umlaute nur bei deutscher Projektsprache, Diakritika über eine eigene Tabelle ohne neue Abhängigkeit, Nicht-Alphanumerisches → `_`, groß); leer oder nur Ziffern → `PREFIX_NNNN`; Gleichstand in Nummernreihenfolge `_2`, `_3`, nächstes freies Suffix; eingefroren: vorhandene Registereinträge bleiben, auch für verschwundene Nummern (`textproject.id-dangling` als Hinweis) | Entscheidung „getauft und eingefroren“ |
| Register | `data/ids/{maps,trainers,trainer_classes,places}.json`, ein JSON-Objekt ID → Nummer, Schlüssel sortiert, ohne eigenes Versionsfeld | Entscheidung „Register `data/ids/<art>.json` (ID → Nummer)“; Version nur im Manifest (S2) |
| Manifest | `{"format": "g4-project", "schemaVersion": 1, "game": {"code", "family", "language"}, "studioVersion", "schemas": {}, "plugins": []}`; `schemas` und `plugins` werden unverändert erhalten, nicht ausgewertet (`textproject.reserved`, Hinweis, wenn nicht leer); keine Basis-SHA-1 | Entscheidung S2 und „Basis-SHA-1 im Manifest: nein“ |
| Migration | `g4k text migrate`: reine Funktionen auf dem Baum (Manifest und alle JSON unter `data/` als `serde_json::Value`), alles im Speicher, dann Schnappschuss der berührten Dateien nach `.g4/backup/textproject-v<von>-<Zeit>/`, dann schreiben; neuere Version: abgelehnt; `export`, `import`, `check` und der Build lehnen eine ältere Version mit dem Satz „…: `g4k text migrate` ausführen“ ab. Probe-Migration v0 → v1 (Manifest-Schlüssel `schema` → `schemaVersion`, Register `{"ids": {"0003": "MAP_T01"}}` → `{"MAP_T01": 3}`) | Entscheidung „Migrationskette mit Schnappschuss, neuere abgelehnt“; v0 ist ein erfundener Vorstand, der nur die Kette beweist |
| rotom-Quirks | `data/rotom-quirks.json`: `{"quirks": {"expanded/scripts/0101.rotom": {"jump_table_end_marker_count": 0}, …}}`, Schlüssel wie in `compile-state.json`, immer geschrieben (auch leer); vor jedem `compile` aus Import oder Build in den Compile-Zustand gelegt: fehlender Eintrag → `FileState::dirty(0, 0, {}).with_quirks(q)`, vorhandener bekommt die Quirks der Projektdatei | Entscheidung „Quirks aus dem Cache in eine Projektdatei“; ein frischer Klon hat keinen Compile-Zustand |
| Scripts beim Export | `g4k text export` übernimmt nicht übernommene Scripts (`scripts::adopt`) und meldet das; über MCP lehnt `text_export` dann ab („erst `adopt_scripts`“) | Adopt schreibt über 1 000 Dateien und ändert den Script-Editor der App; das soll ein Agent nicht nebenbei tun |
| Familien | nur Platin und HGSS; Diamant/Perl lehnt der Export ab („Kein Textprojekt für Diamant/Perl in Phase 4.“) | keine DP-Testprojekte, Header-Form und Archive ungeprüft |
| Was Export besitzt | Export schreibt nur `g4-project.json`, `data/<bekannte Art>/`, `data/ids/`, `data/raw/`, `data/rotom-quirks.json`, fehlende Text-JSON und (bei Adopt) die rotom-Dateien; in eigenen Ordnern löscht er Dateien, die er nicht mehr schreibt (alte Beifiles, alte Blobs); unbekannte Ordner unter `data/` bleiben (`textproject.unknown-art`) | idempotenter Export; Raum für Plugins und 3d-Arten |
| Bereich der Blobs (`data/raw/`) | alle Dateien der Wurzel außer: versteckten Einträgen, `data/`, `expanded/`, `build/`, `plugins/`, den Projektdateien (`g4-*.json`, `rotom.toml`), `*.nds`, `*.srl`, `*.xdelta`, Membern mit Entität, Text- und Script-Membern mit Quelle, `mapname.bin`, `unpacked/dynamicHeaders/` bei angewandtem Patch, den gepackten NARCs aller Archive mit `unpacked/`-Ordner (Build-Ausgabe) und Ordnern unter `unpacked/`, die der Build nicht packt (4.2 `unpacked_unpacked`, nur Hinweis); `arm9/arm9.bin` wird mit ausgeblendetem Header-Ausschnitt verglichen; YAML-Dateien gelten als gleich, wenn ihr Wert gleich ist | Vanilla soll 0 Blobs haben; was der Build nicht liest, kann die ROM nicht ändern |
| Stufe „Entität → Member“ | `Step` `textproject` vor `rotom` und `chatot`, nur mit Manifest; Regeln wörtlich in Task 10; Members sind Eingaben, keine Ausgaben (wie `chatot`), selbst geschriebene stehen in `StepRun::wrote` | eine Änderung von Studio oder DSPRE am Member darf die Stufe nie still überschreiben; der volle Abgleich mit Konfliktliste ist 4.3b |
| Fehler | Schema- und Auflösungsfehler sind `KernelError::Compile(Vec<Problem>)` mit Datei, Zeile und Spalte (aus serde_json) und der Meldung `<code>: <Satz>`; Import und Build prüfen **alles**, bevor sie die erste Datei schreiben | `g4k` und App drucken `Compile` schon; ein Tippfehler darf kein halbes Projekt hinterlassen |
| MCP | `text_check` (`Read`), `text_export` (`Irreversible("Textprojekt exportieren")`, in `NOT_IN_BATCH`, lehnt ab bei ungespeicherten Änderungen, nicht übernommenen Scripts oder fehlender Basis-ROM); `text_import` gibt es über MCP nicht | Import überschreibt Member unter dem offenen Store ohne Undo; der Store liest JSON erst in 4.3b |
| Sichtbares | keine Oberfläche, kein Nachtrag im Design-System | 4.3a ist kopflos (Roadmap); die Flächen „Änderungen von außen“ und „Konflikt“ kommen mit 4.3b |

## Voraussetzungen aus 4.2

Der Plan baut auf dem gemergten Stand von `phase-4-2-build-graph` auf (zuletzt `b2c463b`) und benutzt dort, ohne es zu ändern:

- `crates/g4-kernel/src/build/step.rs`: `trait Step { id, salt, inputs, outputs, run }`, `Dirty { full, changed, added, removed }`, `StepRun { done, total, cached, rows, wrote, full }`, `run_steps(project, &[&dyn Step])`. Ordner als Eingabe stehen für ihre Dateien eine Ebene tief; einen Baum (`data/raw/`) listet die Stufe selbst, wie `RotomStep` es für `expanded/scripts/` tut.
- `build/salt.rs`: `lock(Tool::Uxie)` für das Salz.
- `build.rs`: `prepare(project)` führt heute `rotom` (nur adoptiert) und `chatot` aus; Task 10 setzt die Stufe `textproject` als **erste** davor, nur wenn `textproject::is_text_project(&project.root)`. `run_source_and_pack_steps` (Feature `test-support`) testet die Stufe auf synthetischen Projekten ohne ds-rom.
- `.g4/build/state.json` (`BuildState`, Version 1): **kein neues Feld**; die Stufe hat einen gewöhnlichen Eintrag unter `steps.textproject`. Damit bleibt das Format, auf das 4.1b seine Liste „geändert“ stützt, unverändert.
- `ArchiveMap::pack_only()` (Pfad und Ordnername der Pack-Einträge), `build::unpacked_unpacked`, `build::write_if_changed` (legt Elternordner an), `scripts::watched_paths` (Pfad von `compile-state.json` aus `rotom.toml`), die Textseiten `text::TextSide` mit `KernelError::TextConflict` und die Funktion, mit der 4.2 eine Text-JSON aus einem neueren Member nachzieht.

Heißt etwas nach dem Merge anders, nimmt die Umsetzung den neuen Namen; ändert sich der Vertrag (etwa Ordner rekursiv), gilt der Vertrag und Task 10 passt die Eingabeliste an.

## Global Constraints

- Commits enden mit `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>`; vor jedem cargo-Aufruf `export PATH="$HOME/.cargo/bin:/opt/homebrew/opt/rustup/bin:$PATH"`.
- Originale unter `/Users/POLTEP/Projekte/lab/editor/Roms` nie von Hand öffnen oder ändern; Kopien nur unter `Roms/_testkopien/4-3a-<zweck>` (Ordner `4-3a-<zweck>_DSPRE_contents`, `.nds` `4-3a-<zweck>.nds` daneben). Gated Tests lesen `G4_TEST_PROJECT` (von `scripts/gate.sh` gesetzt), arbeiten auf `vanilla_copy()` und finden die Basis-ROM über `suggest_base(vanilla_project())` (nur gelesen), wie `tests/base.rs`.
- Keine ROM-Daten ins Repo: keine Member, keine Namen aus dem ROM, keine exportierten JSON echter Projekte. Fixtures sind synthetisch. Die Symboltabellen stammen aus PokeAPI (BSD-3-Clause, Namensnennung in `metadata/symbols/`), pret wird nicht kopiert.
- Netz nur in Task 2 zum Erzeugen der Symboltabellen, und nur nach dem OK des Nutzers für den Download (Dateien, Quelle, Größe nennen).
- `g4-formats` bekommt keine serde-Abhängigkeit; die serde-Typen leben in `textproject`. Jeder Entitätstyp trägt `#[serde(deny_unknown_fields)]`, der generische Abbilder prüft Schlüssel selbst gleich streng.
- Kanonische JSON: `serde_json::to_vec_pretty` (zwei Leerzeichen), Schlüssel in Feldreihenfolge des Typs bzw. von `FIELDS`, Zeilenende `\n`, Datei endet mit `\n`; geschrieben nur bei geändertem Inhalt.
- Rebuild-SHA-1 der drei Basen: Platin DE `d40f19ac24d67c721a4ab25e1c013154443d75b4`, HeartGold DE `13b6853aa93121069df1d17446d3ecb36872aecf`, HeartGold US `4fcded0e2713dc03929845de631d0932ea2b5a37`.
- Richtwerte (warm, nur protokolliert, kein Testabbruch): Export ≤ 20 s, Import ≤ 30 s je Projekt.
- Parallel zu 4.1b: `store.rs` bleibt unberührt; in `dispatch.rs` (`COMMANDS`, Arme, Beispielargumente), `lib.rs` (`generate_handler!`), `access.rs` (`COMMAND_KINDS`), `catalog.rs`, `tools.rs` (`NOT_IN_BATCH`), `commands/mod.rs`, `g4k.rs` und `docs/mcp.md` nur **anhängen**.
- Kein UI, kein Design-System-Nachtrag (nichts Sichtbares); `scripts/check-styles.sh` und `check-tokens.sh` sind nicht berührt.
- Vor jedem Commit: `cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, die Tests der berührten Crates (`env -u G4_TEST_PROJECT -u G4_TEST_ROM cargo test -p g4-kernel --all-features`, bei Studio-Änderungen `cargo test -p studio --all-features`).
- Aus einem Worktree: `G4_ROMS_DIR` absolut setzen, `tools/` als Symlink auf den Haupt-Checkout (nie einchecken), `target/agent/gate-*` vor `git worktree remove` sichern.
- Nicht in 4.3a: Store auf JSON, DSPRE-Abgleich mit Konfliktliste, symbolische IDs in `g4-world.json`, `g4-checks.json`, Vorlagen und MCP-Argumenten, „ID umbenennen“, Referenzindex mit Scripts, rotom-Include-Datei, „Als Textprojekt einrichten…“, Git, JSON-Schema-Dateien.

## Review Focus

- Ein Member wurde nach dem Export im Studio oder in DSPRE geändert, danach wird gebaut, später dieselbe Entität in der JSON bearbeitet: der erste Build behält den Member (Hinweis `textproject.member-newer`), der zweite bricht mit `textproject.conflict` ab und schreibt nichts (Task 10 `member_newer_is_kept_and_a_later_json_edit_conflicts`).
- „Neu bauen (ohne Cache)“ löscht den Eintrag der Stufe, und ein Member weicht von seiner JSON und von der Basisschicht ab: der Build stoppt mit `textproject.unsynced` und nennt `g4k text import` und `g4k text export`, statt die JSON gewinnen zu lassen (Task 10 `full_run_never_overwrites_a_member_that_left_the_base`).
- Eine von Hand bearbeitete JSON hat einen vertippten Schlüssel, einen Wert außerhalb des Bereichs oder eine unbekannte ID: Import und Build nennen Datei, Zeile, Spalte und Feld und schreiben keinen einzigen Member, auch keinen der fehlerfreien Dateien (Task 9 `import_validates_everything_before_writing`).
- Zweiter Export, nachdem in der Projektsprache ein Trainer umbenannt und einer angehängt wurde: die alte ID bleibt, der neue bekommt das nächste freie Suffix, keine andere Datei ändert sich (Task 3 `rebaptism_keeps_frozen_ids_and_suffixes_new_ones`, Task 9 `second_export_writes_nothing`).
- Eine Karte, deren Modell erst geändert und dann wieder auf das Original gesetzt wurde: der Export schreibt erst `0003.nsbmd`, danach wieder `base_sha1` und löscht das alte Beifile; ein Import liest kein liegengebliebenes Beifile (Task 6 `map_part_returns_to_base_and_the_side_file_goes`, Task 9 `export_removes_files_it_no_longer_writes`).

---

### Task 1: Gerüst, Manifest, kanonisches JSON und Migrationskette

**Files:**
- Create: `crates/g4-kernel/src/textproject/mod.rs`, `textproject/canon.rs`, `textproject/manifest.rs`, `textproject/migrate.rs`; Fixtures `crates/g4-kernel/tests/fixtures/textproject/v0/{g4-project.json,data/ids/maps.json,data/matrices/0000.json}` und `…/v1/` (erwartet), `crates/g4-kernel/tests/textproject_migrate.rs`
- Modify: `crates/g4-kernel/src/lib.rs` (`pub mod textproject;`)

**Interfaces:**
- Produces:

```rust
// mod.rs
pub const MANIFEST_FILE: &str = "g4-project.json";
pub const DATA_DIR: &str = "data";
pub const SCHEMA_VERSION: u32 = 1;
pub fn is_text_project(root: &Path) -> bool;            // Manifest-Datei vorhanden
// canon.rs
pub(crate) fn to_canonical<T: Serialize>(value: &T) -> Vec<u8>;   // pretty, "\n" am Ende
pub(crate) fn write_if_changed(root: &Path, rel: &Path, bytes: &[u8]) -> Result<bool, KernelError>; // true = geschrieben, atomar
pub(crate) fn hex(bytes: &[u8]) -> String;
pub(crate) fn unhex(text: &str) -> Result<Vec<u8>, String>;
pub(crate) fn json_problem(file: &Path, err: &serde_json::Error, code: &str) -> Problem; // Zeile/Spalte aus serde
pub(crate) fn problem(file: &Path, code: &str, message: String) -> Problem;           // Zeile 1, Spalte 1
// manifest.rs
#[derive(Serialize, Deserialize)] #[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Manifest { pub format: String, pub schema_version: u32, pub game: GameInfo, pub studio_version: String,
    #[serde(default)] pub schemas: BTreeMap<String, serde_json::Value>, #[serde(default)] pub plugins: Vec<serde_json::Value> }
#[derive(Serialize, Deserialize)] #[serde(deny_unknown_fields)]
pub struct GameInfo { pub code: String, pub family: String, pub language: String }  // "pt"/"hgss", Language::locale_key
pub enum SchemaState { Missing, Current, Older(u32), Newer(u32) }
pub fn schema_state(root: &Path) -> Result<SchemaState, KernelError>;  // liest `schemaVersion` oder (v0) `schema`
pub fn read_manifest(root: &Path) -> Result<Manifest, KernelError>;   // nur `Current`, sonst Refused mit Satz
pub fn manifest_for(project: &Project, keep: Option<&Manifest>) -> Manifest; // übernimmt schemas/plugins
// migrate.rs
pub struct Tree { pub files: BTreeMap<PathBuf, serde_json::Value> }   // Manifest + data/**/*.json, relativ, "/"
pub struct Migration { pub from: u32, pub run: fn(&mut Tree) -> Result<(), String> }
pub const MIGRATIONS: &[Migration];                                      // [v0 → v1]
#[derive(Debug, Serialize)] pub struct MigrateReport { pub from: u32, pub to: u32, pub changed: Vec<PathBuf>, pub snapshot: Option<PathBuf> }
pub fn migrate(root: &Path) -> Result<MigrateReport, KernelError>;
```

- Sätze wörtlich: älter `Das Textprojekt hat Schema-Version {v}, diese Fassung kennt {SCHEMA_VERSION}: \`g4k text migrate\` ausführen.`; neuer `Das Textprojekt stammt aus einer neueren Fassung von Gen4 Studio (Schema-Version {v}).`; ohne Manifest `Kein Textprojekt: g4-project.json fehlt.`

- [ ] **Step 0: Voraussetzung.** `crates/g4-kernel/src/build/step.rs` und `build/state.rs` existieren auf der Basis des Worktrees (4.2 gemergt). Sonst anhalten und den Nutzer fragen.
- [ ] **Step 1: Scheiternde Tests** (im Modul und in `tests/textproject_migrate.rs`):
  - `canonical_json_is_pretty_with_a_trailing_newline` (`to_canonical(&json!({"a":1}))` == `b"{\n  \"a\": 1\n}\n"`).
  - `write_if_changed_leaves_an_equal_file_alone` (mtime unverändert, Rückgabe `false`).
  - `manifest_round_trips_and_refuses_unknown_keys` (Tippfehler `schemaVersio` → Fehler, der den Schlüssel nennt).
  - `manifest_keeps_reserved_schemas_and_plugins` (`manifest_for(p, Some(&alt))` übernimmt beide unverändert).
  - `schema_state_tells_missing_current_older_and_newer` (v0-Fixture → `Older(0)`, `schemaVersion: 2` → `Newer(2)`).
  - `migrate_v0_fixture_matches_expected_v1` (Fixture in ein `tempdir` kopieren, `migrate`, jede Datei byte-gleich zu `v1/`; `report.changed` = Manifest und `data/ids/maps.json`; `data/matrices/0000.json` unberührt, gleiche mtime).
  - `migration_snapshots_the_files_before_writing` (unter `.g4/backup/textproject-v0-*/` liegen die alten Bytes beider geänderten Dateien).
  - `migration_failure_leaves_every_file_untouched` (zweite Datei mit kaputtem JSON → `Err`, Manifest unverändert, kein Schnappschuss).
  - `migrate_refuses_a_newer_schema` und `migrate_on_a_current_project_changes_nothing`.
- [ ] **Step 2: Lauf** `cargo test -p g4-kernel --all-features textproject` → FAIL (Modul fehlt).
- [ ] **Step 3: Umsetzen.** `migrate` liest den Baum, wendet `MIGRATIONS` ab der gelesenen Version der Reihe nach an, setzt `schemaVersion = SCHEMA_VERSION`, vergleicht kanonisch und schreibt nur Geändertes; Schnappschuss vor dem ersten Schreiben (Zeitstempel `rfc3339_utc` ohne Doppelpunkte). Die v0-Funktion: Manifest `schema` → `schemaVersion`; jede `data/ids/*.json` mit `ids`-Objekt Nummer → ID wird ID → Zahl.
- [ ] **Step 4: Lauf** → PASS.
- [ ] **Step 5: Commit** `feat(textproject): module skeleton, manifest with schemaVersion, canonical JSON and the migration chain with a v0 probe` (mit Trailer).

### Task 2: Symboltabellen aus PokeAPI

**Files:**
- Create: `scripts/symbols/generate_symbols.py`, `crates/g4-kernel/metadata/symbols/{species,forms,moves,items,abilities,types}.yaml`, `crates/g4-kernel/metadata/symbols/README.md`, `crates/g4-kernel/metadata/symbols/LICENSE-PokeAPI.txt`, `crates/g4-kernel/src/textproject/symbols.rs`

**Interfaces:**
- Produces:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub enum SymbolKind { Species, Move, Item, Ability, Type, Map, Trainer, TrainerClass, Place }
impl SymbolKind { pub fn prefix(self) -> &'static str; }  // SPECIES, MOVE, ITEM, ABILITY, TYPE, MAP, TRAINER, TRAINER_CLASS, PLACE
pub struct SymbolTable { /* number → id, id → number */ }
impl SymbolTable { pub fn id(&self, n: u32) -> Option<&str>; pub fn number(&self, id: &str) -> Option<u32>; pub fn len(&self) -> usize; }
pub fn table(kind: SymbolKind) -> Option<&'static SymbolTable>;   // Some für die fünf festen Arten (Formen in Species)
pub fn fallback_id(kind: SymbolKind, n: u32) -> String;           // "ITEM_0113"
pub fn parse_fallback(kind: SymbolKind, id: &str) -> Option<u32>; // "ITEM_0113" → 113, auch "ITEM_12345"
pub fn normalize(name: &str, german: bool) -> Option<String>;     // None: leer oder nur Ziffern
```

- YAML-Form (je Datei):

```yaml
# Generated by scripts/symbols/generate_symbols.py, do not edit by hand.
# Source: PokeAPI (https://github.com/PokeAPI/pokeapi) commit <sha>, data/v2/csv/<files>; BSD-3-Clause, see LICENSE-PokeAPI.txt.
prefix: SPECIES
ids:
  0: SPECIES_NONE
  1: SPECIES_BULBASAUR
```

- Abbildung im Generator: Spezies `pokemon_species.csv` id 1–493 (Nationaldex = Spielindex), dazu `SPECIES_NONE` 0, `SPECIES_EGG` 494, `SPECIES_BAD_EGG` 495; Formen 496–507 aus `pokemon.csv` in der Reihenfolge von `metadata/species_forms.yaml`: `deoxys-attack`, `deoxys-defense`, `deoxys-speed`, `wormadam-sandy`, `wormadam-trash`, `giratina-origin`, `shaymin-sky`, `rotom-heat`, `rotom-wash`, `rotom-frost`, `rotom-fan`, `rotom-mow`; Attacken `moves.csv` mit `generation_id ≤ 4` (1–467); Items über `item_game_indices.csv` (`generation_id = 4`) und `items.csv`; Fähigkeiten `abilities.csv` mit `generation_id ≤ 4` und `is_main_series = 1` (1–123); Typen über `type_game_indices.csv` (`generation_id = 4`) und `types.csv`. Bezeichner: `-` und Nicht-Alphanumerisches → `_`, groß, mit Präfix. Der Generator bricht ab bei doppelter ID, bei einer ID der Form `PREFIX_<Ziffern>` und bei Lücken in Spezies 0–507, Attacken 0–467, Fähigkeiten 0–123, Typen 0–17.

- [ ] **Step 1: Lizenz und Download.** Den Nutzer um das OK bitten: Quelle `https://raw.githubusercontent.com/PokeAPI/pokeapi/<commit>/…`, Dateien `LICENSE.md` und die acht CSV aus `data/v2/csv/` (zusammen rund 2 MB), Zweck. Danach `LICENSE.md` lesen: bestätigt sie BSD-3-Clause für die Daten, weiter; sonst anhalten und dem Nutzer den Wortlaut nennen (Entscheidung: „die Lizenz der PokeAPI-Daten wird beim Start von 4.3a noch einmal bestätigt“).
- [ ] **Step 2: Generator.** `scripts/symbols/generate_symbols.py` (nur Standardbibliothek, Aufruf `python3 -I scripts/symbols/generate_symbols.py --commit <sha> --out crates/g4-kernel/metadata/symbols [--csv-dir <ordner>]`): lädt die CSV in ein Temp-Verzeichnis (oder liest `--csv-dir`), schreibt die sechs YAML und `LICENSE-PokeAPI.txt` (Lizenztext von PokeAPI mit Copyright-Zeile). Commit-SHA fest im README und in jedem Kopf. Ausführen; die YAML ins Repo.
- [ ] **Step 3: README.** `metadata/symbols/README.md`: Quelle, Commit, Lizenz und Namensnennung, der Erzeugungsbefehl, die eigenen Einträge (0, 494, 495, ggf. `TYPE_MYSTERY`), „Namen sind keine ROM-Daten“, „die Tabellen ändern sich nur mit einer neuen `schemaVersion` samt Migration, denn sie sind Teil des Schemas“.
- [ ] **Step 4: Scheiternde Tests** in `symbols.rs`:
  - `tables_cover_the_gen4_ranges` (Spezies 0–507 lückenlos, Attacken 0–467, Fähigkeiten 0–123, Typen 0–17; Items ≥ 400 Einträge).
  - `ids_are_unique_and_never_look_like_a_fallback` (je Tabelle, `parse_fallback(id)` ist `None`).
  - `well_known_ids` (`SPECIES_PIKACHU` = 25, `SPECIES_ROTOM_MOW` = 507, `MOVE_THUNDER_SHOCK` = 84, `ITEM_POTION` = 17, `TYPE_FIRE` = 10, `ABILITY_LEVITATE` = 26).
  - `fallback_ids_round_trip` (`fallback_id(Item, 113)` = `ITEM_0113`, `parse_fallback(Item, "ITEM_12345")` = 12345, `parse_fallback(Item, "MOVE_0001")` = `None`).
  - `normalize_follows_the_gutachten_rules` (`{TRAINER_NAME:Kai}` → `KAI`; `Nidoran♀` → `NIDORAN_F`; `Jungspund Jörg` mit `german` → `JUNGSPUND_JOERG`; `Pokémon` ohne `german` → `POKEMON`; `Mr. Mime` → `MR_MIME`; `???` → `None`; `0042` → `None`; `Rüpel & Co` → `RUEPEL_AND_CO`).
- [ ] **Step 5: Lauf** → FAIL; **Step 6: Umsetzen** (`include_str!` der YAML, einmal geparst in `OnceLock`; Formen in die Spezies-Tabelle gemischt); **Step 7: Lauf** → PASS.
- [ ] **Step 8: Commit** `feat(textproject): English symbol tables generated from PokeAPI (BSD-3-Clause) with fallback ids and name normalization` (mit Trailer).

### Task 3: Register und Taufe

**Files:**
- Create: `crates/g4-kernel/src/textproject/registry.rs`

**Interfaces:**
- Consumes: Task 1 `to_canonical`, `write_if_changed`; Task 2 `SymbolKind`, `normalize`, `fallback_id`, `parse_fallback`, `table`.
- Produces:

```rust
pub const IDS_DIR: &str = "data/ids";
pub fn registry_file(kind: SymbolKind) -> Option<&'static str>;  // maps.json, trainers.json, trainer_classes.json, places.json
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Registry { pub kind: SymbolKind, pub ids: BTreeMap<String, u32> }
impl Registry {
    pub fn load(root: &Path, kind: SymbolKind) -> Result<Registry, KernelError>; // fehlend: leer; kaputt: Compile mit Datei
    pub fn save(&self, root: &Path) -> Result<bool, KernelError>;              // write_if_changed
    pub fn id_of(&self, n: u32) -> Option<&str>;
    /// Gives every number of `names` (index = number) without an entry an id, in number order.
    pub fn baptize(&mut self, names: &[Option<String>], german: bool);
}
/// Resolves ids for every kind: fixed tables, registries, fallbacks.
pub struct Symbols { /* tables + registries */ }
impl Symbols {
    pub fn load(root: &Path) -> Result<Symbols, KernelError>;
    pub fn registry_mut(&mut self, kind: SymbolKind) -> &mut Registry;
    pub fn id(&self, kind: SymbolKind, n: u32) -> Option<String>;             // Tabelle/Register, sonst None
    pub fn number(&self, kind: SymbolKind, id: &str) -> Option<u32>;          // dazu der Rückfall
    pub fn save(&self, root: &Path) -> Result<u32, KernelError>;             // Zahl geschriebener Register
}
```

- `baptize`: Kandidat = `PREFIX_` + `normalize(name)`, sonst `fallback_id`; vergeben ist, was schon im Register steht (eingefroren) oder in diesem Lauf vergeben wurde; ist der Kandidat vergeben, `_2`, `_3`, … bis frei. Der Rückfall `PREFIX_NNNN` einer Nummer gehört immer dieser Nummer: ein normalisierter Name, der genau so lautet, gilt als leer.

- [ ] **Step 1: Scheiternde Tests:**
  - `baptism_suffixes_collisions_in_number_order` (Namen `[Some("Kai"), Some("Kai"), None, Some("Kai")]`, Trainer → `TRAINER_KAI`, `TRAINER_KAI_2`, `TRAINER_0002`, `TRAINER_KAI_3`).
  - `rebaptism_keeps_frozen_ids_and_suffixes_new_ones` (Review Focus: Register `{TRAINER_KAI: 0, TRAINER_KAI_2: 1}`, Trainer 0 heißt jetzt „Udo“, neuer Trainer 2 „Kai“ → 0 und 1 unverändert, 2 = `TRAINER_KAI_3`).
  - `a_name_that_equals_a_fallback_is_treated_as_empty` (Trainer 5 heißt „0007“ → `TRAINER_0005`; Trainer 7 ohne Namen → `TRAINER_0007`).
  - `registry_file_is_sorted_canonical_json` (Bytes wörtlich), `broken_registry_names_the_file`.
  - `symbols_resolve_ids_registries_and_fallbacks` (`number(Species, "SPECIES_PIKACHU")` = 25, `number(Species, "SPECIES_0600")` = 600, `number(Map, "MAP_T01")` aus dem Register, unbekannt → `None`).
- [ ] **Step 2: Lauf** → FAIL; **Step 3: Umsetzen**; **Step 4: Lauf** → PASS.
- [ ] **Step 5: Commit** `feat(textproject): registries in data/ids and frozen baptism with number-order suffixes` (mit Trailer).

### Task 4: Generischer Abbilder, Verweise, Teile, Sperren und die Datenarten

**Files:**
- Create: `crates/g4-kernel/src/textproject/record.rs`, `textproject/part.rs`, `textproject/schema/mod.rs`, `textproject/schema/data.rs`, `textproject/schema.snap`, `crates/g4-kernel/tests/textproject_vanilla_probe.rs` (gated)

**Interfaces:**
- Consumes: Task 1 `canon`, Task 3 `Symbols`.
- Produces:

```rust
// record.rs: ein fixed_record als JSON-Objekt in FIELDS-Reihenfolge (eigenes Serialize/Deserialize, kein BTreeMap)
pub struct RecordJson(pub Vec<(String, serde_json::Value)>);
pub(crate) fn ref_kind(record: &str, field: &str) -> Option<SymbolKind>;   // die Tabelle der Verweise unten
pub fn record_to_json<R: RawRecord>(r: &R, record: &str, sym: &Symbols) -> RecordJson;
pub fn record_from_json<R: RawRecord>(j: &RecordJson, record: &str, sym: &Symbols) -> Result<R, String>; // Fehler nennt das Feld
// part.rs
#[derive(Serialize, Deserialize)] #[serde(untagged)]   // jede Form ein Objekt mit genau einem Schlüssel; fremde oder mehrere Schlüssel: Fehler
pub enum Part { Hex { hex: String }, Base { base_sha1: String }, File { file: String } }
pub(crate) trait BasePart { fn base_bytes(&self, which: PartKey) -> Result<Option<Vec<u8>>, KernelError>; } // Task 7 liefert die Basisschicht
pub struct PartKey { pub art: &'static str, pub index: u32, pub part: &'static str }  // "maps", 3, "model"
pub(crate) fn part_out(bytes: &[u8], base: Option<&[u8]>, stem: &str, ext: &str, side: &mut Vec<(String, Vec<u8>)>) -> Part;
pub(crate) fn part_in(part: &Part, dir: &Path, key: &PartKey, current: Option<&[u8]>, base: &dyn BasePart) -> Result<Vec<u8>, String>;
#[derive(Serialize, Deserialize)] #[serde(untagged)]
pub enum Member<T> { Value(T), Locked { locked: String, raw: Part } }
// SymRef: ID oder Zahl
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)] #[serde(untagged)]
pub enum SymRef { Id(String), Num(u32) }
// schema/data.rs (alle deny_unknown_fields)
pub struct SpeciesEntity { pub id: String, pub personal: Option<Member<RecordJson>>,
    pub learnset: Option<Member<LearnsetJson>>, pub evolution: Option<Member<RecordJson>> }
pub struct LearnsetJson { pub moves: Vec<LearnMoveJson>, pub terminated: bool, pub tail: String }
pub struct LearnMoveJson { pub level: u8, #[serde(rename = "move")] pub move_id: SymRef }
pub struct MoveEntity { pub id: String, pub data: Member<RecordJson> }
pub struct ItemEntity { pub id: String, pub data: Option<Member<RecordJson>>, pub same_member_as: Option<String> }
pub struct EncounterEntity { pub data: Member<RecordJson> }
pub struct TrainerTextTableEntity { pub entries: Vec<[u16; 2]> }
pub struct TrainerTextOffsetsEntity { pub offsets: Vec<u16> }
/// Every schema type exports and imports through this, `members` in the kind's member order.
pub(crate) trait EntitySchema: Sized + Serialize + DeserializeOwned {
    const ART: &'static str;
    fn export(index: u32, members: &[Option<Vec<u8>>], ctx: &mut ExportCtx<'_>) -> Result<Self, KernelError>;
    fn import(&self, index: u32, ctx: &ImportCtx<'_>) -> Result<Vec<Option<Vec<u8>>>, String>;
    fn sample() -> Self;   // für den Snapshot: jede Option gesetzt, je Liste ein Element
}
pub struct ExportCtx<'a> { pub family: Family, pub symbols: &'a mut Symbols, pub base: &'a dyn BasePart, pub side: Vec<(String, Vec<u8>)> }
pub struct ImportCtx<'a> { pub family: Family, pub symbols: &'a Symbols, pub dir: &'a Path, pub base: &'a dyn BasePart, pub current: &'a [Option<Vec<u8>>] }
```

- Alle Entitäten haben ein Feld `data` bzw. benannte Teile statt `#[serde(flatten)]` (flatten verträgt sich nicht mit `deny_unknown_fields`): `MoveEntity { id, data: Member<RecordJson> }`.
- `RecordJson`-Regeln: Feld ohne `Bits`-Kinder → Wert (`U8`/`U16`/`U32` Zahl, `I8` Zahl mit Vorzeichen, `Bytes` Hex); Container mit `Bits`-Kindern → statt des Containers dessen Bit-Felder in `FIELDS`-Reihenfolge, danach `<container>_rest`, nur wenn die Bits außerhalb aller Masken ≠ 0; Lesen verlangt jeden Pflichtschlüssel genau einmal, lehnt unbekannte ab, prüft Bereiche (Bit-Feld ≤ Maske, `U8` ≤ 255 …) und setzt Container = `rest | Σ (wert << shift)`.
- Verweise (`ref_kind`): `personal.type1/type2` → Type, `personal.item1/item2` → Item, `personal.ability1/ability2` → Ability, `evolution.target0…6` → Species, `move.move_type` → Type, `item.natural_gift_type` → Type, `encounter.*` jedes Feld, das der Codec als Spezies liest (Pt: `grass_NN_species`, `swarm_N`, `day_N`, `night_N`, `radar_N`, die GBA- und Wasser-Spezies; HGSS entsprechend), Learnset `move` → Move. Die vollständige Liste steht im Snapshot (`→SPECIES`).
- Spezies-Datei je Personal-Member (0–507), Teile fehlen nur, wenn das Archiv den Member nicht hat. Items: Abbildung siehe Entscheidungen.

- [ ] **Step 1: Scheiternde Tests** (im Modul, synthetische Bytes):
  - `record_json_writes_bits_instead_of_their_container` (Personal: Schlüssel `color` und `flip`, kein `color_flip`; `ev_yield_rest` fehlt bei 0 und erscheint bei einem gesetzten Bit außerhalb der Masken).
  - `record_json_refuses_unknown_missing_and_out_of_range` (je ein Fall, Meldung nennt das Feld).
  - `refs_are_written_as_ids_and_read_as_id_or_number` (`type1` = 10 → `"TYPE_FIRE"`, Lesen von `"TYPE_FIRE"` und `10` gibt dieselben Bytes; Wert 200 ohne Eintrag → Zahl 200; unbekannte ID → Fehler `textproject.unknown-id`).
  - `pt_encounter_species_word_with_upper_bits_stays_a_number`.
  - `part_rules_hex_base_and_side_file` (≤ 64 Byte → `Hex`; gleich der Basis → `Base`; sonst `File("0003.nsbmd")` und ein Eintrag in `side`).
  - `locked_member_round_trips_as_raw` (Personal mit 3 Byte zu viel → `Member::Locked` mit Grund `personal 0007: 47 bytes, expected 44`, Import gibt die Originalbytes).
  - `items_sharing_a_member_write_same_member_as` (Tabelle: IDs 0 und 5 → Member 0; ID 5 hat `same_member_as: "ITEM_NONE"`; Import lehnt `same_member_as` auf eine ID mit anderem Member ab).
  - `species_entity_joins_personal_learnset_evolution` (eine Datei, drei Teile, Import gibt drei Member byte-gleich).
  - `schema_fields_match_the_snapshot`: sammelt je Schema-Typ aus `sample()` die Schlüsselpfade (`species.personal.hp`, `species.learnset.moves[].move →MOVE` …) und für `RecordJson` die Schlüssel aus `FIELDS`; erste Zeile `schemaVersion 1`. Weicht das Ergebnis ab und ist `SCHEMA_VERSION` gleich der Zahl in der Datei: FAIL mit `Schema geändert: schemaVersion erhöhen und eine Migration schreiben (oder G4_UPDATE_SNAPSHOT=1, wenn die Migration schon da ist)`. Mit `G4_UPDATE_SNAPSHOT=1` schreibt der Test die Datei neu.
  - Proptests: `prop_record_bytes_round_trip` je `fixed_record`-Typ (beliebige `LEN` Bytes → `RecordJson` → kanonisches JSON → zurück → gleiche Bytes) für `Personal`, `Evolution`, `Move`, `Item`, `PtEncounters`, `HgssEncounters`; `prop_learnset_round_trip` (gültige Learnsets mit beliebigem `tail` und `terminated`), `prop_trainer_text_round_trip`.
- [ ] **Step 2: Gated Sonde** `tests/textproject_vanilla_probe.rs` `vanilla_shared_item_members_are_counted`: druckt `items <game_code> ids=<n> members=<m> shared=<k> unreferenced=<u>` (für das Gate-Dokument; kein Assert außer `ids ≥ members`).
- [ ] **Step 3: Lauf** `cargo test -p g4-kernel --all-features textproject` → FAIL; **Step 4: Umsetzen**; **Step 5: Lauf** → PASS, Snapshot mit `G4_UPDATE_SNAPSHOT=1` einmal erzeugt und gelesen (jede Zeile ein Name aus dem Codec).
- [ ] **Step 6: Commit** `feat(textproject): generic FIELDS mapper with refs, parts and locked members; species, moves, items, encounters and trainer text schemas` (mit Trailer).

### Task 5: Trainer, Matrizen, Area-Data und Eventdateien

**Files:**
- Create: `crates/g4-kernel/src/textproject/schema/trainer.rs`, `textproject/schema/world.rs`
- Modify: `textproject/schema/mod.rs`, `textproject/schema.snap`

**Interfaces:**
- Consumes: Task 4 `EntitySchema`, `Member`, `SymRef`, `Part`, `ExportCtx`, `ImportCtx`.
- Produces (alle `deny_unknown_fields`, Optionen mit `skip_serializing_if = "Option::is_none"`):

```rust
pub struct TrainerEntity { pub id: String, pub data: TrainerData }
#[serde(untagged)] pub enum TrainerData {
    Value { class: SymRef, has_moves: bool, has_items: bool, flags_rest: u8, unknown: u8, items: [SymRef; 4],
            ai: u32, battle_type: u32, party: Vec<PartyMonJson> },
    Locked { locked: String, props: Part, party: Part } }
pub struct PartyMonJson { pub species: SymRef, pub form: u8, pub level: u16, pub iv_or_difficulty: u8,
    /* weitere Codec-Felder von PartyMon unter ihrem Rust-Namen */ pub item: Option<SymRef>,
    pub moves: Option<[SymRef; 4]>, pub ball_capsule: Option<u16> }
pub struct MatrixEntity { pub name: String, pub width: u8, pub height: u8, pub maps: Vec<String>,
    pub headers: Option<Vec<String>>, pub altitudes: Option<Vec<String>> }  // Zeilen: "0003 ---- 0004", "05 05"
#[serde(untagged)] pub enum AreaDataEntity {
    Pt { buildings_tileset: u16, map_tileset: u16, unknown: u16, light_type: u16 },
    Hgss { buildings_tileset: u16, map_tileset: u16, dynamic_texture_type: u16, area_type: u8, light_type: u8 },
    Locked { locked: String, raw: Part } }   // beim Lesen nach Familie, nicht nach Glück
pub struct EventEntity { pub bg_events: Vec<BgEventJson>, pub object_events: Vec<ObjectEventJson>,
    pub warp_events: Vec<WarpEventJson>, pub coord_events: Vec<CoordEventJson> }  // oder `{locked, raw}`
pub struct WarpEventJson { pub x: u16, pub z: u16, pub dest_header_id: SymRef /* MAP_* */, pub dest_warp_id: u16, pub height: u32 }
// Bg/Object/Coord: die Felder von uxie unter denselben Namen (Prototyp `events`)
```

- Trainer: `species_word` wird `species` (untere 10 Bit, Verweis) und `form` (obere 6 Bit); `class` → TrainerClass, `items` und `item` → Item, `moves` → Move; ein Paar, das `TrainerDoc::check` nicht als `Exact` liefert, wird `Locked` mit beiden Teilen. Area-Data und Events: Lesen wählt die Form nach `ctx.family` (bzw. `locked`-Schlüssel), damit ein Fehler die echte Ursache nennt statt „no variant matched“.

- [ ] **Step 1: Scheiternde Tests:**
  - `trainer_entity_names_class_items_species_and_moves` (synthetischer `TrainerDoc` mit Moves und Items → IDs; Import byte-gleich für Props und Party).
  - `non_canonical_trainer_pair_is_locked_with_both_parts` (Party mit 4 überzähligen Byte → `Locked`, Import gibt beide Dateien zurück).
  - `matrix_grids_are_text_lines` (2×2 mit `0xFFFF` → `"----"`, mit Headern und Höhen; falsche Spaltenzahl in einer Zeile → Fehler mit Zeile).
  - `area_data_reads_the_family_form` (Pt-JSON in einem HGSS-Kontext → Fehler, der `area_type` nennt).
  - `event_warp_names_its_header` (`dest_header_id` 3 mit Register `MAP_T01 → 3` → `"MAP_T01"`).
  - `event_with_nonzero_bg_padding_is_locked` (Sperrfall: 2-Byte-Padding eines BG-Events ≠ 0 → `locked`, Import byte-gleich).
  - Proptests `prop_matrix_round_trip`, `prop_area_data_round_trip` (je Familie), `prop_event_round_trip` (gültige Wertebereiche je Feld, je Familie), `prop_trainer_round_trip` (gültige `TrainerDoc` mit allen vier Party-Formaten).
  - `schema_fields_match_the_snapshot` neu erzeugt (`G4_UPDATE_SNAPSHOT=1`), Diff gelesen.
- [ ] **Step 2: Lauf** → FAIL; **Step 3: Umsetzen**; **Step 4: Lauf** → PASS.
- [ ] **Step 5: Commit** `feat(textproject): trainer, matrix, area data and event schemas with symbolic class, item, species, move and header refs` (mit Trailer).

### Task 6: Karten und Header

**Files:**
- Create: `crates/g4-kernel/src/textproject/schema/map.rs`, `textproject/schema/header.rs`
- Modify: `textproject/schema/mod.rs`, `textproject/schema.snap`

**Interfaces:**
- Consumes: Task 4 `Part`, `part_out`, `part_in`, `BasePart`; `Project::read_headers`, `internal_names`, `asset_location(HeaderTable, 0)` (`Location::Slice` / `HeaderFiles`), `dynamic_headers::is_applied`.
- Produces:

```rust
pub struct MapEntity { pub source: String /* "binary" */, pub permissions: Vec<String>, pub buildings: Vec<BuildingJson>,
    pub buildings_tail: Option<String>, pub model: Part, pub bdhc: Part, pub bgs: Option<Part> }   // oder {locked, raw}
pub struct BuildingJson { pub model_id: u32, pub position: [i32; 3], pub rotation: [i32; 3], pub scale: [i32; 3], pub dummy: Option<String> }
pub struct HeaderPtEntity { pub id: String, pub internal_name: String, pub internal_name_bytes: Option<String>,
    /* alle Felder von uxie MapHeaderPt in dessen Reihenfolge */ pub location_name: SymRef /* PLACE_* */, … }
pub struct HeaderHgssEntity { /* dito mit MapHeaderHGSS */ }
/// The header table as a whole: export per header, import back to the slice or the Dynamic Headers files.
pub(crate) fn export_headers(project: &Project, ctx: &mut ExportCtx<'_>) -> Result<Vec<(u32, Vec<u8>)>, KernelError>; // (index, kanonische JSON)
pub(crate) struct HeaderWrite { pub table: Vec<u8>, pub mapname: Vec<u8> }
pub(crate) fn import_headers(project: &Project, files: &[(u32, Vec<u8>)], symbols: &Symbols) -> Result<HeaderWrite, Vec<Problem>>;
pub(crate) fn write_headers(project: &Project, w: &HeaderWrite) -> Result<Vec<PathBuf>, KernelError>; // geschriebene Pfade
pub(crate) const MAP_SOURCE_BINARY: &str = "binary";
```

- `permissions`: 32 Zeilen, je 32 Token `kkcc` (Hex von `kind`, `collision`), durch ein Leerzeichen getrennt. `model` mit Endung `nsbmd`, `bdhc` `bdhc`, `bgs` `bgs`. `source` ≠ `"binary"` → Fehler `textproject.map-source: Unbekannte Kartenquelle „…“.`. Header: `internal_name` = Bytes bis zum ersten NUL als Latin-1; `internal_name_bytes` (32 Hex) nur, wenn `name + NUL-Füllung` die 16 Byte nicht ergibt. Import ohne Dynamic Headers mit einer anderen Zahl von Header-Dateien als `mapname.bin` Einträge hat: Fehler `textproject.header-count: Die Zahl der Header ändert sich nur mit dem Patch „Dynamische Header“.`.

- [ ] **Step 1: Scheiternde Tests:**
  - `map_entity_keeps_buildings_dummy_tail_and_parts` (synthetische Karte mit `dummy` ≠ 0 und `buildings_tail = [0x0D, 0x0A]`, Modell gleich der Basis → `Base`, BDHC geändert → `File("0000.bdhc")`; Import byte-gleich).
  - `map_part_returns_to_base_and_the_side_file_goes` (Review Focus: zweiter Export nach Rückkehr zum Original liefert `Base` und kein Eintrag in `side`; das Löschen der alten Datei prüft Task 9).
  - `hgss_small_bgs_is_inline_hex` (BGS ≤ 64 Byte → `Hex`).
  - `a_map_that_does_not_decode_is_locked` (Sperrfall).
  - `headers_round_trip_through_the_arm9_slice` (synthetisches Platin-Projekt mit 2 Headern, `T01`/`T02` → `MAP_T01`/`MAP_T02`, Import schreibt Ausschnitt und `mapname.bin` byte-gleich).
  - `headers_round_trip_through_dynamic_header_files` (Projekt mit angewandtem Patch, `test_support`-Helfer aus `tests/dynamic_headers.rs`).
  - `mapname_garbage_after_the_nul_is_kept` (Füllbyte ≠ 0 → `internal_name_bytes`, Import byte-gleich).
  - `header_count_change_without_dynamic_headers_is_refused`.
  - `header_location_is_a_place_ref` (Register `PLACE_X → 5`).
  - Proptests `prop_map_round_trip` (zufällige Permissions, 0–4 Gebäude, kleine Modelle), `prop_header_round_trip` (zufällige `MapHeaderPt`/`MapHeaderHGSS`).
  - Snapshot neu erzeugt und gelesen.
- [ ] **Step 2: Lauf** → FAIL; **Step 3: Umsetzen**; **Step 4: Lauf** → PASS.
- [ ] **Step 5: Commit** `feat(textproject): map manifests with source binary and parts, headers with internal names through arm9 or dynamic headers` (mit Trailer).

### Task 7: Basisschicht und Blobs

**Files:**
- Create: `crates/g4-kernel/src/textproject/base_layer.rs`
- Modify: `crates/g4-kernel/src/project.rs` (`pub(crate) fn unpack_archive(&self, packed: &Path, unpacked_name: &str) -> Result<usize, KernelError>`, das `ensure_unpacked` künftig selbst benutzt), `crates/g4-kernel/tests/textproject_vanilla_probe.rs` (gated)

**Interfaces:**
- Consumes: `build::extract_rom`, `ArchiveMap::kinds`/`pack_only` (4.2), `build::unpacked_unpacked` (4.2), Task 4 `BasePart`, `PartKey`.
- Produces:

```rust
pub const BASE_LAYER_DIR: &str = ".g4/cache/base-layer";
pub struct BaseLayer { pub root: PathBuf, pub sha1: String }
impl BaseLayer {
    pub fn open(project_root: &Path, sha1: &str) -> Option<BaseLayer>;                 // nur fertige Schicht (Marke `.complete`)
    pub fn ensure(project_root: &Path, base: &BaseRom) -> Result<(BaseLayer, bool /* neu angelegt */), KernelError>;
    pub fn file(&self, rel: &Path) -> Result<Option<Vec<u8>>, KernelError>;
    pub fn member(&self, project: &Project, kind: AssetKind, index: u16) -> Result<Option<Vec<u8>>, KernelError>;
}
impl BasePart for BaseLayer { … }   // Teile von Karten (Modell, BDHC, BGS) und gesperrte Member
pub struct RawDiff { pub changed: Vec<PathBuf>, pub not_built: Vec<String> }  // relativ; Ordner, die der Build nicht packt
pub(crate) fn owned_paths(project: &Project) -> Result<BTreeSet<PathBuf>, KernelError>; // Member mit Entität, Quellen-Member, mapname.bin …
pub fn raw_diff(project: &Project, layer: &BaseLayer) -> Result<RawDiff, KernelError>;
pub(crate) fn place_raw(project: &Project, rel: &Path, bytes: &[u8]) -> Result<bool, KernelError>; // entpackt das Archiv des Ordners zuerst
pub const RAW_DIR: &str = "data/raw";
```

- `ensure`: gibt eine fertige Schicht (`open`) ohne Extraktion zurück (`false`); sonst extrahiert es nach `.g4/cache/base-layer/.<sha1>.tmp`, entpackt jedes Archiv (`kinds()` und `pack_only`) nach `unpacked/`, schreibt `.complete`, benennt dann um; eine halbe Schicht wird beim nächsten Lauf gelöscht. `raw_diff`: Bereich wie in den Entscheidungen; Vergleich erst Größe, dann Bytes; `arm9/arm9.bin` mit dem Header-Ausschnitt beider Seiten genullt; `.yaml`/`.yml`: gleich, wenn `serde_yaml::Value` gleich. `place_raw` für `unpacked/<ordner>/NNNN`: fehlt der Ordner, erst `unpack_archive` aus dem gepackten Pfad dieses Ordners.

- [ ] **Step 1: Scheiternde Tests** (synthetisch; eine „Basisschicht“ ist hier ein von Hand angelegter Ordner mit `.complete`, `BaseLayer { root, sha1 }` direkt gebaut):
  - `raw_diff_ignores_owned_sources_build_outputs_and_hidden_files`.
  - `raw_diff_finds_a_changed_file_and_a_new_file` (`files/a/b.bin` geändert, `files/neu.bin` neu).
  - `raw_diff_masks_the_header_slice_of_arm9` (nur Header geändert → kein Blob; ein Byte außerhalb → Blob `arm9/arm9.bin`).
  - `yaml_with_the_same_value_is_no_blob` (andere Einrückung, gleicher Wert).
  - `folders_the_build_does_not_pack_are_named_not_blobbed` (`unpacked/fremd/0000` → `not_built == ["fremd"]`).
  - `place_raw_unpacks_the_archive_first` (Pack-Eintrag-Ordner fehlt, Blob `unpacked/<ordner>/0002` → Ordner hat danach alle Member, `0002` mit dem Blob).
  - `an_incomplete_base_layer_is_not_opened`.
- [ ] **Step 2: Gated Test** in `tests/textproject_vanilla_probe.rs`: `vanilla_base_layer_has_no_raw_blobs`: `vanilla_copy()`, Basis über `suggest_base(vanilla_project())`, `BaseLayer::ensure` (Zeit messen), `raw_diff` → `changed` leer; druckt `base-layer <game_code> ms=<n> files=<n> bytes=<n> not_built=<liste>`. Ist `changed` nicht leer: die Pfade drucken und die Ursache klären, bevor weitergemacht wird (siehe „Offen“).
- [ ] **Step 3: Lauf** → FAIL; **Step 4: Umsetzen**; **Step 5: Lauf** → PASS, gated einmal auf Platin DE und HeartGold DE mit `--nocapture`.
- [ ] **Step 6: Commit** `feat(textproject): base layer from the extracted base ROM, raw blobs only for changed unmodeled files` (mit Trailer).

### Task 8: rotom-Quirks als Projektdatei

**Files:**
- Create: `crates/g4-kernel/src/textproject/quirks.rs`
- Modify: `crates/g4-kernel/src/scripts.rs` (nur falls `watched_paths` aus 4.2 den Pfad nicht liefert: `pub(crate) fn compile_state_path(project: &Project) -> Result<PathBuf, KernelError>`)

**Interfaces:**
- Consumes: rotom `CompileState`, `FileState`, `BinaryQuirk`; `scripts::watched_paths` (4.2).
- Produces:

```rust
pub const QUIRKS_FILE: &str = "data/rotom-quirks.json";
#[derive(Serialize, Deserialize, Default, PartialEq, Debug)] #[serde(deny_unknown_fields)]
pub struct QuirksFile { pub quirks: BTreeMap<String, QuirkJson> }
#[derive(Serialize, Deserialize, PartialEq, Debug)] #[serde(deny_unknown_fields)]
pub struct QuirkJson { #[serde(skip_serializing_if = "Option::is_none")] pub jump_table_end_marker_count: Option<u8>,
                       #[serde(skip_serializing_if = "Option::is_none")] pub levelscript_padding: Option<u8> }
pub fn quirks_from_state(project: &Project) -> Result<QuirksFile, KernelError>;   // nur Einträge ≠ Default
pub fn seed_quirks(project: &Project, file: &QuirksFile) -> Result<u32, KernelError>; // Zahl geänderter Einträge
```

- [ ] **Step 1: Scheiternde Tests** (synthetisches adoptiertes Projekt wie `a_failed_compile_keeps_the_binary_quirks` in `scripts.rs`):
  - `quirks_file_lists_only_unusual_entries` (ein Eintrag mit `jump_table_end_marker_count: 0`, Datei kanonisch, Schlüssel `expanded/scripts/0000.rotom`).
  - `seed_quirks_creates_missing_entries_dirty` (Compile-Zustand gelöscht, `seed_quirks` → Eintrag mit Status `Dirty` und den Quirks; `scripts::compile` danach schreibt den Member mit der Endmarke wie das Original).
  - `seed_quirks_overrides_a_default_entry_and_keeps_the_rest`.
  - `an_empty_quirks_file_changes_nothing`.
- [ ] **Step 2: Lauf** → FAIL; **Step 3: Umsetzen**; **Step 4: Lauf** → PASS.
- [ ] **Step 5: Commit** `feat(textproject): rotom binary quirks move from the compile-state cache into data/rotom-quirks.json` (mit Trailer).

### Task 9: Export, Import, Check, Quellenliste (Orchestrierung) und Formatdokument

**Files:**
- Modify: `crates/g4-kernel/src/textproject/mod.rs`, `apps/studio/src-tauri/src/mcp/docs.rs` (`("text_project", include_str!(…/docs/formats/text_project.md))` alphabetisch zwischen `text_archive` und `trainer`)
- Create: `docs/formats/text_project.md`, `crates/g4-kernel/tests/textproject.rs` (synthetischer Teil)

**Interfaces:**
- Consumes: Tasks 1–8; `start::{mark_state, MarkState}`, `base::BaseRom`, `scripts::{is_adopted, adopt}`, `data::arm9` (Item-Tabelle), `places::location_bank`, `data::names::{bank_index, Bank}`, `text::TextArchiveDoc`, die Text-Synchronisation aus 4.2, `build::prepare` (4.2).
- Produces:

```rust
pub struct ExportOptions { pub adopt_scripts: bool }
#[derive(Debug, Clone, Default, Serialize)]
pub struct ArtStats { pub art: String, pub entities: u32, pub json_files: u32, pub side_files: u32,
                      pub written: u32, pub removed: u32, pub bytes: u64, pub ms: u64 }
#[derive(Debug, Clone, Default, Serialize)]
pub struct ExportReport { pub arts: Vec<ArtStats>, pub raw_blobs: u32, pub raw_bytes: u64, pub not_built: Vec<String>,
    pub texts: u32, pub text_json_written: u32, pub scripts: u32, pub adopted: Option<u32>, pub registries_written: u32,
    pub base_layer_ms: u64, pub base_layer_created: bool, pub files: u32, pub bytes: u64,
    pub written: u32, pub removed: u32, pub problems: Vec<Problem>, pub ms: u64 }
#[derive(Debug, Clone, Default, Serialize)]
pub struct ImportReport { pub arts: Vec<ArtStats>, pub raw_written: u32, pub members_written: u32, pub quirks_seeded: u32,
    pub compiled: Vec<u16>, pub encoded: Vec<u16>, pub problems: Vec<Problem>, pub ms: u64 }
#[derive(Debug, Clone, Serialize)]
pub struct CheckReport { pub schema: SchemaState, pub problems: Vec<Problem>, pub differs: Vec<PathBuf> }
pub fn export(project: &Project, base: &BaseRom, opts: &ExportOptions) -> Result<ExportReport, KernelError>;
pub fn import(project: &Project, base: Option<&BaseRom>) -> Result<ImportReport, KernelError>;
pub fn check(project: &Project) -> Result<CheckReport, KernelError>;              // schreibt nie
pub fn sources(project: &Project) -> Result<Vec<PathBuf>, KernelError>;          // sortiert, "/"
pub(crate) fn plan_import(project: &Project, base: &dyn BasePart, only: Option<&BTreeSet<PathBuf>>) -> Result<ImportPlan, KernelError>;
pub(crate) struct ImportPlan { pub writes: Vec<(PathBuf, Vec<u8>)>, pub headers: Option<HeaderWrite>, pub quirks: QuirksFile, pub problems: Vec<Problem> }
pub(crate) fn apply_import(project: &Project, plan: &ImportPlan) -> Result<Vec<PathBuf>, KernelError>; // geschriebene Pfade
```

- **Export**, in dieser Reihenfolge: Familie DP → `Refused`; `mark_state` muss `Present` sein und `base.sha1()` gleich der Marke; ältere/neuere Schema-Version → `Refused` (Sätze aus Task 1); Text im Konflikt (4.2) → `KernelError::TextConflict`; nicht adoptiert und `adopt_scripts` → `adopt`, sonst `Refused("Die Scripts sind noch nicht übernommen (adopt_scripts).")`; `BaseLayer::ensure`; Taufe (Header aus `internal_names`, Trainer aus Klassen- und Namensbank, Klassen, Orte aus `location_bank`, Sprache des Projekts, `german` = `Language::German`); je Art Entitäten exportieren, Beifiles und JSON mit `write_if_changed`; Text-JSON nachziehen, wo sie fehlt oder der Member nach 4.2 gilt; `data/rotom-quirks.json`; `raw_diff` → `data/raw/<pfad>`; Manifest; in eigenen Ordnern Übriges löschen; Zählung von Dateien und Bytes über `sources()`.
- **Import:** `plan_import` liest und prüft alles (Schema, IDs, Register, Beifiles, Teile gegen die Basis, Zahl der Member lückenlos, Header-Zahl) und sammelt Probleme; ein Fehler → `Err(Compile(problems))` ohne eine Datei zu schreiben. Danach `apply_import`: Blobs (`place_raw`), Member je Art (vorher `ensure_unpacked` der Art), Header zuletzt, `seed_quirks`; dann `build::prepare` (Stufe `textproject` ist dann ohne Arbeit, `rotom` und `chatot` erzeugen Script- und Text-Member). Basis für `Base`-Teile: der aktuelle Member, wenn sein Teil dieselbe SHA-1 hat, sonst `BaseLayer::open`, sonst `BaseLayer::ensure(base)`, sonst Fehler `textproject.base-missing` mit `base::missing_text`. Member jenseits der letzten JSON einer Art: Fehler `textproject.member-without-json` (Löschen ist 4.3b).
- **Check:** wie `plan_import`, schreibt nichts; `differs` = Pfade, die Import schreiben würde; dazu Hinweise `textproject.unknown-art`, `textproject.reserved`, `textproject.id-dangling`.
- **Quellen:** `g4-project.json`, alles unter `data/`, `expanded/textArchives/*.json`, `expanded/scripts/**` (ohne Versteckte), `rotom.toml`, `.rotom/command_database/**`, `g4-world.json`, `g4-checks.json`, `.g4/base.json`, `.g4/preview.png` (je wenn vorhanden).

- [ ] **Step 1: Scheiternde Tests** in `tests/textproject.rs` (synthetisches Platin-Projekt aus `test_support::synthetic_project`, Basismarke über `write_base_mark` bzw. die Helfer aus `start`-Tests, Basisschicht als Kopie des unveränderten synthetischen Projekts mit `.complete` unter `.g4/cache/base-layer/<sha1>/`; die Funktionen nehmen dafür intern `&dyn BasePart`, `export` baut die Schicht nur bei Bedarf):
  - `export_writes_manifest_entities_registries_and_quirks` (Pfade und Zahlen in `ExportReport`; Manifest `schemaVersion: 1`, `game.code: "CPUE"`).
  - `second_export_writes_nothing` (Review Focus: `written == 0`, `removed == 0`, alle mtimes gleich).
  - `export_removes_files_it_no_longer_writes` (Review Focus: altes Beifile `data/maps/0000.nsbmd` und alter Blob `data/raw/files/x.bin` verschwinden; `data/stories/` bleibt, Hinweis `textproject.unknown-art`).
  - `import_after_deleting_members_restores_every_byte`.
  - `import_validates_everything_before_writing` (Review Focus: `data/moves/0001.json` mit `"powr"` und `data/species/0002.json` mit `"type1": "TYPE_FOO"` → `Err(Compile)` mit zwei Problemen samt Zeile; kein Member und keine mtime unter `unpacked/` geändert).
  - `import_refuses_members_without_json`, `import_needs_the_base_only_for_changed_base_parts`.
  - `check_lists_what_import_would_write_and_writes_nothing`.
  - `sources_lists_the_text_project_files`.
  - `export_refuses_dp_without_mark_another_base_and_an_old_schema` (je ein Fall mit dem Satz).
- [ ] **Step 2: Lauf** → FAIL; **Step 3: Umsetzen**; **Step 4: Lauf** → PASS.
- [ ] **Step 5: Doku** `docs/formats/text_project.md` (Kopf `# Textprojekt`, Muster der übrigen Formatdokumente): Ablage (Wurzel = DSPRE-Ordner, Manifest, `data/<art>/`, Texte und Scripts in `expanded/`), Manifest-Felder, die Arten mit Dateiform und je einem kurzen synthetischen Beispiel, `Part` und `locked`, Feldnamen und Bits-Regel, Verweise und ID-oder-Zahl, Symboltabellen (Quelle, Lizenz) und Register mit Taufe, `data/raw/` und Basisschicht, rotom-Quirks, Schema-Version und Migration (Snapshot-Regel: jede Namensänderung erhöht `schemaVersion`), die Codes `textproject.*` mit Bedeutung, die Quellenliste. In `apps/studio/src-tauri/src/mcp/docs.rs` den Eintrag `text_project` anhängen; `cargo test -p studio table_covers_every_format_file` → PASS.
- [ ] **Step 6: Commit** `feat(textproject): export, import, check and the source list over all entities; format document text_project` (mit Trailer).

### Task 10: Stufe „Entität → Member“ im Build-Graphen

**Files:**
- Create: `crates/g4-kernel/src/textproject/step.rs`
- Modify: `crates/g4-kernel/src/build.rs` (`prepare`: Stufe als erste, nur mit Manifest), `docs/formats/build_state.md` (Schritt `textproject` in der Liste der Schritte)

**Interfaces:**
- Consumes: 4.2 `Step`, `Dirty`, `StepRun`, `run_steps`, `lock(Tool::Uxie)`; Task 9 `plan_import`, `apply_import`; Task 7 `BaseLayer::open`; Task 1 `schema_state`.
- Produces:

```rust
pub(crate) const TEXTPROJECT_REV: u32 = 1;
pub(crate) struct TextProjectStep { pub(crate) problems: RefCell<Vec<Problem>> }
impl Step for TextProjectStep {
    fn id(&self) -> String;     // "textproject"
    fn salt(&self) -> String;   // format!("textproject#{TEXTPROJECT_REV}+schema{SCHEMA_VERSION}+{}", lock(Tool::Uxie))
    fn inputs(&self, p: &Project) -> Result<Vec<PathBuf>, KernelError>;
    fn outputs(&self, p: &Project) -> Result<Vec<PathBuf>, KernelError>;  // leer
    fn run(&self, p: &Project, dirty: &Dirty) -> Result<StepRun, KernelError>;
}
```

- Eingaben: `g4-project.json`, `data/ids`, `data/rotom-quirks.json`, je bekannte Art `data/<art>`, alle Dateien unter `data/raw/` (rekursiv, von der Stufe selbst gelistet), je abgebildete Art der Member-Ordner, `arm9/arm9.bin`, `files/fielddata/maptable/mapname.bin`, bei angewandtem Patch `unpacked/dynamicHeaders`. Eine **Einheit** ist eine Entität (JSON mit Beifiles ↔ ihre Member), ein Blob (`data/raw/X` ↔ `X`), die Header-Tabelle (alle `data/headers/*` ↔ Ausschnitt, `mapname.bin`, Dynamic-Header-Dateien) oder die Quirks-Datei (↔ Compile-Zustand).
- Regeln wörtlich, je Einheit, deren erzeugte Bytes vom Member abweichen:

| Lauf | JSON-Seite geändert | Member-Seite geändert | Wirkung |
|---|---|---|---|
| inkrementell | ja | nein | Member schreiben |
| inkrementell | nein | ja | Member bleibt; Hinweis `textproject.member-newer: <member> ist neuer als <json>; g4k text export übernimmt ihn, g4k text import verwirft ihn.` |
| inkrementell | ja | ja | Fehler `textproject.conflict: <json> und <member> sind beide geändert; nichts geschrieben.` |
| voll (kein Eintrag, anderes Salz) | – | – | Member fehlt oder gleicht dem Member der Basisschicht → schreiben; sonst Fehler `textproject.unsynced: <member> weicht von <json> und von der Basis ab; g4k text import (JSON gilt) oder g4k text export (Member gilt).` |

  Gleichen die erzeugten Bytes dem Member, geschieht nichts. Erst alle Einheiten entscheiden, dann schreiben (ein Fehler → `Err(Compile)`, nichts geschrieben, der Eintrag verfällt wie bei jedem scheiternden Schritt). Ältere oder neuere Schema-Version → Fehler mit dem Satz aus Task 1. Die Quirks-Einheit ruft `seed_quirks`. Geschriebene Member und der Compile-Zustand stehen in `StepRun::wrote`. Zähler: `done` = geschriebene Einheiten, `total` = alle Einheiten.

- [ ] **Step 1: Scheiternde Tests** (synthetisch über `build::run_source_and_pack_steps` nach einem `export`):
  - `the_step_runs_only_with_a_manifest` (ohne Manifest keine Zeile `textproject`; mit Manifest ist sie die erste Zeile).
  - `an_edited_json_reaches_its_member_and_the_packed_narc` (Attacke 1 `power` geändert → Member und `moveData`-NARC tragen den Wert; zweiter Lauf: `textproject` übersprungen).
  - `member_newer_is_kept_and_a_later_json_edit_conflicts` (Review Focus).
  - `full_run_never_overwrites_a_member_that_left_the_base` (Review Focus: Eintrag gelöscht, Member weicht ab → `textproject.unsynced`, Member unverändert; derselbe Fall mit einem Member gleich der Basisschicht → geschrieben).
  - `a_raw_blob_edit_reaches_its_file`, `a_quirks_edit_reaches_the_compile_state`.
  - `an_old_schema_stops_the_build_with_the_migrate_sentence`.
- [ ] **Step 2: Lauf** → FAIL; **Step 3: Umsetzen**; **Step 4: Lauf** → PASS; die Tests von 4.2 (`cargo test -p g4-kernel --all-features build`) bleiben grün (ohne Manifest ändert sich nichts).
- [ ] **Step 5: Doku** `docs/formats/build_state.md`: `textproject` in der Liste der Schritte, „nur mit `g4-project.json`, vor `rotom`“, die Regeltabelle in einem Satz mit Verweis auf `text_project.md`.
- [ ] **Step 6: Commit** `feat(build): step textproject turns entity JSON into members before rotom and chatot, never over a newer member` (mit Trailer).

### Task 11: `g4k text …`

**Files:**
- Modify: `crates/g4-kernel/src/bin/g4k.rs` (Unterbefehl `Text(TextCommand)` am Ende von `Command`), `crates/g4-kernel/tests/cli.rs`

**Interfaces:**
- Consumes: Task 9 (`export`, `import`, `check`, `sources`), Task 1 (`migrate`), 4.1a (`base::locate`, `env_base_dir`, `BaseRom::open`, `missing_text`, `mark_state`).
- Produces (Befehlszeile):
  - `g4k text export <project> [--base-rom <nds>] [--json]`: Basis über `--base-rom`, Nachbar-`.nds`, `G4_BASE_ROM_DIR`, sonst Exit 1 mit `missing_text`; ohne Marke Exit 1 `no base mark: run g4k base assign first`; adoptiert, wenn nötig (`adopted scripts=<n>`); Ausgabe je Art `art=<art> entities=<n> json=<n> side=<n> written=<n> removed=<n> ms=<n>`, dann `raw=<n> texts=<n> scripts=<n> base_layer_ms=<n> created=<bool> files=<n> bytes=<n> written=<n> removed=<n> ms=<n>`, oder `ExportReport` als JSON.
  - `g4k text import <project> [--base-rom <nds>] [--json]`: Probleme über `print_problems`, Exit 1 bei Fehlern; Ausgabe `members=<n> raw=<n> quirks=<n> compiled=<n> encoded=<n> ms=<n>` bzw. JSON.
  - `g4k text check <project> [--json]`: `schema=current|older <v>|newer <v>|missing`, je Problem eine Zeile, `differs=<n>` und die ersten 20 Pfade; Exit 1 bei einem Fehler oder einer anderen Schema-Version.
  - `g4k text sources <project>`: ein Pfad je Zeile.
  - `g4k text migrate <project> [--json]`: `migrated <von> -> <nach> changed=<n> snapshot=<pfad>` oder `already current`; neuere Version Exit 1.

- [ ] **Step 1: Scheiternde Tests** (`assert_cmd`, synthetisch; die Basis-ROM ist hier nicht extrahierbar, darum laufen `export` und `import` über eine vorbereitete Basisschicht im Projekt, deren `.complete` die SHA-1 der Marke trägt, und `--base-rom` auf `synthetic_rom()` mit passender SHA-1):
  - `cli_text_export_then_check_is_clean`, `cli_text_export_twice_writes_nothing` (zweite Ausgabe `written=0 removed=0`).
  - `cli_text_export_without_a_mark_fails`, `cli_text_export_with_a_missing_base_names_it` (`G4_BASE_ROM_DIR` leer).
  - `cli_text_import_reports_problems_with_exit_1` (vertippter Schlüssel → stderr mit Datei und Zeile).
  - `cli_text_sources_lists_manifest_and_data`.
  - `cli_text_migrate_v0_fixture` (Fixture aus Task 1 kopiert, danach `check` → `schema=current`).
- [ ] **Step 2: Lauf** `cargo test -p g4-kernel --all-features --test cli cli_text` → FAIL; **Step 3: Umsetzen**; **Step 4: Lauf** → PASS.
- [ ] **Step 5: Commit** `feat(g4k): text export, import, check, sources and migrate` (mit Trailer).

### Task 12: Gated Tests auf den drei Projekten

**Files:**
- Modify: `crates/g4-kernel/tests/textproject.rs` (gated Teil)

**Interfaces:**
- Consumes: alles aus Tasks 1–11; `test_support::{vanilla_project, vanilla_copy, copy_dir}`, `start::{assign_base, suggest_base, inspect_rom}`, `build::{build, extract_rom}`.

- Gemeinsamer Helfer `exported_copy() -> Option<(TempDir, PathBuf, PathBuf /* nds */, ExportReport)>`: `vanilla_copy()`, `suggest_base(vanilla_project())` (fehlt: `eprintln!("skipped: …")`), `assign_base(copy, nds)` (ohne Warnung), `export(…, adopt_scripts: true)`.

- [ ] **Step 1: Tests schreiben:**
  - `textproject_round_trip`: `exported_copy()`; `export.raw_blobs == 0` und Summe `side_files == 0` (Vanilla); Schnappschuss (Pfad → SHA-1) aller Dateien unter `unpacked/`, `arm9/arm9.bin`, `files/fielddata/maptable/mapname.bin`; alle Member mit Entität, alle Text-Member und alle Script-Member mit Quelle löschen, Header-Ausschnitt bzw. Dynamic-Header-Dateien und `mapname.bin` nullen; `import(copy, Some(&base))`; jeder Pfad des Schnappschusses byte-gleich (die ersten zehn Abweichungen im Fehler); `build::build(copy, <tmp>/out.nds)`; SHA-1 gleich `base.sha1()`. Druckt `textproject-stats <game_code> files=<n> bytes=<n> json=<n> side=<n> raw=<n> base_layer_ms=<n> export_ms=<n> import_ms=<n> build_ms=<n>`.
  - `textproject_export_idempotent`: `exported_copy()`; Schnappschuss (Pfad → SHA-1, mtime) von `sources()`; zweiter `export` → `written == 0`, `removed == 0`, Schnappschuss gleich. Druckt `textproject-second-export <game_code> ms=<n>`.
  - `textproject_from_base`: `exported_copy()`; frischer Ordner `<tmp>/fresh` = `extract_rom(nds)`; jede Datei aus `sources(copy)` dorthin kopieren; `import(fresh, Some(&base))`; `build` → SHA-1 gleich `base.sha1()`. Druckt `textproject-from-base <game_code> extract_ms=<n> import_ms=<n> build_ms=<n> equal`.
- [ ] **Step 2: Lauf** je Projekt einzeln: `G4_TEST_PROJECT="<Roms>/<Projekt>_DSPRE_contents" cargo test -p g4-kernel --all-features --test textproject -- --nocapture --test-threads 1` auf Platin DE, HeartGold DE, HeartGold US → PASS. Abweichungen sind Befunde der Tasks 4–10, keine Ausnahmen im Test.
- [ ] **Step 3: Commit** `test(textproject): round trip, idempotent export and build from the base ROM on the vanilla projects` (mit Trailer).

### Task 13: MCP `text_check` und `text_export`

**Files:**
- Create: `apps/studio/src-tauri/src/commands/textproject.rs` (+ Tests)
- Modify: `src/commands/mod.rs`, `src/lib.rs` (zwei Handler am Ende von `generate_handler!`), `src/commands/dispatch.rs` (`COMMANDS` anhängen, Arme, Beispielargumente im Test aller Befehle), `src/mcp/access.rs` (`COMMAND_KINDS` anhängen, `irreversible_ones_are_the_decided_groups`), `src/mcp/catalog.rs` (zwei Werkzeuge + Tests), `src/mcp/tools.rs` (`NOT_IN_BATCH`), `docs/mcp.md`

**Interfaces:**
- Consumes: Task 9 (`check`, `export`, Reports); 4.1a `AppState::base_roms.locate`, `BaseRom::open`, `NO_MARK`; `dirty_impl` (ungespeicherte Änderungen).
- Produces:

```rust
pub fn text_check_impl(state: &AppState) -> Result<CheckReport, CmdError>;
pub fn text_export_impl(state: &AppState) -> Result<ExportReport, CmdError>;
// CmdError-Codes: "unsaved" („Erst speichern: {n} ungespeicherte Änderungen.“), "base_missing" (Grund aus base_status),
// "not_adopted" (Satz aus Task 9), sonst die Kernel-Meldung
```

- Zugriff: `("text_check", Kind::Read)`, `("text_export", Kind::Irreversible("Textprojekt exportieren"))`; `text_export` in `NOT_IN_BATCH`. Tauri-Befehle `async` mit `spawn_blocking`, keine Argumente. Werkzeuge (englisch): `text_check` „Check text project“ („Checks the text project (g4-project.json, data/) without writing: schema version, problems with file and line, and the members an import would write.“), `text_export` „Export text project“ („Writes the project as a text project: g4-project.json and one JSON per entity under data/ with symbolic ids, binary parts only where they differ from the base ROM; needs a saved project, adopted scripts and the base ROM; this cannot be undone. Headless (studio mcp) it needs --allow-irreversible, without it the call answers needs_confirmation.“).

- [ ] **Step 1: Scheiternde Tests:** `commands/textproject.rs`: `text_check_without_a_manifest_says_missing`, `text_export_refuses_unsaved_changes`, `text_export_refuses_unadopted_scripts`, `text_export_without_a_base_answers_base_missing`; `access.rs`: `text_export` unter den unumkehrbaren mit „Textprojekt exportieren“; `catalog.rs`: `text_check` unter den Lesewerkzeugen, `irreversible_tools_name_the_headless_switch` mit `text_export`; `tools.rs`: `batch_refuses_text_export`.
- [ ] **Step 2: Lauf** `cargo test -p studio --all-features` → FAIL; **Step 3: Umsetzen**; **Step 4: Lauf** → PASS, dazu `cargo test -p studio --test mcp_stdio`.
- [ ] **Step 5: Doku** `docs/mcp.md`: zwei Zeilen in der Werkzeugtabelle, Rückfrage „Textprojekt exportieren“, Satz „`text_import` gibt es über MCP erst mit 4.3b“.
- [ ] **Step 6: Commit** `feat(studio): text_check and text_export as commands and MCP tools, export with confirmation` (mit Trailer).

### Task 14: Gate 4.3a

**Files:**
- Create: `docs/phase-4-3a-gate.md`, `scripts/agent/textproject_diff.py`
- Modify: `docs/ROADMAP.md` (Übersicht Zeile 4, Punkt 4.3a mit Stand), `docs/agent-gate.md` (Gate-Datei in der Liste oben, Abschnitt „Phase 4.3a“), `docs/dev-setup.md` (gated Tests `textproject*`, Basisschicht unter `.g4/cache/base-layer/`)

**Interfaces:**
- Consumes: Messzeilen der gated Tests (`textproject-stats`, `textproject-second-export`, `textproject-from-base`, `base-layer`, `items`) und das Szenario.
- `scripts/agent/textproject_diff.py` (nur Standardbibliothek, `python3 -I scripts/agent/textproject_diff.py <export A> <export B>`): vergleicht `data/` beider Projekte ohne `data/ids/`, ersetzt in beiden jede Zeichenkette, die ein Register des eigenen Projekts kennt (`TRAINER_*`, `TRAINER_CLASS_*`, `PLACE_*`, `MAP_*`), durch ihre Zahl, und druckt die abweichenden Pfade sowie die Zahl abweichender Dateien in `expanded/textArchives/`. So lässt sich „der Diff HeartGold DE gegen US betrifft nur Texte und vier Karten“ prüfen, obwohl Trainer, Klassen und Orte in der Projektsprache getauft sind.

- [ ] **Step 1: Gate-Dokument** `docs/phase-4-3a-gate.md` nach dem Muster von `docs/phase-4-1a-gate.md`: Kopf mit Stand, Branch und Basis, Gate-Text der Roadmap als Zitat, Legende „Wer“ (Agent, Agent + Blick, Hand) mit Verweis auf `docs/agent-gate.md`, Verweise auf Roadmap, Entscheidungen und diesen Plan; Hinweis „keine Oberfläche, keine Namen in der Oberfläche“. Vorbereitung: Kopien `_testkopien/4-3a-{pt,hgde,hgus}_DSPRE_contents` mit `4-3a-{pt,hgde,hgus}.nds` (`cp -cR`/`cp -c`), `4-3a-mcp` (Platin DE); Release-`g4k`; aus einem Worktree `G4_ROMS_DIR` absolut und der `tools`-Symlink.
  - **(1) Automatik:** `scripts/gate.sh` grün (die gated Tests `textproject_round_trip`, `textproject_export_idempotent`, `textproject_from_base`, `vanilla_base_layer_has_no_raw_blobs`, `vanilla_shared_item_members_are_counted` laufen dort je Projekt mit); dazu der Einzelaufruf je Projekt mit `--nocapture --test-threads 1` für die Messzeilen.
  - **(2) Szenario kopflos** (je Spiel auf seiner Kopie): `g4k base assign`; `g4k text export` (Zeiten notieren); zweites `g4k text export` → `written=0 removed=0`; `git init -q && git add $(g4k text sources .) && git commit -qm Ausgangsstand`, `g4k text export`, `git status --porcelain` leer; `g4k text check` → `schema=current`, keine Fehler, `differs=0`; eine Spezies-JSON bearbeiten (`data/species/0025.json`, `personal.attack` +1), `g4k build --out build/t.nds`, `g4k check-data build/t.nds --expect species:25:attack=<neu>`; ein Byte des Members `unpacked/moveData/0001` per Datei ändern (wie DSPRE), `g4k build` → Hinweis `textproject.member-newer`, danach `data/moves/0001.json` ändern, `g4k build` → Exit 1 `textproject.conflict`, beide Dateien unverändert; Klon-Weg von Hand: `g4k extract <nds> <tmp>/fresh`, die Dateien aus `g4k text sources` dorthin kopieren, `g4k text import`, `g4k build`, `shasum` gleich der Basis; `python3 -I scripts/agent/textproject_diff.py <hgde> <hgus>` → nur `data/maps/0468.json`, `0473`, `0475`, `0476` und 829 Textarchive; Migration: Fixture `v0` kopieren, `g4k text check` → `schema=older 0` (Exit 1), `g4k text migrate` → Schnappschuss da, `check` → `current`; ein Manifest mit `schemaVersion: 2` → `migrate` und `export` lehnen mit dem Satz ab.
  - **(3) MCP** (`4-3a-mcp`, kopflos `studio mcp --project … --allow-irreversible` und über die Brücke mit Freigabe „Lesen und bearbeiten“): `mcp.sh call text_check '{}'` → `missing`; `text_export` → über die Brücke Dialog „Agent möchte etwas Unumkehrbares tun“ mit „Textprojekt exportieren“, `call.sh agent_answer … allow`, danach `text_check` → `current`; mit einer ungespeicherten Änderung → `unsaved`.
  - **(4) Hand und Blick:** kein Blick auf eine Oberfläche (keine Oberfläche); Hand: der Nutzer liest `data/species/0025.json`, `data/maps/0003.json` und `data/trainers/0001.json` einer Kopie und bestätigt, dass sie lesbar sind (Namen der Felder, IDs); bestätigt die Lizenzlage der PokeAPI-Tabellen.
  - **Ergebnis:** Tabelle Punkt / Wer / Ergebnis / Datum und die Messtabelle:

    | Spiel | Dateien | Größe | Beifiles | Blobs | Basisschicht anlegen (s) | Export warm (s) | Zweiter Export (s) | Import (s) | Build danach (s) | Round-Trip | from_base SHA-1 gleich | geteilte Item-Member |
    |---|---|---|---|---|---|---|---|---|---|---|---|---|
    | Platin DE | | | | | | | | | | | | |
    | HeartGold DE | | | | | | | | | | | | |
    | HeartGold US | | | | | | | | | | | | |

- [ ] **Step 2: Gate abarbeiten** (Agent: (1) bis (3)); Werte und Befunde eintragen; Befunde beheben (eigene Commits `fix(4.3a): …`), bis alles grün ist.
- [ ] **Step 3: Roadmap** `docs/ROADMAP.md`: Übersicht Zeile 4 („4.3a Gate vom Agenten bestanden <Datum>“) und am Ende des Punkts 4.3a „(Stand <Datum>: Round-Trip und from_base byte-gleich auf allen drei Projekten, Export <a>–<b> s, Import <c>–<d> s, <n> Dateien; beim Nutzer offen: Lesbarkeit, Lizenz)“. `docs/agent-gate.md` und `docs/dev-setup.md` wie unter Files.
- [ ] **Step 4: Commit** `docs(4.3a): gate of the text project in the kernel, roadmap status` (mit Trailer).

---

## Offen für die Umsetzung

- **Basisschicht gegen DSPRE-Ordner:** ob ein Vanilla-DSPRE-Ordner außerhalb der abgebildeten Member wirklich gleich `extract_rom` plus Entpacken ist (Gutachten A hat nur die Entitäten verglichen). Findet `vanilla_base_layer_has_no_raw_blobs` Abweichungen, entscheidet die Ursache: Metadaten von ds-rom, die DSPRE anders schreibt, werden wie YAML semantisch verglichen; Dateien, die nur DSPRE anlegt und die der Build nicht liest, kommen in die Ausnahmeliste der Entscheidungen; echte Byte-Unterschiede bleiben Blobs, und das Gate nennt die Zahl statt „0 Blobs“.
- **Platz der Basisschicht:** 150 bis 180 MiB je Projekt unter `.g4/cache/`; ob sie später in den App-Cache je SHA-1 wandert (wie die Basis-Hashes von 4.1b) und ob `g4k text export` sie nach dem Lauf löschen soll, ist nicht entschieden.
- **Item-IDs ohne Member und Member ohne ID:** wie viele es in Vanilla gibt, misst Task 4; ob die 22 bis 23 „???“-IDs alle auf Member 0 zeigen, ändert nur die Zahl der `same_member_as`-Dateien.
- **PokeAPI:** Commit, genaue Lizenzzeile und ob `type_game_indices.csv` den Typ „???“ (9) für Generation 4 führt; ohne ihn gilt der eigene Eintrag `TYPE_MYSTERY`. Ob PokeAPI-Ortsindizes `PLACE_*` sprachfrei machen könnten, bleibt ungeprüft (Taufe in der Projektsprache).
- **Text-Synchronisation:** wie die 4.2-Funktion heißt, die eine Text-JSON aus einem geltenden Member nachzieht, und ob der Export bei `TextSide::Member` stattdessen ablehnen soll; der Plan zieht nach.
- **Scripts ohne Quelle und Script-Member, die DSPRE nach dem Adopt geändert hat:** der Export sieht das nicht (rotom-Status entscheidet beim nächsten Compile); ein Abgleich gehört zu 4.3b.
- **Store und Textprojekt in der App:** ein in der App geöffnetes Projekt mit Manifest baut über die Stufe `textproject`; Bearbeitungen im Store treffen weiter die Member und werden von der Stufe als `member-newer` behalten. Ob die App einen Hinweis „Textprojekt veraltet, exportieren“ zeigen soll, entscheidet 4.3b.
- **Zeiten:** die Richtwerte (Export ≤ 20 s, Import ≤ 30 s) gelten warm; das erste Anlegen der Basisschicht und `scripts::adopt` stehen getrennt im Protokoll. Parallelisieren (Gutachten §2.4 „seriell“) nur, wenn ein Richtwert verfehlt wird.
- **Merge mit 4.1b:** Konflikte in `dispatch.rs`, `lib.rs`, `access.rs`, `catalog.rs`, `tools.rs` (angehängte Einträge und Testlisten), `commands/mod.rs`, `docs/mcp.md`, `docs/agent-gate.md`, `docs/ROADMAP.md` und `docs/dev-setup.md`; in `project.rs`, falls 4.1b dort ebenfalls Entpack-Helfer anlegt (`unpack_archive` aus Task 7), und in `base.rs`, falls 4.1b die Basis-Member hasht (4.3a ändert `base.rs` nicht). `store.rs` und `build/state.rs` ändert 4.3a nicht. Wer zuerst landet, bestimmt die Basis; der andere zieht nach und lässt die Tests beider Teilphasen laufen.
- **4.2 nach dem Merge:** heißen `prepare`, `watched_paths`, `pack_only` oder die Textseiten anders oder listet der `Step`-Vertrag Ordner rekursiv, folgt Task 10 dem gemergten Stand; die Regeltabelle bleibt.
