# Phase 3b.2: Skripte und Text (rotom, CodeMirror, rotom-lsp, chatot)

Stand: 2026-10-22. Teil 2 von Phase 3b der [Roadmap](../../ROADMAP.md). Baut auf dem Fundament auf ([Spec 3b.1](2026-10-22-phase-3b1-foundation-design.md)) und auf dem Welt-Explorer ([Spec 3a](2026-10-22-world-explorer-design.md)). Recherche-Berichte (rotom, chatot/uxie, DSPRE, Codebasis, Editor-Vergleich) liegen außerhalb des Repos in den Sitzungsnotizen; die Fakten, auf denen diese Spec steht, sind in §2 zusammengefasst.

## 1. Ziel und Umfang

Scripts und Texte entstehen komplett im Studio und wirken im Spiel; das Projekt bleibt in beide Richtungen DSPRE-kompatibel. Eine Spec, ein Plan, zwei Teile: zuerst Text, dann Scripts (die Script-Pipeline schreibt in Textarchive, also muss die Textseite stehen).

3b.2 liefert sieben Posten:

1. **Textarchive editierbar.** `AssetKind::TextArchive` wird editierbar; Quelle ist `expanded/textArchives/NNNN.json` (chatot-JSON). Speichern schreibt JSON und den Member. Panel „Text“: Archive, Nachrichten, Bearbeiten, Anfügen, Suche über alle Archive, Überlängen-Warnung, neues Archiv.
2. **Scripts übernehmen.** Expliziter Schritt (Kasten wie beim Header-Patch): `rotom.toml` und `.rotom/command_database/` anlegen, alle Member dekompilieren, Bericht. Ein von DSPRE oder rotom schon übernommenes Projekt wird ohne Nachfrage benutzt.
3. **Script-Editor.** CodeMirror 6 mit rotom-Syntax und `rotom-lsp` als Sidecar (Diagnosen, Completion mit Befehls-, Flag- und Variablennamen, Hover, Signaturhilfe, Gehe-zu, Outline). Speichern = Quelle schreiben, kompilieren, Member schreiben. Compile-Fehler im Panel „Probleme“ mit Sprung zur Zeile.
4. **Textliterale** (`Message "…"`) sind erlaubt; das betroffene Textarchiv wird nach dem Compile neu geladen.
5. **Level-Scripts.** Formular im Header-Editor plus „Als JSON öffnen“; Quelle ist rotoms JSON.
6. **Navigation.** Vom Header (Scriptdatei, Level-Script: Öffnen, Neu), vom Event (Feld „Script“: Öffnen, Neues Script), Knoten „Scripts“ im Welt-Explorer mit Volltextsuche; Scriptdatei und Level-Script als Kinder des Headers.
7. **Build.** Vor dem Packen: alle veralteten Quellen kompilieren, alle veralteten Text-JSON kodieren; ein Fehler bricht den Build mit Problemliste ab. Ein unverändertes, übernommenes Projekt baut byte-identisch (gated Test).

**Nicht in 3b.2:** visueller Event- oder Node-Editor; Flag-/Variablen-Picker-Dialoge (die Namen kommen über die Completion des LSP); DSPREs `.script`-Textformat als Editorformat (rotom liest es beim Übernehmen, editiert wird `.rotom`); Rückwärtsreferenzen „welche Scripts nutzen Nachricht N“; mehrere Sprachen in einem Archiv (angezeigt und editiert wird der Sprachschlüssel des Spiels, fremde Schlüssel bleiben erhalten); ein Diamant/Perl-Gate (der Code kennt die DP-Datenbank und den DP-Pfad, aber es gibt kein DP-Testprojekt); Umbenennen, Referenzen und Formatieren im Editor (rotom-lsp bietet sie nicht); ein eigener Text-Codec (chatot bleibt die einzige Implementierung).

**Gate.** Auf Kopien von Platin DE und HeartGold DE (unter `../Roms/_testkopien`):

- „Scripts übernehmen“ läuft durch; der Bericht nennt die Anzahl und keine Fehler (Member, die rotom nicht lesen kann, sind als Zahl erlaubt, der Test zählt sie). Ein Build ohne weitere Änderung ist byte-identisch zum ds-rom-Rebuild (gated Test, nicht von Hand).
- Auf der neuen Route aus dem 3a-Gate bekommt ein neuer NPC per „Neues Script“ ein Script mit einem Textliteral; im Spiel spricht er diesen Text. Das Textarchiv des Headers zeigt die neue Nachricht im Panel „Text“.
- Derselbe NPC gibt einmalig ein Item und sagt danach einen anderen Text (eine Flag, deren Namen die Completion vorschlägt).
- Ein Vanilla-Dialog (etwa der der Mutter) wird im Editor geändert und ist im Spiel geändert.
- Ein Level-Script (`on_load` → Script mit einer Nachricht) entsteht im Formular des Header-Editors und läuft beim Betreten der Karte.
- Ein Text wird im Panel „Text“ geändert und einer angefügt; beide erscheinen im Spiel; die Suche findet den neuen Text.
- Das Projekt öffnet weiterhin in DSPRE: Das Textarchiv zeigt die Änderung; DSPRE `main` zeigt wegen `rotom.toml` seine Skripteditoren als deaktiviert, der Avalonia-Zweig benutzt dieselben Quellen.
- `scripts/gate.sh` läuft grün gegen alle drei Projekte (mit der neuen Werkzeug-Variable `G4_ROTOM_LSP`).

## 2. Fakten

### 2.1 Pfade und Layout (an den drei Testprojekten geprüft)

| | Scripts (NARC, Member) | Text (NARC, Member) |
|---|---|---|
| Platin DE (CPUD) | `files/fielddata/script/scr_seq.narc`, 1124 | `files/msgdata/pl_msg.narc`, 724 |
| HeartGold DE (IPKD) | `files/a/0/1/2`, 965 | `files/a/0/2/7`, 829 |
| HeartGold US (IPKE) | `files/a/0/1/2`, 965 | `files/a/0/2/7`, 829 |
| Diamant/Perl | `files/fielddata/script/scr_seq_release.narc` (DSPRE `RomInfo`; **unverifiziert**, kein DP-Projekt) | `files/msgdata/msg.narc` (steht schon in `archives.yaml`) |

DSPRE legt `unpacked/scripts/NNNN` und `unpacked/textArchives/NNNN` an (vierstellig, ohne Endung, Dateiname = Member-Index, byteweise gleich dem Member). Level-Scripts liegen im selben Script-NARC (Index `levelScriptID` des Headers). Keines der drei Projekte hat heute `expanded/`, `rotom.toml` oder `.rotom/`. Platin hat zusätzlich `msgdata/msg.narc` (624 Member); DSPRE und Studio benutzen `pl_msg.narc`.

Heute kennt `archives.yaml` **keinen** Script-Eintrag: `unpacked/scripts` wird vom Build nicht gepackt. Ohne den Eintrag käme kein Script ins ROM.

### 2.2 DSPRE (Referenz, AGPL, nur Verhalten übernommen)

- Avalonia-Zweig: Beim ersten Öffnen des Skripteditors `rotom init --non-interactive` im Projektordner, dann `rotom convert` (bei `.script`) oder `rotom decompile`. Quellen `expanded/scripts/NNNN.rotom`, Level-Scripts `NNNN.json`. `rotom compile --json` beim Speichern im Editor, vor jedem ROM-Speichern oder Build, und per Dateiwächter bei externen Änderungen. Text: `chatot decode|encode -m charmap.json -d expanded/textArchives -a unpacked/textArchives --json --lang xx_YY --newer`; der Text-Editor schreibt nur die JSON, die Member entstehen beim Build (`--newer`: Quelle neuer als Ziel).
- `main`-Zweig: erkennt `rotom.toml` und deaktiviert seine eigenen Skript- und Level-Script-Editoren; schreibt `rotom.toml` nie selbst.
- Text-JSON: `{"key": u16, "messages": [{"id": "msg_NNNN_IIIII", "<lang>": "…" | ["…\\n", "…"]}]}`; Arrays an `\n`, `\r`, `\f` geteilt; fremde Sprachschlüssel derselben Nachricht bleiben erhalten; UTF-8, LF.
- Ein neues Textarchiv hängt immer ans Ende (ID = Anzahl); Nachrichten werden nur am Ende angefügt oder entfernt.
- Nebendateien, die DSPRE anlegt und die das Studio nicht anfasst: `narc-sync.json`, `dspre_labels.json`, `dspre_prefs.json`, `dspre_palette_colors.json`, `backups/`.
- Ordnernamen: `main` schreibt `unpacked/scripts` und `unpacked/textArchives` (camelCase); der Avalonia-Zweig würde `unpacked/Scripts` schreiben. Studio schreibt camelCase (wie `archives.yaml`) und liest beim Übernehmen auch den PascalCase-Ordner, wenn nur er existiert.
- Common-Scripts: Script-Nummern ≥ 2000 liegen außerhalb der Scriptdatei des Headers (Platin/HGSS: 2000–2499 gemeinsame Scripts, 3000–4999 Trainer, 7000–7999 Bodenitems, 8000–8799 versteckte Items; HGSS 9300–9700 mehrdeutig; DP ohne Tabelle). uxie löst sie auf (`script_file::resolve_script_id`, `GlobalScriptTable`).

### 2.3 rotom v0.7.0 (Tag `v0.7.0`, Commit `fb11503`, 2026-10-09, MIT)

- Zwei Crates: `rotom` (Lib + CLI) und `rotom-lsp` (nur Binary, tower-lsp 0.20, stdio). Nicht auf crates.io; als `git`-Abhängigkeit nutzbar. Hängt von `uxie = "0.9.5"` ab; Cargo vereinheitlicht das mit unserem 0.9.7 (ein frischer `cargo check -p g4-kernel` baut uxie 0.9.7 samt nitroarc hier in 15 s). uxie linkt `libnitroarc_ffi.dylib` (LGPL-3, 35 KB) dynamisch; abhängige Binaries haben keinen rpath, `rotom-lsp` braucht die dylib daneben.
- rotoms `build.rs` lädt beim **Bauen** `db-latest.zip` der scrcmd-database aus dem Netz und bettet sie ein (für `rotom init`). Cargo cacht das Build-Skript; ein erster Build braucht Netz.
- Projektmodell: `rotom.toml` (`format_version = 1`, `[project] name`, `[workspace] project_type = "dspre"`, `game_family`, `[paths] database_dir, cache_dir, status_dir, source_roots, binary_roots`, `[database] default_file`). DSPRE-Defaults: `source_roots = ["expanded/scripts"]`, `binary_roots = ["unpacked/scripts"]`. Quelle `expanded/scripts/0042.rotom` ↔ Member `unpacked/scripts/0042` (Stem, ohne Endung). `.rotom/command_database/<family>_v2.json` ist die Befehlsdatenbank (Platin 1111 Befehle, `flags` 4098, `vars` 315); `.rotom/status/compile-state.json` hält je Quelle Hashes, Status und die **Quirks** (`jump_table_end_marker_count`, `levelscript_padding`), ohne die der byte-genaue Round-Trip nicht gelingt. `rotom.toml` wird vom LSP je Datei durch Aufwärtssuche gefunden.
- Lib-API: `compile(&str, &DatabaseV2, &ConstantDb, BinaryQuirk) -> CompiledBytes` (rein im Speicher; Textliterale brauchen den Projektkontext), `compile_batch(work, CompileContext::Project(&ctx), …)`, `project::compile::{compile_project, decompile_project, decompile_project_files}`, `decompile_to_ir` + `ir_to_source`, `LevelScript::{from_json, to_json, from_bytes, to_bytes, validate}`, `compile_levelscript_json_to_bytes`, `ProjectContext::load(root, &RotomConfig)`, `project::config::{load_config, find_project_root}`. `compile_project` zeigt eine `indicatif`-Fortschrittsleiste auf stderr; wir benutzen `compile_batch` mit eigenem Fortschritt.
- Fehler: `CompileError` (serialisierbar) mit **Byte-Spans**; `SourceMap::new(source).byte_to_position(offset)` liefert Zeile (0-basiert) und UTF-16-Spalte. Warnungen `CompileWarning` (z. B. `MessageLineTooLong`).
- Level-Scripts erkennt rotom am Byte-Muster und dekompiliert sie zu JSON: `{"header_entries": [{"type": "on_load" | "on_resume" | "on_transition" | "on_var_condition", "script_id": u32}], "var_conditions": [{"var": u16, "value": u16, "script_id": u16}]}` (`on_var_condition` ohne `script_id`). Leere Member überspringt rotom; Member unter 4 Byte sind `InvalidFormat`.
- Textliterale: `Message "…"`, `Message format("…")`, `Menu("…" -> Label)` werden beim Compile im Textarchiv des Scripts (uxie `text_archive_for_script_file`, über die Header-Tabelle) gesucht oder angehängt; uxie schreibt dabei `expanded/textArchives/NNNN.json` (nur JSON-Archive, nur Anhängen oder Lücken füllen).
- rotom-lsp: ohne Argumente, stdio, kein `initializationOptions`; Capabilities: inkrementelle Synchronisation, Completion, Hover, Signaturhilfe, Definition, `documentSymbol` (mit Slot-Nummer `#N`), `codeLens`, `inlayHint`, Diagnosen (Parse + Analyse, 300 ms Debounce). Nicht: Referenzen, Umbenennen, Formatieren. Nur `.rotom`-Dateien.
- DSPRE liefert nur Linux- und Windows-Binaries. Für macOS bauen wir `rotom-lsp` selbst.

### 2.4 chatot 0.8.1 und uxie 0.9.7

- chatot ist Lib und Binary (MIT), transitiv schon im `Cargo.lock` (über uxie). Lesen in den Speicher: `decode_archive(&Charmap, &mut Read+Seek, false) -> TextArchive { key: u16, messages: Vec<String> }`. Schreiben nur dateibasiert: `encode::encode_texts(charmap, &TextSource, &BinarySource, &Settings { json, lang, newer_only, msgenc_format })`. `validate_message(Option<&Charmap>, &str) -> Vec<ErrorFormat>` prüft eine Nachricht trocken; `format::{line_is_too_long, measure_line_width, DIALOG_LINE_MAX_PX = 200}` für Überlängen. Eine eingebettete Charmap für alle Spiele und Sprachen (2876 Zeichen, 19 Steuercodes); `\n` (E000), `\r` (25BC), `\f` (25BD), Steuercodes als `{STRVAR_1, 8, 0, 0}`, `{COLOR, 0, 5}`, `{TRAINER_NAME:…}`, Sonderglyphen `[PK]`, `[MN]`.
- Verschlüsselung ist für DP, Pt und HGSS gleich; der **Key steht im Archiv-Header** und ist je Archiv verschieden; er muss mitgeführt werden (`uxie::read_text_archive_bin` wirft ihn weg, `decode_archive` nicht).
- Gemessen: alle 724 + 829 + 829 Archive der drei Projekte dekodiert und per JSON zurückkodiert → 100 % byte-identisch, 0 Warnungen, keine `\xNNNN`-Ausgabe.
- uxie: `GameLanguage::locale_key()` → `en_US`, `de_DE`, `fr_FR`, `it_IT`, `es_ES`, `ja_JP`, `ko_KR`; `Game::family()`; `script_file::resolve_script_id`, `GlobalScriptTable` (DP/Pt fest, HGSS aus `arm9.bin`); `Workspace::text_archive_for_script_file`. Kein Script-Parser, keine Befehlsdatenbank.

### 2.5 Codebasis (Stand `main` 61a5e75)

- `AssetKind` (`project.rs`) hat 11 Varianten; `TextArchive` ist read-only (nur für Ortsnamen gelesen). Editierbar im Store: `Map, Matrix, AreaData, Event, HeaderTable`; Undo ist bytebasiert je Asset; `save()` prüft Stale und schreibt über `fsutil::write_atomic`.
- `build()` ist synchron ohne Hooks: `reject_hidden_files`, Pack-Schleife über `archives.kinds()`, ds-rom. `pack_dir` bricht bei jedem Nicht-`NNNN`-Namen ab (Quellen dürfen nicht im Member-Ordner liegen). Der gated Test `vanilla_rebuild_matches_dsrom_cli` verlangt byte-identische ROMs.
- Sidecar-Muster (`export.rs`, `commands/models.rs`): `find_apicula` (Env, neben dem Binary, Debug `tools/apicula/bin/`), `run_with_timeout` (Prozessgruppe, Pipes, `kill_tree`), `tool_fingerprint`; `externalBin` in `tauri.conf.json`, Platzhalter in `src-tauri/build.rs`, `prepare-bundle.sh`, `check-bundle.sh` — alles apicula-spezifisch. One-shot; kein Prozess-Lebenszyklus in `Session`.
- Frontend: sechs feste Panels (`PanelId`, `DEFAULT_PANELS`, `LAYOUT_KEY = "g4.layout.v3"`), keine dynamischen Tabs, kein Problems-Panel (nur die `StatusBar` mit `Message`), Bearbeitung im Inspector (`InspectorModel`-Union). `terms.ts` kennt keine Terme für Script, Text, Probleme; verstreute Literale „Skriptdatei“, „Level-Skript“, „Skript“, „Textarchiv“. Event-Feld `script: u16` ist ein nacktes Zahlenfeld; `script_file_id` ist ein Pick-Feld ohne Maximum, `level_script_id` ein `u16`-Feld.
- CSP: `default-src 'self'`, `style-src 'self' 'unsafe-inline'`, `connect-src ipc: http://ipc.localhost blob:`, kein `worker-src`, kein `unsafe-eval`. Same-Origin-Vite-Worker gehen, `blob:`-Worker und CDN nicht.
- `CmdError.code` ∈ `stale | read_only | tool_missing | partial | error`; `isCmdError` in `api.ts`.

### 2.6 Editor-Vergleich (gemessen mit Vite 8.3.5, vitest 5, der echten CSP)

| | CodeMirror 6 + `@codemirror/lsp-client` 6.3 | Monaco 0.57 + eigener LSP-Client |
|---|---|---|
| Bundle (gzip) | 161 KB, ein Chunk, keine Worker | ≈ 1,35 MB plus Worker |
| CSP | 0 Verstöße | 0 Verstöße |
| jsdom | mountet ohne Stubs; LSP-Round-Trip gegen Fake-Server läuft | 3 Stubs nötig, ≈ 4,5 s Kalt-Import |
| LSP-Features fertig | Completion, Hover, Signaturhilfe, Definition, Diagnosen | alles inkl. `documentSymbol`, `codeLens`, `inlayHint` |
| fehlt | `documentSymbol`, `codeLens`, `inlayHint` (≈ 150 Zeilen eigene Erweiterungen) | sendet URIs kleingeschrieben, synchronisiert jedes Modell an jeden Server, Deep-Import braucht Vite-Alias |

`monaco-languageclient` scheidet aus (8 MB gzip, wasm, Extension-Host-Worker, im Versuch zur Laufzeit abgestürzt). Nicht geprüft: WKWebView (nur Chromium); daher ein manueller Rauchtest mit `tauri build`.

## 3. Entscheidungen im Überblick

- **DSPRE-Layout 1:1, nichts Eigenes daneben:** `rotom.toml`, `.rotom/`, `expanded/scripts/`, `expanded/textArchives/`, Member in `unpacked/`. Ein Projekt, das DSPRE mit rotom bearbeitet hat, öffnet im Studio ohne Schritt; eines, das das Studio übernommen hat, öffnet in DSPRE.
- **Quelle der Wahrheit ist der Text** (`.rotom`, Level-Script-JSON, Text-JSON). Der Member entsteht **beim Speichern** und wird beim Build nur noch nachgezogen, wenn er fehlt oder veraltet ist. DSPRE und das ROM sehen immer den letzten gespeicherten Stand.
- **rotom als Lib + `rotom-lsp` als Sidecar.** Compile und Decompile laufen in-process über rotoms Projekt-API, damit `compile-state.json` und Quirks genau so entstehen wie bei der CLI. `rotom init` ersetzt eigener Code mit einer **eingecheckten Befehlsdatenbank**: kein Netz zur Laufzeit, Compiler und LSP sehen dieselbe DB.
- **Übernehmen ist ein expliziter Schritt für das ganze Projekt**, kein Decompile je Datei.
- **CodeMirror 6 statt Monaco.** Die Roadmap-Zeile „Monaco“ wird korrigiert.
- **Scripts sind keine Store-Assets.** Ihr Undo ist das des Editors; Dirty ist Text ≠ gespeicherter Text. Textarchive dagegen sind Store-Assets mit Byte-Undo wie alle anderen.
- **Schreibweise „Script“** (wie rotom und das Gate), nicht „Skript“. Die bestehenden Literale ziehen mit.

## 4. Text

### 4.1 Asset

`AssetKind::TextArchive` wird editierbar. `Loaded::TextArchive(TextArchiveDoc)`:

```rust
pub struct TextArchiveDoc {
    pub key: u16,
    /// Locale key of the game (uxie `GameLanguage::locale_key()`), the one the editor shows.
    pub lang: String,
    pub messages: Vec<TextMessage>,
}
pub struct TextMessage {
    pub id: String,                       // "msg_0199_00012" or user-defined; kept verbatim
    pub text: String,                     // chatot text form, segments joined
    pub other: serde_json::Map<String, Value>, // foreign language keys, kept verbatim
}
```

Undo-Bytes sind das kanonische JSON (`serde_json::to_vec_pretty`, Feldreihenfolge `id`, dann die Sprachen). Beim Schreiben werden Nachrichten mit `\n`, `\r`, `\f` wie chatot in Segmente geteilt, damit DSPRE und chatot dieselbe Datei erzeugen würden.

**Ort.** Die Quelle ist `expanded/textArchives/NNNN.json`; das Asset hat zwei Dateien (Quelle und Member). **Laden:** Existiert die JSON und ist ihre mtime nicht älter als die des Members, wird sie gelesen. Sonst (keine JSON, oder Member neuer, zum Beispiel nach einer Änderung in DSPRE `main`) wird der Member mit `chatot::decode_archive` gelesen; das Asset gilt dann als „ohne Quelle“ und schreibt beim ersten Speichern die JSON. Ein Member-neuer-als-JSON-Fall wird in der Statuszeile gemeldet („Textarchiv 0199: Member war neuer, Quelle wurde daraus neu erzeugt“) — die mtime-Regel ist DSPREs `--newer`-Vertrag. **Stale-Prüfung** vergleicht beide Dateien gegen den geladenen Stand.

**Speichern.** JSON atomar über `write_atomic` (Ordner `expanded/textArchives` wird angelegt); dann der Member: `chatot::encode::encode_texts` mit `TextSource { txt: [json] }`, `BinarySource { archive: [<.g4/tmp>/NNNN] }`, `Settings { json: true, lang, newer_only: false }`, dann `rename` nach `unpacked/textArchives/NNNN`. Die JSON wird **nach** dem Member nicht noch einmal angefasst, damit ihre mtime ≥ der des Members ist. Warnungen (`validate_message`, `line_is_too_long` > 200 px) gehen als Probleme der Schwere „Warnung“ in die Antwort; sie blockieren nicht.

**Anlegen.** `Store::create_text_archive() -> u16`: nächste Nummer = Member-Anzahl; schreibt den Member (ein Archiv mit Key `0x0000`, einer Nachricht „Neuer Text“) und die JSON; sofort auf Platte, nicht rückgängig (wie `create_matrix`). `append_message(index)` ist eine normale Edit-Operation (`edit_text_archive`) und damit undo-bar. Entfernen von Nachrichten nur am Ende (wie DSPRE), kein Umsortieren.

**Suche.** `Store::text_index()` baut einmal je Session eine Liste `(archive, index, lowercase text)` über alle Archive (≈ 50 000 Nachrichten, wenige MB), invalidiert je Archiv beim Speichern und beim Neuladen. `search_text(query, limit)` liefert Treffer mit Ausschnitt.

**Benutzt von.** `text_archive_users(index) -> Vec<u16>` sind die Header mit diesem `text_archive_id` (aus der Header-Tabelle im Store).

### 4.2 Was sich am bestehenden Lesepfad ändert

`project.rs` liest Ortsnamen heute mit `uxie::read_text_archive_bin` aus dem Member. Das bleibt (Ortsnamen sind ein Vanilla-Archiv, das niemand per JSON ändert), wird aber auf den neuen Lader umgestellt, damit es nur eine Lesefunktion gibt.

## 5. Scripts

### 5.1 Kernel-Modul `scripts`

```rust
pub fn is_adopted(project: &Project) -> bool;                       // rotom.toml exists
pub fn adopt(project: &Project, progress: &dyn Fn(AdoptProgress)) -> Result<AdoptReport, KernelError>;
pub struct AdoptReport { pub decompiled: u32, pub skipped_empty: u32, pub failed: Vec<(u16, String)> }
pub enum CompileScope { File(u16), Stale }
pub fn compile(project: &Project, scope: CompileScope) -> Result<CompileReport, KernelError>;
pub struct CompileReport { pub compiled: Vec<u16>, pub problems: Vec<Problem>, pub touched_archives: Vec<u16> }
pub struct Problem { pub severity: Severity, pub file: PathBuf, pub line: u32, pub col: u32, pub message: String }
pub fn read_source(project, file: u16) -> Result<ScriptSource, KernelError>;    // Rotom(String) | LevelScript(LevelScriptDoc) | Binary
pub fn write_source(project, file: u16, source: &ScriptSource) -> Result<(), KernelError>;
pub fn new_script_file(project) -> Result<u16, KernelError>;         // next member number, "script Main #1:\n\tEnd", compiled
pub fn add_script(project, file: u16) -> Result<u16, KernelError>;   // appends "script Main #N", returns N
pub fn resolve_script(project, header: u16, n: u16) -> Result<ScriptTarget, KernelError>; // File { file, line } | Ambiguous(Vec<u16>) | Unknown
pub fn search_sources(project, query: &str, limit: usize) -> Result<Vec<SearchHit>, KernelError>;
```

- **Übernehmen.** Verweigert, wenn `rotom.toml` existiert (dann ist nichts zu tun) oder wenn `expanded/scripts/*.script` existiert („DSPRE-Textexporte gefunden; erst in DSPRE oder mit `rotom convert` übernehmen“ — rotom meldet sonst `OutputCollision`). Schreibt `rotom.toml` über `RotomConfig` (serialisiert mit `toml`): `format_version = 1`, `[project] name = <Projektordner>`, `[workspace] project_type = "dspre"`, `game_family` aus dem Spielcode, `[paths] database_dir = ".rotom/command_database"`, `cache_dir = ".rotom/cache"`, `status_dir = ".rotom/status"`, `source_roots = ["expanded/scripts"]`, `binary_roots = ["unpacked/scripts"]`, `[database] default_file = "<family>_v2.json"`. Kopiert die DB aus `crates/g4-kernel/metadata/scrcmd/` (`diamond_pearl_v2.json`, `platinum_v2.json`, `hgss_v2.json`, `include_bytes!`, dazu `SOURCE.md` mit Repo, Release und Revision). Dann `ProjectContext::load` und `decompile_project`; rotom schreibt Quellen und `compile-state.json`. Member, die rotom nicht lesen kann, bleiben binär (`ScriptSource::Binary`, im Editor als „Binär, nicht dekompilierbar“ mit Hex-Größe, nicht editierbar) und werden beim Compile übersprungen; `failed` zählt sie. Fortschritt: `AdoptProgress { done, total }` je Datei (rotoms `CompileProgress`-Callback). Läuft im `spawn_blocking` unter dem State-Lock. Liegt der Member-Ordner als `unpacked/Scripts` (PascalCase, Avalonia-DSPRE) und `unpacked/scripts` fehlt, wird er benutzt und `binary_roots` entsprechend gesetzt.
- **Compile.** `ProjectContext::load(root, config)` (gecacht in `Session`, invalidiert, wenn `rotom.toml` oder die DB sich ändern), dann `compile_batch` über die Quellen des Scope mit `CompileContext::Project`, Quirks aus dem `CompileState`. Die Member schreibt rotom direkt nach `unpacked/scripts/NNNN` (DSPRE-Vertrag; die Global-Constraint „nur `write_atomic`“ gilt für unseren Code, rotoms Schreibpfad ist hier bewusst der der CLI). Spans → `SourceMap::byte_to_position` → `Problem` mit 1-basierter Zeile und Spalte. `touched_archives`: vor dem Compile Hash jeder `expanded/textArchives/*.json`, danach Vergleich; geänderte Archive werden gemeldet, der Store lädt sie neu (`reload_from_disk`) und invalidiert den Suchindex.
- **Textliteral-Regel.** `save_script` verweigert, wenn **irgendein** Textarchiv ungespeichert ist („Textarchiv 0199 erst speichern“), weil vor dem Compile nicht feststeht, welches Archiv rotom anfasst. Die Regel ist grob, aber erklärbar und sicher.
- **Level-Scripts.** `LevelScriptDoc` ist rotoms `LevelScript` (serde, `header_entries`, `var_conditions`); Lesen/Schreiben der `NNNN.json` über `LevelScript::from_json/to_json`, Validierung über `validate` (nur die erste `on_var_condition` zählt, `var == 0` beendet die Tabelle). Ein neues Level-Script (`new_script_file` mit `kind = LevelScript`) beginnt leer (`{"header_entries": [], "var_conditions": []}`), kompiliert zu den vier Nullbytes. Ob eine Datei Script oder Level-Script ist, sagt die Endung der Quelle; beim Übernehmen entscheidet rotoms Heuristik.
- **Auflösung Event → Script.** `resolve_script(header, n)`: `n < 2000` → Scriptdatei des Headers, Zeile des `script … #n` aus dem Parse der Quelle (`Parser::new_fallible`, `documentSymbol`-Logik des LSP; fehlt `#n`, Zeile 1 mit Hinweis); `n ≥ 2000` → uxie `resolve_script_id` (Platin/HGSS); HGSS 9300–9700 → `Ambiguous`; DP → `Unknown`.
- **Suche.** Volltext über alle `.rotom`-Quellen (Dateinummer oder Text, kleingeschrieben), Treffer `(file, line, excerpt)`; Quellen werden je Aufruf gelesen (≈ 1 000 Dateien, wenige MB), kein Index.

### 5.2 Metadaten und Build-Abhängigkeit

- `g4-kernel/Cargo.toml`: `rotom = { git = "https://github.com/KalaayPT/rotom", tag = "v0.7.0" }`, `chatot = "0.8"`, `toml = "0.8"`. rotoms `build.rs` lädt beim ersten Bauen `db-latest.zip`; `docs/dev-setup.md` nennt das. Scheitert der Download ohne Netz, scheitert `cargo build` mit rotoms Fehler — akzeptiert für 3b.2; ein Fork mit eingechecktem Zip ist der Ausweg, falls das stört.
- `crates/g4-kernel/metadata/scrcmd/{diamond_pearl,platinum,hgss}_v2.json` aus `db-latest.zip` Revision 33 (die gleiche, die rotom beim Bauen einbettet), `SOURCE.md`. Test: `DatabaseV2::load` liest jede der drei Dateien, `game_family()` passt.
- `archives.yaml`: `scripts: { packed: …, unpacked: "scripts" }` je Familie (Pfade §2.1); `AssetKind::Script` store-only wie `DynamicHeaders` (`ALL` wächst auf 12; Frontend `KIND_LABELS`, `reloadPlan`, `PICK_ASSET` ziehen mit). Der Build packt den Ordner, sobald er existiert — in einem Vanilla-Projekt byte-identisch (gated Test).

## 6. Build

`build()` bekommt zwei Vorschritte zwischen `reject_hidden_files` und der Pack-Schleife, beide nur, wenn ihre Voraussetzung gilt:

1. `scripts::compile(project, CompileScope::Stale)` wenn `is_adopted`. rotoms `compile-state.json` entscheidet, was veraltet ist (Hash der Quelle, der DB, des Compilers).
2. `text::encode_stale(project)`: jede `expanded/textArchives/NNNN.json`, deren Member fehlt oder älter ist (mtime, DSPREs `--newer`), wird wie beim Speichern kodiert.

Probleme der Schwere „Fehler“ aus Schritt 1 oder 2 → `KernelError::Compile(Vec<Problem>)` → der Build packt nichts und schreibt kein ROM. Warnungen werden mit dem `BuildReport` zurückgegeben (`problems`). Der gated Test `vanilla_rebuild_matches_dsrom_cli` läuft zusätzlich einmal **nach `adopt`**: das ROM muss weiterhin byte-identisch sein. Scheitert ein einzelner Member am Round-Trip, nennt der Test ihn mit Nummer; das ist dann ein rotom-Befund, kein Grund, den Member zu überschreiben (rotom lässt nicht veränderte Quellen unangetastet, weil deren Hash im State stimmt — der Test prüft genau diesen Fall).

`build_rom` (Tauri) speichert wie heute vorher alles Dirty im Store, jetzt auch Textarchive. Offene, ungespeicherte Scripts liegen im Frontend: der Knopf „Bauen“ speichert sie zuerst über denselben Pfad wie Cmd+S (`save_script` je Dokument) und ruft erst dann `build_rom`; scheitert ein Speichern, bleibt der Build aus.

## 7. Sidecar `rotom-lsp`

- **Finden** wie apicula: Env `G4_ROTOM_LSP`, dann neben dem Studio-Binary, im Debug `tools/rotom/bin/rotom-lsp`; `libnitroarc_ffi.dylib` muss daneben liegen (rpath `@loader_path`, gesetzt in `prepare-bundle.sh` per `install_name_tool -add_rpath`). Fehlt es: `lsp_start` antwortet `tool_missing`, der Editor läuft ohne Sprachdienste, die Kopfzeile zeigt „rotom-lsp fehlt — Completion und Diagnosen aus“ mit Verweis auf `docs/dev-setup.md`.
- **Lebenszyklus** (`src-tauri/src/lsp.rs`, `Session.lsp: Option<LspProcess>`): ein Prozess je offenem Projekt, `cwd` = Projektwurzel, gestartet beim ersten `lsp_start` (das Panel „Script“ ruft es, wenn ein Projekt übernommen ist). Beendet bei Projektwechsel, `lsp_stop` und `ExitRequested` (`kill_tree`, wird `pub(crate)` in `export.rs`). Stirbt der Prozess, startet er einmal je Minute neu; danach Meldung „rotom-lsp beendet sich wiederholt“ und aus bis zum nächsten manuellen Start (Knopf in der Kopfzeile).
- **Transport:** stdio, `Content-Length`-Framing. Ein Lese-Thread schiebt jede Nachricht als String in einen `tauri::ipc::Channel<String>`, den `lsp_start(channel)` entgegennimmt; `lsp_send(text)` schreibt auf stdin (Mutex). Kein Port, keine CSP-Änderung. Zeitüberschreitungen regelt der Client (3 s Standard des `@codemirror/lsp-client`).
- **Client:** `initialize` mit `rootUri` = Projektwurzel (rotom-lsp ignoriert ihn, der Spec-konforme Wert schadet nicht), `didOpen` je Dokument mit URI `file://<root>/expanded/scripts/NNNN.rotom` (Großschreibung unverändert), inkrementelle `didChange`, `didSave` ohne Text. Server-Anfragen an den Client (`workspace/configuration`, dynamische Registrierung) beantwortet der Client mit `MethodNotFound`; rotom-lsp fällt dann auf statische Watcher-Logik zurück — das Studio sendet nach jedem eigenen Speichern eines Textarchivs oder nach `adopt` ein `workspace/didChangeWatchedFiles`, damit der LSP Nachrichten-IDs und Projektzustand neu lädt.
- **Bundle:** `externalBin` bekommt `rotom-lsp`; `src-tauri/build.rs` legt den Platzhalter an; `prepare-bundle.sh` baut `rotom-lsp` aus dem Git-Tag (`cargo install --git … --tag v0.7.0 rotom-lsp --root tools/rotom`) und kopiert die dylib; `check-bundle.sh` prüft beides. `scripts/gate.sh`: `G4_ROTOM_LSP` als Default, `env -u` im hermetischen Schritt, Preflight je Projekt.

## 8. Frontend

### 8.1 Editor

- Pakete: `codemirror` (basicSetup), `@codemirror/lang-json`, `@codemirror/lsp-client`, `@codemirror/theme-one-dark` (dunkel) und ein helles Theme aus den App-Farben. rotom-Highlighting als `StreamLanguage` (Schlüsselwörter `script`, `function`, `action`, `if`, `else`, `while`, `match`, `End`-Befehle; Befehle = großgeschriebene Bezeichner; `#N`; Zahlen, Strings, `//`-Kommentare, `#include`/`#define`); die Grammatik ist eine Datei (`workbench/script/rotomLanguage.ts`) mit Tests auf Token-Ebene.
- LSP-Erweiterungen, die der Client nicht hat (`workbench/script/lspExtras.ts`, ≈ 150 Zeilen): `documentSymbol` → Outline-Menü und Sprung zu `#N`; `inlayHint` → Dekorationen; `codeLens` → Zeile über dem Script, nur Anzeige. Transport `TauriTransport { send: invoke("lsp_send"), subscribe: Channel }`.
- Komponente `ScriptEditor` (dünn, mockbar): nimmt `doc`, `onChange`, `onSave`, `diagnostics`; CodeMirror-Instanz je Dokument wird im `ScriptDocuments`-Store gehalten (Text, `EditorState`, Version, Dirty, Cursor), damit ein Wechsel nichts verliert und das Editor-Undo je Dokument weiterlebt.

### 8.2 Panels

Drei neue feste Panels; `LAYOUT_KEY` → `g4.layout.v4`, `isSaneLayout` verlangt sie. Position: „Script“ und „Text“ als Tabs der 3D-Gruppe (`within`), „Probleme“ als Tab neben „Status“ (`within status`).

- **„Script“**: Ohne Übernahme zeigt es den Kasten **„Scripts übernehmen“** (Erklärung: legt `rotom.toml` und `.rotom/` an, dekompiliert alle N Scriptdateien nach `expanded/scripts/`, DSPRE benutzt danach dieselben Quellen und schaltet seinen alten Skripteditor ab; Knopf „Übernehmen“, Fortschrittsbalken, danach Bericht mit Fehlliste). Nach der Übernahme: Kopfzeile mit Dokument-Auswahl (offene Dokumente, Punkt für ungespeichert), Titel „Scriptdatei 0042“ bzw. „Level-Script 0042 (JSON)“, Outline-Menü, Knöpfe „Speichern“, „Script anfügen“, LSP-Status. Darunter der Editor. Cmd+S speichert das Dokument. Schließen (× in der Auswahl) eines dirty Dokuments fragt (`ask()`: Speichern / Verwerfen / Abbrechen). Nach `save_script` werden Probleme je Datei ersetzt; `touched_archives` löst `reloadPlan` für die Textarchive aus.
- **„Text“**: links Archivliste (Nummer, Nachrichtenzahl, „Header 0415, 0416“ aus `text_archive_users`), Suchfeld oben (über alle Archive, Treffer „0199 · 12 · …“, Klick wählt Archiv und Nachricht); rechts Nachrichtenliste (Index, erste Zeile, `\n`/`\r`/`\f` sichtbar), darunter ein mehrzeiliges Feld für die gewählte Nachricht (Steuercodes bleiben Text, werden eingefärbt), Überlängen-Warnung inline (Frontend ruft `validate_text(text)` mit Debounce). Knöpfe „Nachricht anfügen“, „Letzte entfernen“, „Neues Archiv“. Undo/Redo, Dirty und Speichern laufen über den Store wie bei allen Assets (Toolbar „Speichern“ speichert auch Textarchive).
- **„Probleme“**: Tabelle Schwere · Datei · Zeile · Meldung · Quelle („Compile“, „Text“, „live“). Quellen: letzter `save_script`/`build_rom` (je Datei ersetzt), Text-Warnungen beim Speichern (je Archiv ersetzt), LSP-Diagnosen des aktiven Dokuments (live, bei Dokumentwechsel ersetzt). Klick öffnet die Datei an der Zeile (Bus-Topic `openScript`). Zähler in der Statuszeile („2 Fehler, 1 Warnung“).

### 8.3 Navigation

- **Header-Editor:** „Scriptdatei“ und „Level-Script“ werden Pick-Felder mit Maximum aus `member_count(Script)`, dazu „Öffnen“ und „Neu“ (`new_script_file` bzw. mit `kind = LevelScript`; die Nummer wird ins Feld geschrieben, der Header ist dirty wie bei jeder Änderung). Neuer Abschnitt **„Level-Script“**: Tabelle der `header_entries` (Auslöser-Auswahl „Beim Laden (on_load)“, „Beim Fortsetzen (on_resume)“, „Beim Betreten (on_transition)“, „Variablen-Bedingungen (on_var_condition)“; Script-Nummer mit „Öffnen“) und der `var_conditions` (Variable, Wert, Script-Nummer); „Eintrag hinzufügen“, „Entfernen“, „Als JSON öffnen“ (öffnet die Datei im Panel „Script“ im JSON-Modus). Speichern des Abschnitts ruft `save_level_script` (Compile sofort; Validierungsfehler als Problem). Level-Script-ID 0: „Kein Level-Script — Anlegen“.
- **Event-Inspector:** Feld „Skript“ (künftig „Script“) bekommt „Öffnen“ (`resolve_script` → Panel „Script“, Zeile; `Ambiguous` → Meldung mit Kandidaten; `Unknown` → Meldung) und „Neues Script“ (`add_script` an der Scriptdatei des Headers; die neue Nummer wird ins Feld geschrieben, das Event ist dirty). Für Trigger-Events gilt dasselbe Feld.
- **Welt-Explorer:** unter jedem Header die Kinder „Scriptdatei NNNN“ und „Level-Script NNNN“ (nur ≠ 0); neuer Wurzelknoten **„Scripts“** mit allen Dateien (`list_script_files`: Nummer, Art, Größe, Header, die sie nutzen) und Suche (Nummer oder Volltext über `search_scripts`, Treffer mit Zeile). `NavTarget` bekommt `{ kind: "script", file, line? }` und `{ kind: "textArchive", index, message? }`.

### 8.4 Begriffe

`terms.ts` wächst um `scriptFile` („Skriptdatei“ → **„Scriptdatei“**, tech `ScriptFile`, „Alle Scripts eines Headers in einer Datei; Events nennen die Nummer `#n` darin.“), `script` („Script“, „Ein Ablauf in rotom, zum Beispiel `script Main #1`.“), `levelScript` („Level-Script“, „Läuft beim Laden oder Betreten der Karte oder wenn eine Variable einen Wert hat.“), `textArchive` („Textarchiv“, tech `TextArchive`, „Die Nachrichten eines Headers; Scripts zeigen sie über die Nummer.“), `message` („Nachricht“), `problem` („Problem“, „Ein Fehler oder eine Warnung aus Compile, Text oder Sprachdienst.“). Die verstreuten Literale („Skriptdatei“, „Level-Skript“, „Skript“, „Textarchiv“ in `inspector/fields.ts`, `eventFields.ts`, `world/events.ts`) werden auf die Terme umgestellt; Schreibweise überall „Script“.

## 9. Befehle (Tauri)

Neu:

- `scripts_status() -> { adopted: bool, lsp: "running" | "stopped" | "missing", file_count: u16 }`
- `adopt_scripts(progress: Channel<AdoptProgress>) -> AdoptReport` (State-Lock, `spawn_blocking`)
- `list_script_files() -> Vec<ScriptFileDto { file, kind: "script" | "levelScript" | "binary", size, headers: Vec<u16> }>`
- `read_script(file) -> ScriptSourceDto { kind, text | levelScript }`
- `save_script(file, text) -> SaveScriptDto { problems, touched_archives }` (verweigert bei dirty Textarchiv: `CmdError { code: "error", message: "Textarchiv 0199 erst speichern" }`)
- `save_level_script(file, doc) -> SaveScriptDto`
- `new_script_file(kind) -> u16`, `add_script(file) -> u16`
- `resolve_script(header, n) -> ScriptTargetDto`
- `search_scripts(query, limit) -> Vec<{ file, line, excerpt }>`
- `lsp_start(channel: Channel<String>) -> ()` (oder `tool_missing`), `lsp_send(text)`, `lsp_stop()`
- `search_text(query, limit) -> Vec<{ archive, index, excerpt }>`, `validate_text(text) -> Vec<ProblemDto>`, `create_text_archive() -> u16`, `text_archive_users(index) -> Vec<u16>`

Bestehend, geändert: `get_asset`/`edit_asset`/`save_all` kennen `TextArchive` als editierbares Asset (`TextArchiveDto { key, lang, messages: [{ id, text }] }`, Patch = ganze Nachrichtenliste oder einzelne Nachricht); `build_rom` liefert `BuildDto { …, problems }` und bei Abbruch `CmdError { code: "compile", problems }`.

## 10. Fehler

`KernelError` bekommt `Compile(Vec<Problem>)` („3 Fehler in Scripts oder Texten“). `CmdError` bekommt den **neuen Code `compile`** und ein optionales Feld `problems: Vec<ProblemDto>` (serialisiert als `[]`, wenn leer; `api.ts` `isCmdError` und der Test ziehen mit). Die übrigen Codes bleiben `stale`, `read_only`, `tool_missing`, `partial`, `error`. `tool_missing` gilt für `rotom-lsp`. Übernehmen ist nicht `partial`: schlägt `decompile_project` fehl, bleiben `rotom.toml` und `.rotom/` stehen, der Bericht nennt die Dateien, und der nächste Versuch läuft als normales Compile/Decompile im bestehenden Projekt.

## 11. Tests

- **Fixture.** `synthetic_project()` bekommt den Script-NARC (Platin-Pfad) mit drei Membern: ein Script mit zwei `script`-Einträgen und einem `Message`-Befehl (Bytes aus `rotom::compile` eines festen Quelltexts im Test selbst erzeugt, damit die Fixture keine Hex-Konstanten braucht), ein leeres Level-Script (vier Nullbytes), ein leerer Member; Header 0 zeigt auf Scriptdatei 0, Level-Script 1 und Textarchiv 0. Die Textarchive der Fixture bekommen ihren Key aus einem kleinen, im Test gebauten Archiv (chatot `encode_texts` auf eine Temp-JSON).
- **Kernel-Unit** (`scripts.rs`, `text.rs`, `build.rs`): `adopt` schreibt `rotom.toml`, DB, Quellen und `compile-state.json`; zweiter Aufruf verweigert; `.script` im Weg verweigert; `compile(File)` mit Syntaxfehler → `Problem` mit erwarteter Zeile/Spalte und unveränderter Member; Textliteral → Archiv-JSON wächst, `touched_archives` nennt es; Level-Script JSON → Bytes → JSON round-trip; `new_script_file`/`add_script` liefern die erwarteten Nummern und kompilieren; `resolve_script` lokal, Common (Platin 3000 → Datei der Tabelle), DP → `Unknown`; Textarchiv laden aus Member, speichern → Member byte-identisch zu `encode_texts`-Referenz, JSON mit `key` und Segmenten; Member neuer als JSON → Member gewinnt; `create_text_archive` + Build packt; `encode_stale` kodiert nur veraltete; `build` mit kaputter Quelle → `KernelError::Compile`, kein ROM, kein gepacktes NARC geschrieben.
- **Gated** (drei Projekte, `vanilla_copy()`): `adopt` ohne `failed` (oder mit dokumentierter Liste je Projekt, die der Test festschreibt), danach `build` byte-identisch zum ds-rom-Rebuild; alle Textarchive laden und ohne Änderung speichern → Member byte-identisch; `resolve_script` für einen bekannten Trainer-Event-Wert.
- **Tauri:** `CmdError` mit Code `compile` und `problems`; `lsp_start` ohne Binary → `tool_missing`; mit einem Fake-LSP (Shell-Skript, das `initialize` beantwortet) → `Channel` erhält die Antwort, `lsp_stop` beendet die Prozessgruppe.
- **Frontend (vitest):** rotom-`StreamLanguage` tokenisiert ein Beispiel wie erwartet; `ScriptEditor` mountet in jsdom, Tippen setzt Dirty, Cmd+S ruft `onSave`; `TauriTransport` schickt und empfängt gegen einen Fake-Channel; `lspExtras` zeigt Symbole als Outline; `ScriptDocuments` behält Text und Undo beim Wechsel; Problems-Klick ruft `goTo` mit Datei und Zeile; Level-Script-Formular erzeugt genau rotoms JSON (`on_load`, `var_conditions`); Text-Panel: Suche filtert, Anfügen erzeugt eine Nachricht, Überlänge zeigt Warnung; Übernehmen-Kasten erscheint nur ohne `adopted`; `LAYOUT_KEY` v4 verwirft ein v3-Layout; `csp.test.ts` unverändert grün.
- **Von Hand** (Gate-Dokument `docs/phase-3b2-gate.md`): die Punkte aus §1, dazu `tauri build`, gepackte App starten, Script öffnen, Completion erscheint (WKWebView-Rauchtest für CodeMirror und den IPC-Transport).

## 12. Risiken und Festlegungen

- **rotom 0.x.** API-Bruch bei einem Update ist zu erwarten; Lib und `rotom-lsp` stehen auf demselben Tag, `docs/dev-setup.md` nennt ihn. Ein Fork ist der eingeplante Ausweg (MIT).
- **Netz beim ersten Build** (rotoms `build.rs`). Akzeptiert; dokumentiert.
- **Byte-Round-Trip über echte ROM-Scripts** ist rotoms Versprechen, von uns nur an einem kleinen Script gemessen. Der gated Test entscheidet. Scheitert er für einzelne Member, bleiben diese Quellen unverändert und ihre Member damit vanilla; betroffen wären nur Edits an genau diesen Dateien.
- **Textliteral-Regel** (kein Speichern bei dirty Textarchiv) ist grob. Feiner wäre, das Zielarchiv vor dem Compile zu bestimmen (`text_archive_for_script_file`); das kann ein Folgeschritt werden, wenn die Regel stört.
- **mtime-Verträge** (JSON vs. Member, DSPREs `--newer`) sind fragil bei Kopien ohne Zeitstempel. Studio schreibt Quelle und Member in der genannten Reihenfolge und setzt nie Zeitstempel zurück.
- **WKWebView** ungetestet für CodeMirror und Channel-Transport; der Rauchtest im Gate deckt es ab.
- **DP** ungeprüft (Pfad, DB, Common-Scripts ohne Tabelle). Kein Gate; `archives.yaml`-Eintrag mit Kommentar „unverifiziert“.
- **LGPL-dylib** (`libnitroarc_ffi`) wird dynamisch gelinkt und mitgeliefert; das ist schon heute über uxie der Fall und kein neuer Umstand, aber `docs/dev-setup.md` nennt die Lizenz.

## 13. Dokumentation

- `docs/ROADMAP.md`: Phase-3b.2-Block auf diese Spec verweisen, „Monaco“ durch „CodeMirror 6“ ersetzen, Status setzen.
- `docs/dev-setup.md`: `rotom-lsp` bauen (`prepare-bundle.sh`), `G4_ROTOM_LSP`, Netz beim ersten Build, dylib.
- `docs/formats/text_archive.md` (Header, Key, Verschlüsselung — Quelle chatot, verifiziert am Platin-Archiv), `docs/formats/script_file.md` (Sprungtabelle `0xFD13`, Level-Script-Layout — Quelle rotom, DSPRE), `docs/formats/rotom_project.md` (Layout, `rotom.toml`, compile-state, DB-Herkunft).
- `docs/phase-3b2-gate.md` nach dem Muster von `phase-3b1-gate.md`.
