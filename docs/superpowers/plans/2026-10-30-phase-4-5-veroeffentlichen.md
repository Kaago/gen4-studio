# Phase 4.5 „Veröffentlichen“ Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Der Assistent „Veröffentlichen“ (Titelleiste, Technik › Veröffentlichen, Palette) führt in vier Schritten (Version, Patch, Prüfen, Fertig) von einer Versionsnummer mit Änderungsnotizen zu einem selbstgeprüften xdelta-Patch `<Projekt>-v<Version>.xdelta` am gewählten Ort samt Begleitdatei (mit Version, Notizen, Datum, Studio-Version) und wahlweise `README.txt`; jede Veröffentlichung steht in `.g4/releases.json`, und Technik › Veröffentlichen zeigt sie als Liste „Bisherige Veröffentlichungen“. „Stand sichern“ legt mit einem Klick (Technik › Änderungen, Schritt Fertig des Assistenten, Palette) über die `git`-CLI einen Commit aller Änderungen an („Stand <Datum> <Uhrzeit>“ oder „Veröffentlicht v<Version>“, Notiz als Body); danach zeigt die Statusleiste „main · keine Änderung“. MCP bekommt `publish_patch` (Rückfrage nur beim Ersetzen einer Datei) und `git_commit_all` (Rückfrage), `g4k` die Befehle `publish` und `git commit-all`.

**Architecture:** Der Kernel erweitert `patch.rs` um die Untermodule `patch/release.rs` (die Liste `.g4/releases.json`, Versionsvorschlag, Prüfung, Dateiname) und `patch/publish.rs` (Ziele am gewählten Ort, Vorprüfung „Datei vorhanden“, Patch über das vorhandene `create_patch` mit erweiterter Begleitdatei, `README.txt`, Eintrag in die Liste); `git/cli.rs` bekommt `commit_all` neben `git_init` (Vorlagen schreiben, `add -A`, `commit` mit Betreff und Body, dieselbe Studio-Identität nur für diesen Commit). Die App bekommt vier Befehle (`releases`, `publish_patch`, `reveal_path`, `git_commit_all`) in einer neuen Datei `commands/publish.rs` und einem Anhang in `commands/git.rs`, zwei MCP-Werkzeuge, den Assistenten `publish/PublishWizard.tsx` im Scharnier (Muster `WizardFrame`, gehostet in `WizardHost`), den Dialog `git/CommitDialog.tsx`, die Taste „Veröffentlichen“ in der Titelleiste und die umgebaute Fläche Technik › Veröffentlichen (Kopf, Taste „Veröffentlichen…“, Liste; die Karte „Basis-ROM“ bleibt).

**Tech Stack:** Rust (g4-kernel: `patch`, `git::cli`, serde_json mit `preserve_order`, `fsutil::write_atomic`; die `git`-CLI über `std::process::Command` wie in 4.4), Tauri-Befehle mit `Channel` für den Fortschritt, MCP (Befehlstabelle, Katalog, Zugriff mit `Kind::Depends`), React 19 mit den Bausteinen aus `apps/studio/src/ui/`, vitest.

**Spec:** [2026-10-30-phase-3d-erzaehlen-design.md](../specs/2026-10-30-phase-3d-erzaehlen-design.md) (§1 letzter Absatz, §4 Zeile 4.5, §5 Zeile „Veröffentlichen-Assistent, Stand sichern“, §6); [entscheidungen.md](../../research/2026-10-30-phase-3d/entscheidungen.md) (Tabelle „Phasenschnitt“ Zeile 4.5; Tabelle „Löschen, Sprites, DS-Varianten“ Zeile „Veröffentlichen-Assistent, „Stand sichern““); [gutachten-c-loeschen-sprites-schnitt.md](../../research/2026-10-30-phase-3d/gutachten-c-loeschen-sprites-schnitt.md) §5 (Zeilen Veröffentlichen-Assistent und „Stand sichern“), §6.3 (Zeile `technik/PublishSurface*`, `patch.rs`, `git.rs`, Titelleiste), §6.4 Gate 4.5; [ROADMAP.md](../../ROADMAP.md) Abschnitt „Phase 4“ (4.1a, 4.4, „Nicht in Phase 4“) und Tabelle 3d (Zeile 4.5); [design-system/README.md](../../../design-system/README.md) „Nachtrag 4.1a“ (Liste der Abweichungen vom Zielbild „Veröffentlichen: Patch erstellen“) und [Audit-Design-System.md](../../../design-system/Audit-Design-System.md) Screen 26 und Zeile „Titelleiste“; die Gates [phase-4-1a-gate.md](../../phase-4-1a-gate.md) und [phase-4-4-gate.md](../../phase-4-4-gate.md); die Formate [xdelta_patch.md](../../formats/xdelta_patch.md) und [project_git.md](../../formats/project_git.md); [entscheidungen.md (Phase 4)](../../research/2026-10-27-phase-4/entscheidungen.md) Abschnitte „Git (Gutachten C, 4.4)“ und „Nicht in Phase 4“; der Plan [4.4](2026-10-28-phase-4-4-git-minimal.md) (Entscheidungen „Unterbefehle“, „Kein Name in Git“, „Commit scheitert“, Global Constraints zu Git) und der Plan [4.1a](2026-10-27-phase-4-1a-basis-rom-patch.md) (Entscheidungen „Name“, „Schreiben“, „Begleitdatei“).

Stand 31.10.2026: `main` bei `3c194c7` (Phase 4 abgeschlossen, 3d spezifiziert). Arbeit im Worktree `/Users/POLTEP/Projekte/lab/editor/gen4-studio/.claude/worktrees/phase-4-5`, Branch `phase-4-5-veroeffentlichen` (existiert, Basis main `3c194c7`), **parallel zu 3d.1, 3d.2 und 3d.3** in eigenen Worktrees. Kernel `crates/g4-kernel/src`, Tauri `apps/studio/src-tauri/src`, Frontend `apps/studio/src`.

## Vorhandenes, auf dem 4.5 aufsetzt

Vor Task 1 mit `grep` prüfen; heißt etwas anders, folgt der Plan dem Namen auf main, die Regeln bleiben.

| Schnittstelle | Ort | gebraucht in |
|---|---|---|
| `patch::{PatchMeta, PatchTargets, PatchReport, patch_name, default_targets, meta, notes_path, app_header, notes, create_patch, create_patch_with (pub(crate)), build_and_create, SELF_CHECK_FAILED, NO_MARK}`; privat `clean(name, fallback)`, `name_for`, `DSPRE_SUFFIX`, `write_tmp`, `sha1_hex` | `crates/g4-kernel/src/patch.rs` | Task 1, 2 |
| `start::{BaseMark, read_base_mark, rfc3339_utc}`, privat `now_rfc3339()` (Task 1 macht es `pub`) | `crates/g4-kernel/src/start.rs` | Task 1, 2 |
| `base::{BaseRom (sha1(), bytes()), base_label, other_rom}` | `crates/g4-kernel/src/base.rs` | Task 2 |
| `build::{build, BuildReport}` | `crates/g4-kernel/src/build.rs` | Task 2 |
| `fsutil::write_atomic` (`pub(crate)`) | `crates/g4-kernel/src/fsutil.rs` | Task 1 |
| `git::cli::{GitCli, find_git, git_init, InitPhase, InitReport, STUDIO_NAME, STUDIO_EMAIL, FIRST_COMMIT, NOT_INSTALLED, IN_REPO, TIMED_OUT}`; privat `Sub` (`Version`, `Init`, `Add`, `Ident`, `Commit { studio_identity }`, `Copy`), `run`, `Ran`, `RunError`, `succeeded`, `failed`, `TIMEOUT`, Test `the_studio_runs_only_its_own_subcommands` mit `Sub::ALL` | `crates/g4-kernel/src/git/cli.rs` | Task 3 |
| `git::repo::{GitRepo::{discover, kind, head, status, index_paths}, RepoKind, GitStatus, HeadInfo, status}` | `crates/g4-kernel/src/git/repo.rs` | Task 3 |
| `git::templates::{write_templates, TemplateReport, Layout, layout_of}`, Metadaten `crates/g4-kernel/metadata/git/{dspre,text}.gitignore` (Zeilen `/.g4/*`, `!/.g4/base.json`, `!/.g4/preview.png`) | `crates/g4-kernel/src/git/templates.rs` | Task 1, 3 |
| `test_support::{synthetic_project, synthetic_rom, vanilla_copy, vanilla_base_rom, GitFixture, TextFixture}` | `crates/g4-kernel/src/test_support.rs` | Task 1 bis 4 |
| `commands::base::{BaseStatusDto (found { patch_name }), PatchDto { report, build }, create_patch_impl, create_patch_checked(state, turn, check), patch_targets}`, `commands::git::{GitStatusDto, git_status_impl, check_git_init, git_init_with, InitBusy, INIT_UNSAVED, SETTING_UP_GIT}`, `commands::{lock, session, open_project_arc}`, `CmdError::refused(code, text)` | `apps/studio/src-tauri/src/commands/` | Task 4 |
| `dispatch.rs`: `COMMANDS`, Makro `run!`, Beispielargumente (`("create_patch", json!({}))`), Arm `create_patch` mit `policy.check_output`; `policy::{UserPolicy, Policy::{check_output, read_path, write_path}, is_in_build}` | `apps/studio/src-tauri/src/commands/` | Task 4 |
| `mcp/access.rs`: `Kind::{Read, Write, Depends, Irreversible(&str), Unavailable(&str)}`, `COMMAND_KINDS`, `classify(command, args, policy)` (Arme `build_rom`, `export_map_part` für `Depends`, `OUTSIDE_BUILD`), `admit`; `mcp/tools.rs`: `NOT_IN_BATCH`, `LONG_TOOLS`, `Call::question` (Arme `assign_base`, `create_patch`, `git_init`), `Call::confirm`; `mcp/catalog.rs`: `ToolDef`, `command(..)`, `renamed(..)`, `object(..)`, `answers_object`, Tests `the_curated_set_is_the_decided_one`, `irreversible_tools_name_the_headless_switch`, `tool_count_is_about_thirty` | `apps/studio/src-tauri/src/mcp/` | Task 4 |
| `api.base.ts` (`BaseStatus`, `PatchDto`, `createPatch`, `formatBytes`), `api.git.ts` (`GitStatus`, `InitReport`, `gitInit` mit `Channel`), `technik/useBase.ts` (`BaseApi { status, busy, createPatch, lastPatch, refresh }`, `UseBaseOptions.createPatch`), `technik/PublishSurface.tsx` (`publishSubtitle`, Karten „Basis-ROM“ und „Patch“, `AssignDialog`, `PatchResult`), `technik/baseTesting.ts` (`found`, `missing`, `patchDto`), `git/gitTesting.ts` (`gitStatusOf`, `noRepo`, `initReportOf`), `git/GitInitDialog.tsx` (Muster: `CheckList`, `Dialog` mit `data`, `finalFocus`, `dismissible`, `exclusive`), `git/gitWords.ts` (`gitFoundText`, `changesWord`) | `apps/studio/src/` | Task 5 bis 7 |
| Assistenten: `workbench/wizards/WizardFrame.tsx` (`WizardFrameProps { title, icon, steps: WizardStep[], step, onStep, busy, error, summary, irreversible, result, submitLabel, onSubmit, onCancel, onShow, showLabel, onPlay }`, `WizardStep { id, title, content, canNext }`), `WizardBar`, `WizardResult.tsx` (`DoneResult { title, lines, next }`), `WizardStep.module.css`, `project/ClearWorldWizard.tsx` (Muster eines Assistenten ohne Welt-Daten), `app/WizardHost.tsx`, `app/useWizards.ts` (`open`, `close(written)`), `workbench/WorkbenchContext.tsx` `WizardRequest` | `apps/studio/src/` | Task 5 |
| Rahmen: `ui/frame/TitleBar.tsx` (`TitleBarProps`, `IconButton` Speichern, Bauen, Thema, Hilfe), `ui/primitives/IconButton.tsx` (`label`, `busy`, `disabled`; Tooltip = `label`), `ui/primitives/icons.ts` (`publish: UploadSimple`, `git: GitBranch`), `app/paletteEntries.ts` (`PaletteCommand`, `CommandState.gitSetup?`, `CommandSpec` mit `locked`), `App.tsx` (`useBase`, `useGitStatus`, `GitInitDialog`, `TitleBar`, `WizardHost`, `paletteEntries`), `app/useProjectSession.ts` (`buildPatch` über `guardedBuild`, `exclusive`) | `apps/studio/src/` | Task 5 bis 7 |
| `technik/ChangesSurface.tsx` (`ChangesSurfaceProps`, `headShown(git, onGitSetup)`, „Neu vergleichen“ `[data-compare-again]`, `[data-git-offer]`), `technik/TechnikArea.tsx` (`ENTRIES`, `publishSubtitle(props.base)`) | `apps/studio/src/technik/` | Task 6, 7 |
| `platform/dialogs.ts` (`openPath({ directory, title, filters })`, Browser-Modus über `WebDialogHost`), `ui/frame/toasts.ts` (`toast.info`, `toast.error`) | `apps/studio/src/platform/` | Task 5, 7 |
| `g4k`: `patch create` (`patch_create`, `required_mark`, `required_base`, `print_problems`, `json_line`), `GitCommand::{Templates, Init, Status}`, `fn git(..)`, `print_templates` | `crates/g4-kernel/src/bin/g4k.rs` | Task 2, 3 |

## Entscheidungen (wo Spezifikation und Gutachten offen lassen)

| Frage | Entscheidung | Grund |
|---|---|---|
| Ort des Codes | Kernel: `patch.rs` bleibt, dazu `patch/release.rs` und `patch/publish.rs` (Rust erlaubt `patch.rs` neben dem Ordner `patch/`), `git/cli.rs` bekommt `commit_all`; App: neue Datei `commands/publish.rs` (`releases`, `publish_patch`, `reveal_path`), `commands/git.rs` angehängt (`git_commit_all`); Frontend: Ordner `publish/` (`PublishWizard.tsx`, `publishWords.ts`), `git/CommitDialog.tsx`, `api.publish.ts` | Gutachten C §6.3: 4.5 berührt `technik/PublishSurface*`, `patch.rs`, `git.rs` und die Titelleiste; neue Dateien halten die Merges mit 3d.1 bis 3d.3 klein |
| `.g4/releases.json` | `{ "version": 1, "releases": [ { "version", "date", "file", "notes", "baseSha1", "size" } ] }`, Schlüssel camelCase wie `g4-project.json`, `deny_unknown_fields` auf beiden Ebenen, Einträge in der Reihenfolge der Veröffentlichung (neueste zuletzt), atomar geschrieben, `preserve_order`, 2 Leerzeichen; `file` relativ zur Projektwurzel mit `/`, wenn die Datei im Projekt liegt, sonst absolut; `date` RFC 3339 UTC (`rfc3339_utc`); `size` in Byte dazu, damit die Liste Größen zeigt, ohne eine vielleicht verschobene Datei zu lesen | Vorgabe „eigen, Standard, versioniert, kleine Liste `{version, date, file, notes, baseSha1}`“; versioniert heißt: `!/.g4/releases.json` kommt in beide `.gitignore`-Vorlagen (Block wird beim nächsten „Stand sichern“ nachgezogen, siehe „Stand sichern“) |
| Kaputte oder neuere `releases.json` | `read_releases` antwortet `Refused(RELEASES_BROKEN)` mit Pfad und Grund; die Liste zeigt die Fehlermeldung, der Assistent öffnet nicht (Taste gesperrt mit dem Satz), nichts wird überschrieben | wie Basismarke „Nicht lesbar“ (4.1a): eine Datei einer neueren Fassung wird nie still ersetzt |
| Versionsvorschlag | `next_version`: letzte Version `^(\d+)\.(\d+)\.(\d+)` → dritte Zahl plus eins, ein Anhang danach (`-beta`) fällt weg; ohne Veröffentlichung `0.1.0`; eine letzte Version ohne dieses Muster wird unverändert vorgeschlagen (die Prüfung lehnt sie dann als vergeben ab und der Nutzer ändert sie) | „Vorschlag aus der letzten Veröffentlichung +1 oder `0.1.0`, semver-artig, frei editierbar“ |
| Gültige Version | `check_version`: nicht leer, höchstens 32 Zeichen, kein Leerraum, keines der Zeichen `/ \ : * ? " < > \|`, kein Steuerzeichen, noch nicht in der Liste (`VERSION_TAKEN`) | sie steht im Dateinamen und im App-Header; semver wird nicht erzwungen |
| Dateiname | `release_file_name(project, version)` = `<Projekt>-v<Version>.xdelta`, `<Projekt>` = `project_stem` (der bereinigte Ordnername ohne `_DSPRE_contents`, derselbe wie in `patch_name` ohne `-<Spielcode>`); der Spielcode steht in Begleitdatei und App-Header; der Nutzer darf den Namen ändern (Endung `.xdelta` wird ergänzt, dieselben Zeichenregeln wie die Version, `BAD_FILE`) | Vorgabe `<Projektname>-v<Version>.xdelta` |
| Speicherort | Standard `<Projekt>/build/`; „Ordner wählen…“ über `openPath({ directory: true, title: "Speicherort wählen" })`; ein relativer `dir` (MCP, `g4k`) liegt relativ zur Projektwurzel; der Ordner wird angelegt, wenn er fehlt; Begleitdatei `<Datei>.txt` und `README.txt` liegen im selben Ordner | Vorgabe; `README.txt` ohne Präfix, weil es neben genau einem Patch liegt |
| Gebaute ROM | der Build geht weiter nach `build/<patch_name>.nds` (`default_targets`), unabhängig vom Speicherort; die ROM gehört nicht zu dem, was weitergegeben wird | der Speicherort nimmt nur Patch und Texte auf („Kein ROM-Inhalt, nur der Patch“); `*.nds` und `/build/` ignoriert die Git-Vorlage |
| Datei schon da | `check_targets(targets, overwrite)`: existiert Patch, Begleitdatei oder (bei `readme`) `README.txt` und `overwrite` ist falsch → `Refused(EXISTS)` mit der Datei davor, **vor dem Build**; der Assistent zeigt den Satz und bietet „Ersetzen“ (sekundär), das denselben Lauf mit `overwrite: true` startet | nichts wird still ersetzt; der Build läuft nicht umsonst |
| Patch ohne Änderung | ist die SHA-1 der gebauten ROM die der Basis-ROM → `Refused(NO_CHANGES)`, nichts geschrieben, kein Eintrag | eine Veröffentlichung ohne Unterschied ist ein Versehen; 4.1a-`create_patch` (MCP) schreibt ihn weiter mit Warnung |
| Begleitdatei | `notes_with(meta, base_sha1, target_sha1, release: Option<&ReleaseInfo>)`: nach `Studio-Version: …` die Zeilen `Version: {version}` und `Datum: {date}`, nach dem Absatz „Anwenden: …“ der Absatz `Änderungen:` mit den Notizen (jede Zeile wie eingegeben, ohne Einrückung), nur wenn Notizen da sind; ohne `release` byte-gleich zum Text von 4.1a (`notes` ruft `notes_with(.., None)`) | „Rückwärtslesen ohne diese Felder bleibt gültig“: die ersten fünf Zeilen und der Anwenden-Absatz stehen unverändert; 4.1a liest die Begleitdatei nur als Text (Gate 4.1a Punkt 8 prüft den Wortlaut) |
| `README.txt` | Kästchen „Änderungsnotizen auch als README.txt daneben“, aus als Standard; Inhalt `readme_text`: `{Projekt} v{version} ({Datum})`, Leerzeile, die Notizen (fehlen sie: „Keine Änderungsnotizen.“), Leerzeile, `Basis-ROM SHA-1: …`, `Anwenden: …` wie in der Begleitdatei | Zielbild „Textexport“; wer weitergibt, hat einen Text ohne Prüfsummen-Kopf |
| Ablauf im Kernel | `publish_built(project, base, built_rom, opts, progress)`: `Encode` und `Verify` über `create_patch_with_notes` (Selbstprüfung aus 4.1a, nichts geschrieben, wenn sie scheitert), `Write` (Patch, Begleitdatei, dann `README.txt` über `write_tmp` + `rename`), `Record` (`append_release`); `build_and_publish` baut davor (`Build`); die App baut über ihren Build-Weg (`guardedBuild`) und ruft `publish_built` | eine Stelle für App, MCP, `g4k`; die Selbstprüfung bleibt die Garantie |
| Schritt „Prüfen“ | zeigt das Ergebnis der Selbstprüfung (Satz von 4.1a, `verify_ms`, beide SHA-1 kurz) und die Vorschau „Was du weitergibst“ (Dateiliste mit Größen aus `files`, Hinweis „Kein ROM-Inhalt, nur der Patch: wer ihn anwendet, braucht dieselbe Basis-ROM.“); der Lauf selbst passiert beim „Weiter“ von „Patch“ | die Selbstprüfung läuft vor dem Schreiben (4.1a); ein Zwischenschritt „prüfen, dann erst schreiben“ bräuchte eine zweite Kodierung oder Bytes im Speicher über zwei Schritte |
| Schritt „Fertig“ | `WizardFrame.result` mit `DoneResult` („Veröffentlicht: v{version}“, Zeilen Pfad des Patches und „{n} Dateien, {Größe}“; „Als Nächstes“: Weitergeben mit Rom Patcher JS oder xdelta3, „Stand sichern“); Tasten der Rahmenleiste: „Im Finder zeigen“ (`onShow` mit `showLabel`), „Schließen“; im Ergebnis die sekundäre Taste „Stand sichern…“ nur mit eigenem Repository (`repo.kind === "own"`), sie schließt den Assistenten und öffnet den Dialog mit Betreff `Veröffentlicht v{version}` und den Notizen | Vorgabe; `WizardFrame` kennt nur `onShow`, `onPlay`, Schließen |
| „Im Finder zeigen“ | neuer Befehl `reveal_path { path }` (nur in der App, `Kind::Unavailable("Nur in der App")`): macOS `open -R <path>`, Windows `explorer /select,<path>`, Linux `xdg-open <Ordner>`; der Pfad muss existieren und eine Datei aus `releases.json` oder deren Ordner sein; im Browser-Modus Meldung „Nur in der App.“ | es gibt kein Opener-Plugin im Projekt; `tauri-plugin-dialog` ist das einzige |
| Fläche Technik › Veröffentlichen | Kopf (Titel „Veröffentlichen“, Satz), Karte „Basis-ROM“ unverändert, dann statt der Karte „Patch“ die Karte „Veröffentlichen“ mit primär „Veröffentlichen…“ (gesperrt ohne gefundene Basis-ROM mit der Notiz des Grundes, Muster 4.1a) und darunter die Liste „Bisherige Veröffentlichungen“ (`Table`: Version, Datum, Datei, Notizen (erste Zeile), je Zeile „Im Finder zeigen“); leer „Noch keine Veröffentlichung.“; `useBase` verliert `createPatch` und `lastPatch`, der MCP-Befehl `create_patch` bleibt | Vorgabe „ersetzt die zwei Karten durch Kopf, Taste und Liste; die Karte Basis-ROM bleibt“ |
| Untertitel in der Technik-Liste | „Veröffentlicht v0.1.0“ (letzte Veröffentlichung) statt „Patch erstellt“, sonst „Bereit zum Veröffentlichen“ statt „Bereit für einen Patch“; die übrigen wie 4.1a | die Liste kommt mit dem Zustand aus einem Befehl |
| Titelleiste | `IconButton` „Veröffentlichen“ (Symbol `publish`) zwischen „Bauen“ und „Thema wechseln“; ohne Projekt `disabled`; mit Projekt ohne gefundene Basis-ROM `aria-disabled` (wie `busy`) mit dem Grund als zweiter Zeile des Tooltips (neue Eigenschaft `hint` von `IconButton`, auch `aria-description`); während Bauen, Speichern oder offenem Assistenten `busy` wie „Bauen“ | „gesperrt ohne Basis-ROM mit Tooltip-Grund“: eine `disabled`-Taste zeigt keinen Tooltip, eine `aria-disabled` behält Fokus und Hover |
| Assistent im Scharnier | `WizardRequest` bekommt `{ kind: "publish" }` (eine Zeile angehängt), `WizardHost` rendert `PublishWizard`; Öffnen zeigt die Welt wie bei jedem Assistenten (`useWizards.onOpen`); während er offen ist, warten Speichern, Bauen und Palette wie bei „Welt leeren“ | „nach dem Muster der vorhandenen Assistenten“: Schrittleiste im Scharnier, Schritt in der Hand |
| Fortschritt im Assistenten | Schritt „Patch“ zeigt während des Laufs `role="status"` mit „Speichern und bauen …“, „Patch kodieren …“, „Selbstprüfung …“, „Dateien schreiben …“, „Veröffentlichung eintragen …“ aus `PublishPhase`; im Browser-Modus ohne `Channel` nur die erste Zeile | Muster 4.4 (`InitPhase`) |
| „Stand sichern“: Umfang | `commit_all`: `Templates` (`write_templates`, damit der Block aktuell ist und `releases.json` versioniert wird), dann Status; ohne Repository `Refused(NO_REPO_TO_COMMIT)`, in einem umschließenden Repository `Refused(ENCLOSING_REPO)`, ohne Änderung `Refused(NOTHING_TO_COMMIT)`, ohne `git` `Refused(GIT_NEEDED)`; sonst `Add` (`git add -A`), `Commit` (`git commit -q -m <Betreff> [-m <Body>]`, Identität wie 4.4); ein gescheiterter Unterbefehl ist `committed: false` mit `reason` (Git-Text, Muster 4.4) | nur das eigene Repository des Projekts; `git add -A` nimmt alles, was die `.gitignore` zulässt, kein Staging |
| Betreff | App und `g4k` bilden ihn: „Stand {YYYY-MM-DD} {HH:MM}“ in Ortszeit (Frontend `new Date()`, `g4k` über die Systemzeit); der Kernel nimmt `message` und fällt ohne auf `default_subject()` = „Stand {YYYY-MM-DD} {HH:MM} UTC“ zurück (nur `rfc3339_utc` im Kernel, keine Zeitzonen-Abhängigkeit); aus dem Assistenten „Veröffentlicht v{version}“; `notes` wird der Body (zweites `-m`) | kein neues Crate für Zeitzonen; der Nutzer sieht seine Uhrzeit |
| `Sub::Commit` | wird `Sub::Commit { studio_identity: bool, subject: String, body: Option<String> }`, `Sub` verliert `Copy` (`Clone`), `args(&self)`; `git_init` ruft es mit `FIRST_COMMIT` ohne Body; der Test `the_studio_runs_only_its_own_subcommands` deckt beide Formen über `Sub::ALL` ab, die Erlaubnisliste bleibt `--version`, `init`, `add`, `var`, `commit` | ein Unterbefehl für beide Commits; kein neuer Weg an `Sub` vorbei |
| Dialog „Stand sichern?“ | `CommitDialog` nach dem Muster `GitInitDialog`: Beschreibung, Prüfliste (gespeichert / „Erst speichern“ sperrt; „Git gefunden: {version}“ / „Git ist nicht installiert“ sperrt; „{n} geänderte Dateien seit dem letzten Commit“ / „Keine Änderung seit dem letzten Commit“ sperrt), Feld „Betreff“ (vorbelegt), Feld „Notiz“ (mehrzeilig, optional, vorbelegt aus dem Assistenten), Tasten „Abbrechen“ / „Sichern“ (primär); im Lauf keine Tasten; danach Meldung „Stand gesichert: {short} „{Betreff}“ ({n} Dateien).“ und `git.refresh()`; ein gescheiterter Commit zeigt `reason` und nur „Schließen“ | Vorgabe „Dialog mit Prüfliste (gespeichert, Git gefunden, N geänderte Dateien) und Notizfeld“ |
| Orte von „Stand sichern“ | Technik › Änderungen: sekundäre Taste „Stand sichern…“ neben „Neu vergleichen“ bei „Gegen letzten Commit“ mit eigenem Repository (`[data-commit-all]`); Schritt Fertig des Assistenten; Palette „Stand sichern…“ (Symbol `git`, nur mit eigenem Repository); ohne `git` ist die Taste `aria-disabled` mit Tooltip „Git ist nicht installiert.“, ohne Repository fehlt sie (dort steht „Git einrichten…“) | Vorgabe; „ohne Git ist die Taste gesperrt mit Grund“ |
| MCP `publish_patch` | Argumente `{ version, notes?, dir?, file?, readme?, overwrite? }`; `Kind::Depends`: `classify` → `Irreversible("Patch ersetzen")`, wenn `overwrite` wahr ist; sonst `Irreversible(OUTSIDE_BUILD)`, wenn `dir` außerhalb von `build/` liegt (wie `build_rom`); sonst `Write`; `Call::question` nennt beim Ersetzen die Datei („Patch ersetzen: <Pfad>“) und lehnt davor unasked ab mit `unsaved`, `base_missing`, `exists` (ohne `overwrite`), `bad_version`, `version_taken`; in `NOT_IN_BATCH` und `LONG_TOOLS` | „Rückfrage nur, wenn eine Datei überschrieben würde“: ohne `overwrite` wird eine vorhandene Datei abgelehnt, nicht gefragt; `Depends` ist der vorhandene Weg für eine Klasse, die von den Argumenten abhängt |
| MCP `git_commit_all` | `{ message?, notes? }`, `Kind::Irreversible("Stand sichern")`, in `NOT_IN_BATCH`; `question` lehnt vorher unasked ab mit `unsaved`, `no_repo`, `enclosing_repo`, `nothing_to_commit`, `no_git`; kopflos nur mit `--allow-irreversible` | wie `git_init`: ein Commit ist in der App nicht rückgängig |
| Befehle ohne Werkzeug | `releases {}` (`Read`, über `g4_call`), `reveal_path { path }` (`Unavailable("Nur in der App")`) | ein Agent liest die Liste aus `publish_patch`-Antworten; den Finder öffnet er nicht |
| Kopflos | `g4k publish <root> --version <v> [--notes <text> \| --notes-file <pfad>] [--dir <ordner>] [--file <name>] [--readme] [--overwrite] [--base-rom <nds>] [--json]`; `g4k git commit-all <root> [--message <betreff>] [--notes <text>]` | Gate (2) ohne App |
| Design-System | README „Nachtrag 4.5“ (Assistent, Liste, Titelleisten-Taste, Dialog), `openpencil/beispiele/veroeffentlichen-assistent.body.js`, `NACHTRAG-4-5.md`; die `.fig` bekommt die Frames nur, wenn OpenPencil antwortet, sonst „folgt“; Audit Screen 26 → „umgesetzt (4.5)“, Zeile „Titelleiste“ ohne „Es fehlt „Veröffentlichen““ | Vorgabe „`.fig` nicht erreichbar, Frames „folgt““ |

## Global Constraints

- Arbeitsort ist der Worktree `/Users/POLTEP/Projekte/lab/editor/gen4-studio/.claude/worktrees/phase-4-5`, Branch `phase-4-5-veroeffentlichen` (existiert, Basis main `3c194c7`): nie `cd` in die Hauptkopie, nie den Branch wechseln, nie nach main mergen, kein `git stash`; `tools/` ist dort ein Symlink auf den Haupt-Checkout (nie einchecken).
- Vor jedem cargo-Aufruf `export PATH="$HOME/.cargo/bin:/opt/homebrew/opt/rustup/bin:$PATH"`; `pnpm install` einmal in `apps/studio`. Commits enden mit `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>`.
- Originale unter `/Users/POLTEP/Projekte/lab/editor/Roms` nie öffnen oder ändern; Testkopien nur unter `Roms/_testkopien/4-5-<zweck>` (`cp -cR` für den Ordner `4-5-<zweck>_DSPRE_contents`, `cp -c` für `4-5-<zweck>.nds`, beide direkt nebeneinander). `G4_ROMS_DIR=/Users/POLTEP/Projekte/lab/editor/Roms` absolut; gated Tests lesen `G4_TEST_PROJECT`, schreiben nur auf `vanilla_copy()` und finden die Basis-ROM über `test_support::vanilla_base_rom()`, nur lesend. Die Test-Umgebung läuft nur auf `G4_VITE_PORT=1730 G4_BRIDGE_PORT=1731`.
- Keine ROM-Daten ins Repo: keine `.nds`, keine Patches aus echten ROMs, keine Begleitdateien echter Spiele, keine Repos aus Testläufen; Fixtures nur synthetisch.
- Git-Regeln aus dem Plan 4.4, wörtlich: Kein Remote und kein Push, nirgends: die App ruft nur die Unterbefehle aus `git::cli::Sub`; Tests legen Repos nur in Temp-Ordnern und Kopien an, klonen über `git init` + `git fetch <pfad>` + `git checkout FETCH_HEAD` (kein `git clone`, das ein `origin` einträgt) und prüfen, dass `.git/config` kein `[remote` enthält. Tests, die die `git`-CLI brauchen, laufen nur, wenn `find_git()` etwas findet (sonst `eprintln!("skipped: git not found")`), und mit eigener Umgebung über `GitCli::env` (`GIT_CONFIG_GLOBAL` auf eine leere Temp-Datei, `GIT_CONFIG_NOSYSTEM=1`, `HOME` auf den Temp-Ordner); nie `std::env::set_var`. Lesende Tests brauchen kein `git` (Repos über `test_support::GitFixture`, nur gix).
- Nicht in 4.5: Remote, Push, Pull, Branches, Merge, Staging, „Auf GitHub veröffentlichen“, Auto-Update, Signierung, Windows-Prüfung von xdelta UI; „Stand sichern“ committet immer alles (`add -A`), nie eine Auswahl.
- UI nur aus `apps/studio/src/ui/primitives` und mit Tokens `--g4-*`, keine Hex-Farben, keine Inline-Styles (`bash scripts/check-styles.sh`, `bash scripts/check-tokens.sh`); deutsche Oberfläche, englische Bezeichner; höchstens eine primäre Taste je Dialog und Karte.
- Anhängepunkte nur anhängen: `apps/studio/src-tauri/src/commands/dispatch.rs` (`COMMANDS`, Arme, Beispielargumente), `src-tauri/src/lib.rs` (`generate_handler!`), `mcp/access.rs` (`COMMAND_KINDS`, ein Arm in `classify`), `mcp/catalog.rs`, `mcp/tools.rs` (`NOT_IN_BATCH`, `LONG_TOOLS`, Arme in `question`), `mcp/symbolic.rs` (nichts Neues), `crates/g4-kernel/src/lib.rs`, `apps/studio/src/api.ts`.
- Parallel laufen 3d.1 (`data/refs.rs`, `textproject/*`, `eventFields.ts`, Fläche Flags), 3d.2 (`shellState.ts`, `Hinge.tsx`, `fields.ts`, Kernel `recipes`, `wizards.rs`, `SceneHost`; 3d.2 stellt alle `depth ===`-Vergleiche um, also in neuen Dateien von 4.5 keine direkten Tiefenvergleiche schreiben, `depth` höchstens durchreichen) und 3d.3 (`ncgr.rs`, `battle_sprites.rs`, `DSScreen`, Pokédex/Trainer): diese Dateien in 4.5 nicht anfassen.
- Vor jedem Commit: `cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, die Tests der berührten Crates (`cargo test -p g4-kernel --all-features`, `cargo test -p studio --all-features`); bei Frontend-Änderungen zusätzlich `cd apps/studio && pnpm test && pnpm build`, `bash scripts/check-styles.sh`, `bash scripts/check-tokens.sh`.

## Review Focus

- Die gewählte Version gibt es schon, oder sie enthält `/` oder Leerzeichen: nichts wird gebaut, der Assistent sperrt „Weiter“ mit dem Satz, MCP antwortet `version_taken` bzw. `bad_version` ohne Frage (Task 1 `check_version_refuses_taken_and_unsafe_versions`, Task 5 `a_taken_or_unsafe_version_locks_weiter_with_the_reason`).
- Am Speicherort liegt schon ein Patch dieses Namens: ohne `overwrite` wird vor dem Build abgelehnt und nichts berührt, der Assistent bietet „Ersetzen“, MCP fragt erst mit `overwrite: true` (Task 2 `an_existing_target_is_refused_before_the_build`, Task 4 `publish_asks_only_when_it_would_overwrite`, Task 5 `an_existing_file_offers_ersetzen`).
- Der gewählte Ordner lässt sich nicht beschreiben (Pfad einer Datei, kein Recht): die Fehlermeldung nennt den Pfad, `releases.json` bleibt unverändert, es liegt kein halber Patch (Task 2 `an_unwritable_dir_writes_nothing_and_records_nothing`).
- `.g4/releases.json` ist kaputt oder aus einer neueren Fassung: die Liste zeigt den Grund, „Veröffentlichen…“ ist gesperrt, nichts wird überschrieben (Task 1 `a_broken_releases_file_is_refused_and_kept`, Task 6 `a_broken_releases_file_locks_publishing_with_its_reason`).
- „Stand sichern“ in einem Repository ohne Commit (von Hand `git init`), in einem umschließenden Repository oder ohne Änderung: der erste Fall legt den ersten Commit an, die beiden anderen antworten mit ihrem Satz, nichts wird committet; ein gescheiterter Commit (Hook, Signatur) nennt Gits Text und lässt das Repository, wie es ist (Task 3 `commit_all_on_an_unborn_head_makes_the_first_commit`, `an_enclosing_repo_and_a_clean_tree_are_refused_with_their_sentence`, `a_failing_commit_reports_git_stderr_and_keeps_the_repo`; Task 7 `a_failed_commit_shows_the_reason_and_keeps_the_dialog`).

---

### Task 1: Liste der Veröffentlichungen, Versionen, Begleitdatei mit Version und Notizen

**Files:**
- Create: `crates/g4-kernel/src/patch/release.rs` (+ Tests im Modul)
- Modify: `crates/g4-kernel/src/patch.rs` (`pub mod release;`, Re-Exporte, `ReleaseInfo`, `notes_with`, `create_patch_with_notes`, `project_stem`; `create_patch_with` bekommt den Parameter `release`), `crates/g4-kernel/src/start.rs` (`now_rfc3339` wird `pub`), `crates/g4-kernel/metadata/git/dspre.gitignore` und `text.gitignore` (`!/.g4/releases.json` nach `!/.g4/preview.png`), `crates/g4-kernel/src/git/templates.rs` (Test), `docs/formats/xdelta_patch.md` (Abschnitte „Begleitdatei“, neu „Veröffentlichungen `.g4/releases.json`“), `docs/formats/project_git.md` (Abschnitt „`.g4/`“: `releases.json` versioniert)

**Interfaces:**
- Consumes: `patch::{PatchMeta, notes, create_patch_with, clean, name_for}`, `start::rfc3339_utc`, `fsutil::write_atomic`, `GitFixture`, `GitRepo::status`.
- Produces:

```rust
// patch/release.rs
pub const RELEASES_FILE: &str = ".g4/releases.json";
pub const RELEASES_VERSION: u32 = 1;
pub const FIRST_VERSION: &str = "0.1.0";
pub const VERSION_MAX: usize = 32;
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Release { pub version: String, pub date: String, pub file: String, pub notes: String, pub base_sha1: String, pub size: u64 }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Releases { pub version: u32, pub releases: Vec<Release> }
impl Releases { pub fn empty() -> Releases; pub fn last(&self) -> Option<&Release>; pub fn has_version(&self, version: &str) -> bool; }
pub const RELEASES_BROKEN: &str = "Die Liste der Veröffentlichungen ist nicht lesbar"; // + ": <Pfad>: <Grund>" bzw. " (Fassung 2 ist neuer als diese App)"
pub const VERSION_TAKEN: &str = "Diese Version gibt es schon.";
pub const BAD_VERSION: &str = "Die Version darf nicht leer sein, höchstens 32 Zeichen haben und weder Leerzeichen noch / \\ : * ? \" < > | enthalten.";
/// `.g4/releases.json`; a missing file is the empty list, a broken or newer one `Refused(RELEASES_BROKEN …)`.
pub fn read_releases(root: &Path) -> Result<Releases, KernelError>;
/// Appends `release` and writes the file atomically (2 spaces, `preserve_order`); returns the new list.
pub fn append_release(root: &Path, release: Release) -> Result<Releases, KernelError>;
/// The proposal for the next version (see „Entscheidungen“).
pub fn next_version(releases: &Releases) -> String;
/// `Err(BAD_VERSION)` or `Err(VERSION_TAKEN)`, else `Ok(())`.
pub fn check_version(version: &str, releases: &Releases) -> Result<(), String>;
/// `<Projekt>-v<Version>.xdelta` with `patch::project_stem`.
pub fn release_file_name(project: &Project, version: &str) -> String;
/// `file` as it goes into the list: relative to `root` with `/` when below it, else absolute.
pub fn release_file_entry(root: &Path, file: &Path) -> String;
// patch.rs
/// The folder name without `_DSPRE_contents`, cleaned as `patch_name` cleans it (`Projekt` if nothing is left).
pub fn project_stem(project: &Project) -> String;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReleaseInfo<'a> { pub version: &'a str, pub date: &'a str, pub notes: &'a str }
/// `notes` plus `Version:` and `Datum:` after `Studio-Version:` and the paragraph `Änderungen:` (only with notes); `None` gives `notes`.
pub fn notes_with(meta: &PatchMeta, base_sha1: &str, target_sha1: &str, release: Option<&ReleaseInfo<'_>>) -> String;
/// `create_patch` with the notes of `notes_with`.
pub fn create_patch_with_notes(base: &BaseRom, built_rom: &Path, out: &Path, meta: &PatchMeta, release: Option<&ReleaseInfo<'_>>) -> Result<PatchReport, KernelError>;
// start.rs
pub fn now_rfc3339() -> String;
```

- `read_releases`: `version > RELEASES_VERSION` → `Refused` mit „ (Fassung {n} ist neuer als diese App)“; JSON-Fehler → `Refused` mit `: <Pfad>: <serde-Fehler>`; `KernelError::Refused` trägt den ganzen Satz, die App gibt ihn als Code `releases_broken` weiter (Task 4).
- Begleitdatei mit `release` wörtlich (Zeilen mit `\n`; der Block „Änderungen“ nur mit nicht leeren Notizen, Notizen wie eingegeben, am Ende genau ein `\n`):

```text
Gen4 Studio: Patch für {base_label}
Spiel: {game_code}
Basis-ROM SHA-1: {base_sha1}
Ziel-ROM SHA-1: {target_sha1}
Studio-Version: {studio_version}
Version: {version}
Datum: {date}

Anwenden: mit xdelta3 („xdelta3 -d -s <Basis-ROM> {name}.xdelta {name}.nds“) oder mit Rom Patcher JS, auf genau die Basis-ROM mit dieser SHA-1.

Änderungen:
{notes}
```

- [ ] **Step 1: Scheiternde Tests** (`release.rs`, Temp-Ordner; `patch.rs` mit `test_meta()`):
  - `a_missing_file_is_the_empty_list` (`read_releases` → `Releases::empty()`, `next_version` → `"0.1.0"`).
  - `append_writes_camel_case_keys_in_order` (zwei Einträge; die Datei enthält `"baseSha1"` und `"releases"`, Einträge in Reihenfolge, `last().version == "0.1.1"`, 2 Leerzeichen Einrückung).
  - `next_version_bumps_the_patch_number_and_drops_a_suffix` (`"0.1.0"` → `"0.1.1"`, `"1.2.9"` → `"1.2.10"`, `"2.0.0-beta"` → `"2.0.1"`, `"final"` → `"final"`).
  - `check_version_refuses_taken_and_unsafe_versions` (Review Focus: `""`, `"1.0 b"`, `"1/0"`, 33 Zeichen → `Err(BAD_VERSION)`; vergebene → `Err(VERSION_TAKEN)`; `"0.1.1"` → `Ok`).
  - `a_broken_releases_file_is_refused_and_kept` (Review Focus: `{"version": 2, "releases": []}` → `Refused` enthält „Fassung 2 ist neuer“; `{` → `Refused` enthält den Pfad; `append_release` auf beiden → `Err`, Datei byte-gleich).
  - `release_file_name_uses_the_stem_and_the_version` (synthetisches Projekt `Mein Hack_DSPRE_contents`, `"0.1.0"` → `Mein Hack-v0.1.0.xdelta`; `project_stem` ohne Spielcode) und `release_file_entry_is_relative_below_the_root` (`<root>/build/x.xdelta` → `build/x.xdelta`; `/tmp/a/x.xdelta` → absolut).
  - `patch.rs`: `notes_with_adds_version_date_and_changes_after_the_old_lines` (die ersten fünf Zeilen und der Anwenden-Absatz byte-gleich zu `notes(..)`; `Version: 0.1.0`, `Datum: 2026-10-31T10:00:00Z`, Block „Änderungen:“ mit zwei Zeilen), `notes_without_a_release_are_unchanged` (`notes_with(.., None) == notes(..)`), `notes_without_text_have_no_changes_block`; `create_patch_with_notes_writes_the_extended_notes` (synthetische Basis wie `create_patch_writes_patch_and_notes_that_apply`; die Begleitdatei enthält `Version:`).
  - `templates.rs`: `the_templates_keep_releases_json_versioned` (synthetisch, `write_templates`, `GitFixture::init`, `.g4/releases.json` angelegt → unter den unverfolgten Pfaden; `.g4/cache/x` nicht).
- [ ] **Step 2: Lauf** `cargo test -p g4-kernel --all-features patch:: git::templates` → FAIL (Modul fehlt).
- [ ] **Step 3: Umsetzen** wie unter Interfaces; `create_patch_with` bekommt `release: Option<&ReleaseInfo<'_>>` und ruft `notes_with`; `create_patch` ruft mit `None`.
- [ ] **Step 4: Lauf** → PASS; die vorhandenen Tests von `patch.rs` (`notes_carry_both_sha1_and_the_instruction`, `create_patch_*`) unverändert grün.
- [ ] **Step 5: Doku** `xdelta_patch.md`: Abschnitt „Begleitdatei“ mit den neuen Zeilen und dem Satz „ohne Veröffentlichung fehlen `Version:`, `Datum:` und `Änderungen:`; die ersten fünf Zeilen und der Anwenden-Absatz sind die von 4.1a“; neuer Abschnitt „Veröffentlichungen `.g4/releases.json`“ mit Struktur, Regeln (Reihenfolge, `file` relativ oder absolut, `size`, Fassung), `next_version`, `check_version`, Dateiname; `project_git.md` „`.g4/`“: `releases.json` in der Erlaubnisliste.
- [ ] **Step 6: Commit** `feat(patch): releases list, version proposal and check, notes with version, date and changes` (mit Trailer).

### Task 2: Veröffentlichen im Kernel, `README.txt`, `g4k publish`, gated

**Files:**
- Create: `crates/g4-kernel/src/patch/publish.rs` (+ Tests im Modul)
- Modify: `crates/g4-kernel/src/patch.rs` (`pub mod publish;`, Re-Exporte), `crates/g4-kernel/src/bin/g4k.rs` (`publish`), `crates/g4-kernel/tests/cli.rs`, `crates/g4-kernel/tests/base.rs` (gated), `docs/formats/xdelta_patch.md` (Abschnitt „Veröffentlichen“), `docs/dev-setup.md` (Tabelle `G4_TEST_PROJECT`: der neue gated Test)

**Interfaces:**
- Consumes: Task 1; `patch::{default_targets, meta, create_patch_with_notes, write_tmp (privat → `pub(crate)`)}`, `build::build`, `start::{read_base_mark, now_rfc3339}`, `base::other_rom`.
- Produces:

```rust
// patch/publish.rs
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublishOptions {
    pub version: String,
    #[serde(default)] pub notes: String,
    /// The folder; relative to the project root; `None`: `build/`.
    #[serde(default)] pub dir: Option<PathBuf>,
    /// The patch's file name; `.xdelta` is added when missing; `None`: `release_file_name`.
    #[serde(default)] pub file: Option<String>,
    #[serde(default)] pub readme: bool,
    #[serde(default)] pub overwrite: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PublishTargets { pub dir: PathBuf, pub patch: PathBuf, pub notes: PathBuf, pub readme: Option<PathBuf>, pub rom: PathBuf }
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)] #[serde(rename_all = "snake_case")]
pub enum PublishPhase { Build, Encode, Verify, Write, Record }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)] pub struct PublishProgress { pub phase: PublishPhase }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)] #[serde(rename_all = "snake_case")]
pub enum PublishedKind { Patch, Notes, Readme }
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PublishedFile { pub kind: PublishedKind, pub path: PathBuf, pub size: u64 }
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PublishReport { pub release: Release, pub patch: PatchReport, pub files: Vec<PublishedFile>, pub releases: Releases, pub phase_ms: BTreeMap<PublishPhase, u64>, pub ms: u64 }
pub const EXISTS: &str = "liegt schon am Speicherort; wähle einen anderen Namen oder ersetze die Datei."; // mit „<Dateiname> “ davor
pub const NO_CHANGES: &str = "Die gebaute ROM gleicht der Basis-ROM; es gibt nichts zu veröffentlichen.";
pub const BAD_FILE: &str = "Der Dateiname darf nicht leer sein und weder / \\ : * ? \" < > | noch Steuerzeichen enthalten.";
pub const NO_ROM_CONTENT: &str = "Kein ROM-Inhalt, nur der Patch: wer ihn anwendet, braucht dieselbe Basis-ROM.";
/// The targets of `opts` (`Err(Refused(BAD_FILE))` for a bad name); creates nothing.
pub fn targets_for(project: &Project, opts: &PublishOptions) -> Result<PublishTargets, KernelError>;
/// `Refused("<name> " + EXISTS)` when a target exists and `overwrite` is false.
pub fn check_targets(targets: &PublishTargets, overwrite: bool) -> Result<(), KernelError>;
/// Version and name checked (`check_version`, `targets_for`, `check_targets`), nothing written; the app and MCP call it before asking or building.
pub fn check_publish(project: &Project, opts: &PublishOptions) -> Result<PublishTargets, KernelError>;
/// `README.txt` (see „Entscheidungen“).
pub fn readme_text(meta: &PatchMeta, release: &ReleaseInfo<'_>, base_sha1: &str) -> String;
/// Encode, verify, write (patch, notes, readme), record; `built_rom` is the ROM the app or `build_and_publish` built.
pub fn publish_built(project: &Project, base: &BaseRom, built_rom: &Path, opts: &PublishOptions, progress: &mut dyn FnMut(PublishProgress)) -> Result<PublishReport, KernelError>;
/// `check_publish`, then `build::build` to `default_targets().rom` (`Build`), then `publish_built`; without a mark or with another base `Refused` before anything is built.
pub fn build_and_publish(project: &Project, base: &BaseRom, opts: &PublishOptions, progress: &mut dyn FnMut(PublishProgress)) -> Result<PublishReport, KernelError>;
```

- `publish_built`: `check_publish` zuerst (auch hier, die Liste könnte sich geändert haben); `PatchMeta` = `meta(project, &mark)` mit `name` = Dateistamm des Patches (ohne `.xdelta`), damit App-Header und Begleitdatei den Namen der Veröffentlichung tragen; `date` = `now_rfc3339()`; SHA-1 der gebauten ROM gleich der Basis → `Refused(NO_CHANGES)` **vor** dem Kodieren; `create_patch_with_notes` (Phasen `Encode`, `Verify`, `Write` aus `encode_ms`/`verify_ms` abgeleitet, `Write` misst das Schreiben); `README.txt` über `write_tmp` + `rename` nach der Begleitdatei (scheitert es, bleiben Patch und Begleitdatei, der Fehler nennt die Datei); `Record` = `append_release` mit `size` des Patches; `files` in der Reihenfolge Patch, Begleitdatei, README.
- `g4k publish <root> --version <v> [--notes <text> | --notes-file <pfad>] [--dir <ordner>] [--file <name>] [--readme] [--overwrite] [--base-rom <nds>] [--json]`: druckt `published version=<v> patch=<pfad> size=<n> base=<sha1> target=<sha1>` und je weitere Datei `file <art> <pfad> <n>`, dann `releases=<zahl>`; `--json` den `PublishReport`; Ablehnungen wie `patch create` (Exit 1 mit dem Satz).

- [ ] **Step 1: Scheiternde Tests** (`publish.rs` mit synthetischer ROM und `TextFixture`-Basis wie in `patch.rs`; `tests/cli.rs`):
  - `targets_default_to_build_and_the_release_name` (`dir == <root>/build`, `patch` endet auf `-v0.1.0.xdelta`, `notes` auf `.xdelta.txt`, `readme == None`; mit `readme` → `Some(<dir>/README.txt)`; `file: "mein"` → `mein.xdelta`; `dir: "aus"` relativ → `<root>/aus`).
  - `a_bad_file_name_is_refused` (`"a/b"`, `""` → `Refused(BAD_FILE)`).
  - `an_existing_target_is_refused_before_the_build` (Review Focus: Patchdatei angelegt → `check_publish` `Err` enthält `EXISTS` und den Namen; `build_and_publish` mit einem Zähler im `progress` → keine Phase gemeldet, `releases.json` fehlt weiter; mit `overwrite: true` läuft es durch).
  - `publish_writes_patch_notes_readme_and_records_the_release` (`build_and_publish` mit `readme: true` und zwei Notizzeilen: drei Dateien mit Größen > 0, Begleitdatei enthält `Version: 0.1.0` und beide Notizzeilen, `README.txt` beginnt mit `<stem> v0.1.0 (`, `releases.json` hat einen Eintrag mit `file == "build/<stem>-v0.1.0.xdelta"`, `size == files[0].size`; `phase_ms` enthält alle fünf Phasen; das Dekodieren des Patches auf der Basis ergibt die gebaute ROM).
  - `a_second_release_proposes_the_next_version_and_refuses_the_taken_one` (`next_version` nach dem ersten → `"0.1.1"`; dieselbe Version → `Refused(VERSION_TAKEN)`).
  - `no_difference_to_the_base_is_refused_and_writes_nothing` (Basis = gebaute ROM → `Refused(NO_CHANGES)`, keine Datei im Ordner, kein Eintrag).
  - `an_unwritable_dir_writes_nothing_and_records_nothing` (Review Focus: `dir` zeigt auf eine vorhandene **Datei** → `Err` nennt den Pfad, `releases.json` fehlt weiter; ein zweiter Lauf mit gültigem Ordner gelingt).
  - `an_absolute_dir_outside_the_project_is_recorded_absolute` (`dir` = Temp-Ordner → `release.file` beginnt mit `/`).
  - `tests/cli.rs` `cli_publish_prints_the_files_and_the_count` (`g4k publish <root> --version 0.1.0 --notes "Erste Fassung" --readme --base-rom <synthetische ROM>` → Zeile beginnt mit `published version=0.1.0`, zwei Zeilen `file `, `releases=1`; zweiter Lauf ohne `--overwrite` → Exit 1 mit `EXISTS`).
- [ ] **Step 2: Gated Test** (`tests/base.rs`, `G4_TEST_PROJECT`, Kopie über `vanilla_copy()`, Basis `vanilla_base_rom()`): `publish_after_a_map_edit_writes_a_small_patch_that_applies` (Karte 3 ein Byte wie in `patch_after_map_and_after_map_script_text_is_small_and_applies`; `build_and_publish` mit `version "0.1.0"`, `readme: true` → Patch ≤ 1 KiB, Dekodieren mit dem eigenen Decoder = SHA-1 der gebauten ROM, mit `xdelta3 -d` wenn im `PATH`; `releases.json` ein Eintrag; Begleitdatei trägt `Version:`; zweiter Lauf `"0.1.1"` → zwei Einträge; druckt `publish <game_code> size=<n> build_ms=<n> encode_ms=<n> verify_ms=<n>`).
- [ ] **Step 3: Lauf** `cargo test -p g4-kernel --all-features patch::publish` → FAIL; **Step 4: Umsetzen**; **Step 5: Lauf** → PASS, `cargo test -p g4-kernel --all-features --test cli cli_publish_prints_the_files_and_the_count`; gated einmal auf Platin DE: `G4_TEST_PROJECT="<Roms>/_testkopien/4-5-pt_DSPRE_contents" cargo test --release -p g4-kernel --all-features --test base publish_after -- --nocapture`.
- [ ] **Step 6: Doku** `xdelta_patch.md` Abschnitt „Veröffentlichen“: Ablauf, Ziele, `EXISTS`, `NO_CHANGES`, `README.txt` wörtlich, `g4k publish`; `dev-setup.md`: der gated Test in der Tabelle.
- [ ] **Step 7: Commit** `feat(patch): publish a release at a chosen place with notes, README and the releases list, g4k publish` (mit Trailer).

### Task 3: „Stand sichern“ im Kernel: `commit_all`, `g4k git commit-all`, gated

**Files:**
- Modify: `crates/g4-kernel/src/git/cli.rs` (`Sub::Commit` mit Betreff und Body, `commit_all`, Konstanten; Tests), `crates/g4-kernel/src/git.rs` (Re-Exporte), `crates/g4-kernel/src/bin/g4k.rs` (`GitCommand::CommitAll`), `crates/g4-kernel/tests/cli.rs`, `crates/g4-kernel/tests/git.rs` (gated), `docs/formats/project_git.md` (Abschnitt „Stand sichern“)

**Interfaces:**
- Consumes: 4.4 `GitCli`, `find_git`, `run`, `succeeded`, `failed`, `TIMEOUT`, `GitRepo::{discover, kind, status, head}`, `write_templates`, `GitFixture`, `TextFixture`; Task 1 (`releases.json` in der Vorlage).
- Produces:

```rust
// git/cli.rs
pub const GIT_NEEDED: &str = "Git ist nicht installiert; „Stand sichern“ braucht die git-Kommandozeile (git-scm.com/downloads).";
pub const NO_REPO_TO_COMMIT: &str = "Dieses Projekt liegt in keinem Git-Repository; richte Git zuerst ein.";
pub const ENCLOSING_REPO: &str = "Das Projekt liegt in einem größeren Git-Repository; sichere den Stand dort mit deinem Git-Werkzeug.";
pub const NOTHING_TO_COMMIT: &str = "Keine Änderung seit dem letzten Commit; nichts zu sichern.";
pub const COMMIT_ALL_FAILED: &str = "Der Commit ist gescheitert";
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommitOptions { #[serde(default)] pub message: Option<String>, #[serde(default)] pub notes: Option<String> }
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)] #[serde(rename_all = "snake_case")]
pub enum CommitPhase { Templates, Add, Commit }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)] pub struct CommitProgress { pub phase: CommitPhase }
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CommitReport { pub committed: bool, pub reason: Option<String>, pub subject: String, pub head: Option<HeadInfo>,
                          pub files: u32, pub studio_identity: bool, pub phase_ms: BTreeMap<CommitPhase, u64>, pub ms: u64 }
/// „Stand {YYYY-MM-DD} {HH:MM} UTC“ from `rfc3339_utc` of now.
pub fn default_subject() -> String;
/// The subject of `opts`: `message` trimmed to its first line, else `default_subject()`.
pub fn subject_of(opts: &CommitOptions) -> String;
/// Refusals before anything runs (no repository, an enclosing one, a clean tree, no git); the templates are written first (they may add a change). Writes nothing else.
pub fn check_commit_all(project: &Project, cli: Option<&GitCli>) -> Result<GitStatus, KernelError>;
/// `check_commit_all`, then `git add -A` and `git commit -q -m <subject> [-m <notes>]` (identity as `git_init`); a failing subcommand is `committed: false` with Git's text.
pub fn commit_all(project: &Project, cli: Option<&GitCli>, opts: &CommitOptions, progress: &mut dyn FnMut(CommitProgress)) -> Result<CommitReport, KernelError>;
// privat
enum Sub { Version, Init, Add, Ident, Commit { studio_identity: bool, subject: String, body: Option<String> } }  // Clone, kein Copy
```

- `check_commit_all`: `Templates` (`write_templates`; ein kaputter Block ist `Err`), `GitRepo::discover`: `None` → `Refused(NO_REPO_TO_COMMIT)`, `Enclosing` → `Refused(ENCLOSING_REPO)`; `cli == None` → `Refused(GIT_NEEDED)`; `status.paths.is_empty()` → `Refused(NOTHING_TO_COMMIT)`; sonst `Ok(status)`. `commit_all`: `files = status.paths.len()`; `Add`; `Commit` (Identität über `Sub::Ident` wie `git_init`; `body` = `notes` getrimmt, leer → `None`); Zeitüberschreitung `TIMED_OUT`; danach `head` über gix, `committed = head.is_some()`. Ein unbeschriebener `HEAD` (Repo ohne Commit) ist kein Fehler: der Commit wird der erste.
- `g4k git commit-all <root> [--message <betreff>] [--notes <text>]`: ohne `--message` gilt `default_subject()` des Kernels (UTC; siehe „Offen“); druckt `committed <short> "<betreff>" files=<n>` oder `not committed: <reason>` (Exit 1 bei einem gescheiterten Unterbefehl), Ablehnungen als Satz mit Exit 1.

- [ ] **Step 1: Scheiternde Tests** (`cli.rs`; die mit `git` überspringen ohne Fund; Umgebung über `GitCli::env`):
  - `the_studio_runs_only_its_own_subcommands` erweitert: `Sub::ALL` enthält `Commit` mit und ohne Body; `args()` von `Commit { subject: "Stand …", body: Some("Notiz") }` ist `["commit", "-q", "-m", "Stand …", "-m", "Notiz"]` (nach den `-c`-Paaren); weiter keine verbotenen Wörter (`commit_args_carry_subject_and_body`).
  - `subject_defaults_to_stand_with_utc_time_and_takes_the_first_line` (`default_subject()` passt auf `^Stand \d{4}-\d{2}-\d{2} \d{2}:\d{2} UTC$`; `subject_of` mit `"Veröffentlicht v0.1.0\nzweite"` → erste Zeile).
  - `without_a_repo_commit_all_is_refused` (läuft immer, `cli = None`: ohne Repo `NO_REPO_TO_COMMIT`; eigenes Repo (`GitFixture`) mit Änderung → `GIT_NEEDED`).
  - `an_enclosing_repo_and_a_clean_tree_are_refused_with_their_sentence` (Review Focus, läuft immer: `GitFixture::commit_all` im Elternordner → `ENCLOSING_REPO`; eigenes Repo ohne Änderung nach `commit_all` der Fixture → `NOTHING_TO_COMMIT`; in beiden Fällen kein neuer Commit).
  - `commit_all_commits_everything_with_subject_and_body` (synthetisches DSPRE-Projekt, `git_init`, Datei geändert und `.g4/releases.json` angelegt → `committed`, `head.summary == "Veröffentlicht v0.1.0"`, voller Commit-Text enthält den Body (über gix `head` Commit-Message lesen), `files == 2`, Status danach leer, `.git/config` ohne `[remote`, `phase_ms` mit `Templates`, `Add`, `Commit`).
  - `commit_all_on_an_unborn_head_makes_the_first_commit` (Review Focus: `GitFixture::init` ohne Commit, eine Datei → `committed`, `head` da).
  - `the_templates_are_refreshed_before_the_commit` (Repo aus 4.4 ohne `!/.g4/releases.json` im Block simulieren: Block von Hand ohne die Zeile schreiben, `releases.json` anlegen → nach `commit_all` steht die Datei im Index (`index_paths` enthält `.g4/releases.json`) und der Block trägt die Zeile).
  - `without_an_identity_the_commit_is_by_gen4_studio` und `a_failing_commit_reports_git_stderr_and_keeps_the_repo` (Review Focus: `commit.gpgsign=true`, `gpg.program=false` wie in 4.4 → `!committed`, `reason` beginnt mit `COMMIT_ALL_FAILED`, `head` unverändert, Index enthält die Datei: `add -A` ist gelaufen).
  - `tests/cli.rs` `cli_git_commit_all_prints_the_commit` (`g4k git init`, Datei ändern, `g4k git commit-all <root> --message "Stand Test" --notes "Notiz"` → Zeile beginnt mit `committed ` und enthält `"Stand Test" files=1`; zweiter Lauf → Exit 1 mit `NOTHING_TO_COMMIT`).
- [ ] **Step 2: Gated Test** (`tests/git.rs`): `publish_then_commit_all_leaves_status_empty_and_no_remote`: `vanilla_copy()`, `git_init` → `committed`; Karte 3 ein Byte; `build_and_publish` (`version "0.1.0"`, `readme: true`) → `releases.json` da; `commit_all` mit `message: "Veröffentlicht v0.1.0"`, `notes: "Erste Fassung"` → `committed`, `files ≥ 2` (`releases.json`, Karten-Member; Patch und Texte unter `build/` zählen nicht), Status leer, `index_paths` enthält `.g4/releases.json` und keinen Pfad unter `build/`, `.git/config` ohne `[remote`; druckt `commit <game_code> files=<n> add_ms=<n> commit_ms=<n> status_ms=<n>`.
- [ ] **Step 3: Lauf** `cargo test -p g4-kernel --all-features git::cli` → FAIL; **Step 4: Umsetzen**; **Step 5: Lauf** → PASS, dazu `--test cli cli_git`; gated einmal auf HeartGold DE (`4-5-hgde`).
- [ ] **Step 6: Doku** `project_git.md` „Stand sichern“: Phasen, Unterbefehl mit Betreff und Body, Identität, Ablehnungen, nie ein Remote, `g4k git commit-all`; der Abschnitt „Git einrichten“ nennt `Sub::Commit` in der neuen Form.
- [ ] **Step 7: Commit** `feat(git): commit everything with a subject and notes (Stand sichern), g4k git commit-all, never a remote` (mit Trailer).

### Task 4: App-Befehle, Brücke und MCP

**Files:**
- Create: `apps/studio/src-tauri/src/commands/publish.rs` (+ Tests)
- Modify: `src/commands/mod.rs` (`pub mod publish; pub use publish::*;`), `src/commands/git.rs` (anhängen: `check_git_commit_all`, `git_commit_all_impl`, Tauri-Befehl), `src/lib.rs` (Handler anhängen), `src/commands/dispatch.rs` (anhängen: `releases`, `publish_patch`, `reveal_path`, `git_commit_all`, Beispielargumente), `src/mcp/access.rs` (`COMMAND_KINDS` anhängen, ein Arm in `classify`), `src/mcp/catalog.rs` (zwei Werkzeuge, Tests), `src/mcp/tools.rs` (`NOT_IN_BATCH`, `LONG_TOOLS`, zwei Arme in `question`, Tests), `docs/mcp.md`

**Interfaces:**
- Consumes: Task 1 bis 3; 4.1a `found_base`/`create_patch_checked` (Muster für Basis-Suche und Build über den Sitzungsweg), 4.4 `check_git_init`, `InitBusy` (Muster), `git_status_impl`; `policy::{check_output, is_in_build}`.
- Produces:

```rust
// commands/publish.rs
#[derive(Debug, Serialize)]
pub struct ReleasesDto { pub stem: String, pub next_version: String, pub dir: String, pub releases: Vec<Release> }  // dir: <root>/build absolut
pub fn releases_impl(state: &AppState) -> Result<ReleasesDto, CmdError>;                 // Refused(RELEASES_BROKEN) → Code `releases_broken`
/// The refusals before the question and before the build (`unsaved`, `base_missing`, `bad_version`, `version_taken`, `bad_file`, `exists`, `releases_broken`); writes nothing.
pub(crate) fn check_publish_patch(state: &AppState, opts: &PublishOptions) -> Result<PublishTargets, CmdError>;
/// Saves (the session's save), builds over the build path of `create_patch_checked`, then `patch::publish_built`; `check` is the policy's word on the targets (dispatch).
pub(crate) fn publish_patch_checked(state: &AppState, opts: PublishOptions, turn: bool, check: &dyn Fn(&PublishTargets) -> Result<(), CmdError>, progress: &mut dyn FnMut(PublishProgress)) -> Result<PublishReport, CmdError>;
pub fn publish_patch_impl(state: &AppState, opts: PublishOptions, progress: &mut dyn FnMut(PublishProgress)) -> Result<PublishReport, CmdError>;
pub const NOT_A_RELEASE_FILE: &str = "Diesen Pfad kennt die Liste der Veröffentlichungen nicht.";
/// `open -R` / `explorer /select,` / `xdg-open`; the path must exist and be a release file or its folder.
pub fn reveal_path_impl(state: &AppState, path: &str) -> Result<(), CmdError>;
// commands/git.rs (angehängt)
pub(crate) fn check_git_commit_all(state: &AppState) -> Result<Arc<Project>, CmdError>;   // `unsaved` (INIT_UNSAVED-Satz „Erst speichern: der Commit nimmt den gespeicherten Stand.“ als COMMIT_UNSAVED), `busy`; dann `git::check_commit_all` → Codes `no_repo`, `enclosing_repo`, `nothing_to_commit`, `no_git`
pub fn git_commit_all_impl(state: &AppState, opts: CommitOptions, progress: &mut dyn FnMut(CommitProgress)) -> Result<CommitReport, CmdError>;
pub(crate) fn git_commit_all_with(state: &AppState, cli: Option<&GitCli>, opts: CommitOptions, progress: &mut dyn FnMut(CommitProgress)) -> Result<CommitReport, CmdError>;  // Store busy mit SAVING_STATE = „Der Stand wird gerade gesichert.“ (Muster InitBusy)
```

- Fehlercodes von `publish_patch`: `unsaved` („Erst speichern: veröffentlicht wird der gespeicherte Stand.“), `base_missing` (Grund von `found_base`), `bad_version`, `version_taken`, `bad_file`, `exists`, `no_changes`, `releases_broken`; die Selbstprüfung wie 4.1a als `error` mit `SELF_CHECK_FAILED` ohne Vorsatz. Tauri: `publish_patch` und `git_commit_all` `async` mit `spawn_blocking` und `Channel`; `releases` und `reveal_path` `async` mit `spawn_blocking`. Der Build läuft wie in `create_patch_checked` (der Patch zählt als letzter Build, die ROM wird nicht die letzte ROM).
- `dispatch.rs`: `"releases"`, `"publish_patch"` (Argumente `{ version: String, notes: Option<String>, dir: Option<String>, file: Option<String>, readme: Option<bool>, overwrite: Option<bool> }`; `dir` über `policy.check_output` wie `build_rom`), `"reveal_path"` (`{ path: String }`), `"git_commit_all"` (`{ message: Option<String>, notes: Option<String> }`); Beispiele `("publish_patch", json!({ "version": "0.1.0", "notes": "Erste Fassung" }))`, `("git_commit_all", json!({ "message": "Stand" }))`, `("releases", json!({}))`, `("reveal_path", json!({ "path": "build/x.xdelta" }))`.
- `access.rs` (anhängen): `("releases", Kind::Read)`, `("publish_patch", Kind::Depends)`, `("reveal_path", APP_ONLY)`, `("git_commit_all", Kind::Irreversible("Stand sichern"))`; Arm in `classify`: `"publish_patch"` → `overwrite == true` → `Class::Irreversible("Patch ersetzen".into())`; sonst `dir` gesetzt und `!policy.is_in_build(dir)` → `Class::Irreversible(OUTSIDE_BUILD)`; sonst `Class::Write`.
- `tools.rs`: `NOT_IN_BATCH` + `publish_patch`, `git_commit_all`; `LONG_TOOLS` + `publish_patch`; `question`: `("publish_patch", Some(_))` → `args` als `PublishOptions` lesen (Schlüssel wie `dispatch` sie nimmt; ein Lesefehler ist `bad_args`), `check_publish_patch` (Ablehnungen unasked), Frage `format!("{why}: {}", targets.patch.display())`; `("git_commit_all", Some(_))` → `check_git_commit_all`, Frage `why`.
- Katalog (englisch):
  - `publish_patch` „Publish a patch“: „Saves, builds the ROM and publishes a release: an xdelta patch <project>-v<version>.xdelta against the base ROM at dir (default build/ in the project), its notes file with version, date and the change notes, optionally README.txt; records the release in .g4/releases.json. The patch is self-checked before it is written. Refuses a taken or unsafe version (version_taken, bad_version), an existing file without overwrite (exists), unsaved changes (unsaved), a missing base ROM (base_missing) and a build equal to the base (no_changes). With overwrite it replaces the files and asks the user first; headless (studio mcp) that needs --allow-irreversible, without it the call answers needs_confirmation. A dir outside build/ asks too.“; Schema `{ version: string (required), notes?: string, dir?: string, file?: string, readme?: boolean, overwrite?: boolean }`.
  - `git_commit_all` „Save the state in Git“: „Commits every change of the project (git add -A, git commit) with the git command line: subject message (default "Stand <date> <time> UTC") and notes as the body; never a remote, never a push. Refuses unsaved changes (unsaved), a project in no repository (no_repo) or in an enclosing one (enclosing_repo), a clean tree (nothing_to_commit) and a missing git (no_git). A commit cannot be undone in the app. Headless (studio mcp) it needs --allow-irreversible, without it the call answers needs_confirmation.“; Schema `{ message?: string, notes?: string }`.
- `BaseStatusDto` bleibt unverändert (`patch_name` weiter für `create_patch`); den Stamm der Veröffentlichung liefert `releases.stem`.

- [ ] **Step 1: Scheiternde Tests** (`commands/publish.rs`, `commands/git.rs`, synthetisches Projekt mit synthetischer ROM wie in `commands/base.rs`):
  - `releases_start_empty_with_the_first_version_and_the_build_dir`, `releases_report_a_broken_file_as_releases_broken`.
  - `publish_refuses_unsaved_taken_version_and_existing_file_before_the_build` (Codes `unsaved`, `version_taken`, `exists`; kein `build/` entstanden).
  - `publish_without_a_base_answers_base_missing`.
  - `publish_with_a_base_builds_writes_and_records` (synthetische Basis wie `create_patch_with_a_base_builds_first`, eine Änderung über den Store, `save`; `publish_patch_impl` → `files.len() == 2`, `releases.len() == 1`; danach `releases_impl().next_version == "0.1.1"`).
  - `reveal_path_refuses_a_path_outside_the_releases` (`NOT_A_RELEASE_FILE`; ein Pfad aus der Liste wird zugelassen, der Aufruf des Systembefehls ist in Tests ein No-op über ein `cfg(test)`-Tor).
  - `commit_all_refuses_unsaved_before_asking`, `commit_all_without_a_repo_is_no_repo`, `commit_all_with_git_commits_and_reads_the_status` (übersprungen ohne `git`: `GitFixture`-Repo, eine Änderung, `git_commit_all_with` → `committed`, `git_status_impl().changes == 0`).
  - `access.rs`: die vier Zeilen; `publish_patch_classifies_by_overwrite_and_dir` (`{}` → `Write`; `{"overwrite": true}` → `Irreversible("Patch ersetzen")`; `{"dir": "/tmp/x"}` → `Irreversible(OUTSIDE_BUILD)`).
  - `catalog.rs`: `the_curated_set_is_the_decided_one` (+ `publish_patch`, `git_commit_all` schreibend; `git_commit_all` unumkehrbar), `irreversible_tools_name_the_headless_switch` (+ `git_commit_all`, `publish_patch`), `tool_count_is_about_thirty` (Grenze heben, wenn nötig, im Commit begründen); `tools.rs`: `batch_refuses_publish_patch_and_git_commit_all`, `publish_asks_only_when_it_would_overwrite` (Review Focus: `{"version":"0.1.0"}` in der App mit Freigabe „Lesen und bearbeiten“ → keine Frage, Antwort oder Ablehnung `base_missing`; mit `"overwrite": true` und vorhandener Datei → Frage „Patch ersetzen: …“; ohne `overwrite` und vorhandener Datei → `exists` unasked), `git_commit_all_needs_confirmation_and_refuses_unsaved_unasked`; `dispatch.rs`: Beispielargumente der vier Befehle.
  - gated (übersprungen ohne `G4_TEST_PROJECT` oder `git`): `publish_and_commit_over_the_command_path_on_the_vanilla_copy` (`commands/publish.rs`: Kopie öffnen, `git_init_impl`, Trainer 1 über `data_edit`, `save`, `publish_patch_impl` `"0.1.0"` → zwei Dateien unter `build/`; `git_commit_all_impl` mit `message "Veröffentlicht v0.1.0"` → `committed`, `git_status_impl().changes == 0`; Zeiten gedruckt).
- [ ] **Step 2: Lauf** `cargo test -p studio --all-features` → FAIL; **Step 3: Umsetzen**; **Step 4: Lauf** → PASS, dazu `cargo test -p studio --test mcp_stdio`.
- [ ] **Step 5: Doku** `docs/mcp.md`: Tabelle „Werkzeuge“ mit `publish_patch` (schreibend; mit `overwrite` oder `dir` außerhalb `build/` unumkehrbar) und `git_commit_all` (unumkehrbar); Abschnitt „Git (seit 4.4)“ ergänzt um „Stand sichern (4.5)“ (kein Remote, `add -A`, Identität); „Befehle ohne eigenes Werkzeug“: `releases`, `reveal_path`; „Unumkehrbares in der App“: Fragen „Patch ersetzen: <Pfad>“ und „Stand sichern“; „Batch“: beide nicht im Batch; Abschnitt „Schreiben und Bestätigung“: Punkt „Veröffentlichen“.
- [ ] **Step 6: Commit** `feat(studio): publish a release, releases list, reveal a file and commit everything over the bridge and MCP` (mit Trailer).

### Task 5: Frontend: Assistent „Veröffentlichen“, Titelleiste, Palette

**Files:**
- Create: `apps/studio/src/api.publish.ts` (+ test), `apps/studio/src/publish/PublishWizard.tsx` (+ `.module.css`, test), `apps/studio/src/publish/publishWords.ts` (+ test), `apps/studio/src/publish/publishTesting.ts`
- Modify: `apps/studio/src/api.ts` (`export * from "./api.publish";`), `apps/studio/src/api.git.ts` (`CommitPhase`, `CommitReport`, `gitCommitAll`), `apps/studio/src/workbench/WorkbenchContext.tsx` (`WizardRequest` + `| { kind: "publish" }`), `apps/studio/src/app/WizardHost.tsx` (Fall `publish`, neue Props), `apps/studio/src/app/useProjectSession.ts` (`buildPatch` wird `publish(opts, onProgress)`), `apps/studio/src/ui/primitives/IconButton.tsx` (+ `hint`, test), `apps/studio/src/ui/frame/TitleBar.tsx` (+ `onPublish`, `publishLocked`; test), `apps/studio/src/app/paletteEntries.ts` (+ `publish`, `commitAll`; `CommandState.publishLocked?`, `commitAll?`; test), `apps/studio/src/App.tsx` (Verdrahtung)

**Interfaces:**
- Consumes: Task 4 (Befehle, DTOs); `WizardFrame`, `DoneResult`, `TextField` (`multiline`), `Checkbox`, `Notice`, `Button`, `Table`; `openPath`; `toast`.
- Produces (TS):

```ts
// api.publish.ts
export interface ReleaseEntry { version: string; date: string; file: string; notes: string; baseSha1: string; size: number }
export interface ReleasesDto { stem: string; next_version: string; dir: string; releases: ReleaseEntry[] }
export interface PublishOptionsDto { version: string; notes?: string; dir?: string; file?: string; readme?: boolean; overwrite?: boolean }
export type PublishPhase = "build" | "encode" | "verify" | "write" | "record";
export interface PublishedFile { kind: "patch" | "notes" | "readme"; path: string; size: number }
export interface PatchReportDto { patch: string; notes: string; rom: string; size: number; base_sha1: string; target_sha1: string; encode_ms: number; verify_ms: number }   // g4_kernel::patch::PatchReport
export interface PublishReportDto { release: ReleaseEntry; patch: PatchReportDto; files: PublishedFile[]; releases: { version: number; releases: ReleaseEntry[] }; phase_ms: Partial<Record<PublishPhase, number>>; ms: number }
export const releases: () => Promise<ReleasesDto>;
export const publishPatch: (opts: PublishOptionsDto, onProgress: (p: { phase: PublishPhase }) => void) => Promise<PublishReportDto>;
export const revealPath: (path: string) => Promise<void>;
// api.git.ts (angehängt)
export type CommitPhase = "templates" | "add" | "commit";
export interface CommitReport { committed: boolean; reason: string | null; subject: string; head: HeadInfo | null; files: number; studio_identity: boolean; phase_ms: Partial<Record<CommitPhase, number>>; ms: number }
export const gitCommitAll: (opts: { message?: string; notes?: string }, onProgress: (p: { phase: CommitPhase }) => void) => Promise<CommitReport>;
// publish/publishWords.ts
export function fileNameFor(stem: string, version: string): string;              // `${stem}-v${version}.xdelta`
export function versionProblem(version: string, taken: readonly string[]): string | null;  // BAD_VERSION- / VERSION_TAKEN-Satz wie der Kernel, null wenn gültig
export function fileProblem(file: string): string | null;                         // BAD_FILE-Satz
export function withXdelta(file: string): string;                                 // Endung ergänzen
export function filesLine(files: readonly PublishedFile[]): string;               // „3 Dateien, 5,1 KiB“ / „2 Dateien, …“
export function dateText(rfc3339: string): string;                                // „31.10.2026“ (de-DE)
export const PHASE_TEXT: Record<PublishPhase, string>;
export const NO_ROM_CONTENT: string;                                              // der Satz des Kernels
export function commitSubjectNow(now: Date): string;                              // „Stand 2026-10-31 14:05“ (Ortszeit)
export function publishedSubject(version: string): string;                       // „Veröffentlicht v0.1.0“
/** Why „Veröffentlichen“ waits: no found base ROM (`unassigned` → „Erst eine Basis-ROM zuordnen.“, else the status' `reason`), a broken releases list (its sentence); null when it can open. */
export function publishLocked(status: BaseStatus | null, releasesError?: string | null): string | null;
// publish/PublishWizard.tsx
export type PublishWizardProps = {
  /** The releases, the stem and the proposal (null while read; an error shows and locks). */
  defaults: ReleasesDto | null;
  defaultsError: string | null;
  /** The project's own repository is there: „Stand sichern…“ is offered on „Fertig“. */
  gitOwn: boolean;
  onPublish: (opts: PublishOptionsDto, onProgress: (p: { phase: PublishPhase }) => void) => Promise<PublishReportDto>;
  onReveal: (path: string) => Promise<void>;
  /** „Stand sichern…“ on „Fertig“: closes the wizard and opens the dialog with this preset. */
  onCommit: (preset: { message: string; notes: string }) => void;
  /** Left; `published` says the releases list changed. */
  onClose: (published: boolean) => void;
};
export function PublishWizard(props: PublishWizardProps): JSX.Element;
// ui/primitives/IconButton.tsx
export type IconButtonProps = { /* 4.1a */ hint?: string };   // zweite Zeile des Tooltips und `aria-description`
// ui/frame/TitleBar.tsx
export type TitleBarProps = { /* … */ onPublish: () => void; /** Why „Veröffentlichen“ waits (no found base ROM): shown as hint, the button is aria-disabled. Null: it can open. */ publishLocked: string | null };
// app/paletteEntries.ts
export type PaletteCommand = /* … */ | "publish" | "commitAll";
export interface CommandState { /* … */ publishLocked?: string | null; commitAll?: boolean }
// app/useProjectSession.ts
publish(opts: PublishOptionsDto, onProgress: (p: { phase: PublishPhase }) => void): Promise<PublishReportDto>;   // ersetzt buildPatch; über guardedBuild
```

- Schritte und Texte wörtlich (Titel „Veröffentlichen“, Symbol `publish`, `steps` = „Version“, „Patch“, „Prüfen“, „Fertig“; `submitLabel` auf „Prüfen“ = „Fertig“):

| Schritt | Inhalt |
|---|---|
| Version (`data-publish-step=version`) | `TextField` „Version“ (vorbelegt `next_version`, `error` = `versionProblem`), `TextField` „Änderungsnotizen“ (`multiline`, `hint` „Was sich seit der letzten Fassung geändert hat. Steht in der Begleitdatei.“), Zeile „Speicherort“ mit dem Pfad in Mono und „Ordner wählen…“ (sekundär, `openPath({ directory: true, title: "Speicherort wählen" })`), `TextField` „Dateiname“ (vorbelegt `fileNameFor`, folgt der Version, solange der Nutzer ihn nicht geändert hat; `error` = `fileProblem`); `canNext` = beide Probleme null; `summary` „Schreibt {Datei} und die Begleitdatei nach {Ordner}.“ |
| Patch (`data-publish-step=patch`) | Satz „Speichert, baut die ROM und schreibt den Patch gegen die Basis-ROM. Die gebaute ROM bleibt unter build/ im Projekt.“; `Checkbox` „Änderungsnotizen auch als README.txt daneben“ (aus); „Weiter“ startet den Lauf (`busy`, `role="status"` `[data-publish-progress]` mit `PHASE_TEXT`: „Speichern und bauen …“, „Patch kodieren …“, „Selbstprüfung …“, „Dateien schreiben …“, „Veröffentlichung eintragen …“); Fehler als `Notice` (Fehler) mit `errorText`, bei Code `exists` dazu die sekundäre Taste „Ersetzen“ (`[data-publish-overwrite]`, derselbe Lauf mit `overwrite: true`), bei `version_taken` oder `bad_version` zurück auf „Version“ mit dem Satz am Feld |
| Prüfen (`data-publish-step=check`) | `Notice` (Erfolg) „Selbstprüfung bestanden: auf die Basis-ROM angewendet ergibt der Patch genau die gebaute ROM.“ mit „Basis {base_sha1 8 Stellen} → Ziel {target_sha1 8 Stellen}, geprüft in {verify_ms} ms“; Abschnitt „Was du weitergibst“ (`[data-publish-preview]`): `Table` (Datei, Größe) mit Patch, Begleitdatei, README; darunter `NO_ROM_CONTENT`; Submit „Fertig“ |
| Fertig (`result`) | `DoneResult` Titel „Veröffentlicht: v{version}“, Zeilen `{Pfad des Patches}` und `filesLine(files)`; „Als Nächstes“: „Den Patch weitergeben: wer dieselbe Basis-ROM hat, wendet ihn mit Rom Patcher JS oder xdelta3 an.“, mit `gitOwn` „Stand sichern legt einen Commit mit „Veröffentlicht v{version}“ an.“; unter dem Ergebnis mit `gitOwn` die sekundäre Taste „Stand sichern…“ (`[data-publish-commit]`); Rahmentasten „Im Finder zeigen“ (`onShow`, `showLabel`) und „Schließen“ |
| Palette | „Veröffentlichen…“ (Symbol `publish`; gesperrt mit `NO_PROJECT`, `publishLocked`, `BUSY`, `WIZARD_OPEN`), „Stand sichern…“ (Symbol `git`; nur mit `commitAll`, gesperrt mit `BUSY`) |
| Titelleiste | `IconButton` „Veröffentlichen“ (`publish`) zwischen „Bauen“ und „Thema wechseln“; `disabled` ohne Projekt; `busy` während `busy` oder mit `publishLocked`, dann `hint` = `publishLocked` („Erst eine Basis-ROM zuordnen.“ bzw. der Grund der fehlenden ROM) |

- `App.tsx`: `publishLocked = publishLocked(base.status)` (bis Task 6 ohne `releasesError`), Öffnen über `wizards.open({ kind: "publish" })`; `WizardHost` bekommt `publish: { defaults, defaultsError, gitOwn, onPublish: session.publish, onReveal, onCommit }` und liest die Liste beim Öffnen (`releases()`); `onClose(published)` → `base.refresh()` und die Liste der Fläche neu (Task 6); `onCommit(preset)` setzt `commitPreset` und öffnet den Dialog (Task 7).

- [ ] **Step 1: Scheiternde Tests:** `api.publish.test.ts` `publish_commands_send_the_documented_arguments`; `publishWords.test.ts` `file_name_follows_stem_and_version`, `version_and_file_problems_match_the_kernel_sentences`, `files_line_counts_and_sums`, `subjects_for_now_and_for_a_version`; `IconButton.test.tsx` `a_hint_is_the_second_tooltip_line_and_the_description`; `TitleBar.test.tsx` `publish_is_disabled_without_a_project_and_locked_with_its_hint`, `calls_on_publish_by_name`; `paletteEntries.test.ts` `publish_and_commit_all_follow_their_state`; `PublishWizard.test.tsx` `proposes_the_next_version_and_the_file_name`, `a_taken_or_unsafe_version_locks_weiter_with_the_reason` (Review Focus), `the_file_name_follows_the_version_until_edited`, `choosing_a_folder_updates_the_summary`, `weiter_on_patch_runs_publish_with_the_options_and_shows_the_phases`, `an_existing_file_offers_ersetzen` (Review Focus: Ablehnung mit Code `exists` → Taste; Klick ruft `onPublish` mit `overwrite: true`), `pruefen_shows_the_self_check_and_the_files`, `fertig_shows_the_result_reveals_and_offers_stand_sichern_only_with_git`; `WizardHost.test.tsx` (falls vorhanden, sonst in `App.test.tsx`) `publish_request_renders_the_publish_wizard`.
- [ ] **Step 2: Lauf** `cd apps/studio && pnpm test` → FAIL; **Step 3: Umsetzen** (CSS-Module nur mit Tokens; in `PublishWizard.tsx` keine Tiefenvergleiche); **Step 4: Lauf** → PASS, `pnpm build`, `bash scripts/check-styles.sh`, `bash scripts/check-tokens.sh` grün.
- [ ] **Step 5: Blick in der Test-Umgebung** (`docs/agent-gate.md` „Je Lauf“, Ports 1730/1731, Kopie `_testkopien/4-5-ui_DSPRE_contents` mit `4-5-ui.nds`, Basis zugeordnet): Titelleiste mit der Taste (frei und gesperrt mit Tooltip), Assistent in allen vier Schritten, der Lauf, „Ersetzen“; Hell, Dunkel, Kompakt; Screenshots nach `target/agent/gate-<datum>/4-5/` (nicht ins Repo).
- [ ] **Step 6: Commit** `feat(studio): publish wizard in four steps, publish in the title bar and the palette` (mit Trailer).

### Task 6: Technik › Veröffentlichen: Kopf, Taste, Liste

**Files:**
- Modify: `apps/studio/src/technik/PublishSurface.tsx` (+ `.module.css`, test), `apps/studio/src/technik/useBase.ts` (+ test: `createPatch`, `lastPatch` entfallen; `releases`, `releasesError`, `refreshReleases`), `apps/studio/src/technik/TechnikArea.tsx` (+ test: Untertitel), `apps/studio/src/App.tsx` (`onPublish`, `onReveal` an die Fläche; nach dem Assistenten `base.refreshReleases()`)

**Interfaces:**
- Consumes: Task 4, 5 (`releases`, `revealPath`, `ReleasesDto`, `dateText`, `formatBytes`); `Table`, `Button`, `Notice`, `EmptyState`, `IconButton`.
- Produces (TS):

```ts
// technik/useBase.ts (geändert)
export interface BaseApi { status; error; errorFrom: Step | "read" | null; busy: Step | null;   // Step = "check" | "assign" | "register"
  check; assign; register; refresh;
  /** The releases of the project (null while read or on a failure: `releasesError` says why). Read with the status and after a publish. */
  releases: ReleasesDto | null; releasesError: string | null; refreshReleases: () => Promise<void> }
// technik/PublishSurface.tsx
export function publishSubtitle(base: BaseApi | null): string;     // „Veröffentlicht v0.1.0“ (letzte), „Bereit zum Veröffentlichen“, sonst wie 4.1a
// die Sperre kommt aus Task 5: publishLocked(base.status, base.releasesError)
export function PublishSurface(props: { base: BaseApi | null; building: boolean; onOpenProject: () => void; onPublish: () => void; onReveal: (path: string) => Promise<void> }): JSX.Element;
```

- Texte wörtlich: Kopfsatz unter dem Titel „Ein Patch gibt deine Änderungen weiter, ohne die ROM: wer dieselbe Basis-ROM hat, wendet ihn mit Rom Patcher JS oder xdelta3 an.“; Karte „Basis-ROM“ wie 4.1a; Karte „Veröffentlichen“ (`data-publish-card`): Satz „Der Assistent fragt Version und Änderungsnotizen, baut die ROM und schreibt den Patch mit Begleitdatei an den Ort, den du wählst.“, primär „Veröffentlichen…“ (`disabled` ohne gefundene Basis-ROM oder mit kaputter Liste, Notiz daneben = `publishLocked`, `aria-describedby` wie 4.1a; `aria-disabled` während `building`), darunter Überschrift „Bisherige Veröffentlichungen“ und die `Table` (`ariaLabel` „Bisherige Veröffentlichungen“, Spalten „Version“, „Datum“ (`dateText`), „Datei“ (Dateiname in Mono, voller Pfad als `title`), „Größe“ (`formatBytes`), „Notizen“ (erste Zeile, Rest als `title`), letzte Spalte `IconButton` „Im Finder zeigen“ (`folder`-Symbol, vorhandenes `open`), neueste zuerst; leer „Noch keine Veröffentlichung.“; `releasesError` als `Notice` (Fehler) statt der Tabelle.
- Untertitel in der Technik-Liste: `releases.releases` nicht leer und Basis gefunden → „Veröffentlicht v{letzte}“; gefunden → „Bereit zum Veröffentlichen“; `releasesError` → „Nicht lesbar“.

- [ ] **Step 1: Scheiternde Tests:** `useBase.test.tsx` `reads_the_releases_with_the_status_and_again_on_request`, `no_patch_step_remains`; `PublishSurface.test.tsx`: die 4.1a-Tests der Karte „Basis-ROM“ bleiben; neu `publish_button_opens_the_wizard_when_the_base_is_found`, `publish_is_locked_with_the_reason_without_a_base`, `the_list_shows_releases_newest_first_with_date_size_and_reveal`, `an_empty_list_says_so`, `a_broken_releases_file_locks_publishing_with_its_reason` (Review Focus), `while_the_app_builds_publish_waits_aria_disabled`; entfallen: `create_patch_asks_first_…`, `a_patch_without_changes_…`, `a_failed_patch_…`, `the_patch_note_describes_the_patch_button` (ersetzt durch `the_publish_note_describes_the_publish_button`); `TechnikArea.test.tsx` `veroeffentlichen_subtitle_names_the_last_release`.
- [ ] **Step 2: Lauf** → FAIL; **Step 3: Umsetzen** (Fokusregel `next` aus 4.1a zielt jetzt auf „Veröffentlichen…“); **Step 4: Lauf** → PASS, `pnpm build`, Stil- und Token-Prüfung grün.
- [ ] **Step 5: Blick in der Test-Umgebung** (Kopie `4-5-ui`): Fläche ohne und mit Veröffentlichungen, gesperrt ohne Basis-ROM; Hell, Dunkel, Kompakt.
- [ ] **Step 6: Commit** `feat(technik): Veröffentlichen with the wizard button and the list of releases` (mit Trailer).

### Task 7: „Stand sichern“ in der App: Dialog, Technik › Änderungen, Palette, Assistent

**Files:**
- Create: `apps/studio/src/git/CommitDialog.tsx` (+ `.module.css`, test)
- Modify: `apps/studio/src/git/gitWords.ts` (+ test: `commitDoneText`, `changedFilesText`), `apps/studio/src/git/gitTesting.ts` (`commitReportOf`), `apps/studio/src/technik/ChangesSurface.tsx` (+ test: Taste „Stand sichern…“), `apps/studio/src/app/paletteEntries.ts` (`commitAll` läuft), `apps/studio/src/App.tsx` (Dialog, Voreinstellung aus dem Assistenten, Palette)

**Interfaces:**
- Consumes: Task 4 (`gitCommitAll`, `CommitReport`), Task 5 (`commitSubjectNow`, `publishedSubject`, `onCommit`); `GitStatusApi` (4.4), `CheckList`, `Dialog`, `TextField`, `Notice`; `toast`.
- Produces (TS):

```ts
// git/gitWords.ts (angehängt)
export function changedFilesText(n: number): string;      // „Keine Änderung seit dem letzten Commit“, „1 geänderte Datei seit dem letzten Commit“, „{n} geänderte Dateien seit dem letzten Commit“
export function commitDoneText(r: CommitReport): string;  // „Stand gesichert: {short} „{subject}“ ({n} Dateien).“ (+ „ Autor ist „Gen4 Studio“, weil Git deinen Namen noch nicht kennt.“ mit studio_identity)
export const NO_GIT_FOR_COMMIT: string;                   // „Git ist nicht installiert.“ (Tooltip der gesperrten Taste)
// git/CommitDialog.tsx
export type CommitDialogProps = {
  open: boolean;
  status: GitStatus | null;
  unsaved: boolean;
  /** From the wizard: subject „Veröffentlicht v…“ and the notes; null: „Stand <Datum> <Uhrzeit>“ and an empty note. */
  preset: { message: string; notes: string } | null;
  onClose: () => void;
  onDone: (report: CommitReport) => void | Promise<void>;
  exclusive?: <T>(work: () => Promise<T>) => Promise<T>;
  finalFocus?: () => HTMLElement | null;
};
export function CommitDialog(props: CommitDialogProps): JSX.Element;
// technik/ChangesSurface.tsx (erweitert)
export type ChangesSurfaceProps = { /* 4.4 */ onCommitAll: () => void };
```

- Texte wörtlich:

| Stelle | Text |
|---|---|
| Dialog, Titel / Beschreibung (`data-commit-dialog`) | „Stand sichern?“ / „Legt einen Commit mit allen Änderungen im lokalen Repository an (git add -A, git commit). Nichts wird hochgeladen.“ |
| Prüfliste (`data-check`) | „Alles gespeichert“ (offen: „Erst speichern“, sperrt); „Git gefunden: {version}“ (`gitFoundText`) bzw. „Git ist nicht installiert“ (sperrt); `changedFilesText(changes)` (bei 0 sperrt, Zustand `warning`) |
| Felder | `TextField` „Betreff“ (vorbelegt `preset.message` bzw. `commitSubjectNow(new Date())`; leer sperrt), `TextField` „Notiz“ (`multiline`, vorbelegt `preset.notes`, `hint` „Optional; steht im Commit unter dem Betreff.“) |
| Tasten | „Abbrechen“, „Sichern“ (primär); im Lauf keine Tasten, `role="status"` `[data-commit-progress]` „Vorlagen prüfen …“, „Dateien aufnehmen …“, „Commit …“ |
| danach | `toast.info(commitDoneText)`, Dialog schließt, `git.refresh()` (Statusleiste „main · keine Änderung“); `committed: false` mit `reason`: `Notice` (Warnung) `reason`, nur „Schließen“ |
| Technik › Änderungen | bei „Gegen letzten Commit“ mit eigenem Repository neben „Neu vergleichen“ die sekundäre Taste „Stand sichern…“ (`[data-commit-all]`); ohne `git` (`status.cli === null`) `aria-disabled` mit Tooltip `NO_GIT_FOR_COMMIT`; bei 0 Änderungen bleibt sie frei (der Dialog sagt „Keine Änderung …“ und sperrt „Sichern“) |
| Palette | „Stand sichern…“ (Task 5) öffnet denselben Dialog ohne Voreinstellung |
| Assistent | „Stand sichern…“ auf „Fertig“ (Task 5) schließt den Assistenten und öffnet den Dialog mit `preset` |

- [ ] **Step 1: Scheiternde Tests:** `gitWords.test.ts` `changed_files_have_none_singular_and_plural`, `commit_done_text_names_short_subject_and_count`; `CommitDialog.test.tsx` `checklist_fields_then_progress_then_toast` (ruft `gitCommitAll` mit `{ message, notes }`), `unsaved_or_no_git_or_no_change_locks_sichern`, `a_preset_fills_subject_and_notes`, `an_empty_subject_locks_sichern`, `a_failed_commit_shows_the_reason_and_keeps_the_dialog` (Review Focus); `ChangesSurface.test.tsx` `head_with_an_own_repo_offers_stand_sichern`, `without_git_stand_sichern_is_locked_with_the_reason`, `no_repo_offers_git_setup_not_stand_sichern`; `App.test.tsx` `the_wizard_hands_its_preset_to_the_commit_dialog`, `after_a_commit_the_status_line_reads_again`.
- [ ] **Step 2: Lauf** → FAIL; **Step 3: Umsetzen**; **Step 4: Lauf** → PASS, `pnpm build`, Stil- und Token-Prüfung grün.
- [ ] **Step 5: Blick in der Test-Umgebung** (Kopie `4-5-ui` mit `g4k git init`): Dialog vor und während des Laufs, aus dem Assistenten mit Voreinstellung, Technik › Änderungen mit der Taste, Statusleiste danach; Hell, Dunkel, Kompakt.
- [ ] **Step 6: Commit** `feat(studio): Stand sichern as one commit from Änderungen, the wizard and the palette` (mit Trailer).

### Task 8: Design-System fortschreiben

**Files:**
- Create: `design-system/openpencil/beispiele/veroeffentlichen-assistent.body.js`, `design-system/openpencil/NACHTRAG-4-5.md`
- Modify: `design-system/README.md` (Abschnitt „Nachtrag 4.5“ nach „Nachtrag 4.4“; im „Nachtrag 4.1a“ die Abweichungen „Keine Taste „Veröffentlichen“ in der Titelleiste“ und „Das Zielbild … nur teilweise umgesetzt“ mit „(bis 4.5)“ markieren), `design-system/Audit-Design-System.md` (Zeile „Titelleiste“: „Veröffentlichen“ ist da, offen bleibt Windows; Screen 26 „umgesetzt (4.5)“ mit „Assistent in vier Schritten, Liste, Titelleiste; `.fig` folgt“)

**Interfaces:**
- Consumes: Texte und Zustände aus Task 5 bis 7, die Screenshots aus Task 5 bis 7 Step 5.

- [ ] **Step 1: README** „Nachtrag 4.5“ nach dem Muster von „Nachtrag 4.4“: Quelle (`publish/PublishWizard.tsx`, `publish/publishWords.ts`, `git/CommitDialog.tsx`, `technik/PublishSurface.tsx`, `ui/frame/TitleBar.tsx`, `ui/primitives/IconButton.tsx` (`hint`), `app/paletteEntries.ts`, `App.tsx`), Plan- und Gate-Verweis (`docs/phase-4-5-gate.md`), Vergleichsbilder unter `target/agent/gate-<datum>/4-5/` (`blick-<fläche>-<zustand>-<thema>.png`), keine neuen Tokens und Symbole (`publish` und `git` gibt es); Komponenten: „Titelleiste“ (Taste „Veröffentlichen“ zwischen Bauen und Thema, Zustände frei, `busy`, gesperrt mit Hinweis), „Assistent Veröffentlichen“ (vier Schritte mit allen Texten der Tabelle aus Task 5, Fortschritt, „Ersetzen“, Ergebnis), „Technik › Veröffentlichen“ (Kopf, Karte „Basis-ROM“, Karte „Veröffentlichen“ mit Liste; Untertitel), „Dialog „Stand sichern?““ (Prüfliste, Felder, Lauf, Meldung), „Stand sichern“ in Technik › Änderungen und der Palette; Abweichungen: kein Remote, kein Push, kein Staging, keine Branches (Nicht in Phase 4 und 4.5), der Lauf passiert beim „Weiter“ von „Patch“ (Prüfen zeigt das Ergebnis), die `.fig` kennt die Frames noch nicht („folgt“).
- [ ] **Step 2: Vorlage** `beispiele/veroeffentlichen-assistent.body.js` nach dem Muster von `aenderungen-git.body.js` (Bausteine aus `jsx-lib-app.js`, Farben über `v('token')`), `MODE`: `'version'` (Frame „App: Assistent, Veröffentlichen, Version“: Welt mit Schrittleiste im Scharnier, Schritt Version in der Hand mit `0.1.1`, zwei Notizzeilen, Speicherort `…/Sinnoh-Remix_DSPRE_contents/build`, Dateiname `Sinnoh-Remix-v0.1.1.xdelta`), `'pruefen'` (Frame „App: Assistent, Veröffentlichen, Prüfen“: Erfolgsmeldung, Tabelle mit drei Dateien, Hinweis), `'liste'` (Frame „App: Technik, Veröffentlichen mit Liste“: Karte Basis-ROM gefunden, Karte Veröffentlichen mit zwei Zeilen in der Liste), `'sichern'` (Frame „App: Dialog, Stand sichern“: Prüfliste mit drei Zeilen, Betreff „Veröffentlicht v0.1.1“, Notiz); Titelleiste in jedem Frame mit der neuen Taste; Projekt „Sinnoh-Remix“, Platin DE.
- [ ] **Step 3: Anleitung** `NACHTRAG-4-5.md` nach dem Muster von `NACHTRAG-4-4.md`: „Die Datei wurde nicht angefasst“ (bis Step 4 gelingt); drei App-Frames auf „App: Texte, Technik, Dialoge“ (nächste freie Spalte der Reihe 0 bzw. Reihe 1 für den Dialog, x = Spalte × 1520, y = Reihe × 1000, vorher `get_page_tree`), Bauen (`bash ../build-screen.sh veroeffentlichen-assistent`, `MODE` je Frame, `op.mjs render`, Vergleich per `export_image` mit den Screenshots, erzeugte `.jsx` nicht einchecken), danach `fix_fig_text.mjs --rgb555` und README-Satz; Brett „Stand der App“: „Veröffentlichen (4.1a)“ → „Veröffentlichen (4.5)“ unter „Umgesetzt“; was nicht gezeichnet werden muss (Lauf, Fehler, „Ersetzen“, Palette, gesperrte Titelleisten-Taste).
- [ ] **Step 4: `.fig`** nur, wenn OpenPencil antwortet (bei gesperrtem Bildschirm laufen die RPCs in Zeitüberschreitungen; bei 10 000 Knoten Einzeländerungen vermeiden und die Frames neu rendern); sonst bleibt „die `.fig` kennt das noch nicht; die Frames folgen“.
- [ ] **Step 5: Audit** wie unter Files; `bash scripts/check-tokens.sh` grün.
- [ ] **Step 6: Commit** `docs(design-system): addendum 4.5 publish wizard, releases list, title bar button, Stand sichern` (mit Trailer).

### Task 9: Gate 4.5

**Files:**
- Create: `docs/phase-4-5-gate.md`
- Modify: `docs/ROADMAP.md` (Übersicht Zeile 3d: 4.5 mit Stand; Abschnitt „Phase 4“: neuer Absatz „4.5 Veröffentlichen“ nach 4.4 mit Lieferumfang, Gate und Stand, der Satz „Nicht in Phase 4“ bekommt „(„Stand sichern“ kam mit 4.5)“; Tabelle 3d Zeile 4.5 mit Stand; „Aus dem Design-System, später“: der Punkt Veröffentlichen nennt 4.5 als umgesetzt), `docs/agent-gate.md` (Gate-Datei in der Liste oben, Abschnitt „Phase 4.5“ mit Befehlen und `data-*`, „Was beim Nutzer bleibt“: 4.5), `docs/dev-setup.md` (die gated Tests in der Tabelle, falls Task 2 und 3 es nicht schon taten), `docs/release.md` (nur falls berührt: nichts in 4.5 ändert den App-Release; sonst unverändert)

**Interfaces:**
- Consumes: alles Vorige; die Ausgaben `publish …`, `commit …` der gated Tests.

- [ ] **Step 1: Gate-Dokument** `docs/phase-4-5-gate.md` nach dem Muster von `docs/phase-4-4-gate.md`: Kopf mit Stand, Gate-Text der Spezifikation (§4 Zeile 4.5 und C §6.4) als Zitat; Legende „Wer“ (Agent, Agent + Blick, Hand) mit Verweis auf `docs/agent-gate.md`; Namen in der Oberfläche (Titelleiste „Veröffentlichen“ mit `hint`, `[data-technik=veroeffentlichen]`, `[data-publish-card]`, Tabelle „Bisherige Veröffentlichungen“, Schrittleiste mit „Version, Patch, Prüfen, Fertig“, `[data-publish-step]`, `[data-publish-progress]`, `[data-publish-overwrite]`, `[data-publish-preview]`, `[data-publish-commit]`, `[data-commit-dialog]`, `[data-check]`, `[data-commit-progress]`, `[data-commit-all]`, `[data-git-status]`, Meldungen „Veröffentlicht: v0.1.0“, „Stand gesichert: …“, Agentenfrage „Patch ersetzen: …“ und „Stand sichern“); Vorbereitung (Kopien `_testkopien/4-5-{pt,hgde,hgus}` für (1), `4-5-k{pt,hgde,hgus}` für (2), `4-5-web`, `4-5-mcp`, `4-5-ui` für (3), je mit `.nds` daneben; `cargo build --workspace --all-features` und Release-`g4k`/`studio`; `git --version` notieren; `G4_BASE_ROM_DIR=<Roms>/_testkopien`; Test-Umgebung nur 1730/1731; Worktree-Hinweise; Last mit `uptime`).
  - **(1) Automatik:** `scripts/gate.sh` grün; einzeln je Projekt im Release-Profil mit `--nocapture --test-threads 1`: `publish_after_a_map_edit_writes_a_small_patch_that_applies` (`--test base`), `publish_then_commit_all_leaves_status_empty_and_no_remote` (`--test git`), der gated Studio-Test `publish_and_commit_over_the_command_path_on_the_vanilla_copy`.
  - **(2) Szenario kopflos** (je Spiel, Release-`g4k` und `studio mcp --project <Kopie> --base-rom <nds> [--allow-irreversible]`): `g4k base assign`; `g4k git init` → `committed`; MCP `data_edit` Trainer 1 (Level +1), `save`; `publish_patch {"version":"0.1.0","notes":"Erste Fassung","readme":true}` ohne `--allow-irreversible` → läuft (keine Frage, nichts wird ersetzt), Antwort mit drei Dateien unter `build/`; dieselbe Version noch einmal → `version_taken`; `{"version":"0.1.1"}` ohne `overwrite` auf vorhandenem `--file` → `exists`; mit `"overwrite": true` ohne `--allow-irreversible` → `needs_confirmation`, mit → ersetzt; Begleitdatei: die ersten fünf Zeilen wie 4.1a, dazu `Version:`, `Datum:`, `Änderungen:`; `xdelta3 -d -s <nds> <patch> out.nds` (wenn im `PATH`) → SHA-1 = `Ziel-ROM SHA-1` der Begleitdatei = SHA-1 von `build/<patch_name>.nds`; `git -C <Kopie> status --porcelain` zeigt `.g4/releases.json` und den Trainer-Member, nichts unter `build/`; `git_commit_all {"message":"Veröffentlicht v0.1.0","notes":"Erste Fassung"}` ohne `--allow-irreversible` → `needs_confirmation`, mit → `committed`; `git status --porcelain` leer, `git log -1 --format=%B` = Betreff und Body, `git remote -v` leer, `.git/config` ohne `[remote`; `g4k publish <Kopie> --version 0.2.0 --dir <Temp-Ordner>` → Dateien dort, `releases.json` mit absolutem `file`; `g4k git commit-all <Kopie>` → `committed`, zweites Mal → `NOTHING_TO_COMMIT`.
  - **(3) Browser-Teil** (Test-Umgebung 1730/1731, `4-5-web`, Basis zugeordnet, Git eingerichtet): Technik › Veröffentlichen zeigt die leere Liste; Titelleiste „Veröffentlichen“ → Assistent: Vorschlag `0.1.0` und `4-5-web-v0.1.0.xdelta`, Notizen eintragen, „Ordner wählen…“ (Pfadfeld im Browser-Modus), Kästchen README, „Weiter“ → Fortschritt → Prüfen mit drei Dateien und Hinweis → „Fertig“ → Ergebnis; „Stand sichern…“ aus dem Ergebnis → Dialog mit Betreff „Veröffentlicht v0.1.0“ und den Notizen, Prüfliste, „Sichern“ → Meldung, Statusleiste „main · keine Änderung“; Technik › Veröffentlichen zeigt die Zeile; zweiter Assistent mit gleicher Version → Sperre am Feld; gleicher Dateiname (von Hand) → „Ersetzen“; `.nds` umbenennen → Titelleiste gesperrt mit Hinweis „Basis-ROM nicht gefunden: …“, Palette „Veröffentlichen…“ gesperrt mit demselben Grund; Technik › Änderungen „Gegen letzten Commit“: Trainer ändern, speichern, „Stand sichern…“ ohne Voreinstellung (Betreff „Stand <Datum> <Uhrzeit>“) → Statusleiste „keine Änderung“; ohne `git` (`G4_GIT=/nicht/da` für die Brücke) ist die Taste gesperrt mit „Git ist nicht installiert.“; MCP über die Brücke (`4-5-mcp`, Freigabe „Lesen und bearbeiten“): `mcp.sh call publish_patch '{"version":"0.1.0"}'` ohne Frage; `mcp.sh call publish_patch '{"version":"0.1.1","file":"4-5-mcp-v0.1.0.xdelta","overwrite":true}'` → Frage „Patch ersetzen: …“; `mcp.sh call git_commit_all '{}'` → Frage „Stand sichern“.
  - **(4) Hand und Blick beim Nutzer:** Rom Patcher JS mit der eigenen Basis-ROM und einem Patch aus (2), SHA-1 des Ergebnisses gegen die Begleitdatei (Hand, wie 4.1a offen); „Im Finder zeigen“ in der gepackten App (Hand); das eigene Git-Werkzeug zeigt den Commit „Veröffentlicht v0.1.0“ mit Body und ohne Remote (Hand); Blick auf Titelleiste, Assistent, Fläche, Dialog in Hell, Dunkel, Kompakt (Bilder aus Task 5 bis 7).
  - **Ergebnis:** Tabelle Punkt / Wer / Ergebnis / Datum und die Zahlentabelle:

    | Spiel | Patch (B) nach Trainer-Änderung | Build (ms) | Kodieren / Prüfen (ms) | Commit: Dateien, add (ms), commit (ms) | Status danach (ms) | xdelta3 gleich | Last |
    |---|---|---|---|---|---|---|---|
    | Platin DE | | | | | | | |
    | HeartGold DE | | | | | | | |
    | HeartGold US | | | | | | | |

- [ ] **Step 2: Gate abarbeiten** (Agent: (1) bis (3)); Werte und Befunde eintragen; Befunde beheben (eigene Commits `fix(4.5): …` mit Regressionstest), bis alles grün ist; `scripts/gate.sh` zuletzt allein aus dem Worktree (absolutes `G4_ROMS_DIR`, `tools`-Symlink).
- [ ] **Step 3: Roadmap und Anleitungen** wie unter Files: im Abschnitt „Phase 4“ der Absatz „**4.5 Veröffentlichen** (S bis M, Zwischenphase parallel zu 3d.1): Assistent in vier Schritten …; „Stand sichern“ …; MCP `publish_patch` (Rückfrage beim Ersetzen) und `git_commit_all` (Rückfrage); `g4k publish`, `g4k git commit-all`. Gate: … **Stand <Datum>:** Gate vom Agenten bestanden: …, Branch `phase-4-5-veroeffentlichen`; [docs/phase-4-5-gate.md](phase-4-5-gate.md); beim Nutzer offen: Rom Patcher JS, „Im Finder zeigen“ in der gepackten App, eigenes Git-Werkzeug, Blick; die `.fig` folgt“; `docs/agent-gate.md` Abschnitt „Phase 4.5“ (Kopien, Befehle aus (1) bis (3), `data-*`, MCP-Zeilen, der Hinweis, dass `publish_patch` ohne `overwrite` nicht fragt).
- [ ] **Step 4: Commit** `docs(4.5): gate of Veröffentlichen, roadmap status` (mit Trailer).

---

## Offen für die Umsetzung

- **`WizardRequest` und `WizardHost`:** `apps/studio/src/workbench/WorkbenchContext.tsx` und `app/WizardHost.tsx` sind nicht in der Sperrliste der 3d-Phasen, aber 3d.2 (Rezept-Taste, reduzierter Inspector) kann sie berühren; die Änderung von 4.5 ist je eine angehängte Zeile bzw. ein angehängter `case`. Trifft der Merge auf Konflikte, gilt: beide Anhänge behalten.
- **`Sub` ohne `Copy`:** verlieren Aufrufer in `cli.rs` dadurch `sub` nach `run(..)`, wird `Sub::Commit` einmal gebaut und als Referenz übergeben (`run(cli, dir, &sub, ..)`); die Testliste `Sub::ALL` wird `fn all() -> Vec<Sub>`.
- **Ortszeit kopflos:** `g4k git commit-all` ohne `--message` nimmt den UTC-Betreff des Kernels; wer die Ortszeit will, gibt `--message`. Soll `g4k` Ortszeit schreiben, braucht der Kernel oder `g4k` eine Zeitzonen-Quelle (etwa das Crate `time` mit `local-offset`), das ist eine eigene Entscheidung (Lizenz, `cargo deny`).
- **Build im Assistenten:** `publish_patch_checked` nimmt den Build-Weg von `create_patch_checked` (Speichern, Agenten-Zug, Build nach `build/<patch_name>.nds`); ändert sich dieser Weg durch 3d.2 (Rezepte schreiben Dateien), bleibt die Regel „der Patch ist der letzte Build, die ROM nicht die letzte ROM“.
- **`reveal_path` unter Linux:** `xdg-open <Ordner>` öffnet nur den Ordner, nicht die markierte Datei; unter Windows ist `explorer /select,` ungeprüft (kein Windows). Beides bleibt beim Nutzer.
- **Patch außerhalb von `build/` im Projekt:** wählt der Nutzer einen Ordner im Projekt außerhalb `build/`, ignoriert die Vorlage den `.xdelta` (`*.xdelta`), aber nicht `.xdelta.txt` und `README.txt`; „Stand sichern“ nimmt sie dann mit. Soll die Vorlage Texte neben Patches ignorieren, ist das eine Zeile `*.xdelta.txt` im Block (nicht in 4.5 entschieden).
- **`DSPRE`-Layout und neue Archive:** `commit_all` packt nicht aus (anders als `git_init`); ein nach dem Einrichten erstmals ausgepackter Ordner liefert eine NARC-Zeile in der Vorlage erst beim nächsten `write_templates`, die `commit_all` jetzt vor dem `add -A` macht; die bis dahin versionierte NARC bleibt im Index (Git entfernt ignorierte Dateien nicht aus dem Index). Eine Bereinigung (`git rm --cached`) ist kein Unterbefehl von `Sub` und bleibt außen vor.
- **Werkzeugzahl:** mit 3d.1 bis 3d.3 und 4.5 kann `tool_count_is_about_thirty` die Grenze erreichen; dann hebt Task 4 sie mit Begründung im Commit.
- **`.fig`:** OpenPencil war beim Schreiben des Plans nicht erreichbar; Task 8 legt Vorlage und Anleitung an, die Frames folgen in einer eigenen Sitzung nach dem Merge (wie 4.4).
