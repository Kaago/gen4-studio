# Scriptdateien (`scr_seq`) und Level-Scripts

**In English:** Documents the script archive (`scr_seq`) and its members: script files (bytecode
with a jump table), level scripts (the init script of a map, without bytecode) and shared scripts.
It explains script numbers, the shared scripts, and how the dialog of a character is found from its
script. The archive is `files/fielddata/script/scr_seq.narc` in Platinum and `files/a/0/1/2` in
HeartGold/SoulSilver. The format knowledge comes from rotom v0.7.0 and uxie 0.9.7. The code is
`crates/g4-kernel/src/scripts.rs`; all numbers are little endian.

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

## Dialog einer Figur

`dialog::dialog_of(store, header, file, kind, index)` (`crates/g4-kernel/src/dialog.rs`) sagt, welche
Nachrichten ein Objekt oder Schild (`kind`: `object`, `bg`) der Eventdatei `file` zeigt, gelesen auf
Header `header`. Der Weg:

1. **Script-Nummer** des Events: `0` und `65535` sind `none` (die Figur sagt nichts; 65535 erreicht
   kein Bereich der Tabelle gemeinsamer Scripts, `resolve_script` findet nichts, Platin
   Zweiblattdorf Objekt 7; UI-Test 3d.5, U5), `3000` bis `4999` und `5000`
   bis `6999` sind `trainer` (Trainer `n − 3000 + 1` bzw. `n − 5000 + 1` als Doppelkampf-Partner, seine
   Texte stehen im Bereich Pokémon), ab `2000` sonst `common` (ein gemeinsames Script, in Bauen zu
   lesen). Diese drei haben keine Nachrichten.
2. **Slot:** unter `2000` ist es Script `#n` der Scriptdatei des Headers (`local` mit Datei, Slot und
   Zeile des Kopfes wie `resolve_script`). Ohne übernommene Scripts, ohne rotom-Quelle (binär, Level-
   Script) oder ohne den Slot in der Datei ist der Dialog `unreadable` mit dem Grund.
3. **Nachrichten:** der Rumpf des Scripts wird gelaufen. Jedes nummerierte Argument eines
   Nachrichtenbefehls (`script.message_params` in `check_limits.yaml`) ist eine Nachricht des
   Textarchivs des Headers, mit Befehl, Zeile und Spalte der Zahl. Steht sie in einem `if`, trägt
   sie die Bedingung wie geschrieben (`else` im Else-Zweig, `else if` seine eigene). Dekompiliertes
   Vanilla verzweigt aber nicht mit `if`, sondern mit bedingten Sprungbefehlen (Platin: 4 283
   Nachrichten, keine in einem `if`; UI-Test 3d.5, U4). Eine Nachricht in einem Label, das ein
   bedingter Sprung erreicht, trägt darum dessen Bedingung als `branch`: der Befehl mit seinen
   Argumenten ohne das Sprungziel, wie geschrieben (`GoToIfSet FLAG_X`, `CallIf EQUAL`). Ist das
   einzige übrige Argument ein Vergleich (`LESS`, `EQUAL`, `GREATER`, `LESS_EQUAL`, `GREATER_EQUAL`,
   `DIFFERENT` oder seine Zahl 0 bis 5, `GoToIf`/`CallIf`/`JumpIf`), steht der Befehl davor, dessen
   Ergebnis verglichen wird, mit Semikolon vorn: `CheckFlag FLAG_X; GoToIf EQUAL`,
   `CompareVarValue VAR_X, 5; CallIf 1` (der Befehl direkt vor dem Sprung oder vor einer Reihe von
   Sprüngen, kein Nachrichtenbefehl). Ein Label, das ein unbedingter Sprung (`GoTo`, `Call`, das
   Sprungwort) erreicht, und Code, in den ein Label weiterläuft, behalten die Bedingung, unter der
   sie erreicht wurden; im Label gilt ein eigenes `if` vor. Der Weg ohne Sprung (nach einem
   bedingten Sprung, der nicht springt) bleibt ohne Bedingung. Der Dialog-Editor zeigt das als
   „· wenn CheckFlag FLAG_X; GoToIf EQUAL“.
4. **Sprünge:** rotoms Sprungwort (`GoTo`/`Jump`, auch klein) und die Befehle aus
   `script.jump_params` (Pt `GoTo`, `Call`, `GoToIf`, `CallIf`, `GoToIfSet`, `GoToIfUnset`; HGSS
   `Jump`, `Call`, `JumpIf`, `CallIf`, `GoToIfSet`, `GoToIfUnset`; über rotoms Datenbank aufgelöst,
   Legacy- und kanonischer Name gelten gleich) springen zu dem Script oder Label derselben Datei,
   das ein Bezeichner-Argument nennt (`Call hilfe`, `GoToIf EQUAL, label_00A3`). Ein Label ist für
   rotom eine Funktion ohne Nummer (`hilfe:`). Jedes Ziel wird **einmal** gelaufen (Schleifen
   brechen ab), und zwar nach dem Rumpf, der springt: erst die eigenen Nachrichten des Scripts, dann
   die der Sprungziele in der Reihenfolge der Sprünge. Vanilla springt mit `GoToIf … label` weg,
   bevor der Standardtext kommt; der steht so vorn. Endet ein Rumpf nicht mit `End`, `Return`, dem
   Sprungwort oder einem `if … else`, das auf beiden Seiten so endet, läuft der Code in die
   nächste Funktion der Datei weiter; sie kommt vor den Sprungzielen dran. rotoms Decompiler
   schreibt so ein Label, das der Code auch von oben erreicht, mitten ins Script (`label_XXXX:`),
   und der Parser macht daraus eine eigene Funktion. `followed` nennt die gelaufenen Labels. Nicht
   verfolgt: gemeinsame Scripts (`CallCommonScript`), Menü-Ziele und Befehle außerhalb der Liste
   (etwa `GoToIfEq`).
5. **Geteilt:** `shared_with` einer Nachricht zählt die **anderen** Objekte, deren Dialog dieselbe
   (Archiv, Index) zeigt: alle Objekte der Eventdateien aller Header, die das Archiv nutzen (jede
   Eventdatei einmal, mit der Scriptdatei des ersten solchen Headers), jedes mit seinem Script wie
   oben. Schilder zählen nicht mit. Gleicher Text an einem anderen Index ist eine andere Nachricht
   (rotom legt ein Literal mit schon vorhandenem Text auf dessen Index, Vanilla hat Dubletten).
6. **Abspaltbar:** `split_refusal` ist der Satz, mit dem `split_message` die Nachricht für diese
   Figur ablehnen würde (Reihenfolge und Regeln dieselben, `SplitScope::check` in `dialog.rs`,
   Tabelle in [text_archive.md](text_archive.md) „Abspalten“), `null`, wenn sie sich abspalten
   lässt; bei einem Schild `NOT_AN_OBJECT` „Nur eine Figur bekommt eine eigene Kopie; ein Schild
   teilt seinen Text, wo er steht.“ Gefunden im UI-Test von 3d.5 (U2): ohne die Angabe bot der
   Dialog-Editor das Abspalten an fast jeder geteilten Nachricht an, und der Kern lehnte ab (Platin
   10 der ersten 11 Kandidaten, HeartGold 306 von 307, meist `SHARED_SCRIPT`). Die Prüfung läuft
   nur, wenn eine Nachricht des Dialogs geteilt ist.

Gemessen auf Platin DE: 3 555 Objekte, keines `unreadable`, 125 Figuren zeigen mindestens eine
geteilte Nachricht (329 Paare aus Archiv und Index); HeartGold DE: 2 667 Objekte, 140 Figuren, 240
Paare. Gutachten A zählte ohne Sprünge 66 bzw. 100 Nachrichten, wohl weil eine Nachricht in einem
Label dort keinem Script gehört (`checks::scripts::walk` gibt ihr keinen Slot). Der Startort Zweiblattdorf (Header 411) hat keine geteilte Nachricht, Neuborkia
(Header 60) zwei.
