# Phase 3c: Gate prüfen

Stand: 2026-10-24. 3c.1 und 3c.2 sind implementiert und automatisch getestet; die Abnahme steht aus und läuft in einer eigenen Session (Teile (A) bis (G) über die Test-Umgebung, (H) und die Punkte mit rotom-lsp beim Nutzer). Die Ergebnistabelle unten ist noch leer.

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
- [ ] 3. Neu öffnen und durchlaufen: Vorlage Header 0415, Schritt Header „Neuer Header“ (vorher „Patch anwenden“ im Kasten „Patch „Dynamische Header“ nötig“; oder „Vorhandenen Header umstellen“), Eventdatei „Kopie von 0394“, Eingang Außen-Header 0411 mit der Kachel aus Schritt 1 und „Tür-Kacheln“ an. Weiter mit Enter oder „Weiter“, zurück mit ⌘[. Die Zusammenfassung zeigt den Satz und die Plakette „Nicht rückgängig“.
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
| (A) Welt bauen: Figur bearbeiten | Agent (Vergleich mit dem Screen: Nutzer) | | | | |
| (B) Texte | Agent (Vergleich mit dem Screen: Nutzer) | | | | |
| (C) Scripts ohne rotom-lsp | Agent | | | | |
| (C) Scripts mit rotom-lsp | Nutzer | | | | |
| (D) Assistent „Raum anlegen“ | Agent | | | | |
| (E) Dunkel und Kompakt, Screenshots | Agent | | | | |
| (E) Dunkel und Kompakt, Blick | Nutzer | | | | |
| (F) Tastatur | Agent | | | | |
| (G) 3a (A) bis (D) | Agent (Stoppuhr: Nutzer) | | | | |
| (G) 3b.1 (A) und (D) | Agent (DSPRE: Nutzer) | | | | |
| (G) 3b.2 (A) bis (F) | Agent (Completion, Hover, Diagnosen: Nutzer) | | | | |
| (G) Palette, Layout, Thema und Dichte, Technik › Patches | Agent | | | | |
| (G) `scripts/gate.sh` | Agent | | | | |
| (H) Gepackte App | Nutzer | | | | |

Ergebnis von `scripts/gate.sh` (Tabelle aus der Ausgabe):

```
(noch nicht gelaufen)
```

## Bekannt vor der Abnahme

Befunde aus der Umsetzung von 3c.2, die nicht als Fehler zählen oder bewusst offen sind:

- Die Zeichengrenze der Texte (38 je Zeile, Balken und „34 von 38“) zählt Zeichen; das Backend warnt nach Breite (200 px). Die Tabelle zeigt deshalb mehr gelbe Balken als es Warnungen im Panel Probleme gibt; die Plaketten der Archivliste zählen nur die Warnungen des Backends.
- Die Nachrichtenzahl steht nur am geladenen Archiv, nicht an jeder Zeile der Liste.
- Technik › Werkzeuge: apicula „Bereit“ folgt aus dem ersten Kartenbild (das auch aus dem Modell-Cache kommen kann), „Fehlt“ bleibt nach einem Fehler für die Sitzung; rotom-lsp zeigt den Zustand des Script-Panels, „Nicht gestartet.“, solange das Panel nicht gezeigt wurde. Pfade der Werkzeuge zeigt Technik nicht (kein Backend-Befehl liefert sie).
- Solange ein Assistent offen ist: Verweise verlassen die Welt nicht, das Prüflicht tut nichts, ein anderes Projekt öffnen schließt den Assistenten; ⌘S und ⌘R sind gesperrt, die Tasten „Speichern“ und „Bauen“ der Titelleiste bleiben klickbar.
- Einstellungen › Tastenkürzel nennt nur die ⌘-Kürzel, nicht die Buchstaben der Werkzeuge.
- Im Raster zeigt auch Bauen die Kennungen der Zellen; Tüfteln zeigt zusätzlich Header-ID (dezimal) und Höhe, die Rohdaten der Zeile in der Hand sind u16 hex.
- Aus dem Durchgang Dunkel und Kompakt offen: der Kopf des Script-Panels bricht in Kompakt um; der Dockview-Reiter „Inspector“ wird in einer schmalen Spalte abgeschnitten; die Brotkrume der Texte wird in einem schmalen Scharnier abgeschnitten.
