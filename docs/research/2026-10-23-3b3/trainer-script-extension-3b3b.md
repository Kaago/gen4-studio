# Kampf-Scriptdatei erweitern: mehr neue Trainer in Platin und HeartGold

Stand 2026-10-24. Spike für Phase 3b.3b, Gate „zwei neue Trainer je Spiel“. Baut auf [trainer-mechanics-3b3b.md](trainer-mechanics-3b3b.md) (§1, §4) und [encounter-mechanics-3b3b.md](encounter-mechanics-3b3b.md) (§4.5, Signatur) auf.

## Antwort

1. Ja: Hängt man in der rotom-Quelle der Kampf-Scriptdatei (Pt `1114`, HGSS `0953`) weitere Nummern an den Kopf des Kampf-Scripts (`script script_1 #[1-928, 930]` bzw. `#[1-739, 741]`), kompiliert der Kernel über `save_script` einen um je 4 Byte längeren Header. Der Code bleibt byte-gleich. Neue Trainer 930 (Pt) und 741 (HGSS) kämpfen beim Ansprechen, ohne arm9-Patch.
2. Die ID direkt hinter dem Vanilla-Ende (Pt 929, HGSS 740) bleibt auch nach der Erweiterung gesperrt. Ihr Header-Eintrag ist der „gesehen“-Code, und dessen Nummer (3928 bzw. 3739) ist im arm9 fest verdrahtet. Beim Ansprechen friert das Spiel ein, jetzt auch in HGSS gemessen.
3. Ein Wächter aus drei Befehlen am Anfang des „gesehen“-Scripts macht auch diese ID nutzbar, ohne arm9-Patch: „wurde dieses Script durch Sichtkontakt gestartet? Sonst zum Kampf-Code.“ In beiden Spielen gemessen, die Sichtlinie bleibt intakt. Für den Plan empfehle ich trotzdem: Erweiterung in `create_trainer`, die ID überspringen, den Wächter nur als Option.

**Quellen**
- pret `pokeplatinum` `be36a9db3d2390a5009adc087c7aef550ce335e7` („Pt-pret“), pret `pokeheartgold` `6988158691eb6178cff697cda0428045ddcba5fb` („HG-pret“), dieselben Stände wie im Trainer-Bericht.
- rotom-Befehlsdatenbank `crates/g4-kernel/metadata/scrcmd/{platinum,hgss}_v2.json`.
- Kernel-Compile `crates/g4-kernel/src/scripts.rs` (`write_source`, `compile`), auf `main` und im 3b.3a-Worktree gleich.
- Brücke: `studio-bridge` aus dem 3b.3a-Worktree, Build vom 2026-10-24 11:27 (packt `trainerProperties`, `trainerParty`, `trainerTextTable`, `trainerTextOffset`; `main` packt sie noch nicht). Direkt gestartet auf Port 1431, ohne Vite, Aufrufe über `scripts/agent/call.sh`.
- Testkopien, frisch aus den Originalen:
  - `../Roms/_testkopien/pt-3b3b-trscript` (Platin DE, CPUD),
  - `../Roms/_testkopien/hg-3b3b-trscript` (HeartGold DE, IPKD).
- Gespielt mit `g4emu`, Spielstände `CPUD-zimmer.sav` und `IPKD-zimmer.sav` (beide ohne Team). Bilder und RAM-Abzüge liegen lokal unter `target/agent/trscript/` (nicht im Repo).

## 1. Struktur der Kampf-Scriptdatei

### 1.1 Binär (beide Spiele)

Eine Scriptdatei beginnt mit einer Tabelle aus `i32`-Einträgen, beendet durch `u16 0xFD13`.
- Eintrag *k* gehört zur Script-Nummer *k* der Datei. Sein Wert ist relativ zum Ende des Eintrags: `Ziel = 4·k + 4 + rel`.
- Pt-pret `src/script_manager.c`, `ScriptContext_JumpToOffsetID`: `ctx->scriptPtr += offsetID * 4; ctx->scriptPtr += ScriptContext_ReadWord(ctx);`
- Es gibt **keine Grenzprüfung** gegen die Zahl der Einträge. Eine Nummer hinter dem letzten Eintrag liest den Terminator und den Code als Offset, das Ergebnis ist undefiniert.

| | Pt `1114` (Vanilla) | HGSS `0953` (Vanilla) |
|---|---|---|
| Einträge | 929 | 740 |
| Kampf-Code (Ansprechen) | Einträge 0–927 → `0x0E86` | Einträge 0–738 → `0x0B92` |
| „gesehen“-Code | Eintrag 928 → `0x1054` | Eintrag 739 → `0x0EC4` |
| Header-Ende | `0x0E84` (929 × 4), dann `FD 13` | `0x0B90`, dann `FD 13` |

### 1.2 Script-Nummer → Eintrag → Trainer (pret)

- **Pt:** `SCRIPT_RANGE_TABLE` lädt für 3000–4999 und 5000–6999 dieselbe Datei `scripts_battles`. `ScriptContext_LoadAndOffsetID` zieht 3000 bzw. 5000 ab, das Ergebnis ist der Eintrag. `Script_GetTrainerID` = `Nummer − 3000 + 1` bzw. `− 5000 + 1`.
- **HGSS:** `src/script_manager.c:59–60` (`_std_npc_trainer`, `_std_npc_trainer_2` → `scr_seq_0953`), `ScriptNumToTrainerNum` Z. 425–431 genauso.
- **Also:** Script `3000 + ID − 1` → Eintrag `ID − 1` → rotom-Slot `#ID`. Doppel-Partner `5000 + ID − 1` landet im **selben** Eintrag. Eine Erweiterung deckt beide Bereiche.
- **Sichtkontakt:** Das Spiel startet immer das Script „letzter Trainer“, die Trainer-ID kommt aus dem Objekt:
  - Pt `src/trainer_encounter.c:97/121`: `ScriptManager_Set(…, SCRIPT_ID(SINGLE_BATTLES, MAX_TRAINERS), …)` = 3928, danach `ScriptManager_SetApproachingTrainer(…, trainerID, …)`.
  - HGSS `asm/unk_020632B0.s`, `TryGetSeenByNpcTrainers`: Literal `_020641E8: .word std_trainer_approach` = 3739 (`include/constants/std_script.h:83`, `LAST_TRAINER_INDEX 740`), danach `FieldSystem_SetEngagedTrainer`.
  - Beide Funktionen liegen im arm9 (Pt `main.lsf.in`: `src/trainer_encounter.c` vor dem ersten `Overlay`; HGSS-Adresse `0x020641E8`). Die Nummer zu verschieben wäre ein arm9-Patch.
- **Folge:**
  - Für die **Sichtlinie** braucht ein neuer Trainer **keinen** Header-Eintrag: Pt 928 und HGSS 738 kämpften per Sicht, und 930/741 würden das auch ohne Erweiterung tun.
  - Den eigenen Eintrag braucht nur das **Ansprechen**, auch nach dem Sieg für Text 2.

### 1.3 rotom-Quelle

Nach `adopt_scripts` (Pt 1124 Dateien, HGSS 965, keine Fehler):

```
script script_1 #[1-928]:        // HGSS: #[1-739]
    PlaySE 1500
    LockAll
    FacePlayer
    GetTrainerID VAR_0x8004       // HGSS: GetTrainerNum VAR_SPECIAL_x8004
    …
script script_929 #929:           // HGSS: script_740 #740
    LockAll                       // HGSS: WaitFollowingPokemonMovement, LockAll, …
    GetApproachingTrainerType VAR_RESULT   // HGSS: GetTrainerEyeType
    …
```

- **Kampf-Eintrag:** ein Slot in der Liste von `script_1`. Alle Slots teilen einen Code; die Trainer-ID rechnet `GetTrainerID` aus der Script-Nummer.
- **„gesehen“-Eintrag:** genau ein eigenes Script, Slot = letzter Kampf-Slot + 1. Es liest Trainer-ID und Art aus dem Script-Manager (`GetApproachingTrainerID 0` / HGSS `GetEyeTrainerNum 0`), nicht aus der Script-Nummer.
- **Warum Ansprechen einfriert:**
  - Der Script-Manager wird bei jedem Start genullt (Pt `ScriptManager_New`: `memset(scriptManager, 0, …)`; HG-pret `ScriptEnvironment_New`, Z. 150).
  - Beim Ansprechen ist deshalb Art 0 (einzeln), Trainer 0, Objekt `NULL`. `StartApproachingTrainerTask 0` startet ohne Objekt, und die Schleife `CheckIsApproachingTrainerTaskDone` endet nie (Pt `src/scrcmd_trainer.c:28–94`).
- **Lücken:** Ein Slot ohne Script (geprüft mit `#[1-928, 932]`) kompiliert mit Warnung „Script slot #930 is empty; the next available script's pointer will be reused“. Die Lücke zeigt dann auf den nächsten Code, hier den Kampf-Code. Für `create_trainer` trotzdem lückenlos schreiben.
- **Roundtrip:** `save_script` mit der unveränderten Quelle ergibt Pt 1114 byte-gleich.

## 2. Verfahren

### 2.1 Was im Studio-Modell funktioniert (gemessen)

**rotom-Quelle bearbeiten und über den Kernel-Compile schreiben:**
- `call.sh save_script '{"file":1114,"text":…}'` mit der geänderten Kopfzeile meldet `{"problems":[]}`.
- Ergebnis Pt: 930 Einträge, Eintrag 928 → „gesehen“ (`0x1058`), Eintrag 929 → Kampf-Code (`0x0E8A`). Code ab dem Header-Ende byte-gleich mit Vanilla, Datei 4712 → 4716 Byte.
- HGSS: 741 Einträge, 739 → „gesehen“, 740 → Kampf-Code, Code byte-gleich.

**Binär anhängen** geht auch, für ein Projekt ohne übernommene Scripts:
- *k* neue `i32` vor `FD 13` einfügen.
- Jeden bestehenden Eintrag um `4·k` erhöhen: Seine Position bleibt, sein Ziel rückt um `4·k`.
- Neuer Eintrag *j*: `rel = Kampf-Code-neu − (4·j + 4)`.
- Sprünge im Code sind relativ und bleiben gültig.
- Nicht gebaut. Im Studio nicht nötig, weil Scripts vor jedem Bearbeiten übernommen werden.

### 2.2 Schritte, die `create_trainer(id)` im Kernel ausführen müsste

Erst prüfen und rechnen, dann alles in einem Changeset (sonst kann ein halber Trainer liegen bleiben):

1. **Kampf-Datei finden:** die Datei, die der Script-Bereich 3000 lädt (`common_scripts()` im Kernel, uxie-Tabelle). Pt 1114, HGSS 953. Nicht hart kodieren.
2. **Quelle lesen** (`read_source`). Ist sie binär, abbrechen mit „Scripts erst übernehmen“; der Binärweg aus 2.1 ist nur die Reserve.
3. **Kopf auswerten:**
   - `S_kampf` = das Script mit Slot 1.
   - `A` = der Slot direkt hinter dem ersten zusammenhängenden Bereich von `S_kampf`, der zu einem anderen Script gehört. Vanilla Pt 929, HGSS 740.
   - `A` muss gleich `arm9-Konstante − 2999` sein (Pt 3928, HGSS 3739). Weicht es ab, abbrechen: Dann hat ein Hack schon umgebaut.
   - `max` = höchster Slot von `S_kampf`.
4. **Erlaubte ID:**
   - 1 bis `A − 1` brauchen keine Änderung.
   - `A` ist gesperrt (2.3).
   - `A + 1` bis 1039 gehen mit Erweiterung; 1039 ist die Flag-Grenze aus dem Trainer-Bericht §4.
5. **Kopf erweitern:** Für `id > max` (und `id > A`) den Bereich `A+1 … id` in die Slot-Liste von `S_kampf` schreiben. Bereiche zusammenfassen: `#[1-928, 930-N]`.
   - Die neuen Header-Einträge zeigen dann auf den Kampf-Code, Lücken bis `id` eingeschlossen.
   - Keine Lücke offen lassen: Für Slots ohne Script nimmt rotom stillschweigend den nächsten Code.
6. **Schreiben und kompilieren** (`write_source` + `compile`). Fehler oder Warnungen zu dieser Datei → alte Quelle zurück, wie `add_script_reported` es schon tut.
   - Danach prüfen: der Header hat `max(id, A)` Einträge, Slot `A` zeigt weiter auf den „gesehen“-Code.
7. **Daten anlegen**, wie im Trainer-Bericht §3/§8 und hier gemessen:
   - `trainerProperties/NNNN` (20 Byte) und `trainerParty/NNNN` lückenlos bis `id`. Die ID `A` bekommt einen Platzhalter, damit die Member-Zahl stimmt.
   - Name in Archiv 618 bzw. 729 (`{TRAINER_NAME:…}`).
   - Texte `{id,0}`, `{id,1}`, `{id,2}` ans Ende von `trtbl`, drei Nachrichten ans Ende von 617 bzw. 728.
   - `trtblofs` bis `id` auffüllen (Lücken 0, `id` = alte Größe von `trtbl`).
8. **Event:** Objekt mit Script `3000 + id − 1`, `trainer_type` und `data[0]`. Die beiden fehlen in `EventPatch`/`ObjectDto` und wurden hier wieder binär gesetzt.

### 2.3 Die ID `A` (Pt 929, HGSS 740)

| Weg | Was | Kosten | Status |
|---|---|---|---|
| Auslassen (empfohlen) | Platzhalter-Datensatz, im Editor nie vergeben; Feld „Trainer“ im Event-Inspector schließt sie aus | eine tote ID je Projekt | abgeleitet + gemessen (Softlock ohne Wächter in beiden Spielen) |
| Wächter im „gesehen“-Script | am Anfang `GetApproachingTrainerID 0, VAR_RESULT` / `CompareVarToValue VAR_RESULT, 0` / `GoToIf EQUAL, label_talk` (HGSS `GetEyeTrainerNum 0, VAR_SPECIAL_RESULT`); `label_talk` = Anfang des Kampf-Codes (`script_1` beginnt dafür mit `GoTo label_talk` und dem Label) | Pt +28, HGSS +28 Byte; ändert Vanilla-Code, den Hacks schon angepasst haben können | gemessen: Ansprechen kämpft, Sicht unverändert |
| arm9-Patch | Literal 3928 (Pt) bzw. `0x020641E8` (HGSS US, DE nicht gesucht) auf einen freien Slot | Patch je Sprache | nicht nötig, nicht gebaut |

Warum der Wächter genügt:
- Beim Sichtkontakt setzt das Spiel die Trainer-ID ≠ 0, bevor das Script läuft. Der Task startet erst im nächsten Bild.
- Beim Ansprechen ist sie 0, der Manager ist genullt (1.3). Trainer 0 ist der leere Dummy und steht nie in einem Objekt.
- Der Kampf-Code rechnet die ID aus der Nummer 3928 bzw. 3739 → 929 bzw. 740.

## 3. Messprotokoll

### 3.1 Aufbau

**Platin** (`pt-3b3b-trscript`):

| Trainer | Klasse | Party | Name | Texte (`trtbl`) | Objekt in Header 411 (Eventdatei 390) |
|---|---|---|---|---|---|
| 928 | 2 Teenager | Lucario L7 | Jens | 2497–2499 | Index 8, (116, 888), Script 3927, gfx 4, Blick Nord, `trainer_type` 1, Sichtweite 1 |
| 929 | 3 Göre | Guardevoir L9 | Mia | 2500–2502 | Index 9, (113, 887), Script 3928, gfx 6, Typ 0 |
| 930 | 2 Teenager | Pikachu L5 | Lars | 2503–2505 | Index 10, (119, 887), Script 3929, gfx 4, Typ 0 |

- Eigenschaften `00 KK 00 01 | 0×8 | AI 1 | Kampfart 0`, Party ein Eintrag `00 00 LL 00 SS SS 00 00`.
- `trtblofs` 928 → 931 Einträge (9988, 10000, 10012).
- Scriptdatei 1114 mit Erweiterung `#[1-928, 930]` und Wächter.
- Gebaut: `call.sh build_rom '{"out":"…/pt-3b3b-trscript/out.nds"}'` packt `trainerProperties` und `trainerParty` mit 931 Membern.

**HeartGold** (`hg-3b3b-trscript`):

| Trainer | Party | Name | Objekt in Header 60 (Eventdatei 57) |
|---|---|---|---|
| 738 | Lucario L7 | Jens | Index 10, (695, 399), Script 3737, gfx 317, `trainer_type` 1, Sichtweite 1 |
| 739 | Guardevoir L9 | Mia | – (nur Daten) |
| 740 | Pikachu L5 | Lea | Index 11, (692, 398), Script 3739, gfx 319, Typ 0 |
| 741 | Evoli L6 | Lars | Index 12, (698, 398), Script 3740, gfx 317, Typ 0 |

- `trtblofs` 735 → 742 (735–737 = 0, dann 6868, 6880, 6892, 6904).
- Zwei ROMs:
  - `out-noguard.nds`: nur `#[1-739, 741]`.
  - `out.nds`: zusätzlich Wächter.

**Ort und Ablauf:** HGSS vor dem Haus statt im Erdgeschoss, weil das Erdgeschoss eng ist. Beim ersten Austreten läuft das Lyra-Ereignis; der Spieler ist etwa 2000 Bilder gesperrt. Erst nach `wait 300` und 25 × `press a:4 wait 60` ist er frei (Zustand `frei.state`). Eine Wartezeit, kein Einfrieren: Der Vergleichslauf auf derselben ROM kämpfte danach gegen 741.

### 3.2 Befehle

Aus `scripts/g4emu`, `R=../../../Roms/_testkopien`, `O=../../target/agent/trscript`.

```sh
# Pt: hinaus nach 411 (Mutter-Dialog an der Tür mit 18 × A)
uv run g4emu run --rom $R/pt-3b3b-trscript/out.nds --save $R/_saves/CPUD-zimmer.sav --out $O/pt-out \
  continue expect-map 415 walk-to 8 4 walk right 1 expect-map 414 wait 150 walk down 2 walk-to 6 10 \
  <18 × press a:4 wait 40> walk down 1 expect-map 411 wait 150 where shot draussen state draussen dump-ram ow
# HGSS: hinaus nach 60, Lyra-Ereignis durchklicken
uv run g4emu run --rom $R/hg-3b3b-trscript/out.nds --save $R/_saves/IPKD-zimmer.sav --out $O/hg-out-b \
  continue expect-map 64 walk left 1 walk down 1 expect-map 63 wait 150 walk down 3 walk-to 3 10 \
  walk down 1 expect-map 60 wait 200 wait 300 <25 × press a:4 wait 60> wait 200 where state frei
```

Je Trainer ein Lauf vom gespeicherten Zustand. Das Hilfsskript macht:
- Annäherung,
- `press a`, Bilder `t0`–`t3`,
- `dump-ram b1` (+300 Bilder), `b2` (+300), 30 × A, `b3`, 25 × A,
- `where`, `dump-ram end`.

Die Auswertung prüft Pt über den Zeiger `u32 @ 0x02101F88` (Gegner `P − 0x8DD8`) und beide Spiele über die Signatur (Art +0x00, acht Bytes `06` ab +0x18, Level +0x34).

| Fall | Annäherung | Erwartet | Gemessen | Bilder |
|---|---|---|---|---|
| Pt 928, Sicht (läuft durch den bewachten „gesehen“-Code) | `walk down 1` | Kampf | „!“, „Ich bin Jens, Trainer 928!“, „Eine Herausforderung von Teenage[r Jens]“. Zeiger `0x022CE738`, Gegner (448, 7), Signatur `0x022C5960` (448, 7). Niederlage („AAAAA hat kein kampffähiges Pokémon mehr!“), Mutter in 414 (8, 8), Zeiger 0, Signatur leer | `pt-t928/strip.png`, `strip2.png`, `strip3.png` |
| **Pt 930, Ansprechen (Header-Erweiterung)** | `walk right 3`, `press down:8` | Kampf | „Ich bin Lars, Trainer 930!“, „Herausforderung von Teenager Lar[s]“, Pikachu L5. `b1`–`b3`: Zeiger ≠ 0, Gegner (25, 5), Signatur (25, 5). Ende in 414 (8, 8), Zeiger 0 | `pt-t930/strip.png` |
| Pt 929, Ansprechen, mit Wächter | `walk left 3`, `press down:8` | Kampf | „Ich bin Mia, Trainer 929!“, „Herausforderung von Göre Mia!“, Guardevoir L9. Gegner (282, 9). Ende in 414, Zeiger 0 | `pt-t929/strip.png` |
| **HGSS 741, Ansprechen (Erweiterung, ohne Wächter)** | von `frei.state` (696, 397): `walk right 2`, `press down:8` | Kampf | „Ich bin Lars, Trainer 741!“, „Herausforderung von Teenager Lar[s]“, Evoli L6. Signatur `0x022C60E4` (133, 6) in `b1`–`b3`, leer am Ende. Ende in 63 (6, 8) bei der Mutter | `hg-t741/strip.png` |
| HGSS 740, Ansprechen, **ohne** Wächter | `walk left 4`, `press down:8`, `press a` | Softlock wie Pt 929 | kein Text, kein Kampf, Signatur leer. `press x` öffnet kein Menü, `press up:20` bewegt nicht (bleibt 692, 397) | `hg-t740-noguard/strip.png` |
| HGSS 740, Ansprechen, mit Wächter | von `hg-out-b/frei.state` (695, 397): `walk left 3`, `press down:8` | Kampf | „Ich bin Lea, Trainer 740!“, „Herausforderung von Göre Lea!“, Pikachu L5, Signatur (25, 5). Ende in 63 (6, 8) | `hg-t740/strip.png` |
| HGSS 738, Sicht, mit Wächter | `walk down 1` | Kampf | „!“, „Ich bin Jens, Trainer 738!“, Lucario L7 (Signatur in `b3`; in `b1`/`b2` lief noch der Text). Ende in 63 (6, 8) | `hg-t738/strip.png` |

- Alle Bildpfade unter `target/agent/trscript/`.
- „Kampf beendet, kein Einfrieren“ heißt hier: Niederlage ohne Team, Rückkehr zur Mutter, `where` liefert die Hausposition, der Pt-Zeiger ist 0, die Signatur leer.
- Ein Kampf ohne Team läuft durch: Der Spieler-BattleMon ist leer, die Meldung kommt nach der ersten Runde.

**Nicht beobachtet:**
- Trainer-ID über `P − 0x255F0` (Pt): liefert ohne Team 11 statt der ID. Die Lage verschiebt sich ohne Team, der Abstand aus dem Trainer-Bericht gilt nur mit Team. Die ID ist hier über den Kampftext belegt.
- Text 2 nach einem **Sieg** über 930/741: Ohne Team gibt es keinen Sieg.

## 4. Geprüft, abgeleitet, offen

| Punkt | Status | Beleg |
|---|---|---|
| Header-Format, keine Grenzprüfung beim Sprung | geprüft (pret + ROM) | 1.1 |
| Script `3000 + ID − 1` → Eintrag `ID − 1` → rotom `#ID`; 5000+ derselbe Eintrag | geprüft (pret, ROM) | 1.2 |
| Sichtkontakt nutzt nur den „gesehen“-Eintrag; seine Nummer steht im arm9 | geprüft (pret) | 1.2 |
| rotom `#[1-928, 930]` / `#[1-739, 741]` über `save_script`: Header +1, Code byte-gleich | geprüft | 2.1 |
| Pt 930 und HGSS 741 kämpfen beim Ansprechen nach der Erweiterung | geprüft (Emulator, Text + RAM) | 3.2 |
| Pt 929 / HGSS 740 frieren ohne Wächter ein | geprüft (Pt im Trainer-Bericht, HGSS hier) | 3.2 |
| Wächter: 929/740 kämpfen beim Ansprechen, Sicht über den bewachten Code geht weiter | geprüft (Pt 928/929, HGSS 738/740) | 3.2 |
| Wächter bei Doppel-Sichtkontakt (zwei Trainer) | abgeleitet: Der Wächter liest nur Trainer 0, der beim Sichtkontakt immer gesetzt ist | – |
| Slot-Lücke: rotom füllt mit dem nächsten Code und warnt | geprüft (Compile) | 1.3 |
| Binär anhängen (Offsets + 4·k) | abgeleitet, nicht gebaut | 2.1 |
| Doppel-Partner 5000+ für neue IDs | abgeleitet (gleicher Eintrag), nicht gespielt | 1.2 |
| Text 2 nach Sieg über einen erweiterten Trainer | offen (Spielstand ohne Team; braucht einen Geber wie im Trainer-Bericht) | – |
| HGSS US (IPKE) | offen, gleiche Datei 953 erwartet | – |
| Literal-Adresse in HGSS DE für einen arm9-Patch | offen, nicht nötig | 2.3 |

## 5. Empfehlung für den Plan

- **Erweiterung in `create_trainer` einbauen**, nicht das Gate senken. Der Weg ist kurz und gemessen:
  - eine Kopfzeile der rotom-Quelle ändern, über den vorhandenen Compile schreiben, prüfen;
  - keine neue Binärlogik, kein arm9-Patch.
- **Schritte** wie in 2.2: Kampf-Datei aus der Script-Tabelle, `A` aus dem Kopf lesen und gegen die arm9-Konstante prüfen, Bereich lückenlos erweitern, alte Quelle bei Fehlern zurück.
- **Die ID `A` auslassen** (Platzhalter-Datensatz, im Editor gesperrt). Den Wächter nicht automatisch einbauen: Er ändert Vanilla-Code im „gesehen“-Script, den Hacks verändert haben können. Als spätere Option „Lücke bei 929 schließen“ ist er belegt.
- **Gate 3b.3b bleibt „zwei neue Trainer je Spiel“:**
  - Pt: 928 (ohne Erweiterung) und 930 (mit).
  - HGSS: 738 und 739 gehen ohne Erweiterung. Für den Nachweis der Erweiterung besser 738 und 741.
  - Mindestens ein Trainer je Spiel beim **Ansprechen** prüfen. Über die Sichtlinie läuft jeder Trainer auch ohne eigenen Header-Eintrag und beweist die Erweiterung nicht.
- **Brücke:** `trainer_type` und `data[0..2]` in `EventPatch`/`ObjectDto` sind weiterhin Voraussetzung (hier wieder binär gesetzt).
- **Gate-Ablauf:**
  - Spielstand ohne Team reicht für „Kampf erkannt und beendet“. Für Text 2 nach dem Sieg braucht es einen Geber.
  - HGSS: Das Lyra-Ereignis vor dem Haus einplanen (25 × A), oder die Figuren ins Erdgeschoss 63 stellen.
