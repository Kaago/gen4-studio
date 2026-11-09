# Gutachten B: Flags und Variablen, Nutzungsindex, eigene Namen, Spielmodus live, Ei-Attacken (Phase 3d)

Stand: 30.10.2026, gelesen auf `main` (`a5d6909`) im Arbeitsbaum `$HOME/Projekte/lab/editor/gen4-studio`. Am Repo-Code ist nichts geändert. Grundlage sind:

- `docs/ROADMAP.md` (Phase 3d, „Aus dem Design-System, später“), Gutachten C von Phase 4 §7 und die [Entscheidungen Phase 4](../2026-10-27-phase-4/entscheidungen.md);
- `design-system/README.md` (Prinzip 7, „Spielmodus“, Screen „Flags und Variablen“) und `Audit-Design-System.md` (Zeile 24, Zeile 69, Screen 13);
- im Kernel `metadata/scrcmd/{platinum,hgss}_v2.json` und `SOURCE.md`, `metadata/script_refs.yaml`, `metadata/templates.yaml`, `metadata/arm9_tables.yaml` und `metadata/archives.yaml`, dazu `src/data/refs.rs`, `src/data/trainer_create.rs`, `src/scripts.rs`, `src/run.rs` und `src/textproject/{include,registry,symbols,rename}.rs`;
- `docs/formats/text_project.md` („Verweise“, „Symboltabellen und Register“, „IDs in Scripts“);
- in der App `workbench/inspector/eventFields.ts` und `LevelScriptSection.tsx`;
- `scripts/g4emu/` (`emu.py`, `games.yaml`, `games.py`, `battle.py`, `mcp.py`) und `docs/agent-gate.md` (Abschnitt Emulator);
- die Quellen von rotom v0.7.0 (Checkout `fb11503`: `decompiler/ir_to_source.rs`, `compiler/ir/lowerer.rs`, `database.rs`) und uxie 0.9.7 (`c_parser/symbol_table.rs`, `egg_move_data/`).

Die Gutachten A (Geschichten, Erzählen) und C (Löschen, Sprites, Schnitt) im selben Ordner habe ich nach meinen Messungen gelesen und abgeglichen. Wo sich Zahlen überschneiden, stimmen sie überein (§1.6).

**Datenbasis.** Ich habe APFS-Klone der DSPRE-Ordner Platin DE und HeartGold DE und ihrer `.nds` unter `Roms/_testkopien/3d-gutachten-b-{pt,hg}` angelegt. Auf beiden lief `g4k base assign` und `g4k text adopt`. Das dauerte 35 s bzw. 56 s und ergab 1 124 bzw. 965 übernommene Scripts. Als Vergleich ohne übernommene Scripts dienten zwei weitere Klone (`…-ptraw`, `…-hgraw`). Für den Emulator nahm ich Kopien der `.nds` (dazu HeartGold US) und Kopien der Spielstände `_saves/{CPUD,IPKD,IPKE}-zimmer.sav`; die Originale wurden nie geöffnet. Alle Kopien sind wieder gelöscht. Der Bericht enthält keine ROM-Inhalte, nur Zahlen, Adressen und die Namen aus den Metadaten.

**Last.** Zwei andere Gutachter liefen parallel, die Load-Average lag bei 8 bis 10 (10 Kerne). Alle Zeiten gelten mit **±50 %**. Gemessen wurde je Zeitangabe drei- bis fünfmal; angegeben sind die Spanne und, wo sinnvoll, der kalte erste Lauf.

**Kurzurteil.**

- **Flags und Variablen sind ein Index-Thema, kein Format-Thema.** Die Namen aus pret liegen vor. rotom erkennt Flags und Variablen schon heute in beiden Richtungen:
  - beim Dekompilieren über eine Namensregel für Parameter (`flag`, `flagID`, `destVarID` …);
  - beim Kompilieren über das Präfix `FLAG_`/`VAR_` (`if FLAG_X then`).

  Was fehlt, ist der Nutzungsindex (Ziele `Flag`/`Var` mit Lese-/Schreibzugriff) und eine Fläche dafür. Auch der Event-Inspector kennt heute weder das Sichtbarkeits-Flag eines Objekts noch Variable und Wert eines Auslösers.
- **Eigene Namen funktionieren mit rotom ohne Änderung an rotom.** Ein Name wie `FLAG_HAT_MEDAILLE` in einer Konstantendatei kompiliert byte-gleich zu `FLAG_UNUSED_0x0094`, auch in `if … then`. Der Kernel muss die Datei aber selbst als `.g4-ids-flags.json` erzeugen: `include.rs:170` löscht fremde `.g4-ids-*`-Dateien.
- **Spielmodus live ist technisch gelöst, aber nicht in der App.** Flags und Variablen liegen im RAM hinter einem festen Zeiger, und zwar demselben, den `games.yaml` schon für den Ort nutzt. Lesen und Schreiben ist bewiesen:
  - Ein gelöschtes Flag lässt beim nächsten Betreten Figuren erscheinen.
  - Eine auf 0 gesetzte Variable löst einen Auslöser aus.

  Die App startet aber einen externen melonDS ohne RAM-Zugang. Das gehört in eine eigene, spätere Teilphase.

---

## 1. Inventar der Flag- und Variablen-Verwendungen (Frage 1)

### 1.1 Befehle (aus den scrcmd-Datenbanken, Revision 33)

| | Platin DE (`platinum_v2.json`) | HeartGold DE (`hgss_v2.json`) |
|---|---|---|
| Befehle ohne Bewegungen | 956 | 886 |
| mit einem Flag-, Variablen- oder `flex`-Parameter | **509** | **239** |
| davon Flag-Parameter (`flag` als Typ in 4 Makros, sonst `u16` mit Namen `flagID`/`flag`) | 7 | 7 |
| davon Variable (Typ `var`, oder nur am Namen erkennbar) | 335 (4 nur am Namen) | 231 (**199 nur am Namen**, Typ `u16`) |
| davon `flex` (Zahl oder Variable) | 270 | 2 |
| Parameter mit `access`-Angabe (`read`, `must_write`, `may_write`, `read_must_write` …) | 349 von 501 Befehlen mit `var`/`flex`; 200 Befehle schreiben | **keine** |
| Flag über eine Variable (`CheckFlagFromVar`, `SetFlagVar` …) | 2 | 3 |

Befunde:

- **`SetFlag`, `ClearFlag` und `CheckFlag` haben in beiden Datenbanken den Parametertyp `u16`, nicht `flag`.** Ein Index „nach Typ“ fände sie nicht. rotom selbst entscheidet beim Dekompilieren nach dem **Namen** des Parameters (`param_semantics` in `ir_to_source.rs:28-58`: `flag|flagid|flag_id` → Flag; 22 Namen wie `var`, `destvarid`, `var_or_trno` → Variable). Ein Wert, der als Variable auflöst, bekommt sonst ohnehin einen `VAR_`-Namen (`:84-88`).
- **HeartGold typisiert kaum.** `CopyVar`, `SetOrCopyVar`, `CompareVarToVar` und die meisten Befehle, die Zahl oder Variable nehmen, haben `u16`. Lese-/Schreibrichtung steht nirgends.
- rotom kennt zusätzlich die Bedingungsformen `if FLAG_X then` (nacktes Flag, Familie per Präfix `FLAG_`, `lowerer.rs:1792-1807`) und `if VAR_X == n then` / `if VAR_X then` (`:1300-1356`). Zuweisungen als Anweisung gibt es nicht. Die Vanilla-Quellen nach `adopt` benutzen nur die Befehlsform.

### 1.2 Vorkommen in den Scripts (frische Kopie nach `adopt`)

Gezählt mit `gutachten-b-inventar.py` (Zeilen der rotom-Quellen, Namen aus der Datenbank, `alias` der Datei) und gegengeprüft mit dem Rust-Prototyp, der mit rotoms Parser arbeitet (`gutachten-b-proto`, §3.3). Beide stimmen überein: Platin 1 947 direkte Flag-Verweise plus 153 über `SetTrainerFlag` & Co. ergibt 2 100. Die Variablen kommen bei beiden auf 12 336.

| | Platin DE | HeartGold DE |
|---|---|---|
| Quellen `.rotom` | 575 | 497 |
| **Flags**: Vorkommen / verschiedene | **2 100 / 774** | **2 587 / 745** |
| … lesend / schreibend | 754 / 1 346 | 624 / 1 963 |
| … Trainer-Flags über `Set/Clear/CheckTrainerFlag` (Vorkommen / verschiedene) | 153 / 65 | 70 / 54 |
| … Flag-Nummer aus einer Variable (dynamisch, nicht auflösbar) | 1 (+7 Trainer-Flags) | 7 (+9) |
| **Variablen**: Vorkommen / verschiedene | **12 336 / 223** | **12 384 / 248** |
| … Script-lokal/Sonder ab `0x8000` (`VAR_RESULT`, `0x8004` …) | 9 575 / 12 | 8 150 / 13 |
| … kartenlokal `0x4000`–`0x401F` | 1 326 / 14 | 2 556 / 18 |
| … Objektgrafik `0x4020`–`0x402F` | 170 / 7 | 27 / 4 |
| … gespeichert `0x4030`–`VARS_END` | 1 265 / 190 | 1 651 / 213 |
| … Zugriff unbekannt (HGSS: von Platin-Befehlen gleichen Namens geborgt) | 418 | 2 802 |

Die häufigste „Variable“ ist in beiden Spielen `0x800C` (`VAR_RESULT`, je rund 4 500 Vorkommen). Gespeicherte Variablen machen nur 10 bzw. 13 % der Variablen-Vorkommen aus.

Eine Eigenheit: rotom schreibt beim Dekompilieren für `0x4000` den Namen `VARS_START` (`CompareVarToValue VARS_START, 0`), weil `ConstantDb::load_from_db` diese Konstante selbst einträgt (`database.rs`). Für die Anzeige gilt daher: `VAR_`-Namen vor Bereichskonstanten.

### 1.3 Events, Level-Scripts, Trainer, Rezepte

| Quelle | Platin DE | HeartGold DE |
|---|---|---|
| Objekt-Events / davon mit Sichtbarkeits-Flag (`hidden_flag`) | 3 555 / **1 613** | 2 667 / **1 108** |
| … verschiedene Flags, davon kartenlokal (≤ `0x40`) | 705, 31 | 655, 17 |
| versteckte Items (BG-Event Typ 2, Script `8000 + n`) / verschiedene | 262 / 250 | 241 / 231 |
| … ihr Flag (Basis + n) | `0x2DA` + n, benutzt `0x2DA`–`0x3EE` | `0x320` (`HIDDEN_ITEMS_FLAG_BASE`) + n, `0x320`–`0x406` |
| Auslöser (Coord-Events) / verschiedene Variablen (alle ≥ `0x4000`) | 186 / 72 | 195 / 126 |
| Level-Scripts `NNNN.json` / `var_conditions` / verschiedene Variablen | 549 / 207 / 59 | 468 / 230 / 71 |
| Trainer hinter Objekt-Events (Script 3000+ / 5000+) → Trainer-Flag lesen und setzen (Engine) | 403 | 390 |
| Rezepte: `done_var` der Vorlage „Leere Region“ (`templates.yaml`) | `VAR_UNUSED_0x406C` | `VAR_UNK_4031` |

**Trainer-Flags** liegen bei `0x550 + Trainer-ID`; die Kapazität ist 1 040 (bis `0x95F`, `arm9_tables.yaml`, `trainer_create.rs:33`). Belegt sind Platin 928 Trainer (bis `0x8EF`, 112 frei) und HeartGold 738 (bis `0x831`, 302 frei). Ab `0x960` beginnen in beiden Spielen die System-Flags: der erste Name nach den Trainern ist Platin `FLAG_BAG_ACQUIRED` (`0x960`), HeartGold `FLAG_HAVE_FOLLOWER` (`0x961`).

### 1.4 Gesamt: wie viel nutzt das Vanilla-Spiel?

| | Platin DE | HeartGold DE |
|---|---|---|
| verschiedene Flags (Scripts ∪ Events ∪ versteckte Items ∪ Trainer-Flags aus Scripts) | **1 434** | **1 271** |
| … ohne Trainer-Flags | 1 369 | 1 217 |
| … ohne Trainer-Flags und versteckte Items (wie Gutachten A, Tabelle 2.2) | 1 119 | 986 |
| verschiedene Variablen (Scripts ∪ Auslöser ∪ `var_conditions`, mit `≥ 0x8000`) | 225 | 266 |
| Flags, die gesetzt, aber nirgends gelesen werden / gelesen, aber nirgends gesetzt | 84 / 428 | 52 / 308 |
| gespeicherte Variablen nur gelesen / nur geschrieben | 9 / 3 | 21 / 7 |

„Gelesen, nirgends gesetzt“ sind vor allem Sichtbarkeits-Flags, die die Engine oder ein Script über eine Variable setzt, oder die nie gesetzt werden (die Figur ist dann immer da). Der Index muss also auch Verweise ohne Gegenstück zeigen und darf sie nicht als Fehler werten.

### 1.5 Bereiche und freie Nummern

Aus den Namen und Konstanten der Datenbanken (`gutachten-b-namen.py`) und aus der Nutzung (§1.4):

| Bereich | Platin | HeartGold | Quelle, Sicherheit |
|---|---|---|---|
| Flag 0 | `FLAG_UNUSED_0x0000` | `FLAG_NOTHING` | Namen |
| kartenlokale Flags | `0x01`–`0x40` (`FLAG_MAP_LOCAL_…`), dazu `0x8E` `FLAG_MAP_LOCAL_REMOVE_OBJECT` | `0x01`–`0x3F` (`MAPTEMP_FLAG_BASE`, `NUM_MAPTEMP_FLAGS` 64) | Konstanten (HGSS), Namen (Pt) |
| allgemeine Flags | bis `0x54F` | bis `0x54F` | Namen |
| Trainer-Flags | `0x550`–`0x95F` | `0x550` (`TRAINER_FLAG_BASE`)–`0x95F` | Konstante (HGSS), `arm9_tables.yaml` |
| System-Flags | ab `0x960` | ab `0x960` | Namen |
| Tages-Flags | `0xAA0`–`0xB5F` (192) | `0xAA0` (`DAILY_FLAG_BASE`), 192 | Konstanten |
| Ende des Spielstands | **`0xB60`, unsicher** | `0xB60` (`NUM_FLAGS` 2 912) | HGSS: Konstante. Platin: 364 Byte Flags im Spielstand gemessen (§4.2); die Datenbank nennt aber Namen bis `0xFFF` (1 184 `FLAG_UNK_…` über `0xB5F`), die wahrscheinlich außerhalb des Spielstands liegen |
| temporäre Flags | keine Namen | `0x4000`–`0x403F` (`TEMP_FLAG_BASE`, 64) | Konstante; nicht im Spielstand |
| kartenlokale Variablen | `0x4000`–`0x401F` | `0x4000`–`0x401F` (`NUM_TEMP_VARS` 32) | Konstanten |
| Objektgrafik-Variablen | `0x4020`–`0x402F` | `0x4020`–`0x402F` | Konstanten |
| gespeicherte Variablen | `0x4030`–`0x411F` (`VARS_END`, 288 insgesamt) | `0x4030`–`0x416F` (`NUM_VARS` 368) | Konstanten |
| Sonder-/Script-lokale Variablen | `0x8000`–`0x800D` (+ `FR_VAR_0x800E`–`0x8010`) | `0x8000`–`0x800D` | Konstanten; nicht gespeichert |

**Freie Nummern.** Frei heißt hier: als `UNUSED` benannt und in der Kopie nirgends benutzt, unter `0x960`.

- **Platin, Flags:** 116 in 33 Läufen. Die längsten:

  | Lauf | Länge | Bemerkung |
  |---|---|---|
  | `0x3BF`–`0x3D9` | 27 | |
  | `0x53C`–`0x54F` | 20 | |
  | `0x2CD`–`0x2D9` | 13 | |
  | `0x881`–`0x889` | 9 | liegt im Trainerbereich: das sind die Trainer 817 bis 825, die es gibt, also **nicht** frei |
  | `0x3EF`–`0x3F5` | 7 | Ende des Bereichs der versteckten Items |

- **Platin, Variablen:** 20 (`0x406C`–`0x406F` am Stück, sonst einzeln).
- **HeartGold:** Keine Nummer ist als `UNUSED` benannt. Kandidaten sind nur `UNK`-Namen ohne Nutzung: 153 Flags in `0x41`–`0x54F` und 82 Variablen. Ob die Engine sie ohne Script-Verweis setzt, ist **ungeprüft**.

Gutachten A zählt mit einer weiteren Regel (`UNUSED`, `DUMMY` oder `UNK` bzw. ohne Namen, unter `0x550`) 135/152 freie Flags und 21/83 Variablen. Beide Zählungen passen zusammen.

**Warnung zu den pret-Namen:** 16 Platin-Flags heißen `UNUSED`, werden in Vanilla aber benutzt.

- In Scripts: `0x76`, `0x7C`, `0x7E`, `0x4E4`, `0x9D3`, `0x9DE`, `0x9E0`, `0x9E2`, `0x9E6`.
- Als Sichtbarkeits-Flag eines Objekts: `0x96`, `0x16B`, `0x16D`–`0x16F`, `0x187`, `0x193`.

`0x96` liegt direkt neben `FLAG_UNUSED_0x0094`, dem Flag, das die Beispielquelle in `scripts.rs:2966` benutzt. Ein Vorschlag „freies Flag“ muss deshalb aus dem Nutzungsindex kommen, nicht aus dem Namen.

### 1.6 Abgleich mit Gutachten A

Gutachten A (Tabelle 2.2) nennt dieselben Grundzahlen:

- Objekte mit Flag 1 613 / 1 108;
- verschiedene Sichtbarkeits-Flags 705 / 655;
- Auslöser 186 / 195;
- `var_conditions` 207 / 230;
- benutzte Flags ohne Trainer und Items 1 119 / 986;
- benutzte gespeicherte Variablen 211 / 251.

Die letzte Zahl misst A über `0x4000`–`0x7FFF` mit Events; mein Prototyp findet 211 / 235 nur in Scripts.

---

## 2. Namen (Frage 2)

### 2.1 Befund

- **pret-Namen.** Die scrcmd-JSON tragen `flags` und `vars` (Platin 4 098/315, HeartGold 2 252/394; Commit `c248fb3` bzw. `9d8b759` von pret laut `meta`). Gelesen werden sie an zwei Stellen:
  - **uxie** lädt sie als Rückfall-Symbole, wenn keine anderen Flag- bzw. Variablen-Symbole bekannt sind (`symbol_table.rs:843-880`, Familie nach Präfix `:132-140`).
  - **rotom** nimmt `FLAG_`/`VAR_`-Namen aus diesen Symbolen. Die Quellen nach `adopt` tragen deshalb schon `SetFlag FLAG_HIDE_…`.

  Die App zeigt die Namen bisher nirgends. Der Kernel hat für Flags keine Symboltabelle: `symbols.rs::table` kennt nur Spezies, Attacken, Items, Fähigkeiten und Typen, und das je Art, nicht je Spielfamilie.
- **Felder in der App.** Der Event-Inspector (`eventFields.ts:72-85`) zeigt bei Objekten weder `hidden_flag` noch bei Auslösern `var` und `value`. Die Werte gibt es nur in Rohdaten bzw. im Textprojekt-JSON. Das Level-Script zeigt die Variable als Zahl (`LevelScriptSection.tsx`).
- **rotom und eigene Namen.** rotom v0.7.0 wertet in DSPRE-Projekten kein `#include`/`#define` aus. Es lädt aber jede `*.json` seines Konstantenordners als Name → Zahl (`database.rs` `load_directory`, ohne `_v2`). Gemessen auf der Platin-Kopie:
  - Ein Script mit `SetFlag`, `if … then ClearFlag … endif` und `SetVar` übersetzt mit `FLAG_UNUSED_0x0094`/`VAR_UNUSED_0x406C` und mit `FLAG_HAT_MEDAILLE`/`VAR_MEIN_ZAEHLER` aus einer eigenen Konstantendatei zu **demselben Member** (SHA-1 `50c35908…`). Damit ist auch die Bedingung `if FLAG_X then` mit eigenem Namen gezeigt; rotom erkennt das Flag dort am Präfix `FLAG_`.
  - Eine von Hand angelegte `.g4-ids-flags.json` löscht der Kernel vor dem Compile (`include.rs:170`, „`starts_with(ID_FILE_PREFIX) && !ours.contains`“). Danach lehnte der Compile mit „Undefined symbol“ ab.
- Gutachten A bestätigt den Weg über die `.g4-ids-*.json`.

### 2.2 Varianten

| | A: nur Register `data/ids/flags.json`, `vars.json` | B: Entität `data/flags/NNNN.json` (`id`, Beschreibung, Standardwert) ohne Register | **C: Register + dünne Entität** (Register `data/ids/{flags,vars}.json` für die Namen, `data/flags/NNNN.json` bzw. `data/vars/NNNN.json` nur für Nummern mit eigenem Namen oder Beschreibung, mit `id` und `description`) |
|---|---|---|---|
| Passt zu | `registry.rs` (ID → Nummer, kanonisch, eingefroren), `rename.rs`, `include.rs` | Gutachten C von 4 („Projekt-Entität“) | beidem: genau das Muster von Trainern und Headern (Entität mit `id`, Register ID → Nummer) |
| Beschreibung | keine | ja | ja |
| Standardwert | – | Das Spiel hat **keinen**: Flags und Variablen starten bei 0, gesetzt wird im Neues-Spiel-Script (Platin Script-Datei 0412 setzt dort Dutzende `FLAG_HIDE_…`). Ein Standardwert müsste in dieses Script hineingebaut werden: Magie, die man im Script nicht sieht | – (bewusst weggelassen) |
| `rename.rs` | direkt (neues Präfix) | eigener Weg | direkt |
| Diff je Entität, Löschsemantik „Datei entfernen“ (Gutachten C) | nur Register-Zeile | ja | ja |
| Prüfstein für Phase 5 (Art ohne ROM-Gegenstück) | schwach | ja | ja |
| Dateien bei typischer Nutzung | 2 | 1 je benanntem Flag | 2 + 1 je benanntem Flag (Dutzende, nicht Tausende) |

**Zusammenspiel mit rotom (alle Varianten):**

- `SymbolKind::Flag` und `SymbolKind::Var` mit den Präfixen `FLAG`/`VAR`.
- Die feste Tabelle kommt aus der scrcmd-Datenbank, **je Familie**: `table()` braucht dafür einen Familien-Parameter.
- In den Registern stehen nur eigene Namen. `KINDS` in `include.rs` bekommt `flags` und `vars`, die Dateien sind `.g4-ids-flags.json`/`.g4-ids-vars.json` und enthalten nur Registereinträge.
- Die bestehende Schatten-Regel greift: ein Name, den rotom schon mit anderer Nummer kennt, fällt weg.
- Ein eigener Name ist ein **zweiter** Name derselben Nummer. Der pret-Name bleibt gültig, Quellen mit dem pret-Namen kompilieren weiter.
- **Pflicht:** Eigene Namen beginnen mit `FLAG_` bzw. `VAR_`. Sonst erkennt rotom sie in `if NAME then` nicht als Flag (`lowerer.rs:1805`, Familie per Präfix). Diese Regel prüft `rename.rs` wie die anderen Präfixe.
- **Umbenennen** über `rename.rs` geht nur für Registereinträge. pret-Namen sind wie rotoms eigene Namen nicht umbenennbar; „Umbenennen…“ auf einem pret-Namen legt einen eigenen Namen an. Mit dem Präfix sind das 4 Zeilen mehr in `registry_file` und den Prüfungen.
- **JSON des Textprojekts.** Die Events (`hidden_flag`, Auslöser `var`) und die Level-Scripts könnten nach der Regel „Verweise“ (`text_project.md`) Namen statt Zahlen schreiben. uxie liest `hidden_flag` als Name oder Zahl (`platinum_json.rs:280-282`). Empfohlen ist das erst nach 3d.1 (Migration der Schema-Version), damit 3d.1 klein bleibt.

**DSPRE-Layout ohne Textprojekt:**

- pret-Namen gibt es (aus den Metadaten), Anzeige, Suche, Picker und Nutzung aus Events und Level-Scripts auch.
- Nutzung aus Scripts gibt es nur mit übernommenen Scripts.
- **Keine eigenen Namen.** „Umbenennen…“ ist dort deaktiviert, mit dem Grund aus `rename.rs::NOT_A_TEXT_PROJECT` und dem Weg „Als Textprojekt einrichten…“.
- Kein Ausweichen auf `g4-world.json`: Die Entscheidungen von Phase 4 wollen neue Wurzel-JSON vermeiden.

**Empfehlung (recommend): Variante C.**

- Register `data/ids/flags.json` und `vars.json` für die Namen.
- `data/flags/NNNN.json` bzw. `data/vars/NNNN.json` mit `{id, description}` nur für benannte oder beschriebene Nummern (vierstellig dezimal wie die anderen Arten).
- Kein Standardwert.
- `.g4-ids-flags.json`/`-vars.json` aus `include.rs`.
- Umbenennen über `rename.rs` mit Pflichtpräfix.
- Im DSPRE-Layout nur pret-Namen.

Eine Geschichte (Gutachten A) verweist auf Flags über deren ID. Die Rückrichtung („in welchen Geschichten“) liefert der Index (§3), nicht die Flag-Datei.

---

## 3. Nutzungsindex (Frage 3)

### 3.1 Messung heute (`RefIndex::rebuild_full`)

Gemessen mit dem vorhandenen gated Test `vanilla_index_builds_under_a_second` (Release-Testbinary, ein Faden) und mit `gutachten-b-proto index`. Werte in ms, Spanne über 3 bis 5 Läufe:

| | Platin ohne Scripts | Platin übernommen | HeartGold ohne Scripts | HeartGold übernommen |
|---|---|---|---|---|
| kalt (erster Lauf nach Klon) | 290 bis 430 | **1 620 bis 1 650** | 395 bis 440 | **1 640 bis 1 830** |
| warm | 142 bis 159 | **220 bis 254** | 124 bis 138 | **205 bis 366** |
| Verweise gesamt | 20 035 | 20 467 (davon Script 432) | 19 548 | 19 880 (Script 332) |
| 200 × `refs(Target::Item(n))` | 4,7 ms | **197 ms** | 4,6 ms | **176 ms** |

**Befunde:**

1. Der Script-Anteil am warmen Neuaufbau ist 80 bis 110 ms. Den größten Teil kostet `ScriptLookup::of` mit `base_constants`, also rotoms Konstanten und den uxie-Workspace. Das **Parsen aller Quellen** braucht nur 18 bis 25 ms auf einem Faden (Prototyp, rotoms `Parser::new_fallible`).
2. **Jede Abfrage `refs()`/`count()` kostet mit übernommenen Scripts rund 1 ms.** `current()` ruft vor jeder Abfrage `refresh()`, und das stattet alle 575 bzw. 497 Quellen und die Konstantendateien (`ScriptSources::refresh`). Eine Flagliste, die je Eintrag zählt (1 400 Flags), käme so auf etwa 1,4 s. Der gated Test misst nur den Aufbau und merkt das nicht.

### 3.2 Schnittstelle

- **Ziele:** `Target::Flag(u16)` und `Target::Var(u16)`.
- **Zugriff:** `Ref` bekommt `access: Access` (`Read`, `Write`, `ReadWrite`, `Unknown`). Die heutigen Arten sind `Read`; die Felder `field`/`slot` bleiben.
- **Quellen und Regeln, deklarativ** (wie Gutachten C R1 es für alle Kanten vorschlägt), als neue Datei `metadata/script_flagvars.yaml` neben `script_refs.yaml`:
  1. **Scripts, Befehle.** Parameter nach Typ (`flag`, `var`, `flex`) **und** nach rotoms Namensregel. Ein Flag-Parameter hat den Wert als Flag. `var` und `flex` zählen nur ab `0x4000` als Variable; darunter ist es ein Wert, wie in `refs.rs::FIRST_VARIABLE`. Den Zugriff liefert Platin aus `access` (fehlt bei 152 Befehlen). HeartGold hat keine Angaben: dort borgt der Kernel sie von Platin-Befehlen gleichen Namens und nutzt eine kurze Ausnahmeliste (`SetVar`, `CopyVar`, `SetOrCopyVar` als Schreiben, `AddVar`/`SubVar` als Lesen und Schreiben). Der Rest ist `Unknown`. `SetFlag`/`ClearFlag` sind Schreiben, alles andere mit Flag ist Lesen.
  2. **Scripts, Bedingungen.** In `if`/`while`/`match` zählen ein nacktes `FLAG_…` und eine Variable im Vergleich als Lesen. `scripts::visit` liefert die Ausdrücke schon.
  3. **Trainer-Flags.** `Set/Clear/CheckTrainerFlag` mit Trainer-Nummer werden zu `Flag(0x550 + n)`. Jedes Objekt-Event mit Trainer-Script wird zu `Flag(0x550 + n)` mit `ReadWrite` (Engine).
  4. **Events.** Ein Objekt mit `hidden_flag` ≠ 0 ist `Read` (Feld `hidden_flag`). Ein BG-Event Typ 2 mit Script `8000 + n` ist `Flag(Basis + n)` `ReadWrite`; die Basis kommt je Spielcode in `arm9_tables.yaml`, Platin `0x2DA`, HeartGold `0x320`. Ein Auslöser ist `Var(var)` `Read`.
  5. **Level-Scripts.** `var_conditions` ergibt `Var` `Read`. Das ist eine neue Quellart `LevelScript` aus `expanded/scripts/NNNN.json`, nur mit übernommenen Scripts.
  6. **Rezepte.** `done_var` aus `templates.yaml` ist ein Verweis aus den Metadaten (Feld `template`), damit „frei“ es nicht vorschlägt.
- **Nicht indexiert:**
  - Variablen ab `0x8000`, also Script-lokale Register. Sie machen 9 575 bzw. 8 150 Vorkommen aus, ohne Aussage über die Handlung; die Liste zeigt sie mit Zähler, ohne Nutzungsdetail.
  - Flags oder Trainer aus einer Variable: Sie werden nur gezählt (§1.2).
- **Kartenlokale Flags und Variablen** (`0x01`–`0x40`, `0x4000`–`0x401F`) bedeuten je Karte etwas anderes. Der Index speichert sie, die Fläche gruppiert sie aber nach Header bzw. Scriptdatei statt sie als ein Flag über 54 Karten zu zeigen (Flag `0x20` steht in Platin an 54 Objekten).
- **Neue Abfrage:** `usage(kind) -> BTreeMap<u16, Usage { read, write, unknown, sources }>` in **einem** Durchlauf, für die Liste. Dazu `refs(Target::Flag(n))` für das Detail.
- **Refresh nur einmal je Anfrage-Bündel:** `refresh()` darf nicht vor jeder Abfrage alle Quellen statten. Dafür reicht eine Zeitsperre von etwa 250 ms oder ein Generationszähler, den die App beim Speichern, Wechseln und Fokus hochzählt. Diese Korrektur hilft auch der Trainerliste von heute.

### 3.3 Zuwachs (geschätzt aus den Zählungen, Prototyp gemessen)

| | Platin DE | HeartGold DE |
|---|---|---|
| neue Verweise aus Scripts (Flags + Trainer-Flags + Variablen < `0x8000`) | 1 947 + 153 + 2 761 ≈ 4 900 | 2 517 + 70 + 4 234 ≈ 6 800 |
| aus Events, versteckten Items, Auslösern, Level-Scripts, Trainern hinter Events | 1 613 + 262 + 186 + 207 + 403 ≈ 2 700 | 1 108 + 241 + 195 + 230 + 390 ≈ 2 200 |
| **Summe neu / heute** | **≈ 7 600 / 20 467 (+37 %)** | **≈ 9 000 / 19 880 (+45 %)** |
| mit den Variablen ab `0x8000` zusätzlich | +9 600 | +8 200 |
| Rechenzeit Erkennen im Prototyp (alle Quellen, ein Faden, nach dem Parsen) | 1 ms | 1 ms |
| Lesen der 549 bzw. 468 Level-Script-JSON | geschätzt 10 bis 40 ms warm, kalt mehr | ebenso |

Der warme Neuaufbau steigt damit geschätzt von 220 bis 250 ms auf 240 bis 300 ms und bleibt unter der Grenze von 1 000 ms aus dem Test. Den Speicher bestimmt die Zahl der `Ref` (24 Byte): etwa 0,2 MB mehr.

### 3.4 Fläche „Flags und Variablen“ (Screen 13, Bereich Geschichten)

- **Liste mit Suche**, Vorlage `PickerList`: Name (eigener Name fett, pret-Name grau in Mono), Nummer hex und dezimal, Zähler „gesetzt N · geprüft M“.
- **Filter-Plaketten:** Flags | Variablen, allgemein / kartenlokal / Trainer / versteckte Items / System, „unbenutzt“, „frei“.
- **Spalte „Live“** in Bernstein, leer bis zum Spielmodus (§4).
- **Detail:** zwei Abschnitte „Wo gesetzt“ und „Wo geprüft“, gruppiert nach Quelle. Jede Zeile ist ein Weg (Prinzip 7):
  - Script-Datei und Zeile öffnen den Script-Editor an der Zeile;
  - ein Objekt oder Auslöser wählt das Event in der Schau;
  - ein Level-Script öffnet den Abschnitt im Inspector des Headers;
  - ein Trainer öffnet den Trainer.
- **Kopf:** „Umbenennen…“ (nur Textprojekt), Beschreibung, und für freie Nummern „Als eigenes Flag benennen…“.
- **Picker „Flag“ und „Variable“** im Event-Inspector: neue Felder „Sichtbarkeits-Flag“ am Objekt, „Variable“ und „Wert“ am Auslöser. Dazu im Level-Script. Treffer zeigen Zähler und Bereich; ein Fuß bietet „Freies Flag nehmen“ mit Vorschlag aus dem Index.
- **MCP:** `list_flags` (Filter, Zähler), `flag_usage` (Verweise mit `access`), `rename_id` mit den neuen Präfixen, `free_flag` (Vorschlag).

**Empfehlung (recommend):**

- `Target::Flag`/`Var` mit `access`, deklarative Regeln in `metadata/script_flagvars.yaml`;
- Quellen Scripts (Befehle und Bedingungen), Events, versteckte Items, Trainer, Level-Scripts, Rezepte;
- ohne Variablen ab `0x8000`;
- eine Abfrage `usage()` in einem Durchlauf;
- `refresh` gebündelt statt je Abfrage.

Die Fläche bekommt Liste, Detail „Wo gesetzt / Wo geprüft“ mit Wegen und die Picker im Event-Inspector. „Frei“ kommt aus dem Index, nicht aus dem Namen `UNUSED`.

---

## 4. Spielmodus live (Frage 4)

### 4.1 Heute

- **„Hier spielen“** baut und startet über `run.rs::launch_emulator` einen **externen** Emulator (`open -a <App> <rom>` bzw. den Pfad aus `localStorage` `g4.emulatorPath`). Die App hat keinen Kanal zu ihm und bekommt nur eine PID.
- **`scripts/g4emu/g4emu/mcp.py`** ist ein MCP-**Client** für `studio mcp` (Szenarien), kein Server.
- **`g4emu`** (libretro melonDS DS 1.4.0, kopflos, Python) liest den 4-MiB-Hauptspeicher über `ram()` als `memoryview` und kann ihn schreiben.
- **Auf dem Rechner** ist melonDS 1.1 installiert. Es hat einen GDB-Stub (ARM9 Port 3333, in den Einstellungen aus), den ich **nicht** geprüft habe.

### 4.2 Lage im RAM (gemessen)

Gefunden durch Vergleich des Spielstands mit dem RAM nach `continue` (`gutachten-b-ram.py find`, 64-Byte-Fenster der Flags, dann der ganze Bereich). Der Zeiger aus `games.yaml` (`location.pointer`) zeigt auf den **allgemeinen Block des Spielstands im RAM**, mit einem kleinen Kopf davor. Der Ort, den `g4emu` heute liest (`+0x1294` bzw. `+0x1244`), ist der Ort im allgemeinen Block (`0x1280` bzw. `0x1234`).

| Spielcode | Zeiger (fest) | Kopf | Variablen ab | Flags ab | Größe | Werte des Zeigers je Start |
|---|---|---|---|---|---|---|
| CPUD Platin DE | `u32 @ 0x02101EE0` | `0x14` | `*p + 0x14 + 0xDAC` (288 × u16) | `*p + 0x14 + 0xFEC` (364 Byte) | 940 Byte | `0x0227E340` bis `0x0227E364` (4 Starts) |
| IPKD HeartGold DE | `u32 @ 0x02111860` | `0x10` | `*p + 0x10 + 0xDE4` (368 × u16) | `*p + 0x10 + 0x10C4` (364 Byte) | 1 100 Byte | `0x0227C1E8` bis `0x0227C29C` (4 Starts) |
| IPKE HeartGold US | `u32 @ 0x02111880` | `0x10` | wie IPKD | wie IPKD | 1 100 Byte | `0x0227C224`, `0x0227C2AC` (2 Starts) |

Flag `n` ist Bit `n % 8` von Byte `n / 8`, Variable `v` ist u16 an `2 × (v − 0x4000)`. Die Offsets im Block (`0xDAC`/`0xFEC`, `0xDE4`/`0x10C4`) kenne ich aus PKHeX (SAV4Pt/SAV4HGSS), aus dem Gedächtnis. Gemessen bestätigt sind sie dadurch:

- Der Bereich hinter dem Zeiger gleicht dem Spielstand bis auf 5 bzw. 4 von 940 bzw. 1 100 Byte (kartenlokale Werte nach dem Laden).
- Die Abstände passen zu `VARS_END`.
- Gesetzt sind 114 bzw. 148 Flags, viele davon `FLAG_HIDE_…`, die das Neues-Spiel-Script setzt.

Eine zweite, exakte Kopie des Spielstands liegt bei `0x0236D6xx` (Pt) bzw. `0x023775xx` (HG). Das ist der Lesepuffer der Karte; er ist nicht lebendig und darf nicht benutzt werden.

**Stabilität:**

- Innerhalb einer Sitzung bleibt der Zeigerwert gleich. Gemessen über einen Kartenwechsel (415 → 414 → 415 bzw. 64 → 63) und 3 000 Bilder, 14 Proben je Spiel, je ein Wert.
- **Zwischen zwei Starts wandert der Block um bis zu `0xB4` Byte** (Heap; trotz gleicher Eingaben).
- Also: immer über den festen Zeiger, nie über eine feste Adresse. Vor dem Laden des Spielstands ist der Zeiger ungültig (Platin gültig ab Bild 107, HeartGold ab 193, `games.yaml`).

### 4.3 Schreiben (gemessen, mit Gegenprobe)

| Probe | Aktion | Ergebnis | Gegenprobe ohne Schreiben |
|---|---|---|---|
| Platin, Flag `0x173` (Sichtbarkeit von Objekt 0 im Zimmer, Header 415) | im RAM gelöscht, Treppe hinunter (414) und wieder hinauf | Flag bleibt 0; **die Figur erscheint** (Bildunterschied im Kasten x 152–171, y 94–106) | Flag 1, Figur fehlt |
| HeartGold, Flags `0x2A9`–`0x2AB` (drei Objekte im Zimmer, Header 64) | gelöscht, hinunter (63) und wieder hinauf | **alle drei Figuren erscheinen** (Kasten x 181–235, y 143–173) | Flags 1, Figuren fehlen |
| Platin, Variable `0x40A5` (Auslöser im Zimmer bei (4,5), Wert 0) | 1 → 0 im RAM, dann nach (4,5) gehen | **der Auslöser feuert**: Figur festgehalten, Text erscheint, der nächste Schritt ist „blockiert bei 4, 5“ | Wert 1: Gang weiter bis (4,6) |

Lesen kostet bei 940 bzw. 1 100 Byte keine messbare Zeit. Ein Schreiben wirkt, sobald das Spiel den Wert liest. Sichtbarkeits-Flags wirken erst beim nächsten Laden der Karte; die Fläche muss das sagen („wirkt beim nächsten Betreten“).

### 4.4 Varianten

| | (a) nur lesen und anzeigen | (b) lesen und schreiben | (c) später |
|---|---|---|---|
| Was die App braucht | einen Emulator, dessen RAM sie erreicht | ebenso, dazu Schreiben und Undo-Grenzen (ein Live-Wert ist kein Projektschritt) | – |
| Weg 1: `g4emu` als Begleitprozess (JSON-RPC über stdio, wie `studio mcp`) | Python-Umgebung und Kern beim Nutzer (heute nur Entwickler-Werkzeug, GPL-Kern per Download); **kein Fenster**: Bilder müssten in die Schau gestreamt werden (256×384×4 Byte × 60/s ≈ 23 MB/s über IPC), dazu Ton und Eingabe | ebenso | – |
| Weg 2: libretro-Kern im Tauri-Prozess (Rust-FFI wie `emu.py`) | eigener Frontend-Teil: Bild in die Schau (Canvas oder geteilter Speicher), Ton (cpal), Eingabe, Speichern. Der Kern bleibt ein separat geladener GPL-3-Download wie heute bei `setup.sh` | ebenso | – |
| Weg 3: externer melonDS 1.1 mit GDB-Stub (TCP 3333, `m`/`M`-Pakete) | kein Fenster-Bau, der Nutzer spielt in seinem melonDS. **Ungeprüft:** ob der Stub Speicher liest, ohne anzuhalten; sonst kurz anhalten, lesen, weiter. Der Nutzer muss den Stub einschalten | ebenso | – |
| Aufwand | Weg 2: L (geschätzt 1 500 bis 2 500 Zeilen Rust und TS); Weg 3: M (300 bis 600 Zeilen, nach einer Probe) | +S | – |
| Risiko | Weg 2: Leistung im Webview, Ton, Lizenzgrenze; Weg 3: Verhalten des Stubs, Abhängigkeit von der melonDS-Version | Ein falsches Schreiben ändert den Spielstand des Nutzers erst beim Speichern im Spiel, also gering | – |

Unabhängig davon ist **jetzt** billig: `g4emu` bekommt die Schritte `flag N`, `set-flag N 0|1`, `var N`, `set-var N W`, `expect-flag N W` und `expect-var N W`. `games.yaml` bekommt `save_block: {pointer, header, vars, flags}` je Spielcode. Das sind etwa 120 Zeilen Python und Tests, und es gibt den Szenarien von 3d.1 und Geschichten ein Gate „Flag nach dem Gespräch gesetzt“ ohne Bildvergleich. Die Lage gehört zusätzlich in ein Kernel-Metadatum, etwa `metadata/save_layout.yaml`, damit eine spätere App-Lösung dieselben Zahlen nutzt.

**Empfehlung (recommend): (c) für die App, mit sofortiger Vorarbeit.**

- Spielmodus live in der App wird eine eigene, bedingte Teilphase **3d.6** wie in Gutachten C, frühestens nach 3d.1. Zuerst kommt eine Probe von Weg 3 (GDB-Stub von melonDS 1.1, eine Stunde Hand). Trägt sie, wird es Weg 3 mit (b) (M); sonst Weg 2 (L), dann eher nach 3d.
- In 3d.1 kommen die RAM-Lage als Metadatum und die `g4emu`-Schritte zum Lesen, Schreiben und Erwarten für die Gates. Die Fläche bekommt die Spalte „Live“ als leeren Platz.

---

## 5. Ei-Attacken (Frage 5)

### 5.1 Befund (gemessen mit `gutachten-b-eier.py`)

Format in beiden Spielen (uxie 0.9.7 `egg_move_data`): eine Folge von u16. `20000 + Art` beginnt einen Eintrag, es folgen die Attacken, `0xFFFF` beendet die Tabelle.

| | HeartGold DE | Platin DE |
|---|---|---|
| Ort | NARC `a/2/2/9`, **ein** Member (`unpacked/eggMoves/0000`), in `archives.yaml` schon als Pack-Eintrag | `arm9_overlays/ov005.bin` (im DSPRE-Ordner unkomprimiert, `overlays.yaml` `compressed: false`), Datei-Offset **`0x2923E`**–`0x2A12A`; Overlay-Basis `0x021D0D20`, im RAM also `0x021F9F5E` |
| Größe | 4 092 Byte, die Tabelle füllt den Member bis zum letzten Byte | **3 820 Byte, genau voll**: 198 Marken + 1 711 Attacken + Ende = 1 910 u16 |
| Arten / Attacken / höchstens je Art | 198 / 1 847 / 14 | 198 / 1 711 / – |
| uxie | liest und schreibt den Member | Test-Offset `0x29222` (`binary.rs:331`): gilt für eine andere Sprachfassung, nicht für DE (Abstand `0x1C`) |
| DSPRE | liest und schreibt den Member | verschiebt die Tabelle per Patch nach `files/zcustom/egg_moves.narc` und biegt den Code dorthin um (`archives.yaml:80`, dspre-report §1) |

### 5.2 Lesen und Schreiben ohne DSPRE-Patch

- **HeartGold:** wie ein gewöhnlicher Daten-Member. Die Länge ist frei, weil es ein NARC ist: Codec in `g4-formats/src/data/` nach dem Muster der anderen, Store-Art, Textprojekt-Teil.
- **Platin in Overlay 5:**
  - Lage je Spielcode in einem Metadatum (Offset, Budget 3 820 Byte), mit Signaturprüfung vor jedem Lesen und Schreiben: Die Folge beginnt bei `20001` und endet mit `0xFFFF` genau am Budgetende. Fällt die Prüfung durch, ist das Feld gesperrt, mit Grund.
  - Schreiben **innerhalb des Budgets**: Attacken tauschen geht immer; hinzufügen nur, wenn anderswo gleich viele entfallen. Die Anzeige zeigt dazu ein Budget („1 910 von 1 910 Einträgen“).
  - Die Überlauf-Verschiebung gibt es nur über den DSPRE-Patch.
- **Patch-Schutz:**
  - Gibt es `files/zcustom/egg_moves.narc`, liest und schreibt das Studio den NARC und fasst Overlay 5 nicht an.
  - Fehlt er und die Signatur stimmt, wird das Overlay bearbeitet.
  - Sonst ist die Tabelle gesperrt.
- **Textprojekt:** Feld `egg_moves` in der Spezies-JSON (`data/species/NNNN.json`). Das passt zu „Entwicklung und Lernset“ im Pokédex.

**Aufwand (geschätzt):** Codec und Overlay-Zugriff mit Signatur und Budget etwa 300 bis 400 Zeilen Kernel, Textprojekt-Feld und Migration etwa 150, App-Reiter „Ei-Attacken“ mit Attacken-Picker etwa 250, Tests. Insgesamt S+ bis M−.

**Empfehlung (recommend): nicht in 3d.1.** Ei-Attacken gehören in die Pokédex-/Sprite-Teilphase, die Gutachten C als **3d.5** vorschlägt, nicht zu den Flags. Bedingung: Das Overlay-Schreiben mit Signatur und Budget wird dort als kleiner allgemeiner Baustein gebaut (`metadata/overlay_tables.yaml`), den später Attacken-Lehrer und Läden wiederverwenden. Wird 3d.5 zu groß, kommt nur das Lesen für beide Spiele (S−); das Schreiben folgt mit den Overlay-Tabellen „später“.

---

## 6. Schnitt-Vorschlag (Frage 6)

### 3d.1 „Flags und Variablen“ (M)

**Kernel:**

- `SymbolKind::Flag`/`Var` mit fester Tabelle je Familie aus scrcmd;
- Register `data/ids/{flags,vars}.json` und dünne Entitäten `data/{flags,vars}/NNNN.json` (Schema-Version je Art);
- `.g4-ids-flags.json`/`-vars.json` in `include.rs`;
- Präfixe in `rename.rs`;
- `Target::Flag`/`Var` mit `access`, Regeln in `metadata/script_flagvars.yaml`;
- neue Quellen: Bedingungen, Events (Flag, versteckte Items, Auslöser), Level-Scripts, Trainer-Flags, Rezepte;
- `usage()`; `refresh` gebündelt.

Geschätzt 1 200 bis 1 600 Zeilen mit Tests.

**App:**

- Fläche „Flags und Variablen“ (Liste, Detail, Wege);
- Picker „Flag“/„Variable“, neue Felder im Event-Inspector und im Level-Script;
- „Umbenennen…“, „Als eigenes Flag benennen…“.

Geschätzt 900 bis 1 200 Zeilen.

**Werkzeug:** `games.yaml` `save_block` und die `g4emu`-Schritte aus §4.4, `metadata/save_layout.yaml`. MCP: `list_flags`, `flag_usage`, `free_flag`, `rename_id`.

**Gate 3d.1:**

- **Gated je Spiel (Pt DE, HG DE)** auf einer frischen Kopie nach `adopt`:
  - Die Liste zählt die Namen der Metadaten (4 097 bzw. 2 244 Nummern).
  - `flag_usage` einer Stichprobe stimmt mit einem Vergleichstest über die Quellen überein (die Zahlen aus §1.2 und §1.3 als Erwartung ±0).
  - Der warme Neuaufbau des Index bleibt ≤ 1 000 ms (strikt im Zeitschritt), 200 Abfragen ≤ 50 ms.
- **Eigener Name:**
  - Ein freies Flag benennen; der Name steht im Register, in `data/flags/` und in `.g4-ids-flags.json`.
  - Ein Script mit `if FLAG_<NAME> then` kompiliert byte-gleich zur Fassung mit dem pret-Namen.
  - Umbenennen zieht die Quellen nach, ein Undo-Schritt.
  - Im DSPRE-Layout ist „Umbenennen…“ gesperrt, mit Grund.
- **Szenario** (`pt-flag.yaml`, `hg-flag.yaml`):
  - Eine Figur mit neuem Sichtbarkeits-Flag (Picker) und ein Script, das das Flag setzt.
  - Bauen, spielen, ansprechen, Karte verlassen und wieder betreten.
  - `expect-flag N 1` über `g4emu`, die Figur ist weg (Bild).
- **Test-Umgebung:** Picker, Liste, Detail und Weg zum Script über die Brücke.
- **Blick:** die Fläche hell und dunkel.

### Was die anderen Teilphasen daraus brauchen

- **Geschichten (Gutachten A, 3d.3):**
  - stabile IDs für Flags und Variablen (Register);
  - `usage()` mit `access`, um Kanten „gesetzt hier → geprüft dort“ zu ziehen (149/159 Flags werden an einem Ort gesetzt und an einem anderen geprüft, Gutachten A);
  - `free_flag`/`free_var` für Rezepte, statt fester Namen wie `done_var`;
  - die `g4emu`-Erwartungen für das Gate.
- **Löschen (Gutachten C, 3d.2):**
  - Ziele `Flag`/`Var` in der deklarativen Kantentabelle.
  - Einen eigenen Namen zu entfernen ist „Datei entfernen + Registereintrag“, gesperrt, solange eine Quelle den Namen schreibt.
  - Einen Trainer zu löschen prüft auch `Flag(0x550 + n)` aus `Set/Clear/CheckTrainerFlag`.
  - Ein Objekt mit Sichtbarkeits-Flag zu löschen ist erlaubt, der Verweis entfällt.
- **Spielmodus live (3d.6, bedingt):** die RAM-Lage aus §4.2 und die Spalte „Live“.

**Empfehlung (recommend):** 3d.1 wie oben (M), ohne Spielmodus live und ohne Ei-Attacken. Die Reihenfolge von Gutachten C passt: 3d.1 ∥ 3d.5, dann 3d.2, 3d.3 und 3d.4; 3d.6 bedingt nach der Probe des GDB-Stubs.

---

## 7. Empfehlungen (Zusammenfassung)

1. **Inventar:**
   - „Frei“ nur aus dem Nutzungsindex (16 Platin-Flags heißen `UNUSED` und sind benutzt).
   - Die Grenzen als Metadaten: Trainer `0x550`–`0x95F`, versteckte Items Platin `0x2DA`/HeartGold `0x320`, Spielstand-Ende `0xB60` (Platin unsicher), Variablen bis `0x411F`/`0x416F`.
2. **Namen (recommend):** Register `data/ids/{flags,vars}.json` + dünne Entitäten `data/{flags,vars}/NNNN.json` mit Beschreibung, kein Standardwert, Pflichtpräfix `FLAG_`/`VAR_`, `.g4-ids-*` vom Kernel, Umbenennen über `rename.rs`, im DSPRE-Layout nur pret-Namen.
3. **Index (recommend):** `Target::Flag`/`Var` mit `access`, deklarative Regeln (Typ + rotoms Namensregel + Bedingungen), Quellen Scripts, Events, Level-Scripts, Trainer und Rezepte, ohne `≥ 0x8000`; `usage()` in einem Durchlauf; `refresh` nicht je Abfrage.
4. **Spielmodus live (recommend):** in der App später (3d.6, bedingt, nach einer Probe des GDB-Stubs von melonDS 1.1). Jetzt die RAM-Lage als Metadatum und die `g4emu`-Schritte zum Lesen, Schreiben und Erwarten.
5. **Ei-Attacken (recommend):** in 3d.5 (Pokédex/Sprites), mit Overlay-Baustein Signatur + Budget und DSPRE-Patch-Erkennung; nicht in 3d.1.
6. **Schnitt (recommend):** 3d.1 „Flags und Variablen“ (M) mit dem Gate aus §6.

---

## 8. Offene Fragen

1. **Platin-Grenze der Flags:** Liegt das Ende des Spielstands wirklich bei `0xB60` wie in HeartGold (gemessen sind 364 Byte nach 288 Variablen), und was bedeuten die 1 184 Namen `FLAG_UNK_0x0B60`–`0x0FFF` der Datenbank? Ein Schreibtest auf `0xB60` würde in den nächsten Block schreiben: nur in einer Testkopie, mit Blick auf den folgenden Block.
2. **HeartGold-Kandidaten** (153 Flags, 82 Variablen mit `UNK`-Namen ohne Script-Verweis): Setzt die Engine einige davon selbst? Ein Abgleich mit `pokeheartgold` (`flags.h`, Verweise in `src/`) klärt das; offline nicht geprüft.
3. **GDB-Stub von melonDS 1.1:** Liest er Speicher, ohne anzuhalten, und wie lange hält er an? Das klärt eine Stunde Hand mit `gdb`/`lldb` gegen Port 3333 und entscheidet Weg 3.
4. **Zugriff in HeartGold:** 2 802 Variablen-Vorkommen haben ohne Angaben der Datenbank keinen sicheren Zugriff. Reicht das Borgen von Platin plus eine Ausnahmeliste, oder soll ein Upstream-Beitrag (`access` in `hgss_v2.json` des scrcmd-database) kommen?
5. **Namen im JSON des Textprojekts** (`hidden_flag`, `var` als `FLAG_…`/`VAR_…`): in 3d.1 mit Migration, oder später?
6. **Platin-Fassungen der Ei-Attacken:** Das Offset unterscheidet sich je Sprache (DE `0x2923E`, uxie `0x29222`). Die übrigen Sprachen sind nicht gemessen; die Signaturprüfung fängt das ab, die Tabelle je Spielcode muss es aber kennen.

---

## Anhang A: Messaufbau und Skripte

Alle Skripte liegen in diesem Ordner. Die Pfade der Kopien waren `Roms/_testkopien/3d-gutachten-b-{pt,hg,ptraw,hgraw}` und `3d-gutachten-b-{pt,hg,hgus}.nds`; alle sind gelöscht.

- **`gutachten-b-namen.py <repo>`:** Bereiche, Läufe, `UNUSED`-Namen und Grenzkonstanten der Flag- und Variablentabellen beider Datenbanken (§1.5).
- **`gutachten-b-inventar.py <repo> <projekt> pt|hgss [--json out]`:** die Vorkommen in rotom-Quellen (Zeilen, Typ- und Namensregel, Zugriff), Events, versteckte Items, Auslöser, Level-Scripts, Vereinigung, frei/benutzt (§1.2 bis §1.5). Python 3, nur Standardbibliothek.
- **`gutachten-b-proto/`** (`Cargo.toml`, `src/main.rs`): Pfad-Abhängigkeit auf `g4-kernel`, dazu rotom `v0.7.0`. Gebaut mit `CFLAGS=-D_POSIX_C_SOURCE=200809L` und eigenem `CARGO_TARGET_DIR`, die `Cargo.lock` des Workspace kopiert (nicht abgelegt); Laufzeit braucht `DYLD_LIBRARY_PATH` auf `…/release/deps` (für `libnitroarc_ffi.dylib`).
  - `index <projekt>`: `rebuild_full` kalt und 4× warm, Verweise je Quelle, 200 Abfragen.
  - `flagvar <projekt> <scrcmd.json>`: rotoms Parser, Flag- und Variablen-Erkennung nach Typ, Namen und Bedingungen, Zeiten getrennt nach Parsen und Erkennen (§3).
  - Der vorhandene gated Test lief zusätzlich als Release-Binary `target/release/deps/refs-b2f0f6bf259fa38e vanilla_index_builds_under_a_second` mit `G4_TEST_PROJECT` (§3.1).
- **`gutachten-b-ram.py <rom> <save> <out> find|stable|control|flagtest|vartest [control]`:** Läuft aus `scripts/g4emu` mit `uv run python …` und nutzt `g4emu` als Bibliothek. Der Spielstand wird zuerst in den Ausgabeordner kopiert.
  - `find`: Lage im RAM.
  - `stable`: Zeiger über einen Kartenwechsel und 3 000 Bilder.
  - `flagtest`/`control`: Sichtbarkeits-Flag löschen bzw. nicht, Karte wechseln, Bild.
  - `vartest [control]`: Variable des Auslösers auf 0 bzw. unverändert, auf die Trigger-Kachel gehen (§4).
  - Die Bilder lagen im Scratchpad und sind gelöscht.
- **`gutachten-b-eier.py member <datei> | scan <overlay>`:** zerlegt die Ei-Attacken-Tabelle bzw. sucht sie im Overlay (§5).
- **Probe eigener Namen (§2.1, von Hand):** in der Platin-Kopie in `expanded/scripts/0412.rotom` fünf Zeilen vorn in `script_1` (`SetFlag`, `if … then ClearFlag … endif`, `SetVar`), einmal mit pret-Namen, einmal mit Namen aus `.rotom/command_database/gutachten-namen.json`, je `g4k build`, Vergleich der SHA-1 von `unpacked/scripts/0412`.
