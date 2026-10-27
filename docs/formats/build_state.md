# Bauzustand `.g4/build/state.json`

Eine Tatsache über Gen4 Studio selbst, keine Spielformat-Quelle: Der Build-Graph merkt sich je
Schritt, was er beim letzten erfolgreichen Lauf gesehen hat, und lässt einen Schritt aus, dessen
Eingaben sich seitdem nicht geändert haben. Implementierung: `crates/g4-kernel/src/build/`
(`state.rs` mit `BuildState`, `fingerprint.rs`, `step.rs` mit dem Runner, `steps.rs` mit den
Schritten, `salt.rs`).

**Die Datei ist Cache, nie Quelle.** Sie darf jederzeit gelöscht werden, ebenso der ganze Ordner
`.g4/build/` („Neu bauen (ohne Cache)“, `build::clear_cache`). Fehlt sie, ist sie kaputt oder hat
sie eine andere `version`, gilt ein leerer Zustand: Der nächste Build lässt jeden Schritt voll
laufen, ohne Fehler, und das Ergebnis ist byte-gleich.

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
| `steps` | je Schritt-ID (`rotom`, `chatot`, `pack:<Ordner unter unpacked/>`, `watch:<Ordner unter unpacked/>`, `rom`) ein Eintrag |
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

## Vergleich

Ein Ordner unter den Eingaben steht für seine Dateien (eine Ebene, versteckte ausgenommen).

- **Gleich:** Größe, mtime, ctime und Inode gleich und der Eintrag ist nicht racy.
- **Geändert:** nur die Größe weicht ab.
- Weicht der `stat` sonst ab, wird gehasht: gleicher Hash → gleich (der neue `stat` wird
  übernommen), sonst geändert. Ohne aufgezeichneten Hash gilt die Datei als geändert.
- `ctime` verrät eine Änderung an Ort und Stelle mit gleicher Größe und zurückgesetzter mtime.

**racy-Regel** (wie Git): Liegt die mtime nicht mindestens 2 s vor `scanned_at_ns`, gilt der
Eintrag als racy. Er wird schon beim Aufzeichnen mit Hash gespeichert und beim Vergleich über den
Inhalt geprüft, auch wenn der `stat` gleich ist. Die 2 s decken auch FAT und exFAT ab. Ein
übersprungener Schritt mit racy oder neu gehashten Einträgen bekommt einen erneuerten Eintrag.

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
rotoms `compile-state.json`, die JSON und Member der kodierten Textarchive); die werden nach dem
Lauf aufgenommen, ebenso die Ausgaben. Ändert jemand anderes eine Eingabe, während der Schritt
läuft, sieht der nächste Build sie als geändert. Scheitert ein Schritt, wird sein alter Eintrag
gelöscht. `prepare_for_build` schreibt die Einträge von `rotom` und
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
| `rotom` (nur mit `rotom.toml`) | alle Dateien unter `expanded/scripts/` (rekursiv, wie rotom sammelt) und unter den `include_roots` aus `rotom.toml`, `rotom.toml`, `.rotom/status/compile-state.json`, `.rotom/command_database/`, `arm9/arm9.bin`, `unpacked/dynamicHeaders/` | der Member-Ordner der Scripts |
| `chatot` | `expanded/textArchives/`, `unpacked/textArchives/` | – |
| `pack:<Ordner>` | `unpacked/<Ordner>/` | das gepackte NARC |
| `watch:<Ordner>` | `unpacked/<Ordner>/` | – |
| `rom` | `config.yaml`, `header.yaml`, `header_logo.png`, `path_order.txt`, `.g4/base.json`, alle Dateien unter `arm9/`, `arm9_overlays/`, `arm7/`, `banner/`, `files/` | die Ziel-ROM |

`rotom`: rotoms eigener `compile-state.json` bleibt maßgeblich; das Tor entscheidet nur, ob
`scripts::compile` läuft. `chatot` voll: jede JSON, die nach `TextSync` gilt (`OnlyJson`,
`JsonNewer`, `Unknown`) und deren Texte der Member noch nicht hält; inkrementell dieselbe Regel nur
für die Archive, deren JSON oder Member sich gegen den Eintrag geändert hat. Ein Archiv mit
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
