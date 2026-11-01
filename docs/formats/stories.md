# Geschichten

Eine Tatsache über Gen4 Studio selbst, keine Spielformat-Quelle: wie das Studio eine Geschichte im
Textprojekt ablegt. Eine Geschichte ordnet, was das Spiel schon hat (Scripts, Objekte, Trainer,
Flags, Variablen), zu den Schritten eines Fadens; sie belegt **eine** Fortschrittsvariable (oder
ein Flag), und jeder Schritt steht für einen Wert davon. Der Build erzeugt nichts aus ihr, die ROM
bleibt byte-gleich. Implementierung: `crates/g4-kernel/src/stories.rs`, `stories/schema.rs`
(`Story`, `Step`, `StepKind`, `Use`, `Progress`, `Layout`, `check_file`, `read_story`,
`read_layout`, `number_of`), `stories/entities.rs` (`create_story`, `update_story`,
`list_stories`), `textproject/kinds.rs` (`DATA_KINDS`, dritte Datenart).

Begründung: [Gutachten A](../research/2026-10-30-phase-3d/gutachten-a-geschichten-erzaehlen.md)
§3.2 (Modell (c), Mischform: Verweis-Modell plus Rezepte, die einmalig erzeugen) und
[Gutachten C](../research/2026-10-30-phase-3d/gutachten-c-loeschen-sprites-schnitt.md) §6.6
(Art-Beschreibung).

## Datei

`data/stories/NNNN.json`, `NNNN` dezimal, mindestens vierstellig, die Nummer aus dem Register.
Kanonisches JSON wie jede Datei des Textprojekts (Schlüssel in dieser Reihenfolge); jeder Typ lehnt
unbekannte Schlüssel ab (`textproject.schema` mit Datei, Zeile und Spalte).

```json
{
  "id": "STORY_TESTFADEN",
  "name": "Testfaden",
  "notes": "Ein Faden zum Testen.",
  "progress": {
    "var": "VAR_STORY_TESTFADEN"
  },
  "steps": [
    {
      "key": "s1",
      "value": 1,
      "kind": "dialog",
      "name": "Gespräch mit dem Wächter",
      "made": {
        "recipe": "story.talk_advances",
        "studio": "0.3.0"
      },
      "uses": [
        {
          "object": {
            "file": 2,
            "local_id": 4
          }
        },
        {
          "script": {
            "file": 2,
            "slot": 7
          }
        }
      ]
    }
  ],
  "flags": [
    "FLAG_STORY_TESTFADEN_WAECHTER"
  ]
}
```

| Feld | Inhalt |
|---|---|
| `id` | `STORY_…`, gleich der ID des Registers für diese Nummer (sonst `textproject.id-mismatch`) |
| `name` | Name für die Oberfläche, nicht leer |
| `notes` | Notizen, fehlt, wenn leer |
| `progress` | genau ein Schlüssel: `{ "var": "VAR_…" }` oder `{ "flag": "FLAG_…" }` |
| `steps` | die Schritte, fehlt, wenn leer |
| `flags` | weitere Flags, die die Geschichte belegt, fehlt, wenn leer |

## Schritte und Arten

Ein Schritt ist `{ key, value, kind, name, notes?, made?, uses? }`: `key` ist innerhalb der
Geschichte eindeutig (ein doppelter Schlüssel ist `textproject.schema` am zweiten), `value` der
Wert der Fortschrittsvariable (16 Bit, `0` bis `65535`), `made` nennt das Rezept, das den Schritt
angelegt hat (ab 3d.7), mit der Studio-Version. Die Arten sind englische Bezeichner wie alle
JSON-Schlüssel; die Oberfläche sagt sie deutsch:

| `kind` | Wort |
|---|---|
| `start` | Start |
| `dialog` | Dialog |
| `condition` | Bedingung |
| `battle` | Kampf |
| `reward` | Belohnung |
| `progress` | Fortschritt |
| `appearance` | Erscheinen/Verschwinden (die Richtung steht im Namen des Schritts) |

## Verweise

`uses` ist eine Liste, jeder Eintrag hat genau einen Schlüssel: `trainer` (`TRAINER_…`), `map`
(`MAP_…`), `flag` (`FLAG_…`), `var` (`VAR_…`), `object { file, local_id }`,
`script { file, slot }`, `message { archive, index }`, `levelScript { file, condition }`.

Die Verweise einer Geschichte sind **schwach**: Eine Geschichte ist Dokumentation, sie sperrt kein
Löschen und hält keinen Build auf. Der Check meldet eine ID in `progress`, `uses` oder `flags`, die
im Projekt nichts nennt (Resolver: Tabellen, Register, pret-Namen, Ersatz-IDs), als Warnung
`story.dangling-ref` am JSON-Pfad (`story.dangling-ref: progress.var: Unbekannte ID: VAR_NOPE.`).
Verweise über Nummern (`object`, `script`, `message`, `levelScript`) prüft der Check nicht.
„ID umbenennen“ zieht jede ID in einer Geschichte mit, wie jede JSON-Zeichenkette unter `data/`
([text_project.md](text_project.md) „ID umbenennen“).

## Beifile

`data/stories/NNNN.layout.json` hält die Lage der Knoten im Graph, je Schlüssel eines Schritts:
`{ "nodes": { "s1": { "x": 120.0, "y": -40.5 } } }`. Getrennt von der Geschichte, damit das Ziehen
von Knoten den Diff der Bedeutung nicht verrauscht. Beim Anlegen ist es leer (`{ "nodes": {} }`);
ein fehlendes Beifile ist kein Fehler (der Graph legt sich selbst aus), ein unlesbares ist
`textproject.schema`.

## Register und ID

Das Register `data/ids/stories.json` (ID → Nummer) ist das siebte Register; der Export schreibt
es (leer als `{}`). Die ID entsteht beim Anlegen aus dem Namen: `STORY_` und der Name nach den
Regeln der Taufe (`symbols::normalize` mit deutschen Umlauten: „Rüpel & Co“ → `STORY_RUEPEL_AND_CO`),
auf 64 Zeichen gekürzt; ist sie vergeben, `_2`, `_3` …; ein Name, von dem nichts oder nur Ziffern
bleiben, gibt die Ersatz-ID `STORY_NNNN`. Die Nummer ist die kleinste freie ab 0 (kein
Registereintrag und keine Datei). Ein Registereintrag ohne Datei ist `textproject.id-dangling`
(Warnung). In die ID-Dateien für rotom kommt keine Geschichte (Scripts kennen sie nicht).
`rename_id STORY_A STORY_B` ändert Register und `id` der Datei; die Nummer bleibt.

## Dateischritt statt Store-Asset

Die Spezifikation nennt eine Store-Art `Story` (kein Member, kein Build-Schritt). Umgesetzt ist die
Geschichte **ohne** Store-Asset: `AssetKind::ALL` zählt Archive, Build und Pack, ein Asset ohne
Member bräuchte eine neue Art von Ablageort im Store, und den baut Phase 3d.4 zur selben Zeit um.
Anlegen und Ändern sind darum je **ein** `FileStep` (Asset: die Header-Tabelle, Art
`FileStepKind::Reset`) mit Register, Geschichte und Beifile, aufgezeichnet über
`Store::record_file_step`, wie „Als eigenes Flag benennen…“. ⌘Z, Verlauf und Label gelten wie bei
jeder anderen Änderung (`Geschichte „Testfaden“ angelegt`, `Geschichte „Testfaden“ geändert`);
„ungespeichert“ gibt es für Geschichten nicht, die Dateien sind mit dem Schritt geschrieben. Undo
und Redo prüfen jede Datei gegen ihre Seite und lehnen ab, wenn ein anderes Werkzeug sie
inzwischen geändert hat.

Abgelehnt wird vor jedem Schreiben (`KernelError::Refused` mit Satz): kein aktuelles Textprojekt
(`Kein Textprojekt: IDs gibt es erst nach „Als Textprojekt einrichten…“.`), ein leerer Name
(`Die Geschichte braucht einen Namen.`), eine Fortschritts-ID, die nichts nennt
(`Unbekannte ID: VAR_NOPE.`), ein Schlüssel in zwei Schritten
(`Der Schlüssel „s1“ steht in zwei Schritten; jeder Schritt braucht einen eigenen.`); beim Ändern
dazu eine unbekannte Geschichte (`Unbekannte ID: STORY_X.`) und `Nichts zu ändern.`. Ändern ersetzt
jedes angegebene Feld, `layout` schreibt nur das Beifile; die `id` ändert nur „ID umbenennen“.
Ohne Store (`create_story_files`, `update_story_files`, für `g4k` und Tests) entstehen dieselben
Dateien ohne Undo. `list_stories` liefert die Geschichten des Registers nach Nummer, eine Datei,
die sich nicht lesen lässt, fehlt oder eine andere `id` hat, als ihr Problem statt eines Fehlers.

Export, Import und Build fassen `data/stories/` nicht an; `textproject::sources` und damit Git
führen beide Dateien (ein Textprojekt mit Geschichte baut die Basis-ROM byte-gleich).

## Fäden

Folgt mit Task 3 (Fäden aus dem Referenzindex, Graph einer Geschichte).

## Befehle

Folgt mit Task 5 (`story_list`, `story_get`, `story_create`, `story_update`, `g4k story …`).

## Löschen

Folgt mit Teil B (`story_delete`: ein Dateischritt, der Geschichte, Beifile und Registereintrag
entfernt).
