# Phase 3d.3 „Kampfsprites, DS-Varianten, Ei-Attacken“ Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Das Studio liest die Kampfsprites von Pokémon, Formen und Trainerklassen aus den eigenen Archiven des Spiels (NCGR im linearen Modus 1, entschlüsselt mit dem gemessenen Generator, Richtung je Archiv und Familie als Metadatum) und zeigt sie in zwei DS-Bildschirm-Varianten: das Pokédex-Profil in der Pokédex-Schau (vorn/hinten, ♀, schillernd) und die Kampfvorschau mit Trainerbild in der Trainer-Schau; dazu werden Ei-Attacken lesbar und editierbar: HeartGold aus dem NARC-Member, Platin aus Overlay 5 mit Signatur und Budget (Baustein `overlay_tables.yaml`), mit Erkennung des DSPRE-Patches, als Feld `egg_moves` in der Spezies-JSON des Textprojekts, als Reiter „Ei-Attacken“ im Pokédex und über `data_edit species`.

**Architecture:** `g4-formats` bekommt in `ncgr.rs` den Kachelmodus (`TileMode`), die lineare Ablage in `render_rgba` und `decrypt(words, direction)` (reine Funktionen, synthetische Tests); `g4-kernel` ein Modul `data/battle_sprites.rs` nach dem Muster `data/icons.rs` (Art × Platz, Formen-Tabelle und Richtungen aus `metadata/battle_sprites.yaml`, Trainerklasse → Member `5·k + 4`/`5·k + 1`, PNG 80 × 80, Cache je Sitzung) und drei lesende `AssetKind`s für die Sprite-Archive. Die App bekommt die Befehle `get_battle_sprite` und `get_trainer_sprite` (Tauri, Brücke, MCP über `g4_call` wie `get_mon_icon`), die Primitives `DSPokedexProfile` und `DSBattlePreview` über `DSScreen` mit neuen Szenen-Tokens `szene/ds/*` aus der Token-Pipeline, und die Anbindung in Pokédex- und Trainer-Schau. Teil B (nach dem Merge von 3d.1): Codec `data/egg_moves.rs` in `g4-formats` (u16-Folge, byte-gleicher Round-Trip), Kernel `data/overlay_tables.rs` (Lage und Budget je Spielcode) und `data/egg_moves.rs` (Quelle NARC, DSPRE-Patch-NARC oder Overlay-Slice mit Signatur), Store-Art `EggMoves` als Ein-Asset-Art wie `HeaderTable` (`Location::Member` bzw. `Location::Slice` auf `arm9_overlays/ov005.bin`), Textprojekt-Feld mit Einheit `egg_moves` nach dem Muster `Place::Headers`, `DataPatch::EggMoves`, Pokédex-Reiter, `g4k check-data egg_moves`.

**Tech Stack:** Rust (g4-formats, g4-kernel, Tauri 2, `png` 0.18, proptest als dev-dependency, serde_yaml für Metadaten), React 19 mit den Bausteinen aus `apps/studio/src/ui/` (CSS Modules, Tokens `--g4-*`), vitest, Python 3 (`design-system/scripts/check_tokens.py`, `export_tokens.py`), die Messskripte aus `docs/research/2026-10-30-phase-3d/` (nur lesen, Kopien im Scratchpad).

**Spec:** [2026-10-30-phase-3d-erzaehlen-design.md](../specs/2026-10-30-phase-3d-erzaehlen-design.md) §1, §2 (Zeilen Kampfsprites, Ei-Attacken), §3.5, §4 (Zeile 3d.3), §6, §7, §8; [entscheidungen.md](../../research/2026-10-30-phase-3d/entscheidungen.md) (Phasenschnitt Zeile 3d.3; „Flags und Variablen (Gutachten B)“ Zeile Ei-Attacken; „Löschen, Sprites, DS-Varianten“ Zeilen Kampfsprites und DS-Bildschirm-Varianten; „Nicht in 3d“; „Was bewusst nicht entschieden ist“ zu Formen, Offsets, Platin-Sprachen); [gutachten-c-loeschen-sprites-schnitt.md](../../research/2026-10-30-phase-3d/gutachten-c-loeschen-sprites-schnitt.md) §3 (Ort und Aufbau, Verschlüsselung, Lizenz, Aufwand) und §4 (DS-Varianten) mit den Messskripten `gutachten-c-sprites-inventar.py`, `gutachten-c-sprites-krypto.py`, `gutachten-c-proto.rs` (Formel und Zählungen sind die Testwerte); [gutachten-b-flags-variablen-live.md](../../research/2026-10-30-phase-3d/gutachten-b-flags-variablen-live.md) §5 (Ei-Attacken) mit `gutachten-b-eier.py`.

Stand 30.10.2026: `main` bei `3c194c7` (Phase 4 abgeschlossen, Spezifikation 3d). Arbeit im Worktree `$HOME/Projekte/lab/editor/gen4-studio/.claude/worktrees/phase-3d-3`, Branch `phase-3d-3-sprites-ds-eier` (existiert, Basis `3c194c7`). Parallel laufen in eigenen Worktrees 3d.1 (`phase-3d-1-flags-variablen`), 3d.2 (`phase-3d-2-erzaehlen-rahmen`) und 4.5 (`phase-4-5-veroeffentlichen`). Kernel `crates/g4-kernel/src`, Formate `crates/g4-formats/src`, Tauri `apps/studio/src-tauri/src`, Frontend `apps/studio/src`.

## Entscheidungen (wo Spezifikation und Gutachten offen lassen)

| Frage | Entscheidung | Grund |
|---|---|---|
| Sprite-Archive als Arten | drei neue lesende `AssetKind`s `PokemonBattleSprites`, `OtherPokemonBattleSprites`, `TrainerGraphics` (Slots 25–27 nach `MonIcons`, `ALL` 25 → 28; Teil B hängt `EggMoves` als Slot 28 an, `ALL` 29), in `archives.yaml` als benannte Einträge je Familie statt `pack_only` (DP-Pfade aus DSPRE RomInfo, `# unverified`) | `icons.rs` liest seine Archive über `AssetKind` (`unpacked_dir`, `read_packed`); der Build packt benannte Arten wie `pack_only`-Einträge, der gated Test `pack_only_entries_repack_byte_identical` zählt drei weniger |
| Ort der Metadaten | eigene Datei `crates/g4-kernel/metadata/battle_sprites.yaml` (Richtung je Archiv und Familie, Formen-Tabelle), nicht `archives.yaml` | Richtung und Formen sind Eigenschaften der Bilder, nicht der Archivpfade; `archives.yaml` bleibt Pfadtabelle |
| Kachelmodus | `Ncgr.mode: TileMode { Tiled, Linear }` aus Kopf-Offset `0x24` (0 gekachelt, 1 linear); jeder andere Wert ist `FormatError::Unsupported { what: "NCGR tile mode" }`; `render_rgba` legt `Linear` Pixel für Pixel zeilenweise ab (Pixel `(x, y)` im Halbbyte `y · width + x`, bei 8 bpp im Byte) | gemessen: alle Kampfsprites Modus 1, alle Symbole Modus 0 (Gutachten C §3.1, `docs/formats/ncgr_nclr.md`); ein unbekannter Modus soll auffallen, nicht falsch gezeichnet werden |
| Entschlüsselung | `pub fn decrypt(words: &mut [u16], direction: Direction)` in `ncgr.rs`: Startwert das erste Wort der Richtung, je Wort `w ^= (k & 0xFFFF) as u16`, dann `k = k.wrapping_mul(0x41C6_4E6D).wrapping_add(0x6073)`; `Ncgr::decrypt(&mut self, direction)` wandelt die Kachelbytes als Little-Endian-Wörter (ein ungerades Endbyte bleibt); Test-Gegenstück `test_support::encrypt(words, direction, seed)` verlangt ein Startwort 0 im Klartext | Formel aus eigener Messung (Gutachten C §3.2, `gutachten-c-sprites-krypto.py`, `gutachten-c-proto.rs`), kein Code aus DSPRE oder pret; mit Startwort 0 ist das verschlüsselte Startwort der Schlüssel, so lassen sich synthetische Fixtures erzeugen |
| Richtungen | `direction: { pt: { pokemon: forward, other: forward, trainer: forward }, hgss: { pokemon: forward, other: forward, trainer: backward }, dp: { unverified: true, … wie pt } }`; `unverified` liefert `None` (kein Bild, kein Fehler) | gemessen auf Platin DE, HeartGold DE, HeartGold US; DP ungemessen |
| Plätze und Rückfall | Member `6·s + Platz`, Platz 0 Rückseite ♀, 1 Rückseite ♂/einzig, 2 Vorderseite ♀, 3 Vorderseite ♂/einzig; Palette `6·s + 4` normal, `6·s + 5` schillernd, erste Palette; ein leeres Member (0 Byte) heißt „kein eigenes Bild“: dann der Platz des anderen Geschlechts derselben Seite; sind beide leer, `None`; `frame` 0 ist die linke, 1 die rechte Hälfte (je 80 × 80) des 160 × 80-Bilds | Gutachten C §3.1 (66 Arten ohne Platz 0/2, 16 ohne Platz 1/3); die Oberfläche soll nie ein leeres Bild zeigen |
| ♀-Umschalter | `data_get species` liefert `sprite_slots: Option<SpriteSlotsDto { female: bool, male: bool }>` (ob Platz 0/2 bzw. 1/3 ein Bild haben); die Schau sperrt ♀ mit „Kein eigenes Bild für ♀“ | der Befehl fällt still auf das andere Geschlecht zurück, die Oberfläche muss den Rückfall kennen |
| Formen | `form` ist der Formindex des Spiels (6-Bit-Feld der Party, Unown 0–27, Arceus 0–17, Deoxys 1–3 …); die Formen-Tabelle `forms` in `battle_sprites.yaml` ist eine Liste `{ species, form, personal?, back?, front?, female_back?, female_front?, palette, shiny, unverified? }` mit Membern in `otherPokemonBattleSprites`; Personal-Member 496–507 lösen über `personal` auf (`get_battle_sprite { species: 503 }` ist Hitze-Rotom), Ei als `{ species: 494, form: 0, personal: 494 }`; eine Form ohne Eintrag zeigt die Grundform; Task 3 belegt mindestens die Einträge für 494 und 496–507 am Augenschein, weitere Formen so weit die Zeit reicht, Unsicheres `unverified: true` (dann Grundform) | Entscheidung „Plan-Task am Augenschein belegen“; die Personal-Nummern 496–507 sind die, die Pokédex-Liste und Trainer-Teams kennen |
| Trainerbild | Member `5·k + 4` (NCGR linear 160 × 80, linke Hälfte 80 × 80) mit Palette 0 aus Member `5·k + 1` (NCLR, 16 Paletten); `k` ≥ Zahl der Klassen → `None` | Gutachten C §3.1 |
| `pokemonSpriteOffsets` | nicht gelesen; die Vorschau stellt jedes Bild an feste Plätze (Gegner oben rechts, eigenes unten links, Boden darunter) | Entscheidung „feste Lage“; die 89 Byte je Art sind unbelegt |
| Cache | `g4_kernel::data::battle_sprites::BattleSpriteCache` in `Session` (neben `sprites`), Schlüssel `SpriteRequest` bzw. Klasse, Wert `Option<Arc<Vec<u8>>>` (PNG); im Frontend der vorhandene Blob-URL-Cache `data/icons.ts` mit Schlüsseln `battle:<species>:<form>:<side>:<f>:<s>:<frame>` und `trainer:<class>`, Reset beim Projektwechsel wie heute | Muster `icons.rs`/`SpriteCache`; ein Bild kostet 9–27 µs Rendern plus Lesen, der Cache spart das Lesen des NARC |
| MCP | `get_battle_sprite` und `get_trainer_sprite` sind Befehle (`access.rs` `Kind::Read`, `symbolic.rs` `/species` bzw. `/class`), erreichbar über `g4_call` (`--expert`) und die Brücke, kein kuratiertes Werkzeug mit Bild-Inhaltsblock | genau wie `get_mon_icon`/`get_item_icon`; ein Bild-Werkzeug für Agenten ist „Offen“ |
| Pokédex-Text, Kategorie, Größe, Gewicht | bleiben leer mit „folgt“: `text_banks.yaml` kennt keine Zukan-Bänke, die Zukan-Daten sind nicht angebunden, und das Anbinden braucht eine eigene Messung (uxie `TextBankIds`, Höhen- und Gewichtstabellen) | Spezifikation §3.5 „das Profil darf vorerst leer bleiben“; geprüft beim Schreiben dieses Plans, Task 3 prüft nichts mehr |
| DS-Varianten | eigene Primitives über `DSScreen variant="empty"`, die ihren Inhalt in dessen `background` zeichnen (gleicher Rahmen, gleiche ganzzahlige Skalierung, gleiche Spielschrift); `DSPokedexProfile`: links das Bild 80 × 80 auf einem Sockel, rechts Nummer („Nr. 397“), Name, zwei `TypeBadge`s (Größe `s`), darunter Kategorie, Größe, Gewicht mit „folgt“, unten die Textbox mit „Pokédex-Text folgt“; `DSBattlePreview`: Gegner oben rechts mit Statuskasten oben links, eigenes Pokémon unten links mit Statuskasten unten rechts, zwei stilisierte Böden (Ellipsen) aus Tokens, `hp` 0..1 färbt den KP-Balken grün (> 0,5), gelb (> 0,2), rot | Gutachten C §4; `DSScreen` liefert Rahmen, Maße und Skalierung schon |
| Kampfvorschau im Trainer | Gegnerseite zeigt wahlweise das Trainerbild („Trainer“) oder das erste Team-Pokémon („Erstes Pokémon“, Vorderseite, Level aus der Party; `Segmented`); die eigene Seite bleibt leer (kein Statuskasten), weil es keine Spielerdaten gibt; ohne Team nur der Trainer | Gate „Kampfvorschau Trainer und erstes Team-Pokémon“; ein erfundenes Spieler-Pokémon wäre Magie |
| Szenen-Tokens | Gruppe `szene/ds` wächst um `status/flaeche #F7F7FF`, `status/rahmen #4A5263`, `status/text #394252`, `kp/spur #5A6373`, `kp/gruen #42C652`, `kp/gelb #EFCE29`, `kp/rot #DE3142`, `kampf/grund #6BA5DE`, `kampf/boden #7BB54A`, `kampf/boden-rand #529C39` (alle RGB555-treu); `check_tokens.py` prüft `ds/status/text` auf `ds/status/flaeche` wie Textbox und Schild | Entscheidung „neue Szenen-Tokens über die Token-Pipeline“; die Werte liegen auf dem Raster der vorhandenen `ds/*`-Tokens |
| Ei-Attacken: Ort im Store | Ein-Asset-Art `AssetKind::EggMoves` (Index 0) wie `HeaderTable`; `Project::member_location(EggMoves, 0)` wählt: `unpacked/eggMoves/0000` (HGSS; Platin, wenn `files/zcustom/egg_moves.narc` existiert: Pack-Eintrag `egg_moves` mit `unpacked: eggMoves`) als `Location::Member`, sonst Platin `Location::Slice { path: arm9_overlays/ov005.bin, offset, len: budget }` aus `overlay_tables.yaml`, nur wenn die Signatur stimmt; sonst lädt der Store das Asset gesperrt mit Grund (`Loaded::Raw`, `locked`) | Entscheidungen „Ei-Attacken“ (B §5.2); `Location::Slice` und das gesperrte Laden gibt es schon |
| Signatur | erstes Wort `20001`, letztes Wort des Budgets `0xFFFF`, jedes Wort dazwischen `0xFFFF` oder `< 20000 + 600`; `signature_missing(offset)` = „Ei-Attacken: Overlay 5 trägt an 0x2923E keine Tabelle (Signatur fehlt)“ | B §5.2; die Prüfung fängt eine andere Sprachfassung oder einen Patch ab |
| Budget | der Codec trägt `budget: Option<usize>` (Bytes); aus einem `Slice` gelesen ist es die Slice-Länge, aus einem Member `None`; `write()` füllt bis zum Budget mit `0xFFFF`-Wörtern auf (der Code liest bis zum ersten `0xFFFF`) und schlägt mit `budget_full(total_words)` = „Ei-Attacken-Tabelle voll: 1 910 von 1 910 Einträgen“ fehl, wenn Marken + Attacken + 1 mehr Wörter sind als das Budget; „Einträge“ zählen alle Wörter einschließlich Ende | „Tauschen immer, Hinzufügen nur gegen Entfernen“; mit Auffüllen bleibt die Signatur nach einem Entfernen gültig |
| `overlay_tables.yaml` | `egg_moves: { overlay: 5, CPUD: { offset: 0x2923E, budget: 3820 }, CPUE: { unverified: true }, CPUS: …, CPUI: …, CPUF: …, CPUJ: … }`; Kommentar: uxie nennt `0x29222` für eine ungenannte Fassung; `unverified` oder fehlend → gesperrt mit „Ei-Attacken: Lage in Overlay 5 für CPUE nicht geprüft“ | B §5.1; der Baustein `data/overlay_tables.rs` nimmt später Attacken-Lehrer und Läden auf |
| Leere Einträge | eine Marke ohne Attacken bleibt im Codec ein Eintrag mit leerer Liste; `set_moves(species, [])` entfernt den Eintrag; neue Einträge stehen in aufsteigender Artnummer | byte-gleicher Round-Trip; der Codec erfindet nichts |
| Textprojekt-Feld | `SpeciesEntity.egg_moves: Option<Vec<SymRef>>` (`serde(default, skip_serializing_if = "Option::is_none")`): `Some` für jede Art mit Eintrag (auch leer), `None` ohne Eintrag oder wenn die Tabelle nicht lesbar ist; der Export liest die Tabelle einmal über `ExportCtx.egg_moves: Option<&EggMoves>`; der Import der Art-Entität ignoriert das Feld, `import.rs` setzt nach der Art-Schleife die Tabelle aus allen `Some` zusammen und schreibt sie an die Location (nur wenn sie abweicht); im Textprojekt ist `EggMoves 0` die Einheit `egg_moves` (`Place::EggMoves` nach dem Muster `Place::Headers`, Quellen alle `data/species/NNNN.json`), ein Store-Schreiben schreibt nur das Feld der betroffenen JSON neu und zeichnet die berührten Art-Einheiten neu auf | Spezifikation „Feld `egg_moves` in der Spezies-JSON“; die Header-Tabelle ist das Vorbild für „ein Asset, viele Dateien“ |
| Schema-Version | bleibt 1: das Feld ist auslassbar, der Export schreibt es nur, wenn die Tabelle lesbar ist, kein Schlüssel wird umbenannt; `schema.snap` wird mit `G4_UPDATE_SNAPSHOT=1` um `species.egg_moves?` erweitert, `migrate::MIGRATIONS` bekommt keinen Schritt; die erste echte Migration (Version 2, 3d.4) nimmt den Zusatz mit | Vorgabe „keine Erhöhung, wenn das Feld auslassbar ist“; ein älteres Studio lehnt eine Datei mit dem Feld als `textproject.schema` ab, das gilt für jeden Zusatz vor Version 2 und wird dort einmal behoben |
| Basisschicht Platin | `base_layer.rs` vergleicht `arm9_overlays/ov005.bin` mit maskiertem Ei-Attacken-Slice, wie `arm9.bin` mit genullter Header-Tabelle | sonst würde jede Änderung an Ei-Attacken einen Blob `data/raw/arm9_overlays/ov005.bin` erzeugen, der die JSON verdoppelt |
| `g4k check-data` | Art `egg_moves`, `MEMBER` ist die Artnummer, Felder `count` und `move0` … `move15`; liest den NARC `a/2/2/9` Member 0 aus dem ROM; Platin ohne Patch-NARC antwortet „CPUD has no egg_moves archive“ wie heute für fehlende Archive | Gate verlangt nur HeartGold; Overlays aus dem ROM lesen bleibt „Offen“ |
| Label | `data_edit species … EggMoves` zeichnet „<Name>: Ei-Attacken 7 → 8“ (Zahl der Attacken alt → neu; gleich viele: „<Name>: Ei-Attacken getauscht“) auf | ein Schritt je Änderung, lesbar im Verlauf |
| Referenzindex | Ei-Attacken werden **nicht** Quelle von `Target::Move` | `data/refs.rs` gehört 3d.1 (parallel); „Verwendet in“ einer Attacke zählt Ei-Attacken erst nach 3d |
| Rückfallregel | sprengt Teil B den Rahmen M, gilt „nur Lesen für beide Spiele“: entfallen in Task 9 `Store::edit_egg_moves` (die Art lädt, ist aber `NotEditable`), in Task 10 `DataPatch::EggMoves` und das Label, in Task 11 das Schreiben im Import, die Einheit `egg_moves` und die Maske der Basisschicht (der Import ignoriert das Feld, der Export schreibt es weiter), in Task 12 Picker, „Zeile hinzufügen“ und Löschen (Liste und Budget bleiben); `check-data` bleibt; die Gate-Datei nennt die Platin-Punkte „Tauschen“ und „Hinzufügen abgelehnt“ als „nicht in 3d.3, Rückfall“ | Spezifikation §3.5 „wird 3d.3 zu groß, nur Lesen“ |

## Global Constraints

- Arbeitsort ist der Worktree `$HOME/Projekte/lab/editor/gen4-studio/.claude/worktrees/phase-3d-3`, Branch `phase-3d-3-sprites-ds-eier` (existiert, Basis main `3c194c7`); nie `cd` in die Hauptkopie, nie Branch wechseln, nie nach main mergen (Ausnahme: Task 8 „Main mergen“ holt main **in den Worktree**); kein `git stash`. `tools/` ist dort ein Symlink (nie einchecken).
- cargo braucht `export PATH="$HOME/.cargo/bin:/opt/homebrew/opt/rustup/bin:$PATH"`; `pnpm install` einmal in `apps/studio`.
- Commits enden mit `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>`.
- Originale unter `$HOME/Projekte/lab/editor/Roms` nie öffnen oder ändern; Testkopien nur `Roms/_testkopien/3d-3-<zweck>` (`cp -cR` für Ordner, `cp -c` für `.nds`). `G4_ROMS_DIR=$HOME/Projekte/lab/editor/Roms` absolut. Gated Tests lesen `G4_TEST_PROJECT` und schreiben nur auf `vanilla_copy()`.
- Test-Umgebung nur auf `G4_VITE_PORT=1720 G4_BRIDGE_PORT=1721`.
- **Keine ROM-Daten ins Repo: keine Sprite-Bilder, keine PNGs aus dem Spiel, keine Member; gated Tests prüfen Zählungen, keine Pixel; Fixtures nur synthetisch (eigene kleine NCGR/NCLR im Test erzeugen).** Gerenderte Bilder für den Augenschein (Task 3) liegen nur im Scratchpad oder unter `target/`.
- UI nur aus `apps/studio/src/ui/primitives` und Tokens `--g4-*`; `bash scripts/check-styles.sh`, `bash scripts/check-tokens.sh`; deutsche Oberfläche, englische Bezeichner.
- Anhängepunkte (`apps/studio/src-tauri/src/commands/dispatch.rs`, `lib.rs`, `mcp/access.rs`, `mcp/catalog.rs`, `mcp/tools.rs`, `mcp/symbolic.rs`, `crates/g4-kernel/src/lib.rs`, `apps/studio/src/api.ts`) nur anhängen.
- **Parallel laufen 3d.1 (`data/refs.rs`, `textproject/*` außer dem Spezies-Schema in Teil B, `eventFields.ts`), 3d.2 (`shellState.ts`, `Hinge.tsx`, `fields.ts`, Kernel `recipes`, `wizards.rs`, `SceneHost`) und 4.5 (`technik/PublishSurface*`, `patch.rs`, `git.rs`, Titelleiste): diese Dateien in 3d.3 nicht anfassen.** Teil B berührt `textproject/*` erst nach Task 8.
- Vor jedem Commit: `cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, die Tests der berührten Crates; bei Frontend-Änderungen `cd apps/studio && pnpm test && pnpm build`, `bash scripts/check-styles.sh`, `bash scripts/check-tokens.sh`.
- Nicht in 3d.3 (wörtlich aus der Entscheidung): Sprites schreiben oder importieren, NCER/NANR-Animationen, `pokemonSpriteOffsets`, DS-Variante „Laden“, echter Kampfhintergrund (LZ10/NSCR), Ei-Attacken in Platin über das Budget hinaus.
- Gate-Zahlen (Spezifikation §4, Zeile 3d.3): alle Sprites lesen (Platin 1 812 + 157, HGSS 1 812 + 161 Pokémon- und Formenbilder; 105 bzw. 129 Trainerbilder), Mittel der zusammenhängenden Flächen linear ≤ 3, durchsichtige Ecken ≥ 1 778 von 1 812, alle Paletten 16 Farben; ein Sprite ≤ 50 ms im Debug-Build über die Brücke (Zeit geprüft nur unter `G4_TIMING_STRICT=1`, sonst gedruckt).

## Review Focus

- Ein Hack-Projekt hat ein Sprite-Member mit anderer Größe oder einem anderen Kachelmodus (etwa 96 × 96 im Modus 0 aus einem Importwerkzeug): der Befehl liefert das Bild in seiner Größe bzw. `None`, nie eine Panik, und die Schau zeigt den Platzhalter (Task 2 `a_member_of_another_size_or_mode_is_rendered_or_none`, Task 4 `an_unreadable_member_answers_empty_not_an_error`).
- Diamant/Perl oder ein Spielcode ohne geprüfte Richtung: `get_battle_sprite` antwortet leer, die Pokédex-Schau zeigt das Profil mit Platzhalter statt eines Fehlers (Task 2 `an_unverified_family_gives_no_sprite`, Task 6 `without_a_sprite_the_profile_shows_the_placeholder`).
- Platin mit DSPRE-Patch **und** verändertem Overlay (die Tabelle im Overlay ist noch da, der Code liest den NARC): der NARC gilt, das Overlay wird nie beschrieben, `data_get` sagt `source: "patch"` (Task 9 `with_the_patch_narc_the_overlay_is_left_alone`).
- Platin ohne Patch, Overlay einer anderen Sprachfassung oder verschobene Tabelle: die Signatur schlägt fehl, das Asset ist gesperrt mit dem Satz, `data_edit` wird mit `read_only` abgelehnt, nichts wird geschrieben (Task 9 `a_wrong_offset_fails_the_signature_and_locks_the_asset`, Task 10 `egg_moves_edit_on_a_locked_table_is_refused_with_the_reason`).
- Hinzufügen einer Ei-Attacke bei vollem Platin-Budget, auch über MCP: Ablehnung mit „Ei-Attacken-Tabelle voll: 1 910 von 1 910 Einträgen“, kein Verlaufsschritt; Tauschen und „eine weg, eine dazu“ gehen (Task 9 `write_refuses_one_word_over_the_budget`, Task 10 `platinum_add_without_remove_is_refused_swap_and_trade_pass`).

---

## Teil A: Kampfsprites und DS-Varianten (nur lesen)

### Task 1: `g4-formats`: Kachelmodus, lineare Ablage, Entschlüsselung

**Files:**
- Modify: `crates/g4-formats/src/ncgr.rs` (`TileMode`, `Ncgr.mode`, `Direction`, `decrypt`, `Ncgr::decrypt`, linearer Zweig in `render_rgba`), `crates/g4-formats/src/test_support.rs` (`ncgr_bytes_with_mode`, `encrypt`), `crates/g4-formats/tests/ncgr.rs`, `docs/formats/ncgr_nclr.md` (Zeile `0x24` und Abschnitt „Kampfsprites“)

**Interfaces:**
- Produces:

```rust
// ncgr.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TileMode { Tiled, Linear }            // Kopf 0x24: 0 bzw. 1; sonst Unsupported { what: "NCGR tile mode" }
pub struct Ncgr { pub width: usize, pub height: usize, pub bpp: u8, pub mode: TileMode, pub tiles: Vec<u8> }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction { Forward, Backward }
pub const KEY_MUL: u32 = 0x41C6_4E6D;
pub const KEY_ADD: u32 = 0x6073;
/// Decrypts in place: the key starts as the first word in `direction`, every word is XORed with
/// `key & 0xFFFF`, then `key = key * KEY_MUL + KEY_ADD` (wrapping). Measured in gutachten C §3.2.
pub fn decrypt(words: &mut [u16], direction: Direction);
impl Ncgr {
    /// `decrypt` over the tile bytes as little-endian words; a trailing odd byte is left as is.
    pub fn decrypt(&mut self, direction: Direction);
}
pub fn render_rgba(ncgr: &Ncgr, palette: &[u16; 16]) -> Vec<u8>;   // unverändert; Linear: Pixel (x, y) aus Halbbyte y*width+x (8 bpp: Byte)
// test_support.rs (feature test-support)
/// `ncgr_bytes` with the tile mode word at 0x24 (`ncgr_bytes` writes 0).
pub fn ncgr_bytes_with_mode(tiles_x: u16, tiles_y: u16, depth: u32, mode: u32, data: &[u8]) -> Vec<u8>;
/// The inverse of `decrypt` for fixtures: the start word of `direction` must be 0 (it becomes `seed`). Panics otherwise.
pub fn encrypt(words: &mut [u16], direction: Direction, seed: u16);
```

- Richtung `Backward`: Startwort ist das letzte Wort, die Schleife läuft rückwärts; die Formel ist dieselbe.

- [ ] **Step 1: Scheiternde Tests** (`tests/ncgr.rs`, synthetisch):
  - `read_reports_the_tile_mode` (`ncgr_bytes` → `Tiled`; `ncgr_bytes_with_mode(…, 1, …)` → `Linear`; Modus 2 → `Err(Unsupported)`).
  - `linear_mode_renders_pixel_by_pixel` (20 × 10 Kacheln, 4 bpp, Daten so, dass Pixel (9, 0) Index 1 und Pixel (0, 1) Index 2 ist: im linearen Bild liegt (9, 0) im Halbbyte 9 und (0, 1) im Halbbyte 160; gekachelt läge Halbbyte 9 in Kachel 1; `render_rgba` prüft beide Pixel über die Palette).
  - `decrypt_forward_turns_the_start_word_to_zero_and_follows_the_generator` (Wörter `[0x1234, 0x1234 ^ k1, 0x1234 ^ k2]` mit `k1 = (0x1234·KEY_MUL + KEY_ADD) & 0xFFFF`, `k2` der nächste → `[0, 0, 0]`).
  - `decrypt_backward_starts_at_the_last_word` (gespiegelte Folge → `[0, 0, 0]`).
  - `encrypt_then_decrypt_is_identity` (proptest über Länge 1..=400 Wörter, Richtung, Seed; Klartext mit Startwort 0).
  - `ncgr_decrypt_leaves_a_trailing_odd_byte` (Kachelbytes ungerader Länge: nur ein künstlicher Puffer; das letzte Byte bleibt).
  - Vorhandene Tests in `tests/ncgr.rs` und `crates/g4-kernel/tests/icons.rs` bleiben grün (Symbole sind `Tiled`).
- [ ] **Step 2: Lauf** `cargo test -p g4-formats --features test-support --test ncgr` → FAIL (Felder und Funktionen fehlen).
- [ ] **Step 3: Umsetzen** wie unter Interfaces; der lineare Zweig in `render_rgba` ist eine zweite Schleife über `y` und `x`, keine Kachelrechnung.
- [ ] **Step 4: Lauf** → PASS; `cargo test -p g4-kernel --all-features --test icons` grün.
- [ ] **Step 5: Doku** `docs/formats/ncgr_nclr.md`: Zeile `0x24` wird „`u32` Kachelmodus (0: gekachelt, 1: linear, Pixel für Pixel zeilenweise)“; neuer Abschnitt „Kampfsprites“ mit Archiven und DSPRE-Ordnern (Tabelle aus Gutachten C §3.1), Plätzen, Paletten, dem 160 × 80-Bild aus zwei Hälften, der Formel mit Startwert und Richtung je Archiv und Familie, der Messung als Quelle (Skripte neben Gutachten C, Zahlen 68,7 % / 1 778 von 1 812), dem Satz „kein Code aus DSPRE oder pret übernommen“; Trainer `5·k + 4` / `5·k + 1`; Formen verweisen auf `battle_sprites.yaml` (Task 3 trägt die Tabelle nach).
- [ ] **Step 6: Commit** `feat(formats): NCGR tile mode, linear rendering and the battle sprite decryption` (mit Trailer).

### Task 2: Metadaten, Sprite-Arten, Kernel `data/battle_sprites.rs`, gated Zählungen

**Files:**
- Create: `crates/g4-kernel/metadata/battle_sprites.yaml` (Richtungen; `forms: []` bis Task 3), `crates/g4-kernel/src/data/battle_sprites.rs`, `crates/g4-kernel/tests/battle_sprites.rs`
- Modify: `crates/g4-kernel/metadata/archives.yaml` (je Familie `pokemon_battle_sprites`, `other_pokemon_battle_sprites`, `trainer_graphics` als benannte Einträge; aus `pack_only` von `pt` und `hgss` entfernt; `dp`: `files/poketool/pokegra/pokegra.narc`, `otherpoke.narc`, `files/poketool/trgra/trfgra.narc` mit `# unverified`), `crates/g4-kernel/src/project.rs` (`AssetKind::{PokemonBattleSprites, OtherPokemonBattleSprites, TrainerGraphics}` in Slots 25–27, `ALL` 28, `label`, `slot`, `FamilyArchives`-Felder; `is_store_kind` unverändert), `crates/g4-kernel/src/store.rs` (`NotEditable`-Arme), `crates/g4-kernel/src/data/mod.rs` (`pub mod battle_sprites;`), `crates/g4-kernel/src/data/icons.rs` (`member_pair` wird `pub(crate)`), `crates/g4-kernel/src/test_support.rs` (synthetisches CPUE-Projekt: `unpacked/pokemonBattleSprites` mit 18 Membern für die Arten 0–2, `trainerGraphics` mit 10 Membern für die Klassen 0–1, `otherPokemonBattleSprites` leer bis Task 3), `crates/g4-kernel/tests/project.rs` (`asset_kinds_have_twenty_five_slots` → 28, Metadaten-Test +3 je Familie, `pack_only`-Zählung −3 für `pt` und `hgss`), `crates/g4-kernel/tests/build.rs` (falls ein Test die `pack_only`-Zahl kennt)

**Interfaces:**
- Consumes: Task 1 (`TileMode`, `Direction`, `Ncgr::decrypt`, `render_rgba`, `encrypt`, `ncgr_bytes_with_mode`, `nclr_bytes`); `icons::{png, member_pair}`; `Project::{unpacked_dir, read_packed, member_count}`; `species_forms.yaml` über `data::names` (Basisart einer Form).
- Produces:

```rust
// data/battle_sprites.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)] #[serde(rename_all = "snake_case")]
pub enum Side { Front, Back }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SpriteRequest { pub species: u16, pub form: u8, pub side: Side, pub female: bool, pub shiny: bool, pub frame: u8 } // frame 0 | 1
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct Directions { pub pokemon: Direction, pub other: Direction, pub trainer: Direction }
/// From `battle_sprites.yaml`; `None` for a family marked `unverified` (DP).
pub fn directions(family: Family) -> Option<Directions>;
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct FormSprite { pub species: u16, pub form: u8, pub personal: Option<u16>, pub back: Option<u16>, pub front: Option<u16>,
                        pub female_back: Option<u16>, pub female_front: Option<u16>, pub palette: u16, pub shiny: u16, #[serde(default)] pub unverified: bool }
pub fn form_sprite(species: u16, form: u8) -> Option<&'static FormSprite>;        // verified entries only
pub fn form_of_personal(personal: u16) -> Option<(u16, u8)>;                       // 496 → (386, 1); None below 494 or unknown
pub const FRAME: usize = 80;
pub const SLOTS_PER_SPECIES: u16 = 6;
/// Member `6·species + slot`: (Back, female) 0, (Back, male) 1, (Front, female) 2, (Front, male) 3.
pub fn pokemon_member(species: u16, side: Side, female: bool) -> u16;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct SpriteSlots { pub female: bool, pub male: bool }    // whether slots 0/2 resp. 1/3 hold bytes
pub fn sprite_slots(project: &Project, species: u16) -> Result<Option<SpriteSlots>, KernelError>;  // None: no archive or unverified family
/// 80 x 80 PNG of the requested half; falls back to the other gender's slot; `None` when both are empty,
/// the species or class is out of range, the family is unverified or the archive lacks the member.
pub fn battle_sprite(project: &Project, req: SpriteRequest) -> Result<Option<Vec<u8>>, KernelError>;
pub fn trainer_sprite(project: &Project, class: u8) -> Result<Option<Vec<u8>>, KernelError>;
#[derive(Default)]
pub struct BattleSpriteCache { /* HashMap<SpriteRequest, Option<Arc<Vec<u8>>>>, HashMap<u8, Option<Arc<Vec<u8>>>> */ }
impl BattleSpriteCache {
    pub fn new() -> Self;
    pub fn battle_sprite(&mut self, project: &Project, req: SpriteRequest) -> Result<Option<Arc<Vec<u8>>>, KernelError>;
    pub fn trainer_sprite(&mut self, project: &Project, class: u8) -> Result<Option<Arc<Vec<u8>>>, KernelError>;
}
```

- Lesen eines Bildes: Member lesen (`member_pair` bzw. ein Einzelmember über dieselbe Logik) → `Ncgr::read` → `ncgr.decrypt(direction)` → `render_rgba` mit der ersten Palette (Pokémon) bzw. Palette 0 (Trainer) → Ausschnitt `frame · 80 .. frame · 80 + 80` jeder Zeile (private `crop`) → `png`. Ein Bild, das nicht 160 × 80 ist, wird ganz geliefert (Review Focus); ein Member, das kein NCGR ist, ist ein Fehler wie bei `icons.rs`.
- `battle_sprites.yaml`:

```yaml
# Battle sprites (pokemonBattleSprites, otherPokemonBattleSprites, trainerGraphics): the decryption
# direction per archive and family, measured on Platinum DE, HeartGold DE and HeartGold US
# (docs/research/2026-10-30-phase-3d/gutachten-c-sprites-krypto.py), and the form table of
# otherPokemonBattleSprites (3d.3 Task 3, by eye). DP is unverified: no project at hand.
direction:
  pt:   { pokemon: forward, other: forward, trainer: forward }
  hgss: { pokemon: forward, other: forward, trainer: backward }
  dp:   { unverified: true, pokemon: forward, other: forward, trainer: forward }
forms: []
```

- [ ] **Step 1: Scheiternde Tests** (`tests/battle_sprites.rs`, synthetisches Projekt; die Fixture-Bilder sind 160 × 80, 4 bpp, Modus 1, mit `encrypt(…, Forward, seed)` verschlüsselt, Klartext-Startwort 0, Palette 16 Farben mit Index 1 = `0x001F`; Art 2 hat die Plätze 0 und 2 leer, Art 1 alle vier; Trainerklasse 1 mit 16 Paletten, Palette 0 Index 1 = `0x03E0`):
  - `directions_follow_the_metadata` (`pt` alles `Forward`; `hgss` Trainer `Backward`; `dp` → `None`).
  - `pokemon_member_maps_slots` (`(1, Back, true) == 6`, `(1, Front, false) == 9`).
  - `front_sprite_is_the_left_half_decrypted` (Art 1, `Front`, `frame 0` → PNG 80 × 80, deckende Pixel genau die, die die Fixture links deckend hat; `frame 1` die rechte Hälfte).
  - `an_empty_female_slot_falls_back_to_the_male_image` (Art 2 `female: true` liefert dieselben Bytes wie `female: false`; `sprite_slots(2) == Some(SpriteSlots { female: false, male: true })`).
  - `species_out_of_range_is_none` (Art 600 → `None`; Klasse 7 → `None`).
  - `trainer_sprite_uses_palette_zero_of_member_5k_plus_1`.
  - `a_member_of_another_size_or_mode_is_rendered_or_none` (Review Focus: ein 32 × 32-Member im Modus 0 → PNG 32 × 32; ein Member aus 7 Bytes → `Err`, nie Panik).
  - `an_unverified_family_gives_no_sprite` (Review Focus: Fixture mit Familie `Dp`, soweit `synthetic_project` eine Familie wählen lässt, sonst `directions(Family::Dp) == None` und ein Unit-Test, dass `battle_sprite` bei `None` `Ok(None)` liefert).
  - `the_cache_reads_a_member_once` (zweiter Aufruf liefert denselben `Arc`).
  - `tests/project.rs`: Slots 28, Metadaten je Familie, `pack_only` von `pt` und `hgss` um drei kürzer.
  - Gated (`G4_TEST_PROJECT`, nur lesend, druckt eine Zeile `sprites <game_code> pokemon=<n> forms=<n> trainers=<n> components=<x.x> corners=<n> palettes16=<n> ms=<n>`): `vanilla_every_battle_sprite_decodes_with_the_measured_counts`: über alle Member von `pokemonBattleSprites` mit Platz 0–3 und Bytes: `Ncgr::read` gelingt, Modus `Linear`, 160 × 80, 4 bpp; nach `decrypt` zusammenhängende 4er-Flächen im linken 80 × 80 (Algorithmus aus `gutachten-c-proto.rs` `components`, im Test nachgebaut) im Mittel ≤ 3; Ecken (0,0), (79,0), (0,79), (79,79) beider Hälften Index 0 bei ≥ 1 778 Bildern; Bilder gezählt = 1 812; jede NCLR der Plätze 4 und 5 liest sich mit genau 16 Farben je Palette (988 Paletten); Trainer: 105 (Pt) bzw. 129 (HGSS) Bilder `5·k + 4` lesen, Flächen ≤ 3, Paletten-Member mit 16 Paletten; Formen zählt erst Task 3 (hier nur „NCGR-Member in `otherPokemonBattleSprites` = 157 bzw. 161“). Die Erwartung je Spielcode steht als Tabelle im Test (CPUD, IPKD, IPKE).
- [ ] **Step 2: Lauf** `cargo test -p g4-kernel --all-features --test battle_sprites --test project` → FAIL.
- [ ] **Step 3: Umsetzen** wie unter Interfaces; die YAML wird mit `include_str!` eingebettet und einmal je Prozess geparst (Muster `arm9.rs`).
- [ ] **Step 4: Lauf** → PASS; gated gegen die drei Kopien `3d-3-{pt,hgde,hgus}` (`G4_TEST_PROJECT=<Kopie> cargo test -p g4-kernel --all-features --test battle_sprites -- --nocapture`), die Zeilen in den Bericht; `cargo test --workspace --all-features` ohne ROM.
- [ ] **Step 5: Commit** `feat(kernel): battle sprite archives as asset kinds, directions as metadata, Pokémon and trainer sprites as PNG` (mit Trailer).

### Task 3: Formen am Augenschein belegen, Formen-Tabelle

**Files:**
- Modify: `crates/g4-kernel/metadata/battle_sprites.yaml` (`forms`), `crates/g4-kernel/src/data/battle_sprites.rs` (Formen-Auflösung in `battle_sprite`), `crates/g4-kernel/tests/battle_sprites.rs` (Formen-Tests, gated Formenzahl, Dump-Helfer), `crates/g4-kernel/src/test_support.rs` (`otherPokemonBattleSprites` mit den Membern, die die Tabelle für 496 nennt), `docs/formats/ncgr_nclr.md` (Tabelle der Formen)
- Create: `docs/research/2026-10-30-phase-3d/task3-3d3-formen.md` (Messprotokoll: Member-Folge der Magics, Zuordnung, was unsicher blieb; keine Bilder)

**Interfaces:**
- Consumes: Task 2.
- Produces: `forms` in `battle_sprites.yaml` (Form siehe „Entscheidungen“), `form_sprite`, `form_of_personal` mit Inhalt; `battle_sprite` löst `species ≥ 494` über `form_of_personal` und `(species, form ≠ 0)` über `form_sprite` auf, Rückfall Grundform; ein Dump-Helfer als ignorierter, gated Test `dump_sprites_for_inspection` (`#[ignore]`, läuft nur mit `G4_SPRITE_DUMP_DIR`, schreibt je NCGR-Member von `otherPokemonBattleSprites` ein PNG `other-<member>-<palette>.png` mit jeder NCLR, die in den folgenden vier Membern liegt, in den Ordner; nie ins Repo).

- [ ] **Step 1: Inventar** auf der Kopie `3d-3-pt` und `3d-3-hgde`: Kopie von `gutachten-c-sprites-inventar.py` im Scratchpad, die Magic-Folge **aller** Member druckt (`python3 -I <kopie>.py <DSPRE-Ordner>`); die Folge in `task3-3d3-formen.md` (nur Magics und Nummern).
- [ ] **Step 2: Augenschein**: `G4_SPRITE_DUMP_DIR=<scratchpad>/formen-pt G4_TEST_PROJECT=<3d-3-pt> cargo test -p g4-kernel --all-features --test battle_sprites dump_sprites_for_inspection -- --ignored --nocapture`, ebenso HGSS; die Bilder mit dem Read-Werkzeug ansehen; je Bild Art, Form, Seite (Rückseiten sind unten häufig angeschnitten, Gutachten C §3.1) und Geschlecht bestimmen; die Palette ist die, mit der das Bild stimmige Farben zeigt. Pflicht: Ei (494), Deoxys 1–3 (496–498), Burmadame 1–2 (499–500), Giratina 1 (501), Shaymin 1 (502), Rotom 1–5 (503–507); dazu, so weit die Zeit reicht, Unown, Castform, Shellos, Gastrodon, Cherrim, Arceus, Burmy, Pichu (HGSS). Unsicher → `unverified: true`. Zeitgrenze zwei Stunden; was fehlt, steht im Protokoll unter „nicht belegt“.
- [ ] **Step 3: Scheiternde Tests**: `form_of_personal_resolves_496_to_507_and_the_egg`, `a_personal_form_renders_from_the_other_archive` (Fixture mit den Membern aus der Tabelle für 496: `battle_sprite { species: 496 }` liefert ein PNG mit den Fixture-Pixeln), `an_unknown_form_falls_back_to_the_base_species` (Art 1, Form 7 → Bytes wie Form 0), `an_unverified_form_falls_back_too`, `every_form_entry_names_existing_members_on_the_fixture_layout` (Mitglieder der Tabelle sind paarweise disjunkt, `palette ≠ shiny`); gated: `vanilla_every_form_sprite_decodes` (jeder NCGR-Member des Archivs: `Linear`, 160 × 80, Flächen ≤ 3; Zahl 157 bzw. 161; jede in `forms` genannte Member-Nummer existiert und hat das richtige Magic).
- [ ] **Step 4: Lauf** → FAIL → **Step 5: Umsetzen** → **Step 6: Lauf** → PASS, gated auf den drei Kopien.
- [ ] **Step 7: Doku**: Tabelle der Formen in `ncgr_nclr.md` (Art, Form, Personal-Member, Member, Palette, Status) und das Protokoll.
- [ ] **Step 8: Commit** `feat(kernel): form table of the battle sprites, checked by eye on Platinum DE and HeartGold DE` (mit Trailer).

### Task 4: Befehle `get_battle_sprite` und `get_trainer_sprite`: Tauri, Brücke, MCP, Frontend-API

**Files:**
- Modify: `apps/studio/src-tauri/src/commands/data/mod.rs` (`get_battle_sprite_impl`, `get_trainer_sprite_impl`, Tauri-Befehle über `off_main`), `apps/studio/src-tauri/src/commands/data/dto.rs` (`DataDocDto::Species.sprite_slots: Option<SpriteSlotsDto>`, `SpriteSlotsDto { female: bool, male: bool }`), `apps/studio/src-tauri/src/commands/data/tests.rs`, `apps/studio/src-tauri/src/state.rs` (`Session.battle_sprites: BattleSpriteCache`), `apps/studio/src-tauri/src/lib.rs` (`generate_handler!`), `apps/studio/src-tauri/src/commands/dispatch.rs` (`COMMANDS`, `run!(bytes, …)`-Arme, beide Fixtures), `apps/studio/src-tauri/src/mcp/access.rs` (`Kind::Read`), `apps/studio/src-tauri/src/mcp/symbolic.rs` (`arg("get_battle_sprite", "/species", SPECIES)`, `arg("get_trainer_sprite", "/class", TRAINER_CLASS)`), `apps/studio/src/api.data.ts` (`getBattleSprite`, `getTrainerSprite`, `BattleSpriteRequest`, `SpriteSlotsDto` im Species-DTO), `apps/studio/src/api.data.test.ts`, `apps/studio/src/data/icons.ts` (+ test: `battleSpriteKey`, `battleSprite`, `trainerSpriteKey`, `trainerSprite`), `docs/mcp.md` (Abschnitt „ID oder Nummer“: beide Befehle; Hinweis unter „Expertenmodus“, dass `g4_call` PNG-Bytes als Base64 liefert wie `get_mon_icon`)

**Interfaces:**
- Consumes: Task 2 und 3 (`battle_sprites::{SpriteRequest, Side, BattleSpriteCache, sprite_slots}`).
- Produces:

```rust
// commands/data/mod.rs
/// PNG of the requested battle sprite; empty when the species has none (see `battle_sprites::battle_sprite`).
pub fn get_battle_sprite_impl(state: &AppState, species: u16, form: Option<u8>, side: Side, female: Option<bool>,
                              shiny: Option<bool>, frame: Option<u8>) -> Result<Vec<u8>, CmdError>;   // frame > 1 → CmdError "frame must be 0 or 1"
pub fn get_trainer_sprite_impl(state: &AppState, class: u8) -> Result<Vec<u8>, CmdError>;
```

```ts
// api.data.ts
export type BattleSpriteRequest = { species: number; form?: number; side: "front" | "back"; female?: boolean; shiny?: boolean; frame?: 0 | 1 };
export const getBattleSprite: (req: BattleSpriteRequest) => Promise<ArrayBuffer | null>;   // Argumente einzeln an invoke: { species, form, side, female, shiny, frame }
export const getTrainerSprite: (cls: number) => Promise<ArrayBuffer | null>;              // invoke("get_trainer_sprite", { class: cls })
export interface SpriteSlotsDto { female: boolean; male: boolean }
// data/icons.ts
export const battleSpriteKey: (req: BattleSpriteRequest) => string;  // `battle:${species}:${form ?? 0}:${side}:${female ? 1 : 0}:${shiny ? 1 : 0}:${frame ?? 0}`
export const battleSprite: (req: BattleSpriteRequest) => Promise<string | null>;
export const trainerSpriteKey: (cls: number) => string;              // `trainer:${cls}`
export const trainerSprite: (cls: number) => Promise<string | null>;
```

- Dispatch-Argumente: `get_battle_sprite { species: u16, form: Option<u8>, side: Side, female: Option<bool>, shiny: Option<bool>, frame: Option<u8> }` (Fixture `json!({ "species": 1, "side": "front" })`), `get_trainer_sprite { class: u8 }` (Fixture `json!({ "class": 1 })`); Antwort `bytes` wie `get_mon_icon`, leerer Body = kein Bild.

- [ ] **Step 1: Scheiternde Tests**: `tests.rs` `get_battle_sprite_returns_png_and_empty_for_a_missing_slot` (Fixture: Art 1 vorn → PNG-Signatur; Art 600 → leer), `an_unreadable_member_answers_empty_not_an_error` (Review Focus: Art 2 `female` → Bytes des ♂-Platzes; `frame: 2` → `CmdError`), `species_doc_carries_sprite_slots` (Art 2 → `{ female: false, male: true }`), `get_trainer_sprite_returns_png`; Dispatch `every_command_accepts_the_keys_api_ts_sends` und `commands_match_the_tauri_handler` mit beiden Befehlen; `symbolic.rs`: `resolve_args("get_battle_sprite", json!({ "species": "SPECIES_PIKACHU", "side": "front" }))` → 25; `api.data.test.ts`: Argumentnamen beider Befehle; `icons.test.ts`: Schlüssel und einmaliges Laden je Schlüssel. Gated (`vanilla_copy()`, Studio-Lib): `battle_sprites_over_the_command_path_on_the_vanilla_copy` (Art 25 vorn, Art 25 hinten schillernd, Klasse 1: je PNG 80 × 80; druckt `sprite <game_code> first_ms=<n> cached_ms=<n> trainer_ms=<n>`; unter `G4_TIMING_STRICT=1` jede erste Anfrage ≤ 50 ms im Debug-Build).
- [ ] **Step 2: Lauf** `cargo test -p studio --all-features --lib data:: dispatch:: symbolic::` und `pnpm test` → FAIL.
- [ ] **Step 3: Umsetzen** → **Step 4: Lauf** → PASS, gated einmal auf Platin DE und HeartGold DE.
- [ ] **Step 5: Commit** `feat(studio): get_battle_sprite and get_trainer_sprite in Tauri, bridge and MCP with a per-session cache` (mit Trailer).

### Task 5: Szenen-Tokens und die Primitives `DSPokedexProfile` und `DSBattlePreview`

**Files:**
- Modify: `design-system/tokens/Szene/Wert.tokens.json` (Gruppe `ds`: `status/{flaeche,rahmen,text}`, `kp/{spur,gruen,gelb,rot}`, `kampf/{grund,boden,boden-rand}` mit den Werten aus „Entscheidungen“), `design-system/scripts/check_tokens.py` (Schleife über `textbox`, `schild`, `status`), generierte `apps/studio/src/ui/tokens/{tokens.css,tokens.ts,dockview-theme.css}` per `pnpm tokens`, `apps/studio/src/ui/primitives/index.ts`, `apps/studio/src/ui/primitives/Sprite.tsx` (+ `.module.css`: Größe `80`), `apps/studio/src/ui/gallery/pages/ScreenPage.tsx` (Block „Pokédex-Profil“ und „Kampfvorschau“ mit Platzhaltern, ohne Spielbilder)
- Create: `apps/studio/src/ui/primitives/DSPokedexProfile.tsx`, `DSPokedexProfile.module.css`, `DSPokedexProfile.test.tsx`, `DSBattlePreview.tsx`, `DSBattlePreview.module.css`, `DSBattlePreview.test.tsx`

**Interfaces:**
- Consumes: `DSScreen` (`variant="empty"`, `background`, `scale`, `decorative`), `TypeBadge`, `Sprite`, Tokens `--g4-szene-ds-*`, `--g4-schrift-familie-spiel`.
- Produces:

```ts
export type DSPokedexProfileProps = {
  number: number;                       // „Nr. 397“, dreistellig
  name: string;
  types: readonly [number] | readonly [number, number];
  spriteUrl: string | null;             // 80 × 80 PNG als Blob-URL; null: Platzhalter (Anfangsbuchstabe, `Sprite` 80)
  category?: string; height?: string; weight?: string; text?: string;   // fehlend: „folgt“ bzw. „Pokédex-Text folgt“
  scale?: number;
  decorative?: boolean;
};
export function DSPokedexProfile(props: DSPokedexProfileProps): JSX.Element;   // role="group" aria-label „Pokédex-Profil Staravia“ (über DSScreen), data-ds-profile
export type BattleSide = { name: string; level: number; spriteUrl: string | null; hp?: number /* 0..1, Standard 1 */ };
// level <= 0: der Statuskasten zeigt nur den Namen, ohne „Lv.“ und ohne KP-Balken (ein Trainer vor dem Kampf)
export type DSBattlePreviewProps = {
  foe: BattleSide;                      // Gegner: Bild oben rechts, Statuskasten oben links
  own: BattleSide | null;               // eigenes Pokémon unten links, Statuskasten unten rechts; null: Boden ohne Bild und Kasten
  scale?: number;
  decorative?: boolean;
};
export function DSBattlePreview(props: DSBattlePreviewProps): JSX.Element;    // aria-label „Kampfvorschau: Staravia Lv. 12 gegen …“, data-ds-battle; KP-Balken role="meter" mit aria-valuenow 0..100, data-hp="gruen|gelb|rot"
```

- Maße in DS-Pixeln (× `--ds-scale`): Profil: Bild bei x 8, y 16; Textblock ab x 100, y 12, Zeilenhöhe 16; Textbox wie `DSScreen` (x 4, Höhe 54, unten 4) mit `szene/ds/textbox/*`. Kampf: Grund `kampf/grund`; Boden-Ellipsen 96 × 24 bei (152, 72) und (8, 160) in `kampf/boden` mit Rand `kampf/boden-rand`; Bilder 80 × 80 mit Unterkante auf der Ellipsenmitte; Statuskästen 104 × 28 bei (8, 16) und (144, 120) in `status/*`, Zeile 1 Name, Zeile 2 „Lv. 12“ und KP-Balken 48 × 4 in `kp/spur` mit Füllung `kp/gruen|gelb|rot`. Schwellen: `hp > 0.5` grün, `> 0.2` gelb, sonst rot.

- [ ] **Step 1: Tokens**: Werte eintragen, `python3 design-system/scripts/check_tokens.py` (Raster und die neue Kontrastzeile), `cd apps/studio && pnpm tokens`, `bash scripts/check-tokens.sh` grün. Die `.fig` bleibt unberührt (OpenPencil nicht erreichbar, Nachtrag in Task 13).
- [ ] **Step 2: Scheiternde Tests**: `DSPokedexProfile.test.tsx` (`number 397` → Text „Nr. 397“; zwei `TypeBadge`s mit `data-type`; fehlende Kategorie → „folgt“ dreimal und „Pokédex-Text folgt“; `spriteUrl: null` → `[data-sprite="placeholder"]`; `decorative` versteckt die Gruppe), `DSBattlePreview.test.tsx` (zwei Kästen mit Namen und „Lv. 12“; `hp 0.3` → `data-hp="gelb"`, `aria-valuenow 30`; `own: null` → ein Kasten; `aria-label` nennt beide Namen), `Sprite` 80 rendert `width=80`.
- [ ] **Step 3: Lauf** `pnpm test -- DSPokedexProfile DSBattlePreview Sprite` → FAIL → **Step 4: Umsetzen** → **Step 5: Lauf** → PASS; `bash scripts/check-styles.sh`; Galerie-Blöcke; Screenshots der Galerie-Seite „Bildschirm“ in Hell, Dunkel, Kompakt nach `target/agent/gate-<datum>/3d-3/galerie-*.png` (Test-Umgebung auf 1720/1721, `?gallery`).
- [ ] **Step 6: Commit** `feat(ui,tokens): DS Pokédex profile and battle preview over DSScreen with status, HP and ground tokens` (mit Trailer).

### Task 6: Pokédex-Schau mit DS-Profil und Umschaltern

**Files:**
- Modify: `apps/studio/src/data/pokedex/PokedexView.tsx` (+ test), `apps/studio/src/data/pokedex/Pokedex.module.css`, `apps/studio/src/data/pokedex/pokedexTesting.tsx` (Fixture-Doc mit `sprite_slots`)

**Interfaces:**
- Consumes: Task 4 (`battleSprite`, `battleSpriteKey`, `useIcon`, `DataDocDto.Species.sprite_slots`), Task 5 (`DSPokedexProfile`), `Segmented`, `Switch`, `TypeBadge`, `useDataArea`, `useSpecies`.
- Produces: Abschnitt „Spielerblick“ (`<section aria-label="Spielerblick" data-ds-profile-section>`) über „Basiswerte“: `DSPokedexProfile` (Skalierung aus der Breite, `decorative` weil die Schau die Art schon nennt), darunter `Segmented` „Seite“ (`Vorderseite | Rückseite`, `data-side`), `Switch` „Weiblich“ (`data-female`; gesperrt mit Hilfetext „Kein eigenes Bild für ♀“, wenn `sprite_slots.female` falsch; umgekehrt gesperrt „Nur ♀“, wenn `male` falsch, dann ist ♀ an) und `Switch` „Schillernd“ (`data-shiny`); das Bild über `useIcon(battleSpriteKey(req), () => battleSprite(req), true)`; ohne Projektbild (`null`) der Platzhalter. Zustand der Umschalter lebt in der Komponente (je Sitzung), nicht in `shellState.ts` (3d.2).

- [ ] **Step 1: Scheiternde Tests** (`PokedexView.test.tsx`): `the_profile_shows_number_name_and_types`, `switching_side_asks_for_the_back_sprite` (Mock `getBattleSprite` mit `side: "back"`), `female_is_locked_without_an_own_image` (Fixture `sprite_slots.female: false` → Schalter `aria-disabled` und der Text), `shiny_asks_with_shiny_true`, `without_a_sprite_the_profile_shows_the_placeholder` (Review Focus: Mock liefert `null`).
- [ ] **Step 2: Lauf** → FAIL → **Step 3: Umsetzen** → **Step 4: Lauf** → PASS; Browser auf `3d-3-web` (Platin DE): Staravia, Umschalter, ⌘1 wie bisher; Screenshots `blick-pokedex-profil-{vorn,hinten,schillernd}-{hell,dunkel,kompakt}.png`.
- [ ] **Step 5: Commit** `feat(studio): Pokédex view shows the DS profile with front, back, female and shiny` (mit Trailer).

### Task 7: Trainer-Kopf und -Schau mit Klassenbild, Kampfvorschau

**Files:**
- Modify: `apps/studio/src/data/trainer/TrainerView.tsx` (+ test), `apps/studio/src/data/trainer/TrainerHand.tsx` (+ test: Klassenbild im Kopf), `apps/studio/src/data/trainer/Trainer.module.css`, `apps/studio/src/data/trainer/trainerTesting.tsx`

**Interfaces:**
- Consumes: Task 4 (`trainerSprite`, `trainerSpriteKey`, `battleSprite`), Task 5 (`DSBattlePreview`, `Sprite` 80), `Segmented`, `useTrainer`, `useDataArea` (`names.classes`, `names.species`).
- Produces: `TrainerView`-Kopf: links `Sprite` 80 des Klassenbilds (`trainerSprite(doc.class)`, `decorative`), `data-trainer-sprite`; `TrainerHand`-Kopf: `Sprite` 48 des Klassenbilds neben dem Titel (das 80 × 80-PNG skaliert der Browser mit `image-rendering: pixelated`); Abschnitt „Kampfvorschau“ (`<section aria-label="Kampfvorschau" data-ds-battle-section>`) unter „Team“: `Segmented` „Gegnerseite“ (`Trainer | Erstes Pokémon`, `data-foe`; ohne Team nur „Trainer“) und `DSBattlePreview` mit `foe = { name: Klassenname und Trainername, level: 0 → Kasten ohne „Lv.“ beim Trainer; beim Pokémon Artname und Level der Party[0] }`, `own: null`; `level 0` lässt die Level-Zeile und den KP-Balken weg (Prop-Regel in Task 5: `level <= 0` → nur der Name).

- [ ] **Step 1: Scheiternde Tests**: `TrainerView.test.tsx` `the_head_shows_the_class_sprite` (Mock `getTrainerSprite` → `[data-trainer-sprite] img`), `the_battle_preview_switches_between_trainer_and_first_pokemon` (Standard Trainer: Kasten ohne „Lv.“; „Erstes Pokémon“ → Artname und „Lv. 12“, `getBattleSprite` mit `side: "front"`), `without_a_team_only_the_trainer_is_offered`; `TrainerHand.test.tsx` `the_head_shows_the_class_sprite_small`.
- [ ] **Step 2: Lauf** → FAIL → **Step 3: Umsetzen** → **Step 4: Lauf** → PASS; Browser: Trainer 1 auf `3d-3-web`, Screenshots `blick-trainer-{kopf,kampfvorschau-trainer,kampfvorschau-pokemon}-{hell,dunkel,kompakt}.png`.
- [ ] **Step 5: Commit** `feat(studio): trainer head and view with the class sprite and a battle preview` (mit Trailer).

---

## Übergang

### Task 8: Main mergen (3d.1 muss drin sein)

**Files:**
- Modify (nur Konfliktlösung): Anhängepunkte `dispatch.rs`, `lib.rs`, `access.rs`, `catalog.rs`, `tools.rs`, `symbolic.rs`, `crates/g4-kernel/src/lib.rs`, `api.ts`, `api.data.ts`, `data/icons.ts`, `docs/mcp.md`, `design-system/README.md`, `ROADMAP.md`

**Interfaces:**
- Consumes: main mit gemergtem 3d.1.
- Produces: der Worktree auf dem Stand von main plus Teil A; `data/ids/flags.json` wird vom Kernel gekannt.

- [ ] **Step 1: Prüfen** im Worktree: `git fetch` ist nicht nötig (lokales Repo), `git log --oneline main -5` und `git grep -n "flags.json" main -- crates/g4-kernel/src/textproject` → mindestens ein Treffer (Register der eigenen Flag-Namen aus 3d.1). **Kein Treffer: hier stoppen, Teil A ist fertig und committet, Bericht an den Nutzer „3d.1 ist nicht auf main, Teil B wartet“; nichts weiter tun.**
- [ ] **Step 2: Mergen** `git merge main` (im Worktree, nie umgekehrt); Konflikte nur an Anhängepunkten erwartet: beide Seiten behalten, Reihenfolge main zuerst, dann 3d.3; `.fig` von main behalten; `schema.snap` von main behalten (Teil B ändert es in Task 11 neu).
- [ ] **Step 3: Volle Prüfung** `cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features`, `cd apps/studio && pnpm test && pnpm build`, die Check-Skripte; die Fixtures in `dispatch.rs` zählen alle Befehle beider Seiten.
- [ ] **Step 4: Commit** der Merge (Git schreibt die Botschaft `Merge branch 'main' into phase-3d-3-sprites-ds-eier`; der Trailer wird angehängt).

---

## Teil B: Ei-Attacken

### Task 9: Codec `egg_moves`, Baustein `overlay_tables.yaml`, Quelle und Store-Art

**Files:**
- Create: `crates/g4-formats/src/data/egg_moves.rs`, `crates/g4-kernel/metadata/overlay_tables.yaml`, `crates/g4-kernel/src/data/overlay_tables.rs`, `crates/g4-kernel/src/data/egg_moves.rs`, `crates/g4-kernel/tests/egg_moves.rs`, `docs/formats/egg_moves.md`
- Modify: `crates/g4-formats/src/data/mod.rs`, `crates/g4-kernel/src/data/mod.rs`, `crates/g4-kernel/metadata/archives.yaml` (`hgss.egg_moves: { packed: files/a/2/2/9, unpacked: eggMoves }` statt des `pack_only`-Eintrags; `pt.egg_moves: { packed: files/zcustom/egg_moves.narc, unpacked: eggMoves }` mit Kommentar „nur mit DSPRE-Patch vorhanden“, `dp` ohne), `crates/g4-kernel/src/project.rs` (`AssetKind::EggMoves` Slot 28, `ALL` 29; `FamilyArchives.egg_moves: Option<ArchiveEntry>` wie `dynamic_headers`; `member_location(EggMoves, 0)` nach „Entscheidungen“; `member_count(EggMoves)` 1 oder 0; `ensure_unpacked(EggMoves)` auf Platin ohne den Patch-NARC tut nichts, kein Fehler; der Build packt `eggMoves` nur, wenn der Ordner da ist), `crates/g4-kernel/src/store.rs` (`Loaded::EggMoves(EggMoves)`, `is_data` + `EggMoves`; geladen wird über `EggMoves::read`, danach setzt der Store `budget = Some(len)`, wenn die Location ein `Slice` ist (`EggMoves::set_budget`, oder `read_fixed`, wenn der Lade-Arm die Location kennt); `to_bytes` wandelt `write()`s `Err(words)` in `KernelError::Refused(budget_full(words_total))`; `Store::egg_moves()`, `Store::edit_egg_moves(label, edit)`, `Tx::egg_moves()`/`edit_egg_moves`), `crates/g4-kernel/src/test_support.rs` (Fixture: `unpacked/eggMoves/0000` mit drei Arten; eine zweite Fixture-Variante mit `arm9_overlays/ov005.bin` und `overlays.yaml`, Tabelle an `0x2923E` in einer genau 3 820 Byte langen vollen Tabelle; der Fixture-Spielcode bleibt CPUE, darum bekommt `overlay_tables.yaml` für Tests eine Einspeisung `overlay_tables::with_table_for_tests` oder die Fixture nutzt `CPUD` als Spielcode, wenn `synthetic_project` den Code wählen lässt, Entscheidung beim Umsetzen, im Commit nennen), `crates/g4-kernel/tests/project.rs` (Slots 29, `pack_only` von `hgss` −1)

**Interfaces:**
- Produces (g4-formats):

```rust
// data/egg_moves.rs
pub const SPECIES_BASE: u16 = 20000;
pub const TERMINATOR: u16 = 0xFFFF;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EggMoveEntry { pub species: u16, pub moves: Vec<u16> }
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EggMoves { pub entries: Vec<EggMoveEntry>, pub terminated: bool, pub tail: Vec<u8>, pub budget: Option<usize> }
impl EggMoves {
    /// Words up to the terminator (`tail` after it) or the end; a word below SPECIES_BASE before the first mark is BadSignature.
    pub fn read(bytes: &[u8]) -> Result<EggMoves, FormatError>;          // budget None
    pub fn read_fixed(bytes: &[u8]) -> Result<EggMoves, FormatError>;    // budget Some(bytes.len()); signature_ok must hold, else BadSignature
    pub fn set_budget(&mut self, bytes: Option<usize>);
    /// Marks and moves, terminator, then: without budget the tail as read while the word count is unchanged, else zero padding to
    /// four bytes; with a budget 0xFFFF words up to it. Err(words) when marks + moves + 1 exceed the budget.
    pub fn write(&self) -> Result<Vec<u8>, usize>;
    pub fn words(&self) -> usize;                                         // marks + moves + 1
    pub fn budget_words(&self) -> Option<usize>;                          // budget / 2
    pub fn moves_of(&self, species: u16) -> Option<&[u16]>;
    /// Replaces the list; an empty list removes the entry; a new entry goes in ascending species order.
    pub fn set_moves(&mut self, species: u16, moves: Vec<u16>);
    /// First word SPECIES_BASE + 1, last word TERMINATOR, every word between TERMINATOR or below SPECIES_BASE + 600.
    pub fn signature_ok(bytes: &[u8]) -> bool;
}
```

- Produces (g4-kernel):

```rust
// data/overlay_tables.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OverlayTable { pub overlay: u16, pub offset: u64, pub budget: usize }
/// `table` is the top-level key of overlay_tables.yaml ("egg_moves"); None for an unknown or unverified game code.
pub fn overlay_table(game_code: &str, table: &str) -> Option<OverlayTable>;
pub fn unverified_reason(game_code: &str, table_label: &str, overlay: u16) -> String; // „Ei-Attacken: Lage in Overlay 5 für CPUE nicht geprüft“
// data/egg_moves.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)] #[serde(rename_all = "snake_case")]
pub enum EggMovesSource { Narc, Patch, Overlay }
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EggMovesPlace { pub source: EggMovesSource, pub location: Location, pub budget: Option<usize> }
/// Where the table lives: HGSS member; Platinum the patch NARC when `files/zcustom/egg_moves.narc` exists, else the overlay slice
/// when the signature holds. `Ok(Err(reason))`: locked (no table for the game code, signature missing).
pub fn egg_moves_place(project: &Project) -> Result<Result<EggMovesPlace, String>, KernelError>;
pub fn signature_missing(offset: u64) -> String;     // „Ei-Attacken: Overlay 5 trägt an 0x2923E keine Tabelle (Signatur fehlt)“
pub fn budget_full(total_words: usize) -> String;    // „Ei-Attacken-Tabelle voll: 1 910 von 1 910 Einträgen“ (Tausenderpunkt wie die anderen Kernel-Sätze: Leerzeichen)
// store.rs
impl Store {
    pub fn egg_moves(&mut self) -> Result<&EggMoves, KernelError>;                      // ReadOnly with the reason when locked
    pub fn edit_egg_moves(&mut self, label: &str, edit: impl FnOnce(&mut EggMoves)) -> Result<Option<ChangeId>, KernelError>;
    // Err(KernelError::Refused(budget_full(..))) when write() exceeds the budget; nothing recorded
}
```

- `overlay_tables.yaml`:

```yaml
# Tables inside the decompressed overlays (arm9_overlays/ovNNN.bin), keyed by game code: file offset in
# the overlay and the budget in bytes the table may fill (the game reads up to the first 0xFFFF). The kernel
# reads a table only for a code without `unverified: true` and only when its signature holds.
# egg_moves: u16 words `20000 + species`, the moves, 0xFFFF; measured on Platinum DE
# (docs/research/2026-10-30-phase-3d/gutachten-b-eier.py): 0x2923E..0x2A12A, 198 species, 1 711 moves, exactly full.
# uxie 0.9.7 names 0x29222 for an unnamed other language (binary.rs:331); not assigned to a code here.
egg_moves:
  overlay: 5
  CPUD: { offset: 0x2923E, budget: 3820 }
  CPUE: { unverified: true }
  CPUS: { unverified: true }
  CPUI: { unverified: true }
  CPUF: { unverified: true }
  CPUJ: { unverified: true }
```

- [ ] **Step 1: Scheiternde Tests**:
  - g4-formats (`data/egg_moves.rs` Test-Modul): `read_parses_marks_moves_and_terminator` (`[20001, 5, 7, 20003, 9, 0xFFFF]` → zwei Einträge), `an_empty_entry_is_kept` (`[20001, 20002, 3, 0xFFFF]`), `write_round_trips_with_tail` (proptest: Einträge, Tail; `read(write(x)) == x`), `set_moves_removes_on_empty_and_inserts_in_order`, `write_with_budget_pads_with_terminators_and_keeps_the_signature`, `write_refuses_one_word_over_the_budget` (Review Focus: Budget 12 Bytes, 6 Wörter + 1 → `Err(7)`), `signature_ok_needs_20001_first_and_ffff_last`, `read_fixed_rejects_a_bad_signature`.
  - g4-kernel `tests/egg_moves.rs` (Fixtures): `hgss_fixture_reads_the_member_as_narc_source`, `platinum_overlay_slice_is_used_when_the_signature_holds` (`source == Overlay`, `budget == Some(3820)`), `with_the_patch_narc_the_overlay_is_left_alone` (Review Focus: beide da → `source == Patch`, `location` ist das Member; nach `edit_egg_moves` + `save` ist `ov005.bin` byte-gleich), `a_wrong_offset_fails_the_signature_and_locks_the_asset` (Review Focus: Overlay mit verschobener Tabelle → `Ok(Err(signature_missing(0x2923E)))`, `Store::egg_moves()` → `ReadOnly` mit dem Satz), `an_unverified_code_is_locked_with_the_reason`, `edit_within_the_budget_records_one_step_and_undo_restores_the_bytes` (Tausch: ein Schritt, `save` schreibt genau 3 820 Byte in den Slice, ⌘Z stellt die Bytes her), `edit_over_the_budget_is_refused_and_records_nothing`, `tests/project.rs` Slots 29.
  - Gated (`G4_TEST_PROJECT`, lesend): `vanilla_egg_moves_read_with_the_measured_counts` (HGSS: `source Narc`, 198 Arten, 1 847 Attacken, höchstens 14 je Art, `write()` byte-gleich zum Member (4 092 Byte); Platin: `source Overlay`, 198 Arten, 1 711 Attacken, `words() == 1910 == budget_words()`, `write()` byte-gleich zum Slice; druckt `egg_moves <game_code> source=<s> species=<n> moves=<n> words=<n> ms=<n>`); `vanilla_egg_moves_are_in_ascending_species_order` (druckt, ob die Marken aufsteigen; schlägt nicht fehl, das Ergebnis entscheidet Task 11 Step 1).
- [ ] **Step 2: Lauf** → FAIL → **Step 3: Umsetzen** → **Step 4: Lauf** → PASS, gated auf den drei Kopien; `cargo test --workspace --all-features` (der Build packt `eggMoves` weiter byte-gleich: `pack_only_entries_repack_byte_identical` zählt eins weniger, der Rebuild-Test bleibt grün).
- [ ] **Step 5: Doku** `docs/formats/egg_moves.md`: Format, Orte je Spiel, Overlay-Lage, Signatur, Budget, Patch-Erkennung, Messung als Quelle, uxie als Vergleich; `docs/formats/` bekommt in der Übersicht (falls es eine gibt) die Zeile.
- [ ] **Step 6: Commit** `feat(formats,kernel): egg move codec, overlay table metadata, egg moves from NARC, patch NARC or overlay 5 with signature and budget` (mit Trailer).

### Task 10: Befehle: `egg_moves` im Spezies-Dokument, `DataPatch::EggMoves`, `g4k check-data egg_moves`

**Files:**
- Modify: `apps/studio/src-tauri/src/commands/data/dto.rs` (`DataDocDto::Species.egg_moves: Option<EggMovesDto>`, `EggMovesDto { moves: Vec<u16>, source: EggMovesSource, budget: Option<BudgetDto>, locked: Option<String> }`, `BudgetDto { used: u32, total: u32 }`, `DataPatch::EggMoves { moves: Vec<u16> }`), `apps/studio/src-tauri/src/commands/data/edit.rs` (Arm mit Label), `apps/studio/src-tauri/src/commands/data/labels.rs` (Feldname „Ei-Attacken“), `apps/studio/src-tauri/src/commands/data/mod.rs` (`data_get` liest `Store::egg_moves()`; gesperrt → `locked` statt Fehler), `apps/studio/src-tauri/src/commands/data/tests.rs`, `apps/studio/src-tauri/src/commands/dispatch.rs` (Fixture-Patch `{ "EggMoves": { "moves": [1, 2] } }` in der zweiten Fixture), `apps/studio/src/api.data.ts` (`EggMovesDto`, `BudgetDto`, `DataPatch` + `EggMoves`, `fieldLabels.species.egg_moves`), `apps/studio/src/api.data.test.ts`, `crates/g4-kernel/src/bin/g4k.rs` (`DataKind::EggMoves`: Name `egg_moves`, `asset_kind` `AssetKind::EggMoves`, Felder `count`, `move0` … `move15`, `read` über `EggMoves::read(member 0).moves_of(species)`; `MEMBER` ist die Artnummer, der Member ist immer 0), `crates/g4-kernel/tests/cli.rs`, `docs/mcp.md` (Zeile `data_edit`: Teil `EggMoves`; „ID oder Nummer“: `patch.EggMoves.moves` nur Nummern)

**Interfaces:**
- Consumes: Task 9.
- Produces: `data_get species` mit `egg_moves` (`None`, wenn das Spiel keine Quelle hat: DP); `data_edit species <i> { EggMoves: { moves } }` → `Store::edit_egg_moves` mit `set_moves(i, moves)`, Label „<Name>: Ei-Attacken <alt> → <neu>“ bzw. „<Name>: Ei-Attacken getauscht“; Budget-Ablehnung als `CmdError` mit dem Kernel-Satz (Code `error`), gesperrt als `read_only` mit dem Grund; `g4k check-data <rom> --expect egg_moves:1:count=7 --expect egg_moves:1:move0=73`.

- [ ] **Step 1: Scheiternde Tests**: `tests.rs` `species_doc_carries_egg_moves_with_source_and_budget` (HGSS-Fixture `Narc`, `budget None`; Overlay-Fixture `Overlay`, `budget { used: 1910, total: 1910 }`), `egg_moves_edit_records_a_speaking_label_and_undoes` (7 → 8 Attacken bei HGSS-Fixture; Label; `undo_impl` stellt `moves` her), `platinum_add_without_remove_is_refused_swap_and_trade_pass` (Review Focus: volle Tabelle; Tausch → Schritt; eine Art −1 und eine andere +1 in zwei Schritten → beide gehen; +1 ohne −1 → Fehler mit „Ei-Attacken-Tabelle voll: 1 910 von 1 910 Einträgen“, `history` unverändert), `egg_moves_edit_on_a_locked_table_is_refused_with_the_reason` (Review Focus: `read_only`); `cli.rs` `check_data_reads_egg_moves_of_a_species` (HGSS-Fixture bauen, `--expect egg_moves:1:count=<n>` grün, falsche Zahl → Exit 1 mit „egg_moves 1 count: expected …“); Dispatch-Fixtures; `api.data.test.ts`.
- [ ] **Step 2: Lauf** → FAIL → **Step 3: Umsetzen** → **Step 4: Lauf** → PASS; gated `vanilla_data_commands_on_copy` erweitert um einen Tausch von Ei-Attacken der Art 1 (HGSS und Platin) mit ⌘Z.
- [ ] **Step 5: Commit** `feat(studio,g4k): egg moves in the species document and patch with budget rule, check-data egg_moves` (mit Trailer).

### Task 11: Textprojekt: Feld `egg_moves`, Einheit `egg_moves`, Basisschicht, Round-Trip

**Files:**
- Modify: `crates/g4-kernel/src/textproject/schema/data.rs` (`SpeciesEntity.egg_moves: Option<Vec<SymRef>>`; `export` füllt aus `ctx.egg_moves`; `import` ignoriert das Feld; `sample()` mit zwei Attacken), `crates/g4-kernel/src/textproject/schema/mod.rs` (`ExportCtx.egg_moves: Option<&'a EggMoves>`; alle Konstruktoren), `crates/g4-kernel/src/textproject/export.rs` (Tabelle einmal lesen, `Art::Species` mit Kontext), `crates/g4-kernel/src/textproject/import.rs` (`import_egg_moves`: Tabelle aus allen `Some` in aufsteigender Artnummer, Attacken über `SymRef` → Nummer; schreibt an `egg_moves_place().location`, nur wenn die Bytes abweichen; gesperrte Quelle: Warnung `textproject.egg-moves-locked` mit dem Grund, nichts geschrieben), `crates/g4-kernel/src/textproject/codes.rs` (Code), `crates/g4-kernel/src/textproject/source/mod.rs` und `units.rs` (`Place::EggMoves` für `AssetId { EggMoves, 0 }`: Einheit `key: "egg_moves"`, `sources` alle `data/species/NNNN.json`, `members` die Location-Datei(en), `assets [EggMoves 0]`; `read`: Tabelle aus den JSON; `write`: je Art das Feld in ihrer JSON ersetzen (kanonisch, byte-stabil für unveränderte Dateien) und die berührten Art-Einheiten neu aufzeichnen), `crates/g4-kernel/src/textproject/base_layer.rs` (Maske des Ei-Attacken-Slice in `arm9_overlays/ov005.bin`, wenn `egg_moves_place` ein Overlay ist), `crates/g4-kernel/src/textproject/schema.snap` (per `G4_UPDATE_SNAPSHOT=1`), `crates/g4-kernel/tests/textproject.rs` (Round-Trip und Idempotenz bleiben grün), `crates/g4-kernel/tests/textproject_source.rs`, `docs/formats/text_project.md` (Art `species`: Feld `egg_moves`, Beispiel; Abschnitt „Ei-Attacken“ mit Einheit, Platin-Maske, Code), `docs/formats/egg_moves.md` (Verweis)

**Interfaces:**
- Consumes: Task 9 (`EggMoves`, `egg_moves_place`), `Place::Headers` als Muster, `SymRef`, `sync::record_all`.
- Produces:

```rust
pub struct SpeciesEntity { pub id: String, pub personal: …, pub learnset: …, pub evolution: …,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub egg_moves: Option<Vec<SymRef>> }
pub struct ExportCtx<'a> { pub family, pub symbols, pub base, pub side, pub egg_moves: Option<&'a EggMoves> }
// source/mod.rs
pub const EGG_MOVES_UNIT_KEY: &str = "egg_moves";
// import.rs
/// Err(reason) when the source is locked; Ok(true) when the location was written.
fn import_egg_moves(project: &Project, species: &[(u32, &SpeciesEntity)], symbols: &Symbols) -> Result<Result<bool, String>, KernelError>;
```

- [ ] **Step 1: Reihenfolge prüfen**: Ergebnis von `vanilla_egg_moves_are_in_ascending_species_order` (Task 9) auf den drei Kopien. Steigen die Marken **nicht** auf, bekommt `EggMoves` zusätzlich `order: Vec<u16>` (die gelesene Markenfolge), und der Import schreibt in dieser Reihenfolge, neue Arten ans Ende; die Entscheidung kommt in den Commit.
- [ ] **Step 2: Scheiternde Tests**:
  - `schema/data.rs`: `species_export_carries_egg_moves_from_the_table` (`ctx.egg_moves` mit Art 1 → `Some([MOVE_…])`; ohne Tabelle `None`; Art ohne Eintrag `None`; Eintrag ohne Attacken `Some([])`), `species_import_ignores_egg_moves` (Member unverändert), `schema_fields_match_the_snapshot` nach `G4_UPDATE_SNAPSHOT=1` (die neue Zeile `species.egg_moves?[]` oder wie die Snapshot-Form Listen schreibt).
  - `import.rs`: `import_assembles_the_table_from_the_species_files_in_species_order`, `import_writes_nothing_when_the_table_is_equal` (mtime unverändert), `import_warns_and_skips_on_a_locked_source` (Code `textproject.egg-moves-locked`).
  - `source/units.rs`: `egg_moves_unit_reads_from_the_species_files_and_writes_only_the_field` (ein Store-Edit von `EggMoves` im Textprojekt ändert genau die JSON der betroffenen Art, byte-gleich sonst; danach `sync::scan` meldet keine offene Änderung für die Art-Einheit: `editing_egg_moves_in_a_text_project_leaves_no_pending_external_change`).
  - `base_layer.rs`: `a_changed_egg_move_slice_in_overlay_5_is_no_blob` (Overlay-Fixture: Slice geändert → `raw_diff` ohne `arm9_overlays/ov005.bin`; ein Byte außerhalb des Slice → Blob).
  - Gated: `textproject_round_trip` und `textproject_export_idempotent` bleiben grün (HGSS: Member, Platin: Overlay-Slice), dazu `text_project_egg_move_edit_builds_and_check_data_finds_it` (HGSS-Kopie als Textprojekt: `data_edit`-Weg über den Store, `save`, `build`, `g4k check-data --expect egg_moves:1:count=<n+0>` nach einem Tausch mit `move0=<neu>`; Platin: Tausch, Build, `ov005.bin` im ROM trägt den Slice).
- [ ] **Step 3: Lauf** → FAIL → **Step 4: Umsetzen** → **Step 5: Lauf** → PASS, gated auf Platin DE und HeartGold DE.
- [ ] **Step 6: Doku** `text_project.md` und `egg_moves.md` wie unter Files; Begründung der unveränderten Schema-Version wörtlich aus „Entscheidungen“.
- [ ] **Step 7: Commit** `feat(textproject): egg_moves field of the species entity, one unit over all species files, overlay slice masked in the base layer` (mit Trailer).

### Task 12: Pokédex-Reiter „Ei-Attacken“

**Files:**
- Create: `apps/studio/src/data/pokedex/EggMovesTab.tsx`, `EggMovesTab.test.tsx`
- Modify: `apps/studio/src/data/pokedex/PokedexHand.tsx` (Reiter nach „Lernset“: `{ id: "eggMoves", label: "Ei-Attacken" }`, Punkt für ungespeichert aus dem Asset `EggMoves 0`), `apps/studio/src/api.data.ts` (`SpeciesTab` + `"eggMoves"`), `apps/studio/src/data/pokedex/useSpecies.ts` (hört `dataChanged` für `species` mit beliebigem Index, weil die Tabelle allen gehört), `apps/studio/src/data/pokedex/Pokedex.module.css`

**Interfaces:**
- Consumes: Task 10 (`EggMovesDto`, `DataPatch.EggMoves`), `Table`, `Picker` (Attacken mit `TypeBadge` wie im Lernset-Reiter, aus `pickers.tsx`), `Notice`, `Button`.
- Produces: Reiter (`data-tab="eggMoves"`): Zeilen „Attacke“ mit `Picker`, Löschen je Zeile, Taste „Zeile hinzufügen“; Fuß: HGSS „7 Ei-Attacken“; Platin „7 Ei-Attacken · Tabelle 1 910 von 1 910 Einträgen“ (`data-egg-budget`), bei voller Tabelle ist „Zeile hinzufügen“ gesperrt mit Hilfetext „Tabelle voll: erst eine Ei-Attacke bei einer anderen Art entfernen“; gesperrt (`locked`) → `Notice kind="warning"` mit dem Grund und ohne Felder; `egg_moves == null` → „Dieses Spiel hat keine Ei-Attacken-Tabelle, die das Studio kennt.“; jede Änderung ruft `data_edit species <i> { EggMoves: { moves } }`; eine Ablehnung des Backends steht als `Notice kind="error"`.

- [ ] **Step 1: Scheiternde Tests**: `rows_show_moves_with_names`, `changing_a_move_calls_data_edit_with_the_whole_list`, `add_is_locked_when_the_platinum_budget_is_full` (Fixture `budget { used: 1910, total: 1910 }` → Taste `aria-disabled`, Text), `a_locked_table_shows_the_reason_without_fields`, `a_game_without_a_table_says_so`.
- [ ] **Step 2: Lauf** → FAIL → **Step 3: Umsetzen** → **Step 4: Lauf** → PASS; `bash scripts/check-styles.sh`; Browser auf `3d-3-web` (Platin) und `3d-3-hgweb` (HeartGold): Tausch, Hinzufügen abgelehnt, ⌘Z; Screenshots `blick-pokedex-eiattacken-{pt,hg}-{hell,dunkel,kompakt}.png`.
- [ ] **Step 5: Commit** `feat(studio): egg moves tab in the Pokédex with move picker and Platinum budget` (mit Trailer).

---

## Abschluss

### Task 13: Design-System-Nachtrag, Gate-Datei, Doku, Roadmap

**Files:**
- Create: `docs/phase-3d-3-gate.md`
- Modify: `design-system/README.md` (Komponenten-Tabelle: Zeilen „DS-Bildschirm: Pokédex-Profil“ und „DS-Bildschirm: Kampfvorschau“; Tokens-Abschnitt Gruppe `ds` um `status`, `kp`, `kampf`; „Was die App umsetzt“ + 3d.3; Abschnitt „Nachtrag 3d.3“ nach dem Muster „Nachtrag 4.4“: Quelle (`ui/primitives/DSPokedexProfile*`, `DSBattlePreview*`, `data/pokedex/PokedexView.tsx`, `EggMovesTab.tsx`, `data/trainer/TrainerView.tsx`, `TrainerHand.tsx`), Plan- und Gate-Verweis, Tokens mit Werten, Zustände und Texte aus Task 5–7 und 12, Vergleichsbilder unter `target/agent/gate-<datum>/3d-3/`, Abweichungen: Pokédex-Text, Kategorie, Größe, Gewicht „folgt“, feste Lage statt `pokemonSpriteOffsets`, stilisierter Boden statt Kampfhintergrund, eigene Seite der Kampfvorschau leer; „die `.fig` ist zurzeit nicht erreichbar, die Frames folgen“), `design-system/Audit-Design-System.md` (Zeilen Pokédex-Profil, Kampfvorschau, Ei-Attacken), `docs/agent-gate.md` (Abschnitt „Phase 3d.3“ mit Kopien, Befehlen, `data-*`-Namen, Gate-Datei in der Liste oben), `docs/mcp.md` (Abschnitt „Noch nicht umgesetzt“ oder „Bekannte Grenzen“: Sprites nur über `g4_call`, Ei-Attacken in Platin nur innerhalb des Budgets), `docs/dev-setup.md` (Tabelle der gated Tests: `battle_sprites.rs`, `egg_moves.rs`, der Studio-Test, `check-data egg_moves`), `docs/ROADMAP.md` (Zeile 3d.3 mit Stand; Tabelle „Aus dem Design-System, später“: Ei-Attacken, Kampfsprites, DS-Varianten → „3d.3 umgesetzt“), `docs/formats/ncgr_nclr.md` und `egg_moves.md` (letzter Abgleich mit dem Code)

**Interfaces:**
- Consumes: alles Vorige; die Zeilen `sprites …`, `sprite …`, `egg_moves …` der gated Tests; die Screenshots aus Task 5–7 und 12.

- [ ] **Step 1: Gate-Dokument** `docs/phase-3d-3-gate.md` nach dem Muster von `docs/phase-4-4-gate.md`: Kopf mit Stand, Gate-Text aus Spezifikation §4 (Zeile 3d.3) als Zitat; Legende „Wer“ mit Verweis auf `docs/agent-gate.md`; Namen in der Oberfläche (`[data-ds-profile-section]`, `[data-side]`, `[data-female]`, `[data-shiny]`, `[data-trainer-sprite]`, `[data-ds-battle-section]`, `[data-foe]`, `[data-tab="eggMoves"]`, `[data-egg-budget]`, Sätze „Kein eigenes Bild für ♀“, „Ei-Attacken-Tabelle voll: 1 910 von 1 910 Einträgen“, „Ei-Attacken: Overlay 5 trägt an 0x2923E keine Tabelle (Signatur fehlt)“); Vorbereitung (Kopien `_testkopien/3d-3-{pt,hgde,hgus}` für (1), `3d-3-{kpt,khgde}` für (2), `3d-3-web`, `3d-3-hgweb`, `3d-3-patch` (Platin mit einem synthetischen `files/zcustom/egg_moves.narc`, der den HGSS-Member-Aufbau trägt) für (3), je `.nds` daneben; Test-Umgebung 1720/1721; Last mit `uptime`).
  - **(1) Automatik:** `scripts/gate.sh` grün; einzeln je Projekt im Release-Profil: `vanilla_every_battle_sprite_decodes_with_the_measured_counts`, `vanilla_every_form_sprite_decodes`, `vanilla_egg_moves_read_with_the_measured_counts`, der Studio-Test `battle_sprites_over_the_command_path_on_the_vanilla_copy` mit `G4_TIMING_STRICT=1` (≤ 50 ms im **Debug**-Build: dafür einmal `cargo test` ohne `--release`), gated Textprojekt-Tests aus Task 11.
  - **(2) Szenario kopflos:** HeartGold-Kopie: `studio mcp --project` → `data_get species 1` (`egg_moves.source narc`), `data_edit species 1 { EggMoves … }` (Tausch), `save`, `g4k build`, `g4k check-data <rom> --expect egg_moves:1:count=<n> --expect egg_moves:1:move0=<neu>`; Platin-Kopie: `data_get` (`source overlay`, Budget 1 910/1 910), Tausch → ok, Hinzufügen ohne Entfernen → Fehler mit dem Satz; `3d-3-patch`: `source patch`, `ov005.bin` byte-gleich nach `save`; `g4_call get_battle_sprite` (`--expert`) liefert Base64 eines PNG.
  - **(3) Browser:** Pokédex Staravia: Profil mit Umschaltern (vorn, hinten, ♀ gesperrt oder an, schillernd); Trainer 1: Klassenbild im Kopf, Kampfvorschau Trainer und erstes Pokémon; Reiter „Ei-Attacken“ auf Platin (Budget, Ablehnung) und HeartGold (Hinzufügen geht); ⌘Z; `call.sh get_battle_sprite '{"species":397,"side":"front"}'` mit Zeitmessung (≤ 50 ms Debug über die Brücke); Blick Hell, Dunkel, Kompakt (Bilder aus Task 5–7 und 12).
  - **(4) Hand und Blick beim Nutzer:** die Bilder stimmen mit dem Spiel überein (Hand im Emulator), Formen am Augenschein (Task 3 Protokoll), Platin-Ei-Attacke im Spiel wirksam (Hand: Zucht), `.fig`-Frames folgen.
  - **Ergebnis:** Tabelle Punkt / Wer / Ergebnis / Datum und die Zahlentabelle: Spiel | Pokémon-Bilder | Formen | Trainer | Flächen (Mittel) | Ecken | erstes Sprite (ms) | Ei-Attacken Arten/Attacken | Last.
- [ ] **Step 2: Gate abarbeiten** (Agent: (1) bis (3)); Werte und Befunde eintragen; Befunde beheben (eigene Commits `fix(3d.3): …` mit Regressionstest), bis alles grün ist; `scripts/gate.sh` im Hintergrund, allein, aus dem Worktree mit absolutem `G4_ROMS_DIR` und `tools`-Symlink; `target/agent/gate-*` vor `git worktree remove` sichern.
- [ ] **Step 3: Doku und Roadmap** wie unter Files; Roadmap-Zeile: „**Stand <Datum>:** Gate vom Agenten bestanden: alle Kampfsprites gelesen (Platin 1 812 + 157, HGSS 1 812 + 161, Trainer 105/129), Flächen ≤ 3, Ecken ≥ 1 778; Profil und Kampfvorschau in der App; Ei-Attacken HGSS geändert und per `check-data` gefunden, Platin Tausch innerhalb des Budgets, Hinzufügen abgelehnt, Patch-NARC erkannt; Branch `phase-3d-3-sprites-ds-eier`; [docs/phase-3d-3-gate.md](phase-3d-3-gate.md); beim Nutzer offen: Bilder gegen das Spiel, Zucht in Platin, `.fig`“.
- [ ] **Step 4: Commit** `docs(3d.3): design-system addendum, gate of battle sprites, DS variants and egg moves, roadmap status` (mit Trailer).

---

## Reihenfolge und Parallelität

- Seriell: **1 → 2 → 3 → 4 → 6 ∥ 7** (6 und 7 nach 4 und 5); **5** kann parallel zu 2–4 laufen (nur Frontend und Tokens).
- **8** erst, wenn 3d.1 auf main ist; davor sind Teil A und ein Zwischenbericht fertig.
- **9 → 10 → 11 ∥ 12 → 13**; 11 und 12 berühren getrennte Dateien.
- Die gated Tests laufen je Task auf frischen Kopien `3d-3-{pt,hgde,hgus}`; die Kopien bleiben bis Task 13 und werden danach gelöscht (`Manual`, `_saves` bleiben).

## Abschluss

Nach Task 13: `superpowers:finishing-a-development-branch` (Merge nach `main` erst nach grünem `scripts/gate.sh` und unabhängiger Abnahme über die Test-Umgebung, Muster Phase 4); die Erfahrung aus Phase 4 gilt: Konflikte an Anhängepunkten gesammelt lösen, `.fig` von main behalten.

## Offen für die Umsetzung

- **Formen-Archiv:** Der Aufbau von `otherPokemonBattleSprites` (157/161 NCGR, 96/100 NCLR) ist ungemessen; Task 3 belegt ihn. Reichen zwei Stunden Augenschein nicht für alle Formen, bleiben die übrigen `unverified` (Grundform) und stehen im Protokoll; HGSS kann von Platin abweichen (vier Bilder mehr), dann bekommt die Tabelle `by_family`-Einträge.
- **`serde` in g4-formats:** `Direction` braucht `Deserialize` nur, wenn der Kernel die YAML direkt in den Typ liest; hat `g4-formats` kein `serde`, liest `battle_sprites.rs` Strings (`forward`/`backward`) und wandelt sie.
- **Fixture-Spielcode:** `synthetic_project()` ist CPUE; die Overlay-Fixture von Task 9 braucht einen Code mit Eintrag in `overlay_tables.yaml` (CPUD) oder eine Test-Einspeisung; Task 9 entscheidet und nennt es im Commit.
- **Reihenfolge der Marken:** steigen die Arten in der Vanilla-Tabelle nicht auf, trägt der Codec die gelesene Reihenfolge (`order`) und der Import schreibt sie (Task 11 Step 1).
- **Zeit über die Brücke:** 50 ms im Debug-Build gelten für die erste Anfrage je Bild; liegt HeartGold unter Last darüber, protokolliert das Gate die Zahl, und der Test prüft nur mit `G4_TIMING_STRICT=1`.
- **`g4_call` und Base64:** die Brücke liefert Bytes; über MCP-`g4_call` kommen PNG-Bytes so an, wie `get_mon_icon` heute antwortet (Base64 oder Bytes); Task 4 dokumentiert die vorgefundene Form in `docs/mcp.md`.
- **Symbolische Attacken in `EggMoves.moves`:** löst `resolve_args` Listen auf, kommt `patch.EggMoves.moves` als `MOVE`-Zeiger in `SYMBOLIC_ARGS`; sonst nur Nummern (dokumentiert).
- **Referenzindex:** Ei-Attacken als Quelle für `Target::Move` folgen nach 3d.1 (Datei von 3d.1); „Verwendet in“ einer Attacke zählt sie bis dahin nicht.
- **Platin `check-data`:** Ei-Attacken im Overlay 5 einer gebauten ROM prüft `check-data` nicht (Overlays aus dem ROM lesen, ggf. BLZ); das Gate prüft Platin über `data_get` und die Bytes von `ov005.bin`.
- **Zukan-Daten:** Pokédex-Text, Kategorie, Größe, Gewicht brauchen Bänke und Tabellen, die keine Metadatei kennt; „folgt“ bleibt, bis eine Messung sie belegt (eigener kleiner Posten, nicht 3d).
- **3d.1-Namen:** weichen Register- oder Schema-Namen aus 3d.1 nach dem Merge ab (etwa die Art-Beschreibung als Daten), folgen Teil B und `schema.snap` den gemergten Namen; die Regeln dieses Plans bleiben.
