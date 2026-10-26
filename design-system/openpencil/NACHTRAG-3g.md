# Nachtrag 3g für die OpenPencil-Datei

Stand: 25.10.2026. Diese Anleitung sagt, wie die Screens zu Phase 3g.1 „Neues Spiel aus ROM“ und 3g.2 „Leere Region mit Startstadt“ in `Editor_DesignSystem.fig` kommen. **Die Datei wurde dabei nicht angefasst**: Die Phase-3g-Sessions haben nur die Vorlagen `beispiele/projektstart.body.js` (3g.1), `beispiele/leere-region.body.js` und `beispiele/welt-leeren.body.js` (3g.2) geschrieben und geprüft, dass sie bauen. Eingespielt wird später, in einer Sitzung, der die `.fig` gehört (ein anderer Zweig ändert die Kernabschnitte des READMEs und die Datei selbst).

## Welche Seite, welcher Frame

- **Seite: „App: Start und Welt“** (keine neue Seite). Grund: Der Screen zeigt den Bereich Start, und auf dieser Seite liegen schon die App-Screens von Start und Welt; das Zielbild dazu („Neues Spiel: Basis-ROM wählen“) steht auf der Seite „Zielbild: Start und Welt“, und der Vergleich Zielbild gegen App bleibt so auf zwei gleich benannten Seiten. Eine eigene Seite „App: Projektstart“ lohnt erst, wenn 3g.2 und das spätere Veröffentlichen mehrere Screens bringen; dann lassen sich die Frames ohne Umbau verschieben.
- **Frame:** „App: Start, Neues Spiel aus ROM“, 1440 × 900, Schritt 1 „ROM“ mit einer erkannten Platin-ROM (Spiel „Platin (Deutsch)“, Plakette „Freigegeben“, „Bekannte Basis-ROM: Platin DE (Testprojekt)“), Projekt „Sinnoh-Remix“ im Ziel der Schau. Die Titelleiste zeigt „Kein Projekt geöffnet“, denn der Assistent läuft ohne offenes Projekt.
- **Lage:** Reihe des Screens „App: Start, Willkommen zurück“ (falls die Seite einen hat; sonst Reihe 0), nächste freie Spalte: x = Spalte × 1520, y = Reihe × 1000. Vorher mit `get_page_tree` (depth 1) die belegten Positionen ablesen; nur eigene Frames anfassen.

## Einspielen

Aus der Repo-Wurzel, Dokument in OpenPencil geöffnet (`document_id` je nach Sitzung, meist `tab-1`; `<SEITE>` ist die ID der Seite „App: Start und Welt“ aus `list_pages`):

```sh
cd design-system/openpencil/beispiele
bash ../build-screen.sh projektstart
node ../../scripts/op.mjs render '{"document_id":"tab-1","page_id":"<SEITE>","x":<X>,"y":<Y>}' --jsx projektstart.jsx
```

Die Antwort nennt die Frame-ID. Ansehen und vergleichen mit der Galerie (`?gallery`, Seite „Projektstart“, „Assistent im eigenen Deck“) oder einem Screenshot der Test-Umgebung (Drop eines Pfads auf Start):

```sh
node ../../scripts/op.mjs export_image '{"document_id":"tab-1","page_id":"<SEITE>","ids":["<ID>"],"maxEdge":1440}' --png projektstart.png
```

Korrigieren: Body ändern, neu bauen, `render` mit `"replace_id":"<ID>"` statt `x`/`y` (höchstens drei Läufe). Die erzeugte `projektstart.jsx` nicht einchecken.

Danach speichern (der Hauptagent, nicht per `op.mjs`: `save_file`), das Dokument in OpenPencil schließen und aus der Repo-Wurzel ausführen:

```sh
node design-system/scripts/fix_fig_text.mjs design-system/Editor_DesignSystem.fig --rgb555
```

Das Skript entfernt die falschen Glyphen-Umrisse und rastet die Farben auf das DS-Raster ein; eine Sicherung `.vor-textfix` liegt danach neben der Datei. Zum Schluss:

1. `design-system/README.md`: den Abschnitt „Nachtrag 3g“ um einen Satz ergänzen, dass der Screen jetzt in der Datei steht, und die Zeile über die drei „App: …“-Seiten (Abschnitt „Screens“) nur dann ändern, wenn der Besitzer der Kernabschnitte es erlaubt.
2. Das Brett „Stand der App“ auf der Titelseite und die Marke über dem Zielbild „Neues Spiel: Basis-ROM wählen“ (Stand-Marke: teilweise umgesetzt, Verweis auf „App: Start, Neues Spiel aus ROM“, Abweichung: Vorlagenkarten und Fortschrittsliste statt Basis-ROM-Liste).
3. `bash scripts/check-tokens.sh` ist nicht betroffen (keine neuen Tokens); es prüft nur die eingecheckten Tokens.

## Was die Vorlage zeigt und was nicht

- Gebaut mit den Bausteinen aus `jsx-lib-app.js` (`Window`, `Deck`, `WizardBar`, `DSScreen`, `Field`, `Btn`, `Badge`, `Statusbar`) und `v('token')`-Farben; Symbole nur als Phosphor-Namen. Keine eigenen Hex-Farben.
- Die Schrittleiste zeigt die vier Schritte mit „ROM“ aktiv. Die Kantenlinie des Scharniers ist die der Welt (`bereich/welt-punkt`), wie in der App (Abweichung im README, „Nachtrag 3g“). Das Symbol im Chip der Leiste ist das der Welt, die App nutzt „open“.
- Der Atlas zeigt alle Bereiche in Ruhe; in der App sind sie bei offenem Assistenten gesperrt.
- Nicht gezeichnet: der Drop-Rahmen „ROM hier ablegen“, der Start-Screen mit dem Hauptknopf (Zielbild „Start: Willkommen zurück“ hat ihn noch nicht) und die Schritte Name und Ziel, Zusammenfassung, Ergebnis (der Schritt Vorlage steht in `leere-region.body.js`, siehe unten).

## 3g.2: zwei weitere Screens

Beide Vorlagen bauen wie `projektstart.body.js` nur mit `jsx-lib-app.js` (`Window`, `Deck`, `WizardBar`, `DSScreen`, `Btn`, `Badge`, `Statusbar`, `Field`, `ListPane`, `ListRow`, `PanelHead`, `DockTabs` und andere) und `v('token')`-Farben; Symbole nur als Phosphor-Namen. Neue Bausteine waren nicht nötig: Auswahlkarte, Spalte, Hinweis mit Aktionstaste und Fortschrittsbalken sind in den Vorlagen aus `Frame` und `T` zusammengesetzt (kleine Funktionen `Karte`, `Hinweis`, `Bar` im Kopf der Datei). `jsx-lib-app.js` wurde nicht geändert.

### „App: Start, Leere Region wählen“

- **Seite: „App: Start und Welt“**, wie der Screen von 3g.1. **Frame:** „App: Start, Leere Region wählen“, 1440 × 900.
- **Lage:** in der Reihe des Screens „App: Start, Neues Spiel aus ROM“, eine Spalte rechts davon: x = Spalte × 1520, y = Reihe × 1000. Vorher mit `get_page_tree` (depth 1) die belegten Positionen ablesen.
- **Bauen und einspielen** (Dokument offen, `<SEITE>` aus `list_pages`):

```sh
cd design-system/openpencil/beispiele
bash ../build-screen.sh leere-region
node ../../scripts/op.mjs render '{"document_id":"tab-1","page_id":"<SEITE>","x":<X>,"y":<Y>}' --jsx leere-region.jsx
node ../../scripts/op.mjs export_image '{"document_id":"tab-1","page_id":"<SEITE>","ids":["<ID>"],"maxEdge":1440}' --png leere-region.png
```

  Korrigieren: Body ändern, neu bauen, `render` mit `"replace_id":"<ID>"` (höchstens drei Läufe). Die erzeugte `leere-region.jsx` nicht einchecken (der Bau erzeugt sie neu). Vergleichen mit der Galerie (`?gallery`, Seite „Projektstart“) oder der Test-Umgebung: ROM ablegen, „Weiter“, im Schritt Vorlage die zweite Karte wählen.
- **Was die Vorlage zeigt:** Der Assistent im Schritt 2 „Vorlage“ (Schrittleiste mit „Vorlage“ aktiv, „ROM“ abgehakt), die Schau mit „POKEMON PL“ auf dem DS-Bildschirm und den Zeilen Spiel „Platin (Deutsch)“, Vorlage „Leere Region“, Ziel; die Hand mit der Frage „Womit startet das Projekt?“ und drei Karten in einer Reihe: „Original“ („Das Spiel, wie es ist.“), „Leere Region mit Startstadt“ (gewählt: `auswahl/flaeche`, doppelte Kante `auswahl/kante`, gefülltes Häkchen) mit dem Satz „Nur die Startstadt mit Spielerhaus bleibt, der Rest ist eine leere Region. Ein Start-Script gibt dir deinen ersten Partner.“ und „Nur Daten“. Titelleiste „Kein Projekt geöffnet“, Statusleiste „Schritt 2 von 4: Vorlage“.
- **Was sie nicht zeigt:** die Plakette „Nicht geprüft“ auf den Karten (sie erscheint nur bei einer ROM mit dieser Freigabe; die Vorlage zeigt Platin DE, Freigegeben); den Satz der Zusammenfassung mit „Scripts werden übernommen.“ (Schritt 4, im Fuß); den Pfeiltasten-Fokus und den Zustand „offen“ der Karten (nur die gewählte und zwei offene Karten ohne Fokusring). Die Atlas-Sperre bei offenem Assistenten ist wie bei 3g.1 nicht gezeichnet. Das Scharnier trägt wie dort die Linie der Welt.

### „App: Technik, Welt leeren“

- **Seite: „App: Start und Welt“** (der Assistent läuft im Deck der Welt, mit der Welt in der Schau; der Atlas steht auf Welt, nicht auf Technik). **Frame:** „App: Technik, Welt leeren“, 1440 × 900. Der Name folgt dem Einstieg (Technik › Werkzeuge › „Welt leeren…“); die Seite folgt dem, was die App zeigt.
- **Lage:** Reihe des Screens „App: Welt bauen, Figur gewählt“ (falls auf der Seite), nächste freie Spalte; sonst eine Reihe unter der von 3g.1. Regel wie oben: x = Spalte × 1520, y = Reihe × 1000.
- **Bauen und einspielen:** wie oben, mit `welt-leeren` statt `leere-region`.
- **Was die Vorlage zeigt:** das Welt-Deck wie im Screen „App: Welt bauen, Figur gewählt“ (Orte-Liste mit Zweiblattdorf geöffnet, 3D-Szene), das Scharnier als `WizardBar` „Welt leeren“ mit den Schritten „Was bleibt“ (aktiv) und „Zusammenfassung“, die Hand mit drei Spalten: „Bleibt“ (Zweiblattdorf bleibt mit 6 Räumen; Start-Script mit CHELAST auf Level 5, Pokédex und Laufschuhen), „Wird geleert“ (536 Orte, 467 Felder der Weltkarte) und rechts die Warnung „Dafür müssen die Scripts übernommen sein.“ mit der Taste „Scripts übernehmen“. Darunter der Fuß mit dem Satz zur Zusammenfassung und der Plakette „Nicht rückgängig“. Das Projekt „Sinnoh-Remix“ ist offen, Statusleiste „Schritt 1 von 2: Was bleibt“.
- **Was sie nicht zeigt:** den Einstieg in Technik › Werkzeuge (Karte „Welt leeren“ mit Taste „Welt leeren…“, `data-tool="welt-leeren"`; auch der gesperrte Zustand ohne Projekt, „Erst ein Projekt öffnen“); die laufende Übernahme (Fortschrittsbalken „0 / 1124 Scriptdateien“ ersetzt die Taste); die Warnung „Speichere zuerst, das Rezept schreibt Scripts direkt auf die Platte.“; den Schritt 2 mit der primären Taste „Welt leeren“ und den Info-Hinweis; das Ergebnis „Welt geleert“ mit „Zeigen“, „Hier spielen“, „Schließen“. In der Vorlage steht „Weiter“ noch aktiv, in der App ist es gesperrt, solange die Scripts nicht übernommen sind. Die Orte-Liste ist eine Auswahl der Platin-Liste und hat keine eigene Markierung für die zu leerenden Orte (die App markiert sie auch nicht). Die Zahlen sind die der Platin-DE-Testkopie. Die Szene und die Punkte der Orte-Liste benutzen wie `welt-figur.body.js` Beispiel-Hexfarben (Welt-Illustration, keine Oberflächenfarben).

## Noch zu zeichnen

- **3g.1 (optional, dieselbe Seite):** „App: Start, Willkommen zurück“ mit der primären Taste „Neues Spiel aus ROM…“ und den beiden Sätzen darunter; „App: Start, ROM ablegen“ mit dem Rahmen (`data-drop=over`); „App: Start, Projekt angelegt“ mit dem Ablauf und dem Ergebnis.
- **3g.2 (optional):** „App: Start, Zusammenfassung Leere Region“ (Fuß mit „Scripts werden übernommen.“); „App: Technik, Werkzeuge“ mit der Karte „Welt leeren“ (auch gesperrt, ohne Projekt); „App: Technik, Welt leeren, Scripts übernehmen“ (Fortschrittsbalken); „App: Technik, Welt leeren, Zusammenfassung“ (Info-Hinweis, primäre Taste „Welt leeren“) und „App: Technik, Welt geleert“ (Ergebnis mit „Zeigen“, „Hier spielen“, „Schließen“). Dazu die Marken im Brett „Stand der App“ und über den Zielbildern, sobald die Screens in der Datei stehen. Vorlage: dieselben Dateien, Schritt und Zustand ändern.
