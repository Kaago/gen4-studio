# Nachtrag 3f für die OpenPencil-Datei

Stand: 26.10.2026. Diese Anleitung sagt, wie die Screens und Komponenten der Phase 3f „Weltmodell“ (3f.1 Raster nach Tüfteln, 3f.2 Eigene Orte, 3f.3 Leichter Patch, 3f.4 Nahtlos und Abspalten) in `Editor_DesignSystem.fig` kommen. **Die Datei wurde dabei nicht angefasst.** Diese Sitzung hat nur geschrieben:

- `beispiele/weltkarte-raumplan.body.js`: „Welt bauen“ mit dem Reiter „Weltkarte“ vorn in der Ansicht „Raumplan“ (Zellen 64 px, „+“ rechts und unten), der Inspector zeigt die gewählte Karte;
- `beispiele/assistent-ort.body.js`: Assistent „Gebiet anlegen“, Schritt „Ort“ („Vorhandener Ort | Neuer Ort“, „Aussehen wie Route 201“);
- `beispiele/zusammenfassung-patch.body.js`: derselbe Assistent, Schritt „Zusammenfassung“ mit der Patch-Zeile (Plakette „Nicht rückgängig“ und Zustimmung);
- `beispiele/inspector-geteilt.body.js`: ein Raum mit der Plakette „Geteilt mit 16 Räumen“ und dem offenen Menü „Aktionen“ mit „Für diesen Raum abspalten…“;
- neue Bausteine am Ende von `jsx-lib-app.js` (Abschnitt „Weltmodell (Phase 3f)“), nur angefügt: `GridCell`, `AreaGridX`, `WorldMapHead`, `PlaceBtn`, `PATCH_TEXT`, `PatchLine`, `SharedBadge`, `InspectorHeadB`, `WeltListe`, `WizardCol`, `WizardHand`. Die Bausteine darüber sind unverändert.

Geprüft ist, dass alle vier Vorlagen bauen (Hell und `--dunkel`), dass die erzeugte JSX syntaktisch stimmt (mit TypeScript übersetzt) und mit Platzhaltern für `Frame`, `Text`, `Icon`, `Ellipse` ohne Fehler läuft, dass jede `v('…')`-Variable in `tokens/` steht und jedes Symbol ein Phosphor-Name ist, und dass `werkbank-figur`, `werkbank-schwebend`, `welt-figur`, `welt-leeren`, `projektstart` und `leere-region` mit der erweiterten Bibliothek weiter laufen. **Nicht angesehen**: weder in OpenPencil noch als HTML-Nachbau. Das Rendern in die Datei übernimmt die Hauptsitzung, der die `.fig` gehört; sie schaut die Bilder an und korrigiert.

## Welche Seite, welcher Frame

- **Seite: „App: Werkbank“.** Grund: Alle vier Screens zeigen die Welt in der Anordnung „Werkbank“ (ein Dock, Scharnier oben, Inspector rechts); 3f ändert darin die Schau (in Bauen nur „3D“ und „Weltkarte“), die Weltkarte, den Inspector und den Assistenten. Gibt es die Seite noch nicht (der Nachtrag 3e ist nicht eingespielt), zuerst sie nach [NACHTRAG-3e.md](NACHTRAG-3e.md) anlegen; die Frames von 3e gehören in Reihe 0, die von 3f in Reihe 2 (Reihe 1 hält 3e für seine noch offenen Screens frei).
- **Frames** (je 1440 × 900, Lage x = Spalte × 1520, y = Reihe × 1000):

| Frame | Vorlage | Spalte, Reihe | x, y |
|---|---|---|---|
| App: Welt bauen, Raumplan (Weltkarte) | `weltkarte-raumplan` | 0, 2 | 0, 2000 |
| App: Welt bauen, Raumplan (Weltkarte, Dunkel) | `weltkarte-raumplan --dunkel` | 1, 2 | 1520, 2000 |
| App: Welt bauen, Gebiet anlegen, Schritt Ort | `assistent-ort` | 2, 2 | 3040, 2000 |
| App: Welt bauen, Gebiet anlegen, Zusammenfassung mit Patch-Zeile | `zusammenfassung-patch` | 3, 2 | 4560, 2000 |
| App: Welt bauen, Raum geteilt (Plakette) | `inspector-geteilt` | 4, 2 | 6080, 2000 |

  Vorher mit `get_page_tree` (depth 1) die belegten Positionen ablesen; ist Reihe 2 belegt, die nächste freie Reihe nehmen. Nur eigene Frames anfassen.
- Der dunkle Frame entsteht wie in 3e mit `build-screen.sh … --dunkel` (die Vorlage schaltet ihre Symbolfarben mit `HX_DUNKEL` selbst um); er heißt beim Rendern wie der helle, danach mit `rename_node` „, Dunkel“ in die Klammer setzen.

## Vor dem Rendern

Keine neuen Variablen: die Vorlagen nutzen nur Variablen, die es seit 3e gibt (`dichte/reiter` legt der Nachtrag 3e an, falls sie fehlt). Gestrichelte Kanten („+“, leere Zelle) tragen beide Angaben (`strokeDash` und `dashPattern`), wie das README unter „Arbeiten an der OpenPencil-Datei“ verlangt. Die Ortsfarben (Punkt am Picker „Ort“, Unterkante im Raumplan) und die Kartenbilder sind Beispiel-Hex wie die Punkte der Orte in `welt-figur.body.js`: in der App rechnet `placeColor` die Ortsfarbe zur Laufzeit aus dem Namen, und die Bilder kommen aus dem ROM.

## Einspielen

Aus der Repo-Wurzel, Dokument in OpenPencil geöffnet (`document_id` je nach Sitzung, meist `tab-1`; `<SEITE>` ist die ID der Seite „App: Werkbank“ aus `list_pages`):

```sh
cd design-system/openpencil/beispiele
bash ../build-screen.sh weltkarte-raumplan
node ../../scripts/op.mjs render '{"document_id":"tab-1","page_id":"<SEITE>","x":0,"y":2000}' --jsx weltkarte-raumplan.jsx
node ../../scripts/op.mjs export_image '{"document_id":"tab-1","page_id":"<SEITE>","ids":["<ID>"],"maxEdge":1440}' --png weltkarte-raumplan.png

bash ../build-screen.sh weltkarte-raumplan --dunkel
node ../../scripts/op.mjs render '{"document_id":"tab-1","page_id":"<SEITE>","x":1520,"y":2000}' --jsx weltkarte-raumplan.jsx
node ../../scripts/op.mjs rename_node '{"document_id":"tab-1","id":"<ID>","name":"App: Welt bauen, Raumplan (Weltkarte, Dunkel)"}'

bash ../build-screen.sh assistent-ort
node ../../scripts/op.mjs render '{"document_id":"tab-1","page_id":"<SEITE>","x":3040,"y":2000}' --jsx assistent-ort.jsx

bash ../build-screen.sh zusammenfassung-patch
node ../../scripts/op.mjs render '{"document_id":"tab-1","page_id":"<SEITE>","x":4560,"y":2000}' --jsx zusammenfassung-patch.jsx

bash ../build-screen.sh inspector-geteilt
node ../../scripts/op.mjs render '{"document_id":"tab-1","page_id":"<SEITE>","x":6080,"y":2000}' --jsx inspector-geteilt.jsx
```

Nach jedem `render` ein Bild holen (`export_image` wie oben) und vergleichen mit der Galerie (`?gallery`, Seite „Welt“: „Umschalter der Weltkarte“, „Raumplan“, „Picker „Ort““, „Neuer Ort“, „Patch-Zeile“, „Plakette „Geteilt““) und mit der Test-Umgebung (Platin-DE-Kopie, Tiefe Bauen): für den Raumplan Ewigwald › „Ebene 1“ wählen und in der Weltkarte die Karte in der Mitte anklicken; für den Assistenten „Neu anlegen“ › „Gebiet anlegen…“, Quelle Sinnoh mit den Zellen 3,26 und 4,26, Name „Neuheim“; für die Plakette Jubelstadt › „Pokémon-Center“ und das Menü „Aktionen“. Korrigieren: Body ändern, neu bauen, `render` mit `"replace_id":"<ID>"` statt `x`/`y` (höchstens drei Läufe je Frame). Scheitert ein `render` mittendrin, bleibt ein halber Frame zurück: mit `get_page_tree` prüfen und löschen. Die erzeugten `.jsx` und `.png` nicht einchecken (löschen).

### Brett „Komponenten (App)“

Auf der Seite „Komponenten“ im Brett „Komponenten (App)“ (aus dem Nachtrag 3e; fehlt es, anlegen) je Eintrag einen kleinen Frame rendern. Ein Body dafür ist ein paar Zeilen: die Bausteine in einen `Frame` legen und mit `return` zurückgeben, gebaut mit `build-screen.sh` wie ein Screen. Einträge (die Namen sind die der Komponenten im README) und Bausteine:

| Eintrag | Zustände | Baustein |
|---|---|---|
| Weltkarte mit Umschalter | Kopf in den Ansichten „Sinnoh“, „Raumplan“ (mit Untertitel „Ewigwald, Ebene 1“), HeartGold „Kanto“; das offene Menü mit „Sinnoh“, „Weitere Gebiete“, „Raumplan“ (Häkchen an der gezeigten Ansicht); Kachel „Weitere Gebiete“ (Name, „20 Karten“, Plan in 32 px) | `WorldMapHead(view, sub)`, `ContextMenu` (das Häkchen von Hand: `I('ph:check', 18)` vor dem Wort), Kachel aus `Frame` (`flaeche/blatt`, Rand `linie/standard`, Radius 6) mit `T`, `Mono` und `AreaGridX` mit `GridCell({ size: 32 })` auf `flaeche/welt` |
| Raumplan | Zelle mit Bild, ohne Bild in der Ortsfarbe, Gewählt, Leer, „+“ | `GridCell({ map, pic, tint, sel, plus })`, `AreaGridX(rows)` |
| Zellrahmen in 3D | Ein (ein flacher Ring, eine halbe Kachel breit, um eine Kachel der Szene) | `Welt3D` und darüber eine `Frame` ohne Fläche mit Kante `auswahl/halo` (Hell; Dunkel `auswahl/kern`), 6 px, um eine Kachel |
| Draufsicht | Ruhe, Gedrückt | in den „Ebenen der Schau“ eine zweite Gruppe hinter einem Trenner: `IconBtn('ph:mountains', { on })` (das Symbol „Karte“ der App ist `Mountains`) |
| Picker „Ort“ | Bauen „Ort“ mit Punkt und Namen; „Kein Ort“; Tüfteln „Ortsname“ mit „Nachricht 1 · Archiv 0433“; der offene Picker „Ort wählen“ mit „Kein Ort (Mysteriöser Ort)“, Namen mit Punkt und Zahl der Gebiete und Räume, Fuß „Neuer Ort…“ | `Row({ label: 'Ort', children: PlaceBtn(name, dot, tech) })`; der Picker aus `Frame` mit `SCHATTEN` (wie `ContextMenu`), `Search('Ort oder Nummer suchen')`, `ListRow({ title, lead: <Ellipse w={8} h={8} fill={dot} />, right })` und `Btn({ label: 'Neuer Ort…', icon: 'ph:plus', small: true })` |
| Neuer Ort, Ort umbenennen | Dialog „Neuer Ort“ (Feld „Name des Orts“, „Mitnehmen“ mit fünf Kästchen, eines leer; Warnung „Länger als 16 Zeichen: …“); Dialog „Ort umbenennen“ mit der Notiz „Trifft alle 7 Gebiete und Räume …“ | Dialog-`Frame` 480 breit, `flaeche/blatt`, Rand `linie/standard`, Radius 12, `SCHATTEN`; `Field`, `Check(on, label)`, `Notice(text, 'warn')`, `Btn` „Abbrechen“ und „Ort anlegen“ bzw. „Umbenennen“ (`Primär`) |
| Schritt „Ort“ | Vorhandener Ort (Taste mit Punkt), Neuer Ort (Feld); „Aussehen wie …“ mit Bild 64 | Spalten aus `assistent-ort.body.js` (`WizardCol`, `Seg`, `PlaceBtn`, `Field`, `Btn({ small: true })`) |
| Patch-Zeile | Zustimmung offen, gegeben; fehlerhaft (Fehlermeldung „Die Anpassung des Spielcodes für neue Orte ist fehlerhaft. Reparieren in Technik › Patches.“) | `PatchLine(false)`, `PatchLine(true)`, `Notice(text, 'fehler')` |
| Plakette „Geteilt“, Abspalten | „Geteilt mit 1 Raum“, „Geteilt mit 16 Räumen“; Baumzeile mit Symbol „geteilt“; Dialog „Raum abspalten?“ mit Plakette „Nicht rückgängig“, Satz, „Abbrechen“, „Abspalten“ | `SharedBadge(n)`, `TreeRowX({ end: I('ph:share-network', 18) })`, `InspectorHeadB({ badges: [SharedBadge(16)] })`, Dialog-`Frame` wie oben mit `Badge('Nicht rückgängig', 'unumkehrbar', 'ph:warning')` |
| Wo liegt das Gebiet? | „Eigene Karte (über Türen verbunden)“; „Auf der Weltkarte Sinnoh (nahtlos begehbar)“ mit Gitter in 32 px: Block frei (gewählt), Block auf einem Gebiet (Kante `status/fehler`, Fehlermeldung „Der Block muss auf freie Zellen oder Landschaft fallen.“) | `Seg([...], aktiv)`, `T` für die Notiz, `AreaGridX` mit `GridCell({ size: 32, sel })` bzw. `GridCell({ size: 32, err: true })`, `Notice(text, 'fehler')` |

### Marken und Brett „Stand der App“

1. Über dem Zielbild „Weltkarte: Region Sinnoh“ (Seite „Zielbild: Start und Welt“): `Stand('teilweise', 'Weltkarte mit Umschalter und Raumplan · App: Welt bauen, Raumplan (Weltkarte)')`; Abweichung in einem Satz: die Weltkarte ist ein Reiter der Schau mit dem Umschalter der Ansichten (Weltkarten, „Weitere Gebiete“, „Raumplan“), Karten wählt und legt man dort und in 3D an, die Region als Illustration gibt es nicht.
2. Über dem Zielbild „Raster: Matrix 0 (Tüfteln)“: `Stand('umgesetzt', 'nur in Tüfteln, seit 3f.1')`; in Bauen übernehmen Weltkarte, Raumplan und der Klick auf den Boden in 3D.
3. Über dem Zielbild „Ort anlegen: Anordnung“: `Stand('teilweise', 'Gebiet anlegen mit Schritt „Ort“ und „Wo liegt das Gebiet?“ · App: Welt bauen, Gebiet anlegen, Schritt Ort')`; Abweichung: die Schau zeigt während des Assistenten keine Live-Vorschau, der Spielcode für neue Orte wird in der Zusammenfassung einmal bestätigt.
4. Im Brett „Stand der App“ auf der Titelseite: Zeile „Weltmodell (3f)“ mit „Teilweise umgesetzt“ und dem Verweis auf die Frames in Reihe 2 der Seite „App: Werkbank“.

### Speichern

Speichern macht die Sitzung, der die Datei gehört (`save_file`), danach das Dokument in OpenPencil schließen und aus der Repo-Wurzel:

```sh
node design-system/scripts/fix_fig_text.mjs design-system/Editor_DesignSystem.fig --rgb555
```

Das Skript entfernt die falschen Glyphen-Umrisse und rastet die Farben auf das DS-Raster ein (auch die Beispiel-Hex der Ortsfarben und Kartenbilder); eine Sicherung `.vor-textfix` liegt danach neben der Datei. Zum Schluss:

1. `design-system/README.md`: im Absatz „Nahtlos und Abspalten (Phase 3f.4)“ unter „Was die App umsetzt“ den Satz „`Editor_DesignSystem.fig` zieht nach (…)“ ersetzen durch: „Seit dem <Datum> zeigt die Seite „App: Werkbank“ der `Editor_DesignSystem.fig` in Reihe 2 Raumplan (Hell, Dunkel), den Schritt „Ort“, die Zusammenfassung mit Patch-Zeile und den geteilten Raum; das Brett „Komponenten (App)“ hat die Einträge von 3f.“
2. `bash scripts/check-tokens.sh` ist nicht betroffen (keine neuen Tokens).

## Was die Vorlagen zeigen und was nicht

**„App: Welt bauen, Raumplan (Weltkarte)“** (`weltkarte-raumplan.body.js`): Titelleiste „Sinnoh-Remix“ (Platin, DE); die Liste „Welt“ mit „Orte“ (Sinnoh offen, Ewigwald offen mit den Räumen „Ebene 1“, gewählt und gezeigt, und „Ebene 2“) und „In dieser Szene“ (Wurzel „Ebene 1“, Karten 9 mit „Karte 0338 · Zelle 0, 0“ bis „Karte 0346 · Zelle 2, 2“, die mittlere gewählt; Figuren, Türen zu); das Scharnier mit „Ewigwald › Ebene 1 0203 › Karte 0342 Zelle 1, 1“; die Schau-Gruppe mit den Reitern „3D“ und „Weltkarte“ (kein „Raster“), vorn die Weltkarte: Kopf „Raumplan“ mit Pfeil und Untertitel „Ewigwald, Ebene 1“, darunter auf `flaeche/welt` das Gitter 3 × 3 in 64 px (Kartenbilder als Waldgrün-Platzhalter, die Nummer als Schild unten links, die Mitte mit der Kobaltkante) und „+“ in der Spalte rechts und der Zeile unten; der Inspector „Karte“ mit Titel „Karte 0342“, Kennung „Zelle 1, 1“, „Karte“ (0 bis 665) und „Höhe“ (0 bis 255) und der Notiz zu „Duplizieren“; Statusleiste „Ewigwald, Karte 0342 (1, 1)“. Ein Raum ohne Gebietsabschnitt hat in der App keine Ortsfarbe an den Zellen und kein Feld „Gebiet“ im Inspector; so ist er gezeichnet.

**„App: Welt bauen, Gebiet anlegen, Schritt Ort“** (`assistent-ort.body.js`): das Scharnier als Schrittleiste „Gebiet anlegen“ mit „Quelle“, „Anordnung“ (abgehakt), „Ort“ (aktiv), „Zusammenfassung“; das Dock darunter niedriger (Schau und Script/Probleme im Verhältnis 548 : 240), der Inspector mit dem gewählten Gebiet Zweiblattdorf und dem Feld „Ort“ (Picker-Taste); darunter der Schritt (286): Spalte „Ort“ mit „Vorhandener Ort | Neuer Ort“ (Neuer Ort gewählt) und „Name des neuen Orts“ = „Neuheim“, Spalte „Aussehen“ mit dem Bild der Vorlage (Platzhalter) und „Aussehen wie Route 201“; der Fuß „Legt das Gebiet „Neuheim“ mit 2 × 1 Karten an.“ mit „Nicht rückgängig“; Statusleiste „Schritt 3 von 4: Ort“.

**„App: Welt bauen, Gebiet anlegen, Zusammenfassung mit Patch-Zeile“** (`zusammenfassung-patch.body.js`): derselbe Rahmen in Schritt 4 („Anlegen“ im Scharnier), der Inspector zeigt die ungespeicherte Änderung „Musik Tag“ 1005 (Plakette „Ungespeichert“, Kobalt-Strich an der Zeile), die Titelleiste „1 Änderung nicht gespeichert“, wie im Gate (3.1); im Schritt der Info-Hinweis mit den sechs Sätzen der Zusammenfassung (wie im Gate (3.4), mit „Aussehen wie Route 201.“) und darunter die Patch-Zeile mit Plakette „Nicht rückgängig“ und dem angehakten Kästchen „Einmalig für dieses Projekt: Spielcode für neue Orte anpassen (DSPRE-kompatibel, Sicherung arm9.bin.backup)“.

**„App: Welt bauen, Raum geteilt (Plakette)“** (`inspector-geteilt.body.js`): Jubelstadt offen im Baum, „Pokémon-Center“ gewählt und gezeigt, am Ende der Zeile das Symbol „geteilt“ (`ph:share-network`); „In dieser Szene“ mit „Karte 0191 · Zelle 0, 0“; das Scharnier „Jubelstadt › Pokémon-Center 0006“; der Inspector (Gruppe mit Fokus) mit dem Kopf „Raum“, „Pokémon-Center“, „Jubelstadt · C01PC0101“, der Plakette „Geteilt mit 16 Räumen“ (Info, Symbol) und dem offenen Menü „Aktionen“: „Raum hier anbinden…“, „Ort umbenennen…“, „Für diesen Raum abspalten…“ (unter dem Zeiger); darunter der Abschnitt „Karten“ mit dem Feld „Ort“ (Taste „Jubelstadt“), „Aussehen“ und „Texte und Scripts“ zu.

Gemeinsam:
- Gebaut nur mit den Bausteinen aus `jsx-lib-app.js` und `v('token')`-Farben; Symbole als Phosphor-Namen mit Hex des Themas (`HX`, in Dunkel `HX_DUNKEL`). Eigene Hex-Werte nur für die Szene der Bibliothek (`Welt3D`), die Kartenbilder und Ortsfarben (Beispiele, siehe oben).
- Die Wörter stammen aus dem Code und den Entscheidungstabellen der Pläne 3f.1 bis 3f.4 (`WorldMapPanel`, `AreaGrid`, `breadcrumb`, `MatrixCellEditor`, `PlaceStep`, `LookPicker`, `PatchConsent`, `wizardSpecs`, `HeaderEditor`, `sharedRooms`, `WorldTree`). Zeigt die App andere Wörter, gilt die App.
- Beispielwerte, nicht aus der Kopie abgelesen: die Kartennummern 0338 bis 0346 um die 0342 des Gates, die Zählungen in den Bäumen (Figuren, Türen, Räume von Jubelstadt), die Kennung „Zweiblattdorf · T01“, die Abfolge der Räume von Jubelstadt. Die Inspector-Abschnitte zeigen nur das Feld „Ort“ (die übrigen Felder des Headers sind weggelassen, nicht verschwunden).
- Nicht gezeichnet: Hover und Fokus, Tooltips (auch der Plakette „Geteilt“), der offene Picker „Ort wählen“ und „Aussehen wählen“, die gesperrte Taste „Anlegen“ im Scharnier (WizardBar kennt keinen gesperrten Zustand; die Vorlage zeigt deshalb die Zustimmung angehakt), die Dialoge selbst (Neuer Ort, Ort umbenennen, Raum abspalten?, „Karte bei x 3, y 0 anlegen?“).

## Noch zu zeichnen

- **Weltkarte Sinnoh** (Ansicht `w0`): ein gewähltes Gebiet mit „+“ an freien Nachbarzellen, das offene Menü der Ansichten; **„Weitere Gebiete“** als Kachelfläche; **HeartGold „Kanto“** mit gedämpften Zellen der anderen Hälfte.
- **3D mit Zellrahmen und Draufsicht**: die gewählte Karte mit dem Ring, die Taste „Draufsicht“ gedrückt, die Szene senkrecht von oben.
- **Dialoge**: „Neuer Ort“ über der Werkbank (aus „+“ › „Neuer Ort…“ der Hierarchie), „Ort umbenennen“, „Raum abspalten?“, „Karte bei x 1, y 9 anlegen?“.
- **Assistent**: Schritt „Anordnung“ mit „Wo liegt das Gebiet?“ (Gitter der Weltkarte in 32 px, ein Block frei und einer rot); die Zusammenfassung ohne Haken und mit fehlerhaftem Patch; das Ergebnis „Gebiet „Neuheim“ angelegt: 2 Karten.“ mit „Die alten Dateien sind unter .g4/backup gesichert.“.
- **Tüfteln**: die Schau mit „3D“, „Raster“, „Weltkarte“; der Picker „Ortsname“ mit „Nachricht 1 · Archiv 0433“; der Schritt „Header“ des Assistenten.
