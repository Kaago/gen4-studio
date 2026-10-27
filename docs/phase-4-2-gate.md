# Phase 4.2: Gate prüfen

Stand: 2026-10-27. Gate 4.2 „Build-Graph“: vom Agenten bestanden, (G1) bis (G6) über die Test-Umgebung und auf frischen Kopien (Ergebnis unten). Beim Nutzer offen: (G7), also „Hier spielen“ in der gepackten App, die DSPRE-Gegenprobe und der Blick auf die Schritte-Tabelle.

Phase 4.2 baut nur noch die Schritte, deren Eingaben sich geändert haben. Lieferumfang:

- ein BLZ-Cache für ARM9 und Overlays unter `.g4/build/blz/`;
- der Bauzustand `.g4/build/state.json` mit Fingerabdrücken je Datei (`stat` zuerst, xxh3 nur bei Abweichung oder im racy-Fenster von 2 s) und Werkzeug-Salzen aus `Cargo.lock`;
- die Schritte `rotom`, `chatot`, `pack:<Ordner>` (Member-genaues Einsetzen in das vorhandene NARC), `watch:<Ordner>` und `rom` (mit den Zeilen `blz:arm9`, `blz:overlays`, `rom`, `crc`);
- Fingerabdrücke statt der Zeitregel für Texte: Ein beidseitig geändertes Archiv fragt (Dialog „Texte auf zwei Seiten geändert“);
- 14 (Platin) bzw. 17 (HGSS) DSPRE-Ordner als Pack-Einträge in `archives.yaml`;
- vier Prüfungen `build.hidden-file` (mit „Entfernen“), `build.member-gap`, `build.unpacked-not-packed` und `build.base-missing`;
- die Anzeige der Schritte mit Dauer in Technik › Prüfen und Bauen, „Neu bauen (ohne Cache)“, `g4k build --timings` und `--no-cache`;
- der Schritt „build graph (timings)“ in `scripts/gate.sh`.

Gate aus der [Roadmap](ROADMAP.md), Phase 4.2:

> je Projekt nach je einer Änderung an Karte, Text und Script „Hier spielen“ bzw. `g4k build --timings` ≤ 0,5 s warm, ohne Änderung ≤ 0,2 s (Zeiten je Projekt und Schritt im Gate-Bericht, kalte Zeiten nur protokolliert); Ergebnis byte-gleich zum vollen Build ohne Cache; eine simulierte DSPRE-Änderung wird erkannt; geändertes Werkzeug-Salz führt zum vollen Build; Branch-Wechsel mit geänderter Text-JSON, dann Build: die JSON des Branches gewinnt; der Vanilla-Rebuild bleibt mit den neuen Pack-Einträgen byte-gleich; `scripts/gate.sh` mit dem Schritt grün.

Wer was prüft: Die Spalte „Wer“ sagt **Agent** (Test-Umgebung aus [agent-gate.md](agent-gate.md): Browser-Fenster mit `studio-bridge`, `call.sh`, Szenario), **Agent + Blick** (der Agent prüft und beschreibt Ort und Zahlen, der Nutzer bestätigt kurz am Bild) oder **Hand** (der Nutzer). Zuordnung: [agent-gate.md](agent-gate.md), Abschnitt „Phase 4.2“.

## Punkte

- (G1) Automatik: `scripts/gate.sh` grün. Darin laufen die gated Tests der Tasks 1 bis 9 auf Platin DE, HeartGold DE und HeartGold US, `project_start_*` je ROM und der Schritt „build graph (timings)“.
- (G2) Byte-Gleichheit:
  - inkrementell gleich voll ohne Cache, nach Karte, Text und Script;
  - der Vanilla-Rebuild gleich der `dsrom`-CLI, mit den Pack-Einträgen;
  - ein extrahiertes Projekt gleich der SHA-1 aus `.g4/base.json`.
- (G3) Zeiten je Projekt: voll (ohne Cache), inkrementell nach einer Karten-, Text- und Scriptänderung, ohne Änderung, kalt (nur Protokoll). Grenzen: ≤ 0,5 s und ≤ 0,2 s warm.
- (G4) DSPRE und Git:
  - eine simulierte DSPRE-Änderung wird erkannt;
  - ein Salzwechsel baut voll;
  - nach einem Branch-Wechsel mit geänderter Text-JSON gewinnt die JSON des Branches;
  - ein beidseitiger Wechsel fragt.
- (G5) Szenarien über `g4emu`: `pt-treppe-warp.yaml` und `hg-neuheim.yaml`. Jede Kopie wird vorher einmal gebaut, damit der Build des Szenarios inkrementell läuft.
- (G6) Browser-Teil, Technik › Prüfen und Bauen:
  - Schritte-Zeilen, „übersprungen“ und „ROM schreiben … aktuell“;
  - eine Kartenänderung zeigt „1 Member eingesetzt“ und die Zeit im Toast;
  - „Neu bauen (ohne Cache)“;
  - ein Text-Konflikt mit beiden Wahlen;
  - die Befunde `build.hidden-file` mit „Entfernen“ und `build.base-missing`.
- (G7) Hand und Blick beim Nutzer, siehe „Hand“ und „Blick“ unten.

### Vorbereitung

1. Kopien nur frisch unter `Roms/_testkopien/4-2-<zweck>` (das Ziel darf nicht existieren). Die Originale werden nie geöffnet. Alle Kopien `4-2-*` sind nach dem Gate gelöscht.
2. Werkzeuge:
   - `cargo build --release -p g4-kernel --bin g4k` für (G3) und (G4);
   - `target/debug/studio-bridge` und `target/debug/g4k` für (G5) und (G6).
3. Test-Umgebung aus dem Worktree auf eigenen Ports: `G4_ROMS_DIR=<Roms> G4_VITE_PORT=1510 G4_BRIDGE_PORT=1511 bash scripts/agent/studio-web.sh`. Die Seite ist `http://127.0.0.1:1510/?project=<Kopie>`, das Viewport 800 × 600.

### (G3) Wie gemessen

Das Skript `g3.sh` (im Scratchpad des Agenten; Ablauf hier) arbeitet je Projekt auf einer frischen Kopie `4-2-zeiten-{pt,hgde,hgus}`:

1. `open_project` und `adopt_scripts` über die Brücke.
2. kalt: `g4k build --timings --no-cache` (der erste Build der Kopie, Seiten-Cache leer).
3. voll: `--no-cache` viermal.
4. Karte: `g4k map set-perm <Kopie> 0 0 0`, die Kollision wechselnd mit 0x80 gekippt, dann `g4k build --timings`.
5. Text: über die Brücke `edit_text_archive` (Nachricht 0 des ersten Archivs mit Nachrichten, ein „!“ mehr) und `save`, dann `g4k build --timings`.
6. Script: an die erste `.rotom`-Quelle ein Probe-Script `script probe #1999` mit `Message <Lauf>` anhängen, dann `g4k build --timings`.
7. ohne Änderung: `g4k build --timings` viermal.

Von je vier Läufen fällt der erste weg (Aufwärmen); der Wert ist der Median der drei anderen, `ms` aus der JSON-Zeile von `--timings` (die Build-Zeit ohne Start des Prozesses). Die Rohzeilen liegen unter `target/agent/gate-2026-10-27/4-2/zeiten/*.jsonl`.

## Ergebnis

Lauf des Agenten am 2026-10-27 im Worktree des Branches `phase-4-2-build-graph`, Test-Umgebung auf den Ports 1510/1511, Viewport 800 × 600, Lastmittel 3 bis 12 (parallele Sitzungen). Die Zeilen (G3) bis (G6) beschreiben den Stand nach der Fix-Runde (Commit 9be9a38, `.fig` 1cadadb); die erste Runde und ihre Befunde stehen darunter. Danach kam die Fix-Welle der Schlussprüfung (8d913c6, f251cc8: unter anderem zählt die ctime jetzt für die racy-Regel, der Schritt `rom` liest auch `arm7_overlays/` und `multiboot_signature.yaml`, „Neu bauen (ohne Cache)“ schreibt erst die Text-Einträge und löscht dann); sie ist durch (G1) gedeckt, das auf f251cc8 lief.

| Punkt | Wer | Datum | Projekt | Ergebnis | Auffälligkeiten |
|---|---|---|---|---|---|
| (G1) `scripts/gate.sh` | Agent | 2026-10-27 | Platin DE, HeartGold DE, HeartGold US | bestanden („Gate: ok (3084 s)“) | auf f251cc8, aus dem Worktree mit `G4_ROMS_DIR` absolut und `tools` als Symlink; alle Schritte beim ersten Lauf grün, auch „build graph (timings)“ (Release, `G4_TIMING_STRICT=1`) ohne Wiederholung: Karte 239 / 261 / 267 ms, Text 248 / 280 / 273 ms, Script 289 / 324 / 305 ms, ohne Änderung 94 / 97 / 95 ms (Platin DE / HeartGold DE / HeartGold US). Tabelle unter dieser; Log `target/agent/gate-4-2.log` |
| (G2) Byte-Gleichheit | Agent | 2026-10-27 | alle drei (gated, auf `vanilla_copy()`) | bestanden | Die gated Tests laufen in (G1) mit: inkrementell gleich voll ohne Cache nach Karte, Text und Script und ohne Änderung (`map_edit_incremental_equals_full`, `text_edit_…`, `script_edit_…`, `incremental_equals_full_after_nothing_changed`); BLZ-Cache gleich `Rom::load` (`blz_cache_path_equals_rom_load`); Vanilla-Rebuild gleich der `dsrom`-CLI mit den Pack-Einträgen (`vanilla_rebuild_matches_dsrom_cli`, `…_after_adopt`, `pack_only_entries_repack_byte_identical`); `project_start_incremental_build_matches_the_rom`. `dspre_mark_build_matches_the_base` meldet „skipped: no base mark“, bis 4.1a die Marke für DSPRE-Ordner bringt (der Vergleich mit `.g4/base.json` läuft in `project_start_*` über `g4k create`) |
| (G3) Zeiten | Agent | 2026-10-27 | `4-2-zeiten-{pt2,hgde,hgus2}` | bestanden | Tabelle unten: nach Karte, Text und Script 237 bis 383 ms, ohne Änderung 73 und 74 ms; zwei Läufe unter Last wiederholt (unten beschrieben) |
| (G4) 1 DSPRE und Salz | Agent | 2026-10-27 | alle drei (gated) | bestanden | `simulated_dspre_edit_is_detected`: Member in place überschrieben, Größe, mtime und Inode gleich, nur die ctime weckt `pack:maps`; `salt_change_builds_fully` |
| (G4) 2 Git-Wechsel | Agent | 2026-10-27 | `4-2-git-pt2` | bestanden | `git init`, nur `expanded/` versioniert; Textarchiv 0012 auf `main` „Hauptzweig“, auf `zweig` „Zweig B“. Nach jedem `git switch` baut `g4k build` `chatot` und `pack:textArchives` inkrementell (214 und 220 ms), der Member folgt dem Zweig (SHA-1 beef2e20afed auf `main`, 1362314d31c8 auf `zweig`), `git status` bleibt leer, ohne Änderung 72 ms. Member von außen geändert und dann gewechselt: `g4k build` endet mit Rückgabe 1 und „text archives changed as JSON and as member since the last build: 0012“; die Brücke antwortet `build_rom` mit HTTP 400, Code `text_conflict` |
| (G5) 1 `pt-treppe-warp.yaml` | Agent + Blick | 2026-10-27 | `4-2-szenario-pt` | bestanden | Vorbau `g4k build` 23,6 s (kalt). Im Szenario baut die Brücke (Debug) nach der Änderung an Warp 0 der Eventdatei 0394 nur `pack:eventFiles` (inkrementell, 11 ms) und `rom`. Rückgabe 0, `"ok": true`, Header 415 → 412 → 411. Einmalig kostete der übersprungene Schritt `chatot` 2,6 s, weil die eben kopierten Text-Member beim Vorbau racy ohne Hash eingetragen waren; danach 5 ms. Bilder `angekommen.png` und `draussen.png` |
| (G5) 2 `hg-neuheim.yaml` | Agent + Blick | 2026-10-27 | `4-2-szenario-hg` | bestanden | Vorbau 38,8 s (kalt). Der Build des Szenarios (neues Gebiet mit Header-Patch, Teile importiert) lief inkrementell: `chatot`, `pack:maps`, `pack:matrices`, `pack:eventFiles`, `pack:textArchives`, `pack:dynamicHeaders`, `blz:overlays` aus dem Cache, `blz:arm9` voll (der Header-Patch ändert ARM9), 3,5 s in der Debug-Brücke. Rückgabe 0, `"ok": true`, Header 64 → 540, Ort 235. Bild `einblendung.png` |
| (G6) 1 Schritte-Zeilen | Agent | 2026-10-27 | `4-2-browser-hgde2` | bestanden | Erstes „Bauen“ 25,15 s (kalt, Debug-Brücke), alle 45 Zeilen `data-status=full` „(erster Build, voll)“. Zweites: „Kein Schritt lief: alles war aktuell.“, „Übersprungen (42)“ (Taste mit `aria-expanded`), darin `rom` „ROM schreiben aktuell“; 0,43 s |
| (G6) 2 Kartenänderung | Agent | 2026-10-27 | `4-2-browser-hgde2` | bestanden | `call.sh set_permission` an Karte 1, dann „Bauen“: Toast „ROM gebaut: …/out.nds in 0,79 s (1 Datei(en) vorher gespeichert)“, Zeilen `pack:maps` incremental „Karten packen 1 Member eingesetzt 0,06 s“, „Spielcode (ARM9) 1 aus dem Cache“, „Overlays 127 aus dem Cache“, „ROM schreiben geschrieben“, „Prüfsummen keine Basis-Marke“. In der ersten Runde (vor 9b) 2,36 s |
| (G6) 3 „Neu bauen (ohne Cache)“ | Agent | 2026-10-27 | `4-2-browser-hgde2` | bestanden | `title` „Löscht den Bauzustand und baut alles neu“, fragt im Browser-Modus nach dem Pfad, 20,20 s, 45 Zeilen `full` mit „(ohne Cache, voll)“, „Overlays 127 komprimiert“ |
| (G6) 4 Text-Konflikt | Agent | 2026-10-27 | `4-2-browser-hgde`, `4-2-browser-hgde2` | bestanden | JSON und Member von Textarchiv 0012 von außen geändert: „Bauen“ öffnet `[data-text-conflict]` „Texte auf zwei Seiten geändert“ mit „Textarchiv 0012“, Gruppe „Fassung von Textarchiv 0012“, „Text (JSON)“ vorgewählt. „Abbrechen“: „Nichts gebaut. Beim nächsten „Bauen“ fragt das Studio wieder, welche Fassung gilt.“ „Spieldatei“: die JSON kommt aus dem Member (Schlüssel 4232, 334 Nachrichten wie Archiv 0013), der Member bleibt. „Text (JSON)“: der Member wird neu kodiert, die Bytes der JSON bleiben (SHA-1 vorher und nachher 54770fc27023). Während des Übernehmens sind beide Tasten und die Wahl gesperrt, darunter „Die gewählten Fassungen werden übernommen …“; danach `chatot` inkrementell „Texte 0 kodiert“ |
| (G6) 5 `build.hidden-file`, `build.base-missing` | Agent | 2026-10-27 | `4-2-browser-hgde2` | bestanden | `files/.DS_Store` angelegt: „Bauen“ meldet „1 versteckte Datei unter files/ hält den Build an; entferne sie (in den Problemen „Entfernen“) und baue erneut.“ mit „Probleme zeigen“. Gruppe „Versteckte Datei im ROM-Ordner 1“, „Beheben…“ mit „Nicht rückgängig“ und „files/.DS_Store → .g4/backup/hidden-<UTC-Zeitpunkt>/files/.DS_Store“, „Entfernen“ verschiebt sie. Das nächste „Bauen“ gelingt (0,44 s) und schließt die alte Fehlermeldung. Gruppe „Basis-ROM unbekannt 1“ mit „Ordne sie in Technik zu („Basis-ROM zuordnen…“)“ |
| (G7) gepackte App, DSPRE, Blick | Hand | – | – | offen | siehe „Hand“ und „Blick“ |


Ergebnis von `scripts/gate.sh` (2026-10-27, Worktree des Branches auf f251cc8, `G4_ROMS_DIR` absolut, Log `target/agent/gate-4-2.log`; die Zeilen `build-timings:` des Laufs: Platin DE voll 1 525, kalt 10 945; HeartGold DE voll 6 751, kalt 20 229; HeartGold US voll 5 293, kalt 15 938 ms):

```
Step                                                 Project                                      Result                     Seconds
----                                                 -------                                      ------                     -------
cargo fmt --check                                    -                                            ok                               1
cargo clippy (-D warnings)                           -                                            ok                              10
cargo test (no ROM data)                             -                                            ok                             124
mcp                                                  -                                            ok                              26
pnpm test                                            -                                            ok                              96
tokens                                               -                                            ok                               0
styles                                               -                                            ok                               2
pnpm build                                           -                                            ok                              21
dist has no bridge                                   -                                            ok                               1
g4emu unit                                           -                                            ok                              32
g4emu integration                                    -                                            ok                               7
gated tests (cargo test)                             Pokemon - Goldene Edition HeartGold (Germany) ok                             808
gated tests (cargo test)                             Pokemon - HeartGold Version (USA)            ok                             866
gated tests (cargo test)                             Pokemon - Platin-Edition (Germany)           ok                             772
build graph (timings)                                Pokemon - Goldene Edition HeartGold (Germany) ok                              90
build graph (timings)                                Pokemon - HeartGold Version (USA)            ok                              72
build graph (timings)                                Pokemon - Platin-Edition (Germany)           ok                              49
gated tests (project start)                          Pokemon - Goldene Edition HeartGold (Germany) ok                              39
gated tests (project start)                          Pokemon - HeartGold Version (USA)            ok                              33
gated tests (project start)                          Pokemon - Platin-Edition (Germany)           ok                              35

Gate: ok (3084 s)
```

### Zeiten je Projekt (G3)

Gemessen am 2026-10-27 mit `g4k build --timings` (Release) auf frischen Kopien, Median aus drei Läufen nach einem Aufwärmlauf, in ms. „heute“ ist der Stand vor 4.2 aus Gutachten B (§1.3 und §2.6: voller Build warm, Median; eine Kartenänderung baute bisher alles). Grenzen: ≤ 500 ms nach Karte, Text und Script, ≤ 200 ms ohne Änderung; „voll“ und „kalt“ haben keine Grenze.

| Projekt | heute (vor 4.2) | voll (ohne Cache) | nach einer Kartenänderung | nach einer Textänderung | nach einer Scriptänderung | ohne Änderung | kalt (nur Protokoll) |
|---|---|---|---|---|---|---|---|
| Platin DE | 1 100 (mit Text-JSON 1 230) | 1 585 | **237** | **239** | **383** | **73** | 20 436 |
| HeartGold DE | 4 690 (4 890) | 6 801 | **246** | **256** | **349** | **74** | 29 739 |
| HeartGold US | 5 240 (4 930) | 7 258 | **237** | **257** | **364** | **73** | 29 024 |

Alle Werte liegen unter den Grenzen. Nach einer Kartenänderung baut „Hier spielen“ auf HeartGold also etwa 19-mal schneller als heute (4,7 s → 0,25 s), auf Platin etwa 4,6-mal (1,1 s → 0,24 s).

Zum Vergleich:

- **Stand vor der Leistungsrunde 9b** (Commit b2c463b, gleiches Verfahren). Werte voll / Karte / Text / Script / ohne Änderung / kalt:
  - Platin DE 1 596 / 292 / 307 / 446 / 125 / 20 381;
  - HeartGold DE 5 265 / 273 / 274 / 416 / 129 / 26 024;
  - HeartGold US 5 481 / 277 / 274 / 417 / 125 / 25 315.
- **Gated Test `build_graph_timings`** (Release, `G4_TIMING_STRICT=1`, in einem Prozess, zwei Läufe je Projekt nach 9b). Je Fall der erste und der zweite Lauf:

  | Fall | Platin DE | HeartGold DE | HeartGold US |
  |---|---|---|---|
  | Karte | 247 / 244 | 265 / 284 | 274 / 260 |
  | Text | 273 / 290 | 289 / 281 | 254 / 266 |
  | Script | 306 / 314 | 287 / 341 | 306 / 290 |
  | ohne Änderung | 96 / 95 | 95 / 103 | 97 / 94 |

  Alle sechs Läufe bestanden beim ersten Versuch. Der schlechteste Wert ist Script 341 ms (32 % unter der Grenze).

Wo die Zeit steckt (Zeilen der JSON, HeartGold DE nach einer Kartenänderung):

- `pack:maps` „1 Member eingesetzt“ 10 bis 25 ms;
- `blz:*` aus dem Cache;
- `rom` 140 bis 180 ms (ds-rom baut und schreibt die ROM);
- `crc` unter 10 ms;
- jeder übersprungene Schritt unter 10 ms.

Nach einer Scriptänderung kommt `rotom` mit 216 bis 275 ms dazu. Davon fallen etwa 150 ms weg, wenn ein Prozess die Quell-Hashes schon kennt (App und gated Test, nicht ein frisches `g4k`). Ohne Änderung bleiben `state.json`, ein `stat` je Member und einmal der Hash der eben geschriebenen ROM (10 bis 15 ms).

**Wiederholungen unter Last:** Der Rechner lief mit Lastmitteln 6 bis 9 (parallele Sitzungen). Microsoft Defender prüfte die frisch kopierten Ordner. Gleichzeitig schrieb der Agent die `.fig` und kopierte sie. Dabei fielen zwei Läufe aus dem Rahmen:

- **Platin DE, erster Lauf:** Karte im Median 870 ms, mit `rom` 676 bis 1 164 ms in zwei von drei Läufen, und Script einmal 1 052 ms. Text 241, Script 436, ohne Änderung 74 ms.
- **HeartGold US, erster Lauf:** voll 38 470 ms, jeder Schritt 5- bis 10-mal langsamer, während Defender die Kopie las. Karte 234, Text 238, Script 337, ohne Änderung 75 ms.

Beide Projekte wurden auf neuen frischen Kopien (`4-2-zeiten-pt2`, `4-2-zeiten-hgus2`) wiederholt; die Tabelle zeigt die Wiederholung. Der volle Build ohne Cache streut mit der Last: auf HeartGold DE von 5,3 s (Last 2 bis 5) bis 6,8 s (Last 8), verteilt auf alle Schritte (BLZ +230 ms, rotom +150 ms, jeder Pack-Schritt +30 bis 60 ms), nicht auf einen einzelnen. Rohzeilen unter `target/agent/gate-2026-10-27/4-2/zeiten/` (Unterordner `vorher/` für den Stand b2c463b).

### Befunde der Gate-Läufe und die Fix-Runde

Die erste Runde von (G3), (G4) und (G6) lief auf dem Stand vor der Leistungsrunde (Commit b2c463b). Ihre Befunde behoben die Commits 7ed9f08 (Leistung), be32f24 (Kernel) und 9be9a38 (Oberfläche). Danach liefen (G3), (G4) und (G6) noch einmal auf neuen frischen Kopien (Zeilen oben).

- **Zu wenig Puffer bei den Zeiten** (Review von Task 9): Script lag bei 444 bis 501 ms, ohne Änderung bei 175 bis 186 ms; der erste Lauf von `gate.sh` scheiterte auf Platin DE mit 501 ms. Ursachen laut Profil:
  - `state.json` wurde je Build 7- bis 13-mal gelesen;
  - rotom hashte bei jedem Build alle 1 124 Quellen;
  - die eben geschriebene ROM und das NARC wurden noch einmal gelesen.

  Jetzt bleibt der Zustand im Prozess (neu gelesen nur bei geändertem `stat`) und wird einmal je Build-Hälfte geschrieben. rotom merkt sich die Hashes unveränderter Quellen. Ausgaben werden mit dem Hash der geschriebenen Bytes eingetragen.
- **F1, Git:** Der Build schrieb eine gewinnende Text-JSON in eigener Form zurück; nach `git switch` und Bauen war der Arbeitsbaum schmutzig (nur der Zeilenumbruch am Ende fehlte). Jetzt schreibt er nur den Member (`text::encode_member`), die Bytes der JSON bleiben.
- **F2:** Nach dem Auflösen eines Konflikts hieß die Zeile `chatot` „(erster Build, voll)“. Jetzt behält ein Schritt, der an einem Konflikt anhält, seinen Eintrag; danach läuft `chatot` inkrementell („Texte 0 kodiert“).
- **F3:** Die Fehlermeldung zur versteckten Datei blieb nach „Entfernen“ und einem gelungenen Build stehen. Jetzt schließt ein gelungener Build die offenen Fehlermeldungen früherer Builds.
- **F4:** Die Fix-Vorschau nannte einen anderen Sicherungsordner (`hidden-20261027-060846`) als den benutzten (`…-060854`). Jetzt steht dort `hidden-<UTC-Zeitpunkt>`.
- **F5:** Nach „Neu bauen (ohne Cache)“ lautete der Grund „(erster Build, voll)“. Jetzt heißt er „(ohne Cache, voll)“.
- **Aus Task 8:**
  - `buildSteps.ts` heißt jetzt `stepWords.ts` (Falle bei Dateisystemen ohne Groß- und Kleinschreibung neben `BuildSteps.tsx`);
  - „Hier spielen“ spielt nach dem Auflösen weiter;
  - der Dialog wartet sichtbar, solange die Wahl geschrieben wird.

### Bekannte Grenzen

- **Zwei Prozesse auf einem Projekt:** `g4k build` parallel zur App kann Einträge in `state.json` verlieren. Das kostet höchstens einen vollen Build und verfälscht das Ergebnis nicht. Der Zustand im Prozess vertraut einem gleichen `stat` von `state.json`; ein fremdes Werkzeug, das die Datei im selben Zeittakt in gleicher Größe neu schreibt, bleibt unbemerkt (alle Schreiber des Studios ersetzen die Datei).
- **Zwei Builds an denselben Pfad zugleich** (zwei Prozesse, oder die Oberfläche und ein MCP-Agent in der App) können dem Emulator für diesen einen Lauf eine halb geschriebene ROM geben: ds-rom schreibt die ROM an Ort und Stelle, und der Build läuft ohne Sperre je Projekt. Der nächste Build bemerkt es und baut neu. Das gab es schon vor 4.2.
- **DSPRE speichert während eines Builds:** Fällt ein Speichern in DSPRE genau in die Millisekunden zwischen der Prüfung und dem Eintrag der Text-Synchronisation, gilt das Archiv als gleich. Der nächste echte Unterschied wird wieder erkannt.
- **Pack-Einträge auf ungeprüften Sprachen:** Fehlt das gepackte NARC eines Pack-Eintrags im Projekt, legt der Build es aus `unpacked/` neu an (das braucht ein Klon ohne die ignorierten NARCs). Liegt die Datei in einer ungeprüften Sprache (etwa Koreanisch) an anderer Stelle, entstünde dort eine überzählige Datei; gemessen sind nur Platin DE, HeartGold DE und HeartGold US.
- **Die Eingaben des Schritts `rom`** stehen fest im Code (das Layout von ds-rom); ein von Hand geändertes `config.yaml`, das auf andere Dateien zeigt, wird nicht verfolgt. Nach 4.1a prüfen, ob die Basis-Marke weitere Dateien liest (`base-roms.json`, Patch); die gehören dann zu den Eingaben von `rom`.
- **Ein beschädigter BLZ-Cache** wird nur am Ende der Datei erkannt; „Neu bauen (ohne Cache)“ heilt ihn.
- **Kopflos lässt sich ein Text-Konflikt** nicht über `g4k` lösen, nur in der App oder über `studio mcp` mit der Freigabe für Unumkehrbares.
- **Windows ist nicht gemessen:** `ctime` und `ino` fehlen dort; die racy-Regel und der Inhaltshash tragen allein. Die gemerkten Hashes von rotom teilen diese Lücke (eine Datei mit gleicher Größe und mtime ersetzt), ebenso der kurze Moment zwischen dem Umbenennen von `state.json` und dem Lesen seines `stat` (unter Unix über den Datei-Handle geschlossen).
- **Ein frisches `g4k build` nach einer Scriptänderung** liest alle Quellen einmal (etwa 150 ms); die App und der gated Test merken sich die Hashes im Prozess.
- **Volle Builds streuen mit der Last** (HeartGold 5,3 bis 7,3 s); sie haben keine Grenze. Unter dieser Last lag der volle Build ohne Cache auf HeartGold über dem Wert von heute (4,7 s, in Gutachten B auf einem ruhigeren Rechner gemessen); ob ein Teil vom Eintragen der Fingerabdrücke kommt, ist nicht getrennt gemessen. Zum Vergleich: Mit Lastmittel 2 bis 5 maß dasselbe Verfahren vor 9b 5,3 s.
- **`build.base-missing`** prüft in 4.2 nur die Marke `.g4/base.json`. Der Satz verweist auf „Basis-ROM zuordnen…“ in Technik; die Stelle bringt 4.1a, beim Zusammenführen anpassen. Ob auch „Marke da, Datei nicht gefunden“ gemeldet wird (über `base-roms.json` von 4.1a), entscheidet der Merge. `dspre_mark_build_matches_the_base` meldet bis dahin „skipped: no base mark“.
- **Member-Hashes für 4.1b:** `state.json` trägt xxh3 nur für Einträge, die gehasht wurden.
- **Ordner, die DSPRE selbst packt:** Ob DSPRE die neuen Pack-Ordner beim Speichern packt, bleibt Hand (G7). Ordner ohne Pack-Eintrag meldet `build.unpacked-not-packed`.
- **Bauen zeigt für 30 Pack-Schritte dasselbe Wort** „Spieldaten packen“ (Ordner ohne eigenes Wort); Tüfteln zeigt die Kennung. Gelaufen sind nach einer Änderung meist nur ein oder zwei davon.
- **Zeitstempel des Sicherungsordners** (`hidden-…`) ist UTC.
- **Vorschaubild aus 4.0:** Der Plan verlangt nach dem Merge von 4.0, dass `git grep -n "preview-standard"` nur `.g4/cache/` zeigt. 4.0 ist in diesem Branch nicht enthalten; die Probe gehört in die Sitzung, die zusammenführt.
- **Das Kreuz des Konflikt-Dialogs** ist während des Wartens nicht gesperrt; ein Klick darauf tut in dieser Zeit nichts (`onOpenChange` wird ignoriert).

## Hand

1. **„Hier spielen“ in der gepackten App auf HeartGold.** In der gebündelten App (Release) ein HeartGold-Projekt öffnen, einmal „Hier spielen“ (der erste Build ist voll), dann eine Kachel malen und noch einmal „Hier spielen“. Erwartet: Der zweite Build ist spürbar schneller, der Toast nennt unter 0,5 s (heute etwa 4,7 s). In Technik › Prüfen und Bauen steht „Karten packen 1 Member eingesetzt“.
2. **DSPRE-Gegenprobe.**
   - In DSPRE auf einer Kopie einen der neuen Pack-Ordner ändern, etwa eine Größe in `pokeHeight`, und speichern.
   - Danach mit dem Studio bauen. Erwartet: In der Tabelle läuft `pack:pokeHeight` (Bauen: „Spieldaten packen … Member eingesetzt“), und die Änderung ist im Spiel.
   - Dazu die offene Frage aus den Entscheidungen: Packt DSPRE diese Ordner beim Speichern selbst (ändert sich die gepackte Datei unter `data/` oder nur `unpacked/`)?

## Blick

Bilder von `g4emu` unter `target/agent/gate-2026-10-27/4-2/` (im Worktree):

- `pt-treppe-warp/angekommen.png`: das Erdgeschoss des Rivalenhauses (Header 412) nach der umgeleiteten Treppe. Die Figur steht unten rechts neben der Matte, oben links die Mutter.
- `pt-treppe-warp/draussen.png`: Zweiblattdorf (Header 411) vor der Haustür, oben links die Einblendung „Zweiblattdorf“.
- `hg-neuheim/einblendung.png`: das neue Gebiet (Header 540) mit der Einblendung „Neuheim“ oben links, Wasser und Bäume, unten das Menü.

In der App (Test-Umgebung oder gebündelte App, Technik › Prüfen und Bauen nach „Hier spielen“ und einer Kartenänderung):

- Die Tabelle „Schritte des letzten Builds“ hell, dunkel und kompakt (Einstellungen › Darstellung). Erwartet:
  - drei Spalten „Schritt“, „Ergebnis“, „Dauer“, die Dauer rechtsbündig;
  - darunter der eingeklappte Abschnitt „Übersprungen (42)“;
  - in Tüfteln die Kennung grau in Mono hinter den Wörtern („Karten packen pack:maps“).
- Der Dialog „Texte auf zwei Seiten geändert“, Ablauf wie in (G6) 4.

In der `.fig` (Commit 1cadadb), zum Vergleich mit der App:

- Seite „App: Texte, Technik, Dialoge“: „App: Technik, Prüfen und Bauen mit Schritten“ (x 6080, y 0) und „App: Technik, Texte auf zwei Seiten geändert“ (x 10640, y 1000).
- Seite „Komponenten“: das Brett „Komponenten (App): Build-Schritte“ (x 3440, y 13020).
- Die Marke über dem Zielbild „Prüfen und Bauen“ und die Zeile im Brett „Stand der App“.
