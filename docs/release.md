# Release: macOS-DMG und Windows-Installer

Gen4 Studio erscheint als DMG für macOS (arm64) und als NSIS-Installer für Windows (x64). Beide Pakete entstehen aus den
hermetischen Quellen: keine ROM, kein Spielstand, kein Geheimnis. Das DMG baut `scripts/release-local.sh` auf dem Mac des
Maintainers, der auch Tag und Release-Entwurf anlegt; nur den Installer baut der Workflow `release.yml` auf einem gehosteten
Windows-Runner, von Hand gestartet. Eine automatische CI gibt es nicht mehr (Abschnitt „Workflows, Tags und Versionen“). Dieses
Dokument beschreibt beide Bundles, den Weg vom Tag zum Release, die Lizenzhinweise und den vorbereiteten Schritt zum Signieren.
Die Abnahme mit einer ROM und mit dem fertigen Paket steht in `docs/release-ci-gate.md`.

Das Bundle enthält Fremdkomponenten, damit die App ohne den Entwicklungsbaum läuft:

- nitroarc (macOS `Contents/Frameworks/libnitroarc_ffi.dylib`, Windows `nitroarc_ffi.dll` neben `studio.exe`): die
  LGPL-lizenzierte Bibliothek, die uxie beim Release-Build als Shared Library erzeugt. Sie bleibt eine separate Datei
  (kein statisches Linken), damit sie austauschbar ist (Abschnitt „nitroarc austauschen“).
- apicula (macOS `Contents/MacOS/apicula`, Windows `apicula.exe` neben `studio.exe`): der Sidecar für NSBMD zu GLB. Tauri entfernt beim Bundeln den
  Target-Triple-Suffix (`apicula-aarch64-apple-darwin` wird zu `apicula`). Die App findet ihn über Eintrag 2 der
  apicula-Suche (siehe `docs/dev-setup.md`), `G4_APICULA` muss nicht gesetzt sein.
- rotom-lsp (macOS `Contents/MacOS/rotom-lsp`, Windows `rotom-lsp.exe` neben `studio.exe`): der Sprachserver der Scripts (Git-Tag
  `v0.7.0`, wie die rotom-Bibliothek). Unter macOS hat er den rpath `@executable_path/../Frameworks` und lädt dieselbe
  `libnitroarc_ffi.dylib`. Die App findet ihn neben dem Studio-Binary (Eintrag 2 der Suche in `docs/dev-setup.md`).

Der Spielmodus live (melonDS-Kern im Fenster) bleibt macOS-only; unter Windows meldet die App das mit einem Hinweis.

## Voraussetzungen (lokal, macOS)

- apicula gebaut unter `tools/apicula/bin/apicula` (siehe `docs/dev-setup.md`).
- `rotom-lsp` unter `tools/rotom/bin/rotom-lsp`; fehlt es, baut `prepare-bundle.sh` es aus dem Git-Tag (Netz beim ersten Mal).
- Kein eigener `PATH` mehr nötig: `pnpm bundle` und `pnpm tauri` laufen über `apps/studio/scripts/with-rust.sh`, das `/opt/homebrew/opt/rustup/bin` und `~/.cargo/bin` vor den `PATH` setzt (`tauri build` braucht `cargo`; ohne das brach der Bau mit „failed to run 'cargo metadata'“ ab).
- Die Datei `.cargo/config.toml` (CFLAGS für nitroarc auf neuen macOS-SDKs) liegt im Repository und gilt automatisch. Unter Windows stört sie nicht.
- Die Fassungen der Werkzeuge (apicula, rotom, cargo-deny, pnpm, Node, cargo-about) stehen an einer Stelle: `scripts/tool-versions.env`.

## macOS: das DMG

`pnpm bundle` baut nur das `.app`; das DMG entsteht in `scripts/release-local.sh` (Abschnitt „Workflows, Tags und Versionen“).

```sh
cd apps/studio
pnpm bundle                      # prepare-bundle.sh, danach tauri build (nur das .app)
bash scripts/check-bundle.sh     # Prüfung des Bundles, Ausgabe: bundle ok
```

`pnpm bundle` macht im Einzelnen:

1. `scripts/prepare-bundle.sh` baut `cargo build --release -p studio` (dabei entsteht `target/release/libnitroarc_ffi.dylib`)
   und kopiert apicula nach `src-tauri/binaries/apicula-<Target-Triple>`, `rotom-lsp` nach
   `src-tauri/binaries/rotom-lsp-<Target-Triple>` (mit rpath, neu signiert) sowie die dylib nach
   `src-tauri/frameworks/libnitroarc_ffi.dylib`. Dazu erzeugt es die Lizenzhinweise (Abschnitt „Lizenzhinweise“).
   Fehlt eine Quelle, bricht das Skript mit einer Fehlermeldung ab. Die Zielordner sind per `.gitignore` ausgeschlossen.
2. `tauri build` legt `externalBin` nach `Contents/MacOS/`, `macOS.frameworks` nach `Contents/Frameworks/` und die
   Lizenzhinweise nach `Contents/Resources/licenses/`. Das Ergebnis liegt in `target/release/bundle/macos/Gen4 Studio.app`.

Das DMG entsteht in `scripts/release-local.sh` mit `pnpm bundle -- --bundles app,dmg` und `CI=true`: Tauris DMG-Skript lässt
dann die Gestaltung des Finder-Fensters per AppleScript aus (auf dem Runner, für den der Schritt gebaut wurde, scheiterte sie
ohne Desktop-Sitzung; lokal bleibt das Ergebnis so dasselbe). Das ist die Entscheidung der Spezifikation (Tauri statt eigenem
Skript). Als Rückfall gilt `scripts/release/macos-dmg.sh`: Hat Tauri kein DMG geliefert, baut das Skript es aus dem fertigen
`.app` mit `hdiutil create -volname "Gen4 Studio" -srcfolder … -format UDZO` und meldet `dmg: tauri` oder `dmg: hdiutil fallback`.
Ohne `.app` scheitert es. Der Name der Datei ist `Gen4-Studio_<Fassung>_aarch64.dmg`.

**Signatur:** Das Bundle ist als Ganzes ad-hoc signiert (`bundle.macOS.signingIdentity: "-"` in `tauri.macos.conf.json`:
Tauri signiert Sidecars, Framework und App mit gebundener `Info.plist` und versiegelten Ressourcen), nicht mit einer Developer ID
und nicht notarisiert. `check-bundle.sh` prüft das mit `codesign --verify --deep --strict`. Dazu ist `hardenedRuntime: false`
gesetzt: Mit Hardened Runtime lehnt die Library Validation die ad-hoc signierte `libnitroarc_ffi.dylib` ab („different Team
IDs“), und `rotom-lsp` startet nicht. Für eine spätere Notarisierung (Developer ID) braucht es Hardened Runtime wieder, dann mit
der Entitlement `com.apple.security.cs.disable-library-validation` oder einer mit derselben ID signierten dylib. Bis `0.1.0-rc.1` trug nur das
Binary die Linker-Signatur; ein heruntergeladenes DMG galt dann als „beschädigt“ statt „nicht verifiziert“ (Gate G5, Befund B1).
Beim ersten Start verlangt Gatekeeper deshalb eine Freigabe:

- Rechtsklick auf die App, dann Öffnen (und im Dialog noch einmal Öffnen); oder Systemeinstellungen, Datenschutz & Sicherheit,
  Trotzdem öffnen.
- Oder im Terminal: `xattr -dr com.apple.quarantine "/Applications/Gen4 Studio.app"`.

Nur die Architektur des Build-Rechners (`aarch64-apple-darwin`); ein Universal-Binary ist nicht vorgesehen. Mindestsystem
macOS 13.0.

## Windows: der Installer

Ziel ist `x86_64-pc-windows-msvc`. Das ist der Standard von `windows-latest`: kein `rustup target add`, kein
`CARGO_BUILD_TARGET`, `target/` ist `target/debug` wie auf dem Mac. Der Probelauf auf dem Runner hat beide Ziele
verglichen: Mit MSVC baut `studio` kalt in 657 s (`cargo build -p studio`); mit `x86_64-pc-windows-gnu` scheitert der Link
von `studio` (`ld returned 8`). Die GNU-Zeile ist deshalb entfallen.

**nitroarc unter Windows:** uxie baut nitroarc mit `make ffi` und `gcc` zu `nitroarc_ffi.dll`. Das braucht MinGW-gcc und make
auch unter MSVC-Rust; beide kommen aus MSYS2 (`msys2/setup-msys2`, Paket `mingw-w64-x86_64-gcc make`). Die CFLAGS aus
`.cargo/config.toml` (`-D_POSIX_C_SOURCE=200809L`) schaden dabei nicht; sie bleiben für alle Plattformen.

**Bundle:** NSIS (`pnpm bundle -- --bundles nsis`, unter Git Bash). Die Einstellungen stehen in
`apps/studio/src-tauri/tauri.windows.conf.json`:

- `installMode: currentUser`: der Installer braucht keine Administratorrechte; Standardordner ist `%LOCALAPPDATA%\Gen4 Studio`
  (Tauris NSIS-Vorlage, `@tauri-apps/cli` 2.12.3: `StrCpy $INSTDIR "$LOCALAPPDATA\${PRODUCTNAME}"`; eine frühere Installation
  behält ihren Ordner).
- WebView2 kommt über den Download-Bootstrapper (`downloadBootstrapper`), der Installer ist also klein und braucht beim
  ersten Installieren Netz, falls WebView2 fehlt.
- `nitroarc_ffi.dll` liegt neben `studio.exe`. Sie steht als Ressource nur in `apps/studio/src-tauri/tauri.bundle.windows.conf.json`,
  die `apps/studio/scripts/bundle.sh` unter Git Bash an `tauri build` übergibt. Grund: `tauri-build` kopiert bei jedem
  `cargo build` und `cargo test` alle `bundle.resources` neben die Binary und überschriebe die echte DLL (von uxies
  `build.rs` dorthin gelegt) mit dem leeren Platzhalter; `target\debug\studio.exe` fände dann die DLL nicht mehr.

**Signatur:** Der Installer ist nicht signiert. SmartScreen warnt beim ersten Start („Der Computer wurde durch Windows
geschützt“): „Weitere Informationen“, dann „Trotzdem ausführen“.

**Prüfung:** `apps/studio/scripts/check-bundle-windows.sh <Installer>` installiert still in einen Ordner unter `$RUNNER_TEMP`,
prüft die Dateien (`studio.exe`, `apicula.exe`, `rotom-lsp.exe`, die DLL, `licenses/`) und ruft `studio.exe doctor` auf; die Ausgabe endet mit
`bundle ok (windows)`. Danach laufen `check-no-rom-data.sh` und der Rauchtest über den Installationsordner.

### NSIS und Vorabversionen

Die Spezifikation (§6.4) fragt, ob NSIS eine Fassung wie `0.1.0-rc.1` annimmt; sonst bekäme der Windows-Build
für `bundle.windows.nsis` die Dateiversion `0.1.0.1`, während der Name des Artefakts bei SemVer bleibt. Der Probelauf
des Release-Workflows baute die Windows-Seite auf Pull-Request-Läufen dazu mit `0.1.0-rc.1` (nie eingecheckt); ein scheiterndes Bundle wäre die Antwort gewesen.

Befund (Probelauf 41960501794, 2026-11-05): NSIS nimmt `0.1.0-rc.1` an. Das Bundle heißt `Gen4 Studio_0.1.0-rc.1_x64-setup.exe`,
installiert still, und `studio doctor` meldet `studio 0.1.0-rc.1`. Eine numerische Ersatzfassung (`0.1.0.1`) braucht es nicht;
der Workflow setzt keine.

## Workflows, Tags und Versionen

**Keine automatische CI.** Seit dem 5. November 2026 startet kein Push, kein Pull Request und kein Tag mehr einen Workflow:
gehostete Minuten kosten Geld (ein Push auf `main` lief rund 1,80 $, macOS-Minuten zählen zehnfach), und die Prüfung, die zählt,
ist ohnehin das volle `scripts/gate.sh` auf dem Mac vor jedem Merge (`docs/dev-setup.md`). Beide Workflows laufen nur von Hand
(`workflow_dispatch`).

**`ci.yml`** fährt nur hermetische Schritte: `gate-macos` das hermetische Gate (`G4_GATE_HERMETIC=1 bash scripts/gate.sh`),
`gate-windows` nur `cargo clippy` und `cargo test --workspace --all-features --no-fail-fast` (`target/` und `TEMP` auf einem
Dev Drive). `gate-macos` läuft bei jedem Start, `gate-windows` nur mit dem Eingang `windows`:

```sh
gh workflow run ci.yml                    # nur gate-macos
gh workflow run ci.yml -f windows=true    # dazu gate-windows (etwa nach einer Änderung, die Windows betrifft)
gh workflow run ci.yml --ref <Branch>     # ein Branch statt main (er muss gepusht sein)
```

**Release, Schritt 1: `scripts/release-local.sh <Tag> [--dry-run]`** auf dem Mac (macOS arm64), aus der Repo-Wurzel:

1. Prüfungen vor dem Bau: `version.sh check <Tag>`, sauberer Arbeitsbaum, HEAD gleich `main` gleich `origin/main` (nach
   `git fetch origin main`), `check-no-rom-data.sh` über das Repository, die Release-Notizen (Abschnitt der Fassung ohne
   Suffix aus `CHANGELOG.md`, `scripts/release/changelog-section.sh`). Ohne `--dry-run` zusätzlich: `gh` ist angemeldet,
   zum Tag gibt es noch keinen Release (auch keinen Entwurf), ein schon vorhandener Tag (lokal oder auf `origin`) zeigt auf HEAD.
2. Bau wie früher der Job `bundle-macos`: `pnpm install --frozen-lockfile`, dann `CI=true pnpm bundle -- --bundles app,dmg`
   (apicula und rotom-lsp baut `prepare-bundle.sh` nach `scripts/tool-versions.env`, wenn sie in `tools/` fehlen; `cargo-about`
   in der Fassung `CARGO_ABOUT_VERSION` muss installiert sein), danach `scripts/release/macos-dmg.sh` mit dem hdiutil-Rückfall.
3. Prüfung: `check-bundle.sh`, `check-no-rom-data.sh <app> <dmg>` und der Rauchtest (`smoke-packaged.sh`, leere Umgebung) auf
   einem synthetischen Projekt aus `make_synthetic`.
4. `dist-release/` (gitignoriert) mit `Gen4-Studio_<Fassung>_aarch64.dmg`, `SHA256SUMS.txt` (`shasum -a 256`) und `notes.md`.
5. Ohne `--dry-run`: `git tag -a <Tag>` (falls er fehlt), `git push origin <Tag>` und
   `gh release create <Tag> --draft [--prerelease] --title "Gen4 Studio <Fassung>" --notes-file dist-release/notes.md` mit DMG
   und `SHA256SUMS.txt`. Einen Tag mit `-rc` legt das Skript als Pre-Release an.

Mit `--dry-run` läuft alles bis auf Tag, Push und Release; die letzte Zeile ist `release-local: Probelauf ok (<Dateien>)`.
`G4_RELEASE_ALLOW_BRANCH=1` lässt einen Probelauf außerhalb von `main` zu (etwa in einem Worktree), nur mit `--dry-run`;
`G4_RELEASE_SKIP_BUILD=1` ist nur für `scripts/tests/release-local.test.sh`. Der Probelauf vom 5. November 2026 (`v0.1.0-rc.2`)
dauerte rund 5 Minuten nach einem Release-Build von gut 3 Minuten, kalt also etwa 10 Minuten.

**Release, Schritt 2: `release.yml`** (Windows) baut den Installer auf `windows-latest` aus dem Tag und prüft ihn
(`check-bundle-windows.sh`, `check-no-rom-data.sh`, Rauchtest):

```sh
gh workflow run release.yml -f tag=vX.Y.Z -f dry_run=false   # Installer an den Entwurf hängen
gh workflow run release.yml -f tag=vX.Y.Z                    # Probelauf (dry_run=true): bauen und prüfen, nichts hochladen
```

Der Job `version` prüft die Fassung des Tags gegen den Tag, `bundle-windows` baut und prüft, und `publish-windows` (nur ohne
`dry_run`) lädt `Gen4-Studio_<Fassung>_x64-setup.exe` an den **vorhandenen** Entwurf, ergänzt dessen `SHA256SUMS.txt` um die
Zeile des Installers (`scripts/release/add-checksum.sh`, eine Zeile gleichen Namens wird ersetzt) und lädt sie mit `--clobber`
neu hoch. Gibt es zum Tag keinen Release oder ist er kein Entwurf mehr, bricht der Job mit einer Meldung ab. Ein Lauf kostet kalt
rund 0,70 $ (Windows-Minuten zählen doppelt); Caches gehören zum Ref, von dem der Workflow startet (Standard `main`), ein
späterer Lauf liest also, was der vorige gespeichert hat.

Danach prüft der Maintainer den Entwurf (DMG, Installer, `SHA256SUMS.txt` mit beiden Zeilen, Text) und veröffentlicht ihn von Hand.

**Versionen:** Die Fassung steht an einer Stelle, `version` in `[workspace.package]` der `Cargo.toml`; `apps/studio/package.json`
trägt dieselbe Zahl (Tauri liest sie von dort). `scripts/version.sh` druckt sie (`version.sh`), setzt sie (`version.sh set <Fassung>`,
schreibt auch `Cargo.lock`) und prüft sie (`version.sh check [<Tag>]`: beide Stellen gleich, gültiges SemVer, ein Abschnitt
in `CHANGELOG.md`, der Tag ist `v<Fassung>`). Tags heißen `vX.Y.Z` und `vX.Y.Z-rc.N`; für `-rc` gilt der Abschnitt der Fassung ohne Suffix.
Ablauf: `version.sh set`, Abschnitt in `CHANGELOG.md` ergänzen, `scripts/gate.sh`, mergen und `main` pushen, dann
`bash scripts/release-local.sh vX.Y.Z` (Tag, Push, Entwurf mit dem DMG), `gh workflow run release.yml -f tag=vX.Y.Z -f dry_run=false`
(Installer dazu), Entwurf prüfen und veröffentlichen.

## Lizenzhinweise

Jedes Bundle enthält den Ordner `licenses/` (macOS `Contents/Resources/licenses/`, Windows `<Installationsordner>\licenses\`):

- `THIRD-PARTY-NOTICES.txt`: erzeugt von `apps/studio/scripts/third-party-notices.sh` (aufgerufen von `prepare-bundle.sh`).
  Die Rust-Crates liefert `cargo-about` (Vorlage `about.hbs`, Erlaubnisliste `about.toml`, die genau der Liste in `deny.toml`
  gleicht), danach folgen die Produktionspakete von `apps/studio` aus `pnpm licenses`.
- die GPL- und LGPL-Texte von nitroarc und `nitroarc-AUSTAUSCH.txt` (die Anleitung unten, im Paket);
- die Lizenzen von apicula (0BSD) und rotom (MIT).

`cargo-about` ist ein Werkzeug des Baus (Lizenz MIT OR Apache-2.0), keine Abhängigkeit des Produkts. Seine Fassung steht als
`CARGO_ABOUT_VERSION` in `scripts/tool-versions.env`.

Die Lizenzprüfung bleibt streng: `cargo deny` (`deny.toml`) und `scripts/check-licenses-npm.sh` lassen nur die Erlaubnisliste
zu. Für die GitHub Actions gilt dasselbe: `.github/actions-allowlist.txt` nennt jede erlaubte Action mit Lizenz,
`scripts/check-actions.sh` prüft, dass jede `uses:`-Zeile auf einen 40-stelligen SHA festgelegt ist und in der Liste steht.
Deshalb gibt es kein `Swatinem/rust-cache` (LGPL-3.0); der Cache läuft über `actions/cache` (MIT).

## nitroarc austauschen

nitroarc steht unter der LGPL und bleibt darum eine eigene Datei. Wer eine eigene Fassung einsetzen will:

- macOS: `Contents/Frameworks/libnitroarc_ffi.dylib` im Paket ersetzen, danach die dylib und die App neu ad-hoc signieren
  (sonst verweigert macOS den Start):

  ```sh
  codesign --force --sign - "/Applications/Gen4 Studio.app/Contents/Frameworks/libnitroarc_ffi.dylib"
  codesign --force --sign - "/Applications/Gen4 Studio.app"
  ```

- Windows: `nitroarc_ffi.dll` neben `studio.exe` im Installationsordner ersetzen. Eine Signatur gibt es nicht, also ist
  nichts weiter nötig.

Die eigene Bibliothek muss unter demselben Namen liegen und dieselbe Schnittstelle bieten wie die gebündelte. `licenses/nitroarc-AUSTAUSCH.txt` im Paket sagt dasselbe.

## Prüfung des gepackten Programms

- `studio doctor [--json]` meldet, ob die Sidecars gefunden werden und nitroarc lädt; mit `--json` für Skripte
  (`"nitroarc": true`). Unter Windows ist `studio.exe` ein GUI-Programm: aus `cmd` oder PowerShell gestartet hängt
  `doctor` sich an deren Konsole und schreibt dorthin, die Shell wartet aber nicht darauf (die Ausgabe kommt nach der
  Eingabeaufforderung, `%ERRORLEVEL%` bleibt alt). Mit Warten und Exit-Code: `studio.exe doctor | more` (cmd) oder
  `studio.exe doctor | Out-Host` (PowerShell); `check-bundle-windows.sh` liest die Ausgabe über eine Pipe.
- `scripts/smoke-packaged.sh <studio-Binary> <Projekt>` ruft in einer leeren Umgebung (`env -i`, kein `G4_*`) `studio doctor` und
  danach `studio mcp` über stdio auf einem synthetischen Projekt (Kollision ändern, lesen, speichern). Das Projekt kommt aus
  Code, ohne ROM: `cargo run -p g4-kernel --example make_synthetic --features test-support -- <Ordner>`.
- `scripts/check-no-rom-data.sh [--history <Bereich>] [Pfad …]` sucht ROM-Daten in Dateien, Ordnern, DMG und Verlauf; auch in
  jedem Release-Lauf über die fertigen Pakete.

## Prüfbefehle (macOS)

`scripts/check-bundle.sh` prüft:

- `Contents/MacOS/apicula` ist ausführbar und startet (`--help`).
- `Contents/MacOS/rotom-lsp` ist ausführbar, hat den rpath `@executable_path/../Frameworks` und startet ohne dyld-Fehler.
- `Contents/Frameworks/libnitroarc_ffi.dylib` existiert und hat den Install-Namen `@rpath/libnitroarc_ffi.dylib`.
- Das Studio-Binary hat den rpath `@executable_path/../Frameworks` und lädt die dylib über `@rpath/...`;
  kein absoluter Pfad in `target/` ist eingebunden.
- Das Symbol des Bundles (`Contents/Resources/<CFBundleIconFile>` aus `Info.plist`, `.icns` ergänzt, wenn es fehlt) existiert
  und hat denselben SHA-256 wie `apps/studio/src-tauri/icons/icon.icns`, und dieser ist nicht der Hash des alten
  Standardsymbols von create-tauri-app (Konstante `OLD_DEFAULT_ICNS` im Skript).

Zusätzlich von Hand:

```sh
APP="target/release/bundle/macos/Gen4 Studio.app"
otool -L "$APP/Contents/MacOS/studio"      # Zeile mit @rpath/libnitroarc_ffi.dylib
env -u G4_APICULA "$APP/Contents/MacOS/studio"   # startet ohne dyld-Fehler
DYLD_PRINT_LIBRARIES=1 "$APP/Contents/MacOS/studio" 2>&1 | grep nitroarc   # Pfad liegt in Contents/Frameworks
```

Abnahme von Hand: die `.app` aus Finder starten, ein Projekt (Kopie) öffnen und einen Header mit Modellen laden
(`G4_APICULA` nicht gesetzt); die Modelle kommen dann aus dem gebündelten apicula.

## App-Symbol

Das Symbol der App ist die Marke aus dem Design-System (`design-system/README.md`, Abschnitt „Marke“). Quelle sind
`design-system/marke/gen4-studio-marke.svg` und `design-system/marke/gen4-studio-marke-1024.png`; beide schreibt
`design-system/scripts/marke.py` aus den Tokens. Alle Größen unter `apps/studio/src-tauri/icons/` entstehen daraus mit
`tauri icon`:

```sh
python3 design-system/scripts/marke.py                         # SVG und 1024-px-PNG neu, aus der Repo-Wurzel
cd apps/studio
pnpm tauri icon ../../design-system/marke/gen4-studio-marke-1024.png
```

`tauri icon` legt dabei auch `64x64.png`, `android/` und `ios/` an. Sie werden nicht eingecheckt (gelöscht), weil
`tauri.conf.json` sie nicht nennt; die Windows-Store-Logos (`Square*Logo.png`, `StoreLogo.png`) bleiben eingecheckt
wie bisher. Unter `tauri dev` zeigt das Dock `icons/icon.png`, im Bundle kommt `icons/icon.icns` nach
`Contents/Resources/` (Name in `CFBundleIconFile`); der Über-Dialog nimmt das Bundle-Symbol.

## Signieren und Notarisieren (vorbereitet, aus)

`scripts/release-local.sh` enthält die Funktion `sign_notarize`, die nur meldet, dass Signieren nicht aktiviert ist; der Kopf des
Skripts beschreibt die Schritte (früher der Schritt `sign-notarize` mit `if: false` in `release.yml`). Mit einer Developer ID
würde es so aktiviert:

1. Apple Developer Program (kostenpflichtig) beitreten und ein Zertifikat „Developer ID Application“ erzeugen; es liegt dann im
   Anmelde-Schlüsselbund des Macs (ein Base64-`.p12` in `APPLE_CERTIFICATE` mit `APPLE_CERTIFICATE_PASSWORD` braucht es nur, wo
   das Zertifikat in keinem Schlüsselbund liegt).
2. `signingIdentity` in `apps/studio/src-tauri/tauri.macos.conf.json` auf den Namen der Identität setzen (etwa „Developer ID
   Application: Name (TEAMID)“, alternativ `APPLE_SIGNING_IDENTITY` vor dem Bundle-Schritt exportieren) und Hardened Runtime
   wieder einschalten (Abschnitt „Signatur“ oben).
3. Notarisierung: Apple-ID (`APPLE_ID`), ein App-spezifisches Passwort (`APPLE_PASSWORD`, nicht das Anmeldepasswort; alternativ
   ein API-Schlüssel) und die Team-ID (`APPLE_TEAM_ID`) in der Umgebung des Bundle-Schritts: Der Tauri-Bundler signiert das `.app`
   und ruft `notarytool` auf, sobald die Variablen gesetzt sind.
4. In `sign_notarize` die Zeilen aus dem Kopf des Skripts einsetzen: das DMG notarisieren (`xcrun notarytool submit … --wait`) und
   das Ticket anheften (`xcrun stapler staple`).

Unter Windows bräuchte es ein Code-Signing-Zertifikat; ohne eines bleibt die Warnung von SmartScreen. Auto-Update gibt es noch nicht.

## Bekannte Grenzen

- Beide Pakete sind nicht mit einer Developer ID signiert und nicht notarisiert (macOS ad-hoc, Windows gar nicht); die Wege
  stehen oben.
- Nur macOS arm64 und Windows x64; Linux ist kein Ziel dieser Phase.
- Auto-Update und Signatur über eine Developer ID folgen später.
