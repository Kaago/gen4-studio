# Gutachten C: Git minimal (4.4), Einordnung von 3d und Anforderungen an das Projektformat

Stand: 2026-10-26 (für die Gutachten-Runde 2026-10-27). Gutachter: Architektur-Review (nur lesend am Code, mit Messungen). Repo: `/Users/POLTEP/Projekte/lab/editor/gen4-studio`, Branch `main` (`2c79058`).

Gelesen: `docs/ROADMAP.md` (Phase 3c, „Ausbau nach Phase 3“, Phase 4 mit Schnitt 4.0 bis 4.4, „Schon da“, offene Fragen, Phase 5, Phase 6, „Aus dem Design-System, später“), Spec 3c §13, `docs/research/2026-10-25-ausbau/entscheidungen.md`, Gutachten C vom 2026-10-25 (Stil), die Hauptspezifikation §5 und §6, die Spezifikation 4.0 auf dem Branch `phase-4-0-nebenaufgaben` (nur über `git show` gelesen, der Worktree ist unberührt), dazu im Code `crates/g4-kernel/src/{build,start,text,fsutil,world_meta,run,export,dynamic_headers,scripts,store}.rs`, `checks/ignore.rs`, `data/refs.rs`, `metadata/archives.yaml`, `metadata/scrcmd/*.json`, `crates/g4-formats/src/data/record.rs`, `apps/studio/src-tauri/src/commands/{models,buildings}.rs`, `mcp/tools.rs`, `apps/studio/src/technik/TechnikArea.tsx`, `apps/studio/src/app/{useHistory,useProjectSession}.ts`, `scripts/g4emu/g4emu/{paths,cli,steps}.py`.

Messungen (alle Skripte liegen neben diesem Gutachten, sie schreiben nichts ins Repo):

| Skript | Was es misst |
|---|---|
| `gutachten-c-inventar.py` | Dateizahl und Bytes je Ordner; für jeden Ordner unter `unpacked/` die NARC unter `files/` mit byte-gleichen Mitgliedern; ob der Build des Studios sie packt (`archives.yaml`) |
| `gutachten-c-messung-git.sh` | Variante A „alles“ (init, erster Commit, kleine Änderungen, Build, Vorlage „Leere Region“), Variante B „mit Vorlage“ (`.gitignore` aus `gutachten-c-gitignore-dspre.txt` plus die NARCs, die das Studio packt), Klon von B und Build aus dem Klon |
| `gutachten-c-messung-projektschicht.sh` | Größe einer reinen Projektschicht in Git (Stellvertreter für das Textprojekt ohne Basisschicht) |
| `gutachten-c-messung-mtime.sh` | die Laderegel der Texte („neuere Datei gewinnt“) nach `git switch` |

Alle Läufe auf frischen Kopien (`cp -c -R`) unter `Roms/_testkopien/4-gutachten-c-*`; die Originale wurden nur gelesen, die Kopien sind gelöscht. `g4k` ist der Debug-Build von `main` (`target/debug/g4k`), Git 2.54.0 (Apple Git-157), macOS auf APFS. Für die Packgrößen läuft `git repack -a -d` (ein erster Versuch mit `git gc` brach in Git 2.54 mit „could not find pack“ ab; das Skript nimmt deshalb `repack`).

---

## 1. Anlass

Der Schnitt von Phase 4 sieht als 4.4 „Git minimal“ vor: `.gitignore`-Vorlage beim Anlegen, Diff je Entität in der App (gegen die Basis und gegen den letzten Commit), „Projekt in Git“ als Gate (`ROADMAP.md`, Abschnitt Phase 4). Offen sind dabei:

- was Git mit dem heutigen DSPRE-Ordner und mit einem künftigen Textprojekt kostet und was davon generiert ist;
- was rechtlich nicht in ein öffentliches Repo darf;
- welche Git-Bibliothek die App nimmt und ob sie committen soll;
- wo 3d (Erzählen) hingehört; die Roadmap empfiehlt „nach Phase 4“, die Entscheidung fällt mit diesen Gutachten;
- welche Anforderungen 4.0 (App-Symbol, `.g4/preview.png`), Phase 5 (Plugins) und Phase 6 (Weltformat) an das Projektformat stellen, damit Phase 4 nichts verbaut.

---

## 2. Befund: Git auf dem heutigen DSPRE-Ordner

### 2.1 Was im Ordner liegt (Inventar)

| | Platin DE | HeartGold DE | HeartGold US |
|---|---|---|---|
| Ordner (`du`) | 201,7 MiB | 237,7 MiB | 236,8 MiB |
| Dateien im Repo (Variante A) | 18 483 | 20 027 | 20 027 |
| `unpacked/`: Ordner, Dateien, Bytes | 36, 18 004, 44,9 MB | 40, 19 497, 53,9 MB | 40, 19 497, 53,5 MB |
| `files/`: Dateien, Bytes | 340, 100,4 MB | 384, 123,8 MB | 384, 123,3 MB |
| davon NARCs (Magie `NARC`) | 215, 82,3 MB | 308, 104,4 MB | 308, 103,9 MB |
| `arm9/` + `arm9_overlays/` + `arm7/` | 4,4 MB | 4,6 MB | 4,6 MB |
| `unpacked/`-Ordner mit byte-gleicher NARC | 36 von 36 | 40 von 40 | 40 von 40 |
| NARC-Bytes, die `unpacked/` doppelt hält | 45,1 MB | 54,1 MB | 53,6 MB |
| davon NARCs, die der Build des Studios packt | 22 Ordner, 28,2 MB | 23 Ordner, 33,8 MB | 23 Ordner, 33,4 MB |
| Ordner, die DSPRE entpackt, das Studio aber nie packt | 14 (16,9 MB) | 17 (20,3 MB) | 17 (20,2 MB) |

Die 14 bzw. 17 Ordner ohne Studio-Art sind `OWSprites`, `battleBg`, `battleTowerPokemon`, `battleTowerTrainers`, `buildingConfigFiles`, `otherPokemonBattleSprites`, `pokeHeight`, `pokemonBattleSprites`, `pokemonSpriteOffsets`, `synthOverlay`, `tradeData`, `trainerGraphics`, dazu in Platin `encounterExtended` und `pokeHeightForms`, in HGSS `battleObj`, `eggMoves`, `headbutt`, `rockSmash`, `safariZone`.

Was davon generiert ist, hängt am Werkzeug:

| Teil | Für das Studio | Für DSPRE | Folgerung für Git (DSPRE-Layout) |
|---|---|---|---|
| NARCs der 22/23 Studio-Arten in `files/` | **generiert**: `build_prepared` packt jeden vorhandenen `unpacked/<art>`-Ordner und schreibt die NARC (`crates/g4-kernel/src/build.rs:186-206`) | generiert beim Speichern der ROM | ignorierbar (Variante B), mit einer Korrektur in `build.rs` (2.3) |
| NARCs der übrigen 14/17 Ordner | **Quelle**: das Studio packt sie nie | generiert | versioniert lassen; sonst fehlen sie nach einem Klon für ds-rom |
| `unpacked/` | Quelle (der Store schreibt dort) | Quelle | versioniert |
| `expanded/scripts`, `expanded/textArchives/*.json` | Quelle; die Mitglieder in `unpacked/` werden daraus kompiliert bzw. kodiert (`build.rs:43-65`) | Quelle | beides versioniert (Folge: 2.4) |
| `arm9/`, Overlays | Quelle (Header-Tabelle in `arm9.bin`, Patches) | Quelle | versioniert |
| übrige `files/` (Sound, Grafik ohne NARC-Entpacken) | Basis, unverändert | Basis | versioniert (heute), regenerierbar aus der Basis-ROM (Textprojekt, 3.2) |

Gen4 Studio legt selbst in den Ordner (gefunden im Code und in Lauf A4):

| Pfad | Art | Beleg |
|---|---|---|
| `.g4/tmp/` | Temp-Dateien für atomares Schreiben | `fsutil.rs:8-18`, `build.rs:201`, `text.rs:316, 342` |
| `.g4/models/` | GLB-Cache von apicula (Schlüssel aus Tool-Hash und Bytes) | `apps/studio/src-tauri/src/commands/models.rs:83`, `buildings.rs:168`, `export.rs:279-330` |
| `.g4/backup/dynamicHeaders-<Zeit>/` | Schnappschuss vor dem Patch | `dynamic_headers.rs:457-488` |
| `.g4/base.json` | Marke der Basis-ROM (3g) | `start.rs:221-247` |
| `g4-world.json`, `g4-checks.json` | Projektdateien im Wurzelordner | `world_meta.rs:1-18`, `checks/ignore.rs:1-9` |
| `rotom.toml`, `.rotom/command_database` (756/568 KiB), `.rotom/status/compile-state.json` (384/332 KiB), `.rotom/cache/` | rotom-Projekt nach „Scripts übernehmen“ | `scripts.rs:25-30`; Lauf A4 |
| `expanded/scripts/*.rotom` (1125 bzw. 966 Dateien, 5,4 bzw. 5,2 MiB) | Script-Quellen | Lauf A4 |
| `<Wurzel>/out.nds` | Vorschlag des Speicherdialogs im Browser-Modus | `apps/studio/src/app/useProjectSession.ts:282` |
| `<Wurzel>/build/` | Build-Ziel der Agenten (`UserPolicy`) | `apps/studio/src-tauri/src/mcp/tools.rs:2110`, Roadmap 3h.1 |

Emulator-Spielstände legt das Studio nicht ins Projekt: `run.rs:20-49` startet nur den Emulator mit der ROM. Der Emulator legt sie aber dorthin, wo die ROM liegt (melonDS schreibt `<rom>.sav` neben die ROM, Savestates `.ml1` … `.ml8`; DeSmuME schreibt `.dsv` in den eigenen Ordner). Liegt die ROM als `out.nds` oder unter `build/` im Projekt, landen die Spielstände mit dort. `g4emu` schreibt nur in einen gewählten Ausgabeordner (`last.state`, `last.png`, `cli.py:153-165`) und sperrt `_testkopien/_saves` (`paths.py:33-47`).

Widerspruch im Code: `world_meta.rs:3-4` und `checks/ignore.rs:7` nennen `.g4/` „a cache that may be deleted“. Seit 3g liegt dort aber `.g4/base.json`, und 4.0 legt `.g4/preview.png` dazu. Beide dürfen nicht gelöscht werden und gehören in Git (Abschnitt 6).

### 2.2 Messung: erster Commit, kleine Änderungen, Build

| | Platin DE | HeartGold DE | HeartGold US |
|---|---|---|---|
| **A „alles“**: `git add -A` | 8,9 s | 9,3 s | 9,1 s |
| `git commit` | 0,17 s | 0,15 s | 0,15 s |
| `.git` lose (nach dem Commit) | 117,8 MiB | 131,8 MiB | 130,9 MiB |
| `.git` gepackt (`size-pack`) | 63,1 MiB | 77,0 MiB | 76,1 MiB |
| `git repack -a -d` | 5,5 s | 6,0 s | 6,0 s |
| `git status` (warm) | 0,05 s | 0,05 s | 0,05 s |
| A1: eine Permission über `g4k map set-perm` (Store speichert `unpacked/maps/0003`) | 1 Datei, +32 KiB lose, +1 KiB gepackt | 1 Datei, +36 KiB / +0 KiB | 1 Datei, +36 KiB / +0 KiB |
| A2: ein Byte einer Eventdatei | 1 Datei, +32 KiB lose, +0 KiB gepackt | +28 KiB / +1 KiB | +28 KiB / +1 KiB |
| A3: `g4k build` (Debug) | 8,5 s; 2 NARCs geändert, +5,8 MiB lose, +1 KiB gepackt | 22,0 s; 2 NARCs, +7,2 MiB / +1 KiB | 20,2 s; 2 NARCs, +7,2 MiB / +1 KiB |
| A4: `g4k template empty-region` (übernimmt Scripts) | 1136 Dateien, +3,0 MiB lose, +461 KiB gepackt | 977 Dateien, +3,1 MiB / +489 KiB | 977 Dateien, +3,1 MiB / +487 KiB |
| **B „mit Vorlage“**: ignorierte Dateien | 22 NARCs | 23 NARCs | 23 NARCs |
| `.git` gepackt | 53,7 MiB (−15 %) | 65,1 MiB (−16 %) | 64,6 MiB (−15 %) |
| `git status` nach `g4k build` | leer | leer | leer |
| **C**: Klon von B, dann `g4k build` | **scheitert** (`files/fielddata/land_data/land_data.narc: No such file or directory`) | 17,1 s, ROM byte-gleich zu B | 18,2 s, ROM byte-gleich zu B |
| C mit angelegten Elternordnern | 10,4 s, ROM byte-gleich zu B, `git status` leer | – | – |

Lesart:

- **Der erste Commit ist billig** (rund 9 s für `add`), das Repo ist gepackt etwa so groß wie die NARCs allein (63–77 MiB). Spieldaten komprimieren schlecht, weil Texte verschlüsselt und Modelle schon gepackt sind.
- **Eine Änderung kostet lose rund 30 KiB, gepackt fast nichts.** Die 30 KiB sind vor allem die neuen Baum-Objekte großer Ordner (`unpacked/eventFiles` hat 534 Einträge, jeder Commit schreibt diesen Baum neu). Ein Build schreibt zwei NARCs neu (5,8–7,2 MiB lose), die Delta-Kompression macht daraus gepackt etwa 1 KiB.
- **Variante B spart 15–16 %** und hält `git status` nach dem Build leer. Das ist der eigentliche Gewinn: Ohne die Vorlage zeigt jeder Build geänderte NARCs, die niemand prüfen kann.
- **Der Klon aus B baut in Platin nicht**: Git legt keine leeren Ordner an, und `write_if_changed` legt den Elternordner nicht an (`build.rs:116-127`; `rename` nach `files/fielddata/land_data/` scheitert). In HGSS liegen die NARCs (`files/a/0/6/5`) zwischen anderen Dateien, deshalb geht es dort. Mit den Elternordnern baut der Klon byte-gleich. Die Korrektur ist eine Zeile (`create_dir_all(path.parent())`) und gehört nach 4.2 (Build-Graph) oder als Vorab-Fix in 4.4.

### 2.3 Zeilenenden: Git hält 81 bis 99 Spieldaten-Dateien für Text

`git ls-files --eol` auf frischen Kopien (ohne `.gitattributes`):

| | Platin DE | HeartGold DE |
|---|---|---|
| `i/-text` (binär erkannt) | 17 604 | 19 177 |
| `i/lf` (Text mit LF) | 94 | 84 |
| `i/crlf` (Text mit CRLF) | 14 | 6 |
| `i/none` (Text ohne Zeilenende) | 771 | 760 |
| davon Spieldaten unter `unpacked/` und `files/` mit LF oder CRLF | 99 (`pokeHeight` 83, `files/data` 14, `pokeHeightForms` 2) | 81 (`pokeHeight` 75, `files/data` 5, `files/pbr` 1) |

Git für Windows setzt `core.autocrlf=true` als Standard. Damit würden diese Dateien beim Auschecken LF → CRLF und beim Einchecken CRLF → LF umgeschrieben: die ROM ändert sich, ohne dass jemand etwas bearbeitet hat. Gegenmittel ist eine `.gitattributes` mit `* -text` (DSPRE-Layout) bzw. `-text` für alle Binärteile (Textprojekt). Die Vorlage von 4.4 braucht sie zwingend. Die Messläufe oben liefen mit `core.autocrlf=false`.

### 2.4 Die Laderegel der Texte verträgt sich nicht mit `git switch`

Der Kernel entscheidet zwischen `expanded/textArchives/NNNN.json` und dem Mitglied `unpacked/textArchives/NNNN` nach der Änderungszeit: „`json` when it exists and is not older than `member`“ (`text.rs:250-270`). Git schreibt beim Auschecken in Pfad-Reihenfolge, `expanded/` vor `unpacked/`. Das Mitglied ist danach immer ein paar Millisekunden jünger.

Messung (`gutachten-c-messung-mtime.sh` und eine Wiederholung von Hand, Platin DE nach der Vorlage „Leere Region“, Archiv 554):

1. Branch `x2`: nur die JSON geändert („Xzwei“), Commit.
2. `main`: die JSON anders geändert, gebaut (das Mitglied wird neu kodiert), Commit.
3. `git switch x2`: die JSON trägt „Xzwei“, das Mitglied ist das alte. Die Zeitstempel sind JSON `…866.934`, Mitglied `…866.945`.
4. `g4k build`: **die JSON wird aus dem Mitglied überschrieben**, aus „Xzwei“ wird wieder der alte Text. `git status` zeigt `expanded/textArchives/0554.json` und `.rotom/status/compile-state.json` geändert.

Ohne Branch-Wechsel (JSON ändern, bauen) bleibt die Änderung erhalten, und das Mitglied wird kodiert (`encoded=[554]`). Der Fehler entsteht also aus Git plus Zeitstempel-Regel. Git hat den Stand noch, die App verliert ihn aber still. Rotom selbst arbeitet mit Hashes (`CompileState` mit `source_hash`, `output_hash`, `dependency_hashes`, rotom `src/compile_state.rs:11-37`), schreibt aber in `compile-state.json` bei jedem Kompilieren `last_compiled` und erzeugt damit Rauschen in Git. Versioniert werden muss die Datei heute trotzdem, weil sie die `BinaryQuirk`s für den byte-gleichen Rebuild trägt (`scripts.rs:403-445`).

Folge: Solange JSON und Mitglied beide versioniert sind und die Zeit entscheidet, ist Git im DSPRE-Layout riskant. Der Build-Graph (4.2) soll die Zeit-Regel durch Fingerprints ersetzen: Den Hash von JSON und Mitglied beim letzten Kodieren merken; ist nur die JSON neu, gewinnt sie; ist nur das Mitglied neu (DSPRE), gewinnt es; sind beide neu, ist das ein Konflikt mit Rückfrage. Nach einem Klon ist `.g4/build/` leer; dann gewinnt die JSON, mit Hinweis, wenn das dekodierte Mitglied abweicht. Im Textprojekt (4.3) ist `unpacked/` nur noch Zwischenstand und ignoriert. Dann gibt es nur noch eine Quelle.

---

## 3. Befund: Git mit einem Textprojekt

### 3.1 Was sich ändert

Im Textprojekt (eine JSON je Entität, symbolische IDs, `unpacked/` als Build-Zwischenstand) gilt:

- `unpacked/` und alle NARCs in `files/`, die der Build erzeugt, sind generiert und ignoriert. Die Doppelung aus 2.1 (45–54 MB) fällt weg.
- Die strukturierten Daten werden Text. Gemessen sind das 7240 (Platin) bzw. 6811 (HGSS) Mitglieder mit 5,3–5,7 MB: Header-nahe Daten, Events, Matrizen, Begegnungen, Pokémon-Daten, Attacken, Items, Trainer, Scripts, Texte.
- Die Binär-Assets bleiben binär (Karten-NSBMD, Texturen, Gebäudemodelle, Sprites, Symbole). In Platin sind das 7278 Mitglieder mit 39,5 MB. Sie machen den größten Teil des Projekts aus, ob mit oder ohne Textprojekt.
- Der Rest der ROM (Sound, Grafik, die 14/17 Archive ohne Studio-Art, `arm9` ohne Patches): 55–70 MB Basis, die der Nutzer nie anfasst.

### 3.2 Zwei Varianten für die Basisschicht

| Variante | Inhalt des Repos | Gepackt (gemessen bzw. abgeschätzt) | Klon braucht |
|---|---|---|---|
| **T1 Vollprojekt** | Projektschicht plus die ganze Basis als ds-rom-Baum, ohne das Generierte | ≈ Variante B: 54–65 MiB | nichts |
| **T2 Basis aus der ROM** | nur die Projektschicht (Entitäten-JSON, Scripts, Texte, vom Nutzer importierte oder geänderte Binärteile, `.g4/base.json`, Projektdateien). Die Basis entsteht aus der Basis-ROM (SHA-1 aus `.g4/base.json`, Pfad der ROM in den App-Einstellungen wie in 4.1). | Stellvertreter gemessen (`gutachten-c-messung-projektschicht.sh`: rotom-Quellen plus Mitglieder der strukturierten Archive): 7241 bzw. 6812 Dateien, 6,8–7,6 MB roh, **5,2–5,6 MiB gepackt** | die eigene ROM (SHA-1 geprüft) |

Der Stellvertreter ist eine obere Schranke für die Packgröße. Die Textarchive sind dort noch verschlüsselte Binärdateien (4,6 MB in Platin, unkomprimierbar). Als chatot-JSON komprimieren sie mehrfach besser, die Rohgröße steigt dabei (JSON-Schlüssel, Klartext).

**Empfehlung aus Git-Sicht (recommend): T2.** Gründe:

- Das Repo ist rund zehnmal kleiner.
- Diffs und `git log` zeigen nur, was der Nutzer gemacht hat.
- Ohne die eigene ROM ist ein Klon ohnehin nutzlos.
- Die Basis-ROM-Logik baut 4.1 sowieso (SHA-1-Prüfung, Pfad in den Einstellungen, xdelta).
- Rechtlich ist T2 ebenfalls besser (Abschnitt 4).

Ob das Textprojekt alle Entitäten ausschreibt oder nur die geänderten, entscheiden die Gutachten zum Format (A/B). Für Git reicht beides, solange die Basisschicht nicht im Repo liegt. Vorschlag für die Basis: `g4k`/App legt sie unter `.g4/base/` (ignoriert) aus der ROM an, wie `extract_rom` heute (`build.rs:235-260`, Platin in 0,49 s).

---

## 4. Rechtliches: was nicht in ein öffentliches Repo darf

Keine Rechtsberatung; die Einschätzung folgt der Praxis der Szene und der Regel der Hauptspezifikation „Keine ROM-Daten im Repository“ (§6).

- **Ein DSPRE-Ordner ist die ganze ROM in Teilen**: `arm9`, Overlays, alle NARCs, Sound, Grafik. Ein öffentliches Repo damit ist das Verbreiten des Spiels. Das gilt auch für ein privates Repo bei einem Dienst, das später öffentlich gestellt oder geteilt wird.
- **Ein Textprojekt enthält trotzdem Spieldaten**: alle Texte des Spiels (Sprachwerke), dekompilierte Scripts, Datentabellen, Karten- und Sprite-Binärteile, auch `.g4/preview-standard.png` (gerendertes Kartenbild). Auch T2 mit vollständig ausgeschriebenen Entitäten enthält den kompletten Spieltext.
- **Was sich teilen lässt**: der xdelta-Patch gegen die Basis-ROM (4.1, die übliche Form in der Szene), eigene Assets, eigene Scripts ohne Originaltexte. Ein „Differenzprojekt“ (nur Abweichungen von der Basis) käme dem nahe. Ob 4.3 das als Format anbietet, ist eine offene Frage an die Format-Gutachten.

Folgen für die App (recommend):

1. Git **nur lokal oder privat**. Die App richtet kein Remote ein, pusht nie und bietet kein „Auf GitHub veröffentlichen“.
2. Ein **Hinweis beim Einrichten**: „Dein Projekt enthält Daten aus dem Spiel. Lade es nicht öffentlich hoch; zum Teilen gibt es den Patch (Technik › Veröffentlichen).“ Der Satz steht auch als Kopfkommentar in der `.gitignore`-Vorlage.
3. `*.nds`, `*.srl` und die Basisschicht sind immer ignoriert, damit nie eine ganze ROM eingecheckt wird.
4. Keine Telemetrie oder Absturzberichte mit Projektdateien (heute nicht vorhanden; als Regel festhalten).

---

## 5. `.gitignore`- und `.gitattributes`-Vorlagen

Vorschlag: die Vorlagen liegen als Metadaten im Kernel (`crates/g4-kernel/metadata/git/{dspre,text}.gitignore`, `…/gitattributes`). Der Kernel schreibt sie beim Anlegen (3g, Assistent) und mit „Git einrichten“. Eine vorhandene Datei überschreibt er nicht, er ergänzt nur fehlende Zeilen in einem markierten Block.

### 5.1 `.g4/` als Erlaubnisliste

Statt jeden Cache einzeln zu ignorieren: `.g4/` ist lokal, bis auf eine kurze Liste versionierter Dateien. Neue Caches (4.2 Fingerprints, apicula-Modelle, Standard-Vorschaubild aus 4.0, Basisschicht T2) sind damit ohne Änderung der Vorlage ignoriert.

```gitignore
/.g4/*
!/.g4/base.json
!/.g4/preview.png
```

Dazu sollte der Code die Bedeutung von `.g4/` festlegen: Oberste Ebene = Projekt-Metadaten (versioniert); `cache/`, `tmp/`, `build/`, `backup/`, `base/` = lokal. Die Kommentare in `world_meta.rs:3-4` und `checks/ignore.rs:7` werden entsprechend korrigiert. Den GLB-Cache nach `.g4/cache/models` zu verschieben, ist optional: die Erlaubnisliste deckt ihn ab.

### 5.2 DSPRE-Layout heute (Git ohne Textprojekt)

Ausgangspunkt ist `gutachten-c-gitignore-dspre.txt` (gemessen in Variante B), mit der Erlaubnisliste statt der Einzelzeilen:

```gitignore
# Gen4 Studio. Dieses Projekt enthält Daten aus dem Spiel: nur lokal oder privat versionieren, nie öffentlich.
# Zum Teilen: Technik › Veröffentlichen (Patch gegen die Basis-ROM).

# Gebaute ROMs und Patches
*.nds
*.srl
*.xdelta
/build/

# Emulator: Spielstände und Savestates
*.sav
*.dsv
*.ml[0-9]
*.mln
*.state

# Studio: nur base.json und preview.png sind Projekt, der Rest ist lokal
/.g4/*
!/.g4/base.json
!/.g4/preview.png

# rotom: Cache (die Quirks in .rotom/status/compile-state.json bleiben versioniert)
/.rotom/cache/

# NARCs, die der Build des Studios aus unpacked/ packt (je Spielfamilie aus archives.yaml)
/files/fielddata/land_data/land_data.narc
# … (22 Zeilen Platin, 23 HGSS; der Kernel erzeugt sie aus ArchiveMap::kinds())

.DS_Store
Thumbs.db
```

```gitattributes
# Spieldaten sind binär, auch wenn Git sie für Text hält (pokeHeight, files/data)
* -text
*.json text eol=lf
*.rotom text eol=lf
*.yaml text eol=lf
*.toml text eol=lf
*.md text eol=lf
```

Die NARC-Zeilen gelten nur, wenn der Build die Elternordner anlegt (2.2). Ohne die Korrektur in `build.rs` lässt die Vorlage sie weg und nimmt die 15 % Mehrgröße und die geänderten NARCs nach jedem Build in Kauf. Ignoriert sind sie auch nur unter einer Bedingung: Der Ordner `unpacked/<art>` existiert. Lazy entpackte Arten (`project.rs:684-732`) haben sonst nur die NARC als Quelle. Der Kernel schreibt die NARC-Zeilen deshalb erst, wenn alle Arten der Zeilen entpackt sind (`ensure_unpacked` für alle 22/23 Arten beim Einrichten, in DSPRE-Ordnern ohnehin der Fall).

### 5.3 Textprojekt

```gitignore
# Gen4 Studio, Textprojekt. Enthält Daten aus dem Spiel: nur lokal oder privat versionieren.
*.nds
*.srl
*.xdelta
/build/
*.sav
*.dsv
*.ml[0-9]
*.mln
*.state

# Build-Zwischenstände: DSPRE-Ordner, aus dem Textprojekt erzeugt
/unpacked/
/expanded/
/files/
/arm9/
/arm9_overlays/
/arm7/
/banner/
/header.yaml
/header_logo.png
/config.yaml
/path_order.txt
/rotom.toml
/.rotom/

# Studio: Basisschicht (T2), Fingerprints, Caches, Sicherungen
/.g4/*
!/.g4/base.json
!/.g4/preview.png

.DS_Store
Thumbs.db
```

Die Liste der DSPRE-Pfade hängt am Layout, das 4.3 festlegt. Liegt der Zwischenstand gebündelt unter einem Ordner (Vorschlag: `.g4/build/rom/` oder ein sichtbarer `rom/`), schrumpft der Block auf eine Zeile. **Empfehlung: den Zwischenstand in einem Ordner bündeln.** Dann bleibt der Projektordner lesbar, und DSPRE öffnet diesen Ordner, wie die Roadmap es für „DSPRE bleibt parallel nutzbar“ vorsieht. Die `.gitattributes` des Textprojekts setzt `* -text`, dazu `eol=lf` für JSON, rotom, YAML und Markdown.

Versioniert bleiben in beiden Fällen: `.g4/base.json`, `.g4/preview.png`, `g4-world.json`, `g4-checks.json`, die `.gitignore` und die `.gitattributes`. Im Textprojekt kommen die Entitäten, das Projekt-Manifest mit Schema-Version und die rotom-Quellen dazu; die Quirks wandern aus `compile-state.json` in die Metadaten des Scripts. Im DSPRE-Layout kommen `rotom.toml`, `.rotom/command_database` und `.rotom/status/compile-state.json` dazu.

---

## 6. Diff je Entität in der App

### 6.1 Zwei Vergleiche, ein Mechanismus

| Vergleich gegen | Woher die alten Bytes kommen |
|---|---|
| **Basis** | das Mitglied aus der Basis-ROM: die ROM mit ds-rom lesen, die NARC in den Speicher, das Mitglied herausnehmen. Ein ganzes Entpacken ist nicht nötig. Der Pfad der ROM steht in den App-Einstellungen, geprüft gegen `.g4/base.json` (4.1). Mit T2 liegt die Basisschicht ohnehin unter `.g4/base/`. |
| **letzter Commit** | der Blob unter `HEAD` für den Pfad der Entität (DSPRE: das Mitglied unter `unpacked/`, Textprojekt: die JSON) |
| **gespeichert** (heute schon da) | `saved_bytes` im Store (`store.rs:399`); `Store::dirty()` (`store.rs:2400`) nennt die geänderten Assets |

Der semantische Diff ist in allen drei Fällen derselbe: alte und neue Bytes → Dekoder des Kernels → Feldwerte mit Namen → Vergleich. Was die Dekoder schon hergeben:

- **Daten-Datensätze** (Pokémon, Attacken, Items, Entwicklungen): `RawRecord::FIELDS` mit `FieldSpan { name, offset, len, kind }` (`crates/g4-formats/src/data/record.rs:22-40`). Daraus ergibt sich der Diff als Tabelle „Feld: alt → neu“ ohne neuen Code je Art, nur mit Anzeige-Namen aus dem Inspector (`Personal::FIELDS` ist schon Englisch benannt).
- **Trainer**: das Trainer-Dokument (`trainer_doc.rs`) als JSON, Partei je Platz.
- **Texte**: je Nachrichten-ID (chatot-JSON, `text.rs:22-38`): hinzugefügt, entfernt, geändert, mit Wort-Diff.
- **Scripts**: Text-Diff der rotom-Quelle. Für die Basis muss das Basis-Mitglied einmal dekompiliert werden (rotom kann das je Datei).
- **Header**: uxie `MapHeader` hat ein JSON-Format (uxie 0.9.7 `src/map_header/json_format.rs`).
- **Events**: die DTOs der App (`get_events`) je Event-Liste, nach Index.
- **Karten** (binär): je Teil (Modell, Permissions, Gebäude, BDHC) „geändert ja/nein“ mit Hash (wie `g4k check-data` mit `model_sha1`, `per_sha1`). Die Permissions zusätzlich als Zellen-Diff, die Gebäude als Liste.
- **Referenzen**: mit symbolischen IDs (4.3) zeigt der Diff `TRAINER_ROCKET_1` statt 312.

Im Textprojekt ist der Diff gegen `HEAD` per Konstruktion ein JSON-Diff derselben Datei. Darum sollte die App von Anfang an auf dem JSON-DTO der Entität vergleichen, nicht auf Bytes, und das DTO soll dasselbe sein, das 4.3 schreibt. Dann ist der Diff in 4.4 schon „Textprojekt-fertig“, und Plugins (Phase 5) bekommen ihn über ihr Schema geschenkt.

### 6.2 Wo er in der App sitzt

- **Technik › Änderungen** (neuer Eintrag in `ENTRIES`, `technik/TechnikArea.tsx:62-67`, neben „Prüfen und Bauen“, „Patches“, „Emulator“, „Werkzeuge“). Links die Liste der geänderten Entitäten nach Art, mit einem Umschalter „gegen Basis / gegen letzten Commit“ (der zweite ist nur aktiv, wenn ein Repo da ist). Rechts die Feldtabelle „alt → neu“ in Nutzerworten, Knöpfe „Öffnen“ (springt in den Bereich) und „Auf Original zurücksetzen“ (4.1, ein Undo-Schritt).
- Im **Inspector-Kopf** später eine Plakette „geändert“ mit Tooltip auf den Diff (nicht in 4.4).
- Der **Verlauf** im Scharnier (`app/useHistory.ts`, `Store::history_entries`, `store.rs:2367`) bleibt die Undo-Liste der Sitzung. Er wird nicht mit Git vermischt: Undo-Schritte sind feiner als Commits, und Agenten-Schritte (`Origin`, `store.rs:416`) gehören dorthin.
- **MCP**: Ein lesendes Werkzeug `diff_entity { kind, id, against: "base" | "head" }` und eine Ressource `g4://changes` geben Agenten dieselbe Sicht. Das passt zu „Gate kopflos“.

### 6.3 Git-Bibliothek

Gemessen mit zwei Prototypen unter `scratchpad/proto-c/` (Status gegen `HEAD` und Blob eines Pfads aus `HEAD`), Release-Build, auf den Testrepos mit 18 483 und 20 027 Dateien:

| | `git2` 0.21.0 (libgit2 1.9.7) | `gix` 0.90.0 | `git`-CLI 2.54 |
|---|---|---|---|
| Lizenz des Crates | MIT OR Apache-2.0 | MIT OR Apache-2.0 | GPL-2.0 (eigener Prozess) |
| mitgebauter C-Code | libgit2: **GPL-2.0 mit Linking Exception**, dazu gebündelt zlib, PCRE2 (BSD), winhttp-Definitionen (LGPL, nur Windows), SHA1DC (MIT) | keiner (reines Rust; zlib-rs) | – |
| Abhängigkeiten | 15 Crates | 137 Crates, alle frei: MIT/Apache (98), Unlicense/MIT, Zlib, BSD-3-Clause, Unicode-3.0, CC0; eine mit Wahl „MIT OR Apache-2.0 OR LGPL-2.1-or-later“ | – |
| Größe des Prototyps | 1,3 MB | 5,1 MB | – (muss installiert sein) |
| Status, kalt / warm | 180–200 ms / 61–65 ms | 62–82 ms / 54–55 ms | 66 ms (Status + `show`) |
| Blob aus `HEAD` | < 1 ms | < 1 ms | im selben Aufruf |
| Schreiben (init, add, commit) | ausgereift | init ja; Index und Commit aus dem Arbeitsbaum noch unvollständig | vollständig |

Zur Lizenz: Die Linking Exception erlaubt, libgit2 in eine MIT-App zu linken. `cargo deny` sieht aber nur die Crate-Lizenz von `libgit2-sys` (MIT/Apache) und nicht die des mitgebauten C-Codes. Das Audit wäre also grün und trotzdem unvollständig; die Lizenz müsste von Hand in den Release-Hinweisen stehen. Das Repo hat **noch keine `deny.toml`** (gesucht im ganzen Repo; schon im Gutachten C vom 2026-10-25 und im Plan 3h.1 als offen vermerkt), und `cargo-deny` ist nicht installiert. Das Audit lief deshalb über `cargo metadata` des Prototyps.

**Empfehlung (recommend): `gix` zum Lesen im Prozess, die `git`-CLI für die wenigen schreibenden Schritte, wenn sie installiert ist.**

- Lesen (Repo erkennen, Status, Blob aus `HEAD`, letzter Commit) braucht der Diff bei jedem Öffnen von „Änderungen“. Das soll ohne installiertes Git gehen (Windows hat keins, macOS nur mit den Command Line Tools) und ohne C-Build.
- Schreiben (init, erster Commit) ist selten. Die CLI ist dort die verlässlichste Wahl, und der Nutzer hat für Commits ohnehin sein eigenes Werkzeug. Ohne CLI zeigt die App „Git ist nicht installiert“ mit Link, und die Vorlagen schreibt der Kernel trotzdem.
- `gix` braucht `features = ["sha1", "status", "index", "revision", "blob-diff"]`. Ohne `sha1` baut `gix-hash` nicht (gemessen). Die 5,1 MB fallen neben der App (Tauri, three.js) kaum ins Gewicht.
- Vor der Aufnahme: `deny.toml` anlegen (Erlaubnisliste MIT, Apache-2.0, BSD-3-Clause, Zlib, Unlicense, Unicode-3.0, CC0-1.0, ISC), `cargo deny check licenses` ins Gate.

Ein Detail für `gix::discover`: Liegt das Projekt in einem größeren Repo (etwa einem Dotfile-Repo im Home-Ordner), findet die Suche dieses Repo. Die App nimmt ein Repo nur, wenn dessen Arbeitsbaum das Projekt enthält, und rechnet die Pfade relativ. Sie bietet „Git einrichten“ nur an, wenn kein Repo das Projekt enthält.

### 6.4 Commit in der App?

**Empfehlung (recommend): minimal, ohne allgemeine Commit-Funktion in 4.4.**

- **„Git einrichten…“** (Technik › Änderungen, Palette, Assistent „Neues Spiel aus ROM“ als Häkchen, Standard aus): schreibt `.gitignore` und `.gitattributes`, macht `git init` und einen ersten Commit „Ausgangsstand“, mit dem Hinweis aus Abschnitt 4. Über MCP als unumkehrbarer Befehl (`git_init`, Rückfrage wie bei den anderen).
- **Lesen**: Status je Entität, Diff gegen `HEAD`, in der Statusleiste Branch und „n Änderungen seit dem letzten Commit“.
- **Nicht in 4.4**: Staging, Commit-Nachricht und Commit, Branches, Pull, Push, Konflikte. Begründung: Diese Fläche ist groß (Konflikte in Binärteilen, Authentifizierung, Fehlerfälle). Die Zielgruppe nutzt entweder schon ein Git-Werkzeug oder braucht eher einen „Stand sichern“-Knopf (Commit von allem mit Datum). Der passt später zu „Veröffentlichen“. Agenten committen per CLI.

---

## 7. Einordnung von 3d (Erzählen)

3d umfasst laut Spec 3c §13 und Roadmap: die Tiefe „Erzählen“ mit Rezepten und Live-Vorschau der Assistenten, Flags und Variablen, Geschichten mit Story-Graph, Löschen mit Referenzsperre, dazu aus der Tabelle 3b.3 Ei-Attacken, Kampfsprites, DS-Bildschirm-Varianten und Erzählen-Layouts.

Was davon am Textprojekt hängt:

| Teil | Hängt am Textprojekt? | Befund |
|---|---|---|
| Erzählen-Tiefe, Live-Vorschau, DS-Varianten, Kampfsprites | **nein** | reine Oberfläche bzw. Dekoder |
| Flags und Variablen, Namen | **nein**, die Namen gibt es schon | Die scrcmd-Datenbank hat sie aus pret: Platin 4098 Flags und 315 Variablen (137 als `UNUSED` markiert), HGSS 2252 Flags und 394 Variablen, DP keine (`metadata/scrcmd/*.json`). rotom-Quellen benutzen sie schon (`FLAG_HAS_POKEDEX` in `metadata/templates.yaml`). |
| eigene Namen für Flags und Variablen | **ja** | Das ist eine neue Projektdatei ohne ROM-Gegenstück. rotom kann `#include`, `#define` und `alias` (rotom `src/compiler/lexer.rs:117-118`, `token.rs:13`), die App kann die Namen also als Include-Datei für die Scripts erzeugen. Das geht in beiden Welten, die Quelle der Namen gehört aber ins Textprojekt. |
| Verwendung eines Flags (wo gesetzt, wo geprüft) | **nein**, braucht aber Script-Referenzen | Der Referenzindex kennt Scripts noch nicht als Quelle: „Scripts are no source yet (3b.4)“ (`data/refs.rs:16`) |
| Geschichten, Story-Graph | **ja** | eine reine Projekt-Entität ohne ROM-Gegenstück; vor Phase 4 wäre das eine weitere Wurzel-JSON neben `g4-world.json`, mit Migration in 4.3 |
| Löschen mit Referenzsperre | **ja, stark** | braucht den Referenzindex mit Scripts. Im DSPRE-Layout heißt Löschen außerdem „Mitglied aus einer NARC entfernen“, und alle folgenden Nummern verschieben sich (oder das Mitglied bleibt als Lücke). Mit symbolischen IDs nummeriert der Build neu, und Löschen wird „Datei entfernen, Referenzen prüfen“. |

**Empfehlung (recommend): 3d nach Phase 4, vor Phase 5; die Script-Referenzen ziehen nach 4.3 vor.**

Gründe:

1. Die zwei schweren Teile von 3d (Geschichten als Entität, Löschen) sind im Textprojekt einfacher und müssten vorher in einem Format gebaut werden, das 4.3 gleich wieder migriert.
2. 4.3 braucht die Script-Referenzen ohnehin. Um Nummern in Scripts durch symbolische IDs zu ersetzen (Trainer, Items, Flags), muss der Export wissen, wo Scripts auf Entitäten zeigen. Das ist derselbe Index, den die Referenzsperre braucht. Also kommt `refs.rs` mit Scripts als Quelle in 4.3 (Gutachten A/B prüfen den Umfang) und 3d baut darauf auf.
3. 3d vor Phase 5 testet das Entitätenmodell mit neuen Arten ohne ROM-Gegenstück (Geschichte, Flag-Name). Genau das sind später Plugin-Entitäten. So zeigt sich, ob Schema und Diff erweiterbar sind, bevor die Plugin-API eingefroren wird.
4. Die Erzählen-Tiefe als Oberfläche könnte jederzeit kommen. Sie allein vorzuziehen bringt aber wenig, weil sie ihre Inhalte (Geschichten, Flags) aus dem Textprojekt bekommt.

Die Alternative „in Phase 4“ (als 4.5) macht die Phase zu groß (4.3 ist schon L). „Vor Phase 4“ hieße Geschichten zweimal zu bauen.

---

## 8. Nebenaufgaben 4.0 und das Projektformat

Gelesen: Roadmap 4.0, `start.rs` (`.g4/base.json`) und die Spezifikation 4.0 auf dem Branch der parallelen Sitzung (`git show phase-4-0-nebenaufgaben:…/2026-10-27-phase-4-0-nebenaufgaben-design.md`, Zeilen 59-62, 80-83, 101, 151).

| Frage | Befund | Empfehlung |
|---|---|---|
| App-Symbol | betrifft nur das Bundle (`src-tauri/icons/`, `design-system/marke/`), nicht das Projekt | keine Anforderung an das Format |
| Ort von `.g4/preview.png` | `.g4/preview.png`, PNG RGB8 256 × 192, „um 50 KB“ (Spec 4.0 Z. 59, 101) | passt; auf die Erlaubnisliste (5.1) |
| `.g4/preview-standard.png` | Cache: die App erzeugt das Bild beim Öffnen neu (Spec 4.0 Z. 60) | **nicht versionieren**; mit der Erlaubnisliste automatisch ignoriert. Besser noch unter `.g4/cache/`, damit die Regel „oberste Ebene von `.g4/` = Projekt“ gilt. Das ist ein Hinweis an die 4.0-Sitzung, kein Muss. |
| Binärdatei in Git | 50 KB, ändert sich selten; jede Änderung kostet rund 50 KB Historie | unkritisch, kein LFS |
| Schema-Version | PNG braucht keine. `BaseMark` hat `version: 1` (`start.rs:224`) und enthält nichts Rechnerabhängiges (keinen ROM-Pfad), ist also Git-tauglich | Phase 4 bekommt ein Projekt-Manifest mit Schema-Version für die Entitäten. `base.json` behält seine eigene Version und erweitert `origin` um `"dspre"` (4.1). |
| Migration beim Textprojekt | Das Textprojekt hat denselben Wurzelordner, `.g4/` bleibt | keine Migration; die Kommentare zu `.g4/` korrigieren (2.1) |
| Rechtliches | das Standardbild ist ein Render von Spielgrafik | im privaten Repo unkritisch, Hinweis aus Abschnitt 4 deckt es ab |

Schreiben atomar über `.g4/` und `rename` (Spec 4.0 Z. 82) passt zu `fsutil::write_atomic` (`.g4/tmp/`), das ignoriert ist.

---

## 9. Was Phase 5 und Phase 6 vom Projektformat brauchen

### 9.1 Phase 5 (Plugin-Host)

Die Contribution Points der Hauptspezifikation (§4.4) umfassen `assetKinds/codecs`, `schemas`, `metadata`, `buildSteps`, `patches` und `settings`. Phase 4 sollte dafür vorhalten:

- **Namensräume für Entitäts-Arten.** Eine Art heißt `<plugin-id>:<art>` (First-Party ohne Präfix oder `g4:`). Der Ordner im Projekt folgt daraus, z. B. `data/<plugin-id>/<art>/<ID>.json`. Kein fest verdrahtetes Enum der Arten im Format; `AssetKind` bleibt das Kernel-Enum für die eingebauten Arten.
- **Schema-Version je Art, nicht nur je Projekt.** Ein Plugin migriert seine Arten selbst. Das Projekt-Manifest führt `schemas: { "<art>": <version> }`.
- **Plugins im Manifest**: `plugins: { "<id>": "<semver-Bereich>" }`, damit ein Klon weiß, was fehlt. Die App zeigt fehlende Plugins an und lässt deren Dateien unberührt (lesen ja, schreiben nein).
- **Projekt-Einstellungen je Plugin** im Manifest oder in `settings/<plugin-id>.json` (versioniert); Nutzer-Einstellungen bleiben in der App.
- **Build-Schritte von Plugins** im Build-Graph (4.2): Der Fingerprint enthält Plugin-ID und Version, Ausgaben gehen nach `.g4/build/<schritt>/`. Der Graph darf keine feste Liste von Schritten sein.
- **Projektlokale Plugins** (Gate von Phase 5: „aus einem Ordner geladen“): Ein Ordner `plugins/` im Projekt wäre bequem, ist aber Code, der mit dem Projekt reist (wie VS Code „Workspace Trust“). Vorschlag: Phase 4 reserviert den Namen `plugins/` und lädt dort nichts. Phase 5 entscheidet mit Vertrauensabfrage.
- **Diff und Referenzen generisch**: Der Diff aus 6.1 arbeitet auf JSON mit Feldnamen aus dem Schema, der Referenzindex auf deklarierten Referenzfeldern. Dann bekommen Plugin-Arten beides ohne eigenen Code.

### 9.2 Phase 6 (Kartenerstellung, Weltformat)

Phase 6 plant „Karte als JSON (Tiles, Höhen, Gebäude, Kollision) plus glTF für Freiform-Geometrie; Compiler zu Kartendatei“ und PNG → NSBTX. Für Phase 4 heißt das:

- **Eine Entität darf mehrere Dateien haben.** Eine Karte wird ein Ordner `maps/MAP_X/` mit `map.json` und Beifiles (`model.nsbmd` heute, `terrain.json`, `mesh.glb`, `tiles.png` später). Die Regel „eine JSON je Entität“ sollte als „eine Manifest-JSON je Entität, Beifiles daneben“ formuliert werden. Sonst muss Phase 6 das Format brechen.
- **Mehrere Quellformen je Art**: Eine Karte ist in 4.3 binär (importiertes NSBMD), in Phase 6 eine Quelle, die der Build kompiliert. Das Manifest der Karte nennt seine Quelle (`source: "binary" | "terrain"`), der Build-Graph kennt die Stufe „Quelle → Kartendatei“ vor `unpacked/`. Das passt zu den Stufen von 4.2.
- **`g4-world.json`** (Namen der Weltkarten, Klarnamen der Räume) geht in die Welt-Entitäten über. Gutachten A (Ausbau) hat das schon als Teil des Textprojekts vorgesehen.
- **Große Binärquellen in Git**: glTF und Tileset-PNGs können groß werden. In 4.4 kein LFS. Die `.gitattributes` lässt aber Platz für `*.glb filter=lfs` als Option, und die Vorlage markiert ihren Block so, dass der Kernel ihn später ergänzen kann.
- **Kollisionen bei symbolischen Namen neuer Karten**: Die Regel für Umbenennen und Kollisionen aus 4.3 gilt auch für in Phase 6 erzeugte Karten. Ein Umbenennen ist in Git ein `rename` des Ordners; der Diff in der App sollte das als eine Änderung zeigen (gix erkennt Umbenennungen).

---

## 10. Empfehlungen (Zusammenfassung)

1. **4.4 bleibt klein (S bis M)**: Vorlagen für `.gitignore` und `.gitattributes` aus dem Kernel, „Git einrichten…“ (init, erster Commit über die CLI, Hinweis „nur privat“), Technik › Änderungen mit semantischem Diff gegen Basis und gegen `HEAD`, MCP-Werkzeug `diff_entity`. Keine allgemeine Commit-Funktion.
2. **`gix` zum Lesen, `git`-CLI zum Schreiben**, wenn installiert; `deny.toml` anlegen und `cargo deny` ins Gate, bevor `gix` kommt. `git2` nur, wenn später Commits ohne installiertes Git gebraucht werden (die Lizenz des C-Teils dann von Hand dokumentieren).
3. **`.gitattributes` mit `* -text` ist Pflicht** (81–99 Spieldaten-Dateien würden mit `core.autocrlf=true` umgeschrieben).
4. **`.g4/` als Erlaubnisliste**: versioniert sind nur `base.json` und `preview.png`, alles andere ist lokal; die Kommentare in `world_meta.rs` und `checks/ignore.rs` werden korrigiert, `preview-standard.png` besser nach `.g4/cache/`.
5. **Zwei Korrekturen, die Git erst tragfähig machen** (4.2, oder als Vorab-Fix in 4.4): `write_if_changed` legt den Elternordner an (`build.rs:116-127`); die Laderegel der Texte nach Zeit (`text.rs:250-270`) wird durch Fingerprints ersetzt (2.4).
6. **Textprojekt mit Basis aus der ROM (T2)**: Das Repo enthält nur die Projektschicht (gemessen ≤ 5,6 MiB gepackt statt 54–77 MiB), die Basis entsteht aus der eigenen ROM. Die Entscheidung liegt bei den Format-Gutachten.
7. **Git nur lokal oder privat**, Hinweis in der App und in der Vorlage; geteilt wird der xdelta-Patch.
8. **3d nach Phase 4, vor Phase 5**; die Script-Referenzen im Referenzindex kommen mit 4.3.
9. **Format offen halten**: Entität = Manifest-JSON plus Beifiles, Arten mit Namensraum und eigener Schema-Version, Plugins und Projekt-Einstellungen im Manifest, Build-Graph mit offenen Schritten.

---

## 11. Folgen für den Schnitt

**4.4 Git minimal (S bis M), Inhalt:**

- Kernel: `git_templates` (Metadaten `metadata/git/`), `git_status` und `git_blob_at_head` über `gix`, `git_init` über die CLI (mit Erkennung „nicht installiert“), `diff_entity` (JSON-DTO alt/neu → Feldliste) für die Arten mit Dekoder; Brücke und MCP.
- App: Technik › Änderungen (Liste, Umschalter Basis/Commit, Feldtabelle, „Öffnen“, „Auf Original zurücksetzen“ aus 4.1), „Git einrichten…“ mit Hinweis, Häkchen im Assistenten „Neues Spiel aus ROM“, Statusleiste mit Branch und Zahl der Änderungen.
- Design-System: Komponente „Änderungstabelle“, Fläche „Änderungen“ in der `.fig`.
- Voraussetzungen aus 4.2: Elternordner im Build, Fingerprints statt Zeit-Regel.

**Gate 4.4** (zusätzlich zum Phasen-Gate „Projekt liegt in Git, unverändertes Projekt rebuildet byte-identisch“):

1. Auf frischen Kopien von Platin DE, HeartGold DE und US richtet „Git einrichten“ (über MCP kopflos und in der Test-Umgebung) Repo, `.gitignore`, `.gitattributes` und ersten Commit ein. `git status` ist danach leer und bleibt nach `g4k build` leer.
2. Ein Klon mit `core.autocrlf=true` baut eine ROM byte-gleich zum Original-Build (gated Test; prüft Elternordner und `.gitattributes`).
3. Ein Trainer und ein Text werden geändert. Technik › Änderungen zeigt beide mit Feldnamen „alt → neu“ gegen `HEAD` und gegen die Basis; „Auf Original zurücksetzen“ nimmt die Änderung als ein Undo-Schritt zurück.
4. Branch-Wechsel mit geänderter Text-JSON, dann Build: Die JSON des Branches gewinnt (Regressionstest zu 2.4).
5. `cargo deny check licenses` grün mit der neuen `deny.toml`; `scripts/gate.sh` grün.

**Einordnung 3d:** nach Phase 4, vor Phase 5. In 4.3 kommen die Script-Referenzen in den Referenzindex (Voraussetzung für symbolische IDs in Scripts und später für die Referenzsperre).

**Nicht in Phase 4:**

- Commit-Oberfläche, Branches, Merge und Konflikte, Pull und Push, Remotes, „Auf GitHub veröffentlichen“;
- Git LFS (Phase 6 prüft es für glTF und PNG);
- „Stand sichern“ als Ein-Klick-Commit (später mit Veröffentlichen);
- Plakette „geändert“ im Inspector;
- semantischer Diff für Binär-Assets jenseits „Teil geändert“ (Bildvergleich von Modellen und Texturen);
- projektlokale Plugins (Phase 5, mit Vertrauensabfrage);
- 3d.

---

## 12. Offene Fragen

1. **T1 oder T2** (Basisschicht im Repo oder aus der ROM)? Hängt an den Format-Gutachten. Mit T2 braucht jeder Klon die eigene ROM und der Diff „gegen Basis“ die Basisschicht unter `.g4/base/`.
2. **Vollprojekt oder Differenzprojekt** im Textprojekt? Ein Differenzprojekt wäre teilbar, verliert aber die vollständige Suche und den Überblick ohne ROM.
3. **Wo liegt der Zwischenstand** im Textprojekt (`rom/` sichtbar oder `.g4/build/rom/`)? Davon hängt ab, welchen Ordner DSPRE öffnet und wie lang die `.gitignore` wird.
4. **Git im DSPRE-Layout vor 4.3 anbieten?** Mit den Korrekturen aus Empfehlung 5 ja, sonst nur mit Warnung. Vorschlag: „Git einrichten“ erst ab dem Stand mit Fingerprints (4.2).
5. **Soll die App bei fehlender `git`-CLI selbst committen** (dann `git2` oder ein gix-Commit-Pfad)? Vorschlag: nein in 4.4, neu prüfen mit „Stand sichern“.
6. **`.rotom/status/compile-state.json`**: Im DSPRE-Layout versioniert (Quirks), mit Rauschen durch `last_compiled`. Soll 4.2 die Quirks in eine eigene, stabile Datei ziehen, auch für das DSPRE-Layout?
7. **`.g4/preview-standard.png` nach `.g4/cache/`?** Hinweis an die 4.0-Sitzung; mit der Erlaubnisliste nicht zwingend.
8. **Projektlokale Plugins** und Vertrauensabfrage: Reserviert Phase 4 den Ordner `plugins/`?
