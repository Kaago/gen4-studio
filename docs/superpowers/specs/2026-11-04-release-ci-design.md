# Release und CI

Stand: 2026-11-04. Teil „Querschnitt“ der [Roadmap](../../ROADMAP.md) („Builds für macOS (arm64) und Windows (x64) … folgen mit einem Remote“). Grundlage ist das Briefing [docs/briefing-release-ci.md](../../briefing-release-ci.md). Branch `release-ci` im Worktree `.claude/worktrees/release-ci`.

Die vier offenen Entscheidungen des Briefings hat der Nutzer am 2026-11-04 getroffen:

- **CI-Runner:** gehostete GitHub-Runner `macos-14`, schlank konfiguriert (Pfadfilter, `concurrency` mit Abbruch, Caches). Kein Self-hosted Runner.
- **Signieren:** vorerst nur ad-hoc. Keine Developer ID, keine Notarisierung. Der Workflow bekommt einen vorbereiteten, abgeschalteten Schritt dafür.
- **Auto-Update:** später. In dieser Phase gibt es keinen Updater, kein Schlüsselpaar und keine öffentliche Update-Quelle.
- **Plattformen:** macOS (arm64) und Windows (x64) zusammen in dieser Phase. Linux entfällt.

Die übrigen Entscheidungen hat der Agent nach der Arbeitsweise des Nutzers autonom getroffen (empfohlene Variante). Wo es eine echte Alternative gab, steht sie mit Grund in §9.

## 1. Ziel und Umfang

Gen4 Studio wird für macOS und Windows als installierbare App gebaut: als DMG und als NSIS-Installer. Die App läuft ohne Entwicklungsbaum. Jeder Push auf `main` fährt automatisch die hermetischen Schritte von `scripts/gate.sh`. Ein Tag `v*` erzeugt einen Release-Entwurf auf GitHub.

Sieben Teile:

1. **R1 Hermetisches Gate und CI:** `G4_GATE_HERMETIC=1` in `gate.sh`, dazu `.github/workflows/ci.yml` mit einem macOS- und einem Windows-Job.
2. **R2 Versionierung:** eine Version für Cargo, npm und Tauri, `scripts/version.sh` und `CHANGELOG.md`.
3. **R3 Release-Workflow:** `.github/workflows/release.yml` baut DMG und Installer, prüft beide und legt einen Release-Entwurf an. Lizenzhinweise kommen ins Bundle.
4. **R4 Windows x64:** Machbarkeit, Windows-Zweig von `prepare-bundle.sh`, Suche der Sidecars mit `.exe`, Tauri-Konfiguration je Plattform, Prüfung des Bundles unter Windows.
5. **R5 Prüfung der gepackten App:** `studio doctor`, ein Rauchtest ohne ROM in CI und die Abnahme mit ROM lokal. Das holt die offenen Punkte „gepackte App“ aus 3a bis 3c nach.
6. **R6 ROM-Schutz:** `scripts/check-no-rom-data.sh` für Repo, Historie, Bundles und Artefakte, dazu eine Erlaubnisliste für Actions.
7. **R7 Dokumentation:** `docs/release.md`, `docs/dev-setup.md`, Roadmap und `docs/release-ci-gate.md`.

**Nicht in dieser Phase:** Signieren und Notarisieren (nur vorbereitet), Auto-Update, Linux, ein Universal-Binary für macOS (x86_64), MSI, Windows on ARM, gated Tests oder Szenarien in CI, ein Self-hosted Runner, ein öffentliches Repo oder ein öffentliches Release, der Spielmodus live unter Windows (bleibt nur macOS, 3d.9), Code-Signing unter Windows (SmartScreen warnt).

## 2. Harte Grenzen (aus dem Briefing, unverändert)

- Nichts aus ROMs geht nach GitHub: keine ROMs, Spielstände oder ROM-Ableitungen, auch nicht als Secret, Cache, Artefakt, Log oder Test-Fixture. CI fährt nur die hermetischen Schritte. Gated Tests, `g4emu integration`, Zeitmessungen und Szenarien bleiben in der lokalen `scripts/gate.sh`.
- Das Release-Artefakt enthält keine Spieldaten. `crates/g4-kernel/metadata/` darf mit, weil es schon im Repo liegt.
- Jede neue Abhängigkeit geht durchs Lizenz-Audit, auch Actions und CLI-Werkzeuge (§7.2). Die LGPL-Bibliothek nitroarc bleibt eine separate, austauschbare Datei: `libnitroarc_ffi.dylib` unter macOS, `nitroarc_ffi.dll` unter Windows.
- Das Repo bleibt privat. Pushes nach GitHub (Branch, `main`, Tags, Release-Entwurf) erst nach Zustimmung des Nutzers. Der Release bleibt ein Entwurf.

## 3. R1: Hermetisches Gate und CI

### 3.1 `gate.sh`

- Mit `G4_GATE_HERMETIC=1` endet `gate.sh` nach `g4emu unit`. Die Tabelle schließt mit `Gate (hermetisch): ok`. Ohne den Schalter läuft alles wie bisher. Lokal und in CI fährt damit dieselbe Liste.
- Drei neue Schritte in der hermetischen Liste, direkt nach `npm licenses`:
  - `actions`: `bash scripts/check-actions.sh`, siehe §7.2.
  - `no rom data`: `bash scripts/check-no-rom-data.sh`, ohne Argument über `git ls-files`, siehe §7.1.
  - `version`: `bash scripts/version.sh check`, siehe §4.
- Die Pfade für Toolchain und `~/.cargo/bin` bleiben, wie sie sind. Sie greifen nur, wenn die Ordner existieren, und schaden auf dem Runner nicht.

### 3.2 `.github/workflows/ci.yml`

- **Auslöser:** `push` auf `main`, `pull_request` auf `main` und `workflow_dispatch`. `paths-ignore` für `docs/**` und `**/*.md`. Ausgenommen ist `CHANGELOG.md`, weil `version check` es liest.
- **`concurrency`:** Gruppe `ci-${{ github.ref }}` mit `cancel-in-progress: true`.
- **`permissions`:** `contents: read`. Keine Secrets.
- **Job `gate-macos`** (`macos-14`, `timeout-minutes: 60`):
  - Checkout und Toolchain aus `rust-toolchain.toml` mit `rustfmt` und `clippy`.
  - pnpm in der lokalen Version (12.16.1) und Node 26, `uv` und `cargo-deny@0.20.2` als vorgebautes Binary.
  - Caches mit `actions/cache`: `~/.cargo/registry`, `~/.cargo/git` und `target/`. Der Schlüssel besteht aus Betriebssystem, Rust-Version und dem Hash von `Cargo.lock`, mit Restore-Präfix. Dazu der pnpm-Store (Schlüssel `pnpm-lock.yaml`) und der uv-Cache (Schlüssel `scripts/g4emu/uv.lock`).
  - Danach `pnpm install --frozen-lockfile` in `apps/studio` und `G4_GATE_HERMETIC=1 bash scripts/gate.sh`.
- **Job `gate-windows`** (`windows-latest`, `timeout-minutes: 90`), nur bei `push` auf `main`, bei `workflow_dispatch` und bei einem Pull Request mit dem Label `windows`. Ohne Label läuft er bei Pull Requests nicht (Kontingent). `workflow_dispatch` geht erst, wenn der Workflow auf `main` liegt; bis dahin laufen Probeläufe vom Branch über einen Draft-PR mit Label.
  - Toolchain und MinGW wie in §6.1 ermittelt.
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
  - `cargo test --workspace --all-features`, ohne `G4_*`.
  - Dieselben Cargo-Caches mit eigenem Schlüssel.
- **Ziel für die Dauer:** ein warmer Lauf von `gate-macos` unter 15 Minuten. Die gemessenen Zeiten (kalt und warm, beide Jobs) und die verbrauchten Minuten mit Faktor 10 für macOS und 2 für Windows kommen ins Gate-Protokoll.

## 4. R2: Versionierung

- **Quelle:** `version` in `[workspace.package]` der Wurzel-`Cargo.toml`. Alle Crates erben sie mit `version.workspace = true`.
- `apps/studio/package.json` trägt dieselbe Version, weil pnpm sie braucht. `tauri.conf.json` liest sie mit `"version": "../package.json"`. Es gibt also zwei Stellen, die `version.sh` gleich hält.
- **`scripts/version.sh`:**
  - `version.sh` gibt die Version aus.
  - `version.sh set <semver>` schreibt beide Stellen und aktualisiert `Cargo.lock` mit `cargo update -w --offline`.
  - `version.sh check [<tag>]` prüft: beide Stellen sind gleich und gültiges SemVer, `CHANGELOG.md` hat einen Abschnitt `## [X.Y.Z]` für die Version ohne Pre-Release-Suffix, und ein übergebener Tag ist `v<Version>`.
  - Bei einem Fehler nennt die Ausgabe Datei und Wert und endet mit Status 1.
- **`CHANGELOG.md`** im Format „Keep a Changelog“, auf Deutsch. Er beginnt mit `## [0.1.0]` und fasst die Phasen 1 bis 4 und 3d in wenigen Zeilen zusammen. Der Release-Workflow nimmt den passenden Abschnitt als Text des Release-Entwurfs.
- **Tags:** `vX.Y.Z` oder `vX.Y.Z-rc.N`. Ein `-rc`-Tag wird ein Pre-Release-Entwurf. Ob NSIS ein Pre-Release-Suffix annimmt, prüft der Agent bei R4 (§6.4).
- Das ist unabhängig von 4.5 „Veröffentlichen“. Dort geht es um die Fassungen der ROM-Patches im Projekt, hier um die Fassung der App. Gemeinsam ist nur die SemVer-Prüfung. Deren Kernel-Funktion wird nicht geteilt, weil das Shell-Skript ohne Rust laufen soll.

## 5. R3: Release-Workflow `.github/workflows/release.yml`

- **Auslöser:** `push` von Tags `v*`. Dazu `workflow_dispatch` mit dem Eingang `dry_run` (Standard `true`): Er baut und prüft alles, legt aber keinen Release an. Außerdem `pull_request` mit den Typen `labeled` und `synchronize`. Dieser Lauf startet nur, wenn der PR das Label `release-dry-run` trägt, und gilt dann als `dry_run`. So lässt sich der Release-Workflow vom Branch aus prüfen, bevor er auf `main` liegt.
- **Job `version`:** führt `bash scripts/version.sh check "$GITHUB_REF_NAME"` aus. Bei `dry_run` entfällt die Prüfung des Tags.
- **Job `bundle-macos`** (`macos-14`):
  - Werkzeuge bauen und cachen. Der Cache-Schlüssel nennt Betriebssystem, Zielarchitektur und Fassung.
    - apicula mit `cargo install --git https://github.com/scurest/apicula --rev 3d4e91e14045392a49c89e86dab8cb936225588c --locked`.
    - rotom-lsp mit Tag `v0.7.0`.
    - Beide Fassungen stehen einmal in `scripts/tool-versions.env`. `docs/dev-setup.md`, `prepare-bundle.sh` und die Workflows lesen von dort.
  - Danach `pnpm bundle -- --bundles app,dmg` mit `CI=true`. Tauris DMG-Skript überspringt dann die Finder-Gestaltung per AppleScript, an der der lokale DMG-Bau bisher scheitert. Lokal baut `pnpm bundle` weiter nur das `.app`.
  - `bash apps/studio/scripts/check-bundle.sh`, `bash scripts/check-no-rom-data.sh <app> <dmg>` und der Rauchtest (§8.1).
  - Hochladen als Workflow-Artefakt mit 7 Tagen Aufbewahrung.
  - Ein vorbereiteter, abgeschalteter Schritt `sign-notarize` (`if: false`). Er nennt die Secrets, die er später bräuchte, und kommentiert, wo sie hingehören.
- **Job `bundle-windows`** (`windows-latest`): Werkzeuge gecacht wie oben, `pnpm bundle -- --bundles nsis`, dann die Windows-Prüfung (§6.3), `check-no-rom-data.sh` und der Rauchtest. Hochladen wie oben.
- **Job `publish`:** braucht beide Bundle-Jobs, entfällt bei `dry_run`.
  - `permissions: contents: write`.
  - `gh release create "$TAG" --draft` mit Titel `Gen4 Studio <Version>` und den Notizen aus dem CHANGELOG-Abschnitt. Bei `-rc` kommt `--prerelease` dazu.
  - Dateien:
    - `Gen4-Studio_<Version>_aarch64.dmg`
    - `Gen4-Studio_<Version>_x64-setup.exe`
    - `SHA256SUMS.txt`
  - Die `gh`-CLI ist auf den Runnern vorinstalliert. Es braucht also keine weitere Action.
- **Lizenzhinweise im Bundle:**
  - `THIRD-PARTY-NOTICES.txt` mit den Lizenzen aller Rust-Crates (über `cargo about generate`, Vorlage `about.hbs`, Erlaubnisliste wie `deny.toml`) und der npm-Pakete (`pnpm licenses list --prod`).
  - Im Ordner `licenses/` LGPL-3.0 und GPL-3.0 für nitroarc, mit einem Satz, wie man die Bibliothek austauscht, dazu die Lizenzen von apicula und rotom.
  - Ziel unter macOS ist `Contents/Resources/`, unter Windows der Installationsordner, beides über `bundle.resources`. `check-bundle.sh` prüft, dass die Dateien da sind.

## 6. R4: Windows x64

### 6.1 Machbarkeit (erster Schritt der Umsetzung)

Unter Windows baut uxie nitroarc mit `make ffi` und MinGW-`gcc` zu `nitroarc_ffi.dll` und linkt sie dynamisch. Auf einem Branch klärt ein Lauf mit `workflow_dispatch` auf `windows-latest`:

1. Welches Rust-Ziel `cargo build -p studio` baut. Bevorzugt ist `x86_64-pc-windows-msvc`, mit MinGW-`gcc` und `make` aus MSYS2 oder dem vorinstallierten `C:\mingw64` für den nitroarc-Teil. Rückfall ist `x86_64-pc-windows-gnu`.
2. Ob `CFLAGS=-D_POSIX_C_SOURCE=200809L` aus `.cargo/config.toml` unter Windows schadet. Wenn ja, gilt es nur noch für macOS, über `[target.aarch64-apple-darwin]` oder über `env` im Workflow und in `with-rust.sh`.
3. Ob rotom-lsp (Tag `v0.7.0`) und apicula unter dem gewählten Ziel bauen.
4. Ob `cargo test --workspace` grün ist. Fehler, die nur unter Windows auftreten, wie Pfadtrenner, Zeilenenden oder `/usr/bin/git`, gehören in diese Phase.

**Entscheidungsregel:** Baut `studio` nach zwei Ansätzen (MSVC, dann GNU) nicht, wird Windows eine eigene Teilphase. Diese Phase liefert dann nur macOS, und der Agent meldet das dem Nutzer mit dem Befund.

### 6.2 Bundle

- `prepare-bundle.sh` bekommt einen Zweig nach `uname -s` (MINGW, MSYS oder CYGWIN). Die Ziele:
  - `binaries/apicula-<Triple>.exe`
  - `binaries/rotom-lsp-<Triple>.exe`
  - `windows/nitroarc_ffi.dll`
  - rpath, `install_name_tool` und `codesign` entfallen dort.
  - `apps/studio/src-tauri/windows/` kommt in `.gitignore`, wie `binaries/` und `frameworks/`.
  - `with-rust.sh` bekommt den passenden PATH für Windows.
- Tauri-Konfiguration je Plattform. Tauri 2 führt die Dateien mit `tauri.conf.json` zusammen:
  - `tauri.macos.conf.json`: `bundle.macOS.frameworks` und `targets: ["app"]`.
  - `tauri.windows.conf.json`: `targets: ["nsis"]`. Mit `bundle.resources` kommt `windows/nitroarc_ffi.dll` als `nitroarc_ffi.dll` neben `studio.exe`. Dort finden sie `studio.exe` und `rotom-lsp.exe` über die normale Suchreihenfolge von Windows (Ordner der EXE). NSIS mit `installMode: "currentUser"` (ohne Admin) und WebView2 per Bootstrapper.
  - `tauri.conf.json` behält, was für beide gilt.
- `build.rs` legt die Platzhalter je Zielsystem an: `frameworks/libnitroarc_ffi.dylib` unter macOS, `windows/nitroarc_ffi.dll` unter Windows, und für die Sidecars den Suffix `.exe`.
- `find_apicula` und `find_rotom_lsp` hängen `std::env::consts::EXE_SUFFIX` an, auch für die Debug-Pfade unter `tools/`. Dazu ein Unit-Test, der den gesuchten Namen prüft.
- Die übrigen Pfade unter Windows prüft der Agent mit einer Suche nach festen `/`-Pfaden, `cfg!(target_os = "macos")` ohne Windows-Zweig und dem Start von melonDS („Hier spielen“). Gefundene Fehler gehören in diese Phase. Der Spielmodus live bleibt nur macOS und zeigt unter Windows den vorhandenen Hinweis.

### 6.3 Prüfung des Windows-Bundles

`apps/studio/scripts/check-bundle-windows.sh` läuft in Git Bash auf dem Runner:

- Er installiert den Installer still nach `$RUNNER_TEMP/g4s` (`/S /D=…`).
- Er prüft, dass diese Dateien existieren und nicht leer sind:
  - `studio.exe`
  - `apicula.exe`
  - `rotom-lsp.exe`
  - `nitroarc_ffi.dll`
  - `THIRD-PARTY-NOTICES.txt`
- `apicula.exe --help` muss mit Status 0 enden.
- `rotom-lsp.exe` muss mit geschlossenem stdin ohne Ladefehler enden. Fehlt die DLL, endet es mit Status `0xC0000135`.
- Die Ausgabe endet mit `bundle ok (windows)`.

### 6.4 Version unter NSIS

Weist NSIS `0.1.0-rc.1` zurück, setzt der Workflow für den Windows-Build `bundle.windows.nsis` beziehungsweise die Dateiversion auf `0.1.0.1`. Der Name des Artefakts trägt weiter die SemVer-Version. Der Agent entscheidet das beim ersten `rc`-Lauf und dokumentiert es in `docs/release.md`.

## 7. R6: ROM-Schutz und Audit

### 7.1 `scripts/check-no-rom-data.sh [Pfad …]`

- **Ohne Argument:** prüft die Dateien aus `git ls-files`. Mit `--history <Bereich>` prüft es alle Blobs eines Commit-Bereichs (`git rev-list --objects`).
- **Mit Pfaden:** prüft Dateien und Ordner rekursiv. Ein DMG wird dafür mit `hdiutil attach -readonly` eingehängt.
- **Treffer ist:**
  - eine Endung `.nds`, `.sav`, `.dsv`, `.srl`, `.xdelta` oder `.narc`;
  - ein Ordnername mit `_DSPRE_contents` oder `_testkopien`;
  - jede Datei ab 0x160 Byte, deren NDS-Header-Prüfsumme des Nintendo-Logos (`0xCF56` bei `0x15C`, Little Endian) passt.
- **Ausnahmen:** keine, auch keine Erlaubnisliste. Bekommt eine echte Datei des Repos einen Fehltreffer, wird die Regel genauer, nicht die Datei ausgenommen.
- **Ausgabe:** jeder Treffer mit Pfad und Grund, Status 1, sonst `no rom data: <n> files ok`.
- **Vor dem ersten Push:** läuft über `--history origin/main..main` (275 Commits). Der Bericht kommt ins Gate-Protokoll.

### 7.2 Actions und Werkzeuge

- Jede `uses:`-Zeile in `.github/workflows/*.yml` ist auf einen 40-stelligen Commit-SHA festgelegt, mit `# vX.Y.Z` als Kommentar.
- Erlaubt sind nur Actions aus `.github/actions-allowlist.txt`. Jede Zeile hat das Format `owner/repo SPDX-Lizenz`, und als Lizenz zählt nur die Erlaubnisliste von `deny.toml`.
- `scripts/check-actions.sh` prüft beides. Ist eine Lizenz nicht erlaubt, gibt es eine Meldung und Status 1.
- **Voraussichtliche Liste:**
  - `actions/checkout`, `actions/cache`, `actions/setup-node` und `actions/upload-artifact` sowie `download-artifact` (MIT);
  - `pnpm/action-setup` (MIT);
  - `astral-sh/setup-uv` (MIT OR Apache-2.0);
  - `taiki-e/install-action` (Apache-2.0 OR MIT) für `cargo-deny` und `cargo-about`;
  - falls nötig `msys2/setup-msys2` (MIT).
- `Swatinem/rust-cache` ist bewusst nicht dabei (LGPL-3.0). `actions/cache` reicht.
- `cargo-about` ist ein Werkzeug im Release-Bau und keine Abhängigkeit des Produkts. Seine Lizenz (MIT OR Apache-2.0) steht in der Liste von `docs/release.md`.

### 7.3 Caches und Logs

- Gecacht werden nur `~/.cargo/registry`, `~/.cargo/git`, `target/`, der pnpm-Store, der uv-Cache und die gebauten Werkzeuge. `target/` enthält in CI nur Ergebnisse der hermetischen Tests: Der synthetische Projektbau (`test_support::synthetic_project`) entsteht aus Code, nicht aus ROM-Daten.
- Im Gate prüft der Agent die Cache-Liste mit `gh cache list` und lädt mindestens einen Cache herunter. Den prüft er mit `check-no-rom-data.sh`, ebenso die heruntergeladenen Artefakte. Die Logs des ersten grünen Laufs durchsucht er nach `.nds`, `DSPRE_contents`, `Roms/` und `_saves`.

## 8. R5: Prüfung der gepackten App

### 8.1 `studio doctor` und Rauchtest ohne ROM

- **Neuer Befehl `studio doctor [--json]`:** neben `studio mcp` in `main.rs`, ohne Fenster.
  - Er nennt Version, Pfad des Binarys, die gefundenen Pfade von apicula, rotom-lsp, git und melonDS (oder `fehlt`) und ob nitroarc geladen ist.
  - Dass er überhaupt startet, beweist das Laden der dynamisch gelinkten Bibliothek. Zusätzlich ruft er eine nitroarc-Funktion von uxie auf.
  - Er startet `apicula --help` und führt mit rotom-lsp den LSP-Handshake `initialize`, `shutdown` und `exit` aus. Das beweist, dass beide Sidecars samt Bibliothek im Bundle laufen.
  - Er endet mit Status 0, wenn apicula und rotom-lsp gefunden sind und beide Starts gelingen, sonst mit 1.
  - Eine Sprache wie die App: Deutsch.
- **`scripts/smoke-packaged.sh <studio-binary>`:** läuft auf beiden Runnern und lokal.
  - Umgebung: `env -i` mit eigenem `HOME` und `TMPDIR`, minimalem `PATH` (`/usr/bin:/bin`, unter Windows `C:\Windows\System32`), Arbeitsordner außerhalb des Repos, kein `G4_*`.
  - Der Lauf:
    1. `studio doctor` muss mit Status 0 enden.
    2. Über `studio mcp` (stdio): `initialize`, das synthetische Projekt öffnen, eine Kollision ändern und speichern.
    3. `build_rom`, wenn das synthetische Projekt baubar ist. Das klärt der Plan am vorhandenen `mcp_stdio`-Test. Ist es nicht baubar, entfällt dieser Schritt im Rauchtest; das Bauen deckt dann §8.2 ab.
  - Das synthetische Projekt erzeugt das Beispielprogramm `cargo run -p g4-kernel --example make_synthetic --features test-support -- <Ordner>` aus `test_support::synthetic_project`. Es läuft vor dem Rauchtest im Quellbaum: auf dem Runner und lokal. Das Release-Binary bleibt ohne `test-support`, und es liegt keine Kopie im Repo. Kein Byte stammt aus einer ROM.

### 8.2 Abnahme mit ROM, lokal auf dem Mac (Agent)

Das DMG kommt aus dem Release-Entwurf (`gh release download`) und wird nicht lokal gebaut. Der Agent führt diese Schritte aus:

1. Einhängen, die App nach `~/Applications/Gen4 Studio.app` kopieren, Quarantäne-Attribut setzen wie bei einem Download, mit `xattr -dr` freigeben (dokumentierter Weg) und mit `open` starten. LaunchServices gibt der App die Umgebung von launchd, also kein Shell-`PATH`, kein `G4_*` und kein `tools/`.
2. Ein Bildschirmfoto (`screencapture`) zeigt Start mit der Marke.
3. Kopflos mit dem Binary aus dem Bundle (`Contents/MacOS/studio mcp`) und gleicher Umgebung, auf einer frischen Kopie von Platin DE:
   - Kollision einer Kachel ändern;
   - `build_rom`;
   - Spiel in melonDS starten;
   - ein Script mit Sprachdiensten öffnen (Diagnose von rotom-lsp);
   - einen Header mit Modellen laden (texturierte GLB von apicula, Treffer im Modell-Cache unter dem Werkzeug-Hash des gebündelten apicula).
4. `studio doctor` aus dem Bundle zeigt beide Sidecars in `Contents/MacOS/`.

### 8.3 Hand (nur der Nutzer)

- macOS: das DMG auf einem frischen Nutzerkonto ohne Entwicklungswerkzeuge. Dazu im Fenster: Projekt öffnen, Kollision ändern, ROM bauen, in melonDS spielen, Script mit Sprachdiensten, texturierte Modelle.
- Windows: der Installer auf einem Windows-PC mit denselben Schritten, dazu die Warnung von SmartScreen.

Ein frisches Benutzerkonto legt der Agent nicht an. Das ist eine Änderung der Systemeinstellungen und bleibt dem Nutzer.

## 9. Entscheidungen mit Alternativen

| Frage | Gewählt | Alternative | Grund |
|---|---|---|---|
| Wer installiert die Toolchain | `rustup` (vorinstalliert) mit `rust-toolchain.toml` | `dtolnay/rust-toolchain` | eine Action weniger, gleiche Toolchain wie lokal |
| Cargo-Cache | `actions/cache` mit eigenem Schlüssel | `Swatinem/rust-cache` | LGPL-3.0 nicht in der Erlaubnisliste |
| Release hochladen | `gh release create` | `softprops/action-gh-release`, `tauri-action` | vorinstalliert, keine weitere Action; `tauri-action` würde `pnpm bundle` umgehen |
| Windows-Installer | NSIS, `currentUser` | MSI (WiX) | ohne Admin, später updater-fähig; MSI verlangt rein numerische Versionen |
| Windows-Job bei PRs | nein, nur `main` | bei jedem PR | Kontingent; ein PR-Lauf auf macOS deckt die meisten Fehler |
| DMG bauen | Tauri `dmg` mit `CI=true` | `hdiutil create` selbst | weniger eigener Code; Rückfall ist `hdiutil`, falls Tauris Skript auf dem Runner scheitert |
| Versionsquelle | `[workspace.package]` + `package.json`, `version.sh` hält beide gleich | nur `package.json` | Cargo braucht eine feste Zahl; Tauri liest `package.json` direkt |
| Lizenzhinweise | `cargo about` + `pnpm licenses` | von Hand | MIT/Apache verlangen den Hinweis bei Weitergabe; von Hand veraltet er |
| Kein Self-hosted Runner | gehostet | Runner auf dem Mac | Entscheidung des Nutzers; ROMs bleiben auf einem Rechner ohne GitHub-Jobs |

## 10. Gate

Ablauf in `docs/release-ci-gate.md`, nach dem Muster der Phasen-Gates:

- **(G1)** Ein Push auf `main` ergibt einen grünen `ci.yml`-Lauf mit `gate-macos` und `gate-windows`. `gate-macos` fährt dieselbe Liste wie `G4_GATE_HERMETIC=1 bash scripts/gate.sh` lokal, und die Tabelle beider Läufe ist gleich.
- **(G2)** Auf dem Testbranch `ci-rot` (als Draft-PR auf `main`) gibt es zwei Commits. Einer verletzt `cargo fmt`, der andere ist ein absichtlich roter `pnpm test`. Beide ergeben einen roten Lauf, jeweils am erwarteten Schritt. Danach wird der Branch gelöscht.
- **(G3)** Der Tag `v0.1.0-rc.1` erzeugt einen Pre-Release-Entwurf mit DMG, `x64-setup.exe` und `SHA256SUMS.txt`. `check-bundle.sh`, `check-bundle-windows.sh`, `check-no-rom-data.sh` und der Rauchtest sind in beiden Bundle-Jobs grün. Die Prüfsummen passen zu den Dateien.
- **(G4)** Der Probelauf (Label `release-dry-run` auf dem Draft-PR, nach dem Merge `workflow_dispatch` mit `dry_run`) baut und prüft alles ohne Release.
- **Kontingent:** macOS-Minuten zählen zehnfach. Entwickelt und geprüft wird zuerst lokal auf dem Mac. Pushes auf den PR werden gebündelt, und ein Lauf auf dem Runner prüft nur, was lokal nicht geht: Windows, den Runner selbst und das DMG mit `CI=true`.
- **(G5)** Die Abnahme aus §8.2 ist mit dem heruntergeladenen DMG auf dem Mac bestanden.
- **(G6)** Der Windows-Installer installiert auf dem Runner still. `studio doctor` und der Rauchtest sind grün, und `find_apicula` und `find_rotom_lsp` finden die `.exe`.
- **(G7)** Kein Artefakt, kein Cache und kein Log enthält ROM-Daten. `check-no-rom-data.sh` ist grün über `--history origin/main..main` vor dem ersten Push, über die heruntergeladenen Artefakte und über einen heruntergeladenen Cache. Die Suche in den Logs aus §7.3 findet nichts.
- **(G8)** `scripts/gate.sh` ist lokal voll grün auf dem Merge-Stand. `cargo deny` und `check-licenses-npm.sh` sind grün mit allen neuen Abhängigkeiten. `check-actions.sh` ist grün.
- **Zeiten:** Dauer kalt und warm beider Jobs und verbrauchte Minuten stehen im Gate-Protokoll.
- **Hand:** §8.3.

## 11. Dokumentation

- `docs/release.md` neu gegliedert:
  - macOS (DMG, ad-hoc, Gatekeeper-Weg);
  - Windows (NSIS, SmartScreen, DLL neben der EXE);
  - Workflows, Tags und Versionen, Lizenzhinweise;
  - der vorbereitete Schritt zum Signieren mit den Schritten, die eine Developer ID bräuchte.
  - Die Grenze „Nur das `.app`, kein DMG“ wird ersetzt.
- `docs/dev-setup.md`: Der Satz „Es gibt kein Remote und keine CI“ wird ersetzt. Neu steht dort: Es gibt ein privates Remote mit CI, die nur die hermetischen Schritte fährt, und vor einem Merge läuft weiter `gate.sh` voll. Dazu `G4_GATE_HERMETIC` und `scripts/tool-versions.env`.
- `docs/ROADMAP.md`, Querschnitt: die Sätze zu Remote, CI und Releases. Die Zeile „Releases: Tauri-Bundles, signiert, mit Auto-Update ab Phase 3b“ wird zu „ad-hoc signiert; Developer ID und Auto-Update später“. Dazu kommt eine Zeile „Release und CI“ mit Stand und Gate. Die Sätze in den Zeilen von Phase 2 und 3b.1 bleiben als Geschichte stehen.
- `docs/release-ci-gate.md` ist das Gate-Protokoll.
