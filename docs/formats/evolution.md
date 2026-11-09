# Entwicklungen (`evolutions/NNNN`)

**In English:** Documents the evolution file of one species (`evolutions/NNNN`), a member of exactly
44 bytes. It holds seven entries of 6 bytes each (method, parameter and target species), followed by
2 bytes of padding. What the parameter means depends on the method, and the document lists this for
the vanilla data and states the limits of the codec. The layout was checked on Platinum DE (member
397, Staravia, level 34 to Staraptor). The code is `crates/g4-formats/src/data/evolution.rs`; all
values are little endian.

Quelle: DSPRE `ROMFiles/EvolutionFile.cs`; Recherche
`docs/research/2026-10-23-3b3/dspre-report.md` §1.4 „Evolutionen“. Die Werte sind
an Platin DE Member 0397 (Staravia, Stufe 34 zu Staraptor) geprüft.
Implementierung: `crates/g4-formats/src/data/evolution.rs` (`Evolution`,
`EvoEntry`). Alle Werte Little Endian. Ein Member ist genau 44 Bytes.

## Byte-Layout

Sieben Einträge zu je 6 Bytes, danach 2 Bytes Padding.

| Offset | Länge | Feld | Bemerkung |
|--------|-------|------|-----------|
| 0x00 + 6·i | u16 | `method<i>` | Entwicklungsmethode, 0 ist „keine“ |
| 0x02 + 6·i | u16 | `param<i>` | Parameter, Bedeutung hängt von der Methode ab |
| 0x04 + 6·i | u16 | `target<i>` | Zielart (Personal-Index) |
| 0x2A | 2 Bytes | `pad_2a` | Padding, in Vanilla immer 0 |

`i` läuft von 0 bis 6. `entry(i)` liefert `EvoEntry { method, param, target }`,
`set_entry(i, e)` schreibt genau die sechs Bytes dieses Eintrags.

## Parameter je Methode (Vanilla)

In Vanilla kommen die Methoden 0 bis 26 vor. Der Codec deutet sie nicht; für die
Oberfläche gilt:

| Parameter | Methoden |
|-----------|----------|
| Stufe | 4, 8 bis 14, 22, 23 |
| Gegenstand | 6, 7, 16 bis 19 |
| Attacke | 20 |
| Art im Team | 21 |
| Schönheit | 15 |
| ohne Parameter | 0 bis 3, 5, 24 bis 26 (24 bis 26 sind Orte) |

## Grenzen des Codecs

`read` lehnt nur eine Länge ungleich 44 ab (`BadSize`). Leere und ungültige
Einträge bleiben an ihrer Stelle; anders als DSPRE (das nur gültige Einträge
nach vorn rückt und mit 0 auffüllt) verschiebt dieser Codec nichts. Es gilt
`write(read(bytes)) == bytes`. Ein Index `i >= 7` bricht ab (Programmfehler).
