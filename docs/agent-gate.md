# Gate mit dem Agenten

Diese Anleitung sagt einem Claude-Agenten, wie er die Gates [phase-3a-gate.md](phase-3a-gate.md) und [phase-3b1-gate.md](phase-3b1-gate.md) abarbeitet. Die Punkte selbst stehen in den Gate-Dateien; hier steht nur, **wer** sie prüft und **wie**. Entwurf: [docs/superpowers/specs/2026-10-22-agent-test-harness-design.md](superpowers/specs/2026-10-22-agent-test-harness-design.md) (§5 ist die Zuordnung).

Das Prinzip: Die echte Oberfläche läuft im Browser-Fenster der Claude-Desktop-App (statt im Tauri-Fenster), ihre Befehle gehen über eine Brücke an dieselbe Rust-Logik. Das Spiel läuft in einem Emulator ohne Fenster (`g4emu`).

Legende der Spalte „Wer“: **Agent** erledigt der Agent allein; **Agent + Blick** prüft der Agent, der Nutzer bestätigt kurz am Bild; **Hand** bleibt beim Nutzer.

## Einmalig einrichten

1. Emulator-Kern holen: `bash scripts/g4emu/setup.sh` (legt die Python-Umgebung an und lädt den festen melonDS-Kern nach `tools/g4emu/core/`; die SHA-256-Summe ist im Skript festgelegt).
2. Spielstände prüfen: `ls ../Roms/_testkopien/_saves` zeigt `CPUD-zimmer.sav` (Platin DE), `IPKD-zimmer.sav` (HeartGold DE) und `IPKE-zimmer.sav` (HeartGold US), je mit einer `.md`-Datei. Sie liegen nicht im Repo und werden nie überschrieben.
3. apicula (Kartenbilder, 3D-Texturen): `tools/apicula/bin/apicula` muss existieren. In einem Git-Worktree fehlt `tools/` (steht in `.gitignore`): entweder `ln -s <Haupt-Checkout>/tools/apicula tools/apicula` oder `export G4_APICULA=<Pfad>`.
4. Prüfen, dass es läuft: `cd scripts/g4emu && uv run g4emu --help` und `uv run pytest -m "not integration" -q`.

## Je Lauf

1. **Frische Kopie** anlegen (nie ein Original öffnen; die Brücke lehnt alles außerhalb von `_testkopien` ab):

   ```sh
   cp -R "../Roms/Pokemon - Platin-Edition (Germany)_DSPRE_contents" ../Roms/_testkopien/pt-3a
   ```

   Für HeartGold DE und US entsprechend `"…Goldene Edition HeartGold (Germany)_DSPRE_contents"` und `"…HeartGold Version (USA)_DSPRE_contents"`. Jede Prüfreihe bekommt eine eigene Kopie (Anlegen, Patch und Bauen lassen sich nicht zurücknehmen). Unter `../Roms` wird nichts gelöscht.
2. **Screenshot-Ordner:** `mkdir -p target/agent/gate-$(date +%F)`; `g4emu` bekommt ihn als `--out` (mit Unterordner je Punkt). Die Werkzeuge des Browser-Fensters liefern Bilder nur an den Agenten, sie schreiben keine Datei: für „Agent + Blick“ beschreibt der Agent das Bild genau (was, wo, welche Zahlen) und nennt dem Nutzer, was er im Fenster ansehen soll; Dateien entstehen nur von `g4emu`.
3. **App starten.** Zwei Wege:
   - Ist die Sitzung im Repo-Ordner geöffnet, der `.claude/launch.json` enthält: `preview_start` mit `name: "studio-web"`.
   - Sonst (zum Beispiel in einem Worktree: `preview_start` liest `launch.json` aus dem Ordner, in dem die Sitzung geöffnet wurde): per Bash im Hintergrund
     `bash scripts/agent/studio-web.sh > target/agent/studio-web.log 2>&1 & echo $! > target/agent/studio-web.pid`
     abwarten, bis das Log „Brücke läuft“ meldet (der erste Lauf baut `studio-bridge`, das kann Minuten dauern), dann `preview_start` mit `url: "http://127.0.0.1:1430"`.

   Ports: Vite 1430, Brücke 1431. Die App des Nutzers (1420/1421) bleibt unberührt. Das Skript bricht ab, wenn 1430 oder 1431 belegt ist.
4. **Projekt öffnen:** `http://127.0.0.1:1430/?project=<absoluter Pfad der Kopie>` (Leerzeichen als `%20`). Die Statusleiste zeigt „Browser-Modus (Brücke)“ und die Projektmeldung. Alternativ „Projekt öffnen“ in der Leiste: ein Dialog mit dem Feld „Pfad“, „OK“ und „Abbrechen“ (Escape bricht ab).
5. **App beenden:** `kill $(cat target/agent/studio-web.pid)` (TERM an das Skript beendet Brücke und Vite; kein `pkill -f`, das träfe auch die Shell). Nach einem Neustart der Brücke ist kein Projekt mehr offen, die Seite muss neu geladen werden.
6. **Ergebnis:** Die Gate-Datei bekommt im Abschnitt „Ergebnis“ je Teil eine Zeile (Datum, Projekt, Ergebnis, Auffälligkeiten) und in der Spalte „Wer“ Agent oder Hand. Am Ende listet der Agent dem Nutzer alle Punkte mit „Hand“ und „Agent + Blick“ samt Pfaden der Bilder.

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
- Die Kopie muss direkt unter `_testkopien` liegen. Das Szenario baut `out.nds` in die Kopie und braucht dafür `cargo`; es arbeitet auf dem Spielstand aus `games.yaml`.

## Hinweise aus dem Handdurchlauf

- Warps feuern nur auf Tür-, Treppen- und Warp-Kacheln. Einen Warp auf eine „freie“ Kachel zu setzen verbindet nichts; im Spiel passiert dort nichts.
- Ein neuer Header mit Eventdatei „neu, leer“ braucht einen Ankunfts-Warp (Index 0, `add_event`/„Warp hinzufügen“). Ohne ihn zeigt der Ziel-Warp 0 ins Leere, und das Spiel zeigt nach dem Übergang schwarz.
- Der Gebiets-Assistent nimmt Header 0 als Vorlage, wenn die erste gewählte Zelle eine Füllfläche ist. Beim Prüfen von Schritt Header darauf achten (Vorlage „Kopie von Header …“ in der Zeile).
- Der von DSPRE übernommene Patch „Dynamische Header“ war für Platin DE/FR/IT/ES fehlerhaft; „Patch reparieren“ und die Korrektur sind in [formats/dynamic_headers.md](formats/dynamic_headers.md) beschrieben. Die Anleitung prüft diese Befunde nicht neu.
- `Cmd+S` im Browser-Fenster kann die Seite statt der App meinen. Tritt das auf, „Speichern“ in der Leiste klicken oder `call.sh save` benutzen.
- Hover und Maus: Das 3D-Fenster lässt sich mit `hover` bedienen. Klappt Ziehen mit der Maus im Browser-Fenster nicht, prüft der Agent die Wirkung über `update_event` per `call.sh` und führt den Punkt als „Agent + Blick“.
- Szenario-Läufe und Gate-Läufe hinterlassen Kopien unter `_testkopien` (zum Beispiel `pt-agent-szenario2`); der Nutzer räumt sie auf.

## Phase 3a ([phase-3a-gate.md](phase-3a-gate.md))

Kopie `pt-3a` (Platin DE), für (D) 4 zusätzlich `hg-3a` (HeartGold US). Die Gate-Datei schreibt die gebündelte App vor; im Browser-Modus ersetzt `studio-web` sie, die Abweichung kommt in „Auffälligkeiten“.

| Punkt | Wer | Wie |
|---|---|---|
| (A) 1–4 Gebiet anlegen (Route 230) | Agent | Browser: Header 0342 wählen, „+ Neu“ → „Gebiet anlegen…“, Zellen abwählen bis zwei übrig sind, Name „Route 230“, Schritt Header: 0001 („NOTHING“), „Anlegen“, „Im Studio zeigen“ |
| (A) 5–10 Raum anlegen (Spielerzimmer in 0002) | Agent | Browser: Maus (`hover`) in der 3D-Ansicht von 0411 auf die Kachel vor dem Rivalenhaus, „+ Neu“ → „Raum anlegen…“, Vorlage Header 0415, Header 0002, Eventdatei „Kopie von …“ ausdrücklich wählen, Eingang Außen-Header 0411, „Anlegen“. Dann im Inspector bei 0411 „Eingänge / Ausgänge“ ansehen (Ziel „0002“, „Zum Ziel“); Gegenprobe `call.sh list_warps` |
| (A) Stoppuhr und „Zahlen getippt“ | Hand | Die Zeit misst der Mensch bei eigener Bedienung. Der Agent trägt nur seine getippten Zahlen ein und sagt, dass seine Zeit nichts aussagt |
| (A) 11 Warps von Hand verbinden | Agent | Browser: in 0411 auf der Kachel einer vorhandenen Tür (Liste mit `call.sh get_events` der Eventdatei von 0411) „Warp hinzufügen“, Ziel Header 0001 / Ziel-Warp 0; dann in 0001 Warp 0 zurück nach 0411 mit dem abgelesenen Index. In Route 230 gibt es keine Tür: die Kachel dort muss ein Warp-Verhalten haben (`call.sh tile_behaviors`, `set_permission`), sonst Befund notieren |
| (A) 12 Gang im Spiel | Agent | „Bauen“ (Pfad `<Kopie>/out.nds`), dann `g4emu run --rom <Kopie>/out.nds --save …/CPUD-zimmer.sav --out …/a12 continue expect-map 415 walk-to 8 4 walk right 1 expect-map 414 …` und weiter aus dem Haus (Tür-Kachel aus `get_events` des Headers 414, dann ein Schritt hinaus) nach 411. Draußen sind die Koordinaten der Warps (`x`, `z`) gleich den `x`, `y` von `where` (Warp 0 von 411 liegt bei 105, 875): `walk-to` auf die Kachel vor der Tür, dann `walk up 1` o. ä. in die Tür; `expect-map` auf die Ziel-ID. Route 230 (Header 1) ebenso über den Warp aus (A) 11 und zurück. Rückgabe 0 und die Header-Folge sind das Ergebnis; `shot` an jeder Station |
| (B) 1–4 Explorer | Agent | Browser: Suche „Zweiblatt“ im Panel „Welt“, Umschalter „Baum / Liste“, Popover „Begriffe“, „Ohne Ortsnamen“; Texte im DOM lesen (`find`, `get_page_text`) |
| (C) 1–4 Raster | Agent | Browser: Tab „Raster“, Header 0411, 0003, Route 230; die „+“-Zelle öffnet den Dialog „Karte bei x …, y … anlegen“. Neue Karten, Raster und Eventdateien sind sofort auf der Platte; `call.sh member_counts` zeigt die Zahlen |
| (D) 1–3 Weltkarte | Agent | Browser: Tab „Weltkarte“, Zellenklick, Zoomen mit dem Mausrad, Linien bei Zweiblattdorf, nach (A) der neue Raum |
| (D) 4 HeartGold US | Agent | frische Kopie `hg-3a`, `?project=…`; Baum und Weltkarte laden, New Bark Town = Header 0060 |
| (E) 1–2 Automatik | Agent | `scripts/gate.sh` (rund 25 Minuten, im Hintergrund mit `> target/agent/gate.log` und Tabelle am Ende übernehmen) |
| (F) Kartenbilder: Ausrichtung | Agent + Blick | Tab „Raster“ und 3D-Ansicht von oben bei Zweiblattdorf nacheinander ansehen, Norden oben, nicht gespiegelt; der Nutzer bestätigt |
| (F) Kartenbilder ohne apicula | Agent | App mit falschem Pfad starten: `G4_APICULA=/nirgends bash scripts/agent/studio-web.sh …`, Platzhalter und Tooltip lesen |
| (F) Layout im schmalen Panel | Agent | Browser-Fenster so verkleinern (`resize_window`) oder den Teiler ziehen, dass das Panel „Welt“ 300 px breit ist; Listenansicht, Popover „Begriffe“ und „+ Neu“ ansehen, Escape und Klick daneben |
| (F) Tastatur während eines Assistenten | Agent | Assistent öffnen, Escape, Backspace, Cmd+Z, Cmd+S drücken; danach `call.sh dirty` (leer = nichts verändert) |
| (F) Raum-Assistent, Fehlerfälle | Agent | Browser: Außen-Kachel x 999 im Schritt Eingang, gesperrte Eventdatei, „Raum hier anbinden…“ im Inspector |
| (F) Layout nach Neustart | Agent | Panel verschieben, Seite neu laden (das Layout liegt im `localStorage`) |
| Gebündelte App statt `tauri dev` | Hand | kurzer Durchgang im nativen Fenster am Ende (`pnpm bundle`, siehe [release.md](release.md)) |

## Phase 3b.1 ([phase-3b1-gate.md](phase-3b1-gate.md))

Kopien `pt-3b1` (Platin DE, **nicht** vorher mit DSPRE gepatcht) und `hg-3b1` (HeartGold DE).

| Punkt | Wer | Wie |
|---|---|---|
| (A) 1 Ausgangslage | Agent | `ls -l <Kopie>/files/fielddata/maptable/mapname.bin` (Größe notieren), `ls <Kopie>/arm9/arm9.bin.backup` muss fehlen; Zahl der Header: `call.sh list_headers` oder letzte ID im Explorer plus eins |
| (A) 2–3 Assistent bis Patch | Agent | Browser: Header 0342, „+ Neu“ → „Gebiet anlegen…“, zwei Zellen übrig, Name „Route 231“, Schritt Header: „Neuer Header“ ist gesperrt, Kasten „Patch „Dynamische Header“ nötig“ lesen, vorher speichern, „Patch anwenden“ |
| (A) 4 Platte nach dem Patch | Agent | `cmp <Kopie>/arm9/arm9.bin <Kopie>/arm9/arm9.bin.backup` meldet Unterschiede; `ls <Kopie>/unpacked/dynamicHeaders \| wc -l` gleich Zahl der Header; `ls -l <Kopie>/arm9/arm9.bin*` |
| (A) 5–6 Neuer Header | Agent | Browser: „Neuer Header“, „Interner Name“ auf `R231`, Zusammenfassung lesen, „Anlegen“, „Im Studio zeigen“ |
| (A) 7 Platte nach dem Anlegen | Agent | `ls -l <Kopie>/unpacked/dynamicHeaders/NNNN` (24 Byte); `mapname.bin` genau 16 Byte länger; `tail -c 16 <Kopie>/files/fielddata/maptable/mapname.bin \| xxd` beginnt mit `R231` |
| (A) 8 Variante „behalten“ | Agent | Browser: zweites Gebiet „Route 232“, Eventdatei „behalten“; Gegenprobe `call.sh get_header '{"id":NNNN}'` (gleiche `event_file_id` wie 0342) |
| (A) 9–10 Warp und Gang im Spiel | Agent | Warps in der Oberfläche setzen (siehe 3a (A) 11) und mit `g4emu` laufen, **oder** das Szenario `pt-neuer-header.yaml` auf einer weiteren frischen Kopie (Patch, neuer Header, Treppe, hin und zurück ohne Klick) |
| (A) 11 NARC im ROM | Agent | `target/debug/g4k extract <Kopie>/out.nds target/agent/gate-<datum>/rom-extrakt` und prüfen, dass `…/files/debug/cb_edit/d_test.narc` existiert (`g4k` steht nach `cargo build -p g4-kernel` bereit; der Zielordner darf nicht unter `../Roms` liegen) |
| (A) 12–15 DSPRE-Gegenprobe | Hand | DSPRE läuft nur unter Windows |
| (A) 16 HeartGold DE | Agent | wie (A) 2–10 auf `hg-3b1`, Interner Name `HG_TEST`, Spielstand IPKD (Zimmer Header 64, Treppe nach 63); `ls <Kopie>/files/a/0/5/0` nach dem Bauen |
| (A) 17 Diamant/Perl | Hand | entfällt ohne Projekt |
| (A) 18 Patch reparieren | Agent | Fehlerzustand auf einer Kopie nachstellen: nach „Patch anwenden“ die 4 Byte bei 0x3A0D6 in `arm9/arm9.bin` auf `CC F7 74 FC` setzen (`printf '\xCC\xF7\x74\xFC' \| dd of=<Kopie>/arm9/arm9.bin bs=1 seek=$((0x3A0D6)) conv=notrunc`), Seite neu laden: Meldung „Patch „Dynamische Header“ fehlerhaft …“ und Knopf „Patch reparieren“, drücken; danach `xxd -s $((0x3A0D6)) -l 4 <Kopie>/arm9/arm9.bin` zeigt `de f7 ad f8`, bauen, `g4emu continue` läuft weiter. Ein unversehrtes Projekt zeigt die Meldung nicht. Dass DSPRE selbst so patcht, gehört zur DSPRE-Gegenprobe (Hand) |
| (B) 1–5 Höhen | Agent + Blick | Browser: Tab „Raster“, „Höhen“ einschalten, Zelle anklicken, Inspector „Höhe“ = 2; Hover und Ziehproben mit `hover` und `left_click_drag`, Messwerte über `get_matrix`; wie die erhöhte Zelle aussieht bestätigt der Nutzer am Bild |
| (C) 1–7 Sprites | Agent + Blick | Agent prüft: Sprite statt gelbem Quadrat, Wechsel des Feldes „Richtung“ (0 bis 3) ohne Neuladen, Platzhalter bei „Grafik“ 60000 (danach Cmd+Z). Aufrecht, nicht gespiegelt und Blickrichtung bestätigt der Nutzer am Bild |
| (D) 1–3 Kachelnamen HGSS | Agent | Browser: Hover auf Bäumen (`HEADBUTT`), Gras, Wasser, Hüpfkante; Platin: derselbe Wert 0x06 ohne `HEADBUTT`; Namen mit `call.sh tile_behaviors` gegenprüfen |
| (E) 1–2 `gate.sh` | Agent | `scripts/gate.sh`, Tabelle in die Ergebnisse übernehmen; keine Zeile „skipped“ außer den dokumentierten |
| (F) 1–3 apicula-Zeitlimit | Agent | `G4_APICULA_TIMEOUT_SECS=1 bash scripts/agent/studio-web.sh …` mit frischer Kopie ohne `.g4/models`; `ls <Kopie>/.g4/models` (keine `.fail`, `last-tool` mit 16 Hexzeichen) |

## Was beim Nutzer bleibt

- **DSPRE-Gegenprobe** (3b.1 (A) 12–15) und **Diamant/Perl** (17): DSPRE läuft nur unter Windows.
- **Stoppuhr und „Zahlen getippt“** (3a (A)): misst die Bedienung durch den Menschen.
- **Natives Fenster:** ein kurzer Durchgang mit der gebündelten App nach `pnpm bundle`.
- **„Agent + Blick“:** 3a (F) Ausrichtung der Kartenbilder; 3b.1 (B) Höhen und (C) Sprites (aufrecht, nicht gespiegelt, Blickrichtung). Der Agent nennt dafür Ort und Zahlen, damit der Blick kurz bleibt.
