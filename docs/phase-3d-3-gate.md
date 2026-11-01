# Phase 3d.3: Gate prüfen

Stand: 2026-11-01. Branch `phase-3d-3-sprites-ds-eier` (Worktree `.claude/worktrees/phase-3d-3`), Kopf `3555171a`; Teil A ist auf main (Merge `f74d2f6`), Teil B liegt nur auf dem Branch und ist nicht gemergt. **Beide Teile hat der Agent geprüft; offen sind `scripts/gate.sh` als Ganzes (die Hauptsitzung) sowie Hand und Blick beim Nutzer.** **Teil A** (Kampfsprites, DS-Profil, Kampfvorschau; Plan-Tasks 1 bis 7; Basis main `1d68c2b`) ist umgesetzt, von der Review-Schleife je Task und einem abschließenden Review des ganzen Teils abgenommen (eine Fix-Runde) und im UI-Test am 2026-10-31 auf `a6f2d41` (Bilder auf `6331244` und `a6f2d41`) durchlaufen: (1) gated Tests auf Platin DE, HeartGold DE und HeartGold US, (2) Kopflos über MCP und die Brücke, (3) Browser, ohne offenen Befund außer den Blick-Punkten U2 und U3. **Teil B** (Ei-Attacken; Plan-Tasks 8 bis 12) wartete auf 3d.1 auf main; nach dessen Merge (Task 8: der Branch wurde auf main `79eb730` vorgespult, ohne Konflikt; Teil B setzt darauf auf) ist er umgesetzt, von der Review-Schleife je Task und einem abschließenden Review über `79eb730..a41f8103` abgenommen (eine Fix-Runde, `3555171a`) und am 2026-11-01 auf `3555171a` durchlaufen (Abschnitt „Teil B“).

Phase 3d.3 bringt in Teil A: Kampfsprites nur lesen (`g4-formats` liest den Kachelmodus, rendert Modus 1 linear und entschlüsselt mit `decrypt(words, direction)`; die Richtung je Archiv und Familie steht als Metadatum in `crates/g4-kernel/metadata/battle_sprites.yaml`, dort auch die Formen-Tabelle), Kernel-Modul `data/battle_sprites.rs` (Art × Vorder- und Rückseite × ♀ und ♂ × normal und schillernd, Formen, Trainerklasse → Member `5·k + 4`), die Befehle `get_battle_sprite` und `get_trainer_sprite` (PNG, Cache je Sitzung), das Feld `sprite_slots` in der Spezies-JSON, die DS-Varianten `DSPokedexProfile` und `DSBattlePreview` über `DSScreen` mit den neuen Szenen-Tokens `ds/status`, `ds/kp`, `ds/kampf`, die Pokédex-Schau mit „Spielerblick“ (Vorderseite, Rückseite, Weiblich, Schillernd) und den Trainer-Kopf mit dem Klassenbild und der „Kampfvorschau“. Roadmap: [docs/ROADMAP.md](ROADMAP.md), Abschnitt „Phase 3d“, Punkt „3d.3“. Spezifikation: [2026-10-30-phase-3d-erzaehlen-design.md](superpowers/specs/2026-10-30-phase-3d-erzaehlen-design.md), §4. Plan: [2026-10-30-phase-3d-3-sprites-ds-eier.md](superpowers/plans/2026-10-30-phase-3d-3-sprites-ds-eier.md). Formate: [ncgr_nclr.md](formats/ncgr_nclr.md) (Abschnitt „Kampfsprites“); Werkzeuge: [mcp.md](mcp.md) (`get_battle_sprite`, `get_trainer_sprite`). Formen am Augenschein: [task3-3d3-formen.md](research/2026-10-30-phase-3d/task3-3d3-formen.md).

In Teil B kommen die Ei-Attacken: der Codec `EggMoves` in `g4-formats` (`data/egg_moves.rs`: Marken `20000 + Art`, Attacken, Ende `0xFFFF`), das Kernel-Modul `data/egg_moves.rs` (Ort, Quelle `narc`, `overlay` oder `patch`, die deutschen Sätze) mit der Lage in Overlay 5 aus `crates/g4-kernel/metadata/overlay_tables.yaml` (Offset, Budget, Signatur), der Teil `EggMoves` von `data_edit` (die ganze Liste einer Art, höchstens 16), das Feld `egg_moves` (`{moves, source, budget, locked}`) in der Spezies-JSON von `data_get`, das Feld `egg_moves` jeder Spezies-Datei im Textprojekt (Manifest-Marken `eggMoves` und `eggMovesPatch`, Einheit `egg_moves`) und der Reiter „Ei-Attacken“ der Pokédex-Hand mit Attacken-Picker und Budget in Platin. Formate: [egg_moves.md](formats/egg_moves.md), [text_project.md](formats/text_project.md) (Abschnitt „Ei-Attacken“); Werkzeuge: [mcp.md](mcp.md) (`data_edit`, Teil `EggMoves`), [dev-setup.md](dev-setup.md) (`g4k check-data` mit KIND `egg_moves`).

Gate-Text der Spezifikation (§4, Zeile 3d.3):

> Gated: alle Sprites lesen (Platin 1 812 + 157, HGSS 1 812 + 161 Pokémon- und Formenbilder, 105 / 129 Trainerbilder), Mittel der zusammenhängenden Flächen linear ≤ 3, Ecken durchsichtig wie gemessen, ein Sprite ≤ 50 ms im Debug-Build über die Brücke; Pokédex zeigt das DS-Profil mit Vorderseite, Rückseite, ♀ und schillernd, Trainer-Kopf das Klassenbild, Kampfvorschau Trainer und erstes Team-Pokémon; Ei-Attacke in HeartGold ändern, bauen, `g4k check-data` findet sie; Platin: Tauschen innerhalb des Budgets, Hinzufügen ohne Entfernen abgelehnt mit Grund, mit `zcustom/egg_moves.narc` gilt der NARC; Blick Hell, Dunkel, Kompakt

Der erste Teil des Satzes (bis „Kampfvorschau Trainer und erstes Team-Pokémon“ und „Blick“) ist Teil A; die Ei-Attacken sind Teil B.

Wer was prüft: Die Spalte „Wer“ sagt **Agent** (die Test-Umgebung aus [docs/agent-gate.md](agent-gate.md): Browser-Fenster mit `studio-bridge`, `call.sh`, `mcp.sh`, dazu `g4k`, `studio mcp` kopflos und die gated Tests), **Agent + Blick** (der Agent prüft und beschreibt Ort und Zahlen, der Nutzer bestätigt kurz am Bild) oder **Hand** (der Nutzer). Befehle: [docs/agent-gate.md](agent-gate.md), Abschnitt „Phase 3d.3“.

Namen in der Oberfläche (Rollen und `data-*` für die Prüfung), Teil A:

- **Pokédex-Schau, Abschnitt „Spielerblick“** `[data-ds-profile-section]` (`<section aria-label="Spielerblick">`, über „Basiswerte“): `DSPokedexProfile` (`[data-ds-profile]`, Gruppe „Pokédex-Profil Staravia“) mit „Nr. 397“, dem Namen, den Typ-Plaketten, „Kategorie“, „Größe“ und „Gewicht“ je „folgt“ und der Textbox „Pokédex-Text folgt“; darunter `Segmented` „Seite“ (`[data-side]`: „Vorderseite“, „Rückseite“), Schalter „Weiblich“ (`[data-female]`) und „Schillernd“ (`[data-shiny]`). Gesperrt ist „Weiblich“ mit dem Satz „Kein eigenes Bild für ♀“ (Schalter aus) oder „Nur ♀“ (Schalter an); der Satz hängt über `aria-describedby` am Schalter. Ein Name, der nicht passt, endet mit „…“ und trägt den vollen Namen als `title`.
- **Trainer-Kopf und -Schau:** das Klassenbild `[data-trainer-sprite]` (80 px im Kopf der Schau, 48 px im Kopf der Hand); Abschnitt „Kampfvorschau“ `[data-ds-battle-section]` unter „Team“ mit `Segmented` „Gegnerseite“ (`[data-foe]`: „Trainer“, „Erstes Pokémon“; ohne Team nur „Trainer“) und `DSBattlePreview` (`[data-ds-battle]`, Boden `[data-ds-ground]`, Statuskästen `[data-ds-status=foe|own]`, KP-Balken `role="meter"` mit `data-hp="gruen|gelb|rot"`). „Trainer“ zeigt nur den Namen („Kampfvorschau: Teenager Gunther“), „Erstes Pokémon“ Name und „Lv. 5“ mit Balken; die eigene Seite ist leer (Boden ohne Bild und Kasten).
- **Galerie** (`?gallery`, Seite „Bildschirm“): die Blöcke „Pokédex-Profil“ und „Kampfvorschau“ mit Platzhaltern, ohne Spielbilder.
- **Brücke und MCP:** `get_battle_sprite {species, form?, side, female?, shiny?, frame?}` und `get_trainer_sprite {class, frame?}`; die Brücke liefert das PNG, `g4_call` und `g4_batch` antworten `{"bytes": <Länge>}` (Ruling R7); `data_get species <Art>` trägt `sprite_slots {female, male}`.

Namen in der Oberfläche, Teil B:

- **Pokédex-Hand, Reiter „Ei-Attacken“** `[data-tab="eggMoves"]` (nach „Lernset“, mit der Zahl der Ei-Attacken beim Namen des Reiters, solange die Tabelle lesbar ist; der Punkt „ungespeichert“ gehört der ganzen Tabelle, nicht einer Art). Die Tabelle „Ei-Attacken von BISASAM“ hat die Spalten „Attacke“ (Feld „Attacke in Zeile N“ mit dem Attacken-Picker), „Typ“ (`TypeBadge`) und „Entfernen“ (Taste „<Attacke> entfernen“); Zeilen `[data-egg]` mit `data-move`. „Zeile hinzufügen“ `[data-add-egg]` öffnet den Picker „Ei-Attacke hinzufügen“ (Fuß „Steht hinten in der Liste der Art.“). Die Fußzeile ist in HeartGold „14 Ei-Attacken“ (eine: „1 Ei-Attacke“), in Platin `[data-egg-budget]` „12 Ei-Attacken · Tabelle 1 910 von 1 910 Einträgen“.
- **Sperren von „Zeile hinzufügen“:** `aria-disabled` (die Taste bleibt fokussierbar, `aria-describedby` zeigt auf den Satz daneben) mit „Tabelle voll: erst eine Ei-Attacke bei einer anderen Art entfernen“ (Platin, wenn `Wörter belegt + 1` das Budget übersteigt, für eine Art ohne Attacken `+ 2`: Marke und Attacke) oder „Höchstens 16 Ei-Attacken je Art“ (R14). Tauschen und Entfernen bleiben auf einer vollen Tabelle frei.
- **Zustände ohne Zeilen:** eine gesperrte Tabelle zeigt eine Warnung mit dem Grund des Kernels und „Die Ei-Attacken lassen sich nicht bearbeiten.“ (Platin mit ungeprüfter Fassung oder fehlender Signatur: „Ei-Attacken: Overlay 5 trägt an 0x2923E keine Tabelle (Signatur fehlt)“, Fassung ungeprüft: „Ei-Attacken: Lage in Overlay 5 für CPUE nicht geprüft“); ein Spiel ohne bekannte Tabelle (Diamant, Perl) die Info „Dieses Spiel hat keine Ei-Attacken-Tabelle, die das Studio kennt.“ Eine Ablehnung des Kerns (Budget, Signatur, ungültige Attacke, mehr als 16) steht als Fehler über den Reitern.
- **Kernel-Sätze:** „Ei-Attacken-Tabelle voll: 1 910 von 1 910 Einträgen“ (ohne Verlaufsschritt), „Ei-Attacken: Overlay 5 trägt an 0x2923E keine Tabelle (Signatur fehlt)“, „Ei-Attacken: Overlay 5 lässt sich nicht finden (…)“ (Overlay-Tabelle nicht lesbar); Verlaufsschritte „BISASAM: Ei-Attacken getauscht“ und „BISASAM: Ei-Attacken 12 → 11“.
- **Brücke und MCP:** `data_get species <Art>` trägt `egg_moves {moves, source, budget, locked}` (`source` `narc`, `overlay` oder `patch`, `budget` `{used, total}` nur in Platin ohne Patch, sonst `null`; `egg_moves` ist `null`, wo das Spiel keine Tabelle kennt); `data_edit species <Art> {"EggMoves":{"moves":[…]}}` schreibt die ganze Liste (Ruling R7 gilt nur für Bilder, die Ei-Attacken laufen über `data_get` und `data_edit`); `g4k check-data <rom> --expect egg_moves:<Art>:count=<n> --expect egg_moves:<Art>:move0=<Attacke>` liest HeartGold und Platin mit Patch-NARC aus dem gebauten ROM.

## Teil A

### Vorbereitung

So durchgeführt am 2026-10-31 auf dem Stand `a6f2d41`:

1. **Frische Kopien** unter `Roms/_testkopien/` per `cp -cR` (Ordner) und `cp -c` (`.nds`) aus den Originalen, die dabei nur gelesen wurden; Ordner `3d-3-<zweck>_DSPRE_contents` und `.nds` `3d-3-<zweck>.nds` direkt nebeneinander:
   - `3d-3-pt` (Platin DE, CPUD), `3d-3-hgde` (HeartGold DE, IPKD), `3d-3-hgus` (HeartGold US, IPKE) für (1); sie werden nur gelesen, die Tests schreiben auf `vanilla_copy()`;
   - `3d-3-web` (Platin DE) und `3d-3-hgweb` (HeartGold DE) für (3) und (2) über die Brücke (die Zeitmessung mit `curl` lief auf `3d-3-web`);
   - `3d-3-basept` und `3d-3-basehgde` (Namen `<stem>_DSPRE_contents`, die der gated Test von Task 3 für Basis und Neuaufbau braucht).

   Die Ordner heißen in Wirklichkeit `3d-3-<zweck>` ohne `_DSPRE_contents` (`3d-3-pt`, `3d-3-hgde`, …), nur `3d-3-basept` und `3d-3-basehgde` tragen den Zusatz. Die Kopien für Teil B stehen im Abschnitt „Teil B“, Vorbereitung. Alle `3d-3-*`-Kopien bleiben bis zum Abschluss von Task 13 und werden danach gelöscht (`Manual` und `_saves` bleiben).
2. **Test-Umgebung** für (3): `G4_ROMS_DIR=<Roms> G4_VITE_PORT=1720 G4_BRIDGE_PORT=1721 bash scripts/agent/studio-web.sh` (Debug-Brücke). Der Port 1720 steht auf Chromes Liste unsicherer Ports (H.323): Chrome headless braucht `--explicitly-allowed-ports=1720,1721`, sonst kommt `ERR_UNSAFE_PORT`. Browser: Chrome headless (`--headless=new`, CDP, ein Node-Skript im Scratchpad), 1440 × 1500 (höher als 900, damit die Schau das ganze Profil über der Hand zeigt); Hell und Dunkel über `prefers-color-scheme`, Kompakt über die Taste der Statusleiste.
3. **Ausgaben** unter `target/agent/gate-2026-10-31/3d-3/` im Worktree (nicht im Repo): `gated-release.log`, `mcp-*.out` und `mcp-data_get-397.json`, `ui-*.log`, die Bilder `blick-*.png`, `galerie-*.png`, `pokedex-*.png`, `montage-*.png` und der Ordner `bridge-pt`. Den Ordner vor `git worktree remove` sichern.
4. **Last:** `uptime` im Lauf: 1-Minuten-Last 6 bis 12, weil andere Orchestratoren gleichzeitig arbeiteten (in einem Lauf des abschließenden Reviews bis 18, siehe Zahlentabelle).

### (1) Automatik

- [x] 1. `G4_TEST_PROJECT=<Kopie> cargo test --release -p g4-kernel --all-features --test battle_sprites -- --nocapture` auf `3d-3-{pt,hgde,hgus}`: je 23 bestanden, 1 ignoriert (der Helfer, der Bilder zum Ansehen außerhalb des Repos schreibt). Die gedruckten Zeilen:
   - Platin DE (CPUD): `pokemon=1812 forms=157 trainers=105 components=1.4 corners=1778 palettes16=988 ms=3723`, `trainer_components=1.7`, Formen `images=157 palettes=96 components=1.1 entries=77 rendered=306 form0=12 form0_like_pokegra=[201, 351, 479, 487] female_only=3`.
   - HeartGold DE (IPKD): `pokemon=1812 forms=161 trainers=129 components=1.3 corners=1778 palettes16=988 ms=3204`, `trainer_components=1.9`, Formen `images=161 palettes=100 components=1.1 entries=79 rendered=314 form0=13 female_only=3`.
   - HeartGold US (IPKE): `pokemon=1812 forms=161 trainers=129 components=1.3 corners=1778 palettes16=988 ms=2664`, `trainer_components=1.9`, Formen wie HeartGold DE.
2. `vanilla_every_battle_sprite_decodes_with_the_measured_counts` und `vanilla_every_form_sprite_decodes` sind die zwei gated Tests dazu (`crates/g4-kernel/tests/battle_sprites.rs`); sie prüfen die gemessenen Zahlen (Bilder, Formen und Trainer als Gleichheit, `palettes16` als Gleichheit, das Mittel der zusammenhängenden Flächen höchstens 3, `corners` mindestens 1 778) und, dass sich jedes Formbild von Form 0 unterscheidet. `form0_like_pokegra=[201, 351, 479, 487]` ist der gemessene Satz der Formarten, deren Form 0 aussieht wie der Eintrag in `pokemonBattleSprites` (Icognito, Formeo, Rotom, Giratina); er gilt in allen drei Spielen.
3. Studio-Test im **Debug**-Build: `G4_TIMING_STRICT=1 G4_TEST_PROJECT=<Kopie> cargo test -p studio --all-features --lib battle_sprites_over_the_command_path -- --nocapture` (`battle_sprites_over_the_command_path_on_the_vanilla_copy`, `apps/studio/src-tauri/src/commands/data/tests.rs`): Platin DE `first_ms=12 cached_ms=0 trainer_ms=5`, HeartGold DE `first_ms=8 cached_ms=0 trainer_ms=6`; beide bestanden, die Grenze ist 50 ms je erste Anfrage.
4. Aus Task 3 (selber Branch, früher): die gated Build-Tests `pack_only_entries_repack_byte_identical`, `vanilla_rebuild_matches_dsrom_cli` und `base_assign_on_the_vanilla_folder_has_no_warning_and_rebuilds_the_nds` auf Platin DE und HeartGold DE grün (die Sprite-Archive wanderten aus `pack_only` in benannte Arten).
5. **Nicht gelaufen:** `scripts/gate.sh` als Ganzes. Die Hauptsitzung führt es nach dieser Welle auf main aus; die neuen gated Tests laufen dort im Schritt der gated Tests je Projekt (Debug) mit, das Skript braucht dafür nichts Neues. Die gated Tests für die Ei-Attacken und für die Textprojekte aus Task 11 stehen im Abschnitt „Teil B“, (1).

### (2) Kopflos und MCP

- [x] 1. MCP über die Brücke auf Port 1721 (`scripts/agent/mcp.sh`, Kopie `3d-3-web`, Platin DE; lokale Agenten an, Expertenwerkzeuge an, Freigabe „Lesen“): `g4_call get_battle_sprite {"species":"SPECIES_STARAVIA","side":"front"}` antwortet `{"result":{"bytes":1496}}`; `g4_call get_trainer_sprite {"class":1}` `{"bytes":1691}`; `frame:2` den Fehler „frame must be 0 or 1“; die Art 600 `{"bytes":0}`. `data_get species SPECIES_STARAVIA` trägt `sprite_slots {female:true, male:true}`, die Art 500 `{female:true, male:false}`.
- [x] 2. **Base64 (Ruling R7):** der Plan nahm an, `g4_call` liefere Base64 eines PNG. Gemessen und in [docs/mcp.md](mcp.md) beschrieben: `g4_call` und `g4_batch` antworten für Bildbefehle wie für `get_mon_icon` mit `{"bytes": <Länge>}`; nur die Brücke gibt die PNG-Bytes zurück (`call.sh` druckt dafür „<n> bytes“). Ein Bildwerkzeug für Agenten bleibt offen (Abschnitt „Offen“).
- [x] 3. Zeit über die Brücke (Debug-Brücke, Kopie `3d-3-web`, `curl`, erste Anfrage je Bild): Art 397 vorn über `call.sh`; 25 vorn 15 ms (wieder 2 ms), 6 hinten 13 ms, 150 schillernd 12 ms, 494 (Ei) 8 ms, 421 Form 1 11 ms, 413 schillernd 10 ms, 412 schillernd 11 ms, 407 ♀ 16 ms, 3 ♀ 16 ms, Trainerklasse 5 14 ms, Klasse 104 11 ms; Klasse 105 und Art 600 liefern einen leeren Körper (0 B). Alle höchstens 16 ms, die Grenze ist 50 ms.

Das Szenario kopflos für die Ei-Attacken (`data_get` und `data_edit` auf `egg_moves`, `g4k build`, `g4k check-data`, Platin mit Budget und Patch) steht im Abschnitt „Teil B“, (2).

### (3) Browser

Test-Umgebung auf 1720/1721, Kopien `3d-3-web` (Platin DE) und `3d-3-hgweb` (HeartGold DE), Chrome headless wie in „Vorbereitung“.

- [x] 1. Pokédex Staravia (397), Platin DE und HeartGold DE: der Abschnitt „Spielerblick“ mit `DSPokedexProfile` („Nr. 397“, STARAVIA, Normal und Flug, „Kategorie“, „Größe“ und „Gewicht“ je „folgt“, „Pokédex-Text folgt“); „Vorderseite“ und „Rückseite“ wechseln das Bild (`data-side` front, back), „Schillernd“ zeigt die schillernde Palette (`data-shiny`), „Weiblich“ ist für Staravia frei (eigenes ♀-Bild) und zeigt es; kein waagrechtes Rollen (`scrollWidth` 1440).
- [x] 2. ♀-Sperre: 413 Burmadame und 500 Burmadame (Lumpenumhang) „Nur ♀“ (♀ an und gesperrt, `aria-describedby` zeigt auf den Satz); 421 Kinoso, 412 Burmy, 503 Hitze-Rotom, 494 Ei und in HeartGold 172 Pichu „Kein eigenes Bild für ♀“; 387 Chelast und 25 Pikachu frei. Kinoso (421) zeigt nach dem Fix des abschließenden Reviews die Wolkenform.
- [x] 3. ⌘1 aus Trainer wechselt in den Pokédex wie bisher.
- [x] 4. Trainer 1: Platin „Teenager Gunther“ (Team STARALILI Lv. 5), HeartGold „Rivale Silber“ (Team NEBULAK Lv. 14): Klassenbild 80 im Kopf der Schau (`[data-trainer-sprite]`), 48 im Kopf der Hand; Kampfvorschau mit Gegnerseite „Trainer | Erstes Pokémon“ (`data-foe`); „Trainer“: Kasten nur mit dem Namen („Kampfvorschau: Teenager Gunther“); „Erstes Pokémon“: „STARALILI“ und „Lv. 5“, KP-Balken `meter` 100 `gruen`; die eigene Seite leer (nur Boden). Die HeartGold-Trainerbilder entschlüsseln rückwärts richtig (Silber ist zu erkennen).
- [x] 5. Galerie (`?gallery`, Seite „Bildschirm“): Pokédex-Profil und Kampfvorschau mit Platzhaltern.
- [x] 6. Blick Hell, Dunkel, Kompakt: Bilder unter `target/agent/gate-2026-10-31/3d-3/`: `blick-pokedex-profil-{vorn,hinten,schillernd}-{hell,dunkel,kompakt}.png` und `blick-pokedex-profil-weiblich-hell.png` (HeartGold mit dem Vorsatz `blick-pokedex-profil-hg-`), `blick-trainer-{kopf,kampfvorschau-trainer,kampfvorschau-pokemon}-{hell,dunkel,kompakt}.png` (HeartGold `-hg-`), `galerie-{profil,kampf}-{hell,dunkel,kompakt}.png`, `pokedex-pt-{387,25,413,421,412,500,503,494}.png` und `pokedex-hg-{172,413,421,500,503}.png`. Der Pedestal (`status/rahmen`, graue Ellipse) ist auf dem dunklen DS-Grund in Hell und Dunkel zu sehen; der Text in den mit `transform` vergrößerten Plaketten ist bei Maßstab 2 in Chrome scharf.

### (4) Hand und Blick

Beim Nutzer (Abschnitt „Hand“ und „Blick“ unten und die Tabelle „Ergebnis“):

- **Hand:** die Bilder gegen das Spiel im Emulator (Pokédex und Kampf), vor allem Kinoso Wolken- und Sonnenform, Burmy und Burmadame schillernd, Pichu (HeartGold, Form 0, siehe „Offen“) und die HeartGold-Trainerbilder.
- **Blick:** die Bilder unter `target/agent/gate-2026-10-31/3d-3/` (Profil, Trainer, Galerie; U2 und U3).
- **Formen am Augenschein:** [task3-3d3-formen.md](research/2026-10-30-phase-3d/task3-3d3-formen.md).
- **`.fig`:** die Frames für `DSPokedexProfile` und `DSBattlePreview` folgen (OpenPencil war nicht erreichbar; `design-system/README.md`, „Nachtrag 3d.3“).

### Ergebnis

| Punkt | Wer | Ergebnis | Datum |
|---|---|---|---|
| (1) 1 gated `--test battle_sprites` | Agent | bestanden: je 23/23 (1 ignoriert) auf Platin DE, HeartGold DE, HeartGold US (Release); 1 812 Pokémon-Bilder, 157 / 161 Formenbilder, 105 / 129 Trainerbilder, Mittel der Flächen ≤ 3 (Pokémon 1,3 bis 1,4, Trainer 1,7 bis 1,9), 1 778 von 1 812 mit durchsichtiger Ecke | 2026-10-31 |
| (1) 2 gated Studio-Test | Agent | bestanden: erste Anfrage 12 ms (Platin DE) und 8 ms (HeartGold DE), Debug-Build, `G4_TIMING_STRICT=1`, Grenze 50 ms | 2026-10-31 |
| (1) 3 gated Build-Tests (Task 3) | Agent | bestanden auf Platin DE und HeartGold DE | 2026-10-31 |
| (1) 4 `scripts/gate.sh` | Agent | offen: läuft in der Hauptsitzung nach der Welle (nicht von diesem Teil gestartet) | – |
| (2) 1 MCP `g4_call`, `data_get` | Agent | bestanden: `{"bytes":1496}`, `{"bytes":1691}`, Fehler bei `frame:2`, `{"bytes":0}` für Art 600, `sprite_slots` | 2026-10-31 |
| (2) 2 Base64 des Plans | Agent | durch Messung ersetzt (Ruling R7): `g4_call` antwortet die Länge, die Brücke das PNG | 2026-10-31 |
| (2) 3 Zeit über die Brücke | Agent | bestanden: höchstens 16 ms je erste Anfrage (Debug-Brücke) | 2026-10-31 |
| (3) 1 bis 6 Browser-Teil | Agent | bestanden auf `a6f2d41`: Profil mit Umschaltern, ♀-Sperre, Trainer-Kopf und Kampfvorschau auf Platin DE und HeartGold DE, Galerie | 2026-10-31 |
| Bilder gegen das Spiel (Emulator) | Hand | offen | – |
| Formen am Augenschein (Task 3) | Agent + Blick | Protokoll liegt vor; der Nutzer bestätigt am Bild | – |
| Blick auf Profil, Trainer-Kopf, Kampfvorschau in Hell, Dunkel, Kompakt | Agent + Blick | offen: 20 Bilder Profil (je 10 Platin und HeartGold), 18 Trainer, 6 Galerie liegen bereit (dazu `pokedex-*.png`), vom Agenten angesehen; U2 und U3 sind Blick-Punkte | – |
| `.fig` | – | nicht angefasst (OpenPencil nicht erreichbar, Fehler 502); die Frames folgen | – |
| Teil B (Ei-Attacken) | Agent | siehe Abschnitt „Teil B“, „Ergebnis“ | 2026-11-01 |

Zahlentabelle (Release, Werte aus den gated Tests (1), in Klammern aus (2); die Spalte „Ei-Attacken“ stammt aus Teil B, `egg_moves`-Test auf `3d-3-{pt,hgde,hgus}`, Einzelheiten im Abschnitt „Teil B“):

| Spiel | Pokémon-Bilder | Formen | Trainer | Flächen (Mittel) | Ecken | erstes Sprite (ms) | Ei-Attacken Arten/Attacken | Last |
|---|---|---|---|---|---|---|---|---|
| Platin DE | 1 812 | 157 Bilder, 96 Paletten, 77 Einträge | 105 | 1,4 Pokémon, 1,7 Trainer, Formen 1,1 | 1 778 von 1 812 | 12 Debug-Studio-Test (Brücke 8 bis 16) | 198 / 1 711 (Overlay 5, 1 910 Wörter) | 6 bis 12 (CPUD-Lauf des Reviews bis 18, `ms=16811` statt 3 723; Teil B 13 bis 19) |
| HeartGold DE | 1 812 | 161 Bilder, 100 Paletten, 79 Einträge | 129 | 1,3 Pokémon, 1,9 Trainer, Formen 1,1 | 1 778 von 1 812 | 8 Debug-Studio-Test (Brücke 8 bis 16) | 198 / 1 847 (NARC, 2 046 Wörter) | 6 bis 12 (Teil B 13 bis 19) |
| HeartGold US | 1 812 | 161 Bilder, 100 Paletten, 79 Einträge | 129 | 1,3 Pokémon, 1,9 Trainer, Formen 1,1 | 1 778 von 1 812 | nicht einzeln gemessen (Studio-Test nur auf Platin DE und HeartGold DE) | 198 / 1 847 (NARC, 2 046 Wörter) | 6 bis 12 (Teil B 13 bis 19) |

Zeiten der gated Läufe (`ms=` im Test, Release): Platin DE 3 723, HeartGold DE 3 204, HeartGold US 2 664 für alle 1 812 Pokémon-Bilder und die Trainer; unter Last im Lauf des abschließenden Reviews Platin DE 16 811, HeartGold DE 3 765, HeartGold US 3 547 (Last um 18). Eine Grenze setzt der Test dafür nicht.

### Befunde

#### UI-Test (2026-10-31)

| Nr. | Schwere | Befund | Fix |
|---|---|---|---|
| U1 | Klein | Lange Formnamen („Burmadame, Lumpenumhang“) waren im Profil hart abgeschnitten. | `a6f2d41` (Review sauber, im UI nachgeprüft): Namen im Profil und in den Statuskästen enden mit „…“ und tragen den vollen Namen als `title`; je ein Testfall je Baustein. |
| U2 | Klein (Blick) | In Dunkel ist der eigene Grund des DS-Bildschirms (`ds/grund`) fast der Grund der App; die Kante des Bildschirms verschwindet fast (der vorhandene Rahmen von `DSScreen`, nicht neu in 3d.3). | Keine Änderung; der Nutzer entscheidet am Bild. |
| U3 | Klein (Blick) | Kleine Arten stehen in der Kampfvorschau über dem Boden (feste Lage ohne `pokemonSpriteOffsets`, Planentscheidung „feste Lage“). | Keine Änderung; Bilder `blick-trainer-kampfvorschau-pokemon-*.png`. |

#### Abschließendes Review von Teil A

Das Review des ganzen Teils (Opus) fand einen wichtigen Punkt und mehrere kleine; eine Fix-Runde (`476f0e3`, `6331244`), danach sauber.

| Nr. | Schwere | Befund | Fix |
|---|---|---|---|
| 1 | Wichtig | Form 0 der Arten mit Formen kam aus `pokemonBattleSprites`; das Spiel zeichnet sie nur aus `otherPokemonBattleSprites`: Kinoso zeigte die Sonnenform für die Wolkenform, Burmy, Burmadame, Schalellos und Gastrodon schillernd gleich normal, Burmadames Palette weicht in 13 von 16 Farben ab. | `6331244` (Ruling R8): Einträge für Form 0 jeder Art mit Formen (Platin 12, HGSS 13), gemessen an `3d-3-pt` und `3d-3-hgde`, am Augenschein bestätigt; Test `form_zero_of_a_species_with_forms_comes_from_the_other_archive`, der gated Test vergleicht die Form-0-Bilder mit `pokegra` (`form0_like_pokegra=[201, 351, 479, 487]`). |
| 2 | Klein | Mit Form 0 aus dem zweiten Archiv hätte Burmadame („Nur ♀“) die Sperre „Kein eigenes Bild für ♀“ bekommen. | `6331244` (Rulings R9 und R10): `sprite_slots` nimmt die Geschlechterverteilung; 254 macht ein einziges Bild zum weiblichen; die Regel gilt für jede Art (in Vanilla ändert sie dort nichts: die 16 rein weiblichen Arten liegen in `pokegra` schon in den weiblichen Plätzen). Tests im Kernel und im Studio, gated `female_only=3`. |
| 5 | Klein | Die Sätze zu den Bildbefehlen in `docs/mcp.md` trennten einen Absatz. | `476f0e3`. |
| 3, 4, 6, 7, 8 | Klein | Dekodieren eines ersten Bildes unter der Sitzungssperre (8 bis 15 ms); `data_get species` liest das `pokegra`-NARC je Aufruf bei gepackten Projekten (5 bis 7 ms); „Nr.“ zeigt für Formen 494 bis 507 den Personal-Member; Sprite 48 ist nicht ganzzahlig skaliert (vom Plan vorgegeben); `TileMode` ist streng (Plan-Entscheidung). | Geparkt (Plan- und Bestandsentscheidungen, nicht tragend; Abschnitt „Offen“). |

### Offen

Ohne Einfluss auf Teil A; was ein Nutzer bemerken kann:

- **Pichu (HeartGold), Form 0 (Ruling R11):** der Eintrag kommt wie die anderen Form-0-Arten aus `otherPokemonBattleSprites` (Member 154 und 155), ohne dass gemessen ist, ob HeartGold Pichu dort zeichnet. Ein Eintrag in `battle_sprites.yaml`; der Nutzer prüft ihn im Emulator.
- **Zukan-Daten:** Pokédex-Text, Kategorie, Größe und Gewicht bleiben „folgt“, bis eine Messung die Bänke und Tabellen belegt (eigener kleiner Posten, nicht 3d).
- **Feste Lage:** `pokemonSpriteOffsets` (494 × 89 Byte) sind ungeprüft; die Kampfvorschau setzt jedes Bild an dieselbe Stelle (U3). Der Boden ist stilisiert, kein Kampfhintergrund; die eigene Seite der Kampfvorschau bleibt leer.
- **Bildwerkzeug für Agenten:** `g4_call` liefert nur die Länge (R7); ein MCP-Werkzeug, das ein PNG ausgibt, wäre ein eigener Posten.
- **Diamant und Perl:** ohne Tabelle und ohne Richtung (Familie ungemessen); `get_battle_sprite` liefert dort nichts.
- **Geparkt aus den Reviews:** das Dekodieren eines ersten Bildes hält die Sitzungssperre (8 bis 15 ms); `data_get species` parst das `pokegra`-NARC je Aufruf bei gepackten Projekten; ein Fehler eines kaputten Archivs wird als „kein Bild“ gemeldet und nicht gemerkt; „Nr.“ zeigt für Formen 494 bis 507 den Personal-Member (`SpeciesRowExtra`); die Pokédex-Textbox schneidet nach zwei Zeilen ohne Hinweis ab (heute „folgt“); die KP-Farbe und die Prozentzahl können an den Schwellen auseinandergehen (gerundet gegen ungerundet); `level` wird nicht normiert („Lv. 12.5“).
- **Beim Nutzer:** Hand (Bilder gegen das Spiel), Blick (U2, U3), `.fig` (Frames).

## Teil B

### Vorbereitung

So durchgeführt am 2026-11-01 auf dem Stand `3555171a` (Basis des Teils: main `79eb730` mit 3d.1, 3d.2, 4.5 und dem Merge von Teil A):

1. **Frische Kopien** unter `Roms/_testkopien/` per `cp -cR` (Ordner) und `cp -c` (`.nds`), die Originale nur gelesen:
   - `3d-3-pt` (Platin DE), `3d-3-hgde` (HeartGold DE), `3d-3-hgus` (HeartGold US) für (1) (nur gelesen; die Tests schreiben auf `vanilla_copy()`); es sind die Kopien aus Teil A;
   - `3d-3-kpt` (Platin DE) und `3d-3-khgde` (HeartGold DE) für das Szenario kopflos (2);
   - `3d-3-patch` (Platin DE mit einem synthetischen `files/zcustom/egg_moves.narc`, einer Kopie von `files/a/2/2/9` aus HeartGold DE, das den HGSS-Member-Aufbau trägt; so liest das Studio es als DSPRE-Patch, ohne dass ein gepatchtes Platin-Projekt vorliegt) für (2);
   - `3d-3-webb` (Platin DE) und `3d-3-hgwebb` (HeartGold DE) für (3) über die Brücke;
   - `3d-3-basept` und `3d-3-basehgde` (Namen `<stem>_DSPRE_contents`) für die gated Textprojekt-Tests aus Task 11.

   Alle `3d-3-*`-Kopien bleiben bis zum Abschluss von Task 13 und werden danach gelöscht (`Manual` und `_saves` bleiben).
2. **Test-Umgebung** für (3) wie in Teil A: `G4_ROMS_DIR=<Roms> G4_VITE_PORT=1720 G4_BRIDGE_PORT=1721 bash scripts/agent/studio-web.sh` (Debug-Brücke), Chrome headless mit `--explicitly-allowed-ports=1720,1721`, 1440 × 1500, Hell und Dunkel über `prefers-color-scheme`, Kompakt über die Taste der Statusleiste. `g4k` im Release-Profil (`cargo build --release -p g4-kernel --bin g4k`).
3. **Ausgaben** unter `target/agent/gate-2026-11-01/3d-3/` im Worktree (nicht im Repo): `gated-release.log`, `kopflos.log`, `ui-eiattacken-{pt,hg}.log` und die Bilder `blick-pokedex-eiattacken-*.png`. Den Ordner vor `git worktree remove` sichern.
4. **Last:** `uptime` im Lauf: 1-Minuten-Last 13 bis 19, weil andere Orchestratoren gleichzeitig arbeiteten (3d.4, 3d.5, 3d.6).

### (1) Automatik

- [x] 1. `G4_TEST_PROJECT=<Kopie> cargo test --release -p g4-kernel --all-features --test egg_moves --test battle_sprites -- --nocapture` auf `3d-3-{pt,hgde,hgus}`: `egg_moves` je 14 bestanden, `battle_sprites` je 23 bestanden und 1 ignoriert. Die gedruckten Zeilen der Ei-Attacken:
   - Platin DE (CPUD): `egg_moves CPUD source=overlay species=198 moves=1711 words=1910 ms=11`; Marken aufsteigend (`ascending=true first=1 last=459`).
   - HeartGold DE (IPKD): `egg_moves IPKD source=narc species=198 moves=1847 words=2046 ms=0`; aufsteigend.
   - HeartGold US (IPKE): `egg_moves IPKE source=narc species=198 moves=1847 words=2046 ms=1`; aufsteigend.
   - Die Zeilen der Kampfsprites sind unverändert gegenüber Teil A (CPUD 1 812 / 157 / 105, IPKD und IPKE 1 812 / 161 / 129, `corners=1778`, `palettes16=988`).
2. `vanilla_egg_moves_read_with_the_measured_counts` und `vanilla_egg_moves_are_in_ascending_species_order` (`crates/g4-kernel/tests/egg_moves.rs`) sind die zwei gated Tests dazu: der erste prüft 198 Arten, in HeartGold 1 847 Attacken (höchstens 14 je Art, 4 092 Byte) und in Platin 1 711 Attacken, 1 910 Wörter und das Budget 1 910, dazu `write()` byte-gleich zum Member bzw. zum Ausschnitt in `ov005.bin`; der zweite druckt nur, ob die Marken aufsteigen (Task 11 entschied danach: kein Feld `order`, der Import schreibt aufsteigend).
3. **Gated Textprojekt-Tests** (der Umsetzer der Fix-Runde, auf `3555171a`, Kopien `3d-3-basept` und `3d-3-basehgde`): `text_project_egg_move_edit_builds_and_check_data_finds_it` (`crates/g4-kernel/tests/textproject_source.rs`; Textprojekt exportiert, Ei-Attacken 1 und 2 von Art 1 getauscht und gespeichert: nur `data/species/0001.json` ändert sich, der Abgleich findet nichts offen, der Bau trägt den Tausch; HeartGold per `g4k check-data egg_moves`, Platin im Ausschnitt von `ov005.bin` der entpackten ROM): CPUD `egg-moves-text CPUD count=12 move0=130 (was 113)`, IPKD `egg-moves-text IPKD count=14 move0=130 (was 113)`, beide bestanden; `textproject_round_trip` und `textproject_export_idempotent` (`tests/textproject.rs`) auf beiden bestanden (`raw=0`, die neu gebaute ROM gleicht der Basis).
4. **Ohne ROM-Daten:** Task 8 (Vorspulen auf main `79eb730`): `cargo fmt`, `clippy`, `cargo test --workspace --all-features`, `pnpm test` (3 591), Bau, `check-styles`, `check-tokens`, `check-depth` grün. Nach Teil B: `g4-kernel` ohne gated Tests 1 626 bestanden, `studio` (Lib) 699 bestanden und 1 ignoriert, `pnpm test` 3 602 bestanden (Task 12), `pnpm build` grün.
5. **Nicht gelaufen:** `scripts/gate.sh` als Ganzes. Die Hauptsitzung führt es nach dem Merge auf main aus; die neuen gated Tests laufen dort im Schritt der gated Tests je Projekt mit, das Skript braucht dafür nichts Neues.

### (2) Kopflos (Brücke mit `call.sh`, `g4k` im Release-Profil)

Das Szenario liegt als Skript im Scratchpad (nicht im Repo), die Ausgabe in `kopflos.log`. Es ruft über `scripts/agent/call.sh` die Befehle `data_get`, `data_edit`, `undo`, `history`, `save`, `build_rom` und danach `g4k check-data`; je Kopie ein frischer Lauf der Brücke.

- [x] 1. **HeartGold DE** (`3d-3-khgde`): `data_get species 1` → `egg_moves` mit `source` `narc`, `budget` `null`, 14 Attacken (die ersten 113, 130, 219), 67 ms; Tausch von `move0` 113 gegen 1 (Pfund) über `data_edit species 1 {"EggMoves":{"moves":[…]}}` ok; ⌘Z (`undo`) stellt 113 wieder her; erneut getauscht, `save` (der Member geschrieben, `ov005.bin` unverändert, `source` bleibt `narc`); `g4k build` ok; `g4k check-data <rom> --expect egg_moves:1:count=14 --expect egg_moves:1:move0=1` meldet „2 expectation(s) met“; mit falscher Zahl Rückgabe 1 und „1 of 1 expectation(s) failed“.
- [x] 2. **Platin DE** (`3d-3-kpt`): `data_get species 1` → `source` `overlay`, `budget` `{used 1910, total 1910}`, 12 Attacken, 94 ms; Tausch ok, ⌘Z stellt 113 wieder her; **Hinzufügen ohne Entfernen** abgelehnt mit `{"code":"error","message":"Ei-Attacken-Tabelle voll: 1 910 von 1 910 Einträgen"}`, der Verlauf bleibt unverändert; **Handel** (Art 1 eine Attacke weniger, Art 7 eine mehr, zwei Schritte) ok; `save` ändert den Ausschnitt in `ov005.bin`; `g4k build` ok; `g4k check-data` antwortet erwartungsgemäß „CPUD has no egg_moves archive“ (der Plan: `check-data` liest die Tabelle in Overlay 5 einer gebauten ROM nicht; der gated Textprojekt-Test aus (1) 3 liest den Ausschnitt aus der gebauten ROM).
- [x] 3. **Patch** (`3d-3-patch`): `data_get species 1` → `source` `patch`, `budget` `null`, 14 Attacken, 43 ms; Tausch ok, ⌘Z stellt 113 wieder her; `save` lässt `ov005.bin` byte-gleich (die SHA-Summe unverändert), `source` bleibt `patch`.
- Ruling R7 (Bilder nur über die Brücke) gilt hier nicht: Ei-Attacken sind Daten und gehen über `data_get` und `data_edit`, auch über `g4_call` und MCP (`data_edit` ist ein kuratiertes Werkzeug).

### (3) Browser

Test-Umgebung auf 1720/1721, Kopien `3d-3-webb` (Platin DE) und `3d-3-hgwebb` (HeartGold DE). Pfad: Pokémon › Pokédex › BISASAM 0001 › Hand „Bearbeiten“ › Reiter „Ei-Attacken“.

- [x] 1. **Platin DE:** 12 Zeilen (Lichtschild, Schädelwumme, Bodyguard …) mit Attacken-Picker und `TypeBadge`; Fußzeile `data-egg-budget` „12 Ei-Attacken · Tabelle 1 910 von 1 910 Einträgen“; „Zeile hinzufügen“ `aria-disabled` mit „Tabelle voll: erst eine Ei-Attacke bei einer anderen Art entfernen“ (`aria-describedby`). Letzte Zeile entfernt: 11 Zeilen, „1 909 von 1 910“, „Zeile hinzufügen“ frei; ⌘Z: 12 Zeilen, 1 910, wieder gesperrt. Zeile 1 über den Picker getauscht gegen „Pfund“ (Attacke 1): das Budget bleibt 1 910; ⌘Z: wieder 113.
- [x] 2. **HeartGold DE:** 14 Zeilen, Fußzeile „14 Ei-Attacken“ (ohne Budget), „Zeile hinzufügen“ frei; „Pfund“ hinzugefügt: 15 Zeilen; ⌘Z: 14.
- [x] 3. Kein waagrechtes Rollen (`scrollWidth` 1440). Die Fußzeile liegt bei 1440 × 1500 unter dem sichtbaren Rand; ihre Werte stammen aus dem DOM.
- [x] 4. Blick Hell, Dunkel, Kompakt: Bilder unter `target/agent/gate-2026-11-01/3d-3/`: `blick-pokedex-eiattacken-pt-{hell,dunkel,kompakt}.png`, `-pt-nach-entfernen-hell.png`, `-pt-getauscht-hell.png`, `blick-pokedex-eiattacken-hg-{hell,dunkel,kompakt}.png`, `-hg-hinzugefuegt-hell.png`.

### (4) Hand und Blick

Beim Nutzer:

- **Hand:** eine Ei-Attacke in Platin im Spiel wirksam: nach einem Tausch innerhalb des Budgets züchten (Hand: Zucht im Emulator oder am Gerät). Dazu ein echtes, mit DSPRE gepatchtes Platin-Projekt (Patch-NARC, Klon und Wiederherstellung): getestet ist der Patch nur mit dem synthetischen NARC von `3d-3-patch`.
- **Blick:** die Bilder `blick-pokedex-eiattacken-*.png` (Platin und HeartGold, Hell, Dunkel, Kompakt, dazu die Zustände „nach Entfernen“, „getauscht“, „hinzugefügt“).
- **`.fig`:** der Frame für den Reiter „Ei-Attacken“ folgt (OpenPencil war nicht erreichbar; `design-system/README.md`, „Nachtrag 3d.3“).

### Ergebnis

| Punkt | Wer | Ergebnis | Datum |
|---|---|---|---|
| (1) 1 gated `--test egg_moves` | Agent | bestanden: je 14/14 auf Platin DE, HeartGold DE, HeartGold US (Release); 198 Arten, Platin 1 711 Attacken in 1 910 Wörtern (Budget 1 910, Overlay), HeartGold 1 847 Attacken in 2 046 Wörtern (höchstens 14 je Art), `write()` byte-gleich, Marken aufsteigend | 2026-11-01 |
| (1) 2 gated Textprojekt-Tests (Task 11) | Agent | bestanden auf `3555171a`, Platin DE und HeartGold DE: `text_project_egg_move_edit_builds_and_check_data_finds_it`, Round-Trip und Idempotenz (`raw=0`, ROM gleich Basis) | 2026-11-01 |
| (1) 3 Tests ohne ROM-Daten | Agent | bestanden: nach Teil B `g4-kernel` 1 626, `studio` 699, `pnpm test` 3 602 (Task 12: `pnpm build`, `check-styles`, `check-tokens`, `check-depth` grün); Task 8 (vor Teil B): `fmt`, `clippy`, `cargo test --workspace`, `pnpm test` 3 591, Bau, `check-styles`, `check-tokens`, `check-depth` | 2026-11-01 |
| (1) 4 `scripts/gate.sh` | Agent | offen: läuft in der Hauptsitzung nach dem Merge (nicht von diesem Teil gestartet) | – |
| (2) 1 HeartGold: Tausch, ⌘Z, speichern, bauen, `check-data` | Agent | bestanden: `source narc`, 14 Attacken, „2 expectation(s) met“, falsche Zahl Rückgabe 1 | 2026-11-01 |
| (2) 2 Platin: Budget, Tausch, Hinzufügen abgelehnt, Handel | Agent | bestanden: `source overlay`, 1 910 von 1 910, Tausch ok, Hinzufügen mit dem Satz abgelehnt (Verlauf unverändert), Handel in zwei Schritten ok, `ov005.bin` geändert; `check-data` liest Overlay 5 nicht (geplant) | 2026-11-01 |
| (2) 3 Patch-NARC | Agent | bestanden: `source patch`, Tausch ok, ⌘Z, `ov005.bin` nach `save` byte-gleich | 2026-11-01 |
| (3) 1 bis 4 Browser | Agent | bestanden auf `3555171a`: Reiter auf Platin DE (12 Zeilen, Budget, gesperrt, nach Entfernen frei, ⌘Z) und HeartGold DE (14 Zeilen, Hinzufügen geht, ⌘Z), Bilder Hell, Dunkel, Kompakt | 2026-11-01 |
| Platin-Ei-Attacke im Spiel (Zucht) | Hand | offen | – |
| Echtes DSPRE-gepatchtes Platin-Projekt (Patch-NARC, Klon, Wiederherstellung) | Hand | offen | – |
| Blick auf den Reiter „Ei-Attacken“ in Hell, Dunkel, Kompakt | Agent + Blick | offen: 9 Bilder liegen bereit; der Agent hat zwei angesehen (`blick-pokedex-eiattacken-pt-hell.png`, `blick-pokedex-eiattacken-hg-dunkel.png`), den Rest aus dem DOM gelesen | – |
| `.fig` | – | nicht angefasst (OpenPencil nicht erreichbar); der Frame folgt | – |

Zahlentabelle (Teil B; Release, Werte aus (1) 1):

| Spiel | Ort (`source`) | Arten | Attacken | Wörter | Lesen (ms) | Last |
|---|---|---|---|---|---|---|
| Platin DE | Overlay 5 (`overlay`) | 198 | 1 711 | 1 910 (Budget 1 910) | 11 | 13 bis 19 |
| HeartGold DE | NARC `a/2/2/9` (`narc`) | 198 | 1 847 | 2 046 | 0 | 13 bis 19 |
| HeartGold US | NARC `a/2/2/9` (`narc`) | 198 | 1 847 | 2 046 | 1 | 13 bis 19 |

### Befunde

#### Rulings

Die Plan-Abweichungen von Teil B stehen im Ledger der Umsetzung (`progress.md`, nicht im Repo); hier die, die ein Leser des Codes braucht:

| Nr. | Gegenstand | Ruling |
|---|---|---|
| R12 | Schema-Version | `SCHEMA_VERSION` bleibt wie auf main (1), kein Migrationsschritt: das Feld `egg_moves` ist auslassbar (Plan: „keine Erhöhung, wenn das Feld auslassbar ist“); `schema.snap` bekommt `species.egg_moves?`. |
| R13 | Patch-Erkennung in Platin | Der Patch gilt, wenn `files/zcustom/egg_moves.narc` **oder** der Ordner `unpacked/eggMoves` da ist (der Plan nannte nur den NARC): die `.gitignore` des DSPRE-Layouts schließt den NARC jedes entpackten Ordners aus, ein Klon eines gepatchten Projekts hat nur den Ordner. Ein Vanilla-Platin-Projekt mit einem verirrten Ordner `eggMoves` läse ihn als Patch. |
| R14 | Obergrenze | `DataPatch::EggMoves` nimmt höchstens 16 Attacken je Art (`check-data` kennt `move0` bis `move15`; Vanilla höchstens 14). Ein Hack mit längeren Listen lässt sich darüber nicht bearbeiten; die Oberfläche sagt „Höchstens 16 Ei-Attacken je Art“. |
| R15 | Marke im Manifest | `eggMoves: true` in `g4-project.json` (Export und Speichern der Ei-Attacken setzen sie). Ohne die Marke lässt der Import die Tabelle liegen und warnt mit `textproject.egg-moves-unexported`, sobald eine Spezies-Datei das Feld trägt (der Plan: „kein Eintrag = fehlt“ gilt nur mit der Marke): sonst löschte ein gemischter Stand aus alten und neuen Dateien Einträge. Nutzer alter Projekte exportieren einmal. |
| R16 | Patch und Git-Klon | Das Manifest trägt `eggMovesPatch: true`, wenn die Tabelle in DSPREs Patch liegt. Import und Wiederherstellung legen mit der Marke `unpacked/eggMoves/0000` aus den Spezies-Dateien an (der Ordner kennzeichnet den Patch wieder nach R13, der Bau packt den NARC), Overlay 5 bleibt unberührt. Auf Import und Wiederherstellung begrenzt; nach einer gescheiterten Wiederherstellung hilft ein erneuter Import. |

#### Abschließendes Review von Teil B

Das Review über `79eb730..a41f8103` (Opus) fand einen kritischen Punkt, einen wichtigen und vier kleine; eine Fix-Runde (`3555171a`), danach sauber (Nachprüfung des Umfangs: alles behoben, nichts Neues).

| Nr. | Schwere | Befund | Fix |
|---|---|---|---|
| 1 | Kritisch | DSPREs Patch ging über einen Klon des Textprojekts verloren: NARC und Member sind Bauergebnis und beide in der `.gitignore`, `egg_moves_place` fiel auf Overlay 5 zurück. | `3555171a` (Ruling R16): Manifest-Marke `eggMovesPatch`; Test `a_clone_of_a_patched_text_project_gets_the_patch_back` (rot ohne die Marke: `left: "arm9_overlays/ov005.bin"`, `right: "unpacked/eggMoves/0000"`). |
| 2 | Wichtig | Der gated Test „bearbeiten, bauen, `check-data`“ lief nach den Fix-Runden von Task 11 nicht mehr. | Wiederholt auf `3555171a`, Platin DE und HeartGold DE, dazu Round-Trip und Idempotenz: grün (Abschnitt (1) 3). |
| 3 | Klein | Der Textimport nahm Attacke 0 und Attacken über der Zahl der Attacken an, anders als `data_edit`. | `3555171a`: `textproject.schema` an `egg_moves[i]` („… ist keine Ei-Attacke (Attacken 1 bis 467)“); Tests im Import und für die Feld-Zahlen. |
| 4 | Klein | Ein älteres HGSS-Textprojekt mit dem Blob des Ei-Members endete im Fehler `textproject.raw-path`. | `3555171a`: der Blob gilt ohne die Marke `eggMoves` weiter als der Member, der nächste Export übernimmt die Tabelle (Test `an_egg_move_blob_of_an_older_text_project_is_its_member`). |
| 5 | Klein | Eine nicht lesbare Overlay-Tabelle (`overlays.yaml` fehlt oder kaputt) brach das Laden oder den Import mit einem harten Fehler ab. | `3555171a`: die Tabelle ist gesperrt mit „Ei-Attacken: Overlay 5 lässt sich nicht finden (…)“ (Test `an_overlay_table_that_does_not_read_locks_the_egg_moves`). |
| 6 | Klein | Ein Ausschnitt mit gültiger Signatur, aber Auffüllung ungleich `0xFF`, wurde gesperrt mit englischem Grund. | `3555171a`: die Auffüllung bleibt, solange die Wortzahl bleibt; sonst Auffüllung mit `0xFFFF` (Tests im Codec und `a_slice_whose_padding_is_not_0xff_loads_and_saves_unchanged`). |

Aus den Fix-Runden je Task: Task 9 (Patch-Ordner als Marke, R13; Phantom-Attacke 0), Task 11 (die Übernahme der Ei-Attacken wandte offene Schreibvorgänge der Arten an; ein gemischter Feldzustand löschte Einträge, R15; falsche Konflikte im Bau-Schritt). Task 11 hob dazu `TEXTPROJECT_REV` von 1 auf 2 (ein voller Lauf) und berührt `sync/watch.rs`, `sync/apply.rs` und `step.rs`, weil eine Spezies-Datei zwei Einheiten gehört (der Art und den Ei-Attacken).

### Offen

Ohne Einfluss auf die Bedienung; was ein Nutzer bemerken kann:

- **Platin-Ei-Attacken im Spiel:** die Wirkung einer geänderten Tabelle in Overlay 5 ist am Spiel nicht gesehen (Hand: Zucht).
- **Gepatchtes Platin-Projekt:** der Patch-Pfad ist mit dem synthetischen NARC von `3d-3-patch` und dem Test der Wiederherstellung geprüft, nicht mit einem echten DSPRE-Projekt. Geparkt: die Wiederherstellung eines gepatchten Klons blendet den Ausschnitt in `ov005.bin` aus, während sie die Blobs legt (die Bytes im Ausschnitt `0x2923E` bis `0x2A12A` blieben vanilla, falls DSPREs Patch dort Bytes änderte; der Patch biegt den Code normalerweise außerhalb davon um).
- **`check-data` und Overlay 5:** `g4k check-data egg_moves` liest die Tabelle in Overlay 5 einer gebauten ROM nicht („CPUD has no egg_moves archive“); Platin prüft das Gate über `data_get` und die Bytes von `ov005.bin` (Overlays aus der ROM zu lesen, womöglich mit BLZ, wäre ein eigener Posten).
- **Andere Platin-Sprachen:** gemessen ist nur Platin DE (`CPUD`, Offset `0x2923E`); die anderen Codes stehen in `overlay_tables.yaml` als `unverified`, die Tabelle ist dort gesperrt („Ei-Attacken: Lage in Overlay 5 für CPUE nicht geprüft“). Diamant und Perl haben keine bekannte Tabelle.
- **Über das Budget hinaus:** in Platin ohne Patch nur Tauschen und Handeln innerhalb der 1 910 Wörter; Hinzufügen braucht ein Entfernen bei einer anderen Art (die Verschiebung der Tabelle ist nur über DSPREs Patch möglich).
- **Obergrenze 16** (R14): eine Art mit mehr als 16 Ei-Attacken in einem Hack lässt sich nicht speichern.
- **Referenzindex:** Ei-Attacken zählen nicht als Quelle für `Target::Move` (der Plan; „Verwendet in“ einer Attacke zählt sie nicht).
- **Geparkt aus den Reviews:** `reload_from_disk` ignoriert die Sperre der Ei-Attacken; `data_list` liest je Ei-Attacken-Änderung neu; die Begründung „Kopfdaten-Tabelle“ im Fehler für einen fehlenden Ausschnitt ist ungenau; `source` kann von `moves` und `budget` abweichen, wenn der Patch während des Öffnens erscheint; ein Textprojekt-Speichern auf einem alten Projekt lässt den älteren Blob liegen (der nächste Import weist ihn ab, bis er gelöscht ist) und ein Hack mit Ei-Attacke ab der Zahl der Attacken exportiert Felder, die sein Import ablehnt; nach einer gescheiterten Wiederherstellung kann ein Speichern der Ei-Attacken im Studio `ov005.bin` schreiben und die Marke löschen (R16 gilt nur für Import und Wiederherstellung); eine gesperrte Ei-Attacken-Meldung zeigt „..“, wenn der Grund mit einem Punkt endet; zwei gleiche Attacken in einer Art haben zwei Tasten mit demselben Namen; die Budget-Sperre von „Zeile hinzufügen“ greift bei einer Art mit leerer Marke und genau einem freien Wort, obwohl der Kern annähme (eine leere Marke ist in den gemessenen Tabellen nicht belegt).
- **Beim Nutzer:** Hand (Zucht in Platin, echtes DSPRE-Patch-Projekt), Blick (die Bilder des Reiters), `.fig` (Frame).
