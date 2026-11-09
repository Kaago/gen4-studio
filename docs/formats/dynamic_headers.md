# Dynamic Headers (ARM9-Patch)

**In English:** Documents the "Dynamic headers" patch to `arm9.bin`. Without it the map header table
(24 bytes per header) is fixed inside the ARM9 binary; with it, a small routine loads each header as
a member of an NARC, so that new headers are simply new members. The document lists the patch data
and file offsets and the NARC that holds the headers, per game family (Platinum and
HeartGold/SoulSilver); these are facts about the game binaries, written down in our own words. The
data is in `crates/g4-kernel/metadata/dynamic_headers.yaml` and the code in
`crates/g4-kernel/src/dynamic_headers.rs`. All offsets refer to the decompressed `arm9/arm9.bin`,
and values are little endian.

Quelle: DSPRE `DS_Map/Resources/ROMToolboxDB/ToolboxDB.cs` (`DynamicHeadersPatchData`,
Z. 272-390) und `DS_Map/PatchToolboxDialog.cs` (`dynamicHeadersButton_Click`,
Z. 1210-1345). Es sind Tatsachen über die Spielbinaries (Offsets, Maschinencode), keine
Übernahme von DSPRE-Programmcode. Die Daten stehen in
`crates/g4-kernel/metadata/dynamic_headers.yaml`, die Implementierung in
`crates/g4-kernel/src/dynamic_headers.rs`. Alle Offsets sind Dateioffsets im
dekomprimierten `arm9/arm9.bin`; Werte sind Little Endian.

## Wirkung

Ohne den Patch liegt die Header-Tabelle (24 Byte je Header) fest in `arm9.bin` (Offset je
Spielcode in `metadata/header_tables.yaml`). Mit dem Patch lädt eine kleine Routine den
Header `id` als Member `id` aus einem NARC; neue Header sind dann neue Member. Der NARC und
sein entpackter Ordner:

| Familie | NARC (gepackt) | Ordner unter `unpacked/` |
|---|---|---|
| Platin | `files/debug/cb_edit/d_test.narc` (DSPRE NARC 0x94) | `dynamicHeaders` |
| HeartGold / SoulSilver | `files/a/0/5/0` (DSPRE NARC 0x32) | `dynamicHeaders` |

Der Ordner enthält eine Datei `NNNN` (vier Ziffern, ab `0000`) je Header mit genau den 24
Bytes des Headers (Layout je Familie siehe uxie `map_header`). In einem ungepatchten
DSPRE-Projekt existiert der Ordner schon und hält die Vanilla-Member des NARC; `apply`
ersetzt ihn, nach einem Schnappschuss (siehe unten). Die alte Tabelle in `arm9.bin` bleibt stehen und wird nicht mehr gelesen.
Diamant/Perl hat den Patch nicht.

## Die Routine (20 Bytes bei `init.offset`)

```
00 B5          push {lr}
01 1C          mov  r1, r0            ; Header-ID
94 20 / 32 20  mov  r0, #0x94 (Pt) / #0x32 (HGSS)   ; NARC-Nummer
00 22          mov  r2, #0
CC F7 xx xx    bl   Load_Memory       ; je Version
03 1C          mov  r3, r0
xx F7 xx xx    bl   Free_Memory       ; je Version
00 BD          pop  {pc}
```

Die Bytefolge je Spielcode (nur die beiden `bl` unterscheiden sich) und `init.offset` stehen
in `dynamic_headers.yaml`. Die Routine liest den Header aus dem geladenen Member und gibt ihn
gleich wieder frei; die Aufrufer lesen danach über `r3` (siehe Referenzstellen). Das geht nur,
weil die Free-Funktion mit `push {r3-r7,lr}` beginnt und `r3` erhält: Ein `bl` auf eine andere
Stelle lässt das Spiel nach dem Intro hängen (schwarzer Bildschirm).

Ein Thumb-`bl` sind zwei Halbwörter `11110 hi` und `11111 lo`; Ziel = Adresse des `bl` + 4 +
Vorzeichen-erweitertes `(hi << 12) | (lo << 1)` (22 Bit). Dateioffset und RAM-Adresse
unterscheiden sich in `arm9.bin` um die Konstante 0x0200_0000, also gilt die Rechnung für
beide.

### Abweichung von DSPRE: Platin DE/FR/IT/ES

DSPRE (`ToolboxDB.cs`) schreibt für `CPUS`, `CPUI`, `CPUF` und `CPUD` als zweiten `bl` die
Bytes `CC F7 74 FC`. Bei 0x3A0D6 ergibt das das Ziel 0x69C2: mitten in einer Funktion (nach
deren `push`/`sub sp` bei 0x69BE), kein einziger Aufruf im Spiel geht dorthin. Mit diesem Patch
hängt Platin DE nach dem Intro. DSPREs eigener Kommentar nennt für diese Sprachen
`Free_Memory` = 0x02018234; dorthin führt `DE F7 AD F8`, und so steht es in
`dynamic_headers.yaml`. Der erste `bl` (`CC F7 00 FD`, Ziel 0x6AD4) stimmt.

Nachgeprüft an Platin DE (`CPUD`, nur gelesen): 0x6AD4 ist die ganze 24-Byte-Funktion, die
ein NARC-Member lädt (15 Aufrufer), byte-gleich mit der Ladefunktion von HeartGold DE/US bei
0x7524. 0x18234 hat 642 Aufrufer und beginnt mit denselben 32 Bytes wie die Free-Funktion von
HeartGold DE/US bei 0x1AB0C (662 Aufrufer). Auf einer Testkopie bootet die gebaute ROM mit
`DE F7 AD F8` in melonDS, und ein neuer Header (0593) ist über einen Warp begehbar.
`CPUS`, `CPUI` und `CPUF` haben dasselbe Layout wie `CPUD` (gleiches `init.offset` und
gleiche Referenzstellen bei DSPRE); ihre `arm9.bin` lag nicht vor. Für sie und für jedes
andere nicht verifizierte Spiel entscheidet die Prüfung der Aufrufziele (unten) beim Anwenden.

### Verifizierte Spiele und Signaturen der Aufrufziele

`verified` in der YAML-Datei nennt die Spiele, deren Routine an ihrer eigenen `arm9.bin`
nachgeprüft ist: `CPUD`, `IPKD`, `IPKE`. Die Aufrufziele dort:

| Spiel | Load (`bl` bei init + 8) | Free (`bl` bei init + 14) |
|---|---|---|
| CPUD | 0x6AD4 | 0x18234 |
| IPKD, IPKE | 0x7524 | 0x1AB0C |

`targets` hält die Signaturen, die in allen drei Spielen gleich sind: für Load die ganze
Funktion (`10 B5 82 B0 00 23 00 93 84 00 03 48 01 93 00 59 FF F7 6A FF 02 B0 10 BD`), für
Free die ersten 16 Bytes (`F8 B5 06 1C 30 1F 00 68 00 06 04 0E 18 48 81 8A`). Referenz-Offsets sind für Platin-US (`CPUE`, 0x3A024) und
HeartGold-US (`IPKE`, 0x3B268) notiert; für jedes andere Spiel gilt
`diff = init.offset(Spiel) - init.offset(Basis)` (zum Beispiel +0xA4 für CPUD, -0x8 für
IPKS, -0x448 für IPKJ/IPGJ, -0x444 für CPUJ, 0 für IPKD/IPKE/IPGS). DSPRE kennt für IPGS
keinen eigenen Zweig und nimmt die IPKE-Werte; das ist dort nicht belegt, die Vorprüfung
fängt Abweichungen ab (Prüfungen 3 bis 6).

## Referenzstellen

An 20 (Platin) bzw. 26 (HGSS) Stellen steht `mov r1,#0x18; mul r1,r0` (`18 21 41 43`,
`id * 24`) und kurz danach ein Literal `Tabellenbasis + n` (RAM-Adresse
`0x0200_0000 + header_table_offset + n`, `n` in 0..24, das Feld im Header). Der Patch ersetzt:

- die vier Bytes bei `item1 + diff` durch `19 00 C0 46` (`lsl r1,r3,#0; nop`);
- das `u32` bei `item2 + diff` durch `wert - header_table_offset - 0x0200_0000`, also `n`.

Die Paare `(item1, item2)` stehen je Familie in der YAML-Datei.

## Sonderfall HGSS

Bei HGSS in jeder Sprache außer Japanisch und Spanisch steht an einer Stelle eine
Instruktion zwischen `mov r1,#0x18` und `mul r1,r0`. Dort werden (alle plus `diff`) bei
`0x3B522` die Bytes `18 21` durch `19 00`, bei `0x3B526` die Bytes `41 43` durch `C0 46` und
bei `0x3B53C` das Literal durch `00 00 00 00` ersetzt.

## Signatur und Vorprüfung

Erkennung: das `u16` bei `init.offset` ist `0xB500` (`push {lr}`). `apply` prüft vor jedem
Schreiben, und jede fehlgeschlagene Prüfung nennt den Offset und schreibt nichts:

1. das Spiel hat den Patch (nicht Diamant/Perl), `init.offset` liegt in `arm9.bin`;
2. die Signatur steht noch nicht da;
3. bei jedem `item1 + diff` stehen `18 21 41 43`;
4. jedes Literal bei `item2 + diff` ist eine RAM-Adresse in `[Tabellenbasis, Tabellenbasis + 24)`;
5. im Sonderfall: `18 21` bei `mov + diff`, `41 43` bei `mul + diff`, Literal wie in 4;
6. beide `bl` der Routine sind `bl`, und an ihrem Ziel in der `arm9.bin` des Projekts beginnt
   die Signatur aus `targets` (Load bzw. Free). Ein falscher oder ungeprüfter Offset wird so
   abgelehnt, statt eine ROM zu erzeugen, die hängt.

Danach: Schnappschuss des alten Ordners (unten), Ordner `unpacked/dynamicHeaders` über einen
temporären Nachbarordner ersetzen, `arm9.bin.backup` anlegen (nur wenn es fehlt), `arm9.bin`
atomar schreiben.

## Schnappschuss

Existiert `unpacked/dynamicHeaders` und ist nicht leer, kopiert `snapshot_folder` ihn vor dem
Ersetzen (erst nachdem alle Prüfungen bestanden sind) samt Unterordnern nach
`<Projekt>/.g4/backup/dynamicHeaders-<YYYYMMDD-HHMMSS>/` (UTC, Sekunden). Gibt es den Ordner
schon, bekommt der Name `-2`, `-3` … angehängt; in einen vorhandenen Ordner wird nie
geschrieben. Eine abgebrochene Kopie wird entfernt, und der Patch schreibt dann nichts. Platin
hält dort die 12 Vanilla-Member des NARC. Der Pfad steht im Ergebnis von `apply_header_patch`
(`snapshot`, `null` ohne Schnappschuss). Wiederherstellen ist Handarbeit: den Ordner
zurückkopieren und `arm9.bin.backup` als `arm9.bin` einsetzen.

## Ungespeicherte Tabelle

`Store::apply_dynamic_headers` lehnt eine Header-Tabelle mit ungespeicherten Änderungen nicht
ab. Gepatcht wird mit dem gespeicherten Stand (dem, den der Store zuletzt gelesen oder
geschrieben hat); weicht `arm9.bin` davon ab, weil ein anderes Werkzeug die Tabelle geändert
hat, ist das `stale` und nichts wird geschrieben. Die neuen Dateien halten danach genau diese
Bytes, und die Bytes des Assets hängen nicht vom Ort ab (Ausschnitt von `arm9.bin` wie
Verkettung der Dateien). Darum bleiben Wert im Speicher, gespeicherter Stand und Verlauf
(Undo/Redo) des Assets erhalten: die Änderung bleibt ungespeichert, ⌘Z geht über den Patch
hinweg zurück, und das nächste Speichern schreibt die geänderten Dateien `NNNN`, nicht mehr
`arm9.bin`. Nur eine gesperrte Tabelle (vom Codec nicht byte-gleich zurückschreibbar, also nie
bearbeitet) wird nach dem Patch aus den Dateien neu gelesen.

## Reparatur eines fehlerhaft angewendeten Patches

Wer den Patch mit DSPRE oder mit Studio vor dieser Korrektur auf Platin DE/FR/IT/ES angewendet
hat, hat die Signatur bei `init.offset`, aber einen `bl` ins Leere. `needs_repair` erkennt das:
Signatur gesetzt, mindestens ein Aufrufziel besteht Prüfung 6 nicht, die Routine weicht von
der des Spiels nur innerhalb der beiden `bl` ab, und die Routine des Spiels besteht Prüfung 6.
`repair` schreibt dann die Routine des Spiels (es ändern sich nur Bytes in den `bl`, bei
DSPREs Platin-Bytes genau die 4 bei `init.offset + 14`), legt `arm9.bin.backup` an, wenn es
fehlt, und schreibt `arm9.bin` atomar; die Header-Dateien bleiben unberührt. In jedem anderen
Fall lehnt es ab und schreibt nichts. Studio meldet einen reparierbaren Patch beim Öffnen des
Projekts (Meldung mit „Patch reparieren“) und im Schritt „Header“ der Assistenten, die dort
bis zur Reparatur keinen neuen Header anlegen.

## Lesen, Speichern und Packen

Ist die Signatur gesetzt, liegt die Header-Tabelle (Asset `HeaderTable`, Index 0) in den
Dateien `0000..count-1` von `unpacked/dynamicHeaders`, jede genau 24 Bytes; `count` ist die
Zahl der 16-Byte-Namen in `files/fielddata/maptable/mapname.bin`. Der Wert des Assets ist die
Verkettung der Dateien. Fehlt eine Datei, meldet das Lesen ihren Pfad; eine Datei mit anderer
Länge als 24 ist ein Fehler. Speichern schreibt nur die Dateien, deren 24 Bytes sich gegenüber
dem zuletzt gelesenen Stand geändert haben (je Datei atomar), `arm9.bin` bleibt unberührt.
`build` packt den Ordner in das NARC (`archives.yaml`, `dynamic_headers`), sobald er existiert;
in einem ungepatchten DSPRE-Projekt enthält er die Vanilla-Member des NARC und packt
byte-identisch zurück.

## Verifikation

Nachgeprüft an den Vanilla-Projekten: Platin DE (`CPUD`, diff 0xA4): `18 21 41 43` bei
0x3A0E2, Literal 0x020E6074 bei 0x3A0EC. HeartGold DE und US (`IPKD`, `IPKE`, diff 0):
`18 21 05 4A 41 43` bei 0x3B522, das Literal `Tabellenbasis` bei 0x3B53C. Die gated Tests
(`crates/g4-kernel/tests/dynamic_headers.rs`) wenden den Patch auf Kopien von CPUD, IPKD und
IPKE an, prüfen jedes geschriebene und jedes unberührte Byte, dekodieren beide geschriebenen
`bl` und vergleichen sie mit den Zielen der Tabelle oben. Ein weiterer Test setzt auf der
CPUD-Kopie DSPREs Bytes `CC F7 74 FC` ein, erwartet `needs_repair` und prüft die Reparatur.
