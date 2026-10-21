# Entwicklungs-Setup

## apicula (NSBMD zu GLB)

Die Matrix-Ansicht zeigt die Map-Modelle (NSBMD) als GLB an. Die Konvertierung übernimmt
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

### Gated Tests

`cargo test -p g4-kernel --test export` führt den Vanilla-Test nur aus, wenn `G4_TEST_PROJECT` (ds-rom-Projektordner)
und `G4_APICULA` gesetzt sind. Das Projekt wird dabei nur gelesen; es wird in ein Temp-Verzeichnis kopiert.

### Cache

Konvertierte Modelle liegen unter `<Projekt>/.g4/models/<hash>.glb`. Der Schlüssel ist der xxh3-64-Hash über
das NSBMD der Karte, gefolgt vom NSBTX-Texturset (`AreaData[area_data_id].map_tileset`), als 16 Hex-Zeichen.
Ändert sich das Modell oder das Tileset, entsteht ein neuer Eintrag; alte Einträge bleiben liegen und dürfen
jederzeit gelöscht werden (`.g4/` ist reiner Cache). Gibt es den Area-Data- oder Tileset-Eintrag nicht,
wird das Modell ohne Texturen konvertiert.
