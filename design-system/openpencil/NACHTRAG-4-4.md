# Nachtrag 4.4 für die OpenPencil-Datei

**Eingespielt am 29.10.2026** (Branch `fig-nachtraege-4-4`, auf dem Stand der `.fig` von main `abc4184` mit den Frames von 4.3b). Auf der Seite „App: Texte, Technik, Dialoge“ stehen „App: Technik, Änderungen gegen letzten Commit“ (x 16720, y 0) und „App: Technik, Änderung einer Nachricht“ (x 18240, y 0, beide Reihe 0), dazu „App: Dialog, Git einrichten“ (x 15200, y 1000, Reihe 1). Auf der Seite „Komponenten“ steht das Brett „Komponenten (App): Änderungstabelle und Git“ (x 3440, y 15380, unter „Build-Schritte“). Das Brett „Stand der App“ (Seite „Titel“, x 1720, y 0) ist neu gerendert aus `beispiele/stand-der-app.body.js`, mit der Zeile „Git minimal (4.4)“ unter „Teilweise umgesetzt“ (19 Screens). Nicht nachgezogen: das Symbol `git` in der Symbolreihe des Bretts „Komponenten (App)“ (dafür gibt es keine Vorlage, wie bei `textproject` aus 4.3b); Frame 2 in Dunkel ist nicht als eigener Frame gezeichnet. Gerendert auf einer Kopie unter `gen4-studio/target/agent/fig-4-4/` der Hauptkopie (OpenPencil lehnt Pfade unter `.claude/` ab), gespeichert, in den Worktree kopiert, `fix_fig_text.mjs --rgb555` (320 Textknoten, 35 554 Farben), wieder geöffnet und als Bild geprüft (Kontrollbilder `target/agent/gate-2026-10-29/fig-4-4/fig-*.png` im Worktree, nicht im Repo). Verglichen mit den Screenshots unter `target/agent/gate-2026-10-29/4-4/`: `blick-aenderungen-commit-trainer-hell.png`, `blick-klon-commit-text-hell.png`, `blick-git-init-dialog-vorher-hell.png` und `ui-03-statusleiste-tooltip.png`. Korrekturen an der Vorlage nach der App: Der Satz unter dem Titel ist höchstens 64ch breit (rund 520 statt 600), ebenso der Satz ohne Repository. Die Warnung im Dialog bricht um, denn `Notice` aus `jsx-lib-app.js` gibt dem Text keine Breite. Vorher lief sie über den Dialog hinaus. Atkinson Hyperlegible Mono ist in OpenPencil ersetzt (Inter), darum sind die Spannen in den Bildern breiter als ihr Text; in der Datei steht die Mono-Schrift. Stand: 29.10.2026. Diese Anleitung sagt, was zu Phase 4.4 „Git minimal“ neu in `Editor_DesignSystem.fig` gehört: Technik › Änderungen mit Umschalter und Detail, der Dialog „Git einrichten?“, das Git-Element der Statusleiste und die Änderungstabelle. Bei 4.4 ist der Schritt „`.fig`“ auf Anweisung der Sitzung entfallen: Eine eigene `.fig`-Sitzung spielt nach dem Merge erst die Frames von 4.3b ([`NACHTRAG-4-3b.md`](NACHTRAG-4-3b.md)) und dann die von 4.4 ein. Darum liegen nur die Vorlage `beispiele/aenderungen-git.body.js` und diese Anleitung bereit. Wer die Frames einspielt, trägt hier „Eingespielt am …“ mit Seite und Lage nach, so wie `NACHTRAG-4-1b.md` und `NACHTRAG-4-2.md`.

Was die App zeigt, beschreibt `design-system/README.md`, Abschnitt „Nachtrag 4.4“. Texte und Zustände stammen von dort und aus dem Code: `apps/studio/src/technik/{ChangesSurface,EntityDiff}.tsx`, `technik/diffWords.ts`, `ui/primitives/ChangeTable.tsx`, `ui/frame/GitStatusItem.tsx`, `git/{GitInitDialog.tsx,gitWords.ts}`. Projekt „Sinnoh-Remix“, Platin DE, Basis-ROM „Platin DE (Testprojekt)“, wie die übrigen App-Screens. Beispielwerte, keine Spieldaten, sind:
- der Commit `4c1d2e3` „Ausgangsstand“,
- die drei Felder des Trainers,
- die beiden Nachrichten des Textarchivs,
- die Git-Version.

Vergleichsquelle sind die Screenshots der Test-Umgebung unter `target/agent/gate-2026-10-29/4-4/`: `blick-<fläche>-<zustand>-<thema>.png`, 1440 × 900, nicht im Repo. Die Liste mit Beschreibung steht im README, Abschnitt „Nachtrag 4.4“. Die Bilder entstanden vor der Korrekturrunde nach dem UI-Test, darum weichen drei Stellen ab:
- Die Taste „Git einrichten…“ ist dort so breit wie die Fläche.
- Die drei Spalten der Änderungstabelle sind dort gleich breit, und der Pfad bricht mitten im Wort („data.party[0].leve“ / „l“).
- Die Prüfliste sagt dort „Git gefunden: git version 2.54.0 (Apple Git-157)“.

Die Frames folgen dem Code, nicht diesen Stellen. Für Dunkel gilt dazu: Die Flächen der Wort-Spannen sind in Dunkel kaum vom Grund der Karte zu unterscheiden (`status/erfolg-flaeche` gegen `flaeche/blatt` 1,23 : 1, `status/fehler-flaeche` 1,35 : 1). Es tragen die Unter- und Durchstreichung. Ob die Flächen kräftiger werden, entscheidet der Nutzer am Bild (README, „Blick“).

Neue Tokens gibt es nicht. Neu ist ein Symbol: `git` (Phosphor `GitBranch`, in der Vorlage `ph:git-branch`) für das Element der Statusleiste und den Eintrag „Git einrichten…“ der Palette. Technik › Änderungen behält `changes` (`ph:git-diff`).

## Was neu zu zeichnen ist

Drei Frames je 1440 × 900 und ein Brett, alle aus `beispiele/aenderungen-git.body.js` (`MODE` wählt). Diese Teile gibt es schon:
- Rahmen, Atlas und Statusleiste,
- die Liste „Technik“ mit acht Einträgen (seit 4.3b),
- die Bausteine Dialog, Segmente, Abschnitt, Plakette, Meldung und Prüfliste.

Neu sind:
- der Umschalter über der Liste,
- die Aufteilung Liste und Detail,
- das Detail mit Kopf und Änderungstabelle,
- das Git-Element in der Statusleiste,
- der Inhalt des Dialogs.

1. **Frame „App: Technik, Änderungen gegen letzten Commit“** (`MODE = 'commit'`).
   - **Liste „Technik“:** acht Einträge. „Änderungen“ ist gewählt (Symbol `ph:git-diff`, Untertitel „2 Änderungen seit dem Commit“), „Textprojekt“ trägt „DSPRE-Layout“.
   - **Fläche „Änderungen“:** Titel `font/titel-s`, Fläche `flaeche/deck`, innen 32 / 24, Kopf in der Lesebreite 880. Darunter der Satz „Gegenüber dem letzten Commit 4c1d2e3 „Ausgangsstand“ auf main. Gezählt wird der gespeicherte Stand.“, rechts die sekundäre Taste „Neu vergleichen“.
   - **Umschalter:** darunter die Segmente der Größe S „Gegen Basis | Gegen letzten Commit | Gegen gespeichert“, gewählt ist „Gegen letzten Commit“.
   - **Liste und Detail:** darunter nebeneinander (die Aufteilung ist bis 1280 breit und hier 1032; je Spalte 508, Abstand 16).
     - Links die Karte der Gruppen (`flaeche/blatt`, Kante `linie/standard`, `radius/fenster`) mit „Textarchive“ (1, „Textarchiv 0554“) und „Trainer“ (1, „Trainer 0001“, gewählt). Je Zeile die Plakette „Geändert“ (Warnung) und die kleine sekundäre Taste „Auf Original zurücksetzen…“.
     - Rechts das Detail, eine Karte wie links, innen 16, Abstand 12. Kopf: „Trainer 0001“ (`text/l`), darunter „Commit 4c1d2e3 „Ausgangsstand“ → Projekt (gespeichert)“ (`text/s` sekundär), rechts die kleinen sekundären Tasten „Öffnen“ und „Auf Original zurücksetzen…“.
     - Darunter die Änderungstabelle „Feld | Commit | Projekt“ mit drei Zeilen: `data.party[0].level` 5 → 7, `data.party[0].species` SPECIES_STARLY → SPECIES_SHINX, `data.party[0].item` „—“ → ITEM_ORAN_BERRY. Die Feldspalte ist so breit wie der längste Pfad, die beiden Seiten teilen den Rest.
   - **Statusleiste:** „101 Warnungen, 10 Hinweise“, dann das Git-Element „main · 2 Änderungen“ (Symbol `ph:git-branch`, `text/xs`, eine leise Taste der Größe S), dann „Gebaut vor 3 Minuten“ und „Komfort“.
2. **Frame „App: Technik, Änderung einer Nachricht“** (`MODE = 'text'`). Dieselbe Fläche, gewählt ist „Textarchiv 0554“. Das Detail zeigt die Tabelle „Nachricht | Commit | Projekt“ mit zwei Zeilen:
   - Nachricht 3, geändert: alt „Der Laden hat heute zu.“ mit „zu“ als entfernter Spanne, neu „Der Laden hat heute bis acht offen.“ mit „bis acht offen“ als dazugekommener Spanne. Die entfernte Spanne liegt auf `status/fehler-flaeche` und ist durchgestrichen, die dazugekommene liegt auf `status/erfolg-flaeche` und ist unterstrichen. Die Spannen haben den Radius `radius/plakette`.
   - Nachricht 7 mit der Plakette „Neu“ (Info): alt „—“, neu „Komm bald wieder!“ ganz als dazugekommene Spanne.

   Die Zeichnung bricht die Zeilen von Hand und zieht die Linie der Spannen als 1-px-Rahmen in `text/primaer`, weil die Vorlage keine Textauszeichnung kennt. In der App fließt der Text, und die Linie ist `text-decoration`.
3. **Frame „App: Dialog, Git einrichten“** (`MODE = 'einrichten'`).
   - **Hintergrund:** Technik › Änderungen auf „Gegen letzten Commit“ ohne Repository: kein Satz unter dem Titel, keine Taste daneben. Darunter der Umschalter, die Info „Dieses Projekt liegt in keinem Git-Repository.“, der Satz „Git einrichten legt ein lokales Repository an; nur lokal oder privat, Gen4 Studio lädt nichts hoch.“ und die sekundäre Taste „Git einrichten…“ in ihrer eigenen Breite. Der Untertitel in der Liste heißt „Kein Git“, die Statusleiste hat kein Git-Element.
   - **Dialog:** darüber die Abdunklung (`flaeche/ueberlagerung`) und der Dialog, 480 breit:
     - Titel „Git einrichten?“, Beschreibung „Legt in diesem Projektordner ein lokales Git-Repository an: .gitignore und .gitattributes, dann einen ersten Commit „Ausgangsstand“. Gen4 Studio richtet kein Remote ein und lädt nichts hoch.“
     - Die Warnung „Dein Projekt enthält Daten aus dem Spiel. Lade es nicht öffentlich hoch; zum Teilen gibt es den Patch (Technik › Veröffentlichen).“ (steht immer da).
     - Die Prüfliste mit vier Zeilen: Haken „Alles gespeichert“, Haken „Git gefunden: 2.54.0 (Apple Git-157)“, Info „Ignoriert: ROMs, Patches, build/, Spielstände und die Archive, die der Build packt“, Info „.gitignore vorhanden: der Block von Gen4 Studio kommt dazu“.
     - Tasten „Abbrechen“ und „Einrichten“ (primär), dazu das Kreuz.
4. **Brett „Komponenten (App): Änderungstabelle und Git“** (`MODE = 'brett'`), für die Seite „Komponenten“. Es zeigt vier Zustände:
   - die Änderungstabelle mit Feldern, einer fehlenden Seite und der Fußzeile „und 3 weitere Felder“;
   - dieselbe für einen Header mit deutschen Feldnamen („Musik Tag“, „Wetter“; der Pfad steht im Tooltip);
   - die Nachrichten mit Wort-Spannen;
   - das Git-Element auf einem Stück Statusleiste in vier Zuständen, je mit seinem Tooltip als Satz daneben: „main · 3 Änderungen“, „main · noch kein Commit“, „4c1d2e3 (losgelöst) · keine Änderung“ und „Git: nicht lesbar“ (Symbol und Text in `status/warnung`, der Grund als Tooltip). Ohne Repository steht kein Element da.

## Lage und Einspielen

- **Lage:**
  - Die Frames 1 und 2 kommen auf die Seite „App: Texte, Technik, Dialoge“ in die nächsten freien Spalten der Reihe der Technik-Screens (Reihe 0), Frame 3 in die nächste freie Spalte der Reihe der Dialoge (Reihe 1). Es gilt x = Spalte × 1520, y = Reihe × 1000.
  - Das Brett kommt auf die Seite „Komponenten“ unter das letzte Brett „Komponenten (App)“ (4.2: „Build-Schritte“ bei x 3440, y 14440).
  - Vorher mit `get_page_tree` (depth 1) die belegten Positionen ablesen und nur eigene Frames anfassen. Die Frames von 4.3b kommen zuerst; ihre Lage steht dann in `NACHTRAG-4-3b.md`.
- **Bauen:**
  1. Aus `openpencil/beispiele/` `bash ../build-screen.sh aenderungen-git` aufrufen (für Dunkel mit `--dunkel`). Das Skript schreibt nur die `.jsx` neben die Vorlage und braucht OpenPencil nicht.
  2. In der erzeugten `aenderungen-git.jsx` `MODE` je Frame umstellen (`'commit'`, `'text'`, `'einrichten'`, `'brett'`).
  3. `node ../../scripts/op.mjs render '{…}' --jsx aenderungen-git.jsx` aufrufen.
  4. Per `export_image` vergleichen (`node ../../scripts/op.mjs export_image '{…}' --png bild.png`), mit den Screenshots aus `target/agent/gate-2026-10-29/4-4/`:
     - Frame 1 mit `blick-aenderungen-commit-trainer-hell.png`; für die Statusleiste mit `ui-05-statusleiste-1-aenderung.png`, für Liste und Detail auch mit `blick-aenderungen-gespeichert-trainer-hell.png` und `blick-aenderungen-basis-trainer-hell.png`.
     - Frame 2 mit `blick-aenderungen-commit-textarchiv-hell.png` und `blick-klon-commit-text-hell.png`; Dunkel mit `blick-klon-commit-text-dunkel.png` (dunkel gebaut vergleichen).
     - Frame 3 mit `blick-git-init-dialog-vorher-hell.png`, für den Hintergrund mit `blick-aenderungen-commit-ohne-repo-hell.png`.
     - Das Brett mit dem Block „Änderungen“ der Galerie-Seite „Inhalt und Status“ und mit `ui-03-statusleiste-tooltip.png`.
     - Für Dunkel und Kompakt die Bilder `-dunkel` und `-kompakt` derselben Namen.

     Die Screenshots zeigen echte Werte des Testprojekts, die Frames Beispielwerte. Verglichen werden Aufbau, Maße und Texte.
  5. Korrigieren: den Body ändern, neu bauen, `render` mit `"replace_id":"<ID>"` statt `x`/`y` (höchstens drei Läufe je Frame). Die erzeugten `.jsx` und `.png` nicht einchecken, sondern löschen.
- **Danach:**
  1. Speichern (der Hauptagent, `save_file`) und das Dokument schließen.
  2. Aus der Repo-Wurzel `node design-system/scripts/fix_fig_text.mjs design-system/Editor_DesignSystem.fig --rgb555` aufrufen.
  3. Im README den Abschnitt „Nachtrag 4.4“ um einen Satz ergänzen, dass die Frames seit … in der Datei stehen (mit Seite und Lage). Den Satz „die `.fig` kennt das noch nicht“ dort, „Die Datei wurde nicht angefasst“ hier und die Zeilen in `Audit-Design-System.md` ersetzen.
  4. Im Brett „Stand der App“ (Seite „Titel“, x 1720, y 0, Vorlage `beispiele/stand-der-app.body.js`, per `render` mit `replace_id`) eine Zeile „Git minimal (4.4)“ unter „Teilweise umgesetzt“ anlegen, mit dem Verweis auf die drei Frames. Ein Zielbild dazu gibt es nicht.
  5. Im Brett „Komponenten (App)“ das Symbol `git` in die Symbolreihe nehmen.

  `check-tokens.sh` ist nicht betroffen.

## Was nicht gezeichnet werden muss

- **Fortschritt:**
  - „Git einrichten?“ im Lauf: dieselbe Prüfliste ohne Tasten und Kreuz, darunter die Phase in `text/s` sekundär („Archive auspacken …“, „Vorlagen schreiben …“, „Repository anlegen …“, „Dateien aufnehmen … (bei DSPRE-Ordnern rund 10 s)“, „Erster Commit …“). Escape schließt nicht. Der Aufbau ist derselbe wie im Frame „App: Dialog, Als Textprojekt einrichten“ von 4.3b.
  - „Lade Vergleich …“ im Detail.
  - „Wird gelesen …“ und „Vergleiche …“ auf der Fläche.
- **Fehler:**
  - die Fehlermeldung im Detail;
  - Git nicht lesbar: die Statusleiste sagt „Git: nicht lesbar“ (Grund im Tooltip), die Fläche zeigt den Grund als Fehlermeldung;
  - der gescheiterte erste Commit (Warnung mit dem Grund von Git, nur „Schließen“, ohne Prüfliste);
  - „Git ist nicht installiert: nur die Vorlagen werden geschrieben“ (Warnung in der Prüfliste, primär dann „Vorlagen schreiben“);
  - „Erst speichern“ (Fehlerzeile, „Einrichten“ gesperrt).

  Sie nutzen die vorhandenen Bausteine Meldung und Prüfliste.
- **Vervollständigen:** Der Dialog „Textprojekt vervollständigen?“ mit dem Satz des Kerns, „Abbrechen“ und „Vervollständigen“ (primär), ohne gefundene Basis-ROM mit der Warnung und „Basis-ROM wählen…“, im Lauf mit „Basis-ROM auspacken …“ bis „Abgleich vorbereiten …“. Es ist derselbe Dialog-Aufbau mit Beschreibung, Meldung und Fortschrittszeile.
- **Häkchen:** „Git einrichten (nur lokal)“ im Schritt „Vorlage“ des Assistenten mit Hilfetext, die Phase „Git einrichten“ im Ablauf und „, mit Git“ in der Zusammenfassung. Das sind der vorhandene Baustein Kästchen mit Hilfetext und die vorhandene Phasenzeile, wie „Als Textprojekt einrichten“ aus 4.3b.
- **Weitere Formen des Details:**
  - Karte: Abschnitt „Teile“ mit den Plaketten „geändert“ und „gleich“, „Zellen der Bewegungsrechte“ als Änderungstabelle „Zelle“ (`5, 7`), „Gebäude“ als Felder, darunter „und {n} weitere Einträge“.
  - Zeilen: je Hunk „Zeile 12 → 12“ und die Zeilen in `code/s` mit „−“, „+“ oder Leerraum vorn, auf den Flächen der Spannen.
  - Binär: „Eigenschaft“ mit „Größe“ und „SHA-1“, „wie in der Basis-ROM“.

  Es sind dieselbe Tabelle und dieselben Spannen mit anderem Inhalt.
- **Die übrigen Vergleiche:**
  - „Gegen Basis“ mit dem Satz von 4.1b, den Plaketten „Geändert“, „Neu“, „Fehlt“ und dem Hinweis „Noch nicht gebaut …“;
  - „Gegen gespeichert“ mit „Was ungespeichert vom gespeicherten Stand abweicht.“, der Plakette „Ungespeichert“ und den Seiten „Gespeichert → Ungespeichert“;
  - „Noch kein Commit: es gibt nichts zu vergleichen.“;
  - die leeren Zustände („Seit dem letzten Commit nichts geändert.“, „Nichts ungespeichert.“) und das Detail ohne Auswahl („Wähle einen Eintrag in der Liste, dann steht hier, was sich geändert hat.“);
  - die gesperrte Taste „Öffnen“ (Tooltip „Für diese Art gibt es keinen Ort in der App.“).
- **Schmal:** Unter 760 px Breite der Aufteilung steht das Detail unter der Liste.
- **Agentenfrage:** „„agent-gate“ möchte: Git einrichten (lässt sich nicht rückgängig machen).“ ist der vorhandene Dialog der Agentenfrage (3h).
- **Dunkel und Kompakt:** Die Bausteine haben ihre Zustände schon in der Datei. Frame 2 lässt sich mit `--dunkel` bauen, damit die Flächen der Spannen in Dunkel zu sehen sind.
