# Phase 6.3a: Gate prüfen

Stand: 2026-11-06. Branch `phase-6-3a-weltformat` (Worktree `.claude/worktrees/phase-6-3a`), nicht nach main gemergt; Kopf beim Gate: `dfdeb0e9` (Tasks 0 bis 13, je von der Review-Schleife abgenommen). Basis `d515c5ee` (main `d05fad84` plus die Plan-Dokumente), Task 10 holte main `f810fdc4` (6.1 Writer, 6.2 BDHC) mit dem Merge `5d7f5cc3` herein. **Das Gate lief am 2026-11-06** aus den Läufen der Tasks: die gated Tests liefen in Task 5 und Task 8 je Projekt im Release-Profil, der Browser-Teil in Task 11, die Messung in Task 0; Task 14 wiederholte `cargo deny` und die zwei Zeittests (Last unten). `scripts/gate.sh` als Ganzes ist **offen: Lauf in der Hauptsitzung** (der Koordinator hat den Lauf im Worktree untersagt); Task 14 hängt den Schritt an und prüft nur die Syntax (`bash -n`).

Phase 6.3a bringt das Weltformat und seine Vorschau: eine Karte des Textprojekts kann `source: "world"` mit dem Objekt `world` tragen (Ebenen als Zeilen, Höhen, Rampen, Teilmodell-Fußabdrücke, Permissions; [world_map.md](formats/world_map.md)), die Tilesets `data/tilesets/NNNN.json` je Kartentextur-Paket entstehen aus dem Paket und den Vanilla-Permissions ([tileset.md](formats/tileset.md)), ein synthetisches Test-Tileset entsteht aus Code. Die Store-Art `WorldMap` legt eine Welt an und ändert sie als je einen Undo-Schritt; der Vernetzer macht daraus eine Flächenliste, die als Netz-DTO über die Brücke in die Szene geht. Schemaversion des Textprojekts 3 (Migration `maps.world` ändert nur das Manifest). Der Member der Karte bleibt bis 6.3b der bisherige; jeder Build bleibt byte-gleich.

Roadmap: [docs/ROADMAP.md](ROADMAP.md), Abschnitt „Phase 6“, Zeile „6.3a“. Spezifikation: [2026-11-05-phase-6-kartenerstellung-design.md](superpowers/specs/2026-11-05-phase-6-kartenerstellung-design.md), §4 Zeile 6.3a. Plan: [2026-11-05-phase-6-3a-weltformat.md](superpowers/plans/2026-11-05-phase-6-3a-weltformat.md). Messung: [task0-6-3a-vorgaben.md](research/2026-11-05-phase-6/task0-6-3a-vorgaben.md). Formate: [world_map.md](formats/world_map.md), [tileset.md](formats/tileset.md), [text_project.md](formats/text_project.md) (Schema 3); Befehle: [mcp.md](mcp.md), „Welt-Karten (6.3a, über `g4_call`)“; Design: [design-system/README.md](../design-system/README.md), „Nachtrag 6.3a“.

Gate-Text der Spezifikation (§4, Zeile 6.3a):

> Für jedes Kartentextur-Paket von Platin DE (74) und HeartGold DE (106) leitet der Kernel ein Tileset ab, dessen Bodenkacheln die Zahl der Texturen treffen (3 130 / 3 659); Task 0 misst die Eindeutigkeit der Vorgaben; eine 32 × 32-Karte aus einer Kachel vernetzt zu ≤ 4 Flächen, ein Schachbrett aus zwei Kacheln zu ≤ 1 024; Vernetzen einer vollen Karte ≤ 20 ms im Release-Build, Netz über die Brücke ≤ 50 ms im Debug-Build; Export idempotent, Round-Trip der JSON byte-gleich, Migration `maps.world` ändert keine andere Datei; ⌘Z nimmt einen Strich (über einen Kernel-Befehl) als Ganzes zurück; die Szene zeigt das Netz einer Welt-Karte mit `[data-world-mesh]`

Dazu (Plan, Global Constraints): gated Tests auf frischen Kopien von Platin DE, HeartGold DE und HeartGold US, Browser-Teil über die Test-Umgebung, `scripts/gate.sh` grün, `cargo deny` grün, Hand und Blick beim Nutzer.

So gelesen (Rulings der Umsetzung, Ledger `.superpowers/sdd/2026-11-05-phase-6-3a-weltformat/progress.md`):

- **Vorgaben:** `DEFAULT_RULE = K1`, `UNIQUE_SHARE = 0,6` (Task 0, Platin DE und HeartGold DE zusammengezählt 72,1 %); abgeleitete Kollision ist 0 für jede abgeleitete Kachel (die Mehrheitsregel 0x80 traf nur 44,5 % / 64,0 %); Palettenregel `<name>_pl` (auf 16 Byte gekürzt), sonst gleichnamig, sonst Palette mit dem Index der Textur, sonst keine.
- **„Ein Strich“** ist ein `world_edit` mit mehreren Änderungen (Kernel-Befehl über `g4_call`); ⌘Z nimmt ihn als Ganzes zurück (Browser-Teil, (3)).
- **Gated Tests auf Kopien, nie auf Originalen:** die nur lesenden Läufe (Task 5) auf den unveränderten Task-0-Kopien `6-3a-task0-{pt,hgde,hgus}`, die mit Basis-ROM (Task 8) auf `6-3a-t8-{pt,hgde,hgus}_DSPRE_contents` mit `.nds` daneben.
- **Ein gesperrter frischer Eintrag mit Welt** lehnt ab (`textproject.map-source`), statt die Welt fallen zu lassen; „Auf Original zurücksetzen“ einer Welt lehnt mit dem 6.3b-Satz ab, `reload_from_disk` liest die JSON neu.
- **Keine kuratierten MCP-Werkzeuge** in 6.3a: die sieben Befehle gehen über `g4_call` (`--expert`), der Katalog bleibt.

Wer was prüft: Die Spalte „Wer“ sagt **Agent** (die Test-Umgebung aus [docs/agent-gate.md](agent-gate.md): Browser-Fenster mit `studio-bridge`, `call.sh`, `mcp.sh`, dazu die gated Tests), **Agent + Blick** (der Agent prüft und beschreibt Ort und Zahlen, der Nutzer bestätigt kurz am Bild) oder **Hand** (der Nutzer). Befehle: [docs/agent-gate.md](agent-gate.md), Abschnitt „Phase 6.3a“.

Namen in der Oberfläche (für die Prüfung):

- **Szene:** die Leinwand trägt bei einer Welt-Karte `canvas[data-world-mesh="<Karte>"]` und `[data-world-mesh-faces="<Zahl der Flächen>"]`; eine Karte ohne Welt zeigt ihr GLB und keines der beiden Attribute. Nach ⌘Z des Anlegens fragt der Refresh das Netz neu: leer → GLB, beide Attribute weg.
- **Verlauf:** „Welt angelegt: Karte NNNN“, „Welt geändert: Karte NNNN (N Zellen)“; der Zähler „Verlauf, N Schritte“.
- **Befehle** (über `g4_call`, `--expert`): `world_get`, `tileset_get`, `get_world_mesh`, `get_tileset_texture` (Lesen), `world_create`, `world_edit`, `tileset_save` (Schreiben). `get_map` trägt `world: true|false`.
- **Prüfungen** im Textprojekt-Check: `world.schema`, `world.tileset-mismatch`, `world.not-baked`, `world.budget` (Wortlaut in [world_map.md](formats/world_map.md)).

## Vorbereitung

Durchgeführt 2026-11-05 bis 2026-11-06 (Befehle: [docs/agent-gate.md](agent-gate.md), „Phase 6.3a“):

1. **Kopien** unter `$HOME/Projekte/lab/editor/Roms/_testkopien/` per `cp -cR` (Ordner) und `cp -c` (`.nds`), die Originale nur gelesen:
   - `6-3a-task0-pt`, `6-3a-task0-hgde`, `6-3a-task0-hgus` (ohne `_DSPRE_contents`, ohne `.nds`): Task 0 und die nur lesenden gated Tests von `tests/world.rs` (Task 5).
   - `6-3a-t8-pt_DSPRE_contents`, `6-3a-t8-hgde_DSPRE_contents`, `6-3a-t8-hgus_DSPRE_contents` je mit `6-3a-t8-<spiel>.nds` daneben: Round-Trip, Export und Migration (Task 8; geschrieben wird nur auf `vanilla_copy()`).
   - `6-3a-web_DSPRE_contents` (Platin DE) mit `6-3a-web.nds`: Browser-Teil (3), eingerichtet über `call.sh open_project`, `assign_base`, `adopt_text_project` (85 s, 6 577 Dateien).
2. **Worktree:** `G4_ROMS_DIR=$HOME/Projekte/lab/editor/Roms` absolut; `tools/` (bzw. `tools/apicula`) als Symlink auf den Haupt-Checkout, nie einchecken; `export PATH="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$PATH"` vor jedem Cargo-Aufruf.
3. **Test-Umgebung** für (3) auf **1460/1461**: `G4_VITE_PORT=1460 G4_BRIDGE_PORT=1461 bash scripts/agent/studio-web.sh` (Log `target/agent/6-3a-web.log`); andere Ports nicht anfassen.
4. **Ausgaben** im Worktree (nicht im Repo): Logs `target/agent/6-3a-*.log`, Browser-Notiz und JSON-Stände unter `target/agent/gate-2026-11-06/6-3a/`. Den Ordner vor `git worktree remove` sichern.
5. **Last:** `uptime` vor und nach den Läufen von Task 14 (`target/agent/6-3a-t14-load.log`). Die Maschine teilten sich während der Tasks bis zu drei Worktrees (6.1, 6.2, 6.3a); die Zeiten der Tasks sind unter Last gemessen, ihre Last ist nicht notiert. Zeitgrenzen gelten nur mit `G4_TIMING_STRICT=1`; ein roter Zeittest unter Last wird allein wiederholt, die Grenze nie gelockert.

## (1) Automatik

- [ ] 1. `scripts/gate.sh` grün, mit den zwei neuen Schritten am Ende: `world mesh (timing)` (Release) und `world mesh bridge (timing)` (Debug), die gated Tests von `tests/world.rs` laufen im bestehenden Schritt je Projekt mit. **Offen: Lauf in der Hauptsitzung.** Statt dessen liefen einzeln (alle Rückgabe 0): `cargo test --workspace --all-features` ohne `G4_*` auf `6a58f17d` (Task 10: 3 375 bestanden, 0 fehlgeschlagen; darin `one_tile_map_meshes_to_one_face`, `checkerboard_meshes_to_1024_faces`, `prop_world_json_round_trips_byte_equal`, `v2_to_v3_changes_only_the_manifest`, `world_create_and_edit_are_one_undo_step_each`), `pnpm test` auf `4f115084` (Task 11: 417 Dateien, 3 970 bestanden, 1 übersprungen), `cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings` und `check-no-rom-data.sh` vor jedem Commit (Tasks 1 bis 13), Tokens, Styles, Tiefe in Task 11; `bash -n scripts/gate.sh` in Task 14.
- [x] 2. **Gated Kernel-Tests** je Projekt im Release-Profil, `--nocapture --test-threads 1`:
   - `vanilla_tilesets_have_one_ground_tile_per_texture` und `vanilla_vote_shares_match_task0` (`crates/g4-kernel/tests/world.rs`, Task 5 auf `6-3a-task0-*`): druckt `tilesets <Spielcode> packages=… tiles=… vanilla=… unclear=… ms=…`. Ergebnis: Platin DE `packages=74 tiles=3130 vanilla=1765 unclear=1365 ms=6470`, HeartGold DE `106 / 3659 / 2612 / 1047, ms=7751`, HeartGold US `106 / 3659 / 2612 / 1047, ms=5988`; die Anteile `vote-shares` (Median 0,951 bzw. 0,918) passen zu Task 0.
   - `textproject_round_trip_with_world`, `textproject_export_idempotent_with_world` (`tests/textproject.rs`) und `vanilla_migration_v2_to_v3_changes_only_the_manifest` (`tests/textproject_migrate.rs`), Task 8 auf `6-3a-t8-*`: Platin DE Welt auf Karte 0000, Paket 0006 (78 Kacheln), HeartGold DE und US Karte 0000, Paket 0002 (67 Kacheln); Round-Trip byte-gleich, zweiter Export ohne Änderung, Migration ändert nur das Manifest (6 577 bzw. 6 305 Quellen). `textproject-stats` Platin DE `world_ms=4625 export_ms=163288 import_ms=123636 build_ms=48738`, HeartGold DE `world_ms=4385 build_ms=50499`, HeartGold US `world_ms=4389 build_ms=50247`.
   - **Stand der Läufe:** die gated Tests von `tests/world.rs` liefen auf `d748f5c1` (Task 5), Round-Trip, Export und Migration auf `48b0ad0b` (Task 8; die Fix-Runde bis `f390e740` brachte nur einen Negativtest und Doku, ohne neuen gated Lauf). Beide Stände liegen **vor** dem Merge von main (`5d7f5cc3`, 6.1 Writer, 6.2 BDHC) und vor der Rampen-Umrechnung zu 6.2 (`6a58f17d`, Lexer der Rampen). Für den Code des Merges sind sie Indiz, kein Nachweis: **maßgeblich vor dem Merge nach main ist der `scripts/gate.sh`-Lauf der Hauptsitzung mit seinen gated Schritten je Projekt.**
- [x] 3. **Zeittests** mit `G4_TIMING_STRICT=1`, in Task 14 wiederholt (Last 8,8 bis 13,4):
   - `mesh_of_a_full_map_is_fast` (Release, Grenze 20 ms): `world-mesh faces=1866 median_us=107` (Task 3: 83 µs).
   - `world_mesh_over_the_command_path_is_fast` (`-p studio`, Debug, Grenze 50 ms): `world-mesh-bridge median_ms=9.56 faces=1802` (Task 9: 6,71 ms).
- [x] 4. `cargo deny check` (Task 14): `advisories ok, bans ok, licenses ok, sources ok`. 6.3a bringt keine neue Abhängigkeit.

## (2) Messung

- [x] Task 0 ([task0-6-3a-vorgaben.md](research/2026-11-05-phase-6/task0-6-3a-vorgaben.md)), Lauf auf `6-3a-task0-{pt,hgde,hgus}`: 74 Pakete / 3 130 Texturen (Platin DE), 106 / 3 659 (HeartGold DE und US, US in jeder abgeleiteten Zahl gleich DE). K1 bei 0,6: Platin 72,1 % (400 von 555 Texturen mit Wahrheit), HeartGold 72,1 % (562 von 780); zusammen 72,1 % gegen K2 71,3 %. **Wahl: K1, `UNIQUE_SHARE` 0,6.** Grenze: K1 nennt fast nur `kind 0` (Wahrheit ≠ 0 richtig 1 von 156 bzw. 4 von 202); Gras- und Wasser-Rollen setzt der Nutzer im Tileset-Editor (6.4a). Die Gegenprobe im Kernel (Task 5) trifft dieselben Zahlen: 1 765 / 1 365 bzw. 2 612 / 1 047 vanilla / unklar.

## (3) Browser

- [x] Task 11 Step 5 auf `6-3a-web` (Platin DE), Test-Umgebung 1460/1461, 800 × 600, Notiz `target/agent/gate-2026-11-06/6-3a/browser.md`:
  1. Zweiblattdorf (Header 411, Matrix 0, Zelle 3, 27 = Karte 0000): GLB, kein `data-world-mesh`.
  2. `g4_call world_create {"map":0}` → `{"map":0,"tileset":6,"cells":1024,"change":1}`; `data-world-mesh="0"`, `data-world-mesh-faces="1"`; Verlauf „Welt angelegt: Karte 0000“.
  3. `g4_call world_edit` (4 Zellen Höhe 2, 3 Zellen Kachel 0001) → `{"cells":7,"change":2}`; Faces 11; „Welt geändert: Karte 0000 (7 Zellen)“, „Verlauf, 2 Schritte“.
  4. ⌘Z → Faces 1; `call.sh world_get {"map":0}` byte-gleich mit dem Stand nach dem Anlegen (`cmp` identisch; Schätzung 1 680, Anteil 0,0273).
  5. ⇧⌘Z → Faces 11. Hell und Dunkel, keine Konsolenfehler.
  6. Wiederholung nach der Fix-Runde (`4f115084`): ⌘Z bis vor das Anlegen → beide Attribute weg, GLB; ⇧⌘Z ⇧⌘Z direkt hintereinander → Faces 11.

## (4) Hand und Blick

- [ ] **Blick** (Agent + Blick): Szene einer Welt-Karte (Textur, Wand, Rampe) in Hell und Dunkel. Vom Agenten beschrieben (Task 11): die Mitte ist eine ebene Fläche mit der Paket-Textur (keine Platzhalterfarbe), darüber das Permissions-Overlay; nach `world_edit` am Nordrand ein Streifen mit anderer Textur und bei Zellen 4–5 / 4–5 ein um eine Kachel erhöhter Block mit dunkler Wand. Eine Rampe war nicht im Browser-Lauf (die Vernetzer-Tests decken Rampen ab). Beim Nutzer offen.
- [ ] **Hand:** das Textprojekt mit einer Welt-Karte in einem älteren Studio öffnen (`schemaVersion` 3 → nur lesen). Beim Nutzer offen.

## Ergebnis

| Punkt | Wer | Ergebnis | Datum |
|---|---|---|---|
| (1) 1 `scripts/gate.sh` | Agent | **offen: Lauf in der Hauptsitzung**; Schritte angehängt, `bash -n` ok; die Teile einzeln grün (Workspace 3 375, pnpm 3 970, fmt, clippy, keine ROM-Daten) | 2026-11-06 |
| (1) 2 gated `tests/world.rs` | Agent | bestanden auf `d748f5c1` (vor dem Merge `5d7f5cc3` und `6a58f17d`): 74 / 3 130, 106 / 3 659, 106 / 3 659; maßgeblich: `scripts/gate.sh` der Hauptsitzung | 2026-11-05 |
| (1) 2 gated Round-Trip, Export, Migration | Agent | bestanden auf allen drei Spielen auf `48b0ad0b` (Fix-Runde bis `f390e740` ohne neuen Lauf; vor dem Merge `5d7f5cc3` und `6a58f17d`); maßgeblich: `scripts/gate.sh` der Hauptsitzung | 2026-11-06 |
| (1) 3 Zeittests | Agent | bestanden: Netz 107 µs Median (≤ 20 ms), Brücke 9,56 ms (≤ 50 ms); Last 8,8 bis 13,4 | 2026-11-06 |
| (1) 4 `cargo deny` | Agent | bestanden | 2026-11-06 |
| (2) Task 0 | Agent | K1 0,6, 72,1 % | 2026-11-05 |
| (3) Browser | Agent | bestanden: Faces 1 → 11 → ⌘Z 1 → ⇧⌘Z 11, `world_get` nach ⌘Z byte-gleich, Hell und Dunkel | 2026-11-06 |
| (4) Blick | Agent + Blick | vom Agenten beschrieben, beim Nutzer offen | – |
| (4) älteres Studio | Hand | offen | – |

Zahlentabelle (Zeiten der Tasks unter Last ohne notierte Zahl; Task 14 mit Last):

| Spiel | Pakete / Kacheln | vanilla / unklar | Regel (Task 0) | Übereinstimmung | Netz Release (ms) | Brücke Debug (ms) | Round-Trip | Export 2. Lauf | Migration | Last |
|---|---|---|---|---|---|---|---|---|---|---|
| Platin DE | 74 / 3 130 | 1 765 / 1 365 | K1, 0,6 | 72,1 % (400 / 555) | 0,107 (Task 3: 0,083), synthetisch | 9,56 (Task 9: 6,71), synthetisch | byte-gleich, Karte 0000, Paket 0006 | gleich (215 s) | nur Manifest (6 577 Quellen, 13 s) | Task 14: 8,8 bis 13,4 |
| HeartGold DE | 106 / 3 659 | 2 612 / 1 047 | K1, 0,6 | 72,1 % (562 / 780) | wie oben | wie oben | byte-gleich, Karte 0000, Paket 0002 | gleich (215 s) | nur Manifest (6 305, 13 s) | – |
| HeartGold US | 106 / 3 659 | 2 612 / 1 047 | K1, 0,6 | wie HeartGold DE | wie oben | wie oben | byte-gleich, Karte 0000, Paket 0002 | gleich (215 s) | nur Manifest (6 305, 13 s) | – |

Die Zeittests rechnen auf synthetischen Daten (`busy_world`, 1 866 bzw. 1 802 Flächen) und gelten für alle Spiele.

## Befunde

Keine offenen Befunde aus dem Gate. Befunde der Tasks sind in ihren Fix-Runden behoben (Ledger): Task 4 (gesperrte Karte), Task 5 (Stub-Paket), Task 6 (Diff-Subjekt), Task 8 (Paket aus den Basis-Zählungen, Negativtest), Task 11 (Markierungs-Rennen, veraltetes Flag beim Refresh). Aufgeschobene Kleinigkeiten stehen im Ledger.

## Offen

- `scripts/gate.sh` als Ganzes mit seinen gated Schritten (Hauptsitzung, vor dem Merge nach main): die gated Läufe der Tasks 5 und 8 liegen vor dem Merge von main und vor `6a58f17d`; erst dieser Lauf belegt den Stand, der gemergt wird.
- Blick auf die Szene einer Welt-Karte in Hell und Dunkel; das Textprojekt in einem älteren Studio (Nutzer).
- Aus der Umsetzung für 6.3b und später: `get_map_heights` liest für Welt-Karten noch die BDHC des Members, Vorschaubilder gemalter Karten kommen weiter aus dem GLB, `From<MeshFace> for bake::Face`.
