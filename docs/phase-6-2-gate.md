# Phase 6.2: Gate prüfen

Stand: 2026-11-06. Branch `phase-6-2-bdhc` (Worktree `.claude/worktrees/phase-6-2`, Basis `d515c5ee` = main `d05fad84` mit den Plänen 6.1, 6.2 und 6.3a), nicht nach main gemergt. Vom Agenten erledigt: (1) Automatik Schritt für Schritt auf dem Stand `286d3ab1` (gated Tests im Release-Profil auf frischen Kopien von Platin DE, HeartGold DE und HeartGold US, Generator, `cargo deny`, fmt, clippy, die Tests ohne ROM-Daten, Frontend, Stil, Tokens, `g4emu`), (2) die Szenarien `pt-gelaende` und `hg-gelaende` mit der Gegenprobe (Task 9, `731af8d2`) und (3) der Browser-Teil mit MCP über die Brücke und den 18 Blick-Bildern (2026-11-06 auf `731af8d2`, 9/9 bestanden). `scripts/gate.sh` als Ganzes ist **offen — läuft in der Hauptsitzung auf main nach dem Merge**.

Beim Nutzer offen: der Blick auf Schicht „Höhen“, Overlay und Budget in Hell, Dunkel und Kompakt und auf die `g4emu`-Bilder; DSPRE (Windows) öffnet die gebaute Karte; `scripts/gate.sh` auf main; optional `pt-gelaende` auf HeartGold US (Abschnitt „(4) Hand und Blick“). Die Frames 6.2 stehen seit dem 09.11.2026 in der `.fig`.

Phase 6.2 bringt BDHC und Gelände: den Codec `crates/g4-formats/src/bdhc.rs` (sechs Tabellen lesen, byte-gleich schreiben, Strukturprüfung `validate`, `height_at` wie die Höhensuche des Spiels, `flat`), den Generator `bdhc/grid.rs` (`HeightGrid`, `Ramp`, `from_grid`: Rechteck-Zerlegung je Ebene, Rampen als geneigte Platten, halboffene Streifen; nach Konstruktion ≤ 35 592 B), im Kernel das Modul `terrain` (Regeltabelle `terrain_rules.yaml` je Familie, Zeilen wie die Permissions, `derive_permissions`, Handwerte, Höhen je Kachelmitte, Budget je Karte), die Prüfungen `map.altitude-walkable`, `map.step-unblocked` (Fix `block_upper` „Obere Zelle blockieren“), `map.buildings-over-limit` und die Strukturregeln in `map.bdhc-size`, die Befehle `get_map_heights`, `set_map_terrain` und `get_map_budget` (Tauri und MCP), in der Szene Overlay, Marker, Picking, Event-Drag und Entwurf auf der BDHC-Höhe statt der Matrix-Ebene, die Schicht „Höhen“ (sieben cividis-Stufen, Isolinien) und die Budget-Leiste im Inspector der Karte, in `g4emu` den Farbfix (RGBX) und den Schritt `expect-height`. Roadmap: [docs/ROADMAP.md](ROADMAP.md), Abschnitt „Phase 6“, Zeile 6.2. Spezifikation: [2026-11-05-phase-6-kartenerstellung-design.md](superpowers/specs/2026-11-05-phase-6-kartenerstellung-design.md) §4. Plan: [2026-11-05-phase-6-2-bdhc.md](superpowers/plans/2026-11-05-phase-6-2-bdhc.md). Formate: [bdhc.md](formats/bdhc.md), [terrain.md](formats/terrain.md); Werkzeuge: [mcp.md](mcp.md).

Gate-Text der Spezifikation (§4, Kopf der Gate-Spalte und Zeile 6.2):

> Gate (zusätzlich: gated Tests auf frischen Kopien von Platin DE, HeartGold DE und HeartGold US; Browser-Teil über die Test-Umgebung; `scripts/gate.sh` grün; `cargo deny` grün; Hand und Blick beim Nutzer)

> Gated: alle 2 018 Vanilla-BDHC byte-gleich, Strukturregeln 0 Treffer; Generator-Eigenschaftstest (1 000 Zufallsraster, alle ≤ 0x9000, Höhe an fünf Punkten je Zelle ≤ 0,1 E); Szenarien `pt-gelaende`/`hg-gelaende`: 1 × 1-Gebiet, Karte mit Ebenen 0, 16, 40, Treppe 0 → 16, Kante 16 → 40 ohne Treppe, Wand, Grasfeld (Begegnungsdatei Rate 100), bis 6.3 über einen Kernel-Befehl „Höhenraster setzen“: `walk` über die Rampe und `expect-height 16`, `walk-until-blocked` + `expect-at` vor der Kante, `map.step-unblocked` meldet die Stufe und ihr Fix blockiert die obere Zelle, Wand blockiert, `walk-until-battle` + `expect-battle`, Bilder je Station; Gegenprobe mit flacher BDHC scheitert an `expect-height`; Studio: auf Vanilla Platin Karte 100 (Boden 32 E) stehen Marker und Overlay auf 2 Kacheln Höhe, `pickTile` am Hang trifft die Kachel unter dem Zeiger; g4emu-Bilder mit richtigen Farben; Hand: DSPRE öffnet die gebaute Karte

Wer was prüft: Die Spalte „Wer“ sagt **Agent** (die Test-Umgebung aus [docs/agent-gate.md](agent-gate.md): Browser-Fenster mit `studio-bridge`, `call.sh`, `mcp.sh`, dazu `g4emu` und die gated Tests), **Agent + Blick** (der Agent prüft und beschreibt Ort und Zahlen, der Nutzer bestätigt kurz am Bild) oder **Hand** (der Nutzer). Befehle: [docs/agent-gate.md](agent-gate.md), Abschnitt „Phase 6.2“. „Entscheidung R<n>“ meint die Entscheidungen des Koordinators während der Umsetzung (Protokoll im Arbeitsordner der Phase, `.superpowers/sdd/2026-11-05-phase-6-2-bdhc/progress.md`, nicht im Repo); die für das Gate wichtigen stehen hier mit ihrem Inhalt.

Namen in der Oberfläche (Rollen und `data-*` für die Prüfung):

- **Leinwand der Szene** `canvas[data-hover-tile="<x>,<z>"]`: die absolute Kachel unter dem Zeiger (höchste Platte zuerst, wie das Auge es sieht); `[data-hover-ground]` ihre Höhe in Szenen-Einheiten (eine Kachel = 16 E, also 32 E = `2.000`). Die Statusleiste nennt dazu „Kachel x 7, y 8 · Karte 0100 · blockiert · Höhe 4 Stufen“ (eine Stufe = 8 E); liegt ein Marker darüber, nennt sie den Marker.
- **Marker** `[data-selected-marker-y]`: die Höhe des gewählten Markers (eine Figur auf 32 E: `2.300`, 0,3 über dem Boden).
- **`window.__g4SceneProbe`** (nur im Dev-Build): `tileToClient(x, z)` gibt den Bildschirmpunkt einer Kachel auf ihrer BDHC-Höhe; an diesem Punkt ein echtes Zeigerereignis.
- **Taste „Höhen“** in der Szenenleiste (Symbol `heights`, neben „Gelände“ und „Draufsicht“ mit dem neuen Symbol `topDown`): schaltet die Schicht „Höhen“ an und „Gelände“ aus, `[data-heights-shown="true"]`, sieben cividis-Stufen, Isolinien an der höheren Seite einer Kante.
- **Budget** im Inspector der Karte (Bauen, Zelle gewählt) `[data-map-budget]`, Abschnitt „Budget“ mit vier Zeilen `[data-budget=model|bdhc|buildings|vram]`: „Modell … von 61.440 B“, „Terrain (BDHC) … von 36.864 B“, „Gebäude … von 32“, „Texturen des Gebiets“ mit „VRAM … von 262.144 B · N Texturen“ und „Vanilla höchstens … B“; je Balken `[data-budget-state=calm|tight|full]` (ruhig ohne Plakette, „Knapp“, „Voll“).

## Vorbereitung

So durchgeführt am 2026-11-05 und 2026-11-06:

1. **Kopien** unter `Roms/_testkopien/`, vom Koordinator am 2026-11-05 angelegt, je `_DSPRE_contents`:
   - `6-2-pt` (Platin DE), `6-2-hgde` (HeartGold DE), `6-2-hgus` (HeartGold US) für (1), frisch und nur gelesen; geschrieben wird nur auf `vanilla_copy()` (`set_map_terrain_on_a_vanilla_copy_survives_save_and_reopen`, der Studio-Test). Die gated Tests von 6.2 brauchen keine Basis-ROM.
   - `6-2-szen-pt`, `6-2-szen-hg` (Szenarien) und `6-2-flach-pt`, `6-2-flach-hg` (Gegenprobe) für (2), je mit `.nds` daneben; jede Kopie ist nach ihrem Lauf verbraucht (gespeichert und gebaut, `out.nds` im Projektordner).
   - `6-2-web` (Platin DE, mit `.nds`) für (3); danach mit einem gespeicherten Objekt 4 in Eventdatei 326 (Punkt 3), alles Weitere zurückgenommen.
   - `6-2-farbe.nds` (Platin DE) für die Farbprobe von `g4emu` (Task 8).
2. **Worktree:** `G4_ROMS_DIR=/Users/POLTEP/Projekte/lab/editor/Roms` absolut; `tools` ist der Symlink auf den Haupt-Checkout (nie einchecken; der Kern von `g4emu` unter `tools/g4emu/core/melondsds_libretro.dylib`); Test-Umgebung auf `G4_VITE_PORT=1440 G4_BRIDGE_PORT=1441`; cargo mit `PATH="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$PATH"`.
3. **Ausgaben** im Worktree (nicht im Repo): Logs von (1) unter `target/agent/6-2/gate-<schritt>.log`, Rückgabe, Dauer und Last je Schritt in `target/agent/6-2/gate-summary-a.txt` (cargo) und `gate-summary-b.txt` (Frontend, Stil, `g4emu`); Szenarien unter `target/agent/gate-2026-11-06/6-2/{pt-gelaende,hg-gelaende,pt-flach,hg-flach}/`, Logs `target/agent/6-2/{pt-gelaende,hg-gelaende,pt-flach,hg-flach}.log`; die 18 Blick-Bilder unter `target/agent/gate-2026-11-06/6-2/blick-*.png`; die Farbprobe `target/agent/6-2/farbe/zimmer.png`. Die Ordner vor `git worktree remove` sichern.
4. **Last:** `uptime` vor und nach jedem Schritt (Spalte „Last“ der Zahlentabelle). Parallel arbeiteten die Worktrees von 6.1 und 6.3a.

## (1) Automatik

`scripts/gate.sh` läuft hier nicht als Ganzes (Entscheidung des Koordinators: die Hauptsitzung lässt es auf main nach dem Merge laufen); stattdessen laufen die Schritte, die 6.2 berührt, einzeln. Die gated Tests von 6.2 laufen in `scripts/gate.sh` im Schritt der gated Tests je Projekt ohne Änderung am Skript mit.

Alle Schritte des Agenten grün im ersten Versuch, 2026-11-06 03:41 bis 04:40 auf `286d3ab1` (der Stand der Abnahme; danach nur dieser Dokumentations-Commit), Logs `target/agent/6-2/gate-<schritt>.log`, Rückgabe, Dauer und Last in `gate-summary-a.txt` und `gate-summary-b.txt`. Die gated Tests liefen nacheinander, Kopie für Kopie, mit `G4_TEST_PROJECT=$G4_ROMS_DIR/_testkopien/6-2-<pt|hgde|hgus>_DSPRE_contents`; die Release-Binaries waren aus den Tasks gebaut, darum kurze Laufzeiten. Kein Zeittest war rot, keine Wiederholung nötig.

- [ ] 1. `scripts/gate.sh` grün: **offen — läuft in der Hauptsitzung auf main nach dem Merge** (Entscheidung des Koordinators; die gated Tests von 6.2 laufen dort im Schritt der gated Tests je Projekt mit).
- [x] 2. **Gated `vanilla_bdhc_round_trip_byte_identical_and_valid`** (Release, `--nocapture --test-threads 1`): Platin DE 666 Karten, 0 Warnungen, Nutzlast min 50 / Median 300 / p90 978 / max **21 452 B (Karte 188)**; HeartGold DE und US je 676 Karten, 1 Warnung (Karte 0464: Platte 0 senkrecht, Normale y = 0), min 50 / Median 311 / p90 1 672 / max **10 778 B (Karte 658)**; jede Datei schreibt sich byte-gleich zurück, `validate` meldet 0 Fehler. Zusammen **2 018 BDHC byte-gleich**; je 1 s.
- [x] 3. **Gated `vanilla_checks_match_the_fixed_counts`** (`g4-kernel --all-features --test vanilla_checks`, alle drei Tests der Datei 3/3): die drei neuen Zeilen und die Strukturregeln je Spiel gleich dem Soll: `map.altitude-walkable` 0 / 0 / 0, `map.step-unblocked` **152 / 52 / 52** (warm 17 470 / 9 897 / 10 604 µs), `map.buildings-over-limit` 0 / 0 / 0, `map.bdhc-size` (mit den Strukturregeln) 0 / 0 / 0; jede Prüfung mit Schwere Fehler 0 Befunde. Die billige Gruppe allein 1 525 / 509 / 446 ms auf einem neuen Stand, danach 491 / 130 / 123 ms.
- [x] 4. **Gated `tests/terrain.rs`** (`g4-kernel --all-features --test terrain`): je 4/4. `vanilla_map_100_floor_is_two_tiles_high` (nur Platin; auf HeartGold „skipped“): Karte 100 hat 32 E auf 724 Kacheln, 96 E auf 300. `vanilla_heights_cover_the_walkable_tiles`: freie Kacheln ohne Höhe 79 789 von 346 823 (23,006 %) bzw. 132 725 von 425 951 (31,160 %), 0 unlesbare BDHC, 67 / 84 / 62 ms über alle Karten. `vanilla_free_steps_are_counted`: 152 freie Stufen auf 5 Karten (16 E × 138, 32 E × 14) bzw. 52 auf 9 Karten (16 E × 32, 19 E × 6, 20 E × 14), dieselbe Zahl mit den rohen Proben ohne Extrapolation, 14 / 32 / 35 µs je Karte. `set_map_terrain_on_a_vanilla_copy_survives_save_and_reopen`: das Szenario-Raster auf Karte 0 der Kopie gibt 334 B, Nutzlast 318, 9 Platten, 5 Streifen, 52 blockiert, 1 Handwert, in 1 ms, und übersteht Speichern und Neuöffnen.
- [x] 5. **Gated Studio-Test** `terrain_over_the_command_path_on_the_vanilla_copy` (`-p studio --release --lib`): je 1/1. Platin: `get_map_heights 100` häufigste Höhe 32,0; `get_map_budget 188` `bdhc.used` 21 452 ruhig; HeartGold: Karte 658 10 778 ruhig; `set_map_terrain` (0 / 0 / 1 ms) und Undo stellen die Bytes wieder her; alle Budgets 260 / 78 / 78 ms. VRAM-Schätzung über 75 bzw. 96 Gebiete: Median 38 272 / 31 136 B, Maximum 115 648 / 89 296 B (Gebiet 6, je eines über dem Vanilla-Maximum, Befund V1).
- [x] 6. **Generator** `cargo test -p g4-formats bdhc::grid --release -- --nocapture`: alle Tests von `bdhc::grid` grün; `generated_bdhc_never_exceeds_the_bound` und `generated_heights_hit_five_points_per_cell` über 1 000 Zufallsraster und die vier Extreme: größte Nutzlast **19 744 B** (Schranke 35 592 B, Grenze 0x9000 = 36 864 B), größte Abweichung an fünf Punkten je Zelle **0,0786 E** (Grenze 0,1 E); 6 s.
- [x] 7. `cargo deny --locked check licenses advisories`: grün (`advisories ok, licenses ok`, Rückgabe 0, 3 s; `--locked` und das Holen der Advisory-Datenbank gingen ohne Fehler); einzige Ausgabe die bekannte Warnung `no-license-field` für `chatot 0.8.1` (über `uxie` und `g4-kernel`, nicht von 6.2; 6.2 bringt keine neue Abhängigkeit).
- [x] 8. `cargo fmt --check` und `cargo clippy --workspace --all-targets --all-features -- -D warnings`: sauber.
- [x] 9. `cargo test -p g4-formats -p g4-kernel --all-features` ohne ROM-Daten (`G4_TEST_PROJECT` nicht gesetzt): 56 Testziele, 2 292 bestanden, 0 fehlgeschlagen, 1 ignoriert (Formate-Bibliothek 197, Kernel-Bibliothek 1 225 in 1 554 s; die gated Tests melden ohne `G4_TEST_PROJECT` „skipped“); 2 736 s bei Last 7 → 14.
- [x] 10. `cargo test -p studio` ohne ROM-Daten: Bibliothek 723 bestanden, 1 ignoriert; `mcp_http` 5, `mcp_proxy` 10, `mcp_stdio` 28; 333 s bei Last 14 → 22.
- [x] 11. `pnpm test` in `apps/studio`: 416 Dateien bestanden, 1 übersprungen; 3 955 Tests bestanden, 1 übersprungen (206 s bei Last 3 → 16); `pnpm build` (tsc und vite) grün (36 s). Ein Skript `pnpm lint` gibt es in `apps/studio/package.json` nicht (die Typprüfung läuft in `pnpm build`); `pnpm --filter studio …` aus der Wurzel ist gleichwertig.
- [x] 12. `bash scripts/check-styles.sh` („check-styles: ok“) und `bash scripts/check-tokens.sh` („Alles stimmig.“).
- [x] 13. `cd scripts/g4emu && uv run pytest -q` mit `G4_ROMS_DIR` absolut: 407 bestanden, 3 übersprungen (27 s). Darin die vier Integrationstests mit dem Kern unter `tools/g4emu/core/` und der Platin-ROM unter `G4_ROMS_DIR` (nur gelesen; der Spielstand wird zuerst in den Temp-Ordner kopiert, unter `Roms` entstand keine Datei); übersprungen die drei Live-Tests von `test_live.py` (`G4_LIVE_ROM_CPUD`, `…_IPKD`, `…_IPKE` nicht gesetzt).

Belegzeilen (Auszug aus den Logs):

```
Pt: 666 maps, 0 warnings, payload min 50 / median 300 / p90 978 / max 21452 (map 188)
Hgss: 676 maps, 1 warnings, payload min 50 / median 311 / p90 1672 / max 10778 (map 658)      (HG DE und HG US)
map.step-unblocked            152    152     17470      (Platin DE; HG DE 52 52 9897, HG US 52 52 10604)
map 100 heights (E: tiles): 32: 724, 96: 300
free steps: 152 on 5 of 666 maps (0 with centres not 16 E apart), 0 unreadable BDHC, 9.167042ms (13.764µs per map)
free steps: 52 on 9 of 676 maps (20 with centres not 16 E apart), 0 unreadable BDHC, 21.960042ms (32.485µs per map)
terrain CPUD map 188: model 2244 B, bdhc 21452 B, buildings 16, area 20 (packs 20 / 16, 45 textures): VRAM 21760 B (Calm, vanilla max 110640)
terrain IPKD map 658: model 22052 B, bdhc 10778 B, buildings 0, area 75 (packs 75 / 73, 120 textures): VRAM 66208 B (Calm, vanilla max 87424)
terrain CPUD VRAM estimate over 75 areas: median 38272 B, max 115648 B (area 6), 1 above the vanilla maximum 110640
terrain IPKD VRAM estimate over 96 areas: median 31136 B, max 89296 B (area 6), 1 above the vanilla maximum 87424
largest payload from a grid: 19744 B
largest deviation from the grid: 0.0786 units
advisories ok, licenses ok
```

Dieselben Zahlen hatten die Tasks 1 bis 5 gemessen (Logs `target/agent/6-2/{vanilla-bdhc,vanilla-checks,terrain,commands}-<kopie>.log`); sie sind auf dem Stand der Abnahme unverändert.

Befehle: [docs/agent-gate.md](agent-gate.md), Abschnitt „Phase 6.2“, „Automatik (1)“.

## (2) Szenarien

**`pt-gelaende` und `hg-gelaende`** (Task 9, 2026-11-06 auf `b9d715f6` + Szenarien, Commit `731af8d2`): über die Brücke (`G4_BRIDGE_BIN=target/debug/studio-bridge`, `G4K_BIN=target/debug/g4k` nach `cargo build --workspace --all-features`), je ein neues 1 × 1-Gebiet auf einem neuen Header (Platin aus Route 201 Karte 5, Vorlage 342; HeartGold Karte 1, Vorlage 33), neue Eventdatei, Begegnungsdatei (Rate 100, Mewtu Lv 5 in allen zwölf Gras-Plätzen), Scriptdatei mit Geschenk; dann (a) `set_map_terrain` mit Raster, Gelände und dem Handwert (18, 25) frei, (b) `run_checks` findet genau die Stufe, `apply_fix block_upper`, (c) `get_map` zeigt (18, 25) blockiert, (d) `set_map_terrain` noch einmal mit `bdhc: ${BDHC}` und `permissions: keep`; Treppe des Zimmers dorthin, `save`, `build: out.nds`; gespielt ab dem Zimmer.

Befehle: [docs/agent-gate.md](agent-gate.md), Abschnitt „Phase 6.2“, „Szenarien (2)“.

- [x] 1. **Platin DE** (`6-2-szen-pt`, `BDHC=grid`): Rückgabe 0, `"ok": true`, Header 415, 593. Neun Erwartungen erfüllt: `set_map_terrain payload = 318`, `overrides = 1`, `permissions_kept = false`; `run_checks groups.length = 3`, `findings[code=map.step-unblocked].length = 1`, `findings.0.key = "step:18,25:18,24"` (Hinweis „Between tile (18, 25) and (18, 24) of map 666 lies a step of 16 units the game lets the player walk.“, Fix `block_upper`); `apply_fix remaining = false`; `get_map permissions.818 = [0, 128]`; (d) `permissions_kept = true`. Terrain (a): 334 B, Nutzlast 318, 9 Platten, 5 Streifen, 52 blockiert, 1 Handwert; (d) dieselbe BDHC, 53 blockiert. Spiel: `expect-height 0` (6 Treffer) auf (14, 21), **`expect-height 16` (6) auf (18, 21)**, `expect-height 12 0.1` (4) auf (17, 21); `walk-until-blocked right 8` hält auf (22, 21) vor der Kante 16 → 40 nach 4 Schritten, `walk-until-blocked down 3` auf (18, 24) vor der gesperrten oberen Zelle nach 1 Schritt, `walk-until-blocked right 3` auf (24, 26) vor der Wand nach 1 Schritt; `walk-until-battle` + `expect-battle 150 5` auf dem ersten Gras-Schritt (16, 10).
- [x] 2. **HeartGold DE** (`6-2-szen-hg`): Rückgabe 0, `"ok": true`, Header 64, 540, dieselben neun Erwartungen (Karte 676), dieselben Höhen und Haltestellen; der Kampf nach 3 Gras-Schritten.
- [x] 3. **Gegenprobe** (`6-2-flach-pt`, `6-2-flach-hg`, `BDHC=flat`: (d) schreibt eine flache Platte auf 0, 66 B, Nutzlast 50, 1 Platte, 53 blockiert): je Rückgabe 2, direkt nach `expect-at 18 21` `{"step": "expect-height", "args": ["16"], "ok": false, "error": "Höhe 0 statt 16 (±0.1) bei Kachel x 18, y 21; Treffer 0 (6×)"}`.
- [x] 4. **Bilder** je Station (nicht im Repo) `target/agent/gate-2026-11-06/6-2/{pt-gelaende,hg-gelaende}/{geschenk,treppe,kante,stufe,wand,gras}.png`, alle zwölf da; vom Agenten angesehen: Farben richtig (grüne Bäume, grauer Weg, natürliche Kampffarben, kein Rot-Blau-Tausch); der Kampf „Ein wildes MEWTU erscheint!“ in beiden Spielen. Die Figur ist in mehreren Bildern von Bäumen verdeckt (Abweichung 4 unten), die Orte belegen `expect-at` und `expect-height` aus dem RAM. Bestätigung beim Nutzer (Abschnitt „Blick“).
- [x] 5. **Farbprobe** (Task 8, `6-2-farbe.nds`, `continue expect-map 415 shot zimmer`, Rückgabe 0): Holzboden des Zimmers in zwei Streifen (3 200 px) im Mittel R 140,1 · G 122,0 · B 104,9, also R > B (braun, vor dem Fix bläulich); Bild `target/agent/6-2/farbe/zimmer.png`. Dazu im Vanilla-Zimmer `expect-height 0` ok (6 Treffer) und `expect-height 16` Rückgabe 2 „Höhe 0 statt 16 (±0.1) bei Kachel x 7, y 4; Treffer 0 (6×)“ (`target/agent/6-2/hoehe.log`).

Abweichungen von Plan und Brief (1, 2 und 4 in beiden Kopfkommentaren der Szenarien festgehalten, 3 in dem von `hg-gelaende.yaml`):

1. **Reihenfolge der Treppe** (Entscheidung R11): `walk right 4`, `expect-at 18 21`, `expect-height 16`, `walk left 1`, `expect-at 17 21`, `expect-height 12 0.1`, `walk right 1`, `expect-at 18 21`, `shot treppe`. In der Reihenfolge des Plans (erst 12 auf der oberen Treppenstufe, dann 16) scheiterte die Gegenprobe schon an „Höhe 0 statt 12“ statt am entschiedenen „Höhe 0 statt 16“; alle Zusicherungen bleiben. Der Plantext von Task 9 ist nachgezogen.
2. **Nutzlast** als `{path: payload, equals: 318}`: Erwartungen kennen nur Gleichheit; 318 ist strenger als „≤ 36 864“. Dazu die Erwartungen `overrides` und `permissions_kept`.
3. **Geschenk-Figur in HeartGold** mit `graphics_id: 317` (wie `hg-trainer-begegnung.yaml`); die 6 des Briefs ist der Platin-Wert.
4. **Figur verdeckt** (Entscheidung R12): das Gelände ändert nur BDHC und Kollision, das Modell bleibt das der Quellkarte. In HeartGold liegen die Stationen unter dem Wald von Route 29 (verdeckt in `geschenk`, `treppe`, `kante`, `stufe`, `wand`), in Platin bei `wand` verdeckt und bei `stufe` nur der Kopf über den Bäumen. Der sichtbare Beweis ist darum schwächer; Orte und Höhen belegen die RAM-Schritte. Ein passendes Modell backt erst 6.3b.

## (3) Browser-Teil

UI-Test 2026-11-06 03:05 bis 03:25 auf `731af8d2`, Last 3,9 bis 4,7. Test-Umgebung `G4_VITE_PORT=1440 G4_BRIDGE_PORT=1441` (Debug-Brücke), Seite `http://127.0.0.1:1440/?project=<Roms>/_testkopien/6-2-web_DSPRE_contents`, Browser-Fenster 1440 × 900, Bauen; Zeigerereignisse als echte Maus am Punkt von `__g4SceneProbe.tileToClient`. Alle neun Punkte bestanden; Bericht mit allen Werten im Arbeitsordner der Phase (`gate-browser-report.md`).

- [x] 1. **Karte 100 finden:** `get_matrix {"index":0}` → `maps[i] == 100` bei Zelle (9, 6), `list_headers` → Header 340 L03 „Stärkeufer“ (Eventdatei 326, Matrix 0); absolute Kacheln x 288 bis 319, z 192 bis 223. Ort gewählt, Szene lädt Gebiet 0340, „Draufsicht“ `aria-pressed=false`. `get_map_heights 100`: zwei Höhen, 32 E (724 Kacheln) und 96 E (300 Kacheln), also eine Kante 32 → 96 E ohne Zwischenhöhen.
- [x] 2. **Kachel mit 32 E:** Kachel 295,200 (Karte 100 x 7, y 8): `data-hover-tile` 295,200, `data-hover-ground` **2.000**, Statusleiste „Kachel x 7, y 8 · Karte 0100 · blockiert · Höhe 4 Stufen“. Das Overlay steht auf 2 Kacheln Höhe.
- [x] 3. **Figur:** Karte 100 hatte keine (Objekte 0 bis 3 liegen auf Karte 102): `call.sh add_event {"eventFile":326,"kind":"object","x":300,"z":212}` → Objekt 4, `save`, Neuladen; Klick auf den Marker: Inspector „Figur Objekt 4 · ID 4“, `data-selected-marker-y` **2.300**. Der Marker steht auf 2 Kacheln Höhe.
- [x] 4. **Zehn Kacheln an der Kante:** 10 von 10 genau (`data-hover-tile` = angefahrene Kachel): 300,210 / 301,210 / 301,212 / 302,209 / 293,221 auf 32 E (`2.000`) und 302,210 / 303,210 / 302,214 / 305,210 / 295,221 auf 96 E (`6.000`), mit gedrehter Kamera (Befund B1). `pickTile` am Hang trifft die Kachel unter dem Zeiger.
- [x] 5. **Figur über die Kante ziehen:** von 300,212 (32 E) nach 303,212 (96 E): `get_events 326` Objekt 4 x 303, z 212; Verlauf „move event“; `data-selected-marker-y` **6.300**.
- [x] 6. **„Höhen“ an:** „Höhen“ `aria-pressed=true`, „Gelände“ `false`, `data-heights-shown="true"`, Isolinien als dünne Linien an der Plateaukante.
- [x] 7. **Budget:** Karte 100 „Modell 31.812 von 61.440 B“, „Terrain (BDHC) 184 von 36.864 B“, „Gebäude 0 von 32“, „VRAM 73.840 von 262.144 B · 57 Texturen, Vanilla höchstens 110.640 B“, alle ruhig; Karte 188 (Matrix 130, Header 423 „Sandgemme, Haus 2“) „Modell 2.244 von 61.440 B“, **„Terrain (BDHC) 21.452 von 36.864 B“** (Balken 58 %), „Gebäude 16 von 32“, „VRAM 21.760 von 262.144 B · 45 Texturen“, alle ruhig. Über alle 666 Karten: 32 Gebäude auf 226, 227, 228, 242, 275, 30 auf 452, 453, 476; Karte 242 (Header 118 „Herzhofen, Haus 11, 1. Stock, Wettbewerbshalle“) zeigt „Gebäude“ **Knapp** 32 von 32 (`data-budget-state=tight`).
- [x] 8. **⌘Z nach `set_map_terrain`:** über `mcp.sh call set_map_terrain` (alle Höhen 3 = 24 E; Antwort `bdhc_bytes 66, plates 1`): vorher 2.000 (300,210) / 6.000 (303,212) / Marker 6.300 → nach dem Setzen 1.500 / 1.500 / Marker **1.800** (Status „begehbar · Höhe 3 Stufen“) → nach ⌘Z (echte Taste) 2.000 / 6.000 / Marker **6.300**, Status „blockiert · Höhe 4 Stufen“. Overlay und Marker folgen dem Undo. Abweichung: der Gate-Text des Plans nennt `call.sh set_map_terrain`; ein Schreiben über `call.sh` erreicht die offene Seite nicht (Befund B2), darum über `mcp.sh` in derselben Sitzung.
- [x] 9. **MCP über die Brücke** („Lokale Agenten zulassen“ mit `call.sh agent_set_enabled`, Nachfrage im Fenster mit „Lesen und bearbeiten“): `mcp.sh call get_map_heights '{"index":100}'` → 1 024 Werte (32.0 × 724, 96.0 × 300), `error` null; `mcp.sh call get_map_budget '{"index":188}'` → `bdhc {used 21452, max 36864, state calm}`, `model 2244`, `buildings 16`, `textures {area 20, textures 45, vanilla_max 110640, vram 21760}`, `textures_missing` null; Rückgabe 0.

Danach: Test-Umgebung über die PID beendet, `lsof -iTCP:1440 -iTCP:1441 -sTCP:LISTEN` leer, Chrome headless beendet; `target/agent/app-data/agents.json` mit Schalter an und Freigabe „Lesen und bearbeiten“ für `6-2-web`.

## (4) Hand und Blick

Beim Nutzer (Abschnitte „Hand“ und „Blick“ unten): der Blick auf Schicht „Höhen“, Overlay und Budget in Hell, Dunkel, Kompakt und auf die `g4emu`-Bilder; DSPRE öffnet die gebaute Karte; optional `pt-gelaende` auf HeartGold US; `scripts/gate.sh` auf main (Hauptsitzung).

## Ergebnis

| Punkt | Wer | Ergebnis | Datum |
|---|---|---|---|
| (1) 1 `scripts/gate.sh` grün | Koordinator | **offen — läuft in der Hauptsitzung auf main nach dem Merge** | – |
| (1) 2 gated `vanilla_bdhc` | Agent | bestanden auf Platin DE, HeartGold DE, HeartGold US: 666 / 676 / 676 BDHC byte-gleich (2 018), Strukturfehler 0, Warnungen 0 / 1 / 1, größte Nutzlast 21 452 B (188) bzw. 10 778 B (658) | 2026-11-06 |
| (1) 3 gated `vanilla_checks` | Agent | bestanden auf allen drei: `map.altitude-walkable` 0, `map.step-unblocked` 152 / 52 / 52, `map.buildings-over-limit` 0, `map.bdhc-size` 0 (Strukturregeln 0 Treffer) | 2026-11-06 |
| (1) 4 gated `tests/terrain.rs` | Agent | bestanden, je 4/4: Karte 100 Boden 32 E, Höhen je Kachelmitte, freie Stufen gezählt, `set_map_terrain` übersteht Speichern | 2026-11-06 |
| (1) 5 gated Studio-Test | Agent | bestanden auf allen drei: Höhen und Budget über den Befehlsweg, Setzen und Undo | 2026-11-06 |
| (1) 6 Generator-Eigenschaftstest | Agent | bestanden: 1 000 Raster, alle ≤ 19 744 B (≤ 0x9000), Abweichung an fünf Punkten je Zelle höchstens 0,0786 E (≤ 0,1 E) | 2026-11-06 |
| (1) 7 `cargo deny` | Agent | grün (`--locked check licenses advisories`) | 2026-11-06 |
| (1) 8 bis 13 fmt, clippy, Tests ohne ROM-Daten, Frontend, Styles, Tokens, `g4emu` | Agent | bestanden: alles grün im ersten Versuch (Kernel und Formate 2 292, Studio 723 + 43, Frontend 3 955, `g4emu` 407) | 2026-11-06 |
| (2) 1 bis 3 Szenarien `pt-gelaende`, `hg-gelaende`, Gegenprobe | Agent | bestanden auf Platin DE und HeartGold DE: `expect-height 16` auf dem Plateau nach der Treppe, `walk-until-blocked` + `expect-at` vor der Kante (22, 21), `map.step-unblocked` meldet die Stufe (18, 25)/(18, 24) und `block_upper` sperrt die obere Zelle (Halt auf (18, 24)), Wand hält auf (24, 26), Kampf Mewtu Lv 5 im Gras; flache BDHC: Rückgabe 2 „Höhe 0 statt 16“ (Treppen-Reihenfolge nach Entscheidung R11) | 2026-11-06 |
| (2) 4 und 5 Bilder je Station, Farben | Agent + Blick | zwölf Bilder mit richtigen Farben (vom Agenten angesehen, Farbprobe R > B gemessen); die Figur teils von Bäumen verdeckt (Entscheidung R12); Bestätigung beim Nutzer offen | 2026-11-06 |
| Gate-Text: Studio auf Platin Karte 100 | Agent | bestanden in (3) 2 bis 5: Overlay `2.000`, Marker `2.300` (2 Kacheln Höhe), `pickTile` an der Kante 10 von 10 unter dem Zeiger (Kamera gedreht, B1), Figur über die Kante gezogen landet auf der Kachel unter dem Zeiger | 2026-11-06 |
| (3) Browser-Teil und MCP über die Brücke | Agent | bestanden, 9/9 auf `731af8d2`: Karte 100 finden, Höhe und Marker, zehn Kacheln, Ziehen, Schicht „Höhen“, Budget (188 „Terrain (BDHC) 21.452 von 36.864 B“, 242 „Gebäude“ Knapp), ⌘Z nach `set_map_terrain` über `mcp.sh` (B2), `get_map_heights` / `get_map_budget` über MCP; Hinweise B1 bis B3 | 2026-11-06 |
| Blick auf Schicht „Höhen“, Overlay, Budget | Agent + Blick | 18 Bilder aufgenommen (Hell, Dunkel, Kompakt, Thema je Bild am Pixel geprüft); Bestätigung beim Nutzer offen (Abschnitt „Blick“) | 2026-11-06 |
| DSPRE öffnet die gebaute Karte | Hand | offen | – |
| `pt-gelaende` auf HeartGold US | Hand | offen (optional; `IPKE` nicht gemessen) | – |
| Frames 6.2 in der `.fig` | Agent | bestanden: zwei Frames auf „App: Werkbank“ (Reihe 3), das Brett „Budget und Höhenstufen“ auf „Komponenten“, die Zeile im Brett „Stand der App“; Branch `phase-6-fig-nachtraege` (`design-system/openpencil/NACHTRAG-6-2.md`) | 2026-11-09 |

Zahlentabelle (Release; (1) aus den gated Tests, Szenario aus (2); Last = 1-Minuten-Last aus `uptime` vor → nach den gated Tests des Spiels bzw. beim Szenario):

| Spiel | BDHC byte-gleich (Karten) | Strukturfehler / Warnungen | größte Nutzlast (Karte) | `map.step-unblocked` | Generator: größte Nutzlast / Abweichung | `expect-height 16` | Gegenprobe | Last |
|---|---|---|---|---|---|---|---|---|
| Platin DE | 666 | 0 / 0 | 21 452 B (188) | 152 (5 Karten; warm 17,5 ms) | 19 744 B / 0,0786 E (1 000 Raster, beide Familien gleich) | 16,0 auf (18, 21), 6 Treffer | Rückgabe 2, „Höhe 0 statt 16 (±0.1) bei Kachel x 18, y 21; Treffer 0 (6×)“ | gated 2,9 → 14,7; Browser 4,2 → 4,2 |
| HeartGold DE | 676 | 0 / 1 (Karte 0464) | 10 778 B (658) | 52 (9 Karten; warm 9,9 ms) | wie Platin | 16,0 auf (18, 21), 6 Treffer | wie Platin | gated 14,7 → 7,3 |
| HeartGold US | 676 | 0 / 1 (Karte 0464) | 10 778 B (658) | 52 (9 Karten; warm 10,6 ms) | wie Platin | – | – | gated 7,3 → 6,7 |

„BDHC byte-gleich“ zählt die Karten, deren BDHC sich unverändert zurückschreibt (zusammen 2 018). „Strukturfehler / Warnungen“ aus `Bdhc::validate`: die einzige Warnung ist die senkrechte Platte 0 der HeartGold-Karte 0464 (Normale y = 0; `height_at` liefert dort keinen Kandidaten). „Generator“ ist familienunabhängig (`from_grid` schreibt dieselben Bytes; die Schranke 35 592 B folgt aus der Konstruktion). Die Szenarien liefen am 2026-11-06 auf den Kopien `6-2-szen-*` und `6-2-flach-*` (Task 9; Last beim Lauf nicht festgehalten). HeartGold US hat kein Szenario (Hand, optional).

## Befunde

Keine roten Schritte in (1): jeder Schritt war im ersten Versuch grün, kein Zeittest rot, kein Fix nötig.

**V1 (bekannt, Entscheidung R7): VRAM-Schätzung zu hoch.** `vram_bytes` zählt geteilte Texeldaten doppelt; auf Vanilla liegt Gebiet 6 je Familie über dem gemessenen Vanilla-Maximum (Platin 115 648 statt 110 640 B, HGSS 89 296 statt 87 424 B) und zeigt „Knapp“ (Platin Karten 0 und 3). Median Platin 38 272 B (A §2.4: 37 744), HGSS 31 136 B (27 144). Behebung nach 6.1 (Texel-Offsets in `Nsbtx`), siehe „Offen / später“.

Aus dem Browser-Teil (keine Produktfehler, siehe (3)):

- **B1 (Hinweis für die Anleitung): Verdeckung an der 64-E-Kante.** In der Schrägsicht beim Laden (Kamera von Südosten) liegen die 32-E-Kacheln direkt an der Kante hinter dem vier Kacheln hohen Plateau; an ihrem Punkt aus `tileToClient` trifft der Strahl zuerst die 96-E-Platte, und die Szene nennt die sichtbare Kachel (`tileToClient(300,210)` → `data-hover-tile` 302,215). Das ist gewollt („höchste Platte zuerst“ in `picking.ts`); die Probe sagt aber nicht, ob ein Punkt verdeckt ist. Mit gedrehter Kamera (die untere Seite vorn) trafen 10 von 10. In [agent-gate.md](agent-gate.md) steht jetzt „Kamera so drehen, dass die untere Seite vorn liegt“.
- **B2 (Abweichung vom Gate-Text des Plans): Schreiben über `call.sh` erscheint nicht in der offenen Seite.** `call.sh set_map_terrain` (und `call.sh add_event`) ändern die Brücke (Verlauf „set terrain“, `get_map_heights` 24 E überall), die Szene zeigt das nicht, auch nach 3 s nicht; Neuladen mit `?project=` öffnet neu und verwirft ungespeicherte Schritte. Das gilt schon vor 6.2 (`call.sh` geht an der Seite vorbei). Über `mcp.sh` in derselben Sitzung zeigt die Seite die Änderung sofort, und ⌘Z stellt Overlay und Marker zurück; der Punkt (3) 8 lief darum über `mcp.sh`, die Anleitung nennt `mcp.sh`.
- **B3 (Beobachtung): Auswahl nach Agenten-Schritt.** Nach `mcp.sh set_map_terrain` und nach ⌘Z springt der Inspector von der gewählten Figur auf „Gebiet Stärkeufer“, `data-selected-marker-y` fehlt dann, bis die Figur neu gewählt wird (dann 1.800 bzw. 6.300). Ob die Auswahl erhalten bleiben soll, ist eine Produktfrage, kein gemessener Fehler; offen für eine spätere Teilphase.

Aus den Szenarien: die Abweichungen 1 bis 4 in (2) (Treppen-Reihenfolge, `equals 318`, HeartGold-Grafik 317, verdeckte Figur).

## Hand

- [ ] **DSPRE öffnet die gebaute Karte** (Windows): `Roms/_testkopien/6-2-szen-pt_DSPRE_contents/out.nds` (Platin DE, neue Karte 666 auf Header 593) und `Roms/_testkopien/6-2-szen-hg_DSPRE_contents/out.nds` (HeartGold DE, Karte 676 auf Header 540) in DSPRE öffnen; die Karte lädt (Längen der Teile stimmen, bei HGSS mit BGS-Teil), und DSPRE zeigt die BDHC-Größe (334 B, 9 Platten, 5 Streifen). Die Gegenprobe-Kopien `6-2-flach-*` haben eine flache BDHC (66 B) und eignen sich als Vergleich.
- [ ] **`pt-gelaende` auf HeartGold US** (optional): `hg-gelaende.yaml` legt `game: IPKD` fest; eine Kopie mit `game: IPKE` im Scratchpad anlegen (`sed 's/^game: IPKD$/game: IPKE/' scripts/agent/scenarios/hg-gelaende.yaml > <scratchpad>/hgus-gelaende.yaml`) und auf einer frischen Kopie von HeartGold US mit denselben `--set` wie HeartGold DE laufen lassen (`NEUER_HEADER=540`; Kacheln und Header-Zahl sind für US nicht gemessen).
- [ ] **`scripts/gate.sh` grün** auf main nach dem Merge (Hauptsitzung): `G4_ROMS_DIR=/Users/POLTEP/Projekte/lab/editor/Roms scripts/gate.sh > target/agent/gate.log 2>&1` im Hintergrund; ein roter gated Test wird erst allein wiederholt, dann nur der Rest.
- [x] **Frames 6.2 in der `.fig`** (09.11.2026, Branch `phase-6-fig-nachtraege`; in Task 10 war Port 7600 zu): „App: Welt bauen, Schicht Höhen“ (x 0, y 3000) und „App: Inspector, Karte mit Budget“ (x 1520, y 3000) auf „App: Werkbank“, das Brett „Komponenten (App): Budget und Höhenstufen“ auf „Komponenten“ (x 3440, y 16440), aus `design-system/openpencil/beispiele/hoehen-budget.body.js` (Modi `hoehen`, `budget`, `brett`) ohne Korrektur; die Symbole `heights` und `topDown` in der Symbolreihe nicht nachgezogen (keine Vorlage). Anleitung `design-system/openpencil/NACHTRAG-6-2.md`.

## Blick

Bilder im Worktree (vor `git worktree remove` sichern), aus dem UI-Test auf `731af8d2` mit Chrome headless über CDP (1440 × 900; Hell und Dunkel über `g4.theme`, Kompakt über `g4.density` mit Hell, je Neuladen; `<html data-theme data-dichte>` je Bild geprüft, Pixel der Titelleiste 231,239,247 hell gegen 33,41,57 dunkel). Vergleich mit dem Design-System: `design-system/README.md`, Abschnitt „Nachtrag 6.2“.

- [ ] `target/agent/gate-2026-11-06/6-2/blick-karte100-gelaende-{hell,dunkel,kompakt}.png`: Stärkeufer (Karte 100), Schau maximiert, Schrägsicht gedreht und an die Kante 32 → 96 E gezoomt, „Gelände“ an.
- [ ] `blick-karte100-hoehen-{hell,dunkel,kompakt}.png`: dieselbe Ansicht mit „Höhen“ an (Gelände aus): die zwei Höhen (4 und 12 Stufen, Farbe 4 und 5 der sieben cividis-Stufen) liegen farblich nah beieinander, die Isolinie trägt die Kante; prüfen, ob das genügt.
- [ ] `blick-karte100-figur-{hell,dunkel,kompakt}.png`: Objekt 4 gewählt (Inspector „Figur“, x 300, z 212), der Marker auf dem Boden der 32-E-Fläche (`data-selected-marker-y` 2.300).
- [ ] `blick-karte100-budget-{hell,dunkel,kompakt}.png`: Karte 0100 gewählt, Inspector mit „Budget“ (vier Zeilen, alle ruhig).
- [ ] `blick-karte188-budget-{hell,dunkel,kompakt}.png`: Sandgemme Haus 2, Karte 0188, „Terrain (BDHC) 21.452 von 36.864 B“ (Balken 58 %).
- [ ] `blick-karte242-budget-{hell,dunkel,kompakt}.png`: Wettbewerbshalle, Karte 0242, „Gebäude“ mit Plakette „Knapp“ 32 von 32.
- [ ] `g4emu`-Bilder `target/agent/gate-2026-11-06/6-2/{pt-gelaende,hg-gelaende}/{geschenk,treppe,kante,stufe,wand,gras}.png`: Farben richtig (kein Rot-Blau-Tausch); die Figur ist in HeartGold in fünf Bildern unter dem Wald von Route 29 verdeckt, in Platin bei `wand` verdeckt und bei `stufe` nur mit dem Kopf zu sehen (Abweichung 4 in (2)); `gras` zeigt den Kampf „Ein wildes MEWTU erscheint!“.
- [ ] Farbprobe `target/agent/6-2/farbe/zimmer.png`: der Holzboden des Zimmers braun (R 140,1 > B 104,9).

## Offen / später

- **VRAM-Schätzung zählt doppelt** (Entscheidung R7): geteilte Texeldaten (216 / 175 Texturen) zählen in `vram_bytes` zweimal, so liegt die Schätzung über Gutachten A §2.4: Platin Median 38 272 statt 37 744 B, Maximum 115 648 statt 110 640 B (Gebiet 6); HGSS 31 136 statt 27 144 B, 89 296 statt 87 424 B (Gebiet 6). Folge: auf Vanilla zeigt Gebiet 6 je Familie „Knapp“. Die Entdopplung über gleiche Texel-Offsets kommt, sobald 6.1 sie in `Nsbtx` zeigt ([terrain.md](formats/terrain.md)).
- **`area_of_map`:** `crates/g4-kernel/src/terrain/budget.rs` bringt eine eigene kleine `area_of_map` mit, weil `map_import.rs` unberührt bleibt; nach dem Merge von 6.1 und 6.2 ruft sie `map_areas` von 6.1 (ein Commit „refactor“ auf main).
- **Werkzeugzahl:** der Katalog-Test pinnt genau 87 Werkzeuge (Grenze 90); bringt 6.1 Werkzeuge mit, hebt der Merge die Zahl mit Begründung im Kommentar des Tests.
- **`mcp/docs.rs`:** `FORMATS` hat die Einträge `bdhc` und `terrain` (sortiert eingefügt, Entscheidung R6); 6.1 und 6.3a fügen dort ebenfalls ein, eine kleine Merge-Stelle.
- **Merge mit 6.3a:** Konflikte erwartet in `apps/studio/src/world/scene/picking.ts`, `content.ts` und `SceneHost.ts` (die Felder `ground` und `heights`) und in den sechs Test-Fixtures mit `heights: []`; `Ground` ist die Schnittstelle, die 6.3a für Welt-Karten aus dem Weltdokument füllt. Dazu die Anhängepunkte `dispatch.rs`, `lib.rs`, `access.rs`, `catalog.rs`, `api.ts`, `icons.ts`, `docs/mcp.md`, `ROADMAP.md`, `agent-gate.md`, `design-system/README.md`.
- **Koordinaten in `expect-height`:** `read_location` liefert die Kachel der ganzen Matrix; in der 1 × 1-Matrix der Szenarien ist das die Kartenkachel. Ob das Spiel den Vektor der Figur auch in größeren Matrizen in Matrix-Koordinaten hält, prüft erst ein späteres Gate (6.3b auf einer Außenkarte).
- **Ränder zur Nachbarkarte:** die Ableitung lässt Kanten über den Kartenrand frei; eine gemalte Karte neben einer höheren Nachbarkarte braucht Handwerte, bis 6.4b die Naht liest.
- **HGSS-Gebäudegrenze:** `buildings.max_loaded` (32) gilt für beide Familien, belegt nur für Platin.
- **Kleine Punkte aus den Reviews** (aufgeschoben, im Arbeitsordner der Phase): u. a. Entwurf-Schicht folgt keinem Höhen-Neuladen, ein gerade gezogener Marker springt bei einem Neuladen mitten im Ziehen zurück, `moveMapGround` verschiebt Warp-Marker zweimal, Importzyklus `content.ts` ↔ `heightLayer.ts` (nach dem Merge mit 6.3a entflechten), `{height:g}` in `expect-height` verliert ab 1 000 E Stellen.
