# Gutachten A: Weltmodell ohne Matrix, Header und Raster in „Bauen“

Stand: 25.10.2026. Gutachter: Architektur-Review (nur lesend). Repo: `/Users/POLTEP/Projekte/lab/editor/gen4-studio`.
Datenbasis: die drei Vanilla-Projekte unter `/Users/POLTEP/Projekte/lab/editor/Roms/*_DSPRE_contents` (Platin DE `CPUD`, HeartGold DE `IPKD`, HeartGold US `IPKE`), nur gelesen. Header aus `arm9/arm9.bin` (Offsets aus `crates/g4-kernel/metadata/header_tables.yaml`), Layout nach uxie 0.9.7 `src/map_header/binary.rs`; Matrizen aus `unpacked/matrices`; Ortsnamen aus `g4k headers` und dem Textarchiv 433 (Pt) bzw. 279 (HGSS). Die Auswerteskripte liegen in `docs/research/2026-10-25-ausbau/analyse.py`, `analyse2.py`, `analyse3.py` und `textdec.py`. HeartGold US gleicht HeartGold DE in allen Zählungen, deshalb steht unten nur „HGSS“.

---

## 1. Was heute was ist (Technik)

| Ding | Technisch | Wo im Code | Fakten aus den Daten |
|---|---|---|---|
| **Header** | 24-Byte-Eintrag der Ortstabelle, Pt und HGSS mit verschiedenen Layouts (uxie `MapHeaderPt`, `MapHeaderHGSS`). Liegt fest in `arm9.bin`. Mit dem Patch „Dynamische Header“ gibt es eine Datei je Header in `unpacked/dynamicHeaders`. Interner Name: 16 Byte je Header in `files/fielddata/maptable/mapname.bin`; die Zahl der Header ist die Zahl dieser Einträge | `crates/g4-kernel/src/project.rs` (`HeaderSummary` Z. 366, `read_headers` Z. 893, `mapname_count` Z. 590), `crates/g4-kernel/src/dynamic_headers.rs` | Pt 593, HGSS 540 |
| **Matrix** („Raster“) | Gitter aus Karten-IDs, 1×1 bis 255×255. Optional ein Header-Abschnitt (u16 je Zelle) und ein Höhen-Abschnitt (u8 je Zelle) | `docs/formats/matrix.md`, `crates/g4-formats/src/matrix.rs` | Pt 289, HGSS 288. **Einen Header-Abschnitt haben nur je zwei**: Pt M0 (30×30, „map“) und M240 (4×5, Großmoor); HGSS M0 (47×17) und M212 (5×4, Safari) |
| **Karte** | `MapFile`: 32×32 Kacheln mit Kollision, Gebäuden, Modell (NSBMD) und BDHC | `docs/formats/map.md` | Pt 656, HGSS 657 benutzte Karten. Nur 2 (Pt) bzw. 1 (HGSS) Karte steckt in mehr als einer Matrix (Pt Karte 176 in M0 und M240) |
| **Area-Data** | Satz der Texturen für Karte und Gebäude, eine Nummer im Header | `docs/formats/area_data.md` | – |
| **Eventdatei** | Warps, Figuren, Trigger und Schilder eines Headers | uxie `EventFile` | Datei 0 teilen sich 60 (Pt) bzw. 50 (HGSS) Header, sonst hat jeder Header seine eigene |
| **Ortsname** | `location_name` im Header, **u8** in Pt und HGSS (u16 nur in DP): Index in die Ortsnamen-Bank des Textarchivs (uxie `TextBankIds::locations`: Pt 433, Pt JP 427, Pt KR 428, HGSS 279, JP 272, KR 274). Gelesen wird der Member, nicht die JSON-Quelle (`project.rs` Z. 875–890) | `project.rs` `location_names`, `header_listing_for` | Pt: 126 Namen, 125 davon benutzt (nur 92 „GTS“ nicht). HGSS: 235 Namen, nur 121 benutzt. **Index 1–125 in HGSS sind die Sinnoh-Namen** (Zweiblattdorf, Sandgemme, …, Gipfelruinen), weil derselbe Index als „Fundort“ im Pokémon steckt; Johto und Kanto beginnen bei 126 (Neuborkia). Längster Name in beiden: 16 Zeichen („Wettbewerbshalle“; Kürzungen wie „Schleiede-Kaufh.“ deuten auf eine feste Grenze) |
| **Ortstyp** | Pt `location_specifier`, HGSS `location_type` (`HeaderSummary.map_type`) | `project.rs` Z. 405 | Nach den Daten: 1 = Stadt (Pt 20, HGSS 32), 2 = Route/Außen (71/64), 3 = Höhle/Dungeon (166/116), 4 = Gebäude (334/326), 6 = Untergrund, 0 = `NOTHING` |
| **HGSS-Extras** | `worldmap_x/y` (Lage auf der Pokégear-Karte), `kanto_flag` | uxie `MapHeaderHGSS` | 199 Header mit Kanto-Flag, 132 Header mit (0, 0) |

**Wie die App das heute abbildet** (`apps/studio/src/workbench/places.ts`, `worldMap.ts`):
- *Ort* sind die Header mit gleichem Ortsnamen-**Text** (`buildPlaces`, Z. 31). Die Orte stehen in der Reihenfolge ihrer ersten Header-ID, „Ohne Ortsnamen“ kommt zuletzt.
- *Außen* oder *Raum* entscheidet allein `matrix.has_headers` (`places.ts` Z. 48).
- Die *Weltkarte* zeigt genau eine Matrix: die mit Header-Abschnitt und den meisten belegten Zellen (`regionMatrix`, `worldMap.ts` Z. 11).
- Ein Raum hängt nur über einen einzigen Warp-Schritt an einer Zelle der Region (`roomLinks`).
- In Bauen heißen Räume nach ihrem internen Namen (`WorldExplorer.tsx` Z. 103: `h.internal_name`). Im Baum steht also „C01PC0102“.
- Der Header-Editor zeigt „Ortsname“ als nackte Zahl (`inspector/fields.ts` Z. 53, 70, 90).

### 1.1 Kombinationen in Vanilla

| Befund | Pt | HGSS |
|---|---|---|
| Zellen der Region mit Header 0 (`EVERYWHERE`), also Landschaft, Meer und Ränder | 299 von 468 belegten Zellen in M0 | 291 von 492 |
| Header, die auf M0 zeigen, aber keine Zelle haben („verwaist“) | 17: `NOTHING`, `D05`, `D06`, `D07`, `D09`, `D11`, `D17`, `D19A/B`, `R226A/B`, `R227A/B`, `R230`, `R232`, `W229`, `W233`, alle mit „Mysteriöser Ort“ | 21: `NOTHING`, `UG`, `D24`, `D24R0201`, `R00`, `W20R0101` und die Sinnoh-Reste `C11*` „Kampfareal“, `D31*` „Duellpark“ und „Duellturm“. Die meisten haben Ortstyp 4 (Gebäude) |
| Orte mit mehr als einem Außen-Header | Route 204, 205, 210, 211, 212 (A/B-Hälften), Großmoor 6, „Mysteriöser Ort“ 18 | Route 2, 10, 16, 20, Alph-Ruinen 3, Kampfareal 9, Duellturm 4, Duellpark 2, „Mysteriöser Ort“ 4 |
| Orte **nur aus Räumen** (kein Außen-Header) | **62 von 125**: Kraterberg (14 Header, je eigene Matrix), Siegesstraße, Zerrwelt, Höhle der Umkehr, aber auch Gebäude in Städten mit eigenem Namen (Jubelstadt TV, Pokétch Ges., Trainerschule, Bergbaumuseum) | **44 von 121**: Glockenturm, Radioturm, Steineichenwald, Siegesstraße … |
| Außenbereich in einer Matrix **ohne** Header-Abschnitt (mehrzellig, Ortstyp 1/2) | `D32` Kampfzone (M256 3×3), `D31` Duellpark (M229 2×2) | `D32` Kampfzone (M230 3×3), `D22R0101–03` Nationalpark (M27 3×3, drei Header teilen sie), `D49` Pokéathlon (M153), `P01R0102` (Matrix `m_port02_`) |
| Teilkarten mit Header-Abschnitt | **Großmoor** M240: 6 Header (`D06R0201–06`) mit je einer Zelle, dazu 14 Zellen mit Header 0. Karte 176 liegt auch in M0 | **Safari** M212: 13 Zellen mit Header 0 und eine mit `D47R0102`. Die Gebiete `SAF01–SAF14` haben eigene Matrizen 197–210; das Spiel setzt die Safari-Zone selbst zusammen |
| Räume teilen Matrix und Karte | 38 Matrizen gehören zu mehr als einem Header. Spitzen: M123 (Häuser, 33 Header), Pokémon-Center M116/M117/M235 (je 17–18) | 27 Matrizen; M77 und M73 Pokémon-Center (25 bzw. 24), M66 Haus (19), Tore M69/M70 (15/14) |
| Mehrzellige Räume | 63 (Höhlen 2×2 bis 3×3, `UG` 15×15) | 67 |
| Matrizen ohne jeden Header | 19 | 8 |
| Johto und Kanto | – | **Eine Matrix** (M0). Die Trennung steckt nur im `kanto_flag` der Header: in den M0-Zellen sind 41 Header Kanto und 35 Johto, und ihre x-Bereiche überlappen (Johto x 1–45, Kanto x 22–44) |
| Sinnoh und Kampfzone | Die Kampfzonen-Insel (Route 225–230, Überlebensareal, Erholungsgebiet) liegt **in M0**. Nur die Kampfzone selbst (`D32`) ist eine eigene 3×3-Matrix ohne Header-Abschnitt | – |

### 1.2 Wo die Hierarchie 1 : n : n bricht

1. **Landschaft gehört zu keinem Gebiet.** Rund 60 % der belegten Weltkartenzellen tragen Header 0. In der Liste stehen sie heute als Ort „Mysteriöser Ort“.
2. **Verwaiste Header** auf M0 gelten in `places.ts` als „Außen“, haben aber keine Fläche. In HGSS trifft das sogar Gebäude (`C11PC0101`).
3. **Ein Ort hat mehrere Außen-Header.** Route 204 A/B, die 6 Großmoor-Header: Ein Gebiet besteht dann aus mehreren Headern (Abschnitten).
4. **Räume hängen über den Namen nicht am Gebiet.** 62 (Pt) und 44 (HGSS) Orte haben gar kein Außen. Gebäude in Städten tragen einen eigenen Ortsnamen (Jubelstadt TV steht nicht unter Jubelstadt). Dungeon-Etagen erreicht man nur über andere Räume (Kraterberg: Raum → Raum → …).
5. **Außen ohne Header-Abschnitt.** Kampfzone, Duellpark und Nationalpark zählen heute als „Raum“.
6. **Teilkarten mit Header-Abschnitt** (Großmoor, Safari-Vorlage) wären nach einer naiven Regel „Weltkarten“.
7. **Geteilte Räume.** Wer die Karte von „Pokémon-Center Jubelstadt“ ändert, ändert alle 17 bis 25 Pokémon-Center.
8. **Räume nur per Script erreichbar** (etwa `DIRECT2/4`, `UNION`). Der Warp-Graph findet für sie keinen Eingang.
9. **HGSS: zwei Regionen in einer Matrix.**
10. **Neue Gebiete aus dem Assistenten** bekommen eine eigene Matrix mit Header-Abschnitt (`wizards/wizardSpecs.ts` Z. 89, `withHeaders: true`). Nach der Regel „Weltkarte = Matrix mit Header-Abschnitt“ würde jede neue Route zur eigenen Weltkarte.

**Wie man damit umgeht** (die Regeln für die Variante B unten):
- *Weltkarte* ist eine Matrix mit Header-Abschnitt, deren Zellen **mindestens zwei verschiedene Ortsnamen** (ohne Index 0) tragen. Pt ergibt Sinnoh (M0), HGSS ergibt Johto und Kanto (M0). Großmoor und Safari erfüllen das nicht, sie werden *Gebiete mit eigener Karte*. Eine Projektdatei kann die Wahl überstimmen (siehe §3).
- *Gebiet* ist auf einer Weltkarte die Menge der Zellen, deren Header denselben Ortsnamen trägt. Die beteiligten Header heißen in Tüfteln „Abschnitte“ (Route 204 hat zwei). Dazu kommen **eigenständige Gebiete**: eine Matrix ohne Header-Abschnitt mit Ortstyp 1 oder 2 und mehr als einer Zelle (Kampfzone, Nationalpark) und eine Matrix mit Abschnitt, die nur einen Ortsnamen trägt (Großmoor, Safari, neue Routen aus dem Assistenten).
- *Raum* ist jeder andere Header mit Karten. Er gehört zum Gebiet, **in dem sein Eingang liegt**: eine Breitensuche im Warp-Graphen (`list_warps`) über Räume hinweg bis zur ersten Gebietszelle. Gibt es mehrere Kandidaten, gewinnt der mit gleichem Ortsnamen, sonst der kürzeste Weg. Findet die Suche keinen Eingang, gilt ein Gebiet mit gleichem Ortsnamen. Bleibt auch das leer, landet der Raum unter „Nicht verbunden“.
- *Landschaft* (Zellen mit Header 0) ist eine neutrale Fläche der Weltkarte und taucht in der Liste nicht auf.
- *Verwaiste Header* (Abschnitt, aber keine Zelle, kein Eingang) und Header ohne Karten stehen nur in Tüfteln, unter „Sonstiges“. Ortsname-Index 0 („Mysteriöser Ort“) gilt in Bauen als „kein Ort“.
- *Geteilte Räume* tragen die Plakette „Geteilt mit 17 Räumen“ und bekommen die Aktion „Für diesen Raum abspalten“ (Muster wie „Für diesen Ort abspalten“ bei den Begegnungen).

---

## 2. Empfehlung (a): Begriffs- und Datenmodell

| Nutzerwort | Technische Entität | Erzählen | Bauen | Tüfteln |
|---|---|---|---|---|
| **Weltkarte** | Matrix mit Header-Abschnitt und ≥ 2 Ortsnamen; in HGSS eine Ansicht von M0 mit Filter Johto, Kanto oder Alle (`kanto_flag`) | „Sinnoh“, „Johto“ | Wurzel im Baum, Reiter „Weltkarte“ mit Umschalter der Weltkarten | zusätzlich „Matrix 0000“, Abschnitte, Größe |
| **Ort** | Ortsname: ein Index in der Ortsnamen-Bank (`location_name`) | Name, der im Spiel erscheint (Einblendung beim Betreten, Fundort) | Etikett und Farbe an Gebiet und Raum, Picker im Inspector, „Neuer Ort…“ | Index und Bank (Archiv 0433, Nachricht 17) |
| **Gebiet** | Zellen einer Weltkarte mit Headern gleichen Ortsnamens (1..n Header) oder eine eigenständige Außen-Matrix | „Route 204“ | zweite Ebene im Baum, Fläche auf der Weltkarte, 3D-Kamera rahmt sie | Header-Liste als „Abschnitte“, Matrix-Zellen |
| **Raum** | Header auf einer Matrix ohne Abschnitt (Gebäude, Höhle, Etage) | „Haus der Mutter“ | dritte Ebene unter dem Gebiet seines Eingangs, Klarname (editorseitig), Plakette „Geteilt“ | Header-ID, internes Kürzel, Matrix |
| **Karte** | `MapFile` in einer Zelle | – | Auswahl in 3D und auf der Weltkarte, Inspector „Karte“ (Höhe, Teile importieren) | Karten-ID, Zelle (x, y), Rohdaten |
| **Eingang / Ausgang** | Warp-Paar | „Tür“ | wie heute | Warp-Index, Ziel-Header |
| **Aussehen** | Area-Data | – | Picker „Aussehen wie …“ mit Kartenbild | Area-Data-Nummer |
| (verborgen) | Eventdatei | – | nur als „Figuren und Türen dieses Ortes“ | Eventdatei-Nummer |
| **Header** | `MapHeader` | – | **nicht sichtbar** (nur „Neu anlegen“ im Hintergrund) | voll sichtbar, Header-Editor, Liste |
| **Raster / Matrix** | Matrix | – | **nicht sichtbar** | Reiter „Raster“ (heutiges `MatrixPanel`) |
| **Landschaft** | Zellen mit Header 0 | – | neutrale Fläche, Karte wählbar, kein Gebiet | „Header 0000 EVERYWHERE“ |

Das Glossar (`workbench/terms.ts`) bekommt die Schlüssel `worldMap`, `area`, `place`, `room`, `map` und `landscape`. Die Texte von `matrix` und `header` gelten nur noch in Tüfteln. Grob 37 sichtbare Zeichenketten mit „Raster“ oder „Matrix“ (unter anderem in `app/historyLabel.ts`, `app/statusContext.ts`, `app/paletteEntries.ts`, `workbench/ExtendAreaDialog.tsx`, `wizards/wizardSpecs.ts` Z. 221/233/242/544, `inspector/headerSections.ts` „Karten und Raster“, `checks/messages.ts` Z. 108 f.) und 34 Stellen mit `TERMS.matrix` sind umzustellen.

---

## 3. Varianten (b)

### Variante A „Orte bleiben oben“ (nach dem Namen, klein)
Der heutige Baum (Ort › Header) bleibt. Darüber kommt die Ebene „Weltkarte“ (über die Zellen des Ortes), „Außen“ heißt jetzt „Gebiet“, Räume bekommen Klarnamen, und der Reiter „Raster“ wandert nach Tüfteln.
- **Vorteile:** deterministisch, kaum Rechenlogik, folgt dem Namen, den das Spiel zeigt; Aufwand klein (vor allem `places.ts`, `WorldExplorer.tsx`, `terms.ts`).
- **Nachteile:** 62 (Pt) und 44 (HGSS) Orte ohne Gebiet stehen lose auf oberster Ebene. Gebäude mit eigenem Namen stehen nicht unter ihrer Stadt. Die 1 : n : n-Sicht des Nutzers ist nur angenähert, Dungeons mit 14 Etagen werden eine flache Liste.

### Variante B „Welt physisch, Ort als Etikett“ (**recommend**)
Der Baum lautet Weltkarte › Gebiet › Räume (Räume über den Warp-Graphen, auch über mehrere Schritte), Regeln wie in §1.2. Der Ort ist ein Etikett mit Farbe an Gebiet und Raum und dient als Filter. Die Weltkarte zeigt mehrere Weltkarten. Eigenständige Gebiete stehen unter „Weitere Gebiete“, nicht Verbundenes unter „Nicht verbunden“, Verwaistes nur in Tüfteln. Eine kleine Projektdatei `g4-world.json` hält nur, was das ROM nicht weiß: Namen der Weltkarten, Klarnamen der Räume, manuelle Zuordnung „Raum gehört zu Gebiet …“, auf Wunsch eine Weltkarte als Wahl.
- **Vorteile:** entspricht dem Bild des Nutzers („das Haus steht in Zweiblattdorf“). Funktioniert mit allen Vanilla-Sonderfällen und mit Räumen aus dem Assistenten (ihr Eingang entsteht gleich mit). Fügt sich in Prinzip 4 (zwei Sprachebenen) und den Audit-Punkt 9.6 „Klarnamen für Räume“ (`design-system/Audit-Design-System.md`).
- **Nachteile:** Die Zuordnung ist berechnet und springt, wenn man einen Warp ändert; Mehrdeutigkeiten (Tore zwischen Route und Stadt) brauchen eine Regel und einen manuellen Override. Mehr Logik und Tests, eine zweite Datei im Projekt.

### Variante C „Struktur im Projekt“ (editorseitig, frei)
Weltkarten, Gebiete und Ordner stehen vollständig in `g4-world.json`. Der erste Start füllt sie einmal nach B, danach gehört die Struktur dem Nutzer (Ziehen und Ablegen, Ordner, Tags).
- **Vorteile:** stabil und frei strukturierbar, auch nach Kapiteln.
- **Nachteile:** eine zweite Quelle der Wahrheit, die vom ROM abweicht (DSPRE legt Header an, das Studio legt Header an, Warps ändern sich). Mehr Bedienung und Migrationen. Kollidiert mit Phase 4 (Textprojekt mit symbolischen IDs) und mit Prinzip 7 („Assistenten erzeugen normale Objekte“).

**Empfehlung: B**, mit dem Ortsetikett aus A und nur dem Override-Teil aus C. Die Struktur kommt aus dem ROM, die Projektdatei korrigiert nur.

### Zu Frage 2: eigene Orte (Empfehlung: im ROM, keine Ordner)
„Neuer Ort…“ legt einen **echten Ortsnamen im ROM** an. Ordner und Tags kommen nicht dazu.

Was „Neuer Ort“ braucht:
1. Eine Nachricht an die Ortsnamen-Bank anhängen. Die Bank kommt aus `TextBankIds` je Sprache (Pt 433, HGSS 279). Das Anhängen gibt es seit 3b.2 im Store, als Transaktion wie in 3b.3a.
2. Prüfen, dass der neue Index ≤ 255 bleibt (u8). **Pt hat 130 freie Plätze (126 bis 255), HGSS nur 21 (235 bis 255).** HGSS kann zusätzlich die Sinnoh-Namen 1–125 wiederverwenden, die kein Header nutzt. Folge: Pokémon aus Sinnoh, die per Transfer oder Pal Park kommen, zeigen dann den neuen Namen als Fundort. Das braucht einen Warnsatz.
3. Länge: Warnung ab 17 Zeichen; das Vanilla-Maximum ist 16.
4. Den Header umhängen: `location_name` setzen, für mehrere Header in einem Undo-Schritt. Vorschlag: das Gebiet und seine Räume per Kästchen. Das Gebietssymbol (`area_icon`, der Stil der Einblendung) kommt von der Vorlage.
5. Die Listen zeigen den neuen Namen sofort. Heute liest `location_names` den **Member auf Platte** (`project.rs` Z. 885), ein ungespeicherter Name fehlt dort. Der Store muss die Namen aus dem eigenen Stand liefern.

Grenzen:
- Die Stadtkarte bzw. Pokégear-Karte und die Flugziele stehen nach unserem Wissen in Code-Tabellen der Overlays, nicht im Header. Im Repo ist das **nicht belegt**, dafür braucht es einen eigenen Recherche-Posten. Ein neuer Ort erscheint dort also nicht. HGSS hat immerhin `worldmap_x/y` im Header (Lage auf der Pokégear-Karte), das man pflegen kann.
- Der Ortsname ist zugleich Fundort im Pokémon-Bericht. Ein Umbenennen trifft alle Header mit diesem Index.

Warum keine Ordner: Eine dritte, rein editorseitige Hierarchie neben Weltkarte › Gebiet › Raum und dem Ort verwirrt. Strukturieren lässt sich über Orte (im Spiel sichtbar, in Pt praktisch unbegrenzt) und über Klarnamen der Räume (editorseitig). Bleibt der Wunsch nach reinen Arbeitsordnern („Kapitel 2“), lässt sich ein Tag-Feld in `g4-world.json` später ergänzen, ohne das Modell zu ändern.

### Zu Frage 3: Sortierung
- **Baum:** Gebiete je Weltkarte „Spielreihenfolge“ (Standard, die heutige Reihenfolge nach erster Header-ID), „Name A–Z“ (`localeCompare('de', {numeric: true})`, damit „Route 2“ vor „Route 10“ kommt) und „Zuletzt gewählt“. Räume im Gebiet nach Klarname oder in Spielreihenfolge. „Nicht verbunden“ und „Sonstiges“ stehen immer zuletzt.
- **Liste** (`HeaderList.tsx`): Nummer (Standard), Name, Kürzel.
- **Merken:** in `g4.shell.v1` als optionales Feld `worldSort: { tree: "game" | "name" | "recent", list: "id" | "name" | "code" }`. Das folgt dem Muster von `sub` und `selected` (`apps/studio/src/ui/frame/shellState.ts`), ohne Migration. Bedient wird es über ein `Menu` im `PanelHead` neben Baum und Liste.
- „Zuletzt **bearbeitet**“ genau je Header geht nicht: Alle Header sind ein Asset (`HeaderTable`). Für v1 wird es „Zuletzt gewählt“, je Projekt in `localStorage` (Schlüssel mit Projektpfad, nicht in `g4.shell.v1`).

### Zu Frage 4: Reiter „Raster“ abschaffen (in Bauen)

Was `MatrixPanel.tsx` heute leistet und wohin es geht:

| Heute im Raster | Neu |
|---|---|
| Gitter mit Kartenbildern und Ortsfarben (`AreaGrid`) | **Weltkarte** (gibt es schon, 32 px). Wird ein Raum gewählt, zeigt der Reiter den **Raumplan** (die eigene Matrix des Raums, 64 px; 63/67 mehrzellige Räume) |
| Zelle klicken öffnet den Inspector „Karte“ (`MatrixCellEditor`: Karte, Header, Höhe, Teile importieren/exportieren) | 3D: Auswählen-Werkzeug auf Boden ohne Marker wählt die Karte, die Szene zeigt einen Kobalt-Rahmen um die Zelle. Weltkarte und Raumplan: Klick wählt das Gebiet, ⌥Klick oder Doppelklick die Karte. Im Inspector heißt „Header“ dann „Gehört zu: Gebiet …“ (Picker) |
| Doppelklick öffnet den Header | Doppelklick öffnet das Gebiet |
| „+“-Zeile und -Spalte (`ExtendAreaDialog`) | Weltkarte: freie Nachbarzellen des gewählten Gebiets zeigen „+“; Raumplan: „+“ am Rand; 3D: optional geisterhafte Felder am Rand des Gebiets |
| Höhen | Feld „Höhe“ im Inspector „Karte“ (gibt es schon); die Szene zeigt sie seit 3b.1 |
| Name, Breite, Höhe, Schalter Header und Höhen, „Neue Matrix“ | **Tüfteln**: Reiter „Raster“ unverändert. Das Zielbild sieht genau das vor (Screen „Raster: Matrix 0 (Tüfteln)“, `design-system/README.md`) |
| Notiz „nicht rückgängig“ | Plakette „Nicht rückgängig“ am „+“ |

Dazu in 3D:
- Die Kamera rahmt das gewählte Gebiet oder den gewählten Raum (Audit §8 und §9.7: heute zeigt sie das ganze M0 von weit oben).
- Geladen werden nur das Gebiet und ein Ring Nachbarzellen. Heute lädt `useSceneLoader.ts` alle Karten der Matrix, in M0 also 468 bzw. 492 Karten.
- Die **Draufsicht** (orthografisch, steht im Design-System) ist billig und hilft bei der Auswahl der Karte. Ich empfehle sie als Umschalter in der Szenenleiste, obwohl die Roadmap sie nach Phase 6 legt.

Umbau:
- Der Reiter `matrix` bleibt als Panel und wird nur in Tüfteln eingeblendet. Das braucht Dockview `addPanel`/`removePanel` beim Wechsel der Tiefe, `isSaneLayout` je Tiefe und Layout-Schlüssel `v9`.
- Brotkrume („Zelle 7, 25“ wird „Karte 0412“), Statusleiste („Raster 0, Zelle 3, 27“ wird „Sinnoh, Karte 3, 27“) und die Palette („Rasterzelle“) ziehen mit.
- `data-cell` und `data-extend` bleiben für die Test-Umgebung erhalten.
- **Schätzung: mittel bis groß**, etwa 1.800–2.500 Zeilen mit Tests: Picking und Rahmen in 3D, Kamera und Teil-Laden, Weltkarte mit Auswahl und „+“, Raumplan, Layout v9.

### Zu Frage 5: Dynamische Header und Patches

**Heute:**
- Assistent, Schritt „Header“ (`wizards/HeaderChoice.tsx`): drei Spalten. Der Schalter „Vorhandenen Header umstellen | Neuer Header“ steht auf „Vorhandenen“; „Neuer“ ist gesperrt, bis der Patch drin ist.
- Ohne Patch steht ein Warnkasten mit „Patch anwenden“ **im Schritt**. Bei ungespeicherter Header-Tabelle lehnt er ab: „Assistent abbrechen, speichern (⌘S) und neu öffnen“ (`useHeaderPatch.ts`, `SAVE_FIRST_IN_WIZARD`), denn der Assistent sperrt ⌘S. Der Kernel verlangt eine saubere Tabelle und vergisst ihren Verlauf (`store.rs` Z. 1359–1379).
- Danach folgen ein interner Name (A–Z, 0–9, _), Area-Data als Zahl und die Eventdatei mit drei Segmenten. Beim Wiederverwenden kommen Heuristiken dazu („wirkt unbenutzt“, Sonder-Header).
- Technik › Patches (`technik/TechnikArea.tsx` Z. 301 ff.): ein Patch, eine Plakette, „Patch anwenden“ mit Rückfrage, „Reparieren“.

**Was umständlich ist:**
- Die Patch-Entscheidung kommt mitten im Formular, und der Weg führt durch Speichern und Neuöffnen.
- Der Standard „Vorhandenen umstellen“ verlangt Wissen über freie Header.
- Interner Name, Area-Data und Eventdatei sind Technikfragen in Bauen.
- Der neue Header erbt Ortsname, Musik und Wetter der Vorlage, ohne dass man es sieht.

**Leichterer Weg:**
1. In Bauen heißt der Schritt „Ort“ statt „Header“. Er fragt: „Zu welchem Ort gehört das? [Ort ▾] [Neuer Ort…]“ und „Aussehen wie …“ (Picker mit Kartenbild statt Area-Data-Zahl). Der Header ist **immer neu** (Kopie der Vorlage). Der interne Name entsteht automatisch (zum Beispiel aus Ortskürzel und laufender Nummer) und ist nur in Tüfteln änderbar. Die Eventdatei ist neu und leer. „Vorhandenen Header wiederverwenden“ gibt es nur in Tüfteln.
2. Der Patch wird **beim ersten Bedarf still vorbereitet**. Die Zusammenfassung zeigt dann eine Zeile mit Plakette „Nicht rückgängig“: „Einmalig für dieses Projekt: Spielcode für neue Orte anpassen (DSPRE-kompatibel, Sicherung arm9.bin.backup)“. Sie hat ein Kästchen, das „Anlegen“ freigibt. Ein Aufruf erledigt Patch und Anlegen. Die Zustimmung je Projekt ist der angewendete Patch selbst; ein zweites Mal wird nie gefragt.
3. Der Kernel wendet den Patch auch bei **ungespeicherter Header-Tabelle** an. Er schreibt den Stand der Platte in die Dateien, wechselt den Ort des Assets, und die Änderungen im Speicher bleiben ungespeichert und rückgängig machbar. Damit entfällt „speichern und neu öffnen“.
4. Vor dem Patch entsteht ein Schnappschuss (Prinzip 8). Heute wird nur `arm9.bin` gesichert, der vorhandene Ordner `unpacked/dynamicHeaders` (Vanilla-Member des NARC, Pt 12 Dateien) wird ersetzt (`dynamic_headers.rs` `apply` und `write_header_folder`).
5. Technik › Patches bleibt die Übersicht: Zustand, wann angewendet, Sicherung, Reparatur. Der Satz dort („Neue Orte brauchen eigene Header…“) passt schon.

### Zu Frage 6: mehrere Weltkarten
- **Finden:** Matrizen mit `has_headers` aus `list_matrices`. Weltkarte ist, was ≥ 2 Ortsnamen ungleich 0 in belegten Zellen trägt; die übrigen sind Gebiete mit eigener Karte. In HGSS teilt `kanto_flag` M0 in zwei Ansichten: die Zellen der Header mit Flag sind Kanto, Landschaftszellen (Header 0) gehen an die nächste benannte Zelle. Namen: für M0 je Familie „Sinnoh“ bzw. „Johto“ und „Kanto“, sonst der häufigste Ortsname der Zellen. `g4-world.json` kann umbenennen und eine Matrix zur Weltkarte machen oder abwählen.
- **Zeigen:**
  - Reiter „Weltkarte“ mit Umschalter im `PanelHead` (Menü wie die Unterbereiche in Pokémon): „Sinnoh“ und „Weitere Gebiete“ (Pt); „Johto“, „Kanto“, „Johto und Kanto“ und „Weitere Gebiete“ (HGSS).
  - „Weitere Gebiete“ ist eine Kachelfläche der eigenständigen Gebiete (Kampfzone, Großmoor, Safari, neue Routen mit eigener Matrix) mit ihrem Raumplan.
  - Im Baum sind die Weltkarten die Wurzeln.
  - `regionMatrix` (`worldMap.ts`) wird zu `worldMaps(): WorldMap[]`.
- **Neue Gebiete:** Der Assistent fragt „Wo liegt das Gebiet?“: „Auf der Weltkarte Sinnoh, an freier Stelle (nahtlos begehbar)“ oder „Eigene Karte (über Türen verbunden)“, letzteres ist das heutige Verhalten. Pt M0 hat 432, HGSS 307 leere Zellen. Die erste Möglichkeit braucht einen neuen Kernel-Befehl (§4).

---

## 4. Kernel und Backend (c)

| Neu oder geändert | Zweck | Ort |
|---|---|---|
| `Store::location_names()` aus dem Store-Stand (Textarchiv-Asset, ungespeicherte Nachrichten eingeschlossen); `header_listing_for` nutzt sie | neue Orte sofort sichtbar | `crates/g4-kernel/src/store.rs`, `project.rs` Z. 950 |
| `create_place(name) -> u8`: Nachricht an die Ortsnamen-Bank anhängen, Index ≤ 255 prüfen, sonst deutsch ablehnen | „Neuer Ort…“ | neu, Kernel und `src-tauri/src/commands/world.rs` oder `headers.rs` |
| `rename_place(index, name)`, `assign_place(headers: Vec<u16>, index)` als Transaktion (ein Undo-Schritt, `Store::transaction`) | Ort umbenennen und umhängen | dito |
| `free_place_slots()`: freie Indizes. HGSS liefert auch die Sinnoh-Plätze 1–125 ohne Header, mit Kennzeichen „Fundort-Name aus Sinnoh“ | Picker und Warnung | dito |
| `apply_dynamic_headers` auch bei ungespeicherter Tabelle (Asset rebasen statt ablehnen); Schnappschuss von `unpacked/dynamicHeaders` vor dem Ersetzen | Patch ohne „erst speichern“ | `store.rs` Z. 1359, `dynamic_headers.rs` `apply` |
| `AssignHeader.allow_patch: bool` in `create_area` und `create_room`: Patch und neuer Header in einem Aufruf; `AssignHeader.place: Option<PlaceChoice { Existing(u8), New(String) }>`; interner Name optional (automatisch) | Assistent in einem Schritt | `apps/studio/src-tauri/src/commands/wizards.rs` Z. 44–66, 255–330 |
| `create_area` mit Ziel `Existing { matrix, x, y }`: Karten in freie Zellen einer vorhandenen Weltkarte setzen, Header-Zellen auf den neuen Header (Prüfung „Zellen frei“) | nahtloses Gebiet | `wizards.rs` |
| `split_room(header)`: Matrix und Karte für einen Header verdoppeln und umstellen (nicht rückgängig, wie `duplicate_map`) | geteilte Räume abspalten | `wizards.rs` oder neu |
| `world_meta_get` / `world_meta_set` für `<projekt>/g4-world.json`: `{ version: 1, worldMaps: [{ matrix, name, filter? }], notWorldMaps: [u16], roomNames: { "<id>": "…" }, roomOwner: { "<id>": "<gebiet-key>" } }`, robust gelesen wie `g4-checks.json` (eine kaputte Datei hält das Projekt nicht zu, Ruling R15) | editorseitige Korrekturen | neu, Muster `checks` |
| Prüfungen: `place.name-too-long` (Warnung > 16, Quelle Vanilla), `place.index-out-of-bank` (Fehler, 0 Treffer auf allen drei Projekten: Pt max 125 < 126, HGSS max 234 < 235), `world.orphan-header` (Hinweis), `room.shared-map` (Hinweis beim Bearbeiten) | Sicherheit | `crates/g4-kernel/src/checks/`, `check_limits.yaml` |
| Dokument `docs/formats/map_header.md` (fehlt): Layout Pt und HGSS, `location_name` u8, `area_icon`, `kanto_flag`, `worldmap_x/y`, Ortstyp-Werte, Ortsnamen-Bänke | Grundlage | `docs/formats/` |

Ohne neue Befehle bleiben (alles im Frontend, rein und testbar): Weltkarten-Erkennung, Gebietszellen, Warp-Breitensuche, Klarnamen-Heuristik für Räume aus dem Kürzel (`PC0101` Pokémon-Center, `PC0102` erster Stock, `FS` Pokémarkt, `GYM` Arena, `R0101` Haus 1, bei `D…R01nn` „Ebene n“). Alle Daten liefern `list_matrices`, `list_warps`, `headers` und `get_matrix`. Vorschlag: ein neues `workbench/worldModel.ts` neben `places.ts`.

---

## 5. Risiken (d)

1. **Berechnete Zuordnung springt.** Ändert man einen Warp, wandert ein Raum. Gegenmittel: stabile Reihenfolge, `roomOwner`-Override, Hinweis „gehört jetzt zu …“.
2. **Versteckte Teilung.** Pokémon-Center und Häuser teilen Matrix und Karte mit bis zu 33 Headern. Wer in Bauen „diesen Raum“ bearbeitet, ändert alle. Plakette „Geteilt“ und „Abspalten“ sind Pflicht, bevor Bauen die Technik verbirgt.
3. **Ortsnamen sind knapp in HGSS** (21 freie Plätze). Das Wiederverwenden von Sinnoh-Namen ändert Fundorte transferierter Pokémon.
4. **Stadtkarte und Flugziele** zeigen neue Orte nicht. Das sorgt für enttäuschte Erwartungen und braucht Recherche und vielleicht Patches.
5. **Heuristiken auf Fremd-Hacks.** DSPRE-Projekte mit eigenen Matrizen samt Header-Abschnitt können als Weltkarte gelten oder nicht. Gegenmittel: `g4-world.json` und Tüfteln.
6. **Der Patch bei ungespeicherter Tabelle** greift in die Store-Invarianten (Ort eines Assets wechselt). Das braucht einen eigenen Kernel-Test mit Undo und Redo über die Grenze.
7. **Teil-Laden in 3D** ändert das Verhalten an Matrixrändern (Nachbarn, Höhen, Picking, Event-Drag über die Gebietsgrenze).
8. **Test-Umgebung und Gate-Dokumente** nennen „Raster“ (`docs/agent-gate.md` Z. 53, 64 f.). Die `data-*`-Attribute bleiben erhalten, die Tabelle ist nachzuziehen.
9. **Phase 4** (Textprojekt, symbolische IDs) wird `g4-world.json` aufnehmen müssen. Das Schema bleibt deshalb klein und versioniert.
10. **Index 0 „Mysteriöser Ort“** ist in Vanilla auch an echten Sonder-Headern (`UG`, `UNION`, `DIRECT2`). In Bauen ausblenden darf nicht heißen, dass sie unerreichbar werden.

---

## 6. Teilphasen (e), Vorschlag „3e Weltmodell“ vor Phase 4

| Teil | Inhalt | Größe | Gate-Idee |
|---|---|---|---|
| **3e.1 Begriffe und Baum** (nur Frontend) | `worldModel.ts` (Weltkarten, Gebiete, Räume über Warps, Landschaft, Verwaistes); Baum Weltkarte › Gebiet › Räume mit Ortsetikett; Klarnamen-Heuristik; Sortierung (`worldSort` in `g4.shell.v1`); Glossar; „Sonstiges“ nur in Tüfteln | M (~1.200–1.800 Zeilen mit Tests) | vitest am synthetischen Projekt; gated Test auf Pt DE, HG DE und HG US: jeder Header genau einmal im Baum (Tüfteln) bzw. kein Verwaister in Bauen, Weltkarten = {Sinnoh} bzw. {Johto, Kanto}, Großmoor und Safari unter „Weitere Gebiete“, ≥ 90 % der Räume einem Gebiet zugeordnet, Kraterberg unter seinem Eingang; **DOM-Scan in Bauen** (Browser-Modus): kein sichtbares „Raster“, „Matrix“ oder „Header“ in Liste, Schau und Hand |
| **3e.2 Raster nach Tüfteln** | Weltkarte mit Umschalter, Zellen-Auswahl und „+“, Raumplan; 3D-Auswahl der Karte, Rahmen, Kamera aufs Gebiet, Teil-Laden, Draufsicht; Reiter „Raster“ nur in Tüfteln (Layout v9); Brotkrume, Status, Palette | L (~1.800–2.500) | Agent-Szenario in Bauen ohne Raster-Reiter: ein Gebiet um eine Karte erweitern (Weltkarte „+“), in die neue Karte Teile importieren (Inspector „Karte“), Höhe setzen, in melonDS begehbar; Ladezeit Zweiblattdorf gemessen < heute; `scripts/gate.sh` grün |
| **3e.3 Eigene Orte** | Kernel `create_place`, `rename_place`, `assign_place`, Namen aus dem Store, Prüfungen; Inspector „Ort“ als Picker; „Neuer Ort…“ im +-Menü und im Assistenten; HGSS-Plätze | M (Kernel ~600, UI ~800) | Auf Kopien von Pt DE und HG DE: Ort „Neuheim“ mit neuem Gebiet und Raum; beim Betreten erscheint die Einblendung „Neuheim“ (Blick oder Screenshot aus g4emu); ein dort gefangenes Pokémon zeigt den Fundort; DSPRE zeigt den Namen; `vanilla_checks` behält 0 Fehler |
| **3e.4 Leichter Patch** | Schritt „Ort“ statt „Header“, immer neuer Header, automatischer interner Name; Patch mit einer Zustimmung in der Zusammenfassung; Kernel: Patch bei ungespeicherter Tabelle, Schnappschuss; `allow_patch` | S–M (Kernel ~250, UI ~600) | Frische Vanilla-Kopie, eine Header-Änderung ungespeichert: „Gebiet anlegen“ mit neuem Ort in einem Durchgang, ohne Technik und ohne Speichern dazwischen; zweiter Assistent fragt nicht mehr; Undo der früheren Änderung geht; DSPRE zeigt den Patch; melonDS begehbar |
| **3e.5 Nahtlos auf der Weltkarte** (optional) | `create_area` mit Ziel `Existing`; der Assistent wählt freie Zellen auf der Weltkarte; „Abspalten“ geteilter Räume (`split_room`) | M | Neue Route grenzt an Route 201 und ist **ohne Warp** begehbar (g4emu läuft hinüber, der Ortsname wechselt); ein abgespaltenes Pokémon-Center ändert nur einen Ort |

Reihenfolge: Mit 3e.1 zuerst ist das Modell geklärt. 3e.4 ist unabhängig und kann parallel laufen. 3e.3 setzt 3e.1 voraus, 3e.2 und 3e.5 setzen 3e.1 voraus.

---

## 7. Offene Fragen an den Nutzer (f)

1. **Zuordnung der Räume:** physisch über die Tür (Variante B, Empfehlung) oder nach dem Ortsnamen (Variante A)?
2. **Orte im Spiel:** Ist es gewollt, dass ein neuer Ort im Spiel als Name erscheint (Einblendung, Fundort)? Oder sucht der Nutzer nur Arbeitsordner, die das Spiel nie zeigt?
3. **HGSS:** Johto und Kanto als zwei Weltkarten oder eine mit Filter?
4. **Neue Gebiete:** standardmäßig nahtlos auf die bestehende Weltkarte (braucht 3e.5) oder wie heute als eigene Karte mit Türen?
5. **Wiederverwenden** freier Vanilla-Header: ganz nach Tüfteln?
6. **Klarnamen für Vanilla-Räume** aus der Heuristik, umbenennbar und nur im Projekt gespeichert: einverstanden?
7. **Stadtkarte und Flugziele** für neue Orte: wichtig genug für einen eigenen Recherche- und Patch-Posten?
8. **Draufsicht-Kamera** schon in 3e.2 statt in Phase 6?
9. **Einordnung:** 3e vor Phase 4, oder Teile (3e.1, 3e.4) zuerst und den Rest nach Phase 4?
10. **Geteilte Räume:** Beim ersten Bearbeiten automatisch „Abspalten?“ fragen oder nur die Plakette zeigen?
