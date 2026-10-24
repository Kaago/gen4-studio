# Phase 3b.3: Gate prüfen

Stand: 2026-10-24. Teil A (Gate 3b.3a „Codecs und Pokémon-Daten“) ist implementiert und automatisch getestet, die Abnahme steht aus. Teil B (Gate 3b.3b „Trainer, Begegnungen“, zugleich das Roadmap-Gate 3b.3) kommt mit dem Plan 3b.3b in diese Datei.

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

- [ ] 1. `vanilla_data` gated grün auf Platin DE, HeartGold DE und HeartGold US: die Tests in `crates/g4-formats/tests/vanilla_data.rs` (`vanilla_fixed_records_round_trip`, `vanilla_variable_records_round_trip`, `vanilla_trainer_parties_round_trip_with_their_properties`, auf Platin dazu `vanilla_pt_water_slots_have_min_le_max`; auf HeartGold meldet dieser „skipped: not a Platinum project“). Ein Ordner mit falscher Member-Zahl, ohne Member oder mit anderen Dateien als `NNNN` lässt den Test fehlschlagen.
- [ ] 2. uxie-Orakel: `vanilla_personal_matches_uxie`, `vanilla_items_match_uxie`, `vanilla_trainer_properties_match_uxie` grün auf allen drei Projekten.
- [ ] 3. Vanilla-Rebuild: `vanilla_rebuild_matches_dsrom_cli` (mit `G4_DSROM_CLI`) grün auf allen drei Projekten; der Test verlangt, dass die Daten-Ordner mitgepackt werden.
- [ ] 4. `scripts/gate.sh` läuft grün gegen alle drei Projekte (die Punkte 1 bis 3 laufen darin mit); keine Zeile „skipped“ außer den dokumentierten. Aus einem Worktree: `G4_ROMS_DIR=<absoluter Pfad zu Roms> scripts/gate.sh`. Die Tabelle aus der Ausgabe kommt unter „Ergebnis“.

Einzeln (ohne das ganze Gate), je Projekt, aus der Repo-Wurzel:

```sh
export PATH="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$PATH"
G4_TEST_PROJECT="<Roms>/Pokemon - Platin-Edition (Germany)_DSPRE_contents" G4_DSROM_CLI="<Roms>/_tools/dsrom" \
  cargo test --workspace --all-features -- vanilla_fixed_records vanilla_variable_records vanilla_trainer_parties \
  vanilla_personal_matches_uxie vanilla_items_match_uxie vanilla_trainer_properties_match_uxie \
  vanilla_pt_water_slots vanilla_rebuild_matches_dsrom_cli --nocapture
```

### (A2) Szenario `pt-daten.yaml`

Auf `pt-3b3a-szenario`, aus `scripts/g4emu`:

- [ ] 1. `uv run g4emu scenario ../agent/scenarios/pt-daten.yaml --project ../../../Roms/_testkopien/pt-3b3a-szenario --out ../../target/agent/gate-<datum>/pt-daten` endet mit Rückgabe 0 und der Zeile `"ok": true`.
- [ ] 2. Die vier `data_edit` (Staravia Angriff 85, Lernset von 12 auf 13 Einträge, Flügelschlag Stärke 65, Trank Preis 250), `save` und `build_rom` sind ok.
- [ ] 3. Der Prüfschritt `g4k check-data out.nds --expect species:397:attack=85 --expect learnset:397:len=13 --expect move:17:power=65 --expect item:17:price=250` meldet „4 expectation(s) met“ (Ausgabe in `<out>/check.log`).
- [ ] 4. Spielen als Starttest: `continue`, `expect-map 415`, `shot start`; das Bild `<out>/start.png` zeigt das Spielerzimmer.
- [ ] 5. Gegenprobe, dass der Prüfschritt beißt: `target/debug/g4k check-data <Kopie>/out.nds --expect species:397:attack=75` endet mit Exit 1 und „species 397 attack: expected 75, found 85“.

### (A3) Editoren gegen die Screens

Auf `pt-3b3a-gate`, Tiefe „Bauen“:

- [ ] 1. Pokédex: Atlas „Pokémon“, Staravia wählen (Suche „Pokédex durchsuchen“, `397`). Screenshot des ganzen Fensters. Zu sehen sein müssen: Listenkopf „Pokédex ▾“ mit „493 Arten, 12 Formen“, Filter „Typ“, Zeilen mit Symbol, Name und Nummer; Schau „Übersicht“ mit Kopfkarte (Symbol, Name, 0397, Typen Normal und Flug, Fähigkeit), sechs Wertebalken mit dem Schnitt aller Arten, „Summe 340“, „Fundorte“ und „Verwendet in“; Hand „Bearbeiten“ mit den Reitern „Werte“, „Fähigkeiten und Typen“, „Lernset“, „Entwicklung“, „TM/VM“; Brotkrume „Pokémon › Pokédex › STARAVIA“, Statusleiste „STARAVIA, 0397“. Der Vergleich mit „Pokédex: Staravia bearbeiten“ ist ein Blick des Nutzers (das DS-Profil des Screens fehlt bewusst, siehe „Später“ in der Spec).
- [ ] 2. Im Reiter „Werte“ Angriff 75 → 85: die Titelleiste sagt „1 Änderung nicht gespeichert“, die Schau „Summe 350, vorher 340“, `call.sh history` nennt „STARAVIA: Angriff 75 → 85“; ⌘Z stellt 75 und „Alles gespeichert“ wieder her.
- [ ] 3. Attacken: ⌘4 (oder der Umschalter), Flügelschlag wählen. Die Karte nennt Name, Nr. 0017, Typ Flug, Physisch, „60 Stärke“, „100 % Genauigkeit“, „35 AP“ und die Merkmale; darunter die Beschreibung als DS-Textbox mit „Beschreibung bearbeiten“ und „Verwendet in“ („23 Lernsets, 11 Trainer“); die Hand hat die Spalten „Grundwerte“ und „Verhalten“. Stärke 60 → 65: Verlauf „Flügelschlag: Stärke 60 → 65“, ⌘Z. Der Vergleich mit „Attacken: Flügelschlag“ ist ein Blick des Nutzers.
- [ ] 4. Gegenstände: Atlas „Gegenstände“, Trank wählen. Brotkrume „Gegenstände › Tasche Medizin › Trank“; die Karte zeigt Symbol, Name, Nr. 0017 und den Preis 300 ₽, darunter die Beschreibung als DS-Textbox und „Verwendet in“; die Hand zeigt die Werte des Gegenstands in Spalten. Preis 300 → 250: Verlauf „Trank: Preis 300 → 250“, die Zeile zeigt „250 ₽“, ⌘Z. Der Vergleich mit „Gegenstände: Trank“ ist ein Blick des Nutzers.

### (A4) ⌘Z über zwei Assets („Wie Staralili“)

Auf `pt-3b3a-gate`, Pokédex, Staravia, Reiter „TM/VM“. In Platin DE hat Staralili dieselben TM/VM wie Staravia; damit die Kopie beide Teile ändert, wird vorher ein Kästchen umgestellt und gespeichert:

- [ ] 1. TM01 einschalten, speichern („Alles gespeichert“).
- [ ] 2. „Von anderer Art übernehmen …“, Staralili wählen: `call.sh history` nennt einen Schritt „STARAVIA: TM/VM und Lernset wie STARALILI“, `call.sh dirty` nennt zwei Assets (Personal-Daten und Lernset von 397), TM01 ist aus, „Lernset“ hat 11 statt 12 Zeilen.
- [ ] 3. Ein ⌘Z: `call.sh dirty` liefert `[]`, TM01 ist wieder an, „Lernset“ hat wieder 12 Zeilen; „Wiederholen“ hält den Schritt.

### (A5) Tüfteln: Hexblock mit markiertem Feld

Auf `pt-3b3a-gate`, im Scharnier „Tüfteln“:

- [ ] 1. Attacken, Flügelschlag: die Hand hat die Spalte „Rohdaten“ (in Bauen nicht). Feld „Stärke“ fokussieren: der Hexblock nennt „files/poketool/waza/pl_waza_tbl.narc, Eintrag 17, 16 Bytes“, Byte 3 ist markiert (`[data-marked]` mit `data-offset="3"`), der Fuß sagt „Byte 3: Stärke, 0x3C = 60“.
- [ ] 2. Pokédex, Staravia: Reiter „Rohdaten“ (nur in Tüfteln) mit drei Blöcken („Pokémon-Daten“, 44 Bytes; „Lernset“; „Entwicklungen“, 44 Bytes); nach Fokus auf „Angriff“ im Reiter „Werte“ ist Byte 1 markiert („Byte 1: Angriff, 0x4B = 75“).
- [ ] 3. Gegenstände, Trank: Spalte „Rohdaten“ mit „files/itemtool/itemdata/pl_item_data.narc, Eintrag 17, 34 Bytes“; nach Fokus auf „Preis“ sind die Bytes 0 und 1 markiert („Byte 0 bis 1: Preis, 0x012C = 300“).

### (A6) Item-Symbole in der Liste

- [ ] 1. Gegenstände: die sichtbaren Zeilen zeigen Symbole aus dem ROM: `document.querySelectorAll('[role="listbox"][aria-label="Gegenstände"] [data-sprite="picture"]').length` ist größer als 0 (Platzhalter tragen `data-sprite="placeholder"`); Symbole laden nur für Zeilen im Blick, nach dem Scrollen kommen weitere dazu.
- [ ] 2. `call.sh get_item_icon '{"item":17}'` meldet eine Zahl von Bytes größer als 0 (PNG).
- [ ] 3. Blick des Nutzers: die Symbole sehen aus wie im Spiel (Farben, nicht verschoben). Ebenso die Pokémon-Symbole der Pokédex-Liste.

### (A7) Renderzeit der Listen

- [ ] 1. Pokédex: die Liste ist gefenstert: in der Liste `[role="listbox"][aria-label="Pokédex"]` stehen deutlich weniger als 508 Zeilen `[data-species]` im DOM, `aria-setsize` der Zeilen ist 508; Pfeiltasten, Pos1 und Ende erreichen jede Zeile.
- [ ] 2. Zeit messen (`performance.now()` im Browser-Fenster, zum Beispiel vom Leeren der Suche „397“ mit Escape bis die Zeilen im DOM stehen): unter 50 ms. Zum Vergleich aus der Umsetzung (Dev-Build, verborgenes Fenster): ohne Fensterung 49 bis 111 ms, mit 22 bis 25 ms.
- [ ] 3. Dasselbe für „Attacken“ (471) und „Gegenstände“ (468 in Platin DE).

### Ergebnis

Hier wird nach dem Durchlauf je Punkt eingetragen: Datum, Projekt, Ergebnis, Auffälligkeiten. „Wer“ steht schon fest.

| Punkt | Wer | Datum | Projekt | Ergebnis | Auffälligkeiten |
|---|---|---|---|---|---|
| (A1) 1 `vanilla_data` gated | Agent | | | | |
| (A1) 2 uxie-Orakel | Agent | | | | |
| (A1) 3 Vanilla-Rebuild byte-identisch | Agent | | | | |
| (A2) Szenario `pt-daten.yaml` | Agent | | | | |
| (A3) Editoren gegen die Screens | Agent + Blick | | | | |
| (A4) ⌘Z über zwei Assets („Wie Staralili“) | Agent | | | | |
| (A5) Tüfteln: Hexblock mit markiertem Feld | Agent | | | | |
| (A6) Item-Symbole in der Liste | Agent + Blick | | | | |
| (A7) Renderzeit der Listen | Agent | | | | |
| (A1) 4 `scripts/gate.sh` | Agent | | | | |

Ergebnis von `scripts/gate.sh` (Tabelle aus der Ausgabe): steht nach dem Lauf hier.

### Bekannt vor der Abnahme

Befunde aus der Umsetzung, die nicht als Fehler zählen oder bewusst offen sind:

- Trainer und Begegnungen stehen im Umschalter, sind aber gesperrt („kommt mit Phase 3b.3b“); Trainer-Plaketten in „Fundorte“ und „Verwendet in“ sind gesperrt („Trainer öffnen kommt mit Phase 3b.3b.“). „Verwendet in“ zählt ohne Scripts.
- In Platin DE teilen sich Gegenstand 0 („Keines“) und die IDs 113 bis 134 den Datenmember 0 (23 IDs); die Hand warnt, dass Preis und Werte für alle gelten, und das Label nennt es.
- Die Pokédex-Farbe 6 heißt „Lila“; ob „Violett“ (wie in den Sticker-Texten der Spiele) besser passt, entscheidet der Nutzer.
- Die erste Liste nach dem Öffnen eines Projekts braucht in der Debug-Brücke rund eine Sekunde (die Member werden zum ersten Mal von der Platte gelesen), danach wenige Millisekunden.
- Der Wechsel des Unterbereichs zeichnet die ganze App neu (rund 50 ms im Dev-Build), unabhängig von der Liste.
- Die DS-Textbox der Beschreibungen ist doppelt groß; in einem normalen Fenster muss man in der Schau scrollen, um „Verwendet in“ zu sehen.
- Die Hand-Höhe merkt sich die App je Bereich, nicht je Unterbereich (kommt mit 3b.3b).
