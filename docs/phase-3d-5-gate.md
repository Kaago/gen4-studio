# Phase 3d.5: Gate prüfen

Stand: 2026-11-02. Branch `phase-3d-5-figur-dialog-tuer` (Worktree `.claude/worktrees/phase-3d-5`), nicht nach main gemergt. Das Gate-Dokument liegt als Gerüst vor: die Punkte stehen fest, die Ergebnisse folgen nach dem UI-Test des Steuer-Agenten („Ergebnis: folgt nach dem UI-Test“). Zahlen, Befunde und der Stand der Roadmap-Zeile werden danach eingetragen.

(1) 1. `scripts/gate.sh` als Ganzes: läuft **nicht** in dieser Phase, sondern vom Hauptorchestrator auf main nach dem Merge (Entscheidung des Steuer-Agenten, R8: unter paralleler Last scheitern seine Zeitgrenzen; die Schritte, die 3d.5 berührt, laufen hier einzeln).

Phase 3d.5 bringt das Erzählen von Figuren, Dialogen und Türen. Im Kernel liegen `dialog_of` (der Dialog einer Figur oder eines Schilds, mit Nachrichten in Lesereihenfolge, Sprüngen in Labels und dem Zähler „geteilt“ je Nachricht aus dem Index der Referenzen, `Target::Message`) und `split_message` (gibt einer Figur eine eigene Kopie einer geteilten Nachricht); die Rezepte „Neue Figur“ (`new_figure`, eine **nummerierte** Nachricht im Archiv des Headers, ein neuer Script-Platz), „Tür verbinden“ (`connect_doors`, beide Event-Dateien in einem Schritt) und „Neuer Trainer hier“ (`new_trainer_here`, immer unumkehrbar: Trainer anlegen, dann Team, Texte und Objekt als ein Schritt) mit `recipe_plan` und `recipe_apply` über MCP und `g4k recipe`. In Erzählen zeigt der Inspector einer Figur das Feld „Dialog“ (ein Schild „Text“) mit der DS-Vorschau, der Dialog-Editor öffnet sich über das eine Textarchiv, drei Assistenten legen die Rezepte an, und eine Tür zeigt die Zeile „Gegenstück“. Zwei Szenarien spielen das Ergebnis: `pt-erzaehlen` und `hg-erzaehlen`.

Gate-Text der Spezifikation (§4, Zeile 3d.5):

> Kopflos über MCP auf den drei Spielen: „Neue Figur“ mit Text → `g4k build` → `g4emu`: zur Figur gehen, `press a`, `shot` zeigt den Text (Agent + Blick; gated: Nachricht im Archiv des Headers gleich der Eingabe); „Tür verbinden“ zwischen zwei Orten → `expect-map` hin und zurück; „Neuer Trainer hier“ → `walk-until-battle`, `expect-battle <Spezies> <Level>`; Dialog einer Vanilla-Figur in der Test-Umgebung ändern → nur das eine Textarchiv im Diff, ⌘Z nimmt es zurück, eine geteilte Nachricht zeigt „Geteilt mit N Figuren“ und spaltet ab; `pt-trainer-begegnung` grün

Wer was prüft: Die Spalte „Wer“ sagt **Agent** (die Test-Umgebung aus [docs/agent-gate.md](agent-gate.md): Browser-Fenster mit `studio-bridge`, `call.sh`, `mcp.sh`, dazu `g4k`, `studio mcp` kopflos, `g4emu` und die gated Tests), **Agent + Blick** (der Agent prüft und beschreibt Ort und Zahlen, der Nutzer bestätigt kurz am Bild) oder **Hand** (der Nutzer). Befehle: [docs/agent-gate.md](agent-gate.md), Abschnitt „Phase 3d.5“.

Namen in der Oberfläche (Rollen und `data-*` für die Prüfung):

- **Rezept-Leiste** (Erzählen, Welt, 3D vorn): `[role=toolbar][aria-label=Rezepte]` mit fünf **freien** Tasten `[data-recipe=create_area]` „Gebiet anlegen“, `[data-recipe=create_room]` „Raum anlegen“, `[data-recipe=new_figure]` „Neue Figur“, `[data-recipe=connect_doors]` „Tür verbinden“, `[data-recipe=new_trainer_here]` „Neuer Trainer hier“ (seit 3d.5 ist keine mehr gesperrt, nur Symbol mit Name im Tooltip und im `aria-label`).
- **Assistenten:** `[data-wizard=new_figure]` „Neue Figur“ (Schritte „Wo und wie“, „Dialog“, „Zusammenfassung“, Taste „Aufstellen“, Ergebnis „Figur aufgestellt“), `[data-wizard=connect_doors]` „Tür verbinden“ (Schritte „Tür A“, „Tür B“, „Zusammenfassung“, Taste „Verbinden“, Ergebnis „Türen verbunden“; je Tür „Vorhandene Tür“ oder „Neue Kachel“), `[data-wizard=new_trainer_here]` „Neuer Trainer hier“ (Schritte „Trainer“, „Team“, „Texte“, „Wo“, „Zusammenfassung“, Taste „Aufstellen“, Ergebnis „Trainer aufgestellt“; in der Zusammenfassung `[data-trainer-barrier]` „Trainer anlegen lässt sich nicht rückgängig machen; Team, Texte und Figur nimmt ein Schritt zurück.“ mit der Plakette „Nicht rückgängig“). Der Entwurf zeigt sich ab Schritt 1 (`[data-draft][data-draft-count]` am Host der 3D-Ansicht, mit Sprite der Figur oder des Trainers, bis dahin der Platzhalter); die Vorschau der Figur im Assistenten `[data-figure-preview]`, die Ortswahl `[data-place-picker]`, die Aussehen-Wahl `[data-look-picker]`.
- **Ergebnis-Tasten** unter „Als Nächstes“ (`[data-next-steps]`, `[data-recipe=<id>]`): nach „Figur aufgestellt“ „Dialog bearbeiten“ **frei** (öffnet den Dialog-Editor der neuen Figur) und „Zu einer Geschichte“ gesperrt „Kommt mit 3d.7“; nach „Türen verbunden“ und „Trainer aufgestellt“ „Hier spielen“ gesperrt „Kommt mit 3d.9“; nach „Raum angelegt“ „Neue Figur hier“ jetzt **frei** (öffnet „Neue Figur“ mit dem Ort vorbelegt), nach „Gebiet angelegt“ „Tür verbinden“ und „Neue Figur hier“ frei.
- **Feld „Dialog“** im Inspector einer Figur in Erzählen (ein Schild: „Text“): `[data-dialog-section]` mit der DS-Vorschau `[data-dialog-preview=textbox|sign]` (erste Seite der ersten Nachricht), einer Zeile („2 Nachrichten · 3 Seiten“, „Sagt nichts“, „Gemeinsames Script N: in Bauen bearbeiten.“, „Kampftexte des Trainers: im Bereich Pokémon.“, „Script nicht lesbar: …“) und der Taste „Dialog bearbeiten…“ (nur bei einem eigenen Script); bei einer Figur ohne Script „Dialog anlegen…“ gesperrt „Kommt mit 3d.7“.
- **Dialog-Editor** `[data-dialog-editor]` (Modal, breit, Titel „Dialog von Objekt 3“ bzw. „Dialog von Schild 0“, Taste „Schließen“): links die Liste „Nachrichten“ mit `[data-dialog-message="<Archiv>:<Index>"]` (Kopf „Nachricht 4 · Archiv 554“, bei einem Sprung „· wenn …“ oder „· sonst“) und je Nachricht dem Textfeld (`MessageEditor`, Shift+Enter eine neue Seite, Übernahme beim Verlassen des Felds, ein Undo-Schritt „Dialog geändert“); rechts die DS-Vorschau der gewählten Nachricht mit „Vorige Seite“, „Seite 1 von 2“, „Nächste Seite“. Die Plakette „Geteilt mit N Figuren“ (eine „Geteilt mit 1 Figur“) erscheint an einer Nachricht, die weitere Figuren zeigen, dort auch die Taste „Für diese Figur abspalten“ (nur bei einer Figur, nicht bei einem Schild); danach „Nachricht N ist jetzt nur diese Figur.“.
- **Tür in Erzählen:** unter „Tür“ die Zeile „Gegenstück“ mit `[data-warp-counterpart]` („Tür 0 in Zweiblattdorf · führt zurück“ oder „führt nicht zurück“, „Ziel hat keine Tür N“) und der Taste `[data-connect-doors]` „Tür verbinden…“, die den Assistenten mit dieser Tür als Tür A öffnet.
- **Agentenfrage** `[data-agent-question]` (über MCP aus der Brücke): „„agent-gate“ möchte: Trainer anlegen (lässt sich nicht rückgängig machen).“, „Ablehnen“ / „Zulassen“, bei `recipe_apply` mit `recipe: "new_trainer_here"`.

## Vorbereitung

Zu tun am 2026-11-02 auf dem Stand des Branch-Kopfs vor dem UI-Test (Zeiten und Commit trägt der Doku-Lauf nach):

1. **Frische Kopien** unter `/Users/POLTEP/Projekte/lab/editor/Roms/_testkopien/` per `cp -cR` (Ordner) und `cp -c` (`.nds`) aus den Originalen, die dabei nur gelesen werden; Ordner `3d-5-<zweck>_DSPRE_contents` und `.nds` `3d-5-<zweck>.nds` nebeneinander:
   - `3d-5-pt` (Platin DE), `3d-5-hgde` (HeartGold DE), `3d-5-hgus` (HeartGold US) für (1); nur gelesen, die Tests schreiben auf `vanilla_copy()`;
   - `3d-5-kpt`, `3d-5-khgde`, `3d-5-khgus` für (2), je mit `g4k git init` (Commit „Ausgangsstand“);
   - `3d-5-szen-pt` und `3d-5-szen-hg` für `pt-erzaehlen.yaml` und `hg-erzaehlen.yaml`; `3d-5-trainer` (Platin DE) für `pt-trainer-begegnung.yaml`; `3d-5-flag` (Platin DE, mit `.nds` für `--set BASIS=../3d-5-flag.nds`) für `pt-flag.yaml`;
   - `3d-5-web` (Platin DE) und `3d-5-webhg` (HeartGold DE) für (3), `3d-5-mcp` (Platin DE) für MCP über die Brücke, je mit `g4k git init`, damit `git status` etwas sagt.

   Alle `3d-5-*`-Kopien nach dem Gate löschen; `Manual`, `_saves` und die Kopien der parallelen Phasen bleiben. Kein ROM-Inhalt im Repository.
2. **Werkzeuge:** `cargo build --release -p g4-kernel --bin g4k`, `cargo build --release -p studio --bin studio` und die Release-Testbinaries (`--test dialog --test refs_messages --test recipes_erzaehlen --test recipes --test templates --test trainer_create --no-run`). `tools/` ist der Symlink auf den Haupt-Checkout (nie einchecken). `G4_ROMS_DIR=/Users/POLTEP/Projekte/lab/editor/Roms` absolut; `export PATH="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$PATH"` vor jedem Cargo-Aufruf.
3. **Test-Umgebung** für (3) auf den Ports **1750/1751**: `G4_ROMS_DIR=<Roms> G4_VITE_PORT=1750 G4_BRIDGE_PORT=1751 bash scripts/agent/studio-web.sh`; Chrome headless 1440 × 900 (`--use-angle=swiftshader --enable-unsafe-swiftshader`), Hell und Dunkel über `prefers-color-scheme` in **einer** CDP-Verbindung je Bild, Kompakt über die Taste der Statusleiste. Andere Ports nicht anfassen.
4. **Ausgaben** unter `target/agent/gate-2026-11-02/3d-5/` im Worktree (Entscheidung R6): `auto-<schritt>.log`, `gated-{dialog,refs_messages,recipes_erzaehlen,recipes,templates,trainer_create}-<k>.log` (1), `kopflos-<k>.{log,json,out}` (2), `pt-erzaehlen/`, `hg-erzaehlen/`, `pt-trainer-begegnung/`, `pt-flag/` (2), `web-*.json`/`.txt`, `mcp-bridge-*` und die Bilder `blick-{figur,dialog,dialog-geteilt,tuer,trainer}-{hell,dunkel,kompakt}.png` (3). Den Ordner vor `git worktree remove` sichern.
5. **Last:** `uptime` vor und nach jedem Lauf; die Maschine teilen sich bis zu vier Orchestratoren, alle Zeiten sind darum **unter Last** (Zeitgrenzen gelten nur mit `G4_TIMING_STRICT=1`). Release-Bauten einzeln, die drei Spiele in (1) und (2) nacheinander, nie parallel.

## (1) Automatik

Ergebnis: folgt nach dem UI-Test

- [ ] 1. **`scripts/gate.sh` als Ganzes:** offen, vom Hauptorchestrator auf main nach dem Merge (R8). Die Schritte, die 3d.5 berührt, laufen hier einzeln (Punkt 2).
- [ ] 2. **Schritte von `scripts/gate.sh`, die 3d.5 berührt**, einzeln aus dem Worktree: `cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test -p g4-kernel --all-features` (ohne `G4_*`), `cargo test -p studio --all-features --lib` (ohne `G4_*`), `cargo test -p studio --test mcp_stdio` (ohne `G4_*`), `bash scripts/check-tokens.sh`, `bash scripts/check-styles.sh`, `bash scripts/check-depth.sh`, `pnpm test`, `pnpm build`. Nicht einzeln: `cargo deny`, „dist has no bridge“, `g4emu` Integration, gated Tests der übrigen Phasen.
- [ ] 3. **Gated Tests** im Release-Profil, je Projekt nacheinander mit `--nocapture --test-threads 1`: `--test dialog` (`vanilla_dialogs_of_a_town_resolve`: Dialoge von Zweiblattdorf bzw. Neuborkia, dann jede Figur des Spiels, keine unlesbare, Platin mit mindestens einer geteilten Nachricht; Zeilen `dialog <Spiel> objects=… local=… messages=… shared=… ms=…` und `dialog-all <Spiel> objects=… unreadable=… shared_pairs=… shared_figures=… ms=…`), `--test refs_messages` (`vanilla_message_refs_count_the_script_messages`: Zeilen `refs <Spiel> messages=… warm_ms=…` und `refs <Spiel> message <Archiv>/<Index> refs=…`), `--test recipes_erzaehlen` (gated: `vanilla_new_figure_message_equals_the_input` mit **Nachricht im Archiv gleich der Eingabe**, undo/redo; `vanilla_doors_round_trip_in_the_town` Türen hin und zurück; `vanilla_trainer_here_in_the_town` Trainer; Zeilen `figure …`, `doors …`, `trainer …` mit Zeiten); dazu `--test recipes`, `--test templates`, `--test trainer_create` unverändert grün (`tests/recipes.rs` mit den fünf kleinen Änderungen aus R19).
- [ ] 4. **`g4emu` Unit:** `cd scripts/g4emu && uv run pytest -q` grün.

Zahlen aus den gated Tests (Release, **unter Last**):

| Spiel | `dialog` Start-Ort (Objekte / lokal / Nachrichten / geteilt / ms) | `dialog-all` (Figuren / unlesbar / geteilte Paare / ms) | `refs` Nachrichten (Zahl / warm ms) | `figure` (apply / undo ms) | `doors` (ms) | `trainer` (ms) | Last |
|---|---|---|---|---|---|---|---|
| Platin DE | | | | | | | |
| HeartGold DE | | | | | | | |
| HeartGold US | | | | | | | |

## (2) Szenario kopflos

Ergebnis: folgt nach dem UI-Test

Release-`studio mcp --project <Kopie> [--allow-irreversible] [--expert]` über JSON-RPC/stdio (Protokoll 2025-11-25), je Spiel nacheinander auf `3d-5-k<spiel>` nach `g4k git init`, auch HeartGold US. Die Eingaben der Rezepte (Platin DE: Zweiblattdorf Header 411, HeartGold: Neuborkia Header 60, freie Kachel aus `get_events`) legt der Doku-Lauf mit den Zahlen ab.

Ohne `--allow-irreversible`:

- [ ] 1. `recipe_list` → **sieben** Einträge: `create_area`, `create_room`, `extend_area`, `empty_region`, `new_figure`, `connect_doors`, `new_trainer_here` (`irreversible` wahr bei `empty_region` und `new_trainer_here`).
- [ ] 2. `recipe_plan new_figure` → `events` 1 mit `look`, `messages` 1; `git status --porcelain` leer.
- [ ] 3. `recipe_apply new_trainer_here` → `needs_confirmation` „Trainer anlegen“; `git status` leer.

Mit `--allow-irreversible --expert` (Scripts übernommen: `g4_call adopt_scripts`, `save`, außen `git add -A && git commit`):

- [ ] 4. `recipe_apply new_figure` → `history.undo` + 1, `report.message`; `get_text_archive` des Header-Archivs zeigt an `report.message` einen Text **gleich der Eingabe**.
- [ ] 5. `undo` → `git status --porcelain` leer (Quelle, Member, `compile-state.json`, Archiv zurück); `redo` → wieder wie nach `apply`.
- [ ] 6. `recipe_apply connect_doors` → ein Schritt (`history` + 1); `get_events` beider Dateien zeigen aufeinander (Warp A → B, Warp B → A), Tür-Kacheln gesetzt.
- [ ] 7. `recipe_apply new_trainer_here` → `created` nennt den Trainer; `history` endet mit „Trainer N angelegt“ und dem Rezept-Label; `data_get` des Trainers zeigt Team und Texte.
- [ ] 8. `dialog_of` der neuen Figur → eine Nachricht, `shared_with` 0.
- [ ] 9. `dialog_of` einer Vanilla-Figur mit `shared_with ≥ 1` (nach R11 nicht in Zweiblattdorf, sondern über alle Figuren des Spiels, Zahl aus `dialog-all … shared_figures=`); `split_message` → `new_index`; `git status` nennt die Quelle (`expanded/scripts/…`, `unpacked/scripts/…`), den Member, `.rotom/status/compile-state.json` und das **eine** Textarchiv (JSON und Member); `undo` → `git status` leer.

Szenarien mit `g4emu` (aus `scripts/g4emu`, Dateien wörtlich, Befehle in [docs/agent-gate.md](agent-gate.md), Abschnitt „Phase 3d.5“):

- [ ] 10. `pt-erzaehlen.yaml` auf `3d-5-szen-pt` (`--set NEUER_HEADER=593 --set S_T1=25 --set L_T1=12 --set TEXT="Hallo, Reisender!"`): `"ok": true`; `expect-map` 415 → neuer Header → 415 → neuer Header (Tür hin und zurück), `figur.png` zeigt „Hallo, Reisender!“ in der Box (Agent + Blick), `expect-battle 25 12` mit Jens.
- [ ] 11. `hg-erzaehlen.yaml` auf `3d-5-szen-hg` (`--set NEUER_HEADER=540 --set S_T1=25 --set L_T1=12 --set TEXT="Hallo, Reisender!"`): wie 10 mit der Treppe von Header 64.
- [ ] 12. `pt-trainer-begegnung.yaml` auf `3d-5-trainer` (`--set NEUER_HEADER=593 --set S_WILD=150 --set L_WILD=5 --set S_T1=25 --set L_T1=12 --set S_T2=133 --set L_T2=14 --set S_GIFT=445`) und `pt-flag.yaml` auf `3d-5-flag` (`--set BASIS=../3d-5-flag.nds --set NEUER_HEADER=593 --set X_FIGUR=54 --set Z_FIGUR=17`): unverändert `"ok": true`.

Zahlen aus (2), Release-`studio mcp` kopflos, **unter Last**:

| Spiel | `recipe_apply` Figur (ms) | `undo` Figur (ms) | `recipe_apply` Tür (ms) | `recipe_apply` Trainer (ms) | `dialog_of` Start-Ort (Objekte / geteilt / ms) | `split_message` (ms) | Szenario (s) | Last |
|---|---|---|---|---|---|---|---|---|
| Platin DE | | | | | | | | |
| HeartGold DE | | | | | | | | |
| HeartGold US | | | | | | | – | |

## (3) Browser-Teil

Ergebnis: folgt nach dem UI-Test

Test-Umgebung auf 1750/1751, Chrome headless 1440 × 900, Seite `http://127.0.0.1:1750/?project=<Roms>/_testkopien/3d-5-web_DSPRE_contents` (Platin DE), danach `3d-5-webhg` (HeartGold DE, Neuborkia) mit demselben Ablauf und `3d-5-mcp` für MCP über die Brücke. Bilder unter `target/agent/gate-2026-11-02/3d-5/`.

- [ ] 1. **Erzählen, Rezept-Leiste:** Zweiblattdorf (Platin, `[data-area-key="w0:1"]`) bzw. Neuborkia (HeartGold, Header 60) in der Schau, „Erzählen“ gedrückt: `[aria-label=Rezepte]` mit **fünf freien** Tasten (`create_area`, `create_room`, `new_figure`, `connect_doors`, `new_trainer_here`, keine `disabled`).
- [ ] 2. **„Neue Figur“ bis zur Zusammenfassung:** Ort, Kachel, Aussehen, Bewegung, Blickrichtung, Text „Hallo, Reisender!“; `[data-draft-count]` gleich `draftCount(recipe_plan)` (Körper der Anfrage mit `window.fetch` mitgeschnitten, `call.sh recipe_plan` mit demselben Körper, selbst zählen); Esc → Assistent zu, `[data-draft-count]` 0, `git -C <Kopie> status --porcelain` leer.
- [ ] 3. **Aufstellen → Ergebnis:** „Figur aufgestellt“, unter „Als Nächstes“ „Dialog bearbeiten“ frei und „Zu einer Geschichte“ gesperrt; „Dialog bearbeiten“ öffnet `[data-dialog-editor]` mit der neuen Nachricht.
- [ ] 4. **Dialog einer Vanilla-Figur ändern:** Figur mit Dialog wählen, im Feld „Dialog“ „Dialog bearbeiten…“, eine Nachricht im `MessageEditor` ändern, Blur → `git -C <Kopie> status --porcelain` nennt **nur** `expanded/textArchives/NNNN.json` und `unpacked/textArchives/NNNN` dieses einen Archivs (nichts unter `scripts`).
- [ ] 5. **⌘Z:** Editor schließen (R38: bei offenem Editor sperrt die Modal-Regel den Store-⌘Z), Fokus in der Seite, Meta+Z → `call.sh dirty` leer, `git status` leer, die Nachricht im Editor beim erneuten Öffnen wieder die alte.
- [ ] 6. **Geteilte Nachricht:** Figur mit `shared_with ≥ 1` (Platin: über alle Figuren des Spiels gesucht, R11) → im Editor die Plakette „Geteilt mit N Figuren“ und „Für diese Figur abspalten“; ein Klick → `git status` nennt die Quelle (`expanded/scripts/…` und `unpacked/scripts/…`), `.rotom/status/compile-state.json` und das Textarchiv (JSON und Member), „Nachricht N ist jetzt nur diese Figur.“, die Plakette der neuen Nachricht fehlt; ⌘Z (Editor zu) → `git status` leer.
- [ ] 7. **Tür verbinden:** eine Tür in Erzählen wählen → unter „Tür“ die Zeile „Gegenstück“ mit „Tür N in <Ort> · führt zurück“ bzw. „führt nicht zurück“; „Tür verbinden…“ öffnet `[data-wizard=connect_doors]` mit Tür A vorbelegt; bis „Verbinden“ und Ergebnis „Türen verbunden“ („Hier spielen“ gesperrt „Kommt mit 3d.9“); ⌘Z → `git status` leer.
- [ ] 8. **Neuer Trainer hier** bis zum Ergebnis „Trainer aufgestellt“: `[data-trainer-barrier]` in der Zusammenfassung sichtbar, Entwurf mit Sprite; `git status` nennt danach den neuen Trainer (Trainerdateien), die Eventdatei und die Scripts.
- [ ] 9. **MCP über die Brücke** (`3d-5-mcp`, Freigabe „Lesen und bearbeiten“): `mcp.sh call recipe_plan '{"recipe":"new_trainer_here",…}'` ohne Frage; `mcp.sh call recipe_apply '{"recipe":"new_trainer_here",…}'` → `[data-agent-question]` „„agent-gate“ möchte: Trainer anlegen (lässt sich nicht rückgängig machen).“ mit „Ablehnen“ / „Zulassen“; „Ablehnen“ → `needs_confirmation` (in Gen4 Studio abgelehnt), `git status` leer.
- [ ] 10. **Konsole und Brückenlog:** kein `console.error`, keine Ausnahme, im Brückenlog kein Fehler und keine Panik.
- [ ] 11. **HeartGold DE** (`3d-5-webhg`, Neuborkia): derselbe Ablauf (1 bis 8) mit Figur und Trainer in Neuborkia; Zahlen und Pfade ablegen.

Nicht im Browser prüfbar: die gepackte App (Abschnitt „Hand“).

## (4) Hand und Blick

Ergebnis: folgt nach dem UI-Test

Beim Nutzer: der Blick auf das Feld „Dialog“, den Dialog-Editor (Hell, Dunkel, Kompakt), den Entwurf mit Sprite und die drei Assistenten (Bilder aus Task 8 Step 5 nach R6, `target/agent/gate-2026-11-02/3d-5/blick-{figur,dialog,dialog-geteilt,tuer,trainer}-{hell,dunkel,kompakt}.png`); die Hand: die `figur.png` der Szenarien lesen (der Text steht lesbar in der Box) und die gepackte App mit einem Dialog-Commit.

### Hand

- [ ] **`figur.png` lesen:** `target/agent/gate-2026-11-02/3d-5/{pt,hg}-erzaehlen/figur.png`: der Text „Hallo, Reisender!“ steht lesbar in der Textbox des Spiels.
- [ ] **Dialog-Commit in der gepackten App:** `pnpm bundle`, App aus dem Finder starten, eine Kopie öffnen, im Dialog-Editor einer Vanilla-Figur einen Satz ändern, „Stand sichern…“; der Commit enthält nur das eine Textarchiv.

### Blick

- [ ] `blick-figur-{hell,dunkel,kompakt}.png`: Inspector einer Figur mit dem Feld „Dialog“ (DS-Vorschau, Zeile, „Dialog bearbeiten…“).
- [ ] `blick-dialog-{hell,dunkel,kompakt}.png`: Dialog-Editor mit mehreren Nachrichten und der Vorschau; `blick-dialog-geteilt-{hell,dunkel,kompakt}.png`: mit der Plakette „Geteilt mit N Figuren“ und „Für diese Figur abspalten“.
- [ ] `blick-tuer-{hell,dunkel,kompakt}.png`: Entwurf von „Tür verbinden“ mit der Zeile „Gegenstück“ im Inspector.
- [ ] `blick-trainer-{hell,dunkel,kompakt}.png`: „Neuer Trainer hier“, Zusammenfassung mit dem Hinweis zur Sperre und dem Entwurf mit Sprite.

## Ergebnis

Ergebnis: folgt nach dem UI-Test

| Punkt | Wer | Ergebnis | Datum |
|---|---|---|---|
| (1) 1 `scripts/gate.sh` grün | Agent | offen: vom Hauptorchestrator auf main nach dem Merge | – |
| (1) 2 Schritte von `gate.sh`, die 3d.5 berührt | Agent | | |
| (1) 3 gated `dialog`, `refs_messages`, `recipes_erzaehlen`, `recipes`, `templates`, `trainer_create` | Agent | | |
| (1) 4 `g4emu` Unit | Agent | | |
| (2) 1 bis 3 ohne Erlaubnis | Agent | | |
| (2) 4 bis 9 mit Erlaubnis | Agent | | |
| (2) 10, 11 Szenarien `pt-erzaehlen`, `hg-erzaehlen` | Agent + Blick | | |
| (2) 12 `pt-trainer-begegnung`, `pt-flag` | Agent | | |
| (3) 1 bis 11 Browser-Teil und MCP über die Brücke | Agent | | |
| Bilder `blick-*.png` | Agent + Blick | | |
| `figur.png` lesen, Dialog-Commit in der gepackten App | Hand | | |

Zahlentabelle (Release-`studio mcp` kopflos aus (2), **unter Last**; in Klammern die Oberfläche über die Debug-Brücke aus (3)):

| Spiel | `recipe_apply` Figur (ms) | `undo` Figur (ms) | `recipe_apply` Tür (ms) | `recipe_apply` Trainer (ms) | `dialog_of` Start-Ort (Objekte / geteilt / ms) | `split_message` (ms) | Szenario (s) | Last |
|---|---|---|---|---|---|---|---|---|
| Platin DE | | | | | | | | |
| HeartGold DE | | | | | | | | |
| HeartGold US | | | | | | | – | |

## Befunde

## Offen

