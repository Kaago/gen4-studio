# Nachtrag 6.3b für die OpenPencil-Datei

Stand: 07.11.2026. Diese Anleitung sagt, was zu Phase 6.3b „Backen und Build“ neu in `Editor_DesignSystem.fig` gehört: der Abschnitt „Quelle“ im Inspector einer Kartenzelle und der Konflikt einer Welt-Karte in Technik › Textprojekt. **Eingespielt am 09.11.2026** (Branch `phase-6-fig-nachtraege`, nach den Frames von 6.2; beim Schreiben dieses Nachtrags lauschte auf 127.0.0.1:7600 kein MCP-Server, die Anfrage endete mit 502). Auf der Seite „App: Werkbank“ steht „App: Inspector, Karte als Quelle“ (x 0, y 4000, Reihe 4), auf der Seite „App: Texte, Technik, Dialoge“ (dort stehen die Technik-Frames von 4.3b) „App: Textprojekt, Welt-Konflikt“ (x 0, y 3000, Reihe 3). Das Brett „Stand der App“ (Seite „Titel“, x 1720, y 0) ist neu gerendert mit der Zeile „Backen und Build (6.3b)“ unter „Teilweise umgesetzt“ (21 Screens). Gerendert in der Hauptkopie, gespeichert, in den Worktree kopiert, `fix_fig_text.mjs --rgb555` (248 Textknoten, 37 196 Farben), wieder geöffnet und als Bild geprüft. Die Screenshots unter `target/agent/gate-2026-11-07/6-3b/` lagen nicht mehr vor; verglichen wurde mit der Beschreibung unten.

Was die App zeigt, beschreibt `design-system/README.md`, Abschnitt „Nachtrag 6.3b“. Texte und Zustände stammen von dort und aus dem Code: `apps/studio/src/workbench/inspector/{WorldSourceSection.tsx,WorldSourceSection.module.css,MatrixCellEditor.tsx}`, `technik/{TextProjectSurface.tsx,stepWords.ts}`. Projekt „Sinnoh-Remix“, Platin DE, wie die übrigen App-Screens. Die Zahlen sind die des Laufs der Test-Umgebung (Karte 0000 Zweiblattdorf nach „Karte als Quelle übernehmen“: „Gebacken · Modell 38.876 B · Terrain 146 B“; der Member durch Karte 0001 ersetzt: die App zeigt 40 Abweichungen und „und 566 weitere Abweichungen“, die Vorlage fünf davon mit demselben Satz); das Dorf in der Schau ist ein Platzhalter, keine Spielgrafik.

Vergleichsquelle sind die Screenshots der Test-Umgebung unter `target/agent/gate-2026-11-07/6-3b/` (Browser-Fenster 1280 × 800, nicht im Repo, Lauf in `browser.md`): `blick-quelle-binaer-{hell,dunkel}.png`, `blick-quelle-welt-{hell,dunkel,kompakt}.png`, `blick-konflikt-welt-hell.png`, `blick-konflikt-wege-hell.png`, `blick-build-karten-backen-{hell,dunkel}.png`. Verglichen werden Aufbau, Abstände und Texte; die Frames sind 1440 × 900 wie alle App-Screens.

Neue Tokens, Komponenten und Symbole gibt es nicht. Der Abschnitt „Quelle“ ist `Section` mit Sätzen, kleinen sekundären Tasten und `Notice`; der Konflikt ist die Zeile des Abgleichs aus 4.3b (`beispiele/textprojekt.body.js`) mit anderen Texten.

**Die Vorlage ist seit dem 09.11.2026 gerendert** (416 und 257 Knoten in der Datei): `quelle` nach einem Korrekturlauf, `konflikt` im ersten Lauf. `beispiele/welt-quelle.body.js` ist gegen `jsx-lib-app.js` geschrieben und lief vorher nur mit einer Attrappe der Bausteine (`Frame`, `Text`, `Icon`, `Ellipse`, `designVar`; JSX über den TypeScript-Übersetzer) in beiden Modi, hell und dunkel, ohne Fehler durch (351 und 227 Knoten). Korrigiert nach dem ersten Bild: die Liste „Orte“ ist 288 statt 248 hoch (sonst war „Haus 2“ halb abgeschnitten). Die vorab vermuteten Stellen und was das Bild zeigt:
- der Ausschnitt „Zustand binär“ unten rechts in der Schau: er verdeckte im ersten Bild ein Haus und den Teich. Korrigiert: das Dorf ist flacher (336 statt 400 hoch, ab y 24), der Ausschnitt sitzt 140 über dem unteren Rand darunter; die Legende bleibt frei;
- die Felder „Karte“ und „Höhe“ stehen als Bezeichnung über dem Feld wie in den Screenshots; die Zeile „Gehört zu“ der App fehlt in der Vorlage (so geblieben);
- die Zeile des Backens bleibt bei 340 Breite einzeilig;
- die gesperrte Taste „Karte als Quelle übernehmen“ (`Btn({ off: true })`) ist blass mit grauer Schrift, wie `disabled` der App.

## Was neu zu zeichnen ist

Zwei Frames je 1440 × 900, beide aus `beispiele/welt-quelle.body.js` (`MODE` wählt). Diese Teile gibt es schon: Rahmen, Atlas, Statusleiste, Scharnier oben, die Liste „Welt“ (`WeltListe`, `TreeRowX`), Dock-Gruppen, die schwebende Werkzeugleiste, die Legende, Inspector-Kopf, `Section`, `Field`, `NumField`, `Notice`, `Badge`, die Liste „Technik“ und die Fläche Technik › Textprojekt aus 4.3b.

1. **Frame „App: Inspector, Karte als Quelle“** (`MODE = 'quelle'`).
   - **Liste „Welt“:** „Orte“ mit Scripts, Sinnoh › Zweiblattdorf (gewählt, gezeigt) mit Haus 1 und Haus 2; „In dieser Szene“ mit Zweiblattdorf › Karten › „Karte 0000“ (gewählt, „3, 27“), Figuren (8).
   - **Scharnier:** „Zweiblattdorf › Karte 0000 Zelle 3, 27“, Bauen, Prüflicht „0 · 101 · 162“, ein Schritt im Verlauf.
   - **Schau:** das Dorf als Platzhalter mit dem Zellrahmen der Karte (`auswahl/halo`), Werkzeugleiste mit „Auswählen“, unten links „Legende“; unten rechts ein Ausschnitt des Inspectors mit dem Abschnitt „Quelle“ einer binären Karte außerhalb eines Textprojekts: „Binäre Karte aus dem ROM oder aus DSPRE.“, „Karte als Quelle übernehmen“ gesperrt, darunter „Boden malen gibt es im Textprojekt (Technik › Textprojekt).“.
   - **Inspector:** „Karte 0000“, Kennung „Zelle 3, 27 · Zweiblattdorf“, die Felder „Karte“ (0) und „Höhe“ (0), der Abschnitt „Quelle“ offen: „Welt-Karte“, „Gebacken · Modell 38.876 B · Terrain 146 B“, Taste „Als binäre Karte übernehmen“, darunter die Erfolgsmeldung „Übernommen.“; dann „Budget“ eingeklappt.
   - **Statusleiste:** „Auswählen · Klick wählt · Ziehen verschiebt“, in der Mitte „Zweiblattdorf, Karte 0000 (3, 27)“.
2. **Frame „App: Textprojekt, Welt-Konflikt“** (`MODE = 'konflikt'`).
   - **Liste „Technik“:** acht Einträge, „Textprojekt“ gewählt mit „1 Konflikt“.
   - **Fläche:** Kopf „Textprojekt“ mit dem Satz und den Tasten „Jetzt abgleichen“, „Zurücknehmen…“; der Abschnitt „Auf zwei Seiten geändert“ (1) mit der Zeile „Karte 0000“ (Plakette „Konflikt“) aufgeklappt: „Diese Welt-Karte wurde außerhalb des Studios geändert.“, die Tabelle Feld / Studio / DSPRE mit fünf Zeilen `permissions[71].collision` bis `permissions[75].collision` (0 gegen 128), „und 566 weitere Abweichungen“, die Tasten „Neu backen“ und „Als binäre Karte übernehmen“; der Abschnitt „Zuletzt übernommen“ (0) mit „Nichts von außen geändert.“.
   - **Meldung** unten rechts: „1 Konflikt zwischen Studio und DSPRE“ mit „Lösen…“.

## Lage und Einspielen

- **Lage:**
  - Frame 1 kommt auf die Seite „App: Werkbank“, Frame 2 auf die Seite mit den Technik-Frames von 4.3b („App: Technik, Textprojekt“), jeweils in die nächste freie Reihe nach allen Phasen, die bis dahin eingespielt sind (auch nach den Frames von 6.2, falls sie dann stehen); es gilt x = Spalte × 1520, y = Reihe × 1000, Spalte 0.
  - Vorher mit `get_page_tree` (depth 1) die belegten Positionen ablesen und nur eigene Frames anfassen. Die Datei liegt an der Grenze von 10.000 Knoten: dann lehnt OpenPencil Einzeländerungen (`update_node`, `set_text`, `set_layout`, `rename_node`) ab, neue Frames gehen nur über `render`, Korrekturen über `render` mit `replace_id`.
- **Bauen:**
  1. Aus `openpencil/beispiele/` `bash ../build-screen.sh welt-quelle` aufrufen (für Dunkel mit `--dunkel`). Das Skript schreibt nur die `.jsx` neben die Vorlage und braucht OpenPencil nicht.
  2. In der erzeugten `welt-quelle.jsx` `MODE` je Frame umstellen (`'quelle'`, `'konflikt'`).
  3. `node ../../scripts/op.mjs render '{"document_id":"tab-1","page_id":"<SEITE>","x":<x>,"y":<y>}' --jsx welt-quelle.jsx` aufrufen (`<SEITE>` aus `list_pages`).
  4. Per `export_image` vergleichen (`node ../../scripts/op.mjs export_image '{"document_id":"tab-1","page_id":"<SEITE>","ids":["<ID>"],"maxEdge":1440}' --png welt-quelle.png`): Frame 1 mit `blick-quelle-welt-hell.png` (Abschnitt „Quelle“) und `blick-quelle-binaer-hell.png` (der Ausschnitt), Frame 2 mit `blick-konflikt-welt-hell.png` und `blick-konflikt-wege-hell.png`.
  5. Korrigieren: den Body ändern, neu bauen, `render` mit `"replace_id":"<ID>"` statt `x`/`y` (höchstens drei Läufe je Frame). Scheitert ein `render` mittendrin, bleibt ein halber Frame zurück: mit `get_page_tree` prüfen und löschen. Die erzeugten `.jsx` und `.png` nicht einchecken, sondern löschen.
- **Danach:**
  1. Speichern (`save_file`) und das Dokument schließen. Bei gesperrtem Bildschirm laufen die RPCs in Zeitüberschreitungen; dann warten, nicht wiederholen.
  2. Aus der Repo-Wurzel `node design-system/scripts/fix_fig_text.mjs design-system/Editor_DesignSystem.fig --rgb555` aufrufen.
  3. Im README den Abschnitt „Nachtrag 6.3b“ um einen Satz ergänzen, dass die Frames seit … in der Datei stehen (mit Seite und Lage), und den Satz „Die Frames von 6.3b folgen, sobald OpenPencil antwortet“ dort, „Die Datei wurde nicht angefasst“ und „nicht gerendert“ hier ersetzen.
  4. Im Brett „Stand der App“ (Seite „Titel“, x 1720, y 0, Vorlage `beispiele/stand-der-app.body.js`, per `render` mit `replace_id`) eine Zeile „Backen und Build (6.3b)“ unter „Teilweise umgesetzt“ anlegen, mit dem Verweis auf die Frames.

  `check-tokens.sh` ist nicht betroffen (keine neuen Tokens).

## Was nicht gezeichnet werden muss

- **Die Zeile „Karten backen“ im Build:** eine Zeile der vorhandenen Schrittliste mit anderem Namen („Karten backen · voll gebaut (erster Build, voll) · 0,36 s“); ein Bild davon ist `blick-build-karten-backen-*.png`.
- **Das abgelehnte Backen** (`[data-world-bake="refused"]`): dieselbe Fehlermeldung (`Notice`, Fehler) an der Stelle der Zeile des Backens, mit dem Satz des ersten Fehlers.
- **„Wird gelesen …“** vor der ersten Antwort von `world_bake` und die Meldungen „Übernommen. 1 Kachel weicht von der Höhe ab.“ / „Übernommen. 12 Kacheln weichen von der Höhe ab.“: derselbe Abschnitt mit anderem Satz.
- **Die Warnung bei einem ungespeicherten Strich** im Konflikt („„Als binäre Karte übernehmen“ verwirft die ungespeicherten Änderungen an diesem Eintrag.“): die Warnmeldung aus 4.3b mit anderem Wort.
- **Dunkel und Kompakt:** die Bausteine haben ihre Zustände schon in der Datei; beide Frames lassen sich mit `--dunkel` bauen, wenn ein dunkler Vergleich gewünscht ist.
