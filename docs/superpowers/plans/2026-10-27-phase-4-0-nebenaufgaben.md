# Phase 4.0 Nebenaufgaben Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Gen4 Studio bekommt eine eigene Marke als App-Symbol und Vorschaubilder der Projekte auf Start.

**Architecture:** Die Marke entsteht aus einer Geometrie in einem Skript (SVG und 1024-PNG), `pnpm tauri icon` erzeugt daraus die Symbole, `check-bundle.sh` prüft den Hash. Die Vorschau lebt als zwei Dateien unter `.g4/` (eigen, Standard); der Kernel liest, passt ein und schreibt; zwei Befehle laufen gleich über Tauri, Brücke und MCP; die App zeigt die Bilder auf Start und in Technik, setzt sie aus der Szene und erzeugt das Standardbild beim Öffnen.

**Tech Stack:** Rust (g4-kernel, Tauri 2, crate `image` 0.25 mit png und jpeg), React 19 + Vite + vitest, three.js, Python 3 (nur Standardbibliothek) für die Marke, OpenPencil-MCP für die `.fig`.

**Spec:** [docs/superpowers/specs/2026-10-27-phase-4-0-nebenaufgaben-design.md](../specs/2026-10-27-phase-4-0-nebenaufgaben-design.md)

## Global Constraints

- Arbeitsort: Worktree `/Users/POLTEP/Projekte/lab/editor/gen4-studio/.claude/worktrees/phase-4-0`, Branch `phase-4-0-nebenaufgaben`. Nie `cd` in die Hauptkopie, nie Branch wechseln, nie nach main mergen, kein `git stash`.
- cargo braucht `export PATH="$HOME/.cargo/bin:/opt/homebrew/opt/rustup/bin:$PATH"`. `pnpm install` einmal in `apps/studio` (der Worktree hat noch kein `node_modules`).
- Jeder Commit endet mit der Zeile `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>` (ASCII-`@`, kein `＠`).
- Originale unter `/Users/POLTEP/Projekte/lab/editor/Roms` nie öffnen oder ändern; Testprojekte nur frische Kopien unter `Roms/_testkopien/4-0-<zweck>`. Keine ROM-Daten ins Repo.
- Gated Tests: `G4_ROMS_DIR=/Users/POLTEP/Projekte/lab/editor/Roms` (absolut), Symlink `tools` liegt im Worktree (nicht einchecken).
- Test-Umgebung nur auf den Ports 1490/1491 (`G4_VITE_PORT=1490 G4_BRIDGE_PORT=1491`); 1420/1421 und 1430/1431 nie anfassen.
- UI nur aus den Bausteinen von `apps/studio/src/ui/primitives` und Tokens `--g4-*`; keine Hex-Werte in CSS (`scripts/check-styles.sh`, `scripts/check-tokens.sh`).
- Sprache der Oberfläche Deutsch, Texte genau wie in der Spec §3.5 (Menüpunkte, Toasts, Sätze der Fehlercodes).
- Vorschau: genau 256 × 192, PNG RGB8; Dateien `.g4/preview.png` (eigen) und `.g4/cache/preview-standard.png` (Standard).
- Fehlercodes im Kernel-Text: `preview.format`, `preview.too_large`, `preview.unreadable`; Grenzen 32 MiB Datei, 8192 × 8192 Pixel.
- Dateien der parallelen Gutachten-Sitzung nicht anfassen: `docs/research/2026-10-27-phase-4/` und den Abschnitt „Phase 4“ der Roadmap außer dem Punkt „4.0 Nebenaufgaben“.

## Review Focus

- Ein Pfad der Startliste, dessen Ordner fehlt oder kein `.g4/` hat: `project_preview_get { path }` antwortet `source: null` statt eines Fehlers, die Karte zeigt den Namen (Test in Task 2 und Task 4).
- Eine PNG-Datei mit Alpha oder Palette, ein JPEG in CMYK oder Graustufen, ein Bild kleiner als 256 × 192: wird eingepasst, nicht abgelehnt (Test in Task 2).
- Eine Datei mit Endung `.png`, die ein JPEG ist (oder umgekehrt): erkannt am Inhalt (Test in Task 2).
- Ein Projekt ohne Rezept (Diamant/Perl) oder ohne Kartenbild (apicula fehlt): kein Standardbild, keine Meldung, kein Fehler im Öffnen (Test in Task 6).
- Ein Agent über MCP mit `path`: abgelehnt; `file` außerhalb der Wurzeln: abgelehnt (Test in Task 3).

---

### Task 1: Marke und App-Symbol

**Files:**
- Create: `design-system/scripts/marke.py`, `design-system/marke/gen4-studio-marke.svg`, `design-system/marke/gen4-studio-marke-1024.png`, `design-system/scripts/test_marke.py`
- Modify: `apps/studio/src-tauri/icons/*` (neu erzeugt), `apps/studio/scripts/check-bundle.sh`, `docs/release.md`, `design-system/README.md` (neuer Abschnitt „Marke“ nach „Typografie“, Zeile in der Tabelle „Inhalt“)

**Interfaces:**
- Produces: `python3 design-system/scripts/marke.py` schreibt SVG und PNG neu (deterministisch, gleiche Bytes bei gleichem Lauf); Farben werden aus `design-system/tokens/` gelesen (Thema Dunkel für `scharnier/flaeche`, `linie/stark`, `flaeche/blatt`, `linie/standard`, `akzent/grund`, `auswahl/kante`; Szene für `ds/grund` und drei Geländefarben).

- [ ] **Step 1: Test schreiben** `design-system/scripts/test_marke.py` (unittest, nur Standardbibliothek): `test_png_is_1024_rgba` (IHDR 1024 × 1024, Farbtyp 6), `test_colours_are_tokens_on_rgb555` (jede Füllfarbe im SVG ist ein Token-Wert und liegt auf dem RGB555-Raster `(v<<3)|(v>>2)`), `test_geometry` (Körper 824 × 824 bei 100/100, Radius 184; zwei Bildschirme 560 × 300; Scharnier-Kante 16 px hoch in `akzent/grund`), `test_deterministic` (zwei Läufe, gleiche Bytes).
- [ ] **Step 2:** `python3 -m unittest design-system/scripts/test_marke.py` → FAIL (Modul fehlt).
- [ ] **Step 3: `marke.py` schreiben.** Eine Geometrie (Liste von Rechtecken mit Radius und Token-Namen, Maße aus Spec §2.1: Landschaft im oberen Bildschirm aus drei Pixel-Stufen im 8-px-Raster, unten 4 × 3 Kacheln, eine in `auswahl/kante`), daraus (a) das SVG mit `id="token-<name>"` je Fläche, (b) das PNG über einen eigenen Rasterizer mit 4 × 4-Supersampling für Kanten und einem PNG-Writer über `zlib` (keine Pakete). Gestaltung mit dem Frontend-Design-Skill prüfen: Lesbarkeit bei 16 und 32 px (zum Ansehen kleine PNGs in den Scratchpad schreiben, nicht einchecken).
- [ ] **Step 4:** Tests → PASS. Dann `cd apps/studio && pnpm install && pnpm tauri icon ../../design-system/marke/gen4-studio-marke-1024.png`. Prüfen: `git status` zeigt nur geänderte Dateien unter `src-tauri/icons/`; neue Dateinamen (z. B. `ios/`, `android/`, `64x64.png`) nicht einchecken, sondern löschen, außer `tauri.conf.json` verweist darauf.
- [ ] **Step 5: `check-bundle.sh`** vor `echo "bundle ok"`:
  ```bash
  # The bundle carries the brand icon: same hash as icons/icon.icns, and not the create-tauri-app default.
  OLD_DEFAULT_ICNS=3dc10493b7de48a61de58f768f8a5708d3a44a068c148cedf0502b9b9b71ba5d
  WANT=$(shasum -a 256 apps/studio/src-tauri/icons/icon.icns | cut -d' ' -f1)
  HAVE=$(shasum -a 256 "$APP/Contents/Resources/icon.icns" | cut -d' ' -f1)
  ```
  mit zwei Fehlermeldungen („bundle icon.icns differs from apps/studio/src-tauri/icons/icon.icns“, „icons/icon.icns is still the create-tauri-app default“). `bash -n` prüft die Syntax.
- [ ] **Step 6: Dokumente.** `docs/release.md`: Abschnitt „App-Symbol“ (Quelle, Neuerzeugung mit `marke.py` und `pnpm tauri icon`, die neue Prüfzeile in der Liste unter „`scripts/check-bundle.sh` prüft:“). README Abschnitt „Marke“: Konstruktion mit Maßen, Farben als Token-Tabelle, Schutzraum, Hell und Dunkel, kleine Größen, Dateien, Brett „Marke“ in der `.fig` (kommt mit Task 7).
- [ ] **Step 7: Commit** `feat(brand): the Gen4 Studio mark as the app icon` (SVG, PNG, Skript, Test, Icons, check-bundle, Doku).

### Task 2: Kernel `preview`

**Files:**
- Create: `crates/g4-kernel/src/preview.rs`, `crates/g4-kernel/tests/preview_gated.rs` (oder an den bestehenden gated Test des Projektstarts anhängen, Muster dort übernehmen)
- Modify: `crates/g4-kernel/Cargo.toml` (`image = { version = "0.25", default-features = false, features = ["png", "jpeg"] }`), `crates/g4-kernel/src/lib.rs` (`pub mod preview;`), `crates/g4-kernel/src/templates.rs` (eine `pub fn start_town(game) -> Option<&str>` o. ä., falls das Rezept nicht schon öffentlich erreichbar ist)

**Interfaces:**
- Produces (genau so, serde camelCase):
  ```rust
  pub const PREVIEW_W: u32 = 256; pub const PREVIEW_H: u32 = 192;
  #[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)] #[serde(rename_all = "lowercase")]
  pub enum PreviewSource { Eigen, Standard }
  #[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)] #[serde(rename_all = "lowercase")]
  pub enum PreviewSlot { #[default] Eigen, Standard }
  pub struct Preview { pub source: Option<PreviewSource>, pub png: Option<Vec<u8>> }
  pub enum PreviewInput<'a> { Bytes(&'a [u8]), File(&'a Path), Remove }
  #[derive(Serialize)] #[serde(rename_all = "camelCase")]
  pub struct StandardMap { pub header: u16, pub map: u16, pub area_data_id: u16 }
  pub fn read_preview(root: &Path) -> Result<Preview, KernelError>;
  pub fn write_preview(root: &Path, slot: PreviewSlot, input: PreviewInput) -> Result<Preview, KernelError>;
  pub fn fit_preview(bytes: &[u8]) -> Result<Vec<u8>, KernelError>;
  pub fn standard_map(project: &Project) -> Option<StandardMap>;
  ```
  Fehler: `KernelError::Refused("preview.format: …")`, `"preview.too_large: …"`, `"preview.unreadable: …"` (Code vorn, dann englischer Rest).

- [ ] **Step 1: Unit-Tests** in `preview.rs` (`#[cfg(test)]`, Bilder im Test erzeugen mit `image`): `fits_png_4_3`, `crops_square_to_4_3_centre` (linke und rechte Spalte eines 300 × 300-Bildes mit roten Rändern oben/unten: das Ergebnis enthält kein Rot), `fits_jpeg`, `detects_by_content_not_extension` (JPEG-Bytes, `File` mit Endung `.png`), `alpha_lies_on_ds_ground` (voll transparentes PNG → jede Farbe = `szene/ds/grund` aus `tokens.ts`-Wert, als Konstante im Kernel mit Verweis), `upscales_small_image`, `keeps_exact_size` (256 × 192 rein → 256 × 192 raus), `refuses_text_as_format`, `refuses_too_large_dimensions` (Header eines 9000 × 10-PNG genügt), `refuses_missing_file_as_unreadable`, `read_prefers_eigen_over_standard`, `read_ignores_broken_file` (Müll in `preview.png` → Standard oder `None`), `read_without_g4_dir_is_none`, `write_is_atomic_and_creates_g4` (keine `*.tmp` übrig), `remove_without_file_is_ok`, `standard_map_on_synthetic_project` (Synthetic aus `test_support`: `None`, wenn das synthetische Projekt kein Rezept hat; sonst die Zelle — den echten Fall prüft der gated Test).
- [ ] **Step 2:** `cargo test -p g4-kernel preview` → FAIL.
- [ ] **Step 3:** Umsetzen. Einpassen: `image::load_from_memory` mit `guess_format`; nur `Png` und `Jpeg` zugelassen; Dimensionen vor dem vollen Decodieren über `ImageReader::into_dimensions` prüfen; Zuschnitt cover auf 4 : 3 mittig; `resize_exact` mit `Lanczos3` beim Verkleinern, `Nearest` bei ganzzahligem Vergrößern, sonst `CatmullRom`; Alpha über `szene/ds/grund` mischen; PNG RGB8 schreiben. Schreiben: Temp-Datei in `.g4/` + `rename` (Helfer aus `fsutil`, falls vorhanden).
- [ ] **Step 4:** `cargo test -p g4-kernel preview` → PASS; `cargo clippy -p g4-kernel -- -D warnings`; `cargo deny check licenses` falls `cargo deny` installiert ist (sonst im Bericht nennen).
- [ ] **Step 5: Gated Test** `standard_map_names_the_start_towns`: frische Kopien wie die bestehenden gated Tests sie anlegen (Platin DE, HeartGold DE): Header-Name `T01` bzw. `T20` (über `standard_map` und den Header), Karte und AreaData sind gültige Indizes. Lauf: `G4_ROMS_DIR=/Users/POLTEP/Projekte/lab/editor/Roms cargo test -p g4-kernel --features <wie die anderen gated Tests> standard_map` → PASS.
- [ ] **Step 6: Commit** `feat(kernel): project preview, read, fit and write .g4/preview.png`.

### Task 3: Befehle in Tauri, Brücke und MCP

**Files:**
- Create: `apps/studio/src-tauri/src/commands/preview.rs`
- Modify: `apps/studio/src-tauri/src/commands/mod.rs`, `apps/studio/src-tauri/src/lib.rs` (Registrierung), `apps/studio/src-tauri/src/commands/dispatch.rs` (`COMMANDS`, Arme, Tests), `apps/studio/src-tauri/src/mcp/access.rs`, `apps/studio/src-tauri/src/mcp/catalog.rs`, `docs/mcp.md`, `apps/studio/src/project/api.start.ts` (oder ein neues `apps/studio/src/project/api.preview.ts`, exportiert über `api.ts` wie die anderen)

**Interfaces:**
- Consumes: Task 2 (`read_preview`, `write_preview`, `standard_map`, `PreviewSlot`, `StandardMap`).
- Produces:
  ```rust
  #[derive(Serialize)] #[serde(rename_all = "camelCase")]
  pub struct PreviewDto { pub source: Option<PreviewSource>, pub png: Option<String> /* base64 */, pub standard_map: Option<StandardMap> }
  pub fn project_preview_get_impl(state: &AppState, path: Option<&Path>) -> Result<PreviewDto, CmdError>;
  pub fn project_preview_set_impl(state: &AppState, path: Option<&Path>, slot: PreviewSlot, png: Option<&str>, file: Option<&Path>) -> Result<PreviewDto, CmdError>;
  ```
  Tauri-Befehle `project_preview_get(path: Option<String>)` und `project_preview_set(path: Option<String>, slot: Option<PreviewSlot>, png: Option<String>, file: Option<String>)`, beide `spawn_blocking`.
  TypeScript:
  ```ts
  export type PreviewSource = "eigen" | "standard";
  export type PreviewDto = { source: PreviewSource | null; png: string | null; standardMap: { header: number; map: number; areaDataId: number } | null };
  export const projectPreviewGet = (path?: string) => invoke<PreviewDto>("project_preview_get", { path: path ?? null });
  export const projectPreviewSet = (args: { path?: string; slot?: PreviewSource; png?: string; file?: string }) => invoke<PreviewDto>("project_preview_set", { path: null, slot: null, png: null, file: null, ...args });
  ```
- Regeln: `png` und `file` zugleich → Fehler „png and file exclude each other“. Ohne `path` und ohne offenes Projekt → der übliche Fehler „kein Projekt offen“. `standardMap` nur, wenn kein `path` oder `path` gleich der Wurzel des offenen Projekts (kanonisch verglichen). Base64 mit dem `base64`-Crate, falls im Workspace (sonst `data_encoding` o. ä. aus `Cargo.lock`; keine neue Lizenzart).

- [ ] **Step 1: Tests** in `dispatch.rs`: `preview_get_on_open_project_has_standard_map_field` (synthetisches Projekt: `source` null, `png` null), `preview_set_png_then_get_returns_eigen` (PNG 10 × 10 als Base64 → `png` decodiert ist 256 × 192), `preview_set_file_under_root`, `preview_set_remove_falls_back_to_standard`, `preview_get_path_outside_roots_is_refused`, `preview_set_file_outside_roots_is_refused`, `preview_get_path_without_g4_is_empty`, `preview_set_png_and_file_is_refused`. In `access.rs`/`catalog.rs` decken die bestehenden Vollständigkeitstests die Einträge; dazu `mcp_preview_set_with_path_is_refused` (Schema hat kein `path`; ein `path` im Aufruf → Fehler).
- [ ] **Step 2:** `cargo test -p studio preview` → FAIL.
- [ ] **Step 3:** Umsetzen. Brücke: `path` über `policy.read_path` (get) bzw. `policy.write_path` (set) — dazu prüfen, ob `write_path` für einen Projektordner unter `_testkopien` gilt; `file` über `policy.read_path`. `access.rs`: `("project_preview_get", Kind::Read)`, `("project_preview_set", Kind::Irreversible("Vorschaubild ändern"))`. Katalog: zwei Werkzeuge, englische Beschreibung („Read the project's preview picture (256×192 PNG, base64) …“), Schema ohne `path`; `slot` mit `enum ["eigen","standard"]`, Standard `"eigen"`. `docs/mcp.md`: Zeilen in der Werkzeugtabelle. Tauri-Registrierung in `lib.rs`.
- [ ] **Step 4:** `cargo test -p studio` (alles) → PASS, `cargo clippy --workspace -- -D warnings`, `cargo fmt --check`; `cargo test -p studio --test mcp_stdio` → PASS.
- [ ] **Step 5: Commit** `feat(studio): project_preview_get and project_preview_set in Tauri, bridge and MCP`.

### Task 4: Projektkarte auf Start

**Files:**
- Create: `apps/studio/src/ui/frame/ProjectPreview.tsx` (+ `.module.css`, `.test.tsx`), `apps/studio/src/project/usePreviews.ts` (+ `.test.tsx`), `apps/studio/src/project/previewMessages.ts`
- Modify: `apps/studio/src/ui/frame/StartPane.tsx`, `StartPane.module.css`, `StartPane.test.tsx`, `apps/studio/src/project/startService.ts` (Service um `projectPreviewGet`, `projectPreviewSet` erweitern), `apps/studio/src/App.tsx` (Verdrahtung), `apps/studio/src/ui/gallery/pages/ProjectPage.tsx` (+ Test)

**Interfaces:**
- Consumes: Task 3 (`PreviewDto`, `projectPreviewGet`, `projectPreviewSet`).
- Produces:
  ```ts
  // ProjectPreview: the picture in the DS frame, or the name on an empty screen.
  export function ProjectPreview(props: { name: string; png: string | null /* base64 */ }): JSX.Element;
  // usePreviews: loads the preview of each recent path not in `missing`, again whenever `visible` turns true.
  export function usePreviews(paths: readonly string[], missing: ReadonlySet<string>, visible: boolean): { previews: ReadonlyMap<string, PreviewDto>; reload: (path: string) => void };
  export function previewErrorText(e: unknown): string; // maps preview.format|too_large|unreadable to the spec's sentences, else errorText(e)
  export async function choosePreviewFile(): Promise<string | null>; // openPath({ directory: false, title: "Vorschaubild wählen", filters: [{ name: "Bilder (PNG, JPEG)", extensions: ["png", "jpg", "jpeg"] }] })
  ```
  `StartPaneProps` neu: `previews: ReadonlyMap<string, PreviewDto>`, `onChoosePreview: (path: string) => void`, `onRemovePreview: (path: string) => void`.

- [ ] **Step 1: Tests** `StartPane.test.tsx`: `card shows the picture when a preview is there` (ein `img` in der Karte, `alt=""`, `src` beginnt mit `data:image/png;base64,`), `card shows the name without a preview`, `missing card stays grey and has only "Aus der Liste nehmen"`, `menu offers "Vorschaubild wählen…", "Vorschaubild entfernen" only for eigen`, `"Zuletzt" stays on the newest card`. `usePreviews.test.tsx`: lädt je Pfad einmal, überspringt fehlende, lädt beim Sichtbarwerden neu, ein Fehler gibt keinen Eintrag. `previewMessages`: drei Codes → drei Sätze aus Spec §3.5 wörtlich.
- [ ] **Step 2:** `pnpm vitest run src/ui/frame/StartPane src/project/usePreviews` → FAIL.
- [ ] **Step 3:** Umsetzen. `ProjectPreview` nutzt `DSScreen` mit `background` (Bild oder Name wie heute), CSS nur Tokens. Menü der Karte: „Vorschaubild wählen…“, „Vorschaubild entfernen“ (nur `source === "eigen"`), Trenner, „Aus der Liste nehmen“ (Trenner so, wie `Menu` ihn anbietet). App: `onChoosePreview` → `choosePreviewFile` → `projectPreviewSet({ path, file })` → `reload(path)`, Toast „Vorschaubild gesetzt“; `onRemovePreview` → `projectPreviewSet({ path })`, Toast „Vorschaubild entfernt“; Fehler → Toast mit `previewErrorText`. Galerie „Projekt“: Projektkarte in den Zuständen eigen, Standard, Platzhalter, Zuletzt, nicht gefunden; Beispielbilder zeichnet die Galerie auf ein Canvas aus Szenen-Token-Farben (`tokens.ts`), keine Spielgrafik.
- [ ] **Step 4:** Tests → PASS; `pnpm test` gesamt, `bash scripts/check-styles.sh`, `pnpm build`.
- [ ] **Step 5: Commit** `feat(start): project cards show the preview picture, choose or remove it`.

### Task 5: Technik › Projekt

**Files:**
- Modify: `apps/studio/src/technik/TechnikArea.tsx` (+ `.module.css`, `.test.tsx`), `apps/studio/src/App.tsx` (Props)

**Interfaces:**
- Consumes: Task 4 (`ProjectPreview`, `choosePreviewFile`, `previewErrorText`), Task 3 (`projectPreviewGet()`, `projectPreviewSet`).
- Produces: `TechnikEntry` um `"projekt"` erweitert (erster Eintrag der Liste, Label „Projekt“, ein passendes vorhandenes Symbol), Fläche „Projekt“ mit Abschnitt „Vorschaubild“.

- [ ] **Step 1: Tests**: `Projekt shows the preview and "Vorschaubild wählen…"`, `"Entfernen" only with an own picture`, `the hint names the start town` („Ohne eigenes Bild zeigt Start das Kartenbild von Zweiblattdorf.“, wenn der Name bekannt ist — Name aus dem Header von `standardMap.header` über die vorhandene Namensquelle der Welt; sonst „… das Kartenbild der Startstadt.“), `choosing calls set with file and shows the toast`.
- [ ] **Step 2:** `pnpm vitest run src/technik` → FAIL.
- [ ] **Step 3:** Umsetzen (Muster der anderen `Surface`s; Taste sekundär; der gespeicherte Eintrag der Liste darf `"projekt"` sein).
- [ ] **Step 4:** Tests → PASS, `pnpm test`, `check-styles.sh`.
- [ ] **Step 5: Commit** `feat(technik): the project's preview picture`.

### Task 6: Aus der Szene und Standardbild

**Files:**
- Modify: `apps/studio/src/world/thumbnailRenderer.ts` (+ Test), `apps/studio/src/world/scene/SceneHost.ts` (`capture`), `apps/studio/src/ui/frame/dockMenu.ts` (+ Test), `apps/studio/src/app/paletteEntries.ts` (+ Test), `apps/studio/src/App.tsx`
- Create: `apps/studio/src/project/useStandardPreview.ts` (+ `.test.tsx`), `apps/studio/src/project/previewFromScene.ts` (+ Test)

**Interfaces:**
- Consumes: Task 3, Task 4 (`previewErrorText`).
- Produces:
  ```ts
  export function createTopDownRenderer(width = 64, height = width): ThumbnailRender; // 4:3 cuts top and bottom, the map width fills the picture
  // SceneHost
  capture(): string; // renders one frame now and returns a PNG data URL of the canvas
  // dockMenu: TabMenuActions gets `setPreview?: (panelId: string) => void` and `previewLocked?: (panelId: string) => string | null`
  // palette: PaletteCommand gets "previewFrom3d" | "previewFromMap"
  export async function setPreviewFrom3d(host: SceneHost): Promise<PreviewDto>;
  export async function setPreviewFromMap(map: number, areaDataId: number): Promise<PreviewDto>; // renders 256×192 with its own renderer instance
  export function useStandardPreview(projectOpen: boolean, render: (map: number, areaDataId: number) => Promise<string>): void;
  ```
- Texte: Reitermenü „Als Vorschaubild“ (am Reiter „3D“ und „Weltkarte“, nach „Maximieren“ mit Trenner; „Weltkarte“ gesperrt ohne gewählte Karte mit „Erst eine Karte wählen“); Palette „Als Vorschaubild: 3D-Ansicht“, „Als Vorschaubild: Kartenbild der gewählten Karte“ (gesperrt ohne Karte mit dem vorhandenen Hinweis `NO_CELL`); Toast „Vorschaubild gesetzt“.

- [ ] **Step 1: Tests**: Renderer mit 256 × 192 (Mock-WebGL wie in den bestehenden Renderer-Tests: Kamera hat `top - bottom == 0.75 * (right - left)`), `tabMenuItems` für Schau-Reiter 3D und Weltkarte (Einträge, gesperrt ohne Karte), andere Schau-Reiter ohne „Als Vorschaubild“, Palette-Einträge mit Sperre, `useStandardPreview`: schreibt `slot: "standard"` nur, wenn das gerenderte Bild vom gespeicherten abweicht; schreibt nichts ohne `standardMap`; ein Fehler beim Rendern oder Schreiben bleibt still (kein Toast, kein Throw); läuft je geöffnetem Projekt einmal.
- [ ] **Step 2:** `pnpm vitest run src/world src/ui/frame/dockMenu src/app/paletteEntries src/project` → FAIL.
- [ ] **Step 3:** Umsetzen. `SceneHost.capture`: `renderer.render(scene, camera)` und direkt danach `renderer.domElement.toDataURL("image/png")` im selben Tick (ohne `preserveDrawingBuffer`). Base64 aus der Data-URL ohne Präfix an `projectPreviewSet({ png })`. Standardbild: nach dem Öffnen per `requestIdleCallback` (Fallback `setTimeout` 2000 ms), vergleicht Base64-Strings.
- [ ] **Step 4:** Tests → PASS, `pnpm test`, `check-styles.sh`, `pnpm build`.
- [ ] **Step 5: Commit** `feat(world): set the preview from the 3D view or the map picture, and the start town as standard`.

### Task 7: Design-System

**Files:**
- Modify: `design-system/README.md` (Komponente „Projektkarte“ in der Tabelle der Komponenten, „Nachtrag 4.0“, Zeile „Stand“), `design-system/Editor_DesignSystem.fig`, `design-system/openpencil/NACHTRAG-4-0.md` (neu, Vorlagen und Ablauf wie die anderen Nachträge)

- [ ] **Step 1:** README: Zeile „Projektkarte“ (Zustände eigen, Standard, Platzhalter, Zuletzt, nicht gefunden; Menü; Maße 256 × 192 im DS-Rahmen; Galerie „Projekt“), Nachtrag 4.0 mit Marke und Vorschau.
- [ ] **Step 2:** `.fig` per OpenPencil (Ablauf und Grenzen aus `design-system/openpencil/NACHTRAG-3h.md` und der README; `render` mit `replace_id`, nicht Einzeländerungen): Brett „Marke“ auf „Grundlagen“ (SVG der Marke importieren, 1024/128/32/16, Hell/Dunkel, Schutzraum, Maße), Screen „Start“ auf „App: Start und Welt“ mit Bildern auf den Karten (Bilder aus der Test-Umgebung, Task 8), Komponente „Projektkarte“ auf dem Brett der App-Komponenten. Speichern, schließen, `node design-system/scripts/fix_fig_text.mjs design-system/Editor_DesignSystem.fig --rgb555`, wieder öffnen und Bild exportieren zur Kontrolle.
- [ ] **Step 3:** `bash scripts/check-tokens.sh` → ok. Commit `docs(design-system): the mark and the project card (4.0)` (nur README, `.fig`, Nachtrag; keine `beispiele/*.jsx|png`).

### Task 8: Prüfung, Gate-Dokument, Roadmap

**Files:**
- Create: `docs/phase-4-0-gate.md`
- Modify: `docs/ROADMAP.md` (nur Punkt „4.0 Nebenaufgaben“: Status), `docs/agent-gate.md` (falls neue Rezepte für die Vorschau nötig sind)

- [ ] **Step 1:** Frische Kopien `Roms/_testkopien/4-0-pt` (Platin DE) und `4-0-hg` (HeartGold DE) wie in `docs/agent-gate.md`; Test-Umgebung mit `G4_VITE_PORT=1490 G4_BRIDGE_PORT=1491 bash scripts/agent/studio-web.sh`.
- [ ] **Step 2:** G1 bis G4 im Browser durchgehen, je Schritt ein Bild nach `target/agent/gate-2026-10-27/`; dazu Dunkel und Kompakt auf Start. Fehler → Fix-Commit im passenden Task-Bereich, dann wiederholen.
- [ ] **Step 3:** `docs/phase-4-0-gate.md` nach dem Muster von `docs/phase-3b4-gate.md`: Zeilen G1 bis G5 mit Ergebnis und Commit, Abschnitt „Beim Nutzer offen“ mit Hand (Dock, Finder, „Über Gen4 Studio“ der gepackten App) und Blick (Start Hell/Dunkel/Kompakt, Marke in `.fig` und `tauri dev`), Pfade der Bilder.
- [ ] **Step 4:** `G4_ROMS_DIR=/Users/POLTEP/Projekte/lab/editor/Roms bash scripts/gate.sh` im Worktree (im Hintergrund, 20 bis 50 min) → grün; Ergebnis ins Gate-Dokument.
- [ ] **Step 5:** Roadmap Punkt 4.0: „umgesetzt auf Branch `phase-4-0-nebenaufgaben`, gate.sh grün (<Commit>, <Sekunden> s), offen: Hand und Blick“. Trailer zählen: `git log --format='%(trailers:only,unfold)' main..HEAD | grep -c 'Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>'` gleich `git rev-list --count main..HEAD`. Commit `docs(4.0): gate and roadmap`.
