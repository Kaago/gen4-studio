# Phase 2: Gate prüfen

Stand: noch nicht durchgeführt.

Phase 2 (2a: Fundament und Orientierung, 2b: Welt-Editoren) ist implementiert und automatisch getestet. Das Gate braucht das GUI und melonDS und wird von Hand durchgeführt. Die Ergebnisse stehen unten im Abschnitt "Ergebnis". Die Originale der Test-ROMs bleiben unberührt: alles läuft auf Kopien.

Das Gate besteht aus drei Teilen:

- (A) Gate von Phase 2a, mit der gebündelten App (nicht `tauri dev`).
- (B) Gate von Phase 2b, die neue Route aus bestehenden Karten.
- (C) Zusätzliche Einzelprüfungen, die in den Reviews der Tasks 2a und 2b gesammelt wurden und sich nur im GUI zeigen.

## Vorbereitung

1. Kopien der Projekte anlegen (Platin DE, HeartGold US), zum Beispiel:
   `cp -R "../Roms/Pokemon - Platin-Edition (Germany)_DSPRE_contents" /tmp/pt-test`
   `cp -R "../Roms/Pokemon - HeartGold Version (USA)_DSPRE_contents" /tmp/hg-test`
2. Gebündelte App bauen: `cd apps/studio && pnpm bundle && bash scripts/check-bundle.sh` (Ausgabe `bundle ok`, siehe `docs/release.md`). Gestartet wird `target/release/bundle/macos/Gen4 Studio.app`, `G4_APICULA` darf nicht gesetzt sein. Beim ersten Start verlangt Gatekeeper eine Freigabe (siehe `docs/release.md`).
3. Für (C), wo es ausdrücklich dasteht, geht auch `pnpm tauri dev` mit `G4_APICULA` (siehe `docs/dev-setup.md`).
4. melonDS (die .app) bereithalten.

## (A) Gate von Phase 2a

Auf einer Kopie von Platin DE und HeartGold US, mit der gebündelten App (nicht `tauri dev`):

- [ ] 1. Header "Zweiblattdorf" (0415, Spielerzimmer) wählen: texturierte Karte, blauer Warp-Marker auf der Treppe, Hover zeigt "0x5E WARP_STAIRS_EAST" und "Warp → 0414".
- [ ] 2. Kachel vor der Treppe blockieren, speichern, bauen, in melonDS prüfen (Figur bleibt stehen).
- [ ] 3. In DSPRE (oder per Hex-Editor) dieselbe Karte in der Kopie ändern, im Studio erneut speichern: Stale-Leiste mit "Neu laden" und "Meine Version behalten", beide Wege funktionieren.
- [ ] 4. Kachel ändern, Cmd+Q: Rückfrage.
- [ ] 5. Header 0003 (Jubelstadt, Matrix 0) öffnen: Overlay sofort, Modelle strömen nach, Fortschritt sichtbar; zweites Öffnen aus dem Cache deutlich schneller.
- [ ] 6. `cargo test --workspace --all-features`, `pnpm test`, clippy, fmt grün; gated Tests auf allen drei Projekten grün.

## (B) Gate von Phase 2b

Hinweis: Dieser Teil läuft künftig über die Assistenten von Phase 3a, siehe [docs/phase-3a-gate.md](phase-3a-gate.md), Teil (A). Die Schritte unten beschreiben den Weg von Hand über den Inspector.

Auf einer Kopie von Platin DE:

- [ ] 1. Zwei Karten duplizieren (zum Beispiel die beiden Karten von Route 201; Header 0342 "Route 201" liegt auf Matrix 0; seine Zellen tragen im Raster die Header-ID 0342), neue Matrix 2 × 1 mit den Kopien anlegen.
- [ ] 2. Header 0001 ("NOTHING") auf die neue Matrix, die Area-Data von Route 201 und ein freies Event-File umstellen. Die Namen bleiben (keine neuen Header, die Header-Tabelle liegt fest im ARM9).
- [ ] 3. In Zweiblattdorf (Header 0411) einen Warp hinzufügen, Ziel Header 0001, Warp 0. In der neuen Route einen Warp zurück nach 0411.
- [ ] 4. Speichern, bauen, in melonDS: durch den Warp in die neue Route laufen, beide Karten sind begehbar, zurückwarpen.
- [ ] 5. Undo-Kette: fünf Schritte zurück und wieder vor, Szene und Inspector folgen.
- [ ] 6. Automatik: `cargo test --workspace --all-features`, `pnpm test`, clippy (`cargo clippy --workspace --all-targets --all-features -- -D warnings`), `cargo fmt --check`, gated Tests auf drei Projekten (`G4_TEST_PROJECT` je Projekt, dazu `G4_APICULA` und `G4_DSROM_CLI`, siehe `docs/dev-setup.md`).

## (C) Zusätzliche Einzelprüfungen

Gesammelt aus den Task-Reviews. Die Unterpunkte sind Dinge, die kein automatischer Test absichert.

### Laden und Texturen

- [ ] Texturierte Karten werden in der App wirklich angezeigt, auch eine Platin-Karte (Texturen, nicht nur Geometrie); der CSP-`blob:`-Fix ist nur per Test abgesichert.
- [ ] Kalter gegen warmen Cache: Öffnen derselben Karte, Zeiten notieren.
- [ ] Platin Header 0003 (Matrix 0): Overlay in unter 5 s, Modelle strömen mit sichtbarem Fortschritt nach, kalt unter 3 min, warm unter 15 s.
- [ ] Schnelles Umschalten zwischen Headern während des Ladens bleibt stabil (keine Reste des vorigen Headers in der Szene).
- [ ] Undo während des Nachladens der Modelle bleibt stabil.

### Stale und Beenden

- [ ] Eine Kartendatei von außen ändern, malen, Speichern: Stale-Leiste mit beiden Knöpfen ("Neu laden", "Meine Version behalten"), beide Wege funktionieren.
- [ ] Cmd+Q mit ungespeicherten Änderungen fragt nach. "Abbrechen" lässt die App offen, "Ja" beendet ohne zweite Rückfrage.
- [ ] Cmd+Q ohne Änderungen oder ohne Projekt beendet sofort.
- [ ] Fenster schließen mit ungespeicherten Änderungen: Rückfrage, Bestätigen beendet die ganze App.
- [ ] Beenden über Dock-Menü und Menüleiste verhält sich wie Cmd+Q.

### Workbench

- [ ] Die Szene bleibt bei Änderungen im Inspector erhalten (kein Neuaufbau, Kamera bleibt).
- [ ] Panels verschieben; nach einem Neustart der App ist das Layout wiederhergestellt.
- [ ] Statusleiste zeigt Stale-Aktionen; eine lange Fehlermeldung samt Stale-Knöpfen passt in die Leiste.
- [ ] Der Quit-Schutz (Cmd+Q, Fenster schließen) funktioniert weiterhin, nachdem die Workbench umgebaut wurde.
- [ ] Header-Liste ohne Zeilenumbrüche; Suche nach "Zweiblatt" findet 0411 bis 0417.
- [ ] Querverlinkung: Zelle springt zum Header, Header zu Area-Data und Matrix, Warp zum Ziel-Header. Die Header-Liste hebt den Eintrag hervor und scrollt ihn ins Bild; das Fokussieren der Panels folgt.

### Hover und Marker

- [ ] Treppe in Platin Header 0415 überfahren: "0x5E WARP_STAIRS_EAST"; Overlay-Farben stimmig.
- [ ] Malen und Orbit funktionieren weiterhin; beim Überfahren wird die Szene nicht neu aufgebaut.
- [ ] Spielerzimmer (Platin 0415): blauer Marker auf der Treppe, Hover zeigt Warp → 0414.
- [ ] Hover über einem Warp zeigt den Ortsnamen des Ziels.

### Header

- [ ] Platin 0415: Matrix auf 126 umstellen, die Szene zeigt das Zimmer des Rivalen; Cmd+Z bringt die vorige Matrix zurück.
- [ ] Hover gegen angeheftet: der gewählte Header bleibt im Inspector angeheftet; ein Hover über der Szene überschreibt ihn nur, solange der Zeiger auf einer Kachel steht, danach erscheint der Header wieder.
- [ ] Undo-Beschriftungen nennen die geänderten Felder lesbar.
- [ ] In ein Header-Feld tippen, dann den Zeiger über die Szene bewegen: der Wert ist geschrieben (nicht verloren).
- [ ] Der Hinweis, dass sich neue Header nicht anlegen lassen, steht im Header-Editor.

### Matrix

- [ ] Eine Matrix 2 × 1 aus zwei duplizierten Karten anlegen; Header 0001 darauf umstellen; die Szene zeigt beide Karten.
- [ ] Speichern und Bauen enthalten die neuen Member (Karten, Matrix).
- [ ] Verkleinern einer Matrix mit gefüllten Zellen fragt nach.
- [ ] Der Hinweis im Matrix-Panel, dass Höhen in der Szene nicht ausgewertet werden, steht da.

### Area-Data

- [ ] Tileset im Area-Editor wechseln: die Gebäudemodelle bekommen die neuen Texturen, die Karten bleiben unverändert; Cmd+Z macht es rückgängig.
- [ ] `area_data_id` im Header ändern: die Szene wird neu texturiert.
- [ ] Speichern und Bauen enthalten die geänderte Area-Data.

### Gebäude

- [ ] Auswählen, ziehen, rückgängig machen.
- [ ] Hinzufügen und Löschen; Entf (Backspace auf dem Mac) fragt vor dem Löschen nach und löscht nicht, solange in ein Feld getippt wird oder eine Modifier-Taste (Cmd, Ctrl, Alt) gedrückt ist.
- [ ] Unbekanntes Modell: es erscheint eine Box statt Modell, die Szene bricht nicht ab.
- [ ] HGSS: Modelle von Innenräumen (Interior-Archiv) erscheinen richtig.
- [ ] Gebäude-Höhe (y) gegenüber dem Gelände, zum Beispiel Platin Header 0411.

### Events

- [ ] Anklicken, ziehen, bearbeiten, hinzufügen, löschen (Objekte, Warps, Trigger, Hintergrund-Events).
- [ ] Entf (Backspace auf dem Mac) fragt auch bei Events vor dem Löschen nach und löscht nicht, solange in ein Feld getippt wird oder eine Modifier-Taste gedrückt ist. Beim Löschen eines Warps warnt die Rückfrage, dass Warp-Ziele anderer Events auf spätere Warps dieser Datei nicht angepasst werden.
- [ ] Cmd+Z im Eingabefeld macht den Text des Feldes rückgängig, nicht die Änderung des Assets.
- [ ] Warp-Ziel wählen über den Picker; "Zum Ziel" springt zum Zielheader.
- [ ] Gesperrte Event-Datei (Lock) ist schreibgeschützt und ändert sich nicht.
- [ ] Escape bricht ein laufendes Ziehen ab, bei Events und bei Gebäuden; die Position springt zurück.
- [ ] Koordinaten-Trigger zeigen ihre Flächen (Area-Planes) in der Szene.
- [ ] HGSS: Warp-Höhe stimmt mit dem Gelände überein.

### Undo

- [ ] Undo nach einer Matrix-Änderung am Header: die Szene springt zurück (Cmd+Z und Toolbar); Redo stellt sie wieder her.
- [ ] Undo und Redo je Asset: Malen, Gebäude, Events, Tileset der Area-Data.
- [ ] Cmd+Z gedrückt halten nach dem Malen auf zwei Nachbarkarten: beide Overlays gehen zurück.
- [ ] Undo während des ersten Kartenladens bleibt stabil.

### Bundle

- [ ] Die `.app` per Finder starten (Gatekeeper-Freigabe), Projektkopie öffnen, einen Header mit Modellen laden, ohne dass `G4_APICULA` gesetzt ist.

## Ergebnis

Hier wird nach dem Durchlauf je Teil eingetragen: Datum, Projekt (Platin DE, HeartGold US), App (gebündelt oder `tauri dev`), bestanden oder nicht, Auffälligkeiten.

| Teil | Datum | Projekt | Ergebnis | Auffälligkeiten |
|---|---|---|---|---|
| (A) Gate 2a | | | | |
| (B) Gate 2b | | | | |
| (C) Einzelprüfungen | | | | |

Gemessene Ladezeiten (Platin Header 0003, Matrix 0):

| | kalt | warm |
|---|---|---|
| Overlay sichtbar | | |
| alle Modelle geladen | | |

Offene Punkte aus dem Durchlauf:

-
