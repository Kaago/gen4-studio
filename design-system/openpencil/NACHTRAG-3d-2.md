# Nachtrag 3d.2 für die OpenPencil-Datei

**Die Datei wurde nicht angefasst.** Phase 3d.2 „Erzählen-Rahmen“ (31.10.2026) hat Code, README und Audit nachgezogen; `design-system/Editor_DesignSystem.fig` kennt die neuen Zustände noch nicht, die Frames folgen. OpenPencil war beim Schreiben nicht erreichbar (die MCP-Verbindung scheiterte mit einem 502). Dieser Nachtrag ist die Anleitung für die Sitzung, die die Datei danach anfasst; die Vorlage `beispiele/erzaehlen-rahmen.body.js` liegt bereit und lässt sich ohne OpenPencil bauen.

Was die App zeigt, beschreibt `design-system/README.md`, Abschnitt „Nachtrag 3d.2“. Texte und Zustände stammen von dort und aus dem Code: `apps/studio/src/ui/frame/{shellState,Hinge,RecipeBar}.tsx`, `ui/primitives/RecipeButton.tsx`, `workbench/recipes.ts`, `workbench/inspector/{EventEditor,FigureLookField,WarpField,LookField,eventFields,fields,headerSections}`, `world/scene/draft.ts`, `workbench/wizards/{NewRoomWizard,WizardResult,DraftNotes,wizardSpecs}`. Projekt „Sinnoh-Remix“, Platin DE, wie die übrigen App-Screens. Beispielwerte, keine Spieldaten, sind:
- der Raum „Haus 5“ (Kopie der Karte 0411),
- das Aussehen 148 der Figur,
- die Lage der geplanten Zelle und Tür in der Schau.

Vergleichsquelle sind die Screenshots der Test-Umgebung unter `target/agent/gate-2026-10-31/3d-2/` (1440 × 900, nicht im Repo). Sie entstehen im Lauf des Gates dieser Phase; die Liste mit Namen steht im Gate-Bericht (`docs/phase-3d-2-gate.md`). Der Plan sah `target/agent/3d-2/` vor; maßgeblich ist der Ordner des Gate-Laufs.

Neue Tokens gibt es nicht. Der Entwurf nutzt `szene/auswahl/halo` gestrichelt bei 50 %. Zwei Namen des Entwurfs gibt es nicht und die Frames brauchen sie auch nicht: für den Hover der Taste gilt `flaeche/deck` (nicht `flaeche/hover`), für den Text der gesperrten Taste `text/deaktiviert` (nicht `text/gedaempft`). Neu ist ein Symbol: `narrate` (Phosphor `BookOpenText`, in der Vorlage `ph:book-open-text`) für das Segment „Erzählen“; `build` (`ph:hammer`), `tinker` (`ph:wrench`) und `lock` (`ph:lock`) gibt es schon.

## Was neu zu zeichnen ist

Zwei Frames je 1440 × 900 und ein Brett, alle aus `beispiele/erzaehlen-rahmen.body.js` (`MODE` wählt). Diese Teile gibt es schon:
- Rahmen, Atlas, Liste „Welt“ und Statusleiste,
- das Dock der Werkbank mit Schau, Script und Probleme und Inspector,
- die Schrittleiste des Assistenten (`WizardBar`) und der Fuß mit „Nicht rückgängig“ (`WizardHand`),
- die Bausteine Segmente, Meldung, Referenz, Plakette und Schau-Platzhalter (`Welt3D`).

Neu sind:
- das Scharnier mit drei Segmenten (die Vorlage zeichnet es selbst; `ScharnierOben` aus `jsx-lib-app.js` kennt nur zwei),
- die Rezept-Taste und die Rezept-Leiste,
- der Entwurf in der Schau,
- der Inspector einer Figur in Erzählen.

1. **Frame „App: Welt, Erzählen, Inspector Figur“** (`MODE = 'inspector'`).
   - **Scharnier:** Brotkrume „Zweiblattdorf › Außen 0411 › Objekt 2 · ID 2 Eventdatei 0390“, dann die Segmente „Tiefe“ mit „Erzählen“ (Symbol `ph:book-open-text-fill`, gewählt), „Bauen“ und „Tüfteln“, Prüflicht, Verlauf, „Hier spielen“, „Schau maximieren“.
   - **Schau:** 3D mit der gewählten Figur, den Ebenen oben links, der Legende unten links und der Rezept-Leiste senkrecht links darunter: „Gebiet anlegen“ und „Raum anlegen“ (laufen), „Neue Figur“, „Tür verbinden“ und „Neuer Trainer hier“ (gesperrt, mit Schloss). Die Werkzeugleiste fehlt.
   - **Inspector** (340 breit): Kopf „Figur“ / „Objekt 2 · ID 2“, darunter vier Felder: „Aussehen“ (Sprite, „Aussehen 148“, „Ändern…“), „Bewegung“ („Blickt nach unten“, Auswahlfeld), „Blickrichtung“ („Unten“), „Trainer“ („kein Trainer“, „Ändern…“).
   - **Statusleiste:** ohne Werkzeugkürzel, „Zweiblattdorf, Raster 0“.
2. **Frame „App: Welt, Erzählen, Entwurf Raum anlegen“** (`MODE = 'entwurf'`).
   - **Scharnier:** die Schrittleiste „Raum anlegen“ (Chip Welt, Kante `bereich/welt-punkt`): „Vorlage, Ort, Eingang, Zusammenfassung“, Schritt 4, „Zurück ⌘[“, „Anlegen ↵“.
   - **Schau:** 3D mit dem Entwurf: ein gestrichelter Zellrahmen (Strich 1, Lücke 0,5 Kachel) in `auswahl/halo` bei 50 %, ein 1 × 1 Kachelrahmen und der halbtransparente Platzhalter der Tür. Der Inspector zeigt das Gebiet mit dem Abschnitt „Ort“.
   - **Hand:** die Zusammenfassung mit den Sätzen des Assistenten („Legt den Raum „Haus 5“ mit einer Kopie der Karte 0411 an, mit Gebäuden.“, „Legt eine Tür zwischen „Haus 5“ und Zweiblattdorf an.“), im Fuß der erste Satz und die Plakette „Nicht rückgängig“. Keine Sätze des Kerns, die stehen nur in Tüfteln.
3. **Brett „Komponenten (App): Rezept-Taste und Segmente“** (`MODE = 'brett'`), für die Seite „Komponenten“. Es zeigt vier Zustände:
   - die Rezept-Taste in Ruhe, Hover, Aktiv und Gesperrt;
   - die Taste in vier Bereichsfarben (Welt, Pokémon, Geschichten, Gegenstände);
   - die Segmente „Tiefe“ im Scharnier, je eine Zeile mit „Erzählen“, „Bauen“ und „Tüfteln“ gewählt;
   - das Ergebnis „Als Nächstes“ mit „Neue Figur hier“ (gesperrt) nach „Raum anlegen“ und „Gebiet anlegen“ (läuft) nach „Welt leeren“.

## Lage und Einspielen

- **Lage:**
  - Die Frames 1 und 2 kommen auf die Seite „App: Werkbank“ in die nächsten freien Spalten, Frame 2 rechts neben Frame 1 (Reihe der Welt-Screens). Es gilt x = Spalte × 1520, y = Reihe × 1000.
  - Das Brett kommt auf die Seite „Komponenten“ unter das letzte Brett „Komponenten (App)“ (4.4: „Änderungstabelle und Git“ bei x 3440, y 15380).
  - Vorher mit `get_page_tree` (depth 1) die belegten Positionen ablesen und nur eigene Frames anfassen. Die Lage steht danach hier und im README, Abschnitt „Nachtrag 3d.2“.
- **Bauen:**
  1. Aus `openpencil/beispiele/` `bash ../build-screen.sh erzaehlen-rahmen` aufrufen (für Dunkel mit `--dunkel`). Das Skript schreibt nur die `.jsx` neben die Vorlage und braucht OpenPencil nicht.
  2. In der erzeugten `erzaehlen-rahmen.jsx` `MODE` je Frame umstellen (`'inspector'`, `'entwurf'`, `'brett'`).
  3. `node ../../scripts/op.mjs render '{…}' --jsx erzaehlen-rahmen.jsx` aufrufen (`op.mjs render` mit `x`, `y` und der Seite, siehe die früheren Nachträge).
  4. Per `export_image` vergleichen (`node ../../scripts/op.mjs export_image '{…}' --png bild.png`), mit den Screenshots aus `target/agent/gate-2026-10-31/3d-2/`: Frame 1 mit den Bildern der Welt in Erzählen (Figur gewählt), Frame 2 mit den Bildern des Assistenten „Raum anlegen“ in Erzählen vor dem Anlegen, das Brett mit den Bildern der Rezept-Leiste und des Ergebnisses. Dunkel und Kompakt mit den Bildern `-dunkel` und `-kompakt` derselben Namen.

     Die Screenshots zeigen echte Werte des Testprojekts, die Frames Beispielwerte. Verglichen werden Aufbau, Maße und Texte. Atkinson Hyperlegible Mono ist in OpenPencil ersetzt (Inter), darum sind Spannen in den Bildern breiter als ihr Text.
  5. Korrigieren: den Body ändern, neu bauen, `render` mit `"replace_id":"<ID>"` statt `x`/`y` (höchstens drei Läufe je Frame). Die erzeugten `.jsx` und `.png` nicht einchecken, sondern löschen.
- **Danach:**
  1. Speichern (der Hauptagent, `save_file`) und das Dokument schließen.
  2. Aus der Repo-Wurzel `node design-system/scripts/fix_fig_text.mjs design-system/Editor_DesignSystem.fig --rgb555` aufrufen.
  3. Im README den Abschnitt „Nachtrag 3d.2“ um einen Satz ergänzen, dass die Frames seit … in der Datei stehen (mit Seite und Lage). Den Satz „die `.fig` kennt das noch nicht, die Frames folgen“ dort (Abschnitt und Abweichungen), „Die Datei wurde nicht angefasst“ hier und in den Zeilen „Segmente“, „Rezept-Taste“ und „Entwurf“ in `Audit-Design-System.md` ersetzen.
  4. Im Brett „Stand der App“ (Seite „Titel“, x 1720, y 0, Vorlage `beispiele/stand-der-app.body.js`, per `render` mit `replace_id`) eine Zeile „Erzählen-Rahmen (3d.2)“ unter „Teilweise umgesetzt“ anlegen, mit dem Verweis auf die beiden Frames.
  5. Im Brett „Komponenten (App)“ das Symbol `narrate` in die Symbolreihe nehmen.
- **Aus dem Gedächtnis (OpenPencil-Grenzen):**
  - Die Datei hat die Grenze von 10 000 Knoten; sie blockiert einzelne Knotenänderungen. Keine Einzeländerungen am Knoten versuchen, sondern den Frame neu rendern (`render` mit `replace_id`).
  - Ist der Bildschirm gesperrt, laufen die Aufrufe in Zeitüberschreitungen. Den Bildschirm entsperrt halten und den Lauf wiederholen.
  - OpenPencil lehnt Pfade unter `.claude/` ab: auf einer Kopie unter `gen4-studio/target/agent/` der Hauptkopie rendern, speichern, in den Worktree zurückkopieren.

  `check-tokens.sh` ist nicht betroffen.

## Was nicht gezeichnet werden muss

- **Weitere Zustände der Taste:** Größe S im Ergebnis, die Taste im Tooltip (zwei Zeilen: Name, darunter was das Rezept macht oder der Grund) und im Fokus. Es sind dieselbe Kachel und dieselben Flächen.
- **Weitere Leistenlagen:** oben oder unten waagerecht (dort in der Mitte der Schau); dieselbe Leiste, die Tasten nebeneinander.
- **Die übrigen Felder in Erzählen:**
  - Tür: „Ziel“ (Referenz) und „Tür“ (`Tür 2 · x 107, z 873`, „Ändern…“, Picker „Tür wählen“);
  - Schild: „Der Text des Schilds kommt mit 3d.5.“;
  - Auslöser: „Geschichte und Schritt kommen mit 3d.7.“;
  - Gebiet und Raum: Abschnitt „Ort“ mit „Ort“, „Aussehen“ (Kartenbild des Aussehens) und „Musik (Tag)“, „Musik (Nacht)“ als Zahlfelder ohne Dez/Hex.

  Es sind dieselben Reihen mit anderem Inhalt.
- **Welt leeren und Gebiet erweitern:** derselbe Entwurf in der Schau (Zellrahmen) und dieselbe Zusammenfassung; „Vorschau nicht möglich: …“ als Warnung in jeder Tiefe.
- **Entwurf auf Weltkarte und Raumplan:** geplante Zellen mit gestricheltem Rand in `auswahl/kante` (`data-draft-cell`).
- **Andere Bereiche in Erzählen:** Sie zeigen Bauen; das Segment bleibt auf Erzählen. Kein eigener Frame.
- **Dunkel und Kompakt:** Die Bausteine haben ihre Zustände schon in der Datei. Frame 2 lässt sich mit `--dunkel` bauen.
