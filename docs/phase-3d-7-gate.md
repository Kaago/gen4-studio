# Phase 3d.7: Gate prüfen

Stand: 2026-11-03. Branch `phase-3d-7-geschichten-erzaehlen` (Worktree `.claude/worktrees/phase-3d-7`), Basis main `b66c4b06` (3d.5 und 3d.6 Teil A), main `9cfafbe5` (3d.8, 3d.9) vor den App-Tasks hineingemergt (`64a3c05d`, Entscheidung R9), Stand der Umsetzung `ed555b64`, Fix-Welle des abschließenden Reviews `05d26961`, `0aca5574`, `e932ef02`, `7cbddf0d`, Befund U1 des UI-Tests `90883c23`, nicht nach main gemergt. **Vom Agenten bestanden: (1) Automatik mit den gated Kernel-Tests auf Platin DE, HeartGold DE und HeartGold US, (2) die Szenarien `pt-geschichte` und `hg-geschichte` kopflos über MCP und (3) der Browser-Teil auf Platin DE und HeartGold DE (2026-11-03, ein Befund U1, behoben).** Offen sind `scripts/gate.sh` als Ganzes auf main nach dem Merge (wie 3d.5, R8 dort: unter paralleler Last scheitern seine Zeitgrenzen; die Schritte, die 3d.7 berührt, laufen einzeln) sowie Hand und Blick beim Nutzer.

Phase 3d.7 bringt das Erzählen von Geschichten. Im Kernel liegen fünf Rezepte im Rahmen von 3d.2 und 3d.5: „Neue Geschichte“ (`new_story`: eine Variable aus dem Pool, benannt `VAR_STORY_…`, Schritt „Anfang“ mit Wert 0), „Gespräch schaltet weiter“ (`talk_advances`: ein Hüll-Script `if VAR == k then … SetVar k+1` vor dem bisherigen Script der Figur), „Figur erscheint/verschwindet ab Schritt“ (`figure_appears`: ein Sichtbarkeits-Flag aus dem Pool und ein Block im Übergangs-Script des Orts), „Szene beim Betreten“ (`scene_on_enter`: eine Bedingung `var == k` im Level-Script mit einem Script, das einen Text zeigt und weiterschaltet) und „Belohnung einmalig“ (`reward_once`: ab Schritt k einmal ein Gegenstand, gemerkt in einem Flag aus dem Pool). Jedes Rezept ist ein Undo-Schritt, hängt Scripts nur an (nie umgeschrieben), nummeriert seine Nachrichten wie „Neue Figur“ und schreibt den Schritt mit `made` in die Geschichte. Über MCP, `g4k recipe` und die App laufen sie gleich; `story_list` nennt den Pool (`free_vars`) und die Flags einer Geschichte. In der App sind die vier Schritt-Rezepte Modal-Dialoge, erreichbar aus dem Detail einer Geschichte („Schritt anlegen“), aus „Zu einer Geschichte“ nach „Neue Figur“ und aus „Dialog anlegen…“ einer Figur ohne Script; „Neue Geschichte“ läuft über das Rezept und nennt den Pool, der Graph speichert gezogene Knoten ins Beifile, der Inspector nennt unter einem Sichtbarkeits-Flag oder einer Auslöser-Variable die Geschichte und den Schritt.

Roadmap: [docs/ROADMAP.md](ROADMAP.md), Abschnitt „Phase 3d“, Zeile „3d.7“. Spezifikation: [2026-10-30-phase-3d-erzaehlen-design.md](superpowers/specs/2026-10-30-phase-3d-erzaehlen-design.md), §3.3 und §4. Plan: [2026-11-02-phase-3d-7-geschichten-erzaehlen.md](superpowers/plans/2026-11-02-phase-3d-7-geschichten-erzaehlen.md). Formate: [stories.md](formats/stories.md) (Abschnitt „Rezepte“), [recipes.md](formats/recipes.md) (Abschnitt „Rezepte der Geschichten (3d.7)“); Werkzeuge: [mcp.md](mcp.md) (`recipe_plan`, `recipe_apply`, `story_list`, Gruppe „Level-Script-Datei anlegen“).

Gate-Text der Spezifikation (§4, Zeile 3d.7):

> Szenarien `pt-geschichte`/`hg-geschichte` über MCP: Geschichte mit drei Schritten anlegen (eine Variable aus dem Pool, eigener Name); Figur A ansprechen → `expect-var` Schritt 0 → 1, `shot`; Figur B erscheint ab Schritt 1 (vorher `walk-until-blocked` durch ihre Kachel frei, danach blockiert, `expect-at`); Raum betreten startet die Szene (`shot`); Kampf `expect-battle`; Belohnung beim zweiten Ansprechen nicht noch einmal (`expect-flag`); `vanilla_checks` 0 Fehler; Pool um eine Variable und die Zahl der Flags kleiner; jedes Rezept ein Undo-Schritt, ⌘Z bis zum Ausgangsstand lässt `git status` leer

Wer was prüft: Die Spalte „Wer“ sagt **Agent** (die Test-Umgebung aus [docs/agent-gate.md](agent-gate.md): Browser-Fenster mit `studio-bridge`, `call.sh`, `mcp.sh`, dazu `g4k`, `studio mcp` kopflos, `g4emu` und die gated Tests), **Agent + Blick** (der Agent prüft und beschreibt Ort und Zahlen, der Nutzer bestätigt kurz am Bild) oder **Hand** (der Nutzer). Befehle: [docs/agent-gate.md](agent-gate.md), Abschnitt „Phase 3d.7“.

Namen in der Oberfläche (Rollen und `data-*` für die Prüfung):

- **„Schritt anlegen“** im Detail einer Geschichte (Bereich Geschichten, oben im Abschnitt „Schritte“): `[role=group][aria-label="Schritt anlegen"][data-step-recipes]` mit vier sekundären Tasten `[data-story-recipe=talk_advances]` „Gespräch schaltet weiter“, `[data-story-recipe=figure_appears]` „Figur erscheint/verschwindet ab Schritt“, `[data-story-recipe=scene_on_enter]` „Szene beim Betreten“, `[data-story-recipe=reward_once]` „Belohnung einmalig“. Eine Geschichte mit einem Flag als Fortschritt sperrt sie mit „Rezepte brauchen eine Variable als Fortschritt“ (Tooltip und `aria-describedby`), außerhalb eines Textprojekts mit dem Satz des Kerns. Ohne Schritte: „Noch keine Schritte. „Schritt anlegen“ legt sie über Rezepte an.“
- **Dialog eines Schritt-Rezepts** `[data-story-recipe-dialog=<id>]` (Modal, breit, Titel = Name des Rezepts, darunter ein Satz, was es tut): links „Geschichte“ (Auswahl der Geschichten mit Variable, daneben leise „Neue Geschichte…“, wo der Aufruf es anbietet), die Felder des Rezepts und „Name des Schritts“ (Platzhalter = der Name, den das Rezept sonst nimmt); rechts `[data-story-recipe-notes]` „Was das Rezept tut“ mit den Sätzen des Plans, was nicht zurückgeht als Warnung in `[data-story-recipe-irreversible]` („Level-Script-Datei n wird angelegt und bleibt“), eine Ablehnung des Plans als Fehler. Eine primäre Taste „Anlegen“ `[data-story-recipe-apply]` (gesperrt, solange etwas fehlt oder der Plan ablehnt), „Abbrechen“; beim Anlegen „Wird angelegt …“, danach der Toast „Schritt „{Name}“ angelegt“.
- **Felder:** „Ort“ und „Figur“ (`FigureField`: `[data-figure-place]`, `[data-figure-pick]` „Figur {local_id} · Aussehen {n}“, je „Wählen…“; ein Trainer trägt die Plakette „Trainer“ und ist für Gespräch und Belohnung gesperrt mit „Trainer sprechen über ihre Kampftexte“), „Ab Schritt“ bzw. „Schritt“ (`StepField`, „{Wert}: {Name}“), „Text“ mit DS-Vorschau, „Die Figur“ (Segmente erscheint | verschwindet), „Sichtbarkeits-Flag“ bzw. „Flag der Belohnung“ aus dem Pool („Noch {n} Flags frei“), „Gegenstand“ (Picker der Gegenstände).
- **„Neue Geschichte“:** Variante Variable mit der Zeile `[data-pool-free]` „Noch {n} Variablen frei“ (Feld vorbelegt mit der freien Variable); ist keine frei, „Keine freie Variable im allgemeinen Bereich.“ und „Geschichte anlegen“ gesperrt. Untertitel der Zeile „Geschichten“ in der Liste: „{n} Geschichten · {free} Variablen frei“.
- **Wege in die Dialoge:** nach „Figur aufgestellt“ unter „Als Nächstes“ `[data-recipe=story_join]` „Zu einer Geschichte“ **frei** (öffnet „Gespräch schaltet weiter“ mit der neuen Figur); im Feld „Dialog“ einer Figur ohne Script „Dialog anlegen…“ **frei** (dasselbe Rezept mit dieser Figur; ein Schild bekommt keine Taste).
- **Hinweis im Inspector** `[data-story-hint]` (`text/xs`): „Geschichte „X“: {Schrittname}“ unter dem Sichtbarkeits-Flag einer Figur oder der Variable eines Auslösers, wenn eine Geschichte das Flag in `flags` bzw. die Variable als Fortschritt führt; in Erzählen an der Stelle des Felds, in Bauen und Tüfteln unter dem `FlagField`. Ein Auslöser, dessen Variable keine Geschichte führt: „Keine Geschichte führt die Variable dieses Auslösers.“
- **Graph und Schritte:** im Detail lassen sich die Knoten ziehen; 500 ms nach dem Loslassen schreibt das Detail das Beifile `data/stories/NNNN.layout.json` (ein Undo-Schritt „Geschichte „X“ geändert“). Die Plakette der Art eines Schritts aus einem Rezept trägt `[data-story-made]` und den Tooltip „Angelegt von „{Rezept}“ (Studio {Version})“, die Notizen eines Schritts stehen darunter (`[data-story-step-notes]`).

## Vorbereitung

1. **Kopien** unter `$HOME/Projekte/lab/editor/Roms/_testkopien/` per `cp -cR` (Ordner) und `cp -c` (`.nds`) aus den Originalen, die dabei nur gelesen werden; Ordner `3d-7-<zweck>` (die gated Kopien) bzw. `3d-7-<zweck>_DSPRE_contents` (Szenarien) neben `3d-7-<zweck>.nds`:
   - `3d-7-pt` (Platin DE), `3d-7-hgde` (HeartGold DE), `3d-7-hgus` (HeartGold US) für (1); nur gelesen, die Tests schreiben auf `vanilla_copy()` (das Textprojekt entsteht je Testprozess einmal in `target/tmp/story-<ordner>` und wird je Test kopiert). Sie bleiben bis zum Browser-Teil.
   - `3d-7-mess-pt` (Platin DE) für die Messung von `VISIBILITY_ENTRY` (Task 5, R1); nach der Messung gelöscht.
   - `3d-7-szen-pt` (Platin DE) und `3d-7-szen-hg` (HeartGold DE) für (2), frisch je Lauf (der Build ändert die Kopie); nach dem letzten grünen Lauf gelöscht.
   - `3d-7-web` (Platin DE) und `3d-7-webhg` (HeartGold DE) für (3), mit `g4k git init` („Ausgangsstand“), `assign_base`, `adopt_text_project` und einem Commit danach; legt der UI-Test an, nach ihm gelöscht.

   Kein ROM-Inhalt im Repository; `Manual`, `_saves` und die Kopien der parallelen Phasen bleiben.
2. **Werkzeuge:** `cargo build --release -p g4-kernel --bin g4k` und `cargo build --release -p studio --bin studio` für die Szenarien; `tools/` ist der Symlink auf den Haupt-Checkout (nie einchecken). `G4_ROMS_DIR=$HOME/Projekte/lab/editor/Roms` absolut; `export PATH="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$HOME/.cargo/bin:$PATH"` vor jedem Cargo-Aufruf.
3. **Test-Umgebung** für (3) auf den Ports **1770/1771**: `G4_ROMS_DIR=<Roms> G4_VITE_PORT=1770 G4_BRIDGE_PORT=1771 bash scripts/agent/studio-web.sh`; eigenes Chrome-Profil unter `target/agent/`. Andere Ports nicht anfassen.
4. **Ausgaben** im Worktree (nicht im Repo): `target/agent/gate-2026-11-02/3d-7/messung/` (Messung, R1), `target/agent/gate-2026-11-03/3d-7/{pt,hg}-geschichte/` (Szenarien, Bilder und Logs), `target/agent/{pt,hg}-geschichte-run{1,2,3}.log` (Läufe), für (3) `target/agent/gate-<Datum>/3d-7/`. Den Ordner vor `git worktree remove` sichern.
5. **Last:** die Maschine teilen sich bis zu vier Orchestratoren; alle Zeiten sind **unter Last** (Lastmittel 12 bis 18 in den Läufen; Zeitgrenzen gelten nur mit `G4_TIMING_STRICT=1`, die Tests von 3d.7 haben keine).

## (1) Automatik

Ergebnis: **bestanden** (Kernel 2026-11-02 und 2026-11-03, Schritt 2 auf `ed555b64` am 2026-11-03). `scripts/gate.sh` als Ganzes ist **offen** (auf main nach dem Merge).

- [ ] 1. **`scripts/gate.sh` als Ganzes:** offen, auf main nach dem Merge. Die Schritte, die 3d.7 berührt, laufen hier einzeln (Punkt 2).
- [x] 2. **Schritte von `scripts/gate.sh`, die 3d.7 berührt**, einzeln aus dem Worktree auf `ed555b64`, 2026-11-03 von 04:07 bis 05:24, ohne `G4_TEST_PROJECT`, Debug, **unter Last** (Lastmittel 13 bis 21):

  | Schritt | Ergebnis |
  |---|---|
  | `cargo test -p g4-kernel --all-features` | **1896 passed, 0 failed, 1 ignored** (48 Blöcke; Bibliothek 1152 passed in 3054 s, `recipes_story` 42 passed; zusammen 71 min) |
  | `cargo test -p studio --lib` | **682 passed, 0 failed, 1 ignored** (338 s) |
  | `pnpm test` (in `apps/studio`) | **400 Dateien passed, 1 übersprungen; 3852 Tests passed, 1 übersprungen** (1402 s) |
  | `check-styles.sh`, `check-tokens.sh`, `check-depth.sh` | grün („check-styles: ok“, „Alles stimmig.“, „check-depth: ok“) |

  **Nach der Fix-Welle** (2026-11-03, auf `e932ef02` bzw. `7cbddf0d`, unter Last 17 bis 20, Logs `target/agent/fixwave/`): `cargo fmt --check` und `cargo clippy … -D warnings` grün, `recipes_story` 45 passed, Bibliothek `recipes::` 43 passed, `cargo test -p studio --lib` 682 passed / 1 ignored, Styles, Tokens, Depth grün, `pnpm build` grün; `pnpm test` zuerst 3860 passed mit einem unbehandelten Fehler (der Verdrahtungstest des Schritt-Rezepts beantwortete die Textprüfung nicht, die er seit `0aca5574` abwartet; behoben in `7cbddf0d`), danach **401 Dateien, 3860 Tests passed, 1 übersprungen, ohne Fehler**. Nach U1 (`90883c23`): `src/stories`, `StoryRecipeHost.test.tsx`, `src/workbench/text` 29 Dateien / 171 Tests passed, `tsc`, Styles, Tokens grün.

  `cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings` und `pnpm build` liefen vor jedem Commit der Tasks (Berichte der Tasks 1 bis 11); der Commit dieser Datei ändert nur Doku. Nicht einzeln: `cargo deny`, `cargo test -p studio --test mcp_stdio`, „dist has no bridge“, `g4emu` Integration, gated Tests der übrigen Phasen.
- [x] 3. **Gated Kette der Geschichte** (Task 7, `crates/g4-kernel/tests/recipes_story.rs`), im Release je Spiel nacheinander (Wiederholung mit der Belohnung `if !FLAG` und dem Kernel der Fix-Welle unten):

  ```sh
  G4_ROMS_DIR=<Roms> G4_TEST_PROJECT=<Roms>/_testkopien/3d-7-pt \
    cargo test --release -p g4-kernel --all-features --test recipes_story -- vanilla hgss_talk --nocapture --test-threads 1
  ```

  (ebenso `3d-7-hgde` und `3d-7-hgus`). Die beiden Filter nehmen die Tests `vanilla_*` und `hgss_talk_uses_npcmsg_and_setvar`; mit `vanilla` allein läuft der HeartGold-Test des Gesprächs nicht mit. Ergebnis auf allen drei Spielen: **4 passed, 0 failed** (Lauf 2 von Task 7, Filter `vanilla`; Platin 1420 s, HeartGold DE 520 s, HeartGold US 476 s, darin je ein Übernehmen der Scripts, 92 bis 247 s). `vanilla_pt_talk_uses_message_and_setvarfromvalue` überspringt auf HeartGold mit „skipped: …“, `hgss_talk_uses_npcmsg_and_setvar` auf Platin.

  Die Kette: Trainer B (`new_trainer_here`) aufstellen, speichern und **committen** (er steht hinter der Undo-Sperre, R7), dann die Marke und sechs Rezepte, **je genau ein Undo-Schritt** (Tiefe Marke + 6): `new_story` → `new_figure` (A) → `talk_advances` (A, 0 → 1) → `figure_appears` (B, ab 1, erscheint) → `scene_on_enter` (derselbe Header, 1 → 2) → `reward_once` (A, ab 2, Gegenstand 17). Die Schritte der Geschichte sind genau `s0` start, `s1` dialog, `a{f1}` appearance (1), `s2` dialog, `r{f2}` reward (2). Nach dem Speichern nennt `git status` auf allen drei Spielen dieselben **12 Pfade** (Eventdatei, zwei Flag-Entitäten, `data/ids/{flags,stories,vars}.json`, Geschichte und Beifile, Variablen-Entität, Level-Script-JSON, Scriptdatei, Textarchiv); keine neue Level-Script-Datei (die Startorte haben eine eigene). **Undo bis zur Marke und Speichern: `git status` leer.** Danach `scripts::compile` ohne Problem, `build::build` schreibt `build/out.nds`, `checks::Runner::run` über `Group::ALL` **0 Fehler**.

  | Spiel | Header | Variable | Flags (erscheint, Belohnung) | B / A (`local_id`) | Pool Variablen | Pool Flags | Prüfungen (Befunde, Fehler) |
  |---|---|---|---|---|---|---|---|
  | Platin DE (CPUD) | 411 | 0x4031 | 0x0042, 0x0043 | 7 / 8 | 20 → 19 | 101 → 99 | 111, 0 |
  | HeartGold DE (IPKD) | 60 | 0x4034 | 0x0040, 0x0041 | 10 / 11 | 81 → 80 | 154 → 152 | 145, 0 |
  | HeartGold US (IPKE) | 60 | 0x4034 | 0x0040, 0x0041 | 10 / 11 | 81 → 80 | 154 → 152 | 153, 0 |

  **Wiederholung in der Fix-Welle** (2026-11-03, Release, Filter `vanilla hgss_talk`, gebaut einmal um 05:43 mit `0ce108d8` und dem Kernel von `05d26961`; danach ohne Neubau, also derselbe Stand wie der Commit), Logs `target/agent/gate-2026-11-03/3d-7/rerun-{hgus,pt,hgde}.log`: **5 passed, 0 failed** auf allen drei Spielen (HeartGold US 2409 s, Platin 1757 s, HeartGold DE 1175 s, unter Last 9 bis 19); dieselben 12 Pfade, `git_after_undo=[]`, Pool wie oben, Prüfungen 111 / 145 / 153 Befunde und 0 Fehler, Build 61 s / 89 s / 263 s. Damit hat auch HeartGold US einen Lauf mit der berichtigten Belohnung.

  Der Pool ist vorher und nachher mit einem frisch gebauten Index gezählt (`RefIndex::new().usage` und `flags::pool`); die drei genommenen Nummern sind danach nicht mehr frei. Übersprungen in den Prüfungen nur `script.message-missing` der gemeinsamen Scriptdateien, die kein Header nutzt (28 auf Platin, 25 auf HeartGold), wie bisher.

  ms je Rezept (Release, unter Last; in Klammern die Spanne über alle Läufe von Task 7):

  | Rezept | Platin DE | HeartGold DE | HeartGold US |
  |---|---|---|---|
  | `new_trainer_here` (vor der Marke) | 3620 (3620–12810) | 2639 (2639–3564) | 2741 (2741–3473) |
  | `new_story` | 334 (334–5442) | 351 (351–392) | 310 (305–385) |
  | `new_figure` | 3502 (3502–20734) | 2415 (2415–2757) | 2649 (2075–3361) |
  | `talk_advances` | 450 (328–5101) | 440 (426–440) | 347 (347–391) |
  | `figure_appears` | 1041 (938–9302) | 836 (836–980) | 1108 (921–1108) |
  | `scene_on_enter` | 665 (665–7841) | 585 (585–860) | 866 (696–866) |
  | `reward_once` | 589 (589–9531) | 527 (527–809) | 1747 (614–1747) |
  | Undo aller sechs | 555 (555–1367) | 466 | 1056 |
  | `compile` / `build` / Prüfungen | 335 ms / 44,4 s / 2,3 s | 295 ms / 47,8 s / 2,0 s | 325 ms / 56,8 s / 1,9 s |

- [x] 4. **Gated Gespräch je Familie** (Task 3, Debug, nach der Fix-Runde `027cdd2e`): `vanilla_pt_talk_uses_message_and_setvarfromvalue` auf `3d-7-pt`: `talk CPUD header=411 local_id=0 slot=10 previous=6 message=15 apply_ms=2154 undo_ms=2`; `hgss_talk_uses_npcmsg_and_setvar` auf `3d-7-hgde`: `talk IPKD header=60 local_id=0 slot=19 previous=1 message=38 apply_ms=3448 undo_ms=3` (die Hülle mit `NPCMsg`, `CloseMsg`, `SetVar`). In der Wiederholung (1) 3 auch auf `3d-7-hgus`: `talk IPKE header=60 local_id=0 slot=19 previous=1 message=38 apply_ms=1517 undo_ms=1`.
- [x] 5. **`g4k`:** `cli_recipe_list` nennt zwölf Rezepte; `cli_recipe_apply_new_story_prints_the_outcome_and_saves` (Bericht, Speichern, zweite Geschichte auf derselben Variable abgelehnt, keine `0001.json`) und `cli_recipe_apply_figure_appears_asks_only_when_a_level_script_file_is_created` grün (Tasks 5 und 7). Ein voller Lauf des Kernels nach Task 7: 1823 passed, 0 failed, 1 ignored.
- [x] 6. **`g4emu` Unit:** `cd scripts/g4emu && uv run pytest -q`: 393 passed, 7 skipped (Task 11).

## (2) Szenario kopflos

Ergebnis: **bestanden** am 2026-11-03 (Lauf 3, `ed555b64`) auf Platin DE und HeartGold DE: Rückgabe 0, Schlusszeile `{"scenario": …, "ok": true, "headers": […]}`, je 22 Zeilen „Erwartung erfüllt“. Befehle, Platzhalter und Kacheln: [docs/agent-gate.md](agent-gate.md), Abschnitt „Phase 3d.7“. Release-`studio mcp` und `g4k`, `mcp: [--allow-irreversible, --expert]`, eine MCP-Sitzung je Szenario. Lauf 3 beider Szenarien parallel: Platin 313 s, HeartGold 320 s, Last 16,8 → 12,3.

Die Bearbeitung: `assign_base`, `apply_header_patch`, `adopt_text_project`, `create_area` (Vorlage 342 bzw. 33), Scriptdatei und Textarchiv, Geschenk-Figur (Script 2, `GivePokemon`/`GiveMon` S_GIFT Lv. 100, ohne Flag), `connect_doors` (415 bzw. 64 Warp 0 ↔ neue Tür), `new_figure` (A), `new_trainer_here` (B, Blick nach Süden, Sicht 3), `create_room`, `save`; dann `free_flag var` → `new_story` „Der Brief“ (`VAR_STORY_DER_BRIEF`), `talk_advances` (A, ab 0), `free_flag flag` → `figure_appears` (B, Schritt 1, erscheint), `scene_on_enter` (Raum, Schritt 1), `free_flag flag` → `reward_once` (A, ab 2, Gegenstand 17), je mit `history` + 1; am Ende `free_flag`, `story_list`, `story_get`, `run_checks`, `save`, Build.

| Gate-Punkt | Platin DE | HeartGold DE |
|---|---|---|
| Geschichte über MCP, eine Variable aus dem Pool, eigener Name | `VAR_STORY_DER_BRIEF` = 0x4031; `story_get` 5 Schritte: `s0` start (0), `s1` dialog (1), `a66` appearance (1), `s2` dialog (2), `r67` reward (2) | 0x4034; dieselben Schritte (`a64`, `r65`) |
| Jedes Rezept ein Undo-Schritt | `history` Undo-Länge 8 → 9 → 10 → 11 → 12 → 13 | ebenso |
| Pool: eine Variable weniger, Flags weniger | Variablen 20 → 19, Flags 101 → 99 (`free_flag` und `story_list.free_vars`) | 81 → 80, 154 → 152 |
| `vanilla_checks` 0 Fehler | `run_checks` 0 Fehler (111 Befunde: 101 Warnungen, 10 Hinweise, alle Vanilla-Texte); `g4k text check` `schema=current`, `differs=0`; `g4k check` `warp.dest-warp` 0, `event.trainer-missing` 0, `event.script-missing` 1 (Vanilla: Schild 1 der Eventdatei 0044) | 0 Fehler (145: 133 Warnungen, 12 Hinweise); `differs=0`; `g4k check` 0 / 0 / 0 |
| B erscheint ab Schritt 1: vorher durch ihre Kachel frei | nach der Ankunft V 0, Bs Flag 66 = 1; `walk-until-blocked up 6` von (54, 21) → (54, 17), Bs Kachel, 4 Schritte, `expect-at` | V 0, Flag 64 = 1; von (15, 16) → (15, 13), 3 Schritte |
| A ansprechen → `expect-var` 0 → 1, `shot` | V 1; `gespraech.png` „Bring den Brief ins Ha…“ (der Text läuft beim Bild noch) | V 1; ebenso |
| Raum betreten startet die Szene, `shot` | Header 594; `szene.png` „Der Brief ist…“; V 2 | Header 541; ebenso; V 2 |
| B danach da: blockiert, Kampf `expect-battle` | nach dem Wiedereintritt Flag 66 = 0; der Weg nach oben endet bei (54, 20), B sieht den Spieler auf 3 Kacheln (`expect-at`, R8); `expect-battle 25 12`, `kampf.png` „Eine Herausforderung von Wanderer Jens!“; `win-battle` | Flag 64 = 0; der Weg nach Westen endet bei (15, 16); `expect-battle 25 12`; `win-battle` |
| Belohnung beim zweiten Ansprechen nicht noch einmal (`expect-flag`) | `belohnung.png` „Danke! Nimm das.“, Flag 67 = 1, V 2; zweites Ansprechen `belohnung-2.png` „Hallo, Reisender!“ (As eigener Text), Flag 67 weiter 1 | Flag 65 = 1; zweites Ansprechen „Hallo, Reisender!“, Flag weiter 1 |
| ⌘Z bis zum Ausgangsstand lässt `git status` leer | nicht im Szenario: kopflos belegt durch die gated Kette (1) 3 (`undo` bis zur Marke, `git status` leer, alle drei Spiele); in der Oberfläche Punkt (3) 10 | – |

Header-Folge 415, 593, 594, 593 (Platin) bzw. 64, 540, 541, 540 (HeartGold). Bilder unter `target/agent/gate-2026-11-03/3d-7/{pt,hg}-geschichte/`: `geschenk.png`, `gespraech.png`, `szene.png`, `kampf.png`, `belohnung.png`, `belohnung-2.png`, `last.png`; vom Agenten gelesen (Agent + Blick). Logs `mcp.jsonl`, `mcp-stderr.log`, `play.jsonl`, `results.json`, `check.log`.

Die Läufe davor: Lauf 1 scheiterte an `check:` (die Erwartungen `text.too-wide=0` und `event.script-missing=0` waren falsch: Vanilla hat 90 bzw. 114 Zeilen „zu breit“ und auf Platin ein Schild ohne Script); Lauf 2 an der Belohnung (`expect-flag R 1`: „Flag 0x0043 ist 0 statt 1“ bzw. „Flag 0x0041 …“, `belohnung.png` zeigte „Hallo, Reisender!“; Befund E1, R2).

## (3) Browser

Ergebnis: **bestanden** am 2026-11-03 auf Platin DE (`3d-7-web`, alle zwölf Punkte) und HeartGold DE (`3d-7-webhg`, Punkte 1, 3 bis 6, 10, 11), ein Befund (U1, behoben in `90883c23` und auf HeartGold DE wiederholt). Test-Umgebung auf 1770/1771 aus dem Worktree, Seite `http://127.0.0.1:1770/?project=<Roms>/_testkopien/3d-7-web` bzw. `…/3d-7-webhg`, Chrome headless (eigenes Profil `target/agent/chrome-prof`, 1440 × 900, `--use-angle=swiftshader`) über CDP, Erzählen über das Segment „Tiefe“. Vorbereitung je Kopie: `g4k git init` („Ausgangsstand“), `assign_base`, `adopt_text_project` (HeartGold 3:42 min), außen `git add -A && git commit` („Textprojekt übernommen“). Zahlen gegen `call.sh` (`free_flag`, `story_list`, `story_get`, `recipe_plan`, `history`, `dirty`) und `git status` der Kopie. Bilder unter `target/agent/gate-2026-11-03/3d-7/browser/`.

- [x] 1. **„Neue Geschichte“:** `[data-pool-free]` „Noch 20 Variablen frei“ = `free_flag var` `pool_free` 20, Feld vorbelegt `VAR_UNUSED_0x4031` (0x4031); „Der Brief“ angelegt → Toast „Geschichte „Der Brief“ angelegt“, Verlauf 0 → 1, `story_get` `s0` start „Anfang“ mit `made.recipe` `new_story`; Untertitel „Keine Geschichten · 20 Variablen frei“ → „1 Geschichte · 19 Variablen frei“, `story_list.free_vars` 19. HeartGold DE: 81 → 80, `0x4034`.
- [x] 2. **„Schritt anlegen“:** vier freie Tasten `[data-story-recipe=…]` im Detail; eine Geschichte „Flaggeschichte“ mit `FLAG_UNK_0x0044` als Fortschritt sperrt alle vier (`disabled`), Tooltip und Beschreibung „Rezepte brauchen eine Variable als Fortschritt“, darunter „Noch keine Schritte. „Schritt anlegen“ legt sie über Rezepte an.“ (`s21-flaggeschichte-gesperrt.png`).
- [x] 3. **„Gespräch schaltet weiter“:** Ort Zweiblattdorf (0411), Figur 2, „Ab Schritt“ 0, „Bring den Brief ins Haus!“; `[data-story-recipe-notes]` = `notes` von `recipe_plan` („Figur 2 bekommt den Zweig „ab Schritt 0“ und schaltet auf 1.“, „Script #10 in Scriptdatei 1052, Nachricht 15 in Archiv 554.“); „Anlegen“ → „Wird angelegt …“, Toast „Schritt „Gespräch mit Figur 2“ angelegt“, Verlauf + 1, `s1` dialog. Ein Trainer (Herzhofen 0092, Figur 0 · Aussehen 52) ist in der Figurenwahl mit der Plakette „Trainer“ gesperrt, „Trainer sprechen über ihre Kampftexte“ (`s11-trainer-gesperrt.png`, im Dialog „Belohnung“; dieselbe Sperre `lockTrainers`). HeartGold DE: Neuborkia (0060) Figur 0, Script #19 in Scriptdatei 842, Nachricht 38 in Archiv 542 (wie die gated Kette).
- [x] 4. **„Figur erscheint/verschwindet ab Schritt“:** Figur 3 in Zweiblattdorf, Schritt 1, erscheint, Flag `FLAG_UNK_0x0042` aus dem Pool („Noch 101 Flags frei“ = `free_flag flag`); `a66` appearance (1), Pool danach 100. In Herzhofen 0092 (Level-Script-Datei 897, die 113 Header teilen) steht im Plan „Level-Script-Datei 1124 wird angelegt und bleibt“ in `[data-story-recipe-irreversible]` (Warnung, `blick-schritt-rezept-erscheinen-hell.png`; nicht angelegt). HeartGold DE: Figur 1, `a64`.
- [x] 5. **„Szene beim Betreten“:** Zweiblattdorf, Schritt 1, „Der Brief ist angekommen.“; Plan „Beim Betreten von Header 411 mit Schritt 1 läuft die Szene und schaltet auf 2.“ = `recipe_plan`; `s2` dialog „Szene beim Betreten von Zweiblattdorf“. HeartGold DE: Neuborkia, `s2`.
- [x] 6. **„Belohnung einmalig“:** Figur 2, „Ab Schritt“ 2, Trank (0017) über den Picker, Flag `FLAG_UNK_0x0043` aus dem Pool; Plan = `recipe_plan` („Figur 2 gibt ab Schritt 2 einmal Gegenstand 17.“, „Benennt Flag 0x0043 als FLAG_STORY_DER_BRIEF_BELOHNUNG_2.“, „Script #13 …“); `r67` reward; Pool Flags 101 → 99, Variablen 20 → 19, `story_list.flags` die beiden Flags. **Befund U1:** der Text blieb leer statt „Du erhältst Trank!“ und „Anlegen“ gesperrt, bis ein Text getippt war (im Lauf auf Platin mit „Danke! Nimm das.“ weiter). Nach `90883c23` auf HeartGold DE: nach dem Gegenstand steht „Du erhältst Trank!“, „Anlegen“ frei, `r65`, Flags 154 → 152.
- [x] 7. **Wege:** „Dialog anlegen…“ im Feld „Dialog“ von Objekt 0 im Zimmer (0415, Script 0) öffnet „Gespräch schaltet weiter“ mit dieser Figur; „Neue Geschichte…“ im Dialog legt „Zweite Geschichte“ an (Verlauf + 1) und wählt sie; „Neue Figur“ im Zimmer (3, 6) → „Figur „Hallo, Reisender!“ aufgestellt“, „Als Nächstes“ `[data-recipe=story_join]` „Zu einer Geschichte“ frei, öffnet „Gespräch schaltet weiter“ mit der neuen Figur 1.
- [x] 8. **Hinweis im Inspector:** Figur 3 in Zweiblattdorf: `[data-story-hint]` „Geschichte „Der Brief“: Figur 3 erscheint ab Schritt 1“ in Erzählen und in Bauen unter dem Sichtbarkeits-Flag (`s17-…`, `s18-hinweis-bauen.png`); Trigger 0 (Variable `VAR_TWINLEAF_TOWN_GUITARIS…`) in Erzählen „Keine Geschichte führt die Variable dieses Auslösers.“ (`s20-ausloeser-erzaehlen.png`; in Bauen ohne Satz, dort steht nur das Variablenfeld).
- [x] 9. **Graph:** Knoten „Anfang“ 120 px nach oben gezogen → nach ≤ 1,8 s ein Undo-Schritt „Geschichte „Der Brief“ geändert“; nach ⌘S `?? data/stories/0000.layout.json` in `git status` mit allen fünf Knoten (`s0` y −127); Tooltip an der Plakette „Start“: „Angelegt von „Neue Geschichte“ (Studio 0.1.0)“.
- [x] 10. **⌘Z bis zum Ausgangsstand:** Platin: neun Schritte (zwei Geschichten mehr, Figur, Layout) mit Meta+Z zurück, Verlauf 0, ⌘S → `call.sh dirty` `[]`, `git status` **leer** (keine Level-Script-Datei angelegt, die Warnung aus 4 blieb Plan). HeartGold DE: fünf Schritte gespeichert (16 Pfade), Meta+Z × 5, ⌘S → `git status` **leer**.
- [x] 11. **Konsole und Brückenlog:** keine Fehler außer `favicon.ico` 404, WebGL-Hinweisen „GPU stall due to ReadPixels“ und erwarteten Ablehnungen: nach dem ⌘Z, das die offene Geschichte entfernt, lehnt die Brücke `story_get` und `thread_graph` mit „Unbekannte ID: STORY_DER_BRIEF.“ ab (HeartGold: je 10 in 0,8 s bei fünf Schritten, bei einer neuen Geschichte je 1; die Seite zeigt danach „Noch keine Geschichten.“). Brückenlog ohne Fehler.
- [x] 12. **Bilder** unter `target/agent/gate-2026-11-03/3d-7/browser/`: `blick-schritt-rezept-gespraech-hell.png`, `blick-schritt-rezept-erscheinen-hell.png` (mit der Warnung), `blick-schritt-rezept-szene-hell.png`, `blick-schritt-rezept-belohnung-hell.png` (vor U1, mit getipptem Text), `blick-pool-hell.png`, `blick-geschichte-schritte-dunkel.png`, dazu `hg-blick-*` (HeartGold DE, Belohnung mit „Du erhältst Trank!“) und die Schritte `s00` bis `s30`, `befund-u1-belohnung-ohne-vorschlag.png`. Vom Agenten gelesen; Kompakt nicht aufgenommen.

## (4) Entscheidungen

Entscheidungen der Umsetzung, die vom Plan abweichen oder ihn festlegen (aus dem Ledger des Steuer-Agenten):

| Nr. | Entscheidung | Grund und Beleg |
|---|---|---|
| R1 | `VISIBILITY_ENTRY` = `on_transition`: der Sichtbarkeits-Block läuft aus dem Übergangs-Script des Orts. | **Gemessen** (Task 5, 2026-11-02) auf `3d-7-mess-pt`: eine Kopie von `pt-flag.yaml` mit `script Main #1: SetFlag FLAG_SZENARIO_FIGUR; End` als `on_transition` von Header 593 (Level-Script 909 der Vorlage 342). Beim **ersten** Betreten ist Flag 66 schon 1 (`expect-flag 66 1` bei Frame 1503, Header 593 ab Frame 1353), die Figur fehlt, `walk-until-blocked up 6` erreicht (54, 17) nach 4 Schritten (durch die Kachel der Figur). `on_transition` läuft also vor dem Aufstellen der Objekte. `pt-flag.yaml` (3d.1) ist die Kontrolle mit gelöschtem Flag (die Figur steht). Bilder `target/agent/gate-2026-11-02/3d-7/messung/erstes-betreten.png`, `weg.png`. Das Szenario (2) bestätigt es auf beiden Spielen. |
| R2 | Die Belohnung prüft ihr Flag mit `if !FLAG then`, nicht mit `if FLAG == 0 then`. | Die Annahme des Plans und von Task 2 („kompiliert in beiden Familien“) stimmte nur für den Compile: rotom 0.7.0 übersetzt `FLAG == 0` in `CompareVarValue <Nummer des Flags>, 0`, das Spiel liest eine Nummer unter 0x4000 als Wert, `67 == 0` ist nie wahr. **Im Spiel gefunden** (Lauf 2 der Szenarien, Befund E1), behoben in `0ce108d8` (Vorlagen `story_reward_{pt,hgss}.rotom`, Test `a_flag_is_tested_by_its_truth_not_compared`), im Spiel bewiesen auf beiden Familien. Gespräch (`if VAR == k`), Sichtbarkeit (`if VAR >= k`) und Szene vergleichen Variablen ab 0x4000 und sind richtig. |
| R3 | Ein **voller Beutel wird nicht geprüft**; der Gegenstand nur gegen die Zahl der Gegenstände (Nummern 1 bis Zahl − 1, Gegenstand 0 ist keiner: Platin 1 bis 467, HeartGold 1 bis 536). | `GiveItem` scheitert bei vollem Beutel still, das Flag ist trotzdem gesetzt, die Belohnung vertan; dokumentiert in [stories.md](formats/stories.md) „Belohnung“. Ein Test des Beutels im Script wäre ein eigener Posten. |
| R4 | Die Hülle eines Gesprächs heißt `gespraech_{local_id}_{n}` (nach ihrem Slot), wie die Belohnung `belohnung_{local_id}_{n}`; der Plan hatte `gespraech_{local_id}_{value}`. | Zwei Geschichten, die über dieselbe Figur vom selben Wert weiterschalten, brauchen zwei Hüllen; der zuletzt angelegte Zweig prüft zuerst und springt zur vorigen (Task 3, Fix-Runde 1, `027cdd2e`). Die Vorlagenzeile im Plan (Zeile 134) ist nicht nachgezogen. |
| R5 | Eine neue Level-Script-Datei (der Header teilt seine Datei, hat keine oder nur eine binäre) **bleibt nach ⌘Z**. | Sie entsteht vor der Marke wie bei `empty_region`; der Header zeigt nach ⌘Z wieder auf die alte Datei. Angekündigt in `plan.irreversible` („Level-Script-Datei n wird angelegt und bleibt“) und `Outcome.created`. Scheitert ein späterer Schritt, bleibt sie ebenfalls (verwaist, Offen). |
| R6 | MCP entscheidet die Frage bei `figure_appears` und `scene_on_enter` mit dem Store der Sitzung (`classify_in` über `is_irreversible_in`): nur eine neue Level-Script-Datei fragt. Ein abgelehnter Plan fragt sicherheitshalber (fail closed); in der App plant `Call::recheck` unter dem Zug des Agenten noch einmal. `g4k recipe apply` öffnet den Store zuerst. | Task 5 (Fix-Runde 1, `dc61c830`) und Task 8 (Fix-Runde 1, `75eccd8c`). Ohne Store (`classify`) bleibt die Antwort konservativ „Level-Script-Datei anlegen“. Das Fenster zwischen Recheck und Anwenden für Änderungen des Nutzers in der App ist in [mcp.md](mcp.md) beschrieben. |
| R7 | In der gated Kette steht Trainer B **vor** der Marke, gespeichert und committet; der „Ausgangsstand“ des Gates ist der Stand nach dem committeten Trainer. | `new_trainer_here` legt den Trainer hinter einer Undo-Sperre an, ⌘Z überquert sie nie (`Store::undo`); mit dem Trainer mitten in der Kette könnte `git status` nie leer werden (Task 7). Die Szenarien stellen B mitten in der Kette auf, sie machen nichts rückgängig. |
| R8 | Szenarien: Tür des Raums auf Platin (50, 22) statt (46, 18) des Plans, auf HeartGold (22, 19); „B danach blockiert“ ist der Halt durch Bs Blick auf 3 Kacheln vor dem Kampf, nicht ein Stoß gegen Bs Kachel. | (46, 18) und jeder Weg dahin ist hohes Gras: ein wilder Kampf (der Spieler hat das Geschenk) bräche den Lauf. B blickt die Spalte hinab, der Blick fängt immer zuerst: vorher (54, 17) bzw. (15, 13) erreicht, nachher Halt bei (54, 20) bzw. (15, 16) und `expect-battle` (Task 11). |
| R9 | main `9cfafbe5` (3d.8, 3d.9) ist vor den App-Tasks in den Branch gemergt (`64a3c05d`, ohne Konflikt). | Die Tasks 8 bis 10 bauen auf `App.tsx`, `WizardHost`, `EventEditor` und dem MCP-Katalog nach 3d.8 und 3d.9 auf; spätere Konflikte entfallen. |
| R10 | Die Schritt-Rezepte sind Modal-Dialoge (`StoryRecipeHost`), eingehängt **in** `DataAreaHost` nach der `Shell`, nicht neben `WizardHost`. | Der Gegenstands-Picker (`PickField choice="item"`) braucht den Datenkontext des Bereichs Pokémon (Task 10). Der Entwurf eines Schritt-Rezepts geht nicht in die Szene (das Modal deckt sie). |
| R11 | Weitere Festlegungen: `scene_on_enter` nennt den Ort als `Use::Map`, wenn das Register den Header nennt (Task 6/7); `snapshot_script` deckt die Level-Script-JSON ab, kein eigener Dateischritt (Task 5); das Beifile hält beim ersten Ziehen alle Knoten (der Graph bleibt stabil; Task 9); der Undo-Test der gespeicherten Belohnung läuft auf dem Textprojekt, nicht auf einem DSPRE-Ordner (die Rezepte brauchen ein Textprojekt; Task 4). | Ledger, Tasks 4 bis 9. |

## Hand und Blick

Ergebnis: die Bilder der Szenarien und des Browser-Teils sind aufgenommen und vom Agenten gelesen. Bestätigung und Hand liegen beim Nutzer und sind **offen**.

### Hand

- [ ] **Mit einem echten Spielstand spielen:** eine Geschichte in der App aus den fünf Rezepten anlegen (Variable aus dem Pool, Gespräch, Erscheinen, Szene, Belohnung), bauen und in melonDS mit einem eigenen Spielstand spielen; die Szenarien nutzen die Stände `_saves/CPUD-zimmer.sav` und `IPKD-zimmer.sav` (Zimmer, ohne Pokémon) und ein Geschenk mit Lv. 100.
- [ ] **Szenario-Bilder lesen:** `target/agent/gate-2026-11-03/3d-7/{pt,hg}-geschichte/{gespraech,szene,kampf,belohnung,belohnung-2}.png`: die Texte „Bring den Brief ins Haus!“, „Der Brief ist angekommen.“, „Danke! Nimm das.“ und beim zweiten Ansprechen „Hallo, Reisender!“ (Gespräch und Szene sind beim Bild noch im Lauf, die ersten Wörter lesbar).

### Blick

- [ ] Die Dialoge der vier Schritt-Rezepte, die Pool-Zeile und die Schritte im Detail in Hell und Dunkel (Bilder aus (3) 12, `target/agent/gate-2026-11-03/3d-7/browser/blick-*.png` und `hg-blick-*.png`); Kompakt nur von Hand.
- [ ] **`.fig`:** die Frames von 3d.7 sind nicht eingespielt (OpenPencil war in dieser Sitzung nicht verbunden); Anleitung in [design-system/README.md](../design-system/README.md), „Nachtrag 3d.7“.

## Ergebnis

| Punkt | Wer | Ergebnis | Datum |
|---|---|---|---|
| (1) 1 `scripts/gate.sh` grün | Agent | offen: auf main nach dem Merge | – |
| (1) 2 Schritte von `gate.sh`, die 3d.7 berührt | Agent | bestanden: Kernel 1896, Studio-Bibliothek 682, `pnpm test` 3852, Styles, Tokens, Depth (unter Last); nach der Fix-Welle `pnpm test` 3860 ohne Fehler (`7cbddf0d`) | 2026-11-03 |
| (1) 3 gated Kette der Geschichte | Agent | bestanden auf Platin DE, HeartGold DE, HeartGold US: je Rezept ein Undo-Schritt, 12 Pfade, `git status` leer nach Undo, Pool − 1 Variable und − 2 Flags, Build, 0 Fehler; wiederholt mit `if !FLAG` und dem Kernel der Fix-Welle | 2026-11-03 |
| (1) 4 gated Gespräch je Familie | Agent | bestanden auf Platin DE, HeartGold DE und HeartGold US | 2026-11-03 |
| (1) 5, 6 `g4k`, `g4emu` Unit | Agent | bestanden | 2026-11-03 |
| (2) Szenarien `pt-geschichte`, `hg-geschichte` | Agent + Blick | bestanden: `"ok": true`, alle Punkte des Gate-Texts außer ⌘Z (kopflos in (1) 3); Bilder vom Agenten gelesen, Bestätigung des Nutzers offen | 2026-11-03 |
| (3) 1 bis 12 Browser-Teil | Agent + Blick | bestanden auf Platin DE (1 bis 12) und HeartGold DE (1, 3 bis 6, 10, 11); U1 behoben in `90883c23`; Bilder vom Agenten gelesen, Bestätigung offen | 2026-11-03 |
| (4) Entscheidungen R1 bis R11 | Agent | festgehalten | 2026-11-03 |
| Echter Spielstand, Szenario-Bilder | Hand | offen | – |
| Blick der Dialoge, `.fig` | Blick | offen | – |

## Befunde

### Aus der Umsetzung

Jede Task lief durch ein Review; Fix-Runden hatten Task 3 (Hülle nimmt bei jedem Fehler alles zurück, Name nach dem Slot, `027cdd2e`), Task 5 (`g4k` fragt `figure_appears` mit dem Store, `dc61c830`) und Task 8 (abgelehnter Plan fragt, Recheck unter dem Zug, `75eccd8c`); die übrigen Reviews waren ohne wichtigen Befund.

| Nr. | Schwere | Punkt | Befund | Status |
|---|---|---|---|---|
| E1 | Hoch (Produkt) | (2) | `reward_once` gab im Spiel nie etwas: `if FLAG == 0 then` ist in rotom ein Vergleich der Flag-Nummer mit 0, nie wahr; das Script fiel durch zur Hülle des Gesprächs und zum eigenen Text der Figur. Synthetische und gated Tests sahen es nicht (die Quelle kompiliert). | behoben: `0ce108d8` (`if !FLAG then`), R2; Lauf 3 grün auf beiden Spielen |
| E2 | Doku | (2) | Die ersten Erwartungen von `check:` (`text.too-wide=0`, `event.script-missing=0`) waren falsch für Vanilla. | Szenario: nur Codes, die auf Vanilla 0 sind, dazu Platins eine Vanilla-`event.script-missing` |
| E3 | Doku | (1) 3 | Die Reihenfolge des Plans (Trainer mitten in der Kette) lässt kein Undo bis zur Marke zu. | R7 |
| E4 | Doku | (2) | [agent-gate.md](agent-gate.md) sagte, `"ok": true` stehe in `results.json`; es steht in der Schlusszeile des Runners auf stdout (und je Schritt in `play.jsonl`). | Doku: korrigiert (3d.7 in `f99b1880`, 3d.5 in `e932ef02`) |

### Aus dem abschließenden Review

Das Review des ganzen Branches (Opus, `9cfafbe5..f99b1880`) sagte „mit Fixes“; die Fix-Welle und ihr Nach-Review (Opus, alle Punkte behoben, nichts Neues kaputt):

| Nr. | Schwere | Befund | Status |
|---|---|---|---|
| F1 | Wichtig | `beforeStoryStep` speicherte vor einem Schritt-Rezept nur die Scriptdatei, nicht die geänderte Level-Script-Datei, die „Szene“ und „Erscheinen“ umschreiben | behoben: `0aca5574` (`app/storyStepSave`) |
| F2 | Wichtig | der gated Nachweis auf HeartGold US stammte von vor `0ce108d8` (`if !FLAG`) | behoben: Wiederholung (1) 3 auf allen drei Spielen |
| F3 | Klein | „Anlegen“ konnte den Plan einer vorigen Eingabe anwenden | behoben: `0aca5574` (`useDraftPlan.current`) |
| F4 | Klein | „Dialog anlegen…“ und „Zu einer Geschichte“ außerhalb eines Textprojekts frei (der Kern lehnt ab) | behoben: `0aca5574` (gesperrt mit dem Satz des Kerns) |
| F5 | Klein | `StoryRecipeHost` zeigte einen Fehler von `story_list` nicht | behoben: `0aca5574` |
| F6 | Klein | eine nach einem späteren Fehler angelegte Level-Script-Datei blieb unangekündigt | behoben: `05d26961` („Level-Script-Datei n wurde angelegt und bleibt.“) |
| F7 | Klein | eine binäre Level-Script-Datei wird ersetzt, ohne dass der Plan es sagt | behoben: `05d26961` („…; die Einträge der binären Level-Script-Datei m werden nicht übernommen“) |
| F8 | Klein | `FigureField` erkannte Trainer nicht an `trainer_type` | behoben: `0aca5574` |
| F9 | Doku | Undo-Folge ab 8, `"ok": true` im Abschnitt 3d.5 | behoben: `e932ef02` |
| F10 | Test | `pnpm test` meldete einen unbehandelten Fehler im Verdrahtungstest des Schritt-Rezepts | behoben: `7cbddf0d` |

### Aus dem UI-Test

| Nr. | Schwere | Punkt | Befund | Status |
|---|---|---|---|---|
| U1 | Mittel (Produkt) | (3) 6 | „Belohnung einmalig“ zeigte nach dem Gegenstand nicht „Du erhältst {Gegenstand}!“, „Anlegen“ blieb gesperrt, bis ein Text getippt war: `SaysField` reichte das Echo des `MessageEditor` beim Einhängen als Eingabe weiter, der Text wurde "" statt `null`. | behoben: `90883c23` (`SaysField` meldet nur Änderungen, Test `RewardOnceForm.test.tsx`), Nach-Review sauber, auf HeartGold DE wiederholt |
| U2 | Klein (Beobachtung) | (3) 11 | Ein ⌘Z, das die offene Geschichte entfernt, löst einen Schwall abgelehnter `story_get`/`thread_graph` aus („Unbekannte ID“, bis je 10 in 0,8 s); die Seite endet richtig. | offen, zurückgestellt |
| U3 | Klein (Text) | (3) 5 | Der Plan der Szene sagt „Header 411“, der Dialog „Zweiblattdorf“. | offen, zurückgestellt |

## Offen

- **(1) 1 `scripts/gate.sh` als Ganzes:** auf main nach dem Merge.
- **Hand:** mit einem echten Spielstand spielen; die Szenario-Bilder bestätigen.
- **Blick:** die Dialoge in Hell und Dunkel (Bilder aus (3) 12), Kompakt von Hand.
- **`.fig`:** die Frames von 3d.7 (OpenPencil nicht verbunden; Anleitung im Nachtrag 3d.7 des Design-Systems).
- **Zurückgestellt aus den Reviews und dem UI-Test:** ein auf leer gelöschter Text der Belohnung kehrt nicht zum Vorschlag zurück und sperrt „Anlegen“; die Formulare „Erscheinen“ und „Szene“ haben keine eigenen Komponententests (die Belohnung seit U1), kein Test, dass „Gespräch“ und „Szene“ Eingaben weiter melden; der Regressionstest von R2 prüft den Vorlagentext, nicht den Bytecode (`CheckFlag`); die Pool-Zeile zählt die vorgeschlagene Variable mit; ein zweites Ziehen während eines laufenden Layout-Schreibens kann verloren gehen; ein Kompilierfehler nach einer angelegten Level-Script-Datei kommt als Text, nicht als Liste von Problemen; der Fehler von `story_list` steht im Dialog nur, wenn keine Geschichte vorgegeben ist; ein Textprojekt, das eine Migration braucht, sperrt die Wege mit „Kein Textprojekt …“; nach einem Teilfehler wird die neue Level-Script-Datei nicht neu gelesen; U2 und U3.
- **rotom upstream:** `if FLAG_X == n` kompiliert still zu einem Vergleich mit der Nummer (R2); die Notiz aus 3d.1 („`if FLAG_X == 1 then` kompiliert“) stimmt, prüft aber das Flag nicht. Meldung an rotom offen.
- **Testkopien:** `3d-7-pt`, `3d-7-hgde`, `3d-7-hgus`, `3d-7-web`, `3d-7-webhg` nach dem UI-Test gelöscht (2026-11-03); `target/agent/` des Worktrees bleibt für den Hauptorchestrator.
- **Merge-Hinweise:** `git merge-tree --write-tree main HEAD` gegen main `123ca8bb` (2026-11-03) meldet Konflikte in `apps/studio/src-tauri/src/mcp/access.rs`, `apps/studio/src-tauri/src/mcp/tools.rs`, `design-system/README.md`, `docs/ROADMAP.md`, `docs/agent-gate.md`, `docs/mcp.md` (Anhängepunkte); gesammelt lösen, die `.fig` von main behalten. MCP-Katalog: 3d.7 fügt kein Werkzeug hinzu.
