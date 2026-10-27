# Phase 4.0: Gate prüfen

Stand: 2026-10-27. Branch `phase-4-0-nebenaufgaben` (Worktree `.claude/worktrees/phase-4-0`), nicht nach main gemergt. Abnahme der Punkte (G1) bis (G4) über die Test-Umgebung auf dem Stand `db4ed8a` (danach nur die Breite von Start in `af493ba` und die Befunde der Abschlussprüfung, `5842bc0` bis `c9f9540`); `scripts/gate.sh`: grün auf `87cf829` (2026-10-27, 4153 s; danach nur Dokumente und die Vorlagen des Design-Systems). Beim Nutzer offen: (G5) und die Hand-Punkte (gepackte App) sowie die Blicke, Abschnitt „Beim Nutzer offen“.

Phase 4.0 bringt die zwei Nebenaufgaben des Nutzers vom 2026-10-26: eine eigene Marke als App-Symbol und Vorschaubilder der Projekte auf Start. Roadmap: [docs/ROADMAP.md](ROADMAP.md), Abschnitt „Phase 4“, Punkt „4.0 Nebenaufgaben“. Spezifikation: [2026-10-27-phase-4-0-nebenaufgaben-design.md](superpowers/specs/2026-10-27-phase-4-0-nebenaufgaben-design.md). Plan: [2026-10-27-phase-4-0-nebenaufgaben.md](superpowers/plans/2026-10-27-phase-4-0-nebenaufgaben.md).

## Lieferumfang

- **Marke** (`44fca7d`, `43ae1bf`, `e7235b9`): ein klares, minimales Vektor-Symbol nach dem Wunsch des Nutzers vom 2026-10-27 („Clean und Minimalistic Vector Design“): fünf flache Formen in drei Token-Farben (Körper `scharnier/grund`, zwei Bildschirme `szene/ds/grund`, Scharnierlinie und Auswahlquadrat `akzent/grund`). Quelle `design-system/marke/gen4-studio-marke.svg` und `-1024.png`, erzeugt von `design-system/scripts/marke.py` (Tests `test_marke.py`); alle Größen unter `apps/studio/src-tauri/icons/` aus `pnpm tauri icon`; `apps/studio/scripts/check-bundle.sh` liest den Namen des Symbols aus `Info.plist` und vergleicht seinen SHA-256 mit `icons/icon.icns`, das alte Standardsymbol wird abgelehnt. Beschrieben in `design-system/README.md` (Abschnitt „Marke“) und `docs/release.md` („App-Symbol“); Brett „Marke“ auf der Seite „Grundlagen“ der `.fig` (`18140a7`).
- **Kernel** (`b13a432`, `0dd3ffa`, `8effd7d`): `crates/g4-kernel/src/preview.rs` liest, passt ein und schreibt `.g4/preview.png` (eigen) und `.g4/cache/preview-standard.png` (Standard), 256 × 192 PNG RGB8; PNG und JPEG am Inhalt erkannt (auch CMYK, EXIF-Drehung), Zuschnitt 4 : 3 mittig, Grenzen 32 MiB und 8192 × 8192, Codes `preview.format`, `preview.too_large`, `preview.unreadable`; `standard_map` nennt die Startstadt aus dem Rezept (Platin T01 Zweiblattdorf, HGSS T20 Neuborkia).
- **Befehle** (`dbcacd1`, `59591cc`, `5842bc0`): `project_preview_get` und `project_preview_set` in Tauri, Brücke und MCP (Zugriff `Read` bzw. `Irreversible("Vorschaubild ändern")`, über MCP ohne `path`, das Bild als MCP-Bildblock statt Base64 im Text).
- **Start** (`e25b547`, `af493ba`, `0bf0f5a`, `c9f9540`): die Projektkarten zeigen das Bild im DS-Rahmen, Menü „Vorschaubild wählen…“, „Vorschaubild entfernen“, „Aus der Liste nehmen“; die Liste nimmt die volle Breite (drei Karten nebeneinander bei 1280 px).
- **Technik › Projekt › Vorschaubild** (`59e0d3f`) mit „Vorschaubild wählen…“, „Entfernen“ und dem Satz zur Startstadt.
- **Szene** (`69dda2e`, `db4ed8a`, `6285c4f`, `70af1d2`): „Als Vorschaubild“ im Reitermenü von „3D“ und „Weltkarte“ und in der Palette; das Standardbild entsteht nach dem Öffnen im Leerlauf und wird nur bei Abweichung neu geschrieben.
- **Design-System**: README „Projektkarte“, „Nachtrag 4.0“, Galerie „Projektstart“ mit Projektkarte und „Technik: Vorschaubild“ (`598720e`, `87cf829`); Brett „Marke“ in der `.fig`; die Vorlagen für Teil B der `.fig` (Start mit Bildern, Projektkarte, Technik › Projekt) mit den Bausteinen in `jsx-lib-app.js`, noch nicht gerendert.

## Punkte

- (G1) Auf einer frischen Platin-DE-Kopie: „Als Vorschaubild“ aus der 3D-Ansicht, aus dem Kartenbild (Palette) und aus dem Reitermenü der Weltkarte; Start zeigt das Bild.
- (G2) „Vorschaubild wählen…“ mit PNG und JPEG auf Start und in Technik, eingepasst auf 256 × 192; eine Textdatei als `.png` gibt den Satz für `preview.format`.
- (G3) „Vorschaubild entfernen“: Start zeigt das Kartenbild der Startstadt (Platin Zweiblattdorf, HeartGold Neuborkia).
- (G4) Ohne Bild der Name auf dem leeren Bildschirm, „nicht gefunden“ und „Zuletzt“ wie bisher.
- (G5) `check-bundle.sh` nach `pnpm bundle` meldet `bundle ok` und erkennt die neue `icon.icns`.
- Blick: Start in Hell, Dunkel und Kompakt; die Marke in der `.fig` und in `tauri dev`.
- Hand: Dock, Finder und „Über Gen4 Studio“ der gepackten App zeigen die Marke.

## Vorbereitung

1. Frische Kopien (nie ein Original): `Roms/_testkopien/4-0-gate-pt` (Platin DE), `4-0-gate-hg` (HeartGold DE), `4-0-gate-pt-neu` (Platin DE, für den Platzhalter).
2. Test-Umgebung aus dem Worktree auf eigenen Ports: `G4_ROMS_DIR=/Users/POLTEP/Projekte/lab/editor/Roms G4_VITE_PORT=1490 G4_BRIDGE_PORT=1491 bash scripts/agent/studio-web.sh` ([docs/agent-gate.md](agent-gate.md)), Seite `http://localhost:1490`.
3. Testbilder (Python-Standardbibliothek und `sips`, keine Spielgrafik) unter `Roms/_testkopien/4-0-gate-bilder/`: `bild-a-640x480.png`, `bild-b-1200x900.jpg`, `kein-bild.png` (Textdatei).

## Ergebnis

Bilder unter `target/agent/gate-2026-10-27/` im Worktree (Bildschirmfotos verkleinert auf 800 px Breite; `datei-*.png` sind Kopien der Projektdateien zum jeweiligen Zeitpunkt). Ausführlich: Abnahmebericht der Test-Umgebung vom 2026-10-26/27.

| Punkt | Wer | Ergebnis | Belege |
|---|---|---|---|
| (G1) | Agent | bestanden: 3D, Palette (Karte 0018) und Reitermenü Weltkarte (Karte 0016) schreiben je `.g4/preview.png` 256 × 192 RGB, Start zeigt es; ohne Karte gesperrt mit „Erst eine Karte wählen“, per Tastatur erreichbar | `g1-*.png`, `datei-pt-preview-3d.png`, `datei-pt-preview-karte-0018-palette.png`, `datei-pt-preview-karte-0016-weltkarte.png` |
| (G2) | Agent | bestanden: PNG 640 × 480 und JPEG 1200 × 900 über Karte und Technik, eingepasst; Textdatei: „Das Bild ist weder PNG noch JPEG. Wähle eine PNG- oder JPEG-Datei.“, Datei unverändert | `g2-*.png`, `datei-pt-preview-png.png`, `datei-pt-preview-jpeg.png` |
| (G3) | Agent | bestanden: Entfernen über Technik und Karte, Start zeigt Zweiblattdorf, „Vorschaubild entfernen“ verschwindet; HeartGold zeigt Neuborkia (Header 60, Karte 0, AreaData 2); erneutes Öffnen schreibt das gleiche Standardbild nicht neu | `g3-*.png`, `datei-pt-preview-standard.png`, `datei-hg-preview-standard.png` |
| (G4) | Agent | bestanden mit Anmerkung: der Platzhalter (Name) erscheint, wenn kein Standardbild da ist; da es gleich nach dem Öffnen entsteht, wurde er mit weggelegtem Standardbild geprüft. „nicht gefunden“ erscheint wie vor 4.0 nach einem fehlgeschlagenen Öffnen, mit nur „Aus der Liste nehmen“. „Zuletzt“ sitzt auf der neuesten Karte | `g4-*.png` |
| 3D-Aufnahme nach `6285c4f` | Agent | bestanden: frische Kopie `4-0-aufnahme-pt`, Zweiblattdorf in 3D, Palette „Als Vorschaubild: 3D-Ansicht“, Meldung „Vorschaubild gesetzt“; `.g4/preview.png` ist 256 × 192 RGB ohne Alpha und zeigt die Ansicht, Start zeigt es auf der Karte | `aufnahme-3d-toast-gesetzt.png`, `aufnahme-start-nach-3d.png`, `datei-pt-preview-3d-512.png` |
| Tastatur | Agent | bestanden: Kartenmenü per Tastatur, gesperrter Eintrag im Reitermenü per Pfeil erreichbar und nie ausgeführt, Enter im Pfaddialog öffnet das Menü nicht erneut | `tastatur-*.png` |
| Automatik | Agent | bestanden: `scripts/gate.sh` auf `87cf829` grün in 4153 s, alle 17 Schritte ok (fmt, clippy, Tests, MCP, vitest, Tokens, Stile, Build, g4emu, gated Tests auf Platin DE, HeartGold DE und HeartGold US, Projektstart aus den drei ROMs) | `target/agent/gate-2026-10-27/gate-sh.log` |
| (G5) | Hand | offen | – |
| Blick Start | Blick | offen (Bilder liegen bereit) | `blick-start-hell.png`, `blick-start-dunkel.png`, `blick-start-kompakt-hell.png`, `blick-technik-projekt-hell.png`, `blick-galerie-projektstart-*.png` |
| Blick Marke | Blick | offen (Brett liegt bereit) | `fig-marke.png`, `design-system/marke/gen4-studio-marke-1024.png` |
| `.fig` Teil B | Agent | offen: Bildschirm gesperrt; Vorlagen gebaut und als JSX geprüft, nicht gerendert | `design-system/openpencil/NACHTRAG-4-0.md` |
| Hand Marke | Hand | offen | – |

Unit- und Integrationstests der Phase: Kernel `preview` 25 Tests (darunter CMYK und EXIF), gated `standard_map_names_the_start_towns` auf frischen Kopien von Platin DE und HeartGold DE; `studio` 504 Tests, `mcp_stdio` 27; vitest 2985 bestanden (1 übersprungen); `test_marke.py` 5 Tests.

## Beim Nutzer offen

**Hand**

- [ ] (G5) In `apps/studio`: `pnpm bundle`, dann aus der Repo-Wurzel `bash apps/studio/scripts/check-bundle.sh`. Erwartet: `bundle ok: …`. Ein Fehlschlag nennt die Bedingung („bundle icon.icns differs …“ oder „… is still the create-tauri-app default“).
- [ ] Marke in der gepackten App (`target/release/bundle/macos/Gen4 Studio.app`): Dock (hell und dunkel), Finder (Symbol- und Listenansicht), Menü „Gen4 Studio“ › „Über Gen4 Studio“ zeigen die neue Marke, nicht die zwei Ringe von create-tauri-app. Falls macOS das alte Symbol aus dem Cache zeigt: die App einmal an einen anderen Ort kopieren oder `killall Dock`.

**Blick**

- [ ] Start mit Bildern in Hell, Dunkel und Kompakt: `blick-start-hell.png`, `blick-start-dunkel.png`, `blick-start-kompakt-hell.png`; Technik › Projekt: `blick-technik-projekt-hell.png`; Galerie „Projektstart“: `blick-galerie-projektstart-dunkel.png`.
- [ ] Die Marke: Brett „Marke“ in der `.fig` (Seite „Grundlagen“, x 1720, y 2900; Bild `fig-marke.png`) und das Symbol im Dock von `tauri dev` (`cd apps/studio && pnpm tauri dev`).
- [ ] Teil B der `.fig` (Agent, bei entsperrtem Bildschirm): Start mit Bildern, Komponente „Projektkarte“ und Technik › Projekt fehlen noch in der Datei. OpenPencil antwortete bei gesperrtem Bildschirm nicht (`RPC timeout`). Die Vorlagen `start-vorschau`, `projektkarte` und `technik-projekt` liegen unter `design-system/openpencil/beispiele/` und ergeben mit `build-screen.sh` gültiges JSX; Orte und Inhalt in `design-system/openpencil/NACHTRAG-4-0.md`, Teil B. Danach `fix_fig_text.mjs --rgb555` und der Blick auf die drei Screens.

## Bekannte Grenzen und Folgepunkte

- `.g4/cache/` ist noch nicht von Git ausgenommen; die Erlaubnisliste für `.g4/` kommt mit Phase 4 (Gutachten C zum Projektformat). Jedes Öffnen schreibt `.g4/cache/preview-standard.png` in das Projekt.
- „nicht gefunden“ erscheint wie vor 4.0 erst nach einem fehlgeschlagenen Öffnen und geht nach einem Neuladen verloren.
- Fehlertexte: ein nicht zugeordneter Fehler beim Setzen (etwa fehlendes apicula beim Kartenbild) zeigt den allgemeinen Satz „Das Vorschaubild ließ sich nicht speichern …“; im Browser-Modus ersetzen einige deutsche Ablehnungen der Brücke durch denselben Satz; Fehler des Browsers oder von three.js erscheinen roh.
- Im Reitermenü steht der Grund einer Sperre nur als Tooltip (für Screenreader über `aria-describedby`), in der Palette sichtbar.
- `g4_call` und `g4_batch` geben die Vorschau-Befehle roh mit Base64 zurück (Expertenweg); die kuratierten Werkzeuge liefern das Bild als Bildblock.
- Der Katalog der MCP-Werkzeuge hat 45 Werkzeuge, die obere Grenze von `tool_count_is_about_thirty`; das nächste Werkzeug muss die Grenze anheben.
- `test_marke.py` läuft nicht in `gate.sh`; nach einer Token-Änderung `python3 design-system/scripts/marke.py` und `pnpm tauri icon` von Hand.
- Ein Bild aus der Szene zeigt die Ansicht, wie sie ist, mit Auswahlrahmen.
