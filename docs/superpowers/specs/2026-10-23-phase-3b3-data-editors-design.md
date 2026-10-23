# Phase 3b.3: Daten-Editoren (Pokémon, Attacken, Gegenstände, Trainer, Begegnungen)

Stand: 2026-10-23. Teil 3 von Phase 3b der [Roadmap](../../ROADMAP.md). Setzt den Doppeldeck-Rahmen voraus ([Spec 3c](2026-10-23-phase-3c-doppeldeck-design.md): Atlas, Liste, Schau, Scharnier, Hand, Primitives, Tiefen Bauen/Tüfteln) und baut auf Text ([Spec 3b.2](2026-10-22-phase-3b2-scripts-text-design.md), Namen kommen aus Textarchiven) und Welt ([Spec 3a](2026-10-22-world-explorer-design.md), Trainer stehen als Events in der Welt) auf. Recherche und die drei Designvorschläge, aus denen diese Spec entschieden wurde, liegen in [docs/research/2026-10-23-3b3/](../../research/2026-10-23-3b3/) (`uxie-report.md`, `dspre-report.md`, `design-report.md`, `proposal-A/B/C.md`); Byte-Layouts und Fundstellen stehen dort, hier nur die Entscheidungen und die Fakten, die sie tragen.

## 1. Ziel und Umfang

Trainer, Begegnungen, Pokémon-Basisdaten, Attacken und Gegenstände entstehen komplett im Studio und wirken im Spiel. Die Oberfläche folgt dem Doppeldeck (Liste · Schau · Hand) nach Pokémon-Studio-Muster: eine Entität je Seite, Suche über Name und Nummer, Referenzzähler „Verwendet in“, harte Grenzen sichtbar („18 von 20“), unbekannte Werte nie still verworfen.

**Zwei Pläne, je mit eigenem Gate:**

- **3b.3a „Codecs und Pokémon-Daten“**: alle Codecs in `g4-formats` (auch Trainer und Begegnungen, damit der Round-Trip von Anfang an vollständig ist), neue Archivarten im Store, mehrteiliger Changeset, Namenstabellen, arm9-Tabellen lesend, Referenzindex, NCGR/NCLR-Dekoder mit Item-Symbolen; Editoren **Pokédex** (Werte, Fähigkeiten und Zucht, Lernset, Entwicklung, TM/VM), **Attacken**, **Gegenstände**; Atlas-Bereiche Pokémon und Gegenstände freigeschaltet.
- **3b.3b „Trainer, Begegnungen, Gate“**: Trainer als zusammengesetztes Asset mit Name und Kampftexten, „Neuer Trainer“, Event ↔ Trainer, Begegnungen mit neuer Datei, Anbindung am Header und „Abspalten“, `PokemonCard`, `g4emu`-Kampfschritte, zwei Szenarien. Gate = Roadmap-Gate 3b.3.

**Später** (steht wörtlich so, damit der Umfang nicht wandert):

| Thema | Wohin | Grund |
|---|---|---|
| Ei-Attacken | 3d oder später | Pt im Overlay 5, HGSS als NARC: zwei Mechaniken |
| Attacken-Lehrer, Läden (Sortiment, Orden-Staffel), Honigbäume, Großmoor, Kopfnuss, Safari, Käferturnier | später (Overlay-Tabellen, Patch-Layer) | je Spiel verschieden, braucht Patch-Schutz |
| Assistent „Neuer Trainer“ mit Live-Vorschau; Erzählen-Layouts für Begegnungen und Läden | 3d (Erzählen) | Erzählen und Live-Vorschau sind in 3c ausgeschlossen |
| DS-Bildschirm-Varianten (Pokédex-Profil, Kampfvorschau, Laden) | 3d | Schau-Kosmetik ohne Beweiswert |
| Kampfsprites von Pokémon und Trainern | 3d | verschlüsselte NCGR, Pt und HGSS verschieden (offen) |
| Pokémon-Menüsymbole | 3b.3a, wenn Plan-Task 0 die Paletten-Tabelle bestätigt, sonst 3d | Kandidat: arm9 Pt DE 0xF07D8, HG DE 0xFFBC4, HG US 0xFFC10 (Muster gefunden, Bedeutung ungeprüft) |
| arm9-Tabellen schreiben (Item-Tabelle, TM-Liste), Klassen-Geschlecht, Preisfaktor, Eye-Music, Trainername > 7 Zeichen | später | Schreiben in arm9 braucht Patch-Schutz; Offsets teils nur aus DSPRE |
| Löschen von Daten, Projektprüfungen (Trainer ohne Party, leere Ersatz-Slots projektweit), Script → Trainer/Item | 3b.4 | Roadmap; Löschen ohne Script-Referenzen wäre ein leeres Versprechen |
| Pokédex-Größe/-Gewicht, Wachstumstabellen, „Auf Original zurücksetzen“ | Phase 4 (Basis-ROM-Vergleich) | Zukan-Daten, Basis-ROM |
| Diamant/Perl | kein Gate | kein Testprojekt; die Metadaten kennen die Pfade, der Editor sperrt DP mit Grund |

**Gate 3b.3a** (Ablauf in `docs/phase-3b3-gate.md`, Teil A):
- `vanilla_data` (gated) grün: `write(read(x)) == x` über alle Member der zehn Archivarten in Platin DE, HeartGold DE, HeartGold US, mit Soll-Member-Zahlen; das uxie-Orakel stimmt für Personal, Items und Trainer-Eigenschaften Feld für Feld überein. Der Vanilla-Rebuild bleibt byte-identisch, obwohl der Build jetzt auch die zehn neuen Ordner packt.
- Szenario `pt-daten.yaml` (Backend): Staravia Angriff 75 → 85, eine Lernset-Zeile mehr, Flügelschlag Stärke 60 → 65, Trank Preis 300 → 250; speichern, bauen; ein Prüfschritt liest die gebauten NARCs mit uxie und erwartet die neuen Werte.
- Von Hand und per Test-Umgebung: die drei Editoren sind gegen die Screens „Pokédex: Staravia bearbeiten“, „Attacken: Flügelschlag“, „Gegenstände: Trank“ wiedererkennbar (ohne DS-Profil); ⌘Z nimmt „Wie Staralili“ (TM-Kopie über zwei Assets) in einem Schritt zurück; Tüfteln zeigt den Hexblock mit markiertem Feld; Item-Symbole aus dem ROM in der Liste; `scripts/gate.sh` grün.

**Gate 3b.3b** (Teil B, Roadmap-Gate): Auf Kopien von Platin DE und HeartGold DE entstehen zwei neue Trainer mit je einem Pokémon eindeutiger Art und Stufe, platziert als Events in Sichtlinie auf einem neuen Ort, und eine neue Begegnungsdatei am Header dieses Orts; `g4emu` läuft hinein, `expect-battle` liest Art und Stufe des Gegners aus dem RAM (§13), gewinnt beide Kämpfe und löst im Gras eine Begegnung mit der gesetzten Art aus. Eine geänderte Vanilla-Begegnungsdatei (Route 201, Slot 0) wird im Build per uxie geprüft. Screenshots belegen für den Menschen („Wanderer Jens möchte kämpfen!“); der Beweis ist der RAM.

## 2. Fakten

Geprüft an den drei Testprojekten, sofern nicht „offen“ (Details und Fundstellen: `uxie-report.md` §1, §4; `dspre-report.md` §1–§3; `proposal-A.md` §0).

**Archive** (DSPRE-Ordner `unpacked/<name>`, in allen drei Projekten vorhanden und byte-identisch zum NARC):

| Art | Ordner | Pt (`files/…`) | HGSS (`files/a/…`) | Member Pt / HGSS | Größe |
|---|---|---|---|---|---|
| Begegnungen | `encounters` | `fielddata/encountdata/pl_enc_data.narc` | `0/3/7` (SoulSilver `1/3/6`) | 183 / 142 | 424 / 196 B |
| Trainer-Eigenschaften | `trainerProperties` | `poketool/trainer/trdata.narc` | `0/5/5` | 928 / 738 | 20 B |
| Trainer-Party | `trainerParty` | `poketool/trainer/trpoke.narc` | `0/5/6` | 928 / 738 | 8..n, Vielfaches von 4 |
| Pokémon-Basisdaten | `personalPokeData` | `poketool/personal/pl_personal.narc` | `0/0/2` | 508 / 508 | 44 B |
| Lernsets | `learnsets` | `poketool/personal/wotbl.narc` | `0/3/3` | 508 / 508 | 4..n, Vielfaches von 4 |
| Entwicklungen | `evolutions` | `poketool/personal/evo.narc` | `0/3/4` | 508 / 508 | 44 B |
| Attacken | `moveData` | `poketool/waza/pl_waza_tbl.narc` | `0/1/1` | 471 / 471 | 16 B |
| Gegenstände | `itemData` | `itemtool/itemdata/pl_item_data.narc` | `0/1/7` | 446 / 514 | 34 B |
| Trainer-Texttabelle | `trainerTextTable` | `poketool/trmsg/trtbl.narc` | `0/5/7` | 1 / 1 | 4 B je Eintrag |
| Trainer-Textoffsets | `trainerTextOffset` | `poketool/trmsg/trtblofs.narc` | `1/3/1` | 1 / 1 | u16 je Trainer |
| Item-Symbole | `itemIcons` | `itemtool/itemdata/item_icon.narc` | `0/1/8` | – | NCGR 560 B + NCLR im Wechsel, Member 0–1 NANR/NCER |

Pfade, die sich im Spielpaar unterscheiden: SoulSilver-Begegnungen `a/1/3/6`, Perl `p_enc_data.narc`, Perl-Personal `poketool/personal_pearl/personal.narc`.

**Layouts** (Codec-relevant): Pt-Begegnung 424 B: Rate u32, 12 Gras-Slots {u32 level (unteres Byte), u32 species (obere 16 Bit Rohwert)}, Ersatzlisten Tag/Nacht/Radar/Dual-Slot (je u32), Wasser-Slots {u8 **max**, u8 **min**, u16 pad, u32 species}, ungenutzter Block 0x0F8–0x123, Formraten Shellos/Gastrodon, Unown. HGSS 196 B: 6 × u8 Raten + 2 Pad, 12 × u8 Level, Morgens/Tags/Nachts je 12 × u16, Radio 2 + 2, Wasser {u8 **min**, u8 **max**, u16 species}, Fels 2 Slots, landSwarm, surfSwarm, nightFish, fishSwarm. Trainer-Eigenschaften 20 B (Flag-Byte: Bit 0 Attacken, Bit 1 Items; Klasse; 4 Items; KI u32; Kampftyp u32; Party-Zahl). Party-Eintrag Pt: u16 IV-Skala (Byte 1 = High-Byte), Level, Species (10 Bit) + Form (6 Bit), [Item], [4 Attacken], Ball-Kapsel: 8/10/16/18 B; HGSS: u8 difficulty, u8 Geschlecht/Fähigkeit (Bits 0–3 / 4–7), Rest wie Pt; DP ohne Ball-Kapsel 6/8/14/16 B. Lernset: u16-Einträge (Attacke Bits 0–8, Level 9–15) bis 0xFFFF, höchstens 20, danach Auffüllen mit 0x0000 auf ein Vielfaches von 4. Entwicklung 7 × {u16 Methode, u16 Parameter, u16 Ziel} + 2 Pad = 44 B. Attacke 16 B mit Flag-Bits 0x40/0x80, die uxie ablehnt. Gegenstand 34 B (DSPRE schreibt fälschlich 36).

**Namen** (Textarchiv-Nummern, Pt / HGSS; gleiche Werte in uxie `TextBankIds`): Species 412 / 237, Attacken 647 / 750, Gegenstände 392 / 222, Fähigkeiten 610 / 720, Typen 624 / 735, Trainernamen 618 / 729 (chatot liefert `{TRAINER_NAME:Kai}`), Klassen 619 / 730, Kampftexte 617 / 728. Nachrichtenzahlen: Species 496 (Personal hat 508: 494/495 Ei/Bad Egg, 496–507 Formen Deoxys, Wormadam, Giratina, Shaymin, Rotom), Attacken 468 (Daten 471), Gegenstände 468 / 537 (Daten 446 / 514), Trainernamen 928 / 738, Klassen 105 / 129, Kampftexte 2497 / 1717 (= Einträge in `trtbl`).

**Trainer-Nebentabellen:** `trtbl` ist nicht nach Trainer sortiert (Pt beginnt mit Trainer 247); Eintrag i ↔ Nachricht i im Kampftext-Archiv; `trtblofs` hat in Pt 928, in HGSS nur **735** Einträge bei 738 Trainern (wie das Spiel bei fehlendem Eintrag sucht: offen). Trainer-Flags HGSS ab `TRAINER_FLAG_BASE 0x550`, nächstes Flag 0x82E: Platz für **734** Trainer bei 738 Datensätzen (pokeheartgold `flags.h`); Pt: offen. Event ↔ Trainer: Overworld-Script 3000–4999 → Trainer = Script − 3000 + 1 (5000–6999 Partner), `trainer_type` im Objekt-Event, Sichtweite vermutlich `data[0]` (offen).

**arm9-Tabellen** (nur lesen): Item-Tabelle 8 B je Item-ID (Datenmember, Symbol, Palette, GBA-Item) bei Pt DE 0xF0D1C, HG DE 0x100148, HG US 0x100194; TM/HM-Liste 100 × u16 bei Pt DE 0xF0C54, HG DE 0x100080, HG US 0x1000CC. Item-IDs sind nicht Datenmember-Indizes; mehrere IDs teilen Member. Slot-Anteile der Begegnungen liegen im Overlay-Code (Pt overlay 6, HGSS overlay 2), nicht in den Dateien.

**uxie 0.9.7:** Encounter-Writer unbrauchbar (leere Slots gelten als ungültig, HGSS-Leser um 8 B versetzt, Schwarm doppelt geschrieben), Move-Codec lehnt 172 von 471 Vanilla-Attacken ab und schreibt 14 statt 16 B; Trainer-Party, Entwicklung, Lernset brauchen Auffüll-Regeln; Trainer-Eigenschaften, Personal, Items laufen byte-identisch. Keine Pfadliste für ds-rom-Projekte, nichts für Klassen, Item-Tabelle, TM-Liste, Kampftexte. uxie bleibt Test-Orakel.

**Codebasis** (Stand `main` vor 3c): `AssetKind` mit `ALL`, `label`, `slot` (`project.rs`), `archives.yaml` mit `FamilyArchives`, `Loaded`/`from_bytes`/`to_bytes` und `apply_edit` (`store.rs`), `append_member` (nicht rückgängig), `unwritable_reason` (Sperre), `write_atomic`; `Changeset` kennt genau ein Asset; der Build packt jeden Ordner `unpacked/<name>` aus `archives.kinds()`. Frontend ab 3c: `ui/primitives`, `PanelHead`, `Picker`, `Reference`, `goTo`, `history()`, `ProblemsStore`.

## 3. Entscheidungen im Überblick

| Frage | Entscheidung | Grund |
|---|---|---|
| Schnitt | 3b.3a Codecs + Pokédex/Attacken/Gegenstände, 3b.3b Trainer + Begegnungen + Gate | ein Plan wäre zu groß für eine Review-Runde; ein Schichtenschnitt lieferte keine bedienbare Software |
| Codecs | eigene Overlay-Codecs in `g4-formats/src/data/`, uxie nur Orakel | uxie parst strikt und sperrt Hacks; Encounter und Moves sind dort kaputt |
| Asset | Member bleibt Asset; Trainer als `Location::MemberPair`; mehrteiliger Changeset | die Party ist ohne die Eigenschaften nicht dekodierbar; Flag ↔ Party als Invariante im Asset |
| arm9 | lesen mit Plausibilitätsprüfung, nie schreiben | Patches verschieben Tabellen; „nicht verfügbar“ statt falscher Werte |
| Navigation | Unterbereich-Umschalter im Listenkopf + ⌘1–4 + Palette + Brotkrume | Atlas bleibt ein Eintrag je Bereich; Schau-Reiter tauschen nur die Schau |
| Tiefen | Bauen und Tüfteln; Erzählen-Screens erscheinen im Bauen-Layout | Erzählen kommt erst mit Rezepten (3d) |
| Sprites | Item-Symbole in 3b.3a, Pokémon-Menüsymbole bedingt, Kampfsprites später | Item-Tabelle liegt ohnehin vor; Kampfsprites sind verschlüsselt und beweisen nichts |
| Referenzen | `RefIndex` im Kernel, lazy, inkrementell mit Invariante `inkrementell == voll` | 471 Attacken mit „Verwendet in“ vertragen keinen Scan je Anfrage |
| Begegnungen | Slot-Tabelle je Methode, Anteile und „Ersetzt durch“ nur lesend, Warnungen statt Sperren | Anteile sind Code; leere Slots sind Vanilla |
| Trainer | „Neu“ als Dialog + Formular, Format abgeleitet, KI als Vorgaben + alle Bits, Name ≤ 7 | Assistent verdoppelt das Formular; DSPRE verliert Bits |
| Gate | Gegner-Art und -Stufe aus dem RAM, kalibriert als erster Task von 3b.3b | Screenshots werden nie automatisch rot |

## 4. Codecs (`crates/g4-formats/src/data/`)

`overlay.rs`: `FieldSpan { name, offset, len, kind }`, `Overlay` mit `get_u8/u16/u32`, `set_*`, `bits(off, mask)`; feste Längen als `struct X { raw: [u8; N] }` mit typisierten Gettern und Settern über die Feldkarte; `read(&[u8]) -> Result<X, FormatError>` prüft nur die Länge, `write()` gibt `raw` zurück. Der Round-Trip ist damit konstruktiv; ein proptest prüft, dass ein Setter nur seine Bytes ändert. Variable Längen (Party, Lernset) als `{ entries, tail }`: bleibt die Zahl gleich, bleibt `tail`; sonst Auffüllen (Party Nullen auf Vielfaches von 4, mindestens 8 B; Lernset nach 0xFFFF mit 0x0000 auf Vielfaches von 4).

Dateien: `encounter.rs` (`PtEncounters`, `HgssEncounters`, `Encounters`; `min()`/`max()` je Familie, die Oberfläche sieht nie die Byte-Reihenfolge), `trainer.rs` (`TrainerProps`, `PartyMon`, `Party` mit `PartyFormat { moves, items }`; Pt/HGSS/DP-Eintragstypen), `personal.rs`, `evolution.rs`, `learnset.rs`, `moves.rs`, `item.rs`, `trainer_text.rs` (`trtbl` {u16 trainer, u16 kind}, `trtblofs`), `arm9_tables.rs` (`ItemTable`, `MachineTable`), `ncgr.rs`, `nclr.rs` (4 bpp, RGB555 → RGBA, nach öffentlicher Formatbeschreibung). Unterschiede je Spiel sind eigene Typen, keine Laufzeit-`if`. Unbekannte Bits (Move-Flags 0x40/0x80, KI-Bits über 11, Species-Hochwort) bleiben Rohwerte und werden nie abgelehnt.

Tests: `tests/vanilla_data.rs` (gated über die drei Projekte, Soll-Member-Zahlen aus §2, `write(read(x)) == x` je Member, Differenz-Orakel gegen uxie für Personal, Items, Trainer-Eigenschaften, `min <= max` für alle 1078 belegten Pt-Wasserslots); proptests ohne Projekt (Overlay-Treue, Setter-Isolation, `read(write(d)) == d` für Party und Lernset); Abdeckungstest der Feldkarten. `docs/formats/<name>.md` je Format nach dem Muster von `narc.md`.

## 5. Metadaten (`crates/g4-kernel/metadata/`)

- `archives.yaml`: die elf Archivarten aus §2 mit `packed` je Familie; neuer optionaler Schlüssel `by_game: { IPG: "files/a/1/3/6" }` (Spielcode-Präfix → Pfad) für SoulSilver und Perl; `ArchiveMap::for_game(family, language, game_code)`; der Test `embedded_metadata_parses_and_resolves_every_family` bekommt Fälle je Spielcode.
- `text_banks.yaml`: die Banknummern aus §2 je Familie und Sprache (Western; JP abweichend, als `unverified`).
- `arm9_tables.yaml`: je Spielcode Offsets von Item-Tabelle und TM/HM-Liste (CPUD, IPKD, IPKE geprüft; andere Codes aus DSPRE mit `unverified: true`), `trainer_name_max: 7`, Trainer-Flag-Kapazität (HGSS 734; Pt aus Plan-Task 0).
- `encounter_odds.yaml`: Slot-Anteile je Familie und Methode (Gras 20/20/10/10/10/10/5/5/4/4/1/1; Surfen 60/30/5/4/1; Pt Alte Angel 60/30/5/4/1, Profi/Super 40/40/15/4/1; HGSS Angeln 40/30/15/10/5; Fels 80/20), nur Anzeige; Plan-Task 0 vergleicht mit pret.
- `encounter_rules.yaml`: Ersetzungsregeln (Pt: Schwarm → Gras 0, 1; Tag/Nacht → 2, 3; Radar → 4, 5, 10, 11; Dual-Slot → 8, 9. HGSS: landSwarm → 0, 1; Radio → 2, 3 und 4, 5; surfSwarm → Surfen 0; fishSwarm → Alte 2, Profi 0/2/3, Super alle; nightFish → Profi 3 und Super 1 nachts, Schwarm hat Vorrang).
- `species_forms.yaml`: Namen der Personal-Member 496–507.

## 6. Store und Assets (`crates/g4-kernel/src/`)

**Neue `AssetKind`s:** `Encounter`, `Personal`, `Learnset`, `Evolution`, `Move`, `Item` (editierbar, `Location::Member`); `TrainerProperties`, `TrainerParty` (Archivarten für Build und Zählung, nicht editierbar); `TrainerTextTable`, `TrainerTextOffsets` (je 1 Member, editierbar); `Trainer` (reine Store-Art, `Location::MemberPair { props, party }`, kanonische Bytes `u32 len(props) | props | party`; `read_member`, `accept_disk_as_base`, `save`, `read_location` bekommen den Fall; `save` schreibt beide Dateien mit `write_atomic`, die Stale-Prüfung vergleicht beide). `TrainerDoc { class, format: PartyFormat, flags_rest: u8, unknown: u8, items: [u16; 4], ai: u32, battle_type: u32, party: Vec<PartyMon> }`; Flag-Byte und Party-Zahl werden beim Schreiben aus `format` und `party.len()` abgeleitet. Sperre, wenn `encode(decode(p, q)) != (p, q)`, mit Grund für beide Dateien.

**Mehrteiliger Changeset:** `Changeset { parts: Vec<Part { asset, before, after }>, label }`, `Store::transaction(label, |tx| …)`; schlägt die Closure fehl, ändert sich nichts; Undo und Redo stellen alle Parts in umgekehrter Reihenfolge her; Dirty, Sperre, Stale und Save bleiben je Asset; `history()` zeigt ein Label je Schritt. Benutzt für Trainer + Name + Kampftexte, „Abspalten“ (Header + Begegnung), „Wie Staralili“ (TM-Bits + Lernset).

**Namen** (`data/names.rs`): Banken aus `text_banks.yaml`, gelesen über `Store::text_archive(bank)` (ungespeicherte Umbenennungen erscheinen sofort, `text_winner` gilt), `NameTables` je Bank mit Generationszähler des Text-Assets; `trainer_name::{unwrap, wrap}` für `{TRAINER_NAME:…}` mit Round-Trip-Test über alle Namen; Formnamen aus `species_forms.yaml`; gated Test der Nachrichtenzahlen aus §2.

**arm9-Tabellen** (`Project::item_table()`, `Project::machine_table()`, gecacht): Plausibilitätsprüfung beim Lesen (TM01 = 264, HM05 ∈ {432, 250}, jeder Item-Datenindex < Member-Zahl von `itemData`); schlägt sie fehl: „nicht verfügbar“, Gegenstände werden nach Datenmember gelistet („Daten 17“), das TM-Raster zeigt Nummern ohne Attacke. Gegenstände sind im Editor über die **Item-ID** geschlüsselt; geteilte Member stehen in der Schau („teilt Daten mit 21 Gegenständen“).

**Anlegen:** `create_trainer(class, name) -> u16` und `create_encounter_file(copy_of: Option<u16>) -> u16` folgen `append_member` (sofort geschrieben, nicht rückgängig, Dialog als Frage, Plakette „Nicht rückgängig“). `create_trainer` prüft vorher die Invarianten (Namenszahl == Trainerzahl, `trtblofs`-Länge, Flag-Kapazität) und verweigert mit Grund; es schreibt Eigenschaften (0 Pokémon), Party (8 Nullbytes), Namen an Index = ID, `trtblofs`-Eintrag auf das Ende von `trtbl`. HGSS lehnt Begegnungsdatei ≥ 255 ab: „HeartGold kann höchstens 255 Begegnungsdateien ansprechen“. DP: die Editoren sind gesperrt („Diamant/Perl: ohne Testprojekt nicht freigegeben“), Codecs und Metadaten kennen die Pfade.

## 7. Referenzindex (`data/refs.rs`)

```rust
pub enum Target { Species(u16), Move(u16), Item(u16), Trainer(u16), TrainerClass(u8), Encounter(u16) }
pub struct Ref { pub source: AssetId, pub field: &'static str, pub slot: Option<u16>, pub target: Target }
```

Quellen: HeaderTable (`wild_pokemon`, Pt 0xFFFF / HGSS 0xFF = keine) → Encounter; Encounter (alle Slots und Ersatzlisten) → Species; Trainer → Klasse, Items, Party-Species, Party-Items, Party-Moves; Objekt-Event (Script 3000–4999) → Trainer; Personal → Wild-Items; Lernset → Moves; Entwicklung → Species, Item oder Move je Methode. Lazy beim ersten `refs(target)` aus dem Store-Zustand (geladene Assets mit aktuellem Wert, alle anderen von der Platte); jede Store-Änderung meldet die betroffenen `AssetId`s, der Index berechnet nur deren Beiträge neu. Invariante `inkrementell == rebuild_full()` per proptest über zufällige Edit/Undo/Redo-Folgen, gated einmal über die drei Projekte; Ziel unter 300 ms für den ersten Aufbau (Messung im Plan). Scripts als Quelle fehlen bis 3b.4: die Schau sagt „ohne Scripts“, damit niemand einer 0 vertraut. In 3b.3 gibt es kein Löschen.

## 8. Befehle (Tauri und Brücke)

Jeder Befehl steht in `COMMANDS`, `dispatch.rs` und im Tauri-Handler (`commands_match_the_tauri_handler`, `every_command_accepts_the_keys_api_ts_sends`). Drei generische Datenbefehle statt zwanzig typisierter; typisiert bleibt es über `DataDoc` (serde-getaggt nach `kind`) und die TS-Union in `api.ts`.

| Befehl | Zweck | Plan |
|---|---|---|
| `data_list { kind, filter? }` | Zeilen für Listen: Index, Name, Kennung, Zusatz (Typen, Preis, Klasse, Orte) | a |
| `data_get { kind, index }` | `DataDoc` und Sperrgrund | a |
| `data_edit { kind, index, patch, label? }` | Teil-Patch je Art, sprechendes Label („Staravia: Angriff 75 → 85“) | a |
| `data_raw { kind, index }` | Bytes, `FieldSpan`-Karte, Pfad (Tüfteln) | a |
| `name_tables {}` | Species, Attacken, Items, Fähigkeiten, Typen, Klassen, Formen | a |
| `machine_table {}`, `item_table {}` | arm9 lesend oder „nicht verfügbar“ | a |
| `refs { target }` | `Ref`-Liste mit Ort und Navigationsziel | a |
| `get_item_icon { item }` | PNG-Bytes, im Frontend gecacht | a |
| `create_trainer { class, name }`, `set_trainer_texts { trainer, texts }` | ein Changeset | b |
| `create_encounter_file { copy_of? }`, `split_encounter_file { header }` | Anlegen, Abspalten | b |

`kind` ∈ `species | learnset | evolution | move | item | trainer | encounter`. `DataDoc::Species` fasst Personal, Lernset, Entwicklung und TM-Bits (Teil von Personal) für den Pokédex zusammen; Patches nennen das Teil-Asset.

## 9. Oberfläche

**Navigation:** `PanelHead` bekommt `switcher?: { items: {id, label, count}[], value, onChange }`: der Listentitel wird ein Menü-Knopf („Pokédex ▾“ mit Zählern). Pokémon = Pokédex · Trainer · Begegnungen · Attacken; Gegenstände = Gegenstände (Läden später). Dazu ⌘1–⌘4 im Bereich, Palette-Einträge („Trainer öffnen …“, „Attacke suchen …“), das zweite Brotkrumen-Glied öffnet dasselbe Menü. Unterbereich, Auswahl und Hand-Höhe je Unterbereich liegen in `shellState` (`g4.shell.v2`). Alles in Tiefe Bauen; Tüfteln ergänzt Rohdaten (`HexBlock` mit markiertem Feld, Dateiname, Member) und Dez/Hex. Die 3b.3-Listen (928 Trainer als Baum, 508 Arten, 471 Attacken, 514 Items flach): ein Plan-Task misst die Renderzeit; über 50 ms wird virtualisiert, sonst nicht.

| Entität | Liste | Schau | Hand | Brotkrume |
|---|---|---|---|---|
| Pokédex (a) | Suche (Name, Nummer), `FilterChips` Typ, `ListRow` Symbol · Name · Nr. | Kopfkarte (Name, `TypeBadge`×2, Fähigkeiten); `StatBar`×6 mit Schnitt aller Arten und „vorher“ = Stand der letzten Speicherung; Summe; „Fundorte“ und „Verwendet in“ als `Reference`-Chips (Begegnungen, Trainer) | `Tabs`: Werte (Basiswerte, EV-Ertrag, Fangrate, Wachstum, Geschlecht als `SelectField` fester Stufen, Ei-Gruppen, Schlüpfen) · Fähigkeiten und Typen · Lernset (`Table`, „12 von 20“) · Entwicklung (`Table` 7 Zeilen, Parameterfeld je Methode) · TM/VM (`ToggleGrid` 92 + 8, Tooltip „TM40 Aero-Ass“, „Wie …“ kopiert TM-Bits und Lernset einer anderen Art in einem Changeset) · Rohdaten (Tüfteln) | Pokémon › Pokédex › Staravia |
| Attacken (a) | Suche, Typ-Chips, `SelectField` Kategorie | Karte (Typ, Kategorie, Stärke, Genauigkeit, AP, Merkmale); Beschreibung als `DSScreen variant="textbox"`; „Verwendet in“ (N Lernsets, N Trainer) | Spalten Grundwerte · Verhalten (benannte Flag-Bits als `Checkbox`, unbekannte Bits nur in Tüfteln) · Rohdaten | Pokémon › Attacken › Flügelschlag |
| Gegenstände (a) | Suche, Taschen-Chips mit Zähler, Preis rechts, Symbol aus dem ROM | Karte; Beschreibung als `DSScreen textbox`; „Verwendet in“ (Trainer-Items, Party-Items, Wild-Items); Hinweis auf geteilte Daten | Spalten Werte (Preis, Taschen, Effekte, Funktionen) · Rohdaten (Party-Parameter) | Gegenstände › Medizin › Trank |
| Trainer (b) | `Segmented` Nach Ort · Nach Klasse; `TreeRow` Ort → Trainer; „Ohne Ort“; Level = höchstes im Team | Teamübersicht als Karte; „Steht in der Welt“: Event-Chips (Ort, Kachel) mit `goTo` Welt; Warnung „Ohne Pokémon stürzt der Kampf ab“ | Team (6 × `PokemonCard`, Klick öffnet Platz-Details: Art per `Picker`, Level, Item, 4 Attacken, Stärke, HGSS Geschlecht/Fähigkeit) · Trainer (Klasse, Name mit „5 von 7“, Schalter Attacken/Items, Doppelkampf, KI, 4 Items, Kampftexte) · Rohdaten | Pokémon › Trainer › Wanderer Kai |
| Begegnungen (b) | Dateien, beschriftet mit den Orten der Header, die sie nutzen; „Ohne Ort“ | „Wer hier wartet“: Arten mit Gesamtanteil je Tageszeit (aus den festen Anteilen); „Verwendet von N Orten“ mit „Für diesen Ort abspalten“; Warnliste | `Tabs` je Methode mit Zähler; `Table` Platz · Pokémon · Level (min–max) · Anteil · Ersetzt durch; Rate als `NumberField` („0 = nie“) | Pokémon › Begegnungen › Route 201 |

**Neue Primitives** (`ui/primitives/`, je mit Galerie-Seite): `TypeBadge` (Typfarben aus `tokens.ts` `typen`, Name), `StatBar` (Wert bis 255, Schnitt-Strich, `deviates`), `Tabs` (Base UI Tabs, Zähler und Ungespeichert-Punkt am Reiter), `Table` (Kopf, Zeilen, Zellen mit vorhandenen Feldern, Roving Tabindex, keine Virtualisierung), `FilterChips` (Mehrfachauswahl, Zähler, „n weitere“ als Popover), `ToggleGrid` (Raster aus `Checkbox` mit Tooltip), `HexBlock` (Bytes mit `FieldSpan`-Markierung), `Sprite` (PNG-Bytes mit Platzhalter und Cache), `PokemonCard` (3b.3b: Ruhe, Gewählt, Leer). Nicht gebaut: Slider, `RecipeButton`, Inline-Picker, `MetricCard`, `MapPeek`, weitere `DSScreen`-Varianten, `Budget` (Hinweistext „5 von 7 Zeichen“ in `status/warnung` ab der Grenze).

**Atlas:** Pokémon und Gegenstände werden in 3b.3a freigeschaltet (Geschichten bleibt gesperrt). Trainer, Begegnungen, Attacken liegen im Bereich Pokémon (Design-Bericht), Läden später unter Gegenstände.

## 10. Begegnungen (Fachlogik im Kernel, `data/encounter_rules.rs`)

Die Oberfläche rechnet nichts: Anteile, Ersetzungen und Warnungen kommen aus dem Kernel (`data_get` liefert je Slot `share`, `replaced_by`, `warnings`). Prüfungen sind Warnungen, nie Sperren, nur bei Rate > 0: Art belegt bei Level 0; min > max („das Spiel tauscht sie“); Level > 100; leerer Ersatz-Slot („Schwarm auf Gras 1 ergibt ein Glitch-Pokémon“). Sie erscheinen inline, in der Schau und im Prüflicht (`ProblemsStore`, Quelle „Begegnungen“). Leere Slots (Art 0, Level 0) sind gültig und gehen unverändert durch den Codec; die Zeile ist gedämpft. Level-Klemmung nur bei Eingabe: min über max zieht max mit und umgekehrt. HGSS Gras mit drei Art-Spalten (Morgens, Tags, Nachts) und gemeinsamer Level-Spalte; Methodenreiter Gras, Surfen, Fels, Alte, Profi, Super, Ersatz. Pt-Extras: Formraten Shellos/Gastrodon als `Switch` West/Ost (Rohwert bleibt bis zum Umschalten, „Ost“ schreibt 100), Unown als `SelectField`. Anbinden ist eine rückgängige Header-Änderung im Header-Editor (Feld „Begegnungen“ als `Reference` mit Picker; Pt u16 0xFFFF / HGSS u8 0xFF = keine). „Für diesen Ort abspalten“ (bei N > 1): Kopie anlegen (nicht rückgängig, der Dialog sagt es), dann im selben Changeset den Header umhängen.

## 11. Trainer

Schalter „Eigene Attacken“ und „Items tragen“ wirken auf alle Pokémon; Einschalten der Attacken füllt jedes Pokémon mit den letzten vier Lernset-Attacken bis zu seinem Level (Spielregel laut pret für Trainer ohne eigene Attacken; Plan-Task 0 prüft), Ausschalten verwirft sie; beides rückgängig, das Label sagt es. Byte 1 Pt: Feld „Stärke (IV-Skala)“ als u16, kein Geschlecht und keine Fähigkeit (High-Byte bleibt erhalten); HGSS: `difficulty` 0–255 („IV = Wert × 31 / 255“) und Geschlecht/Fähigkeit als zwei `SelectField`s (Auto / männlich / weiblich; Auto / 1 / 2). Form-Feld nur bei Arten mit Formen. KI in Bauen als `Segmented` einfach (0x1) · klug (0x7) · Experte (0xF) · eigene (aktiv, wenn der Wert keiner Vorgabe entspricht); Tüfteln zeigt alle 32 Bit als `Checkbox`-Liste, 11 benannt; `battle_type` in Bauen Einzel/Doppel (0/2), andere Bits bleiben. Klasse als `SelectField` aus der Klassenbank (unbekannt: „Klasse 130“); Klassennamen bearbeitet man im Bereich Texte; Geschlechter- und Preisgeld-Tabellen werden nicht gelesen. Name: `TextField`, harte Grenze aus `trainer_name_max` („Höchstens 7 Zeichen; längere Namen brauchen einen arm9-Patch“), geschrieben im selben Changeset in Archiv 618 / 729 an Index = ID. Kampftexte: Bauen zeigt die vorhandenen Arten (vor dem Kampf, Niederlage, danach) als Textfelder; neue Arten hängen Einträge an `trtbl` und Nachrichten ans Archiv 617 / 728; Invariante (gated): `len(trtbl)/4 == Nachrichten`, `trtblofs` ≥ Trainerzahl, sonst Kampftexte schreibgeschützt („Trainertexte passen nicht zur Tabelle“). „Neuer Trainer“: `+` in der Liste öffnet `Dialog` „Trainer 928 anlegen?“ mit Klasse und Name → `create_trainer`, danach ist der Trainer gewählt. „In der Welt platzieren“: wechselt in die Welt mit gespanntem Werkzeug Figur und ausstehender Trainerbindung; die nächste platzierte Figur bekommt `trainer_type = 1`, `script = 3000 + id − 1`, Sichtweite 3 (Feld aus Plan-Task 0). Der Event-Inspector bekommt ein Feld „Trainer“ (`Reference`, aus der Script-Nummer abgeleitet; Ändern setzt die Script-Nummer). Trainer-IDs über der Flag-Kapazität lehnt `create_trainer` ab (HGSS 734; die vier Vanilla-Datensätze darüber bleiben lesbar).

## 12. Sprites

`ncgr.rs`/`nclr.rs` dekodieren 4 bpp und RGB555; `get_item_icon(item)` liefert PNG-Bytes über Symbol- und Palettenindex aus der Item-Tabelle (`itemIcons`, Member 0–1 sind NANR/NCER), Frontend-Cache je Item-ID, `Sprite`-Primitive mit Platzhalter (Farbfläche mit Anfangsbuchstabe). Pokémon-Menüsymbole (`monIcons`: Member 0 NCLR, ab 7 NCGR 32 × 64) nur, wenn Plan-Task 0 die Paletten-Tabelle bestätigt (Kandidaten §1). Kampfsprites von Pokémon und Trainern: Platzhalter bis 3d. Der NSBTX-Dekoder aus 3b.1 hilft nur bei der Farbumrechnung.

## 13. Tests und Gate

- **Kernel:** `tests/store.rs` je Art (laden byte-identisch, Edit, Undo, Dirty, Save, gesperrtes Member); Trainer-proptest (nach jeder Edit-Folge gilt Flag = Format und Party-Zahl = Länge); mehrteiliger Changeset (Undo stellt alle Parts her, fehlschlagende Closure ändert nichts); `refs` inkrementell == voll; arm9-Plausibilität; Namenszahlen (gated). Die bestehenden `store.rs`-Tests bleiben unverändert grün (erstes Kriterium des Changeset-Umbaus).
- **Frontend:** vitest je Editor über Rollen und `data-*` (`data-species`, `data-move`, `data-item`, `data-trainer`, `data-slot`); Galerie-Seiten der neuen Primitives.
- **Gate 3b.3a:** §1; Szenario `pt-daten.yaml` mit uxie-Prüfschritt (`uv run g4emu check-narc …` oder ein Rust-Testbinary; der Plan wählt).
- **Gate 3b.3b**, Szenario `pt-trainer-begegnung.yaml` (und `hg-trainer-begegnung.yaml` für HeartGold DE): (1) neuer Ort per `create_area` aus einer Karte mit hohem Gras, neuer Header mit leerer Eventdatei (keine Story-Trigger); (2) `create_encounter_file`, `set_header` `wild_pokemon`, `data_edit encounter`: alle 12 Gras-Slots auf `S_WILD`/`L_WILD`, Rate 100; zusätzlich Route 201 Slot 0 geändert (uxie-Prüfung des Builds); (3) `create_trainer` × 2 (Wanderer „Jens“, Göre „Mia“) mit je einem Pokémon eindeutiger Art und Stufe, Texte; (4) `add_event` × 2 auf dem neuen Ort: `trainer_type = 1`, Script 3000 + id − 1, Blick quer zum Weg, Sichtweite 3; (5) im Zimmer eine Figur, deren Script einmalig ein Pokémon auf Level 100 gibt (3b.2-Mechanik; Befehlsname aus Plan-Task 0), Treppe auf den neuen Ort umgebogen; (6) `save`, `build_rom`. Spielen: `continue` → Figur ansprechen (`walk-to`, `press a`, `mash a 300`) → Treppe → `expect-map NEU` → `walk-until-battle right 6` → `expect-battle S_T1 L_T1` → `shot trainer1` → `win-battle`; Trainer 2 ebenso; `walk-until-battle up 2 --back-and-forth` → `expect-battle S_WILD L_WILD` → `shot wild`.
- **Neue `g4emu`-Schritte** (`steps.py`, `test_steps.py`): `expect-battle SPECIES LEVEL [--timeout]` (pollt den gegnerischen BattleMon), `walk-until-battle DIR N [--back-and-forth]` (toleriert die Unterbrechung durch die Sichtlinie), `win-battle [--timeout]` (A bis kein Kampf mehr aktiv und der Ort wieder lesbar), `mash BUTTON FRAMES`. `games.yaml` bekommt je Spielcode `battle: { pointer, offset, species_at, level_at }`, kalibriert mit `dump-ram`, `find-u16`, `pointers_to` über zwei Läufe (Gras ganz auf Art A bzw. B). Das ist **Task 1 von 3b.3b**, vor allen UI-Tasks. Fallback ohne stabilen Zeiger: Signatur mit drei Feldern (Art, Level, erste Attacke); zwei Felder reichen nicht (Fehltreffer in der geladenen Begegnungstabelle). Scheitert auch das, meldet der Plan das Gate-Kriterium als offen statt es umzudeuten.
- `docs/agent-gate.md` bekommt Rezepte je Editor; `docs/phase-3b3-gate.md` die Ergebnistabelle.

## 14. Risiken und Festlegungen

1. **Nebentabellen neuer Trainer** (`trtblofs`, Flag-Kapazität, Namensbank): Absturz oder verdorbener Spielstand. Plan-Task 0 prüft die Invarianten an Vanilla; `create_trainer` verweigert, wenn eine nicht hält; das Gate legt zwei Trainer an und kämpft gegen beide.
2. **Gate nicht beweisbar** (RAM, Story-Sperren, kein Pokémon im Spielstand): Kalibrierung zuerst; neuer Ort ohne Trigger; Pokémon per Script; Signatur-Fallback; offen benennen.
3. **Store-Umbau** bricht Undo oder Save bestehender Arten: alte Tests unverändert grün; proptest Undo/Redo-Inverse über gemischte Arten; der Changeset-Umbau ist ein eigener Task vor jeder neuen Art.
4. **Umfangsdrift** durch die Screens: die „Später“-Tabelle gilt wörtlich; eine neue Komponente kommt nur mit einem Editor, der sie im selben Plan benutzt.
5. **Falsche Offsets in Hack-Projekten**: Plausibilitätsprüfung mit „nicht verfügbar“; Overlay-Codecs erhalten jedes Bit; abweichende Member-Länge → gesperrt mit Grund; DP gesperrt.
6. **Sprite-Formate** kosten mehr als gedacht: nur Item-Symbole sind Gate-relevant (Liste); Pokémon-Symbole sind bedingt, Kampfsprites später.
7. **Keine Nintendo-Assets:** alle Grafiken kommen zur Laufzeit aus dem geöffneten Projekt; Tests nutzen synthetische NCGR/NCLR.

## 15. Offene Prüfungen (Plan-Task 0 von 3b.3a bzw. 3b.3b)

`trtbl`-Eintrag → Nachricht für HGSS (Archiv 728 passt zur Zahl; Zuordnung bestätigen); Startindex der Trainer-Flags in Pt und damit die höchste zulässige Trainer-ID; Sichtweite-Feld im Objekt-Event (`data[0]`?); Name des „Pokémon geben“-Befehls in der rotom-Datenbank; ob Rate 100 je Schritt eine Begegnung garantiert; Slot-Anteile gegen pret (`pokeplatinum` overlay006, `pokeheartgold` Wild-Encounter-Code); Pokémon-Symbol-Paletten-Tabelle; Default-Attacken-Regel für Trainer ohne eigene Attacken; Renderzeit der Listen und Aufbauzeit des Index; ob ein Gras-Verhalten auf einer kopierten Karte in einem neuen Header Begegnungen auslöst.

## 16. Dokumentation und Roadmap

- `docs/ROADMAP.md`: 3b.3 bekommt den Schnitt 3b.3a / 3b.3b, die „Später“-Tabelle wird als „Aus dem Design-System, später“ ergänzt (3d Erzählen, Overlay-Tabellen), der Hinweis auf den eigenen Encounter-Writer ist damit entschieden (uxie-Writer unbrauchbar).
- `docs/formats/`: ein Dokument je Codec (§4) mit Byte-Layout und Quelle.
- `docs/dev-setup.md`: neue gated Tests und das uxie-Orakel; `docs/agent-gate.md`: Rezepte; `design-system/README.md`: „Was die App umsetzt“ um 3b.3 ergänzt.
