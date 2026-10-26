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
  "texts": { "0012": { "json": [512, 1, 1, 4711, null], "member": null } },
  "rom": { "path": "/Users/…/out.nds", "fp": [134217728, 1, 1, 815, "00ab…"] }
}
```

| Feld | Inhalt |
|---|---|
| `version` | Formatversion, heute `1` (`salt::STATE_VERSION`) |
| `steps` | je Schritt-ID (`rotom`, `chatot`, `pack:<Ordner unter unpacked/>`, `rom`) ein Eintrag |
| `steps.*.salt` | Werkzeug-Salz des Schritts (siehe unten) |
| `steps.*.scanned_at_ns` | Zeitpunkt der Aufnahme, Nanosekunden seit 1970 (für die racy-Regel) |
| `steps.*.inputs`, `outputs` | Pfad → Fingerabdruck; Pfade projektrelativ mit `/`, eine Ausgabe außerhalb des Projekts (die ROM) absolut |
| `steps.*.ms` | Dauer des letzten Laufs |
| `texts` | je Textarchiv (vierstellig) die Fingerabdrücke von JSON und Member beim letzten Abgleich (ab Task 3 von Phase 4.2) |
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
| voll | eine Ausgabe fehlt (gelöschte ROM, neuer Ausgabepfad) | `removed` |
| voll | eine Ausgabe weicht vom Eintrag ab oder steht nicht darin | `output-changed` |
| übersprungen | keine Eingabe weicht ab | – |
| inkrementell | sonst; der Schritt bekommt die geänderten, neuen und entfernten Eingaben | – |

Ein Schritt bekommt seinen Eintrag erst nach Erfolg, aufgenommen nach seinem Lauf. Scheitert er,
wird sein alter Eintrag gelöscht. `prepare_for_build` schreibt die Einträge von `rotom` und
`chatot`, `build_prepared` die der Pack-Schritte und von `rom`.

## Werkzeug-Salz

`crates/g4-kernel/build.rs` liest die Zeilen von ds-rom, rotom, chatot und uxie aus `Cargo.lock`
(`"<name> <version> <source>"`, sonst `"unknown"`). Das Salz eines Schritts ist diese Zeile plus
`#<Revision>` (`steps.rs`: `ROTOM_REV`, `CHATOT_REV`, `PACK_REV`, `ROM_REV`); `rotom` hängt die
uxie-Zeile an (`+…`), `pack:*` hat kein Werkzeug (`g4-formats narc#<PACK_REV>`). Ein Kernel-Update
ohne Werkzeugwechsel behält den Zustand, ein neues Werkzeug oder eine neue Revision lässt den
Schritt einmal voll laufen.

## Schritte

| Schritt | Eingaben | Ausgaben |
|---|---|---|
| `rotom` (nur mit `rotom.toml`) | `expanded/scripts/`, `rotom.toml`, `.rotom/status/compile-state.json`, `.rotom/command_database/`, `arm9/arm9.bin`, `unpacked/dynamicHeaders/` | der Member-Ordner der Scripts |
| `chatot` | `expanded/textArchives/`, `unpacked/textArchives/` | – |
| `pack:<Ordner>` | `unpacked/<Ordner>/` | das gepackte NARC |
| `rom` | `config.yaml`, `header.yaml`, `header_logo.png`, `path_order.txt`, `.g4/base.json`, alle Dateien unter `arm9/`, `arm9_overlays/`, `arm7/`, `banner/`, `files/` | die Ziel-ROM |

`rotom`: rotoms eigener `compile-state.json` bleibt maßgeblich; das Tor entscheidet nur, ob
`scripts::compile` läuft. `chatot` voll: wie vor dem Graphen jede JSON nach der Laderegel, deren
Texte der Member noch nicht hält; inkrementell dieselbe Regel nur für die Archive, deren JSON oder
Member sich geändert hat. `rom` liefert vier Zeilen: `blz:arm9`, `blz:overlays`, `rom`, `crc`
(`no-base-mark` ohne Basismarke, `secure-area-changed`, wenn die Secure Area nicht mehr passt).

Der BLZ-Cache liegt daneben unter `.g4/build/blz/` (`arm9-<xxh3>.blz`, `ov<NNN>-<xxh3>.blz`).
