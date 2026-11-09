# Phase 6.3b: Gate prüfen

Stand: 2026-11-07. Branch `phase-6-3b-backen-build` (Worktree `.claude/worktrees/phase-6-3b`), nicht nach main gemergt; Basis `e4f248b9` (main `1af7783c` mit 6.1 Writer, 6.2 BDHC und 6.3a Weltformat, plus das Plan-Dokument). Kopf beim Gate: `643e79f1` (Tasks 1 bis 12 je von der Review-Schleife abgenommen, dazu die Fix-Commits von Task 13 und seiner Fix-Runde 1), danach nur Doku-Commits. Die Automatik ohne ROM lief auf `3871be7c`; die Fix-Runde 1 (`643e79f1`, nur Tauri-Code und Doku) wiederholte `cargo test -p studio`, fmt, clippy und `check-no-rom-data.sh`. **Das Gate lief am 2026-11-06 und 2026-11-07** aus den Läufen der Tasks: die gated Tests in Task 9 je Kopie im Release-Profil, der Browser-Teil in Task 10, die Szenarien in Task 11; Task 13 wiederholte die Automatik ohne ROM (Tabelle in (1)). `scripts/gate.sh` als Ganzes ist **offen: Lauf in der Hauptsitzung auf main** (im Worktree untersagt); die neuen gated Tests laufen in den bestehenden Schritten mit, `gate.sh` bleibt unverändert. **Danach folgte die Fix-Welle des abschließenden Branch-Reviews** (sieben Punkte, Bericht `.superpowers/sdd/2026-11-06-phase-6-3b-backen-build/final-fix-report.md`): `0e565914` (Migration v3→v4 löscht nur einfache Beifile-Namen in `data/maps`), `7c06be20` (`decode_quads` liest `BEGIN_VTXS`-Dreiecke), `95febcb0` (Abgleich: Welt-Einheit ohne Karte lehnt ab statt Karte 0), `b7b17751` (`tileset_save` und Kartentextur-Dateischritte backen die Welt-Karten des Pakets neu, über `Store::record_file_step`), `18c463b3` (world_map.md: was ein abgelehntes Backen beim Speichern schreibt), `170d6f6d` (ein abgelehntes Welt-Backen lässt den Import erfolgreich, `world.bake` im Bericht, Abgleich aufgezeichnet), `3c771c78` (ein binär geschriebener Karten-Member löscht den Welt-Datensatz: kein falsches `world.member-changed` nach einem Git-Wechsel) und dieser Doku-Commit. Danach liefen erneut, ohne ROM (`G4_ROMS_DIR` nicht gesetzt, Last ≈ 10 bis 19): `cargo fmt --all --check`, `cargo clippy --workspace --all-features --all-targets -- -D warnings`, `cargo test -p g4-formats -p g4-kernel --all-features` (2 632 bestanden, 0 Fehler, 1 ignoriert; darin `tests/textproject.rs` 64/64 ohne ROM), `cargo test -p studio` (741 + 43 bestanden, 0 Fehler), `bash scripts/check-no-rom-data.sh` (grün); Logs `target/agent/6-3b/final-*.log`. Die gated ROM-Tests liefen nicht erneut: keine Änderung berührt Form 1, `world_adopt` oder die Byte-Pfade des Backens (`decode` ist nur für Tests); der Import mit Welt-Karten ändert sich nur bei einem abgelehnten Backen und beim Datensatz, die gated Varianten von `tests/textproject.rs` wurden ausgelassen.

Phase 6.3b bringt Backen und Build: Eine Welt-Karte des Textprojekts wird gebacken. Aus dem Weltdokument entstehen Modell (Flächenliste des Vernetzers → NSBMD über den Bäcker von 6.1), Terrain (Höhenraster → BDHC über den Generator von 6.2) und Permissions (Vorgaben der Kacheln, Kantenregel, Handwerte) im Member `unpacked/<land_data>/NNNN`. Der Store hält den gebackenen Member als Map-Asset (jeder Strich backt im selben Undo-Schritt), der Build-Schritt `world` backt inkrementell je Karte, ein außen geänderter Member ist der Konflikt `world.member-changed` mit den Wegen „Neu backen“ und „Als binäre Karte übernehmen“, „Karte als Quelle übernehmen“ (Form 1, verbatim) macht jede binäre Karte zur Welt-Karte, die byte-gleich backt; eine per Befehl gemalte Karte ist in g4emu begehbar (`pt-gemalt`, `hg-gemalt`).

Roadmap: [docs/ROADMAP.md](ROADMAP.md), Abschnitt „Phase 6“, Zeile „6.3b“. Spezifikation: [2026-11-05-phase-6-kartenerstellung-design.md](superpowers/specs/2026-11-05-phase-6-kartenerstellung-design.md), §4 Zeile 6.3b und „Phasen-Gate 6“. Plan: [2026-11-06-phase-6-3b-backen-build.md](superpowers/plans/2026-11-06-phase-6-3b-backen-build.md). Formate: [world_map.md](formats/world_map.md) (Backen, Schema 2, Schritt, Form 1), [build_state.md](formats/build_state.md) (Abschnitt `world`), [text_project.md](formats/text_project.md) (Schema 4), [tileset.md](formats/tileset.md) (`tileset_edit`), [terrain.md](formats/terrain.md) (`block_edges`); Befehle: [mcp.md](mcp.md), „Welt-Karten backen (6.3b, über `g4_call`)“; Design: [design-system/README.md](../design-system/README.md), „Nachtrag 6.3b“.

Gate-Text der Spezifikation (§4, Zeile 6.3b):

> Alle 666 / 676 Vanilla-Karten mit Form 1 übernommen backen **byte-gleich**; eine gemalte Karte (Boden, zwei Höhen mit Wand, Rampe, Wasser, Gras) backt mit NSBMD ≤ 0xF000 und BDHC ≤ 0x9000 + 16, apicula liest sie, Netz gleich der apicula-Lesung (Ecken, UV, Materialien), Modellboden = BDHC-Höhe in jeder Zellmitte; `vanilla_checks` 0 Fehler; Build-Schritt `world` inkrementell (eine geänderte JSON → eine Karte), `world.member-changed` mit beiden Wegen; **Phasen-Gate**, siehe unten

Phasen-Gate 6 (§4; in 6.3b der Emulator-Teil, die Karte per Befehl gemalt):

> Eine im Studio gemalte Karte mit eigenem Gelände, Höhenstufe mit Treppe, Wasser und Gras-Kacheln und abgeleiteter Kollision wird gebaut und in eine Matrix gesetzt. Sie ist in g4emu und melonDS begehbar, auf Platin DE und HeartGold DE: die Treppe funktioniert (`expect-height`), die Wand blockiert (`walk-until-blocked`), Gras löst eine Begegnung aus (`expect-battle`), die Außenkarte sieht bei Tag und Nacht richtig beleuchtet aus (Blick). Unveränderte Projekte bauen byte-gleich, `vanilla_checks` hat 0 Fehler, `scripts/gate.sh` ist grün, DSPRE öffnet die Karte (Hand).

Dazu (Kopf der Gate-Spalte): gated Tests auf frischen Kopien von Platin DE, HeartGold DE und HeartGold US, Browser-Teil über die Test-Umgebung, `scripts/gate.sh` grün, `cargo deny` grün, Hand und Blick beim Nutzer.

So gelesen (Abweichungen aus den „Entscheidungen“ des Plans und Rulings der Umsetzung, Ledger `.superpowers/sdd/2026-11-06-phase-6-3b-backen-build/progress.md`):

- **Schritt `world` nach `textproject`**, vor `rotom` (die Spezifikation §3.5 sagt „vor `textproject`“): `world` liest Paket-, Gebiets-, Header- und Matrix-Member, die `textproject` aus der JSON schreibt; Welt-Karten sind aus den Einheiten von `textproject` ausgenommen, die Reihenfolge erzeugt also keinen Konflikt. Nach einem gescheiterten `textproject` schreibt `world` nichts.
- **Schema 2:** `maps.world` 2, Textprojekt-Schema 4. Eine Welt-Karte trägt `permissions`, `model` und `bdhc` nicht mehr (sie entstehen beim Backen); das Verbatim-Modell steht als Teil in `world.freeform = [{nsbmd}]`, das Verbatim-Terrain in `world.terrain = {bdhc}`. Die Migration `v3_to_v4` **entfernt die Handwerte der Permissions einer 6.3a-Welt** mit den übrigen abgeleiteten Teilen (vom Plan so verlangt: 6.3a-Welten waren Vorschau, nur über `g4_call --expert` erreichbar).
- **Verbatim-Modell plus Kacheln später:** eine Form-1-Welt, auf die Kacheln gemalt werden, backt das Modell nicht (`world.verbatim-mixed`, bis 6.5b); BDHC und Permissions folgen der Welt, das Map-Asset behält das Verbatim-Modell. glTF-Freiform lehnt ab (`world.freeform-later`). Die Tests für Schritt und Konflikt auf den Kopien nehmen darum Form-1-Karten (0 und 1) und ändern einen Handwert der JSON.
- **Nacht über den Schritt `clock`:** die Probe in Task 11 war positiv (melonDS DS setzt die Uhr über Kernoptionen); `g4emu` hat den Schritt `clock HH:MM`, die Szenarien machen ein Tag- und ein Nachtbild. Nacht ist damit **Agent + Blick**; der Gang in melonDS bei Tag und Nacht bleibt dazu Hand (Phasen-Gate: „in g4emu und melonDS begehbar“).
- **Bauen über „Prüfen und Bauen“:** im Browser lief der Build über Technik › „Prüfen und Bauen“ („Veröffentlichen“ ist der Patch-Assistent; dieselbe Schrittliste).
- **`WORLD_BAKE_REV` bleibt 1**, obwohl der Fix der Palettenbindung (`60109ff1`) die Bytes mancher Welten ändert: 6.3b ist nicht veröffentlicht, kein Projekt eines Nutzers trägt einen Datensatz mit Rev 1.
- **Gated Tests auf Kopien, nie auf Originalen:** jeder Test richtet auf `vanilla_copy()` von `6-3b-{pt,hgde,hgus}_DSPRE_contents` das Textprojekt ein (Basis-ROM `6-3b-<spiel>.nds` daneben); die Szenarien auf je einer frischen `6-3b-szen-*`-Kopie.
- **Laufhöhe einer übernommenen Karte** (Befund aus Task 12, behoben in Task 13, `185ce3e9`): `get_map` gab für eine Welt-Karte mit leerer Bodenebene (Form 1) überall `null`; jetzt nimmt es für jede Welt mit Verbatim-Terrain (`world.terrain`, auch nach gemalten Kacheln, bis `height` oder `ramp` es beendet) die Höhen der gebackenen BDHC wie `get_map_heights` (Fix-Runde 1, `643e79f1`); sonst gilt das Weltdokument (`null` auf `.`).

Wer was prüft: Die Spalte „Wer“ sagt **Agent** (die Test-Umgebung aus [docs/agent-gate.md](agent-gate.md): Browser-Fenster mit `studio-bridge`, `call.sh`, `mcp.sh`, dazu die gated Tests und `g4emu`), **Agent + Blick** (der Agent prüft und beschreibt Ort und Zahlen, der Nutzer bestätigt kurz am Bild) oder **Hand** (der Nutzer). Befehle: [docs/agent-gate.md](agent-gate.md), Abschnitt „Phase 6.3b“.

Namen in der Oberfläche (für die Prüfung):

- **Inspector einer Karte** (Bauen, Zelle gewählt), Abschnitt „Quelle“ über „Budget“, `[data-world-source="binary|world"]`: binär „Binäre Karte aus dem ROM oder aus DSPRE.“ und der Knopf „Karte als Quelle übernehmen“ `[data-world-adopt]` (außerhalb eines Textprojekts gesperrt: „Boden malen gibt es im Textprojekt (Technik › Textprojekt).“), danach „Übernommen.“ bzw. „Übernommen. N Kacheln weichen von der Höhe ab.“; Welt „Welt-Karte“, `[data-world-bake="ok"]` „Gebacken · Modell 38.876 B · Terrain 146 B“ oder `[data-world-bake="refused"]` mit der ersten Ablehnung, Knopf „Als binäre Karte übernehmen“.
- **Technik › Textprojekt**, „Jetzt abgleichen“: „Auf zwei Seiten geändert“, aufgeklappt „Diese Welt-Karte wurde außerhalb des Studios geändert.“, Knöpfe „Neu backen“ und „Als binäre Karte übernehmen“.
- **Technik › Prüfen und Bauen**, „Bauen“: Zeile „Karten backen“ nach dem Textprojekt-Schritt, vor „Scripts“.
- **Szene:** eine Form-1-Karte hat kein Raster und zeigt das GLB des gebackenen Modells (kein `data-world-mesh`); 0 Flächen oder ein Fehler von `get_world_mesh` → GLB.
- **Verlauf:** „Karte als Quelle übernommen: Karte NNNN“, „Welt aufgegeben: Karte NNNN“, „Tileset NNNN geändert (N Kacheln)“.
- **Befehle** (über `g4_call`, `--expert`): `world_adopt`, `world_to_binary`, `tileset_edit` (Schreiben), `world_bake` (Lesen); `world_get`, `world_create`, `world_edit` antworten mit `bake`. Konflikte über `resolve_external_conflict` (`studio` = Neu backen, `dspre` = Als binäre Karte übernehmen).
- **Prüfungen** im Textprojekt-Check: `world.bake` (Fehler, mit dem Code des Bäckers), `world.member-changed` (Warnung), `world.budget` mit den gebackenen Zahlen; `world.not-baked` entfällt.

## Vorbereitung

Durchgeführt 2026-11-06 bis 2026-11-07 (Befehle: [docs/agent-gate.md](agent-gate.md), „Phase 6.3b“):

1. **Kopien** unter `/Users/POLTEP/Projekte/lab/editor/Roms/_testkopien/` per `cp -cR` (Ordner) und `cp -c` (`.nds`), die Ziele existierten vorher nicht, die Originale nur gelesen:
   - `6-3b-pt_DSPRE_contents`, `6-3b-hgde_DSPRE_contents`, `6-3b-hgus_DSPRE_contents` je mit `6-3b-<spiel>.nds` daneben: gated Tests (Task 9; geschrieben wird nur auf `vanilla_copy()`).
   - `6-3b-szen-pt`, `-pt2`, `-pt3` und `6-3b-szen-hg`, `-hg2` (je `_DSPRE_contents` mit `.nds`): Szenarien (Task 11); maßgeblich sind die letzten Läufe auf `6-3b-szen-pt3` und `6-3b-szen-hg2`. In ihnen liegt die gebaute ROM `out.nds` für melonDS und DSPRE.
   - `6-3b-web_DSPRE_contents` (Platin DE) mit `6-3b-web.nds`: Browser-Teil (Task 10), eingerichtet mit `call.sh open_project`, `assign_base`, `adopt_text_project` (65 s).
2. **Worktree:** `G4_ROMS_DIR=/Users/POLTEP/Projekte/lab/editor/Roms` absolut; `tools` als Symlink auf den Haupt-Checkout (nie einchecken), `G4_APICULA=<Haupt-Checkout>/tools/apicula/bin/apicula`; `export PATH="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$HOME/.cargo/bin:$PATH"` vor jedem Cargo-Aufruf.
3. **Test-Umgebung** für (3) auf **1520/1521**: `G4_VITE_PORT=1520 G4_BRIDGE_PORT=1521 bash scripts/agent/studio-web.sh` (Log `target/agent/6-3b/web.log`); andere Ports nicht angefasst (1500/1501 gehörte 6.5a).
4. **Ausgaben** im Worktree (nicht im Repo): Logs `target/agent/6-3b/*.log`, Bilder und Notiz unter `target/agent/gate-2026-11-07/6-3b/` (Browser `blick-*.png`, `browser.md`; Szenarien `pt-gemalt/`, `hg-gemalt/`). Den Ordner vor `git worktree remove` sichern.
5. **Last:** `uptime` vor jedem langen Lauf, in den Logs und den Tabellen unten notiert (1-Minuten-Last zwischen 2,9 und 8,4 bei den gated Läufen). Die Maschine teilten sich zeitweise mehrere Worktrees (6.5a); ein Test, der unter Last reißt, wird allein wiederholt, die Grenze nie gelockert.

## (1) Automatik

- [ ] 1. `scripts/gate.sh` grün. **Offen: Lauf in der Hauptsitzung auf main** (nach dem Merge; dort laufen `tests/world_bake.rs` und die erweiterten `tests/textproject.rs` im Schritt der gated Tests je Projekt mit). Statt dessen liefen die Teile einzeln (Tabelle unten).
- [x] 2. **Gated Kernel-Tests** je Kopie im Release-Profil, `--nocapture --test-threads 1` (Task 9, Code `fcadf355`, Fix-Runde `33d08a94`; Logs `target/agent/6-3b/gated-<k>*.log`). Der Kernel-Code ist seit `33d08a94` unverändert (Task 13 änderte nur Tests im Kernel und Tauri-Code):
   - `tests/world_bake.rs` 5/5 je Spiel (Platin DE und HeartGold DE liefen den Gesamtlauf noch mit dem früheren Test-Helfer `dspre_member`; auf HeartGold DE scheiterte dort `member_changed_both_ways_on_the_copy` am Testbyte, Gesamtlauf 4/5. Nach der Änderung des Helfers (BGS-Byte) lief dieser Test auf beiden allein auf dem endgültigen Code, je 1/1, Logs `gated-{pt,hgde}-conflict.log`; HeartGold US lief ganz auf dem endgültigen Code), `tests/textproject.rs` 64/64 je Spiel (darin Round-Trip und Export idempotent mit einer Welt-Karte: Platin DE Karte 0000, Paket 0006; HeartGold DE und US Karte 0000, Paket 0002; bei der Welt wird der gelöschte Member vom Import neu gebacken, jede andere Datei byte-gleich; ohne Welt byte-gleich zur Basis).
   - `world-adopt CPUD maps=666 adopted=666 equal=666 deviating_maps=240 deviating_cells=163411 ms=11092`, `IPKD maps=676 adopted=676 equal=676 deviating_maps=424 deviating_cells=238631 ms=11396`, `IPKE` dieselben Zahlen (`ms=4230`). Abweichende Zellen: ohne Platte oder mit |y − 8·Stufe| > 0,1 E; die BDHC bleibt verbatim, die Bytes sind gleich.
   - `world-adopt-build`: das Projekt mit allen Karten als Welt baut die Basis-ROM byte-gleich (SHA-1), nachdem die Karten-Member gelöscht und neu importiert wurden; Check 0 Fehler (686 / 690 / 690 Hinweise und Warnungen). Platin DE 37 s, HeartGold DE 43 s, HeartGold US 25 s.
   - `world-gate … nsbmd=1536 share=0.025 bdhc=232 faces=19 floor_max_dev=0.0000 apicula=ok` auf allen drei (Karte 0, Paket 6 bzw. 2): apicula je Fläche und verschiebungsfest (jedes GLB-Dreieck passt zu einer Fläche mit ihrer Textur, UV-Versatz ein Vielfaches der Texturgröße), 38 GLB-Dreiecke = 2 × 19 Flächen, `decode_quads` = 19, Gras (20, 5) `kind 0x02` frei, Wasser (5, 26) `kind 0x15` frei, Tür (12, 21) Handwert `{0x69, 0x80}`.
   - `world-step … baked=1 of=2` (eine geänderte JSON → eine Karte, 385 bis 463 ms); `world-conflict … rebake=ok binary=ok` auf allen drei.
- [x] 3. **`vanilla_checks`** je Kopie 3/3, jeder Fehlercode 0 Treffer (der Test prüft es und jede Zahl gegen die feste Zählung): Platin DE 264 Befunde (`map.step-unblocked` 152), HeartGold DE 198 (52), HeartGold US 206 (52); dieselben Zahlen wie in 6.2.
- [x] 4. **Automatik ohne ROM** in Task 13 auf `3871be7c` (Skript `target/agent/6-3b/t13-checks.sh`, Logs `target/agent/6-3b/t13-<name>.log`, Zusammenfassung `t13-summary.log`):

| Prüfung | Ergebnis | Log | Last vor dem Lauf (1, 5, 15 min) |
|---|---|---|---|
| `cargo deny --locked check licenses advisories` | Rückgabe 0, `advisories ok, licenses ok`; bekannte Warnung `warning[no-license-field]` für `chatot = 0.8.1` (vorher schon da); 6.3b bringt keine neue Abhängigkeit | `t13-deny.log` | 4,08 6,16 4,64 |
| `cargo fmt --all --check` | Rückgabe 0 | `t13-fmt.log` | 4,08 6,16 4,64 |
| `cargo clippy --workspace --all-features --all-targets -- -D warnings` | Rückgabe 0, keine Warnung | `t13-clippy.log` | 4,08 6,16 4,64 |
| `env -u G4_TEST_PROJECT cargo test --workspace --all-features` | Rückgabe 0: 3 492 bestanden, 0 fehlgeschlagen, 2 ignoriert (37 min) | `t13-workspace.log` | 4,76 6,24 4,69 (während des Laufs bis 13) |
| `cargo test -p g4-formats -p g4-kernel --all-features` | Rückgabe 0: 2 624 bestanden, 0 fehlgeschlagen, 1 ignoriert | `t13-formats-kernel.log` | 12,11 13,64 13,31 |
| `cargo test -p studio` | Rückgabe 0: 784 bestanden, 0 fehlgeschlagen, 1 ignoriert (darin `an_adopted_map_has_the_heights_of_its_baked_terrain`) | `t13-studio.log` | 12,66 14,88 15,50 |
| `(cd apps/studio && pnpm test && pnpm build)` | Rückgabe 0: 418 Dateien (1 übersprungen), 3 980 Tests bestanden, 1 übersprungen; Build ok, mit der bekannten Vite-Warnung „Some chunks are larger than 500 kB after minification“ (vorher schon da) | `t13-frontend.log` | 13,20 15,15 15,48 |
| `check-styles.sh`, `check-tokens.sh`, `check-depth.sh` | `check-styles: ok`, „Alles stimmig.“, `check-depth: ok` | `t13-styles.log`, `t13-tokens.log`, `t13-depth.log` | 8,80 12,88 14,51 |
| `check-no-rom-data.sh` | „no rom data: 2060 files ok“ | `t13-norom.log` | 8,25 12,70 14,44 |
| `(cd scripts/g4emu && uv run pytest -q)` | Rückgabe 0: 415 bestanden, 3 übersprungen (Unit und Integration, darin die Tests des Schritts `clock`) | `t13-g4emu.log` | 8,25 12,70 14,44 |

Kein Test riss unter Last; keine Wiederholung nötig.

## (2) Szenarien

- [x] Task 11 (`1b480a76`), Platin DE auf `6-3b-szen-pt3`, HeartGold DE auf `6-3b-szen-hg2`, über die Brücke (Logs `target/agent/6-3b/pt-gemalt.log`, `hg-gemalt.log`): beide Rückgabe **0**, `"ok": true`, Header `[415, 593, 415, 593]` bzw. `[64, 540, 64, 540]`, `build_rom` mit dem Schritt `world` (1 Karte).
  - Befehlsfolge: `assign_base` (`BASIS`), `apply_header_patch`, `adopt_text_project`, `create_area` 1 × 1 auf neuem Header, Begegnungen (Rate 100, Mewtu Lv 5), Geschenk, `world_create`, `tileset_save`, `tileset_edit` (Gras `kind 2`, Wasser `kind 21`, Wandkachel), ein `world_edit` mit den Zellen von `gate_world`, `world_bake`, Warps, Speichern, `build: out.nds`, Spielen.
  - Acht Erwartungen erfüllt: `tileset_edit tiles = 4`, `world_edit cells = 171`, `world_bake ok = true`, `world_bake style = "outdoor"`, `get_map permissions.112 = [2, 0]` (Gras), `.770 = [21, 0]` (Wasser), `.694 = [0, 128]` (Wand), `.684 = [105, 128]` (Tür).
  - `world_bake` Karte 666 (Platin) bzw. 676 (HeartGold): `ok`, `outdoor`, 19 Flächen, Modell 1 704 B (2,8 % von 0xF000), BDHC 232 B (Nutzlast 216), 6 Platten, 12 blockierte Zellen, keine Meldung; VRAM 110 640 B (Platin, gleich dem Vanilla-Höchstwert, keine Warnung) bzw. 70 800 B.
  - Spielen: `clock 12:00`; `expect-height` 0 auf (14, 21), 16 auf (18, 21), 12 auf (17, 21) (die Treppe); `walk-until-blocked` hält auf (21, 21) und an der Wand auf (20, 23); `expect-battle` Mewtu (150) Lv 5 (Platin nach 1, HeartGold nach 7 Grasschritten); `clock 23:00`, `continue`, derselbe Gang bis an die Wand.
  - Bilder `target/agent/gate-2026-11-07/6-3b/{pt,hg}-gemalt/{tag,nacht,wasser,wiese,treppe,kante,geschenk,gras}.png`. Vom Agenten beschrieben: **tag** hellgrüner Boden mit feiner Textur, die Südseite des Plateaus als brauner Fels- bzw. Klippenstreifen (die Wandkachel), die Rampe bei x 16–17 als hellere Schräge nach Osten, neutrales Tageslicht; **nacht** dieselbe Stelle deutlich dunkelblau-türkis (Platin) bzw. dunkel blaugrau (HeartGold) getönt: das Außenlicht der Nacht wirkt auf die gebackene Karte; **wasser** das Wasserrechteck im Westen (Platin tiefblaue Wellen, HeartGold hellblau mit Glitzern); **wiese** der Grasblock im Nordosten klar vom Boden abgesetzt. Beim Nutzer offen (Blick).
  - Die Uhr: Probe `Starting the RTC at 2004-01-01 11:00:00 PM (ignoring the local time)` im Kern-Log nach `clock 23:00`; Schritt in [docs/agent-gate.md](agent-gate.md), „Phase 6.3b“.

## (3) Browser

- [x] Task 10 Step 5 auf `6-3b-web` (Platin DE), Test-Umgebung 1520/1521, 1280 × 800, Notiz `target/agent/gate-2026-11-07/6-3b/browser.md`, Code `ba1d8f91`, danach `950d83a4` (zwei Fehler aus dem Lauf behoben):
  1. Zweiblattdorf, Karte 0000 (Zelle 3, 27): `[data-world-source="binary"]`, „Karte als Quelle übernehmen“.
  2. Übernehmen → `world`, `[data-world-bake="ok"]` „Gebacken · Modell 38.876 B · Terrain 146 B“ (zuerst „Terrain 162 B“ = ganze Datei; auf die Nutzlast wie im Budget umgestellt, `950d83a4`), „Übernommen.“ (0 Abweichungen), GLB in der Szene, Verlauf „Karte als Quelle übernommen: Karte 0000“.
  3. ⌘Z → `binary` (die Notice blieb stehen: behoben in `950d83a4`, Test ergänzt); ⇧⌘Z → `world`.
  4. ⌘S → „Alles gespeichert“; `data/maps/0000.json` mit `source`, `world`, `buildings` (kein `model`, `permissions`, `bdhc`), `world.version` 2, `freeform[0].nsbmd` und `terrain.bdhc` als `base_sha1`.
  5. Member `unpacked/maps/0000` außen durch `0001` ersetzt → „Jetzt abgleichen“: Konflikt „Karte 0000“ mit „Neu backen“ und „Als binäre Karte übernehmen“; „Neu backen“ → Konflikt weg, `shasum` = e21e2a81… gleich dem Bake vor dem Ersetzen, Datensatz `world.0000`.
  6. Prüfen und Bauen „Bauen“ → „Karten backen · voll gebaut (erster Build, voll) · 0,36 s“ nach dem Textprojekt-Schritt, vor „Scripts“.
  7. Dunkel: „Als binäre Karte übernehmen“ → `binary`, „Welt aufgegeben: Karte 0000“, ⌘Z → `world`. Kompakt lesbar. Keine Konsolenfehler.
  - Bilder `target/agent/gate-2026-11-07/6-3b/blick-quelle-binaer-{hell,dunkel}.png`, `blick-quelle-welt-{hell,dunkel,kompakt}.png`, `blick-konflikt-welt-hell.png`, `blick-konflikt-wege-hell.png`, `blick-build-karten-backen-{hell,dunkel}.png`.
  - Nicht im Browser: der gesperrte Knopf im DSPRE-Layout (Unit-Test `dspre_layout_disables_adopt`).

## (4) Hand und Blick

- [ ] **Blick** (Agent + Blick): die Bilder der Szenarien, Tag und Nacht (`target/agent/gate-2026-11-07/6-3b/{pt,hg}-gemalt/tag.png`, `nacht.png`, dazu `wasser.png`, `wiese.png`), und die Bilder des Inspectors in Hell, Dunkel und Kompakt (`blick-quelle-*.png`, `blick-konflikt-*.png`, `blick-build-karten-backen-*.png`). Hinweis zum Ansehen: das Plateau trägt dieselbe Bodentextur wie die Fläche darum; die Höhe liest man vor allem am Wandstreifen und an der Schattierung der Rampe. Beim Nutzer offen.
- [ ] **Hand, melonDS bei Tag und Nacht:** `out.nds` von `pt-gemalt` (`_testkopien/6-3b-szen-pt3_DSPRE_contents/out.nds`) und `hg-gemalt` (`_testkopien/6-3b-szen-hg2_DSPRE_contents/out.nds`) in melonDS starten (mit einer Kopie des Spielstands `_testkopien/_saves/CPUD-zimmer.sav` bzw. `IPKD-zimmer.sav`), die Treppe des Spielerzimmers führt auf die gemalte Karte (Ankunft auf der Tür bei 12, 21); Treppe, Plateau, Wand, Wasser und Gras begehen; einmal bei Tag und einmal bei Nacht (Systemzeit des Emulators, z. B. 12:00 und 23:00). Beim Nutzer offen.
- [ ] **Hand, DSPRE:** DSPRE öffnet die gebaute Karte (Karte 666 in Platin, 676 in HeartGold, aus `out.nds` bzw. dem Projektordner der Szenario-Kopie): Modell, Terrain und Permissions lesen sich ohne Fehler. Beim Nutzer offen.
- [ ] **`scripts/gate.sh` auf main** (Hauptsitzung, nach dem Merge). Offen.
- [x] **Frames 6.3b in der `.fig`** (09.11.2026, Branch `phase-6-fig-nachtraege`; beim Gate antwortete OpenPencil nicht): „App: Inspector, Karte als Quelle“ auf „App: Werkbank“ (x 0, y 4000) und „App: Textprojekt, Welt-Konflikt“ auf „App: Texte, Technik, Dialoge“ (x 0, y 3000), aus `design-system/openpencil/beispiele/welt-quelle.body.js`; Anleitung `design-system/openpencil/NACHTRAG-6-3b.md`.

## Ergebnis

| Punkt | Wer | Ergebnis | Datum |
|---|---|---|---|
| (1) 1 `scripts/gate.sh` | Agent | **offen: Lauf in der Hauptsitzung auf main**; die Teile einzeln grün (Tabelle in (1) 4) | – |
| (1) 2 gated `world_bake` und `textproject` | Agent | bestanden auf allen drei Spielen: Form 1 666 / 676 / 676 byte-gleich, Projekt byte-gleich gebaut, gemalte Karte 1 536 B / 232 B, Boden 0,0000 E, apicula gleich, Schritt 1 von 2, Konflikt beide Wege | 2026-11-07 |
| (1) 3 `vanilla_checks` | Agent | 0 Fehler auf allen drei | 2026-11-07 |
| (1) 4 Automatik ohne ROM | Agent | bestanden: `cargo deny`, fmt, clippy, Workspace 3 492, Formate + Kernel 2 624, Studio 784, pnpm 3 980 und Build, Stil, Tokens, Tiefe, keine ROM-Daten, `g4emu` 415 | 2026-11-07 |
| (2) Szenarien `pt-gemalt`, `hg-gemalt` | Agent + Blick | bestanden (Rückgabe 0, acht Erwartungen, Treppe, Wand, Gras); Bilder Tag und Nacht vom Agenten beschrieben, Blick beim Nutzer offen | 2026-11-07 |
| (3) Browser | Agent | bestanden (Übernehmen, ⌘Z/⇧⌘Z, Speichern ohne abgeleitete Teile, Konflikt „Neu backen“, Zeile „Karten backen“, „Als binäre Karte übernehmen“, Hell/Dunkel/Kompakt) | 2026-11-07 |
| (4) Blick | Agent + Blick | vom Agenten beschrieben, beim Nutzer offen | – |
| (4) melonDS bei Tag und Nacht | Hand | offen | – |
| (4) DSPRE öffnet die Karte | Hand | offen | – |
| Frames 6.3b in der `.fig` | Agent | bestanden: ein Frame auf „App: Werkbank“ (Reihe 4), einer auf „App: Texte, Technik, Dialoge“ (Reihe 3), die Zeile im Brett „Stand der App“; Branch `phase-6-fig-nachtraege` (`design-system/openpencil/NACHTRAG-6-3b.md`) | 2026-11-09 |

Zahlentabelle (gated Läufe Task 9; Last = 1-Minuten-Last vor dem Lauf `world_bake` + `textproject` / vor dem Wiederholungslauf der gemalten Karte in der Fix-Runde):

| Spiel | Form 1 byte-gleich (Karten) | abweichende Karten / Zellen | Projekt gebaut byte-gleich | gemalte Karte NSBMD (Anteil) | BDHC | Boden max. Abw. (E) | apicula | Schritt inkrementell | Konflikt beide Wege | `vanilla_checks` Fehler | Szenario | Last |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Platin DE | 666 / 666 | 240 / 163 411 | ja (SHA-1 der Basis-ROM, 37 s) | 1 536 B (2,5 %) | 232 B | 0,0000 | gleich (38 = 2 × 19 Dreiecke) | 1 von 2 (401 ms) | ja | 0 | `pt-gemalt` Rückgabe 0 (Karte 666: 1 704 B, 2,8 %) | 4,87 / 8,38 |
| HeartGold DE | 676 / 676 | 424 / 238 631 | ja (43 s) | 1 536 B (2,5 %) | 232 B | 0,0000 | gleich | 1 von 2 (385 ms) | ja | 0 | `hg-gemalt` Rückgabe 0 (Karte 676: 1 704 B, 2,8 %) | 3,61 / 3,70 |
| HeartGold US | 676 / 676 | 424 / 238 631 | ja (25 s) | 1 536 B (2,5 %) | 232 B | 0,0000 | gleich | 1 von 2 (463 ms) | ja | 0 | – | 4,16 / 2,92 |

Die gemalte Karte des gated Tests (`gate_world` auf Paket 6 bzw. 2) und die der Szenarien (dieselben Zellen mit vier per `tileset_edit` gesetzten Kacheln) sind zwei Läufe; daher 1 536 B gegen 1 704 B. Gleiche Zahlen auf allen drei Spielen sind plausibel: dieselbe Welt, und der Vernetzer fasst gleich texturierte, ebene Vierecke zusammen.

## Befunde

- **Behoben in Task 13:** `get_map` gab einer übernommenen Karte (Form 1, Bodenebene ganz `.`) überall `null` als Höhe, obwohl die gebackene BDHC Höhen hat (`185ce3e9 fix(6.3b): get_map gives an adopted map the heights of its baked terrain`, Regressionstest `an_adopted_map_has_the_heights_of_its_baked_terrain` über `dispatch`); in der Fix-Runde 1 (`643e79f1`) folgt die Regel `world.terrain` statt „Bodenebene ohne Kachel“, damit eine gemalte Kachel auf einer übernommenen Karte die Höhen der übernommenen BDHC nicht verliert (der Test malt dazu eine Kachel). Zwei Doku-Ungenauigkeiten (`build_state.md`: fehlender Member gibt bei jeder Eingabe `SourceChanged`; `terrain.md`: kürzere Liste in `block_edges` ist ein Fehler des Aufrufers) mit Tests (`3871be7c`).
- Befunde der Tasks sind in ihren Fix-Runden behoben (Ledger): Task 1 (Lexer-Import, Assertion), Task 5 (Import entscheidet geänderte Member vor dem Schreiben, `--force` backt neu, `world` nach gescheitertem `textproject`), Task 8 (`world_get` bei unlesbaren Zeilen), Task 9 (Palettenbindung `60109ff1`, apicula je Fläche), Task 10 (Terrain-Zahl, Notice nach ⌘Z). Aufgeschobene Kleinigkeiten stehen im Ledger.

## Offen

- `scripts/gate.sh` als Ganzes auf main (Hauptsitzung, nach dem Merge).
- Blick auf die Bilder (Tag, Nacht, Inspector Hell/Dunkel/Kompakt), melonDS bei Tag und Nacht, DSPRE öffnet die gebaute Karte (Nutzer).
- Nach dem Merge von 6.5a (schon auf main): `decode_quads` um Dreiecke erweitern (`Face::is_triangle`) und `BAKE_REV` prüfen; Konflikte an den Anhängepunkten gesammelt lösen.
- Für 6.4a: Format-5-Texturen in der Szene, Dateischritt von `tileset_save`/`tileset_edit` an `HEADER_TABLE`, die doppelt gezählten Texel der VRAM-Schätzung; der Schritt `textproject` heißt in Bauen noch „Weiterer Schritt“; das Plateau der Szenarien trägt die Bodentextur (eine eigene Kachel für das Plateau zeigt die Höhe deutlicher).
