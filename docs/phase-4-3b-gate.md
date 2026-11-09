# Phase 4.3b: Gate prüfen

Stand: 2026-10-29. Branch `phase-4-3b-textprojekt-quelle` (Worktree `.claude/worktrees/phase-4-3b`, Basis `a6933b3` mit 4.3a), nicht nach main gemergt. Gate vom Agenten bestanden: (1) Automatik mit den gated Tests im Release-Profil auf Platin DE, HeartGold DE und HeartGold US und (2) Szenario kopflos über `g4k` und `studio mcp` auf dem Stand `9775467`; (3) Browser-Teil und MCP über die Brücke im UI-Test am 2026-10-28 auf `3578645`, seine acht Befunde (U1 bis U8) behoben in `fdb3483` und `9775467` und U1, U2, U3, U8 im Gate-Lauf nachgeprüft. Ein Befund des Gate-Laufs (G1, ein gated Test unter `scripts/gate.sh`) ist in `891e1e3` behoben; danach ist `scripts/gate.sh` grün auf `891e1e3` in 8 324 s (Abschnitt „Ergebnis“). Die Nachträge I1 bis I3 aus dem Review von 4.3a sind hier behoben (Abschnitt „Nachträge aus 4.3a“). Unabhängige Abnahme am 2026-10-29 auf `6270c44` (main mit 4.3b) mit fünf Befunden (A1 Mittel, A2 bis A5 gering), behoben auf dem Branch `abnahme-4-3b` (Abschnitt „Abnahme (2026-10-29)“). Beim Nutzer offen: die DSPRE-Gegenprobe auf einem eingerichteten Ordner, der Fokuswechsel im gepackten Tauri-Fenster, die Fortschrittsphasen beim Einrichten (Hand), der Blick auf die Bilder des UI-Tests und die `.fig`; Abschnitte „Hand“ und „Blick“.

Phase 4.3b macht das Textprojekt zur Quelle des Studios: In einem Projekt mit `g4-project.json` liest und schreibt der Store die Entitäts-JSON unter `data/` (`Location::Entity`, `EntitySource`), die Member unter `unpacked/` sind nur noch Build-Zwischenstand. Ein DSPRE-Ordner lässt sich über „Als Textprojekt einrichten…“ (Technik › Textprojekt, Hinweis beim Öffnen, Häkchen im Assistenten „Neues Spiel aus ROM“, MCP `adopt_text_project`, `g4k text adopt`) umstellen und über „Zurücknehmen…“ (`g4k text revert`) wieder zurücknehmen. DSPRE bleibt parallel nutzbar: beim Öffnen und beim Fensterfokus gleicht das Studio über die Fingerabdrücke in `.g4/build/state.json` (`sync`) ab, übernimmt einseitige Änderungen und meldet sie, zeigt beidseitige als Konflikt mit Diff und Wahl je Entität in Technik › Textprojekt und überschreibt nie still. Symbolische IDs (ID oder Nummer) gelten in `g4-world.json`, `g4-checks.json` (jetzt mit `version: 1`), den Rezepten der Vorlagen, den MCP-Argumenten und in Scripts (über versteckte ID-Dateien für rotom, siehe „Hinweise“); „ID umbenennen“ zieht jeden Verweis als ein Undo-Schritt nach. Roadmap: [docs/ROADMAP.md](ROADMAP.md), Abschnitt „Phase 4“, Punkt „4.3b Textprojekt als Quelle“. Entscheidungen: [entscheidungen.md](research/2026-10-27-phase-4/entscheidungen.md), Abschnitt „Textprojekt (Gutachten A, 4.3)“ (besonders „DSPRE parallel“, „Umstellung eines DSPRE-Ordners“, „Referenzindex und Scripts“, „Kollisionen, Umbenennen“). Plan: [2026-10-28-phase-4-3b-textprojekt-quelle.md](superpowers/plans/2026-10-28-phase-4-3b-textprojekt-quelle.md). Format: [docs/formats/text_project.md](formats/text_project.md); Werkzeuge: [mcp.md](mcp.md) (`text_project_status`, `adopt_text_project`, `external_changes`, `resolve_external_conflict`, `rename_id`).

Gate-Text der Roadmap (Punkt 4.3b):

> Gate (über die Test-Umgebung und MCP): Spezies im Studio ändern, nur `data/species/NNNN.json` ändert sich; einen Member „von DSPRE“ ändern, beim Fokus erkannt und übernommen; beide Seiten ändern, Konfliktliste, beide Wahlen wirken; ein MCP-Aufruf mit `SPECIES_PIKACHU` wirkt wie mit `25`; ROM in melonDS spielbar, `vanilla_checks` 0 Fehler; `scripts/gate.sh` grün.

Wer was prüft: Die Spalte „Wer“ sagt **Agent** (die Test-Umgebung aus [docs/agent-gate.md](agent-gate.md): Browser-Fenster mit `studio-bridge`, `call.sh`, `mcp.sh`, dazu `g4k`, `studio mcp` kopflos und die gated Tests), **Agent + Blick** (der Agent prüft und beschreibt Ort und Zahlen, der Nutzer bestätigt kurz am Bild) oder **Hand** (der Nutzer). Befehle: [docs/agent-gate.md](agent-gate.md), Abschnitt „Phase 4.3b“.

Namen in der Oberfläche (Rollen und `data-*` für die Prüfung):

- **Technik › Textprojekt** (`[data-technik=textprojekt]`), Untertitel in der Liste „DSPRE-Layout“, „Abgeglichen“, „1 Konflikt“ / „{n} Konflikte“, „1 fehlerhafte Datei“ / „{n} fehlerhafte Dateien“, „Wird eingerichtet …“. DSPRE-Layout: Kopftext und „Als Textprojekt einrichten…“ (primär; ohne Basis-ROM gesperrt mit „Diesem Projekt ist keine Basis-ROM zugeordnet.“, während ein Agent einrichtet `role=status` „Wird von einem Agenten eingerichtet …“). Textprojekt: „Textprojekt, Schema-Version 1. Zuletzt abgeglichen …“, „Jetzt abgleichen“, „Zurücknehmen…“, Abschnitt „Zuletzt übernommen“ (Zeilen wie „Attacke 0033 · MOVE_TACKLE“ mit Plakette „aus DSPRE“, sonst „Nichts von außen geändert.“).
- **Konflikte** `[data-sync-conflicts]` „Auf zwei Seiten geändert“, je Einheit `[data-conflict=<schlüssel>]` (etwa `data/moves/0038.json`) mit Plakette „Konflikt“ oder „Ungespeichert“; aufgeklappt die Tabelle Feld / Studio / DSPRE (bei ungespeicherten Werten heißt die Spalte „Studio (ungespeichert)“), die Tasten „Studio-Fassung behalten“ und „DSPRE-Fassung übernehmen“, bei ungespeicherten Werten die Warnung „„DSPRE-Fassung übernehmen“ verwirft die ungespeicherten Änderungen an diesem Eintrag.“
- **Fehlerhafte Dateien** `[data-sync-broken]`, je Zeile `{datei}:{zeile}:{spalte}: {satz}` (ohne Prüfcode, seit U6).
- **Meldungen:** „1 Änderung aus DSPRE übernommen“ / „{n} Änderungen …“ (+ „Anzeigen“), „1 Konflikt zwischen Studio und DSPRE“ (+ „Lösen…“), „1 Projektdatei ist fehlerhaft“, „Erst die Konflikte in Technik › Textprojekt lösen.“ (beim Bauen, ersetzt die Konfliktmeldung an derselben Stelle), „Textprojekt eingerichtet: 6.574 Dateien in 57 s.“, „Textprojekt zurückgenommen. Sicherung: …“, „ID umbenannt: … → …. ⌘Z nimmt es zurück.“, „Das Projekt ist schon ein Textprojekt.“
- **Hinweis beim Öffnen** eines DSPRE-Ordners: „… DSPRE öffnet den Ordner weiter.“ mit „Einrichten…“, ohne Basis-ROM „… Dafür braucht es die Basis-ROM.“ mit „Basis-ROM zuordnen…“, dazu „Nicht mehr fragen“; er bleibt stehen, bis er beantwortet ist oder das Projekt kein DSPRE-Ordner mehr ist.
- **Dialoge:** „Als Textprojekt einrichten?“ (Prüfliste ✓ „Alles gespeichert“, ✓ „Basis-ROM gefunden: …“, (i) „Scripts werden dabei übernommen“; „Abbrechen“ / „Einrichten“; im Lauf „Wird eingerichtet …“), „Textprojekt zurücknehmen?“ (mit Ungespeichertem „Erst speichern.“), „ID umbenennen“ (Feld „Neue ID“, Plan „2 Verweise in 2 Dateien werden nachgezogen.“ mit den Pfaden, Ablehnungen „… gibt es schon (Nummer 2).“ und „Die neue ID muss mit TRAINER_ beginnen …“).
- **IDs in Köpfen:** Trainer, Pokédex und Header zeigen die ID als `[data-entity-id]` in Mono mit „ID kopieren“; „ID umbenennen…“ im Menü „Aktionen“.
- **Assistent:** Kästchen „Als Textprojekt einrichten“ im Schritt „Vorlage“, Zusammenfassung „Original, als Textprojekt“, Phase „Textprojekt einrichten“ im Ablauf.
- **Agent:** Frage `[data-agent-question]` „Als Textprojekt einrichten“ („Agent möchte etwas Unumkehrbares tun“).

## Vorbereitung

So durchgeführt am 2026-10-29 auf dem Stand `9775467` (01:00 bis 05:45):

1. **Frische Kopien** unter `Roms/_testkopien/` per `cp -cR` (Ordner) und `cp -c` (`.nds`) aus den Originalen, die dabei nur gelesen wurden; Ordner `4-3b-<zweck>_DSPRE_contents` und `.nds` `4-3b-<zweck>.nds` direkt nebeneinander:
   - `4-3b-pt` (Platin DE), `4-3b-hgde` (HeartGold DE), `4-3b-hgus` (HeartGold US) für (1); sie werden nur gelesen, die Tests schreiben auf `vanilla_copy()`, die Basis-ROM ist die Nachbar-`.nds`;
   - `4-3b-kpt`, `4-3b-khgde`, `4-3b-khgus` für (2), je mit `g4k base assign` auf die Nachbar-`.nds`; die gebauten ROMs in `4-3b-k<spiel>-out/`;
   - `4-3b-ui` (Platin DE, ohne Basismarke) und `4-3b-mcp` (Platin DE, Basis zugeordnet) für die Nachprüfung von U1, U2, U3 und U8; der Browser-Teil (3) selbst lief im UI-Test am 2026-10-28 auf `4-3b-ui`, `-ui2`, `-web`, `-mcp`, `-wiz`.

   Alle `4-3b-*`-Kopien sind nach dem Gate gelöscht; `Manual` und `_saves` bleiben.
2. **Werkzeuge:** `cargo build --release -p g4-kernel --bin g4k`, `cargo build --release -p studio --bin studio` und die Release-Testbinaries (`--no-run`), im Worktree ohne rotom-Download (`-p` ging). `tools/` ist der Symlink auf den Haupt-Checkout (nie einchecken). `G4_ROMS_DIR` absolut, `G4_BASE_ROM_DIR=<Roms>/_testkopien`. Vor dem Lauf `target/debug/incremental` gelöscht (Platte: 6 GB frei, danach 16 GB).
3. **Test-Umgebung** für die Nachprüfung: `G4_ROMS_DIR=<Roms> G4_VITE_PORT=1580 G4_BRIDGE_PORT=1581 bash scripts/agent/studio-web.sh`, Browser-Fenster der Desktop-App (1024 × 768), Fokus über `window.dispatchEvent(new Event("focus"))`.
4. **Ausgaben** unter `target/agent/gate-2026-10-29/4-3b/` im Worktree: `gated-{source,studio,start}-<k>.log` (1), `kopflos-k<spiel>.log` (2; `kopflos-kpt-lauf1.log` ist der erste vollständige Platin-Lauf mit knapperem Protokoll), `emu-k<spiel>/` (g4emu-Bilder), `recheck-mcp-adopt*.out`, `build-release.log`; `scripts/gate.sh` nach `target/agent/gate-sh-4-3b.log`. Die Skripte für (1) und (2) (Bash, Python über JSON-RPC/stdio) lagen im Scratchpad. Die Bilder des UI-Tests liegen unter `target/agent/gate-2026-10-28/4-3b/`. Beide Ordner vor `git worktree remove` sichern.
5. **Last:** `uptime` vor und nach jedem Lauf (Spalte „Last“ unten). Zu Beginn lief eine andere Sitzung (1-Minuten-Last 18 bis 27), danach 3 bis 8. Die gated Läufe liefen nacheinander, nie parallel zu (2) oder zum Gate-Skript.

## (1) Automatik

- [x] 1. **Gated Tests** `crates/g4-kernel/tests/textproject_source.rs` auf `4-3b-{pt,hgde,hgus}`, Release, `--nocapture --test-threads 1`: je 9/9 bestanden (Platin DE 798 s, HeartGold DE 938 s, HeartGold US 845 s):
   - `store_on_json_edits_only_the_entity_file_and_builds_like_dspre`: Pikachu Angriff +1 über den Store, gespeichert: unter den Quellen ändert sich nur `data/species/0025.json`; das Textprojekt und der DSPRE-Ordner mit derselben Änderung bauen dieselbe ROM (nicht die Basis). Speichern 9 / 20 / 9 ms, alle 508 Personal-Einträge laden 236 / 1 952 / 184 ms.
   - `simulated_dspre_change_is_adopted_one_sided`: voller Abgleich ohne Aufzeichnung (16 509 / 18 239 / 18 239 Einheiten, 0 nicht im Gleichstand) 51,4 / 36,6 / 38,6 s; Fokus-Abgleich ohne Änderung 86 / 88 / 82 ms über 25 018 / 26 286 / 26 286 Dateien; ein Byte in `unpacked/moveData/0001` gefunden und übernommen („Attacke 0001 · MOVE_POUND“, nur `data/moves/0001.json` geschrieben) in 118 / 144 / 113 ms; `g4k check-data --expect move:1:power=…` auf der gebauten ROM grün.
   - `simulated_change_on_both_sides_is_a_conflict_and_both_choices_work`: JSON und Member von Trainer 1 verschieden, ein Konflikt `data/trainers/0001.json` mit Diff `data.party[0].level` (Platin `6`/`7`, dann `7`/`8`; HeartGold `15`/`16`, dann `16`/`17`); „Studio“ schreibt die Member aus der JSON, „DSPRE“ die JSON aus den Membern, danach ist alles im Gleichstand und der Build läuft.
   - `adopt_dspre_folder_builds_byte_equal_and_reverts`: Umstellung (Phasen in der Zeittabelle), der Build danach hat die SHA-1 der Basis (22,5 / 22,1 / 22,1 s), die Rücknahme 963 / 1 289 / 903 ms, der Build danach ebenso SHA-1-gleich (529 / 610 / 584 ms).
   - `vanilla_checks_on_the_text_project_have_no_errors`: 111 / 145 / 153 Befunde, **0 Fehler** (übersprungen nur `script.message-missing` für die gemeinsamen Scriptdateien, Platin 28, HeartGold 25, wie vor 4.3b).
   - `recipes_with_ids_apply_on_the_vanilla_project`: die Vorlage mit IDs wirkt (Platin Stadt `T01`, Header 411, Starter 387; HeartGold `T20`, 60, 152).
   - `script_refs_on_the_vanilla_project_are_counted`: Verweise aus Scripts Trainer / Item / Spezies Platin 279 / 109 / 19 (296 ms), HeartGold je 202 / 75 / 28 (235, 239 ms); Platin 9 ID-Dateien, 12 von rotom verdeckte IDs (`TRAINER_CLASS_BIKER` …), ID-Dateien 72 ms.
   - `a_script_using_an_id_constant_compiles_to_the_number`: ein Script mit einer Trainer-ID übersetzt zur Nummer.
   - `rename_id_updates_every_reference_on_the_vanilla_project`: `TRAINER_GALAKTIKER_RUEPEL_9` (Platin), `TRAINER_RIVALE_SILBER_15` (HeartGold DE), `TRAINER_RIVAL_SILVER_15` (HeartGold US) → `TRAINER_G4_RENAMED_GATE`, je 3 Dateien und 3 Verweise in 523 / 517 / 518 ms.
- [x] 2. **Gated Studio-Test** `mcp_with_ids_acts_like_numbers_on_the_vanilla_copy` (`apps/studio/src-tauri/src/mcp/tools.rs`), Release: 1/1 auf allen drei (90 / 94 / 77 s); Umstellung über MCP 57,3 / 59,6 / 53,0 s; `data_edit` mit `"SPECIES_PIKACHU"` antwortet und schreibt genau wie mit `25` (gleiche Antwort, gleiche Bytes von `data/species/0025.json`).
- [x] 3. **Gated Projektstart** `project_start_text_project_builds_byte_equal` (`G4_TEST_ROM` = die Nachbar-`.nds`), Release: 1/1 auf allen drei (53 / 56 / 57 s). Der Assistent mit „als Textprojekt“: Phase `TextProject` 22,4 / 23,5 / 25,6 s (5 447 / 5 334 / 5 334 Dateien geschrieben), der Build danach (22,9 / 25,4 / 24,3 s) hat die SHA-1 der ROM.
- [x] 4. **I3 (Nachtrag aus 4.3a)** auf der eingerichteten Platin-DE-Kopie `4-3b-kpt` (Release-`g4k`, ein Byte in `data/species/0025.json`, `"attack": 55,` ↔ `56,`, `g4k build --timings`, je sechs Läufe, alle `incremental`): Schritt `textproject` 78 bis 82 ms, gesamt 303 bis 342 ms (Last 4,4); Wiederholung 80 bis 85 ms und 316 bis 357 ms (Last 5,6). Das bestätigt die Messung von Task 10c (rund 0,08 s / 0,33 s bei Last 5,5).
- [x] 5. **`scripts/gate.sh`:** grün auf `891e1e3` in 8 324 s (03:26 bis 05:45, Last 3,7 bis 11,7; Log `target/agent/gate-sh-4-3b.log`, mit `G4_ROMS_DIR=<Roms>` und `UV_NATIVE_TLS=1`, Exit 0). „build graph (timings)“ war auf allen drei Projekten im ersten Versuch grün (keine Wiederholung). Der erste Lauf auf `9775467` scheiterte nach 3 195 s im Schritt der gated Tests auf HeartGold DE an G1 (Abschnitt „Befunde“); alle Schritte davor und alle Kernel-Suiten dieses Schritts waren grün (Log `target/agent/gate-sh-4-3b-lauf1.log`).

   | Schritt | Sekunden |
   |---|---|
   | `cargo fmt --check`, `cargo clippy -D warnings`, `cargo deny` | 0, 1, 3 |
   | `cargo test` ohne ROM | 853 |
   | mcp | 26 |
   | `pnpm test` | 105 |
   | tokens, styles | 0, 2 |
   | `pnpm build`, dist ohne Brücke | 26, 0 |
   | g4emu unit, integration | 19, 7 |
   | gated `cargo test` HeartGold DE, HeartGold US, Platin DE | 2 340, 2 321, 2 168 |
   | build graph (timings) HeartGold DE, HeartGold US, Platin DE | 83, 74, 42 |
   | gated Projektstart HeartGold DE, HeartGold US, Platin DE | 105, 75, 74 |

Befehle: [docs/agent-gate.md](agent-gate.md), „Phase 4.3b“. Auf stderr stehen `store …`, `sync-full …`, `sync …`, `conflict …`, `adopt <Spielcode> phases …`, `revert …`, `checks …`, `recipe …`, `refs …`, `ids …`, `rename …`, im Studio-Test `adopt: … ms` und im Projektstart `<Spielcode>: phases …`.

## (2) Szenario kopflos

Je Spiel auf `4-3b-k<spiel>` mit Release-`g4k` und Release-`studio mcp --project <Kopie> --base-rom <nds>` über JSON-RPC (stdio; Skript im Scratchpad). Platin DE 01:56 bis 01:58, HeartGold DE 01:58 bis 01:59, HeartGold US 01:59 bis 02:01; Last 2,3 bis 7,7. Alles bestanden, auf allen drei:

- [x] 1. `g4k base assign` (137 bis 177 ms); MCP `text_project_status` → `{"state":"dspre","adoptable":true,"reason":null}`.
- [x] 2. `adopt_text_project` ohne `--allow-irreversible` → `needs_confirmation`; mit → der Bericht (`sync_files` 25 018 bzw. 26 286, `include_written: true`) nach 48,2 / 58,5 / 57,5 s; danach `text_project_status` → `current`.
- [x] 3. `g4k build` → SHA-1 gleich der `.nds` (`d40f19ac…`, `13b6853a…`, `4fcded0e…`), 22,0 / 23,8 / 23,4 s (erster Build).
- [x] 4. `git init` in der Kopie, `g4k text sources .` (6 574 / 6 302 / 6 302 Dateien) committet, `git status --porcelain --untracked-files=no` leer.
- [x] 5. MCP `data_edit` Pikachu `attack` 55 → 56 mit `"index": "SPECIES_PIKACHU"` (21 bis 26 ms), `save` (16 / 23 / 23 ms) → `git status` nennt nur ` M data/species/0025.json` (Diff `"attack": 55,` → `56,`); `undo`, `save` → leer; dasselbe mit `"index": 25` → dieselbe Antwort `[{"kind":"Personal","index":25}]` und derselbe Diff; `undo`, `save` → leer.
- [x] 6. Byte 3 von `unpacked/moveData/0001` (Stärke 40 → 41) wie DSPRE geschrieben → `g4k text sync` übernimmt: `dspre data/moves/0001.json Attacke 0001 · MOVE_POUND`, `stat=25018 hashed=4 written=1` (159 / 137 / 153 ms Prozesszeit); `data/moves/0001.json` hat `power` 41.
- [x] 7. JSON (`party[0].level` +1) und Member (`trainerParty/0001` Byte 2..3, +2) von Trainer 1 verschieden geändert → `g4k text sync` Exit 1 mit `conflict data/trainers/0001.json Trainer 0001 · TRAINER_TEENAGER_GUNTHER` (HeartGold `TRAINER_RIVALE_SILBER` / `TRAINER_RIVAL_SILVER`); MCP `external_changes {"diff": true}` (124 bis 145 ms) zeigt die Einheit (`side: conflict`, Quellen und beide Member) und `diffs` `data.party[0].level` Studio `6` / DSPRE `7` (HeartGold `15`/`16`); `resolve_external_conflict` `studio` (109 bis 112 ms) → JSON und Member `6`; erneut herbeigeführt (`8`/`9`, HeartGold `17`/`18`), `dspre` (95 bis 101 ms) → beide `9`; `g4k text sync` danach jeweils Exit 0.
- [x] 8. MCP `rename_id` von Trainer 2 (`TRAINER_TEENAGER_LOGAN`, `TRAINER_RIVALE_SILBER_2`, `TRAINER_RIVAL_SILVER_2` → `…_GATE`, 596 / 559 / 553 ms, Plan mit `id_files: true`) → `git diff --stat` nennt `data/ids/trainers.json` und `data/trainers/0002.json` (je 1 Zeile; `g4-checks.json` gibt es in den Kopien nicht); `undo` (112 bis 116 ms) → `git status` leer.
- [x] 9. `g4k build` → ROM (1,6 bis 2,3 s); `uv run g4emu run --rom … new-game shot nach-intro`: Platin Header 415, x 4, y 6 nach 10 376 Bildern (9,7 s), HeartGold DE Header 64, x 6, y 6 nach 4 860 Bildern (5,4 s), beide Bilder zeigen das Spielerzimmer (`emu-kpt/nach-intro.png`, `emu-khgde/nach-intro.png`). HeartGold US hat kein `intro` in `games.yaml`: `wait 1800 shot titel` zeigt den laufenden Titelvorspann (`emu-khgus/titel.png`).
- [x] 10. Die alten Member-Bytes (Attacke 1, Trainer 1) wie DSPRE zurückgeschrieben, `g4k text sync` übernimmt beide (`written=2`), `g4k build` → SHA-1 der `.nds`; `g4k text revert` (1 086 / 1 138 / 759 ms) → `backup=…/.g4/backup/textproject-<Zeit>` mit `g4-project.json`, kein Manifest mehr in der Wurzel; `g4k build` → SHA-1 der `.nds` (757 / 566 / 568 ms).

Ein erster Versuch auf Platin brach an einem Fehler des eigenen Skripts ab (die Antwort steckt in `structuredContent.result`); das Skript beendete dabei den Server mitten in der Umstellung, die Kopie wurde verworfen und frisch angelegt. Das ist kein Befund des Produkts.

## (3) Browser-Teil

Gelaufen im UI-Test am 2026-10-28, 23:13 bis 23:48, auf `3578645` (Test-Umgebung, Chrome headless über CDP, 1440 × 900, Hell, Dunkel, Kompakt; Bericht `.superpowers/sdd/2026-10-28-phase-4-3b-textprojekt-quelle/ui-test-report.md`). Ergebnis: bestanden mit acht Befunden (0 Hoch, 3 Mittel, 5 Gering), alle behoben (Abschnitt „Befunde“), U1, U2, U3 und U8 im Gate-Lauf nachgeprüft.

| Schritt | Ergebnis |
|---|---|
| Öffnen zeigt den Hinweis | ja, „Einrichten…“-Variante (`gate3-01`) |
| „Einrichten…“ mit Prüfliste und Fortschritt | Prüfliste vollständig (`gate3-02`); Fortschritt nur „Wird eingerichtet …“ (`gate3-03`, keine Phasen, siehe „Nicht möglich“); `adopt_text_project` 59,0 s, „Textprojekt eingerichtet: 6.574 Dateien in 59 s.“ |
| `git init` und Commit der Quellen | 6 574 Dateien, `git status --untracked-files=no` leer |
| Pokédex › Pikachu zeigt `SPECIES_PIKACHU`, Angriff ändern, speichern | `[data-entity-id]` „SPECIES_PIKACHU“ (`gate3-04`); 55 → 56, Speichern 154 ms; `git status` nur ` M data/species/0025.json` (`gate3-05`) |
| Byte im Member von außen, Fokus | `personalPokeData/0026` 90 → 91: „1 Änderung aus DSPRE übernommen“ (`gate3-06`), „Anzeigen“ → „Zuletzt übernommen“ „Pokémon 0026 · SPECIES_RAICHU“ „aus DSPRE“ (`gate3-07`); JSON 91 |
| beide Seiten ändern | 0027 JSON 80, Member 78: „1 Konflikt zwischen Studio und DSPRE“ (`gate3-08`), Diff `personal.attack 80 78` (`gate3-09`), „Studio-Fassung behalten“ → Member 80; erneut, „DSPRE-Fassung übernehmen“ → JSON 82 (`gate3-10`) |
| ungespeichert plus Member von außen | Plakette „Ungespeichert“ und Warnung (`gate3-11`); der Diff zeigte den gespeicherten Wert (U3, behoben) |
| JSON mit Tippfehler | `data/species/0028.json:6:19: …` unter „Fehlerhafte Dateien“ (`gate3-12`; Prüfcode in der Zeile U6, Untertitel U5, behoben) |
| „ID umbenennen…“ eines Trainers, ⌘Z | Plan 2 Verweise / 2 Dateien (`gate3-13`), Meldung, Kopf neu (`gate3-14`), `git status` zwei Dateien; ⌘Z: beide zurück (`gate3-15`) |
| „Hier spielen“ mit offenem Konflikt | nicht gebaut, „Erst die Konflikte in Technik › Textprojekt lösen.“ (`gate3-16`; vorher Pfadfrage, U8, behoben) |
| „Zurücknehmen…“ | mit Ungespeichertem „Erst speichern.“ (`gate3-18`); Dialog (`gate3-19`), „Textprojekt zurückgenommen. Sicherung: …“, 2,8 s, wieder DSPRE-Layout (`gate3-20`) |
| Assistent mit Häkchen | `create_project` 30,0 s, Technik › Textprojekt „Abgeglichen“ (`gate3-21`) |
| MCP über die Brücke (`4-3b-mcp`, „Lesen und bearbeiten“) | `adopt_text_project` fragt „Als Textprojekt einrichten“ (`mcp-01`), nach „Zulassen“ 72,1 s, die Oberfläche lädt ohne Neuladen nach (`mcp-03`); `data_edit` mit `SPECIES_PIKACHU` ohne Frage (0,12 s); `rename_id` mit `undo_agents: ["agent-gate","agent-gate"]` und beiden Einträgen im Verlaufsmenü (`mcp-06`, `mcp-07`) |
| „Hier spielen“ eines kleinen Hacks | nach dem Lösen gebaut (6,46 s), `g4k check-data` „4 expectation(s) met“, g4emu `new-game` im Spielerzimmer (`emu/nach-intro.png`) |

Zeiten des UI-Tests (Debug-Brücke):

| Messung | Wert | Last |
|---|---|---|
| `g4k text adopt` (CLI) | 48,6 s; Scripts 0,8, Basisschicht 3,3, Einträge 27,3, Abgleich 17,2 s; 6 574 Dateien, 25 018 aufgezeichnet | 13,1 → 8,3 |
| Einrichten in der Oberfläche | 57 s / 59,0 s | 2,4 → 4,5 / 1,7 → 7,9 |
| Einrichten per MCP (mit Klick „Zulassen“) | 72,1 s, `ms` 57 327 | 4,7 / 3,7 |
| Fokus-Abgleich ohne Änderung | 219 bis 235 ms Rundlauf (innen 214 bis 230 ms), 7 Läufe | 1,1 bis 1,9 |
| Fokus-Abgleich mit einer Änderung | 247 bis 677 ms (innen 239 bis 637 ms); Meldung 1,07 bis 1,55 s nach dem Fokus (mit `FOCUS_DELAY_MS` 800) | 1,1 bis 3,9 |
| Abgleich beim Öffnen | 3,4 bis 3,9 s Rundlauf (wartet hinter dem Laden), innen 184 bis 288 ms | 4,7 |
| Speichern einer Art | 147 / 154 ms | 2 bis 4 |
| `conflict_diff` / `resolve_external_conflict` | 18 bis 21 ms / 268 bis 320 ms | 2 bis 4 |
| `plan_rename_id` / `rename_id` / ⌘Z | 1 136 ms / 1 359 ms (innen 982) / 789 ms | 3,4 |
| `revert_text_project` | 2 786 ms | 2,5 |
| `build_rom` über „Hier spielen“ | 6,74 s | 1,9 → 2,2 |
| `create_project` mit Textprojekt | 30,0 s, Öffnen 1,6 s | 1,1 → 4,7 |
| MCP `data_edit` / `rename_id` | 0,12 s / 1,48 s | ~4 |

Der Fokus-Abgleich der Oberfläche (214 bis 230 ms innen) liegt über dem Kernel-Wert (82 bis 88 ms), weil die Brücke ein Debug-Build ist und die Anfrage über HTTP geht; beides unter 0,5 s.

**Nicht möglich** im Browser-Modus (Hand-Punkte):

- Die **Fortschrittsphasen beim Einrichten** („Scripts übernehmen … {n} von {m}“, „Basisschicht anlegen …“, „Einträge schreiben …“, „Abgleich vorbereiten …“) und die **laufende Phase „Textprojekt einrichten“ im Assistenten**: die Brücke reicht keine Tauri-`Channel` durch; der Dialog zeigt die ganze Zeit „Wird eingerichtet …“, der Assistent „Lesen · läuft“.
- Der **volle Abgleich „Gleiche mit DSPRE ab … {done} von {total} Dateien“**: braucht ein Textprojekt ohne Aufzeichnung und Fortschritt über `Channel`; CLI, Oberfläche und Assistent schreiben die Aufzeichnung schon beim Einrichten.
- **„Hier spielen“ startet das Spiel**: im Browser-Modus wird nur gebaut; Ersatz war g4emu.

**Nachprüfung im Gate-Lauf** (2026-10-29, 02:02 bis 02:27, `9775467`, Test-Umgebung auf 1580/1581, Browser-Fenster der Desktop-App):

| Befund | Ergebnis |
|---|---|
| U1 | `4-3b-ui` ohne Basismarke: Hinweis „… Dafür braucht es die Basis-ROM.“, „Basis-ROM zuordnen…“ → Technik › Veröffentlichen, „Zuordnen…“ → „Zuordnen“: Technik › Textprojekt zeigt sofort „Als Textprojekt einrichten…“ freigegeben (kein `aria-disabled`), ohne Neuöffnen. **Behoben.** Der Hinweis selbst ist danach weg: jede Taste einer Meldung schließt sie (`ToastHost`), das Ersetzen an derselben Stelle zeigt sich nur, wenn die Basis auf anderem Weg zugeordnet wird (Agent, Technik direkt). |
| U2 | `4-3b-mcp` (Basis zugeordnet, Hinweis „Einrichten…“ steht, nicht angeklickt): `mcp.sh call adopt_text_project`, Frage „Als Textprojekt einrichten“, „Zulassen“; nach 35,0 s (`ms`) ist der Hinweis weg, Technik › Textprojekt „Abgeglichen“, `text_project_status` `current`. Ein zweites `adopt_text_project` über MCP: nach 390 ms abgelehnt, ohne Frage, `{"code":"error","message":"Das Projekt ist schon ein Textprojekt."}`; über den Befehl der Oberfläche (`call.sh adopt_text_project`) ebenso. **Behoben.** Auf `4-3b-ui` dazu U4: während des Laufs Untertitel „Wird eingerichtet …“, „Als Textprojekt einrichten…“ gesperrt mit „Wird von einem Agenten eingerichtet …“; „Einrichten…“ des Hinweises öffnet keinen Dialog, nur diese Info. |
| U3 | `4-3b-ui` nach dem Einrichten: `data_edit` Pikachu Angriff 57 ungespeichert, Member `personalPokeData/0025` Byte 1 → 60, Fokus: Konflikt „Pokémon 0025 · SPECIES_PIKACHU“ mit „Ungespeichert“, aufgeklappt `Feld | Studio (ungespeichert) | DSPRE`, `personal.attack | 57 | 60`, Warnung; MCP `external_changes {"diff":true}` dasselbe (`"studio": "57", "dspre": "60", "unsaved": true`). **Behoben.** |
| U8 | Derselbe offene Konflikt, „Bauen“ in der Titelleiste: kein Dialog „Speichern unter“, kein `build_rom`, genau eine Meldung „Erst die Konflikte in Technik › Textprojekt lösen.“ mit „Lösen…“ an der Stelle der Konfliktmeldung. **Behoben.** |

## (4) Hand und Blick

Beim Nutzer (Abschnitte „Hand“ und „Blick“ unten): die DSPRE-Gegenprobe auf einem eingerichteten Ordner (Hand), der Fokuswechsel im gepackten Tauri-Fenster (Hand), die Fortschrittsphasen beim Einrichten und im Assistenten (Hand), der Blick auf die Bilder des UI-Tests.

## Ergebnis

| Punkt | Wer | Ergebnis | Datum |
|---|---|---|---|
| (1) 1 gated `textproject_source` | Agent | bestanden: je 9/9 auf Platin DE, HeartGold DE, HeartGold US (Release); nur `data/species/0025.json` geändert, DSPRE-Änderung übernommen, Konflikt mit beiden Wahlen, Umstellung und Rücknahme byte-gleich, `vanilla_checks` 0 Fehler | 2026-10-29 |
| (1) 2 gated Studio-Test MCP mit IDs | Agent | bestanden: 1/1 auf allen drei, `SPECIES_PIKACHU` wie `25` | 2026-10-29 |
| (1) 3 gated Projektstart als Textprojekt | Agent | bestanden: 1/1 auf allen drei, SHA-1 der ROM | 2026-10-29 |
| (1) 4 I3 | Agent | bestanden: Schritt 78 bis 85 ms, gesamt 303 bis 357 ms (Last 4,4 bis 5,6) | 2026-10-29 |
| (1) 5 `scripts/gate.sh` | Agent | bestanden auf `891e1e3`, 8 324 s, alle Schritte im ersten Versuch; der erste Lauf auf `9775467` scheiterte an G1 (behoben in `891e1e3`) | 2026-10-29 |
| (2) 1 bis 4 Status, Umstellung, Build, Git | Agent | bestanden auf allen drei: `needs_confirmation` ohne Erlaubnis, Bericht mit, SHA-1 der Basis | 2026-10-29 |
| (2) 5 `SPECIES_PIKACHU` wie `25` | Agent | bestanden: nur `data/species/0025.json`, gleiche Antwort, gleicher Diff | 2026-10-29 |
| (2) 6 und 7 DSPRE-Änderung, Konflikt, beide Wahlen | Agent | bestanden auf allen drei | 2026-10-29 |
| (2) 8 ID umbenennen, `undo` | Agent | bestanden: Register und Trainer-JSON, `undo` leer | 2026-10-29 |
| (2) 9 ROM in melonDS | Agent | bestanden: Platin und HeartGold DE nach dem Intro im Spielerzimmer, HeartGold US Titelvorspann (kein `intro` für IPKE) | 2026-10-29 |
| (2) 10 Rücknahme | Agent | bestanden: Manifest im Backup, SHA-1 der Basis | 2026-10-29 |
| (3) Browser-Teil und MCP über die Brücke | Agent | bestanden mit U1 bis U8 (UI-Test), behoben in `fdb3483` und `9775467`; U1, U2, U3, U8 nachgeprüft | 2026-10-28, 2026-10-29 |
| DSPRE-Gegenprobe | Hand | offen | – |
| Fokus im gepackten Tauri-Fenster | Hand | offen | – |
| Fortschrittsphasen beim Einrichten und im Assistenten | Hand | offen | – |
| Blick auf Technik › Textprojekt, Einrichten, Umbenennen | Agent + Blick | offen: 80 Bilder liegen bereit, vom Agenten angesehen | – |
| `.fig` | Agent | offen: OpenPencil-MCP antwortete zweimal „RPC timeout (20s)“; Anleitung `design-system/openpencil/NACHTRAG-4-3b.md` | – |
| Unabhängige Abnahme | Agent | bestanden mit fünf behobenen Befunden (A1 Mittel: falscher Konflikt und stilles Überschreiben an `arm9.bin` neben der Header-Tabelle; A2 bis A5 gering: Oberfläche), dazu als Nachtrag A6 (ein unter Last flackernder Studio-Test, deterministisch gemacht); Abschnitt „Abnahme (2026-10-29)“ | 2026-10-29 |

Zeittabelle (Release-Build, gated Tests aus (1) und Szenario aus (2); die Oberfläche steht oben unter (3)):

| Spiel | Umstellung (s, je Phase) | Build danach SHA-1 = Basis | Speichern einer Entität (ms) | Fokus ohne Änderung (ms, Dateien) | mit einer DSPRE-Änderung (ms) | voller Abgleich (s) | Umbenennen (ms) | Rücknahme (ms) | Last |
|---|---|---|---|---|---|---|---|---|---|
| Platin DE | 52,3: Scripts 0,8, Basisschicht 3,5, Einträge 29,6, Abgleich 18,4 (MCP kopflos 48,2; Assistent 22,4) | ja (22,5 s; kopflos 22,0 s; Assistent 22,9 s) | 9 (MCP `save` 16) | 86 (25 018) | 118 (`g4k text sync` 159) | 51,4 | 523 (MCP 596) | 963 (CLI 1 086) | 18,0 → 3,9 (1); 3,7 → 7,7 (2) |
| HeartGold DE | 58,6: 0,7, 3,9, 33,0, 21,0 (MCP 58,5; Assistent 23,5) | ja (22,1 s; 23,8 s; 25,4 s) | 20 (23) | 88 (26 286) | 144 (137) | 36,6 | 517 (559) | 1 289 (1 138) | 4,2 → 3,4 (1); 3,2 → 5,5 (2) |
| HeartGold US | 58,1: 0,7, 3,4, 32,6, 21,5 (MCP 57,5; Assistent 25,6) | ja (22,1 s; 23,4 s; 24,3 s) | 9 (23) | 82 (26 286) | 113 (153) | 38,6 | 518 (553) | 903 (759) | 3,5 → 3,1 (1); 2,3 → 3,2 (2) |

Die Grenzen der Tests (`G4_TIMING_STRICT=1`: Speichern ≤ 200 ms, Fokus-Abgleich ≤ 500 ms, eine Änderung ≤ 1 000 ms) halten alle Werte mit Abstand ein. Die Umstellung braucht je Spiel rund eine Minute, davon die Hälfte für das Schreiben der Einträge; der volle Abgleich ohne Aufzeichnung (nur nach einem verlorenen Zustand) 37 bis 51 s.

## Befunde

### UI-Test (2026-10-28)

Der UI-Test fand 0 Hoch, 3 Mittel und 5 Gering; alle behoben in `fdb3483` (U1 bis U8) und `9775467` (Fix-Runde nach dem Review), je mit Regressionstest, Review sauber. U1, U2, U3 und U8 sind im Gate-Lauf im Browser nachgeprüft (Abschnitt (3), „Nachprüfung“).

| Nr. | Schwere | Befund | Fix |
|---|---|---|---|
| U1 | Mittel | Nach „Basis-ROM zuordnen…“ aus dem Hinweis blieb „Als Textprojekt einrichten…“ gesperrt („Diesem Projekt ist keine Basis-ROM zugeordnet.“), bis das Projekt neu geöffnet wurde. | `fdb3483`: ein Wechsel des Basis-Zustands (Zuordnen, `assign_base` eines Agenten, ROM wieder gefunden) liest `text_project_status` neu; der offene Hinweis wechselt an derselben Stelle auf „Einrichten…“. Test `assigning_the_base_rom_unlocks_setting_up_and_the_hint_offers_it`. |
| U2 | Mittel | Nachdem ein Agent eingerichtet hatte, blieb der Hinweis stehen; „Einrichten…“ öffnete den Dialog auf dem Textprojekt und ließ die Umstellung noch einmal 58 s laufen. | `fdb3483`: der Hinweis geht, sobald das Projekt kein DSPRE-Ordner mehr ist; der Einrichten-Dialog öffnet nur im DSPRE-Layout; die App lehnt `adopt_text_project` auf jedem Textprojekt sofort ab („Das Projekt ist schon ein Textprojekt.“, Code `error`, auch vor der Frage an den Nutzer). `g4k text adopt` bleibt auf einem Textprojekt eine Reparatur (Export ohne Schreiben, neue Aufzeichnung). |
| U3 | Mittel | Der Diff eines Konflikts mit ungespeicherter Änderung zeigte in der Spalte „Studio“ den gespeicherten Wert, nicht den im Editor. | `fdb3483`, `9775467`: mit ungespeicherten Assets ist die Studio-Seite der Wert im Store (Export in den Speicher), die Spalte heißt „Studio (ungespeichert)“; `external_changes` (MCP) zeigt denselben Diff; ist bei ungespeicherten Werten die JSON die andere Seite (Dateiänderung), zeigt die zweite Spalte die Datei (`9775467`, I2). |
| U4 | Gering | Während ein Agent einrichtete (72 s), zeigte die Oberfläche nichts und bot „Einrichten…“ weiter an. | `fdb3483`, `9775467`: Hub-Ereignis `busy` (Anfang und Ende, das Ende auch bei einer Panik); Technik zeigt „Wird von einem Agenten eingerichtet …“ und sperrt Einrichten und Zurücknehmen. |
| U5 | Gering | Untertitel „Abgeglichen“ bei fehlerhaften Dateien. | `fdb3483`: „1 fehlerhafte Datei“ (Konflikte zuerst). |
| U6 | Gering | Prüfcode `textproject.schema:` in der Zeile einer fehlerhaften Datei. | `fdb3483`, `9775467`: die Oberfläche lässt einen führenden `textproject.…:`-Code weg. |
| U7 | Gering | Umbrüche zwischen Zahl und Einheit („57 | s.“) und mitten in Pfaden. | `fdb3483`: geschütztes Leerzeichen in den Zähltexten (`src/words.ts`), `<wbr>` nach jedem `/` in Meldungen. |
| U8 | Gering | „Hier spielen“ mit offenem Konflikt fragte zuerst nach dem Pfad, dann stapelten sich zwei Meldungen mit „Lösen…“. | `fdb3483`: ohne bekannten Pfad gleicht „Bauen“ vorher ab und lehnt ohne Pfadfrage ab; die Ablehnung ersetzt die Konfliktmeldung an derselben Stelle (`CONFLICTS_TOAST`). |

Dazu aus derselben Runde: die Vorschau des Assistenten zeigt „Original, als Textprojekt“.

### Gate-Lauf (2026-10-29)

| Nr. | Schwere | Befund | Fix |
|---|---|---|---|
| G1 | Mittel (Test, nicht Produkt) | Im ersten Lauf von `scripts/gate.sh` scheiterte der gated Studio-Test `mcp_with_ids_acts_like_numbers_on_the_vanilla_copy` im Schritt der gated Tests (HeartGold DE, nach 3 195 s): „Originale sind gesperrt, nur Kopien unter _testkopien: …/Roms“. Das Skript setzt `G4_ROMS_DIR` und nimmt das Original als `G4_TEST_PROJECT`; der Test erlaubte den Ordner der Vanilla-`.nds`, und das ist dort `G4_ROMS_DIR` selbst, den `UserPolicy` zu Recht ablehnt. Die Einzelläufe in (1) liefen ohne `G4_ROMS_DIR` auf Kopien und sahen es nicht. Alle Kernel-Suiten desselben Schritts waren grün (`textproject_source` 9/9 im Debug-Build, 303 s). | `891e1e3`: der Test klont die Vanilla-ROM in seinen Temp-Ordner neben die Kopie und erlaubt nur diesen Ordner. Regressionstest ist der gated Test selbst im Aufbau von `gate.sh` (ein Ersatz-`G4_ROMS_DIR` unter `_testkopien/4-3b-red` mit Projekt und `.nds`): rot mit der Ablehnung in `tools.rs:3978`, danach grün (72 s), ohne `G4_ROMS_DIR` weiter grün (78 s). |

Beobachtung ohne Befund: Jede Taste einer Meldung schließt sie (`ToastHost`), auch „Basis-ROM zuordnen…“ des Hinweises; das Ersetzen des offenen Hinweises durch die „Einrichten…“-Fassung (U1) zeigt sich darum nur, wenn die Basis auf anderem Weg zugeordnet wird. Der Weg über den Hinweis endet in Technik › Textprojekt mit freigegebener Taste.

## Nachträge aus 4.3a

Das abschließende Review von 4.3a ließ drei Punkte (I1 bis I3) auf main offen ([phase-4-3a-gate.md](phase-4-3a-gate.md)); sie sind hier als eigene Tasks behoben, je mit Tests:

- **I1 (Task 10a, `b48c22d` bis `3a20e51`):** `g4k text export` lehnt ab, solange JSON neuer als ihre Member sind, `g4k text import`, solange Member neuer als ihre JSON sind („Im Textprojekt sind Änderungen …“, „Member sind neuer als ihre JSON …“); `--force` verwirft die andere Seite. Der Export zeichnet den Schritt neu auf; der Abgleich zeichnet nur Bytes auf, die sein Schreiber gelesen oder geschrieben hat (Schreib- und Lese-Journal). Rundlauf Platin DE 216 s (Export 87,7, Import 35,6, Build 16,4, Basisschicht 9,0 s).
- **I2 (Task 10b, `19ca881`):** der ARM9-Blob wird ohne die Header-Tabelle verglichen und gesetzt.
- **I3 (Task 10c, `b6b5064`):** ein inkrementeller Plan liest nur die Member, die seine geänderten Dateien nennen. Messung damals auf derselben Platin-DE-Kopie, ein Byte in `data/species/0025.json`, `g4k build --timings`, Last 5,5: vorher (`19ca881`) Schritt `textproject` rund 0,93 s, gesamt rund 1,15 s; nachher (`b6b5064`) rund 0,08 s und 0,33 s; der erste Lauf nach der Umstellung 4,5 bis 7,3 s → 135 ms; Trockenlauf von Export und Import 0,8 bis 1,9 s → 0,3 s. Nicht mit den „4,75 s“ aus 4.3a vergleichen (anderer Code vor I1 und I2, Last 14). Im Gate-Lauf neu gemessen: Platin DE auf `4-3b-kpt` (Stand `9775467`, Release-`g4k`), Schritt `textproject` 78 bis 85 ms, gesamt 303 bis 357 ms bei Last 4,4 bis 5,6, alle Läufe `incremental` (siehe (1) 4).
- **Kleinere Punkte (Task 10d, `6aa8ec8`, `7766a1d`):** „Löschen kommt mit Phase 3d.“ statt eines Fehlers ohne Weg; überzählige Trainer-Member werden gemeldet (im Build ein Fehler); die Symboltabellen sind an `schema.snap` gebunden; Pfade in Berichten mit `/`.

Das Szenario von 4.3a (`git checkout -- data/; g4k text import .` über geänderte Member) braucht seit I1 `--force` (`g4k text import --force .`); [phase-4-3a-gate.md](phase-4-3a-gate.md) bleibt als Protokoll unverändert.

## Hinweise

- **ID-Dateien statt Include-Datei:** Der Plan sah eine rotom-Include-Datei `.g4/include/g4_ids.rotom` mit `#include`/`#define` vor. rotom v0.7.0 ignoriert `#include` und `#define` im DSPRE-Modus (geprüft, `rotom-include-research.md`, Ruling in Task 8). Ersetzt durch versteckte JSON-ID-Dateien `.rotom/command_database/.g4-ids-<art>.json` in rotoms Befehlsdatenbank (Compiler, CLI und LSP lesen sie): mit Kollisionsfilter (`textproject.id-shadowed`, in Vanilla Platin 12 IDs), nur bei Änderung atomar geschrieben, vor jedem Kompilieren erneuert, als Eingaben des rotom-Schritts; die Rücknahme lehnt ab, solange Scripts reine Studio-IDs benutzen. `.g4/include/` und `include_roots` gibt es nicht.
- **Für 4.4:** die `.gitignore`-Vorlage muss `/.rotom/command_database/.g4-ids-*.json` nennen (erzeugt) sowie `.g4/backup/`; das Einrichten schreibt keine `.gitignore`, nach dem Einrichten zeigt `git status` `.g4/build/`, `.g4/cache/`, `.g4/models/` (und `.DS_Store`) ungetrackt. Die Gate-Prüfungen nutzen `--untracked-files=no`.
- **Für 4.4 aus der Abnahme:** keine Änderung an Format, JSON-Schlüsseln, CLI-Ausgaben, Tauri- oder MCP-Befehlen. Geändert ist das Verhalten des Abgleichs (A1, `docs/formats/text_project.md`, „Einheiten“ und „Was unaufgezeichnet bleibt“): ein Eintrag unter `sync.files` in `.g4/build/state.json` kann jetzt `"xxh3": null` tragen (das Feld war schon optional, Schritt-Einträge kannten es). Neu in der Oberfläche: `requestTextProjectFocus` und `type SyncGroup` aus `src/technik/TextProjectSurface.tsx`, `showTextProject(group)` in `App.tsx`, `WJ` in `src/words.ts` (A4, A5); wer in 4.4 an `App.tsx` um die Meldungen des Abgleichs oder an `useTextProjectDialogs.ts` arbeitet, bekommt dort Konflikte beim Zusammenführen.
- **`g4k text adopt` auf einem Textprojekt** ist eine Reparatur (Export ohne Schreiben, neue Aufzeichnung); die App und MCP lehnen es ab (U2).
- **Design-System:** Task 13 (`a81b2db`): README „Nachtrag 4.3b“, `textprojekt.body.js`, Audit. Die `.fig` ist nicht angefasst: der OpenPencil-MCP antwortete zweimal „RPC timeout (20s)“; offen, Anleitung `design-system/openpencil/NACHTRAG-4-3b.md`.

## Abnahme (2026-10-29)

Unabhängige Abnahme durch eine zweite Sitzung auf dem Stand `6270c44` (main mit 4.3b), Branch `abnahme-4-3b`, Worktree `.claude/worktrees/phase-abn-4-3b`, nach den Punkten oben und den Wegen von [docs/agent-gate.md](agent-gate.md). Frische Kopien unter `Roms/_testkopien/` per `cp -cR`/`cp -c`, Ordner und `.nds` nebeneinander: `4-3b-abn-kpt` (Platin DE), `-khgde` (HeartGold DE), `-khgus` (HeartGold US) kopflos; `-ui` (Platin DE, ohne Basismarke: Hinweis, Zuordnen, Einrichten in Technik, Bearbeiten, Konflikte, Umbenennen, „Hier spielen“, Zurücknehmen), `-web` (Platin DE, Basis mit `g4k base assign`: Hinweis „Einrichten…“, Einrichten über den Hinweis, danach I1 bis I3 per CLI), `-mcp` (Platin DE, Basis zugeordnet: MCP über die Brücke), `-wiz` (nur die `.nds`, Assistent; angelegt als `4-3b-abn-wizp`), `-hgdeweb` (angelegt, nicht gebraucht). Release-`g4k` und Release-`studio mcp` (stdio, Python-Skript im Scratchpad), Test-Umgebung auf 1610/1611 (Debug-Brücke). Die Oberfläche lief in einem kopflosen Chrome über CDP (Node-Skript im Scratchpad), 1440 × 900 und 1100 × 720, Hell, Dunkel, Kompakt; die Browser-Pane der Desktop-App lieferte nur einen 800 × 600-Ausschnitt. Fokus-Abgleich über `window.dispatchEvent(new Event("focus"))`, Änderungen „von DSPRE“ per Python in die Member. Bilder und Protokolle unter `target/agent/gate-2026-10-29/abnahme-4-3b/` im Worktree: `abn-01` bis `abn-37` (Oberfläche), `emu-k{pt,hgde,hgus}/`, `emu-ui/` (g4emu), `kopflos-k*.stdout` (Szenario kopflos mit Zeiten und Last), `i1-i3-web.log`, `mcp-adopt.out`, `checks.log`.

**Last:** Die Abnahme lief bei einer 1-Minuten-Last von 2 bis 44, meist 15 bis 35: parallel lief die Sitzung von 4.4 (Tests in `phase-4-4`), und **Microsoft Defender** prüfte jeden Dateizugriff (die drei `wdavdaemon`-Prozesse zusammen rund 330 % CPU). Ein `cat` über die 4 722 Dateien unter `data/` brauchte dabei 25 s (≈5 ms je `open()`); `sample` der Brücke während `plan_rename_id` zeigte 99 % der Zeit in `open()` von `rename::read`. Alle Zeiten mit viel Dateizugriff sind deshalb Vielfache der Gate-Werte; bei Last 2 bis 7 lagen sie wieder dort (Tabelle unten).

| Punkt | Ergebnis | Belege |
|---|---|---|
| Kopflos Platin DE, HeartGold DE, HeartGold US | bestanden auf allen drei, jeder Prüfpunkt grün: `base assign`; `text_project_status` `dspre`/`adoptable`; `adopt_text_project` ohne `--allow-irreversible` `needs_confirmation`, mit der Bericht, ein zweites sofort „Das Projekt ist schon ein Textprojekt.“; `g4k build` SHA-1 = Basis; `git init`, `text sources` (6 574 / 6 302 / 6 302) committet; `data_edit` mit `"SPECIES_PIKACHU"` und mit `25` gleiche Antwort und gleicher Diff, nur ` M data/species/0025.json`, `undo`+`save` leer; `SPECIES_GIBTSNICHT` → `unknown_id`; ein Byte in `moveData/0001` → `text sync` übernimmt `data/moves/0001.json`; Trainer 1 auf beiden Seiten → Exit 1 mit `conflict`, `external_changes` mit Diff, `studio` und `dspre` wirken; `rename_id` von Trainer 2 → `data/ids/trainers.json` und `data/trainers/0002.json`, vergebene ID („… gibt es schon (Nummer 3).“) und falsches Präfix abgelehnt, `undo` leer; Hack gebaut, g4emu: Platin Header 415 (x 4, y 6), HeartGold DE Header 64 (x 6, y 6) im Spielerzimmer, HeartGold US Titelvorspann; Member zurück, `sync`, Build SHA-1 = Basis; `text revert`, Build SHA-1 = Basis | `kopflos-k*.stdout`, `emu-k*/` |
| Hinweis beim Öffnen | bestanden: ohne Basis „… Dafür braucht es die Basis-ROM.“ mit „Basis-ROM zuordnen…“ und „Nicht mehr fragen“ (Region „Meldungen“, `role=status`), nach Zuordnen in Technik › Veröffentlichen ist „Als Textprojekt einrichten…“ frei (U1); mit Basis „… DSPRE öffnet den Ordner weiter.“ mit „Einrichten…“, öffnet den Dialog. Umbruch „Basis-“ / „ROM.“ im Hinweis (A5) | `abn-01`, `-02`, `-03`, `-04`, `-25`, `-26` |
| „Als Textprojekt einrichten…“ | bestanden: Prüfliste ✓ „Alles gespeichert“, ✓ „Basis-ROM gefunden: Platin DE (Testprojekt)“, (i) „Scripts werden dabei übernommen“; Startfokus „Abbrechen“, Tab bleibt im Dialog; Enter auf „Einrichten“, im Lauf nur „Wird eingerichtet …“ (`role=status`), keine Schließen-Taste; Technik danach „Textprojekt, Schema-Version 1“; 3 min 12 s (`ui`, Last 31 → 18) und 170 s (`web`, Last 17). Der Dialog schloss vor der Meldung (A2) | `abn-05` bis `-07`, `-27` |
| Bearbeiten und Speichern | bestanden: Pokédex › Pikachu zeigt `SPECIES_PIKACHU` mit „ID kopieren“; Angriff 55 → 56, ⌘S, `git status` nur ` M data/species/0025.json` | `abn-08` |
| Einseitig von außen | bestanden: `personalPokeData/0026` Byte 1, Fokus → „1 Änderung aus DSPRE übernommen“ (+ „Anzeigen“), „Zuletzt übernommen“ „Pokémon 0026 · SPECIES_RAICHU“ „aus DSPRE“, JSON 91. Die Meldung geht nach 5 s (Maus darüber oder Fokus hält sie) | `abn-09`, `-10` |
| Beidseitig | bestanden: 0027 JSON 80, Member 78 → „1 Konflikt zwischen Studio und DSPRE“ (+ „Lösen…“), Untertitel „1 Konflikt“, Tabelle `personal.attack 80 78` (Name „Abweichungen: Pokémon 0027 · SPECIES_SANDSHREW“); „Studio-Fassung behalten“ → Member 80; erneut 81/82, „DSPRE-Fassung übernehmen“ → JSON 82. Nach „Lösen…“ und nach der Wahl lag der Fokus auf `body` (A4) | `abn-11`, `-12` |
| Ungespeichert plus außen | bestanden: Pikachu 57 ungespeichert, Member 64 → Plakette „Ungespeichert“, Spalte „Studio (ungespeichert)“ `57 | 64`, Warnung; „Studio-Fassung behalten“ → Member aus der JSON, die 57 bleibt ungespeichert im Editor; nach ⌘S JSON und Member 57; erneut 58/66, „DSPRE-Fassung übernehmen“ → JSON und Member 66, der Editor zeigt nach rund 3 s 66, „Alles gespeichert“ | `abn-13`, `-14`, `-15` |
| JSON mit Tippfehler | bestanden: „1 Projektdatei ist fehlerhaft“, Untertitel „1 fehlerhafte Datei“, Zeile `data/species/0028.json:5:5: personal.atack: unbekannter Schlüssel (…)`; nach der Korrektur und dem Fokus sind Zeile und Meldung weg | `abn-16` |
| „ID umbenennen…“ und ⌘Z | bestanden: Trainer 0002 `TRAINER_TEENAGER_LOGAN`, Menü „Aktionen“ › „ID umbenennen…“, Fokus im Feld „Neue ID“; Plan „2 Verweise in 2 Dateien werden nachgezogen.“ mit beiden Pfaden, `TRAINER_GOERE_NADINE` „gibt es schon (Nummer 3).“; „Umbenennen“ → Meldung „ID umbenannt: … ⌘Z nimmt es zurück.“, Kopf neu, Fokus zurück auf „Aktionen“, `git status` zwei Dateien; ⌘Z → Dateien in 0,7 s zurück, Kopf nach ≤ 5 s. Der Plan brauchte unter Defender 35 s ohne jede Anzeige, Enter im Feld tat nichts (A3) | `abn-17` bis `-21` |
| „Hier spielen“ mit Konflikt, kleiner Hack | bestanden: genau eine Meldung „Erst die Konflikte in Technik › Textprojekt lösen.“ (+ „Lösen…“), keine Pfadfrage; nach dem Lösen „Bauen“ → ROM in 27,7 s, `g4k check-data` 3/3 (Pikachu 66, Sandan 83, Raichu 90), g4emu `new-game` Header 415 im Spielerzimmer | `abn-22`, `emu-ui/nach-intro.png` |
| „Zurücknehmen…“ | bestanden: mit Ungespeichertem „Erst speichern.“ und „Zurücknehmen“ gesperrt; danach „Textprojekt zurückgenommen. Sicherung: …/.g4/backup/textproject-2026-10-29T053312Z“ in 9,9 s, kein Manifest mehr | `abn-23`, `-24` |
| Assistent mit Häkchen | bestanden: Kästchen „Als Textprojekt einrichten“ (`role=checkbox`, Leertaste) im Schritt „Vorlage“, Zusammenfassung „Original, als Textprojekt“ mit Phase „Textprojekt einrichten“, angelegt in 95,5 s, Öffnen 4,2 s, Technik „Abgeglichen“, Basismarke gesetzt | `abn-28` bis `-31` |
| MCP über die Brücke | bestanden: Frage „Agent möchte etwas Unumkehrbares tun“ / „„agent-gate“ möchte: Als Textprojekt einrichten …“ (Startfokus „Ablehnen“), nach „Zulassen“ Untertitel „Wird eingerichtet …“, Taste gesperrt, Status „Wird von einem Agenten eingerichtet …“, „Einrichten…“ des Hinweises zeigt nur das (U4); 58,0 s, danach Hinweis weg, „Abgeglichen“ (U2); zweites `adopt_text_project` 0,18 s abgelehnt; `data_edit` mit `SPECIES_PIKACHU` und `MOVE_POUND` ohne Frage; `rename_id` 6,4 s; Verlauf „Verlauf, 3 Schritte“ mit drei Einträgen „agent-gate“ | `abn-32` bis `-35`, `mcp-adopt.out` |
| I1 | bestanden: `text export` lehnt mit geänderter `data/moves/0002.json` ab („Im Textprojekt sind Änderungen, die noch nicht in den Membern sind (data/moves/0002.json); …“), nichts geschrieben; `--force` holt die JSON aus dem Member (50). `text import` lehnt mit geändertem Member ab („Member sind neuer als ihre JSON (unpacked/moveData/0002); …“), `--force` schreibt den Member aus der JSON; danach SHA-1 = Basis | `i1-i3-web.log` |
| I2 | **Befund A1**: ein ARM9-Byte (0x100) „von DSPRE“ wird als Blob `data/raw/arm9/arm9.bin` übernommen und bleibt neben einer Header-Änderung im Gleichstand (Build 2,3 s, `sync` Exit 0), aber dieselben zwei Änderungen in einem Abgleich ergaben einen falschen Konflikt „Header-Tabelle“, und danach schrieb der Build ein von DSPRE zurückgesetztes Byte still mit dem alten Blob zurück | `i1-i3-web.log` |
| I3 | bestanden: ein Byte in `data/species/0025.json`, sechs Läufe `--timings`, Schritt `textproject` 93 bis 174 ms, gesamt 473 bis 829 ms (Last 2,1 bis 3,6; Schritt `rom` 211 bis 523 ms); ohne Änderung 119 bis 123 ms; danach SHA-1 = Basis | `i1-i3-web.log` |
| Hell, Dunkel, Kompakt, 1440 × 900 und 1100 × 720 | bestanden: keine abgeschnittenen Texte, kein waagrechter Bildlauf, Meldungen über dem Inhalt; nur A5 | `abn-*` |
| Tastatur und Screenreader-Namen | bestanden mit A3 und A4: Dialoge mit Namen (`dialog` „Als Textprojekt einrichten?“, „ID umbenennen“, „Textprojekt zurücknehmen?“), Prüfliste `list` „Vor dem Einrichten“, Konflikt-Tasten mit `aria-expanded`, Tabelle `grid` „Abweichungen: …“, Region „Meldungen“; Fokusfalle in den Dialogen, Escape schließt außer während des Laufs | – |
| Automatik nach den Fixes | bestanden auf dem Gesamtstand der fünf Fixes (vor den Commits, 08:19 bis 09:13, Last 6 bis 36): `cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo deny --locked check licenses advisories`, `cargo test --workspace --all-features` (2 178 bestanden, 0 fehlgeschlagen, 1 übersprungen; 2 881 s), vitest (3 276 bestanden, 1 übersprungen; ein Lauf davor unter Last 17 mit einem Ausfall, wiederholt grün), `pnpm build`, `check-styles`, `check-tokens`; nach A1 gated `textproject_source` 9/9 auf `4-3b-abn-kpt`. Die Fixes sind danach als fünf Commits angelegt, die Prüfungen liefen auf ihrer Summe | `checks.log` |

**Zeiten der Abnahme** (Release-`g4k`, Release-`studio mcp`, Debug-Brücke für die Oberfläche; „L“ ist die 1-Minuten-Last):

| Messung | Platin DE | HeartGold DE | HeartGold US | bei geringer Last (Platin DE) |
|---|---|---|---|---|
| Umstellung MCP kopflos | 180,0 s (L 44 → 33) | 162,3 s (L 15 → 23) | 247,4 s (L 22) | MCP über die Brücke 58,0 s (L 7) |
| erster Build danach, SHA-1 = Basis | 126,9 s | 181,2 s | 130,5 s | `web` 28,8 s (L 1,9) |
| `data_edit` / `save` | 24 / 27 ms | 26 / 44 ms | 33 / 33 ms | – |
| `text sync` mit einer DSPRE-Änderung (innen) | 225 ms | 249 ms | 393 ms | 122 bis 194 ms (L 6) |
| `external_changes` mit Diff / `resolve_external_conflict` | 245 / 127 ms | 255 / 128 ms | 263 / 199 ms | – |
| `rename_id` kalt (eigener Prozess) | 24,6 s (L 31) | 38,0 s (L 23) | 40,7 s (L 22) | CLI 0,5 bis 1,0 s (L 3,5 bis 5,6), Brücke 6,4 s (L 7) |
| Build nach Umbenennen und `undo` | 49,0 s | 19,7 s | 82,6 s | 1,3 bis 2,2 s (L 3,5) |
| `text revert` | 3,0 s | 6,0 s | 7,2 s | Oberfläche 9,9 s (L 13) |

Der Abgleich beim Fokus und Speichern bleiben auch unter Last weit unter den Grenzen der Tests; Umstellung, erster Build, Umbenennen und das Laden großer Listen (Pokédex nach dem Öffnen 17 bis 20 s unter Defender, bei Last 3 1,4 s im DSPRE-Layout wie im Textprojekt) folgen dem Dateizugriff.

**Befunde der Abnahme**

| Nr. | Schweregrad | Befund | Fix | Format/Schnittstelle |
|---|---|---|---|---|
| A1 | Mittel | Die Einheit „Header-Tabelle“ sah `arm9/arm9.bin` als ganze Datei: ein Byte außerhalb der Tabelle „von DSPRE“ und eine Header-JSON im selben Abgleich ergaben einen falschen Konflikt (der Build lehnte ab). Schlimmer: mit dem offenen Konflikt blieb `arm9.bin` unaufgezeichnet, der Blob war aber übernommen; setzte DSPRE das Byte zurück, meldete `text sync` nichts (Exit 0) und der nächste Build schrieb das alte Byte still aus dem Blob zurück (ROM-SHA-1 `cbca4c…` statt der Basis) | `d15b3f3`: für die Header-Tabelle zählt an `arm9.bin` nur die Tabelle (der aufgezeichnete Hash ist der Hash der Bytes außerhalb laut Blob bzw. Basisschicht mit der Tabelle von heute); eine mit einer offenen Einheit geteilte Datei, deren andere Einheit übernommen wird, verliert in `.g4/build/state.json` ihren Hash und gilt danach als geändert, bis sie neu aufgezeichnet ist. Tests in `crates/g4-kernel/tests/textproject.rs` | **Verhalten des Abgleichs** und Text in `docs/formats/text_project.md` („Einheiten“, „Was unaufgezeichnet bleibt“); ein Sync-Eintrag in `state.json` kann jetzt `"xxh3": null` tragen (das Feld war schon optional); keine JSON-Schlüssel, CLI-Ausgaben oder Befehle geändert |
| A2 | Gering | Nach „Einrichten“ schloss der Dialog, sobald der Status neu gelesen war (bei 173,5 s), die Meldung „Textprojekt eingerichtet: 6.574 Dateien in 170 s.“ kam erst nach dem Abgleich beim Öffnen (179,6 s); dazwischen keine Rückmeldung | `604ebf1`: der Dialog bleibt mit „Wird eingerichtet …“, bis `onAdopted` die Sitzung gelesen und die Meldung gesetzt hat; die Einrichtung durch einen Agenten schließt einen nicht laufenden Dialog wie bisher. Test `useTextProjectDialogs.test.tsx` | – |
| A3 | Gering | „ID umbenennen“: während der Plan gerechnet wird (er liest jede Quelldatei, unter Defender 30 bis 36 s), zeigte der Dialog nichts, „Umbenennen“ war stumm gesperrt; Enter im Feld „Neue ID“ tat nichts (im Dialog „Projekt öffnen“ nimmt Enter den Pfad) | `589ef46`: `role=status` „Verweise werden gesucht …“ während des Plans und „Wird umbenannt …“ während des Umbenennens; Enter im Feld benennt um, wenn der Plan für genau den Text steht und nichts ablehnt; das Feld behält den getippten Text. Sieben Tests in `RenameIdDialog.test.tsx` | – |
| A4 | Gering (Tastatur) | Nach „Lösen…“ und „Anzeigen“ der Meldungen des Abgleichs und nach „Studio-Fassung behalten“ / „DSPRE-Fassung übernehmen“ lag der Fokus auf `body` | `1fb833b`: die Taste einer Meldung fordert den Fokus für ihre Gruppe an (`requestTextProjectFocus`: Konflikte, fehlerhafte Dateien, „Zuletzt übernommen“, sonst die Überschrift „Textprojekt“); nach einer Wahl geht der Fokus auf den nächsten Konflikt, sonst den vorigen, die Gruppe oder die Überschrift. Sieben Tests in `TextProjectSurface.test.tsx` | – |
| A5 | Gering | Der Hinweis brach „Basis-“ / „ROM.“ um (1440 × 900) | `5c21a7a`: Wortverbinder U+2060 um den Bindestrich (`WJ` in `src/words.ts`); nicht U+2011, die UI-Schrift Atkinson Hyperlegible Next hat kein Zeichen dafür. Tests in `words.test.ts`, `textProjectHint.test.ts` | – |
| A6 | Mittel (Test, nicht Produkt; Nachtrag der Hauptsitzung) | Der Studio-Test `mcp::tools::tests::shared_text_export_asks_then_refuses_unsynced_changes` flackerte auf main (`6270c44`) unter Last (in vier Läufen einmal rot nach 59 s, Panik bei `tools.rs:2768` und `3813`). Ursache: vor der Frage „Textprojekt exportieren“ prüft der Aufruf `check_text_export` (`found_base` hasht die Basis-ROM, `unsynced_export` plant den Build-Schritt über das ganze Fixture); unter Last dauert das länger als die feste Frist von 5 s des Test-Helfers `answer_next` („no question raised“), danach steht die Frage ohne Antwort, die Toolbox gibt nach `WAIT` (5 s) auf und `unwrap()` fällt. Nachgestellt auf `264aae9`: 24 parallele Prozesse des Tests (Last 16 → 26) 24/24 rot an genau diesen Zeilen, 8 parallele grün | `5470cd0`: nur im Testmodul; `PATIENCE` = `QUESTION_WAIT` (120 s, wie die App) als Obergrenze für jedes Warten auf die Frage oder ihre Antwort, neuer Helfer `await_question` (pollt alle 2 ms, schlägt erst nach `PATIENCE` fehl), `answer_next` und die gleich gebaute Schleife in `parallel_calls_share_one_question` nutzen ihn, `WAIT` = `PATIENCE`; der Ablauf-Test behält seine eigenen 20 ms. Danach 24 parallele Prozesse 24/24 grün (199 bis 200 s, Last 21 → 24) und **zehn Runden hintereinander mit je 8 parallelen Prozessen 80/80 grün** (47 bis 152 s je Runde, Last 17 bis 31; Protokoll `target/agent/flaky/zehn-runden.log` im Worktree `phase-abn-4-3b-flaky`) | – (nur Testcode; danach `cargo fmt --check`, `clippy -D warnings`, `cargo deny`, `cargo test --workspace --all-features` 2 178 bestanden / 0 / 1 übersprungen in 1 673 s, vitest 3 276 / 1, `pnpm build`, Styles, Tokens grün bei Last 18 bis 27) |
| H1 | Hinweis | Unter Microsoft Defender liest jeder Plan von „ID umbenennen“ alle rund 6 500 Quellen neu (30 bis 40 s je Plan); ein Zwischenspeicher für die Verweise wäre der eigentliche Hebel | – (Geparkt; die Anzeige kam mit A3) |
| H2 | Hinweis | Nach dem Entfernen eines Blobs bleibt der leere Ordner `data/raw/arm9/` stehen (Git sieht ihn nicht) | – |
| H3 | Hinweis, vor 4.3b | „Bauen“ mit einem Pfad, der nicht existiert: „Pfad nicht lesbar: … No such file or directory (os error 2)“ (englischer Teil, nur Browser-Modus, wie H3 in 4.0) | – |
| H4 | Hinweis, vor 4.3b | Die Tasten der Dialoge tragen `class="… undefined"` (Baustein `Dialog`); die Zeilen der Diff-Tabelle sind Tab-Stopps mit `aria-selected` (Baustein `Table`, rovende Zeilen) | – |
| H5 | Hinweis, aus A1 bewusst so gelassen | Der Build-Schritt selbst sieht `arm9.bin` weiter als ganze Datei: `g4k build` ohne Abgleich davor meldet bei einer DSPRE-Änderung außerhalb der Tabelle neben einer Header-JSON weiter `textproject.conflict`; ein `g4k text sync` (in der App jeder Fokus und jedes Bauen) löst es. Würde der Schritt dort schreiben und aufzeichnen, sähe der Abgleich die DSPRE-Änderung nie. Umgekehrt bleibt ein von Hand geänderter Blob `data/raw/arm9/arm9.bin` neben einer DSPRE-Änderung in der Tabelle ein Konflikt des Blobs (sicher, nur vorsichtig). Solange eine geteilte Datei ohne Hash ist, sieht die offene Einheit ein genaues Zurücksetzen nicht und bleibt Konflikt | – |

**Nicht geprüft in der Abnahme:** `scripts/gate.sh` (zuletzt grün auf `891e1e3`) und die gated Tests als Ganzes; nach dem Fix A1 lief `textproject_source` (Release, gated) auf `4-3b-abn-kpt` mit 9/9 (rund 46 min unter Last); die I2-Reproduktion von oben ergibt danach mit dem Release-`g4k` „dspre data/raw/arm9/arm9.bin“ und „files data/headers/“ ohne Konflikt, das Zurücksetzen entfernt den Blob, der Build hat die SHA-1 der Basis. Die Fortschrittsphasen beim Einrichten und im Assistenten (die Brücke reicht keinen `Channel` durch) und der Fokus im Tauri-Fenster bleiben Hand.

## Hand

- [ ] **DSPRE-Gegenprobe** (Windows): DSPRE öffnet einen eingerichteten Ordner (nach „Einrichten…“), ändert eine Attacke und einen Trainer und speichert; in der App beim Fensterfokus „2 Änderungen aus DSPRE übernommen“, die JSON tragen die Werte, `g4k build` baut sie. Dazu festhalten, was DSPRE beim Speichern schreibt (nur `unpacked/`, auch NARCs, auch `expanded/scripts/`) und ob ein gelöschter Member als fehlerhaft gemeldet wird.
- [ ] **Fokus in der gepackten Tauri-App:** ein anderes Programm in den Vordergrund, eine Datei außen ändern, zurück ins Studio: Abgleich und Meldung ohne Neuladen (im Browser-Modus nur über das `focus`-Ereignis geprüft).
- [ ] **Fortschritt beim Einrichten** in der App: die Phasen „Scripts übernehmen … {n} von {m}“, „Basisschicht anlegen …“, „Einträge schreiben …“, „Abgleich vorbereiten …“ und die laufende Phase „Textprojekt einrichten“ im Assistenten (die Brücke hat keinen `Channel`); ebenso „Gleiche mit DSPRE ab … {done} von {total} Dateien“ beim vollen Abgleich. Seit A2 bleibt der Dialog bis zur Meldung „Textprojekt eingerichtet: …“ stehen; im Tauri-Fenster nachsehen, dass zwischen der letzten Phase und der Meldung „Wird eingerichtet …“ steht.
- [ ] **ARM9 neben Headern (aus A1):** in DSPRE einen Header ändern und speichern, gleichzeitig im Studio (oder per Git) einen anderen Header in `data/headers/` ändern; beim Fokus kein Konflikt „Header-Tabelle“, beide Änderungen kommen an. Dazu festhalten, ob DSPRE beim Speichern eines Headers auch Bytes von `arm9.bin` außerhalb der Header-Tabelle schreibt.
- [ ] **Mit Virenscanner:** ein Umbenennen auf einem Rechner mit aktivem Defender oder einem anderen Echtzeit-Scanner; „Verweise werden gesucht …“ steht, bis der Plan da ist (A3), und die Zeit des Plans notieren (in der Abnahme 30 bis 36 s, H1).

## Blick

**Neu aufgenommen am 2026-10-31.** Die Bilder unter `target/agent/gate-2026-10-28/` und `gate-2026-10-29/` gingen mit dem Ordner `target/` verloren. Sie sind neu aufgenommen, auf dem Stand von main `03a5615` (alle Fixes bis 4.4), auf frischen Kopien: die Oberfläche in der Test-Umgebung mit Chrome headless (Playwright), das Spiel mit `g4emu`. Neuer Ort: `target/agent/gate-2026-10-31/blick-neu/4-3b/` im Haupt-Checkout (aufgenommen im Worktree `.claude/worktrees/blick-4`, Branch `blick-phase-4`), dazu eine Kopie außerhalb des Repos unter `$HOME/Projekte/lab/editor/_blick-phase-4/gate-2026-10-31/blick-neu/4-3b/`. Dort liegen die Bilder des UI-Tests direkt, die der Abnahme in `abnahme-4-3b/`. Es sind 125 Bilder:

- **UI-Test, 80 Bilder:** 79 Bilder der Oberfläche (`blick-*`, `mcp-01` bis `mcp-05` und `mcp-07`, 1440 × 900) und `emu/nach-intro.png`.
- **Abnahme, 45 Bilder:** `abn-01` bis `abn-40` und dazu `abn-31b`, außerdem `emu-ui/`, `emu-kpt/`, `emu-khgde/nach-intro.png` und `emu-khgus/titel.png`.

Abweichungen vom alten Inhalt:

- **Namen:** Die alte Liste der 80 Bilder stand in keiner Datei des Repos. Die Namen sind aus diesem Abschnitt rekonstruiert. Die Bilder, die nur unter Nummer liefen (`gate3-NN`, `mcp-04` bis `mcp-07`), haben jetzt sprechende Namen, etwa `blick-zuruecknehmen-dialog-*`, `blick-hier-spielen-konflikt-*`, `blick-pokedex-id-*`, `blick-textprojekt-zuletzt-uebernommen-*` und `mcp-04-data-edit-hell`. In der Abnahme sind `abn-02` bis `abn-04`, `abn-30` und `abn-31` nach der Tabelle benannt. `abn-31b` zeigt Technik › Textprojekt des neuen Projekts.
- **Behobene Befunde:** Die Bilder zeigen den Stand nach den Fixes:
  - U3: `blick-textprojekt-ungespeichert-*` mit der Spalte „Studio (ungespeichert)“.
  - U5 und U6: `blick-textprojekt-fehlerhaft-*` mit dem Untertitel „1 fehlerhafte Datei“.
  - U7: `blick-einrichten-fertig-hell` ohne Umbruch vor „s.“.
  - U4: `mcp-02-waehrend-hell` mit „Wird von einem Agenten eingerichtet …“ und gesperrter Taste.
  - U2: `mcp-03-nach-adopt-hell` ohne den Hinweis.
  - A5: `abn-01` mit „Basis-ROM.“ in einer Zeile, gleich wie `abn-38`.
  - A2: In `abn-39` liegt der Fokus auf der Überschrift „Textprojekt“ (über `document.activeElement` geprüft), ein Fokusring ist im Bild nicht zu sehen.
- **`abn-40-umbenennen-sucht-hell-nachfix`:** aufgenommen 380 ms nach dem Tippen. Ohne Virenscanner stand der Plan nach 3,5 s.
- **Seit 4.4 (Git):**
  - Der Assistent hat das Häkchen „Git einrichten (nur lokal)“. Es ist in den Hauptbildern aus, in `blick-assistent-vorlage-haekchen-git-hell` an.
  - Technik hat den Eintrag „Änderungen“.
  - Im UI-Test ist die Kopie über `g4k git init` versioniert; die Statusleiste zeigt dort „main · …“. Nach „Zurücknehmen…“ sagt die Meldung zusätzlich „Die .gitignore von Gen4 Studio versioniert jetzt den DSPRE-Ordner.“
  - Die Kopien der Abnahme haben kein Git.
- **Daten:** Konflikte und fehlerhafte Dateien betreffen andere Einträge als damals, etwa Konflikt Pokémon 0027 mit `personal.attack` 80 / 78 und fehlerhafte Datei 0028. Jedes ⌘S zeigt die Meldung „1 Datei(en) gespeichert“.
- **Prüfliste:** Die Prüfliste beim Einrichten hat eine dritte Zeile, „Scripts werden dabei übernommen“. In `-kompakt` steht „Scripts übernommen“, weil das Bild beim zweiten Einrichten nach „Zurücknehmen…“ entstand.
- **Browser-Modus:** „Hier spielen“ fragt zuerst nach dem Pfad, dann meldet es „Im Browser-Modus wird nur gebaut“. Fortschritt beim Einrichten und im Assistenten nur als „Wird eingerichtet …“ bzw. „Lesen läuft“ (Hand-Punkt).
- **Beobachtungen, kein Fix:**
  - **Verlaufsmenü bei langer ID** (`mcp-07-verlauf-menue-hell`, `abn-35-mcp-verlauf-hell`): Nach `rename_id` über MCP mit einer langen ID bricht die Zeile „ID umbenannt: TRAINER_…_MCP → …_MCP2“ um, und das lange Wort stößt ohne Abstand an die Plakette „agent-gate“. Erwartet: ein Abstand oder Umbruch vor der Plakette.
  - **Git-Zähler nach ⌘Z** (`blick-umbenennen-rueckgaengig-hell`): Nach ⌘Z eines „ID umbenennen…“ zeigt die Statusleiste noch „main · 2 Änderungen“, obwohl `git status` leer ist. Nach 8 s war es unverändert. Ob der nächste Fokus-Abgleich den Zähler richtet, ist nicht geprüft.
  - **Pokédex mit fehlerhafter Art-JSON:** Bei einer fehlerhaften `data/species/0028.json` zeigt der Pokédex „Die Liste ist nicht lesbar. 1 Fehler in Scripts oder Texten“ statt der Liste. Der Satz nennt weder die Datei noch die richtige Art von Fehler.
  - **Assistent, Schritt „Name und Ziel“:** „Weiter“ reagierte nach dem Tippen des Namens erst beim zweiten Klick, Enter im Feld tat nichts. Gesehen nur unter Playwright.

Bilder unter `target/agent/gate-2026-10-28/4-3b/` im Worktree (80 Oberflächen-Bilder, 1440 × 900, Hell, Dunkel, Kompakt, dazu `emu/nach-intro.png`), aufgenommen im UI-Test vor `fdb3483`; Liste mit Inhalt je Bild im Abschnitt (3). Der Agent hat alle angesehen: keine abgeschnittenen Texte, kein Überlauf, Dunkel mit Tokens, Kompakt ohne Bruch; optisch nur U7 (behoben). Vor dem Worktree-Entfernen `target/agent/gate-2026-10-28/` und `target/agent/gate-2026-10-29/` sichern.

- [ ] Technik › Textprojekt: `blick-textprojekt-dspre-*`, `-abgeglichen-*`, `-konflikt-diff-*`, `-fehlerhaft-*` (Untertitel dort noch „Abgeglichen“, U5), `-ungespeichert-dunkel` (Spalte dort noch „Studio“, U3).
- [ ] Meldungen: `blick-meldung-uebernommen-*`, `blick-textprojekt-konflikt-meldung-hell`.
- [ ] Einrichten: `blick-hinweis-ohnebasis-*`, `blick-hinweis-einrichten-*`, `blick-einrichten-pruefliste-*`, `-fortschritt-dunkel`, `-fertig-hell` (Umbruch „57 | s.“, U7).
- [ ] Umbenennen und IDs: `blick-trainer-kopf-id-*`, `blick-umbenennen-plan-*`, `-ablehnung-*`, `blick-header-id-hell`.
- [ ] Assistent: `blick-assistent-vorlage-haekchen-*`, `-zusammenfassung-hell`.
- [ ] Agent: `blick-mcp-frage-dunkel`, `mcp-02-waehrend-hell` (vor U4), `mcp-03-nach-adopt-hell` (vor U2).

Aus der Abnahme (2026-10-29), unter `target/agent/gate-2026-10-29/abnahme-4-3b/` im Worktree, 1440 × 900 außer den mit `-1100` (1100 × 720), aufgenommen vor den Fixes A2 bis A5 (die Bilder mit `-nachfix` danach: `abn-38-hinweis-ohnebasis-hell-nachfix` mit „Basis-ROM.“ in einer Zeile, `abn-39-fokus-nach-wahl-hell-nachfix` mit dem Fokus auf „Textprojekt“, `abn-40-umbenennen-sucht-hell-nachfix` mit „Verweise werden gesucht …“); vom Agenten angesehen, optisch nur A5:

- [ ] Hinweis und Einrichten: `abn-01-hinweis-ohnebasis-hell` (Umbruch „Basis-“ / „ROM.“, A5), `abn-02`, `abn-25-hinweis-einrichten-dunkel`, `abn-26-…-kompakt-1100`, `abn-05-einrichten-pruefliste-dunkel-kompakt`, `abn-06-einrichten-laeuft-…`, `abn-27-…-1100`, `abn-07-eingerichtet-…`.
- [ ] Technik › Textprojekt: `abn-04-technik-dspre-…`, `abn-10-zuletzt-uebernommen-hell`, `abn-12-konflikt-diff-hell`, `abn-14-ungespeichert-diff-dunkel`, `abn-16-fehlerhaft-dunkel`, `abn-37-konflikt-fehlerhaft-hell-kompakt-1100`.
- [ ] Meldungen: `abn-09`, `abn-11`, `abn-13`, `abn-22-hier-spielen-konflikt-dunkel`, `abn-36-meldungen-hell-kompakt-1100`.
- [ ] Umbenennen und IDs: `abn-08-pokedex-pikachu-id-hell`, `abn-17` bis `abn-21`.
- [ ] Zurücknehmen, Assistent, Agent: `abn-23`, `abn-24`, `abn-28` bis `abn-35`.
- [ ] Spiel: `emu-ui/nach-intro.png`, `emu-kpt/`, `emu-khgde/nach-intro.png`, `emu-khgus/titel.png`.

Vergleich mit dem Design-System: `design-system/README.md`, Abschnitt „Nachtrag 4.3b“.

## Geparkt

Offen für später, ohne Einfluss auf das Gate (aus den Reviews der Tasks, für das abschließende Review):

- **Kernel, Abgleich:** `sync/tests.rs` Prädikat `blob_member` trifft alle Member; `write_blobs` mit plain `fs::read` und frühem Längenvergleich kann nach einem Export eine einseitige Blob-Einheit melden; ein zusammengeführter Eintrag behält das alte `scanned_at_ns` (erster Scan hasht neu); CLI-Export parallel zum Studio kann ein Verwerfen zurücknehmen (Sperren je Prozess); ABA-Fenster im Millisekundenbereich; Speichern wartet auf einen laufenden Scan; eine Datei ohne Namen auf der Hand-Seite löscht der Export; ein Projekt ohne Aufzeichnung lehnt den JSON-Import ohne `--force` ab (dokumentiert); `mark_source_stale` nach dem Lösen der Sperre; `UnitCache` ohne racy-Schutz.
- **Kernel, Umstellung und IDs:** ein abgelehntes zweites `adopt` zeichnet InSync-Einheiten auf; ein aktuelles Manifest ohne Aufzeichnung überspringt den Schutz; `write_include` schluckt Fehler beim Erneuern; der zwischengespeicherte `ScriptLookup` kennt Änderungen an Textarchiven nicht; Namen in `world_meta` bei zu großen Nummern; `before_move` schluckt einen Fehler von `in_step`; Undo und Redo eines Umbenennens verwerfen dessen Warnungen; gepackte NARCs ohne `unpacked/` werden ganz gelesen (I3).
- **Löschen von Entitäten:** kommt mit 3d (Referenzsperre); ein von Hand gelöschtes letztes JSON legt der Export wieder an; ein Kombinationstest „überzähliger Member plus JSON-Änderung“ fehlt.
- **App:** `rename_id` hält das Sitzungsschloss die ganze Umbenennung (kalt 7,8 s auf HeartGold); ein Fehler in `after_member_write` wird nur geloggt; `sync_session`/`external_changes` sehen „busy“ des Stores nicht; `conflict_diff_impl` läuft unter dem Sitzungsschloss (einige 10 ms).
- **Oberfläche:** eine benannte Meldung, die die Grenze von vier Meldungen verdrängt, gilt als geschlossen (auch der bleibende Hinweis); `useExternalSync` lässt wartende Promises beim Aushängen offen; „Abbrechen“ beim Anheben zeigt kein „busy“, solange das Schließen wartet; Bauen speichert ungespeicherte Scripts auch über eine vom Reopen neu geschriebene Datei.
- **Design-System:** lange Zeilen im README-Nachtrag 4.3b, Namen der Entscheidungen im Plan-Link, eine ungerenderte Konstante der Meldungsposition; `.fig` (siehe „Hinweise“).

