# Phase 3c: Doppeldeck-Fundament (Design-System in der App)

Stand: 2026-10-23. Eingeschoben zwischen 3b.2 und 3b.3 der [Roadmap](../../ROADMAP.md). Gestaltungsquelle ist das Design-System „Doppeldeck“ (OpenPencil-Datei `Editor_DesignSystem.fig` und der Ordner `design-system/`, bisher neben dem Repository, siehe §4). Baut auf dem Welt-Explorer ([Spec 3a](2026-10-22-world-explorer-design.md)), dem Fundament ([Spec 3b.1](2026-10-22-phase-3b1-foundation-design.md)) und Scripts und Text ([Spec 3b.2](2026-10-22-phase-3b2-scripts-text-design.md)) auf.

## 1. Ziel und Umfang

Die App bekommt das Doppeldeck-Raster (Titelleiste, Atlas, Liste, Schau, Scharnier, Hand, Statusleiste), die Tokens, Schriften und Symbole des Design-Systems und eine Komponentenbibliothek, auf der die Daten-Editoren von 3b.3 direkt aufsetzen. Alles, was die App heute kann (Welt, Assistenten, Text, Scripts, Probleme, Bauen, Starten, Browser-Modus der Test-Umgebung), kann sie danach weiter, an seinem neuen Platz und im neuen Gewand. 3c bringt keine neuen Spieldaten-Fähigkeiten.

Warum vor 3b.3: 3b.3 bringt fünf neue Editoren (Pokédex, Trainer, Begegnungen, Attacken, Gegenstände). Im alten Stil gebaut, müssten sie nach dem Umbau ein zweites Mal angefasst werden; heute sind erst drei Bereiche umzustellen (Welt, Texte, Scripts). Die Bausteine, die 3b.3 braucht (Picker mit Vorschau, Referenz mit „Verwendet in“, Zahlfeld mit Bitbreite), entstehen hier einmal.

3c liefert acht Posten:

1. **Design-System im Repo.** `design-system/` zieht nach `gen4-studio/design-system/` um, mit der `.fig`. Die generierten Dateien (`tokens.css`, `tokens.ts`, `dockview-theme.css`) liegen eingecheckt in der App; ein Gate-Schritt prüft, dass sie zum Export passen.
2. **Paket `ui/`** in `apps/studio/src/ui/`: Tokens, Thema-Umschaltung, Primitives auf Base UI, Rahmen-Komponenten, Galerie.
3. **Der Rahmen:** Titelleiste (macOS-Overlay), Atlas mit Start und sechs Bereichen, Liste, Deck aus zwei Dockview-Instanzen (Schau, Hand) mit dem Scharnier dazwischen, Statusleiste, Toasts, Befehlspalette ⌘K.
4. **Bereich Welt** im neuen Raster: Explorer als Liste, 3D / Raster / Weltkarte als Schau, Werkzeuge | Inspector | „An diesem Ort“ als Hand; Werkzeuge mit Buchstabenkürzeln; Szenenfarben aus den Tokens.
5. **Bereich Texte:** Archive als Liste, DS-Bildschirm-Vorschau als Schau, Nachrichten und Editor als Hand.
6. **Scripts und Probleme** als Hand-Panels der Welt; Assistenten als Schritte im Scharnier statt Modal; Start, Technik, Einstellungen, Hilfe als Flächen.
7. **Tiefen Bauen und Tüfteln** im Scharnier; Tüfteln zeigt Rohwerte (§7.7). Erzählen kommt später (§13).
8. **Prüfbarkeit:** Galerie aller Komponenten im Browser-Modus, Stil-Prüfung (`scripts/check-styles.sh`) und Token-Abgleich im Gate, die Abnahmen 3a, 3b.1 und 3b.2 laufen über die Test-Umgebung erneut.

**Zwei Pläne.** 3c.1 „Fundament und Welt“ (Posten 1 bis 4, Tiefe im Inspector, Toasts, Stil-Prüfung mit Ausnahmeliste); 3c.2 „Texte, Scripts, Technik, Assistenten“ (Posten 5 bis 8, Palette, Ausnahmeliste leer, Abnahme). 3c.1 endet lauffähig: Text, Script und Probleme sitzen dann schon in der Hand, innen aber noch im alten Stil. Der Plan für 3c.2 wird nach dem Merge von 3c.1 geschrieben.

**Nicht in 3c:** die Bereiche Geschichten, Pokémon und Gegenstände (gesperrt im Atlas); Tiefe Erzählen mit Rezepten und reduziertem Inspector; Live-Vorschau der Assistenten in der Schau (gestrichelte neue Zellen); Story-Graph; Spielmodus mit Live-Flags und bernsteinfarbenem Scharnier; Veröffentlichen (Phase 4); Plugins (Phase 5); Gelände malen und Draufsicht-Kamera (Phase 6); Muster im Gelände-Overlay (§8.1); Höhen-Overlay in der Schau; eigene Fensterknöpfe auf Windows; senkrechtes Scharnier ab 2200 px; Komponenten `TypeBadge`, `StatBar`, `Budget`, `PokemonCard`, `StoryNode`, `RecipeButton` (kommen mit ihrem Editor); Rückwärtsreferenzen „welche Scripts nutzen Nachricht N“ (bleibt aus 3b.2 ausgeschlossen).

**Gate** (Ablauf in `docs/phase-3c-gate.md`, entsteht mit 3c.2):

- (A) Der Screen „Welt bauen: Figur bearbeiten“ ist in der App mit einer Platin-DE-Kopie wiedererkennbar: Atlas, Explorer, 3D-Schau mit schwebender Ebenen-Leiste und Legende, Scharnier mit Brotkrume, Tiefe, Prüflicht, Verlauf und „Hier spielen“, Hand mit Werkzeugen, Inspector und „An diesem Ort“. Ein Event wird gewählt, ein Feld geändert, ⌘Z nimmt es zurück, ⌘S speichert.
- (B) Texte: der Screen „Texte: Archiv 0412“ ist wiedererkennbar; eine Nachricht wird geändert, die DS-Vorschau zeigt sie seitenweise, die Suche findet einen angefügten Text.
- (C) Scripts: ein Script öffnet sich in der Hand mit rotom-lsp, die Hand ist mindestens 400 px hoch, ein Compile-Fehler erscheint im Prüflicht und im Panel Probleme, der Sprung zur Zeile klappt.
- (D) Assistent „Raum anlegen“ läuft als Schrittleiste im Scharnier durch; der Raum ist in `g4emu` begehbar (Szenario `pt-raum-assistent.yaml` unverändert grün).
- (E) Dunkles Thema und Dichte Kompakt: Galerie und Welt-Bereich ohne unlesbare Stelle (Agent-Screenshots, Blick des Nutzers).
- (F) Tastatur: Atlas, Liste, Werkzeuge, Inspector und Scharnier sind mit Tab, Pfeiltasten und Enter bedienbar; der Fokusring ist nur bei Tastaturbedienung sichtbar.
- (G) Die Abnahmen 3a (A) bis (D), 3b.1 (A) und (D), 3b.2 (A) bis (F) laufen über die Test-Umgebung erneut grün; `scripts/gate.sh` grün gegen alle drei Projekte, einschließlich der neuen Schritte Token-Abgleich und Stil-Prüfung.
- (H) Gepackte App auf macOS (`pnpm bundle`): Overlay-Titelleiste, Schriften geladen (CSP), Thema folgt dem System. Hand des Nutzers.

## 2. Fakten

### 2.1 Design-System (Stand 23.10.2026, `../design-system`)

- Tokens im W3C-DTCG-Format: Sammlungen Primitiv, Szene, Thema (Hell, Dunkel), Raum, Dichte (Komfort, Kompakt), Bewegung, Schrift, Textstile. Alle Farben RGB555-treu; `scripts/check_tokens.py` prüft Raster und WCAG-Kontraste, `scripts/export_tokens.py <root>` schreibt `<root>/css/tokens.css` (400 Zeilen, `--g4-*`, Hell in `:root`, Dunkel unter `:root[data-theme="dunkel"]` und `prefers-color-scheme: dark` ohne `data-theme="hell"`, Kompakt unter `data-dichte="kompakt"`), `<root>/css/dockview-theme.css` (Klasse `dockview-theme-g4`) und `<root>/ts/tokens.ts` (`szene`, `gelaendeMuster`, `ereignisForm`, `hell`, `dunkel`, `syntax`, `typen`, `rgb555()`).
- `README.md` beschreibt Prinzipien, Fensterraster (Titelleiste 40, Atlas 88, Liste 272, Scharnier 44, Hand 286 von 200 bis 560, Statusleiste 28, Referenz 1440 × 900, Mindestgröße 1100 × 720), Typografie (Atkinson Hyperlegible Next 13/18, Zen Maru Gothic für Titel, Atkinson Hyperlegible Mono für Kennungen, Pixelify Sans für DS-Vorschauen), Komponenten, Muster, Szene, Barrierefreiheit. `openpencil/BRIEFING-SCREENS.md` und `openpencil/jsx-lib.js` dienen nur der Arbeit an der `.fig`. `scripts/fix_fig_text.mjs` repariert die `.fig` nach dem Speichern in OpenPencil.
- Die `.fig` (2,2 MB) hat acht Seiten: Titel, Grundlagen, Komponenten (Tasten, Eingaben, Umschalter, Inhalt und Status, Listen und Inspector, Scharnier, Rahmen, Spiel und Welt, Schwebend), Muster, vier Screen-Seiten. Screens zeigen den Zielzustand mit allen Features, nicht den Stand der App.

### 2.2 Codebasis (Stand `main` f141385)

- Frontend `apps/studio/src`: React 19, `dockview-react` 8.6.1 mit `themeLight`, kein Stylesheet außer `dockview.css`; rund 450 `style={{…}}` in 36 Dateien, Hex-Farben verstreut, `fontFamily: "system-ui"` überall. Neun Dockview-Panels (`layout.ts`, `LAYOUT_KEY = "g4.layout.v5"`): `world` (3D), `matrix`, `worldmap`, `text`, `script` als Reiter einer Gruppe, `headers` links (300), `inspector` rechts (320), `status` und `problems` unten. `Toolbar` oben (Projekt öffnen, Undo, Redo, Speichern, Bauen, Starten, Emulator wählen, Ungespeichert-Zähler), `StatusBar` unten (Meldungen, Problemzähler, „Browser-Modus (Brücke)“). Assistenten (`WizardShell`, `NewAreaWizard`, `NewRoomWizard`) und `ExtendAreaDialog` sind Modals mit eigener Fokusfalle (`modal.ts`). `WizardStep { id, title, content, canNext }`.
- `App.tsx` setzt die Hooks zusammen (`useProjectSession`, `useSceneSync`, `useWorldActions`, `useWizards`, `useWorkbenchModel` → `WorkbenchData`/`WorkbenchLive`); Panels lesen Kontexte (`useWorkbench`, `useInspectorModel`, `useMessage`). Invariante aus 3a: Hover und Meldungen rendern keine Panels neu.
- Szene: `world/scene/content.ts` baut das Gelände-Overlay als `InstancedMesh` mit `setColorAt` je Zelle (`overlayColor`, Farben aus `world/colors.ts`: Klassen `grass, water, ice, sand, ledge, block, slide, warp, door, bridge, furniture`, `BLOCKED = 0x80` rot, neutral grau). Marker sind `THREE.Sprite`s (`markers.ts`), Kamera `PerspectiveCamera(50)`, kein orthografischer Modus. Hintergrund der Szene ist in `SceneHost` gesetzt.
- Kernel: `Store` hält `undo: Vec<Changeset>`, `redo: Vec<Changeset>`; `Changeset.label` ist vorhanden, aber ungenutzt („kept for the history UI“). Tauri-Befehle `undo`, `redo`, `dirty`; kein Befehl liefert die Verlaufstiefe.
- Tests: vitest 5, `environment: node`, 39 Dateien mit `// @vitest-environment jsdom`, 925 Tests; Tests fragen überwiegend nach Rollen, `aria-label`, Texten und `data-*`-Attributen (`data-archive`, `data-message`, `data-script`, `data-header`, `data-hit`, `data-script-file`, `data-lsp`).
- Tauri: ein Fenster 1100 × 750, Standard-Titelleiste, natives Menü mit eigenem Quit (`menu.rs`); CSP `default-src 'self'; img-src 'self' data: blob:; style-src 'self' 'unsafe-inline'; connect-src ipc: http://ipc.localhost blob:`. Browser-Modus: Vite auf 1430 mit Proxy zur Brücke, `platform/transport.ts` (`inTauri()`), `platform/dialogs.ts`, `WebDialogHost`.
- Test-Umgebung: `docs/agent-gate.md` (Rezepte nennen Knöpfe und Felder beim Namen), `scripts/agent/scenarios/*.yaml` (nur Backend-Befehle, keine UI-Selektoren), `scripts/gate.sh` mit `run_step`.

### 2.3 Bibliotheken (per `pnpm view`, 23.10.2026)

| Paket | Version | Zweck |
|---|---|---|
| `@base-ui/react` | 1.9.0 | ungestylte Primitives: Dialog, Popover, Menu, Select, Combobox, Tooltip, Tabs, Switch, Checkbox, Toggle Group, Number Field, Field, Input. Nachfolger von Radix von denselben Autoren; shadcn/ui-Standard seit Juli 2026. |
| `@phosphor-icons/react` | 2.1.10 | Symbole, `weight="regular"` für Oberfläche, `"fill"` für Marker; tree-shakable |
| `@fontsource-variable/atkinson-hyperlegible-next` | 5.3.0 | Oberfläche |
| `@fontsource/atkinson-hyperlegible-mono` | 5.3.0 | Kennungen, Code |
| `@fontsource/zen-maru-gothic` | 5.3.1 | Titel (700, 900, Latin) |
| `@fontsource/pixelify-sans` | 5.3.0 | DS-Vorschau |
| `dockview-react` | 8.6.1 (bleibt) | Docks |

Alle Schriften SIL OFL, Phosphor MIT, Base UI MIT; sie werden mit der App ausgeliefert, kein Netz zur Laufzeit.

## 3. Entscheidungen im Überblick

| Frage | Entscheidung | Grund |
|---|---|---|
| Gate-Umfang | Referenz-Screen mit echten Daten (A), Text, Script, Probleme in der Hand, Abnahmen erneut | reiner Rahmen würde die Komponentenarbeit nur nach 3b.3 schieben; Erzählen bräuchte eine zweite Inspector-Variante je Editor |
| Ort des Design-Systems | im Repo, `.fig` eingeschlossen | Gestaltung und Code ein Stand; kein Remote, kein Binär-Diff-Problem |
| Plattform | macOS Overlay-Titelleiste, Windows Systemleiste mit 40-px-Zeile darunter | Windows hier nicht prüfbar; eigene Fensterknöpfe sind ein eigenes Projekt |
| Verhaltensschicht | Base UI 1.9 | Radix verlangsamt, kein Number Field, kein Combobox; Eigenbau kostet Wochen |
| Umbau | neue Hülle auf einem Branch, altes Layout fällt am Ende weg | kein Release, keine Nutzer einer Zwischenversion |
| Deck | zwei Dockview-Instanzen (`schau`, `hand`), Scharnier als eigene Komponente | Dockview kennt keinen eigenen Trenner zwischen Gruppen; Schau und Hand tauschen nie Panels |
| Styling | CSS Modules, nur `--g4-*`-Variablen, kein CSS-in-JS, kein Hex außerhalb `ui/tokens` | Lint-bar; erlaubt später `'unsafe-inline'` aus der CSP zu nehmen |
| Namen | Code englisch (`Button`, `Hinge`), Texte deutsch, Token-Namen wie generiert | wie der Rest der App |
| Tiefe | Bauen und Tüfteln; Erzählen später | Erzählen braucht Rezepte, die es noch nicht gibt |
| Meldungen | Toasts unten rechts; Fehler bleiben, Compile-Probleme ins Prüflicht | Design: Rückmeldung dort, wo das Problem ist, keine Dauerleiste |

## 4. Struktur und Token-Pipeline

**Ordner.** `gen4-studio/design-system/` enthält `tokens/`, `scripts/`, `openpencil/`, `README.md`, `Editor_DesignSystem.fig`. `export_tokens.py` bekommt einen zweiten Aufrufparameter, den Ausgabeordner; `pnpm tokens` (Skript in `apps/studio/package.json`) ruft `python3 ../../design-system/scripts/export_tokens.py ../../design-system src/ui/tokens` auf und schreibt `tokens.css`, `dockview-theme.css`, `tokens.ts` dorthin. Die drei Dateien sind eingecheckt, `pnpm build` braucht kein Python. `scripts/gate.sh` bekommt den Schritt „tokens up to date“: Export in ein Temp-Verzeichnis, `diff -r` gegen `apps/studio/src/ui/tokens/` (nur die drei generierten Dateien), rot bei Abweichung; davor läuft `check_tokens.py`. Der README-Ablauf („Token ändern → check → export → OpenPencil importieren“) bleibt und nennt die neuen Pfade. Vor einem Commit der `.fig` läuft `node design-system/scripts/fix_fig_text.mjs design-system/Editor_DesignSystem.fig --rgb555`; die Sicherung `.vor-textfix` steht in `.gitignore`.

**Paket `apps/studio/src/ui/`:**

| Ordner | Inhalt |
|---|---|
| `tokens/` | `tokens.css`, `dockview-theme.css`, `tokens.ts` (generiert); `base.css` (Reset, `html { font: var(--g4-font-text-s) }`, `lang`, Fokusring, `prefers-reduced-motion` setzt alle `--g4-dauer-*` auf 0, `forced-colors`); `theme.ts` |
| `primitives/` | die Bausteine aus §6, je `Name.tsx`, `Name.module.css`, `Name.test.tsx` |
| `frame/` | `Shell`, `TitleBar`, `Atlas`, `ListPane`, `Deck`, `Hinge`, `StatusLine`, `ToastHost`, `Palette`, `shellState.ts` |
| `gallery/` | `Gallery.tsx` und eine Seite je Primitive |

`theme.ts`: `readTheme(): "system" | "hell" | "dunkel"`, `applyTheme(t)` setzt oder entfernt `data-theme` am `html`, `readDensity(): "komfort" | "kompakt"`, `applyDensity(d)` setzt `data-dichte`; beides in localStorage (`g4.theme`, `g4.density`), Standard System und Komfort. `effectiveTheme(): "hell" | "dunkel"` wertet `matchMedia` aus; `onThemeChange(cb)` feuert bei Umschaltung und bei Systemwechsel (die Szene und CodeMirror hören darauf).

**Styling-Regeln.** Komponenten und Panels benutzen nur Thema-, Raum-, Dichte- und Schrift-Tokens; Primitive (`--g4-platin-500`) und Hex-Werte kommen in `src/` nicht vor, außer in `ui/tokens/` und in `world/` dort, wo three.js Zahlen braucht (die kommen aus `tokens.ts`). `scripts/check-styles.sh` prüft mit `grep`: (1) `style={{` in `.tsx` außerhalb `ui/tokens/` und (2) `#[0-9a-fA-F]{3,8}\b` in `.tsx`, `.ts`, `.css` außerhalb `ui/tokens/` und `world/colors.ts`; eine Ausnahmeliste `scripts/check-styles.allow` nennt Dateien, die noch nicht umgezogen sind (3c.1: `workbench/text/`, `workbench/script/`, `workbench/problems/`, `workbench/wizards/`, `workbench/ExtendAreaDialog.tsx`; 3c.2: leer). Inline-`style` bleibt erlaubt für Werte, die zur Laufzeit entstehen (Position eines Markers, Breite eines Balkens), gekennzeichnet mit `/* dyn */` in derselben Zeile; die Prüfung überspringt solche Zeilen. Das Gate ruft das Skript nach `pnpm test` auf.

**Abhängigkeiten.** Neu: `@base-ui/react`, `@phosphor-icons/react`, die vier `@fontsource`-Pakete (feste Versionen aus §2.3). `main.tsx` importiert die Schriften (`@fontsource-variable/atkinson-hyperlegible-next`, `@fontsource/atkinson-hyperlegible-mono/400.css`, `@fontsource/zen-maru-gothic/700.css` und `/900.css`, `@fontsource/pixelify-sans/400.css`), `ui/tokens/tokens.css`, `base.css`, `dockview/dist/styles/dockview.css`, `ui/tokens/dockview-theme.css`. CSP bleibt (`default-src 'self'` deckt `font-src`); `index.html` bekommt `lang="de"`. Dockview bleibt bei 8.6.1 mit `className="dockview-theme-g4"` statt `theme={themeLight}`.

## 5. Der Rahmen

### 5.1 Hülle

`Shell` ist ein CSS-Grid: Zeilen `40px 1fr 28px`, Spalten `88px auto 1fr`. Die mittlere Spalte ist die Liste (272 px, einklappbar auf 0, Zustand je Bereich), die rechte das Deck. Bereiche ohne Liste und Deck (Start, Technik, Einstellungen) rendern eine Fläche über beide Spalten. Der Rahmen ist Teil von `App`; die Hooks und Kontexte bleiben, `Toolbar`, `StatusBar` und `Workbench` verschwinden.

`shellState.ts` hält (localStorage, Schlüssel `g4.shell.v1`): aktiver Bereich, Liste je Bereich auf oder zu, Hand-Höhe je Bereich (Standard 286; nach Doppelklick 48 mit gemerkter letzter Höhe), Tiefe (`bauen` | `tuefteln`, Standard `bauen`), zuletzt geöffnete Projekte (höchstens fünf: Pfad, Name, Spiel, Zeitpunkt). Der Hook `useShell()` liefert Zustand und Setter; er lebt über den Panels, aber Änderungen daran rendern nur Rahmen-Komponenten (eigener Kontext, nicht `WorkbenchData`).

### 5.2 Titelleiste

40 px, `flaeche/gehaeuse`, auf macOS in Tauri mit 78 px Einzug links für die Ampel (`titleBarStyle: "Overlay"`, `hiddenTitle: true` in `tauri.conf.json`; das native Menü bleibt), sonst ohne Einzug unter der Systemleiste. Die Zeile trägt `data-tauri-drag-region`, Bedienelemente nicht. Inhalt: Projektname (ohne Projekt „Kein Projekt geöffnet“), Spiel-Plakette (`Platin, DE` aus `ProjectInfo`), Ungespeichert-Zeile („● 3 Änderungen nicht gespeichert“ in `akzent/text`, „Alles gespeichert“ mit Haken), Mitte ein Suchfeld-Knopf „Suchen oder Befehl eingeben ⌘K“ (öffnet die Palette, §5.8), rechts Symboltasten Speichern (⌘S), Bauen, Thema (Sonne/Mond, wechselt hell ↔ dunkel; „System“ nur in Einstellungen), Hilfe (Glossar-Dialog). Fenster: 1440 × 900 Start, `minWidth` 1100, `minHeight` 720.

### 5.3 Atlas

88 px, Einträge je 64 px hoch mit Symbol (Phosphor: `House`, `MapTrifold`, `ChatsCircle`, `PawPrint`, `Backpack`, `TextAa`, `Cpu`) und Beschriftung; aktiv trägt `bereich/<x>-flaeche`, Symbol und Text in `bereich/<x>`, links ein 3-px-Strich. Unten Hilfe und Einstellungen. Geschichten, Pokémon und Gegenstände sind gesperrt (Text `text/deaktiviert`, Tooltip „Kommt mit Phase 3b.3“, kein Klick). `role="navigation"`, Einträge als Toggle-Buttons mit `aria-current`. Kompakt: nur Symbole, 56 px breit.

### 5.4 Liste

`ListPane` mit `PanelHead` (Titel, Untertitel, Aktionen) und Inhalt. Welt: der Explorer (§7.1). Texte: die Archive (§7.2). Einklappen über einen Griff am rechten Rand (Doppelklick) und die Palette („Liste ein/aus“); eingeklappt bleibt ein 8-px-Griff.

### 5.5 Deck und Scharnier

`Deck` rendert `DockviewReact` für die Schau, dann `Hinge`, dann `DockviewReact` für die Hand, als Flex-Spalte; die Hand hat die gemerkte Höhe, die Schau den Rest. Beide Docks haben eigene `components`-Tabellen, eigene Layout-Schlüssel (`g4.layout.v6.<bereich>.schau`, `….hand`) und dieselben Regeln wie heute (`isSaneLayout`: jedes Pflichtpanel vorhanden; Panels nicht schließbar, `renderer: "always"`). Gruppen mit einem Panel verbergen Dockviews Reiterleiste (`group.header.hidden = true`), das Panel zeigt dann seinen eigenen `PanelHead`; ab zwei Panels erscheint die Reiterleiste (28 px, `dockview-theme-g4`). Dockview-Reiter haben kein Schließen-Kreuz; der Reiter „Script“ trägt den Kobalt-Punkt, sobald ein Dokument ungespeichert ist.

`Hinge` (44 px, `scharnier/grund`, Text `scharnier/text`, Übergang `dauer/scharnier`): links ein Griff (sechs Punkte) und die Brotkrume mit höchstens drei Gliedern, jedes anklickbar (`goTo`), das letzte fett; Mitte die Tiefe als `Segmented` (Bauen, Tüfteln) mit Symbolen (`Hammer`, `Wrench`); rechts das Prüflicht (Punkt in `status/fehler` oder `status/warnung`, Text „2 Fehler, 1 Hinweis“ aus `countsText`, „Alles stimmig“ in `status/erfolg` bei null; Klick öffnet das Panel Probleme), der Verlauf (Pfeil-Symbol mit Zahl der Undo-Schritte, Klick öffnet ein `Menu` mit den Labels der letzten zehn Schritte, Eintrag klickt bis dorthin zurück; daneben Redo, gesperrt bei leerem Redo-Stapel), und die Taste `Button kind="play"` „Hier spielen ⌘R“ (speichert, baut, startet; während des Laufs „Baut …“ gesperrt; im Browser-Modus baut sie nur und der Toast sagt es). Brotkrume: Ort (Platz aus `buildPlaces`) › Header (Ortsname oder „Raum“ plus Kennung grau) › Auswahl (Event-Titel aus `eventTitle`, „Gebäude 3“, „Zelle 7, 25“). Ohne Projekt steht „Kein Projekt“.

Ziehen am Scharnier (Pointer Capture) setzt die Hand-Höhe zwischen 200 und 560 px; Doppelklick auf den Griff klappt auf 48 px (nur das Scharnier bleibt) und zurück auf die gemerkte Höhe. Öffnet sich ein Script in der Hand und die Hand ist niedriger als 400 px, hebt die App sie auf 400 (einmal je Öffnen).

**Assistenten-Modus.** `useWizards` liefert weiter `request`; ist einer offen, zeigt `Hinge` die Schrittleiste (Bereichssymbol, Titel in `font-titel-s` verkleinert auf 15 px, Schritte nummeriert mit Haken für Erledigtes, rechts Abbrechen, Zurück, `Button kind="primary"` Weiter oder die Anlegen-Taste) und die Hand zeigt statt des Docks den Schritt (`WizardFrame`: Inhalt links, Zusammenfassung oder Fehler als `Notice` unten; `role="dialog"` bleibt für die Tests, `aria-modal` entfällt). Das Hand-Dock bleibt im DOM, nur ausgeblendet (`hidden`), damit CodeMirror und three.js nicht neu entstehen. Die Schau bleibt bedienbar: der Schritt „Eingang“ nimmt wie heute die zuletzt in der Szene gezeigte Kachel. Enter führt weiter (außer in Textfeldern mit Draft), Escape bricht ab (Regeln von `escapeCloses`). Während `busy` ist nichts klickbar. Das Ergebnis ersetzt den Schritt und bietet „Zeigen“ und „Schließen“. `ExtendAreaDialog` wird ein `Dialog` (§6), kein Assistent.

### 5.6 Statusleiste

28 px, `flaeche/gehaeuse`, Text `text/sekundaer`, 12 px. Links das aktive Werkzeug mit seinen Kürzeln (`Kbd`): „Auswählen · Klick wählt · Ziehen verschiebt · ⇧Klick fügt hinzu · Doppelklick öffnet“, bei Gelände malen „Klick setzt Kollision · ⇧Klick löscht“. Mitte der Kontext: Kachel unter dem Zeiger (`tileSummary`), sonst „Zweiblattdorf, Raster 0, Zelle 3, 27“. Rechts Build-Stand (Haken „Gebaut vor 4 Minuten“ aus `session.lastRom` und einem Zeitstempel, „Noch nicht gebaut“, während des Baus Spinner „Baut …“), Dichte-Umschalter („Komfort“ | „Kompakt“ als Textknopf), im Browser-Modus „Browser-Modus (Brücke)“ beziehungsweise „Brücke nicht erreichbar“. Die Statusleiste liest nur `useMessage`-artige, volatile Kontexte; sie rendert nicht die Panels.

### 5.7 Toasts

`ToastHost` unten rechts über der Statusleiste, Breite 360, Stapel bis vier, neueste unten. `toast.info(text, {actions})` verschwindet nach 5 s (Hover hält an), `toast.error(text, {actions})` bleibt bis zum Schließen; `toast.progress(key, text)` ersetzt einen Toast gleichen Schlüssels (Ladefortschritt der Szene, Bericht des Übernehmens). `session.setMessage` und `onLoadStatus` schreiben in den Toast-Store; `Message.actions` werden Tasten. `role="status"` für Info, `role="alert"` für Fehler (Tests fragen danach). Compile-Fehler (`CmdError.code === "compile"`) erzeugen keinen Toast, sie gehen in `ProblemsStore` und ins Prüflicht; ein Toast „1 Fehler in Scripts, Build abgebrochen“ mit Taste „Probleme zeigen“ bleibt für den Build.

### 5.8 Palette

⌘K (und der Titelleisten-Knopf) öffnet `Palette`, ein `Dialog` oben mittig (Breite 640) mit `Combobox`: Eingabe filtert sofort über Orte und Header (Name, Kennung, Nummer, aus `buildPlaces`), Textarchive („Textarchiv 0412, Zweiblattdorf“), Scriptdateien (bei übernommenen Scripts) und Befehle (Projekt öffnen, Speichern, Bauen, Hier spielen, Gebiet anlegen…, Raum anlegen…, Liste ein/aus, Thema wechseln, Dichte wechseln, Begriffe, Einstellungen). Treffer in Gruppen mit Bereichssymbol, Kennung grau, höchstens acht je Gruppe; Enter führt aus (`goTo`, Wizard, Befehl), Escape schließt. Ohne Projekt nur Befehle.

### 5.9 Flächen: Start, Technik, Einstellungen, Hilfe

**Start** (`House`): Titel „Willkommen zurück“ (Zen Maru), die letzten fünf Projekte als Karten (Name = Ordnername, Spiel-Plakette, „bearbeitet vor 2 Stunden“, Pfad als Tooltip; Klick öffnet; fehlender Ordner: Karte grau mit „nicht gefunden“ und Taste „Aus der Liste nehmen“), darunter die Tasten „Projekt öffnen ⌘O“ (primär) und ein Hinweis „Projekte sind entpackte DSPRE-Ordner“. Beim Öffnen eines Projekts wechselt der Atlas nach Welt. Ohne Projekt startet die App auf Start; mit `?project=` (Browser) direkt in Welt.

**Technik** (`Cpu`): Liste mit vier Einträgen, rechts die Fläche: *Prüfen und Bauen* (Problemtabelle wie das Panel Probleme, Tasten Bauen und Starten, letzter Build mit Zeit und Pfad), *Patches* (Dynamische Header: Status aus `header_patch_status` mit Plakette Angewendet / Verfügbar / Fehlerhaft, Tasten „Patch anwenden“ und „Patch reparieren“ wie heute im Assistenten; nur Pt und HGSS), *Emulator* (Pfad, Taste „Emulator wählen“), *Werkzeuge* (apicula- und rotom-lsp-Pfad und -Zustand aus den vorhandenen Statusbefehlen, `data-lsp` bleibt). Keine neuen Backend-Fähigkeiten.

**Einstellungen** (`Gear`): Thema (System, Hell, Dunkel als `Segmented`), Dichte (Komfort, Kompakt), Emulator (wie Technik). **Hilfe** (`Question`): öffnet den Glossar-Dialog (Inhalt von `TermsPopover`, `TERMS`), kein eigener Bereich.

## 6. Komponenten (`ui/primitives/`)

Jede Komponente: `Name.tsx`, `Name.module.css`, `Name.test.tsx`. Props englisch, Varianten als String-Union. Base UI liefert Verhalten (Tastatur, ARIA, Fokus, Positionierung), die CSS Modules das Aussehen; `render`-Prop statt `asChild`. Alle Flächen flach (kein Schatten, keine Rundung über `radius/taste` hinaus), nur Schwebendes (`Menu`, `Tooltip`, `Dialog`, `Popover`, Toast) hat Schatten.

| Komponente | Props und Varianten | Grundlage |
|---|---|---|
| `Button` | `kind: "primary" \| "secondary" \| "quiet" \| "play" \| "danger"`, `icon?`, `shortcut?` (zeigt `Kbd`), `size: "m" \| "s"`, `disabled` | `<button>`; sekundär mit doppelter Unterkante (2 px `linie/stark`), play in `live/grund` mit `live/auf` |
| `IconButton` | `label` (Pflicht, wird Tooltip und `aria-label`), `shortcut?`, `pressed?` | Base UI Tooltip + Toggle |
| `Kbd` | `keys: string` („⌘K“), `onHinge?` | `<kbd>` |
| `Icon` | `name` aus einer festen Tabelle `icons.ts` (Phosphor-Komponente je Name), `weight`, `size` aus `dichte/symbol` | `@phosphor-icons/react` |
| `TextField` | `label`, `value`, `onCommit` (Blur oder Enter, wie `DraftInput`), `error?`, `hint?`, `disabled`, `type: "text" \| "search"` | Base UI Field, Input |
| `NumberField` | `label`, `value: number`, `onCommit(n)`, `min`, `max`, `base: "dec" \| "hex"`, `bits?: 8 \| 16 \| 32`, `step`, `hint?`; Scrubbing an der Bezeichnung (Pointer Capture, 1 je 4 px, ⇧ ×10); ein Wert außerhalb `min..max` oder der Bitbreite zeigt `error` „höchstens 65535“ und wird nicht übergeben, nie still begrenzt | Base UI NumberField |
| `SelectField` | `label`, `value`, `options: {value, label, icon?}[]`, `onChange`; unbekannter Wert als Option „Wert 7“ | Base UI Select |
| `Switch`, `Checkbox` | `label`, `checked` (Checkbox auch `"mixed"`), `onChange` | Base UI Switch, Checkbox |
| `Segmented` | `options: {value, label, icon?}[]`, `value`, `onChange`, `aria-label` | Base UI Toggle Group |
| `Reference` | `area: Area`, `label`, `tech?`, `onOpen`, `usedIn?: {label, onOpen}[]` (Rechtsklick-Menü „Verwendet in“) | Base UI Menu |
| `Badge` | `kind: "unsaved" \| "irreversible" \| "locked" \| "changedOnDisk" \| "ptHgssOnly" \| "error" \| "live" \| "info"`, `children?` | `<span>` mit Symbol |
| `Notice` | `kind: "info" \| "success" \| "warning" \| "error"`, `title?`, `children`, `actions?` | `<div role>` |
| `ListRow`, `TreeRow` | `label`, `tech?`, `icon?`, `selected`, `onSelect`, `trailing?`; `TreeRow` zusätzlich `depth`, `expanded`, `onToggle`; Roving Tabindex über `useRovingList(ref)` | `<div role="option" \| "treeitem">` |
| `InspectorRow` | `label` (116 px), `children`, `deviates?` (Kobalt-Strich links), `hint?` | `<div>` |
| `Section` | `title`, `count?`, `open`, `onToggle`, `children` | `<details>`-artig, eigener Kopf |
| `PanelHead` | `title`, `subtitle?`, `actions?` | `<header>` |
| `EmptyState` | `what`, `why`, `action?: {label, onClick}` | `<div>` |
| `Picker` | `title`, `items: {id, label, tech?, area, preview?: ReactNode, count?}[]`, `onPick`, `filters?`, `footer?` | Base UI Combobox in `Dialog`; Suche filtert sofort über `label` und `tech` |
| `Menu`, `Tooltip`, `Dialog`, `Popover` | dünne Hüllen um Base UI mit den Tokens; `Dialog` mit `title` (als Frage formuliert, wenn er vor etwas Nicht-Rückgängigem steht), `actions` | Base UI |
| `DSScreen` | `variant: "textbox" \| "sign" \| "empty"`, `pages: string[]`, `page`, `scale` (ganzzahlig, aus der verfügbaren Breite) | `<div>` 256 × 192, Pixelify Sans 24/32, Textbox unten 2 Zeilen |
| `ToastHost` | liest den Toast-Store (§5.7) | `<section aria-live="polite">` |

Regeln für alle: Fokusring 2 px außen in `fokus/ring` nur bei `:focus-visible`; Ziele mindestens 24 px, in Komfort 32 (`dichte/steuer`); Übergänge nur über `--g4-dauer-*`; `title` nur zusätzlich zu sichtbarem Text, nie allein; Fehlertexte sagen den Ausweg („Die Tür führt ins Leere. Wähle ein Ziel oder lösche die Tür.“); Tasten sagen, was passiert („Figur anlegen“, nicht „OK“). Eine primäre Taste je Ansicht.

`icons.ts` ist die einzige Stelle, die Phosphor importiert; Panels nennen Symbole beim Namen (`<Icon name="door" />`).

**Galerie** (`?gallery` im Browser-Modus, `main.tsx` verzweigt vor `App`): eine Seite je Primitive mit allen Varianten und Zuständen, oben Umschalter Hell / Dunkel und Komfort / Kompakt (sie setzen `data-theme` und `data-dichte` am `html`, weil die generierten Variablen an `:root` hängen; zwei Spalten nebeneinander gingen nur mit einem zweiten Selektor im Export). Dazu je eine Seite „Scharnier“ (Bearbeiten, Assistent) und „Rahmen“ (Atlas, Statusleiste, Titelleiste macOS und Windows). Kein Storybook, keine Abhängigkeit.

## 7. Bereiche und Umzug

### 7.1 Welt

**Liste:** `WorldExplorer` bleibt in seiner Logik (Baum nach Orten, Liste, Suche, Knoten „Scripts“, `+`-Menü für die Assistenten, `revealToken`), bekommt `PanelHead` „Orte“ mit Region als Untertitel (Spielname), `TextField type="search"` „Ort, Raum oder Kürzel suchen“, `TreeRow` für Orte, Außen und Räume (Symbole `MapTrifold`, `Globe`, `House`), `ListRow` in der Listenansicht, `Segmented` Baum | Liste im Kopf, das `+`-Menü als `Menu`. (Kein Ungespeichert-Punkt je Header: die Header-Tabelle ist ein Asset; der Zähler steht in der Titelleiste.) Das Glossar wandert in den Hilfe-Dialog; der Knopf „Begriffe“ im Explorer entfällt.

**Schau:** Dock `welt.schau` mit den Panels `world` („3D“), `matrix` („Raster“), `worldmap` („Weltkarte“) als Reiter einer Gruppe. Über der 3D-Ansicht schwebt oben links eine Leiste (`flaeche/blatt`, Schatten): `Segmented`-artige Toggle-Tasten „Gelände“ (`showPermissions`) und „Ereignisse“ (`showEvents`) mit Symbolen; unten links die Legende als `Popover`-Knopf („Legende“): Gelände-Klassen mit Farbfeld und Name, Ereignis-Arten mit Symbol. Unten rechts bleibt Platz für Zoom (nicht in 3c). Der Hinweis ohne Header wird ein `EmptyState` („Noch kein Ort gewählt.“ / „Wähle links einen Ort, dann zeigt die Schau seine Karten.“ / „Ersten Ort zeigen“). `MatrixPanel` und `WorldMapPanel` behalten Logik und `AreaGrid`, Kopfzeile und Felder gehen auf `PanelHead`, `TextField`, `NumberField`, `Switch`, `Button`; `ExtendAreaDialog` wird ein `Dialog`.

**Hand:** Dock `welt.hand` mit Standard-Layout drei Gruppen nebeneinander: `tools` („Werkzeuge“, 180 px) | `inspector` (wächst; `text`, `script`, `problems` sind Reiter derselben Gruppe) | `context` („An diesem Ort“, 260 px).

*Werkzeuge:* Tasten 40 × 40 (`dichte/werkzeug`) mit Symbol, Name und Kürzel-Ecke: Auswählen (V), Gelände malen (B), Figur (F), Tür (T), Auslöser (A), Schild (S), Gebäude (G). Genau ein Werkzeug ist aktiv (`shellState.tool`, nicht gemerkt, Start Auswählen); Buchstaben schalten um, wenn kein Textfeld den Fokus hat (`isTypingTarget`), Escape zurück auf Auswählen. Darunter ein Satz, was das Werkzeug tut. Verhalten in der Szene (`canvasClicks.ts`): Auswählen klickt Marker und Gebäude, Ziehen verschiebt (wie heute), malt nie; Gelände malen malt wie heute (`paintEnabled` wird `tool === "paint"`), die Ebene Gelände schaltet sich dabei ein; Figur, Tür, Auslöser, Schild rufen bei Klick auf eine Kachel `onAddEvent(kind)` mit dieser Kachel und springen auf Auswählen zurück; Gebäude ebenso `onAddBuilding`. Die heutigen Knöpfe „Neues Gebäude“ und `AddEvent` in der Szene entfallen; eine gesperrte Eventdatei sperrt die vier Event-Werkzeuge mit `lockedText` als Tooltip.

*Inspector:* `Inspector` und die Editoren (`HeaderEditor`, `AreaEditor`, `BuildingEditor`, `EventEditor`, `MatrixCellEditor`, `ExitsSection`, `LevelScriptSection`) behalten ihre Modelle (`InspectorModel`, `FieldSpec`, `*Fields.ts`, `*Commit.ts`) und tauschen die Darstellung: `PanelHead` mit Titel (Event-Titel, „Gebäude 3 von 5“, Ortsname), Untertitel (Kennung, Header) und `Badge` für Ungespeichert und Gesperrt; `Field.tsx` wird zu `InspectorRow` + `NumberField` | `SelectField` | `Switch` je `FieldSpec.kind` (`bool` → Switch, `select` → SelectField, Zahlen → NumberField mit `max` aus dem Spec; `pick` → `Reference` mit Picker); Abschnitte als `Section`; Referenzen (Matrix, Eventdatei, Textarchiv, Scriptdatei, Area-Data, Zielheader) als `Reference` mit Bereichsfarbe und `onOpen = goTo`; `HeaderPicker` wird ein `Picker` mit Ortsname, Kennung und Kartenbild (Thumbnail-Store) als Vorschau. Die Hover-Ansicht (`tile`, `eventHover`) bleibt schreibgeschützt und bekommt denselben Kopf. Das Verhalten „commit on blur, nie je Tastendruck“ und die Draft-Regeln bleiben.

*An diesem Ort:* neues Panel `ContextPanel`: die Events der gezeigten Eventdatei (aus `eventFetch`/Bus, gruppiert nach Art mit `eventTitle` und Kennung, Klick wählt wie ein Klick in der Szene, Doppelklick bei Türen folgt dem Ziel), darunter die Gebäude der gezeigten Karten („Gebäude 3“, Modellnummer), oben die Zähler („5 Figuren, 2 Türen“). Eine gesperrte Eventdatei steht als `Notice kind="warning"` darin. Ohne Header ein `EmptyState`.

*Text, Script, Probleme:* in 3c.1 unverändert innen, als Reiter der Inspector-Gruppe; in 3c.2 nach §7.2 bis §7.4.

### 7.2 Texte (3c.2)

Atlas-Eintrag Texte öffnet den Bereich mit Liste, Schau und Hand; `TextPanel` zerfällt in drei Panels auf demselben Modell (`useTextArchive`, `textModel`), der Zustand (gewähltes Archiv, Nachricht, Suche) liegt in einem Kontext des Bereichs.

- **Liste** „Textarchive“ mit Zähler: Gruppe *Orte* (Archive, die ein Header nutzt; Name = Ortsname des ersten Headers aus `buildPlaces`, Untertitel Kennung und Zahl der Nachrichten) und *Spielweit* (alle anderen, Untertitel Kennung); Suchfeld „Ort oder Archivnummer“ filtert die Liste; `+` im Kopf legt ein Archiv an; Überlängen-Warnungen als Zähler-Plakette an der Zeile und als Summenzeile unten.
- **Schau:** `DSScreen variant="textbox"` mit den Seiten der gewählten Nachricht (Seitentrennung und Steuercodes, wie `textModel` sie für die Vorschau deutet; Unbekanntes als graue Plakette), Zeile „Seite 2 von 3, 256 × 192, 2-fach“, Tasten Zurück / Weiterblättern. Rechts daneben die Seiten als Karten mit Zeichenzahl je Zeile („34 von 38“), die gewählte mit Kobalt-Kante.
- **Hand:** oben Suchfeld „In allen Archiven suchen“ mit `Segmented` Dieses Archiv | Alle und `Button kind="primary"` „Nachricht anlegen“; Tabelle Nr., Text (Steuercodes als Plaketten), Längste Zeile (Balken; `status/warnung` über 38); rechts die Spalte „Nachricht 005“ mit `TextField` mehrzeilig (Undo im Feld bleibt), Zeile „Verwendet in“ mit den Headern, die das Archiv nutzen (`users`), „Letzte entfernen“ als leise Taste. `data-archive`, `data-message`, `data-hit`, `aria-label="Text der Nachricht"`, `aria-label="Texte suchen"` bleiben.

### 7.3 Scripts (3c.2)

Das Script-Panel bleibt ein Hand-Panel der Welt (ein Script öffnet sich vom Header, Event, Explorer-Knoten oder Problem). Innen: `PanelHead` mit Dokumenttitel, Outline als `Menu`, Tasten Speichern und „Script anfügen“, LSP-Zustand als `Badge` (`data-lsp` bleibt) mit „Neu starten“; offene Dokumente bleiben Reiter innerhalb des Panels (Base UI Tabs, Kobalt-Punkt für ungespeichert, `data-script`), nicht Dock-Panels, weil ein Panel je Dokument das Layout-Schema sprengen würde; der Dock-Reiter „Script“ trägt den Punkt, sobald ein Dokument ungespeichert ist. `AdoptBox` wird ein `Notice`-Kasten mit Fortschritt. CodeMirror bekommt ein Thema aus `tokens.ts` (`syntax`, `hell`/`dunkel`; `@codemirror/theme-one-dark` entfällt), Schrift `schrift/familie/code`, und folgt `onThemeChange`.

### 7.4 Probleme (3c.2)

Tabelle auf Tokens, Zeilen mit Schwere-Symbol, Datei als `Reference`, Zeile:Spalte in Mono, Meldung, Quelle als Plakette; `PanelHead` „Probleme“ mit Zähler; `EmptyState` „Keine Probleme.“ Dieselbe Tabelle rendert Technik › Prüfen und Bauen.

### 7.5 Assistenten (3c.2)

`WizardShell` wird `WizardFrame` (§5.5): dieselben `WizardStep`s, derselbe Zustand in `NewAreaWizard` und `NewRoomWizard`; `HeaderChoice`, `PickGrid`, `WizardResult` gehen auf Primitives (`Segmented`, `NumberField`, `Reference`, `Notice`, `Button`). Der Schritt „Zusammenfassung“ zeigt den Satz aus `areaSummary`/`roomSummary` als `Notice kind="info"` und die Plakette „Nicht rückgängig“, wo Karten entstehen. Das Ergebnis bietet „Zeigen“ (heute „Im Studio zeigen“) und „Schließen“.

### 7.6 Technik, Start, Einstellungen, Hilfe (3c.2)

Nach §5.9. Technik und Einstellungen teilen sich die Emulator-Zeile (eine Komponente).

### 7.7 Tiefen

Die Tiefe gilt app-weit (`shellState.depth`). **Bauen** ist die heutige Darstellung. **Tüfteln** ändert: (1) der Inspector zeigt zusätzlich den Abschnitt „Technisch“ mit den rohen Feldschlüsseln (`FieldSpec.key`), Werten dezimal und hex, Asset-Kennung („Eventdatei 0411, Index 3“) und Dateipfad; Zahlfelder zeigen den Dez | Hex-Umschalter (in Bauen nur dezimal); (2) die Explorer-Zeilen zeigen die Kennung immer, nicht nur im Tooltip; (3) das Raster schreibt in jede Zelle Karten-ID, Header-ID und Höhe (heute nur über den Inspector) und bietet eine Spalte „Rohdaten“ der gewählten Zeile (u16 hex) in der Hand; (4) Probleme und Scripts zeigen Dateipfade statt der Labels. (1) und (2) kommen mit 3c.1, (3) und (4) mit 3c.2. Erzählen ist kein Segment, bis es Rezepte gibt.

## 8. Szene und Thema

### 8.1 Farben

`world/colors.ts` liest aus `tokens.ts` (`szene.gelaende`, `szene.ereignis`, `szene.auswahl`): Klassen `block` und Kollision `0x80` → `blockiert`; `water` → `wasser`; `grass` → `hohes-gras`; `ledge` → `vorsprung`; `ice` → `eis`; `sand`, `slide`, `warp`, `door`, `bridge`, `furniture` → `spezial`; unbenannt und `other` → `unbekannt`. Deckkraft der Füllung 42 %, `toneMapped: false`, `depthWrite: false`, `polygonOffset` (wie heute, geprüft). Marker-Platzhalter in `szene.ereignis.<art>`, Auswahl-Halo `szene.auswahl.halo`. Der Szenenhintergrund ist `flaeche/welt` des aktiven Themas (aus `hell`/`dunkel` in `tokens.ts`, gewechselt über `onThemeChange`). Kartenbilder (`thumbnailRenderer`) behalten ihre Farben. Muster (`gelaendeMuster`) brauchen einen Shader statt `setColorAt`; sie kommen mit dem Overlay-Umbau in Phase 6; bis dahin tragen Legende und Hover-Text die Namen.

### 8.2 Thema und Dichte

Hell ist Standard, Dunkel über `data-theme` oder System (§4). Alle Komponenten und Panels sind in beiden Themen geprüft (Galerie). Dockview folgt über `dockview-theme-g4`. CodeMirror folgt in 3c.2 (§7.3). Dichte Kompakt setzt `--g4-dichte-*` kleiner; Werkzeuge verlieren ihren Namen, der Atlas seine Beschriftung.

## 9. Barrierefreiheit

WCAG 2.2 AA als Untergrenze, wie das Design-System: Kontraste über die Tokens (geprüft von `check_tokens.py`), Ziele mindestens 24 px, Fokusring nur bei `:focus-visible`, Roving Tabindex in Explorer, Listen und Werkzeugen (`useRovingList`), Pfeiltasten im Raster (bleibt), `lang="de"`, `prefers-reduced-motion` (Dauern 0, Scharnier blendet statt zu gleiten), `forced-colors` (Systemfarben für Fokus und Akzent über `base.css`), kein Inhalt nur über Farbe (Status mit Symbol und Text, Legende mit Namen) oder nur über Tooltip. Dockview-Reiter und -Sashes bleiben, wie Dockview sie liefert.

## 10. Befehle (Tauri)

Ein neuer Befehl, sonst keine Backend-Änderung:

- `history() -> HistoryDto { undo: Vec<String>, redo: Vec<String> }`: die Labels der Changesets, ältestes zuerst (`Store::history()`, nutzt das vorhandene `label`). Browser-Brücke: Eintrag in `bridge/dispatch.rs`; der Test `commands_match_the_tauri_handler` zählt ihn mit. `api.ts`: `history(): Promise<HistoryDto>`. Das Scharnier ruft ihn nach jeder Änderung (Bus-Thema `edited`/`history`) auf, nicht bei Hover.

## 11. Tests, Galerie, Prüfskripte

- **Primitives:** je Komponente ein jsdom-Test über Rollen und Beschriftungen (Base UI braucht in jsdom einen `ResizeObserver`-Stub und `matchMedia`; `vitest.setup.ts` liefert beide, `vitest.config.ts` lädt es und setzt `environment: "jsdom"` als Standard, `node` bleibt für reine Logik-Tests per Kommentar). Tests fragen nie nach CSS-Klassen.
- **Rahmen:** `shellState` (Lesen, Schreiben, Grenzen 200..560, Einklappen), `Hinge` (Brotkrume aus Auswahl, Prüflicht-Text, Verlauf-Menü, Assistenten-Modus), `Deck` (zwei Docks, Layout-Schlüssel, `isSaneLayout` je Dock), `ToastHost` (5 s, Fehler bleibt, Fortschritt ersetzt), `Palette` (Gruppen, Enter führt aus), `TitleBar` (macOS-Einzug nur in Tauri auf macOS).
- **Umzug:** die bestehenden Tests der Panels und Editoren bleiben die Spezifikation des Verhaltens; sie werden auf die neuen Beschriftungen und Rollen angepasst, nicht gestrichen. `data-*`-Attribute aus §2.2 bleiben. Neue Tests: Werkzeuge (Kürzel, Klick-Semantik je Werkzeug in `canvasClicks`), `ContextPanel`, Tiefe im Inspector.
- **Szene:** `colors.test.ts` prüft die Klassenzuordnung aus §8.1 gegen `tokens.ts`.
- **Prüfskripte im Gate:** „tokens check“ (`check_tokens.py`), „tokens up to date“ (§4), „styles“ (`check-styles.sh`), nach `pnpm test`; `dev-setup.md` beschreibt sie.
- **Galerie:** keine automatischen Tests; der Agent macht Screenshots hell, dunkel, kompakt für das Gate (E).
- **Test-Umgebung:** `docs/agent-gate.md` bekommt den Abschnitt „Doppeldeck: wo was ist“ (Atlas, Liste, Schau, Scharnier, Hand, Werkzeuge mit Buchstaben, Palette) und die Rezepte nennen die neuen Beschriftungen; die Szenarien bleiben unverändert (Backend).

## 12. Risiken und Festlegungen

- **Zwei Dockview-Instanzen** teilen keine Drag-Quelle: ein Panel lässt sich nicht von der Hand in die Schau ziehen. Gewollt. Sollte Dockview 8.6 beim zweiten `DockviewReact` im selben Dokument globale Zustände (Drag-Overlay, `id`s) kollidieren lassen, bekommt jede Instanz eine eigene `id` (Prop `id`); der Plan prüft das in Task „Deck“ zuerst.
- **Base UI 1.9** ist jung: Abweichungen in jsdom (Popups, Fokus) werden mit dem Setup-Stub gelöst, nicht mit Test-Ausnahmen; eine Komponente, die sich nicht testen lässt, wird nicht gebaut, sondern der Plan ändert den Baustein. `render`-Prop ist der Weg, eigene Elemente zu unterlegen.
- **Werkzeug-Semantik** ändert, wie die Szene auf Klicks reagiert (heute malt jeder Klick auf eine Zelle, wenn Gelände sichtbar ist). Die Statusleiste sagt es; das Gate (A) prüft Auswählen und Malen getrennt.
- **Schriftgewicht:** Atkinson Hyperlegible Next ist variabel (eine Datei); Zen Maru Gothic nur 700 und 900, Latin-Teilmenge. Fehlt eine Schrift (CSP, Bundle), fällt `system-ui` ein; das Gate (H) prüft das Bundle.
- **Layout-Schlüssel** `v6` je Bereich und Deck; ältere Layouts werden ignoriert, nicht migriert.
- **Keine Inline-Styles**: die Ausnahmeliste ist in 3c.1 erlaubt und in 3c.2 leer; `/* dyn */`-Zeilen sind die einzige dauerhafte Ausnahme.
- **Undo-Verlauf** zeigt nur Labels, die der Kernel kennt; Änderungen ohne Label („Änderung“) bleiben lesbar. Mehrstufiges Zurückklicken im Menü ruft `undo` n-mal und lädt wie `step`.
- **Keine Nintendo-Assets:** Start-Karten und Galerie benutzen Kartenbilder des geöffneten Projekts oder Platzhalter-Flächen, nie mitgelieferte Grafiken.

## 13. Dokumentation und Roadmap

- `docs/ROADMAP.md`: Zeile 3c zwischen 3b.2 und 3b.3; Status von 3a, 3b.1, 3b.2 auf „Gate bestanden (DSPRE und gepackte App auf Wunsch ausgelassen)“, Phase 2 „durch Gate 3a abgelöst“; die Regel „3b.2 bis 3b.4 beginnen, wenn beide Gates bestanden sind“ gestrichen; ein Abschnitt „Aus dem Design-System, später“: Tiefe Erzählen mit Rezepten und Live-Vorschau (nach 3b.3, als 3d), Bereiche Pokémon und Gegenstände mit Läden (3b.3), Flags und Variablen, Geschichten und Story-Graph (3b.4), Prüfen und Bauen mit Build-Schritten, Spielmodus live und Veröffentlichen (Phase 4), Plugins im Bereich Technik (Phase 5), Gelände malen, Höhen-Overlay, Draufsicht und Overlay-Muster (Phase 6).
- `docs/dev-setup.md`: Abschnitte „Design-System und Tokens“ (`pnpm tokens`, Gate-Schritte, `.fig`-Pflege) und „Galerie“.
- `docs/agent-gate.md`: §11.
- `design-system/README.md`: neue Pfade, Abschnitt „Was die App umsetzt“ mit dem Stand nach 3c (und was §1 ausschließt).
- `docs/phase-3c-gate.md` (mit 3c.2): Ablauf und Ergebnistabelle nach dem Muster der vorigen Gates.
