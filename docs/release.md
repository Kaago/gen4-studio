# Release: macOS-DMG und Windows-Installer

Gen4 Studio erscheint als DMG für macOS (arm64) und als NSIS-Installer für Windows (x64). Beide Pakete baut der Workflow
`release.yml` auf gehosteten Runnern aus den hermetischen Quellen: keine ROM, kein Spielstand, kein Geheimnis. Dieses Dokument
beschreibt beide Bundles, den Weg vom Tag zum Release, die Lizenzhinweise und den vorbereiteten Schritt zum Signieren.
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

Lokal baut `pnpm bundle` nur das `.app`; im Release-Workflow entsteht zusätzlich das DMG.

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

Das DMG entsteht auf dem Runner mit `pnpm bundle -- --bundles app,dmg` und `CI=true`: Tauris DMG-Skript lässt dann die
Gestaltung des Finder-Fensters per AppleScript aus, die ohne Desktop-Sitzung scheitert. Das ist die Entscheidung der
Spezifikation (Tauri statt eigenem Skript). Als Rückfall gilt `scripts/release/macos-dmg.sh`: Hat Tauri kein DMG
geliefert, baut das Skript es aus dem fertigen `.app` mit `hdiutil create -volname "Gen4 Studio" -srcfolder … -format UDZO`
und schreibt `dmg: tauri` oder `dmg: hdiutil fallback` in die Zusammenfassung des Laufs. Ohne `.app` scheitert es.
Der Name der Datei ist `Gen4-Studio_<Fassung>_aarch64.dmg`.

**Signatur:** Das Bundle ist ad-hoc signiert (`codesign --sign -`), nicht mit einer Developer ID und nicht notarisiert.
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
des Release-Workflows baut die Windows-Seite auf Pull-Request-Läufen dazu mit `0.1.0-rc.1` (nie eingecheckt); scheitert dort das Bundle, ist das die Antwort.

Befund (Probelauf 41960501794, 2026-11-05): NSIS nimmt `0.1.0-rc.1` an. Das Bundle heißt `Gen4 Studio_0.1.0-rc.1_x64-setup.exe`,
installiert still, und `studio doctor` meldet `studio 0.1.0-rc.1`. Eine numerische Ersatzfassung (`0.1.0.1`) braucht es nicht;
der Workflow setzt keine.

## Workflows, Tags und Versionen

**`ci.yml`** läuft bei jedem Pull Request und jedem Push auf `main`. `docs/**` und `*.md` (außer `CHANGELOG.md`) starten
keinen Lauf. Beide Jobs fahren nur hermetische Schritte: `gate-macos` das hermetische Gate (`G4_GATE_HERMETIC=1 bash scripts/gate.sh`,
siehe `docs/dev-setup.md`), `gate-windows` nur `cargo clippy` und `cargo test --workspace --all-features --no-fail-fast`
(`target/` und `TEMP` auf einem Dev Drive):

- `gate-macos`: bei jedem Pull Request und Push auf `main`. Das Label `ci-skip-macos` überspringt ihn für Iterationen, die nur
  Windows betreffen (macOS-Minuten zählen zehnfach); das Label muss vor dem Merge wieder weg.
- `gate-windows`: auf `main`, von Hand (`workflow_dispatch`) und auf Pull Requests mit dem Label `windows`.

**`release.yml`** baut beide Bundles und prüft sie (`check-bundle.sh`, `check-bundle-windows.sh`, `check-no-rom-data.sh`,
Rauchtest). Er startet auf drei Wegen:

- Tag `v*`: das echte Release. Der Job `version` prüft die Fassung gegen den Tag, danach bauen beide Bundles, und `publish`
  legt einen **Entwurf** an mit `Gen4-Studio_<Fassung>_aarch64.dmg`, `Gen4-Studio_<Fassung>_x64-setup.exe` und
  `SHA256SUMS.txt`; als Text dient der Abschnitt der Fassung aus `CHANGELOG.md`. Einen Tag mit `-rc` legt er als Pre-Release an. Der Nutzer prüft den Entwurf
  und veröffentlicht ihn von Hand.
- `workflow_dispatch` mit `dry_run` (Standard `true`, erst nach dem Merge auf `main`): der Probelauf, alles ohne Release. Ein
  Aufruf mit `dry_run=false` scheitert im Job `version`: ein Release entsteht nur durch das Pushen eines Tags.
- Das Label `release-dry-run` auf einem Pull Request: derselbe Probelauf vor dem Merge. Jedes andere Label startet nichts.

`publish` läuft nur bei einem Tag-Push. Den Probelauf von Zeit zu Zeit auf `main` laufen lassen: Caches gelten nur für
denselben Ref oder den Standard-Branch, ein Tag-Lauf liest also, was `main` gespeichert hat.

**Versionen:** Die Fassung steht an einer Stelle, `version` in `[workspace.package]` der `Cargo.toml`; `apps/studio/package.json`
trägt dieselbe Zahl (Tauri liest sie von dort). `scripts/version.sh` druckt sie (`version.sh`), setzt sie (`version.sh set <Fassung>`,
schreibt auch `Cargo.lock`) und prüft sie (`version.sh check [<Tag>]`: beide Stellen gleich, gültiges SemVer, ein Abschnitt
in `CHANGELOG.md`, der Tag ist `v<Fassung>`). Tags heißen `vX.Y.Z` und `vX.Y.Z-rc.N`; für `-rc` gilt der Abschnitt der Fassung ohne Suffix.
Ablauf: `version.sh set`, Abschnitt in `CHANGELOG.md` ergänzen, mergen, `git tag vX.Y.Z`, Tag pushen, Entwurf prüfen und
veröffentlichen.

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

`release.yml` enthält den Schritt `sign-notarize` mit `if: false`. Mit einer Developer ID würde er so aktiviert:

1. Apple Developer Program (kostenpflichtig) beitreten und ein Zertifikat „Developer ID Application“ erzeugen.
2. Das Zertifikat als `.p12` exportieren und als Base64 in das Geheimnis `APPLE_CERTIFICATE` legen, dazu sein Passwort
   (`APPLE_CERTIFICATE_PASSWORD`) und den Namen der Identität (`APPLE_SIGNING_IDENTITY`, etwa „Developer ID Application: Name (TEAMID)“).
3. Notarisierung: Apple-ID (`APPLE_ID`), ein App-spezifisches Passwort (`APPLE_PASSWORD`, nicht das Anmeldepasswort; alternativ ein
   API-Schlüssel) und die Team-ID (`APPLE_TEAM_ID`).
4. In `release.yml` `if: false` auf `true` setzen und die sechs `env:`-Zeilen in den Schritt „Bundle (app, dmg)“ legen: Der Tauri-Bundler
   importiert das Zertifikat selbst in einen temporären Schlüsselbund, signiert das `.app` und ruft `notarytool` auf, sobald die
   Variablen gesetzt sind. Der Schritt `sign-notarize` steht für das Notarisieren des DMG (`xcrun notarytool submit … --wait`) und das
   Anheften des Tickets (`xcrun stapler staple`).

Unter Windows bräuchte es ein Code-Signing-Zertifikat; ohne eines bleibt die Warnung von SmartScreen. Auto-Update gibt es noch nicht.

## Bekannte Grenzen

- Beide Pakete sind nicht mit einer Developer ID signiert und nicht notarisiert (macOS ad-hoc, Windows gar nicht); die Wege
  stehen oben.
- Nur macOS arm64 und Windows x64; Linux ist kein Ziel dieser Phase.
- Auto-Update und Signatur über eine Developer ID folgen später.
