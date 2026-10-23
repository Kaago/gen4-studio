# Vorschlag C: Phase 3b.3 „Daten-Editoren“

Architekt C. Neigung: erst Datenmodell und Prüfbarkeit, Referenzen und Invarianten sauber, YAGNI bei der Oberfläche, das Gate hart beweisbar. Grundlage: die drei Berichte in diesem Ordner, Roadmap §3b.3/§3b.4, Spec 3c, Spec 3b.2, Code auf `main` 11b335e (`store.rs`, `project.rs`, `archives.yaml`, `bridge/dispatch.rs`, `scripts/g4emu`). Nichts im Repo geändert.

Kennzeichnung: **[geprüft]** steht so in einem Bericht, an den Projekten gemessen; **[Code]** aus Quelltext gelesen; **[offen]** von mir nicht geprüft, ein Plan-Task klärt es.

---

## 0. Leitlinien (gelten für alle Antworten)

1. **Ein Byte, eine Stelle.** Jeder Codec legt eine Feldkarte (`FieldSpan { name, offset, len, kind }`) über die Rohbytes. Sie deckt jedes Byte genau einmal ab, als Feld oder als `reserved`. Daraus folgen der Writer (Overlay: Rohbytes kopieren, nur bekannte Felder hineinschreiben), die Hex-Markierung in Tüfteln und ein Abdeckungstest.
2. **Round-Trip als Eigenschaft, nicht als Stichprobe.** Für Formate fester Länge gilt `write(read(b)) == b` für *beliebige* Bytes der richtigen Länge (proptest), zusätzlich gemessen über alle Vanilla-Member der drei Projekte (gated).
3. **Eine Quelle der Wahrheit je Invariante.** Gehört eine Invariante zu zwei Dateien (Trainer-Flags ↔ Party-Format, `party_count` ↔ Party-Länge), gehören beide Dateien zu *einem* Asset.
4. **Nie still ändern.** Werte außerhalb der Vanilla-Grenzen bleiben beim Laden erhalten; Felder lehnen nur *neue* ungültige Eingaben ab. Unbekannte IDs erscheinen als „(unbekannt #n)“.
5. **Diamant/Perl ist ungeprüft.** Alle neuen Datenarten laden dort gesperrt („Diamant/Perl ungeprüft“). Code und Pfade für DP bleiben; freigeschaltet wird erst mit einem DP-Projekt.

---

## 1. Umfang und Schnitt

**Entscheidung: zwei Pläne, beide mit eigenem Gate.**

**3b.3a „Codecs und Pokémon-Daten“** (nur Tabellen, keine Welt):
- Kernel: Codecs in `g4-formats` für *alle* Familien aus §2, auch Trainer und Begegnungen, damit der gated Round-Trip von Anfang an vollständig ist. Dazu neue Archivarten in `archives.yaml` und im Store, Namenstabellen, arm9-Tabellen lesend (Item-Tabelle, TM/VM-Liste), Referenzindex (Quellen Lernset, Entwicklung, Personal, Party, Trainer, Begegnung, Header), mehrteiliger Changeset.
- Editoren: **Pokédex** (Reiter Werte, Fähigkeiten und Zucht, Lernset, Entwicklung, TM/VM), **Attacken**, **Gegenstände**. Atlas: Pokémon und Gegenstände freigeschaltet.
- Primitives: `TypeBadge`, `StatBar`, `Tabs`, `Table`, `FilterChips`, `ToggleGrid`, `HexBlock`. Gate 3b.3a siehe §9.1.

**3b.3b „Trainer, Begegnungen, Gate“** (Daten, die in der Welt wirken): Trainer als zusammengesetztes Asset mit Trainertext-Tabelle und Name; „Neuer Trainer“; Event ↔ Trainer; Begegnungen mit neuer Datei, Anbindung am Header und „Abspalten“; Item-Symbole aus dem ROM; `PokemonCard`; `g4emu`-Kampfschritte und zwei Szenarien. Gate = Roadmap-Gate 3b.3.

**Später:**

| Thema | Wohin | Grund |
|---|---|---|
| Ei-Attacken | 3d oder später | Pt im Overlay 5, HGSS als NARC: zwei Mechaniken |
| Attacken-Lehrer, Läden (Sortiment, Orden-Staffel) | später | Overlay-Tabellen, je Spiel verschieden; Patch-Layer |
| Assistent „Neuer Trainer“ mit Live-Vorschau | 3d | Erzählen und Live-Vorschau sind in 3c ausgeschlossen |
| Kampfvorschau, Pokédex-Profil, Laden als DS-Bildschirm | 3d | reine Schau-Kosmetik, für keinen Beweis nötig |
| Kampfsprites von Pokémon und Trainern | 3d | verschlüsselte NCGR, Pt und HGSS verschieden **[offen]** |
| Pokémon-Menüsymbole | 3b.3b, wenn Plan-Task 0 die Paletten-Tabelle findet, sonst 3d | Palettenindex je Art steht in einer Code-Tabelle **[offen]** |
| arm9-Tabellen schreiben (Item-Tabelle, TM-Liste) | später | selten gebraucht; Schreiben in arm9 braucht Schutz gegen Patches |
| Klassen-Geschlecht, Preisfaktor, Eye-Music | später | Offsets nur aus DSPRE **[offen]**; Belohnung ist eine Klassen-Eigenschaft |
| Projektprüfungen (Trainer ohne Party, leere Ersatz-Slots projektweit) | 3b.4 | Roadmap |
| Script → Trainer/Item (Rückwärtsreferenz über Scripts) | 3b.4 | braucht Auswertung der Befehlsparameter über rotom |
| Pokédex-Größe/-Gewicht, Wachstumstabellen, „Auf Original zurücksetzen“ | später / Phase 4 | Zukan-Daten; Basis-ROM-Vergleich |

Verworfene Alternativen:
- **(a) Ein Plan für alles.** Kernel-Umbau, sieben Codecs, fünf Editoren und ein Emulator-Gate sind zu groß für eine Review-Runde; ein früher Fehler im Store-Modell würde erst am Ende sichtbar.
- **(b) Schnitt nach Schicht** (3b.3a nur Kernel, 3b.3b nur UI). Dann liefert 3b.3a keine bedienbare Software; das widerspricht der Roadmap-Regel.

Preis, falls falsch: Brauchen Trainer und Begegnungen das Store-Modell aus 3b.3a doch anders, bleibt der Codec-Teil gültig. Neu zu machen wären nur der mehrteilige Changeset und `Location::MemberPair` (etwa zwei Tasks).

---

## 2. Codecs

**Entscheidung: eigene Codecs für alle Familien in `crates/g4-formats/src/data/`. uxie dient nur als Test-Orakel.**

**Dateien:** `overlay.rs` (`FieldSpan`, `Overlay` mit `get_u8/u16/u32`, `set_*`, `bits(off, mask)`); `encounter.rs` (`PtEncounters` 424 B, `HgssEncounters` 196 B, Enum `Encounters`); `trainer.rs` (`TrainerProps` 20 B, `PartyMon`, `Party`); `personal.rs` (44 B), `evolution.rs` (44 B), `learnset.rs`, `moves.rs` (16 B), `item.rs` (34 B); `trainer_text.rs` (`trtbl` mit Einträgen `{u16 trainer, u16 kind}`, `trtblofs` mit `u16` je Trainer); `arm9_tables.rs` (`ItemTable` 8 B je Item, `MachineTable` 100 × u16).

**Muster für feste Längen:** `struct X { raw: [u8; N] }` mit typisierten Gettern und Settern über die Feldkarte. `read(&[u8]) -> Result<X, FormatError>` prüft nur die Länge; `write() -> Vec<u8>` gibt `raw` zurück. Der Round-Trip ist damit konstruktiv. Fehler können nur Setter machen, die Nachbarbits verletzen; das prüft ein proptest (Setter auf Feld A, alle Bytes außerhalb von A unverändert).

**Muster für variable Längen** (Party, Lernset): `{ entries: Vec<…>, tail: Vec<u8> }`, wobei `tail` die Bytes nach dem Inhalt sind. Bleibt die Eintragszahl gleich, wird `tail` übernommen. Ändert sie sich, entsteht der Tail neu: Party auf ein Vielfaches von 4 B mit Nullen, mindestens 8 B; Lernset nach `0xFFFF` auf ein Vielfaches von 4 B mit `0x0000`. Diese Regel ist an allen Vanilla-Membern geprüft **[geprüft, uxie-report §1.2, §1.5]**.

**Unterschiede je Spiel**, im Codec als eigene Typen, nicht als Laufzeit-`if`:

| Format | Pt | HGSS | DP |
|---|---|---|---|
| Begegnung | 424 B; 12 × {u32 level, u32 species} (Level im unteren Byte, 3 Pad-Bytes); Wasser {u8 **max**, u8 **min**, u16 pad, u32 species}; Block 0xF8–0x123 ungenutzt; Formraten, Unown, 5 Dual-Slot-Paare | 196 B; 6 × u8 Raten + 2 Pad; 12 × u8 Level gemeinsam; Morgens/Tags/Nachts je 12 × u16; Radio 2 + 2; Wasser {u8 **min**, u8 **max**, u16 species}; Fels 2 Slots; landSwarm, surfSwarm, **nightFish**, fishSwarm | wie Pt laut DSPRE **[offen]** |
| Species-Wort in der Begegnung | u32; obere 16 Bit bleiben als Rohwert (Formbits in Hacks) | u16 | u32 |
| Party-Eintrag | `u16 ivScale` (Byte 1 = High-Byte), level, species (10 Bit) + Form (6 Bit), [item], [4 moves], ballCapsule; 8/10/16/18 B | `u8 difficulty`, `u8 genderAbility` (Bits 0–3 Geschlecht, 4–7 Fähigkeit), Rest wie Pt | ohne ballCapsule: 6/8/14/16 B |
| Trainer-Props, Personal, Moves, Items, Evolution, Lernset | gleich | gleich | gleich laut Berichten **[offen für DP]** |

Wasser min/max: Der Codec bietet `min()` und `max()` an und weiß je Familie, welches Byte welches ist; die Oberfläche sieht nie die Byte-Reihenfolge. Ein Test liest vanilla Pt und prüft `min <= max` für alle 1078 belegten Slots (Bericht: „1078× byte0>byte1“).

**Beweis** (gated, `crates/g4-formats/tests/vanilla_data.rs`):
- Über `G4_TEST_PROJECTS` (Platin DE, HeartGold DE, HeartGold US) liest der Test jedes Member jeder der zehn Archivarten aus `unpacked/<ordner>/` und prüft `write(read(x)) == x`.
- Erwartete Member-Zahlen (Pt / HGSS): encounters 183/142; trainerProperties und trainerParty je 928/738; personalPokeData, learnsets, evolutions je 508/508; moveData 471/471; itemData 446/514; trtbl und trtblofs je 1/1. Abweichende Zahlen lassen den Test fehlschlagen, damit er nicht leer durchläuft (genau daran sind die uxie-Tests gescheitert).
- **Differenz-Orakel:** Für Personal, Items und Trainer-Props (uxie dort 100 % [geprüft]) vergleicht der Test unsere Felder Member für Member mit uxies Feldern. Zwei unabhängige Leser, die übereinstimmen, beweisen die Feldkarte, nicht nur den Round-Trip.
- **Ohne Projekt:** proptests (Overlay-Treue, Setter-Isolation, Party-/Lernset-Dokumente `read(write(d)) == d`) und ein Abdeckungstest der Feldkarten.
- `docs/formats/<name>.md` je Format, nach dem Muster von `narc.md`.

Verworfene Alternativen:
- **(a) uxie-Typen plus Nachbearbeitung** (Padding-Wrapper, gepatchter Move- und Encounter-Codec). Billiger für Personal und Items. Aber: uxie parst strikt (unbekannte Flag- oder AI-Bits sind ein Fehler) und sperrt damit Hacks; die Item-Party-Typen sind außerhalb von uxie nicht benennbar; zwei Codec-Stile nebeneinander.
- **(b) uxie forken.** Ein Fork muss gepflegt werden; rotom hängt an uxie 0.9.5, und ein Fork bricht die Versionsvereinheitlichung durch Cargo.

Preis, falls falsch: Etwa 1 500 Zeilen Codec und Test mehr als nötig. Überschaubar, weil die Formate einfache feste Layouts sind; das Orakel deckt Fehler in der Feldkarte sofort auf.

---

## 3. Asset-Modell im Store

**Entscheidung: Ein Asset ist ein NARC-Member. Ausnahme Trainer: ein zusammengesetztes Asset aus Eigenschaften und Party. Dazu ein mehrteiliger Changeset, der in einem Undo-Schritt mehrere Assets ändert.**

**Neue `AssetKind`s** je Archivart, mit der heutigen Member-Location:
- `Encounter`, `Personal`, `Learnset`, `Evolution`, `Move`, `Item`.
- `TrainerProperties`, `TrainerParty`: nur Archivarten für den Build, nicht editierbar.
- `TrainerTextTable`, `TrainerTextOffsets`: je 1 Member, editierbar.

Zusätzlich `AssetKind::Trainer`, eine reine Store-Art (Vorbild HeaderTable/DynamicHeaders) mit `Location::MemberPair { props, party }`. Kanonische Bytes: `u32 len(props) | props | party`. Jedes `match` auf `Location` bekommt den Fall dazu: `read_member`, `accept_disk_as_base`, `save`, `read_location`. `save` schreibt beide Dateien mit `write_atomic`; die Stale-Prüfung vergleicht beide.

**Was das Trainer-Asset garantiert:**
- Dokument: `TrainerDoc { class, format: PartyFormat { moves, items }, flags_rest: u8, unknown: u8, items: [u16; 4], ai: u32, battle_type: u32, party: Vec<PartyMon> }`.
- Flag-Byte und `party_count` werden beim Schreiben aus `format` und `party.len()` *abgeleitet*. Die DSPRE-Fehler D#115 und D#182 (Moves verloren, falsche Checkbox) sind damit unmöglich, nicht nur getestet.
- Lock: Das Paar ist gesperrt, wenn `encode(decode(p, q)) != (p, q)`, mit einem Grund für beide Dateien.

**Mehrteiliger Changeset** (`store.rs`):
- `Changeset { parts: Vec<Part { asset, before, after }>, label }`; `Store::transaction(label, |tx| …)` sammelt die Parts. Schlägt die Closure fehl, wird alles zurückgesetzt. Undo und Redo stellen alle Parts in umgekehrter Reihenfolge her.
- Dirty, Lock, Stale und Save bleiben je Asset. Damit gilt „Undo je Asset“ weiter: gespeichert werden Byte-Snapshots je Asset, nur der Verlauf fasst sie zu einem Schritt zusammen. `history()` aus 3c zeigt ein Label je Schritt.
- Benutzt für: Trainer + Name (Textarchiv) + Kampftexte (Textarchiv, `trtbl`); „Abspalten“ (Header-Feld + Begegnung); Lernset + Personal beim Kopieren „Wie Staralili“.

**Anlegen:** `create_trainer` und `create_encounter_file` folgen dem heutigen `append_member`-Muster: sofort geschrieben, nicht rückgängig, Plakette „Nicht rückgängig“, Bestätigungsdialog als Frage. Spätere Änderungen sind normal rückgängig.

**Namen** (`crates/g4-kernel/src/data/names.rs`):
- Bank-IDs aus `uxie::TextBankIds::for_game` (der Kernel nutzt das schon für Ortsnamen). Gelesen über `Store::text_archive(bank)`, nicht direkt vom Member: ungespeicherte Umbenennungen erscheinen sofort, die neuere JSON gewinnt (`text_winner`).
- Cache `NameTables` je Bank, invalidiert über einen Generationszähler des Text-Assets (steigt bei Änderung, Undo und Reload).
- Gated Test der Message-Zahlen (Pt / HGSS): Species 496/496, Moves 468/468, Items 468/537, Trainernamen 928/738, Klassen 105/129. Formnamen für die Personal-Member 496–507 aus `metadata/species_forms.yaml` (Deoxys Angriff …).
- Trainernamen kommen von chatot als `{TRAINER_NAME:Kai}`; `names::trainer_name::{unwrap, wrap}` mit Round-Trip-Test über alle 928 und 738 Namen.

**arm9-Tabellen:**
- `metadata/arm9_tables.yaml` je Spielcode (`CPUD`, `IPKD`, `IPKE` geprüft; DSPRE-Werte für weitere Codes mit `unverified: true`), gelesen über `Project::item_table()` und `Project::machine_table()`, gecacht. **Nur lesen.**
- Plausibilitätsprüfung beim Lesen: TM01 = 264, HM05 ∈ {432, 250}, jeder Item-Datenindex kleiner als die Member-Zahl von `itemData`. Schlägt sie fehl (anderer Spielcode, arm9-Patch), sind die Tabellen „nicht verfügbar“: Gegenstände werden nach Datenmember gelistet („Daten 17“), das TM-Raster zeigt Nummern ohne Attacke.
- Gegenstände sind im Editor über die **Item-ID** geschlüsselt. Teilen mehrere IDs ein Datenmember, zeigt die Schau „teilt Daten mit 21 Gegenständen“.

**Pfade, die sich im Spielpaar unterscheiden:** `archives.yaml` bekommt den optionalen Schlüssel `by_game: { IPG: "files/a/1/3/6" }` (Präfix des Spielcodes → `packed`); `ArchiveMap::for_game(family, language, game_code)`. Fälle: SoulSilver-Begegnungen `a/1/3/6`; Perl-Begegnungen `p_enc_data.narc`; Perl-Personal `poketool/personal_pearl/personal.narc`. Der Test `embedded_metadata_parses_and_resolves_every_family` bekommt Fälle mit Spielcode, ohne Projekt.

Verworfene Alternativen:
- **(a) Tabellen-Asset je Archiv** (alle 928 Trainer in einem Asset wie HeaderTable). Jede Änderung kopiert das ganze Archiv in den Changeset; Dirty und Stale gelten nur noch fürs ganze Archiv; die Assets passen nicht mehr zu DSPREs Dateien.
- **(b) Trainer als zwei getrennte Assets mit Gruppen-Undo.** Die Party lässt sich ohne die Props nicht dekodieren, `Loaded::from_bytes` bräuchte also Wissen über ein anderes Asset. Die Invariante Flags ↔ Party wäre über zwei Assets verteilt, der Lock müsste außerhalb des Assets entschieden werden.

Preis, falls falsch: `MemberPair` berührt fünf `match`-Stellen; der Rückbau zu (b) wäre ein Task. Der mehrteilige Changeset bliebe nützlich.

---

## 4. Navigation im Doppeldeck

**Entscheidung: Unterbereiche wechselt man über einen Umschalter im Listenkopf.** `PanelHead` bekommt ein optionales `switcher`: Titel als `Menu`-Knopf „Pokédex ▾“ mit Zählern. Dazu Palette-Einträge („Trainer öffnen“, „Attacke suchen …“); das zweite Brotkrumen-Glied öffnet dasselbe Menü.

- Unterbereiche: Pokémon = Pokédex · Trainer · Begegnungen · Attacken; Gegenstände = Gegenstände (Läden später als zweiter Eintrag).
- Alles in Tiefe **Bauen**; Tüfteln fügt Rohdaten hinzu. Erzählen wird nicht gebaut; Begegnungen und Läden aus den Erzählen-Screens erscheinen im Bauen-Layout.

| Entität | Liste | Schau | Hand | Brotkrume |
|---|---|---|---|---|
| Pokédex | Suche (Name, Nummer, Kennung), `FilterChips` Typ, Zeilen Name · `SPECIES_*` · Nr. | Kopfkarte (Name, `TypeBadge`×2, Fähigkeiten); `StatBar`×6 mit Schnitt aller Arten; Summe mit „vorher“ = Stand der letzten Speicherung; „Fundorte“ als `Reference`-Chips (Begegnungen, Trainer) | `Tabs`: Werte · Fähigkeiten und Zucht · Lernset (`Table`) · Entwicklung (`Table`, 7 Zeilen, Parameterfeld je Methode) · TM/VM (`ToggleGrid` 92 + 8, Tooltip mit Attackenname) · Rohdaten (nur Tüfteln) | Pokémon › Pokédex › Staravia |
| Attacken | Suche, Typ-Chips, `SelectField` Kategorie | Karte (Typ, Kategorie, Stärke/Genauigkeit/AP, Merkmale); Beschreibung als `DSScreen variant="textbox"`; „Verwendet in“ (N Lernsets, N Trainer) | Spalten Grundwerte · Verhalten (8 Flag-Bits benannt, `Checkbox`) · Tüfteln (`HexBlock`) | Pokémon › Attacken › Flügelschlag |
| Gegenstände | Suche, Taschen-Chips mit Zähler, Preis rechts | Karte; Beschreibung als `DSScreen textbox`; „Verwendet in“ (Trainer-Items, Party-Items, Wild-Items in Personal); Hinweis auf geteilte Daten | Spalten Werte (Preis, Taschen, Effekte, Funktionen) · Tüfteln (Party-Parameter roh) | Gegenstände › Medizin › Trank |
| Trainer | `Segmented` Nach Ort · Nach Klasse; `TreeRow` Ort → Trainer; Gruppe „Ohne Ort“; Level = höchstes im Team | Teamübersicht als Karte; „Steht in der Welt“: Event-Chips (Ort, Kachel) mit „In der Welt öffnen“ (`goTo` Welt, Event gewählt); Warnung „Ohne Pokémon stürzt der Kampf ab“ | Team (6 × `PokemonCard`) · Platz-Detail · Trainer (Klasse, Name, Format-Schalter, Doppelkampf, KI, Items, Kampftexte) · Tüfteln | Pokémon › Trainer › Wanderer Kai |
| Begegnungen | Dateien, beschriftet mit den Orten der Header, die sie nutzen (wie DSPRE); „Ohne Ort“ | „Wer hier wartet“: Arten mit Gesamtanteil je Tageszeit (aus festen Slot-Anteilen); „Verwendet von N Orten“ mit „Für diesen Ort abspalten“; Warnliste | `Tabs` je Methode mit Zähler; `Table` Platz · Pokémon · Level (min–max) · Anteil · Ersetzt durch; Rate als `NumberField` | Pokémon › Begegnungen › Route 201 |

**Primitives:**

| Baustein | Entscheidung |
|---|---|
| `TypeBadge` | neu: Typfarben aus `tokens.ts` `typen`, Name |
| `StatBar` | neu: Wert, Schnitt-Strich, `deviates` |
| `Tabs` | neu, Base UI Tabs; Zähler und Ungespeichert-Punkt am Reiter |
| `Table` | neu, minimal: Kopf, Zeilen, Zellen mit vorhandenen Feldern, Roving Tabindex; keine Virtualisierung (höchstens 20 Zeilen) |
| `FilterChips` | neu: Mehrfachauswahl, Zähler, „n weitere“ |
| `ToggleGrid` | neu: Raster aus `Checkbox` für TM/VM, später für die Läden-Matrix |
| `HexBlock` | neu: Bytes mit `FieldSpan`-Markierung; die Feldkarte kommt vom Kernel |
| `PokemonCard` | neu in 3b.3b: Ruhe, Gewählt, Leer |
| nicht gebaut | Slider (`NumberField` für die Rate), `RecipeButton` und Inline-Picker (vorhandener `Picker` mit `preview` und Hinweiszeile je Treffer), neue DSScreen-Varianten (`textbox` reicht), Budget (Hinweistext „5 von 7 Zeichen“) |

Listen: 928 Trainer als Baum, 471 Attacken flach. Ein Plan-Task misst die Renderzeit; über 50 ms wird die Liste virtualisiert, sonst nicht.

Verworfene Alternativen:
- **(a) Unterpunkte im Atlas.** Bricht das 88-px-Raster und die Regel „ein Eintrag = ein Bereich“ aus 3c §5.3.
- **(b) Unterbereiche als Dockview-Reiter in der Schau.** Ein Wechsel muss Liste *und* Hand tauschen; Reiter in der Schau tauschen nur die Schau.

Preis, falls falsch: Der Umschalter ist eine Prop an `PanelHead` plus ein Menü; ihn zu ersetzen ist lokal.

---

## 5. Sprites aus dem ROM

**Entscheidung: 3b.3b liefert Item-Symbole, sonst Platzhalter.**
- **Item-Symbole:** `itemIcons`-NARC (Pt `itemtool/itemdata/item_icon.narc`, HGSS `a/0/1/8`), NCGR 4bpp und NCLR RGB555. Die Member-Indizes für Symbol und Palette stehen in der Item-Tabelle, die wir ohnehin lesen. Dekoder neu in `g4-formats/src/ncgr.rs` und `nclr.rs`, nach öffentlicher Formatbeschreibung, nicht aus DSPRE. Ausgabe als PNG-Bytes über `get_item_icon(item)`, im Frontend gecacht.
- **Pokémon-Menüsymbole:** nur, wenn Plan-Task 0 die Paletten-Index-Tabelle je Art findet (pret-Quelle, Offset je Spielcode, Plausibilitätsprüfung wie bei arm9) **[offen]**.
- **Kampfsprites von Pokémon und Trainern:** Platzhalter (`TypeBadge`-Farbfläche mit Kennbuchstaben). Gründe: Verschlüsselung (DP/Pt rückwärts, HGSS vorwärts **[offen]**), Animation, Paletten.
- Der vorhandene NSBTX-Dekoder gilt für Texturen, nicht für NCGR; er hilft nur bei der Farbumrechnung.

Verworfene Alternativen:
- **(a) Alle Sprites in 3b.3.** Etwa drei Tasks Dekoder, Verschlüsselung und Prüfung je Spiel, für kein Gate-Kriterium.
- **(b) Gar keine Sprites.** Item-Symbole sind fast gratis (die Tabelle ist schon da), und ihr Dekoder erschließt das Format, das alle späteren Sprites brauchen.

Preis, falls falsch: Die Listen sehen bis 3d karger aus als die Screens; funktional fehlt nichts.

---

## 6. Referenzen

**Entscheidung: `crates/g4-kernel/src/data/refs.rs`, ein Index aus Beiträgen je Quell-Asset.**

```rust
pub enum Target { Species(u16), Move(u16), Item(u16), Trainer(u16), TrainerClass(u8), Encounter(u16) }
pub struct Ref { pub source: AssetId, pub field: &'static str, pub slot: Option<u16>, pub target: Target }
struct RefIndex { by_source: HashMap<AssetId, Vec<Ref>>, by_target: HashMap<Target, Vec<Ref>> }
```

**Quellen in 3b.3:**
- HeaderTable (`wild_pokemon`; 0xFFFF in Pt, 0xFF in HGSS = keine) → Encounter.
- Encounter (alle Slots und Ersatzlisten) → Species.
- Trainer → Klasse, Items, Party-Species, Party-Items, Party-Moves.
- Event (Overworld): Script 3000–4999 → Trainer = script − 3000 + 1; Script 5000–6999 → Partner = script − 5000 + 1.
- Personal → Wild-Items; Lernset → Moves; Entwicklung → Species, Item oder Move je Methode.

**Aufbau und Aktualisierung:**
- Lazy beim ersten `refs(target)`. Gelesen wird *der Store-Zustand*: geladene Assets mit ihrem aktuellen Wert, alle anderen von der Platte. Jede Store-Änderung (Edit, Undo, Redo, Reload, Accept, Anlegen) meldet die betroffenen `AssetId`s; der Index berechnet nur deren Beiträge neu (`by_source` ersetzen, `by_target` nachziehen).
- Kosten: etwa 3 000 kleine Dateien beim ersten Aufbau **[offen: Messung im Plan, Ziel unter 300 ms]**.

**Invariante und Beweis:** Nach jeder Folge von Änderungen gilt `incremental == rebuild_full()`. proptest über zufällige Edit-, Undo- und Redo-Folgen an einer kleinen Testprojekt-Kopie, gated einmal über die drei Projekte.

**Was 3b.3 zeigt:** „Verwendet in N“ in Schau und Liste, als `Reference`-Chips mit `goTo`. Script-Treffer fehlen noch; die Schau sagt „ohne Scripts“, damit niemand einer 0 vertraut, die keine ist.

**Löschsperre:** In 3b.3 gibt es kein Löschen. Neue Daten hängen an, Lücken entstehen nicht. 3b.4 bringt „Letzte entfernen“, gesperrt bei N > 0 (Muster DSPRE-Avalonia), und die Prüfungen. Grund: Löschen ohne Script-Referenzen wäre ein Versprechen, das der Index nicht halten kann.

Verworfene Alternativen:
- **(a) Index beim Projektöffnen voll bauen.** Kostet beim Öffnen, auch wer nie Daten anfasst.
- **(b) Kein Index, Scan je Anfrage.** „Verwendet in“ für 471 Attacken in der Liste wären 471 Scans.

Preis, falls falsch: Ist inkrementell zu fehleranfällig, bleibt `rebuild_full` als Fallback hinter einem Generationszähler. Das kostet Geschwindigkeit, keine Korrektheit.

---

## 7. Begegnungen

**Entscheidung: Bauen-Layout (§4) mit fester Fachlogik im Kernel (`data/encounter_rules.rs`). Die Oberfläche rechnet nichts selbst.**

**Slot-Anteile** fest je Familie und Methode in `metadata/encounter_odds.yaml`. Nur Anzeige; den Slot-Odds-Patch (Overlay) gibt es nicht. Herkunft D#269 **[offen]**; Plan-Task 0 vergleicht mit pret (`pokeplatinum` overlay006, `pokeheartgold` Wild-Encounter-Code).

| Methode | Pt | HGSS |
|---|---|---|
| Gras | 20/20/10/10/10/10/5/5/4/4/1/1 | wie Pt |
| Surfen | 60/30/5/4/1 | 60/30/5/4/1 |
| Alte Angel | 60/30/5/4/1 | 40/30/15/10/5 |
| Profi-, Superangel | 40/40/15/4/1 | 40/30/15/10/5 |
| Fels | – | 80/20 |

**Ersetzungsregeln** als Datentabelle `ReplacementRule { source, replaces: &[Slot], when }`:
- Pt: Schwarm → Gras 0, 1; Tag/Nacht → Gras 2, 3; Radar → Gras 4, 5, 10, 11; Dual-Slot → Gras 8, 9. HGSS: landSwarm → Gras 0, 1; Radio [0]/[1] → Gras 2, 3 / 4, 5; surfSwarm → Surfen 0; fishSwarm → Alte 2, Profi 0/2/3, Super alle; nightFish → Profi 3 und Super 1 nachts (Schwarm hat Vorrang).
- Die Hand zeigt je Slot „Ersetzt durch“; die Schau warnt „Leerer Ersatz-Slot: Schwarm auf Gras 1 ergibt ein Glitch-Pokémon“.

**Prüfungen:** nur Warnungen, nie Sperren, nur bei Rate > 0. Art belegt bei Level 0; min > max („das Spiel tauscht sie“); Level > 100; leerer Ersatz-Slot. Sie erscheinen inline und im Prüflicht (Quelle „Begegnungen“, über den vorhandenen `ProblemsStore`).

**Leere Slots:** Art 0 und Level 0 sind gültig und gehen unverändert durch den Codec; das Feld zeigt „leer“, die Zeile ist gedämpft.

**Level-Klemmung** (Porymap): Setzt man min über max, wird max mitgezogen, und umgekehrt. Nur bei neuer Eingabe, nie beim Laden.

**HGSS:** Gras mit drei Art-Spalten (Morgens, Tags, Nachts) und einer gemeinsamen Level-Spalte; Methodenreiter Gras, Surfen, Fels, Alte, Profi, Super, Ersatz.

**Pt-Extras:** Formraten Shellos/Gastrodon als `Switch` West/Ost (der Rohwert bleibt, bis man umschaltet; „Ost“ schreibt 100 wie Vanilla); Unown-Tabelle als `SelectField`.

**Neue Datei und Anbindung:**
- `create_encounter_file(copy_of: Option<u16>) -> u16` hängt an (nicht rückgängig). HGSS lehnt Index ≥ 255 ab: „HeartGold kann höchstens 255 Begegnungsdateien ansprechen“.
- Anbinden ist eine rückgängige Header-Änderung im vorhandenen Header-Editor (Feld „Begegnungen“ als `Reference` mit Picker).
- „Für diesen Ort abspalten“ (bei N > 1): Kopie anlegen, dann im selben Changeset den Header umhängen. Das Anlegen bleibt nicht rückgängig; der Dialog sagt das.

**Nicht in 3b.3:** Honigbäume und Großmoor (`encdata_ex`); HGSS Kopfnuss, Safari, Fels-Items, Käferturnier; Schwarm-Orte; Rate als „jeder n-te Schritt“ (die Umrechnung ist ungeprüft; die Hand zeigt die Zahl mit „0 = nie“).

Verworfene Alternativen:
- **(a) Erzählen-Layout mit Weltszene und Gras-Overlay.** Braucht Draufsicht-Kamera und Overlay-Muster, laut 3c beide Phase 6.
- **(b) Slot-Anteile editierbar per Overlay-Patch.** Ein Patch-Layer, den 3b.3 nicht braucht; DSPRE verweigert ihn schon bei veränderter Instruktion.

Preis, falls falsch: Die Erzählen-Ansicht in 3d setzt auf dieselben Kernel-Daten (Regeln, Anteile, Warnungen) auf; nichts wird doppelt gebaut.

---

## 8. Trainer

**Entscheidung: zusammengesetztes Asset (§3), Format abgeleitet, „Neu“ plus Formular statt Assistent, Event-Verknüpfung über die Script-Nummer.**

**Party-Formate:**
- Die Schalter „Eigene Attacken“ und „Items tragen“ wirken auf alle Pokémon. Einschalten der Attacken füllt jedes Pokémon mit den letzten vier Lernset-Attacken bis zu seinem Level (die Spielregel für Trainer ohne Attacken **[offen: pret prüfen]**). Ausschalten verwirft sie; rückgängig, und das Label sagt es.
- Byte 1 in Pt: Feld „Stärke (IV-Skala)“ als u16; *kein* Geschlecht und keine Fähigkeit. Das High-Byte bleibt erhalten (einmal 9 in Pt DE).
- Byte 1 in HGSS: `difficulty` (0–255, „IV = Wert × 31 / 255“) und Geschlecht/Fähigkeit als zwei `SelectField`s (Auto / männlich / weiblich; Auto / 1 / 2).
- Form-Feld nur bei Arten mit Formen (Tabelle aus `species_forms.yaml`).

**KI:**
- Bauen: `Segmented` einfach (0x1) · klug (0x7) · Experte (0xF) · „eigene“. „Eigene“ ist aktiv, wenn der Wert keiner Vorgabe entspricht (zum Beispiel 519).
- Tüfteln: alle 32 Bit als `Checkbox`-Liste, davon 11 benannt. Alle Bits bleiben erhalten (DSPRE kappt auf 11).
- `battle_type` ebenso: Bauen bietet Einzel/Doppel (0/2); andere Bits bleiben.

**Klasse:** `SelectField` mit Namen aus der Klassenbank; unbekannte IDs als „Klasse 130“. Den Klassennamen bearbeitet man im Bereich Texte. Geschlechter- und Preisgeld-Tabellen werden weder gelesen noch geschrieben (§1); „Belohnung“ entfällt in 3b.3.

**Name:**
- `TextField` mit harter Grenze von 7 Zeichen: „Höchstens 7 Zeichen; längere Namen brauchen einen arm9-Patch.“
- Die Grenze steht in `metadata/arm9_tables.yaml` (`trainer_name_max: 7`), damit ein späterer Patch sie anheben kann.
- Geschrieben im selben Changeset wie der Trainer: Textarchiv 618 (Pt) bzw. 729 (HGSS), Index = Trainer-ID.

**Kampftexte:**
- `trtbl` und `trtblofs` als Assets; Texte im Archiv 617 (Pt), für HGSS **[offen]**.
- Invariante (gated geprüft): `len(trtbl)/4 == Messages(Archiv)`, `trtblofs` hat einen Eintrag je Trainer. Hält eine nicht, sind die Kampftexte schreibgeschützt („Trainertexte passen nicht zur Tabelle“).
- Bauen zeigt die vorhandenen Arten (vor dem Kampf, Niederlage, danach) als Textfelder. Neue Arten hängen Einträge ans Ende von `trtbl` und Nachrichten ans Ende des Archivs. Laut pret bleibt der Text bei fehlendem Eintrag leer (`Trainer_LoadMessage`) **[Code]**.

**„Neuer Trainer“:**
- `+` in der Liste öffnet einen `Dialog` „Trainer 928 anlegen?“ mit Klasse und Name.
- `create_trainer(class, name) -> u16` hängt an, nicht rückgängig: Props mit 0 Pokémon; Party aus 8 Nullbytes; Name an Index = ID (vorher Prüfung Namenszahl == Trainerzahl, sonst Abbruch mit Grund); `trtblofs`-Eintrag, der auf das Ende von `trtbl` zeigt. Danach ist der Trainer gewählt, und das normale Formular übernimmt.
- Taste „In der Welt platzieren“: wechselt in die Welt mit gespanntem Werkzeug Figur und ausstehender Trainerbindung. Die nächste platzierte Figur bekommt `trainer_type = 1`, `script = 3000 + id − 1` und Sichtweite 3 (Feld **[offen]**: `data[0]`?).
- Der Event-Inspector bekommt ein Feld „Trainer“: eine `Reference`, abgeleitet aus der Script-Nummer; Ändern setzt die Script-Nummer.
- **Grenze:** Trainer-Flags liegen im Flag-Bereich (`NUM_FLAGS 2912` in beiden pret-Repos [Code]). Der Startindex der Trainer-Flags und damit die höchste zulässige Trainer-ID sind **[offen]**; Plan-Task 0 bestimmt sie, `create_trainer` lehnt darüber ab.

Verworfene Alternativen:
- **(a) Assistent mit fünf Schritten.** Er verdoppelt das Formular; seine Mehrwerte (Live-Vorschau, Level-Vorschläge) sind in 3c ausgeschlossen bzw. Heuristik, die das Gate nicht braucht.
- **(b) Klassen-Geschlecht und Preisgeld schreibbar.** Unverifizierte Offsets in arm9 und Overlay; eine Änderung wirkt auf alle Trainer der Klasse. Das ist ein eigener Klassen-Editor.

Preis, falls falsch: Wünscht der Nutzer den Assistenten doch, setzt er in 3d auf `create_trainer` und `edit_trainer` auf; kein Kernel-Rückbau.

---

## 9. Gate und Prüfbarkeit

### 9.1 Gate 3b.3a (automatisch und von Hand)

- `vanilla_data` gated grün über alle Member der zehn Arten in den drei Projekten; das uxie-Orakel stimmt überein.
- Der vanilla Rebuild ist weiter byte-identisch; `vanilla_rebuild_matches_dsrom_cli` packt jetzt auch die zehn neuen Ordner.
- Szenario `pt-daten.yaml` (nur Backend, kein Spiel): Staravia Angriff 75 → 85, eine Lernset-Zeile mehr, Flügelschlag Stärke 60 → 65, Trank Preis 300 → 250; speichern, bauen. Ein Prüfschritt liest die gebauten NARCs mit **uxie** und erwartet die neuen Werte. Ein unabhängiger Leser fängt auch Fehler, bei denen Codec und Writer sich gegenseitig bestätigen.
- Von Hand: Die drei Editoren sind wiedererkennbar (Screens Pokédex, Attacken, Gegenstände, ohne DS-Profil); ⌘Z nimmt die TM-Kopie über drei Assets in einem Schritt zurück; Tüfteln zeigt den Hexblock mit markiertem Feld.

### 9.2 Gate 3b.3b: „Zwei neue Trainer und geänderte Begegnungen im Spiel“

**Vorbedingung, das eigentliche Problem:** Der Zimmer-Spielstand hat kein Pokémon, und Route 201 (Pt) sowie Route 29 (HGSS) sind durch Story-Trigger gesperrt. Die Lösung braucht weder einen neuen Spielstand noch Story-Variablen:

1. Neuer Ort per `create_area` (wie `pt-neuer-header.yaml`): Karte kopiert aus einer Karte mit hohem Gras, neuer Header mit leerer Eventdatei, also ohne Trigger.
2. `create_encounter_file`, dann `set_header` mit `wild_pokemon` = neue Datei. `data_edit Encounter`: alle 12 Gras-Slots auf Art `S_WILD`, Level `L_WILD`, Rate 100 **[offen: ob das jeden Schritt garantiert; der Plan misst]**. Zusätzlich auf der Vanilla Route 201 Slot 0 geändert, damit „geändert“ auch eine Vanilla-Datei abdeckt (nur Build-Prüfung per uxie).
3. `create_trainer` ×2 (Wanderer „Jens“, Göre „Mia“), je ein Pokémon mit eindeutiger Art und Stufe (`S_T1`/`L_T1`, `S_T2`/`L_T2`), Texte „vor dem Kampf“ und „Niederlage“.
4. `add_event` ×2 auf dem neuen Ort: Overworld, `trainer_type = 1`, Script 3000 + id − 1, Blick quer zum Weg, Sichtweite 3.
5. Im Zimmer eine Figur, deren Script per `GivePokemon` einmalig ein Pokémon auf Level 100 gibt (3b.2-Mechanik wie im Item-NPC-Gate; Befehlsname **[offen]**). Die Treppe wird auf den neuen Ort umgebogen (`update_event` wie in `pt-raum-assistent.yaml`).
6. `save`, `build_rom`.

**Spielen** (neue `g4emu`-Schritte):
- `continue` → Figur ansprechen (`walk-to`, `press a`, `mash a 300`) → Treppe → `expect-map NEU` → `walk-until-battle right 6` → `expect-battle S_T1 L_T1` → `shot trainer1` → `win-battle`; Trainer 2 ebenso; dann `walk-until-battle up 2 --back-and-forth` → `expect-battle S_WILD L_WILD` → `shot wild`.
- Zweites Szenario `hg-trainer-begegnung.yaml` für HeartGold DE; HeartGold US nur gated Tests, wie bisher.

**Neue `g4emu`-Schritte** (`steps.py`, Tests in `test_steps.py`): `expect-battle SPECIES LEVEL [--timeout F]` pollt, bis der gegnerische BattleMon Art und Level hat, sonst Fehler. `walk-until-battle DIR N [--back-and-forth]` läuft, bis ein Kampf aktiv ist, und toleriert die Unterbrechung durch die Sichtlinie (heute meldet `walk` sie als unerwartete Position). `win-battle [--timeout F]` drückt A, bis kein Kampf mehr aktiv und der Ort wieder lesbar ist. `mash BUTTON FRAMES` klickt Text weiter.

**Kampf im RAM:**
- `games.yaml` bekommt je Spielcode `battle: { pointer, offset, species_at, level_at }`, kalibriert mit dem vorhandenen Werkzeug (`dump-ram`, `find-u16`, `pointers_to`): zwei Läufe mit Gras ganz auf Art A bzw. Art B.
- Das ist Task 1 von 3b.3b, *vor* allen UI-Tasks: Ohne diese Kalibrierung ist das Gate nicht beweisbar.
- Fallback ohne statischen Zeiger: Signatur-Suche mit drei Feldern (Art, Level, erste Attacke) im RAM. Zwei Felder reichen nicht: In Pt läge in der geladenen Begegnungstabelle selbst ein Fehltreffer (Slot-Raster 8 B: Art an X, Level an X + 52 ist ein Slot-Level).
- Screenshots sind Beleg für den Menschen („Wanderer Jens möchte kämpfen!“), nicht der Beweis.

**Was das beweist:** Die Trainer existieren im ROM mit genau der gebauten Party (Art und Level im Kampf); sie sind über Event und Script verknüpft (die Sichtlinie löst aus); Name und Texte laufen ohne Absturz durch; die neue Begegnungsdatei wirkt über den neuen Header-Link; die Änderung eines Vanilla-Members wirkt (uxie-Prüfung des Builds).

**Neue Brücken- und Tauri-Befehle.** Jeder steht in `COMMANDS`, in `dispatch.rs` und im Tauri-Handler; `commands_match_the_tauri_handler` zählt sie.

| Befehl | Zweck |
|---|---|
| `data_list {kind, filter?}` | Zeilen für die Listen: Index, Name, Kennung, Zusatz |
| `data_get {kind, index}` | typisiertes Dokument (`DataDoc`, serde-getaggt nach `kind`) und Lock-Grund |
| `data_edit {kind, index, patch, label?}` | Teil-Patch je Art; Ergebnis `ChangeId` |
| `data_raw {kind, index}` | Bytes, `FieldSpan`-Karte, Pfad (Tüfteln) |
| `name_tables {}` | Species, Moves, Items, Fähigkeiten, Typen, Klassen, Formen |
| `machine_table {}`, `item_table {}` | arm9 lesend, oder „nicht verfügbar“ |
| `refs {target}` | Liste `Ref` mit Ort und Navigationsziel |
| `create_trainer {class, name}` | 3b.3b |
| `create_encounter_file {copy_of?}` | 3b.3b |
| `split_encounter_file {header}` | 3b.3b |
| `set_trainer_texts {trainer, texts}` | 3b.3b, ein Changeset |
| `get_item_icon {item}` | PNG, 3b.3b |

- `kind` ∈ `species | learnset | evolution | move | item | trainer | encounter`. Drei generische Datenbefehle statt rund zwanzig typisierter: eine Dispatch-Zeile je Befehl, ein Testmuster. Typisiert bleibt es über `DataDoc` und die TS-Union in `api.ts`.
- Undo-Labels sind sprechend („Staravia: Angriff 75 → 85“).

**Weitere Tests:** Kernel `tests/store.rs` je Art (laden byte-identisch, Edit, Undo, Dirty, Save, gesperrtes Member). Trainer-proptest: für jede Edit-Folge gelten `flags` = Format und `party_count` = Länge. Mehrteiliger Changeset: Undo stellt alle Parts her, eine fehlschlagende Closure ändert nichts. `refs` inkrementell == voll; arm9-Plausibilität. vitest je Editor über Rollen und `data-*` (`data-species`, `data-trainer`, `data-slot`). `docs/agent-gate.md` mit Rezepten; `docs/phase-3b3-gate.md`.

Verworfene Alternativen:
- **(a) Beweis nur per Screenshot und Blick des Agenten.** Nicht reproduzierbar, wird nie automatisch rot.
- **(b) Neuen Spielstand mit Starter auf Route 201 von Hand erzeugen.** Hängt an einer Handlung des Nutzers und am Story-Fortschritt und ist je Spiel neu. Der Weg oben baut alles im Studio, genau der Hack-Ablauf von 3b.4.

Preis, falls falsch: Findet die Kalibrierung keinen stabilen Zeiger und versagt auch die Signatur, fällt das Gate auf Screenshot und Blick von Hand zurück. Das ist schwächer; der Plan meldet es dann als offenes Gate-Kriterium, statt es zu verschweigen.

---

## 10. Risiken (die fünf größten)

1. **Nebentabellen neuer Trainer.** `trtblofs` ist über die Trainer-ID indiziert; ohne Eintrag liest das Spiel hinter das Member-Ende. Trainer-Flags können über den Flag-Bereich hinauslaufen, und die Namensbank muss genau eine Nachricht je Trainer haben. Folge: Absturz oder verdorbener Spielstand.
   *Gegenmaßnahme:* Plan-Task 0 prüft die Invarianten an Vanilla (`len(trtbl)/4 == Messages`, `trtblofs` = Trainerzahl, Namenszahl = Trainerzahl, Start der Trainer-Flags aus pret). `create_trainer` verweigert, wenn eine nicht hält. Das Emulator-Gate legt *zwei* Trainer an und kämpft gegen beide.
2. **Gate nicht beweisbar** (Kampf im RAM, Story-Sperren, kein Pokémon im Spielstand).
   *Gegenmaßnahme:* Kalibrierung als erster Task von 3b.3b; neuer Ort ohne Trigger; Pokémon per Script im Zimmer; Signatur-Fallback; offen benennen statt umdeuten.
3. **Store-Umbau** (mehrteiliger Changeset, `MemberPair`) bricht Undo oder Save der bestehenden Arten.
   *Gegenmaßnahme:* Die bestehenden `store.rs`-Tests bleiben unverändert grün, als erstes Kriterium; proptest auf Undo/Redo-Inverse über gemischte Arten; der Changeset-Umbau ist ein eigener Task vor jeder neuen Art.
4. **Umfangsdrift durch die Screens** (DS-Profile, Weltszene in Trainer und Begegnungen, Rezepte, Läden, Ei-Attacken).
   *Gegenmaßnahme:* Die „Später“-Liste aus §1 steht wörtlich in der Spec. Schau = Karten aus vorhandenen Primitives. Eine neue Komponente kommt nur mit einem Editor, der sie in diesem Plan benutzt.
5. **Falsche Offsets und Annahmen in Hack-Projekten** (arm9-Patches verschieben Tabellen, Formbits in Species-Wörtern, unbekannte Flag-Bits).
   *Gegenmaßnahme:* Plausibilitätsprüfung mit „nicht verfügbar“ statt falscher Werte; Overlay-Codecs erhalten jedes Bit; abweichende Länge → gesperrt mit Grund; DP gesperrt.

---

## Anhang: was ich nicht geprüft habe

Zuordnung `trtbl`-Eintrag → Nachricht im Kampftext-Archiv und dessen ID für HGSS; Startindex der Trainer-Flags und höchste zulässige Trainer-ID; Sichtweite-Feld im Overworld-Event (`data[0]`); Name von `GivePokemon` in der rotom-Datenbank; ob Rate 100 eine Begegnung je Schritt garantiert; Slot-Anteile gegen pret; Pokémon-Symbol-Paletten-Tabelle; Sprite-Verschlüsselung in Pt und HGSS; Default-Attacken-Regel für Trainer; Renderzeit der Listen und Aufbauzeit des Index; alles zu Diamant/Perl; ob 3c.1 und 3c.2 beim Start von 3b.3 gemergt sind (dieser Vorschlag setzt den 3c-Rahmen mit Primitives, `PanelHead`, `goTo` und `history()` voraus).
