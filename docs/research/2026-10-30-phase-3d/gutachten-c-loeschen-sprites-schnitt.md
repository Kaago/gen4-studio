# Gutachten C: Löschen mit Referenzsperre, Kampfsprites, DS-Bildschirm-Varianten, verwaiste Posten und der Schnitt von 3d

Stand: 2026-10-30. Gutachter: Architektur-Review (nur lesend am Code, mit Messungen an frischen Kopien). Repo: `$HOME/Projekte/lab/editor/gen4-studio`, Branch `main` (`a5d6909`).

Gelesen: `docs/ROADMAP.md` (Phase 3d, Phase 4 mit „Nicht in Phase 4“, „Aus dem Design-System, später“ mit der Tabelle aus 3b.3, Phase 5, Phase 6, Querschnitt), `docs/research/2026-10-27-phase-4/entscheidungen.md` (Einordnung von 3d, Nicht in Phase 4, Vorsorge für Phase 5 und 6), Gutachten C vom 2026-10-26 (§7, §9, Stil), `design-system/README.md` (Komponenten, „Was die App umsetzt“, Nachträge 4.1a bis 4.4), `design-system/Audit-Design-System.md` (Screen-Tabelle, Zeile „Hand-Höhe“), `docs/formats/text_project.md`, `docs/formats/ncgr_nclr.md`, `docs/mcp.md`, Spezifikation 3b.3 §9 und §12, Spezifikation 3b.4 §1; im Code `crates/g4-kernel/src/textproject/{codes,check,import,arts,registry,rename}.rs` und `schema/mod.rs`, `data/refs.rs`, `metadata/script_refs.yaml`, `data/{trainer_create,encounter_create,icons}.rs`, `store.rs` (Append, Platzhalter), `build.rs` und `build/steps.rs` (Packen, `splice`, `REMOVED`), `checks/{world,build}.rs`, `dynamic_headers.rs`, `original.rs`, `diff.rs`, `metadata/archives.yaml`, `crates/g4-formats/src/{ncgr,nclr}.rs`, `apps/studio/src/ui/primitives/{DSScreen,PokemonCard,Sprite}.tsx`, `apps/studio/src-tauri/src/commands/events.rs`, `apps/studio/src/app/useWorldActions.ts`.

Messungen (alle Skripte liegen neben diesem Gutachten, sie lesen nur und geben nur Zahlen aus):

| Skript | Was es misst |
|---|---|
| `gutachten-c-sprites-inventar.py` | Member, leere Member, Magics und Kopf-Felder der NCGR in den Sprite-Ordnern eines DSPRE-Ordners |
| `gutachten-c-sprites-krypto.py` | Entschlüsselung der Pixeldaten in beiden Richtungen: Anteil durchsichtiger Halbbytes, durchsichtige Ecken |
| `gutachten-c-proto.rs` (+ `gutachten-c-proto.Cargo.toml`) | Rust-Prototyp gegen `g4_formats::ncgr` und `nclr`: entschlüsseln, mit `Ncgr::read` und `Nclr::read` lesen, Maße, Farben je Bild, zusammenhängende Flächen bei linearer und gekachelter Ablage, Lage im Bild, Zeit |
| `gutachten-c-referenzen.py` | Verweise zwischen den Welt-Arten eines Textprojekts je Kante „Quelle → Ziel“ und wie viele Entitäten einer Art überhaupt einen sichtbaren Verweis haben |

Datenbasis: frische APFS-Klone (`cp -cR`) von Platin DE, HeartGold DE und HeartGold US unter `Roms/_testkopien/3d-gutachten-c-{pt,hg,hgus}`, die `.nds` daneben per `cp -c`; Platin DE und HeartGold DE mit `g4k base assign` und `g4k text export --base-rom` zum Textprojekt gemacht, HeartGold US blieb DSPRE-Layout. `g4k` ist der vorliegende Release-Build (`target/release/g4k`). Die Originale wurden nur kopiert, die Kopien sind am Ende gelöscht. Zwei andere Gutachter liefen parallel: **alle Zeiten ±50 %**.

---

## 1. Anlass

Phase 4 ist abgeschlossen, als Nächstes kommt 3d „Erzählen“ (vor Phase 5). Die Roadmap nennt für 3d: die Tiefe „Erzählen“ mit Rezepten und Live-Vorschau, Flags und Variablen, Geschichten und Story-Graph, Löschen von Daten mit Referenzsperre, dazu Ei-Attacken, Kampfsprites, DS-Bildschirm-Varianten und Erzählen-Layouts. Die Hauptsitzung schlägt einen Schnitt in vier Teilphasen vor. Dieses Gutachten prüft:

1. was „Löschen mit Referenzsperre“ technisch heißt, im Textprojekt und im DSPRE-Layout, und wie groß die Lücke im Referenzindex ist;
2. wo die Kampfsprites liegen, wie ihre Verschlüsselung geht (Prototyp) und was Lesen und Schreiben kosten;
3. was die DS-Bildschirm-Varianten brauchen;
4. wohin die verwaisten Posten gehören (Spielmodus live, Veröffentlichen-Assistent, Hand-Höhe, Plakette „geändert“, „Stand sichern“);
5. den Gesamtschnitt von 3d mit Größen, Abhängigkeiten, Worktrees und Gates.

---

## 2. Löschen mit Referenzsperre

### 2.1 Befund: wie der Code heute mit fehlenden Membern umgeht

**Build (DSPRE-Layout und Zwischenstand des Textprojekts).** `build::scan_members` verlangt die Member `0000`, `0001`, … ohne Lücke; die erste fehlende Nummer bricht den Build ab („member 0010 is missing“), und die Prüfung `build.member-gap` meldet dasselbe vorher als Fehler. Das Einsetzen ins vorhandene NARC (`build::splice`) ersetzt und hängt an, kürzt aber nie; ein entferntes Member führt im Schritt `pack:*` zu einem vollen Packen (Grund `REMOVED`, `build/steps.rs:382`). Gemessen auf HeartGold US (DSPRE-Layout):

| Versuch | Ergebnis |
|---|---|
| `unpacked/encounters/0010` entfernt, `g4k build` | Abbruch nach 0,09 s: `member 0010 is missing` |
| letztes Member `0141` entfernt, `g4k build --timings` | baut (1,7 s), `pack:encounters` voll mit 141 Membern |
| danach `g4k check` | Fehler `header.wild-dangling`: ein Header zeigt auf die nicht mehr vorhandene Datei 141 |

Der Build verträgt also **Kürzen am Ende, keine Lücke**. Die vorhandenen Prüfungen erkennen hängende Verweise der Welt **nachträglich**, sobald ein Verweis über die Zahl der Member hinauszeigt: `header.ref-range` (Matrix, Eventdatei, Scriptdatei, Level-Script, Textarchiv, Gebietsdaten), `header.wild-dangling`, `matrix.map-range`, `matrix.header-range`, `warp.dest-header`, `warp.dest-warp`, `area.tileset-range`, `map.building-range`, `event.script-missing`, `event.trainer-missing`.

**Textprojekt.** Die Dateien `data/<art>/NNNN.json` sind lückenlos nummeriert, der Dateiname ist die Member-Nummer (`docs/formats/text_project.md`, „Ablage“). `import::check_numbering` meldet eine fehlende JSON unter der Member-Zahl als `textproject.member-without-json` („der Import löscht keine Member, Löschen kommt mit Phase 3d“), eine Lücke darüber als `textproject.schema`. Gemessen auf Platin DE (Textprojekt):

| Versuch | `g4k text check` |
|---|---|
| `data/trainers/0500.json` entfernt (Mitte) | Fehler `member-without-json` an 0500; **dazu** die Warnung `textproject.id-dangling` für die ID der Nummer 927, obwohl deren Datei da ist |
| `data/trainers/0927.json` entfernt (Ende) | Fehler `member-without-json` („das Archiv hat 928 Member, die Dateien enden vor 0927“) und `id-dangling` für 927 (hier zu Recht) |

Der Hinweis `id-dangling` zählt die Dateien statt die höchste Nummer (`textproject/check.rs:226-233`, `numbered(…).len()`); eine Lücke in der Mitte meldet deshalb die letzte ID fälschlich als hängend. Klein, aber vor 3d.2 zu beheben (Zahl = höchste Nummer + 1).

Der DSPRE-Abgleich (4.3b) behandelt ein gelöschtes Member oder eine gelöschte JSON als „fehlerhaft“ („Der Abgleich löscht keine Entität, Löschen kommt mit Phase 3d“, `text_project.md` Z. 614). Das Register `data/ids/*.json` ist eingefroren: ein Eintrag bleibt, wenn seine Nummer verschwindet.

**Anlegen ist heute unumkehrbar.** `Store::append_member` schreibt das neue Member sofort auf Platte, ohne Undo (`store.rs:1468-1484`); so arbeiten `create_trainer`, `create_encounter_file`, `create_event_file`, `duplicate_event_file`, Matrix und Karte der Assistenten. Ein falsch angelegter Trainer bleibt also heute stehen: das ist der häufigste Grund, löschen zu wollen.

**Löschen innerhalb einer Eventdatei rückt schon heute nach, ohne Sperre.** `delete_event_impl` (`commands/events.rs:356`) entfernt das Event und lässt die folgenden derselben Klasse aufrücken; die Oberfläche fragt nur nach (`useWorldActions.ts:239`). Warps anderer Eventdateien zeigen aber per **Index** auf ihren Ziel-Warp (`dest_warp_id`), Scripts setzen Warps per Index um (`SetWarpEventPos`, Platin 38 Aufrufe). Gemessen: Von 1 213 Warps in Platin DE (1 317 in HeartGold DE) liegen 575 (706) vor einem Warp derselben Datei, auf den ein anderer Warp zeigt; ihr Löschen lenkt diese eingehenden Warps **still auf die falsche Tür**. `warp.dest-warp` erkennt nur einen Index über der Zahl, keine Verschiebung. Das gehört in 3d.2 (Abschnitt 2.4).

### 2.2 (a) Welche Arten sind löschbar?

Der Messlauf über die Verweise (`gutachten-c-referenzen.py`) zeigt, wie viel vom Spielcode selbst benutzt wird, ohne dass ein Header, eine Matrix, ein Warp oder ein Script darauf zeigt:

| Art | Platin DE: gesamt / sichtbar verwiesen | HeartGold DE | Folgerung |
|---|---|---|---|
| Scriptdateien (Header: Script und Level-Script) | 1 124 / 962 | 965 / 904 | 162 bzw. 61 Dateien nur aus dem Code (gemeinsame Scripts, Kampf-Script der Trainer) |
| Textarchive (Header) | 724 / 404 | 829 / 398 | rund die Hälfte nur aus dem Code (Systemtexte, Namensbänke) |
| Header (Warp, Matrix, Script) | 593 / 499 | 540 / 482 | 94 bzw. 58 ohne sichtbaren Verweis (Flugziele, Sonderorte, Rückkehrpunkte stehen in ARM9 und Overlays) |
| Matrizen (Header) | 289 / 270 | 288 / 280 | wenige unbenutzt |
| Karten (Matrix) | 666 / 656 | 676 / 657 | wenige unbenutzt |
| Eventdateien (Header) | 534 / 534 | 491 / 491 | alle benutzt |

Verweise aus ARM9 und Overlays sieht kein Index. Daraus folgt die Grundregel: **Was in der Basis-ROM steht, darf bei den „Code-Arten“ nicht gelöscht werden.** Ob eine Entität neu ist, weiß der Kernel seit 4.1b (`original::NO_ORIGINAL`, Basis-Member-Zahl).

| Art | löschbar? | Grund |
|---|---|---|
| Trainer | ja, auch Original mit Warnung | Verweise kennt `refs` (Figuren 3000/5000 + n − 1, Scripts); Kampfplätze im Kampf-Script bleiben, der „gesehen“-Platzhalter bleibt gesperrt |
| Begegnungsdatei | ja, auch Original mit Warnung | einziger Verweis Header → `wild_pokemon` (in `refs`); HGSS hat höchstens 255 |
| Eventdatei | ja, auch Original mit Warnung | Verweis nur aus Headern (alle 534/491 benutzt, Löschen also nur, wenn der Header vorher umgehängt ist) |
| Matrix, Karte | ja, auch Original mit Warnung | Verweise Header → Matrix, Matrix → Karte; Platin 10, HeartGold 19 Karten ohne Matrix |
| Header | **nur neue** | Code-Verweise unsichtbar; neue Header gibt es erst mit „Dynamische Header“ (`dynamic_headers.rs`) |
| Scriptdatei, Textarchiv | **nur neue** | rund 15 % bzw. 50 % nur aus dem Code verwiesen |
| Gebietsdaten, Texturen, Gebäudemodelle | **nur neue** | geteilt (Platin: 593 Header auf 72 Gebietsdaten), Verweise aus Gebietsdaten und Karten |
| eigene Orte (`places`, 3f) | ja, nur eigene | Name in der Ortsnamen-Bank; Verweis Header → `location_name` |
| Geschichten, eigene Flag-Namen (3d.1, 3d.3) | ja | reine Projekt-Entitäten ohne Member: Datei entfernen |
| Spezies, Formen, Attacken, Items, Fähigkeiten, Typen, Trainerklassen | **nie** | feste Tabellen: der Code rechnet mit Zahl und Lage (Item-Tabelle in ARM9, Symbole, Sprites je Spezies, Kampfgrafik je Klasse) |

### 2.3 (b) Was Löschen technisch heißt

| Variante | Vorteile | Nachteile |
|---|---|---|
| **A Nachrücken** (alle folgenden Nummern um eins, alle Verweise umschreiben) | lückenloses Archiv, Nummer und ROM-Platz frei | Verweise per Nummer stehen in Header-JSON (Matrix, Eventdatei, Scripts, Textarchiv als Zahlen), Matrix-Zellen, Script-Quellen (Zahlen, rotom-Namen), Kampf-Script (Trainer `n` ↔ Script 3000 + n − 1, positionsgebunden) und unsichtbar in ARM9/Overlays; ein Fehler lenkt still um (wie heute bei Warps, 2.1); Git-Diff über Hunderte Dateien |
| **B Grabstein** (die Nummer bleibt, die Entität ist als gelöscht markiert) | keine Nummer verschiebt sich; lückenlos bleibt lückenlos, Build, `build.member-gap`, DSPRE-Abgleich und Round-Trip ändern sich nicht; als Bearbeitung des Members im Store **rückgängig** | Nummer bleibt belegt; im DSPRE-Layout gibt es keinen Ort für die Markierung |
| **C Ende kürzen** (nur das letzte Member) | Nummer und Platz wirklich frei, der Build kann es heute schon (`REMOVED`) | nur am Ende; als Entfernen einer Datei nicht im Store-Modell, also wie das Anlegen unumkehrbar |

Für B gibt es zwei Formen: **B1 „markieren, Inhalt bleibt“** (die JSON bekommt `"deleted": true`, das Member behält seine Bytes) und **B2 „leeren“** (neutraler Inhalt: leere Eventdatei, Begegnungsdatei aus Nullen, Platzhalter-Trainer wie der des „gesehen“-Slots). B1 ist sicherer: ein Verweis, den kein Index sieht, trifft weiter gültige Daten, statt ein Spiel ohne Team oder ohne Begegnungen auszulösen. B1 ist im Textprojekt billig: das Schema kennt schon auslassbare Schlüssel (`?` in `schema.snap`), ein Feld `deleted` mit `serde(default)`, das der Export nur bei `true` schreibt, lässt alle vorhandenen Dateien byte-gleich. Weil alle Typen `deny_unknown_fields` tragen, würde ein älteres Studio die Datei ablehnen: das ist der Fall für **`schemaVersion` 2** mit einer Migration ohne Dateiänderung, die erste echte Migration nach der Probe-Migration aus 4.3a.

**Empfehlung (recommend): Grabstein B1 als „Löschen“, Kürzen C als ausdrücklicher Zusatz; kein Nachrücken.**

- **Im Textprojekt** heißt Löschen: `deleted: true` in `data/<art>/NNNN.json` (generisch in der Schema-Schicht, eine Stelle für alle Arten), Store-Änderung mit ⌘Z, Listen blenden die Entität aus (Filter „Gelöschte zeigen“), Prüfungen behandeln sie als fehlend, die Referenzsperre verbietet neue Verweise. Ist die Entität neu und liegt sie am Ende (auch nach weiteren Grabsteinen), bietet der Dialog „Nummer freigeben (nicht rückgängig)“ an: JSON und Member gehen, das Archiv wird kürzer (C). Für das Kürzen über Git hinweg (ein Branch hat weniger Dateien) gilt die Fingerabdruck-Regel aus 4.2: fehlt die JSON am Ende und ist das Member seit der Aufzeichnung unverändert, kürzt der Import; sonst bleibt es beim Fehler `member-without-json` wie heute.
- **Scriptdateien und Textarchive** sind DSPRE-Formate ohne Platz für ein Feld (rotom-Quelle, chatot-JSON); dort nur C für neue am Ende.
- **Im DSPRE-Layout** ohne Textprojekt: nur C für neue Entitäten am Ende (unumkehrbar mit Dialog, wie das Anlegen) und B2 „Leeren“ als gewöhnliche Bearbeitung, wo eine neutrale Form existiert (Eventdatei, Begegnungsdatei, Text). Die Oberfläche sagt bei allem anderen: „Echtes Löschen gibt es im Textprojekt (Technik › Textprojekt).“ Eine weitere Projektdatei für Grabsteine im DSPRE-Layout passt nicht zur Entscheidung, neue Projekt-Daten nur noch im Textprojekt anzulegen.
- **Nicht** Grabsteine beim Anlegen wiederverwenden (Verweise aus Scripts anderer Branches oder aus dem Code könnten noch zeigen); das bleibt später.

### 2.4 (c) Referenzsperre: was `refs.rs` kennt und was fehlt

`refs.rs` kennt als Ziele Species, Move, Item, Trainer, TrainerClass und Encounter; Quellen sind die Header-Tabelle (nur `wild_pokemon`), Eventdateien (nur Figuren → Trainer), Begegnungen, Personal, Lernsets, Entwicklungen, Trainer-Eigenschaften und -Team und seit 4.3b die Script-Quellen über `metadata/script_refs.yaml` (Trainer, Item, Species, Move). Gemessene Kanten im Textprojekt (Verweise / verschiedene Ziele, Platin DE und HeartGold DE):

| Kante | Platin DE | HeartGold DE | in `refs` | billigste Quelle |
|---|---|---|---|---|
| Header → Begegnungsdatei | 154 / 154 | 135 / 135 | ja | – |
| Figur → Trainer | 417 / 413 | 404 / 404 | ja | – |
| Header → Matrix, Eventdatei, Scriptdatei, Level-Script, Textarchiv, Gebietsdaten | je 593 | je 540 | **nein** | dieselbe Header-Tabelle, die schon Quelle ist |
| Header → Ort | 593 / 125 | 540 / 121 | **nein** | ebenso |
| Matrix → Karte | 1 172 / 656 | 958 / 657 | **nein** | neue Quelle Matrix (289 bzw. 288 Member) |
| Matrix → Header | 920 / 73 | 819 / 77 | **nein** | ebenso |
| Warp → Header | 1 213 / 469 | 1 317 / 459 | **nein** | Eventdateien sind schon Quelle |
| Warp → Ziel-Warp (Index) | 993 eingehend | 1 068 eingehend | **nein** | ebenso; nötig fürs Warp-Löschen (2.1) |
| Script → Header (`Warp`, `MoveWarp`) | 67 / 44 | 154 / 58 | **nein** | `script_refs.yaml` um das Ziel `header` erweitern; 1 bzw. 4 Aufrufe mit Variable sind nicht prüfbar |
| Gebietsdaten → Karten- und Gebäudetexturen | 75 / 74, 71 | 106 / 106, 104 | **nein** | neue Quelle Gebietsdaten |
| Karte → Gebäudemodell | 3 476 / 360 | 2 859 / 322 | **nein** | neue Quelle Karte (Gebäudeliste) |
| Flugziele, Sonderorte, Rückkehrpunkte, gemeinsame Scripts, Systemtexte | – | – | nie | stehen in ARM9 und Overlays; Schutz über „nur neue“ (2.2) |

Die Lücke ist also groß in der Zahl der Kanten, aber billig: sechs der zehn fehlenden Kanten kommen aus Quellen, die `refs` schon liest, die übrigen brauchen drei neue Quellen (Matrix, Gebietsdaten, Karte) und eine Zeile in `script_refs.yaml`. Der Index baut sich inkrementell aus `Store::changes_since`, das gilt für die neuen Quellen ebenso.

| Variante | Vorteile | Nachteile |
|---|---|---|
| **R1 `refs` erweitern** um die Ziele Header, Matrix, Karte, Eventdatei, Scriptdatei, Textarchiv, Gebietsdaten, Ort (Texturen und Modelle optional) und die Quellen oben | genau, liefert „Wird verwendet von …“ für Dialog, Inspector, Picker und MCP; dieselbe Tabelle braucht Phase 5 (Referenzen auf deklarierten Feldern) | `refs.rs` (1 193 Zeilen) wächst um rund 400 bis 600 Zeilen mit Tests; Zeittest des Index neu messen |
| **R2 Prüf-Vorlauf**: Löschen im Prüf-Schnappschuss simulieren, neue Fehler der vorhandenen 41 Prüfungen sind die Sperre | nutzt die Bereichsprüfungen von heute, wenig neuer Code | sieht Grabsteine nicht (die Zahl der Member ändert sich nicht), keine Liste „verwendet von“, langsamer (voller Lauf 250 bis 290 ms warm) |

**Empfehlung (recommend): R1, als deklarative Tabelle.** Die Kanten stehen als Daten (Quelle, Feld, Zielart) statt als Code je Art, wie `script_refs.yaml` es für Scripts schon tut. Dann bekommen die Arten von 3d.1 und 3d.3 (Flag, Variable, Geschichte) und später Plugin-Arten Referenzen ohne eigenen Code. Die Prüfungen lernen Grabsteine als „fehlt“ (eine Funktion `is_deleted` in der Prüfquelle) und schließen gelöschte Quellen aus. Warps: „Warp löschen“ zieht die `dest_warp_id` eingehender Warps nach (die Verweise sind bekannt und wenige) und sperrt, solange ein Warp auf den gelöschten selbst zeigt oder ein Script ihn per Index umsetzt; das schließt die stille Umlenkung aus 2.1.

**Kosten der Lücke, wenn man sie offen ließe:** Löschen von Matrix, Karte, Eventdatei, Header und eigenen Orten ginge nur mit „nachträglich prüfen“ (die Bereichsprüfungen greifen erst nach dem Kürzen, bei Grabsteinen nie). Das ist das „leere Versprechen“, vor dem die Tabelle 3b.3 warnt.

### 2.5 (d) Undo, „Auf Original zurücksetzen“, Diff

- **Grabstein = ein Undo-Schritt** im Store (Store-Transaktion: Flag der Entität plus, wo nötig, das Nachziehen der Warps). ⌘Z holt die Entität zurück.
- **Kürzen = unumkehrbar** mit Dialog und Plakette „Nicht rückgängig“, wie `create_trainer`; eine Undo-Grenze im Verlauf.
- **„Auf Original zurücksetzen“ (4.1b)** nimmt bei einer Original-Entität auch das `deleted` zurück („Wiederherstellen“ im Dialog, Vorher „gelöscht“, Nachher „wie in der Basis-ROM“). Bei einer neuen Entität bleibt „Kein Original“; dort holt ⌘Z oder ein „Wiederherstellen“ im Menü der Liste („Gelöschte zeigen“) sie zurück.
- **Diff (4.4)**: `diff.rs` kennt `DiffState::Removed` schon für eine fehlende Seite. Ein Grabstein ist eine geänderte JSON; der Diff zeigt ihn als Plakette „Gelöscht“ statt als Feld `deleted: false → true`. Gegen die Basis: „Gelöscht“ (Original markiert) oder gar nicht (neue Entität, die in Basis und Projekt fehlt bzw. gelöscht ist).

### 2.6 (e) MCP

- `plan_delete {kind, id}` (lesend): Verweise je Kante, Sperrgründe („Header 0451 nutzt die Begegnungsdatei“), ob Kürzen möglich ist; Muster `plan_fix`, `plan_reset`.
- `delete_entity {kind, id}` (schreibend, rückgängig: Grabstein; der Agent steht im Verlauf); mit `free_number: true` **unumkehrbar** (Rückfrage in der App, kopflos `--allow-irreversible`, sonst `needs_confirmation`). Eine Sperre antwortet mit einem eigenen Code (`referenced`) und der Liste der Verweise, nicht mit `error`.
- `restore_entity {kind, id}` (schreibend).

### 2.7 Aufwand

| Teil | Größe |
|---|---|
| `refs` deklarativ, neue Ziele und Quellen, `script_refs.yaml` mit `header`, Warps mit Index | M |
| Grabstein: Schema-Feld generisch, `schemaVersion` 2 mit Migration, Store-Flag, Export/Import, Prüfungen, Listenfilter, Diff „Gelöscht“, Reset | M |
| Kürzen am Ende (Textprojekt und DSPRE-Layout), Import nach Fingerabdruck | S |
| Oberfläche: „Löschen…“ in Inspector-Kopf und Kontextmenü, Dialog mit Verweisen, „Gelöschte zeigen“, Wiederherstellen | M |
| MCP `plan_delete`, `delete_entity`, `restore_entity` | S |
| zusammen | **L** |

---

## 3. Kampfsprites von Pokémon und Trainern

### 3.1 Ort und Aufbau (gemessen)

Alle vier Archive packt der Build seit 4.2 als reine Pack-Einträge (`archives.yaml`, `pack_only`); gelesen hat sie das Studio noch nicht.

| Archiv | Platin (`files/poketool/…`) | HeartGold/SoulSilver | DSPRE-Ordner | Inhalt (alle drei Projekte gleich gebaut) |
|---|---|---|---|---|
| Pokémon | `pokegra/pl_pokegra.narc` | `a/0/0/4` | `pokemonBattleSprites` | 2 964 Member = 494 Arten × 6: vier NCGR (6 448 Byte), zwei NCLR (72 Byte) |
| Formen und Eier | `pokegra/pl_otherpoke.narc` | `a/1/1/4` | `otherPokemonBattleSprites` | Platin 253 Member (157 NCGR, 96 NCLR), HGSS 261 (161, 100); eigene Reihenfolge, braucht eine Zuordnungstabelle wie `FORM_ICON_MEMBERS` |
| Lage im Kampf | `poke_edit/pl_poke_data.narc` | `a/1/8/0` | `pokemonSpriteOffsets` | ein Member zu 43 966 Byte = 494 × 89 Byte je Art (Versatz, Schatten, Animation; Felder ungeprüft) |
| Trainer | `trgra/trfgra.narc` | `a/0/5/8` | `trainerGraphics` | 5 Member je Trainerklasse (Platin 105, HGSS 129, gleich der Zahl der Klassen): NCGR für Zellen (Modus 256, 3 248 bis 19 248 Byte), NCLR (552 Byte, 16 Paletten), NCER, NANR, NCGR linear (6 448 Byte) |

Die Bild-NCGR tragen 20 × 10 Kacheln, 4 bpp, Modus 1 („linear“: Pixel für Pixel, nicht in 8 × 8-Kacheln), also 160 × 80 Pixel: zwei Bilder zu 80 × 80 nebeneinander (das zweite ist das Animationsbild). Die Plätze je Art: 0 und 1 sind Rückseiten, 2 und 3 Vorderseiten (Rückseiten sind in 61 von 428 bzw. 66 von 478 Bildern unten angeschnitten, Vorderseiten in 6 bis 12); 0 und 2 sind die weibliche Fassung, 1 und 3 die männliche bzw. einzige. Belegt durch die leeren Member: 66 Arten ohne Platz 0 und 2 (geschlechtslose und nur männliche), 16 ohne Platz 1 und 3 (die nur weiblichen). Paletten: 4 normal, 5 schillernd, je eine Palette zu 16 Farben; bei 5 Arten sind beide gleich.

### 3.2 Verschlüsselung (Prototyp)

Die Hypothese der Hauptsitzung war: lineare Kongruenz über die Pixelwörter, „Platin rückwärts, HGSS vorwärts, Startwert aus dem letzten bzw. ersten Wort“. Der Prototyp prüft beide Richtungen mit dem Generator `k' = k · 0x41C64E6D + 0x6073` (32 Bit), XOR des Worts mit `k & 0xFFFF`, Startwert das erste Wort der Richtung (dessen Klartext 0 ist, eine durchsichtige Ecke). Gemessen über alle NCGR mit Modus 1:

| Archiv | Richtung | Platin DE | HeartGold DE | HeartGold US |
|---|---|---|---|---|
| Pokémon (1 812 Bilder) | vorwärts | **68,7 % durchsichtig, Ecken 1 778/1 812** | **68,7 %, 1 778/1 812** | **68,7 %, 1 778/1 812** |
| | rückwärts | 7,3 %, 0 | 7,3 %, 0 | 7,3 %, 0 |
| Formen (157/161) | vorwärts | **78,8 %, 137/157** | **78,9 %, 141/161** | **78,9 %, 141/161** |
| Trainer (105/129) | vorwärts | **86,2 %, 105/105** | 7,5 %, 0 | 7,5 %, 0 |
| | rückwärts | 7,6 %, 0 | **85,7 %, 129/129** | **85,7 %, 129/129** |
| roh (unverschlüsselt gelesen) | – | 6,3 % | 6,3 % | 6,3 % |

6,3 % ist der Zufallswert (1/16). **Befund: Pokémon und Formen sind in Platin und HGSS vorwärts verschlüsselt, Trainer in Platin vorwärts und in HGSS rückwärts.** Die Vorgabe „Platin rückwärts, HGSS vorwärts“ stimmt so nicht; die Richtung ist eine Eigenschaft je Archiv und Spielfamilie und gehört in die Metadaten (`archives.yaml` oder eine Sprite-Tabelle), nicht in den Code.

Der Rust-Prototyp liest jedes Bild mit dem vorhandenen `Ncgr::read` (Kopf, Maße, Tiefe), entschlüsselt die Kachelbytes und zählt:

| Messung | Pokémon (je Platz) | Formen | Trainer |
|---|---|---|---|
| `Ncgr::read` liest | alle (428 bzw. 478 je Platz) | alle | alle |
| Maße | 160 × 80, 4 bpp | 160 × 80, 4 bpp | 160 × 80, 4 bpp |
| Farben je Bild (Index 0 eingeschlossen) | 5 bis 16, Mittel 11,9 bis 14,3 | 2 bis 16, Mittel 10,9 | 12 bis 16, Mittel 15,4 bis 15,7 |
| zusammenhängende Flächen im linken Bild, linear gelesen | 1,2 bis 1,5 | 1,1 | 1,7 bis 1,9 |
| dasselbe gekachelt gelesen (wie `render_rgba` heute) | 34 bis 53 | 48 | 69 bis 71 |
| `Nclr::read` | 988 Paletten, je genau 16 Farben | – | 105 bzw. 129 Dateien zu 16 Paletten |

Die Bilder sind also plausibel: ein bis zwei zusammenhängende Gestalten auf durchsichtigem Grund, Palettenindizes 0 bis 15. **Der vorhandene Dekoder liest Kopf und Daten richtig, legt die Pixel aber gekachelt ab**; Modus 1 braucht einen linearen Weg in `render_rgba` (wenige Zeilen, das Feld „Kachelmodus“ liest `Ncgr::read` heute nicht aus). Zeit: alle 1 812 Pokémon-Bilder lesen, entschlüsseln und auswerten 1,5 bis 1,9 s im Release-Build, ein Bild rendern 9 bis 27 µs. Ein Sprite je Anfrage zu dekodieren ist also billig, ein Cache wie bei den Symbolen reicht.

### 3.3 Lizenz- und Quellenlage

- Die Formel (Generator mit den bekannten Konstanten des Spiel-Zufallsgenerators, Richtung, Startwert) ist eine Tatsache über das Datenformat. Sie ist hier **aus eigener Messung** belegt (Skripte neben diesem Gutachten); es ist kein Code übernommen.
- pret (pokeplatinum, pokeheartgold, nitrogfx) beschreibt dasselbe als Bauoption, die Repos tragen aber keine Lizenz: lesen als Quelle für Tatsachen ja, kopieren nein (wie Entscheidung 4.3a zu den Symboltabellen).
- DSPRE ist AGPL; nicht gelesen, nicht kopiert.
- NCGR, NCLR, NCER, NANR: öffentliche Formatbeschreibungen, schon in `docs/formats/ncgr_nclr.md`. Der neue Abschnitt „Kampfsprites“ dort nennt die Messung als Quelle.
- Die Bilder selbst sind Spielinhalt: nie in Tests außerhalb der gated Läufe, nie im Repo; die gated Tests prüfen Zählungen wie oben, keine Pixel.

### 3.4 Aufwand

**Lesen und Anzeigen (M):**
- `g4-formats`: Modus 1 linear in `render_rgba` (oder `Ncgr::read` liefert den Modus), `decrypt(words, direction)`; S.
- Kernel `data/battle_sprites.rs` nach dem Muster `icons.rs`: Art × Platz (Vorderseite/Rückseite, männlich/weiblich, normal/schillernd), Formen-Tabelle für `otherPokemonBattleSprites` (Prüfung wie Plan-Task 0 von 3b.3a), Trainerklasse → `trainerGraphics` Member 5 · Klasse + 4 mit Palette aus Member 5 · Klasse + 1; Richtung je Archiv aus den Metadaten; S bis M.
- Befehle `get_battle_sprite {species, form?, side, female?, shiny?, frame?}` und `get_trainer_sprite {class}` (PNG wie `get_mon_icon`), Frontend-Cache; S.
- Anzeige: Pokédex-Schau (DS-Profil, Umschalter vorn/hinten, ♀, schillernd), Trainer-Kopf und Schau (Klassenbild), Pokémon-Karte groß in der Kampfvorschau (Abschnitt 4); M mit den DS-Varianten.

**Schreiben und Import (L, getrennt):** PNG wählen, auf 15 Farben plus Durchsichtig quantisieren, die schillernde Palette muss dieselben Indizes treffen, zwei Bilder je Datei (Animation), vier Plätze je Art, die durchsichtige Ecke am Startwort erzwingen, verschlüsseln (Startwert der Basis behalten, damit der Diff klein bleibt), Versatz und Schatten in `pokemonSpriteOffsets` (89 Byte je Art, Felder erst zu belegen), dazu Trainer mit NCER/NANR. Im Textprojekt wären das heute Blobs unter `data/raw/`; sinnvoll ist eine Art mit PNG als Quelle und einem Build-Schritt „PNG → NCGR“, genau das Muster „Quelle → Member“ von Phase 6 (PNG → NSBTX) samt der LFS-Frage.

**Empfehlung (recommend): in 3d nur lesen; Schreiben mit Phase 6 (Quelle PNG → Member, gemeinsam mit PNG → NSBTX) oder einer eigenen Grafik-Phase danach.** Lesen kostet wenig und macht Pokédex, Trainer und Kampfvorschau erst „Spielerblick“; Schreiben zieht Paletten-Quantisierung, Offsets und ein neues Quellformat nach sich, das nicht zu „Erzählen“ gehört.

---

## 4. DS-Bildschirm-Varianten

**Befund.** `DSScreen` kann heute drei Varianten: `textbox`, `sign`, `empty`. 256 × 192 in ganzzahliger Vergrößerung (gemessen am Container), Spielschrift Pixelify Sans in 12 px × Faktor, Seiten mit Zeilen und Codes als Plaketten, Warnsymbol bei Überlauf, Weiter-Pfeil, ein freier `background`-Platz hinter der Box. Tokens der Gruppe `szene/ds` (Grund, Textbox, Schild, Pfeil). Was fehlt:

| Variante | braucht | Datenlage | Aufwand |
|---|---|---|---|
| Pokédex-Profil | Vorderseite 80 × 80 (3.4), Nummer, Name, Typen (`TypeBadge` im Spielstil), Kategorie, Größe, Gewicht, Pokédex-Text | Sprites und Typen da; Kategorie und Pokédex-Text als neue Namensbänke (über die Bank-Kennungen von uxie, ungeprüft); Größe und Gewicht stehen in der Roadmap bei Phase 4, sind aber nicht gekommen | S bis M |
| Kampfvorschau | Gegner-Vorderseite, eigene Rückseite, zwei Statuskästen (Name, Level, KP-Balken), Trainerbild vor dem Kampf, Hintergrund | Sprites (3.4); Hintergründe in `battleBg` sind zu 171 von 342 Membern LZ-komprimiert (`0x10`) mit NSCR-Bildschirmen: echter Hintergrund braucht einen LZ10-Dekoder und NSCR, sonst stilisierte Bodenflächen aus Tokens | M (stilisiert), +S (echter Hintergrund) |
| Laden | Sortiment, Preise, Menü | Läden stehen auf „später (Overlay-Tabellen, Patch-Layer)“; es gibt keine Daten | – |

Die Varianten sind Komponenten über `DSScreen` (eigene Primitive `DSPokedexProfile`, `DSBattlePreview`, gleiche Skalierung, gleicher Rahmen) und brauchen neue Szenen-Tokens (Statuskasten, KP-Balken grün/gelb/rot, Boden). Sie haben keinen Beweiswert (Tabelle 3b.3: „Schau-Kosmetik“), sind aber der sichtbarste Teil von „Erzählen“ und die Live-Vorschau des Assistenten „Neuer Trainer“.

**Empfehlung (recommend): Pokédex-Profil und Kampfvorschau in 3d, zusammen mit den Kampfsprites in einer Teilphase; stilisierter Hintergrund zuerst, echter Hintergrund nur, wenn der LZ10-Dekoder ohnehin kommt; „Laden“ nicht in 3d (keine Daten, folgt mit den Läden).**

---

## 5. Verwaiste Posten

| Posten | Herkunft | Befund | Empfehlung (recommend) |
|---|---|---|---|
| Spielmodus live (Live-Flags, bernsteinfarbenes Scharnier, Haltepunkte) | Audit Screen 24 „offen (Phase 4)“; Roadmap „Aus dem Design-System“ nannte ihn bei Phase 4, der Lieferumfang von Phase 4 nie | braucht Lesen und Schreiben im RAM eines laufenden Emulators; `g4emu` kann RAM lesen (libretro-melonDS, kopflos in Python), die App startet melonDS als fremdes Programm ohne Schnittstelle; Machbarkeit prüft Gutachten B. Live-Flags brauchen die Namen aus 3d.1 | **eigene Teilphase 3d.6 nach 3d.1**, nur wenn Gutachten B einen Weg mit vertretbarem Aufwand zeigt; sonst „später“ mit Phase 5. Nicht in 3d.1 bündeln: andere Dateien (Emulator, `run.rs`, Scharnier), anderes Risiko, und 3d.1 bliebe nicht M |
| Veröffentlichen-Assistent (Version, Änderungsnotizen, Speicherort, Vorschau „Was du weitergibst“, Taste in der Titelleiste) | Audit Screen 26 „teilweise“, README 4.1a | Patch, Begleitdatei und Selbstprüfung sind da (4.1a); es fehlen Schritte, Version und Notizen | **kleine Zwischenphase „4.5 Veröffentlichen“ (S bis M)**, parallel zu 3d.1 in eigenem Worktree (Dateien `technik/PublishSurface*`, `patch.rs`, Titelleiste; berührt 3d nicht) |
| „Stand sichern“ (Ein-Klick-Commit) | Nicht in Phase 4 („später mit Veröffentlichen“) | `git`-CLI zum Schreiben ist da (4.4, `git_init`) | **mit 4.5**: „Stand sichern“ = alles committen mit Datum und optionaler Notiz, dieselben Notizen speist der Veröffentlichen-Assistent; MCP mit Rückfrage |
| Plakette „geändert“ im Inspector-Kopf | Nicht in Phase 4; Audit Screen 15 | die Liste „geändert gegenüber Original“ kommt aus Hashes (4.1b, warm 0,1 bis 0,6 s) | **in 3d.2**: dort bekommt der Inspector-Kopf ohnehin „Löschen…“ und den Zustand „gelöscht“; eine Plakette „geändert“/„neu“/„gelöscht“ aus derselben Quelle ist S |
| Hand-Höhe je Unterbereich | Spec 3b.3 §9 (`g4.shell.v2`), Audit „bewusst bis auf Weiteres“ | seit 3e.4 hat jeder Bereich ein Dockview-Dock; Hand-Höhe und Einklappen in `g4.shell.v1` werden nicht mehr geschrieben, Layouts liegen unter `g4.layout.v11.<bereich>`. Die Frage ist jetzt „ein Layout je Unterbereich“ | **in 3d.4 als „Layout je Unterbereich“ (XS)**, weil die Erzählen-Layouts ohnehin je Unterbereich andere Gruppen zeigen; der alte Posten wird in der Roadmap als „durch 3e.4 überholt“ geschlossen |

---

## 6. Gesamtschnitt von 3d

### 6.1 Bewertung des Vorschlags

Der Vorschlag der Hauptsitzung (3d.1 Flags und Variablen mit Spielmodus live und Ei-Attacken; 3d.2 Löschen; 3d.3 Geschichten; 3d.4 Tiefe Erzählen mit Rezepten, Live-Vorschau, Assistent „Neuer Trainer“, Layouts, DS-Varianten, Kampfsprites) hat zwei Schwächen:

1. **3d.4 ist zu groß** (UI-Tiefe, Rezepte, Assistent, Layouts **und** Dekoder, Sprites, zwei DS-Varianten): XL, und der Kernel-Teil (Sprites) wartet unnötig bis zum Schluss. Kampfsprites und DS-Varianten sind unabhängig von Flags, Löschen und Geschichten und können früh parallel laufen; der Assistent „Neuer Trainer“ mit Live-Vorschau braucht sie dann schon.
2. **3d.1 mischt drei Risiken**: Flags (Kernel, Textprojekt, Scripts), Spielmodus live (Emulator, offen bei Gutachten B) und Ei-Attacken (Datencodec, Pokédex, Platin im Overlay 5). Ei-Attacken passen fachlich zum Pokédex und damit zur Sprite-Teilphase; Spielmodus live gehört als eigene Teilphase hinter 3d.1.

### 6.2 Empfohlener Schnitt (recommend)

| Teilphase | Inhalt | Größe | braucht |
|---|---|---|---|
| **3d.1 Flags und Variablen** | Namen aus pret (`metadata/scrcmd`: Platin 4 098 Flags, 315 Variablen; HGSS 2 252 und 394), eigene Namen als erste **Projekt-Art ohne ROM-Gegenstück** (`data/flags/…`, Register, Schema-Version je Art in `schemas`), rotom-ID-Dateien mit den eigenen Namen (`include.rs`), `refs` deklarativ mit Zielen Flag und Variable aus `script_refs.yaml` (`SetFlag`, `CheckFlag`, `SetVar` …), Ansicht „Flags und Variablen“ mit „Wo gesetzt, wo geprüft“, Umbenennen, MCP | M | Phase 4 |
| **3d.5 Kampfsprites, DS-Varianten, Ei-Attacken** | Abschnitte 3 und 4 (nur lesen), Pokédex-Profil, Kampfvorschau, Trainerbild in Kopf und Schau; Ei-Attacken: HGSS als NARC (`a/2/2/9`, schon Pack-Eintrag) lesen und schreiben, Platin aus Overlay 5 nur lesen (Schreiben braucht Patch-Schutz, später; vorher messen) | M | Phase 4; Ei-Attacken als neue Art erst nach dem Merge von 3d.1 (gemeinsame Anhängepunkte) |
| **4.5 Veröffentlichen** (Zwischenphase) | Veröffentlichen-Assistent in vier Schritten, Taste in der Titelleiste, „Stand sichern“ | S bis M | Phase 4 |
| **3d.2 Löschen mit Referenzsperre** | Abschnitt 2: `refs` mit Welt-Zielen, Grabstein und `schemaVersion` 2, Kürzen am Ende, Warp-Löschen ohne stille Umlenkung, Dialog, Plakette „geändert“/„gelöscht“ im Inspector-Kopf, MCP | L | 3d.1 (deklarative `refs`, Art-Register) |
| **3d.3 Geschichten und Story-Graph** | Geschichte als Projekt-Art (`data/stories/…`), Knoten Start, Dialog, Bedingung (Flag/Variable), Kampf (Trainer), Belohnung (Item) mit Verweisen in `refs`, Liste als Standard und Graph (React Flow), Navigation zu Script, Event, Trainer, Text; Löschen einer Geschichte = Datei entfernen mit Sperre | L | 3d.1, 3d.2 |
| **3d.4 Tiefe Erzählen** | Segment Erzählen je Bereich, Rezepte mit „nächster sinnvoller Schritt“ als Taste, Live-Vorschau der Assistenten in der Schau, Assistent „Neuer Trainer“ (mit Kampfsprite und Kampfvorschau), Erzählen-Layouts (Begegnungen; Läden nicht), Layout je Unterbereich | L | 3d.3 (Geschichten als Inhalt), 3d.5 (Sprites) |
| **3d.6 Spielmodus live** (bedingt) | Spielen mit Live-Flags und -Variablen, bernsteinfarbenes Scharnier, Haltepunkte an Ereignissen | M bis L (Gutachten B) | 3d.1 |

**Reihenfolge:** (3d.1 ∥ 3d.5 ∥ 4.5) → 3d.2 (∥ 3d.6) → 3d.3 → 3d.4. „∥“ heißt eigener Worktree.

### 6.3 Parallelisierbarkeit: welche Dateien sich berühren

| Datei oder Ort | 3d.1 | 3d.2 | 3d.3 | 3d.4 | 3d.5 | 3d.6 | 4.5 |
|---|---|---|---|---|---|---|---|
| `g4-kernel/src/data/refs.rs`, `metadata/script_refs.yaml` | ● | ● | ● | | (●) Ei-Attacken | | |
| `textproject/{arts,schema/*,manifest,migrate}.rs`, `schema.snap` | ● | ● | ● | | (●) Ei-Attacken | | |
| `textproject/include.rs`, `registry.rs` | ● | | ● | | | | |
| `store.rs` | | ● | ● | | (●) | | |
| `checks/*` | ● | ● | ● | | | | |
| `g4-formats/src/ncgr.rs`, neue `data/battle_sprites.rs` | | | | | ● | | |
| `ui/primitives/DSScreen*`, `PokemonCard*` | | | | ● | ● | | |
| Pokédex- und Trainer-Ansichten (`apps/studio/src/data/…`) | | ● (Menü) | | ● | ● | | |
| Scharnier, `run.rs`, `scripts/g4emu` | | | | | | ● | |
| `technik/PublishSurface*`, `patch.rs`, `git.rs`, Titelleiste | | | | | | | ● |
| Anhängepunkte: `commands/dispatch.rs`, `mcp/tools.rs`, `docs/mcp.md`, `api.ts`, `ROADMAP.md`, `design-system/README.md`, `.fig` | ● | ● | ● | ● | ● | ● | ● |

Parallel ohne inhaltliche Überschneidung: 3d.1, 3d.5 (ohne Ei-Attacken) und 4.5; sie treffen sich nur an den Anhängepunkten (Befehlsliste, MCP-Katalog, Doku). 3d.1 und 3d.2 berühren dieselben Kernstücke (`refs`, Schema) und laufen nacheinander. Für die Anhängepunkte gilt die Erfahrung aus Phase 4: beim Merge die `.fig` von main behalten und neu rendern, Konflikte an Anhängepunkten gesammelt auflösen.

### 6.4 Gates der Teilphasen (Muster `docs/phase-4-*-gate.md`)

Jedes Gate: Automatik (gated Tests mit `G4_TEST_PROJECT` auf frischen Kopien von Platin DE, HeartGold DE und HeartGold US), Szenario kopflos (`g4k`, `studio mcp --project`, `g4emu`), Browser-Teil über die Test-Umgebung, Hand und Blick beim Nutzer, `scripts/gate.sh` grün.

- **3d.1:** Die Flagliste zählt die Namen der Metadaten je Spiel; „Wo benutzt“ eines Flags nennt dieselben Stellen wie ein gated Vergleichstest über die Script-Quellen; ein eigener Name für ein freies Flag steht in `data/flags/…`, in den rotom-ID-Dateien und kompiliert in einem Script; Umbenennen zieht die Quellen nach (ein Undo-Schritt bzw. wie „ID umbenennen“); MCP `list_flags`, `name_flag`; `g4emu`: das Script setzt das Flag, der Lauf liest es im RAM als gesetzt (wenn `g4emu` Flags lesen kann, sonst über ein Ereignis, das nur mit dem Flag erscheint); Export idempotent, Round-Trip byte-gleich.
- **3d.5:** gated Test über alle Sprites: Platin 1 812 + 157, HGSS 1 812 + 161 Pokémon- und Formenbilder, 105 bzw. 129 Trainerbilder lesen, Mittel der Flächen linear ≤ 3, Ecken durchsichtig wie gemessen; Pokédex zeigt das DS-Profil mit Vorderseite, Rückseite, ♀ und schillernd, Trainer-Kopf das Klassenbild, Kampfvorschau Trainer und erstes Team-Pokémon; ein Sprite in ≤ 50 ms im Debug-Build über die Brücke; Ei-Attacke in HeartGold ändern, bauen, `g4k check-data` findet sie; Blick auf Hell, Dunkel, Kompakt.
- **4.5:** Assistent „Veröffentlichen“ erzeugt Patch und Begleitdatei mit Version und Notizen am gewählten Ort; „Stand sichern“ legt einen Commit an, `git status` danach leer; MCP mit Rückfrage; Rom Patcher JS als Hand.
- **3d.2:** je Art aus 2.2 eine neue Entität anlegen, löschen (Grabstein), bauen: ROM baut, `vanilla_checks` 0 Fehler, ⌘Z holt sie byte-gleich zurück; eine verwiesene Entität (Trainer an einer Figur, Begegnungsdatei an einem Header, Karte in einer Matrix, Header als Warp-Ziel und in einem `Warp`-Script) wird mit der Liste der Verweise abgelehnt, in der Oberfläche und als `referenced` über MCP; Original-Header, -Scriptdatei, -Textarchiv abgelehnt mit Grund; „Nummer freigeben“ kürzt das Archiv, `git status` zeigt die entfernte Datei, ein zweiter Branch ohne die Datei baut nach `git switch`; Warp in der Mitte löschen: eingehende Warps zeigen weiter auf dieselbe Tür (`g4emu` geht durch beide Türen); „Auf Original zurücksetzen“ stellt einen gelöschten Original-Trainer wieder her; Diff zeigt „Gelöscht“; `textproject_round_trip` und `export_idempotent` mit Grabsteinen grün; Migration 1 → 2 ändert keine Datei außer dem Manifest.
- **3d.3:** Eine Geschichte mit vier Knoten (Start, Dialog, Bedingung auf ein eigenes Flag, Kampf gegen einen Trainer) entsteht in der Oberfläche, liegt als `data/stories/…`, jeder Knoten springt zu seinem Ziel; den Trainer der Geschichte zu löschen wird abgelehnt; Diff je Knoten; dieselbe Geschichte über MCP; die Art ist über dieselbe Art-Beschreibung angemeldet, die Phase 5 Plugins geben will (Abschnitt 6.6).
- **3d.4:** In Erzählen entsteht über den Assistenten „Neuer Trainer“ (Live-Vorschau mit Kampfsprite und Team) ein Trainer und steht in der Welt, in unter zwei Minuten über die Test-Umgebung; Rezepte der Welt nennen den nächsten Schritt als Taste; Begegnungen im Erzählen-Layout; Layout je Unterbereich bleibt nach Neustart; Blick.
- **3d.6:** „Hier spielen“ startet im Spielmodus, Scharnier bernsteinfarben, ein Flag live umschalten ändert, was im Spiel erscheint (über `g4emu` geprüft), Haltepunkt an einem Ereignis hält.

**Phasen-Gate 3d** (Gesamtgate, Szenarien `pt-geschichte.yaml` und `hg-geschichte.yaml`, optional HeartGold US): Eine kleine Geschichte entsteht komplett in der Tiefe Erzählen: ein neuer Trainer per Assistent mit Kampfsprite in der Vorschau, ein eigenes Flag, eine Geschichte mit Bedingung und Kampf, ein angelegter und wieder gelöschter Entwurf (die Referenzsperre greift vorher, danach Grabstein und „Nummer freigeben“). Gebaut und in `g4emu` gespielt: der Kampf wird gewonnen, das Flag ist gesetzt, der zweite Besuch verläuft anders. Das Textprojekt liegt in Git mit einem Diff, der nur diese Arbeit zeigt; `vanilla_checks` 0 Fehler; `scripts/gate.sh` grün.

### 6.5 Folgen für den Schnitt

- Die Roadmap-Zeile 3d bekommt den Schnitt 3d.1 bis 3d.6 und die Zwischenphase 4.5; die Tabelle 3b.3 verweist Ei-Attacken, Kampfsprites, DS-Varianten auf 3d.5 und „Assistent Neuer Trainer, Erzählen-Layouts“ auf 3d.4; „Hand-Höhe je Unterbereich“ wird geschlossen („durch 3e.4 überholt, Layout je Unterbereich in 3d.4“); „Spielmodus live“ steht bei 3d.6 statt bei Phase 4; „Plakette geändert“ bei 3d.2; „Stand sichern“ und der Veröffentlichen-Assistent bei 4.5.
- Vor 3d.2 zu beheben: `id-dangling` zählt Dateien statt Nummern (2.1). Unabhängig von 3d, aber mit 3d.2 erledigt: die stille Umlenkung beim Warp-Löschen (2.1); bis dahin wäre ein Hinweis im Löschdialog des Warps die kleinste Abhilfe.
- Jede Teilphase bekommt wie in Phase 4 Spezifikation (Brainstorming), Plan, Gate-Datei und `.fig`-Frames.

### 6.6 Was 3d an Phase 5 und Phase 6 weitergibt

- **Phase 5:** zwei echte Projekt-Arten ohne ROM-Gegenstück (Flag-Namen, Geschichten) über eine Art-Beschreibung (Ordner, Schema-Version in `schemas`, deklarierte Referenzfelder, Löschsemantik „Datei entfernen“, Diff aus dem Schema). Wenn 3d diese Beschreibung als Daten statt als Enum-Zweig baut, ist sie der Prüfstein für `assetKinds`, `schemas` und die Referenzen der Plugin-API. Dazu die erste echte Schema-Migration (`schemaVersion` 2) und die deklarative Referenztabelle, die Plugins für eigene Arten füllen.
- **Phase 6:** Grabstein und Kürzen für Karten und Matrizen (auch für im Studio gemalte Karten); das Muster „Quelle PNG → Member“ für das Schreiben der Kampfsprites, gemeinsam mit PNG → NSBTX, samt LFS-Entscheidung; die Referenzkanten Karte → Gebäudemodell und Gebietsdaten → Texturen, die eine erzeugte Karte braucht.

### 6.7 Nicht in 3d

- Nachrücken von Nummern beim Löschen; Wiederverwenden von Grabstein-Nummern beim Anlegen.
- Löschen von Spezies, Formen, Attacken, Items, Fähigkeiten, Typen und Trainerklassen; Löschen von Original-Headern, -Scriptdateien, -Textarchiven, -Gebietsdaten, -Texturen und -Modellen (nur neue).
- Grabsteine im DSPRE-Layout ohne Textprojekt (dort nur Kürzen am Ende und Leeren).
- Kampfsprites schreiben oder importieren (PNG → NCGR), Trainer-Animationen (NCER/NANR), Bearbeiten von `pokemonSpriteOffsets`.
- DS-Variante „Laden“ und alles zu Läden, Attacken-Lehrern und den übrigen Overlay-Tabellen; Ei-Attacken in Platin schreiben (Patch-Schutz).
- Spielmodus live, falls Gutachten B keinen tragfähigen Weg findet (dann „später“).
- Plugin-API, projektlokale Plugins (Phase 5); Commit-Oberfläche über „Stand sichern“ hinaus (Branches, Merge, Push).
- `script.trainer-target` und andere Prüfungen von Script-Argumenten über die Referenzsperre hinaus.

---

## 7. Empfehlungen (Zusammenfassung)

1. **Löschen = Grabstein** (`deleted: true` in der Entitäts-JSON, Inhalt bleibt, ⌘Z), **„Nummer freigeben“ = Kürzen am Ende** (unumkehrbar, nur neue Entitäten); kein Nachrücken. Original-Header, -Scripts, -Textarchive nie löschen (der Code verweist unsichtbar: rund 15 % der Scriptdateien und 50 % der Textarchive). DSPRE-Layout nur Kürzen und Leeren. `schemaVersion` 2.
2. **Referenzsperre über einen deklarativ erweiterten `refs`** (Ziele Header, Matrix, Karte, Eventdatei, Scriptdatei, Textarchiv, Gebietsdaten, Ort; Warps mit Index; `Warp`-Befehle in Scripts). Sechs der zehn fehlenden Kanten kommen aus Quellen, die `refs` schon liest. MCP `plan_delete`, `delete_entity` (rückgängig), mit `free_number` unumkehrbar.
3. **Kampfsprites in 3d nur lesen.** Die Verschlüsselung ist bestätigt: Pokémon und Formen vorwärts (Platin und HGSS), Trainer Platin vorwärts, HGSS rückwärts; der Dekoder braucht die lineare Ablage (Modus 1). Schreiben später mit Phase 6.
4. **DS-Varianten Pokédex-Profil und Kampfvorschau in 3d (mit den Sprites), Laden nicht.**
5. **Verwaiste Posten:** Spielmodus live als bedingte 3d.6; Veröffentlichen-Assistent und „Stand sichern“ als Zwischenphase 4.5; Plakette „geändert“ in 3d.2; Hand-Höhe als „Layout je Unterbereich“ in 3d.4, alter Posten geschlossen.
6. **Schnitt:** (3d.1 Flags M ∥ 3d.5 Sprites/DS/Ei-Attacken M ∥ 4.5 Veröffentlichen S–M) → 3d.2 Löschen L (∥ 3d.6 Spielmodus M–L, bedingt) → 3d.3 Geschichten L → 3d.4 Tiefe Erzählen L.

---

## 8. Offene Fragen

1. **Spielmodus live:** Welchen Weg findet Gutachten B (`g4emu` als Begleitprozess mit RAM-Zugriff, eingebetteter Kern, GDB-Stub von melonDS)? Davon hängen 3d.6 und das Live-Flag im Gate von 3d.1 ab.
2. **Grabstein in Begegnungsdateien und Trainern als B2 („leeren“) zusätzlich anbieten?** B1 lässt die Daten im ROM; wer ROM-Platz oder ein „wirklich weg“ im Spiel will, bräuchte B2. Vorschlag: nein in 3d.
3. **DSPRE und Löschen:** Ob DSPRE beim Entfernen Nummern nachrückt oder nur „letztes entfernen“ anbietet, ist nicht geprüft (DSPRE nicht geöffnet, Code AGPL). Hand in DSPRE, bevor die Spezifikation 3d.2 das DSPRE-Layout festlegt.
4. **Formen der Kampfsprites:** Die Reihenfolge in `otherPokemonBattleSprites` (157 bzw. 161 Bilder) ist ungeprüft; ein Plan-Task wie Task 0 von 3b.3a belegt sie am Augenschein.
5. **`pokemonSpriteOffsets`:** 89 Byte je Art, Felder unbelegt; für die Kampfvorschau reicht zunächst eine feste Lage, für eine treue Vorschau braucht es die Felder.
6. **Pokédex-Texte, Kategorie, Größe und Gewicht:** Bänke und Zukan-Daten sind nicht angebunden; die Roadmap hatte sie Phase 4 zugeordnet, gekommen sind sie nicht. Gehören sie zu 3d.5 oder bleiben sie beim Pokédex-Profil leer?
7. **Ei-Attacken in Platin:** Lage und Länge der Tabelle in Overlay 5 je Sprache messen, bevor 3d.5 „Platin nur lesen“ zusagt.
8. **Art-Beschreibung für Phase 5:** Soll 3d.1 die Art-Beschreibung (Ordner, Schema, Referenzfelder, Löschsemantik) schon als Daten einführen, auch wenn die eingebauten Arten weiter im Enum stehen? Vorschlag: ja für die neuen Projekt-Arten, die eingebauten bleiben bis Phase 5.
