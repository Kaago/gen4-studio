# Briefing: App-Screens nachzeichnen (Stand der App in OpenPencil)

Du zeichnest Screens von **Gen4 Studio**, einer Tauri-App, mit der Menschen eigene Pokémon-Spiele für den Nintendo DS bauen, in die OpenPencil-Datei des Design-Systems „Doppeldeck“. Es geht um die App, **wie sie heute aussieht** (Stand 25.10.2026), nicht um das Zielbild. Vorlage sind Screenshots aus der Test-Umgebung. Ziel ist ein getreues, sauberes Abbild: gleiche Struktur, gleiche Texte, gleiche Maße und Farben (über Tokens).

Andere Agenten arbeiten gleichzeitig im selben Dokument auf anderen Seiten oder Reihen. Fasse nur deine eigenen Frames an.

## Werkzeuge

- **Bausteine:** `design-system/openpencil/jsx-lib-app.js`. Lies die Datei einmal ganz. Sie bildet den Rahmen der App ab (Titelleiste, Atlas, Liste, Deck, Scharnier, Hand, Statusleiste) und die Primitives (Taste, Feld, Zahlfeld, Segmente, Reiter, Inspector-Zeile, Abschnitt, Panelkopf, Listenzeile, Referenz, Plakette, Typ, Wertebalken, Pokémon-Karte, Meldung, DS-Bildschirm usw.) mit den Maßen aus dem Code.
- **Quellen für Maße:** die CSS-Module in `apps/studio/src/ui/{primitives,frame}/*.module.css` und die Bereiche in `apps/studio/src/{workbench,data,technik,einstellungen}/`. Lies sie, wenn ein Baustein fehlt.
- **Rendern ohne Riesen-Aufrufe:** Schreibe je Screen eine Datei `<name>.body.js` in deinen Arbeitsordner. Sie enthält nur den Screen-Code und endet mit `return Window({...});`; die Bibliothek steht davor im selben Gültigkeitsbereich.
  - Bauen: `bash /Users/POLTEP/Projekte/lab/editor/gen4-studio/design-system/openpencil/build-screen.sh <name>` (im Arbeitsordner; für das dunkle Thema `--dunkel` anhängen).
  - Rendern: `node /Users/POLTEP/Projekte/lab/editor/gen4-studio/design-system/scripts/op.mjs render '{"document_id":"tab-1","page_id":"<SEITE>","x":<X>,"y":<Y>}' --jsx <name>.jsx`. Die Antwort nennt die Frame-ID.
  - Ansehen: `node …/op.mjs export_image '{"document_id":"tab-1","page_id":"<SEITE>","ids":["<ID>"],"maxEdge":1440}' --png <name>.png`, dann die PNG mit dem Read-Werkzeug ansehen und **mit dem Screenshot vergleichen**.
  - Korrigieren: Body ändern, neu bauen, `render` mit `"replace_id":"<ID>"` statt x/y. Höchstens drei Korrekturläufe je Screen.
  - Jedes andere MCP-Werkzeug geht genauso (`get_page_tree`, `delete_node`, `node_bounds` …).
- **Nicht speichern.** Kein `save_file`, kein `close_file`. Das Speichern macht am Ende der Hauptagent, weil OpenPencil beim Speichern das Dokument neu lädt.
- Ändere die gemeinsame Bibliothek nicht. Brauchst du eine Variante, definiere im Body eine eigene Funktion mit neuem Namen (z. B. `const RowWide = …`).

## Regeln für das Abbild

- Frame 1440 × 900 je Screen, Name `App: <Bereich>, <Moment>` (z. B. „App: Pokédex, Staravia Werte“). Lage: x = Spalte × 1520, y = Reihe × 1000.
- **Texte wörtlich aus dem Screenshot** (Deutsch, Groß- und Kleinschreibung, Zahlen). Drei Ausnahmen:
  1. Projektname „Sinnoh-Remix“ statt `pt-ds-nachzug`.
  2. In der Statusleiste kein „Browser-Modus (Brücke)“.
  3. „Hier spielen“ mit Kürzel ⌘R (die gebündelte App zeigt es, der Browser nicht).
- Was der Screenshot abschneidet, darfst du sinnvoll ergänzen, solange es zum Code passt (Labels aus dem Code holen, nicht erfinden).
- Farben über `v('token')` (Tokens aus der Sammlung Thema, Szene). **Symbole bekommen Hex** (`HX` in der Bibliothek), weil OpenPencil Variablen an Vektoren nicht zuverlässig bindet. Für das dunkle Thema: `--dunkel` beim Bauen, Symbolfarben dann hell (z. B. `#ADB5C6`, `#E7EFF7`), `Atlas(active, true)`, `Window({ dark: true, … })`.
- Symbole: Phosphor (`ph:name`, gefüllt `ph:name-fill`), den passenden Namen wählen.
- **Keine nachgebauten Spielgrafiken:**
  - Pokémon- und Item-Symbole als `Sprite('S', 'pokemon')`.
  - Die 3D-Welt und Kartenbilder als abstrakte Flächen (Rechtecke in Grün, Ocker, Blau).
  - DS-Hintergründe ebenso abstrakt.
- Gestrichelte Konturen immer mit `strokeDash={[a, b]} dashPattern={[a, b]}`.
- Lange Texte in schmalen Spalten: `T(text, { one: true, width: N })`.
- Schriften: Atkinson Hyperlegible Next (Oberfläche), Atkinson Hyperlegible Mono (Kennungen), Zen Maru Gothic 700 nur wo die App sie nutzt (Seitentitel wie „Willkommen zurück“, „Technik“, Datenköpfe wie „STARAVIA“ im Pokédex), Pixelify Sans nur im DS-Bildschirm.
- **Qualitätsmaßstab:** Lege dein Bild neben den Screenshot.
  - Stimmen Raster, Abstände, Hierarchie und Texte?
  - Ist nichts abgeschnitten, und überlappt nichts?
  - Ein ruhiges, sauberes Abbild ist wichtiger als Pixelgleichheit.

## Abschluss

Antworte mit einer Liste „Frame-Name: ID“ und offenen Problemen, unter 250 Wörtern.
