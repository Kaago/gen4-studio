# Phase 3b.4: Gate prüfen

Stand: 2026-10-25. Teil A (Gate 3b.4a „Prüfen und Beheben“): implementiert; die Automatik (gated Tests, `scripts/gate.sh`) und der Szenario-Teil sind vom Agenten gelaufen und grün (Ergebnis unten). Offen ist der Browser-Teil (A3) bis (A6), den die Abnahme über die Test-Umgebung prüft. Teil B (Gate 3b.4b „Import und Gesamtgate“, zugleich das 3b-Gesamtgate) folgt mit Plan 3b.4b, Task 4.

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

- **Prüflicht** im Scharnier: `button[data-light]` mit `data-light` `error`, `warning`, `info` oder `sound`; `aria-label` wie die Statusleiste („3 Fehler, 102 Warnungen, 10 Hinweise“, „Alles stimmig“; während eines Laufs mit dem Fortschritt davor, „Prüft … Texte 312/724, …“) und `data-running`, solange ein Lauf läuft; die Zähler `[data-count=error|warning|info]` tragen die Zahl. Ein Klick öffnet „Probleme“.
- **Panel „Probleme“** (`section[aria-label=Probleme]`): Bereiche Welt, Scripts, Texte, Pokémon, Projekt als faltbare Abschnitte mit Zähler, darin je Code eine Gruppe `[role=group][data-check=<code>]` mit Titel und Zahl; Zeilen `li[data-finding="<code>|<art>:<index>|<schlüssel>"][data-check][data-state=error|warning|info]` mit Satz, Ort („Eventdatei 0394 · Warp 0“) und den Tasten „Öffnen“ (nur mit Ziel), „Beheben…“ (nur mit Fix), „Ausblenden“ (nie bei Fehlern). Darüber `[role=list][aria-label="Nicht geprüft"]`, darunter der Fuß mit `[role=status]` (Fortschritt), „N ausgeblendet“ und „Wieder melden…“, „Erneut prüfen“.
- **Fix-Dialog** `[role=dialog]`: Titel = Ort des Befunds, Beschreibung = Satz, Körper `[data-fix=<id>]` mit je geändertem Asset `[role=group][aria-label="Textarchiv 0558"]` und darin „Vorher“ und „Nachher“ (`[data-line]`, geänderte Zeilen `[data-changed]`); Plakette „Nicht rückgängig“ bei `create_script`; Tasten „Abbrechen“ und die Fix-Taste („Umbrechen“, „Nachricht anlegen“, „Tauschen“, „Script anlegen“).
- **Technik › Prüfen und Bauen** (`[data-technik=bauen]`): dieselbe Tabelle, „Ausgangslage übernehmen…“ (Dialog „Ausgangslage übernehmen?“), der faltbare Abschnitt „Prüfungen ein- und ausschalten“ mit einem Schalter je Code.

Der Prüflauf: beim Öffnen des Projekts, auf „Erneut prüfen“ und 300 ms nach der letzten Änderung (Bus-Thema `edited`) läuft jedes Mal der volle Lauf über alle Gruppen (`run_checks { scope: "all" }`); die Caches je Textarchiv und Scriptdatei machen ihn warm schnell (gemessen 250 bis 290 ms auf allen drei Projekten, kalt 1,2 bis 3,8 s). Jeder Bericht ersetzt die Befunde als Ganzes; einer ohne Gruppen oder einer älteren Generation ersetzt nichts. Tauri und Browser-Modus verhalten sich gleich.

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

- [ ] 1. Beim Öffnen dreht das Prüflicht (`data-running`, Fortschritt im `aria-label`) und zeigt danach „101 Warnungen, 10 Hinweise“: `data-light="warning"`, Zähler 0 / 101 / 10, die Null gedämpft. Das Panel „Probleme“ zeigt die Bereiche mit Zählern, die Codes als Gruppen und darüber „Nicht geprüft“ mit „Nachricht fehlt“ (Scripts nicht übernommen).
- [ ] 2. Nach den vier Fehlern über `call.sh` (wie im Szenario) und „Erneut prüfen“: „3 Fehler, 102 Warnungen, 10 Hinweise“, `data-light="error"`; die drei Fehlerzeilen ohne „Ausblenden“, `event.script-missing` und `text.too-wide` mit „Beheben…“, Zeilen ohne Ziel ohne „Öffnen“.
- [ ] 3. Automatischer Lauf: eine Änderung in der Oberfläche (Inspector der Treppe, „Ziel-Warp“ von 99 auf 1) lässt das Licht nach 300 ms drehen, danach zählt es einen Fehler weniger; ⌘Z bringt ihn zurück. Ein Bericht einer älteren Generation erscheint nie.
- [ ] 4. Technik › Prüfen und Bauen zeigt dieselben Gruppen und Zahlen.

### (A4) Fix-Dialog

Rezept „Beheben“.

- [ ] 1. „Beheben…“ an `text.too-wide`: Titel „Textarchiv 0558 · Nachricht 3 · Zeile 1“, die Vorschau „Textarchiv 0558“ mit „Vorher“ (eine Zeile, entfernt markiert) und „Nachher“ (zwei Zeilen); „Umbrechen“ schließt den Dialog, die Zeile ist weg, `call.sh history` endet auf „Behoben: text.too-wide“; ⌘Z bringt den Befund zurück.
- [ ] 2. „Beheben…“ an `event.script-missing`: Plakette „Nicht rückgängig“, „Wird sofort geschrieben, ⌘Z nimmt es nicht zurück.“, Vorher „Leer“, Nachher `script script_950 #950:` / `End`. Mit einem ungespeicherten Textarchiv: „Erst speichern“; nach „Speichern“: „Script anlegen“ behebt den Befund.
- [ ] 3. Veralteter Befund: Dialog öffnen, dann die Stelle über `call.sh` ändern, dann die Fix-Taste: „Befund ist nicht mehr aktuell“ (`role=alert`) mit „Schließen“ und „Erneut prüfen“; `call.sh dirty` und `history` zeigen, dass nichts geschrieben wurde.

### (A5) Ausblenden und Ausgangslage

Rezepte „Ausblenden“ und „Ausgangslage“.

- [ ] 1. „Ausblenden“ an einer Warnung oder einem Hinweis: der Zähler sinkt, der Fuß zeigt „1 ausgeblendet · Wieder melden…“, `<Kopie>/g4-checks.json` hat den Eintrag (`code`, `subject`, `key`, `fingerprint`). „Wieder melden…“ klappt `[role=list][aria-label=Ausgeblendet]` auf, „Wieder melden“ holt die Zeile zurück, die Datei ist wieder leer.
- [ ] 2. Technik › Prüfen und Bauen › „Ausgangslage übernehmen…“: Dialog „Ausgangslage übernehmen?“ nennt die Zahl („111 Warnungen und Hinweise werden ausgeblendet. …“); „Übernehmen“: der Fuß zeigt „111 ausgeblendet“, das Licht nur noch die Fehler.
- [ ] 3. „Prüfungen ein- und ausschalten“: ein Schalter einer Warnung (etwa „Zu viele Zeilen“, `text.three-lines`) aus: ihre Befunde verschwinden, `g4-checks.json` nennt den Code unter `disabled`; wieder an: sie kommen zurück. Schalter von Fehler-Prüfungen sind gesperrt („Fehler lassen sich nicht abschalten.“).

### (A6) Am Objekt

- [ ] 1. „Öffnen“ am `warp.dest-warp`-Befund: die Welt zeigt Header 0415 mit der Treppe gewählt, der Inspector den Abschnitt „Prüfungen 1“ mit dem Befund (ohne „Öffnen“, mit „Beheben…“ nur bei einem Fix).
- [ ] 2. Die Explorer-Zeile von Header 0415 trägt `[data-finding-mark=error]` („Fehler gefunden“).
- [ ] 3. Der Warp-Marker der Treppe ist in der Szene rot (`szene/status/fehler`, `#DE3142`); nach dem Beheben wieder in der Warp-Farbe. Der Agent liest die Farbe und beschreibt Ort und Ansicht, der Nutzer bestätigt sie am Bild.

### Ergebnis

Lauf des Agenten am 2026-10-25 in der Umsetzung (Worktree des Branches `phase-3b4a-checks`); den Browser-Teil prüft die Abnahme.

| Punkt | Wer | Datum | Projekt | Ergebnis | Auffälligkeiten |
|---|---|---|---|---|---|
| (A1) 1 `vanilla_checks` gated | Agent | 2026-10-25 | Platin DE, HeartGold DE, HeartGold US (Originale, nur lesend; `vanilla_script_checks_after_adopt` auf `vanilla_copy()`) | bestanden | Befunde je Projekt 111 / 145 / 153, alle Fehler-Codes 0; alle Gruppen auf neuem Snapshot 1047 / 1083 / 1004 ms, die Prüfungen allein 158 / 164 / 135 ms; nach dem Übernehmen (1124 / 965 / 965 Quellen) lässt `script.message-missing` nur die Common-Scripts aus (28 / 25 / 25 Dateien) |
| (A1) 2 Zeitmessungen | Agent | 2026-10-25 | Platin DE, HeartGold DE, HeartGold US | bestanden | voller Lauf kalt 3,46 / 1,18 / 3,60 s, warm 287 / 280 / 247 ms; Kopie unter dem Lock 0,8 bis 3,1 ms (26 Assets, 3 Textdokumente) |
| (A1) 3 `scripts/gate.sh` | Agent | 2026-10-25 | Platin DE, HeartGold DE, HeartGold US | bestanden („Gate: ok (2452 s)“) | auf Commit 2ddd1b0 (`expect`, Szenario), aus dem Worktree mit `G4_ROMS_DIR`, `G4_APICULA` und `G4_ROTOM_LSP` absolut; die gated Tests 646 / 829 / 813 s (HeartGold DE, HeartGold US, Platin DE; in 3b.3b 605 / 662 / 379 s und nach dessen Abnahme 753 / 580 / 643 s, seitdem kamen die Prüf-Tests dazu); Log `target/agent/gate-3b4a.log`. Danach geändert: nur Doku und der Tooltip von „Geschichten“ in `App.tsx`; `pnpm test` (221 Dateien, 2265 Tests), `pnpm build` und `check-styles.sh` danach erneut grün |
| (A2) Szenario `pt-pruefungen.yaml` | Agent | 2026-10-25 | `pt-3b4a` | bestanden | Rückgabe 0, `"ok": true`, 21 Erwartungen erfüllt, 15 Brücken-Sitzungen, 19 s. `accept_baseline` 111 ausgeblendet; die Läufe in frischen Sitzungen (kalt) 1,5 bis 1,6 s, der zweite Lauf einer Sitzung (warm) 257 ms. Befunde der Reihe nach: „Eventdatei 0394 · Warp 0“, „Eventdatei 0021 · Objekt 0“, „Eventdatei 0390 · Objekt 6“, „Textarchiv 0558 · Nachricht 3 · Zeile 1“. `apply_fix create_script` → `changed` Script 1124, `apply_fix wrap` → `changed` Textarchiv 558, je `remaining: false`. Gegenproben (A2) 7 bestanden. Ausgabe `target/agent/gate-2026-10-25/pt-pruefungen/` |
| (A3) Prüflicht und Panel | Agent | – | `pt-3b4a-gate` | offen (Abnahme) | |
| (A4) Fix-Dialog | Agent | – | `pt-3b4a-gate` | offen (Abnahme) | |
| (A5) Ausblenden und Ausgangslage | Agent | – | `pt-3b4a-gate` | offen (Abnahme) | |
| (A6) 1–2 Inspector und Explorer | Agent | – | `pt-3b4a-gate` | offen (Abnahme) | |
| (A6) 3 Fehlerfarbe am Warp-Marker | Agent + Blick | – | `pt-3b4a-gate` | offen (Abnahme) | in der Umsetzung (Task 6) per JS gelesen: Instanzfarbe linear (0,73, 0,031, 0,054) = `#DE3142` |

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

Testkopien von Teil A (unter `_testkopien`, der Nutzer räumt sie auf): `pt-3b4a` (Szenario, gespeichert: Scripts übernommen, Ausgangslage in `g4-checks.json`, Scriptdatei 1124 an Header 0022 mit Script 950, Figur in Eventdatei 0021, Nachricht 0558/3 umbrochen), `pt-3b4a-probe` und `pt-3b4a-probe2` (Erkundung und erster Szenario-Lauf der Umsetzung), `pt-3b4a-ui` (Browser-Prüfung in Task 6).

### Bekannte Grenzen

- Nur Warps bekommen die Fehlerfarbe in der Szene (Spezifikation §8); Figuren, Auslöser und Schilder nicht.
- Markierungen stehen in den Listen, die Befunde haben können: Explorer (Header, auch die Befunde ihrer Eventdatei), Trainer und Begegnungen; Pokédex, Attacken und Gegenstände haben keine Prüfung.
- Die Vorschau von „Umbrechen“ zeigt Vorher und Nachher als Text (`BeforeAfter`), nicht als DS-Bildschirm (Ruling R11).
- `create_script` legt Script n in der Scriptdatei des ersten Headers an, der die Eventdatei nutzt; haben mehrere Header verschiedene Scriptdateien, bleiben die anderen ohne das Script (der Befund verschwindet trotzdem, die Prüfung verlangt das Script nur in einer der Dateien).

## Teil B: Gate 3b.4b

Folgt mit Plan 3b.4b, Task 4 ([docs/superpowers/plans/2026-10-24-phase-3b4b-import.md](superpowers/plans/2026-10-24-phase-3b4b-import.md)): das 3b-Gesamtgate mit den Szenarien `pt-kleiner-hack.yaml` und `hg-kleiner-hack.yaml` (Entwürfe liegen im Branch; sie nutzen `expect` aus Teil A), dem Dialog „Teile importieren…“ über die Test-Umgebung und der importierten Karte im Spiel.
