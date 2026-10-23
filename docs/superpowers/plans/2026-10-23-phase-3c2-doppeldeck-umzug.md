# Phase 3c.2 „Doppeldeck: Texte, Scripts, Technik, Assistenten“ Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Die restlichen Bereiche und Panels ziehen ins Doppeldeck: Texte als eigener Bereich mit DS-Vorschau, Scripts und Probleme auf den Primitives, Assistenten als Schrittleiste im Scharnier, Start-Karten, Technik, Einstellungen, die Befehlspalette ⌘K, Tüfteln im Raster; die Stil-Ausnahmeliste wird leer, das Gate-Dokument entsteht.

**Architecture:** Baut auf 3c.1 auf (`ui/primitives`, `ui/frame` mit `Shell`, `Deck`, `Hinge` (Prop `wizard`), `ListPane`, `StartPane`, `shellState`, `DECK_PANELS` je Bereich). Neue Primitives `DSScreen` und `Tabs`; Bereiche `texte`, `technik`, `einstellungen` werden freigeschaltet; die Text-Logik (`useTextArchive`, `textModel`) bleibt und bekommt drei Panels; `WizardShell` wird `WizardFrame` (Schritte im Scharnier, Inhalt in der Hand); die Palette sitzt auf `Picker`.

**Tech Stack:** React 19, `@base-ui/react` 1.9 (Tabs, Combobox), CodeMirror 6 (Thema aus `tokens.ts`), Dockview 8.6, vitest 5.

**Spec:** [docs/superpowers/specs/2026-10-23-phase-3c-doppeldeck-design.md](../specs/2026-10-23-phase-3c-doppeldeck-design.md) (§5.5 Assistenten, §5.8 Palette, §5.9 Flächen, §7.2–§7.7, §8.2, §11, Gate §1).

## Global Constraints

- Wie Plan 3c.1 (Testbefehle, Vanilla-Projekte nie beschreiben, Styling nur über `ui/` und `--g4-*`, keine Hex, `/* dyn */` nur für Laufzeitwerte, Base UI nur in `ui/primitives`, Tests über Rollen und `data-*`); zusätzlich: am Ende ist `scripts/check-styles.allow` leer und `bash scripts/check-styles.sh` grün ohne Ausnahmen.
- Die bestehenden `data-*`-Attribute bleiben (`data-archive`, `data-message`, `data-hit`, `data-script`, `data-lsp`, `data-script-file`, `data-header`); die Rezepte in `docs/agent-gate.md` werden auf die neuen Beschriftungen angepasst, nicht gestrichen.
- Kein Backend-Befehl kommt hinzu; die Brücke bleibt wie nach 3c.1.
- Layout-Schlüssel: Bereich `texte` bekommt eigene Dock-Layouts (`g4.layout.v7.texte.*`); das Panel-Set der Welt-Hand ändert sich (Text zieht aus) → `v8` für alle Bereiche.
- Ein Assistent rendert weiter `role="dialog"` (die bestehenden Tests fragen danach), aber ohne `aria-modal`; Fokus bleibt in der Hand.

## Review Focus

- Script öffnen, während die Hand eingeklappt ist: die Hand klappt auf mindestens 400 px auf, das Dokument ist sichtbar, der Reiter „Script“ trägt den Punkt bei ungespeichertem Text (Task 3).
- Textarchiv mit 800 Nachrichten (HGSS 0368): Liste und Tabelle bleiben unter 100 ms Renderzeit, die Suche „Dieses Archiv“ filtert sofort (Task 2).
- Assistent abbrechen mit Escape in einem Textfeld mit Draft: erst wird der Draft zurückgenommen, erst das zweite Escape bricht ab; Enter in einem Zahlfeld committet, führt nicht weiter (Task 5).
- Palette ohne Projekt: nur Befehle; Eingabe „0412“ mit Projekt trifft Header 0412 und Textarchiv 0412 in zwei Gruppen (Task 7).
- Dunkles Thema in CodeMirror: Syntaxfarben aus `syntax.dunkel`, Systemwechsel zur Laufzeit wechselt das Thema ohne Neuladen des Dokuments (Task 3).

---

### Task 1: Primitives `DSScreen` und `Tabs`; Seiten in `textModel`

**Files:**
- Create: `apps/studio/src/ui/primitives/DSScreen.tsx` (+css, test), `Tabs.tsx` (+css, test); `apps/studio/src/ui/gallery/pages/ScreenPage.tsx`
- Modify: `apps/studio/src/workbench/text/textModel.ts` (+ test), `ui/primitives/index.ts`

**Interfaces:**
- Produces: `<DSScreen variant="textbox"|"sign"|"empty" pages: string[][] page: number scale?: number />` (256 × 192 in `flaeche/bildschirm` mit Rahmen `flaeche/bildschirm-rahmen`; `scale` ganzzahlig, Standard aus der Containerbreite; Textbox unten mit zwei Zeilen in `font-spiel-textbox`, Pfeil ▼ wenn weitere Seiten; `sign` zentrierte Tafel; `aria-label="DS-Bildschirm, Seite 2 von 3"`); `<Tabs items: {id, label, count?, unsaved?}[] value onChange ariaLabel>children</Tabs>` (Base UI Tabs; `role="tablist"`, Kobalt-Punkt bei `unsaved`); `textModel.pages(text: string): string[][]` (Seiten aus den `segments`: `scroll`/`clear` beginnen eine Seite, `newline` eine Zeile, `code` bleibt als `{…}`-Text in der Zeile; eine Seite hat höchstens zwei Zeilen, längere Folgen werden in weitere Seiten geschnitten), `textModel.lineWidth(line): number` (Zeichen ohne Codes).

- [ ] **Step 1: Failing tests**: `pages_split_on_scroll_and_clear_and_two_lines_per_page` („A\nB\rC“ → `[["A","B"],["C"]]`), `codes_stay_in_the_line` (`{PLAYER}` bleibt), `DSScreen` zeigt Seite 2 von 3 mit Pfeil, `Tabs` wechselt per Klick und Pfeiltasten, zeigt Zähler und Punkt.
- [ ] **Step 2: Run, FAIL** → **Step 3: Implement** → **Step 4: Run, PASS**, Galerie-Seite „Bildschirm“ (Textbox mit Platzhaltertext, Schild, leer; hell und dunkel).
- [ ] **Step 5: Commit** `feat(ui): DS screen preview, tabs; text pages in the text model`

---

### Task 2: Bereich Texte (Liste, Schau, Hand)

**Files:**
- Create: `apps/studio/src/text/TextArea.tsx`, `TextList.tsx`, `TextView.tsx`, `TextHand.tsx`, `TextAreaContext.tsx` (Zustand: Archiv, Nachricht, Suche, Bereich Dieses Archiv | Alle), Tests je Datei
- Modify: `apps/studio/src/workbench/text/TextPanel.tsx` → Logik in `TextAreaContext`/`useTextArchive`, Komponente entfällt; `workbench/layout.ts` (`DECK_PANELS.texte`: Schau `textView`, Hand `textEdit` mit `problems` als Reiter; `text` verlässt die Welt-Hand; Schlüssel `v8`), `App.tsx` (Bereich `texte`; `goTo({kind:"textArchive"})` wechselt in den Bereich und wählt Archiv und Nachricht), `ui/frame/breadcrumb.ts` (`Texte › Archiv 0412, Zweiblattdorf › Nachricht 005`), `Atlas`-Sperren (`texte` frei), `workbench/panels.tsx`

**Interfaces:**
- Consumes: `useTextArchive`, `textModel` (`archiveRows`, `usersByArchive`, `filterArchives`, `segments`, `pages`, `visibleLine`, `SEARCH_LIMIT`), `ListPane`, `PanelHead`, `TreeRow`/`ListRow`, `TextField`, `Segmented`, `Button`, `Badge`, `Notice`, `DSScreen`, `Reference`, `goTo`.
- Produces: Liste „Textarchive“ mit Zähler, Gruppen *Orte* (Archive mit Nutzern aus `usersByArchive`; Name = Ortsname des ersten Headers über `buildPlaces`, Untertitel `0412, 34 Nachrichten`) und *Spielweit* (Rest, Untertitel Kennung), Suchfeld „Ort oder Archivnummer“, `+` legt ein Archiv an (`createTextArchive`), Überlängen-Zähler als Plakette je Zeile und Summenzeile „3 Texte zu lang, in 2 Archiven“; Schau: `DSScreen variant="textbox"` mit den Seiten der gewählten Nachricht, Zeile „Seite 2 von 3, 256 × 192, 2-fach“, Tasten Zurück / Weiterblättern, rechts Seiten-Karten mit „34 von 38“ je Zeile und Kobalt-Kante für die gewählte; Hand: Kopf mit Suchfeld „In allen Archiven suchen“ (`aria-label="Texte suchen"`), `Segmented` Dieses Archiv | Alle, `Button kind="primary"` „Nachricht anlegen“; Tabelle Nr. · Text (Codes als Plaketten) · Längste Zeile (Balken, `status/warnung` über 38), Zeilen `data-message`, Treffer `data-hit`; rechts Spalte „Nachricht 005“ mit mehrzeiligem `TextField` (`aria-label="Text der Nachricht"`, Commit wie heute), „Verwendet in“ (Header-Chips `data-header`), „Letzte entfernen“ als leise Taste; Fehler als `Notice`. `data-archive` an den Listenzeilen bleibt.

- [ ] **Step 1: Tests anpassen** (`TextPanel.test.tsx` → `TextArea.test.tsx` u. a.): dieselben Fälle (Archiv wählen, Nachricht ändern, anfügen, entfernen, Suche, `textTarget` öffnet Archiv und Nachricht, Warnungen) über die neuen Rollen; neu: Gruppierung Orte/Spielweit, Seitenwechsel in der Schau, Renderzeit-Test mit 800 Nachrichten (unter 100 ms in jsdom als grobe Schranke).
- [ ] **Step 2: Run, FAIL** → **Step 3: Implement** → **Step 4: Run, PASS**, Browser auf einer Platin-Kopie: Archiv 0023 (Jubelstadt) öffnen, Nachricht ändern, Vorschau blättert, Suche „Barry“ über alle Archive, `call.sh get_text_archive '{"index":23}'` zeigt die Änderung nach Speichern.
- [ ] **Step 5: Commit** `feat(studio): texts area with archive list, DS preview and message table`

---

### Task 3: Script-Panel auf Primitives, CodeMirror-Thema aus den Tokens

**Files:**
- Modify: `apps/studio/src/workbench/script/ScriptPanel.tsx` (+ test), `AdoptBox.tsx` (+ test), `ScriptEditor.tsx` (+ test), `apps/studio/package.json` (`@codemirror/theme-one-dark` entfällt), `app/App.tsx` (Dock-Reiter „Script“ mit Punkt: `scripts.dirtyCount > 0` → Dockview-Panel-Titel bekommt ein `●`-Präfix oder ein Tab-Component mit `Badge`), `workbench/Deck`-Tab-Komponente falls nötig

**Interfaces:**
- Consumes: `PanelHead`, `Tabs`, `Menu`, `Button`, `Badge`, `Notice`, `tokens.ts` `syntax` und `hell`/`dunkel`, `onThemeChange`, `effectiveTheme`.
- Produces: `ScriptPanel`: `PanelHead` mit Dokumenttitel (`docTitle`), Untertitel Dateipfad in Tüfteln, Aktionen Outline als `Menu`, Speichern, „Script anfügen“, LSP-Zustand als `Badge` (`data-lsp` bleibt, Texte wie heute) mit „Neu starten“; offene Dokumente als `Tabs` (`data-script`, `unsaved` aus `isDirty`), Schließen über ein `×`-`IconButton` je Reiter (`aria-label="<Name> schließen"`); `AdoptBox` als `Notice kind="info"` mit Fortschrittsbalken (`/* dyn */` Breite) und Bericht; leerer Zustand als `EmptyState` („Kein Script geöffnet.“ / „Eine Scriptdatei öffnet sich über den Header, ein Event oder ein Problem.“); `ScriptEditor` bekommt `theme` aus `effectiveTheme()` und folgt `onThemeChange`; `rotomTheme(mode: "hell"|"dunkel"): Extension` baut `EditorView.theme` und `HighlightStyle` aus `syntax[mode]` und `hell`/`dunkel` (Hintergrund `flaeche/blatt`, Text `text/primaer`, Auswahl `auswahl/flaeche`, Zeilennummern `text/sekundaer`, Schrift `schrift/familie/code`).

- [ ] **Step 1: Tests anpassen/neu**: `ScriptPanel.test.tsx` (Reiter-Rollen, Punkt bei dirty, Outline-Menü, LSP-Badge), `ScriptEditor.test.tsx` (`rotomTheme("dunkel")` setzt die Kommentarfarbe aus `syntax.dunkel.kommentar`; Theme-Wechsel ersetzt nur das Compartment, `doc` bleibt), `AdoptBox.test.tsx`.
- [ ] **Step 2: Run, FAIL** → **Step 3: Implement** → **Step 4: Run, PASS**, `pnpm build`, Browser: Script öffnen, Hand klappt auf ≥ 400, Thema wechseln.
- [ ] **Step 5: Commit** `feat(studio): script panel on primitives, CodeMirror theme from the tokens`

---

### Task 4: Probleme-Panel auf Primitives

**Files:**
- Modify: `apps/studio/src/workbench/problems/ProblemsPanel.tsx` (+ test)

**Interfaces:**
- Produces: `PanelHead` „Probleme“ mit Zähler (`countsText`), Tabelle (eigene `.module.css`, keine neue Primitive) mit Spalten Schwere (Symbol `error`/`warning`/`info`), Datei als `Reference` (Bereich Welt für Scripts, Texte für Archive; `onOpen = goTo(problemTarget)`), Zeile:Spalte in Mono, Meldung, Quelle als `Badge kind="info"`; in Tüfteln zeigt die Datei-Spalte den Pfad statt des Labels; `EmptyState` „Keine Probleme.“ / „Beim Speichern eines Scripts oder Texts landen Fehler hier.“; exportierte Komponente `ProblemsTable` für Technik (Task 6).

- [ ] **Step 1: Tests anpassen** (Rollen, `Reference`-Klick ruft `goTo`, Tüfteln zeigt Pfad) → **Step 2: FAIL** → **Step 3: Implement** → **Step 4: PASS** → **Step 5: Commit** `feat(studio): problems panel on primitives with references`

---

### Task 5: Assistenten als Schrittleiste im Scharnier

**Files:**
- Create: `apps/studio/src/workbench/wizards/WizardFrame.tsx` (+css, test), `apps/studio/src/app/WizardHinge.tsx` (Schrittleiste für `Hinge.wizard`)
- Modify: `workbench/wizards/WizardShell.tsx` (entfällt; Aufrufer nutzen `WizardFrame`), `NewAreaWizard.tsx`, `NewRoomWizard.tsx`, `HeaderChoice.tsx`, `PickGrid.tsx`, `WizardResult.tsx`, `wizards/HeaderSearch.tsx` (→ `Picker`), `app/WizardHost.tsx`, `App.tsx` (bei offenem `wizards.request`: `Hinge wizard={<WizardHinge …/>}`, Hand-Dock `hidden`, `WizardFrame` an dessen Stelle), `ui/frame/Deck.tsx` (Prop `handOverride?: ReactNode`), zugehörige Tests, `scripts/check-styles.allow` (Einträge der Assistenten raus)

**Interfaces:**
- Consumes: `WizardStep { id, title, content, canNext }` (unverändert), `Hinge.wizard`, `Segmented`, `NumberField`, `Reference`, `Notice`, `Button`, `Badge kind="irreversible"`, `Picker`, `escapeCloses`.
- Produces: `WizardHinge` Props `{ icon: IconName; title: string; steps: {id, title}[]; step: number; canNext: boolean; busy: boolean; last: boolean; submitLabel: string; onBack; onNext; onSubmit; onCancel }` (Bereichssymbol, Titel 15 px `font-titel`, nummerierte Schritte mit Haken, Abbrechen / Zurück / `Button kind="primary"` Weiter mit `Kbd ↵`); `WizardFrame` Props wie `WizardShell` minus Modal (`role="dialog"`, `aria-label=title`, kein Backdrop; Inhalt scrollt; `error` als `Notice kind="error"` unten; `result` ersetzt den Inhalt mit Tasten „Zeigen“ und „Schließen“); Enter führt weiter, außer in einem Feld mit Draft (`[data-draft]` unverändert → weiter) oder einem `textarea`; Escape nach `escapeCloses`; während `busy` nichts klickbar. Die Schau bleibt bedienbar (der Schritt „Eingang“ liest `lastTile` wie heute). Zusammenfassung als `Notice kind="info"` mit Plakette „Nicht rückgängig“, wo Karten entstehen.

- [ ] **Step 1: Tests anpassen** (`NewAreaWizard.test.tsx`, `NewRoomWizard.test.tsx` behalten ihre Fälle; `WizardFrame.test.tsx`: Enter/Escape-Regeln, Schrittwechsel, Ergebnis; `App.test.tsx`: offener Assistent blendet das Hand-Dock aus und zeigt die Schrittleiste).
- [ ] **Step 2: Run, FAIL** → **Step 3: Implement** → **Step 4: Run, PASS**, Browser: „Raum anlegen…“ durchlaufen; `g4emu`-Szenario `pt-raum-assistent.yaml` bleibt grün (Backend unverändert, Smoke).
- [ ] **Step 5: Commit** `feat(studio): wizards as a step bar in the hinge with their steps in the hand`

---

### Task 6: Start-Karten, Technik, Einstellungen

**Files:**
- Modify: `apps/studio/src/ui/frame/StartPane.tsx` (+ test): Projekte als Karten (Name, Spiel-Plakette, „bearbeitet vor 2 Stunden“ aus `timeAgo`, Pfad als Tooltip, Platzhalter-Fläche in `flaeche/welt` mit Anfangsbuchstabe; fehlender Ordner grau mit „nicht gefunden“ und „Aus der Liste nehmen“), Tasten „Projekt öffnen ⌘O“ primär, Hinweis „Projekte sind entpackte DSPRE-Ordner“
- Create: `apps/studio/src/technik/TechnikArea.tsx` (+ test): Liste mit *Prüfen und Bauen*, *Patches*, *Emulator*, *Werkzeuge*; Fläche rechts: Prüfen und Bauen = `ProblemsTable` + Tasten Bauen / Starten + letzter Build (Zeit, Pfad); Patches = Dynamische Header aus `headerPatchStatus` (Plakette Angewendet / Verfügbar / Fehlerhaft / Nur Pt und HGSS, Tasten „Patch anwenden“, „Patch reparieren“ über `useHeaderPatch`); Emulator = Pfad + „Emulator wählen“ (`session.onPickEmulator`); Werkzeuge = apicula (aus `useThumbnailToolReport`) und rotom-lsp (`useLsp`-Zustand, `data-lsp`) mit Pfad und Zustand; `apps/studio/src/einstellungen/SettingsArea.tsx` (+ test): Thema `Segmented` System | Hell | Dunkel (`applyTheme`), Dichte Komfort | Kompakt, Emulator-Zeile (gemeinsame Komponente `EmulatorRow` mit Technik)
- Modify: `App.tsx` (Bereiche `technik`, `einstellungen` als `pane`; ⌘O öffnet Projekt), `Atlas`-Sperren (frei), `useShortcuts.ts` (⌘O)

- [ ] **Step 1: Failing tests** (Start-Karten, fehlender Pfad, Technik-Liste wechselt die Fläche, Patch-Status-Plaketten je Zustand, Einstellungen rufen `applyTheme`/`applyDensity`).
- [ ] **Step 2: Run, FAIL** → **Step 3: Implement** → **Step 4: Run, PASS** → **Step 5: Commit** `feat(studio): start cards, Technik and settings areas`

---

### Task 7: Befehlspalette ⌘K

**Files:**
- Create: `apps/studio/src/ui/frame/Palette.tsx` (+ test), `apps/studio/src/app/paletteEntries.ts` (+ test)
- Modify: `TitleBar.tsx` (Knopf aktiv, Tooltip weg), `useShortcuts.ts` (⌘K), `App.tsx`

**Interfaces:**
- Consumes: `Picker` (`items` mit `group`, `area`, `tech`, `preview`), `buildPlaces`, `archiveRows`/`usersByArchive`, `listScriptFiles` (nur bei übernommenen Scripts), `goTo`, `wizards.open`, Befehle aus App.
- Produces: `paletteEntries({ places, headers, archives, scripts, commands, hasProject }): PickerItem[]` (Gruppen „Orte“, „Header“, „Textarchive“, „Scripts“, „Befehle“; höchstens acht je Gruppe nach Filter; ohne Projekt nur Befehle); Befehle: Projekt öffnen, Speichern, Bauen, Hier spielen, Gebiet anlegen…, Raum anlegen…, Liste ein/aus, Layout zurücksetzen (löscht die Dock-Layouts des aktuellen Bereichs und baut die Standard-Docks neu; Befund aus dem 3c.1-Review: ein herausgezogenes Einzelpanel lässt sich sonst nicht zurückholen), Thema wechseln, Dichte wechseln, Begriffe, Einstellungen, Bereich Welt/Texte/Technik; `Palette` als `Picker` oben mittig (Breite 640), Enter führt aus, Escape schließt.

- [ ] **Step 1: Failing tests**: `paletteEntries` ohne Projekt nur Befehle; „0412“ trifft Header und Archiv in zwei Gruppen; Enter auf „Speichern“ ruft `onSave`; ⌘K öffnet (`App.test.tsx`).
- [ ] **Step 2: Run, FAIL** → **Step 3: Implement** → **Step 4: Run, PASS** → **Step 5: Commit** `feat(studio): command palette on the picker`

---

### Task 8: Tüfteln im Raster, Pfade in Probleme und Scripts

**Files:**
- Modify: `apps/studio/src/workbench/MatrixPanel.tsx` (+ test), `AreaGrid.tsx` (+ test: Zellen zeigen in Tüfteln Karten-ID, Header-ID, Höhe als drei Mono-Zeilen), `workbench/inspector/MatrixCellEditor.tsx` (+ test: Abschnitt „Rohdaten, Zeile y“ mit u16 hex der Karten- und Header-Zeile und u8 der Höhen, gewählte Zelle markiert), `ProblemsPanel.tsx`, `ScriptPanel.tsx` (Pfad in Tüfteln; Task 3/4 haben die Stelle vorbereitet)

- [ ] **Step 1: Failing tests** (Zellen in Bauen ohne Zahlen, in Tüfteln mit; Rohdaten-Abschnitt hex vierstellig; Pfade) → **Step 2: FAIL** → **Step 3: Implement** → **Step 4: PASS** → **Step 5: Commit** `feat(studio): tinker depth in the raster with raw rows`

---

### Task 9: Reste aus 3c.1, leere Ausnahmeliste, Dunkel-Durchgang

**Files:**
- Modify: `scripts/check-styles.allow` (leer), alle Dateien, die die Prüfung noch nennt (`DraftInput.tsx` → `TextField`, `WebDialogHost.tsx` → `Dialog`, `WizardHost.tsx`), `workbench/SceneBar.tsx` (toter `extra`-Slot entfällt), `app/useShortcuts.ts` (Werkzeugbuchstaben nur im Bereich Welt und nur bei entsperrtem Werkzeug; Escape-Hinweis in der Statusleiste richtig, wenn etwas gewählt ist), `workbench/ContextPanel.tsx` (Enter auf einer Tür folgt dem Ziel, Ladezustand „Lade …“ statt „Hier ist noch nichts“), `apps/studio/src/ui/gallery` (Seite „Rahmen“ zeigt Scharnier im Assistenten-Modus)

- [ ] **Step 1: `bash scripts/check-styles.sh`** mit leerer Liste → Funde beheben bis Exit 0.
- [ ] **Step 2: Tests** für die Shortcut- und ContextPanel-Änderungen.
- [ ] **Step 3: Dunkel und Kompakt im Browser**: Galerie, Welt, Texte, Script, Technik, Assistent je ein Screenshot (hell, dunkel, kompakt) im Scratch; unlesbare Stellen beheben.
- [ ] **Step 4: Commit** `chore(studio): no style exceptions left, 3c.1 leftovers, dark and compact pass`

---

### Task 10: Dokumentation, Gate-Dokument, Roadmap

**Files:**
- Create: `docs/phase-3c-gate.md` (Ablauf (A)–(H) aus Spec §1 mit Rezepten für die Test-Umgebung, Ergebnistabelle leer)
- Modify: `docs/agent-gate.md` (Abschnitt „Doppeldeck: wo was ist“ auf 3c.2 ergänzt: Bereiche, Palette, Assistent im Scharnier; Rezepte 3b.2 (Text, Script) auf die neuen Beschriftungen), `docs/dev-setup.md` (Galerie-Seiten, `check-styles` ohne Ausnahmen), `docs/ROADMAP.md` (3c „implementiert, Gate offen“), `design-system/README.md` („Was die App umsetzt“ auf 3c-Stand; Liste der Abweichungen: Erzählen fehlt, Live-Vorschau fehlt, Windows-Fensterknöpfe fehlen), `docs/superpowers/plans/2026-10-23-phase-3b3a-codecs-pokemon-data.md` (Task 12: `Tabs` existiert seit 3c.2, dort nur erweitern)

- [ ] **Step 1: Docs schreiben.**
- [ ] **Step 2: Volle Prüfung**: `pnpm test`, `pnpm build`, `bash scripts/check-styles.sh`, `bash scripts/check-tokens.sh`, `cargo test --workspace --all-features`, dann `scripts/gate.sh` im Hintergrund; Tabelle in den Bericht.
- [ ] **Step 3: Commit** `docs: gate document for phase 3c, agent guide and roadmap for the Doppeldeck`

---

## Abschluss

Nach Task 10: `superpowers:finishing-a-development-branch` (Merge nach `main` nach grünem `gate.sh`). Die Abnahme (Gate 3c (A)–(G) über die Test-Umgebung, (H) gepackte App durch den Nutzer) läuft danach in einer eigenen Session; Befunde werden dort behoben. Danach beginnt 3b.3a nach seinem eigenen Plan.
