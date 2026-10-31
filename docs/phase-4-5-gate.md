# Phase 4.5: Gate prüfen

Stand: 2026-10-31. Branch `phase-4-5-veroeffentlichen` (Worktree `.claude/worktrees/phase-4-5`, Basis main `3c194c7`), nicht nach main gemergt. **Gate vom Agenten bestanden.** Gate-Lauf am 2026-10-31 (19:34 bis 20:06) auf `a752a19`: (1) gated Tests einzeln, (2) Szenario kopflos und (3) Browser-Teil auf allen drei Spielen bzw. Platin DE **grün**; neun Befunde (einer mittel, sonst niedrig oder Hinweis, Abschnitt „Befunde“), keiner sperrte einen Gate-Punkt. Die Befunde 2 bis 7 sind behoben (`503a676`, `d8261cf`) und im Nachtest am 2026-10-31 (21:18 bis 21:26, Platin DE, Kopien `4-5-re` und `4-5-re0`, danach gelöscht) bestanden; 1, 8 und 9 sind so gewollt. `cd apps/studio && pnpm test` auf `d8261cf`: 333 Testdateien, 3 406 Tests grün (1 übersprungen), 154 s, ohne Wiederholung. Beim Nutzer bleiben Hand und Blick. Die Zahlen aus den Läufen der Umsetzung (Zeilen „Implementer-Lauf, unter Last“) bleiben zum Vergleich stehen. `scripts/gate.sh` als Ganzes läuft der Orchestrator auf main nach dem Merge, nicht in diesem Worktree.

Beim Nutzer offen (Abschnitte „Hand“ und „Blick“): Rom Patcher JS mit der eigenen Basis-ROM und einem Patch aus (2), „Im Finder zeigen“ in der gepackten App, das eigene Git-Werkzeug zeigt den Commit „Veröffentlicht v0.1.0“ mit Body und ohne Remote, der Blick auf Titelleiste, Assistent, Fläche und Dialog in Hell, Dunkel und Kompakt; die `.fig` folgt (`design-system/openpencil/NACHTRAG-4-5.md`).

Phase 4.5 bringt „Veröffentlichen“ als Zwischenphase parallel zu 3d.1: Ein Assistent in vier Schritten (Version, Patch, Prüfen, Fertig) schreibt eine Fassung des Projekts als selbstgeprüften xdelta-Patch `<Projekt>-v<Version>.xdelta` mit Begleitdatei (Version, Datum, Änderungsnotizen, Basis und Prüfsummen) und wahlweise `README.txt` an einen gewählten Ort (Standard `build/`) und trägt sie in `.g4/releases.json` ein. Der Kernel (`patch::publish`, `patch::release`) prüft vor dem Bauen Version (`bad_version`, `version_taken`), Dateiname (`bad_file`), Ort (`bad_dir`; der Pfad einer Datei oder darunter) und eine vorhandene Datei (`exists`, nur mit `overwrite` ersetzt) und verweigert eine gebaute ROM, die der Basis-ROM gleicht (`no_changes`); eine kaputte oder neuere Liste bleibt, wie sie ist (`releases_broken`). „Stand sichern…“ (`git::commit_all`) frischt die Vorlagen auf, nimmt mit `git add -A` alles auf und legt einen Commit mit Betreff und Notiz an, nie ein Remote, nie ein Push; es verweigert ein Projekt ohne Repository, in einem größeren Repository, ohne Änderung und ohne `git` mit seinem Satz, ein Repository ohne Commit bekommt so seinen ersten. Die Taste „Veröffentlichen“ sitzt in der Titelleiste (gesperrt mit Hinweis im Tooltip, wenn die Basis-ROM fehlt), die Liste „Bisherige Veröffentlichungen“ in Technik › Veröffentlichen, „Stand sichern…“ im Ergebnis des Assistenten, in Technik › Änderungen und in der Palette. MCP bekommt `publish_patch` (fragt nur beim Ersetzen mit `overwrite: true` oder bei einem Ort außerhalb `build/`) und `git_commit_all` (fragt immer), `g4k` die Befehle `publish` und `git commit-all`. Roadmap: [docs/ROADMAP.md](ROADMAP.md), Abschnitt „Phase 4“, Punkt „4.5 Veröffentlichen“. Plan: [2026-10-30-phase-4-5-veroeffentlichen.md](superpowers/plans/2026-10-30-phase-4-5-veroeffentlichen.md). Formate: [xdelta_patch.md](formats/xdelta_patch.md) (Begleitdatei, Veröffentlichung), [project_git.md](formats/project_git.md); Werkzeuge: [mcp.md](mcp.md) (`publish_patch`, `git_commit_all`, `releases`).

Umsetzung (`3c194c7..7fc27c2`):

| Commit | Inhalt |
|---|---|
| `89cecab`, `5c749ab` | Task 1: Liste der Veröffentlichungen, Vorschlag und Prüfung der Version, Notizen mit Version, Datum und Änderungen; `next_version` behält das führende `x.y.z` |
| `4c4ba64` | Task 2: `build_and_publish`, README, Ort nach Wahl, `g4k publish` |
| `34d06fd` | Task 3: `commit_all`, `g4k git commit-all` |
| `f356931`, `cadcc12` | Task 4: Befehle, `publish_patch`, `git_commit_all`, `reveal_path` über Brücke und MCP; der Ort geht vor dem Lesen der Platte durch die Policy |
| `b1d17a9`, `3fba8c4` | Task 5: Assistent, Taste in der Titelleiste, Palette; der Build einer Veröffentlichung speist Schritte und Probleme, „Prüfen“ schließt statt zurückzugehen |
| `9f95451` | Task 6: Technik › Veröffentlichen mit Taste und Liste |
| `34aa2cd` | Task 7: „Stand sichern“ als Dialog, in Technik › Änderungen, im Assistenten und in der Palette |
| `7fc27c2` | Task 8: Design-System (README „Nachtrag 4.5“, Audit, Vorlage, Anleitung) |

Gate-Text der Spezifikation ([2026-10-30-phase-3d-erzaehlen-design.md](superpowers/specs/2026-10-30-phase-3d-erzaehlen-design.md), §4, Zeile 4.5):

> **4.5 Veröffentlichen** (Zwischenphase) | S bis M | Phase 4 | Assistent erzeugt Patch und Begleitdatei mit Version und Notizen am gewählten Ort; „Stand sichern“ legt einen Commit an, `git status` danach leer; MCP mit Rückfrage; Rom Patcher JS als Hand

Gate-Text des Gutachtens C ([gutachten-c-loeschen-sprites-schnitt.md](research/2026-10-30-phase-3d/gutachten-c-loeschen-sprites-schnitt.md), §6.4):

> **4.5:** Assistent „Veröffentlichen“ erzeugt Patch und Begleitdatei mit Version und Notizen am gewählten Ort; „Stand sichern“ legt einen Commit an, `git status` danach leer; MCP mit Rückfrage; Rom Patcher JS als Hand.

Dazu gilt das Phasen-Gate der Spezifikation: gated Tests auf frischen Kopien von Platin DE, HeartGold DE und HeartGold US, der Browser-Teil über die Test-Umgebung, `scripts/gate.sh` grün, Hand und Blick beim Nutzer.

Wer was prüft: Die Spalte „Wer“ sagt **Agent** (die Test-Umgebung aus [docs/agent-gate.md](agent-gate.md): Browser-Fenster mit `studio-bridge`, `call.sh`, `mcp.sh`, dazu `g4k`, `studio mcp` kopflos und die gated Tests), **Agent + Blick** (der Agent prüft und beschreibt Ort und Zahlen, der Nutzer bestätigt kurz am Bild) oder **Hand** (der Nutzer). Befehle: [docs/agent-gate.md](agent-gate.md), Abschnitt „Phase 4.5“.

Namen in der Oberfläche (Rollen und `data-*` für die Prüfung):

- **Titelleiste:** `button` „Veröffentlichen“ (Symbol `publish`) zwischen „Bauen“ und „Thema wechseln“; ohne Projekt `disabled`; gesperrt (`aria-disabled`) mit dem Tooltip in zwei Zeilen (Name und `hint`, auch `aria-description`): „Erst eine Basis-ROM zuordnen.“, „Basis-ROM nicht gefunden: …“ oder der Satz der kaputten Liste; `busy` solange die App baut oder speichert.
- **Technik › Veröffentlichen** `[data-technik=veroeffentlichen]`: Kopfsatz „Ein Patch gibt deine Änderungen weiter, ohne die ROM: …“; Karte „Basis-ROM“ `[data-base-state]` (wie 4.1a); Karte „Veröffentlichen“ `[data-publish-card]` mit der primären Taste „Veröffentlichen…“ (Notiz mit dem Grund daneben, wenn gesperrt) und der Tabelle „Bisherige Veröffentlichungen“ (Spalten Version, Datum, Datei, Größe, Notizen, „Im Finder zeigen“; Zeilen `[data-release=<Version>]`, neueste zuerst; leer „Noch keine Veröffentlichung.“; eine kaputte Liste als Fehlermeldung statt der Tabelle). Untertitel in der Liste „Bereit zum Veröffentlichen“, „Veröffentlicht v0.1.0“, „Nicht lesbar“.
- **Assistent „Veröffentlichen“** (Rolle `dialog`, Name „Veröffentlichen“, im Deck der Welt; Atlas auf Welt): Schrittleiste `ol` „Schritte“ mit „1. Version“, „2. Patch“, „3. Prüfen“, „4. Fertig“; Schritte `[data-publish-step=version|patch|check]`: Felder „Version“, „Änderungsnotizen“, „Dateiname“, Pfad `[data-publish-dir]`, „Ordner wählen…“ (Dateidialog „Speicherort wählen“, im Browser-Modus das Pfadfeld), Kästchen „Änderungsnotizen auch als README.txt daneben“, Lauf `[data-publish-progress]` („Speichern und bauen …“, „Patch kodieren …“, „Selbstprüfung …“, „Dateien schreiben …“, „Veröffentlichung eintragen …“), „Ersetzen“ `[data-publish-overwrite]`, die Vorschau `[data-publish-preview]` (Tabelle „Was du weitergibst“); Fußzeile „Schreibt … und die Begleitdatei nach …“; Ergebnis „Veröffentlicht: v0.1.0“ mit den Tasten „Stand sichern…“ `[data-publish-commit]` (nur mit eigenem Repository), „Im Finder zeigen“ und „Schließen“. Nach dem Lauf: „Schließen“ statt „Abbrechen“, kein „Zurück“.
- **Dialog „Stand sichern?“** `[data-commit-dialog]`: Beschreibung „Legt einen Commit mit allen Änderungen im lokalen Repository an (git add -A, git commit). Nichts wird hochgeladen.“; Prüfliste `[data-check]` (✓ „Alles gespeichert“, ✓ „Git gefunden: 2.54.0 (Apple Git-157)“, ✓ „3 geänderte Dateien seit dem letzten Commit“); Felder „Betreff“ (vorbelegt „Stand 2026-10-31 14:05“ oder „Veröffentlicht v0.1.0“) und „Notiz“; „Abbrechen“ / „Sichern“ (primär); Lauf `[data-commit-progress]` („Vorlagen prüfen …“, „Dateien aufnehmen …“, „Commit …“); danach der Toast „Stand gesichert: 4c1d2e3 „Veröffentlicht v0.1.0“ (3 Dateien).“ und in der Statusleiste `[data-git-status]` „main · keine Änderung“.
- **Technik › Änderungen** bei „Gegen letzten Commit“ im eigenen Repository: die sekundäre Taste „Stand sichern…“ `[data-commit-all]` neben „Neu vergleichen“ (auch ohne Commit); ohne `git` `aria-disabled` mit dem Tooltip „Git ist nicht installiert.“.
- **Palette:** „Veröffentlichen…“ (gesperrt mit „Erst ein Projekt öffnen“, dem Grund der Taste, „Erst Speichern oder Bauen abwarten“ oder „Erst den Assistenten abschließen oder abbrechen“) und „Stand sichern…“ (nur mit eigenem Repository).
- **Agent:** Frage `[data-agent-question]` „„agent-gate“ möchte: Patch ersetzen: <Pfad des Patches> (lässt sich nicht rückgängig machen).“ (bei `publish_patch` mit `overwrite: true`; mit einem Ort außerhalb `build/` „Bauen außerhalb von build/“) und „„agent-gate“ möchte: Stand sichern (lässt sich nicht rückgängig machen).“; `publish_patch` ohne `overwrite` fragt nicht.

## Vorbereitung

Für den Lauf (Entwurf, durchzuführen mit dem Gate; Datum und Stand dann hier eintragen):

1. **Frische Kopien** unter `Roms/_testkopien/` per `cp -cR` (Ordner) und `cp -c` (`.nds`) aus den Originalen, die dabei nur gelesen werden; Ordner `4-5-<zweck>_DSPRE_contents` und `.nds` `4-5-<zweck>.nds` direkt nebeneinander:
   - `4-5-pt` (Platin DE), `4-5-hgde` (HeartGold DE), `4-5-hgus` (HeartGold US) für (1); sie werden nur gelesen, die Tests schreiben auf `vanilla_copy()`, die Basis-ROM ist die Nachbar-`.nds`;
   - `4-5-kpt`, `4-5-khgde`, `4-5-khgus` für (2), je mit `g4k base assign` auf die Nachbar-`.nds`; Ausgabe-Ordner (`<name>-out/`, `<name>-ort/` für `--dir`) daneben;
   - `4-5-web`, `4-5-mcp` und `4-5-ui` (Platin DE, Basis zugeordnet, Git eingerichtet) für (3); `4-5-ui` für die Bilder.

   Alle `4-5-*`-Kopien nach dem Gate löschen (Ordner und `.nds`; `Manual` und `_saves` bleiben).
2. **Werkzeuge:** `git --version` notieren. `cargo build --workspace --all-features` und die Release-Programme `cargo build --release -p g4-kernel --bin g4k` und `cargo build --release -p studio --bin studio`; im Worktree ohne rotom-Download (Hinweis in [dev-setup.md](dev-setup.md)). `tools/` ist der Symlink auf den Haupt-Checkout (nie einchecken). `G4_ROMS_DIR=<absoluter Pfad zu Roms>`, `G4_BASE_ROM_DIR=<Roms>/_testkopien`, nie die Originale. `xdelta3` im `PATH` (optional) für die Gegenprobe.
3. **Test-Umgebung** für (3) nur auf `G4_VITE_PORT=1730 G4_BRIDGE_PORT=1731` (`G4_ROMS_DIR=<absolut> G4_VITE_PORT=1730 G4_BRIDGE_PORT=1731 bash scripts/agent/studio-web.sh`), Seite `http://127.0.0.1:1730/?project=<Roms>/_testkopien/4-5-web_DSPRE_contents`, 1440 × 900; andere Ports gehören den parallelen Phasen.
4. **Worktree-Hinweise:** `G4_ROMS_DIR` absolut, `tools`-Symlink; `scripts/gate.sh` läuft nicht hier, sondern auf main nach dem Merge; unter Last streuen die Zeiten (vitest-Worker, gated Zeitgrenzen): Last mit `uptime` vor und nach jedem Lauf notieren (Spalte „Last“ der Zahlentabelle), Zeitprüfungen binden nur mit `G4_TIMING_STRICT=1`.
5. **Ausgaben** unter `target/agent/gate-2026-10-31/4-5/` im Worktree: `gated-base-<k>.log`, `gated-git-<k>.log`, `gated-studio-<k>.log` (1), `kopflos-<k>.log` (2), die Bilder `blick-*.png` und die Schrittbilder `ui-*.png`. Den Ordner vor `git worktree remove` sichern.

**So lief es am 2026-10-31** (Agent, `a752a19`, 19:34 bis 20:06):

- Werkzeuge: macOS 27.2, `git version 2.54.0 (Apple Git-157)`, `rustc 1.99.1`, `xdelta3` 3.2.2 im `PATH`, Chrome 156 headless über CDP (Pipe, ohne Debug-Port; Node 24), 1440 × 900. `cargo build --release --workspace --all-features` (g4k, studio, studio-bridge) 89 s, dann `cargo test --release --workspace --all-features --no-run`; im Worktree mit Namensfilter über den Workspace statt `-p`/`--test` (dev-setup.md, rotom).
- Kopien (frisch per `cp -cR`/`cp -c`): `4-5-{pt,hgde,hgus}` (1), `4-5-k{pt,hgde,hgus}` (2) mit `4-5-k<k>-ort/`, `4-5-{web,mcp,ui,alt}` (Platin DE, `g4k base assign`, `g4k git init`: 18 450 Dateien je 8 bis 12 s), dazu `4-5-neu` (ohne Basis, ohne Git: Titelleiste gesperrt, (2) 13 `no_repo`, (3) 12 `git init` von Hand) und `4-5-enc/projekt_DSPRE_contents` (Kopie in einem Repository für `enclosing_repo`). Alle nach dem Lauf gelöscht (Ordner und `.nds`), `Manual` und `_saves` unberührt.
- Last: vier Orchestratoren auf der Maschine; `uptime` 20,5 beim Start, Spitze 35,5 (19:39), 6,7 bis 21 während der gated Tests, 40,4 am Ende (fremde Builds). Zeitprüfungen ohne `G4_TIMING_STRICT`; kein Zeittest scheiterte.
- Test-Umgebung nur auf 1730/1731, viermal gestartet (mit `G4_BASE_ROM_DIR`, ohne, mit `G4_GIT=/nicht/da`, wieder ohne); am Ende `lsof -iTCP:1730 -iTCP:1731 -sTCP:LISTEN` leer.

## (1) Automatik

1. `scripts/gate.sh` als Ganzes: **auf main nach dem Merge** (Orchestrator), nicht in diesem Worktree: fmt, clippy, deny, `cargo test` ohne ROM-Daten, `mcp`, `pnpm test`, Tokens, Styles, `pnpm build`, dist ohne Brücke, `g4emu`, die gated Tests je Projekt (die neuen laufen im Schritt der gated Tests ohne Änderung am Skript mit; ohne `git` im `PATH` melden die Git-Tests `skipped: git not found`: vorher `git --version` notieren), build graph (timings), project start. Ergebnis: offen (Orchestrator auf main).
2. **Gated Tests einzeln**, je Projekt im Release-Profil mit `--nocapture --test-threads 1`, nacheinander (`K=<Roms>/_testkopien`, `<k>` = `pt`, `hgde`, `hgus`):
   - `publish_after_a_map_edit_writes_a_small_patch_that_applies` (`crates/g4-kernel/tests/base.rs`, `--test base`): `build_and_publish` nach einem Kartenbyte (`0.1.0`, mit `README.txt`): Phasen `build`, `encode`, `verify`, `write`, `record` in dieser Reihenfolge, Patch ≤ 1 KiB, mit dem eigenen Decoder und (wenn im `PATH`) mit `xdelta3 -d` angewandt gleich der gebauten ROM, Begleitdatei mit `Version:` und `Änderungen:`, ein Eintrag in `.g4/releases.json`, `0.1.1` danach gibt zwei Einträge. Druckt `publish <Spielcode> size=… build_ms=… encode_ms=… verify_ms=…`.
   - `publish_then_commit_all_leaves_status_empty_and_no_remote` (`crates/g4-kernel/tests/git.rs`, `--test git`): Git einrichten, ein Berechtigungsbyte der Karte 3 ändern, als `0.1.0` veröffentlichen, „Stand sichern“ mit Betreff „Veröffentlicht v0.1.0“; der Commit hat mindestens zwei Dateien, `git status` ist leer, `.g4/releases.json` und `.g4/base.json` stehen im Index, nichts unter `build/`, kein `[remote` in `.git/config`. Druckt `commit <Spielcode> files=… add_ms=… commit_ms=… status_ms=…`.
   - `publish_and_commit_over_the_command_path_on_the_vanilla_copy` (gated Studio-Test, `apps/studio/src-tauri/src/commands/publish.rs`): Git einrichten, Trainer 1 über `data_edit` ändern und speichern, als `0.1.0` veröffentlichen (Patch und Begleitdatei unter `build/`, ein Eintrag), „Stand sichern“, Baum sauber; druckt die Zeiten.
   - Dazu `cli_publish_prints_the_files_and_the_count` (`crates/g4-kernel/tests/cli.rs`, gated Teil, `G4_TEST_PROJECT`).

Befehle aus der Wurzel des Worktrees:

```sh
export PATH="$HOME/.cargo/bin:/opt/homebrew/opt/rustup/bin:$PATH"
G4_TEST_PROJECT="$K/4-5-<k>_DSPRE_contents" \
  cargo test --release -p g4-kernel --all-features --test base publish_after_a_map_edit -- --nocapture --test-threads 1
G4_TEST_PROJECT="$K/4-5-<k>_DSPRE_contents" \
  cargo test --release -p g4-kernel --all-features --test git publish_then_commit_all -- --nocapture --test-threads 1
G4_TEST_PROJECT="$K/4-5-<k>_DSPRE_contents" \
  cargo test --release -p studio --all-features --lib publish_and_commit_over_the_command_path_on_the_vanilla_copy -- --nocapture --test-threads 1
```

Bekannte Zeilen aus den Läufen der Umsetzung (**Implementer-Lauf, unter Last**, Release, nicht der Gate-Lauf):

```
publish CPUD size=553 build_ms=97791 encode_ms=114 verify_ms=148        (Platin DE, gated Kernel, --test base)
commit IPKD files=2 add_ms=43 commit_ms=54 status_ms=77                 (HeartGold DE, gated Kernel, --test git)
studio gated, Platin DE: git init 7,7 s, publish 70,7 s (build 69 978 ms, encode 152, verify 169), commit 0,3 s
```

`files=2` im Commit sind `.g4/releases.json` und der geänderte Trainer-Member (die Vorlage ignoriert `build/` mit Patch, Begleitdatei und ROM). Der Build dominiert die Veröffentlichung (70 bis 98 s unter Last; ohne Last ein Bruchteil, vergleiche 4.2); Kodieren und Prüfen liegen bei 114 bis 169 ms.

**Gate-Lauf am 2026-10-31 (Agent, Release, `--nocapture --test-threads 1`, nacheinander; Logs `target/agent/gate-2026-10-31/4-5/gated-*.log`): alle zwölf Läufe grün, kein `skipped`.** Der Studio-Test prüft jetzt auch `build.steps` nicht leer.

```
publish CPUD size=553 build_ms=20548 encode_ms=118 verify_ms=142      publish CPUD write_ms=14 record_ms=0 ms=20944 second_ms=483 xdelta3=true
publish IPKD size=554 build_ms=25207 encode_ms=163 verify_ms=159      publish IPKD write_ms=17 record_ms=0 ms=25662 second_ms=555 xdelta3=true
publish IPKE size=554 build_ms=18660 encode_ms=144 verify_ms=142      publish IPKE write_ms=13 record_ms=0 ms=19068 second_ms=688 xdelta3=true
commit CPUD files=2 add_ms=53 commit_ms=72 status_ms=97
commit IPKD files=2 add_ms=45 commit_ms=52 status_ms=94
commit IPKE files=2 add_ms=38 commit_ms=57 status_ms=75
studio gated, Platin DE:    git init 9,8 s (18 450 Dateien), publish 43,6 s (build 42 888, encode 176, verify 185), Patch 552 B, commit 0,27 s (2 Dateien), Status danach 141 ms
studio gated, HeartGold DE: git init 10,7 s (19 990), publish 33,4 s (build 32 733, encode 149, verify 142), Patch 555 B, commit 0,18 s (2), Status 98 ms
studio gated, HeartGold US: git init 9,2 s (19 990), publish 36,0 s (build 35 317, encode 175, verify 157), Patch 555 B, commit 0,27 s (2), Status 135 ms
```

| Spiel | `--test base` | `--test git` | `cli_publish_…` (gated) | Studio-Test | Last (Start → Ende) |
|---|---|---|---|---|---|
| Platin DE | grün, 63 s (Wand 83 s) | grün, 103 s | grün, 63 s | grün, 144 s | 35,5 → 16,4 |
| HeartGold DE | grün, 76 s | grün, 106 s | grün, 65 s | grün, 104 s | 16,4 → 12,5 |
| HeartGold US | grün, 55 s | grün, 54 s | grün, 56 s | grün, 124 s | 12,5 → 16,1 |

## (2) Szenario kopflos

Release-`g4k` und `studio mcp --project <Kopie> --base-rom <nds> [--allow-irreversible]` über JSON-RPC/stdio, je Spiel nacheinander. **Ergebnis am 2026-10-31: Platin DE, HeartGold DE und HeartGold US grün** (Logs `kopflos-<k>.log`; Client `scripts/g4emu/g4emu/mcp.py`); Abweichung nur am Rand von Punkt 11 (Befund 1). Auf den Kopien `4-5-k<spiel>`:

- [x] 1. `g4k base assign <Kopie> <nds>`: zugeordnet, ohne Warnung.
- [x] 2. `g4k git init <Kopie>` → `committed Ausgangsstand files=…`.
- [x] 3. MCP `data_edit` Trainer 1 (`party[0].level` +1), `save`.
- [x] 4. `publish_patch {"version":"0.1.0","notes":"Erste Fassung","readme":true}` **ohne** `--allow-irreversible`: läuft (keine Frage, nichts wird ersetzt); die Antwort nennt drei Dateien unter `build/` (Patch `<Projekt>-v0.1.0.xdelta`, Begleitdatei `….xdelta.txt`, `README.txt`), `release.version` `0.1.0`, `phase_ms` je Phase und `build`.
- [x] 5. Dieselbe Version noch einmal: `version_taken` („Diese Version gibt es schon.“), nichts wird gebaut.
- [x] 6. `{"version":"0.1.1"}` ohne `overwrite` auf vorhandenem `file` (`<Projekt>-v0.1.0.xdelta`): `exists`; mit `"overwrite": true` ohne `--allow-irreversible`: `needs_confirmation` („publish_patch lässt sich nicht rückgängig machen (Patch ersetzen); kopflos nur mit --allow-irreversible“; der Pfad steht erst in der Frage der App); mit `--allow-irreversible`: ersetzt.
- [x] 7. **Begleitdatei:** `build/<Projekt>-v0.1.0.xdelta.txt` beginnt wie in 4.1a mit den fünf Zeilen `Gen4 Studio: Patch für …`, `Spiel:`, `Basis-ROM SHA-1:`, `Ziel-ROM SHA-1:`, `Studio-Version:`; dazu `Version: 0.1.0`, `Datum: <RFC 3339, UTC>` und am Ende der Absatz `Änderungen:` mit „Erste Fassung“; `README.txt` mit `<Projekt> v0.1.0 (<Tag>)`, den Notizen und `Basis-ROM SHA-1:`.
- [x] 8. **Gegenprobe:** `xdelta3 -d -s <nds> <patch> out.nds` (wenn im `PATH`): SHA-1 von `out.nds` = `Ziel-ROM SHA-1` der Begleitdatei = SHA-1 von `build/<Projekt>-<Spielcode>.nds`.
- [x] 9. `git -C <Kopie> status --porcelain` zeigt `.g4/releases.json` und den Trainer-Member (je nach Layout `unpacked/…`), nichts unter `build/`.
- [x] 10. `git_commit_all {"message":"Veröffentlicht v0.1.0","notes":"Erste Fassung"}` ohne `--allow-irreversible`: `needs_confirmation`; mit: `committed`, `files`, `phase_ms`; danach `git status --porcelain` leer, `git log -1 --format=%B` = Betreff und Body, `git remote -v` leer, `.git/config` ohne `[remote`.
- [x] 11. `g4k publish <Kopie> --version 0.2.0 --dir <Temp-Ordner>` (ein vorhandener Ordner): Patch und Begleitdatei liegen dort, `.g4/releases.json` trägt einen Eintrag mit absolutem `file`. Über MCP muss ein `dir` außer `build/` existieren, sonst wird abgelehnt: die Pfadprüfung der Policy (`bridge/roots.rs`) kommt vor dem Kern und sieht den Ort an, bevor er gebaut wird. Ein `dir`, das eine Datei in einem erlaubten Ordner ist, ist `bad_dir` („Der Speicherort ist kein Ordner: <Pfad>“); ein `dir` **unter** einer Datei und ein fehlender Ordner antworten schon dort mit `error` „Pfad nicht lesbar: …“ (so gewollt, kein `bad_dir` über MCP). `g4k publish` (und die App mit ihrem Pfad) antwortet für die Datei und darunter `bad_dir`. In jedem Fall bleibt `.g4/releases.json`, wie sie war.
- [x] 12. `g4k git commit-all <Kopie>` → `committed <kurz> "Stand … UTC" files=…`; ein zweites Mal ohne Änderung wird verweigert: der Satz „Keine Änderung seit dem letzten Commit; nichts zu sichern.“ (`NOTHING_TO_COMMIT`) steht auf stderr, Exit 1, ohne Vorsatz `not committed:`. Diese Zeile auf stdout („not committed: <Grund>“, Exit 1) gibt es nur für einen Commit, der lief und scheiterte (Hook, Signatur).
- [x] 13. Verweigerungen von „Stand sichern“ ohne Schreiben: ein Projekt ohne Repository (`no_repo`), in einem größeren Repository (`enclosing_repo`), ohne `git` (`no_git`, mit `G4_GIT=/nicht/da`): je der Satz des Kerns auf stderr mit Exit 1 (kein `not committed:` auf stdout), nichts wird geschrieben (`.gitignore`, `.gitattributes` unverändert).

```sh
P=$K/4-5-k<k>_DSPRE_contents; N=$K/4-5-k<k>.nds; G=target/release/g4k
$G base assign $P $N; $G git init $P
target/release/studio mcp --project $P --base-rom $N                         # data_edit, save, publish_patch → läuft; version_taken; exists; git_commit_all → needs_confirmation
target/release/studio mcp --project $P --base-rom $N --allow-irreversible    # overwrite → ersetzt; git_commit_all → committed
xdelta3 -d -s $N $P/build/<Projekt>-v0.1.0.xdelta out.nds; shasum out.nds     # = Ziel-ROM SHA-1 der Begleitdatei
git -C $P status --porcelain; git -C $P log -1 --format=%B; git -C $P remote -v
$G publish $P --version 0.2.0 --dir <Temp-Ordner>
$G git commit-all $P; $G git commit-all $P                                    # committed … (stdout); dann Exit 1 und der Satz „Keine Änderung …“ auf stderr
```

So lief es (je Spiel; Zahlen Platin DE / HeartGold DE / HeartGold US):

- 1, 2: `assigned sha1=… label=…` ohne Warnung; `committed Ausgangsstand files=18450 / 19990 / 19990 branch=main` (21,3 / 8,9 / 11,1 s).
- 3: Trainer 1 `party[0].level` 5 → 6 (Gunther) bzw. 14 → 15 (Silber / Silver), `save` schreibt Trainer 1.
- 4: ohne Frage, 30,3 / 50,5 / 40,9 s; drei Dateien unter `build/` (Patch 552 / 557 / 557 B, Begleitdatei 433 / 440 / 440 B, `README.txt` 266 B), `release.version` `0.1.0`, `phase_ms` `build` 29 642 / 49 612 / 40 186, `encode` 132 / 249 / 162, `verify` 144 / 194 / 147, `write` 23 / 17 / 12, `record` 0; `build` mit `steps`, `packed`, `rom`.
- 5, 6: `version_taken` „Diese Version gibt es schon.“ (1 ms); `exists` „4-5-k<k>-v0.1.0.xdelta liegt schon am Speicherort; wähle einen anderen Namen oder ersetze die Datei.“; mit `overwrite` ohne Schalter `needs_confirmation` „publish_patch lässt sich nicht rückgängig machen (Patch ersetzen); kopflos nur mit --allow-irreversible“; `.g4/releases.json` und die drei Dateien danach unverändert. Mit `--allow-irreversible` ersetzt (0,75 / 0,98 / 0,90 s; die Begleitdatei trägt dann `Version: 0.1.1`).
- 7: Begleitdatei mit den fünf Zeilen, `Version: 0.1.0`, `Datum: 2026-10-31T18:40:04Z`, Anwendungshinweis und am Ende `Änderungen:` / „Erste Fassung“; `README.txt` „4-5-kpt v0.1.0 (2026-10-31)“, Notizen, `Basis-ROM SHA-1:`.
- 8: `xdelta3 -d` ergibt `f6c7cf19…` / `7310ecbc…` / `753a4c85…` = `Ziel-ROM SHA-1` = SHA-1 von `build/<Projekt>-<Spielcode>.nds`.
- 9: ` M unpacked/trainerParty/0001` und `?? .g4/releases.json`, nichts unter `build/`.
- 10: ohne Schalter `needs_confirmation` („git_commit_all … (Stand sichern) …“); mit: `committed`, `files` 2, `phase_ms` (`templates` 80, `add` 46, `commit` 59 auf Platin), 0,22 / 0,30 / 0,26 s; danach `status --porcelain` leer, `log -1 --format=%B` „Veröffentlicht v0.1.0“ / Leerzeile / „Erste Fassung“, `remote -v` leer, kein `[remote` in `.git/config`.
- 11: `g4k publish --version 0.2.0 --dir <K>/4-5-k<k>-ort` schreibt Patch und Begleitdatei dorthin, Eintrag mit absolutem `file`, `releases=3`. Über MCP mit `--allow-irreversible` liegt ein Ort außerhalb von `build/` und der `--allow-dir`-Ordner vorher als `error` „Pfad liegt außerhalb der erlaubten Ordner: …“ (wie `docs/mcp.md`; auch `<Projekt>/header.yaml`); mit `--allow-dir <ort>`: eine Datei als `dir` → `bad_dir` „Der Speicherort ist kein Ordner: …“, ein fehlender Ordner und ein Ort **unter** einer Datei → `error` „Pfad nicht lesbar: …: No such file or directory (os error 2)“ bzw. „… Not a directory (os error 20)“ statt `bad_dir` (Befund 1: so gewollt, die Pfadprüfung kommt vor dem Kern); `.g4/releases.json` blieb jedes Mal unverändert, kein Ordner wurde angelegt; `0.2.1` nach `<ort>` lief. `g4k` antwortet für Datei und „unter einer Datei“ `bad_dir` und legt einen fehlenden `--dir` an.
- 12: `committed eb514e8 "Stand 2026-10-31 18:40 UTC" files=1`; das zweite Mal Exit 1, stderr „error: Keine Änderung seit dem letzten Commit; nichts zu sichern.“, stdout leer.
- 13: `no_repo` („Dieses Projekt liegt in keinem Git-Repository; richte Git zuerst ein.“), `enclosing_repo` („Das Projekt liegt in einem größeren Git-Repository; sichere den Stand dort mit deinem Git-Werkzeug.“), `no_git` mit `G4_GIT=/nicht/da` („Git ist nicht installiert; „Stand sichern“ braucht die git-Kommandozeile (git-scm.com/downloads).“), je Exit 1 auf stderr, kein `not committed:`; `.gitignore`/`.gitattributes` vorher wie nachher (zweimal fehlend, einmal unverändert).

## (3) Browser-Teil

Test-Umgebung auf 1730/1731, Kopie `4-5-web` (Platin DE, Basis zugeordnet, Git eingerichtet), Seite `http://127.0.0.1:1730/?project=<Roms>/_testkopien/4-5-web_DSPRE_contents`. **Ergebnis am 2026-10-31: alle 14 Punkte grün** (Punkt 9 mit geänderter Probe, Hinweis unten; Befunde 2 bis 7 zu Fokus und Wortlaut). Vor Punkt 2 hat der Agent Trainer 1 über `call.sh data_edit` geändert und gespeichert (sonst wäre die gebaute ROM gleich der Basis, `no_changes`); ebenso auf `4-5-mcp` vor Punkt 13. Schritte:

- [x] 1. Technik › Veröffentlichen zeigt die leere Liste („Noch keine Veröffentlichung.“); Untertitel „Bereit zum Veröffentlichen“; Titelleiste „Veröffentlichen“ frei.
- [x] 2. Titelleiste „Veröffentlichen“ → Assistent (Atlas auf Welt): Vorschlag `0.1.0` und `4-5-web-v0.1.0.xdelta`; „Weiter“ gesperrt bei einem leeren Feld, frei mit dem Vorschlag.
- [x] 3. Änderungsnotizen eintragen, „Ordner wählen…“ (Pfadfeld des Browser-Modus), Kästchen README, „Weiter“ auf „Version“ → „Patch“.
- [x] 4. „Weiter“ auf „Patch“ startet den Lauf: Fortschrittszeile (im Browser-Modus nur die erste Phase, die Brücke hat keinen `Channel`), dann „Prüfen“ mit der Erfolgsmeldung, drei Dateien in „Was du weitergibst“ und dem Satz „Kein ROM-Inhalt, nur der Patch: …“; „Zurück“ gibt es nicht mehr, „Schließen“ statt „Abbrechen“.
- [x] 5. „Fertig“ → Ergebnis „Veröffentlicht: v0.1.0“ mit dem Pfad und „3 Dateien, …“.
- [x] 6. „Stand sichern…“ aus dem Ergebnis → Dialog mit dem Betreff „Veröffentlicht v0.1.0“ und den Notizen, Prüfliste (✓ gespeichert, ✓ Git, ✓ „N geänderte Dateien …“), „Sichern“ → Toast „Stand gesichert: …“; die Statusleiste sagt „main · keine Änderung“; `git -C <Kopie> log -1`, `remote -v` (leer), `status --porcelain` (leer).
- [x] 7. Technik › Veröffentlichen zeigt die Zeile `0.1.0` (Datum, Datei, Größe, Notizen); der Untertitel „Veröffentlicht v0.1.0“.
- [x] 8. Zweiter Assistent mit derselben Version von Hand: die Sperre am Feld „Diese Version gibt es schon.“, „Weiter“ gesperrt; mit `0.1.1` und dem Dateinamen von `0.1.0` von Hand: „Weiter“ auf „Patch“ → Fehlermeldung „… liegt schon am Speicherort …“ und „Ersetzen“ (`[data-publish-overwrite]`), Klick ersetzt.
- [x] 9. Die Basis-ROM „verlieren“: in `.g4/base.json` der Kopie die SHA-1 auf einen Wert ohne ROM ändern (Original vorher sichern, danach zurückschreiben). Das Umbenennen der `.nds` reicht in der Test-Umgebung nicht: die Brücke sucht im erlaubten Ordner `_testkopien` (und in `G4_BASE_ROM_DIR`) und findet eine andere Kopie mit derselben SHA-1, etwa die einer parallelen Phase. Danach Technik neu zeigen: Titelleiste „Veröffentlichen“ gesperrt mit dem Tooltip „Basis-ROM nicht gefunden: …“, Palette „Veröffentlichen…“ gesperrt mit demselben Grund, Technik › Veröffentlichen zeigt die Notiz neben der Taste; zurückschreiben.
- [x] 10. Technik › Änderungen „Gegen letzten Commit“: Trainer ändern, speichern, „Stand sichern…“ ohne Voreinstellung (Betreff „Stand <Datum> <Uhrzeit>“), „Sichern“ → Statusleiste „keine Änderung“.
- [x] 11. Ohne `git` (die Brücke mit `G4_GIT=/nicht/da` gestartet): „Stand sichern…“ in Technik › Änderungen ist gesperrt mit „Git ist nicht installiert.“; der Dialog aus der Palette sperrt „Sichern“ mit „Git ist nicht installiert“.
- [x] 12. Ein eigenes Repository ohne Commit (von Hand `git init` in einer weiteren Kopie): Technik › Änderungen bietet „Stand sichern…“ neben der Info „Noch kein Commit: …“; der erste Commit entsteht.
- [x] 13. MCP über die Brücke (Kopie `4-5-mcp`, Basis zugeordnet, Git eingerichtet, Freigabe „Lesen und bearbeiten“):
  - `scripts/agent/mcp.sh call publish_patch '{"version":"0.1.0"}'` läuft ohne Frage;
  - `scripts/agent/mcp.sh call publish_patch '{"version":"0.1.1","file":"4-5-mcp-v0.1.0.xdelta","overwrite":true}'` fragt „„agent-gate“ möchte: Patch ersetzen: …“ (`agent_status` zeigt die Frage, `agent_answer` oder „Zulassen“ im Fenster);
  - `scripts/agent/mcp.sh call git_commit_all '{}'` fragt „Stand sichern“.
- [x] 14. **Repository aus 4.4:** eine weitere Kopie (`4-5-alt`, Basis zugeordnet, `g4k git init`); in der Seite Trainer 1 ändern und speichern (sonst gäbe es nichts zu veröffentlichen); in `.gitignore` die Zeile `!/.g4/releases.json` von Hand löschen (der Block von 4.4) und beides committen (`git -C <Kopie> add -A && git -C <Kopie> commit -m "Block von 4.4"`). In der Seite veröffentlichen (Assistent bis „Fertig“): die Statusleiste sagt danach „keine Änderung“ (der alte Block hält `.g4/releases.json` draußen). „Stand sichern…“ aus dem Ergebnis: die Prüfliste zeigt kurz „Änderungen werden geprüft …“ (`git_commit_prepare` frischt den Block auf), dann „2 geänderte Dateien seit dem letzten Commit“, „Sichern“ ist frei (nicht „Keine Änderung seit dem letzten Commit“), die Statusleiste folgt mit „2 Änderungen“; „Sichern“ → `git -C <Kopie> show --stat --format= HEAD` nennt `.gitignore` und `.g4/releases.json`, `.gitignore` enthält wieder `!/.g4/releases.json`, `status --porcelain` leer.

Nicht im Browser prüfbar: die Fortschrittsphasen (`Channel`), „Im Finder zeigen“ (Toast „Nur in der App.“), die Kästchen- und Dateidialoge des Tauri-Fensters.

So lief es (Agent, Chrome headless über CDP, Tastatur und Maus als echte Eingaben):

- 1: „Noch keine Veröffentlichung.“, Untertitel „Bereit zum Veröffentlichen“, Titelleiste „Veröffentlichen“ frei zwischen „Bauen“ und „Thema wechseln“ (Tooltip „Veröffentlichen“).
- 2: Assistent (Rolle `dialog`, Name „Veröffentlichen“ über `aria-labelledby`, `ol` „Schritte“ mit 1 bis 4), Atlas auf Welt; `0.1.0` und `4-5-web-v0.1.0.xdelta`; leeres Feld: `aria-invalid`, der Satz von `BAD_VERSION`, „Weiter“ `disabled`.
- 3: Pfadfeld „Speicherort wählen“ (leer vorbelegt), Pfad übernommen in `[data-publish-dir]`; Kästchen `aria-checked`, benannt über `aria-labelledby`.
- 4: „Speichern und bauen …“ (`[data-publish-progress]`), nach 48 s „Prüfen“: „Selbstprüfung bestanden: …“, „Basis d40f19ac → Ziel f6c7cf19, geprüft in 157 ms“, drei Dateien (552 Byte, 433 Byte, 266 Byte), „Kein ROM-Inhalt, nur der Patch: …“; „Schließen“ statt „Abbrechen“, kein „Zurück“ in der Schrittleiste.
- 5: „Veröffentlicht: v0.1.0“, Pfad, „3 Dateien, 1,2 KiB“, „Als Nächstes“, „Stand sichern…“, „Im Finder zeigen“ (Toast „Nur in der App.“), „Schließen“.
- 6: Dialog mit „Veröffentlicht v0.1.0“ / „Erste Fassung“, ✓ „Alles gespeichert“, ✓ „Git gefunden: 2.54.0 (Apple Git-157)“, ✓ „2 geänderte Dateien seit dem letzten Commit“; Fokus im „Betreff“; „Sichern“ → „Vorlagen prüfen …“, Toast „Stand gesichert: 92186d7 „Veröffentlicht v0.1.0“ (2 Dateien).“, Statusleiste „main · keine Änderung“; `log -1` mit Body, `remote -v` leer, `status --porcelain` leer, kein `[remote`; der Commit nimmt `.g4/releases.json` und `unpacked/trainerParty/0001`. Der Fokus geht zurück auf „Veröffentlichen“ in der Titelleiste.
- 7: Zeile `0.1.0` · 31.10.2026 · `4-5-web-v0.1.0.xdelta` · 552 Byte · „Erste Fassung“, Untertitel „Veröffentlicht v0.1.0“. Tastatur: die Zeile ist ein Tab-Halt, Tab verlässt die Tabelle, Enter führt auf „Im Finder zeigen“, Escape zurück auf die Zeile (Muster der `Table`, Befund 6).
- 8: Vorschlag `0.1.1`; `0.1.0` von Hand: „Diese Version gibt es schon.“, „Weiter“ `disabled`; `0.1.1` mit `4-5-web-v0.1.0.xdelta`: „4-5-web-v0.1.0.xdelta liegt schon am Speicherort; wähle einen anderen Namen oder ersetze die Datei.“ und „Ersetzen“; Klick ersetzt (Begleitdatei `Version: 0.1.1`, zwei Einträge in `.g4/releases.json`).
- 9: **Probe geändert** (seither der Gate-Text oben). Das Umbenennen der `.nds` sperrt hier nicht: die Brücke findet über ihren erlaubten Ordner `_testkopien` eine andere Kopie mit derselben SHA-1 (`3d-1-pt.nds` einer parallelen Phase), mit und ohne `G4_BASE_ROM_DIR` (Befund 9). Geprüft mit einer SHA-1 in `.g4/base.json` der Kopie, die keine ROM hat (danach zurückgeschrieben): Titelleiste `aria-disabled`, `aria-description` und Tooltip „Basis-ROM nicht gefunden: POKEMON PL (CPUD) (d40f19ac…).“, Palette „Veröffentlichen…“ `aria-disabled` mit demselben Satz, Technik › Veröffentlichen „Veröffentlichen…“ gesperrt mit der Notiz, Untertitel „Basis-ROM fehlt“. Ohne Basis (`4-5-neu`): „Erst eine Basis-ROM zuordnen.“ in Titelleiste und Palette.
- 10: „Gegen letzten Commit“: „Stand sichern…“ neben „Neu vergleichen“; Dialog mit „Stand 2026-10-31 19:52“ (Ortszeit), leere Notiz; „Sichern“ → „Stand gesichert: 6a91ef1 „Stand 2026-10-31 19:52“ (2 Dateien).“, „main · keine Änderung“, Fokus zurück auf „Stand sichern…“. Aus der Palette: Fokus zurück auf das vorher fokussierte Element. Gescheiterter Commit (pre-commit-Hook in der Kopie): „Der Commit ist gescheitert: Hook des Gates: Commit abgelehnt. Es ist nichts committet; deine Änderungen bleiben im Projekt.“ und nur „Schließen“ (Befund 8).
- 11: mit `G4_GIT=/nicht/da`: „Stand sichern…“ `aria-disabled`, Tooltip „Git ist nicht installiert.“ (`aria-describedby`); der Dialog aus der Palette: Zeile „Git ist nicht installiert“, „Sichern“ `disabled`.
- 12: `git init` von Hand in `4-5-neu`: „Noch kein Commit: es gibt nichts zu vergleichen.“ und „Stand sichern…“, Statusleiste „main · noch kein Commit“; der erste Commit „Stand 2026-10-31 19:55“ mit 18 449 Dateien in 10 s, `.gitignore` mit dem Block, danach „main · keine Änderung“ (Befunde 4, 5).
- 13: `publish_patch {"version":"0.1.0"}` ohne Frage (47 s, build 46 341 ms); mit `overwrite` die Frage „„agent-gate“ möchte: Patch ersetzen: /…/4-5-mcp_DSPRE_contents/build/4-5-mcp-v0.1.0.xdelta (lässt sich nicht rückgängig machen).“ (`agent_status.question.what` „Patch ersetzen: …“), „Zulassen“ im Fenster → ersetzt; `git_commit_all {}` fragt „„agent-gate“ möchte: Stand sichern (lässt sich nicht rückgängig machen).“, „Zulassen“ → `committed`, Betreff „Stand 2026-10-31 18:57 UTC“, 2 Dateien; `remote -v` und `status --porcelain` leer, Statusleiste „main · keine Änderung“.
- 14: nach dem Veröffentlichen „main · keine Änderung“; „Stand sichern…“: erst „Änderungen werden geprüft …“, dann „2 geänderte Dateien seit dem letzten Commit“, „Sichern“ frei, Statusleiste „main · 2 Änderungen“; „Sichern“ → `8fd0bb2` mit `.g4/releases.json` und `.gitignore` (`!/.g4/releases.json` wieder da), `status --porcelain` leer.


## (4) Hand und Blick

Beim Nutzer (Abschnitte „Hand“ und „Blick“ unten): Rom Patcher JS mit der eigenen Basis-ROM und einem Patch aus (2), SHA-1 des Ergebnisses gegen die Begleitdatei (Hand, wie 4.1a offen; die Kopien aus (2) sind gelöscht, ein Patch entsteht mit `g4k publish` auf einer neuen Kopie in einer Minute); „Im Finder zeigen“ in der gepackten App (Hand); das eigene Git-Werkzeug zeigt den Commit „Veröffentlicht v0.1.0“ mit Body und ohne Remote (Hand); Blick auf Titelleiste, Assistent, Fläche und Dialog in Hell, Dunkel und Kompakt: die 53 Bilder `blick-*.png` liegen unter `target/agent/gate-2026-10-31/4-5/` (Liste in „Blick“).

## Ergebnis

| Punkt | Wer | Ergebnis | Datum |
|---|---|---|---|
| (1) 1 `scripts/gate.sh` | Agent | offen: läuft der Orchestrator auf main nach dem Merge | – |
| (1) 2 gated `--test base` (`publish_after_a_map_edit_…`) | Agent | grün auf Platin DE, HeartGold DE, HeartGold US (`xdelta3=true` je Spiel) | 2026-10-31 |
| (1) 3 gated `--test git` (`publish_then_commit_all_…`) | Agent | grün auf allen drei (`files=2`) | 2026-10-31 |
| (1) 4 gated Studio-Test (`publish_and_commit_over_the_command_path_…`) | Agent | grün auf allen drei (mit `build.steps` nicht leer) | 2026-10-31 |
| (1) 5 `cli_publish_prints_the_files_and_the_count` (gated Teil) | Agent | grün auf allen drei | 2026-10-31 |
| (2) 1 bis 3 Basis, Git, Trainer ändern | Agent | grün, alle drei Spiele | 2026-10-31 |
| (2) 4 bis 6 `publish_patch`: läuft ohne Frage, `version_taken`, `exists`, `overwrite` | Agent | grün, alle drei Spiele | 2026-10-31 |
| (2) 7 bis 9 Begleitdatei, `xdelta3`, `git status` | Agent | grün, alle drei Spiele (`xdelta3` gleich) | 2026-10-31 |
| (2) 10 bis 13 `git_commit_all`, `g4k publish --dir`, `g4k git commit-all`, Verweigerungen | Agent | grün, alle drei Spiele; über MCP antwortet ein `dir` unter einer Datei `error` statt `bad_dir` (Befund 1: so gewollt) | 2026-10-31 |
| (3) 1 bis 7 Assistent, Ergebnis, „Stand sichern“, Liste | Agent | grün; Befunde 2, 3, 6, 7 behoben, Nachtest bestanden | 2026-10-31 |
| (3) 8 bis 12 Sperren, „Ersetzen“, gesperrte Taste, ohne Git, ohne Commit | Agent | grün; 9 mit geänderter SHA-1 (Befund 9, so gewollt); Befunde 4, 5 behoben, Nachtest bestanden; 8 so gewollt | 2026-10-31 |
| (3) 13 MCP über die Brücke | Agent | grün | 2026-10-31 |
| (3) 14 Repository aus 4.4: „Stand sichern“ nach dem Veröffentlichen frei | Agent | grün | 2026-10-31 |
| Nachtest der Befunde 2 bis 7 auf `d8261cf` | Agent | bestanden (Bilder `nachtest-*.png`) | 2026-10-31 |
| `pnpm test` (apps/studio) auf `d8261cf` | Agent | grün: 333 Dateien, 3 406 Tests, 1 übersprungen, 154 s | 2026-10-31 |
| Rom Patcher JS mit der eigenen Basis-ROM | Hand | offen | – |
| „Im Finder zeigen“ in der gepackten App | Hand | offen | – |
| Eigenes Git-Werkzeug zeigt den Commit mit Body, ohne Remote | Hand | offen | – |
| Blick auf Titelleiste, Assistent, Fläche, Dialog | Agent + Blick | Agent: 53 Bilder aufgenommen (`target/agent/gate-2026-10-31/4-5/blick-*.png`, Liste in „Blick“); Blick des Nutzers offen | 2026-10-31 |
| `.fig` | – | nicht angefasst (OpenPencil nicht erreichbar); Vorlage und Anleitung `design-system/openpencil/NACHTRAG-4-5.md` liegen bereit, die Frames folgen | – |

Zahlentabelle (Release; Werte aus den gated Tests (1) und dem Szenario (2) des Gate-Laufs am 2026-10-31; die Implementer-Läufe stehen in (1)):

| Spiel | Patch (B) nach Trainer-Änderung | Build (ms) | Kodieren / Prüfen (ms) | Commit: Dateien, add (ms), commit (ms) | Status danach (ms) | xdelta3 gleich | Last |
|---|---|---|---|---|---|---|---|
| Platin DE | 552 (kopflos, studio gated); Karte (`--test base`) 553 | `--test base` 20 548; studio gated 42 888; kopflos 29 642 | 118 / 142; studio gated 176 / 185; kopflos 132 / 144 | 2, 53, 72 (`--test git`); studio gated 2 Dateien in 266 ms; kopflos 2 in 217 ms | 97; studio gated 141 | ja (`--test base` und kopflos) | 35,5 bis 16 |
| HeartGold DE | 557 (kopflos), 555 (studio gated); Karte 554 | 25 207; studio gated 32 733; kopflos 49 612 | 163 / 159; 149 / 142; 249 / 194 | 2, 45, 52; studio gated 2 in 185 ms; kopflos 2 in 304 ms | 94; studio gated 98 | ja | 10 bis 19 |
| HeartGold US | 557 (kopflos), 555 (studio gated); Karte 554 | 18 660; studio gated 35 317; kopflos 40 186 | 144 / 142; 175 / 157; 162 / 147 | 2, 38, 57; studio gated 2 in 270 ms; kopflos 2 in 256 ms | 75; studio gated 135 | ja | 6,7 bis 24 |

Der Build ist der Build von „Prüfen und Bauen“ (kalt oder warm je nach Zustand der Kopie): Er geht in die Spalte „Build (ms)“, Kodieren und Selbstprüfung stehen für sich. „xdelta3 gleich“ heißt: der Patch, mit `xdelta3 -d` auf die Basis-ROM angewandt, ergibt die SHA-1 der gebauten ROM. Der Wert steht in der zweiten `publish`-Zeile von `tests/base.rs` (`publish <Spielcode> write_ms=… record_ms=… ms=… second_ms=… xdelta3=…`); ist `xdelta3` nicht im `PATH`, steht dort das Fehlen und nur der eigene Decoder hat geprüft.

## Befunde

Aus der Umsetzung sind zwei Korrekturen schon drin und stehen nicht mehr offen: `cadcc12` (der Ort einer Veröffentlichung geht durch die Policy, bevor die Platte angesehen wird) und `3fba8c4` (der Build einer Veröffentlichung speist die Schritte und Probleme; „Prüfen“ schließt statt zurückzugehen, damit kein zweiter Lauf eine zweite Fassung schreibt). Fixes als eigene Commits `fix(4.5): …` mit Regressionstest.

Gate-Lauf des Agenten am 2026-10-31 (`a752a19`); keiner der Befunde sperrt einen Gate-Punkt:

| Nr. | Schwere | Punkt | Befund (was geschah, was erwartet) | Fix |
|---|---|---|---|---|
| 1 | niedrig | (2) 11 | MCP `publish_patch` mit `dir` **unter** einer Datei (in einem `--allow-dir`-Ordner, mit `--allow-irreversible`): `error` „Pfad nicht lesbar: <ort>/<datei>.xdelta/unter/<Projekt>-v0.2.1.xdelta: Not a directory (os error 20)“; erwartet nach Gate und `docs/mcp.md` `bad_dir` „Der Speicherort ist kein Ordner: <Pfad>“ (so antworten `g4k publish` und MCP für die Datei selbst). Ein fehlender Ordner antwortet mit dem rohen OS-Text „Pfad nicht lesbar: …: No such file or directory (os error 2)“. Die Pfadprüfung der Policy (`bridge/roots.rs`) kommt vor dem Kern. Nichts geschrieben, `.g4/releases.json` unverändert. | kein Fehler, so gewollt: über MCP entscheidet die Pfadprüfung der Policy, bevor der Ort auf der Platte angesehen wird (`docs/mcp.md`); `bad_dir` kommt über MCP nur für eine Datei in einem erlaubten Ordner, sonst aus `g4k` und der App. Gate-Text (2) 11 angepasst |
| 2 | mittel | (3) 2, 7, 8 | Fokus nach dem Assistenten: aus Technik › Veröffentlichen („Veröffentlichen…“) oder aus der Palette geöffnet, landet der Fokus nach „Abbrechen“, Escape oder „Schließen“ (auch nach einer Veröffentlichung) auf `body`, und der Atlas bleibt auf Welt (Technik kommt nicht zurück). Aus der Titelleiste geht er richtig auf „Veröffentlichen“ zurück. Erwartet: zurück auf den Auslöser (Technik › Veröffentlichen mit „Veröffentlichen…“) oder ein festes Ziel. Dasselbe Muster zeigt „Gebiet anlegen…“ aus der Palette (wohl gemeinsam in `WizardHost`). | behoben mit `503a676` (`useWizards.returnFocus`, App `onLeave`), der Ursprung bleibt beim ersetzenden Assistenten mit `d8261cf`; Nachtest 2026-10-31 bestanden: aus Technik › Veröffentlichen nach Escape, „Abbrechen“ und „Schließen“ nach einer Veröffentlichung wieder Technik › Veröffentlichen mit dem Fokus auf „Veröffentlichen…“; aus der Palette (Welt, Texte) und „Gebiet anlegen…“ aus der Palette der Fokus auf dem ersten Bedienelement des Bereichs („Bauen“), nicht `body`. Der Fall „ersetzender Assistent“ ist im Browser nicht erreichbar (⌘K und die Suche öffnen bei offenem Assistenten keine Palette), er steht im Unit-Test |
| 3 | niedrig | (3) 4, 8 | Während des Laufs (48 s) ist „Weiter“ `disabled` und der Fokus fällt auf `body`; nach der Ablehnung `exists` steht er auf dem README-Kästchen statt auf der Meldung oder „Ersetzen“. | behoben mit `503a676`; Nachtest bestanden: im Lauf bleibt der Fokus im Schritt (Dialog), nach `exists` steht er auf „Ersetzen“ (`nachtest-lauf-fokus-hell.png`, `nachtest-ersetzen-fokus-hell.png`) |
| 4 | niedrig | (3) 12 | „Stand sichern…“ in Technik › Änderungen in einem Repository ohne Commit: nach dem ersten Commit liegt der Fokus auf `body` (die Fläche wechselt von „Noch kein Commit“ zur Liste); mit vorhandenem Commit geht er zurück auf „Stand sichern…“. | behoben mit `503a676` (`CommitDialog` `finalFocus`); Nachtest bestanden: nach dem ersten Commit in einem Repository ohne Commit der Fokus auf dem neuen „Stand sichern…“ |
| 5 | niedrig | (3) 12 | Prüfliste ohne Commit: „18.449 geänderte Dateien seit dem letzten Commit“, obwohl es keinen gibt; erwartet etwa „… für den ersten Commit“. | behoben mit `503a676`; Nachtest bestanden: „18.449 Dateien für den ersten Commit“ (`nachtest-erster-commit-hell.png`) |
| 6 | niedrig | (3) 7 | Tabelle „Bisherige Veröffentlichungen“: die Spalte von „Im Finder zeigen“ hat einen leeren Kopf (`th` ohne Text, auch kein `aria-label`), und jede Zeile hat dieselbe Taste „Im Finder zeigen“ ohne Version oder Datei im Namen. Mit der Tastatur ist sie nur über Enter auf der Zeile erreichbar (Tab verlässt die Tabelle; Muster der `Table`, wie die anderen Tabellen) – als Hinweis. | behoben mit `503a676`; Nachtest bestanden: Spaltenkopf „Aktion“ (versteckt), Taste „Im Finder zeigen: v0.1.0“ (`nachtest-liste-hell.png`) |
| 7 | niedrig | (3) 4 | Nach der Veröffentlichung sagt die Statusleiste auf „Prüfen“ weiter „↵ Weiter, ⌘[ Zurück, Esc Abbrechen“, obwohl es kein Zurück gibt und die Taste „Schließen“ heißt (`blick-assistent-veroeffentlichen-pruefen-*.png`). | behoben mit `503a676`; Nachtest bestanden: auf „Prüfen“ nach der Veröffentlichung „↵ Weiter, Esc Schließen“ (`nachtest-pruefen-hinweis-hell.png`) |
| 8 | Hinweis | (3) 10 | Ein gescheiterter Commit (Hook) lässt die Änderungen gestaged (`M ` statt ` M` in `status --porcelain`); der Satz „Es ist nichts committet“ stimmt, „lässt das Repository, wie es ist“ (Review Focus) nur bis auf den Index. | so gewollt: das ist das getestete Verhalten des Plans (Task 3, `a_failing_commit_all_reports_git_stderr_and_keeps_the_repo`: `HEAD` bleibt, der Index behält, was `git add -A` aufnahm; der nächste Versuch nimmt ohnehin alles); steht so in [project_git.md](formats/project_git.md), „Stand sichern“ |
| 9 | Hinweis | (3) 9 | Die Probe „`.nds` umbenennen“ sperrt in der Test-Umgebung nicht: die Brücke sucht im erlaubten Ordner `_testkopien` und findet eine Kopie mit derselben SHA-1 (paralleler Phasen); `G4_BASE_ROM_DIR` (Vorbereitung 2) verstärkt das. Geprüft mit einer SHA-1 in `.g4/base.json`, zu der es keine ROM gibt. | kein Fehler: Gate-Text (3) 9 und agent-gate.md beschreiben jetzt die Probe mit der geänderten SHA-1 |

## Hinweise

- **Kernel, Zusatz zum Plan:** eine weitere Ablehnung `NOT_A_FOLDER` („Der Speicherort ist kein Ordner: <Pfad>“): der Ort ist eine Datei oder liegt darunter; MCP-Code `bad_dir`. Das Datum der `README.txt` ist nur der Tag.
- **`publish_patch` antwortet** den Bericht der Veröffentlichung flach (`release`, `patch`, `files`, `releases`, `phase_ms`, `ms`) und dazu `build` (der Bericht des Builds wie bei `create_patch`). Der Ort geht durch die Policy, bevor die Platte angesehen wird; ein `dir` muss existieren, außer `build/`.
- **`check_commit_all`** verweigert „kein Repository“, „umschließendes Repository“ und „kein `git`“, bevor Vorlagen geschrieben werden.
- **Assistent:** `WizardFrame` und `WizardBar` haben additiv `resultTitle` (die Pille „Fertig“) und `submitAt` und je Schritt `closed`; „Prüfen“ nach der Veröffentlichung zeigt „Schließen“ und kein „Zurück“. Die Schritte in Technik › Prüfen und Bauen tragen nach einer Veröffentlichung die Zeile „Build von „Veröffentlichen“ nach …“.
- **Technik › Änderungen:** „1 Datei“ im Toast steht im Singular; ein eigenes Repository ohne Commit bietet ebenfalls „Stand sichern…“ an.
- **Zeit:** „Stand sichern“ im Dialog nimmt die Ortszeit für den Betreff („Stand 2026-10-31 14:05“), `g4k git commit-all` ohne `--message` die UTC („Stand 2026-10-31 13:05 UTC“); die Liste der Veröffentlichungen und die Begleitdatei tragen UTC.
- **Zahl der Werkzeuge:** `tool_count_is_about_thirty` hat mit `publish_patch` und `git_commit_all` weiter Luft (die Grenze hebt bei Bedarf Task 4 mit Begründung an; offen mit 3d).

## Hand

- [ ] **Rom Patcher JS:** mit der eigenen Basis-ROM (zum Beispiel `Pokemon - Platin-Edition (Germany).nds`) und einem Patch `<Projekt>-v0.1.0.xdelta` aus (2) im Browser mit Rom Patcher JS angewandt: Die SHA-1 des Ergebnisses ist die `Ziel-ROM SHA-1` der Begleitdatei; die Basis-ROM hat die `Basis-ROM SHA-1` der Begleitdatei. Offen, wie in 4.1a (Rom Patcher JS ist ein Werkzeug des Nutzers im Browser; die Originale öffnet der Agent nie).
- [ ] **„Im Finder zeigen“ in der gepackten App:** `pnpm bundle`, die App starten, in Technik › Veröffentlichen „Im Finder zeigen“ in der Zeile einer Veröffentlichung und im Ergebnis des Assistenten: der Finder öffnet den Ordner mit dem Patch markiert (macOS). Unter Linux öffnet `xdg-open` nur den Ordner, unter Windows ist `explorer /select,` ungeprüft (kein Windows beim Agenten).
- [ ] **Eigenes Git-Werkzeug:** in einer veröffentlichten und gesicherten Kopie zeigt das eigene Werkzeug des Nutzers (Terminal, GitHub Desktop, Fork oder ähnlich) den Commit „Veröffentlicht v0.1.0“ mit der Notiz als Body, den Autor (der eigene Name, sonst „Gen4 Studio“) und kein Remote; `git status` ist leer; in `.gitignore` steht, dass `build/` und `*.xdelta` ignoriert sind, und `.g4/releases.json` ist im Commit.
- [ ] **Windows:** die Datei im Explorer markieren und `find_git` mit `%ProgramFiles%\Git\cmd\git.exe` (wie 4.4); kein Windows-Rechner beim Agenten.

## Blick

Bilder unter `target/agent/gate-2026-10-31/4-5/` im Worktree, je 1440 × 900, aus der Test-Umgebung über Chrome headless (Hell und Dunkel über `prefers-color-scheme`, Kompakt über die Taste der Statusleiste vor dem Öffnen von Assistent und Dialog: `data-dichte` allein lässt in einem offenen Modal die Beschriftung der Aktivitätsleiste stehen und gibt ein Mischbild). **Aufgenommen am 2026-10-31 vom Agenten, 53 Bilder** (Projekte `4-5-web`, `4-5-ui`, `4-5-neu`), dazu die Schrittbilder `ui-*.png` (Frage des Agenten „Patch ersetzen“ und „Stand sichern“, Palette und Titelleiste mit „Basis-ROM nicht gefunden“, Änderungen ohne Commit und ohne Git) und `blick-stand-sichern-ohne-git-hell.png`; `<thema>` ist `hell`, `dunkel` oder `kompakt`, der Blick des Nutzers steht aus:

- [ ] `blick-titelleiste-{frei,gesperrt}-<thema>.png`: die Taste „Veröffentlichen“ zwischen „Bauen“ und „Thema wechseln“, gesperrt mit dem Tooltip in zwei Zeilen („Erst eine Basis-ROM zuordnen.“).
- [ ] `blick-assistent-veroeffentlichen-{version,version-sperre,patch-lauf,ersetzen,pruefen,fertig}-<thema>.png`: der Assistent in allen vier Schritten, die Sperre am Feld „Version“, der Lauf mit der Phase, die Fehlermeldung mit „Ersetzen“, „Prüfen“ mit der Erfolgsmeldung und „Was du weitergibst“, das Ergebnis mit „Stand sichern…“ und „Im Finder zeigen“.
- [ ] `blick-veroeffentlichen-{leer,liste,kaputt,gesperrt}-<thema>.png`: Technik › Veröffentlichen ohne und mit Veröffentlichungen, mit kaputter Liste, ohne Basis-ROM.
- [ ] `blick-stand-sichern-{dialog,voreinstellung,lauf,gescheitert}-<thema>.png`: der Dialog ohne Voreinstellung, mit der Voreinstellung aus dem Assistenten, im Lauf und nach einem gescheiterten Commit.
- [ ] `blick-aenderungen-stand-sichern-<thema>.png` und `blick-statusleiste-nach-sichern-hell.png`: Technik › Änderungen mit der Taste „Stand sichern…“, die Statusleiste danach („main · keine Änderung“).

Vergleich mit dem Design-System: `design-system/README.md`, Abschnitt „Nachtrag 4.5“. Die Frames von 4.5 stehen noch nicht in der `.fig` (`design-system/openpencil/NACHTRAG-4-5.md`); verglichen wird mit der Vorlage `veroeffentlichen-assistent.body.js`, wenn die Frames da sind.

## Offen / später

Ohne Einfluss auf das Gate; was ein Nutzer bemerken kann (aus dem Plan, „Offen für die Umsetzung“):

- **Aus dem Gate-Lauf vom 2026-10-31:** die Befunde 2 bis 7 sind behoben (`503a676`, `d8261cf`; Nachtest bestanden), 1, 8 und 9 sind so gewollt (Gate-Text angepasst); offen bleiben `scripts/gate.sh` auf main nach dem Merge, die Punkte „Hand“ und der Blick des Nutzers auf die 53 Bilder.

- **`WizardRequest` und `WizardHost`:** `apps/studio/src/workbench/WorkbenchContext.tsx` und `app/WizardHost.tsx` sind nicht gesperrt, aber 3d.2 (Rezept-Taste, reduzierter Inspector) kann sie berühren; die Änderung von 4.5 ist je eine angehängte Zeile bzw. ein angehängter `case`. Trifft der Merge auf Konflikte, gilt: beide Anhänge behalten.
- **Ortszeit kopflos:** `g4k git commit-all` ohne `--message` nimmt den UTC-Betreff des Kernels; wer die Ortszeit will, gibt `--message`. Soll `g4k` Ortszeit schreiben, braucht der Kernel oder `g4k` eine Zeitzonen-Quelle (etwa das Crate `time` mit `local-offset`): eine eigene Entscheidung (Lizenz, `cargo deny`).
- **Build im Assistenten:** `publish_patch_checked` nimmt den Build-Weg von `create_patch_checked` (Speichern, Agenten-Zug, Build nach `build/<patch_name>.nds`); ändert sich dieser Weg durch 3d.2 (Rezepte schreiben Dateien), bleibt die Regel „der Patch ist der letzte Build, die ROM nicht die letzte ROM“.
- **`reveal_path` unter Linux und Windows:** `xdg-open <Ordner>` öffnet nur den Ordner, nicht die markierte Datei; `explorer /select,` unter Windows ist ungeprüft. Beides bleibt beim Nutzer.
- **Patch außerhalb von `build/` im Projekt:** wählt der Nutzer einen Ordner im Projekt außerhalb `build/`, ignoriert die Vorlage den `.xdelta` (`*.xdelta`), aber nicht `.xdelta.txt` und `README.txt`; „Stand sichern“ nimmt sie dann mit. Soll die Vorlage Texte neben Patches ignorieren, ist das eine Zeile `*.xdelta.txt` im Block (nicht in 4.5 entschieden).
- **`DSPRE`-Layout und neue Archive:** `commit_all` packt nicht aus (anders als `git_init`); ein nach dem Einrichten erstmals ausgepackter Ordner liefert eine NARC-Zeile in der Vorlage erst beim nächsten `write_templates`, die `commit_all` jetzt vor dem `add -A` macht; die bis dahin versionierte NARC bleibt im Index (Git entfernt ignorierte Dateien nicht aus dem Index). Eine Bereinigung (`git rm --cached`) ist kein Unterbefehl von `Sub` und bleibt außen vor.
- **Werkzeugzahl:** mit 3d.1 bis 3d.3 und 4.5 kann `tool_count_is_about_thirty` die Grenze erreichen; dann hebt Task 4 sie mit Begründung im Commit an.
- **`.fig`:** OpenPencil war beim Schreiben nicht erreichbar; Vorlage und Anleitung liegen bereit, die Frames folgen in einer eigenen Sitzung nach dem Merge (wie 4.4).
- **Nicht in 4.5:** Remote, Push, Pull, Branches, Merge, Staging, „Auf GitHub veröffentlichen“, Auto-Update, Signierung, Windows-Prüfung von xdelta UI; „Stand sichern“ committet immer alles (`add -A`), nie eine Auswahl.
