# Nachtrag 4.2 für die OpenPencil-Datei

Stand: 27.10.2026. Diese Anleitung sagt, was zu Phase 4.2 „Build-Graph“ neu in `Editor_DesignSystem.fig` gehört. **Eingespielt auf main am 27.10.2026 (Nachmittag)**, nach 4.0 Teil B und 4.1a: „App: Technik, Prüfen und Bauen mit Schritten“ bei x 10640, y 0 und „App: Technik, Texte auf zwei Seiten geändert“ bei x 10640, y 1000 auf „App: Texte, Technik, Dialoge“, das Brett „Komponenten (App): Build-Schritte“ bei x 3440, y 14440 auf „Komponenten“; die Marke aus `beispiele/stand-marken.body.js` (`MODE = 'bauen'`), die Zeile aus `beispiele/stand-der-app.body.js`. Die Liste „Technik“ hat sechs Einträge (mit „Projekt“ und „Veröffentlichen“). Gerendert auf einer Kopie unter `gen4-studio/target/agent/fig-4/` der Hauptkopie (OpenPencil lehnt Pfade unter `.claude/` ab), gespeichert, in den Worktree kopiert, `fix_fig_text.mjs --rgb555`, wieder geöffnet und als Bild geprüft. Der erste Lauf in der Sitzung von 4.2 (sein Stand ging beim Merge verloren, weil die `.fig` von main behalten wurde) war so beschrieben: eingespielt (auf dem Stand der `.fig` aus `main` nach 4.0): beide Frames, das Brett der Komponente (`MODE = 'brett'`), die Marke über dem Zielbild „Prüfen und Bauen“ und die Zeile im Brett „Stand der App“ (beide über `render` mit `replace_id`, weil OpenPencil Einzeländerungen bei dieser Datei ablehnt). Aus einem Worktree: Das MCP-Root ist `lab/editor`, Dateien im Worktree lehnt `open_file` ab; die Datei wurde darum als Kopie unter `gen4-studio/target/agent/` (ignoriert) geöffnet, gespeichert und zurückkopiert. Was die App zeigt, beschreibt `design-system/README.md`, Komponente „Build-Schritte“ und Abschnitt „Nachtrag 4.2“; Texte und Zustände stammen von dort. Die Vorlage für beide Frames ist `beispiele/technik-bauen-schritte.body.js` (`MODE` wählt den Frame). Die Beispielwerte stammen aus HeartGold DE nach einer Kartenänderung (Projekt „Johto-Remix“).

## Was neu zu zeichnen ist

Nur diese Teile sind neu; alles andere (Rahmen, Atlas, Liste „Technik“, Statusleiste, Baustein Dialog) gibt es schon.

1. **Prüfen und Bauen mit Schritten.** Frame „App: Technik, Prüfen und Bauen mit Schritten“, 1440 × 900, Seite „App: Texte, Technik, Dialoge“, neben dem vorhandenen Technik-Screen (`MODE = 'schritte'`). Die Fläche zeigt:
   - Die Tasten „Bauen“ (primär, Symbol Hammer), „Neu bauen (ohne Cache)“ (sekundär, Symbol Hammer) und „Starten“.
   - „Letzter Build“ mit „gerade eben“ und „Pfad“ in Code-Schrift.
   - Die Überschrift „Schritte“ (`text/m` betont). Darunter im Rahmen der Tabelle der Probleme (`flaeche/blatt`, Kante `linie/leise`, `radius/fenster`) die Tabelle „Schritt | Ergebnis | Dauer“ (Dauer 96 breit, rechtsbündig) mit fünf Zeilen:
     - „Karten packen“, „1 Member eingesetzt“, „0,02 s“;
     - „Spielcode (ARM9)“, „1 aus dem Cache“, „< 0,01 s“;
     - „Overlays“, „127 aus dem Cache“, „0,01 s“;
     - „ROM schreiben“, „geschrieben“, „0,05 s“;
     - „Prüfsummen“, „keine Basis-Marke“, „< 0,01 s“.
   - Darunter im eigenen Rahmen der eingeklappte Abschnitt „Übersprungen (38)“ (Pfeil nach rechts).
   - Danach die Überschrift „Probleme“ mit dem Satz des leeren Zustands.
   - Die Liste links markiert „Prüfen und Bauen“ mit „gebaut gerade eben“, die Statusleiste zeigt „Gebaut gerade eben“.
2. **Texte auf zwei Seiten geändert.** Frame „App: Technik, Texte auf zwei Seiten geändert“ (`MODE = 'konflikt'`), Seite „App: Texte, Technik, Dialoge“, bei den anderen Dialogen. Darunter liegt derselbe Screen vor dem ersten Build: „Noch kein Build in dieser Sitzung.“ unter „Schritte“, „Noch nicht gebaut“. Darüber die Abdunklung (`flaeche/ueberlagerung`) und der Dialog, 480 breit wie `DialogX`, mit diesem Inhalt:
   - Titel „Texte auf zwei Seiten geändert“.
   - Der Satz „Diese Textarchive wurden als Text (JSON) und als Spieldatei geändert, seit das Studio sie zuletzt gebaut hat, etwa in DSPRE oder durch einen Git-Wechsel. Wähle je Archiv, welche Fassung gilt; die andere wird überschrieben.“
   - Zwei Zeilen „Textarchiv 0012“ und „Textarchiv 0433“ (betont), rechts je Segmente der Größe S „Text (JSON) | Spieldatei“. Bei 0012 ist „Text (JSON)“ gedrückt (die Vorgabe), bei 0433 „Spieldatei“.
   - Die Tasten „Abbrechen“ (sekundär) und „Übernehmen und bauen“ (primär).
3. **Komponente „Build-Schritte“** auf der Seite „Komponenten“ als kleines Brett „Komponenten (App): Build-Schritte“. Es zeigt vier Zustände:
   - gelaufen: die Tabelle aus Punkt 1;
   - übersprungen eingeklappt: „Übersprungen (38)“;
   - übersprungen offen: drei Zeilen „Texte | übersprungen“, „Texte packen | übersprungen“, „Spieldaten packen | übersprungen“, daneben als einzelne Zeile „ROM schreiben | aktuell“ (so steht sie, wenn die ROM nicht neu entstand);
   - kein Build: „Noch kein Build in dieser Sitzung.“

   Dazu eine Zeile in Tüfteln, mit der Kennung grau in Mono hinter den Wörtern: „pokeHeight packen  pack:pokeHeight“. Die Zeilen lassen sich aus den Bausteinen `Zeile` und `Rahmen` der Vorlage zusammensetzen.

## Lage und Einspielen

- **Lage:** Die beiden Screens kommen auf der Seite „App: Texte, Technik, Dialoge“ in die nächste freie Spalte der Reihe der Technik-Screens bzw. der Dialoge. Es gilt x = Spalte × 1520 und y = Reihe × 1000. Vorher mit `get_page_tree` (depth 1) die belegten Positionen ablesen und nur eigene Frames anfassen. Das Brett der Komponente kommt auf die Seite „Komponenten“ unter das letzte Brett „Komponenten (App): …“.
- **Bauen:** wie in `NACHTRAG-3g.md`, Abschnitt „Einspielen“. Die Vorlage nach `<name>.body.js` kopieren und `MODE` setzen. Dann `bash ../build-screen.sh technik-bauen-schritte` (dunkel mit `--dunkel`, falls gewünscht) und `node ../../scripts/op.mjs render '{…}' --jsx technik-bauen-schritte.jsx`. Danach mit `export_image` vergleichen, und zwar mit einem Screenshot der Test-Umgebung:
  - Technik › Prüfen und Bauen nach „Hier spielen“ und einer Kartenänderung auf `4-2-browser-hgde`;
  - für den Dialog nach einem Konflikt (JSON und Member einer Kopie von außen ändern), siehe `docs/agent-gate.md`, „Phase 4.2“.

  Die erzeugten `.jsx` nicht einchecken.
- **Danach:** speichern (der Hauptagent, `save_file`) und das Dokument schließen. Dann `node design-system/scripts/fix_fig_text.mjs design-system/Editor_DesignSystem.fig --rgb555` aus der Repo-Wurzel ausführen. Im README den Abschnitt „Nachtrag 4.2“ um einen Satz ergänzen, dass die Frames in der Datei stehen. Im Brett „Stand der App“ und über dem Zielbild „Prüfen und Bauen“ die Marke ergänzen (Teilweise umgesetzt: Schritte neu, nicht im Zielbild). Neue Tokens oder Symbole gibt es nicht; `check-tokens.sh` ist nicht betroffen.

## Was nicht gezeichnet werden muss

- „Übersprungen“ offen mit allen 38 Zeilen: der Abschnitt ist im Standard zu, die drei Zeilen im Brett genügen.
- Die Meldungen als Toast: „ROM gebaut: … in 0,33 s (…)“, „Nichts gebaut. Beim nächsten „Bauen“ fragt das Studio wieder, welche Fassung gilt.“, der zweite Konflikt nach dem Auflösen, der Konflikt beim Speichern und die versteckten Dateien. Sie nutzen den vorhandenen Baustein Meldung.
- Die Zustände der Schritte nach „Neu bauen (ohne Cache)“ (alle Zeilen „(ohne Cache, voll)“), ohne Bauzustand („(erster Build, voll)“) und nach einem Werkzeugwechsel („(neues Werkzeug, voll gebaut)“): Sie unterscheiden sich nur im Text.
- Der wartende Konflikt-Dialog (Tasten und Wahl gesperrt, darunter die Info „Die gewählten Fassungen werden übernommen, danach startet der Build …“): gesperrte Tasten, gesperrtes `Segmented` und `Notice` haben ihre Zustände schon in der Datei.
- Dunkel und Kompakt: Die Bausteine `Table`, `Section`, `Segmented` und `Dialog` haben ihre Zustände schon in der Datei.
