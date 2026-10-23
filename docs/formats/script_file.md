# Scriptdateien (`scr_seq`) und Level-Scripts

Quelle: rotom v0.7.0 (`src/compiler/codegen.rs` `emit_script_file`, `src/decompiler/disassembler.rs`
`detect_script_type`, `parse_jump_table`, `src/decompiler/levelscript.rs` `LevelScript::from_bytes`,
`to_bytes`) und uxie 0.9.7 (`src/script_file/global_script_table.rs`, `script_resolution.rs`).
Es sind Tatsachen über die Spieldaten, keine Übernahme von Programmcode. Die Implementierung
steht in `crates/g4-kernel/src/scripts.rs`. Alle Zahlen sind Little Endian.

## Das Script-Archiv

Ein NARC mit einem Member je Scriptdatei (`unpacked/scripts/NNNN`, vier Ziffern). Packpfade
je Familie in `crates/g4-kernel/metadata/archives.yaml` (Platin `files/fielddata/script/scr_seq.narc`,
HGSS `files/a/0/1/2`). Ein Member ist entweder

- eine **Scriptdatei** (Bytecode mit Sprungtabelle),
- ein **Level-Script** (Init-Script einer Map, ohne Bytecode) oder
- **leer** (0 Byte; die Diamant-Vanilla-ROM hat einen).

Ob ein Member Level-Script oder Script ist, steht nirgends im Member. Welche Datei ein Header
verwendet, sagt der Map-Header: `script_file_id` (Scripts der Map) und `level_script_id`
(Level-Script der Map), siehe uxie `map_header`.

## Scriptdatei

```
+0   u32[n]   Sprungtabelle: je Script ein Eintrag
+4n  u16      Endemarke 0xFD13 (Bytes 13 FD), meist einmal, selten keine oder zwei
...           Bytecode der Scripts und Bewegungslisten
```

- **Eintrag `i`** (ab 0) gehört zum Script mit der Nummer `i + 1`, im Quelltext `script Name #<i+1>:`.
  Der Wert ist der Abstand vom Ende des Eintrags (`Position + 4`) zum ersten Befehl des Scripts.
  Lücken in der Nummerierung füllt rotom mit dem Ziel des nächsten definierten Scripts.
- **Endemarke `0xFD13`.** Der Decompiler sucht sie als erstes Auftreten der zwei Bytes; fehlt sie,
  endet die Tabelle beim ersten Eintrag, dessen Ziel außerhalb der Datei liegt. rotom merkt sich
  die Anzahl der Marken (0, 1 oder mehr) je Member als *Quirk* (`jump_table_end_marker_count`,
  siehe `rotom_project.md`), damit eine Neuübersetzung dieselben Bytes schreibt. In Platin
  haben die Member 101, 164 und 365 (DE) keine Marke.
- **Bewegungslisten** (`action`) beginnen an einer durch 4 teilbaren Adresse.
- Der Befehlssatz (Opcodes, Argumente) steht in der Befehlsdatenbank
  (`crates/g4-kernel/metadata/scrcmd/`), nicht hier.
- **Textliterale** (`Message "…"`) hängt rotom an das Textarchiv des Headers an (uxie 0.9.7
  `workspace.rs` `find_or_add_message`), verwendet aber eine Nachricht wieder, die nur aus Leerraum
  besteht (leer oder Leerzeichen): eine angefügte leere Nachricht kann daher von einem späteren
  Literal belegt werden.

Erkennung Script gegen Level-Script (rotom `detect_script_type`): genau vier Nullbytes sind ein
leeres Level-Script; ein Member mit einer Endemarke an einer durch 4 teilbaren Position ist ein
Script; ohne Marke ist es ein Level-Script, wenn Byte 6 nicht 0 ist oder die Bytes 4 und 5 beide 0
sind, sonst ein Script.

## Level-Script

```
Kopf:     Folge von Einträgen, endet mit 0x00
          0x01 u32 rel   Variablenbedingungen: rel ist der Abstand vom Ende des Feldes zur Tabelle
          0x02 u32 id    OnTransition: Script id beim Betreten der Map
          0x03 u32 id    OnResume: Script id nach dem Laden, kann sich wiederholen
          0x04 u32 id    OnLoad: Script id nach dem Laden des Layouts, vor dem Zeichnen
Tabelle:  (u16 var, u16 value, u16 script_id)*, endet mit u16 0 (var == 0 beendet die Tabelle)
Ende:     aufgefüllt auf ein Vielfaches von 4 Bytes
```

- Ein leeres Level-Script sind vier Nullbytes. Die Reihenfolge der Kopfeinträge bleibt erhalten
  (rotom hält sie in `header_entries`); nur der erste `0x01` zählt, und `var == 0` in der Tabelle
  wäre die Endemarke, daher verweigert rotoms `validate` beides.
- Zusätzliche Nullbytes hinter der Ausrichtung (`padding`) merkt sich rotom als Quirk
  (`levelscript_padding`), sie stehen nicht im JSON.
- Die JSON-Form (`expanded/scripts/NNNN.json`) ist rotoms serde-Darstellung von `LevelScript`:
  `{"header_entries": [{"type": "on_load", "script_id": 1}], "var_conditions": [{"var": 16384, "value": 1, "script_id": 4}]}`
  mit den Typen `on_transition`, `on_resume`, `on_load`, `on_var_condition`.

## Script-Nummern und gemeinsame Scripts

Ein Event (Warp, NPC, Trigger) nennt eine Script-Nummer `n`:

- `n < 2000`: Script Nummer `n` (Slot `#n`) in der Scriptdatei des Map-Headers.
- `n >= 2000`: **gemeinsames Script**. Das Spiel sucht den ersten Eintrag einer absteigend
  sortierten Tabelle mit `n >= min`; der Eintrag nennt Scriptdatei und Textarchiv, der Slot ist
  `n - min + 1`. Mehrere Bereiche können dieselbe Datei verwenden (die Trainerbereiche 3000 und 5000
  in Platin).

Platin (westlich, uxie `platinum_western_hardcoded`, aus `ScriptContext_LoadAndOffsetID`):

| `min` | Datei | Textarchiv | `min` | Datei | Textarchiv |
|---:|---:|---:|---:|---:|---:|
| 10490 | 499 | 499 | 9300 | 406 | 374 |
| 10450 | 500 | 16 | 9200 | 423 | 423 |
| 10400 | 400 | 203 | 9100 | 0 | 11 |
| 10300 | 1051 | 552 | 9000 | 213 | 221 |
| 10200 | 407 | 379 | 8970 | 425 | 7 |
| 10150 | 1116 | 621 | 8950 | 498 | 498 |
| 10100 | 1115 | 622 | 8900 | 424 | 424 |
| 10000 | 409 | 381 | 8800 | 497 | 497 |
| 9950 | 411 | 383 | 8000 | 408 | 380 |
| 9900 | 397 | 213 | 7000 | 404 | 369 |
| 9800 | 212 | 217 | 5000 | 1114 | 213 |
| 9700 | 422 | 422 | 3000 | 1114 | 213 |
| 9600 | 412 | 213 | 2800 | 413 | 397 |
| 9500 | 501 | 501 | 2500 | 1 | 17 |
| 9400 | 426 | 426 | 2000 | 211 | 213 |

HeartGold/SoulSilver: 30 Einträge zu je drei `u16` (`min`, Datei, Textarchiv) in `arm9.bin`. Der
Zeiger auf die Tabelle liegt bei `0x40164` (andere Sprachen: Japanisch `0x3FEB0`, Spanisch
HeartGold `0x4015C`, Koreanisch HeartGold `0x403FC`, SoulSilver `0x403F4`) und ist eine RAM-Adresse
ab `0x02000000`. Die Platin-Tabellen der anderen Sprachen und die westliche Diamant/Perl-Tabelle
stehen in uxie `global_script_table.rs`; der Editor löst für Diamant/Perl kein gemeinsames Script
auf.
