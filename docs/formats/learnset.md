# Lernsets (`learnsets/NNNN`)

Quelle: DSPRE `ROMFiles/LearnsetData.cs`, pret `pokeplatinum`
(`MAX_LEARNSET_ENTRIES`); Recherche
`docs/research/2026-10-23-3b3/dspre-report.md` §1.4 „Learnsets“. An allen drei
Vanilla-Projekten geprüft (Platin DE, HeartGold DE und US, je 508 Member).
Implementierung: `crates/g4-formats/src/data/learnset.rs` (`Learnset`,
`LearnEntry`, `MAX_ENTRIES`). Alle Werte Little Endian. Ein Member hat variable
Länge.

## Byte-Layout

Eine Folge von `u16`-Wörtern, beendet durch `0xFFFF`, danach Auffüllung.

| Bits | Feld | Bemerkung |
|------|------|-----------|
| 0 bis 8 | `move_id` | Attacke, 0 bis 511 |
| 9 bis 15 | `level` | Stufe, 0 bis 127 |

Der Terminator `0xFFFF` entspräche der Attacke 511 auf Stufe 127; dieser Eintrag
ist deshalb nicht darstellbar.

Das Member ist in Vanilla auf ein Vielfaches von 4 Bytes mit `0x0000`
aufgefüllt: in Platin enden 265 der 508 Member auf `ffff0000` und 243 auf
`xxxx ffff` (der Terminator liegt dann schon auf der Vier-Byte-Grenze); in beiden
HeartGold-Projekten sind es 268 und 240. In allen 3 × 508 Vanilla-Lernsets ist der
Rest hinter dem Terminator leer oder reine `0x0000`-Füllung.

## Grenze von 20 Einträgen

pret begrenzt den Puffer des Spiels auf `MAX_LEARNSET_ENTRIES = 20` Einträge
plus Terminator. Mehr überläuft den Puffer. Alle Vanilla-Lernsets haben
höchstens 20 Einträge (Höchstwert 20). Der Codec kennt die Grenze nur als
Konstante `MAX_ENTRIES = 20`; er erzwingt sie nicht, die Oberfläche warnt.

## Struktur im Speicher

```text
Learnset { entries: Vec<LearnEntry { move_id, level }>,
           tail: Vec<u8>, terminated: bool, read_count: usize }
```

- `read` liest Wörter bis `0xFFFF`. Alles hinter dem Terminator wandert
  unverändert nach `tail`. Doppelte Einträge und die Reihenfolge bleiben
  erhalten (DSPRE verwirft Duplikate, dieser Codec nicht).
- Fehlt der Terminator, endet das Lesen am Ende der Daten: `terminated` ist
  `false`, `tail` bleibt leer.
- `read_count` ist die Zahl der gelesenen Einträge.

## Schreiben

- `entries.len() == read_count`: Einträge, `0xFFFF`, dann `tail` unverändert.
  Unveränderte Daten kommen so byte-identisch zurück. Ein Member ohne
  Terminator wird dabei mit Terminator geschrieben (Einträge, `0xFFFF`, kein
  `tail`); es bekommt keine Auffüllung.
- Sonst (Zahl der Einträge geändert): Einträge, `0xFFFF`, dann `0x0000`-Füllung
  auf ein Vielfaches von 4 Bytes. Der alte `tail` entfällt.
- `move_id` und `level` werden beim Schreiben auf 9 und 7 Bit gekürzt und
  laufen nie in das Nachbarfeld über.

## Grenzen des Codecs

`read` lehnt nur ein einzelnes übriges Byte ab, wenn kein Terminator vorkam
(`BadSize`); nach dem Terminator darf der Rest beliebig lang sein. Unbekannte
Werte werden nie abgelehnt. Es gilt `write(read(bytes)) == bytes` für jedes
Member mit Terminator und für jedes Vanilla-Member.
