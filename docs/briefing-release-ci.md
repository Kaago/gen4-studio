# Briefing: Release und CI

Stand: 2026-11-04. Repo: [Kaago/gen4-studio](https://github.com/Kaago/gen4-studio) (privat, seit 2026-11-04).

## Ziel

Gen4 Studio wird eine installierbare App, die ohne Entwicklungsbaum läuft, und jeder Push auf `main` wird automatisch geprüft. Bisher läuft das Studio nur aus dem Quellcode auf dem Rechner des Nutzers. Die gepackte App wurde in keinem Gate geprüft („gepackte App auf Wunsch ausgelassen“ in 3a, 3b.1, 3b.2, 3c).

**Mehrwert:** Andere können das Studio installieren und testen, Regressionen fallen beim Push auf statt erst im stundenlangen `scripts/gate.sh --full`, und die Roadmap-Zusage aus „Querschnitt“ wird eingelöst („Builds für macOS (arm64) und Windows (x64) … folgen mit einem Remote“).

## Ausgangslage

- **Bundle für macOS existiert:** `cd apps/studio && pnpm bundle`, danach `bash scripts/check-bundle.sh`. Beschreibung in `docs/release.md`. Ergebnis ist `target/release/bundle/macos/Gen4 Studio.app`. Es wird nur das Ziel `app` gebaut, kein DMG.
- **Fremdkomponenten im Bundle:**
  - `apicula`: Sidecar, aus dem Quellcode gebaut, Commit `3d4e91e…`.
  - `rotom-lsp`: Sidecar, Tag `v0.7.0`, mit rpath und ad-hoc signiert.
  - `libnitroarc_ffi.dylib`: LGPL, dynamisch gelinkt, liegt in `Contents/Frameworks`.
  - Vorbereitung durch `apps/studio/scripts/prepare-bundle.sh`. `build.rs` legt sonst leere Platzhalter an.
- **Lokales Gate statt CI:** `scripts/gate.sh` fährt fmt, clippy, `cargo deny`, npm-Lizenzen, `cargo test`, `mcp`, `pnpm test`, `tokens`, `styles`, `depth`, `pnpm build`, „dist has no bridge“ und `g4emu unit`: das schnelle Gate, vor jedem Merge, ohne ROM. Mit `--full` folgen die **gated Tests gegen die ROM-Projekte** unter `$G4_ROMS_DIR`, mit `g4emu integration` und den Timings: der Volllauf, nachts und auf Abruf, knapp drei Stunden für drei Projekte (gemessen 9 954 s); er hinterlässt je Commit eine Ergebnisdatei (`docs/dev-setup.md`, „Gate-Skript“ und „Nachtlauf einrichten“).
- **Netz beim Bauen:** rotoms `build.rs` lädt bei jedem sauberen Build `db-latest.zip`.
- **Nur macOS:** `.cargo/config.toml` setzt `CFLAGS` für nitroarc auf neuen macOS-SDKs. Der Spielmodus live (3d.9) gibt es nur unter macOS.
- **App-Daten:** Version `0.1.0` in `tauri.conf.json`, Identifier `dev.gen4studio.app`.

## Harte Grenzen

- **Keine ROMs, Spielstände oder ROM-Ableitungen nach GitHub.** Das gilt auch nicht als Secret, Cache, Artefakt oder Test-Fixture. Die gated Tests, `g4emu integration` und die Szenarien bleiben lokal (`scripts/gate.sh --full`). CI fährt nur die hermetischen Schritte.
- Das Release-Artefakt enthält keine Spieldaten. Die Befehlsdatenbank `crates/g4-kernel/metadata/scrcmd/` und die Metadaten dürfen mit, weil sie schon im Repo liegen.
- Für jede neue Abhängigkeit, auch für Actions, gilt das Lizenz-Audit (`cargo deny`, `scripts/check-licenses-npm.sh`). Die LGPL-Dylib bleibt eine separate, austauschbare Datei.
- Das Repo ist privat. Kein Wechsel auf öffentlich ohne ausdrückliche Zustimmung des Nutzers.

## Lieferumfang (Vorschlag, im Brainstorming zu bestätigen)

1. **CI auf GitHub Actions.**
   - Workflow `ci.yml` für Push und Pull Request auf `main`.
   - Inhalt: die hermetischen Schritte von `gate.sh` auf `macos-14` (arm64). Ideal ist ein Schalter in `gate.sh`, etwa `G4_GATE_HERMETIC=1`, der nach `g4emu unit` endet, damit lokal und CI dieselbe Liste fahren.
   - Caches für Cargo und pnpm, dazu `uv`.
2. **Release-Workflow.**
   - Auslöser ist ein Tag `v*`.
   - Er baut apicula und rotom-lsp in festen Versionen (gecacht), dann `pnpm bundle` und `check-bundle.sh`.
   - Er erzeugt ein DMG und lädt es als GitHub Release hoch, als Entwurf.
3. **Windows x64**, als zweiter Schritt oder Teilphase:
   - `prepare-bundle.sh` braucht ein Windows-Gegenstück: `nitroarc_ffi.dll`, `apicula.exe`, `rotom-lsp.exe`.
   - Die Ziele `nsis` oder `msi` in `tauri.conf.json` kommen dazu.
   - Pfade und `CFLAGS` sind zu prüfen.
4. **Versionierung.** Eine Quelle für die Version (`tauri.conf.json`, `Cargo.toml`, `package.json`), dazu ein Changelog. Das kann auf 4.5 „Veröffentlichen“ aufbauen.
5. **Prüfung der gepackten App** als eigener Gate-Teil. Das Bundle startet auf einem frischen macOS-Nutzerkonto ohne `tools/` und ohne `G4_*`. Es öffnet ein Projekt, baut ein ROM und startet melonDS. Das bedeutet: die offenen Punkte „gepackte App“ aus 3a bis 3c nachholen.

## Offene Entscheidungen (beim Nutzer)

- **Signieren und Notarisieren unter macOS.** Dafür braucht es eine Apple Developer ID (99 $/Jahr). Ohne sie muss jeder Nutzer Gatekeeper umgehen (Rechtsklick › Öffnen).
- **Auto-Update** (Tauri-Updater, laut Roadmap „ab Phase 3b“): Der Updater braucht ein eigenes Schlüsselpaar und eine öffentlich erreichbare Update-Quelle. Aus einem privaten Repo geht das nicht ohne Token. Möglich sind: das Release-Repo öffentlich machen, ein eigener Host, oder Auto-Update später.
- **Actions-Kontingent:** Ein privates Repo hat 2 000 Minuten pro Monat im Free-Plan, macOS-Minuten zählen zehnfach. CI deshalb schlank halten: Pfadfilter, `concurrency` mit Abbruch, Caches. Alternative ist ein Self-hosted Runner auf dem Mac des Nutzers. Damit wären auch die gated Tests möglich, aber die ROMs müssten dafür auf dem Runner liegen.
- **Linux** nur „best effort“: weglassen oder ohne Gewähr mitbauen.

## Gate (Vorschlag)

- Ein Push auf `main` ergibt einen grünen CI-Lauf mit denselben hermetischen Schritten wie `scripts/gate.sh`. Ein absichtlich kaputter Commit auf einem Testbranch ergibt einen roten Lauf.
- Ein Tag `v0.1.0-rc.1` erzeugt einen Release-Entwurf mit DMG. `check-bundle.sh` ist im Workflow grün.
- Das heruntergeladene DMG läuft auf einem frischen Nutzerkonto ohne Entwicklungswerkzeuge:
  - Projekt öffnen, Kollision ändern, ROM bauen, Spiel in melonDS starten.
  - Script mit Sprachdiensten öffnen (`rotom-lsp` gefunden).
  - 3D-Modelle texturiert (`apicula` gefunden).
- Kein Artefakt, kein Cache und kein Log enthält ROM-Daten. Stichprobe: `.nds`, `*_DSPRE_contents` und Spielstände suchen.
- `scripts/gate.sh` bleibt lokal grün.

## Vorgehen

Wie jede Teilphase: Brainstorming (Skill), Entscheidungen zu den offenen Punkten vom Nutzer holen, Spezifikation, Plan, Umsetzung in eigenem Worktree, Gate, Merge auf `main`, Roadmap-Zeile ergänzen. Die Roadmap-Sätze „Es gibt kein Remote und keine CI“ in `docs/ROADMAP.md` (Querschnitt) und `docs/dev-setup.md` sind dabei zu aktualisieren.
