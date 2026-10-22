# Dynamic Headers (ARM9-Patch)

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
ersetzt ihn. Die alte Tabelle in `arm9.bin` bleibt stehen und wird nicht mehr gelesen.
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
in `dynamic_headers.yaml`. Referenz-Offsets sind für Platin-US (`CPUE`, 0x3A024) und
HeartGold-US (`IPKE`, 0x3B268) notiert; für jedes andere Spiel gilt
`diff = init.offset(Spiel) - init.offset(Basis)` (zum Beispiel +0xA4 für CPUD, -0x8 für
IPKS, -0x448 für IPKJ/IPGJ, -0x444 für CPUJ, 0 für IPKD/IPKE/IPGS). DSPRE kennt für IPGS
keinen eigenen Zweig und nimmt die IPKE-Werte; das ist dort nicht belegt, die Vorprüfung
fängt Abweichungen ab.

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
5. im Sonderfall: `18 21` bei `mov + diff`, `41 43` bei `mul + diff`, Literal wie in 4.

Danach: Ordner `unpacked/dynamicHeaders` über einen temporären Nachbarordner ersetzen,
`arm9.bin.backup` anlegen (nur wenn es fehlt), `arm9.bin` atomar schreiben.

## Verifikation

Nachgeprüft an den Vanilla-Projekten: Platin DE (`CPUD`, diff 0xA4): `18 21 41 43` bei
0x3A0E2, Literal 0x020E6074 bei 0x3A0EC. HeartGold DE und US (`IPKD`, `IPKE`, diff 0):
`18 21 05 4A 41 43` bei 0x3B522, das Literal `Tabellenbasis` bei 0x3B53C. Die gated Tests
(`crates/g4-kernel/tests/dynamic_headers.rs`) wenden den Patch auf Kopien von CPUD, IPKD und
IPKE an und prüfen jedes geschriebene und jedes unberührte Byte.
