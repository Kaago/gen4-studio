# Phase 3a: Gate prüfen

Stand: noch nicht durchgeführt.

Phase 3a (Welt-Explorer, Assistenten, Weltkarte) ist implementiert und automatisch getestet. Das Gate braucht das GUI und melonDS und wird von Hand durchgeführt. Die Ergebnisse stehen unten im Abschnitt "Ergebnis". Die Originale der Test-ROMs bleiben unberührt: alles läuft auf Kopien. Spezifikation: [docs/superpowers/specs/2026-10-22-world-explorer-design.md](superpowers/specs/2026-10-22-world-explorer-design.md).

Das Gate besteht aus sechs Teilen:

- (A) Assistenten-Lauf: die neue Route aus Phase 2 (B) mit den Assistenten, plus ein neuer Raum mit Eingang. Das ist das eigentliche Gate.
- (B) Explorer: Baum, Suche, Liste, Begriffe.
- (C) Raster: Kartenbilder und Erweitern.
- (D) Weltkarte.
- (E) Automatik.
- (F) Zusätzliche Einzelprüfungen, die in den Reviews der Tasks gesammelt wurden und sich nur im GUI zeigen.

Der Gate-Teil (B) von Phase 2 ([docs/phase-2-gate.md](phase-2-gate.md)) läuft künftig über die Assistenten, also über (A) dieser Datei. Die alten Handschritte (Karten duplizieren, Matrix anlegen, Header umstellen) bleiben im Inspector möglich, sind aber nicht mehr der Gate-Weg.

Namen in der Oberfläche: linkes Panel "Welt" (Umschalter "Baum | Liste", Knopf "+ Neu" mit "Gebiet anlegen…" und "Raum anlegen…", Popover "Begriffe"); Tabs in der Mitte "3D", "Raster", "Weltkarte"; im Inspector zeigt der Header-Editor den Abschnitt "Eingänge / Ausgänge" mit "Zum Ziel", "Bearbeiten", "Raum hier anbinden…" und "Warp hinzufügen". Der Assistent "Gebiet anlegen…" hat die Schritte Quelle, Anordnung, Header, Zusammenfassung; "Raum anlegen…" hat Vorlage, Header, Eingang, Zusammenfassung. Der Ergebnis-Schritt bietet "Im Studio zeigen".

## Vorbereitung

1. Kopien der Projekte anlegen (Platin DE, HeartGold US), zum Beispiel:
   `cp -R "../Roms/Pokemon - Platin-Edition (Germany)_DSPRE_contents" ../Roms/_testkopien/pt-3a`
   `cp -R "../Roms/Pokemon - HeartGold Version (USA)_DSPRE_contents" ../Roms/_testkopien/hg-3a`
   Für jeden Durchlauf eine frische Kopie nehmen: das Anlegen neuer Karten, Raster und Eventdateien lässt sich nicht rückgängig machen und liegt sofort auf Platte.
2. Gebündelte App bauen: `cd apps/studio && pnpm bundle && bash scripts/check-bundle.sh` (Ausgabe `bundle ok`, siehe `docs/release.md`). Gestartet wird `target/release/bundle/macos/Gen4 Studio.app`, `G4_APICULA` darf nicht gesetzt sein. Beim ersten Start verlangt Gatekeeper eine Freigabe (siehe `docs/release.md`).
3. Für Prüfungen, wo es ausdrücklich dasteht, geht auch `pnpm tauri dev` mit `G4_APICULA` (siehe `docs/dev-setup.md`).
4. melonDS (die .app) bereithalten und eine Stoppuhr für (A).

Bekannte Fakten zu Platin DE, die der Ablauf braucht: Header 0001 ist "NOTHING" und frei für Tests; Route 201 ist Header 0342 (R201) und liegt auf Raster 0000 (30 × 30), Header 0008 ist dagegen "Pokétch Ges." (1 × 1); Header 0002 ist "UG" (Raster 0002, 15 × 15) und damit nicht wirklich frei; Zweiblattdorf hat den Außen-Header 0411 und sechs Räume 0412 bis 0417 (Rivalenhaus T01R0101 und T01R0102, Spielerhaus T01R0201 und das Spielerzimmer T01R0202 = 0415 mit Karte 187, dazu 0416 und 0417). Bei HeartGold US ist New Bark Town Header 0060 (T20).

## (A) Assistenten-Lauf auf Platin DE

Ziel: unter fünf Minuten, ohne eine Zahl zu tippen außer der Header-Wahl. Die Stoppuhr läuft von "+ Neu" bis zum Klick auf "Im Studio zeigen" im Ergebnis-Schritt des Raum-Assistenten (dem letzten Assistenten); bauen und melonDS zählen nicht mit. Jede getippte Zahl (Kachelkoordinaten, ein von Hand gesetzter Warp) kommt in die Spalte "Zahlen getippt" der Zeitentabelle unten.

Auf einer Kopie von Platin DE, mit der gebündelten App:

- [ ] 1. Stoppuhr starten. "+ Neu" → "Gebiet anlegen…". Schritt Quelle: im Feld Raster "Raster 0000" wählen (Route 201 liegt auf Raster 0000, Header 0342), im Gitter zwei nebeneinanderliegende Zellen anklicken, deren Header-ID oben rechts 0342 zeigt (oder Umschalt-Klick für das Rechteck).
- [ ] 2. Schritt Anordnung: Name "Route 230" eintragen. Es gibt keine Felder für Breite und Höhe, die Größe folgt aus der Auswahl; die Vorschau zeigt 2 × 1. "ohne Gebäude" bleibt aus, "Header-Abschnitt anlegen" bleibt an (Vorgabe).
- [ ] 3. Schritt Header: Header 0001 ("NOTHING") wählen; die Zeile zeigt "Warps hierher: 0, wirkt unbenutzt" (Text je Zeile). Area-Data ist mit der des ersten Quell-Headers (0342) vorbelegt. Eventdatei ausdrücklich "neu, leer" wählen; die Vorgabe ist "behalten", dann behielte Header 0001 seine alte Eventdatei. Das ist die einzige Auswahl, die nötig ist; getippt wird keine Zahl.
- [ ] 4. Schritt Zusammenfassung: der Text nennt das Raster "Route 230" 2 × 1, die zwei kopierten Karten und die Umstellung von Header 0001. "Anlegen" drücken. Der Ergebnis-Schritt listet Raster, Karten und Eventdatei; "Im Studio zeigen" wählt Header 0001, die Szene zeigt beide Karten, der Explorer klappt den Ort auf.
- [ ] 5. Header 0411 (Zweiblattdorf) im Explorer öffnen und in der 3D-Ansicht die Maus auf die Kachel vor dem Rivalenhaus (Header 0412 und 0413, T01R0101 und T01R0102) stellen, damit der Assistent diese Kachel später aus der Szene übernimmt. Werden die Kachelnummern stattdessen von Hand getippt, zählen sie in "Zahlen getippt".
- [ ] 6. "+ Neu" → "Raum anlegen…". Schritt Vorlage: das Spielerzimmer (Header 0415, Karte 187) wählen, Karte mit Bild sichtbar.
- [ ] 7. Schritt Header: Header 0002 wählen. Er heißt "UG" und liegt auf Raster 0002; "wirkt unbenutzt" ist nur eine Heuristik (nichts führt per Warp hierher) und auf einer Kopie unbedenklich. Wer das vermeiden will, wählt einen Header, den die Liste mit "Warps hierher: 0" zeigt. Area-Data ist mit der der Vorlage vorbelegt. Eventdatei ausdrücklich "Kopie von …" wählen; die Vorgabe ist "neu, leer", das Wählen der Vorlage schaltet nicht um.
- [ ] 8. Schritt Eingang: Außen-Header Zweiblattdorf (0411) wählen; die Außen-Kachel ist aus der Szene übernommen (Hinweis "Kachel zuletzt in der Szene gewählt"); die Tür innen ist mit dem ersten Warp der Vorlage vorbelegt.
- [ ] 9. Schritt Zusammenfassung prüfen, "Anlegen", Ergebnis-Schritt, "Im Studio zeigen". Stoppuhr anhalten. Zeit und die Zahl der getippten Zahlen notieren (Ziel: unter fünf Minuten, keine Zahl außer der Header-Wahl).
- [ ] 10. Inspector bei Header 0411: der Abschnitt "Eingänge / Ausgänge" zeigt den neuen Warp mit Ziel "0002", "Zum Ziel" springt dorthin; im Raum 0002 zeigt der Abschnitt den Rück-Warp nach 0411.
- [ ] 11. Speichern, bauen, melonDS: in Zweiblattdorf auf die Kachel laufen, im neuen Raum ankommen, hinauslaufen, wieder in Zweiblattdorf.
- [ ] 12. Die neue Route 230 ist nur über Header 0001 erreichbar. Damit sie sich im Spiel begehen lässt, nach dem Lauf einen Warp von Hand setzen: im Header-Editor von Zweiblattdorf (0411) "Warp hinzufügen", Ziel Header 0001, Warp 0 (die getippten Zahlen kommen in "Zahlen getippt", gehören aber nicht zur gemessenen Zeit). Speichern, bauen, melonDS: durch den Warp in Route 230, beide Karten sind begehbar.

## (B) Explorer

Auf derselben Kopie von Platin DE:

- [ ] 1. Suche "Zweiblatt" im Panel "Welt" zeigt den Ort Zweiblattdorf mit Außen (0411) und seinen sechs Räumen (0412 bis 0417) (jede Zeile: Name, Header-ID und Kürzel in Grau, bei Räumen die Kartenzahl).
- [ ] 2. Umschalter "Baum | Liste": die Liste zeigt wie in Phase 2 alle Header, Suche nach "Zweiblatt" findet 0411 bis 0417; zurück auf "Baum" bleibt die Auswahl erhalten.
- [ ] 3. Popover "Begriffe": das Glossar nennt Ort, Header, Außen, Raum, Raster, Karte, Eingang / Ausgang, Eventdatei und Area-Data mit Erklärung.
- [ ] 4. Header ohne Ortsnamen stehen unter "Ohne Ortsnamen". Ein im Inspector, in der Weltkarte oder per Warp gewählter Header klappt seinen Ort im Baum auf und scrollt ins Bild.

## (C) Raster

- [ ] 1. Tab "Raster" bei Zweiblattdorf (Header 0411): Kartenbilder in jeder Zelle, unten die Karten-ID, oben rechts die Header-ID der Zelle; Doppelklick auf eine Zelle öffnet ihren Header.
- [ ] 2. Header 0003 (Jubelstadt, Matrix 0, 30 × 30): die Kartenbilder erscheinen nach und nach, ohne dass die Oberfläche hängt; Zellen ohne Bild zeigen eine Fläche in Ortsfarbe mit der Karten-ID.
- [ ] 3. Route 230 (nach (A)): die gestrichelte "+"-Zelle rechts oder unten öffnet den Dialog "Karte bei x …, y … anlegen"; "Kopie von Karte … (Nachbarzelle)" wählen, "Anlegen": das Raster wächst um eine Zelle, die Szene zeigt die neue Karte.
- [ ] 4. Das Anlegen im Dialog warnt, dass es sich nicht rückgängig machen lässt,.

## (D) Weltkarte

- [ ] 1. Tab "Weltkarte" auf Platin DE: die Region (Matrix 0) mit Ortsfarben und Ortsnamen an der Mitte ihrer Zellen, Rad zoomt, Ziehen verschiebt, Klick auf eine Zelle springt zum Header (Szene und Explorer folgen).
- [ ] 2. Zweiblattdorf gewählt: von den Raum-Kärtchen zweier Häuser (Rival und Spieler) laufen Linien zu den Zellen ihrer Türen; ein Raum ohne verbundenen Eingang zeigt "ohne Eingang" und keine Linie.
- [ ] 3. Der neue Raum aus (A) erscheint, nachdem sein Eingang gesetzt ist, mit einer Linie zu Zweiblattdorf.
- [ ] 4. HeartGold US (gebündelte App, frische Kopie): Baum und Weltkarte laden; New Bark Town ist Header 0060 (T20); die Weltkarte ordnet über das Raster an, nicht über `worldmap_x/y`.

## (E) Automatik

- [ ] 1. `cargo test --workspace --all-features`, `pnpm test`, `pnpm build`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo fmt --check` grün.
- [ ] 2. Gated Tests auf drei Projekten grün: `G4_TEST_PROJECT` nacheinander auf `../Roms/*_DSPRE_contents` (Platin DE, HeartGold DE, HeartGold US), dazu `G4_APICULA="$PWD/tools/apicula/bin/apicula"` und `G4_DSROM_CLI="$PWD/../Roms/_tools/dsrom"` (siehe `docs/dev-setup.md`, Abschnitt "Gated Tests"). `vanilla_copy()` arbeitet auf einer Kopie, die Originale werden nie beschrieben.

## (F) Zusätzliche Einzelprüfungen

Gesammelt aus den Task-Reviews. Die Punkte sind Dinge, die kein automatischer Test absichert.

### Kartenbilder

- [ ] Ausrichtung und Ausschnitt der Kartenbilder stimmen mit der 3D-Ansicht von oben überein: Norden oben, nicht gespiegelt (Zweiblattdorf und eine Karte mit Gebäuden vergleichen).
- [ ] Ohne apicula (`G4_APICULA` auf einen falschen Pfad, `tauri dev`): Zellen zeigen dauerhaft den Platzhalter mit Hinweis im Tooltip, Baum und Raster bleiben nutzbar, die Statusleiste meldet es einmal.

### Layout im schmalen Panel

- [ ] Die Listenansicht ist in einem 300 px breiten Panel "Welt" nicht abgeschnitten (kein horizontales Springen, Zeilen lesbar).
- [ ] Die Popover "Begriffe" und "+ Neu" werden im 300 px breiten Panel nicht abgeschnitten und schließen mit Escape und Klick daneben.

### Tastatur während eines Assistenten

- [ ] Solange ein Assistent oder der Dialog zum Erweitern offen ist, tun Escape (schließt nur den Assistenten, bricht kein Ziehen in der Szene ab), Entf (Backspace auf dem Mac), Cmd+Z und Cmd+S nichts an der Szene dahinter: nichts wird gelöscht, nichts rückgängig gemacht, nichts gespeichert. Nach dem Schließen funktionieren sie wieder.

### Raum-Assistent

- [ ] Ein Raum-Assistent, der über "+ Neu" geöffnet wird und dessen Außen-Header ein anderes Raster hat als das in der Szene gezeigte, fragt im Schritt Eingang nach der Kachel der Tür (keine Vorbelegung aus der falschen Matrix).
- [ ] Ein Raum-Assistent über "Raum hier anbinden…" im Inspector hat den Header als Außen-Header vorgewählt.
- [ ] Gesperrte Eventdatei (Lock) des Außen-Headers: der Schritt Eingang nennt die Sperre und bietet "später verbinden".
- [ ] Außen-Kachel außerhalb des Rasters (zum Beispiel x 999) wird abgelehnt, bevor etwas angelegt wird.

### Fehlerfälle

- [ ] Ein Assistent, der spät scheitert (zum Beispiel Header-Umstellung mit gesperrter Eventdatei), meldet im Ergebnis-Schritt den Schritt und die schon angelegten Member in Klartext.
- [ ] Die Header-, Warp- und Zellenänderungen der Assistenten lassen sich einzeln mit Cmd+Z zurücknehmen; die angelegten Karten, Raster und Eventdateien bleiben liegen (wie angesagt).
- [ ] Layout: die App startet mit dem neuen Layout (Welt, 3D, Raster, Weltkarte, Inspector, Status); ein Layout aus Phase 2 wird verworfen und stört nicht. Verschobene Panels sind nach einem Neustart wiederhergestellt.

## Ergebnis

Hier wird nach dem Durchlauf je Teil eingetragen: Datum, Projekt (Platin DE, HeartGold US), App (gebündelt oder `tauri dev`), bestanden oder nicht, Auffälligkeiten.

| Teil | Datum | Projekt | Ergebnis | Auffälligkeiten |
|---|---|---|---|---|
| (A) Assistenten-Lauf | | | | |
| (B) Explorer | | | | |
| (C) Raster | | | | |
| (D) Weltkarte | | | | |
| (E) Automatik | | | | |
| (F) Einzelprüfungen | | | | |

Gemessene Zeit für (A) (Ziel: unter fünf Minuten, keine Zahl außer der Header-Wahl getippt):

| Lauf | Zeit | Zahlen getippt |
|---|---|---|
| 1 | | |

Offene Punkte aus dem Durchlauf:

-
