# Phase 3b.4: Import und Prüfungen

Stand: 2026-10-23. Teil 4 von Phase 3b der [Roadmap](../../ROADMAP.md), zugleich Träger des 3b-Gesamtgates. Setzt auf 3c (Rahmen, Prüflicht, Panel Probleme, Technik › Prüfen und Bauen), 3b.2 (`ProblemsStore`, `text::validate`, rotom), 3b.3 (`Store::transaction`, RefIndex, Trainer-Asset, Begegnungen-Regeln) und 3a/3b.1 (Karten, Matrizen, Assistenten) auf. Recherche und die drei Designvorschläge liegen in [docs/research/2026-10-23-3b4/](../../research/2026-10-23-3b4/) (`import-report.md`, `checks-report.md`, `proposal-A/B/C.md`); Layouts, Zählungen und Fundstellen stehen dort.

## 1. Ziel und Umfang

Ein kleiner Hack (neue Route, zwei neue Trainer, neue Texte, ein Ereignis-Script, eine importierte Karte) entsteht komplett im Studio, die Prüfungen melden vorher die offenen Stellen, danach ist alles grün, und `g4emu` spielt ihn. Zwei Pläne, je mit Gate:

- **3b.4a „Prüfen und Beheben“**: Kernel-Modul `checks` (Volllauf ohne Lock, Text- und Script-Prüfungen je Datei gecacht), Katalog mit harter Regel (§4), Metadaten `check_limits.yaml` (216 px, Ausnahmen mit Quelle), Fixes als Plan → Vorschau → ein Changeset → Nachprüfung, Ausblenden in `g4-checks.json`, Prüflicht mit Fortschritt, Panel Probleme mit Gruppen, Öffnen, Beheben, Ausblenden, dieselbe Tabelle in Technik, Abschnitt „Prüfungen“ im Inspector, Fehlerfarbe an Warp-Markern. Gate A: gated `vanilla_checks` (Soll je Code und Projekt), rot → grün an vier absichtlichen Fehlern per Test-Umgebung.
- **3b.4b „Import und Gesamtgate“**: Import fertiger Binärteile in bestehende Karten (NSBMD, `.per`, `.bld`, `.bdhc`, `.bgs`, ganze `.bin` auch Pt ↔ HGSS, „Teile aus Karte N übernehmen“), Export der Gegenstücke, Dialog „Teile importieren…“ mit Prüfliste, `map.*`-Prüfungen, Szenarien `pt-kleiner-hack.yaml` und `hg-kleiner-hack.yaml`. Gate B = 3b-Gesamtgate (§9).

**Nicht in 3b.4** (Roadmap wird entsprechend korrigiert, §11): NSBMD-Writer, Baking aus `.pdsts`, OBJ/glTF-Import (apicula liest nur NSBMD, es gibt keinen quelloffenen OBJ → NSBMD-Weg außer DSPREs AGPL-Code), PNG → NSBTX, BDHC-Erzeugung, Kollisionsableitung, PDSMS-Ordnerimport über `.pdsmap` (keine echte Testdatei, zwei Formatvarianten; PDSMS-Exporte werden Karte für Karte über Mehrfachauswahl desselben Stamms importiert), alles Phase 6; Flags/Variablen-Ansicht und Story-Graph (3d); Löschen von Daten mit Referenzsperre (3d, die 3b.3-Spec hatte es 3b.4 zugeordnet); Build-Graph-Prüfungen (Phase 4); `script.rotom-all`, Flag- und Variablenbereiche, Script-Parameter-Typen (später, §4).

## 2. Fakten

Geprüft an den drei Testprojekten, sofern nicht „offen“ (Fundstellen in den Berichten).

- **Kartenmodelle:** Pt DE 666, HG DE und HG US je 676 Karten; alle Modelle haben einen Block, Kopf `BMD0 FFFE 0200`, 4-Byte-ausgerichtet; größtes Modell 61 420 B (HGSS), sechs HGSS-Modelle über 61 000 B; pokeplatinum verlangt Modell ≤ 0xF000, BDHC ≤ 0x9000 und genau ein Modell im MDL0 (`GF_ASSERT numEntry == 1`); HGSS-Puffergrößen nur indirekt belegt (offen). Alle BDHC tragen die Magic `BDHC`. `g4-formats::map` tauscht das Modell byte-sicher (`model` ersetzen, `validate`, `write`; `buildings_tail` und BGS bleiben).
- **DSPRE-Import:** `main` ersetzt nur das Modell, prüft Magic, entfernt TEX0 (MDL0 fest an 0x18), warnt ab 61 000 B; avalonia Limit 0xF000. `.per` (avalonia `.mp`) = 2048 B wie der Permissions-Abschnitt, DSPRE prüft die Länge nicht. Kein Bildimport für Kollision.
- **PDSMS:** `.pdsmap` ist Text (Raster 32 × 32, `tileset`, Position, `areaindex`; Fork mit 9 Ebenen), Kollision/Gebäude/BDHC/BGS als `<stem>_XX_YY.per/.bld/.bdhc/.bgs` daneben; NSBMD entsteht über Nintendos `g3dcvtr.exe`; keine Lizenz im Repo.
- **Prüfungen auf Vanilla Pt DE** (Python-Messung, `checks-report.md`): Referenzprüfungen (Header-Indizes, Warp-Ziele, Event-Script außerhalb der Datei, Trainer-Script ohne Trainer, `Message N` über dem Archivende: 0 von 4410, Matrizen, Entwicklungen, Lernsets) treffen 0; Textbreite 200 px (chatot `DIALOG_LINE_MAX_PX`) trifft 1810 Dialognachrichten (HG DE 1891, HG US 1157), über 216 px nur 90/114/113; „Warp auf Nicht-Tür-Kachel“ 57 Treffer, davon 47 reine Ankunfts-Warps; „Warp ohne Rückweg“ 118 (Union Room, Arenen, Aufzüge). Sechs Vanilla-Warps zielen auf 0x0FFF (dynamisch, offen), Trainer 0 ist ein Dummy ohne Party, Eventdatei 0 teilen 60 Dummy-Header.
- **rotom** prüft beim Compile Syntax, Befehle, Argumentzahl und Bitbreite (keine Wertebereiche), Sprungziele, Symbole, Menüregeln, `MissingSlot`, `MessageLineTooLong` (200 px, nur Textliterale), Level-Script-Struktur; rotom-lsp zeigt dasselbe plus `Module::Label` für offene Dokumente. Nie: fehlende Nachricht, Flag/Variable außerhalb des Bereichs, fehlender Script-Slot aus Event oder Level-Script, nicht existierende Trainer-, Item- oder Header-ID.
- **Kosten:** alle Nicht-Text-Prüfungen 0,4 s in Python (warm), Schätzung 20–80 ms in Rust; Dekodieren der 724 Textarchive 0,8–0,95 s warm, 5 s kalt (Rust release); Breitenmessung vernachlässigbar.
- **Codebasis:** `Store::edit_map` macht jeden Import in eine bestehende Karte zu einem normalen Undo-Schritt; der Brücken-Batch schreibt je Befehl eine JSON-Zeile (`bridge.jsonl`); `AppState` hält den Store hinter einem Mutex; `Store::text_index()` hält heute Kleinbuchstaben.

## 3. Entscheidungen im Überblick

| Frage | Entscheidung | Grund |
|---|---|---|
| Schnitt | 3b.4a Prüfen und Beheben, 3b.4b Import und Gesamtgate | Prüfungen brauchen keinen Import; das Gesamtgate braucht beides |
| Import | fertige Binärteile in bestehende Karten über `edit_map`, rückgängig, ohne Plakette und Schnappschuss | es entsteht nichts Unumkehrbares; neue Karten kommen weiter aus „Gebiet anlegen“ |
| Import-Oberfläche | `Dialog` „Teile importieren…“ im Inspector der Karte mit Prüfliste, kein Assistent, keine eigene Vorschau | ein Schritt; die Szene ist die Vorschau, ⌘Z der Rückweg |
| PDSMS | Mehrfachauswahl desselben Stamms, `.pdsmap` wird nicht gelesen | keine echte Testdatei, nicht beweisbar |
| Prüfkatalog | Fehler nur bei 0 Treffern auf allen drei Vanilla-Projekten (gated bewiesen), ~19 Fehler, ~14 Warnungen, 2 Hinweise | Vertrauen durch Rauschfreiheit |
| Textbreite | 216 px aus `check_limits.yaml`, rotoms 200-px-Warnung wird verworfen, nur Dialogarchive | 1810 falsche Treffer bei 200 px |
| Ausnahmen | Regeln mit Quelle (0x0FFF, Trainer 0, Eventdatei 0, Script 0, Begegnungs-Ende) | kein verstecktes Rauschen |
| Architektur | Snapshot unter dem Lock, Volllauf ohne Lock (300 ms Debounce), Text und Scripts je Datei gecacht; keine Inkrementalität | 20–80 ms reichen; `inkrementell == voll` per Konstruktion |
| Beheben | fünf deterministische Fixes, Vorschau → ein Changeset → Nachprüfung, kein „Alle beheben“ | eindeutige Ergebnisse, nachprüfbar |
| Ausblenden | Fehler nie; Warnungen und Hinweise in `g4-checks.json` mit Fingerprint; Sammel-Ignorieren als Ausgangslage | Vanilla-Hinweise ohne Regelverbiegung |
| Gate-Beweis | Befundzahlen je Code vorher/nachher im Brücken-Batch, Modell im ROM per Bytevergleich, Weg im Spiel, den nur die importierte Kollision freigibt | wird automatisch rot |

## 4. Prüfkatalog (3b.4a, Codes wie `checks-report.md`)

**Harte Regel:** Eine Prüfung ist nur „Fehler“, wenn sie auf Platin DE, HeartGold DE und HeartGold US je 0 Treffer hat. Das beweist der gated Test `vanilla_checks` (Soll je Code und Projekt fest im Test); eine geplante Fehler-Prüfung mit Vanilla-Treffern wird vor dem Ausliefern Warnung oder ihre Regel wird geschärft.

| Schwere | Prüfungen |
|---|---|
| **Fehler** | `warp.dest-header`, `warp.dest-warp`, `warp.off-map`, `event.script-missing` (Figuren, Trigger), `event.trainer-missing`, `event.off-map`, `header.ref-range`, `header.wild-dangling`, `header.script-kind`, `script.message-missing`, `trainer.no-party` (von einem Event genutzt), `trainer.empty-species`, `trainer.id-range`, `matrix.map-range`, `matrix.header-range`, `map.model-invalid` (dieselbe Prüfung wie beim Import), `text.encode`, `text.json-broken`, `asset.unreadable` |
| **Warnung** | `event.script-missing` für Schilder (1 Vanilla-Treffer, Task 0), `header.level-target` (1 Treffer, Header 186, Task 0), `event.trainer-blind`, `event.trigger-empty`, `event.same-spot`, `header.text-split`, `trainer.no-party` (ungenutzt), `trainer.no-moves`, `trainer.double-one`, `text.too-wide`, `text.three-lines`, `map.bdhc-size` (HGSS), `encounter.empty-slot`, `encounter.level-zero` (Regeln aus 3b.3 §10, projektweit) |
| **Hinweis** | `warp.never-fires` (10 Vanilla-Treffer), `encounter.min-max` |
| **bedingt** (nur bei 0 Treffern in Task 0) | `map.building-range`, `area.tileset-range`, `matrix.header-elsewhere` |
| **später** | `script.rotom-all`, `script.trainer-target`, `script.id-range`, `script.flag-range`, `script.warp-target`, `event.common-missing`, `event.*-range`, `header.level-message`, `header.no-map`, `data.*`, `trainer.name-length`, `trainer.texts`, `trainer.flag-capacity` (die 3b.3-Editoren verhindern sie), `build.*`, Texturnamen projektweit |
| **nie** | `warp.one-way` (118 Treffer; nur als Inspector-Zeile „Rückweg: …“), doppelte `local_id`, `trainer_type` ohne Trainer-Script, Figur auf blockierter Kachel, `*.unused` |

Jede Prüfung hat einen stabilen Code, eine deutsche Meldung mit Ausweg („Die Tür führt ins Leere. Wähle ein Ziel oder lösche die Tür.“), ein Navigationsziel und ggf. Fixes (§6). Common-Scripts ohne Header werden als `skipped` gemeldet, nie als „ok“.

**Metadaten `crates/g4-kernel/metadata/check_limits.yaml`** (Werte mit `source` und `verified`): `dialog_line_px: 216` (Plan-Task 0 weist das Feld-Textfenster in pokeplatinum/pokeheartgold nach, Kandidat `Window_Add(…, 27, 4, …)`, 27 × 8 = 216; ohne Beleg bleibt 216 mit `verified: false` und dem Histogramm als Quelle), `dialog_lines_max: 2`, `warp.dynamic_dest: 0x0FFF`, `trainer.dummy: 0`, `event_file.shared: 0`, `script.decoration: 0`, `encounter.none: { pt: 0xFFFF, hgss: 0xFF }`, `model.max_bytes: 0xF000`, `bdhc.max_bytes: 0x9000`, `permissions.bytes: 2048`. `text::validate` liest die Breite über `Project::limits()` statt chatots Konstante; die Breitenprüfung gilt nur für Dialogarchive (von einem Header als `text_archive_id` genutzt); rotoms `MessageLineTooLong` wird beim Umwandeln über die Warnungsart verworfen (macht rotom sie nicht öffentlich: über die Spanne dedupliziert).

## 5. Architektur `checks` (`crates/g4-kernel/src/checks/`)

```
mod.rs      Severity (+Info), Subject, Finding, Check, Registry, run_all, Snapshot
limits.rs   check_limits.yaml
world.rs    warp.*, event.*, header.*, matrix.*, map.*
scripts.rs  script.message-missing (rotom-Parser; Nachrichtenzahl des Archivs, das der Header der Scriptdatei nutzt)
text.rs     text.* (Text-Index mit Originaltext)
data.rs     trainer.*, encounter.* (3b.3-Codecs, encounter_rules)
ignore.rs   g4-checks.json, Fingerprint
```

```rust
pub enum Severity { Error, Warning, Info }   // text::Severity bekommt Info, serde "info"
pub enum Subject { Header(u16), EventFile(u16), ScriptFile(u16), TextArchive(u16), Trainer(u16), Encounter(u16), Matrix(u16), Map(u16), Project }
pub struct Finding { code: &'static str, severity: Severity, subject: Subject, key: String, message: String,
                     target: NavTargetDto, also: Vec<NavTargetDto>, fixes: Vec<FixDto>, file: Option<(PathBuf, u32, u32)>, fingerprint: u64 }
pub trait Check: Sync { fn code(&self) -> &'static str; fn group(&self) -> Group /* Cheap | PerScript | PerArchive */; fn run(&self, snap: &Snapshot, out: &mut Vec<Finding>); }
```

**Snapshot statt Lock:** Unter dem Store-Lock wird nur kopiert, was billig ist (Header-Tabelle, Member-Zahlen, Eventdateien, Matrizen, Trainer, Begegnungen, ungespeicherte Text-Dokumente); geprüft wird ohne Lock auf einem Arbeitsthread (`spawn_blocking`); hat sich die Store-Generation geändert, wird das Ergebnis verworfen und neu angestoßen. Der billige Teil läuft nach jeder Store-Änderung mit 300 ms Debounce vollständig (Budget: unter 100 ms auf HGSS, Task 0 misst); Text- und Script-Prüfungen laufen je Datei und sind über Generation bzw. Quell-Hash gecacht. `Store::text_index()` behält den Originaltext. Ein Volllauf „Erneut prüfen“ lässt sich von der Oberfläche und der Brücke anstoßen; sein Fortschritt („Prüft … Texte 312/724“) kommt über `checks_status` (abgefragt, nicht gepusht, damit die Brücke ihn kennt). Prüfungen blockieren nie das Bauen. Sollte der Volllauf auf großen Hacks zu langsam sein, bekommt der Runner Subjekt-Invalidierung; die Schnittstelle ist schon nach Subjekten geschnitten.

**Vertrag Kernel ↔ Oberfläche:** `ProblemDto` bekommt optionale Felder (`code`, `subject: {kind, index}`, `key`, `label`, `target`, `also`, `fixes: {id, label, irreversible}[]`, `fingerprint`), die Schwere `info` und die Quelle `check`; `ChecksReport { findings, ignored, ranMs, generation, running, progress?, skipped: {code, why}[] }`. `ProblemsStore` bekommt die Quelle `check`, die je Lauf als Ganzes ersetzt wird; `countsText` lernt „1 Hinweis“ / „3 Hinweise“. `NavTargetDto` spiegelt `navigation.ts` plus `trainer` und `encounter`.

## 6. Beheben

Ein Fix nur, wenn das Ergebnis eindeutig ist oder genau eine Auswahl braucht: `warp.dest-*` → Ziel bzw. Ziel-Warp wählen (Picker); `event.script-missing` → Script #n anlegen (`add_script`, nicht rückgängig, Plakette); `script.message-missing` → Nachricht anlegen (`Append`); `text.too-wide` → Umbrechen (chatot `word_wrap` mit 216, Vorschau als `DSScreen`); `event.trainer-missing` → Trainer wählen (die Prüfung hängt am Script 3000–6999, nicht am `trainer_type`: der Ausweg ist ein vorhandener Trainer oder das alte Script); `encounter.min-max` → Tauschen. Ablauf: `plan_fix` liefert die Änderungssätze (Vorher/Nachher), der Dialog zeigt sie und die Plakette, `apply_fix` läuft als ein Changeset (`Store::transaction`) mit Fingerprint-Schutz (verändertes Subjekt → „Befund ist nicht mehr aktuell“), danach Nachprüfung des Subjekts; bleibt der Befund, heißt es „Nicht behoben“. Kein „Alle beheben“. Tests sichern je Fix: der Befund verschwindet, ⌘Z bringt ihn zurück (außer bei Anlagen).

## 7. Ausblenden und Ausgangslage

Fehler lassen sich nie ausblenden. Warnungen und Hinweise: `g4-checks.json` im Projekt (`ignored: [{code, subject, key, fingerprint}]`, `disabled: [code]`); ändert sich das Subjekt, passt der Fingerprint nicht mehr und der Befund erscheint wieder. „Ausgangslage übernehmen“ ist ein Sammel-Ignorieren aller aktuellen Warnungen und Hinweise mit demselben Mechanismus (Dialog nennt die Zahl). Das Panel zeigt „12 ausgeblendet“ mit „Wieder melden“. Die Sollzahlen je Code auf Vanilla stehen fest im gated Test, nicht in einer mitgelieferten Datei.

## 8. Oberfläche (3b.4a)

- **Prüflicht** im Scharnier: drei Zähler (Fehler, Warnungen, Hinweise), Spinner mit Fortschritt während eines Volllaufs, Klick öffnet das Panel.
- **Panel Probleme**: Gruppen nach Bereich (Welt, Scripts, Texte, Pokémon) und Code mit Zähler, Zeilen mit Meldung und Subjekt-Label („Eventdatei 0411 · Warp 2“), Tasten Öffnen (`goTo`), Beheben (bei Fixes), Ausblenden (nicht bei Fehlern); Fuß „12 ausgeblendet · Wieder melden“, Taste „Erneut prüfen“; `skipped` als Hinweiszeile („Common-Scripts ohne Header nicht geprüft“).
- **Technik › Prüfen und Bauen**: dieselbe Tabelle; „Ausgangslage übernehmen“; Schalter je Code (`disabled`).
- **Am Objekt**: Inspector-Abschnitt „Prüfungen“ mit den Befunden des gewählten Subjekts; Fehlerfarbe (`status/fehler`) am Warp-Marker in der Szene; Warnsymbol in Explorer- und Datenlisten (`ListRow.trailing`).
- Neue Primitives: `BeforeAfter` (zwei Spalten mit Diff-Hervorhebung für den Fix-Dialog), `CheckList` (Zeilen mit ok/Warnung/Fehler und Satz; auch für den Import-Dialog).

## 9. Import (3b.4b)

| Teil | Zusage | Prüfung vor dem Schreiben (Ablehnung ändert nichts) |
|---|---|---|
| NSBMD → Modell | ja | Magic `BMD0`, BOM `FFFE`, Version `0x0002`, Größenfeld = Länge, Kopf 16, 1–2 Blöcke mit gültigen Offsets; TEX0 entfernen, MDL0 über Blockoffsets suchen; genau ein Modell; ≤ 0xF000 Fehler; Länge % 4 ≠ 0 Warnung |
| Texturnamen gegen Area-NSBTX | bedingt | nur, wenn Task 0 zeigt, dass der Material-Leser auf allen Vanilla-Karten keine fehlenden Namen meldet (gated); sonst Hinweis „nicht geprüft“ |
| `.per` / `.mp` | ja | genau 2048 B |
| `.bld` | ja | Länge n × 48, `buildings_tail`-Regel, `validate` |
| `.bdhc` / `.bdhcam` | ja | Magic `BDHC`, ≤ 0x9000 (HGSS als Warnung, offen), Zähler passen zur Länge; DP abgelehnt |
| `.bgs` | nur HGSS | Signatur `0x1234`, `validate`; auf Platin abgelehnt |
| ganze Map-`.bin` | ja | `MapFile::read` mit eigener, sonst anderer Familie; Pt → HGSS legt `BLANK_BGS` an; HGSS → Pt lässt BGS weg (Warnung, wenn nicht leer); fremde Familie: Gebäude standardmäßig nicht übernehmen; DP abgelehnt |
| „Teile aus Karte N übernehmen“ | ja | wie Datei, Quelle ist ein Store-Asset (auch ungespeichert) |
| Export | ja | `.nsbmd` (MDL0 ohne TEX0, wie in der Karte), `.per`, `.bld`, `.bdhc`, `.bgs`, `.bin` |

**Oberfläche:** Taste „Teile importieren…“ im Inspector der Karte (Abschnitt „Karte“) und in der Palette; `Dialog` mit `Segmented` Datei | Karte im Projekt, Mehrfachauswahl von Dateien desselben Stamms (Endung bestimmt den Teil), je erkanntem Teil eine `CheckList`-Zeile mit `Checkbox` und Größe (Teile mit Fehler nicht ankreuzbar), Hinweis „BDHC bleibt unverändert, Laufhöhen prüfen“ bei Modell ohne BDHC, Taste „3 Teile übernehmen“; danach zeigt die Szene das neue Modell (Modell-Cache hängt am Hash), der Dialog nennt die neuen Befunde des Subjekts mit „Zeigen“. Export über „Teile exportieren…“ daneben. `Roots` der Brücke bekommt `read_path`/`write_path` für Import- und Exportpfade. Prüfungen `map.model-invalid`, `map.bdhc-size`, bedingt `map.building-range`.

## 10. Befehle (Tauri und Brücke)

3b.4a: `run_checks { scope? }` → `ChecksReport`, `checks_status {}`, `plan_fix { code, subject, key, fix, choice? }` → Änderungssätze, `apply_fix { … }` → `Vec<AssetId>`, `ignore_finding { code, subject, key, fingerprint }`, `unignore_finding { … }`, `accept_baseline {}`, `set_check_enabled { code, enabled }`. 3b.4b: `inspect_map_import { map, source }` (Probelauf, ändert nichts), `import_map_parts { map, source: {files: path[]} | {map: n}, parts, keepBuildings? }`, `export_map_part { map, part, path }`. Jeder Befehl in `COMMANDS`, `dispatch.rs`, Handler, `api.ts`. `g4k check` druckt den `ChecksReport` als JSON (für Szenarien und das Gate), `g4k rom-map-part <rom> <map> <part> --equals-file <datei>` vergleicht einen Kartenteil im gebauten ROM.

## 11. Gate und Tests

- **Gate A (3b.4a):** `vanilla_checks` gated grün (Soll je Code und Projekt, Fehler = 0 überall); Szenario-Teil (Brücke, ohne Spiel): auf einer Kopie vier absichtliche Fehler anlegen (Warp ohne Ziel-Warp, Figur mit fehlendem Script, Trainer-Event ohne Trainer, zu lange Zeile) → `run_checks` zählt je Code 1 → Fixes bzw. Edits → 0; Panel, Prüflicht, Fix-Dialog, „12 ausgeblendet / Wieder melden“ im Browser-Modus über Rollen; `scripts/gate.sh` grün.
- **Gate B (3b.4b, 3b-Gesamtgate):** Szenario `pt-kleiner-hack.yaml` (und `hg-kleiner-hack.yaml`), Fortsetzung von `pt-trainer-begegnung.yaml`; `g4emu` bekommt an Edit-Einträgen ein drittes Element `{expect: {path, equals}}`, das gegen die passende Zeile aus `bridge.jsonl` geprüft wird (Exit 2 mit Pfad, Soll, Ist). Ablauf: (1) `accept_baseline`, `expect run_checks: error 0, warning 0`; (2) Patch, `create_area` mit leerer Eventdatei, Treppe umgebogen → rot `warp.dest-warp == 1` → Ankunfts-Warp → grün; (3) `export_map_part` Modell und `.per` einer Karte SRC desselben Gebiets mit einem Weg, der in der Vorlage blockiert ist (Task 0 wählt SRC), `import_map_parts` in die neue Karte; Gegenfall: synthetisch > 0xF000 scheitert, `dirty` unverändert; (4) `create_trainer` × 2 → rot `trainer.no-party == 2` → Party → grün; (5) Texte: geändert, angefügt mit zu langer Zeile → rot → `apply_fix wrap` → grün; (6) Figur mit fehlendem Script → rot → `apply_fix create_script`, `save_script` mit Textliteral, Item per Flag, anderer Text danach → grün; (7) `run_checks` 0/0, `save`, `build_rom`; (8) `verify: [rom-map-part, …]` Modell im ROM = exportierte Datei; (9) Spielen: Treppe → `expect-map NEU` → `walk-to` über den Weg, den nur die importierte Kollision freigibt → `shot` → Figur ansprechen → beide Trainer mit `walk-until-battle`, `expect-battle`, `win-battle`. Agent + Blick: Route texturiert; Hand: DSPRE öffnet die Kopie nach dem Import, nativer Dateidialog in der gebündelten App.
- **Tests:** je Prüfung ein Unit-Test mit synthetischer Fixture (Treffer und Nicht-Treffer); Snapshot-Test (Kopie unter dem Lock misst < 10 ms auf HGSS); Fix-Tests (Befund weg, Undo zurück); `ignore` mit Fingerprint; Import: gated „Modell durch sich selbst = Original-Bytes“ für jede Vanilla-Karte, A in B und zurück, HGSS ↔ Pt-`.bin`, `.per`-Rundweg, Ablehnung > 0xF000 und zwei Modelle (synthetisch); `docs/phase-3b4-gate.md`.

## 12. Risiken und Festlegungen

1. **Fehlalarme** zerstören Vertrauen: harte Regel mit gated Beweis, Codes stabil, Meldungen mit Ausweg.
2. **Lock und Latenz** (5 s kaltes Textdekodieren): Snapshot unter dem Lock, Arbeit ohne Lock, Fortschritt sichtbar, Text je Datei gecacht.
3. **Importiertes Modell hält das Spiel an** (Assert): strenge Prüfungen vor dem Schreiben, Undo, Bytevergleich im ROM, Spielen im Gate; HGSS-Puffergrößen als Task 0.
4. **216 px oder Fix-Semantik falsch**: `verified`-Flag mit Quelle, Histogramm als Rückfall, Fixes mit Vorschau und Undo.
5. **Umfangsdrift** (PDSMS, Baking, Story-Graph, Löschen): die Liste in §1 gilt wörtlich.
6. **Abhängigkeit von Ungebautem**: 3b.4a startet nach dem Merge von 3b.3b (Trainer-Asset, RefIndex, Begegnungen-Regeln); fehlt etwas, sagt der Plan es und zieht die Prüfung ins „später“.

## 13. Offene Prüfungen (Plan-Task 0)

216 px in pret (`Window_Add` des Feld-Textfensters); Bedeutung von 0x0FFF; HGSS-Puffergrößen für Modell und BDHC; Vanilla-Zählungen auf HGSS für alle Fehler-Kandidaten; Lock-Dauer des Text-Index-Aufbaus und Snapshot-Kosten; ob rotom `MessageLineTooLong` als Warnungsart öffentlich macht; Material-Leser (Texturnamen) auf Vanilla; Karte SRC mit eindeutig neuem Weg für das Gate; ob `XX_YY` in PDSMS-Dateinamen absolut ist (nur für die Doku); ob ein Trainer ohne Party wirklich abstürzt (für die Schwere von `trainer.no-party`).

## Stand nach der Umsetzung (3b.4a und 3b.4b)

Entscheidungen der Umsetzung, die den Text oben ändern (Rulings im Ledger des Plans 3b.4a, `.superpowers/sdd/2026-10-24-phase-3b4a-checks/progress.md`):

- R1 `run_checks { scope? }`: `"all"` (Standard) oder `"cheap"`; `"full"` wird als `"all"` angenommen (die Entwürfe von 3b.4b schreiben es so).
- R2 `plan_fix` und `apply_fix` nehmen `{ finding, fix }`: den Befund, wie `run_checks` ihn lieferte (gelesen werden `code`, `subject`, `key`, `fingerprint`), und `fix` als snake_case-ID (`wrap`, `append_message`, `swap_min_max`, `create_script`); `apply_fix` antwortet `{ changed, remaining }` (§10).
- R3 `NavTargetDto` spiegelt `navigation.ts` genau (`header`, `matrixCell`, `area`, `building`, `event`, `textArchive`, `script`, `species`, `move`, `item`, `trainer`, `encounter`); ein Warp ist `event` mit `eventKind: "warp"`; `target` ist optional, ohne Ziel kein „Öffnen“ (§5).
- R8 Ein TypeScript-Typ je Form: `ProblemDto` trägt die Felder des Befunds optional, `CheckProblemDto` ist `ProblemDto & CheckFieldsDto` (§5).
- R9 `create_script` legt genau Script #n an (`add_script_at_reported`), nicht den nächsten freien Platz (§6).
- R10 „Verändertes Subjekt“ heißt: der Fingerprint des Befunds (die geprüften Bytes) hat sich geändert; `wrap` lehnt zusätzlich ab, wenn Store-Text und Snapshot nicht übereinstimmen (§6).
- R11 Die Vorschau von „Umbrechen“ ist `BeforeAfter` mit Textzeilen, nicht `DSScreen` (§6, §8); die Zeilen brechen nicht um, die Spalten rollen seitlich.
- R12 Der automatische Lauf nach einer Änderung ist ein voller Lauf (alle Gruppen, 300 ms nach der letzten), kein billiger (§5); die Caches je Datei machen ihn warm schnell (250 bis 290 ms auf den drei Projekten). Nach dem Öffnen und nach einem Fix läuft genau ein Lauf.
- R13 `expect` in Szenarien: Pfade aus Schlüsseln mit Punkt, Listenindizes, `.length` und Filtern `[schlüssel=wert]` (Vergleich als Text); ein Filter auf einen Schlüssel, den kein Eintrag einer nicht leeren Liste hat, ist ein fehlender Pfad (§11).
- R15 Eine kaputte `g4-checks.json` hält das Projekt nicht zu: es öffnet ohne Ausblendungen, der Bericht nennt die Datei und den Parse-Fehler, Schreiben in die Datei wird abgelehnt, bis sie repariert oder gelöscht ist (§7).
- R16 `matrix.header-elsewhere` ist Fehler (Task 0 maß 0/0/0 mit der Ausnahme Header-Zelle 0); ebenso die anderen „bedingten“ `map.building-range` und `area.tileset-range` (§4). Dazu kamen im Katalog `script.unreadable`, `encounter.level-high` und `encounter.rate-byte`; `map.bdhc-size` ist Fehler (Grenze 0x9000 ab Byte 16).

Aus der Schlussprüfung und der Abnahme von Gate A: ein Bericht gilt nur, solange Store-Generation und die Epoche der Schreibzugriffe am Store vorbei (Scripts, neue Trainer) seine sind; ohne Bericht sagen Prüflicht und Panel „Nicht geprüft“, nie „Alles stimmig“; der Platzhalter-Trainer des „gesehen“-Slots (`trainer.sight_slot`, Platin 929, HGSS 740) ist weder `asset.unreadable` noch ein Trainer-Befund.

Aus der Umsetzung von 3b.4b (Regelungen im Ledger des Plans 3b.4b, `.superpowers/sdd/2026-10-24-phase-3b4b-import/progress.md`):

- S1 Szenarien kennen den Platzhalter `${sha1-of:PFAD}` (SHA-1 einer Datei, gelesen erst beim Einsetzen, in `check:` nach dem Bauen). Damit vergleicht `g4k check-data --expect map:N:model_sha1=… --expect map:N:per_sha1=…` den Kartenteil im ROM mit dem Export; das ersetzt `verify: [rom-map-part, …]` (§11 (8)) und `g4k rom-map-part` (§10, Entscheidung des Plans).
- S2 Eine Änderung darf abgelehnt werden müssen: `{expect: {error: Teilstring}}` als drittes Element (§11 (3), Gegenfall). Der Lauf speichert vorher, was die Änderungen davor taten; die Ablehnung beendet die Brücken-Sitzung ohne Speichern.
- S3 Jede `run_checks`-Erwartung der Szenarien prüft auch `groups.length = 3`: ein Bericht ohne alle Gruppen ist kein Ergebnis, nie „sauber“.
- S4 Das synthetische Modell über 0xF000 entsteht im Abschnitt `prepare:` eines Szenarios (`make-oversize-nsbmd`, `scripts/g4emu/g4emu/fixtures.py`): Kopf und ein MDL0-Block mit genau einem Modell sind gültig, nur die Größe lehnt ab.
- S5 `map:N:per_sha1` vergleicht mit der Kollision, die das Studio nach dem Setzen der Tür exportiert (die Tür kommt nach dem Import, der die ganze Kollision ersetzt). Die Ablehnung belegen die Meldung („61440“) und `inspect_map_import` mit `state.kind = error` und dem Code `model.too-big`. „`dirty` unverändert“ (§11 (3)) prüft kein Szenario: nach einer Ablehnung öffnet die nächste Sitzung das Projekt neu von der Platte, `dirty` ist dort immer leer, und der Hash im ROM zeigt nur den gelungenen Import. Dass eine Ablehnung nichts schreibt, belegen die Tests vor dem Schreiben (§11 Tests: `import_refuses_on_error_and_changes_nothing` im Kernel, `export_then_import_from_files_and_a_refusal_changes_nothing` im Studio).
- Teilprüfungen (§9) tragen einen Code (`PartCode`, 29 Codes) mit Argumenten; der Dialog sagt jeden deutsch mit Ausweg (`importParts.ts`), Brücke und `call.sh` behalten den englischen Satz des Kernels. Nach dem Import nennt der Dialog alle Befunde der Karte aus dem nächsten vollen Lauf (R12), nicht nur die neuen (§9). Der Picker der Quellkarte zeigt keine Kartenbilder.
- Szenarien, abweichend von §11: Wege und Figuren liegen auf gewöhnlichem Boden statt im hohen Gras (Rate 100), in HeartGold liegt die Ankunft auf (8, 25) statt am Weganfang; Patch und `adopt_scripts` kommen vor `accept_baseline`; der neue Header bekommt ein eigenes Textarchiv; die Begegnungsdatei füllt jede Ersatzliste, die das Spiel im Gras liest (sonst `encounter.empty-slot`); „Item per Flag, anderer Text danach“ ist die Figur, deren Script `create_script` anlegte; die Trainer kämpfen per Sichtlinie bzw. nach Ansprechen statt mit `walk-until-battle` (§11 (9)). Dass der Weg nur mit der importierten Kollision geht, belegen die Gegenproben `pt-kleiner-hack-ohne-import.yaml` und `hg-kleiner-hack-ohne-import.yaml` (sie scheitern am Weg); Text, Flags und Texturen, die `g4emu` nicht liest, belegen Bilder für den Blick des Nutzers („Agent + Blick“). Ablauf und Ergebnis: [docs/phase-3b4-gate.md](../../phase-3b4-gate.md), Teil B.

## 14. Dokumentation und Roadmap

- `docs/ROADMAP.md`: 3b.4 mit Schnitt a/b und dem korrigierten Lieferumfang (kein OBJ über apicula, kein PDSMS-Baking; Verweis auf Phase 6), Flags/Variablen und Story-Graph nach 3d, Löschen von Daten nach 3d; das 3b-Gesamtgate um „importierte Karte“ und „Prüfungen vorher rot, nachher grün“ präzisiert.
- `docs/formats/`: `nsbmd_header.md` (Kopf, Blöcke, MDL0-Dictionary, TEX0-Entfernung), `check_limits.md`.
- `docs/dev-setup.md`: `vanilla_checks`, Import-Tests, `g4k check`, `g4k rom-map-part`; `docs/agent-gate.md`: Rezepte Prüfen, Beheben, Import; `design-system/README.md`: „Was die App umsetzt“ + 3b.4.
