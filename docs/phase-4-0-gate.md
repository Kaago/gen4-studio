# Phase 4.0: Gate prüfen

Stand: 2026-10-27. Branch `phase-4-0-nebenaufgaben` (Worktree `.claude/worktrees/phase-4-0`), nicht nach main gemergt. Abnahme der Punkte (G1) bis (G4) über die Test-Umgebung auf dem Stand `db4ed8a` (danach nur die Breite von Start in `af493ba` und die Befunde der Abschlussprüfung, `5842bc0` bis `c9f9540`); `scripts/gate.sh`: grün auf `87cf829` (2026-10-27, 4153 s; danach nur Dokumente und die Vorlagen des Design-Systems). Unabhängige Abnahme am 2026-10-27 auf `0f32930` mit drei Befunden, behoben in `2b503a4`, `1dfcb70` und `2bfbf58` (Abschnitt „Abnahme (2026-10-27)“); (G5) dabei vom Agenten bestanden. Beim Nutzer offen: die Hand-Punkte der gepackten App und die Blicke, Abschnitt „Beim Nutzer offen“; offen für einen Agenten: Teil B der `.fig` (Bildschirm gesperrt).

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
2. Test-Umgebung aus dem Worktree auf eigenen Ports: `G4_ROMS_DIR=$HOME/Projekte/lab/editor/Roms G4_VITE_PORT=1490 G4_BRIDGE_PORT=1491 bash scripts/agent/studio-web.sh` ([docs/agent-gate.md](agent-gate.md)), Seite `http://localhost:1490`.
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
| (G5) | Agent | bestanden in der Abnahme: `pnpm bundle`, dann `check-bundle.sh` meldet `bundle ok`; das Symbol im Bundle hat den Hash von `icons/icon.icns` | Abschnitt „Abnahme (2026-10-27)“ |
| Blick Start | Blick | offen (Bilder liegen bereit, dazu die der Abnahme) | `blick-start-hell.png`, `blick-start-dunkel.png`, `blick-start-kompakt-hell.png`, `blick-technik-projekt-hell.png`, `blick-galerie-projektstart-*.png` |
| Blick Marke | Blick | offen (Brett liegt bereit) | `fig-marke.png`, `design-system/marke/gen4-studio-marke-1024.png` |
| `.fig` Teil B | Agent | offen: Bildschirm gesperrt, auch in der Abnahme (`CGSSessionScreenIsLocked`, OpenPencil `RPC timeout` bis zum Ende des Laufs); Vorlagen gebaut und als JSX geprüft, nicht gerendert | `design-system/openpencil/NACHTRAG-4-0.md` |
| Hand Marke | Hand | offen | – |

Unit- und Integrationstests der Phase: Kernel `preview` 25 Tests (darunter CMYK und EXIF), gated `standard_map_names_the_start_towns` auf frischen Kopien von Platin DE und HeartGold DE; `studio` 504 Tests, `mcp_stdio` 27; vitest 2985 bestanden (1 übersprungen); `test_marke.py` 5 Tests.

## Abnahme (2026-10-27)

Unabhängige Abnahme durch eine zweite Sitzung auf dem Stand `0f32930`, nach den Punkten oben und den Wegen von [docs/agent-gate.md](agent-gate.md). Frische Kopien unter `Roms/_testkopien/`: `4-0-abn-pt` (Platin DE, G1 bis G3), `4-0-abn-hg` (HeartGold DE, G2 über Technik, G3), `4-0-abn-pt-leer` (Platin DE, ohne Bild: das Standardbild nach dem Öffnen weggelegt, dann eigenes Bild gesetzt und wieder entfernt) und `4-0-abn-pt-weg` (Platin DE, nach dem Öffnen umbenannt: „nicht gefunden“). Testbilder aus der Python-Standardbibliothek und `sips` in `4-0-abn-bilder/`: PNG 800 × 600 (vier Farbfelder), PNG 600 × 1000 (Hochformat, rot, schwarz, blau gestreift), JPEG 1600 × 1200, PNG 9000 × 4, eine abgeschnittene PNG und eine Textdatei als `.png`. Test-Umgebung auf den Ports 1520/1521 (`G4_VITE_PORT=1520 G4_BRIDGE_PORT=1521`), Browser-Fenster 1440 × 900 und 1100 × 720. Die Browser-Pane war ausgeblendet (Hintergrundseite, Eingaben verzögert); geprüft wurde deshalb über den DOM (`find`, `read_page`, `javascript_tool`) und die Dateien im Projekt, Bildschirmfotos nur, wo die Seite stand. Bilder unter `target/agent/gate-2026-10-27/abnahme-4-0/` im Worktree (`start-*.jpg`, `technik-*.jpg` Bildschirmfotos zu 800 px Breite; `datei-*.png` Kopien der Projektdateien; `bundle-icon-128.png` aus dem Bundle-Symbol; `checks.log`, `pnpm-bundle.log`).

| Punkt | Ergebnis | Belege |
|---|---|---|
| (G1) 3D | bestanden: Reitermenü „3D“ › „Als Vorschaubild“ mit gezeigtem Ort schreibt `.g4/preview.png` 256 × 192 RGB, das Bild zeigt die Ansicht. Ohne gezeigten Ort war der Eintrag frei und meldete „Erst die 3D-Ansicht zeigen“ (Befund 1) | `datei-pt-g1-3d.png` |
| (G1) Palette | bestanden: „Als Vorschaubild: Kartenbild der gewählten Karte“ ohne Karte gesperrt („Erst eine Karte wählen (in 3D auf den Boden klicken)“), mit Karte 0000 frei; das Bild ist byte-gleich mit dem Standardbild (Karte 0000 ist Zweiblattdorf), Toast „Vorschaubild gesetzt“ | `datei-pt-g1-palette-karte0000.png` |
| (G1) Weltkarte | bestanden: Reitermenü „Weltkarte“ ohne Karte gesperrt (`aria-disabled`, Tooltip und `aria-describedby` „Erst eine Karte wählen“), mit Karte 0005 (Route 201) frei, das Bild zeigt Route 201 | `datei-pt-g1-weltkarte-karte0005.png` |
| (G2) Start | bestanden: Menü der Karte per Tastatur (Enter öffnet, Fokus auf „Vorschaubild wählen…“), PNG 800 × 600 eingepasst, PNG 600 × 1000 mittig auf 4 : 3 beschnitten (nur der schwarze Mittelstreifen bleibt), danach Fokus wieder auf „…“. Textdatei: „Das Bild ist weder PNG noch JPEG. Wähle eine PNG- oder JPEG-Datei.“ | `datei-ptleer-g2-png-quer.png`, `datei-pt-g2-png-hoch.png` |
| (G2) Technik | bestanden: JPEG 1600 × 1200 auf HeartGold, Untertitel „Eigenes Vorschaubild“, „Entfernen“ erscheint; Textdatei, abgeschnittene PNG und 9000 × 4 geben die Sätze von `preview.format`, `preview.unreadable` und `preview.too_large`, die Datei bleibt unverändert (SHA-1 vor und nach gleich). Der Fokus fiel nach Wählen und Entfernen auf `body` (Befund 2) | `datei-hg-g2-jpeg-technik.png`, `technik-hg-eigen-*.jpg` |
| (G3) | bestanden: „Entfernen“ in Technik (HeartGold) zeigt Neuborkia, Satz „… das Kartenbild von Neuborkia.“; „Vorschaubild entfernen“ auf Start (Platin) per Tastatur zeigt Zweiblattdorf, der Eintrag verschwindet aus dem Menü. Standardbilder: Platin Zweiblattdorf, HeartGold Neuborkia | `datei-pt-standard.png`, `datei-hg-standard.png`, `technik-hg-standard-dunkel.jpg` |
| (G4) | bestanden: ohne Bild der Name auf dem leeren Bildschirm (`4-0-abn-pt-leer`, `4-0-abn-pt-weg`); der umbenannte Ordner zeigt nach dem Klick „nicht gefunden“ mit nur „Aus der Liste nehmen“ (nach dem Neuladen wieder der Name, bekannte Grenze); „Zuletzt“ wandert auf das zuletzt geöffnete Projekt | `start-1440-*.jpg` |
| (G5) | bestanden: `pnpm bundle` (nach den Fixes), `bash apps/studio/scripts/check-bundle.sh` meldet `bundle ok: target/release/bundle/macos/Gen4 Studio.app`; `CFBundleIconFile` ist `icon.icns`, SHA-256 `3b7234b0…c3ec` gleich `apps/studio/src-tauri/icons/icon.icns`, ungleich dem alten Standardsymbol `3dc10493…a5d` (der Stand von `icon.icns` auf der Merge-Basis `2c79058`); das Bundle-Symbol zeigt die Marke | `pnpm-bundle.log`, `bundle-icon-128.png` |
| Start: Hell, Dunkel, Kompakt | bestanden: vier Karten (eigenes Bild, Standardbild, Platzhalter, fehlend), Bild 1 : 1 (256 × 192, `pixelated`), bei 1440 und 1100 Breite drei Karten je Reihe, kein waagrechter Bildlauf; Kontrast des „…“ auf dem Rahmen auch in Hell (`#F7F7FF` auf `#394252`) | `start-1440-hell.jpg`, `start-1440-dunkel.jpg`, `start-1440-kompakt-hell.jpg`, `start-1100-kompakt-hell.jpg`, `start-1100-dunkel.jpg` |
| Tastatur | bestanden: Kartenmenü und Pfaddialog per Tastatur (Enter im Feld öffnet das Menü nicht erneut); Reitermenü per Shift+F10, der gesperrte Eintrag ist per Pfeil erreichbar, Enter tut nichts, keine Datei. Technik siehe Befund 2 | – |
| MCP (`studio mcp`) | bestanden: 40 Werkzeuge (mit `--expert` 42); `project_preview_get` lesend mit Bild-Inhaltsblock `image/png` 256 × 192, im Text nur `source` und `standardMap` (Header 411, Karte 0, AreaData 6), ein `path` ist ein unbekanntes Argument; `project_preview_set` ohne `--allow-irreversible` `needs_confirmation`, eine Datei außerhalb von Projekt und `--allow-dir` abgelehnt, mit `--allow-dir` JPEG und Base64-PNG eingepasst, Textdatei `preview.format`, ohne `png` und `file` entfernt (danach kein Bildblock), `png` und `file` zusammen abgelehnt (englischer Satz, Befund 3); `g4_call` mit `path` `unavailable`, ohne `path` roh mit Base64 (Expertenweg) | – |
| Brücke (`call.sh`) | bestanden: `preview.too_large`, `preview.unreadable`, eine fehlende Datei und eine Datei außerhalb der Wurzeln werden abgelehnt, `project_preview_get` mit `path` liest ohne Öffnen | – |
| Automatik | bestanden nach den Fixes (auf `2bfbf58`): `cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features` (1452 bestanden, 1 übersprungen), vitest (2988 bestanden, 1 übersprungen), `pnpm build`, `check-styles`, `check-tokens`. `scripts/gate.sh` lief in der Abnahme nicht erneut (zuletzt grün auf `87cf829`) | `checks.log` |

**Befunde**

| Nr. | Schweregrad | Befund | Fix |
|---|---|---|---|
| 1 | Gering | Reitermenü „3D“ und Palette „Als Vorschaubild: 3D-Ansicht“ waren ohne gezeigten Ort frei; der Klick meldete als Fehler „Erst die 3D-Ansicht zeigen“, obwohl die 3D-Ansicht vorn lag (sie hat dann keine Szene). Am Reiter „Weltkarte“ war die Sperre schon da | `2b503a4`: beide gesperrt mit „Erst einen Ort zeigen“ (dieselbe Bedingung wie der leere Zustand der Ansicht), der Befehl sagt es auch, falls er doch läuft; Tests in `paletteEntries.test.ts` und `Deck.test.tsx`, README „Nachtrag 4.0“. Im Browser nachgeprüft |
| 2 | Mittel (Tastatur) | Technik › Projekt › Vorschaubild: nach „Vorschaubild wählen…“ und nach „Entfernen“ lag der Fokus auf `body` (die Tasten wurden während des Schreibens `disabled`, „Entfernen“ verschwindet danach) | `1dfcb70`: während des Schreibens `aria-disabled` statt `disabled` (der Baustein `Button` zeigt `aria-disabled` wie gesperrt, README-Zeile „Taste“), danach Fokus auf „Vorschaubild wählen…“; zwei Tests in `TechnikArea.test.tsx`. Mit echter Tastatur nachgeprüft |
| 3 | Gering | `project_preview_set` mit `png` und `file` antwortete englisch („png and file exclude each other“), die übrigen Ablehnungen sind deutsch | `2bfbf58`: „png und file schließen sich aus; nur eins von beiden angeben“ |
| H1 | Hinweis, kein Fix | Auf der HeartGold-Karte bricht „geöffnet vor …“ hinter der langen Plakette „HeartGold/SoulSilver, DE“ in eine zweite Zeile; die Reihe wird dadurch 25 px höher als eine Reihe nur mit Platin-Karten | – |
| H2 | Hinweis, vor 4.0 | Bei 1100 px kürzt die Titelleiste den Projektnamen auf „4…“, wenn die Plakette „HeartGold/SoulSilver, DE“ daneben steht | – (Titelleiste, nicht Teil von 4.0) |
| H3 | Hinweis, bekannt | Ein fehlgeschlagenes Öffnen zeigt im Browser-Modus den rohen Satz der Brücke („Pfad nicht lesbar: … (os error 2)“) | – (Ruling N1, nur Test-Umgebung) |

## Beim Nutzer offen

**Hand**

- [x] (G5) vom Agenten in der Abnahme bestanden (`bundle ok`, Hash gleich `icons/icon.icns`). Wer es selbst sehen will: in `apps/studio` `pnpm bundle`, dann aus der Repo-Wurzel `bash apps/studio/scripts/check-bundle.sh`.
- [ ] Marke in der gepackten App (`target/release/bundle/macos/Gen4 Studio.app`; im Worktree liegt sie schon, gebaut in der Abnahme auf `2bfbf58`): Dock (hell und dunkel), Finder (Symbol- und Listenansicht), Menü „Gen4 Studio“ › „Über Gen4 Studio“ zeigen die neue Marke, nicht die zwei Ringe von create-tauri-app. Falls macOS das alte Symbol aus dem Cache zeigt: die App einmal an einen anderen Ort kopieren oder `killall Dock`.

**Blick**

**Neu aufgenommen am 2026-10-31.** Die Bilder unter `target/agent/gate-2026-10-27/` gingen mit dem Ordner `target/` verloren. Sie sind neu aufgenommen, auf dem Stand von main `03a5615` (alle Fixes bis 4.4), in der Test-Umgebung mit Chrome headless (Playwright) auf frischen Kopien. Die Namen sind dieselben wie unten. Neuer Ort: `target/agent/gate-2026-10-31/blick-neu/4-0/` im Haupt-Checkout (aufgenommen im Worktree `.claude/worktrees/blick-4`, Branch `blick-phase-4`), dazu eine Kopie außerhalb des Repos unter `$HOME/Projekte/lab/editor/_blick-phase-4/gate-2026-10-31/blick-neu/4-0/`. Die Bilder der Abnahme liegen dort im Unterordner `abnahme-4-0/`. Das sind 12 Bilder. Sie weichen in diesen Punkten vom alten Inhalt ab:

- **Alle Bilder:** Die Projekte heißen wie die Kopien (`blick4-4-0-pt1_DSPRE_contents` usw.). Die eigenen Bilder sind Farbfelder ohne Spielgrafik.
- **`abnahme-4-0/start-1100-*.jpg`:** Die Seite ist ans Ende gerollt, damit die vierte Karte („nicht gefunden“) im Bild ist. Bei 720 px Höhe passen nicht alle Karten ins Fenster, deshalb ist die erste Reihe angeschnitten.
- **`start-1440-kompakt-hell.jpg`:** Unten ist der Hilfetext abgeschnitten.
- **Technik:** Die Liste hat seit 4.3b den Eintrag „Textprojekt“.
- **Toasts:** Die Meldung „Dieses Projekt lässt sich als Textprojekt einrichten …“ (seit 4.3b) und der rohe Fehlertoast nach dem fehlgeschlagenen Öffnen (H3) sind vor der Aufnahme geschlossen.
- **`abnahme-4-0/bundle-icon-128.png`:** Es stammt aus `apps/studio/src-tauri/icons/icon.icns` (`sips -Z 128`), nicht aus einem neu gebauten Bundle. Laut (G5) hat das Bundle denselben SHA-256 `3b7234b0…c3ec`. Es zeigt die Marke.
- **`blick-galerie-projektstart-dunkel.png`:** Die Galerie ist `http://127.0.0.1:<port>/?gallery`, Seite „Projektstart“, Dunkel. Sie ist als ganze Seite aufgenommen (1440 × 3290). Laut Aufnahme schneidet der Rahmen „Projektkarte“ die vierte Karte wegen seiner festen Höhe ab (Hinweis, nicht geändert).
- **`fig-marke.png`:** Nicht neu aufgenommen. OpenPencil antwortete am 2026-10-31 nicht (`RPC timeout`). Das Brett liegt unverändert in der `.fig` (Seite „Grundlagen“, x 1720, y 2900).
- **Nicht neu:** Die Bilder der Punkte (G1) bis (G4) (`g1-*.png` usw.) gehören nicht zum Blick und sind nicht neu aufgenommen.

- [ ] Start mit Bildern in Hell, Dunkel und Kompakt: `blick-start-hell.png`, `blick-start-dunkel.png`, `blick-start-kompakt-hell.png`, dazu aus der Abnahme `abnahme-4-0/start-1440-hell.jpg`, `start-1440-dunkel.jpg`, `start-1440-kompakt-hell.jpg`, `start-1100-kompakt-hell.jpg`, `start-1100-dunkel.jpg` (vier Karten: eigenes Bild, Standardbild, Platzhalter, fehlend; Hinweis H1 an der HeartGold-Karte); Technik › Projekt: `blick-technik-projekt-hell.png`, `abnahme-4-0/technik-hg-eigen-hell.jpg`; Galerie „Projektstart“: `blick-galerie-projektstart-dunkel.png`.
- [ ] Das Bundle-Symbol in 128 px: `abnahme-4-0/bundle-icon-128.png` (aus `Gen4 Studio.app/Contents/Resources/icon.icns`).
- [ ] Die Marke: Brett „Marke“ in der `.fig` (Seite „Grundlagen“, x 1720, y 2900; Bild `fig-marke.png`) und das Symbol im Dock von `tauri dev` (`cd apps/studio && pnpm tauri dev`).
- [x] Teil B der `.fig` (Agent, bei entsperrtem Bildschirm): Start mit Bildern, Komponente „Projektkarte“ und Technik › Projekt fehlen noch in der Datei. OpenPencil antwortete bei gesperrtem Bildschirm nicht (`RPC timeout`). Die Vorlagen `start-vorschau`, `projektkarte` und `technik-projekt` liegen unter `design-system/openpencil/beispiele/` und ergeben mit `build-screen.sh` gültiges JSX; Orte und Inhalt in `design-system/openpencil/NACHTRAG-4-0.md`, Teil B. Danach `fix_fig_text.mjs --rgb555` und der Blick auf die drei Screens. **Erledigt am 2026-10-27** in der Sitzung „Design-System: .fig-Nachträge 4.0, 4.1a, 4.2“ (Commit `30d2b6b`, gemergt `076b928`): „App: Start, Projekte mit Vorschaubild“, „App: Technik, Projekt mit Vorschaubild“ und das Brett „Komponenten (App): Projekt“ stehen in der `.fig` (README, „Nachtrag 4.0“).

## Bekannte Grenzen und Folgepunkte

- `.g4/cache/` ist noch nicht von Git ausgenommen; die Erlaubnisliste für `.g4/` kommt mit Phase 4 (Gutachten C zum Projektformat). Jedes Öffnen schreibt `.g4/cache/preview-standard.png` in das Projekt.
- „nicht gefunden“ erscheint wie vor 4.0 erst nach einem fehlgeschlagenen Öffnen und geht nach einem Neuladen verloren.
- Fehlertexte: ein nicht zugeordneter Fehler beim Setzen (etwa fehlendes apicula beim Kartenbild) zeigt den allgemeinen Satz „Das Vorschaubild ließ sich nicht speichern …“; im Browser-Modus ersetzen einige deutsche Ablehnungen der Brücke durch denselben Satz; Fehler des Browsers oder von three.js erscheinen roh.
- Im Reitermenü steht der Grund einer Sperre nur als Tooltip (für Screenreader über `aria-describedby`), in der Palette sichtbar.
- `g4_call` und `g4_batch` geben die Vorschau-Befehle roh mit Base64 zurück (Expertenweg); die kuratierten Werkzeuge liefern das Bild als Bildblock.
- Der Katalog der MCP-Werkzeuge hat 45 Werkzeuge, die obere Grenze von `tool_count_is_about_thirty`; das nächste Werkzeug muss die Grenze anheben.
- `test_marke.py` läuft nicht in `gate.sh`; nach einer Token-Änderung `python3 design-system/scripts/marke.py` und `pnpm tauri icon` von Hand.
- Ein Bild aus der Szene zeigt die Ansicht, wie sie ist, mit Auswahlrahmen.
