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

Ein **Faden** ist eine Variable, mit der das Spiel eine Geschichte erzählt; `stories::threads`
findet sie im Referenzindex (Messung: Gutachten A §2.2, `var_threads` in
`docs/research/2026-10-30-phase-3d/gutachten-a-story.py`). Regel (`threads()`):

- die Variable liegt im allgemeinen Bereich (`area_of(family, Var, n) == General`, ab `0x4030`;
  nicht kartenlokal, Objektgrafik oder Script-lokal; `is_story_var`);
- ihr pret-Hauptname fehlt oder enthält weder `UNUSED` noch `DUMMY` (`UNK` zählt:
  `VAR_UNK_4031` kann ein Faden sein, `VAR_UNUSED_0x406C` nie);
- sie hat mindestens **drei verschiedene Werte** über alle Verweise mit Wert (`Ref.value`: gesetzt
  mit `SetVar`/`SetVarFromValue`, verglichen mit `CompareVarToValue`, einer Bedingung
  `VAR_X == 3`, am Auslöser `value`, im Level-Script `var_conditions[i].value`);
- und mindestens **zwei verschiedene Orte**. Ein Ort ist der erste Header (nach Nummer), dessen
  `script_file_id`, `level_script_id` (für die Bedingungen eines Level-Scripts) oder
  `event_file_id` die Quelle nennt, sonst die Quelle selbst. Jeder Verweis zählt für die Orte,
  auch einer ohne Wert (`SetVar VAR_X, VAR_Y`).

Ein `Thread` trägt `var`, den pret-Hauptnamen (`name`), den eigenen Namen aus
`data/ids/vars.json` (`own_id`), die Werte aufsteigend (`values`), die Zahl der Orte (`places`), die
Header unter den Orten aufsteigend (`headers`), jeden Verweis als `Place` (Quelle, Feld, Platz,
Zugriff, Wert, Header) und die erste Geschichte (nach Nummer), deren `progress.var` die Variable
ist (`story`). Die Liste ist nach Nummer sortiert; ein Lauf geht einmal über den Index und liest
Header-Tabelle, Variablen-Register und Geschichten je einmal. Eine Geschichte, die sich nicht
lesen lässt oder deren Fortschritt nichts nennt, beansprucht keinen Faden; das Register der
Variablen oder der Geschichten, das sich nicht lesen lässt, ist der Fehler des Aufrufs. Lässt sich
ein anderes Register nicht lesen (etwa `data/ids/trainers.json`), beansprucht keine Geschichte
einen Faden, die Fäden stehen trotzdem in der Liste (die Prüfung nennt das Register).

Auf den Vanilla-Spielen (`tests/threads.rs`, gated) findet der Index 60 Fäden in Platin DE
(274 Werte, höchstens 16 Werte und 22 Orte) und 62 in HeartGold DE und US (310 Werte,
höchstens 14 Werte und 29 Orte); die Messung nannte 59 / 62 Fäden und 272 / 311 Werte. Die
Untergrenzen 59 und 62 prüft der Test.

### Graph eines Fadens

`thread_graph(store, index, var, story)` baut den Graph einer Variable aus ihren Verweisen:

| Knoten | `id` | Inhalt |
|---|---|---|
| `Value` | `v:<wert>` | ein Knoten je Wert, aufsteigend; `step` der Schlüssel des ersten Schritts der Geschichte mit diesem Wert; `noted_only` für einen Wert, den nur ein Schritt nennt (keine Kanten, gestrichelt) |
| `Place` | `p:<art>:<nummer>` (`p:script:0101`, `p:event:0394`) | ein Knoten je Quelle in Index-Reihenfolge, mit `header` (der erste Header, wie oben) und `label` („Scriptdatei 0101“, „Level-Script 0005“, „Eventdatei 0394“; `place_label`); `loose`, wenn sie keine Kante hat (keiner ihrer Verweise trägt einen Wert, oder nur mit unbekanntem Zugriff); `field` und `slot` des ersten Verweises (Level-Script und Scriptdatei sind beide Assets der Art `Script`, `field` `var_conditions` sagt Level-Script; `slot` ist Zeile, Bedingung oder Event) |

Kanten (`GraphEdge { from, to, value, access, field, slot }`): je Verweis mit Wert und Zugriff
`Write`/`ReadWrite` eine Kante Ort → Wert („gesetzt in“), je `Read`/`ReadWrite` eine Kante
Wert → Ort („geprüft in“); `field` und `slot` sagen wo (Befehl und Zeile, Bedingung, Auslöser).
Ein Verweis ohne Wert macht nie einen Wert-Knoten; einer mit Wert und unbekanntem Zugriff (`Unknown`)
macht den Wert-Knoten, aber keine Kante. Eine Variable ohne Verweise hat keine Knoten
und keine Kanten (kein Fehler). Mit einer Geschichte ist `layout` ihr Beifile (`None` ohne Datei
oder wenn es sich nicht lesen lässt: die Prüfung nennt es, der Graph legt sich selbst aus).
Das Label ist der Dateiname der Quelle; den Ortsnamen des Headers setzt die Oberfläche dazu
(wie bei „Verwendet in“ der Flags).

## Prüfung

Der Check des Textprojekts (`textproject::check`, `g4k text check`) liest die
Geschichten als dritte Datenart (`kinds::check_files` → `schema::check_file`): Fehler sind
`textproject.schema` (kein JSON, unbekannter Schlüssel, falscher Typ, ein Schlüssel in zwei
Schritten, ein unlesbares Beifile, ein Dateiname, der weder `NNNN.json` noch `NNNN.layout.json`
ist) und `textproject.id-mismatch` (`id` ohne `STORY_` oder nicht die ID der Nummer im Register);
Warnungen sind `textproject.id-dangling` (ein Registereintrag ohne Datei) und
**`story.dangling-ref`**: eine ID in `progress`, `uses` oder `flags`, die im Projekt nichts nennt,
mit Datei und Zeile des Eintrags
(`data/stories/0000.json`, Zeile von `"TRAINER_NOPE"`:
`story.dangling-ref: steps[0].uses[0].trainer: Unbekannte ID: TRAINER_NOPE.`). Eine Warnung hält
weder Import noch Build auf; der Code steht in der Codeliste von
[text_project.md](text_project.md) „Codes“.

## Diff

„Technik › Änderungen“ und `diff_entity` kennen die Geschichte als eigene Entität:
`DiffSubject::Story { index }` (JSON `{"kind": "story", "index": 0}`), Gruppe „Geschichten“, Label
„Geschichte 0000“. `of_path("data/stories/0000.json")` ist die Geschichte; das Beifile
`0000.layout.json` bleibt eine Projektdatei (`file`), damit das Ziehen von Knoten den Diff der
Bedeutung nicht verrauscht. Beide Seiten sind das JSON der Datei (auf der Platte, in `HEAD`), die
Form ist `fields` mit JSON-Pfaden wie `name`, `notes`, `steps[1].name`,
`steps[1].uses[0].trainer`; eine neue Geschichte ist `new`, eine entfernte `removed`. Gegen die
Basis-ROM wird abgelehnt wie bei einer Datei (`Für einzelne Dateien gibt es keinen Vergleich mit
der Basis-ROM.`); „Gegen gespeichert“ vergleicht die Datei mit sich selbst (Geschichten haben keinen
ungespeicherten Stand). Kein „Auf Original zurücksetzen…“, keine Store-Assets; „Öffnen“ aus der
Liste kommt mit der Fläche der Geschichten.

## Befehle

Die App, die Brücke und MCP (`docs/mcp.md`, Tabelle „Werkzeuge“) haben sechs Befehle, alle
unter dem Sitzungsschloss:

| Befehl | Art | Was er tut |
|---|---|---|
| `list_threads` | lesend | die Fäden (`threads()`) mit den Namen ihrer Header (`header_names`, in der Reihenfolge von `headers`), `text_project`, `scripts_adopted` |
| `thread_graph {var, story?}` | lesend | der Graph (`thread_graph()`); ein Ort-Knoten trägt `header_name` und `nav` (die Script-Zeile, den Header eines Level-Scripts, das Event), ein Wert-Knoten den ganzen Schritt der Geschichte (`step`) |
| `story_list` | lesend | die Geschichten nach Nummer (`number`, `id`, `name`, `progress`, `steps`); eine Datei, die sich nicht lesen lässt, ist eine Zeile mit `problem` (Datei, Zeile, Spalte, Satz), `progress` `null` |
| `story_get {id}` | lesend | Geschichte, Beifile (leer ohne Datei), Nummer und `progress_number` (die Nummer des Fortschritts) |
| `story_create {req}` | schreibend | `create_story` als ein Undo-Schritt |
| `story_update {id, patch}` | schreibend | `update_story` als ein Undo-Schritt |

Ablehnungen haben Codes: `not_text_project` (`Kein Textprojekt: …`), `unknown_id` (eine Geschichte
oder ein Fortschritt, die nichts nennen), `refused` (leerer Name, Schlüssel doppelt, `Nichts zu
ändern.`). Über MCP nimmt der Fortschritt auch eine Nummer oder einen anderen Namen derselben
Nummer; die Datei hält dann den eigenen Namen bzw. den pret-Namen der Nummer. Eine Geschichte, deren
Fortschritt ein Flag ist, hat in 3d.6 keinen Graph aus dem Index (`thread_graph` nimmt eine
Variable); die Oberfläche entscheidet mit `progress_number`, was sie zeigt.

`g4k story` arbeitet ohne Store und ohne Undo (wie `g4k text rename-id`):

- `g4k story list <project>`: eine Zeile je Geschichte, `0000 STORY_TESTFADEN var=VAR_… steps=7
  Testfaden`, bei einer Datei, die sich nicht lesen lässt, `0000 STORY_X problem=<datei>:<zeile>:<spalte>: <satz>`;
- `g4k story get <project> <id>`: JSON `{number, story, layout}`;
- `g4k story create <project> --name … (--var VAR_… | --flag FLAG_…) [--notes …]`: druckt
  `created STORY_… number=0 files=data/ids/stories.json,data/stories/0000.json,data/stories/0000.layout.json ms=…`;
- `g4k story threads <project> [--json]`: eine Zeile je Faden, `0x4030 VAR_PLAYER_STARTER 3 Werte · 2 Orte`
  (der eigene Name vor dem pret-Namen, `story=<id>` angehängt, wenn eine Geschichte ihn beansprucht);
  `--json` alle Fäden mit ihren Verweisen;
- `g4k story graph <project> <VAR_…>`: der Graph als JSON.

## Löschen

Folgt mit Teil B (`story_delete`: ein Dateischritt, der Geschichte, Beifile und Registereintrag
entfernt).
