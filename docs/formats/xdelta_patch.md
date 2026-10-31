# xdelta-Patch `build/<Name>.xdelta`

Eine Tatsache über Gen4 Studio selbst, keine Spielformat-Quelle: Der Export „Patch erstellen…“
(Technik › Veröffentlichen, `g4k patch create`, MCP `create_patch`) schreibt einen VCDIFF-Patch, der
die Basis-ROM des Projekts in die gebaute ROM überführt, und daneben eine Begleitdatei.
Implementierung: `crates/g4-kernel/src/patch.rs` (`patch_name`, `default_targets`, `meta`,
`app_header`, `notes`, `notes_path`, `create_patch`, `build_and_create`), Kodierer
`crates/g4-xdelta` (xdelta3 3.0.12, Apache-2.0, siehe dessen `NOTICE`).

## Ziel und Name

`<Name>` ist `<Projekt>-<Spielcode>`, zum Beispiel `Pokemon - Platin-Edition (Germany)-CPUD`
(`patch_name`):

- `<Projekt>` ist der Name des Projektordners ohne die Endung `_DSPRE_contents`;
- die Zeichen `/ \ : * ? " < > |` und Steuerzeichen werden `_`;
- Leerzeichen und Punkte an beiden Enden fallen weg;
- bleibt nichts übrig, heißt das Projekt `Projekt`;
- `<Spielcode>` ist der aus `header.yaml`.

Beispiele: `Mein/Hack:Neu_DSPRE_contents` → `Mein_Hack_Neu-CPUE`, ` .Rom. ` → `Rom-CPUE`,
`...` → `Projekt-CPUE`.

Der Export (`build_and_create`) baut nach `build/<Name>.nds` im Projektordner und lässt die ROM dort
liegen; der Patch geht nach `build/<Name>.xdelta`, die Begleitdatei nach `build/<Name>.xdelta.txt`
(`default_targets`, `notes_path`). Ohne Basismarke lehnt `build_and_create` mit „Diesem Projekt ist
keine Basis-ROM zugeordnet.“ ab, mit einer Basis-ROM anderer SHA-1 mit dem Satz von
`base::other_rom`; in beiden Fällen wird nichts gebaut.

## Format

VCDIFF (RFC 3284) in der Form von xdelta3:

| Eigenschaft | Wert |
|---|---|
| Zweitkompression | keine (Bit `0x01` des Header-Indikators nicht gesetzt); Rom Patcher JS und jedes xdelta3 3.x wenden den Patch an, auch ohne LZMA |
| Prüfsumme | Adler32 je Fenster (`XD3_ADLER32`), geprüft beim Anwenden |
| Quelle | die ganze Basis-ROM als ein Block; Fenster des Ziels höchstens `XD3_DEFAULT_WINSIZE` (8 MiB) |
| App-Header | Bit `0x04`, Inhalt siehe unten |

### App-Header

Text, UTF-8, ohne Zeilenschaltung, in der xdelta-Form `ziel/zielkompression/quelle/quellkompression/`
mit leeren Kompressionsfeldern:

```text
<Name>.nds//<Basislabel> [<SHA-1 der Basis-ROM, 40 kleine Hex-Ziffern>].nds/
```

Zum Beispiel
`Mein Hack-CPUD.nds//Platin DE (Testprojekt) [d40f19ac24d67c721a4ab25e1c013154443d75b4].nds/`.
`<Basislabel>` ist `base::base_label` der Marke (Name aus `base_roms.yaml`, sonst
`<Titel> (<Spielcode>)`), bereinigt wie `<Projekt>` (leer wird `Basis-ROM`); so steht kein `/` darin,
das die Felder trennt. `xdelta3 printhdr <Patch>` zeigt den Header, Rom Patcher JS überspringt ihn.
VCDIFF selbst hat kein Feld für eine Prüfsumme der Quelle; der Header und die Begleitdatei nennen sie.

## Begleitdatei `<Name>.xdelta.txt`

Deutsch, UTF-8, Zeilen mit `\n`, fester Text (`notes`):

```text
Gen4 Studio: Patch für {base_label}
Spiel: {game_code}
Basis-ROM SHA-1: {base_sha1}
Ziel-ROM SHA-1: {target_sha1}
Studio-Version: {studio_version}

Anwenden: mit xdelta3 („xdelta3 -d -s <Basis-ROM> {name}.xdelta {name}.nds“) oder mit Rom Patcher JS, auf genau die Basis-ROM mit dieser SHA-1.
```

`base_label` steht hier unbereinigt, `studio_version` ist die Version des Kernels, `target_sha1` die
SHA-1 der gebauten ROM `build/<Name>.nds`.

Mit einer Veröffentlichung (`notes_with`, `create_patch_with_notes`) kommen `Version:` und `Datum:`
nach `Studio-Version:` und, wenn der Entwickler Notizen eingegeben hat, ein letzter Absatz
`Änderungen:` dazu:

```text
Gen4 Studio: Patch für {base_label}
Spiel: {game_code}
Basis-ROM SHA-1: {base_sha1}
Ziel-ROM SHA-1: {target_sha1}
Studio-Version: {studio_version}
Version: {version}
Datum: {date}

Anwenden: mit xdelta3 („xdelta3 -d -s <Basis-ROM> {name}.xdelta {name}.nds“) oder mit Rom Patcher JS, auf genau die Basis-ROM mit dieser SHA-1.

Änderungen:
{notes}
```

Ohne Veröffentlichung fehlen `Version:`, `Datum:` und `Änderungen:`; die ersten fünf Zeilen und der
Anwenden-Absatz sind die von 4.1a. `Datum` ist `YYYY-MM-DDTHH:MM:SSZ` (UTC, `now_rfc3339`). Der
Block `Änderungen:` steht nur bei nicht leeren Notizen (nur Leerraum zählt als leer); die Notizen
stehen wie eingegeben, Leerraum am Ende ist abgeschnitten, die Datei endet mit genau einem `\n`.

## Veröffentlichungen `.g4/releases.json`

Die Liste der Patches, die das Projekt veröffentlicht hat, liegt versioniert im Projekt (die
Erlaubnisliste der `.gitignore` nennt sie, siehe [project_git.md](project_git.md#g4)). UTF-8, zwei
Leerzeichen Einrückung, Felder in der Reihenfolge der Struktur, am Ende ein `\n`; geschrieben über
Temp-Datei und Umbenennen:

```json
{
  "version": 1,
  "releases": [
    {
      "version": "0.1.0",
      "date": "2026-10-31T10:00:00Z",
      "file": "build/Mein Hack-v0.1.0.xdelta",
      "notes": "Neue Karte",
      "baseSha1": "d40f19ac24d67c721a4ab25e1c013154443d75b4",
      "size": 1234
    }
  ]
}
```

- `releases` steht in der Reihenfolge des Veröffentlichens, die neueste zuletzt
  (`append_release` hängt an).
- `file` ist der Patch relativ zum Projektordner mit `/`, wenn er darunter liegt, sonst der absolute
  Pfad (`release_file_entry`). `size` ist die Größe des Patches in Bytes, `baseSha1` die der
  Basis-ROM, `date` wie in der Begleitdatei, `notes` der Text der Notizen.
- Die Datei darf fehlen: das ist die leere Liste. Ist sie nicht lesbar (kein JSON, fehlende oder
  unbekannte Felder, `version` unbekannt) oder hat sie eine neuere `version` als `1`, ist das
  Ergebnis `KernelError::Refused` mit „Die Liste der Veröffentlichungen ist nicht lesbar: <Pfad>:
  <Grund>“ bzw. „… nicht lesbar (Fassung 2 ist neuer als diese App)“ (`RELEASES_BROKEN`); die Datei
  bleibt Byte für Byte, auch `append_release` überschreibt sie nicht.
- `next_version`: für die leere Liste `0.1.0`; sonst die Version der letzten Veröffentlichung mit
  `major.minor.patch`, Patchzahl plus eins, ein Anhang (`-beta`, `+build`) entfällt (`2.0.0-beta` →
  `2.0.1`, `1.2.9` → `1.2.10`); eine Version, die nicht aus drei Zahlen besteht (`final`), bleibt
  stehen, der Entwickler tippt die nächste.
- `check_version`: `Err(BAD_VERSION)` für eine leere Version, eine mit mehr als 32 Zeichen oder mit
  Leerraum, Steuerzeichen oder einem von `/ \ : * ? " < > |`; `Err(VERSION_TAKEN)` („Diese Version
  gibt es schon.“) für eine, die die Liste schon hat (genau gleich); sonst `Ok`.
- Dateiname des Patches: `<Projekt>-v<Version>.xdelta` (`release_file_name`), `<Projekt>` ist der
  Ordnername ohne `_DSPRE_contents`, bereinigt wie im Patchnamen (`project_stem`, ohne Spielcode).

## Selbstprüfung und Schreiben

`create_patch(base, gebaute_rom, ziel, meta)`:

1. liest die gebaute ROM und bildet ihre SHA-1;
2. kodiert den Patch im Speicher (`encode_ms`);
3. dekodiert ihn wieder auf der Basis-ROM und vergleicht die SHA-1 des Ergebnisses mit der der
   gebauten ROM (`verify_ms`). Ergibt der Patch etwas anderes oder lässt er sich nicht dekodieren,
   ist das Ergebnis `KernelError::Refused` mit „Selbstprüfung fehlgeschlagen: der Patch ergibt auf
   der Basis-ROM nicht die gebaute ROM; nichts geschrieben.“ (`SELF_CHECK_FAILED`, ohne Vorsatz so
   in App, MCP und `g4k` angezeigt; ebenso „Patch nicht erstellt: …“, wenn das Kodieren scheitert),
   und es wird nichts geschrieben: ein alter Patch und seine Begleitdatei bleiben Byte für Byte;
4. schreibt Patch und Begleitdatei je in eine Temp-Datei `.<Datei>.tmp` im Zielordner (mit `fsync`)
   und benennt dann erst den Patch, danach die Begleitdatei um. Scheitert ein Schritt, werden die
   Temp-Dateien entfernt; nur wenn das zweite Umbenennen scheitert, liegt ein neuer Patch neben der
   alten Begleitdatei.

Die Selbstprüfung ist die eigentliche Garantie: Ein abgeschnittener Patch mit mehreren Fenstern
dekodiert ohne Fehler zu einer kürzeren ROM, erst der SHA-1-Vergleich findet ihn.

## Gegenprobe und Stolperfalle

Mit dem xdelta3-CLI (getestet mit 3.2.2):

```sh
xdelta3 -d -s <Basis-ROM> <Name>.xdelta <Name>.nds
```

Die SHA-1 von `<Name>.nds` ist dann die `Ziel-ROM SHA-1` der Begleitdatei. Der gated Test
`patch_after_map_and_after_map_script_text_is_small_and_applies` (`crates/g4-kernel/tests/base.rs`)
macht diese Gegenprobe, wenn `xdelta3` im `PATH` liegt.

**Stolperfalle:** `xdelta3 -d <Patch>` ohne `-s` nimmt den Quellnamen aus dem App-Header, sucht also
eine Datei `<Basislabel> [<SHA-1>].nds` im aktuellen Ordner und bricht ab, wenn es sie nicht gibt.
Das ist harmlos, aber die Quelle muss immer mit `-s` angegeben werden.

## Größen

Gemessen mit dem gated Test (xdelta3 aus `g4-xdelta` mit `opt_level(2)`). „Eine Stelle“ ist der
erste Karten-Header (in Header-Reihenfolge), dessen Textarchiv höchstens 300 Nachrichten hat: seine
Script-Datei wird um 2 Byte länger, Nachricht 0 seines Textarchivs um „ Studio“; die Karte 3 bekommt
vorher ein geändertes Byte. „Archiv 0“ verlängert danach zusätzlich Nachricht 0 des Textarchivs 0.

| Projekt | eine Stelle (Header, Script, Text) | Karte | Karte, Script, Text | dazu Archiv 0 | Erzeugen | Selbstprüfung |
|---|---|---|---|---|---|---|
| Platin DE (`CPUD`) | 0, 401, 23 (101 Nachrichten) | 551 B | 8 227 B | 19 653 B | 103 bis 200 ms | 138 bis 139 ms |
| HeartGold DE (`IPKD`) | 2, 957, 738 (220 Nachrichten) | 552 B | 3 043 B | 7 794 B | 152 bis 201 ms | 138 bis 141 ms |
| HeartGold US (`IPKE`) | 2, 957, 738 (220 Nachrichten) | 552 B | 3 026 B | 7 737 B | 152 bis 201 ms | 138 bis 145 ms |

Eine Änderung, die einen NARC verlängert, verschiebt alle Member dahinter; jede Zeile seiner
Dateitabelle ändert sich in wenigen Bytes, und jede kostet im Patch etwa 4 bis 8 Byte (eine
Anweisung „hinzufügen“ und eine „kopieren“). Wächst eine Nachricht, ändert sich zudem jede
verschlüsselte Zeile der Offset-Tabelle ihres Textarchivs. Archiv 0 von Platin DE hat 1 404
Nachrichten und liegt vorn im Text-NARC, seine Änderung allein kostet rund 16 KB. Das xdelta3-CLI
kommt auf dieselben Größen (Platin DE, Karte, Script-Datei 0 und Archiv 0: 20 432 B bei uns, 20 538 B
mit `-S none`, 20 451 B mit `-9 -S none`, 15 526 B mit LZMA).
