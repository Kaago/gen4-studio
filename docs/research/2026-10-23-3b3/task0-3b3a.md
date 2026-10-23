# Task 0 (3b.3a): Prüfungen für Symbol-Paletten, Formnamen und Namenszahlen

Stand 2026-10-23. Alles lesend geprüft an den drei Testprojekten unter `../Roms/*_DSPRE_contents`:
Platin DE (CPUD), HeartGold DE (IPKD) und HeartGold US (IPKE).
Gelesen wurden `arm9/arm9.bin` und `unpacked/<name>/NNNN`. Textarchive wurden mit chatot 0.8.1 dekodiert (`decode_archive`, Standard-Charmap).
Die Skripte und PNGs liegen nur im Scratch und sind nicht im Repo.
Quellen aus pret: `pokeplatinum` (`src/pokemon_icon.c`, `src/pokemon.c`, `res/pokemon/meson.build`, `res/pokemon/form_registry.json`, `generated/vars_flags.txt`, `src/trainer_encounter.c`) und `pokeheartgold` (`src/pokemon_icon_idx.c`, `include/constants/species.h`, `include/constants/flags.h`), jeweils Stand `HEAD` am 2026-10-23.

## Übersicht „geprüft / offen“

| Punkt | CPUD | IPKD | IPKE | Status |
|---|---|---|---|---|
| `mon_icon_palettes` Offset | `0xF07D8` | `0xFFBC4` | `0xFFC10` | geprüft |
| Länge der Paletten-Tabelle | 540 B | 544 B | 544 B | geprüft (nicht 508, siehe 1.2) |
| `monIcons`-NARC | `poketool/icongra/pl_poke_icon.narc`, 547 Member | `a/0/2/0`, 551 Member | `a/0/2/0`, 551 Member | geprüft |
| Formen in Personal 496–507 | Reihenfolge wie Tabelle 2 | dto. | dto. | geprüft (pret und Basiswerte) |
| Species-Namen | 412: 496 | 237: 496 | 237: 496 | geprüft |
| Attacken-Namen / `move_descriptions` | 647: 468 / **646**: 468 | 750: 468 / **749**: 468 | 750: 468 / **749**: 468 | geprüft |
| Item-Namen / `item_descriptions` | 392: 468 / **391**: 468 | 222: 537 / **221**: 537 | 222: 537 / **221**: 537 | geprüft |
| Fähigkeiten | 610: 124 | 720: 124 | 720: 124 | geprüft |
| Typen | 624: 18 | 735: 18 | 735: 18 | geprüft |
| Trainernamen | 618: 928 | 729: 738 | 729: 738 | geprüft |
| Trainerklassen | 619: 105 | 730: 129 | 730: 129 | geprüft |
| Kampftexte | 617: 2497 | 728: 1717 | 728: 1717 | geprüft |
| Stichprobe Attacke 17 | „Flügelschlag“ | „Flügelschlag“ | „Wing Attack“ | geprüft |
| Stichprobe Item 17 | „Trank“ | „Trank“ | „Potion“ | geprüft |
| `trtbl`-Eintrag i ↔ Nachricht i | 2497 = 2497 | bestätigt | bestätigt | geprüft (HGSS mit Stichproben) |
| Name „Pokémon geben“ (rotom-DB) | `GivePokemon` (ID 150) | `GiveMon` (ID 137) | `GiveMon` (ID 137) | geprüft |
| Sichtweite im Objekt-Event | `data[0]` | `data[0]` | `data[0]` | Pt geprüft (pret-Code), HGSS plausibel |
| Trainer-Flags | ab `0x550`, 928 Flags (Trainer 0–927) | ab `0x550` bis `0x82E`: 734 | dto. | Pt geprüft (pret), HGSS aus Spec |
| Übrige Punkte aus Spec §15 | – | – | – | offen (siehe 4) |

Die Schreibweise „412: 496“ bedeutet: Archiv 412 hat 496 Nachrichten.

## 1. Pokémon-Symbol-Paletten (`arm9_tables.yaml`: `mon_icon_palettes`)

### 1.1 Befund

- Die 508 Bytes ab dem Kandidaten-Offset liegen in allen drei Projekten in {0, 1, 2}. Verteilung: 236 × 0, 117 × 1, 155 × 2.
- Die Folge in {0, 1, 2} reicht ab dem Offset genau **540** Bytes (CPUD) bzw. **544** Bytes (IPKD, IPKE). Danach folgt ein Byte außerhalb von {0, 1, 2}.
  - Das ist genau die Zahl der NCGR-Member im `monIcons`-NARC: 547 − 7 = 540 und 551 − 7 = 544.
- Vor dem Offset stehen ebenfalls Bytes in {0, 1, 2} (118 bzw. 129 Bytes). Das Muster allein legt den Anfang also nicht fest.
  - Den Anfang belegt der Vergleich mit pret.
  - `pokeheartgold` `sPokemonPalNoBySpeciesAndForm` hat 544 Einträge. Sie sind identisch mit den 544 Bytes ab `0xFFBC4` (IPKD) und `0xFFC10` (IPKE): 0 Abweichungen.
  - Die ersten 540 Einträge dieser Tabelle sind auch identisch mit den 540 Bytes ab `0xF07D8` (CPUD): 0 Abweichungen.
  - In `pokeplatinum` ist die Tabelle generiert (`species_icon_palettes.h`) und nicht direkt lesbar.
- Der Abstand IPKE − IPKD beträgt 0x4C, wie bei der Item-Tabelle.

### 1.2 Bedeutung des Index

Die Tabelle ist nach **Symbol-Index** geordnet (NCGR-Member − 7), nicht nach Personal-Member. Das zeigen `PokeIconPaletteIndex` (Pt) und `GetMonIconPaletteEx` (HGSS).
Für die Arten 0–493 sind beide Indizes gleich. Für Ei und Formen weichen sie ab.

| Symbol-Index | Inhalt (Pt und HGSS gleich) |
|---|---|
| 0–493 | Art 0 („?“) bis Arceus |
| 494 | Ei |
| 495 | Manaphy-Ei |
| 496–498 | Deoxys Angriff, Verteidigung, Initiative |
| 499–526 | Icognito A–Z, !, ? (Index 499 = A, Duplikat des Basis-Symbols) |
| 527–528 | Burmy Sand, Lumpen |
| 529–530 | Burmadame Sand, Lumpen |
| 531 / 532 | Schalellos Ost / Gastrodon Ost |
| 533 | Giratina Urform |
| 534 | Shaymin Zenitform |
| 535–539 | Rotom Hitze, Wasch, Frost, Wirbel, Schneid |
| 540–542 | nur HGSS: Formeo Sonne, Regen, Schnee (Kampf-Symbol) |
| 543 | nur HGSS: Kinoso Sonnenform (Kampf-Symbol) |

Die Tabellenwerte ab 494 lauten in allen drei Projekten `1, 2, 0, 0, …`: Ei 1, Manaphy-Ei 2, Deoxys-Formen 0.

**Folge für Task 5 und Task 11:**
- `mon_icon_palettes` hat Länge 540 (CPUD) bzw. 544 (IPKD, IPKE), nicht 508.
- Wer ein Symbol für ein Personal-Member sucht, braucht diese Zuordnung:

| Personal-Member | NCGR-Member in `monIcons` | Palette |
|---|---|---|
| 0–493 | Member + 7 | `table[Member]` |
| 494 (Ei) | 501 | `table[494]` |
| 495 (Bad Egg) | 7 (Symbol „?“; das Spiel nimmt Art 0 für Arten > 493) | `table[0]` |
| 496–498 | 503–505 | `table[496..498]` |
| 499–500 | 536–537 | `table[529..530]` |
| 501 | 540 | `table[533]` |
| 502 | 541 | `table[534]` |
| 503–507 | 542–546 | `table[535..539]` |

### 1.3 Augenschein

Gerendert wurden NCGR-Member 8, 11 und 14 (Art 1, 4, 7). Format: 32 × 64, 4 bpp, gekachelt, 1D (zwei 32 × 32-Frames untereinander).
Als Paletten dienten die ersten drei der 16 Paletten in NCLR-Member 0. Der Datenblock ist 0x200 B groß und liegt ab Byte 0x28; Farben in RGB555.
Gerendert wurde jedes Symbol einmal mit der Palette seines Tabellenwerts und einmal mit allen drei Paletten nebeneinander.

| Art | Tabellenwert | Ergebnis mit dieser Palette | Gegenprobe mit den anderen Paletten |
|---|---|---|---|
| Bisasam (1) | 1 | grün mit grünem Knospenkörper, rote Augen: passt | Palette 0 gelb-braun, Palette 2 violett-blau |
| Glumanda (4) | 0 | orange mit oranger Schwanzflamme: passt | Palette 1 grün-rosa, Palette 2 braun |
| Schiggy (7) | 0 | hellblau mit braunem Panzer: passt | Palette 1 gelb-grün, Palette 2 violett |

Die Bilder und die Pixel sind in allen drei Projekten gleich. Die häufigsten Farben im ersten Frame (CPUD = IPKD = IPKE):
- Bisasam: (123,156,74) und (156,205,74).
- Glumanda: (213,98,65) und (246,148,41).
- Schiggy: (164,197,255).

**Ergebnis:** Offset `0xF07D8` (CPUD), `0xFFBC4` (IPKD), `0xFFC10` (IPKE). Keiner ist `null`.

## 2. Formnamen (`species_forms.yaml`, Personal-Member 496–507)

Die Reihenfolge ist durch drei Quellen belegt:
- `pokeheartgold` `include/constants/species.h` mit `SPECIES_DEOXYS_ATK 496` … `SPECIES_ROTOM_MOW 507`.
- `pokeplatinum` `Pokemon_GetFormNarcIndex` mit den Basen 496, 499, 501, 502 und 503. Die Rotom-Basis ergibt sich aus `form_registry.json`: Formen vom Typ „data“ in der Reihenfolge Deoxys, Wormadam, Giratina, Shaymin, Rotom.
- Die Basiswerte (KP/Ang/Vert/Init/SpA/SpV und Typen) der Member stimmen. Sie sind in allen drei Projekten gleich.

Die deutschen Formnamen stammen aus dem Spiel selbst. In Pt steht der Pokédex-Text in Archiv 697, Nachrichten 17–19 und 111–124. In HGSS ist es Archiv 802, Nachrichten 118–120 und 145–158. Die Inhalte sind in beiden Spielen gleich.
Die Umhang-Namen sind im Spiel abgekürzt („Lumpenumh.“); die Tabelle nennt die volle Form.
Die Basisarten heißen laut Archiv 412/237 (DE) „DEOXYS“, „BURMADAME“, „GIRATINA“, „SHAYMIN“, „ROTOM“.

| Member | Basisart (ID) | Formname deutsch | Spieltext | Kennwerte (Pt = HGSS) |
|---|---|---|---|---|
| 496 | Deoxys (386) | Angriffsform | 697/112, 802/146 | 50/180/20/150/180/20 |
| 497 | Deoxys (386) | Verteidigungsform | 697/113, 802/147 | 50/70/160/90/70/160 |
| 498 | Deoxys (386) | Initiativeform | 697/114, 802/148 | 50/95/90/180/95/90 |
| 499 | Burmadame (413) | Sandumhang | 697/18, 802/119 | Käfer/Boden |
| 500 | Burmadame (413) | Lumpenumhang | 697/19, 802/120 („Lumpenumh.“) | Käfer/Stahl |
| 501 | Giratina (487) | Urform | 697/118, 802/152 | 150/120/100/90/120/100 |
| 502 | Shaymin (492) | Zenitform | 697/116, 802/150 | Pflanze/Flug |
| 503 | Rotom (479) | Hitze-Rotom | 697/120, 802/154 („HITZE-ROTOM“) | |
| 504 | Rotom (479) | Wasch-Rotom | 697/121, 802/155 | |
| 505 | Rotom (479) | Frost-Rotom | 697/122, 802/156 | |
| 506 | Rotom (479) | Wirbel-Rotom | 697/123, 802/157 | |
| 507 | Rotom (479) | Schneid-Rotom | 697/124, 802/158 | |

Rotom 503–507 haben in allen drei Projekten dieselben Werte (50/65/107/86/105/107, Elektro/Geist).
Ei und Bad Egg (494/495) heißen in Archiv 412/237 „Ei“ / „Schl. Ei“ (DE) und „Egg“ / „Bad Egg“ (US).
Die Normalformen heißen im Spieltext „Normalform“ (697/111), „Pflanzenumh.“ (697/17), „Wandelform“ (697/117) und „Landform“ (697/115).
Englische Formnamen stehen in IPKE Archiv 802 (z. B. 146 „Attack Forme“, 150 „Sky Forme“, 155 „WASH ROTOM“).

Hinweis: Die Archive 712–716 (Pt) bzw. 817–821 (HGSS), die `dspre-report.md` als „Formnamen o. ä.“ führt, enthalten keine Formnamen. Sie sind die Artnamen in fünf Sprachen (EN, FR, DE, …) mit je 494 Nachrichten, in Pt mit Dex-Nummer („001  BISASAM“).

## 3. Namenszahlen und Archivnummern

Gezählt wurde mit chatot, je Archiv die Nachrichtenzahl.

| Bank | CPUD | IPKD | IPKE |
|---|---|---|---|
| Species-Namen | 412 (496) | 237 (496) | 237 (496) |
| Species mit Farbcode (Alternativbank) | 413 (496) | 238 (496) | 238 (496) |
| Attacken-Namen | 647 (468) | 750 (468) | 750 (468) |
| `move_descriptions` | **646** (468) | **749** (468) | **749** (468) |
| Item-Namen | 392 (468) | 222 (537) | 222 (537) |
| `item_descriptions` | **391** (468) | **221** (537) | **221** (537) |
| Item-Namen mit Farbcode | 393 (468) | 223 (537) | 223 (537) |
| Fähigkeiten | 610 (124) | 720 (124) | 720 (124) |
| Typen | 624 (18) | 735 (18) | 735 (18) |
| Trainernamen | 618 (928) | 729 (738) | 729 (738) |
| Trainerklassen | 619 (105) | 730 (129) | 730 (129) |
| Kampftexte | 617 (2497) | 728 (1717) | 728 (1717) |

Damit ist die offene Frage aus `dspre-report.md` §0.3 entschieden: Die Item-Beschreibungen in Pt liegen in **391**.
393 enthält die Item-Namen mit `{COLOR}`-Steuerzeichen (z. B. „{COLOR, 0, 255}Trank{COLOR, 0, 0}“), in IPKE mit Artikel („a {COLOR…}Potion…“).

**Stichproben (Index 17):**

| | CPUD | IPKD | IPKE |
|---|---|---|---|
| Attacke 17, Name | Flügelschlag | Flügelschlag | Wing Attack |
| Attacke 17, Beschreibung | „Trifft den Gegner mit weit gespreizten Flügeln.“ | dto. | „The foe is struck with large, imposing wings …“ |
| `moveData/0017` | phys., Stärke 60, Typ 2 (Flug), Gen. 100, AP 35 | dto. | dto. |
| Item 17, Name | Trank | Trank | Potion |
| Item 17, Beschreibung | „Hilft bei Wunden. Die KP eines Pokémon werden um 20 Punkte aufgefüllt.“ | dto. | „A spray-type medicine for wounds. …“ |
| arm9-Item-Tabelle, ID 17 | Daten 17, Symbol 24, Palette 25, GBA 13 | dto. | dto. |
| `itemData/0017` Preis | 300 | 300 | 300 |

Die Zuordnungen „Attacken-Nachricht i ↔ Attacke i“ und „Item-Nachricht i ↔ Item-ID i“ stimmen in den Stichproben.
Nachricht 0 lautet in den Beschreibungsbanken „-“ bzw. „???“; Item 0 heißt „Keines“/„None“.

## 4. Befunde zu Spec §15

| Punkt | Ergebnis | Status |
|---|---|---|
| `trtbl`-Eintrag → Nachricht (HGSS, Archiv 728) | `trainerTextTable/0000` hat 1717 Einträge à 4 B {u16 Trainer, u16 Auslöser}, Archiv 728 hat 1717 Nachrichten. Stichproben siehe unten. Eintrag i ↔ Nachricht i stimmt in IPKD und IPKE. | geprüft |
| Name des „Pokémon geben“-Befehls (rotom-DB, Revision 33) | Pt: `GivePokemon`, ID 150 (0x96), Parameter `pokémon`, `level`, `item` (flex) und `successVar` (Standard `VAR_RESULT`). HGSS: `GiveMon`, ID 137 (0x89), `legacy_name` `GivePokemon`, Parameter `pokémon`, `level`, `item`, `form`, `ability` (u16) und `retvar`. Die Namen unterscheiden sich je Familie. Das Ei heißt `GiveEgg` (Pt ID 151) bzw. Legacy `GivePokemonEgg` (HGSS ID 138). | geprüft |
| Sichtweite im Objekt-Event | `data[0]`. Pt: `pokeplatinum` `src/trainer_encounter.c`, `GetTrainerDistToPlayer` liest `MapObject_GetDataAt(trainerMapObj, 0)` als `trainerSightRange` für `TRAINER_TYPE_NORMAL` und „alle Richtungen“. pret `events_route_202.json` hat z. B. Trainer Tristan mit `data: [5]`. HGSS: den Code nicht gelesen (GitHub-Codesuche ohne Treffer). Belege aus den Daten siehe unten. | Pt geprüft, HGSS plausibel |
| Startindex der Trainer-Flags in Pt | `pokeplatinum` `generated/vars_flags.txt`: `TRAINER_DEFEATED_FLAGS_START` = `FLAG_DEFEATED_TRAINER_NONE` = **0x550** (1360), `TRAINER_DEFEATED_FLAGS_END` = 0x8EF. Das sind 928 Flags für Trainer 0–927, also für alle 928 Datensätze. Danach folgt `FLAG_UNK_0x08F0`. Die Werte sind aus der Reihenfolge der Liste errechnet (Zählung gegen 1769 Flags mit Hex-Namen; 1 Abweichung, `FLAG_DUMMY_0x0193` in Zeile 408 = 0x195, ein Namensfehler vor dem Trainerblock; alle späteren Hex-Namen stimmen). HGSS (Spec): 0x550 bis 0x82E, Platz für 734 von 738. | Pt geprüft (pret), HGSS aus Spec |
| Rate 100 je Schritt garantiert Begegnung | nicht untersucht (Laufzeitverhalten, 3b.3b) | offen |
| Slot-Anteile gegen pret | nicht untersucht (3b.3b, Begegnungen) | offen |
| Default-Attacken-Regel für Trainer ohne Attacken | nicht untersucht (3b.3b, Trainer) | offen |
| Renderzeit der Listen, Aufbauzeit des Index | erst nach der Umsetzung messbar | offen |
| Gras-Verhalten auf kopierter Karte in neuem Header | braucht einen Emulatorlauf (3b.3b) | offen |
| Suche bei fehlendem `trtblofs`-Eintrag (HGSS 735 bei 738 Trainern) | nicht untersucht | offen |

**Stichproben zu `trtbl` ↔ Archiv 728:**
- Trainer 120 (IPKE „Irene“, IPKD „Lola“) hat die Einträge 699/700/701 mit den Auslösern 0/1/2.
  - IPKE: Nachricht 699 „Irene: Eeek! A Trainer!“, 700 „Irene: Ohhh! Too strong!“, 701 „… My sister Kate will get you for this!“.
  - IPKD: „Lola: Aha! Mädels! …“, „Lola: Oooh! Du hast mich umgehauen!“, „… meine Schwester Edna …“.
- Hier stimmen Name, Auslöser (Begrüßung, Niederlage, danach) und Text überein.
- Weitere Treffer zum Auslöser 1 (Niederlage): Eintrag 100 → Trainer 34 (IPKE „Chuck“) „We...lost...“, 103 → Trainer 32 („Pryce“) „… my luck has run out.“, 1104 → Trainer 200 („Grunt“) „Oh, no! They're so useless!“.

**Auslöser-Häufigkeit:**
- Pt: 0: 516, 1: 812, 2: 516, 3–14 (Doppelkampf und Sonderfälle), 15: 26, 16: 52, 17: 382, 18–19: je 8.
- HGSS: 0: 383, 1: 616, 2: 383, 3–10, 15: 68, 16: 71, 17: 45, 18–19: je 2, 20: 23.
- Pt beginnt mit Trainer 247 (Auslöser 13, 16, 1), HGSS mit Trainer 495.

**Belege aus den Daten zur Sichtweite:**
- Pt: 446 Objekte mit `trainer_type` ≠ 0, davon 407 mit Script 3000–4999. `data[0]` liegt bei ihnen in 0–6, Schwerpunkt 2–3.
- HGSS: 407 solche Objekte. `data[0]` liegt in 0–7, Schwerpunkt 3–4.
- Bei Objekten ohne Trainer-Typ ist `data[0]` fast immer 0 (HGSS: 2224 von 2260).

## 5. Folgerungen für die Tasks

- **Task 5** (`arm9_tables.yaml`): `mon_icon_palettes` mit `CPUD: { offset: 0xF07D8, count: 540 }`, `IPKD: { offset: 0xFFBC4, count: 544 }`, `IPKE: { offset: 0xFFC10, count: 544 }`.
  - Plausibilität: jeder Wert in {0, 1, 2}.
  - Textbanken: `move_descriptions` Pt 646 / HGSS 749, `item_descriptions` Pt 391 / HGSS 221.
- **Task 9** (`species_forms.yaml`): Tabelle 2. Pt und HGSS sind gleich.
- **Task 11** (`monIcons`): Member 0 ist NCLR (16 Paletten, genutzt 0–2), Member 1–6 sind NANR/NCER, ab Member 7 folgen NCGR (1072 B, 32 × 64, 4 bpp, gekachelt 1D).
  - „Art + 7“ gilt nur für die Arten 0–493.
  - Für Ei, Bad Egg und die Formen 496–507 gilt die Zuordnung in 1.2.
  - Der Paletten-Index ist der Symbol-Index (NCGR-Member − 7), nicht die Personal-Nummer.
