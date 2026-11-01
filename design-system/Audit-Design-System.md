# Audit: Design-System „Doppeldeck“ gegen die App

Stand: 01.11.2026. Grundlage ist der Stand nach Phase 3 vom 25.10.2026 (3c, 3b.3a, 3b.3b, 3b.4a, 3b.4b; `main` bei `eecd4f5`); die Zeilen der späteren Phasen bis 3d.3 Teil B (Ei-Attacken, Branch `phase-3d-3-sprites-ds-eier`) sind nachgetragen.

Verglichen werden zwei Stände:
- **Initiale Idee:** `Editor_DesignSystem.fig` vom 23.10.2026 (Grundlagen, Komponenten, Muster, 30 Screens) und das README dieses Ordners.
- **App:** Code in `apps/studio/src/`, die Galerie (`?gallery`) und ein Durchlauf in der Test-Umgebung auf einer frischen Platin-DE-Kopie (Hell, Dunkel, alle Bereiche, Dialoge).

Die `.fig` ist mit diesem Audit nachgezogen. Was dort jetzt steht, beschreibt der Abschnitt 7.

Jede Abweichung trägt eine Einordnung:
- **bewusst:** in einer Spezifikation, einem Ruling oder der Roadmap entschieden, mit Quelle.
- **Lücke:** fehlt oder weicht ab, ohne dass es entschieden wurde.
- **Verbesserung:** die App ist besser als das Design; das Design übernimmt sie.

## 1. Kurzfassung

Das Gerüst der Idee steht. Raster, Maße, Tokens, Schriften, Symbole und die Komponentenbibliothek sind umgesetzt und werden vom Gate geprüft. Das Scharnier ist reicher als entworfen, und Tüfteln ist durchgehend gebaut.

Von den 30 Screens sind **11 umgesetzt, 10 teilweise und 9 offen**. Offen sind vor allem die Teile, die die Idee von einem ROM-Editor unterscheiden sollten. Die sechs größten Abweichungen:

1. **Die Tiefe Erzählen ist nur zum Teil da.** Seit 3d.2 gibt es sie in der Welt (Rezept-Leiste, reduzierter Inspector, Entwurf der Assistenten in der Schau); andere Bereiche zeigen dort Bauen, und drei der fünf Rezepte sind gesperrt (bewusst, 3d.5 bis 3d.8).
2. **Die Schau zeigt Daten statt Spielerblick.** Pokédex, Attacken, Trainer und Gegenstände zeigen Datenkarten. DS-Profil und Kampfvorschau stehen seit 3d.3 Teil A (Pokédex, Trainer), das Laden fehlt (bewusst, später: keine Daten).
3. **Es gibt keinen Spielmodus.** „Hier spielen“ startet den Emulator extern; ein bernsteinfarbenes Scharnier mit Live-Flags gibt es nicht (bewusst, Phase 4).
4. **Die zweite Sprachebene ist nur halb da.** Kennungen sind Nummern statt Symbolen. Räume heißen nach ihrem Kürzel (`T01R0102`), Event-Titel sind technisch („Objekt 2 · ID 2“), Grafik und Bewegung sind rohe Zahlen (Lücke).
5. **„Nie nur Farbe“ gilt in der Szene nicht.** Das Gelände-Overlay hat keine Muster, und die Ereignis-Marker sind Quader statt Formen (bewusst für das Overlay, Phase 6; Lücke für die Marker).
6. **Assistenten sind Formulare.** Sie laufen in der Hand, ohne Live-Vorschau in der Schau und ohne große Auswahlkarten. Der Assistent „Neuer Trainer“ ist ein Dialog geworden (bewusst, 3d).

## 2. Was trägt

- **Raster:** Die Maße stimmen alle mit dem Code überein: Titelleiste 40, Atlas 88, Liste 272, Scharnier 44, Hand 286 (200 bis 560), Statusleiste 28, Grundtext 13/18.
- **Tokens:**
  - Aus dem Entwurf wurde kein Token entfernt und keiner im Wert geändert.
  - Neu sind nur die 45 Szenen-Tokens `ds`, `typ` und `status`.
  - Seit 3d.3 Teil A kommen zehn in `ds` dazu (`status`, `kp`, `kampf`).
  - `check-tokens.sh` prüft RGB555, Kontraste und ob die generierten Dateien zum Export passen.
- **Stilregeln:** `check-styles.sh` verbietet Hex, Primitive und Inline-Styles; die Ausnahmeliste ist leer.
- **Bernstein:** Er gehört allein dem Spielen. `live/*` nutzen nur „Hier spielen“ im Scharnier und im Ergebnis der Assistenten.
- **Tüfteln:**
  - Der Abschnitt „Technisch“ mit Dez/Hex.
  - Ein Hexblock mit markiertem Feld.
  - Rohwerte im Raster und Dateipfade.
- **Reversibilität:**
  - Jedes Feld ist ein Schritt mit sprechendem Label im Verlauf.
  - „Nicht rückgängig“ steht in Zusammenfassungen, bei Trainer, Begegnungsdatei, Abspalten, Script anlegen und Patch.
  - Das Anlegen ist eine Undo-Grenze.
- **Base UI 1.9** statt Radix oder Reka (bewusst, Spec 3c §3: Radix hat weder Number Field noch Combobox). Das Verhalten passt zum Design. Felder übernehmen den Wert beim Verlassen, Escape nimmt den Entwurf zurück.

## 3. Konzept und Prinzipien

| Prinzip | Idee | App | Einordnung |
|---|---|---|---|
| Oben zeigen, unten anfassen | Die Schau zeigt, was der Spieler sieht | In Welt und Texte ja. In Pokémon und Gegenstände stehen in der Schau Datenkarten (Kopf, Werte, „Fundorte“, „Verwendet in“). DS-Bildschirme gibt es nur für Texte und Beschreibungen | bewusst (Spec 3b.3 „Später“: DS-Varianten 3d) |
| Drei Tiefen | Erzählen, Bauen, Tüfteln | Drei Segmente seit 3d.2; Erzählen nur in der Welt, sonst Bauen | bewusst (Spec 3d §1, §3.3; Geschichten 3d.6, Pokémon und Texte 3d.8) |
| Zwei Sprachebenen | Schöpferwort vorne, technische Kennung grau | Umgesetzt in Referenzen, Brotkrume, Explorer (Tüfteln) und Glossar. Es fehlen: symbolische Kennungen (`SPECIES_STARAVIA`, die App nutzt `0397`), Klarnamen für Räume (`T01R0102`) und Events („Objekt 2 · ID 2“) und Picker für Grafik und Bewegung (rohe Zahlen) | Lücke; symbolische IDs bewusst Phase 4 |
| RGB555 | Alle Farben DS-treu | Tokens ja. Die Ortsfarben in Explorer, Raster und Weltkarte sind HSL-Pastelle (`workbench/places.ts`, `placeColor`) | Lücke |
| Nie nur Farbe | Status mit Symbol, Overlays mit Muster | Prüflicht, Prüfliste, Typ-Plaketten und Legende ja. Das Gelände-Overlay ist reine Farbfläche (42 %), die Marker sind Quader; nur Warps tragen die Fehlerfarbe, ohne Form | Overlay bewusst (Spec 3c §8.1, Phase 6); Marker Lücke |
| Rezepte statt leerer Leinwand | Rezepte erzeugen normale Objekte | Rezept-Leiste in Erzählen mit „Gebiet anlegen“ und „Raum anlegen“; drei Rezepte gesperrt „Kommt mit 3d.5“. Die Assistenten zeigen ihren Entwurf in der Schau und im Ergebnis den nächsten Schritt als Taste | bewusst (3d.5) |
| Wege statt Nummern | Referenzen mit „Verwendet in“ | Abschnitte „Verwendet in“ und „Fundorte“ gibt es. Das Rechtsklick-Menü der Referenz nutzt kein Editor; Scripts zählen nicht mit; viele Felder bleiben Zahlen (Ziel-Warp, Grafik, Lokale ID) | Lücke; Script-Referenzen bewusst später |
| Reversibel sichtbar | Verlauf, „Nicht rückgängig“, Schnappschuss | Verlauf und Plaketten ja. Einen Schnappschuss vor Unumkehrbarem gibt es nicht. „Textarchiv anlegen“ schreibt ohne Plakette und ohne Rückfrage | Lücke |
| Eine primäre Taste je Ansicht | Kobalt einmal | Eingehalten | – |
| Sprache | Keine Mittelpunkt-Ketten, keine Versal-Labels | Die App nutzt Mittelpunkte in der Statusleiste („Klick wählt · Ziehen verschiebt“), in Event-Titeln, in Orten von Befunden („Eventdatei 0004 · Warp 0“) und im Fuß „N ausgeblendet · Wieder melden…“. Namen aus dem ROM stehen in Versalien („STARAVIA“) | Lücke (Mittelpunkte); Versalien bewusst, Rohdaten des ROMs |

## 4. Rahmen und Scharnier

| Teil | Idee | App | Einordnung |
|---|---|---|---|
| Prüflicht | Ein Punkt mit Text („2 Hinweise“) | Punkt plus drei Zähler (Fehler, Warnungen, Hinweise) mit Symbol. Eine 0 ist gedämpft. Während eines Laufs „Prüft … Texte 312/724“, ohne Ergebnis „Nicht geprüft“, nie fälschlich „Alles stimmig“ | Verbesserung (3b.4a) |
| Verlauf | Undo-Zahl | Menü mit den letzten 10 Schritten und ihren Labels, eigene Taste „Wiederholen“ | Verbesserung |
| Spielmodus | Scharnier in Bernstein, „Spielt in melonDS“, „Neu laden“, „Stoppen“ | Fehlt; „Hier spielen“ zeigt beim Bauen „Baut …“ | bewusst (Spec 3c §1, Phase 4) |
| Assistent | Schrittleiste im Scharnier, Vorschau in der Schau | Die Schrittleiste ist da (Titel Zen Maru 15 px statt 16). Der Schritt steht in der Hand, die Schau bleibt ohne Vorschau. Assistenten gibt es nur in der Welt | Vorschau bewusst (3d) |
| Titelleiste | Projekt, Spiel, Speicherstand, Suche, Aktionen mit „Veröffentlichen“; Windows-Variante | Neu ist „Projekt öffnen“. „Veröffentlichen“ ist seit 4.5 da (zwischen „Bauen“ und „Thema wechseln“, gesperrt mit Hinweis im Tooltip), offen bleibt Windows: Windows hat keine eigenen Knöpfe, die 40-px-Zeile liegt unter der Systemleiste | Windows bewusst (Spec 3c §3) |
| Atlas | 7 Einträge | Dazu Hilfe und Einstellungen unten. Geschichten ist gesperrt („Kommt mit Phase 3d“). Kompakt ist 56 px breit und zeigt nur Symbole. Während eines Assistenten ist alles außer Welt gesperrt | Verbesserung (Varianten jetzt in der `.fig`) |
| Deck | Schau und Hand als ein Dock | Zwei Dockview-Instanzen. Bei nur einem Panel gibt es keine Reiterleiste, sondern einen Panelkopf | bewusst (Spec 3c §3) |
| Hand-Höhe | Je Editor eigene Höhe (286 bis 410) | Eine Höhe je Bereich; das Script hebt die Hand auf mindestens 400 px. Es bleibt der Schlüssel `g4.shell.v1` | bewusst bis auf Weiteres (Spec 3b.3 §9) |
| Reiter | Aktiver Reiter mit 2 px Kobalt | Dockview-Reiter: Kobalt **oben** auf `flaeche/schau`. Primitive `Tabs` (in Pokédex, Trainer, Begegnungen): Kobalt **unten** auf `flaeche/blatt` | Lücke: zwei Formen für dieselbe Sache |
| Toasts | Meldung mit vier Arten, auch als Toast | Toast nur als Info, Fehler und Fortschritt, mit 4-px-Kante links und X; die Meldung im Panel hat keinen Rand | bewusst vereinfacht |

## 5. Grundlagen

**Farben und Tokens**
- **Neu (bewusst):**
  - `szene/ds/*` mit 8 Tokens für den DS-Bildschirm (3c.2). `ds/grund` ist dunkel (#182131) statt des grünen Platzhalters aus der `.fig`.
  - `szene/typ/<typ>/{grund,schrift}` mit 36 Tokens (3b.3a): vorher eine feste Tabelle im Exportskript, Werte gleich, Schlüssel jetzt ASCII.
  - `szene/status/fehler` #DE3142 (3b.4a).
- **Nicht in DTCG (Lücke):**
  - der einzige Schatten `--g4-schatten-schwebend` (0 8 24, Platin-1000 mit 24 %);
  - die Ebenen `--g4-ebene-*`;
  - die Syntaxfarben für CodeMirror;
  - `gelaendeMuster` und `ereignisForm`.

  Sie stehen in `base.css` oder fest in `export_tokens.py` und sind in OpenPencil nicht als Variablen pflegbar.
- **Neues Muster (Verbesserung):** Das Scharnier belegt 13 Thema-Tokens lokal neu (`text-*`, `flaeche-*`, `linie-*`, `auswahl-*`, `fokus-ring`, `akzent-text` auf `scharnier/*`). So erscheinen alle Primitives darin dunkel, in beiden Themen.
- **Nie genutzt:** Ungenutzt sind `text/invers`, `flaeche/bildschirm`, die `-punkt`-Farben außer Welt, `szene/hoehe/*` (es gibt kein Höhen-Overlay) und `szene/ereignis/gegenstand`.

**Typografie**
- Zen Maru Gothic läuft nur als `titel-s` (700, 24/30) für Seitentitel und Datenköpfe. `titel-m`, `titel-l` und `spiel-textbox` nutzt kein Baustein.
- Der Schnitt 900 wird geladen, aber nie benutzt.
- Leere Zustände und Dialogtitel stehen in Atkinson (15/20 bzw. 18/24), nicht in Zen Maru (Lücke).
- Pixelify Sans läuft in 12/16 × Faktor (ganzzahlig) statt im Token 24/32 (bewusst, DS-Maß).
- Die Tabellenziffern aus `tokens.css` wirken nicht global, weil jeder `font`-Kurzschreiber sie zurücksetzt. Explizit stehen sie nur in Scharnier, Zahlfeld und Problemen (Lücke).
- Mono ist nur in 400 geladen, wird aber fett und kursiv benutzt (CodeMirror, Technik); der Browser rechnet die Schnitte künstlich (Lücke).

**Symbole:** Phosphor wie geplant. Der gefüllte Schnitt steht für aktive, gedrückte und Status-Zustände, nicht nur für Marker (bewusst, Doku angepasst).

**Dichte:** Kompakt ändert mehr als Abstände. Der Atlas wird 56 px breit, die Werkzeuge verlieren ihre Namen (Spec 3c §5.3). Die Dockview-Reiter bleiben 36 px hoch und 13 px groß (Lücke).

**Szene (three.js)**
- **Wie geplant:** Farben aus den Tokens, Overlay mit 42 %.
- **Muster und Kontur fehlen:** keine Schraffur, Wellen oder Punkte, keine Kontur mit Halo, kein Höhen-Overlay (bewusst, Phase 6).
- **Marker:**
  - Warp, Auslöser und Schild sind Quader, der Warp mit Bake. Figuren sind Overworld-Sprites.
  - Es gibt keinen Gegenstand-Marker. Die Auslöser-Fläche ist gefüllt statt gestrichelt.
  - Die Auswahl ist eine 1-px-Kontur, im dunklen Thema weiß.

## 6. Komponenten

| `.fig` | Stand in der App | Abweichung |
|---|---|---|
| Taste, Symboltaste, Tastenkürzel | umgesetzt | Kürzel ist Mono 12 auf `flaeche/deck`, ohne dicke Unterkante |
| Eingabefeld, Zahlfeld, Auswahlfeld | umgesetzt | Feld 32 statt 28 hoch, Rand `linie/standard`. Das Zahlfeld hat links eine Griffleiste zum Ziehen; Dez/Hex nur in Tüfteln |
| Schalter, Kästchen | umgesetzt | Gewähltes Segment mit Kobalt-Text und Innenkante (Segmente: nächste Zeile) |
| Segmente | umgesetzt (3d.2) | Das Segment „Tiefe“ hat drei Optionen mit Wort und Symbol (Erzählen `narrate`, Bauen `build`, Tüfteln `tinker`); Erzählen gilt nur in der Welt, jeder andere Bereich zeigt dort Bauen, das Segment bleibt auf Erzählen. Die `.fig` kennt das noch nicht, die Frames folgen |
| Reiter | umgesetzt | siehe Abschnitt 4, zwei Formen |
| Referenz | umgesetzt | Pille (Radius 999) statt Radius 6; Kennung Mono dahinter; Rechtsklick „Verwendet in“ ungenutzt |
| Zustand (Plakette) | erweitert | 12 statt 7 Arten, nur 16 px hoch |
| Typ | umgesetzt | zwei Größen (16 und 22) aus `szene/typ` |
| Meldung | umgesetzt | ohne Rand, Statusfläche |
| Wertebalken | umgesetzt | Schnitt-Strich, Urteil „über/unter Schnitt“, Abweichung links in Kobalt |
| Budget | **fehlt** | nur Längenbalken in Texte („34 von 38“) und Zähler („4 von 7“) |
| Listenzeile, Baumzeile, Inspector-Zeile | umgesetzt | 32 hoch; Inspector-Bezeichnung 400 statt 600 |
| Kontextmenü an Listen- und Baumzeile | teilweise (4.1b) | Rechtsklick, Shift+F10 und Kontextmenü-Taste öffnen das Popup des Menüs. Es trägt bisher nur „… auf Original zurücksetzen…“, nicht die übrigen Einträge des Menüs „Aktionen“ (Plan 4.1b: „gleiche Einträge“); in der `.fig` noch nicht gezeichnet (`openpencil/NACHTRAG-4-1b.md`) |
| Änderungstabelle | umgesetzt (4.4) | Kein Vorbild in der `.fig`. `ChangeTable`: drei Spalten (Feld, alt, neu) in `code/s`, Feldspalte so breit wie ihr Inhalt (Pfad bricht nur an „.“ und „[“), Wort-Spannen als `<del>` auf `status/fehler-flaeche` (durchgestrichen) und `<ins>` auf `status/erfolg-flaeche` (unterstrichen), fehlende Seite „—“ (vorgelesen „fehlt“), Fußzeile für gekappte Zeilen. Keine neuen Tokens. Offen (Blick): in Dunkel sind die Flächen der Spannen kaum zu sehen (1,23 bzw. 1,35 : 1 gegen `flaeche/blatt`), es tragen die Linien. In der `.fig` seit 29.10.2026: Brett „Komponenten (App): Änderungstabelle und Git“ (Seite „Komponenten“, x 3440, y 15380) |
| Scharnier | umgesetzt ohne Spielen | siehe Abschnitt 4 |
| Titelleiste | umgesetzt (macOS) | mit „Veröffentlichen“ (4.5); Windows fehlt |
| Atlas-Eintrag | erweitert | 64 hoch, Symbol 24, gesperrt, Kompakt |
| Werkzeug-Taste | umgesetzt | 2 Spalten, Buchstabe oben rechts |
| Rezept-Taste | umgesetzt (3d.2) | `RecipeButton`: Ruhe, Hover (`flaeche/deck`), Aktiv (`aria-pressed`), Gesperrt (Schloss, Grund im Tooltip und als Beschreibung, `text/deaktiviert`); Symbol auf einer Kachel in der Bereichsfarbe wie `Reference`; `data-recipe`. Die Namen `flaeche/hover` und `text/gedaempft` des Entwurfs gibt es nicht. In der Rezept-Leiste (fünf Tasten, drei gesperrt „Kommt mit 3d.5“) und im Ergebnis „Als Nächstes“. Die `.fig` kennt das noch nicht, die Frames folgen |
| Entwurf | umgesetzt (3d.2) | Schicht der Schau: gestrichelte Zell- und Kachelrahmen in `szene/auswahl/halo` bei 50 %, halbtransparente Platzhaltermarker (Sprite erst mit 3d.5); `data-draft`, `data-draft-count`; Weltkarte und Raumplan `data-draft-cell` gestrichelt in `auswahl/kante`. Kein neues Token. Die `.fig` kennt das noch nicht, die Frames folgen |
| Statusleiste | umgesetzt | Befundzahl, Build-Stand, Dichte-Taste |
| Git in der Statusleiste | umgesetzt (4.4) | Kein Vorbild in der `.fig`. Rechts nach der Befundzahl: leise Taste S mit Symbol `git` (Phosphor `GitBranch`), „main · 3 Änderungen“, „main · noch kein Commit“, „4c1d2e3 (losgelöst) · …“, „Git: nicht lesbar“; ohne Repository kein Element; Klick öffnet Technik › Änderungen gegen den letzten Commit. Zählt Dateien wie `git status`, Technik zählt Einträge (Plan „Zahl der Änderungen“, bewusst); „Dateien“ steht nur im Tooltip. In der `.fig` seit 29.10.2026: in den Frames „App: Technik, Änderungen gegen letzten Commit“ und „… Änderung einer Nachricht“ und mit vier Zuständen im Brett „Komponenten (App): Änderungstabelle und Git“ |
| Flag-Picker | umgesetzt (3d.1) | Kein Vorbild in der `.fig`. `FlagPicker` auf dem `Picker`: „Keins (0)“ als erste Zeile wie jede andere, eigene Namen fett über dem pret-Namen, Suche über Name, Alias, Hex und Dezimal, Bereichsfilter, Fuß „Freies Flag nehmen“ (gesperrt mit Grund, solange die Scripts nicht übernommen sind oder der Pool leer ist) und „Im Verzeichnis zeigen“. Zusätze der Bausteine: `PickerItem.keywords` und `strong`, Filter mit `wrap`. Keine neuen Tokens. Die `.fig` kennt ihn noch nicht |
| Flag-Feld | umgesetzt (3d.1) | Kein Vorbild in der `.fig`. `FlagField` in der Inspector-Zeile: Chip der Fläche Geschichten mit Name und Nummer, „Wählen…“, in Tüfteln das Zahlfeld mit Dez und Hex; ohne Bezeichnung in der Tabelle des Level-Scripts. Die `.fig` kennt es noch nicht |
| DS-Bildschirm | umgesetzt | Kartenbild des Orts als Hintergrund, Steuercodes als Plaketten |
| DS-Bildschirm: Pokédex-Profil | umgesetzt (3d.3 Teil A) | `DSPokedexProfile` in der Pokédex-Schau („Spielerblick“) mit Vorderseite, Rückseite, Weiblich, Schillernd; Kategorie, Größe, Gewicht und Text stehen als „folgt“ (keine Zukan-Daten gelesen); neue Tokens `szene/ds/status`; Frame in der `.fig` folgt |
| Ei-Attacken-Reiter | umgesetzt (3d.3 Teil B) | Kein Vorbild in der `.fig`. Reiter „Ei-Attacken“ der Pokédex-Hand nach „Lernset“ (`EggMovesTab`): `Table` mit Attacken-Picker, `TypeBadge` und Entfernen-Taste, „Zeile hinzufügen“ (`aria-disabled` mit Satz bei vollem Platin-Budget oder 16 Attacken), Fußzeile mit der Zahl und in Platin dem Budget („Tabelle 1 910 von 1 910 Einträgen“); gesperrte Tabelle als Warnung mit dem Grund, Spiel ohne Tabelle als Info; keine neue Komponente, keine neuen Tokens. Der Frame in der `.fig` folgt |
| DS-Bildschirm: Kampfvorschau | umgesetzt (3d.3 Teil A) | `DSBattlePreview` im Trainer (Gegner: Trainer oder erstes Pokémon), eigene Seite leer, feste Lage ohne `pokemonSpriteOffsets`, stilisierter Boden; neue Tokens `szene/ds/kp` und `szene/ds/kampf`; Frame in der `.fig` folgt |
| Weltmarker | **teilweise** | 3D-Quader und Sprites statt Formen |
| Pokémon-Karte | umgesetzt (3b.3b) | Radius 12, Menüsymbol aus dem ROM |
| Story-Knoten | **fehlt** | Geschichten 3d |
| Menü, Tooltip, Picker, Leerer Zustand | umgesetzt | Tooltip hell, ohne Pfeil |
| Dialog Bestätigen/Zerstörend | umgesetzt | Zerstörend ist keine Variante, sondern die Taste „Gefahr“ in den Aktionen |

**Neu in der App, ohne Vorbild in der `.fig`** (jetzt auf der Seite Komponenten):
- Abschnitt und Panelkopf, mit dem Unterbereichs-Menü „Pokédex ▾“.
- Tabelle, Filter-Plaketten, Kästchen-Raster, Hexblock, Symbol (Sprite).
- Prüfliste, Vorher und Nachher, Befundmarke, Befundzahl in der Statusleiste.
- Prüflicht in vier Zuständen, Assistenten-Leiste, Startkarte, Toast.
- Schwebende Szenenleiste.

## 7. Screens

| # | Screen (Zielbild) | Stand | Wichtigste Abweichung |
|---|---|---|---|
| 1 | Welt bauen: Figur bearbeiten | umgesetzt | Inspector in einer Spalte ohne Abschnitte, technischer Titel, Grafik und Bewegung als Zahlen; die 3D-Kamera zeigt das ganze Raster statt den Ort |
| 2 | Start: Willkommen zurück | umgesetzt | ohne „Neues Spiel“ |
| 3 | Neues Spiel: Basis-ROM wählen | offen | keine Phase plant es; nur „Projekt öffnen“ für DSPRE-Ordner |
| 4 | Weltkarte: Region Sinnoh | teilweise | Gitter aus Kartenbildern mit Ortsliste rechts, HSL-Farben, ohne eigene Hand |
| 5 | Ort anlegen: Anordnung | teilweise | heißt „Gebiet anlegen“; Formular in der Hand statt Vorschau in der Schau |
| 6 | Welt erzählen: Tür verbinden | offen (3d) | in Bauen: Werkzeug Tür, Ziel im Inspector |
| 7 | Gelände malen: Route 201 | teilweise (Phase 6) | nur Kollision an/aus, ohne Draufsicht, Palette, Muster |
| 8 | Raster: Matrix 0 | umgesetzt | – |
| 9 | Neue Figur: Bewegung | offen (3d) | Werkzeug Figur, Bewegung als Zahl |
| 10 | Geschichten: Übersicht | offen (3d) | Atlas gesperrt |
| 11 | Geschichte: Graph | offen (3d) | – |
| 12 | Skript: Mutter, Morgen | teilweise | Reiter „Script“ der Welt-Hand, Scripts nach Nummer |
| 13 | Flags und Variablen | teilweise (3d.1: Technik › Flags und Variablen) | liegt bis 3d.6 in Technik statt im Bereich Geschichten; Liste mit Filtern, Detail „Wo gesetzt“ und „Wo geprüft“, Picker und Flag-Feld im Inspector, Dialog „Als eigenes Flag benennen“; „Live“ leer bis 3d.9, Namen nur in der Oberfläche, die Event-JSON tragen bis 3d.4 Zahlen. Die Frames stehen noch nicht in der `.fig` (OpenPencil war nicht erreichbar, `openpencil/NACHTRAG-3D-1.md`) |
| 14 | Dialog bearbeiten | teilweise | über Archiv und Nummer im Bereich Texte, kein Weg von der Figur |
| 15 | Pokédex: Staravia | umgesetzt | DS-Profil seit 3d.3 Teil A (Text, Kategorie, Größe, Gewicht „folgt“); „vorher“ im Wertebalken heißt letzte Speicherung, nicht Original. Den Vergleich mit der Basis-ROM zeigt seit 4.1b nur der Dialog „Auf Original zurücksetzen…“ (teilweise: keine Plakette „geändert“ im Kopf, Nicht in Phase 4) |
| 16 | Entwicklung und Lernset | teilweise | Reiter im Pokédex, keine Kettenansicht; seit 3d.3 Teil B steht der Reiter „Ei-Attacken“ daneben (kein Vorbild in der `.fig`) |
| 17 | Trainer: Wanderer Kai | teilweise | Kampfvorschau und Klassenbild seit 3d.3 Teil A (nur der Gegner); ohne Sichtlinie |
| 18 | Begegnungen: Route 201 | teilweise | Bauen statt Erzählen: Methodenreiter und Tabelle statt Draufsicht und Regler |
| 19 | Attacken: Flügelschlag | umgesetzt | ohne DS-Zusammenfassung und Effektivität |
| 20 | Neuer Trainer: Team | offen (3d) | Dialog „Trainer anlegen…“, dann Reiter Team |
| 21 | Gegenstände: Trank | umgesetzt | ohne Kennzahlkarten und Läden |
| 22 | Läden: Pokémarkt | offen (später) | braucht Patch-Schicht |
| 23 | Texte: Archiv 0412 | umgesetzt | Archivnummern echt (Zweiblattdorf ist in Platin DE 0554) |
| 24 | Spielen: live | offen (Phase 4) | – |
| 25 | Prüfen und Bauen | teilweise | Befundtabelle ja, Build-Schritte nein (Phase 4) |
| 26 | Veröffentlichen | umgesetzt (4.5) | Assistent in vier Schritten, Liste, Titelleiste; `.fig` folgt. Ohne Textexport; die Begleitdatei und die README ersetzen ihn |
| 27 | Technik: Patches und Plugins | teilweise | nur „Dynamische Header“; Plugins Phase 5 |
| 28 | Einstellungen | umgesetzt | Tastenkürzel nur mit ⌘ |
| 29 | Befehlspalette | umgesetzt | – |
| 30 | Welt im dunklen Thema | umgesetzt | – |

**Neu in der App, ohne Design-Screen:**
- Panel „Probleme“ mit Fix-Dialog, Ausblenden und „Ausgangslage übernehmen…“.
- „Teile importieren…“ und „Teile exportieren…“.
- „Trainer anlegen…“, „Begegnungsdatei anlegen…“, „Für diesen Ort abspalten“.
- Assistenten „Raum anlegen“ und „Gebiet erweitern“.
- „Scripts übernehmen“.
- Hilfe mit Glossar.
- Technik › Emulator und Werkzeuge.
- Technik › Änderungen (Fläche; 4.1b, erweitert in 4.4): was im Projekt abweicht, mit dem Umschalter „Gegen Basis | Gegen letzten Commit | Gegen gespeichert“, in Gruppen mit „Auf Original zurücksetzen…“ je Zeile, daneben (schmal darunter) das Detail des gewählten Eintrags mit „Öffnen“ und der Änderungstabelle in der Form seiner Art (Felder, Karte mit Teilen und Zellen der Bewegungsrechte, Nachrichten mit Wort-Spannen, Zeilen, Binär mit Größe und SHA-1). Ohne Repository bietet „Gegen letzten Commit“ „Git einrichten…“ an. Eigene Einordnung: bewusst (Plan 4.4, Entscheidung „Ort in der App“). Es fehlen bewusst (Nicht in Phase 4): Commit-Oberfläche, Staging, Branches, ein Bildvergleich bei Binärteilen; deutsche Feldnamen gibt es nur beim Header, sonst steht der JSON-Pfad. In der `.fig` seit 29.10.2026: „App: Technik, Änderungen gegen letzten Commit“ (x 16720, y 0) und „App: Technik, Änderung einer Nachricht“ (x 18240, y 0) auf der Seite „App: Texte, Technik, Dialoge“ (Vorlage `openpencil/beispiele/aenderungen-git.body.js`).
- Dialog „… auf Original zurücksetzen?“ mit Vorher und Nachher (4.1b), aus dem Menü „Aktionen“ der Köpfe und dem Kontextmenü der Listen; Header Feld für Feld mit Kästchen.
- Technik › Textprojekt (4.3b), mit drei Mustern, die kein Design-Screen kennt: **Änderungen von außen** (Meldung „1 Änderung aus DSPRE übernommen“ mit „Anzeigen“, Abschnitt „Zuletzt übernommen“ mit den Plaketten „aus DSPRE“ und „aus Datei“; abgeglichen beim Öffnen, bei Fokus, vor dem Build und auf Wunsch), **Konflikt** (Zeile mit Plakette „Konflikt“ oder „Ungespeichert“, aufklappbare Feldtabelle Studio / DSPRE, zwei gleichrangige sekundäre Tasten, Warnung vor dem Verwerfen ungespeicherter Änderungen; für DSPRE-Ordner ohne Manifest bleibt der Textdialog von 4.2) und **Textprojekt einrichten** (Dialog mit Prüfliste und Fortschritt in vier Phasen, dazu „Zurücknehmen…“, „Anheben“, der Hinweis beim Öffnen und das Kästchen im Assistenten „Neues Projekt“). Eigene Einordnung: bewusst (Phase 4.3b, Plan „Textprojekt als Quelle“); nicht in der `.fig`, die Vorlage `openpencil/beispiele/textprojekt.body.js` und `openpencil/NACHTRAG-4-3b.md` liegen bereit. Neu in den Bausteinen: die Meldungsart „Warnung“ (bleibt stehen), Meldungen mit Schlüssel, die dauerhafte Info und `Dialog` mit `dismissible`; neues Symbol `textproject`.
- ID-Zeile und „ID umbenennen…“ (4.3b): In einem Textprojekt nennen die Köpfe von Header, Trainer und Pokédex die symbolische ID (`MAP_…`, `TRAINER_…`, `SPECIES_…`) mit „ID kopieren“; Trainer und Header lassen sich im Menü „Aktionen“ umbenennen (Dialog mit Plan der Verweise). Das schließt die zweite Sprachebene zum Teil (siehe 9, Punkt 6): feste IDs sind nicht umbenennbar, Trainerklassen und Orte nur über MCP.
- Git einrichten (4.4): Dialog „Git einrichten?“ mit rechtlichem Hinweis (immer), Prüfliste (gespeichert, Git gefunden, was ignoriert wird, vorhandene Vorlagen) und Fortschritt in fünf Phasen; aus Technik › Änderungen, der Palette (nur ohne Repository), dem Kästchen „Git einrichten (nur lokal)“ im Assistenten „Neues Projekt“ und der Rückfrage eines Agenten. Dazu der Dialog „Textprojekt vervollständigen?“ beim Öffnen eines geklonten Textprojekts. Eigene Einordnung: bewusst (Plan 4.4, nur lokal, kein Remote). In der `.fig` seit 29.10.2026: „App: Dialog, Git einrichten“ (x 15200, y 1000, Seite „App: Texte, Technik, Dialoge“), vor dem Lauf; Lauf, Fehler und „Textprojekt vervollständigen?“ sind nicht gezeichnet (`openpencil/NACHTRAG-4-4.md`).
- Veröffentlichen (4.5): Assistent in vier Schritten (Version, Patch, Prüfen, Fertig) in der Welt-Hand mit Speicherort, Dateiname, Änderungsnotizen, README, Selbstprüfung und „Was du weitergibst“; die Taste „Veröffentlichen“ in der Titelleiste (Tooltip mit Grund, wenn gesperrt); Technik › Veröffentlichen mit der Liste „Bisherige Veröffentlichungen“; Dialog „Stand sichern?“ (ein Commit mit allen Änderungen, lokal, Betreff und Notiz, Prüfliste) aus dem Ergebnis des Assistenten, Technik › Änderungen und der Palette; MCP `publish_patch` und `git_commit_all`. Eigene Einordnung: bewusst (Plan 4.5; kein Remote, kein Push, kein Staging). Die `.fig` folgt (`openpencil/NACHTRAG-4-5.md`).

**Die `.fig` nach dem Nachzug:**
- Die vier bisherigen Screen-Seiten heißen „Zielbild: …“. Sie bleiben der Entwurf für 3d bis 6, und jeder Screen trägt eine Marke Umgesetzt, Teilweise oder Offen.
- Drei neue Seiten „App: …“ zeigen in 26 Screens die App, wie sie am 25.10. aussieht, auf Basis von Screenshots aus der Test-Umgebung:
  - Start und Welt: 10 Screens;
  - Pokémon und Gegenstände: 8 Screens;
  - Texte, Technik, Dialoge: 8 Screens.
- **Titelseite:** Das Brett „Stand der App“ fasst den Stand zusammen.
- **Komponenten:** Unter den Zielbild-Brettern stehen fünf Bretter „Komponenten (App)“ (Rahmen, Scharnier, Prüfen, Eingaben und Inhalt, Daten) mit den Maßen aus den CSS-Modulen. Was fehlt, ist als „Zielbild, noch offen“ markiert.
- **Grundlagen und Muster:** Je ein Brett „Stand der App“.
- **Variablen:** Die 45 neuen Szenen-Tokens sind eingespielt.
- **Werkzeuge:** Bausteine für neue App-Screens in `openpencil/jsx-lib-app.js`, Briefing in `openpencil/BRIEFING-APP-SCREENS.md`.

## 8. Beobachtungen aus dem Durchlauf

Im Durchlauf vom 25.10.2026 (Platin DE, Test-Umgebung) fiel auf:

- **Fix-Dialog „Umbrechen“** (Textarchiv 0034, Nachricht 0, Zeile 1): Die Zeile besteht aus 58 Leerzeichen (232 px). Die Prüfung meldet sie als „Zeile zu breit“, und `plan_fix` macht daraus einen leeren Text. Der Dialog zeigt die Leerzeichen nicht, deshalb wirkt „Vorher“ leer und „Nachher“ sagt „Leer“; man sieht nicht, was der Fix ändert. Vorschlag:
  - Leerzeichen in `BeforeAfter` sichtbar machen;
  - Zeilen aus reinem Leerraum nicht als zu breit melden, oder dafür einen eigenen Fix „Leerraum entfernen“ anbieten.
- **3D-Ansicht:** Nach der Wahl eines Orts zeigt die Kamera das ganze Raster 0 von weit oben, auch wenn eine Figur gewählt ist. Der Ort ist kaum zu erkennen.
- **Technik › Prüfen und Bauen:** Bei 900 px Fensterhöhe drückt die lange Befundliste den Abschnitt „Prüfungen ein- und ausschalten“ auf wenige Pixel zusammen; er ist kaum zu treffen. In der `.fig` ist er als eingeklappter Abschnitt gezeichnet.
- **Die Hand ist bei 900 px Fensterhöhe knapp.** Der Inspector zeigt sechs Zeilen; „Script“, „Trainer“ und „Sichtlinie“ liegen außer Sicht.

## 9. Offene Entscheidungen

Was das Design-System als Nächstes klären sollte, nach Wirkung sortiert:

1. **Reiter vereinheitlichen.** Kobalt oben oder unten, eine Form für Dockview und `Tabs`.
2. **Mittelpunkt-Regel.** Entweder erlauben, als Trenner in Orten und Statusleiste, und die Regel im README ändern; oder in der App durch Kommas ersetzen.
3. **Schatten, Ebenen und Syntaxfarben als DTCG-Tokens.** Dann sind sie in OpenPencil pflegbar und vom Gate geprüft.
4. **Ortsfarben auf RGB555.** Eine feste Palette aus den Bereichs- oder Szenenfarben statt HSL.
5. **Tabellenziffern global wirksam machen** und Mono 600 laden oder nicht benutzen. Zen Maru 900 nicht laden.
6. **Zweite Sprachebene schließen.** Klarnamen für Räume und Events, Picker für Grafik und Bewegung. Die symbolischen Kennungen kommen mit dem Textprojekt (Phase 4). Die IDs stehen seit 4.3b in den Köpfen eines Textprojekts (Header, Trainer, Pokédex); Klarnamen und Picker fehlen weiter.
7. **Kamera auf den gewählten Ort** setzen, wenn ein Ort oder ein Objekt gewählt wird.
