# Vorschlag B: Phase 3b.3 „Daten-Editoren“

Architekt B. Haltung: Zuerst das Nutzererlebnis. Die Design-Screens werden so weit umgesetzt, wie das ROM es hergibt. Einsteiger arbeiten in der Tiefe **Bauen** ohne IDs, Hex-Werte oder Bitfelder. Rohwerte gibt es nur in **Tüfteln**. Stand: 23.10.2026, gelesen auf `main` 11b335e.

**Voraussetzung:** 3c.1 und 3c.2 sind gemerged, also `ui/` mit Primitives, Scharnier, Assistenten-Modus und Galerie. Heute gibt es `apps/studio/src/ui/` auf `main` noch nicht.

**Nicht selbst geprüft** (aus den Berichten übernommen): Byte-Layouts außer den unten genannten Stellen, Sprite-Verschlüsselung und Icon-Paletten, die Trainer-Texttabellen (`trtbl`/`trtblofs`), `ObjectEvent.data[0]` als Sichtweite, die Pt-Kapazität der Trainer-Flags, das `BattleMon`-Layout im RAM, die Umrechnung der Begegnungsrate in Schritte.

**Selbst gelesen:** uxie `ObjectEvent` hat `trainer_type` und `data: [u16; 3]`. `Changeset` (`store.rs:184`) kennt genau ein Asset. `archives.yaml` kennt keine Abweichung je Spiel. g4emu kennt `continue, wait, press, tap, walk, walk-to, expect-map, where, shot, state, write-save, dump-ram`. pokeheartgold `include/constants/flags.h:1394`: `TRAINER_FLAG_BASE 0x550`, das nächste Flag ist `0x82E`. Die HGSS-Trainer-Flags reichen also für **734** Trainer, es gibt aber 738 Datensätze (Risiko 1).

---

## 1. Umfang und Schnitt

**Entscheidung:** Zwei Pläne. Jeder endet mit einer lauffähigen App und einem eigenen Gate.

**3b.3a „Pokémon-Datenbank“** (Kernel-Fundament und drei Editoren)
- Alle Codecs aus §2, auch Trainer und Begegnungen. Diese beiden sind in 3b.3a nur gelesen und per Round-Trip getestet; ihre UI kommt in 3b.3b.
- Namenstabellen, Item-Tabelle und TM-Liste aus arm9 (nur lesen).
- Pokémon-Icons und Item-Icons aus dem ROM.
- Referenzindex v1 über die Datenquellen (§6).
- Atlas: Pokémon und Gegenstände werden freigeschaltet, mit Unterbereichen (§4).
- Editor **Pokédex**: Reiter Werte, Fähigkeiten und Typen, Entwicklung, Lernset, TM/VM, Fundorte, Rohdaten. Die Screens „Pokédex“ und „Entwicklung und Lernset“ gehen hier zusammen auf.
- Editor **Attacken**.
- Editor **Gegenstände**: Karte, Werte, Rohdaten, „Verwendet in“ für Trainer und Pokémon-Wild-Items.
- Namen umbenennen (Art, Attacke, Gegenstand), mit Undo in derselben Gruppe.
- Gate 3b.3a:
  - (A) `scripts/gate.sh` mit dem neuen gated Round-Trip über alle Member der neuen Familien in drei Projekten, ohne gesperrtes Asset.
  - (B) In einer Platin-DE-Kopie: Typ 2 des Starters ändern, eine Attacke umbenennen samt Stärke, einen Gegenstand umbenennen samt Beschreibung. Belegt durch g4emu-Screenshots von Bericht, Kampfmenü und Beutel. Typen liest der Bericht live aus Personal.
  - (C) Galerie mit den neuen Primitives, hell und dunkel.

**3b.3b „Trainer und Begegnungen“** (Weltbezug und Roadmap-Gate)
- Editor **Trainer** mit Team-Karten, Klasse, KI, Gegenständen, Kampftexten, Sichtlinie.
- Assistent **„Neuer Trainer“** mit fünf Schritten im Scharnier.
- Editor **Begegnungen** mit Situationen, Platzkarten und Rate.
- Neue Begegnungsdatei anlegen, am Header anbinden und abspalten.
- Front-Sprites der Pokémon und Kampf-Sprites der Trainer.
- `MapPeek` als Draufsicht auf Kartenbildern.
- Referenzindex v2 über Events und Header.
- Inline-Prüfungen.
- Gate nach §9.

**Später:**

| Was | Wann | Grund |
|---|---|---|
| Läden, Ei-Attacken Pt, Attacken-Lehrer, Honigbaum, Pt `encdata_ex`, HGSS Kopfnuss/Safari/Zertrümmern-Items | nach 3b.4 als **3b.5 „Overlay-Tabellen“** oder in 3d | liegen in arm9/Overlays, Pt und HGSS verschieden, brauchen eine Patch-Schicht |
| Ei-Attacken HGSS (`a/2/2/9`) | 3b.5, zusammen mit Pt | nicht als halbe Funktion nur für ein Spiel |
| TM/VM-Zuordnung, Klassen-Preisfaktor und -Geschlecht schreiben | 3b.5 | arm9-Slices; in 3b.3 nur lesen und anzeigen |
| Pokédex-Größe, -Gewicht, -Kategorie (Zukan) | 3b.5 | Ablage ungeprüft; die Schau zeigt sie bis dahin nicht |
| Projektweite Prüfungen im Panel Probleme | 3b.4 (Roadmap) | 3b.3 zeigt Prüfungen nur im offenen Editor und im Prüflicht |
| Tiefe Erzählen, Rezepte, Live-Vorschau des Assistenten in 3D | 3d | 3c-Festlegung; Begegnungen kommen in Bauen (§7) |
| „Auf Original zurücksetzen“ gegen das Basis-ROM | Phase 4 | in 3b.3 heißt „vorher“ der **gespeicherte Stand** (aus `saved_bytes`) |

**Verworfen:**
- *Ein Plan.* 3c hat gezeigt, dass Rahmen plus Editoren für einen Plan zu groß sind. Ein fehlerhafter Trainer-Teil würde dann auch den Pokédex blockieren.
- *Schnitt nach Schichten* (3b.3a nur Kernel, 3b.3b nur UI). 3b.3a wäre ohne sichtbare Software und ließe sich nicht im Spiel prüfen.

**Preis bei Irrtum:** Ist 3b.3a zu groß, verschiebt sich das Roadmap-Gate um einen Plan. Die Codecs sind aber fertig, und 3b.3b schrumpft entsprechend.

---

## 2. Codecs

**Entscheidung:** Eigene Codecs in `g4-formats` für alle Familien, mit einem gemeinsamen Muster **„Rohbytes plus Felder“**.

Module in `crates/g4-formats/src/data/`: `encounter_dppt.rs`, `encounter_hgss.rs`, `trainer.rs` (Properties und Party), `personal.rs`, `evolution.rs`, `learnset.rs`, `move_data.rs`, `item.rs`, dazu `item_table.rs` und `tm_table.rs` (arm9-Slices, nur lesen).

Muster:
- Jeder Typ hält `raw: Vec<u8>` (das gelesene Member) und typisierte Felder.
- `write()` kopiert `raw` und patcht nur die Felder hinein. Padding, ungenutzte Blöcke (Pt-Encounter `0x0F8..0x124`), Füllbytes der Level-Slots, EV-Bits 12–15 und unbekannte KI-Bits bleiben dadurch ohne eigenen Code erhalten.
- Variable Formate (Lernset, Party) bauen neu und füllen auf ein Vielfaches von 4 Byte auf. Bei der Party ist ein leeres Team 8 Byte lang.
- Lesen wirft nie wegen unbekannter Werte. Ein Wert außerhalb der Vanilla-Grenzen bleibt erhalten und erscheint in der UI als „Wert 212 (unbekannt)“. Das ist das Muster aus DSPRE-Avalonia `MoveDataEditorViewModel`.
- uxie bleibt Dev-Dependency und dient als Test-Orakel für Personal, Items und Trainer-Properties: dieselben Felder wie uxie.

Round-Trip-Garantie, drei Stufen:
- `crates/g4-formats/tests/vanilla_data.rs`, gated mit `G4_TEST_PROJECTS` (alle drei Projekte, ein Lauf in `gate.sh`). Für jedes Member jeder Familie gilt `write(read(x)) == x`.
- Zusätzlich die **No-op-Bearbeitung**: jedes Feld auf seinen eigenen Wert setzen, dann ist `write == x`.
- Zusätzlich die **Feld-Isolation**: Feld F auf einen anderen gültigen Wert setzen; dann unterscheiden sich nur die Bytes von F, und erneutes Lesen liefert den neuen Wert.
- Kernel-Test: Jedes Member der neuen Arten lädt im Store ohne `locked`. `unwritable_reason` bleibt als Netz für modifizierte ROMs.
- Build-Test: Ein unverändertes Projekt baut nach dem Laden aller Daten-Assets byte-identisch (vorhandener gated Test, um die neuen Arten erweitert).

**Unterschiede je Spiel** (über `GameFamily` und die Spielkennung):

| Familie | DPPt | HGSS |
|---|---|---|
| Encounter | 424 B. Species u32. Gras: 12 × {Level u32, Species u32}. Wasser-Slot {**max**, min, pad u16, species u32}. Ersatz: Schwarm (Slots 0, 1), Tag/Nacht (2, 3), Radar (4, 5, 10, 11), GBA (8, 9), Formraten, Icognito | 196 B. Raten 6 × u8. 12 Level gemeinsam, Species Morgens/Tags/Nachts je 12 × u16. Radio Hoenn/Sinnoh (2, 3 / 4, 5). Wasser {**min**, max, species u16}. Zertrümmern × 2. `landSwarm`, `surfSwarm`, `nightFish` (kein Schwarm!), `fishSwarm` |
| Header → Encounter | `wild_pokemon` u16, 0xFFFF = keine | u8, 0xFF = keine, höchstens 255 Dateien |
| Party-Eintrag | DP 6/8/14/16 B, Pt 8/10/16/18 B. Byte 0–1 als u16 `ivScale` | 8/10/16/18 B. Byte 0 Stärke, Byte 1 Geschlecht/Fähigkeit |
| Personal, Evolution (44 B), Lernset, Move (16 B), Item (34 B) | gleich | gleich |

DP läuft nur über den Code, ohne Gate (kein Projekt).

**Verworfen:**
- *uxie-Typen plus Nachbearbeitung.* Encounter-Leser und -Schreiber und der Move-Codec sind für Vanilla unbrauchbar, Party, Evolution und Lernset brauchen Padding-Hüllen. Außerdem lehnt uxie beim strikten Lesen unbekannte Bits ab, und gehackte ROMs würden dann gesperrt.
- *uxie forken und patchen.* Das bringt einen Fork-Unterhalt, und ein Upstream-Merge ist ungewiss. Melden ja, abhängig machen nein.

**Preis bei Irrtum:** Etwa 1500 Zeilen eigener Code statt Hüllen. Die gated Tests fangen Layoutfehler vor dem ersten Speichern.

---

## 3. Asset-Modell im Store

**Entscheidung:** Ein Asset bleibt ein NARC-Member (Undo und Dirty je Asset wie heute). Darüber kommt eine Undo-**Gruppe**: `Changeset` erhält `group: u64`. `Store::transact(label, |tx| …)` vergibt eine Gruppen-ID für alle `apply_edit`s darin. `undo` und `redo` nehmen alle Changesets der obersten Gruppe. `history()` zeigt eine Zeile je Gruppe. Speichern, Stale-Prüfung und Dirty bleiben je Asset.

Neue `AssetKind`s als `Location::Member`: `Encounter`, `TrainerProperties`, `TrainerParty`, `Personal`, `Evolution`, `Learnset`, `MoveData`, `ItemData`, `TrainerTextTable`, `TrainerTextOffsets`. Dazu `ItemTable` und `TmTable` als `Location::Slice` in arm9, nicht editierbar.

Ein Kernel-Modul `entities/` setzt Sichten zusammen, ohne eigenes Asset:
- `TrainerView` = Properties, Party, Name (Archiv 618/729), Kampftexte (`trtbl` + Archiv 617/HGSS-Bank laut Metadaten).
- `PokemonView` = Personal, Evolution, Lernset, Name.

`update_trainer(id, patch)` läuft als eine Transaktion, ein ⌘Z nimmt alles zurück. Umschalten von „eigene Attacken“ setzt das Flag-Byte und formatiert die Party in **derselben** Gruppe. Die Wahrheit ist das Flag-Byte; die Party-Einträge tragen `Option`-Felder.

**Namen:**
- `metadata/text_banks.yaml` je (Familie, Sprache): species, moves, move_desc, items, item_desc, abilities, types, trainer_names, trainer_classes, trainer_texts. Die IDs stehen in Bericht 0.3; die Pt-Itembeschreibung ist ungeprüft (391/393) und wird im Plan an Daten entschieden.
- `NameCache` im Store liest über `read_text_doc`, damit ungespeicherte Textänderungen sichtbar sind. Er wird je Archiv invalidiert, wenn das Archiv bearbeitet wird.
- Das Frontend holt `data_names(kind)` einmal und erneut beim Bus-Thema `edited` für dieses Archiv.
- Umbenennen = `edit_text_archive` in derselben Gruppe.
- Kennungen wie `SPECIES_*` erzeugt die App **nicht**. In Bauen steht „Nr. 397“, in Tüfteln Index dez/hex und Pfad.

**arm9-Tabellen:**
- Item-Tabelle und TM/VM-Liste werden gelesen (Offsets je Spielkennung in `metadata/arm9_tables.yaml`, Bericht 4.3) und nicht geschrieben.
- Die Gegenstände-Liste ist nach **Item-ID** geschlüsselt. Teilen sich mehrere IDs ein Data-Member, zeigt die Hand den Hinweis „Teilt die Werte mit Gegenstand X“.

**Pfade je Spiel** in `archives.yaml`: optionales Feld `by_game: { IPG: "files/a/1/3/6", APA: "…" }`, Schlüssel sind die ersten drei Zeichen von `game_code`. `ArchiveMap::for_game` bekommt den Spielcode. Das braucht einen Test mit synthetischer SS-Kennung.

**Verworfen:**
- *Zusammengesetztes Asset* `Trainer` mit `Location::Pair`. Dann wären Save, Stale-Prüfung, `reload_from_disk` und die DSPRE-Ordner für zwei Dateien plus Text zu duplizieren.
- *Ganzes Archiv als ein Asset* (wie `HeaderTable`). Jede Bearbeitung legte 928 Trainer in ein Changeset, und Dirty je Datensatz ginge verloren.

**Preis bei Irrtum:** Die Gruppe ist ein additives Feld. Fehlt sie, gibt es halbe Undos (Party ohne Flag), und die Codecs würden ein inkonsistentes Paar schreiben. Deshalb prüft ein Kernel-Test: Undo einer Trainer-Gruppe stellt beide Member byte-genau wieder her.

---

## 4. Navigation im Doppeldeck

**Unterbereiche:**
- Oben in der Liste steht ein **Unterbereich-Raster** `SubAreaGrid`: Kacheln mit 2 × 2 je 128 × 36 px, jeweils Symbol, Name und Zähler.
- Pokémon: „Pokédex 493“, „Trainer 928“, „Begegnungen 183“, „Attacken 467“.
- Gegenstände: „Gegenstände 468“ und „Läden“ (gesperrt, Tooltip „Kommt später“).
- Dazu ⌘1–⌘4, Palette-Einträge, und das zweite Brotkrumen-Glied öffnet dasselbe als `Menu`.
- Der Zustand (Unterbereich, Auswahl, Hand-Höhe) liegt in `shellState` unter dem Schlüssel `bereich.unterbereich`.
- Die Wunsch-Hand-Höhe je Unterbereich aus den Screens ist der Standardwert: 286 / 400 / 380 / 300.

| Unterbereich | Liste | Schau | Hand |
|---|---|---|---|
| Pokédex | Suche „Name oder Nummer“, Typ-`FilterChips`, `ListRow` mit Icon, Name, Nr., Ungespeichert-Punkt | Profil: `DSScreen variant="pokedex"` (Front-Sprite ab 3b.3b, vorher Icon), Kopf mit `TypeBadge`, 6 × `StatBar` mit Schnitt-Strich und „vorher“, „Fundorte“ als `Reference`-Chips. Im Reiter Entwicklung: die Kette als `PokemonCard compact` mit Pfeilen | Reiter (`Tabs` mit Punkt): **Werte** (Basiswerte, EV-Ertrag als sechs kleine Zahlfelder, Fang-Rate mit Satz, Wachstum, Geschlecht als Slider „nur ♂ … nur ♀ / geschlechtslos“, Ei-Gruppen 2, Schlüpfen), **Fähigkeiten und Typen** (je zwei), **Entwicklung** (bis 7 Zeilen, Methode als Satz „ab Level 14“, „mit Item Leuchtstein“), **Lernset** (`DataTable`, Budget „12 von 20“), **TM/VM** (Raster aus `Checkbox`, Tooltip „TM40 Aero-Ass“, „Wie Staralili“), **Rohdaten** (nur Tüfteln, `HexBlock`) |
| Attacken | Suche, Typ-Chips, Kategorie-Select | Karte mit `MetricCard` × 3, Merkmalen, Wirksamkeit (berechnet aus fester Typentabelle in `metadata/types.yaml`, kein ROM-Bezug in 3b.3), `DSScreen variant="summary"` mit Budget je Zeile | Spalten Grundwerte, Verhalten (Ziel als Satz, Merkmale als Checkboxen), Tüfteln: Effekt-ID, Flags, Hex |
| Gegenstände | Taschen-Chips mit Zähler, Preis rechts | Karte, Beschreibungs-`DSScreen`, Kennzahlen, „Verwendet in“ (Trainer, Wild-Items; Läden und Funde später, sichtbar als „Funde und Läden: kommt mit 3b.4/3b.5“) | Preis, Tasche, Kampf/Beutel, Schleuder, Tüfteln-Hex. Die Party-Wirkung erscheint als Satz („Heilt 20 KP“), Bearbeiten nur in Tüfteln |
| Trainer | Baum `Nach Ort` / `Nach Klasse` (`Segmented`), „Ohne Ort“; Level rechts | links `DSScreen variant="battle"` (Klassen-Sprite, Textbox „Wanderer Kai möchte kämpfen!“), rechts `MapPeek` mit Sichtlinie | links „Team“ mit 6 × `PokemonCard`, ein Klick öffnet die Platz-Details als `Popover` (Art per Inline-`Picker`, Level, Item, 4 Attacken, Stärke); rechts Trainer-Spalte mit `InspectorRow`s |
| Begegnungen | Orte mit Encounter-Datei (aus `buildPlaces`), „Ohne Ort“ für unbenutzte Dateien | `MapPeek` (Gras/Wasser schraffiert, Flächen „Gras A · 28 Kacheln“), Legende „Wer im Gras wartet“ | §7 |

**Tiefen:** Bauen ist der Standard und zeigt Sätze statt Zahlen. Tüfteln ergänzt den Reiter beziehungsweise die Spalte „Rohdaten“ (Dateiname, Eintrag, `HexBlock` mit markiertem Feld) und Dez/Hex in den Zahlfeldern.

**Neue Primitives in `ui/primitives/`:** `TypeBadge`, `StatBar`, `PokemonCard` (Varianten `team`, `slot`, `compact`, `empty`), `Tabs` (Base UI Tabs, Punkt und Zähler), `DataTable` (Kopf, Zeilenwahl per Roving, Inline-Zelle über vorhandene Felder; ohne Virtualisierung, 928 Zeilen gehen so), `FilterChips` (Toggle Group plus „n weitere“ als Popover), `Slider` (Base UI), `MetricCard`, `HexBlock`, `Budget` (Text „5 von 7“ mit Mini-Balken, Warnfarbe ab Grenze), `SubAreaGrid`, `MapPeek` (Rahmenbaustein). `Picker` bekommt `variant: "dialog" | "inline"` und `hint` je Treffer, `DSScreen` die Varianten `pokedex`, `battle`, `summary`, `description`.

Durch 3c-Bausteine ersetzt, also keine eigene Komponente: RecipeButton → `Button` mit `hint`; TM-Raster → CSS-Grid aus `Checkbox`; Mini-Kurve → Satz „1.059.860 EP bis Level 100“; „100 % verteilt“ → `Notice`; Checkbox-Matrix und Orden-Segmente entfallen mit den Läden.

**Verworfen:**
- *Atlas-Unterpunkte.* Sieben Einträge plus vier Unterpunkte passen nicht in 652 px Höhe.
- *Nur das Brotkrumen-Menü.* Für Einsteiger unauffindbar.

**Preis bei Irrtum:** Das Raster kostet 80 px Listenhöhe. Fällt es durch, wird es zur Reiterzeile (eine Komponente).

---

## 5. Sprites aus dem ROM

**Entscheidung:** Neue Leser `g4-formats/src/{ncgr,nclr,ncer_min}.rs` und `pokegra.rs` (Entschlüsselung). Gerendert wird in Rust zu PNG. Der Tauri-Befehl `sprite_png(kind, id, variant)` liefert die Bytes; der Cache liegt unter `.g4/cache/sprites/`, Schlüssel ist der Hash der Member-Bytes. Die Brücke nutzt die vorhandene Antwortart `bytes()`.

| Was | Quelle (zu prüfen im Plan) | Plan | Rückfall |
|---|---|---|---|
| Pokémon-Icons 32 × 32 (Listen, Karten, Slots) | Pt `poketool/icongra/pl_poke_icon.narc`, HGSS `a/0/2/0`; 4 bpp, unverschlüsselt; Paletten-Index je Art aus einer Tabelle (Ort ungeprüft) | **3b.3a** | Kreis in Typfarbe mit Initiale |
| Item-Icons 24 × 24 | `item_icon.narc` / `a/0/1/8`; Icon- und Paletten-Index aus der arm9-Item-Tabelle (schon gelesen) | **3b.3a** | Taschen-Symbol (Phosphor) |
| Front-Sprites 80 × 80 | Pt `pokegra/pl_pokegra.narc`, HGSS `a/0/0/4`; verschlüsselt (Richtung je Spiel ungeprüft), sechs Dateien je Art | **3b.3b** | Icon vergrößert |
| Trainer-Kampfsprites | `trgra/trfgra.narc` / `a/0/5/8`, je Klasse | **3b.3b** | Klassen-Symbol |
| Kampfhintergrund, Spielfigur-Größenvergleich | – | nein | `DSScreen`-Fläche in Token-Farben |

Testdaten: gated Test „jede Art liefert ein PNG ohne Fehler“, dazu ein Hash-Vergleich von drei Sprites je Projekt gegen von Hand geprüfte PNGs außerhalb des Repos (Agent-Screenshot im Gate). Keine Grafik kommt ins Repo.

**Verworfen:**
- *Alles in 3b.3a.* Die Entschlüsselung ist das größte Unbekannte und würde den Pokédex blockieren.
- *Rendern im Frontend per Canvas.* Doppelter Code für die Galerie und kein Cache.

**Preis bei Irrtum:** Ohne Icons bleibt die UI zahlenarm, aber farbig-abstrakt; ein Einsteiger erkennt Arten dann nur am Namen. Gegenmaßnahme: Die Icons sind Task 1 der UI-Hälfte von 3b.3a.

---

## 6. Referenzen

**Entscheidung:** Kernel-Modul `refs.rs` mit `RefIndex`.
- Es bildet `(TargetKind, id)` auf `Vec<RefUse { source: AssetId, role, place: Option<HeaderId> }>` ab.
- Es wird beim ersten Abruf auf einem Arbeitsthread gebaut; Fortschritt kommt als Toast `progress`.
- Es ist je Quell-Asset gespeichert: `apply_edit`, `undo`, `redo` und `reload_from_disk` markieren das Quell-Asset; der nächste Abruf scannt nur dieses neu.
- Befehle: `refs(kind, id) -> RefsDto { groups: [{role, label, items: [{label, goto}]}], count }` und `ref_counts(kind) -> Vec<u32>` für die Listen-Zähler.

| Quelle → Ziel | Plan |
|---|---|
| Lernset, Evolution → Attacke, Art, Item | 3b.3a |
| Personal → Item (Wild-Item), Fähigkeit, Typ | 3b.3a |
| Trainer-Party → Art, Item, Attacke; Properties → Klasse, Item | 3b.3a (gelesen) |
| Encounter → Art | 3b.3a (gelesen, für „Fundorte“) |
| Header `wild_pokemon` → Encounter-Datei (Ort) | 3b.3a |
| Event (Objekt mit `trainer_type` ≠ 0 oder Script 3000–4999 / 5000–6999) → Trainer, Ort | **3b.3b** |
| Scripts (Kampfbefehle, `GiveItem`) → Trainer, Item | **3b.4**, über die rotom-Quellen, mit „mindestens“-Semantik |

**Löschsperre:**
- In 3b.3 gibt es kein Löschen von Trainern oder Arten.
- „Begegnungen vom Ort lösen“ ist erlaubt.
- Eine Datei ohne Nutzer wird nicht gelöscht, sondern als „unbenutzt“ für Wiederverwendung angeboten.
- Ein Trainer mit „Verwendet in“ = 0 trägt das Badge „unbenutzt“.
- Prüfungen wie „Trainer ohne Team“ oder leere Ersatz-Slots stehen in 3b.3 inline und im Prüflicht der aktuellen Auswahl; projektweit kommen sie mit 3b.4.

**Verworfen:**
- *Scan bei jedem Abruf.* 928 + 508 Member pro Klick sind zu langsam für Listen-Zähler.
- *Persistenter Index auf der Platte.* Invalidierung bei DSPRE-Änderungen außerhalb des Studios ist fehleranfällig.

**Preis bei Irrtum:** Falsche Zähler verführen zu falschen Schlüssen („unbenutzt“). Darum gibt es einen Test: Vanilla-Pt-Trainer 1 hat Event-Treffer, und eine Art aus Route 201 hat einen Fundort.

---

## 7. Begegnungen

**Entscheidung:** Ausgeliefert in **Bauen** mit dem Inhalt des Erzählen-Screens. Platzkarten, Tageszeit und Rate sind das Einsteiger-Layout; Tüfteln ergänzt die Tabelle.

**Hand, links:**
- **Methoden-Reiter** mit Zählern:
  - Pt: Gras, Surfen, Alte Angel, Profiangel, Superangel.
  - HGSS: Gras, Surfen, Zertrümmern, drei Angeln.
  - Leere Methoden sind gedämpft und zeigen den Schalter „Kommt hier vor“. Er setzt die Rate auf den Median der Vanilla-Raten dieser Methode und bietet „Wie Ort X übernehmen“ an.
- Darüber die **Situation**:
  - `Segmented` Tageszeit (Pt: Morgens = Grundliste | Tags | Nachts; HGSS: Morgens | Tags | Nachts).
  - Dazu `FilterChips` für Sonderfälle: Schwarm, Poké-Radar, GBA-Slot (Pt); Radio Hoenn, Radio Sinnoh, Schwarm, Nachtangeln (HGSS).
- **Platzkarten** (`PokemonCard slot`):
  - Sie zeigen immer, **was der Spieler in dieser Situation trifft**.
  - Ersetzte Plätze tragen die Plakette „ersetzt durch Schwarm“, Kobalt gestrichelt.
  - Bearbeiten einer ersetzten Karte schreibt in das Ersatzfeld; der Untertitel sagt es: „Du bearbeitest den Schwarm-Platz“.
  - Inhalt: Art (Inline-`Picker` im Popover, Treffer mit Hinweis „sonst Level 2–4 hier“), Level.
  - Gras: ein Level (HGSS: gilt für alle Tageszeiten, Hinweis).
  - Wasser: „Level von … bis …“ in zwei Feldern. Der Codec ordnet je Familie (DPPt max/min, HGSS min/max).
  - Prozente kommen aus `metadata/encounter_odds.yaml` (Gras 20/20/10/10/10/10/5/5/4/4/1/1, Wasser je Familie). Sie werden nur angezeigt; die Werte im Plan werden an pret geprüft.
- **Leere Slots** (Art 0, Level 0): gestrichelte Karte „leer“.
  - Bei Rate > 0 erscheint `Notice warning` „Platz 5 ist leer: Hier erscheint ein Glitch-Pokémon. Wähle ein Pokémon.“ Das gilt auch für leere Ersatz-Slots, wenn der Sonderfall aktiv werden kann.
  - min > max wird nie gespeichert: Das zweite Feld klemmt mit.
  - Level 0 bei belegter Art ist ein Fehler, der nicht übergeben wird.

**Hand, rechts:**
- „Wie oft passiert etwas?“: `Slider` selten…oft. Darunter steht der Rohwert klein („Rate 30“), in Tüfteln als `NumberField` 0–255.
- Der Satz „jeder n-te Schritt“ erscheint erst, wenn der Plan die Formel aus pret belegt hat; bis dahin „selten / mittel / oft“.
- `Button`s mit Hinweis:
  - „Wie Ort X verteilen“ (Picker über Orte, kopiert die Methode).
  - „Seltenes Pokémon einbauen“ (setzt Platz 11 und 12).
- „Gilt auch für“ listet alle Header mit derselben Datei; dazu der Knopf „Für diesen Ort abspalten“ (dupliziert und bindet um, eine Gruppe).

**Schau:**
- `MapPeek`: Kartenbilder des Headers aus `createTopDownRenderer` mit größerer `size` (256 je Karte), darüber ein 2D-Canvas mit Gras- und Wasserkacheln aus den Permissions, Flächen nach Zusammenhang beschriftet.
- Legende „Wer im Gras wartet“ für die aktuelle Situation.
- Ein Klick auf eine Fläche wählt die Methode.
- Keine zweite three.js-Szene, kein Draufsicht-Modus.

**Neue Datei:**
- Ort ohne Begegnungen: In der Liste („Orte ohne Begegnungen“ eingeklappt) und im Header-Inspector der Welt steht „Wilde Pokémon: keine · Anlegen“.
- `create_encounter_file { header, copy_from? }` hängt ein Member an (leer oder Kopie) und bindet `wild_pokemon` in einer Gruppe. Das Anhängen ist wie bei `create_event_file` nicht rückgängig; Undo löst nur die Bindung, die Datei bleibt als „unbenutzt“.
- HGSS ab Index 255: Ablehnung „HeartGold kann höchstens 255 Begegnungsdateien ansprechen“, mit Angebot „Unbenutzte Datei 0098 verwenden“.

**Verworfen:**
- *DSPRE-artige Tabelle aller Felder als Standard.* Zahlen und Ersatzlogik im Kopf des Nutzers.
- *Erzählen-Segment mit Rezepten jetzt.* Widerspricht 3c §7.7; zwei Inspector-Varianten.

**Preis bei Irrtum:** Das Situationsmodell ist neu. Missversteht der Nutzer es, bearbeitet er Ersatzfelder statt der Grundliste. Gegenmaßnahme: Die Situation ist nach jedem Ortswechsel „Tags, keine Sonderfälle“, und der Untertitel nennt das Zielfeld.

---

## 8. Trainer

**Party:**
- Schalter „Eigene Attacken“ und „Gegenstände tragen“ je Trainer.
- Abschalten verwirft die Werte in derselben Gruppe; ein Toast „Attacken von 3 Pokémon entfernt“ bietet Rückgängig an.
- Ohne eigene Attacken zeigt die Karte „Attacken nach Level“, berechnet aus dem Lernset (App-Logik).
- „Stärke“ als Slider schwach…stark (Byte 0, IV = d·31/255).
- HGSS Byte 1: Geschlecht und Fähigkeit (Auto / 1 / 2) als Select.
- Pt: nicht angeboten, roh in Tüfteln als „IV-Skala, oberes Byte“.
- Form nur bei Arten mit Formen.
- Doppelte Attacken: Warnung.

**KI:**
- `Segmented` „einfach | klug | Experte“ auf die drei häufigsten Vanilla-Werte. Die Zuordnung zählt der Plan an den drei Projekten aus; Kandidaten sind 1 / 7 / 15.
- Jeder andere Wert zeigt „eigene Einstellung“, und Tüfteln zeigt elf benannte Bits plus Rohwert.
- Unbekannte Bits bleiben erhalten.

**Klasse:**
- `SelectField` mit Name und Sprite.
- Belohnung als **berechneter Text** „720 ₽ = 40 ₽ × Level 18 · gilt für alle Wanderer“.
- Geschlecht und Preisfaktor nur gelesen (arm9/Overlay-Offsets aus Metadaten; wo nicht verifiziert, ausgeblendet).

**Name:**
- `TextField` mit `Budget` „3 von 7“.
- Mehr als 7 Zeichen wird nicht übergeben: „Höchstens 7 Zeichen; längere Namen brauchen einen arm9-Patch.“
- Die Grenze steht als Wert je Spiel in Metadaten.

**Kampftexte:** Bearbeitbar direkt in der Hand („Vor dem Kampf“, „Niederlage“, „Nach dem Kampf“) über `trtbl`, `trtblofs` und das Textarchiv, dazu „In Texte öffnen“.

**Sichtweite und Blickrichtung:**
- Sie gehören dem Event. Die Trainer-Hand zeigt sie, wenn genau ein Event auf den Trainer zeigt, und schreibt in die Eventdatei in derselben Gruppe.
- Bei mehreren Events steht „An 2 Orten, Sichtweite je Ort in der Welt“.
- Dass `data[0]` die Sichtweite ist, prüft der Plan mit g4emu.

**„Neuer Trainer“** als **Assistent** im Scharnier (3c §5.5), weil er fünf Stellen schreibt und einen Ort braucht. Schritte:
1. **Ort und Figur**: Kachel im `MapPeek` oder in der Welt-Szene, Blickrichtung, Figur aus einem OW-Sprite-Picker mit Vorschlag je Klasse; „Später platzieren“ ist erlaubt.
2. **Klasse und Name.**
3. **Team**: Inline-Picker; Vorschlag „Level passend zum Ort“ aus wilden Levels und anderen Trainern am Ort, Taste „Level n für alle“.
4. **Texte**: drei Felder mit Vorlage.
5. **Zusammenfassung** mit Plakette „Nicht rückgängig“.

`create_trainer(spec)` schreibt Properties, Party, Name-Nachricht (Index = Trainer-ID), `trtblofs`, `trtbl` mit den Kampftext-Nachrichten und ein Objekt-Event mit `trainer_type = 1` und `script = 3000 + id − 1`.

Die **Trainer-ID** wählt der Kernel: das erste Ende **unterhalb der Flag-Kapazität** (Risiko 1); wenn es keines gibt, wird ein Datensatz wiederverwendet, der leer ist (Team 0, keine Referenz). „+“ in der Trainer-Liste startet denselben Assistenten.

**Verworfen:**
- *„Neu“ + leeres Formular.* Ein Trainer ohne Event, Text und Tabelleneintrag stürzt ab oder ist unsichtbar. Der Einsteiger weiß nicht, welche fünf Dinge nötig sind.
- *Klassen-Preisfaktor als Trainer-Feld* wie im Screen. Das ROM speichert ihn je Klasse, und eine Bearbeitung würde still alle Trainer der Klasse ändern.

**Preis bei Irrtum:** Fehlt die Texttabelle oder ist sie falsch, hängt das Spiel beim Kampf. Gegenmaßnahme: Das Gate prüft genau das in g4emu.

---

## 9. Gate und Prüfbarkeit

**Vorbereitung (einmalig, automatisch):**
- Szenario `pt-team-spielstand.yaml` (und `hg-…`) auf einer Wegwerfkopie: Level-Script `on_var_condition` im Zimmer (Pt 415, HG 64) gibt per rotom-Script ein Pokémon und setzt die Variable.
- Bauen, `continue`, warten, dann `write-save` nach `_saves/{CPUD,IPKD}-team.sav`.
- Grund: Die Zimmer-Spielstände haben vermutlich kein Pokémon, und ohne Team gibt es weder Wild- noch Trainerkämpfe. Ungeprüft; der Plan prüft das zuerst.

**Szenario `pt-trainer-begegnungen.yaml`** (Platin DE; HG DE analog mit Route 29):
1. `update_event` biegt den Treppen-Warp des Zimmers auf Route 201 (Header 342), wie in `pt-raum-assistent.yaml`.
2. Auf der Testkopie werden die Story-Auslöser am Graseingang von Route 201 gelöscht (Trigger-Events, `delete_event`).
3. `update_encounters { file: <wild_pokemon von 342>, method: grass, rate: 255, slots: alle 12 = Art A (z. B. 25), Level 5 }`.
4. `create_trainer` zweimal (`${NEUER_TRAINER_1}`, `_2`):
   - Klasse beliebig, je ein Pokémon mit **eigenen Attacken**, Arten B und C mit bekannten Attacken.
   - Event auf Route 201 an Kachel (x, z) mit Blick auf den Weg, Sichtweite 3.
5. `build`, dann `play`:
   - `continue`, `walk-to` vor die Sichtlinie von Trainer 1, einen Schritt hinein.
   - `expect-mon B m1 m2 m3 m4 600`, `shot trainer1`.
   - Danach `press a` bis zum Kampfende (Schritt `fight-until-over`, siehe unten), dann Trainer 2 ebenso.
   - Zum Schluss `walk-until-battle up 40` im Gras, `expect-mon A`, `shot wild`.

**Neue g4emu-Schritte:**
- `expect-mon SPECIES [M1..M4] [TIMEOUT]`: sucht im Haupt-RAM alle N Frames das unverschlüsselte Kampfmuster (u16 Art, 10 Byte frei, 4 × u16 Attacken). Das Muster leitet sich aus `BattleMon` in pret ab; der Plan prüft es mit `dump-ram` und `find-u16`. Für Wildkämpfe nimmt das Szenario die Attacken, die der Kernel aus dem Lernset berechnet (Bridge-Befehl `default_moves(species, level)`).
- `walk-until-battle DIR N`: geht hin und her, bis `expect-mon` mit beliebiger Art greift oder N Schritte erreicht sind.
- `fight-until-over`: drückt A, bis der Ort wieder lesbar ist (der Ort-Zeiger ist im Kampf gesetzt, aber `where` wechselt; ungeprüft).
- `expect-mon` beweist Art und Attacken; der Screenshot belegt den Namen in der Textbox für den Menschen.

**Neue Brücken-Befehle:** jeder neue Tauri-Befehl in `bridge/dispatch.rs`; `commands_match_the_tauri_handler` zählt ihn mit.
- 3b.3a: `data_names`, `pokemon_list`, `pokemon`, `update_pokemon`, `learnset`, `update_learnset`, `evolutions`, `update_evolutions`, `moves_list`, `move_data`, `update_move`, `items_list`, `item`, `update_item`, `rename_entry`, `refs`, `ref_counts`, `icon_png`, `tm_table`, `default_moves`.
- 3b.3b: `trainers_list`, `trainer`, `update_trainer`, `create_trainer`, `trainer_classes`, `encounter_list`, `encounters`, `update_encounters`, `create_encounter_file`, `bind_encounters`, `split_encounters`, `sprite_png`, `delete_event` (falls noch nicht vorhanden).

**Abnahme 3b.3b:**
- (A) Szenario grün auf CPUD und IPKD.
- (B) Zwei Trainer per Assistent **in der UI** (Agent über die Test-Umgebung, Rezept in `docs/agent-gate.md`) und Begegnungen per Platzkarten, dann „Hier spielen“; der Nutzer sieht die Kämpfe in melonDS.
- (C) DSPRE öffnet das Projekt und zeigt die Trainer (Hand-Punkt, darf ausgelassen werden).
- (D) `gate.sh` grün.

**Verworfen:**
- *Nur Screenshots.* Sie sind nicht maschinell prüfbar.
- *Feste RAM-Adresse der Gegner-Art.* Sie liegt auf dem Heap und ist je Szenario instabil.

**Preis bei Irrtum:** Trifft das Muster nicht, fällt das Gate auf Screenshot plus Blick zurück. Ein halber Tag für `dump-ram` und `find-u16` ist im Plan eingeplant.

---

## 10. Risiken

1. **Trainer-Flags reichen nicht für neue Trainer.**
   - HGSS hat Platz für 734 Trainer-Flags (0x550..0x82D), es gibt schon 738 Datensätze. Ein angehängter Trainer teilt sein „besiegt“-Flag mit einem Story-Flag: Er kämpft dann nie oder setzt Story-Zustand. Für Pt ist die Kapazität ungeprüft.
   - *Gegenmaßnahme:* `metadata/trainer_flags.yaml` je Familie mit `base` und `capacity` (HGSS aus pret, Pt im ersten Plan-Task aus pokeplatinum `generated/` oder per g4emu-Flag-Dump). `create_trainer` wählt nur IDs unter der Kapazität, sonst einen leeren, unbenutzten Datensatz (Referenzindex), sonst eine Ablehnung mit Erklärung. Das Gate prüft den Kampf.
2. **Neue Trainer brauchen Texttabellen, die ungeprüft sind** (`trtbl`/`trtblofs`, HGSS-Bank). Ein fehlender Eintrag liest außerhalb der Tabelle, und das Spiel hängt.
   - *Gegenmaßnahme:* eigener Codec mit gated Round-Trip, Layout zuerst an pret-Headern belegen, dann g4emu-Kampf als Pflichtschritt des Gates. Ohne grünen Kampf wird nicht gemerged.
3. **Umfangsdruck durch die Screens** (Läden, Ei-Attacken, Größenvergleich, Erzählen, Live-Vorschau).
   - *Gegenmaßnahme:* Die Liste „Später“ (§1) wird in die Spec übernommen. Gesperrte Unterbereiche und Reiter zeigen „Kommt später“ statt zu fehlen. Jede UI-Zahl, die das ROM nicht trägt (Belohnung, „jeder n-te Schritt“), ist berechneter Text oder entfällt.
4. **Sprite-Formate** (Verschlüsselung, Paletten-Zuordnung der Icons) kosten mehr als gedacht.
   - *Gegenmaßnahme:* Icons in 3b.3a mit Rückfall-Kreis, Front- und Trainer-Sprites erst in 3b.3b, beide mit Platzhalter. Kein Gate-Punkt hängt an einem Sprite.
5. **Personal-Fallen und Indexe:**
   - Die Default-Form einiger Arten liegt angeblich in einem anderen Archiv; Namenszahl ist nicht Datenzahl (496/508, 468/471, Item-ID ≠ Data-Index).
   - *Gegenmaßnahme:* Auswahl hängt immer an der ID, nie am Listenindex. Formen-Member (496–507) erscheinen als „Deoxys (Angriffsform)“ mit Metadaten-Tabelle. Arten aus der Liste im Changelog bekommen `Notice` „Das Spiel liest diese Form womöglich woanders“, bis der Plan es an einem Projekt geprüft hat. Für geteilte Item-Data-Member gibt es den Hinweis aus §3.

Nebenrisiko: 3c ist noch nicht gemerged. 3b.3a beginnt erst nach dem Merge von 3c.2; die Kernel-Hälfte von 3b.3a (Codecs, Store, Referenzen) kann parallel zu 3c.2 laufen, weil sie kein `ui/` braucht.
