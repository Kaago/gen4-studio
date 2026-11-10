# Ei-Attacken (`eggMoves/0000`, Overlay 5)

**In English:** Documents the egg move table (`eggMoves/0000`), which lists the moves a hatched
Pokémon can learn from its parent. It is a sequence of little-endian `u16` words in which `20000 +
species` starts the entry of a species; in HeartGold/SoulSilver it is an archive member, in Platinum
it lies in overlay 5 within a fixed budget of words. The layout comes from Gen4 Studio's own
measurements on Platinum DE and HeartGold DE; uxie 0.9.7 served only as a comparison. The codec is
`crates/g4-formats/src/data/egg_moves.rs`; location, source and sets are handled in
`crates/g4-kernel/src/data/egg_moves.rs` and `overlay_tables.rs`.

Quelle: eigene Messung, `docs/research/2026-10-30-phase-3d/gutachten-b-flags-variablen-live.md`
§5 mit `gutachten-b-eier.py` (Platin DE, HeartGold DE). uxie 0.9.7 (`egg_move_data`) nur
zum Vergleich. Implementierung: `crates/g4-formats/src/data/egg_moves.rs` (Codec
`EggMoves`), `crates/g4-kernel/src/data/egg_moves.rs` (Ort, Quelle, Sätze),
`crates/g4-kernel/src/data/overlay_tables.rs` mit `metadata/overlay_tables.yaml` (Lage im
Overlay). Alle Werte Little Endian.

## Format

Eine Folge von `u16`-Wörtern:

| Wort | Bedeutung |
|------|-----------|
| `20000 + Art` | beginnt den Eintrag der Art (Marke, `SPECIES_BASE`) |
| `< 20000` | eine Attacke des laufenden Eintrags |
| `0xFFFF` | Ende der Tabelle (`TERMINATOR`); das Spiel liest bis zum ersten |

`EggMoves { entries, terminated, tail, budget }`: `entries` in Tabellenreihenfolge, eine
Marke ohne Attacken bleibt ein Eintrag mit leerer Liste. `tail` sind die Bytes nach dem
Ende. Ein Wort unter 20000 vor der ersten Marke ist `BadSignature`. `words()` zählt Marken,
Attacken und das Ende.

`set_moves(art, liste)` ersetzt die Liste; eine leere Liste entfernt den Eintrag; ein neuer
Eintrag steht vor dem ersten Eintrag einer höheren Art (aufsteigend). Ändert sich die
Wortzahl, bekommt die Tabelle ihr Ende (auch wenn sie ohne eines gelesen wurde) und `tail`
wird zur Null-Auffüllung auf vier Bytes, mit Budget (Overlay) leer, dann füllt `write()` mit
`0xFFFF`-Wörtern auf; sonst schreibt `write()` den Rest wie gelesen zurück
(`write(read(bytes)) == bytes`).

## Orte je Spiel

| Spiel | Ort | Quelle (`EggMovesSource`) |
|-------|-----|---------------------------|
| HeartGold/SoulSilver | NARC `a/2/2/9`, ein Member (`unpacked/eggMoves/0000`) | `narc` |
| Platin mit DSPRE-Patch | `files/zcustom/egg_moves.narc` oder nur `unpacked/eggMoves/0000` | `patch` |
| Platin ohne Patch | `arm9_overlays/ov005.bin`, fester Ausschnitt | `overlay` |
| Diamant/Perl | nicht bekannt, keine Ei-Attacken im Studio | – |

Im Kernel ist die Tabelle eine Ein-Asset-Art `AssetKind::EggMoves` (Index 0) wie die
Header-Tabelle; `egg_moves_place` wählt den Ort.

**Patch-Erkennung:** Gibt es `files/zcustom/egg_moves.narc` oder den Ordner
`unpacked/eggMoves`, liest und schreibt das Studio den Member des NARC und fasst Overlay 5 nie
an (DSPRE biegt den Code zum NARC um). Der Ordner allein reicht, weil die `.gitignore` des
DSPRE-Layouts den NARC jedes entpackten Ordners ausschließt: ein Klon eines gepatchten
Projekts hat nur den Ordner. Auf Platin ohne Patch legt das Studio den Ordner nie an,
Vanilla-Projekte haben keinen von beiden. Der Build packt `unpacked/eggMoves` nur, wenn der
Ordner da ist.

## Lage im Overlay (Platin)

`overlay_tables.yaml` nennt je Spielcode Overlay, Datei-Offset im entpackten Overlay und
Budget. Gemessen ist nur Platin DE (`CPUD`): Offset `0x2923E`–`0x2A12A`, Budget 3 820 Byte;
die Overlay-Basis `0x021D0D20` legt die Tabelle im RAM an `0x021F9F5E`. Die Tabelle ist genau
voll: 198 Marken + 1 711 Attacken + Ende = 1 910 Wörter. Die anderen Platin-Codes stehen als
`unverified` da; uxie nennt `0x29222` für eine ungenannte andere Sprachfassung (Abstand
`0x1C` zu DE), das ist keinem Code zugeordnet. Ohne geprüfte Lage ist die Tabelle gesperrt:
„Ei-Attacken: Lage in Overlay 5 für CPUE nicht geprüft“.

## Signatur

Vor jedem Lesen (und damit vor jedem Schreiben) prüft `EggMoves::signature_ok` den
Ausschnitt: erstes Wort `20001`, letztes Wort des Budgets `0xFFFF`, jedes Wort dazwischen
`0xFFFF` oder unter `20000 + 600`. Fällt sie durch (andere Sprachfassung, verschobene
Tabelle, fremder Patch), ist die Tabelle gesperrt: „Ei-Attacken: Overlay 5 trägt an 0x2923E
keine Tabelle (Signatur fehlt)“. Der Store schreibt keinen Ausschnitt, der die Signatur
verlieren würde (etwa ohne Eintrag für Art 1), und lehnt so eine Änderung ab.

## Budget

Aus dem Overlay gelesen trägt die Tabelle `budget = Some(3820)` (Bytes), aus einem Member
`None`. `write()` füllt bis zum Budget mit `0xFFFF`-Wörtern auf, so bleibt die Signatur nach
einem Entfernen gültig, und gibt `Err(wörter)` zurück, wenn Marken + Attacken + Ende mehr
Wörter sind, als das Budget fasst. Der Store lehnt dann ab: „Ei-Attacken-Tabelle voll: 1 910
von 1 910 Einträgen“ (ohne Verlaufsschritt). Tauschen geht immer, Hinzufügen nur gegen
Entfernen an anderer Stelle. Eine Verschiebung über das Budget hinaus gibt es nur über den
DSPRE-Patch.

## Messung (Vanilla)

| | HeartGold DE | Platin DE |
|---|---|---|
| Ort | Member `a/2/2/9` 0000 | `ov005.bin` `0x2923E` |
| Größe | 4 092 Byte, ohne Rest | 3 820 Byte, genau voll |
| Arten / Attacken / höchstens je Art | 198 / 1 847 / 14 | 198 / 1 711 / – |

Der gated Test `vanilla_egg_moves_read_with_the_measured_counts` prüft diese Zahlen und
`write()` byte-gleich zum Member bzw. Ausschnitt.

## Im Textprojekt

Im Textprojekt ist die Tabelle das Feld `egg_moves` jeder Spezies-Datei (`data/species/NNNN.json`,
die Attacken des Eintrags als IDs, leer für eine Marke ohne Attacken, fehlend ohne Eintrag) und
eine Einheit `egg_moves` über alle Spezies-Dateien; in Platin ohne Patch blendet die Basisschicht
den Ausschnitt in `ov005.bin` aus. Einzelheiten, Code `textproject.egg-moves-locked` und
Schema-Version: [text_project.md](text_project.md) „Ei-Attacken“. Mit DSPRE-Patch merkt sich das
Manifest den Patch (`eggMovesPatch`), denn ein Git-Klon hat weder `files/zcustom/egg_moves.narc`
noch `unpacked/eggMoves/`: der Import legt den Ordner aus den Spezies-Dateien wieder an, statt
die Tabelle in Overlay 5 zu schreiben.

Ein Ausschnitt in Overlay 5, dessen Rest hinter dem Ende nicht aus `0xFF` besteht (die Signatur
hält trotzdem), wird byte-gleich zurückgeschrieben, solange die Zahl der Wörter bleibt; ändert
sie sich, füllt `write()` mit `0xFFFF`-Wörtern auf. Lässt sich die Overlay-Tabelle
(`arm9_overlays/overlays.yaml`) nicht lesen, sind die Ei-Attacken gesperrt („Ei-Attacken:
Overlay 5 lässt sich nicht finden (…)“), statt dass ein Laden oder Import scheitert.
