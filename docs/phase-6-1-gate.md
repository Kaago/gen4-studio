# Phase 6.1: Gate prüfen

Stand: 2026-11-06. Branch `phase-6-1-writer` (Worktree `.claude/worktrees/phase-6-1`, Basis `d515c5ee` = main `d05fad84` mit den Plänen 6.1, 6.2 und 6.3a), nicht nach main gemergt. Arbeit begonnen am 2026-11-05. Vom Agenten erledigt: (1) Automatik Schritt für Schritt auf dem Stand `898d1d58` (`cargo deny`, fmt, clippy, die Tests von `g4-formats` und `g4-kernel` ohne ROM-Daten, die gated Tests `vanilla_nitro` und `bake` im Release-Profil auf Platin DE, HeartGold DE und HeartGold US), (2) die Unit-Punkte des Gates und (3) die Szenarien `pt-bake`, `hg-bake` und `hgus-bake` (Task 10, `898d1d58`, am 2026-11-05) mit dem Blick des Agenten auf die sechs Bilder. `scripts/gate.sh` als Ganzes läuft hier nicht (Vorgabe des Nutzers), sondern in der Hauptsitzung auf main nach dem Merge.

Beim Nutzer offen: der Blick auf die sechs Bilder, `docs/formats/nsbmd.md` gelesen, `scripts/gate.sh` auf main (Abschnitt „(4) Beim Nutzer (Hand und Blick)“).

Phase 6.1 bringt den Writer: das Wörterbuch `nitro_dict` (Nitro-Dictionary mit dem Patricia-Baum der Vanilla-Dateien), den MDL0-Codec `nsbmd` (Leser und Writer, Display-List-Codec `gpu`, SBC-Codec `sbc`, Byte-Gleichheit für jedes Vanilla-Kartenmodell), den BTX0-Writer `nsbtx::write` mit `append` und `build`, den Texturkonverter `texconv` (RGBA → 16 oder 4 Farben mit eigenem Median-Cut in BGR555), im Kernel das Modul `bake` (neutrale Flächenliste `Face` → Kartenmodell mit Strips, `VTX_10`, Gebietsstil, Budget-Bericht und Prüfcodes; Texturen an ein Kartenpaket anhängen als ein Undo-Schritt mit VRAM je Gebiet; Testfläche und `bake_test`), den Befehl `bake_test` (Tauri, MCP über `g4_call`, `g4k`), die Art `map_texture` in `check-data` und die Formatdoku [docs/formats/nsbmd.md](formats/nsbmd.md). 6.1 hat keine Oberfläche: einen Browser-Teil gibt es nicht.

Gate-Text der Spezifikation (§4, Kopf der Gate-Spalte und Zeile 6.1):

> Gate (zusätzlich: gated Tests auf frischen Kopien von Platin DE, HeartGold DE und HeartGold US; Browser-Teil über die Test-Umgebung; `scripts/gate.sh` grün; `cargo deny` grün; Hand und Blick beim Nutzer)
>
> Gated Round-Trip Leser → Writer byte-gleich für alle Kartenmodelle (666, 676, 676) und alle BTX0 der Karten- und Gebäudetexturen (145, 200, 200), alle Wörterbücher byte-gleich; der Bäcker schreibt die Testfläche (32 × 32, vier Texturen 2 × 2, Block) ≤ 40 000 Byte, der eigene Leser liest sie, Zähler im Kopf gleich der Display List, ein Modell über 0xF000 wird mit `bake.model-too-big` abgelehnt; apicula Exit 0, genau eine GLB, Ecken- und UV-Menge gleich der Eingabe; NSBTX: vier synthetische Bilder → Paket, Rücklesen mit `g4-formats` 0 Pixel Abweichung, apicula-PNG gleich `g4-formats`, Anhängen an ein Vanilla-Paket lässt alte Einträge byte-gleich; g4emu auf den drei Spielen: Testfläche mit zwei eigenen Texturen im Zimmer (Platin 187, HeartGold 217) sichtbar mit Block, Nachbarraum (186 / 216, gleiches Paket) zeigt Vanilla-Texturen unverändert (Agent + Blick); `docs/formats/nsbmd.md`

Wer was prüft: Die Spalte „Wer“ sagt **Agent** (die Test-Umgebung aus [docs/agent-gate.md](agent-gate.md): `studio mcp` kopflos, `g4emu` und die gated Tests), **Agent + Blick** (der Agent prüft und beschreibt Ort und Zahlen, der Nutzer bestätigt kurz am Bild) oder **Hand** (der Nutzer). Befehle: [docs/agent-gate.md](agent-gate.md), Abschnitt „Phase 6.1“.

## Vorbereitung

1. **Kopien** unter `Roms/_testkopien/` per `cp -cR` (Ordner) und `cp -c` (`.nds`) aus den Originalen, die dabei nur gelesen wurden:
   - für (1): `6-1-pt_DSPRE_contents` (Platin DE), `6-1-hgde_DSPRE_contents` (HeartGold DE), `6-1-hgus_DSPRE_contents` (HeartGold US), angelegt in Task 3 und für alle gated Läufe wiederverwendet. Sie werden nur gelesen; geschrieben wird nur auf `vanilla_copy()`.
   - für (3): `6-1-szen-pt_DSPRE_contents` + `6-1-szen-pt.nds`, `6-1-szen-hg_DSPRE_contents` + `6-1-szen-hg.nds`, `6-1-szen-hgus_DSPRE_contents` + `6-1-szen-hgus.nds`, frisch in Task 10. Nach dem Lauf sind sie gespeichert und gebaut (`build/out.nds`); ein neuer Lauf braucht neue Kopien (etwa `6-1-szen-pt2`).
2. **Werkzeuge:** `export PATH="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$HOME/.cargo/bin:$PATH"`; `tools/` ist der Symlink auf den Haupt-Checkout (nie einchecken); `G4_ROMS_DIR=/Users/POLTEP/Projekte/lab/editor/Roms` und `G4_APICULA=/Users/POLTEP/Projekte/lab/editor/gen4-studio/tools/apicula/bin/apicula` absolut (apicula `3d4e91e`). Für (3) `cargo build --workspace --all-features` (Debug-`studio` und `g4k`).
3. **Ausgaben** im Worktree: Logs unter `target/agent/6-1/` (`gate-<schritt>.log` für (1) mit `uptime`, Stand, Befehl, Dauer und Rückgabe je Schritt, Übersicht `gate-summary.log`; `t10-{pt,hg,hgus}.log` für (3)); die Bilder der Szenarien unter `target/agent/gate-2026-11-05/6-1/<pt-bake|hg-bake|hgus-bake>/` (`zimmer.png`, `unten.png`). Das Skript für (1) lag im Scratchpad. Den Ordner `target/agent/` vor `git worktree remove` sichern.
4. **Last:** `uptime` vor jedem Schritt (Spalte „Last“ der Zahlentabelle). Auf dem Rechner liefen gleichzeitig andere Sitzungen; die drei Spiele liefen nacheinander, nie parallel.

## (1) Automatik

Nach Vorgabe des Nutzers läuft hier nicht `scripts/gate.sh` als Ganzes (das macht die Hauptsitzung auf main nach dem Merge), sondern die Schritte, die 6.1 berührt, einzeln. Die neuen Tests laufen in `scripts/gate.sh` ohne Änderung am Skript mit: die Unit- und Integrationstests ohne ROM-Daten im Schritt „cargo test (no ROM data)“, `vanilla_nitro` und `bake` im Schritt „gated tests (cargo test)“ je Projekt (Debug).

Alle Schritte grün im ersten Versuch, 2026-11-06 05:07 bis 07:20 auf `898d1d58` (Logs `target/agent/6-1/gate-<schritt>.log`, Übersicht `gate-summary.log`):

- [x] 1. `cargo deny --locked check licenses advisories`: „advisories ok, licenses ok“, 10 s; keine neue Abhängigkeit (`g4-formats` nur `thiserror`, Tests `proptest`; `quantette` nicht gebraucht).
- [x] 2. `cargo fmt --check` (4 s) und `cargo clippy --workspace --all-targets --all-features -- -D warnings` (11 s), sauber.
- [x] 3. `cargo test -p g4-formats -p g4-kernel --all-features` ohne `G4_TEST_PROJECT` (und ohne `G4_APICULA`): 2 343 bestanden in 56 Testzielen, 0 fehlgeschlagen, 1 ignoriert; Bibliothek `g4-formats` 250, Bibliothek `g4-kernel` 1 215, `tests/bake.rs` 8 (die gated und die apicula-Tests überspringen sich ohne ihre Variablen), `tests/cli.rs` 65; 7 452 s bei Last 16 → 20 (`-p g4-kernel` lud im Worktree nichts nach). Dazu `cargo test -p g4-kernel --all-features --lib bake:: -- --nocapture` für die gedruckten Größen der Testfläche: 27 bestanden, 685 s.
- [x] 4. **Gated `--test vanilla_nitro`** (Release, `--nocapture --test-threads 1`): je 3/3 auf Platin DE (17 s), HeartGold DE (12 s), HeartGold US (14 s).
- [x] 5. **Gated `--test bake`** (Release, `--workspace --all-features`, mit `G4_APICULA`): je 8/8 auf Platin DE (151 s), HeartGold DE (154 s), HeartGold US (141 s).
- [ ] 6. `scripts/gate.sh`: offen, läuft in der Hauptsitzung auf main nach dem Merge.

Kein Test war rot, keine Wiederholung nötig. Beim Release-Bau warnt cargo „stripping debug info with `rust-objcopy` failed“ (im Toolchain fehlt `libLLVM.dylib`); das ist eine Warnung der Umgebung, kein Befund von 6.1, die Läufe sind grün.

Belegzeilen (aus `gate-nitro-<k>.log` und `gate-bake-<k>.log`, wörtlich):

```
nitro-append CPUD packs=74 ok=74 ms=618
nitro-models CPUD models=666 equal=666 dicts=3996 stale= ms=3893
nitro-packs CPUD ms=1147
nitro-packs CPUD map=74 building=71 equal=145 dicts_total=4286 odd16=0
nitro-append IPKD packs=106 ok=106 ms=414
nitro-models IPKD models=676 equal=676 dicts=4056 stale=0468,0475,0476 ms=3229
nitro-packs IPKD ms=2238
nitro-packs IPKD map=106 building=94 equal=200 dicts_total=4456 odd16=0
nitro-append IPKE packs=106 ok=106 ms=471
nitro-models IPKE models=676 equal=676 dicts=4056 stale=0468,0473,0475,0476 ms=3401
nitro-packs IPKE ms=2653
nitro-packs IPKE map=106 building=94 equal=200 dicts_total=4456 odd16=0
bake-room CPUD map=187 pack=20 style=indoor size=35020 share=0.570 vram=21656 warn=- ms=228
bake-room-detail CPUD area=20 block=(5, 7) floor=["g4_bake_a", "carpet01_1", "g4_bake_b", "carpet01_2"] appended=["g4_bake_a", "g4_bake_b"] strips=514 single=4
bake-room IPKD map=217 pack=25 style=indoor size=35024 share=0.570 vram=23544 warn=- ms=371
bake-room-detail IPKD area=25 block=(5, 6) floor=["g4_bake_a", "01yuka_14", "g4_bake_b", "05fuchi03"] appended=["g4_bake_a", "g4_bake_b"] strips=514 single=4
bake-room IPKE map=217 pack=25 style=indoor size=35024 share=0.570 vram=23544 warn=- ms=740
apicula test surface: 1032 faces, 1110 distinct corners in, 1110 out, 70 ms
apicula syn_gras: 32 × 32, pixels off 0 (with model: false)
sample syn_himmel: 64 × 64, 16 colours, color0 false, mean error 12.7, pixels off 0
test surface Outdoor: 35208 bytes
test surface Indoor: 35188 bytes
```

`stale` nennt die Vanilla-Modelle, deren Zähler im Kopf nicht zur Display List passen (Platin keins, HeartGold DE 3, HeartGold US 4); der Writer behält sie byte-gleich. `odd16` zählt Vanilla-Paletten mit 16 Farben auf ungerader 8-Byte-Einheit (0, siehe „Befunde und Entscheidungen“). Die apicula-Zeilen sind in allen drei Läufen gleich (1 110 / 1 110 Paare aus Position und UV in 60 bis 70 ms; vier Bilder je 0 Pixel); die mittleren Farbfehler der Beispielbilder 7,6 / 3,2 / 1,9 / 12,7 (gleich dem Prototyp), der Bäcker-Texturen 2,47 / 2,92 (nur BGR555-Rundung).

## (2) Unit-Punkte des Gates

| Gate-Punkt | Test | Ergebnis |
|---|---|---|
| Testfläche ≤ 40 000 Byte | `bake::tests::the_test_surface_bakes_under_40000_bytes` (`crates/g4-kernel/src/bake/mod.rs`) | bestanden in (1) 3: außen 35 208 B, innen 35 188 B (je 57,3 % von 0xF000), keine Meldung |
| der eigene Leser liest sie, Zähler gleich der Display List | `bake::tests::the_own_reader_reads_the_baked_model` | bestanden in (1) 3: `write` des Gelesenen gleich den Bytes, `counters_match`, 3 100 Ecken, 1 032 Flächen, 1 Objekt, 5 Materialien, 5 Shapes |
| über 0xF000 → `bake.model-too-big` | `bake::tests::an_oversize_model_is_refused_with_model_too_big` | bestanden in (1) 3 |
| apicula Exit 0, eine GLB, Ecken- und UV-Menge gleich | `apicula_reads_the_test_surface_like_the_input` (`crates/g4-kernel/tests/bake.rs`, braucht `G4_APICULA`) | bestanden in (1) 5 auf allen drei Läufen: Exit 0, genau eine GLB, 1 110 Paare in, 1 110 aus |
| vier Bilder → Paket, 0 Pixel Abweichung | `sample_images_read_back_without_a_pixel_off` (`tests/bake.rs`) | bestanden in (1) 3 und 5: vier Bilder, je 0 Pixel Abweichung |
| apicula-PNG gleich `g4-formats` | `apicula_decodes_the_sample_pack_like_g4_formats` (`tests/bake.rs`, braucht `G4_APICULA`) | bestanden in (1) 5: `--more-textures` schreibt die PNGs auch ohne Modell, vier Bilder je 0 Pixel |
| Anhängen an ein Vanilla-Paket lässt alte Einträge byte-gleich | `vanilla_append_keeps_every_old_entry` (`crates/g4-formats/tests/vanilla_nitro.rs`, gated) | bestanden in (1) 4: 74 / 106 / 106 Pakete, alle alten Einträge byte-gleich |

## (3) Spiel (Agent + Blick)

Die Szenarien `scripts/agent/scenarios/pt-bake.yaml`, `hg-bake.yaml` und `hgus-bake.yaml` (Task 10) laufen kopflos über `studio mcp` (`mcp: [--allow-irreversible, --expert]`): `g4_call bake_test {map, outDir: build/bake}`, `save`, `build_rom` nach `build/out.nds`, `check:` mit `map:<n>:model_sha1=${sha1-of:${bake.model_file}}` und `map_texture:${bake.pack}:sha1=${sha1-of:${bake.pack_file}}`; gespielt: im Zimmer weiter (Bild `zimmer`), die Treppe hinunter in den Nachbarraum mit demselben Paket (Bild `unten`). Ohne `block`-Argument: der Block steht in der Mitte des alten Modellrahmens und liegt in allen drei Spielen im Bild. Der Spieler läuft durch den Block (Kollision und BDHC bleiben unverändert), das ist gewollt. Gelaufen am 2026-11-05 auf `898d1d58` bei Last um 37, nicht wiederholt (die Kopien sind danach gespeichert und gebaut).

| Spiel | Rückgabe | `bake_test` | `check-data` | Wege |
|---|---|---|---|---|
| Platin DE (`CPUD`) | 0, `"ok": true` | Karte 187, Paket 20, angehängt `g4_bake_a`, `g4_bake_b`; Boden [`g4_bake_a`, `carpet01_1`, `g4_bake_b`, `carpet01_2`]; Block (5, 7); 35 020 B (57,0 % von 0xF000), VRAM 21 656, 3 100 Ecken, 1 032 Flächen, 514 Strips, 4 Einzelvierecke, keine Meldung | 2 Erwartungen erfüllt (Modell von Karte 187, Paket 20) | Header 415 (7, 4) → 414 (10, 3) |
| HeartGold DE (`IPKD`) | 0, `"ok": true` | Karte 217, Paket 25, beide angehängt; Boden [`g4_bake_a`, `01yuka_14`, `g4_bake_b`, `05fuchi03`]; Block (5, 6); 35 024 B (57,0 %), VRAM 23 544, keine Meldung | 2 Erwartungen erfüllt (Karte 217, Paket 25) | Header 64 (3, 4) → 63 (3, 3) |
| HeartGold US (`IPKE`) | 0, `"ok": true` | gleich HeartGold DE (Karte 217, Paket 25, Block (5, 6), gleicher Boden und gleiche Größe) | 2 Erwartungen erfüllt (gleiche SHA-1 wie DE) | 64 → 63 |

Blick des Agenten (Farben nicht verglichen: `g4emu` tauscht Rot und Blau bis zum Farbfix von 6.2):

- **Platin `zimmer.png`:** der ganze Boden des Zimmers ist die Testfläche: das orange-blaue Schachbrett (`g4_bake_a`), die grünen Schrägstreifen (`g4_bake_b`) und die beiden Vanilla-Teppiche im Wechsel; keine Wände (die Testfläche ersetzt das ganze Modell), Möbel und Spieler stehen darauf. Der Block links unter dem Spieler (Kachel (5, 7)) zeigt die Oberseite über dem Bodenraster, die verkürzte Vorderseite und eine schmale rechte Seite. Nicht weggeschnitten.
- **Platin `unten.png`:** das Erdgeschoss (Header 414, Karte 186, Paket 20): Boden, Wände, Treppe, Fernseher, Tisch, Teppich und Mutter mit den Vanilla-Texturen, nichts von den Testtexturen.
- **HeartGold `zimmer.png`:** dasselbe Bild mit `01yuka_14` und `05fuchi03`; der Block bei (5, 6) rechts unter dem Spieler, Oberseite über dem Raster, Vorderseite und schmale rechte Seite. Die schwarze Fläche oben links liegt außerhalb des Modells (keine Wände). Nicht weggeschnitten.
- **HeartGold `unten.png`:** das Erdgeschoss von Header 63 (Karte 216, Paket 25), alles Vanilla; unten das Touch-Menü.
- **HeartGold US:** der obere Bildschirm beider Bilder ist pixelgleich zu HeartGold DE (Differenz-Box von PIL leer), unten das englische Menü.

## (4) Beim Nutzer (Hand und Blick)

- **Blick auf die sechs Bilder** (Agent + Blick): eigene Texturen sichtbar mit Block im Zimmer, Nachbarraum mit Vanilla-Texturen unverändert:
  - `target/agent/gate-2026-11-05/6-1/pt-bake/zimmer.png`, `target/agent/gate-2026-11-05/6-1/pt-bake/unten.png`
  - `target/agent/gate-2026-11-05/6-1/hg-bake/zimmer.png`, `target/agent/gate-2026-11-05/6-1/hg-bake/unten.png`
  - `target/agent/gate-2026-11-05/6-1/hgus-bake/zimmer.png`, `target/agent/gate-2026-11-05/6-1/hgus-bake/unten.png`
  
  (im Worktree `.claude/worktrees/phase-6-1`; `last.png` daneben ist das letzte Bild des Laufs, gleich `unten.png`)
- **`docs/formats/nsbmd.md` gelesen** (Hand): Container, Wörterbuch, Modell, Layout des Writers, Teilmenge des Bäckers mit den Prüfcodes.
- **`scripts/gate.sh` auf main** (Agent, Hauptsitzung): offen, läuft in der Hauptsitzung auf main nach dem Merge.

## Ergebnis

| Punkt | Wer | Ergebnis | Datum |
|---|---|---|---|
| (1) 1 `cargo deny` | Agent | bestanden: advisories ok, licenses ok, keine neue Abhängigkeit | 2026-11-06 |
| (1) 2 fmt, clippy | Agent | bestanden | 2026-11-06 |
| (1) 3 Tests `g4-formats`, `g4-kernel` ohne ROM-Daten | Agent | bestanden: 2 343 in 56 Zielen, 1 ignoriert | 2026-11-06 |
| (1) 4 gated `vanilla_nitro` | Agent | bestanden: je 3/3; Modelle 666 / 676 / 676, Pakete 145 / 200 / 200, Wörterbücher 4 286 / 4 456 / 4 456 (zusammen 13 198) byte-gleich, Anhängen 74 / 106 / 106 alt byte-gleich | 2026-11-06 |
| (1) 5 gated `bake` | Agent | bestanden: je 8/8; Zimmer gebacken, apicula 1 110 / 1 110, 0 Pixel | 2026-11-06 |
| (1) 6 `scripts/gate.sh` | Agent (Hauptsitzung) | offen — läuft in der Hauptsitzung auf main nach dem Merge | – |
| (2) Unit-Punkte des Gates | Agent | bestanden, alle sieben Tests grün (Tabelle in (2)) | 2026-11-06 |
| (3) Szenarien `pt-bake`, `hg-bake`, `hgus-bake` | Agent + Blick | bestanden: Rückgabe 0, `check-data` 2/2, eigene Texturen mit Block im Zimmer, Nachbarraum Vanilla (Blick des Agenten); Bestätigung beim Nutzer offen | 2026-11-05 |
| Browser-Teil | – | entfällt: 6.1 hat keine Oberfläche | – |
| (4) Blick auf die sechs Bilder | Hand | offen | – |
| (4) `docs/formats/nsbmd.md` gelesen | Hand | offen | – |

Zahlentabelle (Release; (1) aus den gated Tests, Szenario aus (3)):

| Spiel | Modelle gleich | Pakete gleich | Wörterbücher | veraltete Zähler | Testfläche (Byte, Anteil) | Zimmer (Byte, VRAM, Meldungen) | Szenario | Last |
|---|---|---|---|---|---|---|---|---|
| Platin DE | 666 / 666 | 145 (Karte 74, Gebäude 71); Anhängen 74 / 74 | 4 286 (Modelle 3 996) | keine | 35 208 außen, 35 188 innen (57,3 %) | Karte 187, Paket 20: 35 020 B (57,0 %), VRAM 21 656, keine | `pt-bake` 0, 2/2, 415 → 414 | gated 22,7 → 23,9; Szenario um 37 |
| HeartGold DE | 676 / 676 | 200 (106, 94); 106 / 106 | 4 456 (Modelle 4 056) | 0468, 0475, 0476 | wie Platin (synthetisch) | Karte 217, Paket 25: 35 024 B (57,0 %), VRAM 23 544, keine | `hg-bake` 0, 2/2, 64 → 63 | gated 22,5 → 23,2; Szenario um 37 |
| HeartGold US | 676 / 676 | 200 (106, 94); 106 / 106 | 4 456 (Modelle 4 056) | 0468, 0473, 0475, 0476 | wie Platin (synthetisch) | Karte 217, Paket 25: 35 024 B (57,0 %), VRAM 23 544, keine | `hgus-bake` 0, 2/2, 64 → 63 | gated 21,8 → 21,4; Szenario um 37 |

„Last“ ist die 1-Minuten-Last aus `uptime` vor `vanilla_nitro` → vor `bake`. Die Testfläche ist synthetisch (32 × 32 Kacheln, vier Texturen 2 × 2, Block) und hängt nicht vom Spiel ab; im Zimmer ist sie innen gebacken mit den Texturen des Pakets.

## Befunde und Entscheidungen

Kein Befund in (1) bis (3), der einen Fix in diesem Task brauchte. Die Entscheidungen der Umsetzung, die für Nutzer und Folgephasen zählen (aus dem Ledger der Umsetzung):

- **Paletten mit 16 Farben auf 16 Byte ausgerichtet** (Abweichung von der Planzeile „Paletten je auf 8 Byte“): die Hardware zählt `PLTT_BASE` in 16-Byte-Einheiten außer bei 4-Farben-Paletten; `push` legt darum eine angehängte Palette mit 16 Farben auf eine gerade 8-Byte-Einheit, eine mit 4 Farben auf 8 Byte. Der gated Test zählt die Vanilla-Paletten mit 16 Farben auf ungerader 8-Byte-Einheit: 0 in allen drei Spielen. Kosten: ein paar Füllbytes je Anhängen; ein angehängtes Paket ist nicht byte-gleich zum Prototyp (kein Test hängt daran). Der Leser zeigt die Lücke hinter einer 4-Farben-Palette als Palette mit 0 Farben (dokumentiert in `docs/formats/nsbtx.md`).
- **Genaue Farbzahl:** `push` nimmt nur genau 4 Farben (Format 2) bzw. 16 Farben (Format 3), sonst `BadSize`; `texconv` füllt auf 4 bzw. 16 auf. Jeder Texel-Index muss unter der Farbzahl liegen.
- **Strenges Wörterbuch-Lesen in `nsbmd_header` und `map_import`:** die Namen liest jetzt `nitro_dict::read`; ein Modell mit einem Wörterbuch, das der Writer nicht so schreiben würde (andere Kopf-Felder), lehnt der Import ab. Alle Vanilla-Modelle und die Ausgabe von g3dcvtr haben das Vanilla-Layout; ein fremd gebautes NSBMD mit abweichenden Feldern käme beim Import nicht mehr durch.
- **Attrappen-Gebäudepakete zählen 0 VRAM:** ein Member des Gebäudetextur-Archivs, das kein BTX0 ist (in HeartGold Attrappen von 4 Byte), zählt im VRAM des Gebiets 0 Byte; das gilt still für jedes Nicht-BTX0-Member.
- **Rahmen des gebackenen Modells in Ecken-Einheiten:** die Box steht wie im Prototyp in Ecken-Einheiten (`box_scale` 2 für die Testfläche), Vanilla hält Welteinheiten (`box_scale == up_scale`). Im Spiel gezeigt und nicht weggeschnitten, in allen drei Spielen (Bilder `zimmer.png`). Schneidet die Engine einmal nach der Box (Außenkarten, Kamera am Kartenrand), ist der Fix in `crates/g4-kernel/src/bake/encode.rs` `bounding_box`: die Box aus Weltecken, für ±256 `box_scale = 64 · 4096 = up_scale` wie Vanilla.
- **Eintrag „nsbmd“ in `FORMATS`** (`apps/studio/src-tauri/src/mcp/docs.rs`, kein Anhängepunkt): Pflicht für `table_covers_every_format_file`; beim Merge trifft er die Zeile von 6.2 (`bdhc`), beide behalten.
- **Textprojekt:** der Abgleich nach `append_textures` schreibt `data/map_textures/NNNN.nsbtx` (Test mit `TextFixture::exported()`), also kein Befund für 6.3b.
- **`save` nach `bake_test`:** `written` nennt nur die Karte, obwohl `changed` von `bake_test` auch `MapTexture` nennt; das Paket ist schon mit dem Dateischritt geschrieben (`check-data` findet es in `out.nds`). Kein Fehler, nur ein Hinweis.

**Für 6.3b und 6.5a** (Schnittstellen wörtlich, `crates/g4-kernel/src/bake/`):

- `pub struct Face { pub corners: [[i32; 3]; 4], pub uv: [[i32; 2]; 4], pub texture: String, pub layer: u8 }`: Ecken in Welteinheiten (16 je Kachel, y oben), von oben gesehen gegen den Uhrzeigersinn; `uv` in Texeln, weltgebunden; `texture` der Name im Kartentexturpaket (≤ 16 Byte). Mit 6.3a abgestimmt (`MeshFace` → `Face` per `From`).
- `pub fn bake(faces: &[Face], input: &BakeInput) -> BakeReport` mit `BakeInput { style: AreaStyle, textures: &[PackTexture], vram_bytes, vram_limit }`; `BakeReport { nsbmd, size, share_of_limit, textures, vram_bytes, verts, polys, strips, single_quads, messages }`, `refusal()` nennt den ersten Fehler.
- `pub fn bake_for_map(store: &mut Store, map: u16, faces: &[Face]) -> Result<BakeReport, KernelError>` über `bake_context(store, map)` (Gebiet und Paket aus dem ersten Header der Karte).
- `pub fn append_textures(store: &mut Store, pack: u16, textures: &[NewTexture], label: &str) -> Result<AppendReport, KernelError>`: ein Undo-Schritt, Diamant/Perl abgelehnt, VRAM je Gebiet des geteilten Pakets.
- Prüfcodes (`BakeCode::as_str`): Fehler `bake.empty`, `bake.model-too-big` (Größe, Grenze), `bake.texture-missing` (Name), `bake.face-out-of-range` (Index, x, y, z), `bake.face-uv-range` (Index, Achse, Spanne); beim Anhängen `texture.side`, `texture.name` (auch im Bäcker, mit Name), `texture.taken`, `texture.pack-full`; Warnung `bake.vram-high`, aus `bake` mit [VRAM, Maximum], aus `append_textures` mit [Gebiet, VRAM, Maximum] (eine je Gebiet). Die Codes stehen in `docs/formats/nsbmd.md`, „Teilmenge des Bäckers“.

## Offen / später

Kleine Punkte aus den Reviews, bewusst zurückgestellt (keiner betrifft das Gate):

- `nitro_dict`: Tests der Layout-Ablehnung prüfen nur `is_err()`; der Leser nimmt Namen an, die der Writer ablehnt (Doppelte, Byte 15 ≥ 0x80), im Modulkopf nicht genannt; `Dict::entry` außerhalb gibt eine leere Scheibe.
- `gpu`/`sbc`: die Proptest-Strategie trifft selten gültige Ströme; das Feld `max` von `OutOfRange` ist bei Tabellenlücken irreführend; `count()` übergeht `BEGIN_VTXS` ohne Parameter.
- `nsbmd`: die Kürzungsschleife 2 in `model.rs` passt die Modellgröße nicht an; der Doc-Kommentar von `check_model_bytes` nennt die alten Regeln; die LE-Helfer stehen dreifach; `nsbmd_header.md` sagt `BadSize` für jede Abweichung (Revision und +4 geben `BadSignature`).
- `texconv`: `to_bgr555` schneidet ab, `nearest()` rechnet in RGB8 (ein Pixel kann im Pfad mit wenigen Farben die Nachbarfarbe bekommen); `check_side` doppelt.
- `bake`: die Grenze von `bake.face-uv-range` ist lockerer als Doku und Meldung; die Tabelle der Prüfcodes in `nsbmd.md` nennt bei `texture.*` keine Argumente, `texture.name` trägt [Name]; `texture.name` je Textur bei geteiltem ungültigem Palettennamen; die Suche des Strip-Anfangs ist bei böswilliger Reihenfolge quadratisch.
- `bake_test`: der Paket-Check für `IPKE` druckt nur (sollte `assert_eq` 25 sein); VRAM-Warnungen anderer Gebiete des Pakets fallen weg; ein gescheiterter Import nach dem Anhängen hinterlässt einen Redo-Eintrag und Dateien im `outDir`; schwache Zusicherungen im Ablehnungstest; die GLB-Prüfung kennt Typ, `normalized` und `sparse` nicht.
- Befehl: eine Ablehnung über `dispatch` kann ein leeres `build/` hinterlassen; Dateien im `outDir` ohne Symlink-Prüfung; der Tauri-Befehl nimmt die Sitzungssperre zweimal.
- Szenarien: die Spielnotiz in `pt-bake.yaml` ist knapper als in den HG-Dateien; „unten ist Karte 216“ in `hg-bake`/`hgus-bake` stammt aus dem Kernel-Test, nicht aus dem Spiel.
