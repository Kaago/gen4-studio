# Overworld-Tabelle (Objekt-Sprites)

Quelle: DSPRE `RomInfo.SetOWtable` und `ReadOWTable` (Offsets je Spielcode, Eintragsformat,
Terminator); es sind Tatsachen über die Spielbinaries, keine Übernahme von Programmcode.
Nachgeprüft am 22.10.2026 an den entpackten Projekten (siehe unten). Die Daten stehen in
`crates/g4-kernel/metadata/overworld_tables.yaml`, die Implementierung in
`crates/g4-kernel/src/sprites.rs`. Werte sind Little Endian.

Ein Objekt-Event trägt `graphics_id` (DSPRE „OW Entry“). Das ist kein Dateiindex: die
**Overworld-Tabelle** des Spiels bildet ihn auf ein Member des Sprite-NARC ab.

## Wo die Tabelle liegt

Overlays liegen bei ds-rom entpackt auf der Platte (`arm9_overlays/ovNNN.bin`); das Flag
`compressed: true` in `arm9_overlays/overlays.yaml` beschreibt das ROM, nicht die Datei.
Es ist also keine BLZ-Dekompression nötig. `overlays.yaml` ist eine Liste
`overlays: [{ id, base_address, code_size, bss_size, ctor_start, ctor_end, file_id,
compressed, signed, file_name }]`; `Project::overlay(id)` liest daraus `base_address`
(RAM-Adresse), `code_size` und den Pfad (`file_name`).

| Familie | Lage | Eintrag |
|---|---|---|
| Diamant/Perl, Platin | Overlay 5 (`ov005.bin`), fester Dateioffset je Spielcode (`overlay5_offset`) | 8 Byte: `u32 entry`, `u32 sprite` |
| HeartGold/SoulSilver | Overlay 1 (`ov001.bin`): an der RAM-Adresse `overlay1_pointer` je Spielcode steht ein `u32`-Zeiger auf die Tabelle | 6 Byte: `u16 entry`, `u16 sprite`, `u16 props` |

Die Tabelle endet mit dem ersten Eintrag mit `entry == 0xFFFF` (bei Diamant/Perl und
Platin als `u32`, bei HGSS als `u16`); der Terminator zählt nicht mit. `entry` ist die
`graphics_id`, `sprite` die Nummer des Members im Sprite-NARC. Kommt eine `graphics_id`
mehrmals vor, gilt der erste Eintrag (das Spiel sucht linear). `props` (HGSS) wird nicht
gelesen.

HGSS im Einzelnen: Dateioffset des Zeigers = `overlay1_pointer - base_address`; der
Zeigerwert ist eine RAM-Adresse, der Dateioffset der Tabelle ist `Zeiger - base_address`.
Liegt die Adresse des Zeigers oder sein Wert nicht in `base_address <= a < base_address +
code_size` von Overlay 1, ist die Tabelle in ein synthetisches Overlay oder (ov131) verlegt
worden; das wird nicht unterstützt und ist ein Fehler („lies outside overlay 1“).
Eine Tabelle ohne Terminator vor Dateiende ist ebenfalls ein Fehler.

Offsets und Zeigeradressen je Spielcode: siehe die YAML-Datei (DSPRE `SetOWtable`). Ein
Spielcode ohne Eintrag (etwa koreanisch) ist `UnknownGame`.

Nachgeprüft:

| Projekt | Spielcode | Fundort | Einträge | Stichprobe |
|---|---|---|---|---|
| Platin DE | CPUD | `ov005.bin` ab 0x2BC50 | 440 | `graphics_id` 0 → Sprite 90 |
| HeartGold DE | IPKD | Zeiger bei RAM 0x021F92DC (Dateioffset 0x139FC) = 0x02207488, Tabelle bei 0x21BA8 in `ov001.bin` (Basis 0x021E5900, `code_size` 148064) | 901 | `graphics_id` 1 → Sprite 0 |
| HeartGold US | IPKE | Zeiger bei RAM 0x021F92FC | 901 | `graphics_id` 1 liefert einen Sprite |

In allen drei Projekten lassen sich alle Einträge zu einem Sprite dekodieren.

## Sprite-NARC und Rahmen

Die Sprites liegen im gepackten NARC `files/data/mmodel/mmodel.narc` (Diamant/Perl, Platin;
470 Member in Platin DE, davon 421 BTX0) bzw. `files/a/0/8/1` (HGSS; 863 Member, 832 BTX0).
Member `sprite` ist ein BTX0 (siehe `nsbtx.md`) mit einer Textur je Rahmen und mindestens
einer Palette. Die übrigen Member sind BMD0-Modelle oder andere Daten; ein Member, das kein
BTX0 ist, bedeutet „kein Sprite“. DSPRE rendert Textur `i` mit Palette 0; `i` hängt von der
Texturanzahl `n` und der Richtung ab (`dir` des Events: 0 oben, 1 unten, 2 links, 3 rechts,
sonst unten):

| n | oben | unten | links | rechts |
|---|---|---|---|---|
| ≤ 1 | 0 | 0 | 0 | 0 |
| ≤ 4 | 0 | 1 | 2 | 3 |
| ≤ 8 | 0 | 2 | 4 | 6 |
| ≤ 16 | 0 | 11 | 2 | 4 |
| > 16 | 0 | 27 | 2 | 4 |

Liegt der Rahmen jenseits der letzten Textur, nimmt der Kernel Rahmen 0. Palettenindex 0
ist immer transparent (unabhängig vom `color0`-Flag der Textur).

## 3D-Overworlds

Bei Diamant/Perl und Platin sind die IDs 91 bis 96 und 101 bis 116 laut DSPRE 3D-Overworlds
(Schilder, variablenabhängige Figuren) ohne Sprite; `is_3d_overworld` meldet sie, der Kernel
liefert für sie keinen Sprite. In den Tabellen von Platin DE kommen diese IDs nicht vor.
HGSS hat keine solchen IDs.

## Verhalten des Kernels

`SpriteCache::object_sprite(project, graphics_id, facing)` liest Tabelle und NARC je Cache
einmal (Fehler werden gemerkt und bei jedem Aufruf wiederholt) und cacht jeden dekodierten
Sprite je `(graphics_id, facing)`. `Ok(None)` heißt „Marker zeigen“: kein Tabelleneintrag,
3D-ID, Member fehlt, Member ist kein BTX0, oder der Sprite ist nicht les- oder dekodierbar
(beschädigte Daten, Texturformat 5). Fehler gibt es nur für Tabelle und NARC selbst.
