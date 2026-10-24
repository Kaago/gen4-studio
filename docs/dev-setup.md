# Entwicklungs-Setup

## apicula (NSBMD zu GLB)

Die Matrix-Ansicht zeigt die Map-Modelle und die Gebäudemodelle (NSBMD) mit ihren Texturen (NSBTX) als GLB an. Die Konvertierung übernimmt
[apicula](https://github.com/scurest/apicula). Für macOS gibt es kein fertiges Release, es wird aus dem Quellcode gebaut:

```sh
export PATH="/opt/homebrew/opt/rustup/bin:$PATH"
cargo install --git https://github.com/scurest/apicula --locked --root tools/apicula
```

Das Ergebnis liegt in `tools/apicula/bin/apicula` (`tools/` ist per `.gitignore` ausgeschlossen).
Gebaut und getestet mit Commit `3d4e91e14045392a49c89e86dab8cb936225588c` (`apicula 0.1.1-dev`).

### Wo die App apicula sucht

1. Umgebungsvariable `G4_APICULA` (voller Pfad zur Binärdatei). Ist sie gesetzt, gilt sie immer, auch wenn der Pfad falsch ist.
2. Eine Datei `apicula` neben dem Studio-Binary.
3. Nur in Debug-Builds: `tools/apicula/bin/apicula` im Repository.

Im App-Bundle liegt apicula als Sidecar in `Contents/MacOS/` (Eintrag 2), siehe `docs/release.md`. Eine leere
`apicula`-Datei (Platzhalter aus `build.rs`) zählt nicht als Fund.

Fehlt apicula, meldet die App `tool apicula unavailable: ...`; Permissions und Gebäude-Boxen werden trotzdem angezeigt.

```sh
G4_APICULA="$PWD/tools/apicula/bin/apicula" pnpm tauri dev
```

### Modell-Cache

Konvertierte Modelle (Karten und Gebäude) liegen unter `<Projekt>/.g4/models/`. Der Schlüssel ist der xxh3-64-Hash über
das Salt `g4glb-v3`, den xxh3-64-Hash der apicula-Binärdatei (Werkzeug-Hash, je Pfad und Prozess einmal berechnet), das
NSBMD und das NSBTX-Texturset (bei Karten `AreaData[area_data_id].map_tileset`, bei Gebäuden `buildings_tileset`), als
16 Hex-Zeichen. Es gibt drei Dateiarten:

- `<hash>.glb`: das konvertierte Modell.
- `<hash>.fail`: Marker einer fehlgeschlagenen *texturierten* Konvertierung, der Inhalt ist der Fehlertext. Wer den Marker
  findet, überspringt den texturierten Versuch und zeigt das Modell ohne Texturen (dafür liegt unter dem Hash des
  untexturierten NSBMD eine `.glb`), statt apicula bei jedem Laden erneut zu starten.
- `last-tool`: der Werkzeug-Hash (16 Hex-Zeichen) der zuletzt benutzten apicula. Fehlt apicula, werden Treffer unter diesem
  Hash gesucht, damit bereits konvertierte Modelle und ihre Marker auch ohne apicula sichtbar bleiben.

Ändert sich das Modell, das Tileset oder die apicula-Binärdatei, entsteht ein neuer Schlüssel und damit ein frischer
Versuch; alte Einträge bleiben liegen. Gibt es den Area-Data- oder Tileset-Eintrag nicht, wird das Modell ohne Texturen
konvertiert.

apicula läuft auf einem Arbeitsthread (`spawn_blocking`) und höchstens `G4_APICULA_TIMEOUT_SECS` Sekunden je Aufruf
(Standard 60). Danach wird der Prozess samt Kindprozessen beendet. Hängt der texturierte Versuch, schreibt das keinen
`.fail`-Marker (apicula war vielleicht nur langsam), das Modell erscheint ohne Texturen, und bis zum Programmende wird
dieses Tileset nicht erneut versucht (nur im Speicher gemerkt; nach einem Neustart gibt es einen frischen Versuch). Nur
wenn auch die Konvertierung ohne Texturen ins Zeitlimit läuft, erscheint die Meldung `apicula did not finish within <n> s`:

```sh
G4_APICULA_TIMEOUT_SECS=120 G4_APICULA="$PWD/tools/apicula/bin/apicula" pnpm tauri dev
```

Folgen:

- `.g4/` ist reiner Cache und darf jederzeit gelöscht werden; `.g4/models/` ebenso.
- Ein apicula-Update entwertet `.glb` und `.fail` automatisch (neuer Werkzeug-Hash), das Löschen der Marker ist
  dafür nicht mehr nötig. Einen Marker ohne Update loszuwerden geht nur von Hand: `rm <Projekt>/.g4/models/*.fail`.
  Ein fehlerhaftes `.glb` lässt sich nur durch Löschen der Datei erneuern.
- Beim Löschen darf die App nicht gerade Modelle laden. Zwischenstände (`apicula-<pid>-<n>`-Ordner) sind harmlos.

## rotom und rotom-lsp (Scripts)

Scripts werden mit [rotom](https://github.com/KalaayPT/rotom) kompiliert; die Bibliothek ist eine Cargo-Abhängigkeit von `g4-kernel`
(Git-Tag `v0.7.0`, mit Kommentar im Code, wo welche API benutzt wird). Completion, Hover und Diagnosen im Script-Editor liefert der
Sprachserver `rotom-lsp`, ein eigener Prozess (Sidecar) aus **demselben Tag `v0.7.0`**. Ohne ihn läuft der Editor weiter (Öffnen,
Bearbeiten, Speichern, Compile), nur die Sprachdienste fehlen; die Kopfzeile zeigt „rotom-lsp fehlt — Completion und Diagnosen aus“.

Bauen (einmal; `tools/` ist per `.gitignore` ausgeschlossen):

```sh
export PATH="/opt/homebrew/opt/rustup/bin:$PATH"
CFLAGS="-D_POSIX_C_SOURCE=200809L" \
  cargo install --git https://github.com/KalaayPT/rotom --tag v0.7.0 rotom-lsp --root tools/rotom
```

- `CFLAGS` ist auf macOS-SDKs ab 15.4 nötig: nitroarcs `strchrnul` kollidiert sonst mit der SDK-Deklaration (derselbe Wert steht in
  `.cargo/config.toml`; `cargo install` hat ihn dort nicht übernommen).
- **Netz beim Bauen:** rotoms `build.rs` lädt bei **jedem sauberen Build** (auch von `g4-kernel`, das rotom als Bibliothek einbindet) `db-latest.zip` aus dem Netz, dazu lädt `cargo` beim ersten Mal die Git-Abhängigkeit. Zur Laufzeit braucht das Studio kein Netz: es benutzt immer die mitgelieferte Befehlsdatenbank in `crates/g4-kernel/metadata/scrcmd/` (Revision 33, Herkunft in `SOURCE.md` dort), nie eine heruntergeladene und nie `rotom init`. Die Warnung „stripping debug info with `rust-objcopy` failed“ ist harmlos.
- Das Binary braucht `libnitroarc_ffi.dylib` (LGPL, dynamisch gelinkt, wie schon bei uxie in der App) und hat **keinen rpath**. Für den
  Entwicklungsbetrieb die dylib daneben legen und einen rpath setzen (`cargo build --release -p studio` erzeugt sie in `target/release/`):

  ```sh
  cp target/release/libnitroarc_ffi.dylib tools/rotom/bin/
  install_name_tool -add_rpath @loader_path tools/rotom/bin/rotom-lsp
  codesign --force --sign - tools/rotom/bin/rotom-lsp
  echo | tools/rotom/bin/rotom-lsp     # antwortet mit einem Parse-Fehler, kein dyld-Fehler
  ```

  `apps/studio/scripts/prepare-bundle.sh` macht dasselbe (und baut `rotom-lsp`, falls es fehlt); für das Bundle bekommt die Kopie in
  `src-tauri/binaries/` zusätzlich den rpath `@executable_path/../Frameworks` (dylib in `Contents/Frameworks`).

### Wo die App rotom-lsp sucht

1. Umgebungsvariable `G4_ROTOM_LSP` (voller Pfad). Ist sie gesetzt, gilt sie immer, auch wenn der Pfad falsch ist.
2. Eine Datei `rotom-lsp` neben dem Studio-Binary (im Bundle `Contents/MacOS/`). Eine leere Datei (Platzhalter aus `build.rs`) zählt nicht.
3. Nur in Debug-Builds: `tools/rotom/bin/rotom-lsp` im Repository.

```sh
G4_ROTOM_LSP="$PWD/tools/rotom/bin/rotom-lsp" pnpm tauri dev
```

### Lebenszyklus

Ein Prozess je offenem Projekt, Arbeitsverzeichnis = Projektwurzel, gestartet vom Script-Panel über `lsp_start(channel)`. Die
Nachrichten laufen als `Content-Length`-gerahmtes JSON-RPC über stdio; ein Lese-Thread schiebt jede Nachricht in den Tauri-Channel,
`lsp_send` reiht die Nachricht für einen Schreib-Thread ein (blockiert nie; liest der Server nicht, füllt sich die Warteschlange und `lsp_send` meldet einen Fehler). Beendet wird er bei Projektwechsel, `lsp_stop` und App-Ende (die ganze Prozessgruppe). Stirbt er, startet
der nächste `lsp_send` ihn neu, aber höchstens einmal je Minute; ein schnellerer Tod ergibt „rotom-lsp beendet sich wiederholt“, und er
bleibt aus bis zum nächsten `lsp_start`. Der Neustart verlangt vom Client ein neues `initialize`.

## Tests

Befehle aus der Wurzel des Repositorys, mit `export PATH="/opt/homebrew/opt/rustup/bin:$PATH"`:

```sh
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo fmt --check
cd apps/studio && pnpm test && pnpm build
```

`--all-features` ist nötig, sobald `g4-kernel` allein getestet wird: die Integrationstests `store`, `cli`, `project`, `build`
und `headers` sowie der Vanilla-Test in `export` sind mit `#![cfg(feature = "test-support")]` bzw.
`#[cfg(feature = "test-support")]` abgesichert. `cargo test -p g4-kernel` ohne das Feature kompiliert sie nicht und meldet
ohne Fehler `0 tests`. `cargo test -p g4-kernel --features test-support` genügt ebenfalls. Bei `--workspace` schaltet der
Studio-Crate das Feature für `g4-kernel` ein; `--all-features` macht das explizit und ist der Standardbefehl für die Prüfung
vor jedem Commit.

**In einem Git-Worktree** nur auf Workspace-Ebene bauen und testen: `cargo test --workspace --all-features <Namensfilter>`
(mehrere Filter nach `--` gehen auch). Befehle je Paket (`-p g4-kernel`, `-p g4-formats`) und `--test <name>` ändern die
Feature-Menge von rotom; dessen `build.rs` läuft dann neu und lädt dabei `db-latest.zip` aus dem Netz (siehe „rotom und
rotom-lsp“). In den Worktrees von 3b.3a, deren `target/` die rotom-Build-Ausgabe aus dem Haupt-Checkout übernommen hatte,
scheiterte genau dieser Download („headers' total size surpassed max_headers_size“). Der Namensfilter läuft dieselben Tests.

### Gated Tests

Tests mit ROM-Daten laufen nur, wenn die Umgebungsvariablen gesetzt sind; sonst melden sie `skipped: ...` auf stderr und
gelten als bestanden. Mit `-- --nocapture` ist zu sehen, was wirklich lief. Das Projekt wird nur gelesen; schreibende Tests
arbeiten auf einer Kopie in einem Temp-Verzeichnis (`g4_kernel::test_support::vanilla_copy()`). Die Originale werden nie
beschrieben.

| Variable | Bedeutung | Gebraucht von |
|---|---|---|
| `G4_TEST_PROJECT` | ds-rom-Projektordner (`..._DSPRE_contents`) | die Vanilla-Tests in `g4-formats`, `g4-kernel` und `studio` |
| `G4_APICULA` | voller Pfad zu `tools/apicula/bin/apicula` | die Modell-Tests (`export`, Gebäude-GLB in `studio`) |
| `G4_DSROM_CLI` | voller Pfad zur ds-rom-CLI `dsrom` | `vanilla_rebuild_matches_dsrom_cli` in `crates/g4-kernel/tests/build.rs` |
| `G4_ROTOM_LSP` | voller Pfad zu `tools/rotom/bin/rotom-lsp` | nur das Gate-Skript reicht ihn durch; die Unit-Tests von `lsp.rs` brauchen ihn nicht (Fake-Server) |
| `G4_TEST_ROM` | eine `.nds`-Datei (nur gelesen) | `extract_rom_creates_openable_project` |

`G4_TEST_PROJECT` wird nacheinander auf jedes der drei Projekte gesetzt (Platin DE, HeartGold DE, HeartGold US); die
Tests laufen einmal pro Projekt. Die Pfade müssen absolut sein, denn `cargo test` startet die Tests im Ordner des jeweiligen Crates:

```sh
G4_TEST_PROJECT="$PWD/../Roms/Pokemon - Platin-Edition (Germany)_DSPRE_contents" \
G4_APICULA="$PWD/tools/apicula/bin/apicula" \
G4_DSROM_CLI="$PWD/../Roms/_tools/dsrom" \
  cargo test --workspace --all-features -- --nocapture
```

`vanilla_rebuild_matches_dsrom_cli` baut dasselbe Projekt mit der Referenz-CLI und mit `g4_kernel::build::build` und verlangt
byte-identische ROMs. Bei Änderungen an Build- oder Pack-Code ist dieser Lauf Pflicht. Seit 3b.3a packt der Build auch die
Daten-Ordner (Begegnungen, Trainer, Pokémon-Basisdaten, Lernsets, Entwicklungen, Attacken, Gegenstände, Trainer-Texttabellen,
Item- und Pokémon-Symbole); der Test prüft, dass die zehn Daten-Archivarten mit ihrer vollen Member-Zahl gepackt sind und das
ROM trotzdem byte-identisch bleibt. Aus einem
Git-Worktree zeigt `$PWD/../Roms` ins Leere: dort `G4_DSROM_CLI=/<absoluter Pfad zu Roms>/_tools/dsrom` setzen (ebenso
`G4_TEST_PROJECT` und `G4_ROMS_DIR` absolut).

#### Daten-Archive und das uxie-Orakel (3b.3a)

Die Codecs der Daten-Archive in `g4-formats/src/data/` sind eigene (Rohbytes mit Feldkarte); uxie 0.9.7 ist nur noch
Test-Orakel (Dev-Abhängigkeit von `g4-formats`, in `g4-kernel` weiter regulär). Grund: uxie lehnt 172 von 471 Vanilla-Attacken
ab und schreibt sie mit 14 statt 16 Byte, sein Begegnungs-Writer ist unbrauchbar; Personal, Items und Trainer-Eigenschaften
liest es dagegen Feld für Feld richtig. Die gated Tests dazu:

| Test | Datei | Prüft |
|---|---|---|
| `vanilla_fixed_records_round_trip`, `vanilla_variable_records_round_trip`, `vanilla_trainer_parties_round_trip_with_their_properties` | `crates/g4-formats/tests/vanilla_data.rs` | `write(read(x)) == x` für jeden Member der zehn Archivarten; die Member-Zahl muss stimmen (Platin / HGSS: encounters 183/142, trainerProperties und trainerParty 928/738, personalPokeData, learnsets, evolutions je 508, moveData 471, itemData 446/514, trainerTextTable und trainerTextOffset je 1). Ein Ordner ohne Member oder mit anderen Dateien als `NNNN` lässt den Test fehlschlagen |
| `vanilla_personal_matches_uxie`, `vanilla_items_match_uxie`, `vanilla_trainer_properties_match_uxie` | ebenda | das uxie-Orakel: jedes Feld der eigenen Codecs gegen die Getter von uxie (Items auch die einzelnen Flags „Wirkung im Team“) |
| `vanilla_pt_water_slots_have_min_le_max` | ebenda | nur Platin (HGSS meldet „skipped: not a Platinum project“): 1080 belegte Wasser-Slots, alle mit min ≤ max |
| `vanilla_arm9_tables_are_plausible` | `crates/g4-kernel/tests/arm9_tables.rs` | Item-Tabelle, TM/VM-Liste und Paletten der Pokémon-Symbole aus `metadata/arm9_tables.yaml` sind plausibel (CPUD, IPKD, IPKE) |
| `vanilla_data_members_load_unlocked_and_byte_identical` | `crates/g4-kernel/tests/store.rs` | jeder Daten-Member lädt ungesperrt und byte-identisch in den Store |
| `vanilla_name_counts_match_task0`, `every_vanilla_trainer_name_round_trips` | `crates/g4-kernel/tests/names.rs` | Namenszahlen der Textarchive, Trainernamen hin und zurück |
| `vanilla_index_builds_under_a_second`, `vanilla_dirty_stays_fast_after_index_build`, `vanilla_route_201_encounters_reference_known_species` | `crates/g4-kernel/tests/refs.rs` | Referenzindex |
| `vanilla_item_icon_of_potion_decodes_to_a_non_empty_png`, `vanilla_every_item_with_an_icon_decodes`, `vanilla_mon_icon_of_bulbasaur_uses_palette_one`, `vanilla_every_species_and_form_icon_decodes` | `crates/g4-kernel/tests/icons.rs` | Item- und Pokémon-Symbole (NCGR/NCLR) |
| `vanilla_data_commands_on_copy` | `apps/studio/src-tauri/src/commands/data/tests.rs` | die Datenbefehle auf `vanilla_copy()` |

Zum Beispiel nur die Round-Trips und das Orakel, je Projekt:

```sh
G4_TEST_PROJECT="/<absoluter Pfad zu Roms>/Pokemon - Platin-Edition (Germany)_DSPRE_contents" \
  cargo test --workspace --all-features -- vanilla_fixed_records vanilla_variable_records vanilla_trainer_parties \
  vanilla_personal_matches_uxie vanilla_items_match_uxie vanilla_trainer_properties_match_uxie vanilla_pt_water_slots --nocapture
```

#### Trainer und Begegnungen (3b.3b)

Seit 3b.3b sind Trainer ein zusammengesetztes Asset (Eigenschaften und Team in zwei Dateien, Name und Kampftexte in
Textarchiven), Begegnungen haben Regeln im Kernel (`data/encounter_rules.rs`), und `create_trainer` erweitert die
Kampf-Scriptdatei ([docs/formats/trainer_scripts.md](formats/trainer_scripts.md)). Die gated Tests dazu:

| Test | Datei | Prüft |
|---|---|---|
| `vanilla_trainers_decode_and_encode_identically` | `crates/g4-formats/tests/vanilla_data.rs` | jeder Trainer (Platin 928, HGSS 738 Paare) liest sich als ein Dokument und schreibt beide Dateien byte-gleich zurück |
| `vanilla_trainers_load_unlocked_through_the_store` | `crates/g4-kernel/tests/store.rs` | jeder Vanilla-Trainer lädt über den Store als ein Asset, ungesperrt und byte-gleich |
| `vanilla_encounter_files_raise_no_warnings` | `crates/g4-kernel/src/data/encounter_rules.rs` (Unit-Test, gated) | keine Vanilla-Begegnungsdatei bricht eine Regel; jede Warnung im Editor zeigt also auf eine Änderung |
| `vanilla_creates_trainers_behind_the_sight_slot` | `crates/g4-kernel/tests/trainer_create.rs` | auf `vanilla_copy()` mit übernommenen Scripts: Platin 928, Platzhalter 929, 930; HGSS 738, 739, Platzhalter 740, 741; die Kampf-Scriptdatei kompiliert sauber, ein neu geöffneter Store zeigt den Platzhalter gesperrt (übernimmt dafür die Scripts der Kopie) |

Je Projekt einzeln:

```sh
G4_TEST_PROJECT="/<absoluter Pfad zu Roms>/Pokemon - Platin-Edition (Germany)_DSPRE_contents" \
  cargo test --workspace --all-features -- vanilla_trainers_decode vanilla_trainers_load vanilla_encounter_files_raise \
  vanilla_creates_trainers_behind --nocapture
```

#### proptest

`proptest` 1 ist Dev-Abhängigkeit von `g4-formats` und `g4-kernel`; die Tests laufen ohne Projekt im normalen
`cargo test --workspace --all-features`:

- `g4-formats/src/data/`: Round-Trip über zufällige Bytes je Codec (Personal, Entwicklung, Attacke, Gegenstand, Begegnung,
  Trainer-Texttabelle), Lernsets aus beliebigen terminierten Bytes hin und zurück (eine geänderte Länge ergibt immer ein
  terminiertes Vielfaches von 4), `read(write(d)) == d` für die Party in jedem Format, Setter-Isolation im Record-Kern
  (`setter_changes_only_its_own_bytes`, `set_bits_changes_only_the_masked_bits`: ein Setter ändert nur seine Bytes oder Bits).
- `g4-kernel/tests/transaction.rs`: `undo_redo_are_inverse_over_mixed_kinds` (64 zufällige Folgen aus Edits, Undo und Redo über
  gemischte Arten).
- `g4-kernel/tests/refs.rs`: `incremental_equals_full_rebuild` (48 Folgen: der inkrementelle Referenzindex gleicht dem voll
  neu gebauten).
- `g4-kernel/tests/names.rs`: `trainer_name_wrap_unwrap_round_trip`.

Die beiden Kernel-Tests mit eigenem `TestRunner` schreiben keine `proptest-regressions`-Dateien; ein Fehlschlag steht nur in
der Ausgabe (mit dem geschrumpften Fall).

#### Daten im gebauten ROM prüfen: `g4k check-data`

`g4k check-data <rom.nds> --expect KIND:MEMBER:FIELD=VALUE …` liest die Daten-NARCs aus einem gebauten ROM (Pfade aus
`archives.yaml` für den Spielcode im ROM-Kopf) und vergleicht einzelne Felder:

- KIND `species` und `item` liest uxie (unabhängig von den eigenen Codecs), `move`, `learnset` und `encounter` die eigenen
  Codecs (uxie lehnt Attacken ab, sein Begegnungs-Leser ist unbrauchbar, siehe oben; für `encounter` weicht das von der
  Spezifikation ab, die „per uxie“ sagt). Die Feldnamen sind die der Codecs (`attack`, `power`, `price` …); ein Lernset hat
  nur `len` (Zahl der Einträge); eine Begegnungsdatei hat `grass_rate`, `grassN` (Art in Gras-Slot N, 0 bis 11; HGSS: die
  Tag-Spalte), `grassN_level` und in HGSS `grassN_morning`, `grassN_night`.
- MEMBER ist der Member im Archiv, bei `item` also der Datenmember, nicht die Item-ID (in Platin ist Trank beides 17).
- Exit 0, wenn alles stimmt (stdout „species 397 attack: 85“ je Erwartung und „N expectation(s) met“); Exit 1 bei einer
  Abweichung oder einem unlesbaren Member (stderr „species 397 attack: expected 75, found 85“); Exit 2 bei einem falsch
  geschriebenen `--expect`, bevor das ROM gelesen wird.

```sh
cargo build --workspace --all-features --bin g4k
target/debug/g4k check-data <Kopie>/out.nds --expect species:397:attack=85 --expect learnset:397:len=13
target/debug/g4k check-data <Kopie>/out.nds --expect encounter:140:grass0=150 --expect encounter:183:grass_rate=100
```

In einem Worktree statt `--bin g4k` den ganzen Workspace bauen (`cargo build --workspace --all-features`), sonst lädt
rotoms Build-Skript neu (siehe oben).

Szenarien rufen es im Abschnitt `check:` auf (Beispiel `scripts/agent/scenarios/pt-daten.yaml`, Beschreibung in
[docs/agent-gate.md](agent-gate.md), „Szenario“).

#### Projektprüfungen: `g4k check`

`g4k check <projekt> [--json] [--expect CODE=N …]` führt alle Prüfungen (Spec 3b.4 §4) auf dem gespeicherten Projekt aus
und liest dabei, wie das Studio, `<projekt>/g4-checks.json`: ausgeblendete Befunde zählen als „ignored“, abgeschaltete
Prüfungen laufen nicht (Fehler werden nie ausgelassen). Ohne `--json` eine Zeile je Befund und eine Summe, mit `--json`
der ganze Bericht (`findings`, `ignored`, `skipped` als `{code, why}`, Subjekte als `{kind, index}`, Fingerprints als
16 Hex-Ziffern). `--expect warp.dest-warp=1` vergleicht die Zahl der nicht ausgeblendeten Befunde je Code: Exit 1 mit
„warp.dest-warp: expected 1, found 0“ auf stderr; ein unbekannter Code endet vor dem Lauf mit Exit 2. Befunde allein
ändern den Exit-Code nicht.

Messlauf, nicht Teil des normalen Durchlaufs (`#[ignore]`): `G4_APICULA=... cargo test -p studio --release -- --ignored
--nocapture bench_matrix_models` lädt eine große Matrix (Standard: Header 3, Platin Matrix 0) und gibt die Zeiten aus;
`G4_BENCH_HEADER` wählt einen anderen Header.

## Design-System und Tokens

Das Design-System „Doppeldeck“ liegt im Repository unter `design-system/` (Beschreibung: [design-system/README.md](../design-system/README.md)). Die Werte stehen als W3C-DTCG-Tokens in `design-system/tokens/`; die App liest sie aus drei **generierten und eingecheckten** Dateien in `apps/studio/src/ui/tokens/`: `tokens.css` (CSS-Variablen `--g4-*`), `dockview-theme.css` (Thema `dockview-theme-g4`) und `tokens.ts` (Zahlen und Hex-Werte für three.js, CodeMirror und Canvas). `pnpm build` braucht deshalb kein Python.

Ein Token ändern:

```sh
python3 design-system/scripts/check_tokens.py     # Raster (RGB555) und Kontraste in beiden Themen
cd apps/studio && pnpm tokens                     # schreibt die drei Dateien neu (python3 export_tokens.py <root> <Ausgabeordner>)
cd ../.. && ./scripts/check-tokens.sh             # check_tokens.py plus Abgleich der eingecheckten Dateien mit einem frischen Export
```

Danach die geänderten Token-Dateien in OpenPencil einspielen („Design-Tokens importieren“, siehe das README des Design-Systems). Die drei Dateien nie von Hand ändern: der Gate-Schritt `tokens` meldet jede Abweichung vom Export.

Stil-Regeln für `apps/studio/src` und der Gate-Schritt `styles` (`scripts/check-styles.sh`, gleiches `grep`-Prinzip, Exit 1 mit `Datei:Zeile` je Fund):

- Stile stehen in CSS Modules (`Name.module.css`) und benutzen nur `--g4-*`-Variablen (Thema, Raum, Dichte, Schrift), nie Primitive wie `--g4-platin-500`.
- Kein `style={` in `.tsx` (weder ein Objekt im Attribut noch ein Stil aus einer Variablen), keine Hex-Farbe (`#rgb` bis `#rrggbbaa`) und keine Primitiv-Variable (`--g4-platin-500`, `--g4-kobalt-600` …) in `.tsx`, `.ts` und `.css`. Ausgenommen sind `ui/tokens/`, `world/colors.ts` (setzt die Zahlen aus `tokens.ts` für three.js um), Testdateien und Zeilen mit `/* dyn */` (Werte, die erst zur Laufzeit entstehen, etwa die Position eines Markers; die Markierung steht in derselben Zeile). Eigene CSS-Variablen, die eine Komponente zur Laufzeit setzt, sind einmal in `apps/studio/src/css-custom-properties.d.ts` typisiert.
- `scripts/check-styles.allow` nennt Dateien und Ordner (je eine Zeile, Glob erlaubt), die noch im alten Stil sein dürfen. Seit 3c.2 ist die Liste leer und bleibt es; neue Dateien kommen nie darauf.

Die `.fig` pflegen: nach dem Speichern in OpenPencil `node design-system/scripts/fix_fig_text.mjs design-system/Editor_DesignSystem.fig --rgb555` (aus der Repo-Wurzel; repariert Text-Glyphen und rastet Farben auf RGB555), dann die Datei einchecken.

## Galerie

Die Galerie zeigt die Bausteine aus `apps/studio/src/ui/primitives/` mit ihren Zuständen, eine Seite je Gruppe (Tasten, Eingaben, Inhalt und Status, Schwebendes, Auswahl, Bildschirm mit dem DS-Bildschirm, Rahmen mit Scharnier im Assistenten-Modus, Startkarten, Palette und Reitern); oben schalten „Thema“ (Hell, Dunkel, System) und „Dichte“ (Komfort, Kompakt) um. Sie läuft nur im Browser-Modus, nie im Tauri-Fenster, und braucht kein Projekt: `http://127.0.0.1:1430/?gallery`, nachdem `scripts/agent/studio-web.sh` gestartet ist (oder mit `pnpm dev` in `apps/studio` auf dem Vite-Port). Es gibt keine automatischen Tests für die Galerie; der Agent macht für das Gate Screenshots in Hell, Dunkel und Kompakt und achtet auf unlesbare Stellen. Ein neuer Baustein bekommt eine Zeile auf einer Seite unter `ui/gallery/pages/` (`pages/index.ts` listet die Seiten).

## Gate-Skript

Es gibt kein Remote und keine CI; `scripts/gate.sh` (Repo-Wurzel, bash) ersetzt sie und ist der Lauf vor einem Merge. Voraussetzung neben Rust und pnpm ist `uv` (der Schritt `g4emu unit` startet `uv run`; der erste Lauf lädt die Python-Pakete von PyPI). Es fährt der Reihe nach:

1. `cargo fmt --check`
2. `cargo clippy --workspace --all-targets --all-features -- -D warnings`
3. `cargo test --workspace --all-features` (ohne ROM-Daten: `G4_TEST_PROJECT`, `G4_TEST_ROM`, `G4_DSROM_CLI`, `G4_APICULA` und `G4_ROTOM_LSP` aus der aufrufenden Shell werden für diesen Schritt entfernt, die gated Tests überspringen sich)
4. `pnpm test` in `apps/studio`
5. `tokens`: `scripts/check-tokens.sh` (Raster und Kontraste mit `check_tokens.py`, danach der Abgleich der eingecheckten Token-Dateien mit einem frischen Export; siehe „Design-System und Tokens“)
6. `styles`: `scripts/check-styles.sh` (keine Inline-Styles und keine Hex-Farben außerhalb der Ausnahmen; siehe „Design-System und Tokens“)
7. `pnpm build` in `apps/studio`, danach die Prüfung, dass `apps/studio/dist` den Text `__bridge` nicht enthält (die Dev-Brücke darf nicht ins Produktions-Bundle)
8. `g4emu unit`: `uv run pytest -m "not integration" -q` in `scripts/g4emu`
9. `g4emu integration`: `uv run pytest -m integration -q` in `scripts/g4emu` (Platin DE mit Spielstand bis ins Zimmer, Treppe und Header-ID im echten Emulator). Läuft nur, wenn der Kern (`tools/g4emu/core/melondsds_libretro.dylib` oder `G4EMU_CORE`, relative Pfade sind erlaubt), `$G4_ROMS_DIR/_testkopien/_saves/CPUD-zimmer.sav` (das Gate sucht Spielstände immer dort; `G4EMU_SAVES` wertet es nicht aus) und `$G4_ROMS_DIR/Pokemon - Platin-Edition (Germany).nds` existieren; sonst steht die Zeile "skipped (kein Kern)", "skipped (kein Spielstand)" bzw. "skipped (keine ROM)" in der Tabelle, ohne Abbruch
10. je Testprojekt unter `$G4_ROMS_DIR` (Muster `*_DSPRE_contents`) `cargo test --workspace --all-features` mit `G4_TEST_PROJECT`, `G4_APICULA`, `G4_DSROM_CLI` und `G4_ROTOM_LSP` (alle absolut), also die gated Tests aus dem Abschnitt "Gated Tests" gegen Platin DE, HeartGold DE und HeartGold US

```sh
scripts/gate.sh
```

Das Skript findet die Rust-Toolchain selbst (`~/.rustup/toolchains/stable-aarch64-apple-darwin/bin`, falls vorhanden; sonst gilt der `PATH` der Shell), braucht also keinen `export PATH`. Variablen, alle optional:

| Variable | Standard | Bedeutung |
|---|---|---|
| `G4_ROMS_DIR` | `../Roms` (neben dem Repository) | Ordner mit den `*_DSPRE_contents`-Projekten; aus einem Git-Worktree absolut setzen, der Standard trifft dort nichts |
| `G4_APICULA` | `tools/apicula/bin/apicula` | apicula-Binärdatei für die Modell-Tests |
| `G4_DSROM_CLI` | `$G4_ROMS_DIR/_tools/dsrom` | ds-rom-Referenz-CLI für den Rebuild-Vergleich |
| `G4EMU_CORE` | `tools/g4emu/core/melondsds_libretro.dylib` | Emulator-Kern für den Schritt `g4emu integration` |
| `G4_ROTOM_LSP` | `tools/rotom/bin/rotom-lsp` | rotom-lsp-Binärdatei (Preflight; wird an die gated Tests durchgereicht) |

Das Skript hält beim ersten Fehler und druckt am Ende eine Tabelle Schritt / Projekt / Ergebnis / Sekunden, auch bei Abbruch; der Exit-Code ist ungleich 0, sobald ein Schritt fehlschlägt. Ein Ordner ohne `header.yaml` zählt nicht als Projekt und erscheint als Zeile "skipped (no header.yaml)". Fehlen `G4_ROMS_DIR`, `G4_APICULA`, `G4_DSROM_CLI` oder `G4_ROTOM_LSP`, oder findet es kein Projekt, bricht es mit einer Meldung ab (sonst würden die gated Tests nur "skipped" melden und trotzdem als bestanden gelten).

Dauer: bei warmem Build-Cache etwa 30 Minuten (gemessen 1704 s in 3c, 1781 s in 3b.3a, 1798 s in 3b.3b; vor 3b.2 waren es 1217 s, dazu rund eine Minute für die g4emu-Schritte; seit 3b.2 kommen die gated Script- und Text-Tests dazu, seit 3b.3a die Daten-Tests aus „Daten-Archive und das uxie-Orakel“; seit 3b.3b die Trainer- und Begegnungs-Tests aus „Trainer und Begegnungen (3b.3b)“; gemessene Läufe stehen in [docs/phase-3c-gate.md](phase-3c-gate.md) (1704 s) und [docs/phase-3b3-gate.md](phase-3b3-gate.md) (Teil A 1781 s, Teil B 1798 s); die drei Projekte brauchen 380 bis 670 s je Projekt, die Schritte davor zusammen unter einer Minute); ein kalter Build kommt obendrauf. Die Projekte unter `$G4_ROMS_DIR` werden nur gelesen: schreibende Tests arbeiten auf einer Kopie im Temp-Verzeichnis (`vanilla_copy()`), und `_testkopien` fasst das Skript nicht an.

## Agent-Testumgebung

Ein Claude-Agent bedient die App und das Spiel selbst; die Anleitung dafür steht in [docs/agent-gate.md](agent-gate.md). Die Bausteine:

- **Brücke `studio-bridge`** (Cargo-Feature `dev-bridge`, nur im Dev-Build, nie im App-Bundle): ein kleiner HTTP-Server auf `127.0.0.1:1431`, der alle Tauri-Befehle der App aufruft. Er nimmt nur Projekte und Build-Ziele unter `$G4_ROMS_DIR/_testkopien` an (nie unter `*_DSPRE_contents`) und verlangt ein Token (`X-G4-Bridge-Token`, steht in `target/agent/bridge.token`). `scripts/agent/studio-web.sh` startet Brücke und Vite (`127.0.0.1:1430`); `.claude/launch.json` kennt sie als `studio-web`. Solange die Brücke läuft, erreicht jeder lokale Nutzer sie über den Vite-Proxy auf `127.0.0.1:1430` (das Token liegt für den Proxy bereit); angenommen ist eine Entwicklungsmaschine mit einem Nutzer. Die Seite läuft dann im Browser-Fenster der Claude-Desktop-App unter `http://127.0.0.1:1430/?project=<Kopie>` und zeigt "Browser-Modus (Brücke)" in der Statusleiste.
- **`scripts/agent/call.sh <befehl> ['<json>']`** ruft einen Befehl der laufenden Brücke auf; `studio-bridge batch <datei.json> --allow-root <dir>` führt eine Liste von Befehlen ohne Server aus.
- **`g4emu`** (`scripts/g4emu`, Python mit `uv`): ein Emulator ohne Fenster (libretro.py mit dem melonDS-DS-Kern 1.4.0). Einrichten mit `bash scripts/g4emu/setup.sh` (lädt den Kern aus dem festen GitHub-Release, SHA-256 geprüft, nach `tools/g4emu/core/`). Aufruf: `uv run g4emu run --rom … --save … --out … <Schritte>` (`continue`, `walk-to`, `expect-map`, `where`, `shot` …), `uv run g4emu scenario <yaml> --project <Kopie> --set NAME=WERT …` (Projekt ändern, bauen, spielen; Beispiele in `scripts/agent/scenarios/`; eine Änderung kann die Antwort der Brücke an einen Namen binden, den spätere Schritte als `${name}` nutzen), `uv run g4emu find-u16` zum Suchen von Adressen. Seit 3b.3b kennt `g4emu` Kampfschritte (`expect-battle`, `walk-until-battle`, `end-battle`, `win-battle`, `mash`): ein Kampf wird an den BattleMon-Datensätzen im RAM erkannt, die Werte je Spielcode stehen in `scripts/g4emu/g4emu/games.yaml` (`battle`, `battle_pointer`, `home_header`; IPKE ohne gemessenen Zeiger); Beschreibung in [docs/agent-gate.md](agent-gate.md), „Kämpfe“. Die Tests: `uv run pytest -m "not integration"` (Einheit) und `-m integration` (echter Kern).
- **Spielstände** liegen außerhalb des Repos in `$G4_ROMS_DIR/_testkopien/_saves/<SPIELCODE>-zimmer.sav` (CPUD Platin DE, IPKD HeartGold DE, IPKE HeartGold US) mit einer `.md`-Datei, die den Weg dorthin beschreibt. `g4emu` kopiert sie vor jedem Lauf und schreibt nie zurück.
- **Ports:** Vite 1430, Brücke 1431. Die eigene App (`pnpm tauri dev`) bleibt auf 1420/1421.
- **Umgebungsvariablen:** `G4_ROMS_DIR` (Ordner `Roms`, Standard `../Roms`), `G4_APICULA`, `G4EMU_CORE`, `G4EMU_SAVES` (anderer Spielstand-Ordner), `G4_BRIDGE_BIN` und `G4K_BIN` (fertiges `studio-bridge` bzw. `g4k` für Szenarien; aus einem Worktree mit `cargo build --workspace --all-features` bauen, `cargo test --no-run` erneuert `studio-bridge` nicht immer). In einem Git-Worktree fehlt `tools/` (nicht versioniert): `tools/apicula` verlinken oder `G4_APICULA` setzen. Dort stimmt auch der Standard von `G4_ROMS_DIR` nicht (`../Roms` liegt neben dem Haupt-Checkout): `scripts/agent/studio-web.sh` und `scripts/gate.sh` brauchen aus einem Worktree `G4_ROMS_DIR=<absoluter Pfad zu Roms>`.

## App-Bundle

Ein lauffähiges `.app` (mit `libnitroarc_ffi.dylib` und den Sidecars apicula und rotom-lsp) entsteht mit:

```sh
cd apps/studio
pnpm bundle                      # prepare-bundle.sh, danach tauri build
bash scripts/check-bundle.sh     # Ausgabe: bundle ok
```

Voraussetzungen, Ablauf, Prüfbefehle und bekannte Grenzen (nicht signiert, Gatekeeper) stehen in [docs/release.md](release.md). Der Rauchtest der gepackten App (Script öffnen, Completion, Fehler beim Speichern) steht im Gate-Dokument [docs/phase-3b2-gate.md](phase-3b2-gate.md), Teil (H).
`pnpm tauri dev` und normale `cargo`-Builds brauchen das nicht.
