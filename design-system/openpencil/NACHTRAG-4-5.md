# Nachtrag 4.5 für die OpenPencil-Datei

Stand: 31.10.2026. Diese Anleitung sagt, was zu Phase 4.5 „Veröffentlichen“ neu in `Editor_DesignSystem.fig` gehört: der Assistent „Veröffentlichen“ in vier Schritten, Technik › Veröffentlichen mit der Liste „Bisherige Veröffentlichungen“, die Taste „Veröffentlichen“ in der Titelleiste und der Dialog „Stand sichern?“. **Die Datei wurde nicht angefasst:** OpenPencil war beim Schreiben nicht erreichbar (der Server antwortete mit 502). Darum liegen nur die Vorlage `beispiele/veroeffentlichen-assistent.body.js` und diese Anleitung bereit; die Frames folgen in einer eigenen Sitzung nach dem Merge. Wer sie einspielt, trägt hier „Eingespielt am …“ mit Seite und Lage nach, so wie `NACHTRAG-4-4.md`, und streicht im README den Satz „die Frames folgen“.

Was die App zeigt, beschreibt `design-system/README.md`, Abschnitt „Nachtrag 4.5“. Texte und Zustände stammen von dort und aus dem Code: `apps/studio/src/publish/{PublishWizard.tsx,publishWords.ts}`, `git/{CommitDialog.tsx,gitWords.ts}`, `technik/PublishSurface.tsx`, `ui/frame/TitleBar.tsx`, `workbench/wizards/{WizardFrame,WizardBar}.tsx`. Projekt „Sinnoh-Remix“, Platin DE, Basis-ROM „Platin DE (Testprojekt)“, wie die übrigen App-Screens. Beispielwerte, keine Spieldaten, sind:
- die Version `0.1.1` und die Fassung `0.1.0` davor,
- die zwei Zeilen der Änderungsnotizen („Der Rivale kämpft ab Level 12.“, „Neue Brücke auf Route 216.“),
- die Größen („5,1 KiB“, „612 Byte“, „248 Byte“, „4,7 KiB“), die Daten und die kurzen SHA-1 (`d40f19ac` → `7be2c014`),
- der Pfad `/Users/du/Spiele/Sinnoh-Remix_DSPRE_contents/build`,
- die Git-Version (2.54.0) und die Zahl der Änderungen.

Vergleichsquelle sind die Screenshots der Test-Umgebung unter `target/agent/gate-<datum>/4-5/` (`blick-<fläche>-<zustand>-<thema>.png`, 1440 × 900, nicht im Repo; das Datum und die Namen stehen im Gate, `docs/phase-4-5-gate.md`, Abschnitt „Blick“). Die Bilder entstehen mit dem UI-Test des Gates, sie gab es beim Schreiben dieser Anleitung noch nicht. Die Screenshots zeigen echte Werte des Testprojekts, die Frames Beispielwerte. Verglichen werden Aufbau, Maße und Texte.

Neue Tokens gibt es nicht. Neue Symbole gibt es auch nicht: `publish` (Phosphor `UploadSimple`, in der Vorlage `ph:upload-simple`) und `git` (`ph:git-branch`) sind da, `ph:folder-open` ist das Symbol `open` für „Ordner wählen…“ und „Im Finder zeigen“.

## Was neu zu zeichnen ist

Vier Frames, je 1440 × 900, alle aus `beispiele/veroeffentlichen-assistent.body.js` (`MODE` wählt). Rahmen, Atlas, Statusleiste, die Liste „Technik“ mit acht Einträgen, die Schrittleiste des Assistenten, die Hand mit Spalten, Dialog, Plakette, Meldung und Karte gibt es schon. Neu sind:
- die Taste „Veröffentlichen“ in der Titelleiste (die Bibliothek `jsx-lib-app.js` kennt nur vier Tasten; die Vorlage zeichnet die Titelleiste mit fünf selbst, `Titelleiste` und `Fenster`),
- die vier Pillen der Schrittleiste mit der Taste „Fertig“ auf „Prüfen“ und dem geschlossenen Zustand (`Schrittleiste`),
- die Felder der Hand, die Tabelle „Was du weitergibst“ und die Tabelle „Bisherige Veröffentlichungen“,
- der Inhalt des Dialogs „Stand sichern?“.

1. **Frame „App: Assistent, Veröffentlichen, Version“** (`MODE = 'version'`).
   - **Welt und Scharnier:** Die Welt steht in der Schau (Orte-Liste mit Zweiblattdorf geöffnet, 3D-Szene, wie in `welt-leeren.body.js`), der Atlas auf Welt. Das Scharnier ist die Schrittleiste „Veröffentlichen“ (Chip mit `ph:upload-simple-fill`, Linie des Bereichs Technik) mit den Pillen „Version“ (aktiv), „Patch“, „Prüfen“, „Fertig“; rechts „Abbrechen“ und „Weiter“ (primär) mit ↵, ohne „Zurück“.
   - **Hand, zwei Spalten:** Links das Feld „Version“ mit `0.1.1` (320 breit) und das mehrzeilige Feld „Änderungsnotizen“ (480 breit, 72 hoch) mit den zwei Zeilen und dem Hinweis „Was sich seit der letzten Fassung geändert hat. Steht in der Begleitdatei.“ (`text/xs` sekundär). Rechts „Speicherort“ (Beschriftung `text/s` 600, darunter der Pfad in Mono 13/20), die sekundäre Taste „Ordner wählen…“ mit dem Symbol `ph:folder-open` und das Feld „Dateiname“ mit `Sinnoh-Remix-v0.1.1.xdelta`.
   - **Fuß:** Listensymbol und der Satz „Schreibt Sinnoh-Remix-v0.1.1.xdelta und die Begleitdatei nach /Users/du/Spiele/Sinnoh-Remix_DSPRE_contents/build.“ Ohne Plakette (es ist keine Handlung, die sich nicht zurücknehmen ließe: jede Fassung ist eine neue). Statusleiste „Schritt 1 von 4: Version“ mit den Tasten ↵, ⌘[, Esc.
2. **Frame „App: Assistent, Veröffentlichen, Prüfen“** (`MODE = 'pruefen'`).
   - **Scharnier:** Die Pillen „Version“ und „Patch“ abgehakt, „Prüfen“ aktiv, „Fertig“ offen. Weil die Veröffentlichung geschrieben ist, steht links „Schließen“ statt „Abbrechen“, es gibt kein „Zurück“, die primäre Taste heißt „Fertig“.
   - **Hand, zwei Spalten (rechts doppelt so breit):** Links die Erfolgsmeldung „Selbstprüfung bestanden: auf die Basis-ROM angewendet ergibt der Patch genau die gebaute ROM.“ mit der Zeile „Basis d40f19ac → Ziel 7be2c014, geprüft in 148 ms“. Rechts „Was du weitergibst“ (`text/s` 600 sekundär) und die Tabelle „Datei | Größe“ mit drei Zeilen (Patch, Begleitdatei `….xdelta.txt`, `README.txt`), darunter der Satz „Kein ROM-Inhalt, nur der Patch: wer ihn anwendet, braucht dieselbe Basis-ROM.“ (sekundär). Kein Fuß: Nach dem Lauf steht dort keine Zusammenfassung mehr. Statusleiste „Schritt 3 von 4: Prüfen“, „Gebaut gerade eben“.
3. **Frame „App: Technik, Veröffentlichen mit Liste“** (`MODE = 'liste'`).
   - **Liste „Technik“:** acht Einträge, „Veröffentlichen“ gewählt (Symbol `ph:upload-simple`, Untertitel „Veröffentlicht v0.1.1“), „Änderungen“ mit „Keine Änderung seit dem Commit“.
   - **Fläche:** Titel `font/titel-s`, darunter der Satz „Ein Patch gibt deine Änderungen weiter, ohne die ROM: wer dieselbe Basis-ROM hat, wendet ihn mit Rom Patcher JS oder xdelta3 an.“ (sekundär, höchstens 64ch, rund 520). Die Karte „Basis-ROM“ wie in `veroeffentlichen.body.js` (4.1a, Zustand gefunden): Plakette „Gefunden“, Angaben ROM, SHA-1, Datei. Die Karte „Veröffentlichen“: der Satz „Der Assistent fragt Version und Änderungsnotizen, baut die ROM und schreibt den Patch mit Begleitdatei an den Ort, den du wählst.“, die primäre Taste „Veröffentlichen…“, die Überschrift „Bisherige Veröffentlichungen“ und die Tabelle „Version | Datum | Datei | Größe | Notizen | Taste“ mit zwei Zeilen, die neueste oben: `0.1.1`, 31.10.2026, Dateiname in Mono, 5,1 KiB, „Der Rivale kämpft ab Level 12.“; `0.1.0`, 30.10.2026, 4,7 KiB, „Erste Fassung“. Letzte Spalte das Symbol `ph:folder-open` als Symboltaste „Im Finder zeigen“.
   - **Statusleiste:** „101 Warnungen, 10 Hinweise“, „main · keine Änderung“ (Symbol `ph:git-branch`), „Gebaut gerade eben“, „Komfort“.
4. **Frame „App: Dialog, Stand sichern“** (`MODE = 'sichern'`).
   - **Hintergrund:** dieselbe Fläche wie Frame 3; die Statusleiste zeigt noch „main · 3 Änderungen“, „Änderungen“ in der Liste „3 Änderungen seit dem Commit“.
   - **Dialog:** darüber die Abdunklung (`flaeche/ueberlagerung`) und der Dialog, 480 breit:
     - Titel „Stand sichern?“, Beschreibung „Legt einen Commit mit allen Änderungen im lokalen Repository an (git add -A, git commit). Nichts wird hochgeladen.“
     - Prüfliste „Vor dem Sichern“ mit drei Zeilen und Haken: „Alles gespeichert“, „Git gefunden: 2.54.0 (Apple Git-157)“, „3 geänderte Dateien seit dem letzten Commit“.
     - Das Feld „Betreff“ mit „Veröffentlicht v0.1.1“ und das mehrzeilige Feld „Notiz“ mit den zwei Zeilen der Notizen und dem Hinweis „Optional; steht im Commit unter dem Betreff.“
     - Tasten „Abbrechen“ und „Sichern“ (primär), dazu das Kreuz.

## Lage und Einspielen

- **Lage:**
  - Frame 3 kommt auf die Seite „App: Texte, Technik, Dialoge“ in die nächste freie Spalte der Reihe der Technik-Screens (Reihe 0), Frame 4 in die nächste freie Spalte der Reihe der Dialoge (Reihe 1). Es gilt x = Spalte × 1520, y = Reihe × 1000.
  - Die Frames 1 und 2 kommen auf die Seite „App: Start und Welt“ (der Assistent läuft im Deck der Welt, mit der Welt in der Schau, der Atlas steht auf Welt), neben „App: Technik, Welt leeren“ in die nächsten freien Spalten derselben Reihe.
  - Vorher mit `get_page_tree` (depth 1) die belegten Positionen ablesen und nur eigene Frames anfassen. Die Frames von 4.4 stehen seit dem 29.10.2026 bei x 16720 und 18240 (Reihe 0) und x 15200 (Reihe 1); ob später etwas dazukam (3d), zeigt der Baum.
- **Bauen:**
  1. Aus `openpencil/beispiele/` `bash ../build-screen.sh veroeffentlichen-assistent` aufrufen (für Dunkel mit `--dunkel`). Das Skript schreibt nur die `.jsx` neben die Vorlage und braucht OpenPencil nicht.
  2. In der erzeugten `veroeffentlichen-assistent.jsx` `MODE` je Frame umstellen (`'version'`, `'pruefen'`, `'liste'`, `'sichern'`).
  3. `node ../../scripts/op.mjs render '{…}' --jsx veroeffentlichen-assistent.jsx` aufrufen.
  4. Per `export_image` vergleichen (`node ../../scripts/op.mjs export_image '{…}' --png bild.png`), mit den Screenshots aus `target/agent/gate-<datum>/4-5/`: je Frame das Bild der Fläche oder des Schritts in Hell (die Namen stehen im Gate); für Dunkel und Kompakt die Bilder `-dunkel` und `-kompakt` derselben Namen (dunkel gebaut vergleichen).
  5. Korrigieren: den Body ändern, neu bauen, `render` mit `"replace_id":"<ID>"` statt `x`/`y` (höchstens drei Läufe je Frame). Die erzeugten `.jsx` und `.png` nicht einchecken, sondern löschen.
- **Danach:**
  1. Speichern (der Hauptagent, `save_file`) und das Dokument schließen. Bei gesperrtem Bildschirm laufen die RPCs von OpenPencil in Zeitüberschreitungen; bei der Datei mit rund 10 000 Knoten Einzeländerungen vermeiden und die Frames neu rendern (`replace_id`). OpenPencil lehnt Pfade unter `.claude/` ab: auf einer Kopie unter `target/agent/` der Hauptkopie arbeiten, speichern und in den Worktree zurückkopieren.
  2. Aus der Repo-Wurzel `node design-system/scripts/fix_fig_text.mjs design-system/Editor_DesignSystem.fig --rgb555` aufrufen.
  3. Im README den Abschnitt „Nachtrag 4.5“ um einen Satz ergänzen, dass die Frames seit … in der Datei stehen (mit Seite und Lage), und den Satz „Die `.fig` kennt die Frames noch nicht …: die Frames folgen“ dort, „Die Datei wurde nicht angefasst“ hier und die Zeilen in `Audit-Design-System.md` („`.fig` folgt“) ersetzen.
  4. Im Brett „Stand der App“ (Seite „Titel“, x 1720, y 0, Vorlage `beispiele/stand-der-app.body.js`, per `render` mit `replace_id`) die Zeile „Veröffentlichen: Patch erstellen“ von „Teilweise umgesetzt“ nach „Umgesetzt“ nehmen, mit der Notiz „Veröffentlichen (4.5): Assistent in vier Schritten, Liste, Titelleiste, „Stand sichern““ und dem Verweis auf die Frames (statt „Veröffentlichen (4.1a)“). Das Zielbild „Veröffentlichen: Patch erstellen“ trägt dann die Marke „Umgesetzt“ (`Stand()`), mit dem Zusatz „Ohne Textexport“.

  `check-tokens.sh` ist nicht betroffen.

## Was nicht gezeichnet werden muss

- **Lauf:** Der Schritt „Patch“ im Lauf: der Satz und das Kästchen „Änderungsnotizen auch als README.txt daneben“ mit der Zeile der Phase („Speichern und bauen …“, „Patch kodieren …“, „Selbstprüfung …“, „Dateien schreiben …“, „Veröffentlichung eintragen …“) in `text/s` sekundär; der Assistent ist dabei gesperrt. Dasselbe im Dialog „Stand sichern?“ (keine Tasten und kein Kreuz, darunter „Vorlagen prüfen …“, „Dateien aufnehmen …“, „Commit …“).
- **Fehler:**
  - Die Fehlermeldung im Schritt „Patch“ (Grund des Kerns, mit Pfad) und die Zeile „Lese die Veröffentlichungen …“ im Schritt „Version“.
  - Die Sperre „Weiter“ mit dem Satz unter dem Feld „Version“ („Diese Version gibt es schon.“, der Satz zur Form der Version) oder „Dateiname“.
  - Die Meldung der gebauten ROM, die der Basis-ROM gleicht.
  - Im Dialog „Stand sichern?“ „Erst speichern“, „Git ist nicht installiert“, „Keine Änderung seit dem letzten Commit“ (Warnung), eine Ablehnung als Fehlermeldung und der gescheiterte Commit (Warnung mit dem Text von Git, nur „Schließen“, ohne Prüfliste).

  Sie nutzen die vorhandenen Bausteine Meldung, Prüfliste und Feld.
- **„Ersetzen“:** Die sekundäre Taste unter der Fehlermeldung, wenn die Datei schon da ist.
- **Ergebnis „Fertig“:** Erfolgsmeldung „Veröffentlicht: v0.1.1“ mit dem Pfad und „3 Dateien, 5,1 KiB“, „Als Nächstes“ mit den zwei Sätzen, darunter die sekundäre Taste „Stand sichern…“, unten „Im Finder zeigen“ (primär) und „Schließen“. Es ist der Baustein `DoneResult` aus den anderen Assistenten.
- **Liste:** Leer („Noch keine Veröffentlichung.“), Laden („Wird gelesen …“), kaputte Liste (Fehlermeldung statt der Tabelle, „Veröffentlichen…“ gesperrt mit dem Grund daneben), gesperrt ohne Basis-ROM (wie 4.1a mit dem Grund neben der Taste).
- **Titelleiste:** Die Taste gesperrt mit dem Tooltip in zwei Zeilen („Veröffentlichen“ und der Grund, etwa „Erst eine Basis-ROM zuordnen.“) und ohne Projekt; sie nutzt den Baustein Symboltaste mit Tooltip.
- **Palette:** „Veröffentlichen…“ und „Stand sichern…“ mit ihren Sperrgründen (Baustein Palette).
- **Technik › Änderungen:** Die Taste „Stand sichern…“ neben „Neu vergleichen“ (sekundär, `data-commit-all`; ohne Git `aria-disabled` mit dem Tooltip „Git ist nicht installiert.“) und die Taste ohne Commit neben der Info „Noch kein Commit: es gibt nichts zu vergleichen.“
- **Toast:** „Stand gesichert: 4c1d2e3 „Veröffentlicht v0.1.1“ (3 Dateien).“ (Baustein `Toast`).
- **Agentenfrage:** „„agent-gate“ möchte: Patch ersetzen: …“ und „„agent-gate“ möchte: Stand sichern“ sind der vorhandene Dialog der Agentenfrage (3h).
- **Dunkel und Kompakt:** Die Bausteine haben ihre Zustände schon in der Datei. Die Vorlage lässt sich mit `--dunkel` bauen.
