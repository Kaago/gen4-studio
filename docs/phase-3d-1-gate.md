# Phase 3d.1: Gate prüfen

Stand: 2026-10-31. Branch `phase-3d-1-flags-variablen` (Worktree `.claude/worktrees/phase-3d-1`, Basis main `3c194c7`), nicht nach main gemergt. Vom Agenten erledigt: (1) Automatik Schritt für Schritt auf dem Stand `8316e0b` (gated Tests im Release-Profil auf Platin DE, HeartGold DE und HeartGold US) und (2) die Szenarien `pt-flag` und `hg-flag` (Task 8, `793f30f`) sowie die kopflose Probe über `studio mcp` (`8316e0b`). **Offen beim Agenten:** (3) Browser-Teil und MCP über die Brücke, die Blick-Bilder, die übrigen Zellen der Zahlentabelle und der Stand in der Roadmap: das folgt im UI-Test nach dem abschließenden Review (wird im Browser-Teil ergänzt). `scripts/gate.sh` als Ganzes läuft auf main nach dem Merge der Welle (3d.1 ∥ 3d.2 ∥ 3d.3 ∥ 4.5), nicht hier.

Beim Nutzer offen: der Blick auf Fläche, Picker und Dialog in Hell, Dunkel und Kompakt; ein eigener Name in DSPRE geöffnet; `expect-flag` auf HeartGold US (Abschnitt „(4) Hand und Blick“).

Phase 3d.1 bringt Flags und Variablen: die Namen aus pret je Familie (die Tabellen `flags` und `vars` der gebündelten Befehlsdatenbank `scrcmd/{platinum,hgss}_v2.json`, mehrere Namen je Nummer, ein Hauptname), die Bereiche (`flag_ranges.yaml`: kartenlokal, allgemein, versteckte Items, Trainer, System, Tagesflags, Script-lokal …), eigene Namen im Textprojekt als Register (`data/ids/flags.json`, `data/ids/vars.json`) mit Entitätsdatei (`data/flags/NNNN.json`, `data/vars/NNNN.json`) und rotoms ID-Dateien (`.g4-ids-flags.json`), den Nutzungsindex „wo gesetzt, wo geprüft“ (Scripts mit Zugriff aus der Befehlsdatenbank, Objekte mit `hidden_flag`, versteckte Items, Auslöser, Level-Scripts, Trainer-Flags) und den Pool „frei“. Dazu Technik › „Flags und Variablen“ mit Liste, Filtern und Detail, die Picker für „Sichtbarkeits-Flag“ (Objekt), „Variable“ und „Wert“ (Auslöser) und im Level-Script, „Als eigenes Flag benennen…“ und „Umbenennen…“, die MCP-Werkzeuge `list_flags`, `flag_usage`, `free_flag`, `name_flag` und die `g4emu`-Schritte `flag`, `set-flag`, `var`, `set-var`, `expect-flag`, `expect-var` über den Spielstand-Block im RAM. Roadmap: [docs/ROADMAP.md](ROADMAP.md), Abschnitt „Phase 3d“, Zeile 3d.1. Spezifikation: [2026-10-30-phase-3d-erzaehlen-design.md](superpowers/specs/2026-10-30-phase-3d-erzaehlen-design.md) §4. Plan: [2026-10-30-phase-3d-1-flags-variablen.md](superpowers/plans/2026-10-30-phase-3d-1-flags-variablen.md). Format: [flags_vars.md](formats/flags_vars.md); Werkzeuge: [mcp.md](mcp.md).

Gate-Text der Spezifikation (§4, Zeile 3d.1):

> Die Liste zählt die Namen der Metadaten (Platin 4 097, HGSS 2 244 Nummern); `flag_usage` einer Stichprobe stimmt mit einem Vergleichstest über die Quellen überein; warmer Neuaufbau ≤ 1 000 ms, 200 Abfragen ≤ 50 ms; ein freies Flag benennen: Name im Register, in `data/flags/` und in `.g4-ids-flags.json`, ein Script mit `if FLAG_<NAME> then` kompiliert byte-gleich zur pret-Fassung, Umbenennen zieht die Quellen nach als ein Undo-Schritt; im DSPRE-Layout „Umbenennen…“ gesperrt mit Grund; Szenario `pt-flag`/`hg-flag`: Figur mit neuem Sichtbarkeits-Flag (Picker) und Script, das es setzt, bauen, ansprechen, Karte verlassen und betreten, `expect-flag N 1`, die Figur ist weg; Picker, Liste, Detail und Weg zum Script über die Brücke; `id-dangling` meldet eine Lücke in der Mitte richtig

Wer was prüft: Die Spalte „Wer“ sagt **Agent** (die Test-Umgebung aus [docs/agent-gate.md](agent-gate.md): Browser-Fenster mit `studio-bridge`, `call.sh`, `mcp.sh`, dazu `studio mcp` kopflos, `g4emu` und die gated Tests), **Agent + Blick** (der Agent prüft und beschreibt Ort und Zahlen, der Nutzer bestätigt kurz am Bild) oder **Hand** (der Nutzer). Befehle: [docs/agent-gate.md](agent-gate.md), Abschnitt „Phase 3d.1“.

Namen in der Oberfläche (Rollen und `data-*` für die Prüfung):

- **Technik-Eintrag** „Flags und Variablen“ (Symbol `flag`, der letzte Eintrag nach „Werkzeuge“), Untertitel „4.097 Flags · 135 frei“ (die Zahlen der gewählten Art), beim Lesen „Wird gelesen …“, ohne Projekt „Kein Projekt“. Palette: „Flags und Variablen“.
- **Fläche** `[data-technik=flags]`: Titel „Flags und Variablen“, Satz „Welche Flags und Variablen das Spiel kennt und wo Scripts, Figuren, Auslöser, Level-Scripts und Trainer sie setzen oder prüfen.“; im DSPRE-Layout die Info „Eigene Namen gibt es im Textprojekt: Technik › Textprojekt › „Als Textprojekt einrichten…“.“; ohne übernommene Scripts „Scripts sind noch nicht übernommen: Script-Verweise fehlen im Index, „frei“ ist darum nur ein Hinweis.“.
- **Filter** `[data-flag-filter]`: „Flags | Variablen“, „Alle“ und je Bereich („allgemein“, „kartenlokal“, „Trainer“, „versteckte Items“, „System“, „Tagesflags“, „Script-lokal“ …), Kästchen „nur unbenutzt“, „nur frei“, „nur mit eigenem Namen“, Suche „Name oder Nummer suchen“ (eigener Name, pret-Name, Aliasse, Nummer hex und dezimal).
- **Liste** `[data-flag-list]`, Zeile `[data-flag-row=<Nummer dezimal>]`: Name (eigener fett), darunter grau in Mono der pret-Name und „0x0094 · 148“; rechts „gesetzt N · geprüft M“ (· „unklar K“), „unbenutzt“, kartenlokal „an 54 Orten“, Plaketten `[data-flag-badge=free]` „frei“ und `[data-flag-badge=reserved]` „reserviert“ (Titel „Vorlage „Leere Region““); Spalte „Live“ (Kopf `[data-flag-live-head]` in Bernstein, Zellen `[data-flag-live]` „–“, Tooltip „Kommt mit dem Spielmodus (3d.9)“); leer „Nichts passt zu Filter und Suche.“; Fuß `[data-flag-list-foot]` „{n} von {m} · Script-lokale Variablen … Vorkommen, Flags aus Variablen … (nicht im Index)“. Liste und Detail nebeneinander in `[data-flag-split]`.
- **Detail** `[data-flag-detail]`: ohne Auswahl „Wähle links ein Flag oder eine Variable.“; Kopf mit Name, „0x0094 · 148 · allgemein“, Beschreibung oder „Keine Beschreibung“; Tasten „Als eigenes Flag benennen…“ / „Als eigene Variable benennen…“ (ohne eigenen Namen), „Umbenennen…“ und „Beschreibung…“ (mit eigenem Namen); im DSPRE-Layout gesperrt mit „Kein Textprojekt: IDs gibt es erst nach „Als Textprojekt einrichten…“. (Technik › Textprojekt)“, außerhalb des Spielstands mit „Liegt außerhalb des Spielstands.“. Abschnitte `[data-flag-section]` „Wo gesetzt“, „Wo geprüft“, „Unklar“, Gruppen `[data-flag-group]` (nach Quelle, kartenlokal nach Header „Zweiblattdorf (0411)“), je Verweis `[data-flag-use=<read|write|read_write|unknown>]` ein Chip zur Quelle (Klick öffnet sie: Script-Editor an der Zeile, Figur in der Schau, Trainer); leer „Nirgends gesetzt.“ / „Nirgends geprüft.“, bei Lesen ohne Setzen „Gelesen, aber nirgends gesetzt: …“.
- **Feld im Inspector** `[data-flag-field=<Schlüssel>]` (`hidden_flag`, `var`, im Level-Script `var:<Index>`): „Sichtbarkeits-Flag“ am Objekt, „Variable“ und „Wert“ am Auslöser, „Variable“ und „Neue Variable“ im Level-Script; Chip mit Name und Nummer, „Wählen…“ öffnet den Picker, in Tüfteln dazu das Zahlfeld.
- **Picker** (Fuß `[data-flag-picker=<flag|var>]`): erster Treffer „Keins (0)“, dann Name, Bereich oder pret-Name, Nummer hex, „gesetzt N · geprüft M“; Fuß „Freies Flag nehmen“ (gesperrt mit „Kein freies Flag im allgemeinen Bereich.“ oder, ohne übernommene Scripts, „Erst Scripts übernehmen: ohne Script-Quellen weiß der Index nicht, welche Flags Scripts benutzen.“) und „Im Verzeichnis zeigen“.
- **Dialog** `[data-name-flag-dialog]`: „Als eigenes Flag benennen“ / „Als eigene Variable benennen“ / „Beschreibung“, Felder „Eigener Name“ (vorbelegt `FLAG_` bzw. `VAR_`, Hilfe „A–Z, 0–9 und _; der pret-Name {name} bleibt gültig.“) und „Beschreibung“; „Abbrechen“ / „Benennen“ (primär) bzw. „Speichern“; danach „{id} benannt (ein Schritt im Verlauf).“ bzw. „Beschreibung gespeichert.“.

## Vorbereitung

So durchgeführt am 2026-10-31 auf dem Stand `8316e0b` ((1) ab 20:33):

1. **Kopien** unter `Roms/_testkopien/` per `cp -cR` (Ordner) und `cp -c` (`.nds`) aus den Originalen, die dabei nur gelesen wurden, Ordner und `.nds` direkt nebeneinander:
   - `3d-1-pt` + `3d-1-pt.nds` (Platin DE) und `3d-1-hgde` + `3d-1-hgde.nds` (HeartGold DE), angelegt in Task 4 und für (1) wiederverwendet; `3d-1-hgus_DSPRE_contents` + `3d-1-hgus.nds` (HeartGold US) neu. Sie werden nur gelesen: `tests/flags.rs` übernimmt die Scripts einmal je Lauf auf einer Kopie unter `target/tmp/flags-<Ordner>`, der schreibende Test und der Studio-Test arbeiten auf `vanilla_copy()`. Die Basis-ROM findet `tests/flags.rs` über `suggest_base` (Ordner `…_DSPRE_contents`) oder als `<Ordner>.nds` neben einer Kopie `<Ordner>`.
   - `3d-1-szen-pt2_DSPRE_contents` + `3d-1-szen-pt2.nds` und `3d-1-szen-hg_DSPRE_contents` + `3d-1-szen-hg.nds` für (2), aus Task 8 (dort nach dem Lauf als Textprojekt mit `FLAG_SZENARIO_FIGUR` und `build/out.nds`); `3d-1-szen-pt2` trägt auch die kopflose Probe.
   - `3d-1-web` (Platin DE, Scripts übernommen, Basis zugeordnet), `3d-1-mcp` (Platin DE, Basis zugeordnet) und `3d-1-ui-text` (Textprojekt) für (3), je mit `.nds` daneben: wird im Browser-Teil ergänzt.
2. **Werkzeuge:** `cargo build --release -p studio --bin studio`, `cargo build --release -p g4-kernel --bin g4k` und die Release-Testbinaries (`cargo test --release -p g4-kernel --all-features --test flags --test refs --no-run`, `cargo test --release -p studio --all-features --lib --no-run`). `tools/` ist der Symlink auf den Haupt-Checkout (nie einchecken). `G4_ROMS_DIR=/Users/POLTEP/Projekte/lab/editor/Roms` absolut; für die Integrationstests von `g4emu`, die die drei `.nds` unter `G4_ROMS_DIR` öffnen, zeigte `G4_ROMS_DIR` auf einen Ordner im Scratchpad mit `cp -c`-Klonen der drei `.nds` und einem Symlink auf `_testkopien/_saves`, damit kein Original geöffnet wird.
3. **Ausgaben** unter `target/agent/gate-2026-10-31/3d-1/` im Worktree: `auto/` (je Schritt von (1) ein Log und `summary.txt` mit Beginn, Ende, Dauer, Rückgabe und Last), `pt-flag/` und `hg-flag/` (Szenarien aus Task 8: `gespraech.png`, `weg.png`, `play.jsonl`, `mcp.jsonl`, `results.json`), `kopflos/` (die Probe über `studio mcp`). Das Skript für (1) und der JSON-RPC-Client für (2) lagen im Scratchpad. Den Ordner vor `git worktree remove` sichern.
4. **Last:** `uptime` vor und nach jedem Schritt (Spalte „Last“ der Zahlentabelle). Auf dem Rechner liefen gleichzeitig die Sitzungen von 3d.2, 3d.3 und 4.5; die 1-Minuten-Last lag bei Beginn von (1) bei 36. Die drei Spiele liefen nacheinander, nie parallel.

## (1) Automatik

Nach Entscheidung R4 läuft hier nicht `scripts/gate.sh` als Ganzes (das macht die Hauptsitzung auf main nach dem Merge der Welle), sondern die Schritte, die 3d.1 berührt, einzeln. Die neuen gated Tests laufen in `scripts/gate.sh` im Schritt der gated Tests je Projekt (Debug) ohne Änderung am Skript mit; `vanilla_index_builds_under_a_second` bleibt im Zeitschritt (Release, `G4_TIMING_STRICT=1`).

Alle Schritte grün im ersten Versuch, 2026-10-31 20:33 bis 21:38 auf `8316e0b` (Logs `target/agent/gate-2026-10-31/3d-1/auto/<schritt>.log`, Zeiten und Last in `summary.txt`):

- [x] 1. `cargo fmt --check` (3 s) und `cargo clippy --workspace --all-targets --all-features -- -D warnings` (23 s), sauber.
- [x] 2. `cargo test -p g4-kernel --all-features` ohne ROM-Daten (`G4_TEST_PROJECT` und die Werkzeug-Variablen entfernt wie in `scripts/gate.sh`): 1 486 bestanden in 37 Testzielen, 0 ignoriert (`g4emu_games_yaml_carries_the_same_block` läuft seit Task 8 mit), 2 407 s bei Last 34 → 19.
- [x] 3. `cargo test -p studio --all-features` ohne ROM-Daten: Bibliothek 663 bestanden, 1 ignoriert; `mcp_http` 5, `mcp_proxy` 10, `mcp_stdio` 28; 172 s.
- [x] 4. `pnpm test` in `apps/studio`: 338 Dateien, 3 412 Tests bestanden, 1 übersprungen (154 s); `pnpm build` grün (31 s).
- [x] 5. `bash scripts/check-styles.sh` und `bash scripts/check-tokens.sh` grün.
- [x] 6. `cd scripts/g4emu && uv run pytest -q`: 371 bestanden (Unit und die vier Integrationstests mit Kern und Spielständen), 19 s.
- [x] 7. Release-Bau von `studio`, `g4k` und den Testbinaries: 257 s.
- [x] 8. **Gated `tests/flags.rs`** (Release, `--nocapture --test-threads 1`, mit `G4_TIMING_STRICT=1` für alle fünf Tests, die Zeitgrenzen also bindend): je 5/5 auf Platin DE (212 s), HeartGold DE (252 s) und HeartGold US (220 s).
   - `vanilla_flag_names_count_the_metadata`: Platin 4 097 Flags / 307 Variablen, HeartGold DE und US je 2 244 / 386.
   - `vanilla_usage_matches_a_recount_of_the_sources`: der Index gleich der Nachzählung ohne Index in allen sechs Arten (Objekte, versteckte Items, Auslöser, Trainer, Level-Scripts, `SetFlag`/`ClearFlag`/`CheckFlag`); jedes BG-Event mit Script `8000 + n` hat den Typ 2 (`hidden_item_other_type=0`).
   - `vanilla_flag_index_is_fast`: warm 287 / 294 / 262 ms (Grenze 1 000), 200 Abfragen je 7 ms (Grenze 50), `usage` je 0 ms (Grenze 50).
   - `vanilla_pool_counts_the_free_numbers`: 101 / 154 / 154 freie Flags (Grenze 80 bis 170), 20 / 81 / 81 freie Variablen (10 bis 100); die kleinste freie Nummer `0x0042` / `0x0040` (Flag) und `0x4031` / `0x4034` (Variable).
   - `vanilla_own_name_compiles_byte_equal`: ein eigener Name auf der ersten freien Nummer kompiliert byte-gleich zur pret-Fassung (187 / 881 / 205 ms) und folgt `rename_id` (6,1 / 33,4 / 6,0 s, HeartGold DE bei Last 15).
   - Die Scripts zu übernehmen kostete je Lauf 53,9 / 50,4 / 64,9 s (1 124 / 965 / 965 Scripts).
- [x] 9. **Gated `--test refs vanilla_index_builds_under_a_second`** (Release, `G4_TIMING_STRICT=1`, DSPRE-Ordner ohne übernommene Scripts): kalt 366 / 433 / 325 ms, warm 158 / 168 / 148 ms (Grenze 1 000 ms).
- [x] 10. **Gated Studio-Test** `flags_over_the_command_path_on_the_vanilla_copy` (Release, ohne `G4_TIMING_STRICT`): je 1/1 (19 / 51 / 27 s). `list_flags` kalt 503 / 2 014 / 413 ms (mit dem Aufbau des Index), danach 0 / 1 / 0 ms; 4 097 / 2 634 / 2 634 Einträge (HeartGold: die 2 244 benannten Nummern und die benutzten ohne Namen), 307 / 386 / 386 Variablen; `scripts_indexed=false`, `free_flag` → `scripts_not_adopted`; `flag_usage` des Sichtbarkeits-Flags `0x0041` (Platin) bzw. `0x0057` (HeartGold) nennt die Eventdatei und den Header „Route 205“ bzw. „Route 44“ (2 / 10 / 2 ms); `name_flag` → `not_text_project`.

Kein Zeittest war rot, keine Wiederholung nötig. Die Last sank während der gated Tests von 14,6 auf 5,4 (die anderen Sitzungen bauten weniger).

Belegzeilen (Auszug aus `flags-<k>.log` und `studio-<k>.log`):

```
adopt CPUD ms=53854 adopted=Some(1124)
index CPUD warm_ms=287 refs200_ms=7 usage_ms=0
names CPUD flags=4097 vars=307
own CPUD flag=0x0042 compile_ms=187 rename_ms=6053
pool CPUD flags=101 vars=20 first_flag=0x0042 first_var=0x4031
usage CPUD hidden_flag=1613 hidden_item=262 trigger=186 trainer=417 level=207 setflag=1947 script_local=9567 dynamic=8 flags_distinct=1796 vars_distinct=213
recount CPUD Recount { hidden_flag: 1613, hidden_item: 262, trigger: 186, trainer: 417, level: 207, setflag: 1947 } hidden_item_other_type=0
flags CPUD list_ms=0 usage_ms=2 cold_ms=503 entries=4097 vars=307 scripts_indexed=false pool_flags=115 pool_vars=23 first_free=0x0042 object=0x0041 header=Some("Route 205")
adopt IPKD ms=50397 adopted=Some(965)
index IPKD warm_ms=294 refs200_ms=7 usage_ms=0
names IPKD flags=2244 vars=386
own IPKD flag=0x0040 compile_ms=881 rename_ms=33360
pool IPKD flags=154 vars=81 first_flag=0x0040 first_var=0x4034
usage IPKD hidden_flag=1108 hidden_item=241 trigger=195 trainer=404 level=230 setflag=2517 script_local=8142 dynamic=16 flags_distinct=1613 vars_distinct=253
recount IPKD Recount { hidden_flag: 1108, hidden_item: 241, trigger: 195, trainer: 404, level: 230, setflag: 2517 } hidden_item_other_type=0
flags IPKD list_ms=1 usage_ms=10 cold_ms=2014 entries=2634 vars=386 scripts_indexed=false pool_flags=273 pool_vars=164 first_free=0x0040 object=0x0057 header=Some("Route 44")
adopt IPKE ms=64900 adopted=Some(965)
index IPKE warm_ms=262 refs200_ms=7 usage_ms=0
names IPKE flags=2244 vars=386
own IPKE flag=0x0040 compile_ms=205 rename_ms=6019
pool IPKE flags=154 vars=81 first_flag=0x0040 first_var=0x4034
usage IPKE hidden_flag=1108 hidden_item=241 trigger=195 trainer=404 level=230 setflag=2517 script_local=8142 dynamic=16 flags_distinct=1613 vars_distinct=253
recount IPKE Recount { hidden_flag: 1108, hidden_item: 241, trigger: 195, trainer: 404, level: 230, setflag: 2517 } hidden_item_other_type=0
flags IPKE list_ms=0 usage_ms=2 cold_ms=413 entries=2634 vars=386 scripts_indexed=false pool_flags=273 pool_vars=164 first_free=0x0040 object=0x0057 header=Some("Route 44")
```

Dieselben Zahlen (Namen, Nachzählung, Pool) hatte Task 4 am 2026-10-31 auf `a9b983b` gemessen (Platin warm 762 ms bei Last 21, HeartGold DE 268 ms); sie sind seither unverändert.

Befehle aus der Wurzel des Worktrees (`K=$G4_ROMS_DIR/_testkopien`, `P` = `$K/3d-1-pt`, `$K/3d-1-hgde` oder `$K/3d-1-hgus_DSPRE_contents`):

```sh
export PATH="$HOME/.cargo/bin:/opt/homebrew/opt/rustup/bin:$PATH"
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
env -u G4_TEST_PROJECT cargo test -p g4-kernel --all-features
env -u G4_TEST_PROJECT cargo test -p studio --all-features
(cd apps/studio && pnpm test && pnpm build)
bash scripts/check-styles.sh; bash scripts/check-tokens.sh
(cd scripts/g4emu && uv run pytest -q)                      # Unit und Integration
G4_TEST_PROJECT=$P G4_TIMING_STRICT=1 \
  cargo test --release -p g4-kernel --all-features --test flags -- --nocapture --test-threads 1
G4_TEST_PROJECT=$P G4_TIMING_STRICT=1 \
  cargo test --release -p g4-kernel --all-features --test refs vanilla_index_builds_under_a_second -- --nocapture --test-threads 1
G4_TEST_PROJECT=$P \
  cargo test --release -p studio --all-features --lib flags_over_the_command_path_on_the_vanilla_copy -- --nocapture --test-threads 1
```

## (2) Szenarien

**`pt-flag` und `hg-flag`** (Task 8, 2026-10-31 20:07 bis 20:13 auf `793f30f`, beide parallel, Last 36 zu Beginn und 30 am Ende, Exit 0): über **eine** Sitzung `studio mcp` mit `mcp: [--allow-irreversible, --expert]` (`new_script_file` und `set_permission` gehen über `g4_call`), Debug-Binaries `G4_STUDIO_BIN=target/debug/studio`, `G4K_BIN=target/debug/g4k` nach `cargo build --workspace --all-features`. Ablauf: `assign_base` (`BASIS`), `adopt_text_project`, neues Gebiet aus der Vorlage 342 mit neuer Eventdatei und Scriptdatei, Treppe des Zimmers dorthin, `free_flag`, `name_flag` → `FLAG_SZENARIO_FIGUR`, Figur mit `hidden_flag` = Nummer, Script `SetFlag FLAG_SZENARIO_FIGUR`, `save_script` ohne Probleme, `build_rom`; gespielt: Flag 0, Weg nach oben endet vor der Figur, ansprechen, Flag 1, durch die Tür hinaus und die Treppe wieder hinunter, Flag weiter 1, derselbe Weg geht eine Kachel weiter, wo die Figur stand.

```sh
# aus scripts/g4emu; W = Worktree, R = $G4_ROMS_DIR, O = $W/target/agent/gate-2026-10-31/3d-1
uv run g4emu scenario $W/scripts/agent/scenarios/pt-flag.yaml --project $R/_testkopien/3d-1-szen-pt2_DSPRE_contents \
  --out $O/pt-flag --set BASIS=../3d-1-szen-pt2.nds --set NEUER_HEADER=593 --set X_FIGUR=54 --set Z_FIGUR=17
uv run g4emu scenario $W/scripts/agent/scenarios/hg-flag.yaml --project $R/_testkopien/3d-1-szen-hg_DSPRE_contents \
  --out $O/hg-flag --set BASIS=../3d-1-szen-hg.nds --set NEUER_HEADER=540 --set X_FIGUR=15 --set Z_FIGUR=13
```

- [x] 1. **Platin DE** `{"scenario": "pt-flag", "ok": true, "headers": [415, 593, 415, 593]}`, 5 min 41 s: `create_area` Header 593, Eventdatei 534, Karten 666/667, Scriptdatei 1124; `free_flag` → `{"number": 66, "name": "FLAG_UNK_0x0042", "pool_free": 101, "scripts_indexed": true}`; `name_flag` → `created: true`, `data/ids/flags.json`, `data/flags/0066.json`; `save_script` ohne Probleme (`SetFlag FLAG_SZENARIO_FIGUR` kompiliert); `build_rom` ok. Spiel: `expect-flag … "flag": 66, "value": 0` (Bild 1503) → blockiert bei (54, 18) nach 3 Schritten → ansprechen → `"value": 1` (2098) → Tür → 415 → Treppe → 593 → `"value": 1` (2850) → bis (54, 17) nach 4 Schritten, wo die Figur stand.
- [x] 2. **HeartGold DE** `{"scenario": "hg-flag", "ok": true, "headers": [64, 540, 64, 540]}`, 5 min 45 s: Header 540, Eventdatei 491, Karte 676, Scriptdatei 965; `free_flag` → `{"number": 64, "name": "FLAG_UNK_040", "pool_free": 154}`; `name_flag` → `data/flags/0064.json`; `save_script` ohne Probleme; `build_rom` ok. Spiel: `"flag": 64, "value": 0` (1306) → blockiert bei (15, 14) nach 2 Schritten → ansprechen → `"value": 1` (1870) → Tür → 64 → Treppe → 540 → `"value": 1` (2621) → bis (15, 13) nach 3 Schritten.
- [x] 3. Bilder (nicht im Repo) `target/agent/gate-2026-10-31/3d-1/{pt,hg}-flag/gespraech.png` (der Spieler vor der Figur, Box „Bis…“; `mash a 240` spricht die Figur nach dem ersten Text ein zweites Mal an, das Bild zeigt die zweite Box, das Flag setzt das erste Gespräch) und `weg.png` (der Spieler auf der Kachel der Figur, keine Figur mehr).

Die Kacheln der Figur stammen aus den Bewegungsrechten der Kopien (Platin `unpacked/maps/0006`, HeartGold `unpacked/maps/0001`, Kollisionsbit `0x80`, hohes Gras `0x02`): nördlich der Ankunft liegt hohes Gras, darum steht die Figur daneben auf freiem Boden, Platin (54, 17) mit Spalte 54 frei von Zeile 21 bis 17, HeartGold (15, 13) mit Spalte 15 frei von 16 bis 13; das Spiel bestätigte beide.

**Kopflos über `studio mcp`** auf der Szenario-Kopie `3d-1-szen-pt2` (nach dem Lauf: Textprojekt, Scripts übernommen, `FLAG_SZENARIO_FIGUR` = `0x0042`), Release-Binary, JSON-RPC über stdio (Protokoll 2025-11-25):

```sh
target/release/studio mcp --project $K/3d-1-szen-pt2_DSPRE_contents --base-rom $K/3d-1-szen-pt2.nds
#   list_flags {"kind":"flag"}                         → entries ≥ 4 097
#   flag_usage {"kind":"flag","number":66}             → ein Objekt-Verweis hidden_flag, ein Script-Verweis SetFlag write
#   flag_usage {"kind":"flag","number":"FLAG_SZENARIO_FIGUR"}  → dieselbe Antwort
```

Bestanden am 2026-10-31 21:25 auf `8316e0b` (Last 14,3, 11,4 s mit Start; Log `kopflos/kopflos-pt.log`, Antworten `kopflos/mcp.jsonl`):

- [x] 4. `list_flags {"kind":"flag"}`: 4 097 Einträge (≥ 4 097), `scripts_indexed: true`, `pool_free` 100 (eins weniger als auf der Vanilla-Kopie: `0x0042` ist jetzt benutzt und benannt), `counts {script_local: 9567, dynamic: 8}`; 7,0 s für die erste Liste (Index und Scripts kalt), danach `list_flags {"kind":"var"}` 307 Einträge in 17 ms. Zeile `0x0042`: `own_id` `FLAG_SZENARIO_FIGUR`, `name` `FLAG_UNK_0x0042`, `description` „Figur im Szenario“, Bereich `general`, `usage {read: 1, write: 1, sources: 2}`, `free: false`.
- [x] 5. `flag_usage {"kind":"flag","number":66}` (5 ms) und mit `"number":"FLAG_SZENARIO_FIGUR"` (4 ms) gleich: genau zwei Verweise, `hidden_flag` an Objekt 0 der Eventdatei 534 (`access: read`, „Objekt 0 · Sichtbarkeits-Flag“, Header 593) und `SetFlag` in Zeile 10 der Scriptdatei 1124 (`access: write`, „Zeile 10 · SetFlag“, Header 593). Der Header heißt „Route 201“, weil das neue Gebiet den Ortsnamen der Vorlage behält.

## (3) Browser-Teil

Wird im Browser-Teil ergänzt (UI-Test nach dem abschließenden Review, Entscheidung R4). Test-Umgebung `G4_VITE_PORT=1700 G4_BRIDGE_PORT=1701`, Kopie `3d-1-web` mit übernommenen Scripts und zugeordneter Basis. Geplant:

- [ ] 1. Technik › „Flags und Variablen“: der Untertitel zählt 4.097 Flags; Filter „nur frei“; eine Zeile wählen.
- [ ] 2. Detail mit Wegen: Klick auf einen Script-Weg öffnet den Script-Editor an der Zeile, Klick auf eine Figur wählt sie in der Schau.
- [ ] 3. Picker im Inspector einer Figur: „Freies Flag nehmen“, speichern, `scripts/agent/call.sh get_events '{"event_file_id":<n>}'` zeigt `hidden_flag`.
- [ ] 4. Level-Script: „Neue Variable“ über den Picker.
- [ ] 5. DSPRE-Layout: „Als eigenes Flag benennen…“ gesperrt mit Grund (`NOT_A_TEXT_PROJECT` und der Weg Technik › Textprojekt).
- [ ] 6. `3d-1-ui-text` (Textprojekt): benennen → Register `data/ids/flags.json`, `data/flags/NNNN.json` und `.g4-ids-flags.json` auf der Platte; „Umbenennen…“ → Script-Quelle und Entität folgen, ⌘Z nimmt es als ein Schritt zurück.
- [ ] 7. MCP über die Brücke (`3d-1-mcp`, Freigabe „Lesen und bearbeiten“): `list_flags {"kind":"flag"}`, `flag_usage {"kind":"flag","number":"FLAG_UNUSED_0x0094"}`, `free_flag {"kind":"var"}` (ohne übernommene Scripts `scripts_not_adopted`), `name_flag {…}` → `not_text_project` im DSPRE-Layout.
- [ ] 8. Konsole und Brückenlog ohne Fehler.

## (4) Hand und Blick

Beim Nutzer (Abschnitte „Hand“ und „Blick“ unten): der Blick auf Fläche, Picker und Dialog in Hell, Dunkel und Kompakt; ein eigener Name in DSPRE geöffnet; `expect-flag` auf HeartGold US.

## Ergebnis

| Punkt | Wer | Ergebnis | Datum |
|---|---|---|---|
| (1) 1 bis 7 fmt, clippy, Kernel- und Studio-Tests ohne ROM-Daten, `pnpm test` und `build`, Styles, Tokens, `g4emu` | Agent | bestanden: alles grün im ersten Versuch (Kernel 1 486, Studio 706, Frontend 3 412, `g4emu` 371) | 2026-10-31 |
| (1) 8 gated `tests/flags.rs` | Agent | bestanden: je 5/5 auf Platin DE, HeartGold DE, HeartGold US (Release, `G4_TIMING_STRICT=1`); Namen 4 097 / 2 244, Index gleich der Nachzählung, warm ≤ 294 ms, 200 Abfragen 7 ms, Pool 101 / 154, eigener Name byte-gleich und umbenannt | 2026-10-31 |
| (1) 9 gated `vanilla_index_builds_under_a_second` | Agent | bestanden auf allen drei, warm 148 bis 168 ms | 2026-10-31 |
| (1) 10 gated Studio-Test | Agent | bestanden auf allen drei: Liste über den Befehlsweg, `flag_usage` mit Header, `scripts_not_adopted`, `not_text_project` | 2026-10-31 |
| (2) 1 bis 3 Szenarien `pt-flag`, `hg-flag` | Agent | bestanden auf Platin DE und HeartGold DE: `expect-flag … 1` nach dem Gespräch und nach Verlassen und Betreten, die Figur ist weg (`weg.png`) | 2026-10-31 |
| (2) 4 und 5 kopflos `list_flags`, `flag_usage` | Agent | bestanden auf `3d-1-szen-pt2`: 4 097 Einträge, `FLAG_SZENARIO_FIGUR` mit genau einem Objekt-Verweis `hidden_flag` und einem Script-Verweis `SetFlag` write | 2026-10-31 |
| Gate-Text: Benennen und Umbenennen im Kernel | Agent | bestanden in (1) 2 und 8: `own_name_is_a_second_name_and_both_compile_the_same` (pret-Name und eigener Name kompilieren gleich), `renaming_an_own_flag_name_follows_script_and_entity` (Script-Quelle und Entitätsdatei folgen), gated `vanilla_own_name_compiles_byte_equal` auf allen drei; Register, `data/flags/` und `.g4-ids-flags.json` auf der Platte und ⌘Z als ein Schritt: wird im Browser-Teil ergänzt | 2026-10-31 |
| Gate-Text: DSPRE-Layout gesperrt mit Grund | Agent | bestanden im Kern: `name_flag_refuses_the_dspre_layout` und der gated Studio-Test (`not_text_project`); in der Oberfläche: wird im Browser-Teil ergänzt | 2026-10-31 |
| Gate-Text: `id-dangling` meldet eine Lücke in der Mitte | Agent | bestanden in (1) 2: `id_dangling_counts_the_highest_number_not_the_files`, `register_entries_of_flags_are_never_dangling` | 2026-10-31 |
| (3) Browser-Teil und MCP über die Brücke | Agent | wird im Browser-Teil ergänzt | – |
| Blick auf Fläche, Picker, Dialog | Agent + Blick | wird im Browser-Teil ergänzt (Bilder aus dem UI-Test) | – |
| Eigener Name in DSPRE geöffnet | Hand | offen | – |
| `expect-flag` auf HeartGold US | Hand | offen (Szenario nur für Platin DE und HeartGold DE; `IPKE`, Zeiger `0x02111880`, nur in `games.yaml` übernommen, nicht gemessen) | – |

Zahlentabelle (Release; (1) aus den gated Tests, Szenario aus (2); was der Browser-Teil misst, wird im Browser-Teil ergänzt):

| Spiel | Namen (Flags / Variablen) | Verweise neu (Flags / Variablen) | Index warm (ms) | 200 Abfragen (ms) | `usage` (ms) | Pool frei (Flags / Variablen) | Szenario `expect-flag` | Last |
|---|---|---|---|---|---|---|---|---|
| Platin DE | 4 097 / 307 | 1 796 / 213 Nummern mit Verweis (Objekte 1 613, versteckte Items 262, Trainer 417, `Set/Clear/CheckFlag` 1 947; Auslöser 186, Level-Scripts 207) | 287 (`refs` 158) | 7 | 0 (Studio 2) | 101 / 20 | `0x0042` (66): 0 → 1 → 1, Figur weg | gated 14,6 → 8,9; Szenario 36 → 30; kopflos 14,3 |
| HeartGold DE | 2 244 / 386 | 1 613 / 253 (Objekte 1 108, versteckte Items 241, Trainer 404, `Set/Clear/CheckFlag` 2 517; Auslöser 195, Level-Scripts 230) | 294 (`refs` 168) | 7 | 0 (Studio 10) | 154 / 81 | `0x0040` (64): 0 → 1 → 1, Figur weg | gated 9,0 → 15,0; Szenario 36 → 30 |
| HeartGold US | 2 244 / 386 | 1 613 / 253 (wie HeartGold DE) | 262 (`refs` 148) | 7 | 0 (Studio 2) | 154 / 81 | – | gated 15,0 → 5,4 |

„Verweise neu“ zählt die Nummern mit mindestens einem Verweis (`flags_distinct`, `vars_distinct` aus `usage`), in Klammern die Verweise je Quelle, die der Vergleichstest nachzählt; die übrigen Script-Befehle mit Flag- oder Variablen-Parameter stehen in den Nummern, nicht in den Klammern. Script-lokale Variablen (ab `0x8000`) und Flags aus Variablen zählt der Index nur: Platin 9 567 / 8, HeartGold 8 142 / 16. „Index warm“ ist der zweite volle Aufbau mit übernommenen Scripts (`vanilla_flag_index_is_fast`), in Klammern der warme Aufbau im DSPRE-Ordner (`vanilla_index_builds_under_a_second`). „200 Abfragen“ fragt die Trainer-Flags `0x550` bis `0x617` einzeln. „Pool“ ist der Stand mit übernommenen Scripts; ohne sie zählt der Studio-Test 115 / 23 bzw. 273 / 164, und `free_flag` lehnt ab. Die Werte der Oberfläche (Liste über die Brücke, Picker) werden im Browser-Teil ergänzt.

## Befunde

Keine Befunde in (1) und (2): jeder Schritt war im ersten Versuch grün, kein Zeittest rot.

Befunde des UI-Tests und des abschließenden Reviews: wird im Browser-Teil ergänzt.

## Hinweise

- **Pool (R9):** die untere Grenze des gated Tests `vanilla_pool_counts_the_free_numbers` für freie Flags ist 80 (Plan: 100). Platin DE liegt bei 101, nah an 100; eine weitere Script-Verwendung aus einem Wechsel von rotom oder der Dekompilierung hätte den Test rot gemacht. Die Grenze für Variablen bleibt 10 bis 100. Gutachten A zählte 135 / 152 Flags und 21 / 83 Variablen mit einer anderen Namensregel; der Index zählt dazu die Trainer-Flags und versteckten Items als Verweise.
- **`free_flag` ohne übernommene Scripts (R12):** auf einem Projekt ohne übernommene Scripts lehnt `free_flag` mit `scripts_not_adopted` ab, weil der Index Script-Verwendungen erst nach dem Übernehmen sieht; der gated Studio-Test `flags_over_the_command_path_on_the_vanilla_copy` prüft das auf den nicht übernommenen Kopien (`scripts_indexed=false`). Die Liste zeigt dort `free: true` nur als Hinweis (Satz der Fläche „… „frei“ ist darum nur ein Hinweis.“); die Pool-Zahlen des Studio-Tests sind darum größer als die des Kernel-Tests, der die Scripts übernimmt.
- **Zeitgrenzen** gelten nur mit `G4_TIMING_STRICT=1` (Index warm ≤ 1 000 ms, 200 Abfragen ≤ 50 ms, `usage` ≤ 50 ms; `vanilla_index_builds_under_a_second` ≤ 1 000 ms); ohne die Variable drucken die Tests nur. Gemessen wurde unter der Last der parallelen Sitzungen; Zeilen mit `uptime` in `auto/summary.txt`.
- **Basis-ROM der gated Tests:** `tests/flags.rs` findet die `.nds` auch neben einer Kopie ohne Endung `_DSPRE_contents`; andere gated Tests, die `test_support::vanilla_base_rom()` nutzen (etwa `textproject_source.rs`), überspringen sich auf `3d-1-pt` und `3d-1-hgde` und sind hier nicht gemeint.
- **Szenario-Platzhalter:** `BASIS=../<Kopie>.nds` (die Szenarien beginnen mit `assign_base`, weil `adopt_text_project` eine Basis-ROM braucht und auf einer frischen DSPRE-Kopie mit `base_missing` ablehnt), `NEUER_HEADER` (Platin 593, HeartGold 540), `X_FIGUR`/`Z_FIGUR` (Platin 54/17, HeartGold 15/13). HeartGold kennt `Message`/`CloseMessage` nicht, das Script dort nutzt `NPCMsg`/`CloseMsg`.

## Hand

- [ ] **Eigener Name in DSPRE:** ein Textprojekt mit eigenem Flag-Namen zurück ins DSPRE-Layout („Zurücknehmen“) und in DSPRE öffnen: DSPRE liest den Konstantenordner nie, die Scripts zeigen weiter die pret-Namen bzw. Nummern, nichts bricht.
- [ ] **HeartGold US im Emulator:** `expect-flag` auf `IPKE` (Zeiger `0x02111880`, sonst wie IPKD) ist nur in `games.yaml` übernommen und nicht gemessen; `hg-flag.yaml` mit `game: IPKE` auf einer frischen Kopie von HeartGold US laufen lassen.

## Blick

Wird im Browser-Teil ergänzt: Bilder `blick-<fläche>-<zustand>-<thema>.png` unter `target/agent/gate-<datum>/3d-1/` aus dem UI-Test (Fläche mit Filtern, Zeile mit eigenem Namen, Detail mit „Wo gesetzt“/„Wo geprüft“ und kartenlokal gruppiert, DSPRE-Sperre, Picker im Inspector einer Figur und am Level-Script, Dialog „Als eigenes Flag benennen“; Hell, Dunkel, Kompakt). Vergleich mit dem Design-System: `design-system/README.md`, Abschnitt „Nachtrag 3d.1“; die Frames sind noch nicht in der `.fig` (OpenPencil nicht erreichbar, Anleitung `design-system/openpencil/NACHTRAG-3D-1.md`).

## Offen / später

- **Umbenennen auf einer Kopie in Vanilla-Größe:** `rename_id` dauerte im gated Test `vanilla_own_name_compiles_byte_equal` 6,0 bis 6,1 s auf Platin DE und HeartGold US, 33,4 s auf HeartGold DE bei Last 15 (Task 4: 37 s auf Platin DE bei Last 21); das ist der bestehende Weg des Umbenennens (veraltete Quellen markieren, ID-Dateien über rund 1 100 Quellen neu), nicht Code von 3d.1. In der Oberfläche fällt es beim Umbenennen eines Flags in einem großen Projekt auf.
- **Nicht in 3d.1:** Namen statt Zahlen in den Event-JSON des Textprojekts (3d.4, `schemaVersion` 2), Spielmodus live in der App (3d.9, die Spalte „Live“ bleibt leer), „Datei entfernen“ eines eigenen Namens (3d.4), `g4k`-Befehle für Flags.
