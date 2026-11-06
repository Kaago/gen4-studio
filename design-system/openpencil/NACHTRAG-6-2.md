# Nachtrag 6.2 für die OpenPencil-Datei

Stand: 06.11.2026. Diese Anleitung sagt, was zu Phase 6.2 „BDHC“ neu in `Editor_DesignSystem.fig` gehört: die Schicht „Höhen“ in der Schau, der Abschnitt „Budget“ im Inspector einer Karte und der Baustein `Budget`. **Die Datei wurde nicht angefasst:** OpenPencil war beim Schreiben dieses Nachtrags nicht erreichbar (der MCP-Server auf 127.0.0.1:7600 lehnte die Verbindung ab), also ist nichts gerendert und nichts gespeichert. Die Frames von 6.2 folgen, sobald OpenPencil antwortet.

Was die App zeigt, beschreibt `design-system/README.md`, Abschnitt „Nachtrag 6.2“. Texte und Zustände stammen von dort und aus dem Code: `apps/studio/src/world/{ground,hover}.ts`, `world/scene/heightLayer.ts`, `app/useSceneLayers.ts`, `workbench/SceneBar.tsx`, `ui/primitives/{Budget.tsx,Budget.module.css,icons.ts}`, `workbench/inspector/{MapBudgetSection,MatrixCellEditor}.tsx` und die Galerie-Seite „Welt“ (`ui/gallery/pages/WorldPage.tsx`, Blöcke „Budget“ und „Höhenstufen“). Projekt „Sinnoh-Remix“, Platin DE, wie die übrigen App-Screens. Beispielwerte, keine Spieldaten, sind:
- die Karte der Schicht „Höhen“ (eine Zweiblattdorf-ähnliche Fläche in den Stufen 2, 3 und 4, Platzhalter-Grund wie `Welt3D`, keine Spielgrafik);
- die Zahlen des Budgets im Frame „App: Inspector, Karte mit Budget“ (Modell 57.344 B Knapp, VRAM 112.896 B Knapp): die echte Karte 188 zeigt in der Test-Umgebung alles Ruhig (Modell 2.244 B, Terrain 21.452 B, 16 Gebäude, VRAM 21.760 B · 45 Texturen); der Frame zeigt die Zustände, die der Plan für ihn verlangt.

Vergleichsquelle sind die Screenshots der Test-Umgebung unter `target/agent/gate-2026-11-06/6-2/` (1440 × 900, nicht im Repo, je `-hell`, `-dunkel`, `-kompakt`): `blick-karte100-{gelaende,hoehen,figur,budget}-*.png` (Karte 100, Stärkeufer), `blick-karte188-budget-*.png` (Sandgemme, Haus 2), `blick-karte242-budget-*.png` (Herzhofen, Wettbewerbshalle, „Gebäude“ Knapp „32 von 32“). Sie zeigen echte Werte des Testprojekts, die Frames Beispielwerte; verglichen werden Aufbau, Maße und Texte. Das Gate steht in `docs/phase-6-2-gate.md` (entsteht mit dem Gate).

Neue Tokens gibt es nicht. Die Schicht nutzt zum ersten Mal `szene/hoehe/0` bis `szene/hoehe/6`; in OpenPencil heißen Szenen-Variablen ohne Präfix, die Vorlage ruft darum `v('hoehe/0')` bis `v('hoehe/6')`. **Vor dem ersten Rendern prüfen,** ob diese sieben Variablen in der Datei stehen (`get_variables` oder die Variablen-Ansicht, Sammlung „Szene“); fehlen sie, mit den Werten aus `tokens/Szene/Wert.tokens.json` anlegen, sonst bricht `render` mittendrin ab. Neu sind zwei Symbole: `heights` (Phosphor `Mountains`, in der Vorlage `ph:mountains`) für die Taste „Höhen“ und `topDown` (Phosphor `Drone`, `ph:drone`) für „Draufsicht“. Das Symbol `map` der Karte (Augenbraue im Inspector, Zeilen „Karte“ der Hierarchie) ist ebenfalls `ph:mountains`.

**Die Vorlage ist nicht gerendert.** `beispiele/hoehen-budget.body.js` ist gegen `jsx-lib-app.js` geschrieben und lief mit einer Attrappe der Bausteine (`Frame`, `Text`, `Icon`, `Ellipse`, `designVar`) in allen drei Modi, hell und dunkel, ohne Fehler durch (868, 375 und 254 Knoten); ein Bild davon gibt es nicht. Beim ersten Rendern sind Maße und Umbrüche zu prüfen und die Vorlage nach dem Bild und den Screenshots zu korrigieren. Wahrscheinliche Stellen:
- die Karte der Schicht: die Kacheln liegen gerade (36 × 28, gestaucht), nicht in der Schrägsicht der App; die Isolinien sind 2-px-Rahmen an der Kante der höheren Kachel;
- die Deckkraft der Flächen (`opacity` 0,42 am Rahmen): wenn OpenPencil sie nicht trägt, die Füllung als Hex mit Alpha setzen;
- der Balken des Budgets: die Füllung ist ein absoluter Rahmen in der Spur (mindestens 4 breit, damit 300 von 36.864 sichtbar bleibt; die App zeichnet den wahren Anteil);
- die Felder „Karte“ und „Höhe“ im Inspector stehen als Bezeichnung über dem Feld, wie in den Screenshots, nicht als Inspector-Zeile mit 116 px.

## Was neu zu zeichnen ist

Zwei Frames je 1440 × 900 und ein Brett, alle aus `beispiele/hoehen-budget.body.js` (`MODE` wählt). Diese Teile gibt es schon: Rahmen, Atlas, Statusleiste, Scharnier oben, die Liste „Welt“ (`WeltListe`, `TreeRowX`), Dock-Gruppen, die schwebende Werkzeugleiste, die Legende, Inspector-Kopf, `Section`, `Field`, `NumField`, `Badge`. Neu sind:
- die Ebenen der Schau mit vier Tasten in zwei Gruppen (`SceneBarX`: „Gelände“ `ph:stack`, „Höhen“ `ph:mountains`, „Ereignisse“ `ph:map-pin`, Trenner, „Draufsicht“ `ph:drone`; der alte Baustein `LayerBar` hat nur zwei);
- die Schicht „Höhen“ (Flächen in `hoehe/<Stufe>` mit 42 %, Isolinien in `hoehe/0`);
- der Baustein Budget (`BudgetX`) und der Abschnitt „Budget“ (`BudgetSection`).

1. **Frame „App: Welt bauen, Schicht Höhen“** (`MODE = 'hoehen'`).
   - **Liste „Welt“:** „Orte“ mit Sinnoh › Zweiblattdorf (gewählt, gezeigt), Route 201, Sandgemme; „In dieser Szene“ mit Zweiblattdorf › Karten › „Karte 0100“ (gewählt), Figuren (5), Türen (3).
   - **Scharnier:** „Zweiblattdorf › Karte 0100 Zelle 0, 0“, Bauen, Prüflicht „0 · 101 · 10“.
   - **Schau** (Reiter „3D“ und „Weltkarte“, Bauen ohne „Raster“): die Karte in drei Stufen, Grund Stufe 2 (`hoehe/2`), ein Hang Stufe 3 im Osten und ein Hügel im Südwesten (`hoehe/3`), darauf ein Plateau Stufe 4 (`hoehe/4`); Isolinien an jedem Stufenwechsel, an der Seite der höheren Kachel. Marker der Ereignisse (fünf Figuren, drei Türen). Oben links die Ebenen der Schau mit „Höhen“ und „Ereignisse“ gedrückt, „Gelände“ aus; die Werkzeugleiste links mit „Auswählen“, unten links „Legende“.
   - **Inspector:** „Karte 0100“, Kennung „Zelle 0, 0“, die Felder „Karte“ (100, „0 bis 665, leer = keine Karte“) und „Höhe“ (0, „0 bis 255“), der Abschnitt „Budget“ eingeklappt, darunter der Satz zu „Duplizieren“.
   - **Statusleiste:** „Auswählen · Klick wählt · Ziehen verschiebt“, in der Mitte „Kachel x 20, y 10 · Karte 0100 · begehbar · Höhe 4 Stufen“ (die Kachel unter dem Zeiger liegt auf dem Plateau), rechts „101 Warnungen, 10 Hinweise“, „Noch nicht gebaut“, „Komfort“.
2. **Frame „App: Inspector, Karte mit Budget“** (`MODE = 'budget'`).
   - **Liste „Welt“:** Sinnoh › Sandgemme › Haus 2 (gewählt, gezeigt); „In dieser Szene“ mit Haus 2 › Karten › „Karte 0188“ (gewählt), Figuren (2), Türen (2).
   - **Schau:** ein Raum als Platzhalter (Boden, Wand, zwei Figuren, eine Tür) mit dem Zellrahmen der gewählten Karte (`auswahl/halo`), die Ebenen der Schau mit „Gelände“ und „Ereignisse“ gedrückt.
   - **Inspector:** „Karte 0188“, „Zelle 0, 0“, „Karte“ (188), „Höhe“ (0) und „Budget“ offen: der Satz „Grenzen des Spiels; Speichern geht immer.“, dann „Modell“ Knapp („57.344 von 61.440 B“, Plakette „Knapp“, Balken `status/warnung`), „Terrain (BDHC)“ Ruhig („21.452 von 36.864 B“), „Gebäude“ Ruhig („16 von 32“), „Texturen des Gebiets“ Knapp („VRAM 112.896 von 262.144 B · 45 Texturen“, Hinweis „Vanilla höchstens 110.640 B“; Knapp, weil über dem Vanilla-Höchstwert, obwohl der Balken erst 43 % zeigt).
   - **Statusleiste:** in der Mitte „Sandgemme, Karte 0188 (0, 0)“.
3. **Brett „Komponenten (App): Budget und Höhenstufen“** (`MODE = 'brett'`), für die Seite „Komponenten“. Es zeigt:
   - das Budget in drei Zuständen, je mit dem Satz daneben: „Terrain (BDHC)“ Ruhig („300 von 36.864 B“), „Modell“ Knapp („58.000 von 61.440 B“), „Gebäude“ Voll („33 von 32“, Balken voll);
   - den Abschnitt „Budget“ in Bauen und in Tüfteln (die Grenzen hex: „0xF000“, „0x9000“, „0x40000“; „Gebäude“ ohne hex);
   - die Zeile „Texturen des Gebiets“ ohne Gebiet („Kein Header zeigt diese Karte.“) und den Abschnitt beim Lesen („Wird gelesen …“);
   - die sieben Höhenstufen („Stufe 0 · 0 E“ bis „Stufe 6 · 48 E“, je ein Farbfeld `hoehe/<n>`) und die Isolinien an einem Beispiel (ein 3 × 3-Block Stufe 2 in Stufe 0, zwölf Kantenstücke);
   - die Ebenen der Schau zweimal: mit „Gelände“ und mit „Höhen“ gedrückt, mit dem Satz zum Ausschluss und zu den Wörtern der Statusleiste.

## Lage und Einspielen

- **Lage:**
  - Die Frames 1 und 2 kommen auf die Seite „App: Werkbank“ (beide zeigen die Welt in der Anordnung „Werkbank“) in die nächste freie Reihe nach den Frames von 3f (Reihe 2) und allen Phasen, die bis dahin eingespielt sind; es gilt x = Spalte × 1520, y = Reihe × 1000, Frame 1 in Spalte 0, Frame 2 in Spalte 1.
  - Das Brett kommt auf die Seite „Komponenten“ unter das letzte Brett „Komponenten (App)“.
  - Vorher mit `get_page_tree` (depth 1) die belegten Positionen ablesen und nur eigene Frames anfassen. Die Datei liegt an der Grenze von 10.000 Knoten: dann lehnt OpenPencil Einzeländerungen (`update_node`, `set_text`, `set_layout`, `rename_node`) ab, neue Frames gehen nur über `render`, Korrekturen über `render` mit `replace_id`.
- **Bauen:**
  1. Aus `openpencil/beispiele/` `bash ../build-screen.sh hoehen-budget` aufrufen (für Dunkel mit `--dunkel`). Das Skript schreibt nur die `.jsx` neben die Vorlage und braucht OpenPencil nicht.
  2. In der erzeugten `hoehen-budget.jsx` `MODE` je Frame umstellen (`'hoehen'`, `'budget'`, `'brett'`).
  3. `node ../../scripts/op.mjs render '{"document_id":"tab-1","page_id":"<SEITE>","x":<x>,"y":<y>}' --jsx hoehen-budget.jsx` aufrufen (`<SEITE>` aus `list_pages`).
  4. Per `export_image` vergleichen (`node ../../scripts/op.mjs export_image '{"document_id":"tab-1","page_id":"<SEITE>","ids":["<ID>"],"maxEdge":1440}' --png hoehen-budget.png`), mit den Screenshots oben: Frame 1 mit `blick-karte100-hoehen-hell.png` (Ebenen der Schau, Flächen, Isolinien), Frame 2 mit `blick-karte188-budget-hell.png` und `blick-karte242-budget-hell.png` (Abschnitt, Plakette „Knapp“), das Brett mit der Galerie (`?gallery`, Seite „Welt“, Blöcke „Budget“ und „Höhenstufen“).
  5. Korrigieren: den Body ändern, neu bauen, `render` mit `"replace_id":"<ID>"` statt `x`/`y` (höchstens drei Läufe je Frame). Scheitert ein `render` mittendrin, bleibt ein halber Frame zurück: mit `get_page_tree` prüfen und löschen. Die erzeugten `.jsx` und `.png` nicht einchecken, sondern löschen.
- **Danach:**
  1. Speichern (`save_file`) und das Dokument schließen. Bei gesperrtem Bildschirm laufen die RPCs in Zeitüberschreitungen; dann warten, nicht wiederholen.
  2. Aus der Repo-Wurzel `node design-system/scripts/fix_fig_text.mjs design-system/Editor_DesignSystem.fig --rgb555` aufrufen.
  3. Im README den Abschnitt „Nachtrag 6.2“ um einen Satz ergänzen, dass die Frames seit … in der Datei stehen (mit Seite und Lage), und den Satz „Die Frames von 6.2 folgen, sobald OpenPencil antwortet“ dort, „Die Datei wurde nicht angefasst“ und „nicht gerendert“ hier ersetzen. In `Audit-Design-System.md` die Zeile „Budget“ (Komponenten) um den Verweis auf das Brett ergänzen.
  4. Im Brett „Stand der App“ (Seite „Titel“, x 1720, y 0, Vorlage `beispiele/stand-der-app.body.js`, per `render` mit `replace_id`) eine Zeile „Höhen und Budget (6.2)“ unter „Teilweise umgesetzt“ anlegen, mit dem Verweis auf die Frames.
  5. Im Brett „Komponenten (App)“ die Symbole `heights` und `topDown` in die Symbolreihe nehmen.

  `check-tokens.sh` ist nicht betroffen (keine neuen Tokens). Die Frames der Welt aus 3e und 3f zeigen zwei Tasten in den Ebenen der Schau und bleiben so: sie sind Bilder ihres Standes, nicht der heutigen App.

## Was nicht gezeichnet werden muss

- **Die Schicht „Gelände“ auf der BDHC-Höhe, Marker, Picking und Ziehen:** Sie sehen aus wie vorher; nur ihre Höhe folgt jetzt der Laufhöhe. Ein Bild davon ist `blick-karte100-figur-*.png`.
- **Draufsicht mit „Höhen“:** dieselbe Schicht senkrecht von oben, mit „Draufsicht“ gedrückt.
- **Tüfteln in der Statusleiste:** „Karte 0100 · Kachel x 20, y 10 · Typ 0x00 NONE · begehbar · Höhe 32 E (4 Stufen)“ ist dieselbe Statusleiste mit anderem Text; das Brett nennt den Satz.
- **„keine Höhe“** (eine Kachel ohne Platte oder mit unlesbarer BDHC) und die übrigen Sätze ohne Gebiet („Gebietsdaten 12 fehlen.“, „Texturpaket 6 nicht lesbar: <Grund>.“, „Gebäude-Texturpaket 6 nicht lesbar: <Grund>.“): derselbe Baustein mit anderem Satz.
- **Budget in Erzählen:** Der Abschnitt fehlt dort; ohne Karte in der Zelle ebenso.
- **Erzwungene Farben:** Spur mit Kante `CanvasText`, Füllung `Highlight`; die Datei kennt keine erzwungenen Farben.
- **Dunkel und Kompakt:** Die Bausteine haben ihre Zustände schon in der Datei; Frame 1 und 2 lassen sich mit `--dunkel` bauen, wenn ein dunkler Vergleich gewünscht ist (die Höhenfarben bleiben gleich, sie sind Szenenfarben).
