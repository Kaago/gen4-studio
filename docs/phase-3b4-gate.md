# Phase 3b.4: Gate prüfen

Stand: 2026-10-25. Teil A (Gate 3b.4a „Prüfen und Beheben“): vom Agenten bestanden, (A1) bis (A6) über die Test-Umgebung (Automatik, Szenario und Browser-Teil; Ergebnis unten); die sieben Befunde der Abnahme und die der Schlussprüfung sind in der Fix-Welle behoben (Liste unter „Ergebnis“). Beim Nutzer offen ist der Blick auf die rote Treppe, (A6) 3. Teil B (Gate 3b.4b „Import und Gesamtgate“, zugleich das 3b-Gesamtgate) folgt mit Plan 3b.4b, Task 4.

Phase 3b.4a bringt die Projektprüfungen: ein Kernel-Modul `checks` mit 39 Prüfungen (Katalog der Spezifikation §4, harte Regel: „Fehler“ nur, wenn Platin DE, HeartGold DE und HeartGold US je 0 Treffer haben), Snapshot unter dem Store-Lock und Volllauf ohne Lock, Text- und Script-Prüfungen je Datei gecacht; vier Fixes mit Vorschau, einem Schritt im Verlauf und Nachprüfung (`wrap`, `append_message`, `swap_min_max`, `create_script`); Ausblenden und „Ausgangslage übernehmen“ in `<projekt>/g4-checks.json`; in der Oberfläche das Prüflicht mit drei Zählern, das Panel „Probleme“ mit Gruppen und den Tasten „Öffnen“, „Beheben…“, „Ausblenden“, dieselbe Tabelle in Technik › Prüfen und Bauen, den Inspector-Abschnitt „Prüfungen“, die Fehlerfarbe am Warp-Marker und Markierungen in Explorer-, Trainer- und Begegnungslisten. Dazu `g4k check` und Erwartungen (`expect`) an den Änderungen eines Szenarios. Die Originale der Test-ROMs bleiben unberührt: alles läuft auf Kopien. Spezifikation: [docs/superpowers/specs/2026-10-23-phase-3b4-import-checks-design.md](superpowers/specs/2026-10-23-phase-3b4-import-checks-design.md), §11 (Gate A). Plan: [docs/superpowers/plans/2026-10-24-phase-3b4a-checks.md](superpowers/plans/2026-10-24-phase-3b4a-checks.md).

## Teil A: Gate 3b.4a

Spezifikation §11, Gate A:

> `vanilla_checks` gated grün (Soll je Code und Projekt, Fehler = 0 überall); Szenario-Teil (Brücke, ohne Spiel): auf einer Kopie vier absichtliche Fehler anlegen (Warp ohne Ziel-Warp, Figur mit fehlendem Script, Trainer-Event ohne Trainer, zu lange Zeile) → `run_checks` zählt je Code 1 → Fixes bzw. Edits → 0; Panel, Prüflicht, Fix-Dialog, „12 ausgeblendet / Wieder melden“ im Browser-Modus über Rollen; `scripts/gate.sh` grün.

Teil A besteht aus diesen Punkten:

- (A1) Automatik: die gated Tests `vanilla_checks_match_the_fixed_counts` und `vanilla_script_checks_after_adopt` sowie die Zeitmessungen in `checks_runner.rs` grün auf Platin DE, HeartGold DE und HeartGold US; `scripts/gate.sh` grün.
- (A2) Szenario `pt-pruefungen.yaml` (Brücke, ohne Spiel): vier absichtliche Fehler werden einzeln rot und danach grün.
- (A3) Prüflicht und Panel „Probleme“ im Browser-Modus, auch der automatische Lauf nach einer Änderung.
- (A4) Fix-Dialog: „Umbrechen“, „Script anlegen“, „Befund ist nicht mehr aktuell“.
- (A5) Ausblenden, „N ausgeblendet · Wieder melden…“, „Ausgangslage übernehmen…“, Prüfungen ein- und ausschalten.
- (A6) Am Objekt: Inspector-Abschnitt „Prüfungen“, Markierung im Explorer, Fehlerfarbe am Warp-Marker.

Wer was prüft: Die Spalte „Wer“ der Ergebnistabelle sagt **Agent** (die Test-Umgebung aus [docs/agent-gate.md](agent-gate.md): Browser-Fenster mit `studio-bridge`, `call.sh`, Szenario), **Agent + Blick** (der Agent prüft und beschreibt Ort und Zahlen, der Nutzer bestätigt kurz am Bild) oder **Hand** (der Nutzer). Zuordnung und Rezepte: [docs/agent-gate.md](agent-gate.md), Abschnitte „Phase 3b.4a“ und „Rezepte“ („Prüfen“, „Beheben“, „Ausblenden“, „Ausgangslage“).

Namen in der Oberfläche (Rollen und `data-*` für die Prüfung):

- **Prüflicht** im Scharnier: `button[data-light]` mit `data-light` `error`, `warning`, `info`, `sound` oder `unchecked`; `aria-label` wie die Statusleiste („3 Fehler, 102 Warnungen, 10 Hinweise“, „Alles stimmig“; während eines Laufs mit dem Fortschritt davor, „Prüft … Texte 312/724, …“; „Nicht geprüft“ mit leerem Ring, solange kein Bericht da ist und keiner läuft, nie „Alles stimmig“) und `data-running`, solange ein Lauf läuft; die Zähler `[data-count=error|warning|info]` tragen die Zahl. Ein Klick öffnet „Probleme“.
- **Panel „Probleme“** (`section[aria-label=Probleme]`): Bereiche Welt, Scripts, Texte, Pokémon, Projekt als faltbare Abschnitte mit Zähler, darin je Code eine Gruppe `[role=group][data-check=<code>]` mit Titel und Zahl; Zeilen `li[data-finding="<code>|<art>:<index>|<schlüssel>"][data-check][data-state=error|warning|info]` mit Satz, Ort („Eventdatei 0394 · Warp 0“) und den Tasten „Öffnen“ (nur mit Ziel), „Beheben…“ (nur mit Fix), „Ausblenden“ (nie bei Fehlern). Darüber `[role=list][aria-label="Nicht geprüft"]`, darunter der Fuß mit `[role=status]` (Fortschritt), „N ausgeblendet“ und „Wieder melden…“, „Erneut prüfen“.
- **Fix-Dialog** `[role=dialog]`: Titel = Ort des Befunds, Beschreibung = Satz, Körper `[data-fix=<id>]` mit je geändertem Asset `[role=group][aria-label="Textarchiv 0558"]` und darin „Vorher“ und „Nachher“ (`[data-line]`, geänderte Zeilen `[data-changed]`); Plakette „Nicht rückgängig“ bei `create_script`; Tasten „Abbrechen“ und die Fix-Taste („Umbrechen“, „Nachricht anlegen“, „Tauschen“, „Script anlegen“); nach „Erst speichern“ „Schließen“ und „Speichern und anlegen“. Die Zeilen von „Vorher“ und „Nachher“ brechen nicht um, die Spalten rollen seitlich.
- **Technik › Prüfen und Bauen** (`[data-technik=bauen]`): dieselbe Tabelle, „Ausgangslage übernehmen…“ (Dialog „Ausgangslage übernehmen?“), der faltbare Abschnitt „Prüfungen ein- und ausschalten“ mit einem Schalter je Code.

Der Prüflauf: beim Öffnen des Projekts, nach einem Fix, auf „Erneut prüfen“ und 300 ms nach der letzten Änderung (Bus-Thema `edited`, auch nach dem Speichern eines Scripts) läuft jedes Mal genau ein voller Lauf über alle Gruppen (`run_checks { scope: "all" }`); die Caches je Textarchiv und Scriptdatei machen ihn warm schnell (gemessen 250 bis 290 ms auf allen drei Projekten, kalt 1,2 bis 3,8 s). Jeder Bericht ersetzt die Befunde als Ganzes; einer ohne Gruppen oder einer älteren Generation ersetzt nichts, und ein Lauf, während dessen am Store vorbei geschrieben wurde (Script gespeichert, Script angelegt), gilt als veraltet. Tauri und Browser-Modus verhalten sich gleich.

### Vorbereitung

1. Kopien anlegen (nie ein Original; das Ziel darf nicht existieren): `pt-3b4a` für das Szenario (A2), `pt-3b4a-gate` für den Browser-Teil (A3) bis (A6), je eine frische Kopie von `"Pokemon - Platin-Edition (Germany)_DSPRE_contents"`.
2. Werkzeuge: `cargo build --workspace --all-features` (legt `target/debug/studio-bridge` und `target/debug/g4k` an). Aus einem Worktree `G4_ROMS_DIR`, `G4_BRIDGE_BIN` und `G4K_BIN` absolut setzen.
3. Für den Browser-Teil: Test-Umgebung wie in [docs/agent-gate.md](agent-gate.md), „Je Lauf“ (aus einem Worktree mit absolutem `G4_ROMS_DIR`). Seite `http://127.0.0.1:1430/?project=<absoluter Pfad von pt-3b4a-gate>`; vor dem Lauf `localStorage.clear()` und neu laden.

Fakten zu Platin DE, die der Ablauf braucht: Die Treppe des Spielerzimmers (Header 0415) ist Warp 0 der Eventdatei 0394 und führt zu Warp 1 von Header 0414 (Eventdatei 0393 hat 2 Warps). Header 0022 nutzt Eventdatei 0021 (ein Warp, keine Figur) und die geteilte Scriptdatei 0401; eine neue Scriptdatei bekommt die Nummer 1124 und enthält nur Script 1. Objekt 6 der Eventdatei 0390 (Zweiblattdorf, Header 0411) hat Script 9 und Trainer-Typ 0; Platin hat 928 Trainer, Script 3999 kämpft als Trainer 1000. Textarchiv 0558 ist das Dialogarchiv des Spielerzimmers, Nachricht 3 lautet „Der X-Knopf öffnet\ndas Menü.“ Vanilla meldet 111 Befunde (101 Warnungen, 10 Hinweise, 0 Fehler); nach dem Übernehmen der Scripts bleibt es bei 111.

### (A1) Automatik

- [x] 1. `vanilla_checks` gated grün auf Platin DE, HeartGold DE und HeartGold US: `vanilla_checks_match_the_fixed_counts` (jeder Code hat sein Soll aus `SOLL` in `crates/g4-kernel/tests/vanilla_checks.rs`, jede Prüfung mit Schwere Fehler 0 Befunde) und `vanilla_script_checks_after_adopt` (auf einer Kopie: die Slot-Prüfungen finden aus den Quellen dasselbe wie aus den Membern, `script.message-missing` läuft und lässt nur die Common-Scripts aus).
- [x] 2. Zeitmessungen in `crates/g4-kernel/tests/checks_runner.rs`: `overlay_under_lock_takes_under_5_ms_on_hgss` (die Kopie unter dem Lock bleibt unter 5 ms) und `full_run_on_vanilla_under_three_seconds_warm` (voller Lauf über alle Gruppen, der zweite mit warmem Cache unter 3 s).
- [x] 3. `scripts/gate.sh` grün gegen alle drei Projekte (die Punkte 1 und 2 laufen darin mit); Tabelle unter „Ergebnis“.

Einzeln, je Projekt, aus der Repo-Wurzel (Liste der Tests in [dev-setup.md](dev-setup.md), „Projektprüfungen“):

```sh
export PATH="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$PATH"
G4_TEST_PROJECT="<Roms>/Pokemon - Platin-Edition (Germany)_DSPRE_contents" \
  cargo test --workspace --all-features -- vanilla_checks_match_the_fixed_counts vanilla_script_checks_after_adopt \
  full_run_on_vanilla overlay_under_lock --nocapture --test-threads 1
```

### (A2) Szenario `pt-pruefungen.yaml`

Aus `scripts/g4emu`, auf der frischen Kopie `pt-3b4a`:

```sh
G4_ROMS_DIR=<Roms> G4_BRIDGE_BIN=<Worktree>/target/debug/studio-bridge G4K_BIN=<Worktree>/target/debug/g4k \
uv run g4emu scenario ../agent/scenarios/pt-pruefungen.yaml \
  --project <Roms>/_testkopien/pt-3b4a --out <Worktree>/target/agent/gate-<datum>/pt-pruefungen
```

Jede Prüfung im Szenario ist ein `run_checks { scope: all }` mit Erwartungen (`expect`) an die Antwort; eine verfehlte Erwartung beendet das Szenario mit Rückgabe 2 und nennt Pfad, Soll und Ist. Auf stderr steht je erfüllter Erwartung eine Zeile „Erwartung erfüllt: …“.

- [x] 1. Ausgangslage: `adopt_scripts` (1124 Quellen), `accept_baseline` (111 Warnungen und Hinweise ausgeblendet), dann `groups.length = 3`, Fehler 0, Warnungen 0.
- [x] 2. Warp 0 der Eventdatei 0394 auf Ziel-Warp 99: `findings[code=warp.dest-warp].length = 1`, Fehler 1.
- [x] 3. `new_script_file` (1124) als Scriptdatei von Header 0022, eine Figur mit Script 950 in Eventdatei 0021: `findings[code=event.script-missing].length = 1`, Fehler 2.
- [x] 4. Objekt 6 der Eventdatei 0390 mit Script 3999 und Trainer-Typ 1: `findings[code=event.trainer-missing].length = 1`, Fehler 3.
- [x] 5. Nachricht 3 von Textarchiv 0558 als eine Zeile mit 60 Zeichen: `findings[code=text.too-wide].length = 1`, Warnungen 1, Fehler 3.
- [x] 6. Zurück: Ziel-Warp wieder der alte (aus der gebundenen Antwort von `get_events`); `apply_fix create_script` auf den Befund, wie `run_checks` ihn geliefert hat (`remaining = false`, Script 950 in Datei 1124 geschrieben, kein Schritt im Verlauf); Script und Trainer-Typ von Objekt 6 wieder die alten; `apply_fix wrap` (`remaining = false`); danach `groups.length = 3`, Fehler 0, Warnungen 0.
- [x] 7. Gegenproben: `target/debug/g4k check <Kopie> --expect warp.dest-warp=0 --expect event.script-missing=0 --expect event.trainer-missing=0 --expect text.too-wide=0` meldet „4 expectation(s) met“ und „111 ignored“; eine falsche Erwartung (`findings[code=warp.dest-warp].length` = 1 auf der grünen Kopie) beendet ein Szenario mit Rückgabe 2 und „Erwartung verfehlt: run_checks findings[code=warp.dest-warp].length: Soll 1, Ist 0“.

Anders als der Wortlaut des Plans („`update_event` `trainer_type = 0`“) setzt das Szenario beim Zurücknehmen auch das Script von Objekt 6 zurück: `event.trainer-missing` liest das Script (3000 bis 4999 kämpft beim Ansprechen als Trainer, unabhängig vom Trainer-Typ), der Trainer-Typ allein macht den Befund nicht grün.

### (A3) Prüflicht und Panel „Probleme“

Auf `pt-3b4a-gate`, Rezept „Prüfen“ in [docs/agent-gate.md](agent-gate.md).

- [x] 1. Beim Öffnen dreht das Prüflicht (`data-running`, Fortschritt im `aria-label`) und zeigt danach „101 Warnungen, 10 Hinweise“: `data-light="warning"`, Zähler 0 / 101 / 10, die Null gedämpft. Das Panel „Probleme“ zeigt die Bereiche mit Zählern, die Codes als Gruppen und darüber „Nicht geprüft“ mit „Nachricht fehlt“ (Scripts nicht übernommen).
- [x] 2. Nach den vier Fehlern über `call.sh` (wie im Szenario), speichern und Seite neu laden (nur das Neuladen bringt Editoren und Titelleiste auf den Stand der Brücke; „Erneut prüfen“ allein zählt dasselbe): „3 Fehler, 102 Warnungen, 10 Hinweise“, `data-light="error"`; die drei Fehlerzeilen ohne „Ausblenden“, `event.script-missing` und `text.too-wide` mit „Beheben…“, Zeilen ohne Ziel ohne „Öffnen“.
- [x] 3. Automatischer Lauf: eine Änderung in der Oberfläche (Inspector der Treppe, „Ziel-Warp“ von 99 auf 1) lässt das Licht nach 300 ms drehen, danach zählt es einen Fehler weniger; ⌘Z bringt ihn zurück. Ein Bericht einer älteren Generation erscheint nie.
- [x] 4. Technik › Prüfen und Bauen zeigt dieselben Gruppen und Zahlen.

### (A4) Fix-Dialog

Rezept „Beheben“.

- [x] 1. „Beheben…“ an `text.too-wide`: Titel „Textarchiv 0558 · Nachricht 3 · Zeile 1“, die Vorschau „Textarchiv 0558“ mit „Vorher“ (eine Zeile, entfernt markiert) und „Nachher“ (zwei Zeilen); „Umbrechen“ schließt den Dialog, die Zeile ist weg, `call.sh history` endet auf „Behoben: text.too-wide“; ⌘Z bringt den Befund zurück.
- [x] 2. „Beheben…“ an `event.script-missing`: Plakette „Nicht rückgängig“, „Wird sofort geschrieben, ⌘Z nimmt es nicht zurück.“, Vorher „Leer“, Nachher `script script_950 #950:` / `End`. Mit einem ungespeicherten Textarchiv: „Erst speichern“ mit „Schließen“ und „Speichern und anlegen“; „Speichern und anlegen“ speichert das Projekt und behebt den Befund.
- [x] 3. Veralteter Befund: Dialog öffnen, dann die Stelle über `call.sh` ändern, dann die Fix-Taste: „Befund ist nicht mehr aktuell“ (`role=alert`) mit „Schließen“ und „Erneut prüfen“; `call.sh history` (kein neuer Schritt „Behoben: …“) und der unveränderte Text der Nachricht (der zweite `call.sh`-Text) zeigen, dass nichts geschrieben wurde. (`call.sh dirty` taugt dafür nicht: die `call.sh`-Änderungen des Rezepts machen das Archiv selbst ungespeichert.)

### (A5) Ausblenden und Ausgangslage

Rezepte „Ausblenden“ und „Ausgangslage“.

- [x] 1. „Ausblenden“ an einer Warnung oder einem Hinweis: der Zähler sinkt, der Fuß zeigt „1 ausgeblendet · Wieder melden…“, `<Kopie>/g4-checks.json` hat den Eintrag (`code`, `subject`, `key`, `fingerprint`). „Wieder melden…“ klappt `[role=list][aria-label=Ausgeblendet]` auf, „Wieder melden“ holt die Zeile zurück, die Datei ist wieder leer.
- [x] 2. Technik › Prüfen und Bauen › „Ausgangslage übernehmen…“: Dialog „Ausgangslage übernehmen?“ nennt die Zahl („111 Warnungen und Hinweise werden ausgeblendet. …“); „Übernehmen“: der Fuß zeigt „111 ausgeblendet“, das Licht nur noch die Fehler.
- [x] 3. „Prüfungen ein- und ausschalten“: ein Schalter einer Warnung (etwa „Zu viele Zeilen“, `text.three-lines`) aus: ihre Befunde verschwinden, `g4-checks.json` nennt den Code unter `disabled`; wieder an: sie kommen zurück. Schalter von Fehler-Prüfungen sind gesperrt („Fehler lassen sich nicht abschalten.“), auch `matrix.header-elsewhere` (Fehler, Ruling R16).

### (A6) Am Objekt

- [x] 1. „Öffnen“ am `warp.dest-warp`-Befund: die Welt zeigt Header 0415 mit der Treppe gewählt, der Inspector den Abschnitt „Prüfungen 1“ mit dem Befund (ohne „Öffnen“, mit „Beheben…“ nur bei einem Fix).
- [x] 2. Die Explorer-Zeile von Header 0415 trägt `[data-finding-mark=error]` („Fehler gefunden“).
- [ ] 3. Der Warp-Marker der Treppe ist in der Szene rot (`szene/status/fehler`, `#DE3142`); nach dem Beheben wieder in der Warp-Farbe. Der Agent liest die Farbe und beschreibt Ort und Ansicht, der Nutzer bestätigt sie am Bild. (Agent bestanden; Blick beim Nutzer offen.)

### Ergebnis

Lauf des Agenten am 2026-10-25 in der Umsetzung (Worktree des Branches `phase-3b4a-checks`); den Browser-Teil (A3) bis (A6) prüfte die Abnahme am selben Tag auf der frischen Kopie `pt-3b4a-gate` (HEAD 7a51393, Test-Umgebung `studio-web.sh`, Viewport 1440 × 900), Gegenproben mit `call.sh`. Die Browser-Zeilen beschreiben den Stand vor der Fix-Welle; was die Fix-Welle daran änderte, steht unter der Tabelle.

| Punkt | Wer | Datum | Projekt | Ergebnis | Auffälligkeiten |
|---|---|---|---|---|---|
| (A1) 1 `vanilla_checks` gated | Agent | 2026-10-25 | Platin DE, HeartGold DE, HeartGold US (Originale, nur lesend; `vanilla_script_checks_after_adopt` auf `vanilla_copy()`) | bestanden | Befunde je Projekt 111 / 145 / 153, alle Fehler-Codes 0; alle Gruppen auf neuem Snapshot 1047 / 1083 / 1004 ms, die Prüfungen allein 158 / 164 / 135 ms; nach dem Übernehmen (1124 / 965 / 965 Quellen) lässt `script.message-missing` nur die Common-Scripts aus (28 / 25 / 25 Dateien) |
| (A1) 2 Zeitmessungen | Agent | 2026-10-25 | Platin DE, HeartGold DE, HeartGold US | bestanden | voller Lauf kalt 3,46 / 1,18 / 3,60 s, warm 287 / 280 / 247 ms; Kopie unter dem Lock 0,8 bis 3,1 ms (26 Assets, 3 Textdokumente) |
| (A1) 3 `scripts/gate.sh` | Agent | 2026-10-25 | Platin DE, HeartGold DE, HeartGold US | bestanden („Gate: ok (2452 s)“) | auf Commit 2ddd1b0 (`expect`, Szenario), aus dem Worktree mit `G4_ROMS_DIR`, `G4_APICULA` und `G4_ROTOM_LSP` absolut; die gated Tests 646 / 829 / 813 s (HeartGold DE, HeartGold US, Platin DE; in 3b.3b 605 / 662 / 379 s und nach dessen Abnahme 753 / 580 / 643 s, seitdem kamen die Prüf-Tests dazu); Log `target/agent/gate-3b4a.log`. Danach geändert: nur Doku und der Tooltip von „Geschichten“ in `App.tsx`; `pnpm test` (221 Dateien, 2265 Tests), `pnpm build` und `check-styles.sh` danach erneut grün |
| (A2) Szenario `pt-pruefungen.yaml` | Agent | 2026-10-25 | `pt-3b4a` | bestanden | Rückgabe 0, `"ok": true`, 21 Erwartungen erfüllt, 15 Brücken-Sitzungen, 19 s. `accept_baseline` 111 ausgeblendet; die Läufe in frischen Sitzungen (kalt) 1,5 bis 1,6 s, der zweite Lauf einer Sitzung (warm) 257 ms. Befunde der Reihe nach: „Eventdatei 0394 · Warp 0“, „Eventdatei 0021 · Objekt 0“, „Eventdatei 0390 · Objekt 6“, „Textarchiv 0558 · Nachricht 3 · Zeile 1“. `apply_fix create_script` → `changed` Script 1124, `apply_fix wrap` → `changed` Textarchiv 558, je `remaining: false`. Gegenproben (A2) 7 bestanden. Ausgabe `target/agent/gate-2026-10-25/pt-pruefungen/` |
| (A3) 1 Prüflicht beim Öffnen, Panel „Probleme“ | Agent | 2026-10-25 | `pt-3b4a-gate` | bestanden | Beim ersten Öffnen (kalt) zeigte das Licht ab 0,1 s `data-running` mit `aria-label` „Prüft …“, dann „Prüft … Welt und Daten 2/30“ … „Scripts 352/1124“ … „Texte 724/724“; nach 4,6 s „101 Warnungen, 10 Hinweise“, `data-light="warning"`, Zähler `error`=0 (Klasse `zero`, Farbe gedämpft rgb(198,206,222) statt rgb(231,239,247)), `warning`=101, `info`=10; nach 5,1 s ohne `data-running`. Klick aufs Licht öffnet den Reiter „Probleme“: Untertitel „101 Warnungen, 10 Hinweise“, Abschnitte „Welt“ 13 und „Texte“ 98, Gruppen `warp.never-fires` „Tür löst nie aus“ 10, `event.same-spot` 2, `event.script-missing` 1, `text.too-wide` „Zeile zu breit“ 90, `text.three-lines` 8; „Nicht geprüft“: „Nachricht fehlt: nicht geprüft, die Scripts sind nicht übernommen. Übernimm sie unter Script, dann prüft das Studio sie.“ `call.sh checks_status`: 101 Warnungen, 10 Hinweise, `groups` cheap/per_script/per_archive, `skipped` `script.message-missing` „scripts not adopted“. Auf jedes Öffnen folgt ein zweiter voller Lauf (Befund 2) |
| (A3) 2 Vier Fehler, Fehlerzeilen | Agent | 2026-10-25 | `pt-3b4a-gate` | bestanden | Fehler per `call.sh` wie im Rezept (`adopt_scripts` 1124 Quellen, Warp 0 von 0394 → Ziel-Warp 99, `new_script_file` = 1124 an Header 0022, Figur mit Script 950 in 0021, Objekt 6 von 0390 Script 3999 / Trainer-Typ 1, 0558/3 als 60-Zeichen-Zeile), `save`, Seite neu geladen: „3 Fehler, 102 Warnungen, 10 Hinweise“, `data-light="error"` (Lauf 1,9 s nach dem Laden). Zeilen `warp.dest-warp|eventFile:394|warp:0`, `event.script-missing|eventFile:21|object:0`, `event.trainer-missing|eventFile:390|object:6` mit `data-state="error"` und ohne „Ausblenden“ (0 Fehlerzeilen mit „Ausblenden“), `text.too-wide|textArchive:558|message:3:line:1` mit `data-state="warning"` (321 px). „Beheben…“ nur an `event.script-missing` und `text.too-wide` (93 Zeilen). „Nicht geprüft“ jetzt „Nachricht fehlt: 28 Common-Scripts ohne Header nicht geprüft.“ „Erneut prüfen“ ist während des Laufs gesperrt (`disabled`, Fuß `[role=status]` „Prüft … Texte 724/724“). „Zeilen ohne Ziel ohne Öffnen“: nicht prüfbar, alle 115 Befunde haben ein `target`; im Code bedingt (`ProblemsPanel.tsx`, Zeile 105). Gate-Datei und Rezept nennen verschiedene Wege (Befund 3) |
| (A3) 3 Automatischer Lauf | Agent | 2026-10-25 | `pt-3b4a-gate` | bestanden | Header 0415 über die Palette (`0415`), Treppe in „An diesem Ort“ („Warp 0 → Header 0414 (T01R0201)“), Inspector „Ziel-Warp“ 99 → 1, Enter: das Licht dreht etwa 0,4 s danach, nach 0,7 s „2 Fehler, 102 Warnungen, 10 Hinweise“; `call.sh history` „edit event“. ⌘Z (außerhalb des Felds): Lauf, wieder „3 Fehler …“. Ältere Generation: ⇧⌘Z und 0,55 s später ⌘Z, also mitten im Lauf: kein Zwischenstand „2 Fehler“ erschien, beide Läufe endeten auf „3 Fehler“, `checks_status` Generation 4, 3 Fehler |
| (A3) 4 Technik › Prüfen und Bauen | Agent | 2026-10-25 | `pt-3b4a-gate` | bestanden | `[data-technik=bauen]`: Kopf „3 Fehler, 102 Warnungen, 10 Hinweise“, Abschnitte Welt 16, Texte 99, dieselben sieben Gruppen mit denselben Zahlen wie der Reiter (`warp.dest-warp` 1, `warp.never-fires` 10, `event.trainer-missing` 1, `event.same-spot` 2, `event.script-missing` 2, `text.too-wide` 91, `text.three-lines` 8), dieselben drei Fehlerzeilen, „Nicht geprüft“, „Ausgangslage übernehmen…“, „Prüfungen ein- und ausschalten“ (zu). „Erneut prüfen“ hier: gesperrt, solange der Lauf läuft (0,25 s warm) |
| (A4) 1 „Umbrechen“ | Agent | 2026-10-25 | `pt-3b4a-gate` | bestanden | Über den Reiter „Probleme“ in Texte (das „Öffnen“ an derselben Zeile führt richtig zu „Texte › Archiv 0558, Zweiblattdorf › Nachricht 003“). Dialog: Titel „Textarchiv 0558 · Nachricht 3 · Zeile 1“, Beschreibung „Die Zeile ist 321 px breit, das Textfenster nur 216 px. Brich sie um oder kürze den Text.“, `[data-fix=wrap]`, Gruppe „Textarchiv 0558“ mit „Vorher“ (eine Zeile, `data-changed`, Zeichen −) und „Nachher“ („Diese Zeile hat sechzig Zeichen und“ / „passt nicht ins Fenster.“, je +), Tasten „Abbrechen“, „Umbrechen“. „Umbrechen“: Dialog zu, Zeile weg, „3 Fehler, 101 Warnungen“; `history` `["Behoben: text.too-wide"]`, `dirty` TextArchive 558. ⌘Z: Zeile wieder da (102), ⇧⌘Z: wieder weg (101). Die Vorschau bricht in den 215 px breiten Spalten weich um (Befund 5) |
| (A4) 2 „Script anlegen“ | Agent | 2026-10-25 | `pt-3b4a-gate` | bestanden | Dialog „Eventdatei 0021 · Objekt 0“, Satz „Das Event ruft Script 950 auf, aber Scriptdatei 1124 hat nur Scripts bis 1. …“, Plakette „Nicht rückgängig“, „Wird sofort geschrieben, ⌘Z nimmt es nicht zurück.“, Gruppe „Scriptdatei 1124“: Vorher „Leer“, Nachher `script script_950 #950:` / `End`. Mit ungespeichertem 0558: `[role=alert]` „Erst speichern — Textarchiv 0558 hat ungespeicherte Änderungen. Speichere zuerst, dann lege das Script an: es wird kompiliert, und das schreibt Textarchive.“, danach nur „Schließen“. Schließen, „Speichern“ in der Titelleiste („1 Datei(en) gespeichert“), Dialog neu öffnen, „Script anlegen“: Befund weg, „2 Fehler, 101 Warnungen“; `read_script` 1124 enthält `script script_950 #950:`; `history` unverändert (kein Schritt), `dirty` leer. Der Ablauf weicht vom Wortlaut des Rezepts ab (Befund 1) |
| (A4) 3 „Befund ist nicht mehr aktuell“ | Agent | 2026-10-25 | `pt-3b4a-gate` | bestanden | `call.sh edit_text_archive` 0558/2 zu lang, „Erneut prüfen“ im Panel (ohne Neuladen): Zeile `…message:2:line:1` (325 px). Dialog öffnen, 0558/2 per `call.sh` erneut ändern, „Umbrechen“: `[role=alert]` „Befund ist nicht mehr aktuell — Die Stelle hat sich seit der Prüfung geändert, es wurde nichts geschrieben. Prüfe erneut und behebe den Befund dann.“, Tasten „Schließen“ und „Erneut prüfen“. `history` ohne neuen Schritt „Behoben“ (nur die zwei „edit text“ von `call.sh`), Text von 0558/2 unverändert der zweite `call.sh`-Text; „Erneut prüfen“ im Dialog schließt ihn, die Zeile zeigt danach 342 px. `dirty` nennt 0558 wegen der `call.sh`-Änderungen selbst (Befund 4). Die zwei 400 in der Konsole sind diese Ablehnung und „Erst speichern“ |
| (A5) 1 „Ausblenden“, „Wieder melden“ | Agent | 2026-10-25 | `pt-3b4a-gate` | bestanden | „Ausblenden“ an `warp.never-fires|eventFile:4|warp:0`: Hinweise 10 → 9, Gruppe „Tür löst nie aus 9“, Fuß „1 ausgeblendet · Wieder melden…“; `g4-checks.json` `ignored: [{code: "warp.never-fires", subject: {kind: "eventFile", index: 4}, key: "warp:0", fingerprint: "8240e225d464a1da"}]`. „Wieder melden…“ `aria-expanded` false → true, `[role=list][aria-label=Ausgeblendet]` mit der Zeile und „Wieder melden“; das holt sie zurück (wieder 10), `ignored: []`. Fingerprint: ausgeblendet, Warp per `call.sh` auf x 5 verschoben, „Erneut prüfen“: Befund wieder da („Die Tür auf x 5, z 14 …“), Fuß ohne „ausgeblendet“; nach `call.sh undo` und „Erneut prüfen“ wieder „1 ausgeblendet“ (der Eintrag blieb in der Datei). Der aufgeklappte Fuß nimmt in der Hand 173 von 250 px (Befund 6) |
| (A5) 2 „Ausgangslage übernehmen…“ | Agent | 2026-10-25 | `pt-3b4a-gate` | bestanden | Technik › Prüfen und Bauen: Dialog „Ausgangslage übernehmen?“ mit „111 Warnungen und Hinweise werden ausgeblendet. Ändert sich eine Stelle, meldet sie sich wieder; unter „Wieder melden…“ holst du sie zurück.“, „Abbrechen“, „Übernehmen“ (per `ref` geklickt). Danach Kopf und Licht „2 Fehler“ (`data-light="error"`, Zähler 2/0/0), Fuß „111 ausgeblendet · Wieder melden…“, nur noch die Gruppen `warp.dest-warp` und `event.trainer-missing`; `g4-checks.json` 111 Einträge, `checks_status` 111 in `ignored`. Nach dem Neuladen: „2 Fehler“, Fuß „111 ausgeblendet“. (Zahl 111 = 101 Warnungen + 10 Hinweise dieser Kopie zu diesem Zeitpunkt, 0558/3 schon umbrochen) |
| (A5) 3 Prüfungen ein- und ausschalten | Agent | 2026-10-25 | `pt-3b4a-gate` | bestanden | `[role=list][aria-label="Prüfungen ein- und ausschalten"]` mit 39 `li[data-check]` und je einem `[role=switch]`; die 23 Fehler-Prüfungen gesperrt mit „Fehler lassen sich nicht abschalten.“ (auch `event.script-missing` und `trainer.no-party`, die eine Warnungsart haben), 16 schaltbar (darunter `matrix.header-elsewhere`, Befund 7). „Zu viele Zeilen“ aus (Leertaste): Gruppe weg, 101 → 93 Warnungen, Texte 98 → 90, `g4-checks.json` `disabled: ["text.three-lines"]`; wieder an: 8 Zeilen zurück, `disabled: []`. Leertaste auf dem Schalter „Ziel-Warp fehlt“ ändert nichts; `call.sh set_check_enabled` für `warp.dest-warp` lehnt mit 400 ab („reports errors and cannot be switched off“) |
| (A6) 1 „Öffnen“, Inspector „Prüfungen“ | Agent | 2026-10-25 | `pt-3b4a-gate` | bestanden | Aus „Kein Ort gewählt“: „Öffnen“ an `warp.dest-warp` → Brotkrume „Zweiblattdorf › Raum 0415 › Warp 0 → Header 0414 (T01R0201)“, Reiter „Inspector“ aktiv, Abschnitt „Prüfungen 1“ (`aria-expanded=true`), `[role=list][aria-label=Prüfungen]` mit dem Befund, `data-state="error"`, ohne Tasten (kein Fix, kein „Öffnen“). Gegenprobe mit Fix: „Öffnen“ an `event.script-missing|eventFile:44|sign:1` → „Erzelingen › Außen 0045 › Hintergrund-Event 1 · Script 45“, Inspector-Zeile mit „Beheben…“ und „Ausblenden“, ohne „Öffnen“ |
| (A6) 2 Markierung im Explorer | Agent | 2026-10-25 | `pt-3b4a-gate` | bestanden | `[data-header-id="415"]` (Zeile „T01R0202, 1 Karte“) trägt `[data-finding-mark=error]` mit `aria-label` „Fehler gefunden“; ebenso `[data-header-id="411"]` „Zweiblattdorf“ (Befund `event.trainer-missing` in Eventdatei 0390). Im Bild ein rotes Kreuz am Zeilenende |
| (A6) 3 Fehlerfarbe am Warp-Marker | Agent + Blick | 2026-10-25 | `pt-3b4a-gate` | Agent + Blick, Blick beim Nutzer offen | Header 0415 (T01R0202, Spielerzimmer), die Treppe = Warp 0 der Eventdatei 0394 auf Kachel x 8, z 4, oben rechts im Zimmer links neben dem Treppenloch: mit dem Befund rot `#DE3142` (`szene/status/fehler`, Instanzfarbe linear 0,730 / 0,031 / 0,054, per JS über die Three.js-Szene gelesen); nach „Ziel-Warp“ 99 → 1 im Inspector magenta `#AD3173` (`szene/ereignis/warp`), Licht „1 Fehler“; nach ⌘Z wieder rot. Im Agenten-Bild Rot und Magenta klar zu unterscheiden; die Kopie bleibt mit dem Fehler gespeichert (Ablauf für den Blick unten) |

Befunde der Abnahme (Bericht `.superpowers/sdd/2026-10-24-phase-3b4a-checks/gate-a-acceptance.md`) und der Schlussprüfung, in der Fix-Welle behoben (keiner blockierte das Gate):

- Abnahme 1: nach „Erst speichern“ bot der Fix-Dialog nur „Schließen“, und „Speichern“ in der Titelleiste schloss ihn nur. Jetzt „Speichern und anlegen“ im Dialog.
- Abnahme 2: nach jedem Öffnen und jedem Fix lief der volle Lauf zweimal. Jetzt genau einmal (das erste Lesen der ungespeicherten Änderungen nach dem Öffnen und das nach einem Fix senden kein `edited`).
- Abnahme 3 und 4: Wortlaut von (A3) 2 und (A4) 3 (oben angepasst: speichern und neu laden; Beleg ist `history` mit dem unveränderten Text).
- Abnahme 5: die Fix-Vorschau brach in den 215 px breiten Spalten weich um. Jetzt bricht keine Zeile um, die Spalte rollt seitlich.
- Abnahme 6: die aufgeklappte Liste „Ausgeblendet“ nahm 173 von 250 px und blieb nach dem Leeren offen (nach „Ausgangslage übernehmen“ mit allen 111 Zeilen). Jetzt klappt sie beim Leeren und beim Hinzukommen von Einträgen zu, der Fuß nimmt höchstens die halbe Hand, die Liste rollt in ihrer eigenen Höhe.
- Abnahme 7: `matrix.header-elsewhere` war im Kernel eine Warnung und abschaltbar. Jetzt Fehler (Ruling R16), sein Schalter gesperrt (24 gesperrt, 15 schaltbar); Vanilla bleibt 0/0/0.
- Schlussprüfung: der gesperrte Platzhalter-Trainer des „gesehen“-Slots (Platin 929, HGSS 740, von `create_trainer` in 3b.3b) war ein nicht ausblendbarer Fehler `asset.unreadable` und ein `trainer.no-party` (jetzt keins von beiden; gated Beweis `vanilla_checks_skip_the_sight_slot_placeholder`); Script-Speichern, Übernehmen, neue Scripts und Level-Scripts prüften nicht neu und galten im Backend nicht als Änderung (jetzt `edited` und eine Epoche der Platte); `checks_status` lief auf dem Hauptthread (jetzt nicht); eine kaputte `g4-checks.json` hielt das Projekt zu (jetzt öffnet es, Ruling R15); Befundzeilen behielten alte Zahlen und zeigten „Beheben…“ nach dem Übernehmen der Scripts nicht (jetzt Vergleich des ganzen Inhalts); vor dem ersten Bericht sagte das Licht „Alles stimmig“ (jetzt „Nicht geprüft“); ein Filter `[schlüssel=…]` mit vertipptem Schlüssel fand in Szenarien still nichts (jetzt fehlender Pfad); der letzte Schritt von `pt-pruefungen.yaml` prüft auch Hinweise 0 (22 Erwartungen).

Für den Blick des Nutzers, (A6) 3: Die Kopie `_testkopien/pt-3b4a-gate` ist gespeichert mit dem Fehler (Warp 0 der Eventdatei 0394 → Ziel-Warp 99) und mit der Ausgangslage (Licht „2 Fehler“). Öffnen über die Test-Umgebung (`http://127.0.0.1:1430/?project=<…>/_testkopien/pt-3b4a-gate`) oder die gebündelte App; Welt › Orte › Zweiblattdorf › „T01R0202“ (Header 0415), Reiter „3D“, mit dem Mausrad heranzoomen. Der Warp-Marker ist die durchscheinende Säule auf Kachel x 8, z 4, oben rechts im Zimmer, links neben dem Treppenloch, zwei Kacheln links von der Figur „Objekt 0“. Erwartet: rot (`#DE3142`). Dann die Treppe wählen („An diesem Ort“ › „Warp 0 → Header 0414“), im Inspector „Ziel-Warp“ 99 → 1, Enter: die Säule wird magenta (`#AD3173`, die Warp-Farbe), das Licht „1 Fehler“; ⌘Z macht sie wieder rot.

Ergebnis von `scripts/gate.sh` (2026-10-25, Worktree des Branches, `G4_ROMS_DIR` absolut, Log `target/agent/gate-3b4a.log`):

```
Step                                                 Project                                      Result                     Seconds
----                                                 -------                                      ------                     -------
cargo fmt --check                                    -                                            ok                               0
cargo clippy (-D warnings)                           -                                            ok                               2
cargo test (no ROM data)                             -                                            ok                              40
pnpm test                                            -                                            ok                              79
tokens                                               -                                            ok                               0
styles                                               -                                            ok                               2
pnpm build                                           -                                            ok                              26
dist has no bridge                                   -                                            ok                               1
g4emu unit                                           -                                            ok                               7
g4emu integration                                    -                                            ok                               7
gated tests (cargo test)                             Pokemon - Goldene Edition HeartGold (Germany) ok                             646
gated tests (cargo test)                             Pokemon - HeartGold Version (USA)            ok                             829
gated tests (cargo test)                             Pokemon - Platin-Edition (Germany)           ok                             813

Gate: ok (2452 s)
```

Testkopien von Teil A (unter `_testkopien`, der Nutzer räumt sie auf): `pt-3b4a` (Szenario, gespeichert: Scripts übernommen, Ausgangslage in `g4-checks.json`, Scriptdatei 1124 an Header 0022 mit Script 950, Figur in Eventdatei 0021, Nachricht 0558/3 umbrochen), `pt-3b4a-probe` und `pt-3b4a-probe2` (Erkundung und erster Szenario-Lauf der Umsetzung), `pt-3b4a-ui` (Browser-Prüfung in Task 6), `pt-3b4a-gate` (Abnahme des Browser-Teils, gespeichert mit dem Fehler an der Treppe für den Blick), `pt-3b4a-fix` (Szenario nach der Fix-Welle).

### Bekannte Grenzen

- Nur Warps bekommen die Fehlerfarbe in der Szene (Spezifikation §8); Figuren, Auslöser und Schilder nicht.
- Markierungen stehen in den Listen, die Befunde haben können: Explorer (Header, auch die Befunde ihrer Eventdatei), Trainer und Begegnungen; Pokédex, Attacken und Gegenstände haben keine Prüfung.
- Die Vorschau von „Umbrechen“ zeigt Vorher und Nachher als Text (`BeforeAfter`), nicht als DS-Bildschirm (Ruling R11).
- `create_script` legt Script n in der Scriptdatei des ersten Headers an, der die Eventdatei nutzt; haben mehrere Header verschiedene Scriptdateien, bleiben die anderen ohne das Script (der Befund verschwindet trotzdem, die Prüfung verlangt das Script nur in einer der Dateien).

## Teil B: Gate 3b.4b

Folgt mit Plan 3b.4b, Task 4 ([docs/superpowers/plans/2026-10-24-phase-3b4b-import.md](superpowers/plans/2026-10-24-phase-3b4b-import.md)): das 3b-Gesamtgate mit den Szenarien `pt-kleiner-hack.yaml` und `hg-kleiner-hack.yaml` (Entwürfe liegen im Branch; sie nutzen `expect` aus Teil A), dem Dialog „Teile importieren…“ über die Test-Umgebung und der importierten Karte im Spiel.
