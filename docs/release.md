# Release: macOS-App-Bundle

Das Bundle enthält drei Fremdkomponenten, damit die App ohne den Entwicklungsbaum läuft:

- `Contents/Frameworks/libnitroarc_ffi.dylib`: die LGPL-lizenzierte nitroarc-Bibliothek, die uxie beim Release-Build als
  Shared Library erzeugt. Sie bleibt eine separate Datei (kein statisches Linken), damit sie austauschbar ist.
- `Contents/MacOS/apicula`: der apicula-Sidecar (NSBMD zu GLB). Tauri entfernt beim Bundeln den Target-Triple-Suffix
  (`apicula-aarch64-apple-darwin` wird zu `apicula`). Die App findet ihn dort über Eintrag 2 der apicula-Suche
  (siehe `docs/dev-setup.md`), `G4_APICULA` muss nicht gesetzt sein.
- `Contents/MacOS/rotom-lsp`: der Sprachserver der Scripts (Git-Tag `v0.7.0`, wie die rotom-Bibliothek). Er hat den rpath
  `@executable_path/../Frameworks` und lädt dieselbe `libnitroarc_ffi.dylib` aus `Contents/Frameworks`. Die App findet ihn neben dem
  Studio-Binary (Eintrag 2 der Suche in `docs/dev-setup.md`).

## Voraussetzungen

- apicula gebaut unter `tools/apicula/bin/apicula` (siehe `docs/dev-setup.md`).
- `rotom-lsp` unter `tools/rotom/bin/rotom-lsp`; fehlt es, baut `prepare-bundle.sh` es aus dem Git-Tag (Netz beim ersten Mal).
- `export PATH="/opt/homebrew/opt/rustup/bin:$PATH"` in der Shell, aus der gebaut wird (`tauri build` braucht `cargo`).
- Die Datei `.cargo/config.toml` (CFLAGS für nitroarc auf neuen macOS-SDKs) liegt im Repository und gilt automatisch.

## Schritte

```sh
cd apps/studio
pnpm bundle                      # prepare-bundle.sh, danach tauri build
bash scripts/check-bundle.sh     # Prüfung des Bundles, Ausgabe: bundle ok
```

`pnpm bundle` macht im Einzelnen:

1. `scripts/prepare-bundle.sh` baut `cargo build --release -p studio` (dabei entsteht `target/release/libnitroarc_ffi.dylib`)
   und kopiert apicula nach `src-tauri/binaries/apicula-<Target-Triple>`, `rotom-lsp` nach
   `src-tauri/binaries/rotom-lsp-<Target-Triple>` (mit rpath, neu signiert) sowie die dylib nach
   `src-tauri/frameworks/libnitroarc_ffi.dylib`. Fehlt eine Quelle, bricht das Skript mit einer Fehlermeldung ab.
   Beide Zielordner sind per `.gitignore` ausgeschlossen.
2. `tauri build` (nur das `.app`-Ziel, kein DMG) legt `externalBin` nach `Contents/MacOS/` und `macOS.frameworks` nach
   `Contents/Frameworks/`. Das Ergebnis liegt in `target/release/bundle/macos/Gen4 Studio.app`.

Normale Builds (`cargo build`, `cargo test`, `clippy`, `tauri dev`) brauchen `prepare-bundle.sh` nicht: `build.rs` legt
leere Platzhalter in `binaries/` und `frameworks/` an, weil Tauri die Dateien aus `externalBin`/`frameworks` bei jedem Build
erwartet. `find_apicula` und `find_rotom_lsp` ignorieren eine leere Datei.

## Prüfbefehle

`scripts/check-bundle.sh` prüft:

- `Contents/MacOS/apicula` ist ausführbar und startet (`--help`).
- `Contents/MacOS/rotom-lsp` ist ausführbar, hat den rpath `@executable_path/../Frameworks` und startet ohne dyld-Fehler.
- `Contents/Frameworks/libnitroarc_ffi.dylib` existiert und hat den Install-Namen `@rpath/libnitroarc_ffi.dylib`.
- Das Studio-Binary hat den rpath `@executable_path/../Frameworks` und lädt die dylib über `@rpath/...`;
  kein absoluter Pfad in `target/` ist eingebunden.

Zusätzlich von Hand:

```sh
APP="target/release/bundle/macos/Gen4 Studio.app"
otool -L "$APP/Contents/MacOS/studio"      # Zeile mit @rpath/libnitroarc_ffi.dylib
env -u G4_APICULA "$APP/Contents/MacOS/studio"   # startet ohne dyld-Fehler
DYLD_PRINT_LIBRARIES=1 "$APP/Contents/MacOS/studio" 2>&1 | grep nitroarc   # Pfad liegt in Contents/Frameworks
```

Abnahme von Hand: die `.app` aus Finder starten, ein Projekt (Kopie) öffnen und einen Header mit Modellen laden
(`G4_APICULA` nicht gesetzt); die Modelle kommen dann aus dem gebündelten apicula.

## Bekannte Grenzen

- Das Bundle ist weder signiert noch notarisiert. Beim ersten Start verlangt Gatekeeper eine Freigabe
  (Rechtsklick auf die App, Öffnen; oder Systemeinstellungen, Datenschutz & Sicherheit, Trotzdem öffnen).
  Wurde die App heruntergeladen oder kopiert, hilft `xattr -dr com.apple.quarantine "Gen4 Studio.app"`.
- Nur die Architektur des Build-Rechners (aktuell `aarch64-apple-darwin`); ein Universal-Binary ist nicht vorgesehen.
- Es wird nur das `.app` gebaut. Das DMG-Ziel benötigt Finder-Automation per AppleScript und scheitert in
  nicht-interaktiven Umgebungen.
- Mindestsystem macOS 13.0.
