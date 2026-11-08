# Nachtrag 6.5b für die OpenPencil-Datei

Stand: 08.11.2026. Diese Anleitung sagt, was zu Phase 6.5b „Freiform und Austausch“ neu in `Editor_DesignSystem.fig` gehört: der Dialog „Teile importieren…“ einer Welt-Karte mit den Zeilen „Freiform (glTF)“ (samt „Lage“) und „Tiled (TMJ)“, und der Abschnitt „Quelle“ mit der Taste „Als glTF übernehmen (Form 2)“. **Die Datei wurde nicht angefasst:** OpenPencil antwortete beim Schreiben dieses Nachtrags nicht (127.0.0.1:7600 gab HTTP 502), und 6.5b läuft parallel zu anderen Phasen, die in `main` zusammenkommen; die binäre `.fig` lässt sich nicht mergen. **Der Frame wird auf `main` nach dem Merge gerendert, sobald OpenPencil antwortet.**

Was die App zeigt, beschreibt `design-system/README.md`, Abschnitt „Nachtrag 6.5b“. Texte und Zustände stammen von dort und aus dem Code: `apps/studio/src/workbench/inspector/{ImportPartsDialog.tsx,ImportPartsDialog.module.css,ExportPartDialog.tsx,WorldSourceSection.tsx,WorldSourceSection.module.css,importParts.ts}`, `ui/primitives/{Dialog,Segmented,TextField,CheckList,Checkbox,Budget,Button,Notice,Section}.tsx`. Projekt „Sinnoh-Remix“, Platin DE. Beispielwerte, keine Spieldaten:
- das Testmodell `g4_imp_cube.obj` mit seiner Textur ist selbst erzeugt (`g4k import-samples`, `model_import::samples`), keine Spielgrafik; Karte 0666, „Einheit“ 32 und die Lage −40, 0, 56 folgen dem Szenario `pt-freiform` (der Würfel nördlich des Spielers), das Modell der Karte mit dem Würfel (2.144 B) dem Lauf dieses Szenarios;
- die Tiled-Datei `0666.tmj`, „37 Zellen“ und die `.per` daneben sind Beispiele;
- die Zahlen von Form 2 (815 Vanilla-Polygone, 816 gebackene Flächen, Modell 38.224 B) sind die Messung auf Platin DE, Karte 0 (`docs/formats/world_map.md`, „Form 2“), das Terrain 146 B kommt aus dem Frame von 6.3b; Material „lake_lm2“, Karte 0123 und Paket 0006 der Ablehnung sind das Muster aus dem Code.

Vergleichsquelle sind die Bilder unter `target/agent/gate-<datum>/6-5b/` (nicht im Repo): die Bilder des Browser-Teils, die mit dem Gate entstehen (Task 13, Test-Umgebung auf den Ports 1580/1581; Ablage nach `docs/phase-6-5b-gate.md`), und die Spielbilder der Szenarien unter `target/agent/gate-2026-11-07/6-5b/{pt,hg}-freiform/freiform.png`. Bilder der App gab es beim Schreiben noch nicht; beim ersten Rendern also zusätzlich die Test-Umgebung öffnen und vergleichen: Aufbau, Maße und Texte, nicht die Zahlen.

Neue Tokens gibt es nicht, neue Symbole auch nicht. Die Frames nutzen `flaeche/blatt`, `flaeche/deck`, `flaeche/ueberlagerung`, `linie/standard`, `linie/leise`, `text/primaer`, `text/sekundaer`, `akzent/grund` (Budget), die Status-Flächen der Meldungen (`status/erfolg-flaeche`, `status/fehler-flaeche`) und die vorhandenen Zustandssymbole (`ph:check-circle-fill`, `ph:x-circle-fill`, `ph:info-fill`).

**Die Vorlage ist nicht gerendert.** `beispiele/welt-austausch.body.js` ist gegen `jsx-lib-app.js` geschrieben und lief mit einer Attrappe der Bausteine (`Frame`, `Text`, `Icon`, `Ellipse`, `designVar`; JSX über den TypeScript-Übersetzer, jeder Token gegen `design-system/tokens` geprüft) in beiden Modi, hell und dunkel, ohne Fehler durch (`import` 120, `quelle` 91 Knoten; dieselbe Attrappe zählt für `import-modell.body.js` die 126 Knoten aus Nachtrag 6.5a); ein Bild davon gibt es nicht. Beim ersten Rendern Maße und Umbrüche prüfen. Wahrscheinliche Stellen:
- „Lage“: drei Felder zu 96 px mit Bezeichnung darüber in einer Reihe (Abstand 12); die Hilfe „Welteinheiten; 16 = eine Kachel; 0, 0, 0 = Kartenmitte“ muss in einer Zeile unter den Feldern stehen;
- die Zeile „Freiform (glTF) · g4_imp_cube.obj“: Symbol, Kästchen und Zeilen in einer Spalte von 414 px; der lange Satz von `tmj.buildings-ignored` in der rechten Spalte muss dort umbrechen;
- im Abschnitt „Quelle“ stehen die zwei Tasten in der Vorlage untereinander (die App bricht die Zeile bei 324 px um, `flex-wrap`); wenn sie im Bild nebeneinander passen, so lassen, wie die App sie zeigt;
- die Fehlermeldung von `world.adopt-texture` ist lang und bricht in der `Notice` mehrmals um.

## Was neu zu zeichnen ist

Zwei Frames, je hell und dunkel, beide aus `beispiele/welt-austausch.body.js` (`MODE` wählt). Diese Teile gibt es schon: `Dialog`, `Segmented` (klein), `Field`, `Check`, `Btn`, `Badge`, `Notice`, `Section`, `InspectorHead` und das Budget aus der Vorlage von 6.5a (`import-modell.body.js`, hier als `BudgetModell` nachgebaut, damit die Vorlage für sich steht).

1. **Frame „Teile importieren: Freiform und Tiled“** (`MODE = 'import'`; zwei Dialoge zu 480 px nebeneinander auf der Abdunklung `flaeche/ueberlagerung`, 1080 breit).
   - **Links, Freiform:** Titel „Teile in Karte 0666 importieren“; Segmente „Quelle“ mit „Datei“ gewählt, Feld „Pfade, eine je Zeile“ mit `~/Modelle/g4_imp_cube.obj`, „Dateien wählen…“; „Umwandeln“ mit „Farben je Textur“ (16 gewählt, 4) und „Einheit“ 32 mit Hilfe; **„Lage“** mit den Feldern x −40, y 0, z 56 und der Hilfe; die Prüfliste mit **„Freiform (glTF) · g4_imp_cube.obj“** (ok, angekreuzt), darunter „OBJ-Modell umgewandelt: 6 Flächen, 6 Vierecke, 0 Dreiecke, 2 Streifen.“, „Neue Textur g4_imp_cube: 16 × 16, 16 Farben, mittlerer Farbfehler 7,6.“, die Vorschau „g4_imp_cube · 16 × 16 · 16 Farben · neu“ und das Budget „Modell“ Ruhig „2.144 von 61.440 B“. Tasten „Abbrechen“ und „1 Teil übernehmen“ (primär).
   - **Rechts, Tiled:** dieselbe Quelle mit `~/Tiled/0666.tmj` und `~/Tiled/karte_0666.per`; die Notiz „Bei Welt-Karten entstehen Modell, Terrain und Permissions beim Backen.“; die Zeile **„Tiled (TMJ) · 0666.tmj“** (ok, angekreuzt) mit „1 Ebene, 37 Zellen geändert“ und „Die Gebäude der Tiled-Datei weichen von denen der Karte ab und werden nicht übernommen; Gebäude bearbeitest du im Studio.“, darunter die Zeile „Kollision · 2.048 Bytes · karte_0666.per“ mit gesperrtem Kästchen. Tasten „Abbrechen“ und „1 Teil übernehmen“.
2. **Frame „Quelle: Als glTF übernehmen“** (`MODE = 'quelle'`; drei Ausschnitte des Inspectors zu 340 px nebeneinander auf `flaeche/deck`, je Kopf „Karte …“, Abschnitt „Quelle“ offen, „Budget“ eingeklappt).
   - **Binäre Karte:** „Binäre Karte aus dem ROM oder aus DSPRE.“, die Tasten „Karte als Quelle übernehmen“ und „Als glTF übernehmen (Form 2)“ (sekundär klein).
   - **Nach Form 2:** „Welt-Karte“, „Gebacken · Modell 38.224 B · Terrain 146 B“, „Als binäre Karte übernehmen“ und die Erfolgsmeldung „Übernommen als glTF: 815 Vanilla-Polygone, 816 gebackene Flächen.“
   - **Ablehnung:** die binäre Karte 0123 mit beiden Tasten und darunter die Fehlermeldung „Material „lake_lm2“ der Karte 0123 bindet keine Textur des Pakets 0006; Form 2 geht für diese Karte nicht, Form 1 schon. (world.adopt-texture)“.

## Lage und Einspielen (auf `main`, nach dem Merge)

- **Lage:** Seite „App: Werkbank“ (dort stehen die Dialoge der Welt), nächste freie Reihe nach allen Phasen, die bis dahin eingespielt sind; x = Spalte × 1520, y = Reihe × 1000, Hell in Spalte 0, Dunkel in Spalte 1; „Quelle: Als glTF übernehmen“ in der Reihe darunter. Vorher mit `get_page_tree` (depth 1) die belegten Positionen ablesen und nur eigene Frames anfassen. An der Grenze von 10.000 Knoten gehen neue Frames nur über `render`, Korrekturen über `render` mit `replace_id`.
- **Bauen:**
  1. Aus `openpencil/beispiele/` `bash ../build-screen.sh welt-austausch` (Hell) und `bash ../build-screen.sh welt-austausch --dunkel` (Dunkel) aufrufen; für den zweiten Frame in der erzeugten `.jsx` `MODE = 'quelle'` setzen. Das Skript schreibt nur die `.jsx` neben die Vorlage.
  2. `node ../../scripts/op.mjs render '{"document_id":"tab-1","page_id":"<SEITE>","x":<x>,"y":<y>}' --jsx welt-austausch.jsx` (`<SEITE>` aus `list_pages`).
  3. Per `export_image` mit den Bildern der Test-Umgebung vergleichen (Dialog einer Welt-Karte mit „Lage“ und den beiden Zeilen, Abschnitt „Quelle“).
  4. Korrigieren: Body ändern, neu bauen, `render` mit `"replace_id":"<ID>"` (höchstens drei Läufe je Frame). Ein halber Frame nach einem abgebrochenen `render` wird gelöscht. Die erzeugten `.jsx` und `.png` nicht einchecken.
- **Danach:**
  1. Speichern (`save_file`), schließen; bei gesperrtem Bildschirm warten, nicht wiederholen.
  2. Aus der Repo-Wurzel `node design-system/scripts/fix_fig_text.mjs design-system/Editor_DesignSystem.fig --rgb555`.
  3. Im README den Satz „Die `.fig` kennt die Frames noch nicht“ im Abschnitt „Nachtrag 6.5b“ durch Seite und Lage ersetzen, hier „nicht gerendert“ und „nicht angefasst“ ebenso.
  4. Im Brett „Stand der App“ (Seite „Titel“, Vorlage `beispiele/stand-der-app.body.js`, `render` mit `replace_id`) die Zeile „Kartenteile importieren“ um „Freiform und Tiled (6.5b)“ ergänzen.

  `check-tokens.sh` ist nicht betroffen (keine neuen Tokens).

## Was nicht gezeichnet werden muss

- **Freiform mit Ablehnung:** dieselbe Zeile mit Fehlersymbol und gesperrtem Kästchen, darüber der Satz des Codes (etwa „Das Modell ist … Byte groß …“ oder `bake.model-too-big`), das Budget „Voll“ wie im Frame von 6.5a.
- **Tiled mit Fehlern:** die Zeile mit Fehlersymbol und den ersten drei Fehlern „layers[0].data[517]: GID 500 gehört nicht in diese Ebene.“ … „… und 4 weitere“; „Keine Änderung.“ mit dem Info-Symbol (nicht wählbar).
- **„Lage“ mit Fehler:** ein Feld mit Fehlerkante und dem Satz des Zahlfelds, wie jedes `TextField` mit Fehler.
- **Export „Tiled (TMJ)“:** der Dialog „Teil von Karte 0666 exportieren“ von 6.5a mit dem letzten Segment „Tiled (TMJ)“ gewählt, Pfad `0666.tmj` und dem Satz „Kachelbilder kommen in den Ordner 0666.tiles daneben.“ (`text/s` sekundär).
- **Binäre Karte außerhalb eines Textprojekts:** beide Tasten gesperrt, darunter „Boden malen gibt es im Textprojekt (Technik › Textprojekt).“ wie im Frame von 6.3b.
- **Szene:** Freiform-Modelle und Teilmodell-Geometrie zeigt das vorhandene Netz der Welt-Karte (Nachtrag 6.3a); kein eigener Frame.
