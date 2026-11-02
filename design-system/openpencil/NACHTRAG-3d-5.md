# Nachtrag 3d.5 für die OpenPencil-Datei

**Die Datei wurde nicht angefasst.** Phase 3d.5 „Figur, Dialog, Tür“ (02.11.2026) hat Code, README und Audit nachgezogen; `design-system/Editor_DesignSystem.fig` kennt die neuen Zustände noch nicht, die Frames folgen. OpenPencil war beim Schreiben nicht verbunden (die MCP-Verbindung scheiterte mit einem 502), darum ist nichts gerendert. Dieser Nachtrag ist die Anleitung für die Sitzung, die die Datei danach anfasst; die Vorlage `beispiele/figur-dialog-tuer.body.js` liegt bereit, ihre Syntax ist geprüft, sie lässt sich ohne OpenPencil bauen.

Was die App zeigt, beschreibt `design-system/README.md`, Abschnitt „Nachtrag 3d.5“. Texte und Zustände stammen von dort und aus dem Code: `apps/studio/src/workbench/inspector/{DialogSection,WarpField,EventEditor}.tsx`, `workbench/dialog/{DialogEditor,useDialog,dialogModel}`, `workbench/wizards/{NewFigureWizard,ConnectDoorsWizard,NewTrainerHereWizard,PlaceField,TextPreview,WizardResult}.tsx`, `workbench/recipes.ts`, `world/scene/draft.ts`, `ui/primitives/Dialog.tsx`. Projekt „Sinnoh-Remix“, Platin DE, wie die übrigen App-Screens. Beispielwerte, keine Spieldaten, sind:
- die Texte der Figur („Willkommen in Zweiblattdorf!“, „Hier beginnt deine Reise.“, „Pass auf dich auf!“),
- Archiv 554 mit den Nachrichten 4 und 5, das Aussehen 148 der Figur und Aussehen 6 der neuen,
- die Lage der geplanten Kachel in der Schau und die Kacheln x 106, z 874.

Vergleichsquelle sind die Screenshots der Test-Umgebung unter `target/agent/gate-2026-11-02/3d-5/` (1440 × 900, nicht im Repo). Sie entstehen im UI-Test dieser Phase; die Liste mit Namen steht im Gate-Bericht (`docs/phase-3d-5-gate.md`). Die Namen sind `blick-{figur,dialog,dialog-geteilt,tuer,trainer}-{hell,dunkel,kompakt}.png`. Der Plan sah `target/agent/3d-5/` vor; maßgeblich ist der Ordner des Gate-Laufs.

Neue Tokens und neue Symbole gibt es nicht. Die Plakette „Geteilt mit N Figuren“ ist `Badge(…, 'warn', 'ph:share-network')` (`status/warnung`); das Symbol `shared` trägt schon die Plakette „Geteilt mit N Räumen“ (3f.4, Art Info). Die Rezept-Taste, die Leiste, die Segmente „Tiefe“ und der Entwurf sind die von 3d.2 (`openpencil/NACHTRAG-3d-2.md`); neu ist nur, dass keine Taste der Leiste mehr gesperrt ist. `ph:text-aa` (`texts`), `ph:chats-circle` (`stories`), `ph:play` (`play`), `ph:door`, `ph:paw-print` und `ph:person-simple-walk` gibt es schon.

## Was neu zu zeichnen ist

Zwei Frames je 1440 × 900 und ein Brett, alle aus `beispiele/figur-dialog-tuer.body.js` (`MODE` wählt). Diese Teile gibt es schon:
- Rahmen, Atlas, Liste „Welt“ und Statusleiste,
- das Dock der Werkbank mit Schau, Script und Probleme und Inspector,
- das Scharnier mit den Segmenten „Tiefe“ und die Rezept-Leiste (die Vorlage zeichnet beide wie `erzaehlen-rahmen.body.js`, die Leiste jetzt mit fünf freien Tasten),
- die Schrittleiste des Assistenten (`WizardBar`), die Hand mit Spalten (`WizardCol`), `DialogX` und `MitDialog` (Dialog über abgedunkeltem Fenster),
- die Bausteine Plakette, Meldung, Referenz, Zeile, Segmente, DS-Bildschirm (`DSScreen`) und Schau-Platzhalter (`Welt3D`).

Neu sind:
- das Feld „Dialog“ im Inspector,
- das Modal „Dialog von Objekt 2“ (960 breit),
- die Plakette „Geteilt mit N Figuren“,
- die Hand des Assistenten „Neue Figur“ mit der Vorschau,
- der Entwurf mit Sprite,
- die Zeile „Gegenstück“.

1. **Frame „App: Welt, Erzählen, Dialog-Editor“** (`MODE = 'dialog'`).
   - **Hintergrund:** die Welt in Erzählen wie in 3d.2 (Scharnier mit Erzählen gewählt, Rezept-Leiste in der Schau, Inspector „Figur“ / „Objekt 2 · ID 2“ mit „Aussehen“, „Bewegung“, „Blickrichtung“, „Trainer“) und darunter im Inspector das Feld „Dialog“: Überschrift „Dialog“ (`text/xs` sekundär), der DS-Bildschirm mit der ersten Seite („Willkommen in Zweiblattdorf!“, „Hier beginnt deine Reise.“), die Zeile „2 Nachrichten · 3 Seiten“ und die leise Taste „Dialog bearbeiten…“. Das Feld ist durch eine Linie `linie/leise` oben abgesetzt.
   - **Abdunklung und Modal:** `flaeche/ueberlagerung` über dem ganzen Fenster, in der Mitte das Modal (960 breit, `flaeche/blatt`, Kante `linie/standard`, `radius/fenster`, Schatten schwebend, Innenabstand 20). Titel „Dialog von Objekt 2“ (18, 600), rechts oben das Schließen-Kreuz, unten rechts „Schließen“ (primär).
   - **Spalten:** links „Nachrichten“ (zwei Karten), rechts die DS-Seite.
     - Karte 1 „Nachricht 4 · Archiv 554“ (gewählt, Kante `linie/stark`) mit dem Nachrichten-Feld (Mono 12, die zwei Zeilen).
     - Karte 2 „Nachricht 5 · Archiv 554“ mit der Plakette „Geteilt mit 2 Figuren“ (Warnung, Symbol `shared`), dem Feld („Pass auf dich auf!“) und der sekundären Taste „Für diese Figur abspalten“.
     - Rechts die Beschriftung „Nachricht 4“, der DS-Bildschirm in Maßstab 1 (256 × 192) und der Blätterer ‹ „Seite 1 von 2“ › (‹ gedämpft auf Seite 1).
2. **Frame „App: Welt, Erzählen, Neue Figur“** (`MODE = 'figur'`).
   - **Scharnier:** die Schrittleiste „Neue Figur“ (Chip Welt, Kante `bereich/welt-punkt`): „Wo und wie, Dialog, Zusammenfassung“, Schritt 2, „Zurück ⌘[“, „Weiter ↵“.
   - **Schau:** 3D mit dem Entwurf: ein 1 × 1 Kachelrahmen gestrichelt in `auswahl/halo` bei 50 % und darauf der Sprite der Figur halbtransparent (50 %), auf der Kachel stehend (die Vorlage nimmt die Kachel `Sprite('F', 'welt', 28)`; in der App ist es der Overworld-Sprite). Der Inspector zeigt das Gebiet mit dem Abschnitt „Ort“.
   - **Dock:** wie im Frame „Assistent: Ort“ von 3f.3 (`assistent-ort.body.js`): unter dem Streifen des Assistenten (286) bleiben 502; Schau-Gruppe und „Script und Probleme“ im Verhältnis 548 : 788.
   - **Hand (Schritt „Dialog“):** links „Was sie sagt“ (das Nachrichten-Feld mit dem Text), die Notiz „Enter: neue Zeile; Umschalt+Enter: neue Seite (▼).“ (`text/xs` sekundär), die Zeile „Sichtbarkeits-Flag“ („Keins“, „Wählen…“) mit dem Hinweis; rechts „Vorschau“ mit dem DS-Bildschirm und „Seite 1 von 1“. Im Fuß die Zeile „Neue Figur in „Zweiblattdorf“, Kachel x 106, z 874“, **ohne** die Plakette „Nicht rückgängig“ (die Figur lässt sich zurücknehmen; nur „Neuer Trainer hier“ trägt sie). Der DS-Bildschirm steht in der Vorlage in Maßstab 0,75, weil die Hand in 286 px nicht mehr Platz hat; die App zeigt Maßstab 1 und rollt.
3. **Brett „Komponenten (App): Plakette Geteilt, Feld Dialog, Tür-Gegenstück“** (`MODE = 'brett'`), für die Seite „Komponenten“. Es zeigt fünf Teile:
   - die Plakette „Geteilt mit N Figuren“ (eine, zwei und fünf Figuren; „Geteilt mit 1 Figur“ im Singular);
   - das Feld „Dialog“ in fünf Zuständen: lokal (Vorschau, „2 Nachrichten · 3 Seiten“, „Dialog bearbeiten…“), gemeinsam („Gemeinsames Script 2005: in Bauen bearbeiten.“), Trainer („Kampftexte des Trainers: im Bereich Pokémon.“), nichts („Sagt nichts“, „Dialog anlegen…“ gesperrt, „Kommt mit 3d.7“), nicht lesbar („Script nicht lesbar: …“), in der Vorlage in kleinerem Maßstab;
   - die Zeile „Gegenstück“ in drei Zuständen („… · führt zurück“, „… · führt nicht zurück“, „Ziel hat keine Tür 7“), je mit der leisen Taste „Tür verbinden…“;
   - das Ergebnis „Als Nächstes“ nach „Neue Figur“ mit „Dialog bearbeiten“ (läuft) und „Zu einer Geschichte“ (gesperrt, „Kommt mit 3d.7“);
   - die Sätze zu den Ergebnissen von „Tür verbinden“ und „Neuer Trainer hier“ („Hier spielen“ gesperrt, „Kommt mit 3d.9“).

## Lage und Einspielen

- **Lage:**
  - Die Frames 1 und 2 kommen auf die Seite „App: Werkbank“ in die nächsten freien Spalten, Frame 2 rechts neben Frame 1 (Reihe der Welt-Screens). Es gilt x = Spalte × 1520, y = Reihe × 1000.
  - Das Brett kommt auf die Seite „Komponenten“ unter das letzte Brett „Komponenten (App)“ (zuletzt 4.4: „Änderungstabelle und Git“ bei x 3440, y 15380; ob später etwas dazukam, zeigt der Baum).
  - Vorher mit `get_page_tree` (depth 1) die belegten Positionen ablesen und nur eigene Frames anfassen. Die Lage steht danach hier und im README, Abschnitt „Nachtrag 3d.5“.
- **Bauen:**
  1. Aus `openpencil/beispiele/` `bash ../build-screen.sh figur-dialog-tuer` aufrufen (für Dunkel mit `--dunkel`). Das Skript schreibt nur die `.jsx` neben die Vorlage und braucht OpenPencil nicht.
  2. In der erzeugten `figur-dialog-tuer.jsx` `MODE` je Frame umstellen (`'dialog'`, `'figur'`, `'brett'`).
  3. `node ../../scripts/op.mjs render '{…}' --jsx figur-dialog-tuer.jsx` aufrufen (`op.mjs render` mit `x`, `y` und der Seite, siehe die früheren Nachträge).
  4. Per `export_image` vergleichen (`node ../../scripts/op.mjs export_image '{…}' --png bild.png`), mit den Screenshots aus `target/agent/gate-2026-11-02/3d-5/`: Frame 1 mit `blick-dialog-geteilt-*` (und `blick-dialog-*`), Frame 2 mit `blick-figur-*`, das Brett mit `blick-figur-*` (Feld „Dialog“), `blick-tuer-*` (Zeile „Gegenstück“) und `blick-trainer-*` (Ergebnis). Dunkel und Kompakt mit den Bildern `-dunkel` und `-kompakt` derselben Namen.

     Die Screenshots zeigen echte Werte des Testprojekts, die Frames Beispielwerte. Verglichen werden Aufbau, Maße und Texte. Atkinson Hyperlegible Mono ist in OpenPencil ersetzt (Inter), darum sind Spannen und das Nachrichten-Feld in den Bildern breiter als ihr Text. Pixelify Sans im DS-Bildschirm kann ebenfalls ersetzt sein.
  5. Korrigieren: den Body ändern, neu bauen, `render` mit `"replace_id":"<ID>"` statt `x`/`y` (höchstens drei Läufe je Frame). Die erzeugten `.jsx` und `.png` nicht einchecken, sondern löschen.
- **Prüfen ohne OpenPencil:** `.jsx` und Vorlage enthalten JSX, `node --check` auf die Vorlage allein scheitert deshalb an `<`. Die Syntax lässt sich prüfen, indem man jsx-lib-app.js (ohne Kommentarzeilen) und den Body in eine Funktion legt, mit dem TypeScript-Compiler des Studios (`apps/studio/node_modules/typescript`, `ts.transpileModule` mit `jsx: React`) übersetzt und das Ergebnis mit `node --check` prüft; so ist die Vorlage geprüft worden.
- **Danach:**
  1. Speichern (der Hauptagent, `save_file`) und das Dokument schließen.
  2. Aus der Repo-Wurzel `node design-system/scripts/fix_fig_text.mjs design-system/Editor_DesignSystem.fig --rgb555` aufrufen.
  3. Im README den Abschnitt „Nachtrag 3d.5“ um einen Satz ergänzen, dass die Frames seit … in der Datei stehen (mit Seite und Lage). Den Satz „Die Frames von 3d.5 folgen, sobald OpenPencil antwortet“ dort (Abschnitt und Abweichungen), „Die Datei wurde nicht angefasst“ hier und die Wörter „die `.fig` kennt die Frames noch nicht“ in `Audit-Design-System.md` (Zeilen „Feld Dialog“, „Dialog-Editor“ und Screens 6, 9, 14, 20) ersetzen.
  4. Im Brett „Stand der App“ (Seite „Titel“, x 1720, y 0, Vorlage `beispiele/stand-der-app.body.js`, per `render` mit `replace_id`) eine Zeile „Figur, Dialog, Tür (3d.5)“ unter „Umgesetzt“ anlegen, mit dem Verweis auf die beiden Frames, und die Zeile „Erzählen-Rahmen (3d.2)“ (drei Tasten gesperrt) anpassen.
  5. Im Brett „Komponenten (App)“ keine Symbole nachtragen (es sind keine neuen dabei).
- **Aus dem Gedächtnis (OpenPencil-Grenzen):**
  - Die Datei hat die Grenze von 10 000 Knoten; sie blockiert einzelne Knotenänderungen. Keine Einzeländerungen am Knoten versuchen, sondern den Frame neu rendern (`render` mit `replace_id`).
  - Ist der Bildschirm gesperrt, laufen die Aufrufe in Zeitüberschreitungen. Den Bildschirm entsperrt halten und den Lauf wiederholen.
  - OpenPencil lehnt Pfade unter `.claude/` ab: auf einer Kopie unter `gen4-studio/target/agent/` der Hauptkopie rendern, speichern, in den Worktree zurückkopieren.

  `check-tokens.sh` ist nicht betroffen.

## Was nicht gezeichnet werden muss

- **Weitere Zustände des Dialog-Editors:** „Lese …“, der Fehler des Kerns, die Info bei einer Quelle, die er nicht bearbeitet, „Das Script dieser Figur zeigt keine Nachricht.“, die Meldung „Nachricht 7 ist jetzt nur diese Figur.“ (Erfolg) über der Liste und der Dialog eines Schilds (DS-Bildschirm in den Farben des Schilds, ohne „Für diese Figur abspalten“). Es sind dieselben Teile mit anderem Inhalt.
- **Die übrigen Schritte der Assistenten:**
  - „Neue Figur“, „Wo und wie“: Ortsfeld (Chip der Fläche Welt, „Ort wählen…“), „Kachel x“, „Kachel z“, „Aussehen“, „Bewegung“, „Blickrichtung“ als Segmente (Oben, Unten, Links, Rechts).
  - „Tür verbinden“: „Tür A“ und „Tür B“ mit den Segmenten „Vorhandene Tür“ und „Neue Kachel“, „Tür wählen…“, dem Abschnitt „Verbindung“ mit zwei Kästchen und den beiden Warnungen.
  - „Neuer Trainer hier“: fünf Schritte „Trainer“, „Team“ (Pokémon-Karten, „Art wählen…“, „Level“, „Entfernen“, „Pokémon hinzufügen“), „Texte“ (drei Nachrichten-Felder), „Wo“ (mit „Sichtweite“), „Zusammenfassung“ mit der Plakette „Nicht rückgängig“ (`data-trainer-barrier`).
  - Alle Zusammenfassungen sind eine Info-Meldung mit drei oder vier Zeilen und den Hinweisen des Entwurfs (`Vorschau nicht möglich: …` als Warnung); sie folgen dem Frame „Entwurf Raum anlegen“ von 3d.2.
- **Ergebnisse der drei Assistenten:** Erfolgsmeldung („Figur aufgestellt“, „Türen verbunden“, „Trainer aufgestellt“), „Als Nächstes“ mit den Tasten und „Zeigen“ (primär), „Schließen“. Das Brett zeigt die Ergebnisse von „Neue Figur“; die übrigen beiden unterscheiden sich im Text und der gesperrten Taste „Hier spielen“.
- **Der Entwurf von „Tür verbinden“ und „Neuer Trainer hier“:** dieselben gestrichelten Kachelrahmen und halbtransparenten Marker wie der der Figur.
- **Weitere Zustände der Rezept-Leiste:** keine; alle fünf Tasten laufen, die Rezept-Taste in Ruhe, Hover, Aktiv und Gesperrt ist das Brett von 3d.2.
- **Dunkel und Kompakt:** Die Bausteine haben ihre Zustände schon in der Datei. Frame 1 und 2 lassen sich mit `--dunkel` bauen.
