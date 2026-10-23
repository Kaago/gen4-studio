# Phase 3c.2 „Doppeldeck: Texte, Scripts, Technik, Assistenten“ Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Die restlichen Bereiche und Panels ziehen ins Doppeldeck: Texte als eigener Bereich mit DS-Vorschau, Scripts und Probleme auf den Primitives, Assistenten als Schrittleiste im Scharnier, Start-Karten, Technik, Einstellungen, die Befehlspalette ⌘K, Tüfteln im Raster; die Stil-Ausnahmeliste wird leer, das Gate-Dokument entsteht.

**Architecture:** Baut auf 3c.1 auf (`ui/primitives`, `ui/frame` mit `Shell`, `Deck`, `Hinge` (Slot `wizard`), `ListPane`, `StartPane`, `shellState`, `DECK_PANELS`, `layoutKey`). Ein Bereich ist ein Paar aus Liste und Deck; `Shell` hält alle Bereiche gemountet und blendet die inaktiven mit `hidden` aus. Assistenten rendern per Portal in Slots des Scharniers und der Hand, ihr Zustand bleibt in `NewAreaWizard`/`NewRoomWizard`. Die Palette ist der `Picker` in der Variante `palette`. Kein neuer Backend-Befehl; Technik liest Zustände aus kleinen Stores (`useSyncExternalStore`), die die vorhandenen Hooks füllen.

**Tech Stack:** React 19, `@base-ui/react` 1.9.0 (Tabs, Combobox über `ui/primitives`), CodeMirror 6 (Thema aus `tokens.ts`), `dockview-react` 8.6.1, CSS Modules, vitest 5 + jsdom, Python 3 für den Token-Export.

**Spec:** [docs/superpowers/specs/2026-10-23-phase-3c-doppeldeck-design.md](../specs/2026-10-23-phase-3c-doppeldeck-design.md) (§1 Gate, §5.5 Assistenten, §5.7 Toasts, §5.8 Palette, §5.9 Flächen, §6, §7.2–§7.7, §8.2, §9, §11). Gestaltung: `design-system/README.md`, `design-system/openpencil/BRIEFING-SCREENS.md` (Screens „Texte: Archiv 0412“, „Befehlspalette“, „Start“, „Ort anlegen: Anordnung“, „Technik“, „Einstellungen“), Maße der DS-Box in `design-system/openpencil/jsx-lib.js`.

Diese Fassung ersetzt den Entwurf vom selben Tag; sie wurde gegen den Stand des Branches `phase-3c1-doppeldeck` (Commit f88c22b) geprüft. Alle Pfade unten sind relativ zu `apps/studio/src`, wenn nicht anders gesagt.

## Global Constraints

- Wie Plan 3c.1: Vanilla-Projekte nie beschreiben; Testbefehle aus der Repo-Wurzel mit `export PATH="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$PATH"`; vor jedem Commit in `apps/studio`: `pnpm test`, `pnpm build`, `bash scripts/check-styles.sh`; UI-Texte deutsch, Code und Commits englisch; Styling nur über CSS Modules und `--g4-*`; keine Hex-Farben und keine Primitive außerhalb `ui/tokens/` und `world/colors.ts`; `style=` nur für Laufzeitwerte mit `/* dyn */` in derselben Zeile; `@base-ui/react` nur in `ui/primitives/`; `@phosphor-icons/react` nur in `ui/primitives/icons.ts`; Tests fragen nach Rollen, `aria-label`, Texten und `data-*`, nie nach CSS-Klassen.
- Bestehende `data-*`-Attribute bleiben: `data-archive`, `data-message`, `data-hit`, `data-script`, `data-lsp`, `data-script-file`, `data-header`. Bestehende Panel- und Assistenten-Tests werden angepasst, nicht gestrichen.
- **Kein Backend-Befehl kommt hinzu**; die Brücke bleibt wie nach 3c.1. Was die Oberfläche nicht aus vorhandenen Befehlen weiß (Nachrichtenzahl je Archiv ohne Laden, Pfad von apicula), zeigt sie nicht.
- Layout-Schlüssel: `layoutKey` in `workbench/layout.ts` ist eine Vorlage für alle Bereiche; sie steigt einmal von `g4.layout.v7.` auf `g4.layout.v8.` (Welt-Hand verliert `text`, Texte kommt dazu).
- Texte des Design-Systems: Tasten sagen, was passiert („Letzte Nachricht entfernen“, nicht „Letzte entfernen“); Plaketten ausgeschrieben („Nur Platin und HGSS“); keine Mittelpunkt-Ketten („A · B · C“) in Fließtext, Kommas oder eigene Spalten.
- Am Ende ist `scripts/check-styles.allow` leer und `bash scripts/check-styles.sh` grün; die Prüfung erkennt dann auch `style={konstante}`.
- Jeder Task nimmt die Einträge aus `scripts/check-styles.allow`, deren Dateien er umzieht, selbst heraus (nicht erst Task 9).

## Review Focus

- Assistent offen, Fokus in einem Zahlfeld mit geändertem Draft, Enter: das Feld übergibt, der Assistent geht nicht weiter; ein zweites Enter geht weiter. Escape in einem Feld mit Draft nimmt den Draft zurück, erst das zweite Escape bricht ab (Task 5a).
- Assistent offen, ein Gebäude ist in der Szene gewählt, Entf: nichts wird gelöscht; ⌘Z, ⌘S, ⌘R und Werkzeugbuchstaben sind aus (Task 5a).
- Script öffnet sich aus Technik oder Texte heraus (Problem anklicken): die App wechselt nach Welt, die Hand ist mindestens 400 px hoch, das Dokument ist sichtbar (Task 2a, Task 3b).
- Textarchiv mit 600 und mehr Nachrichten: die Tabelle rendert nur sichtbare Zeilen plus Puffer; die Suche „Dieses Archiv“ filtert ohne spürbare Verzögerung; das Rezept im Browser misst mit `performance.now()` (Task 2b).
- Thema „System“ und Systemwechsel zur Laufzeit: CodeMirror tauscht nur den `HighlightStyle`, das Dokument bleibt samt Undo-Verlauf; `EditorView.theme` folgt über `var(--g4-*)` von selbst (Task 3a).

---

### Task 1a: Tokens für den DS-Bildschirm, Seiten im Textmodell

**Files:**
- Modify: `design-system/tokens/` (Sammlung Szene, neue Gruppe `ds`), `design-system/README.md` (Abschnitt Szene: Gruppe `ds`)
- Modify (generiert, per `pnpm tokens`): `ui/tokens/tokens.css`, `ui/tokens/tokens.ts`, `ui/tokens/dockview-theme.css`
- Modify: `workbench/text/textModel.ts`, `workbench/text/textModel.test.ts`

**Interfaces:**
- Produces Tokens: `szene/ds/grund` (Spielfläche hinter der Box), `szene/ds/textbox/flaeche`, `szene/ds/textbox/rahmen`, `szene/ds/textbox/text`, `szene/ds/schild/flaeche`, `szene/ds/schild/rahmen`, `szene/ds/schild/text`, `szene/ds/pfeil`; Werte RGB555-treu aus `jsx-lib.js` (Textbox `#F7F7FF`/`#4A5263`/`#394252`, Schild `#E7D6B5`/`#6B5229`/`#423921`, Pfeil wie Textbox-Rahmen, Grund wie `flaeche/bildschirm` des hellen Themas), Kontrast Text auf Fläche ≥ 4.5 (`check_tokens.py`). In CSS `--g4-szene-ds-textbox-flaeche` usw., in `tokens.ts` unter `szene.ds`.
- Produces: `textModel.pages(text: string): Page[]` mit `Page = { lines: Segment[][]; prompt: boolean; overflow: boolean }` (`Segment` wie `segments()`); `textModel.lineWidth(line: Segment[]): number` (Zeichen ohne `code`-Segmente; `[PK]` und `\xNNNN` zählen eins).
- Regeln (Entscheidung (a) des Design-Reviews, Quelle `docs/formats/text_archive.md` und chatot): `\n` (Textform `\\n`) in Zeile 1 wechselt nach Zeile 2; `\n` in Zeile 2 beginnt eine neue Seite mit `overflow: true`; `\r` (0x25BC) beendet die Seite mit `prompt: true`, die nächste beginnt leer; `\f` (0x25BD) beendet die Seite mit `prompt: true`, die nächste beginnt mit der bisherigen Zeile 2 als Zeile 1; ein `\r`/`\f` am Textende erzeugt keine leere Seite; leerer Text ergibt eine Seite mit einer leeren Zeile.
- In `textModel.ts` sind die Namen für `\r` und `\f` heute vertauscht (`BREAKS = {r: "scroll", f: "clear"}`): `\r` ist `clear`, `\f` ist `scroll`. Umbenennen und die bestehenden Tests mit korrigieren.

- [ ] **Step 1: Tokens anlegen**, dann `python3 design-system/scripts/check_tokens.py` (Exit 0) und `cd apps/studio && pnpm tokens`, `bash scripts/check-tokens.sh` (Exit 0). Der Import in die `.fig` (OpenPencil) bleibt beim Nutzer; im Bericht nennen.
- [ ] **Step 2: Failing tests** in `textModel.test.ts`:

```ts
it("pages_split_on_clear_and_two_lines_per_page", () => {
  expect(pages(String.raw`A\nB\rC`).map(p => p.lines.map(l => l.map(s => s.text).join("")))).toEqual([["A","B"],["C"]]);
  expect(pages(String.raw`A\nB\rC`)[0].prompt).toBe(true);
});
it("scroll_keeps_the_last_line", () => {
  expect(pages(String.raw`A\nB\fC`).map(p => p.lines.map(l => l.map(s => s.text).join("")))).toEqual([["A","B"],["B","C"]]);
});
it("a_third_newline_overflows_into_a_new_page", () => {
  const p = pages(String.raw`A\nB\nC`);
  expect(p).toHaveLength(2); expect(p[1].overflow).toBe(true);
});
it("codes_stay_in_the_line_and_count_zero", () => {
  const p = pages("Hallo {PLAYER}!");
  expect(p[0].lines[0].some(s => s.kind === "code")).toBe(true);
  expect(lineWidth(p[0].lines[0])).toBe(6);
});
it("trailing_break_makes_no_empty_page", () => { expect(pages(String.raw`A\r`)).toHaveLength(1); });
```

- [ ] **Step 3: Run, FAIL** (`cd apps/studio && pnpm vitest run src/workbench/text/textModel.test.ts`) → **Step 4: Implement** `pages`, `lineWidth`, Umbenennung `clear`/`scroll` → **Step 5: Run, PASS**, `pnpm test` grün.
- [ ] **Step 6: Commit** `feat(tokens,text): DS screen tokens; pages and line width in the text model`

---

### Task 1b: Primitives `DSScreen`, `Tabs`, neue `Badge`-Arten

**Files:**
- Create: `ui/primitives/DSScreen.tsx`, `DSScreen.module.css`, `DSScreen.test.tsx`; `ui/primitives/Tabs.tsx`, `Tabs.module.css`, `Tabs.test.tsx`; `ui/gallery/pages/ScreenPage.tsx`
- Modify: `ui/primitives/Badge.tsx` (+ test), `ui/primitives/index.ts`, `ui/gallery/pages/index.ts` (Array `pages`: Seite „Bildschirm“)

**Interfaces:**
- Produces `DSScreen`: `<DSScreen variant: "textbox" | "sign" | "empty" pages: Page[] page: number scale?: number background?: ReactNode />`. 256 × 192 DS-Pixel mal `scale` (ganzzahlig; ohne Prop aus der Containerbreite per `ResizeObserver`, mindestens 1; in Tests immer explizit, der Stub feuert nie). `role="group"`, `aria-label="DS-Bildschirm, Seite 2 von 3"`. Fläche `szene/ds/grund`, dahinter optional `background` (Kartenbild). Box nach `jsx-lib.js`: x 4, Höhe 54, 4 vom unteren Rand, Innenabstand 9/6, Kante 2 px, zwei Zeilen Pixelify Sans 12/16 DS-Pixel; `sign` dieselbe Box in den Schild-Tokens. Maßstab über `--ds-scale` (`style={{ "--ds-scale": scale } /* dyn */}`) und `calc()` im CSS. `code`-Segmente als `Badge kind="code"`, unbekannte `\xNNNN` als `Badge kind="code"` mit Text „?“ und `title` des Codes. Ein `prompt` zeigt ▼ unten rechts (`aria-hidden`), `overflow` zeigt ⚠ mit `aria-label="Zeile läuft aus der Box"`. `empty` zeigt nur den Grund.
- Produces `Tabs`: `<Tabs items: {id: string; label: string; count?: number; unsaved?: boolean; closeLabel?: string}[] value: string onChange(id) onClose?(id) ariaLabel: string>children</Tabs>`; Base UI Tabs mit `activateOnFocus`; `role="tablist"`; der Kobalt-Punkt ersetzt das Schließen-× bei `unsaved` und trägt `aria-label="ungespeichert"`; das × ist ein `IconButton` **neben** dem `Tabs.Tab` (kein Button im Button) mit `aria-label={closeLabel}`; Entf oder ⌘W auf dem fokussierten Reiter ruft `onClose`. Kein `Tabs.Indicator`.
- Produces `Badge kind` zusätzlich: `"success"` (Haken), `"warning"` (⚠), `"code"` (Mono, `text/sekundaer` auf `flaeche/leise`), `"neutral"`.

- [ ] **Step 1: Failing tests**: `DSScreen` zeigt bei `page=1` von drei Seiten `aria-label` „DS-Bildschirm, Seite 2 von 3“ und ▼; ein `code`-Segment erscheint als Plakette mit dem Codetext; `sign` rendert denselben Text; `Tabs` wechselt per Klick und per Pfeiltaste (`activateOnFocus`), zeigt `count` und bei `unsaved` ein Element mit `aria-label="ungespeichert"` statt des ×, Entf auf dem Reiter ruft `onClose` mit der id; `Badge kind="code"` rendert den Text.
- [ ] **Step 2: Run, FAIL** → **Step 3: Implement** → **Step 4: Run, PASS**; Galerie-Seite „Bildschirm“ (Textbox mit Platzhaltertext in zwei Seiten, Schild, leer; Maßstab 1 und 2); `bash scripts/check-styles.sh` grün.
- [ ] **Step 5: Commit** `feat(ui): DS screen preview, tabs, badge kinds success/warning/code/neutral`

---

### Task 1c: `Picker`-Variante „palette“, `Menu.onOpenChange`

**Files:**
- Modify: `ui/primitives/Picker.tsx` (+ css, + test), `ui/primitives/Menu.tsx` (+ test), `ui/gallery/pages/PickerPage.tsx`

**Interfaces:**
- Produces `PickerItem`: `area` wird optional; neu `icon?: IconName`, `shortcut?: string` (als `Kbd` rechts), `description?: string` (zweite Zeile, grau). `Picker` neu: `variant?: "dialog" | "palette"` (Standard `dialog`), `limitPerGroup?: number` (nach dem Filtern, je Gruppe; Fuß „N Treffer“ zählt vor dem Kürzen), `groupOrder?: string[]`, `emptyQueryGroups?: string[]` (bei leerer Eingabe nur diese Gruppen). Variante `palette`: oben verankert (Abstand 96 px), 640 breit, Titel nur für Screenreader, kein ×, Treffer-Hervorhebung des Suchtexts, Fuß „↑↓ wählen, ↵ ausführen“. `onPick` wird **nach** dem Schließen aufgerufen (nächster Tick), damit die Fokusrückgabe von Base UI nicht in den Aufruf hineinläuft.
- Filter: ein Suchtext trifft `label`, `tech` (startsWith, auch „0412“ → `tech: "0412"`) und `description`.
- Produces `Menu`: Prop `onOpenChange?(open: boolean)`.

- [ ] **Step 1: Failing tests**: `limitPerGroup=2` zeigt je Gruppe zwei Einträge und im Fuß die volle Zahl; `emptyQueryGroups=["Befehle"]` zeigt ohne Eingabe nur Befehle; „0412“ trifft einen Eintrag mit `tech: "0412"`; `onPick` kommt nach `onClose`; `Menu` ruft `onOpenChange(true)` beim Öffnen.
- [ ] **Step 2: Run, FAIL** → **Step 3: Implement** → **Step 4: Run, PASS**, Galerie zeigt beide Varianten.
- [ ] **Step 5: Commit** `feat(ui): picker palette variant with group limits; menu open callback`

---

### Task 2a: Bereichs-Infrastruktur: mehrere Decks, Navigation mit Bereich, v8

**Files:**
- Modify: `ui/frame/Shell.tsx` (+ test), `ui/frame/Deck.tsx` (+ test), `workbench/layout.ts` (+ test), `workbench/navigation.ts` (+ test), `workbench/breadcrumb.ts` (+ test), `App.tsx`, `app/` (Hooks, die `"welt"` hart setzen: `onDragHeight`, `onToggleCollapse`, `revealHand`, `focusPanel`, `HingeHost`), `App.test.tsx`

**Interfaces:**
- Produces `Shell`: statt `list`/`deck` die Prop `areas: Partial<Record<AreaId, { list?: ReactNode; deck: ReactNode }>>` plus `pane?: ReactNode` für Flächen; alle übergebenen Bereiche bleiben gemountet, nur der aktive ist sichtbar, die anderen tragen `hidden`.
- Produces `layout.ts`: `PanelId` um `textView`, `textEdit` erweitert, `text` entfällt; `DECK_PANELS: Record<"welt" | "texte", { schau: PanelId[]; hand: PanelId[] }>` mit `texte = { schau: ["textView"], hand: ["textEdit", "problems"] }` und Welt-Hand ohne `text`; `deckPanels(area)` per Lookup; `layoutKey(area, deck)` liefert `g4.layout.v8.<area>.<deck>`; `dropOldLayouts(storage)` löscht alle Schlüssel, die mit `g4.layout.v7.` beginnen (einmal beim Start).
- Produces `navigation.ts`: `NavResult` bekommt `area: "welt" | "texte"` (`textArchive` → `texte`, alles andere → `welt`); `goTo` ruft `setArea(result.area)` vor dem Fokussieren; `focusPanel` sucht in den Docks des Zielbereichs.
- Produces `Deck`: `DockApis` je Bereich (Map in App), `resetLayout(area)` exportiert (beide Docks `applyDefaultLayout` plus `saveLayout`); Prop `area` wie bisher; Hand-Höhe und Einklappen je Bereich aus `shellState` (schon je Bereich gespeichert).
- Produces `breadcrumb.ts`: `textCrumbs({ archive, place, message }): Crumb[]` → „Texte › Archiv 0412, Zweiblattdorf › Nachricht 005“.
- App: `textTarget` (aus `goTo({kind:"textArchive", index, message?})`) bleibt als Puffer in `AppBody`, damit ein Ziel vor dem ersten Mount von Texte nicht verloren geht; `LOCKED_AREAS`/`LOCKED_HINTS` in `App.tsx` geben `texte` frei (Technik und Einstellungen in Task 6).

- [ ] **Step 1: Failing tests**: `Shell` rendert zwei Bereiche, der inaktive hat `hidden`; `layoutKey("texte","schau") === "g4.layout.v8.texte.schau"`; `dropOldLayouts` entfernt nur `v7`-Schlüssel; `nextSelection({kind:"textArchive", index: 23})` liefert `area: "texte"`; `goTo` auf ein Archiv ruft `setArea("texte")` (`App.test.tsx`); Welt-Layout aus v7 mit `text` fällt auf den Standard zurück (`isSaneLayout`).
- [ ] **Step 2: Run, FAIL** → **Step 3: Implement** → **Step 4: Run, PASS**, Browser: Welt wie vorher, Atlas „Texte“ zeigt eine leere Liste und ein leeres Deck ohne Fehler in der Konsole.
- [ ] **Step 5: Commit** `feat(studio): areas with their own list and deck, navigation carries the area, layout v8`

---

### Task 2b: Bereich Texte (Liste, Schau, Hand)

**Files:**
- Create: `workbench/text/TextArea.tsx` (Kontext + Zusammenbau), `TextList.tsx`, `TextView.tsx`, `TextHand.tsx`, `TextAreaContext.tsx`, `TextTable.tsx` (virtualisierte Tabelle), je `.module.css`, Tests je Datei (`TextPanel.test.tsx` wird zu `TextArea.test.tsx` mit denselben Fällen)
- Modify: `workbench/text/TextPanel.tsx` (entfällt), `workbench/text/MessageEditor.tsx` (bleibt, bekommt ein CSS-Modul; Verhalten unverändert: Enter → `\n`, Übergabe nach 500 ms Ruhe, bei Blur und ⌘S, `validateText` live, Caret bleibt; neu ⇧↵ fügt `\r` ein), `workbench/text/textModel.ts` (`filterArchives` trifft auch den Ortsnamen), `workbench/panels.tsx` (Panels `textView`, `textEdit`), `App.tsx` (Bereich `texte`, Statusleisten-Kontext „Archiv 0412, 34 Nachrichten, 1 zu lang“, Hinweis „↵ neue Zeile, ⇧↵ neue Seite“ bei Fokus im Editor), `scripts/check-styles.allow` (Zeile `workbench/text/*` raus)

**Interfaces:**
- Consumes: `useTextArchive`, `textModel` (`archiveRows`, `usersByArchive`, `filterArchives`, `segments`, `pages`, `lineWidth`, `visibleLine`, `SEARCH_LIMIT`), `ListPane`, `PanelHead`, `TreeRow`/`ListRow`, `TextField type="search"`, `Segmented`, `Button`, `Badge`, `Notice`, `EmptyState`, `DSScreen`, `Reference`, `goTo`, `ProblemsStore`, Thumbnail-Store (Kartenbild des ersten Headers als `background`).
- Produces `TextAreaContext`: `{ archive: number | null; message: number | null; page: number; query: string; scope: "archive" | "all"; select(archive, message?); setPage; setQuery; setScope }`; liest beim Mount den Puffer `textTarget` aus Task 2a.
- Liste „Textarchive“ (`PanelHead`, Zähler = Zahl der Archive): Gruppen *Orte* (Archive mit Nutzern aus `usersByArchive`; Name = Ortsname des ersten Headers über `buildPlaces`, Untertitel Kennung) und *Spielweit* (Untertitel Kennung); Nachrichtenzahl nur am geladenen Archiv; Überlängen als `Badge kind="warning"` mit Zahl nur aus `ProblemsStore` (Summenzeile „3 Texte zu lang, in 2 geprüften Archiven“); Suchfeld „Ort oder Archivnummer“; `+` legt ein Archiv an (`createTextArchive`); `data-archive` an den Zeilen.
- Schau (`textView`): `PanelHead` „Nachricht 005“, Untertitel „Archiv 0412, Zweiblattdorf“, `Badge kind="neutral"` „3 Seiten“; `DSScreen variant="textbox"`, Zeile „Seite 2 von 3, 256 × 192, 2-fach“, „Zurück“ (leise, Pfeilsymbol) und „Weiterblättern“ (sekundär); rechts Seiten-Karten (Kopf „Seite n“, Text, rechts in Mono „34 von 38“ je Zeile; gewählte mit Kobalt-Kante, Klick wählt, ←/→ blättern bei Fokus in der Schau).
- Hand (`textEdit`): Kopf mit Suchfeld (`aria-label="Texte suchen"`), `Segmented` Dieses Archiv | Alle, `Button kind="primary"` „Nachricht anlegen“; `TextTable` mit `role="grid"`, Spalten Nr. (dreistellig, Mono) · Text (Codes als `Badge kind="code"`) · Längste Zeile (Balken, `status/warnung` und ⚠ plus fetter Zähler „45 von 38“ über 38); Zeilen `data-message`, Treffer `data-hit`, `aria-selected`, Roving Tabindex, Enter springt in den Editor; nur sichtbare Zeilen plus 20 Puffer werden gerendert (eigene Virtualisierung über `scrollTop`, keine Abhängigkeit); rechts Spalte „Nachricht 005“ mit `MessageEditor` (`aria-label="Text der Nachricht"`), „Verwendet in“ als Header-`Reference`s (`data-header`), „Letzte Nachricht entfernen“ leise; Fehler als `Notice kind="error"`. Treffer aus „Alle“ nennen Archiv und Nachricht in zwei Spalten, nicht als „A · B“.
- Leere Zustände: kein Projekt („Kein Projekt geöffnet.“), kein Archiv gewählt („Wähle links ein Archiv.“), Archiv ohne Nachricht (Taste „Nachricht anlegen“), Suche ohne Treffer („Nichts gefunden für „…““).

- [ ] **Step 1: Tests anpassen** (`TextArea.test.tsx` u. a.): dieselben Fälle wie `TextPanel.test.tsx` (Archiv wählen, Nachricht ändern mit 500 ms, anfügen, entfernen, Suche beide Bereiche, `textTarget` öffnet Archiv und Nachricht, Warnungen aus `ProblemsStore`, StrictMode-Ablehnung) über die neuen Rollen; neu: Gruppierung Orte/Spielweit, `filterArchives("Zweiblatt")` trifft Archiv 0412, Seitenwechsel in der Schau ändert das `aria-label`, `TextTable` mit 800 Zeilen rendert höchstens 60 `row`-Elemente, ⇧↵ fügt `\r` ein.
- [ ] **Step 2: Run, FAIL** → **Step 3: Implement** → **Step 4: Run, PASS**, Browser auf einer Platin-Kopie: Archiv 0023 (Jubelstadt) öffnen, Nachricht ändern, Vorschau blättert, Suche „Barry“ über alle Archive, `call.sh get_text_archive '{"index":23}'` zeigt die Änderung nach Speichern; größtes Archiv per `call.sh` suchen und `performance.now()` um das Rendern messen (unter 100 ms).
- [ ] **Step 5: Commit** `feat(studio): texts area with archive list, DS preview and virtualised message table`

---

### Task 3a: CodeMirror-Thema aus den Tokens

**Files:**
- Create: `workbench/script/rotomTheme.ts` (+ test)
- Modify: `workbench/script/ScriptEditor.tsx` (+ test), `workbench/script/ScriptPanel.tsx` (nur die Prop), `apps/studio/package.json` (`pnpm --filter studio remove @codemirror/theme-one-dark`)

**Interfaces:**
- Produces: `ROTOM_THEME: Record<"hell" | "dunkel", Extension>` als Modulkonstanten (je `EditorView.theme({...}, { dark })` mit `var(--g4-*)` für Hintergrund `flaeche/blatt`, Text `text/primaer`, Auswahl `auswahl/flaeche`, Zeilennummern `text/sekundaer`, Cursor `akzent/grund`, aktive Zeile, Diagnosen `status/*`, Tooltips `flaeche/blatt` mit Schatten, Schrift `schrift/familie/code`; plus `syntaxHighlighting(HighlightStyle.define(...))` aus `syntax[mode]` in `tokens.ts`); `ROTOM_HIGHLIGHT: Record<"hell" | "dunkel", readonly TagStyle[]>` exportiert für Tests. `ScriptEditor` Prop `theme: "hell" | "dunkel"` (vorher `"light" | "dark"`), Wert in `ScriptPanel` aus `useSyncExternalStore(onThemeChange, effectiveTheme)`; der vorhandene `themeSlot`-Compartment wird nur bei Wechsel neu konfiguriert (Identität der Konstanten).

- [ ] **Step 1: Failing tests**: `ROTOM_HIGHLIGHT.dunkel` enthält für den Kommentar-Tag die Farbe `syntax.dunkel.kommentar`; `switches_the_theme_without_losing_the_text` bleibt und prüft zusätzlich, dass `ROTOM_THEME.dunkel` dieselbe Instanz bei zwei Rendern ist.
- [ ] **Step 2: Run, FAIL** → **Step 3: Implement** → **Step 4: Run, PASS**, `pnpm build` ohne one-dark im Bundle (`grep -c "one-dark" dist/assets/*.js` ist 0).
- [ ] **Step 5: Commit** `feat(studio): CodeMirror theme from the tokens, one-dark removed`

---

### Task 3b: Script-Panel auf Primitives, Punkt am Dock-Reiter

**Files:**
- Create: `workbench/script/scriptDirtyStore.ts` (+ test)
- Modify: `workbench/script/ScriptPanel.tsx` (+ test, + `.module.css`), `AdoptBox.tsx` (+ test), `ScriptEditor.tsx` (Inline-Styles raus), `ui/frame/Deck.tsx` (`PanelTab`), `workbench/problems/problems.ts` oder der `ScriptPanelContext` (Feld `depth`), `scripts/check-styles.allow` (`workbench/script/*` raus)

**Interfaces:**
- Consumes: `PanelHead`, `Tabs` (Task 1b), `Menu` mit `onOpenChange` (Task 1c), `Button`, `Badge`, `Notice`, `EmptyState`, `WorkbenchData.depth`.
- Produces `scriptDirtyStore`: `{ subscribe(cb): () => void; get(): number; set(n: number) }`; `ScriptPanel` schreibt `documents.dirty().length` hinein; `PanelTab` in `Deck.tsx` liest es für `api.id === "script"` per `useSyncExternalStore` und zeigt den Kobalt-Punkt mit `aria-label="ungespeichert"` (kein `setTitle`, der Titel bleibt „Script“).
- `ScriptPanel`: `PanelHead` mit `docTitle`, Untertitel in Tüfteln der Dateipfad (`data-script-file` bleibt), Aktionen: Outline als `Menu` (lädt `documentSymbols` in `onOpenChange(true)`, Einträge „Name, Z. 4“), Speichern, „Script anfügen“, LSP-Zustand als `Badge` (`data-lsp`, Texte wie heute) mit „Neu starten“; offene Dokumente als `Tabs` (`data-script`, `unsaved` aus `doc.text !== doc.savedText`, `closeLabel` „<Name> schließen“); `AdoptBox` als `Notice kind="info"` mit Fortschrittsbalken (`/* dyn */` Breite) und Bericht; `EmptyState` „Kein Script geöffnet.“ / „Eine Scriptdatei öffnet sich über den Header, ein Event oder ein Problem.“ Die Hand-Höhe ≥ 400 beim Öffnen ist schon gebaut (`SCRIPT_HAND_HEIGHT` in `App.tsx`): nur testen.

- [ ] **Step 1: Tests anpassen/neu**: `ScriptPanel.test.tsx` (Reiter-Rollen, Punkt bei dirty über `aria-label`, Outline-Menü lädt beim Öffnen, LSP-Badge, Pfad nur in Tüfteln), `Deck.test.tsx` (Reiter „Script“ zeigt den Punkt, wenn der Store > 0), `AdoptBox.test.tsx`, `App.test.tsx` (Script öffnen hebt die Hand auf 400).
- [ ] **Step 2: Run, FAIL** → **Step 3: Implement** → **Step 4: Run, PASS**, `pnpm build`, Browser: Script öffnen, Reiter-Punkt nach einer Änderung, Thema wechseln.
- [ ] **Step 5: Commit** `feat(studio): script panel on primitives, unsaved dot on the dock tab`

---

### Task 4: Probleme-Panel auf Primitives

**Files:**
- Modify: `workbench/problems/ProblemsPanel.tsx` (+ test, + `.module.css`), `workbench/problems/problems.ts` (Kontext bekommt `depth`), `scripts/check-styles.allow` (`workbench/problems/*` raus)

**Interfaces:**
- Produces: `ProblemsTable({ rows, depth, goTo })` exportiert (Technik nutzt sie in Task 6b); `PanelHead` „Probleme“ mit `countsText`; Tabelle (`role="table"`) mit Spalten Schwere (Symbol `error`/`warning` plus `aria-label` „Fehler“/„Warnung“; Fehler vor Warnungen), Datei als `Reference` (Bereich Welt für Scripts, Texte für Archive; `onOpen = goTo(problemTarget(row))`; `disabled`, wenn `problemTarget` `null` liefert), Zeile:Spalte in Mono, Meldung, Quelle als `Badge kind="neutral"`; in Tüfteln steht in der Datei-Spalte der Pfad; `EmptyState` „Keine Probleme.“ / „Beim Speichern eines Scripts oder Texts landen Fehler hier.“ Severity bleibt `"error" | "warning"` (kein `info`).

- [ ] **Step 1: Tests anpassen** (Rollen, Sortierung, `Reference`-Klick ruft `goTo`, `null`-Ziel ist gesperrt, Tüfteln zeigt Pfad) → **Step 2: FAIL** → **Step 3: Implement** → **Step 4: PASS** → **Step 5: Commit** `feat(studio): problems panel on primitives with references`

---

### Task 5a: Assistenten als Schrittleiste im Scharnier (Rahmen, Tasten, Sperren)

**Files:**
- Create: `workbench/wizards/WizardFrame.tsx` (+ css, + test), `workbench/wizards/WizardBar.tsx` (Schrittleiste; + test)
- Modify: `workbench/wizards/WizardShell.tsx` (entfällt, Aufrufer nutzen `WizardFrame`), `ui/frame/Hinge.tsx` (fester Slot `<div data-wizard-bar hidden>` statt Prop `wizard`), `ui/frame/Deck.tsx` (Slot `<div data-wizard-hand hidden>` neben dem Hand-Dock; Hand-Dock bekommt `hidden`, solange der Slot belegt ist), `app/WizardHost.tsx`, `app/useShortcuts.ts` (+ test), `app/useInspectorReveal` (oder wo `revealHand` liegt), `App.tsx`, `ui/frame/StatusLine.tsx` (Assistenten-Hinweise), `workbench/modal.ts` (bleibt: `escapeCloses`, `useFocusTrap` entfällt für Assistenten), `scripts/check-styles.allow` (`workbench/wizards/*`, `app/WizardHost.tsx` raus; die Schritt-Dateien ziehen in Task 5b um und bleiben bis dahin auf der Liste als Einzeleinträge)

**Interfaces:**
- Consumes: `WizardStep { id, title, content, canNext }` (unverändert), `Segmented`, `Button`, `Kbd`, `Notice`, `Badge kind="irreversible"`, `escapeCloses`.
- Produces `WizardFrame` Props wie `WizardShell` (`title`, `icon: IconName`, `steps`, `step`, `onStep`, `busy`, `error`, `summary?: string`, `irreversible?: boolean`, `result?`, `submitLabel`, `onSubmit`, `onCancel`, `onShow`): rendert per `createPortal` die `WizardBar` in `[data-wizard-bar]` und den Inhalt in `[data-wizard-hand]` (beide in `useLayoutEffect` gesucht; fehlen sie, wie im Unit-Test, rendert alles inline im `container`); `role="dialog"`, `aria-labelledby` zeigt auf den Titel in der Leiste, **kein** `aria-modal`, `data-wizard` am Wurzelelement; beim Öffnen Fokus ins erste fokussierbare Element des Inhalts, beim Schließen zurück auf das vorher aktive Element; Spalten im Inhalt mit 1-px-Linien `linie/leise`; die Zusammenfassung als Zeile unten in `flaeche/blatt` mit Listensymbol, Satz und rechts `Badge kind="irreversible"` „Nicht rückgängig“ (steht in jedem Schritt, sobald `summary` gesetzt ist; `error` ersetzt sie als `Notice kind="error"`); `result` ersetzt den Inhalt mit Tasten „Zeigen“, „Hier spielen“ (ruft `onPlay` aus App) und „Schließen“; während `busy` `inert` und `aria-busy`.
- `WizardBar`: oben 2 px in `bereich/<x>-punkt`, Symbol-Chip 26 px in `bereich/<x>-flaeche`, Titel Zen Maru 700 15 px; Schritte als `<ol>` mittig als Pillen (erledigt Haken auf `scharnier/taste`, aktiv `aria-current="step"` Pille `scharnier/text` mit Schrift `scharnier/grund`, offen Umriss `scharnier/leise`, Verbinder 16 × 2 px); rechts „Abbrechen“ (leise), „Zurück“ ab Schritt 2 (`Kbd ⌘[`), `Button kind="primary"` „Weiter“ bzw. `submitLabel`, das `Kbd ↵` steht **neben** der Taste (der Tastentext bleibt genau „Weiter“).
- Tastenregeln (Dokument-Listener in `WizardFrame`): Enter führt weiter nur wenn `!e.defaultPrevented`, `frameRef.contains(e.target)` oder Ziel in der Leiste, und das Ziel kein `button`, `a`, `textarea`, `[role=option]`, `[role=combobox]`, `[role=radio]`, `[role=tab]` ist; ein Feld mit geändertem Draft übergibt bei Enter nur (die Primitives rufen `preventDefault`). Escape bricht ab nur bei `escapeCloses(e)` und Fokus im Rahmen oder in der Leiste. ⌘[ geht zurück.
- Sperren: `useShortcuts.modalOpen()` prüft zusätzlich `[data-wizard]`; damit sind ⌘Z, ⌘S, ⌘R, Entf, Escape-Abwählen und Werkzeugbuchstaben aus; beim Öffnen `setTool("select")`; `revealHand`/Inspector-Reveal tun nichts, solange `wizards.request` gesetzt ist; `wizards.open` setzt `setArea("welt")`; eine eingeklappte Hand wird für die Dauer auf mindestens 286 px gehoben, ohne die gemerkte Höhe zu überschreiben; der Doppelklick-Einklapper ist gesperrt; die anderen Atlas-Einträge sind gesperrt mit Tooltip „Erst den Assistenten abschließen oder abbrechen“. Statusleiste: links „↵ Weiter, ⌘[ Zurück, Esc Abbrechen“, Mitte „Schritt 2 von 4: Anordnung“.
- Die Schau bleibt bedienbar; der Schritt „Eingang“ liest `lastTile` wie heute.

- [ ] **Step 1: Tests**: `WizardFrame.test.tsx` (Enter auf Taste löst nur die Taste aus; Enter im Feld mit Draft übergibt, zweites Enter geht weiter; Escape mit Draft nimmt den Draft, zweites Escape bricht ab; Enter in `textarea` geht nie weiter; `busy` sperrt; Ergebnis zeigt „Zeigen“, „Hier spielen“, „Schließen“; Fokus kehrt zum Auslöser zurück); `WizardBar.test.tsx` (`aria-current="step"`, „Zurück“ erst ab Schritt 2, Tastentext „Weiter“); `useShortcuts.test.tsx` (`[data-wizard]` sperrt Entf und ⌘Z); `App.test.tsx` (offener Assistent blendet das Hand-Dock aus, Leiste im Scharnier, Atlas gesperrt); `NewAreaWizard.test.tsx`/`NewRoomWizard.test.tsx`: Helfer auf Rolle und Namen umstellen, „Im Studio zeigen“ → „Zeigen“, sonst unverändert.
- [ ] **Step 2: Run, FAIL** → **Step 3: Implement** → **Step 4: Run, PASS**, Browser: „Raum anlegen…“ durchlaufen; aus `scripts/g4emu`: `uv run g4emu scenario ../agent/scenarios/pt-raum-assistent.yaml …` auf einer frischen Kopie bleibt grün (Backend unverändert).
- [ ] **Step 5: Commit** `feat(studio): wizards as a step bar in the hinge with their steps in the hand`

---

### Task 5b: Assistenten-Schritte auf Primitives

**Files:**
- Modify: `workbench/wizards/NewAreaWizard.tsx`, `NewRoomWizard.tsx`, `HeaderChoice.tsx`, `PickGrid.tsx`, `WizardResult.tsx` (je + test, + `.module.css`); `workbench/wizards/HeaderSearch.tsx` entfällt zugunsten von `workbench/inspector/HeaderPicker.tsx` (sitzt schon auf `Picker`); `workbench/DraftInput.tsx` (+ test) entfällt; `scripts/check-styles.allow` (restliche Assistenten-Einträge und `DraftInput.tsx` raus)

**Interfaces:**
- Consumes: `Segmented`, `NumberField` (lehnt ungültige Werte ab, statt still zu begrenzen), `TextField`, `Reference`, `Notice`, `Button`, `Badge`, `HeaderPicker`.
- `HeaderChoice` auf `Segmented` (Kopie von Header / Neuer Header) und `HeaderPicker`; Zahlen in `NumberField` mit `min`/`max` aus dem Schritt; Tests, die `container.querySelector("select")` nutzen, fragen stattdessen nach `role="combobox"` mit Namen; der Schritt „Zusammenfassung“ gibt `summary` und `irreversible` an `WizardFrame`.

- [ ] **Step 1: Tests anpassen** (gleiche Fälle; Patch-Hinweis, gesperrte Eventdatei, „später verbinden“, Kachel 999 abgelehnt) → **Step 2: FAIL** → **Step 3: Implement** → **Step 4: PASS**, `bash scripts/check-styles.sh` mit leerem `wizards`-Block → **Step 5: Commit** `feat(studio): wizard steps on the primitives`

---

### Task 6a: Start-Karten und Einstellungen

**Files:**
- Modify: `ui/frame/StartPane.tsx` (+ css), `ui/frame/ShellContext.tsx` (+ test), `App.tsx` (Bereich `einstellungen` als Fläche, ⌘O), `world/buildings.ts` (`shortcutAction`: ⌘O), `app/useShortcuts.ts`
- Create: `ui/frame/StartPane.test.tsx`, `einstellungen/SettingsArea.tsx` (+ css, + test), `ui/frame/EmulatorRow.tsx` (+ test; gemeinsam mit Technik)

**Interfaces:**
- Consumes: `shellState.recent` (`{ path, name, game, openedAt }`), `ago(from, now)`/`useNow()` aus `ui/frame/timeAgo.ts`, `applyTheme`/`applyDensity`/`readTheme` aus `ui/tokens/theme.ts`, `session.onPickEmulator`, `session.emulatorPath` (Task 6b liefert; bis dahin `undefined` → „Kein Emulator gewählt“).
- Produces `ShellApi`: `setTheme(t: "system" | "hell" | "dunkel")`, `setDensity(d: "komfort" | "kompakt")` (halten `shell.theme`/`shell.density` aktuell; `toggleTheme`/`toggleDensity` bleiben).
- Start: Karten im Raster (Pfeiltasten über das Raster, Roving Tabindex): Vorschaufläche 4:3 als leerer DS-Bildschirm (`DSScreen variant="empty"`, Rahmen `flaeche/bildschirm-rahmen`, Radius `radius/bildschirm`) mit dem Projektnamen in Pixelify; Name, Spiel-Chip (`Badge kind="neutral"` mit Gamepad-Symbol), „geöffnet vor 2 Stunden“, Plakette „Zuletzt“ auf dem neuesten, „…“-`Menu` mit „Aus der Liste nehmen“, Pfad als `title`; Karte „nicht gefunden“ (grau) erst nach einem fehlgeschlagenen Öffnen (kein Existenzbefehl); `Button kind="primary"` „Projekt öffnen“ mit `shortcut="⌘O"`; Hinweis „Projekte sind entpackte DSPRE-Ordner“ (schon vorhanden).
- Einstellungen: Spalten Darstellung (Thema `Segmented` „Hell | Dunkel | Wie System“ mit Symbolen → `setTheme`; Dichte „Komfort | Kompakt“ → `setDensity`, Hinweis „Kompakt: Zeilen 24 statt 32 px hoch“), Emulator (`EmulatorRow`: Pfad oder „Kein Emulator gewählt“, Taste „Emulator wählen“), Tastenkürzel (`Kbd`-Liste nur zum Lesen aus `shortcutAction`); Statusleiste „Änderungen gelten sofort und für alle Projekte“.

- [ ] **Step 1: Failing tests** (Karten mit „geöffnet vor …“, „Zuletzt“ nur einmal, Menü entfernt den Eintrag, ⌘O ruft `onOpenProject`; `setTheme("dunkel")` ruft `applyTheme` und setzt `shell.theme`; Einstellungen rendern beide `Segmented` und `EmulatorRow`) → **Step 2: FAIL** → **Step 3: Implement** → **Step 4: PASS** → **Step 5: Commit** `feat(studio): start cards, settings area, theme and density setters`

---

### Task 6b: Technik, Werkzeug-Store, Emulatorpfad

**Files:**
- Create: `technik/TechnikArea.tsx` (+ css, + test), `app/toolStatus.ts` (+ test)
- Modify: `app/useProjectSession.ts` (`emulatorPath` als State), `workbench/script/ScriptPanel.tsx` (schreibt LSP-Zustand in `toolStatus`), `world/thumbnails` (`ThumbnailCache.onToolMissing` setzt `toolStatus.apicula = "fehlt"`), `App.tsx` (Bereich `technik` mit Liste und Fläche, `LOCKED_AREAS` ohne `technik`/`einstellungen`)

**Interfaces:**
- Produces `toolStatus`: Store `{ subscribe; get(): { lsp: { state: "aus" | "startet" | "bereit" | "fehlt" | "fehler"; note?: string; restart?: () => void } | null; apicula: "unbekannt" | "bereit" | "fehlt" } }` mit `set(partial)`; `lsp` ist `null`, solange `ScriptPanel` nie gemountet war („nicht gestartet“). Kein Pfad (es gibt keinen Befehl dafür; Abweichung von Spec §5.9 in Task 10 notieren).
- Produces `session.emulatorPath: string | null` (aus `storedEmulator()`, aktualisiert nach `onPickEmulator`).
- Technik: Liste mit vier `ListRow`s mit Symbol und Status-Untertitel: *Prüfen und Bauen* („gebaut vor 4 Minuten“ aus `session.lastBuiltAt`/`ago`, sonst „Noch nicht gebaut“), *Patches* („Dynamische Header angewendet“ / „verfügbar“ / „fehlerhaft“ / „Nur Platin und HGSS“ aus `headerPatchStatus`: `supported`, `applied`, `repairable`), *Emulator* (Pfad oder „Kein Emulator gewählt“), *Werkzeuge* („2 von 2 bereit“ aus `toolStatus`). Fläche: Prüfen und Bauen = `ProblemsTable` (Task 4) + Tasten „Bauen“, „Starten“ + letzter Build (Zeit, Pfad `session.lastRom`); Patches = Plakette (`success` Angewendet, `info` Verfügbar, `error` Fehlerhaft, `ptHgssOnly`) und Tasten „Patch anwenden“ (vorher `Dialog` mit Frage-Titel „Patch „Dynamische Header“ anwenden?“, weil nicht rückgängig) und „Patch reparieren“ über `useHeaderPatch` aus `workbench/wizards/`; Emulator = `EmulatorRow`; Werkzeuge = apicula und rotom-lsp mit Zustand als `Badge` (`data-lsp` am LSP-Eintrag) und „Neu starten“. Ohne Projekt zeigen Prüfen und Patches einen `EmptyState`.

- [ ] **Step 1: Failing tests** (`toolStatus` benachrichtigt Abonnenten; Technik-Liste wechselt die Fläche; Plakette je Patch-Zustand; „Patch anwenden“ fragt erst; `emulatorPath` nach `onPickEmulator` gesetzt) → **Step 2: FAIL** → **Step 3: Implement** → **Step 4: PASS**, Browser: Technik › Patches auf einer frischen Kopie zeigt „Verfügbar“ → **Step 5: Commit** `feat(studio): Technik area with tool status store and emulator path`

---

### Task 7: Befehlspalette ⌘K

**Files:**
- Create: `ui/frame/Palette.tsx` (+ test; nutzt nur `Picker`), `app/paletteEntries.ts` (+ test)
- Modify: `ui/frame/TitleBar.tsx` (Prop `onPalette`, Knopf aktiv, Tooltip „Kommt mit 3c.2“ weg), `world/buildings.ts` (`shortcutAction`: ⌘K), `app/useShortcuts.ts`, `App.tsx`

**Interfaces:**
- Consumes: `Picker variant="palette"` (Task 1c), `buildPlaces`, `archiveRows`/`usersByArchive`, `listScriptFiles` (nur bei übernommenen Scripts), `goTo`, `wizards.open`, `resetLayout(area)` (Task 2a), `setTheme`/`setDensity`/`toggleTheme`/`toggleDensity`, Befehle aus App.
- Produces: `paletteEntries({ places, headers, archives, scripts, hasProject, commands }): PickerItem[]` mit `id` als `header-412`, `place-…`, `archive-412`, `script-…`, `cmd-…`, `tech` als vierstellige Kennung, `group` in der Reihenfolge Befehle, Orte, Header, Textarchive, Scripts; ohne Projekt nur Befehle, Assistenten-Befehle dann gesperrt. Befehle: Projekt öffnen, Speichern, Bauen, Hier spielen, Gebiet anlegen…, Raum anlegen…, Liste ein/aus, Layout zurücksetzen, Thema wechseln, Dichte wechseln, Begriffe, Einstellungen, Bereich Welt, Bereich Texte, Bereich Technik. `Palette` öffnet mit `limitPerGroup={8}`, `emptyQueryGroups={["Befehle"]}`; Enter führt nach dem Schließen aus, Escape schließt.

- [ ] **Step 1: Failing tests**: `paletteEntries` ohne Projekt nur Befehle; „0412“ trifft Header und Archiv in zwei Gruppen; Enter auf „Speichern“ ruft `onSave` nach dem Schließen; ⌘K und der Titelleisten-Knopf öffnen (`App.test.tsx`); „Layout zurücksetzen“ ruft `resetLayout("welt")`.
- [ ] **Step 2: Run, FAIL** → **Step 3: Implement** → **Step 4: Run, PASS** → **Step 5: Commit** `feat(studio): command palette on the picker`

---

### Task 8: Tüfteln im Raster

**Files:**
- Modify: `workbench/MatrixPanel.tsx` (+ test), `workbench/AreaGrid.tsx` (+ test), `workbench/inspector/MatrixCellEditor.tsx` (+ test; hat schon `depth`, `TechnicalSection` und `showBase`)

**Interfaces:**
- In Tüfteln zeigt jede Zelle drei Mono-Zeilen (Karten-ID, Header-ID, Höhe) in `text/sekundaer`, `data-raw="0104 0411 0"`; in Bauen keine Zahlen. `MatrixCellEditor` bekommt den Abschnitt „Rohdaten, Zeile y“: u16 hex vierstellig in Großbuchstaben der Karten- und der Header-Zeile, u8 der Höhen, gewählte Zelle markiert (`aria-current`).

- [ ] **Step 1: Failing tests** (Zellen in Bauen ohne `data-raw`, in Tüfteln mit; Rohdaten-Abschnitt hex vierstellig, gewählte Zelle markiert) → **Step 2: FAIL** → **Step 3: Implement** → **Step 4: PASS** → **Step 5: Commit** `feat(studio): tinker depth in the raster with raw rows`

---

### Task 9: Ausnahmeliste leer, strengere Stilprüfung, Galerie, Dunkel-Durchgang

**Files:**
- Modify: `scripts/check-styles.sh` (Regel 1 fängt `style=\{` statt nur `style={{`, Ausnahme `/* dyn */`), `scripts/check-styles.allow` (leer), `platform/WebDialogHost.tsx` (+ test; → `Dialog` und `TextField`), `workbench/ContextPanel.tsx` (+ test: Enter auf einer Tür folgt dem Ziel; Ladezustand „Lade …“ statt „Hier ist noch nichts“), `app/useShortcuts.ts` (+ test: Werkzeugbuchstabe auf ein gesperrtes Werkzeug tut nichts)
- Create: `ui/gallery/pages/FramePage.tsx` (Scharnier im Assistenten-Modus mit `WizardBar`, Startkarte, Palette, Tabs), Eintrag in `ui/gallery/pages/index.ts`

- [ ] **Step 1: `bash scripts/check-styles.sh`** mit leerer Liste und verschärfter Regel → Funde beheben bis Exit 0; Gegenprobe `grep -rn "CSSProperties" apps/studio/src --include=*.tsx | grep -v test | grep -v "dyn"` ist leer.
- [ ] **Step 2: Tests** für `WebDialogHost`, `ContextPanel`, Shortcuts.
- [ ] **Step 3: Dunkel und Kompakt im Browser**: Galerie, Welt, Texte, Script, Technik, Einstellungen, Assistent je ein Screenshot (hell, dunkel, kompakt) ins Scratch-Verzeichnis; unlesbare Stellen beheben (Kontrast über Tokens, nie über Hex).
- [ ] **Step 4: Commit** `chore(studio): no style exceptions left, stricter style check, frame gallery page, dark and compact pass`

---

### Task 10: Dokumentation, Gate-Dokument, Roadmap, volle Prüfung

**Files:**
- Create: `docs/phase-3c-gate.md`
- Modify: `docs/agent-gate.md`, `docs/dev-setup.md`, `docs/ROADMAP.md`, `design-system/README.md`, `docs/superpowers/plans/2026-10-23-phase-3b3a-codecs-pokemon-data.md` (Task 12 dort: `Tabs` existiert seit 3c.2, nur erweitern)

**Gate-Dokument** (Spec §1 (A)–(H), Muster der vorigen Gates, Ergebnistabelle leer) mit der Spalte „Wer“:
- Test-Umgebung (Agent): (A) mit Screenshots („Hier spielen“ baut im Browser nur); (B) komplett (Rezept: Archiv 0023, Nachricht ändern, `aria-label` „DS-Bildschirm, Seite x von y“ blättern, Suche „Barry“, `call.sh get_text_archive`); (C) ohne LSP (Hand-Höhe per `getBoundingClientRect` des Hand-Docks ≥ 400, Compile-Tippfehler `AddItemm`, Prüflicht, Sprung zur Zeile); (D) über das Scharnier plus Szenario `pt-raum-assistent.yaml`, Escape mit Draft zweimal; (E) Screenshots; (F) Tastaturpfad per Agent, `:focus-visible` per JS; (G) Rezepte 3a, 3b.1, 3b.2 plus `scripts/gate.sh`; Palette „0412“; „Layout zurücksetzen“ und `v8`-Schlüssel; Thema und Dichte über Einstellungen und `localStorage`; Technik › Patches statt „Patch reparieren“ im Assistenten.
- Hand des Nutzers: (C) mit rotom-lsp (Completion, Hover, Diagnosen), (E) Blick, LSP-, Completion- und DSPRE-Punkte in (G), (H) komplett.
- `docs/agent-gate.md`: Abschnitt „Doppeldeck: wo was ist“ um Bereiche Texte, Technik, Einstellungen, Palette und Assistent im Scharnier ergänzen; Rezepte 3b.2 „Panel Text“ → Bereich Texte, Beschriftungen wie in Task 2b, 3b, 5a. `docs/ROADMAP.md`: 3c „implementiert, Gate offen“. `design-system/README.md`: „Was die App umsetzt“ auf 3c-Stand plus Abweichungen (Erzählen fehlt, Live-Vorschau fehlt, Windows-Fensterknöpfe fehlen, Werkzeuge in Technik ohne Pfad, Nachrichtenzahl nur am geladenen Archiv, `.fig` ohne die `szene/ds`-Tokens bis zum Import).

- [ ] **Step 1: Docs schreiben.**
- [ ] **Step 2: Volle Prüfung**: `pnpm test`, `pnpm build`, `bash scripts/check-styles.sh`, `bash scripts/check-tokens.sh`, `cargo test --workspace --all-features`, dann `G4_ROMS_DIR=<absolut> scripts/gate.sh` im Hintergrund (in einem Worktree ist `G4_ROMS_DIR` zwingend absolut zu setzen); Tabelle in den Bericht.
- [ ] **Step 3: Commit** `docs: gate document for phase 3c, agent guide and roadmap for the Doppeldeck`

---

## Reihenfolge und Parallelität

- Parallel in eigenen Worktrees (getrennte Dateien): **1a → 1b** (1b braucht die Tokens) ∥ **1c** ∥ **3a** ∥ **4** ∥ **8**.
- Danach seriell, weil sie `App.tsx`, `Deck.tsx`, `Shell.tsx`, `layout.ts` oder `useShortcuts.ts` anfassen: **2a → 2b → 3b → 5a → 5b → 6a → 6b → 7 → 9 → 10**. Task 7 braucht 2a (`goTo`-Bereich, `resetLayout`), 5a (`wizards.open` → Welt) und 6a (`setTheme`).

## Abschluss

Nach Task 10: `superpowers:finishing-a-development-branch` (Merge nach `main` nach grünem `gate.sh`). Die Abnahme (Gate 3c (A)–(G) über die Test-Umgebung, (H) und die LSP-Punkte durch den Nutzer) läuft danach in einer eigenen Session; Befunde werden dort behoben. Danach beginnt 3b.3a nach seinem eigenen Plan.
