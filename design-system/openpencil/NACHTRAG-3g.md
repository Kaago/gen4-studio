# Nachtrag 3g für die OpenPencil-Datei

Stand: 25.10.2026. Diese Anleitung sagt, wie der Screen zu Phase 3g.1 „Neues Spiel aus ROM“ in `Editor_DesignSystem.fig` kommt. **Die Datei wurde dabei nicht angefasst**: Die Phase-3g-Session hat nur die Vorlage `beispiele/projektstart.body.js` geschrieben und geprüft, dass sie baut. Eingespielt wird später, in einer Sitzung, der die `.fig` gehört (ein anderer Zweig ändert die Kernabschnitte des READMEs und die Datei selbst).

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

Die Antwort nennt die Frame-ID. Ansehen und vergleichen mit der Galerie (`?gallery`, Seite „Projekt“, „Assistent im eigenen Deck“) oder einem Screenshot der Test-Umgebung (Drop eines Pfads auf Start):

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
- Nicht gezeichnet: der Drop-Rahmen „ROM hier ablegen“, der Start-Screen mit dem Hauptknopf (Zielbild „Start: Willkommen zurück“ hat ihn noch nicht) und die Schritte Vorlage, Name und Ziel, Zusammenfassung, Ergebnis.

## Noch zu zeichnen

- **3g.1 (optional, dieselbe Seite):** „App: Start, Willkommen zurück“ mit der primären Taste „Neues Spiel aus ROM…“ und den beiden Sätzen darunter; „App: Start, ROM ablegen“ mit dem Rahmen (`data-drop=over`); „App: Start, Vorlage wählen“ mit den drei Karten (Original gewählt, „Leere Region mit Startstadt“ gesperrt mit „Kommt mit 3g.2“) und „App: Start, Projekt angelegt“ mit dem Ablauf und dem Ergebnis.
- **3g.2:** „App: Start, Leere Region wählen“ (die Karte „Leere Region mit Startstadt“ freigeschaltet und gewählt, mit dem Satz zur Vorlage) und „App: Technik, Welt leeren“ für den Assistenten „Welt leeren…“ im offenen Projekt (Technik › Werkzeuge). Beide folgen, sobald 3g.2 gebaut ist; Vorlage: dieselbe Datei `projektstart.body.js` mit geändertem Schritt.
