# Gutachten A: Textprojekt (Phase 4.3)

Stand: 26.10.2026. Gutachter: Architektur-Review (nur lesend am Code, mit Messungen). Repo: `/Users/POLTEP/Projekte/lab/editor/gen4-studio`, Branch `main` (`2c79058`).
Datenbasis: frische APFS-Klone der drei Vanilla-Projekte (Platin DE `CPUD`, HeartGold DE `IPKD`, HeartGold US `IPKE`) unter `Roms/_testkopien/4-gutachten-a-*`, nach der Messung gelöscht. Die Originale unter `Roms/` blieben unberührt.

Werkzeuge in diesem Ordner (sie geben nur Zahlen aus, keine ROM-Inhalte):
- `gutachten-a-proto.rs` mit `gutachten-a-proto.Cargo.toml`: Prototyp in Rust mit Pfad-Abhängigkeit auf `crates/g4-kernel` und `crates/g4-formats`. Er exportiert jede lesbare Entität als eine JSON-Datei und die binären Teile als Blob, liest alles zurück, codiert mit den Codecs des Kernels und vergleicht Byte für Byte. Er misst auch die Zeiten. Für Texte nimmt er `text::write_archive` (chatot), für Scripts `scripts::adopt` und `scripts::compile` (rotom).
- `gutachten-a-groessen.py`: Größen und Dateizahlen aus der Ausgabe des Prototyps.
- `gutachten-a-namen.py`: Kollisionen symbolischer IDs nach Normalisierung, Abhängigkeit von der Sprache.
- `gutachten-a-fingerprints.py`: was das Erkennen von DSPRE-Änderungen kostet (stat, Hash).

„Gemessen“ heißt unten: an den Testkopien mit diesen Werkzeugen ermittelt. „Geschätzt“ ist ausdrücklich gekennzeichnet. Alle Zeiten stammen von einem Rechner, auf dem zur selben Zeit zwei weitere Gutachter-Agenten bauten und kopierten. Sie streuen deshalb um etwa ±50 %. Der Prozess brauchte nur 1,3 bis 1,6 s CPU (user), der Rest ist Ein- und Ausgabe.

---

## 1. Anlass

Die Roadmap (Abschnitt „Phase 4“, Teilphase 4.3 „Textprojekt“, Größe L) verlangt eine JSON-Datei je Entität, symbolische IDs, Schema-Version mit Migrationen, den Export DSPRE-Layout → Textprojekt → DSPRE-Layout mit byte-gleichem Rebuild als Gate und einen weiter nutzbaren DSPRE (Änderungen von außen werden erkannt und eingelesen). Die Design-Spezifikation (`docs/superpowers/specs/2026-10-21-gen4-studio-design.md`, §3) legt zwei Dinge fest: „DSPRE-Projektformat kompatibel lesen und schreiben“ und „Ab Phase 4: Textprojekt (eine Datei pro Entität, symbolische IDs) mit Build zum ROM“. Offen sind laut Roadmap: wie DSPRE-Änderungen mit dem Textprojekt als Quelle zusammengehen und welche Namenstabellen für symbolische IDs auf Pt, HG und SS stabil sind.

---

## 2. Befund

### 2.1 Entitäten im Kernel heute

`AssetKind` (`crates/g4-kernel/src/project.rs:25`) kennt 25 Arten. Der Store hält editierbare Werte als `Loaded` (`store.rs:78`) und decodiert sie in `Loaded::from_bytes` (`store.rs:136`). Den Ort auf der Platte liefert `Project::asset_location` (`project.rs:538`). Eine wichtige Regel gibt es schon: Was der Codec nicht byte-gleich zurückschreiben kann, wird gesperrt (`unwritable_reason`, `store.rs:380`; `decode`, `store.rs:239`). Ein Datensatz mit falscher Länge bleibt als `Loaded::Raw` erhalten. **Eine serde-Schicht für die Formate gibt es nicht:** `g4-formats` hängt nicht von serde ab (`crates/g4-formats/Cargo.toml`). Die JSON-Formen für die App sind DTOs in der Tauri-App (`apps/studio/src-tauri/src/commands/data/dto.rs`, z. B. `DataDocDto` Z. 315), auf die UI zugeschnitten und mit Namen angereichert. Nur uxie-Typen wie `MapHeader` (uxie 0.9.7 `src/map_header/types.rs:7`) haben serde.

| Entität | Quelle im DSPRE-Layout | Kodierung im Kernel | Unbekannte Bytes (gemessen, Pt / HG) | Form im Textprojekt |
|---|---|---|---|---|
| **Header** | Pt/HG: Ausschnitt aus `arm9/arm9.bin` (Offset `metadata/header_tables.yaml`), mit Patch „Dynamische Header“ `unpacked/dynamicHeaders/NNNN`; interner Name in `files/fielddata/maptable/mapname.bin` (16 Byte) | uxie `MapHeader` vollständig, `HeaderTable` als ein Asset (`project.rs:37`, `store.rs:149`) | Die Felder heißen „unknown“, tragen aber Werte: Pt `unknown1` ≠ 0 in 579 von 593, HG `unknown0` in 540 von 540 und `unknown1` in 532 | JSON je Header, mit internem Namen; unbekannte Felder benannt (`unknown0/1`) |
| **Matrix** | `unpacked/matrices/NNNN` | `g4_formats::matrix::Matrix` vollständig | keine | JSON (Gitter als Zeilen) |
| **Karte** | `unpacked/maps/NNNN` | `MapFile`: Permissions und Gebäude decodiert; NSBMD, BDHC und HG-BGS verbatim (`crates/g4-formats/src/map.rs:51`) | `Building.dummy` ≠ 0 in 6 Karten (Pt), `buildings_tail` in 1 Karte (Pt 506, `0D 0A`), HG-BGS-Block > 64 Byte in 85 Karten | JSON (Permissions, Gebäude) **plus Blobs** `.nsbmd`, `.bdhc`, HG `.bgs` |
| **Area-Data** | `unpacked/areaData/NNNN` | `AreaData` vollständig, 8 Byte | Pt `unknown` ≠ 0 in 47 von 75 | JSON |
| **Eventdatei** | `unpacked/eventFiles/NNNN` | uxie `EventFile`; das 2-Byte-Padding eines BG-Events wird beim Schreiben null gesetzt (uxie `event_file/binary.rs:7`). Abweichende Dateien sperrt der Store | Vanilla: keine (Test `vanilla_event_files_round_trip_byte_identically`, `tests/store.rs:449`) | JSON |
| **Textarchiv** | `unpacked/textArchives/NNNN` und chatot-JSON `expanded/textArchives/NNNN.json` (`project.rs:508`, `text.rs:1`) | chatot, der Member ist die Ableitung der JSON, die Laderegel „neuere Datei gewinnt“ (`text_winner`, `text.rs:252`), `encode_stale` (`text.rs:447`) | keine | **bleibt chatot-JSON** (DSPRE-kompatibel, schon kanonisch: `TextArchiveDoc::to_json`, `text.rs:219`) |
| **Script** | `unpacked/scripts/NNNN`; Quelle rotom `expanded/scripts/NNNN.rotom` und `.json` (Level-Scripts), `rotom.toml`, `.rotom/` (`scripts.rs:1`, `SOURCE_ROOT` Z. 30) | rotom 0.7.0, `adopt` (Z. 249), `compile` (Z. 600) | **Quirks** in `.rotom/status/compile-state.json`: Pt 4 Dateien (0101, 0164, 0365 `jump_table_end_marker_count: 0`, Level-Script 0620 `levelscript_padding: 4`), HG 1 (0907, Endmarke 2). Ohne sie wird der Member nicht byte-gleich (`saved_quirks`/`restore_quirks`, `scripts.rs:403/421`) | **bleibt rotom-Quelle**; die Quirks wandern in eine kleine Projektdatei |
| **Begegnungen** | `unpacked/encounters/NNNN` | `fixed_record` mit Feldkarte (`data/encounter.rs:17/161`), Rohbytes bleiben (`data/record.rs:1`) | Padding-Felder (Pt 21 Arten) alle 0 | JSON (Feldkarte) |
| **Pokémon-Daten** | `personalPokeData`, `learnsets`, `evolutions` (je 508) | `Personal`/`Evolution` als `fixed_record`, `Learnset` mit `tail` und `terminated` (`data/learnset.rs:22`) | keine | **ein** JSON je Spezies oder Form (wie `DataDocDto::Species`) |
| **Attacke** | `moveData` (471) | `fixed_record` | keine | JSON |
| **Item** | `itemData` (Pt 446, HG 514); Item-ID → Member über die arm9-Tabelle | `fixed_record` | keine | JSON je Item-ID, Member-Zuordnung aus der arm9-Tabelle |
| **Trainer** | `trainerProperties` + `trainerParty` (Pt 928, HG 738) | `TrainerDoc` (`data/trainer_doc.rs:22`), Paar als ein Asset (`project.rs:77`) | `unknown` und `flags_rest` in Vanilla 0; alle Paare kanonisch | JSON (Eigenschaften und Team) |
| **Trainer-Texte** | `trainerTextTable`, `trainerTextOffset` (je 1 Member) | `TrainerTextTable`/`Offsets` | keine | zwei JSON, besser später in den Trainer gefaltet |
| **Texturen, Gebäudemodelle** | `mapTextures`, `buildingTextures`, `exteriorBuildingModels`, HG `interiorBuildingModels` | nicht editierbar (`NotEditable`, `store.rs:175`) | – | **Blob mit Hash** |
| **Icons** | `itemIcons`, `monIcons` | nur gezählt | – | bleiben in der Basis |
| **Weltwissen** | `g4-world.json` (`world_meta.rs`, `WORLD_FILE` Z. 18) | serde, `version: 1` (Z. 21), robust gelesen (Z. 75) | – | gehört schon zum Projekt, bekommt symbolische Header-IDs |
| **Ignorierliste** | `g4-checks.json` (`checks/ignore.rs:20`) | serde, **ohne Version** | – | dito; `subject`/`key` nennen Assets heute mit Nummern |
| **Basis-Marke** | `.g4/base.json` (`start.rs:221`) | serde, `version: 1` | – | bleibt in `.g4/` oder geht ins Manifest (§4) |
| **Orte** | Ortsnamen-Bank des Textarchivs (`places.rs:19`, uxie `TextBankIds`) | über den Text | – | Teil der Texte; symbolische ID `PLACE_*` |
| **Vorlagen** | `metadata/templates.yaml` (Starttown als interner Name `T01`/`T20`, Starter als Zahl, Variable als Name der Befehlsdatenbank) | Rezept im Repo | – | nimmt symbolische IDs zusätzlich an |

Nicht im Kernel abgebildet und daher Teil der „Basis“: weitere Member unter `unpacked/` (Pt 9.363 von 18.004, HG 11.238 von 19.497: Sprites, Kampfgrafik, Höhen, Battle Tower u. a.), dazu `arm9/` außerhalb der Header-Tabelle, die Overlays und `files/` außerhalb der NARCs.

### 2.2 Prototyp: Export, Import und Byte-Vergleich (gemessen)

Ein Lauf je Spiel: Export aller Entitäten aus §2.1, Rücklesen, Codieren mit den Codecs des Kernels und Vergleich mit dem Member. Texte laufen durch chatot mit `text::write_archive`, Scripts durch rotom (`adopt` exportiert, dann werden alle Member mit Quelle gelöscht und `compile` erzeugt sie neu).

**Ergebnis: Jede Entität kommt auf allen drei Projekten byte-gleich zurück.**

| Entität | Pt | HG DE | HG US |
|---|---|---|---|
| Header | 593/593 | 540/540 | 540/540 |
| Matrizen | 289/289 | 288/288 | 288/288 |
| Karten (JSON und Blobs) | 666/666 | 676/676 | 676/676 |
| Area-Data | 75/75 | 106/106 | 106/106 |
| Eventdateien | 534/534 | 491/491 | 491/491 |
| Textarchive (chatot) | 724/724 (46.053 Nachrichten) | 829/829 (49.984) | 829/829 |
| Scripts (rotom, mit Quirks) | 1.124/1.124 | 965/965 | 965/965 |
| Begegnungen, Personal, Lernsets, Entwicklungen, Attacken, Items | alle | alle | alle |
| Trainer (Paare), Trainer-Texte | 928/928, 2/2 | 738/738, 2/2 | 738/738, 2/2 |

Ein zweiter Export ergab dieselben Dateien wie der erste (geprüft für Texte, Karten und Events mit `diff -r`): Der Export ist deterministisch.

Zwischen HG DE und HG US unterscheiden sich nur die 829 Textarchive und 4 Karten (0468, 0473, 0475, 0476, im NSBMD, vermutlich Schilder mit Schrift). Header, Matrizen, Events, Begegnungen und alle Daten sind byte-gleich. Daraus folgt eine Anforderung an die IDs (§2.4).

### 2.3 Größe und Dateizahl (gemessen; `gutachten-a-groessen.py`)

| | Pt DE | HG DE | HG US |
|---|---|---|---|
| Lesbare Dateien (JSON und rotom) | 7.559 | 7.286 | 7.286 |
| davon Texte | 724 Dateien, 5,7 MiB | 829 Dateien, 6,1 MiB | 829 Dateien, 5,9 MiB |
| davon Karten-JSON (Permissions als 32 Zeilen) | 666 Dateien, 4,3 MiB | 676 Dateien, 4,2 MiB | 676 Dateien, 4,2 MiB |
| davon rotom-Quellen | 1.124 Dateien, 1,7 MiB | 965 Dateien, 2,1 MiB | 965 Dateien, 2,1 MiB |
| Lesbar gesamt | 16,2 MiB (zlib 2,9 MiB) | 16,7 MiB (zlib 3,0 MiB) | 16,5 MiB (zlib 2,9 MiB) |
| Blobs Karten (NSBMD, BDHC, HG BGS) | 1.332 Dateien, 14,2 MiB | 1.437 Dateien, 17,6 MiB | 1.437 Dateien, 17,6 MiB |
| Blobs Texturen und Gebäudemodelle | 735 Dateien, 5,6 MiB | 772 Dateien, 7,2 MiB | 772 Dateien, 7,2 MiB |
| Blobs gesamt | 2.067 Dateien, 19,8 MiB | 2.209 Dateien, 24,8 MiB | 2.209 Dateien, 24,8 MiB |
| Zum Vergleich: DSPRE-Ordner (Bytes, mit `expanded/` und `.rotom/` nach dem Lauf) | 20.036 Dateien, 147 MiB | 21.419 Dateien, 178 MiB | 21.419 Dateien, 177 MiB |
| `.rotom/` (Befehlsdatenbank 756 KiB, Status 384 KiB) | 1,1 MiB | 0,9 MiB | 0,9 MiB |

Fasst man Personal, Lernset und Entwicklung zu einer Datei je Spezies zusammen, sinkt die Zahl der lesbaren Dateien um 1.016: Pt etwa 6.540, HG etwa 6.270 (geschätzt aus den Zählungen). Die zlib-Größe ist ein Anhaltspunkt für Git-Objekte; Git packt mit Deltas noch dichter.

### 2.4 Zeiten (gemessen, Summe der Abschnitte)

| | Pt DE | HG DE | HG US |
|---|---|---|---|
| Projekt öffnen | 3 ms | – | – |
| Export, Summe der Abschnitte (ohne das Lesen der Member) | 5,6 s | 4,7 s | 5,5 s |
| davon `scripts::adopt` (rotom, alle) | 1,5 s | 1,2 s | 1,5 s |
| Import, Codieren und Vergleich, Summe | 16,5 s | 11,5 s | 15,8 s |
| davon Texte, chatot je Archiv (über Temp-Dateien und atomares Schreiben) | 5,9 s (8 ms je Archiv) | 5,0 s | 5,2 s |
| davon Scripts, `compile` aller Quellen | 5,7 s | 3,1 s | 7,4 s |
| davon Karten | 1,7 s | 1,1 s | 0,6 s |
| Lauf gesamt mit Lesen, Export, Import, Vergleich, rotom | 41,0 s | 29,2 s | 37,4 s |

**Geschätzt:** Ein voller Export beim ersten Öffnen eines DSPRE-Ordners dauert in Rust mit den Lesezeiten 10 bis 20 s; ein voller Import (frischer Klon, Basis-ROM entpackt) 15 bis 30 s plus die Extraktion, die hier nicht gemessen wurde. Ein Import, der nur Geändertes übernimmt, braucht je Entität Millisekunden (Mittel aus den Messungen: unter 1 ms je Daten-JSON, rund 8 ms je Textarchiv, rund 5 ms je Script, das rotom übersetzt). Beides ist I/O-gebunden und lässt sich parallelisieren; der Prototyp läuft seriell.

### 2.5 Erkennen von DSPRE-Änderungen: Kosten (gemessen; `gutachten-a-fingerprints.py`, Python, warm und kalt)

| | Member | stat (mtime und Größe) | SHA-1 aller Member | BLAKE2b |
|---|---|---|---|---|
| Pt, ganzes `unpacked/` | 18.004 (42,9 MiB) | 62 ms warm, 314 ms kalt | 1,5 s warm, bis 28,7 s kalt unter Last | 1,4 s warm, 2,4 s kalt |
| Pt, nur abgebildete Ordner | 8.641 (25,9 MiB) | 30 ms | 0,7–0,8 s | 0,8 s |
| HG DE, ganzes `unpacked/` | 19.497 (51,4 MiB) | 66 ms warm, 328 ms kalt | 1,5 s warm, 27 s kalt | 1,4 s warm, 15 s kalt |
| HG DE, nur abgebildete Ordner | 8.259 (31,2 MiB) | 29 ms | 0,6 s | 0,6 s |

Folgerung: Ein stat-Lauf beim Öffnen kostet unter 0,1 s. Zu hashen sind nur Member mit geänderter mtime oder Größe. Volles Hashen ist kalt zu teuer für jedes Öffnen. Ein `git checkout` setzt allerdings die mtime aller berührten Dateien neu; ab da entscheidet der Hash, ob sich wirklich etwas geändert hat.

### 2.6 Symbolische IDs: Kollisionen und Sprache (gemessen; `gutachten-a-namen.py`)

Normalisierung: chatot-Markup entfernen, `♂/♀` → `_M/_F`, deutsche Umlaute → `AE/OE/UE/SS`, übrige Diakritika entfernen, alles Nicht-Alphanumerische → `_`, Großschreibung. Leere Namen (z. B. „???“, „-----“) zählen extra.

| Bank | Pt DE | HG DE | HG US | Bemerkung |
|---|---|---|---|---|
| Spezies (496 Namen) | 0 Kollisionen, 1 leer | 0, 1 leer | 0, 1 leer | Formen (508 Personal-Member) aus `metadata/species_forms.yaml` |
| Attacken (468) | 0, 1 leer | 0, 1 leer | 0, 1 leer | `moveData` hat 471 Member |
| Items (Pt 468, HG 537) | 0, **22 leer** | 0, 23 leer | 0, 23 leer | leere/„???“-Plätze brauchen eine Nummern-ID |
| Fähigkeiten (124), Typen (18) | 0, 1 leer | 0, 1 leer | 0, 1 leer | – |
| **Trainer nur nach Name** | 117 IDs kollidieren, 600 von 928 betroffen, größte Gruppe 104 | 79 IDs, 376 von 738, größte 70 | 83 IDs, 384, größte 70 | unbrauchbar |
| **Trainer nach Klasse und Name** | 149 IDs, 581 betroffen, 432 bräuchten ein Suffix, größte Gruppe 54 | 90 IDs, 366, 276 Suffixe, größte 31 | 90, 366, 276 | Rematches und namenlose Gegner (Rüpel) |
| Trainerklassen (Pt 105, HG 129) | 15 IDs, 49 betroffen | 16, 63 | 15, 63 | ♂/♀-Varianten mit gleichem Namen |
| Ortsnamen (Pt 126, HG 235) | 0 | 2 IDs (4 Namen) | 1 ID (2) | – |
| **Interne Header-Namen** (`mapname.bin`) | **0**, alle 593 verschieden | **0**, 540 | **0**, 540 | **DE == US: 540/540 gleich** |

Sprachabhängigkeit (HG DE gegen HG US, gleicher Index): Gleiche ID hätten Spezies 29 %, Attacken 3 %, Items 23 %, Fähigkeiten 6 %, Typen 11 %, Trainer 2 %, Trainerklassen 23 %, Ortsnamen 37 %. Interne Header-Namen 100 %. Ein Header-Name ist also kein Weg zu einem Header: 80 von 125 (Pt) bzw. 93 von 121 (HG) Ortsnamen hängen an mehreren Headern, bis zu 46 an einem einzigen.

Zu den Referenztabellen:
- pret: `pokeplatinum` (`generated/map_headers.txt`, etwa 593 Namen `MAP_HEADER_JUBILIFE_CITY_POKECENTER_1F`) und `pokeheartgold` (`include/constants/maps.h`, `MAP_NEW_BARK` mit dem internen Namen `MAP_T20` als Kommentar) pflegen lesbare englische Namen von Hand. Beide Repos haben **keine Lizenzdatei** (GitHub-Seiten, abgerufen am 26.10.2026). Die Tabellen zu kopieren, verträgt sich schlecht mit der MIT-Lizenz.
- PokeAPI (`PokeAPI/pokeapi`, BSD-3-Clause) bringt englische Bezeichner (`pikachu`, `thunder-shock`, `potion`) und mit `item_game_indices.csv` die Item-Nummern je Generation (etwa 510 Einträge für Generation 4). `location_game_indices.csv` hat Einträge für Generation 4; ob Pt und HG getrennt sind, ist **nicht geprüft**.
- uxie kann pret-Konstanten zur Laufzeit aus einem Decomp-Checkout lesen (`SymbolTable`, `MapHeaderJson::from_binary`, uxie `src/map_header/json_format.rs`), bringt aber keine Tabellen mit.
- Repo-Regel „Keine ROM-Daten im Repository“ (Spec §6): Eine aus dem ROM gezogene Namensliste darf nicht ins Repo. Bezeichner aus einer frei lizenzierten Fremdquelle sind dagegen keine ROM-Daten.

### 2.7 Versionen in den heutigen Projektdateien

| Datei | Version | Verhalten bei fremder Version | Code |
|---|---|---|---|
| `g4-world.json` | `version: 1`, Standard per `serde(default)` | liest leer und nennt den Grund, überschreibt nicht (R15) | `world_meta.rs:21, 75` |
| `g4-checks.json` | **keine** | eine kaputte Datei ist `InvalidProject` | `checks/ignore.rs:104` |
| `.g4/base.json` | `version: 1` | unlesbar → `None` | `start.rs:224, 250` |
| `rotom.toml` | `format_version = 1` | rotom | `expanded/`-Wurzel |
| `.rotom/status/compile-state.json` | `version: 4` | rotom; enthält Zeitstempel und Hashes (Cache, nicht für Git) | – |

Es gibt also ein Muster (Versionsfeld, robustes Lesen, atomares Schreiben mit `fsutil::write_atomic`, sortierte Listen für kleine Diffs), aber noch keine Migrationskette.

---

## 3. Varianten

### 3.1 Format und Ablage

**A „Alles neu“:** Ein eigener Ordner `data/` enthält jede Entität als JSON, auch Texte und Scripts in einem eigenen Format. DSPRE sieht davon nichts.
- Vorteil: ein einheitliches Schema.
- Nachteil: Texte und Scripts gibt es dann zweimal (eigenes Format und `expanded/` für DSPRE und rotom). DSPRE und der rotom-LSP arbeiten nicht mehr auf der Quelle.

**B „Textprojekt neben dem DSPRE-Ordner, Texte und Scripts bleiben, wo sie sind“ (recommend):**
- Die Projektwurzel ist zugleich der DSPRE-Ordner und die Git-Wurzel.
- Neu sind `g4-project.json` (Manifest) und `data/<art>/…` für alle Entitäten ohne eigenes Quellformat.
- Texte bleiben chatot-JSON in `expanded/textArchives/` und Scripts rotom-Quellen in `expanded/scripts/`. Beide Formate sind schon textuell, kanonisch, DSPRE-kompatibel und laut Messung byte-gleich.
- `unpacked/`, `arm9/`, `files/` und die Overlays werden Build-Zwischenstand oder Basis.
- Vorteil: kein zweites Format für 45 % der lesbaren Bytes. DSPRE, rotom-CLI und LSP arbeiten weiter auf denselben Dateien. Der Store kennt den Weg Quelle → Member schon (`TextSource`, `text_winner`, `compile`).
- Nachteil: Die Projektwurzel mischt Quelle und Zwischenstand. Das löst die `.gitignore`-Vorlage (4.4, Gutachten C).

**C „Nur Abweichungen“:** Im Projekt liegen nur Entitäten, die sich von der Basis-ROM unterscheiden.
- Vorteil: winzig, rechtlich sauber.
- Nachteil: kein vollständiges, lesbares Projekt, kein Diff gegen „vorher“ ohne Basis, Agenten sehen Unverändertes nicht.

**Blobs:** In B liegen binäre Teile (NSBMD, BDHC, BGS, Texturen, Gebäudemodelle; Vanilla 19,8 bzw. 24,8 MiB) **nur, wenn sie von der Basis abweichen**. Das JSON verweist dann auf `{"sha1": …}` der Basis oder auf die Datei im Projekt. Ein Vanilla-Projekt hat damit 0 Blobs. Bauen braucht ohnehin die Basis-ROM (Sprites, arm9, Overlays liegen nur dort). Dasselbe Prinzip gilt für alles Nicht-Abgebildete: Eine Datei unter `unpacked/` oder `files/`, die DSPRE geändert hat und die keine Entität ist, wandert als Blob nach `data/raw/<pfad>` (vergleichbar mit Variante C, aber nur für den Rest).

**Zuschnitt der Entitäten:** Eine Entität ist nicht immer ein Member. Spezies = Personal, Lernset und Entwicklung (eine Datei, wie `DataDocDto::Species`). Trainer = Eigenschaften und Team. Header = Eintrag in der Tabelle und interner Name. Karte = JSON und Blobs. Dateiname ist die Nummer (`data/species/0025.json`). Die symbolische ID steht in der Datei (`"id": "SPECIES_PIKACHU"`) und in einem Register (§3.2). So bleiben die Dateinamen stabil, wenn umbenannt wird; sie sortieren wie im ROM, und die Nummer, die im Spiel Bedeutung hat (Spezies 25 ist im Code fest verdrahtet, Scripts nennen Header-Nummern), steht im Pfad. Die Variante `0025-pikachu.json` ist lesbarer, bricht aber beim Umbenennen und auf case-insensitiven Dateisystemen schneller.

**Schicht im Code:** ein neues Modul `g4-kernel/src/textproject/` mit eigenen Schema-Typen (serde) je Entität, getrennt von den Codecs. Für die zehn `fixed_record`-Arten reicht ein generischer Abbilder über `RawRecord::FIELDS`: Die Felder ohne `Bits` decken jedes Byte genau einmal ab (`data/record.rs:150–163`, `fields_cover_exactly`). Der Prototyp macht das in rund 40 Zeilen. Die Feldnamen werden damit Teil des Schemas; ein Umbenennen in `FIELDS` ist eine Schema-Änderung (Snapshot-Test). Die DTOs der App bleiben getrennt (UI-Form mit Namen).

### 3.2 Symbolische IDs

**ID-Quelle:**
- **I1 Namen aus dem ROM in der Projektsprache** (`MAP_ZWEIBLATTDORF`, `SPECIES_BISASAM`): einfach. Aber dasselbe Spiel auf Deutsch und Englisch hätte andere IDs (gemessen: nur 2 bis 37 % gleich). Ein Diff zwischen HG DE und HG US träfe dann jede Begegnung und jeden Trainer, obwohl sich die Bytes nicht unterscheiden. Vorlagen und MCP-Argumente wären je Sprache verschieden.
- **I2 Feste englische Referenztabellen im Repo** (recommend für Spezies, Formen, Attacken, Items, Fähigkeiten, Typen): aus PokeAPI (BSD-3-Clause, mit Namensnennung in `metadata/`), über Nationaldex und Spielindizes auf die Gen-4-Nummern abgebildet, als `metadata/symbols/*.yaml`. Damit gilt `SPECIES_PIKACHU`, `MOVE_THUNDER_SHOCK`, `ITEM_POTION` in jedem Projekt, jeder Sprache und für Pt wie HG. Für Nummern ohne Eintrag gilt `ITEM_0113` usw. Die 22–23 leeren Items und die Plätze über der Tabelle (Hacks) fallen darunter.
- **I3 Interne Namen aus dem ROM** (recommend für Header): `MAP_T01`, `MAP_C01PC0101`. Gemessen eindeutig auf allen drei Projekten und in DE und US identisch, sprachfrei, im ROM vorhanden (`mapname.bin`) und schon heute von den Vorlagen benutzt (`metadata/templates.yaml:9`, `start_town: T01`). Lesbar wird es über den Klarnamen der Räume aus `g4-world.json` (3e) und über den Ortsnamen, nicht über die ID.
- **I4 Projekt-Taufe:** Für alles ohne feste Quelle (Trainer, Trainerklassen, Orte, später Flags und Geschichten aus 3d) wird die ID beim ersten Export aus dem Namen in der Projektsprache gebildet und danach eingefroren. Beispiele: `TRAINER_JUNGSPUND_JOEY`, `PLACE_ZWEIBLATTDORF`. Trainer nehmen Klasse und Name, bei Gleichstand ab dem zweiten in Nummernreihenfolge `_2`, `_3` (Pt: 432 Suffixe bei 928 Trainern; bei HG 276 von 738). Ein Trainer ohne verwertbaren Namen bekommt `TRAINER_0123`.
- Matrizen, Karten, Eventdateien, Area-Data, Textarchive und Scripts haben keine Namen: Sie behalten Nummern (`MATRIX_0000` nur als Alias, in JSON reicht die Zahl).

**Register und Umbenennen:** `data/ids/<art>.json` hält `ID → Nummer` für alle Arten aus I3 und I4 und für Abweichungen von I2. Wer einen Ort, Trainer oder internen Namen umbenennt, ändert den **Anzeigenamen**, die ID bleibt. Es gibt einen expliziten Befehl „ID umbenennen“, der alle Verweise im Textprojekt in einem Schritt nachzieht (Store-Transaktion). Neue Entitäten bekommen ihre ID beim Anlegen nach derselben Regel. Kollisionsregel: Suffix `_2`, `_3` … in Nummernreihenfolge; kollidiert der Name einer neuen Entität, gilt das nächste freie Suffix. Eine reine Zahl als ID (`SPECIES_0494`) ist immer gültig und eindeutig, so wie heute die Nummern. Beim Lesen nehmen JSON, Vorlagen, `g4-world.json`, `g4-checks.json` und die MCP-Argumente die ID **oder** die Nummer an; der Export schreibt die ID.

**Was nicht geht:** Header über Ortsnamen identifizieren (bis 46 Header je Ortsname). Trainer nur über den Namen (bis 104 Gleichnamige).

### 3.3 Schema, Version, Migrationen

**S1 Version je Datei:** In jeder der rund 7.000 Dateien steht `"schema": 3`.
- Nachteil: Eine Migration ändert jede Datei (riesiger Diff), und einzelne Dateien können verschiedene Stände haben.

**S2 Version im Manifest** (recommend): `g4-project.json` mit `{ "format": "g4-project", "schemaVersion": 1, "game": { "code": "CPUD", "family": "pt", "language": "de" }, "base": { "sha1": "…" }, "studioVersion": "…" }`.
- Eine Version gilt für das ganze Textprojekt. Beim Öffnen: gleich → lesen; älter → Migrationskette `v1→v2→…` als reine Funktionen auf dem Dateibaum (`serde_json::Value`), vorher Hinweis und Schnappschuss (Git-Commit oder `.g4/backup/`), danach `schemaVersion` hochsetzen; neuer → ablehnen mit „mit einer neueren Studio-Version gespeichert“.
- Die Entitätstypen sind `#[serde(deny_unknown_fields)]`: Ein Tippfehler im JSON fällt sofort auf, statt still einen Wert auf 0 zu setzen. Das ist hier wichtiger als Vorwärtskompatibilität, weil das Gate Byte-Gleichheit verlangt.
- `g4-world.json` behält seine eigene Version (Muster bleibt). `g4-checks.json` bekommt `version: 1` mit `serde(default)`, damit alte Dateien gültig bleiben.
- `.g4/base.json` wandert inhaltlich ins Manifest, denn die SHA-1 der Basis gehört in Git; `.g4/` bleibt Cache.
- Tests: je Migration ein synthetisches Fixture-Projekt im Repo (ohne ROM-Daten, Muster `test_support`) mit Erwartung nach der Migration, dazu ein Snapshot der Feldnamen aller Schemas.

**S3 JSON Schema-Dateien** (`$schema`-Verweis): nützlich für Editoren und Agenten und später aus den serde-Typen erzeugbar (`schemars`), aber kein Ersatz für S2. Kann in 4.3b oder später folgen.

### 3.4 Round-Trip und Gate-Test

Heute schon belegt: byte-gleiche Rundwege je Codec (`crates/g4-formats/tests/vanilla.rs`, `vanilla_data.rs`, `crates/g4-kernel/tests/headers.rs:327`, `tests/store.rs:449, 2308`), rotom-Quellen → Member (`tests/scripts.rs:10`), ROM-Rebuild gleich dem ds-rom-CLI (`tests/build.rs:369, 727`) und Projektstart „Original“ → ROM mit gleicher SHA-1 (`tests/project_start.rs:122`). Neu belegt durch den Prototyp: Auch der Weg über JSON ist für alle abgebildeten Entitäten byte-gleich, und chatot codiert alle 724/829 Vanilla-Textarchive byte-gleich aus ihrer JSON (bisher prüfte `member_is_current` nur die Texte, `text.rs:424`).

Wo Abweichungen drohen (alle in Vanilla abgedeckt, für Hacks wichtig):
1. **rotom-Quirks** (5 Scripts in Vanilla) liegen in `.rotom/status/compile-state.json`. Diese Datei ist ein Cache mit Zeitstempeln. Die Quirks müssen in eine Projektdatei (`data/scripts-quirks.json` oder ein Kommentar-Pragma in der Quelle, falls rotom das kann), sonst baut ein frischer Klon 5 Member anders.
2. **rotom-Abhängigkeit von Texten:** `compile` schreibt die Text-JSON, die es braucht (426 Dateien in Pt). Die Reihenfolge im Build-Graph ist also Texte → Scripts → Text-Codierung.
3. **Unbekannte, aber belegte Felder** (Header `unknown0/1`, Pt Area-Data `unknown`, `Building.dummy`, `buildings_tail`, HG-BGS): Sie müssen benannte JSON-Felder oder Blobs sein, nie weggelassen.
4. **Codec-Sperren:** Was der Store sperrt (`unwritable_reason`, `Loaded::Raw`), exportiert das Textprojekt als Blob mit Markierung `"locked": "<Grund>"`, damit nichts verloren geht. Beispiele: Event-Padding ≠ 0, Personal mit falscher Länge, nicht kanonische Trainer-Paare.
5. **Learnset-Terminator** und `tail`, Trainer-Party kanonisch: Sie sind im Codec abgedeckt und gehören als Felder ins JSON (Prototyp: `tail`, `unterminated`).
6. **Text-Kodierung:** chatot ist das Orakel; DSPRE benutzt dieselbe Bibliothek (`text.rs:5`). Fremde Sprachschlüssel (`other`) bleiben verbatim.
7. **Item-Zuordnung:** Item-ID → Member über die arm9-Tabelle. Exportiert wird je Member, die ID kommt aus der Tabelle; ein geteilter Member (zwei IDs, ein Datensatz) muss erkannt werden (nicht gemessen).
8. **Header-Tabelle in `arm9.bin`:** Der Import schreibt nur den Ausschnitt. Der Rest von `arm9.bin` ist Basis oder bei Abweichung Blob.

**Testaufbau (Vorschlag):**
- `textproject_round_trip` (gated, `G4_TEST_PROJECT` und im Gate auf allen drei Projekten): Kopie → Export → alle abgebildeten Member und den Header-Ausschnitt löschen bzw. nullen → Import → Byte-Vergleich aller Dateien unter `unpacked/`, `arm9/arm9.bin`, `mapname.bin` → `build` → ROM-SHA-1 = Basis-SHA-1 (Muster `project_start_create_then_build_matches_the_rom`).
- `textproject_export_idempotent`: Ein zweiter Export ändert keine Datei (`git status` leer). Das ist die Bedingung für saubere Diffs.
- `textproject_from_base`: frischer Ordner aus der Basis-ROM (`g4k extract`) + Textprojekt aus Git → Build → SHA-1 gleich. Das ist der Weg „Projekt aus Git klonen“.
- Proptests je Schema-Typ auf synthetischen Werten (JSON → Wert → JSON).
- Ein Test je Sperrfall aus Punkt 4 (synthetisch).

### 3.5 DSPRE parallel

Lage: DSPRE schreibt `unpacked/<art>/NNNN`, bei Texten und Scripts auch `expanded/`. Der Store erkennt schon heute Änderungen von außen beim Speichern (`KernelError::Stale`, `Store::save`, `store.rs:2417`) und kann neu laden oder die Platte als Basis übernehmen (`reload_from_disk` Z. 2115, `accept_disk_as_base` Z. 2145). Für Texte gilt die mtime-Regel (`text_winner`), für Scripts rotoms Hash-Status (`source_hash`, `output_hash`).

**D1 Nur mtime:** „Neuere Datei gewinnt“ wie bei den Texten.
- Vorteil: billig.
- Nachteil: Ein `git checkout`, ein Kopieren ohne `-p` oder eine Zeitzone machen alles „neu“; ein Konflikt (beide Seiten geändert) ist nicht erkennbar.

**D2 Fingerprints je Member und je Quelle, Drei-Wege-Abgleich** (recommend):
- Beim letzten Export, Import oder Build schreibt der Kernel `.g4/sync.json` (Cache, nicht in Git): je Entität die Member mit `(mtime, size, xxh3)` und die Quelldatei(en) mit `xxh3`. Das ist dieselbe Form wie die Fingerprints des Build-Graphs aus 4.2 (Stufe „unpacked“); beide sollten eine Datei teilen.
- **Erkennen beim Öffnen und bei Fensterfokus:**
  1. stat über die abgebildeten Ordner (gemessen 30 ms).
  2. Bei abweichender mtime oder Größe hashen.
  3. Bei abweichendem Hash gilt der Member als „von außen geändert“.
  4. Ebenso für die Quelldateien (Git pull, Agent, Hand).
- **Einlesen:**
  - Nur Member geändert → decodieren, nach JSON exportieren, Fingerprints nachziehen, Meldung „3 Änderungen aus DSPRE übernommen“ mit Liste.
  - Nur Quelle geändert → normaler Build-Weg (Import in `unpacked/`).
  - **Beide geändert → Konflikt.** Nichts wird überschrieben. Die Liste in Technik zeigt je Entität „Studio-Fassung behalten“, „DSPRE-Fassung übernehmen“ und den Diff der beiden JSON (der DSPRE-Stand wird dafür nur in den Speicher exportiert).
  - Ein Member, der nicht decodiert (DSPRE schreibt etwas, das der Codec sperrt) → Blob mit `locked` und Hinweis.
- **Ohne `sync.json`** (gelöscht, frischer Klon mit vorhandenem `unpacked/`): Jeder Member wird decodiert und mit der kanonischen JSON verglichen. Das kostet einen vollen Export (gemessen 5 bis 6 s Rechenzeit) und läuft einmal mit Fortschritt.
- Vorteil: robust gegen mtime-Rauschen, erkennt Konflikte, nutzt vorhandene Bausteine (Store-Stale, rotom-Status, xxh3 ist schon Abhängigkeit, `Cargo.toml`).
- Nachteil: eine weitere Zustandsdatei; Mehrfach-Member-Entitäten (Trainer, Spezies, Header in `arm9.bin` bzw. `dynamicHeaders`) brauchen Fingerprints je Teil.

**D3 Exklusiver Wechsel:** Ein Befehl „In DSPRE bearbeiten“ baut `unpacked/` und sperrt das Projekt, „Zurück ins Studio“ liest alles ein.
- Vorteil: einfach, keine Konflikte.
- Nachteil: widerspricht „DSPRE parallel nutzbar“ (Spec §3) und ist umständlich.

**Texte und Scripts** brauchen in B keinen eigenen Abgleich, weil ihre Quelle schon die DSPRE-Datei in `expanded/` ist. Es bleibt die heutige Regel (JSON gewinnt gegen den Member, wenn nicht älter; rotom-Status), die in D2 als Sonderfall „Quelle = DSPRE-Datei“ aufgeht.

---

## 4. Empfehlung

1. **Format:** Variante B.
   - Projektwurzel = DSPRE-Ordner = Git-Wurzel; Manifest `g4-project.json`.
   - Entitäten ohne eigenes Quellformat als eine kanonische JSON je Entität unter `data/<art>/NNNN.json` (pretty, sortierte Schlüssel in fester Reihenfolge, Zeilenende `\n`, Gitter als Textzeilen wie im Prototyp).
   - Texte bleiben chatot-JSON in `expanded/textArchives/`, Scripts rotom-Quellen in `expanded/scripts/`. Die rotom-Quirks kommen in eine Projektdatei.
   - Binäres nur bei Abweichung von der Basis als Blob neben der JSON, sonst ein Verweis per SHA-1.
   - Nicht abgebildete, von DSPRE geänderte Dateien als Blob unter `data/raw/`.
   - Spezies und Trainer als je eine Entität über mehrere Member.
   - Code im neuen Kernel-Modul `textproject` mit eigenen serde-Typen und dem generischen `FIELDS`-Abbilder für alle `fixed_record`-Arten.
   - Umfang (gemessen an Vanilla): rund 6.300 bis 6.500 lesbare Dateien, 16–17 MiB (zlib rund 3 MiB), 0 Blobs.
2. **ID-Quelle:**
   - Spezies, Formen, Attacken, Items, Fähigkeiten und Typen aus festen englischen Tabellen im Repo, abgeleitet aus PokeAPI (BSD-3-Clause, Namensnennung). pret wird nicht kopiert (keine Lizenz).
   - Header als `MAP_<interner Name>` (gemessen eindeutig und sprachfrei).
   - Trainer, Trainerklassen, Orte und spätere 3d-Entitäten werden beim ersten Export in der Projektsprache getauft und eingefroren. Trainer nach Klasse und Name, Kollisionen mit `_2`, `_3` in Nummernreihenfolge, ohne Namen `PREFIX_NNNN`.
   - Register `data/ids/<art>.json`. Umbenennen ändert den Anzeigenamen; die ID ändert nur ein expliziter Befehl, der alle Verweise nachzieht.
   - Überall gilt die ID **oder** die Nummer.
3. **Schema:**
   - S2: Eine `schemaVersion` im Manifest, Migrationskette als reine Funktionen mit Schnappschuss vorher, neuere Versionen werden abgelehnt.
   - `deny_unknown_fields` für Entitäten. `g4-checks.json` bekommt `version: 1`, die Basis-SHA-1 wandert ins Manifest.
   - Fixture-Projekte je Migration und ein Snapshot der Feldnamen.
4. **Round-Trip-Test:** `textproject_round_trip` (Export → Member löschen → Import → Byte-Vergleich → Build → SHA-1 = Basis) und `textproject_export_idempotent` auf allen drei Projekten im Gate, `textproject_from_base` (Extract + Textprojekt → SHA-1). Dazu Proptests je Schema-Typ und je ein synthetischer Test für jeden Sperrfall.
5. **DSPRE-Erkennung:** D2.
   - Fingerprints `(mtime, size, xxh3)` je Member und je Quelle in `.g4/sync.json`, gemeinsam mit dem Build-Graph aus 4.2.
   - Beim Öffnen und bei Fokus erst stat (30 ms), dann Hash nur des Geänderten.
   - Einseitige Änderungen werden automatisch übernommen und gemeldet. Bei beidseitigen Änderungen eine Konfliktliste mit Diff und Wahl je Entität, nie stilles Überschreiben.
   - Ohne Zustand ein einmaliger voller Abgleich mit Fortschritt.

---

## 5. Folgen für den Schnitt

4.3 ist als eine Teilphase L zu groß: Schema für rund 12 Entitätsarten, Register und IDs, Migrationen, Export und Import, Gate-Tests, dazu Store und App auf die neue Quelle umstellen, DSPRE-Abgleich mit Konflikt-UI und symbolische IDs in vier Verbrauchern. **Empfehlung: in 4.3a und 4.3b teilen.**

| Teil | Inhalt | Größe (geschätzt) | Gate |
|---|---|---|---|
| **4.3a Textprojekt im Kernel** (kopflos) | Modul `textproject`: Schema-Typen je Entität, `FIELDS`-Abbilder, Manifest mit `schemaVersion`, Migrationsgerüst (mit einer Probe-Migration v0→v1 an einem Fixture), Symboltabellen (PokeAPI-Ableitung als Skript, YAML in `metadata/symbols/`), Register und Taufe, Blobs nur bei Abweichung (braucht die Basis-ROM aus 4.1), Quirks-Datei für rotom; CLI `g4k text export`, `g4k text import`, `g4k text status`; Build-Graph-Stufe „Quelle → unpacked“ (setzt 4.2 voraus) | L (Kernel ~2.500–3.500 Zeilen mit Tests) | Auf Kopien von Pt DE, HG DE und HG US: Export → Member löschen → Import → Byte-Vergleich aller Dateien; `g4k build` → ROM-SHA-1 gleich der Basis; zweiter Export ohne Änderung (`git status` leer, Projekt in Git); `textproject_from_base` grün; Diff HG DE gegen HG US betrifft nur Texte und die 4 Karten; Zeiten im Gate-Protokoll (Richtwert: Export unter 20 s, Import unter 30 s je Projekt) |
| **4.3b Textprojekt als Quelle** (App und Abgleich) | Store liest und schreibt JSON statt Member (Location-Art `Source`), `unpacked/` als Zwischenstand; `.g4/sync.json` und D2 mit Konfliktliste in Technik; symbolische IDs in `g4-world.json`, `g4-checks.json`, Vorlagen-Rezepten und MCP-Argumenten (ID oder Nummer); „ID umbenennen“ als Transaktion; Erstumstellung eines DSPRE-Ordners mit Fortschritt; Design-System „Änderungen von außen“, „Konflikt“ | L | Szenario über die Test-Umgebung: Spezies im Studio ändern → nur `data/species/NNNN.json` im Diff; Member „von DSPRE“ ändern (Agent schreibt die Datei) → beim Fokus erkannt und übernommen; beide Seiten ändern → Konfliktliste, beide Wahlen funktionieren; MCP-Aufruf mit `SPECIES_PIKACHU` gleich dem mit `25`; ROM spielbar (melonDS), `vanilla_checks` 0 Fehler; `scripts/gate.sh` grün |

Reihenfolge: 4.1 (Basis-ROM) und 4.2 (Build-Graph mit Fingerprint-Format) vor 4.3a, denn Blobs „nur bei Abweichung“ brauchen die Basis und D2 teilt das Format mit 4.2. 4.3b vor 4.4 (Git-Diff je Entität setzt die JSON-Dateien voraus). 3d (Flags, Geschichten) danach als neue Entitäten mit Taufe.

---

## 6. Offene Fragen

1. **Vollständig oder nur Abweichungen?** Sollen unveränderte Entitäten als JSON im Projekt liegen (Empfehlung: ja, für Lesbarkeit und Agenten) oder nur Abweichungen von der Basis (kleiner, sauberer bei öffentlichen Repos, weil vollständige Texte ROM-Inhalt sind)? Mittelweg: vollständig, aber `.gitignore`-Vorlage mit Option „nur Abweichungen veröffentlichen“.
2. **PokeAPI-Tabellen im Repo:** Sind englische Bezeichner aus einer BSD-lizenzierten Fremdquelle mit der Regel „keine ROM-Daten“ vereinbar? Nach meiner Lesart ja (keine Bytes und keine Texte aus dem ROM), bitte bestätigen. Alternativ IDs immer per Taufe (I4) für alles, dann sprachabhängig.
3. **Header-IDs:** `MAP_T01` (intern, sprachfrei, gemessen eindeutig) oder lesbare englische Namen nach pret-Art, von Hand gepflegt und nur für Vanilla?
4. **Trainer-IDs:** Klasse und Name mit Suffix (Pt 432 Suffixe) oder schlicht `TRAINER_0123` mit Anzeigename? Die Suffix-Variante liest sich besser, ist aber bei Rematches wenig aussagekräftig.
5. **Formate von Texten und Scripts:** Bleiben chatot-JSON und rotom-Quelle das Quellformat (Empfehlung)? Oder sollen Texte in die Entitäten wandern (z. B. Speziesname in `data/species/0025.json`)? Das zweite liest sich besser, bricht aber die DSPRE-Kompatibilität der Textarchive.
6. **rotom-Quirks:** als Projektdatei des Studios oder als Vorschlag an rotom (Pragma in der Quelle)?
7. **Konfliktverhalten D2:** bei einseitiger DSPRE-Änderung automatisch übernehmen (Empfehlung) oder immer fragen?
8. **Item-Member, die sich zwei Item-IDs teilen:** prüfen (nicht gemessen), bevor das Item-Schema festliegt.
9. **PokeAPI-Ortsindizes** für `PLACE_*` sprachfrei nutzen? Ob Pt und HG getrennt sind, ist ungeprüft.
10. **Schnitt:** 4.3a/4.3b wie oben, oder 4.3a noch einmal teilen (Daten zuerst, Karten und Header danach)?
