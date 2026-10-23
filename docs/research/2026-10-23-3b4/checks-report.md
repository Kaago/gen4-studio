# Recherche 3b.4 „Projektprüfungen“ (Project Checks)

Stand: 2026-10-23. Für die Spec von Phase 3b.4 (Roadmap: „Project Checks: kaputte Warps, Scripts ohne Ziel, Trainer ohne Party, Textlängen“).

Gelesen: `docs/ROADMAP.md` §3b.4; Spec 3b.2 (`docs/superpowers/specs/2026-10-22-phase-3b2-scripts-text-design.md`, §8.2 Panel „Probleme“, §5, §6, §10); Spec 3b.3 (`…/2026-10-23-phase-3b3-data-editors-design.md`, §7 Referenzindex, §10 Begegnungen, §11 Trainer, „Später“-Tabelle); Spec 3c (`…/2026-10-23-phase-3c-doppeldeck-design.md`, Prüflicht, §7.4); Plan 3b.3a Task 9 (`docs/superpowers/plans/2026-10-23-phase-3b3a-codecs-pokemon-data.md`); Code auf `main` (2d55867): `apps/studio/src/workbench/problems/problems.ts`, `apps/studio/src-tauri/src/commands/problems.rs`, `…/commands/overview.rs` (`list_warps`), `crates/g4-kernel/src/text.rs` (`Problem`, `validate`), `crates/g4-kernel/src/scripts.rs` (`resolve_script`, `declared_slots`), `crates/g4-kernel/src/tiles.rs` (`warp_fires`), `apps/studio/src/workbench/warpTile.ts`, `apps/studio/src/workbench/navigation.ts` (`NavTarget`).

Fremdcode: rotom v0.7.0 (`~/.cargo/git/checkouts/rotom-1ecacf6a82d08c59/fb11503`), uxie 0.9.7 und chatot 0.8.1 (`~/.cargo/registry/src/index.crates.io-*/`), DSPRE `feature/avalonia` (lokaler Klon `/Users/POLTEP/Projekte/lab/editor/DSPRE`, `origin/feature/avalonia` = 0a550272 vom 2026-10-21) und `main` (e751187b, Release 2.3.4), Porymap `master` 26b919d (6.3.1, Klon unter `scratchpad/research-3b3/porymap`), Pokémon Studio `develop` c27957c (Klon unter `scratchpad/research-3b3/repo`).

**Eigene Messungen** (nur lesend, auf `Roms/*_DSPRE_contents` und der Testkopie `Roms/_testkopien/pt-3c1` mit übernommenen Scripts). Werkzeuge liegen in `scratchpad/research-3b4/checks-tools/` (`baseline.py`, `refine*.py`, `msgref.py`, `lvl.py`, `misc.py`, Rust-Werkzeug `textlen/` mit chatot 0.8.1). Alle Zahlen „Vanilla-Rauschen“ unten sind Treffer, die eine Prüfung auf einem **unveränderten** Spiel melden würde; das entscheidet, ob eine Prüfung brauchbar ist.

Markierungen: **[unverifiziert]** = nicht im Code oder am ROM geprüft; **[gemessen]** = von mir an den Testdaten ermittelt.

---

## 0. Kernaussagen

1. Die meisten Referenzprüfungen sind auf Vanilla **rauschfrei** (0 Treffer, gemessen an Platin DE): Header-Indizes, Warp-Ziel-Header und -Warp, Event-Script außerhalb der Scriptdatei, Trainer-Script auf fehlenden Trainer, `Message N` über das Archiv hinaus (0 von 4410), Scriptdatei mit mehreren Textarchiven, Matrix-Zellen, Entwicklungen, Lernsets, leere Party-Arten. Das sind die Kandidaten für „Fehler“.
2. Drei naheliegende Prüfungen sind auf Vanilla **laut** und brauchen eine andere Regel oder eine Ausgangslage:
   - **Textlänge mit 200 px** (chatot `DIALOG_LINE_MAX_PX`, rotom `MessageLineTooLong`, Studio 3b.2 `text::validate`): **1810** Dialognachrichten in Platin DE, 1891 in HeartGold DE, 1157 in HeartGold US wären „zu lang“. Die Verteilung bricht bei **216 px** ab (27 Kacheln × 8 px): über 216 px bleiben 90 / 114 / 113. Empfehlung: Schwelle 216 px aus Metadaten, nicht 200. Das betrifft auch die bestehende 3b.2-Warnung im Panel „Text“.
   - **Warp auf Kachel ohne Tür/Treppe** (Studio `warp_fires`): 57 Treffer in Platin DE, davon 47 reine Ankunfts-Warps (andere Warps führen hin). Nur „löst nie aus **und** niemand führt hin“ bleibt: 10 Treffer (Arenen-Rätsel, Script-Warps).
   - **Warp ohne Rückweg**: 118 Treffer (Union-Raum, Aufzüge, Arenen mit zwei Tür-Kacheln). Keine Projektprüfung; nur Anzeige im Inspector.
3. Der **RefIndex** aus 3b.3 ist das richtige Rückgrat: Eine hängende Referenz ist eine Referenz, deren Ziel-Index ≥ Anzahl ist. DSPREs `ProjectIndex` macht genau das (ein Durchlauf liefert Validierung und „Wo benutzt“). Der Index muss dafür um Ziele erweitert werden, die 3b.3 nicht kennt: Header, Eventdatei, Warp, Scriptdatei, Script-Slot, Textarchiv, Nachricht, Matrix, Karte, Area-Data.
4. rotom und rotom-lsp prüfen **innerhalb einer Scriptdatei** gründlich (Syntax, Befehle, Argumente, Sprungziele, Symbole). Sie prüfen **nicht**: Nachricht existiert im Archiv, Flag-/Variablen-Bereich, Script-Slot eines Events, Level-Script-Ziele, Trainer/Header/Item-IDs als Werte. Der LSP meldet nur für **offene** Dokumente. Das ist die Lücke, die Projektprüfungen füllen.
5. Kosten: Alles außer Text und Scripts ist billig (Python warm 0,4 s für Header, 534 Eventdateien, 1124 Script-Tabellen, 928 Trainer, Begegnungen, Daten, Matrizen; Rust schätzungsweise 20–80 ms). Teuer ist das **Dekodieren** der 724 Textarchive (chatot, Rust release, warm 0,8–0,95 s; kalt 5 s); Messen der Breite ist dagegen fast kostenlos. Text-Prüfungen müssen deshalb den Text-Index aus 3b.2 wiederverwenden und je Archiv inkrementell laufen.

---

## 1. Katalog der Prüfungen

Spalten: **Code** (stabiler Schlüssel, für Ignorieren und Tests) · **Quellen** (Asset-Arten/Felder) · **Regel** · **Schwere** (Fehler / Warnung / Hinweis) · **Meldung** (deutsch, mit Ausweg) · **Ziel** (Navigation, `NavTarget`) · **Braucht** (RI = RefIndex, R = rotom-Quellen/übernommenes Projekt, T = dekodierter Text, B = Build-Zustand, — = nur Store) · **Vanilla Pt DE** (Treffer auf unverändertem Spiel).

Schwere-Regel für den Katalog: **Fehler** = das Spiel liest Müll, stürzt ab, bleibt hängen oder der Build bricht; **Warnung** = funktioniert, aber sehr wahrscheinlich nicht so gemeint; **Hinweis** = kann Absicht sein. Heute kennt der Kernel nur `Severity::{Error, Warning}` (`text.rs:43`); das Prüflicht aus 3c spricht schon von „1 Hinweis“ → dritte Stufe `Info` („Hinweis“) einführen (§4.2).

### 1.1 Warps

Quellen: `Event` (`warp_events`: `x`, `z` absolut in der Matrix des Headers, `dest_header_id`, `dest_warp_id`, HGSS `height`), `HeaderTable` (`event_file_id`, `matrix_id`), `Matrix` (`maps`), `Map` (Permissions `kind`/`collision`), `tiles.rs::warp_fires`.

| Code | Regel | Schwere | Meldung | Ziel | Braucht | Vanilla |
|---|---|---|---|---|---|---|
| `warp.dest-header` | `dest_header_id ≥ Header-Anzahl` und ≠ `0x0FFF` | Fehler | „Die Tür führt ins Leere: Header 0712 gibt es nicht. Wähle ein Ziel oder lösche die Tür.“ | `event` (warp) | RI (`Header`) | 0 (6 × `0x0FFF`, s. u.) |
| `warp.dest-warp` | Ziel-Header gültig, aber `dest_warp_id ≥` Warps seiner Eventdatei | Fehler | „Die Tür führt zu Warp 3 in Zweiblattdorf, dort gibt es nur 2. Wähle einen vorhandenen Ziel-Warp.“ | `event` (warp); zweites Ziel: Eventdatei des Ziels | RI (`Warp(file, i)`) | 0 |
| `warp.off-map` | Zelle `(x/32, z/32)` außerhalb der Matrix oder Zelle leer (`0xFFFF`) | Fehler | „Der Warp steht außerhalb der Karte (Zelle 5, 3 ist leer). Schiebe ihn auf die Karte oder lösche ihn.“ | `event` | — | 0 |
| `warp.never-fires` | `warp_fires(kind, collision) == false` **und** kein Warp führt hierher | Hinweis (mit Ausgangslage, §4.5) | „Dieser Warp löst nie aus: Die Kachel ist weder Tür noch Treppe, und kein anderer Warp führt hierher. Male eine Tür- oder Treppenkachel oder lösche den Warp.“ | `event`; zweites Ziel `matrixCell` | RI (eingehende Warps) | 10 (von 57 auf „falscher“ Kachel; 47 sind reine Ankunft) |
| `warp.arrival-stuck` | Ziel-Warp-Kachel und alle vier Nachbarn blockiert oder Wasser (DSPRE `DeadEnd`/`WarpTrouble`) | Warnung | „Wer durch diese Tür kommt, steht fest: Rund um Warp 1 in Route 201 ist alles blockiert. Öffne eine Nachbarkachel.“ | `event` (Ziel-Warp) | — | nicht gemessen |
| `warp.one-way` | Ziel-Warp führt nicht zu einem Header mit dieser Eventdatei zurück | — (keine Projektprüfung; nur Inspector-Zeile „Rückweg: …“) | — | — | RI | 118 (Union-Raum 0466, Arenen, Aufzüge) |

Befunde:
- `dest_header_id = 0x0FFF` kommt 6× vor (Eventdateien 17, 104, 113, 141, 442, 492; Header C01R0208, C05R0103, C05R0803, C07R0206, T07R0103, C08R0802). Die Namen sprechen für Aufzüge mit dynamischem Ziel **[unverifiziert]**. Die Prüfung muss `0x0FFF` ausnehmen (Konstante mit Quelle in den Metadaten).
- Löschen eines Warps verschiebt die späteren Indizes (`world/events.ts` `WARP_RENUMBER_NOTE`). `warp.dest-warp` fängt genau diese Folge.
- `warp_fires` existiert schon im Kernel und im Frontend (`warpTile.ts` zeigt den Hinweis inline). Die Projektprüfung erweitert ihn um „niemand führt hin“; sonst wären 47 Vanilla-Ankunftspunkte gemeldet.

### 1.2 Events

Quellen: `Event` (`object_events.script`, `trainer_type`, `hidden_flag`, `local_id`, `x`, `z`, `y`; `bg_events.script`; `coord_events.script`, `var`), `HeaderTable` (`script_file_id`, `event_file_id`, `matrix_id`), Script-Slots (übernommen: `scripts::declared_slots` über die `.rotom`-Quelle; sonst die u32-Sprungtabelle bis `0xFD13` im Member), Trainer-Anzahl.

| Code | Regel | Schwere | Meldung | Ziel | Braucht | Vanilla |
|---|---|---|---|---|---|---|
| `event.script-missing` | Script `n` mit `1 ≤ n < 2000` und Slot `#n` fehlt in der Scriptdatei des Headers (für **jeden** Header, der die Eventdatei nutzt) | Fehler | „Die Figur ruft Script #12, Scriptdatei 0042 hat nur 9. Wähle ein Script oder lege mit „Neues Script“ eines an.“ | `event`; zweites Ziel `script` | RI (`ScriptSlot`) | Figuren 0, Trigger 0, Schilder 1 |
| `event.common-missing` | `2000 ≤ n` außerhalb aller Bereiche der Common-Tabelle (uxie `GlobalScriptTable::lookup` liefert nichts; Trainer- und Item-Bereiche ausgenommen) | Fehler | „Die Figur ruft das gemeinsame Script 2741, das es nicht gibt. Wähle ein Script.“ | `event` | — (uxie) | nicht gemessen |
| `event.trainer-missing` | Script 3000–4999 → Trainer `n − 3000 + 1 ≥` Trainer-Anzahl | Fehler | „Die Figur kämpft als Trainer 931, den es nicht gibt. Wähle einen Trainer oder lege einen an.“ | `event`; zweites Ziel Trainer | RI (`Trainer`, in 3b.3 vorgesehen) | 0 (407 Trainer-Figuren) |
| `event.trainer-blind` | Trainer-Script, aber `trainer_type == 0` | Warnung | „Die Figur hat ein Trainer-Script, sieht aber niemanden (Trainer-Typ 0). Setze den Trainer-Typ, sonst kämpft sie nur, wenn man sie anspricht.“ | `event` | — | 0 |
| `event.trigger-empty` | Trigger (`coord`) mit Script 0 | Warnung | „Der Auslöser hat kein Script und tut nichts. Wähle ein Script oder lösche ihn.“ | `event` | — | 0 (Figuren mit Script 0: 202, das ist Deko und bleibt erlaubt) |
| `event.off-map` | Figur/Schild/Trigger außerhalb der Matrix oder auf leerer Zelle | Fehler | „Die Figur steht außerhalb der Karte. Schiebe sie auf die Karte oder lösche sie.“ | `event` | — | 0 |
| `event.same-spot` | Zwei Figuren auf derselben Kachel und Höhe, **beide ohne** `hidden_flag` | Warnung | „Zwei Figuren stehen auf derselben Kachel (12, 7). Verschiebe eine oder gib einer eine Flag.“ | `event` | — | 2 (mit Flags: 30, die bleiben stumm) |
| `event.shared-file` | Eventdatei von Headern mit **verschiedenen** Scriptdateien benutzt | Hinweis | „Eventdatei 0000 wird von 60 Headern mit verschiedenen Scriptdateien genutzt; Script-Nummern treffen je Header ein anderes Script.“ | `header` | — | 1 (Eventdatei 0, Dummy-Header) |
| `event.hidden-flag-range`, `event.trigger-var-range` | Flag/Variable außerhalb der Bereiche der Familie | Warnung | „Flag 0x9000 liegt außerhalb der Flags von Platin. Wähle eine Flag aus der Liste.“ | `event` | — (Bereichs-Metadaten) | nicht gemessen; Bereiche **[unverifiziert]** |

Nicht als Projektprüfung: `local_id` doppelt in einer Datei (40 Vanilla-Treffer), `trainer_type ≠ 0` ohne Trainer-Script (29), Figur auf blockierter Kachel (Theken-NPCs; DSPRE prüft das nur nach einem Kartenimport, s. §3.1 — gehört in den Import-Bericht von 3b.4, nicht ins Prüflicht).

### 1.3 Header

Quellen: `HeaderTable` (alle Indexfelder), Member-Anzahlen je Archiv, Script-Quellen (Art `.rotom`/`.json`), Level-Script-JSON.

| Code | Regel | Schwere | Meldung | Ziel | Braucht | Vanilla |
|---|---|---|---|---|---|---|
| `header.ref-range` | `matrix_id`, `event_file_id`, `script_file_id`, `level_script_id`, `text_archive_id`, `area_data_id` ≥ Member-Anzahl (DSPRE `ValidateHeaders`) | Fehler | „Header 0412 zeigt auf Eventdatei 0600, die es nicht gibt (nur 534). Wähle eine vorhandene oder lege eine an.“ | `header` (Feld) | — (Anzahlen) | 0 |
| `header.wild-dangling` | `wild_pokemon` ≠ keine (Pt `0xFFFF`, HGSS `0xFF`) und ≥ Begegnungsdateien; HGSS ≥ 255 | Fehler | „Die Begegnungen von Route 201 zeigen auf Datei 0190, die es nicht gibt. Wähle eine Datei oder „Keine“.“ | `header` | RI (`Encounter`) | 0 |
| `header.script-kind` | `script_file_id` zeigt auf ein Level-Script (`.json`) oder `level_script_id` auf eine Scriptdatei (`.rotom`) | Fehler | „Die Scriptdatei von Header 0412 ist ein Level-Script. Wähle eine Scriptdatei.“ | `header` | R | 0 |
| `header.text-split` | Eine Scriptdatei wird von Headern mit **verschiedenen** Textarchiven genutzt | Warnung | „Scriptdatei 0042 läuft mit den Textarchiven 0199 und 0201. Nachrichten-Nummern treffen nur in einem davon. Gib beiden Headern dasselbe Textarchiv oder trenne die Scriptdatei.“ | `header` | — | 0 (gemessen über alle 575 Scriptdateien) |
| `header.level-target` | Script-Nummer < 2000 eines Level-Script-Eintrags (`header_entries`, `var_conditions`) fehlt als `#n` in der Scriptdatei | Fehler | „Das Level-Script von Header 0186 startet Script #1, das es in Scriptdatei 0197 nicht gibt. Wähle ein Script oder lege eines an.“ | `script` (Level-Script, JSON) | R | 1 (Header 186) |
| `header.level-message` | Ein `on_load`/`on_resume`/`on_transition`-Script zeigt (direkt oder über `Call`) eine Nachricht | Fehler | „Das Level-Script zeigt beim Laden eine Nachricht; die Karte bliebe schwarz. Nimm eine Variablen-Bedingung (on_var_condition).“ | `script` | R | nicht gemessen (Regel aus dem 3b.2-Gate, Spec 3b.2 §1) |
| `header.no-map` | Header kommt in keiner Matrix-Zelle seiner Matrix vor (Matrizen mit Header-Raster) bzw. Innenraum-Matrix ohne Karte | Warnung | „Header 0412 hat keine Karte: Matrix 0012 enthält ihn nicht. Setze ihn in eine Zelle.“ | `header` | RI | nicht gemessen |

### 1.4 Scripts

Was rotom und rotom-lsp schon melden, steht in §2; hier nur, was sie **nicht** prüfen. Quellen: `.rotom`-Quellen (rotom-Parser, `Parser::new_fallible`), Befehlsdatenbank (`metadata/scrcmd/*_v2.json`, Parametertypen `msg_id`, `flag`, `var`, `trainer_id`, `map_id`, `item`, `species`), Textarchive (Nachrichtenzahl), Header-Tabelle (Script → Textarchiv), uxie `GlobalScriptTable`.

| Code | Regel | Schwere | Meldung | Ziel | Braucht | Vanilla |
|---|---|---|---|---|---|---|
| `script.message-missing` | Text-Slot-Argument (rotom-lsp `is_text_slot`: Parametername `text_slot` oder Typ `MsgId`) als Zahl `N ≥` Nachrichtenzahl des Textarchivs der Scriptdatei | Fehler | „Script #3 zeigt Nachricht 41, Textarchiv 0199 hat nur 40. Lege die Nachricht an oder wähle eine vorhandene.“ | `script` (Zeile) | R, RI (`Message`) | **0 von 4410** (590 weitere in Common-Scriptdateien ohne Header, Archiv dort über die Common-Tabelle) |
| `script.message-empty` | Referenzierte Nachricht ist leer | Hinweis | „Script #3 zeigt eine leere Nachricht (0199 · 12). Schreibe einen Text.“ | `textArchive` (Nachricht) | R, T | Pt 0, HG 3 leere Dialognachrichten (Referenzen nicht geprüft) |
| `script.warp-target` | `Warp map door x y dir` mit Zahl `map ≥` Header-Anzahl; `x`/`y` außerhalb der Zielmatrix | Fehler / Warnung | „Script #5 schickt den Spieler zu Header 0900, den es nicht gibt. Wähle ein Ziel.“ | `script` | R | nicht gemessen |
| `script.trainer-target` | Parameter vom Typ `trainer_id` (z. B. `GoToIfDefeated`) bzw. `TrainerBattle` (eigene Metadaten, DB typisiert ihn nicht) ≥ Trainer-Anzahl | Fehler | „Script #8 fragt nach Trainer 999, den es nicht gibt. Wähle einen Trainer.“ | `script` | R, RI | nicht gemessen |
| `script.id-range` | Parameter `item`/`species`/Attacke ≥ Anzahl (DB typisiert nur `GoToIfCannotFitItem`, `PokemonCryAndMessage`, HGSS `GiveItemNoCheck` u. a.; Rest über eigene Liste) | Fehler | „Script #2 gibt Gegenstand 600, den es nicht gibt. Wähle einen Gegenstand.“ | `script` | R | nicht gemessen |
| `script.flag-range` | Flag- oder Variablen-Zahl ohne Namen in der DB und außerhalb der Bereiche der Familie (DB Pt: 4098 Flags, IDs 0…0x410D; 315 Variablen 0x013D…0x8010) | Warnung | „Script #4 setzt Flag 0x5000; Platin kennt keine solche Flag. Wähle eine Flag aus der Liste.“ | `script` | R | nicht gemessen; Bereiche **[unverifiziert]** (pret `flags.h`/`vars.h` als Quelle) |
| `script.common-missing` | `CallCommonScript n` / `Module::Label` ohne Ziel (nur Zahlen; `Module::Label` prüft rotom-lsp schon) | Fehler | „Script #1 ruft gemeinsames Script 2999, das es nicht gibt.“ | `script` | R | nicht gemessen |
| `script.rotom-all` | rotoms eigene Analyse (`Analyzer::with_database`, wie `rotom-lsp/src/diagnostics.rs`) über **alle** Quellen, nicht nur offene | wie rotom | rotoms Meldung (englisch; deutsche Hülle „rotom: …“) | `script` | R | 0 erwartet nach `adopt` **[unverifiziert]** |
| `script.unused` | Script-Slot von keinem Event, Level-Script oder Script benutzt | — (aus; Code ruft Scripts auch direkt) | — | — | R, RI | nicht gemessen |

### 1.5 Text

Quellen: `TextArchive` (über den Store, `text_winner`), Header-Tabelle (welche Archive Dialoge sind: die 404 `text_archive_id`-Werte in Pt, 398 in HGSS), chatot `measure_line_width`, `validate_message`, `word_wrap`.

| Code | Regel | Schwere | Meldung | Ziel | Braucht | Vanilla Pt / HG DE / HG US |
|---|---|---|---|---|---|---|
| `text.too-wide` | Dialogarchiv: eine Zeile breiter als **216 px** (chatot-Glyphenbreiten) | Warnung | „Zeile 2 ist 231 px breit, ins Textfeld passen 216. Brich die Zeile um („Umbrechen“).“ | `textArchive` (Nachricht) | T | 90 / 114 / 113 (bei 200 px: 1810 / 1891 / 1157) |
| `text.three-lines` | Dialogarchiv: mehr als zwei Zeilen zwischen `\r`/`\f` (das Textfeld hat zwei Zeilen; chatot `word_wrap` setzt deshalb auf der zweiten Zeile `\r`) | Warnung | „Nachricht 12 hat drei Zeilen ohne Seitenwechsel. Setze nach Zeile 2 ein \r.“ | `textArchive` | T | 8 / 10 / 19 |
| `text.encode` | `validate_message` meldet etwas (unbekanntes Zeichen, unbekannter Steuercode) | Fehler (der Build bricht in `encode_stale`) | „Nachricht 12: unbekannter Steuercode {COLR}. Korrigiere ihn; so lässt sich das Archiv nicht bauen.“ | `textArchive` | T | 0 / 0 / 0 |
| `text.json-broken` | `expanded/textArchives/NNNN.json` nicht lesbar (heute schon Fehler in `encode_stale`) | Fehler | „Textarchiv 0199 lässt sich nicht lesen: … Repariere die Datei oder lade sie aus dem ROM neu.“ | `textArchive` | — | 0 |
| `text.name-length` | Namensbänke mit fester Grenze (Trainernamen 7, Pokémon-Spitznamen 10, …) | → bei den Datenprüfungen (§1.6) | | | | |

Die 216-px-Kante ist gemessen (Histogramm der breitesten Zeile je Dialognachricht: 212–215 px: 451, 216–219 px: 69, 220–223 px: 2 in Pt DE; HG gleich). 27 Kacheln × 8 px = 216 px für das Feld-Textfenster ist die naheliegende Erklärung **[unverifiziert; pokeplatinum `Window_Add(…, 27, 4, …)` nicht nachgeschlagen]**. Zeilen mit `{STRVAR_…}` zählen in chatot 0 px (`format.rs:205`); eine Prüfung kann für Platzhalter eine Mindestbreite annehmen (Spielername 7 Zeichen) — **Vorschlag**, nicht gemessen. Beschreibungstexte (Attacken, Items) haben andere Kästen (DSPRE-Avalonia: Attacken-Beschreibung 120 px × 5 Zeilen, Tasche 216/236 px × 3 Zeilen, `MoveDataEditorViewModel.cs:271`, `MoveTextScreens.cs:28`); die gehören in die Daten-Editoren, nicht in `text.too-wide`.

### 1.6 Trainer

Quellen: `Trainer` (`TrainerDoc`: `format`, `party`, `class`, `items`, `battle_type`), Namensbank (Pt 618 / HGSS 729), Klassenbank, Anzahlen Species/Items/Moves, `trtbl`/`trtblofs`, Flag-Kapazität (`arm9_tables.yaml`).

| Code | Regel | Schwere | Meldung | Ziel | Braucht | Vanilla |
|---|---|---|---|---|---|---|
| `trainer.no-party` | Partyzahl 0, Trainer ≠ 0 | Fehler, wenn ein Event/Script ihn nutzt, sonst Warnung | „Wanderer Jens hat kein Pokémon; der Kampf stürzt ab. Füge ein Pokémon hinzu.“ | Trainer | RI (Nutzer) | 1 (Trainer 0, Dummy, ausgenommen) |
| `trainer.empty-species` | Party-Platz mit Art 0 | Fehler | „Platz 2 von Wanderer Jens hat kein Pokémon. Wähle eine Art oder entferne den Platz.“ | Trainer (Platz) | — | 0 |
| `trainer.id-range` | Art, Item, Attacke, Klasse ≥ Anzahl (DSPRE `ValidateTrainers`) | Fehler | „Platz 1 von Wanderer Jens trägt Gegenstand 600, den es nicht gibt.“ | Trainer (Feld) | — | nicht gemessen (DSPRE prüft es) |
| `trainer.no-moves` | Attacken-Schalter an, alle vier Attacken 0 | Warnung | „Platz 1 hat eigene Attacken eingeschaltet, aber keine gewählt; das Pokémon kann nichts. Wähle Attacken oder schalte sie aus.“ | Trainer | — | 0 |
| `trainer.level` | Level 0 oder > 100 | Warnung | „Platz 3 hat Level 0. Setze ein Level von 1 bis 100.“ | Trainer | — | nicht gemessen |
| `trainer.double-one` | Doppelkampf mit weniger als 2 Pokémon | Warnung | „Wanderer Jens kämpft im Doppel, hat aber nur ein Pokémon. Füge eines hinzu oder stelle auf Einzel.“ | Trainer | — | nicht gemessen |
| `trainer.name-length` | Name (ohne `{TRAINER_NAME:…}`) > `trainer_name_max` (7) | Fehler | „Der Name „Wanderjens“ hat 10 Zeichen; das Spiel hat Platz für 7. Kürze ihn.“ | Trainer (Name) | T | nicht gemessen |
| `trainer.flag-capacity` | Trainer-ID über der Flag-Kapazität (HGSS 734) | Warnung | „Trainer 736 hat keine eigene Sieg-Flag; nach dem Kampf gilt er nicht als besiegt.“ | Trainer | — | HGSS: 4 Vanilla-Datensätze darüber (Spec 3b.3 §2) |
| `trainer.texts` | `len(trtbl)/4 ≠` Kampftext-Nachrichten oder `trtblofs` < Trainerzahl | Fehler | „Die Trainertexte passen nicht zur Tabelle; Kampftexte sind gesperrt.“ | Trainer | T | 0 (Invariante gated in 3b.3) |
| `trainer.unused` | kein Event, kein Script nutzt den Trainer | — (aus) | | | RI, R | |

### 1.7 Begegnungen

Quellen: `Encounter` (Pt 424 B, HGSS 196 B), `encounter_rules.yaml`, Header-Tabelle (`wild_pokemon`). Die Regeln aus Spec 3b.3 §10 werden projektweit gesammelt (heute nur inline und je Datei im Prüflicht, Quelle „Begegnungen“).

| Code | Regel | Schwere | Meldung | Ziel | Braucht | Vanilla |
|---|---|---|---|---|---|---|
| `encounter.empty-slot` | Rate > 0 und ein Slot mit Art 0 | Warnung | „Route 201, Gras Platz 4 ist leer, obwohl hier Begegnungen kommen; das ergibt ein Glitch-Pokémon. Wähle eine Art.“ | Begegnung (Slot) | — | 0 (Gras) |
| `encounter.level-zero` | Art gesetzt, Level 0 | Warnung | „Gras Platz 2 hat Level 0. Setze ein Level.“ | Begegnung | — | 0 |
| `encounter.min-max` | min > max | Hinweis | „Wasser Platz 1: min 30 > max 20; das Spiel tauscht sie.“ | Begegnung | — | 0 (1078 Pt-Wasserslots, Spec 3b.3 §4) |
| `encounter.species-range` | Art über der höchsten Art (Formen-Kodierung beachten) | Fehler | „Gras Platz 1 zeigt Art 700, die es nicht gibt.“ | Begegnung | — | nicht gemessen; Formen-Bits **[unverifiziert]** |
| `encounter.empty-replacement` | Ersatzliste (Schwarm, Radar, Tag/Nacht, Dual-Slot, HGSS Radio) leer bei Rate > 0 | Warnung | „Der Schwarm ersetzt Gras Platz 1, ist aber leer.“ | Begegnung | — | Vanilla ist das teils so (Spec 3b.3 §10 „leere Slots sind Vanilla“) → Hinweis oder Ausgangslage |
| `encounter.unused` | keine Header nutzt die Datei | Hinweis (aus) | | | RI | |

### 1.8 Daten (Pokémon, Attacken, Gegenstände)

| Code | Regel | Schwere | Meldung | Ziel | Braucht | Vanilla |
|---|---|---|---|---|---|---|
| `data.evo-target` | Entwicklung: Ziel ≥ Personal-Anzahl; Parameter Item/Attacke/Art außer Bereich (Methode → Parameterbedeutung) | Fehler | „Staravia entwickelt sich zu Art 600, die es nicht gibt. Wähle ein Ziel.“ | Pokédex (Entwicklung) | RI | 0 |
| `data.learnset-limit` | mehr als 20 Einträge | Fehler | „Das Lernset von Staravia hat 21 Einträge; das Spiel liest 20.“ | Pokédex (Lernset) | — | 0 |
| `data.learnset-move` | Attacke ≥ Anzahl | Fehler | | Pokédex | — | nicht gemessen |
| `data.wild-item` | Personal `item1`/`item2` ≥ Item-Anzahl | Fehler | | Pokédex | — | nicht gemessen |
| `data.name-length` | Arten-, Attacken-, Item-Name über der Bankgrenze | Warnung | | Texte | T | Grenzen **[unverifiziert]** |

### 1.9 Matrizen, Karten, Area-Data

| Code | Regel | Schwere | Meldung | Ziel | Braucht | Vanilla |
|---|---|---|---|---|---|---|
| `matrix.map-range` | Karten-ID ≥ Karten-Anzahl | Fehler | „Zelle 4, 7 von Matrix 0000 zeigt auf Karte 0700, die es nicht gibt. Wähle eine Karte oder leere die Zelle.“ | `matrixCell` | — | 0 |
| `matrix.header-range` | Header-Zelle ≥ Header-Anzahl | Fehler | „Zelle 4, 7 gehört zu Header 0900, den es nicht gibt.“ | `matrixCell` | — | 0 |
| `matrix.header-elsewhere` | Header-Zelle nennt einen Header, dessen `matrix_id` eine andere Matrix ist | Warnung | „Zelle 4, 7 nennt Header 0415, der eine andere Matrix benutzt; Events erscheinen dort nicht.“ | `matrixCell` | — | nicht gemessen |
| `area.tileset-range` | Area-Data `buildings_tileset`/`map_tileset` ≥ Anzahl der Textur-Archive | Fehler | „Area-Data 0012 nutzt Kartentextur 0090, die es nicht gibt.“ | `area` | — | nicht gemessen |
| `map.building-range` | Gebäude einer Karte nennt ein Modell ≥ Anzahl | Fehler | „Gebäude 3 auf Karte 0187 nutzt Modell 0700, das es nicht gibt.“ | `building` | — | nicht gemessen |
| `asset.unreadable` | Ein Asset lässt sich nicht lesen oder ist gesperrt (`list_warps`/`list_matrices` `unreadable`, `unwritable_reason`) | Fehler | „Eventdatei 0411 lässt sich nicht lesen: … Die übrigen Prüfungen laufen ohne sie.“ | Asset | — | 0 |

### 1.10 Bauen

| Code | Regel | Schwere | Meldung | Ziel | Braucht |
|---|---|---|---|---|---|
| `build.stale-scripts` | Quellen, deren Hash nicht zu `compile-state.json` passt | Hinweis | „3 Scripts sind seit dem letzten Bauen geändert. „Bauen“ übernimmt sie.“ | Liste/Script | R, B |
| `build.stale-text` | Text-JSON neuer als Member (DSPREs `--newer`) | Hinweis | „2 Textarchive sind seit dem letzten Bauen geändert.“ | Text | B |
| `build.member-newer` | Script-Member neuer als seine Quelle (Änderung in DSPRE `main` oder von Hand) | Warnung | „Scriptdatei 0042 wurde außerhalb geändert; beim nächsten Bauen gewinnt die Quelle. Übernimm die Änderung oder verwirf sie.“ | `script` | R, B |
| `build.rom-old` | `out.nds` älter als die letzte Speicherung | Hinweis | „Das ROM ist älter als deine Änderungen. Baue neu, bevor du spielst.“ | Technik › Prüfen und Bauen | B |

---

## 2. Was rotom und rotom-lsp schon prüfen (nicht doppeln)

Quelle: rotom v0.7.0 `fb11503`.

**Beim Compile** (`src/compiler/{parser,analysis}.rs`, `ir/lowerer.rs`, `codegen.rs`; Fehlertypen in `diagnostic/error.rs`):
- Parse: Syntax, Strukturfehler (Parser fehlertolerant, sammelt mehrere).
- Analyse (bricht beim **ersten** Fehler je Datei ab, `analyze(&mut self, …) -> ParseResult<()>`): `Unknown command`, Argumentzahl (`requires at least`, `accepts at most`), Argumentbreite (`is out of range: … does not fit in …` — nur Bitbreite, kein Wertebereich: `SetFlag 65535` gilt als gültig, Test `analysis.rs:2498`), `Jump target … not found`, `Undefined symbol`, doppelte Symbole, Alias-Konflikte, `match`-Subjekt muss Variable ≥ 0x4000 sein, Menü-Regeln (höchstens 28 Einträge, Spalten, `.width()` nur Pt, HGSS ohne `.columns()`/`.scrollable()`), `script slot IDs must start at #1`, Kodierungsfehler in Stringliteralen.
- Lowering: Makro-Expansion, Bedingungen, `string literals require a loaded project workspace`, `failed to write message to archive`, `Diamond/Pearl and Platinum menu text IDs must fit in u8`, reservierte Nachricht 255.
- Codegen: `Undefined label … for relocation`, Opcode fehlt in der DB, Parameter passt nicht in u8/u16/u32.
- Warnungen (`diagnostic/warning.rs`): `UnusedAlias`, `ShadowedAlias`, **`MissingSlot`** (Lücke in den `#n`, der Zeiger des vorigen Scripts wird wiederholt), **`MessageLineTooLong`** (über uxie `format_line_is_too_long` → chatot `DIALOG_LINE_MAX_PX = 200`, nur für **Textliterale**), `VariantConditionUnresolvable`, `MenuScrollableFalseOverridden`, `NormalMenuTooTall`.
- Level-Scripts: `LevelScript::validate` (`decompiler/levelscript.rs:145`): Variable 0, Bedingungen ohne Header-Eintrag, doppeltes `on_var_condition`. **Nicht**: ob die Script-Nummer in der Scriptdatei existiert.

**Live im LSP** (`rotom-lsp/src/diagnostics.rs`, 300 ms Debounce, `server.rs:413`): Parse-Fehler (fehlertolerant) + Analyse + Warnungen + **Common-Script-Referenzen `Module::Label`** (`resolve_global_script_ref`, `project/context.rs:280`). Lowering-Fehler nur mit Span; Codegen-, IO-, DB-Fehler fallen weg (`_ => return None`). Nur für **offene** `.rotom`-Dokumente (`publish_project_diagnostics` iteriert `self.documents.uris()`); keine Arbeitsbereichs-Diagnosen. `message_refs.rs` sammelt `(archive, msg) → Script-Zeilen` für CodeLens in Text-JSONs (Rückwärtsreferenz), meldet aber **nichts**, wenn die Nachricht fehlt; DSPRE `main` zeigt das nur im Hover-Text (`ScriptEditor.cs:551` „Message ID out of range“).

**Folgerung:** Projektprüfungen dürfen keine eigenen Syntax-/Befehls-/Argument-/Label-Prüfungen bauen. `script.rotom-all` (§1.4) nutzt rotoms Analyzer selbst für Dateien, die nicht offen sind, und wird im `ProblemsStore` gegen `compile`-Zeilen dedupliziert (das Muster existiert für `live`, `problems.ts:178`). Eigene Prüfungen setzen nur dort an, wo Projektkontext nötig ist: Nachrichtenzahl, Slots anderer Dateien, Header/Trainer/Item-Anzahlen, Flag-Bereiche, Level-Script-Ziele, Textbreite in Archiven (nicht Literalen). `message_refs.rs` (MIT) ist die Vorlage für `script.message-missing` und gleichzeitig die Quelle für RefIndex-Ziele `Message(archive, idx)`; `rotom-lsp` ist nur Binary, die Logik wäre zu portieren (≈ 450 Zeilen, nutzt nur die rotom-Lib und uxie `Workspace::resolve_message_id`).

---

## 3. Wie andere Werkzeuge projektweite Prüfungen bauen

### 3.1 DSPRE

- **`main` (WinForms, 2.3.4):** keine Projektprüfung. Prüfungen erst beim Navigieren als Dialog: Trainer-ID existiert nicht (`EventEditor.cs:2342`), Ziel-Warp existiert nicht („Warp is not connected“, `EventEditor.cs:2786`, mit „trotzdem öffnen?“), Matrix-Zelle ohne Karte (`MatrixEditor.cs:928`), Begegnungsdatei fehlt (`WildEditorDPPt.cs:106`). Fehler werden also erst beim Klick entdeckt — das Gegenmuster.
- **`feature/avalonia`, „Validation & Where-Used“** (`DSPRE.Avalonia/Avalonia/Data/ProjectIndex.cs`, `ViewModels/Tools/ProjectChecksViewModel.cs`, `Views/Tools/ProjectChecksView.axaml`): eigenes Fenster mit zwei Reitern. *Validation*: Knopf „Run validation“, läuft per `Task.Run` (Knopf gesperrt solange), Ergebnis als `DataGrid` Typ · Wo · Problem, Status „No problems found. Every reference points at something that exists.“. Drei Prüfgruppen: Header-Indizes gegen Member-Anzahlen (mit „none“-Sentinels 0xFF/0xFFFF), Entwicklungen (Ziel und Parameter je Methode, Parameterbedeutung aus dem anpassbaren `LabelStore`), Trainer (Art, Item, Attacke je Platz). Jede Datei einzeln in `try/catch`: eine unlesbare Datei bricht den Lauf nicht ab, ein kaputter Gesamtlauf wird als eigene Zeile gemeldet. *Where used*: Art + ID → Header-Liste, aus demselben Header-Durchlauf. **Kein** Sprung zum Ort, **kein** Beheben, **kein** Ignorieren, **kein** Cache, nur auf Knopfdruck.
- **Kartenimport-Bericht** (`ViewModels/World/MapEditorViewModel.cs:1508–2020`, `EventClash`, `EventsOnUnwalkableSquares`, `WarpTrouble`, `ArrivalsOf`, `PlanFixes`, `MoveEvents`): nach einem Import prüft DSPRE Events auf jetzt blockierten oder Wasser-Kacheln, Warps, deren Tür-/Treppenkachel übermalt wurde oder die in einer Sackgasse enden („nothing beside it can be walked onto“), Fly-/Blackout-Ankunftspunkte, und Kartenränder, die der Nachbar offen hat. Beheben ist **pro Zeile opt-in** (Häkchen „Move“, „Nothing is moved unless the user ticks it“), mit **Vorschau** („will move to 12,7“, „will put its squares back“), dann gesammelt angewendet; danach wird neu bewertet („A restored warp that still leads nowhere is not fixed“). Für 3b.4 doppelt relevant (Kartenimport und Prüfungen).
- **`WhyNot()`** ist kein Validator, sondern ein Gating-Muster: `string? WhyNot()` liefert den Grund, warum ein Editor/eine Tabelle nicht verfügbar ist (`AvaloniaEditorLauncher.cs:38`, `:436–466`, über 200 Fundstellen). Entspricht unserem `unwritable_reason` und dem „nicht verfügbar“ der arm9-Tabellen.
- Inline-Warnungen in Editoren: Lernset über Vanilla-Grenze (`LearnsetEditorViewModel.cs:101` `ExceedsVanillaLimit`), Beschreibungstexte mit Pixelmaß und Zeilenzahl (`MoveDataEditorViewModel.cs:291` „Line 1 runs 4 px past the box“).

### 3.2 Porymap (6.3.1)

- **Warp-Verhalten-Warnung** (`src/editor.cpp:2104–2131`, `src/ui/eventframes.cpp:617`, `src/mainwindow.cpp:3083`): kontinuierlich, aber billig — nur für das bewegte bzw. gewählte Event neu berechnet (bei Verschieben und beim Öffnen des Event-Reiters; der Kommentar nennt offen, dass Malen ohne Reiterwechsel nicht erfasst wird). Warnsymbol am Event im Inspector, Klick öffnet Erklärung mit dem ausdrücklichen Satz, dass nicht jede Warnung ein Fehler ist (Einbahn-Eingänge, per Script geänderte Kacheln), und einem Knopf zu den Einstellungen. Einstellbar: globale Abschaltung (`warp_behavior_warning_disabled`) und eine **Erlaubt-Liste** von Verhalten (`projectConfig.warpBehaviors`, `docsrc/manual/settings-and-options.rst:227`).
- **Ladefehler**: Karten mit ungültigen Daten werden beim Öffnen übersprungen und geloggt; erst beim Öffnen der Karte wird daraus ein Fehler (`project.cpp:296`, `erroredMaps`). Kein Problems-Panel, keine projektweite Prüfung.

### 3.3 Pokémon Studio (develop c27957c)

- **Schema statt Prüfung:** zod-Schemata je Entität; `useZodForm` leitet Eingabegrenzen ab, Fehler inline, Speichern gesperrt (Bericht 3b.3 §3.1). Was das Schema verhindert, muss keine Prüfung finden.
- **Integritätsprüfung beim Laden** (`src/utils/SerializationUtils.ts`, `src/hooks/useProjectLoad/deserializeProjectData.ts`): jede Datei gegen das Schema; nicht konforme Dateien werden **nicht geladen**, gezählt und geloggt; ein Hinweis „{{count}} non-compliant files have been detected and have not been loaded…“ (`assets/i18n/en.json:1302`).
- **Migrationen reparieren automatisch** beim Versionswechsel (`src/migrations/migrationConfig.ts`, z. B. „Add the missing map links“).
- **Hängende Referenzen tolerant** anzeigen („… deleted“/„???“ im Fehlerstil); kein Referenz-Check beim Löschen (`Deletion.tsx`), nur `cannotDelete` für die letzte Entität (`src/hooks/usePage.ts`). Kein Problems-Panel.

### 3.4 Sprachdienste und Linter (allgemein, aus Kenntnis, **[nicht im Code geprüft]**)

LSP-Server (rotom-lsp, rust-analyzer) melden live für offene Dateien, volle Prüfungen laufen auf Speichern oder Befehl; Linter (ESLint, Clippy, Android Lint) haben stabile Regel-Codes, Unterdrückung je Fundstelle und **Baseline-Dateien** (Android Lint `lint-baseline.xml`: Befunde eines Stands werden festgeschrieben, nur neue werden gemeldet).

### 3.5 Fünf Muster zum Übernehmen

1. **Ein Index, zwei Antworten** (DSPRE `ProjectIndex`): derselbe Referenzindex liefert „Verwendet in“ und die hängenden Referenzen. Bei uns: `RefIndex` (3b.3) + Anzahl je Ziel-Art = generische Regel „Ziel existiert nicht“; die Schau zeigt dieselben Daten, die das Prüflicht zählt. Dazu DSPREs Robustheit: jedes Asset einzeln, eine unlesbare Datei wird ein Befund (`asset.unreadable`), kein Abbruch.
2. **Beheben = planen, zeigen, anwenden, neu prüfen** (DSPRE-Importbericht): ein Fix ist opt-in, nennt vorher genau, was passiert („Ziel-Warp auf 0 setzen“, „Nachricht 41 anfügen“), läuft als **ein** Changeset (3b.3 `Store::transaction`, ⌘Z nimmt ihn zurück) und prüft das Subjekt danach neu; bleibt der Befund, gilt er nicht als behoben. Kein Fix ohne eindeutiges Ergebnis; sonst nur „Öffnen“.
3. **Am Objekt, nicht nur in der Liste** (Porymap): derselbe Befund erscheint als Symbol am Event im Inspector und in der Szene, mit Erklärung, die zugibt, wann es Absicht sein kann; je Prüfung abschaltbar, mit Erlaubt-Liste wo sinnvoll (z. B. Kachel-Verhalten für `warp.never-fires`).
4. **Verhindern vor Melden** (Pokémon Studio): harte Grenzen im Editor (Name ≤ 7, Lernset ≤ 20, Level 1–100, Picker statt Zahlenfeld) sind schon 3b.3-Regel; Prüfungen fangen nur, was der Editor nicht verhindern kann (Fremdänderungen in DSPRE, gelöschte Ziele, Importe). Hängende IDs überall tolerant als „fehlt (#0712)“ im Fehlerstil.
5. **Codes, Debounce, Ausgangslage** (LSP/Linter): jede Prüfung hat einen stabilen Code; billige Prüfungen laufen nach jeder Änderung mit Debounce (300 ms wie rotom-lsp), teure auf Speichern oder „Erneut prüfen“; Vanilla-Rauschen wird beim ersten Lauf als **Ausgangslage** angeboten („57 Befunde aus dem Originalspiel ausblenden?“), danach zählt nur Neues.

---

## 4. Architektur-Vorschlag

### 4.1 Kernel-Modul `checks`

```
crates/g4-kernel/src/checks/
  mod.rs        Check-Trait, Registry, Runner, Cache, Ausgangslage/Ignorieren
  warps.rs      warp.*
  events.rs     event.*
  headers.rs    header.*, matrix.*, area.*, map.*
  scripts.rs    script.*  (rotom-Parser, message_refs-Port)
  text.rs       text.*    (über Store-Text-Index)
  data.rs       trainer.*, encounter.*, data.*  (3b.3-Codecs)
  build.rs      build.*
```

```rust
pub enum Severity { Error, Warning, Info }           // text::Severity bekommt Info („Hinweis“)

pub enum Subject {                                   // worüber der Befund spricht = Einheit der Invalidierung
    Header(u16), EventFile(u16), ScriptFile(u16), TextArchive(u16),
    Trainer(u16), Encounter(u16), Species(u16), Matrix(u16), Map(u16), AreaData(u16), Project,
}

pub struct Finding {
    pub code: &'static str,                          // "warp.dest-warp"
    pub severity: Severity,
    pub subject: Subject,
    pub key: String,                                 // stabil innerhalb des Subjekts: "warp 2", "slot 4", "msg 12"
    pub message: String,                             // deutsch, mit Ausweg
    pub target: NavTarget,                           // Kernel-Spiegel des Frontend-NavTarget (+ data-Ziele aus 3b.3)
    pub also: Vec<NavTarget>,                        // zweite Ziele (Zielwarp, Script, Kachel)
    pub fixes: Vec<FixId>,                           // nur deterministische, als Changeset ausführbar
    pub file: Option<(PathBuf, u32, u32)>,           // für Script/Text: Datei, Zeile, Spalte
}

pub trait Check: Sync {
    fn code(&self) -> &'static str;
    fn tier(&self) -> Tier;                          // Edit | Save | Demand (§4.3)
    fn needs(&self) -> Needs;                        // { refs, scripts_adopted, text, build }
    fn subjects(&self, ctx: &Ctx) -> Vec<Subject>;   // alle Subjekte eines Volllaufs
    fn depends_on(&self, changed: &Change, ctx: &Ctx) -> Vec<Subject>; // betroffene Subjekte
    fn run(&self, ctx: &mut Ctx, s: Subject, out: &mut Vec<Finding>);
}
```

`Ctx` hält `&mut Store` (liest wie der RefIndex: geladene Assets mit aktuellem, auch ungespeichertem Wert, sonst Platte), `&mut RefIndex`, `NameTables`, Member-Anzahlen, Header-Tabelle, Script-Slot-Tabelle (lazy, je Scriptdatei gecacht, invalidiert nach `save_script`/`adopt`), Projekt-Metadaten (`check_limits.yaml`: 216 px, `0x0FFF`-Sentinel, Flag-/Variablen-Bereiche je Familie, Trainer-Dummy 0, Namensgrenzen — jede Zahl mit Quelle oder `unverified: true`).

**Ergebnis** als `Problem` mit `source: "check"`: `ProblemDto` bekommt die optionalen Felder `code`, `target`, `also`, `fixes`, `subject`; `severity` lernt `"info"`. `file`/`line`/`col` bleiben für Script- und Textbefunde (Klick springt wie heute über `openScript`), sonst `file = ""`, und das Frontend rendert `subject` als Label („Eventdatei 0411 · Warp 2“). `ProblemsStore` bekommt eine vierte Quelle `check`, ersetzt je `Subject` (wie heute je Datei/Archiv) und dedupliziert `script.rotom-all` gegen `compile`-Zeilen (gleiche Datei, Zeile, Schwere — Muster von `live`).

### 4.2 Befehle

- `run_checks { scope: "all" | { subjects } }` → `ChecksReport { findings, ran_ms, skipped: [{ code, why }] }` (z. B. „Scripts nicht übernommen“, „Text-Index wird gebaut“). „Erneut prüfen“ = `scope: "all"`, Cache verworfen.
- `checks_since { generation }` → Änderungen seit dem letzten Abruf (inkrementell, §4.3); das Frontend fragt nach jeder Store-Änderung (wie `history()`).
- `apply_fix { code, subject, key, fix }` → Changeset-Label, danach `run_checks` für das Subjekt.
- `ignore_finding { code, subject, key, fingerprint }`, `unignore`, `accept_baseline {}`, `checks_settings { disabled: [code] }`.

### 4.3 Wann geprüft wird

- **Tier Edit** (billig, µs–ms je Subjekt): Header, Warps, Events, Matrizen, Area-Data, Trainer, Begegnungen, Daten. Nach jedem Edit/Undo/Redo, Debounce 300 ms, nur betroffene Subjekte.
- **Tier Save** (ms je Datei): Scripts (Parse über rotom), Level-Scripts, Textbreite/Steuercodes je Archiv. Nach `save_script`, `save_level_script`, Text-Speichern, `adopt`; im Hintergrund nach dem Projektöffnen.
- **Tier Demand**: `script.rotom-all`, `build.*`, Volllauf. Auf „Erneut prüfen“, beim Öffnen von Technik › Prüfen und Bauen, und vor „Bauen“ (Ergebnis nur angezeigt).
- **Bauen wird von Prüfungen nicht blockiert** (nur Compile-Fehler blockieren, wie in 3b.2 §6). Der Build-Toast nennt die Fehlerzahl der Prüfungen mit Taste „Probleme zeigen“. Begründung: DSPRE und Porymap blockieren nie; ein falsch-positiver Befund darf den Spieltest nicht verhindern.

**Inkrementell über den RefIndex.** Eine Store-Änderung liefert `changed: Vec<AssetId>` (3b.3a Task 9: `Store::changed_since(gen)`). Betroffene Subjekte = (a) die geänderten Assets selbst als Subjekt; (b) Subjekte mit Referenzen **auf** das Geänderte: `RefIndex::refs(target)` für jedes Ziel im geänderten Asset (z. B. Eventdatei 0411 verliert Warp 2 → alle Warps mit Ziel `Warp(0411, ≥2)`; Textarchiv 0199 verliert die letzte Nachricht → Scripts mit `Message(0199, n)`); (c) Anzahl-Änderung einer Art (`append_member`, Löschen in 3b.4) → alle Referenzen auf Indizes ≥ neuer Anzahl (Index-Bereichsabfrage `refs_at_or_above(kind, n)`); (d) Header-Tabelle geändert → Subjekte, deren Kontext der Header ist (Eventdatei, Scriptdatei, Textarchiv des Headers). Der RefIndex braucht dafür neue Ziele und Quellen:

| Neues Ziel | Quelle(n) |
|---|---|
| `Header(u16)` | Warp `dest_header_id`, Matrix-Header-Zelle, Script `Warp map …` |
| `Warp(event_file, idx)` | Warp `dest_warp_id` (über den Ziel-Header aufgelöst) |
| `EventFile`, `ScriptFile`, `LevelScript`, `TextArchive`, `Matrix`, `AreaData` | Header-Felder |
| `ScriptSlot(file, n)` | Objekt-/Schild-/Trigger-Script (über alle Header der Eventdatei), Level-Script-Einträge, `CallCommonScript` |
| `Message(archive, idx)` | Script-Text-Slots (rotom-lsp `message_refs`-Port) |
| `Map(u16)` | Matrix-Zellen |
| `Trainer` (vorhanden) | zusätzlich Script-Parameter `trainer_id`/`TrainerBattle` |

Invariante wie im RefIndex: `inkrementell == voll` (proptest über zufällige Edit/Undo/Redo-Folgen, gated einmal über die drei Projekte). Der Cache `HashMap<(code, Subject), Vec<Finding>>` liegt in der `Session` neben `RefIndex` und `NameTables`, verworfen bei Projektwechsel, `reload_from_disk` eines Assets und „Erneut prüfen“.

### 4.4 Oberfläche (für die Spec, kurz)

Panel „Probleme“ und Technik › Prüfen und Bauen (3c §7.4) gruppieren nach Bereich (Warps, Events, Header, Scripts, Text, Trainer, Begegnungen, Daten, Raster, Bauen) mit Zählern, Filter Schwere und Quelle, Kopf „Erneut prüfen“ mit Zeit und Dauer des letzten Laufs, Zeile mit „Öffnen“ (`goTo(target)`), „Beheben …“ (nur wenn `fixes` nicht leer; Menü mit Vorschau-Label), „Ignorieren“. Am Objekt: Inspector-Abschnitt „Prüfungen“ für das gewählte Event/Header/Trainer, Marker in der Szene in `status/fehler`/`status/warnung`. Prüflicht zählt `check` mit („2 Fehler, 1 Hinweis“).

**Fixes, die eindeutig sind** (Vorschlag): `warp.dest-warp` → „Auf Warp 0 umbiegen“ (nur wenn das Ziel ≥ 1 Warp hat); `event.script-missing` → „Neues Script anlegen“ (`add_script`, schreibt die Nummer ins Event; Script-Teil nicht rückgängig, wie heute); `script.message-missing` bei `N == Anzahl` → „Nachricht anfügen“; `text.too-wide`/`text.three-lines` → „Umbrechen“ (chatot `format::word_wrap(text, charmap, widths, 216)`, setzt `\n`/`\r` wie das Spiel); `event.trainer-blind` → „Trainer-Typ 1 setzen“; `encounter.min-max` → „Tauschen“. Alles andere nur „Öffnen“.

### 4.5 Ignorieren und Ausgangslage

Datei im Projekt `g4-checks.json` (Wurzel, damit sie in Phase 4 mit ins Git geht; DSPRE fasst sie nicht an): `{ "version": 1, "disabled": ["warp.never-fires"], "ignored": [{ "code", "subject", "key", "fingerprint", "note" }] }`. `fingerprint` = Hash der für die Regel relevanten Werte (z. B. Warp-Ziel und Kachel); ändern sich die Daten, wird der Befund wieder gemeldet. **Ausgangslage:** Der erste Volllauf eines Projekts bietet an, alle Hinweise und Warnungen dieses Stands zu ignorieren („Originalspiel: 102 Befunde ausblenden“), nie Fehler. In Phase 4 kann die Ausgangslage durch den Basis-ROM-Vergleich ersetzt werden (Befund existiert schon im Basis-ROM).

### 4.6 Kosten auf den Vanilla-Projekten

Gemessen mit Python 3 auf diesem Mac (warm = zweiter Lauf, Datei-Cache voll; kalt = erster Lauf nach langer Pause), Rust-Werte für chatot gemessen, übrige Rust-Werte geschätzt (Python-Zeit / 5–20, I/O-lastig). Platin DE: 593 Header, 534 Eventdateien (1213 Warps, 3555 Figuren, 186 Trigger, 682 Schilder), 1124 Script-Member (575 Scripts, 549 Level-Scripts; 1,7 MB `.rotom`-Quelle in der Testkopie, 4792 `Message`-Aufrufe), 724 Textarchive (46 053 Nachrichten), 928 Trainer, 183 Begegnungen, 289 Matrizen, 666 Karten, 508 Personal/Lernset/Entwicklung.

| Prüfgruppe | Eingabe | Volllauf gemessen | Volllauf Rust (geschätzt) | je Subjekt inkrementell |
|---|---|---|---|---|
| `header.*` (ohne Scripts) | 593 × 24 B aus arm9 | Python 6 ms | < 1 ms | µs |
| Eventdateien lesen | 534 Dateien | Python 37 ms warm, 640 ms kalt | 2–8 ms (Store-Cache danach 0) | µs |
| Script-Slot-Tabellen (binär, `0xFD13`) | 1124 Member | Python 74 ms warm, 1,2 s kalt | 5–15 ms | µs (Datei gecacht) |
| `warp.*`, `event.*` inkl. Kachelzugriff | 1213 Warps, 3555 Figuren, Permissions der benutzten Karten | Python 58 ms warm | 3–10 ms | < 0,1 ms je Eventdatei |
| `trainer.*`, `encounter.*`, `data.*`, `matrix.*` | 1856 Trainer-Dateien, 183 + 3 × 508 + 289 Dateien | Python 226 ms warm, 3,5 s kalt | 15–50 ms | µs |
| `script.message-missing` | 575 Quellen, 4410 Referenzen | Python-Regex 1,2 s (ein Lauf, Näherung ohne Parser) | Parser 50–300 ms **[unverifiziert]** | 0,1–1 ms je Datei |
| `header.level-target` | 549 JSON + Slot-Mengen | Python < 1 s | 10–30 ms | µs |
| `text.*` (Breite, Zeilen, Steuercodes) | 724 Archive, 46 053 Nachrichten | Rust release: Dekodieren 0,8–0,95 s warm, 5,1 s kalt; Messen ≈ 0 zusätzlich | 0 zusätzlich, wenn der Text-Index aus 3b.2 (`Store::text_index()`) die dekodierten Nachrichten hält | ≈ 1,2 ms je Archiv dekodieren, Messen µs |
| `script.rotom-all` | 575 Quellen | nicht gemessen | 100–500 ms **[unverifiziert]** | wie LSP je Datei |
| `build.*` | `compile-state.json`, mtimes | nicht gemessen | < 50 ms (Hashes vorhanden) | — |

Folgerungen: (1) Der erste Volllauf nach dem Öffnen ist I/O-gebunden (kalt mehrere Sekunden, vor allem Text); er gehört in den Hintergrund (`spawn_blocking`, abbrechbar, Fortschritt im Prüflicht „Prüft …“) und nach den RefIndex-Aufbau, dessen Lesevorgänge er teilt. (2) Tier-Edit-Prüfungen sind nach dem ersten Lauf vernachlässigbar (< 1 ms je Edit). (3) Der Text-Index muss die dekodierten Nachrichten halten (heute hält er laut Spec 3b.2 §4.1 `(archive, index, lowercase text)` — lowercase reicht für die Breite nicht; Original-Text mitführen). (4) Messungen für die Spec offen: rotom-Parse aller Quellen, RefIndex-Aufbau mit den neuen Zielen (Ziel 3b.3: < 300 ms), Volllauf HGSS.

---

## 5. Offene Punkte für die Spec

- `0x0FFF` als Warp-Ziel: Bedeutung bestätigen (pokeplatinum, Aufzug-Scripts) **[unverifiziert]**.
- 216 px als Textfeldbreite gegen pokeplatinum/pokeheartgold bestätigen; ob 3b.2 (`text::validate`) und die Meldung an rotom/chatot (Konstante 200) mitziehen.
- Flag- und Variablenbereiche je Familie (pret `include/constants/flags.h`, `vars.h`) als Metadaten.
- Trainer ohne Party: stürzt der Kampf wirklich ab (Spec 3b.3 behauptet es) — g4emu-Szenario.
- Script-Parameter mit Trainer-/Item-/Header-Bedeutung, die die DB nicht typisiert (`TrainerBattle`, `Warp`, `AddItem`, `GiveMon`): eigene Liste `scrcmd_refs.yaml` je Familie.
- Script 0 bei Figuren = „kein Script“ (202 Vanilla-Figuren) **[unverifiziert, aber so behandelt]**; Partner-Trainer 5000–6999 → Trainer-Zuordnung **[unverifiziert]**.
- HGSS-Baseline dieser Prüfungen nicht gemessen (nur Text).
