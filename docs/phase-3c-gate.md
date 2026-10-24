# Phase 3c: Gate prüfen

Stand: 2026-10-24. 3c.1 und 3c.2 sind implementiert und automatisch getestet. Der Agent hat (A) bis (G) am 2026-10-24 über die Test-Umgebung abgenommen (Browser-Modus mit `studio-bridge`, `g4emu`; Branch der Abnahme mit sieben Fix-Commits, siehe „Befunde der Abnahme“); offen beim Nutzer sind die Blicke auf die Screens und auf Dunkel/Kompakt, rotom-lsp ((C) 6) und die gepackte App (H). Ergebnisse unten.

Phase 3c (Doppeldeck: das Design-System in der App, der Rahmen aus Titelleiste, Atlas, Liste, Schau, Scharnier, Hand und Statusleiste, die Bereiche Welt und Texte, Script und Probleme im neuen Stil, Assistenten im Scharnier, Start, Technik, Einstellungen, Befehlspalette) bringt keine neuen Spieldaten-Fähigkeiten: alles, was die App vorher konnte, kann sie an seinem neuen Platz weiter. Das Gate prüft deshalb zweierlei: dass die Screens des Design-Systems in der App wiedererkennbar sind, und dass die Abnahmen von 3a, 3b.1 und 3b.2 mit den neuen Beschriftungen erneut grün laufen. Die Originale der Test-ROMs bleiben unberührt: alles läuft auf Kopien. Spezifikation: [docs/superpowers/specs/2026-10-23-phase-3c-doppeldeck-design.md](superpowers/specs/2026-10-23-phase-3c-doppeldeck-design.md), Abschnitt 1 (Gate). Pläne: [3c.1](superpowers/plans/2026-10-23-phase-3c1-doppeldeck-fundament.md) und [3c.2](superpowers/plans/2026-10-23-phase-3c2-doppeldeck-umzug.md).

Das Gate besteht aus acht Teilen:

- (A) Der Screen „Welt bauen: Figur bearbeiten“ ist in der App wiedererkennbar; ein Event wird gewählt, ein Feld geändert, ⌘Z nimmt es zurück, ⌘S speichert.
- (B) Texte: der Screen „Texte: Archiv 0412“ ist wiedererkennbar; eine Nachricht wird geändert, die DS-Vorschau zeigt sie seitenweise, die Suche findet einen angefügten Text.
- (C) Scripts: ein Script öffnet sich in der Hand (mindestens 400 px hoch), ein Compile-Fehler erscheint im Prüflicht und im Panel Probleme, der Sprung zur Zeile klappt; mit rotom-lsp Completion, Hover und Diagnosen.
- (D) Assistent „Raum anlegen“ als Schrittleiste im Scharnier; der Raum ist in `g4emu` begehbar.
- (E) Dunkles Thema und Dichte Kompakt: Galerie und Welt ohne unlesbare Stelle.
- (F) Tastatur: Atlas, Liste, Werkzeuge, Inspector und Scharnier mit Tab, Pfeiltasten und Enter; der Fokusring nur bei Tastaturbedienung.
- (G) Die Abnahmen 3a (A) bis (D), 3b.1 (A) und (D), 3b.2 (A) bis (F) erneut über die Test-Umgebung; `scripts/gate.sh` grün gegen alle drei Projekte, mit den Schritten `tokens` und `styles`; dazu Palette, Layout, Thema und Dichte, Technik › Patches.
- (H) Gepackte App auf macOS: Overlay-Titelleiste, Schriften, Thema folgt dem System.

Wer was prüft: Die Spalte „Wer“ der Ergebnistabelle sagt **Agent** (die Test-Umgebung aus [docs/agent-gate.md](agent-gate.md): Browser-Fenster mit `studio-bridge`, `call.sh`, `g4emu`) oder **Nutzer** (von Hand, im nativen Fenster oder mit DSPRE). Wo der Agent Bilder macht und der Nutzer nur hinsieht, steht beides. Der Browser-Modus ersetzt die gebündelte App; die Abweichungen (unten) kommen in „Auffälligkeiten“.

Namen in der Oberfläche (die ganze Tabelle steht in [docs/agent-gate.md](agent-gate.md), Abschnitt „Doppeldeck: wo was ist“):

- **Atlas** (`nav` „Bereiche“): Start, Welt, Geschichten, Pokémon, Gegenstände (diese drei gesperrt, „Kommt mit Phase 3b.3“), Texte, Technik, unten Hilfe und Einstellungen.
- **Welt:** Liste „Orte“; Schau mit den Reitern „3D“, „Raster“, „Weltkarte“; Hand mit „Werkzeuge“, dem Inspector (Reiter „Inspector“, „Script“, „Probleme“) und „An diesem Ort“. Das Panel „Text“ gibt es in der Welt nicht mehr: die Textarchive sind der Bereich Texte, die Referenz „Textarchiv“ im Inspector führt dorthin.
- **Texte:** Liste „Textarchive“ (Gruppen „Orte“ und „Spielweit“, Filter „Textarchive filtern“, „+“ heißt „Textarchiv anlegen“, unten die Summe „N Texte zu lang, in M geprüften Archiven“); Schau „Vorschau“ mit dem DS-Bildschirm (`aria-label` „DS-Bildschirm, Seite x von y“), der Zeile „Seite 1 von 2, 256 × 192, 2-fach“, „Zurück“ und „Weiterblättern“ und den Seiten als Karten („34 von 38“ je Zeile); Hand „Nachrichten“ mit der Suche „Texte suchen“, dem Umschalter „Dieses Archiv“ / „Alle“, „Nachricht anlegen“, der Tabelle (Nr., Text, Längste Zeile) und rechts der Spalte „Nachricht bearbeiten“ (Feld „Text der Nachricht“, „Verwendet in“, „Letzte Nachricht entfernen“); daneben der Reiter „Probleme“.
- **Script** (Reiter der Welt-Hand): Kopf mit Dokumenttitel, „Gliederung“, „Speichern“, „Script anfügen“ und der LSP-Plakette („rotom-lsp läuft“, „rotom-lsp fehlt“ …, `data-lsp`); offene Dokumente als Reiter „Offene Scripts“. Ohne Übernahme der Kasten „Scripts übernehmen“ mit „Übernehmen“.
- **Probleme:** Spalten Schwere, Datei (als Referenz, ein Klick springt), Zeile:Spalte, Meldung, Quelle („Compile“, „Text“, „live“); ohne Einträge „Keine Probleme.“
- **Scharnier:** Brotkrume, „Bauen“ / „Tüfteln“, Prüflicht („Alles stimmig“ oder „2 Fehler, 1 Warnung“, Klick öffnet „Probleme“), Verlauf („Verlauf, N Schritte“), „Wiederholen“, „Hier spielen“. Im Assistenten wird es zur Schrittleiste (Titel, nummerierte Schritte, „Abbrechen“, „Zurück“ ⌘[, „Weiter“ ↵ oder „Anlegen“); der Schritt steht in der Hand (Rolle `dialog`), das Ergebnis bietet „Zeigen“, „Hier spielen“ und „Schließen“.
- **Technik:** Liste „Prüfen und Bauen“, „Patches“, „Emulator“, „Werkzeuge“; rechts die Fläche dazu (`data-technik`).
- **Einstellungen:** „Darstellung“ mit „Thema“ (Hell, Dunkel, Wie System) und „Dichte“ (Komfort, Kompakt), „Emulator“, „Tastenkürzel“.
- **Befehlspalette:** ⌘K oder die Schaltfläche „Suchen oder Befehl eingeben“ in der Titelleiste; Gruppen Befehle, Orte, Header, Textarchive, Scripts.

Was im Browser-Modus anders ist als in der gebündelten App:

- „Hier spielen“ baut nur (Toast „Im Browser-Modus wird nur gebaut“), ⌘R gehört dem Browser; in Technik › Prüfen und Bauen ist „Starten“ gesperrt.
- rotom-lsp läuft nicht: die Plakette sagt „rotom-lsp fehlt“, der Editor hat keine Completion, kein Hover, keine Diagnosen. Compile-Fehler beim Speichern kommen trotzdem (sie stammen von rotom, nicht vom Sprachdienst).
- ⌘S kann die Seite statt der App treffen; dann „Speichern“ in der Titelleiste oder `call.sh save`.
- Rückfragen erscheinen als Dialog „Rückfrage“ mit „Nein“ und „Ja“, Pfadabfragen mit dem Feld „Pfad“, „Abbrechen“ und „OK“.

## Vorbereitung

1. Kopien anlegen (nie ein Original öffnen; das Ziel darf noch nicht existieren):
   `cp -R "../Roms/Pokemon - Platin-Edition (Germany)_DSPRE_contents" ../Roms/_testkopien/pt-3c`
   Für (D) eine zweite Kopie `pt-3c-raum` und für das Szenario eine dritte `pt-3c-szenario`; für die Abnahmen in (G) je eine frische Kopie mit der Endung `-3c` (zum Beispiel `pt-3a-3c`, `hg-3a-3c`, `pt-3b1-3c`, `pt-3b1-rep-3c`, `hg-3b1-3c`, `pt-3b2-3c`, `hg-3b2-3c`), weil die Kopien der früheren Läufe schon geändert sind.
2. Test-Umgebung starten, wie in [docs/agent-gate.md](agent-gate.md), „Je Lauf“. Aus einem Git-Worktree muss `G4_ROMS_DIR` absolut gesetzt sein: `G4_ROMS_DIR=<absoluter Pfad zu Roms> bash scripts/agent/studio-web.sh > target/agent/studio-web.log 2>&1 &` (der Standard `../Roms` liegt neben dem Haupt-Checkout, nicht neben dem Worktree). Seite: `http://127.0.0.1:1430/?project=<absoluter Pfad der Kopie>`.
3. Vor einem Lauf den gespeicherten Rahmen leeren, damit das Standard-Layout gilt: im Browser `localStorage.clear()` und neu laden.
4. Für (H): `cd apps/studio && pnpm bundle && bash scripts/check-bundle.sh` (Ausgabe `bundle ok`, siehe [docs/release.md](release.md)).
5. Bilder: Der Agent hält Screenshots klein (halbe Größe) und beschreibt Ort und Zahlen, damit der Blick des Nutzers kurz bleibt. Die Vorlage der Screens ist die Seite „Screens: Start und Welt“ bzw. „Screens: Gegenstände, Texte, Testen“ in `design-system/Editor_DesignSystem.fig` (OpenPencil).

Fakten zu Platin DE, die der Ablauf braucht: Zweiblattdorf ist Header 0411 (Eventdatei 390) mit den Räumen 0412 bis 0417, das Spielerzimmer ist 0415 (Eventdatei 394); Jubelstadt ist Header 0003. Textarchiv 0023 benutzen Header 0001 („NOTHING“, in 3a die Route 230) und Jubelstadt. Ein neuer Header bekommt auf Platin DE die Nummer 0593.

## (A) Welt bauen: Figur bearbeiten

Auf `pt-3c`, Bereich Welt:

- [ ] 1. In der Liste „Orte“ Zweiblattdorf öffnen, Header 0411 wählen; die Schau zeigt „3D“. Screenshot des ganzen Fensters. Zu sehen sein müssen: Atlas mit „Welt“ aktiv, Liste „Orte“, oben links in der 3D-Ansicht die schwebende Leiste mit „Gelände“ und „Ereignisse“, unten links „Legende“; das Scharnier mit Brotkrume (Ort › Header), „Bauen“ / „Tüfteln“, Prüflicht, Verlauf und „Hier spielen“; die Hand mit „Werkzeuge“, Inspector und „An diesem Ort“; die Statusleiste mit Werkzeug-Hinweisen, Kontext und Build-Stand.
- [ ] 2. In „An diesem Ort“ eine Figur anklicken (oder mit dem Werkzeug „Auswählen“ in der 3D-Ansicht): Inspector und Brotkrume zeigen ihren Titel, die Figur hat in der Szene den Auswahl-Halo. Screenshot; der Vergleich mit dem Screen „Welt bauen: Figur bearbeiten“ ist ein Blick des Nutzers.
- [ ] 3. Im Inspector „Blickrichtung“ ändern (zum Beispiel von „Unten“ auf „Links“): die Titelleiste sagt „1 Änderung nicht gespeichert“, der Verlauf „Verlauf, 1 Schritt“; `call.sh dirty` nennt die Eventdatei.
- [ ] 4. Fokus aus dem Feld nehmen, ⌘Z: das Feld steht wieder auf dem alten Wert, die Titelleiste sagt „Alles gespeichert“, `call.sh dirty` liefert `[]`.
- [ ] 5. Erneut ändern, ⌘S (trifft es die Seite, „Speichern“ in der Titelleiste): „Alles gespeichert“; `call.sh get_events '{"eventFileId":390}'` zeigt den neuen Wert.
- [ ] 6. Werkzeuge getrennt prüfen: mit „Auswählen“ (V) auf eine Kachel klicken ändert nichts (`call.sh dirty` leer); mit „Gelände malen“ (B) setzt ein Klick die Kollision (`dirty` nennt die Karte), ⌘Z nimmt es zurück.
- [ ] 7. „Hier spielen“: im Browser-Modus baut es nur (Toast „Im Browser-Modus wird nur gebaut“, dann „Gebaut vor …“ in der Statusleiste); während des Baus steht „Baut …“ und die Taste ist gesperrt. Bauen und Starten in der gebündelten App prüft (H).

## (B) Texte

Auf `pt-3c`, Bereich Texte (Atlas „Texte“):

- [ ] 1. In der Liste „Textarchive“ ein Archiv der Gruppe „Orte“ wählen (für den Vergleich mit dem Screen „Texte: Archiv 0412“), dann eine Nachricht in der Tabelle: die Vorschau zeigt sie auf dem DS-Bildschirm, rechts die Seiten als Karten mit Zeichenzahl je Zeile, unten die Spalte „Nachricht bearbeiten“. Screenshot; der Vergleich ist ein Blick des Nutzers.
- [ ] 2. Archiv 0023 wählen (im Filter „Textarchive filtern“ `0023` tippen), eine Nachricht mit mehreren Seiten wählen. Mit „Weiterblättern“ und „Zurück“ (oder den Pfeiltasten in der Vorschau) blättern: der `aria-label` des Bildschirms wechselt zwischen „DS-Bildschirm, Seite 1 von N“ und „Seite 2 von N“, die gewählte Karte wechselt mit.
- [ ] 3. Im Feld „Text der Nachricht“ den Text ändern und das Feld verlassen: Tabelle und Vorschau zeigen den neuen Text; Enter schreibt einen Zeilenumbruch, ⇧Enter eine neue Seite (die Vorschau hat dann eine Seite mehr).
- [ ] 4. „Nachricht anlegen“: eine leere Nachricht am Ende; in „Text der Nachricht“ zum Beispiel „Barry wartet im Studio“ tippen.
- [ ] 5. Speichern; `call.sh get_text_archive '{"index":23}'` zeigt die geänderte und die angefügte Nachricht.
- [ ] 6. Suche „Texte suchen“ mit „Alle“: `Barry` findet Treffer in mehreren Archiven, darunter die angefügte Nachricht in Archiv 0023 (Spalten Archiv, Nr., Text); Enter oder Klick auf den Treffer wählt Archiv und Nachricht. Escape im gefüllten Suchfeld leert es zuerst.
- [ ] 7. „Letzte Nachricht entfernen“ nimmt die angefügte Nachricht wieder weg, ⌘Z holt sie zurück.

## (C) Scripts

Auf `pt-3c`, ohne rotom-lsp (Agent):

- [ ] 1. Scripts übernehmen: in der Welt-Hand Reiter „Script“, Kasten „Scripts übernehmen“, „Übernehmen“; der Bericht nennt die Zahl der Dateien.
- [ ] 2. Hand auf die Standardhöhe (Doppelklick auf den Griff des Scharniers klappt ein und wieder aus, oder `localStorage.clear()` vor dem Lauf). Ein Script öffnen: im Explorer unter Header 0415 das Kind „Scriptdatei NNNN“. Die Hand steht danach mindestens 400 px hoch: `document.getElementById("dock-welt-hand").getBoundingClientRect().height` ≥ 400.
- [ ] 3. „Script anfügen“, im neuen Rumpf vor `End` die Zeile `AddItemm 17, 1` tippen, speichern („Speichern“ im Script-Kopf): das Prüflicht zeigt „1 Fehler“, der Reiter „Probleme“ eine Zeile mit Quelle „Compile“, Datei „Scriptdatei NNNN“, Zeile und Spalte und der Meldung zu `AddItemm`.
- [ ] 4. Klick auf das Prüflicht öffnet „Probleme“; Klick auf die Datei der Zeile springt im Editor an die Fehlstelle.
- [ ] 5. Tippfehler beheben (`AddItem`), speichern: die Zeile verschwindet, das Prüflicht sagt „Alles stimmig“.

Mit rotom-lsp (Nutzer, gebündelte App oder `pnpm tauri dev -- --bin studio` mit `G4_ROTOM_LSP`):

- [ ] 6. Die Plakette sagt „rotom-lsp läuft“; Completion bei `Mes`, Hover über einen Befehl zeigt die Signatur, ein Tippfehler zeigt eine Diagnose im Editor.

## (D) Assistent „Raum anlegen“

Auf `pt-3c-raum`:

- [ ] 1. In der 3D-Ansicht von 0411 die Maus auf eine freie Kachel halten, dann in der Liste „Orte“ „Neu anlegen“ (+) → „Raum anlegen…“ (oder Palette „Raum anlegen…“). Das Scharnier wird zur Schrittleiste mit dem Titel „Raum anlegen“ und den Schritten Vorlage, Header, Eingang, Zusammenfassung; die Hand zeigt den Schritt, die Schau bleibt bedienbar. Screenshot.
- [ ] 2. Escape mit Draft: im Schritt Vorlage ins Feld „Name des neuen Raums“ tippen (nicht Enter), Escape: der getippte Text ist zurückgenommen, der Assistent bleibt offen. Zweites Escape: der Assistent schließt, `call.sh dirty` ist leer. (Ebenso leert Escape im gefüllten Feld „Vorlage suchen“ zuerst die Suche.)
- [ ] 3. Neu öffnen und durchlaufen: Vorlage Header 0415, Schritt Header „Neuer Header“ (vorher „Patch anwenden“ im Kasten „Patch „Dynamische Header“ nötig“; oder „Vorhandenen Header umstellen“), Eventdatei „Kopie von 0394“, Eingang Außen-Header 0411 mit der Kachel aus Schritt 1 und „Kacheln, auf denen ein Warp nicht auslöst, zur Tür machen (Verhalten 0x69, Kollision 0x80)“ an. Weiter mit Enter oder „Weiter“, zurück mit ⌘[. Die Zusammenfassung zeigt den Satz und die Plakette „Nicht rückgängig“.
- [ ] 4. „Anlegen“: das Ergebnis ersetzt den Schritt („Raum angelegt“, „Als Nächstes“), „Zeigen“ öffnet den neuen Raum, „Schließen“ beendet den Assistenten.
- [ ] 5. Begehbar: das Szenario auf `pt-3c-szenario` (aus `scripts/g4emu`):
  `uv run g4emu scenario ../agent/scenarios/pt-raum-assistent.yaml --project ../../../Roms/_testkopien/pt-3c-szenario --out ../../target/agent/gate-<datum>/raum --set NEUER_HEADER=593 --set TUER_X=105 --set TUER_Y=876`
  läuft unverändert grün (Header-Folge 415, 593, 411, 593).

## (E) Dunkles Thema und Kompakt

- [ ] 1. Galerie `http://127.0.0.1:1430/?gallery`: jede Seite (Tasten, Eingaben, Inhalt und Status, Schwebendes, Auswahl, Bildschirm, Rahmen) in Hell, Dunkel und Kompakt; Screenshots.
- [ ] 2. Welt mit `pt-3c` (Header 0411, eine Figur gewählt) in Dunkel und in Kompakt, dazu Texte mit Archiv 0023 und die Welt-Hand mit dem Reiter „Script“; Screenshots.
- [ ] 3. Blick des Nutzers: keine unlesbare Stelle (Text auf Fläche, Plaketten, Fokusring, Szenenfarben, CodeMirror im dunklen Thema).

## (F) Tastatur

- [ ] 1. Ohne Maus: im Atlas mit Tab von Eintrag zu Eintrag (die Einträge sind Tasten, gesperrte Bereiche reagieren nicht), Enter öffnet den Bereich; in der Liste „Orte“ Pfeiltasten und Enter (Roving Tabindex); in „Werkzeuge“ Pfeiltasten und die Buchstaben V, B, F, T, A, S, G; im Inspector Tab durch die Felder, Enter übernimmt; im Scharnier Tab über Brotkrume, „Bauen“ / „Tüfteln“ (Pfeiltasten), Prüflicht, Verlauf, „Hier spielen“.
- [ ] 2. Fokusring nur bei Tastaturbedienung: nach einem Tab liefert `document.activeElement.matches(":focus-visible")` `true` und der Ring ist zu sehen; nach einem Mausklick auf eine Taste liefert es `false`.

## (G) Abnahmen erneut und Automatik

- [ ] 1. 3a (A) bis (D) nach [phase-3a-gate.md](phase-3a-gate.md) über die Test-Umgebung (Zuordnung und Rezepte in [agent-gate.md](agent-gate.md), Abschnitt „Phase 3a“). Die Stoppuhr von 3a (A) misst der Nutzer.
- [ ] 2. 3b.1 (A) und (D) nach [phase-3b1-gate.md](phase-3b1-gate.md). „Patch reparieren“ (3b.1 (A) 18) über Technik › Patches statt im Assistenten: die Plakette „Fehlerhaft“, die Taste „Patch reparieren“, danach „Angewendet“. Die DSPRE-Gegenprobe (3b.1 (A) 12 bis 15) bleibt beim Nutzer.
- [ ] 3. 3b.2 (A) bis (F) nach [phase-3b2-gate.md](phase-3b2-gate.md); das Panel „Text“ ist jetzt der Bereich Texte, die Beschriftungen stehen in [agent-gate.md](agent-gate.md), Abschnitt „Phase 3b.2 im Browser-Modus“. Completion, Hover und Diagnosen im Editor prüft der Nutzer.
- [ ] 4. Palette: ⌘K, `0412` tippen: Treffer in „Header“ (Header 0412 in Zweiblattdorf) und in „Textarchive“ (Ortsname, Kennung grau, „Textarchiv, verwendet von Header …“); Enter auf dem Header öffnet ihn in der Welt, Enter auf dem Archiv öffnet es im Bereich Texte. Ohne Projekt zeigt die Palette nur Befehle; „Speichern“, „Bauen“ und „Hier spielen“ sind dann gesperrt.
- [ ] 5. Layout: in der Welt ein Panel verschieben, neu laden: es bleibt (Schlüssel `g4.layout.v8.welt.schau` und `g4.layout.v8.welt.hand` im `localStorage`, keine `g4.layout.v7.*` mehr). Palette „Layout zurücksetzen“: Schau und Hand stehen wie beim ersten Start.
- [ ] 6. Thema und Dichte: Einstellungen › Darstellung, „Dunkel“ und „Kompakt“ wählen: `<html>` trägt `data-theme="dunkel"` und `data-dichte="kompakt"`, `localStorage` hat `g4.theme` = `dunkel` und `g4.density` = `kompakt`; nach dem Neuladen gilt beides. „Wie System“ entfernt `data-theme`. Der Knopf „Thema wechseln“ in der Titelleiste und die Palette („Thema wechseln“, „Dichte wechseln“) schalten dasselbe.
- [ ] 7. `scripts/gate.sh` läuft grün gegen Platin DE, HeartGold DE und HeartGold US, mit den Schritten `tokens` (`scripts/check-tokens.sh`) und `styles` (`scripts/check-styles.sh`, Ausnahmeliste leer); keine Zeile „skipped“ außer den dokumentierten. Aus einem Worktree: `G4_ROMS_DIR=<absoluter Pfad zu Roms> scripts/gate.sh`. Die Tabelle aus der Ausgabe kommt unter „Ergebnis“.

## (H) Gepackte App

Von Hand mit `target/release/bundle/macos/Gen4 Studio.app` (`G4_APICULA` und `G4_ROTOM_LSP` nicht gesetzt) auf einer Testkopie:

- [ ] 1. Die Titelleiste liegt unter den Ampeln (Overlay, Einzug links), das native Menü ist da, das Fenster lässt sich an der Titelleiste ziehen.
- [ ] 2. Die Schriften sind geladen (Atkinson Hyperlegible Next in der Oberfläche, Zen Maru Gothic in Titeln wie „Willkommen zurück“, Pixelify Sans im DS-Bildschirm), die Konsole zeigt keine CSP-Verletzung (`Refused to …`).
- [ ] 3. Thema: mit „Wie System“ folgt die App dem Wechsel zwischen hellem und dunklem macOS-Erscheinungsbild ohne Neustart.
- [ ] 4. „Hier spielen“ (⌘R) speichert, baut und startet den Emulator; Technik › Prüfen und Bauen „Starten“ startet das zuletzt gebaute ROM.
- [ ] 5. rotom-lsp aus (C) 6, falls nicht schon in `tauri dev` geprüft.

## Ergebnis

Hier wird nach dem Durchlauf je Teil eingetragen: Datum, Projekt, Ergebnis, Auffälligkeiten. „Wer“ steht schon fest.

| Teil | Wer | Datum | Projekt | Ergebnis | Auffälligkeiten |
|---|---|---|---|---|---|
| (A) Welt bauen: Figur bearbeiten | Agent (Vergleich mit dem Screen: Nutzer) | 2026-10-24 | Platin DE (`pt-3c-gate`) | bestanden (Blick offen) | 1: Atlas „Welt“ aktiv, Liste „Orte“, Szenenleiste „Gelände“/„Ereignisse“, „Legende“, Scharnier „Zweiblattdorf › Außen 0411“, „Bauen“/„Tüfteln“, „Alles stimmig“, „Verlauf, 0 Schritte“, „Hier spielen“, Hand mit Werkzeugen, Inspector, „An diesem Ort“; Statusleiste „Auswählen · Klick wählt · Ziehen verschiebt“, „Zweiblattdorf, Raster 0“, „Noch nicht gebaut“, „Komfort“, „Browser-Modus (Brücke)“. 2: Objekt 0 (x 119, z 878) aus „An diesem Ort“: Brotkrume und Inspector „Objekt 0 · ID 0“; die 3D-Ansicht zeigt das ganze Raster 0, den Halo sieht man nur nach Heranzoomen (**Blick**: Header 0411, Objekt 0 bei x 119, z 878). 3: „Blickrichtung“ Links → Unten: „1 Änderung nicht gespeichert“, „Verlauf, 1 Schritt“, `dirty` Event 390. 4: ⌘Z: „Alles gespeichert“, `dirty` `[]`; ⌘Z lässt die Auswahl los (Events sind per Index gebunden, so gewollt), nach erneuter Auswahl steht „Links“. 5: ⌘S traf die App, `get_events` 390 Objekt 0 `dir` 1. 6: „Auswählen“-Klicks schicken kein `set_permission`; „Gelände malen“ auf schon blockierter Kachel ändert nichts, ⇧Klick löscht die Kollision (Karte 0175, Kachel 22, 13; `dirty` Map 175), ⌘Z nimmt es zurück. 7: Der erste „Hier spielen“ fragt „Speichern unter“ (vorbelegt `out.nds`), dann „Baut …“ (Taste gesperrt), „ROM gebaut …“, „Im Browser-Modus wird nur gebaut“, Statusleiste „Gebaut gerade eben“. |
| (B) Texte | Agent (Vergleich mit dem Screen: Nutzer) | 2026-10-24 | Platin DE (`pt-3c-gate`) | bestanden nach Fix (Blick offen) | 1: Filter `0412` zeigt „Zweiblattdorf 0555“ (Orte) und „Textarchiv 0412“ (Spielweit); 0555 Nachricht 001: DS-Bildschirm, fünf Seitenkarten, Spalte „Nachricht bearbeiten“ (**Blick**: Archiv 0555, Nachricht 001 gegen „Texte: Archiv 0412“). 2: 0023 Nachricht 001: „Seite 1 von 3“ → „Weiterblättern“ 2 von 3 → → 3 von 3 → ← 2 von 3 → „Zurück“ 1 von 3, die gewählte Karte wechselt mit. 3: Text geändert, Enter schreibt `\n`, ⇧Enter `\r`: 3 → 4 Seiten. 4: „Nachricht anlegen“ → 101, „Barry wartet im Studio“ (das Feld bekommt danach keinen Fokus). 5: `get_text_archive` 23: 102 Nachrichten, 001 geändert, 101 angefügt. 6: „Barry“ mit „Alle“: ein Treffer `23:101` (in Platin DE steht der Rivale sonst als Platzhalter, die Annahme „mehrere Archive“ trifft nicht zu); „Lucius“: 16 Treffer in 0023, 0054, 0060, 0143; Klick bzw. Enter auf `54:10` wählt „Archiv 0054, Fleetburg › Nachricht 010“; Escape leert die Suche zuerst. 7: „Letzte Nachricht entfernen“ → 101 Nachrichten, ⌘Z → 102. Befund (behoben, 1b74173): Archiv 0023 wird von 190 Headern benutzt, „Verwendet in“ schob die Taste rund 5400 px nach unten. |
| (C) Scripts ohne rotom-lsp | Agent | 2026-10-24 | Platin DE (`pt-3c-gate`) | bestanden nach Fix | 1: „Übernehmen“ mit Fortschritt „0 / 1124“ … „1124 / 1124 Scriptdateien“, Bericht „1124 Scriptdateien dekompiliert, 0 leere übersprungen.“ 2: Header 0415, „Scriptdatei 1056“: Hand 286 → 400 px. 3: „Script anfügen“ (`script_11 #11`), `AddItemm 17, 1`, „Speichern“: Prüflicht „1 Fehler“, Probleme „Scriptdatei 1056 · 422:2 · Analysis error: Unknown command 'AddItemm' · Compile“. 4: Prüflicht öffnet „Probleme“, Klick auf die Datei setzt den Cursor auf Zeile 422 (Spalte 1, die Meldung sagt 2). 5: `AddItem`, „Speichern“: „Alles stimmig“, „Keine Probleme.“ Befunde (behoben): „Öffnen“ am Event und jeder Sprung in eine noch nicht gezeigte Datei ließ die Ansicht auf Zeile 1 (31e0512); der Kopf des Script-Panels brach in der Hand-Spalte dreizeilig um (05b2b06). |
| (C) Scripts mit rotom-lsp | Nutzer | | | offen (Hand) | im Browser-Modus läuft rotom-lsp nicht (Plakette „rotom-lsp fehlt“) |
| (D) Assistent „Raum anlegen“ | Agent | 2026-10-24 | Platin DE (`pt-3c-gate-raum`, `pt-3c-gate-szenario`) | bestanden | 1: Schrittleiste „Raum anlegen“ mit „1 Vorlage“ … „4 Zusammenfassung“ (`aria-current="step"`), „Abbrechen“, „Weiter“ ↵; der Schritt ist ein `dialog` „Raum anlegen“, die Schau bleibt bedienbar. 2: „Neuer RaumXYZ“ + Escape → „Neuer Raum“, Assistent offen; zweites Escape schließt, `dirty` `[]`; Escape in „Vorlage suchen“ leert zuerst. 3: über die Palette „Raum anlegen…“: 0415, „Patch anwenden“ im Kasten, „Neuer Header“ (0593 `NEUER_RAUM`), „Kopie von 0394“, Außen 0411, Kachel 105, 876 getippt (der Hover vor dem Öffnen lag nicht über Raster 0: „Noch keine Kachel …“), Tür-Kacheln an; Enter auf „Weiter“ und ⌘[ geprüft; Zusammenfassung mit „Nicht rückgängig“. 4: „Raum angelegt“ (Raster 0289, Karte 0666, Eventdatei 0534, Warp 4 in 0411, Rückweg Warp 0), „Zeigen“ öffnet „Zweiblattdorf › Raum 0593“ und schließt den Assistenten. 5: Szenario `pt-raum-assistent.yaml` grün, Header-Folge 415, 593, 411, 593; Bilder `target/agent/gate-2026-10-24/raum/raum.png`, `draussen.png`, `wieder-drin.png`. |
| (E) Dunkel und Kompakt, Screenshots | Agent | 2026-10-24 | Galerie, Platin DE (`pt-3c-gate`) | bestanden | Galerie: alle sieben Seiten in Dunkel (dazu Menü „Verlauf“ und drei Toasts), Tasten in Hell, Eingaben und Inhalt und Status in Kompakt; Welt mit Header 0411 und Objekt 0 in Kompakt hell und Kompakt dunkel, Reiter „Script“ mit Scriptdatei 1052 dunkel (CodeMirror), Texte 0023 Nachricht 000 dunkel kompakt. Keine unlesbare Stelle gefunden; der DS-Bildschirm bleibt in beiden Themen hell (Szenenfarbe). Die Bilder liefert das Browser-Fenster nur dem Agenten (keine Dateien). Befund (behoben, 0ca2565): die Brotkrume kürzte bei 1100 px die letzte Stelle auf einen Buchstaben („… › O“). Der abgeschnittene Reiter „Inspector“ ließ sich nicht nachstellen: bis 100 px Spaltenbreite zeigt Dockview „Inspector“ ganz und die übrigen Reiter im Überlaufmenü. |
| (E) Dunkel und Kompakt, Blick | Nutzer | | | offen (Blick) | ansehen: `?gallery` mit „Dunkel“ und „Kompakt“ (alle Seiten), Welt mit Header 0411 und Objekt 0, Reiter „Script“, Texte 0023 |
| (F) Tastatur | Agent | 2026-10-24 | Platin DE (`pt-3c-gate`) | bestanden nach Fix | Atlas: Tab von „Start“ bis „Einstellungen“, die gesperrten Bereiche sind fokussierbar und reagieren auf Enter nicht, Enter auf „Welt“ öffnet. Liste: „Baum“/„Liste“ ein Tab-Halt, „Neu anlegen“, Suche, dann die Baumzeilen mit Roving Tabindex; ↓ ↓ Enter öffnet Header 0413. Scharnier: Griff, Brotkrume, „Bauen“/„Tüfteln“ (Pfeile bewegen den Fokus, Enter oder Leertaste wählt), Prüflicht, Verlauf, „Hier spielen“ („Wiederholen“ ohne Schritt gesperrt). Werkzeuge: ← → wechseln, Enter wählt, V B F T A S G wählen (↑ ↓ ohne Wirkung, die Liste ist waagrecht). Inspector: Tab durch Abschnittsköpfe, Referenzen und Felder, Enter übernimmt. Fokusring: nach Tab `:focus-visible` `true`, Umriss 2 px; nach Mausklick `false`. Befund (behoben, e04864f): Enter in einem Feld übernahm durch Verlassen, der Fokus fiel auf die Seite, das nächste Tab begann wieder in der Titelleiste. |
| (G) 3a (A) bis (D) | Agent (Stoppuhr: Nutzer) | 2026-10-24 | Platin DE (`pt-3a-3c`), HeartGold US (`hgus-3a-3c`) | bestanden nach Fix | (A) 1–4: Route 230 2 × 1 (R201 hat nur zwei Zellen), Header 0001 „Warps hierher: 0, wirkt unbenutzt“, Area-Data 6, Eventdatei neu (0534). 5–10: „Patch anwenden“ scheiterte erst mit der englischen Meldung „invalid project metadata: the header table has unsaved changes …“, ⌘S ist im Assistenten gesperrt (behoben, cd3ffe5: deutscher Hinweis mit Ausweg); danach Abbrechen, ⌘S, Raum 0593 (Kopie von 0394 → 0535, Warp 4 in 0411, Rückweg 0535/0 → 0411/4). 11: Werkzeug „Tür“ legt Warp 5 an (Rückfrage „zur Tür machen?“), x 110, z 880 und Ziel 0001/0 im Inspector getippt; in 0001 Warp 0 bei 10, 19 → 0411/5; die Tür-Kollisionen setzte der Agent per `set_permission` (Karte 0 und 0666), weil er in der 3D-Szene nicht gezielt auf eine Kachel zeigen kann. 12: `g4emu` 415 → 414 → 411 → 593 → 411 → 0001 (beide Karten, x 34) → 411; Bilder `target/agent/gate-2026-10-24/3a-a12/` (`wohnzimmer`, `draussen`, `raum`, `zurueck-draussen`, `route230`, `route230-karte2`, `zurueck-zweiblatt`). Zahlen getippt: 105, 876, 110, 880, 10, 19, 5 (Zeit des Agenten sagt nichts). (B): Suche, Liste (8 Zeilen), Hilfe-Glossar; die Liste hat ein eigenes Suchfeld und zählt nach dem neuen Header weiter „593 Header“. (C): Kartenbilder im Raster, „+“ → „Karte bei x 2, y 0 anlegen?“ mit „nicht rückgängig“, 3 × 1. (D): Klick → „Route 217 › Außen 0385“; Zweiblattdorf: 0413 und 0415 „ohne Eingang“, 0412, 0414, 0416, 0417 und 0593 verbunden; HG US: Weltkarte lädt, New Bark Town 0060 (T20). |
| (G) 3b.1 (A) und (D) | Agent (DSPRE: Nutzer) | 2026-10-24 | Platin DE (`pt-3b1-3c`, `pt-3b1-szen-3c`, `pt-3b1-rep-3c`), HeartGold DE (`hg-3b1-3c`) | bestanden nach Fix | (A) 1: `mapname.bin` 9488 Byte (593 Header), kein `arm9.bin.backup`. 2–4: „Neuer Header“ gesperrt, Kasten, „Patch anwenden“; Backup 1056448 Byte, `cmp` unterschiedlich, 593 Dateien in `dynamicHeaders`. 5–7: „Kopie von Header 0342 „Route 201““, `R231` → 0593, `dynamicHeaders/0593` 24 Byte, `mapname.bin` 9504, endet auf `R231`. 8: Route 232 „Behalten“ → 0594 mit Eventdatei 328 wie 0342. 9–10: Szenario `pt-neuer-header.yaml` grün (415 → 593, `target/agent/gate-2026-10-24/neuer-header/`). 11: `d_test.narc` (19092 Byte) im ROM. 16: HeartGold DE `HG_TEST` → 0540, `a/0/5/0` im ROM, `g4emu` 64 → 540 über die umgebogene Treppe (`target/agent/gate-2026-10-24/hg-3b1/hg-test.png`). 18 über Technik › Patches: „Verfügbar“ → Rückfrage → „Angewendet“; nach dem Fehler-Patch Toast „… fehlerhaft …“ mit „Patch reparieren“, `repairable: true`, Plakette „Fehlerhaft“; „Patch reparieren“ → „Angewendet“, Bytes `de f7 ad f8`, das Spiel läuft nach dem Intro (`target/agent/gate-2026-10-24/patch-rep/weiter.png`); Gegenprobe `pt-3b1-3c` ohne Meldung. Befund (behoben, c415ed7): nach der Reparatur in Technik blieb der Fehler-Toast stehen, die Meldung „… repariert …“ fehlte. (D): Hover auf Route 29 zeigt `Typ 0x06 HEADBUTT`; HG-Satz: `TALL_GRASS` 0x02, `WATER_RIVER` 0x10, `WATER_SEA` 0x15, `JUMP_*` 0x38–0x3B; Platin hat für 0x06 keinen Namen. DSPRE (12–15) und Diamant/Perl (17): Hand. |
| (G) 3b.2 (A) bis (F) | Agent (Completion, Hover, Diagnosen: Nutzer) | 2026-10-24 | Platin DE (`pt-3b2-3c`), HeartGold DE (`hg-3b2-3c`) | bestanden | (A): 1124 bzw. 965 Dateien, `rotom.toml`, `.rotom/…`, `expanded/scripts`, `unpacked/scripts` unverändert, Kasten nach Neuladen weg, „Bauen“ ohne Probleme. (B): Route 230 per Gebiet-Assistent auf 0001 (ohne Patch), „Neue Scriptdatei anlegen“ → 1124, gespeichert; Figur (F), Grafik 12, x 12, z 21, „Neues Script“ → 2, „Öffnen“ setzt den Cursor auf `script_2`; Script getippt, ⌘S „Alles stimmig“, 0023 Nachricht 101 „Hallo aus dem Studio!“; im Spiel `target/agent/gate-2026-10-24/3b2-npc/npc-text.png` (Treppe und Rückweg per `update_event`/`add_event`). (C): „Hier, ein Geschenk!“, dann „Mehr habe ich nicht.“ (`3b2-item/geschenk.png`, `zweites-mal.png`); Tippfehler und Sprung in (C) oben. (D): Mutter in „An diesem Ort“, „Öffnen“ → Scriptdatei 1055, Zeile 151 `script_4`; 0557 Nachricht 004 geändert, Script-„Speichern“ verweigert mit „Textarchiv 0557 erst speichern“, ⌘S speichert alles; im Spiel „Mutter: Das Studio war hier! …“ (`3b2-mutter/mutter.png`). (E): „Neues Level-Script anlegen“ → 1125, `script_3`, `on_var_condition` 16433 = 0 → 3 (`1125.json`); im Spiel „Willkommen auf Route 230!“ (`3b2-level/willkommen.png`). (F): „Verwendet in“, Überlängen-Hinweis „Zeile 6: …“ und Warnung im Prüflicht, „Textarchiv anlegen“ → 0724 sofort auf der Platte („Neuer Text“). Befund ohne Fix (nicht 3c): auf einer Kopie mit Patch „Dynamische Header“ (`pt-3a-3c`) meldet der Compile für ein Textliteral „Textliteral nicht auflösbar …“, obwohl `dynamicHeaders/0001` die Scriptdatei 1124 trägt. |
| (G) Palette, Layout, Thema und Dichte, Technik › Patches | Agent | 2026-10-24 | Platin DE (`pt-3c-gate`, `pt-3b1-rep-3c`) | bestanden | Palette `0412`: Header „Zweiblattdorf · T01R0101 · 0412“, Textarchive „Textarchiv 0412“ und „Zweiblattdorf · Textarchiv, verwendet von Header 0412 · 0555“; Enter öffnet den Header in der Welt bzw. das Archiv in Texte. Ohne Projekt nur Befehle, „Speichern“, „Bauen“, „Hier spielen“, die Assistenten mit „Erst ein Projekt öffnen“ gesperrt. Layout: Spaltenbreiten der Hand bleiben nach Neuladen, nur `g4.layout.v8.*` (ein gesetzter `v7`-Schlüssel ist weg), „Layout zurücksetzen“ stellt die Standardbreiten her. Einstellungen › Darstellung „Dunkel“/„Kompakt“: `data-theme="dunkel"`, `data-dichte="kompakt"`, `g4.theme`/`g4.density`, nach Neuladen gleich; „Wie System“ entfernt `data-theme` (`g4.theme` ist dann `system`); „Thema wechseln“ in Titelleiste und Palette, „Dichte wechseln“ in der Palette. Technik › Patches siehe 3b.1 (A) 18. |
| (G) `scripts/gate.sh` | Agent | 2026-10-24 | Platin DE, HeartGold DE, HeartGold US | bestanden (1704 s) | auf dem Stand mit allen Fix-Commits (c415ed7), mit den Schritten `tokens` und `styles` (Ausnahmeliste leer); keine Zeile „skipped“ |
| (H) Gepackte App | Nutzer | | | offen (Hand) | |

Ergebnis von `scripts/gate.sh` (Tabelle aus der Ausgabe):

```
Step                                                 Project                                      Result                     Seconds
----                                                 -------                                      ------                     -------
cargo fmt --check                                    -                                            ok                               0
cargo clippy (-D warnings)                           -                                            ok                               4
cargo test (no ROM data)                             -                                            ok                              26
pnpm test                                            -                                            ok                              59
tokens                                               -                                            ok                               0
styles                                               -                                            ok                               2
pnpm build                                           -                                            ok                              21
dist has no bridge                                   -                                            ok                               1
g4emu unit                                           -                                            ok                               2
g4emu integration                                    -                                            ok                               7
gated tests (cargo test)                             Pokemon - Goldene Edition HeartGold (Germany) ok                             529
gated tests (cargo test)                             Pokemon - HeartGold Version (USA)            ok                             561
gated tests (cargo test)                             Pokemon - Platin-Edition (Germany)           ok                             492

Gate: ok (1704 s)
```

Lauf vom 2026-10-24 im Worktree der Abnahme, `G4_ROMS_DIR` absolut gesetzt.

## Bekannt vor der Abnahme

Befunde aus der Umsetzung von 3c.2, die nicht als Fehler zählen oder bewusst offen sind:

- Die Zeichengrenze der Texte (38 je Zeile, Balken und „34 von 38“) zählt Zeichen; das Backend warnt nach Breite (200 px). Die Tabelle zeigt deshalb mehr gelbe Balken als es Warnungen im Panel Probleme gibt; die Plaketten der Archivliste zählen nur die Warnungen des Backends.
- Die Nachrichtenzahl steht nur am geladenen Archiv, nicht an jeder Zeile der Liste.
- Technik › Werkzeuge: apicula „Bereit“ folgt aus dem ersten Kartenbild (das auch aus dem Modell-Cache kommen kann), „Fehlt“ bleibt nach einem Fehler für die Sitzung; rotom-lsp zeigt den Zustand des Script-Panels, „Nicht gestartet.“, solange das Panel nicht gezeigt wurde. Pfade der Werkzeuge zeigt Technik nicht (kein Backend-Befehl liefert sie).
- Einstellungen › Tastenkürzel nennt nur die ⌘-Kürzel, nicht die Buchstaben der Werkzeuge.
- Im Raster zeigt auch Bauen die Kennungen der Zellen; Tüfteln zeigt zusätzlich Header-ID (dezimal) und Höhe, die Rohdaten der Zeile in der Hand sind u16 hex.
- Aus dem Durchgang Dunkel und Kompakt offen: der Kopf des Script-Panels bricht in Kompakt um; der Dockview-Reiter „Inspector“ wird in einer schmalen Spalte abgeschnitten; die Brotkrume der Texte wird in einem schmalen Scharnier abgeschnitten. Stand nach der Abnahme: Kopf und Brotkrume behoben (05b2b06, 0ca2565), der Reiter ließ sich nicht nachstellen (siehe (E)).

## Befunde der Abnahme (2026-10-24)

Behoben im Branch der Abnahme (je mit Test, `pnpm test` und `scripts/check-styles.sh` grün):

- 31e0512 Der Script-Editor zeigt beim ersten Anzeigen eines Dokuments die Zeile des Cursors (vorher blieb die Ansicht nach „Öffnen“ am Event auf Zeile 1).
- 05b2b06 Der Kopf des Script-Panels hat in einer Hand-Spalte zwei statt drei Zeilen: `PanelHead` bekommt einen Platz für den Zustand, die rotom-lsp-Plakette steht neben dem Titel, die Tasten darunter (Container-Abfrage).
- 0ca2565 In einem schmalen Scharnier geben die vorderen Glieder der Brotkrume zuerst nach; die Auswahl bleibt lesbar.
- 1b74173 „Verwendet in“ eines Textarchivs hat eine eigene Bildlaufleiste; „Letzte Nachricht entfernen“ bleibt erreichbar (Archiv 0023: 190 Header).
- e04864f Enter in einem Text- oder Zahlfeld übernimmt und lässt den Fokus im Feld (Tastaturbedienung des Inspectors).
- cd3ffe5 „Patch anwenden“ mit ungespeicherter Header-Tabelle sagt auf Deutsch, was zu tun ist (im Assistenten: abbrechen, speichern, neu öffnen), statt die englische Kernel-Meldung zu zeigen.
- c415ed7 „Patch reparieren“ in Technik › Patches (und im Schritt Header) schließt den Fehler-Toast und meldet „… repariert …“.

Beim Nutzer:

- Blick (A): Header 0411, Objekt 0 (x 119, z 878) gegen „Welt bauen: Figur bearbeiten“, dabei der Auswahl-Halo in der 3D-Ansicht (die Kamera zeigt das ganze Raster 0, heranzoomen).
- Blick (B): Texte, Archiv 0555 (Zweiblattdorf) Nachricht 001 gegen „Texte: Archiv 0412“.
- Blick (E): `?gallery` in „Dunkel“ und „Kompakt“, Welt mit Header 0411 und Objekt 0, Reiter „Script“, Texte 0023.
- Hand: (C) 6 rotom-lsp, (H) gepackte App, die Stoppuhr von 3a (A), DSPRE (3b.1 (A) 12 bis 15, 3b.2 (G)).

Kleinigkeiten, nicht behoben:

- Kernel, nicht 3c: Auf einem Projekt mit Patch „Dynamische Header“ löst der Compile ein Textliteral nicht auf („Textliteral nicht auflösbar …“), obwohl der gespeicherte Header in `unpacked/dynamicHeaders` die Scriptdatei nennt (gesehen auf `pt-3a-3c`, Header 0001, Scriptdatei 1124; ohne Patch auf `pt-3b2-3c` geht es). Behoben in ac7c1f2: rotoms `compile_project` las die Header über uxie aus der alten Tabelle in `arm9.bin` (nur 559 bzw. 540 Einträge); der Kernel kompiliert jetzt mit seiner eigenen Header-Tabelle (`scripts/project_compile.rs`). Gegenprobe auf `pt-dynhdr-bug`: `save_script` 1124 ohne Probleme, Archiv 0023 bekommt Nachricht 101 „Hallo aus dem Studio!“.
- Die 3D-Ansicht zeigt immer das ganze Raster, Treffer für Hover und Werkzeuge gibt es nur über geladenen Karten; nach einem Wechsel des Headers zeigt der Inspector noch den letzten Kachel-Hover einer anderen Karte („Nur Ansicht“).
- „Werkzeuge“: ↑ und ↓ tun nichts (die Liste ist waagrecht, in Kompakt stehen die Werkzeuge in zwei Reihen).
- Dockview meldet über seine Live-Region auf Englisch („Weltkarte opened“, „Probleme opened“).
- „Baue ROM ...“ mit drei Punkten statt „…“ (Technik schreibt „Baue ROM …“).
- Nach „Nachricht anlegen“ hat das Feld „Text der Nachricht“ keinen Fokus.
- Listenansicht der Orte: eigenes Suchfeld (die Suche des Baums geht beim Umschalten verloren) und nach einem neuen Header noch „593 Header“.
- Die Palette findet „Scriptdatei 1052“ nicht, nur „1052“.
- Der Sprung aus „Probleme“ setzt den Cursor an den Zeilenanfang, nicht in die gemeldete Spalte.
- „Zeigen“ im Ergebnis eines Assistenten beendet den Assistenten mit (das Gate sagt „Schließen beendet“).
- Einmal lag die Kurzinfo „Gelände“ offen über dem Reiter „Raster“ und nahm den Klick (nicht nachgestellt).
- In HeartGold hat der Kachelwert 0x00 keinen Namen (der pokeheartgold-Satz kennt ihn nicht).

Angelegte Testkopien unter `Roms/_testkopien`: `pt-3c-gate`, `pt-3c-gate-raum`, `pt-3c-gate-szenario`, `pt-3c-gate-f` (unbenutzt), `pt-3a-3c`, `hgus-3a-3c`, `pt-3b1-3c`, `pt-3b1-szen-3c`, `pt-3b1-rep-3c`, `hg-3b1-3c`, `pt-3b2-3c`, `hg-3b2-3c`, `hg-3c-gate` (unbenutzt). Bilder von `g4emu`: `target/agent/gate-2026-10-24/` im Worktree.
