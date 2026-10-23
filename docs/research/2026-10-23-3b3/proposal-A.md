# Vorschlag A: Phase 3b.3 „Daten-Editoren“

Architekt A. Leitlinie: Risiko klein halten, jeder Schritt läuft für sich, der Kernel kommt zuerst. Stand 23.10.2026, `main` 11b335e, 3c.1 im Worktree `phase-3c1-doppeldeck`. 3b.3 beginnt erst, wenn 3c.2 gemergt ist; vorher fehlen die Primitives.

## 0. Eigene Prüfungen (nur gelesen)

Diese Punkte habe ich zusätzlich zu den drei Berichten an den Testprojekten geprüft:

- **Trainer-Kampftexte.** Pt-Archiv 617 hat 2497 Nachrichten, `trainerTextTable/0000` hat 9988 B, also 2497 Einträge zu 4 B. Bei HGSS (HGDE und HGUS) hat Archiv **728** 1717 Nachrichten bei 1717 Einträgen. Nachricht i gehört damit zu Tabellenzeile i. [geprüft]
- **Offset-Tabelle `trainerTextOffset/0000`.** Pt hat 928 u16-Einträge, genau einen je Trainer. HGSS hat **nur 735 Einträge bei 738 Trainern**. Die Tabelle `trtbl` ist nicht nach Trainer sortiert (Pt beginnt mit Trainer 247). Die Offsets sind Byte-Offsets in `trtbl`. [geprüft; wie das Spiel sucht, ist ungeprüft]
- **Paletten der Pokémon-Icons.** Ich habe die Bytefolge `00 01 01 01 00 00 00 00 02 02 01 01 00` je Spiel genau einmal in `arm9.bin` gefunden: Pt DE 0xF07D8, HG DE 0xFFBC4, HG US 0xFFC10. Der Abstand HG US zu HG DE ist 0x4C, derselbe wie bei der Item-Tabelle (0x100194 zu 0x100148). Das spricht für die Tabelle, bewiesen ist es nur über das Muster. [Muster geprüft, Bedeutung ungeprüft]
- **Header der Sprite-Member** (Pt DE):
  - `monIcons`: Member 0 ist NCLR, 1 bis 6 sind NANR/NCER, ab Member 7 folgen NCGR zu 1072 B (32×64, 4 bpp).
  - `itemIcons`: Member 0 und 1 sind NANR/NCER, danach abwechselnd NCGR (560 B) und NCLR.
  - `trainerGraphics` und `pokemonBattleSprites`: NCGR zu 6448 B (160×80 bzw. 80×160).
  - Alle Ordner liegen in allen drei Projekten unter `unpacked/`. [geprüft]
- **Objekt-Events.** uxie `ObjectEvent` hat `trainer_type: u16` und `data: [u16; 3]`. Wo die Sichtweite steht (vermutlich `data[0]`), ist **ungeprüft**.

Nicht geprüft habe ich: DP, die Kapazität der Trainer-Flags im Spielstand, die Offsets der Klassen-Geschlechtertabelle, die Verschlüsselung der Kampfsprites, die RAM-Adresse der Kampf-Spezies und ob ein Gras-Verhalten in einem Innenraum-Header Begegnungen auslöst.

---

## 1. Umfang und Schnitt

**Entscheidung: zwei Pläne, der Kernel zuerst, und der Gate-Beweis wandert in den ersten Plan.**

### 3b.3a „Datenkern und Nachschlagewerke“

**Kernel**
- Codecs für Encounter, Trainer-Eigenschaften, Party, Personal, Evolution, Learnset, Moves, Items, Trainer-Texttabelle und Offset-Tabelle.
- Neue Asset-Arten.
- Changesets mit mehreren Teilen.
- Namenskatalog.
- arm9-Tabellen lesend.
- Referenzindex v1.
- NCGR/NCLR-Decoder mit Item- und Pokémon-Icons.

**Befehle**
- Tauri-Befehle für alle Familien, auch Encounter- und Trainer-Bearbeitung und `create_trainer`.
- Die Brücke kennt jeden dieser Befehle.

**Oberfläche**
- Atlas „Pokémon“ und „Gegenstände“ freigeschaltet.
- Editoren Pokédex (Werte, Entwicklung, Lernset, TM/VM), Attacken und Gegenstände.

**g4emu**
- Neuer Schritt `expect-battle`, nach einer Kalibrierung je Spielcode.

**Gate 3b.3a**
- Round-Trip-Test über alle Vanilla-Member aller drei Projekte grün.
- Byte-identischer Rebuild bleibt grün.
- Ein **nur über das Backend** laufendes Szenario (`pt-trainer-begegnung.yaml`, dazu `hg-…`) legt zwei Trainer an, ändert eine Encounter-Datei und zeigt beides in `g4emu` (§9).
- Je ein Feld in Pokédex, Attacken und Gegenstände wird über die Test-Umgebung geändert, mit ⌘Z zurückgenommen und gespeichert.

So zeigen sich die größten Unbekannten (Texttabellen, RAM-Adresse, Encounter im Spiel) früh, bevor eine Zeile Trainer-Oberfläche entsteht.

### 3b.3b „Trainer und Begegnungen“

- Editoren Trainer und Begegnungen.
- „Neu“ als Formular, nicht als Assistent.
- „In der Welt platzieren“: Trainer-Event mit Sichtweite.
- „Für diesen Ort abspalten“.
- „Verwendet in“ in allen Editoren.
- Hinweise im Prüflicht.

**Gate 3b.3b (Roadmap-Gate):** Zwei neue Trainer und geänderte Begegnungen entstehen auf Platin DE und HeartGold DE **über die Oberfläche** (Rezepte in `agent-gate.md`). Dieselben Szenarien laufen danach in `g4emu` grün, und `scripts/gate.sh` ist grün.

### Später (3b.4, 3d oder Phase 4)

| Inhalt | Wann | Grund |
|---|---|---|
| Ei-Attacken, Attacken-Lehrer | 3b.4 | Pt im Overlay, Tutor in Overlay und Binärdatei; Patch-Ebene nötig |
| Läden | 3d | Overlay-Tabellen, verschieden je Spiel |
| Assistent „Neuer Trainer“ mit Vorschlägen | 3d, mit Erzählen | Heuristiken, Live-Vorschau in der Szene |
| Klassen bearbeiten (Geschlecht, Preisgeld, Musik) | später | arm9-Offsets nur aus DSPRE |
| Item- und TM-Tabelle schreiben | später | Fehler hier treffen das ganze Spiel |
| Headbutt, Safari, Rock Smash (HGSS); `encdata_ex` (Pt); Odds-Patch | später | Layouts ungeprüft |
| Kampf- und Trainer-Sprites | nach 3b.3 | verschlüsselte NCGR, Aufwand ungewiss |
| Pokédex-Größe und -Gewicht, Kategorie | 3b.4 | `pokeHeight` ungeprüft |
| Neue Spezies, Attacken, Items; CSV und Massenbearbeitung | nicht geplant | |
| „Auf Original zurücksetzen“ gegen das Basis-ROM | Phase 4 | Basis-ROM-Prüfung kommt dort |

**Verworfen:**
- *Ein Plan für alles:* rund 30 Tasks, und das Gate-Risiko zeigte sich erst am Ende.
- *3b.3a mit Pokémon, Attacken und Gegenständen, 3b.3b mit Trainer, Begegnungen und Gate (wie im Brief):* sauber nach Oberfläche geschnitten, aber Texttabellen und RAM-Kalibrierung blieben bis zum letzten Plan offen.

**Preis bei Irrtum:** Wenn 3b.3a zu groß wird, verschieben sich die Oberflächen von Pokédex, Attacken und Gegenstände nach 3b.3b. Der Kernel bleibt davon unberührt, also kostet das nur Zeit.

---

## 2. Codecs

**Entscheidung: eigene Codecs in `g4-formats` für alle Familien, gebaut als Rohbyte-Ansicht mit Feldtabelle.**

**Satz fester Größe** (Move 16, Item 34, Personal 44, Evolution 44, Trainer-Eigenschaften 20, Encounter 424 bzw. 196): `pub struct MoveRecord { raw: [u8; 16] }`.
- Getter und Setter laufen über eine statische Feldtabelle `FIELDS: &[Field { key, offset, bit, bits, signed }]`.
- `write()` gibt `raw` zurück. Damit gilt der Round-Trip schon durch die Bauart, auch Padding und unbekannte Bits bleiben erhalten.
- Dieselbe Feldtabelle treibt die Tüfteln-Ansicht (Hex mit markierten Bytes) und die Grenzen der `NumberField`s.

**Variable Länge**
- **Party:** `Vec<[u8; entry_size]>` plus `tail: Vec<u8>`. Ist `party_count * entry_size` unverändert, bleibt `tail` gleich. Sonst wird neu aufgefüllt: auf ein Vielfaches von 4 runden, mindestens 8 B, Füllbytes 0.
- **Learnset:** `Vec<u16>` plus Terminator plus `tail`. Bei Änderung wird auf 4 B aufgefüllt.
- **Texttabelle:** `Vec<(u16, u16)>`.
- **Offset-Tabelle:** `Vec<u16>`.

**Familienunterschiede** stecken in eigenen Feldtabellen je Familie (`Layout::for_family`):

| Format | DPPt | HGSS |
|---|---|---|
| Encounter | 424 B; Level als u32 mit Padding; Spezies u32; Wasser-Slot **max vor min** | 196 B; Raten u8 ×6; ein Level-Satz für Morgen, Tag und Nacht; Wasser **min vor max**; 0xC0 ist `nightFish` |
| Party-Eintrag | DP 6/8/14/16 B ohne Kapsel; Pt 8/10/16/18 B, Byte 0–1 = u16 `iv_scale` | 8/10/16/18 B, Byte 0 Schwierigkeit, Byte 1 Geschlecht/Fähigkeit |
| `wild_pokemon` im Header | u16, 0xFFFF = keine | u8, 0xFF = keine |

DP wird im Codec über das DSPRE-Layout unterstützt, ist in der UI als „ungeprüft“ markiert und hat kein Gate.

**Tests**
- `crates/g4-formats/tests/vanilla_data.rs`, gated wie die NARC-Tests: `write(read(x)) == x` über **jedes** Member jeder Familie in CPUD, IPKD und IPKE.
- Kernel-Test: jedes Vanilla-Member lädt **ohne Sperre**.
- Proptest je Feld: ein Feld auf einen Zufallswert setzen und prüfen, dass alle anderen Felder gleich bleiben (Bitfelder überschreiben keine Nachbarn).
- Jedes Feld auf seinen eigenen Wert setzen ändert kein Byte.

**Verworfen:**
- *uxie-Typen mit Nachbearbeitung:* Moves und Encounter sind kaputt. Trainer-Eigenschaften und Items lesen zu streng und brechen bei gehackten ROMs ab, statt zu sperren. Die Party-Use-Typen der Items sind außerhalb von uxie nicht benennbar.
- *uxie forken oder patchen:* Upstream-Pflege, und die Byte-Erhaltung bliebe trotzdem Sache jedes Writers.

**Preis bei Irrtum:** etwa 600 Zeilen Codec, die uxie teilweise schon hätte. Lesen und Schreiben bleiben dabei sicher, weil Rohbytes erhalten werden.

---

## 3. Asset-Modell

**Entscheidung: ein Asset ist ein Member. Zusammengehörige Teile laufen über ein Changeset mit mehreren Teilen.**

**Neue `AssetKind`s:** `Encounter`, `TrainerProps`, `TrainerParty`, `Personal`, `Evolution`, `Learnset`, `Move`, `Item`, `TrainerTextTable`, `TrainerTextOffsets`. Dazu kommen:
- je ein Feld in `FamilyArchives`,
- die Längen von `ALL` und `ArchiveMap.entries` (+10),
- der Test, der die Arten je Familie zählt.

Alle Neuen sind `Location::Member`.

**Changeset:** `Changeset { parts: Vec<Part { asset, before, after }>, label }`. `Store::apply_group(&[AssetId], label, closure)` lädt alle Teile, ruft die Closure mit `&mut [&mut Loaded]` auf und legt **ein** Changeset an. `undo` und `redo` stellen alle Teile wieder her oder keinen. `apply_edit` bleibt als Ein-Teil-Fall, die bestehenden Tests ändern sich nicht. Gesperrte Teile verweigern die ganze Gruppe. `history()` aus 3c zeigt das Label der Gruppe.

**Trainer:** Die Gruppe besteht aus `TrainerProps N`, `TrainerParty N` und `TextArchive 618/729`. Das Party-Format folgt den Flags in den Eigenschaften und hat damit eine einzige Quelle.

**Neuer Trainer:** `create_trainer` legt beide Member sofort an, wie `create_event_file`; das ist nicht rückgängig zu machen. Name, Texttabelle, Offset-Tabelle und die Kampftexte (Archive 617/728) folgen als **Gruppen-Edit**, rückgängig zu machen und bis zum Speichern als ungespeichert markiert.

**Namen:** neues Modul `catalog.rs` mit `Catalog { species, moves, items, abilities, types, classes, trainer_names, locations }`.
- Es liest die Archive **über den Store** (`read_text_doc`), damit ungespeicherte Textänderungen und neuere JSON-Dateien sichtbar sind.
- Es wird je Archiv gecacht und bei jedem Edit, Undo oder Neuladen eines Textarchivs verworfen.
- Die Archiv-IDs stehen in `metadata/text_banks.yaml` je (Familie, Sprache), mit den geprüften Werten aus den Berichten: Pt 412/647/392/610/624/619/618/617/433, HGSS 237/750/222/720/735/730/729/728/279.
- Trainernamen tragen in chatot die Hülle `{TRAINER_NAME:…}`. `trainer_name_get/set` entfernt und setzt sie, die Längenprüfung zählt nur den Inhalt.

**arm9-Tabellen:** `metadata/arm9_tables.yaml` je Spielcode: `item_table` (Offset, Anzahl), `tm_moves`, `icon_palettes`. Gelesen wird über `Project::arm9_table`, **nur lesend**. Plausibilitätsprüfung beim Lesen:
- jeder `data`-Index kleiner als die Member-Zahl,
- TM01 ≤ Zahl der Attacken.

Scheitert die Prüfung, steht ein Toast „Item-Tabelle nicht gefunden, Gegenstände nach Datenindex“, und die Funktion fällt auf einen einfacheren Modus zurück. Schreiben bringt eine spätere Phase mit einem `Slice`-Asset.

**Pfade innerhalb eines Spielpaars:** `archives.yaml` bekommt `packed_by_code: { IPG: "files/a/1/3/6", APA: "…" }`. Schlüssel ist das Präfix des Spielcodes, aufgelöst in `ArchiveMap::for_game(family, language, game_code)`. Der `unpacked`-Ordner bleibt gleich.

**Verworfen:**
- *Zusammengesetztes Trainer-Asset mit neuer `Location`:* Stale-Prüfung, Sperre und Speichern müssten drei Dateien und ein Textarchiv als eins behandeln, und jedes `match` auf `Location` wüchse.
- *Ganze Tabelle als ein Asset (wie `HeaderTable`):* Jede Änderung trüge 928 Datensätze im Changeset, und der Ungespeichert-Zustand je Trainer ginge verloren.

**Preis bei Irrtum:** Wenn Gruppen-Changesets Seiteneffekte im Store haben (Stale-Fall, Neuladen von außen), betrifft das alle Asset-Arten. Gegenmittel: eigene Kernel-Tests für Gruppe plus Stale plus Undo, bevor die erste Oberfläche sie nutzt.

---

## 4. Navigation im Doppeldeck

**Entscheidung: Unterbereiche als Titelmenü im Kopf der Liste. Die Brotkrume öffnet dasselbe Menü, die Palette kennt die Ziele.**

- `PanelHead` bekommt `titleMenu?: {value, options, onChange}`. Der Titel „Pokédex ▾“ ist dann ein `Menu` mit Pokédex, Trainer, Begegnungen und Attacken (3b.3a nur Pokédex und Attacken).
- Das zweite Glied der Brotkrume öffnet dasselbe Menü.
- ⌥1 bis ⌥4 schalten um.
- Die Palette bekommt die Befehle „Trainer öffnen“, „Begegnungen öffnen“ sowie Treffer für Spezies, Attacken, Items und Trainer.
- Gegenstände hat in 3b.3 keinen Unterbereich, Läden kommen später.
- `shellState` merkt den Unterbereich und die Hand-Höhe je Unterbereich (`g4.shell.v1` wird `v2`, migriert wird nicht).

| Entität | Liste | Schau | Hand |
|---|---|---|---|
| Pokédex | Suche, Typ-Filter, Zeile mit Icon, Name, Nummer | Profilkarte: Icon ×3 pixelig, Typen, 6 `StatBar`, Fundorte als `Reference` | Reiter Werte · Entwicklung · Lernset · TM/VM · Rohdaten (nur Tüfteln) |
| Attacken | Suche, Typ-Filter, Kategorie-Symbol | Karte: Kennzahlen, Merkmale, Beschreibung im `DSScreen textbox`, Verwendet in | Spalten Grundwerte · Verhalten · Rohdaten (Tüfteln) |
| Gegenstände | Suche, Taschen-Filter mit Zählern, Preis rechts | Karte mit Icon, Beschreibung als `DSScreen`, Verwendet in (Trainer, Personal) | Felder, Party-Effekt als `Section`, Rohdaten (Tüfteln) |
| Trainer (3b.3b) | `Segmented` Nach Ort / Nach Klasse, Baum, „Ohne Ort“ | `DSScreen textbox` „Wanderer Kai möchte kämpfen!“ mit Team-Icons, Liste „In der Welt“ mit Taste „In der Welt öffnen“ (`goTo`); **keine zweite 3D-Szene** | Team (6 `PokemonCard`) · Trainer-Spalte · gewähltes Pokémon |
| Begegnungen (3b.3b) | Dateien nach Ort, Gruppe „Ohne Ort“, Zähler „3 Orte“ | „Wer hier wartet“: Methode und Tageszeit wählbar, Arten mit Icon und Summe der Prozente | Reiter je Methode mit Zähler, Slot-`DataTable` |

**Tiefen:** nur Bauen und Tüfteln. Begegnungen erscheinen im Bauen-Layout, Erzählen folgt in 3d.

**Neue Primitives:**
- `Tabs` (Base UI, Zähler, Ungespeichert-Punkt)
- `DataTable` (Kopf, Roving Rows, Zelle mit Editor, „…“-Menü)
- `TypeBadge`
- `StatBar`
- `PokemonCard` (Ruhe, Gewählt, Leer)
- `FilterChips` (Toggle Group mehrfach, „n weitere“)
- `ToggleGrid` (TM/VM-Raster)
- `HexBlock` (markierte Bytes aus der Feldtabelle)
- `Sprite` (Canvas aus PNG, `image-rendering: pixelated`)

**Ersetzt durch vorhandene Bausteine:**
- Slider → `NumberField` mit Scrubbing und Hinweistext
- Kennzahlkarte → `InspectorRow` und `Badge`
- Inline-Picker → `Picker` als Dialog
- DSScreen-Varianten → `textbox`
- RecipeButton, Checkbox-Matrix, Mini-Kurve → entfallen

**Verworfen:**
- *Eigene Atlas-Einträge je Entität:* sprengt den Atlas mit sieben Bereichen.
- *`Segmented` mit vier Segmenten über der Liste:* in 272 px zu eng und bei Kompakt unlesbar.

**Preis bei Irrtum:** Das Titelmenü wird übersehen. Die Brotkrume und die Palette fangen das auf; ein Umbau betrifft nur `PanelHead`.

---

## 5. Sprites aus dem ROM

**Entscheidung: Item-Icons und Pokémon-Icons kommen in 3b.3a. Kampf- und Trainer-Sprites bleiben Platzhalter.**

- `g4-formats/src/{ncgr.rs, nclr.rs}`: 4 bpp, gekachelt und linear, unkomprimiert, BGR555 nach RGBA. `Rgba` wird aus `nsbtx` wiederverwendet.
- **Item-Icons:** Die arm9-Item-Tabelle liefert die Member-Nummern von Icon und Palette direkt.
- **Pokémon-Icons:** `monIcons` ab Member 7 plus Spezies (die Zuordnung der Formen prüft der Plan), Palette 0 bis 2 aus Member 0, Index aus `icon_palettes` (§0). Fehlt die Tabelle, nimmt der Code Palette 0 und schreibt einen Hinweis in den Tüfteln-Abschnitt.
- **Befehl:** `icons(kind: "species"|"item", ids: u16[]) -> Record<id, dataUrl>`. PNG-Encode im Kernel, LRU-Cache im Kernel, Map im Frontend. Die CSP erlaubt `data:`. Die Brücke liefert dasselbe.
- **Platzhalter:** farbiges Quadrat in der Typfarbe mit Initiale. Kampfvorschau und Pokédex-Profil zeigen das Icon dreifach vergrößert.
- Kampf- und Trainer-Sprites (verschlüsselte NCGR, 160×80) folgen später als eigener Task mit Test gegen DSPRE-Ausgaben.

**Verworfen:**
- *Alles inklusive Kampf-Sprites in 3b.3:* Entschlüsselung ungeprüft, für das Gate unnötig.
- *Keine Sprites:* Listen mit 500 Namen ohne Bild sind schwer zu durchsuchen. Die Icons sind billig, weil der Decoder klein ist.

**Preis bei Irrtum:** falsche Icon-Farben, ein reiner Anzeigefehler. Der Platzhalter bleibt als Rückfall.

---

## 6. Referenzen

**Entscheidung: Kernel-Modul `refs.rs` mit Vorwärtskanten je Quell-Asset. Es wird beim ersten Abruf aufgebaut und bei jedem Edit nachgeführt.**

- `refs_of(kind, &Loaded) -> Vec<Edge { target: Target, field: &'static str }>`.
- `Target` ist eins von `Species(u16)`, `Move(u16)`, `Item(u16)`, `Encounter(u16)`, `Trainer(u16)`, `TrainerClass(u8)`.

| Quelle | Ziele |
|---|---|
| Header | Encounter-Datei (`wild_pokemon`) |
| Encounter | Spezies, aus allen Gruppen |
| Trainer-Eigenschaften | Klasse, Items |
| Party | Spezies, Items, Attacken |
| Personal | Items |
| Evolution | Ziel-Spezies, Param je Methode |
| Learnset | Attacken |
| Event | Trainer über `script` 3000..4999 und 5000..6999 (`trainer = script − 2999` bzw. `− 4999`) |

**Aufbau:**
- Der Index liest beim ersten `used_in` alle Member roh über die Codecs, nicht über den Store. Das sind rund 5000 kleine Dateien, Ziel unter 500 ms, gemessen im Plan.
- `Store::apply_*`, `undo`, `redo` und `reload_from_disk` melden die geänderte `AssetId`. Der Index ersetzt dann nur deren Kanten.

**Befehle:**
- `used_in(target) -> UseDto[] { kind, assetId, label, place?, goTo }`
- `ref_counts(targetKind) -> Record<id, n>` für Zähler in Listen

„Ort“ eines Trainers heißt: die Header, deren Eventdatei das Event enthält.

**In 3b.3:**
- Verwendet in, Zähler, „Ohne Ort“, Fundorte.
- Hinweis beim Bearbeiten einer geteilten Encounter-Datei.
- **Es gibt nichts zu löschen**, also auch keine Löschsperre.

**In 3b.4:**
- Script-Referenzen mit „mindestens“, Löschen mit Sperre.
- Prüfungen: Trainer ohne Party, `party_count` passt nicht, leere Ersatz-Slots, Encounter-Datei unbenutzt, Trainer-Event ohne Kampftexte.

**Verworfen:**
- *Index beim Projektöffnen bauen:* verlangsamt jedes Öffnen, auch wenn niemand Daten-Editoren nutzt.
- *Bei jeder Anfrage scannen, ohne Index:* Zähler für 500 Listenzeilen wären zu langsam.

**Preis bei Irrtum:** Veraltete Kanten zeigen falsche Zähler. Gegenmittel: ein Test, der nach Edit und Undo den Index mit einem Neuaufbau vergleicht.

---

## 7. Begegnungen (3b.3b, Kernel in 3b.3a)

**Entscheidung: Bauen-Layout mit Slot-Tabelle. Prozente und Ersetzungsregeln sind sichtbar, Prüfungen sind Hinweise, nie Sperren.**

**Hand**
- Reiter mit Zählern (Slots mit Spezies ≠ 0): Gras, Surfen, Alte Angel, Profiangel, Superangel, Poké-Radar (Pt), Schwarm, Radio (HGSS), Sonstiges (Pt Dual-Slot, Formen, Icognito).
- Darüber steht die Methoden-Rate als `NumberField` 0–255 mit dem Hinweis „0 = hier nie“. Eine Umrechnung in Schritte gibt es nicht, weil sie ungeprüft ist.
- Gras hat dazu `Segmented` Morgens · Tags · Nachts.
  - HGSS: drei Spezies-Spalten mit einer gemeinsamen Level-Spalte.
  - Pt: Tag und Nacht ersetzen die Slots 2 und 3.
- **`DataTable`-Spalten:**
  - Nr.
  - Anteil: aus `metadata/encounter_odds.yaml`, Kopf „Standard-Spielregel“, nur lesend.
  - Pokémon: `Reference` mit `Picker`, Icon.
  - Level, beim Wasser min und max.
  - „Ersetzt durch“: Plaketten für Schwarm, Radar, Radio, Tag/Nacht, Nachtangeln, mit Klick auf den Reiter.
- Die Byte-Reihenfolge von min und max verbirgt der Codec.

**Schau**
- „Wer hier wartet“ für die gewählte Methode und Tageszeit: Arten zusammengefasst mit Icon, Level-Spanne und Summe der Anteile.
- Darunter „Diese Datei nutzen“ als `Reference`-Chips auf Header.
- Bei mehr als einem Ort erscheint die Notice „Änderungen wirken an 3 Orten“ mit der Taste **„Für diesen Ort abspalten“**: neues Member als Kopie (sofort angelegt) plus Header-Edit (rückgängig zu machen).

**Leere Slots:** Spezies 0 erscheint als „leer“, Level 0 als „–“. Die Prüfung läuft nur bei Rate > 0:
- „Slot 4 ist leer, das Spiel zeigt dort ein Glitch-Pokémon“
- Level außerhalb 1..100
- „min größer als max, das Spiel tauscht sie“
- leerer Ersatz-Slot

Alles landet als Hinweis im Prüflicht, gespeichert wird trotzdem.

**Neue Datei:** „+“ in der Liste legt eine leere Datei oder eine Kopie der gewählten an (`create_encounter`). Angebunden wird im Header-Inspector: Das Feld `wild_pokemon` wird `Reference` plus `Picker`, mit „keine“.
- HGSS: Ist die Member-Zahl ≥ 255, verweigert `create_encounter` mit „HeartGold kann höchstens 255 Begegnungsdateien ansprechen.“
- Pt: Grenze 0xFFFE.

**Verworfen:**
- *Erzählen mit Szene, Gras-Flecken und Rezepten:* braucht Draufsicht (Phase 6) und Rezepte (3d).
- *Platzkarten wie im Screen:* zwölf Karten je Methode statt einer Tabelle, kein Platz für min und max.

**Preis bei Irrtum:** wirkt nüchterner als der Screen. Die Tabelle bleibt in 3d als Bauen-Ansicht neben Erzählen bestehen.

---

## 8. Trainer (3b.3b, Kernel in 3b.3a)

**Party-Formate.** Die Schalter „Eigene Attacken“ und „Gegenstände“ formatieren alle Einträge in **einer** Gruppe (Eigenschaften und Party) um. Vor dem Abschalten fragt ein `Dialog`: „Die Attacken von 3 Pokémon gehen verloren. Abschalten?“ ⌘Z stellt sie wieder her. Ohne eigene Attacken zeigt die Karte „Attacken nach Level“, berechnet aus dem Learnset.

**Byte 1:**
- **Pt:** Feld „IV-Stufe“, u16 0–255, mit dem Hinweis „IV = Stufe × 31 / 255“. Geschlecht und Fähigkeit **gibt es nicht**.
- **HGSS:** Schwierigkeit u8, dazu Geschlecht (Auto, männlich, weiblich) und Fähigkeit (Auto, 1, 2) aus den Nibbles.
- Unbekannte Bits bleiben erhalten (Rohbytes).

**KI:** `Section` „KI“ mit elf `Checkbox`en und deutschen Namen. Unbekannte Bits stehen in Tüfteln und bleiben erhalten. Keine Vereinfachung auf drei Stufen. `battle_type` erscheint als `Segmented` Einzel · Doppel, ein anderer Wert als „Wert n“.

**Klasse:**
- `SelectField` mit Klassennamen.
- Geschlecht und Preisfaktor werden **nicht** gezeigt (Offsets ungeprüft); die Belohnung entfällt.
- Klassen umbenennen heißt eine Nachricht im Textarchiv ändern, und das geht im Bereich Texte.

**Name:** `TextField`. Mehr als 7 Zeichen zeigen den Fehler „höchstens 7 Zeichen“ und werden nicht übernommen. Ein arm9-Patch bleibt außerhalb des Umfangs.

**Neu:** „+“ in der Liste öffnet einen `Dialog` „Neuer Trainer“ mit Klasse, Name und erstem Pokémon (Picker plus Level). `create_trainer` legt an:
- Eigenschaften und Party, ID = Member-Zahl,
- den Namen im Archiv 618/729, an Index = ID,
- zwei Kampftexte im Archiv 617/728 („Vor dem Kampf“, „Niederlage“, Vorgabetexte),
- deren Zeilen in `trtbl` mit Trigger-Werten aus Vanilla, die der Plan prüft,
- den Offset-Eintrag. Bei HGSS füllt er die drei fehlenden Einträge 735..737 mit 0 auf, ungeprüft (§10).

Danach zeigt die Hand den neuen Trainer. Der Assistent in fünf Schritten folgt in 3d.

**Welt:** Taste **„In der Welt platzieren“** wechselt in den Bereich Welt mit dem Werkzeug Figur, vorbelegt als Trainer. Ein Klick auf eine Kachel erzeugt das Objekt-Event:
- Sprite der Klasse aus einer kleinen Tabelle, sonst ein Standard-Sprite,
- `trainer_type = 1`,
- Sichtweite 3,
- `script = 2999 + id`.

Dafür gibt es den Befehl `place_trainer(trainer, event_file, x, z, dir)`. Im Event-Inspector bekommen „Trainer“ (Reference) und „Sichtweite“ eigene Zeilen.

**Kampftexte** erscheinen als `Reference` in den Bereich Texte (Archiv 617/728, Nachricht n).

**Verworfen:**
- *Assistent jetzt:* Er braucht die Live-Vorschau in der Szene und Heuristiken. Das Formular deckt das Gate ab.
- *Gender/Ability auch für Pt anzeigen (wie DSPRE):* verfälscht `iv_scale`.

**Preis bei Irrtum:** Nutzer vermissen den Assistenten. Er setzt in 3d auf denselben Befehl `create_trainer` auf.

---

## 9. Gate und Prüfbarkeit

**Entscheidung: Beweis über die Spezies im Kampf, aus dem RAM gelesen. Zuerst in 3b.3a nur über das Backend, in 3b.3b dieselben Schritte über die Oberfläche.**

**g4emu**
- Neue Schritte:
  - `battle`: meldet `{species, level}` des ersten Gegners oder `null`.
  - `expect-battle SPECIES [LEVEL] [TIMEOUT]`.
  - `step-until-battle X Y N`: läuft zwischen zwei Kacheln hin und her, bis ein Kampf beginnt oder N Schritte vorbei sind.
- `games.yaml` bekommt je Spielcode `battle: { pointer, offset }`.
- **Kalibrierung** (Task in 3b.3a):
  1. Zwei Testläufe mit Gras-Slots auf Spezies 150 und dann 151.
  2. In jedem Kampf `dump-ram`.
  3. `g4emu find-u16` mit der Schnittmenge und `pointers_to` für die Zeigerkette.
- Die Kampf-Struktur liegt unverschlüsselt im Speicher; ungeprüft.

**Szenario `pt-trainer-begegnung.yaml`** (CPUD, Spielstand `CPUD-zimmer.sav`, Spieler im Zimmer, Header 415), dazu `hg-trainer-begegnung.yaml` (IPKD, Header 64).

1. **Pokémon geben.** Im Spielstand hat der Spieler noch keins. Ein NPC im Zimmer bekommt ein Script mit `GivePokemon` (Befehle aus 3b.2), der Gebe-Befehl läuft per `press a`.
2. **Begegnungen.** `wild_pokemon` von Header 415 wird auf eine neue Encounter-Datei gesetzt: Gras-Rate 255, alle 12 Slots auf eine auffällige Spezies, Level 5. Eine Bodenkachel bekommt das Verhalten „hohes Gras“ (Permission-Edit aus Phase 1). Rückfall, falls Innenräume keine Begegnungen auslösen: eine Grasfläche in Zweiblattdorf (Header 411, vor dem Haus); der Plan prüft das im ersten Task.
3. **Zwei Trainer.** `create_trainer` zweimal mit verschiedenen Spezies und Leveln. `place_trainer` setzt sie im Zimmer in Sichtlinie eines Laufwegs, Blick zur Tür, Sichtweite 3.
4. **Spielen:**
   - `continue`, `expect-map 415`, NPC ansprechen, `press a:40`
   - `step-until-battle 5 6 200`, `expect-battle <Spezies> 5`, `shot wild`, fliehen
   - zur Sichtlinie gehen, `press a:40`, `expect-battle <T1-Spezies> <Level>`, `shot trainer1`
   - Kampf beenden (Angriffstaste wiederholt)
   - zweiter Trainer, `expect-battle`, `shot trainer2`

**3b.3b Gate (A):** Dieselben Inhalte entstehen über die Oberfläche in der Test-Umgebung (Rezepte nennen Felder beim Namen). Danach baut `call.sh build`, und das Szenario spielt ohne Edit-Schritte (`--project` auf der von der UI bearbeiteten Kopie).

**Brücke:** neue Einträge in `bridge/dispatch.rs`, der Test `commands_match_the_tauri_handler` zählt mit:
- `list_species`, `list_moves`, `list_items`, `list_trainers`, `list_encounters` (Zusammenfassungen)
- `get_*`, `update_*` (Patch nach Feld-Schlüssel), `set_party`, `set_trainer_name`
- `create_trainer`, `create_encounter`, `split_encounter`, `place_trainer`
- `used_in`, `ref_counts`, `icons`, `catalog`

Szenarien bleiben reine Backend-Aufrufe.

**Verworfen:**
- *Nur Screenshots:* Ein Agent erkennt Pixel-Sprites nicht zuverlässig, ein Mensch müsste mitlaufen.
- *Gerenderten Pokédex-Eintrag oder Spielstand auslesen:* Die Pokémon-Daten im Spielstand sind verschlüsselt, und der Weg dorthin ist länger.

**Preis bei Irrtum:** Gelingt die Kalibrierung nicht, fällt das Gate auf `shot` plus Blick des Nutzers zurück. Das ist langsamer, bleibt aber beweisbar.

---

## 10. Risiken

1. **Neue Trainer jenseits der Vanilla-Zahl** (Kampftext-Suche über `trtblofs`, HGSS-Tabelle mit 3 Einträgen zu wenig, Kapazität der Trainer-Flags): Das Spiel könnte Unsinn zeigen, abstürzen oder fremde Flags setzen.
   *Gegenmaßnahme:* Das Backend-Szenario in 3b.3a beweist das vor jeder Oberfläche. `create_trainer` verweigert IDs über einer Grenze in `metadata/trainer_limits.yaml`, die ein Task aus pret oder Messung festlegt. Ein Hinweis nennt die Grenze.
2. **RAM-Adresse der Kampf-Spezies** liegt hinter einem Heap-Zeiger oder schwankt.
   *Gegenmaßnahme:* Die Kalibrierung ist der erste Task nach den Codecs, mit `pointers_to`. Rückfall wie in §9 beschrieben.
3. **Store-Umbau für Gruppen-Changesets** bricht Undo, Stale oder Speichern für bestehende Arten.
   *Gegenmaßnahme:* Ein-Teil-Pfad unverändert. Neue Kernel-Tests (Gruppe plus Stale, Gruppe plus Sperre, Undo über Gruppen hinweg). Die Test-Umgebung wiederholt 3a (A).
4. **Umfangsdruck aus den Screens** (Erzählen, Läden, Assistent, Vorschläge, Belohnung, Kampf-Sprites).
   *Gegenmaßnahme:* Die Ausschlussliste aus §1 kommt wörtlich in die Spec. Jede Design-Abweichung wird in der Spec als Abweichung benannt, nicht stillschweigend gebaut.
5. **Leistung von Listen und Index** (928 Trainer, 508 Spezies, rund 5000 Dateien) und **falsche arm9-Offsets** in veränderten ROMs.
   *Gegenmaßnahme:*
   - Zusammenfassungs-Befehle lesen roh und gecacht je Art und werden pro geändertem Asset verworfen.
   - Messwerte gehören zu den Abnahmepunkten (Liste unter 200 ms, Index unter 500 ms).
   - arm9-Tabellen nur lesend, mit Plausibilitätsprüfung und Rückfallmodus.
   - Abweichende Member werden durch `unwritable_reason` gesperrt statt beschädigt.
