# Nachtrag 3e für die OpenPencil-Datei

Stand: 25.10.2026. Diese Anleitung sagt, wie die Screens und Komponenten der Phase 3e „Werkbank“ (3e.1 Scharnier oben, Teiler, schwebende Werkzeugleiste; 3e.2 Weltbaum; 3e.3 Hierarchie und Inspector-Kopf; 3e.4 ein Dock je Bereich) in `Editor_DesignSystem.fig` kommen. **Die Datei wurde dabei nicht angefasst.** Diese Sitzung hat nur geschrieben:

- `beispiele/werkbank-figur.body.js`: „Welt bauen: Figur bearbeiten“ in der Anordnung „Werkbank“ (Nachfolger von `welt-figur.body.js`, das die zwei Decks nach Phase 3 zeigt);
- `beispiele/werkbank-schwebend.body.js`: derselbe Moment mit schwebendem Inspector und offenem Kontextmenü des Reiters;
- neue Bausteine am Ende von `jsx-lib-app.js` (Abschnitt „Werkbank“), nur angefügt: die Bausteine darüber sind unverändert, andere Screens und der Zweig von 3g nutzen sie.

Geprüft ist, dass beide Vorlagen bauen (Hell und `--dunkel`), dass die erzeugte JSX syntaktisch stimmt, dass sie ohne Fehler läuft und jede `v('…')`-Variable in `tokens/` steht, und dass `welt-figur` und die drei Vorlagen von 3g mit der erweiterten Bibliothek weiter laufen. Angesehen wurden die Vorlagen als HTML-Nachbau, nicht in OpenPencil: Das Rendern in die Datei übernimmt die Sitzung, der die `.fig` gehört (ein anderer Agent spielt sie später ein).

## Welche Seite, welcher Frame

- **Seite: „App: Werkbank“ (neu)**, nach „App: Texte, Technik, Dialoge“. Grund: 3e ändert das Fensterraster selbst (ein Dock je Bereich, Scharnier als Kopfleiste über dem Dock, Hand an den Rändern), und der Plan 3e.4 bringt mehrere Screens dazu (Welt mit gewählter Figur in Hell, Dunkel und Kompakt, Raster mit Zelle, Assistent „Raum anlegen“ mit dem Schritt unter dem Dock, Inspector schwebend). Die Seite „App: Start und Welt“ zeigt den Stand nach Phase 3 mit zwei Decks („App: Welt bauen, Figur gewählt“); sie bleibt als Vorher stehen, damit Vorher und Nachher in der Datei nebeneinander lesbar sind. Am 25.10.2026 hatte die Datei elf Seiten und keine „App: Werkbank“ (`list_documents`).
- **Frames** (je 1440 × 900, Lage x = Spalte × 1520, y = Reihe × 1000):

| Frame | Vorlage | Spalte, Reihe | x, y |
|---|---|---|---|
| App: Welt bauen, Figur bearbeiten (Werkbank) | `werkbank-figur` | 0, 0 | 0, 0 |
| App: Welt bauen, Figur bearbeiten (Werkbank, Dunkel) | `werkbank-figur --dunkel` | 1, 0 | 1520, 0 |
| App: Welt bauen, Inspector schwebend (Werkbank) | `werkbank-schwebend` | 2, 0 | 3040, 0 |

  Reihe 1 bleibt für die Screens unter „Noch zu zeichnen“ (Kompakt 1100 × 720, Raster mit Zelle, Assistent). Vorher mit `get_page_tree` (depth 1) die belegten Positionen ablesen; wenn die Seite schon Frames hat, die nächste freie Spalte nehmen. Nur eigene Frames anfassen.
- Ein dunkler Body ist nicht nötig: `build-screen.sh … --dunkel` stellt `dunkel-map.js` voran, `v('…')` liefert dann die Hex-Werte des dunklen Themas, und die Vorlage schaltet die Symbolfarben selbst um (`HX_DUNKEL`, `Window({ dark })`). Der Frame heißt dabei wie der helle; nach dem Rendern mit `rename_node` „, Dunkel“ in die Klammer setzen.

## Vor dem Rendern: Variable `dichte/reiter`

Die Reiterleisten des Docks sind `dichte/reiter` hoch (Komfort 28 px, Kompakt 24 px; in `tokens/Dichte/*.tokens.json` und `tokens.css` seit 3e.4). Fehlt die Variable in der Datei, vorher anlegen (aus `design-system/openpencil/beispiele`, `<SAMMLUNG>` ist die ID der Sammlung „Dichte“ aus `list_collections`, die Modus-IDs von Komfort und Kompakt aus `get_collection`):

```sh
node ../../scripts/op.mjs find_variables '{"document_id":"tab-1","query":"dichte/reiter"}'
node ../../scripts/op.mjs list_collections '{"document_id":"tab-1"}'
node ../../scripts/op.mjs get_collection '{"document_id":"tab-1","id":"<SAMMLUNG>"}'
node ../../scripts/op.mjs create_variable '{"document_id":"tab-1","collection_id":"<SAMMLUNG>","name":"dichte/reiter","type":"FLOAT","value":"28"}'
node ../../scripts/op.mjs set_variable '{"document_id":"tab-1","id":"<VARIABLE>","mode":"<KOMFORT>","value":"28"}'
node ../../scripts/op.mjs set_variable '{"document_id":"tab-1","id":"<VARIABLE>","mode":"<KOMPAKT>","value":"24"}'
```

Die Vorlagen setzen die Höhe als Zahl (`DI.reiter`, wie die übrigen Maße der Bibliothek); wer die Bindung in der Datei will, bindet danach die Höhe der Frames „Reiterleiste“ mit `bind_variable` (`field` `height`) an `dichte/reiter`. Andere neue Variablen braucht keine Vorlage: `schatten/schwebend` ist als Schattenwert eingesetzt (`SCHATTEN` in der Bibliothek, Platin 1000 mit 24 % Hell, 48 % Dunkel), weil OpenPencil Schatten nicht an Variablen bindet; die Szenen-Farben heißen ohne Präfix (`ereignis/figur`, `ereignis/warp`, `auswahl/halo`).

## Einspielen

Aus der Repo-Wurzel, Dokument in OpenPencil geöffnet (`document_id` je nach Sitzung, meist `tab-1`). Seite anlegen und ihre ID merken:

```sh
node design-system/scripts/op.mjs create_page '{"document_id":"tab-1","name":"App: Werkbank"}'
node design-system/scripts/op.mjs get_page_tree '{"document_id":"tab-1","page_id":"<SEITE>","depth":1}'
```

Dann je Frame bauen, rendern, ansehen:

```sh
cd design-system/openpencil/beispiele
bash ../build-screen.sh werkbank-figur
node ../../scripts/op.mjs render '{"document_id":"tab-1","page_id":"<SEITE>","x":0,"y":0}' --jsx werkbank-figur.jsx
node ../../scripts/op.mjs export_image '{"document_id":"tab-1","page_id":"<SEITE>","ids":["<ID>"],"maxEdge":1440}' --png werkbank-figur.png

bash ../build-screen.sh werkbank-figur --dunkel
node ../../scripts/op.mjs render '{"document_id":"tab-1","page_id":"<SEITE>","x":1520,"y":0}' --jsx werkbank-figur.jsx
node ../../scripts/op.mjs rename_node '{"document_id":"tab-1","id":"<ID>","name":"App: Welt bauen, Figur bearbeiten (Werkbank, Dunkel)"}'

bash ../build-screen.sh werkbank-schwebend
node ../../scripts/op.mjs render '{"document_id":"tab-1","page_id":"<SEITE>","x":3040,"y":0}' --jsx werkbank-schwebend.jsx
```

Vergleichen mit der Galerie (`?gallery`, Seite „Rahmen“: „Scharnier oben“, „Werkzeugleiste schwebend“, „Hierarchie „In dieser Szene““, „Inspector-Kopf“, die Dock-Reiter) und einem Bild der Test-Umgebung (Kopie `3e-pt`, Welt, Zweiblattdorf, Figur „Objekt 2 · ID 2“ wählen; für die zweite Vorlage die Palette „Inspector schweben lassen“ und Rechtsklick auf den Reiter). Korrigieren: Body ändern, neu bauen, `render` mit `"replace_id":"<ID>"` statt `x`/`y` (höchstens drei Läufe). Scheitert ein `render` mittendrin (etwa an einer fehlenden Variablen), bleibt ein halber Frame zurück: mit `get_page_tree` prüfen und löschen. Die erzeugten `.jsx` und `.png` nicht einchecken (`.gitignore` deckt sie nicht ab: löschen).

### Brett „Komponenten (App)“

Auf der Seite „Komponenten“ neben den bestehenden Einträgen (oder als eigenes Brett „Komponenten (App)“, falls es fehlt) je Eintrag einen kleinen Frame rendern. Ein Body dafür ist ein paar Zeilen: die Bausteine der Bibliothek in einen `Frame` legen und mit `return` zurückgeben. Einträge und Bausteine:

| Eintrag | Zustände | Baustein |
|---|---|---|
| Werkzeugleiste schwebend | senkrecht (Anker Links) und waagrecht (Oben, Unten); Werkzeug Ruhe, Aktiv, Gesperrt; Gruppen-Trenner | `FloatingToolbar({ tool, anchor: 'links' \| 'oben', blocked })` in einem Frame mit `flaeche/welt` |
| Teiler | waagrecht („Höhe der Orte“); senkrecht als gedrehte Variante; Ruhe | `Teiler()`; Hover und Fokus (Linie dicker, `akzent/grund`) von Hand |
| Hierarchie | Ruhe mit Wurzel und sechs Gruppen, Suche, Leer („Noch keine Szene gezeigt.“), Eventdatei gesperrt (Plakette „Gesperrt“ an den vier Ereignisgruppen) | Teil `szene` aus `werkbank-figur.body.js`, `Empty`, `Badge('Gesperrt', 'gesperrt')` |
| Baumzeile | Ruhe, Gewählt, Gezeigt (`ShownMark`), Befund, Gesperrt; Tür mit zweiter Zeile „→ Haus 1“; Gruppe mit Zähler | `TreeRowX({ label, tech, sub, sel, shown, count, end })` |
| Inspector-Kopf | Figur, Tür, Gebiet (Bauen und Tüfteln: „Header“), Karte mit Menü; Ungespeichert, Gesperrt, Nicht rückgängig | `InspectorHead({ word, icon, title, ident, badges })`; Texte wie in der Galerie („Objekt 2 · ID 3“, „Warp 1 → Haus 1“, „Karte 0411“) |
| Dock-Reiter | Ruhe, Aktiv (Gruppe mit Fokus), Sichtbar (andere Gruppe), Ungespeichert (Kobalt-Punkt), Kontextmenü | `DockTab(label, state, { dot })`, `DockTabBar`, `ContextMenu` mit „Maximieren“, „Schweben lassen“, „Zurück ins Dock“, Trenner, „Anordnung zurücksetzen“ |
| Schwebende Gruppe | Titelleiste (Griff), Reiter, Schatten | `FloatingGroup({ x: 0, y: 0, w: 340, h: 240, tabs: ['Inspector'], active: 'Inspector' })` |
| Scharnier oben | Bearbeiten mit „Schau maximieren“ in Ruhe und gedrückt; ohne Griff | `ScharnierOben({ crumbs, maximized })`, `MaxButton(on)` |

### Marken und Brett „Stand der App“

1. Über dem Zielbild „Welt bauen: Figur bearbeiten“ (Seite „Zielbild: Start und Welt“) die Marke tauschen: `Stand('teilweise', 'Anordnung Werkbank · App: Welt bauen, Figur bearbeiten (Werkbank)')`. Abweichungen in einem Satz darunter: die Hand steht an den Rändern statt unter der Schau (links die Hierarchie in der Liste „Welt“, rechts der Inspector über die volle Höhe, unten Script und Probleme), die Werkzeuge schweben als Leiste in der 3D-Ansicht statt in einer Hand-Spalte, „An diesem Ort“ gibt es nicht mehr, das Scharnier ist die Kopfleiste über dem Dock, Erzählen fehlt.
2. Im Brett „Stand der App“ auf der Titelseite: Zeile „Werkbank (3e)“ mit „Teilweise umgesetzt“, Verweis auf die Seite „App: Werkbank“.

### Speichern

Speichern macht die Sitzung, der die Datei gehört (`save_file`, nicht über einen Screen-Agenten), danach das Dokument in OpenPencil schließen und aus der Repo-Wurzel:

```sh
node design-system/scripts/fix_fig_text.mjs design-system/Editor_DesignSystem.fig --rgb555
```

Das Skript entfernt die falschen Glyphen-Umrisse und rastet die Farben auf das DS-Raster ein; eine Sicherung `.vor-textfix` liegt danach neben der Datei. Zum Schluss:

1. `design-system/README.md`: in „Screens“ die Aufzählung der App-Seiten um „App: Werkbank“ ergänzen, und unter „Was die App umsetzt“ hinter dem Absatz zu 3e den Satz: „Seit dem <Datum> zeigt die Seite „App: Werkbank“ der `Editor_DesignSystem.fig` die Welt in der Anordnung Werkbank (Hell, Dunkel, Inspector schwebend); das Brett „Komponenten (App)“ hat Werkzeugleiste schwebend, Teiler, Hierarchie, Baumzeile, Inspector-Kopf, Dock-Reiter, Schwebende Gruppe und das Scharnier oben mit „Schau maximieren“.“ Den Satz „`Editor_DesignSystem.fig` zieht danach nach“ im Absatz 3e.1 dann streichen. Die Kernabschnitte (Prinzipien, Fensterraster, Komponenten) nur ändern, wenn ihr Besitzer es erlaubt.
2. `bash scripts/check-tokens.sh` ist nicht betroffen (`dichte/reiter` steht schon in `tokens/`); es prüft nur die eingecheckten Tokens.

## Was die Vorlagen zeigen und was nicht

**„App: Welt bauen, Figur bearbeiten (Werkbank)“** (`werkbank-figur.body.js`): Titelleiste „Sinnoh-Remix“ (Platin, DE, alles gespeichert); Atlas mit Welt aktiv; die Liste „Welt“ (272) mit dem Abschnitt „Orte“ (Ansicht Baum, Sortierung, „Neu anlegen“, Einklappen; „1 Weltkarte, 72 Gebiete, 492 Räume“; Suche „Gebiet, Raum oder Kürzel suchen“; Baum mit „Scripts“, „Sinnoh“ 72 und den Gebieten Zweiblattdorf (gezeigt: Zeichen `ph:grid-four` in `bereich/welt`, 6 Räume), Route 201, Sandgemme, Route 202), dem Teiler „Höhe der Orte“ (Anteil 0,31) und dem Abschnitt „In dieser Szene“ (Zurück, Vor gesperrt, Hinzufügen; Suche „Name oder Nummer suchen“; Wurzel Zweiblattdorf, Karten 1 mit „Karte 0411 · Zelle 0, 0“, Figuren 8 mit „Objekt 2 · ID 2“ gewählt, Türen 4 mit „Warp 0“ und „→ Haus 1“ bis „→ Haus 4“, Auslöser 2, Schilder 1, Scripts 2 mit „Scriptdatei 0412“ und „Level-Script 0009“). Rechts davon das Scharnier oben (dunkel, 44) mit „Zweiblattdorf › Gebiet 0411 › Objekt 2 · ID 2 Eventdatei 0390“, Bauen, Prüflicht (0, 101, 10), Verlauf, Wiederholen, „Hier spielen ⌘R“ und „Schau maximieren“; darunter das Dock: die Schau-Gruppe (Reiter 3D aktiv, Raster, Weltkarte; 28 hoch) mit der 3D-Szene, „Ebenen der Schau“ oben links, der Werkzeugleiste senkrecht darunter (Auswählen aktiv, Trenner zwischen den vier Gruppen) und „Legende“ unten links; unter der Schau die Gruppe „Script“/„Probleme“ (240, Script mit „Kein Script geöffnet.“); rechts der Inspector (340 breit, 788 hoch) mit dem Kopf (Augenbraue „Figur“, Titel „Objekt 2 · ID 2“, Kennung „Objekt 2 · Eventdatei 0390“, Menü „Aktionen“) und den Feldern der Figur (x und z, Lokale ID, Grafik, Bewegung, Blickrichtung, Script mit „Öffnen“ und „Neues Script“, Trainer „kein Trainer“ mit „Ändern…“, Sichtlinie, Sichtweite, y nur lesbar). Statusleiste mit Auswählen, Klick/Ziehen, „Zweiblattdorf“, „101 Warnungen, 10 Hinweise“, Komfort.

**„App: Welt bauen, Inspector schwebend (Werkbank)“** (`werkbank-schwebend.body.js`): derselbe Moment, der Inspector als schwebende Gruppe 340 × 480 am rechten Rand der Schau (Titelleiste 22 in `flaeche/deck`, `radius/fenster`, Schatten), Schau und Script/Probleme über die volle Breite des Docks (1080), der Reiter „Inspector“ aktiv (die Gruppe hat den Fokus) und sein Kontextmenü offen: „Maximieren“, „Schweben lassen“ (gesperrt, die Gruppe schwebt schon), „Zurück ins Dock“, Trenner, „Anordnung zurücksetzen“. Abweichend von der ersten Vorlage sind die Orte eingeklappt (Kopf mit „Neu anlegen“ und „Orte ausklappen“, eine Zeile „Zweiblattdorf“, die den Wähler öffnet), damit die Hierarchie mehr zeigt (Türen ganz, Auslöser).

Gemeinsam:
- Gebaut nur mit den Bausteinen aus `jsx-lib-app.js` und `v('token')`-Farben; Symbole als Phosphor-Namen mit Hex des Themas (`HX`, in Dunkel `HX_DUNKEL`), wie überall in der Datei. Eigene Hex-Werte nur für die Platzhalter der 3D-Welt (Kacheln, Dächer, Wasser wie in `welt-figur.body.js`) und die dunkle Kontur der Marker.
- Die Wörter stammen aus dem Code (`SceneHierarchy`, `sceneModel`, `WorldExplorer`, `WorldTree`, `InspectorHead`, `EventEditor`, `eventFields`, `TrainerField`, `FloatingToolbar`, `ScriptPanel`, Breadcrumb). Das Kontextmenü folgt der Zeile „Kontextmenü (deutsch)“ des Plans 3e.4; `ui/frame/dockMenu.ts` gab es beim Schreiben noch nicht. Zeigt die App andere Wörter, gilt die App.
- Beispielwerte: die Positionen der Figuren (wie in `welt-figur.body.js`), die Ziele der Türen (Haus 1 bis 4, passend zu den 6 Räumen), Script 3 der Figur, die Nummern von Trigger und Schild, die Raumzahlen der Gebiete und die Karte „Karte 0411 · Zelle 0, 0“ (das Beispiel des READMEs) sind nicht aus der Platin-DE-Kopie abgelesen. Die Zahlen „1 Weltkarte, 72 Gebiete, 492 Räume“ kommen aus dem Auftrag.
- Die Hierarchie ist höher als ihr Platz, wie in der App: In der ersten Vorlage endet sie sichtbar nach „Warp 0 → Haus 1“; Auslöser, Schilder und Scripts liegen darunter (im Frame vorhanden, abgeschnitten). Die Kennungen der Figuren („x 105, z 875“) weichen zuerst und sind gekürzt.
- Die Fokusfarben der Dock-Reiter folgen `dockview-theme.css`: Jeder sichtbare Reiter trägt 2 px Kobalt oben, nur der in der Gruppe mit Fokus dazu `flaeche/schau` (erste Vorlage: Schau, zweite: Inspector).
- Nicht gezeichnet: Hover- und Fokuszustände (Teiler, Reiter, Baumzeile), Tooltips der Werkzeuge, das Menü „Hinzufügen“ der Hierarchie, „Lage der Werkzeugleiste“, „Sortierung“, „Aktionen“; Befundzeichen an Wurzel und Türen; die maximierte Schau (Taste gedrückt); das Ziehen eines Reiters (Zustand „Gezogen“ und die Drop-Zonen); die Ansagen.

## Noch zu zeichnen

- **Kompakt 1100 × 720** (Gate 3e: „Bilder in 1440 × 900 und 1100 × 720, Dunkel und Kompakt“): dieselbe Vorlage mit `Object.assign(DI, DI_KOMPAKT)` am Anfang des Bodys, `Window` mit 1100 × 720 (eigene Variante im Body, die Bibliothek setzt 1440 × 900 fest), Statusleiste „Kompakt“; in einer Ansicht unter 400 px Höhe liegt die Werkzeugleiste flach oben (`anchor: 'oben'`).
- **Raster mit Zelle** (Reiter „Raster“ aktiv, Inspector-Kopf „Karte“ mit Menü „Teile importieren…“, „Teile exportieren…“, „Duplizieren“).
- **Schau maximiert** (Scharnier mit gedrückter Taste, Schau-Gruppe über dem ganzen Dock).
- **Doppeldeck für Texte, Pokémon, Gegenstände**: Schau-Gruppe oben, Bearbeiten-Gruppe unten 286 mit Reitern; Welt in „Doppeldeck“ (Inspector, Script, Probleme als Reiter unten).
- **Assistent mit dem Schritt unter dem Dock** („Raum anlegen“: Schrittleiste im Scharnier, der Streifen `data-wizard-hand` 286 hoch unter dem geschrumpften Dock).
- Danach die Marken über den betroffenen Zielbildern („Raster: Matrix 0“, „Ort anlegen: Anordnung“) und die Zeilen im Brett „Stand der App“.
