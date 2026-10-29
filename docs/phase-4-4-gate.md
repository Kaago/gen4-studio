# Phase 4.4: Gate prüfen

Stand: 2026-10-29. Branch `phase-4-4-git-minimal` (Worktree `.claude/worktrees/phase-4-4`, Basis main `6270c44`), nicht nach main gemergt. Gate vom Agenten bestanden auf dem Stand `01fc2d2`: (1) gated Tests im Release-Profil auf Platin DE, HeartGold DE und HeartGold US, (2) Szenario kopflos über `g4k` und `studio mcp` auf allen drei und (3) Browser-Teil und MCP über die Brücke (16 Schritte), ohne Befund. Davor fand der UI-Test am 2026-10-29 auf `a09637f` neun Punkte (U1 bis U9): U1 und U3 bis U7 behoben in `383e160`, `2a5e9cd` und `af1dbb5`, U2, U8 und U9 entschieden; das abschließende Review fand I1 bis I3 und acht kleinere Punkte (m1 bis m8), alle außer m7 behoben in `8ba4037` bis `01fc2d2` (Abschnitt „Befunde“). Das Gate der ganzen Phase 4 ist auf allen drei Spielen vom Agenten bestanden (Abschnitt „Gate der Phase 4“).

(1) 1. `scripts/gate.sh`: läuft (gestartet 19:39 auf `01fc2d2`; das Ergebnis kommt hier und im Abschnitt „Ergebnis“ nach).

Beim Nutzer offen: das eigene Git-Werkzeug zeigt das Repo, „Git einrichten“ in der gepackten App aus dem Finder, die Fortschrittsphasen in der App, Windows (Hand); der Blick auf Technik › Änderungen, die Dialoge und die Statusleiste in Hell, Dunkel und Kompakt (Abschnitte „Hand“ und „Blick“). Die `.fig` ist nicht angefasst; eine eigene Sitzung spielt die Frames nach dem Merge ein (`design-system/openpencil/NACHTRAG-4-4.md`).

Phase 4.4 bringt Git minimal: Vorlagen für `.gitignore` und `.gitattributes` als Metadaten im Kernel, je eine für das DSPRE-Layout und das Textprojekt (`* -text` für alle Spieldaten, `.g4/` als Erlaubnisliste mit `base.json` und `preview.png`, ROMs, Patches, `build/`, Spielstände und die Archive, die der Build packt, ignoriert); eine vorhandene Datei bekommt nur einen markierten Block am Ende. „Git einrichten…“ schreibt die Vorlagen, ruft `git init` und legt über die `git`-CLI den ersten Commit „Ausgangsstand“ an, nie ein Remote; der Satz dazu sagt „nur lokal oder privat, Gen4 Studio lädt nichts hoch“. Gelesen wird über `gix` im Prozess (Status, Blob aus `HEAD`, Branch), ohne installiertes Git. Technik › Änderungen vergleicht je Entität gegen die Basis-ROM, gegen den letzten Commit oder gegen den gespeicherten Stand (Umschalter), mit Feldtabelle aus demselben JSON-DTO, das 4.3 schreibt, Texten je Nachricht mit Wort-Spannen, Scripts als Zeilen-Diff und Karten je Teil, dazu „Öffnen“ und „Auf Original zurücksetzen…“. Die Statusleiste zeigt Branch und Zahl der geänderten Dateien. Ein Klon eines Textprojekts (ohne Spieldateien) wird über „Textprojekt vervollständigen?“ oder `g4k text restore` aus der eigenen Basis-ROM ergänzt. MCP bekommt `git_status`, `git_init` (Rückfrage), `diff_entity`; `g4k` die Befehle `git init`, `git status`, `git templates`, `diff` und `text restore`. Roadmap: [docs/ROADMAP.md](ROADMAP.md), Abschnitt „Phase 4“, Punkt „4.4 Git minimal“. Entscheidungen: [entscheidungen.md](research/2026-10-27-phase-4/entscheidungen.md), Abschnitt „Git (Gutachten C, 4.4)“. Plan: [2026-10-28-phase-4-4-git-minimal.md](superpowers/plans/2026-10-28-phase-4-4-git-minimal.md). Format: [project_git.md](formats/project_git.md); Werkzeuge: [mcp.md](mcp.md) (`git_status`, `git_init`, `diff_entity`).

Gate-Text der Roadmap (Punkt 4.4):

> Gate (zusätzlich zum Phasen-Gate): auf frischen Kopien der drei Spiele richtet „Git einrichten“ (kopflos über MCP und in der Test-Umgebung) Repo, Vorlagen und ersten Commit ein, `git status` bleibt nach `g4k build` leer; ein Klon mit `core.autocrlf=true` baut byte-gleich; ein geänderter Trainer und ein geänderter Text erscheinen mit Feldnamen gegen `HEAD` und gegen die Basis, „Auf Original zurücksetzen“ nimmt die Änderung als ein Undo-Schritt zurück; `cargo deny check licenses` grün mit `gix`; `scripts/gate.sh` grün.

Gate der Phase 4 (Roadmap, Abschnitt „Phase 4“):

> Gate: Projekt liegt in Git, `g4k build` erzeugt aus dem Textprojekt ein ROM, ein unverändertes Projekt rebuildet byte-identisch (auf Platin DE, HeartGold DE und HeartGold US).

Wer was prüft: Die Spalte „Wer“ sagt **Agent** (die Test-Umgebung aus [docs/agent-gate.md](agent-gate.md): Browser-Fenster mit `studio-bridge`, `call.sh`, `mcp.sh`, dazu `g4k`, `studio mcp` kopflos und die gated Tests), **Agent + Blick** (der Agent prüft und beschreibt Ort und Zahlen, der Nutzer bestätigt kurz am Bild) oder **Hand** (der Nutzer). Befehle: [docs/agent-gate.md](agent-gate.md), Abschnitt „Phase 4.4“.

Namen in der Oberfläche (Rollen und `data-*` für die Prüfung):

- **Statusleiste** `[data-git-status]`: „main · keine Änderung“, „main · 1 Änderung“, „main · 3 Änderungen“ (gezählt werden geänderte Dateien wie bei `git status`), losgelöst „8507d3f (losgelöst) · keine Änderung“, ohne Commit „main · noch kein Commit“, ohne Repo nichts, bei einem Lesefehler „Git: nicht lesbar“ (Grund im Tooltip). Tooltip „Git: keine geänderte Datei seit dem letzten Commit „Ausgangsstand“. Klick öffnet Technik › Änderungen.“, bei Änderungen „Git: 1 geänderte Datei …“ / „{n} geänderte Dateien … Klick zeigt sie in Technik › Änderungen.“. Klick öffnet Technik › Änderungen auf „Gegen letzten Commit“.
- **Technik › Änderungen** `[data-technik=aenderungen]`: Umschalter `[data-against]` (`aria-label` „Vergleichen mit“) mit „Gegen Basis“, „Gegen letzten Commit“, „Gegen gespeichert“; Kopfsatz „Gegenüber dem letzten Commit 5498fd6 „Ausgangsstand“ auf main …“, „Neu vergleichen“ `[data-compare-again]`. Ohne Repo die Info „Dieses Projekt liegt in keinem Git-Repository.“, der Satz „Git einrichten legt ein lokales Repository an; nur lokal oder privat, Gen4 Studio lädt nichts hoch.“ und „Git einrichten…“ (sekundär) in `[data-git-offer]`; Untertitel in der Liste „Kein Git“, „Keine Änderung seit dem Commit“, „2 Änderungen seit dem Commit“, „1 Eintrag ungespeichert“, ohne Änderung „Seit dem letzten Commit nichts geändert.“.
- **Liste** je Gruppe `[data-changes-group=<Gruppe>]` („Trainer“, „Textarchive“, „Karten“, „Projektdateien“ …), je Zeile `[data-change=<Name>]` mit Plakette „Geändert“, „Neu“, „Gelöscht“ oder „Fehlt“, dazu „Ungespeichert“. Liste und Detail stehen nebeneinander (`[data-changes-split]`, ab 760 px Breite der Aufteilung), schmaler übereinander.
- **Detail** `[data-entity-diff]`: Kopf „Trainer 0001“, „Commit 5498fd6 „Ausgangsstand“ → Projekt (gespeichert)“ (gegen Basis „Basis-ROM … → Projekt“, gegen gespeichert „Gespeichert → Ungespeichert“), Tasten „Öffnen“ und „Auf Original zurücksetzen…“ (gesperrt mit Grund, etwa „Für diese Art gibt es keinen Ort in der App.“). Darin die Änderungstabelle Feld / alt / neu (`data.party[0].level` 5 → 6; Pfade brechen nur an `.` und `[`), bei Texten Nachricht / Commit / Projekt mit `<ins>` und `<del>` je Wort, bei Scripts Zeilen-Diff (`[data-hunk]`, `[data-line]`), bei Karten die Teile (`[data-map-parts]`: Modell, Bewegungsrechte, Gebäude, BDHC) und „Zellen der Bewegungsrechte“, darunter „und {n} weitere Einträge“ (`[data-change-table-more]`).
- **Dialog „Git einrichten?“** `[data-git-init-dialog]`: Beschreibung, Warnung „Dein Projekt enthält Daten aus dem Spiel …“, Prüfliste `[data-check]` ✓ „Alles gespeichert“, ✓ „Git gefunden: 2.54.0 (Apple Git-157)“, (i) „Ignoriert: ROMs, Patches, build/, Spielstände und die Archive, die der Build packt“; „Abbrechen“ / „Einrichten“ (primär). Im Lauf keine Tasten, Fortschritt `[data-git-init-progress]` („Archive auspacken …“); danach die Meldung „Git eingerichtet: 18.450 Dateien im ersten Commit „Ausgangsstand“.“ und die Ansicht „Gegen letzten Commit“. Ohne Git: „Git ist nicht installiert. Die Vorlagen sind geschrieben; …“.
- **Dialog „Textprojekt vervollständigen?“** `[data-restore-dialog]`: Satz des Kernels „… Basis-ROM: Platin DE (Testprojekt).“ und „Gen4 Studio legt die Spieldateien aus deiner Basis-ROM an; deine Dateien bleiben, wie sie sind.“, „Abbrechen“ / „Vervollständigen“ (primär), Fortschritt `[data-restore-progress]` („Basis-ROM auspacken …“), danach „Textprojekt vervollständigt in 63 s.“.
- **Assistent „Neues Spiel aus ROM“:** im Schritt „Vorlage“ unter „Als Textprojekt einrichten“ das Kästchen „Git einrichten (nur lokal)“ mit „Legt ein lokales Repository mit erstem Commit an; nichts wird hochgeladen.“; Zusammenfassung „Original, als Textprojekt, mit Git“, Phase „Git einrichten“ im Ablauf; Ergebnis „Git eingerichtet: 6.576 Dateien im ersten Commit „Ausgangsstand“.“.
- **Palette:** ohne Repo „Git einrichten…“ (Symbol `git`).
- **Agent:** Frage `[data-agent-question]` „„agent-gate“ möchte: Git einrichten (lässt sich nicht rückgängig machen).“ mit „Ablehnen“ / „Zulassen“; `diff_entity` und `git_status` lesen ohne Rückfrage.
- **Nach „Als Textprojekt einrichten“ oder „Zurücknehmen“** in einem Projekt mit Git: „Die .gitignore von Gen4 Studio versioniert jetzt das Textprojekt.“ bzw. „… den DSPRE-Ordner.“ (I3).

## Vorbereitung

So durchgeführt am 2026-10-29 auf dem Stand `01fc2d2` ((1) und (2) 18:56 bis 19:38, (3) parallel 18:55 bis 19:17):

1. **Frische Kopien** unter `Roms/_testkopien/` per `cp -cR` (Ordner) und `cp -c` (`.nds`) aus den Originalen, die dabei nur gelesen wurden; Ordner `4-4-<zweck>_DSPRE_contents` und `.nds` `4-4-<zweck>.nds` direkt nebeneinander:
   - `4-4-pt` (Platin DE), `4-4-hgde` (HeartGold DE), `4-4-hgus` (HeartGold US) für (1); sie werden nur gelesen, die Tests schreiben auf `vanilla_copy()`, die Basis-ROM ist die Nachbar-`.nds`;
   - `4-4-kpt`, `4-4-khgde`, `4-4-khgus` (DSPRE-Layout) und `4-4-tpt`, `4-4-thgde`, `4-4-thgus` (Textprojekt) für (2), je mit `g4k base assign` auf die Nachbar-`.nds`; gebaute ROMs in `<name>-out/`, Klone in `<name>-klon/`;
   - `4-4-web` und `4-4-mcp` (Platin DE, Basis zugeordnet), `4-4-ui-text` (Basis zugeordnet, `g4k text adopt`, `g4k git init`), der Klon `4-4-ui-klon_DSPRE_contents` (`git init`, `core.autocrlf true`, `git fetch <ui-text> HEAD`, `git checkout FETCH_HEAD`) und `4-4-wiz.nds` (Assistent, Ziel `_testkopien/4-4-wiz`) für (3).

   Alle `4-4-*`-Kopien sind nach dem Gate gelöscht; `Manual`, `_saves` und eine `.DS_Store` des Finders bleiben.
2. **Werkzeuge:** `git --version` = `git version 2.54.0 (Apple Git-157)`. `cargo build --release -p g4-kernel --bin g4k`, `cargo build --release -p studio --bin studio` und die Release-Testbinaries (`cargo test --release -p g4-kernel --all-features --test git --test diff --no-run`, `cargo test --release -p studio --all-features --lib --no-run`), im Worktree ohne rotom-Download (Log `build-release.log`); für (3) `target/debug/g4k` auf `01fc2d2`. `tools/` ist der Symlink auf den Haupt-Checkout (nie einchecken). `G4_ROMS_DIR` absolut, `G4_BASE_ROM_DIR=<Roms>/_testkopien`, nie die Originale.
3. **Test-Umgebung** für (3): `G4_ROMS_DIR=<Roms> G4_VITE_PORT=1600 G4_BRIDGE_PORT=1601 bash scripts/agent/studio-web.sh` (Bau der Brücke 1 min 01 s), Chrome headless (`--headless=new`, CDP) mit einem kleinen Daemon im Scratchpad, 1440 × 900; Hell und Dunkel über `prefers-color-scheme`, Kompakt über die Taste der Statusleiste (bei offenem Modal über `data-dichte`); Fokus über `window.dispatchEvent(new Event("focus"))`.
4. **Ausgaben** unter `target/agent/gate-2026-10-29/4-4/` im Worktree: `gated-git-<k>.log` (Kernel `--test git` und `--test diff`), `gated-studio-<k>.log` (1), `kopflos-k<k>.log` und `kopflos-t<k>.log` (2), `build-release.log`, die Bilder `blick-*.png`, `gate3-*.png`, `ui-*.png` und `g4emu-klon/`; `scripts/gate.sh` nach `target/agent/gate-sh-4-4.log`. Die Skripte für (1) und (2) (Bash, Python über JSON-RPC/stdio, Protokoll 2025-11-25) und der CDP-Daemon lagen im Scratchpad. Den Ordner vor `git worktree remove` sichern.
5. **Last:** `uptime` vor und nach jedem Lauf (Spalte „Last“ der Zahlentabelle). (1) begann bei einer 1-Minuten-Last von 41,8, weil der Browser-Agent (3) gleichzeitig die Brücke baute und ein Textprojekt einrichtete (bis 34); danach 3,4 bis 7,5. Die drei Spiele liefen in (1) und (2) nacheinander, nie parallel; `scripts/gate.sh` erst danach, allein.

## (1) Automatik

(1) 1. `scripts/gate.sh`: läuft (seit 19:39 auf `01fc2d2`, allein, Log `target/agent/gate-sh-4-4.log`). Die neuen gated Tests laufen dort im Schritt der gated Tests je Projekt (Debug) ohne Änderung am Skript mit. `cargo deny` prüft im Skript `licenses advisories`; das Verbot von `git2` und `libgit2-sys` in `deny.toml` (`bans`) prüfte Task 1 (`c3fba26`, `cargo deny` grün ohne neue Ausnahme).

- [x] 2. **Gated Tests** `crates/g4-kernel/tests/git.rs` auf `4-4-{pt,hgde,hgus}`, Release, `--nocapture --test-threads 1`: je 5/5 bestanden (Platin DE 433 s, HeartGold DE 550 s, HeartGold US 414 s):
   - `git_setup_on_the_dspre_folder_keeps_status_empty_after_build`: einrichten, Status leer, `build::build`, Status leer; nach `git repack -a -d` gepackt 46 311 / 56 792 / 56 289 KiB (Grenze 65 MiB), 18 449 / 19 989 / 19 989 Dateien (Grenze 20 027), 36 / 40 / 40 ignorierte Archive; `add` 8,7 / 9,4 / 9,0 s, Commit 163 bis 170 ms, Status 211 / 182 / 215 ms.
   - `no_game_data_file_falls_under_an_eol_rule`: keine Datei unter `files/` und `unpacked/` trägt eine Endung mit `eol=lf` (18 345 / 19 882 / 19 882 Spieldateien); in den Code-Ordnern nur die fünf `.yaml` von ds-rom, ohne CR (132 / 139 / 139 Dateien).
   - `a_clone_with_autocrlf_builds_byte_equal_dspre`: Klon über `git init`, `config core.autocrlf true`, `fetch <Kopie> HEAD`, `checkout FETCH_HEAD`; Build in 12,3 / 17,0 / 17,0 s mit derselben SHA-1, Status leer, kein `[remote` in beiden `.git/config`.
   - `text_project_in_git_builds_from_base_byte_equal` (Gate der Phase 4): als Textprojekt eingerichtet (`adopt` 57,0 / 69,6 / 51,0 s), Git eingerichtet (6 576 / 6 304 / 6 304 Dateien, `add` 3,1 / 3,0 / 2,4 s, gepackt 3 120 / 3 232 / 3 107 KiB, Grenze 10 MiB), Build = SHA-1 der Basis-ROM, Status leer; Klon mit `core.autocrlf=true`, vervollständigt (`restore` 75,3 / 74,0 / 43,7 s), Build in 1,2 / 17,8 / 10,2 s = SHA-1 der Basis-ROM, Status leer.
   - `trainer_and_text_changes_show_with_field_names_against_head_and_base`: Trainer 1 Level +1 und Nachricht 0 in Textarchiv 1 + „ Studio“, gespeichert; die Liste gegen `HEAD` nennt „Textarchiv 0001“ und „Trainer 0001“ (aus 3 Pfaden); der Diff zeigt `party[0].level` und die eingefügte Wort-Spanne gegen `HEAD` und gegen die Basis mit denselben Zeilen, gegen gespeichert `same`; nach „Auf Original zurücksetzen“ und Speichern ist der Trainer gegen die Basis `same`. Status 202 / 172 / 188 ms, Liste 0 bis 2 ms, beide Diffs gegen `HEAD` zusammen 6 / 7 / 5 ms, gegen die Basis 0 ms.
- [x] 3. **Gated Test** `crates/g4-kernel/tests/diff.rs` `one_entity_diffs_fast_in_the_dspre_layout`, Release: 1/1 auf allen drei (34 / 33 / 31 s). Nach Änderungen an Trainer 1, Pokémon 25, Karte 0 und Header 0 sind diese gegen `HEAD` `Changed`, Attacke 1, Textarchiv 433 und Kartentextur 0 `Same`; die Karte mit genau einer Zelle. Kein Einzel-Diff über 12 ms (Grenze bei `G4_TIMING_STRICT=1`: 200 ms), `open_for_diff` 0 ms.
- [x] 4. **Gated Studio-Test** `git_and_diff_over_the_command_path_on_the_vanilla_copy` (`apps/studio/src-tauri/src/commands/git.rs`), Release: 1/1 auf allen drei (36 / 33 / 32 s). „Git einrichten“ über den Befehlsweg 8,1 / 8,8 / 7,4 s (Phase `Add` 7,8 / 8,5 / 7,2 s), Status danach 277 / 279 / 190 ms, nach Trainer 1 Level +1 genau ein Pfad und „Trainer 0001“ in 83 / 116 / 72 ms, Diff gegen `HEAD` 4 / 7 / 6 ms.

Befehle aus der Wurzel des Worktrees (`K=<Roms>/_testkopien`, `<k>` = `pt`, `hgde`, `hgus`):

```sh
export PATH="$HOME/.cargo/bin:/opt/homebrew/opt/rustup/bin:$PATH"
G4_TEST_PROJECT="$K/4-4-<k>_DSPRE_contents" \
  cargo test --release -p g4-kernel --all-features --test git -- --nocapture --test-threads 1
G4_TEST_PROJECT="$K/4-4-<k>_DSPRE_contents" \
  cargo test --release -p g4-kernel --all-features --test diff -- --nocapture --test-threads 1
G4_TEST_PROJECT="$K/4-4-<k>_DSPRE_contents" \
  cargo test --release -p studio --all-features --lib git_and_diff_over_the_command_path_on_the_vanilla_copy -- --nocapture --test-threads 1
```

Belegzeilen (Auszug aus `gated-git-<k>.log` und `gated-studio-<k>.log`):

```
clone CPUD dspre build_ms=12262 equal
git CPUD dspre files=18449 ignored_archives=36 unpack_ms=15 add_ms=8679 commit_ms=170 status_ms=211 packed_kib=46311
git CPUD text files=6576 add_ms=3114 commit_ms=65 packed_kib=3120
clone CPUD text restore_ms=75297 build_ms=1196 equal
diff CPUD status_ms=202 entries_ms=0 head_ms=6 base_ms=0
clone IPKD dspre build_ms=17023 equal
git IPKD dspre files=19989 ignored_archives=40 unpack_ms=11 add_ms=9352 commit_ms=163 status_ms=182 packed_kib=56792
git IPKD text files=6304 add_ms=2973 commit_ms=71 packed_kib=3232
clone IPKD text restore_ms=73980 build_ms=17807 equal
clone IPKE dspre build_ms=16968 equal
git IPKE dspre files=19989 ignored_archives=40 unpack_ms=13 add_ms=8985 commit_ms=166 status_ms=215 packed_kib=56289
git IPKE text files=6304 add_ms=2418 commit_ms=70 packed_kib=3107
clone IPKE text restore_ms=43661 build_ms=10198 equal
git init on the vanilla copy: 8.061796542s (18449 files, phases {Unpack: 12, Templates: 1, Init: 28, Add: 7846, Commit: 165})
status after init: 277 ms / status after the edit: 83 ms, 1 entries / diff against HEAD: 4 ms
```

Die Grenzen der Tests (gepackt DSPRE ≤ 65 MiB und ≤ 20 027 Dateien, Textprojekt ≤ 10 MiB; mit `G4_TIMING_STRICT=1` Status ≤ 500 ms und jeder Diff < 200 ms) halten alle Werte mit Abstand ein.

## (2) Szenario kopflos

Release-`g4k` und `studio mcp --project <Kopie> --base-rom <nds> [--allow-irreversible]` über JSON-RPC/stdio, je Spiel nacheinander (19:26 bis 19:35, Last 3,4 bis 7,5). DSPRE-Lauf 22 / 29 / 29 s, Textprojekt-Lauf 132 / 141 / 143 s (Platin DE / HeartGold DE / HeartGold US). Alles bestanden im ersten Versuch. Logs `kopflos-k<k>.log` und `kopflos-t<k>.log`.

**DSPRE-Kopie** `4-4-k<spiel>`:

- [x] 1. `g4k base assign`: `assigned sha1=d40f19ac… label=Platin DE (Testprojekt)`, HeartGold DE `13b6853a…`, HeartGold US `4fcded0e…`.
- [x] 2. MCP `git_status`: `repo {"kind":"none"}`, `changes` 0, `cli` 2.54.0, `layout` `dspre`.
- [x] 3. `git_init` ohne `--allow-irreversible`: `code: needs_confirmation`, „git_init lässt sich nicht rückgängig machen (Git einrichten); kopflos nur mit --allow-irreversible“. Mit: `committed: true`, `head.summary` „Ausgangsstand“, `.gitignore` und `.gitattributes` `created`, `ignored_archives` 36 / 40 / 40, `files` 18 450 / 19 990 / 19 990 (mit `.g4/base.json`), `studio_identity: false`, 8,6 / 9,4 / 9,6 s.
- [x] 4. `git_status` danach: `repo own`, Branch `main`, `changes` 0, 218 / 176 / 170 ms.
- [x] 5. `g4k build`: SHA-1 = `.nds` (11,6 / 17,9 / 17,9 s); `git status --porcelain` leer.
- [x] 6. `data_edit` Trainer 1 `party[0].level` +1 (Platin 5 → 6, HeartGold 14 → 15), `edit_text` Archiv 1 Nachricht 0 + „ Gate“, `save`; `git_status`: zwei Einträge „Textarchiv 0001“ und „Trainer 0001“ (3 Pfade, siehe „Hinweise“, O1), 103 / 101 / 94 ms.
- [x] 7. `diff_entity` `trainer`/`head`: `changed`, Feld `data.party[0].level` 5 → 6 (HeartGold 14 → 15), `old_label` „Commit 9dba537 „Ausgangsstand““; `text_archive`/`head`: `changed`, Nachricht 0 mit gleichem Text („SPIELER: {STRVAR_1, 3, 0, 0}“, „ZURÜCK“, „CANCEL“) und eingefügtem „ Gate“; `trainer`/`base`: dieselbe Zeile, `old_label` „Basis-ROM Platin DE (Testprojekt)“.
- [x] 8. Eine weitere Bearbeitung ohne `save`, `diff_entity` `saved`: `changed`, 6 → 7 (15 → 16), „Gespeichert“ → „Ungespeichert“.
- [x] 9. `reset_to_original` Trainer 1 (`recorded: true`, 117 ms), `save`, `diff_entity` `base`: `same`; `git_status` am Ende ein Eintrag „Textarchiv 0001“.
- [x] 10. `git -C <Kopie> remote -v` leer, `git log` nur „Ausgangsstand“.

**Textprojekt-Kopie** `4-4-t<spiel>`:

- [x] 11. `g4k base assign` (126 bis 141 ms), `g4k text adopt` (55,4 / 61,2 / 60,6 s; Platin 1 124 Scripts, 6 574 Dateien), `g4k git init`: `created .gitignore`, `created .gitattributes`, `layout=text_project ignored_archives=0`, `committed Ausgangsstand files=6576 branch=main` (HeartGold 6 304; 2,5 / 2,6 / 3,0 s).
- [x] 12. `g4k build` → SHA-1 = `.nds`; `git status --porcelain` leer.
- [x] 13. Klon: `git init`, `git config core.autocrlf true`, `git fetch <Kopie> HEAD`, `git checkout FETCH_HEAD` (fetch 122 ms, checkout 417 ms auf Platin), kein `git clone`.
- [x] 14. `g4k text restore <Klon> --base-rom <nds>`: 45,0 / 43,4 / 36,9 s (Platin: extract 4,1, merge 13,7, import 25,8, sync 1,2 s); auf stderr erwartet „724 text archives without build state: the JSON version applies.“ (HeartGold 829), weil der Klon noch keinen Build-Zustand hat.
- [x] 15. `g4k build <Klon>` → SHA-1 = `.nds` (0,97 / 4,4 / 6,2 s); im Klon `git status` leer, kein Remote.
- [x] 16. MCP `data_edit` `SPECIES_PIKACHU` Personal `attack` 55 → 56, `save`: `git status` ` M data/species/0025.json`; `g4k diff <Kopie> species SPECIES_PIKACHU --against base` → `field personal.attack 55 -> 56`, `state changed` (224 / 221 / 248 ms, mit Öffnen der Basis-ROM); `--against head` dieselbe Zeile (11 / 19 / 12 ms); `g4k git status --entries` → `changed Pokémon Pokémon 0025`.

Im Textprojekt heißt der Feldpfad `personal.attack` (so steht er in `data/species/0025.json`), im DSPRE-Layout heißt der Pfad des Trainers über MCP `data.party[0].level`.

```sh
P=$K/4-4-k<k>_DSPRE_contents; N=$K/4-4-k<k>.nds; G=target/release/g4k
$G base assign $P $N
target/release/studio mcp --project $P --base-rom $N                         # git_status; git_init → needs_confirmation
target/release/studio mcp --project $P --base-rom $N --allow-irreversible    # git_init → committed; data_edit, edit_text, save; diff_entity …
$G build $P --out <rom>; git -C $P status --porcelain                        # SHA-1 = $N; leer
T=$K/4-4-t<k>_DSPRE_contents
$G base assign $T $K/4-4-t<k>.nds; $G text adopt $T; $G git init $T; $G build $T --out <rom>
mkdir <klon> && git -C <klon> init -q && git -C <klon> config core.autocrlf true \
  && git -C <klon> fetch -q $T HEAD && git -C <klon> checkout -q FETCH_HEAD
$G text restore <klon> --base-rom $K/4-4-t<k>.nds; $G build <klon> --out <rom>   # SHA-1 = .nds
$G diff $T species SPECIES_PIKACHU --against base                             # nach data_edit + save über MCP
```

## (3) Browser-Teil

Test-Umgebung auf 1600/1601, Kopie `4-4-web` (Platin DE, Basis zugeordnet), Seite `http://127.0.0.1:1600/?project=<Roms>/_testkopien/4-4-web_DSPRE_contents`. 16 Schritte, jeder im ersten Versuch bestanden (18:55 bis 19:17, Last 2,2 bis 34 beim Einrichten des Textprojekts, am Ende 5,7). Der UI-Test vom selben Tag (`a09637f`, 13 Schritte) lief denselben Weg vorher einmal ganz, dazu Palette, Karte und Script im Klon und „Hier spielen“; seine Befunde stehen unter „Befunde“.

- [x] 1. Technik › Änderungen „Gegen letzten Commit“ ohne Repo: Info „Dieses Projekt liegt in keinem Git-Repository.“, Satz „… nur lokal oder privat, Gen4 Studio lädt nichts hoch.“, „Git einrichten…“ sekundär in `[data-git-offer]`, 123 px breit in einer Zeile von 880 px (U3); Untertitel „Kein Git“.
- [x] 2. „Git einrichten…“ öffnet „Git einrichten?“: Beschreibung, Warnung, Prüfliste ✓ „Alles gespeichert“, ✓ „Git gefunden: 2.54.0 (Apple Git-157)“ (U6), (i) „Ignoriert: …“, „Abbrechen“ / „Einrichten“ (primär). Im Lauf keine Tasten, „Archive auspacken …“ (Browser-Modus ohne `Channel`); nach 9,3 s die Meldung „Git eingerichtet: 18.450 Dateien im ersten Commit „Ausgangsstand“.“, die Ansicht „Gegenüber dem letzten Commit 5498fd6 „Ausgangsstand“ auf main …“, „Seit dem letzten Commit nichts geändert.“.
- [x] 3. Statusleiste `[data-git-status]` „main · keine Änderung“. Auf der Platte: `git log` = `5498fd6 Ausgangsstand`, `remote -v` leer, `status --porcelain` leer, 18 450 Dateien in `ls-files`.
- [x] 4. Trainer 0001 Level 5 → 6, speichern: „main · 1 Änderung“ 0,37 s nach dem Klick; ein Klick auf die Statusleiste öffnet Technik › Änderungen auf „Gegen letzten Commit“; Gruppe „Trainer“, Zeile „Trainer 0001 · Geändert“; Detail „Commit 5498fd6 „Ausgangsstand“ → Projekt (gespeichert)“, Tabelle Feld / Commit / Projekt `data.party[0].level` 5 → 6, der Pfad einzeilig (U4).
- [x] 5. „Öffnen“ springt nach Pokémon › Trainer › Teenager Gunther 0001 (Level 6).
- [x] 6. „Auf Original zurücksetzen…“ öffnet den Dialog von 4.1b „Trainer 0001 auf Original zurücksetzen?“ (Vorher / Nachher `Team 1 level` 6 → 5); „Zurücksetzen“: ein Schritt im Verlauf, Zeile mit „Ungespeichert“; ⌘Z nimmt ihn als ein Schritt zurück („Alles gespeichert“).
- [x] 7. „Gegen Basis“: dieselbe Zeile „Trainer 0001“, Detail Basis / Projekt 5 → 6, Hinweis „Noch nicht gebaut: …“; „Neu vergleichen“ 18,7 s (ungebaut, Debug-Brücke).
- [x] 8. „Gegen gespeichert“ nach einer weiteren ungespeicherten Änderung (Level 67): Plakette „Ungespeichert“, „Gespeichert → Ungespeichert“, 6 → 67, Untertitel „1 Eintrag ungespeichert“; danach ⌘Z.
- [x] 9. Nachricht 0 in Textarchiv 0001 + „ Studio“ (Texte-Fläche), speichern: „main · 3 Änderungen“ (Pfade, U2), Untertitel „2 Änderungen seit dem Commit“, Zeilen „Textarchiv 0001“ und „Trainer 0001“; Detail Nachricht / Commit / Projekt mit genau einem `<ins>` „ Studio“, kein `<del>`.
- [x] 10. Commit von außen und Fokus. Im DSPRE-Layout schreibt das Speichern einer Nachricht neben dem Member den ungetrackten Spiegel `expanded/textArchives/0001.json` (N1); `git -C <Kopie> commit -qam "Agent"` nimmt nur getrackte Dateien und lässt ihn aus. Nach dem Fokus (0,93 s) darum folgerichtig „main · 1 Änderung“ und nur die Datei-Zeile „Projektdateien · `expanded/textArchives/0001.json` · Neu“ mit Feld-Diff der neuen Datei, „Öffnen“ und „Zurücksetzen“ gesperrt; kein Widerspruch „Neu“ / „Keine Abweichung.“ mehr (U1). Nach `git add -A && git commit` und Fokus „main · keine Änderung“ (0,94 s). Gegenprobe ohne Spiegel: Trainer Level 6 → 7 gespeichert, `commit -qam "Agent Trainer"`, Fokus → „main · keine Änderung“ nach 0,97 s, „Seit dem letzten Commit nichts geändert.“.
- [x] 11. Die Auswahl bleibt nach „Öffnen“ und zurück und beim Wechsel `head` → `base` → `saved`, solange die Liste das Subjekt hat (U7). Nach einem Speichern über `call.sh` (Karte 0003) zeigt „Gegen Basis“ beim Fokus die Karte ohne „Neu vergleichen“ (U5).
- [x] 12. Assistent „Neues Spiel aus ROM“ mit `4-4-wiz.nds`, beide Häkchen: Zusammenfassung „Original, als Textprojekt, mit Git“, Ablauf mit „Textprojekt einrichten“ und „Git einrichten“; Lauf 57 s (im Browser-Modus nur „Lesen läuft“); Ergebnis „Git eingerichtet: 6.576 Dateien im ersten Commit „Ausgangsstand“.“; „Öffnen“ → „main · keine Änderung“, Technik › Änderungen auf „Gegen letzten Commit“; `git log` = `72be4e4 Ausgangsstand`.
- [x] 13. Klon `4-4-ui-klon` öffnen: Dialog „Textprojekt vervollständigen?“, „Vervollständigen“, Fortschritt „Basis-ROM auspacken …“, nach 63,5 s „Textprojekt vervollständigt in 63 s.“; Projekt offen, „8507d3f (losgelöst) · keine Änderung“ (der Klon per `checkout FETCH_HEAD` ist losgelöst); `git status --porcelain` leer mit `core.autocrlf=true`.
- [x] 14. MCP über die Brücke (`4-4-mcp`, Freigabe „Lesen und bearbeiten“): `git_status` → `repo.kind: none`, `cli.version` „git version 2.54.0 (Apple Git-157)“, `layout: dspre`; `git_init '{}'` → Frage `{kind: confirm, client: agent-gate, what: "Git einrichten"}`, Dialog `[data-agent-question]`, „Zulassen“ → `committed: true`, 18 450 Dateien, `phase_ms {unpack 9, templates 1, init 23, add 8792, commit 198}`, `ms` 9 041 (17,4 s mit Rückfrage); Seite „main · keine Änderung“; `remote -v` und `status --porcelain` leer. `diff_entity {"kind":"trainer","index":1,"against":"head"}` ohne Rückfrage, `state: same`, 4 ms.
- [x] 15. Gegenprobe zu I1: `diff_entity {"kind":"file","path":"../x","against":"head"}` und `"/etc/hosts"` → `not_diffable` „Verglichen wird nur, was im Projektordner liegt: ein relativer Pfad ohne „..“, nicht in .git, ein Ordner unter unpacked/ als ein einzelner Name.“.
- [x] 16. Konsole: nur `get_area_data` 400 (2×, beim ersten Laden, schon vor 4.4), `favicon.ico` 404, `open_project` 400 (Klon `restore_needed`, erwartet); keine Ausnahme. Brückenlog ohne Fehler und ohne Panik.

Der UI-Test prüfte dazu: die Palette („Git einrichten…“ nur ohne Repo), Karte 0003 gegen den Commit (Teile, Zelle `5, 7` 0080 → 0000, „Öffnen“ gesperrt mit „Für diese Art gibt es keinen Ort in der App.“), im Klon Pikachu Angriff 55 → 60, eine Nachricht und eine Script-Zeile `PlaySE 1500` (`git diff --stat` nur die drei Quellen), „Hier spielen“ im Klon (16,5 s, `out.nds` von Git ignoriert, `g4emu` bis ins Spielerzimmer), `g4_batch` mit `git_init` abgelehnt („git_init geht nicht in g4_batch“), ein zweites `git_init` mit Repo „Das Projekt liegt schon in einem Git-Repository; Gen4 Studio hat nur die Vorlagen ergänzt.“, „Ablehnen“ → `needs_confirmation`.

Nicht im Browser prüfbar: die Fortschrittsphasen (Git einrichten, Assistent, Vervollständigen; die Brücke hat keinen `Channel`), die Karte über die 3D-Schau malen (kein WebGL im Chrome headless, die Zelle kam über `call.sh set_permission`), „Hier spielen“ mit Emulator (gespielt über `g4emu`), ein Projekt ohne installiertes Git.

Zahlen der Oberfläche (Debug-Brücke): Git einrichten 9,3 s (UI) und 9,0 s (MCP, `add` 8,8 s), je 18 450 Dateien; Assistent mit Textprojekt und Git 57 s (6 576 Dateien); `g4k text adopt` 127,8 s bei Last bis 34; Klon `init`/`fetch`/`checkout` 0,6 s; Vervollständigen 63,5 s; Statusleiste nach dem Speichern 0,26 bis 0,37 s, nach einem Commit von außen und Fokus 0,93 bis 0,97 s; `diff_entity` über MCP 4 ms.

## (4) Hand und Blick

Beim Nutzer (Abschnitte „Hand“ und „Blick“ unten): das eigene Git-Werkzeug zeigt das eingerichtete Repo mit „Ausgangsstand“ und ohne Remote (Hand); „Git einrichten“ in der gepackten App, aus dem Finder gestartet, findet Git trotz kurzem `PATH` (Hand); die Fortschrittsphasen in der App (Hand); Windows bleibt ungeprüft (Hand); der Blick auf Technik › Änderungen, die Dialoge und die Statusleiste in Hell, Dunkel und Kompakt.

## Gate der Phase 4

> Projekt liegt in Git, `g4k build` erzeugt aus dem Textprojekt ein ROM, ein unverändertes Projekt rebuildet byte-identisch (auf Platin DE, HeartGold DE und HeartGold US).

Vom Agenten bestanden am 2026-10-29 auf `01fc2d2`, je Spiel auf einer frischen Kopie, zweimal unabhängig: im gated Test `text_project_in_git_builds_from_base_byte_equal` ((1) 2) und kopflos über `g4k` ((2) 11 bis 15).

| Bedingung | Platin DE | HeartGold DE | HeartGold US |
|---|---|---|---|
| Projekt liegt in Git: Textprojekt-Kopie mit Commit „Ausgangsstand“, `git status` leer | ✓ `e8970db Ausgangsstand`, 6 576 Dateien, `git status --porcelain` leer, auch nach dem Build; gated `git CPUD text files=6576 … packed_kib=3120` | ✓ `16ffd95`, 6 304 Dateien, leer; gated `packed_kib=3232` | ✓ `7800fc6`, 6 304 Dateien, leer; gated `packed_kib=3107` |
| `g4k build` erzeugt aus dem Textprojekt ein ROM | ✓ 25,4 s | ✓ 26,2 s | ✓ 33,3 s |
| Ein unverändertes Projekt rebuildet byte-identisch | ✓ SHA-1 `d40f19ac24d67c721a4ab25e1c013154443d75b4` = Basis-ROM; auch aus dem Klon mit `core.autocrlf=true` nach `g4k text restore` (Build 0,97 s), Klon `git status` leer; gated `clone CPUD text … equal` | ✓ `13b6853aa93121069df1d17446d3ecb36872aecf`, Klon ebenso (4,4 s); gated `clone IPKD text … equal` | ✓ `4fcded0e2713dc03929845de631d0932ea2b5a37`, Klon ebenso (6,2 s); gated `clone IPKE text … equal` |

Dazu im DSPRE-Layout: Git eingerichtet, `git status` nach dem Build leer, Klon mit `core.autocrlf=true` byte-gleich (`clone <code> dspre … equal`, (1) 2; (2) 1 bis 5).

Die Teilphasen haben die Bausteine einzeln abgenommen:

- **4.1a** ([phase-4-1a-gate.md](phase-4-1a-gate.md)): Basis-ROM zuordnen (`g4k base assign`), Rebuild eines DSPRE-Ordners SHA-1-gleich zur `.nds` auf allen drei, xdelta-Patch.
- **4.2** ([phase-4-2-gate.md](phase-4-2-gate.md)): Build-Graph mit Fingerabdrücken, inkrementell byte-gleich zum vollen Build, nach `git switch` gewinnt die JSON des Branches und `git status` bleibt leer.
- **4.1b** ([phase-4-1b-gate.md](phase-4-1b-gate.md)): „Auf Original zurücksetzen“ je Art, Member byte-gleich zur Basis; nach allen Resets ist die gebaute ROM SHA-1-gleich zur Basis (Abnahme).
- **4.3a** ([phase-4-3a-gate.md](phase-4-3a-gate.md)): Textprojekt im Kernel, Round-Trip, Export idempotent, Extract aus der Basis-ROM plus Textprojekt baut SHA-1-gleich (`textproject_from_base`).
- **4.3b** ([phase-4-3b-gate.md](phase-4-3b-gate.md)): das Textprojekt ist die Quelle des Studios, Umstellung und Rücknahme byte-gleich, Assistent „als Textprojekt“ SHA-1-gleich, ROM in melonDS spielbar.
- **4.4** (dieses Dokument): Projekt in Git, `.gitignore` und `.gitattributes` halten den Status nach dem Build leer, der Klon ohne Spieldateien wird aus der eigenen Basis-ROM vervollständigt und baut SHA-1-gleich.

## Ergebnis

| Punkt | Wer | Ergebnis | Datum |
|---|---|---|---|
| (1) 1 `scripts/gate.sh` | Agent | läuft (seit 19:39 auf `01fc2d2`) | – |
| (1) 2 gated `--test git` | Agent | bestanden: je 5/5 auf Platin DE, HeartGold DE, HeartGold US (Release); Status nach dem Build leer, Klon mit `core.autocrlf=true` byte-gleich im DSPRE-Layout und im Textprojekt, Trainer und Text mit Feldnamen gegen `HEAD` und Basis, Zurücksetzen | 2026-10-29 |
| (1) 3 gated `--test diff` | Agent | bestanden: 1/1 auf allen drei, kein Einzel-Diff über 12 ms | 2026-10-29 |
| (1) 4 gated Studio-Test | Agent | bestanden: 1/1 auf allen drei, Einrichten 7,4 bis 8,8 s, Diff 4 bis 7 ms | 2026-10-29 |
| (2) 1 bis 5 Git einrichten kopflos, Build | Agent | bestanden auf allen drei: `needs_confirmation` ohne Erlaubnis, `committed` mit, Status leer nach dem Build, SHA-1 = `.nds` | 2026-10-29 |
| (2) 6 bis 10 Trainer und Text gegen `HEAD`, Basis, gespeichert; Zurücksetzen; kein Remote | Agent | bestanden auf allen drei | 2026-10-29 |
| (2) 11 bis 16 Textprojekt, Klon, `g4k diff` | Agent | bestanden auf allen drei: Klon vervollständigt und SHA-1-gleich, `field personal.attack 55 -> 56` | 2026-10-29 |
| (3) 1 bis 13 und 16 Browser-Teil | Agent | bestanden auf `01fc2d2`, U1 und U3 bis U7 nachgeprüft, Konsole und Brückenlog ohne Fehler | 2026-10-29 |
| (3) 14 und 15 MCP über die Brücke | Agent | bestanden: Rückfrage „Git einrichten“, `diff_entity` ohne Rückfrage, Pfade außerhalb abgelehnt (I1) | 2026-10-29 |
| Gate der Phase 4 | Agent | bestanden auf allen drei Spielen (Abschnitt „Gate der Phase 4“) | 2026-10-29 |
| Eigenes Git-Werkzeug zeigt das Repo | Hand | offen | – |
| „Git einrichten“ in der gepackten App aus dem Finder | Hand | offen | – |
| Fortschrittsphasen in der App | Hand | offen | – |
| Windows (`find_git`, `core.autocrlf`) | Hand | offen (kein Windows-Rechner beim Agenten) | – |
| Blick auf Technik › Änderungen, Dialoge, Statusleiste | Agent + Blick | offen: 59 Bilder `blick-*.png` liegen bereit (dazu je 20 `gate3-*` und `ui-*`), vom Agenten angesehen | – |
| `.fig` | – | nicht angefasst (Anweisung der Sitzung); eine eigene Sitzung spielt die Frames nach dem Merge ein, Anleitung `design-system/openpencil/NACHTRAG-4-4.md` | – |

Zahlentabelle (Release; Werte aus den gated Tests (1), in Klammern aus dem Szenario kopflos (2)):

| Spiel | DSPRE: Dateien, gepackt (KiB), add (ms) | Textprojekt: Dateien, gepackt (KiB), add (ms) | Status (ms) | Diff `HEAD` / Basis (ms) | Klon vervollständigen (ms) | Build aus dem Klon = Basis | Last |
|---|---|---|---|---|---|---|---|
| Platin DE | 18 449, 46 311, 8 679 (MCP `git_init` 18 450 mit `.g4/base.json`, add 8 375, gesamt 8,6 s) | 6 576, 3 120, 3 114 (`g4k git init` gesamt 2 529) | 211 nach dem Build, 202 nach der Änderung (MCP 218 / 103) | 6 / 0 für Trainer und Text zusammen, Einzel-Diff höchstens 12 (`g4k diff` mit Öffnen der Basis 224) | 75 297 (kopflos 45 047) | ja, `d40f19ac…` | gated 41,8 → 6,3; kopflos 4,8 bis 5,2 |
| HeartGold DE | 19 989, 56 792, 9 352 (MCP 19 990, add 9 177, gesamt 9,4 s) | 6 304, 3 232, 2 973 (gesamt 2 645) | 182 / 172 (MCP 176 / 101) | 7 / 0, Einzel höchstens 11 (221) | 73 980 (43 424) | ja, `13b6853a…` | gated 5,8 bis 6,8; kopflos 3,4 bis 5,3 |
| HeartGold US | 19 989, 56 289, 8 985 (MCP 19 990, add 9 415, gesamt 9,6 s) | 6 304, 3 107, 2 418 (gesamt 3 021) | 215 / 188 (MCP 170 / 94) | 5 / 0, Einzel höchstens 12 (248) | 43 661 (36 881) | ja, `4fcded0e…` | gated 3,6 bis 7,1; kopflos 3,4 bis 7,5 |

„Gepackt“ ist `size-pack` nach `git repack -a -d` im gated Test: DSPRE 45,2 bis 55,5 MiB (Grenze 65 MiB), Textprojekt 3,0 bis 3,2 MiB (Grenze 10 MiB). Direkt nach dem Einrichten, vor einem `repack`, liegen lose Objekte: DSPRE 95 bis 107 MiB, Textprojekt rund 24 MiB (`git count-objects -v`). Der Status lag im Release-Build bei 172 bis 215 ms für rund 18 450 bis 19 990 Pfade, gut unter 0,5 s; die App liest ihn darum bei jedem Fensterfokus (nach 800 ms Entprellung) ohne zusätzliche Bremse.

Die Zeit für das Vervollständigen eines Klons streut mit der Last, vor allem in der Phase `import`: gated 75,3 s auf Platin DE (der Lauf begann bei einer Last von 41,8, als der Browser-Agent baute) und 74,0 s auf HeartGold DE (Phase `import` je rund 43 s), HeartGold US 43,7 s (`import` 22,7 s); kopflos bei Last um 5 nur 36,9 bis 45,0 s; in der Oberfläche über die Debug-Brücke 63,5 s (im UI-Test 90 s bei Last um 9). Eine Grenze prüft der Test nicht; in Task 4 lag derselbe Schritt bei Last 25 bis 35 noch bei 205 s und nach der Behebung von I2 (Basisschicht nur einmal) bei 52,8 s.

## Befunde

### UI-Test (2026-10-29)

Der UI-Test auf `a09637f` (Test-Umgebung 1600/1601, Chrome headless, 13 Schritte, 12 bestanden) fand einen wichtigen und acht kleine Punkte. U1 und U3 bis U7 sind behoben, je mit Regressionstest, Review sauber, und im Gate-Lauf (3) nachgeprüft; U2 bleibt nach der Planentscheidung, U8 ist ein Blick-Punkt, U9 ließ sich nicht nachstellen.

| Nr. | Schwere | Befund | Fix |
|---|---|---|---|
| U1 | Wichtig | Im DSPRE-Layout schreibt das Speichern einer Nachricht den ungetrackten Spiegel `expanded/textArchives/0001.json`. Nach `git commit -qam` (nur getrackte Dateien) zeigte die Liste „Textarchiv 0001 · Neu“, das Detail „Keine Abweichung.“; ein Nutzer sah „1 Änderung“ und fand nichts. | `383e160`: `head_entries` vergleicht eine neue Quelle unter `expanded/` (Textarchiv, Script), deren Entität sonst nicht in der Liste steht, einmal mit `diff_entity` gegen `HEAD`; gleich → Datei-Zeile „Projektdateien · `expanded/…json` · Neu“ mit Diff der Datei, verschieden → Entität „Geändert“. Tests `a_new_source_equal_to_heads_member_is_a_file_row_not_a_new_entity`, `a_new_script_source_equal_to_heads_member_is_a_file_row`. |
| U2 | Klein | Die Statusleiste zählt Dateien („3 Änderungen“), Technik › Änderungen Einträge („2 Änderungen seit dem Commit“). | Keine Änderung (Planentscheidung „Zahl der Änderungen“: die Leiste zählt wie `git status`; der Tooltip sagt „geänderte Dateien“). |
| U3 | Klein | „Git einrichten…“ (sekundär) war so breit wie die Fläche (880 px). | `2a5e9cd`: die Taste steht in `[data-git-offer]` (`display: flex`) und nimmt ihre Inhaltsbreite. |
| U4 | Klein | In der Änderungstabelle brachen Feldpfade mitten im Wort („data.party[0].leve“ / „l“), die drei Spalten waren gleich breit. | `2a5e9cd`, `af1dbb5`: Pfade brechen nur nach `.` und vor `[` (`<wbr>`, ohne Lookbehind, das WebKit vor 16.4 nicht kennt); die Feldspalte ist so breit wie der Pfad bis `min(28ch, 40cqi)`, die beiden Seiten teilen den Rest. |
| U5 | Klein | „Gegen Basis“ blieb nach einem Speichern an der Oberfläche vorbei (Brücke, außen) alt, bis „Neu vergleichen“. | `2a5e9cd`, `af1dbb5`: der Basisvergleich liest beim Fensterfokus neu wie der Git-Status, leise (ohne Fortschritt) und nur, solange „Gegen Basis“ zu sehen ist; sonst einmal beim nächsten Zeigen. |
| U6 | Klein | Prüfliste „Git gefunden: git version 2.54.0 (Apple Git-157)“. | `2a5e9cd`: „Git gefunden: 2.54.0 (Apple Git-157)“; Kernel, MCP und `g4k` behalten die volle Ausgabe. |
| U7 | Klein | Die Auswahl ging beim Wechsel des Vergleichs und nach „Öffnen“ verloren. | `2a5e9cd`: die Auswahl ist das Subjekt, sie gilt in jedem Vergleich, dessen Liste es hat, und überlebt das Verlassen von Technik. |
| U8 | Klein (Blick) | In Dunkel sind die Flächen von `<ins>` und `<del>` kaum vom Grund der Karte zu unterscheiden (`status/erfolg-flaeche` gegen `flaeche/blatt` 1,23 : 1, `status/fehler-flaeche` 1,35 : 1); es tragen Unter- und Durchstreichung. | Keine Änderung; der Nutzer entscheidet am Bild (Abschnitt „Blick“). |
| U9 | Klein | Einmal kam ein MCP-`git_init` sofort mit `needs_confirmation` „… in Gen4 Studio abgelehnt“ zurück, ohne sichtbaren Dialog (nach einem `g4_batch` mit falschem Argument). | Nicht nachgestellt: vier weitere Aufrufe im UI-Test und der Aufruf im Gate fragten und warteten wie vorgesehen; Brückenlog ohne Eintrag. |

### Abschließendes Review

Das Review des ganzen Branches (`6270c44..42b5ce0`) fand drei wichtige Punkte und acht kleinere; die Fix-Welle `8ba4037` bis `01fc2d2` behob alle außer m7, das Nach-Review sagte „Ready for the gate“.

| Nr. | Befund | Fix |
|---|---|---|
| I1 | Sicherheit: `diff_entity` las jeden Pfad (`file.path`, `member.folder` ungeprüft); ein Agent mit „Lesen“ hätte gegen `HEAD` Dateien außerhalb des Projekts lesen können (`/etc/hosts`, `../x`, `.git/config`, `/dev/zero` blockierte). | `8ba4037`: `DiffSubject::check` lehnt vor jedem Lesen ab, was kein relativer Pfad aus Namen ist, `.git` enthält oder als Ordner mehr als ein Name ist; Lesen nur über `Inside` (echter Pfad unter dem echten Projektpfad) und nur reguläre Dateien. Code `not_diffable`. Tests im Kernel und über MCP mit „Lesen“; im Gate (3) 15 nachgeprüft. |
| I2 | Der Status zwang unverfolgte Dateien in die Liste; in einem Dotfile-Repo mit `status.showUntrackedFiles=no` zählte das ganze Projekt als geändert. | `af5f9cf`: `no`, `false`, `off`, `0` (beliebige Schreibung) → keine unverfolgten Dateien, sonst Dateien. (gix 0.90 hielt das wörtliche `no` schon ein, die anderen Schreibungen nicht.) |
| I3 | Nach „Als Textprojekt einrichten“ oder „Zurücknehmen“ schrieb nichts den Block der Vorlagen um; die `.gitignore` passte nicht mehr zum Layout. | `461aaa9`: `adopt` und `revert` schreiben die Vorlagen neu, wenn die `.gitignore` den Block von Gen4 Studio trägt; die Meldung nennt es, ein Fehler ist nur eine Warnung. `project_git.md` sagt, dass schon versionierte Dateien bis `git rm -r --cached` verfolgt bleiben. |
| m1 | Ein Fehler in `head_entries` (gepatchte Dynamic Headers, eine Header-Datei fehlt) ließ den ganzen Status scheitern („Git: nicht lesbar“). | `99954e4`: die Pfade der Header werden dann als Datei-Zeilen gelistet, alle anderen Zeilen bleiben. |
| m2 | Das Detail las nach einem Commit oder `git switch` im eigenen Werkzeug nicht neu. | `09ee2fd`: gegen den letzten Commit liest das Detail neu, wenn sich `HEAD` bewegt. |
| m3 | Der Dialog „Textprojekt vervollständigen?“ zeigte rund 30 s „0 von 1“. | `09ee2fd`: ohne feinen Fortschritt keine Zahl. |
| m4 | Ein Enkelprozess von `git`, der die Pipes hält, hätte das Einrichten für immer blockiert. | `99954e4`: höchstens 2 s Nachlauf, dann gilt die Antwort. |
| m5 | `g4k git status --entries` listete ohne Commit jede Datei als neu. | `99954e4`: ohne Commit keine Einträge, wie App und MCP. |
| m6 | Tooltip „0 geänderte Dateien“ neben „keine Änderung“. | `09ee2fd`: „keine geänderte Datei“. |
| m7 | MCP `git_init` prüft nur den Store, nicht ungespeicherte Puffer des Script-Editors. | Nicht behoben (siehe „Offen / später“). |
| m8 | Ein kopfloser Agent bekam bei einem Klon `restore_needed` ohne Weg weiter. | `01fc2d2`: `docs/mcp.md` nennt `g4k text restore <Ordner>`. |

Dieselbe Welle: der Hilfetext von `g4k text restore` nennt die Phase `sync` (`99954e4`); die kleinen Punkte der Doku aus Task 10 (Farbe „—“ in der Vorlage, Wortlaut „Git: nicht lesbar“ im NACHTRAG, die gesperrten Fälle von „Öffnen“ im README, Status-Wort im Audit „umgesetzt (4.4)“, `01fc2d2`).

### Gate-Lauf (2026-10-29)

Keine Befunde in (1), (2) und (3); kein Lauf scheiterte, keiner wurde wiederholt.

## Hinweise

- **N1 / O1, der Textspiegel im DSPRE-Layout:** Das Speichern eines Textarchivs im DSPRE-Layout schreibt neben `unpacked/textArchives/NNNN` die Datei `expanded/textArchives/NNNN.json`; im Vanilla-Ordner gibt es `expanded/` nicht, sie ist also nie im Commit „Ausgangsstand“. `git_status` zählt sie als Pfad und ordnet sie „Textarchiv NNNN“ zu (kopflos „changes 3“ bei 2 Einträgen). `git commit -qam` lässt sie aus; danach steht die widerspruchsfreie Datei-Zeile „Projektdateien · `expanded/textArchives/0001.json` · Neu“ und „1 Änderung“, „keine Änderung“ erst nach `git add -A`. Ob die Vorlage des DSPRE-Layouts den Spiegel ignoriert, ist eine offene Entscheidung.
- **O3:** `g4k git init` druckt keine Phasenzeiten; „add“ des Textprojekts in der Zahlentabelle kommt aus dem gated Test, dazu die Gesamtzeit des Aufrufs.
- **O4:** `g4k text restore` meldet auf stderr „… text archives without build state: the JSON version applies.“ (Platin 724, HeartGold 829): erwartet im Klon ohne Build-Zustand; der Build danach ist byte-gleich.
- **O5 / Aufräumen:** ein offenes Finder-Fenster schrieb beim Löschen `.DS_Store` in die Kopien; der zweite Versuch löschte alles.
- **N2, Bildmethode:** Kompakt bei offenem Modal entsteht über `data-dichte` (die Taste der Statusleiste liegt unter dem Modal); die Leiste links behält dabei ihre Beschriftungen, abgeschnitten, mit einer Bildlaufleiste unten links (`blick-git-init-dialog-vorher-kompakt.png`). Das ist die Methode des Skripts, nicht die App.
- **Zahl der Werkzeuge:** MCP hat mit 4.4 60 Werkzeuge (Grenze des Tests 70).
- **Unter Last:** der Studio-Test `shared_text_export_asks_then_refuses_unsynced_changes` scheiterte einmal unter Last (Frage nicht innerhalb von 5 s), allein grün; im Lauf von `scripts/gate.sh` beachten.

## Entscheidungen bei der Umsetzung

Aus dem Protokoll der Umsetzung (je kurz, mit dem Grund; „falls falsch“ steht dort):

- **Lesen mit gix, Schreiben mit der CLI:** gix 0.90 mit festen Features, ohne `parallel`; `GitRepo` wird je Aufruf in einem Thread geöffnet (`spawn_blocking`), nie im Zustand der App gehalten (ohne `parallel` nicht `Send`/`Sync`). Die Schreib-Features für die Test-Fixture `GitFixture` kommen über das Kernel-Feature `test-support`, nicht in die normale Abhängigkeit.
- **Block der Vorlagen:** ein abweichender Block wandert immer ans Ende der Datei (Zeilen des Nutzers bleiben byte-gleich und verlieren den Vorrang; spätere Zeilen gewinnen bei `* -text` und Ignorier-Regeln); jeder alte Block wird vorher entfernt, ein verschachtelter Anfang gilt als kaputt. Beide Vorlagen ignorieren `/.rotom/command_database/.g4-ids-*` (erzeugte ID-Dateien für rotom).
- **Einrichten:** die Phase „Auspacken“ überspringt Arten, die `ensure_unpacked` als nur gespeichert ablehnt (Dynamic Headers). Die Identität prüft `git -c user.useConfigOnly=true var GIT_COMMITTER_IDENT`; ohne eingetragenen Namen ist der Autor „Gen4 Studio“ (sonst landete der Rechnername im Commit), die Konfiguration des Nutzers bleibt unberührt. Ein scheiterndes `git init` hat einen eigenen Satz. `G4_GIT` gilt nur als absoluter Pfad.
- **Zeilenenden:** die Probe verlangt null Dateien mit `eol`-Regel unter `files/` und `unpacked/`; in den Code-Ordnern sind nur die `.yaml` von ds-rom ohne CR erlaubt (Build-Konfiguration, `eol=lf` hält sie byte-gleich, der Klon-Test bestätigt es).
- **Klon vervollständigen:** eine fremde Basis-ROM lehnt der vorhandene Satz `other_rom` ab. Vor dem Import werden alle Archive ausgepackt (sonst packt der Build ein kürzeres Script-Archiv). Der Ordner wird aus der Basisschicht gefüllt (`BaseLayer::ensure`, dann Kopie, auf APFS als `clonefile`, nie überschreibend), statt die ROM ein zweites Mal auszupacken; abweichend vom Plan („nach `.g4/tmp/` auspacken … verschieben“), mit derselben Absicht. `header.yaml` kommt zuletzt; ein abgebrochener Lauf lässt das Projekt als „zu vervollständigen“ stehen und ein neuer Lauf setzt fort.
- **Diff:** der Export in den Speicher nutzt das vorhandene `EntitySource::export_bytes` / `export_members` statt eines neuen `export_unit`. Ein `HEAD` ohne Header-Datei (Dynamic Headers nach dem Commit angewandt) fällt auf den Ausschnitt aus `arm9.bin` zurück; ein Script mit Quelltext auf einer Seite und Member auf der anderen wird dekompiliert verglichen, wenn eine rotom-Konfiguration da ist, sonst als Bytes. Ein geändertes `mapname.bin` im DSPRE-Layout zerfällt in die Header, deren Namen sich unterscheiden (wie `arm9.bin`).
- **MCP:** `diff_entity` nimmt flache Argumente `{kind, index?, folder?, path?, against}` (symbolisches `index` braucht es); Katalogsätze sind mit „;“ verbunden, um die Grenze von drei Sätzen zu halten. `git_init` hält den Store beschäftigt wie die Umstellung, steht in `LONG_TOOLS` und nicht in `g4_batch`; die gefundene CLI wird in der App zwischengespeichert; ohne Commit gibt `git_status` keine Einträge.
- **Ansicht:** nach dem Assistenten mit Git startet Technik › Änderungen auf „Gegen letzten Commit“; richtet ein Agent Git ein, wechselt die Ansicht des Nutzers nicht.
- **U1, U2:** eine neue Quelle, die dem Member in `HEAD` gleicht, ist eine Datei-Zeile, keine neue Entität. Die Statusleiste zählt weiter Pfade wie `git status`, Technik zählt Entitäten.
- **Blick der Tasks 8 und 9:** einmal gemeinsam im UI-Test nach Task 9 (eine Test-Umgebung statt zwei).
- **Abschließendes Review:** I1 bis I3, m1 bis m6 und m8 in der Fix-Welle; m7 bleibt offen (Scripts speichert der Editor sofort, die Puffer gibt es nur im Frontend).
- **Ablauf:** Commit-Trailer nach der Regel des Auftrags; alle Teilagenten mit demselben Modell; die Nachprüfung der kleinen Fix-Runde von Task 1 las der Steuer-Agent selbst (15 Zeilen); kleine Punkte eines Reviews kamen, wo sie dieselben Dateien berührten, in den Auftrag der nächsten Task (Task 3 → 4, Task 5 → 6).

## Hand

- [ ] **Eigenes Git-Werkzeug:** in einem mit „Git einrichten…“ eingerichteten Projekt (DSPRE-Ordner oder Textprojekt) zeigt das eigene Werkzeug des Nutzers (Terminal, GitHub Desktop, Fork oder ähnlich) den Branch `main` mit genau einem Commit „Ausgangsstand“, den Autor (der eigene Name, wenn Git ihn kennt, sonst „Gen4 Studio“) und kein Remote; `git status` ist leer. Nach einer Änderung im Studio zeigt das Werkzeug dieselben Pfade, die die Statusleiste zählt; ein Commit dort setzt die Statusleiste beim Zurückkehren auf „keine Änderung“.
- [ ] **Gepackte App aus dem Finder:** `pnpm bundle`, die App aus dem Finder (nicht aus dem Terminal) starten; ihr `PATH` ist dann kurz (`/usr/bin:/bin:/usr/sbin:/sbin`). „Git einrichten…“ findet Git trotzdem (Prüfliste „Git gefunden: …“; `find_git` sucht nach `G4_GIT` und `PATH` an den üblichen Orten, etwa `/opt/homebrew/bin`, `/usr/local/bin`, `/usr/bin`) und legt den Commit an.
- [ ] **Fortschrittsphasen in der App:** „Git einrichten“ (Auspacken, Vorlagen, `git init`, Hinzufügen, Commit), die Phase „Git einrichten“ im Assistenten und „Textprojekt vervollständigen“ (Auspacken, Zusammenführen, Import, Abgleich); die Brücke hat keinen `Channel`, im Browser stand nur die erste Phase.
- [ ] **Windows:** `find_git` mit `%ProgramFiles%\Git\cmd\git.exe` und ein Klon mit `core.autocrlf=true` unter Windows sind nur über die Klon-Probe auf macOS geprüft (dort byte-gleich); kein Windows-Rechner beim Agenten.

## Blick

Bilder unter `target/agent/gate-2026-10-29/4-4/` im Worktree, je 1440 × 900 (die schmalen 900 px breit), aus der Test-Umgebung über Chrome headless (Hell und Dunkel über `prefers-color-scheme`, Kompakt über die Taste der Statusleiste, bei offenem Modal über `data-dichte`, siehe N2). Die Bilder der Änderungsfläche und des Dialogs sind im Gate-Lauf auf `01fc2d2` neu aufgenommen (nach U1, U3, U4, U6); die übrigen stammen aus dem UI-Test auf `a09637f`, ihr Aussehen hat sich nicht geändert. Der Agent hat alle angesehen: keine abgeschnittenen Texte, kein waagerechtes Rollen, je Dialog und Karte höchstens eine primäre Taste, Kompakt ohne Überlappung. Beim Nutzer:

- [ ] **Kontrast in Dunkel (U8):** die Flächen von `<ins>` und `<del>` gegen die Karte 1,23 : 1 und 1,35 : 1, es tragen Unter- und Durchstreichung; kräftiger machen oder so lassen. Bilder `blick-aenderungen-commit-textarchiv-dunkel.png`, `blick-klon-commit-text-dunkel.png`.
- [ ] `blick-aenderungen-commit-ohne-repo-{hell,dunkel,kompakt}.png`: „Gegen letzten Commit“ ohne Repo, Info, Satz „… nur lokal oder privat …“, „Git einrichten…“ in Inhaltsbreite.
- [ ] `blick-git-init-dialog-vorher-{hell,dunkel,kompakt}.png`: Dialog „Git einrichten?“ mit Warnung, Prüfliste „Git gefunden: 2.54.0 (Apple Git-157)“, „Abbrechen“ / „Einrichten“.
- [ ] `blick-git-init-dialog-lauf-hell.png`: derselbe Dialog im Lauf, ohne Tasten, „Archive auspacken …“.
- [ ] `blick-aenderungen-commit-leer-{hell,dunkel,kompakt}.png`: nach dem Einrichten „Seit dem letzten Commit nichts geändert.“, Statusleiste „main · keine Änderung“.
- [ ] `blick-aenderungen-commit-trainer-{hell,dunkel,kompakt}.png`: gegen den Commit, Trainer 0001 „Geändert“, Feldtabelle `data.party[0].level` einzeilig 5 → 6, „main · 1 Änderung“.
- [ ] `blick-aenderungen-basis-trainer-{hell,dunkel,kompakt}.png`: gegen die Basis, Basis / Projekt 5 → 6, „Noch nicht gebaut …“.
- [ ] `blick-aenderungen-gespeichert-trainer-{hell,dunkel,kompakt}.png`: gegen gespeichert, „Ungespeichert“, 6 → 67, Titelleiste „1 Änderung nicht gespeichert“.
- [ ] `blick-aenderungen-commit-textarchiv-{hell,dunkel,kompakt}.png`: Textarchiv 0001 mit `<ins>` „ Studio“, Feldspalte schmal; „main · 3 Änderungen“ neben „2 Änderungen seit dem Commit“ (U2).
- [ ] `blick-aenderungen-commit-expanded-neu-{hell,dunkel,kompakt}.png`: nach `commit -qam` die Datei-Zeile `expanded/textArchives/0001.json` „Neu“ mit Diff der Datei, Tasten gesperrt (U1, N1).
- [ ] `blick-aenderungen-commit-karte-{hell,dunkel,kompakt}.png`: Karte 0003, Teile (Bewegungsrechte geändert), Zelle `5, 7` 0080 → 0000, „Öffnen“ gesperrt.
- [ ] `blick-aenderungen-schmal-basis-trainer-{hell,dunkel,kompakt}.png`: 900 px, Detail unter der Liste, kein waagerechtes Rollen; Karte 0003, Textarchiv 0001, Trainer 0001 (U5).
- [ ] `blick-klon-commit-pikachu-{hell,dunkel,kompakt}.png`: Klon gegen Commit 8507d3f (losgelöst), Pokémon 0025 `personal.attack` 55 → 60.
- [ ] `blick-klon-commit-text-{hell,dunkel,kompakt}.png`: Klon, Textarchiv 0001, Nachricht 0 mit `<ins>` „ Studio“.
- [ ] `blick-klon-commit-script-{hell,dunkel,kompakt}.png`: Klon, Scriptdatei 0001 als Zeilen-Diff mit „+“-Zeile (Statusleiste dort noch „3 Änderungen“ aus dem UI-Test).
- [ ] `blick-assistent-vorlage-git-aus-{hell,dunkel,kompakt}.png`: Assistent, Schritt „Vorlage“, „Git einrichten (nur lokal)“ unter „Als Textprojekt einrichten“, beide aus.
- [ ] `blick-assistent-zusammenfassung-git-{hell,dunkel,kompakt}.png`: „Original, als Textprojekt, mit Git“, Ablauf mit „Textprojekt einrichten“ und „Git einrichten“.
- [ ] `blick-assistent-ergebnis-git-{hell,dunkel,kompakt}.png`: „Git eingerichtet: 6.576 Dateien im ersten Commit „Ausgangsstand“.“.
- [ ] `blick-assistent-danach-aenderungen-commit-{hell,dunkel,kompakt}.png`: das neue Projekt offen, Änderungen auf „Gegen letzten Commit“, „main · keine Änderung“.
- [ ] `blick-vervollstaendigen-dialog-{hell,dunkel,kompakt}.png`: „Textprojekt vervollständigen?“ mit Satz des Kernels, „Abbrechen“ / „Vervollständigen“.
- [ ] `blick-vervollstaendigen-lauf-hell.png`: im Lauf „Basis-ROM auspacken …“, ohne Tasten.
- [ ] `blick-mcp-frage-git-einrichten-{hell,dunkel,kompakt}.png`: Agentenfrage „„agent-gate“ möchte: Git einrichten (lässt sich nicht rückgängig machen).“, „Ablehnen“ / „Zulassen“.

Schritt-Bilder ohne Blick-Auftrag: `gate3-*.png` (20, Gate-Lauf (3): ohne Repo, Dialog im Lauf, nach dem Einrichten, Statusleiste, Trainer, „Öffnen“, Reset-Dialog, nach `commit -qam`, Assistent, Klon, Agentenfrage) und `ui-*.png` (20, UI-Test, teils vor den Fixes); `g4emu-klon/start.png` (gebaute ROM des Klons im Spielerzimmer). Vergleich mit dem Design-System: `design-system/README.md`, Abschnitt „Nachtrag 4.4“. Die `.fig` ist nicht angefasst; eine eigene Sitzung spielt die Frames nach dem Merge ein (`design-system/openpencil/NACHTRAG-4-4.md`).

## Offen / später

Ohne Einfluss auf das Gate; was ein Nutzer bemerken kann:

- **Spiegel der Texte im DSPRE-Layout (N1):** ob die DSPRE-Vorlage `expanded/` ignoriert; heute steht nach `commit -a` eine Datei-Zeile „Neu“, bis sie mit `git add -A` im Commit ist. Nach einer Massenänderung mit `commit -a` vergleicht die Liste je solcher Datei einmal (U1-Weg); bei sehr vielen ungetrackten Quellen wird sie langsamer.
- **MCP `git_init` und ungespeicherte Scripts (m7):** der Dialog sperrt „Einrichten“ bei ungespeicherten Script-Puffern, ein Agent nicht; der erste Commit eines Agenten kann einen ungespeicherten Puffer verpassen.
- **`text_export` und die Vorlagen (r1):** `g4k text export` und MCP `text_export` stellen einen DSPRE-Ordner mit Git ebenfalls um, schreiben die Vorlagen aber nicht um (anders als `adopt`, I3).
- **`arm9.bin.backup`** (geschrieben vom Patch der Dynamic Headers) steht als Datei-Zeile in der Liste; eine Zeile in der Vorlage wäre mit dem Nutzer zu entscheiden.
- **Klon vervollständigen:** die Phase `import` hat keinen feinen Fortschritt (22 bis 45 s); eine Datei namens `unpacked` lässt es jedes Mal scheitern; die App öffnet das Projekt danach zweimal.
- **Diff:** der interne Name eines Headers der Basis kommt aus der `mapname.bin` des Projekts (wie in 4.1b); fehlt der alten Seite ein späterer Member, borgt sie ihn von der Platte (kann eine neue Entwicklung verdecken); Dateien ohne NNNN-Namen im Ordner der Dynamic Headers fehlen in der Liste, wenn eine Seite gepatcht ist; ein Lesefehler einer Eventdatei lässt „Öffnen“ ohne Grund warten; „Gelöscht“ gegen den Commit ist mit „Kein Original“ gesperrt (Regel aus 4.1b); Typwechsel plus Änderung erscheint nur als „Geändert“.
- **Oberfläche:** eine zusätzliche Zeile „Git wird gesucht …“ in der Prüfliste, solange der Status fehlt; die Live-Region der Statusleiste beginnt mit ihrem ersten Text.
- **Windows:** der Kernel behält `\\?\`-Pfade in `Enclosing.workdir` (die App zeigt sie ohne); eine `.gitignore` mit CRLF bekommt einen Block mit LF (gemischte Zeilenenden, Git kommt damit zurecht).
- **Aus dem Plan offen:** Sicherungen in der Wurzel nimmt der Export weiter als Blobs auf (ob der Bereich der Blobs von Git ignorierte Pfade auslässt, entscheidet eine Phase nach 4); ein Repo, das einem anderen Nutzer gehört (`safe.directory`), nennt den Fehler von gix im Satz `Git: …`, „Git einrichten“ wird dort nicht angeboten; die Ressource `g4://changes` kommt nicht (`git_status` und `changed_from_original` geben Agenten dieselbe Sicht).
