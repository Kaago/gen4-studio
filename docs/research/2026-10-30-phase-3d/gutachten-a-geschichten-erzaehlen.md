# Gutachten A: Geschichten und Erzählen (Phase 3d)

Stand: 30.10.2026. Gutachter: Architektur-Review (nur lesend am Code, mit Messungen). Repo: `$HOME/Projekte/lab/editor/gen4-studio`, Branch `main` (`a5d6909`).
Datenbasis: frische APFS-Klone von Platin DE (`CPUD`) und HeartGold DE (`IPKD`) unter `Roms/_testkopien/3d-gutachten-a-{pt,hg}` mit der `.nds` daneben, je `g4k base assign` und `g4k text adopt` (Scripts mit rotom übernommen, Textprojekt eingerichtet), nach der Messung gelöscht. Die Originale unter `Roms/` blieben unberührt. HeartGold US ist nicht gemessen: Scripts, Events und Header sind laut Gutachten A der Phase 4 (§2.2) byte-gleich zu HeartGold DE, nur Texte und vier Karten weichen ab.

Werkzeug in diesem Ordner (gibt nur Zahlen und Namen von Befehlen und Kategorien aus, keine ROM-Inhalte):
- `gutachten-a-story.py` (Python 3, nur Standardbibliothek, Aufruf `python3 -I gutachten-a-story.py <projekt> <pt|hg> [--json]`): liest die Befehlsdatenbank `crates/g4-kernel/metadata/scrcmd/*.json`, die rotom-Quellen und Level-Scripts unter `expanded/scripts/`, die Eventdateien und Header unter `data/` und zählt Flags, Variablen, Level-Script-Einträge, Auslöser, Befehle je Knotenart, Fäden (Variablen mit mehreren Werten an mehreren Orten), den freien Pool und den Weg Figur → Script → Nachricht.

„Gemessen“ heißt: an den Testkopien mit diesem Werkzeug oder mit `g4k` ermittelt. „Geschätzt“ ist ausdrücklich gekennzeichnet. Zeiten stammen von einem Rechner, auf dem zur selben Zeit zwei weitere Gutachter arbeiteten (Last 5 bis 7,5); sie streuen um etwa ±50 %. `g4k text adopt` brauchte 38,0 s (Platin) und 44,1 s (HeartGold), davon 1,5 bis 1,8 s CPU; ein Lauf des Werkzeugs 2,4 bis 2,6 s je Spiel.

---

## 1. Anlass

Die Roadmap (Abschnitt „Phase 3d“) fasst 3d als „die Tiefe Erzählen mit Rezepten und Live-Vorschau der Assistenten, Flags und Variablen (…), Geschichten und Story-Graph, Löschen von Daten mit Referenzsperre“ und ordnet 3d nach Phase 4 und vor Phase 5 ein (Entscheidungen Phase 4, „Einordnung von 3d“; Gutachten C §7). Begründung damals: Geschichten und eigene Flag-Namen sind neue Entitäten ohne ROM-Gegenstück und gehören ins Textprojekt. Spezifikation, Schnitt und Gate stehen aus.

Das Design-System beschreibt das Ziel: Prinzip 3 „Erzählen, Bauen, Tüfteln“, Prinzip 7 „Rezepte statt leerer Leinwand … Assistenten erzeugen normale, weiter bearbeitbare Objekte“, die Komponenten „Story-Knoten“ (Start, Dialog, Bedingung, Kampf, Belohnung; Graph-Ansicht, Liste bleibt Standard), „Rezept-Taste“ und „Segmente“ (Erzählen, Bauen, Tüfteln), dazu die Screens 6, 9, 10, 11, 13, 14, 18 und 20 (Audit §7: alle „offen (3d)“ oder „teilweise“). Unter „Was die App umsetzt“ steht als Abweichung: „Die Tiefe Erzählen fehlt (nur Bauen und Tüfteln), ebenso die Live-Vorschau der Assistenten in der Schau und das Rezept-Ergebnis ‚nächster sinnvoller Schritt‘ als Taste.“ Die `.fig` war nicht erreichbar (OpenPencil-Server antwortet nicht); die Screens sind aus README und Audit beschrieben.

Dieses Gutachten beantwortet fünf Fragen: was eine Geschichte als Entität ist (§2.1, §3.1), welches Modell sie bekommt (§3.2), was die Tiefe Erzählen zeigt und welche Rezepte es gibt (§3.3), was für „Dialog bearbeiten“ und „Tür verbinden“ fehlt (§3.4) und wie man 3d für Geschichten und Erzählen schneidet (§5). Flags und Variablen als Entität (Gutachten B) und Löschen mit Referenzsperre (Gutachten C) werden nur dort berührt, wo dieses Gutachten etwas von ihnen braucht.

---

## 2. Befund

### 2.1 Wie Story-Fortschritt im Spiel technisch läuft

Gen 4 hat keine Story-Struktur im ROM. Fortschritt ist Zustand im Spielstand, den Scripts setzen und an vier Stellen prüfen:

| Mechanik | Wo sie steht | Was sie tut |
|---|---|---|
| **Flag** (1 Bit, dauerhaft unter `0x4000`, temporär ab `0x4000` bzw. Kartenflags am Anfang) | `SetFlag`, `ClearFlag`, `CheckFlag` + `GoToIf` in rotom-Quellen | einmalige Dinge: gesprochen, Belohnung erhalten, Ereignis gesehen |
| **Sichtbarkeits-Flag** einer Figur | Feld `hidden_flag` des Objekt-Events (`data/events/NNNN.json`) | gesetztes Flag = Figur fehlt beim Betreten der Karte; Gegenstandsbälle nutzen dafür ihr „erhalten“-Flag, Zwischensequenzen kartenlokale Flags, die beim Kartenwechsel zurückfallen |
| **Trainer-Flag** | `0x550 + Trainer-ID` bis `0x95F` (`data/trainer_create.rs`), gesetzt von der Engine nach dem Kampf, in Scripts über die Trainer-Nummer | besiegt oder nicht |
| **Variable** (16 Bit, dauerhaft `0x4000` bis `0x7FFF`, Script-lokal ab `0x8000`) | `SetVarFromValue`/`SetVar`, `CompareVarToValue` + `GoToIf` | Zustandsautomat: ein Wert je Abschnitt einer Handlung („Szenen-Variable“) |
| **Level-Script** `on_var_condition` | `var_conditions` in `expanded/scripts/NNNN.json` (`{var, value, script_id}`) | beim Betreten der Karte: ist `var == value`, läuft das Script einmal |
| **Level-Script** `on_transition`, `on_load`, `on_resume` | `header_entries` derselben Datei | beim Betreten, Laden, Zurückkehren: setzt meist Sichtbarkeits-Flags und Grafiken passend zum Stand |
| **Auslöser** (Coord-Event) | `var`, `value`, `script` im Event | Betreten der Fläche startet das Script, solange `var == value` |
| **Objekt-, Schild-Script** | `script` im Objekt- und BG-Event | Ansprechen bzw. Lesen startet das Script |

Befehlsdatenbank (gemessen; `gutachten-a-story.py`, Kategorien nach Namensmuster, erste Regel gewinnt):

| | Platin | HeartGold/SoulSilver |
|---|---|---|
| Flag-Namen (verschiedene Nummern) | 4.098 (4.097) | 2.252 (2.244) |
| davon mit sprechendem Namen (Sichtbarkeit, Trainer, versteckte Items, System, täglich, sonstige benannte) | **2.188** | **978** |
| davon `HIDE_…` (Sichtbarkeit) | 335 | 511 |
| davon Trainer | 726 | 0 (nur der Basiswert `TRAINER_FLAG_BASE`) |
| davon `UNK_…` oder nur hex benannt | 1.507 | 1.137 |
| davon `UNUSED`/`DUMMY` | **333** (137 `UNUSED`, 196 `DUMMY`) | **0** |
| davon temporär oder kartenlokal | 70 | 127 |
| Variablen-Namen | 315 | 394 |
| davon Szenen-/Zustandsvariablen (`…_STATE`, `SCENE_…`, `…TRIGGER…`) | **109** | **19** (`VAR_SCENE_…`) |
| davon sonst benannt | 108 | 51 |
| davon `UNK_…` | 15 | **251** |
| davon `UNUSED`/`DUMMY` | 24 | 0 |
| davon temporär/Script-lokal, Objekt-Grafik, Konstanten | 33, 16, 10 | 45, 16, 12 |

Die Zahl „137 als UNUSED markiert“ aus Gutachten C §7 betrifft die **Flags** von Platin, nicht die Variablen. pret hat Platin weit sprechender benannt als HeartGold: Von den Variablen, die HeartGold-Scripts wirklich benutzen, haben 168 von 263 keinen sprechenden Namen.

### 2.2 Was die Vanilla-Scripts zeigen (gemessen)

| | Platin | HeartGold |
|---|---|---|
| rotom-Quellen / Level-Script-Dateien (davon nicht leer) | 575 / 549 (298) | 497 / 468 (292) |
| Scripts (`script … #n`), Befehle gesamt, verschiedene Befehle | 2.856, 64.867, 780 von 1.111 | 2.886, 78.400, 787 von 1.001 |
| Level-Script-Einträge `on_transition` / `on_var_condition` / `on_load` / `on_resume` | 244 / 105 / 46 / 36 | 186 / 140 / 61 / 95 |
| `var_conditions` gesamt, verschiedene Variablen (davon Szenen-/Zustandsnamen) | 207, 59 (39) | 230, 71 (11; 56 `UNK`) |
| Objekt-Events, davon mit Sichtbarkeits-Flag | 3.555, **1.613 (45 %)** | 2.667, **1.108 (42 %)** |
| verschiedene Sichtbarkeits-Flags | 705 | 655 |
| Objekte mit Flag nach Art: dauerhaft `HIDE_…` / Gegenstand erhalten / temporär (kartenlokal, nur für eine Szene) / sonstige | 570 / 327 / 693 / 23 | 480 / 257 / 160 / 211 |
| Objekte mit Sichtlinie (Trainer) | 446 | 407 |
| Auslöser, verschiedene Variablen darin | 186, 72 | 195, 126 |
| Schilder (BG-Events), Warps | 682, 1.213 | 1.034, 1.317 |
| benutzte Flags (Scripts und Events), davon nur gesetzt / nur geprüft oder Sichtbarkeit | 1.119, 80 / 428 | 986, 52 / 308 |
| benutzte gespeicherte Variablen (`0x4000`–`0x7FFF`) | 211 | 251 |
| Flags, die an einem Ort gesetzt und an einem anderen geprüft werden | **149** | **159** |
| Variablen ebenso | 43 | 72 |
| `UNUSED`/`DUMMY`-benannt, aber in Vanilla benutzt | 27 Flags, 4 Variablen | – |

**Befehle je Knotenart** (Zeilen der rotom-Quellen; in Klammern: Dateien, die mindestens einen enthalten):

| Knotenart des Design-Systems | Befehle (Auswahl) | Platin | HeartGold |
|---|---|---|---|
| Dialog | `Message` (4.792) bzw. `NPCMsg` (4.556), Schild-, Varianten-, `MsgBoxExtern` | 5.108 (424) | 5.179 (403) |
| Bedingung | `CheckFlag`, `CompareVarToValue`/`ToVar`, `GoToIf`, `CallIf` | 11.552 (321) | 12.548 (331) |
| Kampf | `StartTrainerBattle` (97) bzw. `TrainerBattle` (123), Wild- und Legendärkämpfe | 142 (69) | 150 (66) |
| Belohnung | `AddItem` (21) bzw. `GiveItem` (42), `GiveBadge`, `GivePokemon`/`GiveMon`, `GiveEgg` u. a. | 59 (31) | 103 (57) |
| *(fehlt im Design-System)* Zustand setzen | `SetFlag`, `ClearFlag`, `SetVar…`, `SetTrainerFlag` | **4.025 (351)** | **5.268 (345)** |
| *(fehlt)* Bewegung | `ApplyMovement` | 2.239 | 3.372 |

Trainerkämpfe nennen den Trainer fast immer als Zahl (Platin 90 von 97, HeartGold 118 von 123), Nachrichten fast immer als Nummer im Textarchiv des Headers (Platin 4.999 von 5.043, HeartGold 4.407 von 4.786).

**Fäden.** Eine gespeicherte Variable mit mindestens drei Werten an mindestens zwei Orten (Header) ist ein Faden im Sinne von Screen 10:

| | Platin | HeartGold |
|---|---|---|
| Variablen mit Werten | 164 | 228 |
| **Fäden** (≥ 3 Werte, ≥ 2 Orte) | **59** (41 mit Szenen-/Zustandsnamen) | **62** (15) |
| Werte (= Schritte) aller Fäden zusammen | 272 | 311 |
| Werte je Variable: höchstens, Median | 16, 2 | 16, 2 |
| Orte je Variable: höchstens, Median | 22, 2 | 29, 2 |

**Freier Pool** für Geschichten des Nutzers (nirgends in Scripts, Level-Scripts oder Events benutzt, Name fehlt oder `UNUSED`/`DUMMY`/`UNK`; Flags im allgemeinen Bereich unter den Trainer-Flags `0x550`, ohne temporäre; Variablen ab `0x4030` bis zum Ende des Bereichs `VARS_END`):

| | Platin | HeartGold |
|---|---|---|
| Flags frei (allgemeiner Bereich) | **135** (96 davon `UNUSED`/`DUMMY`) | **152** (keins so benannt) |
| Flags frei hinter den Trainer-Flags (Systembereich, ohne HGSS-Tagesflags) | 1.360 | 247 |
| **Variablen frei** | **21** (20 davon `UNUSED`) | **83** (alle `UNK` oder ohne Namen) |

Nicht geprüft ist, ob die Engine (arm9, Overlays) Flags oder Variablen ohne Script-Verweis selbst setzt. Die Vorlage „Leere Region“ wählt ihr `done_var` schon nach derselben Regel („one no adopted vanilla script uses“, `metadata/templates.yaml`): Platin `VAR_UNUSED_0x406C`, HeartGold `VAR_UNK_4031`. Dass 27 Platin-Flags mit `UNUSED`/`DUMMY` im Namen in Vanilla benutzt werden, zeigt: der Name aus pret allein reicht nicht, der Pool muss aus der Nutzung kommen.

**Folgerung für den Story-Graph.** Die Handlung eines Gen-4-Spiels ist kein Baum aus Knoten, sondern ein Netz aus Zustand (Variablenwerte und Flags) und Prüfstellen über viele Dateien:
- **Start** entspricht einem Auslöser des Spiels: Figur ansprechen (Objekt-Script), Fläche betreten (Auslöser mit `var == value`), Karte betreten (`on_var_condition`), Schild lesen.
- **Dialog** sind Nachrichten im Textarchiv des Headers, angesprochen über ihre Nummer.
- **Bedingung** ist der häufigste Befehl überhaupt (rund 11.500 bis 12.500 Zeilen), meist Prüfung einer Szenen-Variable oder eines Flags.
- **Kampf** und **Belohnung** sind selten (rund 60 bis 150 Zeilen).
- Den eigentlichen **Fortschritt** macht „Zustand setzen“ (rund 4.000 bis 5.300 Zeilen), dazu Erscheinen und Verschwinden über Sichtbarkeits-Flags (42 bis 45 % aller Figuren tragen eins; dauerhaft für die Handlung, ohne Gegenstände und kartenlokale, 593 bzw. 691 Figuren). Beides fehlt in der Knotenliste des Design-Systems.
- Ein Story-Graph bildet im Spiel also ab: **Schritte = Werte einer Fortschrittsvariable** (oder ein Flag), **Kanten = „wo wird der Wert gesetzt“ → „was ist ab dem Wert möglich oder sichtbar“**, die Knoten Dialog, Kampf und Belohnung sind der Inhalt eines Schritts.
- 780 bzw. 787 verschiedene Befehle in Vanilla heißen: ein Knotenmodell kann vorhandene Scripts nicht verlustfrei abbilden, nur lesen und verweisen.

### 2.3 Was der Code heute trägt

**Kernel.**
- **Vorlagen-Rezepte** (`templates.rs`, `metadata/templates.yaml`, `metadata/templates/start_{pt,hgss}.rotom`): ein Rezept liest seine Eingaben, plant ohne Schreiben (`plan_empty_region`) und wendet an (`apply_empty_region`). Das Start-Script ist rotom-Text mit Platzhaltern (`{n}`, `{species}`, `{level}`, `{done_var}`, `{message}`), getrennt je Spielfamilie. Das Rezept legt eine Level-Script-Datei mit `on_var_condition` an (`write_source` mit `LevelScriptDoc`), hängt das Script an die Datei der Stadt an (`append_source`), übersetzt (`scripts::compile`) und schreibt Header, Matrix, Karte und Events in **einer** Store-Transaktion. Die Script-Dateien schreibt es vorher „at once“, ohne `FileStep`: ⌘Z nimmt Header, Matrix und Events zurück, die Scripts bleiben.
- **Script-Bausteine** (`scripts.rs`): `new_script_file`, `add_script`, `script_stub`, `append_source`, `write_source`, `declared_slots`, `resolve_script` (Event-Script-Nummer → Datei und Zeile; unter 2000 die Datei des Headers, ab 2000 die gemeinsamen Scripts über uxies Tabelle), `script_step` (ein `FileStep` mit rotom-Eintrag, bisher nur für „Auf Original zurücksetzen“).
- **rotom 0.7.0:** kennt `if/then/else/endif`, `while`, `match`, `alias`, Ausdrücke wie `CheckItem(ITEM_POTION, 1) == 1` (Lexer `src/compiler/token.rs`). Ein Textliteral (`Message "…"`) hängt rotom selbst als Nachricht an das Textarchiv des Headers an (`docs/formats/rotom_project.md`, „Textarchive“). `#include` und `#define` wertet rotom in DSPRE-Projekten **nicht** aus (`textproject/include.rs`, Kopfkommentar); Gutachten C §7 nahm das noch an. Der Weg für Namen sind JSON-Dateien im Konstantenordner: `.rotom/command_database/.g4-ids-<art>.json` mit `{"<Nummer>": "<NAME>"}`, geschrieben vom Kernel. Eigene Flag- und Variablen-Namen gehen denselben Weg.
- **Referenzindex** (`data/refs.rs`): Ziele sind nur `Species`, `Move`, `Item`, `Trainer`, `TrainerClass`, `Encounter` (Z. 64). Scripts sind seit 4.3b eine Quelle, aber nur für die Parameter aus `metadata/script_refs.yaml` (Trainer, Item, Spezies, Attacke). **Flags, Variablen, Scripts, Nachrichten und Events sind keine Ziele**; `hidden_flag`, Auslöser-`var` und `var_conditions` sind keine Quellen.
- **Register und IDs** (`textproject/registry.rs`): Register nur für `MAP_`, `TRAINER_`, `TRAINER_CLASS_`, `PLACE_` (`REGISTRY_KINDS`, Z. 24). „ID umbenennen“ (`textproject/rename.rs`) ersetzt jede JSON-Zeichenkette mit der alten ID unter `data/` und in den Projektdateien und jedes Bezeichner-Token in den rotom-Quellen. Eine neue Datei unter `data/` mit IDs zieht damit ohne neuen Code mit.
- **Arten des Textprojekts** (`textproject/arts.rs`): ein festes Enum mit 16 Arten, alle an Member gebunden. Ein unbekannter Ordner unter `data/` bleibt unberührt und wird als Warnung `textproject.unknown-art` gemeldet. Eine Art ohne Member (Geschichte) gibt es noch nicht.
- **Diff** (`diff/subject.rs`): eine Datei ohne Art ist `DiffSubject::File` und erscheint als Zeilen-Diff in der Gruppe „Projektdateien“. Ein Feld-Diff braucht ein eigenes Subjekt.
- **Undo** (`store.rs`): `FileStep` (Z. 502) mit `FileChange { before, after }` (auch Anlegen und Entfernen) und `RotomEntryChange`; Arten `Reset` und `IdRename`. Trainer anlegen ist eine Undo-Sperre (`barrier_transaction`, `trainer_create.rs:320`), weil die Member schon auf der Platte liegen.
- **Prüfungen:** `checks/scripts.rs` kennt je Script-Datei die Nachrichten mit Slot und Zeile (`MessageUse`, Z. 52; `script.message_params` in `check_limits.yaml`) und prüft sie gegen das Textarchiv des Headers (`script.message-missing`). Das ist der halbe Weg Figur → Text, aber nur `pub(crate)` und ohne Sprünge.

**App.**
- **Tiefe** (`ui/frame/shellState.ts:8`): `type Depth = "bauen" | "tuefteln"`, app-weit. `Hinge.tsx:47` `DEPTHS` hat zwei Einträge. Außerhalb der Tests vergleichen 40 Dateien die Tiefe direkt: 34 × `=== "bauen"`, 52 × `=== "tuefteln"`, 2 × `!== "tuefteln"`. Ein dritter Wert fiele heute an 34 Stellen in den Tüfteln-Zweig.
- **Geschichten** gesperrt: `App.tsx:123` `LOCKED_AREAS = ["geschichten"]`, Grund „Kommt mit Phase 3d“.
- **Inspector:** `FieldSpec` (`workbench/inspector/fields.ts:8`) und `EventFieldSpec` (`eventFields.ts`) kennen keine Tiefe; Tüfteln fügt den Abschnitt „Technisch“ hinzu (Spec 3c §7.7). Die Event-Felder enthalten **weder `hidden_flag` noch die `var`/`value` eines Auslösers** (`PATCH_KEYS`, `eventFields.ts`); neue Events bekommen beide als 0 (`src-tauri/src/commands/events.rs:310, 341`). Die zwei wichtigsten Story-Hebel sind in der Oberfläche heute weder sichtbar noch editierbar.
- **Assistenten:** „Gebiet anlegen“, „Raum anlegen“ (`workbench/wizards/`, im Scharnier mit Schritten), „Gebiet erweitern“ (Dialog `ExtendAreaDialog.tsx`), „Neues Spiel aus ROM“ und „Welt leeren“ (`project/`), „Trainer anlegen…“ (Dialog im Bereich Pokémon), dazu „Neuer Ort“ und „Begegnungsdatei anlegen…“. Die Logik von Gebiet und Raum liegt in der Tauri-Schicht (`src-tauri/src/commands/wizards.rs`, `create_area_impl` Z. 706, `create_room_impl` Z. 1078), nicht im Kernel; über MCP gibt es `create_area`, `extend_area`, `create_room`, `create_trainer`, `plan_empty_region`, `apply_empty_region` (`dispatch.rs`).
- **Szene** (`world/scene/SceneHost.ts`, `content.ts`): Marker kommen aus den Events (`setMarkers`), Auswahlrahmen für eine Zelle (`cellFrame`). Eine Schicht für Geplantes gibt es nicht; gestrichelte „+“-Zellen gibt es nur auf Weltkarte und Raumplan (`data-extend`).
- **Bausteine** (`ui/primitives/`): `DSScreen` (Textbox, Schild, Leer), `Picker` (mit Vorschau), `Reference` („Verwendet in“), `Segmented`, `PokemonCard`. Es fehlen `RecipeButton`, `StoryNode` und eine Graph-Bibliothek (in `package.json` weder `@xyflow/react` noch ein Layouter).

### 2.4 Der Weg von der Figur zum Text heute

1. Figur gewählt: Der Inspector zeigt „Script“ als Zahl (`EventFieldSpec` `script`, 0 bis 65535) mit „Öffnen“.
2. `openScriptNumber` (`workbench/inspector/scriptNav.ts`) → Kernel `resolve_script` → Script-Panel an der Zeile `script … #n`.
3. Im Script steht `Message 12` (Nummer). rotom-lsp liefert Inlay-Hinweise (`lspExtras.ts`); einen Sprung zum Text gibt es nicht.
4. Den Text ändert man im Bereich Texte: das Archiv des Headers (`text_archive_id`) suchen, Nachricht 12 wählen. Das Audit (Screen 14) nennt das „über Archiv und Nummer im Bereich Texte, kein Weg von der Figur“.

Gemessen: Von 3.555 bzw. 2.667 Objekten haben 1.485 bzw. 1.326 ein lokales Script (unter 2000). Für 1.481 bzw. 1.324 davon findet sich der Slot in der Datei des Headers, 1.339 bzw. 1.116 haben mindestens eine Nachricht im eigenen Script. 738 bzw. 701 springen oder rufen weiter (`GoTo`, `Call`, `CallIf`, gemeinsame Scripts), ihr Dialog liegt also verteilt. 66 bzw. 100 Nachrichten sprechen mehrere Figuren. Die übrigen Objekte haben gemeinsame Scripts (2000 bis 2999: 210 bzw. 34), Trainer-Scripts (3000 bis 6999: 417 bzw. 404) oder Scripts ab 7000 (Gegenstände u. a.); ihr Text ist nicht „der Dialog der Figur“.

---

## 3. Varianten

### 3.1 Was ist eine „Geschichte“?

Aus §2.2 folgt eine Definition, die zum Spiel passt und Vanilla lesen kann:

> Eine **Geschichte** ist ein benannter Faden: ein Fortschrittszustand (eine gespeicherte Variable, deren Werte die Schritte sind, oder ein einzelnes Flag), dazu die Stellen, die den Zustand setzen, und die Stellen, die von ihm abhängen (Level-Scripts, Auslöser, Script-Zweige, Sichtbarkeit von Figuren). Inhalt eines Schritts sind Dialoge, Kämpfe und Belohnungen, die die Stellen auslösen.

Daraus ergeben sich drei Regeln:
- **Eine Geschichte belegt eine Variable, nicht eine je Schritt.** Platin hat nur 21 freie Variablen; ein Wert ist 16 Bit, genug für jede Zahl von Schritten (Vanilla: höchstens 16).
- **Flags nimmt man für Einmaliges und Sichtbarkeit** (Pool 135 bzw. 152).
- **Der Graph ist zum großen Teil abgeleitet:** wer setzt den Wert k, wer prüft ihn. Was die Geschichte selbst speichert, sind Name, Reihenfolge, Notizen und die Zuordnung von Schritten zu Werten. Vanilla-Fäden erscheinen ohne Zutun (59 bzw. 62).

### 3.2 Drei Modelle

#### (a) „Geschichte ordnet und verweist“

Reine Projekt-Entität `data/stories/NNNN.json`; der Build erzeugt nichts daraus.

```json
{
  "id": "STORY_TESTFADEN",
  "name": "Testfaden",
  "notes": "",
  "progress": { "var": "VAR_UNUSED_0x406C" },
  "steps": [
    {
      "key": "s0", "value": 0, "kind": "start", "name": "Ankunft",
      "uses": [ { "map": "MAP_T01" }, { "levelScript": { "file": 502, "condition": 0 } } ]
    },
    {
      "key": "s1", "value": 1, "kind": "kampf", "name": "Herausforderung",
      "uses": [ { "trainer": "TRAINER_0042" }, { "event": { "file": 2, "object": 4 } },
                { "script": { "file": 2, "slot": 7 } }, { "message": { "archive": 23, "index": 5 } } ]
    }
  ],
  "flags": [ "FLAG_UNUSED_0x0000" ]
}
```

Die Anordnung im Graph liegt als Beifile `data/stories/NNNN.layout.json` daneben (gleicher Stamm, Muster der Beifiles aus 4.3a), damit das Ziehen von Knoten den Diff der Bedeutung nicht verrauscht.
- **Referenzindex:** `Story` wird eine Quelle. Neue Ziele `Flag(u16)`, `Var(u16)`, `Script{file, slot}`, `Message{archive, index}`, `Object{file, local_id}`. Die Kanten des Graphs kommen aus dem Index (Scripts, `hidden_flag`, Auslöser, `var_conditions` als Quellen für Flag und Variable; diese Quellen braucht Gutachten B ohnehin). Verweise einer Geschichte sind **schwach**: Sie sperren kein Löschen, sondern melden einen offenen Verweis (`story.dangling-ref`), denn eine Geschichte ist Dokumentation.
- **ID umbenennen:** Trainer-, Map-, Flag- und Variablen-IDs in der Geschichte zieht `rename.rs` ohne neuen Code mit (jede JSON-Zeichenkette unter `data/`). `STORY_…` braucht ein Register `data/ids/stories.json`. Script-Slots, Nachrichten- und Event-Indizes sind Nummern: Verschiebt sich eine Nachricht (Löschen in der Mitte), zeigt der Verweis daneben; eine Prüfung fängt das ab (Objekte über `local_id` statt Index verweisen, das ist stabil).
- **Diff (4.4):** heute schon als Zeilen-Diff „Projektdateien“; ein `DiffSubject::Story` mit JSON-Pfaden ist klein (S), weil `diff/json.rs` den Feld-Diff kann.
- **Undo:** Geschichte als Store-Art (`AssetKind::Story`, Ort `data/stories/NNNN.json`, kein Member, kein Build-Schritt), damit „ungespeichert“, ⌘S und Undo je Feld wie bei jeder anderen Entität gelten.
- **DSPRE-Ordner ohne Manifest:** keine Geschichten (die Entscheidung von Phase 4 gilt). Der Bereich zeigt den leeren Zustand mit „Als Textprojekt einrichten…“, lesend aber die Fäden aus dem Index.
- **Aufwand:** M (Kernel: Schema, Store-Art, Register, Index-Ziele und -Quelle, Diff-Subjekt, MCP `story_*`; geschätzt 1.500 bis 2.500 Zeilen mit Tests), Oberfläche M (Liste der Fäden, Graph lesend, Inspector des Schritts).
- **Risiko:** gering. Keine Bytes, der byte-gleiche Rebuild ist nicht berührt. Schwäche: Allein erzeugt das Modell nichts Spielbares, Einsteiger bekommen keinen Weg vom Graph ins Spiel.

#### (b) „Geschichte erzeugt Scripts“

Knoten werden vor rotom zu Quellen übersetzt; die Geschichte belegt ihre Flags und Variablen selbst.

```json
{
  "id": "STORY_TESTFADEN",
  "state": { "var": "auto" },
  "nodes": [
    { "key": "n1", "kind": "start", "trigger": { "talk": { "map": "MAP_T01", "object": 4 } } },
    { "key": "n2", "kind": "dialog", "text": { "de": "…" } },
    { "key": "n3", "kind": "bedingung", "if": { "flag": "auto" }, "then": "n4", "else": "n5" },
    { "key": "n4", "kind": "kampf", "trainer": "TRAINER_0042" },
    { "key": "n5", "kind": "belohnung", "item": "ITEM_POTION", "count": 1, "once": true }
  ],
  "edges": [ ["n1", "n2"], ["n2", "n3"] ],
  "allocated": { "var": "VAR_UNUSED_0x406C", "flags": ["FLAG_UNUSED_0x0000"] },
  "generated": [ { "file": 2, "slots": [7, 8], "hash": "…" } ]
}
```

- **Build:** neuer Schritt `stories` vor `rotom`, der in die Quellen der betroffenen Header schreibt. Ein Header hat genau eine Script-Datei; erzeugter und handgeschriebener Code müssen in **dieselbe** Datei. Das verlangt markierte Bereiche (`// <g4-story …>`), die der Build überschreibt.
- **Referenzindex:** Verweise sind hart (der Build bricht sonst). „ID umbenennen“ trifft die Geschichte, die erzeugten Quellen folgen beim nächsten Build.
- **Diff:** eine Änderung erscheint zweimal (Geschichte und erzeugtes Script).
- **Undo:** nur die Geschichte; die Quellen sind abgeleitet.
- **Was rotom trägt:** Text erzeugen kann der Kernel (Muster `start_pt.rotom` mit Platzhaltern, `append_source`), Texte als Literal ebenfalls. Es fehlt alles für Eigentum und Rückweg: Eine Hand-Änderung im erzeugten Bereich überschreibt der nächste Build. DSPRE übersetzt den Member, und die Übernahme aus DSPRE (Dekompilieren) verliert Kommentare und Marken. Vanilla lässt sich nicht als Knoten lesen (780 bzw. 787 Befehle).
- **Aufwand:** L+ (Übersetzer je Spielfamilie, Eigentumsmarken, Neuerzeugung, Pool-Verwaltung, Konflikte mit DSPRE, Round-Trip-Tests) zusätzlich zu allem aus (a).
- **Risiko:** hoch. Prinzip 7 (normale, weiter bearbeitbare Objekte) gilt nicht mehr, DSPRE parallel (Spec §3) bricht an der Marke, und der Pool (21 Variablen auf Platin) wird bei „auto“ je Knoten schnell leer.

#### (c) Mischform: Verweis-Modell plus Rezepte, die einmalig erzeugen (recommend)

Wie (a), dazu Rezepte, die beim Anlegen **einmal** normale Objekte schreiben: Script-Slots mit Literal-Nachrichten, Level-Script-Einträge, Auslöser, `hidden_flag` an Figuren, Flags und Variablen aus dem Pool mit eigenem Namen (Register aus Gutachten B). Danach sind es gewöhnliche Objekte, die Bauen und Tüfteln frei ändern. Die Geschichte merkt sich, welches Rezept einen Schritt angelegt hat, und verweist auf das Erzeugte:

```json
{
  "id": "STORY_TESTFADEN",
  "name": "Testfaden",
  "progress": { "var": "VAR_STORY_TESTFADEN" },
  "steps": [
    {
      "key": "s1", "value": 1, "kind": "fortschritt", "name": "Gespräch mit dem Wächter",
      "made": { "recipe": "story.talk_advances", "studio": "0.3.0" },
      "uses": [ { "object": { "file": 2, "local_id": 4 } }, { "script": { "file": 2, "slot": 7 } } ]
    }
  ],
  "flags": [ "FLAG_STORY_TESTFADEN_WAECHTER" ]
}
```

- **Graph:** Die Kanten kommen auch hier aus dem Index, nicht aus der Datei. Ändert jemand im Script den gesetzten Wert, zeigt der Graph das beim nächsten Lesen. Kanten, die nur notiert sind (der Index findet sie nicht), erscheinen gestrichelt mit „nur notiert“.
- **Rezepte** schreiben über dieselben Bausteine wie die Vorlage „Leere Region“ (`append_source`, `write_source`, Literal-Nachrichten), mit Text je Spielfamilie (`Message`/`NPCMsg`, `AddItem`/`GiveItem`, `StartTrainerBattle`/`TrainerBattle`) und strukturierter Syntax (`if … then … endif`) statt Sprungmarken.
- **Undo:** ein Rezept ist **ein** Undo-Schritt: Store-Transaktion für Events, Header und Geschichte plus `FileStep`s mit rotom-Eintrag für die Scripts (`scripts::script_step` gibt es). Damit wird auch die Lücke der Vorlage „Leere Region“ geschlossen. Neue Trainer bleiben eine Undo-Sperre, solange `trainer_create` Member schreibt.
- **Referenzindex, Umbenennen, Diff:** wie (a). Rezept-Ergebnisse sind gewöhnliche Script- und Event-Änderungen und erscheinen als solche im Diff.
- **Aufwand:** (a) plus Rezept-Rahmen (S bis M) plus je Rezept S. Zusammen L, darum in Teilphasen (§5).
- **Risiko:** mittel bis gering. Byte-Gleichheit ist unberührt (Rezepte ändern nur, was der Nutzer anlegt). Risiken liegen im Pool (21 Variablen auf Platin, Engine-Nutzung ungeprüft) und in den zwei Dialekten der Befehle; beides fangen gated Tests je Familie und ein g4emu-Szenario ab.

| | (a) verweist | (b) erzeugt | (c) Mischform |
|---|---|---|---|
| Bytes aus der Geschichte | keine | alle Story-Scripts | keine (Rezepte schreiben einmal) |
| Vanilla lesbar | ja (Fäden aus dem Index) | nein | ja |
| Prinzip 7 | ja | nein | ja |
| DSPRE parallel | ja | bricht an Marken | ja |
| Referenzen | schwach | hart | schwach |
| ID umbenennen | ohne neuen Code (plus `STORY_`) | ja, mit Neuerzeugung | wie (a) |
| Undo | Store-Art | nur Geschichte | Store-Art plus `FileStep` je Rezept |
| Aufwand | M | L+ | L (geteilt) |
| Risiko | gering | hoch | mittel bis gering |

### 3.3 Die Tiefe „Erzählen“

**Verhältnis zu `shellState.depth`.** Die Tiefe bleibt app-weit (Spec 3c §7.7) und bekommt den dritten Wert `"erzaehlen"`. Zwei Regeln verhindern, dass 34 Stellen still in Tüfteln fallen:
1. Helfer statt Gleichheit: `depthAtLeast(depth, "tuefteln")` für „zeige Technisches“ und `viewDepth(area, depth)` für „welche Ansicht hat dieser Bereich“. Ein Bereich ohne eigene Erzählen-Ansicht zeigt Bauen (Technik, Gegenstände, Attacken).
2. Alle 88 direkten Vergleiche in 40 Dateien werden umgestellt; ein Lint-Test (wie `check-styles.sh`) verbietet danach `depth === "…"` außerhalb von `shellState.ts`.

Das Segment im Scharnier hat dann drei Optionen; der gespeicherte Wert in `g4.shell.v1` liest unbekannte Werte als Bauen.

**Verhältnis zum Inspector.** `FieldSpec` und `EventFieldSpec` bekommen `from?: Depth` (die niedrigste Tiefe, die das Feld zeigt; Standard `"bauen"`). In Erzählen zeigt der Inspector nur Felder mit `from: "erzaehlen"` und ersetzt Zahlen durch Picker:
- **Figur:** Aussehen (Sprite-Picker), Bewegung (Picker mit Wörtern), Blickrichtung, Dialog (DS-Vorschau mit „Dialog bearbeiten“), Geschichte („Sichtbar ab Schritt …“ als Referenz auf `hidden_flag`), Trainer.
- **Tür:** Ziel als Ort und Tür, dazu „Tür verbinden…“.
- **Schild:** Text.
- **Auslöser:** Geschichte und Schritt statt `var`/`value`.
- **Gebiet und Raum:** Name, Ort, Aussehen, Musik als Picker.

Die Felder `hidden_flag` und Auslöser-`var`/`value` müssen dafür erst in Bauen und Tüfteln editierbar werden (heute fehlen sie, §2.3). Das ist eine Vorbedingung, die zu Gutachten B gehört.

**Was die Tiefe je Bereich zeigt und verbirgt:**

| Bereich | Erzählen zeigt | Erzählen verbirgt |
|---|---|---|
| Welt | 3D und Weltkarte; „In dieser Szene“ mit Figuren, Türen, Schildern, Auslösern (Wörter, keine Indizes); Werkzeugleiste als Rezept-Tasten (Neue Figur, Tür verbinden, Neuer Trainer hier, Gegenstand hier); Inspector reduziert | Karten- und Gebäudezeilen, Kartenteile importieren/exportieren, Header-Felder, Höhen, Script-Nummern, `local_id`, Rohwerte, „Raster“ |
| Geschichten | Fäden-Übersicht (Screen 10) als Standard, Graph eines Fadens (Screen 11) mit Schritten als Knoten, „Neue Geschichte“ und Rezepte für Schritte | Variablen- und Flag-Nummern (Bauen zeigt sie als Kennung, Tüfteln die Tabelle Screen 13 mit Nutzung) |
| Pokémon › Trainer | Rezept „Neuer Trainer“ (Screen 20): Klasse, Name, Team als Karten, Ort und Platz, Texte | KI-Bits, Rohdaten, Klassen-Nummern |
| Pokémon › Begegnungen | Erzählen-Layout (Screen 18): „Wer hier wartet“ mit Reglern häufig/selten je Tageszeit | Slot-Tabellen, Methodenreiter, Prozentwerte als Rohzahl |
| Pokémon › Pokédex, Attacken; Gegenstände | wie Bauen (DS-Profil, Kampfvorschau kommen später) | – |
| Texte | „Dialoge“ nach Ort und Figur statt nach Archivnummer, DS-Bildschirm; Steuerzeichen als Chips (Spielername) | Archiv- und Nachrichtennummern, Rohzeichen |
| Technik | wie Bauen | – |

**Rezepte (konkret).** „Nächster sinnvoller Schritt“ ist die Taste im Ergebnis (heute nennt das Ergebnis ihn nur als Text).

| Rezept | Quelle | Eingaben | Erzeugt (normale Objekte) | Nächster sinnvoller Schritt |
|---|---|---|---|---|
| Gebiet anlegen | vorhanden (Assistent) | Name, Karten aus Weltkarte oder Gebiet, Lage, Ort, Aussehen | Matrix oder Zellen, Karten-Kopien, Header (Patch einmal) | „Tür verbinden“ oder „Neue Figur hier“ |
| Raum anlegen | vorhanden | Name, Vorlage, Eingang (Kachel), Ort | Karte, Header, Eventdatei, Warp-Paar | „Neue Figur hier“ |
| Gebiet erweitern | vorhanden (Dialog) | Zelle, Kopie oder Karte | Karte, Zelle der Matrix | „Gelände anpassen“ (später) |
| Neues Spiel aus ROM | vorhanden | ROM, Vorlage, Textprojekt, Git | Projekt | „Welt leeren“ oder „Erste Geschichte“ |
| Welt leeren | vorhanden | – | Grenzkarte, leere Header, Level-Script, Start-Script | „Gebiet anlegen“ |
| Trainer anlegen | vorhanden (Dialog) | Klasse, Name | Trainer (Undo-Sperre) | „In der Welt platzieren“ |
| **Neue Figur** (Screen 9) | neu | Ort (Kachel aus der Szene), Aussehen, Bewegung, erster Dialog | Objekt-Event, Script-Slot mit `Message "…"` (Literal), Nachricht im Archiv | „Dialog bearbeiten“ oder „Zu einer Geschichte“ |
| **Tür verbinden** (Screen 6) | neu, aus „Raum anlegen“ gelöst | Tür A (gewählt oder Kachel), Ziel-Ort, Tür B (vorhanden oder Kachel), „Rückweg anlegen“ | ein Warp oder ein Warp-Paar mit gegenseitigen Zielen | „Hier spielen“ an Tür A |
| **Neuer Trainer hier** (Screen 20) | neu, setzt „Trainer anlegen“ und „In der Welt platzieren“ zusammen | Klasse, Name, Team, Kachel, Sichtweite, Texte vor und nach dem Kampf | Trainer, Trainertexte, Objekt-Event mit Script 3000 + ID − 1 und Sichtlinie | „Hier spielen“ |
| **Wilde Pokémon hier** (Screen 18) | vorhanden als „Begegnungsdatei anlegen…“, Erzählen-Layout neu | Pokémon, häufig/selten, Tageszeit | Begegnungsdatei oder Abspaltung | – |
| **Neue Geschichte** | neu | Name | Geschichte, Fortschrittsvariable aus dem Pool mit eigenem Namen | „Ersten Schritt anlegen“ |
| **Gespräch schaltet weiter** | neu | Figur, Geschichte, ab Schritt k | Script-Zweig `if VAR == k then … SetVar k+1 endif`, Schritt k+1 | „Figur erscheint ab Schritt“ |
| **Figur erscheint/verschwindet ab Schritt** | neu | Figur, Geschichte, Schritt | Flag aus dem Pool, `hidden_flag`, `SetFlag`/`ClearFlag` im Script des Schritts, `on_transition`-Eintrag | „Hier spielen“ |
| **Szene beim Betreten** | neu | Ort, Geschichte, Schritt, Dialog | Level-Script-Eintrag `on_var_condition` (oder Auslöser-Fläche), Script mit Literal-Nachricht | „Hier spielen“ |
| **Belohnung einmalig** | neu | Figur, Item oder Pokémon | Flag, `if !CheckFlag … AddItem/GiveItem … SetFlag endif` | – |

Kein Rezept erzeugt Story-Code beim Build. Jedes ist ein Kernel-Rezept mit `plan` und `apply` (Muster `plan_empty_region`/`apply_empty_region`), über MCP als `recipe_plan` und `recipe_apply` erreichbar (Rückfrage, wenn etwas nicht rückgängig ist). Die Logik von Gebiet und Raum zieht dafür aus `src-tauri/src/commands/wizards.rs` in den Kernel, damit `g4k` und Gates sie ohne App nutzen.

**Live-Vorschau in der Schau (gestrichelte Zellen).** Was die Szene dafür braucht:
1. **Einen Plan als Daten:** Jedes Rezept liefert vor dem Anlegen ein `DraftDto`: geplante Zellen (Matrix, x, y, Quellkarte), geplante Events (Art, Position, Aussehen, Ziel), geplante Nachrichten. Der Plan kommt aus `recipe_plan` im Kernel, nicht aus dem Formular, damit Vorschau und Ergebnis nicht auseinanderlaufen.
2. **Eine Schicht „Entwurf“ in `SceneHost`:** eine eigene `THREE.Group` neben `markers` und `cellFrame` mit gestrichelten Zellrahmen (`LineDashedMaterial`), halbtransparenten Markern (Sprite der gewählten Grafik bei 50 % Deckkraft) und optional dem Modell der Quellkarte an der Zielzelle. Sie wird mit einer Prop `draft` gesetzt und vom Laden der Szene nie überschrieben. `[data-draft]` am Panel und eine Zählung über die Test-API machen sie prüfbar.
3. **Ziele außerhalb der Szene:** Ein neuer Raum ist eine eigene Matrix und nicht in der gezeigten Szene. Für ihn zeigt der Raumplan die gestrichelten Zellen, die 3D-Szene nur die Tür. „Tür verbinden“ zeigt beide Enden als Entwurf, das fremde in der Weltkarte.
4. **DS-Bildschirm:** Dialog-Rezepte zeigen den Text seitenweise in `DSScreen` (vorhanden).

**Mindestumfang in 3d (recommend):**
- die Tiefe Erzählen mit Helfern und reduziertem Inspector für Figur, Tür, Schild, Auslöser, Gebiet und Raum;
- der Rezept-Rahmen im Kernel mit MCP und die Entwurf-Schicht für die vorhandenen Assistenten (Gebiet, Raum, Erweitern) und die neuen Welt-Rezepte;
- „Neue Figur“, „Tür verbinden“, „Neuer Trainer hier“ und der Dialog-Editor an der Figur;
- Geschichten: Fäden lesen, Graph lesend mit Notizen, die Story-Rezepte „Neue Geschichte“, „Gespräch schaltet weiter“, „Figur erscheint/verschwindet ab Schritt“, „Szene beim Betreten“, „Belohnung einmalig“.

**Später:**
- Graph frei bearbeiten mit Knoten, die Code erzeugen (Modell b);
- Bewegungen und Zwischensequenzen (`ApplyMovement`) als Knoten;
- Erzählen-Layout für Läden;
- DS-Varianten (Pokédex-Profil, Kampfvorschau), Kampfsprites;
- Spielmodus mit Live-Flags (eigene Phase, braucht Lesen des Spielstands im Emulator);
- Rezept „Gegenstand verstecken“ (braucht die Tabelle der versteckten Items je Spiel).

### 3.4 „Dialog bearbeiten“ (Screen 14) und „Tür verbinden“ (Screen 6)

**Dialog bearbeiten, was fehlt:**
1. **Kernel `dialog_of(header, event)`:** Script-Nummer → `resolve_script` → Slot → die Nachrichten des Slots in Reihenfolge. Je Nachricht: Archiv (`text_archive_id` des Headers bzw. das Archiv von `MsgBoxExtern`), Index oder Literal, Befehl, Zeile, Zweig (welche Bedingung davor steht) und die Zahl der Scripts und Figuren, die dieselbe Nachricht nutzen. Grundlage ist `MessageUse` aus `checks/scripts.rs`, erweitert um das Verfolgen von `GoTo`/`Call` innerhalb der Datei (738 bzw. 701 Figuren brauchen das). Gemeinsame Scripts (ab 2000) und Trainer-Scripts zeigt er lesend mit dem Grund.
2. **Oberfläche:** Abschnitt „Dialog“ im Inspector der Figur (in Erzählen oben) mit `DSScreen`-Seiten und „Dialog bearbeiten“. Der Editor ist ein Panel der Hand mit den Nachrichten als Folge, Bearbeiten über den vorhandenen Text-Store (Undo, ⌘S, Zeilengrenzen wie in Texte) und der Plakette „Geteilt mit N Figuren“ bei geteilten Nachrichten (66 bzw. 100 in Vanilla) mit „Für diese Figur abspalten“ (neue Nachricht, Script zeigt darauf; das Muster kennt „Für diesen Ort abspalten“).
3. **Rückweg Texte → Figur:** „Verwendet in“ an einer Nachricht braucht `Target::Message` im Referenzindex.
4. **Neue Figuren** bekommen Literal-Nachrichten. rotom legt sie selbst im Archiv an; der Dialog-Editor bearbeitet danach die Nachricht im Archiv wie jede andere. Offen ist, ob rotom ein geändertes Literal bei erneutem Compile an denselben Index schreibt (§6).

**Tür verbinden, was fehlt:**
- Heute: Werkzeug „Tür“ setzt einen Warp. Ziel ist `dest_header_id` (Header-Picker) plus `dest_warp_id` (Zahl). Den Rückweg setzt man von Hand am anderen Ort. Das Warp-Paar entsteht nur im Assistenten „Raum anlegen“ (`create_room_impl`).
- Fehlt:
  - ein Kernel-Rezept `connect_doors` mit `plan` (beide Enden, vorhandene Warp-Indizes, Kollision der Kacheln, Prüfung `warp.*` vorab) und `apply` (ein Undo-Schritt über zwei Eventdateien);
  - im Inspector der Tür „Tür verbinden…“ und die Anzeige des Gegenstücks („→ Haus 1, Tür 0, führt zurück“ oder „führt nicht zurück“);
  - in der Szene der Entwurf am fremden Ende über Weltkarte bzw. Raumplan.
- Die Bausteine (Warp anlegen, Ziel setzen, Prüfungen der Warps aus 3b.4a) sind da; der Aufwand ist S bis M.

---

## 4. Empfehlung

1. **Geschichte als Entität (Frage 1):** Eine Geschichte ist ein benannter Fortschrittszustand (eine Variable, Schritte = Werte; oder ein Flag) mit den Stellen, die ihn setzen und prüfen. Der Graph bildet im Spiel „Wert gesetzt in … → möglich oder sichtbar ab …“ ab.
   - Die Knotenarten des Design-Systems werden um **Fortschritt** (Zustand setzen) und **Erscheinen/Verschwinden** ergänzt (gemessen 4.025 bzw. 5.268 Zeilen gegen 59 bzw. 103 für Belohnung; 593 bzw. 691 Figuren hängen dauerhaft an einem Sichtbarkeits-Flag).
   - Dialog, Kampf und Belohnung sind der Inhalt eines Schritts. Bewegung kommt später.
2. **Modell (Frage 2): recommend (c) Mischform.** Grundlage ist die Projekt-Entität `data/stories/NNNN.json` mit Beifile `.layout.json`, einer Store-Art `Story`, dem Register `STORY_` und schwachen Verweisen im Referenzindex; der Build erzeugt nichts. Die Kanten kommen aus dem Index. Dazu Kernel-Rezepte, die einmal normale Objekte schreiben (Scripts mit strukturierter Syntax und Literal-Nachrichten, Level-Script-Einträge, `hidden_flag`, Flags und Variablen aus dem Pool), als ein Undo-Schritt mit `FileStep` für Scripts. Eine Geschichte belegt **eine** Variable. (b) nicht, auch nicht später als Standard.
3. **Tiefe Erzählen (Frage 3):**
   - dritter Wert von `shellState.depth`, app-weit, mit `depthAtLeast`/`viewDepth`; alle 88 Vergleiche umgestellt, ein Lint dagegen; Bereiche ohne eigene Ansicht zeigen Bauen;
   - `FieldSpec.from` steuert den reduzierten Inspector;
   - Rezepte als Kernel-Module mit `plan`/`apply` und MCP; Live-Vorschau über ein `DraftDto` aus dem Plan und eine Entwurf-Schicht in `SceneHost`;
   - Mindestumfang wie in §3.3, das Übrige später.
4. **Dialog und Tür (Frage 4):**
   - Kernel `dialog_of` auf Basis von `MessageUse` mit Sprungverfolgung, Dialog-Abschnitt und -Editor an der Figur über den Text-Store, Plakette „geteilt“ mit Abspalten, `Target::Message` für den Rückweg;
   - „Tür verbinden“ als Kernel-Rezept `connect_doors` aus dem Code von „Raum anlegen“.
5. **Schnitt (Frage 5):** vier Teilphasen E1, E2, G1, G2 wie in §5. G1 setzt Gutachten B (Flag- und Variablen-Namen, Index-Ziele `Flag`/`Var` mit den Quellen Scripts, `hidden_flag`, Auslöser, `var_conditions`, die Felder im Inspector, die Pool-Funktion) voraus. Die Referenzsperre aus Gutachten C nimmt Geschichten als schwache Quelle auf (warnen, nicht sperren).

---

## 5. Folgen für den Schnitt

| Teil | Inhalt | Größe (geschätzt) | Gate |
|---|---|---|---|
| **3d.E1 Erzählen-Rahmen** | `Depth` mit `"erzaehlen"`, Helfer, Umstellung der 88 Vergleiche, Lint; `FieldSpec.from`/`EventFieldSpec.from`, reduzierter Inspector; Rezept-Taste; Kernel-Modul `recipes` (`plan`/`apply`, `FileStep` für Scripts, auch für „Welt leeren“), Logik von Gebiet, Raum, Erweitern in den Kernel; MCP `recipe_list`, `recipe_plan`, `recipe_apply`; Entwurf-Schicht und `DraftDto`; Ergebnis mit Taste „nächster sinnvoller Schritt“; Design-System (Segmente drei Tiefen, Rezept-Taste, Entwurf) | M | Test-Umgebung auf Platin DE, HeartGold DE, HeartGold US: in Erzählen zeigt der Inspector einer Figur genau die Felder mit `from: "erzaehlen"` (Unit-Snapshot der Feldlisten je Tiefe und Art), jeder Bereich ohne Erzählen-Ansicht gleicht Bauen; „Raum anlegen“ zeigt vor dem Anlegen `[data-draft]` mit Tür und gestrichelten Zellen (Zahl über die Test-API gleich `recipe_plan`), Esc entfernt sie und `git status` bleibt leer; `recipe_apply` über MCP ist ein Undo-Schritt, ⌘Z stellt alle Dateien wieder her (auch Scripts nach „Welt leeren“); `pt-raum-assistent.yaml`, `pt-leere-region.yaml`, `hg-leere-region.yaml` unverändert grün; `scripts/gate.sh` grün |
| **3d.E2 Figur, Dialog, Tür** (nach E1) | Kernel `dialog_of` mit Sprungverfolgung und Zähler „geteilt“; Dialog-Abschnitt und Editor (Screen 14) mit Abspalten; Rezepte „Neue Figur“ (Screen 9), „Tür verbinden“ (Screen 6), „Neuer Trainer hier“ (Screen 20); `Target::Message` im Index | M | Kopflos über MCP auf frischen Kopien der drei Spiele: „Neue Figur“ mit Text → `g4k build` → g4emu: zur Figur gehen, `press a`, `shot` zeigt den Text (Agent + Blick; dazu gated Test: Nachricht im Archiv des Headers gleich der Eingabe); „Tür verbinden“ zwischen zwei Orten → g4emu `expect-map` hin und zurück; „Neuer Trainer hier“ → `walk-until-battle`/`expect-battle <Spezies> <Level>`; Dialog einer Vanilla-Figur in der Test-Umgebung ändern → nur das eine Textarchiv im Diff, ⌘Z nimmt es zurück, geteilte Nachricht zeigt „Geteilt mit N Figuren“ und spaltet ab; `pt-trainer-begegnung.yaml` grün |
| **3d.G1 Geschichten lesen** (nach Gutachten B, parallel zu E2) | Bereich Geschichten frei (`LOCKED_AREAS` leer); Fäden-Übersicht aus dem Index (Screen 10), Graph lesend mit Notizen (Screen 11; Graph-Bibliothek und Layouter nach Lizenzprüfung, `cargo deny`/npm-Lizenzen); Store-Art `Story`, `data/stories/` mit Beifile, Register `STORY_`, `DiffSubject::Story`, Story als schwache Quelle im Index, Prüfung `story.dangling-ref`, MCP `story_list`, `story_get`, `story_set`; DSPRE-Ordner: leerer Zustand mit „Als Textprojekt einrichten…“ | M bis L | Auf den drei Spielen als Textprojekt: Übersicht zeigt mindestens 59 (Platin) bzw. 62 (HeartGold) Fäden mit Werten und Orten (Zahl dieses Gutachtens als Untergrenze im gated Test); Graph eines Fadens hat einen Knoten je Wert und Kanten „gesetzt in → geprüft in“; Geschichte über Oberfläche und MCP anlegen → `git status` zeigt nur `data/stories/0000.json` (und Layout), ⌘Z entfernt sie; `text_project_in_git_builds_from_base_byte_equal` mit einer Geschichte grün (sie erzeugt keine Bytes); „ID umbenennen“ eines Trainers aus der Geschichte zieht die Geschichte mit; Diff zeigt Feldnamen |
| **3d.G2 Geschichten erzählen** (nach G1 und E2) | Rezepte „Neue Geschichte“ (Variable aus dem Pool, eigener Name), „Gespräch schaltet weiter“, „Figur erscheint/verschwindet ab Schritt“, „Szene beim Betreten“, „Belohnung einmalig“; Knotenarten Fortschritt und Erscheinen; Pool-Anzeige („noch 20 Variablen frei“) | M | Neue Szenarien `pt-geschichte.yaml` und `hg-geschichte.yaml` über MCP auf frischen Kopien: Geschichte mit drei Schritten anlegen; Figur A ansprechen (Schritt 0 → 1, `shot`); Figur B erscheint ab Schritt 1 (vorher `walk-until-blocked` durch ihre Kachel frei, danach blockiert an ihr, `expect-at`); Raum betreten startet die Szene (`shot`); Kampf `expect-battle`; Belohnung beim zweiten Ansprechen nicht noch einmal (`shot`); `vanilla_checks` 0 Fehler; Pool um 1 Variable und die Zahl der Flags kleiner; jedes Rezept ein Undo-Schritt, ⌘Z bis zum Ausgangsstand lässt `git status` leer |

Reihenfolge: B.1 (Flag- und Variablen-Namen, Index-Ziele, Inspector-Felder) ∥ E1 → (E2 ∥ G1) → G2. Gutachten C (Löschen) braucht den Index mit Scripts aus 4.3b und die Ziele aus B; es kann parallel zu E2 und G1 laufen. G1 meldet Geschichten der Sperre als schwache Quelle nach.

**Was der Schnitt voraussetzt:**
- **Von Gutachten B (Flags und Variablen):**
  - Register für eigene Namen (`data/ids/flags.json`, `vars.json`) mit Konstanten-Dateien `.g4-ids-flags.json`, `.g4-ids-vars.json` für rotom (kein `#include`), ohne Namen der Befehlsdatenbank zu überschatten;
  - Index-Ziele `Flag` und `Var` mit den Quellen Scripts, `hidden_flag`, Auslöser-`var` und `var_conditions`;
  - die Felder `hidden_flag` und `var`/`value` in Inspector und Patch (heute fehlen sie);
  - eine Pool-Funktion `allocate_flag`/`allocate_var` nach der Regel aus §2.2 (unbenutzt plus Name fehlt, `UNUSED`, `DUMMY` oder `UNK`; allgemeiner Bereich; HGSS ohne Tagesflags);
  - die Tabelle Screen 13.
- **Von Gutachten C (Löschen):** eine Referenzsperre, die schwache Quellen (Geschichten) als Warnung mit Liste zeigt und harte (Scripts, Events) sperrt. Beim Löschen einer Figur, die ein Rezept angelegt hat, bietet sie an, ihren Script-Slot mitzunehmen.
- **g4emu:** Die Szenarien kommen mit `expect-map`, `expect-at`, `walk-until-blocked`, `expect-battle` und `shot` aus. Ein `expect-var`/`expect-flag` (Spielstand im RAM lesen) wäre genauer, braucht aber Adressen je Spiel (§6).

---

## 6. Offene Fragen

1. **Engine-Nutzung des Pools:** Setzt arm9 oder ein Overlay Flags oder Variablen, die kein Script nennt (besonders HGSS `UNK_…` und der Systembereich hinter `0x95F`)? Ohne Prüfung gilt nur der allgemeine Bereich als Pool (Platin 135 Flags, 21 Variablen; HeartGold 152, 83).
2. **Knotenarten:** Ergänzt das Design-System „Fortschritt“ und „Erscheinen/Verschwinden“ (Empfehlung) oder bleibt es bei fünf Arten mit Fortschritt als Kantenbeschriftung?
3. **rotom-Literale:** Behält eine geänderte Literal-Nachricht beim erneuten Compile ihren Index im Archiv, oder hängt rotom eine neue an? Davon hängt ab, ob der Dialog-Editor Literale in der Quelle oder Nachrichten im Archiv bearbeitet (geprüft im Plan von E2).
4. **Geschichten im DSPRE-Ordner:** nur lesend (Fäden aus dem Index, Empfehlung) oder als weitere Wurzeldatei wie `g4-world.json`? Die Entscheidung von Phase 4 spricht für „nur im Textprojekt“.
5. **Graph-Bibliothek:** React Flow (`@xyflow/react`, MIT) mit einem Layouter (dagre, MIT, oder elkjs, EPL-2.0) oder ein eigenes SVG-Layout für Fäden mit höchstens 16 Schritten? Lizenzprüfung gehört in G1.
6. **Rezept-Logik von Gebiet und Raum in den Kernel:** in E1 umziehen (Empfehlung, für `g4k` und Gates) oder in der Tauri-Schicht lassen und nur `plan` ergänzen?
7. **Trainer anlegen als Undo-Schritt:** Kann `trainer_create` im Textprojekt Dateien statt Member schreiben und damit rückgängig werden? Dann wäre „Neuer Trainer hier“ ein ganzer Undo-Schritt.
8. **`expect-var`/`expect-flag` in g4emu:** In diesem Gutachten sind die Adressen des Spielstands für Platin und HeartGold nicht ermittelt. Gutachten B untersucht sie parallel (`gutachten-b-ram.py` in diesem Ordner); liegen sie vor, sollten die Gates von E2 und G2 Variablen und Flags direkt prüfen, sonst über Verhalten (Figur da oder weg, Kampf, Ortswechsel, Bild).
9. **Erzählen-Ansicht von Texte:** „Dialoge nach Figur“ braucht `dialog_of` für alle Figuren eines Orts (1.481 bzw. 1.324 Slots); die Zeit dafür ist nicht gemessen (geschätzt unter 1 s je Ort, da die Prüfungen dieselben Fakten schon lesen).
