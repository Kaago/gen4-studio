# Open Source und Release v1.0.0: Implementierungsplan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Gen4 Studio wird als v1.0.0 öffentlich (MIT, `Kaago/gen4-studio`), auf Englisch und Deutsch bedienbar, mit README, Community-Dateien, englischen Nutzer-Docs und vorbereiteten Launch-Texten.

**Architecture:** Drei Arbeitspakete auf zwei Branches und main. (1) `oss-readiness`: nur Dateien um die App herum (README, Community-Dateien, `docs/README.md`, CI-Auslöser, Repo-Metadaten). (2) `i18n`: i18next-Kataloge je Frontend-Ordner mit typisierten Schlüsseln, `rust-i18n`-Kataloge in `g4-kernel` und `studio`, Locale vom Frontend per Tauri-Befehl, kopflos per `G4_LANG`; Englisch ist Standard und Fallback, Tests halten de und en auf gleichen Schlüsselmengen. (3) main: Version 1.0.0, Umschalten auf public, Release über die vorhandenen Skripte, Launch-Texte.

**Tech Stack:** Rust 2024 (rust-i18n 4.2.4, serde_yaml), Tauri 2, React 19, TypeScript, i18next 27.0.1, react-i18next 18.0.1, vitest, bash 3.2, `gh` 2.103.

**Spec:** [docs/superpowers/specs/2026-11-09-open-source-release-v1-design.md](../specs/2026-11-09-open-source-release-v1-design.md)

## Global Constraints

- Arbeitspaket 1 ändert keinen App-Code; Arbeitspaket 2 ändert keine Codes (`CmdError.code`, Prüfcodes, MCP-Werkzeugnamen, `schemaVersion`).
- Englisch ist Standard- und Fallback-Sprache; Deutsch ist vollständig (Spec 2).
- Frontend-Schlüssel `camelCase`, verschachtelt nach Komponente, Platzhalter `{{name}}`, Plural `_one`/`_other` mit `count` (Spec 4.2). Rust-Schlüssel `module.snake_case`, Platzhalter `%{name}` (Spec 4.3).
- Frontend-Tests laufen mit `lng: "de"` und behalten deutsche Zusicherungen; Rust-Tests sichern Englisch zu (Spec 4.2, 4.3).
- Neue Dateien außerhalb der Kataloge sind Englisch, außer `README.de.md` (Spec 3).
- Verweis auf DSPRE nie als „based on“ oder „fork of“ (Spec 2).
- Keine ROM-Daten, keine Screenshots mit Spielgrafik außer dem, was die App ohnehin zeigt (`scripts/check-no-rom-data.sh` bleibt grün).
- Shell-Skripte laufen auf bash 3.2 und Git Bash; `scripts/tests/*.test.sh` ist das Testformat.
- Fassung nur über `scripts/version.sh`.

## Review Focus

1. Ein Nutzer mit Systemsprache `de-AT` oder `de-CH` erwartet Deutsch, mit `en-GB` oder `fr` Englisch: `detectLanguage` prüft nur das Präfix (Test in Task 7).
2. Ein Fehler-Toast kurz nach dem Start, bevor `set_locale` zurückkam, darf nicht die falsche Sprache tragen: `applyLanguage` ist erst erfüllt, wenn `set_locale` zurückkam, und `main.tsx` wartet darauf vor dem ersten Render (Test in Task 7).
3. Ein Schlüssel, den `de.json` hat und `en.json` nicht, fällt stumm auf Englisch zurück und bleibt unbemerkt: `catalogs.test.ts` und `check_catalogs` scheitern bei Differenzen (Tasks 7 und 8).
4. Ein Plural-Text mit `count: 0` oder `count: 1` liefert in Englisch und Deutsch die richtige Form („1 file“, „0 Dateien“): Test in Task 11.
5. Eine `%{name}`-Variable, die der Rust-Aufruf nicht übergibt, erscheint wörtlich im Text: `check_catalogs` vergleicht die Variablenmengen beider Sprachen, und je Kernel-Modul sichert ein Test eine Meldung mit Variable zu (Tasks 8, 19, 20).

---

## Teil A: Arbeitspaket 1, Branch `oss-readiness`

Basis: `main` nach dem Merge von Phase 6. Worktree über `superpowers:using-git-worktrees`.

### Task 1: README in beiden Sprachen

**Files:**
- Create: `README.md`, `README.de.md`
- Create: `docs/media/.gitkeep`
- Create: `scripts/tests/readme.test.sh`

**Interfaces:**
- Produces: `docs/media/hero-welt-bauen.png`, `docs/media/erzaehlen.png`, `docs/media/technik-aenderungen.png` als Pfade, die Task 6 liefert; Abschnittsüberschriften, die `README.de.md` spiegelt.

- [ ] **Step 1: Test schreiben**

`scripts/tests/readme.test.sh`: holt die `##`-Überschriften beider Dateien (`grep -E '^## '`), zählt sie (erwartet 12) und prüft, dass beide Listen gleich lang sind und jeder Abschnitt der englischen Datei einen Partner in derselben Position hat (Vergleich der Reihenfolge über eine Nummerierung in einem HTML-Kommentar `<!-- section: install -->` über jeder Überschrift; beide Dateien tragen dieselben Kommentare). Prüft außerdem, dass `README.md` die Zeichenketten `"based on DSPRE"` und `"fork of DSPRE"` nicht enthält. Ausgabe bei Erfolg: `readme: ok`.

- [ ] **Step 2: Test laufen lassen, muss scheitern**

Run: `bash scripts/tests/readme.test.sh`
Expected: Exit 1, `README.md: Datei fehlt` oder Abschnittszahl 0.

- [ ] **Step 3: `README.md` schreiben**

Die zwölf Abschnitte aus Spec 3.1 in dieser Reihenfolge, jeder mit dem Kommentar `<!-- section: <slug> -->` davor (Slugs: `about`, `screenshots`, `features`, `games-platforms`, `install`, `quickstart`, `agents`, `project-format`, `dspre`, `acknowledgements`, `legal`, `build-contribute-license`). Der Pitch, der Legal-Text und der DSPRE-Absatz stehen wörtlich in Spec 2, 3.1 (11) und unten. Badges: `https://img.shields.io/github/license/Kaago/gen4-studio`, `https://img.shields.io/github/v/release/Kaago/gen4-studio`, `https://github.com/Kaago/gen4-studio/actions/workflows/ci.yml/badge.svg`. Bilder relativ: `docs/media/hero-welt-bauen.png` usw. Sprachzeile unter dem Titel: `English · [Deutsch](README.de.md)`.

DSPRE-Absatz (wörtlich):

> **Relationship to DSPRE.** Gen4 Studio would not exist without [DSPRE](https://github.com/DS-Pokemon-Rom-Editor/DSPRE), the DS Pokémon ROM Editor, and the years of reverse-engineering work by its maintainers and the DS hacking community. DSPRE is the reference for Gen IV format knowledge and served as our test oracle: Gen4 Studio produces byte-identical output on unchanged data. Gen4 Studio is an independent reimplementation in Rust and TypeScript and shares no code with DSPRE. It reads and writes the DSPRE project layout, so you can open an existing DSPRE project in Gen4 Studio and keep using both tools side by side. If you need the full breadth of DSPRE's roughly one hundred editors, DSPRE remains the right tool; Gen4 Studio focuses on world building, storytelling, scripts, a text-based project format, and agent integration.

Die Quickstart-Schritte nennen die Menüpunkte so, wie die englische UI sie nach Arbeitspaket 2 trägt: „New game from ROM…“, „Play here“, „Publish…“. Install nennt die Gatekeeper-Schritte aus `docs/release.md` (Abschnitt „Signatur“).

- [ ] **Step 4: `README.de.md` schreiben**

Inhaltsgleich, dieselben zwölf Kommentare, deutsche Überschriften, Sprachzeile `[English](README.md) · Deutsch`.

- [ ] **Step 5: Test laufen lassen**

Run: `bash scripts/tests/readme.test.sh`
Expected: `readme: ok`

- [ ] **Step 6: Commit**

```bash
git add README.md README.de.md docs/media/.gitkeep scripts/tests/readme.test.sh
git commit -m "docs(oss): README in English and German with DSPRE acknowledgement"
```

### Task 2: Community-Dateien

**Files:**
- Create: `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, `SECURITY.md`
- Create: `.github/ISSUE_TEMPLATE/bug.yml`, `.github/ISSUE_TEMPLATE/feature.yml`, `.github/ISSUE_TEMPLATE/question.yml`, `.github/ISSUE_TEMPLATE/config.yml`
- Create: `.github/PULL_REQUEST_TEMPLATE.md`, `.github/dependabot.yml`
- Create: `scripts/tests/community-files.test.sh`

- [ ] **Step 1: Test schreiben**

`scripts/tests/community-files.test.sh`: jede der zehn Dateien existiert und ist nicht leer; `bug.yml` enthält `id: no-rom` und `required: true` in demselben Block (Checkbox); `config.yml` enthält `blank_issues_enabled: false`; `dependabot.yml` enthält die drei `package-ecosystem`-Werte `cargo`, `npm`, `github-actions` und `directory: "/apps/studio"` beim npm-Eintrag; `CODE_OF_CONDUCT.md` enthält `Contributor Covenant` und `2.1`. Ausgabe: `community files: ok`.

- [ ] **Step 2: Test laufen lassen, muss scheitern**

Run: `bash scripts/tests/community-files.test.sh`
Expected: Exit 1, erste fehlende Datei genannt.

- [ ] **Step 3: Dateien schreiben**

Inhalte nach Spec 3.2. `CONTRIBUTING.md` nennt den Befehl `G4_GATE_HERMETIC=1 bash scripts/gate.sh` und die Katalogpflicht („every user-facing string goes into both `de` and `en` catalogs“). `SECURITY.md`: Private Vulnerability Reporting, 14 Tage. `dependabot.yml`: `schedule.interval: weekly`, je Ökosystem eine `groups`-Regel `minor-and-patch` mit `update-types: ["minor", "patch"]`. Issue-Formulare mit den Feldern aus Spec 3.2; die Checkbox in `bug.yml`:

```yaml
  - type: checkboxes
    id: no-rom
    attributes:
      label: No game data
      options:
        - label: I have not attached a ROM, a save game or any file derived from one.
          required: true
```

- [ ] **Step 4: Test laufen lassen**

Run: `bash scripts/tests/community-files.test.sh`
Expected: `community files: ok`

- [ ] **Step 5: Commit**

```bash
git add CONTRIBUTING.md CODE_OF_CONDUCT.md SECURITY.md .github
git commit -m "docs(oss): contributing, code of conduct, security policy, issue and PR templates, dependabot"
```

### Task 3: Docs einordnen und Links prüfen

**Files:**
- Create: `docs/README.md`
- Create: `scripts/check-links.sh`, `scripts/tests/check-links.test.sh`
- Modify: `docs/research/2026-10-20-dspre-und-suite-analyse.md:1-3` (Vorspann)
- Modify: `design-system/openpencil/BRIEFING-APP-SCREENS.md:12-13` (absolute Pfade)
- Modify: `scripts/gate.sh` (ein `run_step` für `check-links.sh`, hermetisch, nach „no rom data“)

**Interfaces:**
- Produces: `scripts/check-links.sh [<md>...]`: prüft relative Markdown-Links `[..](pfad)` und `[..](pfad#anker)` in den übergebenen Dateien (ohne Argument: `README.md README.de.md docs/README.md CONTRIBUTING.md`), ob die Zieldatei existiert; Ausgabe je Treffer `link? <datei>: <ziel>`, Exit 1; sonst `links: <n> ok`. Externe URLs werden nicht geprüft.

- [ ] **Step 1: Test für `check-links.sh` schreiben**

`scripts/tests/check-links.test.sh`: legt in `mktemp -d` eine `a.md` mit einem gültigen Link auf `b.md`, einem gültigen `b.md#kopf` und einem ungültigen `fehlt.md` an; erwartet Exit 1 und genau eine Zeile `link? a.md: fehlt.md`; dann ohne den schlechten Link Exit 0 und `links: 2 ok`.

- [ ] **Step 2: Test laufen lassen, muss scheitern**

Run: `bash scripts/tests/check-links.test.sh`
Expected: Exit 1, `check-links.sh: Datei fehlt`.

- [ ] **Step 3: `scripts/check-links.sh` schreiben**

bash 3.2, python3 `-I` im Heredoc wie `check-no-rom-data.sh`: regulärer Ausdruck `\]\(([^)#\s]+)(#[^)]*)?\)`, überspringt Ziele mit `://` und `mailto:`, löst relativ zur Datei auf.

- [ ] **Step 4: Test laufen lassen**

Run: `bash scripts/tests/check-links.test.sh`
Expected: Exit 0.

- [ ] **Step 5: `docs/README.md` schreiben, Vorspann und Pfade ändern, Gate-Schritt**

`docs/README.md` nach Spec 3.3 (drei Teile, Methodik-Absatz). Vorspann der DSPRE-Analyse (deutsch, zwei Sätze, Spec 3.3). In `BRIEFING-APP-SCREENS.md` verlieren `build-screen.sh` und `op.mjs` ihr absolutes Pfadpräfix: `bash design-system/openpencil/build-screen.sh` und `node design-system/scripts/op.mjs` (Aufruf aus der Repo-Wurzel, Satz davor anpassen). `scripts/gate.sh`: `run_step "links" "" bash scripts/check-links.sh` direkt nach dem Schritt „no rom data“.

- [ ] **Step 6: Prüfen**

Run: `bash scripts/check-links.sh && ! git grep -n '/Users/POLT[E]P' && bash scripts/tests/check-links.test.sh`
Expected: `links: <n> ok`, kein Treffer, Exit 0.

- [ ] **Step 7: Commit**

```bash
git add docs/README.md scripts/check-links.sh scripts/tests/check-links.test.sh docs/research/2026-10-20-dspre-und-suite-analyse.md design-system/openpencil/BRIEFING-APP-SCREENS.md scripts/gate.sh
git commit -m "docs(oss): docs map, link check in the gate, no absolute paths"
```

### Task 4: CI bei Pull Requests

**Files:**
- Modify: `.github/workflows/ci.yml:1-30` (Kopfkommentar und `on:`)

- [ ] **Step 1: Auslöser ergänzen**

`on:` bekommt zusätzlich

```yaml
  pull_request:
    branches: [main]
    paths-ignore: ['**.md', 'docs/**', 'design-system/**']
  push:
    branches: [main]
    paths-ignore: ['**.md', 'docs/**', 'design-system/**']
```

`gate-windows` behält `if: ${{ inputs.windows }}` (läuft nur manuell). Der Kopfkommentar erklärt die Kostenlage neu: Standard-Runner sind für öffentliche Repositories kostenlos, deshalb läuft `gate-macos` bei jedem PR; `concurrency.group` wird `ci-${{ github.event.pull_request.number || github.ref }}`.

- [ ] **Step 2: Prüfen**

Run: `bash scripts/check-actions.sh && ruby -ryaml -e "YAML.load_file('.github/workflows/ci.yml'); puts 'yaml ok'"`
Expected: Actions-Audit grün, `yaml ok`.

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "ci: run the hermetic gate on pull requests and pushes to main"
```

### Task 5: Repo-Metadaten

**Files:** keine im Repo; GitHub-Einstellungen.

- [ ] **Step 1: Setzen**

```bash
gh repo edit Kaago/gen4-studio \
  --description "Cross-platform editor for Pokémon Gen 4 (DPPt, HGSS) ROM hacks: worlds, stories, scripts, data, text-based project with Git, build to ROM or xdelta, MCP for AI agents. DSPRE-compatible." \
  --homepage "https://github.com/Kaago/gen4-studio#readme" \
  --enable-discussions --enable-wiki=false --enable-projects=false
for t in pokemon pokemon-rom-hack rom-hacking romhacking nintendo-ds heartgold soulsilver platinum map-editor level-editor game-tools mcp mcp-server tauri rust react typescript threejs desktop-app dspre; do gh repo edit Kaago/gen4-studio --add-topic "$t"; done
```

- [ ] **Step 2: Prüfen**

Run: `gh repo view Kaago/gen4-studio --json description,homepageUrl,repositoryTopics,hasDiscussionsEnabled,hasWikiEnabled --jq '{d:.description|length, t:(.repositoryTopics|length), disc:.hasDiscussionsEnabled, wiki:.hasWikiEnabled}'`
Expected: `t` ist 20, `disc` true, `wiki` false.

### Task 6: Screenshots (Hand, Maintainer)

**Files:**
- Create: `docs/media/hero-welt-bauen.png`, `docs/media/erzaehlen.png`, `docs/media/technik-aenderungen.png`

- [ ] **Step 1: Aufnehmen**

Gepackte App oder `pnpm tauri dev`, Hell, Komfort, Fenster 1440×900, Projekt Platin DE oder HeartGold: (1) Welt bauen mit texturierter Route und Inspector, (2) Erzählen mit einer Geschichte im Graph, (3) Technik › Änderungen mit einem Diff. Nach Arbeitspaket 2 in englischer Sprache erneut aufnehmen (die Bilder im README zeigen die englische UI).

- [ ] **Step 2: Prüfen und Commit**

Run: `bash scripts/check-no-rom-data.sh docs/media && bash scripts/check-links.sh`
Expected: `no rom data: 3 files ok`, `links: <n> ok`.

```bash
git add docs/media
git commit -m "docs(oss): screenshots for the README"
```

Teil A endet mit `G4_GATE_HERMETIC=1 bash scripts/gate.sh` grün und dem Merge von `oss-readiness` nach main (`superpowers:finishing-a-development-branch`).

---

## Teil B: Arbeitspaket 2, Branch `i18n`

Basis: `main` nach Teil A. Die Tasks 7 und 8 legen die Mechanik, 9 bis 11 die zentralen Module, 12 bis 18 wandern die Bereiche, 19 bis 22 die Rust-Seite, 23 die Docs, 24 das Gate.

### Task 7: Frontend-i18n-Fundament

**Files:**
- Modify: `apps/studio/package.json` (dependencies `i18next: "27.0.1"`, `react-i18next: "18.0.1"`)
- Create: `apps/studio/src/i18n/index.ts`, `apps/studio/src/i18n/i18next.d.ts`, `apps/studio/src/i18n/index.test.ts`, `apps/studio/src/i18n/catalogs.test.ts`
- Create: `apps/studio/src/i18n/locales/de/common.json`, `apps/studio/src/i18n/locales/en/common.json` (zunächst mit einem Schlüssel `appName: "Gen4 Studio"`)
- Modify: `apps/studio/vitest.setup.ts` (ruft `init("de")` vor allem anderen)
- Modify: `apps/studio/src/main.tsx` (ruft `init(readLanguage())` und wartet `applyLanguage(readLanguage())` ab, bevor `createRoot` rendert; der Start wird dafür eine `async`-Funktion `boot()`)
- Modify: `apps/studio/index.html:2` (`lang="en"`; der Start setzt den echten Wert)
- Modify: `apps/studio/src/api.base.ts` (neu `setLocale`)

**Interfaces:**
- Produces (`apps/studio/src/i18n/index.ts`):
  - `export type Language = "system" | "de" | "en"`
  - `export type Effective = "de" | "en"`
  - `export const LANGUAGE_KEY = "g4.lang"`
  - `export const defaultNS = "common"`
  - `export const resources: { de: {...}, en: {...} }` (`as const`; jeder Namespace importiert sein JSON; Task 10 bis 18 ergänzen Einträge)
  - `export function detectLanguage(navigatorLanguage: string): Effective` (Präfix `de` vor `-`/`_`, Groß/Klein egal → `"de"`, sonst `"en"`)
  - `export function readLanguage(): Language` (aus `localStorage`, ungültig → `"system"`)
  - `export function effectiveLanguage(l: Language = readLanguage()): Effective`
  - `export function init(l: Language): void` (idempotent; `i18next.use(initReactI18next).init({ lng: effectiveLanguage(l), fallbackLng: "en", defaultNS, resources, interpolation: { escapeValue: false }, returnNull: false })`, setzt `document.documentElement.lang`)
  - `export async function applyLanguage(l: Language): Promise<void>` (schreibt `localStorage`, `changeLanguage`, `documentElement.lang`, dann `await setLocale(effective)`)
  - `export function t(...)`: Re-Export von `i18next.t` für Module ohne React.
- Produces (`apps/studio/src/api.base.ts`): `export async function setLocale(lang: Effective): Promise<void>` (`invoke("set_locale", { lang })` in Tauri, sonst sofort erfüllt).
- Consumes: Tauri-Befehl `set_locale` aus Task 8 (bis dahin scheitert der Aufruf in Tauri; der Browser-Modus ist davon frei).

- [ ] **Step 1: Tests schreiben**

`index.test.ts`:
- `detectLanguage("de-AT")`, `("DE")`, `("de_CH")` → `"de"`; `("en-GB")`, `("fr")`, `("")` → `"en"`.
- `readLanguage()` ohne Eintrag → `"system"`; mit `localStorage.setItem("g4.lang", "en")` → `"en"`; mit `"xx"` → `"system"`.
- `effectiveLanguage("system")` folgt `detectLanguage(navigator.language)` (navigator in jsdom auf `"de-DE"` stubben: `Object.defineProperty(navigator, "language", { value: "de-DE", configurable: true })`).
- `applyLanguage("en")` setzt `document.documentElement.lang` auf `"en"` und ruft `setLocale` mit `"en"` (`vi.mock("../api.base")`), und erst nach dessen Auflösung ist das Promise erfüllt (Review Focus 2: `setLocale` gibt ein steuerbares Promise zurück; vor `resolve()` ist `applyLanguage` pending).

`catalogs.test.ts`: iteriert über `resources.de` und `resources.en`; für jeden Namespace: `flatten` (Punkte) beider Objekte, Schlüsselmengen gleich (Fehlermeldung nennt Namespace und die fehlenden Schlüssel je Seite), jeder Wert `typeof string` und `trim().length > 0`, Platzhaltermengen (`/\{\{(\w+)\}\}/g`) je Schlüssel gleich, zu jedem `…_one` existiert `…_other` und umgekehrt.

- [ ] **Step 2: Tests laufen lassen, müssen scheitern**

Run: `cd apps/studio && pnpm vitest run src/i18n`
Expected: FAIL, Modul `./index` fehlt.

- [ ] **Step 3: Pakete installieren, Modul schreiben, Setup und Start anpassen**

`pnpm add i18next@27.0.1 react-i18next@18.0.1` in `apps/studio` (feste Versionen wie im Rest der `package.json`). `i18next.d.ts` nach dem i18next-Muster (`CustomTypeOptions` mit `defaultNS: typeof defaultNS; resources: typeof resources["en"]`). `vitest.setup.ts`: erste Zeile `import { init } from "./src/i18n"; init("de");`. `main.tsx`: `init(readLanguage())` vor `applyTheme`.

- [ ] **Step 4: Tests laufen lassen**

Run: `cd apps/studio && pnpm vitest run src/i18n && bash ../../scripts/check-licenses-npm.sh`
Expected: PASS; `npm licenses: 0 of <n> packages not allowed`.

- [ ] **Step 5: Commit**

```bash
git add apps/studio/package.json apps/studio/pnpm-lock.yaml apps/studio/src/i18n apps/studio/vitest.setup.ts apps/studio/src/main.tsx apps/studio/index.html apps/studio/src/api.base.ts
git commit -m "feat(i18n): i18next foundation, language detection, catalog parity test"
```

### Task 8: Rust-i18n-Fundament und `set_locale`

**Files:**
- Modify: `Cargo.toml` (`[workspace.dependencies] rust-i18n = "4.2.4"`), `crates/g4-kernel/Cargo.toml`, `apps/studio/src-tauri/Cargo.toml` (`rust-i18n = { workspace = true }`)
- Create: `crates/g4-kernel/src/i18n.rs`, `crates/g4-kernel/locales/en.yml`, `crates/g4-kernel/locales/de.yml` (zunächst je ein Schlüssel `i18n.probe: "probe"` / `"Probe"`)
- Create: `apps/studio/src-tauri/locales/en.yml`, `apps/studio/src-tauri/locales/de.yml` (je `i18n.probe`)
- Modify: `crates/g4-kernel/src/lib.rs` (`#[macro_use] extern crate rust_i18n; i18n!("locales", fallback = "en"); pub mod i18n;`)
- Modify: `apps/studio/src-tauri/src/lib.rs` (dito ohne `pub mod i18n`; Befehl registrieren)
- Create: `apps/studio/src-tauri/src/commands/locale.rs`
- Modify: `apps/studio/src-tauri/src/main.rs:7-16`, `apps/studio/src-tauri/src/bin/studio-bridge.rs` (`main`), `crates/g4-kernel/src/bin/g4k.rs` (`main`): erste Zeile `g4_kernel::i18n::init_from_env();`
- Create: `crates/g4-kernel/tests/i18n.rs`, `apps/studio/src-tauri/tests/locales.rs`

**Interfaces:**
- Produces (`g4_kernel::i18n`):
  - `pub const DEFAULT: &str = "en"`
  - `pub fn set_locale(lang: &str)` (`"de"` → `de`, alles andere → `en`; ruft `rust_i18n::set_locale`)
  - `pub fn locale() -> String`
  - `pub fn init_from_env()` (`G4_LANG` gesetzt → `set_locale(G4_LANG)`; sonst `LANG` mit Präfix `de` → `de`; sonst `en`)
  - `pub fn check_catalogs(dir: &Path) -> Result<usize, Vec<String>>` (liest `en.yml` und `de.yml` mit `serde_yaml`, flacht verschachtelte Maps mit `.` ab, vergleicht Schlüsselmengen, prüft je Schlüssel gleiche `%{…}`-Mengen und nicht leere Werte; `Ok(n)` = Schlüsselzahl)
- Produces (Tauri): `#[tauri::command] pub fn set_locale(lang: String)` in `commands/locale.rs`, registriert in `generate_handler!`.
- Consumes: Task 7 ruft `set_locale` über `invoke`.

- [ ] **Step 1: Tests schreiben**

`crates/g4-kernel/tests/i18n.rs`:
- `catalogs_agree`: `check_catalogs(Path::new(env!("CARGO_MANIFEST_DIR")).join("locales"))` ist `Ok(n)` mit `n >= 1`.
- `check_catalogs_reports_missing_key`: in `tempfile::tempdir()` `en.yml` mit `a: x` und `b: y`, `de.yml` mit `a: x`; `Err` enthält einen String mit `b`.
- `check_catalogs_reports_variable_mismatch`: `en.yml` `a: "%{n} files"`, `de.yml` `a: "Dateien"`; `Err` nennt `a`.
- `set_locale_normalizes`: `set_locale("de-AT")` → `locale() == "de"`; `set_locale("xx")` → `"en"`; danach `set_locale("en")`.
- `probe_translates`: `set_locale("de")`, `t!("i18n.probe") == "Probe"`, `set_locale("en")`, `== "probe"`.

`apps/studio/src-tauri/tests/locales.rs`: `catalogs_agree` wie oben mit dem Pfad der Tauri-Crate.

- [ ] **Step 2: Tests laufen lassen, müssen scheitern**

Run: `cargo test -p g4-kernel --test i18n`
Expected: Compile-Fehler, `g4_kernel::i18n` fehlt.

- [ ] **Step 3: Abhängigkeiten, Modul, Kataloge, Befehl, `init_from_env`-Aufrufe**

`rust_i18n::set_locale` ist eine globale Einstellung des `rust-i18n`-Crates und gilt damit für Kernel und Tauri-Crate gemeinsam. `locale()` gibt `rust_i18n::locale().to_string()`. `set_locale` im Tauri-Befehl ruft `g4_kernel::i18n::set_locale(&lang)`.

- [ ] **Step 4: Tests und Lizenzprüfung laufen lassen**

Run: `cargo test -p g4-kernel --test i18n && cargo test -p studio --test locales && cargo deny check licenses`
Expected: PASS; `licenses ok`.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml Cargo.lock crates/g4-kernel apps/studio/src-tauri
git commit -m "feat(i18n): rust-i18n catalogs in kernel and studio, set_locale command, G4_LANG"
```

### Task 9: Sprachwahl in den Einstellungen

**Files:**
- Modify: `apps/studio/src/einstellungen/SettingsArea.tsx` (Prop `language: Language`, `onLanguage: (l: Language) => void`, `Segmented` mit Optionen `system`, `de`, `en`; Beschriftungen aus Namespace `app`: `settings.language`, `settings.languageSystem`, `settings.languageDe` = „Deutsch“, `settings.languageEn` = „English“ in beiden Katalogen gleich)
- Modify: `apps/studio/src/einstellungen/SettingsArea.test.tsx`
- Modify: `apps/studio/src/App.tsx` (Zustand `language` wie `theme`; `onLanguage` ruft `applyLanguage`)
- Create: `apps/studio/src/i18n/locales/{de,en}/app.json` (Einträge `settings.*`; der Rest von `app` folgt in Task 12)

**Interfaces:**
- Consumes: `Language`, `readLanguage`, `applyLanguage` aus Task 7.

- [ ] **Step 1: Test schreiben**

In `SettingsArea.test.tsx`: `renders language switch with three options and reports a change`: rendert mit `language="system"`, findet die Gruppe mit `aria-label` „Sprache“ (Test läuft in `de`), drei Optionen „System“, „Deutsch“, „English“; Klick auf „English“ ruft `onLanguage("en")`.

- [ ] **Step 2: Test laufen lassen, muss scheitern**

Run: `cd apps/studio && pnpm vitest run src/einstellungen/SettingsArea.test.tsx -t "language switch"`
Expected: FAIL, Gruppe nicht gefunden.

- [ ] **Step 3: Umsetzen**

Die neue Zeile steht unter „Dichte“ in der Spalte „Aussehen“; `App.tsx` reicht `language` und `onLanguage` durch wie `theme`/`onTheme`.

- [ ] **Step 4: Tests laufen lassen**

Run: `cd apps/studio && pnpm vitest run src/einstellungen src/App.test.tsx src/i18n`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add apps/studio/src/einstellungen apps/studio/src/App.tsx apps/studio/src/i18n
git commit -m "feat(i18n): language switch in settings"
```

### Task 10: Prüfmeldungen in Kataloge

**Files:**
- Create: `apps/studio/src/i18n/locales/{de,en}/checks.json`
- Modify: `apps/studio/src/checks/messages.ts` (`CHECK_MESSAGES` behält `group`, `severity`; `title`, `text`, `one`, `building` kommen aus `t("checks:<code>.title")` usw.; der bisherige Renderer der Sätze (`{n}`, `{n:id}`, `{n:field}`) bleibt und liest den Satz aus dem Katalog)
- Modify: `apps/studio/src/checks/messages.test.ts`
- Modify: `apps/studio/src-tauri/src/commands/checks/messages_ts.rs` (liest `../../../../src/i18n/locales/de/checks.json` und `…/en/checks.json` per `include_str!` und `serde_json`; hält die Codes beider Dateien gegen `Registry::all()`, in Katalogreihenfolge; `group` und `severity` weiter aus `messages.ts`)
- Modify: `apps/studio/src-tauri/Cargo.toml:36` (`serde_json = { version = "1", features = ["preserve_order"] }`, damit die JSON-Reihenfolge erhalten bleibt)
- Modify: `apps/studio/src/i18n/index.ts` (Namespace `checks` in `resources`)

**Interfaces:**
- Produces: Katalogform `"<code>": { "title": "…", "text": "…", "one"?: "…", "building"?: { "title"?: "…", "text": "…" } }`.

- [ ] **Step 1: Tests anpassen**

`messages.test.ts`: bestehende Tests bleiben (de). Neu `renders an English sentence after changeLanguage("en")`: `i18next.changeLanguage("en")`, Satz für `warp.dest-header` mit Argumenten `[414, 600]` enthält `Header 0414` und `600 headers`; danach zurück auf `de`. `messages_ts.rs`: Test `catalogs_hold_every_code` für beide Sprachen.

- [ ] **Step 2: Laufen lassen, muss scheitern**

Run: `cd apps/studio && pnpm vitest run src/checks/messages.test.ts; cargo test -p studio messages_ts`
Expected: FAIL (Katalog fehlt).

- [ ] **Step 3: Umsetzen**

Deutsche Texte 1:1 aus `messages.ts` nach `de/checks.json`; englische Fassungen in `en/checks.json` (Register: ruhig, zweiter Satz nennt den Ausweg, wie im Deutschen). Platzhalter `{1}`, `{1:id}` bleiben, weil der eigene Renderer sie füllt; i18next-Interpolation greift nicht (`{{`).

- [ ] **Step 4: Laufen lassen**

Run: `cd apps/studio && pnpm vitest run src/checks src/i18n && cargo test -p studio messages_ts`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add apps/studio/src/checks apps/studio/src/i18n apps/studio/src-tauri/src/commands/checks/messages_ts.rs
git commit -m "feat(i18n): check messages in de and en catalogs, kernel test reads both"
```

### Task 11: Begriffe und Zählwörter

**Files:**
- Create: `apps/studio/src/i18n/locales/{de,en}/terms.json`
- Modify: `apps/studio/src/workbench/terms.ts` (`TERMS` behält `TermKey` und `tech`; `label`, `plural`, `short` über `t("terms:<key>.label")` usw. als Getter-Funktionen `termLabel(key)`, `termPlural(key)`, `termShort(key)`; bestehende Aufrufer werden auf die Funktionen umgestellt)
- Modify: `apps/studio/src/words.ts` (`count` entfällt; `NBSP`, `WJ` bleiben), Aufrufer von `count` nutzen Plural-Schlüssel in ihrem Namespace (`t("…Count", { count: n })`, Werte `"{{count}} Datei"` / `"{{count}} Dateien"` mit `NBSP` als Zeichen im Katalog)
- Modify: `apps/studio/src/workbench/terms.test.ts`, `apps/studio/src/words.test.ts`

- [ ] **Step 1: Tests schreiben**

`terms.test.ts`: `every term has label, plural and short in both languages` (über `resources.de.terms` und `resources.en.terms`, alle `TermKey`). `words.test.ts`: `count` entfällt; neuer Test in `apps/studio/src/i18n/index.test.ts`: `plural keys pick one and other in both languages` mit einem Probeschlüssel `common:filesChanged` (`_one`/`_other` in beiden Katalogen): `count: 1` → „1 Datei hat sich geändert“ / „1 file changed“, `count: 0` → Plural, `count: 2` → Plural (Review Focus 4).

- [ ] **Step 2: Laufen lassen, muss scheitern**

Run: `cd apps/studio && pnpm vitest run src/workbench/terms.test.ts src/words.test.ts src/i18n`
Expected: FAIL.

- [ ] **Step 3: Umsetzen**

- [ ] **Step 4: Laufen lassen**

Run: `cd apps/studio && pnpm test`
Expected: PASS (alle Tests; Aufrufer von `count` sind umgestellt).

- [ ] **Step 5: Commit**

```bash
git add apps/studio/src
git commit -m "feat(i18n): terms and plural words from catalogs"
```

### Rezept für die Bereichs-Tasks 12 bis 18

Jeder Bereichs-Task läuft gleich; die Tasks unten nennen nur Ordner, Namespace, Dateizahl und die Komponente des Englisch-Tests.

1. **Test**: `apps/studio/src/<ordner>/<Komponente>.i18n.test.tsx`: rendert die genannte Komponente nach `i18next.changeLanguage("en")` und findet zwei genannte englische Texte; `afterEach` setzt `de` zurück. Zusätzlich die Grep-Zusicherung als Shell-Schritt (unten).
2. **Laufen lassen**: FAIL (englische Texte fehlen).
3. **Wandern**: Jeder deutsche Text der Dateien des Ordners (JSX-Text, `aria-label`, `title`, `placeholder`, Toast- und Dialogtexte, Konstanten) wird ein Schlüssel in `locales/de/<ns>.json`; Schlüssel nach Komponente verschachtelt; `en/<ns>.json` mit der englischen Fassung. Komponenten: `const { t } = useTranslation("<ns>")`; Module: `t("<ns>:key")`. Plurale über `_one`/`_other`. Texte, die nur Tests sehen (`*.test.*`), bleiben. `resources` in `i18n/index.ts` bekommt den Namespace.
4. **Grep-Zusicherung**: `git grep -nE '"[^"]*[äöüÄÖÜß][^"]*"|>[^<{]*[äöüÄÖÜß][^<{]*<' -- 'apps/studio/src/<ordner>/**/*.ts' 'apps/studio/src/<ordner>/**/*.tsx' ':!**/*.test.*'` liefert nichts (Umlaute als Indikator; der Blick in Task 24 fängt den Rest).
5. **Laufen lassen**: `cd apps/studio && pnpm test && pnpm build` grün (Build prüft die typisierten Schlüssel).
6. **Commit**: `feat(i18n): <ordner> speaks English and German`.

Englischer Wortschatz, einheitlich (aus `docs/en/mcp.md`, Task 23, und `terms.json`): Welt → World, Gebiet → Area, Raum → Room, Ort → Place, Weltkarte → World map, Bauen → Build, Tüfteln → Tinker, Erzählen → Tell, Technik → Technical, Werkbank → Workbench, Schau → View, Hand → Hand, Scharnier → Hinge, Tür → Door, Figur → Character, Geschichte → Story, Veröffentlichen → Publish, Stand sichern → Save a snapshot, Auf Original zurücksetzen → Reset to original, Hier spielen → Play here, Neues Spiel aus ROM… → New game from ROM…, Prüfen und Beheben → Check and fix, Begegnungen → Encounters, Attacken → Moves, Gegenstände → Items, Fähigkeit → Ability.

### Task 12: Bereiche `app`, Wurzeldateien, `platform`, `project`, `bus`

Ordner `app/` (41 Dateien) und Wurzel (`App.tsx`, `api*.ts`, `assets.ts`, `reload.ts`) → Namespace `app` (Wurzel-Texte nach `common`); `platform/` (7) → `platform`; `project/` (15) → `project`. Englisch-Test: `project/NewProjectWizard.i18n.test.tsx` findet „New game from ROM“ und „Template“.

### Task 13: Bereich `ui`

Ordner `ui/` ohne `ui/gallery` (95 Dateien minus 11) → Namespace `ui`. Die Galerie-Seiten bleiben deutsch (Spec 2); gemeinsame Komponenten in `ui/` werden übersetzt. Englisch-Test: `ui/frame/ToastHost.i18n.test.tsx` findet „Close“ und einen englischen Dock-Titel aus `dockMessages`.

### Task 14: Bereich `workbench`, Teil A (Welt-Explorer, Inspector, Dialog)

Ordner `workbench/` Unterordner `inspector/`, `dialog/`, `explorer/` oder gleichwertig nach `ls apps/studio/src/workbench` (der Implementierer teilt die 156 Dateien in zwei Hälften nach Unterordnern und nennt die Aufteilung im Commit) → Namespace `workbench`. Englisch-Test: der Inspector einer Tür findet „Door“ und „Target“.

### Task 15: Bereich `workbench`, Teil B (Script, Text, Rest)

Die zweite Hälfte von `workbench/` (u. a. `script/`). Englisch-Test: der Script-Editor-Rahmen findet „Adopt scripts“ und „Problems“.

### Task 16: Bereiche `world` und `data`

`world/` (55 Dateien) → `world`; `data/` (55) → `data`. Englisch-Test: `data/` Trainer-Editor findet „Trainer“ und „Party“.

### Task 17: Bereiche `stories`, `flags`, `delete`, `original`, `recipes`

→ Namespaces `stories`, `flags`, `delete`, `original`, `recipes`. Englisch-Test: `delete/` Dialog mit Referenzsperre findet „cannot be deleted“ und „references“.

### Task 18: Bereiche `technik`, `git`, `publish`, `textproject`

→ Namespaces `technik`, `git`, `publish`, `textproject`. Englisch-Test: `publish/` Assistent findet „Publish“ und „Version“.

### Task 19: Kernel-Katalog, Teil A (`textproject`, `world`, `original`, `recipes`)

**Files:**
- Modify: `crates/g4-kernel/src/textproject/**`, `world/**`, `original.rs`, `recipes/**` (alle deutschen Meldungen → `t!("<modul>.<schluessel>", …)`; `const … &str`-Meldungen werden `pub fn …() -> String` oder Aufrufe am Ort)
- Modify: `crates/g4-kernel/locales/{en,de}.yml`
- Modify: die Tests dieser Module (deutsche Zusicherungen → englisch)

**Interfaces:**
- Consumes: `t!` aus Task 8.

- [ ] **Step 1: Tests anpassen und einen Variablen-Test ergänzen**

In `crates/g4-kernel/tests/i18n.rs`: `world_message_fills_variables`: `set_locale("de")`, `t!("world.cell_out_of_range", x = 3, y = 7)` (ein realer Schlüssel dieser Module mit zwei Variablen; der Implementierer wählt ihn und nennt ihn im Test) enthält `3` und `7`, in `en` ebenso (Review Focus 5). Deutsche Zusicherungen in `tests/textproject.rs`, `tests/recipes_*.rs`, `tests/delete.rs` und in `#[cfg(test)]`-Modulen werden englisch.

- [ ] **Step 2: Laufen lassen, muss scheitern**

Run: `cargo test -p g4-kernel`
Expected: FAIL (englische Texte fehlen).

- [ ] **Step 3: Wandern**

Schlüssel `textproject.*`, `world.*`, `original.*`, `recipes.*`. `thiserror`-Varianten mit festem Text (`#[error("…")]`, 2 deutsche) bekommen `#[error("{0}")]` mit einem `String`-Feld, das am Entstehungsort per `t!` gefüllt wird.

- [ ] **Step 4: Laufen lassen und Grep**

Run: `cargo test -p g4-kernel && ! git grep -nE '"[^"]*[äöüÄÖÜß][^"]*"' -- 'crates/g4-kernel/src/textproject/**' 'crates/g4-kernel/src/world/**' crates/g4-kernel/src/original.rs 'crates/g4-kernel/src/recipes/**' | grep -v '#\[cfg(test)\]' | grep -v '^.*tests.rs'`
Expected: PASS; Grep ohne Treffer in Nicht-Test-Code (Treffer in `mod tests` sind erlaubt).

- [ ] **Step 5: Commit**

```bash
git add crates/g4-kernel
git commit -m "feat(i18n): kernel messages of textproject, world, original and recipes from catalogs"
```

### Task 20: Kernel-Katalog, Teil B (übrige Module) und `g4k`

Wie Task 19 für `stories`, `scripts`, `places`, `patch`, `dialog`, `store`, `flags`, `error`, `start`, `base`, `text`, `project`, `map_import`, `git`, `checks`, `data`, `delete`, `export`, `build`, `live`, `templates` und `src/bin/g4k.rs` (7 Literale). Grep-Zusicherung über `crates/g4-kernel/src` ohne `locales/`. Ein weiterer Variablen-Test für einen Schlüssel aus `git` oder `patch`. Commit `feat(i18n): remaining kernel messages and g4k from catalogs`.

### Task 21: Tauri-Katalog, Teil A (`commands`)

**Files:**
- Modify: `apps/studio/src-tauri/src/commands/**` (u. a. `dispatch.rs`, `data/labels.rs`, `data/display.rs`, `delete.rs`, `live.rs`, `world_map.rs`, `publish.rs`), `apps/studio/src-tauri/src/errors.rs`, `state.rs`, `base_roms.rs`
- Modify: `apps/studio/src-tauri/locales/{en,de}.yml`
- Modify: Tests (`commands/data/tests.rs`, `#[cfg(test)]`)

- [ ] **Step 1: Tests**

`apps/studio/src-tauri/tests/locales.rs`: `field_labels_translate`: `set_locale("de")`, Label für `egg_cycles` ist „Schlüpfzyklen“, in `en` „Egg cycles“ (über die öffentliche Funktion aus `labels.rs`, die der Implementierer nennt). Deutsche Zusicherungen in `data/tests.rs` werden englisch.

- [ ] **Step 2: FAIL**, **Step 3: Wandern** (Schlüssel `commands.*`, `labels.<feld>`, `errors.*`), **Step 4:** `cargo test -p studio` grün und Grep über `apps/studio/src-tauri/src/commands apps/studio/src-tauri/src/*.rs` ohne Treffer außerhalb von Tests, **Step 5: Commit** `feat(i18n): studio commands and field labels from catalogs`.

### Task 22: Tauri-Katalog, Teil B (`mcp`, `bridge`)

**Files:**
- Modify: `apps/studio/src-tauri/src/mcp/tools.rs`, `access.rs`, `prompts.rs`, `hub.rs`, `http.rs`; `bridge/roots.rs`, `bridge/http.rs`
- Rename: `apps/studio/src-tauri/src/mcp/glossary.md` → `glossary.de.md`; Create: `glossary.en.md`
- Modify: `apps/studio/src-tauri/src/mcp/docs.rs` (Glossar der aktuellen Locale: `fn glossary() -> &'static str` wählt nach `g4_kernel::i18n::locale()`)
- Modify: `apps/studio/src-tauri/tests/mcp_stdio.rs`

**Interfaces:**
- Der Werkzeugkatalog (`catalog.rs`) bleibt englisch und unverändert.

- [ ] **Step 1: Tests**

`mcp_stdio.rs`: `answers_in_german_with_g4_lang`: startet `studio mcp` mit `G4_LANG=de`, ruft ein Werkzeug, dessen Antwort einen Freigabetext trägt (z. B. die Lesefreigabe), erwartet „Nur lesen“; ohne Variable „Read only“. `glossary_follows_locale` in `tests/locales.rs`.

- [ ] **Step 2: FAIL**, **Step 3: Wandern** (Schlüssel `mcp.*`, `bridge.*`; `prompts.rs` liefert je Prompt den Text beider Sprachen aus dem Katalog), **Step 4:** `cargo test -p studio` grün, Grep über `apps/studio/src-tauri/src/mcp apps/studio/src-tauri/src/bridge` ohne Treffer außerhalb von Tests, **Step 5: Commit** `feat(i18n): MCP answers, grants, glossary and bridge messages from catalogs`.

### Task 23: Englische Docs und Changelog

**Files:**
- Modify: `CHANGELOG.md` (englisch; Abschnitt `## [1.0.0] - unreleased` mit den Punkten 4.5 Publish, 3d Tell, 6 Map creation and world format, English and German UI; `scripts/release/changelog-section.sh` muss die Sektion `1.0.0` finden)
- Create: `docs/en/mcp.md`, `docs/en/dev-setup.md`, `docs/en/release.md`
- Modify: `docs/mcp.md`, `docs/dev-setup.md`, `docs/release.md` (Zeile 1: `*English: [docs/en/<name>.md](en/<name>.md)*`), die drei englischen entsprechend zurück
- Modify: `docs/formats/*.md` (40 Dateien: nach der `#`-Überschrift ein Absatz `**In English:** …`, drei bis sechs Sätze)
- Modify: `docs/ROADMAP.md` (Abschnitt „Nach 1.0.0“: Landingpage `gen4studio.dev`, Linux-Build, Developer ID und Notarisierung, `docs/formats` auf Englisch, Git LFS für die `.fig`, Galerie und OpenPencil-Datei auf Englisch)
- Create: `scripts/tests/docs-en.test.sh`

- [ ] **Step 1: Test schreiben**

`docs-en.test.sh`: die drei `docs/en/*.md` existieren und haben jeweils mindestens 80 % der Zeilenzahl ihres deutschen Originals; jede Datei in `docs/formats/*.md` enthält `**In English:**` innerhalb ihrer ersten 12 Zeilen; `bash scripts/release/changelog-section.sh 1.0.0` liefert mindestens 4 Zeilen; `bash scripts/check-links.sh docs/en/*.md docs/formats/*.md` grün. Ausgabe `docs en: ok`.

- [ ] **Step 2: FAIL**, **Step 3: Schreiben** (Übersetzungen inhaltsgleich; Befehle und Pfade unverändert), **Step 4:** `bash scripts/tests/docs-en.test.sh` → `docs en: ok`, **Step 5: Commit** `docs(i18n): English changelog, user and agent docs, English lead in format docs, roadmap after 1.0.0`.

### Task 24: Gate Arbeitspaket 2

- [ ] **Step 1: Automatik**

Run: `cd apps/studio && pnpm test && pnpm build && cd ../.. && cargo test --workspace && G4_GATE_HERMETIC=1 bash scripts/gate.sh`
Expected: alles grün.

- [ ] **Step 2: Grep über alles**

Run:
```bash
git grep -nE '"[^"]*[äöüÄÖÜß][^"]*"|>[^<{]*[äöüÄÖÜß][^<{]*<' -- 'apps/studio/src/**/*.ts' 'apps/studio/src/**/*.tsx' ':!**/*.test.*' ':!apps/studio/src/i18n/**' ':!apps/studio/src/ui/gallery/**'
git grep -nE '"[^"]*[äöüÄÖÜß][^"]*"' -- 'crates/g4-kernel/src/**' 'apps/studio/src-tauri/src/**' | grep -vE 'tests?\.rs|/tests/'
```
Expected: keine Zeile außerhalb von `#[cfg(test)]`-Blöcken (Reste von Hand prüfen und wandern).

- [ ] **Step 3: Blick (Maintainer)**

`G4_LANG=en pnpm tauri dev` oder Systemsprache Englisch: Start, „New game from ROM…“, Welt bauen (Inspector, Werkzeugleiste, Weltbaum), Erzählen, Technik › Changes, Einstellungen (Sprachwahl wirkt sofort, auch auf einen Fehler-Toast, den ein Build ohne Basis-ROM auslöst), Palette. Dann Sprache „Deutsch“: dieselben Orte. `G4_LANG=de studio mcp` antwortet deutsch, ohne Variable englisch. Befunde als Tasks nachtragen.

- [ ] **Step 4: Screenshots in Englisch (Task 6 wiederholen) und Merge**

`superpowers:finishing-a-development-branch`, Merge `i18n` nach main, `bash scripts/gate.sh` auf main.

---

## Teil C: Arbeitspaket 3, auf `main`

### Task 25: Version 1.0.0 und rc-Reste

**Files:**
- Modify: `Cargo.toml`, `apps/studio/package.json`, `Cargo.lock` (über `version.sh`), `CHANGELOG.md` (Datum)

- [ ] **Step 1: Setzen**

Run: `bash scripts/version.sh set 1.0.0 && sed -n '1,20p' CHANGELOG.md`
Dann `## [1.0.0] - unreleased` → `## [1.0.0] - <heutiges Datum>` (kein `sed -i`; Editor).

- [ ] **Step 2: Prüfen**

Run: `bash scripts/version.sh check v1.0.0 && bash scripts/tests/version.test.sh`
Expected: Exit 0.

- [ ] **Step 3: Commit, rc-Reste löschen**

```bash
git add Cargo.toml Cargo.lock apps/studio/package.json CHANGELOG.md
git commit -m "release: 1.0.0"
gh release delete v0.1.0-rc.1 --repo Kaago/gen4-studio --yes --cleanup-tag
gh release delete v0.1.0-rc.2 --repo Kaago/gen4-studio --yes --cleanup-tag
git tag -d v0.1.0-rc.1 v0.1.0-rc.2
```

Run: `gh release list --repo Kaago/gen4-studio && git tag`
Expected: leer.

### Task 26: Launch-Texte

**Files:**
- Create: `docs/launch/announcement.md`, `discord.md`, `reddit.md`, `show-hn.md`, `wiki-pr.md`, `awesome-lists.md`, `README.md` (Reihenfolge und Checkliste des Veröffentlichens)

- [ ] **Step 1: Schreiben**

Englisch, nach Spec 5 (5). Jeder Text beginnt mit dem, was Gen4 Studio ist, nennt macOS und Windows, das Textprojekt mit Git, MCP, die DSPRE-Kompatibilität und dankt dem DSPRE-Team; Links: Release-Seite, README, `docs/en/mcp.md`. `discord.md` höchstens 1.200 Zeichen. `show-hn.md`: Titel „Show HN: Gen4 Studio – a text-based, Git-friendly editor for Pokémon DS ROM hacks with an MCP interface“ und ein Kommentar des Autors (Motivation, Architektur, was fehlt). `wiki-pr.md`: die Markdown-Zeilen für `docs/generation-iv/guides/mapping` und „Getting Started“ des Wikis `ds-pokemon-hacking/ds-pokemon-hacking.github.io`. `awesome-lists.md`: je eine Zeile im Format der Liste.

- [ ] **Step 2: Prüfen und Commit**

Run: `bash scripts/check-links.sh docs/launch/*.md`
Expected: `links: <n> ok`.

```bash
git add docs/launch
git commit -m "docs(launch): announcement texts for Discord, forums, Reddit, HN, wiki and awesome lists"
```

### Task 27: Umschalten auf public (Hand, Maintainer, mit Bestätigung)

- [ ] **Step 1: Letzte Prüfung vor dem Umschalten**

Run: `bash scripts/check-no-rom-data.sh --history $(git rev-list --max-parents=0 HEAD)..HEAD && git status --porcelain && git log origin/main..main --oneline | wc -l`
Expected: `no rom data: … ok`, sauberer Baum, danach `git push origin main`, so dass `origin/main == main`.

- [ ] **Step 2: Umschalten und Einstellungen**

```bash
gh repo edit Kaago/gen4-studio --visibility public --accept-visibility-change-consequences
gh api -X PUT repos/Kaago/gen4-studio/vulnerability-alerts
gh api -X PUT repos/Kaago/gen4-studio/private-vulnerability-reporting
gh api -X PUT repos/Kaago/gen4-studio/branches/main/protection --input - <<'JSON'
{
  "required_status_checks": { "strict": true, "contexts": ["gate-macos"] },
  "enforce_admins": false,
  "required_pull_request_reviews": { "required_approving_review_count": 0 },
  "restrictions": null,
  "allow_force_pushes": false,
  "allow_deletions": false
}
JSON
```

Run: `gh repo view Kaago/gen4-studio --json visibility,hasDiscussionsEnabled --jq .`
Expected: `PUBLIC`, `true`.

### Task 28: Release v1.0.0 (Hand, Maintainer)

- [ ] **Step 1: Probelauf**

Run: `bash scripts/release-local.sh v1.0.0 --dry-run`
Expected: `release-local: Probelauf ok (…)`.

- [ ] **Step 2: Release**

Run: `bash scripts/release-local.sh v1.0.0`
Expected: `release-local: Entwurf v1.0.0 angelegt (…)`.

- [ ] **Step 3: Windows-Installer**

Run: `gh workflow run release.yml -f tag=v1.0.0 -f dry_run=false && gh run watch`
Expected: Lauf grün; `gh release view v1.0.0 --json assets --jq '.assets[].name'` nennt DMG, `.exe` und `SHA256SUMS.txt`; `gh release download v1.0.0 -p SHA256SUMS.txt -O - | wc -l` ist 2 (DMG und Installer; die Zeile des Installers hängt `add-checksum.sh` an).

- [ ] **Step 4: Veröffentlichen**

Run: `gh release edit v1.0.0 --draft=false --latest && gh release view v1.0.0 --json isDraft --jq .isDraft`
Expected: `false`.

### Task 29: Launch (Hand, Maintainer)

- [ ] **Step 1:** In der Reihenfolge aus `docs/launch/README.md` veröffentlichen: KoDSH-Discord, PR an das Wiki, PokéCommunity, Project Pokémon (Datei-Eintrag mit Download-Link auf GitHub), r/PokemonROMhacks, Show HN, Awesome-Listen (PRs).
- [ ] **Step 2:** Erste Woche: Issues und Discussions täglich sichten, Befunde als Patch-Release 1.0.1 sammeln; `docs/ROADMAP.md` „Nach 1.0.0“ mit den Rückmeldungen ergänzen.
