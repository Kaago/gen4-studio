# Vorschlag C: Phase 3b.4 „Import und Prüfungen“

Architekt C. Neigung: Datenmodell, Invarianten und Prüfbarkeit zuerst, YAGNI bei der Oberfläche, und das Gate muss sich hart beweisen lassen. Gelesen habe ich ROADMAP (Worktree `phase-3c1`, Stand 5ea5864), die Specs 3c, 3b.3 (§1, §3–§16), 3b.2 (§4, §8–§10), beide Berichte, außerdem `text.rs` (`Severity`, `Problem`, `validate` mit 200 px), `commands/problems.rs` (`ProblemDto`), `problems.ts` (`ProblemsStore`), `store.rs` (`edit_map` mit Undo, `append_member` ohne Undo), `map.rs`, `scripts.rs::add_script` (schreibt und kompiliert sofort), `bridge/batch.rs` und `dispatch.rs` (`COMMANDS`), `g4emu/scenario.py` und `docs/agent-gate.md`.

Zwei Fakten aus dem Code prägen den Vorschlag:

1. **Karten-Edits sind schon rückgängig machbar.** `Store::edit_map` legt ein Changeset mit kompletten Map-Bytes an und ruft `MapFile::validate` auf. Jeder Import in eine *bestehende* Karte ist damit ein normaler Undo-Schritt. Er braucht keinen Schnappschuss und keine Plakette „Nicht rückgängig“.
2. **Ein Batch-Lauf der Brücke schreibt je Befehl eine JSON-Zeile mit dem Ergebnis** (`bridge.jsonl`). Damit lassen sich Erwartungen an Zwischenstände („vorher rot“) *nachträglich* gegen genau diese Zeilen prüfen, ohne den Batch umzubauen.

---

## 1. Umfang Import

**Entscheidung.** 3b.4 importiert fertige Binärteile in bestehende Karten, rückgängig machbar, mit harten Prüfungen. Gebaut und gebacken wird nichts.

| Quelle | In 3b.4 | Prüfung (Fehler blockiert den Teil, Warnung nicht) |
|---|---|---|
| NSBMD-Datei → Modell | ja | Magic `BMD0`, BOM `FFFE`, Version 2, Größenfeld = Länge, 1–2 Blöcke, Offsets im Bereich; MDL0 über die Blockoffsets suchen, nicht fest an 0x18. TEX0 wird entfernt (Kopf neu). **Genau ein Modell** im MDL0-Dictionary (Assert im Spiel). ≤ 0xF000 nach dem Entfernen (Fehler). Länge % 4 ≠ 0 ist eine Warnung. Materialnamen fehlen im Karten-NSBTX der Area-Data: Warnung mit Liste. Dazu der Hinweis „BDHC unverändert, Laufhöhen prüfen“. |
| ganze Map-`.bin` | ja, **auch Pt ↔ HGSS** | Erkennung über `MapFile::read(bytes, family)` erst mit der eigenen, dann mit der anderen Familie. Gehen beide oder keine: Ablehnung. Bei Familienwechsel wird der BGS-Block angelegt (leer: `34 12 00 00`) oder entfernt, Gebäude werden weggelassen (andere Modell-IDs, wie DSPREs `LeavesOut`), mit Warnung. Das Modell durchläuft dieselben Prüfungen wie oben. |
| `.per`/`.mp` | ja | genau 2048 Bytes, sonst Ablehnung ohne Änderung |
| `.bld` | ja | Länge % 48 == 0; Modell-ID < Gebäudezahl (`map.building-range`) |
| `.bdhc`/`.bdhcam` | ja, roh | Magic `BDHC`, ≤ 0x9000, Kopfzähler passen zur Länge. Ein DP-Dreiecksformat wird abgelehnt. |
| `.bgs` | nur HGSS | Signatur `0x1234`, `len` passt (`MapFile::validate`) |
| „Aus Karte N übernehmen“ | ja, je Teil (Gelände, Gebäude, Modell, BDHC, BGS) | dieselben Prüfungen wie für die Datei |
| Export der Gegenstücke | ja (`.nsbmd`, `.per`, `.bld`, `.bdhc`, `.bgs`, `.bin`) | keine. Das Gate braucht ihn (§7), und er erlaubt den Austausch mit DSPRE. |
| PDSMS | **nur der Export-Ordner**: Dateien `<stem>_XX_YY.{nsbmd,per,bld,bdhc,bdhcam,bgs}` werden nach `XX_YY` gruppiert und relativ zur linken oberen Zelle des gewählten Gebiets zugeordnet (Versatz einstellbar) | wie oben je Teil. Alle Karten zusammen sind **ein** Changeset (`Store::transaction` aus 3b.3a). |
| `.pdsmap`, `.pdsts`, OBJ/glTF, PNG→NSBTX, BDHC- und Kollisionserzeugung | nein, Phase 6 | – |

Warum `.pdsmap` nicht gelesen wird: Die Position steckt schon im Dateinamen (`MapMatrix.getMapName`). `areaindex` wählt im Studio ohnehin die Area-Data. Ob `XX_YY` absolut ist oder relativ zu `mapstart`, ist **ungeprüft** (Plan-Task 0, mit einem synthetischen PDSMS-Lauf des Forks oder am Code). Die Roadmap-Zeile wird korrigiert: „PDSMS-Export-Ordner (ohne Baking); OBJ/glTF erst mit dem NSBMD-Writer in Phase 6; apicula ist nur Lesepfad.“

**Neue Karten entstehen nicht durch den Import.** Wer eine neue Karte will, legt sie mit „Gebiet anlegen“ an (kopiert Karten, nicht rückgängig, wie heute) und importiert dann in die Kopie. So bleibt der einzige nicht rückgängige Pfad der vorhandene.

**Oberfläche.** Im Inspector der Matrixzelle gibt es den Abschnitt „Karte“ mit zwei Tasten: „Teile importieren…“ und „Teile exportieren…“. Dazu kommt ein Palette-Eintrag „Karte importieren…“ für die gewählte Zelle. Der Import ist ein `Dialog`, kein Assistent im Scharnier, denn er legt nichts an. Quelle als `Segmented`: Dateien | Ordner (PDSMS) | Andere Karte. Erkannte Teile stehen als Zeilen mit `Checkbox` „Übernehmen“ und ihrer Prüfliste als `Notice` (Erfolg, Warnung, Fehler; ein Fehler sperrt die Checkbox). Die primäre Taste heißt „N Teile übernehmen“. Danach kommt ein Toast „Karte 0612: Modell ersetzt. ⌘Z nimmt es zurück.“, und die Szene lädt das Modell neu (der GLB-Cache-Schlüssel hängt am Modell-Hash). **Eine eigene Vorschau gibt es nicht.** Übernehmen, in der Szene ansehen und mit ⌘Z zurück ersetzt eine Vorschau-Pipeline über apicula für fremde Dateien.

**Kernel.** `g4-formats/src/nsbmd.rs` bringt `NsbmdInfo::read` (Blöcke, MDL0-Dictionary, Material → Textur- und Palettenname) und `strip_tex0`, ohne Writer. `g4-kernel/src/import.rs` bringt `classify(bytes) -> Part`, `inspect(map, sources) -> ImportReport { parts: [{ part, origin, checks: [CheckLine{code, severity, text}] }] }` und `apply(map, chosen) -> ChangeId` über `edit_map`. Die Prüfzeilen benutzen **dieselben Codes** wie die Projektprüfung (`map.model-size`, `map.model-count`, `map.model-textures`, `map.bdhc-size` …). Import und Prüflicht sagen damit dasselbe.

**Verworfen:**
- (a) `.pdsmap`/`.pdsts` lesen und rendern. Dafür gibt es keine Testdaten (keine öffentliche `.pdsmap`), und Tilesets ohne Lizenz scheiden aus. Ohne Backen nützt das Lesen nichts.
- (b) Ein Import-Assistent im Scharnier mit Live-Vorschau. Das wäre doppelte Oberfläche für einen Vorgang, den Undo schon sicher macht.

**Preis bei Irrtum.** Fehlt doch der `.pdsmap`-Weg, ist das ein kleiner Nachtrag: ein Text-Parser von etwa 100 Zeilen, die Gruppierung bleibt. Erweist sich der Familienwechsel als fehlerhaft (zum Beispiel BDHC-Unterschiede Pt/HGSS), verliert man eine Option im Dialog, nicht den Import selbst.

## 2. Umfang Prüfungen

**Regel für 3b.4:** Aufgenommen wird nur, was auf Vanilla gemessen rauschfrei ist, oder dessen Rauschen in die Originalliste passt (§5). Dazu braucht jede Prüfung eine eindeutige Semantik und keine ungeprüften Wertebereiche.

**Fehler (F):** `warp.dest-header`, `warp.dest-warp`, `warp.off-map`, `event.off-map`, `event.script-missing`, `event.trainer-missing`, `event.common-missing` (nur wenn Task 0 null Treffer misst, sonst Warnung), `header.ref-range`, `header.wild-dangling`, `header.script-kind`, `header.level-target`, `script.message-missing`, `text.encode`, `text.json-broken`, `trainer.no-party` (nur wenn ein Event den Trainer nutzt, sonst Warnung), `trainer.empty-species`, `trainer.id-range`, `trainer.texts`, `matrix.map-range`, `matrix.header-range`, `area.tileset-range`, `map.building-range`, `map.model-size`, `map.model-count`, `map.bdhc-size`, `data.evo-target`, `data.learnset-limit`, `data.id-range` (Lernset-Attacke, Wild-Item), `asset.unreadable`.

**Warnung (W):** `event.trainer-blind`, `event.trigger-empty`, `event.same-spot`, `header.text-split`, `text.too-wide`, `text.three-lines`, `trainer.no-moves`, `trainer.level`, `trainer.double-one`, `trainer.flag-capacity`, `encounter.empty-slot`, `encounter.level-zero`, `map.model-textures`, `warp.arrival-stuck` (Task 0 misst; bei mehr als 10 Vanilla-Treffern wird es ein Hinweis).

**Hinweis (H):** `warp.never-fires` (nur „löst nie aus **und** niemand führt hin“), `encounter.min-max`, `encounter.empty-replacement`.

**Nie:**
- `warp.one-way` (118 Vanilla-Treffer). Bleibt eine Inspector-Zeile.
- `script.unused`, `trainer.unused`, `encounter.unused`, `event.shared-file`.
- doppelte `local_id`, `trainer_type ≠ 0` ohne Trainer-Script, Figur auf blockierter Kachel.
- `build.*`: Die Statusleiste zeigt den Build-Stand schon (3c §5.6).

**Später, mit Grund:**

| Prüfung | Grund |
|---|---|
| `event.hidden-flag-range`, `script.flag-range` | Flag- und Variablenbereiche sind ungeprüft. Sie kommen mit „Flags und Variablen“; die Roadmap führt das unter 3b.4, ich verschiebe es nach 3d. |
| `script.trainer-target`, `script.warp-target`, `script.id-range` | brauchen die eigene Liste `scrcmd_refs.yaml` je Familie |
| `script.common-missing`, `script.rotom-all` | Der LSP und der Compile decken den Großteil ab. Die Kosten (100–500 ms) sind ungeprüft. |
| `header.level-message` | braucht einen Aufrufgraphen. Eine Prüfung nur auf direkte Aufrufe täuscht falsche Sicherheit vor. |
| `header.no-map`, `matrix.header-elsewhere`, `*.name-length` | nicht gemessen; die Editoren verhindern es schon |

**Textbreite 216 statt 200.**
- Plan-Task 0 belegt die Breite des Feld-Textfensters in `pret/pokeplatinum` und `pret/pokeheartgold`: Suche nach dem Window-Template des Feld-Nachrichtenfensters, Breite 27 Kacheln. Fundstelle und Commit kommen in `metadata/check_limits.yaml`: `dialog_line_px: { value: 216, source: "...", verified: true }`.
- `text::validate(text, file, limits)` liest die Grenze von dort statt `DIALOG_LINE_MAX_PX`.
- Im Compile-Bericht werden rotoms `MessageLineTooLong`-Warnungen (200 px, nur Literale) verworfen und mit 216 px neu gemessen. Ein Test fixiert das: eine 210-px-Zeile ergibt keine Warnung, eine 220-px-Zeile genau eine.
- Den Fehler melden wir bei chatot und rotom (Konstante 200).
- Kann Task 0 die 216 nicht belegen, bleibt 216 mit `verified: false`, weil die Messung dafür spricht, und die Meldung sagt „etwa“.

**Ausnahmen sind Regeln, keine Ausgangslage.**
- `0x0FFF` (Warp-Ziel, dynamisch), Trainer 0 (Dummy), Figuren-Script 0 (Deko) und Eventdatei 0 sind Bedeutungen und keine Störgeräusche.
- Sie stehen mit Quelle in `check_limits.yaml` (`warp_dynamic_dest: 0x0FFF`, `trainer_dummy: 0`) und haben je einen Unit-Test.
- Was danach auf Vanilla übrig bleibt, kommt in die Originalliste (§5).

**Verworfen:**
- (a) Der volle Katalog von etwa 60 Prüfungen. Mehrere davon stützen sich auf ungeprüfte Bereiche; jede falsche Fehlermeldung kostet Vertrauen.
- (b) Nur die vier Prüfungen der Roadmap. Dann blieben die rauschfreien, billigen Referenzprüfungen ungenutzt, obwohl Importe und DSPRE-Fremdänderungen genau solche Fehler erzeugen.

**Preis bei Irrtum.** Eine falsche Fehler-Regel fällt im gated Test „Vanilla hat keine Fehler“ sofort auf (§3), nicht erst beim Nutzer. Eine zu laute Warnung landet in der Originalliste und stört nicht.

## 3. Architektur `checks`

**Datenmodell** (`crates/g4-kernel/src/checks/{mod,model,view,run,warps,events,headers,scripts,text,data,maps}.rs`):

```rust
#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Code { WarpDestHeader, WarpDestWarp, /* … */ }      // geschlossen; serialisiert als "warp.dest-header"
pub enum Severity { Error, Warning, Info }                    // text::Severity bekommt Info („Hinweis“)
pub enum Subject { Header(u16), EventFile(u16), ScriptFile(u16), TextArchive(u16), Trainer(u16),
                   Encounter(u16), Species(u16), Matrix(u16), Map(u16), AreaData(u16) }
pub struct Finding {
    pub code: Code, pub severity: Severity, pub subject: Subject,
    pub key: String,          // stabil im Subjekt: "warp 2", "obj 5", "msg 12", "slot 3"
    pub witness: Vec<u32>,    // die Werte, auf denen der Befund beruht (Ziel-Header, Ziel-Warp, Anzahl …)
    pub message: String,      // deutsch, mit Ausweg
    pub target: NavTarget,    // Kernel-Spiegel der Frontend-Union, plus Trainer und Begegnung
    pub fixes: Vec<FixKind>,
}
impl Finding { pub fn fingerprint(&self) -> u64 }  // FNV-1a über (code, subject, key, witness): stabil über Läufe und Versionen
```

**Reine Prüfungen mit automatisch aufgezeichneten Abhängigkeiten.** Statt eines handgeschriebenen `depends_on` je Prüfung liest jede Prüfung nur über `trait Reads`:

- `map(i)`, `event(i)`, `header_table()`, `count(kind)`, `text(i)`, `script_slots(file)`, `message_refs(file)`, `trainer(i)` …
- Die Umsetzung `RecordingView` (umhüllt `&mut Store`) schreibt jeden Zugriff als `Read::{Asset(AssetId), Count(AssetKind), ScriptSource(u16)}` in die Lesemenge der gerade laufenden Auswertung.
- Cache: `HashMap<(Code, Subject), Entry { findings, reads }>`.
- Nach einer Änderung liefert `Store::changed_since(gen)` (3b.3a Task 9) die geänderten AssetIds und Anzahlen. Ungültig wird jeder Eintrag, dessen Lesemenge sie schneidet. Subjektlisten werden neu aufgezählt, wenn sich eine gelesene Anzahl ändert.
- Script-Quellen liegen nicht im Store (`add_script`/`save_script` schreiben direkt). Für sie zählt ein Generationszähler je Datei in der `Session`, erhöht von `save_script`, `save_level_script`, `add_script`, `new_script_file` und `adopt_scripts`.

**Invariante `inkrementell == voll`** gilt hier per Konstruktion: Ein Eintrag ist eine reine Funktion seiner Lesemenge. Damit das so bleibt:
- Das Modul `checks::*` (außer `view.rs`) darf `Store` nicht importieren. Ein Test prüft das per `grep`.
- proptest über zufällige Edit/Undo/Redo-Folgen auf dem synthetischen Projekt aus `test_support`, danach gilt `incremental.sorted() == full.sorted()`.
- gated einmal je Testprojekt mit 50 zufälligen Edits.

Der RefIndex aus 3b.3 bleibt die Quelle für „Verwendet in“. Er wird um die neuen Ziele aus dem Bericht erweitert (`Header`, `Warp`, `ScriptSlot`, `Message`, `Map` …), weil `warp.never-fires` eingehende Warps braucht und die Schau sie zeigen soll. **Die Invalidierung der Prüfungen hängt aber nicht am RefIndex.** Ein fehlendes Ziel im RefIndex kann deshalb keinen veralteten Befund erzeugen.

**Text.** Die Prüfungen lesen über `Store::text_archive(i)`, also mit ungespeicherten Änderungen. Der Suchindex aus 3b.2 bekommt neben dem kleingeschriebenen auch den Originaltext und wird aus denselben geladenen Dokumenten gebaut, statt ein zweites Mal zu dekodieren. Der erste Volllauf läuft in **Stücken je Archiv**: Lock nehmen, ein Archiv prüfen, Lock freigeben. Fortschritt geht über einen `Channel`, abbrechbar beim Projektwechsel. So hält er den Store-Mutex nie 5 s.

**Stufen.**
- Billige Prüfungen (alles außer Text und Scripts, geschätzt 20–80 ms voll, < 1 ms inkrementell) laufen nach jedem Edit, Undo und Redo, mit 300 ms Debounce.
- Text und Scripts laufen nach Speichern oder Compile je Datei und beim Öffnen des Projekts im Hintergrund.
- „Erneut prüfen“ verwirft den Cache.
- **Der Build wird nie blockiert.** Nur Compile-Fehler blockieren, wie in 3b.2 §6.

**rotom.** Es gibt keine eigene Syntax-, Befehls-, Argument- oder Label-Prüfung. `script.message-missing` und `header.level-target` nutzen den rotom-Parser (Port von `message_refs.rs`, MIT, etwa 450 Zeilen) und die Slot-Tabelle `scripts::declared_slots`.

**Vertrag Kernel ↔ Oberfläche.** `ProblemDto` bekommt optionale Felder; bestehende Quellen setzen sie nicht:

```ts
interface ProblemDto {
  severity: "error" | "warning" | "info"; file: string; line: number; col: number; message: string;
  source: "compile" | "text" | "live" | "check";
  code?: string;                         // "warp.dest-warp"
  subject?: { label: string; tech: string };   // "Zweiblattdorf · Warp 2", "EventFile 0390 / warp 2"
  target?: NavTarget;                    // goTo
  fixes?: { id: string; label: string; irreversible: boolean }[];
  fingerprint?: string;                  // hex u64
}
```

- `ProblemsStore` bekommt die vierte Quelle `check` und ersetzt sie bei jedem Bericht als Ganzes (höchstens einige hundert Zeilen, kein Delta-Protokoll).
- `countsText` lernt „Hinweis/Hinweise“.
- Die Prüfungen erzeugen keine Zeilen für Compile- und Text-Probleme, die schon über `save_script` kommen. Doppelungen sind damit ausgeschlossen statt nachträglich entfernt.

**Kosten.** Billige Prüfungen geschätzt 20–80 ms voll. Text 0,8–5 s für die erste Dekodierung (gemessen), danach µs je Nachricht. Scripts: Parser 50–300 ms (**ungeprüft**, Task 0 misst). Der RefIndex-Aufbau mit den neuen Zielen soll unter 300 ms bleiben (Messung in Task 0).

**Verworfen:**
- (a) `depends_on` je Prüfung über den RefIndex, wie im Bericht: Das sind 40 handgeschriebene Abhängigkeitsfunktionen, von denen jede still veralten kann.
- (b) Immer voll prüfen: Ohne Text wäre das billig, aber nach jedem Speichern 1–5 s Text, und Hinweise würden flackern.

**Preis bei Irrtum.** Ist die Aufzeichnung zu grob (eine Änderung an der Header-Tabelle macht alle Warp-Einträge ungültig), kostet das einen Warp-Volllauf von 3–10 ms. Das ist harmlos. Umgeht eine Prüfung `Reads`, schlägt der proptest oder der Import-Test fehl.

## 4. Beheben

**Entscheidung.** Nur deterministische Fixes, die nichts auswählen müssen. Jeder Fix ist ein Changeset mit sprechendem Label.

| Code | Fix | Rückgängig |
|---|---|---|
| `warp.dest-warp` | „Auf Ziel-Warp 0 setzen“ (nur wenn das Ziel mindestens einen Warp hat) | ja |
| `event.script-missing` | „Neues Script anlegen und verknüpfen“ (`add_script`, dann Event-Script = neue Nummer) | Event ja, Script-Datei **nein** (`irreversible: true`, Plakette „Teilweise nicht rückgängig“) |
| `event.trainer-blind` | „Trainer-Typ 1 setzen“ | ja |
| `text.too-wide`, `text.three-lines` | „Umbrechen“ (chatot `word_wrap` mit 216 px und `\r` nach Zeile 2) | ja |
| `encounter.min-max` | „Tauschen“ | ja |

Alles andere bekommt nur „Zeigen“: Tür-Ziel, Trainer-Party und Nachricht anlegen brauchen eine Wahl des Nutzers, und der passende Editor mit Picker existiert schon. Die Regel „Text kürzen“ wird nicht automatisiert.

**Ablauf.**
1. `preview_fix { fingerprint, fix }` berechnet den Befund aus Code, Subjekt und Schlüssel neu und vergleicht den Fingerabdruck. Stimmt er nicht, kommt `CmdError { code: "stale" }` mit „Der Befund hat sich geändert, prüfe erneut.“ Sonst kommt `FixPlan { title, changes: [{ what, before, after }], irreversible }`.
2. `Dialog`, die Frage als Titel („Warp 2 auf Ziel-Warp 0 setzen?“), die Änderungen als Zeilen „Eventdatei 0390 · Warp 2: Ziel-Warp 3 → 0“. Beim Umbrechen zeigt ein `DSScreen` vorher und nachher. Tasten „Beheben“ und „Abbrechen“.
3. `apply_fix` führt in **einer** `Store::transaction` aus, prüft das Subjekt neu und liefert `{ label, resolved: bool, report }`. `resolved: false` heißt, der Befund steht noch da; er gilt nicht als behoben.

**Invarianten (Tests je Fix-Art):**
- Anwenden ⇒ der Befund ist weg, und kein neuer Befund entsteht außer den deklarierten.
- Undo ⇒ genau die alten Befunde mit gleichem Fingerabdruck sind zurück.
- Ein veralteter Fingerabdruck ⇒ `stale`, nichts geändert.

**Verworfen:**
- (a) Fixes mit eingebettetem Picker („Ziel wählen“) im Dialog: Das baut den Inspector ein zweites Mal.
- (b) „Alle beheben“ je Gruppe: Massenänderungen am Vanilla-Text gehören nicht in 3b.4.

**Preis bei Irrtum.** Ein fehlender Fix kostet einen Klick mehr über „Zeigen“. Ein falscher Fix würde stille Datenänderungen bedeuten; die Invariantentests und `stale` verhindern das.

## 5. Baseline und Ignorieren

**Entscheidung: keine Ausgangslage aus dem ersten Lauf. Stattdessen eine eingecheckte, reproduzierbare Originalliste plus Ausblenden je Befund.**

- **Originalliste** `crates/g4-kernel/metadata/checks_original/{CPUD,IPKD,IPKE}.yaml`: Code, Subjekt, Schlüssel und Fingerabdruck der Befunde eines unveränderten Spiels. Sie enthält keine Spielinhalte, nur Nummern.
  - Erzeugt wird sie mit `g4k checks --write-original <projekt>`.
  - Geprüft wird sie vom gated Test `original_list_matches`: Volllauf auf `_DSPRE_contents` == Liste.
  - **Sie darf keinen Fehler enthalten.** Ein Fehler auf Vanilla ist ein Regel-Fehler, und der Test schlägt fehl.
  - Sie altert sichtbar: Jede Regeländerung bricht den Test, und die Liste wird mit Absicht neu geschrieben, im Diff lesbar.
  - Ein Befund unterdrückt sich nur, wenn der Fingerabdruck stimmt. Ändert der Nutzer die Stelle, ändert sich der Zeuge, und der Befund gehört ihm.
  - Unbekannte Spielcodes oder ROM-Revisionen haben keine Liste und zeigen alles. Das sind dann nur Warnungen und Hinweise, also kein Schaden.
- **Ausblenden** in `g4-checks.json` im Projektordner (kommt in Phase 4 ins Git, DSPRE ignoriert die Datei): `{ "version": 1, "hidden": [{ "code", "subject", "key", "fingerprint", "note"? }] }`.
  - Mit Fingerabdruck: Ändert sich der Zeuge, erscheint der Befund wieder.
  - Fehler lassen sich nicht ausblenden. Ein Fehler, der Absicht ist, ist ein Regel-Fehler, und dafür gibt es die Regel-Ausnahmen in `check_limits.yaml`.
  - Geschrieben wird atomar und sofort, außerhalb von Undo; zurückholen geht mit „Wieder zeigen“.
  - Kein `disabled` je Code (YAGNI).
- **Anzeige:** Die Zähler im Prüflicht enthalten weder Original- noch Ausgeblendete. Am Fuß des Panels steht „12 aus dem Original · 3 ausgeblendet · Zeigen“; das blendet sie gedämpft ein.

**Verworfen:**
- (a) Ausgangslage beim ersten Volllauf übernehmen, wie Android-Lint: Das hängt vom Zeitpunkt ab, ist nicht reproduzierbar und versteckt auch echte Fehler eines schon veränderten Projekts.
- (b) Gar keine Liste, nur härtere Regeln: Bei 216 px bleiben 90 bis 114 Textzeilen je Spiel übrig. Eine noch härtere Regel würde echte Überlängen verschweigen.

**Preis bei Irrtum.** Unterscheiden sich Revisionen desselben Spielcodes, sieht der Nutzer einige Original-Warnungen. Das lässt sich mit einer Liste je Revision beheben (Schlüssel Spielcode plus CRC der arm9), ohne das Modell zu ändern.

## 6. Oberfläche

YAGNI: Es gibt keine neuen Flächen, nur bestehende Orte aus 3c.

- **Prüflicht** im Scharnier zählt `check` mit („2 Fehler, 1 Hinweis“). Während des Hintergrundlaufs steht „Prüft … 40 %“ mit Spinner; die billigen Ergebnisse stehen nach etwa 100 ms schon da. Ein Klick öffnet das Panel.
- **Panel Probleme** (3c §7.4):
  - Gruppen nach Bereich (Präfix des Codes: Warps, Events, Header, Scripts, Text, Trainer, Begegnungen, Daten, Karten) als `Section` mit Zähler.
  - Filter Schwere als `Segmented`; Kopf mit „Erneut prüfen“ und „Geprüft vor 2 min, 64 ms“.
  - Zeile: Symbol der Schwere, `subject.label` als `Reference` mit `goTo(target)`, Meldung, Menü mit „Zeigen“, „Beheben…“ (nur wenn `fixes` nicht leer) und „Ausblenden“ (nicht bei Fehlern). Fuß wie in §5.
- **Technik › Prüfen und Bauen** rendert dieselbe Tabelle, wie in 3c schon angelegt.
- **Am Objekt:** nur der Inspector. Ein `Notice` je Befund oben im Inspector des gewählten Events, Headers, Trainers oder der gewählten Zelle (Filter auf `target`), mit denselben Tasten.
  - Kein Warnsymbol in der Szene und keine Plaketten in Explorer-Zeilen. Das Scharnier, das Panel und der Inspector reichen für den Beweis.
  - Szenenmarker kosten Arbeit an Shader und Picking und kommen mit dem Overlay-Umbau in Phase 6.
- **Fehlende Primitives:**
  - `Progress` (bestimmter Balken): nur, falls 3c.2 ihn für `AdoptBox` nicht schon baut.
  - Das Symbol `info` in `icons.ts`.
  - Sonst reichen `Section`, `Notice`, `Badge`, `Dialog`, `Reference`, `Menu`, `Segmented`, `Checkbox`, `DSScreen` und `Table` (3b.3).

**Verworfen:**
- (a) Ein eigener Bereich „Prüfungen“ im Atlas: Das wäre eine doppelte Fläche zu Technik.
- (b) Befundmarker in der 3D-Szene: hohe Kosten, und das Gate braucht sie nicht.

**Preis bei Irrtum.** Wünscht der Nutzer Szenenmarker, hängen sie nur am vorhandenen `target`. Der Vertrag ändert sich nicht.

## 7. Gate

**Szenario `scripts/agent/scenarios/pt-hack.yaml`** auf einer Kopie von Platin DE, ein Aufruf. Danach `hg-hack.yaml` für HeartGold DE mit anderen Platzhaltern.

1. `adopt_scripts`, `apply_header_patch`, `create_area` (neue Route aus zwei Route-201-Karten mit hohem Gras, neuer Header `H`, leere Eventdatei `E`).
2. `export_map_parts { map: SRC, parts: ["model"], dir: "<kopie>/import" }`. `SRC` ist eine Karte mit derselben Area-Data, deren Modell sich von beiden Zielkarten unterscheidet (Task 0 wählt sie und prüft Eindeutigkeit).
3. `inspect_map_import { map: NEU0, files: ["import/SRC.nsbmd"] }`, erwartet: alle Prüfzeilen ohne Fehler, `map.model-textures` ohne Warnung. Dann `import_map_parts { map: NEU0, files: [...] }`.
4. `create_trainer` × 2 (0 Pokémon), `add_event` × 2 mit Trainer-Script, eine Figur mit Script #9 in einer Scriptdatei, die nur #1 hat, und der Ankunfts-Warp in `E` auf Ziel-Warp 1 (es gibt nur 0).
5. **Rot:** `run_checks {}` erwartet genau:
   - `trainer.no-party` × 2 (Fehler, die Trainer werden benutzt)
   - `event.script-missing` × 1
   - `warp.dest-warp` × 1
   - nichts sonst außer der Originalliste.
6. **Beheben:** `data_edit trainer` (Party je ein Pokémon eindeutiger Art und Stufe), `apply_fix` für `warp.dest-warp` (Auf Ziel-Warp 0) und `event.script-missing` (Script anlegen), `save_script` mit dem neuen Script: `Message "Ich lebe auf Route Neu!"`, dann der Kampfbefehl gegen Trainer 2 (Befehlsname aus 3b.3b Task 0). Dazu `edit_text_archive` (eine Vanilla-Nachricht ändern, eine anfügen).
7. **Grün:** `run_checks {}` erwartet `counts.error == 0`, `counts.warning == 0` und `counts.info == 0` ohne Original und ohne Ausgeblendete. `save`, `build_rom`.
8. **Spielen:**
   - `continue`, Treppe, `expect-map H`
   - **`expect-ram-bytes import/SRC.nsbmd@-64:64`** (neu): Der letzte 64-Byte-Block des Modells (Displayliste, die das Spiel beim Binden nicht anfasst) muss im Haupt-RAM vorkommen. Beweis, dass das importierte Modell geladen ist.
   - `walk-until-battle`, `expect-battle S1 L1`, `win-battle`
   - Figur ansprechen, `expect-ram-text "Ich lebe auf Route Neu!"` (neu, Zeichencodes über `g4k text-codes`), `mash a`, `expect-battle S2 L2` (beweist, dass das Ereignis-Script lief), `win-battle`
   - `shot` je Schritt für den Menschen.

**Neue Bausteine:**
- Scenario-Format: ein Edit darf ein drittes Element `{ expect: { "<json-pfad>": wert | { min, max } } }` tragen. `scenario.py` wertet es nach dem Batch gegen die Zeile in `bridge.jsonl` aus, mit Exit 2 und der Abweichung.
- `g4emu`-Schritte: `expect-ram-bytes FILE@OFF:LEN`, `expect-ram-text TEXT` (sucht die u16-Folge im Haupt-RAM; die Zeichencodes liefert der Kernel-CLI `g4k text-codes "<text>"` mit chatot, ohne Verschlüsselung).
- Aus 3b.3b genutzt: `expect-battle`, `walk-until-battle`, `win-battle`, `mash`.
- Optional (Plan wählt): `check-narc` aus 3b.3a vergleicht das Map-Member im gebauten ROM byteweise mit dem Import.

**Fehlende Brücken- und Tauri-Befehle** (je in `COMMANDS`, `dispatch.rs`, Tauri-Handler, `api.ts`):
- `inspect_map_import`, `import_map_parts`, `export_map_parts`
- `run_checks { full?: bool, progress? }`, `checks_status` (läuft, Fortschritt, Generation)
- `preview_fix`, `apply_fix`
- `hide_finding`, `unhide_finding`

**UI-Abdeckung beweisbar:** Ein vitest liest `pt-hack.yaml` und verlangt, dass jede dort benutzte `api.ts`-Funktion von mindestens einer Komponente außerhalb von Tests aufgerufen wird. „Komplett im Studio“ ist damit maschinell geprüft, nicht nur behauptet.

**Per Test-Umgebung (Agent im Browser-Modus):** Import-Dialog mit Prüfliste, ⌘Z nach dem Import, Prüflicht rot → Panel → „Beheben…“ → Dialog → grün, Ausblenden und Wieder zeigen eines Hinweises, „Erneut prüfen“ mit Fortschritt.

**Von Hand:** melonDS-Blick (Modell sieht richtig aus, kein grau-weißes Material), Lesbarkeit der Texte, gepackte App und DSPRE (nach bisheriger Praxis vom Nutzer ausgelassen).

**Verworfen:**
- (a) Screenshots als Beweis für Modell und Text: Sie werden nie automatisch rot.
- (b) Den Import im Gate nur mit „Modell aus Karte N übernehmen“ zeigen: Dann bliebe der Datei-Pfad (Klassifizierung, TEX0, Prüfungen) im Gate unbewiesen.

**Preis bei Irrtum.** Liegt der Modellpuffer nicht roh im RAM oder verändert das Spiel die Displayliste, fällt `expect-ram-bytes` weg. Rückfall ist der `check-narc`-Byte-Beweis im ROM plus `expect-map` plus Blick. Das Gate-Kriterium wird dann offen benannt, nicht umgedeutet.

## 8. Risiken

1. **RAM-Beweise im Gate sind ungeprüft** (Modell roh im RAM, Text als u16-Folge, Kampfbefehl im Script). Gegenmaßnahme: Plan-Task 1 kalibriert beide Sonden auf Vanilla (bekanntes Modell, bekannter Dialog) vor allen UI-Tasks, mit Rückfall wie in §7.
2. **Falsche Fehler-Regeln oder Rauschen.** Gegenmaßnahme: gated `original_list_matches` mit dem Verbot von Fehlern auf Vanilla; eine Positiv- und eine Negativ-Fixture je Code (der Test `every_code_has_fixtures` iteriert über `Code::ALL`); ein Snapshot der Code-Liste (Umbenennen bricht `g4-checks.json` und braucht deshalb eine Migration).
3. **Import verdirbt Karten** (HGSS-Grenzen 0xF000/0x9000 nur indirekt belegt, BDHC passt nicht zum neuen Modell). Gegenmaßnahme: Task 0 belegt die Puffer in pokeheartgold; byte-sichere gated Tests:
   - Selbstersatz ist Identität auf allen 2018 Vanilla-Karten
   - Tausch A → B ändert nur Kopf und Modell
   - `strip_tex0(embed(mdl, nsbtx)) == mdl`
   - Lesen mit der falschen Familie scheitert auf allen Vanilla-Karten (Grundlage der Erkennung)
   - Ablehnungen (> 0xF000, zwei Modelle, `.per` ≠ 2048) ändern nichts.

   Dazu kommt der Hinweis zum BDHC im Dialog, und das Gate läuft auf der importierten Karte.
4. **Lock-Konkurrenz und Kaltstart** (5 s Text). Gegenmaßnahme: billige Stufe zuerst, Text in Stücken je Archiv mit Lock-Freigabe, abbrechbar. Task 0 misst den Parser, den RefIndex und den HGSS-Volllauf; über 1 s je Stufe wird auf „nach Speichern“ verschoben.
5. **Umfang wandert** (die Roadmap ordnet 3b.4 auch Flags und Variablen und den Story-Graphen zu, 3b.3 auch „Löschen von Daten“, dazu `.pdsmap`). Gegenmaßnahme: eine wörtliche „Später“-Tabelle in der Spec:
   - Flags/Variablen und Story-Graph → 3d (Erzählen)
   - Löschen von Daten → nach 3b.4. Die Prüfungen fangen jetzt die Folgen verschobener Indizes; das Gate braucht kein Löschen.
   - `.pdsmap`/`.pdsts`/OBJ → Phase 6.

   Eine neue Komponente kommt nur mit einem Editor, der sie im selben Plan benutzt.

**Plan-Task 0 (ungeprüft, vor allem anderen):**
- 216 px in pret
- `0x0FFF` (Aufzug-Scripts)
- `XX_YY` in PDSMS-Namen
- HGSS-Puffergrößen
- Vanilla-Treffer von `warp.arrival-stuck`, `event.common-missing` und `map.model-textures`
- Kosten von Parser und RefIndex
- Wahl von `SRC` mit eindeutigem 64-Byte-Endblock
- Kampfbefehl in rotom
