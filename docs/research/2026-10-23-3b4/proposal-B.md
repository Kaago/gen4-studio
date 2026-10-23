# Vorschlag B: Phase 3b.4 „Import und Prüfungen“
Architekt B, Neigung Nutzererlebnis: Befunde stehen am Objekt, Beheben geht mit einem Klick, Import ist ein geführter Ablauf, und Einsteiger arbeiten ohne Zahlen. Grundlage: `import-report.md`, `checks-report.md`, Specs 3c, 3b.3 und 3b.2, ROADMAP im Worktree `phase-3c1`. Gelesener Code: `map.rs`, `store.rs` (`edit_map`, `duplicate_map_with`, `history`), `text.rs` (`validate`, `DIALOG_LINE_MAX_PX`), `problems.ts`, `commands/problems.rs`, `commands/models.rs` (`glb_with_fallback`), `g4emu/scenario.py`.

**Leitsatz.** Jede Meldung nennt das Ding so, wie man es sieht, also „Die Tür im Haus des Rivalen“ und nicht „Eventdatei 0390 · Warp 2“. Sie sagt, warum das stört, und bietet genau eine naheliegende Handlung an. Zahlen stehen nur in Tüfteln.

**Schnitt in zwei Pläne.** Ein Plan wäre zu groß für eine Review-Runde.
- **3b.4a „Prüfen und Beheben“:** Kernel `checks`, RefIndex-Erweiterung, Oberfläche, Fixes, Ausgangslage.
- **3b.4b „Import und Gesamtgate“:** NSBMD, Teile, ganze Karte, Export, PDSMS-Ordner (als letzter Task, kann gestrichen werden), Szenario für das 3b-Gate.

Reihenfolge wie im Gedächtnis: 3b.3a → 3b.3b → 3b.4a → 3b.4b.

**Fund außerhalb des Briefs.** Die ROADMAP (Abschnitt „Aus dem Design-System, später“, Zeile 201) ordnet „Flags und Variablen sowie Geschichten und Story-Graph“ 3b.4 zu. Spec 3b.3 schiebt außerdem „Löschen von Daten“ nach 3b.4. Beides passt nicht in 3b.4 (siehe Frage 8, Risiko 5). Entscheidung:
- Flag- und Variablen-Bereiche kommen nur als Metadaten für spätere Prüfungen; Story-Graph, Bereich Geschichten und das Löschen von Daten gehen nach 3d; Roadmap und die „Später“-Tabelle von 3b.3 werden in 3b.4a Task 0 angepasst.

## 1. Umfang Import
**Entscheidung.** 3b.4 importiert fertige Binärteile in bestehende Karten. Alles läuft über einen Assistenten „Karte importieren“ im Scharnier.

| Quelle | Zusage | Regeln |
|---|---|---|
| **NSBMD** → Modell einer Karte | ja | Magic `BMD0`, BOM `FFFE`, Version 2, Größenfeld = Länge, Blockzahl 1–2 mit Offsets im Bereich. MDL0 über die Block-Offsets suchen, nicht fest an 0x18. TEX0 entfernen und den Kopf neu schreiben. Genau ein Modell im MDL0-Dictionary (das Spiel prüft mit Assert). Größe ≤ 0xF000 ist Fehler, Länge % 4 ≠ 0 ist Hinweis. Textur- und Palettennamen gegen das NSBTX der Area-Data der Zielkarte prüfen, fehlende sind eine Warnung („2 Flächen erscheinen grau“). Neu in `g4-formats/src/nsbmd.rs`: `NsbmdInfo { blocks, mdl0_range, model_count, materials: [{name, texture, palette}] }`, `strip_tex0`, `embed_tex0`. |
| **Ganze Map-`.bin`** | ja, auch Pt ↔ HGSS | Erst als Zielfamilie lesen, dann als die andere. Pt → HGSS: `BLANK_BGS` einsetzen. HGSS → Pt: BGS entfällt, mit Hinweis. Gebäude aus der anderen Familie sind standardmäßig aus (die Modell-IDs unterscheiden sich; Regel wie DSPREs `LeavesOut`), per Schalter wieder an. DP wird nicht erkannt. Ein DP-BDHC fällt nur auf, wenn die BDHC-Prüfung ihn ablehnt **[unverifiziert]**. |
| **`.per`/`.mp`** | ja | Genau 2048 Bytes, sonst Ablehnung ohne Änderung. Optional nur die Kachelart oder nur die Kollision übernehmen. |
| **`.bld`** | ja | Länge % 48 = 0. Beim Import aus PDSMS sind Rotation und Dummy-Bytes 0; das Spiel ignoriert sie (`map.md`). |
| **`.bdhc`/`.bdhcam`** | ja, roh | Magic `BDHC`, ≤ 0x9000 (Fehler), Zähler im Kopf passen zur Länge. Der Modell-Import zeigt immer „Laufhöhen bleiben von der alten Karte“. |
| **`.bgs`** | ja, nur HGSS | Signatur `0x1234`, Länge = 4 + len; `MapFile::validate` deckt das ab. |
| **Karte aus diesem Projekt** | ja | Alle oder einzelne Teile von Karte N (Picker mit Kartenbild). Dazu gehört „Gelände von Karte N übernehmen“ als eigene Inspector-Aktion. |
| **PDSMS-Ordner ohne Baking** | ja, letzter Task von 3b.4b, darf gestrichen werden | `.pdsmap` nur für `gameindex`, `mapstart` und `areaindex` lesen, die Raster überspringen. Je Karte `<stem>_XX_YY.{nsbmd,per,bld,bdhc,bdhcam,bgs}` zuordnen, auch die alte Form `<stem>.<ext>`. Das NSBMD erzeugt der Nutzer selbst mit PDSMS/g3dcvtr. 8 und 9 Ebenen werden toleriert, ebenso `exportgroup` und `egcenter`. Ziel sind nur vorhandene Zellen; ein neues Gebiet macht vorher der Assistent „Gebiet anlegen“. |
| **Export der Gegenstücke** | ja | Inspector der Karte, Menü „Teile exportieren“: Modell (NSBMD, wahlweise „mit Texturen“ über `embed_tex0` aus dem Area-NSBTX), Gelände `.per`, Gebäude `.bld`, Laufhöhen `.bdhc`, Klang `.bgs`, ganze Karte `.bin`, glTF (das GLB, das die Szene ohnehin erzeugt). Der Export liefert dem Gate seine Importdatei, ohne dass Assets ins Repo kommen. |
| `.pdsts`, OBJ/glTF → NSBMD, PNG → NSBTX, BDHC-Erzeugung, Kollisionsableitung | **nein, Phase 6** | Die Roadmap-Zeile „OBJ über apicula-Pfad“ wird korrigiert: apicula ist nur Lese- und Exportpfad. |

### Ort in der Oberfläche
Der Assistent hat vier Einstiege, das Ziel ist jeweils vorbelegt:
1. Inspector einer Matrixzelle oder Karte: „Karte ersetzen…“
2. Das `+`-Menü der Orte: „Karte importieren…“
3. Die Palette
4. Eine Datei auf die Schau ziehen. Das gibt es nur in Tauri (`onDragDropEvent`, **[unverifiziert, ob 8.6/Tauri 2 das ohne Plugin liefert]**); die Zelle unter dem Zeiger wird das Ziel.

Technik bekommt keinen Import.
### Schritte des Assistenten (Bauen-Sprache)
1. **„Was kommt rein?“** `Segmented` Datei · Karte aus diesem Projekt · PDSMS-Ordner. Nach der Wahl erkennt die App den Inhalt und zeigt eine Karte, etwa „3D-Modell, 41 KB, mit Texturen (werden entfernt)“ oder „Ganze Karte aus HeartGold“.
2. **„Wohin?“** Kartenbild der Zielzelle mit Ortsname. „Andere Zelle wählen“ übernimmt den nächsten Klick in der Schau, wie der Schritt „Eingang“. „Als neue Karte“ hängt einen Member an und trägt die Plakette „Nicht rückgängig“.
3. **„Welche Teile?“** Eine Zeile je Teil (Modell, Gelände, Gebäude, Laufhöhen, Klang) mit `Switch` „ersetzen“ und dem Herkunftstext „aus der Datei“, „bleibt wie bisher“ oder „nicht in der Datei“. Beim reinen Modell-Import steht dort eine `Notice`: „Gelände und Laufhöhen bleiben von der alten Karte.“ Dazu gibt es die Taste „Gelände von einer Karte übernehmen…“.
4. **„Prüfen“** Eine Prüfliste mit Zeilen ✓, ! und ✕ in Klartext, zum Beispiel „Passt in den Speicher (41 von 60 KB)“ oder „2 Texturen fehlen im Texturpaket dieses Orts, diese Flächen erscheinen grau“. Daneben die Kartenbilder vorher und nachher: der vorhandene `thumbnailRenderer` auf dem GLB von `import_preview_glb`. Ein ✕ sperrt „Importieren“. Tüfteln zeigt zusätzlich Bytes, Offsets und Blockliste.
5. **Ergebnis.** Der Import ist schon angewendet, als **ein** Changeset über `edit_map`, also mit ⌘Z rückgängig. Die Schau springt zur Karte. Angeboten werden „Behalten“ (primär) und „Zurücknehmen“. Darunter steht „Neu durch den Import (2)“: die Prüfbefunde, die vorher nicht da waren, also die Differenz des inkrementellen Laufs vor und nach dem Import. Dazu kommt die Import-Prüfung `import.event-blocked`: Figuren auf Kacheln, die jetzt blockiert sind und vorher frei waren. Jeder Befund hat „Zeigen“.
### Schnappschuss
`apply_import` legt vor der ersten Änderung einer Karte deren Plattenstand in `.g4/backups/maps/NNNN-<zeit>.bin` ab (Ring von 20). Undo endet mit dem Neustart, die Sicherung nicht. Technik › Prüfen und Bauen nennt den Ordner. Die Plakette „Nicht rückgängig“ gibt es nur für „Als neue Karte“.
### Verworfen
- **(a) Einzelne Inspector-Tasten je Teil wie DSPRE main**, ohne Assistent: Dann sieht niemand die Prüfliste, und Modell und Gelände passen nach dem Import nicht zusammen, ohne dass es jemand merkt.
- **(b) Import mit echter Live-Vorschau in der Schau vor dem Anwenden:** Spec 3c schließt die Live-Vorschau von Assistenten aus. „Anwenden und ansehen, dann Behalten oder Zurücknehmen“ nutzt Undo und kostet nichts.
**Preis bei Irrtum.** Falls `model_inputs` den Store-Stand ungespeicherter Karten nicht liest (**[unverifiziert]**), zeigt die Schau nach dem Anwenden das alte Modell. Dann braucht es einen Model-Reload über den Bus (klein). Wird PDSMS gestrichen, verliert man nur einen Komfortweg, denn die Einzelteile gehen trotzdem.

## 2. Umfang Prüfungen
### Schwere-Regel
| Schwere | Bedeutung | Prüflicht |
|---|---|---|
| **Fehler** | Das Spiel liest Müll, hängt, stürzt ab, oder der Build bricht | zählt |
| **Warnung** | Funktioniert, ist aber fast sicher nicht gewollt | zählt |
| **Hinweis** | Kann Absicht sein | zählt, Punkt bleibt neutral |

### In 3b.4a (40 Codes)
**Fehler:**
- Warps: `warp.dest-header`, `warp.dest-warp`, `warp.off-map`; Events: `event.script-missing`, `event.common-missing`, `event.trainer-missing`, `event.off-map`; Header: `header.ref-range`, `header.wild-dangling`, `header.script-kind`, `header.level-target`. Dazu `header.level-message`: direkt und eine `Call`-Ebene tief; das ist die Regel aus dem 3b.2-Gate; Scripts: `script.message-missing` (Port der Logik von rotom-lsp `message_refs.rs`, MIT), `script.trainer-target` (eigene `scrcmd_refs.yaml` für `TrainerBattle` und Co.), `script.common-missing` (nur Zahlen); Text: `text.encode`, `text.json-broken`; Trainer: `trainer.no-party` (Fehler, wenn Event oder Script ihn nutzen, sonst Warnung), `trainer.empty-species`, `trainer.id-range`, `trainer.name-length`, `trainer.texts`; Begegnungen und Daten: `encounter.species-range`, `data.evo-target`, `data.learnset-limit`, `data.learnset-move`, `data.wild-item`; Raster, Karten, Assets: `matrix.map-range`, `matrix.header-range`, `area.tileset-range`, `map.building-range`, `asset.unreadable`; Aus dem Import-Bericht, projektweit für jede Karte: **`map.model-size`** (> 0xF000), **`map.model-count`** (≠ 1), **`map.bdhc-size`** (> 0x9000).

**Warnung:**
- `warp.arrival-stuck`, `event.trainer-blind`, `event.trigger-empty`, `event.same-spot`, `header.text-split`, `matrix.header-elsewhere`; `text.too-wide` (216 px), `text.three-lines`; `trainer.no-moves`, `trainer.level`, `trainer.double-one`, `trainer.flag-capacity`; `encounter.empty-slot`, `encounter.level-zero`; `build.member-newer`; **`map.texture-missing`**: Rauschen auf Vanilla ungemessen, Task 0 misst. Mehr als 0 Treffer gehen in die Ausgangslage.

**Hinweis:**
- `warp.never-fires` (nur, wenn auch kein Warp hinführt), `event.shared-file`; `encounter.min-max`, `encounter.empty-replacement`; `script.message-empty`; `build.stale-scripts`, `build.stale-text`, `build.rom-old`; `map.model-align`.
### Später
| Prüfung | Wohin | Grund |
|---|---|---|
| `script.flag-range`, `event.hidden-flag-range`, `event.trigger-var-range` | 3d, mit Flags/Variablen | Bereiche aus pret `flags.h`/`vars.h` erst als Metadaten |
| `script.warp-target`, `script.id-range` | später | Brauchen eine typisierte Parameterliste über die DB hinaus |
| `data.name-length` | später | Grenzen unverifiziert |
| `header.no-map` | später | Semantik bei Innenraum-Matrizen unklar |
| `script.rotom-all` | später | Der LSP prüft offene Dokumente, der Build kompiliert veraltete; ein unberührtes, übernommenes Projekt kompilierte schon beim `adopt` |

### Nie als Projektprüfung
Folgende Befunde werden höchstens am Objekt angezeigt, nicht als Prüfung gezählt:
- `warp.one-way`: nur die Inspector-Zeile „Rückweg: keiner“; `*.unused`: nur „Verwendet von niemandem“ in der Schau; `local_id` doppelt (40 Vanilla-Treffer); `trainer_type` ohne Trainer-Script (29); Figur auf blockierter Kachel (nur im Import-Ergebnis als Differenz); rotoms `MessageLineTooLong` (200 px).
### Textbreite 216 statt 200 px
**Plan-Task 0 von 3b.4a** belegt die Feldbreite in pret:
- pokeplatinum: Fenster der Feldnachricht, `Window_Add(…, 2, 19, 27, 4, …)` in `src/field_message.c` **[Fundort aus dem Gedächtnis, unverifiziert]**
- pokeheartgold: Gegenstück

Die Breite (27 × 8 = 216) kommt dann mit Quelle nach `crates/g4-kernel/metadata/check_limits.yaml`:
```yaml
dialog_line_px: { value: 216, source: "pokeplatinum src/field_message.c …", verified: true }
warp_dest_dynamic: { value: 0x0FFF, source: "6 Vanilla-Aufzüge", verified: false }
dummy_trainer: 0
dummy_event_file: 0
model_buffer: 0xF000
bdhc_buffer: 0x9000
```

Weitere Änderungen:
- `text::validate(text, file, limits)` misst gegen `limits.dialog_line_px` statt gegen chatots `DIALOG_LINE_MAX_PX`. Die Meldung wird konkret: „Zeile 2 ist 231 px breit, ins Textfeld passen 216.“ Der Test `text.rs:773` zieht mit.
- Der Kernel filtert rotoms `MessageLineTooLong` aus den Compile-Problemen. Ein Textliteral landet nach dem Compile in einem Archiv (`touched_archives`), und dort misst `text.too-wide` mit derselben Grenze. Sonst hätte man zwei widersprüchliche Grenzen.
- Panel Texte (3c §7.2): Der Balken „Längste Zeile“ misst in px gegen dieselbe Grenze (`check_limits`) statt in 38 Zeichen.
- Upstream-Meldung an chatot/rotom als Notiz, ohne Abhängigkeit.
- Belegt Task 0 die 27 Kacheln nicht, bleibt der Wert mit `verified: false` stehen und der Befund wird Hinweis statt Warnung. Gemessen ist die Kante bei 216 ohnehin.
### Ausnahmen
**Ausnahmen mit Spielgrund sind Regeln** in `check_limits.yaml`, mit Quelle: `0x0FFF` (Aufzüge, dynamisches Ziel), Trainer 0, Eventdatei 0 und der Dummy-Header bei `event.shared-file`. **Ausnahmen ohne erkennbaren Grund kommen in die Ausgangslage** (§5): 10 Arena- und Script-Warps, 2 doppelt besetzte Kacheln, die Text-Ausreißer über 216 px und 1 Level-Script-Ziel (Header 186).
### Verworfen
- **(a) Den ganzen Katalog (~60):** Mehr als die Hälfte der Zusatzprüfungen braucht ungeprüfte Metadaten (Flag-Bereiche, Namensgrenzen) und liefert falsche Treffer. Falsche Befunde kosten mehr Vertrauen, als fehlende kosten.
- **(b) Nur die Roadmap-Vier** (Warps, Scripts ohne Ziel, Trainer ohne Party, Textlängen): Importe brauchen `map.*`, und Befunde aus fremden DSPRE-Änderungen (`header.ref-range`, `matrix.*`) sind billig und rauschfrei.
**Preis bei Irrtum.** Ein zu lauter Code wird per Metadaten-Schalter zum Hinweis oder abgeschaltet. Das ist eine Zeile und ein Test.

## 3. Architektur `checks`
### Module
```
crates/g4-kernel/src/problem.rs        Severity { Error, Warning, Info }, Problem (zieht aus text.rs um, re-export bleibt)
crates/g4-kernel/src/checks/mod.rs     Check-Trait, Registry, Runner, Cache, Delta, Fingerprint, Ausgangslage, g4-checks.json
  warps.rs events.rs headers.rs(+matrix, area, map) scripts.rs text.rs data.rs build.rs
crates/g4-kernel/src/checks/fixes.rs   FixPlan, plan/apply
crates/g4-kernel/metadata/{check_limits.yaml, scrcmd_refs.yaml, checks_baseline/{CPUD,IPKD,IPKE}.json}
```

### Typen
```rust
pub struct Finding {
    pub code: &'static str, pub severity: Severity, pub subject: Subject, pub key: String,   // id = code|subject|key
    pub args: BTreeMap<&'static str, Arg>,     // typisierte Werte für die Vorlage im Frontend: Header(712), Warp{file,idx}, Count(2), Px(231)
    pub target: NavTarget, pub also: Vec<NavTarget>,
    pub fixes: Vec<FixOffer>,                  // { id, kind: Direct | Choose(PickerKind), irreversible }
    pub span: Option<(PathBuf, u32, u32)>,     // Script/Text: Datei, Zeile, Spalte
    pub fingerprint: u64,                      // Hash über id + regelrelevante Werte (z. B. Warp-Ziel + Kachel)
    pub fallback: String,                      // deutscher Satz mit Kennungen für CLI, Brücke und Tüfteln
}
pub trait Check: Sync {
    fn code(&self) -> &'static str; fn tier(&self) -> Tier; fn needs(&self) -> Needs;
    fn subjects(&self, cx: &Ctx) -> Vec<Subject>;
    fn affected(&self, changed: &[AssetId], cx: &Ctx) -> Vec<Subject>;   // über RefIndex::refs / refs_at_or_above
    fn run(&self, cx: &mut Ctx, s: Subject, out: &mut Vec<Finding>);
}
```

### Sätze im Frontend
**Entscheidung: Die Sätze baut das Frontend.** `apps/studio/src/workbench/checks/messages.ts` hält je Code eine Vorlage für Bauen (Namen über `buildPlaces`, `eventTitle`, Namenstabellen) und eine für Tüfteln (mit Kennungen), dazu `explain` (warum, und wann es Absicht sein kann). Ein Test verlangt für jeden Code im `check_catalog` eine Vorlage. `fallback` aus dem Kernel bleibt für Brücke, CLI und Szenario-Logs. Der Grund: Ortsnamen leben heute im Frontend (`buildPlaces`), und „ohne Zahlen“ geht nur mit ihnen.
### Wann geprüft wird
- **Volllauf** nach dem Öffnen und nach dem RefIndex-Aufbau, im Hintergrund (`spawn_blocking`, abbrechbar beim Projektwechsel); **Fortschritt** über einen Tauri-`Channel` (`checks_progress { phase, done, total }`), die Texte zuletzt; **Inkrementell** nach jeder Store-Änderung: Das Frontend ruft `checks_delta { since }` auf den Bus-Themen `edited`/`history` (Muster von `history()`), mit 300 ms Debounce; **Speichern:** Prüfungen der Stufe Save laufen nach `save_script`, `save_level_script`, Text-Speichern und `adopt`; **„Erneut prüfen“** verwirft den Cache und startet einen Volllauf.
### RefIndex-Erweiterung
Neue Targets: `Header`, `Warp(file, idx)`, `EventFile`, `ScriptFile`, `LevelScript`, `ScriptSlot(file, n)`, `TextArchive`, `Message(archive, idx)`, `Map`, `Matrix`, `AreaData`. Neue Quellen: Scripts (`Trainer`, `Message`, `CallCommonScript`) und Level-Script-Einträge. Dazu kommt die Bereichsabfrage `refs_at_or_above(kind, n)` für Anzahl-Änderungen.

**Invariante `delta == voll`:** proptest über zufällige Folgen von Edit, Undo, Redo, `append_member` und `apply_fix`, dazu einmal gated über die drei Projekte.
### Text-Index
Der Index aus 3b.2 hält künftig den Originaltext neben dem kleingeschriebenen. Das kostet für 46 053 Nachrichten etwa 3 MB.

**Neu:** ein Plattencache `.g4/cache/text-index.bin`, Schlüssel (Pfad, Länge, mtime) je Member, gültig, solange der Schlüssel passt. Damit fällt die kalte Dekodierung (5 s) nur beim ersten Öffnen an. Bei einem Schlüssel-Fehltreffer wird das Archiv neu dekodiert, nie geraten.
### Kosten
Ziel nach dem Bericht:
- Volllauf ohne Text: unter 150 ms (Rust); Text warm: etwa 1 s, mit Cache etwa 50 ms **[Schätzung]**; inkrementell: unter 1 ms je Edit.

Task 0 misst die RefIndex-Aufbauzeit mit den neuen Zielen (Ziel aus 3b.3: < 300 ms).
### rotom
Nichts wird doppelt geprüft. Syntax, Befehle, Argumente, Labels und `Module::Label` bleiben bei rotom und rotom-lsp. Die Prüfungen setzen nur dort an, wo Projektkontext nötig ist. `MessageLineTooLong` wird gefiltert (§2).
### Vertrag Kernel ↔ Oberfläche
`ProblemDto` bekommt optionale Felder:
```
severity: "error"|"warning"|"info"; source: … | "check"
code?, subject?: {kind, id}, key?, args?, target?: NavTargetDto, also?, fixes?: [{id, kind, picker?, irreversible}],
fingerprint?, state?: "open"|"baseline"|"ignored"
```

- `file`/`line`/`col` bleiben für Script- und Textbefunde, sonst ist `file` leer; `ProblemsStore` bekommt die Quelle `check`, ersetzt je `subject` und zählt `info`. `countsText` liefert dann „2 Fehler, 1 Warnung, 3 Hinweise“; Neuer Typ `NavTargetDto`: serde-getaggter Spiegel von `navigation.ts`, ergänzt um die Datenziele aus 3b.3 (`trainer`, `encounter`, `species`).
### Verworfen
- **(a) Prüfungen im Frontend auf den DTOs:** Das bedeutet doppelte Fachlogik, und Brücke und Szenario könnten nichts prüfen.
- **(b) Deutsche Meldungen fertig aus dem Kernel:** Dann bräuchte der Kernel die Ortsnamen; `buildPlaces` müsste in den Kernel umziehen, eine eigene Baustelle.
**Preis bei Irrtum.** Weichen die Vorlagen vom Kernel ab, zeigen sie falsche Sätze. Abhilfe: der Katalogtest und der Fallback als Tooltip in Tüfteln. Ist die Invalidierung falsch, bleiben alte Befunde stehen. Abhilfe: der proptest, „Erneut prüfen“, und Fixes prüfen das Subjekt immer voll nach.

## 4. Beheben
### Ablauf
`plan_fix { id, fix, choice? }` liefert einen `FixPlan { label, lines: [Change], irreversible: [Change], recheck: [Subject], token }`. Dann `apply_fix { token }`: **ein** Changeset über `Store::transaction` (3b.3), danach eine volle Prüfung der Recheck-Subjekte. Bleibt der Befund, kommt der Toast „Noch nicht behoben: …“, und die Zeile bleibt mit dem Vermerk stehen.

Bei Gruppen gibt es „Alle 14 beheben“, ebenfalls in einem Changeset.
### Fixes in 3b.4
Ein Klick heißt `Direct`, „Wählen…“ heißt `Choose` mit Picker.

| Code | Fix | Art |
|---|---|---|
| `warp.dest-warp` | „Zum ersten Eingang von ‹Ort› führen“ (Ziel hat ≥ 1 Warp) · „Ziel wählen…“ (Picker Ort mit Kartenbild, danach die Eingänge als Kacheln auf dem Kartenbild) | Direct · Choose |
| `warp.dest-header` | „Ziel wählen…“ · „Tür löschen“ | Choose · Direct |
| `warp.off-map`, `event.off-map` | „Auf die Karte schieben“ (nächste gültige Kachel) · „Löschen“ | Direct |
| `event.script-missing`, `header.level-target` | „Script anlegen“ (`add_script`, Nummer ins Event; öffnet das Script) | Direct, **Script-Teil nicht rückgängig** |
| `event.trainer-missing` | „Trainer wählen…“ · „Neuen Trainer anlegen“ (`create_trainer`) | Choose · Direct, **nicht rückgängig** |
| `event.trainer-blind` | „Sieht den Spieler (Sichtweite 3)“ | Direct |
| `event.trigger-empty` | „Script anlegen“ · „Auslöser löschen“ | Direct |
| `header.ref-range`, `header.wild-dangling` | „Wählen…“ · bei Begegnungen „Keine“ | Choose · Direct |
| `script.message-missing` (N = Anzahl) | „Nachricht anlegen“ (hängt eine leere Nachricht an und öffnet sie im Bereich Texte) | Direct |
| `text.too-wide`, `text.three-lines` | „Umbrechen“ (chatot `word_wrap` mit 216 px). Der Plan zeigt vorher und nachher als `DSScreen`-Seiten. | Direct |
| `trainer.no-party` | „Pokémon hinzufügen…“ (öffnet den Trainer mit offenem Art-Picker auf Platz 1) | Choose |
| `trainer.empty-species` | „Platz entfernen“ | Direct |
| `trainer.no-moves` | „Attacken aus dem Lernset“ (Regel aus 3b.3 §11) | Direct |
| `trainer.level`, `trainer.double-one` | „Auf 1 bis 100 setzen“ · „Einzelkampf“ | Direct |
| `encounter.min-max` | „Tauschen“ | Direct |
| `matrix.map-range`, `matrix.header-range`, `map.building-range` | „Zelle leeren“ · „Gebäude entfernen“ | Direct |

Alles andere bekommt nur „Zeigen“. Es gibt keinen Fix ohne eindeutiges Ergebnis.
### Dialog
Kein Modal, sondern ein `FixPopover` an der Zeile, am Inspector-Feld oder am Marker:
- Titel = Label („Zum ersten Eingang von Zweiblattdorf führen“); darunter die Änderungen als Sätze („Tür im Haus des Rivalen: führt zu Eingang 1 statt 4“); Plakette „Nicht rückgängig“ an jeder irreversiblen Zeile; dann ist der Titel als Frage formuliert (3c-Regel); Taste „Beheben“ (primär) und der Hinweis „⌘Z nimmt es zurück“.

Bei `Choose` öffnet sich zuerst der `Picker`, danach dasselbe Popover.

**Nicht rückgängig** sind nur Anlage-Schritte: Script-Quelle mit Member (`add_script`), `create_trainer`, `create_encounter_file`, „Als neue Karte“. Die Bindung daran (Event-Nummer, Header-Feld) ist rückgängig, wie heute in den Assistenten.
### Verworfen
- **(a) Fixes nur als Navigation** („Öffnen“ und dann selbst machen): Das widerspricht dem Ein-Klick-Ziel. Einsteiger finden das Feld nicht.
- **(b) Automatisch beheben beim Speichern:** Das ändert still Daten. DSPREs Importbericht zeigt, dass ein Opt-in je Zeile nötig ist.
**Preis bei Irrtum.** Ein falscher Fix wird mit ⌘Z zurückgenommen. Die irreversiblen Anlagen hinterlassen höchstens einen unbenutzten Member; das ist dieselbe Lage wie heute in den Assistenten.

## 5. Ausgangslage und Ignorieren
**Entscheidung.** Zuerst härtere Regeln, damit Vanilla fast rauschfrei ist. Was übrig bleibt, deckt eine **mitgelieferte Vanilla-Ausgangslage je Spielcode** ab.
### Mitgelieferte Ausgangslage
`crates/g4-kernel/metadata/checks_baseline/{CPUD,IPKD,IPKE}.json` enthält `{code, subject, key, fingerprint}` der Warnungen und Hinweise auf dem unveränderten Spiel, also Hashes und Kennungen, keine Assets.

Ein Befund mit gleichem Fingerprint gilt als **„aus dem Originalspiel“**: Er zählt nicht im Prüflicht und steht im Panel eingeklappt in der Gruppe „Aus dem Originalspiel (97)“. Ändert der Nutzer die Daten, zum Beispiel den Text der Nachricht, ändert sich der Fingerprint, und der Befund gehört ihm. Er wird dann normal gezählt.

**Fehler kommen nie in die Ausgangslage.** Das funktioniert für jeden Hack auf diesen drei Spielen, egal wann er zum ersten Mal geöffnet wird.
### Wie sie entsteht und altert
- Ein gated Test `vanilla_checks_baseline` prüft die drei `_DSPRE_contents` (nur lesend) gegen die Dateien; `G4_UPDATE_BASELINE=1` schreibt sie neu; `scripts/gate.sh` wird rot, wenn sie veraltet sind, wie der Schritt „tokens up to date“; Ändert sich eine Prüfung, wird die Datei im selben Commit neu erzeugt. Damit altert sie nie unbemerkt.

**Rückfall für unbekannte Spielcodes** (SoulSilver, andere Sprachen): Der erste Volllauf schreibt seine Warnungen und Hinweise als `local_baseline` nach `g4-checks.json`. Kommt später eine neue Prüfung hinzu, werden deren erste Treffer ebenfalls aufgenommen; das hält `checks_version` je Code fest. Das Panel sagt es offen: „Stand beim ersten Prüfen am 23.10.: 41 Befunde ausgeblendet.“
### Projektdatei `g4-checks.json` (Wurzel, versioniert, DSPRE fasst sie nicht an)
```json
{ "version": 1, "disabled": ["warp.never-fires"],
  "ignored": [{ "id": "warp.dest-warp|EventFile(390)|warp 2", "fingerprint": "9f…", "note": "Aufzug, per Script", "at": "2026-10-23" }],
  "local_baseline": null }
```

### Anzeige
- Fuß des Panels: „12 ausgeblendet · 97 aus dem Originalspiel“, jeweils als Umschalter. Eingeschaltet erscheinen die Zeilen gedämpft mit „Wieder zeigen“; „Ignorieren…“ fragt optional nach einer Notiz; „Prüfung ausschalten“ steht im Zeilenmenü und in Technik › Prüfen und Bauen; dort lässt sie sich auch wieder einschalten.
### Verworfen
- **(a) Nur eine Ausgangslage beim ersten Lauf:** Ein schon veränderter Hack würde die eigenen Fehler des Nutzers festschreiben.
- **(b) Keine Ausgangslage, nur harte Regeln:** 90 bis 114 Textbefunde je Spiel bleiben auch bei 216 px. Einsteiger sähen beim ersten Öffnen ein gelbes Prüflicht, das sie nicht verursacht haben.
**Preis bei Irrtum.** Ein zu breiter Fingerprint blendet echte Nutzerfehler aus, ein zu enger lässt Originalbefunde nach harmlosen Änderungen wieder auftauchen. Der Fingerprint ist je Code definiert und getestet: Eine Änderung eines nicht relevanten Feldes lässt ihn gleich, eine relevante ändert ihn. Phase 4 kann das durch den Basis-ROM-Vergleich ersetzen.

## 6. Oberfläche
**Prüflicht** (Scharnier):
- Punkt in der schlimmsten Farbe, Text „2 Fehler, 1 Hinweis“ ohne Ausgangslage und Ignoriertes, „Alles stimmig“ bei null; Während des Volllaufs Spinner und „Prüft … Texte 312 von 724“; die Zähler wachsen live; Tooltip mit der Aufteilung nach Bereich. Klick öffnet das Panel Probleme, gefiltert auf „Prüfungen“, wenn nur solche offen sind.

**Panel Probleme** (Hand):
- Kopf: `PanelHead` „Probleme“ mit Zähler und „Erneut prüfen“ (zeigt „vor 2 min · 0,4 s“).
- Filter: `FilterChips` (3b.3) für Schwere und Quelle (Prüfung, Compile, Text, live), dazu ein Suchfeld.
- Gruppen nach Bereich (Türen und Warps, Figuren, Orte, Scripts, Texte, Trainer, Begegnungen, Daten, Raster und Karten, Bauen), darin je Code einklappbar („Tür führt ins Leere (3)“, mit „Alle beheben“, wenn alle `Direct` sind).
- Zeile: Schwere-Symbol, Satz, Subjekt als `Reference`-Chip in Bereichsfarbe, „Zeigen“ (Enter, `goTo(target)`), „Beheben ▾“ (nur mit `fixes`), Menü ⋯ mit „Warum?“ (`explain`), „Ignorieren…“, „Prüfung ausschalten“.
- Fuß wie in §5.

**Technik › Prüfen und Bauen:**
- links die Prüfbereiche mit Zählern und Schaltern je Code (gruppiert, mit `explain`), „Erneut prüfen“ und der Ordner der Karten-Sicherungen; rechts der Stand des letzten Builds (wie 3c); Bauen bleibt erlaubt. Bei Fehlern trägt die Taste den Zusatz „3 Fehler, trotzdem bauen“, ohne Dialog. Der Toast nach dem Build nennt die Fehlerzahl mit „Probleme zeigen“; Build-Schritte als Liste bleiben in Phase 4 (ROADMAP Zeile 202).

**Befunde am Objekt:**
- **Szene:** Marker mit Befund bekommen ein Abzeichen (Kreis in `status/fehler` oder `status/warnung` mit „!“) als Kind-Sprite in `markers.ts`. Klick wählt den Marker, und der Inspector zeigt den Befund oben.
- **Raster:** Eckpunkt in der Zelle (Karte oder Header fehlt).
- **Explorer:** `TreeRow trailing` mit Punkt und Zahl je Ort, aggregiert über die Subjekte seiner Header.
- **Listen:** Trainer, Begegnungen, Textarchive und Scriptdateien zeigen Punkt und Zahl an der Zeile. Bei Textarchiven ersetzt das die heutige Überlängen-Plakette.
- **Inspector:** Der `PanelHead` bekommt die Plakette „1 Fehler“, das betroffene Feld eine `FindingNote` unter der `InspectorRow`. Sie enthält den Satz und die Fix-Tasten inline, zum Beispiel „Zum ersten Eingang führen“ und „Ziel wählen…“.
- **Script-Editor:** Befunde mit `span` laufen als zweite Lint-Quelle in CodeMirror, mit Gutter und Unterstreichung.

**Fehlende Primitives** (je mit Galerie-Seite):
| Primitive | Zweck |
|---|---|
| `FindingMark` | Punkt mit Zahl und Schwere, für Listen, Explorer, Raster |
| `FindingNote` | Inline-Befund mit Aktionen am Feld |
| `FixPopover` | Änderungssätze, Plakette „Nicht rückgängig“, Beheben |
| `ChecklistRow` | ✓ ! ✕ mit Satz und optionalem Detail, für die Import-Prüfung |
| `BeforeAfter` | zwei Kartenbilder oder `DSScreen`s mit Beschriftung |
| `FileDrop` | Ablagefläche über der Schau, nur Tauri |
| `Spinner`/`Progress` im Prüflicht | Fortschritt des Volllaufs |
| `GroupRow` für `Table` | Gruppenkopf mit Zähler und Sammelaktion |

**Verworfen:**
- **(a) Eigenes Fenster wie DSPRE „Validation“:** Dann wären die Befunde weg vom Objekt, ohne Sprung und ohne Beheben.
- **(b) Nur Panel und Prüflicht** (kein Abzeichen in der Szene): Porymap zeigt, dass Warnungen am Event gelesen werden, Listen nicht.
**Preis bei Irrtum.** Abzeichen in großen Matrizen (Matrix 0 mit 1213 Warps) kosten Sprites. Dann bekommen nur die sichtbaren Zellen Abzeichen, wie bei den Modellen.

## 7. Gate
**Szenario `scripts/agent/scenarios/pt-kleiner-hack.yaml`** (dazu `hg-kleiner-hack.yaml` für HeartGold DE), aufbauend auf `pt-neuer-header.yaml` und `pt-trainer-begegnung.yaml` (3b.3b):
1. `apply_header_patch`, `create_area` (zwei Zellen aus Route 201) mit neuem Header und neuer Eventdatei.
2. **Import:**
   - `export_map_part { map: ‹eine Vanilla-Karte mit See›, part: "model", path: "${KOPIE}/import/see.nsbmd" }`
   - `export_map_part { …, part: "permissions", path: "…/see.per" }`
   - `import_map_parts { map: ‹neue Karte 1›, model: "…/see.nsbmd", permissions: "…/see.per" }`, mit Erwartung: Prüfliste ohne ✕, Texturen vollständig (gleiches Gebiet).
3. **Rot:**
   - eine Tür mit `dest_warp_id: 5` auf den neuen Header (der hat nur Warp 0)
   - eine Figur mit Script #2, das es noch nicht gibt
   - `create_trainer` Nr. 1 ohne Party, an eine Figur gebunden
   - `run_checks {}`, Erwartung: `warp.dest-warp`, `event.script-missing` und `trainer.no-party` je 1, Ursprung „open“.
4. **Grün:**
   - `apply_fix warp.dest-warp → first-entrance`
   - `apply_fix event.script-missing → add-script`
   - `save_script` mit dem Ereignis-Script: Textliteral, einmalig ein Item per Flag, danach ein anderer Text
   - `data_edit trainer` (Party 1 Pokémon), `create_trainer` Nr. 2 mit Party und Kampftexten (neue Texte)
   - `edit_asset` am Textarchiv: eine Vanilla-Nachricht geändert, eine angefügt
   - `run_checks {}`, Erwartung: `counts.error == 0`, und es gibt keinen offenen Befund, dessen Subjekt das neue Gebiet ist.
5. `save`, `build_rom`.
6. **Spielen:** `continue` → Treppe → `expect-map ${NEU}` → `shot see-import` → Figur ansprechen (`walk-to`, `press a`, `mash a 300`) → `walk-until-battle` → `expect-battle S_T1 L_T1` → `win-battle` → Trainer 2 ebenso → `shot fertig`.

**Brücken- und Tauri-Befehle, die fehlen:**
| Gruppe | Befehle |
|---|---|
| Prüfungen | `run_checks`, `checks_delta`, `check_catalog`, `check_limits`, `plan_fix`, `apply_fix`, `ignore_finding`, `unignore_finding`, `set_check_enabled` |
| Import und Export | `inspect_import { source, target }` → Bericht ohne Änderung, `apply_import { plan }`, `import_map_parts` (Kurzform für Szenarien), `import_preview_glb`, `export_map_part`, `copy_map_parts { from, to, parts }` („Gelände von Karte N“), `pdsms_scan { path }` |

Jeder Befehl steht in `COMMANDS`, `dispatch.rs` und im Handler; die Tests `commands_match_the_tauri_handler` und `every_command_accepts_the_keys_api_ts_sends` ziehen mit. Im Browser-Modus müssen Importpfade unter den Brücken-Wurzeln liegen (`_testkopien`); der `WebDialogHost` listet sie.

**`g4emu`-Erweiterung:** Ein Edit-Eintrag darf ein drittes Element `{expect: {...}}` tragen. Das ist ein Teilmengen-Vergleich auf der Antwort, zum Beispiel `{counts: {error: 0}}` oder `{codes: {"warp.dest-warp": 1}}`. Er wird gegen das Antwort-Array des Batches ausgewertet. Ob die Brücke das Batch der Reihe nach ausführt und alle Antworten liefert, ist **[unverifiziert]** (Task 0 von 3b.4b). Ohne das wird das Batch in Einzelaufrufe geteilt.

**Per Test-Umgebung (Agent):**
- der Assistent „Karte importieren“ im Browser-Modus mit einer exportierten Datei, mit Screenshots der Prüfliste, des Ergebnisses und von „Zurücknehmen“ (`dirty` vorher und nachher); das Prüflicht rot und danach grün, dazu Abzeichen am Tür-Marker und „Beheben“ im Inspector; `text.too-wide`, „Umbrechen“ mit `DSScreen` vorher und nachher; Pt-↔-HGSS-`.bin`-Import (gated Rust-Test plus ein Screenshot); die Ausgangslage: eine frische Platin-Kopie zeigt „Alles stimmig“ und „97 aus dem Originalspiel“.

**Von Hand:**
- Drag & Drop in der nativen App; „Agent + Blick“ auf den `shot see-import`: Texturen sind sichtbar, nicht grau, Laufhöhen passen; Stoppuhr „Einsteiger ohne Zahlen“: der Ablauf ohne getippte Zahl außer der Ortswahl; DSPRE öffnet die importierte Karte (auf Wunsch auslassbar).

**Gated Rust-Tests:**
- Modell durch sich selbst ersetzen ergibt dieselben Bytes, für alle Vanilla-Karten; Tausch A → B → A ergibt das Original; die anderen Abschnitte sind slice-gleich; `strip_tex0(embed_tex0(m, nsbtx)) == m`; Ablehnung synthetischer Fälle: Größe > 0xF000, zwei Modelle, `.per` mit 2047 Bytes; `vanilla_checks_baseline`; `delta == voll`.

**Synthetische Fixtures:** ein `.pdsmap`-Generator (8 und 9 Ebenen, `exportgroup`/`egcenter`).

## 8. Risiken
1. **Falsche Befunde zerstören das Vertrauen ins Prüflicht.**
   - Jede Prüfung wird vor dem Merge auf drei Vanilla-Projekten gemessen. Ein Fehler-Code mit Vanilla-Treffern ist ein Bug der Regel.
   - Die Ausgangslage ist gated.
   - „Warum?“ nennt die Fälle, in denen es Absicht ist.
   - Jeder Code lässt sich abschalten.
   - Unverifizierte Grenzen (216 px, `0x0FFF`) tragen `verified: false` und werden höchstens Warnung.
2. **Importierte Modelle brechen das Spiel**, schweigend: grau, oder Assert bei zwei Modellen. Die HGSS-Puffergrößen sind unbelegt.
   - Strenge Prüfliste.
   - Task 0 belegt 0xF000 und 0x9000 für HGSS in pokeheartgold; ohne Beleg gilt für HGSS der kleinste Vanilla-sichere Wert (61 420 B) als Warnung.
   - Beide Gate-Szenarien (Pt und HG) spielen die importierte Karte.
   - Schnappschuss und ⌘Z.
3. **Die inkrementelle Invalidierung verpasst Abhängigkeiten.** Beispiele: Header-Kontext von Script zu Textarchiv, Anzahl-Änderungen.
   - proptest `delta == voll` einschließlich `apply_fix` und Import.
   - Fixes prüfen voll nach.
   - „Erneut prüfen“.
   - Tüfteln zeigt Generation und Alter je Befund.
4. **Kosten beim Öffnen.** Kalt 5 s Text, und der RefIndex mit Script-Quellen sprengt eventuell 300 ms.
   - Hintergrund, abbrechbar, Fortschritt im Prüflicht.
   - Texte zuletzt.
   - Plattencache des Text-Index.
   - Task 0 misst den Volllauf auf allen drei Projekten. Über 2 s warm wird `script.*` auf die Stufe Save beschränkt.
5. **Umfang wandert** (40 Codes, fünf Importwege, Fixes, PDSMS, dazu Story-Graph und Flags laut Roadmap, Löschen laut 3b.3).
   - Zwei Pläne mit fester Schnittlinie.
   - PDSMS als letzter, streichbarer Task.
   - Story-Graph, Geschichten, Flag-Bereiche und Löschen offiziell nach 3d; die Roadmap wird in Task 0 korrigiert.
   - Eine neue Primitive kommt nur mit dem Panel, das sie im selben Plan benutzt (Regel aus 3b.3).

**Ungeprüft und für Task 0:**
- pret-Belege für 216 px, `0x0FFF`, HGSS-Puffer; ob ungebundene Materialien im Spiel grau erscheinen (ein `g4emu`-Shot); ob ein Trainer ohne Party tatsächlich abstürzt (bestimmt die Schwere); Vanilla-Rauschen von `map.texture-missing`; ob `model_inputs` den ungespeicherten Store-Stand liest; ob Tauri 2 `onDragDropEvent` ohne zusätzliches Plugin liefert; Batch-Semantik der Brücke für `expect`; RefIndex- und Volllauf-Zeiten.
