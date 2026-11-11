# Release 1.0.1: Auto-Update und Nacharbeiten

Stand: 2026-11-11. Grundlage: Gespräch mit dem Maintainer am 2026-11-11, Triage des Reviews von 1.0.0
(`lab/editor/gate-artefakte/2026-11-11-oss-release-v1-sdd/final-review-notes.md`, Ledger `progress.md` dort).

## 1. Ziel

1.0.1 bringt einen Auto-Updater, der neue Versionen aus GitHub Releases holt, und schließt die für 1.0.1
gesammelten Nacharbeiten aus 1.0.0. Developer-ID, Notarisierung und eine Windows-Signatur gehören nicht dazu:
Die Pakete bleiben wie in 1.0.0 ad-hoc signiert (macOS) und unsigniert (Windows).

## 2. Entscheidungen

| Frage | Entscheidung | Quelle |
|---|---|---|
| Hinweis oder echter Updater | Echter Updater (Tauri-Updater-Plugin): prüfen, laden, installieren, neu starten | Maintainer |
| Developer-ID | Nicht in 1.0.1 (kein Apple-Konto) | Maintainer |
| Wann die App GitHub fragt | Variante B: Beim ersten Start einer Version mit Updater fragt die App einmal. Ja: Prüfung beim Start, höchstens einmal in 24 Stunden. Nein: nur von Hand. In den Einstellungen jederzeit umschaltbar | Maintainer |
| Was bei einer neuen Version passiert | Variante A: Hinweis mit „Installieren…“ und „Diese Version überspringen“; Schließen heißt „später“. „Installieren…“ zeigt Version und Änderungen, warnt vor ungespeicherten Änderungen, lädt mit Fortschritt, installiert und startet neu | Empfehlung, vom Maintainer nicht beantwortet; gesetzt, im Review des Plans änderbar |

## 3. Auto-Update

### 3.1 Quelle und Format

- Endpunkt: `https://github.com/Kaago/gen4-studio/releases/latest/download/latest.json`. GitHub leitet `latest` auf den
  neuesten veröffentlichten Release, der kein Pre-Release ist. Entwürfe und `-rc`-Fassungen sieht die App also nie.
- `latest.json` im statischen Format des Tauri-Updaters: `version` (SemVer ohne `v`), `notes` (Text des Releases),
  `pub_date` (RFC 3339, UTC) und `platforms` mit genau zwei Einträgen, `darwin-aarch64` und `windows-x86_64`, je mit
  `url` (Asset des Releases) und `signature` (Inhalt der `.sig`-Datei, nicht ihr Pfad).
- Assets eines Releases ab 1.0.1: `Gen4-Studio_<v>_aarch64.dmg`, `Gen4-Studio_<v>_aarch64.app.tar.gz` und `.sig`,
  `Gen4-Studio_<v>_x64-setup.exe` und `.sig`, `SHA256SUMS.txt`, `latest.json`. DMG und Installer bleiben die Downloads
  für Menschen; der Installer ist zugleich das Update-Paket für Windows. `SHA256SUMS.txt` nennt weiter nur DMG und Installer.
- `G4_UPDATE_ENDPOINT` (eine `https://`-Adresse) ersetzt den Endpunkt, für die Abnahme mit rc-Releases. Die Prüfung der
  Signatur bleibt dieselbe. Eine Adresse, die nicht mit `https://` beginnt, lehnt die App ab.

### 3.2 Signatur und Schlüssel

- Ein eigenes minisign-Schlüsselpaar aus `tauri signer generate`, mit Passwort. Der öffentliche Schlüssel steht in
  `apps/studio/src-tauri/tauri.conf.json` (`plugins.updater.pubkey`); die App installiert nur Pakete mit gültiger Signatur.
- Der private Schlüssel liegt auf dem Mac des Maintainers unter `~/.tauri/gen4-studio-updater.key`, das Passwort im
  Schlüsselbund (Dienst `gen4-studio-updater`, Konto `$USER`). Auf GitHub als Secrets `TAURI_SIGNING_PRIVATE_KEY` und
  `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`, nur im Bundle-Schritt von `release.yml` gelesen. Kein Workflow, den ein Pull
  Request startet, sieht sie.
- Der Maintainer sichert Schlüssel und Passwort zusätzlich (Passwort-Manager). Geht der Schlüssel verloren, erhalten
  installierte Versionen keine Updates mehr; ein neuer Schlüssel heißt: einmal von Hand neu installieren.
- `bundle.createUpdaterArtifacts` steht nur in `apps/studio/src-tauri/tauri.updater.conf.json`, die die Release-Pfade mit
  `--config` übergeben. Entwicklung, Gate, CI und Beitragende bauen ohne Schlüssel.

### 3.3 App, Rust

- `tauri-plugin-updater` 2 ist registriert; unter Windows `installMode: "passive"` (kleines Fortschrittsfenster, keine Klicks).
- Befehl `update_check` → `UpdateInfo | null` mit `version`, `current` und `notes`; Zeitlimit 30 Sekunden. Das gefundene
  Update bleibt im Zustand der App bis zur Installation oder zur nächsten Prüfung.
- Befehl `update_install { version, discardUnsaved }`: lehnt ab, wenn kein Update gefunden ist oder ein anderes als
  `version` (Code `"error"`, Text aus dem Katalog), und wenn der Store ungespeicherte Änderungen hat und `discardUnsaved`
  falsch ist (Code `"unsaved"`). Sonst lädt er, meldet `update-progress` (Laden mit Bytes, dann Installieren) und
  installiert. Danach unter macOS: `force_quit` setzen, App neu starten. Unter Windows beendet das Plugin die App selbst,
  der Installer installiert und startet die neue Fassung.
- Im Browser-Modus (Dev-Bridge) antwortet `update_check` mit `null`, `update_install` tut nichts. Für Agenten (MCP) sind
  beide Befehle `APP_ONLY`.
- Menüpunkt „Check for Updates…“ (Katalog `menu.check_updates`): unter macOS im App-Menü nach „About“, unter Windows im
  Menü „Help“. Er sendet das Ereignis `menu-check-updates` an die Oberfläche.
- Sonst kein neuer Netzzugriff. Die CSP bleibt, denn die Anfragen stellt Rust.

### 3.4 App, Oberfläche

- Nur in der gepackten App: `inTauri() && !import.meta.env.DEV`. Im Dev-Build, im Browser-Modus und in den Tests der
  übrigen Bereiche ist alles aus.
- Einstellungen in `localStorage`, Schlüssel `g4.update.v1`: `consent` (`"ask"`, `"auto"`, `"off"`), `lastCheck`
  (ISO-Zeit oder `null`), `skipped` (Fassung oder `null`). Fehlt der Eintrag, ist er kaputt oder der Speicher gesperrt,
  gelten die Vorgaben (`"ask"`, `null`, `null`); ein gescheitertes Schreiben stört nichts.
- Beim Start mit `consent: "ask"`: einmal die Frage (natives Dialogfenster), Ja ergibt `"auto"`, Nein `"off"`.
- Mit `"auto"` prüft die App beim Start still, wenn die letzte erfolgreiche Prüfung 24 Stunden oder länger zurückliegt
  (oder in der Zukunft liegt, nach einer Uhrumstellung). `lastCheck` wird nur nach einer erfolgreichen Prüfung gesetzt.
  Fehler der automatischen Prüfung bleiben still; Fehler einer Prüfung von Hand zeigt ein Fehler-Toast.
- Neue Fassung gefunden: ein bleibender Hinweis (Toast, Schlüssel `update`) „Gen4 Studio {{version}} is available.“ mit
  „Install…“ und „Skip this version“. Eine übersprungene Fassung meldet die automatische Prüfung nicht mehr; die Prüfung
  von Hand meldet sie trotzdem.
- „Install…“: natives Dialogfenster mit Fassung und Änderungen (höchstens 1500 Zeichen, sonst gekürzt mit „…“), bei
  ungespeicherten Änderungen mit dem Satz, dass sie verloren gehen, und den Knöpfen „Install and Restart“ und „Cancel“.
  Danach ein Fortschritts-Toast (Schlüssel `update`): Laden mit Prozent (ohne bekannte Größe mit Bytes), dann
  „Installing…“. Ein Fehler zeigt ein Fehler-Toast mit dem Grund; die App läuft weiter.
- Prüfung von Hand ohne neue Fassung: „Gen4 Studio {{version}} is up to date.“
- Einstellungen: eine Spalte „Updates“ mit der Fassung, dem Schalter „Check for updates automatically“ (an heißt `"auto"`,
  aus heißt `"off"`), einem Hinweis, was gefragt wird, dem Knopf „Check now“ und „Last checked …“ (oder „Not checked yet“).
- Texte in beiden Sprachen (Namensraum `app`, Rust-Katalog `commands.update.*` und `menu.*`).

### 3.5 Release-Pfad

- `scripts/release-local.sh`: Ein echter Release braucht den Schlüssel (`TAURI_SIGNING_PRIVATE_KEY` oder die Datei aus 3.2)
  und das Passwort (`TAURI_SIGNING_PRIVATE_KEY_PASSWORD` oder der Schlüsselbund); fehlt eines, bricht er vor dem Bau ab.
  Ein Probelauf signiert, wenn der Schlüssel da ist, sonst baut er ohne Update-Paket und sagt das. Gebaut wird mit
  `--config src-tauri/tauri.updater.conf.json`; `.app.tar.gz` und `.sig` kommen unter den Asset-Namen nach `dist-release/`
  und mit DMG und `SHA256SUMS.txt` in den Entwurf.
- `release.yml`: Der Bundle-Schritt bekommt die Secrets und dieselbe Konfiguration und sammelt `*-setup.exe.sig` ein.
  `publish-windows` lädt Installer und `.sig` hoch, holt die `.sig` des macOS-Pakets aus dem Entwurf, schreibt `latest.json`
  mit `scripts/release/updater-manifest.sh` und lädt sie hoch. Fehlt die macOS-`.sig`, scheitert der Job.
- `scripts/release/updater-manifest.sh` prüft, was es schreibt: Fassung gültig und gleich dem Tag ohne `v`, beide
  Signaturen nicht leer, Base64 und dekodiert mit `untrusted comment:` am Anfang; die Adressen zeigen auf die Assets des Tags.

### 3.6 Datenschutz und Doku

- README (en, de): Abschnitt „Updates“: was gefragt wird, wann, was GitHub dabei sieht (eine normale HTTPS-Anfrage wie
  bei jedem Download, also die IP-Adresse), dass nichts über Projekte gesendet wird, wie man es abschaltet, und dass
  Nutzer von 1.0.0 die 1.0.1 einmal von Hand installieren.
- `docs/release.md` und `docs/en/release.md`: Schlüssel, Secrets, Update-Pakete, `latest.json`, Abnahme mit rc-Releases,
  was bei Verlust des Schlüssels geschieht. `docs/dev-setup.md` und `docs/en/dev-setup.md`: „zur Laufzeit kein Netz“
  bekommt die Ausnahme der Update-Prüfung. `SECURITY.md`: Updates sind signiert; ein Verdacht auf einen kompromittierten
  Schlüssel ist eine Sicherheitsmeldung. `CHANGELOG.md`: Abschnitt 1.0.1.

### 3.7 Abnahme

- Zwei Pre-Releases `v1.0.1-rc.1` und `v1.0.1-rc.2` durch den ganzen Release-Pfad.
- Mac: rc.1 aus dem DMG installieren, mit `G4_UPDATE_ENDPOINT=https://github.com/Kaago/gen4-studio/releases/download/v1.0.1-rc.2/latest.json`
  starten, „Check for Updates…“, mit einer ungespeicherten Änderung installieren (Warnung erscheint), die App startet als
  rc.2 neu, ohne Rückfrage von Gatekeeper. Zusätzlich `latest-badsig.json` am rc.2-Release (Signatur von rc.1): Die
  Installation scheitert mit Fehler-Toast, die App läuft weiter.
- Windows: dasselbe, wenn ein Windows-Rechner da ist. Sonst bleibt es offen, und das erste echte Update unter Windows
  (1.0.1 auf 1.0.2) ist die Probe; das steht im Ledger und in `docs/release.md`.
- Danach rc-Releases und rc-Tags löschen, 1.0.1 veröffentlichen.

## 4. Nacharbeiten aus 1.0.0

Quelle: Triage in `final-review-notes.md` (Pass 7) und die Rulings im Ledger von 1.0.0; Fundstellen am 2026-11-11 auf
`46c68403` geprüft. Was der Fix-Durchgang vor 1.0.0 schon erledigt hat (Basis-ROM-Namen, CONTRIBUTING, Legal-Satz,
Alt-Text, `NewProjectWizard`, `LANG` in `docs/en/mcp.md`, Parität der Platzhalter, Kernel-Zwilling von `catalog_keys.rs`),
fehlt hier.

### 4.1 Sprachwechsel

- `applyLanguage` sagt dem Backend die Sprache zuerst (`set_locale`) und schaltet erst danach i18next um. `set_locale` hat
  ein Zeitlimit von 3000 ms; Fehler und Zeitüberschreitung schreiben eine Warnung in die Konsole, der Wechsel geht weiter.
- Nach einem Wechsel lädt die App die Texte, die das Backend gebaut hat und die oberhalb der Werkbank liegen, neu:
  Übersicht (`useOverview`), `DataStore`, Prüfungen (`useChecks`), Probleme, Projektinfo und Git-Status. Jeder über den
  Weg, den er bei einem Bus-Ereignis schon nimmt.
- Im Browser-Modus mit Dev-Bridge (`import.meta.env.MODE === "development"`) geht `set_locale` auch an die Bridge.
- Falsche Kommentare werden berichtigt: WKWebView und `navigator.language` (`Info.plist`, `Cargo.toml`, `i18n/index.ts`,
  `api.base.ts`, `commands/locale.rs`), „same process“ in `locale.rs`, „German sentence“ in `api.base.ts`, `api.ts` und
  `project/api.start.ts`, „initialised“ in `main.tsx` und `i18n/boot.ts`.

### 4.2 Codes statt Textvergleich

- Kernel: neue Variante `KernelError::Coded { code: &'static str, message: String }` (Anzeige gleich `message`), Codes als
  Konstanten in `g4_kernel::error::codes`. Die Fehler, die das Studio heute am Wortlaut erkennt, entstehen als `Coded`:
  `flags.rs` (`name_error`, auch `is_taken`), `stories.rs` (`story_error`), `textsource.rs` (`text_error`),
  `textproject.rs` (`export_error`), `original.rs` (`reset_error`), `git.rs` (`diff_error`, `commit_error`) und
  `publish.rs` (`publish_error`). Der Code des Kernels ist genau der `CmdError.code`, den das Studio heute wählt; die
  Codes an Oberfläche und MCP ändern sich nicht.
- `GitInitReport` bekommt `reason_code` (`"in_repo"`, `"not_installed"`, `"failed"`); `g4k git init` und `gitWords.ts`
  entscheiden danach.
- `mcp/access.rs` fragt für `recipe_apply`, ob das Rezept eine Level-Script-Datei anlegt (Kernel `recipes::creates_level_script`), statt den Text zu vergleichen.
- Der Hinweis „neu öffnen“ beim Übernehmen als Textprojekt bekommt den Code `"reopen"` (bisher `"error"` plus Textvergleich in `AdoptDialog.tsx`).
- `textProjectHint.ts` vergleicht mit `matchesAnyLanguage` statt mit einer Sprache. Die übrigen `matchesAnyLanguage`-Stellen
  bleiben: Sie erkennen beide Sprachen.

### 4.3 Englisch in der deutschen Oberfläche

- Kernel: die drei Absagen zu Diamant/Perl (`map_import.rs`, `bake/textures.rs`, `bake/test_surface.rs`) und die englischen
  Sätze daneben (Block außerhalb der Karte, fehlendes Textur-Paket, kein Teil gewählt, nicht in der Quelle, Startstadt
  fehlt, kein Ordnername; `g4k`: Welt schon leer) kommen in beide Kernel-Kataloge.
- Studio: „no project open“ (`commands/mod.rs`, `mcp/tools.rs`) und die englischen Fehler in `events.rs`, `buildings.rs`,
  `world.rs`, `data/edit.rs` und `trainer.rs` kommen in beide Studio-Kataloge.
- Undo-Labels bleiben englische Entwicklerwörter; `app/historyLabel.ts` übersetzt sie. Ein Test sichert zu, dass jedes Label
  der Befehle eine deutsche Übersetzung hat.

### 4.4 Kleinkram

- Paletten-Gruppen mit ASCII-Ids (`commands`, `places`, `headers`, `textArchives`, `pokemon`, `trainers`, `encounters`,
  `moves`, `items`, `scripts`); die Überschriften kommen weiter aus dem Katalog.
- Der vorbefüllte Belohnungstext folgt der Sprache der ROM (`ProjectInfo.language`: `"German"` gibt Deutsch, alles
  andere Englisch), nicht der Sprache der Oberfläche.
- Uhrzeiten bei Begegnungen: eine Formatierung für alle Stellen, Englisch 12 Stunden, Deutsch 24 Stunden.
- `FlagField.tsx` nimmt `inSentence` für das Label im Satz.
- Doppelte Werte im Studio-Katalog: `bridge.*`, `mcp.*` und `base_roms.*` rufen die Schlüssel unter `commands.policy.*` und
  `commands.common.*`; die Doppel fallen weg.
- `crates/g4-kernel/tests/cli.rs`: Die leere Zusicherung mit „rückgängig“ prüft „cannot be undone“.
- Ein Test-Helfer für Sätze aus den Kernel-Katalogen statt dreier gleicher Regex-Paare.
- Der Lexer aus dem Kernel-Test `catalog_keys.rs` wird geteilt; der Studio-Test nutzt ihn.
- `docs/formats/xdelta_patch.md`: Die Begleittexte stehen in der Sprache des Prozesses, nicht fest auf Deutsch.

## 5. Nicht in 1.0.1

Developer-ID und Notarisierung, Windows-Signatur, Linux, Landingpage, die Major-Updates der Dependabot-PRs,
`docs/formats` auf Englisch, Git LFS für die `.fig`.

## 6. Risiken

- macOS könnte beim Ersetzen einer ad-hoc signierten App nachfragen (Schutz „App-Verwaltung“). Die Abnahme in 3.7 zeigt
  es; wenn ja, beschreiben README und `docs/release.md` die Rückfrage.
- Ohne Windows-Rechner ist der Update-Weg unter Windows erst mit 1.0.2 im Feld geprüft.
- Verlust des privaten Schlüssels (3.2).
- `latest` zeigt sofort nach dem Veröffentlichen auf den neuen Release: Vorher müssen alle sieben Assets im Entwurf liegen.
  Die Checkliste in `docs/release.md` zählt sie.
