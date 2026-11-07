# Entwicklungs-Setup

## apicula (NSBMD zu GLB)

Die Matrix-Ansicht zeigt die Map-Modelle und die Gebäudemodelle (NSBMD) mit ihren Texturen (NSBTX) als GLB an. Die Konvertierung übernimmt
[apicula](https://github.com/scurest/apicula). Für macOS gibt es kein fertiges Release, es wird aus dem Quellcode gebaut:

```sh
export PATH="/opt/homebrew/opt/rustup/bin:$PATH"
source scripts/tool-versions.env
cargo install --git https://github.com/scurest/apicula --rev "$APICULA_REV" --locked --root tools/apicula
```

Das Ergebnis liegt in `tools/apicula/bin/apicula` (`tools/` ist per `.gitignore` ausgeschlossen).
Die Fassung steht in `scripts/tool-versions.env` (`APICULA_REV`, Commit `3d4e91e14045392a49c89e86dab8cb936225588c`, `apicula 0.1.1-dev`);
`pnpm bundle` baut apicula mit derselben Fassung, wenn `tools/apicula/bin/apicula` fehlt.

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

- Die Dateien auf der obersten Ebene von `.g4/` sind versionierte Projekt-Metadaten (`base.json`, `preview.png`) und bleiben stehen. Nur die
  Unterordner `cache/`, `tmp/`, `build/`, `backup/` und `models/` sind lokal und dürfen jederzeit gelöscht werden, `.g4/models/`
  also auch.
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
cargo deny --locked check licenses advisories
bash scripts/check-licenses-npm.sh
cd apps/studio && pnpm test && pnpm build
```

### cargo-deny (Lizenzen und Advisories)

`cargo deny` prüft den Abhängigkeitsbaum mit allen Features gegen `deny.toml` in der Repo-Wurzel: Lizenzen nur aus der
Freigabeliste, keine offene RustSec-Meldung (Schwachstelle, unmaintained, yanked). Installieren, einmal je Rechner (landet
in `~/.cargo/bin`):

```sh
export PATH="/opt/homebrew/opt/rustup/bin:$PATH"
cargo install --locked cargo-deny@0.20.2
```

Der erste Lauf lädt die RustSec-Datenbank nach `~/.cargo/advisory-dbs`, jeder weitere holt sie neu; eine neue Meldung kann das
Gate also ohne Änderung am Repo rot machen. Dann zuerst `cargo update -p <crate>@<Version>` (semver-verträglich) oder ein
verträgliches Update des direkten Elternteils versuchen; erst wenn keins den Fix bringt, eine Ausnahme je RUSTSEC-ID unter
`[advisories] ignore` mit Grund (Crate und Version, Kette, warum kein Update geht, warum es hinnehmbar ist). Pauschale
Schalter wie `unmaintained = "none"` gibt es nicht.

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

### npm-Lizenzen

`scripts/check-licenses-npm.sh` prüft die Produktionspakete von `apps/studio` (`pnpm licenses list --prod --json`) gegen die
Freigabeliste `MIT`, `Apache-2.0`, `ISC`, `BSD-2-Clause`, `BSD-3-Clause`, `0BSD`, `CC0-1.0`, `Unlicense`, `OFL-1.1`, `MPL-2.0` und
`Zlib`. Ein SPDX-Ausdruck `A OR B` gilt, wenn eine Seite erlaubt ist, `A AND B` nur, wenn beide erlaubt sind; Klammern gruppieren.
Jede andere Lizenz steht mit Paket und Version auf stderr, das Skript endet mit 1; sonst druckt es `npm licenses: <n> packages ok`.
Eine neue Abhängigkeit kommt mit genauer Version (ohne `^`) in `package.json`; eine Lizenz außerhalb der Liste (etwa EPL-2.0 von
`elkjs`) braucht eine Entscheidung, keinen Eintrag im Skript nebenbei.

### Gated Tests

Tests mit ROM-Daten laufen nur, wenn die Umgebungsvariablen gesetzt sind; sonst melden sie `skipped: ...` auf stderr und
gelten als bestanden. Mit `-- --nocapture` ist zu sehen, was wirklich lief. Das Projekt wird nur gelesen; schreibende Tests
arbeiten auf einer Kopie in einem Temp-Verzeichnis (`g4_kernel::test_support::vanilla_copy()`). Die Originale werden nie
beschrieben.

| Variable | Bedeutung | Gebraucht von |
|---|---|---|
| `G4_TEST_PROJECT` | ds-rom-Projektordner (`..._DSPRE_contents`) | die Vanilla-Tests in `g4-formats`, `g4-kernel` und `studio`; `crates/g4-kernel/tests/base.rs` (4.1a; dazu `publish_after_a_map_edit_writes_a_small_patch_that_applies`, 4.5), der gated Teil von `cli_publish_prints_the_files_and_the_count` in `crates/g4-kernel/tests/cli.rs` (4.5) und `crates/g4-kernel/tests/original.rs` (4.1b), `crates/g4-kernel/tests/battle_sprites.rs` und `crates/g4-kernel/tests/egg_moves.rs` (3d.3), `crates/g4-kernel/tests/textproject_source.rs` (4.3b; seit 3d.3 dazu `text_project_egg_move_edit_builds_and_check_data_finds_it`), `crates/g4-kernel/tests/git.rs` und `crates/g4-kernel/tests/diff.rs` (4.4; in `git.rs` dazu `publish_then_commit_all_leaves_status_empty_and_no_remote`, 4.5), `crates/g4-kernel/tests/recipes.rs` (3d.2: `vanilla_room_recipe_matches_the_wizard_scenario`, nur Platin DE, sonst `skipped`; „Welt leeren“ mit Undo der Scripts prüft `vanilla_empty_region_undo_restores_the_world` in `tests/templates.rs`), `crates/g4-kernel/tests/dialog.rs` (3d.5: `vanilla_dialogs_of_a_town_resolve`, die Dialoge von Zweiblattdorf bzw. Neuborkia und jeder Figur des Spiels), `crates/g4-kernel/tests/refs_messages.rs` (3d.5: `vanilla_message_refs_count_the_script_messages`, der Index der Nachrichten aus den Scripts) und `crates/g4-kernel/tests/recipes_erzaehlen.rs` (3d.5: `vanilla_new_figure_message_equals_the_input`, `vanilla_doors_round_trip_in_the_town`, `vanilla_trainer_here_in_the_town`; alle drei arbeiten auf `vanilla_copy()`, Platin DE und HeartGold DE, sonst `skipped`) sowie `reset_over_the_command_path_on_the_vanilla_copy` in `apps/studio/src-tauri/src/commands/original.rs`, `battle_sprites_over_the_command_path_on_the_vanilla_copy` in `apps/studio/src-tauri/src/commands/data/tests.rs` (3d.3), `mcp_with_ids_acts_like_numbers_on_the_vanilla_copy` in `apps/studio/src-tauri/src/mcp/tools.rs` (4.3b), `git_and_diff_over_the_command_path_on_the_vanilla_copy` in `apps/studio/src-tauri/src/commands/git.rs` (4.4) und `publish_and_commit_over_the_command_path_on_the_vanilla_copy` in `apps/studio/src-tauri/src/commands/publish.rs` (4.5, braucht dazu `git`) sowie `crates/g4-kernel/tests/flags.rs` (3d.1; auch `<Ordner>.nds` neben einer Testkopie `<Ordner>, `crates/g4-kernel/tests/threads.rs` und `crates/g4-kernel/tests/stories.rs` (3d.6, ebenso; `stories.rs` braucht dazu `git`) brauchen dazu die `.nds` direkt neben dem Ordner (Basis-ROM über `suggest_base`, nur gelesen); `flags_over_the_command_path_on_the_vanilla_copy` in `apps/studio/src-tauri/src/commands/flags.rs` (3d.1) braucht nur `G4_TEST_PROJECT` (es arbeitet auf `vanilla_copy()` ohne Basis-ROM); Löschen (3d.4): `vanilla_world_edges_match_the_measured_counts` in `crates/g4-kernel/tests/refs.rs` (Zählungen der Welt-Kanten; der Teil Script → Header richtet auf einer Kopie unter `target/tmp/refs-<Ordner>` das Textprojekt ein und braucht dazu die `.nds` neben dem Ordner), `textproject_round_trip_with_tombstone` und `textproject_export_idempotent_with_tombstone` in `crates/g4-kernel/tests/textproject.rs`, `a_branch_without_the_freed_file_builds_after_git_switch` in `crates/g4-kernel/tests/git.rs` (braucht `git` und die `.nds` neben dem Ordner) und `delete_over_the_command_path_on_the_vanilla_copy` in `apps/studio/src-tauri/src/commands/delete.rs` (braucht die `.nds`); `crates/g4-kernel/tests/delete.rs` selbst läuft ohne ROM-Daten (synthetisches Textprojekt, ohne `G4_TEST_PROJECT`); BDHC und Gelände (6.2): `crates/g4-formats/tests/vanilla_bdhc.rs`, `crates/g4-kernel/tests/terrain.rs` (liest `unpacked/maps`, der schreibende Test `set_map_terrain_on_a_vanilla_copy_survives_save_and_reopen` arbeitet auf `vanilla_copy()`) und `terrain_over_the_command_path_on_the_vanilla_copy` in `apps/studio/src-tauri/src/commands/terrain.rs` (auf `vanilla_copy()`) brauchen nur `G4_TEST_PROJECT`, keine Basis-ROM; Weltformat (6.3a): `crates/g4-kernel/tests/world.rs` (`vanilla_tilesets_have_one_ground_tile_per_texture`, `vanilla_vote_shares_match_task0`; leiten je Kartentextur-Paket das Tileset ab, nur gelesen, brauchen nur `G4_TEST_PROJECT`), dazu mit einer Welt-Karte `textproject_round_trip_with_world` und `textproject_export_idempotent_with_world` in `crates/g4-kernel/tests/textproject.rs` sowie `vanilla_migration_v2_to_v3_changes_only_the_manifest` in `crates/g4-kernel/tests/textproject_migrate.rs` (auf `vanilla_copy()`, brauchen die `.nds` neben `<Name>_DSPRE_contents`); Backen und Build (6.3b): `crates/g4-kernel/tests/world_bake.rs` (`vanilla_maps_adopted_form1_bake_byte_equal`, `vanilla_adopted_project_builds_byte_equal`, `painted_map_on_a_vanilla_pack_bakes_within_limits`, `world_step_is_incremental_per_map_on_the_copy`, `member_changed_both_ways_on_the_copy`; jeder richtet auf `vanilla_copy()` das Textprojekt ein und braucht die `.nds` neben `<Name>_DSPRE_contents`, der Vergleich mit apicula dazu `G4_APICULA`, ohne ihn meldet der Test `apicula=skipped`) |
| `G4_APICULA` | voller Pfad zu `tools/apicula/bin/apicula` | die Modell-Tests (`export`, Gebäude-GLB in `studio`) |
| `G4_DSROM_CLI` | voller Pfad zur ds-rom-CLI `dsrom` | `vanilla_rebuild_matches_dsrom_cli` in `crates/g4-kernel/tests/build.rs` |
| `G4_ROTOM_LSP` | voller Pfad zu `tools/rotom/bin/rotom-lsp` | nur das Gate-Skript reicht ihn durch; die Unit-Tests von `lsp.rs` brauchen ihn nicht (Fake-Server) |
| `G4_TEST_ROM` | eine `.nds`-Datei (nur gelesen) | `extract_rom_creates_openable_project`, `project_start_inspects_the_test_rom`, `project_start_create_then_build_matches_the_rom`, `project_start_incremental_build_matches_the_rom`, `project_start_empty_region_from_the_rom`, `project_start_text_project_builds_byte_equal` (4.3b) |
| `G4_BASE_ROM_DIR` | Ordner mit Basis-ROMs (`*.nds`, nicht rekursiv, nur gelesen) | kein gated Test (die Tests in `tests/base.rs` und `tests/original.rs` nehmen die Nachbar-`.nds` des Projekts); `g4k patch create`, `g4k base check`, `studio mcp --project` und die Brücke suchen dort nach der SHA-1 der Marke; im Gate `Roms/_testkopien`, nie die Originale |
| `git` (CLI) | `find_git`: `G4_GIT`, sonst `PATH` und die üblichen Orte | nur die Tests, die ein Repository schreiben (`git::cli`, `create_with_git_reports_the_phase_and_commits`, `cli_git_init_and_status`, `tests/git.rs`, `tests/diff.rs`, `git_and_diff_over_the_command_path_on_the_vanilla_copy`, `commit_all_with_git_commits_and_reads_the_status`, `publish_and_commit_over_the_command_path_on_the_vanilla_copy`); ohne `git` melden sie `skipped: git not found` und bleiben grün. Lesende Tests brauchen kein `git` (`gix`, `test_support::GitFixture`) |
| `G4_GIT` | voller (absoluter) Pfad zu einem `git` | optional: nur dieser Pfad gilt, kein Weitersuchen; ein relativer Pfad (auch `git` allein) zählt als nicht gefunden, nie eine Suche im `PATH` (`cli_git_status_honours_g4_git` prüft beides über einen Kindprozess) |
| `xdelta3` im `PATH` | xdelta3-CLI, optional | Gegenprobe in `patch_after_map_and_after_map_script_text_is_small_and_applies`; fehlt sie, meldet der Test `xdelta3 cross-check skipped: …` und bleibt grün |

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
| `vanilla_flag_names_count_the_metadata`, `vanilla_usage_matches_a_recount_of_the_sources`, `vanilla_flag_index_is_fast`, `vanilla_pool_counts_the_free_numbers`, `vanilla_own_name_compiles_byte_equal` | `crates/g4-kernel/tests/flags.rs` | Flags und Variablen (3d.1): pret-Namen (Platin 4 097 Flags / 307 Variablen, HGSS 2 244 / 386), Nutzungsindex gegen eine Nachzählung der Quellen ohne Index (Objekte, versteckte Items, Auslöser, Trainer, Level-Scripts, `SetFlag`/`ClearFlag`/`CheckFlag`), Zeiten (`G4_TIMING_STRICT=1`: warm ≤ 1 000 ms, 200 Abfragen ≤ 50 ms, `usage` ≤ 50 ms), freier Pool (80–170 Flags, 10–100 Variablen; Platin DE lag bei 101, darum die Untergrenze 80), ein eigener Name kompiliert byte-gleich und folgt `rename_id`; druckt `names …`, `usage …`, `index …`, `pool …`, `own …`. Übernimmt die Scripts einmal je Lauf auf einer Kopie unter `target/tmp/flags-<Ordner>` (beim nächsten Lauf ersetzt); der schreibende Test kopiert sie noch einmal ins Temp-Verzeichnis. Braucht die Basis-ROM neben dem Ordner (`suggest_base`, oder `<Ordner>.nds` neben einer Testkopie `<Ordner>`) |
| `vanilla_threads_count_at_least_the_measured`, `vanilla_thread_graph_of_the_first_thread_has_a_node_per_value` | `crates/g4-kernel/tests/threads.rs` | Fäden (3d.6): mindestens 59 (Platin) bzw. 62 (HGSS) Variablen mit drei Werten an zwei Orten, jeder Faden mit ≥ 3 Werten und ≥ 2 Orten; der Graph des ersten Fadens hat einen Wert-Knoten je Wert und nur Kanten zwischen vorhandenen Knoten; Zeiten (`G4_TIMING_STRICT=1`: `threads()` warm ≤ 100 ms, `thread_graph` ≤ 20 ms); druckt `threads …` (Zahl, Summe der Werte, höchste Zahl der Werte und Orte, kalt mit Indexaufbau) und `graph …`. Übernimmt die Scripts einmal je Lauf auf einer Kopie unter `target/tmp/threads-<Ordner>` (beim nächsten Lauf ersetzt). Braucht die Basis-ROM neben dem Ordner wie `tests/flags.rs` |
| `text_project_with_a_story_builds_from_base_byte_equal` | `crates/g4-kernel/tests/stories.rs` | Geschichten (3d.6): eine Vanilla-Kopie als Textprojekt in Git mit einer Geschichte baut die Basis-ROM byte-gleich, Register, Geschichte und Beifile sind Quellen und im Git-Index; druckt `story-git …`. Arbeitet auf `vanilla_copy()`, braucht die Basis-ROM neben dem Ordner und `git` |
| `vanilla_item_icon_of_potion_decodes_to_a_non_empty_png`, `vanilla_every_item_with_an_icon_decodes`, `vanilla_mon_icon_of_bulbasaur_uses_palette_one`, `vanilla_every_species_and_form_icon_decodes` | `crates/g4-kernel/tests/icons.rs` | Item- und Pokémon-Symbole (NCGR/NCLR) |
| `flags_over_the_command_path_on_the_vanilla_copy` | `apps/studio/src-tauri/src/commands/flags.rs` | Flags und Variablen (3d.1) über den Befehlsweg auf `vanilla_copy()` (Scripts nicht übernommen): `list_flags` mit mindestens 4 097 (Platin) bzw. 2 244 (HGSS) Einträgen, `flag_usage` eines Sichtbarkeits-Flags nennt Eventdatei und Header, die freien Einträge liegen im allgemeinen Bereich, `free_flag` lehnt mit `scripts_not_adopted` ab (`scripts_indexed=false`), `name_flag` im DSPRE-Layout mit `not_text_project`; druckt `flags …` (Zeiten `list_ms`, `usage_ms`, `cold_ms`, die Zahl der Variablen, die Pools ohne Script-Verweise) |
| `vanilla_data_commands_on_copy` | `apps/studio/src-tauri/src/commands/data/tests.rs` | die Datenbefehle auf `vanilla_copy()` |
| `vanilla_every_battle_sprite_decodes_with_the_measured_counts`, `vanilla_every_form_sprite_decodes` | `crates/g4-kernel/tests/battle_sprites.rs` | Kampfsprites (3d.3): alle 1 812 Pokémon-Bilder, 157 / 161 Formenbilder und 105 / 129 Trainerbilder lesen sich; die Zahlen stimmen, das Mittel der zusammenhängenden Flächen liegt bei höchstens 3, mindestens 1 778 Bilder haben eine durchsichtige Ecke, jedes Formbild unterscheidet sich von Form 0. Druckt `sprites …`, `forms …`; Release, je Spiel einige Sekunden (unter Last bis 17 s) |
| `battle_sprites_over_the_command_path_on_the_vanilla_copy` | `apps/studio/src-tauri/src/commands/data/tests.rs` | `get_battle_sprite` und `get_trainer_sprite` über den Befehlsweg auf `vanilla_copy()`; mit `G4_TIMING_STRICT=1` höchstens 50 ms je erste Anfrage, **im Debug-Build** (ohne `--release`); druckt `sprite … first_ms=… cached_ms=… trainer_ms=…` |
| `vanilla_egg_moves_read_with_the_measured_counts`, `vanilla_egg_moves_are_in_ascending_species_order` | `crates/g4-kernel/tests/egg_moves.rs` | Ei-Attacken (3d.3 Teil B): 198 Arten, in HeartGold 1 847 Attacken (höchstens 14 je Art, Member 4 092 Byte), in Platin 1 711 Attacken in 1 910 Wörtern mit dem Budget 1 910 (Ausschnitt in `ov005.bin`); `write()` byte-gleich zum Member bzw. zum Ausschnitt. Der zweite druckt nur, ob die Marken aufsteigen. Druckt `egg_moves <Spielcode> source=… species=… moves=… words=… ms=…` und `egg_moves <Spielcode> ascending=… first=… last=…`; Release, Lesen der Tabelle 0 bis 11 ms |
| `text_project_egg_move_edit_builds_and_check_data_finds_it` | `crates/g4-kernel/tests/textproject_source.rs` | Textprojekt (3d.3 Teil B): Ei-Attacken 1 und 2 von Art 1 über den Store getauscht und gespeichert, nur `data/species/0001.json` ändert sich, der Abgleich findet nichts offen, der Bau trägt den Tausch (HeartGold per `g4k check-data egg_moves`, Platin im Ausschnitt von `ov005.bin` der entpackten ROM); braucht die Basis-ROM neben dem Projekt wie die anderen Tests dieser Datei; druckt `egg-moves-text <Spielcode> count=<n> move0=<Attacke>`; Release, mehrere Minuten (Export des ganzen Projekts) |
| `vanilla_world_edges_match_the_measured_counts` | `crates/g4-kernel/tests/refs.rs` | Welt-Kanten des deklarativen Index (3d.4, `metadata/entity_refs.yaml`): Header → Matrix / Matrix → Karte / Warp → Header je Spiel auf ±1 % (Platin 593 / 1 172 / 1 213, HeartGold DE 540 / 958 / 1 317), dazu Script → Header (Platin 67, HeartGold DE **114**; das Gutachten zählte 154, aber der erste Parameter von HGSS-`MoveWarp` ist ein Warp-Index); druckt `<Spielcode> <Kante>: n / verschiedene` für die 13 Kanten und `script → header`/`script → warp index`. Der Zuwachs liegt bei etwa 12 300 (Platin) und 11 300 (HeartGold) Verweisen; `vanilla_index_builds_under_a_second` gilt weiter (mit `G4_TIMING_STRICT=1`; unter Last nicht beurteilbar, auf ruhiger Maschine messen) |
| `textproject_round_trip_with_tombstone`, `textproject_export_idempotent_with_tombstone` | `crates/g4-kernel/tests/textproject.rs` | der Grabstein (`deleted: true` am letzten Trainer) überlebt Export, Import und Neuaufbau: der Import ignoriert den Umschlag, das Member kommt zurück, das ROM baut byte-gleich, die Datei behält den Umschlag; der zweite Export schreibt nichts (der Umschlag der Datei auf der Platte wird übernommen) |
| `a_branch_without_the_freed_file_builds_after_git_switch` | `crates/g4-kernel/tests/git.rs` | Gate 3d.4: ein neuer Trainer im Textprojekt in Git, auf dem Branch `ohne` gelöscht und freigegeben und committet; `git switch` hin und zurück: der Build hängt die Member wieder an bzw. entfernt sie nach der Kürz-Regel (`textproject.member-removed`), beide Builds gehen, `git status` bleibt leer. Braucht `git` (Identität über `GIT_AUTHOR_*`/`GIT_COMMITTER_*`) und die `.nds` neben dem Ordner |
| `delete_over_the_command_path_on_the_vanilla_copy` | `apps/studio/src-tauri/src/commands/delete.rs` | Löschen (3d.4) über den Befehlsweg auf `vanilla_copy()` als Textprojekt mit Basis-ROM: ein neuer Trainer, `plan_delete`, `delete_entity` als ein Undo-Schritt, Rückgängig, danach „Nummer freigeben“ (die JSON geht); druckt `text project set up: …`, `delete over the command path: … ms` und die Zeit der Trainerliste mit den Grabstein-Flaggen |
| `stories_over_the_command_path_on_the_vanilla_copy` | `apps/studio/src-tauri/src/commands/stories/tests.rs` | Geschichten (3d.6) über den Befehlsweg auf `vanilla_copy()` als Textprojekt mit übernommenen Scripts: `list_threads` mit mindestens 59 (Platin) bzw. 62 (HGSS) Fäden, `thread_graph` des ersten mit einem Wert-Knoten je Wert, `story_create` ändert unter allen Quellen genau `data/ids/stories.json`, `data/stories/0000.json` und `0000.layout.json`, der Faden nennt die Geschichte, `undo` stellt alle Quellen her; druckt `stories <Spielcode> threads_ms=… graph_ms=… create_ms=… threads=… first=… values=… edges=…`. Braucht die Basis-ROM neben dem Ordner |
| `vanilla_bdhc_round_trip_byte_identical_and_valid` | `crates/g4-formats/tests/vanilla_bdhc.rs` | BDHC (6.2): jede Vanilla-BDHC liest, schreibt byte-gleich zurück und besteht die Strukturprüfung (`Bdhc::validate` 0 Fehler); Platin 666 Karten, 0 Warnungen, HGSS 676 Karten, 1 Warnung (Karte 0464, senkrechte Platte); druckt Nutzlast min / Median / p90 / max mit Karte (Platin 21 452 B auf 188, HGSS 10 778 B auf 658) |
| `vanilla_map_100_floor_is_two_tiles_high`, `vanilla_heights_cover_the_walkable_tiles`, `vanilla_free_steps_are_counted`, `set_map_terrain_on_a_vanilla_copy_survives_save_and_reopen` | `crates/g4-kernel/tests/terrain.rs` (Feature `test-support`) | Gelände (6.2): Platin Karte 100 hat den Boden auf 32 E (2 Kacheln) und eine Kante auf 96 E; Höhen je Kachelmitte über alle Karten (Anteil freier Kacheln ohne Höhe gedruckt); freie Stufen über alle Karten gezählt und gedruckt (Platin 152, HGSS 52; das feste Soll hält `vanilla_checks_match_the_fixed_counts` für `map.step-unblocked`); `set_map_terrain` mit dem Szenario-Raster auf `vanilla_copy()` überlebt Speichern und Neuöffnen |
| `terrain_over_the_command_path_on_the_vanilla_copy` | `apps/studio/src-tauri/src/commands/terrain.rs` | `get_map_heights`, `get_map_budget` und `set_map_terrain` (6.2) über den Befehlsweg auf `vanilla_copy()`: Platin Karte 100 häufigste Höhe 32 E, Karte 188 `bdhc.used` 21 452 ruhig, HGSS Karte 658 10 778 ruhig; Setzen und Undo stellen die Bytes wieder her; druckt `terrain <Spielcode> …` mit Budget und VRAM-Schätzung einiger Karten |

Die Basis-ROM finden die gated Tests als `<Name>.nds` neben `<Name>_DSPRE_contents`; die Tests von 3d.1 (`tests/flags.rs`) und 3d.6 (`tests/threads.rs`, `tests/stories.rs`, `stories_over_the_command_path_on_the_vanilla_copy`) nehmen auch `<ordner>.nds` neben einem Ordner ohne den Zusatz (die Testkopien einer Phase, etwa `_testkopien/3d-6-pt` und `3d-6-pt.nds`). Die übrigen Tests mit Basis-ROM (etwa `delete_over_the_command_path_on_the_vanilla_copy`) melden auf einem solchen Ordner `skipped: …` und gelten als bestanden; für sie zeigt `G4_TEST_PROJECT` mit absolutem Pfad (cargo startet die Tests von `-p studio` in `apps/studio/src-tauri`) auf einen Symlink `<name>_DSPRE_contents` neben einem Symlink `<name>.nds` (in 3d.6 unter `target/agent/t10-vanilla/`).

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
- KIND `map` (seit 3b.4b) liest eine Kartendatei des ROMs mit `g4_formats::map` und hat zwei Felder: `model_sha1` (SHA-1 des Modells, so wie es in der Karte steht) und `per_sha1` (SHA-1 der Kollision, dieselben 2048 Byte wie ein exportiertes `.per`), beide als 40 Hex-Ziffern. So prüft ein Szenario, dass ein Import im gebauten ROM ankam: `--expect map:666:model_sha1=<SHA-1 der exportierten .nsbmd>`.
- KIND `egg_moves` (seit 3d.3) liest den einen Member der Ei-Attacken-Tabelle (HeartGold `a/2/2/9`, Platin nur mit dem
  Patch-NARC von DSPRE; die Tabelle in Overlay 5 liest es nicht, dann „CPUD has no egg_moves archive“); MEMBER ist hier
  die Artnummer, die Felder sind `count` (Zahl der Ei-Attacken, 0 ohne Eintrag) und `move0` bis `move15`:
  `--expect egg_moves:1:count=14 --expect egg_moves:1:move0=113` (Bisasam in HeartGold DE).
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

#### Projektprüfungen (3b.4a)

Seit 3b.4a prüft das Modul `crates/g4-kernel/src/checks/` das Projekt mit dem Katalog der Spezifikation 3b.4 §4 und den zwei
Ortsprüfungen aus 3f.2 (41 Codes, `Registry::all`; Grenzen und Ausnahmen mit Quelle in `crates/g4-kernel/metadata/check_limits.yaml`, beschrieben in
[docs/formats/check_limits.md](formats/check_limits.md)). Harte Regel: eine Prüfung ist nur dann „Fehler“, wenn sie auf
Platin DE, HeartGold DE und HeartGold US je 0 Treffer hat; das beweisen die gated Tests. Die gated Tests dazu:

| Test | Datei | Prüft |
|---|---|---|
| `vanilla_checks_match_the_fixed_counts` | `crates/g4-kernel/tests/vanilla_checks.rs` | alle Gruppen auf dem Vanilla-Projekt (nur lesend, Scripts nicht übernommen): die Zahl je Code gleicht dem festen Soll in `SOLL` (Spalten Platin DE, HeartGold DE, HeartGold US), jede Prüfung mit Schwere Fehler hat 0 Befunde; druckt Soll, Ist und Zeit je Code |
| `vanilla_script_checks_after_adopt` | ebenda (Feature `test-support`) | auf `vanilla_copy()` vor und nach `scripts::adopt`: die Slot-Prüfungen finden aus den Quellen dasselbe wie aus den Membern, `script.message-missing` läuft und lässt nur die Common-Scripts ohne Header aus (Platin 28, HeartGold 25 Dateien) |
| `overlay_under_lock_takes_under_5_ms_on_hgss` | `crates/g4-kernel/tests/checks_runner.rs` | auf `vanilla_copy()` mit allen Eventdateien und Textarchiven geladen und einigen geändert: die Kopie unter dem Store-Lock (`snapshot_overlay`) bleibt unter 5 ms |
| `full_run_on_vanilla_under_three_seconds_warm` | ebenda | ein voller Lauf über alle Gruppen, zweimal: der zweite mit dem Cache des ersten unter 3 s; druckt beide Zeiten |

Je Projekt einzeln (die Zeiten mit `--nocapture`; `--test-threads 1`, damit die Zeitmessungen nicht gegeneinander laufen):

```sh
G4_TEST_PROJECT="/<absoluter Pfad zu Roms>/Pokemon - Platin-Edition (Germany)_DSPRE_contents" \
  cargo test --workspace --all-features -- vanilla_checks_match_the_fixed_counts vanilla_script_checks_after_adopt \
  full_run_on_vanilla overlay_under_lock --nocapture --test-threads 1
```

Gemessen am 2026-10-25 (Dev-Build, je Projekt Platin DE / HeartGold DE / HeartGold US): 111 / 145 / 153 Befunde, alle
Fehler-Codes 0; alle Gruppen auf einem neuen Snapshot 1047 / 1083 / 1004 ms, die Prüfungen allein 158 / 164 / 135 ms;
voller Lauf kalt 3,46 / 1,18 / 3,60 s (der HeartGold-DE-Wert mit warmem Datei-Cache des Systems; in Task 6 kalt 3,8 s),
warm 287 / 280 / 247 ms; die Kopie unter dem Lock 0,8 bis 3,1 ms. Die vier Läufe brauchen 90 bis 100 s je Projekt.

**Wann geprüft wird.** Die Oberfläche (`apps/studio/src/checks/useChecks.ts`) startet einen vollen Lauf über alle Gruppen
(`run_checks { scope: "all" }`) beim Öffnen eines Projekts, nach einem Fix, auf „Erneut prüfen“ und 300 ms nach der letzten
Änderung (Bus-Thema `edited`, das `useProjectSession.refreshDirty` nach jedem Schreiben sendet und `useScripts` nach dem
Speichern eines Scripts oder Level-Scripts, „Script anfügen“ und dem Übernehmen; das erste Lesen nach dem Öffnen und das
Lesen nach einem Fix senden es nicht, damit dort genau ein Lauf läuft); in Tauri und im Browser-Modus gleich, es gibt
keinen Zeitgeber im Backend. Bis ein Bericht da ist, sagen Prüflicht, Panel und Technik nie „Alles stimmig“ oder „Keine
Probleme“: während des ersten Laufs „Prüft …“, ohne Bericht und ohne Lauf (der erste Lauf ist gescheitert oder kam ohne
Bericht) „Nicht geprüft“ mit „Erneut prüfen“ (`data-light="unchecked"`). Es läuft immer nur ein Lauf; Anfragen währenddessen werden zu einem
weiteren Lauf zusammengefasst, und solange er läuft, fragt die Oberfläche alle 250 ms `checks_status` nach dem Fortschritt.
Der volle Lauf auch nach jeder Änderung ist warm schnell, weil der Snapshot die Text- und Script-Ergebnisse je Datei cacht
(Schlüssel: Textgeneration und Dateistempel bzw. Dateistempel der Quelle, also Länge und Änderungszeit; auf einem
Dateisystem mit grober Änderungszeit wie HFS+ oder exFAT bliebe eine gleich lange Änderung derselben Sekunde unbemerkt,
APFS hat Nanosekunden): warm kostet er den billigen Teil plus die geänderten Dateien, gemessen
261 ms auf HeartGold DE (Task 6) und 250 bis 290 ms auf allen drei Projekten (oben); kalt, beim Öffnen, 1,2 bis 3,8 s mit
Fortschritt im Prüflicht („Prüft … Texte 312/724“). Jeder Bericht ersetzt die Befunde als Ganzes; ein Bericht ohne
Gruppen (`groups: []`), nur einiger Gruppen oder einer älteren Store-Generation ersetzt nichts. Ändert sich der Store
während eines Laufs, verwirft das Backend das Ergebnis und prüft neu. Dasselbe gilt für Schreibzugriffe am Store vorbei,
die die Prüfungen von der Platte lesen: das Speichern eines Scripts oder Level-Scripts, `add_script`, `new_script_file`,
`adopt_scripts`, `apply_fix create_script` und `create_trainer` zählen eine Epoche der Platte hoch
(`ChecksState::disk_written` über `Session::files_written`, `apps/studio/src-tauri/src/state.rs`); ein Bericht gilt nur,
solange Store-Generation und Epoche die seines Laufs sind (`finish_checks`, `current_report`), sonst liefert
`checks_status` keinen und `run_checks` prüft neu. Der Bereich `scope: "cheap"` (nur die billige
Gruppe) bleibt für die Brücke und Szenarien; die Oberfläche nutzt ihn nicht. In der Brücke ist `run_checks` synchron;
eine frische Sitzung (Szenario-Batch) prüft kalt, auf Platin DE mit übernommenen Scripts 1,5 bis 1,6 s.

**`g4-checks.json`.** Ausgeblendete Befunde und abgeschaltete Prüfungen stehen im Projekt in `<projekt>/g4-checks.json`
(nicht in `.g4/`, das ist löschbarer Cache): `{"version": 1, "ignored": [{"code", "subject": {"kind", "index"}, "key",
"fingerprint"}], "disabled": [code]}`, der Fingerprint als 16 Hex-Ziffern. Seit 4.3b steht `version` (1) immer in der Datei;
eine Datei ohne das Feld (3b.4 bis 4.3a) gilt als Version 1, eine höhere lehnt das Studio ab („g4-checks.json: Version 2
stammt aus einer neueren Fassung von Gen4 Studio.“). `subject.index` ist bei `header` und `trainer` eine ID oder eine
Nummer (`"MAP_T01"` oder `3`; geschrieben wird im Textprojekt die ID), bei allen übrigen Arten eine Nummer; Format in
[docs/formats/text_project.md](formats/text_project.md), „`g4-checks.json`“. `ignore_finding`, `unignore_finding`, `accept_baseline` und
`set_check_enabled` schreiben die Datei sofort (atomar); `open_project` liest sie. Eine kaputte Datei (von Hand bearbeitet,
ein Merge-Konflikt, von einem neueren Studio mit unbekannter Subjekt-Art) hält das Projekt nicht zu (Ruling R15): es öffnet
ohne ausgeblendete Befunde, jeder Bericht nennt die Datei als `skipped`-Zeile mit `code` „g4-checks.json“ und dem
Parse-Fehler (im Panel „g4-checks.json ist nicht lesbar (…): nichts ist ausgeblendet oder abgeschaltet …“), und die vier
Befehle lehnen jedes Schreiben mit „g4-checks.json ist nicht lesbar (…). Repariere oder lösche die Datei …“ ab, damit sie
nie ungelesen überschrieben wird. Vor jedem Schreibversuch liest das Studio sie neu: ist sie repariert oder gelöscht, gilt
sie ab da. `g4k check` lehnt eine kaputte Datei weiter mit „g4-checks.json: …“ ab. Fehler lassen sich nie ausblenden oder
abschalten. Der
Fingerprint geht über die geprüften Bytes des Subjekts: ändert sich die Stelle, passt er nicht mehr, und der Befund meldet
sich wieder; unveränderte bleiben ausgeblendet.

**`g4k check`.** `g4k check <projekt> [--json] [--expect CODE=N …]` führt alle Prüfungen auf dem gespeicherten Projekt aus
und liest dabei, wie das Studio, `<projekt>/g4-checks.json`: ausgeblendete Befunde zählen als „ignored“, abgeschaltete
Prüfungen laufen nicht (Fehler werden nie ausgelassen). Ohne `--json` eine Zeile je Befund und eine Summe, mit `--json`
der ganze Bericht (`findings`, `ignored`, `skipped` als `{code, why}`, Subjekte als `{kind, index}`, Fingerprints als
16 Hex-Ziffern). `--expect warp.dest-warp=1` vergleicht die Zahl der nicht ausgeblendeten Befunde je Code: Exit 1 mit
„warp.dest-warp: expected 1, found 0“ auf stderr; ein unbekannter Code endet vor dem Lauf mit Exit 2. Befunde allein
ändern den Exit-Code nicht. Beispiel nach dem Szenario `pt-pruefungen.yaml`:

```sh
target/debug/g4k check <Kopie> --expect warp.dest-warp=0 --expect event.script-missing=0 \
  --expect event.trainer-missing=0 --expect text.too-wide=0
# 0 finding(s): 0 error(s), 0 warning(s), 0 info(s); 111 ignored; 1553 ms … 4 expectation(s) met
```

Szenarien prüfen die Antworten der Brücke direkt mit Erwartungen an den Änderungen (`{expect: {path, equals}}`,
Beschreibung in [docs/agent-gate.md](agent-gate.md), „Szenario“).

Messlauf, nicht Teil des normalen Durchlaufs (`#[ignore]`): `G4_APICULA=... cargo test -p studio --release -- --ignored
--nocapture bench_matrix_models` lädt eine große Matrix (Standard: Header 3, Platin Matrix 0) und gibt die Zeiten aus;
`G4_BENCH_HEADER` wählt einen anderen Header.

#### Kartenteile importieren (3b.4b)

Seit 3b.4b importiert `crates/g4-kernel/src/map_import.rs` fertige Binärteile in bestehende Karten (`inspect`, `import`, `export`; Modell, Kollision, Gebäude, BDHC, BGS, ganze Kartendatei, „Teile aus Karte N übernehmen“), über `Store::edit_map` als einen Schritt im Verlauf; den NSBMD-Kopf liest `crates/g4-formats/src/nsbmd_header.rs` ([docs/formats/nsbmd_header.md](formats/nsbmd_header.md)). Alle Prüfungen laufen vor dem Schreiben, eine Ablehnung ändert nichts (`import_refuses_on_error_and_changes_nothing` im Kernel, `export_then_import_from_files_and_a_refusal_changes_nothing` im Studio).

**Fehlercodes.** Jede Fehlermeldung, Warnung und jeder Hinweis einer Teilprüfung trägt einen Code aus `PartCode` (`map_import.rs`, `PartCode::ALL`, `as_str`, etwa `model.too-big`, `permissions.size`, `whole.foreign`) mit Argumenten und einem englischen Satz (`PartMessage { code, args, message }`; auf dem Draht `state: {kind, code, args, message}` und `notes`). Den deutschen Satz mit Ausweg setzt die Oberfläche aus Code und Argumenten zusammen: `apps/studio/src/workbench/inspector/importParts.ts` (`PART_TEXTS`, `partText`, `partView`). Zwei Tests halten das zusammen: `every_error_and_warning_of_a_part_carries_its_code` (Kernel: jeder Pfad liefert seinen Code, die erreichten Codes und die benannten unerreichten ergeben `PartCode::ALL`) und `commands::map_import::tests::every_part_code_has_its_german_sentence` (Studio: die Schlüssel von `PART_TEXTS` sind `PartCode::ALL` in derselben Reihenfolge). Ein neuer Code braucht also beide Stellen. Die Ablehnung von `import_map_parts` bleibt der englische Satz („import refused: model: model is 65536 bytes; the game holds at most 61440 (0xF000)“).

**Pfade der Brücke.** `Roots::read_path` lässt Import-Dateien nur unter den erlaubten Ordnern (`--allow-root`, für `studio-web.sh` `$G4_ROMS_DIR/_testkopien`) und unter `G4_IMPORT_ROOT` (Ordner, mit `:` getrennt) zu, nie aus einem Original oder `_saves`; `Roots::write_path` behandelt ein Exportziel wie ein ROM-Ziel (`apps/studio/src-tauri/src/bridge/roots.rs`).

Die gated Tests dazu (`crates/g4-kernel/tests/map_import.rs`, auf `vanilla_copy()`):

| Test | Prüft |
|---|---|
| `every_vanilla_model_reimported_into_itself_is_byte_identical` | jedes Kartenmodell des Projekts, exportiert und in seine eigene Karte importiert, gibt dieselben Bytes |
| `map_a_into_b_and_back_restores_b` | alle Teile von Karte 5 in Karte 6 („Teile aus Karte N übernehmen“) als ein Schritt, dann aus den vorher exportierten Dateien zurück: Karte 6 hat wieder ihre Bytes, auch nach zweimal Undo |
| `hgss_bin_into_pt_and_back` | eine ganze Kartendatei der anderen Familie in Karte 6: Platin lässt den Hintergrundklang mit `whole.bgs-dropped` weg, HeartGold/SoulSilver legt einen leeren an, fremde Gebäude bleiben weg; zurück über die exportierte `.bin` |
| `per_round_trip` | `.per` von Karte 5 in Karte 6 und zurück |

```sh
G4_TEST_PROJECT="/<absoluter Pfad zu Roms>/Pokemon - Platin-Edition (Germany)_DSPRE_contents" \
  cargo test --workspace --all-features -- every_vanilla_model_reimported map_a_into_b hgss_bin_into_pt per_round_trip --nocapture
```

**Szenarien.** `g4emu scenario` kann seit 3b.4b: `${sha1-of:PFAD}` (SHA-1 einer Datei der Kopie, gelesen erst beim Einsetzen, etwa in `check:` nach dem Bauen), `{expect: {error: Teilstring}}` (die Brücke muss den Befehl ablehnen, das Szenario läuft weiter), einen Befehl ohne Argumente mit Erwartung (`[dirty, {}, {expect: …}]`) und den Abschnitt `prepare:` mit dem Schritt `make-oversize-nsbmd {path, bytes?}` (ein synthetisches Modell über 0xF000 Byte für den Gegenfall; `scripts/g4emu/g4emu/fixtures.py`, `make_oversize_nsbmd`; einen eigenen Befehl `g4emu make-oversize-nsbmd` gibt es nicht). Beschreibung in [docs/agent-gate.md](agent-gate.md), „Szenario“. Die Gesamtgate-Szenarien `pt-kleiner-hack.yaml` und `hg-kleiner-hack.yaml` brauchen je unter einer Minute (gemessen 47 bis 49 s, 23 Brücken-Sitzungen) plus den Bau der Werkzeuge; dazu gehören die Gegenproben `pt-kleiner-hack-ohne-import.yaml` und `hg-kleiner-hack-ohne-import.yaml`, die am Weg scheitern müssen. Rezept „Gesamtgate“ in [docs/agent-gate.md](agent-gate.md).

#### Projekt aus ROM anlegen (3g.1)

`crates/g4-kernel/src/start.rs` legt ein Projekt aus einer `.nds`-Datei an; Aufbau der Marke und der CRC-Wiederherstellung in [docs/formats/project_base.md](formats/project_base.md).

```sh
cargo build --workspace --all-features
target/debug/g4k inspect-rom <rom.nds>            # JSON: Titel, Spielcode, Familie, Sprache, SHA-1, known_base, support
target/debug/g4k create <rom.nds> <neuer-ordner>  # [--template original|data-only|empty-region]; druckt root=… und die Phasen mit ms
```

`inspect-rom` liest nur. `create` entpackt nach `<Eltern>/.<Name>.g4-new`, schreibt `.g4/base.json` und benennt erst dann nach `<neuer-ordner>` um; das Ziel muss fehlen oder leer sein. Es nimmt den Namen wörtlich: ein Ordner mit der Endung `_DSPRE_contents` bleibt ein Name wie jeder andere (die Brücke sperrt solche Namen, `g4k` nicht). Diamant/Perl lehnt es ab („Diamant/Perl: ohne Testprojekt nicht freigegeben“), ebenso eine Datei ohne Gen-4-Spielcode. Erfolgsweg-Tests ohne ROM benutzen einen Fake-Extraktor (`create_project_with` in `start.rs`), weil ds-rom die synthetische Kopf-ROM (`test_support::synthetic_rom`) nicht entpacken kann.

Der gated Test `project_start_create_then_build_matches_the_rom` (`G4_TEST_ROM`) legt das Projekt in einem Temp-Ordner an, baut es und vergleicht den SHA-1 der gebauten ROM mit dem der Eingabe (ohne `restore_header_crcs` weichen `0x6C` und `0x15E` ab):

```sh
G4_TEST_ROM="$PWD/../Roms/_testkopien/3g-probe/pt.nds" \
  cargo test --workspace --all-features -- project_start_ --nocapture
```

Im Gate-Skript läuft das als Schritt „gated tests (project start)“, einmal je `*.nds` direkt unter `$G4_ROMS_DIR` (nur gelesen); ohne `.nds` steht die Zeile „skipped (keine ROM)“. Der Filter `project_start_` wählt nur diese Tests; je ROM dauert der Schritt etwa 15 bis 25 s plus den Bau der Tests.

#### Leere Region mit Startstadt (3g.2)

`crates/g4-kernel/src/templates.rs` plant und wendet die Weltvorlage „Leere Region mit Startstadt“ an (Rezept `metadata/templates.yaml`, Start-Scripts `metadata/templates/start_pt.rotom` und `start_hgss.rotom`); Ablauf, Vorbedingungen und Start-Script in [docs/formats/templates.md](formats/templates.md). Rand-Karte, Level-Script und Start-Script schreibt der Kernel sofort (nicht rückgängig), alles Übrige ist ein Schritt im Verlauf („Vorlage: Leere Region mit Startstadt“).

```sh
target/debug/g4k template <projekt> empty-region   # übernimmt die Scripts, wenn nötig; wendet an, speichert, druckt den Bericht
target/debug/g4k create <rom.nds> <neuer-ordner> --template empty-region
```

`g4k template` schreibt ins Projekt; nur auf Kopien unter `_testkopien` anwenden. Die gated Tests (`crates/g4-kernel/tests/templates.rs`, auf `vanilla_copy()`):

| Test | Prüft |
|---|---|
| `vanilla_plan_empty_region` | der Plan: Startstadt, behaltene Räume, Rand-Zellen, leeres Level-Script |
| `vanilla_empty_region_has_no_check_errors` | Scripts übernehmen, anwenden, speichern: alle drei Prüfgruppen ohne Fehler, kein `text.too-wide` im Textarchiv der Stadt, ein neuer Plan sieht `already_empty` |
| `vanilla_empty_region_undo_restores_the_world` | ein ⌘Z stellt Header-Tabelle, Matrix 0 und die Eventdatei der Stadt byte-gleich wieder her, seit 3d.2 auch Start-Script, Level-Script-Datei, die berührten Textarchive und `compile-state.json` (das Projekt liegt im Test in Git: nach ⌘Z nennt `git status` nur die Rand-Karte); ⌘⇧Z bringt das Ergebnis zurück |
| `vanilla_empty_region_builds` | das gespeicherte Ergebnis baut ohne Fehler |

```sh
G4_TEST_PROJECT="/<absoluter Pfad zu Roms>/_testkopien/3g-b-pt" \
  cargo test --workspace --all-features --test templates -- vanilla_ --nocapture --test-threads 1
G4_TEST_ROM="/<absoluter Pfad zu Roms>/_testkopien/3g-probe/pt.nds" \
  cargo test --workspace --all-features -- project_start_ --nocapture
```

`project_start_empty_region_from_the_rom` (`G4_TEST_ROM`) legt ein Projekt mit `Template::EmptyRegion` an und verlangt die Marke `empty_region`, Fortschritt bis `done = total` und 0 Fehler in allen Prüfgruppen. Gemessen: die vier Tests in `templates.rs` brauchen auf Platin DE etwa 50 s, auf HeartGold DE etwa 40 s (dreimal Scripts übernehmen); die Vorlagen-Phase von `create_project` 4 bis 7 s.

#### Basis-ROM und Patch (4.1a)

`crates/g4-kernel/src/start.rs` prüft und ordnet eine Basis-ROM zu (Marke v2 mit `origin: "dspre"`), `base.rs` sucht sie über die SHA-1 und liest aus ihr, `patch.rs` schreibt den xdelta-Patch (xdelta3 3.0.12 im Crate `crates/g4-xdelta`); Formate in [docs/formats/project_base.md](formats/project_base.md) und [docs/formats/xdelta_patch.md](formats/xdelta_patch.md).

```sh
target/debug/g4k base check <projekt>                    # mark=…, sha1=…, suggestion=… (ohne Marke) bzw. base=… (Herkunft, etwa sibling)
target/debug/g4k base check <projekt> --rom <rom.nds>   # prüft die .nds als Basis-ROM, je Warnung eine Zeile warning:
target/debug/g4k base assign <projekt> <rom.nds>        # schreibt .g4/base.json; falsches Spiel: Exit 1
target/debug/g4k patch create <projekt> [--base-rom <rom.nds>] [--out <patch>] [--json]
target/debug/g4k publish <projekt> --version <v> [--notes <text> | --notes-file <pfad>] [--dir <ordner>] [--file <name>] [--readme] [--overwrite] [--base-rom <rom.nds>] [--json]
```

`patch create` baut nach `build/<Name>.nds` und schreibt `build/<Name>.xdelta` mit `<Name>.xdelta.txt` (`<Name>` = `<Projekt>-<Spielcode>`); die Basis-ROM kommt aus `--base-rom`, sonst der Nachbar-`.nds`, sonst `G4_BASE_ROM_DIR`. Ohne Marke endet es mit Exit 1 und „Diesem Projekt ist keine Basis-ROM zugeordnet. Ordne sie mit `g4k base assign <Projekt> <nds>` zu.“; eine unlesbare oder neuere Marke lehnt es mit denselben Sätzen ab wie App und MCP. `publish` (4.5) baut genauso, schreibt die Veröffentlichung `<Projekt>-v<Version>.xdelta` mit Begleitdatei und wahlweise `README.txt` nach `--dir` (Standard `build/`) und trägt sie in `.g4/releases.json` ein; Ausgabe und Ablehnungen in [docs/formats/xdelta_patch.md](formats/xdelta_patch.md#veröffentlichen). `base assign`, `patch create` und `publish` schreiben ins Projekt: nur auf Kopien unter `_testkopien`.

Die gated Tests in `crates/g4-kernel/tests/base.rs` (`G4_TEST_PROJECT`; die Basis-ROM ist die `.nds` neben dem Original, gefunden über `suggest_base` und nur gelesen; was schreibt, arbeitet auf `vanilla_copy()`):

| Test | Prüft |
|---|---|
| `base_assign_on_the_vanilla_folder_has_no_warning_and_rebuilds_the_nds` | Zuordnen ohne Warnung; der Build der Kopie mit Marke hat die SHA-1 der `.nds` |
| `base_rom_members_equal_the_vanilla_folders` | die Archive aus der ROM gleich den Ordnern des Projekts |
| `base_rom_header_table_equals_the_vanilla_project` | die Header-Tabelle aus dem ARM9 der ROM gleich der des Projekts |
| `patch_after_map_and_after_map_script_text_is_small_and_applies` | Patch nach einer Kartenänderung ≤ 1 KiB, nach Karte, Script und Text an einer Stelle ≤ 16 KiB, Erzeugen ≤ 1 s, mit dem eigenen Decoder und (wenn im `PATH`) mit `xdelta3 -d` angewandt gleich; Textarchiv 0 als Stresszeile ohne Grenze |
| `publish_after_a_map_edit_writes_a_small_patch_that_applies` (4.5) | `build_and_publish` nach einem Kartenbyte (`0.1.0`, mit `README.txt`): Phasen `build`, `encode`, `verify`, `write`, `record` in dieser Reihenfolge, Patch ≤ 1 KiB, mit dem eigenen Decoder und (wenn im `PATH`) mit `xdelta3 -d` angewandt gleich der gebauten ROM, Begleitdatei mit `Version:` und `Änderungen:`, ein Eintrag in `.g4/releases.json`; `0.1.1` danach gibt zwei Einträge; druckt `publish <Spielcode> size=… build_ms=… encode_ms=… verify_ms=…` |

```sh
G4_TEST_PROJECT="/<absoluter Pfad zu Roms>/Pokemon - Platin-Edition (Germany)_DSPRE_contents" \
  cargo test -p g4-kernel --all-features --test base -- --nocapture --test-threads 1
```

So lief es im Gate von 4.1a; in einem Worktree, in dem `-p` und `--test` den Download von rotom auslösen (siehe oben), dasselbe mit `cargo test --workspace --all-features --test base -- …`. Auf stderr stehen `rebuild <Spielcode> sha1=… equal`, `patch-sizes <Spielcode> map=… all=… encode_ms=…/… verify_ms=…/…`, `patch-stress …`, `xdelta3 cross-check: map=… all=… stress=…` und (4.5) `publish <Spielcode> size=… build_ms=… encode_ms=… verify_ms=…`. Gemessen am 2026-10-27 (Debug-Build): 132 s (Platin DE), 232 s (HeartGold DE), 139 s (HeartGold US) für die vier Tests; Werte in [docs/phase-4-1a-gate.md](phase-4-1a-gate.md). In `scripts/gate.sh` laufen sie im Schritt der gated Tests je Projekt mit.

#### Auf Original zurücksetzen (4.1b)

`crates/g4-kernel/src/original.rs` setzt eine Entität auf den Stand der Basis-ROM zurück (`plan_reset`, `reset`; Store-Arten über `Tx::replace`, Header Feld für Feld, Scripts, Texturen und Modelle als Dateischritt), `original/changed.rs` listet, was vom Original abweicht (xxh3 je Basis-Member gegen die Fingerabdrücke aus `.g4/build/state.json`), `original/base_hashes.rs` hält die Basis-Hashes im App-Cache; Format in [docs/formats/project_base.md](formats/project_base.md), Abschnitt „Auf Original zurücksetzen“, Werkzeuge in [docs/mcp.md](mcp.md). Einen `g4k`-Unterbefehl gibt es nicht; kopflos geht es über `target/debug/studio mcp --project <Kopie> --base-rom <nds>` und die Werkzeuge `changed_from_original`, `plan_reset`, `reset_to_original`.

Die gated Tests in `crates/g4-kernel/tests/original.rs` (`G4_TEST_PROJECT`; die Basis-ROM ist die `.nds` neben dem Projekt, gefunden über `test_support::vanilla_base_rom()` und nur gelesen; alle schreiben auf `vanilla_copy()`):

| Test | Prüft |
|---|---|
| `reset_of_each_store_kind_gives_the_base_member_back` | Karte, Eventdatei, Textarchiv ganz, Nachricht, Trainer, Pokémon (Personal, Lernset, Entwicklung), Begegnungsdatei: ändern, speichern, zurücksetzen, speichern; jeder Member byte-gleich zum Basis-Member, jeder Reset ≤ 200 ms (vom Öffnen der Basis-ROM bis zum aufgezeichneten Schritt), ⌘Z stellt die Änderung wieder her |
| `header_reset_field_by_field_gives_the_base_record_back` | Header erst ein Feld, dann der Rest, dazu mit Dynamic-Headers-Patch (Basis im ARM9, Projekt im NARC): der gespeicherte Datensatz gleich der Basis, ⌘Z zum halben Stand |
| `script_texture_and_model_reset_give_the_base_member_back` | nach `scripts::adopt` Scriptdatei, Kartentextur und Gebäudemodell: Member byte-gleich, ⌘Z schreibt die alten Bytes zurück |
| `changed_list_after_edits_names_them_and_after_reset_not` | nach einem Build ist die Liste leer (zweiter Lauf ≤ 2 s, hasht nur racy Einträge), nach Änderungen an Karte und Trainer nennt sie genau diese, nach dem Zurücksetzen der Karte nur noch den Trainer |
| `decompile_member_gives_the_source_rotom_writes_for_every_script_file` | `decompile_member` gibt für jede Scriptdatei mit globalen Aufrufen denselben Text wie rotoms eigenes Dekompilieren |

Dazu der gated Studio-Test `reset_over_the_command_path_on_the_vanilla_copy` (`apps/studio/src-tauri/src/commands/original.rs`): Liste, Plan und Reset über den Befehlsweg der App; `ms` zählt das Finden und Öffnen der Basis-ROM mit.

```sh
G4_TEST_PROJECT="/<absoluter Pfad zu Roms>/_testkopien/4-1b-pt_DSPRE_contents" \
  cargo test -p g4-kernel --all-features --test original -- --nocapture --test-threads 1
G4_TEST_PROJECT="/<absoluter Pfad zu Roms>/_testkopien/4-1b-pt_DSPRE_contents" \
  cargo test -p studio --all-features --lib reset_over_the_command_path -- --nocapture --test-threads 1
```

So lief es im Gate von 4.1b, auf frischen Kopien mit der `.nds` daneben (`4-1b-pt.nds` usw.); in einem Worktree, in dem `-p` und `--test` den Download von rotom auslösen (siehe oben), über `cargo test --workspace --all-features` mit Namensfilter. Auf stderr stehen `reset <Spielcode> <Ziel> ms=… (kernel … ms)`, `changed <Spielcode> first_ms=… second_ms=… files=… recorded=… racy=…` und `decompile <Spielcode> <n> files with global calls ms=…`. Gemessen am 2026-10-27 (Debug-Build): 205 s (Platin DE), 210 s (HeartGold DE, roter erster Lauf; danach `changed_list…` allein grün), 236 s (HeartGold US) für die fünf Tests; jeder Reset 9 bis 108 ms, die Liste erst 10 bis 17 s, danach 0,1 bis 0,6 s; Werte in [docs/phase-4-1b-gate.md](phase-4-1b-gate.md). In `scripts/gate.sh` laufen sie im Schritt der gated Tests je Projekt mit.

#### Weltmodell (3e.2)

`apps/studio/src/workbench/worldModel.ts` (`buildWorld`) baut den Weltbaum der Werkbank aus Headern, Matrizen, Warps und der Projektdatei der Welt (Weltkarten, Gebiete, Räume). `worldModel.vanilla.test.ts` prüft das Modell an den drei Vanilla-Projekten. Der Test liest keine ROMs, sondern JSON-Dumps, die `scripts/agent/world-dump.sh` über die Brücke schreibt; ohne `G4_WORLD_DUMP` meldet er einen übersprungenen Test mit Hinweis.

Die Dumps (je Projekt ein Ordner: `project.json`, `headers.json`, `matrices.json`, `matrix-<n>.json` für jede Matrix mit Abschnitt, `warps.json`, `meta.json`) entstehen aus frischen Kopien unter `_testkopien`, bei laufender Brücke (`scripts/agent/studio-web.sh`; `call.sh` liest Port und Token aus `target/agent/`):

```sh
for p in 3e-pt 3e-hgd 3e-hgu; do
  scripts/agent/world-dump.sh "/<absoluter Pfad zu Roms>/_testkopien/$p" "$PWD/target/agent/world-dump/$p"
done
G4_WORLD_DUMP="$PWD/target/agent/world-dump" \
  pnpm --dir apps/studio exec vitest run src/workbench/worldModel.vanilla.test.ts
```

Der Pfad in `G4_WORLD_DUMP` muss absolut sein; die Ordner heißen `3e-pt` (Platin DE, `CPUD`), `3e-hgd` (HeartGold DE, `IPKD`) und `3e-hgu` (HeartGold US, `IPKE`). Geprüft wird je Projekt: die Weltkarten (Platin genau „Sinnoh“, HeartGold/SoulSilver „Johto“ und „Kanto“), mindestens 90 % der Räume einem Gebiet zugeordnet, jeder Header genau einmal im Baum, `buildWorld` unter 200 ms; dazu bei Platin Großmoor und Kampfzone unter „Weitere Gebiete“ und alle Räume des Kraterbergs unter einem Gebiet (mindestens einer unter einem anderen Ort, seinem Eingang), bei HeartGold/SoulSilver die Safari-Zone unter „Weitere Gebiete“. Gemessen (Brücke, frische Kopien): Platin 482 von 492 Räumen zugeordnet, HeartGold/SoulSilver je 427 von 448, `buildWorld` jeweils unter 5 ms.

#### Build-Graph (4.2)

Der Build läuft nur die Schritte, deren Eingaben sich geändert haben (`rotom`, `chatot`, je Archiv `pack:<Ordner>`, `watch:<Ordner>`, `rom`; der ROM-Schritt liefert `blz:arm9`, `blz:overlays`, `rom`, `crc`). Seinen Zustand legt er unter `<Projekt>/.g4/build/` ab: `state.json` (die Fingerabdrücke der Eingaben und Ausgaben je Schritt und der Textabgleich JSON gegen Member, Format in [docs/formats/build_state.md](formats/build_state.md)) und `blz/` (die komprimierten ARM9- und Overlay-Dateien). Ohne den Ordner baut der Build alles voll und byte-gleich. Der Textabgleich (`texts` in `state.json`) ist aber kein Cache, sondern entscheidet, ob die JSON oder der Member eines Textarchivs gilt: „Neu bauen (ohne Cache)“ (`--no-cache`, `fresh: true`) löscht darum alles unter `.g4/build/` außer ihm und schreibt `state.json` nur mit `texts` neu. Alle Schritte laufen dann voll, und ein Member, den DSPRE seit dem letzten Bauen geändert hat, gilt weiter. Nur ganz ohne Zustand (frischer Klon, gelöschter Ordner) gilt für jedes Archiv die JSON, und der Build meldet das als Hinweis „N Textarchive ohne Bauzustand: die JSON-Fassung gilt.“

```sh
cargo build --workspace --all-features
target/debug/g4k build <Kopie> --out <Kopie>/out.nds --timings    # eine JSON-Zeile: {"ms", "rom", "steps": [{id, status, ms, done, total, cached, reason}]}
target/debug/g4k build <Kopie> --out <Kopie>/out.nds --no-cache   # löscht .g4/build/ zuerst (bis auf den Textabgleich), alle Schritte laufen voll
```

`--timings` schreibt statt der Textzeilen genau eine JSON-Zeile auf stdout (Probleme weiter auf stderr); `status` ist `skipped`, `incremental` oder `full`, `reason` nennt bei einem vollen Lauf den Grund (`no-state`, `build.cache-stale` nach einem Werkzeugwechsel, `output-changed`, `removed`; bei `crc` `no-base-mark`). Ohne `--timings` nennt die Zeile `packed` die Arten, deren Pack-Schritt lief (übersprungene fehlen). In der App entspricht `--no-cache` dem Befehl `build_rom` mit `fresh: true`. Ein Textkonflikt (JSON und Member beide geändert) bricht `g4k build` mit der Meldung „text archives changed as JSON and as member since the last build: …“ ab; in der App und über MCP ist das der Fehler `text_conflict`, aufgelöst mit `resolve_text_conflicts`. `g4k` selbst kann einen Textkonflikt nicht auflösen; ohne App geht das über `studio mcp --project <Kopie> --allow-irreversible` und das Werkzeug `resolve_text_conflicts` (unumkehrbar).

Der gated Test `build_timings_prints_one_json_line` (`crates/g4-kernel/tests/cli.rs`, auf `vanilla_copy()`) baut die Kopie zweimal mit `--timings`: Die Ausgabe ist je eine JSON-Zeile mit dem Schritt `rom`, der beim ersten Mal voll läuft und beim zweiten Mal übersprungen wird. `build_no_cache_removes_the_state_first` prüft `--no-cache` ohne ROM-Daten.

Die gated Tests in `crates/g4-kernel/tests/build_graph.rs` (alle auf `vanilla_copy()`) prüfen, dass der inkrementelle Build dieselben Bytes liefert wie ein Build ohne Cache an einen anderen Pfad:

- `incremental_equals_full_after_nothing_changed`, `a_new_out_path_runs_the_rom_step`, `map_edit_incremental_equals_full` (Karte über den Store), `text_edit_incremental_equals_full` (Text über den Store: `chatot` prüft nur dieses Archiv und findet den Member aktuell (der Store hat ihn beim Speichern geschrieben), `pack:textArchives` setzt einen Member ein), `script_edit_incremental_equals_full` (nach `scripts::adopt` ein Script mehr über `write_source`: `rotom` übersetzt genau diese Datei);
- `simulated_dspre_edit_is_detected`: ein Karten-Member an Ort und Stelle mit gleicher Größe überschrieben, die mtime zurückgesetzt (gleiche Größe, mtime und Inode, nur die ctime ändert sich); `pack:maps` läuft inkrementell, die ROM gleicht dem vollen Build;
- `salt_change_builds_fully`: die Salze von `pack:maps` und `rom` in `state.json` ersetzt; beide laufen voll mit `build.cache-stale`, die übrigen Schritte nicht, die ROM bleibt gleich;
- `dspre_mark_build_matches_the_base`: läuft nur, wenn die Kopie eine Basis-Marke hat (nach 4.1a `origin: "dspre"`), und vergleicht den SHA-1 der gebauten ROM mit der Marke; sonst „skipped: no base mark“;
- `build_graph_timings`: die Zeiten von „Hier spielen“ (ein Ausgabepfad). Je Fall ein Aufwärmlauf und drei Läufe, deren Median: `full` (ohne Cache), Karte, Text und Script geändert, keine Änderung; dazu der erste Build ohne Zustand als `cold` (ohne Grenze). Die Zeile auf stderr lautet `build-timings: {"game":"CPUD","full_ms":…,"map_ms":…,"text_ms":…,"script_ms":…,"none_ms":…,"cold_ms":…}`. Mit `G4_TIMING_STRICT=1` gelten Karte, Text und Script ≤ 500 ms und keine Änderung ≤ 200 ms, sonst gibt der Test nur aus.

Dazu `project_start_incremental_build_matches_the_rom` (`G4_TEST_ROM`): ein neues Projekt dreimal an denselben Pfad gebaut (frisch, ohne Änderung, nach einer Karte, die geändert und zurückgeschrieben wurde); jedes Mal ist der SHA-1 der ROM der aus `read_base_mark`.

Die Zeitgrenzen gelten für die App, die im Release-Profil läuft. Im Debug-Profil braucht der Build drei- bis viermal so lange (Platin DE: Karte 1,3 s, keine Änderung 0,6 s), darum misst der Gate-Schritt mit `--release`:

```sh
env G4_TEST_PROJECT="$G4_ROMS_DIR/Pokemon - Platin-Edition (Germany)_DSPRE_contents" G4_TIMING_STRICT=1 \
  cargo test --release -p g4-kernel --all-features --test build_graph -- build_graph_timings --nocapture --test-threads 1
```

Gemessen (Release, 27.10.2026, Median in ms, je zwei Läufe, Last 3 bis 9): Platin DE Karte 244–247, Text 273–290, Script 306–314, keine Änderung 95–96, ohne Cache 1543–1566; HeartGold DE 265–284, 281–289, 287–341, 95–103, 6336–6985; HeartGold US 260–274, 254–266, 290–306, 94–97, 6506–7191. Vor Task 9b waren es Karte 284–349, Text 300–404, Script 454–508 und keine Änderung 149–183 ms. Den Abstand zu den Grenzen schaffen drei Dinge (siehe [build_state.md](formats/build_state.md)): `state.json` wird im Prozess gehalten, solange ihr `stat` gleich bleibt (vorher je Build sieben- bis dreizehnmal gelesen, je rund 10 ms), die gerade geschriebene ROM und die NARCs werden mit dem Hash aus dem Speicher aufgezeichnet, und rotoms Übersetzung nimmt die Hashes unveränderter Quellen aus dem Speicher (vorher rund 150 ms je Build für das Lesen aller Quellen). Den Rest tragen die ROM selbst (ds-rom lädt, baut und schreibt rund 128 MB, etwa 110 bis 130 ms) und die Fingerabdrücke der rund 40 Schritte. Unter Last streuen die Zeiten; das Gate wiederholt einen gescheiterten Lauf einmal und schreibt „ok (2nd run)“ in die Tabelle.

#### Textprojekt (4.3a)

`crates/g4-kernel/src/textproject/` schreibt ein Projekt als Textprojekt (Manifest `g4-project.json`, je Entität eine JSON unter `data/<art>/NNNN.json`, Register `data/ids/`, Blobs `data/raw/`, `data/rotom-quirks.json`) und liest es zurück; Format in [docs/formats/text_project.md](formats/text_project.md). Kopflos:

```sh
target/release/g4k text export <Kopie>    # braucht Basismarke und Basis-ROM (--base-rom, sonst die Nachbar-.nds, sonst G4_BASE_ROM_DIR); übernimmt nicht übernommene Scripts; lehnt ab, solange noch nicht gebaute JSON-Änderungen da sind (--force verwirft sie)
target/release/g4k text check <Kopie>     # schema=…, Probleme, differs=N; schreibt nie
target/release/g4k text import <Kopie>    # prüft alles, bevor es den ersten Member schreibt; lehnt ab, solange Member neuer als ihre JSON sind (--force verwirft sie)
target/release/g4k text sources <Kopie>   # die Dateien des Textprojekts, eine je Zeile (für git add)
target/release/g4k text migrate <Kopie>   # ältere Schema-Version mit Schnappschuss unter .g4/backup/
target/release/g4k text adopt <Kopie>     # als Textprojekt einrichten (Scripts, Export, rotom.toml, Aufzeichnung); phase=… ms=… je Phase
target/release/g4k text sync <Kopie>      # Einseitiges übernehmen; conflict/broken <schlüssel> <label>, Exit 1, solange Konflikte oder fehlerhafte Dateien bleiben; --keep studio|dspre <schlüssel>
target/release/g4k text revert <Kopie>    # zurücknehmen: Manifest, data/, ID-Dateien nach .g4/backup/textproject-<Zeit>/
```

**Basisschicht:** Der Export vergleicht mit der Basis-ROM, extrahiert und mit allen Archiven entpackt unter `<Projekt>/.g4/cache/base-layer/<sha1>/` (rund 150 MB und 18 500 Dateien bei Platin, 182 MB und 20 000 Dateien bei HeartGold), einmal je Basis angelegt (6 bis 8 s bei geringer Last) und jederzeit löschbar; sie liegt nie in Git. Ein Ordner neben `data/` in der Projektwurzel (etwa eine Sicherung `data.prev/`) gilt als nicht abgebildete Änderung und wird ein Blob unter `data/raw/`; Sicherungen gehören außerhalb des Projekts.

Die gated Tests (`G4_TEST_PROJECT`; die Basis-ROM ist die `.nds` neben dem Original, gefunden über `suggest_base` und nur gelesen; alles Schreibende auf `vanilla_copy()`):

| Test | Prüft |
|---|---|
| `textproject_round_trip` (`tests/textproject.rs`) | Export, alle Member mit Entität sowie Text- und Script-Member mit Quelle löschen (Header-Tabelle und interne Namen genullt), Import: jede Datei unter `unpacked/`, `arm9.bin` und `mapname.bin` byte-gleich, der Build hat die SHA-1 der Basis; 0 Blobs, 0 Beifiles |
| `textproject_export_idempotent` | ein zweiter Export schreibt und löscht nichts, keine Quelle ändert Inhalt oder mtime |
| `textproject_from_base` | `extract_rom` der Basis-ROM plus die Dateien aus `sources`, Import, Build: SHA-1 gleich der Basis |
| `vanilla_base_layer_has_no_raw_blobs` (`tests/textproject_vanilla_probe.rs`) | die Basisschicht gleicht dem DSPRE-Ordner außerhalb der abgebildeten Member (keine Blobs) |
| `vanilla_shared_item_members_are_counted` | wie viele Item-IDs sich einen Member teilen (`same_member_as`) |

```sh
G4_TEST_PROJECT="/<absoluter Pfad zu Roms>/Pokemon - Platin-Edition (Germany)_DSPRE_contents" \
  cargo test --release -p g4-kernel --all-features --test textproject --test textproject_vanilla_probe \
  -- textproject_round_trip textproject_export_idempotent textproject_from_base vanilla_ --nocapture --test-threads 1
```

Auf stderr stehen `textproject-stats <Spielcode> files=… bytes=… json=… side=… raw=… base_layer_ms=… export_ms=… import_ms=… build_ms=…`, `textproject-second-export … ms=…`, `textproject-from-base … extract_ms=… import_ms=… build_ms=… equal`, `base-layer … ms=… diff_ms=… files=… bytes=… not_built=…` und `items … ids=… members=… shared=… unreferenced=…`, dazu je Art `textproject-art …` und je gesperrtem Member `textproject-locked …` (in Vanilla keine). Gemessen am 2026-10-27 (Release, Last 4 bis 14): 347 s (Platin DE), 684 s (HeartGold DE), 554 s (HeartGold US) für die fünf Tests samt Bau; Werte und Zeiten in [docs/phase-4-3a-gate.md](phase-4-3a-gate.md). In `scripts/gate.sh` laufen sie im Schritt der gated Tests je Projekt mit (dort im Debug-Profil). Den Diff zweier exportierter Projekte mit den IDs als Nummern zeigt `python3 -I scripts/agent/textproject_diff.py <Export A> <Export B>` (siehe [docs/agent-gate.md](agent-gate.md), „Phase 4.3a“).

#### Textprojekt als Quelle (4.3b)

Seit 4.3b liest und schreibt der Store in einem Projekt mit `g4-project.json` die Entitäts-JSON (`textproject::source`), der Abgleich mit DSPRE steht in `.g4/build/state.json` (`sync`, `textproject::sync`), Umstellung und Rücknahme in `textproject::adopt`, die ID-Dateien für rotom in `.rotom/command_database/.g4-ids-<art>.json` (`textproject::include`), „ID umbenennen“ in `textproject::rename`; Format in [docs/formats/text_project.md](formats/text_project.md). Kopflos: `g4k text adopt`, `g4k text sync [--keep studio|dspre <schlüssel>]` und `g4k text revert` (Kurzbeschreibung unter „Textprojekt (4.3a)“), dazu `g4k text rename-id <Kopie> <alt> <neu>` (eine Zeile `<datei> <anzahl>` je Datei, dann `renamed … files=… refs=… ms=…`; ohne Undo, eine Ablehnung schreibt nichts).

Die gated Tests (`G4_TEST_PROJECT` mit der `.nds` daneben, nur gelesen; alles Schreibende auf `vanilla_copy()`):

| Test | Prüft |
|---|---|
| `store_on_json_edits_only_the_entity_file_and_builds_like_dspre` (`tests/textproject_source.rs`) | Pikachu über den Store geändert: nur `data/species/0025.json` ändert sich, Textprojekt und DSPRE-Ordner bauen dieselbe ROM; druckt `store <Spielcode> save_ms=… load_ms=…` |
| `simulated_dspre_change_is_adopted_one_sided` | voller Abgleich, Fokus-Abgleich ohne Änderung, ein Member-Byte übernommen, `g4k check-data` auf der ROM; druckt `sync-full …` und `sync <Spielcode> stat_files=… stat_ms=… one_ms=…` |
| `simulated_change_on_both_sides_is_a_conflict_and_both_choices_work` | Trainer 1 auf beiden Seiten, Diff, beide Wahlen |
| `adopt_dspre_folder_builds_byte_equal_and_reverts` | Umstellung und Rücknahme, jeweils Build mit der SHA-1 der Basis; druckt `adopt <Spielcode> phases …` und `revert …` |
| `vanilla_checks_on_the_text_project_have_no_errors` | alle Prüfungen auf dem Textprojekt, 0 Fehler |
| `recipes_with_ids_apply_on_the_vanilla_project` | die Vorlagen-Rezepte mit IDs |
| `script_refs_on_the_vanilla_project_are_counted`, `a_script_using_an_id_constant_compiles_to_the_number` | Script-Verweise im Referenzindex, ID-Dateien für rotom; druckt `refs …` und `ids …` |
| `rename_id_updates_every_reference_on_the_vanilla_project` | „ID umbenennen“ eines Trainers über alle Verweise; druckt `rename …` |
| `mcp_with_ids_acts_like_numbers_on_the_vanilla_copy` (`apps/studio/src-tauri/src/mcp/tools.rs`) | MCP `adopt_text_project`, dann `data_edit` mit `SPECIES_PIKACHU` wie mit `25` |
| `project_start_text_project_builds_byte_equal` (`tests/project_start.rs`, `G4_TEST_ROM`) | der Assistent mit „als Textprojekt“ baut die ROM byte-gleich |

```sh
G4_TEST_PROJECT="/<absoluter Pfad zu Roms>/_testkopien/4-3b-pt_DSPRE_contents" \
  cargo test --release -p g4-kernel --all-features --test textproject_source -- --nocapture --test-threads 1
G4_TEST_PROJECT=… cargo test --release -p studio --all-features --lib mcp_with_ids_acts_like_numbers -- --nocapture --test-threads 1
G4_TEST_ROM="/<absoluter Pfad zu Roms>/_testkopien/4-3b-pt.nds" \
  cargo test --release -p g4-kernel --all-features --test project_start -- project_start_text_project_builds_byte_equal --nocapture
```

`G4_TIMING_STRICT=1` macht die Zeiten zu Grenzen (Speichern ≤ 200 ms, Fokus-Abgleich ≤ 500 ms, eine Änderung ≤ 1 000 ms). Gemessen am 2026-10-29 (Release, Last 3 bis 18): `textproject_source` 798 s (Platin DE), 938 s (HeartGold DE), 845 s (HeartGold US), der Studio-Test 77 bis 94 s, der Projektstart 53 bis 57 s; Werte in [docs/phase-4-3b-gate.md](phase-4-3b-gate.md). In `scripts/gate.sh` laufen sie ohne Änderung mit: die Kernel- und Studio-Tests im Schritt der gated Tests je Projekt (Debug), der Projektstart im Schritt `project_start_` je `.nds`. Der volle Lauf von `scripts/gate.sh` dauerte damit 8 324 s (gated Tests je Projekt 2 168 bis 2 340 s im Debug-Profil). Die gated Studio-Tests müssen auch mit gesetztem `G4_ROMS_DIR` laufen, wenn `G4_TEST_PROJECT` das Original ist (so ruft das Gate-Skript sie auf): eine Basis-ROM für den `UserPolicy`-Weg gehört als Kopie in den Temp-Ordner, nie der Ordner der Originale als `--allow-dir`.

#### Git minimal (4.4)

„Git einrichten“ (`git::cli::git_init`, Format in [docs/formats/project_git.md](formats/project_git.md)) ruft die
`git`-CLI. Die Tests, die sie brauchen, laufen nur, wenn `find_git()` etwas findet, und immer in einer eigenen Umgebung
(`test_support::GitFixture::cli(home)`: `GIT_CONFIG_GLOBAL` auf eine leere Datei in einem Temp-Ordner,
`GIT_CONFIG_NOSYSTEM=1`, `HOME` auf den Temp-Ordner, also ohne Identität und ohne die Einstellungen des Entwicklers);
weitere Variablen kommen über `GitCli::env` dazu, nie über `std::env::set_var`. Repositories nur in Temp-Ordnern, nie ein
Remote, nie `git clone`.

Die gated Tests in `crates/g4-kernel/tests/git.rs` (`G4_TEST_PROJECT` und `git`; Identität über `GIT_AUTHOR_*` und
`GIT_COMMITTER_*` in der Umgebung, alles auf `vanilla_copy()`):

| Test | Prüft |
|---|---|
| `git_setup_on_the_dspre_folder_keeps_status_empty_after_build` | einrichten, Status leer; `build::build` nach `build/`, Status leer; `git repack -a -d`, `git count-objects -v`: `packed_kib` ≤ 65 MiB, `files` ≤ 20 027, kein `.nds` und unter `.g4/` nur `base.json`/`preview.png` im Index; druckt `git <Spielcode> dspre files=… ignored_archives=… unpack_ms=… add_ms=… commit_ms=… status_ms=… packed_kib=…` |
| `no_game_data_file_falls_under_an_eol_rule` | nach dem Auspacken trägt keine Datei unter `files/` und `unpacked/` eine Endung mit `eol=lf` (`.json`, `.rotom`, `.yaml`, `.yml`, `.toml`, `.md`); unter `arm9/`, `arm9_overlays/`, `arm7/` nur die `.yaml` von ds-rom, ohne CR; druckt `eol …` |
| `a_clone_with_autocrlf_builds_byte_equal_dspre` | Klon über `git init`, `config core.autocrlf true`, `fetch <Kopie> HEAD`, `checkout FETCH_HEAD`: baut dieselbe SHA-1, Status leer, kein `[remote` in beiden `.git/config`; druckt `clone <Spielcode> dspre build_ms=… equal` |
| `text_project_in_git_builds_from_base_byte_equal` | Gate der Phase 4: Basis zuordnen, als Textprojekt einrichten (`adopt`), einrichten, Status leer; Index ⊇ `textproject::sources()`, sonst nur die Vorlagen und `expanded/`; `build::build` = SHA-1 der Basis-ROM, Status leer; `packed_kib` ≤ 10 MiB; Klon mit `core.autocrlf=true`, `needs_restore`, `restore_from_base`, Abgleich ohne Offenes, Build = SHA-1 der Basis-ROM, Status leer; ein zweiter Klon über `g4k text restore`; druckt `git <Spielcode> text files=… add_ms=… commit_ms=… packed_kib=…`, `clone <Spielcode> text restore_ms=… build_ms=… equal`, `restore_phases <Spielcode> extract=… merge=… import=… sync=…` und `text <Spielcode> adopt_ms=… moved=… members=… sync_files=…` |
| `trainer_and_text_changes_show_with_field_names_against_head_and_base` | Trainer 1 Level +1 und Nachricht 0 in Textarchiv 1 + „ Studio“, gespeichert: die Liste gegen `HEAD` nennt „Trainer 0001“ und „Textarchiv 0001“; der Diff zeigt `party[0].level` und die Wort-Spanne gegen `HEAD` und gegen die Basis mit denselben Zeilen, gegen gespeichert `same`; nach „Auf Original zurücksetzen“ und Speichern ist der Trainer gegen die Basis `same`; druckt `entries …` und `diff <Spielcode> status_ms=… entries_ms=… head_ms=… base_ms=…`; mit `G4_TIMING_STRICT=1` Status ≤ 500 ms |

Dazu `crates/g4-kernel/tests/diff.rs` `one_entity_diffs_fast_in_the_dspre_layout` (nach Änderungen an Trainer 1, Pokémon 25, Karte 0 und Header 0 sind diese gegen `HEAD` geändert, Attacke 1, Textarchiv 433 und Kartentextur 0 gleich; druckt je Diff `diff <Spielcode> <Name> <Gegen> state=… ms=…` und `open_for_diff_ms=… slowest_ms=…`; mit `G4_TIMING_STRICT=1` jeder Diff < 200 ms) und der Studio-Test `git_and_diff_over_the_command_path_on_the_vanilla_copy` in `apps/studio/src-tauri/src/commands/git.rs` („Git einrichten“, Status, Liste und Diff über den Befehlsweg der App; druckt `git init on the vanilla copy: …`, `status after init: … ms`, `diff against HEAD: … ms`).

```sh
G4_TEST_PROJECT="/<absoluter Pfad zu Roms>/_testkopien/4-4-pt_DSPRE_contents" \
  cargo test --release -p g4-kernel --all-features --test git -- --nocapture --test-threads 1
G4_TEST_PROJECT=… cargo test --release -p g4-kernel --all-features --test diff -- --nocapture --test-threads 1
G4_TEST_PROJECT=… cargo test --release -p studio --all-features --lib git_and_diff_over_the_command_path_on_the_vanilla_copy -- --nocapture --test-threads 1
```

Gemessen am 2026-10-29 im Gate (Release, `git version 2.54.0 (Apple Git-157)`, Last 3,6 bis 41,8): `--test git` 433 s
(Platin DE), 550 s (HeartGold DE), 414 s (HeartGold US), `--test diff` 31 bis 34 s, der Studio-Test 32 bis 36 s; gepackt
46 311 / 56 792 / 56 289 KiB (DSPRE) und 3 120 / 3 232 / 3 107 KiB (Textprojekt), Status 172 bis 215 ms, Klon des
Textprojekts vervollständigt in 43,7 bis 75,3 s (je nach Last), Build aus dem Klon SHA-1-gleich; Werte in
[docs/phase-4-4-gate.md](phase-4-4-gate.md). In `scripts/gate.sh` laufen sie im Schritt der gated Tests je Projekt
(Debug) mit, ohne Änderung am Skript.

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

Tiefen-Regel für `apps/studio/src` und der Gate-Schritt `depth` (`scripts/check-depth.sh`, seit 3d.2, gleiches `grep`-Prinzip, Exit 1 mit `Datei:Zeile [depth]` je Fund): die Tiefe (Erzählen, Bauen, Tüfteln) wird nie direkt mit einem Wert verglichen, sondern über `depthAtLeast(depth, min)` gefragt; was ein Bereich zeigt, sagt `viewDepth(area, depth)` (Erzählen nur in den Bereichen aus `NARRATING_AREAS`, sonst Bauen; alles in `ui/frame/shellState.ts`). Eine Zeile scheitert mit einem Namen auf `depth`/`Depth` vor `===`/`!==` und einer Zeichenkette, mit `===`/`!==` vor `"erzaehlen"` oder `"tuefteln"`, oder mit `case "erzaehlen"`/`case "tuefteln"`. `"bauen"` allein zählt nicht (auch der Technik-Eintrag „Bauen“ heißt so). Ausgenommen sind Testdateien und `ui/frame/shellState.ts`.

Die `.fig` pflegen: nach dem Speichern in OpenPencil `node design-system/scripts/fix_fig_text.mjs design-system/Editor_DesignSystem.fig --rgb555` (aus der Repo-Wurzel; repariert Text-Glyphen und rastet Farben auf RGB555), dann die Datei einchecken.

## Galerie

Die Galerie zeigt die Bausteine aus `apps/studio/src/ui/primitives/` mit ihren Zuständen, eine Seite je Gruppe (Tasten, Eingaben, Inhalt und Status, Schwebendes, Auswahl, Bildschirm mit dem DS-Bildschirm, Rahmen mit Scharnier im Assistenten-Modus, Startkarten, Palette und Reitern); oben schalten „Thema“ (Hell, Dunkel, System) und „Dichte“ (Komfort, Kompakt) um. Sie läuft nur im Browser-Modus, nie im Tauri-Fenster, und braucht kein Projekt: `http://127.0.0.1:1430/?gallery`, nachdem `scripts/agent/studio-web.sh` gestartet ist (oder mit `pnpm dev` in `apps/studio` auf dem Vite-Port). Es gibt keine automatischen Tests für die Galerie; der Agent macht für das Gate Screenshots in Hell, Dunkel und Kompakt und achtet auf unlesbare Stellen. Ein neuer Baustein bekommt eine Zeile auf einer Seite unter `ui/gallery/pages/` (`pages/index.ts` listet die Seiten).

## Gate-Skript

Es gibt ein privates Remote auf GitHub, aber keine automatische CI mehr: Gehostete Minuten kosten Geld, deshalb startet kein Push und kein Pull Request einen Lauf. `.github/workflows/ci.yml` fährt die hermetischen Schritte (ohne ROM, Spielstand und Werkzeuge, `G4_GATE_HERMETIC=1`, siehe unten) nur, wenn man es von Hand startet (`gh workflow run ci.yml`, mit `-f windows=true` auch unter Windows; siehe `docs/release.md`). Vor einem Merge läuft das volle `scripts/gate.sh` (Repo-Wurzel, bash) lokal; es ist die Prüfung und die Abnahme. Voraussetzung neben Rust und pnpm ist `uv` (der Schritt `g4emu unit` startet `uv run`; der erste Lauf lädt die Python-Pakete von PyPI). Es fährt der Reihe nach:

1. `cargo fmt --check`
2. `cargo clippy --workspace --all-targets --all-features -- -D warnings`
3. `cargo deny (licenses, advisories)`: `cargo deny --locked check licenses advisories` mit `deny.toml` (siehe „cargo-deny (Lizenzen und Advisories)“ unter „Tests“); fehlt `cargo-deny`, bricht das Skript mit der Zeile "FAILED (cargo-deny fehlt)" und dem Installationsbefehl ab; direkt danach `npm licenses`: `scripts/check-licenses-npm.sh` (die Lizenzen der Produktionspakete von `apps/studio` gegen die Freigabeliste, siehe „npm-Lizenzen“ unter „Tests“)
4. `cargo test --workspace --all-features` (ohne ROM-Daten: `G4_TEST_PROJECT`, `G4_TEST_ROM`, `G4_DSROM_CLI`, `G4_APICULA` und `G4_ROTOM_LSP` aus der aufrufenden Shell werden für diesen Schritt entfernt, die gated Tests überspringen sich)
5. `mcp`: `cargo test -p studio --test mcp_stdio` (das Produkt-Binary ohne Cargo-Features: `studio mcp` über stdio auf dem synthetischen Projekt, darunter Schreiben, Batch als ein Undo-Schritt, Unumkehrbares nur mit `--allow-irreversible` und der Test, dass `g4_call` ohne `--expert` fehlt; hermetisch wie der Schritt davor, dieselben `G4_*`-Variablen werden entfernt; rund 30 s bei warmem Cache, die Tests selbst 8 s; die gated Tests laufen je Projekt im letzten Schritt mit)
6. `pnpm test` in `apps/studio`
7. `tokens`: `scripts/check-tokens.sh` (Raster und Kontraste mit `check_tokens.py`, danach der Abgleich der eingecheckten Token-Dateien mit einem frischen Export; siehe „Design-System und Tokens“)
8. `styles`: `scripts/check-styles.sh` (keine Inline-Styles und keine Hex-Farben außerhalb der Ausnahmen; siehe „Design-System und Tokens“)
9. `depth`: `scripts/check-depth.sh` (kein direkter Vergleich der Tiefe, nur `depthAtLeast`/`viewDepth`; siehe „Design-System und Tokens“)
10. `pnpm build` in `apps/studio`, danach die Prüfung, dass `apps/studio/dist` den Text `__bridge` nicht enthält (die Dev-Brücke darf nicht ins Produktions-Bundle)
11. `g4emu unit`: `uv run pytest -m "not integration" -q` in `scripts/g4emu`
12. `g4emu integration`: `uv run pytest -m integration -q` in `scripts/g4emu` (Platin DE mit Spielstand bis ins Zimmer, Treppe und Header-ID im echten Emulator). Läuft nur, wenn der Kern (`tools/g4emu/core/melondsds_libretro.dylib` oder `G4EMU_CORE`, relative Pfade sind erlaubt), `$G4_ROMS_DIR/_testkopien/_saves/CPUD-zimmer.sav` (das Gate sucht Spielstände immer dort; `G4EMU_SAVES` wertet es nicht aus) und `$G4_ROMS_DIR/Pokemon - Platin-Edition (Germany).nds` existieren; sonst steht die Zeile "skipped (kein Kern)", "skipped (kein Spielstand)" bzw. "skipped (keine ROM)" in der Tabelle, ohne Abbruch
13. je Testprojekt unter `$G4_ROMS_DIR` (Muster `*_DSPRE_contents`) `cargo test --workspace --all-features` mit `G4_TEST_PROJECT`, `G4_APICULA`, `G4_DSROM_CLI` und `G4_ROTOM_LSP` (alle absolut), also die gated Tests aus dem Abschnitt "Gated Tests" gegen Platin DE, HeartGold DE und HeartGold US; ohne `build_graph_timings` (`--skip`, das misst Schritt 14 im Release-Profil), und `G4_TIMING_STRICT` aus der aufrufenden Shell wird für diesen Schritt entfernt
14. `build graph (timings)`: je Testprojekt `cargo test --release -p g4-kernel --all-features --test build_graph -- build_graph_timings --nocapture --test-threads 1` mit `G4_TEST_PROJECT` und `G4_TIMING_STRICT=1` (Zeitgrenzen von „Hier spielen“, siehe „Build-Graph (4.2)“), danach ebenso `--test refs -- vanilla_index_builds_under_a_second` (warmer Aufbau des Referenzindex ≤ 1 000 ms; im parallelen Lauf von Schritt 13 misst er nur, weil die anderen Tests dort dieselben Dateien lesen); scheitert der Lauf, wiederholt das Skript ihn einmal (Zeile „ok (2nd run)“), sonst bricht es ab
15. je `*.nds` unter `$G4_ROMS_DIR` (nur gelesen) `cargo test --workspace --all-features -- project_start_` mit `G4_TEST_ROM` (Projekt anlegen, bauen, SHA-1 vergleichen, siehe „Projekt aus ROM anlegen (3g.1)“); ohne `.nds` eine Zeile "skipped (keine ROM)"

Seit „Release und CI“ stehen zwischen Schritt 3 und 4 vier weitere hermetische Schritte (ohne Nummer, damit die Verweise auf die Schritte 13 bis 15 stimmen): `script tests` (`scripts/tests/run.sh`, die Tests der Shell-Skripte), `actions` (`scripts/check-actions.sh`: jede `uses:`-Zeile auf einen SHA festgelegt und in `.github/actions-allowlist.txt`, Lizenz aus der Liste von `deny.toml`), `no rom data` (`scripts/check-no-rom-data.sh`: keine ROM-, Spielstand- oder Patch-Daten unter `git ls-files`) und `version` (`scripts/version.sh check`). Die Fassungen der Werkzeuge (apicula, rotom, cargo-deny, pnpm, Node, cargo-about) stehen in `scripts/tool-versions.env`; CI, Release-Workflow und Skripte lesen sie dort. Mit `G4_GATE_HERMETIC=1` endet das Gate nach Schritt 11 (`g4emu unit`): das ist genau, was `gate-macos` in `ci.yml` fährt (nur von Hand gestartet). `gate-windows` fährt nicht das Gate, sondern nur `cargo clippy --workspace --all-targets --all-features -- -D warnings` und `cargo test --workspace --all-features --no-fail-fast`.

```sh
scripts/gate.sh
```

Das Skript findet die Rust-Toolchain selbst (`~/.rustup/toolchains/stable-aarch64-apple-darwin/bin`, falls vorhanden; sonst gilt der `PATH` der Shell) und hängt `~/.cargo/bin` (dort liegt `cargo-deny`) hinten an den `PATH`, braucht also keinen `export PATH`. Variablen, alle optional:

| Variable | Standard | Bedeutung |
|---|---|---|
| `G4_GATE_HERMETIC` | nicht gesetzt | `1`: das Gate endet nach dem Schritt `g4emu unit` und die Tabelle schließt mit „Gate (hermetisch): ok“; das fährt die CI (kein `G4_*` sonst nötig). Nicht gesetzt: das volle lokale Gate |
| `G4_ROMS_DIR` | `../Roms` (neben dem Repository) | Ordner mit den `*_DSPRE_contents`-Projekten; aus einem Git-Worktree absolut setzen, der Standard trifft dort nichts |
| `G4_APICULA` | `tools/apicula/bin/apicula` | apicula-Binärdatei für die Modell-Tests |
| `G4_DSROM_CLI` | `$G4_ROMS_DIR/_tools/dsrom` | ds-rom-Referenz-CLI für den Rebuild-Vergleich |
| `G4EMU_CORE` | `tools/g4emu/core/melondsds_libretro.dylib` | Emulator-Kern für den Schritt `g4emu integration` |
| `G4_ROTOM_LSP` | `tools/rotom/bin/rotom-lsp` | rotom-lsp-Binärdatei (Preflight; wird an die gated Tests durchgereicht) |

Das Skript hält beim ersten Fehler und druckt am Ende eine Tabelle Schritt / Projekt / Ergebnis / Sekunden, auch bei Abbruch; der Exit-Code ist ungleich 0, sobald ein Schritt fehlschlägt. Ein Ordner ohne `header.yaml` zählt nicht als Projekt und erscheint als Zeile "skipped (no header.yaml)". Fehlen `G4_ROMS_DIR`, `G4_APICULA`, `G4_DSROM_CLI` oder `G4_ROTOM_LSP`, oder findet es kein Projekt, bricht es mit einer Meldung ab (sonst würden die gated Tests nur "skipped" melden und trotzdem als bestanden gelten).

Dauer (der Schritt `mcp` kommt seit 3h.2 mit rund 30 Sekunden dazu): bei warmem Build-Cache etwa 30 bis 40 Minuten (gemessen 1704 s in 3c, 1781 s in 3b.3a, 1798 s in 3b.3b, 2452 s in 3b.4a; vor 3b.2 waren es 1217 s, dazu rund eine Minute für die g4emu-Schritte; seit 3b.2 kommen die gated Script- und Text-Tests dazu, seit 3b.3a die Daten-Tests aus „Daten-Archive und das uxie-Orakel“; seit 3b.3b die Trainer- und Begegnungs-Tests aus „Trainer und Begegnungen (3b.3b)“, seit 3b.4a die Prüf-Tests aus „Projektprüfungen (3b.4a)“; gemessene Läufe stehen in [docs/phase-3c-gate.md](phase-3c-gate.md) (1704 s), [docs/phase-3b3-gate.md](phase-3b3-gate.md) (Teil A 1781 s, Teil B 1798 s) und [docs/phase-3b4-gate.md](phase-3b4-gate.md) (Teil A 2452 s); die drei Projekte brauchen 380 bis 830 s je Projekt, die Schritte davor zusammen unter einer Minute); ein kalter Build kommt obendrauf. Die Projekte unter `$G4_ROMS_DIR` werden nur gelesen: schreibende Tests arbeiten auf einer Kopie im Temp-Verzeichnis (`vanilla_copy()`), und `_testkopien` fasst das Skript nicht an.

## Agent-Testumgebung

Ein Claude-Agent bedient die App und das Spiel selbst; die Anleitung dafür steht in [docs/agent-gate.md](agent-gate.md). Die Bausteine:

- **Brücke `studio-bridge`** (Cargo-Feature `dev-bridge`, nur im Dev-Build, nie im App-Bundle): ein kleiner HTTP-Server auf `127.0.0.1:1431`, der alle Tauri-Befehle der App aufruft. Seit 3h.1 ist die Befehlsschicht `dispatch` (`commands/dispatch.rs`, 122 Befehle) Produktcode ohne Feature; die Brücke ist nur noch der HTTP-Rahmen darum. Welche Pfade ein Aufruf berühren darf, entscheidet ein `Policy` (`commands/policy.rs`): `DevPolicy` (die frühere Struktur `Roots` in `bridge/roots.rs`, Feature `dev-bridge`) für die Brücke, `UserPolicy` für `studio mcp` (siehe „MCP (`studio mcp`)“). Die Brücke nimmt nur Projekte und Build-Ziele unter `$G4_ROMS_DIR/_testkopien` an (nie unter `*_DSPRE_contents`) und verlangt ein Token (`X-G4-Bridge-Token`, steht in `target/agent/bridge.token`). `scripts/agent/studio-web.sh` startet Brücke und Vite (`127.0.0.1:1430`); `.claude/launch.json` kennt sie als `studio-web`. Solange die Brücke läuft, erreicht jeder lokale Nutzer sie über den Vite-Proxy auf `127.0.0.1:1430` (das Token liegt für den Proxy bereit); angenommen ist eine Entwicklungsmaschine mit einem Nutzer. Die Seite läuft dann im Browser-Fenster der Claude-Desktop-App unter `http://127.0.0.1:1430/?project=<Kopie>` und zeigt "Browser-Modus (Brücke)" in der Statusleiste.
- **`scripts/agent/call.sh <befehl> ['<json>']`** ruft einen Befehl der laufenden Brücke auf; `studio-bridge batch <datei.json> --allow-root <dir>` führt eine Liste von Befehlen ohne Server aus.
- **`g4emu`** (`scripts/g4emu`, Python mit `uv`): ein Emulator ohne Fenster (libretro.py mit dem melonDS-DS-Kern 1.4.0). Einrichten mit `bash scripts/g4emu/setup.sh` (lädt den Kern aus dem festen GitHub-Release, SHA-256 geprüft, nach `tools/g4emu/core/`). Aufruf: `uv run g4emu run --rom … --save … --out … <Schritte>` (`continue`, `walk-to`, `expect-map`, `where`, `shot` …), `uv run g4emu scenario <yaml> --project <Kopie> --set NAME=WERT …` (Projekt ändern, bauen, spielen; Beispiele in `scripts/agent/scenarios/`; eine Änderung kann die Antwort der Brücke an einen Namen binden, den spätere Schritte als `${name}` nutzen, und seit 3b.4a Erwartungen an die Antwort stellen, `{expect: {path, equals}}`, verfehlt mit Rückgabe 2; seit 3b.4b auch erwartete Ablehnungen `{expect: {error: …}}`, Dateihashes `${sha1-of:PFAD}` und den Abschnitt `prepare:`, siehe „Kartenteile importieren (3b.4b)“), `uv run g4emu find-u16` zum Suchen von Adressen. Seit 3b.3b kennt `g4emu` Kampfschritte (`expect-battle`, `walk-until-battle`, `end-battle`, `win-battle`, `mash`): ein Kampf wird an den BattleMon-Datensätzen im RAM erkannt, die Werte je Spielcode stehen in `scripts/g4emu/g4emu/games.yaml` (`battle`, `battle_pointer`, `home_header`; IPKE ohne gemessenen Zeiger); Beschreibung in [docs/agent-gate.md](agent-gate.md), „Kämpfe“. Die Tests: `uv run pytest -m "not integration"` (Einheit) und `-m integration` (echter Kern). Seit 3d.9 gibt es `uv run g4emu repl --rom … --save … --out … --gdb-port P`: das Spiel läuft weiter, jede Zeile auf stdin ist genau ein Schritt (Antwort als JSON-Zeile, `quit` beendet, `expect-halted`/`expect-running` fragen den Stub), und die erste Zeile `{"ready": true, "gdb_port": N, "game": …}` nennt den Port. Auf diesem Port lauscht ein GDB-Stub nach dem Vorbild von melonDS 1.1 (`g4emu/gdbstub.py`: ein Client, Halt beim Verbinden bis `c`, `\x03`, `m`/`M` auf den 4 MiB Arbeitsspeicher, `D`), sodass `g4k live` und der Spielmodus ohne melonDS gegen das laufende Spiel testbar sind. Der gated Test `tests/test_live.py` (`-m integration -k live`, Port 3390) nimmt die ROMs aus `G4_LIVE_ROM_CPUD`, `G4_LIVE_ROM_IPKD` und `G4_LIVE_ROM_IPKE` und `g4k` aus `G4K_BIN` (sonst `cargo build -p g4-kernel --bin g4k`).
- **Spielmodus live (3d.9)** liest und schreibt den Spielstand im Arbeitsspeicher des laufenden Spiels über den GDB-Stub von **melonDS 1.1** oder neuer (Emu settings › Devtools; ältere Fassungen kennen ihn nicht, die App meldet dann „melonDS 1.1 oder neuer nötig“). Der Stub lauscht nur mit `[Instance0.Gdb] Enabled = true` in `~/Library/Preferences/melonDS/melonDS.toml`, bedient genau einen Client (Port 3333 für den ARM9, 3334 für den ARM7; Studio nutzt nur 3333); „Hier spielen“ schaltet ihn ein, sichert die Datei vorher als `melonDS.toml.g4-backup` und ändert sie nur, wenn kein melonDS läuft (melonDS schreibt sie beim Beenden zurück). Der **JIT** muss aus sein (`[JIT] Enable = false`), sonst arbeitet der Stub nicht, und `BreakOnStartup` muss aus sein, damit das Spiel beim Start nicht anhält; die App stellt beides mit dem Stub ein, und die Zeile „Spielmodus live“ in Technik › Emulator zeigt, was fehlt. Ohne melonDS läuft dasselbe gegen den Stub von `g4emu repl` (Port 3390, nie 3333 anfassen, solange ein melonDS des Nutzers laufen könnte); Starten und Einstellen von melonDS gibt es nur unter macOS; Windows und Linux starten das Spiel wie bisher ohne Live-Werte.
- **Spielstände** liegen außerhalb des Repos in `$G4_ROMS_DIR/_testkopien/_saves/<SPIELCODE>-zimmer.sav` (CPUD Platin DE, IPKD HeartGold DE, IPKE HeartGold US) mit einer `.md`-Datei, die den Weg dorthin beschreibt. `g4emu` kopiert sie vor jedem Lauf und schreibt nie zurück.
- **Ports:** Vite 1430, Brücke 1431; `G4_VITE_PORT` und `G4_BRIDGE_PORT` wählen andere (für eine zweite Instanz aus einem Worktree; `call.sh` liest den Brückenport aus `target/agent/bridge.port`). Die eigene App (`pnpm tauri dev`) bleibt auf 1420/1421.
- **Umgebungsvariablen:** `G4_ROMS_DIR` (Ordner `Roms`, Standard `../Roms`), `G4_APICULA`, `G4EMU_CORE`, `G4EMU_SAVES` (anderer Spielstand-Ordner), `G4_BRIDGE_BIN` und `G4K_BIN` (fertiges `studio-bridge` bzw. `g4k` für Szenarien; aus einem Worktree mit `cargo build --workspace --all-features` bauen, `cargo test --no-run` erneuert `studio-bridge` nicht immer). In einem Git-Worktree fehlt `tools/` (nicht versioniert): `tools/apicula` verlinken oder `G4_APICULA` setzen. Dort stimmt auch der Standard von `G4_ROMS_DIR` nicht (`../Roms` liegt neben dem Haupt-Checkout): `scripts/agent/studio-web.sh` und `scripts/gate.sh` brauchen aus einem Worktree `G4_ROMS_DIR=<absoluter Pfad zu Roms>`.

## MCP (`studio mcp`)

Seit 3h.1 gibt es einen MCP-Server für Coding-Agenten (seit 3h.2 liest und bearbeitet er kopflos), in der Programmdatei der App selbst: `studio mcp` (Unterbefehl vor dem Start von Tauri, `src/main.rs` ruft `studio_lib::mcp::main`). Er ist Produktcode ohne Cargo-Feature, nutzt dieselbe Befehlsschicht `dispatch` wie die Brücke und das Protokoll über stdio mit `rmcp` (3.7.0). Für Nutzer steht die Anleitung in [docs/mcp.md](mcp.md); hier steht, was Entwickler brauchen. Module in `apps/studio/src-tauri/src/mcp/`: `mod.rs` (Kommandozeile, Schutz von stdout), `catalog.rs` (die 41 Werkzeuge mit Schemas, Klasse je Befehl in `access.rs`), `tools.rs` (`Toolbox`: Aufruf, `g4_batch`, Ressourcen), `server.rs` (Abbildung auf rmcp), dazu `commands/policy.rs` (`UserPolicy`).

**Bauen und Aufrufen:**

```sh
cargo build -p studio                                   # target/debug/studio (kein Feature nötig)
target/debug/studio mcp --project <Kopie>               # wartet auf JSON-RPC (eine Nachricht je Zeile) auf stdin
target/debug/studio mcp --help
```

Schalter: `--project <DIR>` (Pflicht), `--allow-dir <DIR>` (wiederholbar), `--read-only`, `--allow-irreversible` (nicht mit `--read-only`), `--emulator <PATH>` (für `run_rom`), `--expert` (`g4_call`, `g4_commands`). Der Zugang ist `{write: !read_only, irreversible: allow_irreversible, expert}`; die Policy ist `UserPolicy::new(project, allow_dirs, emulator)`. Beim Schließen von stdin wartet der Server auf laufende Aufrufe (`server::DRAIN_LIMIT`, 600 s); in Debug-Builds hält `G4_MCP_TEST_DELAY_MS=<n>` jeden Aufruf `n` ms auf seinem Worker, damit `closing_stdin_waits_for_a_running_write` das prüfen kann. Diese beiden Haken (`G4_MCP_TEST_DELAY_MS`, `G4_MCP_TEST_NOISE`) gibt es nur mit `debug_assertions`; die Tests, die sie brauchen (die zwei Tests zum Schließen von stdin und `stray_stdout_does_not_break_the_protocol`), sind mit `#[cfg(debug_assertions)]` ausgenommen, in einem `--release`-Lauf von `mcp_stdio` laufen 22 statt 25 Tests. Rückgabe 0, 1 (Fehler im Protokoll) oder 2 (nichts bedient: falsche Argumente, Projekt nicht lesbar, Original gesperrt; Meldung auf stderr). stdout gehört dem Protokoll: unter Unix lenkt der Server fd 1 auf stderr um (`libc::fcntl` mit `F_DUPFD_CLOEXEC`, `dup2`) und schreibt das Protokoll auf die Kopie; unter Windows gibt es das nicht. In Debug-Builds druckt `G4_MCP_TEST_NOISE=1` eine Zeile auf stdout (landet auf stderr), damit Tests den Schutz beweisen können.

**Schreiben:** Der Standard ist lesen und bearbeiten (`--read-only` schaltet es ab). Die Klasse jedes Befehls (lesend, schreibend, unumkehrbar, abhängig vom Argument wie bei `build_rom` und `export_map_part`) steht in `mcp/access.rs`; `admit` gibt `read_only` oder `needs_confirmation`. Unumkehrbare Werkzeuge bleiben in `tools/list` und antworten ohne `--allow-irreversible` mit `needs_confirmation`. `g4_batch` prüft alle Aufrufe vorab, führt sie in einer Sitzung aus und fasst sie mit `squash_since` zu einem Undo-Schritt zusammen; ein `RollbackGuard` nimmt den Batch auch bei einer Panik zurück (Einzelheiten in [docs/mcp.md](mcp.md), „Batch“). Zum Ausprobieren auf einer Kopie: `target/debug/studio mcp --project "$G4_ROMS_DIR/_testkopien/<name>"` und mit `--allow-irreversible` für `create_trainer` oder `apply_header_patch`.

**Szenarien über MCP:** `scripts/g4emu` spielt Szenarien auch über `studio mcp` (YAML-Schlüssel `mcp:`, siehe [docs/agent-gate.md](agent-gate.md), „Szenarien über MCP“). Der Client wählt das Programm so: ist `G4_STUDIO_BIN` gesetzt, gilt es als Pfad der Programmdatei `studio`; sonst baut er mit `cargo build -p studio --bin studio` und nimmt `target/debug/studio`. Ein vorgebautes Binary spart den Bau, etwa `G4_STUDIO_BIN=$PWD/target/debug/studio uv run g4emu scenario …`.

**Originale in der Entwicklung:** Ist `G4_ROMS_DIR` gesetzt, lehnt `UserPolicy` ein Projekt (und jeden `--allow-dir`-Ordner) ab, das unter `$G4_ROMS_DIR` liegt, aber nicht unter `$G4_ROMS_DIR/_testkopien`, dazu alles unter `_testkopien/_saves`: Rückgabe 2, „Originale sind gesperrt, nur Kopien unter _testkopien“. Ebenso abgelehnt ist ein `--allow-dir`-Ordner (oder Projekt), der die Originale enthält, also `$G4_ROMS_DIR` selbst oder ein Ordner darüber („Originale sind gesperrt, der Ordner enthält G4_ROMS_DIR“). Jeder Pfad, den ein Befehl liest oder schreibt, wird zusätzlich einzeln gegen dieselbe Regel geprüft. Lässt sich `G4_ROMS_DIR` nicht auflösen (Ordner fehlt), startet `studio mcp` nicht: „G4_ROMS_DIR ist gesetzt, aber nicht lesbar: …“, Rückgabe 2. Die Namenssperre `*_DSPRE_contents` der Brücke gilt für Nutzer nicht (3g legt Nutzerprojekte so an). Also immer auf eine Kopie zeigen:

```sh
cp -R "$G4_ROMS_DIR/Pokemon - Platin-Edition (Germany)_DSPRE_contents" "$G4_ROMS_DIR/_testkopien/<name>"   # Ziel darf nicht existieren
```

**Tests:**

```sh
cargo test -p studio --all-features                     # Bibliothek (350 seit 3h.2) und tests/mcp_stdio.rs (25, davon 3 gated)
cargo test -p studio --test mcp_stdio                   # nur der Client-Test über stdio (der Gate-Schritt `mcp`), rund 8 s
G4_TEST_PROJECT="$G4_ROMS_DIR/<Projekt>_DSPRE_contents" \
  cargo test -p studio --test mcp_stdio vanilla          # gated: Header, Prüfungen, Texte auf vanilla_copy(), rund 80 s
G4_TEST_PROJECT=… G4_DSROM_CLI=… cargo test -p studio vanilla_build_rom_defaults_into_build   # gated, rund 85 s
```

`tests/mcp_stdio.rs` startet `CARGO_BIN_EXE_studio mcp` mit einem rmcp-Client (`TokioChildProcess`) auf dem synthetischen Projekt (`synthetic_project()`); der gated Test läuft auf einer Kopie im Temp-Verzeichnis (`vanilla_copy()`), das Original wird nur gelesen. Der rmcp-Client überspringt Zeilen auf stdout, die kein JSON sind; wer den Schutz von stdout prüft, nimmt eine rohe Zeile (`initialize_line_with_2025_11_25`).

**Handprobe** (ohne MCP-Client, mit Protokollversion 2025-11-25, wie ein Python-Client sie spricht): ein kleines Skript im Scratchpad startet `studio mcp --project <Kopie>` mit `subprocess.Popen` und schickt Zeilen:

```python
import json, subprocess
p = subprocess.Popen(["target/debug/studio", "mcp", "--project", KOPIE], stdin=subprocess.PIPE,
                     stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True)
def rpc(method, params=None, id=None):
    msg = {"jsonrpc": "2.0", "method": method, **({"params": params} if params is not None else {}), **({"id": id} if id is not None else {})}
    p.stdin.write(json.dumps(msg) + "\n"); p.stdin.flush()
    return json.loads(p.stdout.readline()) if id is not None else None
print(rpc("initialize", {"protocolVersion": "2025-11-25", "capabilities": {}, "clientInfo": {"name": "probe", "version": "0"}}, 1)["result"]["serverInfo"])
rpc("notifications/initialized")
print([t["name"] for t in rpc("tools/list", {}, 2)["result"]["tools"]])
print(rpc("tools/call", {"name": "get_header", "arguments": {"id": 0}}, 3)["result"]["structuredContent"])
p.stdin.close()
```

Eine Antwort je Anfrage kommt nicht unbedingt in der Reihenfolge der Anfragen (der Server arbeitet nebenläufig); bei mehreren offenen Anfragen nach der `id` zuordnen. Ein Befehlsfehler ist `result.isError: true`, ein unbekanntes Werkzeug eine JSON-RPC-Antwort mit `error` (-32602). Die Handprobe vom 2026-10-25 auf `_testkopien/3h-lesen` steht im Gate-Dokument [docs/phase-3h-gate.md](phase-3h-gate.md). Claude Code selbst einzurichten ist der Hand-Punkt des Gates: `claude mcp add gen4-studio -- <repo>/target/debug/studio mcp --project <Kopie>` (siehe [docs/mcp.md](mcp.md)); der Agent ändert die Claude-Konfiguration des Nutzers nicht.

**Gate-Skript:** `scripts/gate.sh` hat seit 3h.2 den Schritt `mcp` (`cargo test -p studio --test mcp_stdio`, direkt nach `cargo test` ohne ROM-Daten); die gated Tests (`vanilla_*`) laufen je Projekt im letzten Schritt mit. Szenarien laufen nicht im Gate-Skript, ihr Lauf und die Bilder stehen im Gate-Dokument ([docs/phase-3h-gate.md](phase-3h-gate.md)).

## App-Bundle

Ein lauffähiges `.app` (mit `libnitroarc_ffi.dylib` und den Sidecars apicula und rotom-lsp) entsteht mit:

```sh
cd apps/studio
pnpm bundle                      # prepare-bundle.sh, danach tauri build (über scripts/with-rust.sh, findet cargo selbst)
bash scripts/check-bundle.sh     # Ausgabe: bundle ok
```

Voraussetzungen, Ablauf, Prüfbefehle und bekannte Grenzen (nicht signiert, Gatekeeper) stehen in [docs/release.md](release.md). Der Rauchtest der gepackten App (Script öffnen, Completion, Fehler beim Speichern) steht im Gate-Dokument [docs/phase-3b2-gate.md](phase-3b2-gate.md), Teil (H).
`pnpm tauri dev` und normale `cargo`-Builds brauchen das nicht.
