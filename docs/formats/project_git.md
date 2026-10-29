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
nicht weiter.

### DSPRE-Layout (`dspre.gitignore`)

| Gruppe | Zeilen | Grund |
|---|---|---|
| Kopf | zwei Kommentarzeilen | Das Projekt enthält Daten aus dem Spiel: nur lokal oder privat versionieren; geteilt wird der Patch gegen die Basis-ROM (Gutachten C §4) |
| Gebaute ROMs und Patches | `*.nds`, `*.srl`, `*.xdelta`, `/build/` | Ausgaben des Builds und des Veröffentlichens; `/build/` ist das Build-Ziel der Agenten (C §2.1) |
| Emulator | `*.sav`, `*.dsv`, `*.ml[0-9]`, `*.mln`, `*.state` | Der Emulator legt Spielstände neben die ROM; liegt sie im Projekt, landen sie dort (C §2.1) |
| Studio | `/.g4/*`, `!/.g4/base.json`, `!/.g4/preview.png` | Erlaubnisliste, Abschnitt [`.g4/`](#g4) |
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
`.rotom/status/compile-state.json`, `.g4/base.json`, `.g4/preview.png`, die `.gitignore` und die
`.gitattributes`.

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
`g4-world.json`, `g4-checks.json`, `.g4/base.json`, `.g4/preview.png`), der Rest von `expanded/`,
die `.gitignore` und die `.gitattributes`. Die ID-Dateien für rotom
(`.rotom/command_database/.g4-ids-*.json`) zählt `sources()` nicht mit, und die Vorlage ignoriert
sie.

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
```

Git hält 81 bis 99 Spieldaten-Dateien für Text (`pokeHeight`, `files/data`, C §2.3). Mit
`core.autocrlf=true` (Standard von Git für Windows) schriebe Git sie beim Auschecken um, und die ROM
änderte sich, ohne dass jemand etwas bearbeitet hat. `* -text` verbietet jede Umwandlung; die
Textquellen bekommen feste LF-Zeilenenden. Eine auskommentierte LFS-Zeile hält den Platz für große
Binärquellen (Phase 6).

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
`init -q`, `add -A`, `var GIT_COMMITTER_IDENT`, `commit -q -m Ausgangsstand`. Nie `remote`, `push`,
`fetch`, `pull`, `clone` oder `config`; ein Test prüft die Liste. Jeder Aufruf läuft mit leerem
stdin, `GIT_TERMINAL_PROMPT=0`, ohne `GIT_DIR`, `GIT_WORK_TREE`, `GIT_INDEX_FILE` und die übrigen
Variablen, die auf ein anderes Repository zeigen (in einem Git-Hook gesetzt), und mit einer
Zeitgrenze von 300 s; danach wird der Prozess samt Gruppe beendet.

**`git` finden** (`find_git`): Ist `G4_GIT` gesetzt, gilt nur dieser Pfad (geprüft mit
`git --version`, kein Weitersuchen). Sonst das erste `git` im `PATH`, dann `/opt/homebrew/bin/git`,
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

## `.g4/`

`.g4/` ist lokal, bis auf eine Erlaubnisliste versionierter Dateien (C §5.1):

```gitignore
/.g4/*
!/.g4/base.json
!/.g4/preview.png
```

Die oberste Ebene von `.g4/` hält Projekt-Metadaten: die Basismarke `base.json`
([project_base.md](project_base.md)) und das Vorschaubild `preview.png`. Alles andere ist lokal und
darf fehlen: `tmp/` (atomares Schreiben), `build/` (Build-Zustand, Fingerprints), `cache/`
(Basisschicht, Modelle), `backup/` (Schnappschüsse). Neue Caches sind damit ohne Änderung der
Vorlage ignoriert.
