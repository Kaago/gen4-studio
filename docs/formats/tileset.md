# Tileset (Kacheln eines Kartentextur-Pakets, Phasen 6.3a und 6.3b)

Eine Tatsache über Gen4 Studio selbst, keine Spielformat-Quelle: Ein Tileset sagt je Textur eines
Kartentextur-Pakets (`map_tileset` der Gebietsdaten, Member von `mapTextures`), wie sie als Kachel
einer [Welt-Karte](world_map.md) wirkt: Rolle auf dem Boden, Vorgaben der Permissions,
Höhenverhalten, Wandtextur, Fußabdruck eines Teilmodells. Quelle: [Spezifikation Phase 6](../superpowers/specs/2026-11-05-phase-6-kartenerstellung-design.md)
§3.2, [Gutachten C](../research/2026-11-05-phase-6/gutachten-c-malen-import.md) §3.3 und §5.2,
die Messung von Task 0 in [task0-6-3a-vorgaben.md](../research/2026-11-05-phase-6/task0-6-3a-vorgaben.md),
Plan [2026-11-05-phase-6-3a-weltformat.md](../superpowers/plans/2026-11-05-phase-6-3a-weltformat.md)
(„Entscheidungen“). Implementierung: `crates/g4-kernel/src/world/tileset.rs`, die Karten eines
Pakets `world/places.rs`, der Zwischenspeicher `Store::tileset`. Texturen liest es über die
bestehenden Leser (`g4_formats::nsbtx`, `nsbmd_header::material_texture_names`).

**Nie im Repo.** Ein Tileset, das aus einer ROM abgeleitet ist, enthält die Texturnamen und
Vorgaben des Spiels: es entsteht nur im Projekt des Nutzers (`data/tilesets/`) oder im Speicher,
nie als Fixture, Beispiel oder Testdatei im Repository (keine Vanilla-Tileset-Metadaten, keine
Texturnamen aus einer ROM; `scripts/check-no-rom-data.sh` fängt die ROM-Dateien selbst ab, diese
Regel gilt darüber hinaus). Das einzige Tileset im Repo ist das synthetische aus Code
(unten); alle Beispiele hier stammen daraus.

## Datei

`data/tilesets/NNNN.json`, der Stamm ist die Nummer des Pakets (vier Dezimalziffern). Kanonisches
JSON wie jede Datei des Textprojekts, Schlüssel in der Reihenfolge der Tabelle, unbekannte
Schlüssel abgelehnt, Zahlen als JSON-Zahlen (die Oberfläche zeigt sie hex). Die Ordnerart
`tilesets` hat keine IDs und kein Register; Export und Import lassen den Ordner liegen, nur der
Check liest ihn (Dateiart `FILE_KINDS` in `textproject/kinds.rs`, Schemaschlüssel `tilesets` im
Manifest, siehe [text_project.md](text_project.md)).

| Schlüssel | Typ | Inhalt |
|---|---|---|
| `version` | Zahl | immer `1` (`TILESET_VERSION`) |
| `package` | Zahl | das Paket; gleich dem Stamm der Datei |
| `tiles` | Liste | eine Kachel je Textur des Pakets: Kachel `i` ist Textur `i` |
| `edge_sets` | Liste | **reserviert** für die Kantensätze von 6.4b; muss leer sein, leer weggelassen |

Je Kachel:

| Schlüssel | Typ | Inhalt |
|---|---|---|
| `texture` | Text | Name der Textur im Paket (höchstens 16 Byte) |
| `texture_index` | Zahl | Index der Textur im Paket |
| `palette` | Text, weggelassen | Name der Palette (fehlt: keine, dekodiert mit Palette 0) |
| `size` | `[b, h]` | Breite und Höhe in Texeln |
| `role` | `ground` \| `wall` \| `ramp` \| `water` \| `grass` \| `piece` | was die Kachel auf der Karte ist |
| `kind` | Zahl | Vorgabe des Verhaltens (`kind`) einer Zelle mit dieser Kachel |
| `collision` | Zahl | Vorgabe des Kollisionsbytes (Bit 7 sperrt) |
| `height` | `flat` \| `ramp` \| `wall` | wie eine Zelle mit der Kachel ihre Höhe nimmt |
| `sound` | Zahl, weggelassen bei 0 | nur HeartGold/SoulSilver: Schrittgeräusch, Bits 0 bis 6 des Kollisionsbytes |
| `wall` | Zahl, weggelassen | die Kachel (Rolle `wall`), deren Textur die Wände unter einer Zelle mit dieser Kachel tragen |
| `footprint` | `[b, t]`, weggelassen | Breite und Tiefe in Zellen; genau die Kacheln der Rolle `piece` haben einen |
| `model` | Text, weggelassen | **reserviert** für das Beifile des Teilmodells (6.5b, `NNNN.<name>.glb`); muss fehlen |
| `origin` | `vanilla` \| `unclear` \| `synthetic` \| `user` | woher die Werte kommen: abgeleitet, nicht entscheidbar, synthetisch, vom Nutzer |

Die reservierten Felder liest serde, damit eine spätere Datei ihr Problem nennt statt eines
unbekannten Schlüssels; der Check lehnt sie ab.

## Prüfungen

Der Check des Textprojekts liest jede Datei unter `data/tilesets/` (`tileset::check_folder`), jede
Meldung `textproject.schema` mit Datei, Zeile und dem JSON-Pfad in der Datei:

- ein Dateiname, der keine Paketnummer ist: „Der Dateiname ist keine Paketnummer (`NNNN.json`); die
  Datei gehört zu keinem Kartentextur-Paket.“
- eine Datei, die serde nicht liest (Pfad und Grund von serde);
- `version`: „Version 2 kennt diese Fassung nicht (1).“; `package`: „Paket 0007 passt nicht zum
  Dateinamen 0006.json.“;
- `edge_sets`: „Kantensätze kommen mit 6.4b.“; `tiles[i].model`: „Teilmodell-Dateien kommen mit
  6.5b.“;
- `tiles[i].wall` jenseits der Kacheln („Kachel 0x0120 gibt es im Tileset 0006 nicht (94
  Kacheln).“) oder ohne Rolle `wall` („Kachel 0x0002 ist keine Wand (Rolle `water`).“);
- `tiles[i].footprint` auf einer Kachel, die kein Teilmodell ist („einen Fußabdruck haben nur
  Teilmodelle (`piece`).“), ein Teilmodell ohne („ein Teilmodell (`piece`) braucht einen Fußabdruck
  (`footprint`).“);
- ein Texturname über 16 Byte; `sound` ≠ 0 außerhalb von HeartGold/SoulSilver („Geräusche gibt es
  nur in HeartGold/SoulSilver.“) oder über Bit 6.

Eine höhere Version unter `schemas.tilesets` im Manifest ist eine Warnung, der Ordner bleibt
ungeprüft, wie er ist.

## Welches Tileset gilt

`Store::tileset(package)` gibt die Tileset-Datei, wenn es sie gibt, sonst **`textures_only`** des
Pakets: je Textur eine Kachel mit Name, Index, Größe und Palette, Rolle `ground`, `kind 0`,
`collision 0`, `height flat`, `origin unclear`, ohne eine Karte zu lesen. Das reicht für Vernetzen,
Szene und die Bereichsprüfung der Kacheln (`world.schema`). Der Store hält das Ergebnis je Paket
zwischen und liest neu, wenn sich Größe oder Änderungszeit der Datei bzw. des Paket-Members ändern
oder die Datei kommt oder geht.

Die Ableitung mit Vorgaben (`derive_tileset`) läuft nur in `tileset_get` ohne Datei (Antwort
`stored: false`) und in `tileset_save`. Angelegt wird die Datei **nur** mit `tileset_save`, geändert
mit `tileset_edit` (6.3b, unten); „Welt anlegen“ schreibt keine. `tileset_save(package)` leitet ab und schreibt
`data/tilesets/NNNN.json` als einen Dateischritt („Tileset 0006 abgeleitet“, Undo entfernt die Datei
wieder) und antwortet mit `{package, file, tiles, vanilla, unclear, change}`. Abgelehnt: im
DSPRE-Layout („Boden malen gibt es im Textprojekt (Technik › Textprojekt).“), wenn die Datei schon
da ist („data/tilesets/0006.json gibt es schon.“), ein Paket, das es nicht gibt
(„Kartentextur-Paket 0200 gibt es nicht (74 Pakete).“), eines, das sich nicht lesen lässt
(„Kartentextur-Paket 0006 lässt sich nicht lesen: …“). `tileset_get` leitet in jedem Layout ab.

## Kacheln ändern (`tileset_edit`, 6.3b)

`tileset_edit {req: {package, changes: [{tile, role?, kind?, collision?, sound?, wall?}]}}` ändert
Kacheln der vorhandenen Datei `data/tilesets/NNNN.json` als **einen** Dateischritt wie
`tileset_save` (`FileStepKind::Reset`; Undo schreibt die Datei, wie sie war), Label „Tileset 0006
geändert (2 Kacheln)“ („(1 Kachel)“). Ein fehlender Schlüssel lässt den Wert, `"wall": null`
entfernt die Wandkachel. Gezählt und mit `origin: user` versehen werden nur Kacheln, deren Werte
sich wirklich ändern; eine Rolle setzt kein `kind` mit (Gras mit abgeleiteter Begegnung braucht
`role: "grass"` und `kind: 2` getrennt). Antwort `{package, tiles, change}`.

Abgelehnt (nichts geschrieben, Verlauf unverändert): im DSPRE-Layout („Boden malen gibt es im
Textprojekt (Technik › Textprojekt).“), ohne Datei („Erst `tileset_save` für Paket 0006.“), eine
Kachel jenseits des Tilesets („changes[0].tile: Kachel 0x0120 gibt es im Tileset 0006 nicht (94
Kacheln).“), ein Aufruf, der nichts ändert („Nichts zu ändern.“), ein Ergebnis mit einem Problem der
Prüfungen oben (das erste, mit dem Pfad der Datei: „data/tilesets/0006.json: tiles[0].wall: …“),
eine Datei, die sich nicht lesen lässt.

Danach backt der Store die sauberen geladenen Welt-Karten dieses Pakets an Ort und Stelle neu
(`Store::forget_world_bakes`, ihr Verlauf bleibt; das zwischengespeicherte Tileset des Pakets fällt
vorher weg), schmutzige backt das nächste Speichern; dasselbe nach Undo und Redo eines Dateischritts
von `tileset_save` oder `tileset_edit`. Der Build-Schritt `world` backt die Karten des Pakets beim
nächsten Build, denn die Tileset-Datei gehört zu ihrem Eingabe-Hash ([build_state.md](build_state.md)).
Der Tileset-Editor von 6.4a baut auf diesem Befehl.

## Palette beim Backen (6.3b)

Das Backen einer Welt-Karte bindet jede Textur mit der Palette, die das Tileset für sie nennt, wenn
das Paket diese Palette hat (`world::bake::world_pack_textures`), sonst wie `bake::pack_textures`
(die gleichnamige Palette, sonst `<Textur>_pl`, sonst die einzige des Pakets). Nennen mehrere Kacheln dieselbe Textur,
entscheidet die erste Kachel in Tileset-Reihenfolge, die eine Palette nennt; eine andere Palette
einer späteren Kachel bleibt ungenutzt. Grund: was die Szene zeigt, zeigt das Spiel; Platins
`allpeak` bindet in jedem Vanilla-Modell die Palette seines Index (`apeak`), die `pack_textures`
nicht findet.

## Ableitung

Kachel `i` der Ableitung ist Textur `i` des Pakets; Name, Index, Größe und Palette wie
`textures_only`, die Vorgaben aus den Vanilla-Permissions der Karten, die das Paket zeigen.

**Karten eines Pakets** (`places::maps_of_packages`, ein Durchgang über Header und Matrizen): ein
Header zeigt die Karten seiner Matrix in den Zellen, die seine sind (jede Zelle einer Matrix ohne
Header-Tabelle); seine Gebietsdaten nennen das Paket. Eine Karte kann so zu mehreren Paketen
gehören. Eine Karte, die nicht lädt, zählt nicht; eine, deren Modell sich nicht lesen lässt, bindet
keine Textur.

**Regel K1 (Mehrheit), `UNIQUE_SHARE = 0,6`** (Task 0, [task0-6-3a-vorgaben.md](../research/2026-11-05-phase-6/task0-6-3a-vorgaben.md)
§8 „Festlegung“, `DEFAULT_RULE = VoteRule::K1`): für Textur T sind M_T die Karten des Pakets, deren
Modell T bindet (`material_texture_names`). Jede Zelle einer solchen Karte stimmt mit dem Gewicht
`1 / |Namen der Karte|` für ihr `kind`: `votes(k) = Σ_{m ∈ M_T} Zellen_m(k) / |Namen_m|`. `k*` ist
das `kind` mit den meisten Stimmen (Gleichstand: das kleinere); entschieden, wenn
`votes(k*) / Σ votes ≥ 0,6`. Die zweite gemessene Regel K2 (Abhebung gegen alle Zellen des Pakets)
liegt im Code, ist aber nicht gewählt.

| Ergebnis | `kind` | `collision` | `sound` (HGSS) | `role` | `origin` |
|---|---|---|---|---|---|
| entschieden | `k*` | 0 | häufigstes `collision & 0x7F` der gewichteten Zellen mit `k*` (Gleichstand: das kleinere) | aus `k*` | `vanilla` |
| unklar oder ohne Karte | 0 | 0 | 0 | `ground` | `unclear` |

**Kollision immer 0**, abweichend vom Plan (dort `0x80`, wenn mindestens die Hälfte der gewichteten
Zellen mit `k*` Bit 7 trägt): die Planregel stimmte in Task 0 seltener als „überall 0“ (44,5 % und
64,0 % gegen 78,7 % und 77,2 % der Texturen mit Wahrheit, Platin DE und HeartGold DE), und eine
blockierte Vorgabe machte gemalten Boden unbegehbar. Das Geräusch von HeartGold/SoulSilver folgt
dem Plan.

**Rolle aus `kind`:** die Klasse des Verhaltens (`tiles::tile_behavior(family, kind).class`):
`grass` → `grass`, `water` → `water`, sonst `ground`; `height` immer `flat`. `wall`, `ramp` und
`piece` leitet nichts ab: unklare und abgeleitete Kacheln sind Boden und werden im Tileset-Editor
(6.4a) umgestellt. Mit K1 sind abgeleitete `grass`- und `water`-Kacheln selten: die Zählung ist
karten-weit (wo eine Textur auf der Karte liegt, sieht erst der Display-List-Codec von 6.1), und auf
den meisten Karten überwiegt `kind 0` (Task 0, §8 und §10).

**Palette** einer Kachel: die Palette `<Textur>_pl` (der Name vor dem Vergleich auf 16 Byte
gekürzt), sonst die gleichnamige, sonst die Palette mit dem Index der Textur, sonst keine
(`palette` fehlt). Eine Textur im Format 7 (direkte Farbe) hat keine. Die dritte Stufe hat Task 0
ergänzt: ohne sie blieben 20,4 % (Platin) und 9,1 % (HeartGold) der Texturen ohne Palette.

**Zahlen** (Gegenprobe im Kernel, Task 0 §12): Platin DE 74 Pakete mit 3 130 Kacheln, davon 1 765
`vanilla`; HeartGold DE und US 106 Pakete mit 3 659 Kacheln, davon 2 612 `vanilla`. Die Zahl der
Kacheln je Paket ist immer die Zahl seiner Texturen.

## Texturen für die Szene

`get_tileset_texture(package, texture)` dekodiert die Textur mit der Palette ihrer Kachel
(`tileset_texture`; ohne `palette` Palette 0) zu RGBA. `null` kommt, wenn das Paket oder die Textur
fehlt, sich das Paket nicht lesen lässt (das synthetische Testprojekt), die Kachel eine Palette
nennt, die das Paket nicht hat, oder das Format keinen Decoder hat: Texturen im **Format 5**
(4 × 4 komprimiert) dekodiert der Kernel noch nicht, die Szene zeigt dort den Platzhalter der Rolle
([world_map.md](world_map.md), „Vorschau in der Szene“).

## Synthetisches Tileset

`Tileset::synthetic()` (nur mit `test-support`), Paket `SYNTHETIC_PACKAGE = 0`, alle Kacheln
`origin: synthetic`, `kind`/`collision` 0, wo nicht anders genannt:

| Kachel | `texture` | `size` | `role` | Besonderes |
|---|---|---|---|---|
| 0 | `syn_boden` | 16 × 16 | `ground` | `wall: 3` |
| 1 | `syn_gras` | 32 × 32 | `grass` | `kind 0x02` |
| 2 | `syn_wasser` | 16 × 16 | `water` | `kind 0x15` |
| 3 | `syn_wand` | 16 × 16 | `wall` | `height wall`, `collision 0x80` |
| 4 | `syn_rampe` | 16 × 16 | `ramp` | `height ramp` |
| 5 | `syn_stueck` | 16 × 16 | `piece` | `footprint [2, 2]` |

Kachel 0 als Datei (Ausschnitt von `data/tilesets/0000.json`, die übrigen fünf Kacheln mit „…“
weggelassen):

```json
{
  "version": 1,
  "package": 0,
  "tiles": [
    {
      "texture": "syn_boden",
      "texture_index": 0,
      "size": [
        16,
        16
      ],
      "role": "ground",
      "kind": 0,
      "collision": 0,
      "height": "flat",
      "wall": 3,
      "origin": "synthetic"
    },
    "…"
  ]
}
```
