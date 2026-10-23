# Phase 3b.2: Gate prüfen

Stand: 2026-10-23 vom Agenten durchlaufen (Browser-Modus mit `studio-bridge`, Emulator `g4emu`, main 6080351), siehe „Ergebnis“; offen sind (G), (H) und die Punkte, die rotom-lsp brauchen (Completion, Hover, Diagnosen im Editor).

Phase 3b.2 (Scripts und Text: Textarchive editieren, Scripts übernehmen, Script-Editor mit rotom-lsp, Level-Scripts, Navigation, Build mit Compile) ist implementiert und automatisch getestet. Das Gate braucht das GUI, melonDS und DSPRE und wird von Hand durchgeführt; die Automatik läuft über `scripts/gate.sh`. Die Ergebnisse stehen unten im Abschnitt "Ergebnis". Die Originale der Test-ROMs bleiben unberührt: alles läuft auf Kopien. Spezifikation: [docs/superpowers/specs/2026-10-22-phase-3b2-scripts-text-design.md](superpowers/specs/2026-10-22-phase-3b2-scripts-text-design.md), Abschnitt 1 (Gate). Plan: [docs/superpowers/plans/2026-10-22-phase-3b2-scripts-text.md](superpowers/plans/2026-10-22-phase-3b2-scripts-text.md).

Das Gate besteht aus neun Teilen:

- (A) Scripts übernehmen und Rebuild: der Bericht ohne Fehler, ein Build ohne Änderung ist byte-identisch.
- (B) NPC mit Textliteral auf der neuen Route. Das ist das eigentliche Gate.
- (C) Flag und Item: derselbe NPC gibt einmalig ein Item und sagt danach etwas anderes.
- (D) Vanilla-Dialog ändern.
- (E) Level-Script im Header-Editor.
- (F) Text-Panel: ändern, anfügen, suchen.
- (G) DSPRE-Gegenprobe.
- (H) Rauchtest der gepackten App (`pnpm bundle`).
- (I) Automatik: `scripts/gate.sh`.

Namen in der Oberfläche: Die Begriffe stehen in `apps/studio/src/workbench/terms.ts` und im Popover "Begriffe": "Scriptdatei" (alle Scripts eines Headers in einer Datei; ein Event nennt die Nummer `#n` darin), "Script" (ein Ablauf in rotom, zum Beispiel `script Main #1`), "Level-Script" (läuft beim Laden oder Betreten der Karte oder wenn eine Variable einen Wert hat), "Textarchiv" (die Nachrichten eines Headers), "Nachricht" (ein Text eines Textarchivs) und "Problem" (ein Fehler oder eine Warnung aus Compile, Text oder Sprachdienst). Geschrieben wird überall "Script", nicht "Skript". Die Panels: "Script" und "Text" sind Tabs der 3D-Gruppe (neben "3D", "Raster" und "Weltkarte"); "Probleme" ist eine eigene Gruppe **unter** der 3D-Gruppe, über dem Streifen "Status", und **kein** Tab neben "Status" (der Streifen hat keine Tab-Leiste und bleibt allein in seiner Gruppe). Im Panel "Script" zeigt ein Projekt ohne Übernahme den Kasten "Scripts übernehmen" mit dem Knopf "Übernehmen"; danach hat die Kopfzeile die Dokument-Auswahl, den Titel ("Scriptdatei 0042" oder "Level-Script 0042 (JSON)"), die Schaltfläche "Gliederung", "Speichern", "Script anfügen" und den LSP-Status (fehlt der Sprachserver: "rotom-lsp fehlt — Completion und Diagnosen aus"). Das Panel "Text" hat links die Archivliste mit Suchfeld und "Neues Archiv", rechts die Nachrichtenliste und das Feld "Text der Nachricht" mit "Nachricht anfügen" und "Letzte entfernen". Das Panel "Probleme" hat die Spalten Schwere, Datei, Zeile, Meldung, Quelle ("Compile", "Text", "live"); ein Klick springt zur Zeile. Im Header-Editor stehen "Scriptdatei" und "Level-Script" als Zahlenfelder mit "Öffnen" und "Neu", dazu der Abschnitt "Level-Script" mit den Auslösern "Beim Laden (on_load)", "Beim Fortsetzen (on_resume)", "Beim Betreten (on_transition)" und "Variablen-Bedingungen (on_var_condition)", "Eintrag hinzufügen", "Entfernen" und "Als JSON öffnen". Im Event-Inspector hat das Feld "Script" die Knöpfe "Öffnen" und "Neues Script". Im Explorer "Welt" hängen unter jedem Header die Kinder "Scriptdatei NNNN" und "Level-Script NNNN", und der Wurzelknoten "Scripts" listet alle Dateien mit Suche.

Zwei Regeln, die man im Ablauf merkt:

- Das Formular des Level-Scripts speichert nie ein Dokument, das rotom ablehnt: es zeigt stattdessen Hinweise ("Nicht gespeichert, bis das Level-Script gültig ist.").
- Ein Script speichern heißt Quelle schreiben, kompilieren, Member schreiben. Bei einem Compile-Fehler ist die Quelle gespeichert, der Member bleibt unverändert, und das Panel "Probleme" zeigt Datei, Zeile und Spalte. Hängt ein Textliteral an einem Textarchiv mit ungespeicherten Änderungen, verweigert das Speichern ("Textarchiv NNNN erst speichern").

## Vorbereitung

1. Kopien der Projekte anlegen (Platin DE, HeartGold DE), zum Beispiel:
   `cp -R "../Roms/Pokemon - Platin-Edition (Germany)_DSPRE_contents" ../Roms/_testkopien/pt-3b2`
   `cp -R "../Roms/Pokemon - Goldene Edition HeartGold (Germany)_DSPRE_contents" ../Roms/_testkopien/hg-3b2`
   Für jeden Durchlauf eine frische Kopie nehmen: "Scripts übernehmen" legt `rotom.toml`, `.rotom/` und `expanded/scripts/` an und schreibt Quellen, Scripts und Texte sofort auf Platte; das lässt sich nicht rückgängig machen. Die Kopie `pt-3b2` darf vor (A) weder von DSPRE noch von `rotom init` übernommen sein (es darf kein `rotom.toml` im Projekt liegen).
2. Gebündelte App bauen: `cd apps/studio && pnpm bundle && bash scripts/check-bundle.sh` (Ausgabe `bundle ok`, siehe `docs/release.md`). Gestartet wird `target/release/bundle/macos/Gen4 Studio.app`, `G4_APICULA` und `G4_ROTOM_LSP` dürfen nicht gesetzt sein. Beim ersten Start verlangt Gatekeeper eine Freigabe (siehe `docs/release.md`). Für Prüfungen, wo es dasteht, geht auch `G4_ROTOM_LSP="$PWD/tools/rotom/bin/rotom-lsp" G4_APICULA="$PWD/tools/apicula/bin/apicula" pnpm tauri dev` (siehe `docs/dev-setup.md`).
3. melonDS (die .app) und DSPRE bereithalten. DSPRE läuft nur unter Windows (oder einer Windows-Umgebung); die Kopie `pt-3b2` muss dort erreichbar sein.
4. Die neue Route aus dem 3a-Gate (Route 230, Header 0001, siehe [docs/phase-3a-gate.md](phase-3a-gate.md) (A)) wird für (B) und (C) gebraucht. Wer sie nicht mehr hat, legt sie auf `pt-3b2` mit dem Assistenten "Gebiet anlegen…" an (Header 0001 "NOTHING" umstellen) und ergänzt von Hand einen Warp hin und zurück (wie 3a (A) 11), damit sie in melonDS begehbar ist.
5. Netz: Das Studio braucht zur Laufzeit kein Netz; nur ein sauberer Cargo-Build lädt einmal `db-latest.zip` (rotoms `build.rs`), und der erste Bau von `rotom-lsp` braucht es auch. Das Studio selbst benutzt immer die mitgelieferte Befehlsdatenbank in `crates/g4-kernel/metadata/scrcmd/` (Revision 33), nie eine heruntergeladene. Siehe `docs/dev-setup.md`.

Fakten zu Platin DE, die der Ablauf braucht: Zweiblattdorf hat den Außen-Header 0411 und sechs Räume 0412 bis 0417, das Spielerzimmer ist Header 0415 (Karte 187); die Mutter steht im Spielerhaus (Erdgeschoss, im Explorer unter Zweiblattdorf zu finden); Route 201 ist Header 0342.

## (A) Scripts übernehmen und Rebuild

Ziel: "Scripts übernehmen" läuft durch, der Bericht nennt die Anzahl und keine Fehler, und das Projekt baut ohne weitere Änderung byte-identisch zum Vanilla-ROM. Der byte-identische Rebuild ist als gated Test automatisiert (`scripts/gate.sh`, Teil (I)); hier wird nur das GUI geprüft.

Auf der Kopie `pt-3b2` (Platin DE), mit der gebündelten App:

- [ ] 1. Projekt öffnen, Tab "Script". Der Kasten "Scripts übernehmen" erklärt, dass `rotom.toml` und `.rotom/` angelegt und alle N Scriptdateien nach `expanded/scripts/` dekompiliert werden und DSPRE danach dieselben Quellen benutzt. Im Projekt liegt noch kein `rotom.toml`.
- [ ] 2. "Übernehmen" drücken: der Fortschrittsbalken läuft, danach der Bericht mit der Zahl der Dateien. Die Fehlliste ist leer (oder nennt nur Member, die rotom nicht lesen kann; der gated Test hält die erlaubte Liste je Projekt fest). Die Panels "Script" und "Probleme" zeigen keine Fehlermeldung.
- [ ] 3. Auf der Platte: `rotom.toml`, `.rotom/command_database/`, `.rotom/status/compile-state.json` und `expanded/scripts/NNNN.rotom` bzw. `NNNN.json` (Level-Scripts) liegen im Projekt; `unpacked/scripts` ist unverändert (`diff -r` gegen das Original ohne `rotom.toml`, `.rotom`, `expanded`).
- [ ] 4. "Übernehmen" erscheint nicht noch einmal (Kasten weg, Dokument-Auswahl da). Projekt erneut öffnen: der Kasten bleibt weg, ohne Nachfrage.
- [ ] 5. Ohne weitere Änderung "Bauen": das ROM entsteht ohne Fehler und ohne Zeilen im Panel "Probleme". Optional `cmp` gegen ein ds-rom-Rebuild derselben Kopie (bei bestandenem gated Test nicht nötig).
- [ ] 6. HeartGold DE (Kopie `hg-3b2`): dieselben Schritte 1 bis 5.

## (B) NPC mit Textliteral

Ziel: Auf der neuen Route bekommt ein neuer NPC per "Neues Script" ein Script mit einem Textliteral; im Spiel spricht er diesen Text, und das Textarchiv des Headers zeigt die Nachricht im Panel "Text". Auf einer Kopie von Platin DE mit übernommenen Scripts (nach (A)):

- [ ] 1. Header 0001 (Route 230) im Explorer wählen; im Header-Editor stehen "Scriptdatei" und "Level-Script". Hat der Header noch keine eigene Scriptdatei (Nummer 0 oder eine geteilte), im Feld "Scriptdatei" "Neu" drücken: es entsteht eine neue Scriptdatei mit dem Rumpf `script Main #1:` und `End`, ihre Nummer steht im Feld, der Header ist ungespeichert (dirty). **Den Header jetzt speichern** (Toolbar "Speichern"): rotom ordnet ein Textliteral dem Textarchiv über die gespeicherten Header zu; solange der Header die neue Scriptdatei nicht auf der Platte hat, meldet der Compile im Panel "Probleme" "Textliteral nicht auflösbar: kein gespeicherter Header verwendet diese Scriptdatei".
- [ ] 2. Ein neues Objekt auf eine freie Kachel setzen: in der Leiste der 3D-Ansicht bei "Art des neuen Events" die Art "Objekt" wählen (Vorgabe ist "Warp"), die Maus über die Kachel halten, "Hinzufügen". Im Event-Inspector beim Feld "Script" "Neues Script" drücken: das Studio hängt an die Scriptdatei des Headers einen leeren Rumpf `script script_<N> #<N>:` mit `End` an (N = höchste vorhandene Nummer plus eins; bei der neuen Scriptdatei aus Schritt 1 ist N = 2) und kompiliert ihn. Die Nummer N steht im Feld, das Event ist dirty.
- [ ] 3. Event-Feld "Script" → "Öffnen": das Panel "Script" zeigt die Scriptdatei des Headers, der Cursor steht auf dem angehängten Rumpf am Ende. **Dieses Script bearbeiten, nicht ein zweites anlegen**: Zeile `script script_<N> #<N>:` stehen lassen (ein zweites `script …` mit demselben Namen oder derselben Nummer meldet "Script name or slot declared twice") und den Rumpf (`End`) durch diesen ersetzen (hier N = 2):
  ```
  script script_2 #2:
      LockAll
      FacePlayer
      Message "Hallo aus dem Studio!"
      WaitButton
      CloseMessage
      ReleaseAll
      End
  ```
  Weitere Befehle schlägt die Completion vor, sobald man einen Befehlsnamen tippt.
- [ ] 4. Cmd+S: Quelle gespeichert, kompiliert, kein Eintrag im Panel "Probleme". Das Textarchiv des Headers wird neu geladen; im Panel "Text" steht die neue Nachricht am Ende, mit `Hallo aus dem Studio!`.
- [ ] 5. Header und Events speichern, bauen (ROM entsteht ohne Fehler), in melonDS zu Route 230 laufen, den NPC ansprechen: er sagt den Text.

(Die Beispiel-Scripts dieses Dokuments aus (B) und (C) kompiliert der Test `gate_document_samples_compile` in `crates/g4-kernel/src/scripts.rs` gegen die mitgelieferte Platin-Datenbank, mit Nummern, die aus dem Rumpf von "Neues Script" folgen.)

## (C) Flag und Item

Derselbe NPC gibt einmalig ein Item und sagt danach einen anderen Text:

- [ ] 1. Im Panel "Script" im Script `script_2` (`#2`) alles unter der Zeile `script script_2 #2:` durch dieses Script ersetzen (die Zeile selbst und die Nummer bleiben; die Marke `AlreadyGiven:` kommt hinter das Script, am besten ans Ende der Datei):
  ```
  script script_2 #2:
      LockAll
      FacePlayer
      GoToIfSet FLAG_UNUSED_0x0094, AlreadyGiven
      Message "Hier, ein Geschenk!"
      WaitButton
      CloseMessage
      AddItem 17, 1
      SetFlag FLAG_UNUSED_0x0094
      ReleaseAll
      End

  AlreadyGiven:
      Message "Mehr habe ich nicht."
      WaitButton
      CloseMessage
      ReleaseAll
      End
  ```
  Beim Tippen von `FLAG_UN` schlägt die Completion die Flag-Namen der Datenbank vor, darunter `FLAG_UNUSED_0x0094` (laut Platin-Datenbank ungenutzt). `AddItem 17, 1` gibt ein Item mit der Nummer 17 (in Platin der Trank) einmal; das Ergebnis landet in der Variable `VAR_RESULT`.
- [ ] 2. Speichern: kein Problem. Ein absichtlicher Tippfehler (falscher Befehlsname) zeigt eine Diagnose im Editor, im Panel "Probleme" erscheint beim Speichern eine Zeile "Compile" mit Datei, Zeile und Spalte; ein Klick auf die Zeile springt an die Fehlstelle; der Member bleibt unverändert. Tippfehler beheben, speichern: die Zeile verschwindet.
- [ ] 3. Bauen, in melonDS: der NPC gibt beim ersten Ansprechen das Item (Beutel prüfen) und sagt beim zweiten Ansprechen den anderen Text.

## (D) Vanilla-Dialog

- [ ] 1. Mutter im Spielerhaus (Zweiblattdorf, Erdgeschoss): Event der Mutter wählen, beim Feld "Script" "Öffnen". Das Panel "Script" springt zu ihrem Script; ein Script, das an mehreren Stellen steht, nennt die Kandidaten statt eines Sprungs.
- [ ] 2. Den Text der Nachricht ändern (im Script bei einem Textliteral, oder im Panel "Text" bei der Nachricht des Archivs) und speichern. Cmd+S speichert zuerst den Store (Textarchive, Header, Events), dann die offenen Scripts, es gibt nur dieses eine Cmd+S (auch mit dem Cursor im Editor). Der Knopf "Speichern" im Script-Panel speichert nur das eine Dokument und verweigert bei einem Archiv mit ungespeicherten Änderungen ("Textarchiv NNNN erst speichern").
- [ ] 3. Bauen, in melonDS: die Mutter sagt den geänderten Text; die Datei `unpacked/textArchives/NNNN` hat sich geändert, die zugehörige `expanded/textArchives/NNNN.json` enthält den neuen Text.

## (E) Level-Script

- [ ] 1. Auf Route 230 den Header-Editor öffnen: der Abschnitt "Level-Script" zeigt "Kein Level-Script" mit "Anlegen" (Level-Script-ID 0). "Anlegen" drücken: eine neue Level-Script-Datei entsteht, ihre Nummer steht im Feld "Level-Script".
- [ ] 2. Erst das Script: im Panel "Script" die Scriptdatei des Headers öffnen (im Explorer das Kind "Scriptdatei NNNN" unter Header 0001) und "Script anfügen" drücken. Das hängt einen Rumpf `script script_<N> #<N>:` mit `End` an die Datei an (N = höchste Nummer plus eins). Den Rumpf durch eine Nachricht ersetzen, zum Beispiel `Message "Willkommen auf Route 230"` und `End` (Textliterale brauchen den gespeicherten Header, siehe (B) 1), und mit Cmd+S speichern. Dann im Header-Editor "Eintrag hinzufügen": Auslöser "Beim Laden (on_load)", Script-Nummer N. (Keine neue Scriptdatei anlegen: "Neu" beim Feld "Scriptdatei" oder "Level-Script" erzeugt eine ganze Datei, kein Script.) Das Formular speichert nur, wenn rotom das Dokument annimmt; die Hinweise ("Nicht gespeichert, bis das Level-Script gültig ist.") decken nur Variable 0, einen zweiten Eintrag "Variablen-Bedingungen (on_var_condition)" und Bedingungen ohne diesen Auslöser ab. Eine unbekannte Script-Nummer zeigt sich erst beim Compile als Eintrag im Panel "Probleme". "Als JSON öffnen" zeigt rotoms JSON im Panel "Script" ("Level-Script NNNN (JSON)") mit dem Eintrag `on_load`.
- [ ] 3. Speichern, bauen, in melonDS: beim Betreten der Karte läuft das Script und zeigt die Nachricht.

## (F) Text-Panel

- [ ] 1. Tab "Text": links die Archivliste mit Nummer und den Headern, die das Archiv nutzen (zum Beispiel "Header 0415, 0416"); die Nachrichtenzahl steht nur beim geöffneten Archiv. Ein Archiv wählen, eine Nachricht wählen: das Feld "Text der Nachricht" zeigt den Text, Steuercodes bleiben als Text sichtbar, `\n`, `\r` und `\f` stehen in der Liste lesbar.
- [ ] 2. Ändern: eine Nachricht im Feld "Text der Nachricht" ändern. Ein sehr langer Text zeigt die Überlängen-Warnung unter dem Feld und beim Speichern eine "Text"-Zeile im Panel "Probleme". Undo: Cmd+Z im Textfeld ist das Undo des Feldes selbst; das Undo der App ist der Knopf "Undo" der Toolbar oder Cmd+Z, nachdem der Fokus das Feld verlassen hat.
- [ ] 3. Anfügen: "Nachricht anfügen" drücken. Das hängt eine **leere** Nachricht an (die Liste zeigt "(leer)"); der Text muss danach im Feld "Text der Nachricht" getippt werden, zum Beispiel "Angefügt im Studio".
- [ ] 4. Speichern (Toolbar "Speichern" speichert auch Textarchive), bauen, in melonDS: geänderter und angefügter Text erscheinen im Spiel (die angefügte Nachricht über ein Script mit ihrer Nummer erreichen).
- [ ] 5. Suchfeld oben: der neue Text wird über alle Archive gefunden, Treffer zeigen Archiv, Nachrichtennummer und Auszug; ein Klick wählt Archiv und Nachricht.
- [ ] 6. "Neues Archiv": ein Archiv als letztes Mitglied entsteht, mit einer Nachricht "Neuer Text". Es wird sofort auf die Platte geschrieben (Member und JSON), braucht kein "Speichern" und lässt sich nicht rückgängig machen. Ein Build packt es.

## (G) DSPRE-Gegenprobe

Platin DE, Kopie `pt-3b2` nach (F):

- [ ] 1. Die Kopie vor dem Öffnen in DSPRE noch einmal kopieren (`cp -R .../pt-3b2 .../pt-3b2-dspre`) und **die zweite Kopie** in DSPRE öffnen, damit DSPRE (das beim Speichern Dateien neu schreibt) das Testergebnis nicht überschreibt. DSPRE öffnet das Projekt ohne Fehlermeldung.
- [ ] 2. Das Textarchiv des Headers (das der Mutter aus (D) bzw. das Archiv aus (B)) zeigt die geänderte und die angefügte Nachricht.
- [ ] 3. DSPRE `main` zeigt wegen `rotom.toml` seine alten Skripteditoren als deaktiviert (der Avalonia-Zweig benutzt dieselben Quellen unter `expanded/scripts`). Ein Level-Script und die Scripts sind dort lesbar bzw. öffnen sich in ihrem Editor, soweit er verfügbar ist.
- [ ] 4. Nichts in DSPRE speichern. Optional: DSPRE baut aus der zweiten Kopie ein ROM, das in melonDS startet.

## (H) Rauchtest der gepackten App

Die Fragen sind WKWebView-spezifisch: laufen CodeMirror 6, der Tauri-Kanal für `rotom-lsp` und die CSP in der gepackten App? Im Entwicklungsbetrieb ist das nicht gedeckt.

Bundle (`cd apps/studio && pnpm bundle && bash scripts/check-bundle.sh`):

Bundle gebaut und mit `check-bundle.sh` geprüft am 2026-10-23: `pnpm bundle` lief ohne Fehler (nur die bekannte, harmlose Warnung „stripping debug info with `rust-objcopy` failed“), `check-bundle.sh` meldete `bundle ok: target/release/bundle/macos/Gen4 Studio.app` (34,55 MiB). Das prüft Sidecars, dylib und rpath, nicht das Verhalten im WKWebView: die Schritte von Hand unten sind **nicht** durchgeführt.

Von Hand mit der gepackten App (`target/release/bundle/macos/Gen4 Studio.app`, `G4_APICULA` und `G4_ROTOM_LSP` nicht gesetzt) auf einer Testkopie (nicht auf einem Original):

- [ ] 1. Die App startet (Gatekeeper-Freigabe beim ersten Mal), das Projekt `pt-3b2` oder `hg-3b2` öffnet.
- [ ] 2. Tab "Script": Script öffnen (über ein Event, "Öffnen"), die Kopfzeile zeigt den LSP-Status als laufend, nicht "rotom-lsp fehlt".
- [ ] 3. Im Editor einen Befehl beginnen (zum Beispiel `Mes`): die Completion erscheint mit Befehlen aus der Befehlsdatenbank; Hover über einen Befehl zeigt die Signatur.
- [ ] 4. Einen absichtlichen Fehler tippen und speichern: das Panel "Probleme" zeigt die Zeile mit Datei, Zeile und Spalte, Klick springt dorthin; Fehler beheben, speichern: Zeile weg.
- [ ] 5. Die Konsole der App (Rechtsklick, "Element untersuchen", falls verfügbar) zeigt keine CSP-Verletzungen (`Refused to ...`) beim Öffnen des Editors.

## (I) Automatik

- [x] 1. (2026-10-23, siehe "Ergebnis") `scripts/gate.sh` läuft grün: `cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features`, `pnpm test`, `pnpm build`, danach `cargo test --workspace --all-features` mit `G4_TEST_PROJECT` gegen Platin DE, HeartGold DE und HeartGold US (dazu `G4_APICULA`, `G4_DSROM_CLI` und neu `G4_ROTOM_LSP`). Die gated Tests dieser Phase: "Scripts übernehmen" läuft auf allen drei Projekten ohne Fehler, ein Compile direkt nach dem Übernehmen ist sauber, und das nach dem Übernehmen neu gebaute ROM ist byte-identisch zum ds-rom-Rebuild. Das Skript liest die Originale nur; schreibende Tests arbeiten auf einer Kopie in einem Temp-Verzeichnis (`vanilla_copy()`). Es braucht rund 25 Minuten, siehe `docs/dev-setup.md`, Abschnitt "Gate-Skript".
- [x] 2. Die Tabelle hat keine Zeile "skipped": alle drei Projekte sind gelaufen.

Der Lauf vom 2026-10-23 steht unter "Ergebnis"; für den Merge wird er wiederholt, wenn danach Code geändert wurde.

## Ergebnis

Hier wird nach dem Durchlauf je Teil eingetragen: Datum, Projekt (Platin DE, HeartGold DE), App (gebündelt oder `tauri dev`), bestanden oder nicht, Auffälligkeiten.

| Teil | Wer | Datum | Projekt | Ergebnis | Auffälligkeiten |
|---|---|---|---|---|---|
| (A) Scripts übernehmen und Rebuild | Agent | 2026-10-23 | Platin DE (`pt-3b2`), HeartGold DE (`hg-3b2`) | bestanden | Browser-Modus mit `studio-bridge` statt gebündelter App. Bericht „1124 Scriptdateien dekompiliert, 0 leere übersprungen“ (HG 965), keine Probleme; `rotom.toml`, `.rotom/command_database`, `compile-state.json`, `expanded/scripts` angelegt, `unpacked/scripts` unverändert; Kasten nach Neuöffnen weg; Bauen ohne Probleme. Byte-Gleichheit: gated Test `vanilla_rebuild_matches_dsrom_cli_after_adopt` (I). |
| (B) NPC mit Textliteral | Agent (Completion: Hand) | 2026-10-23 | Platin DE | bestanden | Scriptdatei 1124 für Header 0001 neu, Objekt mit „Neues Script“ → `script_2 #2`, Literal-Script gespeichert, Archiv 0023 Nachricht 101; im Spiel (`g4emu`) sagt der NPC „Hallo aus dem Studio!“. Der Hinweis „Textliteral nicht auflösbar …“ erscheint vor dem Speichern des Headers; danach muss das Script **erneut** gespeichert werden, Cmd+S allein kompiliert es nicht neu. Completion (rotom-lsp) ist im Browser-Modus aus. |
| (C) Flag und Item | Agent (Completion, Editor-Diagnose, Beutel: Hand) | 2026-10-23 | Platin DE | bestanden | Im Spiel erst „Hier, ein Geschenk!“, dann „Mehr habe ich nicht.“ Tippfehler → Zeile „Analysis error: Unknown command 'AddItemm'“ (Zeile 11, Compile), Klick springt hin, Member unverändert. Den Beutel konnte `g4emu` nicht öffnen (Menü reagiert mit diesem Spielstand nicht). Das Panel „Probleme“ zeigt keine Spalte. |
| (D) Vanilla-Dialog | Agent | 2026-10-23 | Platin DE | bestanden | Satz der Mutter an der Tür (Archiv 557, Nachricht 4) im Panel „Text“ geändert; Member und `expanded/textArchives/0557.json` geändert; im Spiel „Mutter: … Das Studio hat diesen Text …“. Script-Panel „Speichern“ verweigert mit „Textarchiv 0023 erst speichern“, Cmd+S speichert Archiv und Script. Event → „Öffnen“ in (B) geprüft; die Mutter in der 3D-Ansicht anklicken konnte der Agent nicht, Sprungziel über `resolve_script` (Datei 1055, Zeile 151). |
| (E) Level-Script | Agent | 2026-10-23 | Platin DE | **nicht bestanden wie beschrieben** | Header 0001 hat in Platin das geteilte Level-Script 897 (nicht 0), daher „Neu“ → 1125. Eintrag `on_load` → Script 3 (mit `Message`) gespeichert, JSON korrekt, aber im Spiel bleibt die Karte **schwarz** und die Figur hängt. Mit „Variablen-Bedingungen (on_var_condition)“ (Variable 16433 = 0 → Script 3, das die Variable auf 1 setzt) erscheint „Willkommen auf …“ beim Betreten. Die Formular-Hinweise funktionieren. |
| (F) Text-Panel | Agent | 2026-10-23 | Platin DE | bestanden | Steuercodes lesbar, Suche wählt Archiv und Nachricht, angefügte Nachricht 104 im Spiel über `Message 104`, Überlängen-Warnung unter dem Feld und als „Text“-Zeile, „Neues Archiv“ 0724 sofort auf der Platte. Beim Speichern eines Scripts erscheinen 44 bis 58 Überlängen-Warnungen zu unveränderten Original-Texten aus Archiv 0023. Undo im Textfeld nicht geprüft. |
| (G) DSPRE-Gegenprobe | Hand | | | offen | DSPRE läuft nur unter Windows. |
| (H) Rauchtest der gepackten App | Hand | 2026-10-23 (nur Bundle) | | Bundle ok; Schritte von Hand offen | `pnpm bundle` und `check-bundle.sh` laufen; Start, LSP-Status, Completion, Hover und Fehlerprüfung in der gepackten App sind nicht geprüft |
| (I) Automatik (`scripts/gate.sh`) | Agent | 2026-10-23 | Platin DE, HeartGold DE, HeartGold US | bestanden (1335 s; auf main 6080351 erneut grün, 1541 s) | keine |

Ergebnis von `scripts/gate.sh` (Tabelle aus der Ausgabe):

```
Step                                                 Project                                      Result                 Seconds
----                                                 -------                                      ------                 -------
cargo fmt --check                                    -                                            ok                           1
cargo clippy (-D warnings)                           -                                            ok                          12
cargo test (no ROM data)                             -                                            ok                          29
pnpm test                                            -                                            ok                          10
pnpm build                                           -                                            ok                           3
gated tests (cargo test)                             Pokemon - Goldene Edition HeartGold (Germany) ok                         517
gated tests (cargo test)                             Pokemon - HeartGold Version (USA)            ok                         308
gated tests (cargo test)                             Pokemon - Platin-Edition (Germany)           ok                         455

Gate: ok (1335 s)
```

Lauf vom 2026-10-23 auf dem Stand des Commits vor diesem Dokument, mit `G4_ROTOM_LSP`, `G4_APICULA` und Standard-`G4_ROMS_DIR`. In jedem der drei Projekte liefen `vanilla_adopts_and_compiles_clean` (Übernehmen ohne Fehler, Compile direkt danach sauber), `vanilla_rebuild_matches_dsrom_cli` und `vanilla_rebuild_matches_dsrom_cli_after_adopt` (ROM nach dem Übernehmen byte-identisch) grün; keine Zeile "skipped"; 2076 Tests bestanden, 0 fehlgeschlagen (Summe aller `test result`-Zeilen des Laufs).

Offene Punkte aus dem Durchlauf:

- Hand: (G) DSPRE-Gegenprobe; (H) gepackte App mit rotom-lsp (Completion bei `Mes` und `FLAG_UN`, Hover, Diagnose im Editor, keine CSP-Fehler); Beutel nach (C) prüfen.
- (E) im Dokument ändern: eine Nachricht beim Betreten braucht eine Variablen-Bedingung, kein `on_load` (das hängt die Karte auf). Header 0001 hat in Platin schon ein Level-Script.
- Befunde: Cmd+S kompiliert ein Script nach „Textliteral nicht auflösbar“ nicht neu; Überlängen-Warnungen für unveränderte Original-Texte; keine Spalte im Panel „Probleme“; Cmd+A im Feld „Text der Nachricht“ markierte im Agent-Lauf nicht alles.
- Bilder des Laufs: `../gate-artefakte/2026-10-23-3b2/` (außerhalb des Repos).
