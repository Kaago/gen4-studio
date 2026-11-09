# Phase 6.5a: Gate prüfen

Stand: 2026-11-06. Branch `phase-6-5a-import` (Worktree `.claude/worktrees/phase-6-5a`, Basis `f810fdc4` = main mit dem Plan 6.5a), nicht nach main gemergt. Vom Agenten erledigt: (1) Automatik Schritt für Schritt auf dem Stand `90e105b9` (`cargo deny`, fmt, clippy, die Tests von `g4-formats` und `g4-kernel` ohne ROM-Daten, `studio`, Frontend, Stil, Tokens, ROM-Daten, `g4emu`; die gated Tests `model_import` und `bake` im Release-Profil auf Platin DE, HeartGold DE und HeartGold US aus Task 8), (2) die Unit-Punkte des Gates, (3) die Szenarien `pt-import` und `hg-import` (Task 10) mit dem Blick des Agenten auf die vier Bilder und (4) der Browser-Teil über die Test-Umgebung mit sechs Bildern. `scripts/gate.sh` als Ganzes läuft hier nicht (Vorgabe des Nutzers), sondern in der Hauptsitzung auf main nach dem Merge.

Beim Nutzer offen: der Blick auf die vier `g4emu`-Bilder und die sechs Browser-Bilder; in der gebündelten App „Dateien wählen…“ mit einer eigenen Blender-Datei (`.glb` und `.gltf`) und, wer PDSMS hat, mit einem echten „Export as OBJ with Textures“; `docs/formats/model_import.md` gelesen; `scripts/gate.sh` auf main (Abschnitt „(5) Beim Nutzer (Hand und Blick)“).

Phase 6.5a bringt den Import von PNG und Modellen: im Kernel das Modul `model_import` (neutrales Netz `Mesh` → Flächenliste des Bäckers mit Ohrenschneiden und Dreieckspaarung, OBJ und MTL über `tobj`, glTF und GLB über `gltf` mit Knoten-Transformen, beide auf den Modellordner begrenzt, Texturplanung aus Bildern und PNG-Dateien für das Kartenpaket, synthetische Beispiele und `g4k import-samples`), im Bäcker Dreiecke als Flächen, in `map_import` die Teile „Modell“ aus OBJ/glTF und „Texturen“ aus PNG mit Vorschau (Budget, Texturliste, VRAM des Gebiets) und Import als ein Undo-Schritt, im Dialog „Teile importieren…“ die Zeile „Umwandeln“ (Farben je Textur 16/4, Einheit), in `g4emu` den Vorbereitungsschritt `make-import-samples` und die Szenarien `pt-import`/`hg-import`, die Formatdoku [docs/formats/model_import.md](formats/model_import.md) und den Abschnitt „Import aus PNG“ in [docs/formats/nsbtx.md](formats/nsbtx.md). Neue Abhängigkeiten: `tobj` 4.0.5 und `gltf` 1.4.1.

Gate-Text der Spezifikation (§4, Kopf der Gate-Spalte und Zeile 6.5a):

> Gate (zusätzlich: gated Tests auf frischen Kopien von Platin DE, HeartGold DE und HeartGold US; Browser-Teil über die Test-Umgebung; `scripts/gate.sh` grün; `cargo deny` grün; Hand und Blick beim Nutzer)
>
> Ein 64 × 64-PNG mit 40 Farben wird 16-Farben-NSBTX, zurückdekodiert mit Farbfehler ≤ 1 BGR555-Stufe je Kanal für Palettentreffer; eine Würfel-OBJ und eine Ebene-glTF (im Test erzeugt) werden Modell-Teil einer Karte, apicula liest sie, Ablehnung über 0xF000 mit Zahl; eine OBJ im PDSMS-Stil (Vierecke + MTL) wird importiert; `cargo deny` grün mit `tobj` und `gltf`

**Zusatzpunkt der Hauptsitzung** (nicht in der Spezifikation, Abschnitt „Abweichungen“): Würfel und Ebene in `g4emu` sichtbar.

Wer was prüft: Die Spalte „Wer“ sagt **Agent** (die Test-Umgebung aus [docs/agent-gate.md](agent-gate.md): `studio mcp` kopflos, `g4emu`, die gated Tests und die Browser-Test-Umgebung), **Agent + Blick** (der Agent prüft und beschreibt Ort und Zahlen, der Nutzer bestätigt kurz am Bild) oder **Hand** (der Nutzer). Befehle: [docs/agent-gate.md](agent-gate.md), Abschnitt „Phase 6.5a“.

## Vorbereitung

1. **Kopien** unter `Roms/_testkopien/` per `cp -cR` (Ordner) und `cp -c` (`.nds`) aus den Originalen, die dabei nur gelesen wurden:
   - für (1): `6-5a-pt_DSPRE_contents` (Platin DE), `6-5a-hgde_DSPRE_contents` (HeartGold DE), `6-5a-hgus_DSPRE_contents` (HeartGold US), angelegt in Task 8 ohne `.nds`. Sie werden nur gelesen; die Tests schreiben nur auf `vanilla_copy()`.
   - für (3): `6-5a-szen-pt2_DSPRE_contents` + `6-5a-szen-pt2.nds` (Platin DE, grüner Lauf), `6-5a-szen-hg_DSPRE_contents` + `6-5a-szen-hg.nds` (HeartGold DE); `6-5a-szen-pt` + `.nds` war der erste, am Exportpfad gescheiterte Lauf (nichts gespeichert). Nach dem Lauf sind die Kopien gespeichert und gebaut; ein neuer Lauf braucht neue Kopien (`6-5a-szen-pt3`, `6-5a-szen-hg2`).
   - für (4): `6-5a-web_DSPRE_contents` + `6-5a-web.nds` (Platin DE), frisch in Task 11, mit `target/debug/g4k import-samples <Kopie>/import` (14 Dateien). Im Browser wurde importiert und mit ⌘Z zurückgenommen, nichts gespeichert.
2. **Werkzeuge:** `export PATH="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$HOME/.cargo/bin:$PATH"`; `tools/` ist der Symlink auf den Haupt-Checkout (nie einchecken); `G4_ROMS_DIR=/Users/POLTEP/Projekte/lab/editor/Roms` und `G4_APICULA=/Users/POLTEP/Projekte/lab/editor/gen4-studio/tools/apicula/bin/apicula` absolut. Für (3) und (4) `cargo build --workspace --all-features` (Debug-`studio`, `studio-bridge` und `g4k`).
3. **Ausgaben** im Worktree: Logs unter `target/agent/6-5a/` (`t11-<schritt>.log` für (1) mit `uptime` und Befehl im Kopf und der Rückgabe am Ende, Übersicht `t11-summary.log`; `t8-{pt,hgde,hgus}.log` und `t8-{pt,hgde,hgus}-bake.log` für die gated Tests; `t10-{pt,hg}.log` für (3); `studio-web.log`, `t11-blick.log` und das Bildskript `t11-blick.mjs` für (4)); die Bilder unter `target/agent/gate-2026-11-06/6-5a/` (`pt-import/`, `hg-import/` mit `zimmer.png`, `unten.png`; die sechs `blick-import-*.png`). Den Ordner `target/agent/` vor `git worktree remove` sichern.
4. **Last:** `uptime` vor jedem Schritt (Spalte „Last“ der Zahlentabelle). Auf dem Rechner liefen gleichzeitig andere Sitzungen; die drei Spiele liefen nacheinander, nie parallel.

## (1) Automatik

Nach Vorgabe des Nutzers läuft hier nicht `scripts/gate.sh` als Ganzes (das macht die Hauptsitzung auf main nach dem Merge), sondern die Schritte einzeln. Die neuen Tests laufen in `scripts/gate.sh` ohne Änderung am Skript mit: die Unit- und Integrationstests ohne ROM-Daten im Schritt „cargo test (no ROM data)“, `model_import` und `bake` im Schritt „gated tests (cargo test)“ je Projekt (Debug).

Task 11, 2026-11-06 ab 18:04 auf `90e105b9` (Logs `target/agent/6-5a/t11-<schritt>.log`, Übersicht `t11-summary.log`):

- [x] 1. `cargo deny --locked check licenses advisories`: „advisories ok, licenses ok“ (Last 13,9). Neue Crates im Lockfile gegenüber main: `tobj` 4.0.5 (MIT), `gltf`, `gltf-json`, `gltf-derive` 1.4.1 (MIT OR Apache-2.0), `inflections` 1.1.1 (MIT), `lazy_static` 1.5.1 (MIT OR Apache-2.0); `base64` 0.22.1 war schon im Lockfile.
- [x] 2. `cargo fmt --all --check` und `cargo clippy --workspace --all-features --all-targets -- -D warnings`: sauber.
- [x] 3. `env -u G4_TEST_PROJECT cargo test -p g4-formats -p g4-kernel --all-features` (mit `G4_APICULA`, also laufen die apicula-Tests mit): 2 489 bestanden in 59 Testzielen, 0 fehlgeschlagen, 1 ignoriert; Bibliothek `g4-formats` 267, Bibliothek `g4-kernel` 1 333, `tests/model_import.rs` 5, `tests/bake.rs` 8, `tests/cli.rs` 66 (die gated Tests überspringen sich ohne `G4_TEST_PROJECT`); 18:05 bis 19:45, Last 14,2 beim Start (die Bibliothek `g4-kernel` allein 3 113 s). `-p g4-kernel` lud im Worktree nichts nach.
- [x] 4. `cargo test -p studio`: 775 bestanden in 6 Zielen, 0 fehlgeschlagen, 1 ignoriert (Bibliothek 732 in 577 s), Last 27.
- [x] 5. `(cd apps/studio && pnpm test && pnpm build)`: 3 972 bestanden, 1 übersprungen in 417 Dateien (416 bestanden, 1 übersprungen); `pnpm build` gebaut, Last 21.
- [x] 6. `bash scripts/check-styles.sh`, `bash scripts/check-tokens.sh`, `bash scripts/check-no-rom-data.sh`: „check-styles: ok“, „Alles stimmig.“, „no rom data: 2028 files ok“ (Last 19).
- [x] 7. `(cd scripts/g4emu && uv run pytest -m "not integration" -q)`: 404 bestanden, 7 abgewählt (Last 19).
- [x] 8. **Gated `--test model_import`** (Release, `--workspace --all-features`, mit `G4_APICULA`, `--nocapture --test-threads 1`): je 5/5 auf Platin DE (81 s, Last 5,7), HeartGold DE (91 s, Last 6,7), HeartGold US (138 s, Last 4,5).
- [x] 9. **Gated `--test bake`** (Release, mit `G4_APICULA`): je 8/8 (unverändert grün) auf Platin DE (23 s, Last 7,6), HeartGold DE (22 s, Last 5,1), HeartGold US (74 s, Last 12,2).
- [ ] 10. `scripts/gate.sh`: offen, läuft in der Hauptsitzung auf main nach dem Merge.

Die Punkte 8 und 9 sind die Läufe von Task 8 (Commit `23562ea0`, Logs `t8-<spiel>.log`, `t8-<spiel>-bake.log`), nicht in Task 11 wiederholt: seit `23562ea0` hat sich unter `crates/` nichts geändert (`git diff --stat 23562ea0..HEAD -- crates` leer; danach kamen nur Oberfläche, Design-Nachtrag und `g4emu`), die Läufe lagen nur vor einer reinen `cargo fmt`-Umformung dreier Anweisungen in den Testdateien desselben Commits, und die Last lag in Task 11 laut den `uptime`-Zeilen der Logs bei 13,9 bis 29,2. Die Kopien `6-5a-{pt,hgde,hgus}` sind unbeschrieben und für eine Wiederholung bereit. Beim Release-Bau warnt cargo „stripping debug info with `rust-objcopy` failed“ (Umgebung, bekannt aus 6.1).

Belegzeilen der gated Tests (aus `t8-*.log`, wörtlich):

```
import-room CPUD cube map=187 size=672 share=0.011 quads=6 plane map=186 size=472 quads=1 tris=0 pack=20 appended=g4_imp_cube,g4_imp_plane vram=21744 warn=- ms=214
png40-room CPUD pack=20 hits=8/40 max_step=3 mean=12.8203125
import-room-apicula CPUD map=187 faces=6 corners=20
import-room-apicula CPUD map=186 faces=1 corners=4
oversize-room CPUD map=187 size=164272 limit=61440
pdsms-room CPUD map=187 size=3164 quads=64 new=2
import-room IPKD cube map=217 size=672 share=0.011 quads=6 plane map=216 size=472 quads=1 tris=0 pack=25 appended=g4_imp_cube,g4_imp_plane vram=23632 warn=- ms=253
png40-room IPKD pack=25 hits=8/40 max_step=3 mean=12.8203125
import-room-apicula IPKD map=217 faces=6 corners=20
import-room-apicula IPKD map=216 faces=1 corners=4
oversize-room IPKD map=217 size=164272 limit=61440
pdsms-room IPKD map=217 size=3164 quads=64 new=2
import-room IPKE cube map=217 size=672 share=0.011 quads=6 plane map=216 size=472 quads=1 tris=0 pack=25 appended=g4_imp_cube,g4_imp_plane vram=23632 warn=- ms=256
png40-room IPKE pack=25 hits=8/40 max_step=3 mean=12.8203125
import-room-apicula IPKE map=217 faces=6 corners=20
import-room-apicula IPKE map=216 faces=1 corners=4
oversize-room IPKE map=217 size=164272 limit=61440
pdsms-room IPKE map=217 size=3164 quads=64 new=2
bake-room CPUD map=187 pack=20 style=indoor size=35020 share=0.570 vram=21656 warn=- ms=93
bake-room IPKD map=217 pack=25 style=indoor size=35024 share=0.570 vram=23544 warn=- ms=97
bake-room IPKE map=217 pack=25 style=indoor size=35024 share=0.570 vram=23544 warn=- ms=266
```

Die `bake-room`-Zeilen sind gleich denen von 6.1 (die Dreiecke im Bäcker ändern die Testfläche nicht). Jeder gated Raumtest prüft außerdem: alte Einträge des Pakets byte-gleich, genau zwei angehängt (pixelgleich zu `texconv::convert` des PNG), alle anderen Karten-Member auf der Platte unverändert, `dirty` genau die zwei Karten, zwei Undo stellen Paket und Karten byte-gleich her.

## (2) Unit-Punkte des Gates

| Gate-Punkt | Test | Ergebnis |
|---|---|---|
| 64 × 64-PNG mit 40 Farben → 16-Farben-NSBTX, ≤ 1 Stufe je Kanal für Palettentreffer | `model_import::textures::tests::a_64x64_png_with_40_colours_becomes_a_16_colour_texture_within_one_step` (`crates/g4-kernel/src/model_import/textures.rs`) | bestanden in (1) 3: `png40 hits=8/40 pixels=816/4096 max_step=3 mean=12.8203125 exact=0/40`; Treffer im Sinne „Palette hat eine Farbe ≤ 1 Stufe von der Quellfarbe“ (Abweichung 2), dazu über alle Pixel `max_step ≤ 3`, `mean ≤ 13.0` |
| Würfel-OBJ und Ebene-glTF werden Modell-Teil einer Karte (ein Undo-Schritt) | `map_import::tests::import_writes_model_and_pack_as_one_undo_step`, `map_import::tests::the_plane_glb_and_gltf_give_the_same_model` (`crates/g4-kernel/src/map_import.rs`) | bestanden in (1) 3 |
| apicula liest sie | `apicula_reads_the_imported_cube_and_plane` (`crates/g4-kernel/tests/model_import.rs`, braucht `G4_APICULA`) | bestanden in (1) 3 und 8: Würfel 6 Flächen, 20 Ecken; Ebene 1 Fläche, 4 Ecken; Ecken- und UV-Menge gleich der Eingabe |
| Ablehnung über 0xF000 mit Zahl | `map_import::tests::the_oversize_obj_is_refused_with_the_number` | bestanden in (1) 3; gated `oversize-room … size=164272 limit=61440` |
| OBJ im PDSMS-Stil (Vierecke + MTL) wird importiert | `map_import::tests::the_pdsms_style_obj_imports` | bestanden in (1) 3; gated `pdsms-room … size=3164 quads=64 new=2` |
| `cargo deny` grün mit `tobj` und `gltf` | `cargo deny --locked check licenses advisories` | bestanden in (1) 1 |

## (3) Spiel (Agent + Blick)

Die Szenarien `scripts/agent/scenarios/pt-import.yaml` und `hg-import.yaml` (Task 10, `90e105b9`) laufen kopflos über `studio mcp` (`--allow-irreversible`, `--expert`): `prepare: make-import-samples import` (14 Dateien über `g4k import-samples`), `g4_call inspect_map_import` des zu großen OBJ (Erwartungen `0.state.code = "model.too-big"`, `0.state.args.1 = "61440"`), `import_map_parts` desselben abgelehnt („import refused: model: the baked model is 164272 bytes; the game holds at most 61440“), dann Würfel-OBJ auf das Zimmer und Ebene-glTF auf den Nachbarraum, `save`, Export der beiden Modelle nach `build/`, `build_rom`, `check:` mit der SHA-1 der beiden Modelle in `out.nds`; gespielt: im Zimmer weiter (Bild `zimmer`), in den Nachbarraum (Bild `unten`). Gelaufen am 2026-11-06 um 17:58/17:59.

| Spiel | Kopie | Rückgabe | Import | `check-data` | Wege | Last |
|---|---|---|---|---|---|---|
| Platin DE (`CPUD`) | `6-5a-szen-pt2` | 0, `"ok": true`, 14 s | Würfel → [Karte 187, Paket 20], Ebene → [Karte 186, Paket 20]; Modell-SHA-1 `7dc3b55f…` / `d4ca8a2c…` | 2 Erwartungen erfüllt | Header 415 (7, 4) → 414 (10, 3) | 10,3 |
| HeartGold DE (`IPKD`) | `6-5a-szen-hg` | 0, `"ok": true`, 18 s | Würfel → [Karte 217, Paket 25], Ebene → [Karte 216, Paket 25]; gleiche SHA-1 wie Platin | 2 Erwartungen erfüllt | Header 64 (3, 4) → 63 (3, 3) | 5,2 |

Blick des Agenten (Bilder `target/agent/gate-2026-11-06/6-5a/{pt-import,hg-import}/{zimmer,unten}.png`):

- **Platin `zimmer.png`** und **HeartGold `zimmer.png`:** schwarzer Boden und keine Wände (das Zimmermodell ist durch den Würfel ersetzt); sichtbar sind nur die Gebäude-Möbel (Fernseher, Regal, PC-Tisch, Pflanze, Bett) und der Spieler. **Kein Würfel im Bild.**
- **`unten.png`** beider Spiele: schwarzer Boden mit Möbeln, Spieler und Mutter. **Keine Ebene im Bild.**
- Auf den oberen Bildschirmen sind rund 43 000 von 49 152 Pixeln schwarz; die Farben der Würfel- und Ebenentextur kommen nirgends vor.

Grund: der Import verschiebt ein Modell nicht, sein Ursprung liegt in der Kartenmitte (Kachel 16, 16); der Würfel ist eine Kachel groß, die Ebene 4 × 4 Kacheln, beide um diesen Punkt. Die Räume liegen oben links in der Karte, die Kamera folgt dem Spieler 9 bis 13 Kacheln davon entfernt. Belegt ist der Import im Spiel durch die SHA-1 der Modelle in `out.nds` und den verschwundenen Zimmerboden, nicht durch ein Bild des Würfels (Abweichung 7). Der Zusatzpunkt „in g4emu sichtbar“ bleibt darum beim Blick des Nutzers offen. In der Szene des Studios ist der Würfel nach dem Import zu sehen (4c).

## (4) Browser-Teil

UI-Test 2026-11-06 18:08 bis 18:17 auf `90e105b9`, Last 22 bis 29. Test-Umgebung `G4_ROMS_DIR=$R G4_VITE_PORT=1500 G4_BRIDGE_PORT=1501 bash scripts/agent/studio-web.sh` (Debug-Brücke), Seite `http://127.0.0.1:1500/?project=$R/_testkopien/6-5a-web_DSPRE_contents`, Browser-Fenster der Desktop-App auf 1440 × 900, hell, Bauen. Vorher `target/debug/g4k import-samples $R/_testkopien/6-5a-web_DSPRE_contents/import` (14 Dateien). Alle Punkte bestanden.

- [x] a) **Zu großes OBJ:** `call.sh list_headers` → Header 415 `T01R0202`, Ort „Zweiblattdorf“, im Baum „Haus 2, 1. Stock“ (Matrix 129, Area-Data 20; der Plan nennt den Raum „Zuhause, 1. Stock“), Karte 0187, Zelle (0, 0). Inspector-Kopf › „Aktionen“ › „Teile importieren…“, Quelle „Datei“, Pfad des zu großen OBJ → Zeile „Modell · 0 Bytes · g4_imp_big.obj“ mit Fehler „Das Modell ist 164.272 Byte groß, erlaubt sind 61.440. Wähle ein kleineres Modell.“, Kästchen gesperrt, Taste „0 Teile übernehmen“. Der Pfad muss absolut sein (Abweichung 9).
- [x] b) **Würfel-OBJ:** Zeile „Modell · 672 Bytes · g4_imp_cube.obj“ ok mit „OBJ-Modell umgewandelt: 6 Flächen, 6 Vierecke, 0 Dreiecke, 0 Streifen.“, „Neue Textur g4_imp_cube: 16 × 16, 16 Farben, mittlerer Farbfehler 2,5.“, „BDHC bleibt unverändert, Laufhöhen prüfen.“; Vorschau „OBJ · 6 Vierecke · 0 Dreiecke · 0 Streifen“, „g4_imp_cube · 16 × 16 · 16 Farben · neu“, „VRAM des Gebiets 21.584 B · Vanilla höchstens 110.640 B“; Budget „Modell 672 von 61.440 B“ ruhig (`data-budget-state="calm"`); Zeile „Umwandeln“ mit „Farben je Textur“ (16 | 4) und „Einheit“ (16, „Welteinheiten je Modelleinheit; 16 = eine Kachel“). „4“ gewählt → „g4_imp_cube · 16 × 16 · 4 Farben · neu“, VRAM 21.496 B (88 B weniger: 64 B Texel, 24 B Palette); zurück auf „16“ → wieder 16 Farben, 21.584 B.
- [x] c) **„1 Teil übernehmen“:** Erfolgsmeldung „1 Teil übernommen“, „Keine neuen Befunde an dieser Karte.“; Kopf „1 Änderung nicht gespeichert“, Verlauf 1. Die Szene zeigt den Raum ohne Boden und Wände (nur die Gebäude-Möbel) und den kleinen Würfel in der Kartenmitte. `call.sh history '{}'` → `{"undo":["map 0187: model imported"], …}`; `call.sh get_map_budget '{"index":187}'` → Modell 672, Paket 20 mit 46 Texturen, VRAM-Schätzung 21 920.
- [x] d) **⌘Z** (echte Taste im Browser-Fenster): die Szene zeigt das Vanilla-Zimmer, Kopf „Alles gespeichert“; `call.sh history` → `undo []`, `redo ["map 0187: model imported"]`; `get_map_budget 187` → Modell 2 572, Paket 20 mit 45 Texturen, VRAM-Schätzung 21 760, also wie vor dem Import (die Datei `unpacked/mapTextures/0020` hat unverändert 8 108 B, ungespeichert).
- [x] e) **PNG:** Pfad des 40-Farben-PNG → Zeile „Texturen · 2.132 Bytes“ mit „Neue Textur g4_imp_40: 64 × 64, 16 Farben, mittlerer Farbfehler 12,8.“, „g4_imp_40 · 64 × 64 · 16 Farben · neu“, „VRAM des Gebiets 23.504 B · Vanilla höchstens 110.640 B“; Zeile „Umwandeln“ nur mit „Farben je Textur“, ohne Feld „Einheit“. Mit „Abbrechen“ geschlossen, nichts importiert.
- [x] f) **Bilder** (nicht im Repo) unter `target/agent/gate-2026-11-06/6-5a/`: `blick-import-fehler-hell.png`, `blick-import-fehler-dunkel.png` (Zustand a), `blick-import-wuerfel-hell.png`, `blick-import-wuerfel-dunkel.png` (Zustand b mit 16 Farben, vor „übernehmen“), `blick-import-png-hell.png`, `blick-import-png-dunkel.png` (Zustand e). Das Browser-Fenster der Desktop-App liefert Bilder nur an den Agenten; die Dateien kommen darum wie in 6.2 aus Chrome headless über CDP (`--remote-debugging-pipe`, 1440 × 900, Thema über `g4.theme` = `hell`/`dunkel` und `prefers-color-scheme`, je Neuladen; Skript `target/agent/6-5a/t11-blick.mjs`, Log `t11-blick.log`: `<html data-theme>` je Bild `hell` bzw. `dunkel`). Ablauf im Skript wie oben mit echten Maus- und Tastenereignissen (`Input.dispatchMouseEvent`, `Input.insertText`, Tab). Vom Agenten angesehen: Dialog mittig über der Szene, Fehlerzeile mit rotem Symbol und gesperrtem Kästchen, Würfelzeile mit grünem Haken, Budgetbalken, die Zeile „Umwandeln“; im Dunkel-Thema lesbar, Kontraste wie im übrigen Dialog. Rechts unten steht in allen Bildern die Nachricht „Textprojekt einrichten“ (Kopie ohne Basis-ROM), sie verdeckt nichts vom Dialog.

Danach: die Test-Umgebung (Skript, `studio-bridge`, Vite) über ihre PIDs beendet, `lsof -iTCP:1500 -iTCP:1501 -sTCP:LISTEN` leer, Chrome headless beendet.

## (5) Beim Nutzer (Hand und Blick)

- **Blick auf die vier `g4emu`-Bilder** (Agent + Blick): `target/agent/gate-2026-11-06/6-5a/pt-import/zimmer.png`, `…/pt-import/unten.png`, `…/hg-import/zimmer.png`, `…/hg-import/unten.png` (im Worktree `.claude/worktrees/phase-6-5a`). Erwartet nach Abweichung 7: Zimmer ohne Boden und Wände, Würfel und Ebene nicht im Bild. Der Nutzer entscheidet, ob das den Zusatzpunkt „in g4emu sichtbar“ trägt oder ob ein Versatz (6.5b) nötig ist.
- **Blick auf die sechs Browser-Bilder** `target/agent/gate-2026-11-06/6-5a/blick-import-{fehler,wuerfel,png}-{hell,dunkel}.png`.
- **Hand:** in der gebündelten App „Dateien wählen…“ mit einer eigenen Blender-Datei (`.glb` und `.gltf`) und, wer PDSMS hat, mit einem echten „Export as OBJ with Textures“: Import, Szene, Spiel. Weicht ein echter PDSMS-Export ab (Achsen, Ursprung, Unterordner für Texturen), wird das in `docs/formats/model_import.md` nachgetragen.
- **`docs/formats/model_import.md` gelesen** (Hand).
- **`scripts/gate.sh` auf main** (Agent, Hauptsitzung): offen, nach dem Merge.

## Ergebnis

| Punkt | Wer | Ergebnis | Datum |
|---|---|---|---|
| (1) 1 `cargo deny` | Agent | bestanden: advisories ok, licenses ok, `tobj` 4.0.5 und `gltf` 1.4.1 mit vier transitiven Crates (`gltf-json`, `gltf-derive`, `inflections`, `lazy_static`), alle MIT bzw. MIT OR Apache-2.0 | 2026-11-06 |
| (1) 2 fmt, clippy | Agent | bestanden | 2026-11-06 |
| (1) 3 Tests `g4-formats`, `g4-kernel` ohne ROM-Daten | Agent | bestanden: 2 489 in 59 Zielen, 1 ignoriert, mit den apicula-Tests | 2026-11-06 |
| (1) 4–7 `studio`, Frontend, Stil, Tokens, ROM-Daten, `g4emu` | Agent | bestanden: `studio` 775, Frontend 3 972 (1 übersprungen) und Build, Stil, Tokens, ROM-Daten (2 028 Dateien), `g4emu` 404 | 2026-11-06 |
| (1) 8 gated `model_import` | Agent | bestanden: je 5/5 auf Platin DE, HeartGold DE, HeartGold US (Task 8) | 2026-11-06 |
| (1) 9 gated `bake` | Agent | bestanden: je 8/8, `bake-room` gleich 6.1 (Task 8) | 2026-11-06 |
| (1) 10 `scripts/gate.sh` | Agent (Hauptsitzung) | offen — läuft auf main nach dem Merge | – |
| (2) Unit-Punkte des Gates | Agent | bestanden, alle sechs Tests grün; „Palettentreffer“ tolerant gelesen (Abweichung 2) | 2026-11-06 |
| (3) Szenarien `pt-import`, `hg-import` | Agent + Blick | bestanden: Rückgabe 0, Ablehnung über 0xF000 mit 164 272 B, `check-data` 2/2, Header 415 → 414 bzw. 64 → 63 | 2026-11-06 |
| Zusatzpunkt „Würfel und Ebene in g4emu sichtbar“ | Agent + Blick | nicht im Bild (außer Sicht der Kamera, Abweichung 7); Import im Spiel über SHA-1 und fehlenden Zimmerboden belegt; Entscheidung beim Nutzer | 2026-11-06 |
| (4) Browser a) bis f) | Agent | bestanden: Fehler mit 164.272 und 61.440, Würfel 672 B ruhig, 4/16 Farben, ein Undo-Schritt, ⌘Z zurück, PNG ohne „Einheit“, sechs Bilder | 2026-11-06 |
| (5) Blick auf die vier `g4emu`- und sechs Browser-Bilder | Hand | offen | – |
| (5) eigene Blender- und PDSMS-Dateien in der gebündelten App | Hand | offen | – |
| (5) `docs/formats/model_import.md` gelesen | Hand | offen | – |

Zahlentabelle (Release; gated aus Task 8, Szenario aus (3)):

| Spiel | Würfel (Byte, Anteil, Vierecke) | Ebene (Byte, Vierecke, Dreiecke) | Paket (angehängt, VRAM, Meldungen) | PNG 40 (Treffer, max. Stufe, Mittel) | zu groß (Byte) | Szenario | Last |
|---|---|---|---|---|---|---|---|
| Platin DE | Karte 187: 672 B, 1,1 % von 0xF000, 6 | Karte 186: 472 B, 1, 0 | Paket 20: `g4_imp_cube`, `g4_imp_plane`; VRAM 21 744; keine | Paket 20: 8/40, 3, 12,82 | 164 272 (Grenze 61 440) | `pt-import` 0, 2/2, 415 → 414 | gated 5,7 / 7,6; Szenario 10,3 |
| HeartGold DE | Karte 217: 672 B, 1,1 %, 6 | Karte 216: 472 B, 1, 0 | Paket 25: beide; VRAM 23 632; keine | Paket 25: 8/40, 3, 12,82 | 164 272 | `hg-import` 0, 2/2, 64 → 63 | gated 6,7 / 5,1; Szenario 5,2 |
| HeartGold US | Karte 217: 672 B, 1,1 %, 6 | Karte 216: 472 B, 1, 0 | Paket 25: beide; VRAM 23 632; keine | Paket 25: 8/40, 3, 12,82 | 164 272 | – | gated 4,5 / 12,2 |

„Last“ ist die 1-Minuten-Last aus `uptime` vor `model_import` / vor `bake`. VRAM ist der gemessene Wert des Bäckers nach beiden Importen (Kartenpaket und Gebäudepaket des Gebiets); `share` ist Größe / 61 440. Die OBJ im PDSMS-Stil: 3 164 B, 64 Vierecke, zwei neue Texturen in allen drei Spielen.

## Abweichungen

Aus dem Plan (Abschnitt „Abweichungen von Spezifikation und Gutachten“) und aus den Entscheidungen der Umsetzung, soweit sie ändern, was das Gate zeigt:

1. **g4emu-Punkt zusätzlich:** Die Gate-Zeile der Spezifikation verlangt „apicula liest sie“; der Auftrag der Hauptsitzung nennt „in g4emu sichtbar“. Der Plan erfüllt beides: apicula als Gate-Punkt (2), g4emu als Zusatzpunkt (3) mit `pt-import`/`hg-import`.
2. **„Palettentreffer“ tolerant gelesen:** Wörtlich („die Quellfarbe in BGR555 steht in der Palette“) hat das Beispielbild 0 von 40 Treffern (`exact=0/40`): der Median-Cut aus 6.1 legt Mittelwerte seiner Boxen ab, keine Quellfarben. Als Treffer zählt darum „die Palette hat eine Farbe innerhalb einer Stufe je Kanal“: 8 von 40 Farben (816 von 4 096 Pixeln), für sie liegt die zurückdekodierte Farbe ≤ 1 Stufe je Kanal. Weil diese Treffer durch `texconv::nearest` fast garantiert sind, begrenzt der Test zusätzlich den Fehler über alle Pixel (`max_step ≤ 3`, `mean ≤ 13.0`, gemessen 3 und 12,82). Der Konverter aus 6.1 bleibt unverändert; wollte man die wörtliche Lesart, bräuchte er exakte Farben für einfarbige Boxen (Posten in 6.1-Code, nicht 6.5a).
3. **Bäcker mit Dreiecken:** Die Entscheidungen (Writer, „Teilmenge des Bäckers“) sagen, der Bäcker brauche keine Dreiecke. Importierte Modelle haben welche (glTF nur Dreiecke, nicht paarbare Reste); Task 1 erweitert den Bäcker um Dreiecke, ohne `Face` zu ändern.
4. **Genauigkeit 1 Welteinheit statt `pos_scale`-Wahl:** Gutachten C §6.1 schlägt vor, `pos_scale` je Modell zu wählen (1/64 Einheit Genauigkeit). Der Bäcker hat `POS_SCALE` 64 fest und `Face` ganze Welteinheiten und Texel; Importe werden auf 1/16 Kachel und ganze Texel gerundet. Feinere Modelle brauchen später eine `Face` mit Bruchteilen (eigener Posten, nicht 6.5a).
5. **Vorschau als Kennzahlen:** „Import-Dialog mit Vorschau“ (Spezifikation §6) ist hier Prüfbericht mit Budget und Texturliste, keine 3D-Ansicht im Dialog; die Szene zeigt das Modell nach dem Import, ⌘Z ist der Rückweg (Regel des Dialogs seit 3b.4b).
6. **PNG im selben Dialog:** „PNG → NSBTX … in der Oberfläche“ nimmt den vorhandenen Dialog „Teile importieren…“ (Teil „Texturen“, Paket des Gebiets der Karte) statt eines eigenen Dialogs am Gebiet. Eigenes Ohrenschneiden statt `earcutr`, kein `meshopt`: wenige Zeilen eigener Code, keine weitere Abhängigkeit.
7. **Würfel und Ebene außer Sicht in g4emu:** Der Ursprung des Modells liegt in der Kartenmitte (16, 16), die Räume oben links; die Kamera zeigt beide nicht (siehe (3)). Nach dem Plan („Offen: Würfel außer Sicht“) bleiben `unit` und Lage für das Bild unverändert. Der Zusatzpunkt ist darum optisch nicht erfüllt; belegt sind Import, Speichern und Bau über die SHA-1 in `out.nds`. Sichtbar würde er mit einem Versatz-Feld in `ImportOptions` (Produktentscheidung, 6.5b) oder einer anderen Karte/Lage.
8. **Exportpfade der Szenarien `build/…`:** statt `build/import/…` (Plan): die MCP-Pfadregel legt `build/` an, aber keinen Unterordner darunter (gewollt). Nur die Pfade ändern sich.
9. **Relativer Pfad im Dialog:** Der Plan schreibt im Browser-Ablauf `import/g4_imp_big.obj`; das Feld „Pfade, eine je Zeile“ nimmt Pfade wie das Dateifenster, also absolut („Pfad nicht lesbar: import/g4_imp_big.obj: No such file or directory“ mit dem relativen Pfad, so seit 3b.4b). Der Test lief mit `$R/_testkopien/6-5a-web_DSPRE_contents/import/…`. Über MCP (Szenarien) gilt der Pfad relativ zum Projekt (Pfadregel von MCP).
10. **`.fig` erst nach dem Merge gerendert:** Der Design-Nachtrag 6.5a steht in `design-system/README.md`, `design-system/openpencil/NACHTRAG-6-5a.md` und `design-system/openpencil/beispiele/import-modell.body.js`; im Branch blieb die OpenPencil-Datei unberührt (binärer Mergekonflikt mit parallelen Phasen, Grenze von 10 000 Knoten). Erledigt am 09.11.2026 (Branch `phase-6-fig-nachtraege` auf main `7a8e3915`): „Teile importieren: Modell aus OBJ“ hell (x 0, y 5000) und dunkel (x 1520, y 5000) auf der Seite „App: Werkbank“, die Zeile „Kartenteile importieren“ im Brett „Stand der App“.
11. **Fix-Commits getrennt:** Die Korrekturen aus den Reviews liegen als eigene `fix(…)`-Commits auf dem Branch (`9efc5982`, `b2814968`), nicht in die Task-Commits eingefaltet.
12. **Gated Läufe aus Task 8 übernommen, nach der Fix-Welle wiederholt:** Für das Gate (Punkte (1) 8 und 9) zählten zunächst die gated Läufe von Task 8 (kein Commit unter `crates/` seit `23562ea0`). Die Fix-Welle nach dem Schluss-Review (`5b6c4f02`, Grenzen für glTF-Accessoren, Instanzen, Puffer und Ecken je Polygon) ändert die Leser in `model_import`; darum lief gated `--test model_import` (Release, mit `G4_APICULA`, `--nocapture --test-threads 1`) am 2026-11-06 auf `5b6c4f02` je Spiel erneut auf den unbeschriebenen Kopien `6-5a-{pt,hgde,hgus}` (Logs `target/agent/6-5a/fix-gated-{pt,hgde,hgus}.log`): je 5/5, Platin DE 118 s (Last 21,1), HeartGold DE 117 s (Last 8,7), HeartGold US 155 s (Last 6,8). Die sechs Belegzeilen je Spiel (`import-room`, `png40-room`, `import-room-apicula` ×2, `oversize-room`, `pdsms-room`) sind gleich denen aus Task 8 bis auf die Laufzeit `ms=` (294, 269, 226 statt 214, 253, 256). Gated `--test bake` nicht wiederholt: `tests/bake.rs` nutzt die Leser nicht, der Bäcker ist unverändert.

## Befunde

Kein Befund in (1) bis (4), der einen Fix in Task 11 brauchte. Beobachtungen ohne Fix:

- **Zwei VRAM-Zahlen:** Der Dialog nennt die VRAM-Messung des Bäckers (`area_vram`, Texel und Paletten aus BTX0, 21 584 B mit dem Würfel), der Inspector die Schätzung des Budgets aus 6.2 (`terrain::budget::vram_bytes`, geteilte Texel doppelt, 21 760 B ohne Würfel, 21 920 B mit). Die Schätzung liegt laut ihrem Doc-Kommentar 1 bis 15 % über der Messung, bis 6.1s `Nsbtx` die Texel-Offsets zeigt; nebeneinander sieht der Nutzer zwei verschiedene Zahlen für „VRAM des Gebiets“. Posten für die Angleichung der Schätzung (nicht 6.5a-Code).
- **„0 Bytes“ in der Fehlerzeile:** Die abgelehnte Modellzeile heißt „Modell · 0 Bytes · g4_imp_big.obj“ (Größe des gebackenen Teils, das es nicht gibt); die Meldung darunter nennt die 164.272 Byte. Sprachlich lesbar, aber die 0 kann irritieren; Kandidat für den Blick des Nutzers.
- **`save` nach `import_map_parts`** nennt in den Szenarien unter `written` nur die Karten, obwohl auch das Paket 20/25 geändert ist; das Paket liegt trotzdem auf der Platte (wie `bake_test` in 6.1).

## Offen / später

- Versatz-Feld in `ImportOptions` (Würfel ins Bild), erst wenn ein echter Export es braucht (Plan, „Offen“).
- Reale PDSMS-Exporte und eigene Blender-Dateien (Hand-Punkt).
- `map_Kd` mit Optionen, Namensgleichheit zwischen Modell- und PNG-Teil, Vanilla-Paket mit 255 Einträgen: wie im Plan unter „Offen für die Umsetzung“ beschrieben; `docs/formats/model_import.md` nennt die Codes `texture.taken` (auch Modell und PNG mit demselben Namen) und `model.texture-count`.
- Zurückgestellte Kleinpunkte aus den Reviews stehen im Ledger der Umsetzung (`.superpowers/sdd/2026-11-06-phase-6-5a-import/progress.md`, Zeilen „minor (deferred)“).
