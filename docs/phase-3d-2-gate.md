# Phase 3d.2: Gate prüfen

Stand: 2026-10-31. Branch `phase-3d-2-erzaehlen-rahmen` (Worktree `.claude/worktrees/phase-3d-2`, Basis main `3c194c7`), nicht nach main gemergt. Gate vom Agenten auf dem Stand `bb12f49` (alle Tasks, abschließendes Review und seine Fix-Welle) am 2026-10-31 zwischen 20:19 und 21:28 abgearbeitet: (1) die Schritte von `scripts/gate.sh`, die 3d.2 berührt, einzeln grün, gated Tests im Release-Profil auf Platin DE, HeartGold DE und HeartGold US grün; (2) Szenario kopflos über `studio mcp` auf allen drei Spielen bestanden, die drei Szenarien `pt-raum-assistent`, `pt-leere-region`, `hg-leere-region` grün; (3) Browser-Teil auf Platin DE und HeartGold DE und MCP über die Brücke bestanden, mit zwei Befunden: U1 (der Inspector in Erzählen zeigt bei Figur und Schild noch die Zeile „y“ mit dem Rohwert der Höhe und bei allen Events „Hinzufügen“ mit dem Hinweis zum Ziehen) und U2 (Blick: die Rezept-Leiste bricht in zwei Zeilen um und verdeckt im Assistenten den Entwurf in der Schau). Beide sind offen (Abschnitt „Befunde“).

(1) 1. `scripts/gate.sh` als Ganzes: offen, läuft vom Hauptorchestrator auf main nach dem Merge (Entscheidung des Steuer-Agenten: unter paralleler Last scheitern seine Zeitgrenzen; die Schritte, die 3d.2 berührt, liefen hier einzeln).

Beim Nutzer offen: die drei Tiefen in der gepackten App, die Rezept-Leiste mit Tastatur (Hand), der Blick auf Entwurf, Inspector in Erzählen, Rezept-Leiste und Ergebnisse (Abschnitte „Hand“ und „Blick“), dazu die Entscheidung über U1 und U2.

Phase 3d.2 bringt den Rahmen der Tiefe Erzählen: `Depth` hat den dritten Wert `"erzaehlen"`, die Helfer `depthAtLeast` und `viewDepth` ersetzen jeden direkten Vergleich, `scripts/check-depth.sh` hält es so (Schritt „depth“ im Gate-Skript). In Erzählen zeigt die Welt statt der Werkzeugleiste die Rezept-Leiste und im Inspector nur die Felder mit `from: "erzaehlen"` (Picker statt Zahlen); die übrigen Bereiche zeigen in Erzählen, was sie in Bauen zeigen. Im Kernel liegen die Rezepte Gebiet anlegen, Raum anlegen, Gebiet erweitern und Welt leeren als Modul `recipes` mit `plan` (schreibt nichts, liefert einen Entwurf) und `apply` (ein Undo-Schritt; bei „Welt leeren“ sind Start-Script, Level-Script-Datei, Textarchive und `compile-state.json` Dateischritte, ⌘Z stellt sie byte-gleich wieder her); MCP hat `recipe_list`, `recipe_plan`, `recipe_apply`, `g4k` den Befehl `recipe`. Die Assistenten zeigen den Entwurf vor dem Anlegen in der Schau (gestrichelte Rahmen, halbtransparente Marker), auf Weltkarte und Raumplan; das Ergebnis nennt den nächsten Schritt als Rezept-Taste. Roadmap: [docs/ROADMAP.md](ROADMAP.md), Abschnitt „Phase 3d“, Zeile „3d.2 Erzählen-Rahmen“. Spezifikation: [2026-10-30-phase-3d-erzaehlen-design.md](superpowers/specs/2026-10-30-phase-3d-erzaehlen-design.md) §3.3, §4. Plan: [2026-10-30-phase-3d-2-erzaehlen-rahmen.md](superpowers/plans/2026-10-30-phase-3d-2-erzaehlen-rahmen.md). Werkzeuge: [mcp.md](mcp.md) (`recipe_list`, `recipe_plan`, `recipe_apply`).

Gate-Text der Spezifikation (§4, Zeile 3d.2):

> In Erzählen zeigt der Inspector einer Figur genau die Felder mit `from: "erzaehlen"` (Snapshot der Feldlisten je Tiefe und Art); Bereiche ohne Erzählen-Ansicht gleichen Bauen; der Lint findet keinen direkten Vergleich; „Raum anlegen“ zeigt vor dem Anlegen `[data-draft]` mit Tür und gestrichelten Zellen (Zahl über die Test-API gleich `recipe_plan`), Esc entfernt sie und `git status` bleibt leer; `recipe_apply` über MCP ist ein Undo-Schritt, ⌘Z stellt alle Dateien wieder her, auch Scripts nach „Welt leeren“; `pt-raum-assistent`, `pt-leere-region`, `hg-leere-region` unverändert grün

Wer was prüft: Die Spalte „Wer“ sagt **Agent** (die Test-Umgebung aus [docs/agent-gate.md](agent-gate.md): Browser-Fenster mit `studio-bridge`, `call.sh`, `mcp.sh`, dazu `g4k`, `studio mcp` kopflos, `g4emu` und die gated Tests), **Agent + Blick** (der Agent prüft und beschreibt Ort und Zahlen, der Nutzer bestätigt kurz am Bild) oder **Hand** (der Nutzer). Befehle: [docs/agent-gate.md](agent-gate.md), Abschnitt „Phase 3d.2“.

Namen in der Oberfläche (Rollen und `data-*` für die Prüfung):

- **Segment „Tiefe“** im Scharnier: `[role=group][aria-label=Tiefe]` mit den Tasten „Erzählen“ (Symbol `narrate`), „Bauen“, „Tüfteln“, die gewählte mit `aria-pressed="true"`. Das Segment zeigt den gespeicherten Wert, auch in Bereichen, die in Erzählen Bauen zeigen.
- **Rezept-Leiste** (Erzählen, Welt, 3D vorn): `[role=toolbar][aria-label=Rezepte]` mit `data-anchor` wie die Werkzeugleiste; Tasten `[data-recipe=create_area]` „Gebiet anlegen“, `[data-recipe=create_room]` „Raum anlegen“, gesperrt (`disabled`, Schloss, Tooltip mit Name und „Kommt mit 3d.5“) `new_figure` „Neue Figur“, `connect_doors` „Tür verbinden“, `new_trainer_here` „Neuer Trainer hier“.
- **Inspector in Erzählen:** Figur „Aussehen“ (Sprite, „Ändern…“), „Bewegung“ (Wörter aus `MOVEMENT_WORDS`, etwa „Geht umher“, „Schaut sich um“), „Blickrichtung“ („Links“ …), „Trainer“ („kein Trainer“, „Ändern…“); Tür „Ziel“ (Referenz, etwa „Zweiblattdorf 0412 · T01R0101“, „Ändern…“) und „Tür“ („Tür 0 · x 6, z 10“, „Ändern…“); Schild die Info „Der Text des Schilds kommt mit 3d.5.“, Auslöser „Geschichte und Schritt kommen mit 3d.7.“; Gebiet und Raum ein Abschnitt „Ort“ mit „Name“ (beim Raum mit „Umbenennen…“), „Ort“, „Aussehen“ („Aussehen wie Zweiblattdorf“), „Musik (Tag)“, „Musik (Nacht)“.
- **Entwurf:** `[data-draft][data-draft-count]` am Host der 3D-Ansicht (Zahl der gezeichneten Zellen, Kacheln und Marker auf der gezeigten Weltkarte; Text- und Zahlfelder tragen ebenfalls `data-draft`, darum immer mit `[data-draft-count]` abfragen), `[data-draft-cell="x,y"]` auf Weltkarte und Raumplan (`aria-label` „Geplant: Karte bei x 14, y 0“), Sätze der Zusammenfassung `[data-draft-notes]` nur in Tüfteln.
- **Ergebnis-Tasten** unter „Als Nächstes“: nach „Raum anlegen“ `[data-recipe=new_figure]` „Neue Figur hier“ gesperrt mit „Kommt mit 3d.5“ (`aria-describedby`), nach „Welt leeren“ `[data-recipe=create_area]` „Gebiet anlegen“ frei (öffnet den Assistenten „Gebiet anlegen“).
- **Agentenfrage** `[data-agent-question]`: „Agent möchte etwas Unumkehrbares tun“, „„agent-gate“ möchte: Welt leeren (lässt sich nicht rückgängig machen).“, „Ablehnen“ / „Zulassen“.

## Vorbereitung

So durchgeführt am 2026-10-31 auf `bb12f49` ((1) 20:19 bis 21:18, (2) 21:19 bis 21:28, (3) 20:34 bis 21:05):

1. **Frische Kopien** unter `Roms/_testkopien/` per `cp -cR` (Ordner) und `cp -c` (`.nds`) aus den Originalen, die dabei nur gelesen wurden; Ordner `3d-2-<zweck>_DSPRE_contents` und `.nds` `3d-2-<zweck>.nds` nebeneinander:
   - `3d-2-pt` (Platin DE), `3d-2-hgde` (HeartGold DE), `3d-2-hgus` (HeartGold US) für (1); nur gelesen, die Tests schreiben auf `vanilla_copy()`;
   - `3d-2-kpt`, `3d-2-khgde`, `3d-2-khgus` für (2), je mit `g4k git init` (Commit „Ausgangsstand“, 18 449 / 19 989 / 19 989 Dateien); `3d-2-kpt` wurde nach einem ersten, am Skript gescheiterten Lauf (`g4_call` braucht kopflos `--expert`, siehe „Hinweise“) frisch kopiert;
   - `3d-2-raum` (Platin DE) für `pt-raum-assistent.yaml`; `3d-2-leer-pt.nds` und `3d-2-leer-hg.nds`, daraus mit `g4k create` die Projekte `3d-2-leer-pt` und `3d-2-leer-hg` für `pt-leere-region.yaml` und `hg-leere-region.yaml`;
   - `3d-2-web` (Platin DE), `3d-2-webhg` (HeartGold DE) und `3d-2-mcp` (Platin DE) für (3), je mit `g4k git init`.

   Alle `3d-2-*`-Kopien sind nach dem Gate gelöscht; `Manual`, `_saves` und die Kopien der parallelen Phasen bleiben.
2. **Werkzeuge:** `cargo build --release -p g4-kernel --bin g4k`, `cargo build --release -p studio --bin studio` und die Release-Testbinaries (`--test recipes --test templates --no-run`), 20:27 bis 20:32 (Log `build-release.log`). `tools/` ist der Symlink auf den Haupt-Checkout (nie einchecken). `G4_ROMS_DIR=/Users/POLTEP/Projekte/lab/editor/Roms` absolut.
3. **Test-Umgebung** für (3): `G4_ROMS_DIR=<Roms> G4_VITE_PORT=1710 G4_BRIDGE_PORT=1711 bash scripts/agent/studio-web.sh` (Bau der Brücke 1 min 04 s), Chrome 156 headless (`--headless=new`, CDP auf einem eigenen Port, `--use-angle=swiftshader --enable-unsafe-swiftshader`: WebGL 2 ist da, die 3D-Ansicht und die Entwurf-Schicht zeichnen), 1440 × 900; Hell und Dunkel über `prefers-color-scheme`, Kompakt über die Taste der Statusleiste. Ein kleines Node-Skript im Scratchpad sprach CDP (Auswerten, Klicks mit der Maus, Tasten, Bilder); ein Mitschnitt von `window.fetch` merkte die Körper der Anfragen an `/__bridge/invoke/recipe_*`.
4. **Ausgaben** unter `target/agent/gate-2026-10-31/3d-2/` im Worktree: `auto.log` und `auto-<schritt>.log` (1, Debug-Teil), `gated-{recipes,templates}-<k>.log` (1), `kopflos-<k>.{log,json,out}` (2), `scen-raum/`, `scen-leer-{pt,hg}/` mit ihren Logs (2), `web-*.json`/`.txt`, `mcp-bridge-*` und die Bilder `blick-*.png` (3). Die Skripte (CDP, MCP über stdio, Ablauf kopflos) lagen im Scratchpad. Den Ordner vor `git worktree remove` sichern.
5. **Last:** `uptime` vor und nach jedem Lauf; drei andere Orchestratoren teilten die Maschine. (1) begann bei einer 1-Minuten-Last von 3,5, lief dann bei 20 bis 36; (2) bei 11 bis 18; (3) bei 20 bis 36. Alle Zeiten sind darum **unter Last**. Release-Bauten einzeln, die drei Spiele in (1) und (2) nacheinander, nie parallel.

## (1) Automatik

- [ ] 1. **`scripts/gate.sh` grün** (mit dem Schritt „depth“): offen, vom Hauptorchestrator auf main nach dem Merge.
- [x] 2. **Schritte von `scripts/gate.sh`, die 3d.2 berührt**, einzeln aus dem Worktree (Log `auto.log`), alle im ersten Versuch grün:

  | Schritt | Ergebnis | Zeit | Last (1 min) |
  |---|---|---|---|
  | `cargo fmt --check` | grün | 1 s | 3,5 |
  | `cargo clippy --workspace --all-targets --all-features -- -D warnings` | grün | 2 s (gebaut) | 3,5 |
  | `cargo test -p g4-kernel --all-features` (ohne `G4_*`) | 37 Testbinaries, 1 442 bestanden, 0 gescheitert | 2 359 s | 3,5 → 20,1 |
  | `cargo test -p studio --all-features --lib` (ohne `G4_*`) | 651 bestanden, 1 ignoriert | 357 s | 20,1 → 28,3 |
  | `cargo test -p studio --test mcp_stdio` (ohne `G4_*`) | 28 bestanden | 61 s | 28,3 → 20,9 |
  | `bash scripts/check-tokens.sh` | „Alles stimmig.“ | 0 s | 20,9 |
  | `bash scripts/check-styles.sh` | `check-styles: ok` | 9 s | 20,6 |
  | `bash scripts/check-depth.sh` | `check-depth: ok`, 0 Treffer | 5 s | 19,8 |
  | `pnpm test` | 336 Dateien bestanden (1 übersprungen), 3 426 Tests bestanden (1 übersprungen) | 380 s | 19,8 → 20,4 |
  | `pnpm build` | grün (`tsc`, Vite in 3,65 s) | 60 s | 21,6 |

  Nicht einzeln gelaufen (3d.2 berührt sie nicht): `cargo deny`, „dist has no bridge“, `g4emu` Unit und Integration, gated Tests der übrigen Phasen, Projektstart, Zeiten.
- [x] 3. **Snapshot** `apps/studio/src/workbench/inspector/inspectorFields.test.ts` (Feldlisten je Tiefe und Art): 5/5 bestanden (Log `auto-snapshot.log`).
- [x] 4. **`bash scripts/check-depth.sh`:** 0 Treffer (oben).
- [x] 5. **Gated Tests** im Release-Profil, `--nocapture --test-threads 1`, je Projekt nacheinander (Log `seq-gated.log`):

  | Spiel | `--test recipes` | `--test templates` | berührte / gesicherte Textarchive | `git status` nach ⌘Z | Last |
  |---|---|---|---|---|---|
  | Platin DE | 11/11 in 162 s; `vanilla_room_recipe_matches_the_wizard_scenario` grün | 27/27 in 1 044 s | [554] / [554], keines ungesichert | `["?? unpacked/maps/0666"]` (auch nach Build und ⌘Z) | 35,0 → 22,3 |
  | HeartGold DE | 11/11 in 211 s; der Raum-Test meldet `skipped: the scenario is Platinum DE's` | 27/27 in 808 s | [542] / [542] | `["?? unpacked/maps/0676"]` | 22,3 → 21,2 |
  | HeartGold US | 11/11 in 117 s; Raum-Test `skipped` | 27/27 in 352 s | [542] / [542] | `["?? unpacked/maps/0676"]` | 21,2 → 20,1 |

  Belegzeilen (Auszug):

  ```
  test vanilla_room_recipe_matches_the_wizard_scenario ... draft: Draft { cells: [DraftCell { matrix: None, x: 0, y: 0, source_map: 187 }], tiles: [DraftTile { matrix: Some(0), x: 105, z: 876 }], events: [DraftEvent { kind: Warp, matrix: Some(0), x: 105, z: 876, … target: Some(DraftTarget { header: 593, warp: Some(0) }) }, DraftEvent { kind: Warp, matrix: None, x: 8, z: 4, … target: Some(DraftTarget { header: 411, warp: Some(4) }) }], …
  CPUD: touched text archives [554], saved before the compile [554], not saved []
  CPUD: git status after undo ["?? unpacked/maps/0666"]
  IPKD: touched text archives [542], saved before the compile [542], not saved []
  IPKE: git status after build and undo ["?? unpacked/maps/0676"]
  ```

  Die Zeit von `--test templates` kommt fast ganz aus den vier Vanilla-Tests (dreimal Scripts übernehmen, Prüfungen, Build) unter Last 20 bis 35; `dev-setup.md` nennt etwa 50 s ohne Last für Platin DE.

## (2) Szenario kopflos

Release-`studio mcp --project <Kopie> [--allow-irreversible] [--expert]` über JSON-RPC/stdio (Protokoll 2025-11-25), je Spiel nacheinander auf `3d-2-k<spiel>` nach `g4k git init`, 21:20 bis 21:25, Last 11 bis 18. Alle Prüfungen bestanden auf allen drei Spielen (Logs `kopflos-<k>.log`, Zusammenfassung `kopflos-<k>.json`). Die Spezifikation des Raums ist die aus `pt-raum-assistent.yaml` (Platin DE: neuer Header 593 aus Vorlage 415, Karte 187, Eventdatei „Kopie von 394“, Tür außen Zweiblattdorf 411 bei x 105, z 876, innen x 8, z 4, Tür-Kacheln an); für HeartGold dieselbe Form mit dem Spielerzimmer (Header 540 aus Vorlage 64, Karte 217, Kopie von Eventdatei 61, Tür außen Neuborkia 60 bei x 686, z 397, innen x 3, z 4).

Ohne `--allow-irreversible`:

- [x] 1. `recipe_list` → vier Einträge `create_area`, `create_room`, `extend_area`, `empty_region`.
- [x] 2. `recipe_plan create_room` → `cells` 1 (`matrix: null`, die neue Weltkarte), `events` 2 (Tür außen auf Matrix 0, Rückweg im Raum), `tiles` 1 (die Tür-Kachel außen) auf allen drei Spielen; 7 / 38 / 62 ms kalt, 2 / 3 / 17 ms warm. Wie im Szenario (das vorher `apply_header_patch` ruft) gilt das mit `header.allow_patch: true`; die Spezifikation wörtlich, ohne den Schalter, lehnt der Plan auf dem ungepatchten Spiel ab wie `apply`: „new headers need the dynamic headers patch, apply the patch first“ (Hinweis H1).
- [x] 3. `recipe_apply empty_region` → `needs_confirmation` „recipe_apply lässt sich nicht rückgängig machen (Welt leeren); kopflos nur mit --allow-irreversible“; ebenso `recipe_apply create_room` mit `allow_patch`. `git status` danach leer.

Mit `--allow-irreversible`:

- [x] 4. `recipe_apply extend_area` (erste leere Zelle von Matrix 0, Kopie einer vorhandenen Karte) → `history.undo` 0 → 1 („extend area“), `change` gesetzt, `changed` Karte und Matrix 0, `next` `adjust_terrain` „Gelände anpassen“ gesperrt „Kommt mit Phase 6“ (4 / 29 / 14 ms); `undo` → `get_matrix 0` gleich dem Stand vorher.
- [x] 5. `recipe_apply create_room` → ein Schritt „Raum „Testzimmer“ angelegt“, Bericht wie die Szenarien (Platin: Matrix 289, Karte 667, Eventdatei 534, Warp 4 außen, Warp 0 innen wiederverwendet, Tür-Kachel Karte 0 x 9, y 12, `patched: true`), `next` `new_figure` „Neue Figur hier“ gesperrt „Kommt mit 3d.5“; 3 760 / 2 655 / 4 207 ms mit dem Patch. Dann `save`, `g4_call adopt_scripts` (`--expert`), `save`, außen `git add -A && git commit`.
- [x] 6. `recipe_apply empty_region` → `change` gesetzt, `history.undo` + 1 („Vorlage: Leere Region mit Startstadt“), `next` `create_area` „Gebiet anlegen“ frei; 4 526 / 4 207 / 12 675 ms.
- [x] 7. `git -C <Kopie> status --porcelain` nennt die Scripts und Archive: ` M .rotom/status/compile-state.json`, ` M expanded/scripts/1052.rotom`, ` M unpacked/scripts/1052`, ` M unpacked/textArchives/0554`, `?? expanded/scripts/1124.json`, `?? expanded/textArchives/0554.json`, `?? unpacked/maps/0668`, `?? unpacked/scripts/1124` (HeartGold: Script 0842, Level-Script 0965, Textarchiv 0542, Rand-Karte 0678).
- [x] 8. `undo` (5 / 93 / 5 ms) → `git status` nennt nur `?? unpacked/maps/0668` (HeartGold `0678`), keinen Pfad unter `expanded/scripts/`. Eine Prüfsumme über alle Dateien des Projekts außer `.git` vor `apply` und nach `undo` unterscheidet sich nur in der Rand-Karte und im Zwischenstand `.g4/build/state.json` (von Git ignoriert, Hinweis H4): Scripts, Level-Script-Datei, Textarchive (Member und Spiegel) und `compile-state.json` sind byte-gleich.
- [x] 9. `redo` (11 / 7 / 24 ms) → `git status` nennt wieder dieselben acht Pfade wie nach `apply`.

Szenarien mit `g4emu` (aus `scripts/g4emu`, auf dem Endstand, Dateien wörtlich unverändert):

- [x] 10. `pt-raum-assistent.yaml` auf `3d-2-raum` (`--set NEUER_HEADER=593 --set TUER_X=105 --set TUER_Y=876`): `"ok": true`, Header-Folge 415, 593, 411, 593 (79 s); `create_room` antwortete Matrix 289, Karte 666, Eventdatei 534, Warp 4 außen, Warp 0 innen wiederverwendet, Tür-Kachel Karte 0 x 9, y 12; Bilder `scen-raum/{raum,draussen,wieder-drin}.png`.
- [x] 11. `pt-leere-region.yaml` auf `g4k create` → `3d-2-leer-pt`: `"ok": true`, Header-Folge 415, 414, 411 (34 s nach `g4k create`); `adopt_scripts` 1 124 dekompiliert, `apply_empty_region` `plan.start_header` 411, `run_checks` 0 Fehler, `walk-until-blocked` 22 Schritte bis x 112, y 864, `expect-at` erfüllt; Bilder `scen-leer-pt/`.
- [x] 12. `hg-leere-region.yaml` auf `g4k create` → `3d-2-leer-hg`: `"ok": true`, Header-Folge 64, 63, 60 (37 s); `adopt_scripts` 965, `plan.start_header` 60, `run_checks` 0 Fehler, `walk-until-blocked` 23 Schritte bis x 672, y 399; Bilder `scen-leer-hg/`. Die drei Szenario-Dateien sind seit der Basis `3c194c7` unverändert (`git diff` leer).

## (3) Browser-Teil

Test-Umgebung auf 1710/1711, Chrome headless 1440 × 900, Seite `http://127.0.0.1:1710/?project=<Roms>/_testkopien/3d-2-web_DSPRE_contents` (Platin DE), danach `3d-2-webhg` (HeartGold DE) mit demselben Ablauf und `3d-2-mcp` für MCP. Last 20 bis 36. Bilder unter `target/agent/gate-2026-10-31/3d-2/`.

- [x] 1. **Segment „Erzählen“:** das Scharnier zeigt `[aria-label=Tiefe]` mit „Erzählen“, „Bauen“, „Tüfteln“; ein Klick auf „Erzählen“ setzt `aria-pressed`.
- [x] 2. **Welt in Erzählen:** mit Zweiblattdorf (Platin, `[data-area-key="w0:1"]`) bzw. Neuborkia (HeartGold, Header 60) in der Schau ersetzt die Rezept-Leiste die Werkzeugleiste (`[aria-label=Werkzeugleiste]` fehlt, `[aria-label=Rezepte]` mit `create_area`, `create_room` frei und `new_figure`, `connect_doors`, `new_trainer_here` gesperrt). Dock der Welt in Erzählen und Bauen gleich: 5 Reiter (`world`, `worldmap`, `script`, `problems`, `inspector`), in Tüfteln 6 (dazu `matrix`).
- [x] 3. **Reduzierter Inspector**, bestanden mit **Befund U1**: Figur (Platin Objekt 0, HeartGold Objekt 1) „Aussehen“ mit Sprite, „Bewegung“ „Geht umher“ / „Schaut sich um“, „Blickrichtung“ „Links“, „Trainer“ „kein Trainer“; Tür (Warp 0) „Ziel“ „Zweiblattdorf 0412 · T01R0101“ bzw. „Neuborkia 0061 · T20R0101“ und „Tür“ „Tür 0 · x 6, z 10“ bzw. „Tür 0 · x 4, z 14“; Schild „Der Text des Schilds kommt mit 3d.5.“; Auslöser „Geschichte und Schritt kommen mit 3d.7.“; Gebiet „Ort“ mit Name ohne Taste, Ort, Aussehen, Musik (Tag), Musik (Nacht); Raum „Haus 2, 1. Stock“ dieselben Zeilen und „Umbenennen…“. Darunter stehen aber bei Figur und Schild die Zeile „y“ mit dem Rohwert (65536 bzw. 0) und „Nur lesbar: Die Höhe lässt sich hier nicht ändern …“, bei allen vier Events die Auswahl „Warp“ mit „Hinzufügen“ und der Satz zum Ziehen (U1). Bilder `blick-erzaehlen-{figur,tuer,schild,ausloeser,gebiet,raum}.png`, HeartGold `blick-erzaehlen-hg-*.png`.
- [x] 4. **Technik, Pokémon, Texte, Gegenstände zeigen Bauen:** je Bereich das sichtbare Dock in Erzählen, Bauen und Tüfteln verglichen; `[data-dock-tab]` in Erzählen wie in Bauen (Pokémon 2, Texte 3, Gegenstände 2; Technik ohne Dock, ihr Text in Erzählen und Bauen gleich, 18 936 bzw. 24 517 Zeichen, in Tüfteln länger). Pokémon mit gewählter Art: Unterreiter „Übersicht, Bearbeiten, Werte, Fähigkeiten und Typen, Lernset, Entwicklung, TM/VM“ in Erzählen und Bauen, „Rohdaten“ und Hex nur in Tüfteln. Das Segment blieb dabei auf „Erzählen“. Auf beiden Spielen.
- [x] 5. **Zurück in der Welt** wieder Erzählen: Rezept-Leiste, der Inspector der Figur wie in 3.
- [x] 6. **„Raum anlegen“ bis zur Zusammenfassung** (aus der Leiste mit gewähltem Ort; Vorlage „Haus 2, 1. Stock“ 0415 bzw. 0064, Name „Testzimmer“, Ort vorhanden, Eingang außen x 105, z 876 bzw. x 686, z 397): `[data-draft-count]` = **2** auf beiden Spielen, sobald x und z stehen. Derselbe Körper der Anfrage (`recipe_plan`, `create_room`, `header.allow_patch: true`) über `call.sh recipe_plan` gibt 1 Zelle (`matrix: null`), 1 Kachel und 2 Events, davon auf Matrix 0 die Kachel und die Tür außen: `draftCount(…, 0)` = **2**, gleich. Keine `[data-draft-cell]`, weil die Zelle des Raums auf einer neuen Weltkarte liegt (so entschieden). In der Draufsicht mit maximierter Schau sitzt um die Kachel vor der Tür von „Haus 2“ ein kleiner gestrichelter Rahmen in der Auswahlfarbe mit einem halbtransparenten Tür-Marker (`blick-entwurf-nah.png`); in der normalen Anordnung verdeckt die Rezept-Leiste diese Stelle (U2).
- [x] 7. **Esc** (Fokus in der Seite): Assistent zu, `[data-draft-count]` = 0, `git -C <Kopie> status --porcelain` leer; der Fokus steht wieder auf „Raum anlegen“ mit Tooltip. Auf beiden Spielen.
- [x] 8. **Anlegen** (Kästchen der Patch-Zeile angehakt): Ergebnis „Raum angelegt“, „Raum „Testzimmer“ angelegt: Karte 0666.“ (HeartGold 0676), unter „Als Nächstes“ `[data-recipe=new_figure]` „Neue Figur hier“ gesperrt mit „Kommt mit 3d.5“; `[data-draft-count]` 0. 4,3 s (Platin) und 5,9 s (HeartGold) über die Debug-Brücke, mit Patch. Bilder `blick-raum-ergebnis{,-hg}.png`.
- [x] 9. **„Welt leeren“ aus Technik › Werkzeuge** (dort Bauen): „Scripts übernehmen“ und „Speichern“ im Assistenten, außen committet; die Zusammenfassung zeigt den Entwurf mit `[data-draft-count]` = 467 und 467 `[data-draft-cell]` (Platin); HeartGold 491 in der Schau und 246 auf der Weltkarte (sie zeigt Johto, die 245 übrigen Zellen liegen in der Ansicht Kanto derselben Matrix, Hinweis H3). „Welt leeren“ (24,3 s bzw. 23,5 s): „Welt geleert“, „Randkarte 0667, Level-Script 1124, Start-Script 1052 #10“, unter „Als Nächstes“ `[data-recipe=create_area]` „Gebiet anlegen“ frei; ein Klick öffnet „Gebiet anlegen“ (Schrittleiste „Quelle, Anordnung, Ort, Zusammenfassung“), „Abbrechen“. `git status` nach dem Anlegen: Scripts, Textarchiv, `compile-state.json`, Rand-Karte (acht Pfade wie kopflos).
- [x] 10. **⌘Z** (Fokus in der Seite, Taste Meta+Z über CDP): nach 0,3 s steht „Vorlage: Leere Region mit Startstadt“ unter `redo`, `call.sh dirty` → `[]`, `git status` → nur `?? unpacked/maps/0667` (HeartGold `0677`). Prüfsumme über das Projekt ohne `.git` vor dem Anlegen und nach ⌘Z (Platin): verschieden nur die Rand-Karte, `.g4/build/state.json` und 33 Modelle im Zwischenspeicher `.g4/models/` (alles von Git ignoriert, Hinweis H4).
- [x] 11. **MCP über die Brücke** (`3d-2-mcp`, Freigabe „Lesen und bearbeiten“): `mcp.sh call recipe_plan` mit der Raum-Spezifikation und `allow_patch` → 1 Zelle, 1 Kachel, 2 Events in 630 ms, keine Frage (`agent_status.question` null), Verlauf leer, `git status` leer. `mcp.sh call recipe_apply '{"recipe":"empty_region","input":{}}'` → Frage `{kind: confirm, client: agent-gate, what: "Welt leeren"}`, im Fenster `[data-agent-question]` „„agent-gate“ möchte: Welt leeren (lässt sich nicht rückgängig machen).“ mit „Ablehnen“ / „Zulassen“; „Ablehnen“ → `needs_confirmation` „recipe_apply lässt sich nicht rückgängig machen (Welt leeren); in Gen4 Studio abgelehnt“. Bild `blick-agentenfrage-welt-leeren.png`.
- [x] 12. **Konsole und Brückenlog:** auf der HeartGold-Seite (Mitschnitt von `console.error`, `console.warn`, Ausnahmen) kein Eintrag; im Brückenlog kein Fehler und keine Panik.

Nicht im Browser prüfbar: die gepackte App, Ziehen in der 3D-Ansicht mit echter Maus (nicht Teil des Gates).

## (4) Hand und Blick

Beim Nutzer: die drei Tiefen in der gepackten App, der Entwurf in Hell, Dunkel und Kompakt, die Rezept-Leiste mit Tastatur (Abschnitte „Hand“ und „Blick“).

## Ergebnis

| Punkt | Wer | Ergebnis | Datum |
|---|---|---|---|
| (1) 1 `scripts/gate.sh` grün | Agent | offen: vom Hauptorchestrator auf main nach dem Merge | – |
| (1) 2 Schritte von `gate.sh`, die 3d.2 berührt | Agent | bestanden: fmt, clippy, Kernel 1 442, Studio-Bibliothek 651, `mcp_stdio` 28, Tokens, Styles, Depth (0 Treffer), `pnpm test` 3 426, `pnpm build` | 2026-10-31 |
| (1) 3 Snapshot `inspectorFields.test.ts` | Agent | bestanden: 5/5 | 2026-10-31 |
| (1) 4 `check-depth.sh` 0 Treffer | Agent | bestanden | 2026-10-31 |
| (1) 5 gated `--test recipes`, `--test templates` | Agent | bestanden auf Platin DE, HeartGold DE, HeartGold US (je 11/11 und 27/27; der Raum-Test läuft nur auf Platin DE); jedes berührte Textarchiv gesichert, `git status` nach ⌘Z nur die Rand-Karte | 2026-10-31 |
| (2) 1 bis 3 ohne Erlaubnis | Agent | bestanden auf allen drei: vier Rezepte, Plan des Raums 1 / 2 / 1, `needs_confirmation` für „Welt leeren“ | 2026-10-31 |
| (2) 4 bis 9 mit Erlaubnis | Agent | bestanden auf allen drei: Erweitern und Raum je ein Schritt, ⌘Z der Matrix; „Welt leeren“ ein Schritt, `git status` nennt Scripts und Archive, nach `undo` nur die Rand-Karte, Scripts und Archive byte-gleich, `redo` | 2026-10-31 |
| (2) 10 bis 12 Szenarien | Agent | bestanden: `pt-raum-assistent`, `pt-leere-region`, `hg-leere-region` je `"ok": true` auf frischen Kopien, Dateien unverändert | 2026-10-31 |
| (3) 1, 2, 4 bis 12 Browser-Teil und MCP über die Brücke | Agent | bestanden auf Platin DE und HeartGold DE | 2026-10-31 |
| (3) 3 reduzierter Inspector | Agent | bestanden mit Befund U1 (Zeile „y“, „Hinzufügen“ und Ziehen-Satz in Erzählen), offen | 2026-10-31 |
| Entwurf in Hell, Dunkel, Kompakt (Task 8 Step 5) | Agent + Blick | Bilder liegen bereit, vom Agenten angesehen; Befund U2 (Rezept-Leiste verdeckt den Entwurf), offen | – |
| Drei Tiefen in der gepackten App | Hand | offen | – |
| Rezept-Leiste mit Tastatur (Roving, Tooltip der gesperrten Tasten) | Hand | offen; im Browser geprüft: Pfeile wandern über die zwei freien Tasten, Ende/Anfang springen, gesperrte Tasten sind nicht fokussierbar (Entscheidung Task 6), ihr Tooltip „Neue Figur · Kommt mit 3d.5“ erscheint beim Zeigen | – |

Zahlentabelle (Release-`studio mcp` kopflos aus (2), **unter Last**; in Klammern die Oberfläche über die Debug-Brücke aus (3)):

| Spiel | `recipe_plan` Raum (ms) | `recipe_apply` Raum (ms) | `recipe_apply` Welt leeren (ms) | `undo` danach (ms) | Scripts und Archive nach `undo` byte-gleich | Last |
|---|---|---|---|---|---|---|
| Platin DE | 7 kalt, 2 warm | 3 760 mit Patch (UI 4 300) | 4 526 (UI 24 300) | 5 (UI ⌘Z sichtbar nach 297) | ja: `git status` nur `?? unpacked/maps/0668`, Prüfsumme gleich bis auf Rand-Karte und `.g4/build/state.json` | kopflos 17,2 → 13,8; UI 20 bis 36 |
| HeartGold DE | 38 kalt, 3 warm | 2 655 mit Patch (UI 5 900) | 4 207 (UI 23 500) | 93 | ja: nur `?? unpacked/maps/0678` | kopflos 13,5 → 12,0; UI 20 bis 25 |
| HeartGold US | 62 kalt, 17 warm | 4 207 mit Patch | 12 675 | 5 | ja: nur `?? unpacked/maps/0678` | kopflos 11,2 → 14,3 |

Die Zeit von „Welt leeren“ steckt im Compile des Start-Scripts und im Schreiben der Rand-Karte; die 12,7 s auf HeartGold US fielen bei einer Last von 14,6. In der Oberfläche läuft dasselbe über die Debug-Brücke, dazu liest der Assistent die Welt neu.

## Befunde

### Gate-Lauf (2026-10-31)

| Nr. | Schwere | Punkt | Befund | Beleg | Vermutete Stelle |
|---|---|---|---|---|---|
| U1 | Mittel | (3) 3 reduzierter Inspector | In Erzählen zeigt der Inspector einer Figur und eines Schilds unter den Feldern aus Erzählen noch die Zeile „y“ mit dem Rohwert der Höhe (Figur Zweiblattdorf Objekt 0: 65536) und „Nur lesbar: Die Höhe lässt sich hier nicht ändern; sie bleibt unverändert erhalten.“; bei allen Events dazu die Auswahl „Warp“ mit „Hinzufügen“ und den Satz „Ziehen in der 3D-Ansicht verschiebt das Event um ganze Kacheln …; Entf löscht …“. Die Entscheidung „Felder in Erzählen“ nennt für Schild und Auslöser „keine Felder, eine Notice“; der Snapshot `inspectorFields.test.ts` sieht die Zeile nicht, weil sie außerhalb der Feldlisten steht. Auf Platin DE und HeartGold DE gleich. Der Kopf nennt zudem „Objekt 0 · Eventdatei 0390“ (Wörter von Bauen, nicht Teil des Gate-Texts). | `blick-erzaehlen-figur.png`, `blick-erzaehlen-schild.png`, `blick-erzaehlen-hg-figur.png` | `apps/studio/src/workbench/inspector/EventEditor.tsx` Z. 236 bis 240 (Zeile „y“ ohne `!narrating`), Z. 261 bis 266 (`AddEvent` und Satz zum Ziehen) |
| U2 | Klein (Blick) | (3) 6, Entwurf in der Schau | Die Rezept-Leiste ist bei 1440 × 900 in der Standard-Anordnung 724 × 94 px groß (zwei Zeilen, „Neuer Trainer hier“ bricht um; die Schau ist 740 × 433 px, unter 460 px liegt die Leiste oben flach), die Werkzeugleiste in Bauen 381 × 50 px. Mit offenem Assistenten ist die Schau 245 px hoch, und die Leiste deckt genau den oberen Streifen ab, in dem der Entwurf von „Raum anlegen“ in Zweiblattdorf steht (Tür-Kachel vor „Haus 2“); sichtbar wird er erst mit maximierter Schau oder in der Draufsicht. Der Rahmen der Kachel ist dort klein (1 Kachel, 1 px). Ob die Leiste im Assistenten schmaler wird, nur Symbole zeigt oder ausweicht, entscheidet der Nutzer am Bild. | `blick-entwurf-hell.png`, `blick-entwurf-maximiert.png`, `blick-entwurf-draufsicht.png`, `blick-entwurf-nah.png`, `blick-erzaehlen-rezeptleiste.png` | `apps/studio/src/ui/frame/RecipeBar.module.css` (Umbruch), `RecipeBar.tsx` (Beschriftung bei `anchor` „top“) |

Keine Befunde in (1) und (2). Kein Lauf wurde wegen der Last wiederholt; der erste Lauf von (2) auf Platin DE scheiterte am Skript des Gates (ohne `--expert` kein `g4_call`, Hinweis H2) und lief auf einer frischen Kopie noch einmal ganz.

### Aus der Umsetzung

Das abschließende Review des Branches fand zwei wichtige Punkte (I1 Compile nach „Welt leeren“ verwarf den Undo-Schritt, I2 kein Entwurf auf ungepatchten Spielen in Bauen und Erzählen bis zum Haken an der Patch-Zeile) und sieben kleinere; die Fix-Welle `0863674..bb12f49` behob I1, I2 und die nötigen kleinen Punkte, das Nach-Review war sauber. Im Gate bestätigt: der Entwurf erscheint auf beiden ungepatchten Spielen vor dem Haken (I2), `git status` bleibt nach Build und ⌘Z bei der Rand-Karte (I1, gated `vanilla_empty_region_builds`).

## Hinweise

- **H1, Plan ohne `allow_patch`:** `recipe_plan create_room` mit der Spezifikation aus `pt-raum-assistent.yaml` wörtlich (ohne `header.allow_patch`) lehnt auf einem Spiel ohne Patch der Dynamic Headers ab: „new headers need the dynamic headers patch, apply the patch first“, wie `apply`. Das Szenario ruft vorher `apply_header_patch`; der Assistent fragt den Plan immer mit `allow_patch: true` (Fix I2). Ein Agent, der erst plant und dann patcht, muss den Schalter setzen; `mcp.md` sagt es nicht ausdrücklich.
- **H2, `g4_call` kopflos:** `studio mcp --project` bietet `g4_call` (hier für `adopt_scripts`) nur mit `--expert`, an der Brücke nur im Expertenmodus. Ein Rezept-Werkzeug zum Übernehmen der Scripts gibt es nicht; „Welt leeren“ verlangt übernommene Scripts.
- **H3, HeartGold-Weltkarte:** Matrix 0 von HeartGold steht im Weltbaum als zwei Weltkarten (Johto `w0`, Kanto `w0k`); die Weltkarte zeigt nur die gewählte Hälfte, `[data-draft-cell]` zählt darum 246 von 491 Zellen, `[data-draft-count]` in der Schau alle 491.
- **H4, Zwischenstände nach ⌘Z:** `.g4/build/state.json` und Modelle unter `.g4/models/` ändern sich beim Arbeiten (Zwischenspeicher, von Git ignoriert); sie zählen nicht zu „alle Dateien“.
- **H5, Liste in Erzählen:** „In dieser Szene“ nennt in Erzählen dieselben Namen wie in Bauen („Objekt 0 · ID 0“, „Warp 0 → Haus 1“, „Hintergrund-Event 0 · Script 8201“, „Scriptdatei 1052“); die Assistenten nennen Hex-Werte der Tür-Kachel („Verhalten 0x5E WARP_STAIRS_EAST“, „0x69“, „0x80“), wie in Bauen (Entscheidung: Erzählen spricht wie Bauen). Erzählen-Layouts sind 3d.8.
- **H6, Bildmethode:** die Taste der Patch-Zeile ist ein verstecktes `input`; das Skript klickt auf ihr `label`. ⌘Z kam über CDP als Meta+Z mit dem Befehl `undo`, nachdem in die Seite geklickt war.

## Hand

- [ ] **Drei Tiefen in der gepackten App:** `pnpm bundle`, App aus dem Finder starten, ein Projekt öffnen; im Scharnier „Erzählen“, „Bauen“, „Tüfteln“ umschalten: Welt in Erzählen mit Rezept-Leiste und reduziertem Inspector, Technik, Pokémon, Texte und Gegenstände wie Bauen, das Segment bleibt auf „Erzählen“; die App schließen und öffnen: die Tiefe bleibt (`g4.shell.v1`).
- [ ] **Rezept-Leiste mit Tastatur:** Tab in die Leiste, Pfeiltasten wandern über „Gebiet anlegen“ und „Raum anlegen“ (die gesperrten Tasten sind nicht fokussierbar, Entscheidung Task 6), Enter öffnet den Assistenten; mit der Maus über einer gesperrten Taste der Tooltip „Kommt mit 3d.5“. Ob gesperrte Tasten für die Tastatur erreichbar sein sollen (`aria-disabled`), entscheidet der Nutzer.

## Blick

Bilder unter `target/agent/gate-2026-10-31/3d-2/` im Worktree, 1440 × 900, aus der Test-Umgebung über Chrome headless (Hell und Dunkel über `prefers-color-scheme`, Kompakt über die Taste der Statusleiste). Die Bilder aus Task 8 Step 5 gehören hierher (Entscheidung des Steuer-Agenten).

- [ ] `blick-entwurf-{hell,dunkel,kompakt}.png`: „Raum anlegen“, Zusammenfassung, Platin DE, `[data-draft-count]` 2; im Bild verdeckt die Rezept-Leiste die Stelle des Entwurfs (U2). Dazu `blick-entwurf-maximiert.png` (Schau maximiert), `blick-entwurf-draufsicht.png` (Draufsicht ohne Gelände und Ereignisse) und `blick-entwurf-nah.png` (vergrößert: gestrichelter Rahmen um die Kachel x 105, z 876 und der halbtransparente Tür-Marker vor „Haus 2“); HeartGold `blick-entwurf-hg-hell.png`.
- [ ] `blick-erzaehlen-{figur,tuer,schild,ausloeser,gebiet,raum}.png` und `blick-erzaehlen-hg-*.png`: Inspector in Erzählen (U1 sichtbar bei Figur und Schild).
- [ ] `blick-erzaehlen-rezeptleiste.png` und `blick-rezeptleiste-tooltip.png`: Rezept-Leiste mit zwei freien und drei gesperrten Tasten, Tooltip „Neue Figur · Kommt mit 3d.5“.
- [ ] `blick-raum-ergebnis{,-hg}.png`: Ergebnis „Raum angelegt“ mit „Neue Figur hier“ gesperrt.
- [ ] `blick-welt-leeren-entwurf{,-hg}.png` und `blick-welt-leeren-ergebnis.png`: Zusammenfassung von „Welt leeren“ mit dem Entwurf der Randzellen, Ergebnis mit „Gebiet anlegen“.
- [ ] `blick-agentenfrage-welt-leeren.png`: Agentenfrage „Welt leeren“.

## Offen

- **(1) 1 `scripts/gate.sh`:** auf main nach dem Merge (Hauptorchestrator).
- **U1, U2:** offen, Fix auf dem Branch oder Entscheidung des Nutzers.
- **Merge-Hinweise** (aus dem Ledger): Konflikte an den Anhängepunkten mit 3d.1, 3d.3 und 4.5; `check-depth.sh` kann nach dem Merge Stellen aus 3d.1 und 3d.3 finden (etwa `depth === "bauen" ? BAUEN_HINTS` aus 3d.1, wird `!depthAtLeast(depth, "tuefteln")`), die Zielzahl 0 gilt auf main.
- **`.fig`:** die Frames von 3d.2 sind nicht eingespielt (OpenPencil war in der Sitzung nicht verbunden, Task 9).
