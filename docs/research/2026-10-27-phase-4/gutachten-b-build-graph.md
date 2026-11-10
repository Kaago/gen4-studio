# Gutachten B: Basis-ROM, Patch und Build-Graph (Phase 4.1 und 4.2)

Stand: 26.10.2026, gelesen auf `main` (`2c79058`) im Arbeitsbaum `$HOME/Projekte/lab/editor/gen4-studio`, am Repo-Code nichts geändert. Grundlage sind `docs/ROADMAP.md` (Phase 4 mit Schnitt 4.0 bis 4.4, „Schon da“, offene Fragen; Phase 3g; „Aus dem Design-System, später“), [Gutachten C von 3e/3g](../2026-10-25-ausbau/gutachten-c-projektstart-mcp.md) (`.g4/base.json`, Extraktionszeiten) und der Code in `crates/g4-kernel/src/{build,start,scripts,text,export,project}.rs`, `apps/studio/src-tauri/src/commands/project.rs`, `apps/studio/src-tauri/src/mcp/settings.rs`, `apps/studio/src/App.tsx` und `apps/studio/src/app/useProjectSession.ts`. Dazu kommen die Quellen von ds-rom (`rev 8a9c151`, `lib/src/rom/rom.rs`, `overlay.rs`, `arm9.rs`), `oxidelta 0.1.4`, `xdelta3 0.1.5` (Crate) und das Modul `RomPatcher.format.vcdiff.js` von Rom Patcher JS (Stand master, 26.10.2026).

Messungen: ein Scratch-Crate mit Pfad-Abhängigkeit auf `crates/g4-kernel` misst die öffentlichen Funktionen einzeln (`prepare`, `pack_dir`, `Narc::write`, ds-rom `Rom::load`/`build`/`save`, `restore_header_crcs`). Dazu kommen `g4k build` als Release-Build, ein Prototyp des inkrementellen Builds und die xdelta-Kandidaten. Gemessen wurde auf APFS-Klonen der drei DSPRE-Ordner und ihrer `.nds` unter `Roms/_testkopien/4-gutachten-b-{pt,hgde,hgus}`; die Kopien sind wieder gelöscht. Die Skripte liegen in [`gutachten-b-messung/`](gutachten-b-messung/), siehe Anhang A.

**Last:** Zwei andere Gutachter liefen parallel. Die Load-Average lag bei 4 bis 33 (10 Kerne, 24 GB), und der Seiten-Cache wurde zwischen den Läufen mehrfach geleert. Darum stehen bei jeder Zahl ein Median über drei bis fünf Läufe und getrennt der „kalte“ erste Lauf. Die Streuung beim Lesen kleiner Dateien ist groß: dieselbe Stufe brauchte einmal 1,0 s und einmal 21 s.

**Kurzurteil.** Den heutigen Build bremsen weder rotom noch chatot. Teuer sind zwei Dinge:

1. **Das Lesen von 9 600 bis 10 400 kleinen Member-Dateien bei jedem Build.** Warm kostet das 0,6 bis 1,4 s, kalt 10 bis 21 s.
2. **Bei HeartGold die BLZ-Kompression von ARM9 und 127 Overlays in `Rom::load`.** Das sind 3,7 bis 4,7 s bei jedem Build.

`Rom::build` und `save` kosten zusammen nur 40 bis 60 ms. Ein Prototyp leistet dreierlei: Er prüft per `stat` die Fingerabdrücke aller rund 21 000 Eingaben, er setzt den geänderten Member in das vorhandene gepackte NARC ein und er liest die Overlays aus einem BLZ-Cache. Damit baut er nach einer Kartenänderung in **0,17 bis 0,20 s** auf allen drei Projekten, byte-gleich zum vollen Build. Heute dauert derselbe Build **1,0 bis 1,2 s (Platin) und 4,4 bis 5,2 s (HeartGold)**, kalt 13 bis 23 s.

Die drei DSPRE-Ordner rebuilden **byte-gleich zur Original-ROM**, sobald eine `.g4/base.json` mit `origin: "dspre"` die CRC der Secure Area aus der `.nds` trägt. Ohne Marke weichen genau 4 Bytes ab: die Secure-Area-CRC und die Header-CRC.

Für xdelta empfehle ich den **C-Encoder von xdelta3 (Apache-2.0) über eine eigene schmale FFI ohne bindgen**. Er liefert Patches von 0,4 KB (ein Byte geändert) bzw. 4,8 bis 8,3 KB (Karte, Script und Text) in 0,16 bis 0,26 s. Das reine Rust-Crate `oxidelta` (MIT) ist zwar lizenzrein, erzeugt hier aber Patches von 0,7 bis 2,6 MB, weil es verschobene Daten schlecht findet. Das Crate `xdelta3` zieht über bindgen 0.52 vier RustSec-Befunde in die Build-Abhängigkeiten.

---

## 1. Befund: der Build heute

### 1.1 Stufen

```
 Quelle (vom Nutzer/der App/DSPRE bearbeitet)       Zwischenstand            gepackt                 ROM
 ─────────────────────────────────────────────     ──────────────────       ─────────────────       ──────────
 expanded/scripts/NNNN.rotom|.json ──rotom──────▶ unpacked/scripts/NNNN ─┐
 expanded/textArchives/NNNN.json ──chatot──────▶ unpacked/textArchives/NNNN ─┤
 Store (Karten, Events, Daten, Trainer …) ──save─▶ unpacked/<art>/NNNN ─────┼─pack_dir─▶ files/…/x.narc ─┐
 DSPRE (bearbeitet unpacked/ direkt) ───────────▶ unpacked/<art>/NNNN ─────┘                           │
 arm9/, arm9_overlays/, arm7/, banner/, header.yaml, files/ (Rest), path_order.txt ───────────────────┤
                                                              ds-rom Rom::load (BLZ!) → build → save ─┴─▶ out.nds
                                                                          start::restore_header_crcs ──▶ out.nds (CRC)
```

| Stufe | Code | Eingabe | Ausgabe | Heute „stale“? |
|---|---|---|---|---|
| 0 Speichern (App) | `build_rom_impl` speichert zuerst den Store (`apps/studio/src-tauri/src/commands/project.rs:236-258`), das Frontend vorher die offenen Scripts (`useProjectSession.ts:278-313`) | Store, Editor | Member in `unpacked/`, Quellen in `expanded/` | Store: Baseline je Asset (`reload_from_disk`, `accept_disk_as_base`) |
| 1 Prüfung | `prepare_for_build` → `reject_hidden_files(files/)` (`build.rs:132-155`, `172-178`) | `files/` | – | – |
| 2 rotom | `prepare` → `scripts::compile` (`build.rs:43-62`, `scripts.rs:600-723`), nur wenn `rotom.toml` da ist (`scripts.rs:81`) | `expanded/scripts/*`, Header-Tabelle, Text-JSON des Archivs, Befehlsdatenbank | `unpacked/scripts/NNNN`, angehängte Texte in `expanded/textArchives/*.json` und ihre Member | **ja**: rotoms `.rotom/status/compile-state.json` (Hash von Quelle, Befehlsdatenbank, Compiler); `mark_stale` setzt `source_hash = 0`, wenn ein Header die Scriptdatei umhängt (`scripts.rs:560-590`); vorher `refresh_stale_text_json` nach mtime (`scripts.rs:534-551`) |
| 3 chatot | `text::encode_stale` (`text.rs:447-500`) | `expanded/textArchives/NNNN.json` | `unpacked/textArchives/NNNN` | **ja**, aber teuer: Gewinner nach mtime (`text_winner`, `text.rs:251-267`), dann **dekodiert er für jedes gewinnende JSON den Member und vergleicht die Texte** (`member_is_current`, `text.rs:424-439`) |
| 4 NARC packen | `build_prepared` → `pack_dir` je Art aus `archives.kinds()` (`build.rs:71-111`, `186-208`) | **alle** Member jeder Art (Pt 9 600 Dateien, HGSS 10 400) | `files/…/x.narc` | nur am Ausgang: `write_if_changed` vergleicht das fertige NARC byteweise und schreibt nur bei Änderung (`build.rs:116-128`); gelesen und gepackt wird immer alles |
| 5 ds-rom | `Rom::load(config.yaml, RomLoadOptions::default())`, `rom.build(None)`, `raw_rom.save` (`build.rs:211-220`) | `config.yaml`, `header.yaml`, Logo, `arm9/`, `arm9_overlays/` (BLZ-Kompression in `load`: ds-rom `rom.rs:396-399`, `481-485`), `arm7/`, `banner/`, `files/` (340 bzw. 384 Dateien, 96 bzw. 118 MiB), `path_order.txt` | `out.nds` (128 MiB) | nein |
| 6 CRC | `start::restore_header_crcs` (`start.rs:560-596`) | `.g4/base.json` (Secure-Area-CRC und -SHA-1) | 4 Bytes in `out.nds` | – (ohne Marke tut es nichts) |

Einen Vorläufer für werkzeugabhängige Cache-Schlüssel gibt es schon: `export.rs` nimmt xxh3 über `CACHE_SALT`, die Binärdatei von apicula und die Eingaben (`export.rs:19`, `39-64`, `279-292`) und legt den Cache in `.g4/models/`. Unter `.g4/` liegen heute `tmp/`, `backup/`, `models/` und `base.json`, mit 4.0 kommt `preview.png` dazu.

### 1.2 „Hier spielen“

`App.tsx:364-378` (`onPlay`) ruft `session.onBuild(session.lastRom)`. Nur beim ersten Spielen fragt ein Dialog nach dem Pfad (Vorschlag `out.nds`), danach baut es ohne Frage an den Pfad des letzten Builds. `onBuild` (`useProjectSession.ts:278-313`) speichert erst die offenen Scripts (und, falls nötig, vorher den Store) und ruft dann den Tauri-Befehl `build_rom`. Der läuft auf einem Blocking-Worker (`project.rs:383`), hält die Sperre für Speichern und `prepare_for_build` und lässt sie für `build_prepared` frei (`project.rs:236-258`). `build_dto` lädt danach die Textarchive neu, die rotom berührt hat (`project.rs:264-325`). Zum Schluss startet `onRun` den Emulator (`useProjectSession.ts:337-356`, `run_rom_impl` → `g4_kernel::run::launch_emulator`). Den Pfad des Emulators speichert das Frontend in `localStorage` (`g4.emulatorPath`, `useProjectSession.ts:18-40`). Im Browser-Modus wird nur gebaut. Über MCP und die Brücke heißt der Befehl ebenso `build_rom` (`commands/dispatch.rs:459`), kopflos gibt es `g4k build --out <rom> <projekt>` (`bin/g4k.rs:298`).

Für den Build-Graph heißt das: Er sitzt vollständig im Kernel, unter `build::build`, `prepare_for_build` und `build_prepared`. Die Aufrufer (App, Brücke, MCP, `g4k`) bleiben, nur `BuildReport` bekommt die Schritte mit Dauer.

### 1.3 Messungen: der volle Build

Release-Builds; „kalt“ heißt erster Lauf mit leerem Seiten-Cache (frischer APFS-Klon oder Cache verdrängt), „warm“ ist der Median der Folgeläufe ohne Änderung.

| Projekt | Zustand | `g4k build` kalt | `g4k build` warm (Median) |
|---|---|---|---|
| Platin DE | Vanilla-DSPRE-Ordner | 12,9 s | 1,10 s (2,47 / 1,10 / 0,93) |
| HeartGold DE | Vanilla | 20,7 s | 4,69 s (7,12 / 4,69 / 4,44) |
| HeartGold US | Vanilla | 19,5 s | 5,24 s (11,98 / 5,24 / 5,04) |
| Platin DE | Scripts übernommen, alle 724 Texte als JSON | 17,0 s | 1,23 s (1,59 / 1,21 / 1,20 / 1,26) |
| HeartGold DE | übernommen, 829 JSON | 23,0 s | 4,89 s (7,82 / 4,96 / 4,82 / 4,82) |
| HeartGold US | übernommen, 829 JSON | 16,9 s | 4,93 s (5,41 / 5,01 / 4,81 / 4,85) |

Zeitanteile je Stufe (`proto-b stages`, warm, Median; Vanilla-Ordner, danach mit übernommenen Scripts und allen Texten als JSON):

| Stufe | Platin DE | HeartGold DE | HeartGold US |
|---|---|---|---|
| `prepare`, Vanilla (nichts übernommen) | 0 ms | 0 ms | 0 ms |
| `prepare`, Scripts übernommen, nichts geändert (rotom hasht alle Quellen) | 190 ms (188 bis 235) | 200 ms (191 bis 336) | 180 ms (166 bis 292) |
| `prepare` mit allen Text-JSON, nichts geändert (`encode_stale` dekodiert alle) | 500 ms (497 bis 608) | 510 ms (507 bis 616) | 600 ms (506 bis 853) |
| `prepare`, ein Script geändert | 380 ms | 330 ms | 480 ms |
| `pack_dir` aller Arten (lesen) | 1,0 bis 1,4 s warm, **15,7 bis 21 s kalt** | 0,6 bis 1,0 s warm, **9 bis 15,6 s kalt** | 0,6 bis 1,3 s warm, **4,5 bis 12,4 s kalt** |
| `Narc::write` aller Arten | 2 ms | 2 ms | 3 ms |
| Vergleichen, Schreiben bei Änderung | 6 ms | 6 ms | 7 ms |
| `Rom::load` | 60 bis 100 ms (kalt 0,8 bis 2,0 s) | **3,7 s** (3,6 bis 5,4) | **4,0 s** (3,7 bis 4,7) |
| davon BLZ: 127 Overlays / ARM9 | – (Pt: alles unkomprimiert) | 3,0 s / 1,0 s | 2,8 bis 3,0 s / 1,0 s |
| `Rom::build` | 25 bis 35 ms | 14 bis 32 ms | 13 bis 25 ms |
| `save` (128 MiB) | 20 bis 30 ms | 15 bis 28 ms | 15 bis 28 ms |
| `restore_header_crcs` | 2 bis 15 ms | 2 bis 10 ms | 2 bis 19 ms |

Gelesen wird viel: `unpacked/` hat 18 005 (Pt) bzw. 19 498 (HGSS) Dateien, davon packt der Kernel rund 9 600 bzw. 10 400. Ein `stat` über alle Eingaben (`unpacked/`, `expanded/`, `files/`, `arm9*`, `arm7`, `banner`: 20 328 bzw. 21 818 Dateien) kostet **37 bis 195 ms**, ruhig sind es 50 ms. xxh3 über den Inhalt derselben Dateien kostet warm 4 bis 25 s, weil jede Datei geöffnet werden muss. **Fingerabdrücke müssen also auf `stat` beruhen und nur bei Abweichung den Inhalt hashen.**

### 1.4 Ein paar Befunde nebenbei

1. **Byte-gleich nur mit Marke.** Der Vanilla-Rebuild eines DSPRE-Ordners weicht heute in 4 Bytes von der Original-ROM ab: `0x6C..0x6D` (Secure-Area-CRC, ds-rom schreibt `0000`) und `0x15E..0x15F` (Header-CRC). Ich habe zum Test eine Marke `.g4/base.json` mit `origin: "dspre"` geschrieben, deren `secure_area_crc` und `secure_area_sha1` aus der `.nds` stammen (`gutachten-b-messung/mark_dspre.py`). Damit war die SHA-1 der gebauten ROM **auf allen drei Projekten gleich der Original-ROM**: `d40f19ac…`, `13b6853a…`, `4fcded0e…`. Der Gate-Punkt „byte-gleicher Rebuild auch für die drei DSPRE-Testprojekte“ aus 4.1 ist damit belegt machbar.
2. **14 (Pt) bzw. 17 (HGSS) DSPRE-Ordner packt `g4k build` nicht.** Gemeint sind die Ordner ohne Eintrag in `metadata/archives.yaml`:
   - Platin: `OWSprites`, `battleBg`, `battleTowerPokemon`, `battleTowerTrainers`, `buildingConfigFiles`, `encounterExtended`, `otherPokemonBattleSprites`, `pokeHeight`, `pokeHeightForms`, `pokemonBattleSprites`, `pokemonSpriteOffsets`, `synthOverlay`, `tradeData`, `trainerGraphics`;
   - HeartGold zusätzlich `battleObj`, `eggMoves`, `headbutt`, `rockSmash`, `safariZone`, ohne `encounterExtended` und `pokeHeightForms`.

   Wer diese Ordner in DSPRE bearbeitet und mit dem Studio baut, verliert die Änderung, sofern DSPRE das NARC nicht selbst neu geschrieben hat (in DSPRE per Hand prüfen). Der Graph sollte diese Ordner als Eingaben kennen: packen oder warnen (§2.6). Nebenbei gehört dazu `pokeHeight`, also Pokédex-Größe und -Gewicht, die die Roadmap Phase 4 zuordnet.
3. **Agenten-Freigabe hängt an der Marke.** `ProjectKey` setzt sich aus Pfad und `base_id = "<sha1>/<created>"` zusammen (`mcp/settings.rs:64-82`, `grant_for` `:104-109`). Bekommt ein DSPRE-Ordner nachträglich eine Marke, passt seine alte Freigabe (`base_id: None`) nicht mehr, und die App fragt neu. 4.1 sollte die Freigabe beim Zuordnen umschreiben.
4. **Kein `deny.toml` im Repo.** Die Roadmap verlangt „Lizenz prüfen, `cargo deny`“. Mit einer Freigabeliste aus den Lizenzen, die der Abhängigkeitsbaum heute schon nutzt (`gutachten-b-messung/deny.toml`), sind die Lizenzen des Workspace grün. **Advisories sind schon heute rot, ohne xdelta**: acht Befunde, darunter `zip` (drei), `tokio`, `quick-xml` (zwei), `proc-macro-error` und `unsafe-libyaml` (unmaintained). 4.1 braucht also eine echte `deny.toml` mit begründeten Ausnahmen, sonst ist „`cargo deny` grün“ kein erreichbares Gate.

---

## 2. Build-Graph (4.2)

### 2.1 Was der Graph wissen muss

Die Stufen sind fest und wenige: Quelle → `unpacked/` → NARC → ROM. Ab 4.3 kommt vorn „Entität (JSON) → Member“ dazu, ab Phase 5 kommen Build-Schritte aus Plugins. Ein allgemeiner DAG-Löser im Stil von salsa ist nicht nötig. Es genügt eine kleine, explizite Schrittliste mit einem gemeinsamen Vertrag:

```rust
trait Step {
    fn id(&self) -> StepId;                       // "rotom", "chatot", "pack:maps", "blz:ov012", "dsrom", "crc"
    fn inputs(&self, p: &Project) -> Inputs;      // Dateien/Ordner + Werkzeug-Salz
    fn run(&self, p: &Project, dirty: &Dirty) -> Result<StepReport, KernelError>;
}
```

`Dirty` sagt dem Schritt, *welche* Eingaben sich geändert haben (Member-genau). Ein Schritt ohne geänderte Eingabe läuft nicht, und sein Ausgang gilt als unverändert („early cutoff“). Erzeugt ein Schritt bytegleiche Ausgaben, etwa rotom nach einer Kommentaränderung (gemessen: das Script-NARC blieb gleich), endet die Kette dort.

### 2.2 Fingerabdrücke je Schritt

| Schritt | Eingaben | Ausgabe | Fingerabdruck | Werkzeug-Salz |
|---|---|---|---|---|
| `rotom` | `expanded/scripts/NNNN.{rotom,json}`, Header-Tabelle (Script → Textarchiv), Text-JSON der betroffenen Archive | `unpacked/scripts/NNNN`, ggf. angehängte Texte | `stat` je Quelle (Größe, mtime ns, Datei-ID), bei Abweichung xxh3; xxh3 der Zuordnung Header → (Scriptdatei, Textarchiv) | rotom-Tag und -Commit (`v0.7.0 fb11503`), Revision der Befehlsdatenbank (`metadata/scrcmd`, Rev. 33). rotoms eigener `compile-state.json` bleibt maßgeblich, der Graph entscheidet nur, *ob* rotom überhaupt läuft |
| `chatot` | `expanded/textArchives/NNNN.json`, Sprache | `unpacked/textArchives/NNNN` | `stat` je JSON, bei Abweichung xxh3; ersetzt die heutige Dekodier-Prüfung `member_is_current` bei jedem Build | chatot-Version (`0.8.1`), Zeichentabelle |
| `pack:<art>` (eine je Art, auch die DSPRE-Ordner aus §1.4) | alle Member `unpacked/<art>/NNNN`, ihre Anzahl | `files/…/<x>.narc` | `stat` je Member; Ausgabe-xxh3 des NARC | NARC-Writer-Version (`g4-formats`) |
| `blz:arm9`, `blz:ovNNN` | unkomprimierte Bytes, die `Rom::load` ohne Kompression liefert | komprimierte Bytes | xxh3 der unkomprimierten Bytes | ds-rom-Rev. (`8a9c151`) |
| `dsrom` | `config.yaml`, `header.yaml`, `header_logo.png`, `arm7/`, `banner/`, `path_order.txt`, `files/` (alle), Ausgaben von `blz:*` | `out.nds` | `stat` aller Dateien, xxh3 der `blz`-Ausgaben; dazu Pfad, Größe und mtime der zuletzt geschriebenen `out.nds` | ds-rom-Rev. |
| `crc` | `.g4/base.json` (Secure-Area-CRC/-SHA-1) | 4 Bytes in `out.nds` | Inhalt von `base.json` | – |

**Warum `stat` zuerst:** 50 ms für 21 000 Dateien statt 4 bis 25 s für Inhalts-Hashes (§1.3). Für den Fall „mtime unsicher“ gilt die Regel aus Git („racy git“): Liegt die mtime einer Datei nicht mindestens 2 s vor dem Zeitpunkt des letzten Scans, oder ist das Dateisystem grob (FAT/exFAT mit 2 s), wird ihr Inhalt gehasht. Ein Treffer mit gleichem Hash aktualisiert nur den `stat`-Eintrag. Das fängt auch `touch`, Git-Checkouts und Kopien ab, die mtimes ändern, ohne Bytes zu ändern.

**Datei-ID:** Kernel und Store schreiben atomar über `rename`, also mit neuer Inode. DSPRE schreibt vermutlich an Ort und Stelle, dann mit neuer mtime und meist neuer Größe. Beides fällt im `stat` auf. Verzeichnis-mtimes genügen nicht, weil Änderungen an Ort und Stelle sie nicht berühren.

**Werkzeugwechsel:** Ein Salz je Schritt, zur Build-Zeit aus `Cargo.lock` erzeugt. `crates/g4-kernel/build.rs` existiert und kann es als `env!` ausgeben. Ein Test hält das Salz gleich mit dem Lockfile. Ändert sich das Salz eines Schritts, sind dieser Schritt und alles dahinter ungültig. Den Kernel wechselt man nur mit einem Update der App, ein Wechsel kostet also einen vollen Build pro Projekt und Update.

### 2.3 Wo speichern

`.g4/build/`, nicht in Git (Vorlage 4.4), jederzeit löschbar; ohne den Ordner baut der Graph voll und byte-gleich:

```
.g4/build/
  state.json        # { version, salt{…}, steps{ id → { inputs:[[pfad,größe,mtime_ns,id,xxh3]…], output_xxh3, ms } }, rom{pfad,größe,mtime,xxh3} }
  blz/arm9-<xxh3>.blz, blz/ov012-<xxh3>.blz …   # HGSS: 128 Dateien, 3,2 MB
```

Größe von `state.json`: rund 21 000 Einträge zu je etwa 70 Byte JSON ergeben etwa 1,5 MB. Ein Binärformat lohnt erst ab Messungen über 50 ms für Laden und Speichern. Die Ergebnisse der Prüfungen kommen nicht hierher, sie bleiben in `g4-checks.json` und im Speicher.

### 2.4 Kann ds-rom inkrementell?

Nein. `Rom::build` schreibt die ROM jedes Mal neu in einen `Vec` (`rom.rs:764-900`) und legt die Dateien lückenlos in `path_order` aus, mit 512-Byte-Ausrichtung. Wächst ein NARC über seine Ausrichtung hinaus, verschiebt sich alles dahinter. Das ist aber **kein Problem**: `build` plus `save` kosten 40 bis 60 ms, `load` ohne Kompression 55 bis 105 ms (kalt etwa 1 s für `files/`). Teuer ist nur die BLZ-Kompression in `Rom::load`, und die lässt sich cachen.

Der Weg führt nur über öffentliche ds-rom-API:

1. `RomLoadOptions { compress: false, .. }` laden.
2. Für jedes Overlay mit `compressed: true` in `overlays.yaml` die komprimierten Bytes aus `blz/` holen (Schlüssel: xxh3 der unkomprimierten Bytes) und als `Overlay::new(bytes, OverlayOptions { originally_compressed: true, info })` einsetzen.
3. ARM9 ebenso über `Arm9::new(bytes, *arm9.offsets())`.
4. Ein Fehlschlag im Cache komprimiert wie ds-rom selbst (`ov.compress()`, `arm9.compress()`) und füllt den Cache.

Gemessen ist das **byte-gleich** zum normalen Build auf allen drei ROMs, im Prototyp `proto-b dsrom-split` und `inc`. Vorbehalte:

- Signierte Overlays (`signed: true`) sind in den drei Spielen nicht belegt. Für sie müsste der Weg `sign` und `update_overlay_signatures` nachbilden. Bis dahin gilt: Ist ein Overlay signiert, fällt der Graph auf `compress: true` zurück.
- Der Weg hängt an Interna, die ds-rom heute öffentlich hat (`Overlay::new`, `Arm9::new`, `arm9_overlays_mut`). Ein Update von ds-rom braucht einen Test „Cache-Weg == `Rom::load` mit Kompression“ (gated, je Projekt). Schöner wäre ein Haken in ds-rom selbst (Vorschlag upstream: `RomLoadOptions::compressed_cache`), aber das ist nicht nötig.

Ein Patch der fertigen ROM an Ort und Stelle (geänderte Datei in den alten Slot schreiben, FAT anpassen) spart höchstens weitere 100 ms und nimmt ds-rom die Hoheit über das Layout. **Nicht empfohlen.**

### 2.5 Varianten

**Variante A: „Tore vor den heutigen Schritten“** (S). Der Graph kennt die Schritte mit der Granularität einer Art. Er überspringt rotom und chatot ohne geänderte Quelle, packt nur Arten mit geändertem `stat`, aber jede geänderte Art liest wieder alle ihre Member. Dazu kommt der BLZ-Cache. Messwerte: HGSS sinkt von 4,4 bis 5,2 s auf etwa 0,2 s plus die Leseszeit der geänderten Art (Karten: 666 Dateien, warm 80 ms, kalt 1,3 s; Textarchive: 724 Dateien, warm 290 ms, kalt 1,2 s). Platin sinkt von 1,0 bis 1,2 s auf 0,3 bis 0,5 s.

**Variante B: „Member-genau mit Einsetzen“** (M, **recommend**). Wie A, dazu ein `stat` je Member. `pack:<art>` liest das vorhandene gepackte NARC (eine Datei), ersetzt nur die geänderten Member und hängt neue an. Ein gelöschter Member (DSPRE löscht nicht, die App ab 3d vielleicht) oder ein unbekannter Zustand führt zum vollen Packen der Art. Der Prototyp (`proto-b inc`: `stat` aller Eingaben, eine Karte eingesetzt, ds-rom mit BLZ-Cache, CRC) misst:

| | Platin DE | HeartGold DE | HeartGold US |
|---|---|---|---|
| inkrementell, warm (Median aus 2 bis 3) | **0,18 s** (0,183 / 0,194) | **0,19 s** (0,186 / 0,202) | **0,17 s** (0,167; ein halb kalter Lauf 0,86) |
| davon `stat` 20 bis 22 Tsd. Dateien | 51 bis 59 ms | 48 bis 56 ms | 48 bis 75 ms |
| davon Einsetzen in das NARC | 5 bis 6 ms | 6 ms | 6 ms |
| davon ds-rom laden ohne BLZ / Cache / build+save+CRC | 72 / 0 / 55 ms | 71 bis 78 / 11 / 50 ms | 63 / 9 / 40 ms |
| erster Lauf kalt (BLZ-Cache leer) | 1,27 s | 4,85 s | 5,10 s |
| Ergebnis gleich vollem Build | ja | ja | ja |

**Variante C: „Allgemeine Build-Engine“** (L). Ein generischer DAG mit Regeln, Inhalts-Speicher und parallelen Schritten, in der Art von Bazel oder salsa. Für sechs Schrittarten ist das zu viel. Was Phase 5 braucht (Plugin-Schritte), deckt der `Step`-Vertrag aus §2.1 ab. **Nicht empfohlen.**

**Empfehlung (recommend): Variante B.** Die Reihenfolge innerhalb von 4.2:

1. BLZ-Cache. Allein spart er auf HGSS 4 s.
2. `state.json` und die `stat`-Tore für rotom und chatot.
3. Member-genaues Packen.
4. Anzeige in Technik › Prüfen und Bauen.

### 2.6 Erwartete Ersparnis bei „Hier spielen“

Heute: Median warm, kalt in Klammern. Graph: Variante B, gemessen bzw. hochgerechnet aus den Stufenzeiten.

| typische Änderung | Platin DE heute → Graph | HeartGold DE heute → Graph | HeartGold US heute → Graph |
|---|---|---|---|
| eine Karte (Store-Save, Member gleich groß) | 1,2 s (13 bis 17 s) → **0,18 s** gemessen | 4,9 s (21 bis 23 s) → **0,19 s** gemessen | 4,9 s (17 bis 19 s) → **0,17 s** gemessen |
| ein Text (JSON, chatot, NARC wächst) | 1,5 s (gemessen D3) → ≈ 0,25 s (+ chatot für 1 Archiv, ≈ 30 bis 60 ms) | 4,8 s → ≈ 0,25 s | 5,2 s → ≈ 0,25 s |
| ein Script (rotom 1 Datei) | 1,2 s (gemessen C1) → ≈ 0,3 s (rotom 1 Datei ≈ 100 bis 150 ms über den Leerlauf) | 4,9 s → ≈ 0,3 s | 5,6 s → ≈ 0,3 s |
| nichts geändert | 1,0 bis 1,2 s → ≈ 0,06 s (nur `stat`, ROM gilt als aktuell) | 4,4 bis 4,9 s → ≈ 0,06 s | 4,8 bis 5,0 s → ≈ 0,06 s |

Für Text und Script ist der Wert hochgerechnet: der Leerlauf des Prototyps plus die gemessene Differenz von `prepare` gegenüber „nichts geändert“. Den Text-Fall hat der Prototyp nicht eigens gemessen. Der Gewinn beträgt etwa **Faktor 5 bis 7 auf Platin und Faktor 25 auf HeartGold**. Kalt wird er noch größer, weil das Einsetzen statt 10 000 Dateien nur eine Handvoll öffnet. Auf Windows mit Virenscanner, wo jedes Öffnen teuer ist, zählt das am meisten (nicht gemessen).

### 2.7 Prüfungen des Build-Graphs (aus 3b.4 §1)

Vorschlag für Codes im Katalog der Prüfungen, alle mit Satz und Ausweg:

- `build.unpacked-not-packed`: Ein DSPRE-Ordner, den das Studio nicht packt (§1.4), ist neuer als sein NARC. Das wird eine Warnung, bis die Art in `archives.yaml` steht. Danach entfällt die Prüfung, weil der Graph den Ordner packt.
- `build.hidden-file`: heute ein harter Fehler in `reject_hidden_files`, künftig als Befund mit „Entfernen“.
- `build.member-gap`: Lücke in `NNNN`, heute ein Fehler von `pack_dir`.
- `build.base-missing` (Hinweis): Projekt ohne Basis-ROM; xdelta und „Auf Original zurücksetzen“ sind aus.
- `build.cache-stale` (intern, ohne Anzeige): Das Werkzeug-Salz hat sich geändert, der nächste Build läuft voll.

### 2.8 Anzeige

Technik › Prüfen und Bauen zeigt eine Zeile je Schritt aus `BuildReport.steps`, mit Dauer, Zähler und „übersprungen“. Beispiele: „Scripts: 1 von 1124 neu, 0,33 s“, „Texte: aktuell“, „Karten packen: 1 Member“, „Overlays: 127 aus dem Cache“, „ROM schreiben 0,12 s“, „Prüfsummen“. „Neu bauen (ohne Cache)“ löscht `.g4/build/`. Für das Gate schreibt `g4k build --timings` dieselben Zeilen als JSON.

---

## 3. Basis-ROM (4.1)

### 3.1 `.g4/base.json` für geöffnete DSPRE-Ordner

Heute schreibt nur `create_project` die Marke, mit `origin: "extracted"` (`start.rs:221-243`, `467`). Für DSPRE-Ordner schlage ich vor:

- **`BaseMark` erweitern.** `origin: "dspre"`; `template` wird `Option` (für `dspre` leer) oder bekommt den Wert `original`; neu ist `assigned: <RFC 3339>`. `created` bleibt der Zeitpunkt der Marke, weil `base_id` daran hängt. `version` wird 2, und `read_base_mark` liest 1 und 2.
- **Zuordnen.** Den Weg „Basis-ROM zuordnen…“ gibt es in Technik › Projekt und als Hinweis beim Öffnen. Der Kernel bekommt dafür `start::assign_base(root, nds) -> Result<BaseCheck, _>`:
  1. `inspect_rom` liest die SHA-1 der ganzen Datei; gemessen 0,12 s. Dazu kommen Spielcode, Titel und Version.
  2. **Spielcode gegen `header.yaml` (`gamecode`)**: Weichen sie ab, ist das eine **Sperre**. Sonst wären CRC, Archivpfade und Patch falsch.
  3. Titel und `rom_version` gegen `header.yaml`: Abweichung ergibt eine Warnung.
  4. SHA-1 gegen `metadata/base_roms.yaml` (heute die drei Testprojekt-Dumps; No-Intro-Hashes nachtragen): Bei Treffer heißt es „bekannter Dump: <Label>“, sonst kommt die Warnung „Unbekannte oder veränderte ROM; Patches gelten nur für genau diese Datei.“
  5. Optional und billig: `arm7/arm7.bin`, Banner und Logo gegen die ROM (ds-rom `raw::Rom` und `FileSystem::parse` kosten 3 ms). Eine Abweichung ist ein starkes Indiz für die falsche ROM und ergibt eine Warnung.
  6. Schreiben: `secure_area_crc` und `secure_area_sha1` aus der `.nds` wie bei `create_project` (`start.rs:530-552`).
  7. Danach (Hintergrund, nur Information): „N Einträge weichen vom Original ab“ über den Vergleich aus §3.3. Das kostet 0,7 bis 1,4 s warm und bis 13 s kalt, im Hintergrund unkritisch.
- **Warnung statt Sperre.** Außer dem Spielcode sperrt nichts. Fehlt die Marke, ist das kein Fehler, sondern der Hinweis `build.base-missing`.
- **Vorschlag automatisch.** DSPRE legt den Ordner als `<Name>_DSPRE_contents` neben `<Name>.nds` an, so wie die drei Testprojekte. Liegt eine solche `.nds` daneben und stimmt der Spielcode, schlägt die App sie vor („Gefunden: … Zuordnen?“). Der Nutzer bestätigt, denn die Marke schreibt in den Projektordner.
- **Freigabe umschreiben.** Beim Zuordnen ersetzt die App den Eintrag `(pfad, None)` in `agents.json` durch `(pfad, sha1/created)` (§1.4, Punkt 3).
- **Kopflos.** Dafür gibt es `g4k base assign <projekt> <nds>` und `g4k base check <projekt> [--rom <nds>]`. Für MCP gehört das zum Unumkehrbaren mit Rückfrage, weil es eine Marke schreibt.

### 3.2 Woher die Basis-ROM kommt

Das Projekt kennt nur die SHA-1. Die ROM liegt nie im Projekt und nie in Git.

**Empfehlung (recommend):** eine App-Einstellung `<App-Daten>/base-roms.json` nach dem Muster von `agents.json` (`mcp/settings.rs`: Rust-seitig, atomar, 0600). Der Schlüssel ist die **SHA-1, nicht das Projekt**. Mehrere Projekte teilen sich so eine Basis, und ein umbenannter Projektordner verliert nichts:

```json
{ "version": 1, "roms": [ { "sha1": "d40f19ac…", "path": "/…/Pokemon - Platin-Edition (Germany).nds",
    "size": 134217728, "mtime": "…", "game_code": "CPUD", "label": "Platin DE (Testprojekt)", "verified": "2026-10-26T…" } ] }
```

- **Eintragen:** beim „Neues Spiel aus ROM“ (3g.1 kennt den Pfad schon), beim Zuordnen (§3.1) und bei „Basis-ROM suchen…“.
- **Auflösen:** Der Eintrag zur SHA-1 aus der Marke wird nachgeschlagen. Stimmen Größe und mtime, gilt er ohne Hash. Sonst wird die SHA-1 neu berechnet (0,12 s) und der Eintrag bestätigt oder verworfen.
- **Fehlt die Datei:** Nichts sperrt den Build. xdelta-Export, „Auf Original zurücksetzen“ und der Diff gegen die Basis (4.4) sind dann aus, mit Grund („Basis-ROM nicht gefunden: Platin DE (d40f19ac…). Datei suchen…“). Ein ausgewählter Ersatz muss dieselbe SHA-1 haben, sonst kommt „Das ist eine andere ROM (SHA-1 …)“ und es wird nicht gespeichert. Ein Modaldialog beim Öffnen entfällt.
- **Kopflos, MCP, Tests:** `--base-rom <nds>` an `g4k` und `studio mcp`, dazu die Umgebungsvariable `G4_BASE_ROM_DIR`. Darin sucht der Kernel die `*.nds` mit passender SHA-1; im Gate ist das `G4_ROMS_DIR`. Die `UserPolicy` erlaubt Lesen der Basis-ROM nur über diese Einstellung.
- Die Startliste (4.0, Projektvorschau) braucht die Basis nicht.

### 3.3 Vergleich je Entität: Kosten

Gemessen mit `proto-b basecmp` (Basis-ROM aus der Datei, `raw::Rom` + `FileSystem::parse`, dann je Art das NARC aus der ROM gegen den Ordner):

| Teil | Platin DE | HeartGold DE | HeartGold US |
|---|---|---|---|
| ROM lesen (128 MiB) | 14 bis 37 ms | 13 bis 36 ms | 13 bis 56 ms |
| Dateisystem parsen | 3 ms | 3 ms | 3 ms |
| ein NARC aus der Basis lesen und zerlegen | ≤ 6,5 ms (Karten), sonst < 3 ms | ≤ 5,6 ms | – |
| `Rom::extract` (ganz, im Speicher, lazy) | 4 bis 10 ms | 10 bis 16 ms | 11 ms |
| **eine Entität vergleichen oder zurücksetzen** | **< 60 ms** | **< 60 ms** | **< 80 ms** |
| alle Arten, Projektseite aus `unpacked/` gelesen | 1,4 s warm, 12,6 bis 16 s kalt | 0,8 s warm, 7,1 bis 13,3 s kalt | 0,7 s warm, 2,9 s kalt |

Die Basisseite ist billig. Teuer ist wie beim Build nur das Lesen der Projektordner. Für Marken wie „geändert gegenüber Original“ an jeder Zeile empfehle ich deshalb, nicht die Ordner zu lesen. Stattdessen werden Hashes verglichen:

- xxh3 je Basis-Member, einmal je SHA-1 berechnet und im App-Cache abgelegt (`<App-Cache>/base/<sha1>.json`, rund 21 000 Einträge);
- gegen die xxh3 je Member aus `.g4/build/state.json` (§2.3).

Das macht die Liste der geänderten Entitäten zu einem Nebenprodukt des Graphs, mit 0 zusätzlichen Dateizugriffen. Die Basis-ROM bleibt nach dem ersten Gebrauch als `raw::Rom` in der Sitzung im Speicher (128 MiB) oder wird je Aufruf gelesen (15 bis 60 ms). Empfohlen ist das Lesen je Aufruf, das spart Speicher.

### 3.4 Was sinnvoll zurücksetzbar ist

| Entität (Store-Asset) | Quelle in der Basis | Zurücksetzen | Bemerkung |
|---|---|---|---|
| Karte (`Map`) | Member von `land_data` | ganz; später Teile (nur Kollision, nur Gebäude) | DSPRE hängt neue Karten an, Indizes bleiben stabil |
| Matrix, Gebietsdaten (`AreaData`), Eventdatei | Member | ganz | Eventdatei: auch einzelne Events über den Codec, als Vorher-Nachher (`BeforeAfter` gibt es seit 3b.4a) |
| Header | Tabelle im ARM9 bzw. Member von `dynamicHeaders` | Feld für Feld | Mit Dynamic-Headers-Patch liegt die Basis im ARM9, das Projekt im NARC. Der Vergleich geht über den Codec, nicht über Bytes. ARM9 ist in HGSS komprimiert, `Rom::extract` packt es aus |
| Textarchiv | Member | **je Nachricht** und ganz | `TextArchiveDoc::from_member` auf beiden Seiten; neue Nachrichten (Index ≥ Basis) heißen „neu, kein Original“ |
| Scriptdatei | Member | ganz, als rotom-Quelle (Basis-Member dekompilieren) | Level-Scripts ebenso (JSON). Eine binäre Datei bleibt binär |
| Pokémon (Personal, Lernsets, Entwicklungen), Attacken, Items, Begegnungen | Member | Feld für Feld über den Codec | Pokédex-Größe und -Gewicht erst, wenn `pokeHeight` gepackt wird (§1.4) |
| Trainer (Paar Props + Party) | zwei Member | ganz | |
| Texturen, Gebäude-Modelle | Member | ganz | binär, ohne Vorschau des Unterschieds |
| neue Entitäten (Index ≥ Anzahl in der Basis) | – | **nicht**: „Kein Original“ | Löschen ist 3d |
| ARM9-Patches (Dynamic Headers, Erweiterungen), Overlays, Overlay-Tabellen (Läden, Lehrer) | – | **nicht** je Entität | gehören zum Patch-Schutz („später“) |

UI: „Auf Original zurücksetzen…“ im Menü „…“ des Inspector-Kopfs und im Kontextmenü der Liste, mit Vorher-Nachher-Rückfrage. Es ist ein Undo-Schritt im Store, kein Dateischreiben am Store vorbei. Fehlt die Basis-ROM, ist der Eintrag deaktiviert und nennt den Grund.

---

## 4. xdelta-Export

### 4.1 Kandidaten

Ein Byte geändert heißt eine Karte. Karte, Script und Text heißt: ein NARC wächst, alles dahinter verschiebt sich um mindestens 512 Bytes (3,8 bis 4,2 Mio. Bytes an gleicher Stelle verschieden).

| Kandidat | Lizenz | `cargo deny` (Freigabeliste §1.4) | Patch: ein Byte | Patch: Karte + Script + Text | Erzeugen | Anwenden | Bemerkung |
|---|---|---|---|---|---|---|---|
| **`xdelta3` 0.1.5** (Crate, bindet xdelta 3.0.12 als C ein) | Apache-2.0 (Crate und C-Quelle) | Lizenzen ok; **Advisories rot**: `shlex 0.1.1` RUSTSEC-2024-0006 (Schwachstelle), `ansi_term`, `atty`, `rand_os` (unmaintained), alle aus den Build-Abhängigkeiten `bindgen 0.52` und `rand 0.6` | 395 bis 397 B | Pt 4 779 B, HG DE 8 324 B, HG US 8 290 B | 158 bis 261 ms | 12 bis 31 ms | API nur `encode`/`decode` im Speicher mit Flags 0: keine Adler32-Prüfsumme, kein App-Header. bindgen braucht libclang beim Bauen (Windows-CI). „experimental“, seit Jahren ohne Release |
| **`oxidelta` 0.1.4** (reines Rust) | MIT | Lizenzen und Advisories **ok** | 452 bis 464 B | **Pt 2,63 MB, HG DE 682 KB, HG US 914 KB** | 25 bis 314 ms (Stufe 1 bis 9) | 11 bis 47 ms | Findet verschobene Daten schlecht. Ein größeres Fenster geht nicht (Grenze 16 MiB). Xdelta3 dekodiert die Patches korrekt (geprüft). Jung (0.1.x) |
| xdelta3-CLI 3.2.2 (Homebrew, Gegenprobe) | Apache-2.0 | – (kein Crate) | 629 B (`-S none`), 658 B (Standard LZMA) | 3,6 bis 6,2 KB (LZMA), 5,0 bis 8,6 KB (`-S none`) | 217 bis 562 ms | 63 bis 110 ms | Standard-Patches tragen LZMA als Zweitkompression und einen App-Header |
| **eigene FFI auf xdelta3.c** (Vorschlag) | Apache-2.0, die Quelle wird mit `NOTICE` mitgeliefert | nur `cc` als Build-Abhängigkeit | wie Crate | wie Crate | wie Crate | wie Crate | `xd3_config` mit `XD3_ADLER32`, App-Header über `xd3_set_appheader`, ohne Zweitkompression; rund 150 Zeilen `extern "C"` von Hand, kein bindgen |
| `rom-weaver-patches`, `open-vcdiff`-Bindings, `qbsdiff` | AGPL-3.0 bzw. Apache-2.0 (C++) bzw. BSD (kein VCDIFF) | AGPL scheidet aus | – | – | – | – | bsdiff ist kein xdelta. open-vcdiff bringt C++ und eigene Eigenheiten mit, ohne Nutzen gegenüber xdelta3 |

Alle Rückwege sind geprüft: Jeder Patch der Crates wurde mit dem eigenen Decoder **und** mit `xdelta3 -d` angewandt und mit der gebauten ROM byteweise verglichen. Ergebnis: gleich.

### 4.2 Kompatibilität

- **Rom Patcher JS** (Quelltext master, `RomPatcher.format.vcdiff.js`) kann nur anwenden. Es versteht VCD_SOURCE und VCD_TARGET, Adler32 (prüft bei `validate`) sowie die Zweitkompression DJW und LZMA. Den App-Header überspringt es. Eigene Code-Tabellen lehnt es ab, und xdelta3 schreibt keine. Ältere Fassungen von Rom Patcher JS konnten keine Zweitkompression; welche Fassung die Nutzer haben, ist offen.
- **xdelta UI** (Windows) ruft `xdelta3.exe` 3.0.x auf. Patches ohne Zweitkompression dekodiert jede 3.x-Fassung. Mit LZMA hängt es davon ab, ob das Binary LZMA eingebaut hat. Nicht geprüft: kein Windows zur Hand.
- **Folgerung:** **ohne Zweitkompression, mit Adler32.** Die Kosten sind 0 bis 2,4 KB größere Patches (5,0 statt 3,6 KB auf Platin). Dafür kann jeder gängige Patcher den Patch anwenden.

### 4.3 Basis-SHA-1 „im Patch“

VCDIFF hat kein Feld für eine Prüfsumme der Quelle. Adler32 prüft nur die Ziel-Fenster, findet eine falsche Basis also erst beim Anwenden und auch nur dann, wenn ein Fenster betroffen ist. Zwei Wege, die sich nicht ausschließen:

1. **App-Header** im xdelta-Format `<ziel>//<quelle>/`, die Quelle mit SHA-1 im Namen, zum Beispiel `Mein Hack.nds//Pokemon Platin DE [d40f19ac24d6…].nds/`. `xdelta3 printhdr` zeigt ihn, Rom Patcher JS überspringt ihn. Nachteil: Ohne `-s` sucht `xdelta3 -d` eine Datei dieses Namens und bricht ab. Das ist harmlos, aber eine Stolperfalle.
2. **Begleitdatei** `<Name>.xdelta.txt` (oder `.json`) mit Spiel, Basis-SHA-1, Ziel-SHA-1, Studio-Version und einem Satz Anleitung. Diesen Weg kennen ROM-Hack-Nutzer.

Empfehlung: beides. Der Export prüft sich selbst: Patch auf die Basis anwenden (≤ 31 ms), SHA-1 des Ergebnisses gegen die gebaute ROM (0,12 s). Erst dann wird geschrieben.

### 4.4 Ablauf in Technik › Veröffentlichen › „Patch erstellen…“

1. Die Basis-ROM wird aufgelöst (§3.2), sonst gibt es keinen Export.
2. Der Graph baut, sodass die ROM aktuell ist (§2).
3. Der Patch wird erzeugt (0,16 bis 0,26 s).
4. Die Selbstprüfung läuft (§4.3).
5. Patch und Begleitdatei werden gespeichert, Vorschlag `<Projekt>/build/<Projekt>-<Spielcode>.xdelta`. Unter `build/` erlaubt die `UserPolicy` von MCP schon heute Build-Ziele.

Gesamtdauer nach einer typischen Änderung: etwa 0,6 s auf allen drei Projekten.

---

## 5. Empfehlungen (Zusammenfassung)

1. **Graph-Modell (recommend):** Variante B. Eine kleine Schrittliste mit `Step`-Vertrag, Fingerabdrücke Member-genau über `stat` mit xxh3 bei Abweichung, early cutoff, NARC durch Einsetzen in das vorhandene gepackte NARC und ein BLZ-Cache für ARM9 und Overlays über die öffentliche ds-rom-API. ds-rom selbst bleibt voll (40 bis 60 ms), es gibt keinen ROM-Patch an Ort und Stelle.
2. **Fingerprint-Speicher (recommend):** `.g4/build/state.json` (versioniert, mit Werkzeug-Salz je Schritt aus `Cargo.lock`) und `.g4/build/blz/`. Beides kommt in die `.gitignore`-Vorlage und ist jederzeit löschbar. rotoms `compile-state.json` bleibt, der Graph entscheidet nur über den Aufruf. Die Dekodier-Prüfung in `encode_stale` wird durch das `stat`-Tor ersetzt.
3. **Basis-ROM-Herkunft (recommend):** `.g4/base.json` v2 mit `origin: "dspre"`, zugeordnet über `assign_base`. Nur ein falscher Spielcode sperrt, sonst gibt es Warnungen. Eine `.nds` neben `_DSPRE_contents` wird automatisch vorgeschlagen. Der Pfad liegt in `<App-Daten>/base-roms.json`, Schlüssel SHA-1. Fehlt die Datei, sind Export und Zurücksetzen mit Grund deaktiviert. Kopflos über `--base-rom` bzw. `G4_BASE_ROM_DIR`. Die Agenten-Freigabe wird beim Zuordnen umgeschrieben.
4. **xdelta (recommend):** xdelta3.c (Apache-2.0) mit eigener FFI über `cc`, ohne bindgen. Patches ohne Zweitkompression, mit Adler32, App-Header und Begleitdatei mit SHA-1, Selbstprüfung vor dem Schreiben. Das Crate `xdelta3` nur als Übergang, falls die FFI warten muss; dann in `deny.toml` die vier Befunde der Build-Abhängigkeiten begründet ausnehmen. `oxidelta` erst, wenn es verschobene Daten findet (Patch ≤ 10 KB im Testfall „Karte + Script + Text“).
5. **`deny.toml` (recommend):** in 4.1 anlegen: Freigabeliste wie in `gutachten-b-messung/deny.toml`, dazu begründete Ausnahmen für die acht Advisories, die es schon heute gibt (oder die betroffenen Crates aktualisieren). Erst dann taugt `cargo deny check` als Gate-Schritt.
6. **Reset je Entität (recommend):** Basisleser im Kernel (`base::open(sha1) -> BaseRom` über `raw::Rom` + `FileSystem::parse`, Member je `AssetKind`, Header über den Codec). „Auf Original zurücksetzen“ ist ein Undo-Schritt mit Vorher und Nachher, zuerst für Karte, Eventdatei, Textarchiv (je Nachricht), Trainer, Pokémon-Daten und Begegnungen. Nicht für neue Entitäten und nicht für ARM9-Patches. Die Marken „geändert“ kommen aus Hashes (Basis-Cache gegen `state.json`), nicht aus dem Lesen der Ordner.
7. **DSPRE-Ordner, die das Studio nicht packt:** in `archives.yaml` als reine Pack-Einträge aufnehmen (packed-Pfade aus DSPREs `RomInfo`), mit einem Gate „Vanilla bleibt byte-gleich“. Bis dahin warnt `build.unpacked-not-packed`.

---

## 6. Folgen für den Schnitt

**Reihenfolge.** Die Roadmap ordnet 4.1 vor 4.2. Ich empfehle, 4.1 zu teilen und so zu ordnen:

1. **4.1a „Basis-ROM und Patch“ (M−)**: `BaseMark` v2 mit `dspre`, Zuordnen mit Prüfung und Vorschlag, `base-roms.json`, Umschreiben der Freigabe, `deny.toml`, xdelta-FFI, Export in Veröffentlichen, gated Rebuild byte-gleich für die drei DSPRE-Projekte.
2. **4.2 „Build-Graph“ (M)**: BLZ-Cache, `state.json`, Tore, Einsetzen ins NARC, Pack-Einträge für die DSPRE-Ordner, Anzeige der Schritte, Prüfungen.
3. **4.1b „Auf Original zurücksetzen“ (M)**: Basisleser, Reset je Entität für die Editoren, Marken „geändert“ aus den Hashes des Graphs.

Gründe: 4.1a ist klein und liefert sofort zweierlei, den byte-gleichen Beweis und einen veröffentlichbaren Patch. 4.2 braucht 4.1a nicht, und 4.1a braucht 4.2 nicht, denn der Export ruft den heutigen Build. Beide berühren getrennte Dateien (`start.rs`, Einstellungen, Technik › Veröffentlichen gegen `build.rs`, neues `build/`-Modul, Technik › Prüfen und Bauen) und können **parallel in zwei Worktrees** laufen. 4.1b profitiert von den Member-Hashes aus 4.2 und kommt danach. 4.3 (Textprojekt) setzt auf dem `Step`-Vertrag auf und fügt nur den Schritt „Entität → Member“ vorn an.

**Größen.** 4.1a: Kernel etwa 400 Zeilen plus FFI 150, App etwa 300, Tests. 4.2: Kernel etwa 800 bis 1 000 Zeilen (Graph, Zustand, Cache, Einsetzen), App-Anzeige etwa 200. 4.1b: Kernel etwa 300, je Editor ein Menüeintrag mit Vorher-Nachher, also eher breite als tiefe Arbeit.

**Gate-Vorschläge mit Zahlen:**

- **4.1a**
  - Gated je Projekt (Pt DE, HG DE, HG US): `g4k base assign` auf eine frische Kopie des DSPRE-Ordners, dann `g4k build`. Die **SHA-1 der gebauten ROM muss gleich der `.nds` sein** (`d40f19ac…`, `13b6853a…`, `4fcded0e…`), gemessen hier: gleich.
  - Spielcode einer fremden ROM: Zuordnen wird abgelehnt. Unbekannte SHA-1: Warnung, kein Fehler.
  - xdelta: Patch nach einer Kartenänderung **≤ 1 KiB** (gemessen 0,4 KB), nach Karte + Script + Text **≤ 16 KiB** (gemessen 4,8 bis 8,3 KB). Erzeugen ≤ 1 s, Selbstprüfung grün, **mit `xdelta3 -d` angewandt gleich** (gated, wenn `xdelta3` im PATH ist). Rom Patcher JS im Browser von Hand oder über die Test-Umgebung mit der Datei des Nutzers.
  - Fehlende Basis-ROM: Export deaktiviert mit Grund (Test-Umgebung).
  - `cargo deny check licenses advisories` grün mit der neuen `deny.toml`.
- **4.2**
  - Gated je Projekt: nach je einer Änderung an Karte, Text und Script gilt für „Hier spielen“ beziehungsweise `g4k build --timings`: **≤ 0,5 s warm** (gemessen 0,17 bis 0,2 s für die Karte, heute 1,0 bis 1,2 s Pt und 4,4 bis 5,2 s HGSS). Ohne Änderung **≤ 0,2 s**. Die Zeiten je Projekt und Schritt stehen im Gate-Bericht.
  - **Ergebnis byte-gleich zum vollen Build ohne Cache** (`.g4/build/` gelöscht).
  - Simulierte DSPRE-Änderung (Member an Ort und Stelle überschrieben, mtime gleich groß gesetzt): Der Graph erkennt sie, über Größe, Inode oder die racy-Regel.
  - Werkzeug-Salz geändert (Test-Hook): voller Build.
  - Kalter erster Build nur protokollieren, kein Grenzwert: Das schwankt mit der Last zwischen 1,3 und 23 s.
  - Die Pack-Einträge für die DSPRE-Ordner lassen den Vanilla-Rebuild byte-gleich.
  - `scripts/gate.sh` mit dem Schritt.
- **4.1b**
  - Je Art aus der Tabelle in §3.4: ändern, „Auf Original zurücksetzen“, danach ist der Member byte-gleich zur Basis. ⌘Z stellt die Änderung wieder her. Dauer je Reset ≤ 0,2 s (gemessen < 80 ms). Neue Entität: Eintrag deaktiviert.

---

## 7. Offene Fragen

1. Schreibt DSPRE beim Speichern alle Ordner unter `unpacked/` in ihre NARCs zurück, auch `pokemonBattleSprites` und `synthOverlay`, oder nur die, die es kennt? Davon hängt ab, wie dringend die Pack-Einträge (§1.4, Punkt 2) sind. **Hand in DSPRE.**
2. Soll die Basis-SHA-1 in den App-Header (Stolperfalle bei `xdelta3 -d` ohne `-s`) oder nur in die Begleitdatei? Empfehlung: beides, aber die Entscheidung liegt beim Nutzer.
3. Welche Patcher sollen garantiert funktionieren? Vorschlag: xdelta3 3.x (CLI und xdelta UI), Rom Patcher JS ab der Fassung mit LZMA. Auf Windows nicht geprüft.
4. Darf 4.1 die schon heute roten Advisories mit Ausnahmen in `deny.toml` dokumentieren, oder sollen `zip`, `tokio` und `quick-xml` vorher aktualisiert werden (eigener kleiner Schritt)?
5. No-Intro-Hashes in `base_roms.yaml` (nur Prüfsummen, keine Daten): Welche Spiele und Revisionen sollen hinein? Heute stehen dort nur die drei Testprojekt-Dumps mit dem Label „Testprojekt“.
6. Kalter Seiten-Cache: Auf diesem Rechner schwankte das Lesen kleiner Dateien um den Faktor 20, durch Last und vielleicht einen Dienst, der Dateizugriffe prüft. Ist das typisch für Nutzer (Windows mit Defender)? Das Einsetzen ins NARC ist genau dafür gebaut, aber ein Gate mit Grenzwert auf kalte Zeiten wäre wackelig.
7. Ein Upstream-Vorschlag an ds-rom (Cache für die BLZ-Kompression als `RomLoadOptions`), oder bleibt es beim Weg über die öffentliche API mit gated Gleichheitstest?

---

## Anhang A: Messaufbau und Skripte

Ordner [`gutachten-b-messung/`](gutachten-b-messung/). Die Pfade in den Skripten sind absolut auf diesen Rechner gesetzt. Gebaut wird mit `CFLAGS=-D_POSIX_C_SOURCE=200809L` (wie `.cargo/config.toml`) und eigenem `CARGO_TARGET_DIR`.

- `Cargo.toml`, `build.rs`, `src/main.rs`: das Programm `proto-b`, Pfad-Abhängigkeit auf `g4-kernel`, `g4-formats` und ds-rom `8a9c151`. Unterbefehle:
  - `full`: `build::build`;
  - `stages`: die Schritte von `build` einzeln getimt;
  - `scan`: `stat` gegen xxh3;
  - `basecmp`: Basis-ROM gegen Ordner;
  - `delta`: oxidelta;
  - `edit-map`, `edit-text`, `edit-script`, `adopt`, `expand-texts`: typische Änderungen;
  - `dsrom-split`: BLZ getrennt gemessen;
  - `inc`: Prototyp von Variante B;
  - `sha1`.
- `messung-1.sh <spiel>`: voller Build kalt und warm, Stufen, Scan, Basisvergleich. `messung-2.sh <spiel> <karte> auto <archiv>`: Karte, Scripts übernehmen, Script, alle Texte als JSON, Text.
- `mark_dspre.py`: schreibt die Test-Marke `origin: "dspre"` (nur Testkopien).
- `x3/`: Messung mit dem Crate `xdelta3`. `xdelta-cli.sh`: Gegenprobe mit dem CLI.
- `deny.toml`: die Freigabeliste der Lizenzen, wie sie der Abhängigkeitsbaum heute nutzt. Geprüft mit `cargo-deny 0.20.2` (in den Scratchpad installiert).

Testdaten: APFS-Klone (`cp -c -R`) der drei DSPRE-Ordner und `.nds` nach `Roms/_testkopien/4-gutachten-b-{pt,hgde,hgus}`, Ausgaben in `…-out/`. Alles ist am Ende gelöscht, die Originale unter `Roms/` sind unberührt. Das xdelta-CLI kam per `brew install xdelta` (3.2.2) dazu und ist installiert geblieben.
