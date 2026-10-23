# Phase 3b.1: Gate prüfen

Stand: 2026-10-23 vom Agenten durchlaufen (Browser-Modus mit `studio-bridge`, Emulator `g4emu`, main 3b86440); offen sind die Punkte mit „Hand“ und „Agent + Blick“, siehe unten.

Phase 3b.1 (Fundament: neue Header per Patch, Höhen, Objekt-Sprites, HGSS-Kachelnamen, apicula-Zeitlimit, Gate-Skript) ist implementiert und automatisch getestet. Das Gate braucht das GUI, melonDS und DSPRE und wird von Hand durchgeführt; die Automatik läuft über `scripts/gate.sh`. Die Ergebnisse stehen unten im Abschnitt "Ergebnis". Die Originale der Test-ROMs bleiben unberührt: alles läuft auf Kopien. Spezifikation: [docs/superpowers/specs/2026-10-22-phase-3b1-foundation-design.md](superpowers/specs/2026-10-22-phase-3b1-foundation-design.md).

Das Gate besteht aus sechs Teilen:

- (A) Neuer Header per Assistent, inklusive Patch, melonDS-Gang und DSPRE-Gegenprobe. Das ist das eigentliche Gate.
- (B) Höhen in der 3D-Szene.
- (C) Objekt-Sprites.
- (D) Kachelnamen auf HeartGold.
- (E) Automatik: `scripts/gate.sh`.
- (F) Zeitlimit von apicula (optional).

Namen in der Oberfläche: Der Assistent "Gebiet anlegen…" (Knopf "+ Neu" im Panel "Welt") hat die Schritte Quelle, Anordnung, Header, Zusammenfassung; "Raum anlegen…" hat Vorlage, Header, Eingang, Zusammenfassung. Im Schritt "Header" wählt man "Vorhandenen Header umstellen" oder "Neuer Header". Ist der Patch noch nicht gesetzt, ist "Neuer Header" gesperrt und ein Kasten "Patch „Dynamische Header“ nötig" bietet "Patch anwenden"; danach erscheinen die Zeile "Kopie von Header MMMM …" und das Feld "Interner Name". Auf Diamant und Perl steht dort "Neue Header gibt es nur für Platin und HeartGold/SoulSilver". Die Höhen einer Zelle stehen im Inspector unter "Höhe" (Zelle im Tab "Raster" anklicken); der Schalter "Höhen" in der Leiste des Rasters schaltet den Höhenabschnitt der Matrix ein. Objekte haben im Inspector das Feld "Richtung (0 oben, 1 unten, 2 links, 3 rechts)".

## Vorbereitung

Ein Claude-Agent kann den größten Teil dieses Gates selbst abarbeiten (Browser-Fenster, Emulator, Prüfungen auf der Platte); welche Punkte er übernimmt und welche beim Nutzer bleiben, steht in [docs/agent-gate.md](agent-gate.md). Die Spalte "Wer" in der Ergebnistabelle hält fest, wer den Teil geprüft hat (Agent, Hand oder Agent + Blick).

1. Kopien der Projekte anlegen (Platin DE, HeartGold DE), zum Beispiel:
   `cp -R "../Roms/Pokemon - Platin-Edition (Germany)_DSPRE_contents" ../Roms/_testkopien/pt-3b1`
   `cp -R "../Roms/Pokemon - Goldene Edition HeartGold (Germany)_DSPRE_contents" ../Roms/_testkopien/hg-3b1`
   Für jeden Durchlauf eine frische Kopie nehmen: der Patch, neue Header und neue Karten, Raster und Eventdateien lassen sich nicht rückgängig machen und liegen sofort auf Platte. Die Kopie `pt-3b1` darf vor (A) noch nicht mit DSPRE gepatcht sein.
2. Gebündelte App bauen: `cd apps/studio && pnpm bundle && bash scripts/check-bundle.sh` (Ausgabe `bundle ok`, siehe `docs/release.md`). Gestartet wird `target/release/bundle/macos/Gen4 Studio.app`, `G4_APICULA` darf nicht gesetzt sein. Beim ersten Start verlangt Gatekeeper eine Freigabe (siehe `docs/release.md`). Für (F) geht `pnpm tauri dev` mit `G4_APICULA` (siehe `docs/dev-setup.md`).
3. melonDS (die .app) und DSPRE bereithalten. DSPRE läuft nur unter Windows (oder einer Windows-Umgebung); die Kopie `pt-3b1` muss dort erreichbar sein.
4. Vor dem Patch im Projekt nichts Ungespeichertes stehen lassen: "Patch anwenden" verlangt, erst zu speichern.

Fakten zu Platin DE, die der Ablauf braucht: Route 201 ist Header 0342 (R201) auf Raster 0000 (30 × 30); Zweiblattdorf hat den Außen-Header 0411 und sechs Räume 0412 bis 0417; Jubelstadt ist Header 0003 (Raster 0, viele Personen). Die Header-Tabelle liegt vor dem Patch in `arm9/arm9.bin`, danach eine Datei je Header in `unpacked/dynamicHeaders` (Dateiname vierstellig, 24 Bytes), und `files/fielddata/maptable/mapname.bin` hat 16 Byte je Header. Die Sicherung heißt `arm9/arm9.bin.backup`.

## (A) Neuer Header per Assistent

Ziel: Ein neues Gebiet bekommt einen **neuen** Header (statt einen unbenutzten umzustellen); das ROM baut, das Gebiet ist in melonDS begehbar, und DSPRE öffnet das Projekt und kennt Patch und Header.

Auf der Kopie `pt-3b1` (Platin DE), mit der gebündelten App:

- [ ] 1. Projekt `pt-3b1` öffnen. Vor dem Patch notieren: Zahl der Header (letzte Header-ID im Explorer plus eins) und Größe von `files/fielddata/maptable/mapname.bin` (`ls -l`, 16 Byte je Header). Die Datei `arm9/arm9.bin.backup` darf noch nicht existieren (der Patch legt sie an).
- [ ] 2. Header 0342 (Route 201) wählen. "+ Neu" → "Gebiet anlegen…". Schritt Quelle: zwei nebeneinanderliegende Zellen mit Header 0342 übrig lassen, Rest abwählen. Schritt Anordnung: Name "Route 231" eintragen. Weiter zum Schritt Header.
- [ ] 3. Schritt Header: "Neuer Header" ist gesperrt, der Kasten "Patch „Dynamische Header“ nötig" erklärt, dass Bytes in `arm9.bin` geändert werden, die Header danach in einer Datei je Header liegen, sich der Patch nicht zurücknehmen lässt, `arm9.bin.backup` angelegt wird und DSPRE den Patch kennt. "Patch anwenden" drücken. Der Kasten verschwindet, "Neuer Header" ist wählbar.
- [ ] 4. Auf der Platte: `arm9/arm9.bin.backup` existiert (genau so groß wie die ursprüngliche `arm9.bin`); `unpacked/dynamicHeaders` hat eine Datei je bisherigem Header; `arm9.bin` hat sich geändert (`cmp arm9/arm9.bin arm9/arm9.bin.backup` meldet Unterschiede). Der Explorer zeigt unverändert dieselben Header wie vorher.
- [ ] 5. "Neuer Header" wählen: Die Zeile "Kopie von Header 0342 „…“" erscheint, das Feld "Interner Name" ist mit einem aus "Route 231" abgeleiteten Namen vorbelegt (Großbuchstaben, Ziffern, Unterstrich, höchstens 16 Zeichen); der Hinweis nennt die Nummer des neuen Headers (die bisherige Anzahl). Den Namen auf `R231` ändern. Die Eventdatei bleibt bei der Vorgabe "neu, leer".
- [ ] 6. Schritt Zusammenfassung: der Text enthält die Zeile "Der Assistent legt Header NNNN „R231“ neu an (Kopie von 0342) …" mit der Nummer aus Schritt 5. "Anlegen" drücken. Der Ergebnis-Schritt nennt neben Raster, Karten und Eventdatei den neuen Header. "Im Studio zeigen" wählt ihn; die Szene zeigt beide Karten, der Explorer führt den Header unter dem Ort der Vorlage (Route 201).
- [ ] 7. Auf der Platte: `unpacked/dynamicHeaders/NNNN` existiert und hat 24 Byte; `mapname.bin` ist genau 16 Byte länger als in Schritt 1, die letzten 16 Byte beginnen mit `R231`.
- [ ] 8. Variante "behalten": noch einmal "Gebiet anlegen…" (zwei andere Zellen, Name "Route 232"), im Schritt Header "Neuer Header", Eventdatei "behalten": die Zusammenfassung sagt "behalten (geteilt mit Header 0342)" bzw. nennt Header und Vorlage; Anlegen. Der neue Header zeigt im Inspector dieselbe Eventdatei wie 0342.
- [ ] 9. Einen Warp von Hand verbinden (Zahlen tippen, gehört nicht zum Gate): in Zweiblattdorf (0411) auf eine Tür-Kachel oder eine freie Kachel "Warp hinzufügen" (die Frage, ob die Kachel zur Tür werden soll, mit "Ja" beantworten), Ziel der neuen Header NNNN und Ziel-Warp 0; im neuen Header einen Warp zurück nach 0411 anlegen (wie in [docs/phase-3a-gate.md](phase-3a-gate.md) (A) 11).
- [ ] 10. Speichern, bauen (ROM entsteht ohne Fehler), melonDS: in Zweiblattdorf durch den Warp in das neue Gebiet laufen, beide Karten sind begehbar, zurückwarpen. Das ist das Spielbarkeits-Gate des neuen Headers.
- [ ] 11. Nach dem Bauen im Projekt: `unpacked/dynamicHeaders` ist unverändert gepackt worden (das ROM enthält das NARC `files/debug/cb_edit/d_test.narc`).

DSPRE-Gegenprobe (Platin DE, Kopie `pt-3b1` nach Schritt 11):

- [ ] 12. Die Kopie vor dem Öffnen in DSPRE noch einmal kopieren (`cp -R .../pt-3b1 .../pt-3b1-dspre`) und **die zweite Kopie** in DSPRE öffnen, damit DSPRE (das beim Speichern und Packen Dateien neu schreibt) das Testergebnis nicht überschreibt. DSPRE öffnet das Projekt ohne Fehlermeldung.
- [ ] 13. Im Patch-Dialog von DSPRE ("Dynamic Header Allocation") steht der Patch als angewendet, er lässt sich nicht noch einmal anwenden.
- [ ] 14. Im Header-Editor von DSPRE erscheint der neue Header NNNN mit dem Namen `R231` in der Liste; sein Inhalt (Matrix, Eventdatei, Area-Data, Musik) entspricht dem, was Gen4 Studio zeigt. Die Anzahl der Header in DSPRE ist die alte Anzahl plus die zwei neuen aus Schritt 6 und 8.
- [ ] 15. Nichts in DSPRE speichern. Optional: DSPRE baut aus der zweiten Kopie ein ROM, das in melonDS startet.

HeartGold DE (Kopie `hg-3b1`), die Kurzfassung:

- [ ] 16. Kopie `hg-3b1` öffnen, "+ Neu" → "Gebiet anlegen…" mit einem beliebigen Quell-Header, im Schritt Header "Patch anwenden", dann "Neuer Header" mit Interner Name `HG_TEST`, anlegen, speichern, bauen: `files/a/0/5/0` ist im ROM, das ROM startet in melonDS, das neue Gebiet ist über einen von Hand gesetzten Warp begehbar (wie Schritt 9 und 10).
- [ ] 17. Auf Diamant/Perl (falls ein Projekt zur Hand ist; sonst entfällt der Punkt) steht im Schritt Header "Neue Header gibt es nur für Platin und HeartGold/SoulSilver", und "Neuer Header" ist nicht wählbar.

Reparatur eines fehlerhaften Patches (Platin DE; Fehler #2 des ersten Gate-Laufs, siehe [docs/formats/dynamic_headers.md](formats/dynamic_headers.md) "Abweichung von DSPRE"):

- [ ] 18. Eine frische Kopie von Platin DE in DSPRE öffnen und dort den Patch "Dynamic Header Allocation" anwenden (DSPRE schreibt den falschen Aufruf `CC F7 74 FC` bei 0x3A0D6). Die Kopie in Gen4 Studio öffnen: die Meldungszeile sagt "Patch „Dynamische Header“ fehlerhaft …" und bietet "Patch reparieren"; im Schritt Header eines Assistenten steht derselbe Hinweis, und "Neuer Header" ist gesperrt. "Patch reparieren" drücken: Meldung "… repariert …", bei 0x3A0D6 stehen `DE F7 AD F8`, sonst ist `arm9.bin` unverändert. Bauen, melonDS: das Spiel läuft nach dem Intro weiter. Ein intaktes Projekt (`pt-3b1`) zeigt die Meldung nicht.

## (B) Höhen

Auf einer frischen Kopie von Platin DE, mit der gebündelten App:

- [ ] 1. Tab "Raster" bei Route 201 (Header 0342, Raster 0000). Hat die Matrix noch keinen Höhenabschnitt, in der Leiste "Höhen" einschalten.
- [ ] 2. Eine Zelle anklicken, im Inspector "Höhe" auf 2 setzen. In der 3D-Ansicht liegt diese Zelle eine Kachel höher als ihre Nachbarn (eine Stufe ist eine halbe Kachel). Das ganze Teil steigt mit: Kartenmodell, Gebäude, Overlay, Marker (Warps, Events, Trigger-Bereiche), Sprites und die Umrandung der Auswahl.
- [ ] 3. Hover über die erhöhte Zelle zeigt Kachelkoordinaten und Kacheltyp der Kachel unter dem Zeiger, nicht die des Bodens dahinter. Ein Klick wählt die erhöhte Kachel.
- [ ] 4. Ein Event (zum Beispiel ein Objekt) auf einer tiefen Zelle anfassen und auf die erhöhte Zelle ziehen: es landet auf der Kachel unter dem Zeiger (nicht auf der Kachel dahinter oder darunter) und liegt danach in der erhöhten Höhe. Zurückziehen auf die tiefe Zelle geht genauso.
- [ ] 5. Höhe wieder auf 0 setzen (oder Cmd+Z): die Zelle liegt wieder bündig, nichts springt.

## (C) Sprites

Auf einer frischen Kopie von Platin DE, mit der gebündelten App. Die Ausrichtung der Sprites wurde bisher nur durch Überlegung und Unit-Tests geprüft, nie auf einer Grafikkarte: dieser Teil bestätigt sie.

- [ ] 1. Zweiblattdorf (0411) oder Jubelstadt (0003): Personen und Trainer erscheinen als ihr Sprite statt als gelbes Quadrat, aufrecht stehend (die Füße auf der Kachel, der Kopf oben, nicht gekippt, nicht gespiegelt) und von der Kamera aus lesbar (Billboard: dreht man die Ansicht, steht das Sprite weiter aufrecht zur Kamera).
- [ ] 2. Jedes Sprite blickt in die Richtung seines Feldes "Richtung" (0 oben, 1 unten, 2 links, 3 rechts), gesehen von oben: oben ist Norden, links ist Westen. Mindestens ein Objekt je Richtung prüfen (Richtungen im Inspector ablesen). Schaut ein Sprite spiegelverkehrt (links/rechts vertauscht) oder nach hinten, ist das ein Fehler und kommt in die Auffälligkeiten.
- [ ] 3. Im Inspector bei einem Objekt "Richtung" ändern (0 bis 3): das Sprite in der Szene wechselt mit, ohne Neuladen.
- [ ] 4. Ein Objekt ohne Sprite (im Feld "Grafik" einen Wert weit außerhalb der Sprite-Tabelle eintragen, zum Beispiel 60000; danach Cmd+Z) behält das gelbe Quadrat als Marker und löst keinen Fehler in der Statusleiste aus; das Quadrat bleibt auch nach Neuladen der Szene.
- [ ] 5. Beim Laden (Kopie frisch geöffnet, Header gewählt) zeigen die Objekte kurz das gelbe Quadrat, dann das Sprite. Auswahl, Hover und Ziehen funktionieren am Sprite wie am alten Marker; die Umrandung der Auswahl liegt um das Sprite.
- [ ] 6. HeartGold DE (Kopie `hg-3b1`): Neuborkia (New Bark Town in der US-Fassung; im Explorer nach dem Ort suchen) zeigt Personen als Sprites, ebenso mindestens ein Trainer auf einer Route; ein Objekt mit 3D-Grafik oder ohne Eintrag zeigt das gelbe Quadrat.
- [ ] 7. Neue Objekte (Art "Objekt", "Hinzufügen" in der Leiste der 3D-Ansicht) erscheinen sofort als Sprite oder Quadrat, ohne dass die Szene stockt.

## (D) Kachelnamen HGSS

Auf einer frischen Kopie von HeartGold DE (`hg-3b1`), mit der gebündelten App:

- [ ] 1. Eine Karte mit einem Kopfnuss-Baum (Headbutt-Baum; es gibt sie auf Routen und im Ilex-Wald; im Zweifel eine Route öffnen und den Hover über die Kacheln an den Bäumen wandern lassen) öffnen. Der Hover über eine solche Kachel zeigt den Namen `HEADBUTT` (Wert 0x06).
- [ ] 2. Der Hover über Gras, Wasser und eine Hüpfkante (Ledge) zeigt Namen aus dem HGSS-Satz (aus pokeheartgold), keine leeren und keine Zahlen für bekannte Werte.
- [ ] 3. Gegenprobe: im Platin-Projekt zeigt derselbe Wert 0x06 nicht `HEADBUTT` (Platin kennt den Namen nicht). Beim Wechsel zwischen den Projekten (Projekt schließen, anderes öffnen) bleibt der Satz des geöffneten Projekts, ohne Neustart der App.

## (E) Automatik

- [ ] 1. `scripts/gate.sh` läuft grün: `cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features`, `pnpm test`, `pnpm build`, danach `cargo test --workspace --all-features` mit `G4_TEST_PROJECT` gegen Platin DE, HeartGold DE und HeartGold US (dazu `G4_APICULA` und `G4_DSROM_CLI`). Das Skript liest die Originale nur; schreibende Tests arbeiten auf einer Kopie in einem Temp-Verzeichnis (`vanilla_copy()`). Es braucht rund 20 Minuten, siehe `docs/dev-setup.md`, Abschnitt "Gate-Skript". Die Tabelle am Ende ("Step / Project / Result") in die Ergebnisse unten übernehmen.
- [ ] 2. Die Tabelle hat keine Zeile "skipped": alle drei Projekte sind gelaufen.

## (F) Zeitlimit von apicula (optional)

- [ ] 1. `G4_APICULA_TIMEOUT_SECS=1 G4_APICULA="$PWD/tools/apicula/bin/apicula" pnpm tauri dev` mit einer frischen Kopie, deren `.g4/models` leer ist (löschen ist erlaubt, es ist Cache). Eine Karte mit großem Modell öffnen: die Szene zeigt die Karte **ohne Texturen** statt einer Fehlermeldung (der texturierte Versuch läuft ins Zeitlimit und fällt auf das untexturierte Modell zurück; das geht dann dauerhaft für dieses Tileset, bis die App neu startet). Der Rest der Oberfläche bleibt bedienbar, solange apicula arbeitet (kein Hängen).
- [ ] 2. In `<Projekt>/.g4/models/` liegt für den Timeout keine `.fail`-Datei; die Datei `last-tool` enthält 16 Hexzeichen.
- [ ] 3. Ohne `G4_APICULA_TIMEOUT_SECS` (Standard 60 s) und nach dem Löschen von `.g4/models` erscheinen die Modelle wieder mit Texturen.

## Ergebnis

Hier wird nach dem Durchlauf je Teil eingetragen: Datum, Projekt (Platin DE, HeartGold DE), App (gebündelt oder `tauri dev`), bestanden oder nicht, Auffälligkeiten.

| Teil | Wer | Datum | Projekt | Ergebnis | Auffälligkeiten |
|---|---|---|---|---|---|
| (A) Neuer Header per Assistent | Agent | 2026-10-23 | Platin DE (`pt-gate`), HeartGold DE (`hg-3b1`), Platin DE (`pt-3b1-rep`) | bestanden (1–11, 16, 18) | Browser-Modus statt gebündelter App. Platin: Patch schreibt jetzt `DE F7 AD F8`, Header 0593 `R231`, 0594 mit Eventdatei 328 (= 0342). Gang in `g4emu`: Spielerzimmer 0415 → Treppe → 0593 → Tür → 0411; NARC mit 595 Einträgen im ROM. HeartGold: Header 0540 `HG_TEST`, Gang 0064 → 0540 → 0060, `a/0/5/0` mit 541 Einträgen im ROM. (A) 18: Meldung und „Patch reparieren“ erscheinen, Reparatur ändert genau 3 Bytes, Spiel läuft; `pt-gate` zeigt keine Meldung. Warps im Spiel brauchen eine Tür-Kachel (Verhalten 0x69 **und** Kollision 0x80), auf freier Kachel passiert nichts. Eingetippter Name fiel bei „Weiter“ direkt nach dem Tippen auf die Vorgabe zurück (nur bei Klick ohne vorheriges Verlassen des Feldes beobachtet). (A) 17 entfällt (kein Diamant/Perl-Projekt). |
| (A) DSPRE-Gegenprobe | Hand | | | offen | DSPRE läuft nur unter Windows. Der von DSPRE geschriebene Patch für Platin DE ist fehlerhaft (siehe (A) 18). |
| (B) Höhen | Agent + Blick | 2026-10-23 | Platin DE (`pt-gate`) | Daten bestanden, Blick offen | Zelle (4, 26) von Raster 0000 über Inspector „Höhe“ = 2, Nachbar 0, Rückgängig → 0, `dirty` leer. Aussehen in 3D, Hover und Ziehen auf der erhöhten Zelle konnte der Agent nicht gezielt prüfen. |
| (C) Sprites | Agent + Blick | 2026-10-23 | Platin DE, HeartGold DE | Daten bestanden, Blick offen | Sprites je Grafik und Richtung als Bild gerendert (`../gate-artefakte/2026-10-23/sprites-pt.png` und `sprites-hg.png`, außerhalb des Repos): 0 Rücken, 1 Gesicht, 2 links, 3 rechts, aufrecht, nicht gespiegelt. Grafik 60000 und HG 101 ohne Sprite (Platzhalter). Billboard in 3D und Wechsel ohne Neuladen: Blick offen. |
| (D) Kachelnamen HGSS | Agent | 2026-10-23 | HeartGold DE (`hg-3b1`), Platin DE | bestanden | Hover zeigt „Typ 0x06 HEADBUTT“ (Route 29, Karten 0007 und 0053); Platin kennt 0x06 nicht; Projektwechsel ohne Neustart wechselt den Satz. 0x00 hat in HGSS bewusst keinen Namen. |
| (E) Automatik (`scripts/gate.sh`) | Agent | 2026-10-23 | Platin DE, HeartGold DE, HeartGold US | bestanden | 1393 s, keine Zeile „skipped“. |
| (F) apicula-Zeitlimit | Agent | 2026-10-23 | Platin DE (`pt-api-timeout`, frisch) | teilweise | Mit `G4_APICULA_TIMEOUT_SECS=1` schaffte apicula jedes Modell (Jubelstadt, Spielerzimmer) in unter 1 s, der Rückfall auf untexturiert wurde also nicht ausgelöst. Keine `.fail`-Datei, `last-tool` = `1001effb87a42dbf`, Oberfläche blieb bedienbar. |

Ergebnis von `scripts/gate.sh` (Tabelle aus der Ausgabe einfügen):

| Step | Project | Result |
|---|---|---|
| cargo fmt --check | - | ok |
| cargo clippy (-D warnings) | - | ok |
| cargo test (no ROM data) | - | ok |
| pnpm test | - | ok |
| pnpm build | - | ok |
| dist has no bridge | - | ok |
| g4emu unit | - | ok |
| g4emu integration | - | ok |
| gated tests (cargo test) | Pokemon - Goldene Edition HeartGold (Germany) | ok |
| gated tests (cargo test) | Pokemon - HeartGold Version (USA) | ok |
| gated tests (cargo test) | Pokemon - Platin-Edition (Germany) | ok |

Offene Punkte aus dem Durchlauf:

- Hand: DSPRE-Gegenprobe (A) 12–15; kurzer Durchgang mit der gebündelten App.
- Agent + Blick: (B) Aussehen der erhöhten Zelle, Hover und Ziehen; (C) Billboards in 3D und Richtungswechsel ohne Neuladen.
- Befunde (Liste beim Gate-Lauf): Warp auf freier Kachel löst im Spiel nicht aus und die App warnt nicht; Assistenten-Textfelder verlieren den Wert, wenn direkt nach dem Tippen „Weiter“ geklickt wird; Vorlage eines neuen Headers ist Header 0, wenn die erste gewählte Zelle eine Füllfläche ist; Ergebnis-Schritt nennt den neuen Header nicht als neu; (F) Zeitlimit-Rückfall nicht ausgelöst.
