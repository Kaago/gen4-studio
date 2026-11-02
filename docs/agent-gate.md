# Gate mit dem Agenten

Diese Anleitung sagt einem Claude-Agenten, wie er die Gates [phase-3a-gate.md](phase-3a-gate.md), [phase-3b1-gate.md](phase-3b1-gate.md), [phase-3b2-gate.md](phase-3b2-gate.md), [phase-3c-gate.md](phase-3c-gate.md), [phase-3b3-gate.md](phase-3b3-gate.md) (Teil A und B) und [phase-3b4-gate.md](phase-3b4-gate.md) (Teil A und B) sowie [phase-3g-gate.md](phase-3g-gate.md) (Teil A und B), [phase-3h-gate.md](phase-3h-gate.md) (Teile 3h.2 bis 3h.4), [phase-3f-gate.md](phase-3f-gate.md) (3f.1 bis 3f.4 und Gesamtgate), [phase-4-1a-gate.md](phase-4-1a-gate.md) (Basis-ROM und Patch), [phase-4-1b-gate.md](phase-4-1b-gate.md) (Auf Original zurücksetzen), [phase-4-3a-gate.md](phase-4-3a-gate.md) (Textprojekt im Kernel), [phase-4-3b-gate.md](phase-4-3b-gate.md) (Textprojekt als Quelle), [phase-4-4-gate.md](phase-4-4-gate.md) (Git minimal und Gate der Phase 4), [phase-3d-1-gate.md](phase-3d-1-gate.md) (Flags und Variablen), [phase-3d-2-gate.md](phase-3d-2-gate.md) (Erzählen-Rahmen), [phase-3d-3-gate.md](phase-3d-3-gate.md) (Kampfsprites, DS-Profil, Kampfvorschau und Ei-Attacken; Teil A und B) und [phase-4-5-gate.md](phase-4-5-gate.md) (Veröffentlichen), dazu [phase-3d-5-gate.md](phase-3d-5-gate.md) (Figur, Dialog, Tür) und [phase-3d-9-gate.md](phase-3d-9-gate.md) (Spielmodus live) abarbeitet. Die Punkte selbst stehen in den Gate-Dateien; hier steht nur, **wer** sie prüft und **wie**. Entwurf: [docs/superpowers/specs/2026-10-22-agent-test-harness-design.md](superpowers/specs/2026-10-22-agent-test-harness-design.md) (§5 ist die Zuordnung).

Das Prinzip: Die echte Oberfläche läuft im Browser-Fenster der Claude-Desktop-App (statt im Tauri-Fenster), ihre Befehle gehen über eine Brücke an dieselbe Rust-Logik. Das Spiel läuft in einem Emulator ohne Fenster (`g4emu`).

Legende der Spalte „Wer“: **Agent** erledigt der Agent allein; **Agent + Blick** prüft der Agent, der Nutzer bestätigt kurz am Bild; **Hand** bleibt beim Nutzer.

## Einmalig einrichten

1. Emulator-Kern holen: `bash scripts/g4emu/setup.sh` (legt die Python-Umgebung an und **lädt den festen melonDS-Kern aus dem GitHub-Release** nach `tools/g4emu/core/`; die SHA-256-Summe ist im Skript festgelegt). Ist der Kern noch nicht da, vorher den Nutzer um sein OK für den Download bitten; ist `tools/g4emu/core/melondsds_libretro.dylib` schon da, entfällt der Schritt.
2. Spielstände prüfen: `ls ../Roms/_testkopien/_saves` zeigt `CPUD-zimmer.sav` (Platin DE), `IPKD-zimmer.sav` (HeartGold DE) und `IPKE-zimmer.sav` (HeartGold US), je mit einer `.md`-Datei. Sie liegen nicht im Repo und werden nie überschrieben.
3. apicula (Kartenbilder, 3D-Texturen): `tools/apicula/bin/apicula` muss existieren. In einem Git-Worktree fehlt `tools/` (steht in `.gitignore`): entweder `ln -s <Haupt-Checkout>/tools/apicula tools/apicula` oder `export G4_APICULA=<Pfad>`.
4. Prüfen, dass es läuft: `cd scripts/g4emu && uv run g4emu --help` und `uv run pytest -m "not integration" -q`.

## Je Lauf

1. **Frische Kopie** anlegen (nie ein Original öffnen; die Brücke lehnt alles außerhalb von `_testkopien` ab):

   ```sh
   cp -R "../Roms/Pokemon - Platin-Edition (Germany)_DSPRE_contents" ../Roms/_testkopien/pt-3a
   ```

   Für HeartGold DE und US entsprechend `"…Goldene Edition HeartGold (Germany)_DSPRE_contents"` und `"…HeartGold Version (USA)_DSPRE_contents"`. Das Ziel darf noch nicht existieren (sonst landet die Kopie als Unterordner darin). Jede Prüfreihe bekommt eine eigene Kopie (Anlegen, Patch und Bauen lassen sich nicht zurücknehmen). Unter `../Roms` wird nichts gelöscht.
2. **Screenshot-Ordner:** `mkdir -p target/agent/gate-$(date +%F)`; `g4emu` bekommt ihn als `--out` (mit Unterordner je Punkt). Die Werkzeuge des Browser-Fensters liefern Bilder nur an den Agenten, sie schreiben keine Datei: für „Agent + Blick“ beschreibt der Agent Ort und Zahlen (welcher Header, welche Zelle, was zu sehen ist) und sagt dem Nutzer, was er im Fenster ansehen soll. Bilddateien gibt es nur von `g4emu` (`shot`).
3. **App starten.** Zwei Wege:
   - Ist die Sitzung im Repo-Ordner geöffnet, der `.claude/launch.json` enthält: `preview_start` mit `name: "studio-web"`.
   - Sonst (zum Beispiel in einem Worktree: `preview_start` liest `launch.json` aus dem Ordner, in dem die Sitzung geöffnet wurde): per Bash im Hintergrund
     `bash scripts/agent/studio-web.sh > target/agent/studio-web.log 2>&1 & echo $! > target/agent/studio-web.pid`
     abwarten, bis das Log „Brücke läuft“ meldet (der erste Lauf baut `studio-bridge`, das kann Minuten dauern), dann `preview_start` mit `url: "http://127.0.0.1:1430"`.

   **Aus einem Git-Worktree** muss `G4_ROMS_DIR` absolut gesetzt sein, für die Brücke wie für das Gate-Skript: Der Standard `../Roms` meint den Ordner neben dem Haupt-Checkout und existiert vom Worktree aus nicht.

   ```sh
   G4_ROMS_DIR=/Pfad/zu/Roms bash scripts/agent/studio-web.sh > target/agent/studio-web.log 2>&1 & echo $! > target/agent/studio-web.pid
   G4_ROMS_DIR=/Pfad/zu/Roms scripts/gate.sh > target/agent/gate.log 2>&1
   ```

   Ports: Vite 1430, Brücke 1431. Die App des Nutzers (1420/1421) bleibt unberührt. Das Skript bricht ab, wenn 1430 oder 1431 belegt ist. Eine zweite Instanz (ein paralleler Worktree) bekommt eigene Ports: `G4_VITE_PORT=1440 G4_BRIDGE_PORT=1441 bash scripts/agent/studio-web.sh …`; `call.sh` liest den Brückenport aus `target/agent/bridge.port` des eigenen Checkouts, die Seite liegt dann unter `http://127.0.0.1:1440`.
4. **Projekt öffnen:** `http://127.0.0.1:1430/?project=<absoluter Pfad der Kopie>` (Leerzeichen als `%20`). Die Statusleiste zeigt „Browser-Modus (Brücke)“ und die Projektmeldung. Mit `?project=` startet die App in der Welt, ohne auf Start. Alternativ „Projekt öffnen“ in der Titelleiste oder auf Start (⌘O): ein Dialog mit dem Feld „Pfad“, „Abbrechen“ und „OK“ (Enter im Feld nimmt den Pfad, Escape bricht ab).
5. **App beenden:** Weg 1: `preview_stop`. Weg 2: `kill $(cat target/agent/studio-web.pid)` (TERM an das Skript beendet Brücke und Vite; kein `pkill -f`, das träfe auch die Shell). Danach `lsof -iTCP:1430 -iTCP:1431 -sTCP:LISTEN` (leer = Ports frei). Nach einem Neustart der Brücke ist kein Projekt mehr offen, die Seite muss neu geladen werden.
6. **Ergebnis:** Die Gate-Datei bekommt im Abschnitt „Ergebnis“ je Teil eine Zeile (Datum, Projekt, Ergebnis, Auffälligkeiten) und in der Spalte „Wer“ Agent oder Hand. Am Ende listet der Agent dem Nutzer alle Punkte mit „Hand“ und „Agent + Blick“, bei Bildern von `g4emu` mit Pfad, sonst mit Ort und Zahlen.

## Doppeldeck: wo was ist

Seit Phase 3c hat die App das Doppeldeck-Raster. Die älteren Abschnitte dieser Anleitung und die Gate-Dateien 3a bis 3b.2 nennen teils die Namen von vorher („Panel Welt“, „+ Neu“, „Begriffe“, Panel „Text“, Tab „Script“, „Toolbar“); die Tabellen unten sagen, wo es heute steht. Erst der Rahmen, von oben nach unten und von links nach rechts:

| Fläche | Was dort ist | Zum Bedienen |
|---|---|---|
| Titelleiste (40 px) | „Projekt öffnen“, Projektname mit Spiel-Plakette, die Zeile „Ungespeichert“ (liest sich „N Änderungen nicht gespeichert“, ohne Änderungen „Alles gespeichert“), die Palette (Schaltfläche „Suchen oder Befehl eingeben“ oder ⌘K, siehe unten), „Speichern“ (⌘S), „Bauen“, „Veröffentlichen“ (seit 4.5; gesperrt mit Hinweis im Tooltip, wenn die Basis-ROM fehlt), „Thema wechseln“, „Hilfe“ | Rolle `button` mit dem Namen; „Bauen“ fragt im Browser-Modus nach dem Pfad (siehe Werkzeuge unten). Ersetzt die alte „Toolbar“ |
| Atlas (88 px, Kompakt 56 px nur Symbole, der `aria-label` bleibt) | `nav` „Bereiche“: „Start“, „Welt“, „Geschichten“ (gesperrt, Tooltip „Kommt mit Phase 3d“, bis 3b.4a „Kommt mit Phase 3b.4“), „Pokémon“, „Gegenstände“, „Texte“, „Technik“; unten „Hilfe“ und „Einstellungen“. Der aktive Bereich trägt `aria-current="page"` | Tab von Eintrag zu Eintrag, Enter öffnet. „Hilfe“ öffnet den Dialog mit dem Glossar, das früher „Begriffe“ hieß (`workbench/terms.ts`) |
| Liste (272 px) | je Bereich: „Welt“ mit den Abschnitten „Orte“ und „In dieser Szene“ (Welt), „Textarchive“ (Texte), die Liste des Unterbereichs (Pokémon: Pokédex oder Attacken; Gegenstände), „Technik“; Start und Einstellungen haben keine | Griff am rechten Rand (Doppelklick) oder Palette „Liste ein/aus“ klappt sie ein |
| Scharnier (44 px, über der Schau) | Brotkrume (Welt: Ort › Header › Auswahl, eine gewählte Karte in Bauen „Karte 0412“ mit „Zelle 3, 27“ grau, in Tüfteln „Zelle 3, 27“; Texte: Texte › Archiv › Nachricht; Pokémon: Pokémon › Pokédex › STARAVIA, das zweite Glied öffnet das Menü der Unterbereiche; Gegenstände: Gegenstände › Tasche Medizin › Trank, ein Klick auf die Tasche filtert die Liste; jedes Glied klickbar), Umschalter „Tiefe“ mit „Erzählen“ / „Bauen“ / „Tüfteln“ (`[role=group][aria-label=Tiefe]`, je Taste `aria-pressed`; seit 3d.2 drei Tiefen: Erzählen zeigt in der Welt statt der Werkzeugleiste die Rezept-Leiste `[role=toolbar][aria-label=Rezepte]` und im Inspector nur die Felder für Erzählen, in Technik, Pokémon, Gegenstände und Texte dasselbe wie Bauen, das Segment bleibt dabei auf „Erzählen“; Tüfteln zeigt im Inspector Rohwerte, Hex-Felder und Bezeichner, in Probleme und Script Dateipfade, den Reiter „Raster“ mit Header-ID und Höhe je Zelle, in Pokémon und Gegenstände die Rohdaten als Hexblock und Dez/Hex an den Zahlfeldern), Prüflicht (Schaltfläche `button[data-light]`; seit 3b.4a drei Zähler `[data-count=error|warning|info]`, `aria-label` „3 Fehler, 102 Warnungen, 10 Hinweise“ bzw. „Alles stimmig“, während eines Prüflaufs `data-running`, ein Kreisel und der Fortschritt „Prüft … Texte 312/724“; Klick öffnet „Probleme“), Verlauf-Menü („Verlauf, N Schritte“), „Wiederholen“ (⇧⌘Z), „Hier spielen“ (speichert, baut und startet den Emulator; im Browser-Modus baut es nur; ⌘R nur in der App). Im Assistenten die Schrittleiste (siehe unten) | Das Scharnier steht über der Schau, hat keinen Griff und lässt sich nicht ziehen (die Höhe der Hand ändert der Teiler darunter). Öffnet sich ein Script, hebt die App die Hand auf mindestens 400 px |
| Dock (seit 3e.4, eines je Bereich) | `div[data-dock="<bereich>"]#dock-<bereich>` unter dem Scharnier: Gruppen `[data-dock-group]` mit immer sichtbaren Reitern, Panels `[data-dock-panel="<id>"]` (der Inhalt liegt über seiner Gruppe, nicht in ihr: die Gruppe eines Panels findet `[data-dock-tab="<id>"]` des Reiters, `.closest("[data-dock-group]")`). Die Schau-Gruppe (`data-dock-group-kind="schau"`) hält die Ansicht des Bereichs (Welt: 3D mit der schwebenden Werkzeugleiste und Weltkarte, in Tüfteln dazu Raster; Texte: Vorschau; Pokémon und Gegenstände: „Übersicht“); die anderen Gruppen halten, was man anfasst (Welt: Inspector mit Kopf, Script, Probleme; Texte: Nachrichten, Probleme; Pokémon und Gegenstände: „Bearbeiten“). Vorlagen: „Werkbank“ (Welt: Inspector rechts 340 px in voller Höhe, Script und Probleme unter der Schau 240 px) und „Doppeldeck“ (Bearbeiten-Gruppe unter der Schau, 286 px; Standard in Texte, Pokémon, Gegenstände) | Palette „Anordnung: Werkbank“, „Anordnung: Doppeldeck“, „Layout zurücksetzen“, „Schau maximieren“, „Inspector schweben lassen“, „Inspector andocken“; Doppelklick auf einen Reiter maximiert seine Gruppe (Esc oder erneuter Doppelklick stellt her); Rechtsklick oder Umschalt+F10 auf einem Reiter öffnet das Menü „Maximieren“/„Wiederherstellen“, „Schweben lassen“, „Zurück ins Dock“, „Anordnung zurücksetzen“ (Schau-Reiter nur „Maximieren“; mit Umschalt+F10 Pfeiltasten und Enter); Umschalt-Klick auf einen Reiter lässt ihn schweben; Schau-Panels bleiben in der Mitte. Die Höhen und Breiten ändert man an Dockviews Trennlinien |
| Statusleiste (28 px) | links die Hinweise des Werkzeugs („Klick wählt“ …; in den Texten „↵ neue Zeile, ⇧↵ neue Seite“, solange das Feld den Fokus hat), Mitte Kachel- und Ortskontext (in der Welt auch das, was unter dem Zeiger liegt, eine Kachel, ein Marker, eine Tür; der Inspector behält dabei die Auswahl; in den Texten „Archiv 0023, 104 Nachrichten, 3 zu lang“; in Pokémon und Gegenstände Name und Nummer der Auswahl, „STARAVIA, 0397“), rechts Build-Stand („Noch nicht gebaut“, „Baut …“, „Gebaut vor …“), Dichte (Komfort / Kompakt), im Browser-Modus „Browser-Modus (Brücke)“ | die Dichte schaltet um und ist im `localStorage` gemerkt |
| Meldungen | Toasts unten rechts; ein Fehler bleibt, bis man ihn schließt, ein Fortschritt ersetzt sich | im DOM Rolle `alert` (Fehler) bzw. `status` |

Die Bereiche:

| Bereich | Was dort ist | Zum Bedienen |
|---|---|---|
| Start | „Willkommen zurück“, die zuletzt geöffneten Projekte als Karten (Liste „Zuletzt geöffnete Projekte“; Ordnername, Spiel, „geöffnet vor …“, Pfad als Tooltip), „Projekt öffnen“ (⌘O), „Projekte sind entpackte DSPRE-Ordner.“ | Klick auf eine Karte öffnet das Projekt und wechselt in die Welt. Eine Karte, deren Ordner fehlt oder die sich nicht öffnen ließ, sagt „nicht gefunden“ und hat das Menü „…“ (Name „Weitere Aktionen für <Name>“) und die Taste „Aus der Liste nehmen“. Ohne `?project=` startet die App hier |
| Welt, Liste „Welt“ | Seit 3e.3 trägt die Liste den Kopf „Welt“ (Untertitel das Spiel, „Platin, DE“; der Griff „Liste einklappen“) und zwei Abschnitte untereinander, dazwischen ein Teiler: oben `section[aria-label=Orte]`, unten `section[aria-label="In dieser Szene"]`. **„Orte“:** Suche „Welt durchsuchen“ (Platzhalter „Gebiet, Raum oder Kürzel suchen“), Umschalter „Baum“ / „Liste“ (die Liste: eine Zeile je Header mit Ortsname und grau „0412 · T01R0101“, Suche „Orte suchen“, in Tüfteln „Header suchen“ und dazu `M0`), das Menü „Sortierung“ (Baum: Spielreihenfolge, Name A–Z, Zuletzt gewählt; Liste: Nummer, Name, Kürzel), das Menü „Neu anlegen“ (+) mit „Gebiet anlegen…“ und „Raum anlegen…“. Seit 3e.2 ein Baum `[role=tree][aria-label=Orte]`: Weltkarten (`data-world-map`, Platin „Sinnoh“, HGSS „Johto“ und „Kanto“), darunter Gebiete (`data-area-key`, `data-place-key` = Ortsname, `data-header-id` = erster Header), darunter Räume (`data-room`, `data-header-id`, Klarname, Ortsetikett bei fremdem Ort; seit 3f.4 an einem Raum, dessen Karte auch andere Räume nutzen, ein Symbol mit `aria-label` „geteilt“ und dem Titel „Geteilt mit 16 Räumen“); danach die Gruppen `[data-world-group=more|unconnected|other]` („Weitere Gebiete“, „Nicht verbunden“, „Sonstiges“ nur in Tüfteln). Die Zeilen des Baums haben seit 3e.3 keine Script-Kinder mehr (die Scripts der Szene stehen in „In dieser Szene“); der Wurzelknoten „Scripts“ (`[data-scripts-root]`) bleibt mit Volltextsuche („Scripts durchsuchen“). Am gewählten Raum „Weitere Aktionen für …“ (Umbenennen…, Gehört zu Gebiet…, Zuordnung zurücksetzen), Namen in `g4-world.json` der Kopie. Im Kopf von „Orte“ der Knopf „Orte einklappen“ („Orte ausklappen“): eingeklappt steht eine Zeile `button[data-places-row]` mit dem gezeigten Ort („Zweiblattdorf“, für einen Raum „Zweiblattdorf › Haus 1“, ohne gezeigten Ort „Ort oder Raum wählen“), ein Klick öffnet den Picker „Ort oder Raum wählen“ (Suche „Gebiet, Raum oder Nummer suchen“, die Gruppen des Baums; eine Wahl zeigt den Ort und zählt für die Sortierung „Zuletzt gewählt“ wie ein Klick im Baum); der Baum bleibt mit Suche und offenen Ästen im DOM, `hidden`. **Teiler:** `[role=separator][data-splitter=world-list]`, Name „Höhe der Orte“, `aria-valuenow` die Höhe der Orte in px (Spanne: 20 bis 80 Prozent der Listenhöhe, jeder Teil mindestens 96 px; bei 400 px Listenhöhe 200 von 96 bis 296), nur bei ausgeklappten Orten. **„In dieser Szene“:** `[role=tree][aria-label="In dieser Szene"]` mit der Wurzel `[data-scene-group=header]` (das gezeigte Gebiet oder der Raum, `data-header-id`) und den Gruppen `[data-scene-group=maps|object|warp|coord|bg|scripts]` („Karten“, „Figuren“, „Türen“, „Auslöser“, „Schilder“, „Scripts“; Name mit Zähler, „Figuren, 2“; eine leere Gruppe fehlt), darunter die Zeilen `[data-event="<art>:<index>"]` (art `object`, `warp`, `coord`, `bg`), die Karten „Karte 0006 · Zelle 0, 0“ (ohne eigenes Datenattribut; ein Ast, dessen Gebäude `[data-building="<karte>:<index>"]` zu sind, bis man ihn öffnet) und die Scripts `[data-script-file]` (unter dem Baum „In dieser Szene“ suchen: auch der Wurzelknoten „Scripts“ der Orte trägt das Attribut; die Gruppe hier ist `[data-scene-group=scripts]`). Eine Tür hat die zweite Zeile „→ Haus 1“ (unbekanntes Ziel „→ unbekanntes Ziel 0999“; in Tüfteln „→ Header 0414 · Haus 1“); Enter oder Doppelklick auf eine Tür mit bekanntem Ziel zeigt die Szene des Ziels, ein Klick wählt wie in der Szene, Klick auf ein Script öffnet seinen Reiter. Kopf: „Zurück“ und „Vor“ (Tasten mit `aria-label`, ⌥⌘← und ⌥⌘→; gesperrt, wo es keinen Schritt gibt), das Menü „Hinzufügen“ („Figur“, „Tür“, „Auslöser“, „Schild“, „Gebäude“ wählen das Werkzeug, dazu „Raum hier anbinden…“ und seit 3f.2 „Neuer Ort…“: der Dialog „Neuer Ort“ für das gezeigte Gebiet oder den Raum, Inhalt `[data-new-place]` mit dem Feld „Name des Orts“, der Gruppe „Mitnehmen“ (Kästchen für das Gebiet und seine Räume) und „Ort anlegen“; bei Diamant/Perl gesperrt mit Grund), das Suchfeld „In dieser Szene suchen“ (Platzhalter „Name oder Nummer suchen“; Esc leert es). Bei gesperrter Eventdatei die Meldung „Eventdatei gesperrt“ und an den Gruppen Figuren, Türen, Auslöser, Schilder die Plakette „Gesperrt“; Befunde der Prüfungen als Zeichen am Ende der Zeile | ersetzt das alte Panel „Welt“ und „+ Neu“ (bis 3e.2 hieß die ganze Liste „Orte“); „Gebiet erweitern“ öffnet seit 3f.1 über die „+“-Zellen der Weltkarte und des Raumplans (`[data-extend]`, Zeile „Welt, Reiter „Weltkarte““), in Tüfteln auch über die des Reiters „Raster“. Der Teiler folgt dem Zeiger beim Ziehen (nach 4 px), mit dem Fokus ±16 px, Pos1 (größte Höhe der Orte), Ende (kleinste), ein Doppelklick halbiert; Höhe und Einklappen merkt `g4.shell.v1` (`worldListSplit`, `placesFolded`) |
| Welt, Schau | Reiter in Bauen „3D“ und „Weltkarte“ (Panels `world`, `worldmap`), in Tüfteln dazu „Raster“ (`matrix`; seit 3f.1 kommt er beim Wechsel nach Tüfteln dazu, nie vorn, und geht beim Wechsel nach Bauen; ist er vorn, wird vorher „3D“ vorn). Über der 3D-Ansicht schwebt oben links die Szenenleiste mit den Schaltern „Gelände“ und „Ereignisse“ und in einer zweiten Gruppe „Draufsicht“ (`aria-pressed`, gemerkt als `topDown` in `g4.shell.v1`: Kamera senkrecht von oben, Norden oben, kein Drehen, die linke Maustaste verschiebt; aus: zurück zur Schrägsicht), darunter (Anker „Links“, Standard), daneben („Oben“) oder unten in der Mitte („Unten“) die Werkzeugleiste (`[role=toolbar][aria-label=Werkzeugleiste]`, `aria-orientation` `vertical` bei „Links“, sonst `horizontal`, `data-anchor`), unten links die „Legende“. In einer Ansicht unter 460 px Höhe (Kompakt 400 px) liegt eine linke Leiste flach oben; der gemerkte Anker bleibt. **Entwurf** (seit 3d.2): Die Assistenten „Gebiet anlegen“, „Raum anlegen“, „Welt leeren“ und der Dialog „Karte bei … anlegen?“ fragen 300 ms nach der letzten Änderung `recipe_plan` mit ihrer gebauten Eingabe (eine veraltete Antwort wird verworfen); der Host der 3D-Ansicht trägt `[data-draft][data-draft-count]` (gezeichnete Zellen, Kacheln und Marker des Entwurfs auf der gezeigten Weltkarte, gleich `draftCount(<Antwort von recipe_plan>, <Matrix>)`), Weltkarte und Raumplan zeichnen die Zellen gestrichelt als `[data-draft-cell="x,y"]`, der Schritt „Zusammenfassung“ zeigt die Sätze des Entwurfs (`[data-draft-notes]`, das Unumkehrbare hinter der Plakette „Nicht rückgängig“), ein gescheiterter Plan „Vorschau nicht möglich: …“ (Anlegen bleibt möglich); Esc, „Abbrechen“, „Schließen“ und das Anlegen setzen `data-draft-count` auf 0. Abfrage immer mit `[data-draft-count]`: Text- und Zahlfelder tragen `data-draft` für ihren eigenen Entwurf. **Rezepte von Erzählen** (seit 3d.5): die Rezept-Leiste hat alle fünf Tasten frei (senkrecht in Ruhe Symbol und Name; waagerecht, auch bei einer niedrigen Schau mit offenem Assistenten, nur Symbole mit dem Namen als `aria-label` und im Tooltip, `data-icon-only`); „Neue Figur“, „Tür verbinden“ und „Neuer Trainer hier“ öffnen ihren Assistenten im Scharnier (`[data-wizard="new_figure"]`, `[data-wizard="connect_doors"]`, `[data-wizard="new_trainer_here"]`) mit dem gewählten Ort und der zuletzt in der Szene berührten Kachel (nur, wenn die Szene die Weltkarte dieses Orts zeigt); „Ort“ als Chip mit „Ort wählen…“ (`[data-figure-place="<header>"]`), „Kachel x“/„Kachel z“. Figur: Schritte „Wo und wie“ (Aussehen `[data-figure-look]`, „Bewegung“, „Blickrichtung“ Oben/Unten/Links/Rechts), „Dialog“ (`textarea[aria-label="Was sie sagt"]`, Umschalt+Enter neue Seite, DS-Vorschau `[data-figure-preview]` mit „Seite n von m“, „Sichtbarkeits-Flag“ `[data-flag-field=hidden_flag]`), „Zusammenfassung“, „Aufstellen“; Ergebnis „Figur aufgestellt“ mit „Objekt k in Eventdatei NNNN“ und unter „Als Nächstes“ `[data-recipe=edit_dialog]` „Dialog bearbeiten“ (frei: zeigt den Ort und öffnet `[data-dialog-editor]` der neuen Figur) und `[data-recipe=story_join]` gesperrt („Kommt mit 3d.7“). Tür: „Tür A“, „Tür B“ (je `[data-door-end=a|b]` mit „Vorhandene Tür“ | „Neue Kachel“, „Tür wählen…“ mit Einträgen „Tür k · x X, z Z“), Kästchen „Rückweg anlegen (Tür B führt zu Tür A)“ und „Türkacheln setzen (eine neue Tür wird begehbar)“ (die Namen sind länger als „Rückweg anlegen“ und „Türkacheln setzen“: nach dem Präfix suchen; ohne Rückweg muss Tür B vorhanden sein), „Verbinden“. Trainer: „Trainer“ („Klasse“, „Name“ höchstens 7 Zeichen), „Team“ (Zeilen `[data-team-row]` mit Karte, „Art wählen…“, „Level“, „Pokémon k entfernen“; „Pokémon hinzufügen“ bis 6), „Texte“ („Vor dem Kampf“ endet auf ▼, „Verloren“, „Nach dem Kampf“), „Wo“ (dazu „Sichtweite“ 1–15), „Zusammenfassung“ mit `[data-trainer-barrier]` „Trainer anlegen lässt sich nicht rückgängig machen“, „Aufstellen“. Jeder Entwurf zählt wie oben (`[data-draft-count]`); eine Figur im Entwurf trägt ihr Sprite halb durchsichtig (Platzhalter, bis es geladen ist). Inspector einer Tür in Erzählen: Zeile „Gegenstück“ (`[data-warp-counterpart]`: „Tür k in <Ort> · führt zurück“, „… führt nicht zurück“ oder „Ziel hat keine Tür k“) und `button[data-connect-doors]` „Tür verbinden…“ (öffnet den Assistenten mit dieser Tür als Tür A) | die Szene nimmt Klicks nach dem aktiven Werkzeug (unten); die Werkzeugleiste ist nur mit dem 3D-Reiter im Bild. **Karte wählen in 3D** (seit 3f.1): mit „Auswählen“ wählt ein Klick auf den Boden einer geladenen Karte, ohne Marker und ohne Gebäude, die Karte: Inspector `header[data-inspector-kind=map]` „Karte 0174“ mit Kennung „Zelle 2, 9“, Brotkrume „Vollmond-Insel › Gebiet 0260 › Karte 0174 Zelle 2, 9“, Statusleiste „Vollmond-Insel, Karte 0174 (2, 9)“, um die Zelle ein flacher Ring in der Auswahlfarbe (nur im Bild prüfbar); ein Klick auf leeren Boden ändert nichts. Die Szene lädt ein Gebiet einer Weltkarte mit dem Ring seiner 8 Nachbarzellen und rahmt die Kamera auf das Gebiet (Raum und Gebiet mit eigener Karte: die ganze Karte); die Ladezeit misst das Rezept „Ladezeit messen“ |
| Welt, Reiter „Weltkarte“ (seit 3f.1) | Die Wurzel `[data-world-view]` nennt die Ansicht: `w0` (Platin „Sinnoh“, HeartGold „Johto“), `w0k` („Kanto“), `w0all` („Johto und Kanto“), `more` („Weitere Gebiete“), `plan` („Raumplan“, nur solange ein Raum oder ein Gebiet mit eigener Karte gewählt ist). Der Kopf ist der Umschalter: eine Menütaste mit dem Namen der Ansicht (Menü mit Häkchen an der gezeigten), im Raumplan dazu der Name als Untertitel („Ewigwald, Ebene 1“). Die Ansicht folgt der Auswahl; eine Wahl im Menü gilt bis zur nächsten Auswahl. Das Gitter `[role=group]` heißt „Weltkarte, 30 Zeilen, 30 Spalten“ bzw. „Raumplan, 3 Zeilen, 3 Spalten“ (Tüfteln „Raster 0, …“), seine Zellen `button[data-cell="x,y"]` (`aria-pressed` an der gewählten, `data-foreign` gedämpft an Zellen der anderen Hälfte); eine Weltkarte rückt das Gewählte in die Mitte (`[data-world-viewport]`), wenn es nicht schon zu sehen ist. „+“-Zellen `button[data-extend="x,y"]` mit `aria-label` „Karte bei x 1, y 9 anlegen (nicht rückgängig)“: auf einer Weltkarte leere Zellen neben dem gewählten Gebiet, im Raumplan leere Zellen neben belegten und die Spalte rechts und die Zeile unten. „Weitere Gebiete“: je Gebiet eine Kachel `button[data-more-area="<key>"]` mit Name, „20 Karten“ und dem Plan in 32 px | Klick auf eine Zelle eines Gebiets wählt das Gebiet (Baumzeile `aria-selected`); Klick auf Landschaft, ⌥Klick oder Doppelklick wählt die Karte (Inspector „Karte“); im Raumplan wählt jeder Klick die Karte. „+“ öffnet den Dialog „Karte bei x 1, y 9 anlegen?“ mit Plakette „Nicht rückgängig“, „Kopie von Karte 0052 (Nachbarzelle)“ oder „Karte wählen“ und „Karte anlegen“. Eine Kachel wählt ihr Gebiet und springt in den Raumplan. Den Reiter nach vorn holt das Rezept „Dockview-Reiter im Browser umschalten“ (`[data-dock-tab="worldmap"]`) |
| Welt, Hand | die Reiter „Inspector“, „Script“, „Probleme“ (seit 3e.1 ohne Werkzeuge; seit 3e.3 ohne „An diesem Ort“, dessen Zeilen stehen in „In dieser Szene“ der Liste). Der Inspector beginnt mit seinem Kopf `header[data-inspector-kind=figure|door|trigger|sign|building|map|area|room|place|look]`: die Augenbraue `[data-eyebrow]` (das Typ-Wort „Figur“, „Tür“, „Auslöser“, „Schild“, „Gebäude“, „Karte“, „Gebiet“, „Raum“, „Ort“, „Aussehen“; in Tüfteln „Header“ und „Area-Data“), die Überschrift `h2` (Bauen: der Name des Gebiets oder Raums, „Objekt 2 · ID 3“, eine Tür wie ihre Zeile in „In dieser Szene“ „Warp 0 → Haus 1“, eine Trainer-Figur „Objekt 0 · Wanderer Jens“; Tüfteln: „Warp 0 → Header 0412 (T01R0101)“), die graue Kennung `[data-ident]` („Objekt 2 · Eventdatei 0390“; Tüfteln: „Header 0411 · T01“) und rechts die Plaketten „Ungespeichert“, „Gesperrt“, „Nicht rückgängig“ sowie das Menü „Aktionen“ (Karte: „Teile importieren…“, „Teile exportieren…“, „Duplizieren“; Figur, Tür, Auslöser, Schild, Gebäude: „Löschen“ mit Entf; Gebiet und Raum: „Raum hier anbinden…“ und seit 3f.2 „Ort umbenennen…“ (Dialog „Ort umbenennen“ mit „Trifft alle N Gebiete und Räume mit diesem Ort …“ und „Umbenennen“; gesperrt bei „Kein Ort“), ein geteilter Raum seit 3f.4 dazu „Für diesen Raum abspalten…“ (Dialog „Raum abspalten?“, Inhalt `[data-split-room]` mit Plakette „Nicht rückgängig“, Aktionen „Abbrechen“ und „Abspalten“; danach Toast „Raum „…“ abgespalten“); Aussehen und alles ohne Aktion: kein Menü). An einem geteilten Raum steht im Kopf `[data-shared]` mit der Plakette „Geteilt mit 16 Räumen“ (Tooltip „Diese Karten gehören auch anderen Räumen. Was du hier änderst, ändert sie alle.“) | der Inspector zeigt nur die Auswahl; was unter dem Zeiger liegt (Kachel, Marker, Tür), steht in der Mitte der Statusleiste. Im Header-Editor steht statt „Eingänge / Ausgänge“ die Taste „2 Türen, 1 ohne Ziel“ (`p[data-doors]`; ohne Türen „Keine Türen“): sie zeigt die Szene, öffnet die Gruppe „Türen“ und setzt den Fokus darauf. Seit 3e.4 liegen die Reiter im Dock der Welt (`#dock-welt`, Zeile „Dock“ oben); die Höhe einer Gruppe misst man über ihren Reiter (`[data-dock-tab="script"]`). Die Referenz „Textarchiv“ im Header-Editor öffnet den Bereich Texte. **Picker „Ort“** (seit 3f.2): im Header-Editor von Gebiet und Raum (Abschnitt „Karten“) das Feld „Ort“ (Tüfteln „Ortsname“ mit „Nachricht 17 · Archiv 0433“), eine Taste `button[data-place-picker]` mit Punkt in der Ortsfarbe und Namen („Kein Ort“ bei Index 0); sie öffnet den Picker „Ort wählen“ (Suche „Ort oder Nummer suchen“, zuerst „Kein Ort (Mysteriöser Ort)“, jeder Name mit der Zahl der Gebiete und Räume, im Fuß „Neuer Ort…“); eine Wahl ist ein Undo-Schritt. Der Inspector einer Karte heißt in Bauen „Karte 0412“ mit Kennung „Zelle 3, 27 · Zweiblattdorf“ (Tüfteln „Zelle 3, 27“ mit „Raster 0000 · …“) und hat „Karte“ und „Höhe“; eine Höhe an einer Karte ohne Höhenabschnitt legt ihn im selben Undo-Schritt an |
| Welt, Reiter „Script“ | Kopf mit Dokumenttitel („Scriptdatei 0042“, „Level-Script 0042 (JSON)“; in Tüfteln der Dateipfad als Untertitel), „Gliederung“, „Speichern“, „Script anfügen“, die LSP-Plakette (`data-lsp`, siehe unten) und bei gestopptem Server „Neu starten“; offene Dokumente als Reiter (Liste „Offene Scripts“, `data-script`, Kobalt-Punkt „ungespeichert“, „<Name> schließen“, Entf oder ⌘W auf dem Reiter schließt). Ohne Übernahme der Kasten „Scripts übernehmen“ mit „Übernehmen“; ohne Dokument „Kein Script geöffnet.“ | der Dock-Reiter „Script“ trägt den Punkt, solange ein Dokument ungespeichert ist |
| Welt und Texte, Reiter „Probleme“ | Seit 3b.4a oben die Befunde der Prüfungen (`section[aria-label=Probleme]`): Bereiche Welt, Scripts, Texte, Pokémon, Projekt als faltbare Abschnitte mit Zähler, darin je Code eine Gruppe `[role=group][data-check=<code>]` mit Titel und Zahl, Zeilen `li[data-finding][data-state]` mit Satz, Ort („Eventdatei 0394 · Warp 0“) und den Tasten „Öffnen“, „Beheben…“, „Ausblenden“; darüber „Nicht geprüft“, unten der Fuß mit Fortschritt, „N ausgeblendet · Wieder melden…“ und „Erneut prüfen“; vor dem ersten Bericht „Prüft das Projekt …“. Darunter die Tabelle der übrigen Probleme mit Schwere (Symbol, Name „Fehler“ oder „Warnung“), Datei als Referenz („Scriptdatei 0042“, „Textarchiv 0199“; in Tüfteln der Pfad), Zeile:Spalte, Meldung, Quelle („Compile“, „Text“, „live“); leer „Keine Probleme.“ | Klick auf die Datei springt zur Zeile (Script) bzw. zur Nachricht (Texte); „Öffnen“ an einem Befund zeigt sein Ziel (ein Event mit dem ersten Header, der die Eventdatei liest) |
| Texte, Liste „Textarchive“ | Untertitel „N Archive“, „+“ = „Textarchiv anlegen“ (sofort auf der Platte, nicht rückgängig), Filter „Textarchive filtern“ (Platzhalter „Ort oder Archivnummer“), die Gruppen „Orte“ (Archive, die ein Header nutzt, unter dem Ortsnamen, Kennung grau) und „Spielweit“ (`data-text-group`), je Zeile `data-archive` und eine Warn-Plakette mit der Zahl zu langer Texte, unten die Summe „N Texte zu lang, in M geprüften Archiven“ | ersetzt die Archivliste des alten Panels „Text“ und „Neues Archiv“. Die Nachrichtenzahl steht nur beim geladenen Archiv |
| Texte, Schau „Vorschau“ | Kopf „Nachricht 005“ mit „Archiv 0023, <Ort>“ und der Plakette „N Seiten“; der DS-Bildschirm (Rolle `group`, `aria-label` „DS-Bildschirm, Seite x von y“) über dem Kartenbild des Orts; „Seite 1 von 2, 256 × 192, 2-fach“, „Zurück“, „Weiterblättern“; rechts die Liste „Seiten“ mit einer Karte je Seite (`data-page`, „34 von 38“ je Zeile) | Pfeiltasten links und rechts blättern, solange die Vorschau den Fokus hat; Klick auf eine Karte springt zur Seite |
| Texte, Hand „Nachrichten“ | Suche „Texte suchen“ (Platzhalter „In diesem Archiv suchen“ bzw. „In allen Archiven suchen“), Umschalter „Suchbereich“ mit „Dieses Archiv“ / „Alle“, „Nachricht anlegen“; die Tabelle „Nachrichten von Archiv 0023“ (Nr., Text mit Steuercodes als Plaketten, Längste Zeile als Balken, über 38 Zeichen mit ⚠; Zeilen `data-message`) oder bei Suche in allen Archiven „Treffer in allen Archiven“ (Archiv, Nr., Text; `data-hit` „Archiv:Nachricht“); rechts „Nachricht bearbeiten“ mit dem Feld „Text der Nachricht“, „Verwendet in“ (Header als Referenzen, `data-header`) und „Letzte Nachricht entfernen“ | Enter im Feld schreibt `\n`, ⇧Enter `\r` (neue Seite); das Feld übernimmt beim Verlassen. Enter auf einer Zeile öffnet die Nachricht, auf einem Treffer Archiv und Nachricht. Ersetzt „Nachricht anfügen“ und „Letzte entfernen“ |
| Pokémon, Liste | Der Listenkopf ist ein Menü-Knopf mit dem Namen des Unterbereichs und einem Pfeil („Pokédex ▾“); das Menü nennt „Pokédex“, „Trainer“, „Begegnungen“, „Attacken“ mit Zähler und Kürzel ⌘1 bis ⌘4 (seit 3b.3b alle offen). Darunter die Suche („Pokédex durchsuchen“, „Trainer durchsuchen“, „Begegnungen durchsuchen“ oder „Attacken durchsuchen“, Platzhalter „Name oder Nummer“, bei Trainern „Name, Klasse oder Nummer“, bei Begegnungen „Ort oder Nummer“; eine Zahl findet die Nummer und alle, die mit ihr beginnen), die Filter und die Liste (Pokédex und Attacken: Rolle `listbox`, benannt „Pokédex“ oder „Attacken“, Zeilen `data-species` oder `data-move`; Trainer: Rolle `tree` „Trainer“ mit Gruppen `data-group` und Zeilen `data-trainer`; Begegnungen: `listbox` „Begegnungen“ mit Zeilen `data-encounter` und der Gruppe „Ohne Ort“; ungespeicherte Zeilen mit Punkt, gesperrte Zeilen mit der Plakette „Gesperrt“ und dem Grund im Namen der Zeile). Ein aktiver Filter steht als Zeile („Typ Flug: 3“) mit „Filter lösen“. Ohne Auswahl zeigt die Schau „Kein Trainer gewählt.“ usw. mit „Ersten Trainer zeigen“ bzw. „Erste Datei zeigen“ | ⌘1 Pokédex, ⌘2 Trainer, ⌘3 Begegnungen, ⌘4 Attacken (nur im Bereich Pokémon, nicht im Assistenten). Die Liste ist gefenstert: nur die Zeilen im Blick stehen im DOM, `aria-setsize`/`aria-posinset` nennen die echte Lage; Pfeiltasten, Pos1, Ende und Bild-Tasten erreichen jede Zeile. Teil und Auswahl je Teil merkt `g4.shell.v1` |
| Pokémon › Pokédex | Liste: Kopf „493 Arten, 12 Formen“ (Platin), Filter „Typ“ (Plaketten mit Zähler), Zeilen mit Menüsymbol aus dem ROM, Name, Nummer; die Formen 496–507 mit Namen („Hitze-Rotom“), eine Art ohne Namen „Art 0520“. Schau mit `aria-label` „Übersicht STARAVIA“ (Name der Art): Kopfkarte (Symbol, Name, Nummer, Typen, Fähigkeiten), sechs Wertebalken gegen den Schnitt aller Arten, „Summe 340“ (nach einer Änderung „Summe 350, vorher 340“), „Fundorte“ (Orte als Referenz in die Welt) und „Verwendet in“ („N Stellen, ohne Scripts gezählt.“). Hand „Bearbeiten“: Reiter „Werte“, „Fähigkeiten und Typen“, „Lernset“ (Tabelle, „12 von 20“), „Entwicklung“ (Tabelle mit 7 Zeilen), „TM/VM“ (100 Kästchen „TM40 Aero-Ass“, „Von anderer Art übernehmen …“), in Tüfteln „Rohdaten“; ein Punkt am Reiter zeigt Ungespeichertes | jedes Feld übernimmt einzeln als ein Schritt im Verlauf („STARAVIA: Angriff 75 → 85“). Ein gesperrtes Teil (Member mit falscher Länge) nennt den Grund („Pokémon-Daten 0397: 36 Bytes statt 44“) und sperrt seine Felder |
| Pokémon › Attacken | Liste: Kopf „471 Attacken“, Filter „Typ“ und Auswahl „Kategorie“, Zeilen mit Typ-Plakette. Schau „Übersicht“ (`data-move`): Karte (Name als Referenz ins Namensarchiv, Nummer, Typ, Kategorie, Stärke, Genauigkeit, AP, Merkmale), „Beschreibung“ als DS-Textbox mit „Beschreibung bearbeiten“ (öffnet die Nachricht im Bereich Texte), „Verwendet in“ („23 Lernsets, 11 Trainer“). Hand: Spalten „Grundwerte“ und „Verhalten“, in Tüfteln „Rohdaten“ (Effekt-ID, alle acht Bits des Merkmale-Bytes, Hexblock) | Bits ohne Namen (0x40, 0x80) zeigt nur Tüfteln; sie bleiben beim Umschalten der benannten Merkmale erhalten |
| Pokémon › Trainer | Liste: Kopf „928 Trainer“ (Platin; HeartGold 738), Umschalter „Nach Ort“ / „Nach Klasse“, Baum Ort → Trainer („Wanderer Jens“, Nummer grau, rechts „Lv. 12“ = höchstes Level im Team), „Ohne Ort“ zuletzt; nur die Gruppe mit der Auswahl ist offen, beim Suchen alle; der Platzhalter des Sicht-Scripts (Platin 929, HeartGold 740) ist gesperrt („Platzhalter für das Sicht-Script“); „+“ = „Trainer anlegen…“. Schau: Kopf mit Klasse und Name, Nummer, „Doppelkampf“; Team als Pokémon-Karten (`data-slot`, Klick öffnet die Art); „Steht in der Welt“ mit den Figuren als Referenz („Route 202, Kachel 185, 804“) oder „Noch nicht platziert“ und „In der Welt platzieren“; Warnung „Ohne Pokémon stürzt der Kampf ab“. Hand „Bearbeiten“: Reiter „Team“ (sechs Karten, darunter die Details des gewählten Platzes: Art, Level, Stärke, in HeartGold Geschlecht und Fähigkeit, Form nur bei Arten mit Formen, Item bei „Items tragen“, vier Attacken bei „Eigene Attacken“, „Pokémon entfernen“), „Trainer“ (Klasse, Name „4 von 7“, Kampfart Einzel/Doppel, KI einfach · klug · Experte · eigene, Schalter „Eigene Attacken“ und „Items tragen“, vier Items, Kampftexte mit „Text anlegen“ und der Warnung „Der Text vor dem Kampf sollte auf ⏎ enden …“ samt „⏎ anhängen“), in Tüfteln „Rohdaten“ (Hexblock beider Dateien, 32 KI-Bits) | jede Änderung ist ein Schritt („Wanderer Jens: Platz 1 Level 12 → 14“, „…: KI einfach → klug“). „Trainer anlegen…“ ist nicht rückgängig (Plakette „Nicht rückgängig“) und verlangt übernommene Scripts. Die Palette hat „Trainer öffnen“ und die Gruppe „Trainer“ |
| Pokémon › Begegnungen | Liste: Kopf „183 Begegnungsdateien“ (Platin; HeartGold 142), Zeilen mit den Orten der Header, die die Datei nutzen („Route 201“, „verwendet von 1 Ort“), Gruppe „Ohne Ort“ am Ende; „+“ = „Begegnungsdatei anlegen…“ (Leer oder „Kopie von …“). Schau: „Wer hier wartet“ (Arten mit Symbol, Typen und Anteil „Morgens 50 % · Tags 50 % · Nachts 40 %“), „Verwendet von N Orten“ als Referenzen in die Welt, bei mehr als einem Ort je Ort „Für diesen Ort abspalten“, Warnungen. Hand „Bearbeiten“: Reiter je Methode mit Zähler (Gras, Surfen, in HeartGold Fels, Alte Angel, Profi-Angel, Super-Angel, Ersatz), Rate („0 = nie“), Tabelle Platz · Pokémon · Level · Anteil · Ersetzt durch (HeartGold-Gras: Morgens · Tags · Nachts · Level; Wasser „Level (min–max)“), leere Plätze gedämpft (`data-empty`); Ersatz je Liste eine Tabelle, in Platin „Formen“ (Schalellos/Gastrodon Ost, Icognito-Tabelle); in Tüfteln „Rohdaten“ | Anteile, Ersetzungen und Warnungen rechnet der Kernel (`data_get`); die Oberfläche rechnet nichts. Labels „Route 201: Gras Platz 1 STARALILI → PIKACHU“, „Route 201: Gras-Rate 30 → 100“. Die Palette hat die Gruppe „Begegnungen“ |
| Gegenstände | Liste „Gegenstände“ (`data-item` = Item-ID) mit Symbol aus dem ROM, Name, Nummer und Preis rechts, Filter „Taschen“ mit Zählern. Ohne lesbare Item-Tabelle heißen die Zeilen „Daten 0017“ und ein Hinweis sagt, warum. Schau „Übersicht“: Karte mit Symbol, Name, Nummer, Preis; „Beschreibung“ als DS-Textbox mit „Beschreibung bearbeiten“; „Verwendet in“ (Trainer, Team, wild getragen, Entwicklungen). Hand „Bearbeiten“: die Werte in Spalten, in Tüfteln „Rohdaten“ mit Hexblock und den Feldern „Wirkung im Team“. Teilen mehrere Item-IDs einen Datenmember, warnt die Hand und nennt die anderen | Gegenstände sind über die Item-ID geschlüsselt, die Rohdaten über den Datenmember (in Platin ist Trank beides 17) |
| Technik | Liste „Technik“ mit „Prüfen und Bauen“, „Patches“, „Emulator“, „Werkzeuge“ (Untertitel je Eintrag), rechts die Fläche (`data-technik`). Prüfen und Bauen: „Bauen“, „Starten“ (im Browser-Modus gesperrt, „Im Browser-Modus wird nur gebaut“), „Letzter Build“ mit Zeit und Pfad, darunter die Probleme (seit 3b.4a dieselben Gruppen wie im Reiter „Probleme“, dazu „Ausgangslage übernehmen…“ und der faltbare Abschnitt „Prüfungen ein- und ausschalten“ mit einem Schalter `[role=switch]` je Code in `li[data-check]`). Patches: Karte „Dynamische Header“ mit Plakette „Angewendet“, „Verfügbar“, „Fehlerhaft“ oder „Nur Platin und HGSS“ (`data-patch` `applied`, `available`, `broken`, `unsupported`) und „Patch anwenden“ (mit Rückfrage) bzw. „Patch reparieren“. Emulator: Pfad und „Emulator wählen“. Werkzeuge: apicula und rotom-lsp mit Zustand (`data-tool`), ohne Pfad | „Patch reparieren“ hier statt im Assistenten; „Patch anwenden“ geht seit 3f.3 auch bei ungespeicherter Header-Tabelle. In Bauen braucht man Technik dafür nicht: die Zusammenfassung der Assistenten fragt nach dem Patch (Patch-Zeile, siehe „Assistent im Scharnier“); der Kasten steht weiter im Schritt „Header“ (Tüfteln) |
| Einstellungen | „Darstellung“ mit „Thema“ (Hell, Dunkel, Wie System) und „Dichte“ (Komfort, Kompakt), „Emulator“ (dieselbe Zeile wie in Technik), „Tastenkürzel“ (nur die ⌘-Kürzel, nicht die Buchstaben der Werkzeuge; ⌘1 bis ⌘4 mit „(im Bereich Pokémon)“) | wirkt sofort und für alle Projekte; gemerkt in `localStorage` `g4.theme` (`hell`, `dunkel`, `system`) und `g4.density`, sichtbar an `<html>` als `data-theme` und `data-dichte` |

**Assistent im Scharnier.** „Gebiet anlegen…“ und „Raum anlegen…“ (Menü „Neu anlegen“, Palette, „Raum hier anbinden…“ im Inspector) öffnen keinen Dialog mehr: das Scharnier wird zur Schrittleiste (Bereichssymbol, Titel „Gebiet anlegen“ bzw. „Raum anlegen“, Liste „Schritte“ mit „1. Vorlage“ usw., der aktuelle mit `aria-current="step"`, rechts „Abbrechen“, „Zurück“ (⌘[) und „Weiter“ (↵) bzw. „Anlegen“), und der Schritt (Rolle `dialog`, benannt nach dem Titel, `data-wizard`) liegt seit 3e.4 als Streifen `[data-wizard-hand]` (286 px) unter dem Dock; das Dock bleibt und schrumpft, die Schau bleibt bedienbar, der Streifen ist ein Halt der F6-Runde vor der Statusleiste. Schritte in Bauen (seit 3f.3): Gebiet Quelle, Anordnung, Ort, Zusammenfassung; Raum Vorlage, Ort, Eingang, Zusammenfassung; in Tüfteln heißt der Schritt „Ort“ weiter „Header“. **Schritt „Ort“** (Bauen): `Segmented` „Ort“ mit „Vorhandener Ort“ (Taste `button[data-place-picker]`, Picker „Ort wählen“ ohne „Neuer Ort…“) und „Neuer Ort“ (Feld „Name des neuen Orts“, beim Gebiet vorbelegt mit seinem Namen), darunter „Aussehen“ mit `button[data-look-picker]` „Aussehen wie Route 201“ (Picker „Aussehen wählen“); der Header ist immer neu, sein interner Name entsteht von selbst. **Patch-Zeile** (Zusammenfassung in Bauen, solange der Patch fehlt): `[data-patch-consent]` mit der Plakette „Nicht rückgängig“ und dem Kästchen „Einmalig für dieses Projekt: Spielcode für neue Orte anpassen (DSPRE-kompatibel, Sicherung arm9.bin.backup)“; „Anlegen“ ist erst mit Haken frei und wendet den Patch im selben Aufruf an (auch bei ungespeicherter Header-Tabelle; vorher eine Sicherung unter `.g4/backup/dynamicHeaders-<Zeit>/`, `call.sh` zeigt sie im Ergebnis `snapshot`). **„Wo liegt das Gebiet?“** (seit 3f.4, Schritt „Anordnung“ von „Gebiet anlegen“): `Segmented` „Lage“ mit „Eigene Karte (über Türen verbunden)“ und „Auf der Weltkarte Sinnoh (nahtlos begehbar)“; mit einer Weltkarte ein Gitter „Weltkarte, 30 Zeilen, 30 Spalten“ (32 px): ein Klick auf eine leere oder Landschaftszelle setzt die linke obere Zelle, ein Block auf einem Gebiet ist rot (`data-foreign`) mit „Der Block muss auf freie Zellen oder Landschaft fallen.“ und gesperrtem „Weiter“. Der Name aus „Name des neuen Raums“ bzw. „Name des neuen Gebiets“ wird beim Anlegen der Klarname des Headers im Weltbaum (`roomNames` in `g4-world.json`), wie nach „Umbenennen…“. Im Schritt Header „Vorhandenen Header umstellen“ (biegt einen vorhandenen Header um, kopiert nichts) oder „Neuer Header“, Eventdatei „Behalten“, „Neu, leer“ oder „Kopie von NNNN“. Das Ergebnis ersetzt den Schritt („Gebiet angelegt“, „Als Nächstes“) mit „Zeigen“, „Hier spielen“ (im Browser-Modus nur bauen) und „Schließen“. Enter führt weiter, außer ein Feld übernimmt gerade getippten Text; Escape nimmt zuerst getippten Text zurück bzw. leert ein gefülltes Suchfeld, erst das nächste Escape bricht ab. Solange ein Assistent offen ist, verlassen Verweise die Welt nicht, das Prüflicht tut nichts, und ⌘O, ⌘S und ⌘R sind gesperrt, ebenso „Projekt öffnen“, „Speichern“ und „Bauen“ in der Titelleiste. Während er arbeitet (`aria-busy`), ist nichts klickbar.

**Palette.** ⌘K oder „Suchen oder Befehl eingeben“ öffnet den Dialog „Befehlspalette“ (oben mittig). Ohne Eingabe zeigt er nur die Befehle, mit Eingabe die Gruppen Befehle, Orte, Header, Textarchive, Pokémon, Trainer, Begegnungen, Attacken, Gegenstände, Scripts (höchstens acht je Gruppe). Archivzeilen zeigen den Ortsnamen, die Kennung grau und „Textarchiv, verwendet von Header …“; Pokémon zeigen ihre Typen („Normal, Flug“), Attacken ihren Typ, Gegenstände ihren Preis („300 ₽“), je mit vierstelliger Nummer (`0397` findet Staravia, Steinpolitur und TM70). Befehle: „Projekt öffnen“, „Speichern“, „Bauen“, „Hier spielen“, „Gebiet anlegen…“, „Raum anlegen…“, „Liste ein/aus“, „Layout zurücksetzen“, „Thema wechseln“, „Dichte wechseln“, „Begriffe“, „Einstellungen“, „Bereich Welt“, „Bereich Texte“, „Bereich Pokémon“, „Trainer öffnen“, „Attacken öffnen“, „Bereich Gegenstände“, „Bereich Technik“. Gesperrte Befehle nennen den Grund („Erst ein Projekt öffnen“, „Erst Speichern oder Bauen abwarten“, „Dieser Bereich hat keine Liste“). Enter führt aus (ein Header öffnet sich in der Welt, ein Archiv im Bereich Texte, eine Scriptdatei im Reiter „Script“, eine Art, Attacke oder ein Gegenstand in ihrem Bereich), Escape schließt.

**Zwei Wortschätze für `data-lsp`.** Die Plakette im Reiter „Script“ trägt `data-lsp` mit englischen Werten (`starting`, `running`, `missing`, `stopped`; Text „rotom-lsp startet“, „läuft“, „fehlt“, „gestoppt“). Der Eintrag rotom-lsp in Technik › Werkzeuge trägt `data-lsp` mit deutschen Werten (`aus`, `startet`, `bereit`, `fehlt`, `fehler`, und `none`, solange das Script-Panel nie gezeigt wurde). apicula zeigt in Technik „Unbekannt“ bis zum ersten Kartenbild, dann „Bereit“ (das Bild kann aus dem Modell-Cache kommen); nach einem Fehler bleibt „Fehlt“ für die Sitzung.

**Escape in Feldern.** In der ganzen App leert Escape ein gefülltes Suchfeld zuerst und nimmt in einem anderen Feld den getippten, noch nicht übernommenen Text zurück; erst das nächste Escape geht weiter (schließt etwa einen Dialog oder bricht den Assistenten ab).

**Werkzeuge** (Leiste in 3D). Die Leiste hat eine Taste je Werkzeug (`[data-tool=select|paint|figure|door|trigger|sign|building]`, Name als `aria-label` („Auswählen“ …), `aria-pressed` genau bei einer, `aria-keyshortcuts` der Buchstabe; Pfeiltasten wandern durch die Tasten). Ohne gezeigten Ort oder mit gesperrter Eventdatei sind die vier Ereigniswerkzeuge gesperrt (`aria-disabled`, der Grund als Tooltip und Beschreibung, der Klick tut nichts). Davor sitzt der Griff zum Ziehen (nur für den Zeiger, `aria-hidden`) und das Menü „Lage der Werkzeugleiste“ mit „Links“, „Oben“, „Unten“ (der Weg für Tastatur und Agent; die Wahl steht in `g4.shell.v1` als `toolbar`). Der Buchstabe wählt das Werkzeug (nicht im Textfeld und nicht mit Umschalttaste oder Cmd), und zwar nur, solange die 3D-Ansicht im Bild ist; im Raster, in der Weltkarte oder in einem anderen Bereich tut er nichts und ein Hinweis sagt einmal „Werkzeuge wirken in der 3D-Ansicht“. Esc wählt wieder „Auswählen“:

| Werkzeug | Taste | Klick in der Szene |
|---|---|---|
| Auswählen | V | wählt Marker und Gebäude, Ziehen verschiebt; malt nie |
| Gelände malen | B | setzt die Kollision der Kachel, Umschalt-Klick löscht sie |
| Figur | F | stellt eine Figur hin (Objekt-Event) |
| Tür | T | legt einen Warp an (ersetzt den Knopf „Warp hinzufügen“ der Szenenleiste; seit 3e.3 wählt auch „+“ › „Tür“ in „In dieser Szene“ dieses Werkzeug) |
| Auslöser | A | legt einen Trigger an |
| Schild | S | stellt ein Hintergrund-Event auf |
| Gebäude | G | setzt ein Gebäude (ersetzt den Knopf „Neues Gebäude“ der Szenenleiste) |

Nach einem Klick mit Figur, Tür, Auslöser, Schild oder Gebäude steht wieder „Auswählen“. Gemalt wird nur mit „Gelände malen“; ein Klick mit „Auswählen“ ändert nie die Karte. Den Knopf „Warp hinzufügen“ im Abschnitt „Eingänge / Ausgänge“ des Header-Editors gibt es seit 3e.3 nicht mehr (den Abschnitt auch nicht, an seiner Stelle steht die Taste „N Türen, M ohne Ziel“): Türen setzt man mit dem Werkzeug „Tür“ (T) oder mit „+“ › „Tür“ im Kopf von „In dieser Szene“ (das wählt dasselbe Werkzeug) und einem Klick auf die Kachel. „Neues Gebäude“ bleibt im Gebäude-Editor.

**Galerie.** `http://127.0.0.1:1430/?gallery` zeigt alle Bausteine ohne Projekt, mit Schaltern für Thema und Dichte; für die Screenshots des Gates (Hell, Dunkel, Kompakt) siehe [dev-setup.md](dev-setup.md), Abschnitt „Galerie“.

**Layout.** Seit 3e.4 merkt jedes Dock sein Layout unter einem Schlüssel je Bereich (`g4.layout.v11.welt`, `g4.layout.v11.texte`, `g4.layout.v11.pokemon`, `g4.layout.v11.gegenstaende`) und die zuletzt gebaute Vorlage unter `g4.layout.v11.<bereich>.arrangement` (`werkbank` oder `doppeldeck`; beim ersten Aufbau der Standard des Bereichs); dazu `g4.shell.v1` für den Rahmen (Einklappen der Liste, Anker der Werkzeugleiste, Höhe der Orte `worldListSplit`, `placesFolded`, Unterbereich von Pokémon, Auswahl je Teil; `handHeight` und `handCollapsed` stehen noch dort, werden aber nicht mehr geschrieben). Ein unbrauchbares Layout (fehlende Panels, ein schwebendes Schau-Panel) setzt nur dieses Dock auf seine Vorlage zurück. Layouts älterer Versionen (`g4.layout.v10.*` und davor) löscht die App beim Start. Palette „Layout zurücksetzen“ baut die gemerkte Vorlage des gezeigten Bereichs neu (auf Start, Technik und Einstellungen die der Welt; ist das Dock nicht gebaut, löscht es nur seine gespeicherten Schlüssel). Alles zurücksetzen im Browser-Modus: `localStorage.clear()` und die Seite neu laden.

## Werkzeuge

**In-Seite-Dialoge.** Im Browser-Modus gibt es keine nativen Dialoge: Fragen erscheinen auf der Seite als Dialog „Rückfrage“ mit „Nein“ und „Ja“, Pfadabfragen mit dem Feld „Pfad“, „Abbrechen“ und „OK“ (Enter im Feld nimmt den Pfad). „Bauen“ fragt „Speichern unter“, vorbelegt mit `<Projekt>/out.nds`; „OK“ baut, Meldung „ROM gebaut“. Escape heißt „Nein“ bzw. „Abbrechen“. Fehlt die Brücke, steht „Brücke nicht erreichbar“ in der Statusleiste.

**`scripts/agent/call.sh <befehl> ['<json>']`** ruft einen Befehl der laufenden Brücke auf (dieselbe Sitzung wie die Seite) und druckt die Antwort; bei Fehlern Exit 1 und der Fehler auf stderr. Argumente heißen wie in `apps/studio/src/api.ts` (camelCase). Beispiele:

```sh
scripts/agent/call.sh dirty                                  # ungespeicherte Teile, [] heißt sauber
scripts/agent/call.sh get_header '{"id":415}'                # Felder inklusive event_file_id
scripts/agent/call.sh get_events '{"eventFileId":394}'       # Warps mit index, x, z, dest_header_id, dest_warp_id
scripts/agent/call.sh update_event '{"eventFile":394,"kind":"warp","index":0,"patch":{"dest_header_id":412,"dest_warp_id":0}}'
scripts/agent/call.sh header_patch_status                    # supported, applied, repairable
scripts/agent/call.sh save
scripts/agent/call.sh build_rom '{"out":"<Kopie>/out.nds"}'
scripts/agent/call.sh history                                # {"undo":[…],"redo":[…]}, die Labels des Verlaufs
scripts/agent/call.sh data_get '{"kind":"species","index":397}'   # Pokédex-Dokument: Personal, Lernset, Entwicklung, TM-Bits, Sperrgrund
scripts/agent/call.sh data_edit '{"kind":"move","index":17,"patch":{"Move":{"field":"power","value":65}}}'
scripts/agent/call.sh data_raw '{"kind":"item","index":17}'     # Bytes, Feldkarte, Pfad, Member (Tüfteln)
scripts/agent/call.sh refs '{"target":{"kind":"move","index":17}}'
scripts/agent/call.sh get_item_icon '{"item":17}'            # druckt nur "<n> bytes" (PNG), 0 Bytes = kein Symbol
```

Die Datenbefehle: `data_list`, `data_get`, `data_edit`, `data_raw` mit `kind` ∈ `species`, `learnset`, `evolution`, `move`, `item`, `encounter` und `trainer`, außerdem `name_tables`, `item_table` und `machine_table` (`null` heißt: arm9-Tabelle nicht verfügbar), `refs`, `get_item_icon`, `get_mon_icon`. Bei `item` ist `index` die Item-ID. Patches heißen wie im Szenario `pt-daten.yaml` (`Personal`, `Learnset`, `Move`, `Item` …), für Trainer `TrainerParty {party: […]}`, `Trainer`, `TrainerName`, `TrainerFormat`, `TrainerTexts`, für Begegnungen `Encounter {method, slot?, field, value}` (`method` auch ein Ersatzlisten-Schlüssel wie `day`, `night`; HeartGold-Gras mit `species_morning`, `species_night`; Beispiele in `scripts/agent/scenarios/*-trainer-begegnung.yaml`); ohne `label` baut der Kernel das Label des Verlaufs selbst.

Seit 3b.3b dazu: `create_trainer {"class":9,"name":"Jens"}` → `{id, placeholder, script_extended}` (sofort auf der Platte, nicht rückgängig, braucht übernommene Scripts), `set_trainer_texts {"trainer":928,"texts":[{"kind":0,"text":"Halt!\r"}]}` → `{changed, problems}`, `create_encounter_file {"copyOf":null}` → Index der neuen Datei (nicht rückgängig), `split_encounter_file {"header":411}` → `{index, changed}` (die Kopie bleibt, das Umhängen ist ein Schritt), `add_event` mit optionalem `patch` und `label` (Figur und Felder in einem Schritt, zum Beispiel `{"script":3927,"trainer_type":1,"sight":3}`), `update_event` kennt `trainer_type` und `sight`.

Änderungen per `call.sh` sieht die Oberfläche erst nach einem Neuladen der Seite (`?project=` öffnet das Projekt neu).

Seit 3b.4a dazu die Prüfungen. `run_checks`, `checks_status`, `ignore_finding`, `unignore_finding`, `accept_baseline` und `set_check_enabled` antworten mit dem Bericht `{findings, ignored, skipped: [{code, why}], groups, ranMs, generation, running, progress?}` (`groups: []` heißt „kein Ergebnis“, nie „alles gut“). Ein Befund trägt `code`, `severity`, `subject {kind, index}`, `key`, `label` („Eventdatei 0394 · Warp 0“), `args`, optional `target` und `also`, `fixes [{id, label, irreversible}]` und `fingerprint` (16 Hex-Ziffern, so zurückgeben, wie er kam); `plan_fix` und `apply_fix` nehmen den Befund, wie `run_checks` ihn liefert (gelesen werden `code`, `subject`, `key`, `fingerprint`):

```sh
scripts/agent/call.sh run_checks '{"scope":"all"}'          # voller Lauf, in der Brücke synchron; "cheap" nur die billige Gruppe, "full" = "all"
scripts/agent/call.sh checks_status                         # der Bericht des aktuellen Stands (sonst groups: []), running, progress; prüft nicht
F=$(scripts/agent/call.sh run_checks | python3 -c 'import json,sys; print(json.dumps([f for f in json.load(sys.stdin)["findings"] if f["code"]=="text.too-wide"][0]))')
scripts/agent/call.sh plan_fix "{\"finding\":$F,\"fix\":\"wrap\"}"     # {changes: [{asset, label, before, after}], irreversible}, ändert nichts
scripts/agent/call.sh apply_fix "{\"finding\":$F,\"fix\":\"wrap\"}"    # {changed: [AssetId], remaining}; ein Schritt „Behoben: text.too-wide“
W=$(scripts/agent/call.sh run_checks | python3 -c 'import json,sys; print(json.dumps([f for f in json.load(sys.stdin)["findings"] if f["severity"]!="error"][0]))')
scripts/agent/call.sh ignore_finding "$W"                   # nimmt code, subject, key, fingerprint; unignore_finding ebenso
scripts/agent/call.sh accept_baseline                       # alle Warnungen und Hinweise ausblenden
scripts/agent/call.sh set_check_enabled '{"code":"text.three-lines","enabled":false}'
```

Die Fix-Ids sind `wrap` (`text.too-wide`), `append_message` (`script.message-missing`), `swap_min_max` (`encounter.min-max`) und `create_script` (`event.script-missing`, braucht übernommene Scripts und gespeicherte Textarchive, schreibt sofort, kein Schritt im Verlauf). Hat sich die Stelle seit dem Befund geändert, lehnt `apply_fix` mit dem Code `finding_stale` ab („finding is no longer current“) und schreibt nichts. Fehler lassen sich weder ausblenden noch abschalten. Ausblenden, Ausgangslage und Schalter schreiben sofort `<Kopie>/g4-checks.json` (`{ignored: [{code, subject, key, fingerprint}], disabled: [code]}`); eine neue Sitzung (Neuladen der Seite, nächster Batch eines Szenarios) liest die Datei beim Öffnen.

**Emulator** (`cd scripts/g4emu`; alle Pfade zu `../Roms` sind von dort `../../../Roms`; aus einem Git-Worktree stimmt das nicht, dann `G4_ROMS_DIR` absolut setzen und `--rom`, `--project` und `--save` absolut angeben):

```sh
uv run g4emu run --rom <rom.nds> --save ../../../Roms/_testkopien/_saves/CPUD-zimmer.sav \
  --out ../../target/agent/gate-$(date +%F)/<punkt> continue expect-map 415 shot start
```

Schritte: `continue`, `wait N`, `press TASTE[:N]`, `mash TASTE N`, `tap X Y`, `walk RICHTUNG N`, `walk-to X Y`, `expect-map ID [N]`, `where`, `shot NAME`, `state NAME`, `write-save NAME`, `dump-ram NAME`, dazu die Kampfschritte `walk-until-battle RICHTUNG N [back-and-forth]`, `expect-battle ART LEVEL [N]`, `end-battle [N]`, `win-battle [N]` (unten) und seit 3g.2 `new-game`, `walk-until-blocked RICHTUNG N` und `expect-at X Y` (unten, „Neues Spiel ohne Spielstand“), seit 3d.1 `flag N`, `set-flag N 0|1`, `var N`, `set-var N W`, `expect-flag N W` und `expect-var N W` (unten, „Flags und Variablen im Spielstand“). Rückgabe: 0 ok, 2 ein Schritt hat sein Ziel verfehlt (`walk`, `expect-map`, Kampfschritte; die Zeile nennt Position, zuletzt gelesene ID bzw. die zuletzt gesehenen Kampfdaten), 1 sonstiger Fehler (ROM oder Kern fehlt). Jeder Schritt druckt eine JSON-Zeile; im Ausgabeordner liegen `last.png` und `last.state` (mit `--state <out>/last.state` geht es dort weiter, um Schritt für Schritt zu erkunden) und `core.log`. Der Spielstand wird immer zuerst kopiert; `--save` ist nach dem Lauf unverändert. Spielcodes in `scripts/g4emu/g4emu/games.yaml`: CPUD, IPKD, IPKE.

- Platin DE: Start im Spielerzimmer (Header 415) bei x 7, y 4. Die Treppe ist der Warp 0 von Eventdatei 394 bei x 8, z 4: `walk-to 8 4`, `walk right 1` führt nach 414 (Ankunft x 10, y 3). Beim Ankommen läuft die Überblendung noch: `expect-map` kehrt früh zurück, danach `wait 150`, bevor man ein Bild macht oder weitergeht (die Figur macht nach der Ankunft von selbst einen Schritt).
- HeartGold (IPKD/IPKE): Start im Zimmer oben (Header 64, x 3, y 4). `walk left 1` bringt auf die Treppenkachel (2, 4), `walk down 1` löst den Warp nach 63 aus (Ankunft x 3, y 3). Header-IDs im `g4k` sind dezimal.
- Ein Warp feuert nur, wenn die Figur auf der Warp-Kachel steht **und** einen Schritt in Richtung Ausgang macht: nach `walk-to` kommt immer noch ein `walk RICHTUNG 1`.
- Position unklar: `uv run g4emu run … continue where` oder `walk-to` bis zum Rand; keine Wegsuche, ein blockierter Weg meldet „blockiert bei x, y“.

**Kämpfe** (`scripts/g4emu/g4emu/battle.py`, Werte je Spielcode in `games.yaml` unter `battle`, `battle_pointer`, `home_header`). `where` liefert im Kampf weiter Header und Position und taugt nicht als Kampfanzeiger. Erkannt wird ein Kampf an den BattleMon-Datensätzen im RAM: acht Bytes `06` (neutrale Statuswerte) an +0x18, Art (u16) an +0, Level (u8) an +0x34, Art 1–507, Level 1–100, auf 4 Byte ausgerichtet. Ein Kampf läuft, wenn es mindestens einen Treffer gibt und der Kampfzeiger ≠ 0 ist: Platin (CPUD) `u32 @ 0x02101F88`, HeartGold DE (IPKD) `u32 @ 0x0211DB34`; HeartGold US (IPKE) ist nicht gemessen und hat keinen Zeiger, dort zählen die Treffer allein. Mit Team trifft auch das eigene Pokémon (0xC0 vor dem Gegner), deshalb vergleicht `expect-battle` Art und Level mit jedem Treffer.

- `walk-until-battle RICHTUNG N [back-and-forth]`: läuft bis zu N einzelne Kacheln, mit `back-and-forth` nach jeder Kachel in die Gegenrichtung, und endet, sobald ein Kampf läuft (`steps` = gelaufene Kacheln). Beginnt ein Kampf, steht die Figur still und der nächste Schritt meldet „blockiert“; dann wartet der Schritt bis zu 600 Bilder auf den Kampf. Exit 2 „kein Kampf nach N Schritten“ bzw. „blockiert bei x, y, kein Kampf …“.
- `expect-battle ART LEVEL [N=600]`: prüft alle 10 Bilder; Erfolg druckt `species`, `level`, `offset` (ARM9-Adresse des Treffers, wandert mit dem Heap) und `frame`. Exit 2 „kein Kampf nach N Bildern“ oder „Gegner … nicht gefunden …; gesehen: 151 Lv 7 bei 0x…“. HGSS zeigt die Datensätze erst 120–180 Bilder nach Kampfbeginn; nach `walk-until-battle` sind sie schon da.
- `mash TASTE N`: N Bilder lang alle 8 Bilder die Taste (4 Bilder gedrückt).
- `end-battle [N=6000]`: drückt A alle 8 Bilder, bis der Kampf vorbei ist und der Ort 60 Bilder lang denselben Header zeigt, und druckt den Ort. Nach Kampfende kommt kein A mehr. Vorbei heißt: Kampfzeiger 0. Das Fehlen der Signatur allein reicht nicht, denn ein gesenkter Statuswert (etwa durch Heuler) löscht sie mitten im Kampf. Ohne Zeiger (IPKE) ist der Kampf vorbei, wenn kein Treffer mehr da ist und kein im Kampf gesehener Datensatz noch Art und Level trägt. Ohne Team endet ein Wildkampf mit der Niederlage: der Spieler steht danach zu Hause (Platin 414, x 8, y 8; HGSS 63, x 6, y 8), auf dem Bildschirm steht noch der Heimweg-Text (Platin) bzw. Schwarz (HGSS); vor dem nächsten Schritt `mash a 300`.
- `win-battle [N=6000]`: wie `end-battle`, scheitert aber mit „Niederlage: nach dem Kampf zu Hause …“, wenn der Header danach `home_header` ist. Braucht ein Pokémon im Team. Gemessen aus den Kampf-Savestates der Trainer-Recherche (Lucario L7 gegen Knakrack L100): Platin endet nach ~1200 Bildern auf 415 (7, 5) noch mit schwarzem Bild, HeartGold DE nach ~1580 Bildern auf 64 (4, 5) mit der Karte. Weil nach dem Kampfende kein A mehr kommt, kann danach noch ein Text des Trainers offen sein oder die Überblendung laufen: vor dem nächsten Gang `wait 120` und ein Bild; offenen Text mit `press b` schließen (ein A spricht den Trainer erneut an).
- Grenzen: Der HGSS-Zeiger 0x0211DB34 ist mit einem Wildkampf (Niederlage) und einem Trainerkampf (Sieg) gemessen, auf zwei Projekten, aber nur in HeartGold DE. IPKE hat keinen Zeiger; ob dort nach einem Sieg die Datensätze überschrieben werden, ist nicht gemessen (in IPKD geschieht es: Art oder Level ändern sich). Bleibt ein Datensatz stehen, scheitert `end-battle` mit „Kampf läuft noch nach N Bildern“.
- Aus den Szenarien 3b.3b (Kopien `pt-3b3b`, `hg-3b3b`, `*-3b3b-final`): Platin und HeartGold DE gewinnen je zwei Trainerkämpfe mit `win-battle` und stehen danach auf dem Ort des Kampfs (nicht zu Hause). Ohne Pokémon im Spielstand geht jeder Kampf verloren: zuerst eine Figur ansprechen, die einmalig ein Pokémon auf Level 100 gibt (Platin `GivePokemon`, HeartGold `GiveMon`; eine Art ohne Bedroher oder Adlerauge, etwa 445 Knakrack, sonst ändert ein Statuswert die Signatur). Figuren der Karte, auf der der Spielstand gemacht wurde, kommen nach `continue` aus dem Spielstand: der Geber gehört auf den neuen Ort, nicht ins Zimmer. Direkt nach `expect-battle` zeigt der Bildschirm noch die Überblendung: vor `shot` `wait 300` (wild `wait 400`). Ein Kampf per Sichtlinie startet immer das „gesehen“-Script (Platin 3928, HeartGold 3739); nur Ansprechen (`press RICHTUNG:8`, `press a`) läuft über den Eintrag des Trainers selbst (siehe [formats/trainer_scripts.md](formats/trainer_scripts.md)).
- Gemessen (Kopien `pt-3b3b-enc`, `hg-3b3b-enc`, ROM `a.nds` mit Mewtu Lv 5 im Gras des neuen Headers): Platin `continue expect-map 415 walk-to 8 4 walk right 1 expect-map 593 wait 150 walk up 1 walk-until-battle up 20 back-and-forth expect-battle 150 5 end-battle`, HeartGold `continue expect-map 64 walk left 1 walk down 1 expect-map 540 wait 150 walk-until-battle up 20 back-and-forth expect-battle 150 5 end-battle`; `end-battle` meldet 414 bzw. 63.
- Aus den Gesamtgate-Szenarien (3b.4b): Gras mit Rate 100 gibt oft einen Kampf (Platin in hohem Gras 70 % je Schritt, HeartGold ab dem fünften Schritt nach dem Kartenwechsel 20 %, beim Hin-und-her bis 80 %; [Recherche](research/2026-10-23-3b3/encounter-mechanics-3b3b.md) §1). Wege, die ein Szenario mit `walk-to` geht, gehören deshalb auf gewöhnlichen Boden, nur `walk-until-battle` ins Gras. Wer auf einer Tür ankommt, macht von selbst einen Schritt (in den Szenarien Platin nach Osten, HeartGold nach Westen); ein Weg darf danach nicht über die Tür-Kachel führen (Kollision 0x80, Warp). Text, der nach einer Figur kommt, mit `mash b` wegdrücken: B schließt eine Nachricht, spricht die Figur aber nicht erneut an (`mash a` tut das).

**Neues Spiel ohne Spielstand** (seit 3g.2). `g4emu run` ohne `--save` und ohne `--state` startet die ROM wie eine frische Karte. Der Schritt `new-game` spielt dann das Intro des Spielcodes ab Einschalten: die gemessene Folge steht in `games.yaml` unter `intro` (eine Zeile Schritte), der Endpunkt unter `intro_ends: {header, x, y}`. Weicht der Ort danach ab, scheitert der Schritt mit „Intro endet bei Header …, x …, y … statt …“ (Exit 2); ein Spielcode ohne `intro` (IPKE) scheitert mit „IPKE: kein intro in games.yaml“. Die Teilschritte drucken keine eigenen Zeilen; die Zeile von `new-game` nennt den Ort und `intro_frames`.

- Platin DE (CPUD): ~10 400 Bilder, Spieler und Rivale „AAAAA“, Junge; Ende im Zimmer 415 bei x 4, y 6. Platin zeigt im Intro einen veralteten Ort (415, 4, 6), erst der erste Gang belegt die Steuerung. Nach dem ersten Schritt stürmt der Rivale herein (`mash b 1500`), unten kommt die Mutter (`mash b 600`), an der Tür hält sie den Spieler mit einem Text auf (`press down:8 wait 60 … mash b 400 press down:8`).
- HeartGold DE (IPKD): ~4 860 Bilder, Spieler „AAAAAAA“ (der Mash tippt sieben A), Junge; Ende im Zimmer 64 bei x 6, y 6. Unten gibt die Mutter den Pokégear (`mash b 2400` bis zum Ende); Neuborkia erreicht man über die Tür (3, 10) von 63.
- Texte schließt `mash b`: B blättert Texte weiter und tut in der Oberwelt nichts, `mash a` dagegen spricht im Zimmer Gegenstände an. Für das Startmenü `press x:4`, zurück mit zwei `press b:4` (dazwischen `wait 120`; ein B direkt nach der Team-Ansicht geht im Überblenden verloren und lässt das Menü offen).
- Gemessen am 25.10.2026 mit `_testkopien/3g-probe/pt.nds` und `hg.nds`; die Läufe sind deterministisch. Herleitung und Stolpersteine: `target/agent/probe-intro/INTRO.md` (nicht im Repo).
- `walk-until-blocked RICHTUNG N` (N ≥ 1, sonst lehnt schon das Einlesen ab): läuft bis zu N einzelne Kacheln und endet ok, sobald nach mindestens einer gelaufenen Kachel eine blockiert (`steps` = gelaufene Kacheln, dazu der Ort). Exit 2 „blockiert sofort (0 Schritte) bei x …, y …: Menü oder Text offen?“, wenn schon die erste Kachel nicht geht (ein offenes Menü, ein Text oder ein Script hält die Figur genauso fest wie eine Wand), „nicht blockiert nach N Schritten (x …, y …)“, wenn alle N gehen, und „Header … statt … nach k Schritten“, wenn der Header wechselt (ein Warp oder ein Übergang in eine andere Karte).
- `expect-at X Y`: der Spieler steht jetzt auf der Kachel (X, Y), sonst Exit 2 „Spieler bei x …, y … statt x …, y …“. Gehört direkt hinter `walk-until-blocked`: erst die gemessene Haltestelle belegt, dass die Wand die erwartete ist.
- Der Szenario-Lauf legt die JSON-Zeilen des Spiels in `<out>/play.jsonl` ab (Schrittzahlen und Haltestellen bleiben nachprüfbar).
- Im Szenario: `save: none` (einziger erlaubter Wert) spielt ohne Spielstand, der Lauf ruft `g4emu run` dann ohne `--save` auf; `play` beginnt mit `new-game`. Die frische Kopie unter `_testkopien` bleibt Pflicht.

**Flags und Variablen im Spielstand** (seit 3d.1). Die Schritte lesen und schreiben den lebendigen Spielstand-Block im RAM; die Lage je Spielcode steht in `games.yaml` unter `save_block` (`pointer`, `header`, `vars`, `vars_count`, `flags`, `flags_bytes`; dieselben Werte wie `crates/g4-kernel/metadata/save_layout.yaml`, Erklärung in [formats/flags_vars.md](formats/flags_vars.md), „RAM-Lage“). `N` ist dezimal oder `0x…`, Flags 0 bis `0xB5F`, Variablen `0x4000` bis `0x411F` (Platin) bzw. `0x416F` (HeartGold).

- `flag N` druckt `{"flag": N, "value": 0|1}`, `var N` druckt `{"var": N, "value": W}` (N in der Zeile dezimal).
- `set-flag N 0|1` und `set-var N W` (W 0–65535) schreiben und drucken den Wert danach. Ein Sichtbarkeits-Flag wirkt erst beim nächsten Laden der Karte, ein Wert, den ein Script oder Auslöser liest, sobald das Spiel ihn liest.
- `expect-flag N W` und `expect-var N W`: Exit 2 „Flag 0x0173 ist 0 statt 1“ bzw. „Variable 0x40A5 ist 1 statt 0“, sonst die Zeile wie `flag`/`var`.
- Vor dem Laden des Spielstands (vor `continue`, beim Einschalten) ist der Zeiger ungültig: Exit 2 „kein gültiger Spielstand im RAM“. Ein Spielcode ohne `save_block` scheitert mit „save_block fehlt in games.yaml“, eine Nummer außerhalb des Blocks oder ein Wert außerhalb des Bereichs ist eine falsche Eingabe (Exit 1).

**Szenario** (Projekt ändern, bauen, spielen in einem Lauf), aus `scripts/g4emu` heraus; die Kopf-Kommentare der Dateien nennen die Platzhalter und ein Beispiel für `--set`:

```sh
uv run g4emu scenario ../agent/scenarios/pt-treppe-warp.yaml \
  --project ../../../Roms/_testkopien/<kopie> --out ../../target/agent/gate-$(date +%F)/szenario \
  --set WARP=0 --set ZIEL=412 --set TREPPE_X=8 --set TREPPE_Y=4 --set TUER_X=6 --set TUER_Y=10 --set ZURUECK=411
```

- `pt-treppe-warp.yaml`: Treppe des Spielerzimmers auf ein anderes Haus umbiegen, hin und durch die Tür wieder hinaus. Erwartete Header-Folge 415, 412, 411.
- `pt-neuer-header.yaml`: Patch anwenden, Gebiet mit **neuem** Header anlegen, die Treppe dorthin umbiegen, hin und zurück (`NEUER_HEADER` ist die Zahl der Header vorher, 593 bei Platin DE; `NEUE_EVENTDATEI` 534).
- `pt-raum-assistent.yaml`: Raum-Assistent als ein Aufruf (neuer Header als Kopie des Spielerzimmers, Eventdatei „Kopie von 0394“, Tür außen auf einer freien Kachel von Zweiblattdorf, Tür-Kacheln an), hinein, über die kopierte Treppe hinaus nach 0411 und durch die neue Tür wieder hinein. Erwartete Header-Folge 415, 593, 411, 593 (`--set NEUER_HEADER=593 --set TUER_X=105 --set TUER_Y=876`).
- `pt-daten.yaml` (ohne Platzhalter): Datenänderungen über `data_edit` (Staravia 0397 Angriff 75 → 85, Lernset 12 → 13 Einträge, neu Attacke 0019 auf Level 50, Flügelschlag 0017 Stärke 60 → 65, Trank 0017 Preis 300 → 250), speichern, bauen, dann der Prüfschritt `check:` und ein Starttest (`continue`, `expect-map 415`, `shot start`).
- `pt-trainer-begegnung.yaml` und `hg-trainer-begegnung.yaml` (Gate 3b.3b): neuer Ort mit neuem Header, neue Begegnungsdatei (Gras-Rate 100, alle Slots `S_WILD`/`L_WILD`), Route 201 bzw. Route 29 Gras-Slot 0 geändert (Prüfschritt), zwei Trainer mit je einem Pokémon als Figuren mit Sichtlinie, ein Geber, der einmalig ein Pokémon auf Level 100 gibt, Treppe des Zimmers auf den neuen Ort; spielen: Geber, Trainer 1 per Sichtlinie, Trainer 2 angesprochen, beide gewinnen, dann wild im Gras. Platzhalter `NEUER_HEADER` (Platin 593, HeartGold 540), `S_WILD L_WILD S_T1 L_T1 S_T2 L_T2 S_GIFT` (Beispiel `--set S_WILD=150 --set L_WILD=5 --set S_T1=25 --set L_T1=12 --set S_T2=133 --set L_T2=14 --set S_GIFT=445`). Ablauf und Ergebnis: [phase-3b3-gate.md](phase-3b3-gate.md), (B2) und (B3).
- `pt-pruefungen.yaml` (Gate 3b.4a, ohne Platzhalter, ohne Spiel): Scripts übernehmen, Ausgangslage übernehmen, dann vier absichtliche Fehler einzeln rot (`warp.dest-warp`, `event.script-missing` mit einer neuen Scriptdatei an Header 0022, `event.trainer-missing`, `text.too-wide`) und zurück über `update_event`, `apply_fix create_script` und `apply_fix wrap` grün; jede Prüfung ist ein `run_checks` mit Erwartungen. Ablauf und Ergebnis: [phase-3b4-gate.md](phase-3b4-gate.md), (A2).
- `pt-kleiner-hack.yaml` und `hg-kleiner-hack.yaml` (Gate 3b.4b, 3b-Gesamtgate; Fortsetzung der Szenarien von 3b.3b): Patch, Scripts übernehmen, Ausgangslage, dann ein neuer Ort (1 × 1 aus Route 201 Karte 5 bzw. Route 29 Karte 1, neuer Header, leere Eventdatei), die Treppe umgebogen (rot `warp.dest-warp`) und der Ankunfts-Warp (grün); Modell und Kollision von Karte 6 bzw. 2 exportiert, geprüft und importiert, der Gegenfall (synthetisches Modell über 0xF000 aus `prepare:`) abgelehnt; die Tür-Kachel an der Ankunft, danach die Kollision der neuen Karte noch einmal exportiert; neue Begegnungsdatei mit allen Gras-Slots und allen Ersatzlisten, neue Scriptdatei, eigenes Textarchiv; zwei Trainer (rot `trainer.no-party`, mit Team grün), eine angefügte zu lange Zeile (rot, `apply_fix wrap`), eine Figur mit fehlendem Script 3 (rot, `apply_fix create_script`), dann `save_script` mit Geber (Pokémon einmalig per Flag) und Nachbar (Trank einmalig per Flag, danach ein anderer Text). Jede Prüfung ist ein `run_checks` mit `groups.length = 3`. `check:` vergleicht Modell und Kollision der neuen Karte im ROM per `${sha1-of:…}` mit den Exporten. Gespielt: Ankunft, Geber, Nachbar zweimal, der Weg, den nur die importierte Kollision freigibt (Platin `walk-to 5 17`, HeartGold `walk-to 20 24`), Trainer 1 per Sichtlinie, Trainer 2 angesprochen, beide gewonnen, wild im Gras. Platzhalter `NEUER_HEADER` (Platin 593, HeartGold 540), `KOPIE` (absoluter Pfad der Kopie; die Exporte gehen nach `<Kopie>/export/`), `S_WILD L_WILD S_T1 L_T1 S_T2 L_T2 S_GIFT` wie in 3b.3b. Ein Lauf dauert unter einer Minute (23 Brücken-Sitzungen, gemessen 47 bis 49 s), dazu einmal der Bau der Werkzeuge. HeartGold US: dieselbe Datei mit `game: IPKE`. Rezept „Gesamtgate“ unten; Ablauf und Ergebnis: [phase-3b4-gate.md](phase-3b4-gate.md), (B2) bis (B3a).
- `pt-kleiner-hack-ohne-import.yaml` und `hg-kleiner-hack-ohne-import.yaml` (Gegenprobe zu Gate 3b.4b): dieselben Szenarien ohne den gelungenen `import_map_parts`, ohne `check:`, gespielt bis `shot weg`. Sie **müssen scheitern**: `walk-to 5 17` endet mit „blockiert bei x 5, y 21“ (Platin), `walk-to 20 24` mit „blockiert bei x 11, y 24“ (HeartGold), die Zusammenfassung mit `"ok": false`. Damit ist belegt, dass der Weg der Gesamtgate-Szenarien nur mit der importierten Kollision geht. Platzhalter wie die Szenarien, je eine eigene frische Kopie.
- `pt-mcp-route.yaml` und `hg-mcp-route.yaml` (Gate 3h.2, über MCP statt über die Brücke, siehe „Szenarien über MCP“): dieselbe Lage wie `*-trainer-begegnung.yaml` (Patch, neues Gebiet mit neuem Header, Begegnungsdatei, zwei Trainer mit Kampftexten, Geber mit Script-Text, Treppe, Ankunft), aber die Begegnungen als **ein** `g4_batch` (ein Undo-Schritt, danach `history`, `undo`, `data_get`, `redo` mit Erwartungen); spielen: Ankunft, Geschenk, beide Trainerkämpfe, Wildkampf. Platzhalter wie bei den Trainer-Szenarien (`NEUER_HEADER` Platin 593, HeartGold 540).
- `pt-flag.yaml` und `hg-flag.yaml` (Gate 3d.1, über MCP mit `mcp: [--allow-irreversible, --expert]`): Basis zuordnen, als Textprojekt einrichten, neues Gebiet mit Eventdatei und Scriptdatei, Treppe dorthin, `free_flag` und `name_flag` → `FLAG_SZENARIO_FIGUR`, Figur mit diesem Sichtbarkeits-Flag und Script `SetFlag FLAG_SZENARIO_FIGUR`; spielen: Flag 0, ansprechen, Flag 1, Karte verlassen und betreten, Flag weiter 1, die Figur ist weg. Platzhalter `BASIS` (`../<Kopie>.nds`), `NEUER_HEADER` (Platin 593, HeartGold 540), `X_FIGUR` und `Z_FIGUR` (Platin 54/17, HeartGold 15/13); Befehle im Abschnitt „Phase 3d.1“, Ergebnis in [phase-3d-1-gate.md](phase-3d-1-gate.md), (2).
- `pt-leere-region.yaml` und `hg-leere-region.yaml` (Gate 3g.2, ohne Platzhalter, `save: none`): Scripts übernehmen, `apply_empty_region` (Erwartung `plan.start_header` 411 bzw. 60), `run_checks` mit 0 Fehlern in allen drei Gruppen, bauen; dann ein neues Spiel: `new-game`, Zimmer, Treppe, Mutter, Tür, Ankunft in der leeren Startstadt (keine Figuren), das Start-Script („Willkommen in deiner leeren Region! Hier ist dein erstes Pokémon.“), Menü mit POKéDEX und POKéMON, Team mit Chelast bzw. Endivie Lv. 5, zuletzt `walk-until-blocked` bis an die Rand-Karte und `expect-at` auf die gemessene Haltestelle: Platin `walk left 4` (x 112), `walk-until-blocked up 40` und `expect-at 112 864` (22 Kacheln, wo vorher Route 201 begann), HeartGold `walk down 2` (y 399), `walk-until-blocked left 40` und `expect-at 672 399` (23 Kacheln, wo vorher Route 29 begann; Neuborkias Norden sind Bäume). Erwartete Header-Folge 415, 414, 411 bzw. 64, 63, 60; Bilder `zimmer`, `mutter`, `ankunft`, `start-script`, `menue`, `team`, `rand`. Die Kopie kommt aus `g4k create <rom> <kopie>` (Vorlage original), je Lauf eine frische.
- Antworten als Platzhalter: eine Änderung `[befehl, {argumente}, name]` bindet die Antwort des Befehls an `name`; spätere Änderungen, `check` und `play` nutzen sie als `${name}`, `${name.feld.0}` (Felder und Listenindizes) oder `${name.id+2999}` (Zahl plus Versatz). `{$base: "${hdr.Pt}", wild_pokemon: …}` ist das genannte Objekt mit ersetzten Feldern (für `set_header`). Jede benannte Änderung beendet eine Brücken-Sitzung: der Lauf speichert, liest die Antwort und öffnet das Projekt für die nächsten Änderungen neu (`edits.json`, `edits-2.json` …, Antworten in `<out>/results.json`).
- Erwartungen (seit 3b.4a): das dritte Element einer Änderung darf statt eines Namens `{expect: {path, equals}}` sein, mehrere als Liste `{expect: [{path, equals}, …]}`, mit `as: name` bindet es die Antwort zusätzlich. Die Antwort des Befehls muss an jedem `path` den Wert `equals` haben (JSON-Gleichheit: `1` ist nicht `"1"` und nicht `true`; in `equals` gehen Platzhalter wie in den Argumenten). Pfade: Schlüssel mit Punkt getrennt (`remaining`, `findings.0.code`), Listenindizes (`0.state.kind`, auch am Anfang), `.length` an einer Liste und Filter `[schlüssel=wert]`, die die Listeneinträge behalten, deren Feld als Text gleich `wert` ist (`findings[code=warp.dest-warp].length`, `findings[severity=error].length`; der Wert darf Punkte enthalten, Zahlen und `true`/`false`/`null` werden als Text verglichen). Ein Filter auf einen Schlüssel, den kein Eintrag einer nicht leeren Liste hat (`findings[cdoe=…]`), gilt als fehlender Pfad; ein falsch geschriebener Wert (`findings[code=warp.dest-wrap]`) fällt dagegen nicht auf, er findet nur nichts: eine Erwartung `… .length = 0` mit Tippfehler im Wert besteht, deshalb Codes aus dem Katalog kopieren. Auf stderr steht je erfüllter Erwartung „Erwartung erfüllt: run_checks findings[code=warp.dest-warp].length = 1“; eine verfehlte beendet das Szenario mit Rückgabe 2 und „Erwartung verfehlt: run_checks findings[severity=error].length: Soll 0, Ist 1“, ein fehlender Pfad mit „Ist fehlt (findings.7)“. Wie eine benannte Änderung beendet eine Änderung mit Erwartung die Brücken-Sitzung (gespeichert, die nächste öffnet neu). Ein falsch geschriebener Pfad oder eine Erwartung, die weder `path` und `equals` noch `error` (unten) hat, lehnt das Szenario beim Laden ab.
- Erwartungen an eine Liste von Antworten gehen auch mit einem Filter am Anfang: `[part=model].0.state.kind` nimmt aus der Antwort von `inspect_map_import` den Teil `model` (die Reihenfolge der Teile spielt dann keine Rolle). Zahlen in einem Bericht zählen nie als „sauber“, wenn `groups` nicht alle drei Gruppen hat: deshalb prüft jede `run_checks`-Erwartung der Gesamtgate-Szenarien auch `groups.length = 3`. Eine Befundliste, aus der ein Fix seinen Befund nimmt, wird im selben Lauf mit `findings.0.code` geprüft und mit `as:` gebunden (`[apply_fix, {finding: "${r_text.findings.0}", fix: wrap}, {expect: {path: remaining, equals: false}}]`).
- Erwartete Ablehnung (seit 3b.4b): Eine Änderung darf abgelehnt werden müssen: `{expect: {error: Teilstring}}` als drittes Element, allein (ohne `path`/`equals`, ohne `as`). Die Brücke muss den Befehl mit einer Fehlermeldung ablehnen, die den Teilstring enthält (Groß- und Kleinschreibung egal, nur die Meldung, nicht der Code); dann steht auf stderr „Erwartung erfüllt: import_map_parts abgelehnt: "<Meldung>"“ und das Szenario läuft weiter. Eine erfolgreiche Antwort beendet es mit Rückgabe 2 und „… Ablehnung erwartet ("…"), der Befehl war erfolgreich“, eine andere Meldung mit „…, Ist "…"“. Ein Brücken-Batch hält am ersten Fehler: der Lauf speichert deshalb vorher, was die Änderungen davor getan haben (gleiche Sitzung), die Ablehnung beendet die Sitzung ohne Speichern, die nächste öffnet das Projekt neu von der Platte. Kurz vor „Erwartung erfüllt“ druckt die Brücke selbst „import_map_parts fehlgeschlagen: …“; das ist die erwartete Ablehnung, kein Fehler. Den Teilstring aus der echten Meldung nehmen und so wählen, dass er Umformulierungen übersteht (die Szenarien nehmen „61440“).
- Befehl ohne Argumente und `dirty`: Ein Befehl ohne Argumente steht mit `{}` (`[dirty, {}, {expect: {path: length, equals: 0}}]`); `dirty` antwortet mit der Liste der ungespeicherten Dateien und sieht nur die Änderungen der eigenen Sitzung (seit der letzten Änderung mit Namen oder Erwartung). Nach einer Ablehnung beginnt eine frische Sitzung: dort ist `dirty` immer leer, es belegt nicht, dass die abgelehnte Änderung nichts verändert hat (das belegen die Kernel-Tests, siehe [dev-setup.md](dev-setup.md), „Kartenteile importieren (3b.4b)“).
- Dateihash (seit 3b.4b): `${sha1-of:PFAD}` steht für den SHA-1 (40 Hex-Zeichen) einer Datei; PFAD ist absolut oder relativ zur Kopie und darf selbst Platzhalter enthalten (`${sha1-of:${KOPIE}/export/karte6.nsbmd}`). Gelesen wird erst beim Einsetzen: in den Argumenten einer Änderung, nachdem die Änderungen davor gelaufen und gespeichert sind (sie bekommen dafür eine eigene Sitzung), in `equals` nach der Änderung, in `check:` und `play:` nach dem Bauen. Er gilt überall, wo ein `${name}` gilt, außer in `build` und `prepare`; ein unvollständiges `${sha1-of:` lehnt das Szenario beim Laden ab. Fehlt die Datei, endet es mit Rückgabe 1 und „Platzhalter nicht auflösbar in Änderung 5 (befehl): …: Datei fehlt oder ist nicht lesbar: <Pfad>“ (Schritt: `Änderung N (befehl)`, `check N` oder `play`).
- Abschnitt `prepare:` (seit 3b.4b, optional, vor `edits`): Liste von `[schritt, {argumente}]`, ausgeführt nach der Prüfung der Kopie und vor der ersten Brücken-Sitzung. Der einzige Schritt `make-oversize-nsbmd {path, bytes?}` schreibt ein synthetisches NSBMD (Kopf `BMD0`, `FFFE`, Version 2, Größenfeld = Länge, ein MDL0-Block mit einem Modell, sonst Nullen) von `bytes` Byte (Standard 65536, muss über 61440 = 0xF000 liegen), den Gegenfall „Modell zu groß“; stderr „prepare: make-oversize-nsbmd <Pfad> (65536 Byte)“. `path` ist relativ zur Kopie oder absolut und muss in der Kopie liegen (sonst Rückgabe 1, es wird nichts geschrieben); fehlende Ordner (`export/`) werden angelegt, eine vorhandene Datei ersetzt. `--set`-Platzhalter gehen, Namen aus Antworten und `${sha1-of:…}` nicht. Von Hand dieselbe Datei (es gibt keinen eigenen Befehl dafür), aus `scripts/g4emu`: `uv run python -c "from g4emu import fixtures; fixtures.make_oversize_nsbmd('<Kopie>/export/zu-gross.nsbmd')"`.
- `play:`-Einträge teilt der Lauf beim Laden wie eine Shell an Leerzeichen (`shlex`). Die `--set`-Werte setzt er **vor** dem Teilen ein: ein `--set`-Wert mit Leerzeichen zerfällt sonst in mehrere Wörter und gehört im Eintrag in Anführungszeichen (`"${KOPIE}/weg.png"`, auch als Pfad in `"${sha1-of:${KOPIE}/export/karte6.nsbmd}"`), ebenso ein Pfad mit Leerzeichen, der wörtlich im Eintrag steht. Erst nach dem Teilen setzt er Namen aus Antworten (`${name…}`) und die Ergebnisse von `${sha1-of:…}` ein; nur die bleiben ein Wort.
- Abschnitt `check:` (optional, nach `build`): eine Liste von Befehlen `[g4k, …]`, Platzhalter werden ersetzt. Sie laufen nach dem Bauen im Ordner der Kopie (`out.nds` ist dort das gebaute ROM) und vor dem Spielen; die Ausgabe steht in `<out>/check.log`. Der erste Befehl mit Exit ≠ 0 lässt das Szenario mit Rückgabe 1 scheitern („Prüfschritt fehlgeschlagen …“), gespielt wird dann nicht. `check` ohne `build` lehnt das Szenario ab.
- `g4k check-data <rom.nds> --expect KIND:MEMBER:FIELD=VALUE …` liest die Daten-NARCs aus dem gebauten ROM (Pfade aus `archives.yaml` für den Spielcode des ROMs) und vergleicht: `species` und `item` liest uxie, `move` und `learnset` die eigenen Codecs (uxie lehnt 172 von 471 Vanilla-Attacken ab); MEMBER ist der Member im Archiv (bei `item` der Datenmember, nicht die Item-ID), ein Lernset hat das Feld `len`; `encounter` liest eine Begegnungsdatei mit den Feldern `grass_rate`, `grassN` (Art in Gras-Slot N, 0 bis 11; HeartGold: Tag-Spalte), `grassN_level` und in HeartGold `grassN_morning`, `grassN_night` (Beispiel `encounter:140:grass0=150`). Treffer stehen auf stdout („species 397 attack: 85“, am Ende „N expectation(s) met“), Abweichungen auf stderr („species 397 attack: expected 75, found 85“), dann Exit 1; ein falsch geschriebenes `--expect` (unbekannte Art oder unbekanntes Feld) endet vor dem Lesen mit Exit 2 und nennt die gültigen Felder.
- `g4k check <projekt> [--json] [--expect CODE=N …]` führt alle Prüfungen auf dem gespeicherten Projekt aus und liest `<projekt>/g4-checks.json` (ausgeblendete Befunde zählen als „ignored“). `--expect warp.dest-warp=0` vergleicht die Zahl der nicht ausgeblendeten Befunde des Codes: Treffer auf stdout („warp.dest-warp: 0“, am Ende „N expectation(s) met“), Abweichungen auf stderr („text.too-wide: expected 1, found 0“) und Exit 1; ein unbekannter Code endet vor dem Lauf mit Exit 2. Es braucht kein gebautes ROM, läuft also auch ohne `build` von Hand nach einem Szenario: `target/debug/g4k check <Kopie> --expect warp.dest-warp=0 --expect text.too-wide=0`.
- Die Kopie muss direkt unter `_testkopien` liegen. Das Szenario baut `out.nds` in die Kopie und braucht dafür `cargo`; es arbeitet auf dem Spielstand aus `games.yaml`. Die Werkzeuge baut es selbst (`studio-bridge`, für `check:` auch `g4k` mit `cargo build --workspace --all-features --bin g4k`); mit `G4_BRIDGE_BIN` und `G4K_BIN` nimmt es fertige Binaries. In einem Worktree scheitert `cargo build --bin g4k` am Download in rotoms Build-Skript (der Bau ist ein anderer als der der Tests): vorher `cargo build --workspace --all-features` (legt `target/debug/g4k` und `target/debug/studio-bridge` an; `cargo test --workspace --all-features --no-run` erneuert `g4k`, aber nicht immer `studio-bridge`, das Datum der Datei prüfen) und beide Variablen auf diese Dateien setzen. Scheitert das Szenario erst nach `build_rom`, ist die Kopie schon gespeichert und gebaut: für den nächsten Lauf eine neue Kopie anlegen.

### Szenarien über MCP (seit 3h.2)

Ein Szenario mit dem Schlüssel `mcp:` läuft nicht über die Brücke, sondern über **eine** Sitzung von `studio mcp` (das Produkt-Binary, siehe [mcp.md](mcp.md)): der Client in `scripts/g4emu/g4emu/mcp.py` startet `studio mcp --project <Kopie> <Schalter>`, spricht JSON-RPC über stdio und ruft die Werkzeuge nacheinander auf. Eine Sitzung genügt, weil MCP den Zustand hält (die Brücke öffnet nach jeder benannten Änderung neu).

- `mcp: [<Schalter>]`: die Schalter, die an `studio mcp` gehen, etwa `mcp: [--allow-irreversible, --expert]` (Patch, neue Scriptdatei und Trainer sind unumkehrbar, die Befehle ohne kuratiertes Werkzeug gehen über `g4_call`, also mit `--expert`). `mcp: []` heißt ohne Schalter (lesen und bearbeiten). `--project` setzt g4emu selbst; es im Szenario zu nennen lehnt das Laden ab.
- `edits:` sind Werkzeugaufrufe `[werkzeug, {argumente}, name]` mit den **Namen der MCP-Werkzeuge** (`create_area`, `data_edit`, `g4_batch` …, Argumente in snake_case wie in [mcp.md](mcp.md)); was kein kuratiertes Werkzeug hat, geht als `[g4_call, {command: …, args: {…}}]`. Das dritte Element bindet die Antwort (`structuredContent.result`) an `name` und nimmt Erwartungen `{expect: …}` wie auf der Brücke, auch `{expect: {error: Teilstring}}` für ein Werkzeug, das ablehnen muss (`error.message`, ohne Beachtung der Schreibweise). Ein Werkzeug-Fehler ohne erwartete Ablehnung beendet den Lauf mit Rückgabe 1, ein Protokollfehler oder ein toter Server immer.
- Am Ende ruft der Lauf selbst `save {}` und, mit `build:`, `build_rom {out: <build>}`. `build:` muss bei `mcp` ein relativer Pfad **unter `build/`** sein (`build: build/out.nds`), weil `UserPolicy` Build-Ziele nur dort ohne `--allow-irreversible` zulässt; `build/out.nds` liegt in der Kopie, `check:` und `play:` nennen es mit diesem Pfad (`g4k check-data build/out.nds …`).
- Protokolle in `--out`: `mcp.jsonl` (eine Zeile je Aufruf, `{tool, arguments, ok, result|error}`), `mcp-stderr.log` (stderr des Servers; steht darin, warum er nicht startete oder starb) und, wie sonst, `results.json`, `check.log`, die Bilder und die Zusammenfassung. `bridge.jsonl` gibt es bei `mcp` nicht.
- Das Programm: `G4_STUDIO_BIN` (Pfad der Programmdatei `studio`), sonst baut der Lauf `cargo build -p studio --bin studio` und nimmt `target/debug/studio`. In einem Worktree vorher `cargo build --workspace --all-features` und beide Variablen setzen (`G4_STUDIO_BIN`, `G4K_BIN`, siehe „Szenario“ oben). `G4_BRIDGE_BIN` braucht ein `mcp:`-Szenario nicht.
- Aufruf (aus `scripts/g4emu`, `R` ist der absolute `G4_ROMS_DIR`, die Kopie frisch aus einer Kopie des Originals):

```sh
R=<absoluter Pfad zu Roms>; O=../../target/agent/gate-$(date +%F)
export G4_STUDIO_BIN=$PWD/../../target/debug/studio G4K_BIN=$PWD/../../target/debug/g4k
uv run g4emu scenario ../agent/scenarios/pt-mcp-route.yaml --project $R/_testkopien/3h-pt-route --out $O/pt-mcp-route \
  --set NEUER_HEADER=593 --set S_WILD=150 --set L_WILD=5 --set S_T1=25 --set L_T1=12 \
  --set S_T2=133 --set L_T2=14 --set S_GIFT=445
uv run g4emu scenario ../agent/scenarios/hg-mcp-route.yaml --project $R/_testkopien/3h-hg-route --out $O/hg-mcp-route \
  --set NEUER_HEADER=540 --set S_WILD=150 --set L_WILD=5 --set S_T1=25 --set L_T1=12 \
  --set S_T2=133 --set L_T2=14 --set S_GIFT=445
```

Bei Rückgabe 1 oder 2 zuerst `mcp.jsonl` (letzte Zeile) und `mcp-stderr.log` lesen. Die Kopie ist nach einem Fehler nach `build_rom` schon gespeichert; für den nächsten Lauf eine neue anlegen.

`pt-erzaehlen.yaml` und `hg-erzaehlen.yaml` (Gate 3d.5, `mcp: [--allow-irreversible, --expert]`): neues Gebiet wie bei den Trainer-Szenarien mit neuer Scriptdatei und neuem Textarchiv, dann `recipe_apply` mit `new_figure`, `connect_doors` (Treppe des Zimmers ↔ neue Tür, Türkacheln) und `new_trainer_here`; spielen: Treppe, Figur ansprechen (`shot figur` zeigt „Hallo, Reisender!“), durch die Tür zurück und wieder hinunter, vom Trainer gesehen (`expect-battle`, ohne Team im Spielstand verloren mit `end-battle`). Erwartet `"ok": true`, Header-Folge 415, 593, 415, 593 bzw. 64, 540, 64, 540:

```sh
uv run g4emu scenario ../agent/scenarios/pt-erzaehlen.yaml --project $R/_testkopien/3d-5-szen-pt_DSPRE_contents --out $O/pt-erzaehlen \
  --set NEUER_HEADER=593 --set S_T1=25 --set L_T1=12 --set TEXT="Hallo, Reisender!"
uv run g4emu scenario ../agent/scenarios/hg-erzaehlen.yaml --project $R/_testkopien/3d-5-szen-hg_DSPRE_contents --out $O/hg-erzaehlen \
  --set NEUER_HEADER=540 --set S_T1=25 --set L_T1=12 --set TEXT="Hallo, Reisender!"
```

### MCP über die Brücke (`/mcp`, seit 3h.3)

Die Brücke der Test-Umgebung bedient MCP auf ihrem eigenen Port unter `/mcp`, auf **derselben Sitzung** wie `/invoke` und die Seite: Was ein Agent dort tut, sieht die Oberfläche im Browser-Fenster sofort (Welt-Explorer, Szene, Verlauf, Statusleiste), und die Nachfrage erscheint dort als Dialog. Das ist der Browser-Modus von Einstellungen › Agenten; die Zeile des Servers heißt dann „Über die Brücke (/mcp)“. Eine Entdeckungsdatei schreibt die Brücke nicht: Die Einrichtungszeile der Seite (`claude mcp add gen4-studio -- "…/target/debug/studio" mcp`) zeigt nur, was die gepackte App zeigen würde; `studio mcp` erreicht die Brücke nicht. Kunde der Test-Umgebung ist `scripts/agent/mcp.sh`.

**Schalter.** Solange „Lokale Agenten zulassen“ aus ist, antwortet `/mcp` mit 503 „Lokale Agenten sind aus (Einstellungen › Agenten)“ (`mcp.sh` druckt „HTTP 503: …“, Rückgabe 2). Einschalten im Browser-Fenster (Einstellungen › Agenten, `Switch` in `[data-agents-enabled]`) oder mit `call.sh`:

```sh
scripts/agent/call.sh agent_set_enabled '{"on":true}'        # Schalter „Lokale Agenten zulassen“
scripts/agent/call.sh agent_status                           # enabled, expert, read_only, port, server_error, bridge, connections, question
scripts/agent/call.sh agent_settings                         # Einstellungen, Einrichtungszeile, Freigabe des offenen Projekts
scripts/agent/call.sh agent_set_grant '{"level":"read"}'     # Freigabe des offenen Projekts: none, read, write
scripts/agent/call.sh agent_set_expert '{"on":true}'         # Expertenwerkzeuge (g4_call)
scripts/agent/call.sh agent_set_read_only '{"on":true}'      # „Nur lesen“ der Statusleiste
scripts/agent/call.sh agent_disconnect                       # „Trennen“
scripts/agent/call.sh agent_answer '{"question":7,"answer":"read"}'   # Nachfrage beantworten: deny, read, write, allow, dismiss
```

Diese Befehle gibt es nur für die Oberfläche (über `/invoke`); für einen Agenten über MCP sind sie `unavailable`. Für das Gate gilt: Die Nachfrage beantwortet der Agent im Browser-Fenster mit einem Klick auf die Taste des Dialogs `[data-agent-question]`, damit der Weg des Nutzers geprüft ist; `agent_answer` ist für Fälle ohne Fenster.

**`mcp.sh`.** Ein kleiner Client aus der Python-Standardbibliothek (`scripts/agent/mcp_client.py`), Port und Token wie `call.sh` (`target/agent/bridge.port`, `bridge.token`), Protokollversion 2025-11-25, Client-Name `agent-gate` (so heißt er in Statusleiste, Dialog und Verlauf):

```sh
scripts/agent/mcp.sh tools                                   # Namen der Werkzeuge, einer je Zeile
scripts/agent/mcp.sh call get_header '{"id":0}'              # structuredContent als JSON auf stdout
scripts/agent/mcp.sh call create_area '{"spec":{…}}'         # Argumente in snake_case wie in docs/mcp.md
scripts/agent/mcp.sh read g4://glossary                      # Text einer Ressource
scripts/agent/mcp.sh close                                   # beendet die Sitzung
```

Rückgabe 0 bei Erfolg, 1, wenn das Werkzeug ablehnt (sein `structuredContent` mit `code` und `message` auf stderr, etwa `read_only`, `not_permitted`, `no_project`), 2 bei Aufruffehlern, Protokoll- und HTTP-Fehlern (Meldung auf stderr). Die Sitzung steht in `target/agent/mcp.session`; kennt die Brücke sie nicht mehr (HTTP 404, nach einem Neustart der Brücke oder nach „Trennen“), verbindet `mcp.sh` einmal neu und wiederholt den Aufruf. Ein Aufruf wartet bis zu 600 s (eine Nachfrage wartet bis 120 s, `build_rom` dauert): `mcp.sh call` deshalb im Hintergrund starten, wenn danach im Browser-Fenster geklickt werden muss, etwa `scripts/agent/mcp.sh call get_header '{"id":0}' > target/agent/mcp-call.out 2>&1 &`. Die Brücke bedient höchstens 16 `/mcp`-Aufrufe gleichzeitig, jeden auf einem eigenen Faden, damit `/invoke` frei bleibt.

**Freigaben bleiben liegen.** `studio-web.sh` startet die Brücke mit `--app-data target/agent/app-data`: Schalter, Expertenwerkzeuge und Freigaben stehen in `target/agent/app-data/agents.json` (0600) und überstehen Neustarts der Brücke und neue Läufe. Eine Freigabe gilt für kanonischen Pfad und `base_id` der Kopie; eine neue Kopie unter neuem Namen fragt also wieder. Für einen Lauf von vorn (Schalter aus, keine Freigaben) die Umgebung stoppen und den Ordner löschen: `rm -r target/agent/app-data` (das Skript legt ihn beim nächsten Start neu an). Einzelne Freigaben entfernt „Freigabe entfernen“ in Einstellungen › Agenten. „Nur lesen“ wird nie gespeichert.

## Hinweise aus dem Handdurchlauf

- Warps feuern nur auf Treppen- und Warp-Kacheln und auf Türen (Verhalten 0x69) mit Kollision 0x80. Einen Warp auf eine „freie“ Kachel zu setzen verbindet nichts; im Spiel passiert dort nichts. Das Anlegen eines Warps (Werkzeug „Tür“, auch über „+“ › „Tür“ in „In dieser Szene“) fragt deshalb, ob die Kachel zur Tür werden soll, und der Raum-Assistent macht beide Tür-Kacheln zu Türen, wenn nötig.
- Ein neuer Header mit Eventdatei „neu, leer“ braucht einen Ankunfts-Warp (Index 0, `add_event`/Werkzeug „Tür“). Ohne ihn zeigt der Ziel-Warp 0 ins Leere, und das Spiel zeigt nach dem Übergang schwarz.
- Der Gebiets-Assistent nimmt als Vorlage den Header, für den er geöffnet wurde, sonst den der ersten gewählten Zelle, die keine Füllfläche (Header 0) ist. Beim Prüfen von Schritt Header steht die Vorlage in der Zeile „Kopie von Header …“.
- Der von DSPRE übernommene Patch „Dynamische Header“ war für Platin DE/FR/IT/ES fehlerhaft; „Patch reparieren“ und die Korrektur sind in [formats/dynamic_headers.md](formats/dynamic_headers.md) beschrieben. Die Anleitung prüft diese Befunde nicht neu.
- `Cmd+S` im Browser-Fenster kann die Seite statt der App meinen. Tritt das auf, „Speichern“ in der Leiste klicken oder `call.sh save` benutzen.
- Hover und Maus: Das 3D-Fenster lässt sich mit `hover` bedienen. Klappt Ziehen mit der Maus im Browser-Fenster nicht, prüft der Agent die Wirkung über `update_event` per `call.sh` und führt den Punkt als „Agent + Blick“.
- Szenario-Läufe und Gate-Läufe hinterlassen Kopien unter `_testkopien` (zum Beispiel `pt-agent-szenario2`); der Nutzer räumt sie auf.

## Phase 3a ([phase-3a-gate.md](phase-3a-gate.md))

Kopie `pt-3a` (Platin DE), für (D) 4 zusätzlich `hg-3a` (HeartGold US), für die Fehlerfälle aus (F) eine eigene `pt-3a-f`. Die Gate-Datei schreibt die gebündelte App vor; im Browser-Modus ersetzt `studio-web` sie, die Abweichung kommt in „Auffälligkeiten“.

| Punkt | Wer | Wie |
|---|---|---|
| (A) 1–4 Gebiet anlegen (Route 230) | Agent | Browser: Header 0342 wählen, „Neu anlegen“ (+ in der Liste „Orte“) → „Gebiet anlegen…“, Zellen abwählen bis zwei übrig sind, Name „Route 230“, Schritt Header: 0001 („NOTHING“), „Anlegen“, „Zeigen“ |
| (A) 5–10 Raum anlegen (Spielerzimmer als neuer Header 0593) | Agent | Browser: Maus (`hover`) in der 3D-Ansicht von 0411 auf die Kachel vor dem Rivalenhaus, „Neu anlegen“ (+ in der Liste „Orte“) → „Raum anlegen…“, Vorlage Header 0415, „Neuer Header“ (vorher „Patch anwenden“), Eventdatei „Kopie von …“ ausdrücklich wählen, Eingang Außen-Header 0411, „Kacheln, auf denen ein Warp nicht auslöst, zur Tür machen (Verhalten 0x69, Kollision 0x80)“ an lassen, „Anlegen“. Nicht Header 0002 „UG“ (Sonder-Header, bleibt im Spiel schwarz). Dann in „In dieser Szene“ bei 0411 die Gruppe „Türen“ ansehen (eine Tür mit der zweiten Zeile „→ <Name des neuen Raums>“, in Tüfteln „→ Header 0593 · <Name>“; ein Klick auf die Tür zeigt im Inspector das Ziel „0593“, Doppelklick oder Enter folgt ihr); Gegenprobe `call.sh get_events` der neuen Eventdatei: genau ein Warp auf der Rückweg-Kachel, Ziel 0411 |
| (A) Stoppuhr und „Zahlen getippt“ | Hand | Die Zeit misst der Mensch bei eigener Bedienung. Der Agent trägt nur seine getippten Zahlen ein und sagt, dass seine Zeit nichts aussagt |
| (A) 11 Warps von Hand verbinden | Agent | Browser, Schritte in Rezept „Warps verbinden“ unten |
| (A) 12 Gang im Spiel | Agent | „Bauen“, dann `g4emu`, Schritte in Rezept „Gang im Spiel“ unten; Rückgabe 0 und die Header-Folge sind das Ergebnis, `shot` an jeder Station |
| (B) 1–4 Explorer | Agent | Browser: Suche „Zweiblatt“ in der Liste „Orte“, Umschalter „Baum / Liste“, Hilfe-Dialog (Atlas „Hilfe“, früher „Begriffe“), „Ohne Ortsnamen“; Texte im DOM lesen (`find`, `get_page_text`) |
| (C) 1–4 Raster | Agent | Browser: Tab „Raster“, Header 0411, 0003, Route 230; die „+“-Zelle öffnet den Dialog „Karte bei x …, y … anlegen“. Neue Karten, Raster und Eventdateien sind sofort auf der Platte; `call.sh member_counts` zeigt die Zahlen |
| (D) 1–3 Weltkarte | Agent | Browser: Tab „Weltkarte“, Zellenklick, Zoomen mit dem Mausrad, Linien bei Zweiblattdorf, nach (A) der neue Raum |
| (D) 4 HeartGold US | Agent | frische Kopie `hg-3a`, `?project=…`; Baum und Weltkarte laden, New Bark Town = Header 0060 |
| (E) 1–2 Automatik | Agent | `scripts/gate.sh` (rund 25 Minuten, im Hintergrund mit `> target/agent/gate.log` und Tabelle am Ende übernehmen) |
| (F) Kartenbilder: Ausrichtung | Agent + Blick | Tab „Raster“ und 3D-Ansicht von oben bei Zweiblattdorf nacheinander ansehen, Norden oben, nicht gespiegelt; der Nutzer bestätigt |
| (F) Kartenbilder ohne apicula | Agent | laufende App beenden (Ports sind belegt), dann `G4_APICULA=/nirgends bash scripts/agent/studio-web.sh > target/agent/studio-web.log 2>&1 & echo $! > target/agent/studio-web.pid`, Seite öffnen: Platzhalter und Tooltip lesen; danach beenden und normal neu starten |
| (F) Layout im schmalen Panel | Agent | Browser-Fenster so verkleinern (`resize_window`) oder den Teiler ziehen, dass die Liste „Orte“ schmal ist (Atlas auf Kompakt); Listenansicht, Hilfe-Dialog (Atlas „Hilfe“, früher „Begriffe“) und „Neu anlegen“ (+ in der Liste „Orte“) ansehen, Escape und Klick daneben |
| (F) Tastatur während eines Assistenten | Agent | Assistent öffnen, Escape, Backspace, Cmd+Z, Cmd+S drücken; danach `call.sh dirty` (leer = nichts verändert) |
| (F) Raum-Assistent, Fehlerfälle | Agent | Browser, Schritte in Rezept „Raum-Assistent und Fehlerfälle“ unten (Raster-Wechsel, „Raum hier anbinden…“, Kachel x 999, gesperrte Eventdatei, später Fehler, Cmd+Z) |
| (F) Layout nach Neustart | Agent | Panel verschieben, Seite neu laden (das Layout liegt im `localStorage`) |
| Gebündelte App statt `tauri dev` | Hand | kurzer Durchgang im nativen Fenster am Ende (`pnpm bundle`, siehe [release.md](release.md)) |

## Phase 3b.1 ([phase-3b1-gate.md](phase-3b1-gate.md))

Kopien `pt-3b1` (Platin DE, **nicht** vorher mit DSPRE gepatcht) und `hg-3b1` (HeartGold DE). Für (A) 18 eine eigene dritte Kopie `pt-3b1-rep`: `pt-3b1` bleibt unversehrt, sie ist das Gegenbeispiel „ein intaktes Projekt zeigt die Meldung nicht“. In `pt-3b1` und in Originalen werden nie Bytes von Hand geändert.

| Punkt | Wer | Wie |
|---|---|---|
| (A) 1 Ausgangslage | Agent | `ls -l <Kopie>/files/fielddata/maptable/mapname.bin` (Größe notieren), `ls <Kopie>/arm9/arm9.bin.backup` muss fehlen; Zahl der Header: `call.sh list_headers` oder letzte ID im Explorer plus eins |
| (A) 2–3 Assistent bis Patch | Agent | Browser: Header 0342, „Neu anlegen“ (+ in der Liste „Orte“) → „Gebiet anlegen…“, zwei Zellen übrig, Name „Route 231“, Schritt Header: „Neuer Header“ ist gesperrt, Kasten „Patch „Dynamische Header“ nötig“ lesen, vorher speichern, „Patch anwenden“ |
| (A) 4 Platte nach dem Patch | Agent | `cmp <Kopie>/arm9/arm9.bin <Kopie>/arm9/arm9.bin.backup` meldet Unterschiede; `ls <Kopie>/unpacked/dynamicHeaders \| wc -l` gleich Zahl der Header; `ls -l <Kopie>/arm9/arm9.bin*` |
| (A) 5–6 Neuer Header | Agent | Browser: „Neuer Header“, „Interner Name“ auf `R231`, Zusammenfassung lesen, „Anlegen“, „Zeigen“ |
| (A) 7 Platte nach dem Anlegen | Agent | `ls -l <Kopie>/unpacked/dynamicHeaders/NNNN` (NNNN vierstellig, 24 Byte; in JSON ohne führende Nullen, zum Beispiel `{"id":593}`); `mapname.bin` genau 16 Byte länger; `tail -c 16 <Kopie>/files/fielddata/maptable/mapname.bin \| xxd` beginnt mit `R231` |
| (A) 8 Variante „behalten“ | Agent | Browser: zweites Gebiet „Route 232“, Eventdatei „behalten“; Gegenprobe `call.sh get_header '{"id":NNNN}'` (NNNN ohne führende Nullen; gleiche `event_file_id` wie 0342) |
| (A) 9–10 Warp und Gang im Spiel | Agent | Warps in der Oberfläche setzen (siehe 3a (A) 11) und mit `g4emu` laufen, **oder** das Szenario `pt-neuer-header.yaml` auf einer weiteren frischen Kopie (Patch, neuer Header, Treppe, hin und zurück ohne Klick) |
| (A) 11 NARC im ROM | Agent | `target/debug/g4k extract <Kopie>/out.nds target/agent/gate-<datum>/rom-extrakt` und prüfen, dass `…/files/debug/cb_edit/d_test.narc` existiert (`g4k` steht nach `cargo build -p g4-kernel` bereit; der Zielordner darf nicht unter `../Roms` liegen) |
| (A) 12–15 DSPRE-Gegenprobe | Hand | DSPRE läuft nur unter Windows |
| (A) 16 HeartGold DE | Agent | wie (A) 2–10 auf `hg-3b1`, Interner Name `HG_TEST`, Spielstand IPKD (Zimmer Header 64, Treppe nach 63); nach dem Bauen `target/debug/g4k extract <Kopie>/out.nds target/agent/gate-<datum>/rom-hg` und `ls …/rom-hg/files/a/0/5/0` (im ROM, nicht im Projektordner) |
| (A) 17 Diamant/Perl | Hand | entfällt ohne Projekt |
| (A) 18 Patch reparieren | Agent | Auf der **eigenen** frischen Kopie `pt-3b1-rep` (nie `pt-3b1`, nie ein Original), Schritte in Rezept „Patch reparieren“ unten, über Technik › Patches. Dass DSPRE selbst so patcht, gehört zur DSPRE-Gegenprobe (Hand) |
| (B) 1–5 Höhen | Agent + Blick | Browser: Tab „Raster“, „Höhen“ einschalten, Zelle anklicken, Inspector „Höhe“ = 2; Hover und Ziehproben mit `hover` und `left_click_drag`, Messwerte über `get_matrix`; wie die erhöhte Zelle aussieht bestätigt der Nutzer am Bild |
| (C) 1–7 Sprites | Agent + Blick | Agent prüft: Sprite statt gelbem Quadrat, Wechsel des Feldes „Richtung“ (0 bis 3) ohne Neuladen, Platzhalter bei „Grafik“ 60000 (danach Cmd+Z). Aufrecht, nicht gespiegelt und Blickrichtung bestätigt der Nutzer am Bild |
| (D) 1–3 Kachelnamen HGSS | Agent | Browser: Hover auf Bäumen (`HEADBUTT`), Gras, Wasser, Hüpfkante; Platin: derselbe Wert 0x06 ohne `HEADBUTT`; Namen mit `call.sh tile_behaviors` gegenprüfen |
| (E) 1–2 `gate.sh` | Agent | `scripts/gate.sh`, Tabelle in die Ergebnisse übernehmen; keine Zeile „skipped“ außer den dokumentierten |
| (F) 1–3 apicula-Zeitlimit | Agent | `G4_APICULA_TIMEOUT_SECS=1 bash scripts/agent/studio-web.sh …` mit frischer Kopie ohne `.g4/models`; `ls <Kopie>/.g4/models` (keine `.fail`, `last-tool` mit 16 Hexzeichen) |

## Phase 3b.2 im Browser-Modus ([phase-3b2-gate.md](phase-3b2-gate.md))

Die Brücke kennt alle Befehle von Scripts und Texten: `get_text_archive`, `edit_text_archive`, `create_text_archive`, `search_text`, `validate_text`, `scripts_status`, `adopt_scripts`, `list_script_files`, `read_script`, `save_script`, `save_level_script`, `new_script_file`, `add_script`, `resolve_script`, `search_scripts` (per `call.sh` wie die übrigen, Argumente wie in `api.ts`). Sie schreiben nur in das geöffnete Projekt, und das liegt unter `_testkopien`; ein Original oder ein Ordner außerhalb wird schon beim Öffnen abgelehnt. „Scripts übernehmen“ (`adopt_scripts`) läuft ohne Fortschrittsbalken: Es gibt über HTTP keinen Kanal, die Seite meldet erst am Ende einmal „fertig“ und der Aufruf kehrt nach einigen Sekunden mit dem Bericht zurück. Die Sprachdienste (`lsp_start`, `lsp_send`, `lsp_stop`) sind im Browser aus: `lsp_start` meldet „rotom-lsp fehlt — Completion und Diagnosen aus“ (Code `tool_missing`), der Editor läuft wie ohne rotom-lsp, also ohne Completion und Diagnosen. Punkte des 3b.2-Gates, die Completion oder Diagnosen aus rotom-lsp verlangen, bleiben beim Nutzer im gebündelten Fenster (Wer: Hand).

Die Gate-Datei 3b.2 nennt die Namen von vor 3c. Seit 3c gilt: Tab „Script“ ist der Reiter „Script“ der Welt-Hand, Panel „Probleme“ der Reiter „Probleme“ daneben (das Prüflicht im Scharnier öffnet ihn), das Panel „Text“ ist der Bereich Texte (Atlas „Texte“), die „Toolbar“ ist die Titelleiste, der Knopf „Undo“ ist ⌘Z oder das Verlauf-Menü im Scharnier. Kopien `pt-3b2` und `hg-3b2` (für einen Lauf in 3c mit der Endung `-3c`).

| Punkt | Wer | Wie |
|---|---|---|
| (A) 1–6 Scripts übernehmen und Rebuild | Agent | Browser: Welt, Reiter „Script“, Kasten „Scripts übernehmen“, „Übernehmen“ (ohne Fortschrittsbalken, siehe oben); Platte mit `ls` und `diff -r` prüfen; „Bauen“ in der Titelleiste; dasselbe auf `hg-3b2` |
| (B) 1–5 NPC mit Textliteral | Agent (Completion: Hand) | Browser: Header 0001 in der Liste „Orte“, im Inspector bei „Scriptdatei“ „Neu“, Header speichern („Speichern“ in der Titelleiste); ein Objekt mit dem Werkzeug „Figur“ (F) auf eine freie Kachel setzen (früher „Art des neuen Events“ und „Hinzufügen“), im Inspector bei „Script“ „Neues Script“, dann „Öffnen“: der Reiter „Script“ zeigt den Rumpf. Script einsetzen, speichern; die Nachricht steht im Bereich Texte am Ende von Archiv 0023 (`call.sh get_text_archive '{"index":23}'`). Bauen, `g4emu` |
| (C) 1–3 Flag und Item | Agent (Completion, Diagnose im Editor: Hand) | Browser: Script ersetzen, speichern; Tippfehler `AddItemm`: Prüflicht „1 Fehler“, Reiter „Probleme“ mit Quelle „Compile“, Klick auf die Datei springt zur Zeile; beheben. Bauen, `g4emu` |
| (D) 1–3 Vanilla-Dialog | Agent | Browser: Spielerhaus Erdgeschoss (Header 0414), die Mutter in „In dieser Szene“ (Gruppe „Figuren“) anklicken (früher in „An diesem Ort“, davor nur in der 3D-Ansicht), „Script“ → „Öffnen“; den Text im Bereich Texte ändern (die Referenz „Textarchiv“ im Header-Editor führt dorthin); „Speichern“ im Script-Kopf verweigert mit „Textarchiv NNNN erst speichern“, „Speichern“ in der Titelleiste speichert alles. Bauen, `g4emu` |
| (E) 1–5 Level-Script | Agent | Browser: Abschnitt „Level-Script“ im Header-Editor wie in der Gate-Datei; „Als JSON öffnen“ öffnet im Reiter „Script“. Bauen, `g4emu` |
| (F) 1–6 Texte | Agent | Browser, Bereich Texte: Archiv in der Liste „Textarchive“ wählen (die Header, die es nutzen, stehen in der Spalte „Nachricht bearbeiten“ unter „Verwendet in“), Nachricht in der Tabelle; „Text der Nachricht“ ändern (Überlängen-Hinweis unter dem Feld, Balken in „Längste Zeile“); „Nachricht anlegen“ (früher „Nachricht anfügen“) hängt eine leere Nachricht an; Suche „Texte suchen“ mit „Alle“; „+“ „Textarchiv anlegen“ (früher „Neues Archiv“) schreibt sofort auf die Platte. ⌘Z im Feld ist das Undo des Felds, außerhalb das der App |

## Rezepte

**Warps verbinden (3a (A) 11).** Warps feuern nur auf Tür-, Treppen- und Warp-Kacheln.
1. In 0411: `call.sh get_events '{"eventFileId":390}'` zeigt die Türen (Warp 0 bei 105, 875 führt in das Rivalenhaus). Werkzeug „Tür“ (T) wählen (oder „+“ › „Tür“ im Kopf von „In dieser Szene“) und auf die Kachel einer Tür klicken (früher der Knopf „Warp hinzufügen“, der mit dem Abschnitt „Eingänge / Ausgänge“ des Header-Editors seit 3e.3 fehlt), im Inspector Ziel Header 1, Ziel-Warp 0. Den Index des neuen Warps in der Gruppe „Türen“ von „In dieser Szene“ ablesen („Warp N“).
2. In 0001 (Route 230): Werkzeug „Tür“ (T), Klick auf die Zielkachel, Ziel Header 411 und der Index aus Schritt 1. Hat die Kachel kein Warp-Verhalten, passt `call.sh tile_behaviors` den Wert an und `set_permission` setzt ihn; gelingt das nicht, den Befund notieren.
3. Prüfen: `call.sh list_warps` nennt beide Warps.

**Gebäude setzen.** Werkzeug „Gebäude“ (G) wählen und auf eine Kachel der 3D-Ansicht klicken (früher der Knopf „Neues Gebäude“ der Szenenleiste); danach steht wieder „Auswählen“, und der Inspector zeigt das neue Gebäude. Gegenprobe: `call.sh dirty` nennt die Karte.

**Gang im Spiel (3a (A) 12).** Koordinaten von Warps (`x`, `z`) sind draußen gleich `x`, `y` von `where`; die Kachel vor einer Tür liegt eine südlich (Tür bei 105, 875: vorher 105, 876).
1. „Bauen“ (OK im Pfad-Dialog), Ergebnis `<Kopie>/out.nds`.
2. Zimmer, Haus, draußen, wieder hinein (aus `scripts/g4emu`):
   ```sh
   O=../../target/agent/gate-$(date +%F)/a12; R=../../../Roms/_testkopien
   uv run g4emu run --rom $R/<Kopie>/out.nds --save $R/_saves/CPUD-zimmer.sav --out $O \
     continue expect-map 415 walk-to 8 4 walk right 1 expect-map 414 wait 150 shot wohnzimmer
   ```
   Weiter mit `--state $O/last.state` (dasselbe `--rom`, `--out $O`), zum Beispiel `walk-to 6 10 walk down 1 expect-map 411 wait 100 shot draussen walk-to 105 876 walk up 1 expect-map 412` (Tür des Rivalenhauses; Tür des eigenen Hauses nach 414 entsprechend aus `get_events`).
3. Route 230: `walk-to` auf die Warp-Kachel aus „Warps verbinden“ plus ein Schritt in Richtung Ausgang, `expect-map 1`, `shot`; zurück ebenso nach `expect-map 411`.

**Patch reparieren (3b.1 (A) 18)**, auf `pt-3b1-rep`:
1. App öffnen, Atlas „Technik“ › „Patches“: Karte „Dynamische Header“ mit der Plakette „Verfügbar“, „Patch anwenden“, in der Rückfrage „Patch anwenden“ (seit 3c; vorher ging es über „Gebiet anlegen…“ bis zum Schritt Header, der Weg geht weiter). Die Plakette wechselt auf „Angewendet“. App beenden (Seite schließen, Brücke stoppen).
2. Fehler nachstellen: `printf '\xCC\xF7\x74\xFC' | dd of=<pt-3b1-rep>/arm9/arm9.bin bs=1 seek=$((0x3A0D6)) conv=notrunc`
3. App starten, Kopie öffnen: der Fehler-Toast „Patch „Dynamische Header“ fehlerhaft …“ mit der Taste „Patch reparieren“ (`call.sh header_patch_status` zeigt `repairable: true`); Technik › Patches zeigt die Plakette „Fehlerhaft“ (`data-patch="broken"`) und dieselbe Taste. Dort „Patch reparieren“ drücken: Meldung „… repariert …“, die Plakette wechselt auf „Angewendet“.
4. `xxd -g1 -s $((0x3A0D6)) -l 4 <pt-3b1-rep>/arm9/arm9.bin` zeigt `de f7 ad f8`. Bauen, `g4emu run … continue wait 600 shot weiter`: das Spiel läuft nach dem Intro weiter.
5. Gegenprobe: `pt-3b1` öffnen, die Meldung erscheint nicht.

**Raum-Assistent und Fehlerfälle (3a (F))**, auf einer eigenen Kopie `pt-3a-f`:
- *Falsches Raster:* in der Szene Header 0342 (Raster 0000) zeigen, „Neu anlegen“ (+ in der Liste „Orte“) → „Raum anlegen…“, Vorlage 0415, im Schritt Header einen freien Header, im Schritt Eingang als Außen-Header 0504 (Großmoor, Raster 0240) wählen (0002 „UG“ wird nicht als Außen-Header angeboten: Raster 0002 hat keinen Header-Abschnitt). Erwartet: der Schritt fragt nach der Kachel der Tür (leere Felder, kein Hinweis „Kachel zuletzt in der Szene gewählt“). Zum Vergleich Außen-Header 0411 (Raster 0000): die Kachel ist vorbelegt.
- *„Raum hier anbinden…“:* Header 0411 öffnen, im Inspector „Raum hier anbinden…“: der Assistent hat 0411 als Außen-Header vorgewählt.
- *Kachel außerhalb:* im Schritt Eingang x 999 eintippen: der Assistent lehnt ab, bevor etwas angelegt wird (`call.sh member_counts` vorher und nachher gleich).
- *Gesperrte Eventdatei:* App beenden, `printf '\0' >> <pt-3a-f>/unpacked/eventFiles/0390` (hängt ein Byte an die Datei von 0411 an, sie lässt sich dann nicht mehr verlustfrei schreiben), App neu starten. Raum-Assistent mit Außen-Header 0411: der Schritt Eingang nennt die Sperre und bietet „später verbinden“; `call.sh get_events '{"eventFileId":390}'` zeigt `locked`.
- *Später Fehler:* nur mit gepatchter Kopie `pt-3a-f` (zuerst wie in 3b.1 (A) 3 gepatcht). `chmod 555 <pt-3a-f>/unpacked/maps`, „Gebiet anlegen…“ mit „Neuer Header“, „Anlegen“: das Ergebnis meldet den Schritt („map copy failed …“) und das schon Angelegte („header NNNN was created“). Danach `chmod 755 <pt-3a-f>/unpacked/maps`. (Ohne neuen Header ist nichts angelegt und der Fehler lautet „nothing was created“.)
- *Cmd+Z:* vor dem Assistenten `call.sh get_header '{"id":N}'` und `call.sh member_counts` notieren, Assistent mit Header-Umstellung ausführen, `call.sh dirty` zeigt die Änderungen. Cmd+Z so oft drücken, bis `dirty` leer ist (oder `call.sh undo`): `get_header` gleicht wieder dem Vorher-Wert, `member_counts` bleibt erhöht (Karten, Raster und Eventdateien bleiben liegen, wie angesagt).

**Pokédex (3b.3a (A3), (A4), (A5))**, auf einer Platin-Kopie:
1. Atlas „Pokémon“; steht der Listenkopf nicht auf „Pokédex ▾“, ⌘1 oder das Menü im Kopf. In „Pokédex durchsuchen“ `397` tippen, Enter auf der Zeile (`[data-species="397"]`): Schau und Hand zeigen Staravia, die Brotkrume „Pokémon › Pokédex › STARAVIA“. Gegenprobe der Zahlen mit `call.sh data_get '{"kind":"species","index":397}'`.
2. Reiter „Werte“: ein Zahlfeld ändern und mit Enter übernehmen (das Feld behält den Fokus). Die Titelleiste zählt die Änderung, der Reiter und die Listenzeile tragen den Punkt, `call.sh history` nennt das Label („STARAVIA: Angriff 75 → 85“), die Schau „Summe 350, vorher 340“ und am Balken die Abweichung. ⌘Z außerhalb des Felds nimmt es zurück.
3. Reiter „Lernset“: Level einer Zeile ändern (die Tabelle sortiert nach Level, der Fokus wandert mit der Zeile), „Zeile hinzufügen“ öffnet die Attacken-Auswahl (neue Zeile auf Level 1), Entf auf einer Zeile oder „entfernen“ löscht; ab 20 Zeilen ist „Zeile hinzufügen“ gesperrt („20 von 20“). Jede Änderung ist ein Schritt.
4. Reiter „Entwicklung“: sieben Zeilen; Staravia hat „Level 34“ mit Ziel STARAPTOR 0398 (die Referenz springt zur Art, „Ändern …“ wählt ein anderes Ziel), die übrigen sechs „Keine“.
5. Reiter „TM/VM“: 100 Kästchen mit dem Namen der Attacke im `aria-label` („TM40 Aero-Ass“, „VM01 Zerschneider“); ohne arm9-Tabelle nur die Nummern. „Von anderer Art übernehmen …“ öffnet die Auswahl der Arten; die Wahl schreibt TM/VM und Lernset als einen Schritt („STARAVIA: TM/VM und Lernset wie STARALILI“). Für den Nachweis über zwei Assets siehe [phase-3b3-gate.md](phase-3b3-gate.md) (A4): vorher ein Kästchen umstellen und speichern, sonst ändert die Kopie in Platin nur das Lernset.
6. Tüfteln (Scharnier): Reiter „Rohdaten“ mit je einem Hexblock für „Pokémon-Daten“, „Lernset“ und „Entwicklungen“ (Pfad, Eintrag, Länge); markiert ist das zuletzt fokussierte Feld (`[data-marked]`, `data-offset`), der Fuß nennt Byte, Feld und Wert. Die Zahlfelder zeigen Dez/Hex.

**Attacken (3b.3a (A3), (A5))**, auf einer Platin-Kopie:
1. Im Bereich Pokémon ⌘4 oder das Menü im Listenkopf („Attacken“), oder in der Palette „Attacken öffnen“. „Attacken durchsuchen“ `flügel`, die Zeile `[data-move="17"]` wählen.
2. Schau: die Karte mit Name, Nummer, Typ, Kategorie, Stärke, Genauigkeit, AP und Merkmalen; der Name führt ins Namensarchiv (Bereich Texte, Archiv 0647), „Beschreibung bearbeiten“ zur Beschreibung (Archiv 0646, Nachricht 017); die DS-Textbox blättert mit „Weiterblättern“ und „Zurück“. „Verwendet in“ nennt Lernsets und Trainer; die Lernset-Plaketten öffnen die Art im Reiter „Lernset“, die ersten zwölf stehen da, „Alle zeigen, N weitere“ zeigt den Rest.
3. Hand: „Stärke“ ändern, Enter; `call.sh history` nennt „Flügelschlag: Stärke 60 → 65“, ⌘Z. Ein Merkmal in „Verhalten“ umschalten: in Tüfteln zeigt die Spalte „Rohdaten“ das „Merkmale-Byte“ (0x73 → 0x72 bei „Macht Kontakt“ aus; die Bits ohne Namen bleiben).
4. Tüfteln: Spalte „Rohdaten“ mit Effekt-ID, den Kästchen „Bit 0“ bis „Bit 7“ und dem Hexblock „files/poketool/waza/pl_waza_tbl.narc, Eintrag 17, 16 Bytes“; mit Fokus auf „Stärke“ ist Byte 3 markiert („Byte 3: Stärke, 0x3C = 60“).

**Gegenstände (3b.3a (A3), (A5), (A6))**, auf einer Platin-Kopie:
1. Atlas „Gegenstände“. Die Liste zeigt Symbole aus dem ROM (`[data-sprite="picture"]`, geladen nur für Zeilen im Blick) und die Preise; der Filter „Taschen“ hat je Tasche einen Zähler. Trank wählen (`[data-item="17"]`, Suche `trank` oder `17`): Brotkrume „Gegenstände › Tasche Medizin › Trank“; ein Klick auf „Tasche Medizin“ filtert die Liste („Tasche Medizin: 38“, „Filter lösen“).
2. Schau: Karte mit Symbol, Name, Nummer und Preis, die Beschreibung als DS-Textbox mit „Beschreibung bearbeiten“ (Bereich Texte), „Verwendet in“ nach Gruppen (Trainer-Plaketten öffnen seit 3b.3b den Trainer). Gegenprobe `call.sh refs '{"target":{"kind":"item","id":17}}'`.
3. Hand: „Preis“ ändern, Enter: `call.sh history` nennt „Trank: Preis 300 → 250“, die Listenzeile zeigt den neuen Preis mit Punkt; ⌘Z. Geteilte Daten: ein Gegenstand, dessen Datenmember auch andere IDs nutzen (Platin: 0113), zeigt in der Hand die Warnung mit den anderen Gegenständen als Referenzen, und das Label endet auf „gilt für 23 Gegenstände“.
4. Tüfteln: Spalte „Rohdaten“ mit dem Hexblock „files/itemtool/itemdata/pl_item_data.narc, Eintrag 17, 34 Bytes“ (Fokus auf „Preis“: „Byte 0 bis 1: Preis, 0x012C = 300“) und darunter die Felder „Wirkung im Team“.

**Trainer (3b.3b (B4), (B7))**, auf einer Kopie mit übernommenen Scripts (Welt, Reiter „Script“, „Übernehmen“; ohne das lehnt das Anlegen ab mit „Erst Scripts übernehmen (Bereich Welt, Panel Script)“):
1. Im Bereich Pokémon ⌘2, das Menü im Listenkopf oder die Palette „Trainer öffnen“. Der Baum „Trainer“ steht „Nach Ort“; „Trainer durchsuchen“ mit `logan` oder `2` findet Teenager Logan 0002 (`[data-trainer="2"]`). Schau: Team als Karten, „Steht in der Welt“ mit „Route 202, Kachel 185, 804“ (Klick öffnet die Figur in der Welt). Gegenprobe `call.sh data_get '{"kind":"trainer","index":2}'`.
2. „Trainer anlegen…“ (+ im Listenkopf): Dialog „Trainer 928 anlegen?“ (Platin; HeartGold 738), Klasse „Wanderer“, Name `Jens` („Höchstens 7 Zeichen, jetzt 4 von 7“; acht Zeichen lehnt der Dialog ab), Plakette „Nicht rückgängig“, „Anlegen“. Der neue Trainer ist gewählt, `call.sh history` endet auf „Trainer 928 angelegt“, die Schau warnt „Ohne Pokémon stürzt der Kampf ab“. Ein zweites Anlegen nennt „Trainer 930 anlegen?“ und „929 wird Platzhalter für das Sicht-Script“; danach zeigt die Liste 929 gesperrt, die Meldung nennt Platzhalter und Erweiterung der Kampf-Scriptdatei 1114 (HeartGold: erst beim dritten, 741 hinter 740, Datei 0953).
3. Hand „Team“: Karte „Platz 1 leer“ anklicken, Art wählen (Picker mit Typ-Plaketten), Level ändern; `history` nennt „Wanderer Jens: Platz 1 STARAVIA hinzugefügt“ und „…: Platz 1 Level 5 → 14“. Mit „Eigene Attacken“ (Reiter „Trainer“) füllt das Spiel jedes Pokémon mit den letzten vier Lernset-Attacken bis zu seinem Level; der Schalter sagt das im Label.
4. Hand „Trainer“, Kampftexte: „Text anlegen“ bei „vor dem Kampf“, Text ohne ⏎ am Ende: Warnung am Feld, Prüflicht „1 Warnung“, im Reiter „Probleme“ eine Zeile, die den Trainer öffnet; „⏎ anhängen“ behebt sie. Gegenprobe `call.sh data_get` (`texts`, `problems`).
5. ⌘Z nimmt die Schritte einzeln zurück. Steht „Trainer N angelegt“ oben, meldet ⌘Z „Trainer N anlegen lässt sich nicht rückgängig machen“, und `call.sh history` bleibt gleich.
6. Tüfteln: Reiter „Rohdaten“ mit dem Hexblock von Eigenschaften und Team und den 32 KI-Bits.

**Begegnungen (3b.3b (B5), (B7), (B8))**, auf einer Kopie:
1. ⌘3 oder das Menü im Listenkopf. „Begegnungen durchsuchen“ `Route 201`: Datei 0140 („Route 201, verwendet von 1 Ort“, `[data-encounter="140"]`); Brotkrume „Pokémon › Begegnungen › Route 201“. Gegenprobe `call.sh data_get '{"kind":"encounter","index":140}'` (je Slot `share`, `replaced_by`, `warnings`).
2. Schau: „Wer hier wartet“ (Platin DE: STARALILI Morgens 50 % · Tags 50 % · Nachts 40 %, BIDIZA, ZIRPURZE), „Verwendet von 1 Ort“ mit Route 201 (Header 0342).
3. Hand, Reiter „Gras“: Rate 30 → 100, Platz 1 STARALILI → PIKACHU (Picker der Arten, „Keine“ = 0): `history` „Route 201: Gras-Rate 30 → 100“, „Route 201: Gras Platz 1 STARALILI → PIKACHU“; die Schau zeigt PIKACHU 20 %. Ein Level 0 bei belegter Art warnt in der Hand, in der Schau und im Prüflicht (Quelle „Begegnungen“). Reiter „Ersatz“: je Liste eine Tabelle; Platin dazu „Formen“.
4. HeartGold DE: Route 29 (0001): Reiter Gras12 · Surfen0 · Fels0 · Alte Angel0 · Profi-Angel0 · Super-Angel0 · Ersatz5, Gras mit Morgens · Tags · Nachts · Level; HOOTHOOT nur nachts.
5. Feld „Begegnungen“ im Header-Editor (Welt, Header wählen, Inspector): Referenz „Begegnungen 0140“ (öffnet die Datei), „Ändern…“ öffnet die Auswahl der Dateien (Orte, „verwendet von N Orten“, „Keine“). Ein Wechsel ist ein Schritt „Begegnungen“.
6. Abspalten: In Platin nutzt keine Vanilla-Datei mehrere Orte; deshalb Zweiblattdorf (411, Datei 0176) im Header-Editor auf 0140 stellen. Die Schau von 0140 zeigt „Verwendet von 2 Orten“ und je Ort „Für diesen Ort abspalten“. „Zweiblattdorf: für diesen Ort abspalten…“ → Dialog „Zweiblattdorf eine eigene Begegnungsdatei geben?“ → „Abspalten“: die Kopie (0183) ist gewählt und heißt „Zweiblattdorf“. ⌘Z hängt den Header zurück, die Kopie bleibt („von keinem Ort verwendet“). Scheitert das Umhängen nach der Kopie, schließt der Dialog trotzdem, die Kopie ist gewählt und eine Warnung sagt, was bleibt.
7. „Begegnungsdatei anlegen…“ (+): Dialog „Begegnungsdatei 0184 anlegen?“, „Leer“ oder „Kopie von …“ (Picker „Vorlage wählen“), „Nicht rückgängig“; danach ist die neue Datei gewählt, alle Reiter 0.

**Platzieren (3b.3b (B6))**, auf einer Kopie:
1. Pokémon › Trainer, ein Trainer ohne Figur (zum Beispiel in der Gruppe „Ohne Ort“, oder ein neu angelegter): „In der Welt platzieren“. Die App wechselt in die Welt, das Werkzeug ist „Figur“, die Statusleiste sagt „Figur · Klick setzt Wanderer Jens · Esc hebt die Bindung auf“. Vorher einen Ort zeigen; ohne Ort meldet die App „Erst einen Ort wählen, dann den Trainer platzieren.“
2. Ein Klick auf eine Kachel der Szene (`hover`, dann Klick) legt eine Figur mit Script `3000 + ID − 1`, `trainer_type` 1 und Sichtweite 3 an, als **ein** Schritt „Trainer Jens platziert“; danach steht wieder „Auswählen“. Gegenprobe `call.sh get_events '{"eventFileId":N}'` und `call.sh history`. Escape, ein anderes Werkzeug oder ein anderer Bereich heben die Bindung vorher auf.
3. Inspector der Figur: Feld „Trainer“ als Referenz („Wanderer Jens 0928“, öffnet den Trainer), darunter „Sichtlinie“ und „Sichtweite“. „Trainer ändern…“ wählt einen anderen Trainer (ohne Platzhalter und Dummy 0) und schreibt das Script in einem Schritt; eine Partner-Figur (Script 5000–6999, „(Partner)“) bleibt im Partner-Bereich. „In dieser Szene“ nennt Trainer-Figuren mit Namen („Objekt 0 · Teenager Sebastian“, Route 203).
4. Blickrichtung quer zum Weg stellen (Feld „Blickrichtung“); die Sichtweite zählt Kacheln in Blickrichtung. Ob der Trainer sieht, zeigt erst das Spiel (Szenario oder `g4emu`).

**Prüfen (3b.4a (A3))**, auf `pt-3b4a-gate` (frische Platin-Kopie):
1. Seite mit `?project=…` öffnen. Das Prüflicht im Scharnier dreht (`button[data-light][data-running]`, im `aria-label` „Prüft …“), danach `aria-label` „101 Warnungen, 10 Hinweise“, `data-light="warning"`, Zähler `[data-count=error]` 0 (gedämpft), `[data-count=warning]` 101, `[data-count=info]` 10. Ein Klick auf das Licht öffnet den Reiter „Probleme“: Untertitel wie das Licht, Abschnitte „Welt“ (13) und „Texte“ (98), darin die Gruppen `[role=group][data-check]` („Tür löst nie aus“ 10, „Zeile zu breit“ 90 …), darüber `[role=list][aria-label="Nicht geprüft"]` mit „Nachricht fehlt“ (Scripts nicht übernommen). Gegenprobe `call.sh checks_status` (gleiche Zahlen, drei `groups`).
2. Die vier Fehler über `call.sh` (dieselbe Brücke wie die Seite), speichern, Seite neu laden (das Neuladen öffnet das Projekt neu; ungespeicherte Änderungen der Brücke gingen dabei verloren). „Erneut prüfen“ statt Neuladen zählt dasselbe, bringt aber Editoren und Titelleiste nicht auf den Stand der Brücke (die Titelleiste sagt dann „Alles gespeichert“, obwohl die Brücke Ungespeichertes hat); nur das Neuladen tut das:
   ```sh
   scripts/agent/call.sh adopt_scripts
   scripts/agent/call.sh update_event '{"eventFile":394,"kind":"warp","index":0,"patch":{"dest_warp_id":99}}'
   N=$(scripts/agent/call.sh new_script_file '{"kind":"script"}')        # 1124
   H=$(scripts/agent/call.sh get_header '{"id":22}' | N=$N python3 -c 'import json,os,sys; h=json.load(sys.stdin); h["Pt"]["script_file_id"]=int(os.environ["N"]); print(json.dumps({"id":22,"header":h}))')
   scripts/agent/call.sh set_header "$H"
   scripts/agent/call.sh add_event '{"eventFile":21,"kind":"object","x":4,"z":6,"patch":{"script":950}}'
   scripts/agent/call.sh update_event '{"eventFile":390,"kind":"object","index":6,"patch":{"script":3999,"trainer_type":1}}'
   scripts/agent/call.sh edit_text_archive '{"index":558,"edit":{"Set":{"index":3,"text":"Diese Zeile hat sechzig Zeichen und passt nicht ins Fenster."}}}'
   scripts/agent/call.sh save
   ```
   Nach dem Laden: „3 Fehler, 102 Warnungen, 10 Hinweise“, `data-light="error"`. Im Panel `li[data-finding="warp.dest-warp|eventFile:394|warp:0"]`, `…event.script-missing|eventFile:21|object:0`, `…event.trainer-missing|eventFile:390|object:6` (je `data-state="error"`, ohne „Ausblenden“) und `li[data-finding="text.too-wide|textArchive:558|message:3:line:1"]` (`data-state="warning"`); „Beheben…“ nur an `event.script-missing` und `text.too-wide`; Zeilen ohne Ziel (etwa ein Befund am ganzen Projekt) haben kein „Öffnen“. „Erneut prüfen“ im Fuß ist während eines Laufs gesperrt.
3. Automatischer Lauf: in der Welt Header 0415, die Treppe wählen (Szene oder die Gruppe „Türen“ in „In dieser Szene“), im Inspector „Ziel-Warp“ von 99 auf 1, Enter. Etwa 300 ms danach dreht das Licht, dann zeigt es „2 Fehler, …“; ⌘Z nimmt die Änderung zurück, und nach dem nächsten Lauf sind es wieder drei. Technik › Prüfen und Bauen (`[data-technik=bauen]`) zeigt dieselben Gruppen.

**Beheben (3b.4a (A4))**, nach „Prüfen“ auf derselben Kopie (Fix-Dialog `[role=dialog]`, Körper `[data-fix]`):
1. An der Zeile `text.too-wide` „Beheben…“: Titel „Textarchiv 0558 · Nachricht 3 · Zeile 1“, darunter der Satz; die Vorschau `[role=group][aria-label="Textarchiv 0558"]` mit „Vorher“ (die eine Zeile, `[data-changed]`) und „Nachher“ („Diese Zeile hat sechzig Zeichen und“ / „passt nicht ins Fenster.“). „Umbrechen“: der Dialog schließt, die Zeile ist weg, `call.sh history` endet auf „Behoben: text.too-wide“, `call.sh dirty` nennt Textarchiv 558. ⌘Z bringt den Befund nach dem nächsten Lauf zurück, ⇧⌘Z behebt ihn wieder.
2. An der Zeile `event.script-missing` „Beheben…“: Plakette „Nicht rückgängig“ und „Wird sofort geschrieben, ⌘Z nimmt es nicht zurück.“, Vorher „Leer“, Nachher `script script_950 #950:` / `End`. Solange Textarchiv 558 ungespeichert ist (nach Schritt 1 ist es das), meldet „Script anlegen“ „Erst speichern“ (`role=alert`) mit „Schließen“ und „Speichern und anlegen“. „Speichern und anlegen“ speichert das Projekt wie „Speichern“ in der Titelleiste und legt das Script an: der Dialog schließt, der Befund ist weg, `call.sh read_script '{"file":1124}'` zeigt `#950`, `call.sh dirty` ist leer, `history` hat keinen neuen Schritt. (Die Titelleiste erreicht man bei offenem Dialog nicht: ein Klick daneben schließt ihn nur.)
3. Veraltet: „Beheben…“ an einer Zeile `text.too-wide` öffnen (vorher etwa mit `call.sh edit_text_archive` eine zweite zu lange Zeile anlegen und „Erneut prüfen“), dann bei offenem Dialog dieselbe Nachricht über `call.sh edit_text_archive` noch einmal ändern (anderer, ebenso zu langer Text), dann „Umbrechen“: „Befund ist nicht mehr aktuell“ (`role=alert`) mit „Schließen“ und „Erneut prüfen“; `call.sh history` hat keinen neuen Schritt „Behoben: …“.

**Ausblenden (3b.4a (A5))**:
1. An einer Warnung oder einem Hinweis (etwa in „Tür löst nie aus“) „Ausblenden“: der Zähler im Licht sinkt um eins, der Fuß zeigt „1 ausgeblendet · Wieder melden…“; `cat <Kopie>/g4-checks.json` hat den Eintrag unter `ignored` (`code`, `subject`, `key`, `fingerprint`). Fehlerzeilen haben keine Taste „Ausblenden“.
2. „Wieder melden…“ (`aria-expanded`) klappt `[role=list][aria-label=Ausgeblendet]` auf (sie rollt in ihrer eigenen Höhe, der Fuß nimmt höchstens die halbe Hand); dort „Wieder melden“: die Zeile steht wieder in ihrer Gruppe, `ignored` in der Datei ist leer, und die leere Liste klappt zu. Ändert sich die Stelle eines ausgeblendeten Befunds, passt der Fingerprint nicht mehr und er meldet sich von selbst wieder.
3. Technik › Prüfen und Bauen › „Prüfungen ein- und ausschalten“ aufklappen: `[role=list][aria-label="Prüfungen ein- und ausschalten"]`, je Code `li[data-check]` mit `[role=switch]`. „Zu viele Zeilen“ (`text.three-lines`) aus: ihre Befunde verschwinden, `g4-checks.json` nennt den Code unter `disabled`; wieder an: sie kommen zurück. Die Schalter der Fehler-Prüfungen sind gesperrt („Fehler lassen sich nicht abschalten.“), auch „Header einer anderen Matrix“ (`matrix.header-elsewhere`, Fehler seit Ruling R16): 24 gesperrt, 15 schaltbar.

**Ausgangslage (3b.4a (A5))**: Atlas „Technik“ › „Prüfen und Bauen“, „Ausgangslage übernehmen…“: der Dialog „Ausgangslage übernehmen?“ nennt die Zahl („111 Warnungen und Hinweise werden ausgeblendet. …“ auf frischem Platin), „Übernehmen“ (per `ref` klicken; ein Klick per Koordinaten traf in Task 6 daneben und schloss den Dialog ohne Wirkung). Danach zeigt der Fuß „111 ausgeblendet“ (die Liste bleibt zugeklappt) und das Licht nur noch die Fehler („Alles stimmig“ ohne Fehler); `call.sh checks_status` hat 111 Einträge in `ignored`, `g4-checks.json` ebenso. Nach dem Neuladen der Seite bleibt die Ausgangslage (die Datei wird beim Öffnen gelesen).

**Teile importieren (3b.4b (B5) bis (B7))**, auf `pt-3b4b-gate` (frische Platin-Kopie). Die Brücke liest Import-Dateien nur unter `_testkopien` (nie aus einem Original oder `_saves`) und unter `G4_IMPORT_ROOT` (Ordner, mit `:` getrennt); Exporte schreibt sie wie ein ROM-Ziel nur unter `_testkopien`, und nur in einen Ordner, den es schon gibt. Die Schritte 3 und 4 legen ihre Dateien in `<Kopie>/export/` an; den Ordner hat eine frische Kopie nicht, deshalb vor Schritt 3 `mkdir -p <Kopie>/export`.
1. Prüfliste: Raster von Route 201 (Header 0342), Zelle mit Karte 5 wählen (oder in „In dieser Szene“ die Zeile „Karte 0005 · Zelle …“). Im Kopf des Inspectors das Menü „Aktionen“ › „Teile importieren…“ (früher die Taste am Feld „Karte“; oder Palette „Teile in die Karte importieren…“, braucht eine gewählte Zelle). Im Dialog „Teile in Karte 0005 importieren“ (`[data-import-dialog=5]`): Quelle „Karte im Projekt“, Feld „Quellkarte“ 6, Enter (oder „Karte wählen…“: der Picker „Quellkarte wählen“ zeigt „Karte 0006“ usw. ohne Kartenbild, die Zielkarte gesperrt; beim Schließen bleibt die Liste stehen). Die Prüfliste `[role=list][aria-label=Teile]` hat je Teil eine Zeile `li[data-check=<teil>][data-state=ok|warning|error|info]` mit einem Kästchen, das nach dem Teil heißt („Modell“, „Kollision“, „Gebäude“, „BDHC (Laufhöhen)“); Teile mit Fehler lassen sich nicht ankreuzen, leere („Leer, wird nicht übernommen“) sind nicht vorgewählt. Die Taste nennt Zahl und Verb („N Teile übernehmen“, bei einem Teil „1 Teil übernehmen“; mit nur „Modell“ und „Kollision“ angekreuzt „2 Teile übernehmen“).
2. Import, Befunde, Szene, ⌘Z: nur „Modell“ und „Kollision“ angekreuzt lassen, „2 Teile übernehmen“: Meldung „2 Teile übernommen“ mit „Schließen“; darunter „Prüft die Karte …“ (`role=status`), nach dem nächsten vollen Lauf „Keine neuen Befunde an dieser Karte.“ oder die Liste `[role=list][aria-label="Befunde an Karte 0005"]` (Zeilen wie im Panel „Probleme“ mit `data-check` und `data-finding`; „Zeigen“ schließt den Dialog und führt zum Ziel), ohne Bericht „Die Prüfungen haben kein Ergebnis geliefert. Starte sie unter Probleme mit „Erneut prüfen“.“ Die Liste zeigt alle Befunde der Karte aus diesem Lauf, nicht nur neue. Die Szene (Reiter „3D“) lädt das Modell neu (nur das Modell dieser Karte; die übrigen bleiben stehen, kein Toast „Modelle n/306“). Der Import holt die Karte außerdem neu wie ⌘Z: wer nur „Kollision“ ankreuzt und die Gelände-Schicht (Taste „Gelände“ über der Szene) eingeschaltet hat, sieht die Farben von Karte 5 sofort mit der neuen Kollision (`get_map` einmal im Netzwerk der Seite, GB8). Karte 5 ins Bild holen: das Mausrad zoomt auf die Mitte, Ziehen mit der linken Taste dreht die Kamera, Rechts-Ziehen (oder Umschalt und linke Taste) schwenkt sie. Das Browser-Werkzeug kann kein Rechts-Ziehen, und sein Ziehen dreht auch mit Umschalt; deshalb ein Rechts-Ziehen per `javascript_tool` auf dem Canvas auslösen (`pointerdown`, `pointermove`, `pointerup` mit `button: 2`; das bewegt nur die Kamera, `call.sh dirty` bleibt leer). Beim Darüberfahren nennt die Statusleiste „Karte 0005 · Kachel x …“. Gelingt das nicht, genügt als Beleg das Vorschaubild. Das Vorschaubild der Zelle wird neu gezeichnet (vorher und nachher `getComputedStyle(…).backgroundImage` der Zelle `[data-cell]` per `javascript_tool` lesen: zwei verschiedene Bilder). `call.sh history` nennt „map 0005: model and permissions imported“. ⌘Z (außerhalb eines Felds) nimmt den Import als einen Schritt zurück: Szene und Vorschaubild zeigen wieder Karte 5, `call.sh dirty` nennt die Karte nicht mehr; ⇧⌘Z stellt ihn wieder her.
3. Export: erst `mkdir -p <Kopie>/export`. Zelle mit Karte 6, im Kopf des Inspectors „Aktionen“ › „Teile exportieren…“ (früher die Taste am Feld „Karte“): Dialog „Teil von Karte 0006 exportieren“, Segmente „Teil“ („Modell“ … „Ganze Karte“, in zwei Zeilen), Feld „Pfad“ `<Kopie>/export/karte6.nsbmd`, „Exportieren“: „Modell exportiert nach <Pfad>“. Dann „Kollision“ wählen: die Meldung verschwindet, der Pfad wird zu `<Kopie>/export/karte6.per`; „Exportieren“: „Kollision exportiert nach <Pfad>“ (`wc -c` zeigt 2048). Fehlt der Ordner, sagt der Dialog „Den Ordner <Kopie>/export gibt es nicht. Lege ihn an oder wähle einen anderen Pfad.“ Ohne Dialog: `call.sh export_map_part '{"map":6,"part":"model","path":"<Kopie>/export/karte6.nsbmd"}'`.
4. Deutsche Fehlertexte: zwei kaputte Dateien anlegen (das zu große Modell aus `scripts/g4emu`, die zu kurze Kollision aus dem Export von Schritt 3):
   ```sh
   uv run python -c "from g4emu import fixtures; fixtures.make_oversize_nsbmd('<Kopie>/export/zu-gross.nsbmd')"
   head -c 100 <Kopie>/export/karte6.per > <Kopie>/export/kurz.per
   ```
   Dialog an Karte 5, Quelle „Datei“, in „Pfade, eine je Zeile“ beide absoluten Pfade, ⌘Enter oder das Feld verlassen: Zeile „Modell“ mit `data-state="error"`, gesperrtem Kästchen und „Das Modell ist 65.536 Byte groß, erlaubt sind 61.440. Wähle ein kleineres Modell.“, Zeile „Kollision“ mit „Die Kollision ist 100 Byte groß, erwartet sind genau 2.048. Wähle eine vollständige .per-Datei.“, die Taste „0 Teile übernehmen“ gesperrt. Kein englischer Satz in der Liste: jeder Code der Teilprüfung (`PartCode` im Kernel) hat seinen deutschen Satz mit Ausweg in `apps/studio/src/workbench/inspector/importParts.ts` (`PART_TEXTS`); ein Modell oder Terrain, das in einer ganzen Kartendatei kaputt ist, sagt „Das Modell in der Kartendatei ist ungültig (…). Wähle eine andere Kartendatei.“ bzw. „Das Terrain in der Kartendatei ist beschädigt (…). Wähle eine andere Kartendatei.“; der Grund in Klammern ist der englische Text des Kernels (so gewollt, bei `zu-gross.nsbmd` und `kurz.per` kommt keiner vor). Mit `karte6.nsbmd` und `karte6.per` aus Schritt 3 sind beide Zeilen `ok`.
5. Gegenprobe ohne Dialog: `call.sh inspect_map_import '{"map":5,"source":{"map":6}}'` (je Teil `{part, size, file, state, notes}`; `state` ist `{kind: "ok"}` oder `{kind, code, args, message}` mit dem Code wie oben und dem englischen Satz des Kernels) und `call.sh import_map_parts '{"map":5,"source":{"map":6},"parts":["model","permissions"]}'` (liefert `[{"kind":"Map","index":5}]`). Eine Ablehnung über die Brücke bleibt englisch: `import_map_parts` mit `zu-gross.nsbmd` meldet Exit 1 und „import refused: model: model is 65536 bytes; the game holds at most 61440 (0xF000)“.

**Gesamtgate (3b.4b, Teil B (B2) bis (B3a))**, aus einem Worktree:
1. Je eine frische Kopie `pt-3b4b` und `hg-3b4b` (optional `hgus-3b4b`), für die Gegenproben `pt-3b4b-kontrolle` und `hg-3b4b-kontrolle`. `cargo build --workspace --all-features`; `G4_ROMS_DIR`, `G4_BRIDGE_BIN` und `G4K_BIN` absolut. Die Kopf-Kommentare der Szenarien zeigen den Befehl mit relativen Pfaden (`../../../Roms`), die aus einem Worktree ins Leere zeigen; dort die absolute Form (der Ausgabeordner muss vor der Umleitung der Konsole existieren):
   ```sh
   W=<absoluter Pfad des Worktrees>; R=<absoluter Pfad zu Roms>; O=$W/target/agent/gate-$(date +%F)
   mkdir -p "$O"
   export G4_ROMS_DIR=$R G4_BRIDGE_BIN=$W/target/debug/studio-bridge G4K_BIN=$W/target/debug/g4k
   cd $W/scripts/g4emu
   uv run g4emu scenario ../agent/scenarios/pt-kleiner-hack.yaml --project $R/_testkopien/pt-3b4b \
     --out $O/pt-kleiner-hack --set NEUER_HEADER=593 --set KOPIE=$R/_testkopien/pt-3b4b \
     --set S_WILD=150 --set L_WILD=5 --set S_T1=25 --set L_T1=12 --set S_T2=133 --set L_T2=14 --set S_GIFT=445 \
     > $O/pt-kleiner-hack.out 2> $O/pt-kleiner-hack.err
   uv run g4emu scenario ../agent/scenarios/hg-kleiner-hack.yaml --project $R/_testkopien/hg-3b4b \
     --out $O/hg-kleiner-hack --set NEUER_HEADER=540 --set KOPIE=$R/_testkopien/hg-3b4b \
     --set S_WILD=150 --set L_WILD=5 --set S_T1=25 --set L_T1=12 --set S_T2=133 --set L_T2=14 --set S_GIFT=445 \
     > $O/hg-kleiner-hack.out 2> $O/hg-kleiner-hack.err
   ```
   HeartGold US (optional): dieselbe Datei mit `game: IPKE`, Kopie `hgus-3b4b`:
   ```sh
   sed 's/^game: IPKD/game: IPKE/' ../agent/scenarios/hg-kleiner-hack.yaml > $O/hgus-kleiner-hack.yaml
   uv run g4emu scenario $O/hgus-kleiner-hack.yaml --project $R/_testkopien/hgus-3b4b \
     --out $O/hgus-kleiner-hack --set NEUER_HEADER=540 --set KOPIE=$R/_testkopien/hgus-3b4b \
     --set S_WILD=150 --set L_WILD=5 --set S_T1=25 --set L_T1=12 --set S_T2=133 --set L_T2=14 --set S_GIFT=445 \
     > $O/hgus-kleiner-hack.out 2> $O/hgus-kleiner-hack.err
   ```
   `prepare:` legt `<Kopie>/export/zu-gross.nsbmd` selbst an; `KOPIE` muss der absolute Pfad der Kopie sein, weil die Exporte und Importe über ihn laufen.
2. Bestanden heißt: Rückgabe 0, `"ok": true` in der letzten Zeile von stdout, 37 Zeilen „Erwartung erfüllt“ auf stderr, in `check.log` „5 expectation(s) met“ (HeartGold 7). Vor „Erwartung erfüllt: import_map_parts abgelehnt: …“ druckt die Brücke selbst „import_map_parts fehlgeschlagen: import refused: model: model is 65536 bytes; the game holds at most 61440 (0xF000)“: das ist die erwartete Ablehnung, kein Fehler. Ein Lauf dauert unter einer Minute (gemessen 47 bis 49 s, 23 Brücken-Sitzungen).
3. Gegenprobe: `pt-kleiner-hack-ohne-import.yaml` auf `pt-3b4b-kontrolle` und `hg-kleiner-hack-ohne-import.yaml` auf `hg-3b4b-kontrolle`, mit denselben `--set`-Werten (`KOPIE` = die Kontroll-Kopie), Ausgabe `--out $O/pt-kleiner-hack-ohne-import` und Konsole `> $O/pt-kleiner-hack-ohne-import.out 2> $O/pt-kleiner-hack-ohne-import.err` (HeartGold entsprechend). Bestanden heißt hier: das Szenario **scheitert** am Weg (Rückgabe 2), die letzte Schrittzeile meldet „blockiert bei x 5, y 21“ (Platin) bzw. „blockiert bei x 11, y 24“ (HeartGold), die Zusammenfassung `"ok": false`. Damit ist belegt, dass der Weg nur mit der importierten Kollision geht; das Szenario selbst zeigt nur, dass er mit ihr geht.
4. Was die Szenarien nicht beweisen: Dass die abgelehnte Änderung nichts schrieb, zeigt kein Szenario (`dirty` ist nach einer Ablehnung leer, weil die nächste Sitzung neu von der Platte öffnet; der Hash im ROM zeigt nur den gelungenen Import); das belegen die Tests vor dem Schreiben. Ein Tippfehler im Wert eines Filters (`findings[code=warp.dest-wrap]`) fällt nicht auf: Codes aus dem Katalog kopieren. Pfade mit Leerzeichen in `play:` quoten (siehe „Szenario“).
5. Die Bilder für „Agent + Blick“ liegen im Ausgabeordner, weil `g4emu` weder Text noch Flags liest: `weg.png` (Route texturiert, die Figur auf dem Weg, den nur die importierte Kollision freigibt), `nachbar.png` und `nachbar_danach.png` (Trank einmalig per Flag, danach ein anderer Text), `geschenk.png` (der Geber schweigt danach; das Pokémon belegen die gewonnenen Kämpfe), dazu `ankunft.png`, `trainer1.png`, `trainer2.png`, `wild.png`. Der Agent nennt dem Nutzer Pfad und was zu sehen ist. Scheitert ein Lauf nach `build_rom`, für den nächsten eine neue Kopie mit Endung anlegen.

**DOM-Scan Bauen (3e.2, seit 3f.1 ohne Ausnahme).** In Bauen steht in Liste, Scharnier, Dock und Statusleiste kein „Raster“, „Matrix“ oder „Header“; den Reiter „Raster“ gibt es in Bauen seit 3f.1 nicht mehr, eine Ausnahme braucht es nicht. Ohne offenen Assistenten, 3D vorn, dann noch einmal mit der Weltkarte vorn (Rezept „Dockview-Reiter im Browser umschalten“):
1. Ein Gebiet wählen (`[data-place-key="Zweiblattdorf"][data-area-key]` anklicken), warten bis `[data-event]` erscheint, dann eine Figur (`[data-event="object:0"]`), einen Raum (`[data-room]`) und eine Karte (in „In dieser Szene“ die Gruppe „Karten“ öffnen, `[data-scene-group=maps]`, und eine Kartenzeile anklicken: der Inspector zeigt die Zelle).
2. Nach jeder Auswahl im Browser-Fenster (Vite liefert das Modul aus): `const {visibleWords} = await import('/src/ui/visibleText.ts')`, dann für die Liste (`section[aria-label="Orte"]` und `section[aria-label="In dieser Szene"]`, je einzeln), Scharnier (`[role=toolbar][aria-label=Scharnier]`), das Dock (`#dock-welt`, seit 3e.4 Schau und Bearbeiten-Gruppen in einem; die Inhalte der Panels liegen über ihren Gruppen, deshalb das ganze Dock) und die Statusleiste `visibleWords(el).match(/.{0,20}(Raster|Matrix|Header).{0,20}/g)`.
3. Bestanden heißt: kein Treffer, auch nicht im Dock (die Reiter der Schau lesen sich „3D Weltkarte …“). Dasselbe gilt in jedem Schritt der Assistenten „Gebiet anlegen“ und „Raum anlegen“ (`[data-wizard]` und das Scharnier mit der Schrittleiste) und in den Dialogen „Neuer Ort“, „Karte bei … anlegen?“ und „Raum abspalten?“. Bis 3f.1 war der Reiter „Raster“ der eine erlaubte Treffer. `visibleWords` überspringt `hidden`, `aria-hidden="true"`, `display: none` und `visibility: hidden` (Dockview verbirgt so die inaktiven Panels).

**Ladezeit messen (3f).** `scripts/agent/measure-scene-load.js` misst, wie lange das Zeigen eines Gebiets dauert: vom Klick auf die Baumzeile bis zur letzten Antwort der Brücke (`/__bridge/`), wenn 4 s lang keine Anfrage mehr kam. So geht es:
1. Seite `http://127.0.0.1:<Vite-Port>/?project=<Kopie>` laden (vorher `localStorage.clear()`, damit kein Gebiet schon gezeigt ist), Fenster 1440 × 900.
2. Den ganzen Inhalt der Datei mit `javascript_tool` in die Seite einfügen (er legt `window.measureSceneLoad` an), dann `await measureSceneLoad('[data-area-key="w0:1"]')` (Platin, Zweiblattdorf) bzw. `'[data-area-key="w0:126"]'` (HeartGold, Neuborkia). Die Funktion wartet, bis die Zeile da ist, dann 3 s, und liefert `{ ms, requests, maps, models }` (`maps` = Aufrufe `get_map`, `models` = `get_map_model_glb`). Die Abfrage `agent_poll` des Browser-Modus (alle 500 ms, seit 3h.3) zählt nicht mit, sonst würde es nie 4 s still.
3. **Kalt** ist die erste Messung nach dem Start der Brücke (die Kartenmodelle sind noch nicht in ihrem Cache), **warm** jede weitere nach Neuladen der Seite (Schritt 2 wiederholen, das Neuladen löscht die Funktion). Für einen kalten Wert die Test-Umgebung neu starten.
4. Zahlen aus [phase-3f-gate.md](phase-3f-gate.md), (1.10) und (G.7), Debug-Build der Brücke, verborgenes Browser-Fenster: Zweiblattdorf vor 3f.1 (`974a545`) kalt 7523 ms, warm 2851 und 2481 ms, 852 Anfragen, `get_map` 352, Modelle 176; nach 3f.1 warm 347 und 510 ms, 45 Anfragen, `get_map` 12, Modelle 6; nach 3f.3 (`f6f958e`) kalt 471 ms, warm 570 ms. Neuborkia vorher kalt 9856 ms, warm 3854 ms, 1058 Anfragen, `get_map` 408, Modelle 204; nachher warm 596 ms, 55 Anfragen, `get_map` 12, Modelle 6. Die Szene lädt das Gebiet und den Ring seiner 8 Nachbarzellen; geprüft wird, dass „nachher“ kleiner ist als „vorher“, nicht ein fester Wert (die Zeiten schwanken mit dem Rechner).

**Dockview-Reiter im Browser umschalten (3f).** Dockview wechselt den Reiter beim `pointerdown` (linke Taste) auf dem Element `.dv-tab`, nicht beim `click`; ein `element.click()` allein wechselt deshalb nicht. Im 3f-Gate lief das Browser-Fenster verborgen, darum der Weg über `javascript_tool` mit der ganzen Folge synthetischer Ereignisse auf der Mitte des Reiters:

```js
const tab = document.querySelector('[data-dock-tab="worldmap"]').closest('.dv-tab');
const r = tab.getBoundingClientRect();
const at = { bubbles: true, cancelable: true, button: 0, clientX: r.x + r.width / 2, clientY: r.y + r.height / 2 };
for (const [Kind, type] of [[PointerEvent, 'pointerdown'], [MouseEvent, 'mousedown'], [PointerEvent, 'pointerup'], [MouseEvent, 'mouseup'], [MouseEvent, 'click']])
  tab.dispatchEvent(new Kind(type, { ...at, ...(Kind === PointerEvent ? { pointerId: 1, isPrimary: true, pointerType: 'mouse' } : {}) }));
```

Panel-IDs der Welt: `world` („3D“), `worldmap` („Weltkarte“), `matrix` („Raster“, nur in Tüfteln), `inspector`, `script`, `problems`. Prüfen: `tab.classList.contains('dv-active-tab')`, und für die Weltkarte `document.querySelector('[data-world-view]')`. Kein Umschalt-Klick (der lässt den Reiter schweben) und keine Ziehbewegung zwischen `pointerdown` und `pointerup`.

## Phase 3c ([phase-3c-gate.md](phase-3c-gate.md))

Kopien `pt-3c`, `pt-3c-raum`, `pt-3c-szenario` (Platin DE) und für (G) je eine frische Kopie der früheren Gates mit der Endung `-3c`. Die Schritte stehen in der Gate-Datei; hier nur, wer und wie.

| Punkt | Wer | Wie |
|---|---|---|
| (A) Welt bauen: Figur bearbeiten | Agent (Vergleich mit dem Screen: Nutzer) | Browser: Header 0411, Screenshot; Figur in „In dieser Szene“ wählen, „Blickrichtung“ ändern, ⌘Z, erneut ändern, speichern; `call.sh dirty` und `get_events '{"eventFileId":390}'` als Gegenprobe; „Auswählen“ und „Gelände malen“ getrennt; „Hier spielen“ baut nur |
| (B) Texte | Agent (Vergleich mit dem Screen: Nutzer) | Browser, Bereich Texte: Archiv 0023, Nachricht ändern, blättern (den `aria-label` „DS-Bildschirm, Seite x von y“ lesen), „Nachricht anlegen“ mit `Barry` im Text, speichern, Suche „Barry“ in „Alle“; `call.sh get_text_archive '{"index":23}'` |
| (C) Scripts ohne rotom-lsp | Agent | Browser: übernehmen, Script über den Explorer öffnen, Höhe der Gruppe des Scripts `document.querySelector('[data-dock-tab="script"]').closest("[data-dock-group]").getBoundingClientRect().height` ≥ 400 (bis 3e.3 `#dock-welt-hand`); `AddItemm` speichern: Prüflicht, Reiter „Probleme“, Klick springt zur Zeile |
| (C) Completion, Hover, Diagnosen | Hand | gebündelte App oder `pnpm tauri dev -- --bin studio` mit `G4_ROTOM_LSP` |
| (D) Assistent „Raum anlegen“ | Agent | Browser: Schrittleiste lesen, Escape mit getipptem Text zweimal, Durchlauf bis „Zeigen“; danach das Szenario `pt-raum-assistent.yaml` auf `pt-3c-szenario` |
| (E) Dunkel und Kompakt | Agent (Blick: Nutzer) | Screenshots der Galerie (alle Seiten) und von Welt und Texte in Dunkel und Kompakt; der Nutzer sieht auf unlesbare Stellen |
| (F) Tastatur | Agent | Tastendrücke über das Browser-Werkzeug (`key`), `document.activeElement` lesen; `:focus-visible` per `document.activeElement.matches(":focus-visible")` nach Tab (`true`) und nach Mausklick (`false`) |
| (G) Abnahmen 3a, 3b.1, 3b.2 | Agent (Stoppuhr, DSPRE, rotom-lsp: Hand) | die Tabellen der Abschnitte oben, mit den Namen aus „Doppeldeck: wo was ist“; „Patch reparieren“ über Technik › Patches |
| (G) Palette, Layout, Thema und Dichte | Agent | ⌘K, `0412`; `localStorage` lesen (`g4.layout.v11.*`, `g4.theme`, `g4.density`) und `document.documentElement.dataset` (`theme`, `dichte`) |
| (G) `scripts/gate.sh` | Agent | im Hintergrund, `> target/agent/gate.log`, aus einem Worktree mit absolutem `G4_ROMS_DIR`; die Tabelle am Ende übernehmen |
| (H) Gepackte App | Hand | `pnpm bundle`, Overlay-Titelleiste, Schriften, Thema folgt dem System |

## Phase 3b.3a ([phase-3b3-gate.md](phase-3b3-gate.md), Teil A)

Kopien `pt-3b3a-gate` (Oberfläche) und `pt-3b3a-szenario` (Szenario), beide Platin DE. Die Schritte stehen in der Gate-Datei; hier nur, wer und wie.

| Punkt | Wer | Wie |
|---|---|---|
| (A1) 1–3 `vanilla_data`, uxie-Orakel, Rebuild | Agent | laufen in `scripts/gate.sh` mit; einzeln mit den Namensfiltern aus der Gate-Datei, je Projekt mit `G4_TEST_PROJECT` und `G4_DSROM_CLI` (absolut), immer `cargo test --workspace --all-features` (siehe [dev-setup.md](dev-setup.md), „Gated Tests“) |
| (A1) 4 `scripts/gate.sh` | Agent | im Hintergrund, `> target/agent/gate.log`, aus einem Worktree mit absolutem `G4_ROMS_DIR`; die Tabelle am Ende übernehmen |
| (A2) Szenario `pt-daten.yaml` | Agent | `uv run g4emu scenario ../agent/scenarios/pt-daten.yaml --project …/pt-3b3a-szenario --out …/pt-daten` (siehe „Szenario“ oben); Rückgabe 0, `check.log` mit „4 expectation(s) met“, `start.png`; Gegenprobe mit einem falschen `--expect` (Exit 1) |
| (A3) Editoren gegen die Screens | Agent + Blick | Browser, Rezepte „Pokédex“, „Attacken“, „Gegenstände“; Screenshots der drei Editoren mit Staravia, Flügelschlag und Trank, Ort und Zahlen für den Nutzer; Gegenprobe der Edits mit `call.sh history` und `dirty` |
| (A4) ⌘Z über zwei Assets | Agent | Browser, Reiter „TM/VM“; `call.sh history` und `call.sh dirty` vor und nach dem einen ⌘Z |
| (A5) Hexblock in Tüfteln | Agent | Browser, Scharnier „Tüfteln“; `[data-marked]` und `data-offset` im DOM lesen, den Fuß des Hexblocks per `find` oder `get_page_text`; Gegenprobe `call.sh data_raw '{"kind":"move","index":17}'` |
| (A6) Item-Symbole | Agent + Blick | Browser: `[data-sprite="picture"]` in der Liste „Gegenstände“ zählen, `call.sh get_item_icon '{"item":17}'`; wie die Symbole aussehen, bestätigt der Nutzer am Bild |
| (A7) Renderzeit der Listen | Agent | Browser: Zeilen im DOM gegen `aria-setsize` zählen, Zeit mit `performance.now()` per `javascript_tool` messen; die Zahlen kommen in „Auffälligkeiten“ (das Browser-Fenster ist oft verborgen, die Zeiten sind dann eher zu hoch) |

## Phase 3b.3b ([phase-3b3-gate.md](phase-3b3-gate.md), Teil B)

Kopien `pt-3b3b` und `hg-3b3b` für die Szenarien (Endung `-2` …, wenn der Name existiert), `pt-3b3b-gate` und `hg-3b3b-gate` für die Oberfläche. Die Schritte stehen in der Gate-Datei; hier nur, wer und wie.

| Punkt | Wer | Wie |
|---|---|---|
| (B1) gated Tests, `scripts/gate.sh` | Agent | im Hintergrund, `> target/agent/gate.log`, aus einem Worktree mit absolutem `G4_ROMS_DIR` (rund 30 Minuten, gemessen 1798 s); die Tabelle am Ende übernehmen |
| (B2), (B3) Szenarien | Agent | `uv run g4emu scenario ../agent/scenarios/pt-trainer-begegnung.yaml` bzw. `hg-trainer-begegnung.yaml` (siehe „Szenario“ oben); Rückgabe 0, `check.log`, die JSON-Zeilen von `expect-battle` (Art, Level) je Kampf und die Bilder `trainer1`, `trainer2`, `wild` |
| (B4) Trainer | Agent | Browser, Rezept „Trainer“; Gegenprobe mit `call.sh data_get`, `history`, `dirty` |
| (B5) Begegnungen | Agent | Browser, Rezept „Begegnungen“, Schritte 1 bis 5 und 7 |
| (B6) Platzieren | Agent | Browser, Rezept „Platzieren“; `call.sh get_events` als Gegenprobe |
| (B7) ⌘Z | Agent | Browser: ⌘Z nach Feldänderungen, an der Grenze „Trainer N angelegt“ und nach dem Platzieren; `call.sh history` vor und nach |
| (B8) Abspalten | Agent | Browser, Rezept „Begegnungen“, Schritt 6 |
| (B9) Tastatur | Agent | Tastendrücke über das Browser-Werkzeug (`key`), `document.activeElement` lesen; Baum „Trainer“ (Pfeile, Pfeil rechts/links öffnet und schließt eine Gruppe), Liste „Begegnungen“, Reiter, Tabellen, Dialoge mit Escape |
| (B10) Dunkel und Kompakt | Agent + Blick | Screenshots von Trainer und Begegnungen in Dunkel und Kompakt, Galerie-Seite „Daten“ („Team-Karten“); der Nutzer sieht auf unlesbare Stellen |
| (B11) Screens „Trainer“ und „Begegnungen“ | Hand | Vergleich mit „Trainer: Wanderer Kai“, „Neuer Trainer: Team“ und „Begegnungen: Route 201“ |
| (B12) DSPRE | Hand | DSPRE läuft nur unter Windows |
| (B13) HeartGold US im Spiel | Agent (optional) | `hg-trainer-begegnung.yaml` mit `game: IPKE` (Kopie der Datei, `--set NEUER_HEADER=540`) auf einer frischen Kopie von HeartGold US; IPKE hat keinen gemessenen Kampfzeiger, der Lauf 2026-10-24 war trotzdem grün |

## Phase 3b.4a ([phase-3b4-gate.md](phase-3b4-gate.md), Teil A)

Kopien `pt-3b4a` (Szenario) und `pt-3b4a-gate` (Oberfläche), beide Platin DE. Die Schritte stehen in der Gate-Datei; hier nur, wer und wie.

| Punkt | Wer | Wie |
|---|---|---|
| (A1) 1–2 `vanilla_checks`, Zeitmessungen | Agent | laufen in `scripts/gate.sh` mit; einzeln je Projekt mit `G4_TEST_PROJECT` (absolut) und `cargo test --workspace --all-features -- vanilla_checks_match_the_fixed_counts vanilla_script_checks_after_adopt full_run_on_vanilla overlay_under_lock --nocapture --test-threads 1` (siehe [dev-setup.md](dev-setup.md), „Projektprüfungen“) |
| (A1) 3 `scripts/gate.sh` | Agent | im Hintergrund, `> target/agent/gate.log`, aus einem Worktree mit absolutem `G4_ROMS_DIR`, `G4_APICULA`, `G4_ROTOM_LSP` (rund 30 Minuten); die Tabelle am Ende übernehmen |
| (A2) Szenario `pt-pruefungen.yaml` | Agent | `uv run g4emu scenario ../agent/scenarios/pt-pruefungen.yaml --project …/pt-3b4a --out …/pt-pruefungen` (siehe „Szenario“ oben); Rückgabe 0 und 22 Zeilen „Erwartung erfüllt“ auf stderr; danach `target/debug/g4k check <Kopie> --expect warp.dest-warp=0 --expect event.script-missing=0 --expect event.trainer-missing=0 --expect text.too-wide=0` |
| (A3) Prüflicht und Panel | Agent | Browser, Rezept „Prüfen“; Gegenprobe der Zahlen mit `call.sh checks_status` |
| (A4) Fix-Dialog | Agent | Browser, Rezept „Beheben“; Gegenprobe mit `call.sh history`, `dirty`, `read_script` |
| (A5) Ausblenden und Ausgangslage | Agent | Browser, Rezepte „Ausblenden“ und „Ausgangslage“; `g4-checks.json` mit `cat` lesen |
| (A6) 1–2 Inspector und Explorer | Agent | Browser: „Öffnen“ an `warp.dest-warp`, Abschnitt „Prüfungen“ im Inspector (`[role=list][aria-label=Prüfungen]`), `[data-header-id] [data-finding-mark=error]` im Explorer |
| (A6) 3 Fehlerfarbe am Warp-Marker | Agent + Blick | Browser: Header 0415 in der 3D-Ansicht, Treppe im Blick; die Farbe liest der Agent (Szene oder `javascript_tool`, `#DE3142` = `szene/status/fehler`), der Nutzer bestätigt am Bild, dass die Treppe rot ist und nach dem Beheben wieder in der Warp-Farbe |
| Gebündelte App | Hand (optional) | Prüflicht nach dem Öffnen und nach einer Änderung im nativen Fenster (`pnpm bundle`, siehe [release.md](release.md)); dieselbe Logik wie im Browser-Modus |

## Phase 3b.4b ([phase-3b4-gate.md](phase-3b4-gate.md), Teil B: 3b-Gesamtgate)

Kopien `pt-3b4b` und `hg-3b4b` (Szenarien, optional `hgus-3b4b`), `pt-3b4b-kontrolle` und `hg-3b4b-kontrolle` (Gegenproben), `pt-3b4b-gate` (Oberfläche), je frisch; existiert ein Name, mit Endung `-2` …. Die Schritte stehen in der Gate-Datei; hier nur, wer und wie.

| Punkt | Wer | Wie |
|---|---|---|
| (B1) gated Import-Tests, `scripts/gate.sh` | Agent | im Hintergrund, `> target/agent/gate.log`, aus einem Worktree mit absolutem `G4_ROMS_DIR`, `G4_APICULA`, `G4_ROTOM_LSP`; die Tabelle am Ende übernehmen. Die Import-Tests einzeln je Projekt mit `G4_TEST_PROJECT` (absolut) und `cargo test --workspace --all-features -- every_vanilla_model_reimported map_a_into_b hgss_bin_into_pt per_round_trip --nocapture` (siehe [dev-setup.md](dev-setup.md), „Kartenteile importieren (3b.4b)“) |
| (B2), (B3) Szenarien | Agent | Rezept „Gesamtgate“, Schritte 1 und 2; Rückgabe 0, 37 Zeilen „Erwartung erfüllt“, `check.log`, die JSON-Zeilen von `walk-to` (Weg), `expect-battle` und `win-battle` |
| (B2) 10, (B3) 6 Gegenproben ohne Import | Agent | Rezept „Gesamtgate“, Schritt 3; das Szenario muss mit „blockiert bei …“ scheitern |
| (B3a) HeartGold US | Agent (optional) | Rezept „Gesamtgate“ mit einer Kopie der Datei mit `game: IPKE`; IPKE hat keinen gemessenen Kampfzeiger |
| (B4) Bilder: Route texturiert, Nachbar, Geber | Agent + Blick | die Bilder `weg.png`, `nachbar.png`, `nachbar_danach.png`, `geschenk.png` (und `trainer1.png`) beider Läufe ansehen und mit Pfad, Ort und Inhalt beschreiben; der Nutzer bestätigt am Bild. `g4emu` liest weder Text noch Flags noch Texturen |
| (B5) Dialog „Teile importieren…“ | Agent | Browser, Rezept „Teile importieren“, Schritte 1, 2 und 4 (Prüfliste, Befunde der Karte nach dem Import, deutsche Fehlertexte); Gegenprobe mit `call.sh inspect_map_import` |
| (B6) „Teile exportieren…“ | Agent | Browser, Rezept „Teile importieren“, Schritt 3 (vorher `mkdir -p <Kopie>/export`); `ls -l` bzw. `wc -c` der Dateien |
| (B7) Szene, Vorschaubild, ⌘Z nach dem Import | Agent | Browser, Rezept „Teile importieren“, Schritt 2; Hintergrundbild der Zelle per `javascript_tool`, die Karte in der Szene per Rechts-Ziehen (über `javascript_tool`) und Hover in der Statusleiste, `call.sh history` und `dirty` vor und nach ⌘Z |
| (B8) DSPRE | Hand | DSPRE läuft nur unter Windows; die Szenario-Kopien `pt-3b4b`, `hg-3b4b` öffnen |
| (B9) Nativer Dateidialog | Hand | gebündelte App (`pnpm bundle`, siehe [release.md](release.md)): „Dateien wählen…“ im Import-Dialog, „Pfad wählen…“ im Export-Dialog; im Browser-Modus gibt es beide Tasten nicht (dort Pfade tippen) |

## Phase 3h.2 ([phase-3h-gate.md](phase-3h-gate.md), Teil 3h.2: Schreiben und Batch)

Kopien `3h-pt-route` (Platin DE) und `3h-hg-route` (HeartGold DE), je frisch aus `…_DSPRE_contents` unter `$G4_ROMS_DIR/_testkopien` (Ziel darf nicht existieren). Die Schritte stehen in der Gate-Datei; hier nur, wer und wie.

| Punkt | Wer | Wie |
|---|---|---|
| (a), (b) Szenarien `pt-mcp-route.yaml`, `hg-mcp-route.yaml` | Agent | Aufruf wie in „Szenarien über MCP“; Rückgabe 0, die Zeilen „Erwartung erfüllt“ auf stderr (darunter `history`, `undo`, `redo` des Batchs), `mcp.jsonl`, `check.log`, die JSON-Zeilen von `walk-to`, `expect-battle` und `win-battle` |
| (c) Batch ist ein Undo-Schritt | Agent | die Erwartungen der Szenarien zu `history` und `undo` und der Test `batch_over_stdio_is_one_undo_step` (`cargo test -p studio --test mcp_stdio batch_over_stdio_is_one_undo_step`) |
| (d) Unumkehrbares nur mit `--allow-irreversible` | Agent | `cargo test -p studio --test mcp_stdio irreversible run_rom build_inside` und gated `vanilla_trainer_and_builds_over_stdio` (siehe [dev-setup.md](dev-setup.md), „MCP (`studio mcp`)“) |
| (e) Bilder der Szenarien | Agent + Blick | die Bilder beider Läufe ansehen und mit Pfad, Ort und Inhalt beschreiben; der Nutzer bestätigt am Bild. `g4emu` liest weder Text noch Flags noch Texturen |
| (f) `scripts/gate.sh` mit Schritt `mcp` | Agent | im Hintergrund, `> target/agent/gate.log`, aus einem Worktree mit absolutem `G4_ROMS_DIR`, `G4_APICULA`, `G4_ROTOM_LSP`; die Tabelle am Ende übernehmen |

## Phase 3h.3 ([phase-3h-gate.md](phase-3h-gate.md), Teil 3h.3: In der laufenden App)

Im Browser-Modus geprüft, Test-Umgebung auf eigenen Ports (`G4_VITE_PORT=1480 G4_BRIDGE_PORT=1481`), Browser-Fenster in einem eigenen Reiter, Kopie `3h3-pt` frisch aus Platin DE. Reihenfolge des Laufs: (a) Schalter aus und an, (f) Nachfrage, (g) Expertenwerkzeuge, Unumkehrbares, (b) Gebiet, (c) Verlauf, (d) ⌘Z, (e) „Nur lesen“, „Trennen“. Der Header-Patch aus „Unumkehrbares“ muss vor (b) liegen (ein neuer Header braucht ihn).

| Punkt | Wer | Wie |
|---|---|---|
| (a) Schalter, 503, `mcp.json` | Agent | `mcp.sh tools` vor dem Einschalten (HTTP 503), Schalter im Browser-Fenster, `call.sh agent_status`; App-Seite über `cargo test -p studio --all-features --test mcp_http` und die Bibliothekstests `mcp::http`, `mcp::discovery` |
| (b) Gebiet über `/mcp` | Agent + Blick | Welt-Explorer vorher ablesen (Zeile „N Orte, M Header“), `mcp.sh call create_area` mit `spec` (Name, Zellen mit `source_map`, `with_headers`, `header` mit `new_header`), danach ohne Neuladen Explorer, Titelleiste („N Änderungen nicht gespeichert“) und Verlauf ablesen, den neuen Eintrag anklicken, Szene und Inspector beschreiben; `extend_area`, während das Gebiet zu sehen ist |
| (c) Verlauf mit Client-Namen | Agent + Blick | Verlaufsmenü im Scharnier öffnen (Einträge mit Roboter-Symbol und „· agent-gate“), `call.sh history` (`undo_agents`) |
| (d) ⌘Z | Agent + Blick | erst in die Seite klicken (Fokus), dann `cmd+z` je Schritt; `call.sh history` (`redo`, `redo_agents`); Szene beschreiben; `cmd+shift+z` zurück |
| (e) „Nur lesen“ | Agent | Menü `[data-agent-status]` in der Statusleiste, Eintrag „Nur lesen“; Text der Statusleiste ablesen; `mcp.sh call set_matrix_cell …` muss mit Rückgabe 1 und `read_only` enden, `get_matrix` unverändert |
| (f) Nachfrage | Agent | ohne Freigabe (frischer `target/agent/app-data` oder neue Kopie): `mcp.sh call get_header '{"id":0}'` im Hintergrund, `call.sh agent_status` zeigt `question`, Dialog `[data-agent-question]` ablesen, „Lesen“ klicken; danach `agents.json` und ein schreibender Aufruf (`read_only`) |
| (g) `g4_call` im Standard aus | Agent | `mcp.sh tools` (38 Werkzeuge, ohne `g4_call`), `call.sh agent_set_expert '{"on":true}'`, `mcp.sh tools` (40, die Liste wird je Anfrage gebildet), wieder aus; Tests `fresh_hub_has_expert_off`, `shared_tools_list_omits_g4_call_by_default` |
| (h) `studio mcp` ohne `--project` | Agent | `cargo test -p studio --all-features --test mcp_proxy`; `target/debug/studio mcp --app-data <leerer Ordner>` (Rückgabe 2) |
| (i) gepackte App mit Claude Code | Hand | Ablauf in der Gate-Datei; der Agent ändert die Claude-Konfiguration des Nutzers nicht |
| Unumkehrbares, „Trennen“ | Agent | Freigabe „Lesen und bearbeiten“ im Segment, `mcp.sh call apply_header_patch '{}'` im Hintergrund, Dialog „Agent möchte etwas Unumkehrbares tun“, „Zulassen“; „Trennen“ im Menü der Statusleiste, dann ein Aufruf: die Nachfrage kommt neu |
| 3h.4 Ressourcen | Agent | `mcp.sh read g4://glossary`, `mcp.sh read g4://docs/formats/matrix` |

## Was beim Nutzer bleibt

- **DSPRE-Gegenprobe** (3b.1 (A) 12–15) und **Diamant/Perl** (17): DSPRE läuft nur unter Windows.
- **Stoppuhr und „Zahlen getippt“** (3a (A)): misst die Bedienung durch den Menschen.
- **Natives Fenster:** ein kurzer Durchgang mit der gebündelten App nach `pnpm bundle`; in 3c ist das Teil (H) (Overlay-Titelleiste, Schriften, Thema).
- **rotom-lsp:** Completion, Hover und Diagnosen im Editor (3b.2 (B), (C), 3c (C)); im Browser-Modus läuft der Sprachdienst nicht.
- **3b.3b:** der Blick auf die Screens „Trainer: Wanderer Kai“, „Neuer Trainer: Team“ und „Begegnungen: Route 201“ (B11) und die DSPRE-Gegenprobe der Szenario-Kopie (B12). HeartGold US im Spiel (B13) hat der Agent in der Abnahme mit dem Szenario erledigt (`game: IPKE` in einer Kopie der YAML-Datei).
- **„Agent + Blick“:** 3a (F) Ausrichtung der Kartenbilder; 3b.1 (B) Höhen und (C) Sprites (aufrecht, nicht gespiegelt, Blickrichtung); 3c (A) und (B) der Vergleich mit den Screens des Design-Systems, 3c (E) dunkles Thema und Kompakt; 3b.3a (A3) Pokédex, Attacken und Gegenstände gegen ihre Screens, (A6) die Item- und Pokémon-Symbole; 3b.3b (B10) Trainer und Begegnungen in Dunkel und Kompakt; 3b.4a (A6) 3 die rote Treppe in der Szene; 3b.4b (B4) die Bilder der Gesamtgate-Szenarien (texturierte Route `weg.png`, Nachbar mit Trank und zweitem Text, Geber). Der Agent nennt dafür Ort und Zahlen, damit der Blick kurz bleibt.
- **3h.1:** Claude Code per `claude mcp add` auf `_testkopien/3h-lesen` (a): der Agent ändert die Claude-Konfiguration des Nutzers nicht; stellvertretend läuft die Handprobe über JSON-RPC/stdio.
- **3h.2:** der Blick auf die Bilder der MCP-Szenarien (e).
- **3h.3:** die gepackte App mit Claude Code im selben Projekt (i): Einrichtungszeile kopieren, `claude mcp add`, Nachfrage, Gebiet anlegen, Verlauf, ⌘Z, „Nur lesen“, „Trennen“. Dazu der Blick auf (b) bis (d): das neue Gebiet im Welt-Explorer und in der Szene, die Einträge mit Roboter-Symbol und Client-Namen im Verlaufsmenü und die Szene nach ⌘Z; und auf Einstellungen › Agenten (seit der Abnahme zwei Spalten breit, die Segmente der Freigabe in einer Zeile) und die Agenten-Zeile der Statusleiste bei 1440 × 900 und 1100 × 720; dazu von Hand ⌘Q und Ausschalten in der gepackten App (löschen `mcp.json`). Liste mit Ort: [phase-3h-gate.md](phase-3h-gate.md), „Beim Nutzer offen (3h.1 bis 3h.4)“.
- **3b.4b:** DSPRE öffnet die Szenario-Kopie nach dem Import (B8) und der native Dateidialog in der gebündelten App (B9).
- **3f:** die Bilder der Einblendung „Neuheim“ und „Neuweg“ (G.3, 2.8, 4.2), der Zellrahmen und die Draufsicht im Bild (1.7, 1.8), ein im neuen Gebiet gefangenes Pokémon mit dem Fundort „Neuheim“ (G.4) und DSPRE mit Patch und Ortsnamen (G.5).
- **4.2 (G7):** „Hier spielen“ in der gepackten App auf HeartGold spürbar schneller, die DSPRE-Gegenprobe (ein Ordner wie `pokeHeight` in DSPRE geändert, mit dem Studio gebaut, die Änderung ist im Spiel; dazu die offene Frage, ob DSPRE selbst packt) und der Blick auf die Schritte-Tabelle in Technik › Prüfen und Bauen hell, dunkel und kompakt.
- **3g:** DSPRE öffnet den angelegten Ordner (A5) und ein mit der Vorlage angelegtes Projekt (B6), die gebündelte Tauri-App mit Dateidialog, Finder-Drop und Vorschlag (A6) sowie der Blick auf die Bilder der Szenarien (B2: `b2-pt`, `b2-hg`) und die Screens des Assistenten.
- **4.1a:** Rom Patcher JS im Browser mit der eigenen Basis-ROM und einem Patch aus (2), die SHA-1 des Ergebnisses gegen die Begleitdatei; xdelta UI unter Windows; der Blick auf die 13 Bilder `blick-veroeffentlichen-*.png`. Liste mit Ort: [phase-4-1a-gate.md](phase-4-1a-gate.md), Abschnitte „Hand“ und „Blick“.
- **4.1b:** das Kontextmenü in der echten App (Zwei-Finger-Klick auf dem Trackpad, Shift+F10 in WebKit, die Kontextmenü-Taste mit einer PC-Tastatur), die Fortschrittszahlen von Technik › Änderungen in der App (die Brücke hat keinen `Channel`), ein Urteil über die englischen Feldnamen in den Vorschau-Zeilen; der Blick auf die 17 Bilder `blick-*.png` (Dialog Karte, Header, Script und Technik › Änderungen in Hell, Dunkel, Kompakt). Liste mit Ort: [phase-4-1b-gate.md](phase-4-1b-gate.md), Abschnitte „Hand“ und „Blick“.
- **4.3b:** die DSPRE-Gegenprobe (DSPRE öffnet einen eingerichteten Ordner, speichert eine Änderung, die App übernimmt sie beim Fokus; was DSPRE dabei unter `unpacked/` und `expanded/` schreibt), der Fensterfokus in der gepackten Tauri-App (`onFocusChanged`), die Fortschrittsphasen von „Einrichten…“ und die laufende Phase „Textprojekt einrichten“ im Assistenten (die Brücke hat keinen `Channel`); der Blick auf die Bilder des UI-Tests (`blick-*.png`, `gate3-*.png`, `mcp-*.png`) in Hell, Dunkel und Kompakt. Liste mit Ort: [phase-4-3b-gate.md](phase-4-3b-gate.md), Abschnitte „Hand“ und „Blick“.
- **4.4:** das eigene Git-Werkzeug des Nutzers zeigt ein eingerichtetes Repo mit dem Commit „Ausgangsstand“ und ohne Remote; „Git einrichten“ in der gepackten App, die aus dem Finder gestartet wurde (findet Git trotz kurzem `PATH`); die Fortschrittsphasen von „Git einrichten“, der Phase „Git einrichten“ im Assistenten und „Textprojekt vervollständigen“ (die Brücke hat keinen `Channel`); Windows (`find_git` mit `%ProgramFiles%`, `core.autocrlf`); der Blick auf die 59 Bilder `blick-*.png` (Technik › Änderungen mit drei Vergleichen und den Formen, die Dialoge, die Statusleiste in Hell, Dunkel, Kompakt; der Kontrast von `<ins>`/`<del>` in Dunkel). Liste mit Ort: [phase-4-4-gate.md](phase-4-4-gate.md), Abschnitte „Hand“ und „Blick“.
- **3d.1:** ein eigener Flag-Name in DSPRE (Textprojekt zurückgenommen, DSPRE zeigt weiter die pret-Namen bzw. Nummern), `expect-flag` auf HeartGold US (`IPKE` nur in `games.yaml` übernommen, nicht gemessen); der Blick auf Fläche, Picker und Dialog in Hell, Dunkel und Kompakt. Liste mit Ort: [phase-3d-1-gate.md](phase-3d-1-gate.md), Abschnitte „Hand“ und „Blick“.
- **3d.3 (Teil A):** die Kampfbilder gegen das Spiel im Emulator (Pokédex und Kampf; vor allem Kinoso Wolken- und Sonnenform, Burmy und Burmadame schillernd, HeartGold-Pichu, die HeartGold-Trainerbilder), der Blick auf Profil, Trainer-Kopf und Kampfvorschau in Hell, Dunkel und Kompakt (U2, U3) und die Frames in der `.fig`. Liste mit Ort: [phase-3d-3-gate.md](phase-3d-3-gate.md), „(4) Hand und Blick“.
- **3d.3 (Teil B):** eine Ei-Attacke in Platin im Spiel wirksam (Zucht nach einem Tausch innerhalb des Budgets), ein echtes, mit DSPRE gepatchtes Platin-Projekt (Patch-NARC, Klon und Wiederherstellung; getestet ist nur der synthetische NARC von `3d-3-patch`), der Blick auf die Bilder `blick-pokedex-eiattacken-*.png` in Hell, Dunkel und Kompakt und der Frame in der `.fig`. Liste mit Ort: [phase-3d-3-gate.md](phase-3d-3-gate.md), Abschnitt „Teil B“, „(4) Hand und Blick“.
- **4.5:** Rom Patcher JS im Browser mit der eigenen Basis-ROM und einem Patch aus (2), die SHA-1 des Ergebnisses gegen die Begleitdatei; „Im Finder zeigen“ in der gepackten App (macOS markiert die Datei, Linux öffnet nur den Ordner, Windows ungeprüft); das eigene Git-Werkzeug zeigt den Commit „Veröffentlicht v0.1.0“ mit Body und ohne Remote; der Blick auf Titelleiste, Assistent, Fläche und Dialog „Stand sichern?“ in Hell, Dunkel, Kompakt. Liste mit Ort: [phase-4-5-gate.md](phase-4-5-gate.md), Abschnitte „Hand“ und „Blick“.

## Phase 3g im Browser-Modus ([phase-3g-gate.md](phase-3g-gate.md), Teil A und B)

3g.1 legt ein Projekt aus einer ROM an, ohne dass ein Projekt offen ist. Im Browser-Modus gelten zwei Regeln: Die ROM muss unter `_testkopien` (allgemein: unter einer erlaubten Wurzel) oder unter `G4_IMPORT_ROOT` liegen, und das Ziel heißt **ohne** die Endung `_DSPRE_contents` (die Brücke behandelt sie als Original und lehnt sie ab: „Originale (_DSPRE_contents) sind gesperrt: … Im Browser-Modus ohne die Endung anlegen.“). Der Elternordner des Ziels muss existieren und unter einer Wurzel liegen; das Ziel selbst fehlt oder ist ein leerer Ordner. Die Originale unter `Roms/` bleiben tabu, auch als ROM-Eingabe.

**Die vier Brückenbefehle** (Argumente camelCase wie in `apps/studio/src/project/api.start.ts`; Felder der Antworten snake_case):

```sh
scripts/agent/call.sh inspect_rom '{"path":"<Roms>/_testkopien/3g-gate-a/pt.nds"}'     # title, game_code, family, language, size, sha1, known_base, support
scripts/agent/call.sh default_project_parent                                          # {parent, dspre_suffix}: im Browser die erste Wurzel (_testkopien), dspre_suffix false
scripts/agent/call.sh check_project_dest '{"dest":"<Roms>/_testkopien/3g-gate-a/Gate Platin","romSize":134217728}'   # {state, free_bytes, needed_bytes, ok}; needed = 2,5 × romSize
scripts/agent/call.sh create_project '{"rom":"<Roms>/_testkopien/3g-gate-a/pt.nds","dest":"<Roms>/_testkopien/3g-gate-a/Gate Platin","template":"original"}'   # {root, game_code, template, sha1, phases:[{phase, ms}]}
```

`support` ist `verified`, `untested`, `locked` oder `unknown` (die beiden letzten mit `reason`); `template` ist `original` oder `data_only`; `state` ist `missing`, `empty`, `not_empty`, `not_a_directory` oder `unreadable`. `inspect_rom` liest den Kopf und rechnet den SHA-1 der ganzen Datei. `create_project` entpackt im Debug-Build rund 1 s (Platin, gemessen; die ganze Anlage ist nicht eigens gemessen), öffnet das Projekt nicht und liefert keinen Live-Fortschritt, nur `phases` mit der Dauer je Phase (`read`, `extract`, `template`, `check`); der Assistent zeigt bis zur Antwort „Lesen läuft“. Das Projekt öffnet danach `call.sh open_project '{"path":"<root>"}'` oder die Taste „Öffnen“. Gegenproben: eine Kopie mit Spielcode `ADAD` (Diamant/Perl) → „Diamant/Perl: ohne Testprojekt nicht freigegeben“; ein Ziel mit Inhalt → „destination … is not empty“; ein Ziel mit `_DSPRE_contents` → die Sperre oben.

**Wo der Assistent sitzt:** Auf Start (ohne `?project=` öffnet die Seite dort) ist „Neues Spiel aus ROM…“ die primäre Taste; der Assistent läuft in einem eigenen kleinen Deck (`[data-start-deck]`: Schau `[data-start-schau]`, Scharnier mit `[data-wizard-bar]`, Hand mit `[data-wizard-hand]`), nicht im Deck der Welt. Er hat die Schritte ROM, Vorlage, Name und Ziel, Zusammenfassung; „Weiter“ und „Anlegen“ sind Tasten des Scharniers. Im Browser steht im Schalter „Endung _DSPRE_contents (für DSPRE)“ der Vorschlag aus; lässt man ihn aus, entsteht `<Ordner>/<Name>`. Ablauf mit der Seite (Rollen und Namen aus [phase-3g-gate.md](phase-3g-gate.md)): Seite `http://127.0.0.1:<Vite-Port>/`, `localStorage.clear()` und neu laden; ROM per Drop (unten) oder im Feld „ROM-Datei“ (Pfad eintippen, Enter oder Fokus verlassen liest sie); Karte „Original“ oder „Nur Daten“ (`[data-template=…]`); „Name“ und „Ordner“; „Anlegen“; die Zeilen von „Ablauf“ tragen `data-phase`; „Öffnen“ lädt das Projekt (bei „Nur Daten“ im Bereich Pokémon).

**Drop im Browser:** Das Browser-Fenster liefert keinen echten Datei-Drop mit Pfad. Der Agent schickt ein synthetisches `DragEvent` an die Fläche `[data-drop]` mit dem Pfad als `text/uri-list` (`file://` mit prozentkodierten Zeichen) oder als `text/plain` (absoluter Pfad):

```js
const dt = new DataTransfer();
dt.setData('text/uri-list', 'file:///Pfad/zu/Roms/_testkopien/3g-gate-a/pt.nds');
const surface = document.querySelector('[data-drop]');
for (const type of ['dragenter', 'dragover', 'drop']) surface.dispatchEvent(new DragEvent(type, { dataTransfer: dt, bubbles: true, cancelable: true }));
```

`dragenter` und `dragover` sind nötig, damit `[data-drop]` auf `over` wechselt (der Rahmen „ROM hier ablegen“); `drop` öffnet den Assistenten mit dem Pfad, und nach erfolgreichem `inspect_rom` einer freigegebenen ROM springt er zu „Vorlage“. Eine nicht freigegebene (nur `untested`) bleibt auf „ROM“, damit man die Warnung liest. Mehr als eine Datei, mehrere Zeilen oder eine andere Endung als `.nds` zeigt unter den Tasten „Nur eine .nds-Datei ablegen.“; eine echte Datei ohne Pfad öffnet den Schritt ROM mit „Im Browser-Modus kennt die Seite den Pfad der Datei nicht. Gib ihn ein.“

**Zuordnung:**

| Punkt | Wer | Wie |
|---|---|---|
| (A1) `project_start_*` gated | Agent | `G4_TEST_ROM=<Kopie> cargo test --workspace --all-features -- project_start_` je Kopie (Platin DE, HeartGold DE, HeartGold US) |
| (A2) ADAD und nicht leeres Ziel abgelehnt | Agent | `call.sh inspect_rom` und `create_project` mit den Gegenproben oben |
| (A3) Start, Drop, Schritte, „Öffnen“ | Agent + Blick | Browser, Ablauf oben; der Agent nennt Ort und Zahlen (Platzzeile, SHA-1, Header-Zahl, Bereich nach „Öffnen“), der Nutzer sieht Start und den Assistenten kurz an; der Neubau mit `g4k build` hat denselben SHA-1 wie die Eingabe |
| (A4) Ziel mit `_DSPRE_contents` | Agent | `call.sh create_project` mit einem solchen Ziel, Fehler „Originale (_DSPRE_contents) sind gesperrt …“ |
| (A5) DSPRE | Hand | DSPRE läuft nur unter Windows; einen mit der Endung angelegten Ordner öffnen |
| (A6) Tauri-App | Hand | gebündelte App: Dateidialog mit `.nds`-Filter, Drop aus dem Finder, Vorschlag `~/Documents/Gen4 Studio` |

**Teil B (3g.2, leere Region):** Die Befehle `plan_empty_region` und `apply_empty_region` laufen über `call.sh` (der Plan sagt `already_empty`, Start-Header, behaltene und geleerte Orte, Randzellen). In Technik › Werkzeuge hat die Karte `[data-tool=welt-leeren]` die Taste „Welt leeren…“, die den Assistenten „Welt leeren“ im Scharnier der Welt öffnet (Schritte „Was bleibt“, „Zusammenfassung“, Ergebnis „Welt geleert“; die Scripts müssen übernommen sein, sonst bietet er „Scripts übernehmen“ an). Rückgängig geht über die Taste im Scharnier oder mit ⌘Z, wenn der Fokus in der Seite liegt (in der Abnahme: erst in die Hand klicken, dann `cmd+z`); ohne Fokus erreicht ⌘Z aus dem Browser-Werkzeug die Seite nicht. Im Assistenten „Neues Spiel aus ROM“ ist `[data-template=empty_region]` die Karte der Vorlage. Die Szenarien `pt-leere-region.yaml` und `hg-leere-region.yaml` stehen oben unter „Neues Spiel ohne Spielstand“ und im Abschnitt Szenario; die Test-Umgebung für B3 lief auf den Ports 1462 und 1463.

| Punkt | Wer | Wie |
|---|---|---|
| (B1) gated Kernel- und Brückentests der Vorlage | Agent | `G4_TEST_ROM=<Kopie von 3g-probe/*.nds> cargo test --workspace --all-features -- <Test>` je ROM (Platin DE, HeartGold DE), dazu `g4k template <Kopie> empty-region` mit `g4k check` |
| (B2) Szenarien `pt-leere-region.yaml`, `hg-leere-region.yaml` | Agent + Blick | `g4emu scenario` auf einer frischen `g4k create`-Kopie; der Agent nennt Ort und Zahlen, der Nutzer sieht die Bilder in `b2-pt` und `b2-hg` an |
| (B3) „Welt leeren…“ und die Vorlage im Assistenten | Agent + Blick | Browser (Ports 1462/1463), Ablauf wie oben; der Agent nennt Texte, Zahlen und Statusleiste, der Nutzer sieht die Screens kurz an |
| (B4) `scripts/gate.sh` | Agent | `G4_ROMS_DIR=<absolut> G4_APICULA=<absolut> G4_ROTOM_LSP=<absolut> bash scripts/gate.sh` (rund 55 Minuten) |
| (B5) Gegenprobe Original | Agent | `call.sh plan_empty_region` auf einer Original-Kopie: `already_empty` false, Figuren in der Stadt, Weg hinaus |
| (B6) DSPRE | Hand | ein mit der Vorlage und der Endung angelegtes Projekt in DSPRE öffnen |

## Phase 3f ([phase-3f-gate.md](phase-3f-gate.md))

3f baut das Weltmodell aus: 3f.1 Raster nach Tüfteln, 3f.2 Eigene Orte, 3f.3 Leichter Patch, 3f.4 Nahtlos und Abspalten. Die Punkte und die Ergebnisse stehen in der Gate-Datei; hier nur, wer sie prüft und wie. Wo die Dinge in der Oberfläche liegen, sagt „Doppeldeck: wo was ist“ (Zeilen „Welt, Schau“, „Welt, Reiter „Weltkarte““, „Welt, Hand“, „Assistent im Scharnier“).

**Test-Umgebung aus dem Worktree:** eigene Ports und absolutes `G4_ROMS_DIR`, etwa `G4_ROMS_DIR=<absoluter Pfad zu Roms> G4_VITE_PORT=1470 G4_BRIDGE_PORT=1471 bash scripts/agent/studio-web.sh > target/agent/studio-web.log 2>&1 &`, warten auf „Brücke läuft“, Seite `http://127.0.0.1:1470/?project=<Kopie>`. **Nach jeder Änderung am Rust-Code die Umgebung neu starten**: die Brücke läuft sonst mit dem alten Stand (im Gate lehnte `set_matrix_cell` eine Höhe noch mit „the matrix has no altitude section“ ab, bis die Brücke neu gebaut war). Vor jedem Teil `localStorage.clear()` und neu laden; Fenster 1440 × 900, Hell, Komfort, Bauen. Kopien je Teil frisch aus `…_DSPRE_contents` (Anlegen, Patch und Abspalten lassen sich nicht zurücknehmen).

| Teil | Punkte | Wer | Wie |
|---|---|---|---|
| 3f.1 Raster nach Tüfteln | (1.1) Reiter je Tiefe, (1.3) Umschalter, (1.4) Gebiet und Karte auf der Weltkarte, (1.5) „+“, (1.6) Raumplan, (1.9) Höhe in Bauen | Agent | Browser: Reiter `[data-dock-tab]` je Tiefe zählen, Weltkarte nach vorn (Rezept „Dockview-Reiter im Browser umschalten“), `[data-world-view]`, `[data-cell]`, `[data-extend]`, `[data-more-area]` wie in der Zeile „Welt, Reiter „Weltkarte““; Höhe über das Feld „Höhe“ im Inspector „Karte“, Gegenprobe `call.sh get_matrix '{"index":7}'` (`altitudes`) und `call.sh undo` |
| | (1.2) DOM-Scan Bauen | Agent | Rezept „DOM-Scan Bauen“, seit 3f.1 ohne Ausnahme |
| | (1.7) Karte in 3D wählen, (1.8) Draufsicht | Agent + Blick | Klick auf den Boden mit „Auswählen“; Inspector, Brotkrume und Statusleiste prüft der Agent, den Rahmen um die Zelle und die Ansicht von oben sieht der Nutzer am Bild; `aria-pressed` der Taste „Draufsicht“ und `JSON.parse(localStorage['g4.shell.v1']).topDown` |
| | (1.10) Ladezeit | Agent | Rezept „Ladezeit messen“, vorher auf dem Stand `974a545` (eigener Worktree), nachher auf dem Branch |
| 3f.2 Eigene Orte | (2.1) Picker „Ort“, (2.2) Neuer Ort, (2.3) Undo und Redo, (2.4) lange Namen, (2.5) Umbenennen | Agent | Browser: `button[data-place-picker]`, Picker „Ort wählen“; „+“ in „In dieser Szene“ › „Neuer Ort…“, Dialog `[data-new-place]`; Gegenprobe mit `call.sh history` (`["Ort „Neuheim“ angelegt"]`) und `call.sh dirty` (TextArchive 433, HeaderTable); ⌘Z/⌘⇧Z mit dem Fokus in der Seite |
| | (2.6) Plätze, (2.7) Prüfungen | Agent | `call.sh place_slots` (Platin 126 Namen, 130 frei; HeartGold DE Bank 279, 235 Namen, 21 frei, `reusable`), `call.sh run_checks`; gated `vanilla_checks` (`G4_TEST_PROJECT=<Kopie> cargo test -p g4-kernel --all-features --test vanilla_checks`) auf Platin DE, HeartGold DE und US |
| | (2.8) Im Spiel | Agent + Blick | Szenario `pt-neuer-ort.yaml` (unten), Bild `einblendung` |
| 3f.3 Leichter Patch | (3.1) bis (3.7) | Agent | Browser in Bauen, ohne den Reiter „Raster“ und ohne Technik: „Musik Tag“ +1 (ungespeichert), „Neu anlegen“ › „Gebiet anlegen…“, Schritt „Ort“, Patch-Zeile `[data-patch-consent]` ankreuzen, „Anlegen“; dann `call.sh header_patch_status` (`applied`), `history`, `dirty`, die Sicherung unter `<Kopie>/.g4/backup/`, ⌘Z bis vor die Header-Änderung und ⌘⇧Z zurück, ⌘S; `visibleWords` in jedem Schritt ohne „Raster/Matrix/Header“ |
| | (3.8) Tüfteln | Agent | vitest `shows_the_step_place_in_bauen_and_header_in_tuefteln` |
| Gesamtgate | (G.1) Gebiet in Bauen, (G.2) Erweitern, Teile, Höhe | Agent | Teil 3f.3 auf Platin DE und HeartGold DE, dann im Raumplan des neuen Gebiets „+“ bei 2,0, „Karte anlegen“, Inspector „Karte“ › „Aktionen“ › „Teile importieren…“ aus einer Karte im Projekt, „Höhe“ 1, `call.sh run_checks` |
| | (G.3) Einblendung | Agent + Blick | Szenarien `pt-neuheim.yaml` und `hg-neuheim.yaml` (unten), Bilder `einblendung` (und `osten`) |
| | (G.4) Fundort, (G.5) DSPRE | Hand | siehe „Was beim Nutzer bleibt“ |
| | (G.6) Prüfungen | Agent | gated Workspace-Tests je frischer Kopie: `G4_TEST_PROJECT=<Kopie> cargo test --workspace --all-features -- --test-threads 1` (Platin DE, HeartGold DE und US; darin `vanilla_checks` mit 41 Prüfungen und `dynamic_headers`) |
| | (G.7) Ladezeit | Agent | Rezept „Ladezeit messen“ |
| | `scripts/gate.sh` | Agent | auf main, im Hintergrund, mit absolutem `G4_ROMS_DIR` (aus einem Worktree dazu `tools/` als Symlink auf das Hauptverzeichnis, siehe „Einmalig einrichten“, Punkt 3; den Symlink nie einchecken) |
| 3f.4 Nahtlos und Abspalten | (4.1) Lage im Assistenten | Agent | Browser: „Gebiet anlegen…“, Quelle Sinnoh, Schritt „Anordnung“, „Auf der Weltkarte Sinnoh (nahtlos begehbar)“, Klick auf eine Gebietszelle (nichts) und auf Landschaft (gewählt), Zusammenfassung lesen, abbrechen |
| | (4.2) Nahtlos im Spiel | Agent + Blick | Szenario `pt-nahtlos.yaml` (unten), Bild `einblendung` |
| | (4.3) Plakette „Geteilt“, (4.4) Abspalten, (4.5) nur ein Ort | Agent | Browser: Jubelstadt › „Pokémon-Center“ (Header 0006): Baumzeile mit Symbol `aria-label` „geteilt“, im Kopf `[data-shared]` „Geteilt mit 16 Räumen“; „Aktionen“ › „Für diesen Raum abspalten…“, `[data-split-room]`, „Abspalten“; Gegenprobe `call.sh list_headers` (Matrix des Headers 0006 neu, die anderen auf 116) und `call.sh set_permission` an der Kopie gegen `get_map` der alten Karte. `split_room` ist kein kuratiertes MCP-Werkzeug (über `g4_call`) |

**Szenarien von 3f** (aus `scripts/g4emu`, Erklärung im Abschnitt „Szenario“ und unter „Gesamtgate (3b.4b …)“): `pt-neuer-ort.yaml` (3f.2: `create_place` „Neuheim“ nur für den Außen-Header 411, aus dem Haus treten, Einblendung), `pt-neuheim.yaml` und `hg-neuheim.yaml` (Gesamtgate: `create_area` mit neuem Header, neuem Ort und `allow_patch`, `extend_area`, Teile importieren, Höhe; die Treppe des Zimmers führt ins Gebiet; Platzhalter `KOPIE`), `pt-nahtlos.yaml` (3f.4: „Neuweg“ in der Landschaftszelle (4, 27) östlich von Zweiblattdorf, ohne Tür hinüber). Aus einem Worktree zeigen die relativen Pfade der Kopf-Kommentare ins Leere; dort mit absoluten Pfaden und nach `cargo build --workspace --all-features`:

```sh
W=<absoluter Pfad des Worktrees>; R=<absoluter Pfad zu Roms>; O=$W/target/agent/gate-$(date +%F)
mkdir -p "$O"
export G4_ROMS_DIR=$R G4_BRIDGE_BIN=$W/target/debug/studio-bridge G4K_BIN=$W/target/debug/g4k
cd $W/scripts/g4emu
uv run g4emu scenario ../agent/scenarios/pt-neuer-ort.yaml --project $R/_testkopien/3f2-ort-pt \
  --out $O/pt-neuer-ort > $O/pt-neuer-ort.out 2> $O/pt-neuer-ort.err
uv run g4emu scenario ../agent/scenarios/pt-neuheim.yaml --project $R/_testkopien/3f-gate-pt \
  --out $O/pt-neuheim --set KOPIE=$R/_testkopien/3f-gate-pt > $O/pt-neuheim.out 2> $O/pt-neuheim.err
uv run g4emu scenario ../agent/scenarios/hg-neuheim.yaml --project $R/_testkopien/3f-gate-hgd \
  --out $O/hg-neuheim --set KOPIE=$R/_testkopien/3f-gate-hgd > $O/hg-neuheim.out 2> $O/hg-neuheim.err
uv run g4emu scenario ../agent/scenarios/pt-nahtlos.yaml --project $R/_testkopien/3f4-nahtlos-pt \
  --out $O/pt-nahtlos > $O/pt-nahtlos.out 2> $O/pt-nahtlos.err
```

Je Szenario eine frische Kopie (die Namen oben sind Beispiele; das Ziel darf nicht existieren). `G4_BRIDGE_BIN` und `G4K_BIN` zeigen auf den Build des Worktrees: nach einer Änderung am Rust-Code erst neu bauen, sonst läuft das Szenario mit dem alten Stand (für die Brücke der Seite gilt dasselbe, siehe oben). Bestanden heißt: Rückgabe 0, `"ok": true` in der letzten Zeile von stdout, die Zeilen „Erwartung erfüllt“ auf stderr (`pt-neuheim`: Patch vorher aus, nachher an, `location_name` 126, Breite 3, Höhe 1, 0 Fehler; `hg-neuheim`: Ort 235; `pt-nahtlos`: `headers.814` = 593, 0 Fehler), und der Header-Wechsel im Spiel (`pt-neuheim` 415 → 593, `hg-neuheim` 64 → 540, `pt-nahtlos` 415 → 414 → 411 → 593 ohne Warp). Die Bilder liegen im Ausgabeordner (`einblendung.png`, bei `pt-neuheim` dazu `osten.png`); der Agent nennt dem Nutzer Pfad und was zu sehen ist (die Einblendung „Neuheim“ bzw. „Neuweg“ oben links). Bleibt ein Lauf unterwegs stehen (etwa eine Begegnung im hohen Gras), mit einer neuen Kopie wiederholen.

## Phase 4.1a ([phase-4-1a-gate.md](phase-4-1a-gate.md))

4.1a ordnet einem DSPRE-Ordner seine Basis-ROM zu und schreibt in Technik › Veröffentlichen einen xdelta-Patch. Die Punkte und Ergebnisse stehen in der Gate-Datei; hier nur, wer sie prüft und wie.

**Kopien:** je Zweck ein Ordner `4-1a-<zweck>_DSPRE_contents` und die `.nds` `4-1a-<zweck>.nds` direkt nebeneinander unter `$G4_ROMS_DIR/_testkopien` (frisch aus dem Original, das nur gelesen wird; `cp -cR` bzw. `cp -c` auf APFS). Nur in dieser Form greift der Vorschlag (die `.nds` neben `<Name>_DSPRE_contents`). Seit `eeba3c8` öffnet die Brücke Ordner mit der Endung `_DSPRE_contents`, wenn ein früherer Bestandteil des Pfads `_testkopien` heißt; einen solchen Ordner neu anlegen lehnt sie weiter ab (3g.1 (A4)). Zweck: `pt`, `hgde`, `hgus` (kopflos), `web` (Browser), `mcp` (MCP), dazu `4-1a-pt-fremd.nds` (Kopie von `4-1a-pt.nds`, letztes Byte gekippt). Die Kopien nach dem Gate löschen.

| Punkt | Wer | Wie |
|---|---|---|
| (1) gated `--test base` | Agent | je Projekt `G4_TEST_PROJECT=<Original> cargo test -p g4-kernel --all-features --test base -- --nocapture --test-threads 1`, `xdelta3` im PATH; die Zeilen `rebuild … equal`, `patch-sizes …`, `patch-stress …`, `xdelta3 cross-check: …` in die Tabelle der Gate-Datei |
| (1) `cargo deny`, `scripts/gate.sh` | Agent | `scripts/gate.sh` im Hintergrund, aus einem Worktree mit absolutem `G4_ROMS_DIR` und dem `tools`-Symlink; Schritt `cargo deny (licenses, advisories)` |
| (2) Szenario kopflos | Agent | Release-`g4k`, Befehle unten |
| (3) Veröffentlichen, Basis-ROM fehlt, suchen | Agent | Browser, Ablauf unten |
| (3) MCP | Agent | `mcp.sh` im Hintergrund, „Zulassen“ im Browser-Fenster, Befehle unten |
| Blick | Agent + Blick | Bilder `blick-veroeffentlichen-*.png`, der Agent nennt Zustand und Texte je Bild |
| Rom Patcher JS, xdelta UI | Hand | siehe „Was beim Nutzer bleibt“ |

Löst `-p g4-kernel` in einem Worktree den Download von rotom aus (siehe [dev-setup.md](dev-setup.md), „In einem Git-Worktree“), dann über den ganzen Workspace mit Namensfilter: `cargo test --workspace --all-features -- base_assign base_rom patch_after_map --nocapture --test-threads 1` und `cargo build --release --workspace --all-features`. Im Gate 4.1a lief `-p g4-kernel` im Worktree ohne Download.

**Kopflos (2)**, aus der Wurzel des Worktrees nach `cargo build --release -p g4-kernel --bin g4k` (`K` ist `$G4_ROMS_DIR/_testkopien`, `<k>` je Spiel):

```sh
G=target/release/g4k
$G base check  $K/4-1a-<k>_DSPRE_contents                     # mark=none, suggestion=…/4-1a-<k>.nds
$G base assign $K/4-1a-<k>_DSPRE_contents $K/4-1a-<k>.nds     # ohne warning:, assigned sha1=… label=…
$G build $K/4-1a-<k>_DSPRE_contents --out $K/4-1a-<k>_DSPRE_contents/build/rebuild.nds
shasum $K/4-1a-<k>_DSPRE_contents/build/rebuild.nds $K/4-1a-<k>.nds    # gleich
$G base assign $K/4-1a-pt_DSPRE_contents $K/4-1a-hgde.nds      # Exit 1, „Falsches Spiel …“
$G base check  $K/4-1a-pt_DSPRE_contents --rom $K/4-1a-pt-fremd.nds   # warning: Unbekannte oder veränderte ROM …, Exit 0
$G map set-perm $K/4-1a-<k>_DSPRE_contents 3 2 0 --kind 0 --collision 0   # Platin; HeartGold mit --collision 128; muss „saved 1 file(s)“ melden
G4_BASE_ROM_DIR=$K $G patch create $K/4-1a-<k>_DSPRE_contents --json   # size, encode_ms, verify_ms, target_sha1
mkdir -p target/agent/4-1a
xdelta3 -d -f -s $K/4-1a-<k>.nds <patch> target/agent/4-1a/out-<k>.nds && shasum target/agent/4-1a/out-<k>.nds   # gleich build/<Name>.nds
xdelta3 printhdr <patch>                                       # VCD_APPHEADER, secondary compressor: none
```

Meldet `map set-perm` „saved 0 file(s)“, hatte die Zelle den Wert schon: dann ist der Patch leer (Ziel-SHA-1 = Basis), einen anderen Wert setzen (im Gate bei Platin `--collision 0` statt 128). `G4_BASE_ROM_DIR` zeigt im Gate auf `_testkopien`, nie auf die Originale.

**Browser (3)**, Test-Umgebung auf eigenen Ports (`G4_ROMS_DIR=<absolut> G4_VITE_PORT=1500 G4_BRIDGE_PORT=1501 bash scripts/agent/studio-web.sh`), Seite `http://127.0.0.1:1500/?project=<…>/_testkopien/4-1a-web_DSPRE_contents`, Fenster 1440 × 900, Hell, Komfort. Technik › „Veröffentlichen“ (`[data-technik=veroeffentlichen]`): Karte `[data-base-state=unassigned]` mit dem Vorschlag, „Zuordnen…“, Dialog „Basis-ROM zuordnen?“ ablesen, „Zuordnen“; `[data-base-state=found]`; in `[data-patch-card]` „Patch erstellen…“, Dialog „Patch erstellen?“, „Patch erstellen“, auf „Patch erstellt: …“ warten (mit Bauen rund 15 s). Dann die `.nds` umbenennen (`mv 4-1a-web.nds 4-1a-web-verschoben.nds`), Technik verlassen und neu zeigen: `[data-base-state=missing]` mit Grund, „Patch erstellen…“ gesperrt. „Basis-ROM suchen…“ öffnet im Browser-Modus das Pfadfeld: erst `4-1a-hgde.nds` (Fehler „Das ist eine andere ROM …“), dann `4-1a-web-verschoben.nds` („Gefunden“). Seit 4.5 gibt es `[data-patch-card]` „Patch erstellen…“ nicht mehr: an ihrer Stelle steht die Karte „Veröffentlichen“ mit dem Assistenten, siehe Abschnitt „Phase 4.5“ unten; die Schritte mit der Basis-ROM gelten weiter, statt „Patch erstellen…“ ist dann „Veröffentlichen…“ frei bzw. gesperrt.

**MCP (3)** über die Brücke (siehe „MCP über die Brücke“), Kopie `4-1a-mcp` in der Seite geöffnet:

```sh
scripts/agent/call.sh agent_set_enabled '{"on":true}'
scripts/agent/call.sh agent_set_grant '{"level":"write"}'                 # „Lesen und bearbeiten“, base_id null
scripts/agent/mcp.sh call assign_base '{"nds":"../4-1a-mcp.nds"}' > target/agent/mcp-assign.out 2>&1 &
# im Browser-Fenster: Dialog „Agent möchte etwas Unumkehrbares tun“ („Basis-ROM zuordnen“), Klick auf „Zulassen“
cat target/agent/mcp-assign.out                                         # state found, source known, patch_name
scripts/agent/mcp.sh call base_status                                   # sofort, ohne Frage nach der Freigabe
scripts/agent/mcp.sh call create_patch '{}' > target/agent/mcp-patch.out 2>&1 &
# Dialog „… möchte: Patch erstellen (…)“, „Zulassen“
```

Der Pfad `nds` ist relativ zum Projekt; die Nachbar-`.nds` liegt außerhalb des Projektordners und ist nur erlaubt, weil sie genau der Vorschlag ist. Nach dem Zuordnen steht dieselbe Freigabe in `target/agent/app-data/agents.json` mit der neuen `base_id` (`<sha1>/<created>`).

**Blick-Bilder (optional):** Die Werkzeuge des Browser-Fensters schreiben keine Datei. Die 13 Bilder unter `target/agent/gate-2026-10-27/4-1a/` entstanden mit Chrome headless über CDP (ein kleines Node-Skript im Scratchpad, nicht im Repo; Seite der laufenden Test-Umgebung, 1440 × 900). Wer keine Bilder braucht, beschreibt Zustand und Texte im Gate-Bericht und lässt den Nutzer ins Browser-Fenster sehen.

## Phase 4.2 ([phase-4-2-gate.md](phase-4-2-gate.md))

4.2 baut nur noch die Schritte, deren Eingaben sich geändert haben, und zeigt sie in Technik › Prüfen und Bauen. Die Punkte und die Ergebnisse stehen in der Gate-Datei (sie entsteht mit dem Gate); hier nur, wer sie prüft und wie. Kopien nur frisch unter `$G4_ROMS_DIR/_testkopien/4-2-<zweck>` (das Ziel darf nicht existieren), danach löschen.

**Test-Umgebung aus dem Worktree:** eigene Ports und absolutes `G4_ROMS_DIR`, etwa `G4_ROMS_DIR=<absoluter Pfad zu Roms> G4_VITE_PORT=1510 G4_BRIDGE_PORT=1511 bash scripts/agent/studio-web.sh > target/agent/studio-web.log 2>&1 &`, warten auf „Brücke läuft“, Seite `http://127.0.0.1:1510/?project=<Kopie>`. Nach einer Änderung am Rust-Code die Umgebung neu starten.

**Wo was ist (Technik › Prüfen und Bauen):** Tasten „Bauen“ und „Neu bauen (ohne Cache)“ (`title` „Löscht den Bauzustand und baut alles neu“), Fläche `[data-technik=bauen]`. Unter der Überschrift „Schritte“ steht ohne Build „Noch kein Build in dieser Sitzung.“, nach einem Build die Tabelle „Schritte des letzten Builds“ (`[data-build-steps]`) mit den Spalten „Schritt“, „Ergebnis“, „Dauer“. Je Zeile `tr[data-step=<id>][data-status=skipped|incremental|full]`, die Kennungen `rotom`, `chatot`, `pack:<Ordner>`, `blz:arm9`, `blz:overlays`, `rom`, `crc` (`watch:*` steht nie da). Die übersprungenen liegen im Abschnitt „Übersprungen (N)“ (Taste mit `aria-expanded`, eingeklappt), offen in der Tabelle „Übersprungene Schritte“. Bauen zeigt keine Kennung; Tüfteln setzt sie grau hinter die Wörter. Die Meldung nach dem Build lautet „ROM gebaut: <Pfad> in 0,33 s (… Datei(en) vorher gespeichert)“.

**Dialog bei einem Text-Konflikt:** `[role=dialog][data-text-conflict]` mit dem Titel „Texte auf zwei Seiten geändert“. Je Archiv eine Zeile `[data-archive=<Nummer>]` mit dem Namen `[data-archive-name]` („Textarchiv 0012“) und der Gruppe `[role=group][aria-label="Fassung von Textarchiv 0012"]` („Text (JSON)“ vorgewählt, `aria-pressed`, oder „Spieldatei“). Die Tasten sind „Abbrechen“ und „Übernehmen und bauen“. „Übernehmen und bauen“ ruft `resolve_text_conflicts` und baut an denselben Pfad (nach „Neu bauen (ohne Cache)“ wieder ohne Cache). „Abbrechen“ baut nichts und meldet „Nichts gebaut. Beim nächsten „Bauen“ fragt das Studio wieder, welche Fassung gilt.“ Einen Konflikt erzeugen: in einer Kopie die JSON `expanded/textArchives/NNNN.json` und den Member `unpacked/textArchives/NNNN` eines Archivs von außen ändern, das schon einmal gebaut wurde. Danach `call.sh build_rom '{"out":"…"}'`: Fehler `text_conflict`. Speichern fragt nicht selbst, es meldet „Textarchiv 0012 wurde als Text und als Spieldatei geändert. Wähle beim nächsten „Bauen“, welche Fassung gilt.“

| Punkt | Wer | Wie |
|---|---|---|
| (G1) `scripts/gate.sh` | Agent | im Hintergrund, `> target/agent/gate.log`, aus dem Worktree mit absolutem `G4_ROMS_DIR` und `tools`-Symlink; die Tabelle am Ende übernehmen, Zeile „build graph (timings)“ unter Last einmal wiederholen |
| (G2) Byte-Gleichheit | Agent | die gated Tests der Tasks 1 bis 9 (inkrementell gleich voll ohne Cache, Vanilla gleich `dsrom`-CLI, extrahiertes Projekt gleich `.g4/base.json`) |
| (G3) Zeiten | Agent | `g4k build --timings` auf `4-2-zeiten-{pt,hgde,hgus}`, Median aus drei Läufen je Fall, gegen die Zeilen `build-timings:` |
| (G4) DSPRE und Git | Agent | Tests aus Task 3; von Hand auf `4-2-git-pt` mit `git init`/`git switch` und geänderter Text-JSON, danach „Bauen“ im Browser: die JSON des Branches gewinnt, ein beidseitiger Wechsel öffnet `[data-text-conflict]` |
| (G5) Szenarien | Agent + Blick | `pt-treppe-warp.yaml` auf `4-2-szenario-pt` und `hg-neuheim.yaml` auf `4-2-szenario-hg` (Aufruf wie unter Phase 3f), je Kopie vorher einmal `g4k build`; Bilder ansehen und mit Pfad, Ort und Inhalt beschreiben |
| (G6) Browser | Agent | auf `4-2-browser-hgde` zweimal „Hier spielen“, dann Technik › Prüfen und Bauen lesen. Die Zeilen `[data-step]` stehen unter „Übersprungen“, `rom` heißt dort „ROM schreiben … aktuell“. Eine Karte ändern und „Hier spielen“: `[data-step="pack:maps"][data-status=incremental]` „1 Member eingesetzt“, Zeit im Toast. „Neu bauen (ohne Cache)“: jede Zeile `data-status=full`, die Gründe „(ohne Cache, voll)“. Text-Konflikt erzeugen, `[data-text-conflict]`, beide Wahlen je einmal: danach JSON und Member vergleichen; nach „Übernehmen und bauen“ steht `[data-step="chatot"][data-status=incremental]` (kein „erster Build“), eine behaltene JSON hat ihre Bytes behalten. Befunde `build.hidden-file` (Meldung „1 versteckte Datei unter files/ hält den Build an; …“, in den Problemen „Entfernen“) und `build.base-missing` |
| (G7) gepackte App, DSPRE, Blick | Hand | siehe „Was beim Nutzer bleibt“: „Hier spielen“ in der gepackten App auf HeartGold, DSPRE-Gegenprobe mit einem Ordner wie `pokeHeight`, die Schritte-Tabelle hell, dunkel und kompakt |

## Phase 4.1b ([phase-4-1b-gate.md](phase-4-1b-gate.md))

4.1b setzt eine Entität auf den Stand der Basis-ROM zurück (Kopf, Kontextmenü, MCP) und listet in Technik › Änderungen, was vom Original abweicht. Die Punkte und Ergebnisse stehen in der Gate-Datei; hier nur, wer sie prüft und wie.

**Kopien:** je Zweck ein Ordner `4-1b-<zweck>_DSPRE_contents` und die `.nds` `4-1b-<zweck>.nds` direkt nebeneinander unter `$G4_ROMS_DIR/_testkopien` (frisch aus dem Original, das nur gelesen wird; `cp -cR` bzw. `cp -c`), sonst findet `start::suggest_base` die Basis-ROM nicht. Zweck: `pt`, `hgde`, `hgus` (gated Tests, nur gelesen), `kpt`, `khgde`, `khgus` (kopflos), `web` (Browser), `mcp` (MCP über die Brücke). Die Kopien nach dem Gate löschen.

**Wo was ist:** Der Eintrag „Auf Original zurücksetzen…“ (Symbol `undo`, mit der Art davor, wo der Kopf mehrere Ziele hat: „Eventdatei …“, „Header …“, „Scriptdatei …“, „Kartentextur …“, „Gebäudetextur …“, „Gebäudemodell …“, „Nachricht …“, „Ganzes Textarchiv …“) steht im Menü „Aktionen“ der Köpfe (`aria-label` „Aktionen“, Symboltaste `more`; Pokédex, Trainer, Begegnungen, Inspector-Köpfe der Welt, Hand der Texte, Script-Panel) und im Kontextmenü der Listen- und Baumzeilen (Rechtsklick, oder Fokus auf die Zeile und Shift+F10: der Eintrag hat dann den Fokus, Enter öffnet den Dialog). Karten nur im Kopf des Zellen-Inspectors. Gesperrt trägt er rechts „Wird gelesen …“, „Basis-ROM fehlt“ oder „Kein Original“. Der Dialog ist `[role=dialog][data-reset-dialog]`, Titel `h2` „<Name> auf Original zurücksetzen?“, je Änderung `[data-change]` mit Überschrift und `[role=group][data-wrap]` (Vorher, Nachher), beim Header `[data-header-field=<feld>]` mit Kästchen; Tasten „Abbrechen“ und „Zurücksetzen“, sonst nur „Schließen“. Technik › „Änderungen“ (`[data-technik=aenderungen]`): „Neu vergleichen“, je Gruppe `[data-changes-group=<Gruppe>]` (Titel wie „Karten“, „Header“), je Zeile `[data-change=<Name>]` (`aria-label` „Karte 0003, Geändert“, mit „, Ungespeichert“) und ihre Taste „Auf Original zurücksetzen…“.

| Punkt | Wer | Wie |
|---|---|---|
| (1) gated `--test original` und Studio-Test | Agent | je Projekt `G4_TEST_PROJECT=$G4_ROMS_DIR/_testkopien/4-1b-<k>_DSPRE_contents cargo test -p g4-kernel --all-features --test original -- --nocapture --test-threads 1` und `… cargo test -p studio --all-features --lib reset_over_the_command_path -- --nocapture --test-threads 1`, im Hintergrund (je Projekt rund 4 min Debug); die Zeilen `reset <Spielcode> <Ziel> ms=…`, `changed <Spielcode> first_ms=… second_ms=…` und `reset over the command path: …` in die Zeittabelle der Gate-Datei |
| (1) `scripts/gate.sh` | Agent | im Hintergrund, aus dem Worktree mit absolutem `G4_ROMS_DIR` und `tools`-Symlink; die gated Tests von 4.1b laufen im Schritt der gated Tests je Projekt mit |
| (2) Szenario kopflos | Agent | `studio mcp` über stdio, Ablauf unten |
| (3) Browser | Agent | Test-Umgebung, Ablauf unten |
| (3) MCP über die Brücke | Agent | `mcp.sh`, Befehle unten |
| Kontextmenü in der App, Fortschrittszahlen, Feldnamen | Hand | siehe „Was beim Nutzer bleibt“ |
| Blick | Agent + Blick | Bilder `blick-*.png` unter `target/agent/gate-<Datum>/4-1b/`, der Agent nennt Zustand und Texte je Bild |

Löst `-p g4-kernel --test original` in einem Worktree den Download von rotom aus (siehe [dev-setup.md](dev-setup.md), „In einem Git-Worktree“), dann über den ganzen Workspace mit Namensfilter: `cargo test --workspace --all-features -- reset_of_each header_reset script_texture changed_list decompile_member reset_over_the_command_path --nocapture --test-threads 1`. Im Gate 4.1b lief `-p` im Worktree ohne Download.

**Kopflos (2)**, aus der Wurzel des Worktrees nach `cargo build --workspace --all-features` (`K` ist `$G4_ROMS_DIR/_testkopien`, `<k>` je Spiel):

```sh
target/debug/g4k base assign $K/4-1b-<k>_DSPRE_contents $K/4-1b-<k>.nds
target/debug/studio mcp --project $K/4-1b-<k>_DSPRE_contents --base-rom $K/4-1b-<k>.nds
```

Den Server bedient ein kleines Skript über JSON-RPC (stdio; im Gate ein Python-Skript im Scratchpad): `changed_from_original` (leer), dann `data_edit` (Trainer 1, Level +1), `set_header` (Header 3, `weather_id`), `edit_text` (Archiv 1, Nachricht 0), `save`; `changed_from_original` (genau „Textarchiv 0001“, „Trainer 0001“, „Header 0003“); je Ziel `plan_reset` und `reset_to_original` mit `{"target":{"kind":"trainer","index":1}}`, `{"target":{"kind":"header","id":3}}`, `{"target":{"kind":"text_message","archive":1,"message":0}}`, `save`; `changed_from_original` (leer); `undo`; `reset_to_original` mit `trainer` 5000 (`no_original`); `g4_batch` mit `reset_to_original` eines Scripts (abgelehnt). Danach `target/debug/g4k extract $K/4-1b-<k>.nds <tmp>` und `cmp` von `unpacked/trainerProperties/0001`, `unpacked/trainerParty/0001` und `unpacked/textArchives/0001` gegen die extrahierte Basis. Die erste Liste auf einer ungebauten Kopie dauert 30 bis 50 s (kein App-Cache, jeder Ordner gehasht).

**Browser (3)**, Test-Umgebung auf eigenen Ports (`G4_ROMS_DIR=<absolut> G4_VITE_PORT=1540 G4_BRIDGE_PORT=1541 bash scripts/agent/studio-web.sh`), Seite `http://127.0.0.1:1540/?project=<…>/_testkopien/4-1b-web_DSPRE_contents`, Fenster 1440 × 900, Hell, Komfort. Ohne Zuordnung zuerst Technik › Änderungen lesen (Grund, Einträge „Basis-ROM fehlt“), dann in Technik › Veröffentlichen „Zuordnen…“. Je Art ändern und speichern (Karte, Eventdatei, Nachricht und ganzes Textarchiv, Trainer, Pokémon, Begegnung, Header mit einem abgewählten Feld, Script, Kartentextur, Gebäudemodell), dann zurücksetzen über Kopf „Aktionen“, Rechtsklick, Shift+F10 auf der fokussierten Zeile und die Zeilen von Technik › Änderungen (jeder Weg mindestens einmal); Dialog lesen, „Zurücksetzen“, speichern, Technik › Änderungen ohne den Eintrag. ⌘Z und ⌘⇧Z; ⌘Z eines Dateischritts schreibt die Datei zurück (Gebäudemodell); eine Kartentextur nach dem Zurücksetzen außen ändern, dann ⌘Z: Meldung „… wurde seit dem Zurücksetzen außerhalb des Studios geändert; …“, der Schritt fällt weg. Neue Begegnungsdatei: Eintrag gesperrt mit „Kein Original“, in der Liste „Neu“. Zum Schluss die `.nds` umbenennen und Technik neu zeigen: „Basis-ROM nicht gefunden: …“, Einträge „Basis-ROM fehlt“. Im Browser-Modus zeigt der Vergleich nur „Vergleiche …“ ohne Zahlen (kein `Channel` in der Brücke).

**MCP (3)** über die Brücke (siehe „MCP über die Brücke“), Kopie `4-1b-mcp` mit zugeordneter Basis-ROM in der Seite geöffnet, Trainer 1 vorher geändert und gespeichert:

```sh
scripts/agent/call.sh agent_set_enabled '{"on":true}'
scripts/agent/call.sh agent_set_grant '{"level":"write"}'                              # „Lesen und bearbeiten“
scripts/agent/mcp.sh call changed_from_original
scripts/agent/mcp.sh call plan_reset '{"target":{"kind":"trainer","index":1}}'
scripts/agent/mcp.sh call reset_to_original '{"target":{"kind":"trainer","index":1}}'  # ohne Rückfrage (Write, Undo-Schritt)
scripts/agent/mcp.sh call history                                                    # undo_agents: ["agent-gate"]
scripts/agent/mcp.sh call reset_to_original '{"target":{"kind":"trainer","index":5000}}'   # Rückgabe 1, code no_original
```

Die Oberfläche lädt den Trainer nach dem Reset neu; ein `g4_batch` mit `reset_to_original` eines Scripts lehnt ab („reset_to_original geht nicht in g4_batch“).

**Blick-Bilder:** Die Werkzeuge des Browser-Fensters schreiben keine Datei. Die 17 Bilder unter `target/agent/gate-2026-10-27/4-1b/` entstanden mit Chrome headless über CDP (ein kleines Skript im Scratchpad, nicht im Repo; Seite der laufenden Test-Umgebung, 1440 × 900; Hell und Dunkel über `prefers-color-scheme`, Kompakt über die Taste der Statusleiste). Namen `blick-<fläche>-<zustand>-<thema>.png`, Liste in der Gate-Datei, Abschnitt „Blick“.

## Phase 4.3a ([phase-4-3a-gate.md](phase-4-3a-gate.md))

4.3a schreibt ein Projekt als Textprojekt (`g4-project.json`, `data/`) und liest es zurück, kopflos über `g4k text …` und über MCP `text_check`/`text_export`; es gibt keine Oberfläche. Die Punkte und Ergebnisse stehen in der Gate-Datei; hier nur, wer sie prüft und wie.

**Kopien:** frisch unter `$G4_ROMS_DIR/_testkopien` per `cp -cR` (Ordner) und `cp -c` (`.nds`), Ordner `4-3a-<zweck>_DSPRE_contents` und `.nds` `4-3a-<zweck>.nds` direkt nebeneinander (sonst findet `g4k` die Basis-ROM nicht von selbst): `pt`, `hgde`, `hgus` für das Szenario, `mcp` (Platin DE) für MCP. Der Klon-Weg legt `4-3a-klon-<k>/` an (`repo/` ist der Git-Klon der Kopie, `fresh/` das Extract). `git init` läuft **in der Kopie**, nie im Repo. Kein exportiertes `data/` eines Vanilla-Projekts ins Repo.

| Punkt | Wer | Wie |
|---|---|---|
| (1) `scripts/gate.sh` | Agent | im Hintergrund, `> target/agent/gate-sh-4-3a.log`, aus dem Worktree mit absolutem `G4_ROMS_DIR` und `tools`-Symlink; die gated Tests des Textprojekts laufen je Projekt mit; „build graph (timings)“ unter Last einmal wiederholen |
| (1) Messzeilen | Agent | je Projekt `G4_TEST_PROJECT=<Original> cargo test --release -p g4-kernel --all-features --test textproject --test textproject_vanilla_probe -- textproject_round_trip textproject_export_idempotent textproject_from_base vanilla_ --nocapture --test-threads 1`; die Zeilen `textproject-stats`, `textproject-second-export`, `textproject-from-base`, `base-layer`, `items` in die Messtabelle |
| (2) Szenario kopflos | Agent | Release-`g4k`, Befehle unten und in der Gate-Datei |
| (3) MCP | Agent | kopflos `studio mcp --project <Kopie> [--allow-irreversible]`, über die Brücke `mcp.sh` und `call.sh`, Befehle unten |
| (4) Lesbarkeit, Lizenz | Hand | siehe Gate-Datei, Abschnitt „(4) Hand und Blick“ |

**Szenario (2)**, aus der Wurzel des Worktrees nach `cargo build --release -p g4-kernel --bin g4k` (`K` ist `$G4_ROMS_DIR/_testkopien`, `P=$K/4-3a-<k>_DSPRE_contents`):

```sh
G=$PWD/target/release/g4k
$G base assign $P $K/4-3a-<k>.nds
$G text export $P                       # kalt: Basisschicht unter .g4/cache/base-layer/<sha1>/, Scripts übernommen
$G text export $P                       # written=0 removed=0
(cd $P && git init -q && $G text sources . | tr '\n' '\0' | xargs -0 git add && git commit -qm Ausgangsstand)
$G text export $P && git -C $P status --porcelain --untracked-files=no   # leer
$G text check $P                        # schema=current, differs=0
python3 -I scripts/agent/textproject_diff.py $K/4-3a-klon-hgde/repo $K/4-3a-klon-hgus/repo   # vier Karten, 829 Textarchive
```

Zeiten mit `/usr/bin/time -p` und `uptime` daneben messen; ein Export dauert unter Last ein Vielfaches (Microsoft Defender prüft jede Datei). Für „Export warm“ `data/` und `g4-project.json` **außerhalb** des Projekts ablegen: ein Ordner in der Wurzel (etwa `data.prev/`) wird beim Export ein Blob unter `data/raw/` und beim Import wieder angelegt.

**MCP (3).** Kopflos (der Python-Client der Handprobe in [dev-setup.md](dev-setup.md), „MCP“): `text_check` → `missing`; `text_export` ohne `--allow-irreversible` → `needs_confirmation`, mit ihm → `not_adopted`, bis die Scripts übernommen sind; nach einem ungespeicherten `set_matrix_cell` → `unsaved`. Über die Brücke (Ports wie in der Gate-Datei, Seite mit `?project=<Kopie>` im Browser-Fenster):

```sh
scripts/agent/call.sh agent_set_enabled '{"on":true}'
scripts/agent/call.sh agent_set_grant '{"level":"write"}'                 # „Lesen und bearbeiten“
scripts/agent/mcp.sh call text_check '{}'                                 # missing
scripts/agent/mcp.sh call text_export '{}'                                # not_adopted, ohne Frage
scripts/agent/call.sh adopt_scripts '{}'                                  # der eigene Schritt der Oberfläche
scripts/agent/mcp.sh call text_export '{}' > target/agent/mcp-export.out 2>&1 &
scripts/agent/call.sh agent_status                                        # question: {kind: confirm, what: "Textprojekt exportieren"}
# im Browser-Fenster „Zulassen“, oder: scripts/agent/call.sh agent_answer '{"question":<id>,"answer":"allow"}'
scripts/agent/mcp.sh call text_check '{}'                                 # current
```

Mit einer ungespeicherten Änderung (`mcp.sh call set_matrix_cell …`) antwortet `text_export` `unsaved`, ohne zu fragen; `undo` nimmt sie zurück.

## Phase 4.3b ([phase-4-3b-gate.md](phase-4-3b-gate.md))

4.3b macht das Textprojekt zur Quelle des Studios: der Store liest und schreibt die Entitäts-JSON unter `data/`, ein DSPRE-Ordner lässt sich einrichten und zurücknehmen, Änderungen von außen (DSPRE, Hand, Branch) werden beim Öffnen und beim Fensterfokus übernommen oder als Konflikt gezeigt, IDs gelten wie Nummern, „ID umbenennen“ zieht alle Verweise nach. Die Punkte und Ergebnisse stehen in der Gate-Datei; hier nur, wer sie prüft und wie.

**Kopien:** frisch unter `$G4_ROMS_DIR/_testkopien` per `cp -cR` (Ordner) und `cp -c` (`.nds`), Ordner `4-3b-<zweck>_DSPRE_contents` und `.nds` `4-3b-<zweck>.nds` direkt nebeneinander: `pt`, `hgde`, `hgus` für die gated Tests (nur gelesen, die Tests schreiben auf `vanilla_copy()`), `kpt`, `khgde`, `khgus` für das Szenario kopflos, `web`, `mcp`, `ui` (Platin DE) für Browser und MCP über die Brücke. `git init` läuft **in der Kopie**, nie im Repo. Alle Kopien nach dem Gate löschen (`Manual` und `_saves` bleiben).

**Wo was ist:** Technik › „Textprojekt“ (`[data-technik=textprojekt]`; Untertitel „DSPRE-Layout“, „Abgeglichen“, „1 Konflikt“, „1 fehlerhafte Datei“, „Wird eingerichtet …“): im DSPRE-Layout der Kopftext und „Als Textprojekt einrichten…“ (gesperrt mit „Diesem Projekt ist keine Basis-ROM zugeordnet.“), im Textprojekt „Textprojekt, Schema-Version 1. Zuletzt abgeglichen …“, „Jetzt abgleichen“, „Zurücknehmen…“, der Abschnitt „Zuletzt übernommen“ (Zeilen „Attacke 0033 · MOVE_TACKLE“ mit Plakette „aus DSPRE“, sonst „Nichts von außen geändert.“), `[data-sync-conflicts]` „Auf zwei Seiten geändert“ mit je Einheit `[data-conflict=<schlüssel>]` (aufklappen: Tabelle Feld / Studio / DSPRE, bei ungespeicherten Werten „Studio (ungespeichert)“, Tasten „Studio-Fassung behalten“ und „DSPRE-Fassung übernehmen“, Plakette „Ungespeichert“ mit Warnung) und `[data-sync-broken]` „Fehlerhafte Dateien“ (`{datei}:{zeile}:{spalte}: {satz}`). Meldungen: „1 Änderung aus DSPRE übernommen“ (+ „Anzeigen“), „1 Konflikt zwischen Studio und DSPRE“ (+ „Lösen…“), „1 Projektdatei ist fehlerhaft“, „Erst die Konflikte in Technik › Textprojekt lösen.“. Hinweis beim Öffnen eines DSPRE-Ordners: „… DSPRE öffnet den Ordner weiter.“ mit „Einrichten…“ (ohne Basis-ROM „… Dafür braucht es die Basis-ROM.“ mit „Basis-ROM zuordnen…“), dazu „Nicht mehr fragen“. Dialoge „Als Textprojekt einrichten?“ (Prüfliste ✓ „Alles gespeichert“, ✓ „Basis-ROM gefunden: …“, „Abbrechen“/„Einrichten“, im Lauf „Wird eingerichtet …“), „Textprojekt zurücknehmen?“, „ID umbenennen“ (Feld „Neue ID“, Plan „2 Verweise in 2 Dateien werden nachgezogen.“). Die ID steht im Kopf von Trainer, Pokédex und Header als `[data-entity-id]` mit „ID kopieren“, „ID umbenennen…“ im Menü „Aktionen“. Im Assistenten „Neues Spiel aus ROM“ das Kästchen „Als Textprojekt einrichten“ im Schritt „Vorlage“ und die Phase „Textprojekt einrichten“ im Ablauf. Agentenfrage `[data-agent-question]` „Als Textprojekt einrichten“.

| Punkt | Wer | Wie |
|---|---|---|
| (1) `scripts/gate.sh` | Agent | im Hintergrund, aus dem Worktree mit absolutem `G4_ROMS_DIR` und `tools`-Symlink; die neuen gated Tests laufen im Schritt der gated Tests je Projekt (Debug) und im Schritt `project_start_` je `.nds` mit, das Skript braucht dafür nichts Neues |
| (1) gated Tests einzeln | Agent | je Projekt im Release-Profil, nacheinander (Last mit `uptime` je Lauf), Befehle unten; die Zeilen `store`, `sync-full`, `sync`, `conflict`, `adopt`, `revert`, `checks`, `recipe`, `refs`, `ids`, `rename`, `adopt: … ms` und `<Spielcode>: phases …` in die Zeittabelle |
| (1) I3 (aus 4.3a) | Agent | auf der eingerichteten Platin-Kopie ein Byte in `data/species/0025.json` (`"attack": 55,` ↔ `56,`), dann `g4k build <Kopie> --out <rom> --timings`; aus der JSON-Zeile `steps[id=textproject].ms` und `ms`, sechs Läufe |
| (2) Szenario kopflos | Agent | Release-`g4k` und `studio mcp --project <Kopie> --base-rom <nds> [--allow-irreversible]` über JSON-RPC, Ablauf unten |
| (3) Browser | Agent | Test-Umgebung, Ablauf unten |
| (3) MCP über die Brücke | Agent | `mcp.sh`, Befehle unten |
| (4) DSPRE-Gegenprobe, Fokus im Tauri-Fenster, Fortschritt | Hand | siehe „Was beim Nutzer bleibt“ |
| Blick | Agent + Blick | Bilder `blick-*.png`, `gate3-*.png`, `mcp-*.png` unter `target/agent/gate-2026-10-28/4-3b/` |

**Gated Tests (1)**, aus der Wurzel des Worktrees (`K=$G4_ROMS_DIR/_testkopien`, `<k>` = `pt`, `hgde`, `hgus`):

```sh
export PATH="$HOME/.cargo/bin:/opt/homebrew/opt/rustup/bin:$PATH"
G4_TEST_PROJECT="$K/4-3b-<k>_DSPRE_contents" \
  cargo test --release -p g4-kernel --all-features --test textproject_source -- --nocapture --test-threads 1
G4_TEST_PROJECT="$K/4-3b-<k>_DSPRE_contents" \
  cargo test --release -p studio --all-features --lib mcp_with_ids_acts_like_numbers -- --nocapture --test-threads 1
G4_TEST_ROM="$K/4-3b-<k>.nds" \
  cargo test --release -p g4-kernel --all-features --test project_start -- project_start_text_project_builds_byte_equal --nocapture --test-threads 1
```

Mit `G4_TIMING_STRICT=1` prüfen `store_on_json_edits_only_the_entity_file_and_builds_like_dspre` (Speichern ≤ 200 ms) und `simulated_dspre_change_is_adopted_one_sided` (Fokus-Abgleich ≤ 500 ms, eine Änderung ≤ 1000 ms) ihre Grenzen; ohne die Variable drucken sie nur.

**Kopflos (2)**, nach `cargo build --release -p g4-kernel --bin g4k` und `cargo build --release -p studio --bin studio` (`P=$K/4-3b-<k>_DSPRE_contents`, `N=$K/4-3b-<k>.nds`, `G=target/release/g4k`):

```sh
$G base assign $P $N
target/release/studio mcp --project $P --base-rom $N        # text_project_status → dspre; adopt_text_project → needs_confirmation
target/release/studio mcp --project $P --base-rom $N --allow-irreversible
#   adopt_text_project → Bericht (phases, files, written, sync_files, ms)
$G build $P --out <rom>                                     # SHA-1 = $N
(cd $P && git init -q && $G text sources . | tr '\n' '\0' | xargs -0 git add && git commit -qm Ausgangsstand)
#   data_edit {"kind":"species","index":"SPECIES_PIKACHU","patch":{"Personal":{"field":"attack","value":<+1>}}}, save
git -C $P status --porcelain --untracked-files=no           # nur data/species/0025.json; undo, save; dasselbe mit "index": 25 → gleicher Diff
#   ein Byte in unpacked/moveData/0001 (Byte 3, die Stärke), dann:
$G text sync $P                                             # dspre data/moves/0001.json …
#   data/trainers/0001.json party[0].level und unpacked/trainerParty/0001 Byte 2..3 verschieden ändern:
$G text sync $P                                             # Exit 1, conflict data/trainers/0001.json …
#   external_changes {"diff":true}; resolve_external_conflict {"key":"data/trainers/0001.json","keep":"studio"}; erneut, dann "dspre"
#   rename_id {"id":<Trainer 2>,"new_id":<…_GATE>} → git diff --stat; undo → leer
$G build $P --out <rom>; (cd scripts/g4emu && uv run g4emu run --rom <rom> --out <ordner> new-game shot nach-intro)
#   die alten Member-Bytes zurückschreiben, $G text sync $P, $G build → SHA-1 = $N
$G text revert $P                                           # backup=…/.g4/backup/textproject-<Zeit> rewritten=… ms=…
$G build $P --out <rom>                                     # SHA-1 = $N
```

Im Gate bediente ein Python-Skript im Scratchpad den Server (die Handprobe aus [dev-setup.md](dev-setup.md), „MCP“). HeartGold US hat kein `intro` in `games.yaml`: dort `wait 1800 shot titel` statt `new-game`.

**Browser (3)**, Test-Umgebung auf eigenen Ports (`G4_ROMS_DIR=<absolut> G4_VITE_PORT=<v> G4_BRIDGE_PORT=<b> bash scripts/agent/studio-web.sh`), Seite `http://127.0.0.1:<v>/?project=<…>/_testkopien/4-3b-web_DSPRE_contents`, 1440 × 900. **Fokus-Trick:** Der Abgleich läuft beim Fensterfokus; ein Browser-Fenster, das ihn schon hat, bekommt keinen neuen. In der Seite `window.dispatchEvent(new Event("focus"))` auslösen (der Abgleich startet nach `FOCUS_DELAY_MS`, 800 ms; die Meldung kommt rund 1 bis 1,5 s danach). Eine Änderung „von DSPRE“ schreibt der Agent per Shell in einen Member (`python3 -c` oder `printf | dd conv=notrunc`), eine JSON-Änderung mit dem Editor oder per Shell in `data/…`. Ablauf: Öffnen zeigt den Hinweis; „Einrichten…“ (Prüfliste, Fortschritt nur „Wird eingerichtet …“, weil die Brücke keinen `Channel` hat); `git init` und Commit der Quellen in der Kopie; Pokédex › Pikachu (`[data-entity-id]` „SPECIES_PIKACHU“), Angriff ändern, speichern, `git -C <Kopie> status --porcelain --untracked-files=no`; Member-Byte, Fokus, Meldung und „Zuletzt übernommen“; beide Seiten, Fokus, Konflikt aufklappen, beide Tasten; ungespeichert plus Member; JSON mit Tippfehler; „ID umbenennen…“ eines Trainers, ⌘Z; „Hier spielen“ mit offenem Konflikt (fragt nicht nach dem Pfad, eine Meldung „Erst die Konflikte …“); „Zurücknehmen…“; Assistent mit Häkchen.

**MCP (3)** über die Brücke (Kopie `4-3b-mcp` mit zugeordneter Basis-ROM in der Seite geöffnet):

```sh
scripts/agent/call.sh agent_set_enabled '{"on":true}'
scripts/agent/call.sh agent_set_grant '{"level":"write"}'                    # „Lesen und bearbeiten“
scripts/agent/mcp.sh call adopt_text_project '{}' > target/agent/mcp-adopt.out 2>&1 &
scripts/agent/call.sh agent_status                                           # question: {kind: confirm, what: "Als Textprojekt einrichten"}
# im Browser-Fenster „Zulassen“; während des Laufs zeigt Technik › Textprojekt „Wird von einem Agenten eingerichtet …“
scripts/agent/mcp.sh call adopt_text_project '{}'                            # auf dem Textprojekt sofort abgelehnt: „Das Projekt ist schon ein Textprojekt.“
scripts/agent/mcp.sh call data_edit '{"kind":"species","index":"SPECIES_PIKACHU","patch":{"Personal":{"field":"attack","value":60}}}'   # ohne Rückfrage
scripts/agent/mcp.sh call rename_id '{"id":"TRAINER_TEENAGER_GUNTHER","new_id":"TRAINER_TEENAGER_GUNTHER_MCP"}'
scripts/agent/mcp.sh call history                                            # undo_agents: ["agent-gate", "agent-gate"]
```

## Phase 4.4 ([phase-4-4-gate.md](phase-4-4-gate.md))

4.4 bringt Git minimal: Vorlagen für `.gitignore` und `.gitattributes` je Layout, „Git einrichten…“ (Vorlagen, `git init`, erster Commit „Ausgangsstand“ über die `git`-CLI, nie ein Remote), Lesen über `gix`, Technik › Änderungen gegen Basis, letzten Commit und gespeicherten Stand mit Feldtabelle je Entität, die Statusleiste mit Branch und Zahl der geänderten Dateien und das Vervollständigen eines geklonten Textprojekts aus der eigenen Basis-ROM. Die Gate-Datei hat dazu den Abschnitt „Gate der Phase 4“. Die Punkte und Ergebnisse stehen in der Gate-Datei; hier nur, wer sie prüft und wie.

**Kopien:** frisch unter `$G4_ROMS_DIR/_testkopien` per `cp -cR` (Ordner) und `cp -c` (`.nds`), Ordner `4-4-<zweck>_DSPRE_contents` und `.nds` `4-4-<zweck>.nds` direkt nebeneinander: `pt`, `hgde`, `hgus` für die gated Tests (nur gelesen, die Tests schreiben auf `vanilla_copy()`), `kpt`, `khgde`, `khgus` (DSPRE-Layout) und `tpt`, `thgde`, `thgus` (Textprojekt) für das Szenario kopflos, `web`, `mcp`, `ui-text` und `wiz.nds` (Platin DE) für Browser und MCP über die Brücke. Repositories entstehen **nur in den Kopien**, nie im Repo des Studios; kein Remote, kein Push. Alle Kopien, Ausgabe-Ordner (`<name>-out/`) und Klone (`<name>-klon/`) nach dem Gate löschen (`Manual` und `_saves` bleiben). Ein offenes Finder-Fenster auf `_testkopien` schreibt `.DS_Store` in die Kopien: dann löscht erst ein zweites `rm -rf`.

**Klon ohne `git clone`:** ein Klon entsteht immer so (in Tests und im Gate gleich), damit kein Remote eingetragen wird:

```sh
mkdir <klon> && git -C <klon> init -q && git -C <klon> config core.autocrlf true \
  && git -C <klon> fetch -q <kopie> HEAD && git -C <klon> checkout -q FETCH_HEAD
```

Der Klon ist danach losgelöst (Statusleiste „<kurz-SHA> (losgelöst) · …“). Ein Klon eines Textprojekts hat keine Spieldateien: die App fragt beim Öffnen „Textprojekt vervollständigen?“, kopflos `g4k text restore <klon> --base-rom <nds>`; `studio mcp --project <klon>` startet bis dahin nicht und nennt den Satz von `restore_needed` mit `g4k text restore` (Exit 2).

**Wo was ist:** Statusleiste `[data-git-status]` („main · keine Änderung“, „main · 1 Änderung“, losgelöst „8507d3f (losgelöst) · …“, „Git: nicht lesbar“; gezählt werden Pfade wie bei `git status`, Klick öffnet Technik › Änderungen auf „Gegen letzten Commit“). Technik › „Änderungen“ (`[data-technik=aenderungen]`): Umschalter `[data-against]` („Gegen Basis“, „Gegen letzten Commit“, „Gegen gespeichert“), „Neu vergleichen“ `[data-compare-again]`, ohne Repo die Info „Dieses Projekt liegt in keinem Git-Repository.“ und „Git einrichten…“ in `[data-git-offer]`; je Gruppe `[data-changes-group=<Gruppe>]`, je Zeile `[data-change=<Name>]` mit Plakette „Geändert“, „Neu“, „Gelöscht“, „Fehlt“, „Ungespeichert“; das Detail `[data-entity-diff]` mit Kopf „Commit 5498fd6 „Ausgangsstand“ → Projekt (gespeichert)“, „Öffnen“, „Auf Original zurücksetzen…“ und der Änderungstabelle (Texte mit `<ins>`/`<del>`, Scripts `[data-hunk]`/`[data-line]`, Karten `[data-map-parts]`). Dialoge „Git einrichten?“ (`[data-git-init-dialog]`, Prüfliste `[data-check]`, Fortschritt `[data-git-init-progress]`) und „Textprojekt vervollständigen?“ (`[data-restore-dialog]`, `[data-restore-progress]`). Meldungen „Git eingerichtet: 18.450 Dateien im ersten Commit „Ausgangsstand“.“, „Textprojekt vervollständigt in 63 s.“. Im Assistenten „Neues Spiel aus ROM“ das Kästchen „Git einrichten (nur lokal)“ unter „Als Textprojekt einrichten“, in der Palette ohne Repo „Git einrichten…“. Agentenfrage `[data-agent-question]` „Git einrichten“.

| Punkt | Wer | Wie |
|---|---|---|
| (1) `scripts/gate.sh` | Agent | im Hintergrund, allein, aus dem Worktree mit absolutem `G4_ROMS_DIR` und `tools`-Symlink; die neuen gated Tests laufen im Schritt der gated Tests je Projekt (Debug) mit, das Skript braucht dafür nichts Neues. Ohne `git` im `PATH` melden sie `skipped: git not found` und bleiben grün: vorher `git --version` notieren |
| (1) gated Tests einzeln | Agent | je Projekt im Release-Profil, nacheinander (Last mit `uptime` je Lauf), Befehle unten; die Zeilen `git …`, `eol …`, `clone …`, `restore_phases …`, `text …`, `entries …`, `diff …` und `git init on the vanilla copy: …` in die Zahlentabelle |
| (2) Szenario kopflos | Agent | Release-`g4k` und `studio mcp --project <Kopie> --base-rom <nds> [--allow-irreversible]` über JSON-RPC, Ablauf unten |
| (3) Browser | Agent | Test-Umgebung, Ablauf unten |
| (3) MCP über die Brücke | Agent | `mcp.sh`, Befehle unten |
| (4) eigenes Git-Werkzeug, gepackte App aus dem Finder, Fortschritt, Windows | Hand | siehe „Was beim Nutzer bleibt“ |
| Blick | Agent + Blick | Bilder `blick-*.png` unter `target/agent/gate-2026-10-29/4-4/` |

**Gated Tests (1)**, aus der Wurzel des Worktrees (`K=$G4_ROMS_DIR/_testkopien`, `<k>` = `pt`, `hgde`, `hgus`):

```sh
export PATH="$HOME/.cargo/bin:/opt/homebrew/opt/rustup/bin:$PATH"
G4_TEST_PROJECT="$K/4-4-<k>_DSPRE_contents" \
  cargo test --release -p g4-kernel --all-features --test git -- --nocapture --test-threads 1
G4_TEST_PROJECT="$K/4-4-<k>_DSPRE_contents" \
  cargo test --release -p g4-kernel --all-features --test diff -- --nocapture --test-threads 1
G4_TEST_PROJECT="$K/4-4-<k>_DSPRE_contents" \
  cargo test --release -p studio --all-features --lib git_and_diff_over_the_command_path_on_the_vanilla_copy -- --nocapture --test-threads 1
```

Die Tests richten Git in einer eigenen Umgebung ein (`GIT_CONFIG_GLOBAL` leer, `GIT_CONFIG_NOSYSTEM=1`, `HOME` im Temp-Ordner, Wartung aus); die Einstellungen des Nutzers wirken nicht. Mit `G4_TIMING_STRICT=1` prüft `trainer_and_text_changes_show_with_field_names_against_head_and_base` den Status (≤ 500 ms) und `one_entity_diffs_fast_in_the_dspre_layout` jeden Diff (< 200 ms); ohne die Variable drucken sie nur. Die Grenzen der Größe (DSPRE ≤ 65 MiB und ≤ 20 027 Dateien, Textprojekt ≤ 10 MiB) gelten immer.

**Kopflos (2)**, nach `cargo build --release -p g4-kernel --bin g4k` und `cargo build --release -p studio --bin studio` (`G=target/release/g4k`):

```sh
P=$K/4-4-k<k>_DSPRE_contents; N=$K/4-4-k<k>.nds
$G base assign $P $N
target/release/studio mcp --project $P --base-rom $N        # git_status → repo.kind none; git_init → needs_confirmation
target/release/studio mcp --project $P --base-rom $N --allow-irreversible
#   git_init → committed, files, ignored_archives; git_status → changes 0
#   data_edit Trainer 1 (Level +1), edit_text Archiv 1 Nachricht 0, save; git_status → 2 Einträge
#   diff_entity {"kind":"trainer","index":1,"against":"head"}, {"kind":"text_archive","index":1,"against":"head"}, trainer/base
#   weitere Bearbeitung ohne save → diff_entity saved; reset_to_original Trainer 1, save → diff_entity base → same
$G build $P --out <rom>; git -C $P status --porcelain; git -C $P remote -v    # SHA-1 = $N; leer; leer
T=$K/4-4-t<k>_DSPRE_contents
$G base assign $T $K/4-4-t<k>.nds; $G text adopt $T; $G git init $T          # committed Ausgangsstand files=…
$G build $T --out <rom>; git -C $T status --porcelain                        # SHA-1 = .nds; leer
#   Klon wie oben, dann:
$G text restore <klon> --base-rom $K/4-4-t<k>.nds; $G build <klon> --out <rom>   # SHA-1 = .nds; git -C <klon> status leer
#   studio mcp auf $T: data_edit {"kind":"species","index":"SPECIES_PIKACHU",…attack +1}, save, dann:
$G diff $T species SPECIES_PIKACHU --against base             # field personal.attack 55 -> 56
$G git status $T --entries                                     # changed Pokémon Pokémon 0025
```

Im Gate bediente ein Python-Skript im Scratchpad den Server (Protokoll 2025-11-25).

**Browser (3)**, Test-Umgebung auf eigenen Ports (`G4_ROMS_DIR=<absolut> G4_VITE_PORT=<v> G4_BRIDGE_PORT=<b> bash scripts/agent/studio-web.sh`), Seite `http://127.0.0.1:<v>/?project=<…>/_testkopien/4-4-web_DSPRE_contents`, 1440 × 900. Ablauf: Technik › Änderungen „Gegen letzten Commit“ ohne Repo, „Git einrichten…“ (Prüfliste, Fortschritt nur „Archive auspacken …“, Meldung), Statusleiste und `git -C <Kopie> log`/`remote -v`/`status --porcelain`; Trainer ändern und speichern, Statusleiste, Klick darauf, Detail, „Öffnen“, „Auf Original zurücksetzen…“ mit ⌘Z; „Gegen Basis“, „Gegen gespeichert“ nach einer ungespeicherten Änderung; eine Nachricht ändern (Wort-Spannen). **Commit von außen:** `git -C <Kopie> commit -qam "Agent"`, dann in der Seite `window.dispatchEvent(new Event("focus"))` (Status nach `FOCUS_DELAY_MS`, 800 ms, gelesen, rund 0,9 s). Im DSPRE-Layout schreibt das Speichern einer Nachricht den ungetrackten Spiegel `expanded/textArchives/NNNN.json`, den `commit -qam` auslässt: „keine Änderung“ kommt dann erst nach `git add -A && git commit`, oder den Schritt vor der Nachricht machen. Danach der Assistent mit beiden Häkchen (Ziel unter `_testkopien` ohne Endung `_DSPRE_contents`, die der Browser-Modus als Ziel ablehnt) und der Klon von `4-4-ui-text` (Dialog „Textprojekt vervollständigen?“). Kompakt bei offenem Modal nur über `document.documentElement.dataset.dichte`, weil die Taste der Statusleiste unter dem Modal liegt.

**MCP (3)** über die Brücke (Kopie `4-4-mcp` mit zugeordneter Basis-ROM in der Seite geöffnet):

```sh
scripts/agent/call.sh agent_set_enabled '{"on":true}'
scripts/agent/call.sh agent_set_grant '{"level":"write"}'                    # „Lesen und bearbeiten“
scripts/agent/mcp.sh call git_status '{}'                                    # repo.kind none, cli.version, layout dspre
scripts/agent/mcp.sh call git_init '{}' > target/agent/mcp-git-init.out 2>&1 &
scripts/agent/call.sh agent_status                                           # question: {kind: confirm, what: "Git einrichten"}
# im Browser-Fenster „Zulassen“, oder: scripts/agent/call.sh agent_answer '{"question":<id>,"answer":"allow"}'
scripts/agent/mcp.sh call diff_entity '{"kind":"trainer","index":1,"against":"head"}'   # ohne Rückfrage
scripts/agent/mcp.sh call diff_entity '{"kind":"file","path":"../x","against":"head"}'  # not_diffable
git -C <Kopie> remote -v; git -C <Kopie> status --porcelain                  # beide leer
```

## Phase 3d.1 ([phase-3d-1-gate.md](phase-3d-1-gate.md))

3d.1 bringt Flags und Variablen: die pret-Namen je Familie (aus den Tabellen `flags` und `vars` der gebündelten Befehlsdatenbank), die Bereiche (`crates/g4-kernel/metadata/flag_ranges.yaml`), eigene Namen im Textprojekt (Register `data/ids/flags.json` und `vars.json`, Entitätsdatei `data/flags/NNNN.json` bzw. `data/vars/NNNN.json`, rotoms `.g4-ids-flags.json`), den Nutzungsindex mit Zugriff (Scripts, `hidden_flag` der Objekte, versteckte Items, Auslöser, Level-Scripts, Trainer-Flags), den Pool „frei“, Technik › „Flags und Variablen“, die Picker im Inspector, die MCP-Werkzeuge `list_flags`, `flag_usage`, `free_flag`, `name_flag` und die `g4emu`-Schritte für Flags und Variablen im Spielstand (oben, „Flags und Variablen im Spielstand“). Die Punkte und Ergebnisse stehen in der Gate-Datei; hier nur, wer sie prüft und wie.

**Kopien:** unter `$G4_ROMS_DIR/_testkopien` per `cp -cR` (Ordner) und `cp -c` (`.nds`), Ordner und `.nds` direkt nebeneinander: `3d-1-pt` + `3d-1-pt.nds`, `3d-1-hgde` + `3d-1-hgde.nds`, `3d-1-hgus_DSPRE_contents` + `3d-1-hgus.nds` für die gated Tests (nur gelesen; `tests/flags.rs` übernimmt die Scripts einmal je Lauf unter `target/tmp/flags-<Ordner>` und findet die Basis-ROM über `suggest_base` oder als `<Ordner>.nds` neben der Kopie; geschrieben wird nur auf `vanilla_copy()`); `3d-1-szen-pt2_DSPRE_contents` und `3d-1-szen-hg_DSPRE_contents` (je mit `.nds`) für die Szenarien, je Lauf eine frische (nach `build_rom` ist die Kopie geändert); `3d-1-web` (Platin DE, Scripts übernommen, Basis zugeordnet), `3d-1-mcp` (Platin DE, Basis zugeordnet) und `3d-1-ui-text` (Textprojekt) für Browser und MCP über die Brücke. Andere gated Tests, die `test_support::vanilla_base_rom()` nutzen, überspringen sich auf Kopien ohne Endung `_DSPRE_contents`.

**Wo was ist:** Technik › „Flags und Variablen“ (`[data-technik=flags]`, Untertitel z. B. „4.097 Flags · 101 frei“ (Platin DE mit übernommenen Scripts), in der Palette „Flags und Variablen“): Filterzeile `[data-flag-filter]` („Flags | Variablen“, Bereiche, „nur unbenutzt“, „nur frei“, „nur mit eigenem Namen“, Suche „Name oder Nummer suchen“), Liste `[data-flag-list]` mit Zeilen `[data-flag-row=<Nummer dezimal>]`, Plaketten `[data-flag-badge=free|reserved]`, Spalte „Live“ (`[data-flag-live-head]`, Zellen `[data-flag-live]` „–“), Fuß `[data-flag-list-foot]`; Detail `[data-flag-detail]` mit Abschnitten `[data-flag-section]` („Wo gesetzt“, „Wo geprüft“, „Unklar“), Gruppen `[data-flag-group]` und Wegen `[data-flag-use=<read|write|read_write|unknown>]`, Tasten „Als eigenes Flag benennen…“, „Umbenennen…“, „Beschreibung…“ (im DSPRE-Layout gesperrt mit „Kein Textprojekt: IDs gibt es erst nach „Als Textprojekt einrichten…“. (Technik › Textprojekt)“). Im Inspector das Feld `[data-flag-field=<Schlüssel>]` („Sichtbarkeits-Flag“ `hidden_flag` am Objekt, „Variable“ `var` und „Wert“ am Auslöser, im Level-Script `var:<Index>`) mit „Wählen…“; der Picker mit „Keins (0)“ als erster Zeile und dem Fuß `[data-flag-picker=<flag|var>]` („Freies Flag nehmen“, ohne übernommene Scripts gesperrt mit „Erst Scripts übernehmen: …“, „Im Verzeichnis zeigen“). Dialog `[data-name-flag-dialog]` („Als eigenes Flag benennen“, „Eigener Name“ vorbelegt `FLAG_`, „Beschreibung“, „Abbrechen“ / „Benennen“), Meldung „FLAG_X benannt (ein Schritt im Verlauf).“.

| Punkt | Wer | Wie |
|---|---|---|
| (1) Automatik | Agent | Schritt für Schritt, Befehle unten; `scripts/gate.sh` als Ganzes läuft auf main nach dem Merge (dort laufen die neuen gated Tests im Schritt der gated Tests je Projekt mit, `vanilla_index_builds_under_a_second` im Zeitschritt); `uptime` vor und nach jedem Schritt |
| (1) gated Tests | Agent | je Projekt im Release-Profil, nacheinander; die Zeilen `names …`, `usage …`, `recount …`, `index …`, `pool …`, `own …`, `adopt …` und `flags …` in die Zahlentabelle |
| (2) Szenarien `pt-flag`, `hg-flag` | Agent | `g4emu scenario` über MCP, Befehle unten; Bilder `gespraech.png`, `weg.png` |
| (2) kopflos | Agent | Release-`studio mcp --project <Kopie> --base-rom <nds>` über JSON-RPC: `list_flags`, `flag_usage` |
| (3) Browser | Agent | Test-Umgebung, Ablauf unten |
| (3) MCP über die Brücke | Agent | `mcp.sh`, Befehle unten |
| (4) eigener Name in DSPRE, HeartGold US im Emulator | Hand | siehe „Was beim Nutzer bleibt“ |
| Blick | Agent + Blick | Bilder `blick-*.png` aus dem UI-Test unter `target/agent/gate-<datum>/3d-1/` |

**Automatik (1)**, aus der Wurzel des Worktrees (`K=$G4_ROMS_DIR/_testkopien`; `P` nacheinander `$K/3d-1-pt`, `$K/3d-1-hgde`, `$K/3d-1-hgus_DSPRE_contents`):

```sh
export PATH="$HOME/.cargo/bin:/opt/homebrew/opt/rustup/bin:$PATH"
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
env -u G4_TEST_PROJECT -u G4_TEST_ROM -u G4_DSROM_CLI -u G4_APICULA -u G4_ROTOM_LSP cargo test -p g4-kernel --all-features
env -u G4_TEST_PROJECT -u G4_TEST_ROM -u G4_DSROM_CLI -u G4_APICULA -u G4_ROTOM_LSP cargo test -p studio --all-features
(cd apps/studio && pnpm test && pnpm build)
bash scripts/check-styles.sh; bash scripts/check-tokens.sh
# g4emu Unit und Integration; die Integration öffnet die drei .nds unter G4_ROMS_DIR, darum auf Klone umlenken:
S=<scratchpad>/roms3d1; mkdir -p $S/_testkopien; ln -sfn $G4_ROMS_DIR/_testkopien/_saves $S/_testkopien/_saves
for f in "Pokemon - Platin-Edition (Germany).nds" "Pokemon - Goldene Edition HeartGold (Germany).nds" \
         "Pokemon - HeartGold Version (USA).nds"; do cp -c "$G4_ROMS_DIR/$f" "$S/$f"; done
env G4_ROMS_DIR=$S bash -c 'cd scripts/g4emu && uv run pytest -q'
cargo build --release -p studio --bin studio; cargo build --release -p g4-kernel --bin g4k
G4_TEST_PROJECT=$P G4_TIMING_STRICT=1 \
  cargo test --release -p g4-kernel --all-features --test flags -- --nocapture --test-threads 1
G4_TEST_PROJECT=$P G4_TIMING_STRICT=1 \
  cargo test --release -p g4-kernel --all-features --test refs vanilla_index_builds_under_a_second -- --nocapture --test-threads 1
G4_TEST_PROJECT=$P \
  cargo test --release -p studio --all-features --lib flags_over_the_command_path_on_the_vanilla_copy -- --nocapture --test-threads 1
```

`tests/flags.rs` hat fünf Tests: `vanilla_flag_names_count_the_metadata` (Platin 4 097 Flags / 307 Variablen, HGSS 2 244 / 386), `vanilla_usage_matches_a_recount_of_the_sources` (Index gegen eine Nachzählung ohne Index), `vanilla_flag_index_is_fast` (mit `G4_TIMING_STRICT=1`: warm ≤ 1 000 ms, 200 Abfragen ≤ 50 ms, `usage` ≤ 50 ms; ohne die Variable druckt er nur), `vanilla_pool_counts_the_free_numbers` (80 bis 170 freie Flags, 10 bis 100 Variablen), `vanilla_own_name_compiles_byte_equal` (eigener Name kompiliert byte-gleich zur pret-Fassung und folgt `rename_id`). Die Scripts zu übernehmen kostet je Lauf 50 bis 65 s bei Last 5 bis 15 (191 s bei Last 21). Der Studio-Test arbeitet auf einer nicht übernommenen Kopie: `scripts_indexed=false`, `free_flag` → `scripts_not_adopted`, `name_flag` → `not_text_project`. Steht ein Zeittest unter Last rot, ihn einmal allein wiederholen und beide Werte mit `uptime` notieren. In `scripts/gate.sh` läuft `vanilla_flag_index_is_fast` nur nicht streng (Debug, ohne `G4_TIMING_STRICT`); die Grenzen der Spezifikation prüft dieser strenge Release-Lauf.

**Szenarien (2)**, über MCP (`mcp: [--allow-irreversible, --expert]`: Basis zuordnen und Patch sind unumkehrbar, `new_script_file` und `set_permission` gehen über `g4_call`), aus `scripts/g4emu`, nach `cargo build --workspace --all-features` im Worktree (`W` Worktree, `R` der absolute `G4_ROMS_DIR`):

```sh
export G4_ROMS_DIR=$R G4_STUDIO_BIN=$W/target/debug/studio G4K_BIN=$W/target/debug/g4k
O=$W/target/agent/gate-$(date +%F)/3d-1
uv run g4emu scenario $W/scripts/agent/scenarios/pt-flag.yaml --project $R/_testkopien/3d-1-szen-pt2_DSPRE_contents \
  --out $O/pt-flag --set BASIS=../3d-1-szen-pt2.nds --set NEUER_HEADER=593 --set X_FIGUR=54 --set Z_FIGUR=17
uv run g4emu scenario $W/scripts/agent/scenarios/hg-flag.yaml --project $R/_testkopien/3d-1-szen-hg_DSPRE_contents \
  --out $O/hg-flag --set BASIS=../3d-1-szen-hg.nds --set NEUER_HEADER=540 --set X_FIGUR=15 --set Z_FIGUR=13
```

Platzhalter: `BASIS` ist die `.nds` neben der Kopie, relativ zum Projektordner (`../<Kopie>.nds`); die Szenarien beginnen mit `assign_base`, weil `adopt_text_project` ohne Basis-ROM mit `base_missing` ablehnt. `NEUER_HEADER` ist die Zahl der Header vorher (Platin 593, HeartGold 540). `X_FIGUR`/`Z_FIGUR` ist die Kachel der Figur (Platin 54/17, HeartGold 15/13, aus den Bewegungsrechten der Karten 6 bzw. 1 neben dem hohen Gras); die Haltestellen sind `${Z_FIGUR+1}` (Figur steht) und `Z_FIGUR` (Figur weg). Erwartet: `"ok": true`, Header-Folge 415, 593, 415, 593 bzw. 64, 540, 64, 540, drei `expect-flag`-Zeilen (0, 1, 1). Bei Rückgabe 1 oder 2 zuerst `mcp.jsonl` und `mcp-stderr.log` lesen; eine Kopie nach `build_rom` ist verbraucht.

**Kopflos (2)**, auf der Szenario-Kopie nach dem Lauf (Textprojekt mit `FLAG_SZENARIO_FIGUR`), nach `cargo build --release -p studio --bin studio`. Die Kopie heißt `3d-1-szen-pt2`, weil der erste Lauf auf `3d-1-szen-pt` (noch ohne den Schritt `assign_base`) an `adopt_text_project` mit `base_missing` scheiterte, nachdem `apply_header_patch` die Kopie schon geändert hatte; sie wurde gelöscht. Im Gate bediente ein Python-Skript im Scratchpad den Server (JSON-RPC über stdio, Protokoll 2025-11-25).

```sh
K=$G4_ROMS_DIR/_testkopien
target/release/studio mcp --project $K/3d-1-szen-pt2_DSPRE_contents --base-rom $K/3d-1-szen-pt2.nds
#   list_flags {"kind":"flag"}                                → entries.length ≥ 4097, scripts_indexed true
#   flag_usage {"kind":"flag","number":66}                    → ein Objekt-Verweis hidden_flag, ein Script-Verweis SetFlag write
#   flag_usage {"kind":"flag","number":"FLAG_SZENARIO_FIGUR"} → dieselben Verweise
```

**Browser (3)**, Test-Umgebung auf eigenen Ports (`G4_ROMS_DIR=<absolut> G4_VITE_PORT=1700 G4_BRIDGE_PORT=1701 bash scripts/agent/studio-web.sh`), Seite `http://127.0.0.1:1700/?project=<…>/_testkopien/3d-1-web_DSPRE_contents`, 1440 × 900. Ablauf: Technik › „Flags und Variablen“ (Untertitel „4.097 Flags · … frei“), Filter „nur frei“, eine Zeile wählen; im Detail ein Script-Weg öffnet den Script-Editor an der Zeile, eine Figur wählt sie in der Schau; im Inspector einer Figur „Sichtbarkeits-Flag“ › „Wählen…“ › „Freies Flag nehmen“, speichern, `scripts/agent/call.sh get_events '{"eventFileId":<n>}'` (die Brücke nimmt camelCase) zeigt `hidden_flag`; im Level-Script „Neue Variable“ über den Picker; im DSPRE-Layout „Als eigenes Flag benennen…“ gesperrt mit Grund. Auf `3d-1-ui-text` (Textprojekt): benennen → `data/ids/flags.json`, `data/flags/NNNN.json` und `.g4-ids-flags.json` auf der Platte; „Umbenennen…“ → Script-Quelle und Entitätsdatei folgen, ⌘Z nimmt es als ein Schritt zurück. Speichert der Script-Editor und fragt die Fläche innerhalb von 250 ms, zeigt sie bis zur nächsten Anfrage den alten Stand (Zeitsperre des Index).

**MCP (3)** über die Brücke (Kopie `3d-1-mcp` mit zugeordneter Basis-ROM in der Seite geöffnet, Scripts nicht übernommen):

```sh
scripts/agent/call.sh agent_set_enabled '{"on":true}'
scripts/agent/call.sh agent_set_grant '{"level":"write"}'                         # „Lesen und bearbeiten“
scripts/agent/mcp.sh call list_flags '{"kind":"flag"}'                             # entries ≥ 4097, scripts_indexed false
scripts/agent/mcp.sh call flag_usage '{"kind":"flag","number":"FLAG_UNUSED_0x0094"}'
scripts/agent/mcp.sh call free_flag '{"kind":"var"}'                               # scripts_not_adopted (ohne übernommene Scripts)
scripts/agent/mcp.sh call name_flag '{"kind":"flag","number":148,"id":"FLAG_GATE"}' # not_text_project im DSPRE-Layout
```

## Phase 3d.2 ([phase-3d-2-gate.md](phase-3d-2-gate.md))

3d.2 bringt den Rahmen von Erzählen: die dritte Tiefe „Erzählen“ app-weit (Helfer `depthAtLeast`/`viewDepth`, Lint `scripts/check-depth.sh` als Schritt „depth“ im Gate-Skript), den reduzierten Inspector der Welt (nur Felder mit `from: "erzaehlen"`, Picker statt Zahlen), die Rezept-Leiste statt der Werkzeugleiste, die Rezepte Gebiet anlegen, Raum anlegen, Gebiet erweitern und Welt leeren im Kernel (`recipes`, `plan` ohne Schreiben, `apply` als ein Undo-Schritt, Scripts und Textarchive als Dateischritte), MCP `recipe_list`, `recipe_plan`, `recipe_apply` und `g4k recipe`, den Entwurf in Schau, Weltkarte und Raumplan und den nächsten Schritt als Rezept-Taste im Ergebnis. Die Punkte und Ergebnisse stehen in der Gate-Datei; hier nur, wer sie prüft und wie.

**Kopien:** frisch unter `$G4_ROMS_DIR/_testkopien` per `cp -cR` (Ordner) und `cp -c` (`.nds`), Ordner `3d-2-<zweck>_DSPRE_contents` und `.nds` `3d-2-<zweck>.nds` nebeneinander: `pt`, `hgde`, `hgus` für die gated Tests (nur gelesen, die Tests schreiben auf `vanilla_copy()`), `kpt`, `khgde`, `khgus` für das Szenario kopflos (vorher `g4k git init`), `raum` für `pt-raum-assistent.yaml`, `web` (Platin DE), `webhg` (HeartGold DE) und `mcp` (Platin DE) für Browser und MCP über die Brücke (je `g4k git init`, damit `git status` etwas sagt); für `pt-leere-region.yaml` und `hg-leere-region.yaml` nur die `.nds` `3d-2-leer-{pt,hg}.nds`, das Projekt entsteht mit `g4k create <nds> _testkopien/3d-2-leer-{pt,hg}`. Alle `3d-2-*` nach dem Gate löschen.

**Wo was ist:** Segment „Tiefe“ im Scharnier (`[role=group][aria-label=Tiefe]`, Tasten „Erzählen“, „Bauen“, „Tüfteln“ mit `aria-pressed`). In Erzählen schwebt über der 3D-Ansicht `[role=toolbar][aria-label=Rezepte]` mit `[data-recipe=create_area]` „Gebiet anlegen“, `[data-recipe=create_room]` „Raum anlegen“ und gesperrt `new_figure`, `connect_doors`, `new_trainer_here` (`disabled`, nur Symbol mit Schloss, Name und Grund im Tooltip und im `aria-label`, etwa „Tür verbinden“ · „Kommt mit 3d.5“; seit `c692e3d` bleibt die Leiste in einer Zeile, 520,7 × 50 px bei 1440 × 900); die gesperrten Tasten bekommen keinen Fokus, Pfeiltasten wandern nur über die freien. Inspector in Erzählen: Figur „Aussehen“ (Sprite und „Ändern…“), „Bewegung“ (Wörter, etwa „Geht umher“), „Blickrichtung“, „Trainer“ („kein Trainer“, „Ändern…“); Tür „Ziel“ (Referenz und „Ändern…“) und „Tür“ („Tür 0 · x 6, z 10“, „Ändern…“); Schild die Info „Der Text des Schilds kommt mit 3d.5.“, Auslöser „Geschichte und Schritt kommen mit 3d.7.“; Gebiet und Raum ein Abschnitt „Ort“ mit „Name“ (beim Raum „Umbenennen…“), „Ort“, „Aussehen“, „Musik (Tag)“, „Musik (Nacht)“. Entwurf: `[data-draft][data-draft-count]` am Host der 3D-Ansicht, `[data-draft-cell="x,y"]` auf Weltkarte und Raumplan (Zeile „Welt, Schau“ oben). Ergebnis eines Assistenten: unter „Als Nächstes“ je Schritt eine Rezept-Taste `[data-recipe]` (Raum: „Neue Figur hier“ gesperrt mit „Kommt mit 3d.5“, über `aria-describedby`; Welt leeren: „Gebiet anlegen“ frei, öffnet den Assistenten). Agentenfrage `[data-agent-question]` „„agent-gate“ möchte: Welt leeren (lässt sich nicht rückgängig machen).“.

**Zählung über die Test-API:** `[data-draft-count]` ist gleich `draftCount(<Antwort von recipe_plan>, <gezeigte Matrix>)` (Zellen, Kacheln und Events mit `matrix` der gezeigten Weltkarte). Die Eingabe, die der Assistent schickt, liest man im Browser mit, indem man vor dem Assistenten `window.fetch` umhüllt und den Körper der Anfragen an `/__bridge/invoke/recipe_plan` merkt; dann `scripts/agent/call.sh recipe_plan '<derselbe Körper>'` und selbst zählen. Für „Raum anlegen“ mit Eingang in Zweiblattdorf ist es 2 (Türkachel und Tür außen; die Zelle des Raums liegt auf einer neuen Weltkarte, `matrix: null`, und zählt nicht). Auf HeartGold ist Weltkarte 0 in der Weltkarte zweigeteilt (Johto `w0`, Kanto `w0k`): `[data-draft-cell]` zählt nur die Zellen der gezeigten Hälfte, `[data-draft-count]` alle der Matrix.

| Punkt | Wer | Wie |
|---|---|---|
| (1) Schritte von `scripts/gate.sh` | Agent | fmt, clippy, `cargo test -p g4-kernel --all-features`, `cargo test -p studio --all-features --lib`, `cargo test -p studio --test mcp_stdio` (alle drei ohne `G4_*`), `pnpm test`, `pnpm build`, `check-tokens.sh`, `check-styles.sh`, `check-depth.sh` (0 Treffer) einzeln; das ganze Skript läuft auf main nach dem Merge |
| (1) Snapshot | Agent | `cd apps/studio && npx vitest run src/workbench/inspector/inspectorFields.test.ts` |
| (1) gated Tests | Agent | je Projekt im Release-Profil, nacheinander, Befehle unten |
| (2) Szenario kopflos | Agent | Release-`studio mcp --project <Kopie> [--allow-irreversible]` über JSON-RPC/stdio, Ablauf unten; die Szenarien mit `g4emu` |
| (3) Browser | Agent | Test-Umgebung, Ablauf unten, Chrome headless 1440 × 900 |
| (3) MCP über die Brücke | Agent | `mcp.sh`, Befehle unten |
| (4) drei Tiefen in der gepackten App, Tastatur der Rezept-Leiste | Hand | siehe Gate-Datei, Abschnitt „Hand“ |
| Blick | Agent + Blick | Bilder `blick-*.png` unter `target/agent/gate-2026-10-31/3d-2/` |

**Gated Tests (1)**, aus der Wurzel des Worktrees (`K=$G4_ROMS_DIR/_testkopien`, `<k>` = `pt`, `hgde`, `hgus`):

```sh
export PATH="$HOME/.cargo/bin:/opt/homebrew/opt/rustup/bin:$PATH"
G4_TEST_PROJECT="$K/3d-2-<k>_DSPRE_contents" \
  cargo test --release -p g4-kernel --all-features --test recipes -- --nocapture --test-threads 1
G4_TEST_PROJECT="$K/3d-2-<k>_DSPRE_contents" \
  cargo test --release -p g4-kernel --all-features --test templates -- --nocapture --test-threads 1
```

`vanilla_room_recipe_matches_the_wizard_scenario` läuft nur auf Platin DE (die Zahlen der Spezifikation sind die von `pt-raum-assistent.yaml`) und meldet auf HeartGold `skipped`; `vanilla_empty_region_undo_restores_the_world` druckt die berührten und die gesicherten Textarchive und prüft `git status` nach ⌘Z (nur die Rand-Karte).

**Kopflos (2)**, nach `cargo build --release -p g4-kernel --bin g4k` und `cargo build --release -p studio --bin studio`:

```sh
P=$K/3d-2-k<k>_DSPRE_contents
target/release/g4k git init $P                               # Ausgangsstand, damit git status etwas sagt
target/release/studio mcp --project $P                       # ohne Erlaubnis:
#   recipe_list → vier Einträge
#   recipe_plan {"recipe":"create_room","input":<Spezifikation mit header.allow_patch true>} → cells 1, tiles 1, events 2
#   recipe_apply {"recipe":"empty_region","input":{}} → needs_confirmation „… (Welt leeren) …“
target/release/studio mcp --project $P --allow-irreversible --expert
#   recipe_apply extend_area (leere Zelle von Matrix 0) → history + 1; undo → get_matrix wie vorher
#   recipe_apply create_room → ein Schritt „Raum „…“ angelegt“, next new_figure gesperrt; save
#   g4_call adopt_scripts; save; danach außen: git add -A && git commit
#   recipe_apply empty_region → change, history + 1; git status nennt Scripts, Textarchiv, compile-state
#   undo → git status nur „?? unpacked/maps/<Rand-Karte>“; redo → wieder wie nach apply
```

Die Spezifikation des Raums ist die aus `pt-raum-assistent.yaml` (Platin DE: Header 593 aus Vorlage 415, Karte 187, Eventdatei „Kopie von 394“, Tür außen 411 bei x 105, z 876, innen x 8, z 4); auf HeartGold dieselbe Form mit dem Spielerzimmer (Header 540 aus Vorlage 64, Karte 217, Kopie von Eventdatei 61, Tür außen Neuborkia 60 bei x 686, z 397, innen x 3, z 4). Ohne `header.allow_patch` lehnt `recipe_plan` auf einem Spiel ohne Patch der Dynamic Headers ab wie `apply` („new headers need the dynamic headers patch, apply the patch first“); der Assistent fragt den Plan darum immer mit `allow_patch: true`. Im Gate bediente ein Python-Skript im Scratchpad den Server (Protokoll 2025-11-25); `g4_call` (für `adopt_scripts`) gibt es kopflos nur mit `--expert`, an der Brücke nur im Expertenmodus; das Übernehmen der Scripts geht in der Oberfläche über „Scripts übernehmen“ im Assistenten „Welt leeren“.

**Szenarien (2)**, aus `scripts/g4emu` mit `G4_ROMS_DIR` absolut:

```sh
uv run g4emu scenario ../agent/scenarios/pt-raum-assistent.yaml --project $K/3d-2-raum_DSPRE_contents \
  --out <aus>/scen-raum --set NEUER_HEADER=593 --set TUER_X=105 --set TUER_Y=876
target/release/g4k create $K/3d-2-leer-pt.nds $K/3d-2-leer-pt
uv run g4emu scenario ../agent/scenarios/pt-leere-region.yaml --project $K/3d-2-leer-pt --out <aus>/scen-leer-pt
# HeartGold DE ebenso mit 3d-2-leer-hg und hg-leere-region.yaml
```

**Browser (3)**, Test-Umgebung auf eigenen Ports (`G4_ROMS_DIR=<absolut> G4_VITE_PORT=<v> G4_BRIDGE_PORT=<b> bash scripts/agent/studio-web.sh`), Seite `http://127.0.0.1:<v>/?project=<…>/_testkopien/3d-2-web_DSPRE_contents`, 1440 × 900; Chrome headless mit `--use-angle=swiftshader --enable-unsafe-swiftshader` hat WebGL, die 3D-Ansicht und die Entwurf-Schicht zeichnen also auch im Browser-Modus. Hell und Dunkel über `Emulation.setEmulatedMedia` gelten nur, solange dieselbe CDP-Verbindung offen ist: Aussehen setzen und Bild aufnehmen in einer Verbindung (oder ein Daemon), sonst zeigt das Bild das Aussehen des Systems. Ablauf: Zweiblattdorf (`[data-area-key="w0:1"]`, HeartGold Neuborkia `[data-header-id="60"]`) in Bauen öffnen, `[data-dock-tab]` zählen; „Erzählen“ drücken: Rezept-Leiste statt Werkzeugleiste, Inspector des Gebiets, dann über „In dieser Szene“ (`[data-event="object:0"]`, `warp:0`, `bg:0`, `coord:0`) Figur, Tür, Schild, Auslöser und einen Raum des Orts; Technik, Pokémon (eine Art gewählt), Texte und Gegenstände je in Erzählen, Bauen und Tüfteln vergleichen (Text und `[data-dock-tab]` des sichtbaren Docks gleich in Erzählen und Bauen, „Rohdaten“ nur in Tüfteln, das Segment bleibt auf „Erzählen“), zurück in die Welt. „Raum anlegen“ aus der Leiste mit gewähltem Ort: Vorlage suchen und wählen, Name, „Weiter“, „Weiter“, „Außen: Kachel x“ und „z“ eintippen, „Weiter“ bis „Zusammenfassung“; `[data-draft-count]` mit `call.sh recipe_plan` vergleichen (oben), Esc (Fokus in der Seite) → `0` und `git status` leer; noch einmal bis zur Zusammenfassung, das Kästchen der Patch-Zeile über sein `label` anklicken (das `input` ist versteckt), „Anlegen“ → Ergebnis mit „Neue Figur hier“ gesperrt. Speichern (`call.sh save`), außen `git add -A && git commit`; Technik › Werkzeuge › „Welt leeren…“, „Scripts übernehmen“, „Speichern“, wieder außen committen, „Weiter“, „Welt leeren“ → Ergebnis mit „Gebiet anlegen“ (öffnet „Gebiet anlegen“, „Abbrechen“); in die Seite klicken, ⌘Z → `call.sh history` hat „Vorlage: Leere Region mit Startstadt“ unter `redo`, `call.sh dirty` leer, `git status` nur `?? unpacked/maps/<Rand-Karte>`.

**MCP (3)** über die Brücke (Kopie `3d-2-mcp` in der Seite geöffnet):

```sh
scripts/agent/call.sh agent_set_enabled '{"on":true}'
scripts/agent/call.sh agent_set_grant '{"level":"write"}'
scripts/agent/mcp.sh call recipe_plan '{"recipe":"create_room","input":{…,"header":{…,"allow_patch":true}}}'   # ohne Rückfrage
scripts/agent/mcp.sh call recipe_apply '{"recipe":"empty_region","input":{}}' &                               # wartet auf die Frage
scripts/agent/call.sh agent_status            # question: {kind: confirm, what: "Welt leeren"}
# im Browser-Fenster „Ablehnen“ → needs_confirmation „… in Gen4 Studio abgelehnt“, oder „Zulassen“
```

## Phase 3d.3 ([phase-3d-3-gate.md](phase-3d-3-gate.md), Teil A und B)

Teil A von 3d.3 bringt die Kampfsprites (nur lesen, PNG über `get_battle_sprite` und `get_trainer_sprite`), das DS-Profil in der Pokédex-Schau und die Kampfvorschau im Trainer. Teil B bringt die Ei-Attacken (Reiter „Ei-Attacken“ in der Pokédex-Hand, `egg_moves` in `data_get species`, Teil `EggMoves` von `data_edit`, `g4k check-data` mit KIND `egg_moves`); seine Befehle stehen unten unter „Teil B“. Die Punkte und Ergebnisse stehen in der Gate-Datei; hier nur, wer sie prüft und wie.

**Kopien:** frisch unter `$G4_ROMS_DIR/_testkopien` per `cp -cR` (Ordner) und `cp -c` (`.nds`), Ordner `3d-3-<zweck>_DSPRE_contents` und `.nds` `3d-3-<zweck>.nds` direkt nebeneinander: `pt`, `hgde`, `hgus` für die gated Tests (nur gelesen, die Tests schreiben auf `vanilla_copy()`), `web` (Platin DE) und `hgweb` (HeartGold DE) für Browser und MCP über die Brücke, dazu `basept` und `basehgde` (Namen `<stem>_DSPRE_contents`) für die gated Build-Tests von Task 3. Alle Kopien nach dem Gate (nach Teil B) löschen (`Manual` und `_saves` bleiben). Die Ordner der Kopien heißen in Wirklichkeit ohne `_DSPRE_contents` (`3d-3-pt`, `3d-3-hgde`, …), nur `3d-3-basept` und `3d-3-basehgde` tragen den Zusatz. Die Kopien von Teil B stehen unten.

**Ports:** `G4_VITE_PORT=1720 G4_BRIDGE_PORT=1721`. Der Port 1720 steht auf Chromes Liste unsicherer Ports (H.323): Chrome headless braucht `--explicitly-allowed-ports=1720,1721`, sonst scheitert die Seite mit `ERR_UNSAFE_PORT`. Seite `http://127.0.0.1:1720/?project=<Roms>/_testkopien/3d-3-web_DSPRE_contents`; die Galerie unter `?gallery` (Seite „Bildschirm“). Fensterhöhe 1500 statt 900, damit die Schau das ganze Profil über der Hand zeigt.

**Wo was ist:** Pokédex-Schau, Abschnitt „Spielerblick“ `[data-ds-profile-section]` mit dem Profil `[data-ds-profile]`, `Segmented` „Seite“ `[data-side]` (Vorderseite, Rückseite), Schalter „Weiblich“ `[data-female]` und „Schillernd“ `[data-shiny]`; gesperrt „Weiblich“ mit „Kein eigenes Bild für ♀“ oder „Nur ♀“ (`aria-describedby`). Trainer: Klassenbild `[data-trainer-sprite]` (Kopf der Schau 80, Kopf der Hand 48), Abschnitt „Kampfvorschau“ `[data-ds-battle-section]` mit `Segmented` „Gegnerseite“ `[data-foe]` (Trainer, Erstes Pokémon), dem Bild `[data-ds-battle]`, Boden `[data-ds-ground=foe|own]`, Statuskästen `[data-ds-status=foe|own]` und KP-Balken `role="meter"` mit `data-hp="gruen|gelb|rot"`. Bilder des Gates: `target/agent/gate-2026-10-31/3d-3/` (`blick-*.png`, `galerie-*.png`, `pokedex-*.png`, `montage-*.png`).

| Punkt | Wer | Wie |
|---|---|---|
| (1) `scripts/gate.sh` | Agent | im Hintergrund, allein, aus dem Worktree mit absolutem `G4_ROMS_DIR` und `tools`-Symlink; die neuen gated Tests laufen im Schritt der gated Tests je Projekt (Debug) mit |
| (1) gated Tests einzeln | Agent | je Projekt im Release-Profil, nacheinander (Last mit `uptime`), Befehle unten; die Zeilen `sprites …`, `forms …` und `sprite …` in die Zahlentabelle |
| (2) MCP und Zeit über die Brücke | Agent | `mcp.sh` und `call.sh`, Befehle unten |
| (3) Browser | Agent | Test-Umgebung auf 1720/1721, Ablauf unten |
| Bilder gegen das Spiel, `.fig`, Blick | Hand, Agent + Blick | siehe Gate-Datei, Abschnitt „(4) Hand und Blick“ |

**Gated Tests (1)**, aus der Wurzel des Worktrees (`K=$G4_ROMS_DIR/_testkopien`, `<k>` = `pt`, `hgde`, `hgus`):

```sh
export PATH="$HOME/.cargo/bin:/opt/homebrew/opt/rustup/bin:$PATH"
G4_TEST_PROJECT="$K/3d-3-<k>_DSPRE_contents" \
  cargo test --release -p g4-kernel --all-features --test battle_sprites -- --nocapture
#   23 bestanden, 1 ignoriert; Zeilen: sprites <Spielcode> pokemon=… forms=… trainers=… components=… corners=… palettes16=… ms=…
#   forms <Spielcode> images=… palettes=… entries=… rendered=… form0=… form0_like_pokegra=[…] female_only=…
G4_TIMING_STRICT=1 G4_TEST_PROJECT="$K/3d-3-<k>_DSPRE_contents" \
  cargo test -p studio --all-features --lib battle_sprites_over_the_command_path -- --nocapture
#   Debug-Build (ohne --release): erste Anfrage höchstens 50 ms; Zeile sprite <Spielcode> first_ms=… cached_ms=… trainer_ms=…
```

Der Studio-Test läuft für die 50-ms-Grenze im **Debug**-Build, nicht im Release-Profil. Ohne `G4_TIMING_STRICT=1` druckt er die Zeiten nur. Der Test `vanilla_every_battle_sprite_decodes_with_the_measured_counts` braucht je Spiel einige Sekunden (unter Last bis 17 s).

**MCP und Zeit (2)** über die Brücke (Kopie `3d-3-web` in der Seite geöffnet, Agenten an, Expertenwerkzeuge an):

```sh
scripts/agent/call.sh agent_set_enabled '{"on":true}'
time scripts/agent/call.sh get_battle_sprite '{"species":397,"side":"front"}'    # "<n> bytes" (die Brücke liefert das PNG), erste Anfrage
time scripts/agent/call.sh get_trainer_sprite '{"class":1}'                       # "<n> bytes"; Klasse 105 oder Art 600: 0 Bytes
scripts/agent/mcp.sh call g4_call '{"command":"get_battle_sprite","args":{"species":397,"side":"front"}}'   # {"result":{"bytes":1496}}
scripts/agent/mcp.sh call g4_call '{"command":"get_trainer_sprite","args":{"class":1,"frame":2}}'          # Fehler „frame must be 0 or 1“
scripts/agent/mcp.sh call data_get '{"kind":"species","index":397}'                                          # sprite_slots {female, male}
```

`g4_call` antwortet für Bildbefehle die Länge, kein Base64 (Ruling R7; wie `get_mon_icon`); die Zeit gilt für die erste Anfrage je Bild, danach liegt das Bild im Cache der Sitzung (um 2 ms).

**Browser (3)**, Test-Umgebung auf 1720/1721, Chrome mit `--headless=new` und `--explicitly-allowed-ports=1720,1721` über CDP, 1440 × 1500. Ablauf: Pokédex Staravia (397): „Spielerblick“, Vorderseite, Rückseite, „Schillernd“, „Weiblich“ (frei); 413 und 500 („Nur ♀“), 421, 412, 503, 494 und in HeartGold 172 („Kein eigenes Bild für ♀“); ⌘1 aus Trainer. Trainer 1: Klassenbild im Kopf von Schau und Hand, Kampfvorschau „Trainer“ und „Erstes Pokémon“. Danach Hell und Dunkel über `prefers-color-scheme` und Kompakt über die Taste der Statusleiste, jeweils ein Bild; die Galerie mit `?gallery`.

### Teil B: Ei-Attacken

**Kopien** (wie oben, `cp -cR` und `cp -c`; in diesem Lauf hießen die Ordner `3d-3-<zweck>` mit `3d-3-<zweck>.nds` daneben, nur die beiden Basis-Kopien trugen den Zusatz `_DSPRE_contents`): `kpt` (Platin DE) und `khgde` (HeartGold DE) für das Szenario kopflos, `patch` (Platin DE mit einem synthetischen `files/zcustom/egg_moves.narc`: eine Kopie von `files/a/2/2/9` aus HeartGold DE, die den HGSS-Member-Aufbau trägt; das Studio liest sie als DSPRE-Patch), `webb` (Platin DE) und `hgwebb` (HeartGold DE) für den Browser; die gated Tests laufen auf `pt`, `hgde`, `hgus` und für das Textprojekt auf `basept` und `basehgde` (Namen `<stem>_DSPRE_contents`). Den Patch-NARC legt man so an: in der Kopie `3d-3-patch` (Platin DE) `mkdir -p files/zcustom` und `files/a/2/2/9` der Kopie `3d-3-khgde` dorthin als `egg_moves.narc` kopieren (`cp -c`); er liegt nur in der Kopie, nie im Repo und nie in einem Original. Ports wie oben (1720/1721).

**Wo was ist:** Pokémon › Pokédex › Art (zum Beispiel Bisasam 0001) › Hand „Bearbeiten“ › Reiter „Ei-Attacken“ `[data-tab="eggMoves"]`: Tabelle „Ei-Attacken von BISASAM“ mit Zeilen `[data-egg]` (`data-move` = Attackennummer; Feld „Attacke in Zeile N“, „Typ“, Taste „<Attacke> entfernen“), „Zeile hinzufügen“ `[data-add-egg]` (gesperrt als `aria-disabled` mit dem Satz daneben und `aria-describedby`: „Tabelle voll: erst eine Ei-Attacke bei einer anderen Art entfernen“ oder „Höchstens 16 Ei-Attacken je Art“), die Fußzeile „14 Ei-Attacken“ (HeartGold) bzw. `[data-egg-budget]` „12 Ei-Attacken · Tabelle 1 910 von 1 910 Einträgen“ (Platin ohne Patch); gesperrte Tabelle: Warnung mit dem Grund („Ei-Attacken: Overlay 5 trägt an 0x2923E keine Tabelle (Signatur fehlt)“), Diamant und Perl: „Dieses Spiel hat keine Ei-Attacken-Tabelle, die das Studio kennt.“ Bilder: `target/agent/gate-2026-11-01/3d-3/` (`blick-pokedex-eiattacken-*.png`).

| Punkt | Wer | Wie |
|---|---|---|
| (1) `scripts/gate.sh` | Agent | wie in Teil A; die gated Tests der Ei-Attacken laufen im Schritt der gated Tests je Projekt (Debug) mit |
| (1) gated Tests einzeln | Agent | je Projekt im Release-Profil, Befehle unten; die Zeilen `egg_moves …` in die Zahlentabelle |
| (2) Szenario kopflos | Agent | `call.sh` und `g4k` im Release-Profil, Ablauf unten |
| (3) Browser | Agent | Test-Umgebung auf 1720/1721, Ablauf unten |
| Zucht in Platin, echtes DSPRE-Patch-Projekt, Blick, `.fig` | Hand, Agent + Blick | siehe Gate-Datei, Abschnitt „Teil B“, „(4) Hand und Blick“ |

**Gated Tests (1)**, aus der Wurzel des Worktrees (`K=$G4_ROMS_DIR/_testkopien`, `<k>` = `pt`, `hgde`, `hgus`; `<b>` = `basept`, `basehgde`):

```sh
export PATH="$HOME/.cargo/bin:/opt/homebrew/opt/rustup/bin:$PATH"
G4_TEST_PROJECT="$K/3d-3-<k>" \
  cargo test --release -p g4-kernel --all-features --test egg_moves -- --nocapture
#   14 bestanden; Zeilen: egg_moves <Spielcode> source=… species=… moves=… words=… ms=…  und  egg_moves <Spielcode> ascending=… first=… last=…
G4_ROMS_DIR=<Roms> G4_TEST_PROJECT="$K/3d-3-<b>_DSPRE_contents" \
  cargo test --release -p g4-kernel --all-features --test textproject_source -- --nocapture --test-threads 1 \
  text_project_egg_move_edit_builds_and_check_data_finds_it
G4_ROMS_DIR=<Roms> G4_TEST_PROJECT="$K/3d-3-<b>_DSPRE_contents" \
  cargo test --release -p g4-kernel --all-features --test textproject -- --nocapture --test-threads 1 \
  textproject_round_trip textproject_export_idempotent
#   egg-moves-text <Spielcode> count=<n> move0=<Attacke>; Round-Trip und Export laufen über das ganze Projekt, je Projekt mehrere Minuten
```

**Szenario kopflos (2)** auf `3d-3-khgde`, `3d-3-kpt` und `3d-3-patch` (je ein frischer Lauf der Brücke mit der Kopie geöffnet, Agenten an):

```sh
scripts/agent/call.sh data_get '{"kind":"species","index":1}'     # egg_moves {moves, source narc|overlay|patch, budget {used,total}|null, locked}
scripts/agent/call.sh data_edit '{"kind":"species","index":1,"patch":{"EggMoves":{"moves":[1,130,219, …]}}}'   # die ganze Liste; Tausch von move0 113 gegen 1
scripts/agent/call.sh undo                                         # stellt move0 wieder her
scripts/agent/call.sh save                                         # HeartGold: Member; Platin: Ausschnitt in ov005.bin; Patch: ov005.bin bleibt byte-gleich
scripts/agent/call.sh build_rom '{"out":"<Kopie>-built.nds"}'
target/release/g4k check-data <Kopie>-built.nds --expect egg_moves:1:count=14 --expect egg_moves:1:move0=1   # nur HeartGold (auf `3d-3-patch` lief kein `check-data`); Platin: „CPUD has no egg_moves archive“
```

Auf `3d-3-kpt` zusätzlich: `data_edit` mit einer um eine Attacke längeren Liste (Art 1 mit 13 statt 12) ohne Entfernen ergibt den Fehler „Ei-Attacken-Tabelle voll: 1 910 von 1 910 Einträgen“, `history` bleibt unverändert; ein Handel (Art 1 mit einer Attacke weniger, Art 7 mit einer mehr, zwei `data_edit`) geht. Auf `3d-3-patch`: `source` ist `patch`, `budget` `null`, und die SHA-Summe von `arm9_overlays/ov005.bin` ändert sich durch `save` nicht (`shasum -a 1` davor und danach). Bilder liefert das Szenario nicht, es gibt hier kein Bild-Argument (Ruling R7 betrifft nur `get_battle_sprite` und `get_trainer_sprite`).

**Browser (3)**, Test-Umgebung auf 1720/1721, Chrome mit `--headless=new` und `--explicitly-allowed-ports=1720,1721` über CDP, 1440 × 1500, Seite `http://127.0.0.1:1720/?project=<Roms>/_testkopien/3d-3-webb` (HeartGold: `3d-3-hgwebb`). Ablauf auf Platin: Pokédex › BISASAM › Reiter „Ei-Attacken“: 12 Zeilen, Fußzeile mit dem Budget, „Zeile hinzufügen“ gesperrt; letzte Zeile entfernen (11 Zeilen, „1 909 von 1 910“, Taste frei), ⌘Z (12, gesperrt); Zeile 1 über den Picker gegen „Pfund“ tauschen (Budget unverändert), ⌘Z. Auf HeartGold: 14 Zeilen, „Zeile hinzufügen“ frei, „Pfund“ hinzufügen (15), ⌘Z (14). Danach Hell und Dunkel über `prefers-color-scheme` und Kompakt über die Taste der Statusleiste, jeweils ein Bild; die Fußzeile liegt bei 1440 × 1500 unter dem Rand, ihre Werte kommen aus dem DOM.

## Phase 4.5 ([phase-4-5-gate.md](phase-4-5-gate.md))

4.5 bringt „Veröffentlichen“ als Zwischenphase: den Assistenten in vier Schritten (Version, Patch, Prüfen, Fertig) mit selbstgeprüftem xdelta-Patch, Begleitdatei, wahlweise `README.txt` und dem Eintrag in `.g4/releases.json`, die Taste in der Titelleiste, die Liste „Bisherige Veröffentlichungen“ in Technik › Veröffentlichen und „Stand sichern…“ (ein Commit mit allen Änderungen, nur lokal). Die Punkte und Ergebnisse stehen in der Gate-Datei; hier nur, wer sie prüft und wie.

**Kopien:** frisch unter `$G4_ROMS_DIR/_testkopien` per `cp -cR` (Ordner) und `cp -c` (`.nds`), Ordner `4-5-<zweck>_DSPRE_contents` und `.nds` `4-5-<zweck>.nds` direkt nebeneinander: `pt`, `hgde`, `hgus` für die gated Tests (nur gelesen, die Tests schreiben auf `vanilla_copy()`), `kpt`, `khgde`, `khgus` für das Szenario kopflos (je mit `g4k base assign`), `web`, `mcp` und `ui` (Platin DE, Basis zugeordnet, `g4k git init`) für Browser, MCP über die Brücke und die Bilder. Repositories entstehen **nur in den Kopien**, nie im Repo des Studios; kein Remote, kein Push. Die Test-Umgebung läuft nur auf `G4_VITE_PORT=1730 G4_BRIDGE_PORT=1731` (die übrigen Ports gehören den parallelen Phasen). `G4_ROMS_DIR` absolut, `G4_BASE_ROM_DIR=<Roms>/_testkopien`; `git --version` notieren (ohne `git` melden die Git-Tests `skipped: git not found`). Alle Kopien, Ausgabe-Ordner (`<name>-out/`, `<name>-ort/`) nach dem Gate löschen (`Manual` und `_saves` bleiben).

**Wo was ist:** Titelleiste, Taste „Veröffentlichen“ (Rolle `button`; gesperrt `aria-disabled` mit `aria-description` = Grund, Tooltip mit zweiter Zeile). Technik › „Veröffentlichen“ (`[data-technik=veroeffentlichen]`): Karte „Basis-ROM“ `[data-base-state]`, Karte „Veröffentlichen“ `[data-publish-card]` mit „Veröffentlichen…“ und der Tabelle „Bisherige Veröffentlichungen“ (Zeilen `[data-release=<Version>]`, „Im Finder zeigen“ je Zeile). Der Assistent (Rolle `dialog` „Veröffentlichen“, Atlas auf Welt): Schritte `[data-publish-step=version|patch|check]`, Pfad `[data-publish-dir]`, Lauf `[data-publish-progress]` (`role="status"`), „Ersetzen“ `[data-publish-overwrite]`, Vorschau `[data-publish-preview]`, im Ergebnis „Stand sichern…“ `[data-publish-commit]` (nur mit eigenem Repository). Der Dialog „Stand sichern?“ `[data-commit-dialog]` mit Prüfliste `[data-check]`, Feldern „Betreff“ und „Notiz“, Lauf `[data-commit-progress]`; in Technik › Änderungen „Gegen letzten Commit“ die Taste `[data-commit-all]`; die Statusleiste `[data-git-status]` danach „main · keine Änderung“. Meldungen: Assistent „Veröffentlicht: v0.1.0“, Toast „Stand gesichert: <kurz> „<Betreff>“ (<n> Dateien).“, Sperren „Diese Version gibt es schon.“, „<Datei> liegt schon am Speicherort …“, „Erst eine Basis-ROM zuordnen.“, „Git ist nicht installiert.“.

| Punkt | Wer | Wie |
|---|---|---|
| (1) `scripts/gate.sh` | Agent (Orchestrator, auf main nach dem Merge) | im Hintergrund, allein; die neuen gated Tests laufen im Schritt der gated Tests je Projekt (Debug) mit, das Skript braucht nichts Neues |
| (1) gated Tests einzeln | Agent | je Projekt im Release-Profil, nacheinander (Last mit `uptime` je Lauf), Befehle unten; die Zeilen `publish …` und `commit …` in die Zahlentabelle |
| (2) Szenario kopflos | Agent | Release-`g4k` und `studio mcp --project <Kopie> --base-rom <nds> [--allow-irreversible]` über JSON-RPC, Ablauf unten |
| (3) Browser | Agent | Test-Umgebung auf 1730/1731, Ablauf unten |
| (3) MCP über die Brücke | Agent | `mcp.sh`, Befehle unten |
| (4) Rom Patcher JS, „Im Finder zeigen“, eigenes Git-Werkzeug | Hand | siehe „Was beim Nutzer bleibt“ |
| Blick | Agent + Blick | Bilder `blick-*.png` unter `target/agent/gate-<datum>/4-5/` |

**Gated Tests (1)**, aus der Wurzel des Worktrees (`K=$G4_ROMS_DIR/_testkopien`, `<k>` = `pt`, `hgde`, `hgus`):

```sh
export PATH="$HOME/.cargo/bin:/opt/homebrew/opt/rustup/bin:$PATH"
G4_TEST_PROJECT="$K/4-5-<k>_DSPRE_contents" \
  cargo test --release -p g4-kernel --all-features --test base publish_after_a_map_edit -- --nocapture --test-threads 1
G4_TEST_PROJECT="$K/4-5-<k>_DSPRE_contents" \
  cargo test --release -p g4-kernel --all-features --test git publish_then_commit_all -- --nocapture --test-threads 1
G4_TEST_PROJECT="$K/4-5-<k>_DSPRE_contents" \
  cargo test --release -p studio --all-features --lib publish_and_commit_over_the_command_path_on_the_vanilla_copy -- --nocapture --test-threads 1
```

Auf stderr stehen die zwei `publish`-Zeilen (`publish <Spielcode> size=… build_ms=… encode_ms=… verify_ms=…` und `publish <Spielcode> write_ms=… record_ms=… ms=… second_ms=… xdelta3=…`), auf stdout die Zeile `commit <Spielcode> files=… add_ms=… commit_ms=… status_ms=…` (`--nocapture` zeigt beide). Die Git-Tests richten Git in einer eigenen Umgebung ein (`GIT_CONFIG_GLOBAL` leer, `GIT_CONFIG_NOSYSTEM=1`, `HOME` im Temp-Ordner); die Einstellungen des Nutzers wirken nicht. Zeitgrenzen binden nur mit `G4_TIMING_STRICT=1`; scheitert ein Zeittest unter Last, einmal allein wiederholen und beide Werte notieren.

**Kopflos (2)**, nach `cargo build --release -p g4-kernel --bin g4k` und `cargo build --release -p studio --bin studio` (`G=target/release/g4k`):

```sh
P=$K/4-5-k<k>_DSPRE_contents; N=$K/4-5-k<k>.nds
$G base assign $P $N; $G git init $P                                          # committed Ausgangsstand files=…
target/release/studio mcp --project $P --base-rom $N                          # ohne --allow-irreversible
#   data_edit Trainer 1 (party[0].level +1), save
#   publish_patch {"version":"0.1.0","notes":"Erste Fassung","readme":true}   → läuft OHNE Frage; drei Dateien unter build/
#   publish_patch {"version":"0.1.0"} nochmal                                 → version_taken
#   publish_patch {"version":"0.1.1","file":"<Projekt>-v0.1.0.xdelta"}        → exists
#   publish_patch {"version":"0.1.1","file":"<Projekt>-v0.1.0.xdelta","overwrite":true} → needs_confirmation
#   git_commit_all {"message":"Veröffentlicht v0.1.0","notes":"Erste Fassung"} → needs_confirmation
target/release/studio mcp --project $P --base-rom $N --allow-irreversible     # overwrite → ersetzt; git_commit_all → committed
xdelta3 -d -s $N $P/build/<Projekt>-v0.1.0.xdelta out.nds; shasum out.nds      # = Ziel-ROM SHA-1 der Begleitdatei = SHA-1 von build/<Projekt>-<Spielcode>.nds
git -C $P status --porcelain; git -C $P log -1 --format=%B; git -C $P remote -v; grep -c '\[remote' $P/.git/config   # leer; Betreff und Body; leer; 0
$G publish $P --version 0.2.0 --dir <Temp-Ordner>                             # Dateien dort, releases.json mit absolutem file
$G git commit-all $P; $G git commit-all $P                                    # committed … (stdout); dann Exit 1 und „Keine Änderung seit dem letzten Commit; nichts zu sichern.“ auf stderr
```

Vor dem ersten Commit zeigt `git status --porcelain` `.g4/releases.json` und den Trainer-Member, nichts unter `build/`. Die Begleitdatei trägt die fünf Zeilen wie 4.1a (`Gen4 Studio: Patch für …`, `Spiel:`, `Basis-ROM SHA-1:`, `Ziel-ROM SHA-1:`, `Studio-Version:`), dazu `Version:` und `Datum:` (RFC 3339, UTC) und am Ende den Absatz `Änderungen:`; `README.txt` hat `<Projekt> v<Version> (<Tag>)`, die Notizen und `Basis-ROM SHA-1:`. Die Verweigerungen von „Stand sichern“ (`no_repo`, `enclosing_repo`, `no_git` mit `G4_GIT=/nicht/da`, `nothing_to_commit`) kommen bei `g4k` als Satz auf stderr mit Exit 1 (ohne `not committed:`; diese Zeile auf stdout gibt es nur für einen Commit, der lief und scheiterte) und schreiben nichts: `.gitignore` und `.gitattributes` bleiben, wie sie waren.

**Browser (3)**, Test-Umgebung auf 1730/1731 (`G4_ROMS_DIR=<absolut> G4_VITE_PORT=1730 G4_BRIDGE_PORT=1731 bash scripts/agent/studio-web.sh`), Seite `http://127.0.0.1:1730/?project=<…>/_testkopien/4-5-web_DSPRE_contents`, 1440 × 900. Ablauf: Technik › Veröffentlichen (leere Liste), Titelleiste „Veröffentlichen“, Assistent mit Vorschlag `0.1.0` und `4-5-web-v0.1.0.xdelta`, Notizen, „Ordner wählen…“ (Pfadfeld im Browser-Modus), Kästchen README, „Weiter“ bis „Prüfen“ (drei Dateien, Hinweis ohne ROM-Inhalt, kein „Zurück“), „Fertig“, „Stand sichern…“ aus dem Ergebnis (Betreff „Veröffentlicht v0.1.0“, Notizen, Prüfliste), „Sichern“, Toast und Statusleiste „main · keine Änderung“, `git -C <Kopie> log -1`/`remote -v`/`status --porcelain`; zweiter Assistent mit derselben Version (Sperre am Feld) und mit dem Dateinamen von Hand (Fehlermeldung, „Ersetzen“); die Basis-ROM „verlieren“ über eine falsche SHA-1 in `.g4/base.json` der Kopie (Original sichern, danach zurückschreiben; Titelleiste und Palette gesperrt mit „Basis-ROM nicht gefunden: …“). Die `.nds` umzubenennen reicht hier nicht: die Brücke sucht im erlaubten Ordner `_testkopien` (und in `G4_BASE_ROM_DIR`) und findet eine andere Kopie mit derselben SHA-1, etwa die einer parallelen Phase; Technik › Änderungen „Gegen letzten Commit“: „Stand sichern…“ ohne Voreinstellung (Betreff „Stand <Datum> <Uhrzeit>“); ohne `git` (`G4_GIT=/nicht/da` für die Brücke) ist die Taste gesperrt mit „Git ist nicht installiert.“; ein eigenes Repository ohne Commit bietet „Stand sichern…“ ebenfalls an; ein Repository aus 4.4 (weitere Kopie `4-5-alt`, nach `g4k git init` Trainer 1 geändert und gespeichert, die Zeile `!/.g4/releases.json` in `.gitignore` von Hand gelöscht, beides committet): veröffentlichen, dann „Stand sichern…“ aus dem Ergebnis: erst „Änderungen werden geprüft …“, dann „2 geänderte Dateien …“ und „Sichern“ frei, die Statusleiste folgt; der Commit nimmt `.gitignore` (wieder mit `!/.g4/releases.json`) und `.g4/releases.json` (`git -C <Kopie> show --stat --format= HEAD`). Bilder nach `target/agent/gate-<datum>/4-5/` (Namen im Gate, Abschnitt „Blick“). Im Browser-Modus fehlen die Phasen des Laufs (die Brücke hat keinen `Channel`) und „Im Finder zeigen“ sagt „Nur in der App.“.

**MCP (3)** über die Brücke (Kopie `4-5-mcp` mit zugeordneter Basis-ROM und Git in der Seite geöffnet):

```sh
scripts/agent/call.sh agent_set_enabled '{"on":true}'
scripts/agent/call.sh agent_set_grant '{"level":"write"}'                    # „Lesen und bearbeiten“
scripts/agent/mcp.sh call publish_patch '{"version":"0.1.0"}'                # läuft OHNE Frage (nichts wird ersetzt)
scripts/agent/mcp.sh call publish_patch '{"version":"0.1.1","file":"4-5-mcp-v0.1.0.xdelta","overwrite":true}' > target/agent/mcp-publish.out 2>&1 &
scripts/agent/call.sh agent_status                                           # question: „Patch ersetzen: …“
# im Browser-Fenster „Zulassen“, oder: scripts/agent/call.sh agent_answer '{"question":<id>,"answer":"allow"}'
scripts/agent/mcp.sh call git_commit_all '{}' > target/agent/mcp-commit.out 2>&1 &
scripts/agent/call.sh agent_status                                           # question: „Stand sichern“
git -C <Kopie> remote -v; git -C <Kopie> status --porcelain                  # beide leer
```

`publish_patch` ohne `overwrite` fragt nicht: es legt nur neue Dateien unter `build/` an, und eine vorhandene Datei lehnt es mit `exists` ab, statt zu fragen; unumkehrbar wird es mit `overwrite: true` und mit einem `dir` außerhalb von `build/` („Bauen außerhalb von build/“). `git_commit_all` fragt immer. Beide sind nicht in `g4_batch` erlaubt, ebenso `git_commit_prepare` (über `g4_call`; seine Prüfung kann den Block der `.gitignore` schreiben). Ein `dir` außerhalb `build/` muss über MCP in einem `--allow-dir`-Ordner liegen und existieren: die Pfadprüfung der Policy kommt vor dem Kern. Eine Datei als `dir` ist dort `bad_dir`, ein Ort **unter** einer Datei oder ein fehlender Ordner antwortet schon in der Pfadprüfung mit `error` „Pfad nicht lesbar: …“ (so gewollt); `g4k publish` antwortet für beides `bad_dir`. Der Eintrag in `.g4/releases.json` bleibt in jedem Fall; `undo` nimmt ihn nicht zurück.

## Phase 3d.5 ([phase-3d-5-gate.md](phase-3d-5-gate.md))

3d.5 bringt Figur, Dialog und Tür in Erzählen: `dialog_of` und `split_message` im Kernel, über MCP und die Brücke (der Dialog einer Figur mit Sprüngen und dem Zähler „geteilt“, das Abspalten einer geteilten Nachricht), die Rezepte „Neue Figur“ (`new_figure`, nummerierte Nachricht im Archiv des Headers), „Tür verbinden“ (`connect_doors`) und „Neuer Trainer hier“ (`new_trainer_here`, unumkehrbar) in `recipe_list`, `recipe_plan`, `recipe_apply` und `g4k recipe`, in der Oberfläche das Feld „Dialog“, den Dialog-Editor, die Zeile „Gegenstück“ an der Tür und die drei Assistenten, dazu die Szenarien `pt-erzaehlen` und `hg-erzaehlen`. Die Punkte und Ergebnisse stehen in der Gate-Datei; hier nur, wer sie prüft und wie.

**Vorab (Befund A des Gates):** vor dem ersten `recipe_plan` einer Figur oder eines Trainers müssen die Scripts übernommen sein, sonst lehnt der Plan ab („Erst Scripts übernehmen (Bereich Welt, Panel Script)“): kopflos in einer Sitzung mit `--allow-irreversible --expert` `g4_call adopt_scripts`, `save`, außen `git add -A && git commit`; in der Oberfläche „Übernehmen“ im Panel Script, danach außen committen. Erst dann die Pläne und Anwendungen unten. **Wann `git status` etwas nennt (Befunde B3 und C2):** Bearbeitungen und Abspalten in der Oberfläche liegen bis zum Speichern im Speicher (`call.sh dirty`); `git status` nennt das Textarchiv (JSON und Member) erst nach ⌘S (kopflos nach `save`). Direkt nach dem Abspalten stehen nur die Quelle, der Script-Member und `compile-state.json` im Status. ⌘S **und** ⌘Z tun nichts, solange der Dialog-Editor oder ein Assistent offen ist: Editor schließen, dann Meta+Z (Fokus in der Seite) und bei einer gespeicherten Änderung danach noch einmal ⌘S, erst dann sind `dirty` und `git status` leer.

**Kopien:** frisch unter `$G4_ROMS_DIR/_testkopien` per `cp -cR` (Ordner) und `cp -c` (`.nds`), Ordner `3d-5-<zweck>_DSPRE_contents` und `.nds` `3d-5-<zweck>.nds` nebeneinander: `pt`, `hgde`, `hgus` für die gated Tests (nur gelesen, die Tests schreiben auf `vanilla_copy()`), `kpt`, `khgde`, `khgus` für das Szenario kopflos (vorher `g4k git init`), `szen-pt` und `szen-hg` für `pt-erzaehlen.yaml` und `hg-erzaehlen.yaml`, `trainer` für `pt-trainer-begegnung.yaml`, `flag` für `pt-flag.yaml` (mit `.nds` für `--set BASIS=../3d-5-flag.nds`), `web` (Platin DE), `webhg` (HeartGold DE) und `mcp` (Platin DE) für Browser und MCP über die Brücke (je `g4k git init`, damit `git status` etwas sagt). Alle `3d-5-*` nach dem Gate löschen. Bilder und Logs unter `target/agent/gate-<datum>/3d-5/`; die Test-Umgebung läuft auf **1750/1751**.

**Wo was ist:** Rezept-Leiste `[role=toolbar][aria-label=Rezepte]` mit fünf freien Tasten `[data-recipe=create_area|create_room|new_figure|connect_doors|new_trainer_here]` (senkrecht mit Namen, waagerecht nur Symbole mit `aria-label`, U3); die Assistenten `[data-wizard=new_figure|connect_doors|new_trainer_here]` mit dem Entwurf `[data-draft][data-draft-count]`, in der Zusammenfassung des Trainers `[data-trainer-barrier]`; im Inspector einer Figur `[data-dialog-section]` (Schild: „Text“) mit `[data-dialog-preview]` und „Dialog bearbeiten…“; der Dialog-Editor `[data-dialog-editor]` mit `[data-dialog-message="<Archiv>:<Index>"]` je Nachricht, der Plakette „Geteilt mit N Figuren“ und „Für diese Figur abspalten“ (die Taste ist **aktiv nur, wenn `dialog_of` für die Nachricht `split_refusal: null` meldet**, sonst `disabled` mit dem Grund als leiser Zeile darunter, `aria-describedby`; die meisten geteilten Nachrichten lassen sich nicht abspalten, Platin 18 von 1325 Paaren, HeartGold DE 89 von 905: erst `dialog_of` über alle Figuren suchen) und dem Kopf „Nachricht 4 · Archiv 554“ mit „· wenn <Bedingung>“ aus bedingten Sprüngen (etwa „· wenn CheckFlag FLAG_X; GoToIf EQUAL“; der lange Kopf bricht um); an der Tür unter „Tür“ die Zeile „Gegenstück“ (`[data-warp-counterpart]`) und `[data-connect-doors]` „Tür verbinden…“; die Ergebnis-Tasten unter „Als Nächstes“ (`[data-next-steps]`): „Dialog bearbeiten“ frei, „Zu einer Geschichte“ gesperrt „Kommt mit 3d.7“, „Hier spielen“ gesperrt „Kommt mit 3d.9“. Die Agentenfrage `[data-agent-question]` heißt bei `recipe_apply` mit `new_trainer_here` „Trainer anlegen“. Bei offenem Dialog-Editor oder Assistenten sperrt die Modal-Regel ⌘S und den ⌘Z des Stores: Editor schließen, dann ⌘S bzw. ⌘Z.

**Zählung über die Test-API:** wie bei 3d.2: `[data-draft-count]` ist gleich `draftCount(<Antwort von recipe_plan>, <gezeigte Matrix>)`; den Körper der Anfrage mit einem umhüllten `window.fetch` mitschneiden (`/__bridge/invoke/recipe_plan`) und mit `scripts/agent/call.sh recipe_plan '<derselbe Körper>'` selbst zählen. Der Entwurf der Figur steht ab Schritt 1 (mit dem Platzhaltertext „...“, bis Text getippt ist).

| Punkt | Wer | Wie |
|---|---|---|
| (1) `scripts/gate.sh` | Agent (Orchestrator, auf main nach dem Merge) | im Hintergrund, allein; in dieser Phase **nicht** im Worktree (unter Last scheitern die Zeitgrenzen), die Schritte, die 3d.5 berührt, laufen einzeln (fmt, clippy, die drei Testläufe ohne `G4_*`, `pnpm test`, `pnpm build`, Tokens, Styles, Depth) |
| (1) gated Tests | Agent | je Projekt im Release-Profil, nacheinander (Last mit `uptime` je Lauf), Befehle unten; die Zeilen `dialog …`, `refs …`, `figure …`, `doors …`, `trainer …` in die Zahlentabelle |
| (1) `g4emu` Unit | Agent | `cd scripts/g4emu && uv run pytest -q` |
| (2) Szenario kopflos | Agent | Release-`studio mcp --project <Kopie> [--allow-irreversible] [--expert]` über JSON-RPC/stdio, Ablauf unten; die Szenarien mit `g4emu` |
| (3) Browser | Agent | Test-Umgebung auf 1750/1751, Ablauf in der Gate-Datei (Scripts vorher übernehmen und committen; Änderungen zählen im `git status` erst nach ⌘S), Chrome headless 1440 × 900 |
| (3) MCP über die Brücke | Agent | `mcp.sh`, Befehle unten |
| (4) `figur.png` lesen, Dialog-Commit in der gepackten App | Hand | siehe Gate-Datei, Abschnitt „Hand“ |
| Blick | Agent + Blick | Bilder `blick-{figur,dialog,dialog-geteilt,tuer,trainer}-{hell,dunkel,kompakt}.png` unter `target/agent/gate-<datum>/3d-5/` |

**Gated Tests (1)**, aus der Wurzel des Worktrees (`K=$G4_ROMS_DIR/_testkopien`, `<k>` = `pt`, `hgde`, `hgus`):

```sh
export PATH="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$PATH"
for t in dialog refs_messages recipes_erzaehlen recipes templates trainer_create; do
  G4_TEST_PROJECT="$K/3d-5-<k>_DSPRE_contents" \
    cargo test --release -p g4-kernel --all-features --test $t -- --nocapture --test-threads 1
done
```

Auf stderr stehen `dialog <Spiel> objects=… local=… messages=… shared=… ms=…` und `dialog-all <Spiel> objects=… unreadable=… shared_pairs=… shared_figures=… ms=…` (jede Figur des Spiels, keine unlesbare; auf Platin mindestens eine geteilte Nachricht), `refs <Spiel> messages=… warm_ms=…`, `figure <Spiel> slot=… message=… apply_ms=… undo_ms=…`, `doors <Spiel> a=… b=… tile=… map=… door_tile=… ms=…`, `trainer <Spiel> id=… tile=… object=… ms=…`. `vanilla_new_figure_message_equals_the_input` prüft die Nachricht im Archiv des Headers gleich der Eingabe und undo/redo byte-gleich; `recipes`, `templates` und `trainer_create` bleiben unverändert grün (`recipes.rs` mit den kleinen Änderungen für sieben Rezepte); auf beiden HeartGold-Kopien meldet `--test recipes` für `vanilla_room_recipe_matches_the_wizard_scenario` „skipped: the scenario is Platinum DE's“ (das Platin-Szenario `pt-raum-assistent`), `--test recipes_erzaehlen` überspringt nichts.

**Kopflos (2)**, nach `cargo build --release -p g4-kernel --bin g4k` und `cargo build --release -p studio --bin studio` (`G=target/release/g4k`):

```sh
P=$K/3d-5-k<k>_DSPRE_contents
$G git init $P                                               # Ausgangsstand, damit git status etwas sagt
target/release/studio mcp --project $P --allow-irreversible --expert
#   zuerst: g4_call adopt_scripts; save; danach außen: git add -A && git commit   (ohne das lehnt recipe_plan new_figure ab)
target/release/studio mcp --project $P                       # ohne Erlaubnis:
#   recipe_list → sieben Einträge
#   recipe_plan {"recipe":"new_figure","input":{header,x,z,movement_type,dir,text}} → events 1 (look), messages 1; git status leer
#   recipe_apply {"recipe":"new_trainer_here","input":{…}} → needs_confirmation „Trainer anlegen“
target/release/studio mcp --project $P --allow-irreversible --expert
#   recipe_apply new_figure → history + 1, report.message; get_text_archive des Header-Archivs: Text gleich der Eingabe
#   undo → git status leer; redo → wie nach apply (auch mit save zwischen apply und undo: save → undo → save → leer)
#   recipe_apply connect_doors {a, b, back, door_tiles} → ein Schritt; get_events beider Dateien zeigen aufeinander
#   recipe_apply new_trainer_here → created nennt den Trainer, history endet mit „Trainer N angelegt“ und dem Rezept-Label
#   dialog_of {header, event_file, kind:"object", index} der neuen Figur → eine Nachricht, shared_with 0
#   dialog_of einer Vanilla-Figur mit shared_with ≥ 1 und split_refusal null (die meisten sind nicht abspaltbar: Platin Header 75 Objekt 0 Nachricht 95:4,
#   HeartGold Header 22 Objekt 12 Nachricht 351:7) → split_message {header, event_file, index, archive, message} → new_index;
#   git status nennt Quelle, Member und compile-state.json, das eine Textarchiv (JSON und Member) erst nach save; undo → git status leer (auch nach save)
```

**Szenarien (2)**, aus `scripts/g4emu` mit `G4_ROMS_DIR` absolut (`G4_STUDIO_BIN` und `G4K_BIN` auf die Release-Programme, `G4_BRIDGE_BIN` für `pt-trainer-begegnung`):

```sh
uv run g4emu scenario ../agent/scenarios/pt-erzaehlen.yaml --project $K/3d-5-szen-pt_DSPRE_contents --out $O/pt-erzaehlen \
  --set NEUER_HEADER=593 --set S_T1=25 --set L_T1=12 --set TEXT="Hallo, Reisender!"
uv run g4emu scenario ../agent/scenarios/hg-erzaehlen.yaml --project $K/3d-5-szen-hg_DSPRE_contents --out $O/hg-erzaehlen \
  --set NEUER_HEADER=540 --set S_T1=25 --set L_T1=12 --set TEXT="Hallo, Reisender!"
uv run g4emu scenario ../agent/scenarios/pt-trainer-begegnung.yaml --project $K/3d-5-trainer_DSPRE_contents --out $O/pt-trainer-begegnung \
  --set NEUER_HEADER=593 --set S_WILD=150 --set L_WILD=5 --set S_T1=25 --set L_T1=12 --set S_T2=133 --set L_T2=14 --set S_GIFT=445
uv run g4emu scenario ../agent/scenarios/pt-flag.yaml --project $K/3d-5-flag_DSPRE_contents --set BASIS=../3d-5-flag.nds --out $O/pt-flag \
  --set NEUER_HEADER=593 --set X_FIGUR=54 --set Z_FIGUR=17
```

`"ok": true` in `results.json`; `pt-erzaehlen` und `hg-erzaehlen` haben zweimal `expect-map` (Tür hin und zurück) und `expect-battle`, `figur.png` zeigt „Hallo, Reisender!“ in der Box (Agent + Blick). Die Szenarien haben keinen `check:`-Block (`g4k check-data` kennt keinen Trainer; der gated Test prüft das Team).

**Browser (3)**, Test-Umgebung auf 1750/1751 (`G4_ROMS_DIR=<absolut> G4_VITE_PORT=1750 G4_BRIDGE_PORT=1751 bash scripts/agent/studio-web.sh`), Seite `http://127.0.0.1:1750/?project=<…>/_testkopien/3d-5-web_DSPRE_contents`, 1440 × 900, Chrome headless mit `--use-angle=swiftshader --enable-unsafe-swiftshader`; Hell und Dunkel gelten nur in der CDP-Verbindung, die sie setzt. Ablauf: Erzählen, Rezept-Leiste mit fünf freien Tasten; „Neue Figur“ bis zur Zusammenfassung (Zählung wie oben), Esc, `git status` leer; Aufstellen, „Dialog bearbeiten“; ein Dialog einer Vanilla-Figur geändert (Blur) → `dirty` nennt das Archiv, `git status` ist noch leer; Editor schließen, ⌘S → nur das eine Textarchiv im `git status` (` M unpacked/textArchives/NNNN` und `?? expanded/textArchives/NNNN.json`, nichts unter `scripts`); ⌘Z (Meta+Z, Fokus in der Seite) → `dirty` zeigt das Archiv bis zum nächsten ⌘S, danach `call.sh dirty` und `git status` leer (ohne Speichern dazwischen sofort); eine geteilte, **abspaltbare** Nachricht (Platin Header 75, HeartGold Header 22) → Plakette, „Für diese Figur abspalten“, `git status` nennt Quelle, Member und `compile-state.json`, nach ⌘S das Archiv, ⌘Z und ⌘S → leer; eine nicht abspaltbare (Platin Jubelstadt, HeartGold Neuborkia Objekt 3) → Taste gesperrt mit dem Grund; Tür → „Gegenstück“, „Tür verbinden…“; „Neuer Trainer hier“ (`git status` nennt keine Scriptdatei: der Trainer nutzt das gemeinsame Kampf-Script, dafür Eventdatei, zwei Textarchive, `trainerTextTable`, `trainerTextOffset` und die Trainerdateien); HeartGold DE (`3d-5-webhg`) ebenso.

**MCP (3)** über die Brücke (Kopie `3d-5-mcp` in der Seite geöffnet):

```sh
scripts/agent/call.sh agent_set_enabled '{"on":true}'
scripts/agent/call.sh agent_set_grant '{"level":"write"}'
scripts/agent/mcp.sh call recipe_plan '{"recipe":"new_trainer_here","input":{…}}'          # ohne Rückfrage
scripts/agent/mcp.sh call recipe_apply '{"recipe":"new_trainer_here","input":{…}}' > target/agent/mcp-trainer.out 2>&1 &   # wartet auf die Frage
scripts/agent/call.sh agent_status            # question: {kind: confirm, what: "Trainer anlegen"}
# im Browser-Fenster „Ablehnen“ → needs_confirmation „… in Gen4 Studio abgelehnt“, oder „Zulassen“
```

## Phase 3d.9 ([phase-3d-9-gate.md](phase-3d-9-gate.md))

3d.9 bringt den Spielmodus live: „Hier spielen“ startet das Spiel in melonDS 1.1 mit dem GDB-Stub (nur macOS), der Kernel (`crates/g4-kernel/src/live/`) liest alle 250 ms Zeiger, Variablen und Flags des Spielstands aus dem Arbeitsspeicher und schreibt sie mit Rücklesen (`g4k live status|set|wait|resume`), das Scharnier wird bernsteinfarben (`data-hinge-mode="play"`), die Fläche „Flags und Variablen“ zeigt in der Spalte „Live“ die Werte und im Detail den Abschnitt „Live“, der Inspector setzt Haltepunkte an Figuren (Sichtbarkeits-Flag) und Auslösern (Variable), MCP hat `live_status` und `live_set`. `g4emu repl` lässt das Spiel laufen und hat einen eigenen GDB-Stub, so prüft das Gate ohne melonDS; nur Punkt (4) braucht das echte. **Port 3390 für den g4emu-Stub, nie 3333 (und 3334) anfassen, solange ein melonDS des Nutzers laufen könnte** (`pgrep -x melonDS` leer; nur Punkt (4) benutzt 3333). Der Stub, `g4k live` und die App bedienen je Port genau einen Client.

**Kopien:** frisch unter `$G4_ROMS_DIR/_testkopien` per `cp -cR` (Ordner) und `cp -c` (`.nds`), `3d-9-<zweck>` (ein ds-rom-Projektordner) und `3d-9-<zweck>.nds` nebeneinander: `pt`, `hgde`, `hgus` für den gated Test und das Szenario kopflos (nur gelesen, `repl` spielt die `.nds`), `web` (Platin DE, als `3d-9-web_DSPRE_contents`) und `webhg` (HeartGold DE) für Browser und MCP über die Brücke (je `g4k git init`, damit `git status` etwas sagt). Spielstände unverändert aus `_saves` (`g4emu` kopiert sie vor jedem Lauf).

**Wo was ist:** Scharnier im Spielmodus `[data-hinge-mode="play"]` mit `[data-live-state]` (`starting`, `connecting`, `waiting_for_save`, `live`, `halted`, `lost`; bei `off` und `unavailable` bleibt die Leiste dunkel), `[data-live-resume]` „Weiter“ (nur im Halt), `[data-live-reload]` „Neu laden“, `[data-live-stop]` „Stoppen“. Technik › Flags und Variablen: Kopf `[data-flag-live-head]` (im Spielmodus `="on"`), Zellen `[data-flag-live]` (Flag: Taste mit `="1"` oder `="0"`, Klick schaltet um; Variable: dezimal; sonst „–“), Detail `[data-flag-section="Live"]` (Schalter „Im Spiel gesetzt“ bzw. Feld „Wert im Spiel“ und „Setzen“), Warnung `[data-flag-live-warning]` für eine nicht gewählte Zeile, deren Wert das Spiel nicht behielt. Inspector einer Figur oder eines Auslösers: `[data-live-breakpoint]` „Haltepunkt“ (nur im Spielmodus; bei Tür und ohne Flag bzw. Variable gesperrt mit Grund). Technik › Emulator: `[data-live-row]` „Spielmodus live“. „Spielmodus“ auf den Flächen heißt: nicht `off`, nicht `unavailable` (wie am Scharnier). Brücke: `live_status`, `live_connect {port, host?}` (verbindet mit einem vorhandenen Stub, auch dem von `g4emu repl`), `live_set`, `live_stop`, `live_resume`, `live_breakpoints`, `live_breakpoint_set`, `live_breakpoint_clear`; `live_play` lehnt die Brücke ab wie `run_rom`. MCP: nur `live_status` und `live_set`.

| Punkt | Wer | Wie |
|---|---|---|
| (1) Automatik | Agent | fmt, clippy, `cargo test -p g4-kernel --all-features`, `cargo test -p studio --all-features --lib`, `cargo test -p studio --test mcp_stdio` (alle drei ohne `G4_*`), `pnpm test`, `pnpm build`, `check-tokens.sh`, `check-styles.sh`, `check-depth.sh`, `g4emu` Einheit (`uv run pytest -m "not integration" -q`) einzeln; das ganze `gate.sh` läuft auf main nach dem Merge |
| (1) gated `test_live.py` | Agent | Platin DE, HeartGold DE, HeartGold US mit `g4emu repl` + `g4k live` auf Port 3390; `took_ms` und `game_check` je Spiel in die Tabelle der Gate-Datei |
| (2) kopflos | Agent | `g4emu repl` auf `3d-9-pt` (Port 3390) und Release-`studio mcp`: `live_status` → `off`, `live_set` → `not_live`; die Verbindung selbst ist App-only, darum `live_set` gegen den Stub über die Brücke (nächste Zeile) |
| (3) Browser | Agent + Blick | Test-Umgebung 1790/1791, Projekt `3d-9-web_DSPRE_contents`; `g4emu repl` mit `CPUD-zimmer.sav` auf 3390; `call.sh live_connect`; Scharnier bernsteinfarben (Bild), Spalte „Live“ (Bild), Flag `0x173` per Klick auf 0 → repl `expect-flag 0x173 0`, Treppe hinunter und über den Warp in 414 wieder hinauf (Ablauf unten, mit `wait 150` nach jedem Kartenwechsel); die Figur erscheint (R23): Flag `0x1F1` (Figur 1 der Eventdatei 393 auf der Fußmatte von 414, (6, 10)) per Klick auf 0, 414 neu betreten, von (6, 8) `walk-until-blocked down 2` → 1 Schritt, `expect-at 6 9` (`shot figur`); Variable `0x40A5` auf 0 → `walk-to 5 5`, `walk-until-blocked left 4` (1 Schritt), `expect-at 4 5`; Haltepunkt am Auslöser mit `var 0x40A5` → repl `set-var 0x40A5 1` → Scharnier „Gehalten: …: 0 → 1“ (Bild), `expect-halted`; „Weiter“ → `expect-running`; „Stoppen“ → Scharnier dunkel, `live_status.state == "off"` |
| (3) MCP über die Brücke | Agent | `mcp.sh call live_status`, `live_set` (→ `read_back`), `live_set` außerhalb des Spielstands → `outside_save` |
| (3) HeartGold DE | Agent | wie (3) mit `IPKD-zimmer.sav`, Flag `0x2A9` (Figuren im Zimmer, Header 64), nur Flag und Scharnier |
| (4) echtes melonDS | Agent + Hand | Agent (nur bei leerem `pgrep -x melonDS`): `melonDS.toml` sichern, eine Kopie mit `g4_kernel::live::melonds::enable_stub` (Wegwerfprogramm im Scratchpad) umschreiben, einsetzen, melonDS 1.1 mit `3d-9-pt` starten, `g4k live status --port 3333 --game CPUD` (Antwort = melonDS liest die umgeschriebene Datei, `toml` 0.8 genügt; sonst Befund, `toml_edit`), `verified`, `has_save`; Original byte-gleich zurück (`cmp`), melonDS beenden. Hand: im melonDS-Fenster bis ins Zimmer fortsetzen; „Hier spielen“ in der gepackten App, Scharnier bernsteinfarben, Figur erscheint nach Flag-Klick beim nächsten Betreten des Zimmers |
| Blick | Agent + Blick | Bilder `blick-*.png` und `shot-*.png` unter `target/agent/gate-<datum>/3d-9/` |

**Gated Test (1)**, aus der Wurzel des Worktrees (`K=$G4_ROMS_DIR/_testkopien`, Kern unter `tools/g4emu/core/`, `g4k` gebaut mit `cargo build -p g4-kernel --bin g4k`):

```sh
export PATH="$HOME/.cargo/bin:/opt/homebrew/opt/rustup/bin:$PATH"
export G4_ROMS_DIR=<absolut>; K=$G4_ROMS_DIR/_testkopien
G4K_BIN=$PWD/target/debug/g4k \
  G4_LIVE_ROM_CPUD=$K/3d-9-pt.nds G4_LIVE_ROM_IPKD=$K/3d-9-hgde.nds G4_LIVE_ROM_IPKE=$K/3d-9-hgus.nds \
  bash -c 'cd scripts/g4emu && uv run pytest -m integration -k live -q -s'
```

Der Test (`scripts/g4emu/tests/test_live.py`, Port 3390, die Spielstände werden in einen Temp-Ordner kopiert) überspringt ein Spiel, dessen Variable `G4_LIVE_ROM_<CODE>` fehlt, und druckt je Spiel `game_check`, `took_ms` von `status` und die Wanduhr von `set` (mit den eingebauten 100 ms Warten); `took_ms ≤ 100` erzwingt nur `G4_TIMING_STRICT=1`. Platin DE: `0x173` := 0, `0x40A5` := 0, Auslöser über `walk-to 5 5`, `walk-until-blocked left 4` (genau 1 Schritt), `expect-at 4 5` (auf dem echten Spiel feuert der Auslöser beim Ankommen auf (4, 5); die Probe `walk-to 4 5` + `walk-until-blocked down 3` blockiert sofort, die Gegenprobe mit Variable 1 geht bis (1, 5)), `g4k live wait --var 0x40A5 --timeout 10` mit `set-var 0x40A5 1` im `repl`: `halted` true, `before` 0, `after` 1, `expect-halted`, Zeilenumbruch auf stdin von `wait`, `expect-running`; HeartGold DE und US: Flag `0x2A9` setzen und `expect-flag`.

**`g4emu repl` für Hand und Browser (2) (3)**, aus `scripts/g4emu`, die Schritte kommen von einer Datei (`tail -f`), die Antworten stehen als JSON-Zeilen in `repl.out`:

```sh
K=$G4_ROMS_DIR/_testkopien; OUT=<Worktree>/target/agent/gate-<datum>/3d-9; mkdir -p $OUT; : > $OUT/repl.in
( tail -n +1 -f $OUT/repl.in | uv run g4emu repl --rom $K/3d-9-pt.nds --save $K/_saves/CPUD-zimmer.sav \
    --out $OUT/repl --gdb-port 3390 > $OUT/repl.out 2> $OUT/repl.err ) &
# erste Zeile von repl.out: {"ready": true, "gdb_port": 3390, "game": "CPUD"}
echo "continue" >> $OUT/repl.in; echo "expect-map 415" >> $OUT/repl.in
echo "expect-flag 0x173 0" >> $OUT/repl.in           # nach dem Klick in der Oberfläche; jede Antwort eine Zeile mit ok true/false
echo "set-var 0x40A5 1" >> $OUT/repl.in; echo "expect-halted" >> $OUT/repl.in
echo "quit" >> $OUT/repl.in                           # danach das tail beenden (kill)
```

Der Stub hält beim Verbinden an, bis `c` kommt (wie melonDS); die App und `g4k live` senden es. Ein zweiter Client bekommt keine Antwort.

**Kopflos (2)**, nach `cargo build --release -p g4-kernel --bin g4k` und `cargo build --release -p studio --bin studio`:

```sh
K=$G4_ROMS_DIR/_testkopien; P=$K/3d-9-pt
target/release/studio mcp --project $P                        # live_status → state off; live_set {kind flag, number 371, value 0} → not_live; tools/list nennt beide
target/release/g4k live status --port 3390 --game CPUD        # mit laufendem repl: state, game_check verified, took_ms
target/release/g4k live set --port 3390 --game CPUD --flag 0x173 --value 0
git -C $P status --porcelain                                   # nichts (ein Live-Wert schreibt kein Projekt)
```

**Browser (3)**, Test-Umgebung auf 1790/1791 (`G4_ROMS_DIR=<absolut> G4_VITE_PORT=1790 G4_BRIDGE_PORT=1791 bash scripts/agent/studio-web.sh`), Seite `http://127.0.0.1:1790/?project=<…>/_testkopien/3d-9-web_DSPRE_contents`, 1440 × 900, Chrome headless wie bei 3d.2 (Hell und Dunkel in **einer** CDP-Verbindung mit dem Bild). Zuerst `g4emu repl` auf 3390 (oben), dann:

```sh
scripts/agent/call.sh live_connect '{"port":3390}'            # die App verbindet mit dem Stub; Scharnier wird bernsteinfarben
scripts/agent/call.sh live_status                             # state live, game_check verified, snapshot
scripts/agent/call.sh live_set '{"kind":"flag","number":371,"value":0}'   # 0x173; read_back null, danach live_status.read_back true
scripts/agent/call.sh live_stop                               # oder „Stoppen“ im Fenster: Scharnier dunkel, state off
```

Ablauf in der Oberfläche: Scharnier (Bild `blick-scharnier-live-{hell,dunkel,kompakt}.png`), Technik › Flags und Variablen (Spalte „Live“, Bild `blick-flags-live-*.png`), Flag `0x173` per Klick auf die Zelle auf 0 (Detail zeigt „Im Spiel gesetzt“ aus), repl `expect-flag 0x173 0`; im repl Treppe hinunter (`walk-to 8 4`, `walk right 1`, `expect-map 414`, `wait 150`, `mash b 600` für die Mutter) und über den Warp der Treppe in 414 hinauf (die Ankunft x 10, y 3 liegt auf ihm, ein Schritt nach Westen löst ihn aus, wie in `pt-neuer-ort.yaml`: `walk left 1`, `expect-map 415`, `wait 150`; Kachel und Richtung aus `scripts/agent/call.sh get_events '{"eventFileId":393}'` (Eventdatei von Header 414, zwei Warps) lesen und prüfen), die Figur erscheint (R23; Figur 0 der Eventdatei 394 auf (10, 4) gehört zur Szene des Rivalen und ist nicht zu erreichen): in 414 Figur 1 der Eventdatei 393 (Kachel (6, 10), Flag `0x1F1`, Startwert im Spielstand 1 notieren; mit 1 ist die Fußmatte leer, Bild auf (8, 8)), in der Spalte „Live“ `0x1F1` per Klick auf 0, repl `expect-flag 0x1F1 0`, 414 neu betreten (`walk-to 10 3`, `walk left 1`, `expect-map 415`, `wait 150`, `walk right 1`, `expect-map 414`, `wait 150`, `mash b 300`), `walk down 2`, `walk-to 6 8` (blockiert bei (8, 8) an der Mutter, dort Bild), `walk down 1`, `walk-to 6 9`, `walk up 1`, `walk-until-blocked down 2` → 1 Schritt, blockiert vor (6, 10), `expect-at 6 9`, `shot figur`; Variable `0x40A5` im Detail auf 0 setzen, `walk-to 5 5`, `walk-until-blocked left 4`, `expect-at 4 5`; Welt › Eventdatei 394 › Auslöser mit `var 0x40A5` › Kästchen „Haltepunkt“, repl `set-var 0x40A5 1` → Scharnier „Gehalten: Auslöser … · Eventdatei 394: 0 → 1“ (Bild `blick-halt-*.png`), `expect-halted`, „Weiter“, `expect-running`; „Stoppen“. HeartGold DE ebenso auf `3d-9-webhg` mit `IPKD-zimmer.sav` und Flag `0x2A9` (nur Flag und Scharnier).

**MCP (3)** über die Brücke (Kopie in der Seite geöffnet, `live_connect` wie oben gemacht):

```sh
scripts/agent/call.sh agent_set_enabled '{"on":true}'
scripts/agent/call.sh agent_set_grant '{"level":"write"}'                   # „Lesen und bearbeiten“
scripts/agent/mcp.sh call live_status '{}'                                   # state live, snapshot mit flags und vars
scripts/agent/mcp.sh call live_set '{"kind":"flag","number":371,"value":0}'  # read_back null, danach live_status true; repl expect-flag 0x173 0
scripts/agent/mcp.sh call live_set '{"kind":"flag","number":4000,"value":1}' # outside_save
scripts/agent/call.sh history '{}'                                           # unverändert (kein Schritt)
```

Keine Rückfrage (`agent_status.question` null): `live_set` ist schreibend, aber ohne Verlauf; `live_connect` und `live_play` sind in MCP „Nur in der App“.

**Echtes melonDS (4)**, nur wenn `pgrep -x melonDS` leer ist (kein melonDS des Nutzers). Es prüft, ob melonDS die vom Kernel (`toml` 0.8) umgeschriebene Datei liest; die Funktion `enable_stub` ist in der App nur über `live_play` erreichbar (in der Brücke abgelehnt), darum ein Wegwerfprogramm im Scratchpad (nie ein Test im Repo, nie eingecheckt):

```sh
K=$G4_ROMS_DIR/_testkopien; OUT=<Worktree>/target/agent/gate-<datum>/3d-9; S=<Scratchpad>/enable-stub; mkdir -p $S
C=~/Library/Preferences/melonDS/melonDS.toml
pgrep -x melonDS && echo "läuft: abbrechen"                                       # muss leer sein
cp -p $C $OUT/melonDS.toml.original; shasum $C; cp $C $S/melonDS.toml            # Original sichern, Kopie zum Umschreiben
# $S (vorher mkdir -p): cargo init --bin; Cargo.toml: g4-kernel = { path = "<Worktree>/crates/g4-kernel" }
# main.rs: println!("{:?}", g4_kernel::live::melonds::enable_stub(std::path::Path::new("<Scratchpad>/enable-stub/melonDS.toml")));
(cd $S && cargo run -q)                                                          # druckt die Einstellungen: enabled true, break_on_startup false, jit false
cp $S/melonDS.toml $C                                                            # die umgeschriebene Kopie einsetzen
cp $K/3d-9-pt.nds $OUT/pt.nds; cp $K/_saves/CPUD-zimmer.sav $OUT/pt.sav          # Annahme: melonDS findet den Spielstand als <ROM-Name>.sav
/Applications/melonDS.app/Contents/MacOS/melonDS $OUT/pt.nds >/dev/null 2>&1 &
# Hand: im melonDS-Fenster das Spiel fortsetzen, bis es im Zimmer steht (kein Eingabewerkzeug für das Fenster)
target/release/g4k live status --port 3333 --game CPUD                           # Antwort: melonDS liest die Datei, toml 0.8 genügt; game_check verified, has_save true, took_ms
# keine Antwort („Kein GDB-Stub auf Port 3333“): Befund, toml_edit als Abhängigkeit (cargo deny, Begründung im Commit)
pkill -x melonDS; while pgrep -x melonDS >/dev/null; do sleep 1; done            # melonDS schreibt die Datei beim Beenden zurück
cp -p $OUT/melonDS.toml.original $C; cmp $OUT/melonDS.toml.original $C && echo "byte-gleich"
```

Hand: im melonDS-Fenster das Spiel bis ins Zimmer fortsetzen (oben) und „Hier spielen“ in der gepackten App (`pnpm bundle`), Scharnier bernsteinfarben, nach dem Klick auf ein Flag in der Spalte „Live“ erscheint die Figur beim nächsten Betreten des Zimmers. Antwortet der Stub nach dem Umschreiben nicht, wird `toml_edit` Abhängigkeit (siehe Gate-Datei, „Offen“).
