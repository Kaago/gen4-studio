# Phase 3d.3: Gate prüfen

Stand: 2026-10-31. Branch `phase-3d-3-sprites-ds-eier` (Worktree `.claude/worktrees/phase-3d-3`, Basis main `1d68c2b`), Kopf `a6f2d41`, nicht nach main gemergt. **Teil A** (Kampfsprites, DS-Profil, Kampfvorschau; Plan-Tasks 1 bis 7) ist umgesetzt, von der Review-Schleife je Task und einem abschließenden Review des ganzen Teils abgenommen (eine Fix-Runde) und im UI-Test am selben Tag auf `a6f2d41` (Bilder auf `6331244` und `a6f2d41`) durchlaufen: (1) gated Tests auf Platin DE, HeartGold DE und HeartGold US, (2) Kopflos über MCP und die Brücke, (3) Browser, ohne offenen Befund außer den Blick-Punkten U2 und U3. **Teil B** (Ei-Attacken, Plan-Tasks 8 bis 12) wartet, bis 3d.1 auf main ist (Prüfung am 2026-10-31: `git grep -l flags.json main -- crates/g4-kernel/src/textproject` leer, main `c6345d6`); dieses Dokument wird mit Teil B vollständig (Abschnitt „Teil B“).

Phase 3d.3 bringt in Teil A: Kampfsprites nur lesen (`g4-formats` liest den Kachelmodus, rendert Modus 1 linear und entschlüsselt mit `decrypt(words, direction)`; die Richtung je Archiv und Familie steht als Metadatum in `crates/g4-kernel/metadata/battle_sprites.yaml`, dort auch die Formen-Tabelle), Kernel-Modul `data/battle_sprites.rs` (Art × Vorder- und Rückseite × ♀ und ♂ × normal und schillernd, Formen, Trainerklasse → Member `5·k + 4`), die Befehle `get_battle_sprite` und `get_trainer_sprite` (PNG, Cache je Sitzung), das Feld `sprite_slots` in der Spezies-JSON, die DS-Varianten `DSPokedexProfile` und `DSBattlePreview` über `DSScreen` mit den neuen Szenen-Tokens `ds/status`, `ds/kp`, `ds/kampf`, die Pokédex-Schau mit „Spielerblick“ (Vorderseite, Rückseite, Weiblich, Schillernd) und den Trainer-Kopf mit dem Klassenbild und der „Kampfvorschau“. Roadmap: [docs/ROADMAP.md](ROADMAP.md), Abschnitt „Phase 3d“, Punkt „3d.3“. Spezifikation: [2026-10-30-phase-3d-erzaehlen-design.md](superpowers/specs/2026-10-30-phase-3d-erzaehlen-design.md), §4. Plan: [2026-10-30-phase-3d-3-sprites-ds-eier.md](superpowers/plans/2026-10-30-phase-3d-3-sprites-ds-eier.md). Formate: [ncgr_nclr.md](formats/ncgr_nclr.md) (Abschnitt „Kampfsprites“); Werkzeuge: [mcp.md](mcp.md) (`get_battle_sprite`, `get_trainer_sprite`). Formen am Augenschein: [task3-3d3-formen.md](research/2026-10-30-phase-3d/task3-3d3-formen.md).

Gate-Text der Spezifikation (§4, Zeile 3d.3):

> Gated: alle Sprites lesen (Platin 1 812 + 157, HGSS 1 812 + 161 Pokémon- und Formenbilder, 105 / 129 Trainerbilder), Mittel der zusammenhängenden Flächen linear ≤ 3, Ecken durchsichtig wie gemessen, ein Sprite ≤ 50 ms im Debug-Build über die Brücke; Pokédex zeigt das DS-Profil mit Vorderseite, Rückseite, ♀ und schillernd, Trainer-Kopf das Klassenbild, Kampfvorschau Trainer und erstes Team-Pokémon; Ei-Attacke in HeartGold ändern, bauen, `g4k check-data` findet sie; Platin: Tauschen innerhalb des Budgets, Hinzufügen ohne Entfernen abgelehnt mit Grund, mit `zcustom/egg_moves.narc` gilt der NARC; Blick Hell, Dunkel, Kompakt

Der erste Teil des Satzes (bis „Kampfvorschau Trainer und erstes Team-Pokémon“ und „Blick“) ist Teil A; die Ei-Attacken sind Teil B.

Wer was prüft: Die Spalte „Wer“ sagt **Agent** (die Test-Umgebung aus [docs/agent-gate.md](agent-gate.md): Browser-Fenster mit `studio-bridge`, `call.sh`, `mcp.sh`, dazu `g4k`, `studio mcp` kopflos und die gated Tests), **Agent + Blick** (der Agent prüft und beschreibt Ort und Zahlen, der Nutzer bestätigt kurz am Bild) oder **Hand** (der Nutzer). Befehle: [docs/agent-gate.md](agent-gate.md), Abschnitt „Phase 3d.3“.

Namen in der Oberfläche (Rollen und `data-*` für die Prüfung), Teil A:

- **Pokédex-Schau, Abschnitt „Spielerblick“** `[data-ds-profile-section]` (`<section aria-label="Spielerblick">`, über „Basiswerte“): `DSPokedexProfile` (`[data-ds-profile]`, Gruppe „Pokédex-Profil Staravia“) mit „Nr. 397“, dem Namen, den Typ-Plaketten, „Kategorie“, „Größe“ und „Gewicht“ je „folgt“ und der Textbox „Pokédex-Text folgt“; darunter `Segmented` „Seite“ (`[data-side]`: „Vorderseite“, „Rückseite“), Schalter „Weiblich“ (`[data-female]`) und „Schillernd“ (`[data-shiny]`). Gesperrt ist „Weiblich“ mit dem Satz „Kein eigenes Bild für ♀“ (Schalter aus) oder „Nur ♀“ (Schalter an); der Satz hängt über `aria-describedby` am Schalter. Ein Name, der nicht passt, endet mit „…“ und trägt den vollen Namen als `title`.
- **Trainer-Kopf und -Schau:** das Klassenbild `[data-trainer-sprite]` (80 px im Kopf der Schau, 48 px im Kopf der Hand); Abschnitt „Kampfvorschau“ `[data-ds-battle-section]` unter „Team“ mit `Segmented` „Gegnerseite“ (`[data-foe]`: „Trainer“, „Erstes Pokémon“; ohne Team nur „Trainer“) und `DSBattlePreview` (`[data-ds-battle]`, Boden `[data-ds-ground]`, Statuskästen `[data-ds-status=foe|own]`, KP-Balken `role="meter"` mit `data-hp="gruen|gelb|rot"`). „Trainer“ zeigt nur den Namen („Kampfvorschau: Teenager Gunther“), „Erstes Pokémon“ Name und „Lv. 5“ mit Balken; die eigene Seite ist leer (Boden ohne Bild und Kasten).
- **Galerie** (`?gallery`, Seite „Bildschirm“): die Blöcke „Pokédex-Profil“ und „Kampfvorschau“ mit Platzhaltern, ohne Spielbilder.
- **Brücke und MCP:** `get_battle_sprite {species, form?, side, female?, shiny?, frame?}` und `get_trainer_sprite {class, frame?}`; die Brücke liefert das PNG, `g4_call` und `g4_batch` antworten `{"bytes": <Länge>}` (Ruling R7); `data_get species <Art>` trägt `sprite_slots {female, male}`.

Teil B (folgt mit Teil B): Reiter `[data-tab="eggMoves"]`, `[data-egg-budget]`, die Sätze „Ei-Attacken-Tabelle voll: 1 910 von 1 910 Einträgen“ und „Ei-Attacken: Overlay 5 trägt an 0x2923E keine Tabelle (Signatur fehlt)“ und das Feld `egg_moves` der Spezies-JSON.

# Teil A

## Vorbereitung

So durchgeführt am 2026-10-31 auf dem Stand `a6f2d41`:

1. **Frische Kopien** unter `Roms/_testkopien/` per `cp -cR` (Ordner) und `cp -c` (`.nds`) aus den Originalen, die dabei nur gelesen wurden; Ordner `3d-3-<zweck>_DSPRE_contents` und `.nds` `3d-3-<zweck>.nds` direkt nebeneinander:
   - `3d-3-pt` (Platin DE, CPUD), `3d-3-hgde` (HeartGold DE, IPKD), `3d-3-hgus` (HeartGold US, IPKE) für (1); sie werden nur gelesen, die Tests schreiben auf `vanilla_copy()`;
   - `3d-3-web` (Platin DE) und `3d-3-hgweb` (HeartGold DE) für (3) und (2) über die Brücke (die Zeitmessung mit `curl` lief auf `3d-3-web`);
   - `3d-3-basept` und `3d-3-basehgde` (Namen `<stem>_DSPRE_contents`, die der gated Test von Task 3 für Basis und Neuaufbau braucht).

   Die Kopien für Teil B (`3d-3-kpt`, `3d-3-khgde` für das Szenario kopflos, `3d-3-patch` mit einem synthetischen `files/zcustom/egg_moves.narc`) kommen mit Teil B. Alle `3d-3-*`-Kopien bleiben bis zum Abschluss von Task 13 und werden danach gelöscht (`Manual` und `_saves` bleiben).
2. **Test-Umgebung** für (3): `G4_ROMS_DIR=<Roms> G4_VITE_PORT=1720 G4_BRIDGE_PORT=1721 bash scripts/agent/studio-web.sh` (Debug-Brücke). Der Port 1720 steht auf Chromes Liste unsicherer Ports (H.323): Chrome headless braucht `--explicitly-allowed-ports=1720,1721`, sonst kommt `ERR_UNSAFE_PORT`. Browser: Chrome headless (`--headless=new`, CDP, ein Node-Skript im Scratchpad), 1440 × 1500 (höher als 900, damit die Schau das ganze Profil über der Hand zeigt); Hell und Dunkel über `prefers-color-scheme`, Kompakt über die Taste der Statusleiste.
3. **Ausgaben** unter `target/agent/gate-2026-10-31/3d-3/` im Worktree (nicht im Repo): `gated-release.log`, `mcp-*.out` und `mcp-data_get-397.json`, `ui-*.log`, die Bilder `blick-*.png`, `galerie-*.png`, `pokedex-*.png`, `montage-*.png` und der Ordner `bridge-pt`. Den Ordner vor `git worktree remove` sichern.
4. **Last:** `uptime` im Lauf: 1-Minuten-Last 6 bis 12, weil andere Orchestratoren gleichzeitig arbeiteten (in einem Lauf des abschließenden Reviews bis 18, siehe Zahlentabelle).

## (1) Automatik

- [x] 1. `G4_TEST_PROJECT=<Kopie> cargo test --release -p g4-kernel --all-features --test battle_sprites -- --nocapture` auf `3d-3-{pt,hgde,hgus}`: je 23 bestanden, 1 ignoriert (der Helfer, der Bilder zum Ansehen außerhalb des Repos schreibt). Die gedruckten Zeilen:
   - Platin DE (CPUD): `pokemon=1812 forms=157 trainers=105 components=1.4 corners=1778 palettes16=988 ms=3723`, `trainer_components=1.7`, Formen `images=157 palettes=96 components=1.1 entries=77 rendered=306 form0=12 form0_like_pokegra=[201, 351, 479, 487] female_only=3`.
   - HeartGold DE (IPKD): `pokemon=1812 forms=161 trainers=129 components=1.3 corners=1778 palettes16=988 ms=3204`, `trainer_components=1.9`, Formen `images=161 palettes=100 components=1.1 entries=79 rendered=314 form0=13 female_only=3`.
   - HeartGold US (IPKE): `pokemon=1812 forms=161 trainers=129 components=1.3 corners=1778 palettes16=988 ms=2664`, `trainer_components=1.9`, Formen wie HeartGold DE.
2. `vanilla_every_battle_sprite_decodes_with_the_measured_counts` und `vanilla_every_form_sprite_decodes` sind die zwei gated Tests dazu (`crates/g4-kernel/tests/battle_sprites.rs`); sie prüfen die gemessenen Zahlen (Bilder, Formen und Trainer als Gleichheit, `palettes16` als Gleichheit, das Mittel der zusammenhängenden Flächen höchstens 3, `corners` mindestens 1 778) und, dass sich jedes Formbild von Form 0 unterscheidet. `form0_like_pokegra=[201, 351, 479, 487]` ist der gemessene Satz der Formarten, deren Form 0 aussieht wie der Eintrag in `pokemonBattleSprites` (Icognito, Formeo, Rotom, Giratina); er gilt in allen drei Spielen.
3. Studio-Test im **Debug**-Build: `G4_TIMING_STRICT=1 G4_TEST_PROJECT=<Kopie> cargo test -p studio --all-features --lib battle_sprites_over_the_command_path -- --nocapture` (`battle_sprites_over_the_command_path_on_the_vanilla_copy`, `apps/studio/src-tauri/src/commands/data/tests.rs`): Platin DE `first_ms=12 cached_ms=0 trainer_ms=5`, HeartGold DE `first_ms=8 cached_ms=0 trainer_ms=6`; beide bestanden, die Grenze ist 50 ms je erste Anfrage.
4. Aus Task 3 (selber Branch, früher): die gated Build-Tests `pack_only_entries_repack_byte_identical`, `vanilla_rebuild_matches_dsrom_cli` und `base_assign_on_the_vanilla_folder_has_no_warning_and_rebuilds_the_nds` auf Platin DE und HeartGold DE grün (die Sprite-Archive wanderten aus `pack_only` in benannte Arten).
5. **Nicht gelaufen:** `scripts/gate.sh` als Ganzes. Die Hauptsitzung führt es nach dieser Welle auf main aus; die neuen gated Tests laufen dort im Schritt der gated Tests je Projekt (Debug) mit, das Skript braucht dafür nichts Neues. Die gated Tests für die Ei-Attacken und für die Textprojekte aus Task 11 folgen mit Teil B.

## (2) Kopflos und MCP

- [x] 1. MCP über die Brücke (`scripts/agent/mcp.sh`, lokale Agenten an, Expertenwerkzeuge an, Freigabe „Lesen“): `g4_call get_battle_sprite {"species":"SPECIES_STARAVIA","side":"front"}` antwortet `{"result":{"bytes":1496}}`; `g4_call get_trainer_sprite {"class":1}` `{"bytes":1691}`; `frame:2` den Fehler „frame must be 0 or 1“; die Art 600 `{"bytes":0}`. `data_get species SPECIES_STARAVIA` trägt `sprite_slots {female:true, male:true}`, die Art 500 `{female:true, male:false}`.
- [x] 2. **Base64 (Ruling R7):** der Plan nahm an, `g4_call` liefere Base64 eines PNG. Gemessen und in [docs/mcp.md](mcp.md) beschrieben: `g4_call` und `g4_batch` antworten für Bildbefehle wie für `get_mon_icon` mit `{"bytes": <Länge>}`; nur die Brücke gibt die PNG-Bytes zurück (`call.sh` druckt dafür „<n> Bytes“). Ein Bildwerkzeug für Agenten bleibt offen (Abschnitt „Offen“).
- [x] 3. Zeit über die Brücke (Debug-Brücke, Kopie `3d-3-web`, `curl`, erste Anfrage je Bild): Art 397 vorn über `call.sh`; 25 vorn 15 ms (wieder 2 ms), 6 hinten 13 ms, 150 schillernd 12 ms, 494 (Ei) 8 ms, 421 Form 1 11 ms, 413 schillernd 10 ms, 412 schillernd 11 ms, 407 ♀ 16 ms, 3 ♀ 16 ms, Trainerklasse 5 14 ms, Klasse 104 11 ms; Klasse 105 und Art 600 liefern einen leeren Körper (0 B). Alle höchstens 16 ms, die Grenze ist 50 ms.

Das Szenario kopflos für die Ei-Attacken (`data_get` und `data_edit` auf `egg_moves`, `g4k build`, `g4k check-data`, Platin mit Budget und Patch) folgt mit Teil B.

## (3) Browser

Test-Umgebung auf 1720/1721, Kopien `3d-3-web` (Platin DE) und `3d-3-hgweb` (HeartGold DE), Chrome headless wie in „Vorbereitung“.

- [x] 1. Pokédex Staravia (397), Platin DE und HeartGold DE: der Abschnitt „Spielerblick“ mit `DSPokedexProfile` („Nr. 397“, STARAVIA, Normal und Flug, „Kategorie“, „Größe“ und „Gewicht“ je „folgt“, „Pokédex-Text folgt“); „Vorderseite“ und „Rückseite“ wechseln das Bild (`data-side` front, back), „Schillernd“ zeigt die schillernde Palette (`data-shiny`), „Weiblich“ ist für Staravia frei (eigenes ♀-Bild) und zeigt es; kein waagrechtes Rollen (`scrollWidth` 1440).
- [x] 2. ♀-Sperre: 413 Burmadame und 500 Burmadame (Lumpenumhang) „Nur ♀“ (♀ an und gesperrt, `aria-describedby` zeigt auf den Satz); 421 Kinoso, 412 Burmy, 503 Hitze-Rotom, 494 Ei und in HeartGold 172 Pichu „Kein eigenes Bild für ♀“; 387 Chelast und 25 Pikachu frei. Kinoso (421) zeigt nach dem Fix des abschließenden Reviews die Wolkenform.
- [x] 3. ⌘1 aus Trainer wechselt in den Pokédex wie bisher.
- [x] 4. Trainer 1: Platin „Teenager Gunther“ (Team STARALILI Lv. 5), HeartGold „Rivale Silber“ (Team NEBULAK Lv. 14): Klassenbild 80 im Kopf der Schau (`[data-trainer-sprite]`), 48 im Kopf der Hand; Kampfvorschau mit Gegnerseite „Trainer | Erstes Pokémon“ (`data-foe`); „Trainer“: Kasten nur mit dem Namen („Kampfvorschau: Teenager Gunther“); „Erstes Pokémon“: „STARALILI“ und „Lv. 5“, KP-Balken `meter` 100 `gruen`; die eigene Seite leer (nur Boden). Die HeartGold-Trainerbilder entschlüsseln rückwärts richtig (Silber ist zu erkennen).
- [x] 5. Galerie (`?gallery`, Seite „Bildschirm“): Pokédex-Profil und Kampfvorschau mit Platzhaltern.
- [x] 6. Blick Hell, Dunkel, Kompakt: Bilder unter `target/agent/gate-2026-10-31/3d-3/`: `blick-pokedex-profil-{vorn,hinten,schillernd}-{hell,dunkel,kompakt}.png` und `blick-pokedex-profil-weiblich-hell.png` (HeartGold mit dem Vorsatz `blick-pokedex-profil-hg-`), `blick-trainer-{kopf,kampfvorschau-trainer,kampfvorschau-pokemon}-{hell,dunkel,kompakt}.png` (HeartGold `-hg-`), `galerie-{profil,kampf}-{hell,dunkel,kompakt}.png`, `pokedex-pt-{387,25,413,421,412,500,503,494}.png` und `pokedex-hg-{172,413,421,500,503}.png`. Der Pedestal (`status/rahmen`, graue Ellipse) ist auf dem dunklen DS-Grund in Hell und Dunkel zu sehen; der Text in den mit `transform` vergrößerten Plaketten ist bei Maßstab 2 in Chrome scharf.

## (4) Hand und Blick

Beim Nutzer (Abschnitt „Hand“ und „Blick“ unten und die Tabelle „Ergebnis“):

- **Hand:** die Bilder gegen das Spiel im Emulator (Pokédex und Kampf), vor allem Kinoso Wolken- und Sonnenform, Burmy und Burmadame schillernd, Pichu (HeartGold, Form 0, siehe „Offen“) und die HeartGold-Trainerbilder.
- **Blick:** die Bilder unter `target/agent/gate-2026-10-31/3d-3/` (Profil, Trainer, Galerie; U2 und U3).
- **Formen am Augenschein:** [task3-3d3-formen.md](research/2026-10-30-phase-3d/task3-3d3-formen.md).
- **`.fig`:** die Frames für `DSPokedexProfile` und `DSBattlePreview` folgen (OpenPencil war nicht erreichbar; `design-system/README.md`, „Nachtrag 3d.3“).

## Ergebnis

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
| Teil B (Ei-Attacken) | – | wartet auf 3d.1 auf main (Abschnitt „Teil B“) | – |

Zahlentabelle (Teil A; Release, Werte aus den gated Tests (1), in Klammern aus (2)):

| Spiel | Pokémon-Bilder | Formen | Trainer | Flächen (Mittel) | Ecken | erstes Sprite (ms) | Ei-Attacken Arten/Attacken | Last |
|---|---|---|---|---|---|---|---|---|
| Platin DE | 1 812 | 157 Bilder, 96 Paletten, 77 Einträge | 105 | 1,4 Pokémon, 1,7 Trainer, Formen 1,1 | 1 778 von 1 812 | 12 Debug-Studio-Test (Brücke 8 bis 16) | — (Teil B) | 6 bis 12 (CPUD-Lauf des Reviews bis 18, `ms=16811` statt 3 723) |
| HeartGold DE | 1 812 | 161 Bilder, 100 Paletten, 79 Einträge | 129 | 1,3 Pokémon, 1,9 Trainer, Formen 1,1 | 1 778 von 1 812 | 8 Debug-Studio-Test (Brücke 8 bis 16) | — (Teil B) | 6 bis 12 |
| HeartGold US | 1 812 | 161 Bilder, 100 Paletten, 79 Einträge | 129 | 1,3 Pokémon, 1,9 Trainer, Formen 1,1 | 1 778 von 1 812 | nicht einzeln gemessen (Studio-Test nur auf Platin DE und HeartGold DE) | — (Teil B) | 6 bis 12 |

Zeiten der gated Läufe (`ms=` im Test, Release): Platin DE 3 723, HeartGold DE 3 204, HeartGold US 2 664 für alle 1 812 Pokémon-Bilder und die Trainer; unter Last im Lauf des abschließenden Reviews Platin DE 16 811, HeartGold DE 3 765, HeartGold US 3 547 (Last um 18). Eine Grenze setzt der Test dafür nicht.

## Befunde

### UI-Test (2026-10-31)

| Nr. | Schwere | Befund | Fix |
|---|---|---|---|
| U1 | Klein | Lange Formnamen („Burmadame, Lumpenumhang“) waren im Profil hart abgeschnitten. | `a6f2d41` (Review sauber, im UI nachgeprüft): Namen im Profil und in den Statuskästen enden mit „…“ und tragen den vollen Namen als `title`; je ein Testfall je Baustein. |
| U2 | Klein (Blick) | In Dunkel ist der eigene Grund des DS-Bildschirms (`ds/grund`) fast der Grund der App; die Kante des Bildschirms verschwindet fast (der vorhandene Rahmen von `DSScreen`, nicht neu in 3d.3). | Keine Änderung; der Nutzer entscheidet am Bild. |
| U3 | Klein (Blick) | Kleine Arten stehen in der Kampfvorschau über dem Boden (feste Lage ohne `pokemonSpriteOffsets`, Planentscheidung „feste Lage“). | Keine Änderung; Bilder `blick-trainer-kampfvorschau-pokemon-*.png`. |

### Abschließendes Review von Teil A

Das Review des ganzen Teils (Opus) fand einen wichtigen Punkt und mehrere kleine; eine Fix-Runde (`476f0e3`, `6331244`), danach sauber.

| Nr. | Schwere | Befund | Fix |
|---|---|---|---|
| 1 | Wichtig | Form 0 der Arten mit Formen kam aus `pokemonBattleSprites`; das Spiel zeichnet sie nur aus `otherPokemonBattleSprites`: Kinoso zeigte die Sonnenform für die Wolkenform, Burmy, Burmadame, Schalellos und Gastrodon schillernd gleich normal, Burmadames Palette weicht in 13 von 16 Farben ab. | `6331244` (Ruling R8): Einträge für Form 0 jeder Art mit Formen (Platin 12, HGSS 13), gemessen an `3d-3-pt` und `3d-3-hgde`, am Augenschein bestätigt; Test `form_zero_of_a_species_with_forms_comes_from_the_other_archive`, der gated Test vergleicht die Form-0-Bilder mit `pokegra` (`form0_like_pokegra=[201, 351, 479, 487]`). |
| 2 | Klein | Mit Form 0 aus dem zweiten Archiv hätte Burmadame („Nur ♀“) die Sperre „Kein eigenes Bild für ♀“ bekommen. | `6331244` (Rulings R9 und R10): `sprite_slots` nimmt die Geschlechterverteilung; 254 macht ein einziges Bild zum weiblichen; die Regel gilt für jede Art (in Vanilla ändert sie dort nichts: die 16 rein weiblichen Arten liegen in `pokegra` schon in den weiblichen Plätzen). Tests im Kernel und im Studio, gated `female_only=3`. |
| 5 | Klein | Die Sätze zu den Bildbefehlen in `docs/mcp.md` trennten einen Absatz. | `476f0e3`. |
| 3, 4, 6, 7, 8 | Klein | Dekodieren eines ersten Bildes unter der Sitzungssperre (8 bis 15 ms); `data_get species` liest das `pokegra`-NARC je Aufruf bei gepackten Projekten (5 bis 7 ms); „Nr.“ zeigt für Formen 494 bis 507 den Personal-Member; Sprite 48 ist nicht ganzzahlig skaliert (vom Plan vorgegeben); `TileMode` ist streng (Plan-Entscheidung). | Geparkt (Plan- und Bestandsentscheidungen, nicht tragend; Abschnitt „Offen“). |

## Offen

Ohne Einfluss auf Teil A; was ein Nutzer bemerken kann:

- **Pichu (HeartGold), Form 0 (Ruling R11):** der Eintrag kommt wie die anderen Form-0-Arten aus `otherPokemonBattleSprites` (Member 154 und 155), ohne dass gemessen ist, ob HeartGold Pichu dort zeichnet. Ein Eintrag in `battle_sprites.yaml`; der Nutzer prüft ihn im Emulator.
- **Zukan-Daten:** Pokédex-Text, Kategorie, Größe und Gewicht bleiben „folgt“, bis eine Messung die Bänke und Tabellen belegt (eigener kleiner Posten, nicht 3d).
- **Feste Lage:** `pokemonSpriteOffsets` (494 × 89 Byte) sind ungeprüft; die Kampfvorschau setzt jedes Bild an dieselbe Stelle (U3). Der Boden ist stilisiert, kein Kampfhintergrund; die eigene Seite der Kampfvorschau bleibt leer.
- **Bildwerkzeug für Agenten:** `g4_call` liefert nur die Länge (R7); ein MCP-Werkzeug, das ein PNG ausgibt, wäre ein eigener Posten.
- **Diamant und Perl:** ohne Tabelle und ohne Richtung (Familie ungemessen); `get_battle_sprite` liefert dort nichts.
- **Geparkt aus den Reviews:** das Dekodieren eines ersten Bildes hält die Sitzungssperre (8 bis 15 ms); `data_get species` parst das `pokegra`-NARC je Aufruf bei gepackten Projekten; ein Fehler eines kaputten Archivs wird als „kein Bild“ gemeldet und nicht gemerkt; „Nr.“ zeigt für Formen 494 bis 507 den Personal-Member (`SpeciesRowExtra`); die Pokédex-Textbox schneidet nach zwei Zeilen ohne Hinweis ab (heute „folgt“); die KP-Farbe und die Prozentzahl können an den Schwellen auseinandergehen (gerundet gegen ungerundet); `level` wird nicht normiert („Lv. 12.5“).
- **Beim Nutzer:** Hand (Bilder gegen das Spiel), Blick (U2, U3), `.fig` (Frames).

# Teil B

Teil B (Ei-Attacken: Plan-Tasks 8 bis 12) wartet, bis 3d.1 auf main ist: die Ei-Attacken brauchen dessen Stand als Quelle für `Target::Move` und die Textprojekt-Dateien. Prüfung am 2026-10-31: 3d.1 ist nicht auf main (`git grep -l flags.json main -- crates/g4-kernel/src/textproject` leer, main `c6345d6`). Die Punkte stehen offen und kommen mit Teil B in dieses Dokument:

- (1) Automatik: `scripts/gate.sh` grün; einzeln je Projekt im Release-Profil `vanilla_egg_moves_read_with_the_measured_counts` (Zahlen Arten/Attacken je Spiel, in die Zahlentabelle), der Studio-Test mit den Ei-Attacken-Zeilen und die gated Textprojekt-Tests aus Task 11. Offen.
- (2) Szenario kopflos: HeartGold-Kopie `studio mcp --project`, `data_get species 1` (`egg_moves.source narc`), `data_edit species 1 { EggMoves … }` (Tausch), `save`, `g4k build`, `g4k check-data <rom> --expect egg_moves:1:count=<n> --expect egg_moves:1:move0=<neu>`; Platin-Kopie: `data_get` (`source overlay`, Budget 1 910 von 1 910), Tausch ok, Hinzufügen ohne Entfernen Fehler mit dem Satz; `3d-3-patch` (Platin mit einem synthetischen `files/zcustom/egg_moves.narc`, der den HGSS-Member-Aufbau trägt): `source patch`, `ov005.bin` byte-gleich nach `save`. Offen.
- (3) Browser: Reiter „Ei-Attacken“ auf Platin (Budget, Ablehnung) und HeartGold (Hinzufügen geht), ⌘Z, die Bilder von Task 12. Offen.
- (4) Hand beim Nutzer: Platin-Ei-Attacke im Spiel wirksam (Zucht). Offen.
- Die Spalte „Ei-Attacken Arten/Attacken“ der Zahlentabelle und die Rulings und Befunde zu Task 8 bis 12.
