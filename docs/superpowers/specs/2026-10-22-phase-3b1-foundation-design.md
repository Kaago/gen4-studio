# Phase 3b.1: Fundament (neue Header, Höhen, Sprites, Kachelnamen, apicula, Gate-Skript)

Stand: 2026-10-22. Teil 1 von Phase 3b der [Roadmap](../../ROADMAP.md). Baut auf dem Welt-Explorer auf ([Spec 3a](2026-10-22-world-explorer-design.md)).

## 1. Ziel und Umfang

Phase 3b (Daten-Editoren, Skripte, Text, Import, Prüfungen) ist zu groß für eine Spec. Sie läuft in vier Teilprojekten mit je eigener Spec, eigenem Plan und eigenem Gate:

| Teil | Inhalt |
|---|---|
| 3b.1 Fundament (diese Spec) | Phase-2-Reste plus neue Header per ARM9-Patch |
| 3b.2 Skripte und Text | rotom, CodeMirror 6 (geändert 2026-10-23, siehe Spec 3b.2), chatot, Text-Editor |
| 3b.3 Daten-Editoren | Encounter, Trainer, Pokémon, Moves, Items |
| 3b.4 Import und Prüfungen | Kartenimport, Projektprüfungen |

3b.1 liefert sieben Posten:

1. Neue Header anlegen, DSPRE-kompatibel über dessen Patch „Dynamic Headers“, angeboten im Assistenten.
2. Höhen (`altitudes`) der Matrix in der 3D-Szene auswerten.
3. Objekt-Sprites statt gelber Marker für Objekt-Events.
4. Kachelverhalten-Namen je Spielfamilie (HGSS bekommt seinen eigenen Satz).
5. apicula mit Zeitlimit und außerhalb des Async-Pools aufrufen.
6. Modell-Cache-Schlüssel mit apicula-Version.
7. Gate-Skript statt CI: die vollen Tests laufen lokal gegen die drei Testprojekte.

Nicht in 3b.1: Header löschen, DSPREs ARM9-Expansion oder andere Patches, die OW-Tabellen-Erweiterung (`OWTBLXPANDV1`), Animation der Sprites, CI auf fremden Rechnern.

**Gate.** Auf Kopien von Platin DE und HeartGold DE (unter `../Roms/_testkopien`):

- Ein neues Gebiet entsteht per Assistent mit einem **neuen** Header; der Patch wird dabei nach Nachfrage gesetzt. Das ROM baut, das Gebiet ist in melonDS begehbar, und das Projekt öffnet weiterhin in DSPRE (DSPRE zeigt den Patch als angewendet und den neuen Header in seiner Liste).
- Eine Matrixzelle mit Höhe 2 liegt in der Szene eine Kachel höher als ihre Nachbarn; Gebäude, Overlay und Marker stehen mit darauf.
- Personen und Trainer zeigen ihr Sprite in Blickrichtung; ein Objekt ohne Sprite zeigt den Marker wie bisher.
- Auf HeartGold zeigt der Hover HGSS-Kachelnamen (zum Beispiel `HEADBUTT`, das es in Platin nicht gibt).
- `scripts/gate.sh` läuft grün gegen alle drei Projekte.

## 2. Neue Header (Dynamic Headers)

### 2.1 Fakten

Die Header-Tabelle liegt fest in `arm9/arm9.bin` (Offset je Spielcode in `metadata/header_tables.yaml`), die Anzahl folgt aus `files/fielddata/maptable/mapname.bin` (16 Byte je Header). DSPRE löst das mit dem Patch „Dynamic Header Allocation“ (`DS_Map/Resources/ROMToolboxDB/ToolboxDB.cs` Z. 272–390, `PatchToolboxDialog.cs` Z. 1210–1345):

- Eine 20-Byte-Thumb-Routine bei `initOffset` lädt den Header als Member `id` aus einem NARC (Platin NARC 0x94 = `files/debug/cb_edit/d_test.narc`, HGSS NARC 0x32 = `files/a/0/5/0`). Entpackt: `unpacked/dynamicHeaders`, eine Datei `NNNN` je Header mit genau den 24 Bytes des Headers.
- An 20 (Platin) bzw. 26 (HGSS) Stellen wird `mov r1,#0x18; mul r1,r0` (`18 21 41 43`) durch `lsl r1,r3,#0; nop` (`19 00 C0 46`) ersetzt, und das zugehörige Literal `Tabellenbasis + n` wird zu `n` (Wert − `header_table_offset` − `0x02000000`).
- Alle Offsets sind für Platin-US (`initOffset` 0x3A024) und HG-US (0x3B268) notiert und werden um `pointerDiff = initOffset(Spielcode) − initOffset(Basis)` verschoben. HGSS außer Japanisch und Spanisch hat einen Sonderfall: `19 00` bei 0x3B522, `C0 46` bei 0x3B526, `00 00 00 00` bei 0x3B53C (plus `pointerDiff`).
- Erkennung: `u16` LE bei `initOffset` == `0xB500` (der `push {lr}` der Routine).
- Diamant/Perl: kein Patch. Die alte Tabelle im ARM9 bleibt stehen und wird nur nicht mehr gelesen. DSPRE legt `arm9.bin.backup` an.
- Die Offsets wurden am Platin-DE- (CPUD: `18 21 41 43` bei 0x3A0E2, Literal 0x020E6074 bei 0x3A0EC) und am HeartGold-DE-Projekt (IPKD: `18 21 41 43` bei 0x3B282, Literal 0x020F6B95 bei 0x3B28C) nachgeprüft. Für IPGS (SoulSilver ES) nimmt DSPRE den Default-Zweig, obwohl IPKS einen eigenen hat; das ist dort nicht belegt und wird von der Vorprüfung (2.3) abgefangen.

Nach dem Patch entsteht ein neuer Header, indem die Datei `NNNN` (N = bisherige Anzahl) angelegt und `mapname.bin` um einen 16-Byte-Namen verlängert wird. DSPRE kopiert Header 0 und nennt ihn `NEWMAP`.

### 2.2 Entscheidungen

- **DSPRE-kompatibel.** Gen4 Studio wendet denselben Patch an (gleiche Bytes, gleiches NARC, gleicher Ordner) und erkennt ihn mit derselben Signatur. Ein Projekt, in dem DSPRE den Patch gesetzt hat, wird ohne weiteres Zutun erkannt. Die Daten stehen in `crates/g4-kernel/metadata/dynamic_headers.yaml` mit Quellenangabe. Es sind Tatsachen über die Spielbinaries (Offsets, Maschinencode des Spiels), keine Übernahme von DSPRE-Programmcode; DSPRE steht unter AGPL-3, Gen4 Studio unter MIT.
- **Auslöser im Assistenten.** Im Schritt „Header“ der Assistenten „Gebiet anlegen“ und „Raum anlegen“ gibt es zusätzlich **„Neuer Header“**. Ist der Patch nicht gesetzt, erklärt derselbe Schritt, was er ändert (ARM9-Bytes, Header wandern in eine Datei je Header, DSPRE kennt den Patch, er lässt sich nicht zurücknehmen, `arm9.bin.backup` wird angelegt) und bietet „Patch anwenden“. Erst danach lässt sich „Neuer Header“ wählen. Kein eigenes Menü.
- **Diamant/Perl:** „Neuer Header“ wird nicht angeboten; der Schritt sagt „Neue Header gibt es nur für Platin und HeartGold/SoulSilver“.
- **Die Header-Tabelle bleibt ein Asset** (`AssetKind::HeaderTable`, Index 0, Undo wie heute). Nur ihr Ort wechselt: ohne Patch `Location::Slice` in `arm9.bin`, mit Patch `Location::HeaderFiles { dir, count }` (Ordner `unpacked/dynamicHeaders`, `count` aus `mapname.bin`). Lesen liefert die Verkettung der Dateien `0000..count`; Speichern schreibt nur die Dateien, deren 24 Bytes sich geändert haben; die Stale-Prüfung vergleicht die Verkettung wie bisher.
- **Build.** `archives.yaml` bekommt für `pt` und `hgss` den Eintrag `dynamic_headers` (packed `files/debug/cb_edit/d_test.narc` bzw. `files/a/0/5/0`, unpacked `dynamicHeaders`), dazu `AssetKind::DynamicHeaders` (store-only wie `HeaderTable`: nicht editierbar, nie lazy entpackt). Damit packt `build` den Ordner, sobald er existiert. In einem ungepatchten DSPRE-Projekt existiert er bereits (die Vanilla-Member des NARC) und packt byte-identisch zurück; der gated Rebuild-Test deckt das ab.
- **Anlegen** (`Store::create_header(template, internal_name) -> u16`) verlangt den Patch, kopiert den Vorlage-Header so, wie der Editor ihn zeigt (ungespeicherte Änderungen eingeschlossen), schreibt die Datei `NNNN` und hängt den Namen an `mapname.bin` an. Wie `create_matrix` sofort auf Platte und nicht rückgängig. Höchstens 10 000 Header, Nummern 0000 bis 9999 (vierstellige Dateinamen, DSPREs Sortierung). Der interne Name ist ASCII, höchstens 16 Zeichen, aus dem Namen des Assistenten abgeleitet (Großbuchstaben, Ziffern, Unterstrich; leer → `NEWMAP`). Der Ortsname (`location_name`) ist der der Vorlage, der neue Header erscheint also unter deren Ort.
- **Assistenten.** `AssignHeader` bekommt `new_header: Option<{ template: u16, internal_name: String }>`. Ist es gesetzt, muss `id` gleich der aktuellen Header-Anzahl sein (die Nummer, die der neue Header bekommt); der Backend-Befehl legt ihn als ersten Schritt an, vor den Kartenkopien. Schlägt danach etwas fehl, nennt die `partial`-Meldung auch den Header. `check_header_fits` prüft die Vorlage mit der künftigen Nummer. Die Vorlage ist der Header, dessen Area-Data der Schritt ohnehin vorbelegt (Gebiet: erster gewählter Quell-Header; Raum: Header der Vorlagen-Karte), sonst Header 0.
- **Patch anwenden** läuft im Store (`Store::apply_dynamic_headers()`): verweigert, wenn die Header-Tabelle ungespeicherte Änderungen hat („erst speichern“); legt `arm9.bin.backup` an, wenn es fehlt; schreibt zuerst den Ordner (vollständig, über einen temporären Nachbarordner und `rename`), dann `arm9.bin` atomar; lädt danach die Tabelle aus den Dateien neu. Die Undo-Historie der Tabelle wird verworfen (sie war leer, weil nichts dirty war).

### 2.3 Vorprüfung und Fehler

Vor dem Schreiben prüft `apply`: Spielcode bekannt und Familie Platin oder HGSS; `u16` bei `initOffset` ≠ `0xB500`; an jeder Referenzstelle (Item1 + `pointerDiff`) stehen `18 21 41 43`; jedes Literal (Item2 + `pointerDiff`) liegt in `[Tabellenbasis, Tabellenbasis + 24)` als RAM-Adresse; im Sonderfall stehen bei 0x3B522 + diff `18 21` und bei 0x3B526 + diff `41 43`. Schlägt eine Prüfung fehl, wird nichts geschrieben und die Meldung nennt den Offset. So bleibt ein Spielstand mit anderen Patches oder eine nicht belegte Region (IPGS) unverändert.

Fehlercodes bleiben `stale`, `read_only`, `tool_missing`, `partial`, `error`.

### 2.4 Befehle

- `header_patch_status() -> HeaderPatchDto { supported: bool, applied: bool, narc: String | null }`
- `apply_header_patch() -> HeaderPatchDto` (läuft unter dem State-Lock, danach `headerTable`-Refetch im Frontend)
- `create_area`, `create_room`: `AssignHeader.new_header` wie oben.

## 3. Höhen in der Szene

Platin berechnet die Y-Position eines Kartenmodells als `altitude × (MAP_OBJECT_TILE_SIZE / 2)` mit `MAP_OBJECT_TILE_SIZE = 16 × FX32_ONE` (pokeplatinum `src/overlay005/land_data.c`, `LandDataManager_CalculateRenderingPosition`). Eine Höhenstufe ist also **eine halbe Kachel**: `ALTITUDE_STEP = 0.5` Szeneneinheiten.

- `cellY(matrix, col, row)` = `(matrix.altitudes?.[row*width+col] ?? 0) × 0.5`. Ohne Höhenabschnitt bleibt alles bei 0.
- Angehoben wird alles der Zelle: Kartenmodell, Gebäude (Zellenursprung bekommt ein `y`), Overlay-Kacheln (`OVERLAY_Y + cellY`), Marker und Sprites (nach der Zelle der Kachel des Events), Trigger-Flächen, Auswahlrahmen.
- Picking: statt einer Ebene bei `OVERLAY_Y` werden die verschiedenen Höhen der Matrix (höchstens 256, meist wenige) von oben nach unten durchprobiert; der erste Treffer, dessen Zelle genau diese Höhe hat, gilt. Drag-Ebenen von Events und Gebäuden liegen auf der Höhe ihrer Zelle.
- Eine Höhenänderung im Rastereditor zählt als Layoutänderung (`sameLayout` vergleicht auch `altitudes`), die Szene lädt die Matrix neu. Kartenbilder (Draufsicht) ändern sich nicht.

## 4. Objekt-Sprites

### 4.1 Fakten

- Das Objekt-Event trägt `graphics_id` (DSPRE „OW Entry“) und `dir` (uxie `ObjectEvent.dir`: 0 oben, 1 unten, 2 links, 3 rechts). `graphics_id` ist kein Dateiindex; die **Overworld-Tabelle** bildet ihn auf eine Sprite-Datei ab.
- Tabelle (DSPRE `RomInfo.SetOWtable`/`ReadOWTable`): Diamant/Perl und Platin in Overlay 5 (`arm9_overlays/ov005.bin`), 8-Byte-Einträge `u32 entry, u32 sprite`; HGSS über einen Zeiger in Overlay 1 (`ov001.bin`) auf 6-Byte-Einträge `u16 entry, u16 sprite, u16 props`. Ende bei `entry == 0xFFFF`. ds-rom legt Overlays entpackt ab (`overlays.yaml` nennt `base_address` und `code_size`), also kein BLZ. Nachgeprüft: Platin DE 440 Einträge ab 0x2BC50 (Eintrag 0 → Sprite 90), HeartGold DE 901 Einträge (Zeiger bei RAM 0x021F92DC, Tabelle bei 0x21BA8 in ov1, Eintrag 1 → Sprite 0).
- Sprites: NARC `files/data/mmodel/mmodel.narc` (DP, Pt) bzw. `files/a/0/8/1` (HGSS), Member = `sprite`. Jede Datei ist ein BTX0 (NSBTX) mit mehreren Texturen (eine je Frame) und mindestens einer Palette. DSPRE rendert Textur `i` mit Palette 0; `i` nach Texturanzahl `n` und Richtung:

  | n | oben | unten | links | rechts |
  |---|---|---|---|---|
  | ≤ 1 | 0 | 0 | 0 | 0 |
  | ≤ 4 | 0 | 1 | 2 | 3 |
  | ≤ 8 | 0 | 2 | 4 | 6 |
  | ≤ 16 | 0 | 11 | 2 | 4 |
  | > 16 | 0 | 27 | 2 | 4 |

- DPPt-IDs 91–96 und 101–116 sind laut DSPRE 3D-Overworlds (Schilder, variablenabhängige Figuren) ohne Sprite.
- Texturparameter (NSBTX): Format = Bits 10–12 (1 A3I5, 2 vier Farben, 3 sechzehn Farben, 4 256 Farben, 5 4×4-Texel, 6 A5I3, 7 direkt), Breite `8 << ((p >> 4) & 7)`, Höhe `8 << ((p >> 7) & 7)`, `color0`-Flag Bit 13. Paletten BGR555, 4 bpp mit niedrigem Nibble zuerst.

### 4.2 Entscheidungen

- `g4-formats::nsbtx`: eigener kleiner Decoder (BTX0/TEX0 lesen, Texturen und Paletten auflisten, eine Textur mit einer Palette nach RGBA8 dekodieren). Formate 1, 2, 3, 4, 6, 7; 5 ist `Unsupported`. Der Decoder beachtet `color0`; die Sprite-Schicht setzt Index 0 zusätzlich immer transparent. Referenzen: apicula `src/nitro/tex.rs` (0BSD, lokal im Cargo-Git-Checkout), DSPRE `LibNDSFormats/NSBTX/nsbtx.cs`. Formatdoku `docs/formats/nsbtx.md`.
- `g4-kernel::sprites`: `metadata/overworld_tables.yaml` (DP/Pt: Offset in ov5 je Spielcode; HGSS: RAM-Adresse des Zeigers je Spielcode), `OverworldTable::load(project)`, `object_sprite(project, graphics_id, facing) -> Option<Rgba>`; Sprites werden direkt aus dem gepackten NARC gelesen (kein Entpacken, kein `AssetKind`). Ein HGSS-Zeiger außerhalb von Overlay 1 (synthetisches Overlay, ov131) ist ein Fehler „Sprite-Tabelle liegt außerhalb von Overlay 1 (Patch), nicht unterstützt“; die Szene zeigt dann Marker. Formatdoku `docs/formats/overworld_table.md`.
- Tauri: `ObjectDto.dir: i16`, `EventPatch.dir`, Inspector-Feld „Richtung“; `get_object_sprite(graphics_id, facing) -> SpriteDto { width, height, rgba } | null` (null: kein Eintrag, 3D-ID, Format nicht unterstützt). Tabelle und dekodierte Sprites werden je Session gecacht (`Session.sprites`).
- Szene: Objekt-Marker werden `THREE.Sprite`s (Billboards) mit Anker unten (`center = (0.5, 0)`) auf der Kachelmitte in Zellenhöhe, Größe `Breite/16 × Höhe/16` Kacheln, `NearestFilter`. Bis die Textur da ist, und bei `null` oder Fehler, zeigt der Sprite ein gelbes Quadrat in Markergröße. Auswahl, Hover, Drag und Outline laufen über denselben Pfad wie bisher (`entries` nimmt Sprites auf). Texturen werden je Projekt gecacht und beim Projektwechsel verworfen (gleicher Ort wie der Kartenbild-Cache).

## 5. Kachelverhalten-Namen je Familie

- `metadata/tile_behaviors_hgss.yaml` aus pokeheartgold `include/constants/metatile_behavior.h`, Commit `6988158691eb6178cff697cda0428045ddcba5fb`: `TILE_BEHAVIOR_<NAME>` ohne Präfix; rein numerische Einträge (`TILE_BEHAVIOR_4`) und `_UNUSED` entfallen; `class` nach denselben zwölf Klassen wie bisher (`grass`, `water`, `ice`, `sand`, `ledge`, `block`, `slide`, `warp`, `door`, `bridge`, `furniture`, `other`), zugeordnet nach Namensbestandteilen.
- `tiles::tile_behaviors(family)` und `tile_behavior(family, value)`; DP nutzt den Platin-Satz. Der Tauri-Befehl `tile_behaviors` liefert den Satz des offenen Projekts; der Frontend-Cache gilt je Projekt und wird beim Projektwechsel geleert.

## 6. apicula

- `nsbmd_to_glb` startet apicula als Kindprozess und wartet höchstens `G4_APICULA_TIMEOUT_SECS` Sekunden (Standard 60, Umgebungsvariable für Tests), dann `kill` und `KernelError::ToolTimeout { name: "apicula", secs }` („apicula hat nach 60 s nicht geantwortet“, Code `error`). Ein Timeout schreibt **keinen** `.fail`-Marker.
- `get_map_model_glb` und `get_building_glb` laufen wie `build_rom` in `tauri::async_runtime::spawn_blocking`, nicht auf dem Async-Pool.
- Cache-Schlüssel: Salt `g4glb-v3`, dann der xxh3-64 der apicula-Binärdatei (einmal je Pfad und Prozess berechnet), dann NSBMD und NSBTX. `.fail`-Marker teilen den Schlüssel, ein apicula-Update entwertet beide. Der zuletzt benutzte Werkzeug-Hash steht in `<Projekt>/.g4/models/last-tool`; fehlt apicula, werden Treffer unter diesem Hash gesucht, damit gecachte Modelle ohne apicula sichtbar bleiben.

## 7. Gate-Skript statt CI

Es gibt kein Remote und keine CI. `scripts/gate.sh` (Repo-Wurzel) fährt der Reihe nach: `cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features`, `pnpm test` und `pnpm build` in `apps/studio`, dann je Testprojekt unter `$G4_ROMS_DIR` (Standard `../Roms`, Muster `*_DSPRE_contents`) `cargo test --workspace --all-features` mit `G4_TEST_PROJECT`, `G4_APICULA` und `G4_DSROM_CLI`. Es hält beim ersten Fehler, druckt am Ende eine Tabelle Schritt/Projekt/Ergebnis und endet mit Exit-Code ≠ 0 bei einem Fehler. Die Originale werden nur gelesen (`vanilla_copy()`). `docs/dev-setup.md` beschreibt es; die Roadmap-Posten „CI“ verweisen darauf.

## 8. Tests

- Synthetisches Platin-Projekt (`test_support`) wächst um `d_test.narc`, die Referenzstellen (`18 21 41 43` und Literale `Tabellenbasis + n`) und die Vorbedingungen des Patches. Darauf: Vorprüfung lehnt fremde Bytes ab, `apply` schreibt genau die erwarteten Bytes, `is_applied`, Header lesen/editieren/speichern über Dateien, Stale-Erkennung je Datei, `create_header` (Datei, `mapname.bin`, Listing), Build packt das NARC.
- Gated (Platin DE, HeartGold DE, HeartGold US, je auf `vanilla_copy()`): Vorprüfung besteht, `apply` + Listing unverändert, `create_header` + Build läuft, Rebuild ohne Änderungen byte-identisch (bestehender Test, jetzt mit `dynamic_headers`-NARC).
- Overworld-Tabelle gated: Platin DE 440 Einträge, Eintrag 0 → 90; HeartGold DE 901 Einträge, Eintrag 1 → 0. Sprite 0 beider Spiele dekodiert mit sichtbaren und transparenten Pixeln.
- NSBTX: Unit-Tests mit im Test gebauten BTX0 (16 Farben, 256 Farben, A3I5), Formatfehler.
- Szene (vitest): Höhen verschieben Modell, Gebäude, Overlay, Marker; Picking trifft die erhöhte Zelle; `sameLayout` sieht Höhenänderungen; Sprite-Marker werden gepickt und bewegt.
- apicula: gefälschtes apicula (Shell-Skript mit `sleep`) läuft in den Timeout, kein `.fail`-Marker; Schlüssel ändert sich mit der Binärdatei.
- Kachelnamen: HGSS-Satz enthält `HEADBUTT` (0x06), Platin nicht; `tile_behaviors` liefert je Familie.
