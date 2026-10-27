# Phase 4.3b „Textprojekt als Quelle“ Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** In einem Projekt mit `g4-project.json` liest und schreibt der Store die Entitäts-JSON unter `data/` (die Member unter `unpacked/` sind nur noch Build-Zwischenstand), ein DSPRE-Ordner lässt sich über „Als Textprojekt einrichten…“ (Technik, Hinweis beim Öffnen, Häkchen im Assistenten) mit Fortschritt umstellen und wieder zurücknehmen, DSPRE bleibt parallel nutzbar (beim Öffnen und bei Fensterfokus werden einseitige Änderungen übernommen und gemeldet, beidseitige landen in einer Konfliktliste in Technik mit Diff und Wahl je Entität), und symbolische IDs (ID oder Nummer) gelten in `g4-world.json`, `g4-checks.json`, den Rezepten der Vorlagen, den MCP-Argumenten, in Scripts über eine erzeugte rotom-Include-Datei und beim Befehl „ID umbenennen“.

**Architecture:** Der Kernel bekommt im Modul `textproject` vier neue Teile: `ids` (ein `Resolver` für ID oder Nummer aller Arten), `source` (eine Entität aus ihrer JSON lesen und in sie schreiben, mit Taufe neuer Entitäten), `sync` (Zustand `sync` in `.g4/build/state.json`, Scan erst über `stat`, dann Hash, Einheiten, Regeltabelle, einseitige Übernahme, Konflikte mit Diff und Wahl) und `adopt` (Umstellung, Zurücknehmen), dazu `include` (rotom-Konstanten) und `rename` („ID umbenennen“ als Dateischritt auf dem Undo-Stapel). Der Store behält seine Byte-Form (Member-Bytes) und seine Undo-Logik; neu ist die Location-Art `Location::Entity`: Laden geht über `EntitySource::read`, Speichern schreibt die JSON (Quelle) und danach den Member (Zwischenstand) und zeichnet beide Fingerabdrücke auf, so dass Build-Schritt `textproject` und Abgleich nichts zu tun finden. Die App bekommt Befehle und MCP-Werkzeuge mit Zugriffsart, einen Auflöser für IDs in `Call::prepare`, einen Fokus-Abgleich im Frontend und die neue Technik-Fläche „Textprojekt“.

**Tech Stack:** Rust (g4-kernel, serde_json mit `preserve_order`, xxhash-rust xxh3, rotom `Lexer`/`Parser` für Script-Verweise und Umbenennen), Tauri-Befehle mit `Channel` für den Fortschritt, MCP (Befehlstabelle, Katalog, Zugriff), React 19 mit den Bausteinen aus `apps/studio/src/ui/`, Tauri `getCurrentWindow().onFocusChanged`, vitest.

**Spec:** [entscheidungen.md](../../research/2026-10-27-phase-4/entscheidungen.md): „Phasenschnitt und Reihenfolge“, „Textprojekt (Gutachten A, 4.3)“ (die Spalte „Entscheidung“ gilt wörtlich, besonders „DSPRE parallel“, „Umstellung eines DSPRE-Ordners“, „Referenzindex und Scripts“, „Kollisionen, Umbenennen“, „Schema und Version“), „Build-Graph“ (Zeilen „Laderegel der Texte“ und „Speicher“: eine Zustandsdatei auch für den DSPRE-Abgleich), „Nicht in Phase 4“, „Einordnung von 3d“ (die Referenzsperre kommt später, der Index mit Scripts jetzt); [gutachten-a-textprojekt.md](../../research/2026-10-27-phase-4/gutachten-a-textprojekt.md) §3.2 (Register und Umbenennen), §3.5 (D2, Fingerabdrücke), §4, §5 (Gate-Vorschlag 4.3b); [ROADMAP.md](../../ROADMAP.md) „Phase 4“, Punkt 4.3b mit Gate; der Plan [4.3a](2026-10-27-phase-4-3a-textprojekt-kernel.md) mit seinem Abschnitt „Offen für die Umsetzung“, [phase-4-3a-gate.md](../../phase-4-3a-gate.md) (Hinweise H1 bis H4) und das Format [text_project.md](../../formats/text_project.md).

Stand 28.10.2026: `main` bei `4ea9827` (4.0, 4.1a, 4.2, 4.1b gemergt); **4.3a (`phase-4-3a-textprojekt`) wird vor dem Start von 4.3b auf main gemergt**, der Plan setzt diesen Stand voraus (Abschnitt „Voraussetzungen aus 4.3a“). Arbeit im eigenen Worktree `.claude/worktrees/phase-4-3b`, Branch `phase-4-3b-textprojekt-quelle`, **ohne parallelen Branch** (4.4 folgt danach). Kernel `crates/g4-kernel/src`, Tauri `apps/studio/src-tauri/src`, Frontend `apps/studio/src`.

## Voraussetzungen aus 4.3a

Vor Task 1 auf dem gemergten main prüfen (`grep` genügt); heißt etwas anders, folgt der Plan dem gemergten Namen, die Regeln bleiben:

| Schnittstelle | erwartet | gebraucht in |
|---|---|---|
| `textproject::{MANIFEST_FILE, DATA_DIR, SCHEMA_VERSION, is_text_project, export, import, check, sources}` | wie in `textproject/mod.rs` | Task 2, 6 |
| `manifest::{schema_state, SchemaState, read_manifest}`, `manifest::refusal` (`pub(crate)`) | `Missing`, `Current`, `Older(v)`, `Newer(v)` | Task 3, 6 |
| `schema::{EntitySchema, ExportCtx, ImportCtx}` | `export(index, members, ctx)`, `import(&self, index, ctx) -> Result<Vec<Option<Vec<u8>>>, String>` | Task 2 |
| `schema::data::{export_items, import_items}`, `schema::header::{export_headers, import_headers, write_headers, HeaderWrite}` | wie in 4.3a | Task 2 |
| `registry::{Symbols, Registry, IDS_DIR, registry_file}`, `symbols::{SymbolKind, table, fallback_id, parse_fallback, normalize}` | `Symbols::{load, id, number, registry_mut, save}`, `Registry::baptize` | Task 1, 2, 9 |
| `import::{plan_import, apply_import, LazyBase, ImportPlan, PlannedWrite}` (`pub(crate)`) | `plan_import(project, base, only: Option<&BTreeSet<PathBuf>>)`; `PlannedWrite.unit` = Quelldateien der Einheit | Task 2, 4 |
| `base_layer::{BaseLayer, raw_diff, owned_paths, RAW_DIR}`, `part::{BasePart, Part, PartKey}` | wie in 4.3a | Task 2, 4 |
| `step::{SYNC_FILE, clear_kept, TextProjectStep}` | `.g4/build/textproject.json` mit den als neuer behaltenen Membern | Task 2 (`forget_kept` ergänzen) |
| `export::write_folder`, `ArtFiles` | schreibt nur Geändertes, löscht Fremdes im Art-Ordner | Task 2 (nur für Einzelschreiben nachgebaut, nicht aufgerufen) |
| `apps/studio/src-tauri/src/commands/textproject.rs` | `text_check_impl`, `check_text_export`, `text_export_impl` | Task 10 |

Aus 4.1b und 4.2 (schon auf main): `Tx::replace`, `FileStep`/`FileChange`, `Store::{record_file_step, forget_asset_history, reload_from_disk, accept_disk_as_base}`, `scripts::{decompile_member, reset_script, mark_stale}`, `Session::{after_write, files_written, disk_changed}`, `commands::{found_base, base_missing}`, `BuildState::{load, update, clear_keeping_texts}`, `fingerprint::{stat, capture_hashed, compare, is_racy, now_ns}`, `text::{TextSync, TextSide, resolve_conflict, record_text_sync, TextKeep}`.

## Entscheidungen (wo Roadmap und Gutachten offen lassen)

| Frage | Entscheidung | Grund |
|---|---|---|
| Byte-Form der Store-Assets im Textprojekt | unverändert die Member-Byte-Form (Trainer `trainer_pair_bytes`, Header-Tabelle die 24-Byte-Datensätze, Textarchiv kanonische chatot-JSON); neue Location `Location::Entity { unit, members }`: Laden über `EntitySource::read` (JSON → Member-Bytes), der Vergleich „dirty“ und „stale“ wie heute auf Bytes | Undo, `Tx::replace`, 4.1b-Reset, Referenzindex und Prüfungen bleiben unverändert; die JSON ist Quelle, ihre Form steht in 4.3a fest |
| Was „Speichern“ schreibt | erst die JSON der Einheit (mit Beifiles, nur bei Änderung), dann den Member (Zwischenstand) wie heute, dann die Fingerabdrücke beider in `sync`; die Einheit fällt aus der Liste der als neuer behaltenen Member (`forget_kept`) | „`unpacked/` wird Build-Zwischenstand“; Leser außerhalb des Stores (Szene, Prüfungen, Referenzindex, DSPRE) sehen weiter aktuelle Member, und `textproject` findet beim Build gleiche Bytes und schreibt nichts |
| Mehrteilige Entitäten | Spezies (Personal, Lernset, Entwicklung) und Item-Member (alle IDs der Item-Tabelle auf denselben Member) sind je **eine** Einheit: Speichern eines Teils exportiert die ganze JSON aus dem neuen Teil und den übrigen (ungespeicherte Werte des Stores, sonst die gespeicherten, sonst aus der JSON gelesen); die Header-Tabelle ist eine Einheit über alle `data/headers/*.json`, geschrieben werden nur Header, deren JSON sich ändert | so entsteht dieselbe JSON wie beim vollen Export (Gate „nur `data/species/NNNN.json` ändert sich“) |
| Neue Entitäten | `append_member`, `append_trainer` und `create_header` schreiben im Textprojekt sofort auch die JSON (Taufe nach 4.3a, Register, Include-Datei); nicht rückgängig wie heute | die Member entstehen heute schon sofort und ohne Undo; ohne JSON wäre der neue Member `member-without-json` |
| Binäres, das am Store vorbei geschrieben wird (4.1b-Reset von Texturen und Modellen, deren Undo, Kartenimport) | `source::after_member_write(project, members)` exportiert die betroffenen Einheiten (Beifile nur bei Abweichung von der Basisschicht) und zeichnet sie auf; die App ruft es aus `Session::files_written` | sonst meldete der nächste Abgleich die eigene Änderung als „aus DSPRE übernommen“ |
| Ohne Basisschicht beim Speichern | Teile, deren Bytes die SHA-1 behalten, die die gelesene JSON als `base_sha1` nannte, bleiben `base_sha1`; ein geänderter Teil wird `{"file"}` bzw. `{"hex"}`; die Basisschicht wird beim Speichern nie angelegt | Speichern darf an der Basis-ROM nicht hängen („nichts sperrt“); der nächste volle Export normalisiert einen auf die Basis zurückgesetzten Teil |
| Zustand des Abgleichs | neuer Abschnitt `sync` in `.g4/build/state.json`: `{ scanned_at_ns, files: { <pfad>: Fp mit xxh3 } }` über alle Dateien, die der Abgleich beobachtet; **nicht** der Eintrag des Schritts `textproject` | Entscheidung „eine Zustandsdatei“; ein eigener Abschnitt, weil der Build-Eintrag nur nach Builds wandert und behaltene Member (`member-newer`) dort als „gesehen“ gälten; „Neu bauen (ohne Cache)“ behält `sync` wie `texts` |
| Was der Abgleich beobachtet | Quelle: `data/**` (ohne versteckte Einträge), `expanded/textArchives/*.json`, `expanded/scripts/**`, `g4-project.json`; DSPRE-Seite: alle Dateien im Bereich der Blobs (`raw_diff`-Regeln: Member, `arm9/`, Overlays, `files/`, `unpacked/**`), ohne gepackte NARCs mit `unpacked/`-Ordner; Fokus-Abgleich nur `stat`, Hash bei abweichendem `stat` oder racy | D2; eine DSPRE-Änderung in einem nicht abgebildeten Ordner wird Blob statt still verloren |
| Einheiten und Regeltabelle | Einheit wie im Import-Plan von 4.3a (Entität, Item-Member, Binärmember, Blob, Header-Tabelle, Quirks) plus Textarchiv (JSON ↔ Member) und Scriptdatei (Quelle ↔ Member); für eine Einheit mit geändertem Fingerabdruck: Bytes aus der Quelle gleich dem Member → **abgeglichen** (neu aufzeichnen); sonst nur Member-Seite geändert → **aus DSPRE** (Quelle aus dem Member schreiben), nur Quellseite → **aus Datei** (Member aus der Quelle schreiben), beide → **Konflikt**, Quelle nicht lesbar → **fehlerhaft** | Gutachten A §3.5; „aus Datei“ schreibt den Member sofort, damit Szene und DSPRE nach `git switch` nicht auf altem Stand bleiben (der Build täte es sonst erst später) |
| Texte im Abgleich | über `TextSync` aus 4.2: `MemberNewer` wird **aus DSPRE** (`resolve_conflict(…, TextKeep::Member)`), `JsonNewer` **aus Datei** (Member kodieren), `Conflict` ein Konflikt; die Aufzeichnung bleibt `texts` | die Laderegel steht seit 4.2; im Textprojekt ist die Text-JSON die versionierte Quelle und soll DSPRE-Änderungen sofort tragen |
| Scripts im Abgleich | nur mit übernommenen Scripts: Member geändert, Quelle nicht → **aus DSPRE**: `decompile_member` und Quelle schreiben, Eintrag im Compile-Zustand wie bei `reset_script` (4.1b); dekompiliert rotom ihn nicht → **fehlerhaft** mit Grund; Quelle geändert → nichts (rotom kompiliert beim Build) | 4.3a „Script-Member, die DSPRE nach dem Adopt geändert hat: ein Abgleich gehört zu 4.3b“ |
| Ungespeicherte Store-Werte | eine Einheit mit einem dirty Asset wird nie übernommen: jede Änderung von außen daran wird ein Konflikt mit `unsaved: true` | „nie stilles Überschreiben“ gilt auch für ungespeicherte Arbeit |
| Wahl je Konflikt | „Studio-Fassung behalten“: die JSON gilt, der Member wird aus ihr geschrieben, ungespeicherte Werte bleiben ungespeichert (bei `other = files`: `accept_disk_as_base`, das Speichern überschreibt die Datei); „DSPRE-Fassung übernehmen“ bzw. „Geänderte Datei übernehmen“ (`other = files`): die Seite von außen gilt, ungespeicherte Werte dieser Assets fallen weg (`reload_from_disk`); jede Wahl zeichnet die Einheit auf | eine Wahl, die die Dateien in einen Abgleichsstand bringt; Undo bleibt beim Store |
| Diff | entitäts-, text- und quirks-JSON: Feldpfad, Studio-Wert, DSPRE-Wert (JSON kompakt), höchstens 40 Zeilen, Rest als Zahl; Scripts: die abweichenden Zeilen mit drei Zeilen Umgebung, höchstens 40; Binärmember und Blobs: Größe und xxh3 je Seite | Gutachten A §3.5 „Diff der beiden JSON (der DSPRE-Stand nur in den Speicher exportiert)“; ein Zeilen-Diff ohne neue Abhängigkeit |
| Wann abgeglichen wird | beim Öffnen (nach `open_project`), bei Fensterfokus (verzögert 800 ms, höchstens ein Lauf zugleich, ein Fokus während eines Laufs löst genau einen weiteren aus), vor jedem Build und auf „Jetzt abgleichen“; nicht während eines Builds; ohne `sync` einmal voll mit Fortschritt | D2; vor dem Build, damit ein Konflikt nicht erst als `textproject.conflict` kommt |
| Ort der Konfliktliste | neuer Technik-Eintrag **„Textprojekt“** nach „Projekt“ (Symbol `textproject`, Phosphor `FileCode`): Stand, Einrichten und Zurücknehmen, „Jetzt abgleichen“, „Auf zwei Seiten geändert“ (Konflikte), „Fehlerhafte Dateien“, „Zuletzt übernommen“ | Technik › Änderungen bleibt der Vergleich mit dem Original (4.1b) und später mit Git (4.4); Abgleich und Umstellung sind ein eigenes Thema |
| Build mit offenen Konflikten | die App baut nicht und meldet „Erst die Konflikte in Technik › Textprojekt lösen.“ mit der Taste „Lösen…“; die Text-Rückfrage von 4.2 bleibt für DSPRE-Ordner ohne Manifest | der Schritt `textproject` bräche ohnehin mit `textproject.conflict` ab |
| Umstellung | `textproject::adopt::adopt`: Scripts übernehmen (falls nicht), Export (4.3a) mit Fortschritt, `rotom.toml` bekommt die Include-Wurzel, Include-Datei schreiben, `sync` und `texts` voll aufzeichnen; die App öffnet danach die Sitzung neu (Store im JSON-Modus); verlangt gespeicherten Stand und gefundene Basis-ROM | Entscheidung „ausdrücklicher Schritt mit Fortschritt“; der Export ist schon byte-gleich (4.3a) |
| Rücknahme | **Manifest entfernen statt Schnappschuss**: „Zurücknehmen…“ verschiebt `g4-project.json`, `data/` und `.g4/include/` nach `.g4/backup/textproject-<UTC>/`, schreibt `g4-world.json` und `g4-checks.json` mit Nummern, leert `sync`; nur ohne Konflikte, ohne ungespeicherte Werte und nach einem Abgleich ohne ausstehende Änderung | der Export ändert keinen Member und überschreibt nur Text-JSON, die nach 4.2 ohnehin dem geltenden Member gleichen; danach halten Speichern und Abgleich die Member stets aktuell; also ist der DSPRE-Teil jederzeit vollständig und das Entfernen des Manifests sofort und verlustfrei; ein Schnappschuss vor der Umstellung kostete 150 MB und wäre nach der ersten Bearbeitung veraltet; das Verschieben (statt Löschen) lässt die Rücknahme selbst rückgängig machen |
| Hinweis beim Öffnen | DSPRE-Ordner ohne Manifest (Platin, HGSS): eine Meldung mit „Einrichten…“ und „Nicht mehr fragen“ (merkt sich je Projektpfad in `localStorage`, Schlüssel `g4.textProjectHint.<root>`); ohne Basis-ROM sagt die Meldung, dass sie gebraucht wird, und bietet „Basis-ROM zuordnen…“ | nicht modal (Muster „Fehlende Basis-ROM“ in 4.1a); die Wahl ist eine Bequemlichkeit des Nutzers, keine Projektdatei |
| Häkchen im Assistenten | im Schritt „Vorlage“ unter der Vorlagenwahl, **aus** als Standard; neue Phase `text_project` nach `check`, vor dem Umbenennen des Staging-Ordners | DSPRE-Nutzer entscheiden selbst (Entscheidung); kostet beim Anlegen 10 bis 120 s |
| Ältere und neuere Schema-Version beim Öffnen | älter: Dialog „Textprojekt anheben?“ (`migrate`, Schnappschuss von 4.3a), „Abbrechen“ schließt das Projekt; neuer: Öffnen abgelehnt mit dem Satz von `manifest::refusal`; kaputtes Manifest: abgelehnt mit Datei, Zeile, Spalte; `studio mcp` startet in allen drei Fällen nicht | ein Store auf einem Schema, das er nicht liest, schriebe falsche JSON |
| ID oder Nummer | `ids::Resolver` je Projekt: feste Tabellen (Spezies, Formen, Attacken, Items, Fähigkeiten, Typen) überall; Register (Maps, Trainer, Trainerklassen, Orte) nur im Textprojekt; `MAP_<interner Name>` immer über `mapname.bin` (auch nach einem Umbenennen und ohne Textprojekt); `PREFIX_NNNN` immer; eine Zahl immer; unbekannt → `Unbekannte ID: <id>.` | Entscheidung „überall ID oder Nummer“; Vorlagen müssen auch in DSPRE-Ordnern laufen |
| `g4-world.json` | liest in den Header-Schlüsseln und -Werten (`roomNames`, `roomOwner`) ID oder Nummer, schreibt im Textprojekt IDs, sonst Nummern; `version` bleibt 1; eine unbekannte ID macht die Datei „nicht lesbar“ mit Grund (Regel R15: wird nie ungelesen überschrieben); Matrizen bleiben Zahlen | Entscheidung „behält seine Version“; Matrizen haben keine ID |
| `g4-checks.json` | bekommt `version: 1` (`serde(default)`, immer geschrieben, eine höhere Version wird abgelehnt); `subject.index` von `header` und `trainer` als ID oder Nummer, im Textprojekt als ID geschrieben | Entscheidung |
| Rezepte der Vorlagen | `metadata/templates.yaml`: `start_town: MAP_T01` bzw. `MAP_T20`, `species: SPECIES_TURTWIG` bzw. `SPECIES_CHIKORITA`; der Leser nimmt auch die alten Formen (`T01`, `387`) | Roadmap „Rezepte … mit symbolischen IDs zusätzlich zu Nummern“ |
| IDs in MCP-Argumenten | in `Call::prepare` (gilt für Werkzeuge und `g4_call`) über eine Tabelle `SYMBOLIC_ARGS` (Befehl, JSON-Zeiger, Art oder „Art aus dem Feld `kind`“), dazu die Werte von `data_edit`-Patches, deren Feld nach `record::ref_kind` ein Verweis ist; unbekannte ID → Fehler `unknown_id`; Antworten bleiben Zahlen | Gate „`SPECIES_PIKACHU` wirkt wie `25`“; eine Stelle für alle Wege |
| Script-Verweise im Referenzindex | kuratierte Tabelle `metadata/script_refs.yaml` je Familie: Befehl und Parametername → Ziel (`trainer`, `item`, `species`, `move`); nur Literale (Zahl oder bekannte ID-Konstante) zählen, Variablen nicht; Quelle `AssetId { kind: Script, index: file }`, `field` = Befehlsname, `slot` = Zeile; Auffrischen über `stat` der Quellen | die Parameternamen der Befehlsdatenbank sind freie Texte; „ID umbenennen“ und die spätere Referenzsperre (3d) brauchen denselben Index |
| Include-Datei | `.g4/include/g4_ids.rotom` mit `#define <ID> <Nummer>` für alle Tabellen- und Register-IDs (keine Ersatz-IDs), sortiert nach Art und Nummer, Kopfkommentar „Erzeugt von Gen4 Studio, nicht bearbeiten“; `rotom.toml` bekommt `.g4/include` in `include_roots`; geschrieben bei Umstellung, Taufe, Umbenennen und beim Öffnen (nur bei Änderung); Scripts nutzen sie mit `#include "g4_ids.rotom"` | erzeugt, also lokal und nie Blob (versteckter Ordner); vorhandene Quellen werden nicht umgeschrieben (Nicht in Phase 4) |
| „ID umbenennen“: welche IDs | nur Register-IDs (`MAP_`, `TRAINER_`, `TRAINER_CLASS_`, `PLACE_`); feste Tabellen bleiben fest | 4.3a legt Register nur für diese vier Arten an; ein Register für Abweichungen der festen Tabellen ist eine eigene Nutzerentscheidung |
| „ID umbenennen“: was nachgezogen wird | Register, das Feld `id` der Entität, jeder JSON-Stringwert gleich der alten ID unter `data/**.json`, `g4-world.json`, `g4-checks.json`, jedes Bezeichner-Token gleich der alten ID in `expanded/scripts/*.rotom` (rotom-Lexer, keine Kommentare, keine Strings), die Include-Datei; alles als **ein** `FileStep` auf dem Undo-Stapel (Asset: Trainer `n` bzw. `HEADER_TABLE`) | „Store-Transaktion über alle Verweise“; die IDs stehen nicht in den Member-Bytes, also ist es ein Dateischritt wie in 4.1b, ⌘Z schreibt alle Dateien zurück |
| Regeln einer neuen ID | Präfix der Art und `_`, danach `A–Z`, `0–9`, `_`, höchstens 64 Zeichen; nicht vergeben; keine Ersatz-ID einer anderen Nummer; abgelehnt, bevor etwas geschrieben wird | 4.3a „ein Register darf keine ID doppelt nennen … keine fremde Fallback-ID tragen“ |
| Ort von „ID umbenennen…“ | Menü „Aktionen“ im Kopf von Trainer und Header (Inspector); die Köpfe von Trainer, Header und Pokémon zeigen im Textprojekt die ID als leise Zeile (`[data-entity-id]`) mit „ID kopieren“; Trainerklassen und Orte nur über MCP | die Gate-Beispiele (Trainer, Spezies) und der häufigste Fall; eine eigene Fläche für Klassen und Orte wäre neue Oberfläche ohne Bedarf in Phase 4 |
| MCP-Werkzeuge und Zugriff | `text_project_status` (Read), `adopt_text_project` (Irreversible „Als Textprojekt einrichten“, Rückfrage), `external_changes` (Read, scannt ohne zu schreiben, mit `diff: true` je Konflikt der Diff), `resolve_external_conflict` (Irreversible „Eine Fassung überschreiben“), `rename_id` (Write, ein Undo-Schritt, nicht im Batch); Befehle ohne eigenes Werkzeug: `plan_rename_id`, `conflict_diff`, `entity_ids` (Read), `external_sync`, `revert_text_project`, `migrate_text_project` (App-only) | Vorgabe der Teilphase; der automatische Abgleich gehört der App, Zurücknehmen und Anheben sind bewusste Schritte des Nutzers |
| Kopflos | `g4k text adopt`, `g4k text revert`, `g4k text sync [--keep studio|dspre <einheit>]`, `g4k text rename-id <alt> <neu>` | Gate (2) und Werkzeuge ohne App |
| Parallelisieren des Exports | nicht in 4.3b; die Umstellung zeigt Fortschritt, die Zeiten kommen ins Gate-Protokoll (4.3a H1) | einmaliger Schritt; die Messung unter Last war I/O-gebunden, nicht CPU |

## Global Constraints

- Commits enden mit `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>`; vor jedem cargo-Aufruf `export PATH="$HOME/.cargo/bin:/opt/homebrew/opt/rustup/bin:$PATH"`.
- Originale unter `/Users/POLTEP/Projekte/lab/editor/Roms` nie von Hand öffnen oder ändern; Testprojekte nur als frische Kopien unter `Roms/_testkopien/4-3b-<zweck>` (Ordner `4-3b-<zweck>_DSPRE_contents` und `4-3b-<zweck>.nds` direkt nebeneinander, `cp -cR` bzw. `cp -c`). Gated Tests lesen `G4_TEST_PROJECT` (von `scripts/gate.sh` je Projekt gesetzt), schreiben nur auf `vanilla_copy()` und finden die Basis-ROM über `test_support::vanilla_base_rom()`, nur lesend. Aus dem Worktree `G4_ROMS_DIR` absolut und den `tools`-Symlink setzen (nie einchecken); `target/agent/gate-*` vor `git worktree remove` sichern.
- Keine ROM-Daten ins Repo: keine `.nds`, keine Member, keine exportierte JSON eines Spiels, keine Namenslisten aus dem ROM; Fixtures nur synthetisch (`test_support`).
- Ohne Manifest arbeitet alles wie heute (DSPRE-Layout); jeder neue Weg prüft `Project::source_mode()` zuerst.
- Nie stilles Überschreiben: weder der Abgleich noch Speichern noch Build überschreiben eine Seite, die seit der letzten Aufzeichnung auf beiden Seiten geändert ist; ungespeicherte Store-Werte werden nie verworfen ohne Wahl des Nutzers.
- Fehlende Basis-ROM sperrt weder Speichern noch Abgleich noch Build; nur Umstellung und das Anlegen der Basisschicht brauchen sie und nennen den Grund (`base::missing_text`, `patch::NO_MARK`).
- Gate-Zahlen (warm, Release, protokolliert mit Last): Fokus-Abgleich ohne Änderung ≤ 0,5 s, mit einer DSPRE-Änderung ≤ 1 s; Speichern einer Entität ≤ 0,2 s; nach Umstellung und nach Rücknahme baut ein unverändertes Projekt SHA-1-gleich zur Basis-ROM; Umstellung und „ID umbenennen“ nur protokolliert.
- UI: nur Bausteine und Tokens aus `apps/studio/src/ui/`, keine Hex-Farben, keine Inline-Styles (`bash scripts/check-styles.sh`, `bash scripts/check-tokens.sh`), deutsche Sätze, Bezeichner englisch; höchstens eine primäre Taste je Dialog und Karte.
- In `dispatch.rs` (`COMMANDS`, Arme, Beispielargumente), `lib.rs` (`generate_handler!`), `access.rs` (`COMMAND_KINDS`), `catalog.rs`, `tools.rs` und `crates/g4-kernel/src/lib.rs` nur anhängen.
- Vor jedem Commit: `cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, die Tests der berührten Crates; bei Frontend-Änderungen zusätzlich `cd apps/studio && pnpm test && pnpm build`, `bash scripts/check-styles.sh`, `bash scripts/check-tokens.sh`.

## Review Focus

- Eine Entität hat im Store eine ungespeicherte Bearbeitung, und DSPRE schreibt ihren Member: kein Neuladen, kein Übernehmen, sondern ein Konflikt „Ungespeichert“; „Studio-Fassung behalten“ lässt die Bearbeitung ungespeichert und schreibt den Member aus der JSON, „DSPRE-Fassung übernehmen“ verwirft sie (Task 5 `an_unsaved_asset_with_a_dspre_change_is_a_conflict_not_a_reload`).
- Jemand bearbeitet eine JSON von Hand mit einem Tippfehler (unbekannter Schlüssel, unbekannte ID), während die App offen ist: der Abgleich listet die Datei als fehlerhaft mit Datei, Zeile, Spalte und schreibt nichts; der Store lehnt das Laden mit demselben Ort ab (Task 4 `a_broken_json_is_listed_with_its_position_and_nothing_is_written`, Task 3 `loading_a_broken_entity_json_names_file_line_and_column`).
- `git checkout` oder ein Kopieren ohne `-p` setzt die mtime Tausender Dateien neu, ohne Inhalt zu ändern: kein Konflikt, keine Meldung „übernommen“, nur neue Aufzeichnung (Task 4 `a_touched_tree_reports_nothing_and_rerecords`).
- „ID umbenennen“ auf eine vergebene ID, eine fremde Ersatz-ID oder mit falschen Zeichen wird abgelehnt, bevor eine Datei geschrieben wird; ⌘Z nach einer Änderung einer der berührten Dateien von außen schreibt nichts und sagt warum (Task 9 `rename_refuses_taken_fallback_and_malformed_ids_and_writes_nothing`, `undo_of_a_rename_after_an_outside_edit_writes_nothing`).
- Viele Fokuswechsel kurz hintereinander, oder ein Fokus während eines Builds: höchstens ein Abgleich zugleich, nach dem Lauf genau einer mehr, keiner während des Builds und einer danach; im Kernel laufen zwei Scans eines Projekts nacheinander (Task 11 `focus_during_a_running_sync_runs_once_more_after_it`, `no_sync_while_building_then_once_after`; Task 4 `concurrent_scans_of_one_project_run_one_after_the_other`).

---

### Task 1: Gemeinsame Fixture und `Resolver` (ID oder Nummer)

**Files:**
- Create: `crates/g4-kernel/src/textproject/ids.rs` (+ Tests im Modul)
- Modify: `crates/g4-kernel/src/textproject/mod.rs` (`pub mod ids;`), `crates/g4-kernel/src/test_support.rs` (`TextFixture`), `crates/g4-kernel/tests/textproject.rs` (nutzt `TextFixture` statt seines lokalen `Fx`-Aufbaus, Verhalten unverändert)

**Interfaces:**
- Consumes: 4.3a `Symbols`, `symbols::{table, parse_fallback, fallback_id}`, `Project::mapname_path`, `textproject::{export, ExportOptions}`.
- Produces:

```rust
// textproject/ids.rs
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RefInput { Number(u32), Id(String) }          // JSON: 25 oder "SPECIES_PIKACHU"
pub const UNKNOWN_ID_CODE: &str = "unknown_id";
pub fn unknown_id_text(id: &str) -> String;            // "Unbekannte ID: {id}."
pub struct Resolver { /* Symbols (nur im Textprojekt), interne Namen aus mapname.bin, text_project: bool, Stempel der Register */ }
impl Resolver {
    pub fn for_project(project: &Project) -> Result<Resolver, KernelError>;
    /// Without a project: the fixed tables and the fallbacks only (recipes, tests).
    pub fn fixed() -> Resolver;
    pub fn number(&self, kind: SymbolKind, input: &RefInput) -> Result<u32, String>;
    /// "25", "SPECIES_PIKACHU", "MAP_T01", "T01" (only for maps: an internal name).
    pub fn parse(&self, kind: SymbolKind, text: &str) -> Result<u32, String>;
    pub fn id(&self, kind: SymbolKind, n: u32) -> Option<String>;
    /// Whether project files write ids (a text project of the current schema).
    pub fn writes_ids(&self) -> bool;
    /// The form a project file writes: `Id` in a text project when the number has one, else `Number`.
    pub fn output(&self, kind: SymbolKind, n: u32) -> RefInput;
    /// Whether the registers changed on disk since this was made (one `stat` per register).
    pub fn is_current(&self, project: &Project) -> bool;
}
// test_support.rs (feature test-support)
pub struct TextFixture { /* Synthetic, Pfad der synthetischen ROM, Basismarke */ }
impl TextFixture {
    /// The synthetic project with a base mark of the synthetic ROM, not exported.
    pub fn dspre() -> TextFixture;
    /// `dspre()`, then `export` with `adopt_scripts: true`: a text project of the current schema.
    pub fn exported() -> TextFixture;
    pub fn root(&self) -> &Path;
    pub fn project(&self) -> Project;
    pub fn base(&self) -> BaseRom;
    pub fn write(&self, rel: &str, bytes: &[u8]);
    pub fn read(&self, rel: &str) -> Vec<u8>;
    pub fn edit(&self, rel: &str, from: &str, to: &str);   // ersetzt genau einmal, Pflicht vorhanden
}
```

- Auflösung je Art: Zahl immer (Bereichsprüfung macht der Verbraucher); `PREFIX_NNNN` immer; feste Tabellen für ihre Arten; für `Map` erst das Register (nur Textprojekt), dann `MAP_<interner Name>` und für `parse` auch der bloße interne Name; `Trainer`, `TrainerClass`, `Place` nur über das Register, sonst `Err("Unbekannte ID: {id} (kein Textprojekt).")`.

- [ ] **Step 1: Scheiternde Tests** (`ids.rs`, mit `TextFixture`):
  - `numbers_and_fixed_ids_resolve_everywhere` (`Resolver::fixed()`: `SPECIES_PIKACHU` → 25, `"25"` → 25, `SPECIES_0600` → 600, `MOVE_POUND` → 1).
  - `map_ids_resolve_by_register_and_by_internal_name` (exportierte Fixture: Register-ID und `MAP_T01` und `T01` → 0; nach Ändern des Registereintrags auf `MAP_HOME` gilt `MAP_HOME` und weiter `MAP_T01`).
  - `register_kinds_need_a_text_project` (`TextFixture::dspre()`: `TRAINER_YOUNGSTER_KAI` → Fehler mit „kein Textprojekt“; exportiert → 1).
  - `unknown_ids_name_themselves` (`SPECIES_PIKACHUU` → `Unbekannte ID: SPECIES_PIKACHUU.`).
  - `output_writes_ids_only_in_a_text_project`, `ref_input_reads_number_or_string`, `is_current_sees_a_register_change`.
- [ ] **Step 2: Lauf** `cargo test -p g4-kernel --all-features textproject::ids` → FAIL (Modul fehlt).
- [ ] **Step 3: Umsetzen** wie unter Interfaces; `TextFixture` übernimmt den Aufbau aus `tests/textproject.rs` (Basismarke, synthetische ROM) und der Test nutzt ihn.
- [ ] **Step 4: Lauf** → PASS; dazu `cargo test -p g4-kernel --all-features --test textproject` (unverändert grün).
- [ ] **Step 5: Commit** `feat(textproject): resolver for id or number of every kind, shared text project fixture` (mit Trailer).

### Task 2: `EntitySource`: eine Entität lesen und schreiben

**Files:**
- Create: `crates/g4-kernel/src/textproject/source.rs` (+ Tests im Modul)
- Modify: `crates/g4-kernel/src/textproject/mod.rs` (`pub mod source;`), `crates/g4-kernel/src/textproject/step.rs` (`pub(crate) fn forget_kept(root: &Path, members: &[PathBuf]) -> Result<(), KernelError>`), `crates/g4-kernel/src/textproject/export.rs` (die Funktion, die für eine Art und einen Index die Dateien erzeugt, als `pub(crate) fn entity_files(…)` aus `art_files`/`entities` herausziehen, ohne Verhaltensänderung), `docs/formats/text_project.md` (Abschnitt „Bearbeiten im Studio“)

**Interfaces:**
- Consumes: Task 1; 4.3a `EntitySchema`, `ExportCtx`, `ImportCtx`, `export_items`/`import_items`, `export_headers`/`import_headers`, `Registry::baptize`, `LazyBase`, `BaseLayer::open`, `canon::{to_canonical, write_if_changed}`.
- Produces:

```rust
/// What a store asset is in the text project: its unit and the files of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntityUnit { pub key: String, pub sources: Vec<PathBuf>, pub members: Vec<PathBuf>, pub assets: Vec<AssetId> }
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct SourceWrite { pub sources: Vec<PathBuf>, pub registers: Vec<PathBuf>, pub include: bool }
pub struct EntitySource { /* Symbols mit Stempel, Familie, gelesene base_sha1 je Teil, LazyBase ohne ensure */ }
impl EntitySource {
    pub fn open(project: &Project) -> Result<EntitySource, KernelError>;
    /// The unit of `id`: `data/species/0025.json` for Personal/Learnset/Evolution 25, the item
    /// member's files for an Item, `data/headers/` for HEADER_TABLE, …; `None` for kinds the
    /// text project does not model (text archives keep `Location::TextSource`).
    pub fn unit(&self, project: &Project, id: AssetId) -> Result<Option<EntityUnit>, KernelError>;
    /// The asset's bytes from its JSON (the store's byte form); `None` when the JSON is missing.
    /// A JSON that does not read is `KernelError::Compile` with file, line, column, path.
    pub fn read(&mut self, project: &Project, id: AssetId) -> Result<Option<Vec<u8>>, KernelError>;
    /// Writes the JSON of every unit among `writes` (asset, bytes in the store's byte form);
    /// parts of a unit not in `writes` come from `current` (the store's saved or unsaved bytes)
    /// or the JSON. Writes only files that change; baptizes new entities; never writes members.
    pub fn write(&mut self, project: &Project, writes: &[(AssetId, Vec<u8>)],
                 current: &dyn Fn(AssetId) -> Option<Vec<u8>>) -> Result<SourceWrite, KernelError>;
    /// The JSON of a member appended just now (`append_member`, `append_trainer`, `create_header`).
    pub fn append(&mut self, project: &Project, id: AssetId) -> Result<SourceWrite, KernelError>;
}
/// Exports the units of members written past the store (binary kinds) and records them.
pub fn after_member_write(project: &Project, members: &[PathBuf]) -> Result<SourceWrite, KernelError>;
```

- Schlüssel einer Einheit (`key`, auch in `sync` und den DTOs): der Pfad der Haupt-JSON (`data/species/0025.json`), `data/headers/` für die Header-Tabelle, `data/items/@<member>` für einen Item-Member, `data/<art>/NNNN.<ext>` für Binäres, `data/raw/<pfad>` für Blobs, `data/rotom-quirks.json`, `expanded/textArchives/NNNN.json`, `expanded/scripts/NNNN` (beide Quellformen).
- `write` und `append` schreiben das Register, wenn die Taufe es ändert, und dann die Include-Datei (Task 8 liefert `include::write_include`; bis dahin ist `include` immer `false`). Nach dem Schreiben `forget_kept(root, members der Einheiten)`.

- [ ] **Step 1: Scheiternde Tests** (`source.rs`, `TextFixture::exported()`):
  - `read_gives_the_member_bytes_of_each_art` (je Art Karte 0, Matrix 0, Gebietsdaten 1, Eventdatei 0, Begegnung 0, Personal/Lernset/Entwicklung 1, Attacke 1, Trainer 1, Header-Tabelle: `read` = Bytes des Members bzw. `trainer_pair_bytes` bzw. Tabellen-Ausschnitt).
  - `write_of_one_species_part_rewrites_only_its_json` (Personal 1 mit geänderter Angriffszahl → nur `data/species/0001.json` hat neue Bytes, gleich dem, was ein voller Export schreiben würde; Lernset und Entwicklung darin unverändert).
  - `write_of_an_unchanged_asset_writes_nothing` (mtime bleibt).
  - `write_of_one_header_rewrites_only_that_header_file`.
  - `append_of_a_new_trainer_baptizes_and_writes_its_json_and_register` (Trainer am Ende angehängt, `append` → `data/trainers/NNNN.json` mit getaufter ID, `data/ids/trainers.json` mit dem Eintrag).
  - `a_changed_map_model_without_base_layer_becomes_a_side_file_and_an_unchanged_one_stays_base_sha1` (Basisschicht gelöscht).
  - `write_forgets_the_kept_members_of_its_units` (`.g4/build/textproject.json` mit dem Member vorher, nachher ohne).
  - `after_member_write_puts_a_changed_texture_into_data_and_removes_it_when_equal_to_the_base`.
  - `unit_names_the_files_of_each_kind` (Schlüssel wie oben).
- [ ] **Step 2: Lauf** `cargo test -p g4-kernel --all-features textproject::source` → FAIL.
- [ ] **Step 3: Umsetzen.** `read` nutzt `E::import(index, &ImportCtx { current: Member auf der Platte, base: LazyBase ohne ensure, … })`; `write` nutzt `entity_files` mit einem `BasePart`, das zuerst die Basisschicht (falls da) und sonst die beim Lesen gemerkten `base_sha1` befragt.
- [ ] **Step 4: Lauf** → PASS; `cargo test -p g4-kernel --all-features --test textproject` grün.
- [ ] **Step 5: Doku** `docs/formats/text_project.md`, Abschnitt „Bearbeiten im Studio“: Einheit und Schlüssel, Reihenfolge JSON → Member → Aufzeichnung, neue Entitäten, Teile ohne Basisschicht.
- [ ] **Step 6: Commit** `feat(textproject): read and write one entity from its JSON, new entities baptized at once` (mit Trailer).

### Task 3: Store auf JSON

**Files:**
- Modify: `crates/g4-kernel/src/project.rs` (`SourceMode`, `Project::source_mode`, `Location::Entity`, `asset_location`), `crates/g4-kernel/src/store.rs` (Feld `source: Option<EntitySource>`, `read_disk_as`, `read_location`/`read_base` für `Entity`, `save`, `accept_disk_as_base`, `append_member`, `append_trainer`, `create_header`; Tests im Testmodul), `crates/g4-kernel/src/textproject/sync.rs` (neu, hier nur `pub(crate) fn record_paths(project, paths: &[PathBuf]) -> Result<(), KernelError>`, Rest in Task 4)
- Create: `crates/g4-kernel/tests/textproject_source.rs` (gated)

**Interfaces:**
- Consumes: Task 2 `EntitySource`.
- Produces:

```rust
// project.rs
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum SourceMode { Dspre, TextProject, Older { schema: u32 }, Newer { schema: u32 }, Broken { reason: String } }
impl Project {
    /// From `g4-project.json`, read once at `Project::open` (Broken: the manifest's problem with position).
    pub fn source_mode(&self) -> &SourceMode;
    /// `Ok` for Dspre and TextProject; else `Refused` with the sentence of `manifest::refusal` or the problem.
    pub fn editable(&self) -> Result<(), KernelError>;
}
pub enum Location { /* wie heute */ Entity { unit: EntityUnit, members: Box<Location> } }
// textproject/sync.rs
pub(crate) fn record_paths(project: &Project, paths: &[PathBuf]) -> Result<(), KernelError>;  // Fp mit xxh3 in `sync.files`
```

- `asset_location` gibt im `TextProject` für jede Art mit Einheit `Location::Entity` zurück, `members` = die Location von heute; Textarchive bleiben `TextSource`, Scripts und Binärarten sind keine Store-Assets. `Store::new` öffnet `EntitySource` nur im `TextProject`; in `Older`/`Newer`/`Broken` lehnt jede Bearbeitung mit `project.editable()` ab (Lesen geht).
- `save` im JSON-Modus: Stale-Prüfung wie heute gegen `source.read` (JSON geändert → `Stale`); dann `source.write(alle dirty Einheiten)`, danach je Asset die `members`-Location wie heute schreiben, dann `record_paths(Quellen + Member)`. Ein Fehler beim Schreiben der JSON schreibt keinen Member.

- [ ] **Step 1: Scheiternde Tests** (Testmodul `store.rs`, `TextFixture`):
  - `without_a_manifest_locations_are_as_before`.
  - `in_a_text_project_assets_load_from_their_json` (JSON von Attacke 1 von Hand geändert, Member nicht → `store.move_data(1)` zeigt den JSON-Wert).
  - `save_writes_json_then_member_and_records_both` (Attacke 1 bearbeiten, speichern → JSON und Member neu, `sync.files` hat beide mit xxh3; zweiter `textproject`-Build-Lauf schreibt nichts).
  - `save_detects_a_json_changed_on_disk_as_stale`.
  - `loading_a_broken_entity_json_names_file_line_and_column` (Review Focus: unbekannter Schlüssel → `Compile` mit `data/moves/0001.json:…:…` und `textproject.schema`).
  - `an_appended_matrix_gets_its_json_at_once`, `a_new_trainer_gets_its_json_and_register_at_once`.
  - `older_and_newer_schemas_refuse_edits_with_the_manifest_sentence`.
  - `undo_and_redo_in_a_text_project_write_nothing_until_save`.
- [ ] **Step 2: Gated Test** `store_on_json_edits_only_the_entity_file_and_builds_like_dspre` (`tests/textproject_source.rs`): zwei Kopien (`vanilla_copy()` zweimal); Kopie A `textproject::export` mit der Basis-ROM; in A und B Pikachu (Personal 25) `attack` +1 über den Store, speichern; in A: `textproject::sources` vorher/nachher → genau `data/species/0025.json` anders; `build::build` beider → gleiche SHA-1; Druckt `store <game_code> save_ms=<n>` (Grenze 200 ms nur gedruckt, `G4_TIMING_STRICT` prüft).
- [ ] **Step 3: Lauf** `cargo test -p g4-kernel --all-features store::` → FAIL; **Step 4: Umsetzen**; **Step 5: Lauf** → PASS, gated einmal auf Platin DE: `G4_TEST_PROJECT="<Roms>/Pokemon - Platin-Edition (Germany)_DSPRE_contents" cargo test --release -p g4-kernel --all-features --test textproject_source -- store_on_json --nocapture --test-threads 1`.
- [ ] **Step 6: Commit** `feat(store): in a text project assets load from and save to their entity JSON, members stay the build's intermediate` (mit Trailer).

### Task 4: Abgleich: Zustand, Scan, einseitige Übernahme

**Files:**
- Modify: `crates/g4-kernel/src/build/state.rs` (`SyncRecord`, Feld `sync`, `clear_keeping_texts` behält `sync`), `crates/g4-kernel/src/textproject/sync.rs` (Scan, Einheiten, Übernahme), `crates/g4-kernel/tests/textproject_source.rs` (gated), `docs/formats/build_state.md` (Abschnitt „`sync` (4.3b)“), `docs/formats/text_project.md` (Abschnitt „DSPRE parallel“)

**Interfaces:**
- Consumes: Task 2, 3; 4.2 `fingerprint::{stat, compare, capture_hashed, is_racy, now_ns}`, `TextSync`, `record_text_sync`, `text::resolve_conflict`; 4.3a `plan_import(…, only)`, `apply_import`, `base_layer::{walk, owned_paths}`; 4.1b `scripts::decompile_member`.
- Produces:

```rust
// build/state.rs
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SyncRecord { pub scanned_at_ns: i128, pub files: BTreeMap<String, Fp> }
pub struct BuildState { /* … */ #[serde(default)] pub sync: Option<SyncRecord> }
// textproject/sync.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)] #[serde(rename_all = "snake_case")]
pub enum UnitSide { InSync, Dspre, Files, Conflict, Broken }
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SyncUnit { pub key: String, pub label: String, pub side: UnitSide, pub assets: Vec<AssetId>,
                      pub sources: Vec<PathBuf>, pub members: Vec<PathBuf>, pub problems: Vec<Problem> }
#[derive(Debug, Clone, Default, Serialize)]
pub struct ScanReport { pub units: Vec<SyncUnit>, pub stat_files: u32, pub hashed: u32, pub full: bool, pub ms: u64 }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)] pub struct SyncProgress { pub done: u32, pub total: u32 }
/// Finds every unit whose files changed since `sync` (all units when there is none, `full`);
/// writes nothing. Units in sync whose fingerprints moved are listed as `InSync`.
pub fn scan(project: &Project, progress: &mut dyn FnMut(SyncProgress)) -> Result<ScanReport, KernelError>;
#[derive(Debug, Clone, Default, Serialize)]
pub struct ApplyReport { pub dspre: Vec<SyncUnit>, pub files: Vec<SyncUnit>, pub written: Vec<PathBuf>, pub ms: u64 }
/// Takes over the one-sided units (`Dspre`, `Files`) not `held` and records them and every
/// `InSync` unit; conflicts and broken units stay unrecorded.
pub fn apply_one_sided(project: &Project, scan: &ScanReport, held: &dyn Fn(&SyncUnit) -> bool) -> Result<ApplyReport, KernelError>;
/// Records every file the scan watches as it is now (adoption, import, export).
pub fn record_all(project: &Project) -> Result<u32, KernelError>;
```

- Labels (`label`, deutsch): `Pokémon 0025 · SPECIES_PIKACHU`, `Attacke 0001 · MOVE_POUND`, `Item 0004 · ITEM_POKE_BALL` (Item-Member: die erste ID), `Trainer 0001 · TRAINER_…`, `Karte 0003`, `Matrix 0000`, `Gebietsdaten 0001`, `Eventdatei 0000`, `Begegnungsdatei 0000`, `Header-Tabelle`, `Textarchiv 0012`, `Scriptdatei 0101`, `Kartentextur 0003` (Binärarten wie 4.1b), `Datei <pfad>` (Blob), `rotom-Eigenheiten`.
- Übernahme je Seite: `Dspre` → Entitäten, Items, Header, Binäres, Blobs über `entity_files` bzw. Blob-Kopie nach `data/raw/`; Text → `resolve_conflict(…, Member)`; Script → `decompile_member`, Quelle und Compile-Eintrag schreiben. `Files` → `plan_import(project, base, Some(Quellen der Einheit))` und `apply_import`; Text → Member kodieren; Script → nichts. Danach `record_paths` bzw. `record_text_sync`, `forget_kept`.
- Ein prozessweites Schloss je Projektwurzel um `scan` und `apply_one_sided` (wie `build::state::LOCK`).

- [ ] **Step 1: Scheiternde Tests** (`sync.rs`, `TextFixture::exported()` nach `record_all`):
  - `nothing_changed_scans_empty` (`units` leer, `hashed == 0`).
  - `a_dspre_member_change_is_dspre_and_apply_writes_the_json` (Byte 3 von `unpacked/moveData/0001` +1 → eine Einheit `Dspre` mit Label `Attacke 0001 · MOVE_POUND`; `apply_one_sided` → JSON `power` +1, zweiter Scan leer).
  - `a_json_change_is_files_and_apply_writes_the_member`.
  - `both_changed_is_a_conflict_and_apply_leaves_both`.
  - `a_dspre_text_change_writes_the_text_json`, `a_dspre_script_member_change_is_decompiled_into_its_source` (adoptierte Fixture), `an_undecompilable_script_member_is_broken_with_its_reason`.
  - `a_change_in_an_unmodelled_folder_becomes_a_blob` (neue Datei unter `unpacked/pokeHeight/`, falls die Fixture den Ordner hat, sonst eine geänderte Datei unter `files/`).
  - `a_broken_json_is_listed_with_its_position_and_nothing_is_written` (Review Focus).
  - `a_touched_tree_reports_nothing_and_rerecords` (Review Focus: alle Dateien mit neuer mtime, gleichem Inhalt → nur `InSync`, nach `apply_one_sided` hat `sync` die neuen `stat`, nichts geschrieben).
  - `without_a_record_the_scan_is_full_and_reports_progress` (`full == true`, Fortschritt bis `total`).
  - `concurrent_scans_of_one_project_run_one_after_the_other` (Review Focus: zwei Threads, Reihenfolge über einen Zähler in der Schlossfunktion).
  - `clear_keeping_texts_keeps_sync`.
- [ ] **Step 2: Gated Test** `simulated_dspre_change_is_adopted_one_sided`: Kopie, Export, `record_all`; `scan` → leer, Zeit `stat_ms`; Byte in `unpacked/moveData/0001` ändern → `scan` + `apply_one_sided` → `data/moves/0001.json` geändert, Zeit `one_ms`; `build::build` → `check-data`-Gegenprobe über den gebauten Member. Druckt `sync <game_code> stat_files=<n> stat_ms=<n> one_ms=<n>` (Grenzen 500 / 1000 ms gedruckt, `G4_TIMING_STRICT` prüft).
- [ ] **Step 3: Lauf** → FAIL; **Step 4: Umsetzen**; **Step 5: Lauf** → PASS (gated einmal auf HeartGold DE).
- [ ] **Step 6: Doku** `build_state.md` „`sync` (4.3b)“: Form, wer schreibt (Speichern, Abgleich, Umstellung, Import, Export, Wahl), dass „ohne Cache“ ihn behält; `text_project.md` „DSPRE parallel“: Regeltabelle, Einheiten, Labels, beobachtete Dateien.
- [ ] **Step 7: Commit** `feat(textproject): external changes found by fingerprints, one-sided ones taken over and recorded` (mit Trailer).

### Task 5: Abgleich: Konflikte, Diff, Wahl, ungespeicherte Assets

**Files:**
- Modify: `crates/g4-kernel/src/textproject/sync.rs` (Diff, Wahl, `sync_store`), `crates/g4-kernel/tests/textproject_source.rs` (gated)

**Interfaces:**
- Consumes: Task 4; `Store::{dirty, reload_from_disk, accept_disk_as_base}`.
- Produces:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize)] pub struct DiffLine { pub path: String, pub studio: String, pub dspre: String }
#[derive(Debug, Clone, PartialEq, Eq, Serialize)] pub struct ConflictDiff { pub lines: Vec<DiffLine>, pub more: u32 }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)] #[serde(rename_all = "snake_case")]
pub enum Keep { Studio, Dspre }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)] #[serde(rename_all = "snake_case")]
pub enum Other { Dspre, Files }
#[derive(Debug, Clone, PartialEq, Eq, Serialize)] pub struct SyncEntry { pub key: String, pub label: String, pub assets: Vec<AssetId> }
#[derive(Debug, Clone, PartialEq, Eq, Serialize)] pub struct ConflictEntry { pub key: String, pub label: String, pub assets: Vec<AssetId>, pub unsaved: bool, pub other: Other }
#[derive(Debug, Clone, PartialEq, Eq, Serialize)] pub struct BrokenEntry { pub key: String, pub label: String, pub problems: Vec<Problem> }
#[derive(Debug, Clone, Default, Serialize)]
pub struct SyncOutcome { pub adopted: Vec<SyncEntry>, pub applied: Vec<SyncEntry>, pub conflicts: Vec<ConflictEntry>,
                         pub broken: Vec<BrokenEntry>, pub reloaded: Vec<AssetId>, pub full: bool, pub ms: u64 }
pub fn conflict_diff(project: &Project, key: &str) -> Result<ConflictDiff, KernelError>;
/// Applies `scan` to the store's project: units with a dirty asset become conflicts (`unsaved`),
/// the others as `apply_one_sided`; loaded, clean assets of written units are reloaded.
pub fn sync_store(store: &mut Store, scan: ScanReport) -> Result<SyncOutcome, KernelError>;
/// The user's choice for the conflict `key` (see the decision table); returns the assets to reload.
pub fn resolve(store: &mut Store, key: &str, keep: Keep) -> Result<Vec<AssetId>, KernelError>;
/// For a project without store (g4k, MCP headless without changes).
pub fn resolve_files(project: &Project, key: &str, keep: Keep) -> Result<(), KernelError>;
pub const NO_CONFLICT: &str = "Für diese Einheit gibt es keinen Konflikt mehr.";
```

- `Other::Files`, wenn nur die Quelle von außen geändert ist und das Asset dirty ist; sonst `Dspre`. Diff: DSPRE-Seite in den Speicher exportiert (nie geschrieben), beide als `serde_json::Value`, Pfade wie in den Import-Fehlern (`data.power`, `learnset.moves[3].move`), Werte kompakt; Textarchive über ihre JSON (`messages[2].text`); Scripts Zeilen (`path` = `Zeile 12`); Binäres `Größe`/`xxh3`.
- `resolve` prüft, dass die Einheit noch Konflikt ist (sonst `Refused(NO_CONFLICT)`), wählt wie in der Entscheidungstabelle, zeichnet auf, gibt die Assets zum Neuladen zurück.

- [ ] **Step 1: Scheiternde Tests:**
  - `conflict_diff_names_field_paths_with_both_values` (Attacke 1: JSON `power` 40 → 45, Member auf 50 → eine Zeile `data.power`, `45`, `50`).
  - `conflict_diff_of_a_text_archive_names_the_message`, `conflict_diff_of_a_script_names_lines`, `conflict_diff_caps_at_forty_lines_and_counts_the_rest`.
  - `keep_studio_writes_the_member_from_the_json_and_records`, `keep_dspre_writes_the_json_from_the_member_and_records`.
  - `an_unsaved_asset_with_a_dspre_change_is_a_conflict_not_a_reload` (Review Focus: dirty Attacke 1, Member von außen → `conflicts[0].unsaved`, nichts geschrieben, Asset weiter dirty; `Keep::Studio` → Member aus JSON, Asset weiter dirty mit dem ungespeicherten Wert; zweiter Fall `Keep::Dspre` → Asset neu geladen, nicht mehr dirty, Wert von DSPRE).
  - `an_unsaved_asset_with_a_changed_json_offers_the_file` (`other == Files`; `Keep::Studio` → `accept_disk_as_base`, das nächste `save` schreibt den Store-Wert ohne `Stale`).
  - `sync_store_reloads_clean_loaded_assets_it_rewrote`.
  - `resolve_of_a_settled_unit_is_refused`.
- [ ] **Step 2: Gated Test** `simulated_change_on_both_sides_is_a_conflict_and_both_choices_work`: Kopie, Export, `record_all`; Trainer 1 in JSON und Member verschieden geändert → `sync_store` → ein Konflikt; `conflict_diff` nicht leer; `Keep::Studio` → Member = Import der JSON; dasselbe noch einmal herbeiführen, `Keep::Dspre` → JSON = Export des Members; danach `scan` leer und `build::build` ohne `textproject.conflict`.
- [ ] **Step 3: Lauf** → FAIL; **Step 4: Umsetzen**; **Step 5: Lauf** → PASS (gated einmal auf HeartGold US).
- [ ] **Step 6: Commit** `feat(textproject): conflicts with a field diff and a choice per unit, unsaved store values never lost` (mit Trailer).

### Task 6: Umstellung und Zurücknehmen

**Files:**
- Create: `crates/g4-kernel/src/textproject/adopt.rs` (+ Tests)
- Modify: `crates/g4-kernel/src/textproject/export.rs` (`ExportProgress`, `export_reported`; `export` ruft es mit leerem Rückruf), `crates/g4-kernel/src/scripts.rs` (`pub(crate) fn ensure_include_root(project: &Project, root: &str) -> Result<bool, KernelError>`), `crates/g4-kernel/src/start.rs` (`CreateOptions`, `create_project_with_options`, `Phase::TextProject`, `Created.text_project`), `crates/g4-kernel/src/bin/g4k.rs` (`text adopt`, `text revert`, `text sync`), `crates/g4-kernel/tests/cli.rs`, `crates/g4-kernel/tests/textproject_source.rs` (gated), `crates/g4-kernel/tests/project_start.rs` (gated, `project_start_`), `docs/formats/text_project.md` (Abschnitt „Einrichten und Zurücknehmen“)

**Interfaces:**
- Consumes: Task 4 `record_all`, Task 8 `include::write_include` (bis Task 8 ein Aufruf, der `Ok(false)` liefert; Task 8 ersetzt ihn), 4.3a `export`, `migrate`, `scripts::adopt_as`.
- Produces:

```rust
// export.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)] #[serde(rename_all = "snake_case")]
pub enum ExportStep { Scripts, BaseLayer, Arts, Blobs }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)] pub struct ExportProgress { pub step: ExportStep, pub done: u32, pub total: u32 }
pub fn export_reported(project: &Project, base: &BaseRom, opts: &ExportOptions,
                       progress: &mut dyn FnMut(ExportProgress)) -> Result<ExportReport, KernelError>;
// adopt.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)] #[serde(rename_all = "snake_case")]
pub enum AdoptPhase { Scripts, BaseLayer, Entities, Sync }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)] pub struct AdoptProgress { pub phase: AdoptPhase, pub done: u32, pub total: u32 }
#[derive(Debug, Clone, Serialize)] pub struct AdoptReport { pub export: ExportReport, pub include_written: bool, pub sync_files: u32, pub ms: u64 }
pub fn adopt(project: &Project, base: &BaseRom, progress: &mut dyn FnMut(AdoptProgress)) -> Result<AdoptReport, KernelError>;
#[derive(Debug, Clone, Serialize)] pub struct RevertReport { pub backup: PathBuf, pub rewritten: Vec<PathBuf>, pub ms: u64 }
pub fn revert(project: &Project) -> Result<RevertReport, KernelError>;
pub const INCLUDE_ROOT: &str = ".g4/include";
pub const REVERT_PENDING: &str = "Erst abgleichen: es gibt Änderungen von außen, Konflikte oder fehlerhafte Dateien (Technik › Textprojekt).";
// start.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateOptions { pub template: Template, pub text_project: bool }
pub fn create_project_with_options(rom: &Path, dest: &Path, opts: &CreateOptions,
                                   progress: &mut dyn FnMut(Progress)) -> Result<Created, KernelError>;
pub enum Phase { Read, Extract, Template, Check, TextProject }   // serde "text_project"
```

- `adopt`: lehnt ab wie der Export (Diamant/Perl, ohne Marke, andere Basis, neuere Schema-Version); `Scripts` aus `adopt_as`, `BaseLayer`/`Entities` aus `export_reported` (`Arts` zählt die 16 Arten, `Blobs` gehört zu `Entities`), dann `ensure_include_root(INCLUDE_ROOT)`, `write_include`, `record_all`, `record_text_sync` aller Archive (`Sync`). Ein zweites `adopt` auf einem Textprojekt ist ein Export ohne Änderung plus Aufzeichnung.
- `revert`: `scan` muss ohne `Dspre`, `Files`, `Conflict`, `Broken` sein (sonst `Refused(REVERT_PENDING)`); dann `g4-world.json` und `g4-checks.json` mit Nummern neu schreiben (Task 7 liefert die Schreiber mit `Resolver`; bis dahin unverändert), dann `g4-project.json`, `data/`, `.g4/include/` per `rename` nach `.g4/backup/textproject-<UTC wie migrate>/`, `sync` leeren, `rotom.toml` behält die Include-Wurzel (harmlos ohne Datei). Ungespeicherte Werte prüft die App.
- `create_project_with_options` mit `text_project`: nach `Check` (Marke geschrieben), vor dem `rename` des Staging-Ordners `adopt` mit `BaseRom::open(rom)`; Fortschritt als `Phase::TextProject`.
- `g4k text adopt <root> [--base-rom <nds>]`, `g4k text revert <root>`, `g4k text sync <root> [--keep studio|dspre <schlüssel>]` (ohne `--keep`: scannen, Einseitiges übernehmen, Konflikte und Fehlerhaftes als Zeilen `conflict <schlüssel> <label>` bzw. `broken …`, Exit 1 bei Konflikten).

- [ ] **Step 1: Scheiternde Tests** (`adopt.rs`, `TextFixture::dspre()`): `adopt_writes_the_text_project_records_sync_and_reports_phases_in_order`, `adopt_adds_the_include_root_to_rotom_toml_once`, `a_second_adopt_writes_nothing`, `revert_moves_manifest_data_and_include_to_the_backup_and_the_dspre_folder_still_builds`, `revert_with_a_pending_external_change_is_refused`, `adopt_without_a_base_mark_is_refused_with_the_export_sentence`; `start.rs`: `create_with_text_project_reports_the_phase_and_leaves_a_text_project` (synthetische ROM); `cli.rs`: `cli_text_adopt_sync_and_revert`.
- [ ] **Step 2: Gated Tests:** `adopt_dspre_folder_builds_byte_equal_and_reverts` (`textproject_source.rs`: Kopie, `adopt` mit der Basis-ROM (Zeiten je Phase drucken), `build::build` → SHA-1 = Basis-ROM; `revert` → Manifest weg, Backup da, `build::build` → SHA-1 = Basis-ROM); `vanilla_checks_on_the_text_project_have_no_errors` (nach `adopt` und einer Store-Bearbeitung je Art Attacke, Trainer, Header: der Prüfkatalog wie in `tests/vanilla_checks.rs` meldet 0 Fehler); `project_start_text_project_builds_byte_equal` (`tests/project_start.rs`, `G4_TEST_ROM`: `create_project_with_options` mit `text_project: true`, `build::build` → SHA-1 = ROM).
- [ ] **Step 3: Lauf** → FAIL; **Step 4: Umsetzen**; **Step 5: Lauf** → PASS (gated je einmal auf Platin DE).
- [ ] **Step 6: Doku** `text_project.md` „Einrichten und Zurücknehmen“: Phasen, Voraussetzungen, warum Zurücknehmen das Manifest verschiebt (Begründung aus der Entscheidungstabelle), was im Backup liegt.
- [ ] **Step 7: Commit** `feat(textproject): set up a DSPRE folder as text project with progress and take it back by moving the manifest` (mit Trailer).

### Task 7: Symbolische IDs in `g4-world.json`, `g4-checks.json` und den Rezepten

**Files:**
- Modify: `crates/g4-kernel/src/world_meta.rs` (`load`, `save` mit `&Resolver`), `crates/g4-kernel/src/checks/ignore.rs` (`IgnoreFile.version`, Subjekte als `RefInput` in der Datei, `IgnoreSet::{load, save}` mit `&Resolver`), `crates/g4-kernel/src/templates.rs` (`RefInput` im YAML, Auflösung), `crates/g4-kernel/metadata/templates.yaml`, `crates/g4-kernel/src/textproject/adopt.rs` (Rücknahme schreibt Nummern), `apps/studio/src-tauri/src/commands/world_meta.rs`, `apps/studio/src-tauri/src/commands/checks.rs` (`load_ignore` und die Schreiber reichen den `Resolver` der Sitzung), `docs/formats/templates.md`, `docs/formats/` (Abschnitte zu `g4-world.json` und `g4-checks.json`, wo sie beschrieben sind)

**Interfaces:**
- Consumes: Task 1 `Resolver`, `RefInput`.
- Produces:

```rust
// world_meta.rs
pub fn load(root: &Path, ids: &Resolver) -> (WorldMeta, Option<String>);
pub fn save(root: &Path, meta: &WorldMeta, ids: &Resolver) -> Result<(), KernelError>;
// checks/ignore.rs
pub const IGNORE_VERSION: u32 = 1;
/// The file form of an IgnoreKey: `subject.index` as id or number (`Subject` itself stays numeric for Tauri and MCP).
pub struct IgnoreKeyFile { pub code: String, pub subject: SubjectFile, pub key: String, #[serde(with = "fingerprint_serde")] pub fingerprint: u64 }
pub struct SubjectFile { pub kind: SubjectKind, #[serde(default)] pub index: RefInput }
pub struct IgnoreFile { #[serde(default = "one")] pub version: u32, pub ignored: Vec<IgnoreKeyFile>, pub disabled: Vec<String> }
impl IgnoreSet {
    pub fn load(root: &Path, ids: &Resolver) -> Result<IgnoreSet, KernelError>;
    pub fn save(&self, root: &Path, ids: &Resolver) -> Result<(), KernelError>;
}
// templates.rs: RegionRecipe.starter_species bleibt u16 (über Resolver::fixed()), start_town bleibt der interne Name
```

- `WorldMeta` im Speicher bleibt mit `u16`; nur die Datei liest `"12"` oder `"MAP_T01"` als Schlüssel und Wert von `roomNames`/`roomOwner` und schreibt `resolver.output(Map, n)`. Unbekannte ID → `(WorldMeta::default(), Some("unbekannte ID MAP_X in roomNames"))`, Datei unberührt.
- `g4-checks.json`: `version` fehlt → 1, `> 1` → `InvalidProject("g4-checks.json: Version {v} stammt aus einer neueren Fassung von Gen4 Studio.")`; nur `header` und `trainer` als ID, die übrigen Subjekte bleiben Zahlen.
- `templates.yaml`: `start_town: MAP_T01` / `MAP_T20`, `species: SPECIES_TURTWIG` / `SPECIES_CHIKORITA`; `region_recipe` liest auch `T01` und `387`; `start_town(family)` gibt weiter den internen Namen (`T01`) zurück.

- [ ] **Step 1: Scheiternde Tests:** `world_meta`: `ids_and_numbers_read_alike`, `a_text_project_writes_map_ids_and_a_dspre_folder_numbers`, `an_unknown_id_reads_broken_and_is_not_overwritten`; `ignore.rs`: `version_one_is_written_and_a_missing_version_reads_as_one`, `a_newer_version_is_refused`, `header_and_trainer_subjects_read_and_write_ids_in_a_text_project`; `templates.rs`: `recipe_reads_ids_and_the_old_numbers`, `the_bundled_recipe_names_ids`; `adopt.rs`: `revert_writes_world_and_checks_with_numbers`.
- [ ] **Step 2: Gated Test** `recipes_with_ids_apply_on_the_vanilla_project` (`textproject_source.rs`): Kopie, `adopt`, `plan_empty_region` → Startstadt = Header mit internem Namen `T01` bzw. `T20`, Starter 387 bzw. 152.
- [ ] **Step 3: Lauf** → FAIL; **Step 4: Umsetzen** (die App-Aufrufer bekommen den `Resolver` aus der Sitzung, Task 10 cached ihn; bis dahin `Resolver::for_project` je Aufruf); **Step 5: Lauf** → PASS, `cargo test -p studio --all-features` grün.
- [ ] **Step 6: Commit** `feat(project-files): world file, ignore file (version 1) and template recipes take an id or a number` (mit Trailer).

### Task 8: Scripts im Referenzindex und die Include-Datei

**Files:**
- Create: `crates/g4-kernel/metadata/script_refs.yaml`, `crates/g4-kernel/src/textproject/include.rs` (+ Tests)
- Modify: `crates/g4-kernel/src/data/refs.rs` (Quelle Scripts, Auffrischen über `stat`, Modul-Doku), `crates/g4-kernel/src/textproject/mod.rs`, `crates/g4-kernel/src/textproject/source.rs` und `adopt.rs` (rufen `write_include`), `crates/g4-kernel/tests/refs.rs`, `crates/g4-kernel/tests/textproject_source.rs` (gated), `docs/formats/text_project.md` (Abschnitt „IDs in Scripts“)

**Interfaces:**
- Consumes: Task 1 `Resolver`; `scripts::{parse_source, numbered_files, SOURCE_ROOT, scrcmd_database}`; rotom AST (`StatementKind`, `ExpressionKind`).
- Produces:

```rust
// textproject/include.rs
pub const INCLUDE_FILE: &str = ".g4/include/g4_ids.rotom";
/// The defines of every table and register id (no fallbacks), sorted by kind and number.
pub fn include_text(project: &Project) -> Result<String, KernelError>;
/// Writes INCLUDE_FILE when it changes; `false` outside a text project.
pub fn write_include(project: &Project) -> Result<bool, KernelError>;
// data/refs.rs
pub(crate) struct ScriptRefTable { /* Familie → Befehl → Parametername → Zielart, aus script_refs.yaml */ }
pub(crate) fn script_ref_table(family: Family) -> &'static ScriptRefTable;
impl RefIndex {
    /// Also re-parses script sources whose `stat` changed since they were read (a save in the
    /// script editor, DSPRE, a branch switch); a removed source drops its references.
    pub fn refresh(&mut self, store: &Store) -> Result<Vec<AssetId>, KernelError>;   // erweitert
}
```

- `script_refs.yaml` je Familie (`pt`, `hgss`), mindestens: Trainer `SetTrainerFlag.trainer_id`, `ClearTrainerFlag.trainer_id`, `CheckTrainerFlag.trainer_id`, `StartTrainerBattle.trainer_1`, `StartTrainerBattle.trainer_2`, `PrintTrainerDialogue.trainer_id`, `GetRematchTrainerID.trainerID`; Item `AddItem.item`, `RemoveItem.item`, `CanFitItem.item`, `CheckItem.item`, `BufferItemName.item`; Spezies `GivePokemon.pokémon`; die HGSS-Namen aus `hgss_v2.json` nachsehen. Ein Argument zählt, wenn es ein Zahl-Literal oder ein Bezeichner ist, den der `Resolver` für die Zielart kennt.
- Ohne übernommene Scripts gibt es keine Script-Quelle (wie heute keine Verweise aus Scripts).

- [ ] **Step 1: Scheiternde Tests:** `include.rs`: `include_defines_table_and_register_ids_sorted`, `include_is_written_only_when_it_changes`, `outside_a_text_project_nothing_is_written`; `refs` (Testmodul oder `tests/refs.rs`): `script_refs_name_existing_commands_and_params` (jeder Eintrag der YAML steht in der gebündelten Befehlsdatenbank seiner Familie), `a_trainer_battle_in_a_script_is_a_reference_with_its_line`, `an_id_constant_counts_like_its_number`, `a_variable_argument_is_no_reference`, `a_script_saved_outside_the_store_is_reread_on_refresh`; `textproject_source` (synthetisch, adoptiert): `a_script_using_an_id_constant_compiles_to_the_number` (Quelle mit `#include "g4_ids.rotom"` und `StartTrainerBattle TRAINER_…, 0` kompiliert zum selben Member wie mit der Zahl).
- [ ] **Step 2: Gated Test** `script_refs_on_the_vanilla_project_are_counted`: Kopie, `adopt`, `RefIndex::rebuild_full` → Zahl der Script-Verweise je Zielart > 0 für Trainer und Items; Druckt `refs <game_code> scripts trainer=<n> item=<n> species=<n> ms=<n>`.
- [ ] **Step 3: Lauf** → FAIL; **Step 4: Umsetzen**; **Step 5: Lauf** → PASS (gated einmal auf Platin DE).
- [ ] **Step 6: Doku** „IDs in Scripts“: Include-Datei, `include_roots`, Beispiel `#include` und eine Zeile mit `TRAINER_…`, dass vorhandene Quellen nicht umgeschrieben werden.
- [ ] **Step 7: Commit** `feat(refs): script sources as a source of references, ids as rotom constants in a generated include` (mit Trailer).

### Task 9: „ID umbenennen“ im Kernel

**Files:**
- Create: `crates/g4-kernel/src/textproject/rename.rs` (+ Tests)
- Modify: `crates/g4-kernel/src/textproject/mod.rs`, `crates/g4-kernel/src/bin/g4k.rs` (`text rename-id`), `crates/g4-kernel/tests/cli.rs`, `crates/g4-kernel/tests/textproject_source.rs` (gated), `docs/formats/text_project.md` (Abschnitt „ID umbenennen“)

**Interfaces:**
- Consumes: Task 1, 7 (Schreiber von `g4-world.json`, `g4-checks.json`), Task 8 (`write_include`, rotom-Lexer); 4.1b `FileStep`, `FileChange`, `Store::record_file_step`.
- Produces:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize)] pub struct RenameFile { pub path: PathBuf, pub count: u32 }
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RenamePlan { pub kind: SymbolKind, pub number: u32, pub old_id: String, pub new_id: String,
                        pub files: Vec<RenameFile>, pub total: u32, pub refusal: Option<String> }
#[derive(Debug, Clone, Serialize)] pub struct RenameResult { pub plan: RenamePlan, pub change: ChangeId, pub ms: u64 }
/// What `rename_id` would change; a refusal (see the rules) is `refusal`, nothing is written.
pub fn plan_rename(project: &Project, old_id: &str, new_id: &str) -> Result<RenamePlan, KernelError>;
/// Writes every file of the plan and records them as one file step labelled
/// "ID umbenannt: {old} → {new}" (asset: Trainer n, else HEADER_TABLE).
pub fn rename_id(store: &mut Store, old_id: &str, new_id: &str) -> Result<RenameResult, KernelError>;
/// Without a store (g4k): the same files, no undo.
pub fn rename_id_files(project: &Project, old_id: &str, new_id: &str) -> Result<RenamePlan, KernelError>;
```

- Ablehnungen (`refusal`, wörtlich): `Kein Textprojekt: IDs gibt es erst nach „Als Textprojekt einrichten…“.`; `Unbekannte ID: {old}.`; `{old} steht in einer festen Tabelle; umbenennen lassen sich nur IDs mit Register (MAP_, TRAINER_, TRAINER_CLASS_, PLACE_).`; `Die neue ID muss mit {PREFIX}_ beginnen und darf danach nur A–Z, 0–9 und _ enthalten (höchstens 64 Zeichen).`; `{new} gibt es schon (Nummer {n}).`; `{new} ist die Ersatz-ID der Nummer {n}.`; `Die neue ID ist die alte.`. `MAP_` und `TRAINER_CLASS_` werden nach dem längsten passenden Präfix getrennt (`TRAINER_CLASS_X` ist eine Klasse, kein Trainer).
- Undo und Redo prüfen jede Datei gegen ihre erwartete Seite (Regel von 4.1b); danach liest `EntitySource` die Register neu (`Resolver::is_current`), und die Include-Datei wird neu geschrieben.

- [ ] **Step 1: Scheiternde Tests** (`TextFixture::exported()`, Scripts adoptiert, eine Quelle mit `#include` und `TRAINER_YOUNGSTER_KAI` im Code und im Kommentar):
  - `plan_counts_every_reference_per_file` (Register 1, Trainer-JSON 1, Script 1 (nicht der Kommentar), Include 1, `g4-checks.json` mit einem ignorierten Trainer-Befund 1).
  - `rename_writes_all_files_as_one_undo_step_and_undo_restores_them` (danach `Resolver` kennt nur die neue ID, `build` unverändert; `undo` → alle Dateien byte-gleich vorher).
  - `rename_refuses_taken_fallback_and_malformed_ids_and_writes_nothing` (Review Focus: je Ablehnung; mtimes aller Dateien unverändert).
  - `undo_of_a_rename_after_an_outside_edit_writes_nothing` (Review Focus: Trainer-JSON danach von außen geändert → `undo` `Refused` mit dem Satz von 4.1b, keine Datei geschrieben, Schritt fällt aus dem Verlauf).
  - `a_map_rename_updates_warps_world_file_and_headers`, `a_trainer_class_rename_is_not_taken_for_a_trainer`, `a_fixed_table_id_is_refused`; `cli.rs`: `cli_text_rename_id`.
- [ ] **Step 2: Gated Test** `rename_id_updates_every_reference_on_the_vanilla_project`: Kopie, `adopt`; einen Trainer mit Script-Verweis aus dem Referenzindex wählen, in einer Script-Quelle seine Zahl durch die Konstante ersetzen und `#include` voranstellen, kompilieren; `rename_id` → Plan nennt Register, Trainer-JSON, Script, Include; `build::build` → SHA-1 wie vor dem Umbenennen; `undo` → Dateien wie vorher. Druckt `rename <game_code> files=<n> refs=<n> ms=<n>`.
- [ ] **Step 3: Lauf** → FAIL; **Step 4: Umsetzen**; **Step 5: Lauf** → PASS (gated einmal auf HeartGold DE).
- [ ] **Step 6: Doku** „ID umbenennen“: welche IDs, Regeln, was nachgezogen wird, ein Undo-Schritt.
- [ ] **Step 7: Commit** `feat(textproject): rename an id with every reference as one undo step` (mit Trailer).

### Task 10: App-Befehle, Brücke und MCP

**Files:**
- Create: `apps/studio/src-tauri/src/commands/textsource.rs` (+ Tests), `apps/studio/src-tauri/src/mcp/symbolic.rs` (+ Tests)
- Modify: `src/commands/mod.rs`, `src/commands/project.rs` (`ProjectInfo.text_project`, Öffnen nach `source_mode`, Ablehnung neuer/kaputter Schemata), `src/commands/start.rs` (`create_project` mit `text_project: bool`), `src/state.rs` (`Session.ids: Resolver`, `Session::files_written` ruft `source::after_member_write` für geschriebene Binär-Member, `Session::ids()` frischt bei geänderten Registern auf), `src/lib.rs` (Handler anhängen), `src/commands/dispatch.rs` (anhängen, Beispielargumente), `src/mcp/access.rs`, `src/mcp/catalog.rs`, `src/mcp/tools.rs` (`Call::prepare` ruft `symbolic::resolve_args`; `not_in_batch("rename_id")`), `src/mcp/mod.rs` bzw. der Start von `studio mcp` (lehnt `Older`/`Newer`/`Broken` mit dem Satz ab), `docs/mcp.md`

**Interfaces:**
- Consumes: Task 3 bis 9; 4.3a `commands::textproject::check_text_export`; 4.1b `found_base`, `base_missing`.
- Produces:

```rust
#[derive(Debug, Serialize)] #[serde(tag = "state", rename_all = "snake_case")]
pub enum TextProjectStatusDto {
    Dspre { adoptable: bool, reason: Option<String> },        // reason: Diamant/Perl, ohne Marke, Basis-ROM fehlt
    Current { schema: u32, last_sync_at: Option<i64> },     // Unix-Millisekunden aus `sync.scanned_at_ns`
    Older { schema: u32 }, Newer { schema: u32 }, Broken { reason: String },
}
pub struct ProjectInfo { /* wie heute */ pub text_project: TextProjectStatusDto }
pub fn text_project_status_impl(state: &AppState) -> Result<TextProjectStatusDto, CmdError>;
pub fn adopt_text_project_impl(state: &AppState, progress: &mut dyn FnMut(AdoptProgress)) -> Result<AdoptReport, CmdError>;
pub fn revert_text_project_impl(state: &AppState) -> Result<RevertReport, CmdError>;
pub fn migrate_text_project_impl(state: &AppState) -> Result<MigrateReport, CmdError>;
pub fn external_sync_impl(state: &AppState, progress: &mut dyn FnMut(SyncProgress)) -> Result<SyncOutcome, CmdError>;
#[derive(Debug, Serialize)] pub struct ExternalChangesDto { pub scan: ScanReport, pub diffs: BTreeMap<String, ConflictDiff> }
pub fn external_changes_impl(state: &AppState, diff: bool) -> Result<ExternalChangesDto, CmdError>;
pub fn conflict_diff_impl(state: &AppState, key: String) -> Result<ConflictDiff, CmdError>;
pub fn resolve_external_conflict_impl(state: &AppState, key: String, keep: Keep) -> Result<Vec<AssetId>, CmdError>;
pub fn plan_rename_id_impl(state: &AppState, id: String, new_id: String) -> Result<RenamePlan, CmdError>;
pub fn rename_id_impl(state: &AppState, id: String, new_id: String) -> Result<RenameResult, CmdError>;
pub fn entity_ids_impl(state: &AppState, kind: SymbolKind) -> Result<BTreeMap<u32, String>, CmdError>;
// mcp/symbolic.rs
pub(crate) struct ArgRef { pub command: &'static str, pub pointer: &'static str, pub kind: RefKindOf }
pub(crate) enum RefKindOf { Fixed(SymbolKind), FromKindField(&'static str) }   // "species"→Species, "move"→Move, "item"→Item, "trainer"→Trainer, "header"→Map
pub(crate) const SYMBOLIC_ARGS: &[ArgRef];
pub(crate) fn resolve_args(command: &str, args: Value, ids: &Resolver) -> Result<Value, ToolError>;
```

- `SYMBOLIC_ARGS` mindestens: `data_get`/`data_edit`/`data_raw` `/index` (aus `/kind`), `data_edit` `/patch/*/value` (wenn `record::ref_kind(record, field)` ein Verweis ist), `refs` `/target/index` (aus `/target/kind`), `get_mon_icon` `/species`, `get_item_icon` `/item`, `create_trainer` `/class` (TrainerClass), `set_trainer_texts` `/trainer`, `get_header`/`set_header` `/id` (Map), `split_encounter_file` `/header`, `split_room` `/header`, `resolve_script` `/header`, `update_event` `/patch/dest_header_id` (Map), `plan_reset`/`reset_to_original` `/target/index` (species, trainer) und `/target/id` (header). Unbekannt → `ToolError { code: "unknown_id", message: unknown_id_text(id) }`.
- `SymbolKind` bekommt `Deserialize` (für `entity_ids`); `SubjectKind` aus `checks` wird für Task 7 `pub(crate)`.
- Fehlercodes: ohne Textprojekt `not_text_project` („Kein Textprojekt: …“), ungespeichert `unsaved` (wie 4.3a), Basis fehlt `base_missing`, ausstehende Änderungen beim Zurücknehmen `pending` (`REVERT_PENDING`), Konflikte beim Bauen `conflicts` („Erst die Konflikte in Technik › Textprojekt lösen.“).
- `adopt_text_project_impl` prüft vor der Frage (Muster 4.3a B3): `unsaved`, `base_missing`, Diamant/Perl; adoptiert die Scripts selbst; öffnet danach die Sitzung auf demselben Pfad neu (`open_project_impl`). `external_sync_impl`: `scan` ohne Sitzungsschloss, dann unter Agenten-Zug und Schloss `sync_store`, danach `session.disk_changed()` wenn etwas geschrieben wurde. `build_rom` im Textprojekt ruft vorher `external_sync_impl` und lehnt mit `conflicts` ab.
- Zugriff (anhängen): `text_project_status` Read, `adopt_text_project` `Irreversible("Als Textprojekt einrichten")`, `revert_text_project` APP_ONLY, `migrate_text_project` APP_ONLY, `external_sync` APP_ONLY, `external_changes` Read, `conflict_diff` Read, `resolve_external_conflict` `Irreversible("Eine Fassung überschreiben")`, `plan_rename_id` Read, `rename_id` Write, `entity_ids` Read. Katalog (englisch): `text_project_status`, `adopt_text_project` („cannot be undone; headless only with --allow-irreversible“), `external_changes` (`{ diff?: boolean }`, „reads, writes nothing; the app takes one-sided changes over by itself“), `resolve_external_conflict` (`{ key, keep: "studio"|"dspre" }`), `rename_id` (`{ id, new_id }`, „one undo step; not in g4_batch“); jede Beschreibung eines Werkzeugs mit Nummern-Argumenten bekommt den Satz „Numbers or symbolic ids (SPECIES_PIKACHU, MAP_T01, TRAINER_…) are accepted.“ über `data_note!()` bzw. eine neue Konstante.

- [ ] **Step 1: Scheiternde Tests** (`commands/textsource.rs`, `TextFixture` über die vorhandenen Test-Helfer der App):
  - `status_names_dspre_with_its_reason_and_current_after_adopt`, `adopt_refuses_unsaved_and_missing_base_before_asking`, `adopt_reopens_the_session_in_json_mode` (`data_edit` danach ändert die JSON).
  - `external_sync_takes_over_a_member_change_and_reports_it`, `build_with_a_conflict_is_refused_with_conflicts`.
  - `rename_over_the_command_records_one_undo_step`, `open_refuses_a_newer_schema_with_the_manifest_sentence`, `open_reports_an_older_schema`.
  - `files_written_exports_a_reset_texture` (4.1b-Reset einer Kartentextur im Textprojekt → `data/map_textures/NNNN.nsbtx`, dann `undo` → Datei wieder weg).
  - `symbolic.rs`: `species_id_and_number_give_the_same_arguments`, `patch_values_of_reference_fields_are_resolved`, `an_unknown_id_is_unknown_id`, `every_symbolic_arg_names_a_command_of_the_table`.
  - `access.rs`: die neuen Zeilen; `catalog.rs`: `the_curated_set_is_the_decided_one` (lesend `text_project_status`, `external_changes`; schreibend `rename_id`; unumkehrbar `adopt_text_project`, `resolve_external_conflict`), `tool_count_is_about_thirty` (Grenze heben, im Commit begründen); `tools.rs`: `batch_refuses_rename_id`, `data_edit_with_species_id_acts_like_the_number`.
  - gated (übersprungen ohne `G4_TEST_PROJECT`): `mcp_with_ids_acts_like_numbers_on_the_vanilla_copy` (Kopie, `adopt_text_project_impl`, `data_edit` Pikachu `attack` +1 einmal mit `"index": "SPECIES_PIKACHU"`, `undo`, dann mit `25` → gleiche Antwort und gleiche JSON).
- [ ] **Step 2: Lauf** `cargo test -p studio --all-features` → FAIL; **Step 3: Umsetzen** (Tauri: `adopt_text_project` und `external_sync` `async` mit `spawn_blocking` und `Channel` für den Fortschritt, `create_project` mit `textProject`); **Step 4: Lauf** → PASS, dazu `cargo test -p studio --test mcp_stdio`.
- [ ] **Step 5: Doku** `docs/mcp.md`: die Werkzeuge mit Zugriff, Rückfrage und Batch-Ausnahme, die Befehle ohne Werkzeug, „ID oder Nummer“ mit der Liste der Argumente, die Fehlercodes.
- [ ] **Step 6: Commit** `feat(studio): text project commands, external sync and rename over the bridge and MCP, ids in tool arguments` (mit Trailer).

### Task 11: Frontend: Abgleich bei Öffnen und Fokus, Technik › Textprojekt

**Files:**
- Create: `apps/studio/src/api.textproject.ts` (+ test), `apps/studio/src/platform/windowFocus.ts` (+ test), `apps/studio/src/app/useExternalSync.ts` (+ test), `apps/studio/src/technik/TextProjectSurface.tsx` (+ `.module.css`, test), `apps/studio/src/technik/syncWords.ts` (+ test)
- Modify: `apps/studio/src/api.ts` (`export * from "./api.textproject";`), `apps/studio/src/technik/TechnikArea.tsx` (+ test: Eintrag `textprojekt` nach `projekt`, Untertitel, Weiche), `apps/studio/src/ui/primitives/icons.ts` (`textproject: FileCode`), `apps/studio/src/App.tsx` (Hook, nach einem Lauf `reloadChanged(outcome.reloaded, true)`, Meldungen), `apps/studio/src/app/useProjectSession.ts` (Build: vorher abgleichen, bei `conflicts` nicht bauen)

**Interfaces:**
- Consumes: Task 10 (Befehle, DTOs); `toast` aus `ui/frame/toasts`; `Section`, `ListRow`, `Badge`, `Button`, `Notice`, `EmptyState`, `Table`, `CheckList`.
- Produces (TS):

```ts
export type TextProjectStatus =
  | { state: "dspre"; adoptable: boolean; reason: string | null }
  | { state: "current"; schema: number; last_sync_at: number | null }
  | { state: "older"; schema: number } | { state: "newer"; schema: number } | { state: "broken"; reason: string };
export interface SyncEntry { key: string; label: string; assets: AssetId[] }
export interface ConflictEntry extends SyncEntry { unsaved: boolean; other: "dspre" | "files" }
export interface BrokenEntry { key: string; label: string; problems: Problem[] }
export interface SyncOutcome { adopted: SyncEntry[]; applied: SyncEntry[]; conflicts: ConflictEntry[]; broken: BrokenEntry[]; reloaded: AssetId[]; full: boolean; ms: number }
export interface SyncProgress { done: number; total: number }
export interface DiffLine { path: string; studio: string; dspre: string }
export interface ConflictDiff { lines: DiffLine[]; more: number }
export type Keep = "studio" | "dspre";
export const textProjectStatus: () => Promise<TextProjectStatus>;
export const externalSync: (onProgress: (p: SyncProgress) => void) => Promise<SyncOutcome>;
export const conflictDiff: (key: string) => Promise<ConflictDiff>;
export const resolveExternalConflict: (key: string, keep: Keep) => Promise<AssetId[]>;
// platform/windowFocus.ts
export function subscribeWindowFocus(onFocus: () => void): () => void;   // Tauri onFocusChanged(true); Browser: window "focus" und visibilitychange → visible
// app/useExternalSync.ts
export interface ExternalSyncApi { last: SyncOutcome | null; running: boolean; progress: SyncProgress | null; error: string | null; run: () => Promise<SyncOutcome | null> }
export function useExternalSync(opts: { project: string | null; textProject: boolean; busy: boolean; onOutcome: (o: SyncOutcome) => void }): ExternalSyncApi;
export const FOCUS_DELAY_MS = 800;
```

- `useExternalSync`: läuft beim Öffnen eines Textprojekts, bei Fokus nach `FOCUS_DELAY_MS`, höchstens ein Lauf; ein Fokus während eines Laufs merkt genau einen weiteren vor; während `busy` (Build) nichts, nach dem Ende von `busy` einmal, wenn ein Fokus kam; Fehler landen in `error` ohne Meldung bei Fokus-Läufen.
- Texte wörtlich (`syncWords.ts` hält die Zählformen):

| Stelle | Text |
|---|---|
| Meldung nach Übernahme aus DSPRE | `{n} Änderungen aus DSPRE übernommen` (1: `1 Änderung aus DSPRE übernommen`), Taste „Anzeigen“ → Technik › Textprojekt |
| Meldung nach Übernahme aus Dateien | `{n} geänderte Projektdateien übernommen` (1: `1 geänderte Projektdatei übernommen`), Taste „Anzeigen“ |
| Meldung bei Konflikten (Warnung) | `{n} Konflikte zwischen Studio und DSPRE` (1: `1 Konflikt zwischen Studio und DSPRE`), Taste „Lösen…“ |
| Meldung bei fehlerhaften Dateien (Fehler) | `{n} Projektdateien sind fehlerhaft` (1: `1 Projektdatei ist fehlerhaft`), Taste „Anzeigen“ |
| voller Abgleich | `role="status"`: „Gleiche mit DSPRE ab … {done} von {total} Dateien“; darunter „Das erste Mal dauert länger; danach geht es schnell.“ |
| Build mit Konflikten | „Erst die Konflikte in Technik › Textprojekt lösen.“, Taste „Lösen…“ |
| Technik-Liste, Untertitel | ohne Projekt „Kein Projekt geöffnet“; `dspre` „DSPRE-Layout“; läuft „Gleicht ab …“; Konflikte „{n} Konflikte“ (1: „1 Konflikt“); sonst „Abgeglichen“; `older` „Schema veraltet“; `newer`/`broken` „Nicht lesbar“ |
| Fläche, Kopf | Titel „Textprojekt“; `dspre`: „Das Projekt liegt im DSPRE-Layout. Als Textprojekt bekommt jeder Eintrag eine lesbare Datei unter data/, bereit für Git; DSPRE öffnet den Ordner weiter.“ + Taste „Als Textprojekt einrichten…“ (primär; gesperrt mit `reason`); `current`: „Textprojekt, Schema-Version {schema}. Zuletzt abgeglichen {ago}.“ + „Jetzt abgleichen“ (sekundär) + „Zurücknehmen…“ (sekundär) |
| Abschnitt Konflikte | Titel „Auf zwei Seiten geändert“ (`data-sync-conflicts`); je Zeile (`data-conflict=<key>`) das Label, `Badge` „Konflikt“ (Warnung) bzw. „Ungespeichert“ (`unsaved`), aufgeklappt eine `Table` mit Spalten „Feld“, „Studio“, „DSPRE“ (bzw. „Datei“ bei `other = files`) und bei `more > 0` „und {more} weitere Abweichungen“; Tasten „Studio-Fassung behalten“ und „DSPRE-Fassung übernehmen“ bzw. „Geänderte Datei übernehmen“ (beide sekundär); bei `unsaved` darüber `Notice` Warnung „„DSPRE-Fassung übernehmen“ verwirft die ungespeicherten Änderungen an diesem Eintrag.“ |
| Abschnitt Fehlerhaft | Titel „Fehlerhafte Dateien“ (`data-sync-broken`); je Zeile das Label und je Problem `{datei}:{zeile}:{spalte}: {satz}` in Mono |
| Abschnitt zuletzt übernommen | Titel „Zuletzt übernommen“; je Eintrag Label und `Badge` „aus DSPRE“ bzw. „aus Datei“; leer: „Nichts von außen geändert.“ |
| ohne Projekt | `EmptyState` „Kein Projekt geöffnet.“ / „Öffne ein Projekt, dann zeigt Technik hier sein Textprojekt.“ / „Projekt öffnen“ |

- [ ] **Step 1: Scheiternde Tests:** `api.textproject.test.ts` `commands_send_the_documented_arguments`; `windowFocus.test.ts` `browser_focus_and_visible_call_back_and_unsubscribe_stops`; `useExternalSync.test.tsx` `runs_on_open_of_a_text_project_only`, `focus_runs_after_the_delay`, `focus_during_a_running_sync_runs_once_more_after_it` (Review Focus), `no_sync_while_building_then_once_after` (Review Focus), `many_focus_events_make_one_run`; `syncWords.test.ts` `counts_have_singular_and_plural`; `TextProjectSurface.test.tsx` `dspre_offers_set_up_and_locks_it_with_the_reason`, `current_shows_schema_and_last_sync`, `a_conflict_row_shows_the_diff_and_both_choices_resolve`, `an_unsaved_conflict_warns_before_dspre`, `broken_files_name_file_line_column`, `without_a_project_offers_to_open_one`; `TechnikArea.test.tsx` `lists_eight_entries_with_textprojekt_after_projekt`, `textprojekt_subtitle_counts_conflicts`; `App.test.tsx` `a_sync_with_adoptions_reloads_and_says_so`, `a_build_with_conflicts_is_not_started`.
- [ ] **Step 2: Lauf** `cd apps/studio && pnpm test` → FAIL; **Step 3: Umsetzen** (CSS-Modul nur mit Tokens); **Step 4: Lauf** → PASS, `pnpm build`, `bash scripts/check-styles.sh`, `bash scripts/check-tokens.sh` grün.
- [ ] **Step 5: Blick in der Test-Umgebung** (`docs/agent-gate.md`, „Je Lauf“; Kopie `_testkopien/4-3b-ui_DSPRE_contents` mit `4-3b-ui.nds`, vorher `g4k text adopt`): Technik › Textprojekt im Zustand DSPRE-Layout, abgeglichen, mit einem Konflikt aufgeklappt, mit einer fehlerhaften Datei; Meldung „1 Änderung aus DSPRE übernommen“ (Fokus im Browser per `window.dispatchEvent(new Event("focus"))`); Hell, Dunkel, Kompakt; Screenshots nach `target/agent/gate-<datum>/4-3b/` (nicht ins Repo).
- [ ] **Step 6: Commit** `feat(technik): Textprojekt with external changes, conflicts with diff and choice, sync on open and focus` (mit Trailer).

### Task 12: Frontend: Einrichten, Hinweis, Assistent, Anheben, ID anzeigen und umbenennen

**Files:**
- Create: `apps/studio/src/textproject/AdoptDialog.tsx` (+ `.module.css`, test), `apps/studio/src/textproject/RevertDialog.tsx` (+ test), `apps/studio/src/textproject/MigrateDialog.tsx` (+ test), `apps/studio/src/textproject/RenameIdDialog.tsx` (+ `.module.css`, test), `apps/studio/src/textproject/textProjectHint.ts` (+ test), `apps/studio/src/textproject/useEntityIds.ts` (+ test)
- Modify: `apps/studio/src/api.textproject.ts` (`adoptTextProject`, `revertTextProject`, `migrateTextProject`, `planRenameId`, `renameId`, `entityIds`), `apps/studio/src/project/api.start.ts` (`createProject(…, textProject: boolean, …)`, `CreatePhase` + `"text_project"`), `apps/studio/src/project/NewProjectWizard.tsx` (+ test: Häkchen im Schritt „Vorlage“, Zusammenfassung, Phasenwort), `apps/studio/src/technik/TextProjectSurface.tsx` (öffnet die Dialoge), `apps/studio/src/App.tsx` (Hinweis nach dem Öffnen, Anheben-Dialog bei `older`, Fehler bei `newer`/`broken`), `apps/studio/src/data/trainer/TrainerView.tsx`, `apps/studio/src/workbench/inspector/HeaderEditor.tsx`, `apps/studio/src/data/pokedex/PokedexView.tsx` (ID-Zeile, Menüeintrag) mit ihren Tests

**Interfaces:**
- Consumes: Task 10, 11; `Dialog`, `CheckList`, `TextField`, `Notice`, `Checkbox`, `Menu`/`MenuEntry`, `HeadActions` bzw. `InspectorHead.actions` (4.1b).
- Produces (TS):

```ts
export type AdoptPhase = "scripts" | "base_layer" | "entities" | "sync";
export interface AdoptProgress { phase: AdoptPhase; done: number; total: number }
export interface AdoptReport { export: { files: number; written: number; ms: number }; include_written: boolean; sync_files: number; ms: number }
export interface RenameFile { path: string; count: number }
export interface RenamePlan { kind: string; number: number; old_id: string; new_id: string; files: RenameFile[]; total: number; refusal: string | null }
export const adoptTextProject: (onProgress: (p: AdoptProgress) => void) => Promise<AdoptReport>;
export const revertTextProject: () => Promise<{ backup: string; rewritten: string[]; ms: number }>;
export const migrateTextProject: () => Promise<unknown>;
export const planRenameId: (id: string, newId: string) => Promise<RenamePlan>;
export const renameId: (id: string, newId: string) => Promise<{ plan: RenamePlan; ms: number }>;
export const entityIds: (kind: "Species" | "Trainer" | "Map") => Promise<Record<number, string>>;
// textproject/textProjectHint.ts
export function shouldHint(root: string, status: TextProjectStatus): boolean;
export function dismissHint(root: string): void;     // localStorage, in try/catch
// textproject/useEntityIds.ts
export function useEntityIds(kind: "Species" | "Trainer" | "Map", enabled: boolean): Record<number, string> | null;
```

- Texte wörtlich:

| Stelle | Text |
|---|---|
| Hinweis beim Öffnen (Info) | „Dieses Projekt lässt sich als Textprojekt einrichten: eine lesbare Datei je Eintrag, bereit für Git. DSPRE öffnet den Ordner weiter.“ Tasten „Einrichten…“, „Nicht mehr fragen“; ohne Basis-ROM statt des zweiten Satzes „Dafür braucht es die Basis-ROM.“ und Taste „Basis-ROM zuordnen…“ statt „Einrichten…“ |
| Einrichten, Titel / Beschreibung | „Als Textprojekt einrichten?“ / „Schreibt g4-project.json und rund 6 500 Dateien unter data/. Die Spieldateien bleiben, wie sie sind; zurücknehmen geht in Technik › Textprojekt.“ |
| Einrichten, Prüfliste | „Alles gespeichert“, „Basis-ROM gefunden: {label}“, „Scripts übernommen“ (offen: „Scripts werden dabei übernommen“) |
| Einrichten, Fortschritt | `role="status"`: „Scripts übernehmen … {done} von {total}“, „Basisschicht anlegen …“, „Einträge schreiben … {done} von {total} Arten“, „Abgleich vorbereiten …“ |
| Einrichten, Tasten / danach | „Abbrechen“, „Einrichten“ (primär), während des Laufs keine Tasten; Meldung „Textprojekt eingerichtet: {files} Dateien in {s} s.“ |
| Zurücknehmen | Titel „Textprojekt zurücknehmen?“, Beschreibung „Das Studio arbeitet danach wieder direkt mit den DSPRE-Dateien. g4-project.json und data/ wandern nach .g4/backup/; die Spieldateien bleiben, wie sie sind.“; bei ungespeicherten Änderungen `Notice` Warnung „Erst speichern.“ und „Zurücknehmen“ gesperrt; Fehler `pending` als `Notice` mit dem Satz des Kernels; Tasten „Abbrechen“, „Zurücknehmen“ (primär); danach „Textprojekt zurückgenommen. Sicherung: {backup}“ |
| Anheben | Titel „Textprojekt anheben?“, Beschreibung „Dieses Textprojekt hat Schema-Version {v}; diese Fassung von Gen4 Studio schreibt Version {current}. Anheben sichert die geänderten Dateien vorher in .g4/backup/.“; Tasten „Abbrechen“ (schließt das Projekt), „Anheben“ (primär) |
| neuer/kaputt beim Öffnen | Fehlermeldung mit dem Satz des Kernels; das Projekt bleibt zu |
| Assistent, Häkchen | „Als Textprojekt einrichten“, Hilfetext „Eine lesbare Datei je Eintrag, bereit für Git; das Anlegen dauert etwas länger.“; Zusammenfassung hängt „, als Textprojekt“ an; Phasenwort „Textprojekt einrichten“ |
| ID-Zeile im Kopf | die ID in Mono (`data-entity-id`), Kontextmenü-Eintrag bzw. kleine Taste „ID kopieren“ |
| Menüeintrag | „ID umbenennen…“ (Trainer, Header; nur im Textprojekt) |
| Umbenennen, Titel / Feld | „ID umbenennen“ / `TextField` „Neue ID“, vorbelegt mit der alten |
| Umbenennen, Plan | `Notice` Fehler mit `refusal`, sonst Info „{total} Verweise in {files.length} Dateien werden nachgezogen.“ mit höchstens acht Pfaden und „und {n} weitere“; Plan verzögert 300 ms nach der Eingabe |
| Umbenennen, Tasten / danach | „Abbrechen“, „Umbenennen“ (primär, gesperrt bei `refusal`); Meldung „ID umbenannt: {old} → {new}. ⌘Z nimmt es zurück.“ |

- [ ] **Step 1: Scheiternde Tests:** `AdoptDialog.test.tsx` `checklist_then_progress_in_order_then_result`, `unsaved_locks_set_up`; `RevertDialog.test.tsx` `unsaved_locks_and_pending_shows_the_kernel_sentence`; `MigrateDialog.test.tsx` `cancel_closes_the_project_and_lift_migrates`; `RenameIdDialog.test.tsx` `plan_follows_the_input_after_a_pause`, `a_refusal_locks_rename`, `rename_says_undo_is_possible`; `textProjectHint.test.ts` `hints_once_per_root_until_dismissed`, `no_hint_for_a_text_project_or_dp`, `storage_errors_do_not_break_the_hint`; `useEntityIds.test.tsx` `reads_once_per_kind_and_only_when_enabled`; `NewProjectWizard.test.tsx` `text_project_checkbox_is_off_and_passes_through_with_its_phase`; `TrainerView.test.tsx`/`HeaderEditor.test.tsx` `text_project_shows_the_id_and_offers_rename`, `dspre_layout_shows_neither`; `PokedexView.test.tsx` `text_project_shows_the_species_id`; `App.test.tsx` `opening_a_dspre_folder_hints_and_an_older_schema_asks_to_lift`.
- [ ] **Step 2: Lauf** → FAIL; **Step 3: Umsetzen**; **Step 4: Lauf** → PASS, `pnpm build`, Stil- und Token-Prüfung grün.
- [ ] **Step 5: Blick in der Test-Umgebung** (wie Task 11, frische Kopie `4-3b-ui2`): Hinweis beim Öffnen, Einrichten mit Prüfliste und Fortschritt, Assistent mit Häkchen, Trainer-Kopf mit ID und „ID umbenennen…“, Umbenennen-Dialog mit Plan und mit Ablehnung; Hell, Dunkel, Kompakt.
- [ ] **Step 6: Commit** `feat(studio): set up, take back and lift a text project, the wizard checkbox, ids in heads and rename id` (mit Trailer).

### Task 13: Design-System fortschreiben

**Files:**
- Create: `design-system/openpencil/beispiele/textprojekt.body.js`, `design-system/openpencil/NACHTRAG-4-3b.md`
- Modify: `design-system/README.md` (Abschnitt „Nachtrag 4.3b“ nach „Nachtrag 4.2“), `design-system/Audit-Design-System.md` (Zeilen „Änderungen von außen“, „Konflikt“, „Textprojekt einrichten“)

**Interfaces:**
- Consumes: die Texte und Zustände aus Task 11 und 12, die Screenshots aus Task 11 Step 5 und Task 12 Step 5.

- [ ] **Step 1: README** „Nachtrag 4.3b“ nach dem Muster von „Nachtrag 4.1b“: Quelle (`technik/TextProjectSurface.tsx`, `technik/syncWords.ts`, `app/useExternalSync.ts`, `platform/windowFocus.ts`, `textproject/*`, die Köpfe aus Task 12, `project/NewProjectWizard.tsx`), Plan- und Gate-Verweis (`docs/phase-4-3b-gate.md`), Vergleichsbilder unter `target/agent/gate-<datum>/4-3b/` (Namen `blick-<fläche>-<zustand>-<thema>.png`), neues Symbol `textproject` (Phosphor `FileCode`), keine neuen Tokens; die Muster „Änderungen von außen“ (Meldung mit Zählform und „Anzeigen“, Abschnitt „Zuletzt übernommen“ mit den Plaketten „aus DSPRE“/„aus Datei“) und „Konflikt“ (Zeile mit Plakette, aufklappbare Feldtabelle Studio/DSPRE, zwei gleichrangige sekundäre Tasten, Warnung bei „Ungespeichert“); die Dialoge Einrichten (Prüfliste, Fortschritt in vier Phasen), Zurücknehmen, Anheben, ID umbenennen (Plan unter dem Feld); die ID-Zeile im Kopf; der Hinweis beim Öffnen; Abweichungen: keine Konflikt-UI für DSPRE-Ordner ohne Manifest (dort bleibt der Textdialog von 4.2), kein Umbenennen fester IDs, Trainerklassen und Orte nur über MCP; die `.fig` kennt das noch nicht.
- [ ] **Step 2: Vorlage** `beispiele/textprojekt.body.js` nach dem Muster von `auf-original.body.js` mit den Bausteinen aus `jsx-lib-app.js` und `v('token')`-Farben, `MODE`: `'technik'` (Frame „App: Technik, Textprojekt“: acht Listeneinträge, „Textprojekt“ gewählt, Kopf abgeglichen, ein Konflikt „Attacke 0001 · MOVE_POUND“ aufgeklappt mit zwei Zeilen, eine Zeile „Ungespeichert“, Abschnitt „Zuletzt übernommen“ mit drei Einträgen), `'einrichten'` (Frame „App: Dialog, Als Textprojekt einrichten“: Prüfliste und Fortschritt „Einträge schreiben … 9 von 16 Arten“), `'umbenennen'` (Frame „App: Dialog, ID umbenennen“: Trainer-Kopf mit ID-Zeile, Dialog mit Plan „7 Verweise in 4 Dateien“); Projekt „Sinnoh-Remix“, Platin DE.
- [ ] **Step 3: Anleitung** `NACHTRAG-4-3b.md` nach dem Muster von `NACHTRAG-4-1b.md`: „Die Datei wurde nicht angefasst“; drei Frames (Seiten „App: Texte, Technik, Dialoge“), Lage (nächste freie Spalte, x = Spalte × 1520, y = Reihe × 1000, vorher `get_page_tree`), Bauen (`bash ../build-screen.sh textprojekt`, `MODE` je Frame, `op.mjs render`, Vergleich per `export_image` mit den Screenshots, erzeugte `.jsx` nicht einchecken), danach `fix_fig_text.mjs --rgb555` und README-Satz; was nicht gezeichnet werden muss (Fortschritt des vollen Abgleichs, Fehlermeldungen, Anheben, Zurücknehmen, Hinweis beim Öffnen, Häkchen im Assistenten).
- [ ] **Step 4: Audit** wie unter Files; `bash scripts/check-tokens.sh` grün.
- [ ] **Step 5: Commit** `docs(design-system): addendum 4.3b text project, external changes, conflict and rename id` (mit Trailer).

### Task 14: Gate 4.3b

**Files:**
- Create: `docs/phase-4-3b-gate.md`
- Modify: `docs/ROADMAP.md` (Übersicht und Punkt 4.3b mit Stand), `docs/agent-gate.md` (Gate-Datei in der Liste oben, Abschnitt „Phase 4.3b“ mit Befehlen, `data-*` und dem Fokus-Trick im Browser), `docs/dev-setup.md` (gated Tests `tests/textproject_source.rs` und `project_start_text_project_builds_byte_equal` in der Tabelle `G4_TEST_PROJECT`/`G4_TEST_ROM`), `scripts/gate.sh` (nur falls die neuen gated Tests nicht schon über die vorhandenen Muster laufen)

**Interfaces:**
- Consumes: alles Vorige; die Ausgaben `store …`, `sync …`, `refs …`, `rename …` der gated Tests.

- [ ] **Step 1: Gate-Dokument** `docs/phase-4-3b-gate.md` nach dem Muster von `docs/phase-4-1b-gate.md`: Kopf mit Stand und dem Gate-Text der Roadmap als Zitat; Legende „Wer“ (Agent, Agent + Blick, Hand) mit Verweis auf `docs/agent-gate.md`; Namen in der Oberfläche (Technik › Textprojekt `[data-technik=textprojekt]`, `[data-sync-conflicts]`, `[data-conflict]`, `[data-sync-broken]`, `[data-entity-id]`, Dialogtitel, Meldungstexte); Vorbereitung (Kopien `_testkopien/4-3b-{pt,hgde,hgus}` für (1), `4-3b-k{pt,hgde,hgus}` für (2), `4-3b-web`, `4-3b-mcp`, `4-3b-ui` für (3), je mit `.nds` daneben; `cargo build --workspace --all-features` und Release-`g4k`; `G4_BASE_ROM_DIR=<Roms>/_testkopien`; Worktree-Hinweise; Last mit `uptime` zu jeder Zeit).
  - **(1) Automatik:** `scripts/gate.sh` grün; einzeln je Projekt im Release-Profil mit `--nocapture --test-threads 1`: `store_on_json_edits_only_the_entity_file_and_builds_like_dspre`, `simulated_dspre_change_is_adopted_one_sided`, `simulated_change_on_both_sides_is_a_conflict_and_both_choices_work`, `adopt_dspre_folder_builds_byte_equal_and_reverts`, `vanilla_checks_on_the_text_project_have_no_errors`, `recipes_with_ids_apply_on_the_vanilla_project`, `script_refs_on_the_vanilla_project_are_counted`, `rename_id_updates_every_reference_on_the_vanilla_project`, der gated Studio-Test `mcp_with_ids_acts_like_numbers_on_the_vanilla_copy` und mit `G4_TEST_ROM` `project_start_text_project_builds_byte_equal`.
  - **(2) Szenario kopflos** (je Spiel, Release-`g4k` und `studio mcp --project <Kopie> --base-rom <nds> [--allow-irreversible]`): `g4k base assign`; MCP `text_project_status` → `dspre`; `adopt_text_project` ohne Erlaubnis → `needs_confirmation`, mit → Bericht; `g4k build` → SHA-1 = `.nds`; `git init`, `g4k text sources` committen; MCP `data_edit` Pikachu `attack` +1 mit `"index": "SPECIES_PIKACHU"`, `save` → `git status --porcelain --untracked-files=no` nennt nur `data/species/0025.json`; `undo`, `save`, dasselbe mit `25` → gleicher Diff; ein Byte in `unpacked/moveData/0001` (wie DSPRE) → `g4k text sync` übernimmt (`dspre data/moves/0001.json`); JSON und Member von Trainer 1 verschieden ändern → `g4k text sync` Exit 1 mit `conflict data/trainers/0001.json`; MCP `external_changes {"diff": true}` zeigt den Diff; `resolve_external_conflict` `studio`, dann neu herbeiführen und `dspre` → beide wirken; `rename_id` eines Trainers → `git diff --stat` nennt Register, Trainer-JSON, ggf. `g4-checks.json`; `undo` → leer; `g4k build` → ROM; `uv run g4emu` startet die ROM, ein `shot` nach dem Intro (melonDS spielbar); `g4k text revert` → Manifest im Backup, `g4k build` → SHA-1 = `.nds`.
  - **(3) Browser-Teil** (Test-Umgebung, `4-3b-web`, Basis zugeordnet): Öffnen zeigt den Hinweis; „Einrichten…“ mit Prüfliste und Fortschritt; Pokédex › Pikachu zeigt `SPECIES_PIKACHU`, Angriff ändern, speichern → nur `data/species/0025.json` geändert (`git status` in der Kopie); der Agent schreibt per Shell ein Byte in einen Member („von DSPRE“), dann Fokus im Browser → Meldung „1 Änderung aus DSPRE übernommen“, Technik › Textprojekt „Zuletzt übernommen“ nennt ihn; beide Seiten ändern → Meldung „1 Konflikt …“, Diff aufklappen, „Studio-Fassung behalten“, dann erneut und „DSPRE-Fassung übernehmen“; ungespeicherte Bearbeitung plus Member von außen → Zeile „Ungespeichert“ mit Warnung; eine JSON mit Tippfehler → „Fehlerhafte Dateien“ mit Datei, Zeile, Spalte; „ID umbenennen…“ eines Trainers mit Plan, ⌘Z; „Hier spielen“ mit offenem Konflikt → nicht gebaut, Meldung; „Zurücknehmen…“; Assistent „Neues Spiel aus ROM“ mit Häkchen → Phase „Textprojekt einrichten“, danach Technik › Textprojekt „abgeglichen“; MCP über die Brücke (`4-3b-mcp`, Freigabe „Lesen und bearbeiten“): `mcp.sh call adopt_text_project '{}'` → Rückfrage „Als Textprojekt einrichten“, `mcp.sh call data_edit` mit `SPECIES_PIKACHU` ohne Rückfrage, `mcp.sh call rename_id …` mit dem Agenten im Verlauf.
  - **(4) Hand und Blick beim Nutzer:** DSPRE öffnet einen eingerichteten Ordner und speichert eine Änderung, die App übernimmt sie beim Fokus (Hand); Fokuswechsel im gepackten Tauri-Fenster (Hand); Blick auf Technik › Textprojekt (abgeglichen, Konflikt, fehlerhaft), Einrichten, Umbenennen in Hell, Dunkel, Kompakt (Screenshots aus Task 11 und 12).
  - **Ergebnis:** Tabelle Punkt / Wer / Ergebnis / Datum und die Zeittabelle:

    | Spiel | Umstellung (s, je Phase) | Build danach SHA-1 = Basis | Speichern einer Entität (ms) | Fokus ohne Änderung (ms, Dateien) | mit einer DSPRE-Änderung (ms) | voller Abgleich (s) | Umbenennen (ms) | Rücknahme (ms) | Last |
    |---|---|---|---|---|---|---|---|---|---|
    | Platin DE | | | | | | | | | |
    | HeartGold DE | | | | | | | | | |
    | HeartGold US | | | | | | | | | |

- [ ] **Step 2: Gate abarbeiten** (Agent: (1) bis (3)); Werte und Befunde eintragen; Befunde beheben (eigene Commits `fix(4.3b): …` mit Regressionstest), bis alles grün ist.
- [ ] **Step 3: Roadmap** `docs/ROADMAP.md`: Übersicht („4.3b Gate vom Agenten bestanden <Datum>, DSPRE-Hand und Blick beim Nutzer offen“) und am Ende des Punkts 4.3b „**Stand <Datum>:** Gate vom Agenten bestanden: Umstellung und Rücknahme byte-gleich auf allen drei Projekten, Store auf JSON (nur `data/species/0025.json` im Diff), DSPRE-Änderung beim Fokus übernommen, Konflikt mit beiden Wahlen, `SPECIES_PIKACHU` wie `25`, Fokus-Abgleich <…> ms, Speichern <…> ms; Branch `phase-4-3b-textprojekt-quelle`; [docs/phase-4-3b-gate.md](phase-4-3b-gate.md); beim Nutzer offen: DSPRE-Gegenprobe, Fokus im Tauri-Fenster, Blick“. `docs/agent-gate.md` und `docs/dev-setup.md` wie unter Files.
- [ ] **Step 4: Commit** `docs(4.3b): gate of the text project as source, roadmap status` (mit Trailer).

---

## Offen für die Umsetzung

- **rotom-Präprozessor:** die genaue Form von `#include` und `#define` (Dateiendung, Anführungszeichen, ob `include_roots` relativ zur Wurzel gilt, ob der LSP die Wurzel sieht) in rotom nachsehen (`src/compiler/` Präprozessor, `rotom.toml`-Leser); passt eine andere Endung oder Form besser, ändern sich nur `INCLUDE_FILE` und `include_text`. Kollidieren Namen mit Konstanten der Befehlsdatenbank (`flags`, `vars`, `sounds`), nennt der Test `include_defines_table_and_register_ids_sorted` sie, und die Include-Datei lässt sie weg.
- **`rotom.toml` bearbeiten:** ob `toml_edit` schon im Abhängigkeitsbaum ist (dann damit, Kommentare bleiben); sonst ergänzt `ensure_include_root` den Schlüssel textuell, wenn er fehlt, und lehnt ein vorhandenes, anders gesetztes `include_roots` nicht ab, sondern hängt `.g4/include` an.
- **Einzel-Export:** ob `EntitySchema::export` einzelner Entitäten Zustand braucht, den nur der volle Lauf aufbaut (Item-Tabelle, Trainertext-Tabellen, Taufe der Trainer aus Klasse und Name); `source::write` lädt dann diesen Zustand einmal je `EntitySource` und frischt ihn bei geänderten Registern auf.
- **Header-Tabelle und interne Namen:** der Store bearbeitet `mapname.bin` nicht (nur `create_header`); `source::write` der Header-Tabelle liest die internen Namen und Ortsnamen aus der vorhandenen JSON. Ändert ein Werkzeug `mapname.bin` am Store vorbei, ist das eine Member-Änderung und landet im Abgleich.
- **Zeiten:** liegt der Fokus-Abgleich auf HeartGold über 0,5 s (rund 20 000 `stat` im Bereich der Blobs), ist die erste Wahl, den Bereich ohne `unpacked/`-Ordner des Builds nur beim vollen Abgleich zu walken und beim Fokus nur Quellen, abgebildete Member und die nicht gepackten DSPRE-Ordner zu prüfen; das Gate nennt dann die Zahl der beobachteten Dateien.
- **Texte mit gleichem Inhalt auf beiden Seiten:** `TextSync` meldet `InSync` mit Neuaufzeichnung; ob der Abgleich das in „Zuletzt übernommen“ zeigen soll (Vorschlag: nein), entscheidet der Blick.
- **Scripts, die DSPRE in seinem eigenen Format (`expanded/scripts/*.txt` o. ä.) ändert:** nur der Member wird beobachtet; schreibt DSPRE Quellen, die rotom nicht liest, bleiben sie, wie sie sind. Was DSPRE beim Speichern genau schreibt, prüft die Hand (4).
- **Konflikt während eines Agenten-Aufrufs:** `external_sync` nimmt den Agenten-Zug; ein langer MCP-Aufruf verzögert den Fokus-Abgleich, bis er endet. Ob der Fokus-Abgleich dann ausgelassen statt nachgeholt werden soll, zeigt der Browser-Teil.
- **Löschen von Entitäten** (`textproject.member-without-json`) bleibt 3d (Referenzsperre); der Abgleich meldet einen von DSPRE gelöschten Member als fehlerhaft mit dem Satz des Imports.
- **Basisschicht im App-Cache** (4.3a „Platz der Basisschicht“): unverändert unter `.g4/cache/base-layer/`; ein Umzug in den App-Cache je SHA-1 ist nicht Teil von 4.3b.
- **4.4 danach:** die `.gitignore`-Vorlage muss `.g4/include/` und `.g4/backup/` (lokal) sowie die Ordner nennen, die der Hinweis H2 von 4.3a als Falle beschreibt (Sicherungen neben `data/` werden Blobs).
