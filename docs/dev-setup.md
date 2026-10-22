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

Es gibt kein Remote und keine CI; `scripts/gate.sh` (Repo-Wurzel, bash) ersetzt sie und ist der Lauf vor einem Merge. Voraussetzung neben Rust und pnpm ist `uv` (der Schritt `g4emu unit` startet `uv run`; der erste Lauf lädt die Python-Pakete von PyPI). Es fährt der Reihe nach:

1. `cargo fmt --check`
2. `cargo clippy --workspace --all-targets --all-features -- -D warnings`
3. `cargo test --workspace --all-features` (ohne ROM-Daten: `G4_TEST_PROJECT`, `G4_TEST_ROM`, `G4_DSROM_CLI` und `G4_APICULA` aus der aufrufenden Shell werden für diesen Schritt entfernt, die gated Tests überspringen sich)
4. `pnpm test` und `pnpm build` in `apps/studio`, danach die Prüfung, dass `apps/studio/dist` den Text `__bridge` nicht enthält (die Dev-Brücke darf nicht ins Produktions-Bundle)
5. `g4emu unit`: `uv run pytest -m "not integration" -q` in `scripts/g4emu`
6. `g4emu integration`: `uv run pytest -m integration -q` in `scripts/g4emu` (Platin DE mit Spielstand bis ins Zimmer, Treppe und Header-ID im echten Emulator). Läuft nur, wenn der Kern (`tools/g4emu/core/melondsds_libretro.dylib` oder `G4EMU_CORE`, relative Pfade sind erlaubt), `$G4_ROMS_DIR/_testkopien/_saves/CPUD-zimmer.sav` (das Gate sucht Spielstände immer dort; `G4EMU_SAVES` wertet es nicht aus) und `$G4_ROMS_DIR/Pokemon - Platin-Edition (Germany).nds` existieren; sonst steht die Zeile "skipped (kein Kern)", "skipped (kein Spielstand)" bzw. "skipped (keine ROM)" in der Tabelle, ohne Abbruch
7. je Testprojekt unter `$G4_ROMS_DIR` (Muster `*_DSPRE_contents`) `cargo test --workspace --all-features` mit `G4_TEST_PROJECT`, `G4_APICULA` und `G4_DSROM_CLI` (alle absolut), also die gated Tests aus dem Abschnitt "Gated Tests" gegen Platin DE, HeartGold DE und HeartGold US

```sh
scripts/gate.sh
```

Das Skript findet die Rust-Toolchain selbst (`~/.rustup/toolchains/stable-aarch64-apple-darwin/bin`, falls vorhanden; sonst gilt der `PATH` der Shell), braucht also keinen `export PATH`. Variablen, alle optional:

| Variable | Standard | Bedeutung |
|---|---|---|
| `G4_ROMS_DIR` | `../Roms` (neben dem Repository) | Ordner mit den `*_DSPRE_contents`-Projekten |
| `G4_APICULA` | `tools/apicula/bin/apicula` | apicula-Binärdatei für die Modell-Tests |
| `G4_DSROM_CLI` | `$G4_ROMS_DIR/_tools/dsrom` | ds-rom-Referenz-CLI für den Rebuild-Vergleich |
| `G4EMU_CORE` | `tools/g4emu/core/melondsds_libretro.dylib` | Emulator-Kern für den Schritt `g4emu integration` |

Das Skript hält beim ersten Fehler und druckt am Ende eine Tabelle Schritt / Projekt / Ergebnis / Sekunden, auch bei Abbruch; der Exit-Code ist ungleich 0, sobald ein Schritt fehlschlägt. Ein Ordner ohne `header.yaml` zählt nicht als Projekt und erscheint als Zeile "skipped (no header.yaml)". Fehlen `G4_ROMS_DIR`, `G4_APICULA` oder `G4_DSROM_CLI`, oder findet es kein Projekt, bricht es mit einer Meldung ab (sonst würden die gated Tests nur "skipped" melden und trotzdem als bestanden gelten).

Dauer: bei warmem Build-Cache etwa 20 Minuten (zuletzt 1217 s, vor den g4emu-Schritten, die rund eine Minute dazukommen; die drei Projekte brauchen 300 bis 450 s je Projekt, die Schritte davor zusammen unter einer Minute); ein kalter Build kommt obendrauf. Die Projekte unter `$G4_ROMS_DIR` werden nur gelesen: schreibende Tests arbeiten auf einer Kopie im Temp-Verzeichnis (`vanilla_copy()`), und `_testkopien` fasst das Skript nicht an.

## Agent-Testumgebung

Ein Claude-Agent bedient die App und das Spiel selbst; die Anleitung dafür steht in [docs/agent-gate.md](agent-gate.md). Die Bausteine:

- **Brücke `studio-bridge`** (Cargo-Feature `dev-bridge`, nur im Dev-Build, nie im App-Bundle): ein kleiner HTTP-Server auf `127.0.0.1:1431`, der alle Tauri-Befehle der App aufruft. Er nimmt nur Projekte und Build-Ziele unter `$G4_ROMS_DIR/_testkopien` an (nie unter `*_DSPRE_contents`) und verlangt ein Token (`X-G4-Bridge-Token`, steht in `target/agent/bridge.token`). `scripts/agent/studio-web.sh` startet Brücke und Vite (`127.0.0.1:1430`); `.claude/launch.json` kennt sie als `studio-web`. Solange die Brücke läuft, erreicht jeder lokale Nutzer sie über den Vite-Proxy auf `127.0.0.1:1430` (das Token liegt für den Proxy bereit); angenommen ist eine Entwicklungsmaschine mit einem Nutzer. Die Seite läuft dann im Browser-Fenster der Claude-Desktop-App unter `http://127.0.0.1:1430/?project=<Kopie>` und zeigt "Browser-Modus (Brücke)" in der Statusleiste.
- **`scripts/agent/call.sh <befehl> ['<json>']`** ruft einen Befehl der laufenden Brücke auf; `studio-bridge batch <datei.json> --allow-root <dir>` führt eine Liste von Befehlen ohne Server aus.
- **`g4emu`** (`scripts/g4emu`, Python mit `uv`): ein Emulator ohne Fenster (libretro.py mit dem melonDS-DS-Kern 1.4.0). Einrichten mit `bash scripts/g4emu/setup.sh` (lädt den Kern aus dem festen GitHub-Release, SHA-256 geprüft, nach `tools/g4emu/core/`). Aufruf: `uv run g4emu run --rom … --save … --out … <Schritte>` (`continue`, `walk-to`, `expect-map`, `where`, `shot` …), `uv run g4emu scenario <yaml> --project <Kopie> --set NAME=WERT …` (Projekt ändern, bauen, spielen; Beispiele in `scripts/agent/scenarios/`), `uv run g4emu find-u16` zum Suchen von Adressen. Die Tests: `uv run pytest -m "not integration"` (Einheit) und `-m integration` (echter Kern).
- **Spielstände** liegen außerhalb des Repos in `$G4_ROMS_DIR/_testkopien/_saves/<SPIELCODE>-zimmer.sav` (CPUD Platin DE, IPKD HeartGold DE, IPKE HeartGold US) mit einer `.md`-Datei, die den Weg dorthin beschreibt. `g4emu` kopiert sie vor jedem Lauf und schreibt nie zurück.
- **Ports:** Vite 1430, Brücke 1431. Die eigene App (`pnpm tauri dev`) bleibt auf 1420/1421.
- **Umgebungsvariablen:** `G4_ROMS_DIR` (Ordner `Roms`, Standard `../Roms`), `G4_APICULA`, `G4EMU_CORE`, `G4EMU_SAVES` (anderer Spielstand-Ordner), `G4_BRIDGE_BIN` (fertiges `studio-bridge` für Szenarien). In einem Git-Worktree fehlt `tools/` (nicht versioniert): `tools/apicula` verlinken oder `G4_APICULA` setzen.

## App-Bundle

Ein lauffähiges `.app` (mit `libnitroarc_ffi.dylib` und dem apicula-Sidecar) entsteht mit:

```sh
cd apps/studio
pnpm bundle                      # prepare-bundle.sh, danach tauri build
bash scripts/check-bundle.sh     # Ausgabe: bundle ok
```

Voraussetzungen, Ablauf, Prüfbefehle und bekannte Grenzen (nicht signiert, Gatekeeper) stehen in [docs/release.md](release.md).
`pnpm tauri dev` und normale `cargo`-Builds brauchen das nicht.
