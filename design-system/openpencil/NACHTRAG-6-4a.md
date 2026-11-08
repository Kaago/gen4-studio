# Nachtrag 6.4a für die OpenPencil-Datei

Stand: 08.11.2026. Diese Anleitung sagt, was zu Phase 6.4a „Malen“ neu in `Editor_DesignSystem.fig` gehört: das Werkzeug „Gelände malen“ mit seinen sieben Unterwerkzeugen, die Abschnitte „Palette“, „Ebenen“ und „Kachel“ im Inspector einer Welt-Karte, der Satz „Nicht gebacken: …“ im Budget und der Dialog „Neue Karte malen“ mit seinem Entwurf. **Die Datei wurde nicht angefasst:** OpenPencil war beim Schreiben dieses Nachtrags nicht erreichbar (127.0.0.1:7600 antwortete mit HTTP 502), also ist nichts gerendert und nichts gespeichert. Die Frames von 6.4a folgen, sobald OpenPencil antwortet.

Was die App zeigt, beschreibt `design-system/README.md`, Abschnitt „Nachtrag 6.4a“. Texte und Zustände stammen von dort und aus dem Code: `apps/studio/src/workbench/{paintTools.ts,paint/PaintContext.tsx,NewPaintedMapDialog.tsx,ExtendAreaDialog.tsx}`, `ui/frame/{FloatingToolbar.tsx,FloatingToolbar.module.css}`, `app/ToolHints.tsx`, `world/scene/{strokes,strokePreview}.ts`, `workbench/inspector/{TilePalette,LayerList,TileEditor,MapBudgetSection,MatrixCellEditor}.tsx` mit ihren `.module.css`, `ui/primitives/icons.ts` und die Galerie-Seite „Welt“ (`ui/gallery/pages/WorldPage.tsx`, Blöcke „Malwerkzeuge“, „Palette“, „Ebenen“). Projekt „Sinnoh-Remix“, Platin DE, wie die übrigen App-Screens. Beispielwerte, keine Spieldaten, sind:
- die Kacheln: die sechs Rollen des synthetischen Tilesets mit Beispielnamen (`boden`, `weg`, `wasser_a`, `gras_a`, `fels`, `treppe`, `baum`, wie die Galerie), ihre „Bilder“ Platzhalter aus zwei Beispielfarben, keine Spielgrafik;
- die Karte in der Draufsicht: Flächen in der Lage der Gate-Karte (Gras, Wasser, Weg, Plateau mit Wandband und Treppe), keine Kacheln;
- Karte 0667 in Zelle (1, 0) neben 0666, Weltkarte 0289, Header 593, Tileset 0006 und die Sätze des Entwurfs folgen dem Phasen-Gate über die Oberfläche (Platin DE); das Budget (Modell 2.064 B, Terrain 216 B, VRAM 64.480 B · 42 Texturen) ist ein Beispiel.

Vergleichsquelle sind die Protokolle der Test-Umgebung unter `target/agent/gate-2026-11-07/6-4a/` (nicht im Repo): `browser.md` (Task 9: Dialog, Striche, Inspector, Werkzeugleiste bei schmaler Szene; Platin hell und Komfort, HeartGold dunkel und kompakt) und `ui-gate.md` (Phasen-Gate: Malansicht, Dialog, Statusleiste der Rampe, die Unterwerkzeuge als zweite Spalte), dazu die Spielbilder `pt-gemalt-ui/` und `hg-gemalt-ui/` (`tag`, `nacht`, `wasser`, `wiese`, `treppe`). Bilder der App gibt es dort nicht, nur ihre Beschreibung; beim ersten Rendern also zusätzlich die Test-Umgebung öffnen (Anleitung in `docs/agent-gate.md`, Abschnitt „Phase 6.4a“, sobald Task 13 ihn schreibt) und vergleichen: Aufbau, Maße und Texte. Das Gate steht in `docs/phase-6-4a-gate.md` (entsteht mit dem Gate).

Neue Tokens gibt es nicht. Zum ersten Mal in einem Frame stehen die Szenenfarben `szene/gelaende/{unbekannt,hohes-gras,wasser,blockiert,spezial}` (Kacheln ohne Bild); in OpenPencil heißen Szenen-Variablen ohne Präfix, die Vorlage ruft darum `v('gelaende/wasser')` usw. **Vor dem ersten Rendern prüfen,** ob diese fünf Variablen in der Datei stehen (`get_variables` oder die Variablen-Ansicht, Sammlung „Szene“); fehlen sie, mit den Werten aus `tokens/Szene/Wert.tokens.json` anlegen, sonst bricht `render` mittendrin ab. Neu sind sieben Symbole: `rectangle` (Phosphor `Rectangle`, in der Vorlage `ph:rectangle`), `bucket` (`PaintBucket`, `ph:paint-bucket`), `eraser` (`Eraser`, `ph:eraser`), `eyedropper` (`Eyedropper`, `ph:eyedropper`), `ramp` (`Stairs`, `ph:stairs`), `eye` (`Eye`, `ph:eye`) und `eyeSlash` (`EyeSlash`, `ph:eye-slash`); gedrückt zeigt die Vorlage die gefüllte Form (`…-fill`). „Pinsel“ nimmt `ph:paint-brush`, „Höhe“ `ph:mountains`.

**Die Vorlage ist nicht gerendert.** `beispiele/gelaende-malen.body.js` ist gegen `jsx-lib-app.js` geschrieben und lief mit einer Attrappe der Bausteine (`Frame`, `Text`, `Icon`, `Ellipse`, `designVar`; JSX über den TypeScript-Übersetzer) in allen vier Modi, hell und dunkel, ohne Fehler durch (`malen` 556, `entwurf` 466, `tileset` 587, `brett` 467 Knoten); ein Bild davon gibt es nicht. Beim ersten Rendern sind Maße und Umbrüche zu prüfen und die Vorlage nach dem Bild und der Test-Umgebung zu korrigieren. Wahrscheinliche Stellen:
- die zweite Spalte der Unterwerkzeuge (`position="absolute"` rechts neben der aufrechten Leiste, x = 52): wenn OpenPencil ein absolutes Kind außerhalb des Rahmens abschneidet, die Spalte als eigenen Rahmen neben die Leiste in die Szene legen;
- der Inspector ist länger als die Gruppe (Palette, Ebenen, Kachel, Quelle, Budget): unten wird abgeschnitten, wie in der App gerollt; wenn „Budget“ nicht mehr sichtbar ist, „Ebenen“ einklappen;
- die Palette bricht nicht um (die Vorlage hat höchstens sieben Kacheln je Gruppe in einer Reihe; die App bricht mit `flex-wrap` um);
- die Marke „90° ⇆“ unten rechts in der Kachel ist nach der Textlänge geschätzt (x = 26 − 6 je Zeichen); die Deckkraft der Vorschauplatten (`opacity` 0,85): wenn OpenPencil sie nicht trägt, die Füllung als Hex mit Alpha setzen;
- das Radio „Aktiv“ ist ein Kreis mit Punkt (die App zeigt das native Radio des Systems in `akzent/grund`).

## Was neu zu zeichnen ist

Drei Frames je 1440 × 900 (einer zusätzlich dunkel) und ein Brett, alle aus `beispiele/gelaende-malen.body.js` (`MODE` wählt). Diese Teile gibt es schon: Rahmen, Atlas, Statusleiste, Scharnier oben, die Liste „Welt“ (`WeltListe`, `TreeRowX`), Dock-Gruppen, die Werkzeugtaste (`ToolKey`), die Legende, Inspector-Kopf, `Section`, `Row`, `Field`, `NumField`, `Check`, `Notice`, `Badge`, die Weltkarte (`GridCell`, `AreaGridX`, `WorldMapHead`), die Abdunklung (`MitDialog`) und das Budget aus 6.2 (in der Vorlage neu aufgebaut wie in `hoehen-budget.body.js`). Neu sind:
- die Werkzeugleiste mit Unterwerkzeugen (`PaintToolbar`: aufrecht, „Gelände malen“ gedrückt, die sieben Tasten als zweite Spalte daneben oder unter einem Trennstrich; `PaintToolbarFlat`: flach in einer Zeile);
- die Palettenkachel (`PalTile`) und der Abschnitt „Palette“ (`Palette`), die Ebenenzeile (`LayerRow`) und der Abschnitt „Ebenen“ (`Layers`), der Abschnitt „Kachel“ (`TileSection`);
- die Draufsicht einer gemalten Karte mit Zellrahmen und Strichvorschau (`MalSzene`) und der Dialog „Neue Karte malen“.

1. **Frame „Gelände malen: Route 201“** (`MODE = 'malen'`).
   - **Liste „Welt“:** „Orte“ mit Sinnoh › Route 201 (gewählt, gezeigt); „In dieser Szene“ mit Route 201 › Karten › „Karte 0666“ („0, 0“) und „Karte 0667“ („1, 0“, gewählt), Figuren (1), Türen (1).
   - **Scharnier:** „Route 201 › Karte 0667 Zelle 1, 0“, Bauen, ein Schritt im Verlauf; Titelleiste mit einer ungespeicherten Änderung.
   - **Schau** (Reiter „3D“): Karte 0667 von oben mit dem Zellrahmen (`auswahl/halo`), links ein Streifen von 0666; ein Pinselstrich aus sechs Zellen als Platten (Bild der Kachel „weg“, 85 %). Oben links die Ebenen der Schau mit „Ereignisse“ und „Draufsicht“ gedrückt; die aufrechte Werkzeugleiste mit „Gelände malen“ gedrückt, rechts daneben die Unterwerkzeuge Pinsel (gedrückt), Rechteck, Füllen, Radierer, Pipette, Höhe, Rampe, je mit Buchstabe oben rechts; unten links „Legende“.
   - **Inspector:** „Karte 0667“, Kennung „Zelle 1, 0 · Route 201“, Plakette „Ungespeichert“; „Palette“ offen (Gruppen Boden, Gras, Wasser, Wand, Rampe, Teilmodell; „Kachel 0001“ gewählt), „Ebenen“ offen („Boden · Boden“ aktiv, „Büsche · Überlagerung“ mit „Entfernen“, darunter „Ebene 2“ und „Ebene hinzufügen“), „Kachel 0001“ eingeklappt, „Quelle“ eingeklappt, „Budget“ offen (Modell 2.064 von 61.440 B, Terrain 216, Gebäude 0 von 32, VRAM 64.480 · 42 Texturen).
   - **Statusleiste:** „Gelände malen · Pinsel · Ziehen malt · Rechtsklick nimmt auf · Z dreht · X spiegelt“, in der Mitte „Kachel x 9, y 14 · Karte 0667 · begehbar · Höhe 0 Stufen“.
2. **Frame „Gelände malen: Route 201 (Dunkel)“**: derselbe Modus, mit `--dunkel` gebaut (der Name folgt dem Thema).
3. **Frame „Neue Karte malen: Entwurf“** (`MODE = 'entwurf'`).
   - **Schau** (Reiter „Weltkarte“): ein Ausschnitt der Weltkarte in 32er-Zellen mit Kartennummern, die gewählte Zelle mit `auswahl/kante`, leere Zellen und ein „+“ gestrichelt (`linie/stark`), die Zelle des Entwurfs gestrichelt in `auswahl/kante` (2 px, wie `[data-draft-cell]`).
   - **Dialog** (480 breit, über der Abdunklung): „Neue Karte malen“; „Name“ „Malweg“ mit „Pflicht“; „Boden“ mit dem Abschnitt „Palette“ (Kachel 0000 gewählt); „Höhe“ 0 mit „Stufen, −16 bis 62“; die Notizen „Neue Karte „Malweg“ in Zelle x 1, y 0 der Weltkarte 0289.“, „Gebietsdaten von Header 593.“, „Boden aus Tileset 0006, Kachel 0000, Höhe 0.“; die Plakette „Nicht rückgängig“ mit „Karte 0667 wird angelegt“; „Abbrechen“ und „Karte malen“ (primär).
4. **Frame „Tileset-Editor“** (`MODE = 'tileset'`).
   - **Schau:** wie Frame 1 ohne Strich; unten rechts ein Ausschnitt des Inspectors: „Kachel 0000“ ohne Datei in HeartGold und Tüfteln: „Tileset im Projekt anlegen“ mit dem Satz „Die Vorgaben stehen dann in data/tilesets/0019.json und gelten für alle Welt-Karten dieses Pakets.“, alle Felder gesperrt („Gelände“ als „0x00“ mit dem Hinweis „0x00 NONE“, „Schrittgeräusch“ 0 mit „0 bis 127“), „Nicht eindeutig, Vorgabe Boden“.
   - **Inspector:** „Palette“ mit „Kachel 0002“ (Wasser) gewählt, „Ebenen“ eingeklappt, „Kachel 0002“ offen: „Rolle“ Wasser, „Gelände“ WATER_SEA, „Blockiert“ aus, „Wand“ leer („keine“, Hinweis „Kachelnummer, leer = keine“), „Von dir gesetzt“; „Quelle“ und „Budget“ eingeklappt.
5. **Brett „Komponenten (App): Malen“** (`MODE = 'brett'`), für die Seite „Komponenten“. Es zeigt:
   - die sieben Unterwerkzeuge in Ruhe und die flache Leiste mit „Gelände malen“ und „Pinsel“ gedrückt, mit dem Satz zur Lage in der aufrechten Leiste;
   - die Statusleiste je Unterwerkzeug (sieben Zeilen, die Rampe „Klick setzt Rampe nach Osten, 1 Stufe · Z dreht · 1/2 Stufen · ⇧Klick entfernt“);
   - Palettenkacheln mit Bild, ohne Bild (die fünf Platzhalterfarben), gewählt, gewählt mit der Marke „90° ⇆“, in Tüfteln mit Nummer;
   - die Ebenen zweimal: Boden aktiv; „Büsche“ aktiv und verborgen (Auge „Zeigen“ gedrückt);
   - das Budget nach einem abgelehnten Backen („Modell“ Voll, darunter „Nicht gebacken: Das gebackene Modell hat 63 120 Byte, das Spiel fasst höchstens 61 440 Byte; die Karte behält ihr letztes Modell. Das Spiel bekommt das letzte Modell.“);
   - die Notiz der binären Karte („Boden malen: Karte als Quelle übernehmen“ mit der Taste).

## Lage und Einspielen

- **Lage:**
  - Die Frames 1 bis 4 kommen auf die Seite „App: Werkbank“ in die nächste freie Reihe nach allen Phasen, die bis dahin eingespielt sind (auch nach den Frames von 6.2 und 6.3b, falls sie dann stehen); es gilt x = Spalte × 1520, y = Reihe × 1000, Frame 1 in Spalte 0, Frame 2 in Spalte 1, Frame 3 in Spalte 2, Frame 4 in Spalte 3.
  - Das Brett kommt auf die Seite „Komponenten“ unter das letzte Brett „Komponenten (App)“.
  - Vorher mit `get_page_tree` (depth 1) die belegten Positionen ablesen und nur eigene Frames anfassen. Die Datei liegt an der Grenze von 10.000 Knoten: dann lehnt OpenPencil Einzeländerungen (`update_node`, `set_text`, `set_layout`, `rename_node`) ab, neue Frames gehen nur über `render`, Korrekturen über `render` mit `replace_id`. Die Draufsicht ist darum aus Flächen gebaut, nicht aus 1.024 Kacheln.
- **Bauen:**
  1. Aus `openpencil/beispiele/` `bash ../build-screen.sh gelaende-malen` aufrufen (für Frame 2 mit `--dunkel`). Das Skript schreibt nur die `.jsx` neben die Vorlage und braucht OpenPencil nicht.
  2. In der erzeugten `gelaende-malen.jsx` `MODE` je Frame umstellen (`'malen'`, `'entwurf'`, `'tileset'`, `'brett'`).
  3. `node ../../scripts/op.mjs render '{"document_id":"tab-1","page_id":"<SEITE>","x":<x>,"y":<y>}' --jsx gelaende-malen.jsx` aufrufen (`<SEITE>` aus `list_pages`).
  4. Per `export_image` vergleichen (`node ../../scripts/op.mjs export_image '{"document_id":"tab-1","page_id":"<SEITE>","ids":["<ID>"],"maxEdge":1440}' --png gelaende-malen.png`): Frame 1 und 4 mit der Malansicht der Test-Umgebung (Werkzeugleiste, Inspector, Statusleiste), Frame 3 mit dem Dialog am „+“, das Brett mit der Galerie (`?gallery`, Seite „Welt“, Blöcke „Malwerkzeuge“, „Palette“, „Ebenen“).
  5. Korrigieren: den Body ändern, neu bauen, `render` mit `"replace_id":"<ID>"` statt `x`/`y` (höchstens drei Läufe je Frame). Scheitert ein `render` mittendrin, bleibt ein halber Frame zurück: mit `get_page_tree` prüfen und löschen. Die erzeugten `.jsx` und `.png` nicht einchecken, sondern löschen.
- **Danach:**
  1. Speichern (`save_file`) und das Dokument schließen. Bei gesperrtem Bildschirm laufen die RPCs in Zeitüberschreitungen; dann warten, nicht wiederholen.
  2. Aus der Repo-Wurzel `node design-system/scripts/fix_fig_text.mjs design-system/Editor_DesignSystem.fig --rgb555` aufrufen.
  3. Im README den Abschnitt „Nachtrag 6.4a“ um einen Satz ergänzen, dass die Frames seit … in der Datei stehen (mit Seite und Lage), und den Satz „Die Frames von 6.4a folgen, sobald OpenPencil antwortet“ dort, „Die Datei wurde nicht angefasst“ und „nicht gerendert“ hier ersetzen.
  4. Im Brett „Stand der App“ (Seite „Titel“, x 1720, y 0, Vorlage `beispiele/stand-der-app.body.js`, per `render` mit `replace_id`) eine Zeile „Gelände malen (6.4a)“ unter „Teilweise umgesetzt“ anlegen, mit dem Verweis auf die Frames.
  5. Im Brett „Komponenten (App)“ die sieben neuen Symbole in die Symbolreihe nehmen.

  `check-tokens.sh` ist nicht betroffen (keine neuen Tokens).

## Was nicht gezeichnet werden muss

- **Die übrigen Unterwerkzeuge in der Schau:** Rechteck (gestrichelter Rahmen des Entwurfs um die Platten), Füllen, Radierer (Platten schraffiert in `szene/gelaende/unbekannt`), Höhe und Rampe (Platten in `szene/hoehe/<Zielstufe>`, dazu die Schicht „Höhen“ aus 6.2) sind dieselbe Szene mit anderen Platten; das Brett nennt ihre Statusleisten.
- **Schrägsicht beim Malen:** dieselbe Leiste und derselbe Inspector, die Szene wie in den Frames der Welt.
- **Die flache Leiste in der Schau** (oben, unten verankert) und die aufrechte mit den Unterwerkzeugen unter den Werkzeugen (hohe Szene): das Brett zeigt die flache, Frame 1 die zweite Spalte.
- **Ablehnungen der Ebenen** („Die Bodenebene (0) bleibt.“, „Eine Ebene braucht einen Namen.“, „Ebene „Büsche“ gibt es schon.“) und des Tileset-Editors („Eine Kachel von 0000 bis 0005.“ am Feld „Wand“): der Fehlerweg des Inspectors bzw. die Fehlerzeile des Felds.
- **Der Hinweis „3 Zellen schon an der Grenze (−16 bis 62).“** und die Tüfteln-Token in der Statusleiste („Kachel 0041r1 · Ebene 0 · Höhe 2 (16 E)“): dieselbe Statusleiste mit anderem Text.
- **„Stattdessen neu malen“** im Dialog „Gebiet erweitern“: eine weitere Fuß-Taste des Dialogs aus 3f.
- **Die Erfolgsmeldung nach „Karte als Quelle übernehmen“** aus der Notiz der binären Karte: die Meldung aus 6.3b an anderer Stelle.
- **Kompakt:** die Bausteine haben ihre Zustände schon in der Datei; die Unterwerkzeuge folgen `dichte/werkzeug`.
