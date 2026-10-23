# Phase 3c.1 „Doppeldeck: Fundament und Welt“ Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Das Design-System „Doppeldeck“ kommt ins Repo und in die App: Tokens, Schriften, Symbole, eine Komponentenbibliothek `ui/` auf Base UI, der Rahmen (Titelleiste, Atlas, Deck aus zwei Docks mit Scharnier, Statusleiste, Toasts) und der Bereich Welt im neuen Raster (Explorer, 3D/Raster/Weltkarte, Werkzeuge | Inspector | An diesem Ort) mit den Tiefen Bauen und Tüfteln.

**Architecture:** `design-system/` (DTCG-Tokens, Skripte, `.fig`) ist die Quelle, `apps/studio/src/ui/tokens/` die eingecheckte Ausgabe; ein Gate-Schritt hält beides gleich. `ui/primitives/` sind dünne, per CSS Modules gestylte Hüllen um Base UI; `ui/frame/` hält Hülle, Scharnier und Docks. `App` behält seine Hooks und Kontexte und tauscht `Toolbar`, `StatusBar` und `Workbench` gegen `Shell` mit `Deck`; die Panels behalten ihre Modelle und bekommen neue Darstellung. Text, Script und Probleme ziehen in die Hand, bleiben innen aber alt (3c.2).

**Tech Stack:** React 19, `@base-ui/react` 1.9.0, `@phosphor-icons/react` 2.1.10, `@fontsource*` 5.3, `dockview-react` 8.6.1, CSS Modules (Vite), vitest 5 + jsdom; Python 3 für den Token-Export; ein Tauri-Befehl `history` (Rust).

**Spec:** [docs/superpowers/specs/2026-10-23-phase-3c-doppeldeck-design.md](../specs/2026-10-23-phase-3c-doppeldeck-design.md)

## Global Constraints

- Die Vanilla-Projekte unter `../Roms/*_DSPRE_contents` werden nie beschrieben; das GUI läuft nur auf Kopien unter `../Roms/_testkopien`.
- Testbefehle aus der Repo-Wurzel mit `export PATH="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$PATH"`. Vor jedem Commit, der Rust berührt: `cargo fmt`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features`; vor jedem Commit in `apps/studio`: `pnpm test`, `pnpm build`.
- UI-Texte deutsch (Begriffe aus `apps/studio/src/workbench/terms.ts`, Schreibweise „Script“), Code, Kommentare und Commit-Messages englisch; Komponenten- und Dateinamen englisch (`Button`, `Hinge`); Token-Namen wie generiert (`--g4-flaeche-blatt`).
- Styling nur über CSS Modules (`Name.module.css`) und `--g4-*`-Variablen; keine Hex-Farben und keine Primitive (`--g4-platin-500`) außerhalb `ui/tokens/` und `world/colors.ts`; kein neues `style={{…}}` in umgezogenen Dateien außer für Laufzeitwerte, in derselben Zeile mit `/* dyn */` markiert.
- Versionen fest: `@base-ui/react@1.9.0`, `@phosphor-icons/react@2.1.10`, `@fontsource-variable/atkinson-hyperlegible-next@5.3.0`, `@fontsource/atkinson-hyperlegible-mono@5.3.0`, `@fontsource/zen-maru-gothic@5.3.1`, `@fontsource/pixelify-sans@5.3.0`. Kein Netz zur Laufzeit; CSP bleibt wie in `tauri.conf.json`.
- Base UI: eigene Elemente über die `render`-Prop unterlegen, nie `asChild`; nur `ui/primitives/` importiert `@base-ui/react`; nur `ui/primitives/icons.ts` importiert `@phosphor-icons/react`.
- Tests fragen nach Rollen, `aria-label`, Texten und den bestehenden `data-*`-Attributen (`data-archive`, `data-message`, `data-script`, `data-header`, `data-hit`, `data-script-file`, `data-lsp`), nie nach CSS-Klassen. Bestehende Panel-Tests werden angepasst, nicht gestrichen.
- Frontend-Invarianten aus 3a: Hover und Meldungen rendern keine Panels neu (`WorkbenchData` über `useShallowStable`, Rahmen-Zustand in eigenem Kontext); wer ein Dock-Panel-Set ändert, zählt den Layout-Schlüssel hoch (hier: `g4.layout.v6.*` in Task 9, `v7` in Task 12).
- Backend: einzige Änderung ist der Befehl `history` (Task 7); jeder neue Tauri-Befehl steht in `bridge/dispatch.rs` (`COMMANDS`, `run!`-Arm, Fixture in `every_command_accepts_the_keys_api_ts_sends`).
- Keine Nintendo-Grafiken im Repo; Vorschauen kommen aus dem geöffneten Projekt (Thumbnail-Store) oder sind Platzhalter-Flächen.
- Der Browser-Modus (`scripts/agent/studio-web.sh`, `?project=`) muss nach jedem Task weiter funktionieren; `pnpm build` darf `__bridge` nicht ins `dist` bringen (Gate-Schritt besteht).

## Review Focus

- Hand-Höhe außerhalb 200..560 im gespeicherten Zustand (Hand verändert, alter Wert): beim Lesen auf den Bereich begrenzen, nie eine Hand von 0 oder 2000 px rendern (Task 8).
- Thema „System“ und ein Systemwechsel zur Laufzeit: `effectiveTheme()` und `onThemeChange` liefern genau einen Wechsel, die Szene färbt den Hintergrund um, ohne Modelle neu zu laden (Task 2, Task 11).
- Zahlfeld mit ungültiger oder zu großer Eingabe („0xZZ“, leer, 70000 bei 16 Bit): kein Commit, Fehlertext mit dem Höchstwert, der alte Wert bleibt; nie still begrenzen (Task 4).
- Werkzeug-Buchstabe und Escape in einem Textfeld: schalten kein Werkzeug um und setzen keines zurück; ein Klick in der Szene mit dem Werkzeug Auswählen malt nie (Task 12).
- Ein gespeichertes Layout eines Docks ist unbrauchbar (Panel fehlt, JSON kaputt): nur dieses Dock fällt auf sein Standard-Layout zurück, das andere bleibt; drei Panels in einer Gruppe zeigen die Reiterleiste, eines nicht (Task 9).
- Toast-Regeln: ein Fehler mit Aktionen überlebt die 5 s, ein Fortschritt gleichen Schlüssels ersetzt den vorigen statt zu stapeln, `setMessage(null)` räumt nur den eigenen Toast ab (Task 5, Task 8).

---

### Task 1: Design-System ins Repo und Token-Pipeline

**Files:**
- Create: `design-system/` (Kopie von `../design-system/`: `tokens/`, `scripts/`, `openpencil/`, `README.md`) und `design-system/Editor_DesignSystem.fig` (Kopie von `../Editor_DesignSystem.fig`)
- Create: `apps/studio/src/ui/tokens/tokens.css`, `apps/studio/src/ui/tokens/dockview-theme.css`, `apps/studio/src/ui/tokens/tokens.ts` (generiert)
- Create: `scripts/check-tokens.sh`
- Modify: `design-system/scripts/export_tokens.py`, `design-system/README.md`, `apps/studio/package.json`, `scripts/gate.sh:97-98`, `.gitignore`
- Delete (im Repo nichts; die Ordner außerhalb bleiben, der Bericht nennt sie zum Aufräumen durch den Nutzer)

**Interfaces:**
- Produces: `pnpm tokens` (in `apps/studio`) schreibt die drei Dateien; `bash scripts/check-tokens.sh` (Repo-Wurzel) prüft Raster und Abgleich, Exit 1 bei Abweichung; Importpfade `./ui/tokens/tokens.css`, `./ui/tokens/dockview-theme.css`, `../ui/tokens/tokens.ts` (`szene`, `hell`, `dunkel`, `syntax`, `rgb555`).

- [ ] **Step 1: Kopieren**

```bash
cp -R ../design-system design-system && cp ../Editor_DesignSystem.fig design-system/Editor_DesignSystem.fig
rm -rf design-system/css design-system/ts
```

- [ ] **Step 2: Export mit Ausgabeordner**

In `export_tokens.py`: `OUT = sys.argv[2] if len(sys.argv) > 2 else None`. Mit `OUT` schreibt das Skript flach `OUT/tokens.css`, `OUT/dockview-theme.css`, `OUT/tokens.ts` (Ordner anlegen); ohne `OUT` wie bisher nach `ROOT/css` und `ROOT/ts`. Docstring und `print` nennen den Zielordner.

- [ ] **Step 3: Skript und Gate**

`apps/studio/package.json`: `"tokens": "python3 ../../design-system/scripts/export_tokens.py ../../design-system src/ui/tokens"`. `scripts/check-tokens.sh`:

```bash
#!/usr/bin/env bash
# Tokens: RGB555 and contrast check, then the committed export must match a fresh one.
set -euo pipefail
cd "$(dirname "$0")/.."
python3 design-system/scripts/check_tokens.py
tmp=$(mktemp -d); trap 'rm -rf "$tmp"' EXIT
python3 design-system/scripts/export_tokens.py design-system "$tmp" >/dev/null
for f in tokens.css dockview-theme.css tokens.ts; do diff "$tmp/$f" "apps/studio/src/ui/tokens/$f"; done
```

`gate.sh`: nach `run_step "pnpm test" …` die Zeile `run_step "tokens" "" bash scripts/check-tokens.sh`. `.gitignore`: `design-system/*.vor-textfix`.

- [ ] **Step 4: Erzeugen und prüfen**

Run: `cd apps/studio && pnpm tokens && cd ../.. && bash scripts/check-tokens.sh; echo exit=$?`
Expected: `exit=0`, drei Dateien unter `apps/studio/src/ui/tokens/`, `tokens.css` beginnt mit dem Generator-Kommentar und enthält `--g4-flaeche-blatt`.

- [ ] **Step 5: Abweichung wird rot**

Run: `sed -i '' 's/--g4-raum-8: 8px/--g4-raum-8: 9px/' apps/studio/src/ui/tokens/tokens.css && bash scripts/check-tokens.sh; echo exit=$?; git checkout apps/studio/src/ui/tokens/tokens.css 2>/dev/null || (cd apps/studio && pnpm tokens)`
Expected: `exit=1` mit einer `diff`-Zeile; danach wieder sauber. (Heißt der Token anders, eine beliebige Zeile von `tokens.css` ändern.)

- [ ] **Step 6: README-Pfade**

`design-system/README.md`: Tabelle „Inhalt“ nennt `apps/studio/src/ui/tokens/` als Ziel der generierten Dateien und `pnpm tokens` als Weg; Abschnitt „Ablauf bei Änderungen“ ergänzt `bash scripts/check-tokens.sh`; `.fig`-Pfad ist jetzt `design-system/Editor_DesignSystem.fig`, der `fix_fig_text.mjs`-Aufruf lautet `node design-system/scripts/fix_fig_text.mjs design-system/Editor_DesignSystem.fig --rgb555` (aus der Repo-Wurzel).

- [ ] **Step 7: Commit**

```bash
git add design-system apps/studio/src/ui/tokens scripts/check-tokens.sh scripts/gate.sh apps/studio/package.json .gitignore
git commit -m "feat(design-system): tokens, scripts and the .fig in the repo; generated tokens in ui/tokens with a gate check"
```

---

### Task 2: Schriften, Symbole, Basis-Stylesheet, Thema, Test-Setup

**Files:**
- Create: `apps/studio/src/ui/tokens/base.css`, `apps/studio/src/ui/tokens/theme.ts`, `apps/studio/src/ui/tokens/theme.test.ts`, `apps/studio/vitest.setup.ts`
- Modify: `apps/studio/package.json` (Abhängigkeiten), `apps/studio/src/main.tsx`, `apps/studio/index.html`, `apps/studio/vitest.config.ts`, `apps/studio/src/workbench/Workbench.tsx:76` (`className="dockview-theme-g4"` statt `theme={themeLight}`)

**Interfaces:**
- Produces (`theme.ts`): `type Theme = "system" | "hell" | "dunkel"`, `type Density = "komfort" | "kompakt"`, `readTheme(): Theme`, `applyTheme(t: Theme): void`, `effectiveTheme(): "hell" | "dunkel"`, `onThemeChange(cb: (t: "hell" | "dunkel") => void): () => void`, `readDensity(): Density`, `applyDensity(d: Density): void`. Schlüssel `g4.theme`, `g4.density`; `applyTheme("system")` entfernt `data-theme`.

- [ ] **Step 1: Abhängigkeiten**

Run: `cd apps/studio && pnpm add @base-ui/react@1.9.0 @phosphor-icons/react@2.1.10 @fontsource-variable/atkinson-hyperlegible-next@5.3.0 @fontsource/atkinson-hyperlegible-mono@5.3.0 @fontsource/zen-maru-gothic@5.3.1 @fontsource/pixelify-sans@5.3.0`

- [ ] **Step 2: Failing test `theme.test.ts`**

```ts
// @vitest-environment jsdom
it("defaults_to_system_and_writes_the_attribute_only_for_a_fixed_theme", () => {
  expect(readTheme()).toBe("system");
  applyTheme("dunkel");
  expect(document.documentElement.dataset.theme).toBe("dunkel");
  expect(localStorage.getItem("g4.theme")).toBe("dunkel");
  applyTheme("system");
  expect(document.documentElement.dataset.theme).toBeUndefined();
});
it("effective_theme_follows_the_system_only_without_a_fixed_theme", () => {
  stubMatchMedia(true); // prefers dark
  applyTheme("system"); expect(effectiveTheme()).toBe("dunkel");
  applyTheme("hell"); expect(effectiveTheme()).toBe("hell");
});
it("on_theme_change_fires_once_per_switch_and_per_system_change", () => { /* applyTheme → 1 call; matchMedia listener fired → 1 call; after unsubscribe → none */ });
it("density_defaults_to_komfort_and_sets_data_dichte", () => { /* applyDensity("kompakt") → dataset.dichte === "kompakt" */ });
```

- [ ] **Step 3: Run, FAIL** (`pnpm vitest run src/ui/tokens/theme.test.ts`: module not found)

- [ ] **Step 4: Implement**

`theme.ts` nach Interfaces (localStorage-Zugriffe in try/catch wie `layout.ts`). `base.css`: `*, *::before, *::after { box-sizing: border-box }`, `html { font: var(--g4-font-text-s); color: var(--g4-text-primaer); background: var(--g4-flaeche-gehaeuse); -webkit-font-smoothing: antialiased }`, `body { margin: 0 }`, `button, input, select, textarea { font: inherit; color: inherit }`, `:focus-visible { outline: 2px solid var(--g4-fokus-ring); outline-offset: 2px }`, `:focus:not(:focus-visible) { outline: none }`, `@media (prefers-reduced-motion: reduce) { :root { --g4-dauer-kurz: 0ms; --g4-dauer-mittel: 0ms; --g4-dauer-scharnier: 0ms } }`, `@media (forced-colors: active) { :root { --g4-fokus-ring: Highlight; --g4-akzent-grund: Highlight; --g4-akzent-auf: HighlightText } }`. `main.tsx`: Schrift-Imports (`@fontsource-variable/atkinson-hyperlegible-next`, `@fontsource/atkinson-hyperlegible-mono/400.css`, `@fontsource/zen-maru-gothic/700.css`, `@fontsource/zen-maru-gothic/900.css`, `@fontsource/pixelify-sans/400.css`), dann `./ui/tokens/tokens.css`, `./ui/tokens/base.css`, `dockview/dist/styles/dockview.css`, `./ui/tokens/dockview-theme.css`; vor dem Render `applyTheme(readTheme()); applyDensity(readDensity())`. `index.html`: `lang="de"`. `vitest.config.ts`: `environment: "jsdom"`, `setupFiles: ["./vitest.setup.ts"]`, `css: false` bleibt Standard. `vitest.setup.ts`: `ResizeObserver`-Stub (observe/unobserve/disconnect leer) und `matchMedia`-Stub (`matches: false`, `addEventListener`/`removeEventListener`), beide nur wenn nicht vorhanden.

- [ ] **Step 5: Run, PASS**; dann die ganze Suite

Run: `pnpm test`
Expected: alle bisherigen Tests grün (Umgebung jsdom als Standard darf nichts brechen; bricht ein reiner Logik-Test, bekommt er `// @vitest-environment node`).

- [ ] **Step 6: Build und Blick**

Run: `pnpm build` grün. Browser-Modus starten (`scripts/agent/studio-web.sh`) und prüfen, dass die Seite in Atkinson rendert (Computed `font-family` am `html`) und Dockview die Klasse `dockview-theme-g4` trägt.

- [ ] **Step 7: Commit** `feat(ui): fonts, base stylesheet, theme and density switches, jsdom test setup`

---

### Task 3: Primitives I: Icon, Button, IconButton, Kbd, Badge, Tooltip, Galerie-Gerüst

**Files:**
- Create: `apps/studio/src/ui/primitives/icons.ts`, `Icon.tsx`, `Button.tsx` + `.module.css` + `.test.tsx`, `IconButton.tsx` (+css, test), `Kbd.tsx` (+css), `Badge.tsx` (+css, test), `Tooltip.tsx` (+css), `apps/studio/src/ui/primitives/index.ts`
- Create: `apps/studio/src/ui/gallery/Gallery.tsx`, `Gallery.module.css`, `pages/ButtonsPage.tsx`
- Modify: `apps/studio/src/main.tsx` (Galerie-Verzweigung)

**Interfaces:**
- Produces: `IconName` (Schlüssel der Tabelle unten), `<Icon name weight?="regular"|"fill" />` (Größe über CSS `1em`, Container setzt `font-size: var(--g4-dichte-symbol)`), `<Button kind="primary"|"secondary"|"quiet"|"play"|"danger" size?="m"|"s" icon?: IconName shortcut?: string>` (natives `<button>`, alle Button-Props durchgereicht), `<IconButton label icon shortcut? pressed? onClick>` (Tooltip = `label` + `shortcut`, `aria-label=label`, `aria-pressed` bei `pressed`), `<Kbd keys onHinge?>`, `<Badge kind="unsaved"|"irreversible"|"locked"|"changedOnDisk"|"ptHgssOnly"|"error"|"live"|"info">children?</Badge>` (Standardtext je `kind`: „Ungespeichert“, „Nicht rückgängig“, „Gesperrt“, „Auf Platte geändert“, „Nur Platin und HGSS“, „Fehler“, „Live“, „Info“; Symbol dazu), `<Tooltip content>{render}</Tooltip>` (Base UI Tooltip, Öffnen nach 400 ms).
- `icons.ts` Tabelle Name → Phosphor-Export: `house→House, world→MapTrifold, stories→ChatsCircle, pokemon→PawPrint, items→Backpack, texts→TextAa, tech→Cpu, help→Question, settings→Gear, save→FloppyDisk, build→Hammer, tinker→Wrench, play→Play, undo→ArrowCounterClockwise, redo→ArrowClockwise, search→MagnifyingGlass, sun→Sun, moon→Moon, place→MapTrifold, outside→Globe, room→House, select→Cursor, paint→PaintBrush, figure→PersonSimpleWalk, door→Door, trigger→Lightning, sign→Signpost, building→Cube, warning→Warning, error→XCircle, info→Info, success→CheckCircle, close→X, plus→Plus, caretDown→CaretDown, caretRight→CaretRight, grip→DotsSixVertical, layers→Stack, legend→ListBullets, lock→Lock, script→Code, matrix→GridFour, pin→MapPin, unsaved→Circle, list→List, tree→TreeStructure, link→ArrowSquareOut, check→Check, minus→Minus, history→ClockCounterClockwise`.

- [ ] **Step 1: Failing tests**

```tsx
// Button.test.tsx
it("renders_the_label_with_shortcut_and_forwards_disabled", () => {
  render(<Button kind="primary" shortcut="⌘S" disabled>Speichern</Button>);
  const b = screen.getByRole("button", { name: /Speichern/ });
  expect(b).toBeDisabled(); expect(b.textContent).toContain("⌘S");
});
// IconButton.test.tsx
it("names_itself_by_the_label_and_shows_it_as_tooltip_on_hover", async () => { /* aria-label "Speichern"; hover → getByRole("tooltip") text "Speichern ⌘S" */ });
it("reports_pressed_state", () => { /* pressed → aria-pressed="true" */ });
// Badge.test.tsx
it("uses_the_default_text_of_its_kind_and_an_icon", () => { /* <Badge kind="irreversible"/> → text "Nicht rückgängig", contains svg */ });
```

(Das Projekt hat keine Testing Library: die Tests rendern wie die bestehenden mit `react-dom/client` und `act` und fragen über `container.querySelector('[role=…]')`, `getAttribute` und `textContent`; `screen.getByRole` oben beschreibt die Absicht.)

- [ ] **Step 2: Run, FAIL**
- [ ] **Step 3: Implement** nach Interfaces. CSS: Button Höhe `var(--g4-dichte-steuer)`, Radius `var(--g4-radius-taste)`, primär `akzent/grund` auf `akzent/auf`, sekundär `flaeche/taste` mit `border: 1px solid var(--g4-linie-stark); border-bottom-width: 2px`, leise ohne Rahmen, play `live/grund` auf `live/auf` fett, danger `status/fehler` auf `akzent/auf`; Hover über `akzent/hover` bzw. `flaeche/deck`; `:disabled` Text `text/deaktiviert`. `index.ts` exportiert alle Primitives (jede spätere Task ergänzt ihn).
- [ ] **Step 4: Galerie** `Gallery.tsx`: Kopf mit `Segmented`-Ersatz aus Buttons (bis Task 4: drei `Button kind="quiet"` für Hell, Dunkel, Kompakt-Schalter, die `applyTheme`/`applyDensity` rufen), links Seitenliste, rechts die Seite; `pages: { id: string; title: string; element: ReactNode }[]` in `gallery/pages/index.ts`; `ButtonsPage` zeigt alle `kind` × Ruhe/Gesperrt, IconButtons, Kbd, alle Badges. `main.tsx`: `if (!inTauri() && new URLSearchParams(location.search).has("gallery")) root.render(<Gallery />) else root.render(<App />)`.
- [ ] **Step 5: Run, PASS**; `pnpm build`; Browser: `http://127.0.0.1:1430/?gallery` zeigt die Seite in hell und nach Klick dunkel (Screenshot im Bericht nennen).
- [ ] **Step 6: Commit** `feat(ui): icon table, button, icon button, kbd, badge, tooltip and the gallery shell`

---

### Task 4: Primitives II: TextField, NumberField, SelectField, Switch, Checkbox, Segmented

**Files:**
- Create: `apps/studio/src/ui/primitives/TextField.tsx`, `NumberField.tsx`, `numberParse.ts`, `SelectField.tsx`, `Switch.tsx`, `Checkbox.tsx`, `Segmented.tsx` (+ je `.module.css`), Tests `TextField.test.tsx`, `NumberField.test.tsx`, `numberParse.test.ts`, `SelectField.test.tsx`, `Segmented.test.tsx`, `Switch.test.tsx`
- Create: `apps/studio/src/ui/gallery/pages/InputsPage.tsx`; Modify `gallery/pages/index.ts`, `Gallery.tsx` (Kopf nutzt jetzt `Segmented`)

**Interfaces:**
- Produces: `<TextField label value onCommit(value: string) type?="text"|"search" error? hint? disabled placeholder? multiline?>` (Commit bei Blur und Enter, Escape stellt den Wert zurück, `data-draft` solange unverändert, wie `DraftInput`; `aria-label=label`, sichtbare Bezeichnung optional über `showLabel`); `<NumberField label value: number onCommit(n: number) min?=0 max? base?="dec"|"hex" bits?: 8|16|32 step?=1 hint? disabled showBase?>` (Höchstwert = `min(max, 2^bits − 1)`; Fehlertext `höchstens ${max}` bzw. `mindestens ${min}`; Scrubbing über Base UI `NumberField.ScrubArea` an der Bezeichnung; `showBase` zeigt den Dez | Hex-Umschalter; Hex-Darstellung `0x` + Großbuchstaben); `parseNumber(text: string, base: "dec"|"hex"): number | null` und `formatNumber(n, base)` in `numberParse.ts`; `<SelectField label value options: {value: number|string; label: string; icon?: IconName}[] onChange disabled>` (unbekannter Wert als Option „Wert 7“); `<Switch label checked onChange>`; `<Checkbox label checked: boolean|"mixed" onChange>`; `<Segmented options: {value; label; icon?}[] value onChange ariaLabel size?>` (Base UI Toggle Group, genau ein Wert).

- [ ] **Step 1: Failing tests**

```ts
// numberParse.test.ts
expect(parseNumber("0x1F", "hex")).toBe(31); expect(parseNumber("1f", "hex")).toBe(31);
expect(parseNumber("0xZZ", "hex")).toBeNull(); expect(parseNumber("", "dec")).toBeNull(); expect(parseNumber("12.5", "dec")).toBeNull();
expect(formatNumber(255, "hex")).toBe("0xFF");
// NumberField.test.tsx
it("commits_on_enter_and_blur_and_reverts_on_escape", …);
it("refuses_a_value_above_the_bit_width_with_the_maximum_in_the_error_and_keeps_the_old_value", () => { /* bits 16, type 70000, Enter → onCommit not called, error text "höchstens 65535" */ });
it("switches_to_hex_and_parses_hex_input", () => { /* showBase, value 255, click "Hex" → input shows "0xFF"; type "0x10", Enter → onCommit(16) */ });
// SelectField.test.tsx: unbekannter Wert → Option „Wert 7“ sichtbar und gewählt
// Segmented.test.tsx: radiogroup-artig: aria-pressed/aria-checked genau einer; Klick ruft onChange
// Switch.test.tsx: role="switch", aria-checked folgt checked
```

- [ ] **Step 2: Run, FAIL**
- [ ] **Step 3: Implement.** Base UI: `Field.Root/Label/Control/Error` für Text und Zahl, `NumberField.Root/ScrubArea/Input` (dec), für Hex ein eigener Eingabepfad über `parseNumber` im selben Feld (Base UI formatiert nur dezimal); `Select.Root/Trigger/Popup/Item`; `Switch.Root/Thumb`; `Checkbox.Root/Indicator`; `Toggle`/`ToggleGroup`. Höhen `dichte/steuer`, Rahmen `linie/standard`, Fokus über `base.css`, Fehler `status/fehler` mit Satz darunter.
- [ ] **Step 4: Run, PASS**; Galerie-Seite „Eingaben“ mit Ruhe, Fokus (Hinweis), Fehler, Gesperrt, Hex, Scrubbing-Hinweis; `pnpm build`.
- [ ] **Step 5: Commit** `feat(ui): text, number (dec/hex, bit width, scrubbing), select, switch, checkbox and segmented fields`

---

### Task 5: Primitives III: Listen, Inspector-Zeile, Struktur, Meldungen, Toasts, Schwebendes, Referenz

**Files:**
- Create: `ListRow.tsx`, `TreeRow.tsx`, `useRovingList.ts`, `InspectorRow.tsx`, `Section.tsx`, `PanelHead.tsx`, `EmptyState.tsx`, `Notice.tsx`, `Menu.tsx`, `Dialog.tsx`, `Popover.tsx`, `Reference.tsx` (+ css) unter `apps/studio/src/ui/primitives/`; `apps/studio/src/ui/frame/toasts.ts`, `toasts.test.ts`, `ToastHost.tsx`, `ToastHost.module.css`, `ToastHost.test.tsx`; Tests `ListRow.test.tsx`, `useRovingList.test.tsx`, `Section.test.tsx`, `Notice.test.tsx`, `Menu.test.tsx`, `Dialog.test.tsx`, `Reference.test.tsx`
- Create: `gallery/pages/ContentPage.tsx`, `gallery/pages/OverlayPage.tsx`

**Interfaces:**
- Produces: `<ListRow label tech? icon? selected onSelect trailing? dataAttrs?>` (`role="option"`, `aria-selected`), `<TreeRow …same… depth expanded? onToggle?>` (`role="treeitem"`, `aria-expanded`, `aria-level`), `useRovingList(ref: RefObject<HTMLElement>, opts?: { orientation?: "vertical"|"horizontal" })` (Pfeiltasten bewegen `tabIndex=0` zwischen `[data-roving]`-Kindern, Home/End), `<InspectorRow label deviates? hint?>children</InspectorRow>` (Bezeichnung 116 px, `deviates` zeichnet 2 px `akzent/grund` links), `<Section title count? open onToggle>children</Section>` (Kopf als Button mit `aria-expanded`), `<PanelHead title subtitle? actions?: ReactNode />` (`<header>`, Titel `font-text-s-betont`, Untertitel Mono grau), `<EmptyState what why action?: {label; onClick}>`, `<Notice kind="info"|"success"|"warning"|"error" title? actions?: {label; onClick}[]>children</Notice>` (`role="status"`, bei `error` `role="alert"`), `toasts.ts`: `toast.info(text, { actions? }): id`, `toast.error(text, { actions? }): id`, `toast.progress(key, text | null): void`, `toast.dismiss(id)`, `subscribeToasts(cb)`, `listToasts()`, `INFO_MS = 5000`; `<ToastHost />` (`<section aria-live="polite">`, höchstens vier, Hover hält die Uhr an); `<Menu trigger: ReactNode items: ({label; icon?; onSelect; disabled?} | "separator")[] />` (Base UI Menu, `role="menu"`/`menuitem`), `<Dialog open onOpenChange title actions?: {label; kind?; onClick}[] description?>children</Dialog>` (Base UI Dialog, Fokusfalle, Escape schließt, `role="dialog"`, `aria-modal`), `<Popover trigger>children</Popover>`, `<Reference area: Area label tech? onOpen usedIn?: {label; onOpen}[]>` mit `type Area = "welt"|"geschichten"|"pokemon"|"gegenstaende"|"texte"|"technik"` (Chip in `bereich/<area>-flaeche`, Text `bereich/<area>`, Klick `onOpen`, Rechtsklick öffnet Menü „Verwendet in“, leer: Eintrag „Nirgends verwendet“ gesperrt).

- [ ] **Step 1: Failing tests** (Auszug; je Komponente ein Rollen-Test)

```ts
// toasts.test.ts (fake timers)
it("info_disappears_after_five_seconds_and_error_stays", …);
it("progress_with_the_same_key_replaces_instead_of_stacking", () => { toast.progress("load","1/3"); toast.progress("load","2/3"); expect(listToasts()).toHaveLength(1); toast.progress("load", null); expect(listToasts()).toHaveLength(0); });
it("keeps_at_most_four_and_drops_the_oldest_info_first", …);
// useRovingList.test.tsx: ArrowDown moves tabIndex 0 to the next [data-roving], End to the last
// Reference.test.tsx: contextmenu → menu with usedIn labels; click → onOpen
// Dialog.test.tsx: open → role dialog with title; Escape → onOpenChange(false)
```

- [ ] **Step 2: Run, FAIL**
- [ ] **Step 3: Implement** nach Interfaces; Schatten nur in `Menu`, `Tooltip`, `Dialog`, `Popover`, Toast (`0 8px 24px color-mix(in srgb, var(--g4-platin-1000) 24%, transparent)` — der einzige erlaubte Nicht-Token-Wert, als Variable `--g4-schatten-schwebend` in `base.css`).
- [ ] **Step 4: Run, PASS**; Galerie-Seiten „Inhalt und Status“ (Listen, Inspector-Zeilen, Section, PanelHead, EmptyState, Notices, Badges, Referenzen aller Bereiche) und „Schwebendes“ (Menü, Tooltip, Dialog, Popover, Toast-Knöpfe); `pnpm build`.
- [ ] **Step 5: Commit** `feat(ui): list and tree rows, inspector row, section, panel head, empty state, notice, toasts, menu, dialog, popover, reference`

---

### Task 6: Picker

**Files:**
- Create: `apps/studio/src/ui/primitives/Picker.tsx`, `Picker.module.css`, `Picker.test.tsx`, `pickerFilter.ts`, `pickerFilter.test.ts`; `gallery/pages/PickerPage.tsx`

**Interfaces:**
- Produces: `interface PickerItem { id: string | number; label: string; tech?: string; area: Area; preview?: ReactNode; count?: string; group?: string }`, `<Picker open onOpenChange title items onPick(item) filters?: {value; label}[] filter? onFilter? placeholder? footer?: ReactNode emptyText? />` (Base UI Combobox im `Dialog`; Suche filtert sofort über `label` und `tech`, Groß/klein egal, Treffer nach `group`; Enter wählt den markierten Treffer, Pfeiltasten bewegen; `filterItems(items, query): PickerItem[]` in `pickerFilter.ts`: Teilstring in `label` oder `tech`, Zahl trifft auch `id` mit führenden Nullen, Reihenfolge: Präfix-Treffer vor Teilstring).

- [ ] **Step 1: Failing tests**

```ts
expect(filterItems(items, "zwei").map(i => i.id)).toEqual([411]);          // label "Zweiblattdorf"
expect(filterItems(items, "T01R0102").map(i => i.id)).toEqual([412]);      // tech
expect(filterItems(items, "0411").map(i => i.id)).toEqual([411]);          // id with zeros
expect(filterItems(items, "").length).toBe(items.length);
// Picker.test.tsx: typing narrows the listbox, Enter calls onPick with the first match, Escape closes
```

- [ ] **Step 2: Run, FAIL** → **Step 3: Implement** → **Step 4: Run, PASS**, Galerie-Seite mit Vorschau-Platzhaltern (`flaeche/welt`-Rechtecke, nie Bilder aus dem Repo), `pnpm build`.
- [ ] **Step 5: Commit** `feat(ui): picker on a combobox in a dialog`

---

### Task 7: Befehl `history`

**Files:**
- Modify: `crates/g4-kernel/src/store.rs` (nach `redo`), Store-Tests in derselben Datei; `apps/studio/src-tauri/src/commands/project.rs`, `apps/studio/src-tauri/src/lib.rs` (`generate_handler`), `apps/studio/src-tauri/src/bridge/dispatch.rs` (`COMMANDS`, `run!`-Arm, Fixture-Liste bei `("dirty", json!({}))`), `apps/studio/src/api.ts` (bei `undo`/`redo`), `apps/studio/src/api.test.ts`

**Interfaces:**
- Produces: Rust `pub fn history(&self) -> (Vec<String>, Vec<String>)` (Undo-Labels ältestes zuerst, Redo-Labels in der Reihenfolge, in der `redo()` sie anwenden würde, also das nächste zuerst); `HistoryDto { undo: Vec<String>, redo: Vec<String> }`; Tauri `history`; TS `interface HistoryDto { undo: string[]; redo: string[] }`, `history(): Promise<HistoryDto>`. Das `#[allow(dead_code)]` an `Changeset.label` entfällt.

- [ ] **Step 1: Failing Rust test** (im Test-Modul von `store.rs`, neben den Undo-Tests): zwei Edits mit Labels „A“, „B“ → `history() == (["A","B"], [])`; nach `undo()` → `(["A"], ["B"])`; nach `redo()` wieder `(["A","B"], [])`.
- [ ] **Step 2: Run, FAIL** (`cargo test -p g4-kernel history`)
- [ ] **Step 3: Implement** Store, `history_impl`, Befehl, Handler, Dispatch (Tabelle, Arm, Fixture), `api.ts`.
- [ ] **Step 4: Failing TS test** in `api.test.ts`: `history()` ruft `invoke("history")` und liefert das Objekt.
- [ ] **Step 5: Run, PASS**: `cargo test --workspace --all-features` (insbesondere `commands_match_the_tauri_handler`, `every_command_accepts_the_keys_api_ts_sends`), `cargo clippy … -D warnings`, `cargo fmt`, `pnpm test`.
- [ ] **Step 6: Commit** `feat(kernel,studio): history command with the undo and redo labels`

---

### Task 8: Hülle: Shell, Titelleiste, Atlas, Statusleiste, Toasts statt Toolbar und StatusBar

**Files:**
- Create: `apps/studio/src/ui/frame/shellState.ts`, `shellState.test.ts`, `ShellContext.tsx`, `Shell.tsx`, `Shell.module.css`, `TitleBar.tsx` (+css, test), `Atlas.tsx` (+css, test), `StatusLine.tsx` (+css, test), `StartPane.tsx` (+css); `apps/studio/src/platform/os.ts` (`isMacTauri(): boolean` = `inTauri() && /Mac/.test(navigator.platform)`), `os.test.ts`
- Modify: `apps/studio/src/App.tsx`, `apps/studio/src/App.test.tsx`, `apps/studio/src-tauri/tauri.conf.json`, `apps/studio/src/app/useProjectSession.ts` (nur: `lastRom` bekommt einen Zeitstempel `lastBuiltAt: number | null`), `apps/studio/src/workbench/panels.tsx` (`StatusPanel` entfällt, Panel `status` aus `layout.ts` und `COMPONENTS`, `LAYOUT_KEY` bleibt bis Task 9)
- Delete: `apps/studio/src/Toolbar.tsx` (der Typ `Message` zieht nach `apps/studio/src/app/message.ts`; alle Importe folgen)

**Interfaces:**
- Produces (`shellState.ts`): `type AreaId = "start"|"welt"|"geschichten"|"pokemon"|"gegenstaende"|"texte"|"technik"|"einstellungen"`, `type Depth = "bauen"|"tuefteln"`, `type Tool = "select"|"paint"|"figure"|"door"|"trigger"|"sign"|"building"`, `interface ShellState { area: AreaId; listOpen: Record<AreaId, boolean>; handHeight: Record<AreaId, number>; handCollapsed: Record<AreaId, boolean>; depth: Depth; recent: RecentProject[] }`, `interface RecentProject { path: string; name: string; game: string; openedAt: number }`, `HAND_MIN = 200`, `HAND_MAX = 560`, `HAND_DEFAULT = 286`, `HAND_COLLAPSED = 48`, `readShellState(): ShellState` (Schlüssel `g4.shell.v1`; fehlende oder ungültige Felder bekommen Standard, `handHeight` wird auf 200..560 begrenzt), `writeShellState(s)`, `rememberRecent(list, p): RecentProject[]` (vorne einsortieren, Duplikat nach Pfad entfernen, höchstens fünf). `ShellContext.tsx`: `useShell(): { state: ShellState; setArea; setListOpen; setHandHeight; toggleHandCollapsed; setDepth; tool: Tool; setTool }` (Werkzeug nicht persistiert). `Shell.tsx`: `<Shell title: ReactNode atlas: ReactNode list?: ReactNode deck?: ReactNode pane?: ReactNode status: ReactNode />` (Grid `40px 1fr 28px` × `88px auto 1fr`; `pane` ersetzt Liste und Deck). `TitleBar`: Props `projectName | null`, `game | null`, `dirtyCount`, `busy`, `onOpen`, `onSave`, `onBuild`, `onHelp`, `theme`, `onToggleTheme`; auf `isMacTauri()` 78 px Einzug und `data-tauri-drag-region`; Palette-Knopf ist in 3c.1 gesperrt mit Tooltip „Kommt mit 3c.2“. `Atlas`: Props `area`, `onArea`, `locked: AreaId[]`, `lockedHint: Record<AreaId, string>`; `role="navigation"`, Einträge `aria-current="page"`. `StatusLine`: Props `left: ReactNode`, `middle: ReactNode`, `right: ReactNode` plus feste Teile `bridge: "none"|"ok"|"missing"`, `density`, `onToggleDensity`, `build: { builtAt: number | null; building: boolean }`.

- [ ] **Step 1: Failing tests** `shellState.test.ts` (Standardwerte; `handHeight` 1000 im Speicher → 560, 10 → 200; `rememberRecent` dedupliziert und kappt bei fünf; kaputtes JSON → Standard), `TitleBar.test.tsx` (Einzug nur bei `isMacTauri()`; Ungespeichert-Text „3 Änderungen nicht gespeichert“ bzw. „Alles gespeichert“; Tasten per Name), `Atlas.test.tsx` (gesperrter Eintrag ist `aria-disabled` und ruft `onArea` nicht), `StatusLine.test.tsx` („Browser-Modus (Brücke)“ bei `bridge:"ok"`, „Brücke nicht erreichbar“ bei `"missing"`, „Noch nicht gebaut“ / „Gebaut vor …“).
- [ ] **Step 2: Run, FAIL**
- [ ] **Step 3: Implement** nach Interfaces. `App`: `ShellProvider` um alles; `Shell` mit `TitleBar`, `Atlas` (gesperrt: geschichten, pokemon, gegenstaende mit „Kommt mit Phase 3b.3“; texte, technik, einstellungen mit „Kommt mit 3c.2“), für `area === "start"` die `StartPane` (Titel „Willkommen zurück“ in `font-titel-s`, Liste `recent` als `ListRow`s mit Name, Spiel, „vor 2 Stunden“, Klick öffnet über `session.openProjectAt`; Taste „Projekt öffnen“ primär; fehlt ein Pfad beim Öffnen, Toast-Fehler und Taste „Aus der Liste nehmen“), für `welt` die bisherige `Workbench` als `deck` (der Explorer bleibt in diesem Task noch ein Dock-Panel; Task 9 zieht ihn in `Shell.list`, Task 10 gestaltet ihn um). Ein erfolgreich geöffnetes Projekt ruft `rememberRecent` und `setArea("welt")`; ohne Projekt startet die App auf `start`, mit `?project=` auf `welt`. `ToastHost` im Shell-Grid unten rechts; `useEffect` in App spiegelt `session.message` in den Toast-Store (`toast.progress("session", null | text)` für Info, `toast.error` für Fehler mit `actions`, `kind:"info"` ohne Aktionen verschwindet nach 5 s und ruft dann `session.setMessage(null)`); `onLoadStatus`-Meldungen laufen über denselben Weg. `tauri.conf.json`: `"width": 1440, "height": 900, "minWidth": 1100, "minHeight": 720, "titleBarStyle": "Overlay", "hiddenTitle": true`. `useShortcuts` bleibt. `App.test.tsx`: „counts unsaved scripts in the toolbar“ prüft jetzt den Text der Titelleiste; „status_bar_shows_the_browser_mode“ die Statusleiste; neue Tests: Start zeigt die letzten Projekte, Öffnen wechselt nach Welt, `session.setMessage(null)` räumt nur den Sitzungs-Toast ab (ein Fehler-Toast mit Aktionen bleibt), eine Info verschwindet nach 5 s (fake timers).
- [ ] **Step 4: Run, PASS** (`pnpm test`), `pnpm build`, Gate-Schritt „dist has no bridge“ von Hand: `grep -rq "__bridge" dist; echo $?` → 1.
- [ ] **Step 5: Blick im Browser-Modus**: `?project=<Kopie>` zeigt Titelleiste, Atlas (Welt aktiv), das alte Dock in der Mitte, Statusleiste mit „Browser-Modus (Brücke)“; Speichern erzeugt einen Toast, der nach 5 s geht; ohne `?project` die Start-Fläche.
- [ ] **Step 6: Commit** `feat(studio): shell with title bar, atlas, start pane, status line and toasts; overlay title bar on macOS`

---

### Task 9: Deck mit zwei Docks und Scharnier

**Files:**
- Create: `apps/studio/src/ui/frame/Deck.tsx`, `Deck.module.css`, `Deck.test.tsx`, `Hinge.tsx`, `Hinge.module.css`, `Hinge.test.tsx`, `breadcrumb.ts`, `breadcrumb.test.ts`, `useHistory.ts`
- Modify: `apps/studio/src/workbench/layout.ts` (+ Test), `apps/studio/src/workbench/Workbench.tsx` → wird zu einem dünnen Aufruf von `Deck` oder entfällt, `apps/studio/src/workbench/Workbench.test.tsx` → `Deck.test.tsx`, `apps/studio/src/workbench/panels.tsx`, `apps/studio/src/App.tsx`, `apps/studio/src/app/useShortcuts.ts` (⌘R), `apps/studio/src/world/buildings.ts` (`shortcutAction` kennt `"play"` für ⌘R, nur wenn `inTauri()`; im Browser lädt ⌘R die Seite)

**Interfaces:**
- Consumes: `history()` (Task 7), `useShell()` (Task 8), `Segmented`, `Button`, `IconButton`, `Menu`, `Kbd` (Tasks 3–5), `ProblemsStore.counts/subscribe`, `countsText`.
- Produces (`layout.ts`): `type DeckId = "schau" | "hand"`, `layoutKey(area: AreaId, deck: DeckId) = \`g4.layout.v6.${area}.${deck}\``, `DECK_PANELS: Record<"welt", Record<DeckId, PanelSpec[]>>` mit `schau: world, matrix, worldmap` (Reiter einer Gruppe) und `hand: inspector` (Anker), `text`, `script`, `problems` (Reiter der Inspector-Gruppe); der Explorer (`headers`) ist kein Dock-Panel mehr: App rendert `HeadersPanel` (alte Optik) als `Shell.list`; `isSaneLayout(layout, panels: PanelSpec[])` prüft gegen die übergebene Liste; `saveLayout(key, json)`, `loadLayout(key)`. `interface Crumb { label: string; tech?: string; onClick?: () => void }`. `Deck`: Props `area: AreaId`, `schauComponents`, `handComponents` (Record wie `COMPONENTS`), `schauRef`, `handRef` (DockviewApi-Refs; `goTo` nutzt beide: ein Ziel wird in dem Dock aktiviert, das sein Panel hat), `hinge: ReactNode`; rendert Schau-Dock (`id="dock-schau"`), `hinge`, Hand-Dock (`id="dock-hand"`, Höhe aus `useShell().state.handHeight[area]`, eingeklappt `HAND_COLLAPSED` → Hand `hidden`); Gruppen mit einem Panel: `group.header.hidden = true`, ab zwei sichtbar (Hooks `onDidAddPanel`, `onDidRemovePanel`, `onDidLayoutChange`). `Hinge`: Props `crumbs: Crumb[]`, `depth`, `onDepth`, `problems: ProblemCounts`, `onProblems`, `history: HistoryDto`, `onUndoTo(stepsBack: number)`, `onRedo`, `onPlay`, `playing: boolean`, `canPlay: boolean`, `onDragHeight(h: number)`, `onToggleCollapse`, `wizard?: ReactNode` (ersetzt den Inhalt, Task 3c.2); `role="toolbar"`, `aria-label="Scharnier"`; Griff mit `data-hinge-grip`, Ziehen mit Pointer Capture setzt die Höhe (begrenzt 200..560), Doppelklick `onToggleCollapse`. `breadcrumb.ts`: `crumbsFor(input: { places: Place[]; headers: HeaderSummary[]; selected: number | null; pinned: InspectorModel; matrixName?: string }): Crumb[]` (höchstens drei: Ort › Header (Ortsname oder „Raum“ + Kennung) › Auswahl (`eventTitle`, „Gebäude 3“, „Zelle 7, 25“); ohne Projekt `[{ label: "Kein Projekt" }]`). `useHistory(bus, revision)`: liest `history()` nach jedem Bus-Thema außer `headerReveal` und nach `step`, debounced 100 ms.

- [ ] **Step 1: Failing tests** `layout.test.ts` (Schlüssel je Bereich und Deck; `isSaneLayout` gegen die Panel-Liste des Decks), `breadcrumb.test.ts` (Ort › Header › Event-Titel; Raum ohne Ortsnamen heißt „Raum“ mit Kennung; ohne Auswahl zwei Glieder; ohne Projekt „Kein Projekt“), `Hinge.test.tsx` (Prüflicht-Text „2 Fehler, 1 Warnung“ und „Alles stimmig“; Verlauf-Zahl und Menü mit Labels, Klick auf den dritten Eintrag → `onUndoTo(3)`; Redo gesperrt bei leerem `redo`; Play gesperrt bei `!canPlay`; Tiefe wechselt), `Deck.test.tsx` (zwei Docks; Standard-Layouts; kaputtes Schau-Layout → nur Schau zurückgesetzt; Gruppe mit einem Panel ohne Reiterleiste, mit zwei Panels mit Reiterleiste).
- [ ] **Step 2: Run, FAIL**
- [ ] **Step 3: Implement.** App: `Deck` für `welt` mit `hinge={<Hinge …/>}`; `onPlay` = `session.onBuild()` dann, wenn `lastRom` gesetzt und `inTauri()`, `session.onRun()`; im Browser nur Bauen und `toast.info("Im Browser-Modus wird nur gebaut")`; `onUndoTo(n)` ruft `step(undo)` n-mal nacheinander; `dockApi`-Ref wird zu `{ schau, hand }`, `useSceneSync.goTo` aktiviert das Panel im passenden Dock. Öffnet `scripts.api` ein Dokument (Wechsel von `active`) und `handHeight.welt < 400`, setzt App `setHandHeight("welt", 400)` (eingeklappt: zuerst aufklappen). `Workbench.tsx` und `WorkbenchProviders` bleiben als Provider; `Dock` entfällt. Depth-Wechsel hat in 3c.1 noch keine Wirkung außer dem Segment (Task 13).
- [ ] **Step 4: Run, PASS**; `pnpm build`; Browser: Scharnier sichtbar, Ziehen ändert die Hand, Doppelklick klappt ein, Prüflicht zeigt Zähler nach einem absichtlichen Compile-Fehler (Script mit Tippfehler speichern), Verlauf zählt Edits, „Hier spielen“ baut.
- [ ] **Step 5: Commit** `feat(studio): deck with a view dock and a hand dock joined by the hinge; breadcrumb, depth, problem light, history, play`

---

### Task 10: Welt: Liste (Explorer) und Hilfe-Dialog

**Files:**
- Modify: `apps/studio/src/workbench/WorldExplorer.tsx` (+ test), `HeaderList.tsx`, `apps/studio/src/App.tsx` (`ListPane` um den Explorer), `apps/studio/src/workbench/panels.tsx` (`HeadersPanel` wird eine normale Komponente `ExplorerPane`)
- Create: `apps/studio/src/ui/frame/HelpDialog.tsx` (+ test; Inhalt aus `TERMS` wie `TermsPopover`), `apps/studio/src/ui/frame/ListPane.tsx` (+css: `PanelHead` + Inhalt + Griff zum Einklappen)
- Delete: `apps/studio/src/workbench/TermsPopover.tsx` (+ test), `useDismiss.ts` falls danach ungenutzt

**Interfaces:**
- Consumes: `TreeRow`, `ListRow`, `useRovingList`, `TextField type="search"`, `Segmented`, `Menu`, `PanelHead`, `EmptyState`, `Dialog`, `useShell().setListOpen`.
- Produces: `<ListPane title subtitle? actions? open onToggle>children</ListPane>`; `WorldExplorer` behält seine Props (§2.2 der Spec) und `data-script-file`, `aria-label="Scripts durchsuchen"`; `HelpDialog` Props `open`, `onOpenChange`.

- [ ] **Step 1: Tests anpassen** `WorldExplorer.test.tsx`: dieselben Fälle (Suche, Baum/Liste, `+`-Menü ruft `onOpenWizard`, Scripts-Knoten, `revealToken` scrollt) über `role="treeitem"`/`"option"` und die Menü-Rolle; Test „Begriffe“ wandert nach `HelpDialog.test.tsx` (öffnet mit allen `TERMS`-Labels). Neuer Test `ListPane`: Griff klappt ein, `onToggle` gerufen.
- [ ] **Step 2: Run, FAIL** → **Step 3: Implement**: Baum mit `TreeRow` (Ort `place`, Außen `outside`, Raum `room`; Kennung über `tech`), Liste mit `ListRow`, Kopf `PanelHead` „Orte“ mit Untertitel (Spielname aus `info`) und Aktionen `Segmented` Baum | Liste, `Menu` „+“ (Gebiet anlegen…, Raum anlegen…); Suchfeld darunter; Scripts-Knoten als `TreeRow` mit Kindern; Hilfe-Dialog aus der Titelleiste (`onHelp`). Inline-Styles der Datei entfernen (`placeColor` bleibt als `/* dyn */` Hintergrund der Orts-Plakette).
- [ ] **Step 4: Run, PASS**; Browser: Liste links von der Schau, Einklappen, Suche, Doppelklick öffnet den Header.
- [ ] **Step 5: Commit** `feat(studio): world explorer as the list pane on tree and list rows; help dialog with the glossary`

---

### Task 11: Welt: Schau (3D-Leiste, Legende, Raster, Weltkarte, Dialog „Gebiet erweitern“) und Szenenfarben

**Files:**
- Modify: `apps/studio/src/workbench/panels.tsx` (`WorldPanel`), `apps/studio/src/workbench/MatrixPanel.tsx` (+ test), `WorldMapPanel.tsx` (+ test), `ExtendAreaDialog.tsx` (+ test), `AreaGrid.tsx` (nur Farben/Schrift), `apps/studio/src/world/colors.ts` (+ test), `apps/studio/src/world/events.ts` (`MARKER_COLORS` aus `szene.ereignis`), `apps/studio/src/world/scene/SceneHost.ts` (Hintergrund aus dem Thema, `setBackground(hex)`), `apps/studio/src/world/MatrixScene.tsx` (hört `onThemeChange`)
- Create: `apps/studio/src/workbench/SceneBar.tsx` (+css, test), `apps/studio/src/workbench/Legend.tsx` (+css, test), `apps/studio/src/world/legend.ts` (+ test: Einträge Gelände und Ereignis mit Name und Farbe)

**Interfaces:**
- Consumes: `szene`, `hell`, `dunkel` aus `ui/tokens/tokens.ts`; `IconButton`, `Popover`, `Button kind="quiet"`, `PanelHead`, `TextField`, `NumberField`, `Switch`, `Dialog`, `Notice`.
- Produces: `permissionColor(kind, collision, cls)` liefert `#RRGGBB` aus `szene.gelaende` nach der Zuordnung der Spec §8.1 (`block`/Kollision → `blockiert`, `water` → `wasser`, `grass` → `hohes-gras`, `ledge` → `vorsprung`, `ice` → `eis`, `sand, slide, warp, door, bridge, furniture` → `spezial`, sonst `unbekannt`); `legendEntries(): { terrain: {key; label; color}[]; events: {kind; label; color}[] }`; `SceneBar` Props `showPermissions`, `showEvents`, `onShowPermissions`, `onShowEvents`, `extra?: ReactNode` (in 3c.1 die beiden Knöpfe „Neues Gebäude“ und `AddEvent` als `Button kind="quiet"`, bis Task 12 sie ersetzt); `Legend` als `Popover`-Knopf unten links; `SceneHost.setBackground(hex: string)`.

- [ ] **Step 1: Failing tests** `colors.test.ts` (Zuordnung aller Klassen; Kollision schlägt Klasse), `SceneHost.test.ts` (`setBackground("#101821")` ändert die Clear-Farbe und lässt `models` und `content` unberührt), `legend.test.ts` (sieben Gelände-, fünf Ereignis-Einträge mit deutschen Namen: Blockiert, Wasser, Hohes Gras, Vorsprung, Eis, Spezial, Unbekannt; Figur, Tür, Auslöser, Schild, Gegenstand), `SceneBar.test.tsx` (Toggles `aria-pressed`), `MatrixPanel.test.tsx`/`WorldMapPanel.test.tsx`/`ExtendAreaDialog.test.tsx` angepasst (Rollen statt Inline-Markup; Dialog über `role="dialog"`).
- [ ] **Step 2: Run, FAIL** → **Step 3: Implement**; Szenenhintergrund: beim Mount `hell["flaeche/welt"]` bzw. `dunkel[…]` nach `effectiveTheme()`, `onThemeChange` ruft `setBackground` und `render()`; Modelle werden nicht neu geladen. `EmptyState` statt des Hinweistexts ohne Header („Noch kein Ort gewählt.“ / „Wähle links einen Ort, dann zeigt die Schau seine Karten.“ / Taste „Ersten Ort zeigen“ wählt `headers[0]`).
- [ ] **Step 4: Run, PASS**; Browser: Overlay-Farben neu, Legende passt zu den Farben, Thema-Wechsel färbt den Hintergrund ohne Neuladen, Raster und Weltkarte lesbar, „Gebiet erweitern“ als Dialog mit Fokusfalle.
- [ ] **Step 5: Commit** `feat(studio): scene bar with layers and legend, token colours in the scene, raster and world map on primitives, extend dialog`

---

### Task 12: Welt: Werkzeuge und „An diesem Ort“

**Files:**
- Create: `apps/studio/src/workbench/ToolsPanel.tsx` (+css, test), `apps/studio/src/workbench/tools.ts` (+ test: `TOOLS: { id: Tool; label: string; key: string; icon: IconName; hint: string; statusHints: {keys: string; does: string}[] }[]`, `toolForKey(key): Tool | null`), `apps/studio/src/workbench/ContextPanel.tsx` (+css, test), `apps/studio/src/workbench/context.ts` (+ test: `groupEvents(events: EventsDto): { kind: EventKind; label: string; rows: {index; title; tech}[] }[]`, `contextCounts(...)` → „5 Figuren, 2 Türen“)
- Modify: `apps/studio/src/workbench/layout.ts` (`v7`; Hand `welt`: `tools` links 180, `inspector` Anker mit `text`, `script`, `problems` als Reiter, `context` rechts 260), `panels.tsx` (Registrierung; `SceneBar.extra` leer), `WorkbenchContext.tsx` (+ `tool`, `onTileClick(tile)`), `app/useWorkbenchModel.ts`, `app/useWorldActions.ts` (`onAddEvent(kind, at?: {x; z})`, `onAddBuilding(fallbackMap, at?: {map; x; y})`), `world/MatrixScene.tsx` und `world/scene/types.ts` (Callbacks `tool`, `onTileClick`), `world/scene/canvasClicks.ts` (+ test), `app/useShortcuts.ts` (+ test: Buchstaben V B F T A S G über `toolForKey`, nicht in Textfeldern, nicht mit Modifier; Escape ohne Auswahl setzt `select`), `ui/frame/StatusLine`-Zuspielung in App (Werkzeug-Hinweise links, Kontext Mitte aus `tileSummary`)

**Interfaces:**
- Consumes: `useShell().tool/setTool`, `fetchEvents(eventFileId)` + Bus `eventsRefetch`/`eventData`, `eventTitle`, `KIND_LABELS`, `lockedText`, `eventEditGate`, `Button`, `PanelHead`, `ListRow`, `Notice`, `EmptyState`, `Kbd`.
- Produces: Klick-Semantik in `canvasClicks` nach Werkzeug: `select` wählt Marker, dann Gebäude, sonst nichts (malt nie); `paint` malt wie heute (`paintEnabled` = `tool === "paint" && !busy`; die Ebene Gelände wird beim Wechsel auf `paint` eingeschaltet); `figure|door|trigger|sign|building` rufen `onTileClick({ mapId, x, y, absX, absZ })`, App ruft `onAddEvent(kind, {x: absX, z: absZ})` (`figure→object`, `door→warp`, `trigger→coord`, `sign→bg`) bzw. `onAddBuilding(null, {map: mapId, x, y})` und setzt das Werkzeug zurück auf `select`. `ToolsPanel`: sieben Tasten 40 × 40 (`dichte/werkzeug`) mit Symbol, Name (in Kompakt nur Symbol), Kürzel-Ecke, `aria-pressed`; Event-Werkzeuge gesperrt mit `lockedText` als Tooltip, wenn `eventLock` gesetzt oder kein Header gewählt; darunter der `hint`-Satz des aktiven Werkzeugs. `ContextPanel`: Kopf „An diesem Ort“ mit Zähler, Gruppen je Art mit `ListRow`s (Klick `onSelectEvent`, Doppelklick bei Warps `goTo({kind:"header", id: dest})`), Abschnitt „Gebäude“ (Zahl je Karte aus dem Scene-Host, Klick `onSelectBuilding`), `Notice kind="warning"` bei Sperre, `EmptyState` ohne Header.

- [ ] **Step 1: Failing tests** `tools.test.ts` (`toolForKey("f") === "figure"`, Großbuchstabe gleich, `"x"` null), `canvasClicks.test.ts` (mit `dragTestKit`: Werkzeug `select` + Klick auf freie Kachel → kein `setPermission`; `paint` → `setPermission`; `door` → `onTileClick` mit Kachel), `useShortcuts.test.tsx` (Buchstabe in einem `input` schaltet nicht; `b` außerhalb schaltet auf `paint`; Escape ohne Auswahl → `select`), `ToolsPanel.test.tsx` (sieben Tasten, aktives `aria-pressed`, gesperrte Event-Werkzeuge mit Sperrtext), `context.test.ts` (Gruppen und Zählertext „2 Figuren, 1 Tür“; Singular/Plural), `ContextPanel.test.tsx` (Klick wählt; Warp-Doppelklick folgt).
- [ ] **Step 2: Run, FAIL** → **Step 3: Implement**; Statusleiste: links `TOOLS[tool].statusHints` als `Kbd`-Paare („Klick“ wählt, „Ziehen“ verschiebt …), Mitte `tileSummary` des Hover, sonst Ort und Zelle. Die Knöpfe „Neues Gebäude“ und `AddEvent` in der Schau entfallen (`AddEvent.tsx` wird gelöscht oder von `ToolsPanel` genutzt).
- [ ] **Step 4: Run, PASS**; Browser: F dann Klick setzt eine Figur und springt zurück auf Auswählen; V-Klick malt nicht; B-Klick malt; die Kontextspalte listet die Events des Headers, Klick wählt im Inspector; `g4emu`-Szenario `pt-treppe-warp.yaml` weiter grün (Backend unverändert, nur Smoke).
- [ ] **Step 5: Commit** `feat(studio): tool palette with letter shortcuts and click semantics per tool; context panel "An diesem Ort"`

---

### Task 13: Inspector auf Primitives

**Files:**
- Modify: `apps/studio/src/workbench/Inspector.tsx`, `workbench/inspector/Field.tsx` (+ test) → nutzt `InspectorRow` + `NumberField` | `SelectField` | `Switch` | `Reference`, `HeaderEditor.tsx` (+ test), `AreaEditor.tsx`, `BuildingEditor.tsx`, `EventEditor.tsx` (+ test), `MatrixCellEditor.tsx`, `ExitsSection.tsx` (+ test), `LevelScriptSection.tsx` (+ test), `HeaderPicker.tsx` (+ test) → `Picker` mit `useThumbnail` als Vorschau, `AddEvent.tsx` falls noch vorhanden
- Create: `apps/studio/src/workbench/inspector/InspectorHead.tsx` (+ test: Titel, Untertitel, Badges Ungespeichert/Gesperrt aus Props), `apps/studio/src/workbench/inspector/referenceFor.ts` (+ test: `referenceFor(spec: FieldSpec, value: number, headers, …): { area: Area; label: string; tech?: string; target: NavTarget }`)

**Interfaces:**
- Consumes: alle Primitives; `FieldSpec` (`u8`→`NumberField bits=8`, `u16`→`bits=16`, `bool`→`Switch`, `select`→`SelectField`, `pick`→`Reference` mit `onOpen = goTo(target)` und Taste „Ändern…“, die den `Picker` öffnet; `pickMax` bleibt die Obergrenze); `InspectorModel` unverändert.
- Produces: unveränderte Props der Editoren (die `*Context`-Typen bleiben), neue Darstellung; `Field` behält Commit-Regeln (Blur/Enter, Draft, Unmount-Commit) und die Tests dafür.

- [ ] **Step 1: Tests anpassen**: `fields.test.ts`, `Field.test.tsx`, `HeaderEditor.test.tsx`, `EventEditor.test.tsx`, `ExitsSection.test.tsx`, `LevelScriptSection.test.tsx`, `HeaderPicker.test.tsx` auf Rollen (`spinbutton`/`textbox` je nach Base UI, `switch`, `combobox`, `button` mit Namen); jeder Test behält seine Aussage. Neue Tests `InspectorHead.test.tsx`, `referenceFor.test.ts` (Matrix → `welt`, Textarchiv → `texte`, Scriptdatei → `welt`, Area-Data → `welt`, Zielheader → `welt` mit Ortsname und Kennung).
- [ ] **Step 2: Run, FAIL** → **Step 3: Implement**: `InspectorHead` je Modell (Event: `eventTitle` + „Figur 3 von 5 in <Ort>“; Gebäude: „Gebäude 3 von N“; Header: Ortsname + Kennung; Zelle: „Zelle x, y“ + Matrixname; Hover-Modelle mit Badge „Nur Ansicht“ = `kind:"info"`), Felder in `Section`s (Header: „Karten und Raster“, „Aussehen“ (Musik, Wetter, Kamera, Kampf), „Texte und Scripts“, „Eingänge / Ausgänge“ (ExitsSection), „Level-Script“), Referenzen als `Reference`, Picker für Zielheader mit Kartenbild. Alle Inline-Styles dieser Dateien weg.
- [ ] **Step 4: Run, PASS**; Browser: Event wählen, Blickrichtung über `SelectField` ändern, ⌘Z, Header-Felder, Zielheader über den Picker mit Bild wählen; `call.sh get_header` zeigt den neuen Wert.
- [ ] **Step 5: Commit** `feat(studio): inspector editors on inspector rows, number/select/switch fields, references and the header picker`

---

### Task 14: Tiefe Tüfteln im Inspector und Explorer

**Files:**
- Create: `apps/studio/src/workbench/inspector/TechnicalSection.tsx` (+ test), `apps/studio/src/workbench/inspector/technical.ts` (+ test: `technicalRows(model: InspectorModel, header?: MapHeader, …): { key: string; dec: number; hex: string }[]`, `assetLine(model): string` z. B. „Eventdatei 0411, Index 3“, `filePath(model, info): string | null` z. B. `unpacked/eventFiles/0411`)
- Modify: `Inspector.tsx` (Abschnitt „Technisch“ nur bei `depth === "tuefteln"`; `NumberField showBase` nur dann), `WorldExplorer.tsx` (+ test: Kennung in jeder Zeile sichtbar bei `tuefteln`, sonst nur im Tooltip), `App.tsx`/`useWorkbenchModel.ts` (`depth` in `WorkbenchData`)

- [ ] **Step 1: Failing tests** `technical.test.ts` (Zeilen für einen Pt-Header: alle `FieldSpec.key`s mit dez und `0x`-hex; `assetLine` für Event und Gebäude; `filePath` je Modell), `TechnicalSection.test.tsx` (zugeklappt per Default, Kopf „Technisch“), Inspector-Test: Abschnitt fehlt in `bauen`, da in `tuefteln`; Explorer-Test: Kennung sichtbar nur in `tuefteln`.
- [ ] **Step 2: Run, FAIL** → **Step 3: Implement** → **Step 4: Run, PASS**; Browser: Tiefe im Scharnier umschalten, Abschnitt erscheint, Hex-Umschalter an Zahlfeldern, Kennungen im Explorer.
- [ ] **Step 5: Commit** `feat(studio): tinker depth shows the technical section, hex fields and identifiers`

---

### Task 15: Stil-Prüfung, Reste, Dokumentation, Roadmap

**Files:**
- Create: `scripts/check-styles.sh`, `scripts/check-styles.allow`
- Modify: `scripts/gate.sh` (Schritt „styles“ nach „tokens“), `docs/dev-setup.md` (Abschnitte „Design-System und Tokens“, „Galerie“, Gate-Liste um zwei Schritte), `docs/agent-gate.md` (Abschnitt „Doppeldeck: wo was ist“ vor „Werkzeuge“; Rezepte: „Warp hinzufügen“ → Werkzeug Tür (T) und Klick, „Neues Gebäude“ → Werkzeug Gebäude (G); „Begriffe“ → Hilfe in der Titelleiste), `docs/ROADMAP.md` (Zeile 3c „in Arbeit (3c.1 implementiert, Gate offen)“ zwischen 3b.2 und 3b.3; Status 3a, 3b.1, 3b.2 „Gate bestanden, DSPRE und gepackte App auf Wunsch ausgelassen“; Phase 2 „durch Gate 3a abgelöst“; Regel „3b.2 bis 3b.4 beginnen, wenn beide Gates bestanden sind“ gestrichen; Abschnitt „Aus dem Design-System, später“ nach Spec §13), `design-system/README.md` (Abschnitt „Was die App umsetzt“), `apps/studio/README.md` falls er die Struktur beschreibt
- Modify: alle in Tasks 8–14 umgezogenen Dateien, in denen noch `style={{` ohne `/* dyn */` oder Hex steht (die Prüfung findet sie)

**Interfaces:**
- Produces: `bash scripts/check-styles.sh` Exit 1 mit Dateiname:Zeile je Fund; Regeln: (1) `style={{` in `apps/studio/src/**/*.tsx`, (2) `#[0-9a-fA-F]{3,8}\b` in `*.tsx|*.ts|*.css` unter `apps/studio/src`; ausgenommen `ui/tokens/`, `world/colors.ts`, Zeilen mit `/* dyn */`, Testdateien (`*.test.*`), und die Dateien aus `scripts/check-styles.allow` (eine je Zeile, Glob erlaubt; 3c.1-Inhalt: `apps/studio/src/workbench/text/*`, `apps/studio/src/workbench/script/*`, `apps/studio/src/workbench/problems/*`, `apps/studio/src/workbench/wizards/*`, `apps/studio/src/app/WizardHost.tsx`, `apps/studio/src/platform/WebDialogHost.tsx`, `apps/studio/src/workbench/DraftInput.tsx`).

- [ ] **Step 1: Skript schreiben, laufen lassen**: `bash scripts/check-styles.sh` listet die Reste; jede gefundene Zeile außerhalb der Allow-Liste wird auf Tokens oder `/* dyn */` umgestellt, bis Exit 0.
- [ ] **Step 2: Gegenprobe**: eine Hex-Farbe in `Atlas.module.css` eintragen → Exit 1 mit der Zeile; zurücknehmen.
- [ ] **Step 3: Gate-Schritt** `run_step "styles" "" bash scripts/check-styles.sh` nach „tokens“; Doku-Änderungen nach der Dateiliste.
- [ ] **Step 4: Volle Prüfung**: `pnpm test`, `pnpm build`, `cargo test --workspace --all-features`, dann `scripts/gate.sh` im Hintergrund (etwa 25 min); Ergebnis-Tabelle im Bericht.
- [ ] **Step 5: Commit** `chore(studio): style check in the gate, docs for tokens, gallery and the Doppeldeck layout, roadmap row 3c`

---

## Abschluss

Nach Task 15: `superpowers:finishing-a-development-branch` (Merge nach `main` nach grünem `gate.sh`). Der Plan 3c.2 (Texte, Scripts, Probleme, Assistenten im Scharnier, Start-Karten mit Bildern, Technik, Einstellungen, Palette, Tüfteln im Raster, Allow-Liste leer, Gate-Dokument und Abnahme) wird danach aus derselben Spec geschrieben.
