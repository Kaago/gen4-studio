# Basismarke `.g4/base.json`

Eine Tatsache über Gen4 Studio selbst, keine Spielformat-Quelle: Wenn der Kernel ein Projekt aus
einer `.nds`-Datei anlegt (`g4_kernel::start::create_project`, `g4k create`), schreibt er in den
Werkzeugordner des Projekts die Datei `.g4/base.json`. Ein Projekt, das ein anderes Werkzeug
(zum Beispiel DSPRE) angelegt hat, hat keine. Implementierung: `crates/g4-kernel/src/start.rs`
(`BaseMark`, `read_base_mark`, `restore_header_crcs`). Eine fehlende oder unlesbare Datei liest
`read_base_mark` als `None`; das ist kein Fehler.

## Felder

JSON, eingerückt, UTF-8, eine Zeilenschaltung am Ende.

| Feld | Typ | Inhalt |
|---|---|---|
| `version` | Zahl | Formatversion, heute `1` |
| `origin` | Text | `"extracted"`: aus einer ROM entpackt |
| `sha1` | Text | SHA-1 der ganzen ROM-Datei, 40 kleine Hex-Ziffern |
| `game_code` | Text | Spielcode aus dem ROM-Kopf (`0x0C`), zum Beispiel `CPUD` |
| `title` | Text | Titel aus dem ROM-Kopf (`0x00..0x0C`), ohne NUL am Ende |
| `revision` | Zahl | Versionsbyte (`0x1E`) |
| `size` | Zahl | Größe der ROM in Byte |
| `template` | Text | `"original"`, `"data_only"` oder `"empty_region"`; `data_only` ändert am Projekt nichts, nur die App startet im Bereich Pokémon; `empty_region` ist die Leere Region mit Startstadt ([templates.md](templates.md)) |
| `created` | Text | RFC 3339, UTC, Sekunden: `2026-10-25T14:50:00Z` |
| `studio_version` | Text | Version des Kernels, der das Projekt anlegte |
| `secure_area_crc` | Zahl | CRC-16 der Secure Area aus der ROM (`0x6C`, Little Endian) |
| `secure_area_sha1` | Text | SHA-1 der Secure Area (`0x4000..0x8000`) der ROM |

## Anlegen

`create_project(rom, dest, template, progress)` arbeitet in vier Phasen (`read`, `extract`,
`template`, `check`, je am Anfang gemeldet; `template` mit `empty_region` meldet zusätzlich
`done/total`):

1. **read**: `inspect_rom` (Kopf, SHA-1); Diamant/Perl und Nicht-Gen-4 werden mit `Refused` abgelehnt;
   das Ziel muss fehlen oder ein leerer Ordner sein und 2,5 Mal die ROM-Größe frei haben
   (`check_destination`).
2. **extract**: ds-rom entpackt in `<Eltern>/.<Name>.g4-new` (fehlende Eltern legt der Kernel an).
3. **template**: für `original` und `data_only` nichts; für `empty_region` werden die Scripts
   übernommen (Fortschritt über die Member), das Rezept angewendet und gespeichert
   ([templates.md](templates.md)).
4. **check**: das Projekt muss sich öffnen lassen und mindestens einen Karten-Header haben; dann
   wird `.g4/base.json` geschrieben und der Ordner nach `dest` umbenannt. Ein vorhandenes leeres
   `dest` wird dafür mit `remove_dir` entfernt (das verweigert alles Nichtleere).

Bei einem Fehler löscht der Kernel nur den Ordner `.<Name>.g4-new`, den er selbst angelegt hat, nie
das Ziel oder fremde Dateien. Existiert `.<Name>.g4-new` schon vor dem Start (Rest eines abgebrochenen
Laufs oder Datei eines anderen), bricht `create_project` mit `Refused` ab („staging folder … is in the
way; remove it or choose another name“) und lässt den Ordner unberührt.

## `base_roms.yaml`

`crates/g4-kernel/metadata/base_roms.yaml` listet ROMs, gegen die der Editor ausprobiert wurde
(`sha1`, `game_code`, `label`, `source`). `inspect_rom` gibt das `label` als `known_base` zurück,
wenn SHA-1 und Spielcode passen. Die Einträge heißen „Testprojekt“, nicht „sauberer Dump“: ein
Abgleich mit der No-Intro-Datenbank ist nicht gemacht. Die Tabelle ist eine Auskunft, keine
Freigabe; die Freigabe steht in `Support` (`inspect_rom`).

## Byte-gleicher Neubau und die CRC-Wiederherstellung

ds-rom kann die CRC-16 der Secure Area (Header `0x6C`) ohne den Blowfish-Schlüssel des ARM7-BIOS
nicht berechnen und schreibt beim Bauen `0000`; die Header-CRC bei `0x15E` hängt davon ab. Gemessen
an Platin DE und HeartGold DE: ein Entpacken und Bauen ohne weitere Änderung weicht genau an den
Bytes `0x6C..0x6E` und `0x15E..0x160` ab; die Secure Area selbst (`0x4000..0x8000`) ist gleich.

`build()` ruft darum nach dem Schreiben der ROM `restore_header_crcs(projekt, rom)`:

1. ohne Marke geschieht nichts (DSPRE-Ordner bleiben, wie sie waren);
2. trägt die ROM nicht den Spielcode der Marke (`0x0C..0x10`), geschieht ebenfalls nichts;
3. hat die gebaute Secure Area nicht den SHA-1 aus der Marke, bleibt `0000` stehen (die CRC gilt
   nur für diese Bytes);
4. sonst wird `secure_area_crc` nach `0x6C` geschrieben und die Header-CRC bei `0x15E` neu
   berechnet: CRC-16/MODBUS (Startwert `0xFFFF`, Polynom `0xA001` gespiegelt) über `0x000..0x15E`,
   Little Endian.

So gleicht die gebaute ROM eines unveränderten Projekts der Eingabe Bit für Bit (gated Test
`project_start_create_then_build_matches_the_rom`, SHA-1 gleich).
