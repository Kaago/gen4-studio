# Phasen-Gate 3d: Erzählen

Stand: 2026-11-04. Gate-Lauf des Agenten auf main `fbbf9d9f` (3d.1 bis 3d.9, 4.5 und der Fix U5 gemergt) ohne Codeänderung; dazu die Szenarien `pt-geschichte-gate-buehne.yaml` und `pt-geschichte-gate.yaml` und diese Dokumentation. Danach `scripts/gate.sh` auf main durch den Koordinator (vier Läufe, zwei Fixes: die Story-Kette nach Undo eines eigenen Flag-Namens, Merge `68c98254`, und der Archivtest auf Vanilla Platin, `fcdb8eae`); Endstand main `fcdb8eae`. **Vom Agenten bestanden (2026-11-04): (A) die Szenarien `pt-geschichte` und `hg-geschichte` kopflos, (B) die kleine Geschichte komplett in der Tiefe Erzählen über die Test-Umgebung auf Platin DE, gebaut, geprüft und in `g4emu` gespielt.** **`scripts/gate.sh` auf main: grün auf `fcdb8eae` (Lauf 4 mit Wiederholung des Platin-Schritts und dem Rest, Abschnitt unten).** Offen sind Hand, Blick und die `.fig`-Frames beim Nutzer.

Phase 3d bringt das Erzählen: Flags und Variablen mit Namen und Nutzungsindex (3d.1), die Tiefe Erzählen mit Rezept-Rahmen und Entwurf-Schicht (3d.2), Kampfsprites, DS-Profil, Kampfvorschau und Ei-Attacken (3d.3), Löschen mit Referenzsperre, Grabstein und „Nummer freigeben“ (3d.4), Figur, Dialog und Tür (3d.5), Geschichten lesen (3d.6) und erzählen (3d.7), die Erzählen-Layouts mit dem Assistenten „Neuer Trainer“ (3d.8) und den Spielmodus live (3d.9); dazu die Zwischenphase 4.5 Veröffentlichen, die nicht zum Gate gehört. Die Teilphasen haben je ein eigenes Gate (Links unten); dieses Gate prüft, dass sie zusammen eine Geschichte tragen.

Roadmap: [docs/ROADMAP.md](ROADMAP.md), Abschnitt „Phase 3d“, Absatz „Gate der Phase“. Spezifikation: [2026-10-30-phase-3d-erzaehlen-design.md](superpowers/specs/2026-10-30-phase-3d-erzaehlen-design.md), §4 (Absatz „Phasen-Gate 3d“) und §8. Befehle: [docs/agent-gate.md](agent-gate.md), Abschnitt „Phasen-Gate 3d“.

Gate-Text der Spezifikation (§4):

> Eine kleine Geschichte entsteht komplett in der Tiefe Erzählen: ein neuer Trainer per Assistent mit Kampfsprite in der Vorschau, ein eigenes Flag, eine Geschichte mit drei Schritten (Gespräch schaltet weiter, Figur erscheint ab Schritt, Kampf, Belohnung einmalig), dazu ein angelegter und wieder gelöschter Entwurf (die Referenzsperre greift vorher, danach Grabstein und „Nummer freigeben“). Gebaut und in `g4emu` gespielt: Figur A schaltet die Variable weiter (`expect-var`), Figur B erscheint, der Kampf wird gewonnen, die Belohnung kommt einmal (`expect-flag`), der zweite Besuch verläuft anders. Das Textprojekt liegt in Git mit einem Diff, der nur diese Arbeit zeigt; `vanilla_checks` 0 Fehler; `scripts/gate.sh` grün.

Wer was prüft: **Agent** (Test-Umgebung mit `studio-bridge`, `call.sh`, Chrome headless über CDP, `g4k`, `studio mcp` kopflos, `g4emu`), **Agent + Blick** (der Agent prüft und beschreibt, der Nutzer bestätigt am Bild), **Hand** (der Nutzer), **Koordinator** (`scripts/gate.sh` allein auf main).

## Vorbereitung

1. **Kopien** unter `/Users/POLTEP/Projekte/lab/editor/Roms/_testkopien/` per `cp -cR` (Ordner) und `cp -c` (`.nds`) aus den Originalen, die dabei nur gelesen werden: `3d-gate-szen-pt_DSPRE_contents` + `3d-gate-szen-pt.nds` (Platin DE) und `3d-gate-szen-hg_DSPRE_contents` + `3d-gate-szen-hg.nds` (HeartGold DE) für (A), `3d-gate-web_DSPRE_contents` + `3d-gate-web.nds` (Platin DE) für (B). Der Ordner von (B) trägt den Zusatz `_DSPRE_contents`, weil `studio mcp` nur die `.nds` neben einem `*_DSPRE_contents`-Ordner zulässt (Befund B3). Alle `3d-gate-*` nach dem Gate gelöscht; `Manual` und `_saves` bleiben.
2. **Werkzeuge:** `cargo build --release -p g4-kernel --bin g4k` und `cargo build --release -p studio --bin studio` (57 s); `G4_ROMS_DIR` absolut.
3. **Bühne von (B)** kopflos: `pt-geschichte-gate-buehne.yaml` über `studio mcp` (Textprojekt, neuer Header 593 aus den Route-201-Karten 5 und 6, neue Scriptdatei 1124 und neues Textarchiv 724, das Geschenk auf (45, 20) mit Level 100, die Treppe des Zimmers 415 ↔ neue Tür (44, 21)), dann `g4k git init` („Ausgangsstand“, 6 592 Dateien, `git status` leer); 86 s, Last 12,7 → 10,4. Die Bühne ist kein Teil der Geschichte: der Spielstand `_saves/CPUD-zimmer.sav` hat kein Pokémon, der Kampf braucht eines, und ein Vanilla-Ort trüge fremde Ereignisse (Entscheidung E1).
4. **Test-Umgebung** auf **1800/1801** (`G4_ROMS_DIR=/Users/POLTEP/Projekte/lab/editor/Roms G4_BASE_ROM_DIR=/Users/POLTEP/Projekte/lab/editor/Roms/_testkopien G4_VITE_PORT=1800 G4_BRIDGE_PORT=1801 bash scripts/agent/studio-web.sh`), Seite `http://127.0.0.1:1800/?project=<Roms>/_testkopien/3d-gate-web_DSPRE_contents`, Chrome headless mit eigenem Profil unter `target/agent/3d-gate/chrome-prof` (1440 × 900, `--use-angle=swiftshader`, über `--remote-debugging-pipe`), Hell und Dunkel über `Emulation.setEmulatedMedia`. Nach dem Gate Chrome, Brücke und Vite über ihre PIDs beendet, beide Ports frei.
5. **Ausgaben** (nicht im Repo): `target/agent/gate-2026-11-04/3d-gate/{pt-geschichte,hg-geschichte,buehne,browser,gespielt}/`, Logs `target/agent/3d-gate/*.log`.
6. **Last:** Microsoft Defender und die parallelen Szenarien; Lastmittel 16,6 beim Bauen, 14,6 → 9,2 während (A), 2,6 beim Spielen von (B). Keine Zeitgrenze in diesem Gate.

## (A) Szenarien kopflos

Ergebnis: **bestanden** am 2026-11-04 auf Platin DE und HeartGold DE, beide parallel: Rückgabe 0, Schlusszeile `{"scenario": "pt-geschichte", "ok": true, "headers": [415, 593, 594, 593], …}` bzw. `hg-geschichte` mit `[64, 540, 541, 540]`, je **22** Zeilen „Erwartung erfüllt“; Platin 182 s, HeartGold 188 s **unter Last** (14,6 → 9,7). Befehle und Platzhalter wie unter „Phase 3d.7“ in [agent-gate.md](agent-gate.md), mit den Kopien `3d-gate-szen-*`.

| Punkt | Platin DE | HeartGold DE |
|---|---|---|
| Geschichte über MCP, Variable aus dem Pool | `VAR_STORY_DER_BRIEF` = 0x4031 (16433), `story_get` 5 Schritte | 0x4034 (16436), 5 Schritte |
| Pool: eine Variable, zwei Flags weniger | Variablen 20 → 19, Flags 101 → 99 | 81 → 80, 154 → 152 |
| `run_checks` | 0 Fehler (111: 101 Warnungen, 10 Hinweise) | 0 Fehler (145: 133 Warnungen, 12 Hinweise) |
| `check.log` | `schema=current`, `differs=0`, `3 expectation(s) met` | ebenso |
| B vorher frei, A schaltet weiter | V 0, Flag 66 = 1, `walk-until-blocked` 4 Schritte bis (54, 17); A → V 1 | V 0, Flag 64 = 1, 3 Schritte bis (15, 13); A → V 1 |
| Szene, B erscheint, Kampf | Raum 594 → V 2; Flag 66 = 0, Halt nach 1 Schritt, `expect-battle 25 12`, `win-battle` | Raum 541 → V 2; Flag 64 = 0, `expect-battle 25 12`, `win-battle` |
| Belohnung einmal, zweiter Besuch anders | Flag 67 = 1, V 2; noch einmal Flag 67 = 1, As eigener Text | Flag 65 = 1; ebenso |

Bilder und Logs unter `target/agent/gate-2026-11-04/3d-gate/{pt,hg}-geschichte/`: `geschenk.png`, `gespraech.png`, `szene.png`, `kampf.png`, `belohnung.png` („Danke! Nimm das.“), `belohnung-2.png` („Hallo, Reisender!“), `last.png`; `check.log`, `play.jsonl`, `results.json`, `mcp.jsonl`. HeartGold US: nicht gelaufen (optional; IPKE hat keinen gemessenen Kampfzeiger, siehe 3b-Gesamtgate).

## (B) Browser: die Geschichte in der Tiefe Erzählen

Ergebnis: **bestanden** am 2026-11-04 auf Platin DE (`3d-gate-web_DSPRE_contents`), alle sieben Punkte; die Arbeit über Dialoge und Assistenten in Erzählen (Segment „Tiefe“), `call.sh` nur zum Nachprüfen. HeartGold DE im Browser: nicht gelaufen (das Spiel beweist (A) auf HeartGold).

- [x] 1. **Neuer Trainer per Assistent mit Kampfsprite in der Vorschau (3d.8):** Pokémon › Trainer (⌘2) › „Neuer Trainer…“ `[data-new-trainer-recipe]` öffnet „Neuer Trainer hier“ in der Welt mit dem gewählten Ort. Klasse „Wanderer“, Name „Jens“, Team PIKACHU Lv. 12, Texte „Halt! Ich bin Jens!“ (endet auf ▼, Umschalt+Enter; ohne ▼ bleibt „Weiter“ gesperrt mit dem Satz dazu), „Du bist stark!“, „Gut gekämpft.“, Wo: Route 201 (Header 593), Kachel x 54, z 17, Blick Unten, Sichtweite 3; „Aufstellen“ → „Trainer aufgestellt“: Trainer 0928 (bleibt nach Rückgängig), Objekt 2 in Eventdatei 0534, Kampf-Script 3927, Kampfvorschau „Wanderer Jens gegen PIKACHU Lv. 12“. Die Vorschau `[data-trainer-preview]` zeigt das Klassenbild des Wanderers und in der Kampfvorschau `[data-foe]` das Kampfsprite PIKACHU (80 × 80) mit Statuskasten „PIKACHU Lv. 12“. `get_events 534`: `(54, 17)`, `script 3927`, `trainer_type 1`, `sight 3`. Vorher ebenso in Erzählen über die Rezeptleiste „Neue Figur“: Figur A, Objekt 1 auf (48, 19), „Hallo, Reisender!“, Script #3, Nachricht 2 in Archiv 0724.
- [x] 2. **Ein eigenes Flag (3d.1):** Geschichten › Flags und Variablen, „nur frei“, Zeile 66 (`FLAG_UNK_0x0042`), „Als eigenes Flag benennen…“ `[data-name-flag-dialog]` → `FLAG_BRIEF_GELESEN` mit Beschreibung; Meldung „FLAG_BRIEF_GELESEN benannt (ein Schritt im Verlauf).“; auf der Platte `data/ids/flags.json`, `data/flags/0066.json` und `.rotom/command_database/.g4-ids-flags.json`. Der Pool zählt das benannte Flag nicht mehr: 101 → 100 frei, `free_flag` gibt danach 67 statt 66.
- [x] 3. **Geschichte mit drei Schritten (3d.7):** „Neue Geschichte…“ mit `[data-pool-free]` „Noch 20 Variablen frei“, Feld vorbelegt `VAR_UNUSED_0x4031`; „Der Brief“ → Toast „Geschichte „Der Brief“ angelegt“, `VAR_STORY_DER_BRIEF` = 16433, Untertitel „1 Geschichte · 19 Variablen frei“. Dann über „Schritt anlegen“, je Plan im Dialog gleich `recipe_plan` über `call.sh`:
  - „Gespräch schaltet weiter“: Ort Route 201 (0593), Figur 1, ab Schritt 0, „Bring den Brief zu Jens!“; Plan „Figur 1 bekommt den Zweig „ab Schritt 0“ und schaltet auf 1.“, „Script #4 in Scriptdatei 1124, Nachricht 3 in Archiv 724.“; der Trainer steht in der Figurenwahl gesperrt („Trainer sprechen über ihre Kampftexte“). Schritt `s1` dialog.
  - „Figur erscheint/verschwindet ab Schritt“: Figur 2 (Jens), Schritt 1, erscheint, Flag 0x0043 aus dem Pool; Plan „Figur 2 bekommt das Sichtbarkeits-Flag FLAG_STORY_DER_BRIEF_SICHTBAR_2 und erscheint ab Schritt 1.“, „Block sichtbarkeit_0_593 in Scriptdatei 1124.“ und in `[data-story-recipe-irreversible]` „Level-Script-Datei 1125 wird angelegt und bleibt“ (Header 593 teilt Level-Script 909 mit seiner Vorlage 342). Schritt `a67` appearance.
  - Der Kampf ist Jens (Schritt „erscheint ab 1“, Kampf beim Blick).
  - „Belohnung einmalig“: Figur 1, ab Schritt 1, Trank (0017) über den Picker; der Text steht danach als Vorschlag „Du erhältst Trank!“ und „Anlegen“ ist frei (U1 aus 3d.7 bestätigt); Flag 0x0044, Plan „Figur 1 gibt ab Schritt 1 einmal Gegenstand 17.“, „Benennt Flag 0x0044 als FLAG_STORY_DER_BRIEF_BELOHNUNG_1.“, „Script #6 …, Nachricht 4 in Archiv 724.“. Schritt `r68` reward.

  `story_get`: `s0` start (0), `s1` dialog (1), `a67` appearance (1), `r68` reward (1), `flags` die beiden Story-Flags; `story_list.free_vars` 19; Flags 98 frei. Der Inspector von Jens nennt `[data-story-hint]` „Geschichte „Der Brief“: Figur 2 erscheint ab Schritt 1“. Jede Bearbeitung ein Schritt im Verlauf (acht bis hier).
- [x] 4. **Entwurf angelegt und wieder gelöscht (3d.4):** „Neuer Trainer hier“ aus der Rezeptleiste der Welt: Trainer 0930 „Entwurf“ ([PK][MN]-Trainer, BIDIZA Lv. 5) auf (38, 22), Objekt 3, Script 3929 (Platin legt davor den gesperrten Platzhalter 0929 des Sicht-Slots an). Pokémon › Trainer › 0930 › Aktionen › „Löschen…“: Dialog „Trainer 0930 löschen“, „1 Verweis: zuerst umhängen.“, `[data-reference]` „Eventdatei 0534, Objekt 3“, „Löschen“ gesperrt (= `plan_delete` `references` mit Eventdatei 534, Feld `script`, Platz 3). Ein Klick auf den Verweis führt in die Welt zu Objekt 3 (Inspector „Trainer: [PK][MN]-Trainer Entwurf 0930“); dort „…“ › „Löschen“, Rückfrage „Objekt 3 (x 38, z 22) löschen?“ › Ja. Danach `references` leer; „Löschen…“: „Inhalt und Member bleiben; ⌘Z holt den Eintrag zurück.“ → „Gelöscht: Trainer 0930“, Plakette `[data-entity-state=deleted]` „gelöscht“ im Kopf, `data/trainers/0930.json` mit `"deleted": true`, Verlauf + 1. Erneut „Löschen…“: „Trainer 0930 ist schon gelöscht.“, Kästchen „Nummer freigeben (nicht rückgängig)“ mit Plakette „Nicht rückgängig“ und „Entfernt 3 Dateien: data/trainers/0930.json, unpacked/trainerProperties/0930, unpacked/trainerParty/0930“, Taste „Löschen und Nummer freigeben“ → „Nummer freigegeben: Trainer 0930“: `git status` nennt `data/trainers/0930.json` nicht mehr (vorher `?? data/trainers/0929.json` und `?? data/trainers/0930.json`, danach nur 0929). Verlauf am Ende 13 Schritte.
- [x] 5. **Git zeigt nur diese Arbeit** (nach ⌘S, `call.sh dirty` `[]`), gegen „Ausgangsstand“: geändert `data/events/0534.json` (Figuren A, Jens), `data/headers/0593.json` (`level_script_id` 909 → 1125), `data/ids/flags.json`, `data/ids/stories.json`, `data/ids/trainers.json`, `data/ids/vars.json`, `data/trainer_text_offsets/0000.json`, `data/trainer_text_table/0000.json`, `expanded/scripts/1114.rotom`, `expanded/scripts/1124.rotom`, `expanded/textArchives/0617.json`, `0618.json`, `0724.json` (13 Dateien, 149 Zeilen dazu, 6 weg); neu `data/flags/0066.json`, `0067.json`, `0068.json`, `data/stories/0000.json`, `0000.layout.json`, `data/trainers/0928.json`, `0929.json`, `data/vars/16433.json`, `expanded/scripts/1125.json`. Kein Pfad außerhalb von Figur, Trainer, Flag, Geschichte und Entwurf; vom Entwurf bleiben Spuren (Befund B1). Liste: `target/agent/gate-2026-11-04/3d-gate/browser/git-status.txt`, `git-diff-stat.txt`.
- [x] 6. **Bauen und prüfen:** `g4k build` 4 717 ms (kalt), SHA-1 `e66443a5…` gleich dem Build über `studio mcp` (`build_rom`); `g4k check` 111 Befunde, **0 Fehler** (101 Warnungen, 10 Hinweise; dieselben wie Vanilla); `run_checks` über die Brücke 0 Fehler (111); `g4k text check` `schema=current`, `differs=0`, dazu die Warnung `textproject.id-dangling` für `TRAINER_PKMN_TRAINER_ENTWURF` (930, eingefroren, B1) neben den Vanilla-Warnungen `id-shadowed`; die Erwartungen `warp.dest-warp=0`, `event.script-missing=1` (Vanilla-Schild der Eventdatei 0044), `event.trainer-missing=0` erfüllt.
- [x] 7. **Gespielt:** `pt-geschichte-gate.yaml` (neu, nur Spielen: `mcp: []`, keine Änderungen, `studio mcp` speichert und baut, dann `check` und der Spielteil von `pt-geschichte` ohne Raum) mit `--set HEADER=593 VAR=16433 FLAG_B=67 FLAG_R=68 X_A=48 Z_A=19 X_B=54 Z_B=17 S_T=25 L_T=12`: Rückgabe 0, `"ok": true`, Header 415, 593, 415, 593; 27 s (Last 2,6). Ankunft, Geschenk; V 0 und Jens' Flag 67 = 1, `walk-until-blocked` 4 Schritte bis auf seine Kachel (54, 17); A → `gespraech.png` „Bring den Brief zu Jen…“ (Text noch im Lauf), V 1; durch die Ankunftstür ins Zimmer 415 und zurück: Flag 67 = 0, Jens sieht den Spieler nach 1 Schritt bei (54, 20), `expect-battle 25 12`, `kampf.png` „Eine Herausforderung von Wanderer Jens!“, `win-battle`; A → `belohnung.png` „Du erhältst Trank!“, Flag 68 = 1, V 1; noch einmal → `belohnung-2.png` „Hallo, Reisender!“ (As eigener Text, keine zweite Belohnung), Flag 68 weiter 1.

Bilder unter `target/agent/gate-2026-11-04/3d-gate/browser/`: die Schritte `s00` bis `s27` und die Blick-Bilder (Liste unter „Blick“), vom Agenten gelesen.

## Entscheidungen

| Nr. | Entscheidung | Grund |
|---|---|---|
| E1 | Die Bühne (Ort, Geschenk, Tür) entsteht kopflos vor dem „Ausgangsstand“, die Geschichte selbst (Figur A, Trainer, Flag, Geschichte, Schritte, Entwurf) im Browser in Erzählen. | Der Spielstand hat kein Pokémon, der Kampf braucht das Geschenk (ein `GivePokemon`-Script, kein Erzählen-Werkzeug); ein Vanilla-Ort trägt fremde Ereignisse. Der Diff gegen „Ausgangsstand“ zeigt so nur die Arbeit der Geschichte. |
| E2 | Drei Schritte ohne Raum: Gespräch (0 → 1), Erscheinen ab 1, Belohnung ab 1; der Kampf ist Jens. „Szene beim Betreten“ verlangt das Phasen-Gate nicht (sie läuft in (A)). | Ohne Raum geht der Spieler für das Wiedererscheinen durch die Ankunftstür ins Zimmer und zurück (das Übergangs-Script läuft beim Betreten). Ein Trainer kann kein Gespräch weiterschalten, darum kommt die Belohnung ab Schritt 1 und wird im Spiel nach dem Kampf abgeholt. |
| E3 | Gespielt über ein Szenario ohne Änderungen (`pt-geschichte-gate.yaml`, `mcp: []`, `edits: []`), die Zahlen aus dem Browser als `--set`. | `g4emu scenario` läuft mit leerer Änderungsliste: `studio mcp` speichert und baut, `check` und `play` laufen wie sonst. |

## Ergebnis

| Punkt | Wer | Ergebnis | Datum |
|---|---|---|---|
| (A) Szenario `pt-geschichte` (Platin DE) | Agent + Blick | bestanden: Rückgabe 0, `ok: true`, 22 × „Erwartung erfüllt“, 182 s unter Last | 2026-11-04 |
| (A) Szenario `hg-geschichte` (HeartGold DE) | Agent + Blick | bestanden: Rückgabe 0, `ok: true`, 22 × „Erwartung erfüllt“, 188 s unter Last | 2026-11-04 |
| (A) HeartGold US | Agent (optional) | nicht gelaufen | – |
| (B) 1 Neuer Trainer per Assistent, Kampfsprite in der Vorschau | Agent + Blick | bestanden: Trainer 0928 Jens, Objekt 2, Vorschau Klassenbild und PIKACHU Lv. 12 | 2026-11-04 |
| (B) 2 eigenes Flag | Agent | bestanden: `FLAG_BRIEF_GELESEN` (66), drei Dateien, Pool 101 → 100 | 2026-11-04 |
| (B) 3 Geschichte mit drei Schritten | Agent + Blick | bestanden: „Der Brief“, 4 Einträge (Start, Gespräch, Erscheinen, Belohnung), Pläne gleich `recipe_plan`, Pool Variablen 20 → 19, Flags 100 → 98 | 2026-11-04 |
| (B) 4 Entwurf: Sperre, Grabstein, „Nummer freigeben“ | Agent + Blick | bestanden: 1 Verweis sperrt, Weg zur Quelle, Plakette „gelöscht“, `0930.json` fort aus `git status`; Spuren B1 | 2026-11-04 |
| (B) 5 Diff nur dieser Arbeit | Agent | bestanden: 13 geänderte, 9 neue Dateien, alle aus der Geschichte und dem Entwurf | 2026-11-04 |
| (B) 6 `g4k build`, `g4k check`, `vanilla_checks` | Agent | bestanden: 0 Fehler (111 Befunde wie Vanilla), `differs=0` | 2026-11-04 |
| (B) 7 in `g4emu` gespielt | Agent + Blick | bestanden: `expect-var` 0 → 1, B erscheint, Kampf gewonnen, `expect-flag` Belohnung einmal, zweiter Besuch „Hallo, Reisender!“ | 2026-11-04 |
| `g4emu` Unit mit den neuen Szenarien | Agent | bestanden: `uv run pytest -m "not integration"` 394 passed, darunter `test_phase_gate_3d_scenarios_load` (Bühne und Spielen laden, Platzhalter, Erwartungen) | 2026-11-04 |
| `scripts/gate.sh` auf main | Koordinator | bestanden auf `fcdb8eae`: Lauf 4 bis Platin DE grün (gated HeartGold DE 5 875 s, HeartGold US 5 900 s), Platin DE rot an einem Last-Flake (`git::cli::tests::the_templates_are_refreshed_before_the_commit`, allein 5 von 5 grün), Wiederholung des Schritts und Rest (Timings, Projektstart) grün (4 994 s; Timings 119 / 73 / 50 s, Projektstart 85 / 76 / 70 s); davor Lauf 2 fand die Regression der Story-Kette (Fix `68c98254`), Lauf 3 den Archivtest auf Vanilla Platin (Fix `fcdb8eae`) | 2026-11-04 |
| Hand, Blick, `.fig` | Hand, Blick | offen | – |

### `scripts/gate.sh` auf main (Koordinator)

Vier Läufe am 2026-11-04, jeder allein auf main mit `G4_ROMS_DIR=/Users/POLTEP/Projekte/lab/editor/Roms bash scripts/gate.sh`; Logs unter `target/agent/gate-2026-11-04/3d-gate/gate-sh-lauf<N>-<Commit>.log` (nicht im Repo). Das Skript stoppt beim ersten roten Schritt; die gated Projekte laufen alphabetisch (HeartGold DE, HeartGold US, Platin DE). Die Last kam den ganzen Tag vor allem von Microsoft Defender (`wdavdaemon`, Load 7 bis 61); die Automatik-Schritte waren in jedem Lauf grün.

| Lauf | Commit | Ergebnis | Dauer | Roter Schritt | Einordnung |
|---|---|---|---|---|---|
| 1 | `762d7036` (3d.7 und 3d.6 Teil B gemergt) | rot | 5 730 s | gated HeartGold DE: `dynamic_headers::store_applies_the_patch_creates_a_header_and_builds`, NotFound einer frisch kopierten `unpacked/OWSprites/0310` | Last (Load bis 61); allein 3 von 3 grün in 49 s; kein Fix |
| 2 | `fd305d1a` (Gate-Doku des Agenten) | rot | 5 730 s | gated HeartGold DE: `recipes_story::vanilla_story_chain_is_five_undo_steps_and_git_status_is_empty_after_undo`, `Stale(Event 57)` beim `save()` nach dem Undo | Regression 3d.4 × 3d.7 (Befund B4); allein reproduzierbar; Fix `373d7c7e`, Merge `68c98254` |
| 3 | `68c98254` (Fix der Story-Kette) | rot | 16 711 s | gated Platin DE: `project::vanilla_every_archive_path_exists`, `files/zcustom/egg_moves.narc` fehlt | latent seit 3d.3 Teil B (Befund B5); Test-Fix `fcdb8eae`; davor gated HeartGold DE 6 440 s und HeartGold US 5 424 s grün |
| 4 | `fcdb8eae` (Test-Fix) | rot, in der Wiederholung grün | 15 395 s + 5 467 s | gated Platin DE: `git::cli::tests::the_templates_are_refreshed_before_the_commit`, `gix` „Could not create index from tree“ bei Load 18 bis 27 | Last-Flake (Befund B6): im selben Lauf dreimal grün, allein 5 von 5; Wiederholung des Schritts und Rest grün |

Lauf 4 (12:29 bis 16:45, `gate-sh-lauf4-fcdb8eae.log`):

| Schritt | Projekt | Ergebnis | Sekunden |
|---|---|---|---|
| `cargo fmt --check`, `cargo clippy (-D warnings)`, `cargo deny`, npm licenses | – | ok | 1, 6, 3, 0 |
| `cargo test` ohne ROM, mcp, `pnpm test` | – | ok | 2 133, 27, 143 |
| tokens, styles, depth, `pnpm build`, dist ohne Brücke | – | ok | 0, 4, 1, 25, 0 |
| g4emu unit, integration | – | ok | 16, 10 |
| gated `cargo test` | HeartGold DE, HeartGold US | ok | 5 875, 5 900 |
| gated `cargo test` | Platin DE | **rot** (B6) | 1 251 |

Wiederholung und Rest (16:48 bis 18:19, `gate-rest-fcdb8eae.log`, Skript `gate-rest.sh` daneben: dieselben Befehle wie in `scripts/gate.sh`, Zeilen 182 bis 184, 202 bis 206 und 232 bis 235):

| Schritt | Projekt | Ergebnis | Sekunden |
|---|---|---|---|
| gated `cargo test` (Wiederholung) | Platin DE | ok | 4 994 |
| build graph (timings), erster Versuch | HeartGold DE, HeartGold US, Platin DE | ok | 119, 73, 50 |
| gated Projektstart | HeartGold DE, HeartGold US, Platin DE | ok | 85, 76, 70 |

Build-Graph-Zeiten (Release, ein Thread, Grenzen Karte, Text, Script je ≤ 500 ms, ohne Änderung ≤ 200 ms): HeartGold DE Karte 268, Text 282, Script 335, ohne Änderung 95 ms; HeartGold US 244, 255, 322, 103 ms; Platin DE 239, 260, 307, 105 ms. Referenzindex warm 181, 166, 201 ms (Grenze 1 000 ms, Ziel 300 ms). Damit ist `scripts/gate.sh` auf main `fcdb8eae` in jedem Schritt grün, der Platin-Schritt in der Wiederholung.

## Befunde

| Nr. | Schwere | Punkt | Befund | Status |
|---|---|---|---|---|
| B1 | Klein | (B) 4, 5 | „Nummer freigeben“ des Entwurfs 0930 entfernt seine drei Dateien, aber drei Spuren bleiben im Diff: der Kampf-Slot `930` in der geteilten Scriptdatei 1114 (`script script_1 #[1-928, 930]`), der Registereintrag `TRAINER_PKMN_TRAINER_ENTWURF: 930` in `data/ids/trainers.json` (das Register ist eingefroren, 4.3a; `g4k text check` warnt `textproject.id-dangling … die ID bleibt eingefroren`) und der gesperrte Platzhalter 0929 des Sicht-Slots (`data/trainers/0929.json`, Name „-“ in Archiv 0618, `TRAINER_0929`), den der Assistent vor 0930 angelegt hat. Ursache: `delete::free_number` kürzt für Trainer nur Namensbank und `trtblofs`, nicht die Slots des Kampf-Scripts (`scripts::extend_battle_slots_with` hängt sie beim Anlegen an); Register und Platzhalter sind so gewollt. Folgen: keine im Spiel (gebaut, gespielt, 0 Fehler); der nächste neue Trainer bekommt wieder 0930 und nutzt den Slot (`next_trainer` erweitert nur über `battle_slots.max()`). Kleinster Fix: `free_number` schneidet für Trainer die Slots über dem letzten lebenden Trainer aus dem `#[…]` der Kampf-Scriptdatei (ein Compile wie beim Anhängen), Test in `tests/delete.rs`. | offen, dokumentiert (kein Schaden; Fix berührt Kernel und Compile, größer als ein Gate-Fix) |
| B2 | Klein (Beobachtung) | (B) | Beim allerersten Öffnen der Kopie meldete die Konsole zwei `400` von `__bridge/invoke/get_area_data` (vor jeder Bedienung, mit `favicon.ico` 404). Nicht wiederholbar: Neuladen, auch mit geleertem `localStorage` und einer Fetch-Hülle für die Anfrage, ohne Fehler; die Brücke schreibt die Ursache nicht ins Log. Sonst nur WebGL-Hinweise „GPU stall due to ReadPixels“. | offen, nicht reproduziert |
| B3 | Doku | Vorbereitung | Das Bühnen-Szenario scheiterte zuerst an `assign_base` („Pfad liegt außerhalb der erlaubten Ordner: ../3d-gate-web.nds“): `studio mcp` erlaubt die `.nds` nur neben einem `*_DSPRE_contents`-Ordner (oder mit `--allow-dir`); die Szenario-Kopien des Browsers brauchen den Zusatz. | Doku: [agent-gate.md](agent-gate.md), „Phasen-Gate 3d“; Kopie umbenannt |

| B4 | Mittel | `scripts/gate.sh` Lauf 2 | `recipes_story::vanilla_story_chain_is_five_undo_steps_and_git_status_is_empty_after_undo` (HeartGold DE): nach dem Undo der Kette bis zur Marke scheitert `save()` mit `Stale(Event 57)`. Ursache: seit Schema 2 (3d.4) steht der eigene Flag-Name im Event-JSON (`hidden_flag`); das Undo von `name_flag` (3d.7, Dateischritt `IdRename`) nimmt den Namen aus dem Register, die Eventdatei nennt ihn weiter, kompiliert nicht mehr (`textproject.unknown-id`) und der Store hält seine eigene Datei für fremd geändert. Dasselbe nach dem Undo von `rename_id`. In der App verdeckt das Neuladen nach Dateischritten den Fehler nicht ganz: das Projekt auf der Platte importiert nicht, bis das Event neu gespeichert ist. | behoben: Undo und Redo eines `IdRename`-Schritts schreiben Eventdateien mit einem entfernten Namen auf die Nummer zurück (`textproject::rename::unnamed_events`, aus `move_files` unter der Sperre des Schritts, Regel von 3d.4 `remove_own_name`); zwei Tests in `flags/entities/tests.rs`; `373d7c7e`, Merge `68c98254`; gated `recipes_story` auf allen drei Spielen grün |
| B5 | Klein | `scripts/gate.sh` Lauf 3 | `project::vanilla_every_archive_path_exists` (Platin DE): seit 3d.3 Teil B nennt `archives.yaml` für Platin den Ei-Attacken-Patch-NARC `files/zcustom/egg_moves.narc`, den Vanilla Platin nicht hat (Tabelle in Overlay 5); der Test verlangte die Datei. Latent seit `ed4ab430` (2026-11-01), nie gated auf Platin gelaufen, weil die Gate-Läufe vorher auf HeartGold DE abbrachen. | behoben im Test: auf Platin prüft er den dokumentierten Vanilla-Zustand (weder NARC noch Ordner `eggMoves`); `fcdb8eae` |
| B6 | Beobachtung | `scripts/gate.sh` Lauf 4 | `git::cli::tests::the_templates_are_refreshed_before_the_commit` (4.4-Test, synthetisches Repo) fiel im gated Schritt Platin DE bei Last 18 bis 27 mit `gix`: „Could not create index from tree“ direkt nach einem Commit über die Git-CLI; grün im selben Lauf ohne ROM, auf HeartGold DE und HeartGold US, in Lauf 3 viermal, allein 5 von 5. | Last-Flake, kein Fix; Wiederholung des Schritts grün |

Die Fixes B4 und B5 sind auf main (`68c98254`, `fcdb8eae`); sie kamen aus `scripts/gate.sh`, nicht aus dem Gate-Lauf des Agenten. Neu im Repo sind sonst nur die beiden Szenarien, ihr Ladetest in `scripts/g4emu/tests/test_scenario.py` und die Dokumentation.

## Beim Nutzer

### Hand

- [ ] **Die Geschichte mit einem echten Spielstand:** in der gepackten App in Erzählen nachbauen (Trainer per Assistent, eigenes Flag, „Der Brief“ mit Gespräch, Erscheinen und Belohnung, Entwurf löschen und freigeben), bauen und in melonDS mit einem eigenen Spielstand spielen; die Szenarien nutzen `_saves/CPUD-zimmer.sav` und `IPKD-zimmer.sav` (ohne Pokémon) und ein Geschenk mit Level 100.
- [ ] Die offenen Hand-Punkte der Teilphasen (Gate-Dateien 3d.1 bis 3d.9).

### Blick

Agent + Blick: der Agent hat die Bilder gelesen und beschrieben; der Nutzer bestätigt am Bild.

- [ ] `target/agent/gate-2026-11-04/3d-gate/browser/blick-trainer-assistent-team-{hell,dunkel}.png`: „Neuer Trainer hier“, Schritt „Team“ mit PIKACHU Lv. 12, rechts die Vorschau mit Kampfvorschau.
- [ ] `…/browser/blick-trainer-assistent-vorschau-{hell,dunkel}.png`: die Vorschau oben: Klassenbild „Jens · Wanderer“, Kampfvorschau mit dem Kampfsprite PIKACHU und dem Statuskasten „PIKACHU Lv. 12“ (Taste „Erstes Pokémon“).
- [ ] `…/browser/blick-flag-benennen-hell.png`: Dialog „Als eigenes Flag benennen“ mit `FLAG_BRIEF_GELESEN` und Beschreibung; `s16-flag-benannt.png` das Detail danach.
- [ ] `…/browser/blick-neue-geschichte-{hell,dunkel}.png`: „Neue Geschichte“ mit „Noch 20 Variablen frei“ und `VAR_UNUSED_0x4031`.
- [ ] `…/browser/blick-schritt-gespraech-{hell,dunkel}.png`, `blick-schritt-erscheinen-{hell,dunkel}.png` (mit der Warnung „Level-Script-Datei 1125 wird angelegt und bleibt“), `blick-schritt-belohnung-{hell,dunkel}.png` (Trank, „Du erhältst Trank!“): die drei Schritt-Dialoge mit „Was das Rezept tut“.
- [ ] `…/browser/blick-geschichte-schritte-{hell,dunkel}.png`: „Der Brief“ mit vier Schritten und ihren Verweisen.
- [ ] `…/browser/blick-inspector-hinweis-{hell,dunkel}.png`: Jens in der Welt, im Inspector „Geschichte „Der Brief“: Figur 2 erscheint ab Schritt 1“.
- [ ] `…/browser/blick-loeschen-sperre-{hell,dunkel}.png`: „Trainer 0930 löschen“, „1 Verweis: zuerst umhängen.“, „Löschen“ gesperrt.
- [ ] `…/browser/blick-grabstein-{hell,dunkel}.png`: der Entwurf mit der Plakette „gelöscht“; `blick-nummer-freigeben-{hell,dunkel}.png`: der Dialog mit dem Kästchen „Nummer freigeben“ und den drei Dateien.
- [ ] `target/agent/gate-2026-11-04/3d-gate/gespielt/{gespraech,kampf,belohnung,belohnung-2}.png`: die im Browser erzählte Geschichte gespielt: „Bring den Brief zu Jen…“, „Eine Herausforderung von Wanderer Jens!“, „Du erhältst Trank!“, „Hallo, Reisender!“.
- [ ] `target/agent/gate-2026-11-04/3d-gate/{pt,hg}-geschichte/{gespraech,szene,kampf,belohnung,belohnung-2}.png`: die Szenarien von (A).
- [ ] Kompakt: nicht aufgenommen (nur von Hand).
- [ ] **`.fig`:** die Frames der Phase 3d sind nicht eingespielt; OpenPencil war in dieser Sitzung nicht erreichbar. Anleitung in [design-system/README.md](../design-system/README.md), Nachträge 3d.1 bis 3d.9.

## Offen

- `scripts/gate.sh` auf main: grün (Tabelle oben); die Flakes unter Last (Lauf 1 `dynamic_headers`, Lauf 4 `git::cli`) bleiben Beobachtungen.
- Hand, Blick und `.fig` (oben).
- B1 (Kampf-Slot des freigegebenen Trainers), B2 (nicht reproduziert).
- HeartGold US als Szenario und HeartGold DE im Browser (optional, nicht gelaufen).
- Testkopien: alle `3d-gate-*` nach dem Gate gelöscht (2026-11-04); `target/agent/gate-2026-11-04/3d-gate/` bleibt.

## Links

Teilphasen: [3d.1](phase-3d-1-gate.md), [3d.2](phase-3d-2-gate.md), [3d.3](phase-3d-3-gate.md), [3d.4](phase-3d-4-gate.md), [3d.5](phase-3d-5-gate.md), [3d.6](phase-3d-6-gate.md), [3d.7](phase-3d-7-gate.md), [3d.8](phase-3d-8-gate.md), [3d.9](phase-3d-9-gate.md), Zwischenphase [4.5](phase-4-5-gate.md). Formate: [stories.md](formats/stories.md), [recipes.md](formats/recipes.md), [text_project.md](formats/text_project.md). Werkzeuge: [mcp.md](mcp.md). Szenarien: `scripts/agent/scenarios/pt-geschichte.yaml`, `hg-geschichte.yaml`, `pt-geschichte-gate-buehne.yaml`, `pt-geschichte-gate.yaml`.
