# Release und CI: Gate

Stand: 2026-11-05. Auf `main` gemergt: `ebf44628` (Phase, PR [#1](https://github.com/Kaago/gen4-studio/pull/1)), `933cf092` (Versionstests), `09cb649a` (vollständige ad-hoc-Signatur, Befund B1), `1a7c112c` (ohne automatische CI, Release lokal). Release-Entwürfe `v0.1.0-rc.1` (Befund B1, überholt) und `v0.1.0-rc.2` (Stand des Gates).

Die Phase bringt:

- ein privates GitHub-Remote mit CI (`ci.yml`), die nur die hermetischen Schritte fährt (`G4_GATE_HERMETIC=1`), seit der Umstellung nur noch von Hand (siehe Abschnitt Umstellung);
- den Release-Weg: DMG für macOS arm64 und Entwurf mit Prüfsummen lokal (`scripts/release-local.sh`), NSIS-Installer für Windows x64 von Hand mit `release.yml`;
- Versionierung mit `scripts/version.sh` und `CHANGELOG.md`;
- Lizenzhinweise im Paket;
- das Windows-Bundle mit dem Ziel MSVC;
- die Prüfung des gepackten Programms ohne ROM (`studio doctor`, `scripts/smoke-packaged.sh`).

Grenze der Phase: keine ROMs, Spielstände oder Ableitungen nach GitHub.

- Spezifikation: [2026-11-04-release-ci-design.md](superpowers/specs/2026-11-04-release-ci-design.md)
- Plan: [2026-11-04-release-ci.md](superpowers/plans/2026-11-04-release-ci.md)
- Pakete: [release.md](release.md)

## Punkte

| Punkt | Inhalt (Spezifikation §10) | Ergebnis |
|---|---|---|
| G1 | Ein Push auf `main` ergibt grüne `gate-macos` und `gate-windows`; dieselbe Liste wie `G4_GATE_HERMETIC=1 bash scripts/gate.sh` lokal | **bestanden.** Lauf 41998736234 auf `933cf092`: macOS 17 min, Windows 56 min. Lauf 42010447761 auf `09cb649a`: macOS 19 min, Windows 62 min. Die Schrittliste von `gate-macos` ist gleich der lokalen hermetischen Tabelle (fmt, clippy, deny, npm licenses, script tests, actions, no rom data, version, cargo test, mcp, pnpm test, tokens, styles, depth, pnpm build, dist has no bridge, g4emu unit). |
| G2 | Testbranch `ci-rot` rot am erwarteten Schritt, danach gelöscht | **bestanden mit Abweichung.** Lauf 41998492509 (PR [#2](https://github.com/Kaago/gen4-studio/pull/2)) rot an `cargo fmt --check` nach 45 s; PR geschlossen, Branch gelöscht. **Abweichung:** Den zweiten roten Commit (`pnpm test`) gab es nicht. Er hätte rund 16 macOS-Minuten (160 abgerechnet) gekostet, weil `pnpm test` hinter `cargo test` liegt. Dass `gate.sh` an einem roten Schritt abbricht und die Tabelle `FAILED` zeigt, belegt derselbe Mechanismus in G2 und der lokale rote Lauf von G8 (Lauf 2). |
| G3 | Tag `v0.1.0-rc.N` erzeugt einen Pre-Release-Entwurf mit DMG, Installer und `SHA256SUMS.txt`; alle Prüfungen in beiden Bundle-Jobs grün | **bestanden** mit `v0.1.0-rc.2`, Lauf 42010450603. Die Jobs: `version`, `bundle-macos` 11 min, `bundle-windows` 67 min (kalt: Caches aus PR-Läufen sind für Tags nicht sichtbar), `publish`. Im Log stehen `bundle ok`, `bundle ok (windows)`, `no rom data: 12 files ok` (macOS) und `27 files ok` (Windows) sowie `smoke ok` auf beiden Plattformen. Die heruntergeladenen Dateien: `shasum -a 256 -c SHA256SUMS.txt` OK. `rc.1` (Lauf 41998479850) war ebenfalls grün, trägt aber B1. |
| G4 | Probelauf ohne Release | **bestanden.** Auf dem PR (Label `release-dry-run`): Lauf 41960501794 und Schlusslauf 41983722455 auf `83558b64` grün, beide ohne `publish`. Nach der Umstellung der neue Weg: `release-local.sh v0.1.0-rc.2 --dry-run` lokal grün (DMG 21 362 214 B, SHA-256 `bd51ce61…fef54`, `bundle ok`, `no rom data`, `smoke ok`). Der Windows-Teil per `workflow_dispatch` mit `dry_run=true`: Lauf 42027703959 auf `ci-local` grün, `bundle-windows` 68 min (kalt), `no rom data: 12 files ok`, `smoke ok`, `publish-windows` übersprungen. |
| G5 | Abnahme §8.2 mit dem heruntergeladenen DMG auf dem Mac | **bestanden** mit `rc.1`, DMG SHA-256 `0e8e6451…9879`, byte-gleich mit der Release-Datei. Siehe Abschnitt G5. Auf `rc.2` wiederholt: Signatur gültig, `doctor: ok`, Start und Ende über LaunchServices, Rauchtest im Release-Lauf. |
| G6 | Windows-Installer still installiert; `studio doctor` und Rauchtest grün; die Sidecars werden als `.exe` gefunden | **bestanden** in den Läufen 42010450603 (`rc.2`) und 41998479850 (`rc.1`). Still installiert nach `RUNNER_TEMP\g4s`, `check-bundle-windows.sh` meldet `bundle ok (windows)`. `doctor` findet `apicula.exe` und `rotom-lsp.exe` mit `sidecar::find_in`, derselben Suche wie `find_apicula` und `find_rotom_lsp`, und startet beide. Der Rauchtest ist grün. |
| G7 | Kein Artefakt, Cache oder Log mit ROM-Daten | **bestanden mit Abweichung.** Siehe Abschnitt G7. |
| G8 | `scripts/gate.sh` voll grün auf dem Merge-Stand | **bestanden** auf `09cb649a` (Lauf 3 und Wiederholung der Schritte nach dem Download-Timeout). Siehe Abschnitt G8. |

## G5: Abnahme des gepackten Programms (Mac, mit ROM)

Ein Agent prüfte das DMG aus dem Lauf des Tags `v0.1.0-rc.1`. Belege liegen unter `target/agent/release-ci-gate/g5/`. Mitschnitte mit ROM-Daten gibt es nur im Scratchpad.

Installation und Start:

- App nach `~/Applications` kopiert und eine Quarantäne wie nach einem Download gesetzt. Den dokumentierten Weg `xattr -dr` gegangen.
- Gestartet mit `env -i /usr/bin/open`. Die App hat dann nur die Umgebung von launchd: kein Shell-`PATH`, kein `G4_*`, kein `tools/`.
- Das Fenster zeigt Start („Willkommen zurück“), das Symbol des Bundles ist die Marke (`icon.icns` mit dem SHA-256 aus dem Repo).

`studio doctor` aus dem Bundle:

- apicula und rotom-lsp liegen in `Contents/MacOS` und starten.
- git `/usr/bin/git`, melonDS `/Applications/melonDS.app`.
- nitroarc ist geladen, laut `DYLD_PRINT_LIBRARIES` aus `Contents/Frameworks`.

Kopflos mit `studio mcp` aus dem Bundle, auf einer frischen Kopie von Platin DE:

- **Kollision:** Zweiblattdorf (Karte 0) Zelle (16,16) von `[0,0]` auf `[0,128]` geändert. Gespeichert wurde genau 1 Byte.
- **ROM-Build:** `build_rom` in 43,2 s, SHA-1 `589f5d5b4d67f769c0e0a0491cfd945c4d767b8e`.
- **melonDS:** `run_rom` startet melonDS 1.1 mit der gebauten ROM bis zum Copyright-Bildschirm.
- **Sprachdienste:** `lsp_start` und `lsp_send` gibt es nur in der App. Ersatzbeleg ist ein LSP-Client gegen das gebündelte rotom-lsp: ohne Fehler 0 Diagnosen, `Unknown command 'G5GibtEsNicht'` bei einem falschen Befehl, 1 964 Einträge in der Completion.
- **Modelle:** `get_map_model_glb` für T01 liefert 19 Materialien, alle mit Textur. `last-tool` ist der xxh3-64-Hash des gebündelten apicula. Die Treffer im Cache kommen in einem neuen Prozess in 0,02 s.

## G7: ROM-Schutz

- **Historie vor jedem Push:** `check-no-rom-data.sh --history origin/main..main` ohne Treffer.
  - vor dem ersten Push: 2 521 Dateien, dazu `origin/main..release-ci` mit 2 549;
  - vor `ebf44628`: 2 701 Dateien;
  - vor `09cb649a`: 6 Dateien.
- **Release-Dateien:** `rc.1` und `rc.2` heruntergeladen, `check-no-rom-data.sh` meldet jeweils 3 Dateien ok, DMG eingehängt. Die Prüfsummen stimmen. Dazu die Artefakte des Probelaufs: 14 Dateien ok.
- **Logs:** Die Logs von fünf Läufen (CI auf `main`, Release `rc.1`, Schlussläufe des PR, `ci-rot`) wurden durchsucht. Es gibt 0 Treffer für `_DSPRE_contents`, `/Roms/`, `/_saves/`, `.sav`, `.nds` und `Pokemon - `. Zu `_testkopien` gibt es 16 Treffer, alle Namen synthetischer Tests (`bridge::roots::tests::…_testkopien…`, `test_testkopien_folder`).
- **Caches:** `gh cache list` zeigt nur `cargo-*`, `release-cargo-*`, `node-cache-*`, `setup-uv-*`, `msys2-pkgs-*` und `tools-*` (apicula und rotom-lsp).
- **Abweichung:** `gh` kann Actions-Caches nicht herunterladen, deshalb wurde kein Cache ausgepackt. Begründung: Auf den Runnern liegt nie eine ROM, das Repo enthält nachweislich keine, und CI fährt nur hermetische Schritte. Ein Cache kann also nichts aus einer ROM enthalten.

## G8: volles `scripts/gate.sh` auf dem Merge-Stand

| Lauf | Stand | Ergebnis |
|---|---|---|
| 1 | `ebf44628` | rot an `script tests` (53 s). Die Versionstests kopierten die echten Versionsdateien und erwarteten `0.1.0`; mit `0.1.0-rc.1` auf `main` wurden sie rot. Behoben in `933cf092`: Die Tests setzen ihre Kopie selbst auf `0.1.0`. |
| 2 | `933cf092` | rot an `cargo test (no ROM data)` nach 2 648 s, 790 von 791 Tests grün. `commands::models::tests::a_hanging_tileset_falls_back_to_untextured_without_marker_and_is_not_retried` („apicula did not finish within 1 s“) lief unter Last: Gleichzeitig liefen ein `pnpm bundle` und die Abnahme G5. Allein 5-mal grün, also ein Flake unter Last. |
| 3 | `09cb649a` | rot an `build graph (timings)` HeartGold DE nach 17 832 s. Alle Schritte davor grün, auch `gated tests (cargo test)` für HeartGold DE (5 361 s), HeartGold US (4 992 s) und Platin DE (4 547 s). Kein Testfehler: Das Build-Skript von `rotom` lädt im Release-Build eine Datei, beide Versuche liefen in einen Timeout („download failed: the timeout of the request was reached“). |
| 3, Rest | `09cb649a` | **ok** in 533 s, „rot im Gate-Lauf, ok in der Wiederholung“: die Schritte ab dem roten mit den Befehlen von `gate.sh` (`target/agent/gate-2026-11-05/gate-rest.sh`). `build graph (timings)` für HeartGold DE, HeartGold US und Platin DE ok, `gated tests (project start)` auf allen drei ROMs ok (je 7 Tests `project_start_`). Logs: `gate-full-3.log`, `gate-rest.log`. |

`ci-local` (`1a7c112c`) ändert an `gate.sh` nur den Kopfkommentar. Nach dem Merge grün: `script tests` (12), `actions` (28 uses), `check-no-rom-data.sh --history origin/main..main`.

## Zeiten und Minuten

- **gate-macos:** warm 17 bis 19 min, Gate rund 1 020 s, davon `cargo test` 726 s. Das Ziel der Spezifikation waren unter 15 min; es wird nicht erreicht.
- **gate-windows:** warm 56 bis 62 min. Ohne Dev Drive dauerten die Kernel-Unit-Tests allein 47 min.
- **bundle-macos:** 7 bis 11 min. **bundle-windows:** 25 min warm, 66 bis 67 min kalt. Ein Tag sieht die Caches aus PR-Läufen nicht.
- **Lokal:** hermetisch 2 530 bis 2 636 s unter Last.

Abgerechnete Minuten (macOS mal 10, Windows mal 2), aus den Job-Zeiten der API, aufgerundet:

| Abschnitt | abgerechnet |
|---|---|
| Umsetzung (PR #1, alle Läufe bis `83558b64`) | rund 1 524 |
| Orchestrator (`ci-rot`, abgebrochener Lauf, `rc.1`, `main` zweimal, `rc.2`) | 1 132 |
| Windows-Probelauf nach der Umstellung (Lauf 42027703959, 68 min) | rund 140 |
| **Summe** | **rund 2 800** |

Das ist mehr als die 2 000 Minuten des Free-Plans. Ein Billing-Fehler trat nicht auf; das Kontingent des Kontos ist nicht abfragbar (`gh` ohne Scope `user`). Ein Lauf auf `main` kostet rund 310 abgerechnete Minuten (macOS 190, Windows 120), ein Release rund 270.

Laut Abrechnung des Kontos (GitHub Pro, $18 Actions-Guthaben im Monat) waren nach den Läufen bis `rc.2` $15,25 verbraucht. Der Windows-Probelauf kam mit rund $1,10 dazu; geschätzt waren $0,70, aber er fand keinen Rust-Cache: Caches eines Branches sieht nur dieser Branch, und `ci-local` hatte keinen. Läufe ohne `--ref` laufen auf `main`; deren Cache sehen alle Refs, also auch spätere Läufe.

## Umstellung: ohne automatische CI

Ein Lauf auf `main` kostet rund $1,80, ein Release rund $1,50. Das trägt bei $18 im Monat nicht. Der Nutzer entschied am 2026-11-05: ganz lokal, keine automatische CI, kein eigener Runner. Umgesetzt auf dem Branch `ci-local`, gemergt in `1a7c112c`:

- `891f1b87`: `ci.yml` nur noch mit `workflow_dispatch` (Eingabe `windows`, Standard aus); kein Push, kein Pull Request, kein Label startet einen Lauf.
- `22354fc0`: `scripts/release-local.sh <tag> [--dry-run]` baut auf dem Mac DMG, `SHA256SUMS.txt` und die Notizen, prüft wie `bundle-macos` (Version, sauberer Baum auf `main` gleich `origin/main`, `check-bundle.sh`, keine ROM-Daten, `doctor`, Rauchtest) und legt danach Tag und Entwurf mit `gh` an. Mit Shell-Tests.
- `eb9e0d39`: `release.yml` nur noch mit `workflow_dispatch` (`tag`, `dry_run` Standard an). Es baut den Windows-Installer aus dem Tag und hängt ihn mit seiner Zeile in `SHA256SUMS.txt` an den vorhandenen Entwurf.
- `762fe877`: `release.md`, `dev-setup.md`, Roadmap-Querschnitt und Kopfkommentar von `gate.sh`.

Die Prüfung vor einem Merge bleibt das volle `scripts/gate.sh` lokal (G8).

## Befunde

| Nr. | Schwere | Befund | Stand |
|---|---|---|---|
| B1 | mittel | Das macOS-Bundle war nicht vollständig ad-hoc signiert: Nur die Binaries trugen die Linker-Signatur, ohne gebundene `Info.plist` und versiegelte Ressourcen. `codesign --verify --deep --strict` scheiterte, und eine heruntergeladene App bekommt dann „beschädigt“ statt „Trotzdem öffnen“. | behoben in `09cb649a`: `signingIdentity: "-"`, `hardenedRuntime: false`, weil die Library Validation die ad-hoc signierte dylib in rotom-lsp ablehnte. `check-bundle.sh` prüft die Signatur. Auf `rc.2` mit Quarantäne: `codesign` gültig, `spctl` „rejected“ (nicht notarisiert, also „Trotzdem öffnen“); auf `rc.1`: „code has no resources but signature indicates they must be present“. |
| B2 | gering | Versionstests hingen an der Version des Repos (G8, Lauf 1) | behoben in `933cf092` |
| B3 | gering | Flake unter Windows: `refs.rs` `a_saved_source_shows_after_the_gate` (gleich große Neuschrift innerhalb eines Zeitstempel-Takts) | Test wartet einen Takt, in `ebf44628`. Produktgrenze offen: Der Ref-Index erkennt Änderungen nur über (Größe, mtime); eine Racy-Regel wie bei den Fingerprints fehlt. |
| B4 | gering | Flake unter Last: `a_hanging_tileset_…` mit einer Zeitgrenze von 1 s (G8, Lauf 2) | offen, nur unter Last |
| B5 | Hinweis | Die Runner nutzen verschiedene Rust-Fassungen (macOS 1.98.0, Windows 1.99.1), weil `rust-toolchain.toml` nur `stable` nennt | offen |
| B6 | Hinweis | Die Caches belegen rund 10,5 GB, mehr als das Limit von 10 GB je Repo; GitHub verdrängt alte Einträge. Teilcaches `-partial-<run>` sammeln sich an. | offen |
| B7 | Hinweis | `open` aus einer Shell gibt der App die ganze Shell-Umgebung mit; für einen echten LaunchServices-Start braucht es `env -i /usr/bin/open` | dokumentiert |
| B9 | Hinweis | Im Release-Build bricht `rust-objcopy` mit SIGABRT ab (findet `libLLVM.dylib` nicht); Cargo entfernt dann keine Debug-Infos aus Build-Skripten und meldet nur eine Warnung (G8, Lauf 3) | offen, kosmetisch |
| B10 | Hinweis | `publish-windows` (Entwurf suchen, Prüfsumme ergänzen, Upload mit `--clobber`) ist nach der Umstellung nicht auf dem Runner gefahren; der Probelauf überspringt ihn. Die Zeile in `SHA256SUMS.txt` macht `scripts/release/add-checksum.sh`, das Shell-Tests hat. | offen, beim nächsten Release prüfen |
| B8 | Hinweis | Kleinpunkte aus dem Review der Umsetzung (nicht blockierend):<br>• Das Ergebnis von `_dup2` wird unter Windows nicht geprüft.<br>• `HttpServer::stop` blockiert bis zu 2 s.<br>• `fingerprint::stat` hat unter Windows keinen Rückfall.<br>• Die npm-Hinweise sind nur eine Liste.<br>• `AttachConsole` von doctor unter cmd ist nicht bestätigt.<br>• rotom-lsp endet nach `exit` nicht, solange stdin offen bleibt. | offen |

## Hand (nur der Nutzer)

| Punkt | Ergebnis |
|---|---|
| macOS: das DMG `v0.1.0-rc.2` auf einem frischen Benutzerkonto ohne Entwicklungswerkzeuge: Projekt öffnen, Kollision ändern, ROM bauen, in melonDS spielen, Script mit Sprachdiensten, texturierte Modelle; dazu der Gatekeeper-Weg (Systemeinstellungen › Datenschutz & Sicherheit › „Trotzdem öffnen“). Die rc.2-App liegt schon in `~/Applications/Gen4 Studio.app`. | offen |
| Windows: der Installer `v0.1.0-rc.2` auf einem Windows-PC mit denselben Schritten (ohne Spielmodus live), dazu die Warnung von SmartScreen | offen |
| Entwürfe auf GitHub: `rc.2` ansehen; `rc.1` (Befund B1) löschen oder liegen lassen. Veröffentlicht wird nichts automatisch. | offen |
| Nächstes Release: Tag und Entwurf `v0.1.0-rc.2` stammen aus dem alten Workflow, `release-local.sh` bricht an einem vorhandenen Release ab. Also `scripts/version.sh set 0.1.0-rc.3` (oder `0.1.0`) mit Eintrag in `CHANGELOG.md`, dann `release-local.sh` und `release.yml -f dry_run=false`; oder `rc.2` samt Tag löschen. | offen |
