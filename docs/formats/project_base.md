# Basismarke `.g4/base.json`

Eine Tatsache über Gen4 Studio selbst, keine Spielformat-Quelle: Die Datei `.g4/base.json` im
Werkzeugordner eines Projekts nennt seine Basis-ROM. Der Kernel schreibt sie, wenn er ein Projekt aus
einer `.nds`-Datei anlegt (`g4_kernel::start::create_project`, `g4k create`, `origin: "extracted"`),
und wenn einem DSPRE-Ordner eine Basis-ROM zugeordnet wird (`g4_kernel::start::assign_base`,
`origin: "dspre"`). Ein DSPRE-Ordner ohne Zuordnung hat keine. Implementierung:
`crates/g4-kernel/src/start.rs` (`BaseMark`, `mark_state`, `read_base_mark`, `check_base`,
`assign_base`, `suggest_base`, `restore_header_crcs`).

`mark_state` unterscheidet vier Fälle: `Missing` (keine Datei), `Unreadable(Grund)` (nicht lesbar,
kein JSON oder ein Feld fehlt), `Newer(Version)` (`version` größer als die eigene; erkannt, bevor die
übrigen Felder gelesen werden) und `Present(Marke)`. `read_base_mark` gibt nur bei `Present` die Marke
zurück, sonst `None`; das ist kein Fehler, der Build läuft ohne Marke weiter.

## Felder

JSON, eingerückt, UTF-8, eine Zeilenschaltung am Ende.

| Feld | Typ | Inhalt |
|---|---|---|
| `version` | Zahl | Formatversion: jede neue Marke trägt `2` (`BASE_MARK_VERSION`); `1` wird weiter gelesen |
| `origin` | Text | `"extracted"`: aus einer ROM entpackt; `"dspre"`: DSPRE-Ordner mit zugeordneter Basis-ROM |
| `sha1` | Text | SHA-1 der ganzen ROM-Datei, 40 kleine Hex-Ziffern |
| `game_code` | Text | Spielcode aus dem ROM-Kopf (`0x0C`), zum Beispiel `CPUD` |
| `title` | Text | Titel aus dem ROM-Kopf (`0x00..0x0C`), ohne NUL am Ende |
| `revision` | Zahl | Versionsbyte (`0x1E`) |
| `size` | Zahl | Größe der ROM in Byte |
| `template` | Text, optional | `"original"`, `"data_only"` oder `"empty_region"`; `data_only` ändert am Projekt nichts, nur die App startet im Bereich Pokémon; `empty_region` ist die Leere Region mit Startstadt ([templates.md](templates.md)). Fehlt bei `origin: "dspre"`; Version 1 hat das Feld immer |
| `created` | Text | RFC 3339, UTC, Sekunden: `2026-10-25T14:50:00Z`; Anlegen des Projekts oder erste Zuordnung dieser ROM. Die Agenten-Freigabe hängt an `<sha1>/<created>` |
| `assigned` | Text, optional | RFC 3339, UTC: letzte Zuordnung; nur bei `origin: "dspre"` |
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

## Zuordnen (DSPRE-Ordner)

`check_base(projekt, nds)` prüft eine `.nds` als Basis-ROM und schreibt nichts. Ergebnis `BaseCheck`
mit `info` (`inspect_rom`), `warnings` (Art und deutscher Satz) und `refused`:

1. **Spielcode** der ROM gegen `gamecode` aus `header.yaml` (`Project::open`): weicht er ab, ist das
   die einzige Sperre, `refused` = „Falsches Spiel: die ROM ist {rom}, das Projekt {projekt}.“; dann
   gibt es keine Warnungen.
2. **Titel** gegen `title` aus `header.yaml`: `title_differs`, „Der Titel der ROM („{rom}“) weicht vom
   Projekt ab („{projekt}“).“
3. **Version** (`0x1E`) gegen `rom_version` aus `header.yaml`, nur wenn der Schlüssel da ist (ds-rom
   schreibt ihn): `revision_differs`, „Die Version der ROM ({rom}) weicht vom Projekt ab ({projekt}).“
4. **Bekannter Dump**: steht die SHA-1 nicht in `base_roms.yaml`, `unknown_dump`, „Unbekannte oder
   veränderte ROM; Patches gelten nur für genau diese Datei.“
5. **ARM7**: `arm7/arm7.bin` gegen die ARM7-Bytes der ROM (ds-rom `raw::Rom`), `arm7_differs`, „Der
   ARM7-Code der ROM weicht vom Projekt ab; vielleicht ist es die falsche ROM.“
6. **Banner**: die Titel aus `banner/banner.yaml` (alle Sprachen) gegen die Titel des Banners der ROM,
   `banner_differs`, „Das Banner der ROM weicht vom Projekt ab; vielleicht ist es die falsche ROM.“

Punkt 5 und 6 gelten nur, wenn beide Seiten lesbar sind (Datei im Projekt da und lesbar, Bereich in der
ROM gültig); sonst gibt es keine Warnung. Die Warnungen kommen in dieser Reihenfolge und sperren nie.

`assign_base(projekt, nds)` ordnet zu und gibt `(BaseCheck, BaseMark)` zurück:

- zuerst `mark_state`: eine unlesbare Marke wird mit „Die Basismarke .g4/base.json ist nicht lesbar
  ({fehler}); entferne sie, dann ordne neu zu.“ abgelehnt, eine neuere mit „Die Basismarke stammt aus
  einer neueren Fassung von Gen4 Studio (Version {v}).“; die Datei bleibt, wie sie ist;
- dann `check_base`; `refused` wird zur Ablehnung (`Refused`), nichts wird geschrieben;
- hat die vorhandene Marke dieselbe SHA-1, wird nichts geschrieben und die Marke unverändert
  zurückgegeben (auch bei `origin: "extracted"`);
- eine andere SHA-1 bei `origin: "extracted"` (oder einem anderen Ursprung als `dspre`) lehnt ab: „Das
  Projekt wurde aus einer ROM angelegt; seine Basis-ROM steht fest.“;
- sonst (keine Marke oder `origin: "dspre"` mit anderer SHA-1) schreibt sie eine neue Marke
  `version: 2`, `origin: "dspre"`, ohne `template`, `created` = `assigned` = jetzt, Secure-Area-CRC und
  -SHA-1 aus der `.nds` wie beim Anlegen. Mit dem neuen `created` ändert sich die `base_id`.

**Vorschlag.** `suggest_base(projekt)` liefert `<Eltern>/<Name>.nds`, wenn der Projektordner
`<Name>_DSPRE_contents` heißt (`<Name>` nicht leer), die Datei daneben liegt und ihr Kopf (die ersten
0x200 Byte, kein Hash) denselben Spielcode trägt wie `header.yaml`; sonst `None`.

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

1. ohne lesbare Marke geschieht nichts (DSPRE-Ordner ohne Zuordnung bleiben, wie sie waren); der
   Ursprung der Marke spielt keine Rolle;
2. trägt die ROM nicht den Spielcode der Marke (`0x0C..0x10`), geschieht ebenfalls nichts;
3. hat die gebaute Secure Area nicht den SHA-1 aus der Marke, bleibt `0000` stehen (die CRC gilt
   nur für diese Bytes);
4. sonst wird `secure_area_crc` nach `0x6C` geschrieben und die Header-CRC bei `0x15E` neu
   berechnet: CRC-16/MODBUS (Startwert `0xFFFF`, Polynom `0xA001` gespiegelt) über `0x000..0x15E`,
   Little Endian.

So gleicht die gebaute ROM eines unveränderten Projekts der Eingabe Bit für Bit (gated Test
`project_start_create_then_build_matches_the_rom`, SHA-1 gleich). Dasselbe gilt für einen
unveränderten DSPRE-Ordner, sobald ihm seine `.nds` zugeordnet ist: die Marke mit `origin: "dspre"`
liefert die Secure-Area-CRC (gated Test `base_assign_on_the_vanilla_folder_has_no_warning_and_rebuilds_the_nds`,
Platin DE `d40f19ac24d67c721a4ab25e1c013154443d75b4`). Ohne Marke weicht ein solcher Neubau genau in
den vier Bytes `0x6C..0x6E` und `0x15E..0x160` ab.

## Der Ordner `.g4`

`.g4/` ist der Werkzeugordner von Gen4 Studio im Projekt. Er hat zwei Teile, die sich verschieden
verhalten:

- **Die oberste Ebene ist Projekt-Metadaten und versioniert:** `base.json` (diese Marke) und
  `preview.png` (das Vorschaubild des Projekts). Sie gehören zum Projekt wie die Quellen und
  wandern mit ihm in Git.
- **Die Unterordner sind lokal und dürfen jederzeit gelöscht werden:**

  | Ordner | Inhalt |
  |---|---|
  | `cache/` | berechnete Ansichten, zum Beispiel das Standard-Vorschaubild |
  | `tmp/` | Temp-Dateien atomarer Schreibvorgänge (Schreiben, dann Umbenennen) |
  | `build/` | der Build-Cache (`build::BUILD_DIR`): `blz/` und der Bauzustand |
  | `backup/` | Sicherungen vor Umbauten, zum Beispiel `dynamicHeaders-<Zeit>/` |
  | `models/` | der Cache der Kartenmodelle (GLB über apicula) |

  Fehlt einer, legt der Kernel ihn bei Bedarf neu an. Ohne `build/` baut der nächste Build voll
  und byte-gleich, der Ordner ist Cache, nie Quelle. „Neu bauen (ohne Cache)“ löscht ihn
  (`build::clear_cache`).

Projektdateien wie `g4-checks.json` und `g4-world.json` liegen in der Projektwurzel, weil sie
Projektinhalt sind, nicht weil `.g4/` gelöscht werden dürfte.

### BLZ-Cache `build/blz/`

ds-rom komprimiert beim Laden eines Projekts das ARM9 und jedes Overlay mit `compressed: true`
(BLZ), und das ist der größte Teil eines Builds. Der Build lädt darum ohne Kompression
(`RomLoadOptions { compress: false, .. }`) und holt die komprimierten Bytes aus dem Cache
(`crates/g4-kernel/src/build/blz.rs`):

| Datei | Schlüssel |
|---|---|
| `ov<NNN>-<xxh3>.blz` | `NNN` die Overlay-ID, `xxh3` über die unkomprimierten Bytes |
| `arm9-<xxh3>.blz` | `xxh3` über die unkomprimierten Bytes, dazu die Basisadresse |

`xxh3` hat 16 Hex-Ziffern, sein Seed ist der Hash der `Cargo.lock`-Zeile von ds-rom (beim ARM9
mit der Basisadresse, die nach dem Komprimieren in der Build-Info steht). Nach einem Update von
ds-rom trifft darum kein alter Eintrag mehr, auch wenn der Kompressor andere Bytes schreibt.

Ein Treffer setzt die Bytes als komprimiertes Modul ein, ein Fehlschlag komprimiert wie ds-rom
und legt die Datei ab (über `tmp/` und Umbenennen). Passt die Fußzeile einer Datei nicht zum
Modul (Länge, Kopflänge, Zuwachs beim Entpacken), etwa weil sie abgeschnitten ist, gilt sie als
Fehlschlag und wird überschrieben. Die ROM ist dieselbe wie mit ds-roms eigener Kompression
(gated Test `blz_cache_path_equals_rom_load` je Projekt). Signiert `arm9_overlays/overlays.yaml`
ein Overlay oder die Tabelle, lädt der Build wie ds-rom mit Kompression (`BlzReport.fallback`);
in Platin und HeartGold kommt das nicht vor. In den DSPRE-Ordnern von HeartGold DE und US liegen
nach dem ersten Build 128 Dateien im Ordner (127 von 129 Overlays und das ARM9), in dem von
Platin DE keine (dort ist nichts als komprimiert markiert). Alte Einträge bleiben liegen, bis der
Ordner gelöscht wird.
