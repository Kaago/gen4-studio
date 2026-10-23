# Briefing: Screens für das Gen4-Studio-Design-System in OpenPencil

Du baust Editor-Screens für **Gen4 Studio**, eine neue, benutzerfreundliche Engine, mit der Menschen eigene Pokémon-Spiele für den Nintendo DS (Basis: Platin, HeartGold, SoulSilver) erschaffen. Ziel: vereinfachte Abläufe, aufgeräumte Assistenten, Einsteiger erzählen Geschichten, Profis tüfteln an Rohdaten. Die Tauri-2-App (React, three.js-Welt, CodeMirror/Monaco) setzt später exakt dieses Design-System um. Die Screens zeigen, wie **alle Features** aussehen sollen, nicht den heutigen Stand.

Das Design-System ist in OpenPencil schon angelegt (Tokens, Grundlagen, Komponenten, ein Referenz-Screen). Du ergänzt Screens auf **einer** Seite, die dir zugewiesen ist. Andere Agenten arbeiten gleichzeitig auf anderen Seiten desselben Dokuments: fasse fremde Seiten nicht an.

## Konzept „Doppeldeck“ (verbindlich)

- **Oben zeigen, unten anfassen.** Wie beim DS: Die **Schau** (oben) zeigt, was der Spieler sehen wird (3D-Welt, DS-Bildschirm-Vorschau, Graph, Profil). Die **Hand** (unten) hält Werkzeuge, Inspector, Formulare, Listen.
- Dazwischen das **Scharnier** (44 px, Graphit): links Brotkrumen (max. 3 Glieder, letztes fett), Mitte Tiefe (Erzählen | Bauen | Tüfteln), rechts Prüflicht, Rückgängig-Zähler, „Hier spielen ⌘R“ (Bernstein). Im **Assistenten** wird es zur Schrittleiste (Titel in Zen Maru, nummerierte Schritte, Abbrechen/Zurück/Weiter). Im **Spielmodus** ist es bernsteinfarben.
- Das Scharnier ist ziehbar: Die Hand darf je nach Aufgabe 200 bis 560 px hoch sein (Code-Editor: Hand groß, Schau klein).
- **Drei Tiefen**: Erzählen (Einsteigerwörter, Rezepte, große Tasten), Bauen (vollständiger Inspector, Malen, Graph), Tüfteln (Rohdaten, Hex, Code, IDs).
- **Zwei Sprachebenen**: vorne das Wort des Spieleerstellers, dahinter grau und in Mono die technische Kennung (z. B. „Haus des Rivalen“ + `T01R0102`).
- **Bernstein (#F7A510, live/grund) gehört allein dem Spielen.** Warnungen nutzen status/warnung (Ocker).
- **Nie nur Farbe**: Status immer mit Symbol oder Text; Overlays mit Muster/Form.
- Eine primäre (Kobalt) Taste pro Ansicht.

## Fensterraster (1440 × 900, wie der Referenz-Screen)

```
Titelleiste 40 (macOS-Overlay mit Ampel, Projekt, Suche ⌘K, Aktionen)
[Atlas-Leiste 88][optional Liste/Explorer 272][ Schau (wächst) ]
                                              [ Scharnier 44   ]
                                              [ Hand 286 (200–560) ]
Statusleiste 28 (aktives Werkzeug mit Kürzeln, Kontext, Build-Stand, Dichte)
```

Hand-Spalten werden durch 1-px-Linien (`linie/leise`) getrennt, nie durch Karten mit Schatten. Panels sind angedockt und flach. Nur Schwebendes (Menüs, Tooltips, Dialoge, Paletten, Overlays in der Schau) hat Schatten.

Referenz zum Ansehen: Seite „Screens: Start und Welt“, Frame „Welt bauen: Figur bearbeiten“ (per `find_nodes` mit name finden, per `export_image` ansehen, per `get_jsx` Struktur lesen).

## Werkzeuge und Regeln für OpenPencil (wichtig, sonst geht Arbeit verloren)

1. Dokument: die `document_id` aus `list_documents` (Datei `/Users/POLTEP/Projekte/lab/editor/Editor_DesignSystem.fig`). Hole die Seiten-ID per `list_documents` über den **Seitennamen**. Gib bei jedem Aufruf `document_id` und `page_id` an.
2. Baue **einen Screen pro `render`-Aufruf** als Frame `w={1440} h={900}` mit Namen „Bereich: Was passiert“ (z. B. „Pokédex: Staravia bearbeiten“). Lege Screens nebeneinander: x = n × 1520, y = 0 (zweite Reihe y = 1000).
3. **Nach jedem Screen `save_file` aufrufen.** Danach `export_image` (maxEdge 1440) und das Bild **selbst kritisch prüfen**: Überlappungen, abgeschnittener Text, ungleiche Ausrichtung, leere Flächen, Lesbarkeit. Korrigiere per `render` mit `replace_id` (ersetzt den Frame an seiner Stelle). Höchstens 2 Korrekturdurchläufe pro Screen; viele Löschen/Neu-Zyklen machen OpenPencil instabil.
4. Die erste Verwendung neuer Schriften/Symbole kann ein `RPC timeout` liefern; der Frame entsteht meist trotzdem. Prüfe mit `get_page_tree` (depth 1), bevor du neu renderst, und lösche halbe Frames.
5. Wenn eine Seite „not found“ ist oder IDs nicht mehr stimmen: Das Dokument wurde neu geladen. `list_documents` erneut, neue Seiten-ID per Namen, mit `get_page_tree depth 1` prüfen, was gespeichert ist, fehlende Screens neu bauen.
6. Wenn OpenPencil gar nicht antwortet (MCP-Fehler, Verbindung weg): App neu starten mit Bash `osascript -e 'quit app "OpenPencil"'; sleep 3; open -a OpenPencil /Users/POLTEP/Projekte/lab/editor/Editor_DesignSystem.fig`, 15 s warten (Bash: `sleep 15` ist erlaubt? Falls nicht, einfach einen harmlosen Aufruf wie `list_documents` wiederholen), dann weiter wie Punkt 5. Der MCP-Server ist Teil der App.
7. **Farben**: Flächen, Text, Linien immer über `designVar('token')` binden (Token-Liste unten). **Symbole (`Icon`) und SVG-Pfade bekommen Hex-Werte** (OpenPencil bindet Variablen an Vektoren nicht zuverlässig); nimm die Hex-Werte des hellen Themas aus `HX` in der Bibliothek.
8. Keine `Component`/`ComponentSet` in Screens anlegen (die Bibliothek existiert schon). Baue mit den JSX-Bausteinen.
9. Schriften: UI `Atkinson Hyperlegible Next` (Basis 13 px, nie unter 12 px), Titel/Ortsnamen `Zen Maru Gothic` (700, sparsam), Code/IDs `Atkinson Hyperlegible Mono`, Spieltext in DS-Vorschauen `Pixelify Sans`. Symbole: Phosphor (`ph:name`, `ph:name-fill` für Marker).
10. Gestrichelte Konturen immer mit `strokeDash={[a, b]} dashPattern={[a, b]}`; OpenPencil speichert nur `dashPattern`.
11. Nach dem Speichern das Dokument schließen und `node design-system/scripts/fix_fig_text.mjs Editor_DesignSystem.fig --rgb555` ausführen (repariert Text-Glyphen, rastet Farben auf RGB555).
12. Lange Texte in schmalen Zeilen: `T(text, { one: true })` kürzt einzeilig. Feste Breiten statt Umbruch-Chaos.

## JSX-Bausteinbibliothek

Datei: `design-system/openpencil/jsx-lib.js`. Lies sie einmal vollständig. Jeder `render`-Aufruf ist ein einzelner JSX-Ausdruck; kopiere die Bibliothek (oder den Teil, den du brauchst) in eine IIFE:

```jsx
{(() => {
  /* Inhalt von jsx-lib.js hier einfügen (nur benötigte Bausteine) */
  return Window({ name: 'Pokédex: Staravia bearbeiten', active: 'pokemon', status: Statusbar({ ... }),
    children: [ ListePanel, Deck({ schau: ..., scharnier: Scharnier({ ... }), hand: ..., handH: 286 }) ] });
})()}
```

Hinweis: `Window` erwartet `children` als Array oder Element; `Deck` liefert Schau/Scharnier/Hand. Für eigene Teile nutze `T`, `I`, `Btn`, `Ref`, `Badge`, `Typ`, `Field`, `Toggle`, `Check`, `Seg`, `Row`, `Section`, `PanelHead`, `Search`, `Thumb`, `ListRow`, `DSScreen`, `Kbd`, `IconBtn`.

## Tokens (Sammlung „Thema“, Modi Hell/Dunkel)

Flächen: `flaeche/gehaeuse` (App-Grund), `flaeche/deck` (Hand), `flaeche/taste` (Bedienflächen), `flaeche/blatt` (Eingaben, Schwebendes), `flaeche/schau` (Schau ohne Welt), `flaeche/welt` (3D-Hintergrund), `flaeche/bildschirm-rahmen`, `flaeche/bildschirm`, `flaeche/ueberlagerung`.
Text: `text/primaer`, `text/sekundaer`, `text/deaktiviert`, `text/invers`. Linien: `linie/leise`, `linie/standard`, `linie/stark`.
Akzent: `akzent/grund`, `akzent/hover`, `akzent/text`, `akzent/auf`; `auswahl/flaeche`, `auswahl/kante`, `fokus/ring`.
Live: `live/grund`, `live/auf`, `live/flaeche`, `live/text`. Scharnier: `scharnier/grund`, `scharnier/text`, `scharnier/leise`, `scharnier/kante`, `scharnier/taste`.
Status: `status/fehler|warnung|erfolg|info` und jeweils `-flaeche`.
Bereiche: `bereich/{welt|geschichten|pokemon|gegenstaende|texte|technik}` (Text), `-flaeche`, `-punkt`.
Primitive (nur wenn nötig): `platin/0…1000`, `kobalt/100…900`, `bernstein/…`, `signal/…`, `moos/…`, `zerr/…`, `koralle/…`, `lagune/…`, `sand/…`.
Szene (Hex, gleich in beiden Themen): Gelände blockiert `#D65A00` (Schraffur), Wasser `#0073B5` (Wellen), hohes Gras `#009C73` (Punkte), Vorsprung `#E79C00` (Pfeil), Eis `#52B5E7`, Spezial `#CE7BA5`; Ereignisse Figur `#4273AD` Kreis, Tür `#AD3173` Raute, Auslöser `#CEBD42` gestrichelte Fläche, Schild `#218C31` Etikett, Gegenstand `#63CEEF` rundes Quadrat; Höhen cividis `#00214A #31426B #636B73 #948C7B #C6B563 #E7CE5A #FFEF42`; Auswahl-Halo `#3152DE`.
Pokémon-Typen: Baustein `Typ('Feuer')` usw.

Dunkles Thema (nur falls dein Auftrag einen Dunkel-Screen verlangt; dann mit Hex statt designVar): gehaeuse `#212939`, deck `#293142`, taste `#31394A`, blatt `#394252`, schau/welt `#182131`, text primär `#E7EFF7`, sekundär `#ADB5C6`, linie leise `#31394A`, standard `#4A5263`, stark `#848CA5`, akzent `#6384F7` (Text darauf `#101821`), akzent/text `#94ADF7`, auswahl/flaeche `#29397B`, scharnier `#42527B` (Text `#E7EFF7`, leise `#B5BDCE`, taste `#5A6B94`), live `#F7A510`, Bereichsflächen dunkel: welt `#213929`/Text `#B5E7C6`, geschichten `#392952`/`#D6C6F7`, pokemon `#4A2921`/`#FFC6B5`, gegenstaende `#183939`/`#B5E7E7`, texte `#423921`/`#E7D6B5`, technik `#31394A`/`#CED6E7`; status warnung `#F7BD31` auf `#423110`, fehler `#F77B84` auf `#4A2129`, erfolg `#73D69C` auf `#213929`.

## Sprache und Inhalt

- Deutsch, Satzschreibung, du. Tasten sagen, was passiert („Trainer anlegen“, nicht „OK“). Meldungen: was passiert ist + Ausweg.
- **Verboten**: Versal-Labels (ALL CAPS), Mittelpunkt-Ketten „A · B · C“ (nimm Kommas oder getrennte Elemente), angehängte Pfeile „→“ an Tastentext, Emoji, Lorem ipsum, Pokéball-Rot-Weiß-Motive, Nintendo-Grafiken. Pokémon/Figuren als Platzhalter: `Thumb('pokemon', 'ph:bird')` o. ä. oder abstrakte Formen, keine nachgebauten Sprites.
- Echte deutsche Namen (Platin): Orte Zweiblattdorf, Sandgemme, Jubelstadt, Erzelingen, Flori, Ewigenau, Herzhofen, Route 201–230; Personen Prof. Eibe, Barry (Rivale), Lucas, Lucia, Arenaleiter Veit (Erzelingen, Gestein); Pokémon Chelast, Panflam, Plinfa, Staralili, Staravia, Staraptor, Bidiza, Sheinux, Knospi; Items Trank, Supertrank, Pokéball, Superball, Fluchtseil, Sinelbeere. Kennungen im Stil `SPECIES_STARLY`, `ITEM_POTION`, `FLAG_…`, `VAR_…`, Header-Kürzel `T01R0102`, IDs vierstellig.
- Zahlen realistisch (Basiswerte, Level, Preise in Pokédollar „₽“ ist ok, Begegnungsraten 20/20/10/10/10/10/5/5/4/4/1/1 %, als „1 von 5“ erklärt).

## Qualitätsmaßstab

Jeder Screen muss wie ein echtes, durchdachtes Produkt aussehen: klare Hierarchie, eine primäre Aktion, realistischer Inhalt, keine leeren Platzhalterflächen, keine abgeschnittenen Texte. Ein Screen zeigt einen konkreten Moment (z. B. „Schritt 2 von 4“, „Staravia gewählt, Werte geändert“), damit Zustände (Auswahl, ungespeichert, Abweichung, Hinweis) sichtbar werden. Der Screen muss die Prinzipien oben sichtbar machen.

## Abschluss

Wenn alle Screens stehen: `save_file`, dann antworte mit einer Liste „Screen-Name: was er zeigt“ (je 1 Satz), den Frame-IDs, und offenen Problemen. Halte die Antwort unter 400 Wörtern.
