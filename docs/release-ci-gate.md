# Release und CI: Gate prüfen

Stand: 2026-11-05. Branch `release-ci` (Worktree `.claude/worktrees/release-ci`), PR #1, nicht nach `main` gemergt. **Gate offen.** Dieses Dokument ist das Gerüst: bekannte Messwerte stehen schon da, alles andere ist `offen` und wird beim Lauf eingetragen.

Die Phase bringt ein privates GitHub-Remote mit CI, die nur die hermetischen Schritte fährt (`ci.yml`, `G4_GATE_HERMETIC=1`), den Release-Workflow (`release.yml`: DMG für macOS arm64, NSIS-Installer für Windows x64, Entwurf mit Prüfsummen), Versionierung (`scripts/version.sh`, `CHANGELOG.md`), Lizenzhinweise im Paket, das Windows-Bundle und die Prüfung des gepackten Programms ohne ROM (`studio doctor`, `scripts/smoke-packaged.sh`). Grenze der Phase: keine ROMs, Spielstände oder Ableitungen nach GitHub, weder als Datei, Cache, Artefakt, Log noch Geheimnis. Spezifikation: [2026-11-04-release-ci-design.md](superpowers/specs/2026-11-04-release-ci-design.md), §10 (Gate) und §8 (Prüfung des Pakets); Plan: [2026-11-04-release-ci.md](superpowers/plans/2026-11-04-release-ci.md); Beschreibung der Pakete: [release.md](release.md).

Wer was prüft: **Agent** (Läufe auf dem Runner und lokal auf dem Mac), **Hand** (nur der Nutzer, Abschnitt „Hand“). Kontingent: macOS-Minuten zählen zehnfach. Entwickelt und geprüft wird zuerst lokal auf dem Mac; ein Lauf auf dem Runner prüft nur, was lokal nicht geht (Windows, den Runner selbst, das DMG mit `CI=true`).

## Vorbereitung

1. Privates Remote auf GitHub, Branch `main`, PR #1 für `release-ci`.
2. Vor dem ersten Push: `bash scripts/check-no-rom-data.sh --history origin/main..main` und `--history origin/main..release-ci` (Ergebnis unten, G7).
3. Fassungen der Werkzeuge aus `scripts/tool-versions.env`; Actions nur aus `.github/actions-allowlist.txt`.
4. Labels auf dem PR: `windows` (startet `gate-windows`), `ci-skip-macos` (überspringt `gate-macos`, vor dem Merge entfernen), `release-dry-run` (Probelauf von `release.yml`).

## Punkte

| Punkt | Inhalt (Spezifikation §10) | Wer | Ergebnis | Datum |
|---|---|---|---|---|
| G1 | Ein Push auf `main` ergibt einen grünen `ci.yml`-Lauf mit `gate-macos` und `gate-windows`. `gate-macos` fährt dieselbe Liste wie `G4_GATE_HERMETIC=1 bash scripts/gate.sh` lokal, und die Tabelle beider Läufe ist gleich. | Agent | offen. Vorab: `gate-macos` auf dem PR grün, Lauf 41936883009 (kalt 18 min 21 s, Gate 1 020 s, davon `cargo test` 726 s); lokale Tabelle: siehe „Zeiten und Minuten“. Tabellenvergleich und `gate-windows` auf `main`: offen | – |
| G2 | Testbranch `ci-rot` (Draft-PR auf `main`) mit zwei Commits: einer verletzt `cargo fmt`, einer ist ein absichtlich roter `pnpm test`. Beide ergeben einen roten Lauf am erwarteten Schritt; danach wird der Branch gelöscht. | Agent | offen | – |
| G3 | Der Tag `v0.1.0-rc.1` erzeugt einen Pre-Release-Entwurf mit `Gen4-Studio_0.1.0-rc.1_aarch64.dmg`, `…_x64-setup.exe` und `SHA256SUMS.txt`. `check-bundle.sh`, `check-bundle-windows.sh`, `check-no-rom-data.sh` und der Rauchtest sind in beiden Bundle-Jobs grün; die Prüfsummen passen zu den Dateien. | Agent | offen | – |
| G4 | Der Probelauf (Label `release-dry-run` auf dem PR, nach dem Merge `workflow_dispatch` mit `dry_run`) baut und prüft alles ohne Release. | Agent | Auf dem PR grün: Probelauf 41960501794 (Label `release-dry-run`), beide Bundle-Jobs mit `check-bundle*`, `no rom data` und Rauchtest; Artefakte heruntergeladen, `check-no-rom-data.sh` darüber: 14 Dateien ok. Nach dem Merge `workflow_dispatch`: offen | – |
| G5 | Die Abnahme aus Spezifikation §8.2 (ROM, lokal auf dem Mac) ist mit dem heruntergeladenen DMG bestanden. | Agent | offen | – |
| G6 | Der Windows-Installer installiert auf dem Runner still. `studio doctor` und der Rauchtest sind grün; `find_apicula` und `find_rotom_lsp` finden die `.exe`. | Agent | Probelauf 41960501794: der Installer installiert still nach `RUNNER_TEMP\g4s`, `studio doctor` (apicula und rotom-lsp gefunden und gestartet, nitroarc geladen) und der Rauchtest sind grün. `find_apicula`/`find_rotom_lsp` nutzen dieselbe Suche (`sidecar::find_in`) wie `doctor`; ein eigener Aufruf über MCP fehlt | – |
| G7 | Kein Artefakt, kein Cache, kein Log enthält ROM-Daten: `check-no-rom-data.sh` grün über `--history origin/main..main` vor dem ersten Push, über die heruntergeladenen Artefakte und über einen heruntergeladenen Cache; die Suche in den Logs (§7.3) findet nichts. | Agent | Teil „vor dem ersten Push“ erfüllt: `origin/main..main` und `origin/main..release-ci` ohne Treffer (2 521 und 2 549 Dateien, `target/agent/release-ci/history-check-prepush.txt`). Artefakte, Cache, Logs: offen | – |
| G8 | `scripts/gate.sh` ist lokal voll grün auf dem Merge-Stand; `cargo deny` und `scripts/check-licenses-npm.sh` grün mit allen neuen Abhängigkeiten; `scripts/check-actions.sh` grün. | Agent | offen. Vorab: hermetischer Lauf lokal grün (siehe „Zeiten und Minuten“); voller Lauf auf dem Merge-Stand: offen | – |

## Zeiten und Minuten

Gemessen:

| Lauf | Wo | Dauer | Bemerkung |
|---|---|---|---|
| `G4_GATE_HERMETIC=1 bash scripts/gate.sh` | lokal, Mac | 2 530 s | unter Last (rund 20); `cargo test` 2 287 s, `mcp` 27 s, `pnpm test` 147 s, `g4emu unit` 20 s, jeder andere Schritt unter 25 s |
| `gate-macos`, Lauf 41936883009 | Runner `macos-14`, kalt | 18 min 21 s (Job); Gate 1 020 s | `cargo test` 726 s |
| `gate-macos` warm | Runner | offen | – |
| `gate-windows` (Probe, Lauf 41936497980), MSVC | Runner `windows-latest`, kalt | 16 min 7 s | `cargo build -p studio` 657 s; `cargo clippy` scheiterte an ungenutzten Imports unter Windows (danach behoben) |
| `gate-windows` (Probe), GNU | Runner, kalt | 22 min 39 s | Link von `studio` scheitert (`ld returned 8`); Ziel verworfen, siehe [release.md](release.md) |
| `gate-windows` MSVC, Lauf 41942261090 | Runner, ohne Dev Drive | 90 min (Abbruch nach dem ersten roten Test-Binary) | `cargo clippy` 451 s grün; `cargo test` 4 312 s, Kernel-Unit-Tests 2 849 s, 14 Fehler |
| `gate-windows` MSVC, Lauf 41965391702 | Runner, Dev Drive, ohne Debug-Info | 66 min | `cargo clippy` 489 s; `cargo test --no-fail-fast` 3 101 s, 13 Fehler in 8 Binaries (danach behoben) |
| `gate-windows` MSVC, Lauf 41977356870 | Runner, Dev Drive, warm | 57 min, grün | `cargo clippy` 107 s; `cargo test --no-fail-fast` 2 749 s (64 Test-Binaries); Cache laden 135 s, speichern 293 s |
| `bundle-macos`, Probelauf 41960501794 | Runner `macos-14`, kalt | 10 min | `Bundle (app, dmg)` 480 s inklusive apicula und rotom-lsp; Rauchtest grün |
| `bundle-windows`, Probelauf 41960501794 | Runner `windows-latest`, kalt | 64 min | `Bundle (nsis)` 2 984 s, synthetisches Projekt 453 s; `check-bundle-windows`, `no rom data` und Rauchtest grün |

Verbrauchte Minuten bis Lauf 41977356870 (Job-Dauer aufgerundet; macOS zählt zehnfach, Windows doppelt, Linux einfach):

| System | Minuten | Faktor | abgerechnet |
|---|---|---|---|
| macOS | 19 + 10 | 10 | 290 |
| Windows | 23 + 17 + 90 + 22 + 64 + 12 + 65 + 57 | 2 | 700 |
| Linux | 1 | 1 | 1 |
| Summe | | | 991 |

Der Schlusslauf (ci ohne `ci-skip-macos` und `release-dry-run`) kommt dazu; Zahlen im Bericht des Agenten.

Am Ende der Phase: Gesamtverbrauch und Rest des Monatskontingents: offen.

## Hand

Nur der Nutzer (Spezifikation §8.3). Ein frisches Benutzerkonto legt der Agent nicht an; das ist eine Änderung der Systemeinstellungen.

| Punkt | Ergebnis | Datum |
|---|---|---|
| macOS: das DMG auf einem frischen Benutzerkonto ohne Entwicklungswerkzeuge: Projekt öffnen, Kollision ändern, ROM bauen, in melonDS spielen, Script mit Sprachdiensten, texturierte Modelle; dazu der Gatekeeper-Weg (Rechtsklick, Öffnen) | offen | – |
| Windows: der Installer auf einem Windows-PC mit denselben Schritten (ohne den Spielmodus live, der ist macOS-only); dazu die Warnung von SmartScreen („Weitere Informationen“, „Trotzdem ausführen“) | offen | – |
| Entwurf auf GitHub ansehen und veröffentlichen (erst nach dem Gate, nie automatisch) | offen | – |

## Befunde

| Nr. | Schwere | Punkt | Befund | Fix |
|---|---|---|---|---|
| 1 | – | NSIS und Vorabversion (Spezifikation §6.4) | NSIS nimmt `0.1.0-rc.1` an (Probelauf 41960501794); keine Ersatzfassung nötig, siehe [release.md](release.md) | – |
| 2 | – | DMG auf dem Runner | Tauris DMG mit `CI=true` gelingt auf `macos-14` (Zeile `dmg: tauri`); der Rückfall `hdiutil` wurde nicht gebraucht | – |
| 3 | mittel | Windows-Tests | Ziel MSVC (GNU linkt nicht). Der erste volle Lauf fand 14 Fehler nur unter Windows (Pfadtrenner in Schlüsseln und Meldungen, Änderungserkennung ohne ctime, Testannahmen); behoben. Die Tests brauchten auf der Runner-Platte 47 min für die Kernel-Unit-Tests allein; mit Dev Drive (ReFS) 265 s | – |

## Ergebnis

Gate offen. Der Orchestrator trägt die Ergebnisse der Punkte G1 bis G8, die Zeiten und die Hand ein.
