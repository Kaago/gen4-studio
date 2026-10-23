# Gate mit dem Agenten

Diese Anleitung sagt einem Claude-Agenten, wie er die Gates [phase-3a-gate.md](phase-3a-gate.md) und [phase-3b1-gate.md](phase-3b1-gate.md) abarbeitet. Die Punkte selbst stehen in den Gate-Dateien; hier steht nur, **wer** sie prüft und **wie**. Entwurf: [docs/superpowers/specs/2026-10-22-agent-test-harness-design.md](superpowers/specs/2026-10-22-agent-test-harness-design.md) (§5 ist die Zuordnung).

Das Prinzip: Die echte Oberfläche läuft im Browser-Fenster der Claude-Desktop-App (statt im Tauri-Fenster), ihre Befehle gehen über eine Brücke an dieselbe Rust-Logik. Das Spiel läuft in einem Emulator ohne Fenster (`g4emu`).

Legende der Spalte „Wer“: **Agent** erledigt der Agent allein; **Agent + Blick** prüft der Agent, der Nutzer bestätigt kurz am Bild; **Hand** bleibt beim Nutzer.

## Einmalig einrichten

1. Emulator-Kern holen: `bash scripts/g4emu/setup.sh` (legt die Python-Umgebung an und **lädt den festen melonDS-Kern aus dem GitHub-Release** nach `tools/g4emu/core/`; die SHA-256-Summe ist im Skript festgelegt). Ist der Kern noch nicht da, vorher den Nutzer um sein OK für den Download bitten; ist `tools/g4emu/core/melondsds_libretro.dylib` schon da, entfällt der Schritt.
2. Spielstände prüfen: `ls ../Roms/_testkopien/_saves` zeigt `CPUD-zimmer.sav` (Platin DE), `IPKD-zimmer.sav` (HeartGold DE) und `IPKE-zimmer.sav` (HeartGold US), je mit einer `.md`-Datei. Sie liegen nicht im Repo und werden nie überschrieben.
3. apicula (Kartenbilder, 3D-Texturen): `tools/apicula/bin/apicula` muss existieren. In einem Git-Worktree fehlt `tools/` (steht in `.gitignore`): entweder `ln -s <Haupt-Checkout>/tools/apicula tools/apicula` oder `export G4_APICULA=<Pfad>`.
4. Prüfen, dass es läuft: `cd scripts/g4emu && uv run g4emu --help` und `uv run pytest -m "not integration" -q`.

## Je Lauf

1. **Frische Kopie** anlegen (nie ein Original öffnen; die Brücke lehnt alles außerhalb von `_testkopien` ab):

   ```sh
   cp -R "../Roms/Pokemon - Platin-Edition (Germany)_DSPRE_contents" ../Roms/_testkopien/pt-3a
   ```

   Für HeartGold DE und US entsprechend `"…Goldene Edition HeartGold (Germany)_DSPRE_contents"` und `"…HeartGold Version (USA)_DSPRE_contents"`. Das Ziel darf noch nicht existieren (sonst landet die Kopie als Unterordner darin). Jede Prüfreihe bekommt eine eigene Kopie (Anlegen, Patch und Bauen lassen sich nicht zurücknehmen). Unter `../Roms` wird nichts gelöscht.
2. **Screenshot-Ordner:** `mkdir -p target/agent/gate-$(date +%F)`; `g4emu` bekommt ihn als `--out` (mit Unterordner je Punkt). Die Werkzeuge des Browser-Fensters liefern Bilder nur an den Agenten, sie schreiben keine Datei: für „Agent + Blick“ beschreibt der Agent Ort und Zahlen (welcher Header, welche Zelle, was zu sehen ist) und sagt dem Nutzer, was er im Fenster ansehen soll. Bilddateien gibt es nur von `g4emu` (`shot`).
3. **App starten.** Zwei Wege:
   - Ist die Sitzung im Repo-Ordner geöffnet, der `.claude/launch.json` enthält: `preview_start` mit `name: "studio-web"`.
   - Sonst (zum Beispiel in einem Worktree: `preview_start` liest `launch.json` aus dem Ordner, in dem die Sitzung geöffnet wurde): per Bash im Hintergrund
     `bash scripts/agent/studio-web.sh > target/agent/studio-web.log 2>&1 & echo $! > target/agent/studio-web.pid`
     abwarten, bis das Log „Brücke läuft“ meldet (der erste Lauf baut `studio-bridge`, das kann Minuten dauern), dann `preview_start` mit `url: "http://127.0.0.1:1430"`.

   Ports: Vite 1430, Brücke 1431. Die App des Nutzers (1420/1421) bleibt unberührt. Das Skript bricht ab, wenn 1430 oder 1431 belegt ist.
4. **Projekt öffnen:** `http://127.0.0.1:1430/?project=<absoluter Pfad der Kopie>` (Leerzeichen als `%20`). Die Statusleiste zeigt „Browser-Modus (Brücke)“ und die Projektmeldung. Alternativ „Projekt öffnen“ in der Leiste: ein Dialog mit dem Feld „Pfad“, „OK“ und „Abbrechen“ (Escape bricht ab).
5. **App beenden:** Weg 1: `preview_stop`. Weg 2: `kill $(cat target/agent/studio-web.pid)` (TERM an das Skript beendet Brücke und Vite; kein `pkill -f`, das träfe auch die Shell). Danach `lsof -iTCP:1430 -iTCP:1431 -sTCP:LISTEN` (leer = Ports frei). Nach einem Neustart der Brücke ist kein Projekt mehr offen, die Seite muss neu geladen werden.
6. **Ergebnis:** Die Gate-Datei bekommt im Abschnitt „Ergebnis“ je Teil eine Zeile (Datum, Projekt, Ergebnis, Auffälligkeiten) und in der Spalte „Wer“ Agent oder Hand. Am Ende listet der Agent dem Nutzer alle Punkte mit „Hand“ und „Agent + Blick“, bei Bildern von `g4emu` mit Pfad, sonst mit Ort und Zahlen.

## Doppeldeck: wo was ist

Seit Phase 3c hat die App das Doppeldeck-Raster. Die älteren Abschnitte dieser Anleitung und die Gate-Dateien 3a bis 3b.2 nennen teils die Namen von vorher („Panel Welt“, „Neu anlegen“ (+ in der Liste „Orte“), „Begriffe“); die Tabelle unten sagt, wo es heute steht. Von oben nach unten und von links nach rechts:

| Fläche | Was dort ist | Zum Bedienen |
|---|---|---|
| Titelleiste (40 px) | Projektname mit Spiel-Plakette, die Zeile „Ungespeichert“, die Palette (gesperrt, Tooltip „Kommt mit 3c.2“), „Projekt öffnen“, „Speichern“ (⌘S), „Bauen“, „Thema wechseln“, „Hilfe“ | Rolle `button` mit dem Namen; „Bauen“ fragt im Browser-Modus nach dem Pfad (siehe Werkzeuge unten) |
| Atlas (88 px, Kompakt 56 px nur Symbole, der `aria-label` bleibt) | `nav` „Bereiche“: „Start“, „Welt“ (aktiv), „Geschichten“, „Pokémon“, „Gegenstände“ (diese drei gesperrt, „Kommt mit Phase 3b.3“), „Texte“, „Technik“, „Einstellungen“ (gesperrt, „Kommt mit 3c.2“), „Hilfe“ | „Hilfe“ öffnet den Dialog mit dem Glossar, das früher „Begriffe“ hieß (`workbench/terms.ts`) |
| Liste „Orte“ (272 px) | Suche „Welt durchsuchen“, Umschalter „Baum“ / „Liste“, das Menü „Neu anlegen“ (+) mit „Gebiet anlegen…“ und „Raum anlegen…“, der Knoten „Scripts“ mit Volltextsuche | ersetzt das alte Panel „Welt“ und „Neu anlegen“ (+ in der Liste „Orte“); „Gebiet erweitern“ öffnet über die „+“-Zellen im Reiter „Raster“ |
| Schau (oben) | Reiter „3D“, „Raster“, „Weltkarte“; die Szenenleiste mit den Schaltern „Gelände“ und „Ereignisse“ und die „Legende“ | die Szene nimmt Klicks nach dem aktiven Werkzeug (unten) |
| Scharnier (44 px) | Brotkrume (Ort, Auswahl), Umschalter „Bauen“ / „Tüfteln“ (Tüfteln zeigt im Inspector Rohwerte, Hex-Felder und Bezeichner), Prüflicht, Verlauf-Menü („Verlauf, N Schritte“), „Wiederholen“ (⇧⌘Z), „Hier spielen“ (baut und startet den Emulator; ⌘R nur in der App) | Ziehen am Scharnier ändert die Höhe der Hand (200 bis 560 px), Doppelklick auf den Griff klappt sie ein |
| Hand (unten) | links „Werkzeuge“, in der Mitte der Inspector mit den Reitern „Text“, „Script“, „Probleme“, rechts „An diesem Ort“ (Ereignisse und Gebäude der Karte; Klick wählt) | der Inspector zeigt das Gewählte; Text, Script und Probleme sind die Editoren aus 3b.2 |
| Statusleiste (28 px) | links die Hinweise des Werkzeugs („Klick wählt“ …), Mitte Kachel- und Ortskontext, rechts Build-Stand („Noch nicht gebaut“, „Baut …“, „Gebaut vor …“), Dichte (Komfort / Kompakt), im Browser-Modus „Browser-Modus (Brücke)“ | die Dichte schaltet um und ist im `localStorage` gemerkt |
| Meldungen | Toasts unten rechts; ein Fehler bleibt, bis man ihn schließt, ein Fortschritt ersetzt sich | im DOM Rolle `alert` (Fehler) bzw. `status` |

**Werkzeuge** (Hand, links). Der Buchstabe wählt das Werkzeug (nicht im Textfeld und nicht mit Umschalttaste oder Cmd), Esc wählt wieder „Auswählen“:

| Werkzeug | Taste | Klick in der Szene |
|---|---|---|
| Auswählen | V | wählt Marker und Gebäude, Ziehen verschiebt; malt nie |
| Gelände malen | B | setzt die Kollision der Kachel, Umschalt-Klick löscht sie |
| Figur | F | stellt eine Figur hin (Objekt-Event) |
| Tür | T | legt einen Warp an (ersetzt den Knopf „Warp hinzufügen“ der Szenenleiste) |
| Auslöser | A | legt einen Trigger an |
| Schild | S | stellt ein Hintergrund-Event auf |
| Gebäude | G | setzt ein Gebäude (ersetzt den Knopf „Neues Gebäude“ der Szenenleiste) |

Nach einem Klick mit Figur, Tür, Auslöser, Schild oder Gebäude steht wieder „Auswählen“. Gemalt wird nur mit „Gelände malen“; ein Klick mit „Auswählen“ ändert nie die Karte. Im Inspector bleiben „Warp hinzufügen“ im Abschnitt „Eingänge / Ausgänge“ des Header-Editors (legt den Warp auf der zuletzt berührten Kachel an) und „Neues Gebäude“ im Gebäude-Editor.

**Galerie.** `http://127.0.0.1:1430/?gallery` zeigt alle Bausteine ohne Projekt, mit Schaltern für Thema und Dichte; für die Screenshots des Gates (Hell, Dunkel, Kompakt) siehe [dev-setup.md](dev-setup.md), Abschnitt „Galerie“.

**Layout.** Jedes Deck merkt sein Layout im `localStorage` (`g4.layout.v7.welt.schau` und `g4.layout.v7.welt.hand`, dazu `g4.shell.v1` für den Rahmen); ein unbrauchbares Layout setzt nur dieses Deck auf den Standard zurück. Zum Zurücksetzen im Browser-Modus `localStorage.clear()` ausführen und die Seite neu laden.

## Werkzeuge

**In-Seite-Dialoge.** Im Browser-Modus gibt es keine nativen Dialoge: Fragen erscheinen auf der Seite mit „Ja“ und „Nein“, Pfadabfragen mit dem Feld „Pfad“, „OK“ und „Abbrechen“. „Bauen“ fragt „Speichern unter“, vorbelegt mit `<Projekt>/out.nds`; „OK“ baut, Meldung „ROM gebaut“. Escape heißt „Nein“ bzw. „Abbrechen“. Fehlt die Brücke, steht „Brücke nicht erreichbar“ in der Statusleiste.

**`scripts/agent/call.sh <befehl> ['<json>']`** ruft einen Befehl der laufenden Brücke auf (dieselbe Sitzung wie die Seite) und druckt die Antwort; bei Fehlern Exit 1 und der Fehler auf stderr. Argumente heißen wie in `apps/studio/src/api.ts` (camelCase). Beispiele:

```sh
scripts/agent/call.sh dirty                                  # ungespeicherte Teile, [] heißt sauber
scripts/agent/call.sh get_header '{"id":415}'                # Felder inklusive event_file_id
scripts/agent/call.sh get_events '{"eventFileId":394}'       # Warps mit index, x, z, dest_header_id, dest_warp_id
scripts/agent/call.sh update_event '{"eventFile":394,"kind":"warp","index":0,"patch":{"dest_header_id":412,"dest_warp_id":0}}'
scripts/agent/call.sh header_patch_status                    # supported, applied, repairable
scripts/agent/call.sh save
scripts/agent/call.sh build_rom '{"out":"<Kopie>/out.nds"}'
```

Änderungen per `call.sh` sieht die Oberfläche erst nach einem Neuladen der Seite (`?project=` öffnet das Projekt neu).

**Emulator** (`cd scripts/g4emu`; alle Pfade zu `../Roms` sind von dort `../../../Roms`):

```sh
uv run g4emu run --rom <rom.nds> --save ../../../Roms/_testkopien/_saves/CPUD-zimmer.sav \
  --out ../../target/agent/gate-$(date +%F)/<punkt> continue expect-map 415 shot start
```

Schritte: `continue`, `wait N`, `press TASTE[:N]`, `tap X Y`, `walk RICHTUNG N`, `walk-to X Y`, `expect-map ID [N]`, `where`, `shot NAME`, `state NAME`, `write-save NAME`, `dump-ram NAME`. Rückgabe: 0 ok, 2 `walk` oder `expect-map` gescheitert (die Zeile nennt Position bzw. zuletzt gelesene ID), 1 sonstiger Fehler (ROM oder Kern fehlt). Jeder Schritt druckt eine JSON-Zeile; im Ausgabeordner liegen `last.png` und `last.state` (mit `--state <out>/last.state` geht es dort weiter, um Schritt für Schritt zu erkunden) und `core.log`. Der Spielstand wird immer zuerst kopiert; `--save` ist nach dem Lauf unverändert. Spielcodes in `scripts/g4emu/g4emu/games.yaml`: CPUD, IPKD, IPKE.

- Platin DE: Start im Spielerzimmer (Header 415) bei x 7, y 4. Die Treppe ist der Warp 0 von Eventdatei 394 bei x 8, z 4: `walk-to 8 4`, `walk right 1` führt nach 414 (Ankunft x 10, y 3). Beim Ankommen läuft die Überblendung noch: `expect-map` kehrt früh zurück, danach `wait 150`, bevor man ein Bild macht oder weitergeht (die Figur macht nach der Ankunft von selbst einen Schritt).
- HeartGold (IPKD/IPKE): Start im Zimmer oben (Header 64, x 3, y 4). `walk left 1` bringt auf die Treppenkachel (2, 4), `walk down 1` löst den Warp nach 63 aus (Ankunft x 3, y 3). Header-IDs im `g4k` sind dezimal.
- Ein Warp feuert nur, wenn die Figur auf der Warp-Kachel steht **und** einen Schritt in Richtung Ausgang macht: nach `walk-to` kommt immer noch ein `walk RICHTUNG 1`.
- Position unklar: `uv run g4emu run … continue where` oder `walk-to` bis zum Rand; keine Wegsuche, ein blockierter Weg meldet „blockiert bei x, y“.

**Szenario** (Projekt ändern, bauen, spielen in einem Lauf), aus `scripts/g4emu` heraus; die Kopf-Kommentare der Dateien nennen die Platzhalter und ein Beispiel für `--set`:

```sh
uv run g4emu scenario ../agent/scenarios/pt-treppe-warp.yaml \
  --project ../../../Roms/_testkopien/<kopie> --out ../../target/agent/gate-$(date +%F)/szenario \
  --set WARP=0 --set ZIEL=412 --set TREPPE_X=8 --set TREPPE_Y=4 --set TUER_X=6 --set TUER_Y=10 --set ZURUECK=411
```

- `pt-treppe-warp.yaml`: Treppe des Spielerzimmers auf ein anderes Haus umbiegen, hin und durch die Tür wieder hinaus. Erwartete Header-Folge 415, 412, 411.
- `pt-neuer-header.yaml`: Patch anwenden, Gebiet mit **neuem** Header anlegen, die Treppe dorthin umbiegen, hin und zurück (`NEUER_HEADER` ist die Zahl der Header vorher, 593 bei Platin DE; `NEUE_EVENTDATEI` 534).
- `pt-raum-assistent.yaml`: Raum-Assistent als ein Aufruf (neuer Header als Kopie des Spielerzimmers, Eventdatei „Kopie von 0394“, Tür außen auf einer freien Kachel von Zweiblattdorf, Tür-Kacheln an), hinein, über die kopierte Treppe hinaus nach 0411 und durch die neue Tür wieder hinein. Erwartete Header-Folge 415, 593, 411, 593 (`--set NEUER_HEADER=593 --set TUER_X=105 --set TUER_Y=876`).
- Die Kopie muss direkt unter `_testkopien` liegen. Das Szenario baut `out.nds` in die Kopie und braucht dafür `cargo`; es arbeitet auf dem Spielstand aus `games.yaml`.

## Hinweise aus dem Handdurchlauf

- Warps feuern nur auf Treppen- und Warp-Kacheln und auf Türen (Verhalten 0x69) mit Kollision 0x80. Einen Warp auf eine „freie“ Kachel zu setzen verbindet nichts; im Spiel passiert dort nichts. Das Anlegen eines Warps (Werkzeug „Tür“ oder „Warp hinzufügen“) fragt deshalb, ob die Kachel zur Tür werden soll, und der Raum-Assistent macht beide Tür-Kacheln zu Türen, wenn nötig.
- Ein neuer Header mit Eventdatei „neu, leer“ braucht einen Ankunfts-Warp (Index 0, `add_event`/Werkzeug „Tür“). Ohne ihn zeigt der Ziel-Warp 0 ins Leere, und das Spiel zeigt nach dem Übergang schwarz.
- Der Gebiets-Assistent nimmt als Vorlage den Header, für den er geöffnet wurde, sonst den der ersten gewählten Zelle, die keine Füllfläche (Header 0) ist. Beim Prüfen von Schritt Header steht die Vorlage in der Zeile „Kopie von Header …“.
- Der von DSPRE übernommene Patch „Dynamische Header“ war für Platin DE/FR/IT/ES fehlerhaft; „Patch reparieren“ und die Korrektur sind in [formats/dynamic_headers.md](formats/dynamic_headers.md) beschrieben. Die Anleitung prüft diese Befunde nicht neu.
- `Cmd+S` im Browser-Fenster kann die Seite statt der App meinen. Tritt das auf, „Speichern“ in der Leiste klicken oder `call.sh save` benutzen.
- Hover und Maus: Das 3D-Fenster lässt sich mit `hover` bedienen. Klappt Ziehen mit der Maus im Browser-Fenster nicht, prüft der Agent die Wirkung über `update_event` per `call.sh` und führt den Punkt als „Agent + Blick“.
- Szenario-Läufe und Gate-Läufe hinterlassen Kopien unter `_testkopien` (zum Beispiel `pt-agent-szenario2`); der Nutzer räumt sie auf.

## Phase 3a ([phase-3a-gate.md](phase-3a-gate.md))

Kopie `pt-3a` (Platin DE), für (D) 4 zusätzlich `hg-3a` (HeartGold US), für die Fehlerfälle aus (F) eine eigene `pt-3a-f`. Die Gate-Datei schreibt die gebündelte App vor; im Browser-Modus ersetzt `studio-web` sie, die Abweichung kommt in „Auffälligkeiten“.

| Punkt | Wer | Wie |
|---|---|---|
| (A) 1–4 Gebiet anlegen (Route 230) | Agent | Browser: Header 0342 wählen, „Neu anlegen“ (+ in der Liste „Orte“) → „Gebiet anlegen…“, Zellen abwählen bis zwei übrig sind, Name „Route 230“, Schritt Header: 0001 („NOTHING“), „Anlegen“, „Im Studio zeigen“ |
| (A) 5–10 Raum anlegen (Spielerzimmer als neuer Header 0593) | Agent | Browser: Maus (`hover`) in der 3D-Ansicht von 0411 auf die Kachel vor dem Rivalenhaus, „Neu anlegen“ (+ in der Liste „Orte“) → „Raum anlegen…“, Vorlage Header 0415, „Neuer Header“ (vorher „Patch anwenden“), Eventdatei „Kopie von …“ ausdrücklich wählen, Eingang Außen-Header 0411, „Tür-Kacheln“ an lassen, „Anlegen“. Nicht Header 0002 „UG“ (Sonder-Header, bleibt im Spiel schwarz). Dann im Inspector bei 0411 „Eingänge / Ausgänge“ ansehen (Ziel „0593“, „Zum Ziel“); Gegenprobe `call.sh get_events` der neuen Eventdatei: genau ein Warp auf der Rückweg-Kachel, Ziel 0411 |
| (A) Stoppuhr und „Zahlen getippt“ | Hand | Die Zeit misst der Mensch bei eigener Bedienung. Der Agent trägt nur seine getippten Zahlen ein und sagt, dass seine Zeit nichts aussagt |
| (A) 11 Warps von Hand verbinden | Agent | Browser, Schritte in Rezept „Warps verbinden“ unten |
| (A) 12 Gang im Spiel | Agent | „Bauen“, dann `g4emu`, Schritte in Rezept „Gang im Spiel“ unten; Rückgabe 0 und die Header-Folge sind das Ergebnis, `shot` an jeder Station |
| (B) 1–4 Explorer | Agent | Browser: Suche „Zweiblatt“ in der Liste „Orte“, Umschalter „Baum / Liste“, Hilfe-Dialog (Atlas „Hilfe“, früher „Begriffe“), „Ohne Ortsnamen“; Texte im DOM lesen (`find`, `get_page_text`) |
| (C) 1–4 Raster | Agent | Browser: Tab „Raster“, Header 0411, 0003, Route 230; die „+“-Zelle öffnet den Dialog „Karte bei x …, y … anlegen“. Neue Karten, Raster und Eventdateien sind sofort auf der Platte; `call.sh member_counts` zeigt die Zahlen |
| (D) 1–3 Weltkarte | Agent | Browser: Tab „Weltkarte“, Zellenklick, Zoomen mit dem Mausrad, Linien bei Zweiblattdorf, nach (A) der neue Raum |
| (D) 4 HeartGold US | Agent | frische Kopie `hg-3a`, `?project=…`; Baum und Weltkarte laden, New Bark Town = Header 0060 |
| (E) 1–2 Automatik | Agent | `scripts/gate.sh` (rund 25 Minuten, im Hintergrund mit `> target/agent/gate.log` und Tabelle am Ende übernehmen) |
| (F) Kartenbilder: Ausrichtung | Agent + Blick | Tab „Raster“ und 3D-Ansicht von oben bei Zweiblattdorf nacheinander ansehen, Norden oben, nicht gespiegelt; der Nutzer bestätigt |
| (F) Kartenbilder ohne apicula | Agent | laufende App beenden (Ports sind belegt), dann `G4_APICULA=/nirgends bash scripts/agent/studio-web.sh > target/agent/studio-web.log 2>&1 & echo $! > target/agent/studio-web.pid`, Seite öffnen: Platzhalter und Tooltip lesen; danach beenden und normal neu starten |
| (F) Layout im schmalen Panel | Agent | Browser-Fenster so verkleinern (`resize_window`) oder den Teiler ziehen, dass die Liste „Orte“ schmal ist (Atlas auf Kompakt); Listenansicht, Hilfe-Dialog (Atlas „Hilfe“, früher „Begriffe“) und „Neu anlegen“ (+ in der Liste „Orte“) ansehen, Escape und Klick daneben |
| (F) Tastatur während eines Assistenten | Agent | Assistent öffnen, Escape, Backspace, Cmd+Z, Cmd+S drücken; danach `call.sh dirty` (leer = nichts verändert) |
| (F) Raum-Assistent, Fehlerfälle | Agent | Browser, Schritte in Rezept „Raum-Assistent und Fehlerfälle“ unten (Raster-Wechsel, „Raum hier anbinden…“, Kachel x 999, gesperrte Eventdatei, später Fehler, Cmd+Z) |
| (F) Layout nach Neustart | Agent | Panel verschieben, Seite neu laden (das Layout liegt im `localStorage`) |
| Gebündelte App statt `tauri dev` | Hand | kurzer Durchgang im nativen Fenster am Ende (`pnpm bundle`, siehe [release.md](release.md)) |

## Phase 3b.1 ([phase-3b1-gate.md](phase-3b1-gate.md))

Kopien `pt-3b1` (Platin DE, **nicht** vorher mit DSPRE gepatcht) und `hg-3b1` (HeartGold DE). Für (A) 18 eine eigene dritte Kopie `pt-3b1-rep`: `pt-3b1` bleibt unversehrt, sie ist das Gegenbeispiel „ein intaktes Projekt zeigt die Meldung nicht“. In `pt-3b1` und in Originalen werden nie Bytes von Hand geändert.

| Punkt | Wer | Wie |
|---|---|---|
| (A) 1 Ausgangslage | Agent | `ls -l <Kopie>/files/fielddata/maptable/mapname.bin` (Größe notieren), `ls <Kopie>/arm9/arm9.bin.backup` muss fehlen; Zahl der Header: `call.sh list_headers` oder letzte ID im Explorer plus eins |
| (A) 2–3 Assistent bis Patch | Agent | Browser: Header 0342, „Neu anlegen“ (+ in der Liste „Orte“) → „Gebiet anlegen…“, zwei Zellen übrig, Name „Route 231“, Schritt Header: „Neuer Header“ ist gesperrt, Kasten „Patch „Dynamische Header“ nötig“ lesen, vorher speichern, „Patch anwenden“ |
| (A) 4 Platte nach dem Patch | Agent | `cmp <Kopie>/arm9/arm9.bin <Kopie>/arm9/arm9.bin.backup` meldet Unterschiede; `ls <Kopie>/unpacked/dynamicHeaders \| wc -l` gleich Zahl der Header; `ls -l <Kopie>/arm9/arm9.bin*` |
| (A) 5–6 Neuer Header | Agent | Browser: „Neuer Header“, „Interner Name“ auf `R231`, Zusammenfassung lesen, „Anlegen“, „Im Studio zeigen“ |
| (A) 7 Platte nach dem Anlegen | Agent | `ls -l <Kopie>/unpacked/dynamicHeaders/NNNN` (NNNN vierstellig, 24 Byte; in JSON ohne führende Nullen, zum Beispiel `{"id":593}`); `mapname.bin` genau 16 Byte länger; `tail -c 16 <Kopie>/files/fielddata/maptable/mapname.bin \| xxd` beginnt mit `R231` |
| (A) 8 Variante „behalten“ | Agent | Browser: zweites Gebiet „Route 232“, Eventdatei „behalten“; Gegenprobe `call.sh get_header '{"id":NNNN}'` (NNNN ohne führende Nullen; gleiche `event_file_id` wie 0342) |
| (A) 9–10 Warp und Gang im Spiel | Agent | Warps in der Oberfläche setzen (siehe 3a (A) 11) und mit `g4emu` laufen, **oder** das Szenario `pt-neuer-header.yaml` auf einer weiteren frischen Kopie (Patch, neuer Header, Treppe, hin und zurück ohne Klick) |
| (A) 11 NARC im ROM | Agent | `target/debug/g4k extract <Kopie>/out.nds target/agent/gate-<datum>/rom-extrakt` und prüfen, dass `…/files/debug/cb_edit/d_test.narc` existiert (`g4k` steht nach `cargo build -p g4-kernel` bereit; der Zielordner darf nicht unter `../Roms` liegen) |
| (A) 12–15 DSPRE-Gegenprobe | Hand | DSPRE läuft nur unter Windows |
| (A) 16 HeartGold DE | Agent | wie (A) 2–10 auf `hg-3b1`, Interner Name `HG_TEST`, Spielstand IPKD (Zimmer Header 64, Treppe nach 63); nach dem Bauen `target/debug/g4k extract <Kopie>/out.nds target/agent/gate-<datum>/rom-hg` und `ls …/rom-hg/files/a/0/5/0` (im ROM, nicht im Projektordner) |
| (A) 17 Diamant/Perl | Hand | entfällt ohne Projekt |
| (A) 18 Patch reparieren | Agent | Auf der **eigenen** frischen Kopie `pt-3b1-rep` (nie `pt-3b1`, nie ein Original), Schritte in Rezept „Patch reparieren“ unten. Dass DSPRE selbst so patcht, gehört zur DSPRE-Gegenprobe (Hand) |
| (B) 1–5 Höhen | Agent + Blick | Browser: Tab „Raster“, „Höhen“ einschalten, Zelle anklicken, Inspector „Höhe“ = 2; Hover und Ziehproben mit `hover` und `left_click_drag`, Messwerte über `get_matrix`; wie die erhöhte Zelle aussieht bestätigt der Nutzer am Bild |
| (C) 1–7 Sprites | Agent + Blick | Agent prüft: Sprite statt gelbem Quadrat, Wechsel des Feldes „Richtung“ (0 bis 3) ohne Neuladen, Platzhalter bei „Grafik“ 60000 (danach Cmd+Z). Aufrecht, nicht gespiegelt und Blickrichtung bestätigt der Nutzer am Bild |
| (D) 1–3 Kachelnamen HGSS | Agent | Browser: Hover auf Bäumen (`HEADBUTT`), Gras, Wasser, Hüpfkante; Platin: derselbe Wert 0x06 ohne `HEADBUTT`; Namen mit `call.sh tile_behaviors` gegenprüfen |
| (E) 1–2 `gate.sh` | Agent | `scripts/gate.sh`, Tabelle in die Ergebnisse übernehmen; keine Zeile „skipped“ außer den dokumentierten |
| (F) 1–3 apicula-Zeitlimit | Agent | `G4_APICULA_TIMEOUT_SECS=1 bash scripts/agent/studio-web.sh …` mit frischer Kopie ohne `.g4/models`; `ls <Kopie>/.g4/models` (keine `.fail`, `last-tool` mit 16 Hexzeichen) |

## Phase 3b.2 im Browser-Modus ([phase-3b2-gate.md](phase-3b2-gate.md))

Die Brücke kennt alle Befehle von Scripts und Texten: `get_text_archive`, `edit_text_archive`, `create_text_archive`, `search_text`, `validate_text`, `scripts_status`, `adopt_scripts`, `list_script_files`, `read_script`, `save_script`, `save_level_script`, `new_script_file`, `add_script`, `resolve_script`, `search_scripts` (per `call.sh` wie die übrigen, Argumente wie in `api.ts`). Sie schreiben nur in das geöffnete Projekt, und das liegt unter `_testkopien`; ein Original oder ein Ordner außerhalb wird schon beim Öffnen abgelehnt. „Scripts übernehmen“ (`adopt_scripts`) läuft ohne Fortschrittsbalken: Es gibt über HTTP keinen Kanal, die Seite meldet erst am Ende einmal „fertig“ und der Aufruf kehrt nach einigen Sekunden mit dem Bericht zurück. Die Sprachdienste (`lsp_start`, `lsp_send`, `lsp_stop`) sind im Browser aus: `lsp_start` meldet „rotom-lsp fehlt — Completion und Diagnosen aus“ (Code `tool_missing`), der Editor läuft wie ohne rotom-lsp, also ohne Completion und Diagnosen. Punkte des 3b.2-Gates, die Completion oder Diagnosen aus rotom-lsp verlangen, bleiben beim Nutzer im gebündelten Fenster (Wer: Hand).

## Rezepte

**Warps verbinden (3a (A) 11).** Warps feuern nur auf Tür-, Treppen- und Warp-Kacheln.
1. In 0411: `call.sh get_events '{"eventFileId":390}'` zeigt die Türen (Warp 0 bei 105, 875 führt in das Rivalenhaus). Werkzeug „Tür“ (T) wählen und auf die Kachel einer Tür klicken (früher der Knopf „Warp hinzufügen“; der Knopf im Abschnitt „Eingänge / Ausgänge“ des Header-Editors geht weiterhin und nimmt die zuletzt berührte Kachel), im Inspector Ziel Header 1, Ziel-Warp 0. Den Index des neuen Warps in „Eingänge / Ausgänge“ ablesen.
2. In 0001 (Route 230): Werkzeug „Tür“ (T), Klick auf die Zielkachel, Ziel Header 411 und der Index aus Schritt 1. Hat die Kachel kein Warp-Verhalten, passt `call.sh tile_behaviors` den Wert an und `set_permission` setzt ihn; gelingt das nicht, den Befund notieren.
3. Prüfen: `call.sh list_warps` nennt beide Warps.

**Gebäude setzen.** Werkzeug „Gebäude“ (G) wählen und auf eine Kachel der 3D-Ansicht klicken (früher der Knopf „Neues Gebäude“ der Szenenleiste); danach steht wieder „Auswählen“, und der Inspector zeigt das neue Gebäude. Gegenprobe: `call.sh dirty` nennt die Karte.

**Gang im Spiel (3a (A) 12).** Koordinaten von Warps (`x`, `z`) sind draußen gleich `x`, `y` von `where`; die Kachel vor einer Tür liegt eine südlich (Tür bei 105, 875: vorher 105, 876).
1. „Bauen“ (OK im Pfad-Dialog), Ergebnis `<Kopie>/out.nds`.
2. Zimmer, Haus, draußen, wieder hinein (aus `scripts/g4emu`):
   ```sh
   O=../../target/agent/gate-$(date +%F)/a12; R=../../../Roms/_testkopien
   uv run g4emu run --rom $R/<Kopie>/out.nds --save $R/_saves/CPUD-zimmer.sav --out $O \
     continue expect-map 415 walk-to 8 4 walk right 1 expect-map 414 wait 150 shot wohnzimmer
   ```
   Weiter mit `--state $O/last.state` (dasselbe `--rom`, `--out $O`), zum Beispiel `walk-to 6 10 walk down 1 expect-map 411 wait 100 shot draussen walk-to 105 876 walk up 1 expect-map 412` (Tür des Rivalenhauses; Tür des eigenen Hauses nach 414 entsprechend aus `get_events`).
3. Route 230: `walk-to` auf die Warp-Kachel aus „Warps verbinden“ plus ein Schritt in Richtung Ausgang, `expect-map 1`, `shot`; zurück ebenso nach `expect-map 411`.

**Patch reparieren (3b.1 (A) 18)**, auf `pt-3b1-rep`:
1. App öffnen, „Neu anlegen“ (+ in der Liste „Orte“) → „Gebiet anlegen…“, bis Schritt Header, „Patch anwenden“, App beenden (Seite schließen, Brücke stoppen).
2. Fehler nachstellen: `printf '\xCC\xF7\x74\xFC' | dd of=<pt-3b1-rep>/arm9/arm9.bin bs=1 seek=$((0x3A0D6)) conv=notrunc`
3. App starten, Kopie öffnen: Meldung „Patch „Dynamische Header“ fehlerhaft …“ und Knopf „Patch reparieren“ (`call.sh header_patch_status` zeigt `repairable: true`). Knopf drücken: Meldung „… repariert …“.
4. `xxd -g1 -s $((0x3A0D6)) -l 4 <pt-3b1-rep>/arm9/arm9.bin` zeigt `de f7 ad f8`. Bauen, `g4emu run … continue wait 600 shot weiter`: das Spiel läuft nach dem Intro weiter.
5. Gegenprobe: `pt-3b1` öffnen, die Meldung erscheint nicht.

**Raum-Assistent und Fehlerfälle (3a (F))**, auf einer eigenen Kopie `pt-3a-f`:
- *Falsches Raster:* in der Szene Header 0342 (Raster 0000) zeigen, „Neu anlegen“ (+ in der Liste „Orte“) → „Raum anlegen…“, Vorlage 0415, im Schritt Header einen freien Header, im Schritt Eingang als Außen-Header 0504 (Großmoor, Raster 0240) wählen (0002 „UG“ wird nicht als Außen-Header angeboten: Raster 0002 hat keinen Header-Abschnitt). Erwartet: der Schritt fragt nach der Kachel der Tür (leere Felder, kein Hinweis „Kachel zuletzt in der Szene gewählt“). Zum Vergleich Außen-Header 0411 (Raster 0000): die Kachel ist vorbelegt.
- *„Raum hier anbinden…“:* Header 0411 öffnen, im Inspector „Raum hier anbinden…“: der Assistent hat 0411 als Außen-Header vorgewählt.
- *Kachel außerhalb:* im Schritt Eingang x 999 eintippen: der Assistent lehnt ab, bevor etwas angelegt wird (`call.sh member_counts` vorher und nachher gleich).
- *Gesperrte Eventdatei:* App beenden, `printf '\0' >> <pt-3a-f>/unpacked/eventFiles/0390` (hängt ein Byte an die Datei von 0411 an, sie lässt sich dann nicht mehr verlustfrei schreiben), App neu starten. Raum-Assistent mit Außen-Header 0411: der Schritt Eingang nennt die Sperre und bietet „später verbinden“; `call.sh get_events '{"eventFileId":390}'` zeigt `locked`.
- *Später Fehler:* nur mit gepatchter Kopie `pt-3a-f` (zuerst wie in 3b.1 (A) 3 gepatcht). `chmod 555 <pt-3a-f>/unpacked/maps`, „Gebiet anlegen…“ mit „Neuer Header“, „Anlegen“: das Ergebnis meldet den Schritt („map copy failed …“) und das schon Angelegte („header NNNN was created“). Danach `chmod 755 <pt-3a-f>/unpacked/maps`. (Ohne neuen Header ist nichts angelegt und der Fehler lautet „nothing was created“.)
- *Cmd+Z:* vor dem Assistenten `call.sh get_header '{"id":N}'` und `call.sh member_counts` notieren, Assistent mit Header-Umstellung ausführen, `call.sh dirty` zeigt die Änderungen. Cmd+Z so oft drücken, bis `dirty` leer ist (oder `call.sh undo`): `get_header` gleicht wieder dem Vorher-Wert, `member_counts` bleibt erhöht (Karten, Raster und Eventdateien bleiben liegen, wie angesagt).

## Was beim Nutzer bleibt

- **DSPRE-Gegenprobe** (3b.1 (A) 12–15) und **Diamant/Perl** (17): DSPRE läuft nur unter Windows.
- **Stoppuhr und „Zahlen getippt“** (3a (A)): misst die Bedienung durch den Menschen.
- **Natives Fenster:** ein kurzer Durchgang mit der gebündelten App nach `pnpm bundle`.
- **„Agent + Blick“:** 3a (F) Ausrichtung der Kartenbilder; 3b.1 (B) Höhen und (C) Sprites (aufrecht, nicht gespiegelt, Blickrichtung). Der Agent nennt dafür Ort und Zahlen, damit der Blick kurz bleibt.
