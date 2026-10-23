# Vorschlag A: Phase 3b.4 „Import und Prüfungen“

Architekt A. Neigung: Risiko klein halten, kleine lauffähige Schritte, Kernel zuerst, nur versprechen, was sich beweisen lässt. Grundlage: Roadmap, Specs 3c, 3b.3, 3b.2, `import-report.md`, `checks-report.md` und der Code im Worktree `phase-3c1` (`map.rs`, `store.rs`, `text.rs`, `problems.ts`, `bridge/batch.rs`, `g4emu/scenario.py`). Markierung **[ungeprüft]** heißt: nicht im Code oder am ROM nachgesehen.

## 0. Rahmen, der für alle Fragen gilt

**Eingangskriterien.** 3b.4 baut auf Dingen auf, die es heute **noch nicht gibt**: `Store::transaction` und `RefIndex` (3b.3a, nur Plan), Trainer-Asset und `create_trainer` (3b.3b), Panel Probleme und Technik › Prüfen und Bauen im neuen Stil (3c.2, nur Plan). 3b.4 beginnt erst, wenn 3b.3b gemergt ist und das Szenario `pt-trainer-begegnung.yaml` grün läuft. Das Gesamtgate setzt genau dieses Szenario fort (§7).

**Zwei Pläne, beide beginnen mit Kernel-Tasks und enden lauffähig:**

- **3b.4a „Prüfungen“**: Task 0 (Messungen, Belege), dann Kernel `checks/` mit Volllauf, Befehle und Brücke, Prüflicht und Panel, Ignorieren und Ausgangslage, Fixes, Befunde am Objekt. Teilgate: `vanilla_checks` (gated) grün, Szenario `pt-pruefungen.yaml` (rot → Fix → grün).
- **3b.4b „Kartenimport und Gesamtgate“**: Task 0, `g4-formats::nsbmd` (nur Kopf, Blöcke, MDL0-Dictionary), Kernel `map_import`, Befehle, Dialog „Teile importieren“, Export, Szenario `pt-kleiner-hack.yaml` und `hg-kleiner-hack.yaml`. Gate = Roadmap-Gate 3b.

**Aus der Roadmap heraus.** Zeile 201 der Roadmap schiebt „Flags und Variablen sowie Geschichten und Story-Graph“ nach 3b.4. Mein Vorschlag: beides nach 3d (Erzählen). Das Gate braucht beides nicht. Wer es in 3b.4 lässt, verdoppelt die Phase. Ebenso fallen heraus: Löschen von Daten (3b.3 hatte es nach 3b.4 geschoben; ohne Script-Referenzen wäre es ein leeres Versprechen), Referenzen aus Scripts auf Trainer und Items, PDSMS-Ordnerimport (siehe Frage 1).

---

## 1. Umfang Import

**Entscheidung.** 3b.4 importiert **fertige Binärteile in bestehende Karten**, nichts wird gebacken. Alles läuft über `Store::edit_map` und ist damit ein normaler, **rückgängiger** Changeset (die Karte ist heute schon ein Asset mit Undo, `store.rs:470`).

| Teil | Zusage 3b.4 | Prüfung vor dem Schreiben (Ablehnung ändert nichts) |
|---|---|---|
| NSBMD → Modellabschnitt | ja | Magic `BMD0`, BOM `FFFE`, Version `0x0002`, Größenfeld = Länge, Kopf 16, 1–2 Blöcke mit gültigen Offsets; TEX0 entfernen, MDL0 über die Blockoffsets suchen (nicht fest an 0x18 wie DSPRE); MDL0-Signatur; **genau ein Modell** im Dictionary (pokeplatinum `GF_ASSERT(numEntry == 1)`); **≤ 0xF000** Fehler (nicht DSPREs 61 000, das träfe 6 Vanilla-HGSS-Modelle); Länge % 4 ≠ 0 nur Warnung |
| Texturnamen gegen das NSBTX des Gebiets | **bedingt** | nur, wenn Task 0 zeigt, dass der Material-Leser für alle Vanilla-Karten keine fehlenden Namen meldet (gated). Sonst Hinweis „nicht geprüft“ und später |
| `.per` / `.mp` | ja | genau 2048 Bytes |
| `.bld` | ja | Länge n × 48; die vorhandene Regel `buildings_tail` und `validate` |
| `.bdhc` / `.bdhcam` | ja | Magic `BDHC`, ≤ 0x9000 (Pt belegt; für HGSS **[ungeprüft]**, deshalb als Warnung), Zähler im Kopf passen zur Länge; DP-BDHC abgelehnt |
| `.bgs` | nur HGSS | Signatur `0x1234`, Länge passt (`validate`); auf Platin abgelehnt mit Grund |
| ganze Map-`.bin` | ja | `MapFile::read` mit der eigenen Familie, sonst mit der anderen. Pt → HGSS: `BLANK_BGS` anlegen. HGSS → Pt: BGS weglassen, Warnung, falls er nicht leer war. Fremde Familie: Gebäude standardmäßig **nicht** übernehmen (Gebäude-IDs unterscheiden sich, DSPREs `LeavesOut`). DP-Karten abgelehnt |
| „Teile aus Karte N übernehmen“ | ja | wie Datei, aber die Quelle ist ein Store-Asset (auch ungespeichert). Das deckt „Permissions aus Karte N übernehmen“ ab |
| Export der Gegenstücke | ja | `.nsbmd` (MDL0 ohne TEX0, so wie es in der Karte steht), `.per`, `.bld`, `.bdhc`, `.bgs`, ganze `.bin`. Der Export macht das Gate in sich geschlossen, und DSPRE und PDSMS können die Dateien lesen |
| PDSMS-Ordner (`.pdsmap` → Matrixzellen) | **nein, Phase 6** | Es gibt keine echte Testdatei (weder in PDSMS noch in DSPRE noch in den Testprojekten). Die Ausrichtung von `XX_YY` ist nur am Code abgelesen, und es gibt zwei Formatvarianten. Das wäre nicht beweisbar. Ersatz in 3b.4: Im Dateidialog darf man mehrere Dateien desselben Stamms wählen (`route_00_01.nsbmd/.per/.bdhc`), die Endung bestimmt den Teil. Damit sind PDSMS-Exporte Karte für Karte importierbar |
| `.pdsts`, OBJ/glTF, PNG → NSBTX, BDHC-Erzeugung, Kollisionsableitung | nein, Phase 6 | wie in der Roadmap; die Zeile „OBJ über apicula-Pfad“ wird korrigiert (apicula liest nur) |

**Oberfläche.** Im Inspector der Matrixzelle bzw. Karte steht im Abschnitt „Karte“ die Taste „Teile importieren…“, dazu derselbe Eintrag in der Palette. Sie öffnet einen `Dialog` (keinen Assistenten): oben die Quelle (`Segmented` „Datei“ | „Karte im Projekt“), darunter je erkanntem Teil eine Zeile mit `Checkbox`, Größe und **Prüfliste** (je Prüfung ok, Warnung oder Fehler, mit Satz). Ein Teil mit Fehler lässt sich nicht ankreuzen. Unten steht immer der Hinweis „BDHC bleibt unverändert, Laufhöhen prüfen“, wenn das Modell ersetzt wird, aber kein BDHC dabei ist. Taste „3 Teile übernehmen“. **Keine eigene Vorschau:** Nach dem Übernehmen zeigt die Szene das neue Modell (der Modell-Cache hängt am Hash der NSBMD-Bytes, `export.rs:282`), und ⌘Z nimmt es zurück. Das Ergebnis im Dialog nennt die Prüfbefunde des Subjekts nach dem Import, zum Beispiel „Warp 2 löst nicht mehr aus (Tür-Kachel übermalt)“, mit „Zeigen“. Keine Plakette „Nicht rückgängig“, kein Schnappschuss: es entsteht nichts, was sich nicht zurücknehmen ließe. Ein Assistent „Karte importieren“ im Scharnier entfällt: die neue Karte entsteht wie heute über „Gebiet anlegen“ (Kopie), danach importiert man Teile hinein.

**Verworfen.** (a) DSPRE-Avalonia-Weg mit Baking aus `.pdsts`: AGPL, Phase 6. (b) Import über eine **neue** Karte (anlegen und füllen in einem Schritt): nicht rückgängig, und doppelt zum Gebiets-Assistenten.

**Preis bei Irrtum.** Ist die NSBMD-Prüfung zu streng (ein gültiges Fremdmodell hat zum Beispiel drei Blöcke), lehnt der Import gute Dateien ab. Der Ausweg kostet wenig: eine Regel lockern, ohne dass Daten verloren gehen. Ist sie zu lasch, landet ein Modell im ROM, das das Spiel per Assert anhält. Dagegen helfen die Undo-Möglichkeit und das Spielen im Gate. PDSMS-Nutzer mit vielen Karten importieren in 3b.4 Karte für Karte.

## 2. Umfang Prüfungen

**Entscheidung.** Wenige Prüfungen mit harter Regel: **„Fehler“ nur, wenn die Prüfung auf allen drei Vanilla-Projekten 0 Treffer hat**. Das beweist der gated Test `vanilla_checks`. Hat eine geplante Fehler-Prüfung Vanilla-Treffer, wird sie vor dem Ausliefern zur Warnung herabgestuft oder ihre Regel geschärft. Codes wie im Katalog.

| Schwere | 3b.4a |
|---|---|
| **Fehler** | `warp.dest-header`, `warp.dest-warp`, `warp.off-map`, `event.script-missing` (Figuren, Trigger), `event.trainer-missing`, `event.off-map`, `header.ref-range`, `header.wild-dangling`, `header.script-kind`, `script.message-missing`, `trainer.no-party` (genutzt von einem Event), `trainer.empty-species`, `trainer.id-range`, `matrix.map-range`, `matrix.header-range`, `map.model-invalid` (dieselbe Prüfung wie beim Import: Größe, ein Modell, Kopf), `text.encode`, `text.json-broken`, `asset.unreadable` |
| **Warnung** | `event.script-missing` für Schilder (Pt hat 1 Vanilla-Treffer; Task 0 sieht ihn an), `header.level-target` (1 Treffer, Header 186; Task 0), `event.trainer-blind`, `event.trigger-empty`, `event.same-spot`, `header.text-split`, `trainer.no-party` (ungenutzt), `trainer.no-moves`, `trainer.double-one`, `text.too-wide`, `text.three-lines`, `map.bdhc-size` (HGSS), `encounter.empty-slot`, `encounter.level-zero` (Regeln aus 3b.3 §10, jetzt projektweit) |
| **Hinweis** | `warp.never-fires` (10 Vanilla-Treffer), `encounter.min-max` |
| **bedingt** (nur, wenn Task 0 auf allen drei Projekten 0 misst) | `map.building-range`, `area.tileset-range`, `matrix.header-elsewhere` |
| **später** | `script.rotom-all` (Kosten [ungeprüft], Abgleich mit LSP und Compile); `script.trainer-target`, `script.id-range`, `script.flag-range`, `script.warp-target`, `event.common-missing`, `event.*-range` (brauchen eine eigene Typliste der Befehlsparameter und Flag-Bereiche, beides [ungeprüft]); `header.level-message` (Aufrufgraph); `header.no-map`; `data.*`, `trainer.name-length`, `trainer.texts`, `trainer.flag-capacity` (die 3b.3-Editoren verhindern das schon); `build.*` (Phase 4, Build-Graph); Texturnamen als Projektprüfung |
| **nie** | `warp.one-way` (118 Treffer, nur als Inspector-Zeile „Rückweg: …“), doppelte `local_id`, `trainer_type` ohne Trainer-Script, Figur auf blockierter Kachel außerhalb des Imports, `*.unused` |

**Textbreite.** Die Grenze wird ein Metadatenwert in `crates/g4-kernel/metadata/check_limits.yaml`: `dialog_line_px: 216`, mit `source` und `verified`. `text::validate` liest ihn über `Project::limits()` statt chatots `DIALOG_LINE_MAX_PX` (200, `text.rs:11`). Die Meldung lautet „Zeile 2 ist 231 px breit, ins Textfeld passen 216.“ Die Breitenprüfung gilt nur noch für **Dialogarchive** (Archive, die ein Header als `text_archive_id` nutzt). Beschreibungstexte haben andere Kästen und gehören zu den Daten-Editoren. **Plan-Task 0 von 3b.4a:** in pokeplatinum und pokeheartgold das Feld-Textfenster nachweisen (Kandidat `Window_Add(…, 27, 4, …)`, 27 × 8 = 216) und die Fundstelle als `source` eintragen. Findet sich kein Beleg, bleibt 216 mit `verified: false` und dem Histogramm als Quelle. Die Warnung `MessageLineTooLong` von rotom (200 px, nur Textliterale) wird beim Umwandeln im Kernel über ihre Warnungsart verworfen (nicht über den Text). Das Literal landet ohnehin im Archiv, und `text.too-wide` meldet es dort. Ob rotom die Warnungsart öffentlich macht, ist **[ungeprüft]**. Wenn nicht, wird über die Spanne dedupliziert.

**Ausnahmen sind Regeln, keine Ausgangslage.** In `check_limits.yaml` stehen, je mit Quelle: das Warp-Ziel `0x0FFF` (dynamisch, 6 Vanilla-Fälle), Trainer 0 (Dummy), Eventdatei 0 (geteilt), Script 0 bei Figuren (Dekoration), das Ende der Begegnungen bei Pt 0xFFFF und HGSS 0xFF. Die Ausgangslage (Frage 5) fängt nur echtes Rauschen von Warnungen und Hinweisen.

**Verworfen.** (a) Den ganzen Katalog (~60 Prüfungen): viele Bereiche [ungeprüft], hohe Fehlalarmquote, kein Beweis. (b) Nur die vier Roadmap-Beispiele: Importbefunde und Header-Indizes kosten fast nichts und sind auf Vanilla rauschfrei.

**Preis bei Irrtum.** Eine falsche Fehler-Prüfung kostet Vertrauen, blockiert aber nichts, weil Bauen nicht von Prüfungen abhängt. Fehlt eine Prüfung, gilt der Stand von heute: Der Fehler zeigt sich erst im Spiel. Ist 216 px falsch und die Wahrheit 200, übersehen wir Zeilen von 201 bis 216 px. Das Histogramm macht das unwahrscheinlich.

## 3. Architektur `checks`

**Entscheidung: zuerst ein Volllauf, ohne inkrementellen Pfad.** Die Schätzung im Bericht ergibt für alles außer Text und Script-Parsing 20 bis 80 ms in Rust. Wenn Task 0 das bestätigt (Ziel: Volllauf ohne Text unter 100 ms auf HGSS), läuft nach jeder Store-Änderung mit 300 ms Debounce **der ganze billige Teil** neu. Text- und Script-Prüfungen laufen je Datei und sind über Generation bzw. Quell-Hash gecacht. Damit gilt `inkrementell == voll` per Konstruktion, und es braucht keinen proptest und keine neuen RefIndex-Ziele für Header, Warp, EventFile und Co. in 3b.4. Die Schnittstelle ist trotzdem schon nach Subjekten geschnitten. Sollte der Volllauf zu langsam werden, ändert sich nur der Runner.

```
crates/g4-kernel/src/checks/
  mod.rs      Severity (+Info), Subject, Finding, Check, Registry, run_all, Snapshot
  limits.rs   check_limits.yaml laden (Werte mit Quelle)
  world.rs    warp.*, event.*, header.*, matrix.*, map.*   (Snapshot)
  scripts.rs  script.message-missing   (rotom-Parser, Port des Kerns von rotom-lsp message_refs.rs, MIT)
  text.rs     text.*                   (Text-Index mit Originaltext)
  data.rs     trainer.*, encounter.*   (3b.3-Codecs, encounter_rules)
  ignore.rs   g4-checks.json, Fingerprint
```

```rust
pub enum Severity { Error, Warning, Info }            // text::Severity bekommt Info, serde "info"
pub enum Subject { Header(u16), EventFile(u16), ScriptFile(u16), TextArchive(u16),
                   Trainer(u16), Encounter(u16), Matrix(u16), Map(u16), Project }
pub struct Finding { code: &'static str, severity: Severity, subject: Subject, key: String,
                     message: String, target: NavTargetDto, also: Vec<NavTargetDto>,
                     fixes: Vec<FixDto>, file: Option<(PathBuf, u32, u32)>, fingerprint: u64 }
pub trait Check: Sync {
    fn code(&self) -> &'static str;
    fn group(&self) -> Group;                          // Cheap | PerScript | PerArchive
    fn run(&self, snap: &Snapshot, out: &mut Vec<Finding>);
}
```

**Snapshot statt Lock.** Unter dem Store-Lock wird nur kopiert, was billig ist: Header-Tabelle, Member-Anzahlen, Eventdateien, Matrizen, Trainer und Begegnungen, die ungespeicherten Text-Dokumente. Das dauert wenige Millisekunden **[ungeprüft, Task 0 misst]**. Gelesen und geprüft wird ohne Lock auf einem Arbeitsthread (`spawn_blocking`). Hat sich die Store-Generation inzwischen geändert, wird das Ergebnis verworfen und neu angestoßen. Die 5 s kaltes Dekodieren der Texte laufen so nie unter dem Lock. Das ist die wichtigste Bauentscheidung; heute hält `AppState` den Store hinter einem Mutex **[ungeprüft, wie lange der Text-Index-Aufbau den Lock hält]**.

**Text-Index.** `Store::text_index()` hält heute `(archive, index, lowercase)`. Er bekommt zusätzlich den Originaltext. Das kostet ein paar MB, und die Breitenmessung braucht die echten Zeichen. `text.*` prüft je Archiv und wird beim Speichern bzw. Neuladen des Archivs ungültig, wie die Suche.

**rotom.** Nichts doppelt: Syntax, Befehle, Argumente, Sprungziele und `Module::Label` bleiben bei rotom und rotom-lsp. `script.message-missing` prüft nur, was rotom nicht kennt (Nachrichtenzahl des Archivs, das der Header der Scriptdatei nutzt). Scriptdateien, die kein Header nutzt (Common-Scripts), werden ausgelassen und als `skipped` gemeldet, nicht als „ok“.

**Vertrag Kernel ↔ Oberfläche.** `ProblemDto` bekommt optionale Felder, die alten Quellen bleiben unverändert:

```ts
interface ProblemDto {
  severity: "error" | "warning" | "info"; file: string; line: number; col: number; message: string;
  source: "compile" | "text" | "check";
  code?: string;                       // "warp.dest-warp"
  subject?: { kind: SubjectKind; index: number };
  key?: string;                        // stabil im Subjekt: "warp 2", "msg 12"
  label?: string;                      // "Eventdatei 0411 · Warp 2", statt file bei file == ""
  target?: NavTarget; also?: NavTarget[];
  fixes?: { id: string; label: string; irreversible: boolean }[];
  fingerprint?: string;
}
interface ChecksReport { findings: ProblemDto[]; ignored: number; ranMs: number; generation: number;
  running: boolean; progress?: { done: number; total: number; what: string };
  skipped: { code: string; why: string }[] }
```

`NavTargetDto` ist der Kernel-Spiegel von `navigation.ts` (header, matrixCell, area, building, event, textArchive, script) plus `trainer` und `encounter` aus 3b.3. `ProblemsStore` bekommt die Quelle `check`, die je Lauf als Ganzes ersetzt wird. `countsText` lernt „1 Hinweis“ bzw. „3 Hinweise“.

**Verworfen.** (a) Inkrementell über erweiterte RefIndex-Ziele mit `depends_on` je Prüfung, wie im Bericht: richtig für 10× größere Projekte, aber eine zweite Invariante und eine zweite Fehlerquelle, ohne gemessenen Bedarf. (b) Prüfungen im Frontend: die Brücke und `g4k` sähen sie nicht, und das Gate könnte sie nicht prüfen.

**Preis bei Irrtum.** Ist der Volllauf auf einem großen Hack langsamer als 100 ms, ruckelt das Prüflicht nach jedem Edit. Dann wird der Debounce länger (1 s), und der Runner bekommt Subjekt-Invalidierung. Das ist eine lokale Änderung, weil die Schnittstelle schon nach Subjekten geschnitten ist.

## 4. Beheben

**Entscheidung.** Ein Fix ist nur erlaubt, wenn sein Ergebnis eindeutig ist oder genau eine Auswahl braucht. Er wird als **ein** Changeset angewendet (`Store::transaction`), danach wird das Subjekt erneut geprüft. Bleibt der Befund, sagt ein Toast „Nicht behoben: …“, und die Zeile bleibt.

| Code | Fix | Rückgängig |
|---|---|---|
| `warp.dest-header` | „Ziel wählen…“ → `Picker` Header, dann Ziel-Warp | ja |
| `warp.dest-warp` | „Ziel-Warp wählen…“ → `Picker` der Warps des Ziels (Index, Kachel) | ja |
| `event.script-missing` | „Script #n anlegen“ (3b.2 `add_script` mit genau dieser Nummer) | **nein** (Quelle und Member werden geschrieben, wie heute) |
| `script.message-missing` bei N = Anzahl | „Nachricht N anlegen“ (`append_message`), danach öffnet sich der Text | ja |
| `text.too-wide`, `text.three-lines` | „Umbrechen“ (chatot `format::word_wrap(text, …, 216)`, vorhanden in chatot 0.8.1) | ja |
| `event.trainer-blind` | „Trainer-Typ 1 setzen“ | ja |
| alle anderen | nur „Öffnen“ (`goTo(target)`), zum Beispiel „Trainer öffnen“ auf dem Reiter Team | – |

**Dialog** (`Dialog`, Titel als Frage, wenn etwas nicht rückgängig ist: „Script #12 in Scriptdatei 0042 anlegen?“): darin der Satz, was passiert, bei Text Vorher und Nachher (neues Primitive `BeforeAfter`, Zeilen mit Pixelbreite), die Plakette „Nicht rückgängig“ für nicht rückgängige Teile, und eine Taste mit dem Verb („Umbrechen“). **Kein „Alle beheben“** in 3b.4.

**Verworfen.** (a) Automatische Fixes ohne Dialog: ein falscher Sammel-Fix zerstört Arbeit. (b) Fixes, die raten, wie „Auf Warp 0 umbiegen“: Das Ergebnis ist eindeutig, aber oft falsch. Die Auswahl mit Picker kostet einen Klick mehr.

**Preis bei Irrtum.** Ein schlecht gemachter Fix ist mit ⌘Z zurückzunehmen. Nur „Script anlegen“ ist es nicht, aber das legt höchstens ein leeres Script an.

## 5. Baseline und Ignorieren

**Entscheidung: härtere Regeln zuerst, dazu eine kleine Ausgangslage als Sammel-Ignorieren.** Fehler sind auf Vanilla per Regel und Test rauschfrei (Frage 2) und werden nie ausgeblendet. Für Warnungen und Hinweise gibt es **einen** Mechanismus: `g4-checks.json` im Projektordner:

```json
{ "version": 1, "disabled": ["warp.never-fires"],
  "ignored": [{ "code": "text.too-wide", "subject": "TextArchive:199", "key": "msg 12",
                "fingerprint": "9f3c…", "note": "Ausgangslage" }] }
```

`fingerprint` ist der Hash der Werte, die für die Regel zählen (Text der Nachricht; Warp-Ziel und Kachel). Ändern sich diese Werte, meldet die Prüfung den Befund wieder. Die Ausgangslage **altert** also nur dort, wo jemand die Daten anfasst, und dort ist das gewollt. **Ausgangslage:** Der erste Volllauf eines Projekts ohne `g4-checks.json` zeigt im Panel eine `Notice`: „Das Originalspiel hat 104 Warnungen und Hinweise. Ausblenden?“ Sie ist kein eigener Begriff, sondern schreibt dieselben `ignored`-Einträge mit `note: "Ausgangslage"`. Die Kopfzeile des Panels zeigt „12 ausgeblendet“ als Umschalter: Ausgeblendete Zeilen erscheinen gedämpft mit „Wieder melden“. „Prüfung abschalten“ steht im Zeilenmenü. Die Datei wird atomar geschrieben, ein Changeset ist sie nicht (Einstellungsdatei). DSPRE fasst sie nicht an, in Phase 4 geht sie mit ins Git. In Phase 4 kann der Vergleich mit dem Basis-ROM die Ausgangslage ersetzen.

**Gated Test `vanilla_checks`** (alle drei Projekte): 0 Fehler. Die Zahl der Warnungen und Hinweise je Code ist als Sollwert festgeschrieben, damit Regeländerungen auffallen. Task 0 misst HGSS (bisher nur Text gemessen).

**Verworfen.** (a) Mitgelieferte Vanilla-Baseline-Datei je Spiel: Sie hängt an Sprache und Dump, altert mit jeder Regeländerung und enthält Projektdaten. (b) Gar keine Ausgangslage: 90 bis 114 Breitenhinweise schon auf Vanilla machen das Prüflicht wertlos.

**Preis bei Irrtum.** Sind die Fingerprints instabil (etwa durch Feldreihenfolge), tauchen ausgeblendete Befunde wieder auf. Das nervt, ist aber harmlos. Ein Test prüft: zwei Volläufe hintereinander ergeben dieselben Fingerprints.

## 6. Oberfläche

- **Prüflicht** (Scharnier, 3c §5.5): zählt Fehler, Warnungen und Hinweise aus allen Quellen ohne Ausgeblendete („2 Fehler, 1 Hinweis“). Während eines Laufs zeigt es „Prüft … Texte 312 von 724“. Der Fortschritt kommt aus `checks_status` per Abfrage alle 250 ms, denn die Brücke hat keinen Push und kennt so jeden Befehl. Klick öffnet das Panel Probleme.
- **Panel Probleme** (Hand, 3c §7.4): Kopf mit „Erneut prüfen“ (Cache verworfen), „Zuletzt geprüft vor 2 min, 84 ms“, „12 ausgeblendet“, Filter nach Schwere und Quelle. Gruppen nach Bereich (Warps, Events, Header, Scripts, Text, Trainer, Begegnungen, Karten), darin die Zeilen nach Code. Eine Zeile zeigt Schwere-Symbol, `label` als `Reference` (`onOpen = goTo(target)`), Meldung und Code in Mono, dazu die Tasten „Beheben…“ (nur mit `fixes`) und das Menü „Ignorieren“ (dieser Befund / Prüfung abschalten).
- **Technik › Prüfen und Bauen**: dieselbe Tabelle, dazu Bauen und Starten. Build-Schritte kommen in Phase 4.
- **Am Objekt:** Der Inspector bekommt eine `Section` „Prüfungen“ für das gewählte Event, den Header, die Karte bzw. den Trainer (Filter nach `subject` und `key`, keine eigene Berechnung). Im Panel „An diesem Ort“ und in den Listen der Daten-Editoren trägt die Zeile das Schwere-Symbol. In der 3D-Szene bekommen nur Warp-Marker mit Fehler die Farbe `status/fehler`, eine Farbänderung über das vorhandene `setColorAt`, kein neues Symbol. Andere Marker kommen später.
- **Wann geprüft wird:** im Hintergrund nach dem Öffnen (nach dem Szenenaufbau), nach jeder Store-Änderung mit Debounce, nach Speichern, Kompilieren und Übernehmen, auf „Erneut prüfen“. Bauen wird nie blockiert. Der Build-Toast nennt die Fehlerzahl der Prüfungen mit „Probleme zeigen“.
- **Fehlende Primitives:** `BeforeAfter` (Fix-Dialog) und `CheckList` (Zeilen mit Zustand ok, Warnung, Fehler und Satz; für Import-Dialog und Fix-Dialog). `Table` aus 3b.3 reicht für das Panel, wenn es Gruppenzeilen kann. Sonst werden die Gruppen als `Section` mit `Table` gebaut, ohne neues Primitive. Ein Spinner im Prüflicht ist ein `Icon` mit `prefers-reduced-motion`.

**Verworfen.** (a) Eigenes Fenster wie DSPREs „Validation“: kein Sprung zum Ort, Doppel zum Panel. (b) Symbole an allen Szenen-Markern: Das braucht Sprites oder Shader und ist teuer für den Nutzen. Die Farbe am Warp reicht für die häufigste Fehlerart.

**Preis bei Irrtum.** Ist das Panel bei 100+ Zeilen träge, wird virtualisiert (wie 3b.3 bei über 50 ms). Bleibt die Ausgangslage ungenutzt, ist das Prüflicht dauerhaft gelb. Ein Gate-Punkt prüft, dass das Angebot erscheint.

## 7. Gate

**Szenario `scripts/agent/scenarios/pt-kleiner-hack.yaml`** (und `hg-kleiner-hack.yaml`). Es setzt `pt-trainer-begegnung.yaml` aus 3b.3b fort, alles als Brückenbefehle mit `--set`-Platzhaltern. **Neu in `g4emu`:** Pseudo-Änderung `[expect, {run: [befehl, {args}], path: "counts.byCode.warp.dest-warp", equals: 1}]`. `scenario.py` setzt den Befehl an dieser Stelle in den Batch und vergleicht nach dem Lauf die passende Zeile aus `bridge.jsonl` (der Batch druckt eine Zeile je Befehl, `batch.rs:89`). Scheitert ein Vergleich, endet das Szenario mit Exit 2 und nennt Pfad, Soll und Ist.

1. `accept_baseline {}` auf der frischen Kopie; `expect run_checks: counts.error == 0, counts.warning == 0` (Vanilla mit Ausgangslage = grün).
2. `apply_header_patch`, `create_area` (neuer Header, leere Eventdatei, Karte mit Gras), Treppe des Zimmers auf den neuen Header, Warp 0, umgebogen. **Rot:** `expect warp.dest-warp == 1` (der bekannte Fehler aus dem Handdurchlauf: kein Ankunfts-Warp). `add_event warp` (Ankunft) → **grün** `== 0`.
3. **Import:** `export_map_part {map: SRC, part: "model", path: "<Kopie>/import/src.nsbmd"}`, `export_map_part {… part: "permissions" …}`; dann `import_map_parts {map: NEU, source: {file: …}, parts: ["model", "permissions"]}`. SRC ist eine Karte desselben Gebiets (gleiches NSBTX) mit einem Weg, der in der Vorlage blockiert ist (Task 0 wählt SRC und Kachelfolge). Ein Gegenfall: `import_map_parts` mit einer synthetisch aufgeblähten Datei (> 0xF000) scheitert, `expect member_counts` und `dirty` bleiben unverändert.
4. **Trainer** (3b.3b): `create_trainer` × 2 → **rot** `expect trainer.no-party == 2` (beide schon als Events gesetzt) → `data_edit trainer` Party → **grün**.
5. **Texte:** eine Vanilla-Nachricht geändert, eine angefügt, mit absichtlich zu langer Zeile → **rot** `expect text.too-wide == 1` → `apply_fix {code: "text.too-wide", subject, key, fix: "wrap"}` → **grün**.
6. **Ereignis-Script:** Figur mit Script #N, das noch fehlt → **rot** `expect event.script-missing == 1` → `apply_fix … "create_script"` → `save_script` mit Inhalt (Textliteral, einmalig ein Item per Flag, danach anderer Text; die 3b.2-Mechanik) → **grün**.
7. `expect run_checks: counts.error == 0, counts.warning == 0`, `save`, `build_rom`.
8. **Beweis Modell im ROM:** neuer Schritt `verify: [rom-map-part, {rom: out.nds, map: NEU, part: model, equals_file: import/src.nsbmd}]` über einen `g4k`-Unterbefehl (Rust liest NARC und `MapFile`, Python parst nichts). Der Plan entscheidet zwischen `g4k` und einem Testbinary wie bei 3b.3a.
9. **Spielen:** `continue` → Treppe → `expect-map NEU` → `walk-to` über den Weg, den nur die importierte Kollision freigibt (Exit 0 nur, wenn der Import wirkt) → `shot route` → Figur ansprechen, `mash a`, `shot npc` → Trainer 1 und 2: `walk-until-battle`, `expect-battle S L`, `win-battle` (aus 3b.3b).

**Brückenbefehle, neu** (je in `COMMANDS`, `dispatch.rs` und im Tauri-Handler; der Test `commands_match_the_tauri_handler` zählt sie): `run_checks {scope?}`, `checks_status {}`, `apply_fix {code, subject, key, fix, choice?}`, `ignore_finding {code, subject, key, fingerprint}`, `unignore_finding {…}`, `accept_baseline {}`, `set_check_enabled {code, enabled}`, `inspect_map_import {map, source}` (Probelauf: Teile, Prüfliste, ändert nichts), `import_map_parts {map, source: {file: path[]} | {map: n}, parts, keepBuildings?}`, `export_map_part {map, part, path}`. `Roots` bekommt `read_path` und `write_path`: Import- und Exportpfade müssen unter den erlaubten Wurzeln liegen.

**Per Test-Umgebung (Agent):** beide Szenarien; Panel, Prüflicht, Fix-Dialog, Import-Dialog im Browser-Modus über Rollen und Beschriftungen; „12 ausgeblendet“ und „Wieder melden“; `vanilla_checks` und alle gated Import-Tests (Modell durch sich selbst = Original-Bytes für jede Vanilla-Karte; A in B und zurück; HGSS ↔ Pt-`.bin`; `.per`-Rundweg; Ablehnung > 0xF000 und zwei Modelle, synthetisch) in `scripts/gate.sh`. **Agent + Blick:** Bild der Route mit dem importierten Modell (texturiert, nicht grau), Text der Figur. **Hand:** DSPRE öffnet die Kopie nach dem Import und zeigt Modell und Kollision; nativer Dateidialog in der gebündelten App; ein HeartGold-Lauf von Hand, falls die HG-Kampfkalibrierung aus 3b.3b fehlt.

**Verworfen.** (a) Gate nur über Screenshots: wird nie automatisch rot. (b) Prüfen im Gate mit „alles grün“ ohne Ausgangslage: wegen 90+ Vanilla-Hinweisen nicht erreichbar, oder das Gate müsste die Regeln verbiegen.

**Preis bei Irrtum.** Findet Task 0 keine Karte SRC mit einem eindeutig neuen Weg, bleibt als Beweis für den Import nur der Bytevergleich im ROM plus „Agent + Blick“. Der Plan sagt das dann offen, statt das Kriterium umzudeuten.

## 8. Risiken

1. **Abhängigkeit von Ungebautem** (`Store::transaction`, `RefIndex`, Trainer, 3c.2-Panels). *Gegenmaßnahme:* Eingangskriterium „3b.3b gemergt“. 3b.4a beginnt mit Prüfungen, die nur Store und Header brauchen (Warps, Events, Header, Matrix, Text). Trainer- und Begegnungsprüfungen sind der letzte Kernel-Task.
2. **Fehlalarme zerstören Vertrauen** (HGSS ungemessen; Schild-Treffer, Header 186; Bereiche [ungeprüft]). *Gegenmaßnahme:* Fehler nur bei 0 Vanilla-Treffern, `vanilla_checks` gated mit festen Sollzahlen je Code, ungeprüfte Bereiche „später“, Bauen nie blockiert.
3. **Lock und Latenz** (5 s kaltes Dekodieren, Mutex um den Store). *Gegenmaßnahme:* Snapshot unter dem Lock, Arbeit ohne Lock, Generationsvergleich, Fortschritt per `checks_status`, Abbruch beim Projektwechsel. Task 0 misst Snapshot-Zeit und Volllauf auf HGSS, mit Budget 100 ms ohne Text.
4. **Importiertes Modell bricht das Spiel still** (Texturen, BDHC passt nicht, HGSS-Grenzen [ungeprüft]). *Gegenmaßnahme:* strenge Prüfung vor dem Schreiben mit HGSS-Grenzen so streng wie Pt, Undo, Hinweis „BDHC unverändert“, gated Rundwege über alle Vanilla-Karten, das Gate spielt die importierte Karte. Der Texturabgleich ist bedingt und wird nicht behauptet, solange er nicht bewiesen ist.
5. **216 px oder Fix-Semantik falsch** (Umbrechen ändert, wie DSPRE den Text sieht; „Script anlegen“ ist nicht rückgängig). *Gegenmaßnahme:* Beleg in pret als Task 0, Wert in Metadaten, Vorher/Nachher im Dialog, kein Sammel-Fix, Plakette „Nicht rückgängig“. Nach jedem Fix wird erneut geprüft, und „Nicht behoben“ wird ehrlich gemeldet.

**Plan-Task 0 (Sammlung):** Beleg für 216 px (Pt, HGSS); HGSS-Vanilla-Zahlen aller Prüfungen; Schild-Treffer in Pt und Header 186; Messung von Snapshot und Volllauf; MDL0-Modellzahl aller Vanilla-Karten (= 1); Material-Leser gegen das Area-NSBTX (bedingter Texturabgleich); Wahl von SRC und Weg für das Gate; ob rotom die Warnungsart `MessageLineTooLong` öffentlich macht; Dauer, die der Text-Index-Aufbau heute den Lock hält.
