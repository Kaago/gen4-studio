# Trainer-Mechanik für 3b.3b: Kampf per Script, Nebentabellen, Grenzen, Gate-Nachweis

Stand 2026-10-24. Recherche für Phase 3b.3b (Trainer-Editor, „Neuer Trainer“, Objekt-Event ↔ Trainer, Gate „zwei neue Trainer kämpfen“).
Grundlage: Spec [2026-10-23-phase-3b3-data-editors-design.md](../../superpowers/specs/2026-10-23-phase-3b3-data-editors-design.md) §11–§15, [dspre-report.md](dspre-report.md) §1.3, [uxie-report.md](uxie-report.md), [task0-3b3a.md](task0-3b3a.md).

**Quellen**
- pret `pokeplatinum`, Commit `be36a9db3d2390a5009adc087c7aef550ce335e7` (2026-10-23), im Text „Pt-pret“.
- pret `pokeheartgold`, Commit `6988158691eb6178cff697cda0428045ddcba5fb` (2026-10-19), im Text „HG-pret“.
- rotom-Befehlsdatenbank im Repo: `crates/g4-kernel/metadata/scrcmd/{platinum,hgss}_v2.json` (Revision 33).
- Die drei Projekte unter `../Roms/*_DSPRE_contents`; sie wurden nur gelesen.
- Zwei frische Testkopien, gebaut mit dem `studio-bridge` aus dem 3b.3a-Worktree (der packt die Trainer-Archive):
  - `../Roms/_testkopien/pt-3b3b-trainer` (Platin DE, CPUD),
  - `../Roms/_testkopien/hg-3b3b-trainer` (HeartGold DE, IPKD).
- Gespielt mit `g4emu` und den Spielständen `CPUD-zimmer.sav` bzw. `IPKD-zimmer.sav`.
- Bilder und RAM-Abzüge liegen lokal unter `target/agent/tm3b3b/` (nicht im Repo).

## Übersicht

| # | Punkt | Ergebnis kurz | Status |
|---|---|---|---|
| 1 | Event-Script ↔ Trainer | Script `3000 + ID − 1` (Doppel-Partner `5000 + ID − 1`). Alle diese Nummern laden **eine** gemeinsame Scriptdatei (Pt `scr_seq` 1114, HGSS 953). Deren Header hat je Trainer-ID einen Eintrag, alle zeigen auf denselben Kampf-Code. Danach folgt **ein** Eintrag für „Trainer hat dich gesehen“. | geprüft (pret, ROM, Emulator) |
| 1 | Minimaler NPC | Kein eigenes Script nötig: Objekt mit Script `3000 + ID − 1`. Ansprechen geht auch mit `trainer_type` 0. Für die Sichtlinie: `trainer_type` 1 und Sichtweite in `data[0]`. | geprüft (Emulator Pt und HGSS) |
| 1 | Vanilla-Beispiel | Route 203 (Eventdatei 330): Teenager Sebastian, Trainer 356, Script 3355, `trainer_type` 1, `data[0]` 5. | geprüft (ROM gegen pret) |
| 1 | Grenze aus dem Script-Header | Pt: Trainer 1–928 haben einen Eintrag; 929 (Script 3928) ist der „gesehen“-Eintrag. **Ein neuer Trainer 929 friert das Spiel ein** (Softlock). HGSS: Trainer 1–739; 740 (Script 3739) ist der „gesehen“-Eintrag. | geprüft (Emulator Pt; HGSS aus ROM und pret) |
| 2 | Default-Attacken | Exakter Ablauf `BoxPokemon_SetDefaultMoves`: Lernset in Reihenfolge, bis Level; bekannte Attacke überspringen; bei vollen Slots Slot 1 raus, neue hinten an. Das ist **nicht** „die letzten vier Einträge“ (Unterschied in 2570 von 50 700 Art × Level). Lernset der Grundform. | geprüft (pret; RAM: Lucario L7, Zirpurze L7, Knakrack L100) |
| 3 | `trtbl`/`trtblofs` | `trtblofs[ID]` ist ein Byte-Offset in `trtbl`. Der Text-Lader sucht von dort vorwärts bis Tabellenende. Die Prüfung „gibt es Text X“ bricht beim ersten fremden Trainer ab. Also gilt: Block je Trainer zusammenhängend, nicht sortiert. Trainer ohne Text haben Offset 0. | geprüft (pret, alle drei ROMs) |
| 3 | Fehlender `trtblofs`-Eintrag | Gelesen wird hinter dem Member. Text leer, zufällig gefunden oder Endlosschleife (Offset nicht durch 4 teilbar). HGSS gemessen: Text vor dem Kampf leer, danach gefunden. | geprüft (Emulator HGSS), Endlosschleife abgeleitet |
| 3 | Anfügen | Einträge `{ID, Typ}` ans Ende von `trtbl`, Nachrichten ans Ende von Archiv 617/728 (Index = Eintrag). `trtblofs` bis zur höchsten ID auffüllen: Lücken 0, neue ID = alte Größe von `trtbl`. | geprüft (Emulator Pt 928, HGSS 738/739) |
| 4 | Flag-Kapazität | Flag = `0x550 + ID` in beiden Spielen. Bis `0x95F` ist in pret nichts belegt; ab `0x960` liegen System-Flags (Pt `FLAG_BAG_ACQUIRED`, HGSS `FLAG_UNK_960`, `FLAG_HAVE_FOLLOWER`). Also IDs bis **1039** ohne Kollision. Die Grenze 734 (HGSS) aus der Spec ist keine: Vanilla 734–737 liegen schon darüber. | Pt-Flag 0x8F0 geprüft (Emulator), Rest geprüft (pret) / abgeleitet |
| 4 | Grenze je Spiel | Ohne Änderung an der Kampf-Scriptdatei: Pt **nur ID 928**, HGSS **738 und 739**. Darüber: Header der Scriptdatei erweitern, die ID direkt nach dem letzten Kampf-Eintrag (Pt 929, HGSS 740) auslassen, höchstens 1039. | abgeleitet (Header-Erweiterung nicht gebaut) |
| 5 | Sprites | Overworld-Grafik = Event-Feld `graphics_id`, nicht die Klasse. Kampfsprite = Klasse: `trfgra`-Member `Klasse × 5 + {0 NCGR, 1 NCLR, 2 NCER, 3 NANR, 4 Scan}` (525 = 105 × 5, 645 = 129 × 5). Tabelle Klasse → OW-Grafik aus Vanilla-Events in §5. `get_object_sprite` liefert für alle Stichproben 32 × 32. | geprüft |
| 6 | Kampf im RAM (Pt DE) | Pt DE, aus 5 Kampf- und 5 Overworld-Abzügen in zwei Heap-Lagen. `u32 @ 0x02101F88` ist außerhalb des Kampfs 0, im Kampf ein Zeiger P. Gegner-BattleMon = P − 0x8DD8 (Art +0, Attacken +0x0C, Level +0x34, Größe 0xC0, Spieler davor). Gegner-Trainer-ID (`u32 trainerIDs[1]`) = P − 0x255F0. | geprüft (CPUD) |
| 6 | Kampf im RAM (HGSS) | BattleMon-Layout gleich (Level +0x34, Größe 0xC0). Gegner bei 0x022C614C in einer Heap-Lage. Kandidat für „Kampf aktiv“: `u32 @ 0x0211DB34`. | IPKD vorläufig, IPKE offen |
| 7 | Namenslänge | Grenze: **8 u16 inklusive Endzeichen**, gespeichert. `{TRAINER_NAME:…}` ist 9-Bit-komprimiert: bis **10 Zeichen**; unkomprimiert 7. Länger: Name nicht kopiert, im Kampf „??????…“ über den Kasten hinaus, Spiel läuft weiter. Bei aktiver Funkverbindung Fehlerbildschirm (pret). | geprüft (Emulator: „Maximilian“ gut, „Bartholomeus“ Müll) |
| – | Gate-Falle | Objekte der Karte, auf der gespeichert wurde, kommen nach `continue` aus dem Spielstand. Neue Figuren im Zimmer erscheinen erst nach Verlassen und Wiederbetreten. | geprüft (Pt), HGSS angenommen |
| – | Brücke | `update_event` kennt `trainer_type` und `data[0..2]` nicht (`EventPatch`, `ObjectDto` in `api.ts`). Für die Messung binär gesetzt. 3b.3b braucht beide Felder. | geprüft |

---

## 1. Trainer-Kampf per Script

### 1.1 Wie das Spiel Script-Nummer und Trainer verbindet

**Platin** (Pt-pret):
- `include/script_manager.h:93–94`: `SCRIPT_ID_OFFSET_SINGLE_BATTLES 3000`, `SCRIPT_ID_OFFSET_DOUBLE_BATTLES 5000`.
- `src/script_manager.c:58–59` (`SCRIPT_RANGE_TABLE`): beide Bereiche laden dieselbe Datei `scripts_battles`.
- `ScriptContext_LoadAndOffsetID` zieht 3000 bzw. 5000 ab. `ScriptContext_JumpToOffsetID` (Z. 412) springt über den Header-Eintrag `Nummer − 3000`.
- `Script_GetTrainerID` (Z. 498–505): `script − 3000 + 1` bzw. `script − 5000 + 1`. 5000+ setzt zusätzlich „Partner“ (`Script_GetTrainerBattlerIndex`).
- `res/field/scripts/scripts_battles.s` enthält den Kampf-Code `Battles_Trainer` (ansprechen) und `Battles_ApproachingTrainer` (gesehen werden).
- Den Header erzeugt `tools/dataproc/src/trainerproc.c:108–118`:
  - `MAX_TRAINERS` (928) Einträge `ScriptEntry Battles_Trainer`,
  - dann **ein** Eintrag `Battles_ApproachingTrainer`.
  - Derselbe Code bricht den Build ab, wenn es mehr Trainer als Trainer-Flags gibt.
- `src/trainer_encounter.c:97` und `:121` starten bei Sichtkontakt das Script `SCRIPT_ID(SINGLE_BATTLES, MAX_TRAINERS)` = **3928**. Die Trainer-ID kommt aus der Script-Nummer des Objekts (`GetTrainerIDFromMapObj`, Z. 348; `apprTrTemplate->trainerID`, Z. 160).

**HeartGold** (HG-pret):
- `include/constants/std_script.h:81–86`: `_std_npc_trainer 3000`, `std_trainer(t) = t − 1 + 3000`, `std_trainer_approach = std_trainer(LAST_TRAINER_INDEX)`, `_std_npc_trainer_2 5000`.
- `include/constants/trainers.h:744`: `LAST_TRAINER_INDEX 740`. Der „gesehen“-Eintrag ist also Script 3739.
- `src/script_manager.c:59–60`: beide Bereiche laden `scr_seq_0953`. `ScriptNumToTrainerNum` (Z. 425–431): `script − 3000 + 1`.
- `files/fielddata/script/scr_seq/scr_seq_0953.s`: 740 `ScrDef`. 000–738 zeigen auf denselben Code (Z. 1487 ff.), 739 ist der „gesehen“-Code (Z. 1708).

**ROM-Messung** (`unpacked/scripts`, Header bis `0xFD13`):

| Projekt | Datei | Einträge | Ziele |
|---|---|---|---|
| CPUD | 1114 | 929 | 928 × `0x0E86` (Kampf), Eintrag 928 → `0x1054` (gesehen) |
| IPKD, IPKE | 0953 | 740 | 739 × `0x0B92` (Kampf), Eintrag 739 → `0x0EC4` (gesehen) |

rotom dekompiliert das als `script script_1 #[1-928]:` und `script script_929 #929:` (Pt) bzw. `#[1-739]` und `#740` (HGSS). Die rotom-Scriptnummer ist also gleich der Trainer-ID.
Mehrfach-Bereiche schreibt rotom schon heute, z. B. `#[13, 15-16]` in Pt 0213. Eine Erweiterung `#[1-928, 930-N]` ist damit ausdrückbar (abgeleitet, nicht gebaut).

**Antwort:** Die Scripts 3000–4999 sind generisch. Es gibt keine Datei je Trainer: Das Spiel rechnet die Trainer-ID aus der Nummer. Alle Nummern landen über den Header einer gemeinsamen Datei im selben Kampf-Code. Die Spec-Formel `script = 3000 + id − 1` stimmt.

### 1.2 Was der NPC braucht

| Feld im Objekt-Event | Wert | Beleg |
|---|---|---|
| `script` | `3000 + ID − 1` (Doppel-Partner `5000 + ID − 1`) | oben |
| `trainer_type` | 1 = normal mit Sichtlinie; 0 reicht zum Ansprechen | Pt-pret `trainer_encounter.c`; Emulator: Trainer 4 mit Typ 0 kämpft beim Ansprechen |
| `data[0]` | Sichtweite in Kacheln | Pt `trainer_encounter.c:192/208` (`MapObject_GetDataAt(obj, 0)`); HGSS `asm/unk_020632B0.s` um Z. 2189 (`MapObject_GetParam(obj, 0)` als Reichweite an `sub_0206439C`); Emulator Pt und HGSS mit Sichtweite 1 |
| `movement_type` / `dir` | Blickrichtung, z. B. Pt 14 `LOOK_NORTH`, 15 Süd, 16 West, 17 Ost (`generated/movement_types.txt`, 0-basiert); HGSS gleich verwendet | Route 203: Sebastian `movement 16`, `dir 2` = `MOVEMENT_TYPE_LOOK_WEST` |
| `graphics_id` | Overworld-Grafik, frei (§5) | |

**Vanilla-Beispiel Route 203** (ROM-Eventdatei 0330 gegen pret `res/field/events/events_route_203.json`):

| Objekt | gfx | Bewegung | `trainer_type` | Script | Richtung | `data[0]` | Trainer-ID |
|---|---|---|---|---|---|---|---|
| 0 Sebastian | 4 Teenager | 16 | 1 | 3355 | 2 | 5 | 356 |
| 1 Michael | 4 | 16 | 1 | 3003 | 2 | 2 | 4 |
| 2 Kaitlin | 6 Göre | 3 | 1 | 3322 | 0 | 1 | 323 |

**Minimales rotom-Script:** keins. Der Kampf-Code in 1114 bzw. 953 erledigt alles:
1. Sound, `LockAll`, `FacePlayer`, `GetTrainerID`.
2. Flag prüfen; ist der Trainer besiegt, Text 2 und Ende.
3. Musik und Text 0.
4. Pt `StartTrainerBattle` bzw. HGSS `TrainerBattle`, dann `CheckWonBattle` bzw. `CheckBattleWon`.
5. Verloren: `BlackOutFromBattle` bzw. `WhiteOut`.
6. Gewonnen: `SetTrainerFlag`, `ReleaseAll`.

Wer trotzdem ein eigenes Kartenscript will (etwa mit Story-Logik), schreibt mit den Namen der Datenbank (Revision 33):

| Schritt | Pt (ID) | HGSS (ID) |
|---|---|---|
| Kampf | `StartTrainerBattle trainer_1, trainer_2` (0xE5) | `TrainerBattle trainer, 0, 0, 0` (0xD5, Makro `asm/macros/script.inc:1295`) |
| gewonnen? | `CheckWonBattle VAR_RESULT` (0xEC) | `CheckBattleWon var` (0xDC) |
| verloren | `BlackOutFromBattle` (0xEB) | `WhiteOut` (0xDB) |
| Flag | `SetTrainerFlag` / `CheckTrainerFlag` (0x23 / 0x25) | `SetTrainerFlag` / `CheckTrainerFlag` (0x24 / 0x26) |
| Texte | `PrintTrainerDialogue id, typ` (0xE6), `GetTrainerMessageTypes` (0xE7) | `TrainerMessage` (0xD6), `GetTrainerMsgParams` (0xD7) |

Ein eigenes Kartenscript verliert die Sichtlinie: Die Trainer-ID kommt dort aus der Script-Nummer des Objekts. Für 3b.3b also immer `3000 + ID − 1` setzen. Das eigene Script ist kein Ersatz.

### 1.3 Die Kollision mit dem „gesehen“-Eintrag (Messung)

Gebaut wurde Pt-Kopie `pt-3b3b-trainer`:
- Trainer 928 (Teenager „Jens“, Lucario L7) und 929 (Göre „Mia“, Guardevoir L9),
- Texte in `trtbl` und Archiv 617, Namen in 618,
- im Zimmer (Header 415) vier Figuren:

| Figur | Platz | Script | Typ |
|---|---|---|---|
| Geber | (6, 4) | Kartenscript 11: `GivePokemon 445, 100, 0, VAR_RESULT` | 0 |
| T928 | (7, 6) | 3927 | 1, Sichtweite 1, Blick Nord |
| Trainer 4 (Vanilla) | (8, 5) | 3003 | 0 |
| T929 | (6, 6) | 3928 | 0 |

| Fall | Erwartet (pret) | Gemessen |
|---|---|---|
| T928 per Sichtlinie (Script 3928 intern) | Kampf | „Jens: H…“, „Eine Herausforderung von Teenager Jens!“, Lucario L7 (`run4/s2`, `s5`, `s6`) |
| T928 nach dem Sieg ansprechen (Script 3927 → Eintrag 927) | Text 2, Flag 0x8F0 gesetzt | „Jens: Du bist stark.“ (`run5/after1`) |
| Trainer 4 mit `trainer_type` 0 ansprechen | Kampf | Vanilla-Text, Kampf gegen Zirpurze L7 (`run6`, `run7`) |
| **T929 ansprechen (Script 3928)** | läuft in `Battles_ApproachingTrainer` ohne Sichtkontakt | **Softlock:** kein Text, kein Kampf. Startmenü öffnet nicht, Figur bewegt sich nicht (`run10/x4`, `run11/menu`, `walk right` „blockiert bei 6, 5“) |

HGSS-Kopie `hg-3b3b-trainer`:
- Trainer 738 (Lucario L7), 739 (Guardevoir L9), 740 (Pikachu L5),
- `trtblofs` auf 741 Einträge aufgefüllt,
- Geber `GiveMon 445, 100, 0, 0, 0, VAR_SPECIAL_RESULT`.

| Fall | Gemessen |
|---|---|
| T738 per Sichtlinie (Script 3737) | „Jens: H…“, „Teenager Jens in den Kampf ge…“, Lucario L7 (`hg2/s2`, `s6`) |
| T739 ansprechen (Script 3738) | „Mia: H…“, Kampf gegen Guardevoir L9 (`hg5/a1`, `hg4/t5`) |
| T740 (Script 3739) | nicht gespielt. Erwartet ist derselbe Softlock wie bei Pt 929 (Eintrag 739 ist der „gesehen“-Code, ROM-Messung oben) |

### 1.4 Folgerung für `create_trainer`

- Pt: ohne Eingriff in Datei 1114 ist genau **ein** neuer Trainer möglich (928). Das Gate „zwei neue Trainer“ legt in Pt sonst 928 und 929 an, und 929 friert ein.
  - Weg A (empfohlen): `create_trainer` erweitert den Header der Kampf-Scriptdatei.
    - Neue Einträge hinter dem „gesehen“-Eintrag zeigen auf den Kampf-Code.
    - In rotom: `#[1-928, 930-N]`, `#929` bleibt der „gesehen“-Code.
    - Die ID direkt danach (Pt 929, HGSS 740) wird nie vergeben. Ein Platzhalter-Datensatz hält die Member-Zahl lückenlos; das Feld „Trainer“ im Event-Inspector schließt sie aus.
  - Weg B: freie Vanilla-IDs wiederverwenden. Nicht empfohlen, Pt hat keine sicher freien.
- HGSS: 738 und 739 gehen ohne Eingriff. Ab 740 gilt dasselbe wie in Pt.
- Die Grenze liest `create_trainer` aus dem Projekt: Einträge im Header der Kampf-Scriptdatei, Ziel des letzten Eintrags. Nicht hart kodieren; Hacks können die Datei schon erweitert haben.

## 2. Default-Attacken (Party ohne `moves`, Flag-Bit 0 = 0)

**Code:**
- Pt-pret `src/trainer_data.c` `TrainerData_BuildParty` ruft je Pokémon `Pokemon_InitWith` (Z. 216 bzw. 241) und setzt die Form erst danach (Z. 218 bzw. 248).
- `Pokemon_InitWith` ruft `BoxPokemon_SetDefaultMoves` (`src/pokemon.c:489`, Rumpf Z. 3819–3843):

```
für jeden Lernset-Eintrag in Dateireihenfolge, solange Level(Eintrag) <= Level(Pokémon):
    BoxPokemon_AddMove: erster leerer Slot; Attacke schon bekannt → überspringen
    alle vier Slots voll → BoxPokemon_ReplaceMove: Slot 2..4 rücken nach vorn, neue Attacke in Slot 4 (volle AP)
```

- HG-pret: dasselbe in `InitBoxMonMoveset` (`src/pokemon.c:3035`), aufgerufen über `CreateMon`. `src/trainer_data.c:334/343` setzt die Form ebenfalls erst danach.
- Mit Bit 0 = 1 überschreibt `Pokemon_SetMoveSlot` bzw. `MonSetMoveInSlot` alle vier Slots. Attacke 0 heißt leerer Slot.

**Folgen für den Editor:**
- „Einschalten füllt mit dem, was das Spiel ohnehin nähme“ stimmt nur mit genau diesem Ablauf.
- „Die letzten vier Einträge bis zum Level“ weicht in **2570** von 50 700 Fällen (507 Arten × Level 1–100, Pt-Lernsets) ab.
- „Die letzten vier verschiedenen Attacken“ weicht noch in **719** Fällen ab, meist in der Reihenfolge. Beispiel Bisaknosp L3: Spiel `[33, 45, 73]`, „letzte verschiedene“ `[33, 73, 45]`.
- Das Spiel nimmt das Lernset der **Grundform**, weil die Form erst nach dem Füllen gesetzt wird. Bei Burmadame- und Deoxys-Formen ohne eigene Attacken greift also das Lernset der Normalform (abgeleitet).
- Weniger als vier Attacken bis zum Level ergeben leere Slots (0).

**Gemessen** (RAM, Pt):

| Pokémon | Learnset-Einträge bis Level | RAM-BattleMon Attacken | Ablauf oben |
|---|---|---|---|
| Lucario L7 (Trainer 928) | 399, 98, 193, 197, 232, 68 | 193, 197, 232, 68 | 193, 197, 232, 68 |
| Zirpurze L7 (Trainer 4) | 45, 117 | 45, 117, 0, 0 | 45, 117 |
| Knakrack L100 (`GivePokemon`, gleiche Funktion) | – | 337, 91, 242, 407 | 337, 91, 242, 407 |

HGSS: Lucario L7 in HGSS ebenfalls 193, 197, 232, 68.

## 3. Kampftexte: `trtbl`, `trtblofs`, fehlende Einträge

**Code** (Pt-pret `src/trainer_data.c:98–151`, HG-pret `src/trainer_data.c:79–123`):
- `offset = trtblofs[ID]` (u16, Byte-Offset in `trtbl`).
- `Trainer_LoadMessage` / `GetTrainerMessageByIdPair` lesen 4-Byte-Einträge `{u16 trainer, u16 typ}` ab `offset`, bis `trainer == ID && typ == gesucht`. Die Nachricht ist `offset / 4` im Archiv 617/728.
  - Diese Schleife bricht bei fremden Trainern **nicht** ab, sie läuft bis `offset == Größe(trtbl)`; dann leerer Text.
- `Trainer_HasMessageType` / `TrainerMessageWithIdPairExists` brechen beim ersten fremden Trainer ab.
  - Das sind die Texte im Kampf: erster Treffer, halbe KP, letztes Pokémon. Pt `src/battle/battle_lib.c:1897–1956`.

**ROM-Messung:**

| | CPUD | IPKD | IPKE |
|---|---|---|---|
| `trtbl` | 2497 Einträge | 1717 | 1717 |
| `trtblofs` | 928 | 735 | 735 |
| Trainer mit Texten, alle zusammenhängend | 836, ja | 636, ja | 636, ja |
| `trtblofs[ID]` = erster eigener Eintrag × 4 | alle 836 | alle 636 | alle 636 |
| Trainer ohne Text: `trtblofs` | 92 × 0 | 99 × 0 | 99 × 0 |
| sortiert nach Trainer | nein (247, 246, 244, 245, 1, …) | nein (495, 496, 497, 1, 266, …) | nein |
| hinter `trtblofs` im NARC | Dateiende | 2 Byte `FF FF`, dann Dateiende | dto. |

**Antworten:**
- Sortiert muss `trtbl` nicht sein. Nötig ist: die Einträge eines Trainers stehen zusammenhängend ab `trtblofs[ID]`.
- Trainer ohne Text bekommen Offset 0. Der Lader durchsucht dann alles und liefert leer.
- HGSS-Trainer 735–737 (Rivale als Partner) haben keinen Eintrag. Der Lese-Offset liegt hinter dem Member: 735 liest das Füllwort `0xFFFF`, 736/737 lesen hinter dem Dateiende (Ergebnis zufällig).
  - Ist der Startwert nicht durch 4 teilbar, trifft die u16-Schleife `Größe(trtbl)` nie: Endlosschleife (abgeleitet). Bei `0xFFFF` genügt dafür, dass er zufällig keinen Eintrag trifft.
- **Gemessen** mit HGSS-Variante `out-noofs.nds` (`trtbl` erweitert, `trtblofs` auf 735 belassen): Trainer 738 zeigte vor dem Kampf einen **leeren** Textkasten (`hg6/s2`). Der Kampf lief, und nach dem Sieg fand der Lader „Jens: Du bist st…“ trotzdem (`hg7/end`).
  - Das Verhalten ist also undefiniert. Die Spec-Invariante „`trtblofs` ≥ Trainerzahl, sonst Kampftexte gesperrt“ ist richtig und muss vor dem ersten neuen HGSS-Trainer greifen.
- **Anfügen** (gemessen Pt 928, HGSS 738/739):
  - `{ID, 0}`, `{ID, 1}`, `{ID, 2}` ans Ende von `trtbl`.
  - Drei Nachrichten ans Ende von 617/728; Index = Eintragsindex, die Invariante `len(trtbl)/4 == Nachrichten` bleibt.
  - `trtblofs` bis Index ID auffüllen: HGSS 735–737 mit 0, ID mit der alten Größe von `trtbl`.
- **Textvorlage:** 382 von 383 HGSS-Texten „vor dem Kampf“ (Typ 0) und 412 von 516 in Pt enden auf `\r`. Ohne `\r` schloss der HGSS-Kasten beim Ansprechen sofort (`hg5/a1` zeigt „Mia: H…“, 80 Bilder später Kampf). Die Vorlage für Typ 0 endet also auf `\r`.
- Typen (`generated/trainer_message_types.txt`): 0 vor dem Kampf, 1 Niederlage (im Kampf), 2 danach, 3–10 Doppelkampf, 13–16 Kampfereignisse, 17–19 Revanche, HGSS 20 Sieg. Für einen Einzel-Trainer reichen 0, 1, 2.

## 4. Trainer-Flag-Kapazität

| | Pt | HGSS |
|---|---|---|
| Flag | `TRAINER_DEFEATED_FLAGS_START + ID` = `0x550 + ID` (Pt-pret `script_manager.c:517–531`) | `TRAINER_FLAG_BASE + ID` = `0x550 + ID` (HG-pret `flags.h:1394`, `script_manager.c:441–454`) |
| benannter Trainerblock | bis `0x8EF` (`TRAINER_DEFEATED_FLAGS_END`, `generated/vars_flags.txt:2294`) | `FLAG_UNK_82E` … ab Z. 1397: nur Platzhalter |
| danach bis `0x95F` | `FLAG_UNK_0x08F0` … `FLAG_UNK_0x095F`, 112 Namen, nirgends in `res/`, `src/`, `include/`, `asm/` verwendet; kein `*_START`/`*_END`-Bereich deckt sie | `FLAG_UNK_82E` … `FLAG_UNK_95F`, keiner außerhalb von `flags.h` verwendet |
| erstes belegtes Flag | `0x960` `FLAG_BAG_ACQUIRED` (Z. 2407), dann `FLAG_HAS_PARTNER`, `FLAG_STRENGTH_ACTIVE`, … | `0x960` `FLAG_UNK_960` (3 Verwendungen), `0x961` `FLAG_HAVE_FOLLOWER` |
| `NUM_FLAGS` | 2912 (`include/vars_flags.h:8`) | 2912 (`flags.h:2223`) |
| höchste ID ohne Kollision | **1039** | **1039** |
| Vanilla | 0–927, Flags bis 0x8EF | 0–737, Flags bis 0x831; 734–737 liegen schon in den Platzhaltern |

- Die Spec-Zahl „HGSS 734“ ist keine Grenze, sondern nur der Anfang der unbenannten Flags. `arm9_tables.yaml` im 3b.3a-Worktree führt `trainer_flag_capacity: 928 / 734`. Das sollte die Flag-Grenze 1040 werden, und die bindende Grenze die aus dem Kampf-Script-Header (§1.4).
- **Bei ID ≥ 1040:** Ein Sieg setzt ein System-Flag.
  - Pt: 1040 → „Beutel erhalten“, 1041 → „hat Partner“, 1042 → „Stärke aktiv“, …
  - HGSS: 1041 → `FLAG_HAVE_FOLLOWER`.
  - Ist das System-Flag schon gesetzt, gilt der Trainer als besiegt: Er sieht nicht und gibt nur Text 2.
- Ab ID ≥ 1552 liegt der Index hinter `NUM_FLAGS`. `VarsFlags_SetFlag` hat ein `GF_ASSERT` (Pt `src/vars_flags.c:57`), das im Einzelspiel nichts tut; dann wird hinter dem Flag-Feld geschrieben (abgeleitet).
- **Gemessen:** Trainer 928 (Flag 0x8F0) setzt sein Flag und liest es: Text 2 nach dem Sieg. Zimmer und Spielstart blieben unauffällig.

## 5. Sprites: Klasse, Kampfsprite, Overworld-Grafik

- **Overworld:** nur das Event-Feld `graphics_id` (u16 an Offset 2 des 32-Byte-Objekts). Weder `trdata` noch die Klasse kennen eine OW-Grafik. Die Vorschau im Editor nimmt `get_object_sprite(graphics_id)` aus 3b.1.
  - Geprüft für die Stichproben unten: alle 14 Pt-IDs und 14 HGSS-IDs liefern 32 × 32.
- **Kampfsprite:** Klasse → `trfgra`. Pt-pret `src/pokemon.c:246–252, 3456–3465`: Member `Klasse × 5 + {0 Kacheln, 1 Palette, 2 Zellen, 3 Animation, 4 Scan}`.
  - Member-Zahlen 525 (Pt, 105 Klassen) und 645 (HGSS, 129 Klassen) passen.
  - Rückseiten (`trbgra`) laufen über eine eigene Tabelle (`SpriteSystem_TrainerClassBackSpriteIndex`). Laut Spec bleiben Kampfsprites Platzhalter bis 3d.
- Byte 2 von `trdata` („sprite“) ist in Vanilla immer 0 und wird beim Kampf nicht gelesen. Es bleibt erhalten.

**Klasse → OW-Grafik aus den Vanilla-Events** (alle Objekte mit Script 3000–6999; Klasse aus `trainerProperties[ID]`; Zahl = Anzahl Objekte).

Platin DE:

| Klasse | Name (pret) | OW-Grafik (pret-Name) | n |
|---|---|---|---|
| 2 | YOUNGSTER (Teenager) | 4 `YOUNGSTER` | 12 |
| 3 | LASS (Göre) | 6 `LASS` | 8 |
| 4 | CAMPER | 52 `CAMPER` | 8 |
| 5 | PICNICKER | 53 `PICNICKER` | 8 |
| 6 | BUG_CATCHER | 5 `BUG_CATCHER` | 4 |
| 9 | HIKER (Wanderer) | 20 `HIKER` | 13 |
| 11 | FISHERMAN | 54 `FISHERMAN` | 17 |
| 24 / 25 | ACE_TRAINER m / w | 11 / 14 `ACE_TRAINER_M/F` | 23 / 23 |
| 29 / 30 / 39 / 40 | Drachenprofi, Vogelfänger, Ranger m/w | 11 bzw. 14 (Ass-Trainer-Grafik) | 11 / 8 / 5 / 4 |
| 73 / 89 | Galaktik-Rüpel m / w | 124 / 125 `GRUNT_M/F` | 21 / 13 |

HeartGold DE:

| Klasse | Name (pret) | OW-Grafik (`SPRITE_…`) | n |
|---|---|---|---|
| 2 | YOUNGSTER | 317 `GSBOY2` | 11 |
| 3 | LASS | 319 `GSGIRL1` | 9 |
| 4 | CAMPER | 345 `CAMPBOY_2` | 15 |
| 5 | PICNICKER | 346 `PICNICGIRL_2` | 17 |
| 6 | BUG_CATCHER | 318 `GSBOY3` | 15 |
| 9 | HIKER | 333 `MOUNT_2` | 20 |
| 11 | FISHERMAN | 347 `FISHING_2` | 20 |
| 24 / 25 | ACE_TRAINER m / w | 324 `GSMAN3` / 327 `GSWOMAN3` | 16 / 15 |
| 55 / 62 | TEAM_ROCKET m / w | 222 `ROCKETM` / 221 `ROCKETW` | 20 / 4 |
| 69 | BIRD_KEEPER_GS | 323 `GSMAN2` | 22 |

Die volle Liste (alle Klassen mit Events) entsteht mit demselben Vorgehen: Eventdateien lesen, Objekte mit Script 3000–6999, Klasse aus `trainerProperties[Script − 2999]` bzw. `[Script − 4999]`. Vorschlag für den Editor: Beim Platzieren schlägt er die häufigste Grafik der Klasse aus den Events des Projekts vor; die Zuordnung ist Datenlage, keine Regel. Ein Klassen-Default aus einer festen Tabelle wäre für Hacks falsch.

## 6. Gate-Nachweis mit `g4emu`

### 6.1 Messprotokoll (Platin DE, CPUD)

Alle Läufe aus `scripts/g4emu`, `--out ../../target/agent/tm3b3b/<lauf>`. Die ROM ist `pt-3b3b-trainer/out.nds` bzw. `out2.nds` (Namen geändert, sonst gleich).

| Lauf | Schritte | Ergebnis |
|---|---|---|
| vanilla | `continue where shot` (Original-ROM) | 415, x 7, y 4 |
| run1 | `continue press left:4 press a …` | Neue Figuren **nicht sichtbar**: Die Objekte kommen aus dem Spielstand |
| run2 | `continue walk-to 8 4 walk right 1 expect-map 414 wait 200 press left:16 expect-map 415 wait 200` | nach Wiederbetreten alle Figuren sichtbar; Ankunft 415 x 8, y 4. Die Treppe in 414 auf (10, 3) hat Verhalten 0x5F, Ausgang nach links |
| run3 | `walk left 1 press a:4 …` (Geber) | Pokémon erhalten (später Party mit 1 Ball) |
| run4 | `dump-ram ow walk down 1 wait 90 shot s1 … dump-ram b1 … dump-ram b2` | Sichtlinie: „!“, „Jens: H…“, Herausforderung, Lucario L7 |
| run5 | 45 × `press a:4 wait 40` | Sieg, zurück auf 415 (7, 5); das letzte A spricht T928 an: „Jens: Du bist stark.“ |
| run6/7 | `press right:8 press a:4 …`, `dump-ram b4` | Trainer 4: Kampf gegen Zirpurze L7 |
| run8/9 | A-Folge | zweites Pokémon (Zubat), Sieg, Overworld |
| run10/11 | `walk left 1 press down:8 press a:4 …` | T929: Softlock (§1.3) |
| run12–14 | `out2.nds`: Namen 928 = „Maximilian“, 4 = „Bartholomeus“ | §7 |

**RAM-Auswertung** (Abzüge 4 MiB ab 0x02000000):

| Größe | Fundstelle | Belege |
|---|---|---|
| Kampf aktiv | `u32 @ 0x02101F88`: 0 in `ow0` (Vanilla), `ow`, `b3` (Dialog vor dem Kampf), `after1`, `after2`; ≠ 0 in `b1`, `b2`, `b4`, `nb1`, `kb` | 5 × 0, 5 × Zeiger |
| Zeiger P | 0x022CE7AC (run4–7), 0x022CE708 (run12–14): die Lage wandert mit dem Heap | zwei Lagen |
| Gegner-BattleMon (Kampfplatz 1) | P − 0x8DD8: 0x022C59D4 bzw. 0x022C5930 | in beiden Lagen gleich weit |
| Spieler-BattleMon (Kampfplatz 0) | P − 0x8E98, also Gegner − 0xC0 | Knakrack L100 |
| Felder (pret `include/battle/battle_mon.h:57 ff.`) | +0x00 Art, +0x02..0x0B Werte, +0x0C 4 × Attacke, +0x24/25 Typen, +0x27 Fähigkeit, +0x34 Level; Größe 0xC0 | Lucario: Typen 1/8, Fähigkeit 80 |
| Gegner-Trainer-ID | `u32` P − 0x255F0 = `FieldBattleDTO.trainerIDs[1]` (pret `include/field_battle_data_transfer.h:45 ff.`: battleType, parties[4], resultMask, trainerIDs[4]) | 928, 4, 928, 4 in 4 Abzügen |
| schon beim Herausforderungstext gefüllt | BattleMon und Trainer-ID stehen in `b1` (Bild „Eine Herausforderung …“) | |
| nach dem Kampf | 0x02101F88 = 0; der BattleMon-Speicher behält alte Werte (`after1`: noch 448) | Art allein ist kein Kampfbeweis |
| `where` im Kampf | liefert weiter Header und Position: kein Kampfanzeiger | run8 |

`find-u16 448 b2.bin 401 b4.bin` findet 0x022C59D4, aber keinen statischen Zeiger in 0x2000 Abstand. Erst die Differenz der Overworld- und Kampfabzüge im statischen Bereich 0x020F0000–0x02120000 ergab 0x02101F88. Es liegt 0xA8 hinter dem Location-Zeiger 0x02101EE0.

**HeartGold DE (IPKD), vorläufig:**
- Gegner-BattleMon 0x022C614C, Spieler 0x022C608C (−0xC0). Gleiches Layout, Level +0x34.
- Nur eine Heap-Lage gemessen (zwei Kämpfe, beide gleich).
- Kampf-aktiv-Kandidaten (0 in drei Overworld-Abzügen, auch nach dem Kampf; ≠ 0 in fünf Kampfabzügen inklusive Preisgeld-Bild): `0x0211DB34`, `0x0211E894`, `0x0211EAE4` u. a.
- `0x0211C674` wird schon beim Preisgeld-Bild 0: ungeeignet.
- Abstand P → BattleMon ist mit einer Lage nicht belegbar.
- IPKE: nicht gemessen. Vermutung: wie beim Location-Zeiger +0x20, also `0x0211DB54`. Offen.

### 6.2 Billigster Nachweis

**Falle:** Figuren auf der Karte des Spielstands (Pt 415, HGSS 64) erscheinen nach `continue` nicht. Spec-Gate Schritt (5) „im Zimmer eine Figur“ braucht deshalb ein Wiederbetreten oder einen anderen Ort. Den Geber also ins Erdgeschoss (Pt 414, HGSS 63) stellen oder einmal Treppe runter und rauf.

**Pt, ohne neuen Ort:**
1. Header 414: Geber mit `GivePokemon` (beim Durchgehen ansprechen).
2. Header 411: NPC mit Script `3000 + ID − 1`, `trainer_type` 1, Sichtweite 2, Blick Nord, zwei Kacheln südlich der Haustür (Tür 116, 885, Ankunft 116, 886).
3. Ablauf:
```
continue  walk-to 8 4  walk right 1  expect-map 414  wait 150
walk-to <vor Geber>  press <Richtung>:8  press a:4  mash a 200
walk-to 6 10  walk down 1  expect-map 411  wait 150
expect-battle --trainer <ID> --timeout 900      # die Sichtlinie greift beim Austreten
expect-enemy-species <Art> <Level>
shot trainer1  win-battle
```
Zweiter Trainer daneben ebenso; in Pt mit ID 930 (nach Header-Erweiterung) oder in HGSS 739. Beim Ansprechen statt Sichtlinie: `press a:4`, `mash a 120`, dann `expect-battle`.

Alles ist mit den gemessenen Bausteinen belegt: Treppe, Geber, Sichtlinie bei Abstand 1, Ansprechen, Texte, RAM-Erkennung. Den Gang nach 411 hat das 3a-Gate schon.

### 6.3 Vorschlag: neue Schritte in `scripts/g4emu`

`games.yaml` je Spielcode:

```yaml
CPUD:
  battle:
    active: 0x02101F88     # u32; 0 = kein Kampf, sonst Zeiger P (Lage wandert mit dem Heap)
    enemy_mon: -0x8DD8     # P + Wert = BattleMon Kampfplatz 1 (Gegner, erstes Pokémon)
    trainer_id: -0x255F0   # P + Wert = u32 FieldBattleDTO.trainerIDs[1]
    mon: { size: 0xC0, species: 0x00, moves: 0x0C, level: 0x34 }
IPKD:
  battle:
    active: 0x0211DB34     # vorläufig (eine Heap-Lage)
    enemy_mon: null        # nicht kalibriert -> Signatursuche
    trainer_id: null
    mon: { size: 0xC0, species: 0x00, moves: 0x0C, level: 0x34 }
IPKE:
  battle: null             # offen; Vermutung active 0x0211DB54
```

**`expect-battle [--trainer ID] [--timeout N=1800]`**
- Pollt alle 4 Bilder `u32 @ active`. Erfolg, sobald ≠ 0.
- Mit `--trainer`: zusätzlich `u32 @ P + trainer_id` == ID (Wildkampf: 0).
- Ausgabe `{"step":"expect-battle","ok":true,"frame":…,"trainer":928,"enemy":{"species":448,"level":7,"moves":[193,197,232,68],"addr":"0x022C59D4","via":"pointer"}}`.
- Zeitüberschreitung: Exit 2 mit dem letzten gelesenen Wert.

**`expect-enemy-species SPECIES [LEVEL] [--move M] [--timeout N=600]`**
- Setzt einen aktiven Kampf voraus und pollt, bis der Gegner-BattleMon die Art (und Level und erste Attacke) zeigt.
- Adresse zuerst über `P + enemy_mon`, mit Plausibilitätsprüfung: Art 1..507, Level 1..100, BattleMon davor (−0xC0) mit Art 1..507 und Level 1..100.
- Scheitert sie oder fehlt die Kalibrierung, sucht der Schritt in 0x02200000–0x02400000 nach einem Paar benachbarter BattleMons (Abstand 0xC0) mit Art und Level an +0 / +0x34. `via: "scan"` in der Ausgabe.
- Das ist die Spec-Signatur mit drei Feldern, verschärft durch den Nachbarn.
- Die erste Attacke folgt aus §2 und ist ein guter dritter Wert, wenn der Editor sie nicht vorgibt.

**Kalibrierung je Spielcode** (Ablauf wie hier, zwei Heap-Lagen):
1. Zwei Kämpfe gegen bekannte Arten, je `dump-ram` vor dem Kampf, beim Herausforderungstext und danach in der Overworld.
2. Statisches Wort suchen, das in allen Overworld-Abzügen 0 und in allen Kampfabzügen ≠ 0 ist.
3. BattleMon per `find-u16` (Art im ersten, andere Art im zweiten Kampf) und Level an +0x34.
4. Abstand P → BattleMon und P → `trainerIDs[1]` in **zwei** Lagen gleich? Dann eintragen, sonst `null` (Signatursuche).
5. Zwei Lagen entstehen schon durch einen anderen Weg zum Kampf (hier: einmal mit, einmal ohne Fernsehtext im Zimmer).

`win-battle`: In der Messung reichten 45 × `press a:4 wait 40` für einen Kampf mit einem Gegner-Pokémon (L100 gegen L7). Bei zwei Gegner-Pokémon waren es ~90.
- Ende erkennen: `active` = 0 und `where` lesbar.
- Danach `press b` zweimal. Das letzte A spricht sonst den Trainer erneut an (run5).

## 7. Trainer-Name: Länge und `{TRAINER_NAME:…}`

**Code:**
- Pt-pret `src/trainer_data.c:42–44`: `String_ToChars(name, dto->trainer[i].name, TRAINER_NAME_LEN + 1)`, `TRAINER_NAME_LEN 7` (`include/constants/string.h:5`).
- `String_ToChars` (`src/string_gf.c:372–382`) kopiert nur bei `size + 1 ≤ 8`, sonst `GF_ASSERT(FALSE)`.
- HG-pret: `CopyStringToU16Array(…, PLAYER_NAME_LENGTH + 1)` (`src/trainer_data.c:36`, `src/pm_string.c:358–366`), gleich.
- Kopiert wird die **gespeicherte** Form, also inklusive `0xF100` und gepackter 9-Bit-Zeichen. Entpackt wird erst beim Anzeigen (`String_ConcatTrainerName`, `string_gf.c:426 ff.`).
- `GF_ASSERT` ist im Retail-Build aktiv (`-DPM_KEEP_ASSERTS` in beiden `meson.build`). `ErrorHandling_AssertFail` / `GF_AssertFail` setzen das Spiel aber nur bei aktiver Funkverbindung zurück (`src/error_handling.c`). Im Einzelspiel geht es weiter, der Name-Puffer bleibt uninitialisiert (Stack).

**ROM-Messung** (gespeicherte Länge inklusive `0xFFFF`):

| | Pt 618 | HG 729 (DE) |
|---|---|---|
| komprimiert (`0xF100`) | 885 | 728 |
| unkomprimiert | 43 (z. B. „Perenus“, „Distmar“: 7 Zeichen = 8 Einheiten) | 10 |
| größte Länge | 8 Einheiten | 8 |
| längste Namen | „Inga & Don“, „Vera & Tim“: 10 Zeichen, 8 Einheiten | „Thom & Kae“ u. a.: 10 Zeichen, 8 Einheiten |

Formel komprimiert: `2 + ⌈9 · n / 15⌉` Einheiten (Marke, Daten, `0xFFFF`; das `0xFFFF` dient auch als 9-Bit-Ende). ≤ 8 ⇒ **n ≤ 10**. Unkomprimiert `n + 1` ≤ 8 ⇒ n ≤ 7.
chatot 0.8.1 schreibt `{TRAINER_NAME:…}` komprimiert (`encode.rs:812 ff.`). Dabei maskiert es jeden Zeichencode mit `& 0x1FF` ohne Fehler: Zeichen über 0x1FF gehen kaputt.

**Gemessen** (Pt, `out2.nds`):

| Name | Einheiten | Anzeige |
|---|---|---|
| „Jens“ (Trainer 928, `out.nds`) | 5 | „Eine Herausforderung von Teenager Jens!“ |
| „Maximilian“ (928) | 8 | „… Teenager Maximilian!“ (`run12/n1`) |
| „Bartholomeus“ (Trainer 4) | 10 | „… Teenager ????????????????????????“ über den Kasten hinaus, danach „…A“; der Kampf läuft weiter (`run14/k3`, `k4`, `k5`) |

**Folgerung:**
- Die harte Grenze im Editor ist „gespeichert ≤ 8 Einheiten“: komprimiert bis 10 Zeichen ohne Zeichen über 0x1FF, sonst 7.
- Der Text kann „Höchstens 10 Zeichen (7 mit Sonderzeichen); längere Namen brauchen einen arm9-Patch“ heißen.
- `trainer_name_max: 7` in `arm9_tables.yaml` wird zu einer Einheiten-Grenze 8. Das arm9-Byte ist die Konstante 8 in `Trainer_Encounter`, die DSPRE als `trainerNameLenOffset` patcht.
- Längere Namen verderben die Anzeige: Müll im Kampf, bei Funkverbindung Rücksetzen. Die Spec-Sperre bleibt richtig.

## 8. Folgerungen für Spec und Plan 3b.3b

1. **`create_trainer`:**
   - Erlaubte IDs aus dem Header der Kampf-Scriptdatei lesen (Pt 1114, HGSS 953).
   - Die ID gleich dem „gesehen“-Eintrag + 1 nie vergeben (Pt 929, HGSS 740).
   - Für mehr IDs den Header erweitern; Flag-Grenze 1039.
   - Pt-Gate: zwei neue Trainer heißt 928 und 930 (mit Erweiterung). Sonst nur einer in Pt und der zweite in HGSS.
2. **`arm9_tables.yaml`:**
   - `trainer_flag_capacity` auf 1040 (Pt und HGSS), Begründung §4.
   - `trainer_name_max` als Einheiten-Grenze 8 (§7).
3. **Nebentabellen** beim Anlegen im selben Changeset:
   - `trtblofs` bis zur ID auffüllen (Lücken 0),
   - Namensbank bis zur ID auffüllen,
   - `trtbl` und das Archiv um denselben Betrag wachsen lassen.
4. **Event-Inspector und Brücke:**
   - `ObjectDto` und `EventPatch` brauchen `trainer_type` und `data` (Sichtweite `data[0]`). Ohne sie kann 3b.3b keine Trainer-Figur anlegen.
   - Das Feld „Trainer“ leitet die ID aus `script − 2999` ab; 5000+ als „Partner“ zeigen.
5. **Default-Attacken:** genau der Ablauf aus §2 (Duplikate überspringen, Grundform-Lernset), nicht „die letzten vier“.
6. **Textvorlage** Typ 0 mit `\r` am Ende (§3).
7. **Gate:**
   - Geber nicht auf die Karte des Spielstands, oder vorher wiederbetreten.
   - `expect-battle` und `expect-enemy-species` wie §6.3.
   - Pt-Kalibrierung liegt vor; HGSS braucht eine zweite Heap-Lage.

## 9. Offen

| Punkt | Warum offen | Weg |
|---|---|---|
| Erweiterter Kampf-Script-Header (Pt 930+, HGSS 741+) | nicht gebaut und nicht gespielt | in Task 0 oder 1 von 3b.3b: rotom `#[1-928, 930-931]`, Trainer 930 per Sicht und per Ansprechen |
| HGSS 740 (Script 3739) | nicht gespielt | Softlock erwartet wie Pt 929 (ROM-Header gemessen) |
| HGSS-Kalibrierung (Zeiger-Abstände, IPKD und IPKE) | eine Heap-Lage, IPKE gar nicht | §6.3 Schritte 1–5 |
| Endlosschleife bei fehlendem `trtblofs` | nur leerer bzw. zufällig gefundener Text beobachtet | nicht nötig, wenn `create_trainer` die Invariante hält |
| Figur aus dem Spielstand (HGSS) | nur der Weg mit Wiederbetreten gespielt | angenommen wie Pt |
| Doppel-Trainer (5000+, `battle_type` 2) | nicht untersucht | eigener Gate-Punkt, falls 3b.3b Doppelkämpfe anbietet |
| Zeichen über 0x1FF in Trainernamen | chatot maskiert still | Editor-Prüfung: bei solchen Zeichen unkomprimiert schreiben (7er-Grenze) oder ablehnen |
