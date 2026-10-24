# Begegnungs-Mechanik für Phase 3b.3b: Raten, Slots, Ersatzlisten, Gate-Beweis

Stand: 24.10.2026. Ergänzt `dspre-report.md` §1.1–§1.2 und beantwortet die offenen Punkte zu Begegnungen aus Spec §15 (`docs/superpowers/specs/2026-10-23-phase-3b3-data-editors-design.md`).

## Quellen

**pret-Decomps.** Sparse-Clones, nur gelesen:

| Decomp | Commit | Kurzname unten |
|---|---|---|
| [pret/pokeplatinum](https://github.com/pret/pokeplatinum) | `be36a9db3d2390a5009adc087c7aef550ce335e7` | **PT** |
| [pret/pokeheartgold](https://github.com/pret/pokeheartgold) | `6988158691eb6178cff697cda0428045ddcba5fb` | **HG** |

Links zeigen auf diese Commits. Zeilennummern stammen aus diesen Ständen.

**Hauptquellen**
- PT `src/overlay006/wild_encounters.c` ([Link](https://github.com/pret/pokeplatinum/blob/be36a9db3d2390a5009adc087c7aef550ce335e7/src/overlay006/wild_encounters.c)): Gras und Surfen ab Z. 231 (`WildEncounters_TryWildEncounter`).
- HG `src/field/encounter_check.c` ([Link](https://github.com/pret/pokeheartgold/blob/6988158691eb6178cff697cda0428045ddcba5fb/src/field/encounter_check.c)): Land und Surfen ab Z. 215 (`FieldSystem_PerformLandOrSurfEncounterCheck`).

**Eigene Messungen**
- Werkzeug: `g4emu` (melonDS DS 1.4.0) auf drei neuen Kopien unter `Roms/_testkopien`:
  - `pt-3b3b-enc`: Platin DE;
  - `hg-3b3b-enc`: HeartGold DE;
  - `pt-3b3b-vanilla`: Kopie der unveränderten Platin-DE-ROM.
- Spielstände aus `_testkopien/_saves`.
- Bilder, RAM-Abzüge und Hilfsskripte liegen in `_testkopien/pt-3b3b-enc/_messung` und `_testkopien/hg-3b3b-enc/_messung`.
- Hilfsskripte:
  - `narc_enc.py` hängt ein Begegnungs-Member an ein NARC an oder ersetzt eines;
  - `sig.py` sucht den Kampf-Datensatz eines Pokémon (`BattleMon`) im RAM;
  - `ptr.py` und `cand.py` vergleichen RAM-Abzüge;
  - `montage.py` setzt Bilder nebeneinander.
- Python lief immer mit `-I`.

**Kennzeichnung**
- **geprüft**: im Emulator gemessen oder an Projektdaten nachgerechnet.
- **abgeleitet**: aus dem pret-Quelltext gelesen, nicht gemessen.
- **offen**: weder gemessen noch sicher abgeleitet.

## Ergebnisübersicht

| # | Aussage | Status | Beleg |
|---|---|---|---|
| 1a | Rate ist ein Prozentwert der zweiten Würfelprobe je Schritt; Pt liest die u32 als u8, HGSS die u8 direkt. Über 100 wirkt wie 100. In Pt sind 256 und mehr modulo 256 (256 = nie). | abgeleitet | §1 |
| 1b | Rate 100 garantiert **keine** Begegnung je Schritt. Pt: 40 % je Grasschritt (hohes Gras oder Rad 70 %). HGSS: 20 % je Schritt, bis 80 % nach vier Kehrtwenden, die ersten 4 Schritte nach Kartenwechsel nie. | abgeleitet; Messung passt (Pt 3., HG 2. Grasschritt) | §1, §4.3 |
| 1c | Modifikatoren hängen am ersten Team-Pokémon (Fähigkeit, Item), an Flöten, Schutz, Datum (Pt), Radio (HGSS) und Rad/hohem Gras. Mit leerem Team wirkt keiner. | abgeleitet | §1.3 |
| 2 | Gras 20/20/10/10/10/10/5/5/4/4/1/1 in beiden Spielen. Pt: Surfen 60/30/5/4/1, Angel 60/30/5/4/1, Profi- und Super-Angel 40/40/15/4/1. HGSS: Surfen 60/30/5/4/1, alle Angeln 40/30/15/10/5, Zertrümmerer 80/20, Kopfnuss 50/15/15/10/5/5. | abgeleitet (Code) | §2 |
| 3a | Pt: Tag/Nacht ersetzt Gras-Slots 2, 3 (10–19 Uhr Tag, 20–3 Uhr Nacht, 4–9 Uhr Grundliste). Schwarm ersetzt 0, 1; Radar 4, 5, 10, 11; Dual-Slot 8, 9; Trophäengarten 6, 7. | abgeleitet | §3.1 |
| 3b | HGSS: die Art-Spalte kommt aus der Tageszeit (4–9 morgens, 10–19 tags, 20–3 nachts). Radio Hoenn/Sinnoh ersetzt 2, 3 bzw. 4, 5; Schwarm ersetzt Gras 0, 1, Surfen 0 und die Angel-Slots. | abgeleitet | §3.2 |
| 3c | Im Emulator mit den Zimmer-Spielständen wirken nur Grundliste und Tageszeit-Ersatz: kein Dual-Slot, kein Radar, kein Schwarm, kein Radio. | abgeleitet; Uhrzeit des Emulators offen | §3.3 |
| 4a | Beide Spielstände haben **kein** Pokémon im Team (Team-Zähler 0). | geprüft (Spielstand-Bytes) | §4.1 |
| 4b | Ohne Team startet trotzdem ein Wildkampf, mit einem „Lv0 0/0“-Platzhalter; der Kampf endet mit Niederlage, das Spiel bringt die Figur nach Hause (Pt 414, HGSS 63). | geprüft | §4.3, §4.4 |
| 4c | Gegner-Art und -Level stehen im `BattleMon`-Datensatz (Größe 0xC0, Gegner direkt hinter dem Spieler). Eine Signatur aus drei Feldern findet ihn eindeutig. Ein stabiler statischer Zeiger wurde nicht gefunden. | geprüft (Pt DE, HG DE) | §4.5 |
| 4d | Route 201 (Header 342, Datei 140) ist vom Pt-Spielstand aus gesperrt: Eine Figur blockiert den Ortsausgang, danach folgen Story-Szenen. Der billigste Beweis ist ein neuer Ort ohne Trigger. | geprüft (Sperre), abgeleitet (Szenen) | §4.2 |
| 5 | Gras auf einer per `create_area` kopierten Karte in einem **neuen** Header löst Begegnungen aus. Es entscheiden nur `wild_pokemon` ≠ „keine“, das Kachelverhalten der Karte und die Rate in der Datei. | geprüft (Pt 593, HG 540) | §5 |
| 6 | Member-Zahl Pt 183, HGSS 142 (geprüft). Das Spiel liest die Zahl zur Laufzeit aus dem NARC; es gibt keine arm9-Tabelle. Index N (= neues Member) funktioniert. Grenzen: Header-Feld Pt u16 (bis 65534), HGSS u8 (bis 254), Member genau 424 bzw. 196 B. | geprüft (Pt 183, HG 142), Rest abgeleitet | §6 |
| offen | Uhrzeit des melonDS-DS-Kerns (Host-Uhr?); Verhalten bei Art 0 in einem wirksamen Slot; Zeiger-Kette zum `BattleMon` über Läufe mit anderem Team | offen | §7 |

---

## 1. Rate-Semantik

### 1.1 Platin

**Rate lesen.** Pfad im Code (PT `wild_encounters.c`):
- `GetTileEncounterRateAndType` (Z. 782) liefert die Gras- oder Surf-Rate als **u8**.
- Sie kommt aus `int encounterRate` des geladenen `WildEncounters` (`include/overlay006/wild_encounters.h`) über `GetGrassEncounterRate` (Z. 1258).
- Von der u32 im Member zählt also nur das untere Byte:
  - Rate 256 wirkt wie 0 (keine Begegnung);
  - Rate 300 wirkt wie 44.

**Ablauf je Schritt.** Der Aufruf kommt aus `Field_CheckWildEncounter` (PT `src/overlay005/field_control.c` Z. 543), nach jedem beendeten Schritt. Voraussetzung: Der Header hat eine Datei (`MapHeader_HasWildEncounters`, `wildEncountersArchiveID != 0xFFFF`, PT `src/map_header.c` Z. 72), und die Kachel hat das Flag `TILE_BEHAVIOR_FLAG_ENCOUNTER` (Gras 0x02, hohes Gras 0x03, Höhlenboden usw., PT `src/map_tile_behavior.c`).

1. **Rate anpassen:** zuerst `ModifyEncounterRateWithFieldParams` (Z. 1326), dann `ModifyEncounterRateWithFlute` (Z. 930), dann `ModifyEncounterRateWithHeldItem` (Z. 921). Ist das erste Team-Pokémon kein Ei, wird auf 100 begrenzt.
2. **Schonfrist** `GracePeriodStepsUsed` (Z. 798): Grenze = 8 − min(8, Rate/10).
   - Solange `encounterAttempts` darunter liegt, zählt der Zähler hoch, und nur 5 % der Schritte kommen weiter.
   - Bei Rate ≥ 80 ist die Grenze 0, es gibt keine Schonfrist.
   - Der Zähler wird nach jeder Begegnung auf 0 gesetzt.
3. **Pauschalprobe** `ShouldGetRandomEncounter` (Z. 750): `rand(100) < 40`.
   - +30 bei hohem Gras (Verhalten 0x03) oder auf dem Rad.
   - ±5/±10 an festen Kalendertagen (`SpecialDates_ModifyEncounterRate`, PT `src/overlay006/special_dates.c` Z. 125); nicht, wenn die Uhr kürzlich verstellt wurde. Für den 24.10. gibt es keinen Eintrag; im Oktober gibt es +5 am 5., 15. und 30.
   - Begrenzt auf 100.
4. **Ratenprobe:** `rand(100) < Rate` (`CheckEncounterRateSuccess`).
5. **Poké-Radar und Roamer:**
   - Ein aktiver Poké-Radar-Fleck erzwingt die Begegnung.
   - Ein Roamer auf diesem Header ersetzt sie mit 50 %. Roamer hängen an Header-IDs, nie an neuen Headern.

**Ergebnis für Rate 100** in normalem Gras, ohne Team-Modifikatoren: **40 % je Schritt**. In hohem Gras oder auf dem Rad 70 %. Eine Garantie gibt es nicht. Rate 100 schaltet nur die Schonfrist ab und macht die zweite Probe sicher.

### 1.2 HeartGold/SoulSilver

`FieldSystem_PerformLandOrSurfEncounterCheck` (HG `encounter_check.c` Z. 215).

1. **Sperre nach Kartenwechsel:** Solange `encounterInhibitSteps <= 3`, gibt es keine Begegnung.
   - Der Zähler steigt bei jedem Schritt (HG `src/field/field_control.c` Z. 471).
   - Er wird beim Kartenwechsel und nach jedem Kampf auf 0 gesetzt (`include/field_system.h` Z. 209: „don't generate an encounter in the first 4 steps on a new map“).
2. **Rate** ist das u8 `encounterRate_walking` (HG `include/wild_encounter.h`). Danach greifen Fähigkeit (Z. 1120), Flöte und Item wie in Pt.
3. **Pauschalprobe** `FieldSystem_EncounterRateRoll` (Z. 565):
   - Grundwert 40.
   - −20 zu Fuß. Ausnahme: Rad, Surfen und ein Laufzustand 0x58–0x5B (`sub_0205DE98`, vermutlich Rennen, nicht bestätigt).
   - +40 in hohem Gras oder +30 auf dem Rad.
   - Kehrtwende-Bonus `getStepCountEncounterRateBoost` (Z. 595): ab 2 Kehrtwenden +30, ab 3 +40, ab 4 +60. `reverseTurnFrameSteps` zählt, wie oft die Blickrichtung sich auf Begegnungskacheln umkehrt (Z. 1397). Er wird bei Kartenwechsel und Kampf zurückgesetzt.
   - Radio „Marsch“ +25, „Wiegenlied“ −25.
   - Begrenzt auf 100.
4. **Ratenprobe:** `rand(100) < min(Rate, 100)` (`FieldSystem_SecondEncounterRoll`).

**Ergebnis für Rate 100** zu Fuß:
- ab dem 5. Schritt nach dem Kartenwechsel 20 % je Schritt;
- beim Hin-und-her-Laufen nach vier Kehrtwenden 80 %;
- auf dem Rad oder in hohem Gras bis 100 %.

### 1.3 Modifikatoren und das Gate

**Wirkung je Modifikator.** Alle hängen am **ersten** Team-Pokémon, außer Flöte und Schutz.

| Modifikator | Wirkung |
|---|---|
| Ausweglos, Schildlos, Erleuchtung | Rate × 2 |
| Duftwolke, Pulverrauch, Rasanz | Rate ÷ 2 |
| Sandschleier bei Sandsturm (nur Pt), Schneemantel bei Schnee | Rate ÷ 2 |
| Item Schutzband oder Scharmrauch | Rate × 2/3 |
| Schwarze Flöte | Rate ÷ 2 |
| Weiße Flöte | Rate × 1,5 |
| Adlerauge, Bedroher | bei Level des ersten Pokémon > 5: Begegnung mit Level ≤ Level − 5 wird zu 50 % verworfen (PT Z. 1359, HG Z. 1145) |
| Schutz (Repel) | verwirft Begegnungen unter dem Level des ersten kampffähigen Pokémon |
| Magnetfalle, Statik | bevorzugen zu 50 % Slots vom Typ Stahl bzw. Elektro; wirkungslos, wenn alle Slots dieselbe Art haben |
| Übereifer, Erzwinger, Munterkeit | bevorzugen den höchsten Level derselben Art; wirkungslos bei gleichen Leveln |

**Ohne Team.** Das erste Pokémon ist leer: Art 0, Fähigkeit 0, kein Ei. Dann wirkt **kein** Modifikator. Die Rate wird nur auf 100 begrenzt.

**Für das Gate**
- Gibt das Szenario vorher ein Pokémon (Spec §13 Schritt 5), muss dessen Fähigkeit neutral sein. Ein Level-100-Pokémon mit Adlerauge oder Bedroher verwirft jede zweite Begegnung mit Level ≤ 95.
- Datum und Uhr kommen vom Emulator. Bei Rate 100 und Hin-und-her-Laufen sind 30 Schritte mehr als genug: Die Fehlchance ist in Pt 0,6^30 ≈ 2·10⁻⁷.

## 2. Slot-Anteile

Ein `rand(100)` wählt den Slot über feste Grenzen im Code (Overlay 6 bzw. Overlay 2, siehe DSPRE-Bericht).

**Gras, beide Spiele.** Quellen: PT `GetGroundEncounterSlot` Z. 820, HG `EncounterSlot_WildMonSlotRoll_Land` Z. 632. Gras-Level ist fest je Slot (`maxLevel = minLevel = level`).

| Slot | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 | 11 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| % | 20 | 20 | 10 | 10 | 10 | 10 | 5 | 5 | 4 | 4 | 1 | 1 |

Slot 10 hat genau den Wurf 98, Slot 11 den Wurf 99.

**Wasser und Sonderarten**

| Methode | Pt | HGSS |
|---|---|---|
| Surfen | 60/30/5/4/1 (`GetWaterEncounterSlot` Z. 851) | 60/30/5/4/1 (Z. 662) |
| Angel | 60/30/5/4/1 (`GetRodEncounterSlot` Z. 868) | 40/30/15/10/5 (Z. 678, gleich für alle Angeln) |
| Profi-Angel | 40/40/15/4/1 | 40/30/15/10/5 |
| Super-Angel | 40/40/15/4/1 | 40/30/15/10/5 |
| Zertrümmerer | – (Block in Pt unbenutzt) | 80/20 (Z. 694) |
| Kopfnuss | – | 50/15/15/10/5/5 (Z. 700; eigene Datei je Header) |

**Level im Wasser.** Gleichverteilt zwischen min und max. Liegt max unter min, tauscht das Spiel die beiden (PT `GetWildMonLevel` Z. 953).

**Abgleich.** Die Werte decken sich mit DSPRE D#269 und mit `dspre-report.md`. Die Spec-Annahme für Gras ist bestätigt.

## 3. Tageszeit und Ersatzlisten

### 3.1 Platin

**Tageszeit.** `GetTimeOfDay` (PT `src/rtc.c` Z. 145) teilt die Stunden so ein:
- 0–3 späte Nacht;
- 4–9 Morgen;
- 10–16 Tag;
- 17–19 Dämmerung;
- 20–23 Nacht.

Die Ersetzungen laufen auf einer Kopie der 12 Gras-Slots, in dieser Reihenfolge (Z. 334 ff.; Sonderfälle werden **nach** den Grundwerten eingesetzt):

| Quelle (Offset) | ersetzt Gras-Slot | Bedingung |
|---|---|---|
| `day[0..1]` (0x6C) | 2, 3 | Tag oder Dämmerung, 10–19 Uhr (`WildEncounters_ReplaceTimedEncounters` Z. 182; Kommentar „Default encounters are morning“) |
| `night[0..1]` (0x74) | 2, 3 | Nacht oder späte Nacht, 20–3 Uhr |
| `swarm[0..1]` (0x64) | 0, 1 | Schwarm aktiv (`SpecialEncounter_IsSwarmEnabled`) **und** Header-ID = Schwarm-Header des Tages (Z. 195) |
| Trophäengarten (`encdata_ex` Member 8) | 6, 7 | Header ist Trophäengarten und nationaler Dex vorhanden |
| Dual-Slot (0xA4–0xCB) | 8, 9 | nationaler Dex **und** GBA-Modul R/S/E/FR/LG im Slot (`gSystem.gbaCartridgeVersion`, PT `src/overlay006/dual_slot_encounters.c`) |
| Großmoor-Tagesarten | 6, 7 | Safari-Spiel aktiv |
| `radar[0..3]` (0x7C) | 4, 5, 10, 11 | nur bei einer Begegnung im Poké-Radar-Fleck mit Schüttelart 1 (`TryGenerateGrassEncounter_WithRadar` Z. 688) |

Die Level bleiben die der Grundslots. Ersetzt wird nur die Art.

**Folge für Editor und Gate.** Zwischen 10 und 3 Uhr, also an 18 von 24 Stunden, kommen die Slots 2 und 3 (20 % aller Grasbegegnungen) aus `day` bzw. `night`.
- Eine neue, leere Datei hat dort Art 0.
- Eine Kopie von Route 201 hat dort Staralili, Bidiza und Zirpurze.
- Für einen sicheren Gate-Beweis müssen deshalb auch `day[0..1]` und `night[0..1]` auf `S_WILD` stehen.
- Der Codec kennt die Felder (`day_0` … `night_1`); `create_encounter_file` oder das Szenario muss sie mit setzen.

### 3.2 HeartGold/SoulSilver

**Tageszeit.** Gleiche Stundengrenzen (HG `src/gf_rtc.c` Z. 166 ff.). `EncSlotArray_Init_Land` (Z. 119) nimmt die Art-Spalte so:
- morgens (4–9): `species_morn`;
- tags und abends (10–19): `species_day`;
- nachts (20–3): `species_nite`.

Das Level kommt immer aus `levels[i]`.

| Quelle | ersetzt | Bedingung |
|---|---|---|
| `landSwarm` (0xBC) | Gras 0, 1 | Schwarm aktiv (`RoamerSave_OutbreakActive`, gesetzt nur per Script-Befehl, HG `src/scrcmd_c.c` Z. 3438) **und** Header steht im Schwarm-Eintrag des Tages (`sSwarmMapLUT`, 20 feste Header-IDs, HG `src/unk_02097F6C.c` Z. 16) |
| Radio Hoenn `[0]` / `[1]` (0x5C) | Gras 2, 3 / 4, 5 | Pokégear-Radio spielt „Hoenn-Klänge“ (`GetRadioMusicPlayingSeq`, Z. 152) |
| Radio Sinnoh `[0]` / `[1]` (0x60) | Gras 2, 3 / 4, 5 | Radio spielt „Sinnoh-Klänge“ |
| `surfSwarm` (0xBE) | Surfen 0 | Schwarm wie oben, Art 1 |
| `nightFish` (0xC0) | Profi-Angel 3, Super-Angel 1 | nachts (Z. 140) |
| `fishSwarm` (0xC2) | Angel 2; Profi 0, 2, 3; Super alle | Schwarm wie oben, Art 2 (Z. 181); wird **nach** `nightFish` angewendet und hat damit Vorrang |

**Abgleich mit `dspre-report.md` §1.2:** bestätigt, mit zwei Präzisierungen.
- Radio `[0]` belegt **beide** Slots 2 und 3, `[1]` beide Slots 4 und 5.
- Schwarm-Berechtigung hängt an der Header-ID in einer arm9-Tabelle. Neue Header schwärmen nie.

### 3.3 Was im Emulator mit den Zimmer-Spielständen wirkt

- **Dual-Slot:** wirkt nie. Es braucht den nationalen Dex und ein GBA-Modul; der Kern hat keinen GBA-Slot belegt.
- **Radar, Trophäengarten, Großmoor:** wirken nicht. Es fehlen Item bzw. Story, und es sind die falschen Header.
- **Schwarm:** wirkt nicht. In Pt schaltet ihn erst ein Story-Fortschritt frei, in HGSS ein Script-Befehl. Neue Header stehen in keiner Schwarm-Tabelle.
- **Radio:** wirkt nicht. Der HG-Spielstand hat das Pokégear (Notiz `IPKD-zimmer.md`), aber noch kein Radio. Damit fallen auch Marsch und Wiegenlied weg.
- **Roamer:** wirken nicht; es gibt in diesem Stand keine, und neue Header haben ohnehin keine.

**Es wirken also nur:**
- in Pt die 12 Grundslots plus der Tag/Nacht-Ersatz der Slots 2, 3;
- in HGSS die Spalte der aktuellen Tageszeit.

Welche Uhrzeit der Kern meldet, ist nicht gemessen (§7). Das Gate muss deshalb alle Tageszeit-Listen gleich setzen.

## 4. Deterministischer Begegnungs-Beweis

### 4.1 Team im Spielstand (geprüft)

Der Team-Block `Party {int capacity; int currentCount; Pokemon[6]}` (PT `include/party.h`) liegt in beiden Spielständen im zweiten Sicherungsblock. Der erste Block (0x0) ist leer.

| Spielstand | Kapazität bei | Wert | Zähler | Bedeutung |
|---|---|---|---|---|
| `CPUD-zimmer.sav` | 0x40098 | 6 | 0 | kein Pokémon |
| `IPKD-zimmer.sav` | 0x40090 | 6 | 0 | kein Pokémon |

```sh
python3 -I -c "import struct; d=open('CPUD-zimmer.sav','rb').read(); print(struct.unpack_from('<2I', d, 0x40098))"
# (6, 0)
```

### 4.2 Route 201 vom Zimmer aus (Platin)

**Wo die Daten stehen** (geprüft an `pl_enc_data.narc` und `dynamicHeaders/0342`):
- Route 201 ist Header 342, Begegnungsdatei 140 (`encounters.order` Zeile 141), Rate 30.
- Die Karten sind 5 und 6 im Raster 0.

**Messung auf `pt-3b3b-vanilla/rom.nds`** (Bilder `m5.png`, `m8.png`):

1. Die Treppe führt nach 414.
2. Mutters Szene startet beim ersten Schritt. Nach ihrem Text führt die Tür nach 411 (Ankunft 116, 886).
3. `walk-to 111 860` endet bei x 111, y 868 „blockiert“. Eine Figur steht im Ausgang und sagt „Hiya, AAAAA. BBB hat dich gesucht.“

Dahinter (abgeleitet aus PT `res/field/events/events_route_201.json`, `scripts_route_201.s`):
- Auf Route 201 löst bei x 110–113, z 857 und `VAR_FOLLOWER_RIVAL_STATE == 0` die Szene `Route201_CoordEvent_ChooseStarterScene` aus. Der Rivale, Prof. Eibe und „You have no Pokémon“ folgen bis zur Starterwahl.

Ins Gras von Route 201 kommt man vom Spielstand aus also nur über die Story. **Der billigste Beweis ist ein neuer Ort ohne Trigger** (§4.3); die Spec sieht das bereits vor.

### 4.3 Messung: neuer Ort, neue Begegnungsdatei, Mewtu Lv 5

**Platin DE, Kopie `pt-3b3b-enc`.**

1. Szenario-Lauf mit dem Kernel von `main`. Die Datei liegt in `_messung/pt-enc-batch1.yaml`.
   - `apply_header_patch`.
   - `create_area` mit Route 201 Karten 5 und 6, neuem Header 593 (Vorlage 342) und neuer Eventdatei 534.
   - `add_event warp 534 (44, 21)` → 415/0.
   - Treppe 394/0 → 593/0.
   - Ergebnis: Raster 289; Header 593 hat `wild_pokemon` = 140, aus der Vorlage übernommen.
2. `narc_enc.py` hängt an `files/fielddata/encountdata/pl_enc_data.narc` Member **183** an (424 B).
   - Rate 100.
   - 12 Gras-Slots mit Art 150 und Level 5.
   - Schwarm, Tag, Nacht, Radar und Dual-Slot ebenfalls 150.
   - Der NARC-Schreiber erzeugt das Original-NARC byte-identisch (`roundtrip_identical=True`).
3. `unpacked/dynamicHeaders/0593` Offset 0x0E (u16) auf 183 gesetzt.
4. Bauen mit `build: a.nds` (`build-a.yaml`). Ebenso `b.nds` mit Mew (151) Lv 7.
   - Der Build auf `main` packt `encounters` nicht neu, sondern übernimmt das NARC aus `files/`.
   - Geprüft: Das neue NARC liegt byte-gleich in der ROM.

```sh
cd scripts/g4emu
uv run g4emu scenario <scratch>/pt-enc-batch1.yaml --project ../../../Roms/_testkopien/pt-3b3b-enc --out <scratch>/b1
python3 -I narc_enc.py pl_enc_data.vanilla.narc pl_enc_mewtwo.narc pt 183 150 5 100
uv run g4emu scenario <scratch>/build-a.yaml --project ../../../Roms/_testkopien/pt-3b3b-enc --out <scratch>/ba
uv run g4emu run --rom .../pt-3b3b-enc/a.nds --save .../_saves/CPUD-zimmer.sav --out <scratch>/runA \
  continue expect-map 415 walk-to 8 4 walk right 1 expect-map 593 wait 150 where shot ankunft dump-ram ram_field state ankunft
#  -> 593 bei x 44, y 21, nach wait 150 x 45, y 21 (die Figur macht einen Schritt nach rechts)
uv run g4emu run --rom .../a.nds --state <scratch>/runA/ankunft.state --out <scratch>/runA2 \
  walk up 2 where walk up 1 walk down 1 walk up 1 walk down 1 ...
#  -> (45,19) Gras, (45,18), (45,19); der nächste walk meldet "blockiert bei x 45, y 19": Kampf
uv run g4emu run --rom .../a.nds --state <scratch>/runA2/last.state --out <scratch>/runA3 \
  where wait 60 shot t060 wait 120 shot t180 dump-ram ram_t180 wait 120 shot t300 state t300 wait 300 shot t600 dump-ram ram_t600 where
```

**Ergebnis Platin** (`m1.png`):
- Grasübergang, dann „Ein wildes MEWTU erscheint!“, MEWTU Lv5, danach „Los!“.
- Der Kampf kam beim 3. Grasschritt, in Lauf A wie in Lauf B.
- `where` meldet während des ganzen Kampfs weiter Header 593, x 45, y 19. Die Ortsabfrage taugt also nicht zur Kampferkennung.

**HeartGold DE, Kopie `hg-3b3b-enc`.**

1. `hg-batch1.yaml`:
   - Patch.
   - `create_area` mit Route-29-Karte 1, neuem Header 540 (Vorlage 33, `wild_pokemon` = 1 übernommen) und Eventdatei 491.
   - Warp 491/0 bei (17, 16) → 64/0.
   - Treppe 61/0 → 540/0.
2. Route 29 ist Header 33 mit Rasterzellen (18..20, 12) und den Karten 1, 2, 3; Datei 1 (`gs_enc_data.json` Eintrag 1, „R29“).
3. NARC `files/a/0/3/7`: Member **142** angehängt (196 B).
   - Rate 100.
   - 12 Level 5, alle drei Art-Spalten 150, Radio und Land-Schwarm 150.
4. `0540` Byte 0x00 auf 142 gesetzt.

```sh
uv run g4emu run --rom .../hg-3b3b-enc/a.nds --save .../_saves/IPKD-zimmer.sav --out <scratch>/hrunA \
  continue expect-map 64 walk left 1 walk down 1 expect-map 540 wait 150 where shot ankunft dump-ram ram_field state ankunft
#  -> 540 bei x 17, y 16, nach wait 150 x 16, y 16
uv run g4emu run --rom .../a.nds --state <scratch>/hrunA/ankunft.state --out <scratch>/hrunA2 walk up 1 where walk up 1 walk down 1 ...
#  -> (16,15), (16,14); der nächste walk meldet "blockiert bei x 16, y 14": Kampf
```

**Ergebnis HeartGold** (`m6.png`): MEWTU Lv5 mit „Los!“, beim 2. Grasschritt, in Lauf A wie in Lauf B.

### 4.4 Kampfende ohne Team (geprüft)

**Platin** (`m2.png`, `m3.png`, `m4.png`):
1. Nach „Los!“ erscheint ein leerer Platzhalter: „♂ Lv0, KP 0/0“, Sprite-Rauschen.
2. „MEWTU (Wild) übt seine Fähigkeit Erzwinger aus!“, dann das Kampfmenü.
3. `tap 128 170` (FLUCHT) bringt „Flucht gescheitert!“.
4. Mit A weiter: „AAAAA hat kein kampffähiges Pokémon mehr“ und „gerät in Panik und verliert 8 ₽ …“.
5. Danach „Mutter: AAAAA! Willkommen zu Hause.“; `where` = 414 (8, 8).

**HeartGold** (`m7.png`): ebenso. Nach etwa 50 A-Tasten folgt „Vielleicht solltest du dich etwas ausruhen …“, danach Header 63 (6, 8).

**Folgen für das Gate**
- Ein Wildkampf lässt sich **ohne** Team beweisen und beenden. A drücken bis zur Niederlage; danach steht die Figur zu Hause.
- `win-battle` gelingt so nicht. Für Trainer bleibt das Pokémon aus Spec-Schritt 5 nötig.

### 4.5 Kampfbeginn und Gegner-Art im RAM

**Layout `BattleMon`** (PT `include/battle/battle_mon.h` Z. 57; HG `include/battle/battle.h` Z. 212, gleiches Layout):

| Offset | Feld |
|---|---|
| +0x00 | u16 species |
| +0x02..+0x0B | Werte |
| +0x0C | 4 × u16 Attacken |
| +0x14 | IV-Bits |
| +0x18 | s8 statBoosts[8], neutral = 6 |
| +0x20 | int Gewicht |
| +0x24, +0x25 | Typ 1, Typ 2 |
| +0x27 | Fähigkeit |
| +0x34 | u8 level |
| +0x4C | KP |

**Größe 0xC0.** Gemessen: Der Spieler-Datensatz (Art 0, Lv 0) liegt genau 0xC0 vor dem Gegner, in Pt (0x022C5908 / 0x022C59C8) wie in HG (0x022C6038 / 0x022C60F8).

**Differenz-Dump (`cand.py`).**
- Gesucht waren u16 150 in A und 151 in B an derselben Adresse.
- Es fanden sich nur die fünf Treffer der geladenen Begegnungstabelle (0x022A1744 …), die schon im Feld-Abzug stehen.
- Der Kampf-Datensatz liegt in A und B an verschiedenen Adressen:

| Spiel | Lauf A (Mewtu) | Lauf B (Mew) | Abstand |
|---|---|---|---|
| Pt | 0x022C59C8 | 0x022C5930 | 0x98 |
| HG | 0x022C60F8 | 0x022C6118 | 0x20 |

Der Heap verschiebt sich, vermutlich durch die unterschiedlich langen Namen. Eine feste Adresse gibt es also nicht.

**Zeiger.** `ptr.py` fand Zeiger auf den Datensatz nur im Heap:
- Pt: u32 @0x021BFCAC → +0x5000 und @0x021BF820 → +0x5E7C;
- HG: @0x021D1580 → +0x5050 und @0x021D10F0 → +0x5ECC.

In beiden Läufen sind sie gleich. Ob sie bei anderem Team oder anderer Karte stabil bleiben, ist nicht geprüft. Ein statischer Zeiger aus dem arm9-Datenbereich wurde nicht gefunden.

**Signatur (`sig.py`).** Gesucht wird ein Datensatz mit:
- u16 Art an +0x00, im Bereich 1..493;
- acht Bytes `06` an +0x18;
- u8 Level an +0x34, im Bereich 1..100;
- Ausrichtung auf 4 Byte.

| Abzug | Pt | HG |
|---|---|---|
| Feld vor dem Kampf (A und B) | 0 Treffer | 0 Treffer |
| Kampf A, Bild 180 und 600 | genau (150, 5) | genau (150, 5) |
| Kampf B, Bild 180 und 600 | genau (151, 7) | genau (151, 7) |
| nach der Niederlage, wieder zu Hause | 0 Treffer | 0 Treffer |

Die Begegnungstabelle selbst trifft die Signatur nicht. Eine Zwei-Felder-Suche (Art, Level an +0x34) trifft sie dagegen fünfmal; das bestätigt die Spec-Sorge.

**Zeitpunkt** (Abzüge 0/10/30/60/90/120 Bilder nach dem „blockierten“ `walk`):
- Pt: ab Bild 30 sichtbar.
- HG: erst zwischen Bild 120 und 180.
- `expect-battle` braucht deshalb ein Zeitlimit von mindestens etwa 600 Bildern.

**Empfehlung für `expect-battle SPECIES LEVEL`.**
- Alle ~10 Bilder das RAM nach der Signatur durchsuchen und Art und Level vergleichen.
- Hat das Team ein Pokémon, liefert die Signatur auch den Spieler-Datensatz. Es gilt der Treffer mit der erwarteten Art; alternativ Spieler + 0xC0.
- `games.yaml` braucht dann keinen Zeiger, nur die Signatur.
- Die Spec-Variante „Art, Level, erste Attacke“ ist nicht nötig; die Statuswerte-Bytes sind robuster.
- `walk-until-battle` sollte ein „blockiert“ von `walk` als möglichen Kampfbeginn behandeln und dann `expect-battle` pollen.

### 4.6 Billigster Gate-Ablauf (Vorschlag)

**Platin DE**, Kopie aus dem DSPRE-Ordner. Die Platzhalter gelten für ein unverändertes Projekt.

```yaml
game: CPUD
edits:
  - [apply_header_patch, {}]
  - - create_area
    - spec: {name: Begegnung, width: 2, height: 1,
             cells: [{x: 0, y: 0, source_map: 5, clear_buildings: false}, {x: 1, y: 0, source_map: 6, clear_buildings: false}],
             with_headers: true,
             header: {id: 593, area_data_id: null, event_file: New, new_header: {template: 342, internal_name: T_ENC}}}
  - [add_event, {eventFile: 534, kind: warp, x: 44, z: 21}]
  - [update_event, {eventFile: 534, kind: warp, index: 0, patch: {dest_header_id: 415, dest_warp_id: 0}}]
  - [update_event, {eventFile: 394, kind: warp, index: 0, patch: {dest_header_id: 593, dest_warp_id: 0}}]
  # 3b.3b: create_encounter_file (-> 183), set_header 593 wild_pokemon = 183,
  # data_edit: Rate 100, 12 Gras-Slots S_WILD/L_WILD, day_0/1 und night_0/1 = S_WILD
build: out.nds
play:
  - continue
  - expect-map 415
  - walk-to 8 4
  - walk right 1
  - expect-map 593
  - wait 150
  - walk up 2                                # (45, 19), Gras
  - walk-until-battle up 1 --back-and-forth  # neu; gemessen: Kampf beim 3. Grasschritt
  - expect-battle S_WILD L_WILD
  - shot wild
```

Danach `mash a` bis zur Niederlage, dann `expect-map 414`. Mit Pokémon im Team endet der Kampf über Flucht oder Sieg.

**HeartGold DE.** Gleicher Ablauf mit diesen Werten:
- Route-29-Karte 1, Vorlage 33, Header 540, Eventdatei 491, Warp (17, 16).
- Treppe 61/0.
- Spielen: `continue expect-map 64 walk left 1 walk down 1 expect-map 540 wait 150 walk up 1`, dann `walk-until-battle up 1 --back-and-forth`.
- Gemessen: Kampf beim 2. Grasschritt.
- In der Begegnungsdatei alle drei Art-Spalten setzen.

**Warum dieser Ablauf**
- Keine Story-Trigger: Die Eventdatei ist neu und leer.
- Die Init-Scripts der Vorlage (Route 201: nur `OnTransition` mit Grafik-Variablen) haben in der Messung nichts ausgelöst.
- Kein Pokémon nötig.
- Etwa 2 bis 3 Grasschritte bis zum Kampf.
- Die Treppe bleibt der einzige Eingriff am Bestand.

**Route 201 Slot 0 der Spec** (uxie-Prüfung) kann so bleiben; im Spiel ist er vom Spielstand aus nicht erreichbar (§4.2).

## 5. Gras auf kopierter Karte in neuem Header

**Geprüft.** Siehe §4.3:
- Pt Header 593 auf Kopien der Route-201-Karten 5 und 6;
- HG Header 540 auf einer Kopie der Route-29-Karte 1;
- beide mit dem Patch „Dynamische Header“;
- beide lösen Begegnungen aus der neuen Datei aus.

**Was entscheidet** (abgeleitet und durch die Messung gedeckt):

1. **Header-Feld `wild_pokemon`.** Pt u16 bei 0x0E, ≠ 0xFFFF (PT `MapHeader_HasWildEncounters`). HGSS u8 bei 0x00, ≠ 0xFF (HG `src/map_header.c` Z. 98).
   - Sonst prüft das Spiel am Header nichts: weder Ortstyp noch Flags, Kamera oder Gebietsdaten (PT `field_control.c` Z. 543, HG `field_control.c` Z. 471).
   - Mit dem Patch liest das Spiel den Header aus dem NARC-Member. Die alte Grenzprüfung `MapHeader_IDBoundsCheck` (PT `src/map_header.c` Z. 8, `NELEMS(sMapHeaders)`) wird nicht mehr erreicht (siehe `docs/formats/dynamic_headers.md`); die Messung bestätigt das.
2. **Kachelverhalten** in der Berechtigungsschicht der Karte, nicht im Header.
   - Gras 0x02, hohes Gras 0x03 (Bonus), Höhlenboden und weitere mit `TILE_BEHAVIOR_FLAG_ENCOUNTER`.
   - Surfbare Begegnungskacheln wählen die Surf-Rate.
   - `create_area` kopiert die Karte samt Verhalten.
3. **Rate der Datei** > 0 (Pt nach u8-Kürzung).
4. **Was mit der Header-ID verbunden bleibt:** Schwarm (Pt `Swarm_GetMapId`, HG `sSwarmMapLUT`), Roamer, Trophäengarten, Großmoor und in HGSS Kopfnuss und Safari.
   - Neue Header haben davon nichts; das schadet dem Gate nicht.

**Nebenbefund.** `create_area` mit `new_header.template` übernimmt `wild_pokemon` der Vorlage: 593 bekam 140, 540 bekam 1. Ohne „Für diesen Ort abspalten“ teilt der neue Ort die Datei der Vorlage. Eine Änderung wirkt dann auch auf Route 201 bzw. Route 29.

## 6. Neue Begegnungsdatei (Index N)

**Member-Zahlen** (geprüft):
- Pt `pl_enc_data.narc`: 183 Member (BTAF-Zähler 0xB7). Die Begleit-NARCs `d_enc_data.narc` und `p_enc_data.narc` liegen gleich groß daneben; Platin liest nur `pl_enc_data` (PT `src/map_header_data.c` Z. 218–227).
- HG DE `a/0/3/7`: 142 Member.

**Wie das Spiel liest** (abgeleitet):
- Der einzige Leser ist `MapHeaderData_LoadWildEncounters` (Pt) bzw. `WildEncounters_ReadFromNarc` (HG `src/map_events.c` Z. 193).
  - Er liest das **ganze** Member in einen festen Puffer von 424 bzw. 196 Byte (`memset` vorher).
  - Die Member-Zahl kommt zur Laufzeit aus dem BTAF des NARC (PT `src/narc.c` Z. 229–231, HG `src/filesystem.c` Z. 25–26).
  - Die Prüfung `GF_ASSERT(fileCount > memberIndex)` ist in der Verkaufsversion leer (`include/global/assert.h`: nur mit `PM_KEEP_ASSERTS`).
- In arm9 gibt es **keine** Tabelle, die die Zahl der Begegnungsdateien kennt. Die Pfadtabelle `sNarcFileList` nennt nur das NARC.
- Sonst nutzt niemand den Index. Das Pokédex-Gebiet kommt aus eigenen NARCs (`zukan_enc_platinum.narc`, HG-Pokédex-Overlay) und zeigt neue Begegnungen nicht.

**Messung** (geprüft):
- Ein NARC mit 184 (Pt) bzw. 143 (HG) Membern und ein Header mit Index 183 bzw. 142 laden die neue Datei.
- Mewtu bzw. Mew erscheinen (§4.3).
- Das Wachsen des NARC verschiebt die ROM-Dateien; der Build erledigt das.

**Grenzen und Regeln für `create_encounter_file`**
- **Index:** Neue Datei = Index N (Anhängen). Ein Index ≥ Member-Zahl liest hinter dem BTAF Unsinn und muss verboten sein.
- **Pt-Header:** u16, nutzbar 0..65534 (0xFFFF = keine).
- **HGSS-Header:** u8, nutzbar 0..254 (0xFF = keine). Vanilla 142, Platz für 113 weitere. SoulSilver `a/1/3/6` gleich.
- **Member-Größe:** genau 424 bzw. 196 Byte.
  - Länger überschreibt den Puffer.
  - Kürzer lässt den Rest 0; das ist nicht geprüft und sollte trotzdem verboten sein.
- **Pt-Rate** als u32 im Editor: Werte über 100 sind wirkungslos, ab 256 zählt nur das untere Byte. Vorschlag: Eingabe 0..100, sonst Warnung.
- **Neue leere Datei:** Art 0 in den Tag/Nacht-Ersatzslots (Pt) oder in einer Tageszeit-Spalte (HGSS) ist zu den entsprechenden Uhrzeiten wirksam (§3). Die Spec-Warnung „leerer Ersatz-Slot“ muss Tag/Nacht in Pt und die drei Spalten in HGSS einschließen. Was Art 0 im Kampf auslöst, ist offen (§7).

## 7. Offen

1. **Uhrzeit des Emulators.** melonDS DS bekommt keine RTC-Option von `g4emu` (`emu.py` `CORE_OPTIONS`). Vermutlich übernimmt er die Host-Uhr; das ist nicht gemessen. Daran hängt, welche Tag/Nacht-Liste wirkt. Das Gate umgeht es, indem es alle Listen gleich setzt.
2. **Art 0 in einem wirksamen Slot** (leere Ersatzliste): Bad Egg, Glitch oder Absturz. DSPRE warnt mit „glitch Pokémon“. Nicht gemessen.
3. **Stabilität der Heap-Zeiger** aus §4.5 bei anderem Team, anderer Karte oder Trainerkampf. Für das Gate unnötig, wenn `expect-battle` die Signatur nutzt.
4. **HGSS `sub_0205DE98`** (Laufzustände 0x58–0x5B, ohne −20): vermutlich Rennen, nicht bestätigt.
5. **Trainerkampf ohne Team:** Nicht gemessen. Für die Trainer-Schritte des Gates bleibt das Pokémon per Script vorgesehen, mit neutraler Fähigkeit (§1.3).
