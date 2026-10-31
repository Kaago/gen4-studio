# Nachtrag 3d.1 für die OpenPencil-Datei

Stand: 31.10.2026. Diese Anleitung sagt, was zu Phase 3d.1 „Flags und Variablen“ (Technik › Flags und Variablen, der Picker „Flag wählen“ und das Flag-Feld im Inspector) neu in `Editor_DesignSystem.fig` gehört. **Die Datei wurde nicht angefasst:** OpenPencil war beim Schreiben dieses Nachtrags nicht erreichbar (der MCP-Server antwortete mit einem Fehler 502), also ist nichts gerendert und nichts gespeichert. Die Frames von 3d.1 folgen, sobald OpenPencil antwortet.

Was die App zeigt, beschreibt `design-system/README.md`, Abschnitt „Nachtrag 3d.1“. Texte und Zustände stammen von dort und aus dem Code: `apps/studio/src/flags/{FlagsSurface,FlagList,FlagDetail,FlagPicker,FlagField,NameFlagDialog}.tsx`, `flags/flagWords.ts`, `workbench/inspector/{eventFields.ts,EventEditor.tsx,LevelScriptSection.tsx}`, `technik/TechnikArea.tsx`. Projekt „Sinnoh-Remix“, Platin DE, wie die übrigen App-Screens. Beispielwerte, keine Spieldaten, sind:
- der eigene Name `FLAG_HAT_MEDAILLE` mit seiner Beschreibung,
- die Zahlen der Verwendung („gesetzt 1 · geprüft 2“) und die Verweise des Details,
- die Zahl der Flags und der freien („4.097 Flags · 135 frei“, so zählt Platin DE),
- die Zahlen im Fuß der Liste („Script-lokale Variablen 1.284 Vorkommen, Flags aus Variablen 37“).

Die Namen der pret-Tabellen (`FLAG_UNUSED_0x0094`, `FLAG_VILLA_FURNITURE_TABLE` und ähnliche) sind Beispiele in der Form der Tabellen, nicht ausgesuchte Einträge.

Vergleichsquelle sind die Screenshots der Test-Umgebung unter `target/agent/gate-<datum>/3d-1/` (1440 × 900, nicht im Repo; Namen `blick-<fläche>-<zustand>-<thema>.png`). Sie entstehen im UI-Test von 3d.1; als diese Anleitung entstand, lagen sie noch nicht vor. Die Liste der Bilder steht im README, Abschnitt „Nachtrag 3d.1“, sobald sie da sind. Das Gate steht in `docs/phase-3d-1-gate.md` (entsteht mit dem Gate).

Neue Tokens gibt es nicht. Die Spalte „Live“ der Liste nutzt `status/warnung` (Kopf) und `text/sekundaer` (Zellen). Neu ist ein Symbol: `flag` (Phosphor `Flag`, in der Vorlage `ph:flag`) für den Eintrag „Flags und Variablen“ der Liste „Technik“ und für die Palette. Die Fläche Geschichten gibt dem Chip des Flag-Felds `ph:chats-circle` (`stories` in der App, wie jede Referenz dieser Fläche).

**Die Vorlage ist nicht gerendert.** `beispiele/flags-variablen.body.js` ist gegen `jsx-lib-app.js` geschrieben und lief mit einer Attrappe der Bausteine (`Frame`, `Text`, `Icon`, `designVar`) in allen drei Modi, hell und dunkel, ohne Fehler durch; ein Bild davon gibt es nicht. Beim ersten Rendern sind Maße und Umbrüche zu prüfen und die Vorlage nach dem Bild und den Screenshots zu korrigieren. Wahrscheinliche Stellen:
- die zweite Zeile der Listenzeilen (Mono, `one`): lange pret-Namen mit Nummer und rechts die Verwendung teilen sich 492 px;
- der Filter des Pickers: die Vorlage zeichnet die zwei Zeilen des `Segmented` mit `wrap` von Hand, die App bricht dort um, wo es nicht passt;
- die Zeilen des Flag-Felds: der Chip steht allein auf einer Zeile und „Wählen…“ darunter, wenn beide nicht nebeneinander in die Spalte (rund 200 px) passen (Schalter `wrap` der Vorlage);
- das Detail: die Mono-Zeilen und die Beschreibung haben feste Breiten.

## Was neu zu zeichnen ist

Zwei Frames je 1440 × 900 und ein Brett, alle aus `beispiele/flags-variablen.body.js` (`MODE` wählt). Diese Teile gibt es schon:
- Rahmen, Atlas und Statusleiste;
- die Liste „Technik“ (jetzt mit neun Einträgen) und `ListRow`, `Section`, `Notice`, `Ref`, `Badge`, `Btn`, `Check`, `Search`;
- die Bausteine des Werkbank-Frames „Welt bauen, Figur bearbeiten“ (Hintergrund von Frame 2), Dialog und Abdunklung.

Neu sind:
- der Eintrag „Flags und Variablen“ in der Liste „Technik“,
- die Filterzeile mit Chips, Kästchen und Suche,
- die Zeile der Liste mit zweiter Zeile in Mono, Verwendung, Plaketten und der Spalte „Live“,
- das Detail mit „Wo gesetzt“ und „Wo geprüft“,
- der Picker „Flag wählen“,
- das Flag-Feld.

1. **Frame „App: Technik, Flags und Variablen“** (`MODE = 'liste'`).
   - **Liste „Technik“:** neun Einträge. „Flags und Variablen“ ist gewählt (Symbol `ph:flag`, Untertitel „4.097 Flags · 135 frei“), „Textprojekt“ trägt „Abgeglichen“, „Änderungen“ „Keine Abweichung“.
   - **Fläche:** Titel `font/titel-s`, Fläche `flaeche/deck`, innen 32 / 24, Lesebreite 880; der Satz unter dem Titel höchstens 64ch (rund 520).
   - **Filterzeile:**
     - Chips „Art“ („Flags“ gewählt, „Variablen“);
     - Chips „Bereich“ („Alle“ gewählt, „kartenlokal“, „allgemein“, „versteckte Items“, „Trainer“, „System“, „Tagesflags“);
     - drei Kästchen („nur unbenutzt“, „nur frei“, „nur mit eigenem Namen“);
     - die Suche „Name oder Nummer suchen“ (360 breit).
   - **Liste und Detail:** darunter nebeneinander (bis 1280 breit, hier 1032; je Spalte 508, Abstand 16).
     - Links die Karte der Liste: Kopf mit „Live“ (rechts, `status/warnung`, 600), sechs Zeilen und der Fuß „6 von 4.097 · Script-lokale Variablen … (nicht im Index)“.
     - Die Zeilen sind: unbenannt und kartenlokal (`Flag 0x0003`, „an 54 Orten“), eigener Name `FLAG_HAT_MEDAILLE` (fett, darunter `FLAG_UNUSED_0x0094 · 0x0094 · 148`, gewählt), pret-Name (`FLAG_HIDE_TWINLEAF_MOM`, „gesetzt 1 · geprüft 3“), frei (Plakette „frei“, „unbenutzt“), mit unklarem Zugriff („gesetzt 2 · geprüft 1 · unklar 1“) und eine unbenannte Nummer mit Verwendung. Die Zelle „Live“ trägt „–“.
     - Rechts das Detail von `FLAG_HAT_MEDAILLE`: Kopf mit Name, pret-Name, „0x0094 · 148 · allgemein“, Beschreibung und den Tasten „Umbenennen…“ und „Beschreibung…“; „Wo gesetzt“ (1) mit der Gruppe „Scriptdatei 0412“ und dem Chip „Script 1 · SetFlag“; „Wo geprüft“ (2) mit „Eventdatei 0390“ („Objekt 3 · Sichtbarkeits-Flag“) und „Scriptdatei 0040“ („Script 2 · CheckFlag“).
   - **Statusleiste:** „101 Warnungen, 10 Hinweise“, „Gebaut vor 3 Minuten“, „Komfort“ (die Standard-Statusleiste, ohne Git).
2. **Frame „App: Dialog, Flag wählen“** (`MODE = 'picker'`).
   - **Hintergrund:** Welt bauen, Figur bearbeiten (Werkbank) wie in `werkbank-figur.body.js`, verkürzt; im Inspector der Figur (Objekt 2) die Zeile „Sichtbarkeits-Flag“ nach „Sichtweite“: der Chip „Keins“ (ohne Nummer), „Wählen…“ und der Satz „Die Figur ist da, solange das Flag 0 ist.“
   - **Dialog:** darüber die Abdunklung (`flaeche/ueberlagerung`) und der Picker, 480 breit.
     - Titel „Flag wählen“, das Kreuz;
     - das Suchfeld „Name oder Nummer“ mit Lupe;
     - der Filter (`Segmented` S mit `wrap`): „Alle“ gewählt, „kartenlokal“, „allgemein“, „versteckte Items“, „Trainer“, „System“, „Tagesflags“ (zwei Zeilen);
     - die Treffer: „Keins (0)“ als erste Zeile und hervorgehoben (`auswahl/flaeche`, `akzent/text`), dann `Flag 0x0003` (kartenlokal), `FLAG_HAT_MEDAILLE` (fett, darunter `FLAG_UNUSED_0x0094`), `FLAG_HIDE_TWINLEAF_MOM`, `FLAG_UNUSED_0x00F0`, `FLAG_VILLA_FURNITURE_TABLE`; je Zeile das Symbol der Fläche Geschichten, der Name, die Nummer in Mono, rechts die Verwendung;
     - der Fuß (Linie `linie/leise` darüber): die leisen Tasten „Freies Flag nehmen“ und „Im Verzeichnis zeigen“ (S).
3. **Brett „Komponenten (App): Flag-Picker und Flag-Feld“** (`MODE = 'brett'`), für die Seite „Komponenten“. Es zeigt:
   - das Flag-Feld in Bauen (Name mit Nummer, „Keins“, unbenannt; die Variable und der Wert eines Auslösers, mit den Sätzen der Zeilen);
   - das Flag-Feld in Tüfteln (dazu das Zahlfeld mit Dez | Hex);
   - das Flag-Feld in der Tabelle des Level-Scripts (ohne Bezeichnung);
   - die Zeile des Pickers in vier Zuständen („Keins (0)“, eigener Name, pret-Name, unbenannt);
   - den Fuß des Pickers in vier Zuständen (offen, gesperrt ohne übernommene Scripts, gesperrt bei leerem Pool, beim Lesen), je mit dem Satz daneben;
   - die Zeile der Liste in sieben Zuständen (eigener Name, pret-Name, unbenannt, frei, reserviert, kartenlokal, gewählt), je mit dem Satz daneben. „Reserviert“ steht als Variable (`VAR_UNUSED_0x406C`, die Vorlage „Leere Region“ nimmt sie aus „frei“), denn nur Variablen sind reserviert.

## Lage und Einspielen

- **Lage:**
  - Frame 1 kommt auf die Seite „App: Texte, Technik, Dialoge“ in die nächste freie Spalte der Reihe der Technik-Screens (Reihe 0), Frame 2 in die nächste freie Spalte der Reihe der Dialoge (Reihe 1). Es gilt x = Spalte × 1520, y = Reihe × 1000. Zuletzt belegt waren nach 4.4 in Reihe 0 x 18240 und in Reihe 1 x 15200; die Frames von 4.3b, 4.4 und aller Phasen, die bis dahin eingespielt sind, kommen zuerst.
   - Das Brett kommt auf die Seite „Komponenten“ unter das letzte Brett „Komponenten (App)“ (nach 4.4: „Änderungstabelle und Git“ bei x 3440, y 15380).
  - Vorher mit `get_page_tree` (depth 1) die belegten Positionen ablesen und nur eigene Frames anfassen.
- **Bauen:**
  1. Aus `openpencil/beispiele/` `bash ../build-screen.sh flags-variablen` aufrufen (für Dunkel mit `--dunkel`). Das Skript schreibt nur die `.jsx` neben die Vorlage und braucht OpenPencil nicht.
  2. In der erzeugten `flags-variablen.jsx` `MODE` je Frame umstellen (`'liste'`, `'picker'`, `'brett'`).
  3. `node ../../scripts/op.mjs render '{…}' --jsx flags-variablen.jsx` aufrufen.
  4. Per `export_image` vergleichen (`node ../../scripts/op.mjs export_image '{…}' --png bild.png`), mit den Screenshots aus `target/agent/gate-<datum>/3d-1/` (`blick-<fläche>-<zustand>-<thema>.png`): die Fläche, die Zeile mit eigenem Namen, das Detail mit „Wo gesetzt“ und „Wo geprüft“, der Picker im Inspector einer Figur. Die Screenshots zeigen echte Werte des Testprojekts, die Frames Beispielwerte. Verglichen werden Aufbau, Maße und Texte.
  5. Korrigieren: den Body ändern, neu bauen, `render` mit `"replace_id":"<ID>"` statt `x`/`y` (höchstens drei Läufe je Frame). Die erzeugten `.jsx` und `.png` nicht einchecken, sondern löschen.
- **Danach:**
  1. Speichern (der Hauptagent, `save_file`) und das Dokument schließen. Ab 10.000 Knoten lehnt OpenPencil Einzeländerungen ab; dann die Frames über `render` mit `replace_id` neu setzen. Bei gesperrtem Bildschirm laufen die RPCs in Zeitüberschreitungen.
  2. Aus der Repo-Wurzel `node design-system/scripts/fix_fig_text.mjs design-system/Editor_DesignSystem.fig --rgb555` aufrufen.
  3. Im README den Abschnitt „Nachtrag 3d.1“ um einen Satz ergänzen, dass die Frames seit … in der Datei stehen (mit Seite und Lage), und den Satz „Die Frames von 3d.1 folgen, sobald OpenPencil antwortet“ dort, „Die Datei wurde nicht angefasst“ und „nicht gerendert“ hier ersetzen. Den Stand der Zeile in `Audit-Design-System.md` (Screen 13) nachziehen.
  4. Im Brett „Stand der App“ (Seite „Titel“, x 1720, y 0, Vorlage `beispiele/stand-der-app.body.js`, per `render` mit `replace_id`) eine Zeile „Flags und Variablen (3d.1)“ unter „Teilweise umgesetzt“ anlegen, mit dem Verweis auf die Frames. Zum Screen 13 des Zielbilds trägt die Marke dann „Teilweise“.
  5. Im Brett „Komponenten (App)“ das Symbol `flag` in die Symbolreihe nehmen.

  `check-tokens.sh` ist nicht betroffen. Die Frames der Liste „Technik“ aus 4.2 bis 4.4 zeigen acht Einträge und bleiben so: sie sind Bilder ihres Standes, nicht der heutigen App.

## Was nicht gezeichnet werden muss

- **Dialog „Als eigenes Flag benennen“** (und „Als eigene Variable benennen“, „Beschreibung“): ein Dialog im Aufbau von „Git einrichten?“ (`beispiele/aenderungen-git.body.js`) mit dem Feld „Eigener Name“ (vorbefüllt `FLAG_` oder `VAR_`, Hinweis „A–Z, 0–9 und _; der pret-Name FLAG_UNUSED_0x0094 bleibt gültig.“), dem mehrzeiligen Feld „Beschreibung“, „Abbrechen“ und „Benennen“ (primär; „Speichern“ bei „Beschreibung“). Die Ablehnung des Kerns steht als Fehlermeldung unter dem Feld.
- **Level-Script:** das Feld „Variable“ in der Tabelle „Variablen-Bedingungen“ und in der Zeile „Neue Variable“ ist das Flag-Feld ohne Bezeichnung, wie es das Brett zeigt; die Tabelle selbst gab es vorher.
- **Leere und Randzustände der Fläche:**
  - „Kein Projekt geöffnet.“ mit „Projekt öffnen“;
  - „Wird gelesen …“;
  - „Nichts passt zu Filter und Suche.“;
  - die Notiz bei mehr als 500 Zeilen („Die ersten 500 Zeilen; Suche und Filter erreichen die übrigen.“);
  - die Fehlermeldung eines Lesefehlers;
  - die Info „Eigene Namen gibt es im Textprojekt: …“ im DSPRE-Layout;
  - die Info „Scripts sind noch nicht übernommen: Script-Verweise fehlen im Index, „frei“ ist darum nur ein Hinweis.“

  Es sind die vorhandenen Bausteine Meldung und leerer Zustand.
- **Zustände des Details:**
  - ohne Auswahl „Wähle links ein Flag oder eine Variable.“;
  - gesperrte Tasten mit Grund (DSPRE-Layout, „Liegt außerhalb des Spielstands.“, „Erst Speichern oder Bauen abwarten“);
  - kartenlokal gruppiert nach Header („Zweiblattdorf (0411)“, in Tüfteln mit der `MAP_…`-ID);
  - „Unklar“ als dritter Abschnitt;
  - die Hinweise „Nirgends gesetzt.“, „Nirgends geprüft.“ und „Gelesen, aber nirgends gesetzt: …“;
  - „Wird gelesen …“ und die Fehlermeldung.

  Es sind dieselben Abschnitte und Chips mit anderem Inhalt.
- **Variablen:** die Fläche mit „Variablen“ gewählt (Bereiche „kartenlokal“, „Objektgrafik“, „allgemein“, „Script-lokal“) und der Picker „Variable wählen“ mit „Freie Variable nehmen“ sind dieselben Frames mit anderen Wörtern.
- **Palette:** der Eintrag „Flags und Variablen“ (Symbol `flag`, ohne Projekt gesperrt mit „Erst ein Projekt öffnen“) ist ein Eintrag der vorhandenen Palette.
- **Schmal:** Unter 760 px Breite der Aufteilung steht das Detail unter der Liste.
- **Dunkel und Kompakt:** Die Bausteine haben ihre Zustände schon in der Datei; Frame 1 lässt sich mit `--dunkel` bauen, wenn ein dunkler Vergleich gewünscht ist.
