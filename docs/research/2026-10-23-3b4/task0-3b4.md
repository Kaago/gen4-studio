# Task 0 (3b.4): Grenzen, Vanilla-Zählungen, Import-Proben und Snapshot-Kosten

Stand 2026-10-24. Klärt die offenen Prüfungen aus Spec 3b.4 §13 ([Spec](../../superpowers/specs/2026-10-23-phase-3b4-import-checks-design.md)), aufbauend auf [checks-report.md](checks-report.md) und [import-report.md](import-report.md).

**Daten:** nur lesend an `Roms/*_DSPRE_contents`: Platin DE (CPUD, 593 Header), HeartGold DE (IPKD, 540 Header) und HeartGold US (IPKE, 540 Header). Script-Prüfungen laufen auf den übernommenen Kopien `_testkopien/pt-3c1` und `_testkopien/hg-3b2-3c`. Bei beiden sind `unpacked/scripts`, `unpacked/textArchives` und `arm9.bin` byte-gleich mit Vanilla (`diff -rq`, `cmp`), alle Quellen stammen aus einem Übernahme-Lauf. Für HeartGold US gibt es keine übernommene Kopie, dort fehlen die Script-Prüfungen („nicht gemessen“). Für den Emulator-Versuch (Punkt 5) gibt es drei neue Kopien unter `_testkopien/`, angelegt mit `cp -R` auf ein nicht vorhandenes Ziel (Liste in §5).

**pret:** `pret/pokeplatinum` `be36a9db3d2390a5009adc087c7aef550ce335e7` und `pret/pokeheartgold` `6988158691eb6178cff697cda0428045ddcba5fb`. Beide waren am 2026-10-24 HEAD laut GitHub-API und liegen als flacher Klon im Scratch. Alle Zeilennummern beziehen sich auf diese Commits.

**Werkzeuge:**
- Python 3 mit `-I`. Skripte und Ausgaben liegen nur im Scratch dieser Sitzung (`…/scratchpad/t4`, `t5`, `t6`, `t7`), nicht im Repo.
- `tools/apicula/bin/apicula info`.
- `g4k build` und g4emu (main, melonDS DS 1.4.0).
- Ein temporärer Rust-Test `crates/g4-kernel/tests/t0_snapshot_timing.rs`. Er wurde nach der Messung gelöscht, nicht committet, und der Debug-Build ist wiederhergestellt.

**Header-Layout:**
- Platin: uxie 0.9.7 `MapHeaderPtRaw`.
- HGSS: `MapHeaderHGSSRaw` (`map_header/binary.rs:84–100`), also `wild u8, area u8, coords u16, matrix u16 @4, script @6, level @8, text @10, …, event @16`.
- Achtung: `baseline.py` aus der Recherche las HGSS ab Offset 6 (falsch, „see below“) und wurde dort nur für Platin benutzt. Die Zählungen hier nutzen das korrigierte Layout.
- Header-Zahl = `mapname.bin` / 16, wie `Project::mapname_count`.

## Übersicht

| # | Frage | Ergebnis | Status |
|---|---|---|---|
| 1 | Breite des Feld-Textfensters | **216 px** (27 Kacheln × 8, Druck ab x 0), Pt und HGSS → `dialog_line_px: 216, verified: true` | geprüft (pret) |
| 2 | Warp-Ziel 0x0FFF | `MAP_HEADER_DYNAMIC`. Ausgelöst wird es durch **Ziel-Warp 0x100**: Das Ziel kommt aus dem Spielstand (`special`/`dynamicWarp`, gesetzt von `SetSpecialLocation`/`SetDynamicWarp`). Vanilla: Pt 6, HG DE/US je 9 Warps, alle genau (0x0FFF, 256) | geprüft (pret + ROM) |
| 3 | HGSS-Puffer Modell / BDHC | **0xF000 / 0x9000** wie Platin (asm in pokeheartgold). Größtes Vanilla-Modell Pt 60 556 B, HG 61 420 B; größtes BDHC Pt 21 468 B, HG 10 794 B | geprüft (pret-asm + ROM) |
| 4 | Vanilla-Zählungen HGSS | Alle Fehler-Kandidaten 0 auf allen drei Projekten (HG US ohne Script-Prüfungen). `matrix.header-elsewhere` 14/13/13, alle mit Header-Zelle 0 (mit Ausnahme 0/0/0). Tabelle §4 | gemessen |
| 5 | Trainer ohne Party | **Kein Absturz** (Pt und HG DE): Kampf gegen ein unsichtbares „!!!!!!!!!“ Lv 0, „Aber da ist gar kein Ziel…“, Sieg mit Preisgeld, Spiel läuft weiter | gemessen (g4emu) |
| 6 | Material-Leser | Der Python-Leser liest alle Modelle (666/676/676), apicula stimmt überein. Es **fehlen schon in Vanilla Namen**: Pt 14 Karten, HG DE/US je 7 → kein Fehler, höchstens Hinweis | gemessen |
| 7 | Karte SRC | Pt: Vorlage 5 ↔ SRC 6 (beide Route 201), Weg (11, 18) → (11, 14). HG: Vorlage 1 ↔ SRC 2 (Route 29), Weg (8, 24) → (20, 24). Gegenrichtung in §7 | gemessen |
| 8 | Lock- und Snapshot-Kosten | Snapshot-Kopie < 0,3 ms (HG DE). Erstes Laden aller Assets 150–500 ms. Texte dekodieren 156–245 ms warm, 2,2 s beim ersten Lesen. Text-Index 100 ms | gemessen (Rust, opt-level 3) |
| 9 | `XX_YY` in PDSMS-Namen | **relativ**: x − kleinstes x, y − kleinstes y aller Karten des PDSMS-Projekts, `%02d` | geprüft (Quelle) |
| §13 | rotom `MessageLineTooLong` öffentlich? | ja: `rotom::CompileWarning::MessageLineTooLong { span, line_index }`, öffentliches Enum, nicht `non_exhaustive` | geprüft (Quelle) |

---

## 1. `dialog_line_px` = 216

**Platin** (`pokeplatinum` be36a9d):
- `src/field_message.c:30` in `FieldMessage_AddWindow` (Z. 27): `Window_Add(bgConfig, window, BG_LAYER_MAIN_3, 2, 19, 27, 4, 12, …)`. Das ist x 2, y 19, **27 Kacheln breit**, 4 hoch. Z. 32 ist dasselbe für den Sub-Bildschirm.
- Gedruckt wird ab x 0: `Text_AddPrinterWithParams(window, fontID, string, 0, 0, renderDelay, NULL)` (Z. 64, `FieldMessage_PrintWithParams`; ebenso Z. 54).
- Weg vom Script:
  - `ScrCmd_Message` (`src/scrcmd.c:1303`) ruft `ScriptMessage_Show` (`src/overlay005/script_message.c:43`).
  - `OpenMessageBox` ruft `FieldMessage_AddWindow` (Z. 130).
  - Danach kommt `FieldMessage_PrintWithParams` (Z. 162).

**HeartGold** (`pokeheartgold` 6988158):
- `src/dialog_box.c:12–15` definiert `DIALOG_BOX_X 2`, `DIALOG_BOX_Y 19`, `DIALOG_BOX_W 27`, `DIALOG_BOX_H 4`.
- Fenster: `DialogBox_AddWindowToLayer3` (Z. 29, `AddWindowParameterized` Z. 31).
- Druck ab x 0: `AddTextPrinterParameterized(window, fontId, message, 0, 0, textSpeed, NULL)` (Z. 58, ebenso Z. 51).
- Weg vom Script: `ScrCmd_NPCMsg` (`src/field/scrcmd_message.c:142`) → `ov01_021EF4DC` (Z. 206) → `ovFieldMain_CreateMessageBox` (Z. 277) → `DialogBox_AddWindowToLayer3`.

**Wert für `check_limits.yaml`:**
```yaml
dialog_line_px:
  value: 216
  verified: true
  source: "pokeplatinum be36a9d src/field_message.c:30 Window_Add(…, 2, 19, 27, 4, …), Druck ab x 0 (Z. 64); pokeheartgold 6988158 src/dialog_box.c:12–15 DIALOG_BOX_W 27, Z. 31, Druck ab x 0 (Z. 58)"
```

Das Histogramm aus `checks-report.md` (Kante bei 216 px, darüber 90/114/113 Nachrichten) passt dazu.

**Nebenbefund:** Schilder haben ein eigenes Fenster (Pt `FieldMessage_AddSignpostWindow`, `field_message.c:81–92`). Ortsschilder und Wegweiser sind 20 Kacheln = **160 px** breit, die übrigen Schilder 27. Schildnachrichten liegen im selben Textarchiv wie die Dialoge. `text.too-wide` mit 216 px übersieht dort zu breite Ortsschild-Texte. Das ist nicht gemessen, gehört nicht zum Umfang von 3b.4 und ist hier nur notiert.

## 2. Warp-Ziel 0x0FFF

**Platin:**
- Konstante: `generated/map_headers.txt:596` `MAP_HEADER_DYNAMIC = 4095`.
- Die Weiche ist der **Ziel-Warp**, nicht der Header. `src/overlay005/field_control.c`, `Field_MapConnection` (Z. 995):
  ```c
  if (warpEvent->destWarpID == 0x100) {                 // Z. 1009
      GF_ASSERT(warpEvent->destHeaderID == 0xfff);       // Z. 1010
      *nextMap = *(FieldOverworldState_GetSpecialLocation(...));   // Z. 1011
  } else {
      Location_Set(nextMap, warpEvent->destHeaderID, warpEvent->destWarpID, ...);  // Z. 1013
  }
  ```
- Das Ziel ist ein ganzer `Location` (Header, Warp, x, z, Richtung) aus dem Spielstand: `FieldOverworldState.special` (`include/field_overworld_state.h:17`, Zugriff `src/field_overworld_state.c:62–70`).
- Gesetzt wird es von `ScrCmd_SetSpecialLocation` (`src/scrcmd.c:3898–3910`). Beispiel: Kaufhaus-Aufzug Schleiede, `res/field/scripts/scripts_veilstone_store_elevator.s:73` `SetSpecialLocation MAP_HEADER_VEILSTONE_STORE_B1F, 1, 18, 2, DIR_SOUTH`.
- Kommt man auf einem Warp mit Ziel-Warp 0x100 an, setzt `src/field_map_change.c:232–237` das Ziel auf den Eingang, durch den man kam. Ohne Etagenwahl führt der Aufzug also zurück.

**HeartGold:**
- Gleiche Logik, keine benannte Konstante: `src/field/field_control.c:919–921` (`if (warpEvent->anchor == 0x100) { GF_ASSERT(warpEvent->header == 0xFFF); *location = *LocalFieldData_GetDynamicWarp(...)`).
- Ankunft: `src/field_warp_tasks.c:210–211`.
- Spielstand: `LocalFieldData.dynamicWarp` (`src/save_local_field_data.c:17`, Zugriff Z. 68–74).
- Script-Befehl: `ScrCmd_SetDynamicWarp` (`src/scrcmd_c.c:2495`).

**Vanilla, gemessen:** Ziel-Header 0x0FFF kommt in Pt 6-mal vor, in HG DE und US je 9-mal, immer mit Ziel-Warp 256 (`counts.py`, Info `dynamic warps`). Keine andere Kombination.

**Folge für den Katalog:**
- Die Ausnahme ist das **Paar** `(dest_header_id 0x0FFF, dest_warp_id 0x100)`, gilt für `warp.dest-header` **und** `warp.dest-warp`, Quelle wie oben. Metadaten: `warp.dynamic: { dest_header: 0x0FFF, dest_warp: 0x100, source: … }` statt nur `warp.dynamic_dest: 0x0FFF`.
- 0x0FFF mit anderem Ziel-Warp führt zu `Location_Set` mit Header 4095. Ziel-Warp 0x100 mit anderem Header verletzt den Assert und springt trotzdem zum Spielstand-Ziel. Beides bleibt **Fehler**: `warp.dest-header` bzw. eine eigene Meldung „dynamisches Ziel braucht Header 0x0FFF und Warp 256“.
- Inspector-Zeile für dynamische Warps: „Ziel: aus dem Spielstand (Aufzug)“. `warp.never-fires` und der Rückweg-Hinweis überspringen sie.

## 3. Puffergrößen Modell und BDHC

**Platin:**
- `src/overlay005/loaded_map_buffers.c:11` `#define MAP_MODEL_FILE_SIZE 0xF000`, Z. 20 `static u8 sGlobalMapModelFiles[4][MAP_MODEL_FILE_SIZE]`. Die Heap-Variante steht in Z. 111 und wird von `land_data.c:2008` genutzt.
- `include/constants/field/bdhc.h:4` `#define BDHC_BUFFER_SIZE 0x9000`. Der Assert steht in `src/overlay005/bdhc.c:282` `GF_ASSERT(offset <= BDHC_BUFFER_SIZE)` in `BDHC_PrepareBuffers` (Z. 254).

**HeartGold** (nur asm, aber eindeutig):
- `asm/overlay_01_021FAC44.s:146–147` `ov01_02209B70: .space 4 * 0xF000`: vier Modellpuffer zu 0xF000 (Schrittweite `mov r1, #0xf; lsl r1, r1, #0xc`, Z. 27–28).
- Z. 44–47 `mov r1, #9; lsl r1, r1, #0xc; bl Heap_Alloc`: BDHC-Puffer 0x9000.
- BDHC-Assert in `asm/overlay_01_021FB04C.s:105–109` (`ov01_021FB0B0`, Gegenstück zu `BDHC_PrepareBuffers`): `mov r0, #9; lsl r0, r0, #0xc; cmp r1, r0; ble …; bl GF_AssertFail`.

**Wichtig:** Der BDHC-Assert prüft die **Nutzlast ohne den 16-Byte-Kopf**: `Punkte × 8 + Normalen × 12 + Konstanten × 4 + Platten × 8 + Streifen × 8 + Zugriffsliste × 2`. Die Zähler stehen nach der Magic (`bdhc.c:239–251`). Gemessen: In allen Vanilla-BDHC aller drei Projekte gilt Länge = 16 + Nutzlast.

**Größte Vanilla-Werte** (`counts.py`, Info `maxes`):

| | Modell | BDHC (Datei / Nutzlast) |
|---|---|---|
| Platin DE | 60 556 B (Karte 489) | 21 468 / 21 452 B (Karte 188) |
| HeartGold DE | 61 420 B (Karte 318) | 10 794 / 10 778 B (Karte 658) |
| HeartGold US | 61 420 B (Karte 318) | 10 794 / 10 778 B (Karte 658) |

**Folge:**
- `model.max_bytes: 0xF000` und `bdhc.max_bytes: 0x9000` gelten für beide Familien, mit `verified: true`.
- `map.bdhc-size` wird für HGSS **Fehler** wie für Platin. „HGSS als Warnung, offen“ in Spec §4 und §9 entfällt.
- Die Regel lautet: Nutzlast aus den Zählern ≤ 0x9000 **und** Länge = 16 + Nutzlast. 0 Vanilla-Treffer.

## 4. Vanilla-Zählungen

**Regeln** (`scratchpad/t4/counts.py`, wie `baseline.py` aus der Recherche, mit diesen Änderungen):
- HGSS-Header-Layout korrigiert.
- Event-Prüfungen gegen **jeden** Header, der die Eventdatei nutzt.
- Slot-Zahl einer Scriptdatei: u32-Sprungtabelle bis `0xFD13` **oder** bis zum Beginn des ersten Scripts. Pt-Dateien 0101, 0164 und 0365 haben kein `0xFD13`. Der naive Suchlauf hielte sie für Level-Scripts und ergäbe 15 falsche `event.script-missing` und 3 falsche `header.script-kind`.
- Diese Binärregel stimmt bei allen 1124 Pt- und 965 HG-Membern mit der Einteilung der übernommenen Kopien überein (`.rotom` gegen `.json`).

**Was genau gezählt wird:**
- `event.trainer-missing`: Script 3000–4999 → Trainer `n − 3000 + 1`, 5000–6999 → `n − 5000 + 1`.
- `trainer.id-range`: Art ≥ Artnamen (496), Item ≥ `itemData`, Attacke ≥ `moveData`, Klasse ≥ Klassennamen (Pt 105, HG 129). Ebenso die vier Trainer-Items.
- Begegnungen: nur Methoden mit Rate > 0.
  - Pt-Rate = niedriges Byte.
  - HGSS-Gras je Spalte (morgens, tags, nachts).
- `map.model-invalid`, alle Importregeln aus Spec §9 auf das Vanilla-Modell angewandt:
  - Magic, BOM `FFFE`, Version 2, Größenfeld, Kopf 16.
  - 1–2 Blöcke, Offsets gültig, genau ein MDL0 mit genau einem Modell.
  - ≤ 0xF000.
- `map.building-range`: Platin gegen `exteriorBuildingModels` (590). HGSS gegen `exteriorBuildingModels` (340); gegen `interiorBuildingModels` (222) lägen 360 Gebäude-Einträge darüber, siehe unten.
- `area.tileset-range`: `buildings_tileset` ≥ `buildingTextures`, `map_tileset` ≥ `mapTextures`.

| Code | Pt DE | HG DE | HG US | Spec-Schwere | Folge |
|---|---|---|---|---|---|
| `warp.dest-header` (ohne 0x0FFF/0x100) | 0 | 0 | 0 | Fehler | Fehler |
| `warp.dest-warp` | 0 | 0 | 0 | Fehler | Fehler |
| `warp.off-map` | 0 | 0 | 0 | Fehler | Fehler |
| `event.script-missing` Figuren | 0 | 0 | 0 | Fehler | Fehler |
| `event.script-missing` Trigger | 0 | 0 | 0 | Fehler | Fehler |
| `event.script-missing` Schilder | 1 | 0 | 0 | Warnung | Warnung (Pt Eventdatei 44, Schild 1, Script 45, Scriptdatei hat 22) |
| `event.trainer-missing` | 0 | 0 | 0 | Fehler | Fehler |
| `event.off-map` (Figuren, Trigger, Schilder) | 0 | 0 | 0 | Fehler | Fehler |
| `header.ref-range` | 0 | 0 | 0 | Fehler | Fehler |
| `header.wild-dangling` | 0 | 0 | 0 | Fehler | Fehler |
| `header.script-kind` | 0 | 0 | 0 | Fehler | Fehler (HG US mit der Binärregel) |
| `script.message-missing` | 0 von 4412 | 0 von 3945 | nicht gemessen | Fehler | Fehler (HG US ohne übernommene Kopie) |
| `header.level-target` | 1 | 1 | nicht gemessen | Warnung | Warnung (Pt Header 186 → Script #1 fehlt in 197; HG Header 499, Level-Script 521 → #4 fehlt in Scriptdatei 748) |
| `trainer.no-party` (genutzt) | 0 | 0 | 0 | Fehler | Fehler (Meldung ändern, §5) |
| `trainer.no-party` (ungenutzt, ohne Trainer 0) | 0 | 0 | 0 | Warnung | Warnung |
| `trainer.empty-species` | 0 | 0 | 0 | Fehler | Fehler |
| `trainer.id-range` | 0 | 0 | 0 | Fehler | Fehler |
| `matrix.map-range` | 0 | 0 | 0 | Fehler | Fehler |
| `matrix.header-range` | 0 | 0 | 0 | Fehler | Fehler |
| `map.model-invalid` | 0 | 0 | 0 | Fehler | Fehler |
| `asset.unreadable` | 0 | 0 | 0 | Fehler | Fehler |
| `text.encode`, `text.json-broken` | 0 | 0 | 0 | Fehler | Fehler (Werte aus `checks-report.md` §1.5; Vanilla hat keine Text-JSONs) |
| **bedingt:** `map.building-range` | 0 | 0 | 0 | bedingt | Fehler zulässig, HGSS gegen die Exterior-Zahl (siehe unten) |
| **bedingt:** `area.tileset-range` | 0 | 0 | 0 | bedingt | Fehler zulässig |
| **bedingt:** `matrix.header-elsewhere` | 14 | 13 | 13 | bedingt | mit Ausnahme „Header-Zelle 0“ 0/0/0 → Warnung (siehe unten) |
| `event.trainer-blind` | 0 | 0 | 0 | Warnung | Warnung |
| `event.trigger-empty` | 0 | 1 | 1 | Warnung | Warnung (HG Eventdatei 125, Trigger 0) |
| `event.same-spot` | 2 | 8 | 8 | Warnung | Warnung |
| `header.text-split` | 0 | 0 | 0 | Warnung | Warnung |
| `trainer.no-moves` | 0 | 0 | 0 | Warnung | Warnung |
| `trainer.double-one` | 0 | 0 | 0 | Warnung | Warnung |
| `map.bdhc-size` | 0 | 0 | 0 | Warnung (HGSS) | **Fehler** in beiden Familien (§3) |
| `encounter.empty-slot` | 0 | 0 | 0 | Warnung | Warnung |
| `encounter.level-zero` | 0 | 0 | 0 | Warnung | Warnung |
| `encounter.min-max` | 0 | 0 | 0 | Hinweis | Hinweis |
| `warp.never-fires` | 10 | 12 | 12 | Hinweis | Hinweis |

**Anmerkungen:**
- **`matrix.header-elsewhere`:**
  - Alle Treffer sind Header-Zellen mit dem Wert **0** in der einzigen zweiten Matrix mit Header-Raster: Pt Matrix 240 (4 × 5, Füllkarte 176), HG Matrix 212 (5 × 4, Füllkarte 211). Header 0 nutzt Matrix 0.
  - Mit „Header-Zelle 0 = kein Header“ bleiben 0/0/0. Dass das Spiel 0 so liest, ist nicht in pret nachgeschlagen (die Regel ist gemessen).
  - Vorschlag: Warnung mit Ausnahme `matrix.header_none: 0` (Quelle: Messung). Fehler erst mit pret-Beleg.
- **`map.building-range` HGSS:** Welches Gebäude-Archiv HGSS je Karte nimmt, ist hier nicht geklärt. Gemessen ist nur: Alle Gebäude-IDs aller Karten liegen unter 340 (Exterior). Gegen Interior (222) gäbe es 360 Treffer. Regel für die Prüfung: ID ≥ Zahl der Exterior-Modelle (das größere Archiv), offen bis zur Klärung.
- **Begegnungsregel:** `encounter.empty-slot` zählt hier wie in `checks-report.md` „Rate > 0 und Art 0“. Der 3b.3b-Zweig (`d3a212b`, `encounter_rules.rs`) erlaubt leere Slots ausdrücklich. Beide Lesarten ergeben 0 Vanilla-Treffer. Die Spec muss eine wählen.
- `trainer.no-party` zählt nur Event-Nutzer (Objekt-Scripts 3000–6999). Trainer, die nur aus einem Script heraus kämpfen, sind nicht erfasst. Vanilla hat ohnehin 0 Trainer ohne Party außer Trainer 0.
- Trainer 0 hat in allen drei Projekten `party_count 0`, Klasse 0 und keinen Event-Nutzer. Die Ausnahme `trainer.dummy: 0` bleibt.

**Messprotokoll** (Scratch `t4/run.sh`):
```sh
python3 -I counts.py "$R/Pokemon - Platin-Edition (Germany)_DSPRE_contents" pt E6074 $M/tile_behaviors.yaml $R/_testkopien/pt-3c1 $M/scrcmd/platinum_v2.json > pt.json
python3 -I counts.py "$R/Pokemon - Goldene Edition HeartGold (Germany)_DSPRE_contents" hgss F6B94 $M/tile_behaviors_hgss.yaml $R/_testkopien/hg-3b2-3c $M/scrcmd/hgss_v2.json > hgde.json
python3 -I counts.py "$R/Pokemon - HeartGold Version (USA)_DSPRE_contents" hgss F6BE0 $M/tile_behaviors_hgss.yaml > hgus.json
```
`M = crates/g4-kernel/metadata`.
- Die Nachrichtenbefehle kommen aus der Befehlsdatenbank: Parameter `text_slot` oder Typ `msg_id`, ohne `MessageFromBank*`, TV-Befehle und Menüeinträge. Pt sind das 15 Befehle (u. a. `Message`, `MessageInstant`, `NPCMessage`, `ShowMapSign`), HG 12 (u. a. `NPCMsg`, `NonNPCMsg`, `DirectionSignpost`, `TrainerTips`).
- Laufzeit: Python 0,3–0,5 s warm je Projekt.

## 5. Trainer ohne Party: kein Absturz, aber ein Geisterkampf

**Vorbemerkung zur Aufgabe:** Ein Objekt-Event mit Script 3000 meint Trainer **1**, nicht 0. pret `Script_GetTrainerID` rechnet `script − 3000 + 1` (siehe `trainer-mechanics-3b3b.md` §1). Trainer 0 ist über ein Objekt-Event nicht erreichbar. Gemessen wurde deshalb mit einem echten Trainer, dessen `party_count` auf 0 gesetzt ist; die Party-Datei sind 8 Nullbytes, wie bei Trainer 0.

**Ein erster Versuch an Header 411 taugte nicht.** Kopie `pt-3b4-t0-noparty`: Vanilla plus Figur Trainer 4 bei (116, 888), Script 3003, `trainer_type` 1, Sicht 2, Blick Nord.
- Die Sichtlinie greift: Bild „!“, dann der Text „Der Elan meiner Pokémon…“.
- Aber der Spielstand `CPUD-zimmer.sav` hat noch kein eigenes Pokémon, ein Kampf dort prüft also etwas anderes. Die Kopie behält ihre Party unverändert.
- Weiter ging es mit den 3b.3b-Kopien, die einen Geber haben.

**Platin** (Kopie `pt-3b4-t0-noparty2` = `cp -R pt-3b3b-trainer`):
- `control.nds`: unverändert gebaut.
- `noparty.nds`: `trainerProperties/0928` Byte 3 = 0, `trainerParty/0928` = 8 Nullbytes. Trainer 928 ist die Figur bei (7, 6) in Zimmer 415, Script 3927, Typ 1, Sicht 1, Blick Nord.
- Ablauf:
```sh
cd scripts/g4emu; R=…/Roms/_testkopien
uv run g4emu run --rom $R/pt-3b4-t0-noparty2/noparty.nds --save $R/_saves/CPUD-zimmer.sav --out $O \
  continue walk-to 8 4 walk right 1 expect-map 414 wait 200 press left:16 expect-map 415 wait 200 \
  walk left 1 (press a:4 wait 60)×14 shot g0          # Geber: Knakrack L100
uv run g4emu run --rom … --state $O/gift.state --out $O/s walk down 1 wait 90 shot s1 press a:4 … shot s5
… --state b1.state … press a:4 (Attacke wählen) … shot u1…u6
… --state after.state … walk-to 4 5 where            # Spiel läuft weiter
```

| Schritt | Kontrolle (`control.nds`) | Ohne Party (`noparty.nds`) |
|---|---|---|
| Sichtlinie, Text | „Eine Herausfo…“ | gleich |
| Ball-Anzeige Gegner | 1 Ball | 0 Bälle (6 leere Kreise) |
| Gegner schickt | „LUCARIO“ | „!!!!!!!!!!“ (unsichtbar, Lv 0, leere KP-Leiste) |
| Erster Zug | – | Menü erscheint; Attacke → „Aber da ist gar kein Ziel…“ |
| Ende | – | „AAAAA gewinnt 2448 ₽!“, zurück im Zimmer, `walk-to 4 5` geht |

**HeartGold DE** (Kopie `hg-3b4-t0-noparty` = `cp -R hg-3b3b-trainer`, Trainer 738 bei (4, 6) in Zimmer 64, Script 3737):
- `trainerProperties/0738` Byte 3 = 0, Party-Datei 8 Nullbytes, `noparty.nds`.
- Ablauf: `continue walk left 1 walk down 1 expect-map 63 wait 200`, `press left:16 expect-map 64 wait 200`, `walk right 1`, Geber 14× A, `walk down 1`.
- Ergebnis wie Platin:
  - „Eine Herausforderung von Teenager Jens!“, dann „!!!!!!!!!! wird von Teenager Jens in den…“, Gegner Lv 0 ohne KP.
  - Kampfmenü, Attacke → „Aber da ist gar kein Ziel…“.
  - Nach dem Sieg zurück in Zimmer 64 (`where` 64, 4, 5).
- Kontrolle ohne Änderung: in 3b.3b gemessen (`trainer-mechanics-3b3b.md` §1.3, Lucario L7).
- Die Bilder liegen im Scratch (`t5/c2`, `t5/n2`, `t5/hg`), nicht im Repo.

**pret dazu** (pokeplatinum, abgeleitet, nicht im Emulator nachvollzogen):
- `TrainerData_BuildParty` (`src/trainer_data.c:176`) läuft mit `partySize` 0 durch alle Schleifen; die Party bleibt leer.
- `Party_GetPokemonBySlotIndex` hat `GF_ASSERT(slot < currentCount)` (`src/party.c:10–15`). `ErrorHandling_AssertFail` (`src/error_handling.c:11–16`) reagiert nur bei initialisierter Kommunikation. Im Einzelspiel geht es also mit dem genullten Slot 0 weiter (Art 0, KP 0).
- `BattleControllerPlayer_CheckBattleOver` (`src/battle/battle_controller_player.c:4242–4258`) wertet das als Sieg.
- Im Link-Kampf löste der Assert den Fehlerbildschirm aus. Nicht getestet.

**Folge:**
- „stürzt ab“ in der Spec-Meldung ist falsch.
- Das Spiel liest trotzdem Müll: Name aus Art 0, Lv 0, kostenloser Sieg mit Preisgeld. Die Sieg-Flag wird gesetzt (nach dem 3b.3b-Kampfscript), der Trainer ist danach endgültig „besiegt“.
- Nach der Katalogregel („Fehler = Spiel liest Müll …“) bleibt `trainer.no-party` für genutzte Trainer **Fehler**. Neue Meldung: „Wanderer Jens hat kein Pokémon. Im Kampf erscheint ein leeres Pokémon und der Kampf ist sofort gewonnen. Füge ein Pokémon hinzu.“
- Nicht gemessen: Doppelkampf mit 0 oder 1 Pokémon (`trainer.double-one`), HG US.

## 6. Material-Leser (Texturnamen)

**Leser** (`scratchpad/t6/textures.py`):
- Liest das Nitro-Dictionary nach den Regeln aus `docs/formats/nsbtx.md`: Namen 16 Byte nach dem Eintragsblock, `ofsEntry` an +6, `sizeUnit`/`ofsName` am Eintragsblock.
- MDL0 → Modell 0 → `ofsMat` (Modellkopf +8) → `ofsDictTexToMatList`, `ofsDictPlttToMatList` → Texturnamen und Palettennamen.
- NSBTX → TEX0 → Texturen-Dict (+0x0E) und Paletten-Dict (+0x34).
- Zuordnung Karte → Pack:
  - Für jeden Header die Karten seiner Matrix; bei Header-Raster nur die Zellen dieses Headers.
  - Dann `area_data_id` → `map_tileset` → `mapTextures/NNNN`.
  - Eine Karte mit mehreren Packs gilt als ok, wenn ein Pack alle Namen hat.

**Ergebnis:**
- Der Leser liest **alle** Modelle ohne Fehler: Pt 666 (7060 Textur-, 6716 Palettennamen), HG DE/US je 676 (10 529 / 10 441).
- Gegenprobe mit `apicula info <modell.nsbmd> <pack.nsbtx>` für alle Karten mit genau einem Pack (Pt 624, HG DE 644): dieselben „Texture: … (not found)“ wie der Leser (`t6/apx.py`).

| | Pt DE | HG DE | HG US |
|---|---|---|---|
| Karten ohne Header (nicht geprüft) | 34 | 30 | 30 |
| Karten mit mehreren Packs | 8 | 2 | 2 |
| **Karten mit fehlenden Namen** | **14** | **7** | **7** |
| fehlende Textur- / Palettennamen | 46 / 47 | 7 / 7 | 7 / 7 |

**Welche Karten:**
- Pt:
  - Weltkarten 173, 176, 177, 179: Zellen der Matrix 0 mit Header-Zelle 0, also Pack 0. Hier ist die Zuordnung zweifelhaft.
  - 238, 239, 240: `h_kage`, Pack 57.
  - 249: Pack 21.
  - 295: 18 Namen, Pack 25.
  - 301, 302: `m_fac_01–04`, Pack 32.
  - 507: nur Palette `dun_floor2_d20`, Pack 71.
  - 594, 617: Pack 73.
- HG:
  - `bike_02_2`: Karten 241, 280, 281, 359, 372.
  - `t_d_yuka3`: Karte 333.
  - `droad01`: Karte 566.

**Folge:**
- Die Prüfung „Texturnamen gegen Area-NSBTX“ ist technisch möglich: Der Leser ist klein und deckt Vanilla vollständig ab. Sie trifft aber auf allen drei Projekten Vanilla.
- Nach der harten Regel also **kein Fehler und kein gated Soll 0**.
- Vorschlag: beim Import als **Hinweis** („3 Texturnamen fehlen im Pack von Area-Data 6: …“). Am besten nur Namen, die im ersetzten Modell nicht schon fehlten.
- Projektweit bleibt die Prüfung „später“. Dort wäre ein Soll je Projekt nötig (14/7/7).
- Laut pret bindet `Easy3D_BindTextureToResource` immer mit Rückgabe TRUE (`import-report.md` §2.2). Fehlende Namen lassen das Spiel also nicht abstürzen.

## 7. Karte SRC für Gate B

**Vorgehen:**
- Kandidaten SRC: alle Karten, die ein Header mit derselben `map_tileset` wie die Vorlage zeigt. Gleiches Pack heißt: Das importierte Modell erscheint texturiert.
- Auf der Vorlage blockiert heißt Kollisionsbyte & 0x80. Begehbar heißt Bit 7 frei und Verhalten 0x00 oder 0x02 (hohes Gras).
- Ein gerader Weg soll in SRC ganz begehbar sein und in der Vorlage an einer Kachel enden.
- Werkzeuge: `t7/src.py` (Suche), `t7/show.py` (ASCII-Karten), `t7/sim.py` (Gang wie `walk_to`: erst x, dann y, ohne Wegsuche), `t7/bd.py` (BDHC).

**Platin**, Vorlage Header 342 (Route 201):
- Matrix 0, Zellen (3, 26) = Karte 5 und (4, 26) = Karte 6.
- Area-Data 6, Pack 6; 22 Kandidaten.

| Vorlage → SRC | Start → Ziel (lokal) | Vorlage | SRC |
|---|---|---|---|
| **5 → 6** (beide Route 201) | (11, 18) → (11, 14) | „blockiert bei x 11, y 18“ (Kachel 11, 17: Verhalten 0x00, Kollision 0x80, Baumreihe) | erreicht (11, 14) durch hohes Gras |
| 6 → 5 | (12, 18) → (24, 18) | „blockiert bei x 18, y 18“ (Kachel 19, 18: 0x00/0x80) | erreicht (24, 18) |

Ein längerer Weg wäre 5 → 6 von (12, 22) nach (12, 12). Die Vorlage stoppt dort aber an einer Kante (Verhalten 0x3B), weniger eindeutig.

**HeartGold** (DE und US identisch), Vorlage Header 33 (Route 29):
- Matrix 0, Zellen (18, 12) = Karte 1, (19, 12) = Karte 2, (20, 12) = Karte 3.
- Area-Data 2, Pack 2; 21 Kandidaten.

| Vorlage → SRC | Start → Ziel (lokal) | Vorlage | SRC |
|---|---|---|---|
| **1 → 2** | (8, 24) → (20, 24) | „blockiert bei x 11, y 24“ (Kachel 12, 24: 0x06/0x80) | erreicht (20, 24) |
| 2 → 1 | (8, 18) → (24, 18) | „blockiert bei x 11, y 18“ (Kachel 12, 18: 0x06/0x80) | erreicht (24, 18) |

**Hinweise für den Plan:**
- Die Koordinaten sind lokal in der Karte. In einer Matrix mit mehreren Zellen kommt 32 × Zellenindex dazu (`walk-to` arbeitet mit absoluten Koordinaten).
- Die BDHC der Karten 5, 6 (Pt) und 1, 2 (HG) sind flach mit derselben Höhe (eine Normale nach oben, Konstante −16). Import von Modell und `.per` ohne BDHC hält die Laufhöhe.
- HGSS-Kollisionsbytes tragen weitere niedrige Bits (0x04, 0x06, 0x0A …: in HG DE rund 91 000 Zellen mit Bit 7 frei und Wert ≠ 0). „Blockiert“ heißt also nur Bit 7. Platin kennt nur 0x00 und 0x80.
- Die Ankunft muss auf einer Kachel liegen, die in Vorlage und SRC frei ist, am besten der Start des Wegs. Die Ankunft (10, 10) aus `pt-neuer-header.yaml` liegt in Karte 5 auf einer blockierten Zeile (Zeilen 0–13 sind ganz blockiert).

## 8. Lock- und Snapshot-Kosten

**Messung:** Temporärer Test `crates/g4-kernel/tests/t0_snapshot_timing.rs` über `Store`:
- `header_table`, `member_count`, `event`, `matrix`, `encounter`, `map`, `text_archive`, `search_text`.
- Trainer direkt von der Platte mit `TrainerProps::read` und `Party::read`, weil Trainer auf `main` noch nicht im Store sind.
- Snapshot = `clone()` der geladenen Werte.

Gebaut wurde mit
```sh
cargo test --workspace --all-features --no-run --config 'profile.dev.package.chatot.opt-level=3' \
  --config 'profile.dev.package.g4-kernel.opt-level=3' --config 'profile.dev.package.g4-formats.opt-level=3' \
  --config 'profile.dev.package.uxie.opt-level=3' --config 'profile.dev.package.serde_json.opt-level=3'
DYLD_LIBRARY_PATH=target/debug/deps T0_PROJECT=<projekt> target/debug/deps/t0_snapshot_timing-<hash> --nocapture
```
- `cargo --release` scheitert am Download im rotom-Build-Skript („the headers' total size surpassed max_headers_size“). Auch `cargo test --test …` allein baut rotom neu und scheitert so.
- Übrige Abhängigkeiten (binrw u. a.) blieben ohne Optimierung. Echte Release-Werte liegen eher etwas darunter.
- „Kalt“ im Sinn eines leeren Datei-Caches ließ sich nicht erzwingen (`purge` braucht sudo). Der „erste Lauf“ ist jeweils der erste Zugriff des Tages auf das Projekt.
- Danach wurde der Test gelöscht und der normale Debug-Build wiederhergestellt.

**HeartGold DE** (540 Header, 491 Eventdateien, 288 Matrizen, 142 Begegnungen, 738 Trainer, 676 Karten, 829 Textarchive mit 49 984 Nachrichten):

| Schritt | erster Lauf | zweiter Lauf |
|---|---|---|
| Header-Tabelle | 2,7 ms | 0,7 ms |
| Member-Zahlen (7 Arten) | 8,8 ms | 2,1 ms |
| Eventdateien laden | 108 ms | 31 ms |
| Matrizen laden | 45 ms | 17 ms |
| Begegnungen laden | 22 ms | 9 ms |
| Trainer (Platte + Codec) | 237 ms | 89 ms |
| Karten laden (für Permissions) | 126 ms | 47 ms |
| **Snapshot-Kopie** Header + Events + Matrizen + Begegnungen + Trainer | 0,23 ms | 0,07–0,13 ms |
| + Permissions aller Karten | 0,25 ms | 0,05–0,15 ms |
| Textarchive in den Store dekodieren (829) | 245 ms | 156 ms |
| Zugriff auf schon dekodierte Archive | 0,04 ms | 0,04 ms |
| alle Text-Dokumente klonen | 2,0 ms | 2,0 ms |
| Text-Index (`search_text`, erster Aufruf, dekodiert erneut von der Platte) | 103 ms | 103 ms |
| Text-Index, zweiter Aufruf | 1,7 ms | 1,7 ms |

**Platin DE** zum Vergleich:
- Snapshot 0,08–0,28 ms.
- Texte dekodieren 2224 ms beim ersten Zugriff des Tages, 148 ms danach.
- Index 98–142 ms.
- Ohne Optimierung (reiner Debug-Build, HG DE): Texte 6,3 s beim ersten Zugriff, 1,4 s warm; Index 0,88 s; Snapshot 0,5 ms.

**Budget:**
- **Unter dem Lock: höchstens 5 ms.** Kopiert werden nur geladene und ungespeicherte Werte (Snapshot < 0,3 ms auf HGSS) und ungespeicherte Text-Dokumente (alle 829 zusammen 2 ms). Das Spec-Ziel „< 10 ms“ hält mit Faktor 30 und mehr.
- **Nicht unter dem Lock:**
  - Das erste Laden noch nicht geladener Assets kostet 150–500 ms und ist I/O.
  - Das Dekodieren der Texte kostet 150 ms warm und über 2 s kalt.
  - Der Text-Index kostet 100 ms.
  - Der Arbeits-Thread liest, was der Store nicht geladen hat, selbst von der Platte (Basis = Platte). Er nimmt nur die Generation unter dem Lock mit.
- **Volllauf ohne Lock:**
  - Billiger Teil inklusive Lesen von der Platte: ≤ 100 ms warm. Python brauchte 0,3–0,5 s für alle Prüfungen inklusive I/O; Rust-Laden allein 0,2 s warm.
  - Texte: 150–250 ms warm, einige Sekunden kalt, mit Fortschritt.
- **Folge für die Spec:** `Store::text_archive` und `search_text` dekodieren unter `&mut self`. Wer sie unter dem App-Mutex aufruft, hält den Lock 100–250 ms (kalt Sekunden). Die Text-Prüfung muss Archive selbst lesen (`text_winner`, dann `from_member` oder `from_json`) und vom Store nur die ungespeicherten Dokumente nehmen.

## 9. `XX_YY` in PDSMS-Dateinamen: relativ

- PDSMS upstream `ac30b653`, `src/main/java/editor/mapmatrix/MapMatrix.java:622–627`, `getMapName`:
  ```java
  Point minCoords = getMinCoords(matrix);
  mapName += "_" + String.format("%02d", mapCoords.x - minCoords.x) + "_" + String.format("%02d", mapCoords.y - minCoords.y);
  ```
- `getMinCoords` (Z. 708–720) nimmt das kleinste x und das kleinste y über alle Karten des PDSMS-Projekts, unabhängig voneinander.
- Dieselbe Verschiebung schreibt `saveGridsToFile` als `mapstart` in die `.pdsmap` (Z. 129–133).
- Der Fork AdAstra-LD `de409512` ist gleich (`MapMatrix.java:775–780`).
- Quellen: Klone aus der Recherche (`scratchpad/research-3b4/src/pdsms`, `pdsms-adastra`, Sitzung e1c6e331).

**Doku-Satz:** „`XX_YY` = Spalte und Zeile der Karte relativ zur kleinsten belegten Spalte bzw. Zeile des PDSMS-Projekts, zweistellig (`%02d`), nicht die Zelle in der Matrix des Spiels.“

## Nebenbefund zu §13: rotoms `MessageLineTooLong`

- rotom v0.7.0 `fb11503`: `src/compiler/diagnostic/warning.rs:10` `pub enum CompileWarning` (kein `non_exhaustive`), Variante Z. 34 `MessageLineTooLong { span, line_index }`.
- Öffentlich exportiert in `src/lib.rs:30–34` (`pub use compiler::{…, diagnostic::{CompileError, CompileWarning, …}}`).
- Das Umwandeln kann die Warnung also über `matches!(w, rotom::CompileWarning::MessageLineTooLong { .. })` verwerfen. Das Deduplizieren über die Spanne entfällt.

## Änderungen an der Spec (Vorschläge aus Task 0)

1. `check_limits.yaml`:
   - `dialog_line_px: 216` mit `verified: true` und den Quellen aus §1.
   - `warp.dynamic: { dest_header: 0x0FFF, dest_warp: 0x100 }` statt `warp.dynamic_dest`.
   - `model.max_bytes` und `bdhc.max_bytes` (Nutzlast) für beide Familien `verified: true`.
   - `matrix.header_none: 0` (gemessen, nicht pret-belegt).
2. `map.bdhc-size`: Fehler für Pt und HGSS. Regel: Nutzlast ≤ 0x9000 und Länge = 16 + Nutzlast. Gleiche Regel beim BDHC-Import (§9 der Spec), dort „HGSS als Warnung, offen“ streichen.
3. `trainer.no-party` (genutzt) bleibt Fehler, mit neuer Meldung (§5). „Stürzt ab“ streichen.
4. „Texturnamen gegen Area-NSBTX“: nicht gated. Beim Import Hinweis, projektweit „später“ mit Soll 14/7/7.
5. `matrix.header-elsewhere`: Warnung mit Ausnahme Header-Zelle 0. `map.building-range` und `area.tileset-range` dürfen Fehler sein (0/0/0). HGSS-Gebäude gegen die Exterior-Zahl.
6. Slot-Zahl einer binären Scriptdatei: Sprungtabelle bis `0xFD13` oder bis zum ersten Script-Beginn. Sonst gibt es 15 falsche Fehler in Platin.
7. Gate B:
   - SRC und Weg aus §7.
   - Die Ankunft im neuen Ort auf den Weg-Start legen, nicht auf (10, 10).
   - HGSS „blockiert“ = Bit 7.
8. Soll-Tabelle für `vanilla_checks`: die Spalten aus §4. HG US bekommt die Script-Prüfungen erst mit einer übernommenen Kopie im Test.

## Offen

- HGSS: welches Gebäude-Archiv (Exterior oder Interior) eine Karte nutzt. Gemessen ist nur, dass alle IDs unter der Exterior-Zahl liegen.
- Bedeutung von Header-Zelle 0 in Matrizen mit Header-Raster (pret nicht nachgeschlagen).
- HG US: `script.message-missing`, `header.level-target` (keine übernommene Kopie).
- Trainer ohne Party: Doppelkampf, Link-Kampf, HG US (nicht gespielt).
- Echte Release-Zeiten und ein Lauf mit leerem Datei-Cache (Build- bzw. sudo-Grenze, §8).
- Ortsschild-Breite 160 px für Schildtexte im selben Archiv (§1, außerhalb 3b.4).
