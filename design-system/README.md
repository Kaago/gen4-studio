# Gen4 Studio Design-System „Doppeldeck“

Stand: 25.10.2026. Quelle der Gestaltung ist die OpenPencil-Datei [`Editor_DesignSystem.fig`](Editor_DesignSystem.fig). Quelle der Werte sind die DTCG-Tokens in [`tokens/`](tokens). Dieser Ordner liefert alles, was die Tauri-2-App (React, Dockview, three.js, CodeMirror/Monaco) zum Umsetzen braucht.

Gen4 Studio ist eine Engine, mit der Menschen eigene Pokémon-Spiele für den Nintendo DS erschaffen (Basis Platin, HeartGold, SoulSilver). Das Design-System richtet sich an zwei Gruppen zugleich: Einsteiger, die Orte, Figuren und Geschichten erzählen wollen, und Profis, die an Skripten und Rohdaten tüfteln. Es beschreibt den Zielzustand mit allen Features, nicht den heutigen Stand der App.

## Inhalt

| Pfad | Inhalt |
|---|---|
| `Editor_DesignSystem.fig` | OpenPencil-Datei: Titel (mit „Stand der App“), Grundlagen, Komponenten, Muster, vier Seiten „Zielbild: …“ und drei Seiten „App: …“ |
| `Audit-Design-System.md` | Abweichungen der App von der initialen Idee, mit Einordnung (bewusst, Lücke, Verbesserung) und offenen Entscheidungen |
| `tokens/` | Tokens im W3C-DTCG-Format, eine Datei je Sammlung und Modus (wie OpenPencil/Figma sie exportiert) |
| `apps/studio/src/ui/tokens/tokens.css` | Alle Tokens als CSS-Variablen `--g4-*`, Hell als Standard, Dunkel über `data-theme`, Kompakt über `data-dichte` |
| `apps/studio/src/ui/tokens/dockview-theme.css` | Dockview-Thema `dockview-theme-g4`, gespeist aus den Tokens |
| `apps/studio/src/ui/tokens/tokens.ts` | Hex-Werte für three.js, Monaco/CodeMirror und Canvas: Szene, Themen, Syntax, Pokémon-Typen (`typen`, aus der Szene-Gruppe `typ`), `rgb555()` |
| `scripts/export_tokens.py` | Erzeugt die drei Dateien oben aus `tokens/`; Aufruf über `pnpm tokens` (in `apps/studio`), Ausgabeordner als zweiter Parameter |
| `scripts/check_tokens.py` | Prüft RGB555-Treue und WCAG-Kontraste aller wichtigen Paare in beiden Themen |
| `scripts/fix_fig_text.mjs` | Repariert nach dem Speichern in OpenPencil verstümmelte Texte und rastet mit `--rgb555` alle Farben der `.fig` auf das DS-Raster |
| `openpencil/jsx-lib.js` | JSX-Bausteine des Zielbilds (Fenster, Atlas, Scharnier, Statusleiste, Inspector, DS-Bildschirm …), mit denen die Zielbild-Screens gebaut sind |
| `openpencil/BRIEFING-SCREENS.md` | Briefing für neue Zielbild-Screens: Konzept, Raster, Regeln, Tokens, Sprache |
| `openpencil/jsx-lib-app.js` | JSX-Bausteine mit den Maßen der App (CSS-Module in `apps/studio/src/ui`), für die Seiten „App: …“ und die Bretter „Komponenten (App)“; `dunkel-map.js` davor liefert das dunkle Thema als Hex |
| `openpencil/BRIEFING-APP-SCREENS.md` | Briefing zum Nachzeichnen der App aus Screenshots der Test-Umgebung; Beispiel in `openpencil/beispiele/` |
| `openpencil/build-screen.sh`, `scripts/op.mjs` | Baut aus Bibliothek und Screen-Datei ein JSX und ruft damit den OpenPencil-MCP-Server direkt auf (`render`, `export_image` mit `--png` …), ohne riesige Aufrufe |

Ablauf bei Änderungen: Token in `tokens/*.json` ändern, `python3 design-system/scripts/check_tokens.py`, dann `pnpm tokens` (in `apps/studio`, schreibt die drei generierten Dateien nach `apps/studio/src/ui/tokens/`), dann `bash scripts/check-tokens.sh` aus der Repo-Wurzel (Raster, Kontraste und Abgleich der eingecheckten Dateien mit einem frischen Export; läuft auch im Gate), dann die geänderten Dateien in OpenPencil über „Design-Tokens importieren“ (MCP `import_design_tokens`) einspielen. OpenPencil legt fehlende Variablen an und aktualisiert vorhandene über den Namen.

## Prinzipien

1. **Zeigen und anfassen.** Die *Schau* zeigt, was der Spieler sehen wird: 3D-Welt, DS-Bildschirm, Story-Graph, Pokédex-Profil. Die *Hand* hält, was man anfasst: Hierarchie, Inspector, Formulare, Listen, und als schwebende Leiste die Werkzeuge auf der Schau. Beide spiegeln dieselbe Auswahl. In der Welt steht die Hand an den Rändern (links die Hierarchie, rechts der Inspector, unten Script und Probleme); in Texten und Daten liegt sie unter der Schau wie beim DS. Jeder darf die Hand umordnen; die Schau bleibt eine Gruppe in der Mitte.
2. **Das Scharnier weiß, wo du bist.** Eine dunkle Leiste über der Schau trägt Ort und Auswahl, die Tiefe, das Prüflicht, den Verlauf und „Hier spielen“; darunter die Reiter der Schau. Im Assistenten wird sie zur Schrittleiste, im Spielmodus bernsteinfarben. Das ist die eine Signatur des Systems; alles andere bleibt ruhig.
3. **Erzählen, Bauen, Tüfteln.** Statt eines Expertenmodus hat jeder Bereich eine Tiefe. Erzählen nutzt Rezepte und Wörter wie Ort, Figur, Geschichte. Bauen zeigt den vollständigen Inspector, Malen und Graph. Tüfteln zeigt Skripte, Hex-Werte, Header und Matrizen.
4. **Zwei Sprachebenen.** Vorne steht das Wort des Spieleerstellers, dahinter grau und in Mono die technische Kennung: „Haus des Rivalen `T01R0102`“. So lernt man Gen-4-Begriffe nebenbei.
5. **DS-treue Farbe.** Jede Farbe liegt im 15-Bit-Raster des DS (RGB555, Kanalwerte `(v << 3) | (v >> 2)`). Was der Editor zeigt, kann das Spiel darstellen.
6. **Nie nur Farbe.** Status trägt Symbol und Text, Szenen-Overlays tragen Muster und Form, Pokémon-Typen tragen ihren Namen.
7. **Rezepte statt leerer Leinwand, Wege statt Nummern.** Assistenten erzeugen normale, weiter bearbeitbare Objekte. Wo das Spiel eine ID erwartet, wählt man per Picker mit Namen und Vorschau. Jede Referenz ist ein klickbarer Weg mit „Verwendet in“.
8. **Reversibel sichtbar.** Alles ist mit ⌘Z umkehrbar, außer dem, was sofort auf Platte geht (neue Karten, Patches). Das trägt die Plakette „Nicht rückgängig“, und vorher entsteht ein Schnappschuss.

## Fensterraster

```
┌ Titelleiste 40: Ampel oder Fensterknöpfe, Projekt mit Spiel und Speicherstand, Suche ⌘K, Speichern, Bauen, Veröffentlichen, Thema, Hilfe
├ Atlas 88 ┬ Liste/Explorer 272 (optional) ┬ Scharnier 44 (über der Schau, nicht ziehbar)
│          │                               ├ Schau (wächst); in 3D schwebt die Werkzeugleiste
│          │                               ├ Teiler 8 (ziehbar, Tasten)
│          │                               └ Hand 286 (200 bis 560)
└ Statusleiste 28: aktives Werkzeug mit Kürzeln, Kontext, Build-Stand, Dichte
```

- Referenzgröße 1440 × 900. Mindestgröße des Fensters 1100 × 720 (Windows: `minWidth` höchstens 500 halten, sonst bricht Snap; die App selbst skaliert ab 1100 sinnvoll).
- In der Welt hat die Liste zwei Abschnitte unter dem Kopf „Welt“: oben „Orte“ (der Weltbaum, auf eine Zeile einklappbar), unten „In dieser Szene“ (die Hierarchie des gezeigten Gebiets oder Raums), dazwischen ein waagrechter Teiler „Höhe der Orte“ (Anteil 0,2 bis 0,8 der Höhe, jeder Teil mindestens 96 px). Die Listen der anderen Bereiche haben diese Teilung nicht.
- Hand-Spalten werden durch 1-px-Linien getrennt (`linie/leise`), nicht durch Karten. Angedockte Flächen haben keinen Schatten und keine Rundung.
- Der Atlas hat Start und sechs Bereiche: Welt, Geschichten, Pokémon, Gegenstände, Texte, Technik. Die Bereichsfarbe färbt Atlas-Eintrag, Referenz-Chips, Assistenten-Kopf und Story-Knoten.
- Dockview: Schau-Gruppe und Hand-Gruppe sind zwei Dockview-Gruppen; das Scharnier ist eine eigene React-Leiste über der Schau und der Teiler ein eigener React-Trenner zwischen Schau und Hand, beide kein Panel. Profis dürfen Panels umdocken. Mit 3e.4 bekommt jeder Bereich ein Dock statt zweier: Hand und Schau werden Gruppen darin, das Scharnier bleibt die Kopfleiste darüber.

## Tokens

Sammlungen (in OpenPencil und `tokens/`):

| Sammlung | Modi | Inhalt |
|---|---|---|
| Primitiv | Wert | Rampen Platin (Neutrale), Kobalt, Bernstein, Signal, Moos, Zerr, Koralle, Lagune, Sand |
| Szene | Wert | Gelände-, Ereignis-, Höhen- und Auswahlfarben für three.js, Gruppe `ds` für den DS-Bildschirm, Gruppe `typ` für die 18 Pokémon-Typen, Gruppe `status` für Prüfbefunde an Markern; gleich in beiden Themen |
| Thema | Hell, Dunkel | Semantik: `flaeche/*`, `text/*`, `linie/*`, `akzent/*`, `auswahl/*`, `fokus/ring`, `live/*`, `scharnier/*`, `status/*`, `bereich/*`, `schatten/*` (Typ `shadow`: `schatten/schwebend`, Platin 1000 mit 24 % in Hell, 48 % in Dunkel; als CSS `--g4-schatten-schwebend`, nicht in `tokens.ts`) |
| Raum | Wert | `raum/0…64` (Pixel im Namen), `radius/feld|taste|bildschirm|fenster|plakette`, `strich/*`, `schrift/groesse/*`, `schrift/zeile/*` |
| Dichte | Komfort, Kompakt | `dichte/steuer`, `zeile`, `zeile-innen`, `panel-innen`, `feld-abstand`, `scharnier`, `symbol`, `werkzeug` |
| Bewegung | Wert | `dauer/sofort|kurz|mittel|scharnier` (0/120/200/280 ms) |
| Schrift | Wert | `familie/ui|titel|code|spiel` |
| Textstile | – | `text/xs…l`, `titel/s…l`, `code/s`, `spiel/textbox` |

Regeln:
- Komponenten benutzen nur Thema-, Raum- und Dichte-Tokens, nie Primitive oder Hex.
- **Bernstein 500 gehört allein dem Spielen** (Spielen-Taste, Spielmodus, Live-Werte). Warnungen benutzen `status/warnung` (Bernstein 750) und ein Symbol.
- Wer schwebt, hat Schatten (`schatten/schwebend`, Rand `linie/standard`); wer andockt, nie: Menü, Tooltip, Dialog, Popover, Toast, Szenenleiste und Werkzeugleiste.
- Kobalt heißt: das bist du gerade (Auswahl, Fokus) oder das passiert, wenn du klickst (primäre Taste). Höchstens eine primäre Taste pro Ansicht.
- Kontraste: Text und Status auf allen Flächen mindestens 4,5 : 1, Steuerkanten (`linie/stark`) und Fokusring mindestens 3 : 1. `scripts/check_tokens.py` prüft das für beide Themen.
- Dark Mode folgt dem System, außer `data-theme="hell"` oder `data-theme="dunkel"` ist gesetzt. Kompakt über `data-dichte="kompakt"` am `html`-Element. Bei `prefers-reduced-motion` werden alle Dauern 0 ms (Überblendung statt Bewegung). Bei `forced-colors` übernehmen Systemfarben Fokus und Akzent.

## Typografie

| Rolle | Schrift | Einsatz |
|---|---|---|
| Oberfläche | Atkinson Hyperlegible Next | Alles, was man liest und bedient. Basis 13/18, nie unter 12 px. Eindeutige Formen für I, l, 1, 0. Tabellenziffern. |
| Titel | Zen Maru Gothic (700/900) | Ortsnamen, Assistententitel, Kopf von Editoren und leeren Zuständen. Höchstens ein großer Titel pro Ansicht. |
| Code und Kennungen | Atkinson Hyperlegible Mono | Skripte, Hex-Werte, technische Namen hinter dem Klartext. |
| Stimme des Spiels | Pixelify Sans | Nur in DS-Bildschirm-Vorschauen (Textbox, Schild, Menü). |

Skala: 12/16, 13/18, 15/20, 18/24, 24/30, 32/38, 44/52. Alle Schriften stehen unter der SIL Open Font License und werden mit der App ausgeliefert (CSP erlaubt nur `self`): `@fontsource-variable/atkinson-hyperlegible-next`, `@fontsource/atkinson-hyperlegible-mono`, `@fontsource/zen-maru-gothic` (700, 900; nur Latin-Teilmenge), `@fontsource/pixelify-sans`. Auf Windows statische, gehintete Schnitte bevorzugen.

Symbole: Phosphor Icons (MIT), Stil „regular“ für Oberfläche, „fill“ für Marker in der Welt. In der App erben Symbole `currentColor`.

## Komponenten (Seite „Komponenten“)

| Komponente | Varianten und Eigenschaften | Umsetzung |
|---|---|---|
| Taste | Art Primär, Sekundär, Leise, Spielen, Gefahr × Zustand Ruhe, Hover, Fokus, Gesperrt; Beschriftung, Symbol | `<Taste art>`; sekundär mit doppelter Unterkante; Fokus 2-px-Ring außen |
| Symboltaste | Ruhe, Hover, Aktiv, Gesperrt | immer mit Tooltip (Name + Kürzel) |
| Tastenkürzel | Hell, Auf Scharnier | in Tooltip, Menü, Statusleiste, Picker-Fuß |
| Eingabefeld | Ruhe, Fokus, Fehler, Gesperrt; Bezeichnung, Wert, Hinweis | Fehlertext sagt den Ausweg |
| Zahlfeld | Dez, Hex | Scrubbing an der Bezeichnung (Pointer Capture), Bitbreite prüfen, nie still abschneiden |
| Auswahlfeld | Ruhe, Offen | kurze feste Listen mit Symbol; lange Listen über Picker |
| Schalter, Kästchen | An/Aus, An/Aus/Gemischt | Schalter wirkt sofort |
| Segmente | Erzählen, Bauen, Tüfteln | zwei bis vier gleichrangige Optionen |
| Reiter | Aktiv, Ruhe, Ungespeichert | Kobalt-Punkt statt Schließen bei Ungespeichertem |
| Referenz | Bereich Welt … Technik; Name | klickbar, Rechtsklick „Verwendet in“ |
| Zustand | Ungespeichert, Nicht rückgängig, Gesperrt, Auf Platte geändert, Nur Platin und HGSS, Fehler, Live | überall, wo das Asset erscheint |
| Typ | 18 Typen der 4. Generation | Farbe plus Name; Farben aus `szene/typ/<schluessel>/{grund,schrift}`, unbekannte Nummer als „Typ 19“ auf `flaeche/deck` |
| Meldung | Info, Erfolg, Warnung, Fehler; Titel, Text, Aktion | im Panel, im Assistenten oder als Toast unten rechts |
| Wertebalken | Über Schnitt, Unter Schnitt | Wert bis 255, Strich für den Durchschnitt |
| Budget | Ruhig, Knapp, Voll | DS-Grenzen ruhig zeigen, Speichern blockiert nie |
| Listenzeile, Baumzeile | Ruhe, Hover, Gewählt, Gezeigt, Befund, Gesperrt / im Ortsbaum: Weltkarte, Gebiet, Raum, Gruppe („Weitere Gebiete“, „Nicht verbunden“, „Sonstiges“); in der Szene: Gebiet oder Raum als Wurzel, Karte, Gebäude, Figur, Tür mit Ziel (zweite Zeile „→ Ziel“), Auslöser, Schild, Script, Gruppe mit Zähler | Name vorne, Kennung grau (in Bauen der Klarname, in Tüfteln dazu Nummer und Kürzel); ein Raum eines anderen Ortes trägt das Ortsetikett: 8-px-Punkt in der Ortsfarbe plus Ortsname grau, nie Farbe allein; rechts Zähler (Gebiete, Räume) und am gewählten Raum das Menü „Weitere Aktionen für …“. Ein Gebiet ohne Räume (und in Tüfteln ohne Abschnitte) ist ein Blatt, ohne Pfeil. **Gezeigt:** die Zeile, deren Raster die Schau zeigt, trägt am Ende ein Symbol mit Namen (`ShownMark`), nicht Kobalt. **Gewählt:** Kobalt, wie überall. **Befund:** ein Zeichen mit Namen am Zeilenende (Fehler vor Warnung); an der Wurzel der Szene der schlimmste Befund des Headers, an den Ereigniszeilen der Fehler der Prüfungen. **Gesperrt:** die Plakette „Gesperrt“ an den vier Ereignisgruppen, solange die Eventdatei schreibgeschützt ist. Die Szenenzeilen tragen das Symbol ihrer Art (Karte, Gebäude, Figur, Tür, Auslöser, Schild, Script) und grau, wo das Ding steht („x 11, z 21“; die Karte „Karte 0411 · Zelle 0, 0“); die Tür hat zwei Zeilen: „Warp 0“ und darunter „→ Haus 1“ (unbekanntes Ziel: „→ unbekanntes Ziel 0999“, in Tüfteln „→ Header 0414 · Haus 1“). Die Gruppe ist ein Wort mit dem Zähler rechts, keine Versalien |
| Hierarchie | Wurzel Gebiet oder Raum, Gruppen Karten, Figuren, Türen, Auslöser, Schilder, Scripts; Ruhe, Suche, Leer, Eventdatei gesperrt | der Abschnitt „In dieser Szene“ der Liste „Welt“: Panelkopf mit „Zurück“ (⌥⌘←), „Vor“ (⌥⌘→) und dem Menü „Hinzufügen“ („+“ wählt das Werkzeug Figur, Tür, Auslöser, Schild oder Gebäude, dazu „Raum hier anbinden…“; die Ereignisse gesperrt mit Grund, wenn auch die Werkzeuge es sind), darunter das Suchfeld „Name oder Nummer suchen“ und der Baum (`role="tree"`, Name „In dieser Szene“, Roving, Pfeil rechts und links öffnen und schließen). Alles ist offen, nur die Gebäude einer Karte sind zu. Eine Gruppe ohne Zeilen fehlt. Ein Klick wählt wie ein Klick in der Szene, ein Script öffnet seinen Reiter, Enter oder Doppelklick auf eine Tür mit bekanntem Ziel zeigt die Szene des Ziels. Die Suche behält Zeilen, deren Titel oder Kennung (auch das Ziel einer Tür) passt, und lässt Gruppen und Karten mit Treffern offen; das Gruppenwort behält alle Zeilen der Gruppe. Eine gesperrte Eventdatei steht als Meldung „Eventdatei gesperrt“ unter der Suche. Ohne gezeigte Szene der leere Zustand „Noch keine Szene gezeigt.“ |
| Inspector-Kopf | Art Figur, Tür, Auslöser, Schild, Gebäude, Karte, Gebiet, Raum, Ort, Aussehen × Zustand Ungespeichert, Gesperrt, Nicht rückgängig; mit und ohne Menü | der Kopf, den jede Ansicht des Inspectors teilt (`InspectorHead`): links die Augenbraue, das Typ-Wort in Satzschreibung mit Symbol davor (`text/xs`, `text/sekundaer`, keine Versalien; in Tüfteln „Header“ für Gebiet, Raum und Ort, „Area-Data“ für das Aussehen), der Titel (`titel/s`, Zen Maru Gothic, eine Zeile) und grau in Mono die Kennung („Objekt 2 · Eventdatei 0390“, `code/s`); rechts die Plaketten der Zustände und das Menü „Aktionen“ (Symbol „Mehr“, am rechten Ende; ohne Einträge fehlt es). Die Einträge heißen nach dem, was passiert: Karte „Teile importieren…“, „Teile exportieren…“, „Duplizieren“; Figur, Tür, Auslöser, Schild und Gebäude „Löschen“ mit Entf (gesperrt bei schreibgeschützter Eventdatei); Gebiet und Raum „Raum hier anbinden…“. Der Rand unten ist eine 1-px-Linie `linie/leise`, der Kopf schwebt nicht |
| Inspector-Zeile | Text, Auswahl, Zahl, Schalter, Referenz, Flags; Abweichung | Bezeichnung 116 px, Kobalt-Strich bei Abweichung von der Vorlage |
| Scharnier | Bearbeiten, Assistent, Spielen; Lage oben, ohne Griff, nicht ziehbar | eigene React-Komponente, 44 px über der Schau, Übergang 280 ms; Tokens `scharnier/*` |
| Werkzeugleiste, schwebend | Ausrichtung senkrecht, waagrecht × Dichte Komfort, Kompakt; Anker Links, Oben, Unten; Werkzeug Ruhe, Hover, Aktiv, Gesperrt; Gruppen-Trenner | Overlay der 3D-Ansicht, `flaeche/blatt`, Rand `linie/standard`, `schatten/schwebend`, `radius/taste`; Taste `dichte/werkzeug` (40/32); `role="toolbar"` mit Roving; Griff (nur Zeiger) und Menü „Lage der Werkzeugleiste“; ab 460 px (Kompakt 400 px) Höhe der Ansicht senkrecht, darunter oben |
| Teiler | waagrecht, senkrecht; Ruhe, Hover, Fokus | `role="separator"` mit `aria-orientation`, 8 px, Linie `linie/leise`, `aria-valuenow` in px. Ziehen beginnt erst nach 4 px (kürzere Wege sind ein Klick, kein Zug); Pfeiltasten ±16 px entlang seiner Achse (waagrecht hoch und runter, senkrecht links und rechts; die Taste in Wachstumsrichtung des ersten Teils vergrößert ihn), Pos1 den Höchstwert, Ende den Mindestwert; Enter und Doppelklick bestimmt der Besitzer. Zwischen Schau und Hand: Höhe der Hand 200 bis 560 px, Enter oder Doppelklick klappt sie ein. In der Liste „Welt“ („Höhe der Orte“): der Anteil der Orte 0,2 bis 0,8 der Höhe (gemerkt in `g4.shell.v1`), jeder Teil mindestens 96 px, Doppelklick halbiert |
| Titelleiste | macOS, Windows | macOS `titleBarStyle: "Overlay"` + natives Menü; Windows `decorations: false`, eigenes Menü und Fensterknöpfe |
| Atlas-Eintrag | 7 Bereiche × Ruhe/Aktiv | 88 px breite Leiste |
| Werkzeug-Taste, Rezept-Taste | Ruhe, Hover, Aktiv, Gesperrt / 4 Bereiche | Werkzeuge in Bauen: Symbol mit dem Buchstaben in der Ecke, ohne Namen unter dem Symbol; Name im Tooltip und in der Statusleiste; `data-tool`. Rezepte in Erzählen |
| Statusleiste | – | lehrt Kürzel des aktiven Werkzeugs |
| DS-Bildschirm | Textbox, Schild, Leer | 256 × 192, ganzzahlig skaliert, Spielschrift |
| Weltmarker | 6 Arten × Ruhe/Gewählt | Billboards in three.js |
| Pokémon-Karte | Ruhe, Gewählt, Leer | Team-Platz im Trainer-Editor |
| Story-Knoten | Start, Dialog, Bedingung, Kampf, Belohnung | Graph-Ansicht (React Flow), Liste bleibt Standard |
| Menü, Tooltip, Picker, Leerer Zustand, Dialog | – | nur Schwebendes hat Schatten |

Für die Umsetzung eignen sich Radix- oder Reka-Primitives (Dialog, Popover, Tabs, Switch, Checkbox, Toggle Group) als Verhaltensschicht unter den eigenen Komponenten; Tastatur und ARIA kommen von dort.

## Muster

- **Assistent:** Das Scharnier zeigt nummerierte Schritte, die Schau zeigt live, was entsteht, die Hand stellt eine Frage mit großen Auswahlkarten. Vor dem Anlegen steht eine Zusammenfassung in einem Satz. Danach ein Ergebnis mit „Zeigen“, „Hier spielen“ und dem nächsten sinnvollen Schritt. Enter führt weiter, Esc bricht ab und stellt den Schnappschuss wieder her. Assistenten erzeugen nur normale Objekte.
- **Picker:** ein Baustein für alle Referenzen (Pokémon, Items, Orte, Flags, Texte, Sprites, Musik). Suche filtert sofort, Bereichsfilter als Plakette, Treffer mit Vorschau, Kennung und Zähler, Fuß mit Tasten.
- **Hierarchie und Inspector:** Die Auswahl hat einen Ort. Ein Klick in der Szene, auf einer Zeile von „In dieser Szene“, auf einer Zelle des Rasters oder auf einem Ort wählt dasselbe, und die Zeile der Hierarchie zeigt es in Kobalt. Der Inspector zeigt nur die Auswahl, mit seinem Kopf: Art, Name, Kennung, Zustände und Aktionen. Was unter dem Zeiger liegt (Kachel, Marker, Tür), steht in der Mitte der Statusleiste und verdrängt nie die Auswahl. „Zurück“ und „Vor“ (⌥⌘←, ⌥⌘→) gehen durch die gezeigten Szenen; eine Tür mit Ziel folgt man mit Doppelklick oder Enter, vom Header aus führt die Zeile „2 Türen, 1 ohne Ziel“ in die Gruppe „Türen“. Etwas hinzufügen heißt Werkzeug (Buchstabe) oder „+“ der Hierarchie; was man mit dem Gewählten tut, steht im Menü „Aktionen“ des Kopfs.
- **Referenzen:** jede Entität hat „Verwendet in“; ein Klick öffnet das Ziel, Türen lassen sich in der Welt per Doppelklick durchqueren.
- **Meldungen:** dort, wo das Problem ist. Prüfungen sammeln sich im Prüflicht des Scharniers und im Panel „Prüfen und Bauen“. Keine modalen Erfolgsmeldungen.
- **Spielmodus:** „Hier spielen“ baut nur geänderte Schritte, startet den Emulator am Cursor und färbt das Scharnier bernsteinfarben. Flags und Variablen sind live änderbar, Haltepunkte sitzen an Ereignissen.
- **Werkzeuge:** Der Buchstabe wählt das Werkzeug, Esc wählt wieder „Auswählen“. Die Buchstaben wirken nur, solange die Ansicht sichtbar ist, die die Leiste trägt (die 3D-Ansicht); sonst sagt ein Hinweis „Werkzeuge wirken in der 3D-Ansicht“.
- **Leere Zustände:** ein Satz, was fehlt, ein Satz, wozu, eine Taste, die es tut.

## Szene (three.js)

- Farben aus `apps/studio/src/ui/tokens/tokens.ts` (`szene`), Material `toneMapped: false`, `depthWrite: false`, `polygonOffset`. Füllung 42 % Deckkraft, opake Kontur mit dunklem Halo.
- Gelände: blockiert diagonal schraffiert, Wasser Wellen, hohes Gras Punkte, Vorsprung Pfeil in Sprungrichtung, Eis Kreuzschraffur, Spezial Schachbrett, unbekannt gestrichelt. Muster hängen an Kachelkoordinaten (`gelaendeMuster`).
- Ereignisse: Figur Kreis, Trainer Kreis mit Sichtlinie, Tür Raute, Auslöser gestrichelte Fläche, Schild Etikett, Gegenstand rundes Quadrat. Auswahl mit 3-px-Kobalt-Halo.
- DS-Bildschirm (Gruppe `ds`, Spielinhalt, gleich in beiden Themen): `ds/grund` ist die Spielfläche hinter der Box, `ds/textbox/*` und `ds/schild/*` (je `flaeche`, `rahmen`, `text`) sind die Textbox und die Schildbox des Spiels, `ds/pfeil` der Weiter-Pfeil. In CSS `--g4-szene-ds-textbox-flaeche` usw., in `tokens.ts` als `szene.ds['textbox/flaeche']`. Text auf Fläche hat mindestens 4,5:1 (`check_tokens.py`).
- Pokémon-Typen (Gruppe `typ`, Spielfarben, gleich in beiden Themen): je Typ `grund` und `schrift`, Schlüssel in der Reihenfolge der Spieldaten (`normal`, `kampf`, `flug`, `gift`, `boden`, `gestein`, `kaefer`, `geist`, `stahl`, `fragezeichen` für den ???-Typ, `feuer`, `wasser`, `pflanze`, `elektro`, `psycho`, `eis`, `drache`, `unlicht`). In CSS `--g4-szene-typ-feuer-grund` usw., in `tokens.ts` als `typen.feuer`. Schrift auf Grund hat mindestens 4,5:1 (`check_tokens.py`).
- Status (Gruppe `status`, gleich in beiden Themen, seit 3b.4a): `status/fehler` (`#DE3142`, aus Primitiv `signal/500`) färbt einen Warp-Marker, an dem eine Prüfung einen Fehler meldet; die übrigen Marker behalten ihre Farbe. In CSS `--g4-szene-status-fehler`, in `tokens.ts` als `szene.status.fehler`.
- Höhen: sieben Stufen cividis, eine Stufe ist eine halbe Kachel, Isolinien bei Stufenwechsel.
- Standardkamera ist die Gen-4-Kamera; „Draufsicht“ schaltet orthografisch zum präzisen Malen.

## Barrierefreiheit

WCAG 2.2 AA als Untergrenze: Kontraste wie oben, Ziele mindestens 24 px (Komfort 32 px), sichtbarer Fokusring nur bei Tastaturbedienung (`:focus-visible`), Roving Tabindex in Listen, Bäumen und Werkzeugen, Kachel-Cursor mit Pfeiltasten in der Welt, `lang="de"`, reduzierte Bewegung und erzwungene Farben berücksichtigt, kein Inhalt nur über Farbe oder nur über Tooltip.

## Screens (Seiten „Zielbild: …“ und „App: …“)

Jeder Screen ist 1440 × 900 und zeigt einen konkreten Moment mit echten Zuständen (Auswahl, ungespeichert, Abweichung, Hinweis).

Seit dem 25.10.2026 hat die Datei zwei Arten von Screen-Seiten:
- **Zielbild:** Die 30 Screens vom 23.10. zeigen die Idee mit allen Features. Über jedem steht eine Marke „Umgesetzt“, „Teilweise umgesetzt“ oder „Zielbild, noch offen“ mit dem Verweis auf den App-Screen und der wichtigsten Abweichung.
- **App:** „App: Start und Welt“, „App: Pokémon und Gegenstände“ und „App: Texte, Technik, Dialoge“ zeichnen die App so nach, wie sie nach Phase 3 aussieht. Vorlage waren Screenshots der Test-Umgebung (Platin DE); Projektname „Sinnoh-Remix“.

Die Übersicht steht auf der Titelseite im Brett „Stand der App“. Abweichungen im Einzelnen stehen in [`Audit-Design-System.md`](Audit-Design-System.md).

Zielbild-Seiten:

| Seite | Screens |
|---|---|
| Start und Welt | Welt bauen: Figur bearbeiten (Referenz), Start: Willkommen zurück, Neues Spiel: Basis-ROM wählen, Weltkarte: Region Sinnoh, Ort anlegen: Anordnung, Welt erzählen: Tür verbinden, Gelände malen: Route 201, Raster: Matrix 0 (Tüfteln) |
| Figuren und Geschichten | Neue Figur: Bewegung, Geschichten: Übersicht (Fäden), Geschichte: Graph der Hauptgeschichte, Skript: Mutter, Morgen (Tüfteln), Flags und Variablen, Dialog bearbeiten: Mutter, Morgen |
| Pokémon und Kämpfe | Pokédex: Staravia bearbeiten, Entwicklung und Lernset, Trainer: Wanderer Kai, Begegnungen: Route 201, Attacken: Flügelschlag, Neuer Trainer: Team |
| Gegenstände, Texte, Testen | Gegenstände: Trank, Läden: Pokémarkt Jubelstadt, Texte: Archiv 0412, Spielen: Zweiblattdorf live, Prüfen und Bauen, Veröffentlichen: Patch erstellen, Technik: Patches und Plugins, Einstellungen, Befehlspalette über der Welt, Welt bauen im dunklen Thema |

## Was die App umsetzt

Stand nach Phase 3c (3c.1 und 3c.2; Spezifikation: [docs/superpowers/specs/2026-10-23-phase-3c-doppeldeck-design.md](../docs/superpowers/specs/2026-10-23-phase-3c-doppeldeck-design.md), Gate: [docs/phase-3c-gate.md](../docs/phase-3c-gate.md)), Phase 3b.3a (Daten-Editoren Pokédex, Attacken, Gegenstände; Spezifikation: [docs/superpowers/specs/2026-10-23-phase-3b3-data-editors-design.md](../docs/superpowers/specs/2026-10-23-phase-3b3-data-editors-design.md), Gate: [docs/phase-3b3-gate.md](../docs/phase-3b3-gate.md), Teil A), Phase 3b.3b (Trainer und Begegnungen; Gate: [docs/phase-3b3-gate.md](../docs/phase-3b3-gate.md), Teil B), Phase 3b.4a (Prüfen und Beheben; Spezifikation: [docs/superpowers/specs/2026-10-23-phase-3b4-import-checks-design.md](../docs/superpowers/specs/2026-10-23-phase-3b4-import-checks-design.md), Gate: [docs/phase-3b4-gate.md](../docs/phase-3b4-gate.md), Teil A), Phase 3b.4b (Import von Kartenteilen; Gate: [docs/phase-3b4-gate.md](../docs/phase-3b4-gate.md), Teil B), Phase 3e.1 (Scharnier über der Schau, Teiler, schwebende Werkzeugleiste; Plan: [docs/superpowers/plans/2026-10-25-phase-3e1-scharnier-teiler-werkzeugleiste.md](../docs/superpowers/plans/2026-10-25-phase-3e1-scharnier-teiler-werkzeugleiste.md)), Phase 3e.2 (Weltbaum und Sortierung; Plan: [docs/superpowers/plans/2026-10-25-phase-3e2-weltbaum-sortierung.md](../docs/superpowers/plans/2026-10-25-phase-3e2-weltbaum-sortierung.md); Gate: [docs/phase-3e-gate.md](../docs/phase-3e-gate.md)) und Phase 3e.3 (Hierarchie und Inspector; Plan: [docs/superpowers/plans/2026-10-25-phase-3e3-hierarchie-inspector.md](../docs/superpowers/plans/2026-10-25-phase-3e3-hierarchie-inspector.md); Gate: [docs/phase-3e-gate.md](../docs/phase-3e-gate.md)). Die Screens oben zeigen das Ziel, die App holt es in Etappen ein.

Umgesetzt:
- Tokens, Schriften und Symbole aus diesem Ordner: die drei generierten Dateien in `apps/studio/src/ui/tokens/`, die vier Schriftpakete und Phosphor Icons. Thema Hell, Dunkel oder „Wie System“ (Einstellungen; der Knopf in der Titelleiste wechselt Hell und Dunkel), Dichte Komfort oder Kompakt (Einstellungen und Statusleiste). CodeMirror und die 3D-Szene folgen dem Thema.
- Komponentenbibliothek `apps/studio/src/ui/primitives/` auf Base UI (Taste, Symboltaste, Tastenkürzel, Eingabefeld, Zahlfeld, Auswahlfeld, Schalter, Kästchen, Segmente, Reiter, Referenz, Plakette, Meldung, Listen- und Baumzeile, Inspector-Zeile, Abschnitt, Menü, Tooltip, Popover, Dialog, Picker, Leerer Zustand, DS-Bildschirm), sichtbar in der Galerie (`?gallery` im Browser-Modus, mit einer Seite „Rahmen“). Für die Daten-Editoren (Phase 3b.3a) dazu Typ-Plakette (`TypeBadge`), Wertebalken (`StatBar`), Tabelle (`Table`), Filter-Plaketten (`FilterChips`), Kästchen-Raster (`ToggleGrid`), Hexblock (`HexBlock`) und Symbol (`Sprite`), der Panelkopf mit Unterbereichs-Menü (`PanelHead.switcher`); Galerie-Seite „Daten“. Mit Phase 3b.3b die Pokémon-Karte (`PokemonCard`: Ruhe, Gewählt, Leer „Platz 4 leer“; Fläche Blatt, Kobalt-Kante bei Auswahl, Menüsymbol aus dem ROM, „Lv. 12“ und Item), in der Galerie unter „Daten“, „Team-Karten“.
- Die Pokémon-Typen als Szenen-Tokens `szene/typ/<schluessel>/{grund,schrift}` (18 Typen, RGB555-treu, Kontrast von `check_tokens.py` geprüft); `tokens.ts` liest `typen` daraus, die Typ-Plakette zeigt Farbe und Name.
- Der Rahmen: Titelleiste, Atlas (Start, Welt, Pokémon, Gegenstände, Texte, Technik, dazu Geschichten gesperrt bis 3d, Hilfe, Einstellungen), Liste je Bereich, Deck aus Scharnier, Schau, Teiler und Hand (das Scharnier oben mit Brotkrume, Bauen und Tüfteln, Prüflicht, Verlauf, „Hier spielen“), Statusleiste, Toasts, Befehlspalette ⌘K.
- Welt: 3D, Raster und Weltkarte als Schau; die Werkzeuge mit Buchstabenkürzeln (V, B, F, T, A, S, G) als schwebende Leiste in der 3D-Ansicht; der Inspector mit seinem Kopf als Hand, Script und Probleme als Reiter daneben, die Hierarchie „In dieser Szene“ in der Liste; Szenenfarben aus den Tokens.
- Werkbank, erster Schritt (Phase 3e.1): das Scharnier steht über der Schau, ohne Griff und nicht mehr ziehbar; zwischen Schau und Hand sitzt der Teiler (`role="separator"`, 8 px) mit Ziehen, Pfeiltasten, Pos1, Ende und Enter. Die Werkzeuge verließen die Hand und schweben als Leiste in der 3D-Ansicht (Anker links, oben, unten, gemerkt in `g4.shell.v1`; in einer niedrigen Ansicht liegt sie flach oben); die Buchstaben wirken nur bei sichtbarem 3D. `schatten/schwebend` ist ein DTCG-Token. Galerie „Rahmen“ › „Scharnier oben“ und „Werkzeugleiste schwebend“. Hierarchie und Inspector-Kopf kamen mit 3e.3 (unten), ein Dock je Bereich folgt in 3e.4; `Editor_DesignSystem.fig` zieht danach nach.
- Weltbaum und Sortierung (Phase 3e.2): die Liste „Orte“ ist ein Baum Weltkarte › Gebiet › Räume (Platin: Sinnoh; HeartGold und SoulSilver: Johto und Kanto), danach „Weitere Gebiete“ (Gebiete auf eigener Karte wie Großmoor oder die Safari-Zone), „Nicht verbunden“ und, nur in Tüfteln, „Sonstiges“. Räume hängen am Gebiet ihres Eingangs (über die Türen) und tragen Klarnamen („Pokémon-Center, 1. Stock“, „Haus 2“, „Ebene 3“), die man umbenennen und einem anderen Gebiet zuordnen kann; das merkt die Projektdatei `g4-world.json`. Das Menü „Sortierung“ im Listenkopf: Spielreihenfolge, Name A–Z („Route 2“ vor „Route 10“), Zuletzt gewählt; in der Listenansicht Nummer, Name, Kürzel. Bauen spricht vom Spiel (Weltkarte, Gebiet, Raum, Ort, Karte), Tüfteln vom ROM (Header, Matrix, Raster); der Reiter „Raster“ selbst heißt bis 3f.1 so. Das Glossar kennt `worldMap`, `area`, `landscape`.
- Hierarchie und Inspector (Phase 3e.3): die Liste der Welt heißt „Welt“ und hat zwei Abschnitte, „Orte“ (der Weltbaum von 3e.2, auf eine Zeile mit dem Wähler „Ort oder Raum wählen“ einklappbar, gemerkt als `placesFolded`) über „In dieser Szene“, getrennt durch den waagrechten Teiler „Höhe der Orte“ (Anteil `worldListSplit` 0,2 bis 0,8 in `g4.shell.v1`, jeder Teil mindestens 96 px). Der Baustein `Splitter` gibt es damit senkrecht und waagrecht (4-px-Schwelle zum Ziehen, Pfeile ±16 px, Pos1 und Ende) und trägt auch den Teiler der Hand. „In dieser Szene“ (`SceneHierarchy`, `role="tree"`) zeigt das gezeigte Gebiet oder den Raum als Wurzel mit den Gruppen Karten (mit ihren Gebäuden), Figuren, Türen, Auslöser, Schilder und Scripts samt Zähler; eine Tür nennt ihr Ziel in der zweiten Zeile, Befunde und gesperrte Eventdateien stehen als Zeichen und Plakette an den Zeilen, die Suche „Name oder Nummer suchen“ filtert, „Zurück“ und „Vor“ (⌥⌘←, ⌥⌘→) gehen durch die gezeigten Szenen, das Menü „Hinzufügen“ (+) wählt das Werkzeug für Figur, Tür, Auslöser, Schild oder Gebäude. Der Inspector hat einen gemeinsamen Kopf (`InspectorHead`): Augenbraue mit Art-Wort und Symbol, Titel, Kennung grau in Mono, Plaketten und das Menü „Aktionen“ (Karte: Teile importieren…, Teile exportieren…, Duplizieren; Figur, Tür, Auslöser, Schild, Gebäude: Löschen; Gebiet und Raum: Raum hier anbinden…). Er zeigt nur die Auswahl; was unter dem Zeiger liegt, nennt die Mitte der Statusleiste. Im Header-Editor ersetzt die Zeile „2 Türen, 1 ohne Ziel“ (sie führt in die Gruppe „Türen“) den Abschnitt „Eingänge / Ausgänge“ mit „Warp hinzufügen“; Türen setzt man mit dem Werkzeug „Tür“ oder „+“ › „Tür“. „An diesem Ort“ gibt es nicht mehr; die Layouts liegen unter `g4.layout.v10.*`. Galerie „Rahmen“ › „Inspector-Kopf“, „Hierarchie „In dieser Szene““ und „Teiler waagrecht und senkrecht“.
- Texte nach dem Screen „Texte: Archiv 0412“: Archive als Liste (Orte und Spielweit), DS-Bildschirm mit Seitenkarten als Schau, Nachrichtentabelle mit Längenbalken und Editor als Hand.
- Pokémon und Gegenstände nach den Screens „Pokédex: Staravia bearbeiten“, „Attacken: Flügelschlag“ und „Gegenstände: Trank“ (Phase 3b.3a): der Listenkopf wird zum Menü der Unterbereiche (Pokédex, Trainer, Begegnungen, Attacken mit Zählern, ⌘1 bis ⌘4), das zweite Glied der Brotkrume öffnet dasselbe Menü; Listen mit Suche über Name und Nummer, Filter-Plaketten und Symbolen aus dem ROM; die Schau mit Kopfkarte, Wertebalken gegen den Schnitt, „Fundorte“ und „Verwendet in“ als Referenzen („ohne Scripts gezählt“), Beschreibungen als DS-Textbox; die Hand mit Reitern (Pokédex) oder Spalten (Attacken, Gegenstände), jedes Feld ein Schritt mit sprechendem Label im Verlauf; harte Grenzen sichtbar („20 von 20“). Tüfteln zeigt die Rohdaten als Hexblock mit markiertem Feld und Dez/Hex an den Zahlfeldern.
- Trainer und Begegnungen nach den Screens „Trainer: Wanderer Kai“, „Neuer Trainer: Team“ und „Begegnungen: Route 201“ (Phase 3b.3b): Trainer als Baum nach Ort oder Klasse mit „Lv. 12“ rechts, die Schau mit dem Team als Pokémon-Karten und „Steht in der Welt“ (Figuren als Referenzen in die Welt, „In der Welt platzieren“), die Hand mit den Reitern Team (sechs Karten, darunter die Details des Platzes), Trainer (Klasse, Name mit „4 von 7“, KI als Segmente, Schalter, Kampftexte) und Rohdaten; „Trainer anlegen…“ als Dialog mit der Plakette „Nicht rückgängig“. Begegnungen als Liste der Dateien mit den Orten, die sie nutzen; die Schau „Wer hier wartet“ mit Anteil je Tageszeit und „Verwendet von N Orten“ mit „Für diesen Ort abspalten“; die Hand mit Reitern je Methode, Tabelle mit „Ersetzt durch“ und gedämpften leeren Plätzen; Warnungen inline, in der Schau und im Prüflicht. In der Welt das Feld „Trainer“ im Inspector und das Werkzeug „Figur“ mit Trainerbindung (Statusleiste „Klick setzt Wanderer Jens“).
- Prüfen und Beheben (Phase 3b.4a): das Prüflicht im Scharnier mit drei Zählern (Fehler, Warnungen, Hinweise, je Symbol und Zahl, eine Null gedämpft; die Farbe folgt der höchsten Schwere), während eines Prüflaufs ein Kreisel mit Fortschritt („Prüft … Texte 312/724“) und, solange die Prüfungen kein Ergebnis haben, ein leerer Ring mit „Nicht geprüft“ statt „Alles stimmig“ (Panel und Technik ebenso, mit „Erneut prüfen“; Galerie „Rahmen“ › „Prüflicht“). Das Panel „Probleme“ gruppiert die Befunde nach Bereich (Welt, Scripts, Texte, Pokémon, Projekt) und Code mit Zählern; jede Zeile hat ihren Satz mit Ausweg, den Ort („Eventdatei 0394 · Warp 0“) und darunter die Tasten „Öffnen“, „Beheben…“ und „Ausblenden“ (nie bei Fehlern), der Fuß „N ausgeblendet · Wieder melden…“ (die Liste klappt beim Leeren und beim Hinzukommen von Einträgen zu und rollt in ihrer eigenen Höhe, der Fuß nimmt höchstens die halbe Hand) und „Erneut prüfen“, nicht Geprüftes als Hinweiszeilen. Dieselbe Tabelle steht in Technik › Prüfen und Bauen, dort mit „Ausgangslage übernehmen…“ (Rückfrage mit der Zahl) und einem Schalter je Prüfung. Am Objekt: der Inspector-Abschnitt „Prüfungen“ mit den Befunden des gewählten Datensatzes, die Fehlerfarbe `szene/status/fehler` an Warp-Markern und Markierungen (Fehler, Warnung) in Explorer-, Trainer- und Begegnungslisten. Der Fix-Dialog zeigt die Änderung mit dem neuen Baustein `BeforeAfter` (Vorher und Nachher in zwei Spalten, geänderte Zeilen mit − und + auf `auswahl/flaeche`, eine leere Seite als „Leer“; Zeilen brechen nie um, eine Spalte rollt seitlich; Galerie „Daten“ › „Vorher und Nachher“) und bei „Script anlegen“ die Plakette „Nicht rückgängig“; Befundzeilen nutzen die Prüfliste `CheckList` (aus 3b.4b). Die Meldungen sind deutsche Sätze mit Ausweg aus `apps/studio/src/checks/messages.ts`; ein Rust-Test hält sie mit dem Katalog des Kernels gleich.
- Kartenteile importieren und exportieren (Phase 3b.4b): im Menü „Aktionen“ des Inspector-Kopfs einer Karte (Rasterzelle oder Zeile „Karte …“ der Hierarchie) „Teile importieren…“ und „Teile exportieren…“ (bis 3e.3 Tasten am Feld „Karte“; dazu der Palettenbefehl „Teile in die Karte importieren…“). Der Dialog „Teile importieren…“ ist ein `Dialog` ohne eigene Vorschau (die Szene ist die Vorschau, ⌘Z der Rückweg): Segmente „Quelle“ mit „Datei“ (Pfade, eine je Zeile; in der gebündelten App „Dateien wählen…“) und „Karte im Projekt“ (Feld „Quellkarte“ und Picker), darunter die Prüfliste `CheckList` (Galerie „Daten“ › „Prüfliste“) mit einer Zeile je Teil: Kästchen mit Name und Größe, Zustand ok, Warnung, Fehler oder Hinweis als Symbol, darunter ein deutscher Satz mit Ausweg („Das Modell ist 65.536 Byte groß, erlaubt sind 61.440. Wähle ein kleineres Modell.“). Teile mit Fehler lassen sich nicht ankreuzen, leere sind nicht vorgewählt („Leer, wird nicht übernommen“); bei einer ganzen Karte der anderen Spielfamilie der Schalter „Gebäude übernehmen“. Die Taste nennt Zahl und Verb („2 Teile übernehmen“, „1 Teil übernehmen“). Nach dem Import eine Erfolgsmeldung, „Prüft die Karte …“ und dann die Befunde der Karte als zweite `CheckList` mit „Zeigen“ (oder „Keine neuen Befunde an dieser Karte.“); Szene und Vorschaubild im Raster folgen dem neuen Modell. „Teile exportieren…“: Segmente „Teil“ (sechs Teile, mehr als die zwei bis vier der Regel; im 480 px breiten Dialog brechen sie in eine zweite Zeile um, `Segmented` mit `wrap`), Feld „Pfad“ (in der gebündelten App „Pfad wählen…“; ein anderer Teil gibt dem Pfad seine Endung), „Exportieren“; danach „Modell exportiert nach <Pfad>“, ein fehlender Ordner heißt „Den Ordner … gibt es nicht. Lege ihn an oder wähle einen anderen Pfad.“ Die Sätze je Code stehen in `apps/studio/src/workbench/inspector/importParts.ts`; ein Rust-Test hält sie mit den Codes des Kernels gleich.
- Assistenten als Schrittleiste im Scharnier, der Schritt in der Hand; Zusammenfassung mit „Nicht rückgängig“, Ergebnis mit „Zeigen“, „Hier spielen“ und „Schließen“.
- Start („Willkommen zurück“ mit den zuletzt geöffneten Projekten), Technik (Prüfen und Bauen, Patches, Emulator, Werkzeuge), Einstellungen (Darstellung, Emulator, Tastenkürzel); Hilfe öffnet das Glossar als Dialog.
- Tiefe Tüfteln: Rohwerte und Hex im Inspector, Kennungen im Explorer, Header-ID und Höhe im Raster mit Rohdaten der Zeile, Dateipfade in Probleme und Script.
- Die Regeln aus diesem README prüft das Gate: `check-tokens.sh` (Raster, Kontraste, eingecheckte Dateien gleich frischem Export) und `check-styles.sh` (keine Hex-Farben, keine Primitive, keine Inline-Styles außer `/* dyn */`; die Ausnahmeliste `scripts/check-styles.allow` ist leer).

Abweichungen vom Design-System, bewusst oder bis auf Weiteres:
- Die Tiefe Erzählen fehlt (nur Bauen und Tüfteln), ebenso die Live-Vorschau der Assistenten in der Schau und das Rezept-Ergebnis „nächster sinnvoller Schritt“ als Taste (das Ergebnis nennt ihn als Text).
- Windows: keine eigenen Fensterknöpfe und kein eigenes Menü; die Titelleiste liegt unter der Systemleiste.
- Technik › Werkzeuge zeigt den Zustand von apicula und rotom-lsp, aber keinen Pfad (kein Befehl liefert ihn).
- Die Archivliste der Texte nennt die Zahl der Nachrichten nur am geladenen Archiv.
- Die Zeilengrenze der Texte (38 Zeichen, „34 von 38“) zählt Zeichen; die Prüfung „Zeile zu breit“ (`text.too-wide`, seit 3b.4a) misst die Breite gegen 216 px, nur in Dialogarchiven (die frühere 200-px-Warnung des Backends gibt es nicht mehr).
- Einstellungen › Tastenkürzel zeigt nur die ⌘-Kürzel, die Buchstaben der Werkzeuge stehen in der Statusleiste und an den Werkzeugen.
- Im Raster zeigt auch Bauen die Kennungen der Zellen (wie vor 3c); Tüfteln fügt Header-ID und Höhe hinzu.
- Die Szenen-Tokens `ds`, `typ` und `status` (3c.2, 3b.3a, 3b.4a) sind seit dem 25.10.2026 auch in `Editor_DesignSystem.fig` eingespielt. In OpenPencil heißen Szenen-Variablen ohne Präfix (`ds/grund`, `typ/flug/grund`, `status/fehler`).
- Quellkarten ohne Bild (3b.4b): Der Picker „Quellkarte wählen“ im Import-Dialog zeigt die Karten als „Karte 0006“ ohne Kartenbild; der Plan sah die Kartenbilder als Vorschau vor (das Muster „Picker“ verlangt Treffer mit Vorschau).
- Fix-Vorschau als Text (3b.4a): „Umbrechen“ zeigt Vorher und Nachher als Zeilen in `BeforeAfter`, nicht als DS-Bildschirm (den sieht die Spezifikation 3b.4 §6 vor); die Seiten trennt eine Leerzeile. Ein DS-Bildschirm in der Vorschau braucht den rohen Text des Fixes und kommt später.
- Nur Warps tragen die Fehlerfarbe (3b.4a, Spezifikation §8); Figuren, Auslöser und Schilder zeigen Befunde im Inspector und in den Listen, nicht am Marker. Pokédex, Attacken und Gegenstände haben keine Prüfung und keine Markierung.
- Typfarben als Tokens (3b.3a): Die Pokémon-Typen sind die Token-Gruppe `szene/typ` in `tokens/Szene/Wert.tokens.json` statt einer festen Tabelle in `export_tokens.py`; die Werte entsprechen den bisherigen Typfarben. Seit dem 25.10.2026 sind sie in der `.fig` eingespielt.
- Kein `g4.shell.v2` (3b.3a): Die Spezifikation sieht für Unterbereich, Auswahl und Hand-Höhe je Unterbereich einen neuen Rahmen-Schlüssel vor. Die App bleibt bei `g4.shell.v1` und ergänzt die optionalen Felder `sub` (Unterbereich von Pokémon) und `selected` (Auswahl je Teil); ältere Stände lesen sich ohne Migration.
- Keine Hand-Höhe je Unterbereich (3b.3a, auch nach 3b.3b offen): Die Hand merkt ihre Höhe je Bereich; Pokédex, Trainer, Begegnungen und Attacken teilen sie. Grund: die Höhe hängt am Deck des Bereichs, eine Höhe je Unterbereich wartet auf den Umbau des Decks.
- Beschreibungen über `data_get` (3b.3a): Beschreibungen von Attacken und Gegenständen kommen mit dem Dokument aus `data_get` (Archiv, Nachricht, Text) und stehen nur lesend als DS-Textbox in der Schau; bearbeitet werden sie im Bereich Texte („Beschreibung bearbeiten“ springt dorthin).
- Der Pokédex zeigt kein DS-Profil des Screens (Pokédex-Profil, Kampfvorschau kommen mit 3d); Kampfsprites sind nicht da, Item- und Pokémon-Symbole kommen aus dem geöffneten ROM.
- Kampfsprites später (3b.3b): Trainer und Pokémon haben keine Kampfsprites (Spezifikation 3b.3 §12, Platzhalter bis 3d; verschlüsselte NCGR, Platin und HGSS verschieden); die Pokémon-Karten zeigen die Menüsymbole aus dem ROM. Ebenso später: der Assistent „Neuer Trainer“ mit Live-Vorschau (3d, Erzählen); die App legt Trainer über einen Dialog an.

Nicht in 3b.3: Läden und die übrigen Punkte der Tabelle „Später“ der Spezifikation 3b.3 (in [docs/ROADMAP.md](../docs/ROADMAP.md) übernommen) und das Budget. Trainer, Begegnungen und die Pokémon-Karte kamen mit 3b.3b.

Nicht in 3e.3: der Reiter „Raster“ selbst (3f.1, bis dahin heißt er so), der Schritt „Header“ der Assistenten (3f.3), eigene Orte (3f.2) und ein Dock je Bereich statt zweier (3e.4; Hand und Schau sind bis dahin zwei Dockview-Gruppen, `dock-welt-hand` bleibt).

Nicht in 3c (§1 der Spezifikation): die Bereiche Geschichten, Pokémon und Gegenstände (im Atlas gesperrt; Pokémon und Gegenstände kamen mit 3b.3a); Tiefe Erzählen mit Rezepten und reduziertem Inspector; Live-Vorschau der Assistenten in der Schau; Story-Graph; Spielmodus mit Live-Flags und bernsteinfarbenem Scharnier; Veröffentlichen; Plugins; Gelände malen und Draufsicht-Kamera; Muster im Gelände-Overlay; Höhen-Overlay in der Schau; eigene Fensterknöpfe auf Windows; senkrechtes Scharnier ab 2200 px; die Komponenten Budget, Pokémon-Karte, Story-Knoten und Rezept-Taste (kommen mit ihrem Editor; Typ-Plakette und Wertebalken kamen mit Phase 3b.3a, die Pokémon-Karte mit 3b.3b). Die Roadmap führt sie unter „Aus dem Design-System, später“ ([docs/ROADMAP.md](../docs/ROADMAP.md)).

## Arbeiten an der OpenPencil-Datei

- **Gestrichelte Konturen** immer mit beiden Angaben bauen: `strokeDash={[6, 4]} dashPattern={[6, 4]}`. OpenPencil 0.16 speichert `strokeDash` nicht; `dashPattern` am Knoten wird gespeichert und beim Laden auf die Kontur übertragen.
- **Nach jedem Speichern in OpenPencil** einmal `node design-system/scripts/fix_fig_text.mjs design-system/Editor_DesignSystem.fig --rgb555` (aus der Repo-Wurzel) ausführen (Dokument vorher in OpenPencil schließen). OpenPencil schreibt beim Speichern für manche Texte Glyphen-Umrisse mit falschen Indizes, die nach dem nächsten Öffnen als Zeichensalat erscheinen. Das Skript entfernt diese Umrisse, dann setzt OpenPencil die Texte beim Öffnen mit der echten Schrift. `--rgb555` rastet zusätzlich alle Farben auf das DS-Raster ein. Eine Sicherung liegt danach neben der Datei (`.vor-textfix`).
- Ab 10.000 Knoten lehnt OpenPencil `update_node`, `set_text` und `set_layout` ab; Änderungen gehen dann über `render` mit `replace_id`.
- Große JSX nicht in den MCP-Aufruf kopieren: Screen-Code in `<name>.body.js` schreiben, mit `openpencil/build-screen.sh <name>` bauen und mit `node scripts/op.mjs render '{…}' --jsx <name>.jsx` rendern. Bilder holt `node scripts/op.mjs export_image '{…}' --png bild.png`. `op.mjs` schließt seine Sitzung nach jedem Aufruf; der Server erlaubt höchstens zehn gleichzeitige Sitzungen.
- Scheitert ein `render` mittendrin (etwa an einer unbekannten Variablen), bleibt ein halber Frame zurück. Mit `get_page_tree` (depth 1) prüfen und löschen.
- Szenen-Variablen heißen ohne Präfix: `v('ds/grund')`, nicht `v('szene/ds/grund')`.
- Den MCP-Server startet die App selbst, wenn `openpencil-mcp-http` im PATH liegt. Fehlt er nach einem Neustart, hilft `OPENPENCIL_MCP_AUTH_TOKEN="" openpencil-mcp-http` aus dem Projektordner und danach ein Neustart der App.

## Grenzen der OpenPencil-Datei

- OpenPencil setzt per MCP keine Modi pro Rahmen. Alle Screens zeigen das helle Thema; ein Screen im dunklen Thema ist mit Dunkel-Hex-Werten nachgebaut. In der App schalten die Tokens.
- Token-Farben an Vektoren (Symbole) bindet OpenPencil nicht zuverlässig. In den Komponenten sind die Symbole nachträglich an Tokens gebunden; in den Screens tragen sie die Hex-Werte des hellen Themas. In der App erben Symbole `currentColor`.
- Pokémon-, Figuren- und Spielgrafiken sind bewusst Platzhalter. Das Projekt liefert keine Nintendo-Assets aus.
