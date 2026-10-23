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
byte-identische ROMs. Bei Änderungen an Build- oder Pack-Code ist dieser Lauf Pflicht.

Messlauf, nicht Teil des normalen Durchlaufs (`#[ignore]`): `G4_APICULA=... cargo test -p studio --release -- --ignored
--nocapture bench_matrix_models` lädt eine große Matrix (Standard: Header 3, Platin Matrix 0) und gibt die Zeiten aus;
`G4_BENCH_HEADER` wählt einen anderen Header.

## Gate-Skript

Es gibt kein Remote und keine CI; `scripts/gate.sh` (Repo-Wurzel, bash) ersetzt sie und ist der Lauf vor einem Merge. Es fährt der Reihe nach:

1. `cargo fmt --check`
2. `cargo clippy --workspace --all-targets --all-features -- -D warnings`
3. `cargo test --workspace --all-features` (ohne ROM-Daten: `G4_TEST_PROJECT`, `G4_TEST_ROM`, `G4_DSROM_CLI` und `G4_APICULA` und `G4_ROTOM_LSP` aus der aufrufenden Shell werden für diesen Schritt entfernt, die gated Tests überspringen sich)
4. `pnpm test` und `pnpm build` in `apps/studio`
5. je Testprojekt unter `$G4_ROMS_DIR` (Muster `*_DSPRE_contents`) `cargo test --workspace --all-features` mit `G4_TEST_PROJECT`, `G4_APICULA`, `G4_DSROM_CLI` und `G4_ROTOM_LSP` (alle absolut), also die gated Tests aus dem Abschnitt "Gated Tests" gegen Platin DE, HeartGold DE und HeartGold US

```sh
scripts/gate.sh
```

Das Skript findet die Rust-Toolchain selbst (`~/.rustup/toolchains/stable-aarch64-apple-darwin/bin`, falls vorhanden; sonst gilt der `PATH` der Shell), braucht also keinen `export PATH`. Variablen, alle optional:

| Variable | Standard | Bedeutung |
|---|---|---|
| `G4_ROMS_DIR` | `../Roms` (neben dem Repository) | Ordner mit den `*_DSPRE_contents`-Projekten |
| `G4_APICULA` | `tools/apicula/bin/apicula` | apicula-Binärdatei für die Modell-Tests |
| `G4_DSROM_CLI` | `$G4_ROMS_DIR/_tools/dsrom` | ds-rom-Referenz-CLI für den Rebuild-Vergleich |
| `G4_ROTOM_LSP` | `tools/rotom/bin/rotom-lsp` | rotom-lsp-Binärdatei (Preflight; wird an die gated Tests durchgereicht) |

Das Skript hält beim ersten Fehler und druckt am Ende eine Tabelle Schritt / Projekt / Ergebnis / Sekunden, auch bei Abbruch; der Exit-Code ist ungleich 0, sobald ein Schritt fehlschlägt. Ein Ordner ohne `header.yaml` zählt nicht als Projekt und erscheint als Zeile "skipped (no header.yaml)". Fehlen `G4_ROMS_DIR`, `G4_APICULA`, `G4_DSROM_CLI` oder `G4_ROTOM_LSP`, oder findet es kein Projekt, bricht es mit einer Meldung ab (sonst würden die gated Tests nur "skipped" melden und trotzdem als bestanden gelten).

Dauer: bei warmem Build-Cache etwa 20 bis 25 Minuten (zuletzt vor 3b.2 1217 s; seit 3b.2 kommen die gated Script- und Text-Tests dazu, der aktuelle Lauf steht in [docs/phase-3b2-gate.md](phase-3b2-gate.md); die drei Projekte brauchen 300 bis 450 s je Projekt, die Schritte davor zusammen unter einer Minute); ein kalter Build kommt obendrauf. Die Projekte unter `$G4_ROMS_DIR` werden nur gelesen: schreibende Tests arbeiten auf einer Kopie im Temp-Verzeichnis (`vanilla_copy()`), und `_testkopien` fasst das Skript nicht an.

## App-Bundle

Ein lauffähiges `.app` (mit `libnitroarc_ffi.dylib` und den Sidecars apicula und rotom-lsp) entsteht mit:

```sh
cd apps/studio
pnpm bundle                      # prepare-bundle.sh, danach tauri build
bash scripts/check-bundle.sh     # Ausgabe: bundle ok
```

Voraussetzungen, Ablauf, Prüfbefehle und bekannte Grenzen (nicht signiert, Gatekeeper) stehen in [docs/release.md](release.md). Der Rauchtest der gepackten App (Script öffnen, Completion, Fehler beim Speichern) steht im Gate-Dokument [docs/phase-3b2-gate.md](phase-3b2-gate.md), Teil (H).
`pnpm tauri dev` und normale `cargo`-Builds brauchen das nicht.
