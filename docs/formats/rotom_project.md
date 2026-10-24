# rotom-Projekt (`rotom.toml`, `.rotom/`)

Quelle: rotom v0.7.0 (`src/project/config.rs`, `src/project/compile.rs`, `src/compile_state.rs`,
`src/project/init.rs`). Das Studio liest und schreibt dasselbe Layout wie `rotom init` und DSPRE,
damit beide Werkzeuge dasselbe Projekt benutzen können. Die Implementierung steht in
`crates/g4-kernel/src/scripts.rs` (`adopt`, `compile`) und `crates/g4-kernel/src/scripts/project_compile.rs`
(der Compile-Lauf).

## Layout

```
rotom.toml                          Konfiguration, wird beim Übernehmen geschrieben
.rotom/command_database/NAME_v2.json  Befehlsdatenbank (NAME = platinum | hgss | diamond_pearl)
.rotom/status/compile-state.json    Übersetzungsstand
.rotom/cache/                       Zwischenspeicher von rotom (leer bei DSPRE-Projekten)
expanded/scripts/NNNN.rotom         Quelltext einer Scriptdatei
expanded/scripts/NNNN.json          Level-Script (Quelle)
expanded/textArchives/NNNN.json     Textarchive, die rotom für Textliterale braucht
unpacked/scripts/NNNN               Member des Script-Archivs (von rotom geschrieben)
```

`expanded/scripts/NNNN.script` sind DSPREs Textexporte (anderes Format); sie liegen nie neben den
`.rotom`-Quellen, rotom meldet sonst eine Ausgabekollision, und das Studio übernimmt solche
Projekte nicht.

## `rotom.toml`

Vom Studio geschrieben (`toml::to_string_pretty` über rotoms `RotomConfig`):

```toml
format_version = 1

[project]
name = "Pokemon - Platin-Edition (Germany)"   # Ordnername ohne _DSPRE_contents

[workspace]
project_type = "dspre"
game_family = "Platinum"                       # DP | Platinum | HGSS

[paths]
database_dir = ".rotom/command_database"
cache_dir = ".rotom/cache"
status_dir = ".rotom/status"
source_roots = ["expanded/scripts"]
binary_roots = ["unpacked/scripts"]            # "unpacked/Scripts" in Avalonia-DSPRE-Projekten

[database]
default_file = ".rotom/command_database/platinum_v2.json"
```

`default_file` ist ab der Projektwurzel gerechnet (rotom löst es dagegen auf). Ein Projekt, das die
rotom-CLI oder DSPRE angelegt hat, hat dieselben Schlüssel; das Studio benutzt es unverändert.

## Befehlsdatenbank

`DatabaseV2`-JSON mit allen Befehlen, Argumenttypen und Konstanten einer Familie. Das Studio
kopiert die Dateien aus `crates/g4-kernel/metadata/scrcmd/` (Revision 33 des scrcmd-database-
Releases, siehe dort `SOURCE.md`), nie die in rotom eingebettete Kopie und nie aus dem Netz.
Ändert sich die Datei, ist der Stand veraltet und alles wird neu übersetzt.

## `compile-state.json`

```json
{
  "version": 4,
  "db_hash": 10215144399853279044,
  "compiler_version": "0.7.0",
  "entries": {
    "expanded/scripts/0000.rotom": {
      "source_hash": 8130443045825130790,
      "output_hash": 8406560577717197405,
      "dependency_hashes": {},
      "status": "Decompiled",
      "last_compiled": "2026-10-22T21:05:24.950297Z",
      "quirks": { "jump_table_end_marker_count": null, "levelscript_padding": null }
    }
  }
}
```

- Hashes sind xxh3-64 (`xxhash-rust`) über die Bytes der Quelle (`source_hash`) und des Members
  (`output_hash`). `status` ist `Decompiled` (nach der Übernahme), `Compiled`, `Transpiled` oder
  `Dirty` (letzter Versuch scheiterte).
- **Veraltet** ist eine Quelle, wenn ihr Eintrag fehlt oder `Dirty` ist, oder wenn ihr Hash, der des
  Members (fehlt: 0) oder einer ihrer Abhängigkeiten vom Eintrag abweicht. Gelten `version`,
  `db_hash` oder `compiler_version` nicht mehr, ist alles veraltet. Mtimes zählen nie.
- **Quirks** halten fest, was am Original ungewöhnlich war: `jump_table_end_marker_count`
  (Anzahl der `0xFD13`-Marken, `null` = eine) und `levelscript_padding` (zusätzliche Nullbytes).
  Der Decompiler schreibt sie, der Compiler wendet sie an, damit eine unveränderte Quelle dieselben
  Member-Bytes ergibt. rotom v0.7.0 ersetzt den Eintrag einer Quelle, die nicht übersetzt, durch
  einen ohne Quirks; `scripts::compile` setzt sie danach zurück.
- Nach der Übernahme ist nichts veraltet; ein Member, dessen Quelle unverändert ist, wird nie
  angefasst.

## Textarchive

Ein Textliteral (`Message "…"`) fügt rotom als Nachricht an das Textarchiv des Map-Headers der
Scriptdatei an (`expanded/textArchives/NNNN.json`, Nachrichten-ID `msg_NNNN_<index>`); maßgeblich
ist der erste Header der Tabelle, der die Scriptdatei nennt. Die Header-Tabelle liest das Studio
selbst (`Project::read_headers`: `arm9.bin` oder, mit dem Patch „Dynamische Header“, die Dateien
in `unpacked/dynamicHeaders`) und gibt sie rotom mit. rotoms eigenes `compile_project` (und die
`rotom`-CLI) lädt das Projekt über uxie 0.9.7, das in einem DSPRE-Projekt nur die ersten 559
(Platin) bzw. 540 (HGSS) Header aus `arm9.bin` liest: Mit dem Patch ist das die stehengebliebene
alte Tabelle, neue Header fehlen ganz. Deshalb läuft der Compile im Studio über rotoms
Einbettungs-API (`ProjectContext::from_parts`, `compile_batch`) und führt `compile-state.json`
genau wie `compile_project` (gleiche Einträge, gleiche Abhängigkeiten). rotom braucht
diese JSON für jede Scriptdatei, die es übersetzt; fehlt sie, weil DSPRE die Archive nicht
expandiert hat, schreibt das Studio sie aus dem Member (`unpacked/textArchives/NNNN`) und
übersetzt erneut. Geänderte Archive kodiert das Studio danach zum Member (siehe
`text_archive.md`).
