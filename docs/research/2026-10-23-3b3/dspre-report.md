# Recherche für Phase 3b.3 „Daten-Editoren“: DSPRE, pret, Pokémon Studio

Stand: 23.10.2026.

## Quellen

**DSPRE**
- `main` liegt lokal unter `$HOME/Projekte/lab/editor/DSPRE`: Commit e751187, Release 2.3.4, Ordner `DS_Map/`.
- `feature/avalonia` ist Commit 0a550272 vom 21.10.2026, gelesen per `git show origin/feature/avalonia:…`.

**pret-Decomps**
- Sparse-Clones von `include/` unter `…/scratchpad/research-3b3/pret/{pokeplatinum,pokeheartgold}`.

**uxie 0.9.7** (MIT)
- Ordner `~/.cargo/registry/src/index.crates.io-*/uxie-0.9.7/src`.

**Testprojekte (nur gelesen)**
- Platin DE (`CPUD`), HeartGold US (`IPKE`) und HeartGold DE (`IPKD`).
- Ordner `$HOME/Projekte/lab/editor/Roms/*_DSPRE_contents/unpacked/*`.
- Byte-Layouts und Zählwerte habe ich mit einem Skript an echten Membern geprüft (`…/research-3b3/chk.py`, `chk2.py`).

**Zwei Sonnet-Subagenten**
- DSPRE-Issues per `gh` (alle drei Tracker).
- Pokémon Studio, Porymap, HMA: Shallow-Clone von PokemonStudio `develop` und weiteren Repos unter `…/research-3b3/{repo,porymap,hma-wiki,src}`.

**Kennzeichnung**
- **[geprüft]**: an Projektdaten verifiziert.
- **[Code]**: aus Quelltext gelesen, nicht an Daten geprüft.
- **[unverifiziert]**: aus Issues oder Sekundärquellen.

Kein AGPL-Code übernommen. Beschrieben sind nur Verhalten und Byte-Layouts.

Im Repo gibt es noch kein Formatdokument für diese Strukturen.
- `docs/formats/` deckt Header, Text, NARC usw. ab.
- Die Research-Datei `docs/research/2026-10-20-dspre-und-suite-analyse.md` nennt für die Datenbank-Editoren nur das Ziel: „Liste plus Detail, Suche, Filter, Bulk-Edit, Referenzzähler … Querverlinkung“ (Abschn. 6).
- `crates/g4-kernel/metadata/archives.yaml` kennt noch keine der unten genannten NARCs; sie müssen ergänzt werden.

---

## 0. Archive und Textarchive (Übersicht)

### 0.1 NARCs (gepackter Pfad relativ zu `files/`)

Quelle: DSPRE `RomInfo.cs` Z. 2310–2515. Pt- und HGSS-Pfade sind im Projekt vorhanden [geprüft].

| Daten | DP | Platin | HGSS | Unpacked-Ordner (DSPRE) |
|---|---|---|---|---|
| Wild-Encounter | `fielddata/encountdata/d_enc_data.narc` bzw. `p_enc_data.narc` | `fielddata/encountdata/pl_enc_data.narc` | HG `a/0/3/7`, SS `a/1/3/6` | `encounters` |
| Personal | `poketool/personal/personal.narc` (Perl: `poketool/personal_pearl/personal.narc`) | `poketool/personal/pl_personal.narc` | `a/0/0/2` | `personalPokeData` |
| Learnsets | `poketool/personal/wotbl.narc` | dto. | `a/0/3/3` | `learnsets` |
| Evolutionen | `poketool/personal/evo.narc` | dto. | `a/0/3/4` | `evolutions` |
| Egg Moves | Overlay 5 (Tabelle), DSPRE-Variante `zcustom/egg_moves.narc` | Overlay 5 | `a/2/2/9` (1 Member, 4092 B) | `eggMoves` |
| Moves | `poketool/waza/waza_tbl.narc` | `poketool/waza/pl_waza_tbl.narc` | `a/0/1/1` | `moveData` |
| Items | `itemtool/itemdata/item_data.narc` | `itemtool/itemdata/pl_item_data.narc` | `a/0/1/7` | `itemData` |
| Item-Icons | `itemtool/itemdata/item_icon.narc` | dto. | `a/0/1/8` | `itemIcons` |
| Trainer-Daten | `poketool/trainer/trdata.narc` | dto. | `a/0/5/5` | `trainerProperties` |
| Trainer-Party | `poketool/trainer/trpoke.narc` | dto. | `a/0/5/6` | `trainerParty` |
| Trainer-Grafik | `poketool/trgra/trfgra.narc` | dto. | `a/0/5/8` | `trainerGraphics` |
| Trainer-Texttabelle | `poketool/trmsg/trtbl.narc` und `trtblofs.narc` | dto. | `a/0/5/7` und `a/1/3/1` | `trainerTextTable`/`trainerTextOffset` |
| Spezial-Encounter Pt (Trophy Garden, Honigbaum, Großmoor) | `arc/encdata_ex.narc` | dto. | – | `encounterExtended` |
| Headbutt / Safari / Rock-Smash-Items (HGSS) | – | – | `a/2/5/2` / `a/2/3/0` / `a/2/5/3` | `headbutt`/`safariZone`/`rockSmash` |

**Falle Platin [geprüft]:** Das Platin-ROM enthält noch die DP-Altdateien `d_enc_data.narc`, `p_enc_data.narc`, `personal.narc`, `item_data.narc` und `waza_tbl.narc`. Verbindlich sind nur die `pl_`-Varianten, ebenso `pl_growtbl.narc`.

### 0.2 Member-Zahlen und -Größen

| | Platin DE | HeartGold US |
|---|---|---|
| encounters | 183 × 424 B | 142 × 196 B |
| personal | 508 × 44 B | 508 × 44 B |
| evolutions | 508 × 44 B | 508 × 44 B |
| learnsets | 508, variabel (Vielfache von 4) | 508, variabel |
| moveData | 471 × 16 B | 471 × 16 B |
| itemData | 446 × 34 B | 514 × 34 B |
| trainerProperties | 928 × 20 B | 738 × 20 B |
| trainerParty | 928, variabel (auf 4 B aufgefüllt) | 738, variabel (auf 4 B aufgefüllt) |

[geprüft]

### 0.3 Textarchive (Namen)

Message-Zahl in Klammern. Die Werte stammen aus DSPRE `RomInfo.cs` Z. 1369–1832. Für Pt DE, HG US und HG DE habe ich sie über die Message-Anzahl geprüft [geprüft]; DP ist [Code].

| Inhalt | DP | Platin | HGSS (JP abweichend) |
|---|---|---|---|
| Species-Namen | 362 (363) | 412 (496), auch 413 | 237 (496), auch 238; JP 232 |
| Formnamen o. ä. | – | 712–716 (712: 494) | 817–821 (817: 494) |
| Move-Namen / -Beschreibungen | 588 / 587 | 647 (468) / 646 (468) | 750 (468) / 749 (468); JP Namen 739 |
| Item-Namen / -Beschreibungen | 344 (JP 341) / ? | 392 (468) / **[unverifiziert]: 391 oder 393** (beide 468 Einträge; DSPRE setzt 0) | 222 (537) / 221 (537); JP 219 |
| Fähigkeiten | 552 | 610 (124) | 720 (124) |
| Typen | 565 | 624 (18) | 735 (18) |
| Trainernamen | 559 (JP 550) | 618 (928 = Trainerzahl) | 729 (738 = Trainerzahl); JP 719 |
| Trainerklassen | 560 (JP 551) | 619 (105), EN-Beschreibung 620 | 730 (129); JP 720 |
| Ortsnamen | 382 | 433 (126) | 279 (235); JP 272 |
| Trainer-Kampftexte | 558 (JP 549) | 617 (2497) | [Code] |

**Konsequenz:** Textarchiv-IDs gehören als Tabelle je (Familie, Sprache) in die Metadaten (`metadata/*.yaml`), nicht in den Code. Laut Issues D#216, A#78 und A#79 las DSPRE beim JP-HGSS falsche Archive [unverifiziert].

**Zählfallen [geprüft]:** Die Zahl der Namen ist nie die Zahl der Datensätze.
- 496 Species-Namen stehen 508 Personal-Membern gegenüber.
- 468 Move-Namen stehen 471 Move-Membern gegenüber.
- 468 Item-Namen (Pt) stehen 446 Item-Data-Membern gegenüber, bei HGSS 537 zu 514.

---

## 1. Editoren

### 1.1 Wild-Encounter DP/Pt

#### Layout

Ein Member ist 0x1A8 = 424 B, alle Werte LE. Quellen: pret `pokeplatinum/include/overlay006/wild_encounters.h` (`WildEncounters`) und DSPRE `ROMFiles/EncounterFile.cs` (`EncounterFileDPPt`). Die Größe ist geprüft an Pt DE (183/183).

| Offset | Typ | Feld | Bemerkung |
|---|---|---|---|
| 0x000 | u32 | Walking-Rate | 0 = auf diesem Gras nie Encounter. DSPRE castet auf u8 und schreibt u32. |
| 0x004 | 12 × { u32 level, u32 species } | Walking-Slots | pret: `s8 level; u8 pad[3]; int species`. Ein Level je Slot, kein Min/Max. Padding in Pt DE immer 0 [geprüft]. |
| 0x064 | u32[2] | Swarm | ersetzt Walking-Slots 0, 1 |
| 0x06C | u32[2] | Tag | ersetzt Walking-Slots 2, 3 tagsüber; morgens gilt die Grundliste |
| 0x074 | u32[2] | Nacht | ersetzt Walking-Slots 2, 3 nachts |
| 0x07C | u32[4] | Poké-Radar | ersetzt Walking-Slots 4, 5, 10, 11 |
| 0x08C | u32[5] | Formraten | [0] Shellos, [1] Gastrodon: 0 = West, ≠0 = Ost (Vanilla schreibt 100). [2..4] ungenutzt. Avalonia: „only tests for zero“, behält den Rohwert bei. |
| 0x0A0 | u32 | Unown-Tabelle | 0..8: Most Forms (0), Most Forms, Only F, R, I, N, E, D, „! and ?“. Avalonia `UnownTableNames`. |
| 0x0A4 | u32[2] × 5 | Dual-Slot Ruby, Sapphire, Emerald, FireRed, LeafGreen | je 2 Einträge, bei 0x0A4/0x0AC/0x0B4/0x0BC/0x0C4. Ersetzen Walking-Slots 8, 9 (nur mit nationalem Dex). |
| 0x0CC | u32 + 5 × { u8 **max**, u8 **min**, u16 pad, u32 species } | Surf | **max vor min** [geprüft: 1078× byte0>byte1] |
| 0x0F8 | 44 B | ungenutzt (pret `WaterEncounters unused`) | Rock Smash, in DPPt auskommentiert. In Pt DE überall 0 [geprüft]. DSPRE schreibt hier Nullen, erhält den Block also nicht. |
| 0x124 | u32 + 5 Slots | Old Rod | Slots wie Surf |
| 0x150 | u32 + 5 Slots | Good Rod | Slots wie Surf |
| 0x17C | u32 + 5 Slots | Super Rod | Slots wie Surf; Ende bei 0x1A8 |

Species sind in DPPt u32 (Vanilla: obere 16 Bit = 0 [geprüft]). Unter hg-engine-artigen Hacks trägt das Species-Wort auch die Form (A#76; Avalonia `ShowForm`, `FormShift`).

**DP vs. Pt:** DSPRE nutzt für beide eine Klasse; das Layout ist laut DSPRE identisch [Code]. Ein DP-Projekt lag nicht vor [unverifiziert]. Trophy Garden (Slots 6, 7), Honigbäume und Großmoor liegen in Pt in `arc/encdata_ex.narc`; in DPPt gibt es separate Tabellen (D#195, #159, #165, #234).

#### Was DSPRE zeigt

- **`main` (WinForms):**
  - Rate je Methode (0–255).
  - Walking-Tabelle: Level und Species.
  - Swarm, Tag, Nacht, Radar, die fünf Dual-Slot-Gruppen.
  - Shellos/Gastrodon West/Ost, Unown-Tabelle.
  - Surf und drei Ruten (min/max/species).
  - Encounter-Datei-Liste beschriftet mit „Ortsnamen aller Header, die diese Datei nutzen“.
  - Nur die letzte Datei lässt sich hinzufügen oder entfernen.
  - Export/Import als `.wld`, Funktion „Repair all“.
- **Avalonia zusätzlich** (`WildEditorDPPtViewModel.cs`):
  - Spalte „Replaced by“ mit Icons der ersetzenden Species je Walking-Slot.
  - Hinweise „Swarm / Poké Radar / Trophy Garden / Dual Slot“.
  - Warnungen für leere Ersatz-Slots: „gives a glitch Pokémon“.
  - Undo, Dirty-Titel mit „●“.

#### Validierung

- **`main`:** NumericUpDown 0–255. Beim Lesen werden fehlende Felder mit 0 „repariert“ und gemeldet (`ReportErrors`). Sonst keine Prüfung.
- **Avalonia** (`WildTimeRow.cs` → `WildEncounterChecks`): Prüft nur bei Rate > 0.
  - Slot ohne Species.
  - Level außerhalb 1..100.
  - min > max mit dem Hinweis „the game swaps them“.
  - Ergebnis ist der Dialog „Save anyway?“. **Es sind Warnungen, keine Sperren.**
- **Löschen einer Encounter-Datei:**
  - Avalonia **verweigert** es, solange ein Header darauf zeigt: „Encounter file N is used by header(s) … Point it elsewhere first.“ (`WildEditorHGSSViewModel.cs` Z. 628; DPPt analog [Code]).
  - `main` prüft das nicht.

#### Geschriebene Dateien

- Nur das Member im `unpacked/encounters`-Ordner. Das NARC wird erst beim ROM-Build gepackt.
- Separat und optional: der Slot-Odds-Editor.
  - Er patcht `cmp r0,#boundary` in Overlay 6 (Pt) bzw. Overlay 2 (HGSS), siehe `DSPRE.Core/ROMFiles/EncounterSlotOdds.cs` und D#269.
  - Er verweigert, wenn die Instruktion nicht `cmp` ist („may have been patched“) oder eine Grenze an zwei Stellen verschiedene Werte hat.

#### Slot-Wahrscheinlichkeiten

Vanilla-Werte [unverifiziert, D#269]:
- Gras: 20, 20, 10, 10, 10, 10, 5, 5, 4, 4, 1, 1.
- Surf: 60, 30, 5, 4, 1.
- Old Rod: 60, 30, 5, 4, 1.
- Good und Super Rod: 40, 40, 15, 4, 1.

Sie liegen im Code, nicht in der Datei; im Editor also nur anzeigen.

### 1.2 Wild-Encounter HGSS

#### Layout

Ein Member ist 0xC4 = 196 B. Quelle: pret `pokeheartgold/include/wild_encounter.h` (`EncounterData`, Offsets dort kommentiert). Geprüft an HG US (142/142) und HG DE.

| Offset | Typ | Feld |
|---|---|---|
| 0x00–0x05 | u8 × 6 | Raten: Walking, Surf, Rock Smash, Old, Good, Super |
| 0x06 | u8[2] | Padding (immer 0 [geprüft]) |
| 0x08 | u8[12] | Walking-Level, **gemeinsam für Morgen, Tag und Nacht** |
| 0x14 / 0x2C / 0x44 | u16[12] je | Species Morgen / Tag / Nacht |
| 0x5C / 0x60 | u16[2] je | Radio: Hoenn-Sound / Sinnoh-Sound |
| 0x64 | 5 × { u8 **min**, u8 **max**, u16 species } | Surf (**min vor max**, umgekehrt zu DPPt [geprüft: 244× byte0<byte1, nie >]) |
| 0x78 | 2 × Slot | Rock Smash |
| 0x80 / 0x94 / 0xA8 | 5 × Slot je | Old / Good / Super Rod |
| 0xBC | u16 | landSwarm |
| 0xBE | u16 | surfSwarm |
| 0xC0 | u16 | **nightFish**: kein Swarm, sondern der Nachtangel-Slot |
| 0xC2 | u16 | fishSwarm |

#### Ersetzungsregeln

Quelle: Avalonia `WildEditorHGSSViewModel.SetNotes()` Z. 516 ff. [Code].

| Quelle | ersetzt |
|---|---|
| landSwarm | Walking-Slots 0, 1 zu jeder Tageszeit |
| Radio Hoenn/Sinnoh [0] | Walking-Slots 2, 3 |
| Radio Hoenn/Sinnoh [1] | Walking-Slots 4, 5 |
| surfSwarm | Surf-Slot 0 |
| fishSwarm | Old Rod Slot 2; Good Rod Slots 0, 2, 3; alle Super-Rod-Slots |
| nightFish | Good Rod Slot 3 und Super Rod Slot 1 nachts, außer ein Fishing-Swarm ist aktiv (Swarm hat Vorrang, D#16) |

#### Slot-Prozente HGSS

[unverifiziert, D#269]
- Land wie DPPt.
- Surf 60, 30, 5, 4, 1.
- Rock Smash 80, 20.
- Alle Ruten 40, 30, 15, 10, 5.

#### Weitere HGSS-Sondertabellen (eigene Dateien, nicht im Encounter-Member)

- Headbutt `a/2/5/2`: je Header-ID. Ein neuer Header braucht eine neue Headbutt-Datei (D#297).
- Safari `a/2/3/0`.
- Bug Contest `data/mushi/mushi_encount.bin` und `mushi_trainer.bin` (D#146, D#264).
  - Die Raten dort sind kumulative Schwellen; doppelte oder fallende Werte machen Slots unerreichbar.
- Swarm-Orte: eine eigene Tabelle von Header-IDs; sie bestimmt, wo ein Swarm überhaupt aktiv werden kann (D#270) [unverifiziert].
- DPPt: Wild-Held-Item-Schwellen in ARM9 (D#266) [unverifiziert].

#### UI, Validierung und Schreiben

- Felder wie in 1.1, aber:
  - Walking-Level einmal; drei Species-Spalten Morgen/Tag/Nacht.
  - Avalonia koppelt die Level der drei Tageszeiten-Tabellen (`LinkWalkingLevels`).
  - Radio als ein Raster mit Spalten Hoenn und Sinnoh.
  - Swarm-Raster mit den Labels „Swarm while walking“, „Swarm while surfing“, „Night fishing“, „Swarm while fishing“.
- Validierung und Schreiben wie in 1.1.
- Leere Ersatz-Slots erzeugen konkrete Warnungen, z. B.:
  - „if a swarm comes here, surf slot 1 gives a glitch Pokémon“
  - „at night, Good Rod slot 4 and Super Rod slot 2 give a glitch Pokémon“

#### Header → Encounter

- Header-Feld `wild_pokemon`: Quelle uxie `map_header/binary.rs`, DSPRE `SetNullEncounterID`.
- **DP/Pt:** u16 bei Offset 0x0E, „keine Encounter“ = 0xFFFF.
- **HGSS:** u8 bei Offset 0x00, „keine“ = 0xFF.
  - Damit sind höchstens 255 Encounter-Dateien adressierbar (0..254).
  - Neue Dateien über 254 sind für HGSS nicht referenzierbar.
- Mehrere Header können dieselbe Datei teilen; die Swarm-Berechtigung hängt aber am Header (D#270).

### 1.3 Trainer und Party

#### Trainer-Properties

Ein Member ist 20 B (alle Familien). Quellen: pret `pokeplatinum/include/struct_defs/trainer_data.h` (`TrainerHeader`), `pokeheartgold/include/trainer_data.h` (`TrainerData`), DSPRE `ROMFiles/TrainerFile.cs`.

| Off | Typ | Feld | Werte |
|---|---|---|---|
| 0x00 | u8 | monDataType | Bit0 = eigene Moves, Bit1 = Held Item; 0..3. Pt: 0: 672, 1: 198, 3: 54, 2: 4 [geprüft]. |
| 0x01 | u8 | Trainerklasse | → Klassen-Textarchiv, Klassen-Grafik, Preisgeld- und Geschlechtertabellen. Pt 105 Klassen, HGSS 129. |
| 0x02 | u8 | unbekannt / „sprite“ | immer 0 [geprüft]; DSPRE erhält den Wert (`trDataUnknown`) |
| 0x03 | u8 | Party-Größe | 0..6; Trainer 0 hat 0 |
| 0x04 | u16[4] | Trainer-Items | → Item-IDs |
| 0x0C | u32 | AI-Flags | Vanilla-Werte 1, 7, 5, 33, 35, 15, 17, 3, 519 (HGSS Bit 9) [geprüft] |
| 0x10 | u32 | battleType | 0 oder 2 (= `BATTLE_TYPE_DOUBLES`, 1<<1) [geprüft] |

#### Party-Member

Ein Member enthält n Einträge (n = Party-Größe) und ist auf ein Vielfaches von 4 B aufgefüllt [geprüft].
- Pt: 43 Dateien, HGSS: 35 Dateien sind 2 B länger als n × Eintragsgröße.
- Trainer 0 hat 0 Mons und eine 8-B-Datei.

| Familie | Eintrag ohne Item/Moves | + Item | + Moves | + beides |
|---|---|---|---|---|
| DP | 6 B | 8 B | 14 B | 16 B |
| Pt, HGSS | 8 B | 10 B | 16 B | 18 B |

Layout eines Eintrags, in dieser Reihenfolge:
- Difficulty/IV-Skala
- `u16 level`
- `u16 species`: Bits 0–9 Species, Bits 10–15 Form
- `[u16 item]`
- `[u16 moves[4]]`
- `[u16 ballCapsule]`: nur Pt/HGSS, DP hat keine Kapsel

Difficulty und Gender/Ability je Familie:
- **pret Pt:** `u16 ivScale`.
- **pret HGSS:** `u8 difficulty`, dann `u8 genderAbilityOverride`.
  - Bits 0–3: 1 = männlich, 2 = weiblich erzwingen.
  - Bits 4–7: 1 oder 2 = Fähigkeit 1 oder 2 erzwingen.
  - In HG US hat Byte 1 bei 368 von 1776 Mons den Wert 0x20 [geprüft].
- **DSPRE:** liest für alle Familien `u8 difficulty, u8 genderAndAbilityFlags`.
  - Flags: 0x01 männlich, 0x02 weiblich, 0x10 Fähigkeit 1, 0x20 Fähigkeit 2.
  - Die Kommentare sagen „only used for HGSS, filler byte for the rest“.
- **uxie:** liest DP/Pt als u16.
- **Falle [unverifiziert]:** Gender/Ability-Flags in einem Vanilla-Pt zu setzen, ändert das High-Byte von `ivScale`. In Pt DE ist Byte 1 fast immer 0, einmal 9 [geprüft]. Gender/Ability nur für HGSS (oder Pt mit Backport-Patch, siehe D PR#151) anbieten.

IV = difficulty × 31 / 255 (`MAX_IV_SCALE 255`). Die Party-Form-Bits sind in Pt-Vanilla selten (8× Form 1, 2× Form 2 [geprüft]).

#### Was DSPRE zeigt

- **`main`:**
  - Trainerliste „[ID] Klasse Name“, Suche (`TrainerSearch.cs`).
  - Name, wobei die Länge gegen `RomInfo.trainerNameMaxLen` geprüft wird.
  - Klasse (Liste mit Grafik-Vorschau und Animationsframes).
  - Party-Größe, Doppelkampf, Items 1–4.
  - 11 AI-Checkboxen: Basic, Evaluate Attack, Expert, Setup, Risky, Prioritize Extremes, Baton Pass, Tag Strategy, Check HP, Weather, Harassment.
  - Checkboxen „Choose Moves“ und „Choose Items“.
  - Je Mon: Species, Form, Level, IV/DV, Ability (Default/1/2), Gender (HGSS), Item, 4 Moves, Ball-Kapsel.
  - Reorder, DV-Rechner, Showdown-Export.
  - Klassen-Einstellungen: Eye-Music in einer ARM9-Tabelle, Preisgeld-Multiplikator (Overlay/ARM9), Geschlecht.
- **Avalonia zusätzlich** (`TrainerEditorViewModel.cs`):
  - „Where the trainer is used“: Liste mit Art, Ort und Header-Orten, Doppelklick öffnet Script oder Event (siehe 2.2).
  - Preisgeld-Anzeige: Level des letzten Mons × Klassen-Multiplikator.
  - Kampftexte inline.
  - Undo; Add/Remove Trainer (D#215).
  - Trainerklassen hinzufügen (Patch nach `/a/1/5/5`, D PR#272).
  - Shiny-Flag je Mon (ASM-Patch, PR#271), Custom Balls (PR#307).

#### Validierung (`main`, `TrainerEditor.cs` Z. 1150–1205)

| Fall | Verhalten |
|---|---|
| Doppelte Moves | Warnung „This will cause issues in game“; gespeichert wird trotzdem |
| Name > `trainerNameMaxLen` | Gespeichert, danach Warnung. Max. 7 in Vanilla laut A#25/A#29. Das ARM9-Byte bei `trainerNameLenOffset` (z. B. DP EN 0x6AC32) kann der Patch Toolbox erweitern. |
| Trainerindex < 0 | Speichern abgelehnt („trainer zero bug“, PR#202) |
| > 256 Trainerklassen | Abbruch beim Laden |

`main` prüft Level, Species und Form nicht.

#### Geschriebene Dateien

- Member `trainerProperties/NNNN` und `trainerParty/NNNN`.
- Trainername im Textarchiv 618/729 (expanded JSON).
- Klassenname bei Bearbeitung.
- Bei Klassen-Einstellungen: ARM9 (Eye-Music-Tabelle, Geschlechtertabelle) und die Preisgeld-Tabelle (Pt/DP 1 B je Klasse; HGSS u16-Paare {Klasse, Multiplikator}; siehe `RomInfo.cs` Z. 71–81).

#### Verlustfallen in DSPRE [Code]

- AI wird nur mit 11 Bit zurückgeschrieben (`AI_COUNT = 11`); höhere Bits gehen verloren.
- battleType wird auf 0 oder 2 normalisiert, d. h. `TAG` (1<<4) usw. gehen verloren.
- Moves 0xFFFF werden beim Lesen zu 0.
- Das 4-B-Padding der Party-Datei wird nicht geschrieben.
- Für ein byte-identisches Round-Trip: alle Bits erhalten und das Padding nachbilden.

### 1.4 Pokémon-Basisdaten (Personal) samt Formen, Evolutionen, Learnsets, Egg Moves

#### Personal

Ein Member ist 44 B. Quellen: pret `pokeplatinum/include/struct_defs/species.h` (`SpeciesData`), DSPRE `ROMFiles/PokemonPersonalData.cs`, uxie `personal_data`.

| Off | Typ | Feld | Bereich |
|---|---|---|---|
| 0x00–0x05 | u8 × 6 | KP, Ang, Vert, Init, SpAng, SpVert | 0–255 |
| 0x06, 0x07 | u8 | Typ 1, Typ 2 | 0–17 (9 = ???-Typ); → Typen-Textarchiv |
| 0x08 | u8 | Fangrate | |
| 0x09 | u8 | Basis-EP | |
| 0x0A | u16 | EV-Yield | je 2 Bit: KP, Ang, Vert, Init, SpAng, SpVert (Bits 0–11). Bits 12–15 sind in Vanilla 0 [geprüft]; DSPRE schreibt sie als 0. |
| 0x0C, 0x0E | u16 | Wild-Item 1, 2 | → Item-IDs |
| 0x10 | u8 | Geschlechterverhältnis | 0 = nur männlich, 254 = nur weiblich, 255 = geschlechtslos |
| 0x11 | u8 | Brutzyklen | |
| 0x12 | u8 | Basis-Freundschaft | |
| 0x13 | u8 | Wachstumskurve | 0 MediumFast, 1 Erratic, 2 Fluctuating, 3 MediumSlow, 4 Fast, 5 Slow |
| 0x14, 0x15 | u8 | Ei-Gruppe 1, 2 | 0–15 |
| 0x16, 0x17 | u8 | Fähigkeit 1, 2 | → Fähigkeiten-Textarchiv |
| 0x18 | u8 | Safari-Fluchtrate | |
| 0x19 | u8 | Farbe (Bits 0–6), Flip (Bit 7) | Flip ohne Funktion in HGSS (D#287) |
| 0x1A | u16 | Padding | immer 0 [geprüft, Pt und HGSS] |
| 0x1C | u32[4] | TM/HM-Bits | 100 genutzt (TM01–92, HM01–08), höchstes gesetztes Bit 99 [geprüft]; Bits 100–127 = 0 |

Formen und Indizes [geprüft an den Basiswerten]:
- 0..493 Species.
- 494 Ei, 495 Bad Egg (Stats 10).
- 496–498 Deoxys Angriff/Verteidigung/Initiative.
- 499–500 Wormadam Sand/Lumpen.
- 501 Giratina Urform, 502 Shaymin Zenit.
- 503–507 Rotom Hitze/Wasch/Frost/Wirbel/Schneid. In Pt-Vanilla haben alle fünf dieselben Werte.
- Quelle für die Zuordnung: DSPRE `Resources/PokeDatabase.cs` Z. 1880 (`personalExtraFiles`).
- HGSS hat dieselbe Zählung (508). DP hat weniger Member: D#244 meldete einen Absturz bei `personalPokeData\0501` [unverifiziert].

**Falle Default-Form [unverifiziert]:** Laut `CHANGELOG_2.3_User.md` liegt die Default-Form einiger Species nicht im Haupt-Member, sondern im Alternativ-Formen-Archiv. Betroffen sind Deoxys, Unown, Castform, Burmy, Wormadam, Cherrim, Shellos, Gastrodon, Arceus und spielabhängig Shaymin, Rotom, Giratina, Pichu. Wer dann nur das Haupt-Member editiert, ändert Bytes, die das Spiel nie liest.

**Weitere Personal-nahe Daten, nicht im Personal-Member:**
- Pokédex-Höhe und -Gewicht (`zukan_data`, D#287).
- Wachstumstabellen (`pl_growtbl.narc`, HGSS `a/0/0/3`, je 8 × 101 u32; D#256).
- Tutor-Tabellen: HGSS Overlay 1 plus `waza_oshie.bin`, Pt Overlay 5 (D#259).

#### Evolutionen

Ein Member ist 44 B = 7 × { u16 method, u16 param, u16 target } + 2 B Padding (0 [geprüft]).
- Methoden 0..26 kommen in Vanilla vor [geprüft]. Die Namen stehen in DSPRE `ROMFiles/EvolutionFile.cs`.
- Param-Bedeutung je Methode:
  - Level: 4, 8–14, 22, 23.
  - Item: 6, 7, 16–19.
  - Move: 20.
  - Species im Team: 21.
  - Schönheit: 15.
  - Ohne Param: 0–3, 5, 24–26 (24–26 sind Orte: Kraterberg, Ewigwald, Route 217).
- **Erweiterung** über 7 Einträge nur mit ARM9-Patch an 4 Stellen (D#3) [unverifiziert].
- **DSPRE-Writer [Code]:** schreibt nur „gültige“ Einträge und rückt sie nach vorn (`isValid`):
  - Level-Methoden brauchen param 1..100.
  - Alle anderen brauchen target > 0.
  - Ungültige Zeilen verschwinden also beim Speichern still; danach wird mit 0 aufgefüllt.
- Form-Member (496+) haben in Vanilla keine eigenen Evolutionen (Changelog 2.3).

#### Learnsets

- Folge von `u16` = move (Bits 0–8) | level << 9 (Bits 9–15), Terminator 0xFFFF.
- Danach auf 4 B mit 0x0000 aufgefüllt [geprüft]: 265/268 Dateien enden auf `ffff0000`, die übrigen auf `xxxx ffff`.
- pret: `MAX_LEARNSET_ENTRIES 20` plus Terminator (`ALIGN_4`). Mehr als 20 Einträge überlaufen den Puffer, das ist ein harter Grenzwert.
- DSPRE zeigt bei > 20 eine Warnung, die sich pro Projekt per Marker-Datei `unpacked/.learnset_warning_dismissed` abschalten lässt; gespeichert wird trotzdem.
- Move-IDs sind auf ≤ 511 und Level auf ≤ 127 begrenzt.
- **DSPRE [Code]:** liest in eine `UniqueList`, d. h. doppelte (Level, Move)-Paare verschwinden still. Das Speichern sortiert nicht.

#### Egg Moves

- **DPPt:** Tabelle in Overlay 5 bei `RomInfo.GetEggMoveTableOffset()`, Grenze 0xEEC B.
- **HGSS:** `a/2/2/9` (ein Member, 4092 B [geprüft]).
- Format: `species + 20000` als Marker, danach Move-IDs; Ende 0xFFFF.
- DSPRE begrenzt auf 16 Moves je Species (`MAX_EGG_MOVES`) und warnt bei Überschreitung. Quelle: DSPRE `Editors/EggMoveEditor.cs` Z. 36–303.

#### Was DSPRE zeigt

- `PokemonEditor` als Container mit Tabs: Personal, Learnset, Evolution, Sprite, Battle Display. Die Checkbox „sync selection across tabs“ hält alle Tabs auf derselben Species.
- Personal zeigt alle Felder oben, die TM/HM-Checkliste, CSV-Import/-Export und einen TM-Bulk-Editor.
- Die Liste nennt Formen als „Species – Form“.
- Learnset-Bulk-Editor:
  - CSV, Kopieren auf andere Species, globales Move-Ersetzen, Level-Shift.
  - Filter (D#79: Bug mit gefiltertem Index).

#### Geschriebene Dateien

- Nur die Member in `personalPokeData`, `learnsets`, `evolutions`, `eggMoves`.
- Bei DPPt-Egg-Moves direkt Overlay 5.
- Bei CSV-Umbenennung zusätzlich Textarchive.
- DSPRE fügt in `main` keine Species hinzu; die Zahl der Member bestimmt den Maximalwert.

### 1.5 Moves

Ein Member ist 16 B. Quellen: pret `pokeplatinum/include/move_table.h`, DSPRE `MoveData.cs`. Pt und HGSS haben je 471 Member, aber 468 Namen [geprüft].

| Off | Typ | Feld | Bereich / Bemerkung |
|---|---|---|---|
| 0x00 | u16 | Battle-Effect | Index in die Effektskripte |
| 0x02 | u8 | Kategorie | 0 physisch, 1 speziell, 2 Status |
| 0x03 | u8 | Stärke | 0 = „—“ |
| 0x04 | u8 | Typ | 0–17 |
| 0x05 | u8 | Genauigkeit | 0 = trifft immer; u8, also 0–255, Vanilla ≤ 100 (D#273) |
| 0x06 | u8 | AP | |
| 0x07 | u8 | Effekt-Chance | 0–255, Vanilla ≤ 100 |
| 0x08 | u16 | Range / Ziel | Bitfeld: 0, 1<<0 … 1<<10 (DSPRE `AttackRangeDescriptions`) |
| 0x0A | s8 | Priorität | |
| 0x0B | u8 | Flags | contact, protect, magic coat, snatch, mirror move, king's rock, keep HP bar, hide shadow |
| 0x0C | u8 | Contest-Effekt | |
| 0x0D | u8 | Contest-Typ | 0–4: cool, beauty, cute, smart, tough |
| 0x0E | u16 | Padding | immer 0 [geprüft] |

#### Was DSPRE zeigt und schreibt

- **`main`** zeigt die Felder oben, CSV-Import/-Export und Massen-Umbenennung. Es schreibt nur `moveData/NNNN` und bei Umbenennung das Textarchiv 647/750.
- **Avalonia zusätzlich:**
  - Beschreibung und „Bag-Text“ mit Pixelbreiten-Warnung (`DescriptionWarning`).
  - Listen „Punching“ (Eisenfaust) und „Sound“ (Lärmschutz). Das sind Tabellen im Battle-Overlay, nicht im Move-Struct (D#285); sie zu erweitern verlangt einen Patch.
- „Vanilla ranges, widened for a move that already holds a value outside them so loading never changes it“ (`MoveDataEditorViewModel.cs` Z. 153). Dieses Muster lohnt sich zu übernehmen.

Neue Moves gibt es in DSPRE nicht. Mehr als 467 Moves sprengen außerdem die 9-Bit-Grenze der Learnsets nicht; die liegt erst bei 511.

### 1.6 Items

Ein Member ist 34 B. Quellen: pret `pokeplatinum/include/item.h` (`ItemData`, `ItemPartyParam`), DSPRE `ItemData.cs`.

| Off | Typ | Feld |
|---|---|---|
| 0x00 | u16 | Preis (Wert, nicht zwingend Kaufpreis; D#14) |
| 0x02 | u8 | Hold-Effekt |
| 0x03 | u8 | Hold-Parameter |
| 0x04 | u8 | Pluck-Effekt |
| 0x05 | u8 | Fling-Effekt |
| 0x06 | u8 | Fling-Stärke |
| 0x07 | u8 | Natural-Gift-Stärke |
| 0x08 | u16 | Bitfeld: Natural-Gift-Typ (Bits 0–4), preventToss (5), canRegister (6), Feldtasche (7–10), Kampftasche (11–15) |
| 0x0A | u8 | Feld-Funktion |
| 0x0B | u8 | Kampf-Kategorie |
| 0x0C | u8 | partyUse (1 = Party-Parameter gültig) |
| 0x0D | u8 | Padding |
| 0x0E | 20 B | `ItemPartyParam`, siehe unten |

Aufbau von `ItemPartyParam`:
- 7 Byte Bitfelder:
  - Heilt Schlaf, Gift, Verbrennung, Eis, Paralyse, Verwirrung, Anziehung.
  - Guard Spec, Revive, ReviveAll, LevelUp, Evolve.
  - Stat-Stufen je 4 Bit; Crit 2 Bit.
  - PP Up, PP Max, PP Restore, PP RestoreAll, HP Restore.
  - EV-Flags für 6 Stats; Freundschaft Low/Mid/High.
- 11 Werte: 6 × s8 EV, HP-Wert, AP-Wert, 3 × s8 Freundschaft.
- 2 B Padding.

**Indirektion über ARM9 [geprüft]:**
- Item-ID → 8-B-Eintrag in der ARM9-Tabelle `{u16 itemData-Member, u16 Icon, u16 Palette, u16 GBA-Item-ID}`.
- `itemTableOffset` je Sprache, DSPRE `RomInfo.SetItemTableOffset`:
  - Pt: DE 0xF0D1C, EN 0xF0CC4.
  - HGSS: EN 0x100194, DE 0x100148.
  - DP: EN 0xF85B4.
- In Pt DE sind die Einträge 0..467 gültig (468 = Namenszahl). Die unbenutzten IDs 113..134 zeigen auf Data-Member 0.
- Nur 113 der ersten 440 IDs haben data == ID.
- HGSS: 537 IDs auf 514 Data-Member.
- **Item-ID ≠ Member-Index.**

#### Was DSPRE zeigt

- Felder wie oben, Taschen als Dropdowns.
- Party-Parameter: Bälle teilen sich Flagkombinationen mit Heilitems; deshalb grauen sie die allgemeinen Effekte aus (D#14).
- Icon- und Palettenwahl.
- Item-ID-Feld bis `itemNames.Length - 1`, Data-Feld bis Member-Zahl - 1.

#### Geschriebene Dateien

- `itemData/NNNN`.
- **ARM9** (`saveItemButton_Click`, `ItemEditor.cs` Z. 1022–1043) für die Zuordnung Data/Icon/Palette/GBA.
- Item-Icons.

**Code-Befund [Code, Auswirkung unverifiziert]:**
- `ItemData.ToByteArray()` schreibt 36 B: Die Party-Parameter schreiben selbst 2 Byte Padding, danach schreibt der Aufrufer noch einmal 2.
- Vanilla-Member sind 34 B; pret `sizeof(ItemData)` ist wegen `ALIGN_4` 36.
- Für ein Round-Trip: 34 B schreiben bzw. die Originallänge erhalten.

Separate Item-Editoren, nicht Teil von 3b.3: Läden (Overlay), Hidden Items, Pickup (je Sprache), Untergrund, Beeren.

---

## 2. Querverweise („Verwendet in“, Referenzzähler)

### 2.1 Richtung „A verweist auf B“

| Quelle (Feld) | Ziel | Kodierung |
|---|---|---|
| Header `wild_pokemon` | Encounter-Datei | DPPt u16 @0x0E (0xFFFF = keine), HGSS u8 @0x00 (0xFF = keine) |
| Encounter-Slots (alle Gruppen, Swarm, Radio, Dual Slot, Radar, Nachtangeln) | Species | DPPt u32, HGSS u16; 0 = leer |
| DPPt-Encounter Formraten / Unown | Formen von Shellos, Gastrodon, Unown | |
| Header (DPPt) | Swarm-Ortstabelle, Headbutt-Datei (HGSS je Header-ID) | eigene Tabellen |
| Trainer `class` | Trainerklasse | Klassenname-Text, Grafik, Preisgeld, Geschlecht, Eye-Music |
| Trainer `items[4]` | Item-IDs | |
| Trainer-ID | Name (Textarchiv Trainernamen, Index = Trainer-ID), Party-Member (gleicher Index) | |
| Party `species` | Species (Bits 0–9) + Form (10–15) | Form nur sinnvoll für Species mit Formen |
| Party `item` | Item-ID | |
| Party `moves[4]` | Move-IDs | |
| Party `ballCapsule` | Ballkapsel-ID (Pt/HGSS) | |
| Personal `item1/item2` | Item-IDs | |
| Personal `ability1/2` | Fähigkeits-ID (Text) | |
| Personal `type1/2` | Typ-ID (Text) | |
| Personal TM-Bits | TM/HM-Nummer → Item → Move (TM-Tabelle in ARM9, nicht im Personal) | |
| Evolution `target` | Species | |
| Evolution `param` | Item / Move / Species / Level, je nach Methode | |
| Learnset-Eintrag | Move-ID | |
| Egg Move | Species (Marker), Move-IDs | |
| Item-ID | itemData-Member, Icon, Palette (ARM9-Tabelle) | |
| Move-ID | Name, Beschreibung (Index = ID), Effekt-Skript | |
| Event (Overworld mit Trainer-Typ) | Trainer-ID über die Script-Nummer | 3000..4999 → Trainer = script − 3000 + 1; 5000..6999 = Doppel-Partner, Trainer = script − 5000 + 1 (Avalonia `FieldPlayer.cs`, `TrainerScripts`) |
| Scripts | Trainer, Species, Items, Moves (Parameter je Befehl) | Pt: `0x00E5` TrainerBattle, `0x0023–25` Trainer-Flags, `0x00D8`, `0x00E6`, `0x00EA`, `0x0125`, `0x02A0`. HGSS: `0x00D5`, `0x0024–26`, `0x00D6`, `0x00DA`, `0x01CC`, `0x0232` (Avalonia `TrainerReferenceScanner.cs`). |
| Trainer-Kampftext-Tabelle (`trtbl`) | Trainer-ID | 4-B-Einträge {u16 trainer, u16 trigger} |
| VS Seeker (Pt), Pokégear-Rematch und Telefonbuch (HGSS) | Trainer-ID | eigene Tabellen |

### 2.2 Vorbild in DSPRE

Avalonia `DSPRE.Core/ROMFiles/TrainerReferenceScanner.cs` und `TrainerEditorViewModel.FindUsesAsync`:
- Ein Scan über Events, Scripts (inkl. Functions), Kampftexte, VS Seeker, Pokégear-Rematch und Telefonbuch.
- Ergebnis-Datensatz `TrainerUse(Kind, Location, Places, SourceId)`. „Places“ sind die Header, die die Script- bzw. Event-Datei laden (`HeaderLinks.HeadersLinking`).
- Bei 0 Treffern: „Not used anywhere.“
- `OpenUse` springt in den Script- bzw. Event-Editor.
- Beim Entfernen eines Trainers wird gescannt; fehlt der entpackte Ordner, wird das Entfernen verweigert („must be unpacked before removing a trainer“).

### 2.3 Empfehlung für die Spec

- Pro Entitätstyp einen Index Ziel-ID → Liste von (Quelle, Ort, Navigationsziel), beim Projektladen gebaut und bei jedem Save inkrementell aktualisiert.
- Zähler „Verwendet in N“ im Listeneintrag und im Detail.
- Löschen bzw. „letzte Datei entfernen“ blockieren, solange N > 0, so wie DSPRE-Avalonia für Encounter-Dateien.
- Für Species, Item und Move sind die Quellen oben vollständig nur mit dem Script-Parser (3b.2) erfassbar. Script-Treffer als eigene Kategorie mit „mindestens“-Semantik ausweisen: Variablen-Parameter sind statisch nicht auflösbar.

---

## 3. Pokémon Studio und ähnliche Editoren

Quelle: Subagent, Shallow-Clone PokemonStudio `develop` vom 20.10.2026. Pfade relativ zu `https://github.com/PokemonWorkshop/PokemonStudio/blob/develop/`. Wiki und Release-Notes wurden nicht gelesen.

### 3.1 Pokémon Studio

**Seitenaufbau**
- Eine Entität je Seite, keine Listenansicht (`src/views/pages/database/*.page.tsx`).
- Oben eine Control-Bar mit:
  - „Neu“.
  - Durchsuchbarem Select: `StudioDropDown.tsx` filtert auf dbSymbol und Label.
  - Form-Select und „Neue Form“.
- Darunter „Data Blocks“ (Karten mit Lesewerten). Ein Klick öffnet einen Editor-Drawer.
- Tabs: Creature / Movepool / Resources.

**Navigation**
- Prev/Next nur per Tastatur (`hooks/useShortcuts.tsx`):
  - Ctrl/Alt+←/→ zur vorigen/nächsten Entität, Ctrl+Shift+←/→ zur Form.
  - Ctrl+N neu.
- Die Shortcuts sind gesperrt, solange ein Dialog offen ist.

**Movepool** (`database/pokemon/movepool/MovepoolTable.tsx`)
- Inline-Tabelle: Move-Select, dazu read-only Typ, Kategorie, AP, Stärke, Genauigkeit, Löschen.
- Doppelte Moves rot (`getOccurrences`).
- „Zuletzt hinzugefügt“ wird hervorgehoben.
- „Add several“-Dialog, „Import“ aus anderer Species/Form, „Clear“ mit Bestätigung.

**Evolutionen** (`EvolutionEditor.tsx`)
- Paginiert, Bedingungen als Liste.
- Ctrl+Klick auf das Ziel öffnet diese Species.

**Trainer-Party** (`PokemonBattlerList.tsx`)
- Kartenraster.
- „Reihenfolge ändern“ als Drag-and-drop-Modal.
- Showdown-Import.
- Level min/max mit Live-Fehler.

**Wild-Gruppen**
- Gleiche Komponente.
- Kopf-Tag „N %“ mit der Summe der Chancen; keine Normalisierung.

**Referenzen**
- Kein Usage-Check beim Löschen: `Deletion.tsx` zeigt nur „unwiderruflich“.
- Hängende Referenzen werden tolerant als „… deleted“ / „???“ mit Fehlerstil gerendert (`StudioDropDown` `deletedOption`, `MovepoolTable`).
- Reverse-Lookups gibt es als read-only Unterseiten: `Move.Pokemon.page.tsx` („Creatures with move X“, gruppiert nach Lernart, Level-Spalte, Navigation); analog Ability und Type.

**Validierung**
- Zod-Schemata in `src/models/entities/creature.ts`.
- Die Editoren nehmen per `.pick()` Teilschemata.
- `useZodForm` leitet min/max für die Inputs ab; Fehler stehen inline.
- Editoren übernehmen beim Schließen, aber nur wenn `canClose()` gültig ist (`useHandleCloseEditor.tsx`).
- Kein Undo. Speichern projektweit (Ctrl+S). Warnung beim Beenden.

**Bulk**
- Keine Tabellen- oder Bulk-Edit-Ansicht für Entitäten. Tabellen gibt es nur für Dex, Typ-Matrix, TM-Items und Zonen-Gruppen (`DexPokemonListTable.tsx`, virtualisiert).

### 3.2 Andere

**Porymap** (Wild-Encounter; `src/ui/encountertablemodel.cpp`, `docsrc/manual/editing-wild-encounters.rst`)
- Je Feldtyp ein Tab; leere Feldtypen sind deaktiviert.
- Spalten: Slot, Gruppe, Species, Min, Max, berechnete Chance „%.2f%“, Slot-Ratio, Rate.
- Level-Auto-Klemmung: min hebt max an und umgekehrt.
- Gruppen-Dropdown für Varianten; „Neue Gruppe aus aktueller kopieren“.
- Artübergreifende Species-Suche über alle Maps mit sortierbaren Ergebnissen; Doppelklick öffnet.
- Zusammenfassungsdiagramm.

**HexManiacAdvance**
- Tabellen-Werkzeug mit Sprungpfeil neben jedem Fremdschlüssel.
- „Find Uses“ öffnet einen Ergebnis-Tab.
- Vor dem Editieren geteilter Daten: „used by 25 pointers [Repoint] [All]“.

**RPG Maker MZ** (https://rpgmakerofficial.com/product/MZ_help-en/01_08.html)
- Liste links, Detail rechts.
- Copy/Paste/Clear auf Listeneinträgen.
- „Change Maximum“ mit der Warnung, dass Daten über dem Limit gelöscht werden.

**PKHeX Batch Editor**
- Filter- und Setzzeilen (`=Prop=Value`, `.Prop=Value`). Kein Undo.

### 3.3 Fünf Muster für die „Datenbank-UI nach Pokémon-Studio-Muster“

1. **Kopfleiste je Datenbank: Suche über ID und Name, Form-Select, Tastatur-Prev/Next, „Neu/Duplizieren“.**
   - Quelle: Studio `PokemonControlBar.tsx`, `StudioDropDown.tsx`, `useShortcuts.tsx`.
   - Ergänzung für Gen 4: die Liste links (RPG-Maker-Stil) mit Spalten statt nur Dropdown, sortierbar.
   - Die Selektion hängt immer an der ID, nicht am Listenindex; das verhindert den Bug aus D#79.
2. **Inline-Tabelle mit verbundenen Read-only-Spalten für Unterlisten.**
   - Learnset, Egg Moves, TM, Party-Moves und Encounter-Slots.
   - Doppelte rot markieren, „Add“-Zeile mit Vorauswahl.
   - „Aus anderer Species übernehmen“ (`MovepoolTable.tsx`, `MovepoolImport.tsx`).
   - Dazu harte Gen-4-Grenzen als Zähler „18/20“: Learnset 20, Egg Moves 16, Evolution 7, Party 6.
3. **Referenzen in beide Richtungen.**
   - Fremdschlüssel als Link (Ctrl+Klick bzw. Pfeil wie HMA).
   - Reverse-Panel „Verwendet in N“, gruppiert nach Quellenart wie `Move.Pokemon.page.tsx` und DSPRE `TrainerUse`.
   - Löschen bzw. Entfernen mit Zähler im Dialog blockieren oder warnen; Studio selbst tut das nicht, DSPRE-Avalonia für Encounter-Dateien schon.
4. **Tolerante Darstellung unbekannter Werte.**
   - Hängende IDs als „??? (#1234)“ im Fehlerstil.
   - Bereiche nur für neue Eingaben einschränken; ein geladener Wert außerhalb bleibt erhalten (DSPRE-Avalonia `MoveDataEditorViewModel` Z. 153; Abstürze in D#312 mit modifizierten ROMs).
5. **Schema-getriebene Validierung als Warnungen mit Ort, plus Override-Anzeige.**
   - Ein Schema liefert Input-Grenzen, Inline-Fehler und die Save-Prüfung (Studio `useZodForm`).
   - Der Save-Dialog listet Probleme konkret („Surf slot 3: lowest level above highest (the game swaps them)“) mit „Trotzdem speichern“ (DSPRE `WildEncounterChecks`).
   - Für Encounter: berechnete, nur lesbare Slot-Prozente und die Spalte „Ersetzt durch“ (Swarm, Radio, Tag/Nacht, Radar, Dual Slot) mit Warnung bei leeren Ersatz-Slots.
   - Dazu die Porymap-Level-Klemmung.

Optional:
- **Bulk-Edit als Tabellenansicht** (Spreadsheet-Modus mit Mehrfachauswahl, Spalte setzen, CSV). Das hat Studio nicht; DSPRE hat es für Learnsets, TM und Personal-CSV.
- **Muster „Shared-Data-Warnung“ aus HMA:** eine Encounter-Datei, die mehrere Header teilen, anzeigen und „für diesen Header abspalten“ anbieten.

---

## 4. Fallstricke (aus Code, Daten und Issues)

### 4.1 IDs, Indizes, Sortierung

- **Item-ID ≠ itemData-Index** wegen der ARM9-Tabelle; viele IDs zeigen auf Member 0 [geprüft]. Editor-Schlüssel ist die Item-ID; die Datenzeile wird über die Tabelle aufgelöst. Mehrere IDs können dasselbe Member teilen, deshalb mit Shared-Hinweis arbeiten.
- **Namenszahl ≠ Datenzahl:**
  - Species 496 zu 508, Moves 468 zu 471, Items 468 zu 446 bzw. 537 zu 514 [geprüft].
  - DP hat weniger Personal-Member als Formnamen (D#244).
  - Zählen nach Datenarchiv; Namen für Formen kommen aus einer eigenen Tabelle.
- **Listen-Index statt ID:** D#79 (gefilterter Bulk-Learnset löscht die falsche Zeile), D#225 und D#231 (Combo-Suche wählt die falsche Species bzw. stürzt ab).
- **Trainer 0:** Dummy (alle Felder 0, Party 8 B Nullen) [geprüft]. Script-Nummern sind 1-basiert (`script − 3000 + 1`).
- **Personal-Formen:** Index 496–507 mit fester Bedeutung je Spiel (Abschn. 1.4). Die Default-Form mancher Species liegt in einem anderen Archiv (Changelog 2.3) [unverifiziert].
- **Platin enthält DP-Altarchive;** `pl_`-Dateien verwenden [geprüft].
- **Textarchiv-IDs sprachabhängig:** JP-HGSS weicht ab (D#216, A#78, A#79).

### 4.2 Versteckte Bytes und Round-Trip

- Personal-Padding 0x1A–0x1B und EV-Bits 12–15 sind in Vanilla 0 [geprüft]. Modifizierte ROMs können dort etwas haben; erhalten.
- DPPt-Encounter-Block 0x0F8–0x123 ungenutzt (Vanilla 0). DSPRE überschreibt ihn mit 0.
- DPPt Walking-Level-Padding (3 B je Slot) und Water-Padding (2 B) sind in Vanilla 0 [geprüft]; erhalten statt neu erzeugen. A#37 und D#12 zeigen, dass DSPRE früher Formdaten beim Speichern zurücksetzte.
- Trainer-Party-Dateien sind auf 4 B aufgefüllt [geprüft]. DSPRE schreibt ohne Padding.
- Trainer-AI: DSPRE kappt auf 11 Bit; battleType auf 0/2. Moves 0xFFFF → 0.
- Item-Writer in DSPRE schreibt 36 statt 34 B [Code].
- Evolution-Writer verdichtet ungültige Zeilen und entfernt sie still [Code]. Learnset-Reader verwirft Duplikate [Code].
- **Empfehlung:** Jedes Format im Kernel als „Rohbytes plus geparste Felder“ führen und nur geänderte Felder zurückschreiben. Gated Test `write(read(x)) == x` über alle Member aller drei Projekte, wie es schon für NARC und Text gilt (`docs/formats/narc.md`, `text_archive.md`).

### 4.3 Party-Formate

- Größe hängt von `monDataType` (Bits Moves/Item) und Familie ab: DP ohne Ball-Kapsel, also 6/8/14/16 B; Pt/HGSS 8/10/16/18 B.
- Ein Umschalten von „eigene Moves“ oder „Items“ formatiert alle Mons um; dabei Moves/Items erhalten oder ausdrücklich verwerfen.
- D#115, #182, #168 und #218: Dirty-Tracking verlor Moves, „Choose Moves“ war nach dem Laden falsch.
- Das Flag-Byte im Trainer und das Party-Format müssen eine Quelle der Wahrheit haben.
- Byte 1 eines Party-Eintrags ist in HGSS Gender/Ability, in Pt (pret) das High-Byte der u16-IV-Skala. Nur für HGSS (oder Pt mit Backport) als Gender/Ability zeigen.
- „Default“ bei Ability/Gender heißt in Pt/HGSS eigentlich „Auto“: Die PID-Erzeugung verkettet frühere Mons (D#290, D#306, PR#151) [unverifiziert].
- Form-Bits (10–15) nur für Species mit Formen anbieten; Vorschau formspezifisch.
- A#20: HGSS-DV mit Bit 13 gesetzt, wohl ein Flag. Bits maskieren und unbekannte erhalten [unverifiziert].
- Trainernamen: Vanilla-Limit 7 Zeichen. Längere Namen nur mit ARM9-Patch; Limit aus dem Projekt lesen, nicht hart kodieren (A#25 ff., D#306).

### 4.4 HGSS-Encounter

- Ein Level je Walking-Slot, gemeinsam für Morgen/Tag/Nacht. Water- und Rock-Smash-Slots haben min/max.
- In der UI drei Species-Spalten, eine Level-Spalte.
- Swarm-Index 2 ist **Nachtangeln**, kein Swarm (pret `nightFish`).
- Byte-Reihenfolge min/max ist in HGSS umgekehrt zu DPPt [geprüft].
- `wild_pokemon` ist u8: Encounter-Dateien > 254 sind nicht referenzierbar.
- Leere Ersatz-Slots (Swarm, Radio, Nachtangeln, DPPt Tag/Nacht, Radar, Dual Slot) ergeben im Spiel Glitch-Pokémon (Avalonia-Warnungen).
- Slot-Prozente stehen im Overlay-Code; Anzeige read-only oder über einen separaten Patch-Editor (D#269).

### 4.5 Sonstiges

- **Harte Grenzen:**
  - Learnset 20 Einträge (pret `MAX_LEARNSET_ENTRIES`).
  - Evolution 7 (Erweiterung braucht einen ARM9-Patch an 4 Stellen, D#3).
  - Egg Moves 16 je Species, Tabelle 0xEEC B (DPPt).
  - Move-ID ≤ 511 im Learnset.
  - Species ≤ 1023 im Party-Feld.
- **Punching- und Sound-Flags** sind keine Move-Felder, sondern Tabellen im Battle-Overlay (D#285).
- **Hacks** (hg-engine, Salty Platinum) brechen Vanilla-Annahmen: Form-Bits in Encounter-Species (A#76), Comboboxen mit Indizes außerhalb des Bereichs (D#312). Nie werfen, „(unbekannt #n)“ zeigen.
- **Gepackt vs. entpackt:** NARC und `unpacked/`-Ordner können divergieren (D#288, D#281). Der Kernel kennt das Problem schon aus 3b.1/3b.2; für die neuen Archive gilt dieselbe mtime-Regel.

---

## Anhang: Datei- und Code-Fundstellen

**DSPRE `main` (`$HOME/Projekte/lab/editor/DSPRE/DS_Map/`)**
- `ROMFiles/EncounterFile.cs`, `ROMFiles/TrainerFile.cs`, `ROMFiles/PokemonPersonalData.cs`, `ROMFiles/MoveData.cs`, `ROMFiles/ItemData.cs`, `ROMFiles/EvolutionFile.cs`, `ROMFiles/LearnsetData.cs`
- `Editors/WildEditorDPPt.cs`, `Editors/WildEditorHGSS.cs`, `Editors/TrainerEditor/TrainerEditor.cs`, `Editors/PersonalDataEditor.cs`, `Editors/PokemonEditor.cs`, `Editors/LearnsetEditor.cs`, `Editors/EggMoveEditor.cs`, `Editors/MoveDataEditor.cs`, `Editors/ItemEditor.cs`
- `RomInfo.cs`: Pfade Z. 2310–2515, Textarchive Z. 1369–1832, Item-Tabelle Z. 859
- `Resources/PokeDatabase.cs` Z. 1880

**DSPRE `feature/avalonia`**
- `DSPRE.Avalonia/Avalonia/ViewModels/Pokemon/{WildEditorDPPtViewModel,WildEditorHGSSViewModel,WildTimeRow,SwarmsViewModel,MoveDataEditorViewModel}.cs`
- `DSPRE.Avalonia/Avalonia/ViewModels/Trainers/TrainerEditorViewModel.cs`
- `DSPRE.Core/ROMFiles/{TrainerReferenceScanner,EncounterSlotOdds,FieldPlayer}.cs`

**pret** (`…/scratchpad/research-3b3/pret/`)
- `pokeplatinum/include/overlay006/wild_encounters.h`, `pokeplatinum/include/struct_defs/trainer_data.h`, `pokeplatinum/include/struct_defs/species.h`, `pokeplatinum/include/move_table.h`, `pokeplatinum/include/item.h`
- `pokeheartgold/include/wild_encounter.h`, `pokeheartgold/include/trainer_data.h`

**uxie 0.9.7 (MIT)**
- `src/{personal_data,trainer_data,encounter_file,map_header,item_data,move_data,evolution_data,learnset_data,egg_move_data}/binary.rs`
- Lesend nutzbar und lizenzfrei übernehmbar.
- Abweichung: Pt-Party Byte 0–1 als u16, wie in pret.

**Issues**
- https://github.com/DS-Pokemon-Rom-Editor/DSPRE/issues/ mit Nummern wie oben (D#…).
- https://github.com/AdAstra-LD/DS-Pokemon-Rom-Editor/issues/ (A#…).
- Changelog: https://github.com/DS-Pokemon-Rom-Editor/DSPRE/blob/main/Changelogs/CHANGELOG_2.3_User.md
