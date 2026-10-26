# Phase 4 „Textprojekt und Build-Graph“: Entscheidungen (2026-10-27)

Anlass: Phase 4 ist die nächste Phase nach dem Ausbau 3e bis 3h. Die Roadmap hatte am 2026-10-26 einen Schnitt 4.0 bis 4.4 vorgeschlagen, dazu die Liste „Schon da“ und vier offene Fragen (DSPRE-Änderungen bei Textprojekt als Quelle, Herkunft der Basis-ROM, stabile Namenstabellen für symbolische IDs, Lizenz und Pfad des xdelta-Encoders). Drei Gutachten (nur lesend am Code, mit Messungen an frischen Kopien von Platin DE, HeartGold DE und HeartGold US, die danach gelöscht wurden) haben das geprüft:

- [gutachten-a-textprojekt.md](gutachten-a-textprojekt.md): Entitäten im Kernel, Format und Ablage, symbolische IDs, Schema und Migrationen, byte-gleicher Round-Trip, DSPRE parallel; mit einem Prototyp in Rust (Export, Import, Byte-Vergleich).
- [gutachten-b-build-graph.md](gutachten-b-build-graph.md): Stufen und Zeiten des heutigen Builds, Fingerabdrücke, Prototyp eines inkrementellen Builds, `.g4/base.json` für DSPRE-Ordner, Herkunft der Basis-ROM, Vergleich je Entität, xdelta-Kandidaten mit Lizenz und Messung.
- [gutachten-c-git-einordnung.md](gutachten-c-git-einordnung.md): Git auf DSPRE-Ordner und Textprojekt, `.gitignore`/`.gitattributes`, Diff je Entität, Git-Bibliothek, Rechtliches, Einordnung von 3d, Anforderungen von 4.0, Phase 5 und Phase 6 an das Format.

Die Entscheidungen folgen der Empfehlung („recommend“) der Gutachten. Wo die Gutachten sich widersprechen oder Fragen offen ließen, steht hier die gewählte Vorgabe mit Grund. Der Nutzer kann jede einzeln umstoßen; die Roadmap ([docs/ROADMAP.md](../../ROADMAP.md), Abschnitt „Phase 4“) setzt sie voraus. Die Spezifikation jeder Teilphase folgt erst mit deren Brainstorming und Plan.

## Wichtigste Messwerte

| Messung | Platin DE | HeartGold DE | HeartGold US | Quelle |
|---|---|---|---|---|
| `g4k build` ohne Änderung, warm (kalt) | 1,1–1,2 s (13–17 s) | 4,7–4,9 s (21–23 s) | 4,9–5,2 s (17–20 s) | B §1.3 |
| Prototyp Build-Graph nach einer Kartenänderung, byte-gleich zum vollen Build | 0,18 s | 0,19 s | 0,17 s | B §2.5 |
| Rebuild des DSPRE-Ordners mit Marke `origin: "dspre"` gleich der `.nds` (SHA-1) | ja | ja | ja | B §3.1 |
| xdelta3 (C) Patch: ein Byte / Karte + Script + Text | 0,4 KB / 4,8 KB | 0,4 KB / 8,3 KB | 0,4 KB / 8,3 KB | B §4.1 |
| Textprojekt-Prototyp: Round-Trip aller Entitäten byte-gleich | ja | ja | ja | A §2.2 |
| Lesbare Dateien und Größe (Blobs nur bei Abweichung: 0 in Vanilla; mit einer Datei je Spezies rund 1 000 weniger) | 7 559, 16,2 MiB | 7 286, 16,7 MiB | 7 286, 16,5 MiB | A §2.3 |
| Export / Import mit Codieren und Vergleich (unter Last, ±50 %) | 5,6 s / 16,5 s | 4,7 s / 11,5 s | 5,5 s / 15,8 s | A §2.4 |
| Git: erster Commit des DSPRE-Ordners, `.git` gepackt | 8,9 s, 63 MiB | 9,3 s, 77 MiB | 9,1 s, 76 MiB | C §2.2 |
| Git: nur die Projektschicht (Stellvertreter Textprojekt), gepackt | 5,2 MiB | 5,6 MiB | 5,6 MiB | C §3.2 |

## Phasenschnitt und Reihenfolge

| Teilphase | Inhalt | Größe | Warum an dieser Stelle |
|---|---|---|---|
| 4.0 Nebenaufgaben | App-Symbol, Projektvorschau `.g4/preview.png` (läuft parallel in eigener Sitzung) | S | unabhängig vom Kernel; einzige Anforderung ans Format siehe unten |
| 4.1a Basis-ROM und Patch | `BaseMark` v2 mit `origin: "dspre"`, „Basis-ROM zuordnen…“, `base-roms.json` in den App-Daten, Basisleser im Kernel, xdelta-Export, `deny.toml` | M− | klein, liefert den byte-gleichen Beweis für DSPRE-Ordner und einen veröffentlichbaren Patch; Basisleser braucht 4.1b und 4.3a |
| 4.2 Build-Graph | BLZ-Cache, `.g4/build/state.json`, Tore je Schritt, Member-genaues Einsetzen ins NARC, Pack-Einträge für die DSPRE-Ordner, Fingerabdrücke statt Zeit-Regel bei Texten, Anzeige der Schritte | M | unabhängig von 4.1a (getrennte Dateien), **parallel in zwei Worktrees**; liefert das Fingerabdruck-Format für 4.1b und 4.3 |
| 4.1b Auf Original zurücksetzen | Reset je Entität als Undo-Schritt, Liste „geändert gegenüber Original“ aus den Hashes des Graphs | M | braucht Basisleser (4.1a) und Member-Hashes (4.2) |
| 4.3a Textprojekt im Kernel | Modul `textproject`, Manifest mit Schema-Version, Symboltabellen und Register, Export/Import, `g4k text …`, Stufe „Entität → Member“ im Graphen, Round-Trip-Gate | L | braucht 4.1a (Blobs nur bei Abweichung) und 4.2 (Graph); kann parallel zu 4.1b laufen |
| 4.3b Textprojekt als Quelle | Store auf JSON, DSPRE-Abgleich mit Konfliktliste, symbolische IDs in den Verbrauchern, Referenzindex mit Scripts, „ID umbenennen“, Umstellung eines DSPRE-Ordners | L | braucht 4.3a |
| 4.4 Git minimal | Vorlagen `.gitignore`/`.gitattributes`, „Git einrichten…“, Technik › Änderungen mit semantischem Diff gegen Basis und `HEAD`, `diff_entity` über MCP | S–M | braucht die JSON-Entitäten (4.3b) und die Korrekturen aus 4.2 |

Reihenfolge: 4.0 ∥ (4.1a ∥ 4.2) → (4.1b ∥ 4.3a) → 4.3b → 4.4. Der vorgeschlagene Schnitt (4.1 vor 4.2, 4.3 als eine L-Phase) wird damit geändert: 4.1 teilt sich, weil „Zurücksetzen“ die Hashes des Graphs braucht (B §6); 4.3 teilt sich, weil Kernel-Format und App-Umstellung je für sich L sind und der Kernel-Teil kopflos ein eigenes Gate hat (A §5). Der Basisleser wandert gegenüber Gutachten B von 4.1b nach 4.1a, damit 4.3a nicht auf 4.1b warten muss.

## Basis-ROM und Patch (Gutachten B, 4.1)

| Frage | Entscheidung | Grund |
|---|---|---|
| Marke für DSPRE-Ordner | `BaseMark` v2: `origin: "dspre"`, `template` optional, neu `assigned`; `read_base_mark` liest v1 und v2; `secure_area_crc` und `secure_area_sha1` aus der `.nds` wie bei `create_project` | mit Marke rebuilden alle drei DSPRE-Ordner byte-gleich zur Original-ROM, ohne Marke weichen 4 Bytes ab (gemessen) |
| Zuordnen | `start::assign_base(root, nds)`, `g4k base assign` und `g4k base check`; nur ein falscher Spielcode gegen `header.yaml` sperrt, Titel, Version, unbekannte SHA-1 und abweichendes ARM7/Banner warnen; liegt `<Name>.nds` neben `<Name>_DSPRE_contents`, schlägt die App sie vor, der Nutzer bestätigt; über MCP unumkehrbar mit Rückfrage | Warnung statt Sperre wie im Roadmap-Vorschlag; der Spielcode macht CRC und Patch sonst falsch |
| Agenten-Freigabe | beim Zuordnen schreibt die App den Eintrag `(pfad, None)` in `agents.json` auf die neue `base_id` um | die Marke ändert die `base_id`, die alte Freigabe passte sonst nicht mehr |
| Herkunft der Basis-ROM | App-Einstellung `<App-Daten>/base-roms.json` (Muster `agents.json`, atomar, 0600), Schlüssel SHA-1; eingetragen bei „Neues Spiel aus ROM“, beim Zuordnen und über „Basis-ROM suchen…“; Auflösen über Größe und mtime, sonst neu hashen (0,12 s); kopflos `--base-rom` und `G4_BASE_ROM_DIR` | die ROM liegt nie im Projekt und nie in Git; mehrere Projekte teilen eine Basis |
| Fehlende Basis-ROM | nichts sperrt den Build; xdelta-Export, Zurücksetzen und Diff gegen die Basis sind aus und nennen den Grund; Ersatz nur mit gleicher SHA-1 | kein Modaldialog beim Öffnen |
| Basisleser | Kernel-Modul `base`: ROM per SHA-1 öffnen (`raw::Rom` + `FileSystem::parse`), Member je `AssetKind`, Header über den Codec; je Aufruf lesen (15–60 ms), nicht im Speicher halten; in **4.1a** | gebraucht von xdelta-Selbstprüfung, 4.1b und 4.3a |
| xdelta-Encoder | **xdelta3.c (Apache-2.0) mit eigener schmaler FFI über `cc`**, ohne bindgen; Quelle mit `NOTICE` im Repo | das Crate `xdelta3` zieht über bindgen 0.52 vier RustSec-Befunde und kann weder Adler32 noch App-Header; `oxidelta` (MIT) erzeugt 0,7–2,6 MB statt 5–8 KB |
| Patch-Form | ohne Zweitkompression, mit Adler32; Basis-SHA-1 im App-Header (`<ziel>//<quelle [sha1]>/`) und in einer Begleitdatei `<Name>.xdelta.txt` (Spiel, Basis-SHA-1, Ziel-SHA-1, Studio-Version, ein Satz Anleitung); Selbstprüfung (Patch auf die Basis anwenden, SHA-1 gegen die gebaute ROM) vor dem Schreiben; Ziel `build/<Projekt>-<Spielcode>.xdelta` | jeder gängige Patcher kann ihn anwenden (xdelta3 3.x, Rom Patcher JS); 0–2,4 KB größer als mit LZMA |
| Ort in der App | Technik › Veröffentlichen › „Patch erstellen…“, MCP-Werkzeug mit Rückfrage | Design-System „Veröffentlichen“ |
| `cargo deny` | `deny.toml` in 4.1a anlegen (Freigabeliste wie `gutachten-b-messung/deny.toml`); die heute schon roten Advisories zuerst durch Updates beheben, nur was sich nicht beheben lässt als begründete Ausnahme; `cargo deny check licenses advisories` wird Gate-Schritt | das Repo hat noch keine `deny.toml`; ein Lizenz-Gate ist Voraussetzung für `gix` (4.4) |
| Auf Original zurücksetzen (4.1b) | Undo-Schritt im Store mit Vorher-Nachher-Rückfrage, im Menü „…“ des Inspector-Kopfs und im Kontextmenü der Liste; zuerst Karte, Eventdatei, Textarchiv (je Nachricht und ganz), Trainer, Pokémon-Daten, Begegnungen, dann Header (Feld für Feld über den Codec), Scripts (Basis dekompilieren), Texturen und Modelle (ganz); nicht für neue Entitäten („Kein Original“) und nicht für ARM9-Patches | B §3.4; ein Reset kostet < 80 ms |
| Liste „geändert gegenüber Original“ | aus Hashes: xxh3 je Basis-Member, einmal je SHA-1 im App-Cache, gegen die Member-Hashes aus `.g4/build/state.json`; keine Plakette im Inspector in Phase 4 | Ordner lesen kostet kalt bis 16 s, Hashes 0 Dateizugriffe |

## Build-Graph (Gutachten B, 4.2)

| Frage | Entscheidung | Grund |
|---|---|---|
| Modell | **Variante B „Member-genau mit Einsetzen“**: kleine Schrittliste mit `Step`-Vertrag (Eingaben, Ausgaben, Werkzeug-Salz), Stufen Quelle → `unpacked/` → NARC → ROM, early cutoff; keine allgemeine Build-Engine | Prototyp 0,17–0,20 s nach einer Kartenänderung, byte-gleich; eine DAG-Engine wäre für sechs Schrittarten zu viel, Plugin-Schritte (Phase 5) passen in den `Step`-Vertrag |
| Reihenfolge in 4.2 | BLZ-Cache für ARM9 und Overlays zuerst (spart auf HGSS allein 4 s), dann Zustand und Tore für rotom und chatot, dann Einsetzen ins NARC, dann die Anzeige | größter Gewinn zuerst |
| Fingerabdrücke | `stat` (mtime, Größe, Inode) zuerst, xxh3 des Inhalts nur bei Abweichung; racy-Regel für Dateien, die im selben Zeitschlitz geschrieben wurden | `stat` über 21 000 Eingaben 37–195 ms, Hash aller Inhalte warm 4–25 s |
| Speicher | `.g4/build/state.json` (mit Formatversion, Werkzeug-Salz je Schritt aus `Cargo.lock`) und `.g4/build/blz/`; lokal, jederzeit löschbar („Neu bauen (ohne Cache)“); **eine** Zustandsdatei auch für den DSPRE-Abgleich von 4.3b (statt eines eigenen `.g4/sync.json`) | A §3.5 und B §2.3 beschreiben dieselbe Form |
| ds-rom | bleibt voll (40–60 ms); BLZ-Cache über die öffentliche API mit gated Test „Cache-Weg == `Rom::load`“; kein ROM-Patch an Ort und Stelle; ein Upstream-Vorschlag an ds-rom ist erlaubt, aber keine Voraussetzung | B §2.4 |
| Laderegel der Texte | die Zeit-Regel `text_winner` („neuere Datei gewinnt“) wird durch Fingerabdrücke ersetzt: nur JSON neu → JSON gewinnt, nur Member neu → Member gewinnt, beide neu → Konflikt mit Rückfrage, ohne Zustand gewinnt die JSON mit Hinweis | nach `git switch` überschreibt der Build heute still die JSON des Branches (C §2.4, reproduziert) |
| Elternordner | `write_if_changed` legt den Elternordner an | ein Klon ohne die ignorierten NARCs baut Platin sonst nicht (C §2.2) |
| DSPRE-Ordner, die das Studio nicht packt | die 14 (Pt) bzw. 17 (HGSS) Ordner (`pokeHeight`, `synthOverlay`, `pokemonBattleSprites` …) kommen als reine Pack-Einträge in `archives.yaml`, mit Gate „Vanilla bleibt byte-gleich“; bis dahin Prüfung `build.unpacked-not-packed` | DSPRE-Änderungen dort gehen beim Bauen mit dem Studio heute verloren (B §1.4) |
| Prüfungen | `build.unpacked-not-packed`, `build.hidden-file` (Befund mit „Entfernen“ statt hartem Fehler), `build.member-gap`, `build.base-missing` (Hinweis), `build.cache-stale` (intern) | Roadmap: Prüfungen des Build-Graphs (3b.4 §1) |
| Anzeige | Technik › Prüfen und Bauen zeigt eine Zeile je Schritt aus `BuildReport.steps` (Dauer, Zähler, „übersprungen“); `g4k build --timings` schreibt dieselben Zeilen als JSON fürs Gate | Design-System „Build-Schritte“ |

## Textprojekt (Gutachten A, 4.3)

| Frage | Entscheidung | Grund |
|---|---|---|
| Format und Ablage | **Variante B**: Projektwurzel = DSPRE-Ordner = Git-Wurzel. Neu sind das Manifest `g4-project.json` und `data/<art>/NNNN.json`, eine kanonische JSON je Entität (feste Schlüsselreihenfolge, `\n`, Gitter als Textzeilen). Texte bleiben chatot-JSON in `expanded/textArchives/`, Scripts rotom-Quellen in `expanded/scripts/` | beide Formate sind schon textuell, kanonisch, DSPRE-kompatibel und byte-gleich (gemessen); DSPRE, rotom-CLI und LSP arbeiten weiter auf denselben Dateien |
| Zwischenstand gebündelt in `rom/`? (Vorschlag aus C §5.3) | **nein**: `unpacked/`, `files/`, `arm9/`, Overlays bleiben an ihrem Ort in der Wurzel und sind Zwischenstand bzw. Basis; die `.gitignore`-Vorlage zählt die Pfade auf (der Kernel erzeugt sie aus dem Layout) | ein vorhandener DSPRE-Ordner wird an Ort und Stelle zum Textprojekt, ohne Dateien zu verschieben, und DSPRE öffnet ihn weiter |
| Entität | eine Manifest-JSON je Entität, binäre Teile als Beifiles mit gleichem Stamm daneben (`data/maps/0003.json` + `0003.nsbmd`); Spezies (Personal, Lernset, Entwicklung) und Trainer (Eigenschaften, Team) sind je eine Entität über mehrere Member; Dateiname ist die Nummer, die symbolische ID steht in der Datei | Phase 6 hängt weitere Beifiles an (`terrain.json`, `mesh.glb`), ohne das Format zu brechen (C §9.2); Dateinamen bleiben beim Umbenennen stabil |
| Vollständig oder nur Abweichungen | strukturierte Entitäten **vollständig** als JSON; Binäres (Karten-Modelle, BDHC, Texturen, Gebäude) **nur bei Abweichung** von der Basis als Beifile, sonst Verweis per SHA-1; nicht abgebildete, geänderte Dateien (auch `arm9.bin` außerhalb der Header-Tabelle) als Blob unter `data/raw/<pfad>` | lesbar und durchsuchbar für Nutzer und Agenten; ein Vanilla-Projekt hat 0 Blobs |
| Basisschicht | **aus der Basis-ROM (T2)**: nach einem Klon entsteht der DSPRE-Teil aus der eigenen ROM (SHA-1 geprüft) und dem Textprojekt; die Basisschicht liegt nie in Git | Repo rund zehnmal kleiner (≤ 5,6 MiB gepackt statt 54–77 MiB), Diffs zeigen nur Nutzerarbeit, rechtlich sauberer; 4.1a baut die Basis-ROM-Logik ohnehin |
| Symbolische IDs: Quelle | Spezies, Formen, Attacken, Items, Fähigkeiten, Typen aus **festen englischen Tabellen im Repo** (`metadata/symbols/*.yaml`, abgeleitet aus PokeAPI unter BSD-3-Clause mit Namensnennung; pret wird nicht kopiert, keine Lizenz); Header als `MAP_<interner Name>` (`MAP_T01`); Trainer, Trainerklassen und Orte werden beim ersten Export **in der Projektsprache getauft und eingefroren** (Trainer aus Klasse und Name); Matrizen, Karten, Eventdateien, Area-Data, Textarchive und Scripts behalten Nummern | gemessen: Spezies, Attacken, Fähigkeiten, Typen 0 Kollisionen; interne Header-Namen eindeutig und in DE/US gleich; IDs aus der Projektsprache wären zwischen DE und US nur zu 2–37 % gleich. Namen sind keine ROM-Daten (keine Bytes, keine Texte aus dem ROM); die Lizenz der PokeAPI-Daten wird beim Start von 4.3a noch einmal bestätigt |
| Kollisionen, Umbenennen | Suffix `_2`, `_3` in Nummernreihenfolge; ohne verwertbaren Namen `PREFIX_NNNN`; Register `data/ids/<art>.json` (ID → Nummer); Umbenennen ändert den Anzeigenamen, die ID nur der explizite Befehl „ID umbenennen“, der alle Verweise in einer Store-Transaktion nachzieht; überall gilt ID **oder** Nummer, der Export schreibt die ID | Pt: 432 Suffixe bei 928 Trainern, hinnehmbar, weil eingefroren |
| Schema und Version | **S2**: `schemaVersion` im Manifest `g4-project.json` (dazu Spielcode, Familie, Sprache, Studio-Version), Feld `schemas` je Art und Feld `plugins` reserviert für Phase 5; Migrationskette als reine Funktionen auf dem Dateibaum mit Schnappschuss vorher, neuere Versionen werden abgelehnt; Entitätstypen mit `deny_unknown_fields`; `g4-world.json` behält seine Version, `g4-checks.json` bekommt `version: 1` mit `serde(default)` | eine Migration ändert nicht 7 000 Dateien; Tippfehler fallen sofort auf |
| Basis-SHA-1 im Manifest? (A §3.3) | **nein**: `.g4/base.json` bleibt die einzige Quelle der Basis und ist versioniert; das Manifest dupliziert sie nicht | die Marke gibt es schon (3g, MCP-Freigabe, 4.1a), DSPRE-Ordner ohne Textprojekt brauchen sie auch |
| Code | neues Kernel-Modul `textproject` mit eigenen serde-Typen, getrennt von den Codecs (`g4-formats` bleibt ohne serde); generischer Abbilder über `RawRecord::FIELDS` für alle `fixed_record`-Arten; Feldnamen sind Schema (Snapshot-Test); DTOs der App bleiben getrennt | A §3.1 |
| Byte-Gleichheit | rotom-Quirks wandern aus dem Cache `.rotom/status/compile-state.json` in eine Projektdatei; belegte „unknown“-Felder, `Building.dummy`, `buildings_tail`, HG-BGS, Learnset-`tail` sind benannte JSON-Felder; was der Store sperrt, wird Blob mit `"locked": "<Grund>"`; Build-Reihenfolge Texte → Scripts → Text-Codierung | A §3.4, alle in Vanilla abgedeckt, für Hacks nötig |
| Round-Trip-Test | `textproject_round_trip` (Export → Member löschen → Import → Byte-Vergleich → Build → SHA-1 = Basis), `textproject_export_idempotent` (zweiter Export ändert nichts), `textproject_from_base` (Extract aus der ROM + Textprojekt aus Git → SHA-1 gleich), gated auf allen drei Projekten; Proptests je Schema-Typ und je ein synthetischer Test je Sperrfall; Fixture-Projekte je Migration ohne ROM-Daten | das ist das Gate der Phase |
| DSPRE parallel | **D2**: Fingerabdrücke je Member und je Quelle in `.g4/build/state.json`; beim Öffnen und bei Fensterfokus erst `stat` (30 ms), dann nur Geändertes hashen; einseitige Änderungen werden automatisch übernommen und gemeldet („3 Änderungen aus DSPRE übernommen“), beidseitige ergeben eine Konfliktliste in Technik mit Diff und Wahl je Entität, nie stilles Überschreiben; ohne Zustand einmal ein voller Abgleich mit Fortschritt | robust gegen mtime-Rauschen (Git, Kopieren), erkennt Konflikte |
| Umstellung eines DSPRE-Ordners | ausdrücklicher Schritt „Als Textprojekt einrichten…“ (Technik, Hinweis beim Öffnen, Häkchen im Assistenten „Neues Spiel aus ROM“) mit Fortschritt; ohne Manifest arbeitet das Studio weiter im DSPRE-Layout wie heute | schreibt rund 6 500–7 500 Dateien; DSPRE-Nutzer entscheiden selbst |
| Referenzindex und Scripts | Scripts kommen in 4.3b als Quelle in den Referenzindex (`data/refs.rs`); eine vom Kernel erzeugte rotom-Include-Datei macht die IDs in Scripts als Konstanten nutzbar; vorhandene Script-Quellen werden in Phase 4 nicht auf IDs umgeschrieben | „ID umbenennen“ und später die Referenzsperre (3d) brauchen denselben Index; rotom kann `#include` und `#define` |

## Git (Gutachten C, 4.4)

| Frage | Entscheidung | Grund |
|---|---|---|
| Umfang | **minimal**: Vorlagen, „Git einrichten…“ (Vorlagen schreiben, `git init`, erster Commit „Ausgangsstand“), Lesen von Status und `HEAD`, Technik › Änderungen mit semantischem Diff, Statusleiste mit Branch und Zahl der Änderungen, MCP `diff_entity` und `git_init` (Rückfrage); **keine** allgemeine Commit-Funktion | Commit, Branches und Konflikte sind eine große Fläche; die Zielgruppe hat ein Git-Werkzeug oder braucht später „Stand sichern“ |
| Bibliothek | **`gix` zum Lesen** im Prozess (reines Rust, alle 137 Crates frei lizenziert, Features `sha1`, `status`, `index`, `revision`, `blob-diff`), **`git`-CLI zum Schreiben**, wenn installiert (sonst „Git ist nicht installiert“, die Vorlagen schreibt der Kernel trotzdem); kein `git2` | Lesen muss ohne installiertes Git gehen; libgit2 ist GPL-2.0 mit Linking Exception, was `cargo deny` nicht sieht |
| Vorlagen | Metadaten im Kernel (`metadata/git/`), je eine für DSPRE-Layout und Textprojekt; eine vorhandene Datei wird nicht überschrieben, nur ein markierter Block ergänzt; Kopfkommentar „nur lokal oder privat“ | C §5 |
| `.gitattributes` | **Pflicht**: `* -text`, dazu `text eol=lf` für JSON, rotom, YAML, TOML, Markdown | 81–99 Spieldaten-Dateien würden mit `core.autocrlf=true` umgeschrieben (gemessen) |
| `.g4/` | **Erlaubnisliste**: die oberste Ebene von `.g4/` ist Projekt-Metadaten und versioniert (`base.json`, `preview.png`), die Unterordner `cache/`, `tmp/`, `build/`, `backup/` und alles andere sind lokal (`/.g4/*` mit `!/.g4/base.json`, `!/.g4/preview.png`); die Kommentare in `world_meta.rs` und `checks/ignore.rs` („Cache, darf gelöscht werden“) werden korrigiert (in 4.2) | neue Caches sind ohne Änderung der Vorlage ignoriert |
| Immer ignoriert | `*.nds`, `*.srl`, `*.xdelta`, `/build/`, Emulator-Spielstände (`*.sav`, `*.dsv`, `*.ml[0-9]`, `*.mln`, `*.state`), `/.rotom/cache/`; im DSPRE-Layout die NARCs, die der Build packt (nur wenn ihre `unpacked/`-Ordner da sind); im Textprojekt der ganze DSPRE-Teil außer `expanded/` und die Projektdateien | C §5.2, §5.3; im Textprojekt sind `expanded/textArchives` und `expanded/scripts` Quelle und bleiben versioniert |
| Git schon im DSPRE-Layout? | ja, ab dem Stand von 4.2 (Elternordner, Fingerabdrücke statt Zeit-Regel); 4.4 bietet „Git einrichten…“ für beide Layouts an | vorher drohte Datenverlust nach `git switch` |
| Rechtliches | Git **nur lokal oder privat**: die App richtet kein Remote ein, pusht nie und bietet kein „Auf GitHub veröffentlichen“; Hinweis beim Einrichten und in der Vorlage; geteilt wird der xdelta-Patch; keine Telemetrie mit Projektdateien | auch ein Textprojekt enthält den ganzen Spieltext |
| Diff je Entität | ein Mechanismus für drei Vergleiche (Basis, `HEAD`, gespeichert): alte und neue Bytes → Dekoder → JSON-DTO mit Feldnamen → Tabelle „Feld: alt → neu“; das DTO ist dasselbe, das 4.3 schreibt; Karten binär nur „Teil geändert“ plus Zellen-Diff der Permissions; Texte je Nachricht mit Wort-Diff; Scripts als Text-Diff | in 4.4 „Textprojekt-fertig“, Plugins bekommen ihn später über ihr Schema |
| Ort in der App | Technik › Änderungen (neuer Eintrag): Liste nach Art, Umschalter „gegen Basis / gegen letzten Commit“, Feldtabelle, „Öffnen“, „Auf Original zurücksetzen“ (4.1b); der Verlauf im Scharnier bleibt die Undo-Liste und wird nicht mit Git vermischt | C §6.2 |

## Nebenaufgaben 4.0 und das Projektformat (Gutachten C §8)

| Frage | Entscheidung | Grund |
|---|---|---|
| App-Symbol | keine Anforderung an das Projektformat | betrifft nur das Bundle |
| `.g4/preview.png` | bleibt, wo 4.0 sie anlegt (PNG RGB8 256 × 192, um 50 KB), versioniert, kein LFS, keine eigene Schema-Version | passt zur Erlaubnisliste |
| berechnetes Standardbild | gehört als Cache unter `.g4/cache/` (z. B. `.g4/cache/preview-standard.png`), nicht auf die oberste Ebene von `.g4/` | Regel „oberste Ebene = Projekt“; mit der Erlaubnisliste in jedem Fall ignoriert. Der Hinweis ging am 2026-10-26 an die 4.0-Sitzung; landet 4.0 mit `.g4/preview-standard.png`, zieht 4.2 die Datei um |

## Einordnung von 3d und Vorsorge für Phase 5 und 6 (Gutachten C §7, §9)

| Frage | Entscheidung | Grund |
|---|---|---|
| 3d (Erzählen, Flags und Variablen, Geschichten, Löschen mit Referenzsperre) | **nach Phase 4, vor Phase 5**; die Script-Referenzen im Referenzindex ziehen in 4.3b vor | Geschichten und eigene Flag-Namen sind neue Entitäten ohne ROM-Gegenstück und gehören ins Textprojekt; Löschen wird dort „Datei entfernen, Referenzen prüfen“ statt Nummern in NARCs zu verschieben; 3d vor Phase 5 testet das Entitätenmodell mit neuen Arten, bevor die Plugin-API einfriert. Die Flag- und Variablennamen aus pret gibt es schon (`metadata/scrcmd/*.json`) |
| Phase 5 (Plugin-Host) | Phase 4 hält offen: Arten ohne fest verdrahtetes Enum im Format (eingebaute Arten ohne Präfix, Plugins später mit Namensraum), `schemas` je Art und `plugins` im Manifest reserviert, Build-Graph mit offener Schrittliste (`Step`-Vertrag), Diff und Referenzindex auf deklarierten Feldern; der Ordner `plugins/` ist reserviert, Phase 4 lädt dort nichts | nichts verbauen; projektlokale Plugins brauchen eine Vertrauensabfrage (Phase 5) |
| Phase 6 (Kartenerstellung) | Entität = Manifest-JSON plus Beifiles; das Kartenmanifest nennt seine Quelle (`source: "binary"`, später `"terrain"`), der Graph kennt dann eine Stufe „Quelle → Kartendatei“ vor `unpacked/`; die `.gitattributes` lässt Platz für LFS-Zeilen | Phase 6 bricht das Format nicht |

## Nicht in Phase 4

- Commit-Oberfläche, Staging, Branches, Merge und Konflikte in Git, Pull, Push, Remotes, „Auf GitHub veröffentlichen“, „Stand sichern“ als Ein-Klick-Commit (später mit Veröffentlichen), Git LFS (Phase 6 prüft es).
- Plakette „geändert“ im Inspector-Kopf; semantischer Diff von Binär-Assets über „Teil geändert“ hinaus (Bildvergleich von Modellen und Texturen).
- Ein „Differenzprojekt“ (nur Abweichungen, teilbar) als eigenes Format; JSON-Schema-Dateien (`schemars`) für Editoren und Agenten.
- Vorhandene Script-Quellen auf symbolische IDs umschreiben.
- Zurücksetzen von ARM9-Patches, Overlays und Overlay-Tabellen (gehört zum Patch-Schutz, später).
- Projektlokale Plugins; 3d.

## Was bewusst nicht entschieden ist

- Ob DSPRE beim Speichern alle Ordner unter `unpacked/` zurück in ihre NARCs schreibt, auch die, die das Studio heute nicht packt (Hand in DSPRE; bestimmt die Dringlichkeit der Pack-Einträge in 4.2).
- Ob xdelta UI unter Windows die Patches ohne Zweitkompression anwendet (nicht geprüft, kein Windows); welche Fassungen von Rom Patcher JS gelten.
- Welche No-Intro-Prüfsummen in `metadata/base_roms.yaml` kommen (nur Prüfsummen, keine Daten); heute stehen dort die drei Testprojekt-Dumps.
- Ob zwei Item-IDs sich einen Item-Member teilen (nicht gemessen; vor dem Item-Schema in 4.3a prüfen).
- Ob PokeAPI-Ortsindizes für `PLACE_*` sprachfrei taugen (ungeprüft; bis dahin Taufe in der Projektsprache).
- Kalte Zeiten unter Windows mit Virenscanner (nicht gemessen); Gates setzen Grenzwerte nur auf warme Zeiten.
- Ein Upstream-Vorschlag an ds-rom (BLZ-Cache als `RomLoadOptions`) und an rotom (Quirks als Pragma in der Quelle).
