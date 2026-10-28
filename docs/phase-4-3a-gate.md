# Phase 4.3a: Gate prüfen

Stand: 2026-10-27. Branch `phase-4-3a-textprojekt` (Worktree `.claude/worktrees/phase-4-3a`, Basis `df7d6bf`), nicht nach main gemergt. Gate vom Agenten bestanden: (1) Automatik, (2) Szenario kopflos und (3) MCP auf dem Stand `49065c8` (Gate-Runde ab `824a11a`); drei Befunde der Gate-Runde (B1 bis B3) sind behoben, je mit eigenem Commit und Regressionstest. Unabhängige Abnahme am 2026-10-28 auf dem gemergten Stand `a6933b3` bestanden, vier Befunde (A1 bis A4) behoben in `9654750`, `af08608` und `0e0a94b`; A2 ändert die Taufe unbenannter Trainer, A1, A3 und A4 die Texte von Meldungen (Abschnitt „Abnahme (2026-10-28)“, „Für 4.3b“). Beim Nutzer offen: (4) Lesbarkeit der JSON und der Fehlermeldungen, Lizenzlage der PokeAPI-Tabellen und eine Zeitmessung ohne parallele Sitzung (Hand).

Phase 4.3a bringt das Textprojekt im Kernel, kopflos: `g4k text export`, `import`, `check`, `sources` und `migrate`, das Modul `textproject` mit eigenen serde-Typen, Manifest `g4-project.json`, je Entität eine JSON unter `data/<art>/NNNN.json` mit symbolischen IDs (Register `data/ids/`), Binäres nur bei Abweichung von der Basisschicht (`.g4/cache/base-layer/<sha1>/`, aus der eigenen Basis-ROM), nicht Abgebildetes als Blob unter `data/raw/`, die rotom-Quirks in `data/rotom-quirks.json`, die Stufe „Entität → Member“ (`textproject`) im Build-Graphen und über MCP `text_check` und `text_export`. Es gibt keine Oberfläche und keine Namen in der Oberfläche; Sichtbares kommt mit 4.3b. Roadmap: [docs/ROADMAP.md](ROADMAP.md), Abschnitt „Phase 4“, Punkt „4.3a Textprojekt im Kernel“. Entscheidungen: [entscheidungen.md](research/2026-10-27-phase-4/entscheidungen.md), Abschnitt „Textprojekt (Gutachten A, 4.3)“. Plan: [2026-10-27-phase-4-3a-textprojekt-kernel.md](superpowers/plans/2026-10-27-phase-4-3a-textprojekt-kernel.md). Format: [docs/formats/text_project.md](formats/text_project.md).

Gate-Text der Roadmap (Punkt 4.3a):

> Gate: auf Kopien von Platin DE, HeartGold DE und HeartGold US Export → Member löschen → Import → Byte-Vergleich aller Dateien → `g4k build` → ROM-SHA-1 gleich der Basis (`textproject_round_trip`); zweiter Export ändert keine Datei (`textproject_export_idempotent`, `git status` leer); Extract aus der Basis-ROM plus Textprojekt baut SHA-1-gleich (`textproject_from_base`); der Diff HeartGold DE gegen US betrifft nur Texte und vier Karten; Zeiten im Gate-Protokoll (Richtwert Export ≤ 20 s, Import ≤ 30 s je Projekt).

Wer was prüft: Die Spalte „Wer“ sagt **Agent** (die Test-Umgebung aus [docs/agent-gate.md](agent-gate.md): `g4k`, die gated Tests, `studio mcp` kopflos, die Brücke mit `call.sh` und `mcp.sh`), **Agent + Blick** (der Agent prüft und beschreibt, der Nutzer bestätigt kurz am Bild; in 4.3a gibt es nichts zu sehen) oder **Hand** (der Nutzer). Befehle: [docs/agent-gate.md](agent-gate.md), Abschnitt „Phase 4.3a“.

## Vorbereitung

So durchgeführt am 2026-10-27:

1. **Frische Kopien** unter `Roms/_testkopien/` per `cp -cR` (Ordner) und `cp -c` (`.nds`) aus den Originalen, die dabei nur gelesen wurden; Ordner und `.nds` nebeneinander, damit der Vorschlag der Basis-ROM greift:
   - `4-3a-pt` (Platin DE), `4-3a-hgde` (HeartGold DE), `4-3a-hgus` (HeartGold US) für (2);
   - `4-3a-mcp` (Platin DE) für (3);
   - beim Szenario angelegt: `4-3a-klon-{pt,hgde,hgus}` (je `repo/`, der Git-Klon der Kopie, und `fresh/`, das Extract der `.nds`).

   Die Kopien `4-3a-*` bleiben bis nach dem letzten Gate liegen und werden dann gelöscht.
2. **Werkzeuge:** `cargo build --release -p g4-kernel --bin g4k` (mit den Fixes B1 und B2 neu gebaut); `target/debug/studio` für `studio mcp`, `studio-bridge` baut `studio-web.sh` selbst. Aus dem Worktree `G4_ROMS_DIR` absolut und `tools/` als Symlink auf den Haupt-Checkout (nie einchecken).
3. **Last:** Auf dem Rechner liefen parallel andere Sitzungen (darunter das Gate von 4.1b) und Microsoft Defender (`wdavdaemon`, `epsext`, in einer Stichprobe zusammen rund 370 % CPU, während der Export Tausende Dateien las und schrieb). Jede Zeit unten steht mit dem Lastmittel (`uptime`, 1 Minute) daneben.
4. **Ausgaben** unter `target/agent/gate-2026-10-27/4-3a/` im Worktree: `pt-*.log`, `scenario-hgde.log`, `scenario-hgus.log` (Szenario), `migration.log`, `newer-manifest.log`, `diff-hgde-hgus.log`, `timings-{1,2,3}.log` und `timings-rawms.log` (Zeiten), `gated-release.log` (Messzeilen), `mcp-*.log` (MCP), dazu `target/agent/gate-sh-4-3a.log` (`scripts/gate.sh`). Die Skripte des Szenarios lagen im Scratchpad der Sitzung, nicht im Repo. Vor `git worktree remove` den Ordner `target/agent/` sichern.

## (1) Automatik

- [x] 1. `scripts/gate.sh` grün auf `49065c8`: „Gate: ok (6032 s)“, Log `target/agent/gate-sh-4-3a.log`, Last 6 bis 26. Schritte in Sekunden: fmt 2, clippy 7, deny 4, `cargo test` ohne ROM 576, mcp 33, `pnpm test` 115, tokens 1, styles 4, `pnpm build` 20, g4emu 17 + 8, gated `cargo test` HeartGold DE 2 317, HeartGold US 1 532, Platin DE 1 122, „build graph (timings)“ im ersten Lauf ok (80 / 75 / 42 s; HeartGold DE Karte 278, Text 295, Script 348, ohne Änderung 102 ms; HeartGold US 268, 253, 394, 91 ms; Platin DE 283, 244, 322, 98 ms), Projektstart 36 / 21 / 20. Darin laufen je Projekt die gated Tests `textproject_round_trip`, `textproject_export_idempotent`, `textproject_from_base` (`crates/g4-kernel/tests/textproject.rs`), `vanilla_base_layer_has_no_raw_blobs` und `vanilla_shared_item_members_are_counted` (`tests/textproject_vanilla_probe.rs`) mit. Ein erster Lauf auf `824a11a` wurde nach den Schritten bis `g4emu` (alle grün) angehalten, um die Befunde B1 bis B3 zu beheben; er zählt nicht.
- [x] 2. Einzeln je Projekt im Release-Profil für die Messzeilen (Original nur gelesen, alles Schreibende auf `vanilla_copy()`), Log `gated-release.log`:

```sh
export PATH="$HOME/.cargo/bin:/opt/homebrew/opt/rustup/bin:$PATH"
G4_TEST_PROJECT="<Roms>/Pokemon - Platin-Edition (Germany)_DSPRE_contents" \
  cargo test --release -p g4-kernel --all-features --test textproject --test textproject_vanilla_probe \
  -- textproject_round_trip textproject_export_idempotent textproject_from_base vanilla_ --nocapture --test-threads 1
```

Ebenso mit `"…Goldene Edition HeartGold (Germany)_DSPRE_contents"` und `"…HeartGold Version (USA)_DSPRE_contents"`. Je 5/5 grün:

| Spiel | Tests | `textproject-stats` (Release) | zweiter Export | `from_base` | `base-layer` | `items` | Last (1 min) |
|---|---|---|---|---|---|---|---|
| Platin DE | 5/5 | `files=6574 bytes=18459013 json=4717 side=0 raw=0 base_layer_ms=3843 export_ms=28895 import_ms=16272 build_ms=3156` | `ms=20609` | `extract_ms=257 import_ms=17719 build_ms=1520 equal` | `ms=6177 diff_ms=15077 files=18484 bytes=149748791 not_built=` | `ids=468 members=446 shared=22 unreferenced=0` | 3,7 bis 7,9 |
| HeartGold DE | 5/5 | `files=6302 bytes=18719385 json=4499 side=0 raw=0 base_layer_ms=7760 export_ms=43699 import_ms=32015 build_ms=9411` | `ms=47967` | `extract_ms=535 import_ms=54588 build_ms=6034 equal` | `ms=6819 diff_ms=21029 files=20028 bytes=182288113 not_built=` | `ids=537 members=514 shared=23 unreferenced=0` | 7,9 bis 10,5 |
| HeartGold US | 5/5 | `files=6302 bytes=18441867 json=4499 side=0 raw=0 base_layer_ms=11522 export_ms=75482 import_ms=19035 build_ms=7675` | `ms=39116` | `extract_ms=305 import_ms=33997 build_ms=7107 equal` | `ms=7427 diff_ms=18702 files=20028 bytes=181335326 not_built=` | `ids=537 members=514 shared=23 unreferenced=0` | 8,7 bis 14,3 |

Die Dauer je Projekt (Bau der Testprogramme inklusive): 347 s, 684 s, 554 s. JSON je Art wie in Task 12 gemessen: Header 593 / 540 / 540, Matrizen 289 / 288 / 288, Karten 666 / 676 / 676, Gebietsdaten 75 / 106 / 106, Events 534 / 491 / 491, Begegnungen 183 / 142 / 142, Spezies 508, Attacken 471, Items 468 / 537 / 537, Trainer 928 / 738 / 738, je eine Trainertext-Tabelle und -Offsets; die Binärarten (Kartentexturen, Gebäudetexturen, Gebäudemodelle, bei HeartGold auch Innenraum-Gebäudemodelle) haben keine Datei, weil nichts von der Basis abweicht.

Gesperrte Member (`"locked"`): keine auf allen drei Projekten. Blobs unter `data/raw/` und Beifiles: 0 auf allen drei; die Basisschicht ist außerhalb der abgebildeten Member gleich dem DSPRE-Ordner (`changed` leer, `not_built` leer), also gilt „0 Blobs“ ohne Ausnahmeliste (Punkt „Basisschicht gegen DSPRE-Ordner“ des Plans).

## (2) Szenario kopflos

Release-`g4k`, je Spiel auf seiner Kopie (`K=<Roms>/_testkopien`, `<k>` = `pt`, `hgde`, `hgus`, `P=$K/4-3a-<k>_DSPRE_contents`):

```sh
G=target/release/g4k
$G base assign $P $K/4-3a-<k>.nds                      # assigned sha1=… label=… (ohne Warnung)
$G text export $P                                      # kalt: adopted scripts=N, je Art eine Zeile, Summe
$G text export $P                                      # written=0 removed=0
cd $P && git init -q && $G text sources . | tr '\n' '\0' | xargs -0 git add && git commit -qm Ausgangsstand
$G text export .                                       # (seit B1 auch mit `.`)
git status --porcelain --untracked-files=no            # leer
$G text check .                                        # schema=current, differs=0, Exit 0
# data/species/0025.json: personal.attack 55 → 56
$G build . --out build/t.nds && $G check-data build/t.nds --expect species:25:attack=56
# Byte 3 (Stärke) von unpacked/moveData/0001 +1, wie DSPRE es schreibt
$G build . --out build/t.nds                           # Warning: … textproject.member-newer …
# data/moves/0001.json: data.power 40 → 45
$G build . --out build/t.nds                           # Exit 1, textproject.conflict; Member, JSON und ROM unverändert
# Klon-Weg
git clone -q $P $K/4-3a-klon-<k>/repo && $G extract $K/4-3a-<k>.nds $K/4-3a-klon-<k>/fresh
(cd $K/4-3a-klon-<k>/repo && git ls-files -z | cpio -0 -pdm $K/4-3a-klon-<k>/fresh)
$G text import $K/4-3a-klon-<k>/fresh --base-rom $K/4-3a-<k>.nds
$G build $K/4-3a-klon-<k>/fresh --out …/fresh/build/klon.nds && shasum …/klon.nds $K/4-3a-<k>.nds
python3 -I scripts/agent/textproject_diff.py $K/4-3a-klon-hgde/repo $K/4-3a-klon-hgus/repo
```

- [x] 1. **Zuordnen:** ohne Warnung, `d40f19ac…` (Platin DE), `13b6853a…` (HeartGold DE), `4fcded0e…` (HeartGold US).
- [x] 2. **Erster Export (kalt):** übernimmt die Scripts (`adopted scripts=1124` bzw. `965`), legt die Basisschicht an und schreibt 5 447 (Platin) bzw. 5 334 (HeartGold) Dateien; `raw=0`, `side=0` je Art, `texts=724` bzw. `829`. Zeiten unter „Zeiten“.
- [x] 3. **Zweiter Export:** `written=0 removed=0` auf allen drei Kopien.
- [x] 4. **Git:** der Commit „Ausgangsstand“ nimmt genau die Dateien aus `g4k text sources` auf (6 574 bei Platin, 6 302 bei HeartGold: die JSON unter `data/`, 4 Register, `data/rotom-quirks.json`, `g4-project.json`, `expanded/textArchives/`, `expanded/scripts/`, `rotom.toml`, `.rotom/command_database`, `.g4/base.json`). Ein Export danach: `git status --porcelain --untracked-files=no` leer auf allen drei. Bei Platin lehnte `g4k text export .` an dieser Stelle zuerst ab (Befund B1); nach dem Fix dort nachgeholt: `git checkout -- data/`, `g4k text import .` (`members=2`, die Änderungen aus 6 und 7 zurück), `g4k text export .` → `written=0 removed=0`, `git status --porcelain --untracked-files=no` leer, `check` → `current` (`pt-git-after-fix.log`); HeartGold DE und US liefen mit absolutem Pfad. Das wörtliche `git status --porcelain` zeigt 14 Zeilen `??` (15 mit `build/` nach dem Bauen), alle außerhalb des Textprojekts (`arm9/`, `files/`, `unpacked/`, `.g4/cache/`, `.g4/build/`, `header.yaml`, `.DS_Store`, …): die Vorlage `.gitignore` kommt erst mit 4.4. Keine Zeile betrifft eine Quelle.
- [x] 5. **`g4k text check`:** `schema=current`, `differs=0`, keine Probleme, Exit 0 auf allen drei.
- [x] 6. **Spezies bearbeiten und bauen:** `git diff --stat` 1 Datei, `g4k build` schreibt den Member (`packed …personalPokeData=508…`), `g4k check-data build/t.nds --expect species:25:attack=56` → „species 25 attack: 56“, „1 expectation(s) met“, auf allen drei.
- [x] 7. **Member neuer:** nach dem Byte in `unpacked/moveData/0001` (`0028` → `0029`) baut `g4k build` mit `Warning: data/moves/0001.json:1:1: textproject.member-newer: unpacked/moveData/0001 ist neuer als data/moves/0001.json; g4k text export übernimmt ihn, g4k text import verwirft ihn.`, Exit 0, auf allen drei.
- [x] 8. **Konflikt:** danach `data/moves/0001.json` geändert: `Error: data/moves/0001.json:1:1: textproject.conflict: data/moves/0001.json und unpacked/moveData/0001 sind beide geändert; nichts geschrieben.`, Exit 1; die SHA-1 von Member (`75847c83…`), JSON (`274d28f3…`) und `build/t.nds` sind vorher und nachher gleich, auf allen drei. Die Summenzeile hieß dabei „1 problem(s) in scripts or texts“ (Befund B2).
- [x] 9. **Klon-Weg:** `g4k extract` der `.nds`, die Dateien des Commits darüber, `g4k text import --base-rom …` (`members=0 raw=0 quirks=4 compiled=1124 encoded=724` bei Platin, `quirks=1 compiled=965 encoded=829` bei HeartGold), `g4k build`: SHA-1 gleich der Basis auf allen drei (`d40f19ac…`, `13b6853a…`, `4fcded0e…`).
- [x] 10. **Diff HeartGold DE gegen US** (`scripts/agent/textproject_diff.py` auf den Klonen, Stand „Ausgangsstand“):

  ```
  data/maps/0468.json
  data/maps/0473.json
  data/maps/0475.json
  data/maps/0476.json
  data: 4 of 4500 files differ (A 4500, B 4500; data/ids not compared)
  expanded/textArchives: 829 of 829 files differ (A 829, B 829)
  ```

  Genau die Erwartung: vier Karten und alle 829 Textarchive. Ohne das Ersetzen der IDs durch ihre Nummern unterscheiden sich byteweise zusätzlich 418 Header und 735 Trainer (`PLACE_*` und `TRAINER_*` sind in der Projektsprache getauft); dafür gibt es das Skript.
- [x] 11. **Migration** (auf `4-3a-klon-pt/fresh`): das exportierte Projekt in die Form des Fixtures `tests/fixtures/textproject/v0` gebracht (Manifest `"schema": 0`, jedes Register als `{"ids": {"NNNN": "ID"}}`). `g4k text check` → `schema=older 0`, „error: Das Textprojekt hat Schema-Version 0, diese Fassung kennt 1: `g4k text migrate` ausführen.“, Exit 1; `g4k text migrate` → `migrated 0 -> 1 changed=5 snapshot=./.g4/backup/textproject-v0-2026-10-27T195429Z` (Manifest und vier Register im Schnappschuss); danach `schema=current`, `differs=0`, Manifest und Register byte-gleich zum Export vor dem Umbau.
- [x] 12. **Neueres Manifest** (`"schemaVersion": 2`): `check` → `schema=newer 2`, Exit 1; `migrate`, `export` und `import` → „error: Das Textprojekt stammt aus einer neueren Fassung von Gen4 Studio (Schema-Version 2).“, Exit 1; `g4k build` → `Error: g4-project.json:1:1: textproject.schema: …` mit demselben Satz, Exit 1. Nichts geschrieben, kein neuer Schnappschuss.

## (3) MCP

Kopie `4-3a-mcp` (Platin DE), vorher `g4k base assign` mit der Nachbar-`.nds`. Kopflos über `studio mcp --project … [--allow-irreversible]` (ein kleiner Python-Client über stdio im Scratchpad, wie die Handprobe in [dev-setup.md](dev-setup.md), „MCP“); über die Brücke mit `G4_VITE_PORT=1550 G4_BRIDGE_PORT=1551`, Seite `http://127.0.0.1:1550/?project=…/4-3a-mcp_DSPRE_contents` im Browser-Fenster, `call.sh agent_set_enabled '{"on":true}'`, Freigabe „Lesen und bearbeiten“ (`agent_set_grant '{"level":"write"}'`).

- [x] 1. **Kopflos, vor dem Export:** `text_check` → `{"schema": {"state": "missing"}, "problems": [], "differs": []}`; `text_export` ohne `--allow-irreversible` → `needs_confirmation` („… (Textprojekt exportieren); kopflos nur mit --allow-irreversible“); mit `--allow-irreversible` → `not_adopted` („Die Scripts sind noch nicht übernommen (adopt_scripts).“); nach `set_matrix_cell` (ungespeichert) → `unsaved` („Erst speichern: 1 ungespeicherte Änderungen.“). Nichts geschrieben (`data/` fehlt).
- [x] 2. **Brücke, `text_check`:** `missing`.
- [x] 3. **Brücke, `text_export` vor dem Übernehmen der Scripts:** Dialog „Agent möchte etwas Unumkehrbares tun“ mit „„agent-gate“ möchte: Textprojekt exportieren (lässt sich nicht rückgängig machen).“; nach „Zulassen“ kam `not_adopted` (Befund B3). Seit `49065c8` kommen `unsaved`, `base_missing` und `not_adopted` ohne Frage zurück. Der allererste Aufruf der Sitzung kam sofort mit `needs_confirmation` („… in Gen4 Studio abgelehnt“) zurück, ohne dass der Agent geklickt hatte (Hinweis H3).
- [x] 4. **Brücke, Export:** Scripts über die Oberfläche übernommen (`call.sh adopt_scripts` → `decompiled: 1124`); `mcp.sh call text_export '{}'` im Hintergrund → Frage `{"kind": "confirm", "what": "Textprojekt exportieren"}` (`agent_status`), Dialog wie oben; `call.sh agent_answer '{"question":3,"answer":"allow"}'` → Bericht mit 5 447 geschriebenen Dateien, `raw_blobs: 0`, `texts: 724`, Basisschicht neu (19,3 s), 94,8 s gesamt (Debug-Brücke, Last 6 bis 15), Exit 0. Danach `text_check` → `current`, keine Probleme, `differs: []`.
- [x] 5. **Brücke, ungespeichert (auf `49065c8`):** `set_matrix_cell` über MCP, dann `text_export` → `unsaved` („Erst speichern: 1 ungespeicherte Änderungen.“) sofort, ohne Frage (`agent_status` `question: null`); `undo`.
- [x] 6. **Brücke, zweiter Export:** Dialog, Klick auf „Zulassen“ im Browser-Fenster → `written: 0`, `removed: 0`, 13,9 s (Last 4,4); `text_check` → `current`.
- [x] 7. **Kopflos, danach:** `text_check` → `current`; `text_export` mit `--allow-irreversible` → `written: 0`, 12,7 s (Last 4,4).

## (4) Hand und Blick

Kein Blick: 4.3a hat keine Oberfläche. Hand beim Nutzer:

1. In einer Kopie (etwa `4-3a-pt_DSPRE_contents`, solange sie liegt; sonst nach `g4k base assign` und `g4k text export` auf einer frischen Kopie) `data/species/0025.json`, `data/maps/0003.json` und `data/trainers/0001.json` lesen: sind Feldnamen und IDs (`SPECIES_PIKACHU`, `MOVE_THUNDER_SHOCK`, `MAP_…`, `TRAINER_…`) verständlich?
2. Die Lizenzlage der Symboltabellen bestätigen: `crates/g4-kernel/metadata/symbols/` aus PokeAPI (Commit `2bf639e5`, BSD-3-Clause, `LICENSE-PokeAPI.txt` daneben).
3. Seit der Abnahme (2026-10-28): die Fehlermeldungen von `g4k text check` lesen, etwa die 15 Zeilen in `target/agent/gate-2026-10-28/abnahme-4-3a/check-broken-after-fix.log` im Worktree `phase-abn-4-3a` (`matrix_id: die Zahl 70000 passt nicht in u16 (0 bis 65535).`, `zzz_unbekannt: unbekannter Schlüssel (erlaubt: `id`, `data`).`, `kein gültiges JSON: ein Schlüssel muss ein Text in Anführungszeichen sein.`): verständlich für jemanden, der eine JSON von Hand bearbeitet hat?
4. Zeiten ohne Last: Die Abnahme lief bei Last 12 bis 31 (4.3b und Fix-Agenten parallel). Auf einer frischen Kopie ohne andere Sitzung `g4k text export` zweimal und `g4k text import` einmal mit `/usr/bin/time -p` messen und mit den Richtwerten (Export ≤ 20 s, Import ≤ 30 s) vergleichen.

Stand der Abnahme (2026-10-28): 1 bis 4 offen. Die Testkopien der Abnahme sind gelöscht; für 1 und 3 eine frische Kopie nach `g4k base assign` und `g4k text export` nehmen. Ein Blick entfällt weiter (keine Oberfläche in 4.3a); der Dialog „Agent möchte etwas Unumkehrbares tun“ mit „Textprojekt exportieren“ ist Oberfläche aus 3h und sah in der Abnahme aus wie im Gate beschrieben.

## Ergebnis

| Punkt | Wer | Ergebnis | Datum |
|---|---|---|---|
| (1) 1 `scripts/gate.sh` | Agent | bestanden auf `49065c8`, 6 032 s; „build graph (timings)“ ohne Wiederholung grün | 2026-10-27 |
| (1) 2 gated Tests einzeln (Release) | Agent | bestanden: je 5/5 auf Platin DE, HeartGold DE, HeartGold US; Round-Trip und `from_base` byte-gleich, zweiter Export schreibt nichts, 0 Blobs, 0 Beifiles, nichts gesperrt (Messtabelle) | 2026-10-27 |
| (2) 1 bis 5 Zuordnen, Export, zweiter Export, Git, Check | Agent | bestanden auf allen drei Kopien; `git status` leer für die Quellen (`--untracked-files=no`), die übrigen 14 Zeilen sind `??` außerhalb des Textprojekts (`.gitignore` ist 4.4) | 2026-10-27 |
| (2) 6 Spezies bearbeiten, `check-data` | Agent | bestanden: `species 25 attack: 56` auf allen drei | 2026-10-27 |
| (2) 7 und 8 `member-newer`, `conflict` | Agent | bestanden: Hinweis, dann Exit 1 mit `textproject.conflict`, Member, JSON und ROM unverändert | 2026-10-27 |
| (2) 9 Klon-Weg | Agent | bestanden: SHA-1 gleich der Basis auf allen drei | 2026-10-27 |
| (2) 10 Diff HeartGold DE gegen US | Agent | bestanden: `data/maps/0468`, `0473`, `0475`, `0476` und 829 Textarchive | 2026-10-27 |
| (2) 11 und 12 Migration, neueres Manifest | Agent | bestanden: `older 0` → `migrate` mit Schnappschuss → `current`; `schemaVersion: 2` von `check`, `migrate`, `export`, `import` und `build` abgelehnt | 2026-10-27 |
| (3) MCP kopflos und über die Brücke | Agent | bestanden nach B3: `missing`, `needs_confirmation`, `not_adopted`, `unsaved`, Export mit Frage „Textprojekt exportieren“, `current` | 2026-10-27 |
| (4) Lesbarkeit von `data/species/0025.json`, `data/maps/0003.json`, `data/trainers/0001.json` | Hand | offen | – |
| (4) Lizenzlage der PokeAPI-Tabellen | Hand | offen | – |
| (4) Lesbarkeit der Fehlermeldungen, Zeiten ohne Last | Hand | offen | – |
| Abnahme (unabhängig) | Agent | bestanden nach A1 bis A4 (`9654750`, `af08608`, `0e0a94b`): Export, Idempotenz, Import nach Löschen, Build SHA-1-gleich, Klon-Weg, Gegenprobe DE/US auf frischen Kopien aller drei Spiele; je Art eine Änderung im DSPRE-Layout bis in die JSON und zurück; MCP kopflos und über die Brücke; siehe Abschnitt „Abnahme (2026-10-28)“ | 2026-10-28 |

### Messtabelle

Dateien und Größe: die Quellen des Textprojekts (`g4k text sources`); Beifiles und Blobs: Vanilla. Basisschicht anlegen: `base-layer … ms` aus `vanilla_base_layer_has_no_raw_blobs` (Release). Export warm und zweiter Export: Release-`g4k` auf den Kopien von (2), Basisschicht vorhanden, je zwei bis drei Läufe; „Export warm“ ist ein voller Export, nachdem `data/` und `g4-project.json` aus dem Projekt genommen wurden (das Ergebnis war jedes Mal byte-gleich zum vorigen `data/`). Import und Build danach: `textproject_round_trip` (Release; alle Member mit Entität, Text- und Script-Member gelöscht, Header-Tabelle und interne Namen genullt, Basisschicht vorhanden). Geteilte Item-Member: `vanilla_shared_item_members_are_counted`.

| Spiel | Dateien | Größe | Beifiles | Blobs | Basisschicht anlegen (s) | Export warm (s) | Zweiter Export (s) | Import (s) | Build danach (s) | Round-Trip | from_base SHA-1 gleich | geteilte Item-Member |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Platin DE | 6 574 (4 717 JSON) | 18 459 013 B (17,6 MiB) | 0 | 0 | 6,2 (Schicht 18 484 Dateien, 150 MB) | 5,6–6,1 | 4,3–22,9 (gated 20,6) | 16,3 | 3,2 | byte-gleich, SHA-1 `d40f19ac…` | ja (Import 17,7, Build 1,5) | 22 (468 IDs, 446 Member) |
| HeartGold DE | 6 302 (4 499 JSON) | 18 719 385 B (17,9 MiB) | 0 | 0 | 6,8 (20 028 Dateien, 182 MB) | 7,0–19,8 | 23,3–24,1 (gated 48,0) | 32,0 | 9,4 | byte-gleich, SHA-1 `13b6853a…` | ja (Import 54,6, Build 6,0) | 23 (537 IDs, 514 Member) |
| HeartGold US | 6 302 (4 499 JSON) | 18 441 867 B (17,6 MiB) | 0 | 0 | 7,4 (20 028 Dateien, 181 MB) | 20,6–21,5 | 23,9–27,0 (gated 39,1) | 19,0 | 7,7 | byte-gleich, SHA-1 `4fcded0e…` | ja (Import 34,0, Build 7,1) | 23 (537 IDs, 514 Member) |

Lastmittel: Export warm und zweiter Export mit `g4k` 3,3 bis 6,5; die gated Werte (Import, Build, `from_base`, „gated“) 3,7 bis 14,3 (Tabelle oben).

### Zeiten

Richtwerte (warm, nur protokolliert): Export ≤ 20 s, Import ≤ 30 s je Projekt.

| Messung | Platin DE | HeartGold DE | HeartGold US | Last (1 min) |
|---|---|---|---|---|
| erster Export, kalt (Scripts übernehmen, Basisschicht anlegen), `g4k` | 114,5 s (Basisschicht 7,7 s) | 83,0 s (11,6 s) | 150,0 s (18,4 s) | 13 bis 31 |
| zweiter Export im Szenario, `g4k` | 121,8 s | 40,1 s | 79,5 s | 15 bis 35 |
| zweiter Export, `g4k`, drei bis vier Läufe | 22,0 / 4,3 / 6,5 / 22,9 s | 24,1 / 23,7 / 23,5 / 23,3 s | 27,0 / 23,9 / 24,0 s | 3,3 bis 6,5 |
| davon Vergleich mit der Basisschicht (`raw_ms`) und Arten | 11,0 s und 10,9 s | 12,3 s und 10,1 s | – | 4,0 bis 4,9 |
| voller Export warm (`data/` vorher entfernt), `g4k` | 5,6 / 6,1 s | 7,0 / 19,8 s | 21,5 / 20,6 s | 4,0 bis 5,9 |
| Import ohne etwas zu schreiben, `g4k` | 3,0 / 8,0 s | 8,2 / 6,7 s | 6,8 / 7,4 s | 4,2 bis 5,6 |
| Import nach dem Löschen der Member (gated `textproject_round_trip`, Release) | 16,3 s | 32,0 s | 19,0 s | 3,7 bis 14,3 |
| Import im Klon-Weg (Basisschicht neu, `g4k`) | 60,8 s | 69,4 s | 35,6 s | 19 bis 32 |
| Import `from_base` (gated, Release) | 17,7 s | 54,6 s | 34,0 s | 3,7 bis 14,3 |
| `g4k text check` | 1,0 / 2,0 s | 0,9 / 2,8 s | 1,0 / 3,0 s | 3,8 bis 7,7 |
| über MCP: Export kalt (Debug-Brücke) / zweiter Export (Brücke, kopflos) | 94,8 s / 13,9 s, 12,7 s | – | – | 6 bis 15 / 4,4 |

Bewertung: Unter Last (13 bis 35) liegen alle Zeiten weit über den Richtwerten; die Prozesse brauchen dabei wenig CPU (`/usr/bin/time`: erster Export 1,1 s user und 5,6 s sys bei 115 s Wanduhr), sie warten auf die Dateizugriffe, die Microsoft Defender prüft. Bei Last 3 bis 7 gilt:

- **Export ≤ 20 s:** Platin hält ihn (4 bis 23 s, meist darunter), HeartGold verfehlt ihn knapp: zweiter Export 23 bis 27 s, voller Export warm 7 bis 22 s. Gut die Hälfte ist der Vergleich der Wurzel mit der Basisschicht (`raw_ms` 11 bis 12 s über 18 484 bzw. 20 028 Dateien), der Rest das Lesen und Vergleichen der Arten (10 bis 11 s). Die Streuung (Platin 4 bis 23 s im selben Zustand) kommt vom Dateicache und vom Virenscanner, nicht vom Code.
- **Import ≤ 30 s:** gehalten für Platin (Round-Trip 16 s, `from_base` 18 s, Last 4 bis 8). HeartGold verfehlt ihn unter höherer Last: HeartGold US Round-Trip 19 s, aber `from_base` 34 s (Last 9 bis 14); HeartGold DE 32 s (Round-Trip) und 55 s (`from_base`) bei Last 8 bis 10. Ein Import, der nichts schreibt, braucht 3 bis 8 s.

Nach dem Ruling der Gate-Runde wird nicht parallelisiert; das entscheidet 4.3b (Gutachten §2.4 „seriell“). Gemessen sind die Zeiten neu, das Ergebnis steht hier mit der Last.

## Befunde

Befunde der Gate-Runde, alle behoben, je eigener Commit mit Regressionstest (RED vor dem Fix), voller Prüflauf vor jedem Commit (`cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features` ohne `G4_TEST_PROJECT`/`G4_TEST_ROM`, `cargo deny check licenses advisories`, `pnpm test`, `pnpm build`):

| Nr. | Befund | Fix |
|---|---|---|
| B1 | `g4k text export .` aus dem Projektordner heraus (der Git-Ablauf des Szenarios) lehnte mit „Basis-ROM nicht gefunden: Platin DE (Testprojekt) (d40f19ac…).“ ab; `g4k base check .` zeigte `base=missing`. `start::suggest_base` nahm den Stamm aus `root.file_name()`, und `.` hat keinen. | `4e65706`: Eine Wurzel ohne letzten Namen (`.`, `…/..`) wird zuerst aufgelöst, jede andere bleibt, wie sie ist. Tests `suggest_base_resolves_a_root_without_a_folder_name` und `cli_base_check_finds_the_suggestion_from_inside_the_project`. |
| B2 | Ein abgelehnter Build oder Import des Textprojekts endete mit der Summenzeile „error: 1 problem(s) in scripts or texts“, auch bei `data/moves/0001.json` oder `g4-project.json`: seit 4.3a meldet das Textprojekt seine Probleme als `KernelError::Compile`. | `a86e8d3`: „… problem(s) in scripts, texts or the text project“; Test `error_messages_name_asset_and_path`. |
| B3 | Über die Brücke fragte `text_export` „Textprojekt exportieren“ und antwortete erst nach „Zulassen“ mit `not_adopted`: der Nutzer ließ einen Schritt zu, der dann nichts tat (wie 4.1a A3 bei `assign_base`). | `49065c8`: `check_text_export` hält die Ablehnungen, die keinen Export brauchen (`unsaved`, `base_missing`, `not_adopted`); die Frage der App ruft es vorher, `text_export_impl` beginnt damit. Test `shared_text_export_refuses_before_asking` (ersetzt `shared_text_export_asks_then_refuses_unadopted_scripts`); [docs/mcp.md](mcp.md) nennt es. |

Hinweise ohne Fix:

- **H1 Zeiten und Last:** Siehe „Zeiten“. Parallelisieren ist eine Entscheidung von 4.3b (Ruling: neu messen, dokumentieren).
- **H2 Ordner in der Wurzel werden Blobs:** Beim ersten Messlauf lag die Sicherung `data.prev/` (und `g4-project.json.prev`) in der Projektwurzel: der Export nahm ihre 4 723 bzw. 4 505 Dateien als Blobs unter `data/raw/` auf, der Import legte sie wieder an. Das ist die Regel des Plans („Bereich der Blobs: alle Dateien der Wurzel außer …“), aber eine Falle für Nutzer, die neben `data/` sichern; die Vorlage `.gitignore` (4.4) und der Hinweis „Textprojekt veraltet“ (4.3b) sollten sie nennen. Die Messung wurde mit der Sicherung außerhalb des Projekts wiederholt (`timings-2.log`, `timings-3.log`).
- **H3 Erste Frage ohne Klick abgelehnt:** Der erste `text_export` über die Brücke, rund 20 s nach dem Laden der Seite, kam sofort mit `needs_confirmation` „… in Gen4 Studio abgelehnt“ zurück; der Agent hatte nicht geklickt. Der Dialog `AgentQuestion` antwortet `dismiss`, sobald er sich ohne Taste schließt (Escape, Klick daneben). Die nächsten drei Fragen der Sitzung standen offen, bis sie beantwortet wurden; nicht nachgestellt. Kein Code von 4.3a.
- **H4 Byte-Zahl:** Ein Export der Kopie `4-3a-mcp` meldet 1 Byte mehr als der von `4-3a-pt` (18 459 014 statt 18 459 013); einzige Abweichung der Quellen ist `name = "4-3a-mcp"` statt `"4-3a-pt"` in `rotom.toml` (der Name des Projektordners). Ebenso HeartGold DE und US im Szenario gegenüber den gated Tests (Name der Temp-Kopie). Kein Befund.

## Abweichungen vom Plan-Szenario

- `git status --porcelain` ist ohne `.gitignore` nicht leer (14 Zeilen `??`, alle außerhalb des Textprojekts); geprüft ist `git status --porcelain --untracked-files=no` leer und dass keine `??`-Zeile eine Quelle ist. Die Vorlage `.gitignore` ist 4.4.
- Die Migration lief auf dem exportierten Klon `4-3a-klon-pt/fresh`, umgebaut in die Form des Fixtures `v0` (wie `cli_text_migrate_v0_fixture`); das Fixture allein ist kein Projekt.
- MCP: `text_export` über die Brücke braucht übernommene Scripts; sie wurden über die Oberfläche (`call.sh adopt_scripts`) übernommen, wie es die App als eigenen Schritt vorsieht (Ruling Task 13).

## Abnahme (2026-10-28)

Unabhängige Abnahme durch eine zweite Sitzung im Worktree `.claude/worktrees/phase-abn-4-3a` auf dem Branch `abnahme-4-3a`, auf dem gemergten Stand `a6933b3` (4.3a mit 4.0, 4.1a, 4.2 und 4.1b auf main), danach die Fixes unten. Release-`g4k` (gebaut auf `a6933b3`) für alle Zeiten, Debug-`studio mcp` für MCP kopflos, die Test-Umgebung auf 1590/1591 (`G4_ROMS_DIR` absolut, Seite im Browser-Fenster) für MCP über die Brücke. Ausgaben unter `target/agent/gate-2026-10-28/abnahme-4-3a/` (`s1-<k>.log` Export und Git, `s2-<k>.log` Round-Trip, `s3-<k>.log` und `s5-<k>.log` Änderungen je Art, `s4-<k>-import-back.log` Import zurück, `script-<hg>.log`, `blobs-pt*.log`, `klon-<k>.log`, `diff-hgde-hgus.log`, `check-broken.log`, `migration.log`, `schema-paths.log`, `mcp-headless-*.log`, `mcp-bridge-*.log`/`.out`); die Skripte lagen im Scratchpad der Sitzung.

**Last:** Während der ganzen Abnahme baute die 4.3b-Sitzung parallel, ab der Mitte liefen dazu bis zu drei Fix-Agenten dieser Sitzung mit `cargo test`; das Lastmittel (1 min) lag zwischen 4,5 und 25, meist 12 bis 20. Jede Zeit unten steht mit ihrer Last. Unter dieser Last sind die Zeiten kein Maß für den Code (siehe H1).

**Kopien** frisch unter `Roms/_testkopien/` (`cp -cR`, `cp -c`): `4-3a-abn-{pt,hgde,hgus}` (Szenario), `4-3a-abn-mcp` (Platin DE, MCP), beim Szenario angelegt `4-3a-abn-<k>-ref` (Vergleichsstand vor dem Löschen) und `4-3a-abn-klon-<k>/{repo,fresh}` (Klon des Commits „Ausgangsstand“ und Extract der `.nds`). Je Kopie liegt die `.nds` daneben, `g4k base assign` ohne Warnung.

### Punkte

| Punkt | Projekt | Ergebnis |
|---|---|---|
| Zuordnen, kalter Export | `abn-{pt,hgde,hgus}` | bestanden: `assigned sha1=d40f19ac…` / `13b6853a…` / `4fcded0e…` ohne Warnung; der erste Export übernimmt die Scripts (`adopted scripts=1124` / `965` / `965`), legt die Basisschicht an und schreibt 5 447 / 5 334 / 5 334 Dateien, `raw=0`, `side=0` je Art |
| Idempotenz | alle drei | bestanden: zweiter und dritter Export `written=0 removed=0`; nach `git init` und dem Commit „Ausgangsstand“ (genau die Dateien aus `g4k text sources`: 6 574 / 6 302 / 6 302) ein weiterer Export und `git status --porcelain --untracked-files=no` leer; `g4k text check` → `schema=current`, `differs=0` |
| Vanilla ohne Blobs | alle drei | bestanden: `data/raw/` fehlt, 0 Beifiles, keine Datei unter `data/<art>/` außer JSON |
| Import nach Löschen der Member | alle drei | bestanden: alle Member der abgebildeten Archive, Texte und Scripts gelöscht (`maps`, `matrices`, `areaData`, `eventFiles`, `encounters`, `personalPokeData`, `learnsets`, `evolutions`, `moveData`, `itemData`, `trainerProperties`, `trainerParty`, `trainerTextTable`, `trainerTextOffset`, Texturen, Gebäudemodelle, `textArchives`, `scripts`), die Header-Tabelle in `arm9.bin` (Platin ab 0xE6074, 593 × 24 Byte; HeartGold DE 0xF6B94, US 0xF6BE0, je 540 × 24) und `mapname.bin` genullt; `g4k text import` (`members=6783` / `6464` / `6464`, `compiled=1124` / `965`, `encoded=724` / `829`): `unpacked/`, `arm9.bin` und `mapname.bin` byte-gleich zum Stand davor; ein zweiter Import schreibt nichts |
| Build gleich der Basis | alle drei | bestanden: `g4k build` nach dem Import, SHA-1 gleich der Basis (`d40f19ac…`, `13b6853a…`, `4fcded0e…`) |
| Änderung je Art im DSPRE-Layout, Export | alle drei | bestanden: je ein Byte in Karte 0003 (Kollision der Zelle 0,0), Header 0003 in `arm9.bin` (Byte 10 des Eintrags), der ersten Eventdatei mit Warp (`x` von Warp 0), Trainer 0001 (Level des ersten Pokémon), Spezies 0025 (Initiative), der ersten Begegnung mit Gras (Level von Slot 0), einer Matrix (Platin: Name von 0001 `single` → `singme`; HeartGold: Zelle 14 von 0000, Karte 208 → 209), Area-Data 0001 (`light_type`); Textarchiv 0010 als neuerer Member (über einen Build kodiert, dann zurückgelegt); Script `0003.rotom` (Platin `Message 0` → `1`, HeartGold `NPCMsg 3` → `4`). `g4k text check` nennt die geänderten Member unter `differs`, der Export schreibt genau eine JSON je Art (`git diff` je eine Zeile: `collision`-Zelle `0080` → `0000`, `music_day_id` bzw. `text_archive_id`, `x`, `level`, `speed`, `grass_00_level`/`grass_level_00`, `name`/`maps`, `light_type`, der Text in `expanded/textArchives/0010.json`); das Script bleibt Quelle, `g4k build` kompiliert es (`compiled=[3]`) und der Script-Member ändert sich |
| JSON gegen das Format | alle drei | bestanden: jeder Schlüsselpfad aller exportierten JSON (Platin 129 000, HeartGold je 128 000 Werte) steht in `schema.snap` (`schema-paths.log`; die übrigen Zeilen dort sind leere Listen und Teile `base_sha1`, die das Prüfskript nicht auflöst); symbolische IDs wie beschrieben (`SPECIES_PIKACHU`, `TYPE_ELECTRIC`, `ITEM_ORAN_BERRY`, `MAP_C01`, `PLACE_JUBELSTADT`, `TRAINER_TEENAGER_GUNTHER`, `TRAINER_CLASS_TEENAGER`), Umlaute deutsch (`TRAINER_GOERE_NADINE`, `PLACE_BIZARRE_HOEHLE`), Register sortiert ohne doppelte Nummer; Suffixe bei Namenskollisionen in Nummernreihenfolge (Platin 432 Trainer mit `_2` …, etwa `TRAINER_ANGLER_JAKOB_2`, `_3`; 34 Trainerklassen wie `TRAINER_CLASS_ASS_TRAINER_2` bis `_4`); Trainer 0 („-“) hieß `TRAINER_PKMN_TRAINER` (A2); nach dem Fix tauft ein neuer Export ihn `TRAINER_0000` (`a2-after-fix.log`, Register auf `klon-pt/fresh` neu angelegt) |
| Import zurück und Build | alle drei | bestanden: nach den Änderungen Member erneut gelöscht, Import, byte-gleich zum geänderten Stand, `g4k build`, `g4k check-data … --expect species:25:speed=91 --expect encounter:<n>:grass0_level=<l>` → „2 expectation(s) met“; SHA-1 der ROM weicht von der Basis ab, wie es soll |
| Blob und Beifiles | `abn-pt` | bestanden: `unpacked/pokeHeight/0005` (nicht abgebildet), Textur `mapTextures/0001` und das Modell von Karte 0010 (31 956 Byte) je ein Byte geändert; Export `raw=1`, `data/raw/unpacked/pokeHeight/0005`, `data/map_textures/0001.nsbtx`, `data/maps/0010.nsbmd` mit `"model": {"file": "0010.nsbmd"}`; nach Löschen (auch der Blob-Datei) und Import byte-gleich (`raw=1`) |
| Klon-Weg (`from_base`) | alle drei | bestanden: `git clone` des Commits „Ausgangsstand“, `g4k extract` der `.nds`, die Dateien darüber, `g4k text import --base-rom`, `g4k build`: SHA-1 gleich der Basis auf allen drei |
| Gegenprobe HeartGold DE gegen US | Klone | bestanden: `data/maps/0468`, `0473`, `0475`, `0476` und 829 von 829 Textarchiven, sonst nichts (wie im Gate) |
| Migration v0 | `abn-pt` | bestanden: Manifest `"schema": 0` und Register `{"ids": {"NNNN": "ID"}}` wie das Fixture; `check` → `schema=older 0` mit „Das Textprojekt hat Schema-Version 0, diese Fassung kennt 1: `g4k text migrate` ausführen.“, Exit 1; `export` lehnt mit demselben Satz ab, `g4k build` mit `textproject.schema` und demselben Satz; `migrate` → `migrated 0 -> 1 changed=5 snapshot=./.g4/backup/textproject-v0-2026-10-28T000955Z` (Manifest und vier Register); danach `current`, `git status` leer (byte-gleich zum Export), ein zweites `migrate` → `already current` |
| Neuere Schema-Version | `abn-pt` | bestanden: `schemaVersion: 2` → `check` `schema=newer 2`; `check`, `import`, `export`, `migrate`: „Das Textprojekt stammt aus einer neueren Fassung von Gen4 Studio (Schema-Version 2).“, Exit 1; `build` mit `textproject.schema` und demselben Satz, nichts geschrieben |
| `text check` mit kaputten Dateien | `abn-pt` | bestanden nach A1 und A4 (`check-broken-after-fix.log`): unbekanntes Feld, falsche ID (`unknown-id`, `id-mismatch`), fehlendes Beifile, doppelte Nummer im Register, Zahl außerhalb des Feldes, Text statt Zahl, kein JSON; je Datei, Zeile, Spalte und Code; `import` und `build` lehnen mit denselben Zeilen ab und schreiben nichts. Vor den Fixes kamen die Fehler aus serde-Typen englisch und ohne JSON-Pfad (A1), das fehlende Beifile mit englischem Betriebssystem-Text (A4) |
| MCP kopflos | `abn-mcp` | bestanden: `text_check` → `missing`; `text_export` ohne `--allow-irreversible` → `needs_confirmation`; mit ihm ohne Basismarke → `base_missing` „Diesem Projekt ist keine Basis-ROM zugeordnet.“; mit Marke, aber ohne `.nds` → `base_missing` „Basis-ROM nicht gefunden: Platin DE (Testprojekt) (d40f19ac…).“; mit `.nds` → `not_adopted`; nach `set_matrix_cell` → `unsaved` („Erst speichern: 1 ungespeicherte Änderungen.“, A3; nach dem Fix „… 1 ungespeicherte Änderung.“, `a3-after-fix.log`), `undo`; `g4_call adopt_scripts` (mit `--expert`) → `decompiled: 1124`, dann `text_export` → Bericht, 5 447 Dateien, 39,5 s |
| MCP über die Brücke | `abn-mcp` | bestanden: `text_check` → `current`, `differs: ["unpacked/personalPokeData/0001"]` (vorher im DSPRE-Layout geändert); `unsaved` ohne Frage (`question: null`); `text_export` fragt „Textprojekt exportieren“ (Dialog „Agent möchte etwas Unumkehrbares tun“, „„agent-gate“ möchte: Textprojekt exportieren (lässt sich nicht rückgängig machen).“), Klick auf „Ablehnen“ → `needs_confirmation` „… in Gen4 Studio abgelehnt“, nichts geschrieben; `agent_answer` „allow“ → `written: 1` (`data/species/0001.json`, `attack` 49 → 50), 41 s (Debug-Brücke, Last 4,5); danach `text_check` → `current`, `differs: []` |

### Befunde

| Nr. | Schwere | Befund | Fix | Format oder Schnittstelle |
|---|---|---|---|---|
| A1 | Mittel | Schema-Fehler aus den serde-Typen (Header, Matrizen, Area-Data, Events, Trainer, Trainertext-Tabellen, ein unbekannter Schlüssel auf oberster Ebene von Begegnungen, Spezies, Attacken und Items, dazu Manifest, Register und Quirks) kamen als englischer serde-Text ohne JSON-Pfad: `data/headers/0005.json:6:20: textproject.schema: invalid value: integer `70000`, expected u16`, `… unknown field `zzz_unbekannt`, expected `data``, `data/moves/0001.json:1:2: textproject.schema: key must be a string`. Das Format verspricht deutsche Sätze mit dem Pfad des Feldes. | `9654750`: `canon::parse_json` (über `serde_path_to_error` 0.1, MIT/Apache-2.0) führt den Pfad und übersetzt die serde-Fälle: `matrix_id: die Zahl 70000 passt nicht in u16 (0 bis 65535).`, `width: erwartet wird eine Zahl von 0 bis 255, gefunden ist der Text "abc".`, `zzz_unbekannt: unbekannter Schlüssel (erlaubt: `id`, `data`).`, `$: der Schlüssel `id` fehlt.`, `kein gültiges JSON: ein Schlüssel muss ein Text in Anführungszeichen sein.`; die Meldungen von `Part` ebenso deutsch. Tests `parse_json_*` in `canon.rs`, `parts_and_members_report_german_sentences_with_their_path` in `part.rs`, `broken_registry_names_the_file` mit Pfad | Schnittstelle: Text der `textproject.schema`-Meldungen (Code, Datei, Zeile, Spalte unverändert). Kein Format: keine JSON-Schlüssel, `schema.snap` unverändert. Neue Abhängigkeit `serde_path_to_error` |
| A2 | Gering | Trainer, deren Name allein normalisiert nichts ergibt, bekamen die Klasse als ID: Platin-Trainer 0 („-“, Klasse „PKMN TRAINER“) hieß `TRAINER_PKMN_TRAINER` statt `TRAINER_0000`, wie es „ohne Namen `TRAINER_NNNN`“ verlangt. Ursache: `name.trim().is_empty()` prüfte den rohen Namen. | `af08608`: `registry::trainer_baptism_name` nimmt `normalize(name)`; leer, nur Markup, Satzzeichen oder Ziffern → `TRAINER_NNNN`. Tests `a_trainer_whose_name_normalizes_to_nothing_is_unnamed`, `places_classes_and_maps_of_only_punctuation_get_the_fallback`; [text_project.md](formats/text_project.md) präzisiert | **Format (Taufe):** ein neuer Export tauft diese Trainer `TRAINER_NNNN`; schon exportierte Register bleiben eingefroren (kein Umbenennen). Betrifft auf Platin DE Trainer 0 |
| A3 | Gering | `text_export` mit einer ungespeicherten Änderung: „Erst speichern: 1 ungespeicherte Änderungen.“ | `0e0a94b`: „… 1 ungespeicherte Änderung.“, sonst „… N ungespeicherte Änderungen.“; Test `unsaved_message_has_the_german_plural`; [mcp.md](mcp.md) | Schnittstelle: Text der MCP-Meldung `unsaved` (Code unverändert) |
| A4 | Gering | Ein fehlendes Beifile meldete `textproject.side-file: model.file: das Beifile `0003.nsbmd` fehlt oder ist nicht lesbar (No such file or directory (os error 2)).`, also einen englischen Betriebssystem-Text im deutschen Satz. | `9654750` (zusammen mit A1, dieselben Dateien): „das Beifile `0003.nsbmd` fehlt.“, bei anderen Fehlern „… ist nicht lesbar (<Fehler>).“; Test `part_in_reads_hex_side_files_and_the_base` | Schnittstelle: Text der `textproject.side-file`-Meldung |

### Messtabelle der Abnahme

Release-`g4k` auf `a6933b3`, Wanduhr je Aufruf, in Klammern das Lastmittel (1 min). Dateien und Größe: `g4k text sources` (der Ordnername der Kopie steht in `rotom.toml`, daher 4 bzw. 6 Byte mehr als im Gate).

| Messung | Platin DE | HeartGold DE | HeartGold US |
|---|---|---|---|
| Dateien (davon JSON unter `data/`) | 6 574 (4 722, davon 4 717 Entitäten) | 6 302 (4 504) | 6 302 (4 504) |
| Größe | 18 459 017 B (17,6 MiB) | 18 719 391 B (17,9 MiB) | 18 441 873 B (17,6 MiB) |
| Beifiles, Blobs (Vanilla) | 0, 0 | 0, 0 | 0, 0 |
| erster Export, kalt (davon Basisschicht) | 33,1 s (5,1 s) (20) | 62,5 s (4,7 s) (13) | 41,4 s (8,7 s) (16) |
| zweiter Export | 22,3 s (15) | 113,2 s (21) | 79,3 s (14) |
| dritter Export / nach dem Commit | 4,0 / 3,7 s (14) | 37,3 / 25,6 s (18 / 14) | 74,2 / 79,1 s (16) |
| Export nach den Änderungen je Art | 100,4 s, 9 Dateien (17) | 26,6 s, 8 Dateien (7) | 17,7 s, 8 Dateien (6) |
| `g4k text check` | 1,0 s (14) | 20,4 s (13) | 22,7 s (15) |
| Import nach Löschen aller Member (Vanilla) | 24,6 s (19) | 11,4 s (12) | 31,8 s (11) |
| Import nach Löschen (geänderter Stand) | 40,0 s (22) | 8,6 / 8,8 s (8) | 8,7 / 19,3 s (8 / 21) |
| Import, der nichts schreibt | 10,9 bis 77,9 s (15 bis 17) | 12,9 bis 78,9 s (10 bis 19) | 12,3 bis 24,1 s (6 bis 20) |
| Build nach dem Import | 2,0 bis 34,8 s (14 bis 17) | 2,9 bis 12,7 s (10) | 2,0 bis 25,2 s (6 bis 20) |
| Klon-Weg: Import mit neuer Basisschicht / Build | 65,6 / 1,4 s (18) | 163,6 / 9,1 s (23) | 30,4 / 5,2 s (18) |
| MCP kopflos, Export kalt (Debug) | 39,5 s | – | – |
| MCP über die Brücke, Export mit 1 Datei (Debug) | 41 s (4,5) | – | – |

Richtwerte (Export ≤ 20 s, Import ≤ 30 s je Projekt): bei Last 6 bis 8 gehalten (HeartGold Export nach Änderungen 17,7 s und 26,6 s, Import 8,6 bis 8,8 s), bei Last 12 bis 23 meist verfehlt, im selben Zustand bis zum Zwanzigfachen (Platin dritter Export 4,0 s, zweiter 22,3 s). Die Streuung folgt der Last, nicht dem Code (H1).

### Hinweise ohne Fix

- **H1 Zeiten unter Last:** Unter dem Lastmittel 12 bis 25 dieser Abnahme (4.3b baute, dazu die Fix-Agenten) dauerten derselbe Export ohne Schreiben 4 bis 113 s und ein Import ohne Schreiben 11 bis 79 s; das Gate maß bei Last 3 bis 7 4 bis 27 s und 3 bis 8 s. Ein Maß für den Code ist erst eine Messung ohne parallele Sitzung (Hand, oder in 4.3b mit der Entscheidung über das Parallelisieren).
- **H2 Wartezeit der Frage heißt „abgelehnt“:** Bleibt die Frage „Textprojekt exportieren“ 120 s ohne Antwort (`QUESTION_WAIT`), antwortet `text_export` `needs_confirmation` „… in Gen4 Studio abgelehnt“, obwohl niemand abgelehnt hat (so in dieser Abnahme beim ersten Export über die Brücke, weil der Aufruf im Hintergrund wartete). Das ist der Agenten-Hub aus 3h, kein Code von 4.3a. Den Hinweis H3 des Gates (sofort abgelehnt, ohne Klick) konnte die Abnahme nicht nachstellen: drei weitere Fragen, auch direkt nach `set_matrix_cell` und `undo`, blieben offen, bis sie beantwortet wurden.
- **H3 Ein kaputtes Register verdeckt andere Fehler:** Steht in `data/ids/maps.json` eine Nummer doppelt, meldet `text check` nur diesen Fehler; die übrigen kaputten Dateien desselben Laufs (falsche ID, Wert außerhalb des Feldes) kamen erst, als das Register wieder stimmte. Folgerichtig (ohne Register lassen sich die IDs nicht auflösen), aber ein Nutzer braucht dann zwei Läufe.
- **H4 Script-Übernahme kopflos:** `text_export` über `studio mcp` antwortet `not_adopted` „… (adopt_scripts)“; ein Agent kommt an `adopt_scripts` nur mit `--expert` über `g4_call`. So dokumentiert ([mcp.md](mcp.md): „`adopt_scripts` ist ein eigener Schritt in der App“).
- **H5 Roadmap nennt `g4k text status`:** Der Befehl heißt `g4k text check`.
- Hinweis H2 des Gates (Ordner in der Wurzel werden Blobs) gilt weiter; die Abnahme hat ihn nicht neu geprüft, ein Blob unter `unpacked/` (`pokeHeight/0005`) kam wie beschrieben.

### Prüflauf vor den Commits

Auf dem Stand mit allen Fixes (vor `9654750`, Last 17 bis 31): `cargo fmt --check` ok, `cargo clippy --workspace --all-targets --all-features -- -D warnings` ok, `cargo test --workspace --all-features` ohne `G4_TEST_PROJECT`/`G4_TEST_ROM` grün (g4-kernel 596, studio 585 und die übrigen Ziele), `cargo deny check licenses advisories` ok (neu: `serde_path_to_error` 0.1.20, MIT/Apache-2.0), `pnpm build` ok; `pnpm test` 3 186 von 3 187 grün, rot war `useWorkbenchModel.test.tsx` („Editor wird geladen …“ statt „Scripts werden geprüft“) bei Last 31, einzeln zweimal 10/10 grün; die Fixes ändern kein Frontend. Die drei Commits trennen nur unabhängige Dateien (`registry.rs` per Teil-Patch); `9654750` allein besteht `cargo test -p g4-kernel --lib textproject` (119). `scripts/gate.sh` lief nicht erneut, die gated Tests auch nicht: die Fixes ändern Meldungstexte und die Taufe unbenannter Trainer, das Szenario oben lief mit dem `g4k` vor den Fixes, die Proben nach den Fixes stehen in `check-broken-after-fix.log`, `a2-after-fix.log` und `a3-after-fix.log`. Log `checks.log`, `pnpm-test.log`.

### Für 4.3b: Änderungen an Format und Schnittstellen

- **Taufe (A2, Format der IDs):** Ein Trainer, dessen eigener Name normalisiert nichts ergibt (`-`, `?`, nur Ziffern, nur Markup), heißt bei einem neuen Export `TRAINER_NNNN`, nicht mehr `TRAINER_<KLASSE>`. Schon geschriebene Register bleiben eingefroren. Auf Platin DE betrifft das Trainer 0 (`TRAINER_PKMN_TRAINER` → `TRAINER_0000`); wer in 4.3b eine ID wie `TRAINER_PKMN_TRAINER` in Tests oder Fixtures nutzt, prüft das.
- **Meldungen (A1, A4, Schnittstelle):** Der Text von `textproject.schema` ist jetzt `<JSON-Pfad>: <deutscher Satz>` (Wurzel `$`), auch für Manifest, Register und Quirks; Syntaxfehler `kein gültiges JSON: …`; `textproject.side-file` sagt bei fehlender Datei „… fehlt.“. Code, Datei, Zeile und Spalte sind unverändert. Wer Meldungen per Text vergleicht (Tests, UI), passt das an. Neue Kernel-Funktion `canon::parse_json` (crate-intern) statt `serde_json::from_slice` + `json_problem`.
- **MCP (A3):** `unsaved` von `text_export` sagt bei einer Änderung „Erst speichern: 1 ungespeicherte Änderung.“.
- Keine Änderung an JSON-Schlüsseln, Dateinamen oder `schemaVersion`; `schema.snap` unverändert.
