# Grenzen und Ausnahmen der Prüfungen (`check_limits.yaml`)

**In English:** Documents `check_limits.yaml`, the table of limits and exceptions behind the project
checks, for example the width of a dialog line in pixels, the maximum size of a map model and of its
BDHC, and the one warp target that is read from the save game. Every entry has a value, a meaning, a
source (a place in the pret decompilations or a measurement on the three test projects) and a
`verified` flag. The file is `crates/g4-kernel/metadata/check_limits.yaml`, read by
`crates/g4-kernel/src/checks/limits.rs`. Besides the checks, the map part import and the per-map
budget read it.

Quelle der Werte: [task0-3b4.md](../research/2026-10-23-3b4/task0-3b4.md) §1–§4 (pret `pokeplatinum`
`be36a9db3d2390a5009adc087c7aef550ce335e7`, `pokeheartgold` `6988158691eb6178cff697cda0428045ddcba5fb`, Messungen auf Platin DE,
HeartGold DE und HeartGold US) und [checks-report.md](../research/2026-10-23-3b4/checks-report.md).
Implementierung: `crates/g4-kernel/metadata/check_limits.yaml`, gelesen von `crates/g4-kernel/src/checks/limits.rs`
(`Limits`, `Project::limits()`, einmal je Prozess geparst). Außer den Prüfungen liest sie der Kartenimport
(`map_import.rs`: `model.max_bytes`, `bdhc.max_bytes`) und ab Phase 6.2 das Budget je Karte (`get_map_budget`:
`model.max_bytes`, `bdhc.max_bytes`, `buildings.max_loaded`, `vram.*`).

Jeder Eintrag hat `verified` (gegen den Code der Spiele belegt, nicht nur gemessen) und `source` (Fundstelle, Pflicht,
ein Test prüft, dass keiner leer ist). Ein Paar oder ein Wert je Familie nennt seine Felder selbst. Hex-Werte stehen
wie im Spiel.

| Schlüssel | Wert | Bedeutung | Quelle | verified |
|---|---|---|---|---|
| `dialog_line_px` | 216 | Breite des Feld-Textfensters in Pixeln (27 Kacheln zu 8 px, Druck ab x 0), beide Familien. `text.too-wide` meldet Zeilen darüber. | pokeplatinum `src/field_message.c:30`, pokeheartgold `src/dialog_box.c:12–15`; task0 §1 | ja |
| `dialog_lines_max` | 2 | Zeilen, die das Textfenster bis zum Seitenwechsel zeigt (Fenster 4 Kacheln hoch). `text.three-lines`. | gleiches `Window_Add`; der 16-px-Zeilenabstand ist in pret nicht nachgeschlagen | nein |
| `warp.dynamic` | `header: 0x0FFF`, `warp: 256` | Ein Warp mit genau diesem Ziel-Paar liest sein Ziel aus dem Spielstand (Aufzüge). Kein Fehler in `warp.dest-header` und `warp.dest-warp`, kein `warp.never-fires`. Nur das Paar zählt, nicht ein Einzelwert. | pokeplatinum `src/overlay005/field_control.c:1009–1013`, pokeheartgold `src/field/field_control.c:919–921`; task0 §2; Vanilla Pt 6, HG DE/US je 9, alle genau dieses Paar | ja |
| `trainer.dummy` | 0 | Trainer 0 ist der Platzhalter der Trainertabellen (keine Party, Klasse 0, kein Event-Nutzer); `trainer.no-party` nimmt ihn aus. | Messung auf allen drei Projekten; task0 §4 | nein |
| `trainer.sight_slot` | `pt: 929`, `hgss: 740` | Die Trainer-ID des „gesehen“-Slots (`seen_script − 2999`): ihr Eintrag im Kampf-Script ist der „gesehen“-Code, sie ist nie ein echter Trainer. `create_trainer` (3b.3b) legt dort einen gesperrten Platzhalter an (Eigenschaften 0, kein Team). Die Trainer-Prüfungen (`trainer.no-party` und die übrigen `trainer.*`) übergehen einen Trainer dieser Form an dieser ID; ein eigener Trainer eines Hacks dort wird geprüft. Die Sperre des Platzhalters ist kein `asset.unreadable` (das meldet nur Sperren, die die Bytes verursachen). | `metadata/trainer_scripts.yaml` (pret pokeplatinum `trainer_encounter.c`, pokeheartgold `std_trainer_approach`), [trainer_scripts.md](trainer_scripts.md); 3b.3b | ja |
| `event_file.shared` | 0 | Eventdatei 0 teilen die Dummy-Header (60 auf Platin DE) mit verschiedenen Scriptdateien; die Prüfung „geteilte Eventdatei“ nimmt sie aus. | Messung; checks-report `event.shared-file` | nein |
| `script.decoration` | 0 | Script-Nummer 0 an einer Figur heißt „kein Script“ (Dekoration, 202 Figuren auf Platin DE): kein `event.script-missing`. | Messung; checks-report `event.trigger-empty` | nein |
| `script.message_params` | je Familie Liste `{command, arg}`: DP 8, Pt 15, HGSS 11 Befehle | Befehle, deren Argument `arg` (ab 0) eine Nachricht des Textarchivs ist, mit dem der Header die Scriptdatei startet (`script.message-missing`). Regel: Parameter `text_slot` oder Typ `msg_id`, ohne Befehle mit eigenem Archiv (`MessageFromBank*`, `*FromArchive`, `MsgBoxExtern`), TV-Befehle und Menüeinträge (`AddMenuEntryImm`, `MenuItemAdd`, HGSS `ScrCmd_066`). Ein Befehl wird über rotoms Datenbank aufgelöst, sein Legacy-Name (DSPRE) und `ScrCmd_*`-Alias zählen also mit; Zahlen ab `0x4000` sind Variablen. | `metadata/scrcmd/*_v2.json` (Revision 33, die Datenbank, die rotom v0.7.0 liest); Regel aus task0 §4 (dort HGSS 12 gezählt, hier 11: `ScrCmd_066` ist ein Menüeintrag) | nein |
| `script.jump_params` | je Familie Liste von Befehlsnamen: DP 4, Pt 6, HGSS 6 | Befehle, deren Bezeichner-Argument ein Script oder Label derselben Datei nennt, zu dem sie springen; `dialog_of` läuft jedes Ziel einmal ([script_file.md](script_file.md), „Dialog einer Figur“). rotoms Sprungwort `GoTo`/`Jump` gilt immer. Ein Name wird über rotoms Datenbank aufgelöst, Legacy-Name (`Jump`, `JumpIf`) und kanonischer Name (`GoTo`, `GoToIf`) zählen gleich. HGSS hat kein `JumpIfSet`/`JumpIfUnset`, seine Flag-Sprünge sind wie in Platin die Makros `GoToIfSet`/`GoToIfUnset`. Ein Test prüft jeden Namen gegen die Datenbank. | `metadata/scrcmd/*_v2.json` (Revision 33): `GoTo`, `Call`, `GoToIf`, `CallIf` mit `relative_jump`, Makros mit Parameter vom Typ `label`; rotom `normalize_control_keyword` | nein |
| `encounter.none` | `pt: 0xFFFF`, `hgss: 0xFF` | Header-Wert für „keine Begegnungen“; `header.wild-dangling` nimmt ihn aus. | Messung; DSPRE avalonia `ProjectIndex`; checks-report `header.wild-dangling` | nein |
| `matrix.header_none` | 0 | Eine Header-Zelle mit 0 heißt „kein Header“ (Header 0 benutzt Matrix 0): `matrix.header-elsewhere` nimmt sie aus. Alle 14/13/13 Vanilla-Treffer sind solche Zellen. | Messung; dass das Spiel 0 so liest, ist nicht in pret belegt (task0 §4) | nein |
| `model.max_bytes` | `0xF000` | Größter Kartenmodell-Puffer, beide Familien. `map.model-invalid` und der Modellimport lehnen größere ab. | pokeplatinum `src/overlay005/loaded_map_buffers.c:11`, pokeheartgold `asm/overlay_01_021FAC44.s:146–147`; task0 §3; größtes Vanilla-Modell Pt 60 556 B, HG 61 420 B | ja |
| `bdhc.max_bytes` | `0x9000` | Größte Gelände-Nutzlast **nach dem 16-Byte-Kopf** (Summe aus den Zählern: Punkte × 8 + Normalen × 12 + Konstanten × 4 + Platten × 8 + Streifen × 8 + Zugriffsliste × 2), beide Familien. `map.bdhc-size` (Fehler) und der Geländeimport. Beide prüfen danach die Struktur ([bdhc.md](bdhc.md), `Bdhc::validate`): jeder Index in seiner Tabelle, jeder Streifen in der Zugriffsliste und aufsteigend; der erste Fehler ist der Grund (`map.bdhc-size`, Import `bdhc.structure`), eine senkrechte Platte (`y = 0`) ist nur eine Warnung und kein Grund. | pokeplatinum `include/constants/field/bdhc.h:4`, `src/overlay005/bdhc.c:282`, pokeheartgold `asm/overlay_01_021FB04C.s:105–109`; task0 §3 | ja |
| `permissions.bytes` | 2048 | Länge des Abschnitts Begehbarkeit (32 × 32 Zellen, je Art und Kollision); `.per`-Import verlangt genau diese Länge. | `g4-formats` (`PERMISSIONS_LEN`), [map.md](map.md); DSPRE `Collisions.toByteArray` | ja |
| `buildings.max_loaded` | 32 | Gebäude je Karte, die das Spiel lädt; weitere übergeht es still. `map.buildings-over-limit` (Warnung) meldet **mehr als** 32, die Budget-Leiste zeigt „n von 32“. | pokeplatinum `include/overlay005/map_prop.h:13` `MAX_LOADED_MAP_PROPS`; in HGSS im asm nicht belegt; Vanilla-Maximum Pt 32, HG 30; Gutachten B §2.4 | ja |
| `vram.bank` | 262 144 | Textur-VRAM des Felds (Bänke A und B, geteilt mit den Figuren); Budget „Texturen des Gebiets“ „n von 262 144 B“. | pokeplatinum `src/fieldmap.c:527` `GX_VRAM_TEX_01_AB`, `src/easy3d.c:83`; Gutachten B §2.8 | ja |
| `vram.vanilla_max` | `pt: 110640`, `hgss: 87424` | Größtes Karten- plus Gebäudepaket eines Gebiets in Vanilla; darüber sagt das Budget „Knapp“ (Diamant/Perl wie Platin). | Messung; Gutachten A §2.4, Entscheidungen Phase 6 | nein |
| `vram.full_estimate` | 204 800 | Darüber sagt das Budget „Voll“; geschätzt, der Anteil der Figuren an der Bank ist nicht gemessen. | Gutachten B §2.8 „Voll geschätzt ab 200 KB“ | nein |

Die Gelände-Prüfungen der Phase 6.2 (`checks/terrain.rs`, im Katalog nach `map.building-range`): `map.altitude-walkable`
(Fehler) liest `matrix.header_none` (eine Zelle mit Karte, Höhe ≠ 0 und Header ≠ 0, oder in einer Matrix ohne
Header-Abschnitt ein Header, der sie nennt); `map.step-unblocked` (Hinweis, Fix `block_upper` „Obere Zelle blockieren“)
liest keinen Wert dieser Datei, sondern die Kantenregel `edge` aus `terrain_rules.yaml` ([terrain.md](terrain.md),
`free_steps`); `map.buildings-over-limit` (Warnung) liest `buildings.max_loaded`.

Ändert sich ein Wert, ändert sich nur die YAML-Datei (und diese Tabelle): Prüfungen und Import lesen ihn aus `Limits`.
`verified: false` heißt: gemessen oder aus anderer Quelle übernommen, am Spielcode nicht nachgeschlagen; der Katalog
behandelt den Wert trotzdem als Ausnahme, weil sie auf allen drei Vanilla-Projekten das Soll 0 für Fehler hält.
