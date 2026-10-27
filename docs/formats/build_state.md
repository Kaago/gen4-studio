# Bauzustand `.g4/build/state.json`

Eine Tatsache über Gen4 Studio selbst, keine Spielformat-Quelle: Der Build-Graph merkt sich je
Schritt, was er beim letzten erfolgreichen Lauf gesehen hat, und lässt einen Schritt aus, dessen
Eingaben sich seitdem nicht geändert haben. Implementierung: `crates/g4-kernel/src/build/`
(`state.rs` mit `BuildState`, `fingerprint.rs`, `step.rs` mit dem Runner, `steps.rs` mit den
Schritten, `salt.rs`).

**Die Datei ist nie Quelle.** Sie darf gelöscht werden, ebenso der ganze Ordner `.g4/build/`. Fehlt
sie, ist sie kaputt oder hat sie eine andere `version`, gilt ein leerer Zustand: Der nächste Build
lässt jeden Schritt voll laufen, ohne Fehler, und das Ergebnis ist byte-gleich. Die Einträge unter
`texts` sind dabei mehr als Cache: Sie entscheiden, ob die JSON oder der Member eines Textarchivs
gilt. Ohne sie gilt für jedes Archiv die JSON, auch wenn DSPRE den Member seitdem geändert hat (der
Build meldet das als Hinweis).

**„Neu bauen (ohne Cache)“** (`build::clear_cache`, `g4k build --no-cache`, `build_rom` mit
`fresh: true`) löscht darum alles unter `.g4/build/` (BLZ-Cache, Schritt-Einträge, `rom`), behält
aber die `texts`-Einträge: Zuerst ersetzt es `state.json` atomar durch einen Zustand nur mit
ihnen, erst danach löscht es den Rest des Ordners, sodass ein Absturz dazwischen den Textabgleich
nicht verliert. Alle Schritte laufen danach
voll (`no-state`), die Seiten der Textarchive folgen weiter den Fingerabdrücken. Fehlt `state.json`
oder ist sie kaputt, verschwindet nur der Ordner.

## Felder

JSON, kompakt, UTF-8.

```json
{
  "version": 1,
  "steps": {
    "chatot": {
      "salt": "chatot 0.8.1 registry+https://github.com/rust-lang/crates.io-index#1",
      "scanned_at_ns": 1761480000123456789,
      "inputs": { "expanded/textArchives/0012.json": [512, 1761470000000000000, 1761470000000000000, 4711, null] },
      "outputs": {},
      "ms": 31
    }
  },
  "texts": { "0012": { "json": [512, 1, 1, 4711, "00ab…"], "member": [380, 1, 1, 4712, "00cd…"], "scanned_at_ns": 1761480000123456789 } },
  "rom": { "path": "/Users/…/out.nds", "fp": [134217728, 1, 1, 815, "00ab…"] }
}
```

| Feld | Inhalt |
|---|---|
| `version` | Formatversion, heute `1` (`salt::STATE_VERSION`) |
| `steps` | je Schritt-ID (`textproject`, `rotom`, `chatot`, `pack:<Ordner unter unpacked/>`, `watch:<Ordner unter unpacked/>`, `rom`) ein Eintrag |
| `steps.*.salt` | Werkzeug-Salz des Schritts (siehe unten) |
| `steps.*.scanned_at_ns` | Zeitpunkt der Aufnahme, Nanosekunden seit 1970 (für die racy-Regel) |
| `steps.*.inputs`, `outputs` | Pfad → Fingerabdruck; Pfade projektrelativ mit `/`, eine Ausgabe außerhalb des Projekts (die ROM) absolut |
| `steps.*.ms` | Dauer des letzten Laufs |
| `texts` | je Textarchiv (vierstellig) die Fingerabdrücke von JSON und Member beim letzten Abgleich, immer mit Hash, und `scanned_at_ns` (fehlt es, gilt der Eintrag als racy); siehe „Welche Fassung gilt“ in [text_archive.md](text_archive.md) |
| `rom` | die zuletzt geschriebene ROM: Pfad wie in `outputs` und ihr Fingerabdruck |

Ein **Fingerabdruck** ist ein Array `[size, mtime_ns, ctime_ns, ino, "xxh3hex" | null]`: Größe,
Änderungszeit und Statusänderungszeit in Nanosekunden, Inode und der xxh3-64 des Inhalts als
16 Hex-Ziffern, wenn er berechnet wurde. Unter Windows sind `ctime_ns` und `ino` 0.

Geschrieben wird nur über `BuildState::update` (laden, ändern, atomar über `.g4/tmp/` schreiben,
unter einer prozessweiten Sperre). Zwei Prozesse auf demselben Projekt können sich Einträge
überschreiben; das kostet höchstens einen vollen Lauf.

Der Prozess hält den zuletzt gelesenen oder geschriebenen Zustand je Projekt im Speicher, mit dem
`stat` der Datei (Größe, mtime, ctime, Inode; nach dem Schreiben über das Handle der geschriebenen
Datei genommen). Solange die Datei diesen `stat` hat, wird sie nicht neu gelesen; jeder andere
`stat` (ein anderer Prozess, eine gelöschte oder von Hand geänderte Datei) liest sie neu. Jeder
Schreiber ersetzt die Datei, also ändert jedes Schreiben den `stat`; darauf baut auch der Store,
der seine Textstände neu lädt, sobald sich der `stat` ändert. Ein Build schreibt die Datei je
Hälfte einmal: `prepare_for_build` am Ende (dazu die Textstände, die `rotom` und `chatot` aufnehmen),
`build_prepared` am Ende für alle Pack-, Beobachtungs- und den ROM-Schritt, auch wenn ein Schritt
scheitert. Kann der Kernel nach dem Schreiben den `stat` der geschriebenen Datei nicht lesen, merkt
er sich nichts; das nächste Lesen liest die Datei (`fsutil::write_atomic` scheitert daran nie, die
Bytes liegen schon).

## Vergleich

Ein Ordner unter den Eingaben steht für seine Dateien (eine Ebene, versteckte ausgenommen).

- **Gleich:** Größe, mtime, ctime und Inode gleich und der Eintrag ist nicht racy.
- **Geändert:** nur die Größe weicht ab.
- Weicht der `stat` sonst ab, wird gehasht: gleicher Hash → gleich (der neue `stat` wird
  übernommen), sonst geändert. Ohne aufgezeichneten Hash gilt die Datei als geändert.
- `ctime` verrät eine Änderung an Ort und Stelle mit gleicher Größe und zurückgesetzter mtime.

**racy-Regel** (wie Git): Liegt die spätere von mtime und ctime nicht mindestens 2 s vor
`scanned_at_ns`, gilt der Eintrag als racy (die ctime fängt eine gerade geschriebene Datei mit
zurückgesetzter mtime, etwa einen kodierten Member; unter Windows ist sie 0). Er wird schon beim Aufzeichnen mit Hash gespeichert und beim Vergleich über den
Inhalt geprüft, auch wenn der `stat` gleich ist. Die 2 s decken auch FAT und exFAT ab. Ein
übersprungener Schritt mit racy oder neu gehashten Einträgen bekommt einen erneuerten Eintrag.

Eine Ausgabe, die der Schritt gerade geschrieben hat (die ROM, ein NARC), ist beim Aufzeichnen
immer racy. Ihren Hash nimmt der Eintrag aus den Bytes, die der Schritt geschrieben hat
(`StepRun.hashes`; bei der ROM der Kopf bis `0x8000` aus der Datei, weil `restore_header_crcs` dort
die CRCs einträgt, der Rest aus dem Speicher), statt die Datei noch einmal zu lesen. Der nächste
Build vergleicht sie trotzdem über den Inhalt, denn eine Änderung von außen im selben Zeitstempel
zeigt nur der Inhalt. Weicht die Datei vom aufgezeichneten Hash ab (auch wenn der Hash selbst
falsch wäre), läuft der Schritt voll (`output-changed`); als aktuell gilt nie eine Ausgabe, die es
nicht ist. Gehasht wird eine große Datei in Stücken von 1 MB.

rotoms Übersetzung liest bei jedem Lauf alle Quellen und Ausgaben, um ihre Hashes mit
`compile-state.json` zu vergleichen. Der Kernel merkt sich diese Hashes im Prozess mit dem `stat`
der Datei (`fingerprint::hash_remembered`) und nimmt sie nur wieder, wenn der `stat` gleich ist
und die Datei beim Hashen nicht racy war; die Werte sind dieselben, rotoms Zustand bleibt
maßgeblich. Der Speicher fasst 16 384 Dateien (ein Projekt hat rund 3 000 Quellen, Ausgaben und
Include-Dateien; zusammen etwa 4 MB), danach beginnt er von vorn.

## Entscheidung je Schritt

| Ergebnis | wann | `reason` |
|---|---|---|
| voll | kein Eintrag (auch nach „ohne Cache“ oder kaputter Datei) | `no-state` |
| voll | Salz weicht ab | `build.cache-stale` |
| voll | eine Ausgabe fehlt (gelöschte ROM, neuer Ausgabepfad, eine aufgezeichnete Datei fehlt im Ausgabeordner) | `removed` |
| voll | eine Ausgabe weicht vom Eintrag ab oder steht nicht darin | `output-changed` |
| übersprungen | keine Eingabe weicht ab | – |
| inkrementell | sonst; der Schritt bekommt die geänderten, neuen und entfernten Eingaben | – |

Ein Schritt bekommt seinen Eintrag erst nach Erfolg. Die Eingaben stehen darin so, wie sie vor
dem Lauf waren (was der Schritt gelesen hat), außer denen, die er selbst schrieb (`StepRun.wrote`:
rotoms `compile-state.json`, die Member der kodierten Textarchive); die werden nach dem
Lauf aufgenommen, ebenso die Ausgaben. Ändert jemand anderes eine Eingabe, während der Schritt
läuft, sieht der nächste Build sie als geändert. Scheitert ein Schritt, wird sein alter Eintrag
gelöscht. Ausnahme ist ein Textkonflikt (`KernelError::TextConflict`): `rotom` und `chatot` melden
ihn, bevor sie etwas schreiben, also behalten sie ihren Eintrag, und der Build nach dem Auflösen
läuft inkrementell statt mit `no-state` („erster Build“). `prepare_for_build` schreibt die Einträge von `textproject`, `rotom` und
`chatot`, `build_prepared` die der Pack- und Beobachtungs-Schritte und von `rom`.

## Werkzeug-Salz

`crates/g4-kernel/build.rs` liest die Zeilen von ds-rom, rotom, chatot und uxie aus `Cargo.lock`
(`"<name> <version> <source>"`, sonst `"unknown"`). Das Salz eines Schritts ist diese Zeile plus
`#<Revision>` (`steps.rs`: `ROTOM_REV`, `CHATOT_REV`, `PACK_REV`, `ROM_REV`); `rotom` hängt die
uxie-Zeile an (`+…`), `pack:*` hat kein Werkzeug (`g4-formats narc#<PACK_REV>`), `watch:*` ebenso
nicht (`watch#<WATCH_REV>`). Ein Kernel-Update
ohne Werkzeugwechsel behält den Zustand, ein neues Werkzeug oder eine neue Revision lässt den
Schritt einmal voll laufen.

## Schritte

| Schritt | Eingaben | Ausgaben |
|---|---|---|
| `textproject` (nur mit `g4-project.json`, vor `rotom`) | `g4-project.json`, `data/ids/`, `data/rotom-quirks.json`, je bekannte Art `data/<art>/`, alle Dateien unter `data/raw/` (rekursiv, vom Schritt gelistet) und je Blob `data/raw/X` die Datei `X`, je abgebildete Art der Member-Ordner unter `unpacked/`, `arm9/arm9.bin`, `files/fielddata/maptable/mapname.bin`, bei angewandtem Dynamic-Headers-Patch `unpacked/dynamicHeaders/` | – |
| `rotom` (nur mit `rotom.toml`) | alle Dateien unter `expanded/scripts/` (rekursiv, wie rotom sammelt) und unter den `include_roots` aus `rotom.toml`, `rotom.toml`, `.rotom/status/compile-state.json`, `.rotom/command_database/`, `arm9/arm9.bin`, `unpacked/dynamicHeaders/` | der Member-Ordner der Scripts |
| `chatot` | `expanded/textArchives/`, `unpacked/textArchives/` | – |
| `pack:<Ordner>` | `unpacked/<Ordner>/` | das gepackte NARC |
| `watch:<Ordner>` | `unpacked/<Ordner>/` | – |
| `rom` | `config.yaml`, `header.yaml`, `header_logo.png`, `path_order.txt`, `multiboot_signature.yaml`, `.g4/base.json`, alle Dateien unter `arm9/`, `arm9_overlays/`, `arm7/`, `arm7_overlays/`, `banner/`, `files/` (`multiboot_signature.yaml` und `arm7_overlays/` schreibt ds-rom nur, wenn die ROM sie hat; keines der drei Spiele hat sie) | die Ziel-ROM |

`textproject`: schreibt je Einheit (Entität, Blob, Header-Tabelle, Quirks) die Member aus der JSON,
wenn sich inkrementell nur die JSON-Seite geändert hat; ist nur der Member neuer, bleibt er
(Hinweis `textproject.member-newer`), sind beide geändert, scheitert der Schritt
(`textproject.conflict`); voll schreibt er nur über fehlende oder der Basisschicht gleiche Member,
sonst `textproject.unsynced`. Die Regeltabelle steht in [text_project.md](text_project.md) unter
„Build“. Die Member sind Eingaben, keine Ausgaben (wie bei `chatot`); die geschriebenen stehen in
`StepRun.wrote`. Die als neuer behaltenen Member merkt er sich in `.g4/build/textproject.json`
(`{"version": 1, "member_newer": ["unpacked/…/NNNN", …]}`, fehlt, wenn es keine gibt), damit eine
spätere Änderung ihrer JSON ein Konflikt wird; „ohne Cache“ löscht die Datei mit, der volle Lauf
danach urteilt über die Basisschicht. Salz: `textproject#<TEXTPROJECT_REV>+schema<SCHEMA_VERSION>+`
und die uxie-Zeile.

`rotom`: rotoms eigener `compile-state.json` bleibt maßgeblich; das Tor entscheidet nur, ob
`scripts::compile` läuft. `chatot` voll: jede JSON, die nach `TextSync` gilt (`OnlyJson`,
`JsonNewer`, `Unknown`) und deren Texte der Member noch nicht hält; inkrementell dieselbe Regel nur
für die Archive, deren JSON oder Member sich gegen den Eintrag geändert hat. Kodiert wird nur der
Member (`text::encode_member`); die JSON ist die Quelle und behält ihre Bytes (Formatierung, Zeilenende
am Schluss, wie Git sie auscheckt), der Member bekommt höchstens ihre mtime. Ein Archiv mit
`Conflict` lässt den Schritt scheitern (`KernelError::TextConflict`), bevor er schreibt. `rom` liefert vier Zeilen: `blz:arm9`, `blz:overlays`, `rom`, `crc`
(`no-base-mark` ohne Basismarke, `secure-area-changed`, wenn die Secure Area nicht mehr passt).

`pack:<Ordner>` voll packt den ganzen Ordner (`pack_dir`). Inkrementell liest er nur die
geänderten und neuen Member-Dateien, liest das gepackte NARC, ersetzt die geänderten Member und
hängt die neuen an, wenn sie lückenlos ab der bisherigen Länge folgen. Ein NARC hat genau ein
Layout (Member auf 4 Bytes mit `0xFF` aufgefüllt), also ist das Ergebnis byte-gleich zu einem
vollen Packen. Statt einzusetzen packt er voll, und die Zeile zeigt „voll“: bei einem entfernten
Member (`removed`, das NARC schrumpft) und wenn die Member nicht zum NARC passen
(`output-changed`). Geschrieben wird das NARC nur bei geänderten Bytes, der Elternordner wird
dabei angelegt (ein Klon ohne die ignorierten NARCs).

`pack:<Ordner>` gibt es je Archiv (`AssetKind::ALL`-Reihenfolge) und danach je Pack-Eintrag aus
`archives.yaml` (`pack_only`: DSPRE-Ordner, die kein Editor liest, nur Ordner, deren Vanilla-Repack
auf Platin DE, HeartGold DE und HeartGold US byte-gleich ist), jeweils nur, wenn der Ordner existiert.

`watch:<Ordner>` steht für jeden übrigen Unterordner von `unpacked/` (`build::unpacked_unpacked`).
Der Schritt läuft nie und schreibt nichts, seine Zeile ist immer „übersprungen“ und wird nicht
angezeigt. Sein Eintrag ist die Grundlinie für die Prüfung `build.unpacked-not-packed`: Ohne
Eintrag (oder mit anderem Salz) nimmt er jede Datei des Ordners mit Inhaltshash auf, damit ein
kopiertes Projekt über den Inhalt vergleicht. Danach erneuert er nur den `stat` der Dateien mit
unverändertem Inhalt; eine geänderte oder entfernte Datei behält ihren Fingerabdruck der
Grundlinie, eine neue kommt nicht hinzu. Die Grundlinie gilt also bis „Neu bauen (ohne Cache)“.
Verschwindet der Ordner (oder packt ihn ein neuer Pack-Eintrag), entfällt sein Eintrag beim
nächsten Bauen; ein neu angelegter Ordner bekommt eine neue Grundlinie.

Der BLZ-Cache liegt daneben unter `.g4/build/blz/` (`arm9-<xxh3>.blz`, `ov<NNN>-<xxh3>.blz`).
Eine Cache-Datei, deren BLZ-Fußzeile nicht zum Modul passt, gilt als Fehltreffer und wird neu
geschrieben; die komprimierten Daten selbst prüft der Build nicht. Eine trotzdem beschädigte Datei
behebt „Neu bauen (ohne Cache)“, das den Cache löscht.
