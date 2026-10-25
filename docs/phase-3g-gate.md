# Phase 3g: Gate prüfen

Stand: 2026-10-25. Teil A (Gate 3g.1 „Neues Spiel aus ROM“): vom Agenten bestanden, (A1) bis (A4) über Test und Test-Umgebung (Ergebnis unten); beim Nutzer offen sind (A5) DSPRE öffnet den angelegten Ordner und (A6) die gebündelte Tauri-App mit nativem Dateidialog, Drop aus dem Finder und dem Vorschlag `~/Documents/Gen4 Studio`. Teil B (Gate 3g.2 „Leere Region mit Startstadt“) folgt, wenn 3g.2 umgesetzt ist.

Phase 3g.1 bringt den Weg von einer ROM zu einem Projekt, ohne dass ein Projekt offen sein muss: der Kernel-Befehl `create_project` (Lesen, Entpacken, Vorlage anwenden, Prüfen) legt das Projekt atomar an (erst in `<Ordner>/.<Name>.g4-new`, dann umbenannt) und schreibt die Marke `.g4/base.json` (Herkunft `extracted`, SHA-1 der Basis-ROM, Vorlage, CRC und SHA-1 der Secure Area). Ein Neubau mit `g4k build` ist byte-gleich zur Eingabe: `build()` stellt die Secure-Area-CRC (Offset `0x6C`) wieder her und rechnet die Header-CRC (`0x15E`) neu, nur wenn die gebaute Secure Area denselben SHA-1 hat wie die Marke. Dazu `inspect_rom` (Spielcode, Sprache, Freigabe CPUD, IPKD, IPKE; Diamant/Perl gesperrt mit Grund; „Bekannte Basis-ROM“ aus `metadata/base_roms.yaml`), `default_project_parent` und `check_project_dest` (leer oder fehlt, freier Platz: 2,5-fache ROM-Größe). Oberfläche: der Start mit dem Hauptknopf „Neues Spiel aus ROM…“, Drop einer `.nds`-Datei, und der Assistent in einem eigenen kleinen Deck (Schau, Scharnier im Assistentenmodus, Hand) mit den Schritten ROM, Vorlage (Original, Nur Daten; „Leere Region mit Startstadt“ gesperrt bis 3g.2), Name und Ziel, Zusammenfassung mit dem Ablauf, Ergebnis „Öffnen“. Plan: [docs/superpowers/plans/2026-10-25-phase-3g1-neues-spiel-aus-rom.md](superpowers/plans/2026-10-25-phase-3g1-neues-spiel-aus-rom.md); Gutachten: [docs/research/2026-10-25-ausbau/gutachten-c-projektstart-mcp.md](research/2026-10-25-ausbau/gutachten-c-projektstart-mcp.md), Teil A. Die Originale der ROMs bleiben unberührt: alles läuft auf Kopien unter `Roms/_testkopien/3g-gate-a`.

## Teil A: Gate 3g.1 „Neues Spiel aus ROM“

Teil A besteht aus diesen Punkten:

- (A1) Automatik: die gated Tests `project_start_*` (Kernel und Brücke) auf Kopien von Platin DE, HeartGold DE und HeartGold US: Anlegen, Bauen, SHA-1 des Neubaus gleich der Eingabe.
- (A2) `inspect_rom` auf eine Kopie mit Spielcode ADAD (synthetisch, die ersten 0x8000 Byte von Platin DE): abgelehnt mit Grund. Ein nicht leeres Ziel wird abgelehnt und bleibt unverändert.
- (A3) Browser: Start zeigt „Neues Spiel aus ROM…“ primär; Drop eines Pfads; die Schritte ROM, Vorlage, Name und Ziel (ohne Endung, unter `_testkopien/3g-gate-a/`), Zusammenfassung, Anlegen, „Öffnen“ landet in der Welt mit dem neuen Projekt; „Nur Daten“ landet im Bereich Pokémon.
- (A4) Die Brücke lehnt ein Ziel mit `_DSPRE_contents` ab.
- (A5) DSPRE öffnet den angelegten Ordner (mit Endung, aus der App).
- (A6) Tauri-App: nativer Dateidialog mit `.nds`-Filter, Drop aus dem Finder, Vorschlag `~/Documents/Gen4 Studio`.

Wer was prüft: Die Spalte „Wer“ der Ergebnistabelle sagt **Agent** (die Test-Umgebung aus [docs/agent-gate.md](agent-gate.md): Browser-Fenster mit `studio-bridge`, `call.sh`), **Agent + Blick** (der Agent prüft und beschreibt Ort und Zahlen, der Nutzer bestätigt kurz am Bild) oder **Hand** (der Nutzer). Rezept und Befehle: [docs/agent-gate.md](agent-gate.md), Abschnitt „Phase 3g im Browser-Modus“.

Namen in der Oberfläche (Rollen und `data-*` für die Prüfung):

- **Start:** `[data-start-actions]` mit der primären Taste „Neues Spiel aus ROM…“ und „Projekt öffnen“ (⌘O); darunter „Du brauchst deine eigene ROM-Datei (.nds).“ und „Gen4 Studio liefert keine Spieldaten; du nutzt deine eigene ROM.“. Die Fläche trägt `[data-drop=idle|over]`; solange eine Datei darüber liegt, zeigt der Rahmen „ROM hier ablegen“. Die Palette kennt „Neues Spiel aus ROM…“.
- **Assistent ohne Projekt:** `[data-start-deck]` mit `[data-start-schau]` (Vorschau: Titel der ROM auf dem DS-Bildschirm, Spiel, Vorlage, Ziel), dem Scharnier im Assistentenmodus (`[data-wizard-bar]`) und der Hand (`[data-wizard-hand]`); der Dialog `[role=dialog][data-wizard]` heißt „Neues Spiel aus ROM“. Schritte „ROM“, „Vorlage“, „Name und Ziel“, „Zusammenfassung“; solange er offen ist, sind die anderen Bereiche des Atlas gesperrt.
- **Schritt ROM:** Feld „ROM-Datei“, „Datei wählen…“, die Karte der ROM mit Spiel, Sprache, Spielcode, Freigabe (Plakette „Freigegeben“, „Nicht geprüft“, „Gesperrt“ oder „Kein Gen-4-Spiel erkannt“) und Basis („Bekannte Basis-ROM: Platin DE (Testprojekt)“ oder „Unbekannte Fassung: vielleicht schon verändert.“).
- **Schritt Vorlage:** `[role=radiogroup]` mit den Karten `[data-template=original|empty_region|data_only]`; die Karte „Leere Region mit Startstadt“ ist `aria-disabled` und trägt „Kommt mit 3g.2“.
- **Schritt Name und Ziel:** Felder „Name“ und „Ordner“, „Ordner wählen…“, Schalter „Endung _DSPRE_contents (für DSPRE)“, „Neues Projekt“ mit dem vollen Pfad, die Zeile „Platz: … MB frei, gebraucht etwa … MB.“.
- **Zusammenfassung:** der Satz „Legt „…“ aus … mit der Vorlage … an, in …. Die ROM bleibt unverändert.“ und die Liste „Ablauf“ (`[aria-label=Ablauf]`, Zeilen Lesen, Entpacken, Vorlage anwenden, Prüfen mit `[data-phase=open|running|done|failed]`), Taste „Anlegen“.
- **Ergebnis:** „Projekt angelegt“, der Pfad, „Basis-ROM SHA-1 …“, „Öffnen“ und „Schließen“.

Der Ablauf des Anlegens: In der Brücke gibt es keinen Live-Fortschritt; die Antwort von `create_project` trägt `phases: [{phase, ms}]`, die Oberfläche hakt danach alle Phasen ab. In Tauri meldet ein Kanal jede Phase, wenn sie beginnt. Anlegen dauert auf einem Desktop rund eine Sekunde (Platin, Debug-Build); ein Projekt ist etwa 135 MB groß.

### Vorbereitung

1. Kopien anlegen (nie ein Original): `mkdir -p …/Roms/_testkopien/3g-gate-a`, dort `pt.nds` (Platin DE), `hg.nds` (HeartGold DE) und `hgus.nds` (HeartGold US) als Kopien der ROM-Dateien; `dp-kopf.nds` aus den ersten 0x8000 Byte von `pt.nds` (`head -c 32768`), mit dem Spielcode `ADAD` statt `CPUD` an Offset 0xC (4 Byte ersetzt; die Datei ist keine ROM, nur ein Kopf).
2. Werkzeuge: `cargo build --workspace --all-features` (legt `target/debug/studio-bridge` und `target/debug/g4k` an). Aus einem Worktree `G4_ROMS_DIR` absolut setzen.
3. Test-Umgebung wie in [docs/agent-gate.md](agent-gate.md), „Je Lauf“, mit eigenen Ports: `G4_VITE_PORT=1450 G4_BRIDGE_PORT=1451 bash scripts/agent/studio-web.sh …`; die Seite liegt unter `http://127.0.0.1:1450/` (ohne `?project=`, damit sie auf Start beginnt), vor dem Lauf `localStorage.clear()` und neu laden.
4. A1: `G4_TEST_ROM=<Kopie> cargo test --workspace --all-features -- project_start_` je Kopie.

Fakten, die der Ablauf braucht: SHA-1 der drei Kopien (gemessen 25.10.2026): Platin DE (CPUD) `d40f19ac24d67c721a4ab25e1c013154443d75b4`, HeartGold DE (IPKD) `13b6853aa93121069df1d17446d3ecb36872aecf`, HeartGold US (IPKE) `4fcded0e2713dc03929845de631d0932ea2b5a37`. `g4k extract` und `g4k build` allein sind nicht byte-gleich: ohne die Wiederherstellung weichen genau zwei Stellen ab, `0x6C` (Secure-Area-CRC16, ds-rom schreibt ohne Blowfish-Schlüssel `0000`) und `0x15E` (Header-CRC16, hängt davon ab). Die Brücke sperrt jede Pfadkomponente mit `_DSPRE_contents`.

### Ergebnis

Lauf des Agenten (Controller) am 2026-10-25 auf dem Branch `phase-3g-projektstart` (HEAD 83f29cd), Kopien unter `Roms/_testkopien/3g-gate-a`, Browser-Fenster 1440 × 900, Test-Umgebung auf den Ports 1450/1451. Logs: `target/agent/gate-2026-10-25/` (`a1-project-start.log`, `a2-a4-bruecke.log`, `a3-sha1.txt`).

| Punkt | Wer | Datum | Projekt | Ergebnis | Auffälligkeiten |
|---|---|---|---|---|---|
| (A1) `project_start_*` gated | Agent | 2026-10-25 | Kopien von Platin DE, HeartGold DE, HeartGold US | bestanden | je Kopie `project_start_inspects_the_test_rom`, `project_start_create_then_build_matches_the_rom` (SHA-1 des Neubaus gleich der Eingabe) und `bridge::dispatch::tests::project_start_bridge_creates_and_opens` grün. Messung vorher: ohne die CRC-Wiederherstellung weichen nur `0x6C..0x6E` und `0x15E..0x160` ab (Platin: `b43d13f0…` statt `d40f19ac…`) |
| (A2) `inspect_rom` lehnt ADAD ab, nicht leeres Ziel abgelehnt | Agent | 2026-10-25 | `dp-kopf.nds` (synthetisch), Ordner mit `keep.txt` | bestanden | `inspect_rom` → `family` dp, `support` locked, „Diamant/Perl: ohne Testprojekt nicht freigegeben“; `create_project` damit → HTTP 400 mit demselben Satz, kein Ordner angelegt. `check_project_dest` auf den Ordner mit `keep.txt` → `{"state":"not_empty",…,"ok":false}`; `create_project` dorthin → „destination … is not empty“, `keep.txt` unverändert |
| (A3) Browser, Platin DE, Vorlage Original | Agent + Blick | 2026-10-25 | `…/3g-gate-a/Gate Platin` | bestanden | Start: „Neues Spiel aus ROM…“ primär, „Projekt öffnen“ sekundär, darunter die beiden Sätze. Drop von `file://…/3g-gate-a/pt.nds` (synthetisches Drop-Event mit `text/uri-list`): der Assistent öffnet im eigenen Scharnier und springt nach `inspect_rom` zu „2. Vorlage“ (Schau: DS-Bildschirm „POKEMON PL“, Spiel „Platin (Deutsch)“). Vorlage Original, Name „Gate Platin“, Ordner `…/_testkopien/3g-gate-a`, Endung aus (Browser-Vorschlag), „Platz: 12.926 MB frei, gebraucht etwa 336 MB.“; Zusammenfassung mit Satz und Ablauf (Lesen, Entpacken, Vorlage anwenden, Prüfen); „Anlegen“ → „Projekt angelegt“, Pfad, „Basis-ROM SHA-1 d40f19ac…“; „Öffnen“ → Welt mit „Gate Platin“, 593 Header; die Prüfung läuft, danach 0 Fehler, 101 Warnungen, 10 Hinweise in der Statusleiste. Das Projekt mit `g4k build` gebaut: SHA-1 `d40f19ac24d67c721a4ab25e1c013154443d75b4` = Eingabe. Marke `.g4/base.json`: `origin` extracted, `sha1`, `template` original, `secure_area_crc` 38508. Größe rund 135 MB |
| (A3) Browser, HeartGold DE, „Nur Daten“ | Agent + Blick | 2026-10-25 | `…/3g-gate-a/Gate HeartGold Daten` | bestanden | Drop von `hg.nds`, Vorlage „Nur Daten“, Name „Gate HeartGold Daten“; „Öffnen“ landet im Bereich Pokémon (Pokédex). Marke mit `template` data_only |
| (A4) Ziel mit `_DSPRE_contents` abgelehnt | Agent | 2026-10-25 | `…/3g-gate-a/Test_DSPRE_contents` | bestanden | `create_project` → „Originale (_DSPRE_contents) sind gesperrt: … Im Browser-Modus ohne die Endung anlegen.“ `inspect_rom` auf die Original-ROM unter `Roms/` → „Pfad liegt außerhalb der erlaubten Ordner: …“ |
| (A5) DSPRE öffnet den angelegten Ordner | Hand | | | offen | Ordner mit Endung `_DSPRE_contents` aus der App anlegen (Schalter „Endung _DSPRE_contents (für DSPRE)“ an). Hinweis: `config.yaml` hat keinen `padding:`-Block (gepinntes ds-rom); `unpacked/` entsteht erst beim Öffnen einer Art |
| (A6) Tauri-App: Dateidialog, Finder-Drop, Vorschlag | Hand | | | offen | gebündelte App (`pnpm bundle`, siehe [release.md](release.md)): „Datei wählen…“ mit `.nds`-Filter, Drop einer `.nds` aus dem Finder (nur mit einem Mock getestet), Vorschlag `~/Documents/Gen4 Studio` mit dem Schalter „Endung _DSPRE_contents“ an |

Stehen gelassen für den Blick des Nutzers: `Roms/_testkopien/3g-gate-a/Gate Platin` und `…/Gate HeartGold Daten`.

### Bekannte Grenzen

- Im Browser kennt die Seite den Pfad einer abgelegten Datei nicht. Eine Datei ohne Pfad öffnet den Assistenten im Schritt ROM mit dem Hinweis „Im Browser-Modus kennt die Seite den Pfad der Datei nicht. Gib ihn ein.“; ein Pfad kommt nur aus `text/uri-list` (`file://…`) oder `text/plain` (absoluter Pfad auf `.nds`).
- Im Browser-Modus gibt es keinen Live-Fortschritt beim Anlegen; die Phasen werden nach der Antwort abgehakt.
- Die Brücke legt nur unter `_testkopien` oder `G4_IMPORT_ROOT` an und lehnt `_DSPRE_contents` ab; das ist ein Weg für Agenten-Tests, in der App ist der Schalter „Endung _DSPRE_contents“ frei.
- Eine verschlüsselte ROM erkennt `inspect_rom` nicht eigens; `create_project` meldet den Fehler von `extract_rom` („encrypted ROMs are not supported“).
- „Bekannte Basis-ROM“ heißt nur: SHA-1 und Spielcode stehen in `metadata/base_roms.yaml` (Testprojekt). Ein Abgleich mit No-Intro ist nicht gemacht.
- Die Vorlage „Nur Daten“ ändert nichts am Projekt, nur die Marke (`template: "data_only"`) und den Bereich, in dem die App nach „Öffnen“ startet.
- Der freie Platz kommt per `statvfs` nur auf Unix; sonst bleibt `free_bytes` leer und die Zeile nennt nur den Bedarf.

## Teil B: Gate 3g.2 „Leere Region mit Startstadt“

Folgt, wenn 3g.2 umgesetzt ist.
