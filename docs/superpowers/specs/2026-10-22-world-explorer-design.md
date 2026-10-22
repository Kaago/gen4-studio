# Welt-Explorer, Assistenten und Weltkarte (Phase 3a): Design

Stand: 2026-10-22. Ergänzt die [Gesamtspezifikation](2026-10-21-gen4-studio-design.md). Anlass: Gate-Durchlauf von Phase 2 am 2026-10-22, Punkt 6 ("Matrix-Panel funktioniert, aber die Bedienung ist nicht verständlich"). Gewählte Richtung aus dem Brainstorming: **A, Welt-Explorer mit Assistenten, plus die Weltkarten-Übersicht aus B als zusätzliche Ansicht.**

## 1. Ziel

Ein Nutzer, der die Begriffe Header, Matrix, Eventdatei und Area-Data nicht kennt, kann im Studio

1. ein neues Gebiet aus bestehenden Karten anlegen,
2. eine Karte aus einer Vorlage anlegen (Kopie einer bestehenden Karte, wahlweise ohne Gebäude; Gelände malen bleibt Phase 6),
3. ein bestehendes Gebiet um Karten erweitern,
4. einen neuen Innenraum anlegen und mit einem Eingang verbinden,

und sieht nach jedem Schritt das Ergebnis in der 3D-Ansicht. Die vier Hürden aus dem Gate (Begriffe unklar, Reihenfolge unklar, Zahlen statt Bilder, Ergebnis nicht sichtbar) sind damit adressiert.

Der Gate-Ablauf (B) von Phase 2 (neue Route aus zwei kopierten Karten, Header umstellen, Warps setzen, in melonDS laufen) soll mit den Assistenten in unter fünf Minuten gelingen, ohne eine einzige Zahl außer der Header-Wahl zu tippen.

## 2. Nicht-Ziele

- Keine neuen Header. Die Header-Tabelle liegt fest im ARM9; ein Anlegen braucht einen Patch und bleibt ein eigener Posten in Phase 3b. Jeder Assistent hat deshalb einen Schritt "Header wählen" und zeigt, welche Header noch nicht benutzt wirken.
- Kein Gelände malen, keine neuen Modelle (Phase 6). "Leere Karte" heißt: Kopie einer Vorlagenkarte, wahlweise ohne Gebäude.
- Keine Höhen (`altitudes`) in der Szene, keine Objekt-Sprites; die bleiben in Phase 3b.
- Die Weltkarte ist eine Übersicht zum Orientieren und Springen, kein Editor: Karten werden dort nicht verschoben.
- Keine Änderung am Kernel-Store-Modell (Changesets, Undo). Anlegen neuer Member bleibt nicht rückgängig machbar und wird so angesagt.

## 3. Begriffe in der Oberfläche

Die Oberfläche spricht von Orten, Gebieten, Räumen und Karten; die technischen Namen stehen daneben in Grau und in Tooltips. Ein Glossar (`workbench/terms.ts`) liefert für jeden Begriff Beschriftung und Erklärung, damit alle Panels dieselben Worte nutzen.

| Begriff in der UI | Technisch | Regel |
|---|---|---|
| Ort | Gruppe von Headern mit demselben Ortsnamen (`location_name`) | Header ohne Ortsnamen stehen unter "Ohne Ortsnamen" |
| Header | Eintrag der Ortstabelle (`MapHeader`) | Verweist auf Raster, Eventdatei, Area-Data, Musik, Wetter. Bleibt als Wort erhalten, weil Gate-Dokumente und Warps es benutzen |
| Außen | Header, dessen Raster einen Header-Abschnitt hat | Die Region (meist Matrix 0): die Zellen des Rasters sagen, welcher Header wo gilt |
| Raum | Header, dessen Raster keinen Header-Abschnitt hat | Innenräume, Höhlen, Dungeons; "1 Karte" oder "n Karten (b × h)" |
| Raster | Matrix | Raster aus Karten, 1 × 1 bis 255 × 255 |
| Karte | Kartendatei (`MapFile`) | 32 × 32 Kacheln, Modell, Gebäude, Kollision |
| Eingang / Ausgang | Warp | Ein Warp-Paar verbindet zwei Header |
| Eventdatei | `EventFile` | Warps, Objekte, Trigger, Hintergrund-Events eines Headers |
| Area-Data | `AreaData` | Welche Texturen (Tilesets) ein Header benutzt |

## 4. Architektur

Die Arbeit liegt zum größten Teil im Frontend. Das Backend bekommt Lese-Befehle für Übersichten und drei zusammengesetzte Befehle für die Assistenten, damit ein Assistent ein Aufruf ist, der im Kernel-Test gegen das synthetische Projekt läuft.

```
apps/studio/src
  workbench/WorldExplorer.tsx      Baum nach Orten (ersetzt die Header-Liste als Panel; Liste bleibt als Umschalter)
  workbench/places.ts              rein: Header + Raster-Übersicht → Orte, Außen/Raum, Nutzung
  workbench/terms.ts               Glossar
  workbench/AreaGrid.tsx           Raster mit Kartenbildern (Panel "Raster", Assistenten, Weltkarte)
  workbench/WorldMapPanel.tsx      Weltkarte (Region + Räume je Ort + Warp-Linien)
  workbench/worldMap.ts            rein: Region finden, Zellen je Ort, Raum → Eingangszelle
  workbench/wizards/               WizardShell, NewAreaWizard, NewRoomWizard, ExtendAreaDialog, wizardSpecs.ts (rein)
  world/thumbnails.ts              Kartenbilder aus den GLBs (Top-down), Cache
apps/studio/src-tauri/src/commands
  overview.rs                      list_matrices, list_warps
  wizards.rs                       create_area, extend_area, create_room
crates/g4-kernel/src/store.rs      create_event_file, duplicate_event_file, duplicate_map_with
```

### 4.1 Backend

**Übersichten (lesend, nur Store-Stand inklusive ungespeicherter Änderungen):**

- `list_matrices() -> Vec<MatrixSummary>` mit `index, name, width, height, has_headers, has_altitudes, filled` (Zahl belegter Zellen). Liest jede Matrix des Archivs; eine unlesbare Matrix bricht die Liste nicht ab, sie fehlt und wird in `unreadable: Vec<(u16, String)>` gemeldet (`MatrixListing { matrices, unreadable }`).
- `list_warps() -> WarpListing` mit `links: Vec<WarpLink { event_file, index, x, z, dest_header_id, dest_warp_id }>` über alle Eventdateien und `unreadable: Vec<(u16, String)>`. Gesperrte (read-only) Dateien werden gelesen wie heute (`Store::event`), nur nicht beschrieben.

**Store-Erweiterungen (nicht rückgängig machbar, wie `create_matrix`):**

- `create_event_file() -> u16`: leere Eventdatei (vier Nullzähler) als neuer Member.
- `duplicate_event_file(source) -> u16`: Kopie wie `duplicate_map` (Store-Stand, bei gesperrter Quelle die Datei von Platte).
- `duplicate_map_with(source, clear_buildings: bool) -> u16`: wie `duplicate_map`, mit leerer Gebäudeliste (`buildings_tail` bleibt) wenn `clear_buildings`. `duplicate_map(source)` bleibt als Kurzform.

**Assistenten-Befehle (`commands/wizards.rs`).** Jeder prüft zuerst alles, was ohne Schreiben prüfbar ist (Header im Bereich, Quellkarten vorhanden, Größe 1..=255, Zellen innerhalb und eindeutig, betroffene Eventdateien nicht gesperrt), legt dann die neuen Member an und führt zuletzt die rückgängig machbaren Änderungen (Zellen, Header, Warps) aus. Schlägt ein später Schritt fehl, bleiben die schon angelegten Member bestehen; die Fehlermeldung nennt den Schritt und zählt die schon angelegten Member auf ("matrix 0131 and maps 0571, 0572 were created; assigning header 0001 failed: …"), damit nichts unbemerkt im Archiv liegt.

```rust
pub enum EventFileChoice { Keep, New, Copy(u16) }
pub struct AreaCell { x: u8, y: u8, source_map: u16, clear_buildings: bool }
pub struct AssignHeader { id: u16, area_data_id: Option<u8>, event_file: EventFileChoice }
pub struct AreaSpec { name: String, width: u8, height: u8, cells: Vec<AreaCell>, with_headers: bool, header: Option<AssignHeader> }
pub struct AreaReport { matrix: u16, maps: Vec<(u8, u8, u16)>, event_file: Option<u16>, header: Option<u16> }

pub struct ExtendSpec { matrix: u16, x: u8, y: u8, source_map: u16, clear_buildings: bool, header_cell: Option<u16> }
pub struct ExtendReport { map: u16, width: u8, height: u8 }

pub struct Entrance { outside_header: u16, x: u16, z: u16, inside_x: u16, inside_z: u16 }
pub struct RoomSpec { name: String, source_map: u16, clear_buildings: bool, header: AssignHeader, entrance: Option<Entrance> }
pub struct RoomReport { matrix: u16, map: u16, event_file: Option<u16>, outside_warp: Option<u16>, inside_warp: Option<u16> }
```

- `create_area(spec)`: Karten kopieren, Matrix anlegen, auf `width × height` bringen, Header-Abschnitt bei `with_headers` einschalten und (wenn ein Header zugewiesen wird) alle belegten Zellen auf diesen Header setzen, Karten eintragen; dann Header auf die Matrix (und wahlweise Area-Data, Eventdatei) umstellen.
- `extend_area(spec)`: Matrix wächst auf `max(width, x + 1) × max(height, y + 1)`, Kopie wird in `(x, y)` eingetragen, bei Header-Abschnitt wird `header_cell` gesetzt. Eine belegte Zielzelle wird abgelehnt.
- `create_room(spec)`: Karte kopieren, Matrix 1 × 1 anlegen und füllen, Eventdatei nach Wahl, Header umstellen; mit `entrance`: in der Eventdatei des Außen-Headers einen Warp bei `(x, z)` anhängen, der auf den Raum-Header und den Index des neuen Raum-Warps zeigt, und in der Eventdatei des Raums einen Warp bei `(inside_x, inside_z)` zurück auf den Außen-Header und den neuen Außen-Warp. `inside_x/inside_z` sind absolute Rasterkacheln (bei 1 × 1 also 0..31). Bei `EventFileChoice::Keep` erhält der Raum-Header seine bisherige Eventdatei; der Rück-Warp landet dann in dieser Datei.

### 4.2 Frontend

**Welt-Explorer** (Panel "Welt", ersetzt die Header-Liste als Standard). Baum: Ort → Außen-Header, dann Räume; jede Zeile zeigt Name, in Grau die Header-ID und das Kürzel, bei Räumen die Kartenzahl. Suche nach Ort, Kürzel oder ID filtert den Baum. Der Ort des gewählten Headers ist aufgeklappt und die Zeile scrollt ins Bild (wie heute). Kopf: "+ Neu ▾" (Gebiet anlegen…, Raum anlegen…), Umschalter "Baum | Liste" (Liste = heutige `HeaderList`), "Begriffe" (Popover mit dem Glossar). Die Gruppierung ist eine reine Funktion `buildPlaces(headers, matrices)`; die Raster-Übersicht kommt aus `list_matrices` und wird nach Mitgliederänderungen und Matrix-Bearbeitungen neu gelesen.

**Kartenbilder** (`world/thumbnails.ts`). Ein Bild je (Karte, Area-Data) aus dem GLB, das das Backend ohnehin cached: Top-down, orthografisch, 64 × 64 px, als Data-URL in einem Cache mit Obergrenze (1200 Einträge, älteste zuerst raus). Höchstens zwei Renderings gleichzeitig; gleiche Anfragen werden geteilt. Bis das Bild da ist, zeigt eine Zelle eine Fläche in der Farbe ihres Ortes mit der Karten-ID. Fehlschläge (kein Modell, apicula fehlt) zeigen dauerhaft den Platzhalter mit Hinweis im Tooltip. Der Renderer ist austauschbar (Tests injizieren einen), die Warteschlange und der Cache sind rein.

**Raster** (Panel "Raster", das heutige Matrix-Panel). Kopfzeile bleibt (Nummer, Name, Breite, Höhe, Header, Höhen). Das Gitter ist `AreaGrid`: Zellen 64 px mit Kartenbild, unten die Karten-ID, oben rechts die Header-ID, wenn das Raster einen Header-Abschnitt hat; Auswahl wie heute per Klick, Doppelklick öffnet den Header. Rechts und unten eine gestrichelte "+"-Reihe: Klick öffnet `ExtendAreaDialog` (Quelle: Kopie der Nachbarzelle, eine Karte aus der Liste, oder Vorlage ohne Gebäude; Header-Zelle vorbelegt mit der Nachbarzelle) und ruft `extend_area`. Der Hinweistext zu Höhen bleibt.

**Assistenten** (`wizards/`). Ein modaler `WizardShell` über der Workbench: Schrittliste links, Inhalt rechts, "Zurück", "Weiter", im letzten Schritt "Anlegen"; eine Zusammenfassung in Worten ("Legt Raster 'Route 230' 2 × 1 an, kopiert Karte 0521 und 0522, stellt Header 0001 'NOTHING' darauf um") steht vor dem Anlegen. Nach dem Anlegen: Ergebnis-Schritt mit dem, was entstanden ist, Schaltfläche "Im Studio zeigen" (wählt den Header, die Szene lädt, der Explorer klappt den Ort auf) und "Nächste Schritte" (zum Beispiel nach "Gebiet anlegen": "Mit der Welt verbinden: Header-Editor › Eingänge / Ausgänge › Warp hinzufügen"). Die Umsetzung von Formularzustand in Spec und Zusammenfassung liegt in `wizardSpecs.ts` (rein, getestet). Eine Ablehnung des Backends vor dem ersten Schreiben (Fehlertext im Klartext) erscheint als Hinweis neben den Knöpfen im Dialog; der Assistent bleibt auf der Zusammenfassung stehen.

**Eine Route wird in 3a von Hand verbunden:** einen Assistenten dafür gibt es nicht. Nach "Gebiet anlegen" verbindet man die Route über den Header-Editor, "Eingänge / Ausgänge", "Warp hinzufügen" (das Ziel wird im Event-Editor des Warps gewählt). "Raum anlegen" verbindet dagegen einen Raum mit einem Außen-Header.

- *Gebiet anlegen:* 1 Quelle (ein Raster aus der Liste wählen, dann Zellen im `AreaGrid` anklicken, Umschalt für Rechtecke; oder einzelne Karten per ID; die Zellen tragen die Ortsfarbe, eine Zeile unter dem Gitter nennt für die überfahrene oder zuletzt geklickte Zelle "Ort · Header 0342 · Karte 0521"; wird der Assistent mit gewähltem Header geöffnet, sind dessen Zellen im Raster schon angewählt), 2 Anordnung (es gibt keine Felder für Breite und Höhe: die Größe folgt der Hülle der Auswahl, ein Wachsen geht über die "+"-Zellen des Rasters und über "Karten per Nummer"; Vorschau als Gitter; Schalter "ohne Gebäude"), 3 Header (Picker mit Suche; Spalte "Warps hierher: n" aus `list_warps`, 0 heißt "wirkt unbenutzt"; Area-Data vorbelegt mit der des ersten Quell-Headers; Eventdatei: neu leer (Vorgabe), behalten, Kopie der Quelle), 4 Zusammenfassung.
- *Raum anlegen:* 1 Vorlage (ein Raum aus dem Baum, Bild der Karte; "ohne Gebäude"), 2 Header (wie oben, Area-Data der Vorlage vorbelegt, Eventdatei: Kopie der Vorlage oder neu leer), 3 Eingang ("später verbinden" oder: Außen-Header aus dem Baum, Kachel x/z vorbelegt mit der zuletzt in der Szene überfahrenen Kachel, Rück-Warp-Kachel im Raum vorbelegt mit der Position des ersten Warps der Vorlage, sonst 16/30), 4 Zusammenfassung.
- *Gebiet erweitern:* kein Mehrschritt-Assistent, der Dialog aus dem Raster.

**Inspector: Abschnitt "Ausgänge"** im Header-Editor: die Warps der Eventdatei des Headers mit Ziel in Worten ("Treppe → 0414 Zweiblattdorf (T01R0201)", Position), "Zum Ziel", dazu "Raum hier anbinden…" (öffnet Raum anlegen mit diesem Header als Außen-Header) und "Warp hinzufügen" (heutige Funktion). Liest `getEvents` und folgt dem Bus-Thema `eventData`.

**Weltkarte** (Panel "Weltkarte", Tab neben 3D und Raster). Links die Region: das Raster mit Header-Abschnitt und den meisten Zellen (`regionMatrix`), als `AreaGrid` in 32 px mit Kartenbildern, Zellen eines Ortes mit gemeinsamer Umrandungsfarbe, Ortsname an der Mitte seiner Zellen; Rad zoomt, Ziehen verschiebt. Rechts die Orte als Liste mit Raum-Kärtchen. Ein Klick auf eine Zelle oder ein Kärtchen springt dorthin (`goTo`). Für den gewählten Ort werden Linien (SVG-Overlay) von jedem Raum-Kärtchen zu der Zelle gezogen, in der sein Ausgang landet: der Raum-Warp zeigt auf einen Außen-Header und einen Warp-Index, dessen Position in der Außen-Eventdatei die Zelle `(x / 32, z / 32)` ergibt (`roomLinks`). Fehlt die Kette (Warp ohne Ziel, Ziel kein Außen-Header), gibt es keine Linie und das Kärtchen zeigt "ohne Eingang".

**Workbench.** Panels: `headers` (Titel "Welt", Komponente Welt-Explorer), `world` ("3D"), `matrix` ("Raster"), `worldmap` ("Weltkarte"), `inspector`, `status`. Der Layout-Schlüssel wird `g4.layout.v3`; `isSaneLayout` verlangt alle sechs Panels. Die Assistenten werden von einem `WizardHost` in App gerendert; `WorkbenchData` bekommt `onOpenWizard(kind, prefill)`. Nach einem Assistenten: Mitgliederzähler neu lesen, `matrixData` senden, Header-Liste neu lesen, `goTo` auf den Header.

### 4.3 Datenfluss und Aktualisierung

- Raster-Übersicht (`list_matrices`) und Warps (`list_warps`) werden beim Projektöffnen und danach bei `onCountsChanged`, Bus `matrixData` (Übersicht) und Bus `eventData` / `eventsRefetch` (Warps) neu gelesen; die Lesevorgänge sind klein (Platin: 130 Matrizen, rund 600 Eventdateien), ein Lauf pro Ereignis genügt, laufende Leser werden nicht gestapelt (ein Zähler, der jüngste gewinnt).
- Kartenbilder werden je Zelle beim Sichtbarwerden angefordert (Reihenfolge der Anforderung); die Weltkarte fordert in Zeilenreihenfolge an und zeigt den Fortschritt in der Statusleiste nur, wenn mehr als 50 Bilder ausstehen.
- Ein Assistent arbeitet gegen den Store-Stand; die neuen Member sind nach dem Aufruf sofort auf Platte, die Header- und Warp-Änderungen sind ungespeichert und rückgängig machbar (jede für sich, in Aufrufreihenfolge).

### 4.4 Fehlerbehandlung

- Jeder Assistent zeigt Backend-Fehler im Ergebnis-Schritt in Klartext, mit dem Schritt, in dem es scheiterte, und den schon angelegten Membern ("Raster 0131 und Karten 0571, 0572 sind angelegt; das Umstellen von Header 0001 schlug fehl: …"). Es gibt kein "Erneut versuchen": der Dialog verweist auf Raster und Header-Editor, wo der Rest von Hand geschieht.
- Gesperrte Eventdateien (read-only) werden vor dem Anlegen erkannt: der Eingang-Schritt zeigt die Sperre und bietet "später verbinden".
- Eine fehlende apicula (`tool_missing`) lässt die Bilder aus, die Baum- und Rasterfunktionen bleiben nutzbar; die Statusleiste meldet es einmal.
- Header ohne Ortsnamen, Raster ohne Header-Abschnitt, leere Region (kein Raster mit Header-Abschnitt): der Explorer zeigt "Ohne Ortsnamen", die Weltkarte zeigt den Hinweis "Keine Region gefunden" und nur die Raum-Liste.

### 4.5 Tests

- Rust: jede neue `*_impl`-Funktion gegen das synthetische Projekt (2 Header, 3 Karten, 1 Matrix 2 × 1, 2 Eventdateien, 2 Area-Data): Report-Inhalt, Mitgliederzähler, Ablehnungen ohne Schreiben, Undo der Header- und Warp-Änderungen; ein gated Test gegen Vanilla-Platin: `list_warps` liefert mehr als 1000 Links und Header 0415 hat genau die Warps, die `get_events` zeigt.
- vitest: `places.ts`, `worldMap.ts`, `wizardSpecs.ts`, Thumbnail-Warteschlange und -Cache (rein), `WorldExplorer` und `AreaGrid` (jsdom: Rendern, Klicks, Auswahl), Layout v3.
- Manuell (`docs/phase-3a-gate.md`): auf einer Platin-Kopie die neue Route per Assistent, Raum anbinden, bauen, in melonDS laufen; Weltkarte zeigt Zweiblattdorf mit zwei Häusern und Linien.

## 5. Offene Punkte, die bewusst nicht gelöst werden

- Welche Header "frei" sind, ist eine Heuristik (keine Warps hierher). Ein Header kann über Skripte erreicht werden, das sieht Phase 3a nicht; der Hinweis heißt deshalb "wirkt unbenutzt".
- HGSS trägt `worldmap_x/y` im Header; die Weltkarte nutzt sie nicht, sie ordnet über das Raster an (gleich für alle Familien).
- Mehrere Räume teilen sich in Vanilla oft eine Matrix und Karte (Pokémon-Center). Der Baum zeigt sie je unter ihrem Ort; "ohne Gebäude" kopiert nur die Karte, die Vorlage bleibt unverändert.
