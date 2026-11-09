# Gutachten C: Projektstart aus ROM und MCP-Server als Produktbestandteil

Stand: 2026-10-25. Gutachter: Architektur-Review (nur lesend). Repo: `$HOME/Projekte/lab/editor/gen4-studio`.
Gelesen: `docs/ROADMAP.md`, `docs/superpowers/specs/2026-10-21-gen4-studio-design.md`, `crates/g4-kernel/src/{project,build,run,disk,export,dynamic_headers}.rs`, `crates/g4-kernel/src/bin/g4k.rs`, `crates/g4-kernel/tests/build.rs`, `crates/g4-formats/src/game.rs`, `apps/studio/src/ui/frame/StartPane.tsx`, `apps/studio/src/platform/transport.ts`, `apps/studio/src/bus/editBus.ts`, `apps/studio/src/reload.ts`, `apps/studio/src-tauri/src/{lib,state}.rs`, `commands/{mod,project,wizards,data/mod}.rs`, `bridge/{mod,http,dispatch,roots,batch}.rs`, `bin/studio-bridge.rs`, `docs/{dev-setup,release,agent-gate}.md`, `docs/superpowers/specs/2026-10-22-agent-test-harness-design.md`, Analyse `docs/research/2026-10-20-dspre-und-suite-analyse.md`.
Eigene Messung: `target/debug/g4k extract` auf Platin DE und HeartGold DE in den Scratchpad (Originale nur gelesen, Ergebnis danach gelöscht).

Hinweis zu den Annahmen im Auftrag: `run.rs` startet nur den Emulator, `export.rs` ist der apicula-Modellexport. Beide haben mit dem Projektstart nichts zu tun. Das Entpacken steckt in `build.rs` (`extract_rom`). Die Brücke hat nicht rund 120, sondern **91 Befehle** (`bridge::COMMANDS` in `apps/studio/src-tauri/src/bridge/dispatch.rs`; ein Test hält die Liste gleich mit `generate_handler!` in `lib.rs`).

---

## Teil A: Projektstart „Neues Spiel aus ROM“

### A.1 Was der Kernel heute kann (Befund)

| Fähigkeit | Stand | Beleg |
|---|---|---|
| ROM entpacken | **Ja, als Bibliothek.** `extract_rom(nds, dest)` ruft `ds_rom::rom::raw::Rom::from_file`, dann `Rom::extract` und `Rom::save`, danach `Project::open`. Ziel muss fehlen oder leer sein. Ein verschlüsseltes ARM9 ergibt eine eigene Meldung. | `crates/g4-kernel/src/build.rs:231-260` |
| CLI | `g4k extract <rom> <dest>` | `crates/g4-kernel/src/bin/g4k.rs:41-42, 150-152` |
| Test | `extract_rom_creates_openable_project` (gated über `G4_TEST_ROM`): Spielcode aus Bytes 0x0C..0x10 = `Project::game_code`, Karten entpackbar, Header lesbar | `crates/g4-kernel/tests/build.rs:413-430`, `docs/dev-setup.md:153` |
| App-Befehl | **Fehlt.** Kein Tauri-Befehl, keine Brücke, keine UI. Der Startbildschirm kennt nur „Projekt öffnen“ und sagt „Projekte sind entpackte DSPRE-Ordner.“ | `apps/studio/src/ui/frame/StartPane.tsx:117-128`; `grep extract apps/studio` ohne Treffer |
| Spielerkennung | `Project::open` liest `gamecode` aus `header.yaml`. `Family::from_game_code`: ADA/APA → Dp, CPU → Pt, IPK/IPG → Hgss. `Language` kommt aus dem 4. Zeichen. `ArchiveMap::for_game` liefert die NARC-Pfade je Spielcode. | `project.rs:466-493`, `crates/g4-formats/src/game.rs:1-50` |
| Getestete Codes | CPUD, IPKD, IPKE (`verified: [CPUD, IPKD, IPKE]` beim Patch, drei Testprojekte im Gate) | `crates/g4-kernel/metadata/dynamic_headers.yaml:43`, `docs/dev-setup.md:417, 444` |
| Diamant/Perl | Öffnen geht, die Daten-Editoren sperren sich mit `DP_LOCKED = "Diamant/Perl: ohne Testprojekt nicht freigegeben"`, Trainer-Scripts geben für DP `None` | `commands/data/mod.rs:51-72`, `scripts.rs:1055-1083` |
| Entpacken der NARCs | **lazy**: `ensure_unpacked` entpackt eine Art beim ersten Zugriff atomar (Temp-Ordner, dann rename). `expanded/` entsteht erst durch „Scripts übernehmen“ und Text-JSON. | `project.rs:684-732, 503-507`; `docs/phase-3b2-gate.md:180` |

**Messung (2026-10-25, Debug-Build, ds-rom mit `opt-level = 3`):**

| ROM | Dauer `g4k extract` | Größe danach | DSPRE-Ordner zum Vergleich |
|---|---|---|---|
| Platin DE (128 MiB) | 0,49 s | 101 MB | 202 MB (mit allen 41 `unpacked/`-Ordnern) |
| HeartGold DE (128 MiB) | 0,37 s | 124 MB | 238 MB |

**Kompatibilität mit DSPRE:** `arm9/`, `arm9_overlays/`, `arm7/`, `banner/`, `files/`, `header.yaml` und `path_order.txt` sind nach SHA-1 **byte-gleich** mit den DSPRE-Ordnern unter `../Roms`. Es gibt genau eine Abweichung: Die `config.yaml` von DSPRE hat einen zusätzlichen Block `padding:` (arm9 0, sonst 255). Den schreibt unser gepinntes ds-rom (`rev 8a9c151`, `crates/g4-kernel/Cargo.toml:15`) nicht, das DSPRE-ds-rom ist neuer. Der Vanilla-Rebuild bleibt byte-gleich (gated Test `vanilla_rebuild_matches_dsrom_cli`). Ob DSPRE einen Ordner ohne `padding:` und ohne vorab entpackte `unpacked/`-Ordner öffnet, ist **nicht belegt**. Das gehört als Handpunkt ins Gate (wie B8 in 3b.4b).

Fazit A.1: Der Kernel kann das Entpacken schon, schnell und DSPRE-nah. Es fehlen Befehl, Erkennung vor dem Entpacken, Vorlagen, Metadaten zum Basis-ROM und die Oberfläche.

### A.2 Spielerkennung vor dem Entpacken

Kernel-Funktion `inspect_rom(path) -> RomInfo`. Sie liest nur den Kopf und die Prüfsumme:
- Titel (0x00..0x0C), Spielcode (0x0C..0x10), Version (0x1E), Größe;
- SHA-1 der ganzen Datei, gegen eine Tabelle bekannter sauberer Dumps (nur Hashes, keine ROM-Daten; Metadatei `metadata/base_roms.yaml`). Das Ergebnis heißt „sauberer Dump“, „unbekannt oder schon verändert“ oder „verschlüsselt“;
- Freigabe-Stufe aus Spielcode und Metadaten:
  - **freigegeben**: CPUD, IPKD, IPKE;
  - **ohne Testprojekt**: andere CPU-, IPK- und IPG-Sprachen, SoulSilver. Die Metadaten kennen die Pfade (`dynamic_headers.yaml:14-31`). Entpacken ist erlaubt, mit Badge „nicht geprüft“;
  - **gesperrt**: ADA/APA (Diamant/Perl) mit dem Grund aus `DP_LOCKED`. Die Empfehlung ist, gar nicht erst zu entpacken, weil die Vorlagen und Editoren dort nicht gelten;
  - **unbekannt**: alles andere, mit „Kein Gen-4-Spiel erkannt“.

Die SHA-1 brauchen wir sowieso: Phase 4 verlangt eine „Basis-ROM-Prüfung per SHA-1“ und den xdelta-Export (`ROADMAP.md:213`). Darum schreibt der Projektstart sie gleich mit (siehe A.5, `.g4/base.json`).

### A.3 Was „leere Welt“ technisch heißt

Was das Spiel zum Booten und Spielen mindestens braucht (Gen 4 ist kein Decomp-Build, `ROADMAP.md`/Analyse §Decomps):

1. **Header-IDs bleiben alle.** Der Code (ARM9 und Overlays) verweist hart auf Header: Heilpunkte (Fliegen, Ohnmacht → zurück zum letzten Pokémon-Center oder nach Hause), Union-Raum und Kommunikationsräume, Kampfzonen, Ruhmeshalle, Pal-Park, in Platin der Untergrund, in HGSS Pokéathlon, Safari und Käferturnier. Löschen oder Umnummerieren bricht solche Pfade. Die Studio-Regel „Header löschen: nicht in 3b.1“ (`ROADMAP.md:115`) passt dazu.
2. **Der Startpunkt ist im Code fest:** Header, Koordinaten und Blickrichtung des neuen Spiels. DSPRE hat dafür einen eigenen Spawn-Editor mit ARM9-Offset. Gen4 Studio kennt den Startpunkt nicht (keine Metadaten, `grep spawn` nur in der scrcmd-Datenbank). Das Studio portiert keinen DSPRE-Code (Spec §2), der Offset müsste also aus pret kommen und je Spielcode geprüft werden.
3. **Intro (Professor-Rede, Namenswahl)** ist Overlay-Code. Überspringen heißt einen Code-Patch einspielen. Die Texte selbst liegen in Textarchiven und sind schon heute editierbar.
4. **Globale Scripts und Systemtexte müssen bleiben**: gemeinsame Script-Dateien für Heilen, PC und Items, Menü-, Kampf- und Systemtexte. Leeren darf man nur, was an Orten hängt: Eventdateien, Level-Scripts und Ort-Scripts der Außen-Header.
5. **Story-Flags steuern das Menü.** Pokédex, Pokétch und Pokégear, Laufschuhe und Starter vergibt die Story per Script. In einer leeren Welt fehlt das, wenn kein Start-Script es ersetzt.
6. **Matrix 0 ist die Oberwelt** (Stadtkarte, Fliegen). Leere Zellen (`MATRIX_EMPTY = 0xFFFF`, `crates/g4-formats/src/matrix.rs:7`) sind in Gen 4 riskant, sobald man sie betreten kann. Sicherer ist eine Rand-Karte mit gesperrter Kollision.

Daraus folgen drei Stufen:

| Stufe | Inhalt | Machbarkeit |
|---|---|---|
| L0 „Original behalten“ | Entpacken, nichts ändern | sofort (Kernel kann es) |
| L1 „Leere Region mit Startstadt“ | Alle Header und Daten bleiben. Matrix 0 wird bis auf die Startstadt mit einer Rand-Karte gefüllt (eine kopierte Karte mit gesperrter Kollision, ohne Gebäude: `duplicate_map` mit `clear_buildings`, Permissions setzen). Die Außen-Header verweisen auf leere Eventdateien (`create_event_file`), ihre Level-Scripts werden entfernt. Die Startstadt behält den **originalen Start-Header** und das Spielerhaus mit seinen Innenräumen, bekommt aber eine leere Außenkarte (oder die originale Stadt ohne Figuren). Dazu kommt ein **Start-Script** (rotom, Vorlage): Starter geben, Flags für Pokédex und Laufschuhe setzen. Script-Dateien und Textarchive werden **nicht gelöscht, nur nicht mehr referenziert**: das ist umkehrbar und hält IDs stabil. | mit vorhandenen Befehlen machbar (siehe A.6). Der Startpunkt muss sich nicht ändern, weil der Start-Header bleibt. |
| L2 „Ganz leer“ | Eigener Startort an beliebiger Stelle, Intro übersprungen, eigene Heilpunkte | braucht ARM9- und Overlay-Patches (Startpunkt, Intro, Heilpunkt-Tabelle). Erst nach einem Spike mit pret-Quellen und g4emu, eher ab Phase 4 oder 6 |

**Empfehlung (recommend): L1 als „Leere Welt“ anbieten und so benennen, wie es ist: „Leere Region mit Startstadt“.** L2 nicht versprechen. Der Trick ist, den originalen Start-Header und das Spielerhaus wiederzuverwenden: dann bleiben Startpunkt, Intro und Heilpunkt „Zuhause“ gültig, ohne ein Byte im ARM9 zu patchen.

### A.4 Vorlagen je Edition

Vorlagen sind **Rezepte, keine Dateien**: eine Metadatei je Spielfamilie mit Header-IDs, Matrix, Start-Header und Kartenwahl. Dazu kommt eine Folge von Kernel-Schritten. Im Repo liegen damit nie ROM-Daten (Spec §6).

| Vorlage | Platin | HGSS | Inhalt |
|---|---|---|---|
| **Original** | ja | ja | L0, Startbereich Welt |
| **Leere Region mit Startstadt** | Startstadt auf dem Platz von Zweiblattdorf (Start-Header bleibt) | Startstadt auf dem Platz von Neuborkia. Kanto liegt in derselben Matrix 0 und wird mitgeleert. Der Begleiter und die Pokégear-Karten hängen an Flags. | L1 |
| **Nur Daten** (Balancing-Hack) | ja | ja | L0, aber Atlas startet in Pokémon, „Scripts übernehmen“ nicht nötig, die Prüfungen sind auf Daten gefiltert. Das ist eine UI-Voreinstellung, kein anderes Projekt. |

Nicht vor Phase 4: Vorlagen mit eigener Region als Textprojekt, symbolische IDs (`MAP_TWINLEAF_TOWN` statt 414) in den Rezepten, „Auf Original zurücksetzen“ und xdelta je Vorlage. Phase 4 macht die Rezepte robuster und das Leeren zum Löschen von Entitäten. Das Rezept-Format sollte deshalb von Anfang an Namen statt Zahlen erlauben, mit einer Zuordnungstabelle je Spielcode. Was Phase 3d bringen soll (Flags und Variablen, `ROADMAP.md:246`), braucht der Start-Script-Assistent: bis dahin ist das Start-Script eine feste rotom-Vorlage mit Flags aus der scrcmd-Datenbank.

### A.5 Varianten für den Startfluss

**Variante S1: Assistent im Scharnier (wie „Gebiet anlegen“)**, recommend.
Start-Screen: zweiter Hauptknopf **„Neues Spiel aus ROM…“** neben „Projekt öffnen“, Hinweis „Du brauchst deine eigene ROM-Datei (.nds).“ Die Schritte:
1. **ROM wählen** (nativer Dialog, Filter `.nds`). `inspect_rom` zeigt Spiel, Sprache, Freigabe und Dump-Status.
2. **Vorlage** (drei Karten mit DS-Bild, ohne Testprojekt mit Badge, DP gesperrt mit Grund).
3. **Name und Zielordner**: Vorschlag `~/Documents/Gen4 Studio/<Name>_DSPRE_contents`, Prüfung „leer oder fehlt“ wie in `extract_rom`, freier Platz ungefähr 2,5-mal die ROM-Größe.
4. **Zusammenfassung** („wird angelegt: …, die ROM bleibt unverändert“).
5. **Anlegen mit Fortschritt** in Phasen: Lesen, Entpacken, Vorlage anwenden (n/m Schritte), Prüfen.
6. **Ergebnis**: „Öffnen“ (Standard), dazu ein Toast mit dem nächsten Schritt.

Gleiche Muster wie die vorhandenen Assistenten, also wenig neue UI.

**Variante S2: Ein Dialog ohne Schritte** („ROM wählen → sofort Original entpacken und öffnen“, Vorlagen später als Assistent „Welt leeren…“ im offenen Projekt). Schneller gebaut, aber die Wahl beim Start, die der Nutzer will, geht verloren.

**Variante S3: Drag-and-drop einer `.nds` auf den Start-Screen** als Abkürzung in S1 (springt zu Schritt 2). Sinnvoll als Zusatz.

Empfehlung: S1 plus S3. Dazu kommt ein Assistent „Welt leeren…“ im offenen Projekt (Technik › Werkzeuge), der dasselbe Rezept anwendet. Dann ist die Vorlage kein Einmal-Pfad, und der Gate-Test kann sie auf jeder Testkopie fahren.

**Ordnername und Brücke, ein echter Konflikt:** Die Brücke behandelt jede Pfadkomponente mit `_DSPRE_contents` als **Original** und sperrt sie (`bridge/roots.rs:7-17`). `gate.sh` nimmt alle `*_DSPRE_contents` unter `G4_ROMS_DIR` als Testprojekte (`docs/dev-setup.md:417`). Legt der neue Fluss DSPRE-typisch `<Name>_DSPRE_contents` an, ergeben sich zwei Folgen:
- Agenten-Tests müssen in `_testkopien` ohne diese Endung anlegen (Option `--no-dspre-suffix` oder ein freier Name);
- für das Produkt (MCP, Teil B) taugt die Suffix-Regel nicht als Schutz. Ersatz ist eine Marke `.g4/base.json` mit `origin: "extracted"` oder die Freigabe je Projekt.

Ob DSPRE die Endung braucht, ist nicht belegt (Handpunkt). Empfehlung: Standardname mit Endung, damit DSPRE den Ordner kennt, aber frei änderbar.

### A.6 Nötige Kernel- und App-Befehle

Kernel (`g4-kernel`):
- `rom::inspect(path) -> RomInfo { title, game_code, family, language, revision, sha1, size, support: Verified|Untested|Locked(reason)|Unknown, clean_dump: Option<&'static str> }`
- `build::extract_rom_with(nds, dest, progress: &mut dyn FnMut(Phase))`: Hülle um das vorhandene `extract_rom`, die die Phasen meldet. ds-rom selbst meldet keinen Fortschritt, bei 0,5 s reicht das. Sie schreibt `.g4/base.json` (`sha1`, `game_code`, `template`, `created`, `studio_version`) und, falls nötig, einen vorgegebenen `padding:`-Block für DSPRE (nur nach Prüfung).
- `templates::list(family) -> Vec<TemplateInfo>` und `templates::apply(project, id, progress)`. Das Rezept geht über den `Store` (eine `Store::transaction`, ein Verlaufsschritt „Vorlage: Leere Region“). Es nutzt die vorhandenen Bausteine `duplicate_map`, `set_permission`, `set_matrix_cell`, `create_event_file`, `set_header`, `new_script_file` und `write_source`. Das Rezept steht in `metadata/templates.yaml`.
- Optional `extract_rom` mit eagerem Entpacken aller Arten („für DSPRE vorbereiten“), nur falls DSPRE das braucht.

App (`apps/studio/src-tauri/src/commands/project.rs` oder neues `start.rs`) und Brücke:
- `inspect_rom { path }`, `create_project { rom, dest, template }` mit Fortschritts-Events (Tauri `emit("create-progress")`), danach wie `open_project_impl`. In der Brücke gilt dieselbe Pfadregel wie bei `build_rom`: Quelle lesbar unter den Wurzeln oder `G4_IMPORT_ROOT`, Ziel unter `_testkopien`.
- Frontend: `StartPane` mit zweitem Knopf, `NewProjectWizard`, `api.start.ts`.

### A.7 Risiken

- **Recht:** Nur eigene Dumps. Im Repo, in Tests und in Vorlagen liegen keine ROMs und keine ROM-abgeleiteten Assets. Die Rezepte enthalten nur IDs und Offsets. Die SHA-1-Tabelle sauberer Dumps ist unkritisch (öffentliche Prüfsummen wie bei No-Intro). Im UI steht ein Satz „Gen4 Studio liefert keine Spieldaten; du nutzt deine eigene ROM“. Kein Download-Link, keine Suche nach ROMs.
- **Dateigröße und Dauer:** rund 100 bis 125 MB je Projekt direkt nach dem Entpacken, mit allen entpackten Ordnern und `expanded/scripts` gut 250 MB. Gemessene 0,4 bis 0,5 s (SSD, macOS). Auf Windows mit Virenscanner und tausenden kleinen Dateien eher Sekunden bis Minuten (rund 40 Ordner mit Tausenden `NNNN`-Dateien). „Scripts übernehmen“ (1124 Dateien, Platin) gehört **nicht** in den Startfluss, das bleibt der explizite Schritt aus 3b.2.
- **DSPRE-Kompatibilität:** `padding:` in `config.yaml` und lazy `unpacked/`. Nachweis per Hand in DSPRE.
- **L1 auf HGSS:** Kanto in Matrix 0, Begleiter-Pokémon, Pokégear-Flags, Telefon-Scripts. Mehr Prüfaufwand als auf Platin.
- **Verstecktes Story-Wissen:** Das Spielerhaus kann über Flags auf Ereignisse außerhalb warten (Mutter, Rivale). Ein g4emu-Szenario muss zeigen, dass man nach dem Intro frei laufen kann und nach einer Niederlage zu Hause aufwacht.
- **Prüfungen:** Nach L1 meldet `checks` viele Befunde (Warps ins Leere, `event.script-missing`). Das Rezept muss so arbeiten, dass `run_checks` danach 0 Fehler zeigt. Das ist ein gutes Gate-Kriterium.

### A.8 Teilphasen und Gates

**3e.1 „Neues Spiel aus ROM“ (Original)**
- Lieferung: `inspect_rom`, `create_project` (Vorlage Original, Nur Daten), `.g4/base.json`, Start-Screen mit Assistent und Drag-and-drop, Brückenbefehle, Hinweise zu Recht und Speicherplatz.
- Gate:
  - gated `extract_then_open` auf allen drei ROMs (`G4_TEST_ROM` wird zu einer Liste);
  - SHA-1 erkannt, DP-ROM abgelehnt (synthetischer Kopf genügt), Ziel nicht leer abgelehnt;
  - Browser-Zeilen über die Test-Umgebung (Knopf, Schritte, Fortschritt, Öffnen);
  - das frisch angelegte Projekt baut **byte-identisch zur Eingabe-ROM** (gated: `extract → build → sha1 == rom.sha1`; das ist neu und stark);
  - Hand: DSPRE öffnet den Ordner.

**3e.2 „Leere Region mit Startstadt“**
- Lieferung: `templates.yaml` je Familie, `templates::apply`, Start-Script-Vorlage (rotom), Assistent „Welt leeren…“.
- Gate:
  - auf Kopien von Platin DE und HeartGold DE: `run_checks` danach 0 Fehler, `scripts/gate.sh` grün;
  - g4emu-Szenarien `pt-leere-welt.yaml` und `hg-leere-welt.yaml`: neues Spiel ohne Spielstand (Intro per Skript, das gibt es schon für die HGSS-Spielstände), Spieler steht in der Startstadt, Start-Script gibt den Starter, Pokédex-Flag gesetzt, der Weg aus der Stadt ist gesperrt, nach einer verlorenen Begegnung wacht man zu Hause auf;
  - Gegenprobe Original: dieselbe Kopie ohne Vorlage hat Figuren.

**(später) L2 „Eigener Startort, Intro überspringen“**: nur nach Spike, eigener Patch mit Erkennung und Reparatur wie Dynamic Headers (`dynamic_headers.rs` ist das Vorbild: Metadaten je Spielcode, `plan`, `apply`, `is_applied`, `repair`, Liste `verified`).

**Platz in der Roadmap:** **Phase 3e, vor Phase 4.** Begründung: Der Kernel ist fertig, der Aufwand ist klein, und der Nutzen für neue Nutzer ist groß (heute braucht man DSPRE, um überhaupt ein Projekt zu bekommen). Phase 4 übernimmt danach `.g4/base.json` (Basis-ROM-Prüfung und xdelta) und das Rezept-Format mit symbolischen IDs. `create_project` muss so geschnitten sein, dass Phase 4 nur das Ausgabeformat tauscht (DSPRE-Layout oder Textprojekt).

---

## Teil B: MCP-Server als Produktbestandteil

### B.1 Befund zur heutigen Brücke

- `studio-bridge` ist ein **eigener Prozess mit eigenem `AppState`** (`bridge/http.rs:57-66`: `state: AppState::default()`). Er ist nicht mit der Tauri-App verbunden. Im Browser-Modus teilen sich die Web-Oberfläche und `call.sh` diese eine Sitzung (`platform/transport.ts` → `/__bridge/invoke/<cmd>`). Ein „Agent und Mensch in derselben Sitzung“ gibt es also schon, nur im Dev-Modus.
- `dispatch(state, roots, name, args)` ist eine handgeschriebene Tabelle über die `*_impl`-Funktionen (`dispatch.rs:185ff`). Sie ist **generisch über `&AppState`** und damit 1:1 in der Tauri-App wiederverwendbar (dort `.manage(AppState::default())`, `lib.rs:35`). Heute ist sie hinter `#[cfg(feature = "dev-bridge")]` (`lib.rs:1-2`).
- Schutz heute (`http.rs`):
  - nur `127.0.0.1`;
  - Host-Prüfung (403), Token `X-G4-Bridge-Token` (401), `Content-Type: application/json` (415), Origin (403), Body höchstens 64 MiB;
  - Wurzeln `--allow-root`, Originale `_DSPRE_contents` und `_saves` gesperrt (`roots.rs`).
  Das passt gut zu den MCP-Pflichten (Origin prüfen, an 127.0.0.1 binden, Authentifizierung).
- Die Oberfläche bekommt **keine Push-Meldungen** vom Backend. Tauri sendet nur `confirm-quit` (`lib.rs:142, 157`). Neu laden geschieht über den eigenen `editBus` (`bus/editBus.ts`) und `reloadPlan` nach Undo und Redo aus den zurückgegebenen `AssetId`s (`reload.ts`). Das ist der Anschluss, den ein Agent in der laufenden App braucht.
- Verlauf: `Store` hat Changesets mit `label`, `Store::transaction` als einen Schritt über mehrere Assets und `change_generation` je Asset (`store.rs:416, 712-717, 983-1064`). Der Befehl `history` liefert die Labels für das Scharnier.
- Die Harness-Spec hatte sich 2026-10-22 bewusst **gegen** einen HTTP-Thread in der laufenden Tauri-App entschieden (`2026-10-22-agent-test-harness-design.md` §1): Das Ziel war Testen ohne Eingriff in die App des Nutzers. Für ein Produkt-Feature dreht sich das Ziel um, und die Entscheidung muss neu fallen.

### B.2 MCP und Rust-SDK (Stand 2026-10-25)

- Spezifikation: aktuell Revision **2026-07-28** (davor 2025-11-25 und 2025-06-18). Transporte sind stdio und Streamable HTTP. Die Revision 2026-07-28 entfernt den GET-Stream und die Protokoll-Sitzungen aus Streamable HTTP. Server **müssen** `Origin` prüfen (DNS-Rebinding) und **sollen** lokal an 127.0.0.1 binden und authentifizieren. Bausteine sind Tools (JSON-Schema für Ein- und Ausgabe), Ressourcen und Prompts.
- `cargo search`/`cargo info` ergeben:
  - `rmcp` 3.7.0, das offizielle SDK `modelcontextprotocol/rust-sdk`: Apache-2.0, MSRV 1.88, tokio, Makros, `schemars`, Features `transport-io` (stdio) und `transport-streamable-http-server` (tower/hyper);
  - `rust-mcp-sdk` 2.0.0 (Drittanbieter);
  - `mcp-sdk` 0.0.3 (veraltet).
  Empfehlung: **`rmcp`**. Apache-2.0 verträgt sich mit MIT. Das Lizenz-Audit (`cargo deny`, `ROADMAP.md:275`) hat im Repo noch keine `deny.toml`, das gehört dazu. tokio liegt über Tauri schon im Lockfile (1.54), `schemars` 0.8, 0.9 und 1.2 ebenfalls.
- Vorbild OpenPencil (`open-pencil`, MIT):
  - Paket `@open-pencil/mcp` mit stdio (`openpencil-mcp`, Claude Code: `claude mcp add … -- openpencil-mcp`) und HTTP (`127.0.0.1:7600/mcp`, unter macOS und Linux bevorzugt ein privater Unix-Socket, `PORT=0` schaltet TCP ab);
  - Dateizugriffe begrenzt über `OPENPENCIL_MCP_ROOT`;
  - das CLI wirkt per RPC auf die laufende App („live canvas“).
  Das Muster ist also: **ein Werkzeugsatz, zwei Wirte (laufende App oder kopflos), stdio als Einstieg**.

### B.3 Varianten

| | (1) Sidecar-Prozess spricht die Brücke | (2) MCP in der laufenden App | (3) Kopflos (`g4k`/`studio mcp`) |
|---|---|---|---|
| Sitzung | eigene oder die der Brücke, **nicht** die der App | **dieselbe wie der Nutzer**, gemeinsamer Undo | eigene, kein UI |
| Beobachtbar | nein | ja (Statusleiste, Verlauf, Szene lädt neu) | nein (Log) |
| Aufwand | klein, aber zwei Protokolle hintereinander (MCP → HTTP-Brücke) | mittel: Push-Events, Freigabe-UI, Herkunft im Verlauf | klein bis mittel |
| Risiko | Doppelpflege, die Brücke wird Produkt | Gleichzeitigkeit Mensch und Agent, Sicherheit im laufenden Prozess | `g4k` liegt im Kernel, die Befehle (`commands/*`, `AppState`) im App-Crate: `g4k` kann sie nicht ohne Umbau nutzen |
| Nutzen | gering | **hoch: genau der Wunsch** | CI, Szenarien, Massenänderungen |

Zu (3): `g4k` (`crates/g4-kernel/src/bin/g4k.rs`) sieht nur den Kernel. Der Werkzeugsatz liegt aber über `commands::*_impl` im Crate `studio`. Kopflos geht daher über **dasselbe `studio`-Binary mit Unterbefehl** (`studio mcp --project <dir>`): Vor `tauri::Builder` wird argv geprüft, ohne Fenster. Ein zweites Sidecar im Bundle ist nicht nötig (sonst kämen rund 35 MB dazu, gemessen am Debug-`g4k`). Ein Umzug der Befehle in ein eigenes Crate `g4-commands` wäre sauberer und passt zu Phase 5. Für den Anfang reicht das nicht als Bedingung.

### B.4 Empfehlung (recommend): Hybrid aus (2) und (3) über eine gemeinsame Werkzeugschicht

```
Claude Code / anderer MCP-Client
   │ stdio                         │ Streamable HTTP (127.0.0.1, Bearer-Token)
   ▼                               ▼
studio mcp  ──(Proxy, falls App läuft: Entdeckungsdatei)──►  Gen4 Studio (Tauri)
   │ (sonst kopflos: eigener AppState, --project)              │ AppState (Sitzung des Nutzers)
   ▼                                                           ▼
            mcp::tools  (kuratierte Tools + g4_call + g4_batch, Schemas, Ressourcen)
                                   │
            commands::dispatch  (heute bridge/dispatch.rs, entfeaturet; Roots → Policy)
                                   │
            commands::*_impl  →  g4-kernel (Store, Changesets, Undo, Checks, Build)
```

- **`commands::dispatch` aus dem Feature lösen**: Die Tabelle wandert nach `commands/dispatch.rs`. `bridge::roots::Roots` wird zur Schnittstelle `Policy`. Zwei Ausprägungen:
  - `DevPolicy`: heutiges Verhalten mit `_testkopien`, Suffix-Sperre und `_saves`;
  - `UserPolicy`: nur das offene Projekt, Build-Ziele unter `<Projekt>/build/` oder vom Nutzer freigegebene Ordner, Import nur aus Ordnern, die der Nutzer im Dialog freigegeben hat, `open_project` nur kopflos.
- **In der App**: Ein Schalter in Einstellungen › Agenten („Lokale Agenten zulassen“, Standard aus) startet einen Streamable-HTTP-Server auf `127.0.0.1:<frei>`, auf einem tokio-Task des Tauri-Runtimes. Token zufällig je Start. Eine **Entdeckungsdatei** im App-Datenordner (`~/Library/Application Support/<id>/mcp.json`, Rechte 0600) enthält Port, Token und PID. Prüfungen: Host, Origin (ohne Browser-Origin), Token. Unter Unix zusätzlich ein Socket, wie bei OpenPencil.
- **`studio mcp`** (stdio) liest die Entdeckungsdatei. Läuft die App und ist die Freigabe an, reicht es stdio an HTTP durch, und der Agent arbeitet **im Projekt des Nutzers**. Sonst arbeitet es kopflos mit `--project <dir>` (Pflicht) und `UserPolicy`. Einrichtung für den Nutzer in einer Zeile: `claude mcp add gen4-studio -- "/Applications/Gen4 Studio.app/Contents/MacOS/studio" mcp`. Die App zeigt diese Zeile mit Kopierknopf.
- **Freigabe je Projekt**: „Agenten dürfen dieses Projekt: nicht / lesen / lesen und bearbeiten“. Gespeichert in den App-Einstellungen, nach kanonischem Pfad und `.g4/base.json`-ID, **nicht im Projektordner**, denn ein fremdes Projekt darf sich nicht selbst freischalten. Ohne Freigabe antwortet jeder Tool-Aufruf mit einem klaren Fehler. Optional fragt die App beim ersten Verbinden nach („Claude Code möchte … bearbeiten. Zulassen?“).
- **Bestätigungspflicht** für Schritte, die nicht rückgängig gehen oder nach außen wirken:
  - `apply_header_patch`, `repair_header_patch`;
  - `create_trainer` und Script anlegen (beide als „nicht rückgängig“ markiert);
  - `build_rom` an einen Pfad außerhalb von `build/`, `run_rom`, `accept_baseline`.
  In der App erscheint ein Toast mit „Zulassen/Ablehnen“. Kopflos gilt ein Flag `--allow-irreversible`.
- **Beobachtbarkeit**:
  - Statusleiste „Agent verbunden: Claude Code“ (Name aus `initialize.clientInfo`) mit Menü „Trennen“ und „Nur lesen“;
  - im Verlauf (Scharnier) bekommen Agenten-Schritte ein Symbol und den Client-Namen. Dafür bekommt `Store::Change` neben `label` ein Feld `origin: User | Agent { client }`, und `history` liefert es mit;
  - nach jedem schreibenden Agenten-Aufruf sendet das Backend `emit("agent-changed", { assets, command })`. Die Oberfläche fährt denselben `reloadPlan` wie nach Undo und Redo (`reload.ts`). Für Befehle, die keine `AssetId`s liefern, ermittelt der Store die geänderten Assets über `change_generation` vor und nach dem Aufruf;
  - ⌘Z des Nutzers nimmt auch Agenten-Schritte zurück. Ein `g4_batch` ist **ein** Schritt („Agent: 12 Änderungen“) über `Store::transaction`.
- **Gleichzeitigkeit**: Der Lock um die Sitzung (`AppState::session: Mutex`) gilt je Befehl, Build und Prüfungen geben ihn wie heute frei. Ein Editor mit ungesichertem Formular sieht beim `agent-changed` einen veralteten Stand. Dann greift die vorhandene Stale-Logik je Asset („reload_asset/accept_disk“), die das Formular sichtbar machen muss. `open_project` und das Schließen gibt es in der App für Agenten nicht.

### B.5 Werkzeugschnitt

91 Befehle 1:1 sind zu viele. Sie sind zu nah an der UI: camelCase-DTOs, `lsp_*`, `force_quit`, GLB-Bytes, Getter für einzelne Szenenbausteine. Jeder MCP-Client lädt alle Tool-Schemas in den Kontext, und nicht jeder Client hat eine Tool-Suche wie Claude Code. Empfehlung:

- **Kuratiert, rund 30 Tools**, englische Namen und Beschreibungen. Modelle arbeiten damit am verlässlichsten, das Ökosystem ist englisch. Die Ergebnistexte und Fehlermeldungen bleiben deutsch wie im Kernel. Später sind die Beschreibungen per Locale wählbar. Die Gruppen:
  - Projekt: `project_info`, `save`, `undo`, `redo`, `history`;
  - Welt: `list_places`, `get_header`, `set_header`, `get_matrix`, `set_matrix_cell`, `create_area`, `create_room`, `extend_area`, `list_warps`, `get_events`, `add_event`, `update_event`, `delete_event`;
  - Texte: `get_text_archive`, `edit_text`, `search_text`;
  - Scripts: `scripts_status`, `read_script`, `save_script`, `search_scripts`;
  - Daten: `data_list`, `data_get`, `data_edit`, `create_trainer`, `refs`;
  - Prüfen: `run_checks`, `plan_fix`, `apply_fix`;
  - Bauen: `build_rom`.
  Jedes Tool bekommt ein Eingabe- und Ausgabeschema. Annotationen setzen `readOnlyHint` und `destructiveHint`, damit Clients selbst nachfragen.
- **`g4_batch`**: eine Liste von Aufrufen als eine Transaktion. Das ist das MCP-Gegenstück zu `bridge/batch.rs`.
- **`g4_call` und `g4_commands`** (Experten- und Testmodus, Standard aus): der rohe Dispatch über alle zulässigen Befehle. Damit ist die bestehende Test-Umgebung ohne Lücke abgedeckt.
- **Ressourcen**: `g4://project`, `g4://checks`, `g4://glossary` (Begriffe aus dem Welt-Explorer), `g4://docs/formats/{name}` (die `docs/formats/*.md` sind eigene Texte, keine ROM-Daten), `g4://text/{archive}`, `g4://script/{file}`. Später `g4://render/{header}.png` (Kartenbild aus dem Raster) für visuelle Kontrolle.
- **Prompts**: „Neue Route anlegen“, „NPC mit Item“, „Trainer platzieren“. Die Rezepte gibt es in `docs/agent-gate.md` § Rezepte schon in Prosa.
- **Schemas**: `rmcp` erzeugt sie mit `schemars`. Fremdtypen wie `uxie::MapHeader` (`g4_kernel::store::MapHeader`) haben kein `JsonSchema`. Für die kuratierten Tools braucht es eigene DTOs oder handgeschriebene Schemas. Das ist der größte Einzelposten an Fleißarbeit.

### B.6 Sicherheit (Zusammenfassung)

- Nur Loopback, unter Unix lieber ein Socket mit 0600. Keine Bindung an 0.0.0.0, nicht einstellbar.
- Origin- und Host-Prüfung (Pflicht laut MCP), Bearer-Token je App-Start, Entdeckungsdatei 0600, Vergleich in konstanter Zeit (gibt es schon: `same_secret`, `http.rs`).
- Standard **aus**. Freigabe je Projekt in den App-Einstellungen, Stufe „nur lesen“, ein „Trennen“ jederzeit, Anzeige in der Statusleiste.
- Pfade: nur das offene Projekt. Import und Export nur aus Ordnern, die der Nutzer freigegeben hat. Nie Pfade aus Tool-Argumenten außerhalb dieser Wurzeln (`canonicalize` wie in `roots.rs`, Symlinks aufgelöst).
- Bestätigung für Unumkehrbares (B.4), dazu ein Größenlimit je Aufruf (heute 64 MiB) und ein Rate-Limit.
- Prompt-Injection: Texte, Scripts und Namen aus fremden Hacks sind für den Agenten Daten. Die Tool-Beschreibungen sagen das. Ausgaben von Textarchiven werden als Daten gekennzeichnet (`structuredContent`).
- Kein Netzzugriff durch den Server, keine Shell-Ausführung, `run_rom` nur nach Bestätigung.

### B.7 Tests

- Rust: Integrationstests mit dem `rmcp`-Client gegen den Server im selben Prozess:
  - `initialize`, `tools/list` (Anzahl und Schemas gültig), Aufruf, Batch als ein Undo-Schritt;
  - Freigabe aus heißt Fehler, „nur lesen“ lehnt schreibende Tools ab;
  - Origin, Host und Token wie die heutigen Brückentests.
- Synthetisches Projekt (`test_support::synthetic_project`) für alles Kopflose, gated gegen die drei Testprojekte für `create_area` per MCP mit anschließendem Bauen.
- **Die Agenten-Test-Umgebung wird Kunde des MCP-Servers**: Die Brücke bekommt neben `/invoke` einen Endpunkt `/mcp` auf **demselben** `AppState`. Dann arbeiten im Browser-Modus der Agent über MCP und die Web-Oberfläche in einer Sitzung, und Statusleiste, Verlauf und Neu-Laden lassen sich über die Test-Umgebung abnehmen, ohne natives Fenster. Die Szenarien (`g4emu scenario`) können ihre Brücken-Batches schrittweise auf `studio mcp --project` umstellen. `studio-bridge batch` bleibt, bis alle umgestellt sind.
- Gate-Skript: neuer Schritt `mcp` (kopfloser Lauf auf dem synthetischen Projekt), dazu die Prüfung, dass `g4_call` im Release standardmäßig aus ist.

### B.8 Teilphasen und Gates

**M1 „Werkzeugschicht und kopflos“**
- Lieferung: `dispatch` aus dem Feature lösen, `Policy` (Dev/User), `mcp::tools` mit 10 lesenden Tools und Ressourcen, `studio mcp --project` (stdio, `rmcp`), Lizenz-Audit.
- Gate: Claude Code verbindet sich per `claude mcp add`, liest Header, Prüfungen und Texte einer Testkopie; die Rust-Tests sind grün; `gate.sh` grün.

**M2 „Schreiben kopflos und Batch“**
- Lieferung: die schreibenden kuratierten Tools, `g4_batch`, Bestätigungs-Flags, Annotationen, Szenario `pt-mcp-route.yaml` (Route per MCP anlegen, bauen, mit g4emu begehen).
- Gate: das Szenario ist grün auf Platin DE und HeartGold DE, ein Batch ist ein Undo-Schritt.

**M3 „In der laufenden App“**
- Lieferung: HTTP-Server in der Tauri-App, Entdeckungsdatei, Proxy in `studio mcp`, Einstellungen › Agenten mit Freigabe je Projekt, Statusleiste, Herkunft im Verlauf, `agent-changed` mit `reloadPlan`, Bestätigungs-Toasts.
- Gate: über `/mcp` der Brücke im Browser-Modus legt der Agent ein Gebiet an; die Szene zeigt es ohne Neuladen, der Verlauf markiert den Schritt, ⌘Z nimmt ihn zurück, „Nur lesen“ sperrt. Per Hand in der gepackten App: Claude Code und App arbeiten im selben Projekt.

**M4 „Politur“**
- Lieferung: Prompts, `g4://render`, Beschreibungen per Locale, Unix-Socket, Windows (Named Pipe oder nur TCP), Doku „Mit Agenten arbeiten“.

### B.9 Risiken

- **Spezifikation in Bewegung** (drei Revisionen in gut einem Jahr, 2026-07-28 ohne Sitzungen): `rmcp` folgt dem, ein eigener Nachbau über `tiny_http` würde veralten. Die Version von `rmcp` wird gepinnt, ein Wechsel bekommt einen eigenen Test.
- **Async gegen sync:** `rmcp` braucht tokio, die Befehle sind synchron mit Mutex. Lange Befehle laufen in `spawn_blocking`, wie `build_rom` heute.
- **Schemas für Fremdtypen** (uxie) brauchen Aufwand und sind fehleranfällig. Gegenmittel: die kuratierte Menge klein halten.
- **Mensch und Agent gleichzeitig:** veraltete Formulare, ein Agent überschreibt eine laufende Eingabe. Gegenmittel: Stale-Anzeige und „Nur lesen“ als Standardstufe.
- **Sicherheit im Produkt:** Jeder lokale Prozess desselben Nutzers kann die Entdeckungsdatei lesen. Das ist dasselbe Vertrauensmodell wie bei OpenPencil und dem Socket von VS Code. Darum Standard aus, Freigabe je Projekt und Anzeige.
- **Öffentliche API:** Was als MCP-Tool erscheint, ist eine Zusage. Die Tools bekommen eine Version (`g4.v1`), Phase 4 (symbolische IDs) muss die Argumente **zusätzlich** zulassen und darf nichts brechen.
- **Supportlast:** Nutzer-Agenten richten Schaden an. Gegenmittel: Undo, `.g4/base.json`, Projekt in Git (Phase 4).

### B.10 Platz in der Roadmap

- MCP ist eine Spielart der dritten Plugin-Stufe „Out-of-Process-Tools“ (Spec §3 „Plugin-Modell dreistufig“, §4.4). Es braucht den Wasmtime-Host und die JS-Contribution-Registry von Phase 5 **nicht**.
- Umgekehrt wird die kuratierte Werkzeugschicht die Befehls-API, auf der Phase 5 die Contribution-Points `commands` aufbaut. Wer MCP zuerst baut, testet die API an echten Kunden.
- **Empfehlung: eine eigene kleine Phase „3f Agenten-Schnittstelle (MCP)“ nach 3e und vor Phase 4**, mit M1 und M2 als Pflicht, M3 als Gate der Phase und M4 optional. Begründung:
  - das Gate von Phase 4 (Textprojekt, Build-Graph) lässt sich über `studio mcp` kopflos fahren;
  - die Test-Umgebung wird einfacher;
  - die Speicherschicht berührt MCP nicht.
- Alternativ, falls Phase 4 Vorrang hat: M1 und M2 parallel zu Phase 4 und M3 zusammen mit Phase 5. Dann bleibt aber der eigentliche Wunsch (gemeinsames Projekt, sichtbarer Agent) bis Phase 5 liegen.

---

## Gesamtreihenfolge (recommend)

1. **3e.1** Neues Spiel aus ROM (Original, Nur Daten) mit `.g4/base.json`. Gate: Rebuild gleich Eingabe-ROM, Browser-Zeilen, DSPRE per Hand.
2. **3e.2** Leere Region mit Startstadt. Gate: 0 Fehler in `run_checks`, g4emu-Szenarien neues Spiel auf Platin DE und HeartGold DE.
3. **3f** MCP: M1, M2, M3. Gate: Agent legt im Browser-Modus über `/mcp` ein Gebiet an, Oberfläche und Verlauf zeigen es, ⌘Z wirkt.
4. **Phase 4** übernimmt Basis-ROM-SHA-1, Rezepte mit symbolischen IDs und MCP-Argumente mit symbolischen IDs.
5. **Phase 5** formalisiert die Befehls-API, die MCP schon nutzt.

## Quellen außerhalb des Repos

- MCP-Spezifikation, Streamable HTTP und Changelog: https://modelcontextprotocol.io/specification/latest/basic/transports/streamable-http, https://modelcontextprotocol.io/specification/latest/changelog
- rmcp 3.7.0: https://github.com/modelcontextprotocol/rust-sdk (`cargo info rmcp`)
- OpenPencil (MCP stdio/HTTP, Root-Begrenzung, CLI auf der laufenden App): https://github.com/itorr/open-pencil (Spiegel von open-pencil/open-pencil), https://www.linuxlinks.com/openpencil-design-editor/
