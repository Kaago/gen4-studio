# Phase 4.4 „Git minimal“ Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ein Projekt (DSPRE-Layout oder Textprojekt) lässt sich über „Git einrichten…“ (Technik › Änderungen, Palette, Häkchen im Assistenten, MCP mit Rückfrage, `g4k git init`) als lokales Git-Repository anlegen: der Kernel schreibt `.gitignore` und `.gitattributes` aus Vorlagen je Layout (eine vorhandene Datei bekommt nur einen markierten Block), die `git`-CLI macht `git init` und den ersten Commit „Ausgangsstand“; Status, Branch und `HEAD` liest `gix` im Prozess auch ohne installiertes Git; Technik › Änderungen vergleicht jede Entität semantisch gegen die Basis-ROM, gegen den letzten Commit und gegen den gespeicherten Stand („Feld: alt → neu“, Karten je Teil mit Zellen der Bewegungsrechte, Texte je Nachricht mit Wort-Diff, Scripts als Text-Diff), mit „Öffnen“ und „Auf Original zurücksetzen…“; die Statusleiste zeigt Branch und Zahl der Änderungen; ein Klon eines Textprojekts lässt sich aus der Basis-ROM vervollständigen und baut byte-gleich.

**Architecture:** Zwei neue Kernel-Module. `git` liest mit `gix` (Repo finden, auch ein umschließendes, Status mit unverfolgten Dateien, `HEAD`, Blob eines Pfads unter `HEAD`), schreibt die Vorlagen (`git::templates`, Metadaten unter `crates/g4-kernel/metadata/git/`) und ruft für das Einrichten die `git`-CLI mit einer festen Liste von Unterbefehlen (`git::cli`, nie `remote`, `push`, `fetch`, `clone`). `diff` vergleicht eine Entität (`DiffSubject`): beide Seiten werden zu demselben JSON-DTO, das das Textprojekt schreibt (Member-Bytes über den In-Speicher-Export von 4.3a/4.3b, Text-JSON über die Datei, Scripts als rotom-Quelltext), Binärteile werden auf ihre SHA-1 normalisiert, dann entsteht je Form eine Liste (Felder, Karte, Nachrichten, Zeilen, Binär); Wort- und Zeilen-Diff laufen über `gix`s `blob-diff` (imara-diff). Dazu kommt `textproject::restore` (Klon vervollständigen). Die App bekommt vier Befehle (`git_status`, `git_init`, `diff_entity`, `unsaved_changes`) plus `restore_text_project`, drei MCP-Werkzeuge, ein Statusleisten-Element, die Dialoge „Git einrichten?“ und „Textprojekt vervollständigen?“ und in Technik › Änderungen einen Umschalter mit Detailansicht.

**Tech Stack:** Rust (g4-kernel, `gix` mit `default-features = false` und den Features `sha1`, `status`, `index`, `revision`, `blob-diff`, kein `git2`; die `git`-CLI über `std::process::Command`), serde_json mit `preserve_order`, Tauri-Befehle mit `Channel` für den Fortschritt, MCP (Befehlstabelle, Katalog, Zugriff), React 19 mit den Bausteinen aus `apps/studio/src/ui/`, vitest, `cargo deny`.

**Spec:** [entscheidungen.md](../../research/2026-10-27-phase-4/entscheidungen.md): „Phasenschnitt und Reihenfolge“ (Zeile 4.4), „Git (Gutachten C, 4.4)“ (die Spalte „Entscheidung“ gilt wörtlich), „Textprojekt“ (Zeilen „Basisschicht“, „Format und Ablage“), „Nicht in Phase 4“; [gutachten-c-git-einordnung.md](../../research/2026-10-27-phase-4/gutachten-c-git-einordnung.md) §2 (Messungen Git auf dem DSPRE-Ordner, Zeilenenden), §3 (Projektschicht), §4 (Rechtliches), §5 (Vorlagen), §6 (Diff je Entität, Bibliothek, kein Commit in der App), §11 (Gate-Vorschlag); [ROADMAP.md](../../ROADMAP.md) „Phase 4“, Punkt 4.4 mit Gate und das Gate der Phase; der Plan [4.3b](2026-10-28-phase-4-3b-textprojekt-quelle.md) (Abschnitt „Offen für die Umsetzung“, letzter Punkt), der Plan [4.1b](2026-10-27-phase-4-1b-auf-original-zuruecksetzen.md) (Technik › Änderungen), [phase-4-3a-gate.md](../../phase-4-3a-gate.md) (Hinweis H2).

Stand 28.10.2026: `main` bei `4397c54` (4.0, 4.1a, 4.2, 4.1b, 4.3a mit Abnahme gemergt); **4.3b (`phase-4-3b-textprojekt-quelle`) wird vor dem Start von 4.4 auf main gemergt**, der Plan setzt diesen Stand voraus (Abschnitt „Voraussetzungen aus 4.3b“). Arbeit im eigenen Worktree `.claude/worktrees/phase-4-4`, Branch `phase-4-4-git-minimal`, ohne parallelen Branch. Kernel `crates/g4-kernel/src`, Tauri `apps/studio/src-tauri/src`, Frontend `apps/studio/src`.

## Voraussetzungen aus 4.3b

Vor Task 1 auf dem gemergten main prüfen (`grep` genügt); heißt etwas anders, folgt der Plan dem gemergten Namen, die Regeln bleiben. Fehlt etwas, steht dabei, was 4.4 selbst ergänzt.

| Schnittstelle | erwartet | gebraucht in |
|---|---|---|
| `Project::source_mode() -> &SourceMode` (`Dspre`, `TextProject`, `Older`, `Newer`, `Broken`) | wie im Plan 4.3b Task 3 | Task 2, 5 (Layout) |
| `textproject::source::{EntitySource, EntityUnit, unit_of, text_archive_key, script_key, raw_key}` | Einheit eines Assets mit `sources`, `members`, `assets` | Task 5, 6 |
| In-Speicher-Export einer Einheit aus Member-Bytes, ohne zu schreiben (4.3b braucht ihn für `sync::conflict_diff`, „DSPRE-Seite in den Speicher exportiert“) | hier `EntitySource::export_unit(&mut self, project, unit: &EntityUnit, members: &[Option<Vec<u8>>]) -> Result<Vec<(PathBuf, Vec<u8>)>, KernelError>`; fehlt er, legt Task 5 ihn in `textproject/source/writers.rs` an (aus `export::entity_files`) | Task 5 |
| `EntitySource::open` außerhalb eines Textprojekts | lehnt 4.3b ab, ergänzt Task 5 `EntitySource::open_for_diff(project)`: Symbole ohne Taufe (Register nur, wenn es sie gibt, sonst `symbols::fallback_id`), keine Basisschicht, schreibt nie | Task 5 |
| JSON-Felddiff von `sync::conflict_diff` (Pfade wie in den Import-Fehlern, Werte kompakt, Obergrenze, Rest als Zahl) | Task 5 zieht die reine Funktion nach `diff/json.rs` (`json_changes`), `conflict_diff` ruft sie mit Obergrenze 40, Ausgabe unverändert | Task 5 |
| `textproject::ids::{Resolver, RefInput}`, `apps/studio/src-tauri/src/mcp/symbolic.rs` (`SYMBOLIC_ARGS`, `resolve_args`) | ID oder Nummer in MCP-Argumenten | Task 7 (`diff_entity`) |
| `textproject::adopt::adopt(project, base, progress)`, `textproject::sync::{record_all, scan}` | Umstellung, Aufzeichnung des Abgleichs | Task 4, 6 (gated) |
| `start::{CreateOptions { template, text_project }, create_project_with_options, Phase::TextProject}` | Assistent mit Häkchen „Als Textprojekt einrichten“ | Task 3, 8 |
| `apps/studio/src/platform/windowFocus.ts` `subscribeWindowFocus(onFocus) -> () => void` | Fokus des Fensters | Task 8 |
| Technik mit acht Einträgen (`textprojekt` nach `projekt`) | `TechnikArea.tsx` | Task 9 (Tests zählen die Einträge) |
| `.g4/include/`, `.g4/backup/textproject-<UTC>/` | lokal | Task 2 (fallen unter `/.g4/*`) |

Aus 4.1b und früher (auf main): `original::{Original, ResetTarget, ChangedEntry, ChangedReport, changed_from_original, asset_label, NO_ORIGINAL}`, `base::BaseRom::{open, open_unchecked}`, `test_support::{synthetic_project, synthetic_rom, vanilla_copy, vanilla_base_rom, ProjectOriginal}`, `build::{build, extract_rom, pack_entries}`, `Project::{ensure_unpacked, read_headers, read_header_table_bytes, asset_location}`, `Store::{dirty, saved_bytes, current_bytes, header_table}`, `scripts::decompile_member`, `start::assign_base`, `commands::base::{found_base, base_missing}`, `Session::{after_write, files_written, disk_changed}`.

## Entscheidungen (wo Roadmap und Gutachten offen lassen)

| Frage | Entscheidung | Grund |
|---|---|---|
| Ort des Codes | Kernel-Module `git` (`repo.rs` mit gix, `templates.rs`, `cli.rs`) und `diff` (`subject.rs`, `json.rs`, `text.rs`, `side.rs`), dazu `textproject/restore.rs`; App `commands/git.rs`; Frontend `git/`, `technik/EntityDiff.tsx`, Baustein `ChangeTable` | Gutachten C §11 „Kernel: git_templates, git_status und git_blob_at_head über gix, git_init über die CLI, diff_entity“; kopflos über `g4k` und MCP |
| `gix`-Version | neueste Version, mit der `cargo deny --locked check licenses advisories bans` ohne neue Ausnahme grün ist, mindestens 0.90 (gemessen in C §6.3); `default-features = false`, Features genau `sha1`, `status`, `index`, `revision`, `blob-diff`; braucht der Status eine weitere Feature (etwa für den Vergleich Baum ↔ Index), kommt sie dazu und `cargo deny` läuft erneut; `deny.toml` bekommt unter `[bans]` `git2` und `libgit2-sys` als verboten | Entscheidung „gix zum Lesen, kein git2“; das Verbot hält sie im Lizenz-Gate fest |
| Vorlagen | drei Dateien unter `crates/g4-kernel/metadata/git/` (`dspre.gitignore`, `text.gitignore`, `gitattributes`), per `include_str!`; der Kernel ersetzt Platzhalter (`{{packed_archives}}`, `{{dspre_part}}`) aus dem Layout; der ganze Inhalt steht in einem Block zwischen `# >>> Gen4 Studio (dieser Block wird von Gen4 Studio gepflegt) >>>` und `# <<< Gen4 Studio <<<` | Entscheidung „Metadaten im Kernel“; ein Block lässt sich wiederfinden und ersetzen |
| Vorhandene Datei | ohne Block: Block ans Ende (eine Leerzeile davor, Zeilen des Nutzers byte-gleich); mit Block: Block ersetzen, wenn er abweicht, sonst nichts schreiben; Anfang ohne Ende: Ablehnung mit Satz, nichts geschrieben; die Datei bekommt den Block immer **am Ende** | Entscheidung „nur markierten Block ergänzen“; in beiden Dateien gewinnt die spätere Zeile, so gelten `* -text` und die Ignorier-Regeln auch hinter einem `* text=auto` des Nutzers |
| Layout der Vorlage | `SourceMode::TextProject`, `Older` und `Newer` → Textprojekt, sonst DSPRE; nach „Als Textprojekt einrichten…“ ersetzt das nächste Schreiben (oder „Git einrichten…“) den DSPRE-Block durch den Textblock | ein umgestelltes Projekt soll seine Spieldateien nicht weiter versionieren |
| DSPRE: gepackte Archive | je Pack-Eintrag aus `build::pack_entries` eine Zeile `/<NARC-Pfad>`, nur wenn `unpacked/<Ordner>/` existiert; „Git einrichten…“ packt vorher im DSPRE-Layout jede Art aus, die das Studio kennt (`Project::ensure_unpacked` je Art aus `ArchiveMap::kinds()`), damit alle Zeilen gelten | Entscheidung „nur wenn ihre unpacked/-Ordner da sind“; seit 4.2 packt der Build jeden ausgepackten Ordner (auch die 14/17 DSPRE-Ordner), also sind genau diese NARCs erzeugt; ein später lazy ausgepacktes Archiv wäre sonst schon versioniert und zeigte nach jedem Build eine Änderung |
| Textprojekt: DSPRE-Teil | `{{dspre_part}}` = feste Liste `DSPRE_PART` der Namen, die `build::extract_rom` in der Wurzel anlegt, plus `/unpacked/`; ein Test prüft die Liste gegen ein Extract der synthetischen ROM; `expanded/` bleibt versioniert; von `.rotom/` bleibt nur `command_database` (`/.rotom/*`, `!/.rotom/command_database`), `rotom.toml` bleibt versioniert | Entscheidung „DSPRE-Teil außer expanded/“; `textproject::sources()` (4.3a) zählt `rotom.toml` und `.rotom/command_database` zu den Quellen; die Quirks stehen in `data/rotom-quirks.json`, `compile-state.json` ist dort Cache |
| Sicherungen in der Wurzel | beide Vorlagen ignorieren `*.bak`, `*.orig`, `*~`, die Textvorlage dazu `/*.prev` und `/*.prev/`, mit Kommentar | 4.3a-Gate H2: Sicherungen neben `data/` werden beim Export Blobs; die Vorlage kann den Export nicht ändern, aber die üblichen Muster aus Git heraushalten |
| `.gitattributes` | gleich für beide Layouts: `* -text`, `text eol=lf` für `*.json`, `*.rotom`, `*.yaml`, `*.yml`, `*.toml`, `*.md`; Kommentar mit der Begründung und einer auskommentierten LFS-Zeile als Platzhalter | Entscheidung „Pflicht“; Phase 6 ergänzt LFS im Block |
| `git` finden | `git::cli::find_git`: `G4_GIT`, dann `PATH`, dann `/opt/homebrew/bin/git`, `/usr/local/bin/git`, unter Windows `%ProgramFiles%\Git\cmd\git.exe`; `/usr/bin/git` auf macOS nur, wenn `xcode-select -p` gelingt; geprüft mit `git --version` | eine App aus dem Finder hat nur `/usr/bin:/bin:/usr/sbin:/sbin` im `PATH`; `/usr/bin/git` ohne Command Line Tools öffnet sonst den Installationsdialog von macOS |
| Unterbefehle | nur `--version`, `init -q`, `add -A`, `var GIT_COMMITTER_IDENT`, `commit -q -m Ausgangsstand` (privates Enum `Sub`); Umgebung `GIT_TERMINAL_PROMPT=0`, stdin leer, Zeitgrenze 300 s | Entscheidung „kein Remote, kein Push“, im Code erzwungen; keine hängende Abfrage einer GUI-App |
| Branch | `git init` ohne `-b`: der Name kommt aus der Git-Konfiguration des Nutzers (`init.defaultBranch`) | die App greift nicht in die Gewohnheiten des Nutzers ein; Tests lesen den Namen statt ihn anzunehmen |
| Kein Name in Git | scheitert `git var GIT_COMMITTER_IDENT`, läuft nur dieser Commit mit `-c user.name="Gen4 Studio" -c user.email=studio@gen4.invalid`; der Bericht sagt es (`studio_identity`), die Meldung nennt es | ein Klick soll reichen; `.invalid` ist reserviert und nie zustellbar; die Konfiguration des Nutzers bleibt unberührt |
| Commit scheitert (Signatur, Hook, Zeit) | Repository und Vorlagen bleiben, `committed: false` mit `Der erste Commit ist gescheitert: <stderr>. Repository und Vorlagen sind angelegt; committe mit deinem Git-Werkzeug.`; keine Abschaltung von `commit.gpgsign` | die Einstellung des Nutzers wird respektiert, der Fehler ist lesbar |
| Vorhandenes oder umschließendes Repo | `git_init` schreibt nur die Vorlagen und antwortet `committed: false` mit `IN_REPO`; „Git einrichten…“ wird nur ohne Repo angeboten; gelesen wird ein umschließendes Repo relativ zum Projekt | Gutachten C §6.3 (Dotfile-Repo im Home-Ordner); nie ein Repo im Repo |
| Ungespeicherte Änderungen | `git_init` lehnt mit `unsaved` ab, bevor es fragt („Erst speichern: der erste Commit nimmt den gespeicherten Stand.“) | Muster 4.3a B3; der Commit sähe sonst einen anderen Stand als die App |
| Zahl der Änderungen | geänderte Pfade seit `HEAD` wie `git status` sie zählt: geändert, hinzugefügt, gelöscht (Index und Arbeitsbaum zusammen, je Pfad einmal) und unverfolgt, nicht ignoriert; eine Umbenennung zählt als gelöscht plus neu | stimmt mit dem Git-Werkzeug des Nutzers überein; Entitäten zählt Technik › Änderungen |
| Wann der Status gelesen wird | beim Öffnen, nach Speichern, Bauen, Zurücksetzen, ⌘Z/⌘⇧Z mit Dateischritt, Abgleich (4.3b), Schreiben eines Agenten, nach „Git einrichten“ und bei Fensterfokus (verzögert 800 ms wie 4.3b); nie zwei Läufe zugleich, einer mehr, wenn währenddessen ein Grund kommt; ohne Sitzungsschloss | Status kostet 54–82 ms (C §6.3); ein Commit im Git-Werkzeug des Nutzers soll beim Zurückkehren sichtbar sein |
| Statusleiste | ohne Repo kein Element; sonst Symbol `git` und „main · 3 Änderungen“ („1 Änderung“, „keine Änderung“, ohne Commit „noch kein Commit“, losgelöst „<kurz> (losgelöst)“ statt des Branch); Klick öffnet Technik › Änderungen „Gegen letzten Commit“ | Entscheidung „Statusleiste mit Branch und Zahl der Änderungen“ |
| Diff: neue Seite | gegen Basis und gegen letzten Commit der **gespeicherte** Stand (Dateien), gegen gespeichert der **ungespeicherte** Wert im Store; Zeilen mit ungespeicherter Bearbeitung tragen in den ersten beiden „Ungespeichert“ | jede Liste und ihr Detail beschreiben denselben Stand; wie 4.1b „Gezählt wird der gespeicherte Stand“ |
| Diff: DTO | beide Seiten werden JSON des Textprojekt-Schemas: im Textprojekt die Entitätsdatei selbst (`HEAD`-Blob und Datei), sonst Member-Bytes über den In-Speicher-Export (`EntitySource::open_for_diff`); im DSPRE-Layout tragen Trainer, Trainerklassen und Orte Ersatz-IDs (`TRAINER_0001`), es wird nie getauft und nie geschrieben | Entscheidung „das DTO ist dasselbe, das 4.3 schreibt“; eine Taufe einer einzelnen Entität ohne die übrigen ergäbe andere Suffixe als die Umstellung |
| Binärteile im DTO | jedes Teilobjekt (`{"base_sha1"}`, `{"file"}`, `{"hex"}`, auch in `{"locked","raw"}`) wird vor dem Vergleich zu `{"sha1": "<40 hex>"}` der Bytes | sonst zeigte ein gleicher Teil in zwei Formen einen Unterschied |
| Formen des Diffs | `fields` (JSON-Pfad, alt, neu; höchstens 200 Zeilen, Rest als Zahl), `map` (Teile Modell, Bewegungsrechte, Gebäude, BDHC, BGS geändert ja/nein mit SHA-1; je abweichende Zelle `x, y` mit `kkcc` alt und neu, höchstens 200; Gebäude als Felder), `messages` (je Nachrichtenindex: neu, entfernt, geändert mit Wort-Spannen), `lines` (Hunks mit drei Zeilen Umgebung, höchstens 400 Zeilen), `binary` (Größe und SHA-1 je Seite, „wie in der Basis-ROM“, „fehlt“) | Entscheidung „Karten binär nur Teil geändert plus Zellen-Diff, Texte je Nachricht mit Wort-Diff, Scripts als Text-Diff“; die Karten-JSON hat die Bewegungsrechte schon als Gitter `kkcc` |
| Wort- und Zeilen-Diff | im Kernel über `gix::diff::blob` (imara-diff, Histogram; Wörter = Folgen von Buchstaben/Ziffern, Leerraum, Einzelzeichen); das Frontend zeigt nur die Spannen | `blob-diff` ist ohnehin gewählt; eine Stelle für App und MCP |
| Werte in der Tabelle | Zeichenketten ohne Anführungszeichen, sonst kompaktes JSON, fehlend `—`; Spalte „Feld“ ist der JSON-Pfad in Mono (`data.party[0].level`), beim Header der Feldname aus `headerFieldSpecs` (4.1b) mit dem Pfad als Tooltip | die Pfade sind die Namen der Datei und dieselben, die Agenten und `git diff` sehen; deutsche Namen gibt es heute nur für Header |
| Subjekt | `DiffSubject` mit einheitlichem Feld `index` (Header-ID, Scriptdatei, Item-ID), dazu `member { folder, index }` (Ordner ohne Studio-Art) und `file { path }` (Projektdateien, Register, Blobs außerhalb von `unpacked/`); `ChangedEntry` (4.1b) bekommt `subject`; `Item.index` ist die Item-ID (wie in der Item-Liste der App und in `data/items/`), der Member folgt über `textproject::item_owners` | ein Typ für drei Listen, MCP und „Öffnen“ |
| Listen | Basis: `changed_from_original` (4.1b) mit `subject`; letzter Commit: Status-Pfade → Subjekte (`diff::head_entries`, Trainer- und Pokémon-Teile zu je einer Zeile, eine geänderte `arm9.bin` zu den abweichenden Headern plus `file`, wenn außerhalb der Tabelle etwas abweicht); gespeichert: `Store::dirty()` → Subjekte (`diff::unsaved_entries`, die Header-Tabelle je abweichendem Header) | dieselbe Zeilenform (`ChangedEntry`) in allen drei Fällen |
| Ort in der App | Technik › Änderungen: oben `Segmented` „Gegen Basis / Gegen letzten Commit / Gegen gespeichert“; Liste links, Detail rechts (Containerbreite ≥ 760 px, sonst untereinander); Start „Gegen Basis“, nach „Git einrichten“ „Gegen letzten Commit“; die Wahl hält die App für die Sitzung | Entscheidung „Ort in der App“ (C §6.2); der Verlauf im Scharnier bleibt die Undo-Liste |
| „Öffnen“ | über `NavTarget` (`workbench/navigation.ts`): Header, Gebietsdaten, Textarchiv (erste geänderte Nachricht), Scriptdatei (erste geänderte Zeile), Pokémon, Attacke, Item, Trainer, Begegnungsdatei, Matrix (Zelle 0, 0), Eventdatei, wenn der erste geänderte Pfad in einer Event-Liste liegt; sonst gesperrt mit „Für diese Art gibt es keinen Ort in der App.“ | dieselben Ziele wie die Sprünge aus den Problemen |
| Orte von „Git einrichten…“ | Technik › Änderungen („Gegen letzten Commit“ ohne Repo), Palette (nur ohne Repo), Häkchen „Git einrichten (nur lokal)“ im Assistenten „Neues Spiel aus ROM“ (aus als Standard, Phase `git` nach `text_project`), MCP `git_init`, `g4k git init` | Gutachten C §6.4 und §11 |
| Klon eines Textprojekts | **in 4.4**: `textproject::restore::restore_from_base` (Basis-ROM nach `.g4/tmp/` auspacken, fehlende Einträge in die Wurzel verschieben, nie eine vorhandene Datei überschreiben, `import`, `sync::record_all`), `g4k text restore`, und beim Öffnen eines Ordners mit `g4-project.json` ohne `header.yaml` der Dialog „Textprojekt vervollständigen?“ | Entscheidung T2 „nach einem Klon entsteht der DSPRE-Teil aus der eigenen ROM“; 4.3a hat nur den Testweg (`textproject_from_base`), ohne ihn wäre ein Textprojekt in Git als Sicherung nicht zu öffnen |
| MCP | `git_status` (Read), `git_init` (`Irreversible("Git einrichten")`, Rückfrage, nicht im Batch, kopflos nur mit `--allow-irreversible`), `diff_entity` (Read, `data_note!()`); Befehle ohne Werkzeug: `unsaved_changes` (Read), `restore_text_project` (App-only) | Vorgabe der Teilphase; der Klon wird vor dem Öffnen vervollständigt, ein Agent hat dann noch kein Projekt |
| Kopflos | `g4k git templates <root>`, `g4k git init <root> [--templates-only]`, `g4k git status <root> [--entries]`, `g4k diff <root> <art> <index|id> [--against base|head] [--base-rom <nds>] [--json]`, `g4k text restore <root> [--base-rom <nds>]` | Gate (2) ohne App |
| Debug-Profil | `[profile.dev.package.<gix-Crate>] opt-level = 3` für die Crates, die beim Status hashen und entpacken (`gix-hash`, `gix-features`, `gix-index`, `gix-status`, `zlib-rs`; Namen mit `cargo tree` prüfen) | wie `xxhash-rust`: der Debug-Build ist sonst um ein Vielfaches langsamer |

## Global Constraints

- Commits enden mit `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>`; vor jedem cargo-Aufruf `export PATH="$HOME/.cargo/bin:/opt/homebrew/opt/rustup/bin:$PATH"`.
- Originale unter `/Users/POLTEP/Projekte/lab/editor/Roms` nie von Hand öffnen oder ändern; Testprojekte nur als frische Kopien unter `Roms/_testkopien/4-4-<zweck>` (Ordner `4-4-<zweck>_DSPRE_contents` und `4-4-<zweck>.nds` direkt nebeneinander, `cp -cR` bzw. `cp -c`). Gated Tests lesen `G4_TEST_PROJECT` (von `scripts/gate.sh` je Projekt gesetzt), schreiben nur auf `vanilla_copy()` und finden die Basis-ROM über `test_support::vanilla_base_rom()`, nur lesend. Aus dem Worktree `G4_ROMS_DIR` absolut und den `tools`-Symlink setzen (nie einchecken); `target/agent/gate-*` vor `git worktree remove` sichern.
- Keine ROM-Daten ins Repo: keine `.nds`, keine Member, keine exportierte JSON eines Spiels, keine Repos aus Testläufen; Fixtures nur synthetisch.
- Kein Remote und kein Push, nirgends: die App ruft nur die Unterbefehle aus `git::cli::Sub`; Tests legen Repos nur in Temp-Ordnern und Kopien an, klonen über `git init` + `git fetch <pfad>` + `git checkout FETCH_HEAD` (kein `git clone`, das ein `origin` einträgt) und prüfen, dass `.git/config` kein `[remote` enthält.
- Tests, die die `git`-CLI brauchen, laufen nur, wenn `find_git()` etwas findet (sonst `eprintln!("skipped: git not found")`), und mit eigener Umgebung über `GitCli::env` (`GIT_CONFIG_GLOBAL` auf eine leere Temp-Datei, `GIT_CONFIG_NOSYSTEM=1`, `HOME` auf den Temp-Ordner); nie `std::env::set_var`. Lesende Tests brauchen kein `git` (Repos über `test_support::GitFixture`, nur gix).
- Nicht in 4.4 (Entscheidung „Nicht in Phase 4“, wörtlich): keine Commit-Oberfläche, kein Staging, keine Branches, kein Merge und keine Konflikte in Git, kein Pull, kein Push, keine Remotes, kein „Auf GitHub veröffentlichen“, kein „Stand sichern“, kein Git LFS, keine Plakette „geändert“ im Inspector-Kopf, kein Bildvergleich von Binärteilen; der Verlauf im Scharnier bleibt die Undo-Liste und wird nicht mit Git vermischt.
- Fehlende Basis-ROM sperrt weder Status noch „Gegen letzten Commit“ noch „Gegen gespeichert“ noch „Git einrichten“; nur „Gegen Basis“ und das Vervollständigen nennen den Grund (`base::missing_text`, `patch::NO_MARK`).
- Gate-Zahlen: nach „Git einrichten“ und nach `g4k build` ist `git status` leer (DSPRE-Layout und Textprojekt, alle drei Spiele); ein Klon mit `core.autocrlf=true` baut byte-gleich; `.git` gepackt im DSPRE-Layout ≤ 65 MiB, im Textprojekt ≤ 10 MiB (gemessen und protokolliert, C §2.2 und §3.2); Status warm ≤ 0,5 s (protokolliert, `G4_TIMING_STRICT` prüft).
- UI: nur Bausteine und Tokens aus `apps/studio/src/ui/`, keine Hex-Farben, keine Inline-Styles (`bash scripts/check-styles.sh`, `bash scripts/check-tokens.sh`), deutsche Sätze, Bezeichner englisch; höchstens eine primäre Taste je Dialog und Karte.
- In `dispatch.rs` (`COMMANDS`, Arme, Beispielargumente), `lib.rs` (`generate_handler!`), `access.rs` (`COMMAND_KINDS`), `catalog.rs`, `tools.rs` (`NOT_IN_BATCH`), `mcp/symbolic.rs` (`SYMBOLIC_ARGS`) und `crates/g4-kernel/src/lib.rs` nur anhängen.
- Vor jedem Commit: `cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, die Tests der berührten Crates; bei Frontend-Änderungen zusätzlich `cd apps/studio && pnpm test && pnpm build`, `bash scripts/check-styles.sh`, `bash scripts/check-tokens.sh`; nach einer Änderung an `Cargo.toml` oder `Cargo.lock` zusätzlich `cargo deny --locked check licenses advisories bans`.

## Review Focus

- Das Projekt liegt in einem größeren Repo (Dotfiles im Home-Ordner, ein Monorepo): der Status liest relativ zum Projekt und zeigt nur dessen Pfade, die App bietet „Git einrichten…“ nicht an, `git_init` schreibt nur die Vorlagen und legt kein Repo im Repo an (Task 1 `an_enclosing_repo_is_read_relative`, Task 3 `in_an_enclosing_repo_init_writes_templates_only`).
- Der Nutzer hat schon eine `.gitignore`/`.gitattributes` mit eigenen Zeilen (etwa `* text=auto`): seine Zeilen bleiben byte-gleich, der Block kommt einmal ans Ende, ein zweites Schreiben ändert nichts, ein kaputter Block wird abgelehnt (Task 2 `an_existing_file_keeps_its_lines_and_gets_the_block_once`, `a_broken_block_is_refused_and_nothing_written`).
- Git ist installiert, aber der Commit scheitert (Signatur ohne Schlüssel, Hook): kein Hängen, Repository und Vorlagen bleiben, der Grund steht mit dem Text von Git in der Antwort und im Dialog (Task 3 `a_failing_commit_reports_git_stderr_and_keeps_the_repo`, Task 8 `a_failed_commit_shows_the_reason_and_keeps_the_dialog`).
- Ein Repo ohne Commit (`git init` von Hand) oder ein losgelöster `HEAD`: die Statusleiste sagt „noch kein Commit“ bzw. „(losgelöst)“, „Gegen letzten Commit“ antwortet `no_commit` statt eines Fehlers (Task 1 `an_unborn_head_reads_as_no_commit`, `a_detached_head_names_no_branch`; Task 5 `no_repo_and_no_commit_are_refused_with_their_sentence`).
- Seit dem Commit ist eine Entität neu (unverfolgte Datei) oder gelöscht: die Liste zeigt „Neu“ bzw. „Gelöscht“, das Detail alle Felder als „— → Wert“ bzw. „Wert → —“, „Auf Original zurücksetzen…“ ist für die neue gesperrt mit „Kein Original“ (Task 5 `a_new_and_a_deleted_entity_diff_against_head`, Task 6 `head_entries_name_new_and_deleted`).

---

### Task 1: `gix`, Lizenzprüfung und Lesen des Repos

**Files:**
- Modify: `Cargo.toml` (Debug-Profil der gix-Crates), `crates/g4-kernel/Cargo.toml` (`gix`), `deny.toml` (`[bans] deny`), `crates/g4-kernel/src/error.rs` (`KernelError::Git(String)`, Anzeige `Git: {0}`), `crates/g4-kernel/src/lib.rs` (`pub mod git;` am Ende), `crates/g4-kernel/src/test_support.rs` (`GitFixture`)
- Create: `crates/g4-kernel/src/git.rs` (Modul-Doku, Re-Exporte), `crates/g4-kernel/src/git/repo.rs` (+ Tests im Modul)

**Interfaces:**
- Produces:

```rust
// git/repo.rs
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RepoKind { None, Own, Enclosing { workdir: PathBuf } }
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HeadInfo { pub id: String, pub short: String, pub summary: String, pub time: i64 } // 40 hex, 7 hex, erste Zeile, Unix-Sekunden
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)] #[serde(rename_all = "snake_case")]
pub enum PathState { Modified, Added, Deleted, Untracked, TypeChange }
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StatusPath { pub path: String, pub state: PathState }   // relativ zur Projektwurzel, `/`
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GitStatus { pub repo: RepoKind, pub branch: Option<String>, pub detached: bool,
                       pub head: Option<HeadInfo>, pub paths: Vec<StatusPath>, pub ms: u64 }
pub struct GitRepo { /* gix::ThreadSafeRepository, Präfix des Projekts im Arbeitsbaum */ }
impl GitRepo {
    /// The repository whose work tree holds `root` (searching upwards); `None` without one.
    pub fn discover(root: &Path) -> Result<Option<GitRepo>, KernelError>;
    pub fn kind(&self) -> &RepoKind;
    /// `None` while HEAD is unborn.
    pub fn head(&self) -> Result<Option<HeadInfo>, KernelError>;
    /// Changes since HEAD below the project, sorted by path; untracked files that are not ignored.
    pub fn status(&self) -> Result<GitStatus, KernelError>;
    /// The bytes of `rel` (relative to the project root) in HEAD's tree; `None`: no such file or no commit.
    pub fn head_blob(&self, rel: &Path) -> Result<Option<Vec<u8>>, KernelError>;
}
/// `GitRepo::discover(root)?.status()`, or a status with `RepoKind::None` and nothing else.
pub fn status(root: &Path) -> Result<GitStatus, KernelError>;
// test_support.rs (feature test-support)
pub struct GitFixture;
impl GitFixture {
    /// An empty repository at `root`, HEAD unborn on `main`; gix only.
    pub fn init(root: &Path);
    /// Commits every file below `root` (without `.git`, ignoring nothing) on top of HEAD if any; the
    /// index then matches the commit. Returns the commit id. gix only, no git binary.
    pub fn commit_all(root: &Path, message: &str) -> String;
    /// Points HEAD at commit `id` directly (detached).
    pub fn detach(root: &Path, id: &str);
}
```

- Zustand je Pfad: Baum ↔ Index und Index ↔ Arbeitsbaum zusammen, je Pfad ein Eintrag (Arbeitsbaum gewinnt: gelöscht im Arbeitsbaum ist `Deleted`, im Index neu ist `Added`); unverfolgte Ordner werden bis zu den Dateien aufgelöst; Pfade außerhalb des Präfixes (umschließendes Repo) fallen weg.

- [ ] **Step 1: Lizenzprüfung** (eigener Schritt vor jedem Code): `gix` mit den Features unter „Entscheidungen“ eintragen, `cargo update -p gix`, `cargo deny --locked check licenses advisories bans` → grün ohne neue Ausnahme; `cargo tree -i libgit2-sys` → „did not match any packages“; `deny.toml` `[bans]`: `deny = [{ crate = "git2", reason = "Entscheidung Phase 4: gix zum Lesen, git-CLI zum Schreiben" }, { crate = "libgit2-sys", reason = "GPL-2.0 mit Linking Exception im mitgebauten C-Code, unsichtbar für cargo deny" }]` (Syntax der installierten cargo-deny 0.20.2 prüfen); ein Advisory, das sich nicht durch ein Update beheben lässt, nur als begründete Ausnahme im Stil der vorhandenen. Die Zahl der neuen Crates (`cargo tree -p gix -e normal --prefix none | sort -u | wc -l`) kommt in die Commit-Botschaft.
- [ ] **Step 2: Scheiternde Tests** (`git/repo.rs`, Temp-Ordner, `GitFixture`):
  - `no_repo_is_none` (`discover` → `None`; `status(root).repo == RepoKind::None`, `paths` leer).
  - `own_repo_reads_branch_head_and_a_clean_status` (drei Dateien, `commit_all("Ausgangsstand")` → `Own`, `branch == Some("main")`, `head.summary == "Ausgangsstand"`, `short.len() == 7`, `paths` leer).
  - `modified_added_deleted_and_untracked_paths_are_listed` (`a` geändert, `b` gelöscht, `c` neu, `.gitignore` mit `d` und `d` angelegt → `[.gitignore Untracked, a Modified, b Deleted, c Untracked]`, `d` fehlt).
  - `head_blob_reads_the_committed_bytes` (nach Änderung von `a` die alten Bytes; fehlender Pfad `None`).
  - `an_enclosing_repo_is_read_relative` (Review Focus: Repo im Temp-Ordner, Projekt in `sub/proj`; `Enclosing { workdir }`; eine Änderung außerhalb von `sub/proj` fehlt; Pfade ohne `sub/proj/`; `head_blob("x")` liest `sub/proj/x`).
  - `an_unborn_head_reads_as_no_commit` (Review Focus: `GitFixture::init`, eine Datei → `head == None`, `branch == Some("main")`, die Datei `Untracked`; `head_blob` → `None`).
  - `a_detached_head_names_no_branch` (Review Focus: `detach` → `branch == None`, `detached`).
- [ ] **Step 3: Lauf** `cargo test -p g4-kernel --all-features git::repo` → FAIL (Modul fehlt).
- [ ] **Step 4: Umsetzen** wie unter Interfaces; gix-Fehler werden `KernelError::Git(<Pfad>: <Fehler>)`.
- [ ] **Step 5: Lauf** → PASS; `cargo deny --locked check licenses advisories bans` grün.
- [ ] **Step 6: Commit** `feat(git): read repository, head and status with gix, license gate without git2` (mit Trailer).

### Task 2: Vorlagen `.gitignore` und `.gitattributes`

**Files:**
- Create: `crates/g4-kernel/metadata/git/dspre.gitignore`, `crates/g4-kernel/metadata/git/text.gitignore`, `crates/g4-kernel/metadata/git/gitattributes`, `crates/g4-kernel/src/git/templates.rs` (+ Tests), `docs/formats/project_git.md` (Abschnitte „Vorlagen“, „Block“, „`.g4/`“)
- Modify: `crates/g4-kernel/src/git.rs`, `crates/g4-kernel/src/bin/g4k.rs` (`git templates`), `crates/g4-kernel/tests/cli.rs`

**Interfaces:**
- Consumes: Task 1 (`GitFixture`, `GitRepo::status` für die Ignorier-Prüfung); `build::pack_entries`; `Project::source_mode`; `textproject::sources`.
- Produces:

```rust
pub const BLOCK_START: &str = "# >>> Gen4 Studio (dieser Block wird von Gen4 Studio gepflegt) >>>";
pub const BLOCK_END: &str = "# <<< Gen4 Studio <<<";
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)] #[serde(rename_all = "snake_case")]
pub enum Layout { Dspre, TextProject }
pub fn layout_of(project: &Project) -> Layout;
/// What `build::extract_rom` writes at the root, ignored in a text project (with `unpacked`).
pub const DSPRE_PART: &[&str];
pub fn gitignore_block(project: &Project) -> Result<String, KernelError>;  // mit Markern
pub fn gitattributes_block() -> String;                                   // mit Markern
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)] #[serde(rename_all = "snake_case")]
pub enum TemplateAction { Created, Appended, Updated, Unchanged }
#[derive(Debug, Clone, PartialEq, Eq, Serialize)] pub struct TemplateFile { pub path: String, pub action: TemplateAction }
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TemplateReport { pub layout: Layout, pub files: Vec<TemplateFile>, pub ignored_archives: u32 }
/// `existing` with `block` merged in (rules under „Entscheidungen“); `Err` with the sentence for a broken block.
pub fn merge_block(existing: Option<&str>, block: &str) -> Result<(String, TemplateAction), String>;
pub fn write_templates(project: &Project) -> Result<TemplateReport, KernelError>;   // `.gitignore`, `.gitattributes`, atomar
pub const BROKEN_BLOCK: &str = "Der Block von Gen4 Studio ist unvollständig (Anfang ohne Ende); bitte von Hand reparieren."; // mit Dateiname davor
```

- Inhalt der Vorlagen wörtlich (Kommentare aus C §5, die Erlaubnisliste aus der Entscheidung):

```gitignore
# dspre.gitignore
# Gen4 Studio. Dieses Projekt enthält Daten aus dem Spiel: nur lokal oder privat versionieren, nie öffentlich.
# Zum Teilen: Technik › Veröffentlichen (Patch gegen die Basis-ROM).

# Gebaute ROMs und Patches
*.nds
*.srl
*.xdelta
/build/

# Emulator: Spielstände und Savestates
*.sav
*.dsv
*.ml[0-9]
*.mln
*.state

# Studio: nur base.json und preview.png sind Projekt, der Rest ist lokal
/.g4/*
!/.g4/base.json
!/.g4/preview.png

# rotom: Cache (die Quirks in .rotom/status/compile-state.json bleiben versioniert)
/.rotom/cache/

# Sicherungen gehören nicht in den Projektordner
*.bak
*.orig
*~

# Archive, die der Build aus unpacked/ packt
{{packed_archives}}

.DS_Store
Thumbs.db
```

  `text.gitignore`: dieselben ersten drei Gruppen mit dem Kopf „# Gen4 Studio, Textprojekt. Enthält Daten aus dem Spiel: nur lokal oder privat versionieren, nie öffentlich.“ und derselben zweiten Zeile, dann „# Spieldateien: aus der Basis-ROM und dem Textprojekt erzeugt (nach einem Klon: „Textprojekt vervollständigen“)“ mit `{{dspre_part}}`, dann „# rotom: nur die Befehlsdatenbank ist Projekt (die Quirks stehen in data/rotom-quirks.json)“ mit `/.rotom/*` und `!/.rotom/command_database`, dann „# Sicherungen neben data/ würden beim Export Blobs unter data/raw/: nicht im Projektordner sichern“ mit `*.bak`, `*.orig`, `*~`, `/*.prev`, `/*.prev/`, dann `.DS_Store`, `Thumbs.db`. `gitattributes`:

```gitattributes
# Spieldaten sind binär, auch wenn Git sie für Text hält (pokeHeight, files/data);
# ohne diese Zeilen schreibt core.autocrlf=true sie beim Auschecken um, und die ROM ändert sich.
* -text
*.json text eol=lf
*.rotom text eol=lf
*.yaml text eol=lf
*.yml text eol=lf
*.toml text eol=lf
*.md text eol=lf
# Platz für große Binärquellen (Phase 6), etwa: *.glb filter=lfs diff=lfs merge=lfs -text
```

- `{{packed_archives}}`: je Pack-Eintrag mit vorhandenem `unpacked/<Ordner>/` eine Zeile `/<NARC-Pfad mit />`, sortiert; `{{dspre_part}}`: `/<name>/` für Ordner, `/<name>` für Dateien aus `DSPRE_PART`, dann `/unpacked/`.

- [ ] **Step 1: Scheiternde Tests** (`templates.rs`; synthetische Projekte und `TextFixture` aus 4.3b):
  - `merge_creates_appends_updates_and_keeps_user_lines` (rein: `None` → `Created`; Text ohne Block → `Appended`, Text davor byte-gleich, eine Leerzeile, Block am Ende; anderer Block → `Updated`; gleicher Block → `Unchanged`).
  - `an_existing_file_keeps_its_lines_and_gets_the_block_once` (Review Focus: `.gitattributes` mit `* text=auto` und `.gitignore` mit `notes/`; zweimal `write_templates` → zweites Mal beide `Unchanged`, die Zeilen des Nutzers am Anfang byte-gleich).
  - `a_broken_block_is_refused_and_nothing_written` (Review Focus: nur `BLOCK_START` → `Err` mit `.gitignore: ` + `BROKEN_BLOCK`, Datei unverändert).
  - `dspre_block_lists_packed_archives_only_with_their_unpacked_folder` (ein `unpacked/<Ordner>` entfernt → seine NARC-Zeile fehlt, `ignored_archives` eins weniger).
  - `dspre_template_leaves_no_generated_archive_untracked` (synthetisch: `write_templates`, `GitFixture::init`, `GitRepo::status` → unter den unverfolgten Pfaden kein NARC eines Pack-Eintrags, kein `.g4/build/…`, aber `.g4/base.json`, `.gitignore`, `.gitattributes` und die Dateien unter `unpacked/`).
  - `text_template_ignores_the_dspre_part_and_keeps_the_sources` (`TextFixture::exported()`; `write_templates`, `GitFixture::init`, Status → unverfolgte Pfade ⊇ `textproject::sources()`, und jeder weitere ist `.gitignore`, `.gitattributes` oder liegt unter `expanded/`).
  - `a_changed_layout_replaces_the_block` (`TextFixture::dspre()`: `write_templates` → DSPRE-Block; Export; `write_templates` → `.gitignore` `Updated`, Block enthält `/unpacked/`, keine NARC-Zeile).
  - `the_dspre_part_matches_what_extract_writes` (`build::extract_rom(synthetic_rom)` in einen Temp-Ordner: jeder Name der Wurzel steht in `DSPRE_PART`).
  - `attributes_mark_everything_binary_and_text_sources_lf` (Zeilen in der Reihenfolge oben).
  - `tests/cli.rs` `cli_git_templates` (`g4k git templates <root>` → Ausgabe `created .gitignore`, `created .gitattributes`; zweiter Lauf `unchanged …`).
- [ ] **Step 2: Lauf** `cargo test -p g4-kernel --all-features git::templates` → FAIL.
- [ ] **Step 3: Umsetzen** wie unter Interfaces; geschrieben wird über `fsutil::write_atomic`.
- [ ] **Step 4: Lauf** → PASS, dazu `cargo test -p g4-kernel --all-features --test cli cli_git_templates`.
- [ ] **Step 5: Doku** `docs/formats/project_git.md`: beide Vorlagen mit Begründung je Gruppe (C §2.1, §2.3, §5), der Block und seine Regeln, `.g4/` als Erlaubnisliste, was je Layout versioniert bleibt (DSPRE: `rotom.toml`, `.rotom/command_database`, `.rotom/status/compile-state.json`; Textprojekt: `textproject::sources()`), Hinweis H2.
- [ ] **Step 6: Commit** `feat(git): .gitignore and .gitattributes templates per layout, merged as one marked block` (mit Trailer).

### Task 3: „Git einrichten“ über die CLI, Assistent, gated auf dem DSPRE-Ordner

**Files:**
- Create: `crates/g4-kernel/src/git/cli.rs` (+ Tests), `crates/g4-kernel/tests/git.rs` (gated)
- Modify: `crates/g4-kernel/src/git.rs`, `crates/g4-kernel/src/start.rs` (`CreateOptions.git`, `Phase::Git`, `Created.git`; Test im Modul), `crates/g4-kernel/src/bin/g4k.rs` (`git init`, `git status`), `crates/g4-kernel/tests/cli.rs`, `docs/formats/project_git.md` (Abschnitt „Git einrichten“), `docs/dev-setup.md` (Git für Tests, `G4_GIT`, gated `tests/git.rs`)

**Interfaces:**
- Consumes: Task 1, 2; `Project::ensure_unpacked`, `build::pack_entries`; 4.3b `CreateOptions`, `create_project_with_options`.
- Produces:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitCli { pub path: PathBuf, pub version: String, pub env: Vec<(String, String)> } // env: nur Tests
pub fn find_git() -> Option<GitCli>;
pub const NOT_INSTALLED: &str = "Git ist nicht installiert. Die Vorlagen sind geschrieben; das Repository legst du an, sobald Git da ist (git-scm.com/downloads).";
pub const IN_REPO: &str = "Das Projekt liegt schon in einem Git-Repository; Gen4 Studio hat nur die Vorlagen ergänzt.";
pub const FIRST_COMMIT: &str = "Ausgangsstand";
pub const STUDIO_NAME: &str = "Gen4 Studio";
pub const STUDIO_EMAIL: &str = "studio@gen4.invalid";
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)] #[serde(rename_all = "snake_case")]
pub enum InitPhase { Unpack, Templates, Init, Add, Commit }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)] pub struct InitProgress { pub phase: InitPhase }
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)] pub struct InitOptions { #[serde(default)] pub templates_only: bool }
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InitReport { pub templates: TemplateReport, pub committed: bool, pub reason: Option<String>,
                        pub head: Option<HeadInfo>, pub files: u32, pub studio_identity: bool,
                        pub phase_ms: BTreeMap<InitPhase, u64>, pub ms: u64 }
pub fn git_init(project: &Project, cli: Option<&GitCli>, opts: &InitOptions,
                progress: &mut dyn FnMut(InitProgress)) -> Result<InitReport, KernelError>;
// start.rs (4.3b-Typ erweitert)
pub struct CreateOptions { pub template: Template, pub text_project: bool, #[serde(default)] pub git: bool }
pub enum Phase { Read, Extract, Template, Check, TextProject, Git }      // serde "git"
pub struct Created { /* … */ pub git: Option<InitReport> }
```

- Ablauf von `git_init`: `Unpack` (nur DSPRE-Layout: `ensure_unpacked` je Art aus `ArchiveMap::kinds()`), `Templates` (`write_templates`); ist ein Repo da (`Own`/`Enclosing`) → `committed: false`, `reason: IN_REPO`; `templates_only` → `committed: false`, `reason: None`; `cli == None` → `committed: false`, `reason: NOT_INSTALLED`; sonst `Init` (`git init -q`), `Add` (`git add -A`), `Commit` (`git var GIT_COMMITTER_IDENT` scheitert → mit Studio-Identität), danach `head` und `files` (Einträge im Index) über gix. Ein scheiternder Unterbefehl nach `Init` gibt `committed: false` mit dem Satz aus „Entscheidungen“ (stderr gekürzt auf 400 Zeichen), Zeitüberschreitung „Git hat nach 5 Minuten nicht geantwortet.“ mit demselben Rahmen. Ein Fehler vor `Init` (Vorlagen) ist `Err`.
- `create_project_with_options` mit `git`: nach `Check` und `TextProject`, vor dem `rename` des Staging-Ordners, `git_init(&project, find_git().as_ref(), &InitOptions::default(), …)` als `Phase::Git`; ohne Git bleibt es bei den Vorlagen, `Created.git.reason` sagt es.
- `g4k git init <root> [--templates-only]` druckt `committed Ausgangsstand files=<n> branch=<b>` oder `not committed: <reason>`; `g4k git status <root>` druckt `branch <b|detached> head <short|none> changes <n>` bzw. `no repository`, darunter `git <version>` bzw. `git not found` aus `find_git` (Task 6 ergänzt `--entries`).

- [ ] **Step 1: Scheiternde Tests** (`cli.rs`; die mit `git` überspringen ohne Fund; Umgebung über `GitCli::env`):
  - `without_git_only_the_templates_are_written` (läuft immer: `cli = None` → `reason == Some(NOT_INSTALLED)`, kein `.git`, beide Vorlagen da).
  - `in_an_enclosing_repo_init_writes_templates_only` (läuft immer, Review Focus: `GitFixture::commit_all` im Elternordner → `reason == Some(IN_REPO)`, kein `.git` im Projekt).
  - `the_studio_runs_only_its_own_subcommands` (läuft immer: die Argumente aller `Sub` enthalten weder `push`, `remote`, `fetch`, `pull`, `clone` noch `config`).
  - `init_commits_ausgangsstand_with_the_templates_and_no_remote` (synthetisches DSPRE-Projekt: `committed`, `head.summary == "Ausgangsstand"`, `.git/config` ohne `[remote`, `GitRepo::status().paths` leer, `files` = Zahl der Index-Einträge, der Index enthält `.gitignore` und `.gitattributes`; `phase_ms` hat alle fünf Phasen in Reihenfolge des Fortschritts).
  - `without_an_identity_the_first_commit_is_by_gen4_studio` (isolierte Umgebung → `studio_identity`, Autor des Commits über gix `Gen4 Studio <studio@gen4.invalid>`; die Konfiguration des Repos enthält keinen `user.`-Eintrag).
  - `with_an_identity_the_users_name_is_kept` (`GIT_AUTHOR_NAME`/`_EMAIL`, `GIT_COMMITTER_NAME`/`_EMAIL` in `env` → `!studio_identity`, Autor aus der Umgebung).
  - `a_failing_commit_reports_git_stderr_and_keeps_the_repo` (Review Focus: `env` `GIT_CONFIG_COUNT=2`, `GIT_CONFIG_KEY_0=commit.gpgsign`, `GIT_CONFIG_VALUE_0=true`, `GIT_CONFIG_KEY_1=gpg.program`, `GIT_CONFIG_VALUE_1=false` → `!committed`, `reason` beginnt mit „Der erste Commit ist gescheitert:“, `.git` und Vorlagen da, `head == None`).
  - `tests/cli.rs` `cli_git_status_honours_g4_git` (läuft immer; Kindprozess statt `set_var`: `g4k git status <root>` mit `G4_GIT=/nicht/da` → Zeile `git not found`; mit `G4_GIT=<gefundenes git>`, nur wenn es eines gibt → Zeile beginnt mit `git version`). `G4_GIT` gesetzt heißt: nur dieser Pfad, kein Weitersuchen.
  - `start.rs` `create_with_git_reports_the_phase_and_commits` (synthetische ROM, `git: true`; Phasen enden mit `Git`; mit Git `created.git.committed`, ohne `reason == NOT_INSTALLED`).
  - `tests/cli.rs` `cli_git_init_and_status` (`g4k git init` → `committed Ausgangsstand`, `g4k git status` → `changes 0`).
- [ ] **Step 2: Gated Tests** (`tests/git.rs`, `G4_TEST_PROJECT` und `git`; isolierte Umgebung mit gesetzter Identität):
  - `git_setup_on_the_dspre_folder_keeps_status_empty_after_build`: `vanilla_copy()`, `git_init` → `committed`, Status leer; `build::build` in `build/` → Status leer; `git repack -a -d -q`, `git count-objects -v` → `size-pack`; druckt `git <game_code> dspre files=<n> ignored_archives=<n> unpack_ms=<n> add_ms=<n> commit_ms=<n> status_ms=<n> packed_kib=<n>`; `packed_kib ≤ 65 * 1024`, `files ≤ 20_027` (C §2.2 Variante A als obere Grenze), kein Index-Pfad endet auf `.nds` oder liegt unter `.g4/` außer `base.json`/`preview.png`.
  - `no_game_data_file_falls_under_an_eol_rule`: auf der Kopie trägt keine Datei unter `files/`, `unpacked/`, `arm9/`, `arm9_overlays/`, `arm7/` eine der Endungen `.json`, `.rotom`, `.yaml`, `.yml`, `.toml`, `.md`.
  - `a_clone_with_autocrlf_builds_byte_equal_dspre`: Kopie einrichten, ROM A bauen; Klon `git init`, `git -C <klon> config core.autocrlf true`, `git -C <klon> fetch -q <kopie> HEAD`, `git -C <klon> checkout -q FETCH_HEAD`; `build::build(<klon>)` → SHA-1 = A; `GitRepo::status(<klon>)` leer; `.git/config` beider ohne `[remote`. Druckt `clone <game_code> dspre build_ms=<n> equal`.
- [ ] **Step 3: Lauf** `cargo test -p g4-kernel --all-features git::cli start::` → FAIL; **Step 4: Umsetzen**; **Step 5: Lauf** → PASS; gated einmal auf Platin DE: `G4_TEST_PROJECT="<Roms>/Pokemon - Platin-Edition (Germany)_DSPRE_contents" cargo test --release -p g4-kernel --all-features --test git -- --nocapture --test-threads 1`.
- [ ] **Step 6: Doku** `project_git.md` „Git einrichten“: Phasen, Unterbefehle, Identität, Fehler, vorhandenes Repo, kein Remote; `dev-setup.md`: `git` nur für die schreibenden Tests nötig, `G4_GIT`, die Umgebung der Tests, `tests/git.rs` in der Tabelle `G4_TEST_PROJECT`.
- [ ] **Step 7: Commit** `feat(git): set up a project with templates, git init and the first commit Ausgangsstand, never a remote` (mit Trailer).

### Task 4: Klon eines Textprojekts vervollständigen, Gate der Phase kopflos

**Files:**
- Create: `crates/g4-kernel/src/textproject/restore.rs` (+ Tests)
- Modify: `crates/g4-kernel/src/textproject/mod.rs` (`pub mod restore;`), `crates/g4-kernel/src/bin/g4k.rs` (`text restore`), `crates/g4-kernel/tests/cli.rs`, `crates/g4-kernel/tests/git.rs` (gated), `docs/formats/text_project.md` (Abschnitt „Nach einem Klon“)

**Interfaces:**
- Consumes: Task 3; `build::extract_rom`, `textproject::import`, 4.3b `sync::record_all`, `start::read_base_mark`, `BaseRom`.
- Produces:

```rust
pub const RESTORE_NEEDED: &str = "Dieser Ordner ist ein Textprojekt ohne Spieldateien, etwa ein frischer Git-Klon.";
/// `g4-project.json` is there, `header.yaml` is not.
pub fn needs_restore(root: &Path) -> bool;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)] #[serde(rename_all = "snake_case")]
pub enum RestorePhase { Extract, Merge, Import, Sync }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)] pub struct RestoreProgress { pub phase: RestorePhase, pub done: u32, pub total: u32 }
#[derive(Debug, Clone, Serialize)] pub struct RestoreReport { pub moved: u32, pub import: ImportReport, pub sync_files: u32, pub ms: u64 }
pub fn restore_from_base(root: &Path, base: &BaseRom, progress: &mut dyn FnMut(RestoreProgress)) -> Result<RestoreReport, KernelError>;
```

- Regeln: ohne `.g4/base.json` → `Refused(patch::NO_MARK)`; SHA-1 der Basis ≠ Marke → `Refused` mit dem Satz von `assign_base` für eine fremde ROM; `extract_rom` nach `.g4/tmp/restore-<pid>/`; jeder Eintrag des Extracts, den die Wurzel nicht hat, wird verschoben, Ordner werden zusammengeführt (Datei nur, wenn sie fehlt); eine vorhandene Datei wird nie überschrieben; danach `import(&Project::open(root)?, Some(base))`, `sync::record_all`, Temp-Ordner weg. Ohne `needs_restore` → `Refused("Das Projekt hat seine Spieldateien schon.")`.

- [ ] **Step 1: Scheiternde Tests** (`restore.rs`, `TextFixture::exported()`; „Klon“ = neuer Ordner mit genau `textproject::sources()` und den beiden Vorlagen):
  - `restore_rebuilds_the_dspre_part_from_the_base_and_builds_like_before` (`needs_restore` vorher wahr, nachher falsch; `build::build` beider → gleiche SHA-1).
  - `restore_never_overwrites_a_project_file` (alle Quellen danach byte-gleich).
  - `restore_refuses_another_base_rom`, `restore_without_a_mark_is_refused`, `a_dspre_folder_needs_no_restore`.
  - `tests/cli.rs` `cli_text_restore` (`g4k text restore <klon> --base-rom <synthetische ROM>` → `restored moved=<n>`).
- [ ] **Step 2: Gated Test** (`tests/git.rs`) `text_project_in_git_builds_from_base_byte_equal` (Gate der Phase 4): `vanilla_copy()`, `start::assign_base` mit `vanilla_base_rom()`, `textproject::adopt::adopt` (Zeit), `git_init` → `committed`, Status leer; Index-Pfade ⊇ `textproject::sources()`, jeder weitere ist eine Vorlage oder liegt unter `expanded/`; `build::build` → SHA-1 = Basis-ROM; Status leer; `repack` → druckt `git <game_code> text files=<n> add_ms=<n> commit_ms=<n> packed_kib=<n>`, `packed_kib ≤ 10 * 1024`; Klon wie in Task 3 mit `core.autocrlf=true`; `needs_restore(<klon>)`; `restore_from_base` (Zeit) → `build::build(<klon>)` → SHA-1 = Basis-ROM; Status des Klons leer. Druckt `clone <game_code> text restore_ms=<n> build_ms=<n> equal`.
- [ ] **Step 3: Lauf** → FAIL; **Step 4: Umsetzen**; **Step 5: Lauf** → PASS (gated einmal auf HeartGold DE).
- [ ] **Step 6: Doku** `text_project.md` „Nach einem Klon“: was fehlt, woher es kommt, die Regel „nie überschreiben“, `g4k text restore`, der Dialog der App (Task 8).
- [ ] **Step 7: Commit** `feat(textproject): complete a cloned text project from the base ROM, phase gate in git` (mit Trailer).

### Task 5: Diff einer Entität

**Files:**
- Create: `crates/g4-kernel/src/diff.rs` (Typen, `diff_entity`), `crates/g4-kernel/src/diff/subject.rs`, `crates/g4-kernel/src/diff/json.rs`, `crates/g4-kernel/src/diff/text.rs`, `crates/g4-kernel/src/diff/side.rs` (+ Tests je Datei)
- Modify: `crates/g4-kernel/src/lib.rs` (`pub mod diff;`), `crates/g4-kernel/src/project.rs` (`read_headers` teilt sich in `pub(crate) fn headers_from_table_bytes(&self, table: &[u8]) -> Result<Vec<MapHeader>, KernelError>`), `crates/g4-kernel/src/textproject/sync.rs` (`conflict_diff` ruft `diff::json::json_changes`, Ausgabe unverändert), `crates/g4-kernel/src/textproject/source/` (nur falls fehlend: `open_for_diff`, `export_unit`)

**Interfaces:**
- Consumes: Task 1 `GitRepo::head_blob`, `HeadInfo`; 4.1b `Original`, `ResetTarget`, `asset_label`; 4.3b `EntitySource`, `unit_of`, `textproject::item_owners`; `TextArchiveDoc::from_member`; `scripts::decompile_member`; `Store::{saved_bytes, current_bytes, header_table}`.
- Produces:

```rust
// diff/subject.rs
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DiffSubject {
    Map { index: u16 }, Matrix { index: u16 }, AreaData { index: u16 }, EventFile { index: u16 },
    Encounter { index: u16 }, Species { index: u16 }, Move { index: u16 }, Item { index: u16 },
    Trainer { index: u16 }, Header { index: u16 }, TextArchive { index: u16 }, Script { index: u16 },
    MapTexture { index: u16 }, BuildingTexture { index: u16 }, BuildingModel { index: u16 },
    InteriorBuildingModel { index: u16 }, Member { folder: String, index: u16 }, File { path: String },
}
impl DiffSubject {
    pub fn label(&self) -> String;      // Wörter von asset_label; „Header 0003“; „{folder} {NNNN}“; der Pfad
    pub fn group(&self) -> String;      // Gruppen von 4.1b (`changed::GROUPS`), `file` „Projektdateien“
    pub fn from_asset(project: &Project, id: AssetId) -> Option<DiffSubject>; // Teile → Species/Trainer, Item-Member → erste ID; HEADER_TABLE → None
    pub fn reset_target(&self) -> Option<ResetTarget>;
    pub fn of_path(project: &Project, rel: &str) -> Option<DiffSubject>;
}
// diff/json.rs
pub fn json_changes(old: Option<&Value>, new: Option<&Value>, cap: usize) -> (Vec<FieldChange>, u32);
pub(crate) fn normalize_parts(value: &mut Value, side_file: &mut dyn FnMut(&str) -> Result<Option<Vec<u8>>, KernelError>) -> Result<(), KernelError>;
// diff/text.rs
pub fn word_spans(old: &str, new: &str) -> (Vec<WordSpan>, Vec<WordSpan>);   // Spannen der alten und der neuen Seite
pub fn line_hunks(old: &str, new: &str, context: usize, cap: usize) -> (Vec<LineHunk>, u32);
// diff.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)] #[serde(rename_all = "snake_case")]
pub enum Against { Base, Head, Saved }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)] #[serde(rename_all = "snake_case")]
pub enum DiffState { Changed, New, Removed, Same }
#[derive(Debug, Clone, PartialEq, Eq, Serialize)] pub struct FieldChange { pub path: String, pub old: Option<String>, pub new: Option<String> }
#[derive(Debug, Clone, PartialEq, Eq, Serialize)] pub struct CellChange { pub x: u8, pub y: u8, pub old: String, pub new: String }
#[derive(Debug, Clone, PartialEq, Eq, Serialize)] pub struct PartChange { pub part: String, pub changed: bool, pub old: Option<String>, pub new: Option<String> }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)] #[serde(rename_all = "snake_case")] pub enum SpanOp { Same, Removed, Added }
#[derive(Debug, Clone, PartialEq, Eq, Serialize)] pub struct WordSpan { pub op: SpanOp, pub text: String }
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MessageChange { pub index: u32, pub state: DiffState, pub old: Vec<WordSpan>, pub new: Vec<WordSpan> }
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LineHunk { pub old_start: u32, pub new_start: u32, pub lines: Vec<(SpanOp, String)> }
#[derive(Debug, Clone, PartialEq, Eq, Serialize)] #[serde(tag = "side", rename_all = "snake_case")]
pub enum BlobSide { Bytes { size: u64, sha1: String }, Base, Absent }
#[derive(Debug, Clone, PartialEq, Eq, Serialize)] #[serde(tag = "shape", rename_all = "snake_case")]
pub enum EntityDiff {
    Fields { changes: Vec<FieldChange>, more: u32 },
    Map { parts: Vec<PartChange>, cells: Vec<CellChange>, buildings: Vec<FieldChange>, more: u32 },
    Messages { messages: Vec<MessageChange>, more: u32 },
    Lines { hunks: Vec<LineHunk>, more: u32 },
    Binary { old: BlobSide, new: BlobSide },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EntityDiffReport { pub subject: DiffSubject, pub against: Against, pub label: String,
                              pub old_label: String, pub new_label: String, pub state: DiffState,
                              pub diff: EntityDiff, pub reset: Option<ResetTarget>, pub ms: u64 }
pub struct DiffSources<'a> { pub original: Option<&'a dyn Original>, pub base_label: Option<String>, pub repo: Option<&'a GitRepo> }
pub fn diff_entity(project: &Project, store: Option<&mut Store>, sources: &DiffSources<'_>,
                   subject: &DiffSubject, against: Against) -> Result<EntityDiffReport, KernelError>;
pub const FIELD_CAP: usize = 200;
pub const LINE_CAP: usize = 400;
pub const NO_REPO: &str = "Dieses Projekt liegt in keinem Git-Repository.";
pub const NO_COMMIT: &str = "Noch kein Commit: es gibt nichts zu vergleichen.";
pub const NO_BASE_FOR_FILES: &str = "Für einzelne Dateien gibt es keinen Vergleich mit der Basis-ROM.";
pub const SAVED_NEEDS_THE_APP: &str = "Den ungespeicherten Stand kennt nur die App.";
pub const SCRIPTS_HAVE_NO_UNSAVED: &str = "Scripts speichert der Editor sofort; „Gegen gespeichert“ gibt es für sie nicht.";
```

- Seiten je Subjekt (alt = Basis, `HEAD` bzw. gespeichert; neu = Datei auf der Platte bzw. Store-Wert; `side.rs`):

| Subjekt | Basis | `HEAD` | Platte | gespeichert / ungespeichert |
|---|---|---|---|---|
| Felder-Arten (Matrix, Gebietsdaten, Eventdatei, Begegnung, Pokémon, Attacke, Item, Trainer) | Basis-Member → In-Speicher-Export | Textprojekt: Blob der Entitäts-JSON; DSPRE: Blobs der Member → Export | Textprojekt: Entitäts-JSON; DSPRE: Member → Export | `saved_bytes` / `current_bytes` → Export |
| Karte | wie Felder, Form `map` | wie Felder | wie Felder | wie Felder |
| Header | `original.header_table()[i]` → Header-Export | Textprojekt: `data/headers/NNNN.json`; DSPRE: Tabelle aus dem Blob von `arm9/arm9.bin` (Ausschnitt der `Location::Slice`) bzw. den Blobs von `unpacked/dynamicHeaders/` über `headers_from_table_bytes` | Textprojekt: Datei; DSPRE: `read_headers` | Datensatz `i` der gespeicherten / aktuellen Header-Tabelle |
| Textarchiv | `TextArchiveDoc::from_member` | Blob `expanded/textArchives/NNNN.json`, fehlt er: Blob des Members | JSON, fehlt sie: Member | Store-Bytes (kanonische JSON) |
| Scriptdatei | `decompile_member` → Quelltext; ohne Quelle `binary` | Blob `expanded/scripts/NNNN.rotom` bzw. `.json` | Datei | `Refused(SCRIPTS_HAVE_NO_UNSAVED)` |
| Texturen, Modelle | Basis-Member → `binary` | Textprojekt: Blob `data/<art>/NNNN.<ext>`, fehlt er: `BlobSide::Base`; DSPRE: Blob des Members | ebenso von der Platte | – (keine Store-Werte) |
| `member { folder, index }` | Member aus `original.narc_at(<NARC des Pack-Eintrags>)` | Blob `unpacked/<folder>/NNNN` bzw. im Textprojekt `data/raw/unpacked/<folder>/NNNN` (fehlt: `Base`) | ebenso | – |
| `file { path }` | `Refused(NO_BASE_FOR_FILES)` | Blob des Pfads | Datei | – |

  `file`: `.json` → `fields` (JSON des Dateiinhalts), `.rotom`, `.yaml`, `.yml`, `.toml`, `.md`, `.txt` → `lines`, sonst `binary`. Gegen `HEAD` ohne Repo → `Refused(NO_REPO)`, ohne Commit → `Refused(NO_COMMIT)`; gegen gespeichert ohne Store → `Refused(SAVED_NEEDS_THE_APP)`; gegen Basis ohne `original` → `Refused(<base_label oder NO_MARK>)` (die App setzt vorher den Grund ein).
- Zustand: alt fehlt, neu da → `New`; alt da, neu fehlt → `Removed`; gleich nach Normalisierung → `Same` (Form mit leeren Listen); sonst `Changed`. `old_label`: `Basis-ROM {base_label}`, `Commit {short} „{summary}“`, `Gespeichert`; `new_label`: `Projekt (gespeichert)` bzw. `Ungespeichert`.
- `json_changes`: Objekte nach Schlüssel (Reihenfolge der neuen Seite, dann entfernte der alten), Felder nach Index, Pfade wie in den Import-Fehlern (`data.party[2].species`, ohne `$`), Blätter wie unter „Entscheidungen“; ein fehlender Teilbaum wird bis zu den Blättern aufgefaltet. Karten: `permissions` → Zellen (`x` = Spalte, `y` = Zeile, Zelle `kkcc`), Teile `model`, `bdhc`, `bgs` → `PartChange` mit den ersten 8 Stellen der SHA-1, `buildings` → Felder; `source`, `buildings_tail` als Felder unter `buildings`. Textarchive: Nachrichten nach Index, `text` mit `word_spans`, `id` und übrige Felder einer Nachricht zählen nicht.

- [ ] **Step 1: Scheiternde Tests** (synthetische Projekte, `ProjectOriginal`, `GitFixture`, `TextFixture`):
  - `json.rs`: `json_changes_name_paths_with_both_values`, `arrays_compare_by_index_and_missing_items_expand`, `the_cap_counts_the_rest`, `strings_show_without_quotes_and_absent_as_none`, `binary_parts_compare_by_sha1_whatever_their_form` (`{"base_sha1"}` gegen `{"file"}` mit denselben Bytes → keine Änderung).
  - `text.rs`: `word_spans_mark_the_changed_words` („Hallo Welt“ → „Hallo schöne Welt“: neue Seite `Same "Hallo "`, `Added "schöne "`, `Same "Welt"`), `line_hunks_keep_three_lines_of_context`.
  - `subject.rs`: `subject_json_is_tagged_with_index` (`{"kind":"trainer","index":1}`, `{"kind":"member","folder":"pokeHeight","index":5}`), `subjects_from_assets` (Personal 25 → `Species{25}`, `TrainerParty 3` → `Trainer{3}`, Item-Member → erste ID), `subjects_from_paths` (DSPRE `unpacked/trainerProperties/0001` → `Trainer{1}`, `expanded/textArchives/0012.json` → `TextArchive{12}`, `expanded/scripts/0101.rotom` → `Script{101}`, `unpacked/pokeHeight/0005` → `Member`; Textprojekt `data/species/0025.json` → `Species{25}`, `data/maps/0003.nsbmd` → `Map{3}`, `data/headers/0012.json` → `Header{12}`, `data/items/0004.json` → `Item{4}`, `data/raw/unpacked/pokeHeight/0005` → `Member`, `g4-world.json` → `File`), `reset_targets_of_subjects` (`Header{3}` → `ResetTarget::Header{id:3}`, `Matrix` → `None`).
  - `diff.rs`:
    - `a_trainer_diffs_by_field_against_head_in_the_dspre_layout` (synthetisch, `commit_all`; Level des ersten Teammitglieds von Trainer 1 über den Store +1, speichern → `Fields` mit genau einem Pfad, der auf `party[0].level` endet, alt/neu als Zahlen, `old_label` beginnt mit `Commit `).
    - `the_same_trainer_diffs_against_the_base` (`ProjectOriginal` der unberührten Kopie → dieselbe Zeile, `reset == Some(Trainer{1})`).
    - `a_text_message_diffs_with_word_spans` (Nachricht 0 von Archiv 1 + „ Studio“ → `Messages` mit einem `Changed`, neue Spannen enden mit `Added " Studio"`; gegen `HEAD` und gegen Basis).
    - `a_script_diffs_by_lines` (adoptierte Fixture, eine Zeile angehängt → ein Hunk).
    - `a_map_diffs_parts_and_cells` (eine Zelle der Bewegungsrechte → `cells` mit einer Zelle, Teil `permissions` geändert, `model` gleich).
    - `a_texture_diffs_as_binary`.
    - `saved_diffs_the_unsaved_value_against_the_saved_one` (Bearbeitung ohne Speichern → gegen gespeichert eine Zeile, gegen `HEAD` `Same`).
    - `in_a_text_project_head_compares_the_json_files` (`TextFixture::exported()`, `commit_all`, `data/moves/0001.json` von Hand `power` geändert → Pfad `data.power`).
    - `a_header_diffs_field_by_field_in_both_layouts` (DSPRE: `arm9.bin`-Ausschnitt; Textprojekt: Header-Datei).
    - `a_new_and_a_deleted_entity_diff_against_head` (Review Focus: neue `data/trainers/NNNN.json` unverfolgt → `New`, alle Pfade mit `old == None`, `reset` gesperrt erst in der Oberfläche; gelöschte Begegnungsdatei → `Removed`, `new == None`).
    - `no_repo_and_no_commit_are_refused_with_their_sentence` (Review Focus), `a_file_has_no_base_diff`, `saved_without_a_store_is_refused`, `in_the_dspre_layout_register_kinds_show_fallback_ids` (Trainerklasse im DTO als `TRAINER_CLASS_NNNN`, kein Register geschrieben).
  - `textproject/sync.rs`: die vorhandenen `conflict_diff_*`-Tests unverändert grün.
- [ ] **Step 2: Lauf** `cargo test -p g4-kernel --all-features diff::` → FAIL.
- [ ] **Step 3: Umsetzen** wie unter Interfaces; Wort-Token für `word_spans`: Folgen von Buchstaben und Ziffern (Unicode), Folgen von Leerraum, jedes andere Zeichen einzeln; Steuersequenzen der Texte (`\n`, `{…}`) sind je ein Token.
- [ ] **Step 4: Lauf** → PASS; `cargo test -p g4-kernel --all-features` (auch 4.3b-Tests grün).
- [ ] **Step 5: Commit** `feat(diff): one entity field by field against the base ROM, the last commit or the saved state` (mit Trailer).

### Task 6: Listen je Vergleich, `g4k diff`, gated Diff auf den Spielen

**Files:**
- Modify: `crates/g4-kernel/src/original/changed.rs` (`ChangedEntry.subject`), `crates/g4-kernel/src/diff.rs` (`head_entries`, `unsaved_entries`), `crates/g4-kernel/src/bin/g4k.rs` (`diff`, `git status --entries`), `crates/g4-kernel/tests/cli.rs`, `crates/g4-kernel/tests/git.rs` (gated), `docs/formats/project_git.md` (Abschnitt „Diff je Entität“)

**Interfaces:**
- Consumes: Task 1, 5; 4.1b `ChangedEntry`, `ChangeState`.
- Produces:

```rust
pub struct ChangedEntry { /* 4.1b */ pub subject: DiffSubject }
/// The status paths as entities: Modified/TypeChange → Changed, Added/Untracked → New, Deleted → Missing;
/// the members of one entity make one entry; an `arm9/arm9.bin` change lists the headers that differ
/// (and `file` when bytes outside the table differ). Group order of 4.1b, `file` entries last.
pub fn head_entries(project: &Project, repo: &GitRepo, status: &GitStatus) -> Result<Vec<ChangedEntry>, KernelError>;
/// The store's dirty assets as entities (state Changed); the header table per header that differs.
pub fn unsaved_entries(store: &mut Store) -> Result<Vec<ChangedEntry>, KernelError>;
```

- `g4k diff <root> <art> <index|id> [--against base|head] [--base-rom <nds>] [--json]`: `<art>` ist das `kind` von `DiffSubject`, die ID wird über 4.3b `Resolver` aufgelöst; Standard `head`; Ausgabe je Zeile `field <pfad> <alt> -> <neu>`, `cell <x>,<y> <alt> -> <neu>`, `part <name> changed|same`, `message <i> <state> <alt> -> <neu>`, `line <op> <text>`, `binary <alt> -> <neu>`, zuletzt `state <state>`; `--json` druckt den `EntityDiffReport`. `g4k git status <root> --entries` druckt zusätzlich je Eintrag `<state> <group> <label>`.

- [ ] **Step 1: Scheiternde Tests:**
  - `changed.rs`: `the_base_list_carries_a_subject_per_entry` (Karte → `Map`, Trainer → `Trainer`, ein Ordner ohne Art → `Member`).
  - `diff.rs`: `head_entries_merge_members_of_one_entity` (Eigenschaften und Team von Trainer 1 geändert → ein Eintrag `Trainer 0001`), `head_entries_name_new_and_deleted` (Review Focus: `New` und `Missing`), `an_arm9_change_lists_the_headers_that_differ` (Header 2 im Ausschnitt geändert → `Header 0002`, kein `file`; ein Byte außerhalb → zusätzlich `file arm9/arm9.bin`), `unsaved_entries_split_the_header_table_by_header`, `project_files_go_last_under_projektdateien`.
  - `tests/cli.rs`: `cli_diff_prints_fields_against_head` (synthetisch mit `GitFixture`, kein `git` nötig: Trainer geändert → eine Zeile `field …party[0].level 5 -> 6`, `state changed`), `cli_git_status_lists_entries`.
- [ ] **Step 2: Gated Test** (`tests/git.rs`) `trainer_and_text_changes_show_with_field_names_against_head_and_base`: Kopie einrichten (`git_init`); Trainer 1 (Level des ersten Teammitglieds +1) und Nachricht 0 von Archiv 1 (+ „ Studio“) über den Store, speichern; `head_entries` enthält `Trainer 0001` und `Textarchiv 0001`; `diff_entity` gegen `HEAD`: Trainer `Fields` mit einem Pfad auf `party[0].level`, Text `Messages` mit Nachricht 0 `Changed` und `Added " Studio"`; gegen Basis (`vanilla_base_rom()`, `BaseRom::open`) dieselben Zeilen; gegen gespeichert `Same`; `original::reset` des Trainers, speichern → gegen Basis `Same`; Zeiten von Status, `head_entries`, beiden Diffs. Druckt `diff <game_code> status_ms=<n> entries_ms=<n> head_ms=<n> base_ms=<n>`; `status_ms ≤ 500` nur mit `G4_TIMING_STRICT`.
- [ ] **Step 3: Lauf** → FAIL; **Step 4: Umsetzen**; **Step 5: Lauf** → PASS (gated einmal auf HeartGold US).
- [ ] **Step 6: Doku** `project_git.md` „Diff je Entität“: die drei Vergleiche, die Tabelle der Seiten, die Formen, die Normalisierung der Teile, Ersatz-IDs im DSPRE-Layout, `g4k diff`.
- [ ] **Step 7: Commit** `feat(diff): changed entities against the last commit and the saved state, g4k diff` (mit Trailer).

### Task 7: App-Befehle, Brücke und MCP

**Files:**
- Create: `apps/studio/src-tauri/src/commands/git.rs` (+ Tests)
- Modify: `src/commands/mod.rs`, `src/commands/project.rs` (Öffnen eines Klons → `restore_needed`), `src/commands/start.rs` (`create_project` mit `git: bool`), `src/lib.rs` (Handler anhängen), `src/commands/dispatch.rs` (anhängen, Beispielargumente), `src/mcp/access.rs`, `src/mcp/catalog.rs`, `src/mcp/tools.rs` (`NOT_IN_BATCH` + `git_init`), `src/mcp/symbolic.rs` (`diff_entity` `/index` aus `/kind`), `docs/mcp.md`

**Interfaces:**
- Consumes: Task 1 bis 6; 4.1b `found_base`, `base_missing`, `BaseRom::open_unchecked`; 4.3b `symbolic::resolve_args`.
- Produces:

```rust
#[derive(Debug, Serialize)] pub struct GitCliDto { pub version: String }
#[derive(Debug, Serialize)]
pub struct GitStatusDto { pub repo: RepoKind, pub branch: Option<String>, pub detached: bool, pub head: Option<HeadInfo>,
                          pub changes: u32, pub entries: Vec<ChangedEntry>, pub cli: Option<GitCliDto>,
                          pub layout: Layout, pub templates: Vec<String>, pub ms: u64 } // templates: vorhandene Vorlagen-Dateien
pub fn git_status_impl(state: &AppState) -> Result<GitStatusDto, CmdError>;
pub fn git_init_impl(state: &AppState, opts: InitOptions, progress: &mut dyn FnMut(InitProgress)) -> Result<InitReport, CmdError>;
pub(crate) fn git_init_with(state: &AppState, cli: Option<&GitCli>, opts: InitOptions, progress: &mut dyn FnMut(InitProgress)) -> Result<InitReport, CmdError>;
pub fn diff_entity_impl(state: &AppState, subject: DiffSubject, against: Against) -> Result<EntityDiffReport, CmdError>;
pub fn unsaved_changes_impl(state: &AppState) -> Result<Vec<ChangedEntry>, CmdError>;
pub fn restore_text_project_impl(state: &AppState, root: String, nds: Option<String>,
                                 progress: &mut dyn FnMut(RestoreProgress)) -> Result<RestoreReport, CmdError>;
```

- `git_status_impl` und `diff_entity_impl` (Basis, `HEAD`) arbeiten ohne Sitzungsschloss (Projekt-`Arc` holen, freigeben); gegen gespeichert unter dem Schloss. Fehlercodes: `no_repo` (`NO_REPO`), `no_commit` (`NO_COMMIT`), `base_missing` (Grund von `found_base`), `not_diffable` (`NO_BASE_FOR_FILES`, `SCRIPTS_HAVE_NO_UNSAVED`), `unsaved` („Erst speichern: der erste Commit nimmt den gespeicherten Stand.“, vor der Frage), `restore_needed` (`RESTORE_NEEDED` + „ Basis-ROM: {label}.“), `git_failed` gibt es nicht (ein gescheiterter Commit ist `committed: false`).
- `git_init_impl` prüft `unsaved` vor der Rückfrage, ruft `git_init_with(state, find_git().as_ref(), …)`, danach `session.files_written()`. `restore_text_project_impl`: Basis über `BaseRoms::locate(<SHA-1 aus .g4/base.json>)` oder `nds` (SHA-1 geprüft, dann `register_base_rom`), `restore_from_base`, danach `open_project_impl(root)`. Tauri: `git_init` und `restore_text_project` `async` mit `spawn_blocking` und `Channel`; `git_status`, `diff_entity`, `unsaved_changes` `async` mit `spawn_blocking`; `create_project` nimmt `git`.
- Zugriff (anhängen): `("git_status", Read)`, `("git_init", Irreversible("Git einrichten"))`, `("diff_entity", Read)`, `("unsaved_changes", Read)`, `("restore_text_project", APP_ONLY)`. Katalog (englisch, drei Werkzeuge):
  - `git_status` „Git status“: „Reads the project's Git state and changes nothing: whether a repository holds the project (none, own, or an enclosing one), branch, last commit (id, summary), the number of changed paths since it and the changed entities (each with the subject diff_entity takes), the layout of the templates and whether the git command line is installed. Works without git installed.“
  - `git_init` „Set up Git“: „Sets the project up for Git, only local: writes .gitignore and .gitattributes (an existing file only gets the Gen4 Studio block), runs git init and makes a first commit "Ausgangsstand" with the git command line; it never adds a remote and never pushes. Without git installed, with templates_only, or inside an existing repository it writes the templates only (committed: false with the reason). Refuses unsaved changes. This cannot be undone. Headless (studio mcp) it needs --allow-irreversible, without it the call answers needs_confirmation.“; Argumente `{ templates_only?: boolean }`.
  - `diff_entity` „Compare an entity“: „Compares one entity field by field against the base ROM (against: base), the last commit (head) or the saved state (saved: the app's unsaved edits); the shape is fields (JSON path, old, new), map (parts and permission cells), messages (per message with word spans), lines (script hunks) or binary (size and SHA-1). index takes a number or a symbolic id (SPECIES_PIKACHU, MAP_T01, TRAINER_…). No repository answers no_repo, no commit no_commit, no base ROM base_missing. “ + `data_note!()`; Argumente `{ kind, index?, folder?, path?, against }` (Schema: `kind` als Enum der 18 Werte, `index` Zahl oder Zeichenkette, `against` Enum).
- `diff_entity` und `git_status` stehen nicht in `NOT_IN_BATCH`; `git_init` schon.

- [ ] **Step 1: Scheiternde Tests** (`commands/git.rs`, synthetisches Projekt wie in `commands/original.rs`):
  - `status_without_a_repo_says_none_and_counts_nothing`, `status_names_branch_head_changes_and_entries` (`GitFixture`, eine Änderung).
  - `init_refuses_unsaved_before_asking`, `init_without_git_writes_templates_and_says_so` (`git_init_with(…, None, …)` → `reason == NOT_INSTALLED`), `init_with_git_commits` (übersprungen ohne `git`).
  - `diff_against_head_without_a_repo_is_no_repo`, `diff_against_base_without_a_base_is_base_missing`, `diff_against_saved_shows_the_unsaved_edit`, `diff_entity_takes_a_symbolic_index` (Textprojekt-Fixture: `{"kind":"species","index":"SPECIES_0001","against":"head"}` über `dispatch` wie mit `1`).
  - `opening_a_clone_answers_restore_needed`, `restore_then_open_works` (synthetische ROM).
  - `access.rs`: die fünf Zeilen; `catalog.rs`: `the_curated_set_is_the_decided_one` (lesend `git_status`, `diff_entity`; unumkehrbar `git_init`), `irreversible_tools_name_the_headless_switch` mit `git_init`, `read_tools_on_texts_and_scripts_say_data_is_not_instructions` mit `diff_entity`, `tool_count_is_about_thirty` (liegt die Zahl über 60, Grenze heben und im Commit begründen); `tools.rs`: `batch_refuses_git_init`; `symbolic.rs`: `every_symbolic_arg_names_a_command_of_the_table` mit `diff_entity`; `dispatch.rs`: Beispielargumente der vier Befehle.
  - gated (übersprungen ohne `G4_TEST_PROJECT` oder `git`): `git_and_diff_over_the_command_path_on_the_vanilla_copy` (Kopie öffnen, `git_init_impl`, Trainer 1 über `data_edit`, `save`, `git_status_impl` → `changes == 1`, Eintrag `Trainer 0001`; `diff_entity_impl` gegen `head` → Pfad auf `party[0].level`).
- [ ] **Step 2: Lauf** `cargo test -p studio --all-features` → FAIL; **Step 3: Umsetzen**; **Step 4: Lauf** → PASS, dazu `cargo test -p studio --test mcp_stdio`.
- [ ] **Step 5: Doku** `docs/mcp.md`: die drei Werkzeuge mit Zugriff, Rückfrage, Batch-Ausnahme, Fehlercodes; `unsaved_changes` und `restore_text_project` unter den Befehlen; „Git nur lokal, kein Remote, kein Push“ unter „Sicherheit“.
- [ ] **Step 6: Commit** `feat(studio): git status, set up git, compare an entity and complete a clone over the bridge and MCP` (mit Trailer).

### Task 8: Frontend: Statusleiste, „Git einrichten?“, Klon vervollständigen, Assistent

**Files:**
- Create: `apps/studio/src/api.git.ts` (+ test), `apps/studio/src/git/useGitStatus.ts` (+ test), `apps/studio/src/git/GitInitDialog.tsx` (+ `.module.css`, test), `apps/studio/src/git/RestoreDialog.tsx` (+ test), `apps/studio/src/git/gitWords.ts` (+ test), `apps/studio/src/ui/frame/GitStatusItem.tsx` (+ `.module.css`, test)
- Modify: `apps/studio/src/api.ts` (`export * from "./api.git";`), `apps/studio/src/App.tsx` (Hook, Statusleiste `right` vor `agent`, Dialoge, Öffnen mit `restore_needed`), `apps/studio/src/app/paletteEntries.ts` (+ test: „Git einrichten…“ nur ohne Repo), `apps/studio/src/ui/primitives/icons.ts` (`git: GitBranch`), `apps/studio/src/project/api.start.ts` (`git: boolean`, `CreatePhase` + `"git"`), `apps/studio/src/project/NewProjectWizard.tsx` (+ test)

**Interfaces:**
- Consumes: Task 7 (Befehle, DTOs); 4.3b `subscribeWindowFocus`; `Dialog`, `CheckList`, `Notice`, `Button`, `Tooltip`, `Icon`, `Checkbox`; `toast`.
- Produces (TS):

```ts
export type RepoKind = { kind: "none" } | { kind: "own" } | { kind: "enclosing"; workdir: string };
export interface HeadInfo { id: string; short: string; summary: string; time: number }
export type DiffSubject =
  | { kind: "map" | "matrix" | "area_data" | "event_file" | "encounter" | "species" | "move" | "item" | "trainer" | "header" | "text_archive" | "script" | "map_texture" | "building_texture" | "building_model" | "interior_building_model"; index: number }
  | { kind: "member"; folder: string; index: number } | { kind: "file"; path: string };
// ChangedEntry (4.1b) bekommt `subject: DiffSubject`
export interface GitStatus { repo: RepoKind; branch: string | null; detached: boolean; head: HeadInfo | null; changes: number; entries: ChangedEntry[]; cli: { version: string } | null; layout: "dspre" | "text_project"; templates: string[]; ms: number }
export type InitPhase = "unpack" | "templates" | "init" | "add" | "commit";
export interface InitReport { templates: { layout: string; files: { path: string; action: string }[]; ignored_archives: number }; committed: boolean; reason: string | null; head: HeadInfo | null; files: number; studio_identity: boolean; ms: number }
export type RestorePhase = "extract" | "merge" | "import" | "sync";
export interface RestoreProgress { phase: RestorePhase; done: number; total: number }
export const gitStatus: () => Promise<GitStatus>;
export const gitInit: (opts: { templatesOnly?: boolean }, onProgress: (p: { phase: InitPhase }) => void) => Promise<InitReport>;
export const restoreTextProject: (root: string, nds: string | null, onProgress: (p: RestoreProgress) => void) => Promise<{ moved: number; ms: number }>;
// git/useGitStatus.ts
export interface GitStatusApi { status: GitStatus | null; error: string | null; refresh: () => Promise<void> }
export function useGitStatus(opts: { project: string | null; revision: number }): GitStatusApi;   // Fokus über subscribeWindowFocus, 800 ms
// git/gitWords.ts
export function changesWord(n: number): string;                  // „keine Änderung“, „1 Änderung“, „{n} Änderungen“
export function statusText(s: GitStatus): string | null;          // null ohne Repo
// ui/frame/GitStatusItem.tsx
export function GitStatusItem(props: { status: GitStatus | null; onOpen: () => void }): JSX.Element | null;
// git/GitInitDialog.tsx, git/RestoreDialog.tsx
export function GitInitDialog(props: { open: boolean; status: GitStatus | null; unsaved: boolean; onClose: () => void; onDone: (r: InitReport) => void }): JSX.Element;
export function RestoreDialog(props: { root: string; message: string; onClose: () => void; onOpened: () => void }): JSX.Element;
```

- Texte wörtlich:

| Stelle | Text |
|---|---|
| Statusleiste (`data-git-status`, `Button` quiet S mit Symbol `git`) | `{branch} · {changesWord(n)}`; ohne Commit `{branch} · noch kein Commit`; losgelöst `{short} (losgelöst) · {changesWord(n)}`; Tooltip „Git: {n} geänderte Dateien seit dem letzten Commit „{summary}“. Klick zeigt sie in Technik › Änderungen.“ (ohne Commit „Git: noch kein Commit.“); ohne Repo kein Element; Fehler „Git: nicht lesbar“ mit dem Grund als Tooltip |
| Dialog, Titel / Beschreibung | „Git einrichten?“ / „Legt in diesem Projektordner ein lokales Git-Repository an: .gitignore und .gitattributes, dann einen ersten Commit „Ausgangsstand“. Gen4 Studio richtet kein Remote ein und lädt nichts hoch.“ |
| Dialog, Hinweis (Warnung, immer) | „Dein Projekt enthält Daten aus dem Spiel. Lade es nicht öffentlich hoch; zum Teilen gibt es den Patch (Technik › Veröffentlichen).“ |
| Dialog, Prüfliste | „Alles gespeichert“ (offen: „Erst speichern“, sperrt „Einrichten“); „Git gefunden: {version}“ bzw. „Git ist nicht installiert: nur die Vorlagen werden geschrieben“; DSPRE „Ignoriert: ROMs, Patches, build/, Spielstände und die Archive, die der Build packt“, Textprojekt „Ignoriert: ROMs, Patches, build/, Spielstände und die Spieldateien (sie entstehen aus der Basis-ROM)“; je vorhandene Vorlage „{datei} vorhanden: der Block von Gen4 Studio kommt dazu“ |
| Dialog, Fortschritt (`role="status"`) | „Archive auspacken …“, „Vorlagen schreiben …“, „Repository anlegen …“, „Dateien aufnehmen … (bei DSPRE-Ordnern rund 10 s)“, „Erster Commit …“ |
| Dialog, Tasten | „Abbrechen“, „Einrichten“ (primär) bzw. ohne Git „Vorlagen schreiben“ (primär); während des Laufs keine Tasten, Escape schließt nicht |
| danach | Meldung „Git eingerichtet: {files} Dateien im ersten Commit „Ausgangsstand“.“ (mit `studio_identity` dazu „ Autor ist „Gen4 Studio“, weil Git deinen Namen noch nicht kennt.“); ohne Git „Vorlagen geschrieben. Git ist nicht installiert; das Repository legst du an, sobald Git da ist.“; `committed: false` mit `reason`: der Dialog bleibt mit `Notice` (Warnung) `reason` und nur „Schließen“ |
| Vervollständigen, Titel / Beschreibung | „Textprojekt vervollständigen?“ / `{message}` (Satz des Kernels mit Basis-ROM) + „ Gen4 Studio legt die Spieldateien aus deiner Basis-ROM an; deine Dateien bleiben, wie sie sind.“ |
| Vervollständigen, ohne gefundene Basis | `Notice` (Warnung) mit dem Grund, Taste „Basis-ROM wählen…“ (Dateidialog `.nds`); eine fremde ROM zeigt den Satz des Kernels |
| Vervollständigen, Fortschritt | „Basis-ROM auspacken …“, „Spieldateien einsetzen …“, „Textprojekt einspielen … {done} von {total}“, „Abgleich vorbereiten …“ |
| Vervollständigen, Tasten / danach | „Abbrechen“, „Vervollständigen“ (primär); das Projekt öffnet sich, Meldung „Textprojekt vervollständigt in {s} s.“ |
| Palette | „Git einrichten…“ (nur mit Projekt und ohne Repo) |
| Assistent, Häkchen (Schritt „Vorlage“, unter „Als Textprojekt einrichten“) | „Git einrichten (nur lokal)“, Hilfetext „Legt ein lokales Repository mit erstem Commit an; nichts wird hochgeladen.“; Zusammenfassung hängt „, mit Git“ an; Phasenwort „Git einrichten“ |

- [ ] **Step 1: Scheiternde Tests:** `api.git.test.ts` `git_commands_send_the_documented_arguments`; `gitWords.test.ts` `changes_have_none_singular_and_plural`, `status_text_covers_unborn_detached_and_no_repo`; `useGitStatus.test.tsx` `reads_on_open_and_after_a_revision`, `focus_reads_again_after_the_delay`, `never_two_reads_at_once`; `GitStatusItem.test.tsx` `hidden_without_a_repo`, `shows_branch_and_count_and_opens_technik`, `unborn_and_detached_read_as_such`; `GitInitDialog.test.tsx` `checklist_then_progress_in_order_then_toast`, `unsaved_locks_set_up`, `without_git_offers_templates_only`, `a_failed_commit_shows_the_reason_and_keeps_the_dialog` (Review Focus), `the_legal_note_is_always_shown`; `RestoreDialog.test.tsx` `restores_then_opens`, `without_a_base_offers_to_choose_one`; `paletteEntries.test.ts` `git_setup_only_without_a_repo`; `NewProjectWizard.test.tsx` `git_checkbox_is_off_and_passes_through_with_its_phase`; `App.test.tsx` `opening_a_clone_offers_to_complete_it`.
- [ ] **Step 2: Lauf** `cd apps/studio && pnpm test` → FAIL; **Step 3: Umsetzen** (CSS-Module nur mit Tokens); **Step 4: Lauf** → PASS, `pnpm build`, `bash scripts/check-styles.sh`, `bash scripts/check-tokens.sh` grün.
- [ ] **Step 5: Blick in der Test-Umgebung** (`docs/agent-gate.md`, „Je Lauf“; Kopie `_testkopien/4-4-ui_DSPRE_contents` mit `4-4-ui.nds`): Dialog „Git einrichten?“ vor und während des Laufs, Statusleiste mit „main · keine Änderung“ und nach einer Änderung, Dialog „Textprojekt vervollständigen?“ (Klon einer adoptierten Kopie `4-4-ui-text`), Assistent mit Häkchen; Hell, Dunkel, Kompakt; Screenshots nach `target/agent/gate-<datum>/4-4/` (nicht ins Repo).
- [ ] **Step 6: Commit** `feat(studio): git in the status line, set up git, complete a cloned text project, wizard checkbox` (mit Trailer).

### Task 9: Technik › Änderungen mit drei Vergleichen und Detail

**Files:**
- Create: `apps/studio/src/ui/primitives/ChangeTable.tsx` (+ `.module.css`, test), `apps/studio/src/technik/EntityDiff.tsx` (+ `.module.css`, test), `apps/studio/src/technik/useEntityDiff.ts` (+ test), `apps/studio/src/technik/useUnsaved.ts` (+ test), `apps/studio/src/technik/diffWords.ts` (+ test)
- Modify: `apps/studio/src/api.git.ts` (`diffEntity`, `unsavedChanges`, Typen), `apps/studio/src/technik/ChangesSurface.tsx` (+ test), `apps/studio/src/technik/TechnikArea.tsx` (+ test), `apps/studio/src/App.tsx` (Vergleichswahl, `goTo`, Git-Status und Ungespeichertes an die Fläche), `apps/studio/src/ui/primitives/index.ts`, `apps/studio/src/ui/gallery/pages/ContentPage.tsx` (Beispiel `ChangeTable`)

**Interfaces:**
- Consumes: Task 7, 8; 4.1b `useChanges`, `useReset`, `RESET_LABEL`, `headerFieldSpecs`; `NavTarget` (`workbench/navigation.ts`); `Segmented`, `Section`, `ListRow`, `Badge`, `Button`, `Notice`, `Tooltip`, `EmptyState`.
- Produces (TS):

```ts
export type Against = "base" | "head" | "saved";
export type SpanOp = "same" | "removed" | "added";
export interface WordSpan { op: SpanOp; text: string }
export interface FieldChange { path: string; old: string | null; new: string | null }
export type EntityDiff =
  | { shape: "fields"; changes: FieldChange[]; more: number }
  | { shape: "map"; parts: { part: string; changed: boolean; old: string | null; new: string | null }[]; cells: { x: number; y: number; old: string; new: string }[]; buildings: FieldChange[]; more: number }
  | { shape: "messages"; messages: { index: number; state: DiffState; old: WordSpan[]; new: WordSpan[] }[]; more: number }
  | { shape: "lines"; hunks: { old_start: number; new_start: number; lines: [SpanOp, string][] }[]; more: number }
  | { shape: "binary"; old: BlobSide; new: BlobSide };
export type DiffState = "changed" | "new" | "removed" | "same";
export type BlobSide = { side: "bytes"; size: number; sha1: string } | { side: "base" } | { side: "absent" };
export interface EntityDiffReport { subject: DiffSubject; against: Against; label: string; old_label: string; new_label: string; state: DiffState; diff: EntityDiff; reset: ResetTarget | null; ms: number }
export const diffEntity: (subject: DiffSubject, against: Against) => Promise<EntityDiffReport>;
export const unsavedChanges: () => Promise<ChangedEntry[]>;
// ui/primitives/ChangeTable.tsx
export type ChangeCell = string | null | readonly WordSpan[];
export interface ChangeTableRow { key: string; field: ReactNode; title?: string; old: ChangeCell; new: ChangeCell }
export function ChangeTable(props: { columns: readonly [string, string, string]; rows: readonly ChangeTableRow[]; ariaLabel: string; more?: string }): JSX.Element;
// technik/diffWords.ts
export function sideWords(against: Against): { old: string; new: string };      // Basis/Projekt, Commit/Projekt, Gespeichert/Ungespeichert
export function stateBadge(against: Against, state: ChangeState): { kind: BadgeKind; text: string };
export function navTargetOf(subject: DiffSubject, diff: EntityDiff | null): NavTarget | null;
// technik/useEntityDiff.ts, technik/useUnsaved.ts
export function useEntityDiff(subject: DiffSubject | null, against: Against, revision: number): { report: EntityDiffReport | null; error: string | null; busy: boolean };
export function useUnsaved(opts: { project: string | null; shown: boolean; revision: number }): { entries: ChangedEntry[] | null; error: string | null };
// technik/ChangesSurface.tsx (erweitert)
export type ChangesSurfaceProps = { /* 4.1b */ against: Against; onAgainst: (a: Against) => void; git: GitStatusApi | null;
  unsaved: ChangedEntry[] | null; onGitSetup: () => void; goTo: (t: NavTarget) => void; revision: number };
export function changesSubtitle(against: Against, changes: ChangesApi | null, status: OriginalStatus | null, git: GitStatus | null, unsaved: ChangedEntry[] | null): string;
```

- `ChangeTable`: echte `<table>` mit `<caption>` (sichtbar versteckt) aus `ariaLabel`, Spaltenköpfe aus `columns`, Feld und Werte in `code/s`; Spannen `removed` als `<del>` auf `--g4-status-fehler-flaeche`, `added` als `<ins>` auf `--g4-status-erfolg-flaeche`, beide zusätzlich unterstrichen bzw. durchgestrichen (nicht nur Farbe); `null` als „—“ mit verstecktem „fehlt“; `more` als Fußzeile.
- `navTargetOf` nach der Tabelle unter „Entscheidungen“; Event-Listen über die Schlüssel der Eventdatei-JSON aus `schema/world.rs` → `EventKind`.
- Texte wörtlich:

| Stelle | Text |
|---|---|
| Umschalter (`Segmented` S, `ariaLabel` „Vergleichen mit“, `data-against`) | „Gegen Basis“, „Gegen letzten Commit“, „Gegen gespeichert“ |
| Satz unter dem Titel | Basis wie 4.1b; Commit „Gegenüber dem letzten Commit {short} „{summary}“ auf {branch}. Gezählt wird der gespeicherte Stand.“ (losgelöst „ (losgelöst)“ statt „ auf {branch}“, umschließendes Repo davor „Repository {workdir}. “); gespeichert „Was ungespeichert vom gespeicherten Stand abweicht.“ |
| Commit ohne Repo | `Notice` (Info) „Dieses Projekt liegt in keinem Git-Repository.“, darunter „Git einrichten legt ein lokales Repository an; nur lokal oder privat, Gen4 Studio lädt nichts hoch.“ und „Git einrichten…“ (sekundär) |
| Commit ohne Commit | `Notice` (Info) „Noch kein Commit: es gibt nichts zu vergleichen.“ |
| leer | Basis „Alles wie in der Basis-ROM.“, Commit „Seit dem letzten Commit nichts geändert.“, gespeichert „Nichts ungespeichert.“ |
| Plaketten | Basis „Geändert“/„Neu“/„Fehlt“; Commit „Geändert“/„Neu“/„Gelöscht“; gespeichert „Ungespeichert“ |
| Untertitel in der Technik-Liste | Basis wie 4.1b; Commit „Kein Git“, „Noch kein Commit“, „Keine Änderung seit dem Commit“, „1 Änderung seit dem Commit“, „{n} Änderungen seit dem Commit“; gespeichert „Nichts ungespeichert“, „1 Eintrag ungespeichert“, „{n} Einträge ungespeichert“ |
| Detail ohne Auswahl (`data-entity-diff`) | „Wähle links einen Eintrag, dann steht hier, was sich geändert hat.“ |
| Detail, Kopf | Titel = `label`; Zeile „{old_label} → {new_label}“; „Öffnen“ (sekundär, gesperrt mit Tooltip „Für diese Art gibt es keinen Ort in der App.“), `RESET_LABEL` (sekundär, Sperre wie die Zeile in 4.1b, dazu „Kein Original“ bei `new`) |
| Detail, Laden / Fehler / gleich | „Lade Vergleich …“ (`role="status"`) / `Notice` (Fehler) mit `errorText(e)` / „Keine Abweichung.“ |
| Felder | `ChangeTable` „Feld“, `sideWords.old`, `sideWords.new`; `more` „und {more} weitere Felder“ |
| Karte | Abschnitt „Teile“: je Teil („Modell“, „Bewegungsrechte“, „Gebäude“, „BDHC“, „BGS“) eine Zeile mit `Badge` „geändert“ (Warnung) bzw. „gleich“; Abschnitt „Zellen der Bewegungsrechte“: `ChangeTable` „Zelle“ (`{x}, {y}`), alt, neu; Abschnitt „Gebäude“: `ChangeTable` wie Felder |
| Nachrichten | `ChangeTable` „Nachricht“ (`{index}` mit `Badge` „Neu“/„Entfernt“), alt, neu mit Spannen; `more` „und {more} weitere Nachrichten“ |
| Zeilen | je Hunk Überschrift „Zeile {old_start} → {new_start}“, darunter die Zeilen in `code/s` mit Zeichen „−“, „+“ bzw. Leerraum am Anfang und den Flächen der Spannen; `more` „und {more} weitere Zeilen“ |
| Binär | `ChangeTable` „Eigenschaft“: „Größe“ (`{n} Byte`), „SHA-1“ (8 Stellen); `base` „wie in der Basis-ROM“, `absent` `null` |

- [ ] **Step 1: Scheiternde Tests:** `ChangeTable.test.tsx` `renders_columns_rows_and_marks_spans_with_del_and_ins`, `absent_reads_as_dash_with_fehlt`; `diffWords.test.ts` `side_words_per_against`, `badges_per_against`, `nav_target_of_each_subject_kind_and_none_for_maps_and_files`; `useEntityDiff.test.tsx` `reads_on_selection_and_after_a_revision`, `drops_an_answer_for_an_older_selection`; `useUnsaved.test.tsx` `reads_when_shown_and_after_a_revision`; `EntityDiff.test.tsx` `fields_show_path_old_and_new_with_side_words`, `header_fields_show_their_german_name`, `map_shows_parts_cells_and_buildings`, `messages_show_word_spans`, `lines_show_hunks`, `binary_shows_size_and_sha1`, `open_jumps_and_is_locked_without_a_place`, `reset_opens_the_dialog_and_is_locked_for_a_new_entity`; `ChangesSurface.test.tsx` `the_switch_has_three_options_and_starts_on_basis`, `head_without_a_repo_offers_git_setup`, `head_without_a_commit_says_so`, `head_lists_entries_with_geloescht_and_neu`, `saved_lists_the_unsaved_entries`, `selecting_a_row_shows_its_diff` (bestehende 4.1b-Tests grün); `TechnikArea.test.tsx` `aenderungen_subtitle_follows_the_comparison`; `App.test.tsx` `after_git_setup_technik_compares_against_the_last_commit`, `the_status_line_opens_aenderungen_against_the_last_commit`.
- [ ] **Step 2: Lauf** → FAIL; **Step 3: Umsetzen** (CSS-Module nur mit Tokens, Container-Abfrage für die zwei Spalten); **Step 4: Lauf** → PASS, `pnpm build`, Stil- und Token-Prüfung grün.
- [ ] **Step 5: Blick in der Test-Umgebung** (wie Task 8, Kopie `4-4-ui`): Technik › Änderungen in allen drei Vergleichen mit gewähltem Trainer (Felder), Textarchiv (Wort-Diff), Karte (Zellen), Script (Zeilen), Commit ohne Repo; Hell, Dunkel, Kompakt; Komfort mit schmalem Fenster (Detail unter der Liste).
- [ ] **Step 6: Commit** `feat(technik): Änderungen against the base, the last commit or the saved state with a field diff per entity` (mit Trailer).

### Task 10: Design-System fortschreiben

**Files:**
- Create: `design-system/openpencil/beispiele/aenderungen-git.body.js`, `design-system/openpencil/NACHTRAG-4-4.md`
- Modify: `design-system/README.md` (Abschnitt „Nachtrag 4.4“ nach „Nachtrag 4.3b“), `design-system/Audit-Design-System.md` (Zeilen „Änderungstabelle“, „Änderungen (Fläche)“, „Git in der Statusleiste“, „Git einrichten“), bei Antwort von OpenPencil `design-system/Editor_DesignSystem.fig`

**Interfaces:**
- Consumes: Texte und Zustände aus Task 8 und 9, die Screenshots aus Task 8 Step 5 und Task 9 Step 5.

- [ ] **Step 1: README** „Nachtrag 4.4“ nach dem Muster von „Nachtrag 4.1b“: Quelle (`git/*`, `ui/frame/GitStatusItem.tsx`, `ui/primitives/ChangeTable.tsx`, `technik/{ChangesSurface,EntityDiff}.tsx`, `technik/{useEntityDiff,useUnsaved,diffWords}.ts`, `project/NewProjectWizard.tsx`), Plan- und Gate-Verweis (`docs/phase-4-4-gate.md`), Vergleichsbilder unter `target/agent/gate-<datum>/4-4/` (`blick-<fläche>-<zustand>-<thema>.png`), neues Symbol `git` (Phosphor `GitBranch`), keine neuen Tokens (Spannen auf `status/fehler-flaeche` und `status/erfolg-flaeche`); Komponenten „Änderungstabelle“ (drei Spalten, Spannen, fehlend, Fußzeile, nicht nur Farbe), „Git in der Statusleiste“ (alle Zustände der Tabelle aus Task 8); die Fläche „Änderungen“ mit Umschalter, Liste und Detail in allen Formen und Texten aus Task 9; die Dialoge „Git einrichten?“ und „Textprojekt vervollständigen?“; das Häkchen im Assistenten; Abweichungen: keine Commit-Oberfläche, kein Staging, keine Branches (Nicht in Phase 4), deutsche Feldnamen nur beim Header, kein Bildvergleich bei Binärteilen.
- [ ] **Step 2: Vorlage** `beispiele/aenderungen-git.body.js` nach dem Muster von `auf-original.body.js` (Bausteine aus `jsx-lib-app.js`, Farben über `v('token')`), `MODE`: `'commit'` (Frame „App: Technik, Änderungen gegen letzten Commit“: acht Technik-Einträge, „Änderungen“ gewählt, Umschalter auf „Gegen letzten Commit“, Gruppen „Trainer“ (1) und „Textarchive“ (1), „Trainer 0001“ gewählt mit drei Feldzeilen, Statusleiste „main · 2 Änderungen“), `'text'` (Frame „App: Technik, Änderung einer Nachricht“: Detail eines Textarchivs mit zwei Nachrichten und Wort-Spannen), `'einrichten'` (Frame „App: Dialog, Git einrichten“: Prüfliste mit vier Zeilen und rechtlichem Hinweis), `'brett'` (Brett „Komponenten (App): Änderungstabelle und Git“: `ChangeTable` mit Feldern, Spannen, fehlendem Wert; Statusleisten-Element in vier Zuständen); Projekt „Sinnoh-Remix“, Platin DE.
- [ ] **Step 3: Anleitung** `NACHTRAG-4-4.md` nach dem Muster von `NACHTRAG-4-1b.md`: „Die Datei wurde nicht angefasst“ (bis Step 4 gelingt); drei Frames auf „App: Texte, Technik, Dialoge“ und das Brett auf „Komponenten“, Lage (nächste freie Spalte, x = Spalte × 1520, y = Reihe × 1000, vorher `get_page_tree`), Bauen (`bash ../build-screen.sh aenderungen-git`, `MODE` je Frame, `op.mjs render`, Vergleich per `export_image` mit den Screenshots, erzeugte `.jsx` nicht einchecken), danach `fix_fig_text.mjs --rgb555` und README-Satz; was nicht gezeichnet werden muss (Fortschritt, Fehler, Vervollständigen, Häkchen, Formen Karte, Zeilen, Binär).
- [ ] **Step 4: `.fig`** nur, wenn OpenPencil antwortet (bei gesperrtem Bildschirm laufen die RPCs in Zeitüberschreitungen; bei 10 000 Knoten Einzeländerungen vermeiden und die Frames neu rendern): Frames nach der Anleitung einspielen, README-Satz „steht seit … in der `.fig`“; sonst bleibt der Satz „die `.fig` kennt das noch nicht“.
- [ ] **Step 5: Audit** wie unter Files; `bash scripts/check-tokens.sh` grün.
- [ ] **Step 6: Commit** `docs(design-system): addendum 4.4 changes against base, commit and saved state, git in the status line` (mit Trailer).

### Task 11: Gate 4.4 und Gate der Phase 4

**Files:**
- Create: `docs/phase-4-4-gate.md`
- Modify: `docs/ROADMAP.md` (Übersicht, Punkt 4.4 mit Stand, Phase 4 mit Gesamtstand), `docs/agent-gate.md` (Gate-Datei in der Liste oben, Abschnitt „Phase 4.4“ mit Befehlen, `data-*`, dem Klon-Weg ohne `git clone`), `docs/dev-setup.md` (gated `tests/git.rs` und die Studio-Tests in der Tabelle `G4_TEST_PROJECT`), `scripts/gate.sh` (nur falls `git` für die gated Tests geprüft werden soll: Schritt mit `record … "FAILED (git fehlt)"` wie bei `cargo-deny`)

**Interfaces:**
- Consumes: alles Vorige; die Ausgaben `git …`, `clone …`, `diff …` der gated Tests.

- [ ] **Step 1: Gate-Dokument** `docs/phase-4-4-gate.md` nach dem Muster von `docs/phase-4-1b-gate.md`: Kopf mit Stand, Gate-Text der Roadmap (Punkt 4.4) und Gate der Phase 4 als Zitate; Legende „Wer“ (Agent, Agent + Blick, Hand) mit Verweis auf `docs/agent-gate.md`; Namen in der Oberfläche (`[data-git-status]`, Dialogtitel „Git einrichten?“ und „Textprojekt vervollständigen?“, `[data-technik=aenderungen]`, `[data-against]`, `[data-changes-group]`, `[data-change]`, `[data-entity-diff]`, Meldungstexte); Vorbereitung (Kopien `_testkopien/4-4-{pt,hgde,hgus}` für (1), `4-4-k{pt,hgde,hgus}` (DSPRE) und `4-4-t{pt,hgde,hgus}` (Textprojekt) für (2), `4-4-web`, `4-4-mcp`, `4-4-ui` für (3), je mit `.nds` daneben; `cargo build --workspace --all-features` und Release-`g4k`; `git --version` notieren; `G4_BASE_ROM_DIR=<Roms>/_testkopien`; Worktree-Hinweise; Last mit `uptime` zu jeder Zeit).
  - **(1) Automatik:** `scripts/gate.sh` grün (mit `cargo deny … bans`); einzeln je Projekt im Release-Profil mit `--nocapture --test-threads 1`: `git_setup_on_the_dspre_folder_keeps_status_empty_after_build`, `no_game_data_file_falls_under_an_eol_rule`, `a_clone_with_autocrlf_builds_byte_equal_dspre`, `text_project_in_git_builds_from_base_byte_equal`, `trainer_and_text_changes_show_with_field_names_against_head_and_base`, der gated Studio-Test `git_and_diff_over_the_command_path_on_the_vanilla_copy`.
  - **(2) Szenario kopflos** (je Spiel, Release-`g4k` und `studio mcp --project <Kopie> --base-rom <nds> [--allow-irreversible]`): DSPRE-Kopie: `g4k base assign`; MCP `git_status` → `repo.kind == "none"`; `git_init` ohne Erlaubnis → `needs_confirmation`, mit → `committed`; `git_status` → `changes 0`; `g4k build` → `git status --porcelain` leer; MCP `data_edit` Trainer 1 (Level +1), `edit_text` Archiv 1 Nachricht 0, `save`; `git_status` → zwei Einträge; `diff_entity` `trainer`/`head`, `text_archive`/`head`, `trainer`/`base`; eine weitere Bearbeitung ohne `save` → `diff_entity` `saved`; `reset_to_original` Trainer 1, `save` → `diff_entity` `base` `same`; `git -C <kopie> remote -v` leer. Textprojekt-Kopie: `g4k base assign`, `g4k text adopt`, `g4k git init` → `committed`; `g4k build` → SHA-1 = `.nds`; `git status --porcelain` leer; Klon (`git init`, `config core.autocrlf true`, `fetch <kopie> HEAD`, `checkout FETCH_HEAD`); `g4k text restore <klon> --base-rom <nds>`; `g4k build <klon>` → SHA-1 = `.nds`; `g4k diff <kopie> species SPECIES_PIKACHU --against base` nach einer Änderung über MCP → `field personal.attack …`.
  - **(3) Browser-Teil** (Test-Umgebung, `4-4-web`, Basis zugeordnet): Technik › Änderungen „Gegen letzten Commit“ ohne Repo → „Git einrichten…“ → Prüfliste, rechtlicher Hinweis, Fortschritt, Meldung; Statusleiste „<branch> · keine Änderung“; Trainer ändern, speichern → Statusleiste „1 Änderung“, Liste „Trainer 0001“, Detail mit Feldtabelle, „Öffnen“ springt zum Trainer, „Auf Original zurücksetzen…“ öffnet den Dialog von 4.1b; Umschalter „Gegen Basis“ zeigt dieselbe Zeile, „Gegen gespeichert“ nach einer weiteren ungespeicherten Änderung; Nachricht ändern → Wort-Spannen; per Shell `git -C <kopie> commit -qam …` (Agent), Fokus im Browser (`window.dispatchEvent(new Event("focus"))`) → Statusleiste „keine Änderung“; Assistent „Neues Spiel aus ROM“ mit beiden Häkchen → Phasen „Textprojekt einrichten“, „Git einrichten“, danach Statusleiste mit Branch; Klon einer adoptierten Kopie öffnen → „Textprojekt vervollständigen?“ → das Projekt öffnet sich; MCP über die Brücke (`4-4-mcp`, Freigabe „Lesen und bearbeiten“): `mcp.sh call git_init '{}'` → Rückfrage „Git einrichten“, `mcp.sh call diff_entity '{"kind":"trainer","index":1,"against":"head"}'` ohne Rückfrage.
  - **(4) Hand und Blick beim Nutzer:** das eigene Git-Werkzeug des Nutzers (Terminal, GitHub Desktop oder ähnlich) zeigt das eingerichtete Repo mit dem Commit „Ausgangsstand“ und ohne Remote (Hand); „Git einrichten“ in der gepackten App, die aus dem Finder gestartet wurde (findet Git trotz kurzem `PATH`, Hand); Blick auf Technik › Änderungen (drei Vergleiche, Formen), die Dialoge und die Statusleiste in Hell, Dunkel, Kompakt (Screenshots aus Task 8 und 9).
  - **Gate der Phase 4** (eigener Abschnitt): je Spiel die drei Bedingungen „Projekt liegt in Git“ (Textprojekt-Kopie mit Commit „Ausgangsstand“, `git status` leer), „`g4k build` erzeugt aus dem Textprojekt ein ROM“, „ein unverändertes Projekt rebuildet byte-identisch“ (SHA-1 = Basis-ROM, auch aus dem Klon mit `core.autocrlf=true`), mit Verweis auf die Zeilen aus (1) und (2) und die Gates 4.1a bis 4.3b.
  - **Ergebnis:** Tabelle Punkt / Wer / Ergebnis / Datum und die Zahlentabelle:

    | Spiel | DSPRE: Dateien, gepackt (KiB), add (ms) | Textprojekt: Dateien, gepackt (KiB), add (ms) | Status (ms) | Diff `HEAD` / Basis (ms) | Klon vervollständigen (ms) | Build aus dem Klon = Basis | Last |
    |---|---|---|---|---|---|---|---|
    | Platin DE | | | | | | | |
    | HeartGold DE | | | | | | | |
    | HeartGold US | | | | | | | |

- [ ] **Step 2: Gate abarbeiten** (Agent: (1) bis (3)); Werte und Befunde eintragen; Befunde beheben (eigene Commits `fix(4.4): …` mit Regressionstest), bis alles grün ist.
- [ ] **Step 3: Roadmap** `docs/ROADMAP.md`: Übersicht („4.4 Gate vom Agenten bestanden <Datum>; Phase 4 vom Agenten abgeschlossen, Hand und Blick beim Nutzer offen“) und am Ende des Punkts 4.4 „**Stand <Datum>:** Gate vom Agenten bestanden: Git eingerichtet und `git status` nach dem Build leer im DSPRE-Layout und im Textprojekt auf allen drei Projekten, Klon mit `core.autocrlf=true` byte-gleich (DSPRE direkt, Textprojekt nach dem Vervollständigen), Trainer und Text mit Feldnamen gegen `HEAD`, Basis und gespeichert, `.git` gepackt <…> MiB (DSPRE) und <…> MiB (Textprojekt), Status <…> ms; `cargo deny` mit `gix` grün; Branch `phase-4-4-git-minimal`; [docs/phase-4-4-gate.md](phase-4-4-gate.md); beim Nutzer offen: eigenes Git-Werkzeug, gepackte App aus dem Finder, Blick“; im Abschnitt „Phase 4“ ein Satz zum Gate der Phase mit Verweis auf den Abschnitt im Gate-Dokument. `docs/agent-gate.md` und `docs/dev-setup.md` wie unter Files.
- [ ] **Step 4: Commit** `docs(4.4): gate of git minimal and of phase 4, roadmap status` (mit Trailer).

---

## Offen für die Umsetzung

- **gix-API der gewählten Version:** die Namen für den kombinierten Status (Baum ↔ Index und Index ↔ Arbeitsbaum, unverfolgte Dateien als Dateien), für den Branch-Namen (`head_name`, losgelöst), für `index::State::from_tree` und das Schreiben des Index in `GitFixture` mit `cargo doc -p gix --open` nachsehen; braucht `GitFixture` eine Feature nur zum Schreiben (etwa `tree-editor`), kommt sie als `dev-dependency`-Feature dazu, nicht in die normale Abhängigkeit.
- **imara-diff:** reicht die Re-Exportierung unter `gix::diff::blob` für Token-Folgen (Wörter) nicht, kommt `imara-diff` in der Version, die gix selbst zieht, als direkte Abhängigkeit dazu (Apache-2.0, in der Freigabeliste); die Tests bleiben dieselben.
- **Einzelexport im DSPRE-Layout:** ob der In-Speicher-Export von 4.3b Zustand braucht, den nur ein volles Export-Projekt hat (Item-Tabelle, Trainertext-Tabellen); `open_for_diff` lädt ihn dann einmal je Aufruf. Bleibt ein Diff im DSPRE-Layout über 0,2 s, wird `EntitySource` je Sitzung gehalten (in `Session`) und bei geänderten Registern neu gelesen.
- **Items:** ob alle Item-IDs eines Members dieselbe Datei-Form tragen; der Diff vergleicht die JSON der angefragten ID, die Liste zeigt je Member die erste ID. Stellt sich heraus, dass IDs eines Members verschiedene Felder haben, zeigt die Liste je ID einen Eintrag.
- **Event-Ziele für „Öffnen“:** die Schlüssel der Event-Listen in der Eventdatei-JSON (`schema/world.rs`) gegen `EventKind` der App abgleichen; fehlt eine Zuordnung, bleibt „Öffnen“ für diese Liste gesperrt.
- **Windows:** `find_git` mit `%ProgramFiles%\Git\cmd\git.exe` und das Verhalten mit `core.autocrlf=true` sind nur über die Klon-Probe auf macOS geprüft (kein Windows); bleibt beim Nutzer.
- **Großer Status:** liegt der Status unter Last auf HeartGold über 0,5 s, liest die App ihn bei Fensterfokus höchstens alle 2 s; das Gate nennt Zahl der Pfade und Zeit.
- **Blobs und `.gitignore`:** der Export von 4.3a nimmt Sicherungen in der Wurzel weiter als Blobs auf (H2); ob der Bereich der Blobs von Git ignorierte Pfade auslassen soll, ist eine eigene Entscheidung nach Phase 4.
- **Vertrauen in fremde Repos:** gix vertraut einem Repo, das einem anderen Nutzer gehört (externes Laufwerk, `safe.directory`), nur eingeschränkt; der Status nennt dann den Fehler von gix im Satz `Git: …`, „Git einrichten“ wird nicht angeboten.
- **Ressource `g4://changes`** (C §6.2) kommt nicht: `git_status` und `changed_from_original` geben Agenten dieselbe Sicht.
- **Werkzeugzahl:** mit 4.3b und 4.4 kann `tool_count_is_about_thirty` die Grenze 60 erreichen; dann hebt Task 7 sie mit Begründung im Commit.
- **Namen aus 4.3b:** weichen `EntitySource`, `export_unit`, `adopt`, `record_all`, `subscribeWindowFocus` oder `CreateOptions` nach dem Merge ab, folgen Interfaces und Tests den gemergten Namen; die Regeln dieses Plans bleiben.
