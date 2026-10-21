# Phase 1: GUI-Gate prüfen

Stand 2026-10-21: Gate bestanden. Malen, Speichern, Bauen und Starten funktionieren. Die Kollision wirkt im Spiel (Platin DE, Spielerzimmer, Header 0415 T01R0202, Karte 187). Offen sind nur die Rückfrage beim Schließen und Cmd+Q.

Automatisch geprüft sind Codecs, Store, Build und der Ablauf Malen → Speichern → Bauen auf Kopien der drei Test-ROMs (Platin DE, HeartGold DE, HeartGold US). Offen ist nur der Teil mit Fenster und Emulator. Er wird von Hand geprüft.

## Vorbereitung

1. Eine Kopie des Projekts anlegen. Die Originale bleiben unberührt:
   `cp -R "../Roms/Pokemon - HeartGold Version (USA)_DSPRE_contents" /tmp/hg-test`
2. apicula bauen, falls noch nicht geschehen (siehe `docs/dev-setup.md`).
3. App starten: `cd apps/studio && G4_APICULA="$PWD/../../tools/apicula/bin/apicula" pnpm tauri dev`

## Ablauf

1. "Projekt öffnen" und die Kopie wählen.
2. Den Header von New Bark Town wählen. Die Matrix lädt, "Permissions anzeigen" ist an.
3. Vor der Haustür auf eine Kachel klicken. Sie wird rot (Kollision). Shift+Klick entfernt die Kollision. Ziehen mit der Maus dreht die Kamera, ohne zu malen.
4. Oben steht "● ungespeichert (1)". Cmd+Z macht rückgängig, Shift+Cmd+Z stellt wieder her.
5. Cmd+S zeigt unten "1 Datei(en) gespeichert".
6. "Bauen" mit Zielpfad `out.nds`. Unten erscheint "ROM gebaut: …".
7. "Starten": beim ersten Mal melonDS (die .app) wählen. Im Spiel prüfen, dass die Kachel blockiert ist.
8. Dasselbe mit einer Kopie des Platin-Projekts.
9. Eine Kachel ändern und das Fenster über den Schließen-Knopf schließen. Es erscheint eine Rückfrage.
10. Dasselbe mit Cmd+Q. Erscheint hier keine Rückfrage, ist das ein bekannter offener Punkt für Phase 2.
11. Fehlerfall: falscher Emulator-Pfad oder eine von außen geänderte Datei zeigt eine rote Leiste unten, kein Popup.

## Worauf achten

- Sitzen die Gebäude-Boxen auf den Häusern? Wenn nicht, ist die Gebäude-Koordinatenumrechnung falsch.
- Liegt das Permissions-Raster passend auf dem Gelände?
- Die Modelle erscheinen in Phase 1 ohne Texturen.
