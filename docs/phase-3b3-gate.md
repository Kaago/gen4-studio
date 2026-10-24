# Phase 3b.3: Gate prüfen

Stand: 2026-10-24. Teil A (Gate 3b.3a „Codecs und Pokémon-Daten“) ist vom Agenten über die Test-Umgebung abgenommen (Ergebnis unten); offen sind die Blicke des Nutzers auf die Screens und die Symbole. Teil B (Gate 3b.3b „Trainer, Begegnungen“, zugleich das Roadmap-Gate 3b.3) ist implementiert: die beiden Szenarien und `scripts/gate.sh` sind grün (Ergebnis in Teil B), die Abnahme der Editoren über die Test-Umgebung und die Punkte des Nutzers sind offen.

Phase 3b.3a bringt eigene Codecs für alle Daten-Archive (Begegnungen, Trainer, Pokémon-Basisdaten, Lernsets, Entwicklungen, Attacken, Gegenstände, Trainer-Texttabellen), die Archive im Store mit mehrteiligem Changeset, Namenstabellen, arm9-Tabellen (nur lesend), einen Referenzindex, Item- und Pokémon-Symbole aus dem ROM und die Editoren **Pokédex**, **Attacken** und **Gegenstände** in den Atlas-Bereichen Pokémon und Gegenstände. Die Originale der Test-ROMs bleiben unberührt: alles läuft auf Kopien. Spezifikation: [docs/superpowers/specs/2026-10-23-phase-3b3-data-editors-design.md](superpowers/specs/2026-10-23-phase-3b3-data-editors-design.md), Abschnitt 1 (Gate 3b.3a). Plan: [docs/superpowers/plans/2026-10-23-phase-3b3a-codecs-pokemon-data.md](superpowers/plans/2026-10-23-phase-3b3a-codecs-pokemon-data.md).

## Teil A: Gate 3b.3a

Teil A besteht aus sieben Punkten:

- (A1) Automatik: der gated Test `vanilla_data` ist grün (`write(read(x)) == x` über alle Member der zehn Archivarten in Platin DE, HeartGold DE und HeartGold US, mit Soll-Member-Zahlen), das uxie-Orakel stimmt für Personal, Items und Trainer-Eigenschaften Feld für Feld, der Vanilla-Rebuild bleibt byte-identisch, obwohl der Build jetzt auch die Daten-Ordner packt; `scripts/gate.sh` grün.
- (A2) Szenario `pt-daten.yaml` (Backend): Staravia Angriff 75 → 85, eine Lernset-Zeile mehr, Flügelschlag Stärke 60 → 65, Trank Preis 300 → 250; speichern, bauen; der Prüfschritt `g4k check-data` liest die gebauten NARCs und findet die neuen Werte; das ROM startet.
- (A3) Die drei Editoren sind gegen die Screens „Pokédex: Staravia bearbeiten“, „Attacken: Flügelschlag“ und „Gegenstände: Trank“ wiedererkennbar (ohne DS-Profil).
- (A4) ⌘Z nimmt „Wie Staralili“ (TM/VM und Lernset einer anderen Art, zwei Assets) in einem Schritt zurück.
- (A5) Tüfteln zeigt die Rohdaten als Hexblock mit markiertem Feld.
- (A6) Item-Symbole aus dem ROM in der Liste „Gegenstände“.
- (A7) Renderzeit der Listen: über 50 ms wird die Liste gefenstert (Spec §9).

Wer was prüft: Die Spalte „Wer“ der Ergebnistabelle sagt **Agent** (die Test-Umgebung aus [docs/agent-gate.md](agent-gate.md): Browser-Fenster mit `studio-bridge`, `call.sh`, `g4emu`), **Agent + Blick** (der Agent prüft und beschreibt Ort und Zahlen, der Nutzer bestätigt kurz am Bild) oder **Hand** (der Nutzer). Zuordnung und Rezepte: [docs/agent-gate.md](agent-gate.md), Abschnitte „Phase 3b.3a“ und „Rezepte“.

Namen in der Oberfläche (die ganze Tabelle steht in [docs/agent-gate.md](agent-gate.md), Abschnitt „Doppeldeck: wo was ist“):

- **Atlas:** „Pokémon“ und „Gegenstände“ sind offen, „Geschichten“ bleibt gesperrt („Kommt mit Phase 3b.4“).
- **Pokémon:** Der Listenkopf ist ein Menü (der Name des Unterbereichs mit Pfeil, „Pokédex ▾“) mit „Pokédex“, „Trainer“, „Begegnungen“, „Attacken“ und Zählern; Trainer und Begegnungen sind gesperrt („kommt mit Phase 3b.3b“). ⌘1 bis ⌘4 wechseln im Bereich Pokémon, die Palette hat „Bereich Pokémon“ und „Attacken öffnen“. Brotkrume „Pokémon › Pokédex › STARAVIA“, das zweite Glied öffnet dasselbe Menü.
- **Gegenstände:** eine Liste „Gegenstände“ mit Symbol, Name, Nummer und Preis rechts, Filter „Taschen“; Brotkrume „Gegenstände › Tasche Medizin › Trank“, ein Klick auf die Tasche filtert die Liste.
- **Schau und Hand:** die Dock-Reiter heißen „Übersicht“ und „Bearbeiten“. Rohdaten (Hexblock, Dateiname, Eintrag) zeigt nur die Tiefe „Tüfteln“ im Scharnier.

Was im Browser-Modus anders ist als in der gebündelten App: Chrome kann ⌘1 bis ⌘4 selbst für den Tab-Wechsel nehmen; dann den Umschalter im Listenkopf oder die Palette benutzen. Sonst gilt das Gleiche wie in [phase-3c-gate.md](phase-3c-gate.md).

### Vorbereitung

1. Kopien anlegen (nie ein Original öffnen; das Ziel darf noch nicht existieren):
   `cp -R "../Roms/Pokemon - Platin-Edition (Germany)_DSPRE_contents" ../Roms/_testkopien/pt-3b3a-gate`
   Für das Szenario eine zweite frische Kopie `pt-3b3a-szenario` (das Szenario speichert und baut darin).
2. Test-Umgebung starten, wie in [docs/agent-gate.md](agent-gate.md), „Je Lauf“ (aus einem Git-Worktree mit absolutem `G4_ROMS_DIR`). Seite: `http://127.0.0.1:1430/?project=<absoluter Pfad von pt-3b3a-gate>`. Vor dem Lauf `localStorage.clear()` und neu laden.
3. Bilder: Der Agent hält Screenshots klein und beschreibt Ort und Zahlen. Die Vorlagen sind die Seiten „Screens: Pokémon und Kämpfe“ („Pokédex: Staravia bearbeiten“, „Attacken: Flügelschlag“) und „Screens: Gegenstände, Texte, Testen“ („Gegenstände: Trank“) in `design-system/Editor_DesignSystem.fig` (OpenPencil).

Fakten zu Platin DE, die der Ablauf braucht: Staravia ist Art 0397 (Angriff 75, zwölf Lernset-Einträge, entwickelt sich mit Level 34 zu Staraptor 0398), Staralili ist 0396. Flügelschlag ist Attacke 0017 (Stärke 60, Merkmale-Byte 0x73). Trank ist Gegenstand 0017 und Datenmember 17 (Preis 300, Tasche Medizin). Platin schreibt die Artnamen in Großbuchstaben („STARAVIA“), Attacken und Gegenstände nicht.

### (A1) Automatik

- [x] 1. `vanilla_data` gated grün auf Platin DE, HeartGold DE und HeartGold US: die Tests in `crates/g4-formats/tests/vanilla_data.rs` (`vanilla_fixed_records_round_trip`, `vanilla_variable_records_round_trip`, `vanilla_trainer_parties_round_trip_with_their_properties`, auf Platin dazu `vanilla_pt_water_slots_have_min_le_max`; auf HeartGold meldet dieser „skipped: not a Platinum project“). Ein Ordner mit falscher Member-Zahl, ohne Member oder mit anderen Dateien als `NNNN` lässt den Test fehlschlagen.
- [x] 2. uxie-Orakel: `vanilla_personal_matches_uxie`, `vanilla_items_match_uxie`, `vanilla_trainer_properties_match_uxie` grün auf allen drei Projekten.
- [x] 3. Vanilla-Rebuild: `vanilla_rebuild_matches_dsrom_cli` (mit `G4_DSROM_CLI`) grün auf allen drei Projekten; der Test verlangt, dass die Daten-Ordner mitgepackt werden.
- [x] 4. `scripts/gate.sh` läuft grün gegen alle drei Projekte (die Punkte 1 bis 3 laufen darin mit); keine Zeile „skipped“ außer den dokumentierten. Aus einem Worktree: `G4_ROMS_DIR=<absoluter Pfad zu Roms> scripts/gate.sh`. Die Tabelle aus der Ausgabe kommt unter „Ergebnis“.

Einzeln (ohne das ganze Gate), je Projekt, aus der Repo-Wurzel:

```sh
export PATH="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$PATH"
G4_TEST_PROJECT="<Roms>/Pokemon - Platin-Edition (Germany)_DSPRE_contents" G4_DSROM_CLI="<Roms>/_tools/dsrom" \
  cargo test --workspace --all-features -- vanilla_fixed_records vanilla_variable_records vanilla_trainer_parties \
  vanilla_personal_matches_uxie vanilla_items_match_uxie vanilla_trainer_properties_match_uxie \
  vanilla_pt_water_slots vanilla_rebuild_matches_dsrom_cli --nocapture
```

### (A2) Szenario `pt-daten.yaml`

Auf `pt-3b3a-szenario`, aus `scripts/g4emu`. Aus einem Git-Worktree zeigt `../../../Roms` nicht auf die Roms (von `scripts/g4emu` aus ist das `.claude/worktrees/Roms`): dann `G4_ROMS_DIR=<absoluter Pfad zu Roms>` setzen und `--project` absolut angeben; scheitert dort der Bau von `g4k` am Download in rotoms Build-Skript, nimmt das Szenario mit `G4K_BIN=<Worktree>/target/debug/g4k` und `G4_BRIDGE_BIN=<Worktree>/target/debug/studio-bridge` fertige Binaries (beide baut `cargo test --workspace --all-features --no-run`).

- [x] 1. `uv run g4emu scenario ../agent/scenarios/pt-daten.yaml --project ../../../Roms/_testkopien/pt-3b3a-szenario --out ../../target/agent/gate-<datum>/pt-daten` endet mit Rückgabe 0 und der Zeile `"ok": true`.
- [x] 2. Die vier `data_edit` (Staravia Angriff 85, Lernset von 12 auf 13 Einträge, Flügelschlag Stärke 65, Trank Preis 250), `save` und `build_rom` sind ok.
- [x] 3. Der Prüfschritt `g4k check-data out.nds --expect species:397:attack=85 --expect learnset:397:len=13 --expect move:17:power=65 --expect item:17:price=250` meldet „4 expectation(s) met“ (Ausgabe in `<out>/check.log`).
- [x] 4. Spielen als Starttest: `continue`, `expect-map 415`, `shot start`; das Bild `<out>/start.png` zeigt das Spielerzimmer.
- [x] 5. Gegenprobe, dass der Prüfschritt beißt: `target/debug/g4k check-data <Kopie>/out.nds --expect species:397:attack=75` endet mit Exit 1 und „species 397 attack: expected 75, found 85“.

### (A3) Editoren gegen die Screens

Auf `pt-3b3a-gate`, Tiefe „Bauen“:

- [x] 1. Pokédex: Atlas „Pokémon“, Staravia wählen (Suche „Pokédex durchsuchen“, `397`). Screenshot des ganzen Fensters. Zu sehen sein müssen: Listenkopf „Pokédex ▾“ mit „493 Arten, 12 Formen“, Filter „Typ“, Zeilen mit Symbol, Name und Nummer; Schau „Übersicht“ mit Kopfkarte (Symbol, Name, 0397, Typen Normal und Flug, Fähigkeit), sechs Wertebalken mit dem Schnitt aller Arten, „Summe 340“, „Fundorte“ und „Verwendet in“; Hand „Bearbeiten“ mit den Reitern „Werte“, „Fähigkeiten und Typen“, „Lernset“, „Entwicklung“, „TM/VM“; Brotkrume „Pokémon › Pokédex › STARAVIA“, Statusleiste „STARAVIA, 0397“. Der Vergleich mit „Pokédex: Staravia bearbeiten“ ist ein Blick des Nutzers (das DS-Profil des Screens fehlt bewusst, siehe „Später“ in der Spec).
- [x] 2. Im Reiter „Werte“ Angriff 75 → 85: die Titelleiste sagt „1 Änderung nicht gespeichert“, die Schau „Summe 350, vorher 340“, `call.sh history` nennt „STARAVIA: Angriff 75 → 85“; ⌘Z stellt 75 und „Alles gespeichert“ wieder her.
- [x] 3. Attacken: ⌘4 (oder der Umschalter), Flügelschlag wählen. Die Karte nennt Name, „Nummer 0017“, Typ Flug, Physisch, „60 Stärke“, „100 % Genauigkeit“, „35 AP“ und die Merkmale; darunter die Beschreibung als DS-Textbox mit „Beschreibung bearbeiten“ und „Verwendet in“ („23 Lernsets, 11 Trainer“); die Hand hat die Spalten „Grundwerte“ und „Verhalten“. Stärke 60 → 65: Verlauf „Flügelschlag: Stärke 60 → 65“, ⌘Z. Der Vergleich mit „Attacken: Flügelschlag“ ist ein Blick des Nutzers.
- [x] 4. Gegenstände: Atlas „Gegenstände“, Trank wählen. Brotkrume „Gegenstände › Tasche Medizin › Trank“; die Karte zeigt Symbol, Name, „Nummer 0017“ und den Preis 300 ₽, darunter die Beschreibung als DS-Textbox und „Verwendet in“; die Hand zeigt die Werte des Gegenstands in Spalten. Preis 300 → 250: Verlauf „Trank: Preis 300 → 250“, die Zeile zeigt „250 ₽“, ⌘Z. Der Vergleich mit „Gegenstände: Trank“ ist ein Blick des Nutzers.

### (A4) ⌘Z über zwei Assets („Wie Staralili“)

Auf `pt-3b3a-gate`, Pokédex, Staravia, Reiter „TM/VM“. In Platin DE hat Staralili dieselben TM/VM wie Staravia; damit die Kopie beide Teile ändert, wird vorher ein Kästchen umgestellt und gespeichert:

- [x] 1. TM01 einschalten, speichern („Alles gespeichert“).
- [x] 2. „Von anderer Art übernehmen …“, Staralili wählen: `call.sh history` nennt einen Schritt „STARAVIA: TM/VM und Lernset wie STARALILI“, `call.sh dirty` nennt zwei Assets (Personal-Daten und Lernset von 397), TM01 ist aus, „Lernset“ hat 11 statt 12 Zeilen.
- [x] 3. Ein ⌘Z: `call.sh dirty` liefert `[]`, TM01 ist wieder an, „Lernset“ hat wieder 12 Zeilen; „Wiederholen“ hält den Schritt.

### (A5) Tüfteln: Hexblock mit markiertem Feld

Auf `pt-3b3a-gate`, im Scharnier „Tüfteln“:

- [x] 1. Attacken, Flügelschlag: die Hand hat die Spalte „Rohdaten“ (in Bauen nicht). Feld „Stärke“ fokussieren: der Hexblock nennt „files/poketool/waza/pl_waza_tbl.narc, Eintrag 17, 16 Bytes“, Byte 3 ist markiert (`[data-marked]` mit `data-offset="3"`), der Fuß sagt „Byte 3: Stärke, 0x3C = 60“.
- [x] 2. Pokédex, Staravia: Reiter „Rohdaten“ (nur in Tüfteln) mit drei Blöcken („Pokémon-Daten“, 44 Bytes; „Lernset“; „Entwicklungen“, 44 Bytes); nach Fokus auf „Angriff“ im Reiter „Werte“ ist Byte 1 markiert („Byte 1: Angriff, 0x4B = 75“).
- [x] 3. Gegenstände, Trank: Spalte „Rohdaten“ mit „files/itemtool/itemdata/pl_item_data.narc, Eintrag 17, 34 Bytes“; nach Fokus auf „Preis“ sind die Bytes 0 und 1 markiert („Byte 0 bis 1: Preis, 0x012C = 300“).

### (A6) Item-Symbole in der Liste

- [x] 1. Gegenstände: die sichtbaren Zeilen zeigen Symbole aus dem ROM: `document.querySelectorAll('[role="listbox"][aria-label="Gegenstände"] [data-sprite="picture"]').length` ist größer als 0 (Platzhalter tragen `data-sprite="placeholder"`); Symbole laden nur für Zeilen im Blick, nach dem Scrollen kommen weitere dazu.
- [x] 2. `call.sh get_item_icon '{"item":17}'` meldet eine Zahl von Bytes größer als 0 (PNG).
- [ ] 3. Blick des Nutzers: die Symbole sehen aus wie im Spiel (Farben, nicht verschoben). Ebenso die Pokémon-Symbole der Pokédex-Liste.

### (A7) Renderzeit der Listen

- [x] 1. Pokédex: die Liste ist gefenstert: in der Liste `[role="listbox"][aria-label="Pokédex"]` stehen deutlich weniger als 508 Zeilen `[data-species]` im DOM, `aria-setsize` der Zeilen ist 508; Pfeiltasten, Pos1 und Ende erreichen jede Zeile.
- [x] 2. Zeit messen (`performance.now()` im Browser-Fenster, zum Beispiel vom Leeren der Suche „397“ mit Escape bis die Zeilen im DOM stehen): unter 50 ms. Zum Vergleich aus der Umsetzung (Dev-Build, verborgenes Fenster): ohne Fensterung 49 bis 111 ms, mit 22 bis 25 ms.
- [x] 3. Dasselbe für „Attacken“ (471) und „Gegenstände“ (468 in Platin DE).

### Ergebnis

Durchlauf des Agenten über die Test-Umgebung am 2026-10-24 (Browser-Fenster 1440 × 900, Dev-Build, Fenster verborgen).

| Punkt | Wer | Datum | Projekt | Ergebnis | Auffälligkeiten |
|---|---|---|---|---|---|
| (A1) 1 `vanilla_data` gated | Agent | 2026-10-24 | Platin DE, HeartGold DE, HeartGold US (Originale, nur lesend) | bestanden | läuft in `scripts/gate.sh` mit, siehe unten |
| (A1) 2 uxie-Orakel | Agent | 2026-10-24 | Platin DE, HeartGold DE, HeartGold US | bestanden | läuft in `scripts/gate.sh` mit |
| (A1) 3 Vanilla-Rebuild byte-identisch | Agent | 2026-10-24 | Platin DE, HeartGold DE, HeartGold US | bestanden | läuft in `scripts/gate.sh` mit (`G4_DSROM_CLI`) |
| (A2) Szenario `pt-daten.yaml` | Agent | 2026-10-24 | `pt-3b3a-szenario-2` | bestanden | Rückgabe 0, `"ok": true`; vier `data_edit`, `save` (vier Assets geschrieben) und `build_rom` ok; `check.log`: „species 397 attack: 85“, „learnset 397 len: 13“, „move 17 power: 65“, „item 17 price: 250“, „4 expectation(s) met“; `start.png` zeigt das Spielerzimmer (Header 415, x 7, y 4). Gegenprobe `--expect species:397:attack=75`: Exit 1, „species 397 attack: expected 75, found 85“. Bild: `target/agent/gate-2026-10-24/abnahme-3b3a/pt-daten/start.png`. Aus dem Worktree brauchte das Szenario `G4_ROMS_DIR`, einen absoluten `--project` und `G4K_BIN`/`G4_BRIDGE_BIN` (der Bau von `g4k` scheitert dort am Download in rotoms Build-Skript); der erste Lauf auf `pt-3b3a-szenario` brach deshalb nach `build_rom` ab, die Kopie ist gespeichert und gebaut. Die Anleitung nennt das jetzt. |
| (A3) Editoren gegen die Screens | Agent + Blick | 2026-10-24 | `pt-3b3a-gate`, Gegenprobe `hg-3b3a-gate` | Agent-Teil bestanden; Blick offen | Pokédex: Kopf „Pokédex ▾“, „493 Arten, 12 Formen“ (bei 272 px Breite in Komfort als „493 Arten, 12 Form…“ gekürzt, in Kompakt ganz), Filter „Typ“, Zeilen mit Symbol, Name, Nummer; Schau „Übersicht STARAVIA“ mit 0397, Normal und Flug, „Fähigkeit: Bedroher“, sechs Balken (55, 75, 50, 40, 40, 80 mit „über/unter Schnitt“), „Summe 340“, „Fundorte 8 Orte, 14 Trainer-Teams“, „Verwendet in“ („STARALILI, Entwicklung 1 Ziel“, „1 Stelle, ohne Scripts gezählt.“); Hand mit „Werte“, „Fähigkeiten und Typen“, „Lernset 12“, „Entwicklung“, „TM/VM 26“; Brotkrume Pokémon › Pokédex › STARAVIA; Statusleiste „STARAVIA, 0397“. Angriff 75 → 85: „1 Änderung nicht gespeichert“, „Summe 350, vorher 340“, Verlauf „STARAVIA: Angriff 75 → 85“, ⌘Z stellt 75 und „Alles gespeichert“ her. Lernset: „Zeile hinzufügen“ mit Windstoß 0016 → Verlauf „STARAVIA: Lernset 12 → 13 Attacken“, die Zeile steht bei Level 1. Entwicklung: „Level“, 34, STARAPTOR 0398, sechs Zeilen „Keine“. Attacken: Karte „Flügelschlag“ (Archiv 0647), „Nummer 0017“, Flug, Physisch, „60 Stärke“, „100 % Genauigkeit“, „35 AP“, vier Merkmale, „Priorität 0“; Beschreibung „Seite 1 von 2“ mit „Beschreibung bearbeiten“ (Archiv 0646); „Verwendet in“ „23 Lernsets, 11 Trainer“ mit „Alle zeigen, 11 weitere“; Stärke 60 → 65 („Flügelschlag: Stärke 60 → 65“) und Priorität 0 → −1 („Flügelschlag: Priorität 0 → −1“, Byte 10 = 0xFF), beides mit ⌘Z zurück. Gegenstände: Brotkrume Gegenstände › Tasche Medizin › Trank, Karte mit Symbol, „Nummer 0017“, „300 ₽ Preis“, „150 ₽ Verkauf“, „30 Schleuder-Stärke“, Beschreibung „Hilft bei Wunden.“, „Verwendet in“ „8 Trainer-Items, 0 Team-Items, 0 wild getragen, 0 Entwicklungen“; Preis 300 → 250: Verlauf „Trank: Preis 300 → 250“, Zeile „250 ₽“ mit Punkt, ⌘Z. Geteilte Daten: 0113 warnt „Preis und Werte gelten auch für 22 andere Gegenstände“, das Label endet auf „gilt für 23 Gegenstände“. Palette: „0397“ findet Route 222 (Header), Textarchiv 0397, STARAVIA, Steinpolitur, TM70 (6 Treffer). Blick: Bilder unter `target/agent/gate-2026-10-24/abnahme-3b3a/browser/` (`pokedex-staravia-hell.jpg`, `pokedex-staravia-hell-angriff85.jpg`, `pokedex-tmvm-hell.jpg`, `attacken-fluegelschlag-hell.jpg`, `attacken-fluegelschlag-hell-textbox.jpg`, `gegenstaende-trank-hell.jpg`; 800 × 500 aus 1440 × 900). |
| (A4) ⌘Z über zwei Assets („Wie Staralili“) | Agent | 2026-10-24 | `pt-3b3a-gate` | bestanden | TM01 an, gespeichert („Alles gespeichert“); „Von anderer Art übernehmen …“ (Rückfrage-Text „Ersetzt TM/VM und Lernset von STARAVIA“), STARALILI 0396: Verlauf „STARAVIA: TM/VM und Lernset wie STARALILI“, `dirty` = Personal 397 und Learnset 397, TM01 aus, „Lernset 11“, „2 Änderungen nicht gespeichert“. Ein ⌘Z: `dirty` = `[]`, TM01 an, „Lernset 12“, „Alles gespeichert“, „Wiederholen“ bereit, `redo` hält den Schritt. |
| (A5) Tüfteln: Hexblock mit markiertem Feld | Agent | 2026-10-24 | `pt-3b3a-gate` | bestanden | Attacken: Spalte „Rohdaten“ nur in Tüfteln, „files/poketool/waza/pl_waza_tbl.narc, Eintrag 17, 16 Bytes“, Fokus „Stärke“: `data-offset="3"`, „Byte 3: Stärke, 0x3C = 60“ (nach der Änderung „0x41 = 65“), dazu Effekt-ID, „Merkmale-Byte 0x73“ mit Bit 0 bis 7; `data_raw` gleich. Pokédex: Reiter „Rohdaten“ mit „Pokémon-Daten“ (pl_personal.narc, Eintrag 397, 44 Bytes), „Lernset“ (wotbl.narc, 28 Bytes), „Entwicklungen“ (evo.narc, 44 Bytes), Fokus „Angriff“: Byte 1, „Byte 1: Angriff, 0x4B = 75“; Zahlfelder mit Dez/Hex. Gegenstände: „files/itemtool/itemdata/pl_item_data.narc, Eintrag 17, 34 Bytes“, Fokus „Preis“: Bytes 0 und 1, „Byte 0 bis 1: Preis, 0x012C = 300“, darunter „Wirkung im Team“. |
| (A6) Item-Symbole in der Liste | Agent + Blick | 2026-10-24 | `pt-3b3a-gate`, `hg-3b3a-gate` | Agent-Teil bestanden; Blick offen | Platin: 21 Bilder (`data-sprite="picture"`) für die Zeilen 0 bis 20 im Blick, 10 Platzhalter darunter außerhalb; nach dem Scrollen Bilder für 93 ff.; `get_item_icon {"item":17}`: 518 Bytes. HeartGold DE: 21 Bilder. Pokédex-Liste: 21 Bilder. Blick: Farben und Lage der Symbole in `gegenstaende-trank-hell.jpg`, `gegenstaende-trank-dunkel-kompakt.jpg` (Liste links, Keines bis Hyperheiler), `pokedex-staravia-dunkel-kompakt.jpg` (LIEBISKUS 0370 bis STARAVIA 0397) und `hg-gegenstaende-trank.jpg`. |
| (A7) Renderzeit der Listen | Agent | 2026-10-24 | `pt-3b3a-gate` | bestanden nach Fix | Gefenstert: Pokédex 41 Zeilen im DOM (31 plus die gewählte Zeile), `aria-setsize` 508; Gegenstände 31 von 468, Attacken 30 von 471. Ende → „Schneid-Rotom, 0507“ (508), Pos1 → „-----, 0000“, Bild ab → 0023 (Pos. 24), Enter wählt. Zeit vom Escape in der Suche bis die Zeilen stehen (MutationObserver, verborgenes Fenster, Dev-Build, fünf Läufe): vorher Pokédex 61 bis 64 ms (lange Aufgaben 61 bis 65 ms), Gegenstände 52 bis 65 ms, Attacken 33 bis 43 ms; Ursache: eine Suche zeichnete auch Schau und Hand neu (der Reiter „Werte“ allein rund 30 ms). Nach dem Fix (Suche im eigenen Kontext): Pokédex 34 bis 36 ms, Gegenstände 22 bis 25 ms, Attacken 20 bis 21 ms, keine lange Aufgabe. |
| (A1) 4 `scripts/gate.sh` | Agent | 2026-10-24 | Platin DE, HeartGold DE, HeartGold US | bestanden („Gate: ok (1781 s)“) | nach den drei Fix-Commits, aus dem Worktree mit `G4_ROMS_DIR`; Log `target/agent/gate-3b3a-abnahme.log`; keine Zeile „skipped“ |

Weitere Prüfungen der Abnahme (Review Focus des Plans, Design-System; 2026-10-24, Agent, ohne eigene Gate-Nummer):

| Prüfung | Wer | Projekt | Ergebnis | Auffälligkeiten |
|---|---|---|---|---|
| Gesperrtes Member | Agent | `pt-3b3a-gate-lock` (Item-Member 0017 per `printf '\x00\x00' >>` auf 36 Bytes, Personal 0397 per `head -c 36` auf 36 Bytes) | bestanden | Gegenstände: Zeile „Trank 0017“ mit Plakette „Gesperrt“; Schau „Daten 0017: 36 Bytes statt 34. Die Werte dieses Gegenstands sind nicht lesbar.“; Hand nur „… Die Felder dieses Gegenstands sind gesperrt.“ (keine Felder), in Tüfteln der Hexblock „Eintrag 17, 36 Bytes“. Pokédex: Schau „Pokémon-Daten 0397: 36 Bytes statt 44. Ihre Werte sind nicht lesbar.“, Hand „… Die Felder dieses Teils sind gesperrt.“, alle Felder der Werte gesperrt (0), Lernset bleibt bearbeitbar. Nichts stürzt. Bilder `lock-trank-36bytes.jpg`, `lock-staravia-36bytes.jpg`. |
| Item-Tabelle unplausibel | Agent | `pt-3b3a-gate-0017` (arm9 0xF0DA4, Eintrag 17 der Item-Tabelle, mit 8 × 0xFF überschrieben) | bestanden | `item_table` und `machine_table` liefern `null`; Liste „Daten 0000“, „Daten 0017“ …, ohne Symbole, Hinweis „Item-Tabelle nicht lesbar: Namen und Symbole fehlen“; TM/VM-Raster nur „TM01“ … ohne Attackennamen. Bild `itemtable-unplausibel-tmvm.jpg`. |
| Formen und Arten ohne Namen | Agent | `pt-3b3a-gate` | bestanden | Ende der Liste „Schneid-Rotom, 0507“; Kopf „493 Arten, 12 Formen“. |
| ⌘1 bis ⌘4 | Agent (vitest) | – | bestanden | `apps/studio/src/app/useShortcuts.test.tsx` (⌘4 und ⌘1 im Bereich Pokémon); im Browser über das Menü im Listenkopf: „Pokédex 508 ⌘1“, „Trainer, kommt mit Phase 3b.3b ⌘2“ (gesperrt), „Begegnungen, kommt mit Phase 3b.3b ⌘3“ (gesperrt), „Attacken 471 ⌘4“. |
| Tastatur | Agent | `pt-3b3a-gate` | bestanden | Liste: Pfeile, Pos1, Ende, Bild ab, Enter wählt, `:focus-visible` nach Taste `true`. Reiter der Hand: Pfeil rechts wechselt (automatische Aktivierung). Tabelle „Lernset“: Tab auf die Zeile, Pfeil ab zur nächsten Zeile, Enter in die Felder („Level von Heuler“), Escape zurück zur Zeile, Tab verlässt die Tabelle. TM/VM-Raster: ein Tab-Stopp, Pfeil rechts TM02, Pfeil ab TM12, Ende VM08. Filter-Plaketten sind je ein Tab-Stopp (sechs Stopps vor der Liste). |
| Dunkel und Kompakt | Agent + Blick | `pt-3b3a-gate` | Bilder erstellt; Blick offen | `pokedex-staravia-dunkel-kompakt.jpg`, `attacken-fluegelschlag-dunkel-kompakt.jpg`, `gegenstaende-trank-dunkel-kompakt.jpg` (`data-theme="dunkel"`, `data-dichte="kompakt"`). Im Dunkeln verschwindet der dunkle DS-Grund der Beschreibung fast im Hintergrund der Schau; die Textbox selbst steht unten im Bild. |
| HeartGold DE | Agent | `hg-3b3a-gate` | bestanden | Plakette „HeartGold/SoulSilver, DE“; Pokédex lädt („493 Arten, 12 Formen“), Namen deutsch (STARAVIA 0397, „Fähigkeit: Bedroher“); Palette „0017“: TAUBOGA, Flügelschlag, Trank 300 ₽; `data_get` Trank 17: Preis 300, Beschreibung „Hilft bei Wunden. …“ (Archiv 221), Flügelschlag 17: Stärke 60 (Archiv 749); Liste „Gegenstände“ 537, „Trank 0017 300 ₽“ mit Symbol. Bilder `hg-pokedex-staravia.jpg`, `hg-gegenstaende-trank.jpg`. |

Die Bilder des Browser-Fensters liegen in `target/agent/gate-2026-10-24/abnahme-3b3a/browser/` (Worktree der Abnahme), das Bild des Emulators in `target/agent/gate-2026-10-24/abnahme-3b3a/pt-daten/start.png`.

Befunde der Abnahme, im Branch behoben:

- Renderzeit (A7): eine Suche zeichnete Schau und Hand des Bereichs neu, die Pokédex-Liste brauchte 61 bis 64 ms. Die Suche hat jetzt einen eigenen Kontext (`useDataSearch`); Messung oben.
- Leere Schau ohne Taste (Design-System „Leere Zustände“: ein Satz, was fehlt, ein Satz, wozu, eine Taste): die Schau von Pokédex, Attacken und Gegenstände hat jetzt „Erste Art zeigen“, „Erste Attacke zeigen“ und „Ersten Gegenstand zeigen“ (erstes Member über 0).
- Pokédex-Farbe 6 heißt „Violett“ wie in den Spieltexten.

Kleinigkeiten ohne Gate-Bezug (notiert, nicht behoben):

- Trainer-Plaketten in „Fundorte“ und „Verwendet in“ nennen die Nummer doppelt („Trainer 0027 Robert“ und grau 0027); mit den Klassennamen in 3b.3b („Wanderer Kai“) neu fassen.
- Der Listenkopf kürzt „493 Arten, 12 Formen“ in Komfort bei 272 px auf „493 Arten, 12 Form…“ (so gewollt: der Untertitel weicht).
- Eine gesperrte Gegenstandszeile zeigt statt des Symbols den Buchstaben, obwohl die Item-Tabelle das Symbol kennt; der Name der gesperrten Art wird in der Zeile gekürzt („STARA… 0…“).
- Die Reiter-Punkte „ungespeichert“ hängen am Asset: Angriff ändern setzt den Punkt an „Werte“, „Fähigkeiten und Typen“ und „TM/VM“ (alle drei sind Personal-Daten).
- Die Filter-Plaketten sind je ein Tab-Stopp statt einer Gruppe mit Pfeiltasten.

Ergebnis von `scripts/gate.sh` (2026-10-24, Worktree der Abnahme, `G4_ROMS_DIR` absolut):

```
Step                                                 Project                                      Result                     Seconds
cargo fmt --check                                    -                                            ok                               1
cargo clippy (-D warnings)                           -                                            ok                              24
cargo test (no ROM data)                             -                                            ok                              59
pnpm test                                            -                                            ok                              73
tokens                                               -                                            ok                               0
styles                                               -                                            ok                               2
pnpm build                                           -                                            ok                              22
dist has no bridge                                   -                                            ok                               0
g4emu unit                                           -                                            ok                               3
g4emu integration                                    -                                            ok                               7
gated tests (cargo test)                             Pokemon - Goldene Edition HeartGold (Germany) ok                             484
gated tests (cargo test)                             Pokemon - HeartGold Version (USA)            ok                             631
gated tests (cargo test)                             Pokemon - Platin-Edition (Germany)           ok                             475

Gate: ok (1781 s)
```

Testkopien der Abnahme (unter `_testkopien`, der Nutzer räumt sie auf): `pt-3b3a-gate` (Oberfläche; Staravia hat TM01 gespeichert), `pt-3b3a-szenario` (erster, abgebrochener Szenario-Lauf, gespeichert und gebaut), `pt-3b3a-szenario-2` (Szenario), `pt-3b3a-gate-lock` (36-Byte-Member), `pt-3b3a-gate-0017` (Item-Tabelle überschrieben), `hg-3b3a-gate` (HeartGold DE).

Beim Nutzer offen: der Blick auf die drei Editoren gegen die Screens „Pokédex: Staravia bearbeiten“, „Attacken: Flügelschlag“, „Gegenstände: Trank“ (A3), auf die Item- und Pokémon-Symbole (A6, 3) und auf Dunkel und Kompakt der beiden Bereiche; Bilder und Orte stehen in der Tabelle.


### Bekannt vor der Abnahme

Befunde aus der Umsetzung, die nicht als Fehler zählen oder bewusst offen sind:

- Trainer und Begegnungen stehen im Umschalter, sind aber gesperrt („kommt mit Phase 3b.3b“); Trainer-Plaketten in „Fundorte“ und „Verwendet in“ sind gesperrt („Trainer öffnen kommt mit Phase 3b.3b.“). „Verwendet in“ zählt ohne Scripts.
- In Platin DE teilen sich Gegenstand 0 („Keines“) und die IDs 113 bis 134 den Datenmember 0 (23 IDs); die Hand warnt, dass Preis und Werte für alle gelten, und das Label nennt es.
- Die Pokédex-Farbe 6 hieß in der Umsetzung „Lila“; seit der Abnahme heißt sie „Violett“ wie in den Texten der Spiele („VIOLETT“).
- Die erste Liste nach dem Öffnen eines Projekts braucht in der Debug-Brücke rund eine Sekunde (die Member werden zum ersten Mal von der Platte gelesen), danach wenige Millisekunden.
- Der Wechsel des Unterbereichs zeichnet die ganze App neu (rund 50 ms im Dev-Build), unabhängig von der Liste.
- Die DS-Textbox der Beschreibungen ist doppelt groß; in einem normalen Fenster muss man in der Schau scrollen, um „Verwendet in“ zu sehen.
- Die Hand-Höhe merkt sich die App je Bereich, nicht je Unterbereich (war für 3b.3b vorgesehen, bleibt offen; siehe Teil B, „Bekannte Grenzen“).

## Teil B: Gate 3b.3b

Phase 3b.3b bringt den Trainer als zusammengesetztes Asset (Eigenschaften, Team, Name, Kampftexte) mit „Trainer anlegen…“ und der Verbindung zu Objekt-Events in der Welt („In der Welt platzieren“, Feld „Trainer“ im Inspector), die Begegnungen mit Anteilen, Ersatzlisten und Warnungen aus dem Kernel, „Begegnungsdatei anlegen…“, das Feld „Begegnungen“ im Header-Editor und „Für diesen Ort abspalten“, die Pokémon-Karte (`PokemonCard`), die Kampfschritte von `g4emu` und zwei Szenarien. Plan: [docs/superpowers/plans/2026-10-24-phase-3b3b-trainer-encounters.md](superpowers/plans/2026-10-24-phase-3b3b-trainer-encounters.md). Spezifikation §1, Gate 3b.3b:

> Auf Kopien von Platin DE und HeartGold DE entstehen zwei neue Trainer mit je einem Pokémon eindeutiger Art und Stufe, platziert als Events in Sichtlinie auf einem neuen Ort, und eine neue Begegnungsdatei am Header dieses Orts; `g4emu` läuft hinein, `expect-battle` liest Art und Stufe des Gegners aus dem RAM, gewinnt beide Kämpfe und löst im Gras eine Begegnung mit der gesetzten Art aus. Eine geänderte Vanilla-Begegnungsdatei (Route 201, Slot 0) wird im Build geprüft. Screenshots belegen für den Menschen; der Beweis ist der RAM.

Teil B besteht aus diesen Punkten:

- (B1) Automatik: die gated Tests von 3b.3b grün auf Platin DE, HeartGold DE und HeartGold US, `scripts/gate.sh` grün.
- (B2) Szenario `pt-trainer-begegnung.yaml` (Platin DE).
- (B3) Szenario `hg-trainer-begegnung.yaml` (HeartGold DE).
- (B4) Trainer-Editor über die Test-Umgebung: Liste, Schau, Hand, „Trainer anlegen…“, Kampftexte mit Warnung.
- (B5) Begegnungen-Editor über die Test-Umgebung: Liste, „Wer hier wartet“, Methodenreiter, Ersatzlisten, Warnungen, „Begegnungsdatei anlegen…“, Feld „Begegnungen“ im Header-Editor.
- (B6) Platzieren: „In der Welt platzieren“ und das Feld „Trainer“ im Inspector.
- (B7) ⌘Z: Felder von Trainer und Begegnungen je ein Schritt; das Anlegen ist eine Undo-Grenze.
- (B8) „Für diesen Ort abspalten“.
- (B9) Tastatur in den beiden neuen Unterbereichen.
- (B10) Dunkel und Kompakt.
- (B11) Blick auf die Screens „Trainer: Wanderer Kai“, „Begegnungen: Route 201“ und „Neuer Trainer: Team“.
- (B12) DSPRE-Gegenprobe.
- (B13) HeartGold US im Spiel (optional).

Wer was prüft (Legende wie in Teil A): **Agent** für die Szenarien, die Editoren über die Test-Umgebung, ⌘Z, Abspalten, Tastatur und Dunkel/Kompakt (mit Blick des Nutzers auf die Bilder); **Hand** für den Blick auf die Screens „Trainer“ und „Begegnungen“, DSPRE (nur Windows) und HeartGold US im Spiel. Rezepte: [docs/agent-gate.md](agent-gate.md), Abschnitte „Phase 3b.3b“ und „Rezepte“ („Trainer“, „Begegnungen“, „Platzieren“).

Namen in der Oberfläche: Im Bereich Pokémon sind alle vier Unterbereiche offen (⌘1 Pokédex, ⌘2 Trainer, ⌘3 Begegnungen, ⌘4 Attacken); die Palette hat „Trainer öffnen“ und die Gruppen „Trainer“ und „Begegnungen“. Brotkrumen „Pokémon › Trainer › Wanderer Jens“ und „Pokémon › Begegnungen › Route 201“.

### Vorbereitung

1. Kopien anlegen (nie ein Original; das Ziel darf nicht existieren): je eine frische Kopie für jedes Szenario (`pt-3b3b`, `hg-3b3b`; existiert der Name, mit Endung `-2` …) und für die Oberfläche (`pt-3b3b-gate`, `hg-3b3b-gate`).
2. Werkzeuge: `cargo build --workspace --all-features` (legt `target/debug/studio-bridge` und `target/debug/g4k` an; `cargo test --no-run` erneuert `studio-bridge` nicht immer). Aus einem Worktree `G4_ROMS_DIR`, `G4_BRIDGE_BIN` und `G4K_BIN` absolut setzen.
3. Für die Oberfläche: Test-Umgebung wie in Teil A, Vorbereitung 2; vor „Trainer anlegen…“ im Bereich Welt, Reiter „Script“, „Scripts übernehmen“ (ohne das lehnt das Anlegen ab: „Erst Scripts übernehmen (Bereich Welt, Panel Script)“).

### (B1) Automatik

- [x] 1. Gated Tests von 3b.3b auf allen drei Projekten (laufen in `scripts/gate.sh` mit, Liste in [dev-setup.md](dev-setup.md), „Trainer und Begegnungen (3b.3b)“): `vanilla_trainers_decode_and_encode_identically` (jeder Trainer als ein Dokument, byte-gleich zurück), `vanilla_trainers_load_unlocked_through_the_store`, `vanilla_encounter_files_raise_no_warnings` (Vanilla-Begegnungen brechen keine Regel, jede Warnung zeigt also auf eine Änderung), `vanilla_creates_trainers_behind_the_sight_slot` (auf einer Kopie: Platin 928, Platzhalter 929, 930; HeartGold 738, 739, Platzhalter 740, 741; die Kampf-Scriptdatei kompiliert sauber, ein neu geöffneter Store zeigt den Platzhalter gesperrt). Dazu bleiben die Tests aus Teil A grün.
- [x] 2. `scripts/gate.sh` grün gegen alle drei Projekte; Tabelle unter „Ergebnis Teil B“.

### (B2) Szenario `pt-trainer-begegnung.yaml`

Aus `scripts/g4emu`, auf einer frischen Kopie:

```sh
G4_ROMS_DIR=<Roms> G4_BRIDGE_BIN=<Worktree>/target/debug/studio-bridge G4K_BIN=<Worktree>/target/debug/g4k \
uv run g4emu scenario ../agent/scenarios/pt-trainer-begegnung.yaml \
  --project <Roms>/_testkopien/pt-3b3b --out <Worktree>/target/agent/gate-<datum>/pt-trainer-begegnung \
  --set NEUER_HEADER=593 --set S_WILD=150 --set L_WILD=5 --set S_T1=25 --set L_T1=12 \
  --set S_T2=133 --set L_T2=14 --set S_GIFT=445
```

- [x] 1. Änderungen über die Brücke: Patch „Dynamische Header“, „Scripts übernehmen“, `create_area` aus den Karten 5 und 6 von Route 201 als neuer Header 593 mit leerer Eventdatei 534; `create_encounter_file` (183), `new_script_file` (1124), `set_header` mit `wild_pokemon` 183; Gras-Rate 100, alle 12 Gras-Slots und die Tag/Nacht-Listen auf Mewtu (150) Lv 5; Route 201 (Datei 140) Gras-Slot 0 auf 150.
- [x] 2. `create_trainer` Klasse 9 (Wanderer) „Jens“ → 928, „Mia“ → 930 mit Platzhalter 929 und erweiterter Kampf-Scriptdatei 1114; je ein Pokémon (Pikachu Lv 12, Evoli Lv 14), Kampftexte vor dem Kampf (mit `\r`), Niederlage, danach. Objekte mit Script 3927 bzw. 3929, `trainer_type` 1, Sichtweite 3; Jens sieht nach Norden quer zum Weg, Mia nach Süden.
- [x] 3. Geber: Objekt auf dem neuen Ort, Script 2 der neuen Datei gibt einmalig Knakrack (445) Lv 100 über ein Flag (`FLAG_UNUSED_0x0094`), Textliteral „Hier, nimm das.“; Ankunfts-Warp mit Tür-Kachel, Treppe 394/0 auf 593. `save`, `build_rom`.
- [x] 4. Prüfschritt `g4k check-data`: `encounter:140:grass0=150`, `encounter:183:grass_rate=100`, `encounter:183:grass0=150`, `encounter:183:grass11_level=5` → „4 expectation(s) met“.
- [x] 5. Spielen: Geber ansprechen; Jens kämpft per **Sichtlinie** (`expect-battle 25 12`), Mia wird **angesprochen** (`press a`) und kämpft über ihren eigenen Eintrag der erweiterten Kampf-Scriptdatei (`expect-battle 133 14`). `expect-battle` liest nur Art und Level und unterscheidet Sicht und Ansprechen nicht; dass Mias Kampf durch Ansprechen beginnt, zeigt die Anordnung: sie sieht nach Süden, vom Weg weg, der Spieler stellt sich neben sie (53, 21), dreht sich zu ihr (`press right:8`) und drückt A; beide `win-battle` enden auf 593 (nicht zu Hause); `walk-until-battle up 40 back-and-forth`, `expect-battle 150 5`. Bilder `trainer1`, `trainer2`, `wild`.

### (B3) Szenario `hg-trainer-begegnung.yaml`

Wie (B2) mit `hg-trainer-begegnung.yaml`, Kopie `hg-3b3b`, `--set NEUER_HEADER=540` und denselben Arten.

- [x] 1. `create_area` aus Route 29 Karte 1 als neuer Header 540 (Vorlage 33), Eventdatei 491; `create_encounter_file` (142), `new_script_file` (965); Gras-Rate 100, alle 12 Slots in den Spalten Morgens, Tags und Nachts auf 150, Level 5; Route 29 (Datei 1) Gras-Slot 0 in allen drei Spalten auf 150.
- [x] 2. `create_trainer` dreimal: „Jens“ → 738, „Lea“ → 739, „Mia“ → 741 mit Platzhalter 740 und erweiterter Kampf-Scriptdatei 0953. 739 („Lea“) bekommt weder Team noch Objekt; er ist nur da, damit der dritte Trainer hinter dem Platzhalter liegt und das Gate auch in HeartGold die Erweiterung beweist.
- [x] 3. Geber mit `GiveMon` 445 Lv 100 einmalig über `FLAG_UNK_0A1`, Ankunft (24, 16) als Tür, Treppe 61/0 auf 540. `save`, `build_rom`.
- [x] 4. `g4k check-data`: Datei 1 `grass0`, `grass0_morning`, `grass0_night` = 150; Datei 142 `grass_rate` 100, `grass0`, `grass11_morning`, `grass11_night` = 150, `grass11_level` 5 → „8 expectation(s) met“.
- [x] 5. Spielen: Jens (738) per Sichtlinie (`expect-battle 25 12`), Mia (741) angesprochen (`expect-battle 133 14`; sie sieht nach Norden, vom Weg in Zeile 16 weg, der Spieler stellt sich neben sie (15, 16), `press left:8`, A; auch hier belegt die Anordnung das Ansprechen, nicht `expect-battle`), beide Siege enden auf 540, wild `expect-battle 150 5`. Bilder `trainer1`, `trainer2`, `wild`.

Abweichungen von der Spezifikation (§13): Die Spezifikation nennt Wanderer „Jens“ und Göre „Mia“, die Szenarien legen beide als Wanderer an (eine Klasse mit sichtbarer Figur, Grafik 20 bzw. 333). Der Geber steht auf dem **neuen Ort**, nicht im Zimmer: Figuren der Karte, auf der der Spielstand gemacht wurde, kommen nach `continue` aus dem Spielstand, eine neue Figur im Zimmer wäre nicht da. Für das Gras nutzen die Szenarien `walk-until-battle up 40 back-and-forth` statt `up 2`. HeartGold legt drei Trainer an statt zwei: 739 („Lea“, ohne Team und ohne Figur) nur, damit der zweite gekämpfte Trainer (741) hinter dem Platzhalter 740 liegt und die Erweiterung auch dort bewiesen ist. Statt `walk-until-battle right 6` laufen die Szenarien gezielt in die Sichtlinie (`walk-to`, `walk`) bzw. sprechen Mia an, damit ein Kampf über den eigenen Eintrag der erweiterten Kampf-Scriptdatei läuft (ein Sichtlinien-Kampf nutzt immer den „gesehen“-Eintrag und beweist die Erweiterung nicht). Die Prüfung der Route-201-Datei im Build liest die Begegnungen mit dem Codec des Projekts, nicht mit uxie (dessen Begegnungs-Leser ist unbrauchbar, siehe [dev-setup.md](dev-setup.md)).

### (B4) Trainer-Editor

Auf `pt-3b3b-gate` (Scripts übernommen), Tiefe „Bauen“, Rezept „Trainer“ in [agent-gate.md](agent-gate.md):

- [ ] 1. ⌘2 (oder Menü im Listenkopf): Liste „928 Trainer“ als Baum „Nach Ort“ (Ort → Trainer, „Ohne Ort“ zuletzt), Umschalter „Nach Klasse“, Suche über Name, Klasse und Nummer, Zeilen mit „Lv. 12“ (höchstes Level im Team).
- [ ] 2. Ein Vanilla-Trainer (Teenager Logan 0002): Schau mit Team als Karten und „Steht in der Welt“ („Route 202, Kachel 185, 804“, Klick führt in die Welt); Hand mit den Reitern „Team“ (sechs Karten, Platz-Details: Art, Level, Stärke, Item, vier Attacken), „Trainer“ (Klasse, Name „N von 7“, Kampfart, KI, Schalter „Eigene Attacken“ und „Items tragen“, vier Items, Kampftexte) und in Tüfteln „Rohdaten“.
- [ ] 3. „Trainer anlegen…“ (+): Dialog „Trainer 928 anlegen?“ mit Klasse, Name und der Plakette „Nicht rückgängig“; ein Name mit 8 Zeichen wird abgelehnt. Anlegen wählt den neuen Trainer, die Schau warnt „Ohne Pokémon stürzt der Kampf ab“. Der zweite: der Dialog nennt „Trainer 930 anlegen?“ und „929 wird Platzhalter für das Sicht-Script“; die Liste zeigt 929 gesperrt.
- [ ] 4. Team: Platz 1 → Art wählen (neue Attacken nach dem Lernset, wenn „Eigene Attacken“ an ist), Level ändern; der Verlauf nennt „Wanderer Jens: Platz 1 Level 5 → 14“.
- [ ] 5. Kampftexte: „Text anlegen“ vor dem Kampf ohne `\r` → Warnung im Feld und im Prüflicht, „⏎ anhängen“ behebt sie.
- [ ] 6. HeartGold DE (`hg-3b3b-gate`): Geschlecht und Fähigkeit als zwei Auswahlfelder, „Stärke“ mit „IV = Wert × 31 / 255“.

### (B5) Begegnungen-Editor

Auf `pt-3b3b-gate`, Rezept „Begegnungen“:

- [ ] 1. ⌘3: Liste „183 Begegnungsdateien“, Zeilen mit den Orten der Header („Route 201“, „verwendet von 1 Ort“), Gruppe „Ohne Ort“ am Ende; Suche „Ort oder Nummer“.
- [ ] 2. Route 201 (0140): Schau „Wer hier wartet“ (STARALILI Morgens 50 % · Tags 50 % · Nachts 40 % …) und „Verwendet von 1 Ort“; Hand mit den Reitern Gras, Surfen, Alte Angel, Profi-Angel, Super-Angel, Ersatz (HeartGold dazu „Fels“), je mit Zähler; Tabelle Platz · Pokémon · Level · Anteil · Ersetzt durch.
- [ ] 3. Gras-Slot 1 auf eine andere Art, Rate 30 → 100: Verlauf „Route 201: Gras Platz 1 STARALILI → PIKACHU“, „Route 201: Gras-Rate 30 → 100“; die Schau rechnet die Anteile neu (aus dem Kernel).
- [ ] 4. Warnung: Level 0 bei belegter Art → Warnung in der Hand, in der Schau und im Prüflicht (Quelle „Begegnungen“).
- [ ] 5. Ersatz-Reiter: je Liste eine Tabelle (Platin: Schwarm, Tag, Nacht, Poké-Radar, Dual-Slot Rubin bis Blattgrün; HeartGold: Schwarm im Gras, beim Surfen, beim Angeln, Radio Hoenn und Sinnoh) mit „Ersetzt“ und „Anteil“; Platin „Formen“ mit „Schalellos Ost“/„Gastrodon Ost“ und „Icognito-Tabelle“.
- [ ] 6. „Begegnungsdatei anlegen…“ (+): Dialog „Begegnungsdatei 0183 anlegen?“, „Leer“ oder „Kopie von …“, Plakette „Nicht rückgängig“.
- [ ] 7. Header-Editor (Welt): Feld „Begegnungen“ als Referenz „Begegnungen 0140“ mit „Ändern…“ (Auswahl der Dateien, „Keine“ = Platin 0xFFFF, HeartGold 0xFF); das Umhängen ist ein Schritt („Begegnungen“).
- [ ] 8. HeartGold DE: Gras mit den Spalten Morgens, Tags, Nachts und einer Level-Spalte; Label „…: Gras Platz 1 nachts Hoothoot → Taubsi“.

### (B6) Platzieren

Auf `pt-3b3b-gate`, Rezept „Platzieren“:

- [ ] 1. Ein Trainer ohne Figur: „In der Welt platzieren“ wechselt in die Welt, Werkzeug „Figur“, Statusleiste „Figur · Klick setzt Wanderer Jens · Esc hebt die Bindung auf“. Ein Klick in die Szene legt **eine** Figur mit Script 3000 + ID − 1, Sichtlinie an, Sichtweite 3 an (ein Schritt „Trainer Jens platziert“); danach steht wieder „Auswählen“. Escape, ein anderes Werkzeug oder ein anderer Bereich heben die Bindung auf.
- [ ] 2. Inspector einer Figur: Feld „Trainer“ (Referenz „Wanderer Jens 0928“, öffnet den Trainer; ohne Trainer-Script „kein Trainer“); „Trainer ändern…“ setzt das Script (und Sichtlinie und Sichtweite, wenn beide 0 waren) in einem Schritt; eine Partner-Figur (Script 5000–6999) behält den Partner-Bereich. „An diesem Ort“ zeigt „Objekt 0 · Teenager Sebastian“.
- [ ] 3. Die Schau des Trainers zeigt danach „Steht in der Welt“ mit dem Ort und der Kachel.

### (B7) ⌘Z

- [ ] 1. Jede Änderung in Trainer und Begegnungen ist ein Schritt mit sprechendem Label; ⌘Z nimmt ihn zurück, „Wiederholen“ stellt ihn her.
- [ ] 2. Anlegen ist nicht rückgängig: steht „Trainer N angelegt“ oben im Verlauf, meldet ⌘Z „Trainer N anlegen lässt sich nicht rückgängig machen“ und ändert nichts (auch nicht die Schritte davor). Eine angelegte Begegnungsdatei ist kein Schritt im Verlauf; sie bleibt.
- [ ] 3. Platzieren: ein ⌘Z entfernt die platzierte Figur ganz (ein Schritt).

### (B8) Für diesen Ort abspalten

- [ ] 1. Vanilla Platin hat keine Datei mit mehreren Orten: im Header-Editor von Zweiblattdorf (411) „Begegnungen“ auf 0140 stellen. Die Schau von 0140 zeigt „Verwendet von 2 Orten“ und je Ort „Für diesen Ort abspalten“.
- [ ] 2. „Zweiblattdorf: für diesen Ort abspalten…“: Dialog „Zweiblattdorf eine eigene Begegnungsdatei geben?“, Plakette „Nicht rückgängig: die Kopie“, „Das Umhängen des Headers ist rückgängig.“; danach ist die Kopie gewählt und heißt „Zweiblattdorf“.
- [ ] 3. ⌘Z hängt den Header zurück; die Kopie bleibt („von keinem Ort verwendet“).

### (B9) Tastatur

- [ ] 1. Trainer-Baum: Pfeile, Pos1, Ende, Pfeil rechts/links öffnet und schließt eine Gruppe, Enter wählt; Team-Karten mit Enter/Leertaste, Reiter der Hand mit Pfeiltasten.
- [ ] 2. Begegnungen: Liste mit Pfeilen und Enter, Methodenreiter mit Pfeiltasten, Tabelle mit Tab auf die Zeile, Pfeil ab, Enter in die Felder, Escape zurück.
- [ ] 3. Dialoge „Trainer anlegen…“ und „Begegnungsdatei anlegen…“: Escape bricht ab, Fokus bleibt sichtbar (`:focus-visible`).

### (B10) Dunkel und Kompakt

- [ ] 1. Trainer (Schau mit Team-Karten, Hand „Team“) und Begegnungen (Schau „Wer hier wartet“, Hand „Gras“) in Dunkel und Kompakt (`data-theme="dunkel"`, `data-dichte="kompakt"`); Galerie-Seite „Daten“ mit „Team-Karten“ (Ruhe, Gewählt, Leer). Der Nutzer sieht auf unlesbare Stellen.

### (B11) Blick auf die Screens (Hand)

- [ ] 1. „Trainer: Wanderer Kai“, „Neuer Trainer: Team“ und „Begegnungen: Route 201“ auf der Seite „Screens: Pokémon und Kämpfe“ in `design-system/Editor_DesignSystem.fig` gegen die Bilder aus (B4), (B5) und (B10): wiedererkennbar, ohne Kampfsprites (kommen mit 3d) und ohne Live-Vorschau des Assistenten.

### (B12) DSPRE-Gegenprobe (Hand)

- [ ] 1. DSPRE (Windows) öffnet die gespeicherte Szenario-Kopie `pt-3b3b`: Trainer 928 „Jens“ und 930 „Mia“ mit ihrem Pokémon und den Texten, Header 593 mit der Begegnungsdatei 183. DSPRE läuft nur unter Windows.

### (B13) HeartGold US im Spiel (Hand, optional)

- [ ] 1. `hg-trainer-begegnung.yaml` mit `game: IPKE` auf einer Kopie von HeartGold US. Für IPKE ist der Kampfzeiger nicht gemessen (die Erkennung stützt sich dort nur auf die Signatur, siehe „Bekannte Grenzen“); ein Fehlschlag dort ist kein Gate-Fehler, nur ein Befund.

### Ergebnis Teil B

Stand 2026-10-24, nach der letzten Fix-Welle (HEAD `99ca5e2`), aus dem Worktree mit absolutem `G4_ROMS_DIR`. Bildnummern (`frame`) und Siegpositionen der Szenarien stammen aus der Konsolenausgabe der Läufe, gesichert als `target/agent/gate-2026-10-24/final/<szenario>/console.log`.

| Punkt | Wer | Datum | Projekt | Ergebnis | Auffälligkeiten |
|---|---|---|---|---|---|
| (B1) 1 gated Tests 3b.3b | Agent | 2026-10-24 | Platin DE, HeartGold DE, HeartGold US (Originale, nur lesend; schreibende Tests auf `vanilla_copy()`) | bestanden | laufen in `scripts/gate.sh` mit, siehe unten |
| (B1) 2 `scripts/gate.sh` | Agent | 2026-10-24 | Platin DE, HeartGold DE, HeartGold US | bestanden („Gate: ok (1798 s)“) | Log `target/agent/gate.log`; Tabelle unten |
| (B2) Szenario `pt-trainer-begegnung.yaml` | Agent | 2026-10-24 | `pt-3b3b-final` | bestanden | Rückgabe 0, `"ok": true`, Header-Folge 415, 593. Antworten: Ort `{matrix 289, maps [[0,0,666],[1,0,667]], event_file 534, header 593}`, Begegnungsdatei 183, Scriptdatei 1124, `trainer1` `{id 928, placeholder null, script_extended false}`, `trainer2` `{id 930, placeholder 929, script_extended true}`. `check.log`: „4 expectation(s) met“. Kämpfe: Jens (Sichtlinie) `{"species": 25, "level": 12}` Bild 2322, Sieg auf 593 (50, 21); Mia (angesprochen) `{"species": 133, "level": 14}` Bild 4399, Sieg auf 593 (53, 21); wild nach 5 Schritten bei (48, 19) `{"species": 150, "level": 5}` Bild 6539. Bilder `target/agent/gate-2026-10-24/final/pt-trainer-begegnung/trainer1.png` („Eine Herausforderung von Wanderer Jens!“), `trainer2.png` („… Wanderer Mia!“), `wild.png` („Ein wildes MEWTU erscheint!“), dazu `ankunft.png`, `geschenk.png`. Lauf 32 s. |
| (B3) Szenario `hg-trainer-begegnung.yaml` | Agent | 2026-10-24 | `hg-3b3b-final` | bestanden | Rückgabe 0, `"ok": true`, Header-Folge 64, 540. Antworten: Ort `{matrix 288, maps [[0,0,676]], event_file 491, header 540}`, Begegnungsdatei 142, Scriptdatei 965, `trainer1` `{id 738}`, `trainer_lea` `{id 739}`, `trainer2` `{id 741, placeholder 740, script_extended true}`. `check.log`: „8 expectation(s) met“. Kämpfe: Jens (Sichtlinie) `{"species": 25, "level": 12}` Bild 2092, Sieg auf 540 (20, 16); Mia (angesprochen) `{"species": 133, "level": 14}` Bild 4236, Sieg auf 540 (15, 16); wild nach 5 Schritten bei (17, 15) `{"species": 150, "level": 5}` Bild 6415. Bilder `target/agent/gate-2026-10-24/final/hg-trainer-begegnung/trainer1.png`, `trainer2.png`, `wild.png` („Ein wildes MEWTU erscheint!“, MEWTU Lv5). Lauf 36 s. |
| (B4) Trainer-Editor | Agent | – | `pt-3b3b-gate`, `hg-3b3b-gate` | offen | in der Umsetzung im Browser geprüft (Kopien `pt-3b3b-t6`, `pt-3b3b-t8`), nicht als Abnahme |
| (B5) Begegnungen-Editor | Agent | – | `pt-3b3b-gate`, `hg-3b3b-gate` | offen | in der Umsetzung im Browser geprüft (`pt-3b3b-t7`, `hg-3b3b-t7`) |
| (B6) Platzieren | Agent | – | `pt-3b3b-gate` | offen | in der Umsetzung im Browser geprüft (`pt-3b3b-t8`) |
| (B7) ⌘Z | Agent | – | `pt-3b3b-gate` | offen | Undo-Grenze beim Anlegen durch Kernel- und vitest-Tests belegt |
| (B8) Abspalten | Agent | – | `pt-3b3b-gate` | offen | in der Umsetzung im Browser geprüft (`pt-3b3b-t7`) |
| (B9) Tastatur | Agent | – | `pt-3b3b-gate` | offen | |
| (B10) Dunkel und Kompakt | Agent + Blick | – | `pt-3b3b-gate` | offen | |
| (B11) Screens | Hand | – | – | offen | |
| (B12) DSPRE | Hand | – | `pt-3b3b` | offen | nur Windows |
| (B13) HeartGold US | Hand (optional) | – | – | offen | IPKE ohne gemessenen Kampfzeiger |

Die Szenarien liefen zuerst in der Umsetzung (Task 9a/9b, Kopien `pt-3b3b`, `hg-3b3b`, gleiche Ergebnisse) und nach der letzten Fix-Welle noch einmal auf den frischen Kopien `pt-3b3b-final` und `hg-3b3b-final` (Tabelle oben).

Ergebnis von `scripts/gate.sh` (2026-10-24, Worktree von 3b.3b, `G4_ROMS_DIR` absolut):

```
Step                                                 Project                                      Result                     Seconds
cargo fmt --check                                    -                                            ok                               1
cargo clippy (-D warnings)                           -                                            ok                               1
cargo test (no ROM data)                             -                                            ok                              35
pnpm test                                            -                                            ok                              77
tokens                                               -                                            ok                               1
styles                                               -                                            ok                               1
pnpm build                                           -                                            ok                              24
dist has no bridge                                   -                                            ok                               0
g4emu unit                                           -                                            ok                               6
g4emu integration                                    -                                            ok                               6
gated tests (cargo test)                             Pokemon - Goldene Edition HeartGold (Germany) ok                             605
gated tests (cargo test)                             Pokemon - HeartGold Version (USA)            ok                             662
gated tests (cargo test)                             Pokemon - Platin-Edition (Germany)           ok                             379

Gate: ok (1798 s)
```

Testkopien von Teil B (unter `_testkopien`, der Nutzer räumt sie auf): `pt-3b3b`, `hg-3b3b` (Szenarien der Umsetzung), `pt-3b3b-final`, `hg-3b3b-final` (Szenarien nach der Fix-Welle), dazu die Kopien der Umsetzung (`pt-3b3b-t6`, `-t7`, `-t8`, `hg-3b3b-t7`, `pt-3b3b-9a-probe`, `pt-3b3b-9a-try1`, `hg-3b3b-9b-probe`, `hg-3b3b-9b-try1`) und der Recherche (`*-3b3b-enc`, `*-3b3b-trainer`, `*-3b3b-trscript`, `pt-3b3b-vanilla`).

### Bekannte Grenzen

Befunde aus der Umsetzung, die nicht als Fehler zählen oder bewusst offen sind:

- **Anlegen ist eine Undo-Grenze.** „Trainer N angelegt“ steht im Verlauf, ⌘Z verweigert dort mit „Trainer N anlegen lässt sich nicht rückgängig machen“. Auch Schritte **vor** dem Anlegen lassen sich in dieser Sitzung nicht mehr zurücknehmen (die Grenze bleibt, bis das Projekt neu geöffnet wird). Die Spezifikation verlangt „Anlegen ist nicht rückgängig“; der Plan sah die Namens- und Offset-Änderungen als einen rückgängigen Schritt vor, die Spezifikation gilt.
- **HeartGold: Trainer 739 ohne Team.** Das HeartGold-Szenario legt „Lea“ (739) mit leerem Team und ohne Figur an, nur damit der dritte Trainer (741) hinter dem Platzhalter 740 liegt. Ein Kampf gegen 739 würde abstürzen („Ohne Pokémon stürzt der Kampf ab“); er steht nirgends in der Welt.
- **HeartGold: Flag des Gebers.** `FLAG_UNK_0A1` nutzt kein Vanilla-Script; ob der C-Code des Spiels es setzt oder liest, ist nicht geprüft. Der grüne Lauf zeigt nur, dass es im Spielstand `IPKD-zimmer.sav` nicht gesetzt ist.
- **Textliteral des Gebers im geteilten Archiv.** Der neue Header teilt das Textarchiv seiner Vorlage (Platin 466 von Route 201, HeartGold 373 von Route 29); „Hier, nimm das.“ landet am Ende dieses Archivs. Harmlos für das Gate, bei echten Projekten ein eigenes Archiv anlegen.
- **Prüflicht nur für geöffnete Dokumente.** Warnungen zu Trainern (Text vor dem Kampf ohne `\r`) und Begegnungen (Level 0, min > max, leerer Ersatz-Slot) erscheinen im Prüflicht nur für Dokumente, die in dieser Sitzung geöffnet wurden; eine projektweite Prüfung kommt mit 3b.4.
- **`g4k check-data encounter` liest mit dem Codec des Projekts, nicht mit uxie** (Abweichung von Spezifikation §1, „per uxie geprüft“): der Begegnungs-Leser von uxie 0.9.7 ist unbrauchbar (HGSS um 8 Byte versetzt, leere Slots ungültig). `species` und `item` liest `check-data` weiter mit uxie.
- **HeartGold US: Kampferkennung ungemessen.** IPKE hat keinen Kampfzeiger in `games.yaml`; `expect-battle` stützt sich dort nur auf die Signatur, ob `end-battle`/`win-battle` nach einem Sieg enden, ist nicht gemessen ((B13) optional).
- **Platzhalter.** Die ID des „gesehen“-Slots (Platin 929, HeartGold 740) bekommt einen gesperrten Platzhalter (Liste „Platzhalter für das Sicht-Script“); sie lässt sich nicht bearbeiten und nicht platzieren. Struktur und Grund: [formats/trainer_scripts.md](formats/trainer_scripts.md).
- **Keine Hand-Höhe je Unterbereich.** Die Hand merkt ihre Höhe weiter je Bereich, Pokédex, Trainer, Begegnungen und Attacken teilen sie; das wartet auf den Umbau des Decks.
- **Kampfsprites** von Pokémon und Trainern fehlen (die Team-Karten zeigen die Menüsymbole aus dem ROM); sie kommen mit 3d.
- **Diamant/Perl** bleibt gesperrt („Diamant/Perl: ohne Testprojekt nicht freigegeben“); Löschen von Trainern und Begegnungsdateien kommt mit 3b.4.
