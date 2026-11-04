# Flags und Variablen

Eine Tatsache über Gen4 Studio selbst, keine Spielformat-Quelle: wie das Studio die Flags und
Variablen der Gen-4-Spiele benennt, in Bereiche teilt und eigene Namen im Textprojekt ablegt. Ein
Flag ist eine Nummer ab 1 (0 heißt „kein Flag“), eine Variable eine Nummer ab `0x4000` (darunter
gibt es keine; ab `0x8000` sind es Script-lokale Register). Implementierung:
`crates/g4-kernel/src/flags.rs` (`FlagKind`), `flags/names.rs` (`names`, `NameTable`,
`name_is_unused`), `flags/ranges.rs` (`Area`, `ranges`, `area_of`, `in_save`), `flags/entities.rs`
(`FlagEntity`, `own_names`, `name_flag`), `flags/pool.rs` (`pool`, `free_flag`, `reserved`),
`textproject/kinds.rs` (`DataKind`, `DATA_KINDS`).

Begründung und Messungen: [Gutachten B](../research/2026-10-30-phase-3d/gutachten-b-flags-variablen-live.md)
(§1 Namen und Bereiche, §2 eigene Namen) und
[Gutachten C](../research/2026-10-30-phase-3d/gutachten-c-loeschen-sprites-schnitt.md) §6.6
(Art-Beschreibung).

## Namen aus pret

Die Namen stammen aus den Abschnitten `flags` und `vars` der gebündelten Befehlsdatenbanken
(`crates/g4-kernel/metadata/scrcmd/{platinum,hgss}_v2.json`, `{ "<NAME>": { "id": n } }`), je
Familie. Diamant/Perl hat dort keine Flag-Namen; das Studio führt für `dp` keine pret-Namen.

| | Platin | HeartGold/SoulSilver |
|---|---|---|
| Flags mit Namen (verschiedene Nummern) | 4 097 (4 098 Einträge) | 2 244 (2 252 Einträge) |
| Variablen mit Namen | 307 | 386 |

Eine Nummer kann mehrere Namen haben (`VARS_START`, `MAP_LOCAL_VARS_START` und
`VAR_MAP_LOCAL_0x00` sind alle `0x4000`). **Hauptname** ist der erste Name in der Reihenfolge der
Datei, der mit `FLAG_` bzw. `VAR_` beginnt, nicht auf `_START`, `_END` oder `_BASE` endet und nicht
die Hex-Form `PREFIX_0x…` hat; gibt es keinen, der erste mit dem Präfix; sonst der erste überhaupt.
So heißt `0x4000` `VAR_MAP_LOCAL_0x00`, `0x800C` `VAR_RESULT`, Platin `0x997`
`FLAG_VILLA_FURNITURE_TABLE`, HeartGold `0x550` `TRAINER_FLAG_BASE` (sein einziger Name). Die
übrigen Namen sind Aliasse: sie lösen zur Nummer auf (Suche, `NameTable::number`), angezeigt wird
der Hauptname.

Ein pret-Name gilt als „unbenutzt“, wenn er `UNUSED`, `DUMMY` oder `UNK` enthält oder fehlt
(`name_is_unused`). Das engt „frei“ nur ein, nie aus: ob eine Nummer frei ist, entscheidet der
Nutzungsindex.

## Bereiche

`crates/g4-kernel/metadata/flag_ranges.yaml`, je Familie, alle Grenzen einschließlich. Eine Nummer
gehört zum engsten Bereich, der sie enthält (die versteckten Items liegen im allgemeinen Bereich
und gewinnen); eine Nummer in keinem Bereich ist „außerhalb“ (`Area::Outside`), auch Flag 0.

| Bereich (`Area`) | Wort | Platin | HeartGold/SoulSilver |
|---|---|---|---|
| Flags `map_local` | kartenlokal | `0x001`–`0x040` | `0x001`–`0x03F` |
| Flags `general` | allgemein | `0x041`–`0x54F` | `0x040`–`0x54F` |
| Flags `hidden_items` | versteckte Items | `0x2DA`–`0x3F5` | `0x320`–`0x406` |
| Flags `trainer` | Trainer | `0x550`–`0x95F` | `0x550`–`0x95F` |
| Flags `system` | System | `0x960`–`0xA9F` | `0x960`–`0xA9F` |
| Flags `daily` | Tagesflags | `0xAA0`–`0xB5F` | `0xAA0`–`0xB5F` |
| Flags `temp` | temporär | – | `0x4000`–`0x403F` |
| Variablen `map_local` | kartenlokal | `0x4000`–`0x401F` | `0x4000`–`0x401F` |
| Variablen `object_gfx` | Objektgrafik | `0x4020`–`0x402F` | `0x4020`–`0x402F` |
| Variablen `general` | allgemein | `0x4030`–`0x411F` | `0x4030`–`0x416F` |
| Variablen `script_local` | Script-lokal | `0x8000`–`0x8010` | `0x8000`–`0x800D` |

**Spielstand.** `flags_end` und `vars_end` sind exklusiv: der Spielstand hält die Flags
`1..flags_end` (`0xB60`, 2 912 Flags, 364 Byte, beide Familien) und die Variablen
`0x4000..vars_end` (Platin `0x4120`, 288 Variablen; HeartGold `0x4170`, 368). Script-lokale
Register und die temporären Flags von HeartGold gehören nicht dazu (`in_save`). Die Basis der
versteckten Items ist der Anfang von `hidden_items` (Platin `0x2DA`, HeartGold `0x320`), das
Trainer-Flag von Trainer `n` ist `0x550 + n` (`trainer_flag`, bis `0x95F`), neue Nummern kommen aus
`general` (`pool_range`). Diamant/Perl: wie Platin (nicht an einer ROM geprüft).

## Eigene Namen

Nur im Textprojekt (aktuelle Schema-Version). Ein eigener Name ist ein **zweiter** Name derselben
Nummer: der pret-Name bleibt gültig, Quellen mit dem pret-Namen kompilieren weiter zu denselben
Bytes.

**Register.** `data/ids/flags.json` und `data/ids/vars.json`, ID → Nummer wie die Register der
anderen Arten (kanonisch, Schlüssel sortiert, keine Nummer doppelt, keine fremde Ersatz-ID). Der
Export schreibt sie (leer `{}`), getauft wird nie: ein Eintrag entsteht nur durch „Als eigenes Flag
benennen…“ (`name_flag`).

```json
{
  "FLAG_HAT_MEDAILLE": 148
}
```

**Entität.** Jeder eigene Name hat eine Datei `data/flags/NNNN.json` bzw. `data/vars/NNNN.json`
(Nummer dezimal, mindestens vier Stellen: `0148`, `16492`), mit `id` und `description` (nur, wenn
nicht leer); weitere Schlüssel lehnt das Schema ab. Eine Nummer ohne eigenen Namen hat keine Datei
und keine Beschreibung. Ein Standardwert fehlt mit Absicht: das Spiel kennt keinen (gesetzt wird
im Neues-Spiel-Script).

```json
{
  "id": "FLAG_HAT_MEDAILLE",
  "description": "Orden erhalten"
}
```

**ID-Dateien für rotom.** `.rotom/command_database/.g4-ids-flags.json` und `…-vars.json` enthalten
nur die Registereinträge, nie die pret-Namen (die kennt rotom aus seiner Datenbank):
`{"148": "FLAG_HAT_MEDAILLE"}`. Die Schatten-Regel von „IDs in Scripts“
([text_project.md](text_project.md)) gilt unverändert. Ein eigener Name muss mit `FLAG_` bzw.
`VAR_` beginnen, sonst erkennt rotom ihn in `if NAME then` nicht als Flag.

**Auflösen.** `Symbols::with_family` und der `Resolver` eines Projekts lösen eine ID einer Flag-
oder Variablen-Nummer erst im Register (eigener Name), dann in den pret-Namen der Familie (jeder
Alias), dann als Ersatz-ID `FLAG_NNNN` auf; die ID einer Nummer ist der eigene Name, sonst der
pret-Hauptname. Im DSPRE-Layout gibt es nur die pret-Namen; `Resolver::fixed()` kennt keine.

**Benennen (`name_flag`).** `NameRequest { kind, number, id?, description? }` benennt eine Nummer
oder setzt ihre Beschreibung, als **ein** Dateischritt (Register und Entität) der Art
`FileStepKind::IdRename` mit den Folgen eines Umbenennens: unter dem Schloss des Abgleichs, die
vorher abgeglichenen Dateien neu aufgezeichnet, die ID-Dateien neu geschrieben. Bezeichnung im
Verlauf: „Flag benannt: FLAG_X (0x0094)“, „Variable benannt: VAR_X (0x406C)“, „Beschreibung
geändert: FLAG_X“. Die Beschreibung wird ohne Leerraum an den Enden gespeichert. Abgelehnt wird,
bevor etwas geschrieben ist (`Refused`, wörtlich):

| Fall | Satz |
|---|---|
| kein Textprojekt der aktuellen Version | `Kein Textprojekt: IDs gibt es erst nach „Als Textprojekt einrichten…“.` (ältere/neuere Version: der Satz des Manifests) |
| Nummer außerhalb des Spielstands | `Nummer 0x0FFF liegt außerhalb des Spielstands (Flags 0x0001 bis 0x0B5F).` (Variablen: `(Variablen 0x4000 bis 0x411F)`) |
| ID ohne Präfix der Art oder mit anderen Zeichen | `Die neue ID muss mit FLAG_ beginnen und darf danach nur A–Z, 0–9 und _ enthalten (höchstens 64 Zeichen).` |
| ID ist ein eigener Name einer anderen Nummer | `FLAG_X gibt es schon (Nummer 148).` |
| ID ist die Ersatz-ID einer anderen Nummer | `FLAG_0149 ist die Ersatz-ID der Nummer 149.` |
| ID ist der pret-Name einer anderen Nummer | `FLAG_UNUSED_0x0095 ist der pret-Name der Nummer 149.` |
| rotom kennt die ID mit anderem Wert | `FLAG_X kennt rotom schon als Nummer 7.` |
| Nummer hat schon einen anderen eigenen Namen | `Die Nummer hat schon einen eigenen Namen; ändern über „Umbenennen…“.` |
| keine ID für eine Nummer ohne eigenen Namen | `Die Nummer hat noch keinen eigenen Namen; dafür braucht es eine ID.` |
| nichts zu tun | `Nichts zu ändern.` |

Der pret-Name der eigenen Nummer ist als eigener Name erlaubt, wenn er die Form einer ID hat
(`FLAG_VILLA_FURNITURE_TABLE` für `0x997`; `FLAG_UNUSED_0x0094` hat ein kleines `x` und ist keine).
Der pret-Name einer anderen Nummer wird vor der Form geprüft, damit der Satz sagt, wem er gehört. Ein eigener Name wird über „ID
umbenennen“ (`rename_id`) umbenannt; das zieht Register, die `id` der Entität, die Script-Quellen
und die ID-Datei nach. Ein pret-Name lässt sich nicht umbenennen (`Unbekannte ID: …`), und kein
Umbenennen darf auf den pret-Namen einer anderen Nummer führen. Eine Nummer ohne eigenen Namen
bekommt über „Als eigenes Flag benennen…“ einen; „Umbenennen…“ legt keinen an. Einen eigenen Namen
entfernt seit 3d.4 `delete_entity` mit `flag` bzw. `var` (Datei und Registereintrag als ein
Undo-Schritt, `delete/files.rs`).

**Check.** Der Check des Textprojekts prüft die Dateien (`kinds::check_files`): eine Datei, die
nicht `{id, description?}` ist, ein Dateiname ohne Nummer und eine Nummer außerhalb des Spielstands
(Datei oder Registereintrag) sind `textproject.schema`; eine `id`, die nicht der Registereintrag
ihrer Nummer ist, `textproject.id-mismatch`; ein Registereintrag ohne Datei ist
`textproject.id-dangling` (Warnung, mit eigenem Satz: „`FLAG_Z` (Nummer 300) hat keine Datei
data/flags/0300.json; jeder eigene Name hat eine.“). Ein Registereintrag eines Flags gilt sonst nie
als verwaist: jede Nummer des Spielstands gibt es.

## Art-Beschreibung

Flags und Variablen sind die ersten Arten des Textprojekts ohne Gegenstück in der ROM. Sie sind
als Daten beschrieben, nicht als Zweig des Enums der Arten (`textproject/kinds.rs`):

```rust
pub struct DataKind {
    pub folder: &'static str,          // Ordner unter data/, Schlüssel in `schemas`
    pub symbol: SymbolKind,            // Art der IDs, Register data/ids/<folder>.json
    pub schema_version: u32,           // Version des Entitätsformats
    pub ref_fields: &'static [&'static str], // Felder, die andere Entitäten nennen
    pub delete: DeleteRule,            // RemoveFile: Datei (und Registereintrag) entfernen
}
```

| `folder` | `symbol` | `schema_version` | `ref_fields` | `delete` |
|---|---|---|---|---|
| `flags` | `Flag` | 1 | – | `remove_file` |
| `vars` | `Var` | 1 | – | `remove_file` |

`arts::is_known_folder` kennt die Ordner (kein `textproject.unknown-art`), Export und Import fassen
sie nicht an (kein Member), der Abgleich nimmt sie als Dateien ohne Member. `manifest_for` schreibt
`schemas` mit jeder Art und ihrer Version (`{"flags": 1, "vars": 1}`) und behält fremde Schlüssel;
der Hinweis `textproject.reserved` gilt nur noch für fremde Schlüssel in `schemas` (und für
`plugins`). Nennt `schemas` für eine Art eine höhere Version, als diese Fassung kennt, warnt der
Check (`textproject.schema` am Schlüssel) und lässt die Dateien der Art ungelesen; abgelehnt wird
nichts. „Datei entfernen“ ist deklariert, ausgeführt wird es ab 3d.4.

## Nutzungsindex

Der Verweisindex (`crates/g4-kernel/src/data/refs.rs`) kennt Flags und Variablen als Ziele
`Target::Flag(n)` und `Target::Var(n)`. Jeder Verweis trägt einen Zugriff `access`: `read`,
`write`, `read_write` oder `unknown` (`Access`, im DTO `RefDto.access`). Alle übrigen Ziele
(Arten, Attacken, Items, Trainer, Klassen, Begegnungsdateien) werden nur gelesen und haben
`read`. Begründung: [Gutachten B](../research/2026-10-30-phase-3d/gutachten-b-flags-variablen-live.md)
§1.1 und §3.2.

### Scripts

Quelle sind die Script-Quellen `expanded/scripts/NNNN.rotom` (nur mit übernommenen Scripts), wie
für Trainer und Items. Implementierung: `crates/g4-kernel/src/data/refs/flagvars.rs`
(`FlagVarTable`, `script_flagvar_refs`), die Regeln in
`crates/g4-kernel/metadata/script_flagvars.yaml`. `field` eines Verweises ist rotoms
Befehlsname (auch für einen DSPRE-Namen oder einen Alias des Befehls in der Quelle), bei einer
Bedingung `if` (auch `else if`), `while` oder `match`; `slot` ist die Zeile des Arguments.

**Welche Parameter.** Ein Parameter eines Befehls ist

| Rolle | wenn |
|---|---|
| Trainer-Flag | die Liste `trainer_flags` ihn nennt (`SetTrainerFlag`, `ClearTrainerFlag`, `CheckTrainerFlag`) |
| Flag | die Liste `flags` ihn nennt (gleich welchen Typs), sonst sein Typ `flag` ist oder sein Name in `name_rule.flag` steht |
| Variable | die Liste `vars` ihn nennt, sonst sein Typ `var` oder `flex` ist oder sein Name in `name_rule.var` steht |

`name_rule` sind rotoms Parameternamen für Flag (4) und Variable (22), wörtlich aus
`src/decompiler/ir_to_source.rs:28-58` (rotom v0.7.0, `param_semantics`), ohne Rücksicht auf
Groß- und Kleinschreibung. Die Namensregel gilt nur für Parameter mit 16-Bit-Typ (`u16`, `var`,
`flex`, `flag`): HeartGolds `u8`-Parameter `flag` (`SafariZoneAction`) und `shiny_flag`
(`WildBattle`) sind ein Modus und ein Schalter. Befehle, deren Parameter nur je Variante eines
Modus existieren (Platin `MysteryGiftGive`, `CallTVBroadcast`, `CallTVInterview`), tragen
nichts bei.

**Zugriff.** Ein Eintrag einer Liste nennt ihn selbst. Ohne Eintrag ist ein Flag
`flag_default` (`read`: nur `SetFlag`/`ClearFlag` und ihre Varianten über eine Variable
schreiben, und die stehen in der Liste). Eine Variable ohne Eintrag:

- Platin (`var_default: database`): das `access` des Parameters in der gebündelten Datenbank,
  übersetzt mit `access_words` (`read` → `read`, `must_write`/`may_write` → `write`,
  `read_must_write`/`read_may_write` → `read_write`, `none` → `unknown`); ohne `access`
  `unknown`. Makros (`SetVar`, `CompareVar`, `GoToIfEq`, …) tragen in der Datenbank kein
  `access`; ihr Zugriff kommt aus den Befehlen, zu denen sie sich entfalten, und steht in der
  Liste `vars` (`SetVar`: `destVarID` `write`, `valueOrVarID` `read`).
- HeartGold (`var_default: borrow`): die Datenbank hat keine Angaben. Der Kernel borgt den
  Zugriff des gleichnamigen Platin-Befehls, Parameter an derselben Stelle, wenn er dort eine
  Variable ist; sonst `unknown`. Ausnahmen (Liste `vars`): `SetVar` schreibt, `CopyVar` und
  `SetOrCopyVar` schreiben das Ziel und lesen die Quelle, `AddVar`/`SubVar` lesen und schreiben.

**Welcher Wert zählt.**

| Parameter | Wert | ergibt |
|---|---|---|
| Variable | `0x4000`–`0x7FFF` | `Var(n)` |
| Variable | ab `0x8000` | nur gezählt (`ScriptCounts.script_local`) |
| Variable | unter `0x4000` | nichts (ein Wert) |
| Flag | `1`–`0x3FFF` | `Flag(n)` |
| Flag | `0` | nichts (kein Flag) |
| Flag | ab `0x4000`, als Flag-Name geschrieben | `Flag(n)`, wenn der Befehl das Flag als Zahl nimmt (Parametertyp `u16` oder `flag`, etwa `SetFlag`) |
| Flag | ab `0x4000`, sonst | nur gezählt (`ScriptCounts.dynamic`, ein Flag aus einer Variable) |
| Trainer-Flag | Trainer `n` ab 1 | `Flag(0x550 + n)`, solange es im Bereich `trainer` liegt; der Verweis auf `Trainer(n)` bleibt |
| Trainer-Flag | ab `0x4000` | nur gezählt (`dynamic`) |

Ein Argument zählt, wenn es eine Zahl ist oder ein Bezeichner, den der Build auflöst: ein Alias
der Datei, die IDs des Textprojekts (auch ein eigener Name wie `FLAG_HAT_MEDAILLE`), rotoms
eigene Namen (`FLAG_UNUSED_0x0094`, `VAR_RESULT`). Ein Ausdruck, ein unbekannter Name, ein
Alias ohne Wert und ein Name, den die Datei selbst deklariert, zählen nicht.

**Flags ab `0x4000`.** Ein Flag-Argument ab `0x4000` ist meist ein Flag aus einer Variable.
Es gibt aber Flags mit solchen Nummern: HeartGolds temporäre Flags `FLAG_TEMP_x4000` bis
`FLAG_TEMP_x403F` (Bereich `temp`) und Platins `FLAG_GALACTIC_HQ_CONTROL_ROOM_STATE` (`0x410D`).
Darum ist ein solches Argument ein `Flag(n)` mit dem Zugriff des Parameters, wenn es ein Name
ist, der als Flag auflöst: ein pret-Flag-Name der Familie, ein eigener Flag-Name oder ein Alias
der Datei für einen davon. Eine nackte Zahl (`SetFlag 0x4001`) und ein Variablenname
(`SetFlag VAR_RESULT`) ab `0x4000` bleiben nur gezählt (`dynamic`). Das gilt nur für Parameter,
die der Befehl als Zahl nimmt: ein Parameter vom Typ `var` oder `flex` (HeartGold `SetFlagVar`,
`ClearFlagVar`, `CheckFlagVar`, Platin `SetFlagFromVar`, `CheckFlagFromVar`) ist ab `0x4000`
die Variable, die das Flag hält, gleich welchen Namen eine dekompilierte Quelle ihm gibt
(Vanilla-HeartGold Script 0952 schreibt `SetFlagVar FLAG_TEMP_x4001`, nachdem es
`VAR_TEMP_x4001` auf 653 gesetzt hat). Trainer-Flags bleiben wie oben. Ein nackter `FLAG_…` in
einer Bedingung folgt der Regel von `SetFlag` (rotom lowert ihn zu `CheckFlag`).

**Bedingungen.** `if`, `else if` und `while` lesen, was sie prüfen: `!`, `&&` und `||`
verbinden Bedingungen; ein nackter Bezeichner, dessen Name mit `FLAG_` beginnt und auflöst, ist
ein Flag (`read`, wie rotom ihn zu `CheckFlag` lowert); ein Operand eines Vergleichs (und ein
nackter Operand, den rotom zu `CompareVarValue x, 0` lowert) ab `0x4000` ist eine Variable
(`read`), ab `0x8000` nur gezählt. Der Kopf von `match` ist ein solcher Operand. Ein Aufruf in
einer Bedingung (`CheckItem(ITEM_POTION, 1) == 1`) ist ein Befehl wie jeder andere; seine
Ergebnisvariable ist kein Verweis. Quelle: rotom `src/compiler/ir/lowerer.rs:1300-1356,
1792-1807`.

**Der Wert.** Ein Verweis auf eine Variable trägt den konstanten Wert, auf den er sie setzt oder
mit dem er sie vergleicht (`Ref.value`, im DTO `FlagRefDto.value`; Grundlage der Fäden einer
Geschichte, gemessen in [Gutachten A](../research/2026-10-30-phase-3d/gutachten-a-geschichten-erzaehlen.md)
§2.2):

| Stelle | Wert |
|---|---|
| Befehl mit Eintrag in `values` der Familie | das Argument des dort genannten Parameters, am Verweis des einen anderen Variablen-Parameters des Befehls |
| Vergleich in `if`/`while` (`==`, `!=`, `<`, `<=`, `>`, `>=`, auch durch `!`, `&&`, `\|\|`) | der andere Operand, auf welcher Seite auch immer (`if 5 >= VAR_X` → 5) |
| Auslöser (Events) | sein `value` |
| Level-Script | `var_conditions[i].value` |

`values` (`metadata/script_flagvars.yaml`): Platin `SetVar` (`valueOrVarID`), `SetVarFromValue`
und `CompareVarToValue` (`value`), `CompareVar`, `GoToIf…` und `CallIf…` (`valueOrVarID`);
HeartGold `SetVar` und `CompareVarToValue` (`val`, HeartGold hat kein `SetVarFromValue`, sein
`SetVar` nimmt den Wert) und `SetOrCopyVar` (`flexible_value`). `AddVar`/`SubVar` addieren und
tragen keinen Wert. Ein Wert zählt, wenn er zu einer Zahl unter `0x4000` auflöst (Zahl, Alias der
Datei, Name, den der Build kennt); kein Wert (`None`) bleibt

- ein Argument oder Operand ab `0x4000`: eine Variable (`SetVar VAR_X, VAR_Y` gibt beiden
  Verweisen keinen Wert, `CompareVarToValue VAR_X, VAR_RESULT` keinen; dort ist `VAR_RESULT` die
  Zahl `0x800C`, die der Befehl als Wert nimmt, und wird nicht gezählt), ebenso bei Auslöser und
  Level-Script ein `value` ab `0x4000`;
- was nicht auflöst (unbekannter Name, Alias ohne Wert, Ausdruck, Aufruf wie
  `VAR_X == CheckItem(…)`);
- der Kopf von `match` und ein nackter Operand (`if VAR_X then`);
- jedes Flag und jedes Trainer-Flag.

Die Prüfung `values_name_existing_commands` hält `values` ehrlich: der Befehl steht in der
Datenbank, der Parameter an genau einer Stelle, und der Befehl hat außer ihm genau einen
Variablen-Parameter, der den Wert trägt.

**Nicht indexiert, nur gezählt.** Variablen ab `0x8000` (Script-lokale Register) und Flags oder
Trainer aus einer Variable (ein Flag ab `0x4000` nur, wenn es nicht als Flag-Name geschrieben
ist, s. o.). Die Zähler je Datei summiert der Index: `RefIndex::script_counts`
liefert `ScriptCounts { script_local, dynamic }`.

**Nicht lesbar.** Eine Quelle, die nicht parst, trägt nichts bei, weder Verweise noch Zähler;
rotoms Parser ist fehlertolerant, ein halb geschriebenes Script liefert die Teile, die er
versteht.

Eine Prüfung (`script_flagvars_name_existing_commands_and_params`) hält die Datei ehrlich: jeder
Eintrag nennt einen Befehl und Parameter der Datenbank seiner Familie, `access_words` deckt jedes
`access` der Datenbank.

### Events

Quelle sind die Eventdateien (`AssetKind::Event`), wie für die Trainer hinter Objekten. Je Datei
stehen zuerst die Objekte (je Objekt Sichtbarkeits-Flag, Trainer, Trainer-Flag), dann die
Hintergrund-Events, dann die Auslöser; `slot` ist der Index des Events in seiner Liste.

| Feld | Event | Ziel | Zugriff |
|---|---|---|---|
| `hidden_flag` | Objekt mit `hidden_flag ≠ 0` | `Flag(hidden_flag)` | `read` (ist das Flag gesetzt, bleibt das Objekt verborgen) |
| `script` | Objekt mit Script `3000`–`4999` (Trainer `n` = Script − 2999) oder `5000`–`6999` (Partner, Script − 4999) | `Trainer(n)` wie bisher **und** `Flag(0x550 + n)`, solange es im Bereich `trainer` liegt | `read` bzw. `read_write` (die Engine prüft das Flag vor dem Kampf und setzt es nach dem Sieg) |
| `hidden_item` | Hintergrund-Event mit `event_type == 2` und Script `8000 + n` | `Flag(hidden_items.start + n)`: Platin `0x2DA + n`, HeartGold `0x320 + n`; über die Länge des Bereichs hinaus (284 bzw. 231) kein Verweis | `read_write` (geprüft, ob das Item liegt, gesetzt beim Aufheben) |
| `var` | Auslöser mit `var` `0x4000`–`0x7FFF` | `Var(var)` mit dem Wert `value` | `read` (der Auslöser läuft, wenn die Variable `value` hat) |

Ein Sichtbarkeits-Flag 0 und eine Auslöser-Variable 0 sind kein Verweis (kein Flag, keine
Variable); der Wert bleibt speicherbar. Begründung: Gutachten B §1.3 und §3.2 Punkt 3. Ob jedes
Hintergrund-Event mit einem Script `8000 + n` den Typ 2 hat, prüft der gated Test
`vanilla_usage_matches_a_recount_of_the_sources` (`hidden_item_other_type=0` in Platin DE und
HeartGold DE).

### Level-Scripts

`expanded/scripts/NNNN.json` (rotoms `LevelScript`, nur mit übernommenen Scripts) hat dieselbe
Quell-ID wie eine `.rotom`-Datei dieser Nummer (`AssetKind::Script`, eine Nummer, eine Quelle;
gibt es beide, gilt die `.rotom` wie bei `scripts::source_kind`). Jede Bedingung
`var_conditions[i]` mit `var` `0x4000`–`0x7FFF` ist `Var(var)`, `read`, Feld `var_conditions`,
Slot `i`, mit dem Wert `value` der Bedingung. `var` 0 beendet die Tabelle wie in der Engine: weder dieser Eintrag noch die folgenden
zählen. Eine Variable ab `0x8000` wird nur gezählt (`ScriptCounts.script_local`). Eine JSON, die
nicht liest, trägt nichts bei.

### Aktualisieren

`RefIndex::refresh` fragt den Store jedes Mal nach seinen Änderungen (billig), stattet die
Script-Quellen (`.rotom` und `.json`, das teure Stück) aber höchstens alle 250 ms
(`REFRESH_SCRIPTS_EVERY`): liegt das letzte Statten weniger lange zurück, bleibt der Script-Teil
aus. `rebuild_full` und `RefIndex::refresh_scripts_now` setzen die Sperre; `refresh_scripts_now`
stattet unabhängig von ihr (nach dem Speichern einer Quelle, in Tests). Folge: speichert der
Script-Editor eine Quelle und fragt die Oberfläche innerhalb von 250 ms nach dem letzten Statten,
zeigt sie noch den alten Stand, bis eine spätere Abfrage oder `refresh_scripts_now` sie liest.
Ändern sich die Dateien, nach denen Bezeichner auflösen, liest der Index alle `.rotom`-Quellen neu;
die Level-Scripts nennen keine Bezeichner und werden nur bei geändertem Stempel neu gelesen.

### Nutzung

`RefIndex::usage(store, kind) -> BTreeMap<u16, Usage { read, write, unknown, sources }>`: jede
Nummer, zu der der Index einen Verweis kennt, in einem Durchlauf über den Index, ohne Cache.
`read_write` zählt bei `read` und `write`, `sources` sind die verschiedenen Quellen.
`RefIndex::script_counts` liefert dazu die Zähler `ScriptCounts { script_local, dynamic }`.

## Frei

`flags::pool(project, usage, kind) -> Pool { kind, range, free, total, reserved }`
(`crates/g4-kernel/src/flags/pool.rs`). Eine Nummer ist **frei**, wenn

- sie im allgemeinen Bereich liegt (`area_of == General`; die versteckten Items darin sind ihr
  eigener Bereich, kartenlokale, Trainer-, System- und Tagesflags sind nie frei),
- der Nutzungsindex keinen Verweis auf sie kennt (jeder Zugriff zählt),
- sie keinen eigenen Namen hat (`own_names`),
- ihr pret-Name fehlt oder unbenutzt heißt (`name_is_unused`: `UNUSED`, `DUMMY`, `UNK`),
- und kein Rezept sie reserviert.

Der Namens-Teil engt nur ein: `FLAG_UNUSED_0x0096` an einem Objekt (Platin) ist nicht frei, und
ein vanilla benannter Fortschritts-Flag ohne Script-Verweis (die Engine setzt ihn) ist es nie.
`free` ist aufsteigend; `free_flag`/`free_var` geben die kleinste Nummer, `None` bei leerem Pool
(`pool_empty_text`: „Kein freies Flag im allgemeinen Bereich.“ / „Keine freie Variable im
allgemeinen Bereich.“). `total` ist die Zahl der Nummern des allgemeinen Bereichs ohne die
versteckten Items: Platin 1 011 Flags und 240 Variablen, HeartGold 1 065 und 320.
Gemessen auf den deutschen Vanilla-Spielen mit übernommenen Scripts (gated `tests/flags.rs`):
Platin 101 freie Flags und 20 freie Variablen (kleinste `0x0042`, `0x4031`), HeartGold 154 und 81
(`0x0040`, `0x4034`).

**Reservierung.** Die Variable, mit der die Vorlage „Leere Region“ ihr Startscript als gelaufen
markiert (`done_var` in `metadata/templates.yaml`: Platin `VAR_UNUSED_0x406C`, HeartGold
`VAR_UNK_4031`), ist keine Referenz (eine `Ref` braucht eine Quelle mit Ort), sondern eine
Reservierung: `flags::reserved(family)` nennt sie mit dem Wort „Vorlage „Leere Region““,
`pool` nimmt sie aus „frei“ und führt sie in `Pool.reserved`.

## RAM-Lage

Wo die Flags und Variablen des Spielstands im laufenden Spiel liegen (Gutachten B §4.2, gemessen
mit dem `g4emu`-Kern durch Vergleich des Spielstands mit dem RAM nach `continue`). Dieselben Werte
stehen zweimal: im Kernel in `crates/g4-kernel/metadata/save_layout.yaml`
(`flags::save_layout::save_block(code)`) und für `g4emu` in `scripts/g4emu/g4emu/games.yaml` unter
`save_block`; der Kernel-Test `g4emu_games_yaml_carries_the_same_block` verlangt Gleichheit.

| Spielcode | Zeiger (`pointer`) | Kopf (`header`) | Variablen (`vars`, `vars_count`) | Flags (`flags`, `flags_bytes`) |
|---|---|---|---|---|
| CPUD Platin DE | `u32 @ 0x02101EE0` | `0x14` | `0xDAC`, 288 × u16 (`0x4000`–`0x411F`) | `0xFEC`, 364 Byte (`0x000`–`0xB5F`) |
| IPKD HeartGold DE | `u32 @ 0x02111860` | `0x10` | `0xDE4`, 368 × u16 (`0x4000`–`0x416F`) | `0x10C4`, 364 Byte (`0x000`–`0xB5F`) |
| IPKE HeartGold US | `u32 @ 0x02111880` | `0x10` | wie IPKD | wie IPKD |

- **Anfang:** `start = (u32 @ pointer) − 0x02000000 + header` (Index in den 4 MiB Hauptspeicher).
  Der Zeiger ist derselbe u32 wie `location.pointer`: er zeigt auf den allgemeinen Block des
  Spielstands, der Ort des Spielers liegt im selben Block (`+0x1294` bzw. `+0x1244`).
- **Flag `n`:** Bit `n % 8` von Byte `start + flags + n / 8`; gültig `0 ≤ n < flags_bytes × 8`
  (2 912 Flags, `flags_end` `0xB60` aus `flag_ranges.yaml`).
- **Variable `v`:** u16 little-endian an `start + vars + 2 × (v − 0x4000)`; gültig
  `0x4000 ≤ v < 0x4000 + vars_count` (`vars_end` aus `flag_ranges.yaml`). Script-lokale
  Variablen ab `0x8000` liegen nicht im Spielstand.
- **Zeiger je Sitzung:** Innerhalb einer Sitzung bleibt der Zeigerwert gleich (gemessen über einen
  Kartenwechsel und 3 000 Bilder). Zwischen zwei Starts wandert der Block um bis zu `0xB4` Byte
  (Heap, trotz gleicher Eingaben): immer über den Zeiger lesen, nie über eine feste Adresse. Vor
  dem Laden des Spielstands ist der Zeiger ungültig (Platin gültig ab Bild 107, HeartGold ab 193).
- **Lesepuffer:** Eine zweite, exakte Kopie des Spielstands liegt bei `0x0236D6xx` (Platin) bzw.
  `0x023775xx` (HeartGold). Das ist der Lesepuffer der Karte; er ist nicht lebendig und darf
  weder gelesen noch geschrieben werden.
- **Schreiben** wirkt, sobald das Spiel den Wert liest. Sichtbarkeits-Flags (`hidden_flag` eines
  Objekts) wirken erst beim nächsten Laden der Karte (Gutachten B §4.3).

`g4emu` liest und schreibt die Werte mit den Schritten `flag N`, `set-flag N 0|1`, `var N`,
`set-var N W`, `expect-flag N W` und `expect-var N W` (`scripts/g4emu/g4emu/games.py`:
`block_start`, `read_flag`, `write_flag`, `read_var`, `write_var`); siehe
[agent-gate.md](../agent-gate.md), „Werkzeuge“.
