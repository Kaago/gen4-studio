# Projekt in Git: `.gitignore` und `.gitattributes`

Eine Tatsache über Gen4 Studio selbst, keine Spielformat-Quelle: Ein Projekt (DSPRE-Ordner oder
Textprojekt) lässt sich als lokales Git-Repository führen. Der Kernel schreibt dafür eine
`.gitignore` und eine `.gitattributes` in die Projektwurzel, aus Vorlagen je Layout. Die Vorlagen
liegen als Metadaten im Kernel (`crates/g4-kernel/metadata/git/dspre.gitignore`, `text.gitignore`,
`gitattributes`, per `include_str!`). Implementierung: `crates/g4-kernel/src/git/templates.rs`
(`layout_of`, `gitignore_block`, `gitattributes_block`, `merge_block`, `write_templates`,
`DSPRE_PART`). Kopflos: `g4k git templates <root>` druckt je Datei `<Aktion> <Datei>` (`created`,
`appended`, `updated`, `unchanged`), dann `layout=dspre|text_project ignored_archives=N`.

Begründung und Messungen: [Gutachten C](../research/2026-10-27-phase-4/gutachten-c-git-einordnung.md)
§2.1 (was im Ordner liegt und was davon erzeugt ist), §2.3 (Zeilenenden), §5 (Vorlagen).

## Vorlagen

Welche `.gitignore` ein Projekt bekommt, entscheidet `layout_of` aus `Project::source_mode()`:
ein Textprojekt jeder Schema-Version (`TextProject`, `Older`, `Newer`) bekommt die Textvorlage,
ein DSPRE-Ordner und ein Manifest, das sich nicht lesen lässt (`Broken`), die DSPRE-Vorlage. Wird ein
DSPRE-Ordner zum Textprojekt („Als Textprojekt einrichten…“), ersetzt das nächste Schreiben den
DSPRE-Block durch den Textblock (`updated`): ein umgestelltes Projekt versioniert seine Spieldateien
nicht weiter. Dieses Schreiben folgt von selbst: Steht der Block von Gen4 Studio schon in der
`.gitignore`, schreibt `textproject::adopt::adopt` nach dem Einrichten und `revert` nach dem
Zurücknehmen beide Vorlagen neu (Bericht `templates`, ein Fehler dabei nur als `templates_warning`,
das Einrichten gilt trotzdem); ein Projekt ohne den Block bekommt keine Vorlagen. Was schon
committet ist, bleibt verfolgt, bis es mit `git rm -r --cached <Pfad>` aus dem Index genommen ist;
das tut Gen4 Studio nicht.

### DSPRE-Layout (`dspre.gitignore`)

| Gruppe | Zeilen | Grund |
|---|---|---|
| Kopf | zwei Kommentarzeilen | Das Projekt enthält Daten aus dem Spiel: nur lokal oder privat versionieren; geteilt wird der Patch gegen die Basis-ROM (Gutachten C §4) |
| Gebaute ROMs und Patches | `*.nds`, `*.srl`, `*.xdelta`, `/build/` | Ausgaben des Builds und des Veröffentlichens; `/build/` ist das Build-Ziel der Agenten (C §2.1) |
| Emulator | `*.sav`, `*.dsv`, `*.ml[0-9]`, `*.mln`, `*.state` | Der Emulator legt Spielstände neben die ROM; liegt sie im Projekt, landen sie dort (C §2.1) |
| Studio | `/.g4/*`, `!/.g4/base.json`, `!/.g4/preview.png`, `!/.g4/releases.json` | Erlaubnisliste, Abschnitt [`.g4/`](#g4) |
| rotom | `/.rotom/cache/`, `/.rotom/command_database/.g4-ids-*` | Cache von rotom und die ID-Dateien für rotom, die das Studio aus den Registern neu schreibt (`textproject/include.rs`); die übrige `.rotom/command_database` und die Quirks in `.rotom/status/compile-state.json` bleiben versioniert |
| Sicherungen | `*.bak`, `*.orig`, `*~` | Sicherungen gehören nicht in den Projektordner |
| Archive, die der Build packt | `/<NARC-Pfad>` je Zeile, sortiert | siehe unten |
| Systemdateien | `.DS_Store`, `Thumbs.db` | |

**Archive, die der Build packt** (`{{packed_archives}}`): je Eintrag aus
`build::pack_entries` (die Arten aus `archives.yaml` und die `pack_only`-Ordner der Familie) eine Zeile
`/<gepackter Pfad>` mit `/`, aber nur, wenn `unpacked/<Ordner>/` existiert. Seit 4.2 packt der
Build jeden vorhandenen `unpacked/`-Ordner, also sind genau diese NARCs erzeugt; ohne die Zeilen
zeigte jeder Build geänderte NARCs, die niemand prüfen kann (C §2.2, Variante B). Ein Archiv ohne
`unpacked/`-Ordner ist Quelle und bleibt versioniert, auch die Header-Ablage des Dynamic-Headers-Patches
(`unpacked/dynamicHeaders/` entsteht erst mit dem Patch oder durch DSPRE). „Git einrichten…“ packt
vorher jede Art aus, die das Studio kennt, damit alle Zeilen gelten. Die Zahl der Zeilen steht im
Bericht als `ignored_archives` (die vanilla DSPRE-Ordner: 36 in Platin, 40 in HeartGold).

Versioniert bleiben: `unpacked/`, `expanded/`, `arm9/`, die Overlays, die übrigen `files/` (auch die
NARCs ohne `unpacked/`-Ordner), `header.yaml`, `config.yaml`, `g4-world.json`, `g4-checks.json`,
`rotom.toml`, `.rotom/command_database` (ohne die ID-Dateien `.g4-ids-*`),
`.rotom/status/compile-state.json`, `.g4/base.json`, `.g4/preview.png`, `.g4/releases.json`, die
`.gitignore` und die `.gitattributes`.

### Textprojekt (`text.gitignore`)

Dieselben ersten drei Gruppen (Kopf mit „Textprojekt“, gebaute ROMs, Emulator, Studio), dann:

| Gruppe | Zeilen | Grund |
|---|---|---|
| Spieldateien | `/<Name>/` je Ordner und `/<Name>` je Datei aus `DSPRE_PART`, dann `/unpacked/` | Aus der Basis-ROM und dem Textprojekt erzeugt; nach einem Klon entstehen sie mit „Textprojekt vervollständigen“ aus der eigenen ROM |
| rotom | `/.rotom/*`, `!/.rotom/command_database`, `/.rotom/command_database/.g4-ids-*` | Nur die Befehlsdatenbank ist Projekt; die Quirks stehen in `data/rotom-quirks.json`, `compile-state.json` ist hier Cache; die ID-Dateien für rotom sind erzeugt (das Studio schreibt sie beim Öffnen, nach einer Taufe und vor dem Kompilieren neu) |
| Sicherungen | `*.bak`, `*.orig`, `*~`, `/*.prev`, `/*.prev/` | Hinweis H2, unten |
| Systemdateien | `.DS_Store`, `Thumbs.db` | |

`DSPRE_PART` ist die feste Liste der Namen, die `build::extract_rom` (ds-rom `Rom::save` mit der
Konfiguration aus `Rom::extract`) in der Wurzel anlegen kann: `arm7/`, `arm7_overlays/`, `arm9/`,
`arm9_overlays/`, `banner/`, `files/`, `config.yaml`, `header.yaml`, `header_logo.png`,
`multiboot_signature.yaml`, `path_order.txt` (die Overlay-Ordner und die Signatur nur, wenn die ROM
sie hat). Ein Test mit Freigabe (`G4_TEST_PROJECT`) entpackt die Basis-ROM und prüft jeden Namen der
Wurzel gegen die Liste. `expanded/` steht nicht darin: es bleibt versioniert.

Versioniert bleiben: die Dateien aus `textproject::sources()` (`g4-project.json`, `data/`,
`expanded/textArchives/*.json`, `expanded/scripts/`, `rotom.toml`, `.rotom/command_database/`,
`g4-world.json`, `g4-checks.json`, `.g4/base.json`, `.g4/preview.png`), außerdem
`.g4/releases.json`, der Rest von `expanded/`, die `.gitignore` und die `.gitattributes`. Die
ID-Dateien für rotom (`.rotom/command_database/.g4-ids-*.json`) zählt `sources()` nicht mit, und die
Vorlage ignoriert sie.

**Hinweis H2** (Abnahme 4.3a): Der Export nimmt jede Datei der Wurzel, die keine Spieldatei ist, als
Blob unter `data/raw/` auf. Eine Sicherung neben `data/` (`data.prev/`, `g4-project.json.prev`)
wird so zu Tausenden Blobs. Die Vorlage kann den Export nicht ändern, hält aber die üblichen Muster
aus Git heraus; Sicherungen gehören außerhalb des Projektordners.

### `.gitattributes` (`gitattributes`)

Gleich für beide Layouts:

```gitattributes
* -text
*.json text eol=lf
*.rotom text eol=lf
*.yaml text eol=lf
*.yml text eol=lf
*.toml text eol=lf
*.md text eol=lf
# Beifiles der Phase 6 (Modelle, Bilder): ohne Diff und Merge; LFS erst ab 1 MB je Datei, etwa: *.glb filter=lfs diff=lfs merge=lfs -text
*.png binary
*.glb binary
```

Git hält 81 bis 99 Spieldaten-Dateien für Text (`pokeHeight`, `files/data`, C §2.3). Mit
`core.autocrlf=true` (Standard von Git für Windows) schriebe Git sie beim Auschecken um, und die ROM
änderte sich, ohne dass jemand etwas bearbeitet hat. `* -text` verbietet jede Umwandlung; die
Textquellen bekommen feste LF-Zeilenenden. Seit 6.5b sind die Beifiles der Phase 6 (Modelle
`*.glb`, etwa die Freiform- und Teilmodell-Beifiles aus [world_map.md](world_map.md), und Bilder
`*.png`) `binary`: Git zeigt keinen Text-Diff und mergt sie nicht zeilenweise. Git LFS gibt es
nicht; der Kommentar nennt die Zeile, mit der ein Projekt große Modelle (ab 1 MB je Datei) selbst
in LFS legen kann. Die Repo-`.gitattributes` von gen4-studio hat dieselben zwei Zeilen. Tests
`attributes_mark_everything_binary_and_text_sources_lf` und
`the_project_attributes_mark_png_and_glb_binary` in `git/templates.rs`.

## Block

Der ganze Inhalt einer Vorlage steht in einem Block zwischen zwei Zeilen:

```
# >>> Gen4 Studio (dieser Block wird von Gen4 Studio gepflegt) >>>
…
# <<< Gen4 Studio <<<
```

`write_templates` liest beide Dateien, setzt den Block ein (`merge_block`) und schreibt erst dann,
jede Datei atomar (`fsutil::write_atomic`); eine unveränderte Datei wird nicht geschrieben:

| Vorher | Nachher | Aktion |
|---|---|---|
| keine Datei | der Block allein | `created` |
| Datei ohne Block | die Zeilen des Nutzers byte-gleich, eine Leerzeile (keine bei einer leeren Datei oder einer, die schon mit einer Leerzeile endet), der Block am Ende | `appended` |
| Datei mit genau einem Block, gleich und am Ende (danach höchstens Leerraum) | unverändert (auch, wenn er sich nur in `\r\n`-Zeilenenden unterscheidet) | `unchanged` |
| Datei mit anderem Block, mit gleichem, hinter dem noch Zeilen stehen, oder mit mehreren Blöcken (doppelt eingefügt) | jeder Block herausgenommen (die Zeilen davor, dazwischen und danach bleiben byte-gleich und in ihrer Reihenfolge), dann der Block am Ende wie bei einer Datei ohne Block | `updated` |
| Anfangszeile ohne Endzeile danach, oder eine zweite Anfangszeile vor der Endzeile | Ablehnung `<Datei>: Der Block von Gen4 Studio ist unvollständig (Anfang ohne Ende); bitte von Hand reparieren.`, keine der beiden Dateien geschrieben | Fehler |

Der Block steht immer am Ende: In beiden Dateien gewinnt die spätere Zeile, so gelten
`* -text` und die Ignorier-Regeln auch hinter einem `* text=auto` oder einer eigenen Regel des
Nutzers. Eine Datei, die kein UTF-8 ist, wird abgelehnt (`<Datei>: Die Datei ist kein UTF-8-Text;
bitte von Hand prüfen.`).

## Git einrichten

`git::cli::git_init` (`crates/g4-kernel/src/git/cli.rs`) richtet ein Projekt als lokales
Git-Repository ein. Orte: Häkchen „Git einrichten (nur lokal)“ im Assistenten „Neues Spiel aus ROM“
(`CreateOptions.git`, Phase `git` nach `text_project`, im Staging-Ordner vor dem Umbenennen),
`g4k git init`; App und MCP folgen in eigenen Schritten. Phasen (`InitPhase`, je beim Start gemeldet,
die Millisekunden im Bericht unter `phase_ms`):

| Phase | Was | Scheitert |
|---|---|---|
| `unpack` | nur im DSPRE-Layout: `Project::ensure_unpacked` je Art aus `ArchiveMap::kinds()`, damit der Block jede Archivzeile hat (ohne die Header-Ablage des Dynamic-Headers-Patches und ohne Art, deren Archiv und Ordner fehlen) | `Err` |
| `templates` | `write_templates` (oben) | `Err` (etwa ein kaputter Block), nichts geschrieben |
| `init` | `git init -q`, ohne `-b`: der Branch heißt nach `init.defaultBranch` des Nutzers | Antwort, siehe unten |
| `add` | `git add -A` | Antwort |
| `commit` | `git -c user.useConfigOnly=true var GIT_COMMITTER_IDENT`, dann `git commit -q -m Ausgangsstand` | Antwort |

Vor `init` hört `git_init` ohne Fehler auf, mit `committed: false`:

- Ein Repository hält das Projekt schon (das eigene oder ein umschließendes, etwa ein Dotfile-Repo im
  Home-Ordner): nur die Vorlagen, `reason` = „Das Projekt liegt schon in einem Git-Repository; Gen4
  Studio hat nur die Vorlagen ergänzt.“ Nie ein Repository im Repository.
- `templates_only` (`g4k git init --templates-only`): nur die Vorlagen, kein `reason`.
- Kein `git` gefunden: nur die Vorlagen, `reason` = „Git ist nicht installiert. Die Vorlagen sind
  geschrieben; das Repository legst du an, sobald Git da ist (git-scm.com/downloads).“ Status,
  Branch und `HEAD` liest `gix` ohnehin ohne installiertes Git.

**Unterbefehle:** Das Studio ruft nur die Unterbefehle seines privaten Enums `Sub`: `--version`,
`init -q`, `add -A`, `var GIT_COMMITTER_IDENT` und `Sub::Commit { studio_identity, subject, body }`
= `commit -q -m <Betreff> [-m <Body>]` (hier `commit -q -m Ausgangsstand` ohne Body, für „Stand
sichern“ siehe unten). Betreff und Body stehen je als eigenes Argument nach ihrem `-m`, nie als
Option gelesen. Nie `remote`, `push`, `fetch`, `pull`, `clone` oder `config`; ein Test prüft die
Liste (mit `Commit` mit und ohne Body). Jeder Aufruf läuft mit leerem
stdin, `GIT_TERMINAL_PROMPT=0`, ohne `GIT_DIR`, `GIT_WORK_TREE`, `GIT_INDEX_FILE` und die übrigen
Variablen, die auf ein anderes Repository zeigen (in einem Git-Hook gesetzt), und mit einer
Zeitgrenze von 300 s; danach wird der Prozess samt Gruppe beendet.

**`git` finden** (`find_git`): Ist `G4_GIT` gesetzt, gilt nur dieser Pfad (geprüft mit
`git --version`, kein Weitersuchen); er muss absolut sein, ein relativer (auch `git` allein)
zählt als nicht gefunden. Sonst das erste `git` im `PATH`, dann `/opt/homebrew/bin/git`,
`/usr/local/bin/git`, unter Windows `%ProgramFiles%\Git\cmd\git.exe`, unter macOS zuletzt
`/usr/bin/git`. `/usr/bin/git` gilt unter macOS nur, wenn `xcode-select -p` gelingt: ohne die
Command Line Tools öffnet es den Installationsdialog. Eine App aus dem Finder hat nur
`/usr/bin:/bin:/usr/sbin:/sbin` im `PATH`, daher die festen Orte.

**Identität:** Hat Git keinen Namen konfiguriert (`git var GIT_COMMITTER_IDENT` mit
`user.useConfigOnly=true` scheitert; `GIT_COMMITTER_NAME`/`_EMAIL` in der Umgebung zählen als
konfiguriert), läuft nur dieser eine Commit mit `-c user.name="Gen4 Studio" -c
user.email=studio@gen4.invalid` (`.invalid` ist reserviert und nie zustellbar), der Bericht sagt es
(`studio_identity`). Die Konfiguration des Nutzers bleibt unberührt. `useConfigOnly` beim Prüfen
verhindert, dass Git aus Konto- und Rechnername eine Adresse rät (unter macOS etwa
`name@rechner.local`) und diese im Commit landet.

**Fehler:** Scheitert ein Unterbefehl nach dem Anlegen, bleiben Repository und Vorlagen;
`committed: false`, `reason` = „Der erste Commit ist gescheitert: `<stderr>`. Repository und Vorlagen
sind angelegt; committe mit deinem Git-Werkzeug.“ (stderr in einer Zeile, auf 400 Zeichen gekürzt).
Eine Zeitüberschreitung steht im selben Rahmen („… gescheitert: Git hat nach 5 Minuten nicht
geantwortet. Repository …“). Scheitert schon `git init`: „Git konnte das Repository nicht anlegen:
`<stderr>`. Die Vorlagen sind geschrieben; lege das Repository mit deinem Git-Werkzeug an.“ Das
Studio schaltet nichts ab, was der Nutzer eingestellt hat (`commit.gpgsign`, Hooks).

**Bericht** (`InitReport`): `templates` (wie `g4k git templates`), `committed`, `reason`, `head`
(Id, Kurz-Id, Betreff, Zeit; über `gix`), `files` (Einträge im Index), `studio_identity`,
`phase_ms`, `ms`.

**Kein Remote:** Das Studio legt nie ein Remote an und schiebt nie; Tests klonen über `git init`,
`git fetch <pfad> HEAD` und `git checkout FETCH_HEAD` (kein `git clone`, das ein `origin` einträgt)
und prüfen, dass `.git/config` kein `[remote` enthält.

**Kopflos:** `g4k git init <root> [--templates-only]` druckt die Zeilen von `git templates`, dann
`committed Ausgangsstand files=<n> branch=<b>` oder `not committed: <Grund>` (ohne Grund bei
`--templates-only`); Rückgabe 1 nur, wenn ein Unterbefehl von Git scheiterte. `g4k git status <root>`
druckt `branch <b|detached> head <kurz|none> changes <n>` bzw. `no repository`, darunter
`git version …` des gefundenen `git` oder `git not found`.

Gemessen auf Platin DE (vanilla DSPRE-Ordner, Release, 2026-10-29, `tests/git.rs`): 18 449 Dateien
im Index, 36 ignorierte Archive, `git add -A` 8,8 s, Commit 0,2 s, Status danach 75 ms, `.git`
gepackt 45,2 MiB (Grenze 65 MiB); ein Klon mit `core.autocrlf=true` baut byte-gleich.

## Stand sichern

`git::cli::commit_all` (`crates/g4-kernel/src/git/cli.rs`) legt einen Commit aller Änderungen im
eigenen Repository des Projekts an: immer alles, was die `.gitignore` zulässt (`git add -A`), nie
eine Auswahl, kein Staging. Orte: `g4k git commit-all`; App (Dialog „Stand sichern?“, Technik ›
Änderungen, Schritt Fertig von „Veröffentlichen“, Palette) und MCP (`git_commit_all`) folgen in
eigenen Schritten. Phasen (`CommitPhase`, je beim Start gemeldet, die Millisekunden im Bericht
unter `phase_ms`):

| Phase | Was | Scheitert |
|---|---|---|
| `templates` | `check_commit_all`: die Ablehnungen (unten), dann `write_templates`, dann der Status über `gix` | `Err` (Ablehnung als Satz, ein kaputter Block), nichts committet |
| `add` | `git add -A` | Antwort, siehe unten |
| `commit` | `git -c user.useConfigOnly=true var GIT_COMMITTER_IDENT`, dann `git commit -q -m <Betreff> [-m <Notiz>]` | Antwort |

**Ablehnungen** (`KernelError::Refused`, vor jedem Unterbefehl, in dieser Reihenfolge):

| Fall | Satz | geschrieben |
|---|---|---|
| kein Repository | `NO_REPO_TO_COMMIT` „Dieses Projekt liegt in keinem Git-Repository; richte Git zuerst ein.“ | nichts |
| umschließendes Repository | `ENCLOSING_REPO` „Das Projekt liegt in einem größeren Git-Repository; sichere den Stand dort mit deinem Git-Werkzeug.“ | nichts |
| kein `git` | `GIT_NEEDED` „Git ist nicht installiert; „Stand sichern“ braucht die git-Kommandozeile (git-scm.com/downloads).“ | nichts |
| keine Änderung seit `HEAD` | `NOTHING_TO_COMMIT` „Keine Änderung seit dem letzten Commit; nichts zu sichern.“ | nichts (eine nötige Änderung der Vorlagen wäre selbst eine Änderung) |

Die Vorlagen werden vor dem Status geschrieben, weil ein nachgezogener Block eine Änderung bringen
kann: ein Repository aus 4.4 kennt `!/.g4/releases.json` noch nicht, nach dem ersten „Stand
sichern“ stehen Block und Liste im Commit. Ein unbeschriebener `HEAD` (Repository von Hand mit
`git init`, ohne Commit) ist keine Ablehnung: der Commit wird der erste. `commit_all` packt nicht
aus (anders als `git_init`).

**Betreff und Notiz** (`CommitOptions { message, notes }`): Betreff ist die erste Zeile von
`message`, getrimmt; ohne (oder leer) `default_subject()` = „Stand {YYYY-MM-DD} {HH:MM} UTC“ (der
Kernel kennt keine Zeitzone; App und Assistent geben ihren Betreff in Ortszeit bzw.
„Veröffentlicht v{Version}“). Die Notiz, getrimmt, ist der Body (zweites `-m`); leer fällt er weg.

**Identität:** wie bei „Git einrichten“: ohne konfigurierte Identität nur dieser Commit mit
`-c user.name="Gen4 Studio" -c user.email=studio@gen4.invalid` (`studio_identity`), die
Konfiguration bleibt unberührt.

**Fehler:** Scheitert `git add -A` oder der Commit (ein Hook, eine Signatur), ist das kein `Err`:
`committed: false`, `reason` = „Der Commit ist gescheitert: `<stderr>`. Es ist nichts committet;
deine Änderungen bleiben im Projekt.“ (stderr in einer Zeile, auf 400 Zeichen gekürzt; eine
Zeitüberschreitung im selben Rahmen). `HEAD` bleibt, wo er war; was `git add -A` in den Index
genommen hat, bleibt dort (so gewollt: Gits Zustand nach einem gescheiterten Hook, kein Zurücksetzen
des Index durch das Studio; der nächste Versuch nimmt ohnehin alles auf). Das Studio schaltet nichts ab,
was der Nutzer eingestellt hat.

**Bericht** (`CommitReport`): `committed`, `reason`, `subject`, `head` (über `gix`), `files` (die
geänderten Pfade laut Status vor dem Commit), `studio_identity`, `phase_ms`, `ms`.

**Kein Remote:** wie überall: kein Remote, kein Push; die Tests prüfen `.git/config` auf `[remote`.

**Kopflos:** `g4k git commit-all <root> [--message <Betreff>] [--notes <Text>]` druckt
`committed <kurz> "<Betreff>" files=<n>`, bei einem gescheiterten Unterbefehl `not committed:
<Grund>` mit Rückgabe 1; Ablehnungen als Satz (`error: …`) mit Rückgabe 1. Ohne `--message` gilt
der UTC-Betreff des Kernels; wer die Ortszeit will, gibt `--message`.

Gemessen auf HeartGold DE (vanilla DSPRE-Ordner, eingerichtet, eine Karte geändert, dann
veröffentlicht; Release, 2026-10-31, `tests/git.rs`): 2 Dateien im Commit (`.g4/releases.json` und
das Karten-Member; Patch und Texte unter `build/` nicht), `git add -A` 43 ms, Commit 54 ms, Status
danach 77 ms und leer.

## Diff je Entität

`diff::diff_entity` (`crates/g4-kernel/src/diff.rs`, Seiten in `diff/side.rs`, Subjekte in
`diff/subject.rs`, Listen in `diff/lists.rs`) vergleicht eine Entität semantisch, nie Datei gegen
Datei. Sie liest nur: nichts wird geschrieben, getauft oder als abgeglichen vermerkt.

**Subjekt** (`DiffSubject`, JSON `{"kind": "trainer", "index": 1}`): die Arten der Studio-Listen
(`map`, `matrix`, `area_data`, `event_file`, `encounter`, `species`, `move`, `item`, `trainer`,
`header`, `text_archive`, `script`, `map_texture`, `building_texture`, `building_model`,
`interior_building_model`) mit einheitlichem `index` (Header-ID, Scriptdatei, Item-ID wie in der
Item-Liste und in `data/items/`), dazu `member { folder, index }` für einen Ordner unter `unpacked/`
ohne Studio-Art und `file { path }` für jede andere Projektdatei. Ein Trainer (Eigenschaften und
Team) und ein Pokémon (Daten, Attacken, Entwicklung) sind je ein Subjekt; ein Pfad wird über
`DiffSubject::of_path` zum Subjekt, eine Datei der Dynamic Headers bei angewandtem Patch zu ihrem
Header (sonst ein `member` ihres Ordners), ein Item-Member, den keine Item-ID nennt, zu `member`.

**Drei Vergleiche** (`Against`):

| Vergleich | alte Seite | neue Seite | Ablehnung |
|---|---|---|---|
| `base` „Gegen Basis“ | die Basis-ROM | der **gespeicherte** Stand (Dateien) | eine Datei („Für einzelne Dateien gibt es keinen Vergleich mit der Basis-ROM.“), keine Basis-ROM (der Grund der App bzw. die fehlende Marke) |
| `head` „Gegen letzten Commit“ | der Baum von `HEAD` | der gespeicherte Stand | kein Repository, noch kein Commit |
| `saved` „Gegen gespeichert“ | der gespeicherte Wert im Store | der **ungespeicherte** Wert im Store | ohne Store („Den ungespeicherten Stand kennt nur die App.“), ein Script (der Editor speichert sofort) |

Ein Store, der die Entität nicht geladen hat, gibt für `saved` zweimal den gespeicherten Stand
(`Same`); Texturen, Modelle, Member und Dateien haben keinen ungespeicherten Wert.

**Seiten** je Art:

| Subjekt | Basis-ROM | `HEAD` | gespeichert (Platte) |
|---|---|---|---|
| Entität aus Membern (Felder, Karte) | Member der Basis, im Speicher exportiert | Textprojekt: `data/<art>/NNNN.json` mit Nebendateien; DSPRE: Member-Blobs (fehlt `unpacked/…`, der Member im gepackten NARC von `HEAD`), im Speicher exportiert | Textprojekt: die Datei; DSPRE: die Member exportiert |
| Header | Eintrag der Tabelle der Basis mit dem Namen aus `mapname.bin` **der Platte** | Textprojekt: `data/headers/NNNN.json`; DSPRE: die Header-Dateien von `HEAD`, wenn dessen `arm9.bin` den Dynamic-Headers-Patch trägt, sonst der Ausschnitt der Tabelle aus dessen `arm9.bin`; Namen aus dem `mapname.bin` von `HEAD`, ohne die Datei Nullen | die Tabelle des Projekts, Name aus `mapname.bin` (Nullen hinter dem Dateiende) |
| Textarchiv | Member der Basis | `expanded/textArchives/NNNN.json`, sonst der Member | ebenso |
| Scriptdatei | Member der Basis, dekompiliert | `.rotom`, sonst Level-Script-`.json`, sonst der Member | ebenso |
| Textur, Modell | Member der Basis | Textprojekt: Datei unter `data/`, ohne sie „wie in der Basis-ROM“; DSPRE: der Member | ebenso |
| Member ohne Art | Member im NARC der Basis | Textprojekt: `data/raw/unpacked/<ordner>/NNNN`, ohne ihn „wie in der Basis-ROM“; DSPRE: der Member, sonst im gepackten NARC | ebenso |
| Datei | — | der Blob des Pfads | die Datei |

Steht bei einem Script auf einer Seite die Quelle und auf der anderen nur der Member (ein Commit vor
der Übernahme der Scripts), wird der Member dekompiliert wie der der Basis; nur wenn das nicht geht
(keine rotom-Konfiguration), vergleicht der Diff Bytes.

**DTO.** Beide Seiten werden zu derselben Form, bevor sie verglichen werden: das JSON des
Textprojekt-Schemas (im Textprojekt die Datei selbst, sonst die Member über den In-Speicher-Export
`EntitySource::open_for_diff`), die Nachrichten eines Textarchivs, der Quelltext eines Scripts, die
Bytes eines Members. **Ersatz-IDs im DSPRE-Layout:** ohne Register (kein Textprojekt) tragen
Trainer, Trainerklassen, Orte und Karten die Ersatz-ID (`TRAINER_0001`, `TRAINER_CLASS_0001`,
`PLACE_0001`, `MAP_0000`); die Quelle des Diffs tauft nie und schreibt nie (`write` außerhalb eines
Trockenlaufs wird abgelehnt), denn eine Taufe einer einzelnen Entität ergäbe andere Suffixe als die
Umstellung zum Textprojekt.

**Normalisierung der Teile.** Jedes Teilobjekt (`{"base_sha1": …}`, `{"file": …}`, `{"hex": …}`,
auch im `raw` von `{"locked", "raw"}`) wird vor dem Vergleich zu `{"sha1": "<40 hex>"}` der Bytes
(eine fehlende Nebendatei `{"sha1": null}`). Ein gleicher Teil in zwei Formen (Basis-Verweis gegen
Datei) ist so gleich. Gegen die Basis gilt ein Member, für den das Textprojekt keine Datei hält,
als gleich.

**Formen** (`EntityDiff`, nach der Art der Seiten):

| Form | Inhalt | Grenze |
|---|---|---|
| `fields` | je abweichendem Wert JSON-Pfad (`data.party[0].level`), alt, neu; Zeichenketten ohne Anführungszeichen, sonst kompaktes JSON, fehlend `None` | 200 Zeilen, Rest als Zahl `more` |
| `map` | Teile `model`, `permissions`, `buildings`, `bdhc`, `bgs` (`raw` einer gesperrten Karte) geändert ja/nein mit den ersten 8 Hex-Ziffern des SHA-1 je Seite; je abweichende Zelle `x, y` mit `kkcc` alt und neu; Gebäude, Quelle und Rest als Felder | 200 Zellen, 200 Felder |
| `messages` | je Nachrichtenindex neu, entfernt oder geändert, mit Wort-Spannen beider Texte (`gix::diff::blob`, Histogram; Wörter = Buchstaben/Ziffern, Leerraum, Einzelzeichen) | 200 Nachrichten |
| `lines` | Hunks mit drei Zeilen Umgebung (Scripts, Textdateien) | 400 Zeilen |
| `binary` | Größe und SHA-1 je Seite, „wie in der Basis-ROM“ oder fehlt | — |

Zustand (`DiffState`): `changed`, `new` (nur die neue Seite), `removed` (nur die alte), `same`.

**Listen.** Alle drei Vergleiche listen dieselben Zeilen (`ChangedEntry`: Gruppe, Name, Zustand
`changed`/`new`/`missing`, Ziel von „Auf Original zurücksetzen“, Store-Assets, Ordner, `subject`),
in der Gruppenfolge von 4.1b (Ordner ohne Gruppe danach, Projektdateien unter „Projektdateien“
zuletzt), in der Gruppe nach Index:

- gegen Basis: `original::changed_from_original` (4.1b), jede Zeile mit `subject`;
- gegen `HEAD`: `diff::head_entries` aus den Pfaden von `git status`: geändert und Typwechsel →
  `changed`, hinzugefügt und unverfolgt → `new`, gelöscht → `missing` (unverfolgte Dateien einzeln;
  sagt die Konfiguration des Repositorys `status.showUntrackedFiles` `no` oder falsch, wie ein
  Dotfile-Repository im Home-Ordner, keine, wie `git status`); die Pfade einer Entität (zwei
  Trainer-Member, Textarchiv-Member und seine JSON) sind eine Zeile, in verschiedenen Zuständen
  `changed`. Im DSPRE-Layout werden die Pfade, die Header tragen (eine geänderte `arm9/arm9.bin`,
  `mapname.bin`, die Dateien des Dynamic-Headers-Ordners, sobald eine Seite den Patch trägt),
  einmal Header für Header verglichen, beide Seiten so gelesen wie im Diff (Header-Dateien, wenn
  das `arm9.bin` dieser Seite den Patch trägt, sonst die Tabelle darin; Namen aus ihrem
  `mapname.bin`); gelistet wird nur ein Header, der abweicht, egal in welcher Form jede Seite ihn
  hält. Dazu `file arm9/arm9.bin`, wenn Bytes außerhalb der Tabelle abweichen (etwa die Routine
  des Patches); eine geänderte `arm9.bin` oder `mapname.bin`, die keine Zeile ergibt (nur der
  Modus, die veraltete Tabelle einer gepatchten Seite), steht als Datei da. Ohne Patch auf beiden
  Seiten hält der Dynamic-Headers-Ordner die Member des NARC (`member`, wie in der Liste gegen
  die Basis);
- gegen gespeichert: `diff::unsaved_entries` aus `Store::dirty()`, die Header-Tabelle je
  abweichendem Header.

Die Liste gegen `HEAD` zählt Entitäten; die Statusleiste zählt Pfade wie `git status`.

**Kopflos:** `g4k diff <root> <art> <index|id> [--against base|head] [--base-rom <nds>] [--json]`.
`<art>` ist das `kind` des Subjekts (`member` mit `<ordner>/<NNNN>`, `file` mit dem Pfad), die ID
wird über den `Resolver` (4.3b) aufgelöst (Pokémon, Attacken, Items, Trainer, Header als `MAP_…`);
Standard `head`. Je Zeile `field <pfad> <alt> -> <neu>`, `cell <x>,<y> <alt> -> <neu>`,
`part <name> changed|same`, `message <i> <state> <alt> -> <neu>`, `line <op> <text>`,
`binary <alt> -> <neu>` (eine Seite `<größe>:<sha1>`, `base` oder `absent`), ein fehlender Wert
`—`, Zeilenumbrüche als `\n`; zuletzt `state <state>`. `--json` druckt den `EntityDiffReport`.
Eine Ablehnung endet mit Rückgabe 1. `g4k git status <root> --entries` druckt nach der Zeile
`branch …` je Entität `<state> <gruppe> <name>`.

Gemessen auf HeartGold US (vanilla DSPRE-Ordner, Release, 2026-10-29,
`tests/git.rs::trainer_and_text_changes_show_with_field_names_against_head_and_base`, Last 9–11):
Trainer 1 und Textarchiv 1 über den Store geändert, drei Pfade im Status, zwei Zeilen in der Liste;
Status 178 ms, Liste unter 1 ms, beide Diffs gegen `HEAD` 6 ms, gegen die Basis-ROM unter 1 ms.
Ein Diff dauert auf Platin DE und HeartGold US höchstens 14 ms (Debug, `tests/diff.rs`).

## `.g4/`

`.g4/` ist lokal, bis auf eine Erlaubnisliste versionierter Dateien (C §5.1):

```gitignore
/.g4/*
!/.g4/base.json
!/.g4/preview.png
!/.g4/releases.json
```

Die oberste Ebene von `.g4/` hält Projekt-Metadaten: die Basismarke `base.json`
([project_base.md](project_base.md)), das Vorschaubild `preview.png` und die Liste der
Veröffentlichungen `releases.json`
([xdelta_patch.md](xdelta_patch.md#veröffentlichungen-g4releasesjson)). Alles andere ist lokal und
darf fehlen: `tmp/` (atomares Schreiben), `build/` (Build-Zustand, Fingerprints), `cache/`
(Basisschicht, Modelle), `backup/` (Schnappschüsse). Neue Caches sind damit ohne Änderung der
Vorlage ignoriert.
