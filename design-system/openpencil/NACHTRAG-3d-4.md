# Nachtrag 3d.4 für die OpenPencil-Datei

**Die Datei wurde nicht angefasst.** Phase 3d.4 „Löschen“ (02.11.2026) hat Code, README und Audit nachgezogen; `design-system/Editor_DesignSystem.fig` kennt die neuen Zustände noch nicht, die Frames folgen. OpenPencil war beim Schreiben nicht erreichbar (die MCP-Verbindung scheiterte mit einem 502). Dieser Nachtrag ist die Anleitung für die Sitzung, die die Datei danach anfasst; die Vorlage `beispiele/loeschen.body.js` liegt bereit und lässt sich ohne OpenPencil bauen.

Was die App zeigt, beschreibt `design-system/README.md`, Abschnitt „Nachtrag 3d.4“. Texte und Zustände stammen von dort und aus dem Code: `apps/studio/src/delete/{DeleteDialog,DeleteContext,ShowDeleted}.tsx`, `ui/primitives/EntityStateBadge.tsx`, `workbench/inspector/{InspectorHead,HeaderEditor,MatrixCellEditor,WarpField}.tsx`, `original/HeadActions.tsx`, `data/trainer/{TrainerList,TrainerView}.tsx`, `data/encounters/{EncounterList,EncounterView}.tsx`, `workbench/{WorldTree,HeaderList}.tsx`, `technik/{ChangesSurface,EntityDiff,diffWords}`. Projekt „Sinnoh-Remix“, Platin DE, wie die übrigen App-Screens. Beispielwerte, keine Spieldaten, sind:
- die Verweise des Trainers 0001 (Figur „Route 202, Kachel 166, 813“ in Eventdatei 0329, Script 0329),
- der Raum „Haus 1“ (Zweiblattdorf, `T01R0101`) als Grabstein,
- der Trainer 0930 und die Datei der freigegebenen Nummer (`data/trainers/0930.json`),
- die Zeilen von Technik › Änderungen.

Vergleichsquelle sind die Screenshots der Test-Umgebung unter `target/agent/3d-4/` (1440 × 900, nicht im Repo). Sie entstehen im Lauf des Gates dieser Phase; die Liste mit Namen steht im Gate-Bericht (`docs/phase-3d-4-gate.md`). Vorgesehen sind zwölf Bilder: `blick-dialog-*` (Dialog im Normalfall), `blick-verweise-*` (Dialog mit Verweisen), `blick-liste-*` (Liste mit „Gelöschte zeigen“, Kopf mit Plakette) und `blick-aenderungen-*` (Technik › Änderungen), je mit `hell`, `dunkel` und `kompakt`.

Neue Tokens und neue Symbole gibt es nicht. Die Plakette ist die vorhandene `Badge` in den Arten Warnung, Info und Fehler (Symbole `warning`, `info`, `error` = Phosphor `ph:warning`, `ph:info`, `ph:x-circle`), der Dialog der vorhandene `Dialog` mit Meldung, Listenzeile, Kästchen und der Plakette „Nicht rückgängig“.

## Was neu zu zeichnen ist

Zwei Frames je 1440 × 900 und ein Brett, alle aus `beispiele/loeschen.body.js` (`MODE` wählt). Diese Teile gibt es schon:
- Rahmen, Atlas, Statusleiste, das Dock der Werkbank, die Liste „Welt“ mit Baum „Orte“, der Inspector-Kopf (`InspectorHead` aus `jsx-lib-app.js`, die Plaketten kommen als `badges`),
- die Trainer-Ansicht aus `textprojekt.body.js` (Liste „Trainer“, Kopf, Team),
- das Menü (`MenuX`), das Kontextmenü (`ContextMenu`), der Dialog mit Abdunklung (`MitDialog`),
- die Bausteine Plakette (`Badge`), Meldung, Listenzeile, Kästchen und Schau-Platzhalter (`Welt3D`).

Neu sind der Dialog „… löschen“ mit seinen Zuständen, die Plakette Entitätszustand in Kopf und Zeilen und das Kästchen „Gelöschte zeigen“ im Listenkopf.

1. **Frame „App: Trainer, Löschen mit Verweisen“** (`MODE = 'dialog'`).
   - **Fenster:** Pokémon › Trainer wie im Frame „ID umbenennen“ von 4.3b: Liste „Trainer“ mit dem Kästchen „Gelöschte zeigen“ (aus) über den Gruppen, „Teenager Gunther“ (0001) gewählt, Kopf mit Name, Nummer und Menü „Aktionen“, Team, „Steht in der Welt“.
   - **Dialog** „Trainer 0001 löschen“ (480 breit, ohne Beschreibung): Fehlermeldung „2 Verweise: zuerst umhängen.“, darunter die zwei Verweise als Zeilen mit Name und Quelle in Mono („Figur · Route 202, Kachel 166, 813“ / „Eventdatei 0329“, „Script · Teenager Gunther“ / „Scriptdatei 0329“), „Abbrechen“ und „Löschen“ (primär, gesperrt).
2. **Frame „App: Welt, Inspector mit Plakette gelöscht“** (`MODE = 'kopf'`).
   - **Liste „Welt“:** im Baum „Orte“ Zweiblattdorf geöffnet, die Zeile „Haus 1“ (gewählt, gezeigt) mit der Plakette „gelöscht“ am Ende; „In dieser Szene“ mit „Haus 1“ als Wurzel. Die Zeile steht nur bei eingeschaltetem „Gelöschte zeigen“ da; das Kästchen im Kopf der Orte zeichnet `WeltListe` nicht (siehe „Noch zu zeichnen“).
   - **Inspector** (340 breit): Kopf „Raum“ / „Haus 1“ / „Zweiblattdorf · T01R0101“ mit den Plaketten „gelöscht“ und „Ungespeichert“ (in dieser Reihenfolge) und dem offenen Menü „Aktionen“: „Raum hier anbinden…“, „Ort umbenennen…“, „Löschen…“ (unter dem Zeiger), „Für diesen Raum abspalten…“, Trenner, „Auf Original zurücksetzen…“. Das Frame zeigt zwei Augenblicke zugleich (Menü und Grabstein), damit eines reicht.
   - **Scharnier und Statusleiste:** wie die übrigen Welt-Frames (Bauen, Brotkrume „Zweiblattdorf › Haus 1“).
3. **Brett „Komponenten (App): Entitätszustand und Dialog Löschen“** (`MODE = 'brett'`), für die Seite „Komponenten“. Es zeigt:
   - die drei Plaketten „geändert“, „neu“, „gelöscht“ und einen Kopf mit „gelöscht“, „Ungespeichert“ und „Nicht rückgängig“;
   - „Gelöschte zeigen“ aus und an, mit der gelöschten Zeile („Teenager Anton 0930“, Plakette „gelöscht“) und ihrem Kontextmenü („Wiederherstellen“ vor „Auf Original zurücksetzen…“);
   - die acht Zustände des Dialogs (Laden, Abgelehnt, Verweise, Neu nicht am Ende, Neu am Ende mit „Nummer freigeben“, Original mit Warnung, Läuft, Gescheitert);
   - „Gelöscht“ in Technik › Änderungen: zwei Zeilen („Wiederherstellen“ für die neue, „Auf Original zurücksetzen…“ für das Original) und die Diff-Plaketten „Gelöscht“ und „Wiederhergestellt“.

## Lage und Einspielen

- **Lage:**
  - Die Frames 1 und 2 kommen auf die Seite „App: Werkbank“ in die nächsten freien Spalten, Frame 2 rechts neben Frame 1. Es gilt x = Spalte × 1520, y = Reihe × 1000.
  - Das Brett kommt auf die Seite „Komponenten“ unter das letzte Brett „Komponenten (App)“.
  - Vorher mit `get_page_tree` (depth 1) die belegten Positionen ablesen und nur eigene Frames anfassen; andere Phasen (3d.3 B, 3d.5, 3d.6) legen in derselben Zeit ebenfalls Frames an. Die Lage steht danach hier und im README, Abschnitt „Nachtrag 3d.4“.
- **Bauen:**
  1. Aus `openpencil/beispiele/` `bash ../build-screen.sh loeschen` aufrufen (für Dunkel mit `--dunkel`). Das Skript schreibt nur die `.jsx` neben die Vorlage und braucht OpenPencil nicht.
  2. In der erzeugten `loeschen.jsx` `MODE` je Frame umstellen (`'dialog'`, `'kopf'`, `'brett'`).
  3. `node ../../scripts/op.mjs render '{…}' --jsx loeschen.jsx` aufrufen (`op.mjs render` mit `x`, `y` und der Seite, siehe die früheren Nachträge).
  4. Per `export_image` vergleichen (`node ../../scripts/op.mjs export_image '{…}' --png bild.png`), mit den Screenshots aus `target/agent/3d-4/`: Frame 1 mit `blick-verweise-*`, Frame 2 mit `blick-liste-*` (Kopf und Zeile mit Plakette), das Brett mit `blick-dialog-*` (der Normalfall des Dialogs) und `blick-aenderungen-*`. Dunkel und Kompakt mit den Bildern `-dunkel` und `-kompakt` derselben Namen.

     Die Screenshots zeigen echte Werte des Testprojekts, die Frames Beispielwerte. Verglichen werden Aufbau, Maße und Texte. Atkinson Hyperlegible Mono ist in OpenPencil ersetzt (Inter), darum sind Spannen in den Bildern breiter als ihr Text.
  5. Korrigieren: den Body ändern, neu bauen, `render` mit `"replace_id":"<ID>"` statt `x`/`y` (höchstens drei Läufe je Frame). Die erzeugten `.jsx` und `.png` nicht einchecken, sondern löschen.
- **Danach:**
  1. Speichern (der Hauptagent, `save_file`) und das Dokument schließen.
  2. Aus der Repo-Wurzel `node design-system/scripts/fix_fig_text.mjs design-system/Editor_DesignSystem.fig --rgb555` aufrufen.
  3. Im README den Abschnitt „Nachtrag 3d.4“ um einen Satz ergänzen, dass die Frames seit … in der Datei stehen (mit Seite und Lage). Den Absatz „Die Frames von 3d.4 stehen noch nicht in `Editor_DesignSystem.fig`“ und den Punkt „Die `.fig` kennt die Frames noch nicht“ dort, „Die Datei wurde nicht angefasst“ hier und die Zeile „Löschen“ in `Audit-Design-System.md` ersetzen.
  4. Im Brett „Stand der App“ (Seite „Titel“, x 1720, y 0, Vorlage `beispiele/stand-der-app.body.js`, per `render` mit `replace_id`) eine Zeile „Löschen (3d.4)“ unter „Teilweise umgesetzt“ anlegen (ein Zielbild gibt es nicht, wie bei „Auf Original zurücksetzen (4.1b)“), mit dem Verweis auf die Frames.
- **Aus dem Gedächtnis (OpenPencil-Grenzen):**
  - Die Datei hat die Grenze von 10 000 Knoten; sie blockiert einzelne Knotenänderungen. Keine Einzeländerungen am Knoten versuchen, sondern den Frame neu rendern (`render` mit `replace_id`).
  - Ist der Bildschirm gesperrt, laufen die Aufrufe in Zeitüberschreitungen. Den Bildschirm entsperrt halten und den Lauf wiederholen.
  - OpenPencil lehnt Pfade unter `.claude/` ab: auf einer Kopie unter `gen4-studio/target/agent/` der Hauptkopie rendern, speichern, in den Worktree zurückkopieren.

  `check-tokens.sh` ist nicht betroffen.

## Noch zu zeichnen

- **Das Kästchen „Gelöschte zeigen“ im Kopf der Orte und der Liste:** Die Vorlage zeichnet es nur in der Trainerliste (aus) und im Brett (aus und an). `WeltListe` aus `jsx-lib-app.js` hat dafür keinen Platz; ein Frame der Welt mit eingeschaltetem Kästchen entsteht, wenn die Bibliothek eine Zeile im Kopf der Orte kennt.
- **Weitere Köpfe mit Plakette:** Trainer, Begegnungsdatei und Karte tragen dieselbe Plakette („geändert“, „neu“, „gelöscht“) vor dem Menü „Aktionen“; es sind dieselben Reihen mit anderem Namen.

## Was nicht gezeichnet werden muss

- **Weitere Zustände des Dialogs:** Die acht Zustände des Bretts decken die Sätze ab; die übrigen Ablehnungen („Scriptdateien und Textarchive haben keinen Grabstein …“, „Texturen und Modelle haben keinen Grabstein …“, „Echtes Löschen gibt es im Textprojekt …“, „{Name} ist schon gelöscht.“) sind derselbe Zustand „Abgelehnt“ mit anderem Satz, der Plan ohne Verweis mit „Original“ derselbe wie im Brett.
- **Weiche Verweise:** in 3d.4 leer; der Block „Weiche Verweise“ (Zeilen wie die Verweise, über dem Grabstein-Satz) kommt mit 3d.6.
- **Der Abschnitt „Löschen“ in Technik › Textprojekt:** Feld „Art“, Feld „Nummer“ und die Taste „Löschen…“, ein Abschnitt wie die übrigen der Fläche; sein Dialog ist der Dialog oben.
- **Das Feld „Tür“ mit gelöschtem Ziel:** derselbe Hinweis unter dem Namen wie bei „Das Ziel hat keine Tür 3. Wähle eine seiner Türen.“, nur der Satz „Das Ziel ist gelöscht.“ und „Ändern…“ gesperrt.
- **Dunkel und Kompakt:** Die Bausteine haben ihre Zustände schon in der Datei. Die Frames lassen sich mit `--dunkel` bauen.
