# Phase 4.1b: Gate prüfen

Stand: 2026-10-27. Branch `phase-4-1b-auf-original` (Worktree `.claude/worktrees/phase-4-1b`, Basis main `df7d6bf`), nicht nach main gemergt. Gate vom Agenten bestanden: (1) Automatik mit den gated Tests auf Platin DE, HeartGold DE und HeartGold US (Stand `9c2b421`, HeartGold DE nach der Testanpassung `80b010b`), (2) Szenario kopflos über `studio mcp` und (3) Browser-Teil mit MCP über die Brücke; die sechs Befunde des UI-Tests (U1 bis U6) und die Punkte des abschließenden Reviews sind in einer Fix-Welle behoben (`bda7a27` bis `9c2b421`, Abschnitt „Befunde“). Der volle Lauf von `scripts/gate.sh` läuft. Beim Nutzer offen: das Kontextmenü mit Trackpad und Tastatur in der echten App und der Blick auf Dialog und Technik › Änderungen in Hell, Dunkel und Kompakt; Abschnitte „Hand“ und „Blick“.

Phase 4.1b bringt „Auf Original zurücksetzen“: Jede Entität der entschiedenen Arten (Karte, Eventdatei, Textarchiv je Nachricht und ganz, Trainer, Pokémon-Daten, Begegnungen, Header Feld für Feld, Scripts, Kartentexturen, Gebäudetexturen und Gebäudemodelle) lässt sich über „Auf Original zurücksetzen…“ im Menü „Aktionen“ ihres Kopfs, im Kontextmenü ihrer Liste und über MCP nach einer Rückfrage mit Vorher und Nachher als ein Undo-Schritt auf den Stand der Basis-ROM bringen; ihr Member ist danach byte-gleich zum Basis-Member. Store-Arten laufen über `Tx::replace` (Header über `Tx::edit_header` Feld für Feld), Scripts, Texturen und Modelle werden sofort geschrieben und liegen als Dateischritt auf demselben Undo-Stapel. Technik › Änderungen listet aus den Hashes des Build-Graphs (xxh3 je Basis-Member im App-Cache gegen die Member-Fingerabdrücke aus `.g4/build/state.json`), was im gespeicherten Projekt von der Basis-ROM abweicht. Neue Entitäten, ARM9-Patches, Overlays und Overlay-Tabellen lassen sich nicht zurücksetzen. Roadmap: [docs/ROADMAP.md](ROADMAP.md), Abschnitt „Phase 4“, Punkt „4.1b Auf Original zurücksetzen“. Entscheidungen: [entscheidungen.md](research/2026-10-27-phase-4/entscheidungen.md), Abschnitt „Basis-ROM und Patch (Gutachten B, 4.1)“, Zeilen „Auf Original zurücksetzen (4.1b)“ und „Liste „geändert gegenüber Original““. Plan: [2026-10-27-phase-4-1b-auf-original-zuruecksetzen.md](superpowers/plans/2026-10-27-phase-4-1b-auf-original-zuruecksetzen.md). Formate: [project_base.md](formats/project_base.md), Abschnitt „Auf Original zurücksetzen“; Werkzeuge: [mcp.md](mcp.md) (`changed_from_original`, `plan_reset`, `reset_to_original`).

Gate-Text der Roadmap (Punkt 4.1b):

> Gate: je Art aus dieser Liste ändern, zurücksetzen, der Member ist byte-gleich zur Basis, ⌘Z stellt die Änderung wieder her, ein Reset dauert ≤ 0,2 s; neue Entität: Eintrag deaktiviert mit Grund; über die Test-Umgebung und MCP.

Wer was prüft: Die Spalte „Wer“ sagt **Agent** (die Test-Umgebung aus [docs/agent-gate.md](agent-gate.md): Browser-Fenster mit `studio-bridge`, `call.sh`, `mcp.sh`, dazu `studio mcp` kopflos und die gated Tests), **Agent + Blick** (der Agent prüft und beschreibt Ort und Zahlen, der Nutzer bestätigt kurz am Bild) oder **Hand** (der Nutzer). Befehle: [docs/agent-gate.md](agent-gate.md), Abschnitt „Phase 4.1b“.

Namen in der Oberfläche (Rollen und `data-*` für die Prüfung):

- **Menü „Aktionen“:** im Inspector-Kopf das vorhandene Menü (Symboltaste `more`); in Pokédex, Trainer und Begegnungen ein neues Menü „Aktionen“ am Ende der Kopfzeile (`HeadActions`), in der Hand der Texte hinter „Nachricht anlegen“ und im Kopf des Script-Panels. Einträge (Symbol `undo`): „Auf Original zurücksetzen…“, wo Kopf oder Zeile genau das Ziel nennt, sonst mit der Art: „Eventdatei auf Original zurücksetzen…“, „Header …“, „Scriptdatei …“, „Kartentextur …“, „Gebäudetextur …“, „Gebäudemodell …“, „Nachricht …“, „Ganzes Textarchiv auf Original zurücksetzen…“. Gesperrt mit leiser Notiz rechts: „Wird gelesen …“, „Basis-ROM fehlt“, „Kein Original“ (dazu „Keine Nachricht gewählt“, „Kein Script geöffnet“).
- **Kontextmenü** an `ListRow` und `TreeRow` (`contextMenu`): Rechtsklick, Shift+F10 oder Kontextmenü-Taste auf der fokussierten Zeile; über die Tastatur hat der erste wählbare Eintrag den Fokus. Karten haben keine Liste, sie gehen nur über den Kopf des Zellen-Inspectors.
- **Dialog** `[data-reset-dialog]`, Titel „<Name> auf Original zurücksetzen?“ (etwa „Karte 0003 auf Original zurücksetzen?“, „Nachricht 0 in Textarchiv 0001 …“, „Pokémon 0397 …“), Beschreibung „Stellt den Stand der Basis-ROM <Name> wieder her. ⌘Z nimmt es zurück.“; je Änderung `[data-change]` mit sichtbarer Überschrift (Name vom Kern) und Vorher und Nachher (`BeforeAfter` mit `[data-wrap]`); beim Header statt dessen je Feld ein Kästchen `[data-header-field=<feld>]` mit „vorher → nachher“. Tasten „Abbrechen“ und „Zurücksetzen“ (primär); „Schon im Stand der Basis-ROM.“ und ein abgelehnter Plan nur mit „Schließen“. Danach die Meldung „Auf Original zurückgesetzt: <Name>.“ und im Verlauf „Auf Original zurückgesetzt: <Name>“.
- **Technik › Änderungen:** Listeneintrag „Änderungen“ nach „Veröffentlichen“ (Symbol `changes`), Untertitel „Basis-ROM fehlt“, „Vergleiche …“, „Noch nicht verglichen“, „Keine Abweichung“, „1 Eintrag weicht ab“, „<n> Einträge weichen ab“; die Fläche `[data-technik=aenderungen]` mit „Neu vergleichen“, je Gruppe ein Abschnitt `[data-changes-group=<Gruppe>]` („Karten“, „Trainer“, „Header“ …) und je Abweichung eine Zeile `[data-change=<Name>]` mit Plakette „Geändert“, „Neu“ oder „Fehlt“ (dazu „Ungespeichert“) und der kleinen Taste „Auf Original zurücksetzen…“, gesperrt mit Tooltip „Kein Original“ oder „Für diese Art gibt es kein Zurücksetzen.“

## Vorbereitung

So durchgeführt am 2026-10-27:

1. **Frische Kopien** unter `Roms/_testkopien/` per `cp -cR` (Ordner) und `cp -c` (`.nds`) aus den Originalen, die dabei nur gelesen wurden; Ordner `4-1b-<zweck>_DSPRE_contents` und `.nds` `4-1b-<zweck>.nds` direkt nebeneinander, damit `start::suggest_base` die Basis-ROM findet:
   - `4-1b-pt` (Platin DE), `4-1b-hgde` (HeartGold DE), `4-1b-hgus` (HeartGold US) für die gated Tests von (1); sie werden nur gelesen, die Tests schreiben auf `vanilla_copy()`;
   - `4-1b-kpt`, `4-1b-khgde`, `4-1b-khgus` für (2), je mit `g4k base assign` auf die Nachbar-`.nds` zugeordnet;
   - `4-1b-web` (Platin DE) für den Browser-Teil, `4-1b-mcp` (Platin DE) für MCP über die Brücke.

   Die Kopien von (2) und (3) sind nach dem Gate gelöscht; `4-1b-{pt,hgde,hgus}` vor dem Merge löschen.
2. **Werkzeuge:** `cargo build --workspace --all-features` (Debug: `target/debug/studio` für (2), `target/debug/g4k` für `base assign` und `extract`); die Brücke baut `studio-web.sh` selbst. Aus dem Worktree `G4_ROMS_DIR` absolut setzen und `tools/` als Symlink auf den Haupt-Checkout legen (apicula, rotom; den Symlink nie einchecken). `G4_BASE_ROM_DIR=<Roms>/_testkopien`, nie die Originale.
3. **Test-Umgebung:** `G4_ROMS_DIR=<Roms> G4_VITE_PORT=1540 G4_BRIDGE_PORT=1541 bash scripts/agent/studio-web.sh`; Browser-Fenster der Desktop-App 1440 × 900, Hell, Komfort.
4. **Ausgaben** unter `target/agent/gate-2026-10-27/4-1b/` im Worktree: `gated-original-<k>.log` und `gated-studio-<k>.log` (1), `kopflos-<k>.log` (2), die 17 Blick-Bilder `blick-*.png`. Das Python-Skript des kopflosen Teils (MCP über stdio) und das Node-Skript für die Bilder (Chrome headless über CDP) lagen im Scratchpad der Sitzung, nicht im Repo. Vor `git worktree remove` den Ordner `target/agent/gate-2026-10-27/` sichern: er liegt im `target/` des Worktrees und geht sonst mit ihm verloren.

## (1) Automatik

- [x] 1. Gated Tests in `crates/g4-kernel/tests/original.rs` auf den Kopien `4-1b-{pt,hgde,hgus}` (die Basis-ROM ist die Nachbar-`.nds`, gefunden über `test_support::vanilla_base_rom()` und nur gelesen; was schreibt, arbeitet auf `vanilla_copy()`), Debug-Build:
   - `reset_of_each_store_kind_gives_the_base_member_back`: Karte 3, Eventdatei 3, Textarchiv 0 ganz, Nachricht 0 in Textarchiv 1, Trainer 1, Pokémon 1 (Personal, Lernset, Entwicklung), Begegnungsdatei 1 je ändern, speichern, zurücksetzen, speichern: jeder Member byte-gleich zum Basis-Member, jeder Reset ≤ 200 ms; ⌘Z stellt die Änderung wieder her;
   - `header_reset_field_by_field_gives_the_base_record_back`: Header mit zwei geänderten Feldern, erst ein Feld (`weather_id`), dann der Rest, dazu ein Lauf mit Dynamic-Headers-Patch (Basis im ARM9, Projekt im NARC): der gespeicherte Datensatz gleich dem der Basis, danach plant `plan_reset` `same`; ⌘Z führt zum halb zurückgesetzten Stand;
   - `script_texture_and_model_reset_give_the_base_member_back`: nach `scripts::adopt` Scriptdatei, Kartentextur und Gebäudemodell als Dateischritt: Member byte-gleich, ⌘Z schreibt die alten Bytes zurück;
   - `changed_list_after_edits_names_them_and_after_reset_not`: nach einem Build die Liste leer (erster Lauf rechnet die Basis-Hashes und trägt fehlende Hashes nach, der zweite hasht nur racy Einträge, ≤ 2 s), nach den Änderungen genau „Karte 0003“ und „Trainer 0001“, nach dem Zurücksetzen der Karte nur noch „Trainer 0001“;
   - `decompile_member_gives_the_source_rotom_writes_for_every_script_file`: der Sweep (auf Dateien mit globalen Aufrufen beschränkt) vergleicht `decompile_member` mit rotoms eigenem Text: Platin DE 202 Dateien in 15,9 s, HeartGold DE und US je 234 Dateien in 15,1 bzw. 15,5 s.

   Platin DE 5/5 (205 s), HeartGold US 5/5 (236 s). HeartGold DE im ersten Lauf 4/5: `changed_list…` erwartete im zweiten Lauf `hashed == 0`, gehasht wurden 244 racy Einträge des frischen Builds (Folge der Regel „racy wird immer gehasht“ aus `3d5d7f1`); die Testannahme ist in `80b010b` korrigiert (racy Einträge gezählt), danach grün: `changed IPKD first_ms=10133 second_ms=344 files=19497 recorded=19209 racy=288`.
- [x] 2. Gated Studio-Test `reset_over_the_command_path_on_the_vanilla_copy` (`apps/studio/src-tauri/src/commands/original.rs`): Liste, Plan und Reset über den Befehlsweg der App auf `vanilla_copy()`, dazu die Prüfung aus U6, dass `ms` das Öffnen der Basis-ROM mitzählt (bei gehaltenem Schloss `ms >= 140`). 1/1 auf allen drei: Reset Platin DE 10 ms, HeartGold DE 11 ms, HeartGold US 31 ms; Liste erst 12,8 / 17,0 / 13,8 s (ungebautes Projekt, alle Dateien gehasht).
- [ ] 3. `scripts/gate.sh` gegen alle drei Projekte: läuft.

Einzeln, je Projekt, aus der Wurzel des Worktrees (so gelaufen am 2026-10-27, Logs `gated-original-<k>.log`, `gated-studio-<k>.log`):

```sh
export PATH="$HOME/.cargo/bin:/opt/homebrew/opt/rustup/bin:$PATH"
G4_TEST_PROJECT="<Roms>/_testkopien/4-1b-pt_DSPRE_contents" \
  cargo test -p g4-kernel --all-features --test original -- --nocapture --test-threads 1
G4_TEST_PROJECT="<Roms>/_testkopien/4-1b-pt_DSPRE_contents" \
  cargo test -p studio --all-features --lib reset_over_the_command_path -- --nocapture --test-threads 1
```

Ebenso mit `4-1b-hgde` und `4-1b-hgus`. Auf stderr stehen die Zeilen `reset <Spielcode> <Ziel> ms=… (kernel … ms)`, `changed <Spielcode> first_ms=… second_ms=… files=… recorded=…` und `decompile <Spielcode> <n> files with global calls ms=…`; der Studio-Test druckt `reset over the command path: … (kernel … ms)` und die Zeiten der Liste.

Zeiten je Reset in ms (gated, Debug-Build; vom Öffnen der Basis-ROM mit `BaseRom::open_unchecked` bis zum aufgezeichneten Schritt; den Kernel-Anteil ohne das Öffnen nennt das Log in Klammern). Header als ein Feld (`weather_id`) / der Rest der Felder / mit Dynamic-Headers-Patch. Liste: erster Lauf (Basis-Hashes rechnen, fehlende Hashes nachtragen) / zweiter Lauf, nach einem Build.

| Spiel | Karte | Eventdatei | Textarchiv | Nachricht | Trainer | Pokémon | Begegnung | Header | Script | Textur | Modell | Liste erst / warm (ms) |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Platin DE | 22 | 16 | 61 | 10 | 9 | 11 | 9 | 25 / 20 / 93 | 96 | 21 | 12 | 15 090 / 101 |
| HeartGold DE | 34 | 18 | 11 | 14 | 10 | 10 | 9 | 31 / 25 / 68 | 91 | 21 | 19 | 16 560 / 566 (grüne Wiederholung 10 133 / 344) |
| HeartGold US | 29 | 16 | 10 | 13 | 10 | 12 | 10 | 30 / 25 / 68 | 108 | 20 | 12 | 16 502 / 120 |

Grenze 0,2 s je Reset: eingehalten, 9 bis 108 ms im Debug-Build (die App läuft im Release-Profil). Das Textarchiv 0000 von Platin (1404 Nachrichten) kostet 51 ms im Kernel für das Kodieren; das Script 62 bis 79 ms für Dekompilieren und Zustand von rotom. Die HeartGold-DE-Zeiten der Resets stammen aus dem ersten Lauf (die Reset-Tests waren dort grün), die Liste aus der Wiederholung.

## (2) Szenario kopflos

MCP über stdio, je Spiel auf seiner Kopie (`K=<Roms>/_testkopien`, `<k>` = `kpt`, `khgde`, `khgus`), Debug-Build:

```sh
target/debug/g4k base assign $K/4-1b-<k>_DSPRE_contents $K/4-1b-<k>.nds
target/debug/studio mcp --project $K/4-1b-<k>_DSPRE_contents --base-rom $K/4-1b-<k>.nds
# über JSON-RPC (Skript im Scratchpad):
#   changed_from_original                                   → leer
#   data_edit (Trainer 1, Level +1), set_header (Header 3, weather_id), edit_text (Archiv 1, Nachricht 0), save
#   changed_from_original                                   → „Textarchiv 0001“, „Trainer 0001“, „Header 0003“
#   plan_reset {target} je Ziel; reset_to_original {target} je Ziel; save
#   changed_from_original                                   → leer
#   undo; reset_to_original {"target":{"kind":"trainer","index":5000}}; g4_batch mit reset_to_original eines Scripts
target/debug/g4k extract $K/4-1b-<k>.nds <tmp>              # Basis zum Vergleich, dann cmp der drei Member
```

- [x] 1. `changed_from_original` auf der unberührten Kopie: leer, auf allen drei (alle Dateien gehasht: Platin 18 004, HeartGold 19 497).
- [x] 2. Nach Trainer 1 (`data_edit`, Level +1), Header 3 (`set_header`, `weather_id`), Nachricht 0 von Archiv 1 (`edit_text`, „Studio“) und `save` nennt die Liste genau „Textarchiv 0001“, „Trainer 0001“, „Header 0003“.
- [x] 3. `plan_reset` je Ziel mit `same: false` und Vorher und Nachher; Header mit `fields` `weather_id` 1 → 0; die Nachricht „Studio“ → der Text der Basis (Platin „SPIELER: {STRVAR_1, 3, 0, 0}“, HeartGold DE „ZURÜCK“, HeartGold US „CANCEL“).
- [x] 4. `reset_to_original` je Ziel `recorded: true`, Wand-Zeit über MCP 117 bis 155 ms (mit dem Prozessweg des Skripts; der gemeldete `ms` zählte damals das Öffnen der Basis-ROM nicht mit, U6, seitdem behoben), `save`; danach ist die Liste leer.
- [x] 5. `cmp` gegen die mit `g4k extract` ausgepackte Basis-ROM: `trainerProperties/0001`, `trainerParty/0001` und `textArchives/0001` byte-gleich, auf allen drei.
- [x] 6. `undo` stellt die letzte Änderung wieder her (die Nachricht, danach plant `plan_reset` wieder `same: false`).
- [x] 7. Trainer 5000 (Index hinter der Anzahl der Basis) → `no_original`, „Kein Original: diesen Eintrag gibt es in der Basis-ROM nicht.“
- [x] 8. `g4_batch` mit `reset_to_original` eines Scripts → abgelehnt: „Aufruf 0 (reset_to_original): reset_to_original geht nicht in g4_batch“.

| Spiel | Liste unberührt, erst / zweiter Lauf (s) | nach den Änderungen (s) | nach dem Zurücksetzen (s) | Reset Trainer / Header / Nachricht, Wand-Zeit (ms) | Member gleich |
|---|---|---|---|---|---|
| Platin DE | 30,0 / 3,5 | 1,6 | 1,7 | 119 / 124 / 117 | ja |
| HeartGold DE | 49,8 / 13,8 | 11,6 | 6,7 | 126 / 146 / 155 | ja |
| HeartGold US | 26,5 / 44,1 | 40,9 | 11,3 | 119 / 129 / 124 | ja |

Die Liste kopflos hat keinen App-Cache für die Basis-Hashes und lief auf ungebauten Kopien (jeder Ordner ohne Build-Eintrag wird bei jedem Lauf gehasht, `unbuilt`); der Rechner war unter Last (zwei weitere Sitzungen liefen), daher die Streuung bei HeartGold. Logs `kopflos-{pt,hgde,hgus}.log`.

## (3) Browser-Teil

Test-Umgebung auf 1540/1541, Kopie `4-1b-web` (Platin DE, Basis-ROM zugeordnet), Seite `http://127.0.0.1:1540/?project=<Roms>/_testkopien/4-1b-web_DSPRE_contents`.

- [x] 1. Ohne Zuordnung: Technik › Änderungen „Diesem Projekt ist keine Basis-ROM zugeordnet.“, die Einträge gesperrt mit „Basis-ROM fehlt“.
- [x] 2. Je Art geändert und gespeichert, Technik › Änderungen nennt alle zehn Arten (Karte, Eventdatei, Textarchiv, Trainer, Pokémon, Begegnung, Header, Script, Kartentextur, Gebäudemodell).
- [x] 3. Zurückgesetzt über jeden Weg: Kopf „Aktionen“ (Trainer, Pokédex), Kontextmenü per Rechtsklick (Pokédex), per Shift+F10 (Trainer; nach U3 öffnet Shift+F10 mit dem Eintrag im Fokus, Enter öffnet den Dialog), Menü „Aktionen“ der Text-Hand (Nachricht), Kontextmenü der Archivliste (ganzes Archiv: „Schon im Stand der Basis-ROM.“, nachdem die Nachricht zurückgesetzt war), die Zeilen von Technik › Änderungen (Karte, Eventdatei, Begegnung, Header mit abgewähltem Feld „Wetter“, Script, Kartentextur, Gebäudemodell). Je Dialog Vorher und Nachher (Header nur mit Häkchen), „Zurücksetzen“, gespeichert: Technik › Änderungen ohne den Eintrag, am Ende leer.
- [x] 4. ⌘Z und ⌘⇧Z beim Header stellen die Änderung wieder her und nehmen sie wieder zurück; ⌘Z eines Dateischritts (Gebäudemodell) schreibt die Datei zurück. Eine außen geänderte Kartentextur: ⌘Z meldet „Kartentextur 0000 wurde seit dem Zurücksetzen außerhalb des Studios geändert; …“, der Schritt fällt weg, das nächste ⌘Z nimmt den älteren Schritt.
- [x] 5. Neue Entität: eine neue Begegnungsdatei 0183 → Eintrag gesperrt mit „Kein Original“, in Technik › Änderungen mit „Neu“.
- [x] 6. `.nds` umbenannt: `original_status` `unavailable`, Technik › Änderungen „Basis-ROM nicht gefunden: Platin DE (Testprojekt) (d40f19ac…).“, die Einträge gesperrt mit „Basis-ROM fehlt“.
- [x] 7. MCP über die Brücke (Kopie `4-1b-mcp`, Freigabe „Lesen und bearbeiten“): `mcp.sh call reset_to_original '{"target":{"kind":"trainer","index":1}}'` ohne Rückfrage; `history` zeigt `undo_agents: ["agent-gate"]`, die Oberfläche lädt den Trainer neu (Lv. 9 → 5); `no_original` und die Ablehnung im Batch auch über die Brücke.

Im Browser-Modus zeigt die Liste während des Vergleichs nur „Vergleiche …“: die Brücke hat keinen `Channel` für die Zahlen „Vergleiche {done} von {total} Dateien …“ (so dokumentiert in `api.original.ts`); in der App kommen sie über `Channel` (Hand-Punkt).

## (4) Hand und Blick

Beim Nutzer (Abschnitte „Hand“ und „Blick“ unten): das Kontextmenü mit Trackpad und Tastatur in der echten App (WebKit), die Fortschrittszahlen der Liste in der App, die englischen Feldnamen in den Vorschau-Zeilen und der Blick auf Dialog und Technik › Änderungen in Hell, Dunkel und Kompakt.

## Ergebnis

| Punkt | Wer | Ergebnis | Datum |
|---|---|---|---|
| (1) 1 gated `--test original` | Agent | bestanden: je 5/5 auf Platin DE, HeartGold DE (nach `80b010b`), HeartGold US; jeder Member byte-gleich zur Basis, ⌘Z stellt wieder her, Reset 9 bis 108 ms (Tabelle oben) | 2026-10-27 |
| (1) 2 gated Studio-Test | Agent | bestanden: 1/1 auf allen drei, Befehlsweg 10 / 11 / 31 ms, `ms` mit dem Öffnen der Basis-ROM (U6) | 2026-10-27 |
| (1) 3 `scripts/gate.sh` | Agent | läuft | – |
| (2) 1 bis 4 Liste, Plan, Reset kopflos | Agent | bestanden auf allen drei: Liste leer, nach den Änderungen genau drei Einträge, nach dem Zurücksetzen leer; Wand-Zeit 117 bis 155 ms | 2026-10-27 |
| (2) 5 Member gegen die extrahierte Basis | Agent | bestanden: drei Member byte-gleich auf allen drei | 2026-10-27 |
| (2) 6 bis 8 `undo`, `no_original`, Batch | Agent | bestanden | 2026-10-27 |
| (3) 1 und 6 ohne Basis-ROM | Agent | bestanden: Grund in Technik › Änderungen, Einträge gesperrt mit „Basis-ROM fehlt“ | 2026-10-27 |
| (3) 2 bis 4 je Art, alle Wege, ⌘Z | Agent | bestanden nach U1 bis U5 | 2026-10-27 |
| (3) 5 neue Entität | Agent | bestanden: „Kein Original“, in der Liste „Neu“ | 2026-10-27 |
| (3) 7 MCP über die Brücke | Agent | bestanden: ohne Rückfrage, Agent im Verlauf, Oberfläche lädt neu | 2026-10-27 |
| Kontextmenü mit Trackpad, Shift+F10, Kontextmenü-Taste in der App | Hand | offen | – |
| Fortschrittszahlen der Liste in der App | Hand | offen | – |
| Englische Feldnamen in den Vorschau-Zeilen | Hand | offen (zu beurteilen) | – |
| Blick Dialog und Technik › Änderungen | Agent + Blick | offen: 17 Bilder liegen bereit, vom Agenten angesehen | – |
| `.fig` | Agent | nicht eingespielt (Entscheidung des Plans); Vorlage `design-system/openpencil/beispiele/auf-original.body.js`, Anleitung `design-system/openpencil/NACHTRAG-4-1b.md` | 2026-10-27 |

## Befunde

Befunde des UI-Tests, alle behoben in der Fix-Welle nach dem abschließenden Review und im Browser nachgeprüft:

| Nr. | Befund | Fix |
|---|---|---|
| U1 | `BeforeAfter` brach lange Zeilen im Dialog nicht um; bei Nachrichten, Eventdateien (JSON je Event) und Kartenzeilen war der Unterschied ohne seitliches Rollen nicht zu sehen. | `a2dd920`: `BeforeAfter` bekommt `wrap` (Umbruch in der Spalte, Folgezeilen hängen um `raum/16` ein), der Dialog nutzt es. |
| U2 | Der Name einer Änderung („Karte 0003: 1 Zelle der Bewegungsrechte anders“, beim Pokémon Basisdaten, Lernset, Entwicklung) stand nur als `aria-label`. | `a2dd920`: sichtbar über jedem Vorher und Nachher. Dazu zeigt der Header nur noch die Häkchen mit deutschen Feldnamen (vorher zusätzlich die Zeilen mit den Namen des Formats). |
| U3 | Nach Shift+F10 lag der Fokus auf dem Menü, kein Eintrag war hervorgehoben, Enter tat nichts. | `8c4484a`: über die Tastatur geöffnet fokussiert das Menü den ersten wählbaren Eintrag (WAI-ARIA), nach einem Rechtsklick ist keiner hervorgehoben. |
| U4 | „Noch nicht gebaut: …“ nannte bei einem nie gebauten Projekt 36 Ordner. | `9c2b421`: höchstens fünf mit Namen, dann „und {n} weitere“. |
| U5 | Nach einem Reset einer Store-Art verglich Änderungen neu, obwohl der gespeicherte Stand gleich blieb (auf einem ungebauten Projekt 30 bis 60 s Hashen je Reset). | `be22267`: neu verglichen wird nur nach Schritten, die eine Datei schreiben (Script, Textur, Modell), und nach dem Speichern. |
| U6 | `ms` von `reset_to_original` zählte ab `original::reset`, ohne Finden und Öffnen der Basis-ROM (gemeldet 0 bis 12 ms, Wand-Zeit 120 bis 155 ms). | `4e7bdba`: `ms` ab dem Eintritt in den Befehl; der gated Studio-Test prüft es. |

Aus dem abschließenden Review behoben (dieselbe Welle): App-⌘Z eines Script-Resets lädt den offenen Editor neu und Änderungen vergleicht neu (`be22267`); eine später auftauchende Basis-ROM wird übernommen, ein Gebäudemodell ohne Area-Data setzt das Außenmodell zurück statt bei „Wird gelesen …“ zu bleiben (`29130ee`); Undo und Redo eines Dateischritts melden das Schreiben, „Script anfügen“ und die Übernahme vergessen die Dateischritte (`4e7bdba`); die Meldung beim Redo nennt ihre eigene Richtung, ein nicht beschreibbarer Cache lässt die Liste nicht scheitern (`bda7a27`); racy Einträge werden immer gehasht, Arten ohne Zurücksetzen heißen im Singular („Matrix 0000“) (`3d5d7f1`); eine anders geschriebene, aber zur Basis kompilierende Scriptdatei gilt als gleich (`1ee4bc9`); der Katalogtext von `changed_from_original` sagt, dass ein nie gebautes Projekt bei jedem Lauf gehasht wird (`8449342`).

Befund im Gate-Lauf selbst: der gated Test der Liste auf HeartGold DE (siehe (1) 1), Testannahme in `80b010b` korrigiert; kein Fehler im Produkt.

Bewusst nicht geändert (Entscheidung): das doppelte „⌘Z nimmt es zurück.“ im Dialog eines Dateiziels (Beschreibung und Hinweis, beide wörtlich aus dem Plan); der Hinweis „Die Basis-ROM ordnest du in Technik › Veröffentlichen zu.“ auch bei zugeordneter, aber fehlender ROM (Plan wörtlich; Veröffentlichen bietet dort „Basis-ROM suchen…“); die englischen Feldnamen in den Vorschau-Zeilen von Trainer, Pokémon und Begegnung (der Plan legt `RawRecord::FIELDS` und die serde-Namen fest; Blick-Punkt).

## Hand

- [ ] **Kontextmenü in der App** (WebKit, Tauri auf macOS): auf einer Zeile der Trainer-Liste Zwei-Finger-Klick auf dem Trackpad, dann Shift+F10 auf der fokussierten Zeile: öffnet das Menü mit „Auf Original zurücksetzen…“, über die Tastatur mit dem Eintrag im Fokus, Enter öffnet den Dialog. Ob Shift+F10 in WebKit ankommt oder vom System abgefangen wird, zeigt nur die echte App. Die Kontextmenü-Taste fehlt auf Mac-Tastaturen (mit einer PC-Tastatur prüfen, wenn zur Hand).
- [ ] **Fortschritt der Liste in der App:** Technik › Änderungen auf einem nie gebauten Projekt öffnen: „Vergleiche 312 von 4096 Dateien …“ zählt hoch (im Browser-Modus nur „Vergleiche …“).
- [ ] **Englische Feldnamen** in den Vorschau-Zeilen von Trainer, Pokémon und Begegnung (etwa `flags: 0`, `level: 5`): beurteilen, ob sie bleiben dürfen oder deutsche Namen brauchen.

## Blick

Bilder unter `target/agent/gate-2026-10-27/4-1b/` im Worktree, je 1440 × 900, aus der laufenden Test-Umgebung über Chrome headless (CDP; Hell und Dunkel über `prefers-color-scheme`, Kompakt über die Taste der Statusleiste), Stand nach der Fix-Welle. Die Werkzeuge des Browser-Fensters liefern Bilder nur an den Agenten und schreiben keine Datei.

Die Dialoge liegen über Technik › Änderungen der Kopie `4-1b-web` (sieben Einträge in sieben Gruppen, darunter „Begegnungsdatei 0183“ mit „Neu“ und gesperrter Taste, die Info „Noch nicht gebaut: maps, matrices, areaData, eventFiles, textArchives und 31 weitere. …“).

- [ ] `blick-dialog-karte-hell.png`, `-dunkel.png`, `-kompakt.png`: Dialog „Karte 0003 auf Original zurücksetzen?“, Überschrift „Karte 0003: 1 Zelle der Bewegungsrechte anders“, Vorher und Nachher mit umgebrochenen Kartenzeilen („Bewegungsrechte 58149cf8“ → „ce31df4b“ markiert, „Gebäude 0“, „Modell 7688 Byte 334bf7af“, „BDHC 66 Byte 02d606f6“), „Abbrechen“ und „Zurücksetzen“ (primär).
- [ ] `blick-dialog-header-hell.png`, `-dunkel.png`, `-kompakt.png`: Dialog „Header 0003 auf Original zurücksetzen?“ ohne Vorher und Nachher, drei angehakte Felder mit den Namen aus dem Inspector und „vorher → nachher“ in Mono: „Musik Tag“ 1011 → 1010, „Wetter“ 2 → 0, „Kamera“ 1 → 0.
- [ ] `blick-dialog-script-hell.png`, `-dunkel.png`, `-kompakt.png`: Dialog „Scriptdatei 0010 auf Original zurücksetzen?“ mit der Info „Wird sofort geschrieben; ⌘Z nimmt es zurück.“; die Kopie hat die Scripts nicht übernommen, darum Vorher und Nachher als „Binär, 1684 Byte“ mit der xxh3 (die Zeile der xxh3 markiert).
- [ ] `blick-aenderungen-liste-hell.png`, `-dunkel.png`, `-kompakt.png`: Technik › Änderungen mit Untertitel „7 Einträge weichen ab“, den Gruppen mit Zähler, Plaketten „Geändert“ und „Neu“ und den kleinen Tasten „Auf Original zurücksetzen…“, keine primäre Taste auf der Fläche.
- [ ] `blick-aenderungen-fortschritt-hell.png`: der laufende Vergleich, Untertitel „Vergleiche …“, auf der Fläche „Vergleiche …“ und leise „Das erste Mal dauert länger; danach geht es schnell.“, „Neu vergleichen“ gesperrt (Browser-Modus: ohne Zahlen).
- [ ] `blick-aenderungen-leer-hell.png` (Kopie `4-1b-mcp`): Untertitel „Keine Abweichung“, die Info „Noch nicht gebaut: … und 31 weitere. …“ (U4) und „Alles wie in der Basis-ROM.“
- [ ] `blick-aenderungen-ohne-basis-hell.png` (Kopie `4-1b-mcp`, `.nds` umbenannt): Warnmeldung „Basis-ROM nicht gefunden: Platin DE (Testprojekt) (d40f19ac…).“ und „Die Basis-ROM ordnest du in Technik › Veröffentlichen zu.“; Untertitel von Veröffentlichen und Änderungen „Basis-ROM fehlt“.
- [ ] `blick-kontextmenue-liste-hell.png`: das Kontextmenü einer Zeile des Pokédex mit „Auf Original zurücksetzen…“ und Symbol `undo`.
- [ ] `blick-kopf-aktionen-hell.png`: das offene Menü „Aktionen“ (`more`) im Kopf von BISASAM 0001 mit „Auf Original zurücksetzen…“.

Vergleich mit dem Design-System und die Abweichungen: `design-system/README.md`, Abschnitt „Nachtrag 4.1b“.

## Geparkt

Offen für später, ohne Einfluss auf das Gate (aus den Reviews der Tasks und dem Plan):

- **Folgephasen:** „Öffnen“ aus der Liste und der Umschalter „gegen Basis / gegen letzten Commit“ kommen mit 4.4; ob die Liste bei Textarchiven die JSON einbezieht (eine neuere JSON als der Member), entscheidet 4.4 (Diff je Nachricht). Matrix, Gebietsdaten, Attacken und Items stehen nur in der Liste, ein Zurücksetzen dafür braucht eine Entscheidung des Nutzers.
- **Merge mit 4.3a:** einziger Textkonflikt im Probe-Merge `crates/g4-kernel/Cargo.toml` (`serde_json` mit `preserve_order` auf beiden Seiten, eine Zeile behalten); `build.rs` hat neue Re-Exporte (`pack_entries`, `is_racy`). Ein voller `textproject`-Lauf (4.3a) überschreibt einen auf die Basis zurückgesetzten Member mit der JSON der Entität: in 4.3b beachten.
- **Store und Dateischritt:** das Verwerfen eines gemischten Changesets nimmt auch die Store-Teile mit, `rollback_to` bricht dort ab; kein Test für ein Zurückrollen nach halbem Schreiben; `net_files` vergleicht Pfade ohne Normalisierung; `record_file_step` nimmt einen leeren Schritt an.
- **Kernel:** die Nachricht dekodiert das Basis-Archiv zweimal, `BaseRom::count` parst ganze NARCs; die Verfügbarkeit von Pokémon kommt nur aus der Personal-Anzahl; kein Test für Trainer-Archive ungleicher Länge; `header_fields` schluckt serde-Fehler; doppeltes `current_member` beim Script; Testlücken beim Innenraum-Gebäudemodell und beim Wechsel Script ↔ Level-Script; die Gleich-Regel des Scripts kennt rotoms Abhängigkeitshashes nicht.
- **Befehle und Oberfläche:** Fehlercodes als String verglichen; `G4_APP_DATA_DIR` trennt den Cache der Basis-Hashes nicht; der Dialog setzt seinen Zustand bei einem Zielwechsel nicht selbst zurück (der `key` deckt es); die Notiz „ungespeichert“ steht bei jeder Header-Zeile; kein `aria-keyshortcuts`; eine gesperrte Zeile öffnet ihr Kontextmenü; kein eigener Hinweis, wenn ein Script-Dokument nach dem Zurücksetzen geschlossen wird; ein zweites `getAreaData` je Szene; doppelte Vergleiche bei ⌘S mit Bauen.
