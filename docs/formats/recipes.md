# Rezepte (`recipes.yaml`, `metadata/recipes/`)

Eine Tatsache über Gen4 Studio selbst, keine Spielformat-Quelle: Rezepte sind Aufgaben in den Worten
des Spiels („Gebiet anlegen“, „Raum anlegen“, „Neue Figur“, „Tür verbinden“ …). Implementierung:
`crates/g4-kernel/src/recipes.rs` (Verteiler `plan`, `apply`, `list`, `is_irreversible`), je Rezept ein
Modul unter `crates/g4-kernel/src/recipes/`, die gemeinsamen Typen in `recipes/types.rs`; Vorgaben in
`crates/g4-kernel/metadata/recipes.yaml`, Scripts in `crates/g4-kernel/metadata/recipes/`. „Welt
leeren“ beschreibt `templates.md`.

## Rahmen

- `plan(store, input)` läuft alle Prüfungen der Anwendung und antwortet einen `Draft` (Zellen,
  Kacheln, Events mit Matrix und Aussehen, Nachrichten mit Archiv, Sätze `notes`, `irreversible`),
  ohne etwas zu schreiben.
- `apply(store, input)` legt erst neue Member an (nicht rückgängig, `Outcome.created`) und macht dann
  jede Änderung als **einen** Undo-Schritt. Dateien außerhalb des Stores (Script-Quellen, Member,
  rotoms Compile-Zustand, Textarchive, die ein Compile schreibt) gehen als Dateischritte
  (`FileStep`, Art `Recipe`) in denselben Schritt (`Store::squash_since`).
- `Outcome.next` nennt die Rezepte, die danach Sinn ergeben (`NextStep`): frei, wenn `list()` das
  Rezept hat oder der Schritt kein Rezept ist (`edit_dialog` öffnet den Dialog-Editor); sonst
  gesperrt mit Grund „Kommt mit 3d.7“ (`play_here` „Kommt mit 3d.9“, `adjust_terrain` „Kommt mit
  Phase 6“).

## Vorgaben (`recipes.yaml`)

| Feld | Platin (und Diamant/Perl) | HeartGold/SoulSilver | Inhalt |
|---|---|---|---|
| `default_look.<familie>.figure` | 6 | 317 | `graphics_id` einer neuen Figur, wenn die Eingabe keines nennt; die Werte der Gate-Szenarien `pt-flag.yaml` / `hg-flag.yaml` (Geschenk-Figur der Trainer-Szenarien) |

## Neue Figur (`new_figure`)

Modul `recipes/new_figure.rs`. Stellt ein Objekt-Event auf eine Kachel eines Headers, hängt ein Script
an die Scriptdatei des Headers und die Nachricht der Figur an sein Textarchiv.

**Eingabe** (`NewFigureInput`): `header`, `x`, `z` (absolute Kacheln der Matrix des Headers),
`graphics_id` (`null`: Vorgabe der Familie), `movement_type`, `dir`, `text` (chatot-Textform wie im
Texte-Bereich), `hidden_flag` (`null` oder 0: immer sichtbar; ein freies Flag sucht der Aufrufer mit
`free_flag`).

**Prüfungen** (in `plan` und `apply`, vor dem ersten Schreiben, in dieser Reihenfolge): der Header
existiert; Scripts übernommen (sonst „Erst Scripts übernehmen (Bereich Welt, Panel Script)“); `text`
nicht leer; das Flag liegt im Spielstand; die Scriptdatei des Headers ist eine rotom-Quelle
(`NNNN.rotom`); Eventdatei und Textarchiv sind schreibbar; die Kachel liegt in einer Zelle des Headers
(`doors::outside_map`) und kein Objekt derselben Datei steht darauf („auf x, z steht schon Objekt k“,
k der Index in der Datei); der neue Slot liegt unter 2000 (ab 2000 beginnen die gemeinsamen Scripts);
die Datei nennt `figur_<local_id>` noch nicht; die Nummer der Nachricht passt in den Nachrichtenbefehl
(Platin `Message`, HeartGold `NPCMsg`: u8 in rotoms Datenbank, also höchstens 255).

**Script je Familie** (`metadata/recipes/figure_pt.rotom`, `figure_hgss.rotom`; Diamant/Perl wie
Platin), Platzhalter `{n}` (Slot = höchster deklarierter Slot + 1; 1 in einer Datei ohne Slot), `{local_id}`,
`{message}`:

| Platin | HeartGold/SoulSilver |
|---|---|
| `script figur_{local_id} #{n}:` | `script figur_{local_id} #{n}:` |
| `LockAll`, `FacePlayer` | `LockAll`, `FacePlayer` |
| `Message {message}` | `NPCMsg {message}` |
| `WaitButton`, `CloseMessage` | `WaitButton`, `CloseMsg` |
| `ReleaseAll`, `End` | `ReleaseAll`, `End` |

**Nachricht nummeriert, kein Literal.** rotom lowert `Message "…"` über uxies `find_or_add_message`:
steht der Text schon im Archiv, nimmt es **dessen** Index, sonst überschreibt es den ersten leeren Slot
oder hängt an. Zwei Figuren mit gleichem Text teilten sich also eine Nachricht, und ein geändertes
Literal bekäme beim nächsten Compile einen neuen Index, die alte Nachricht bliebe verwaist. Darum hängt
das Rezept die Nachricht über den Store an das Archiv `text_archive_id` des Headers (Index = bisherige
Zahl der Nachrichten, Kennung `msg_NNNNN`) und schreibt die Nummer ins Script, wie es Vanilla für fast
alle Nachrichten tut. Der Dialog-Editor ändert danach die Nachricht im Archiv, ohne die Quelle
anzufassen.

**Undo-Schritt** „Figur „<Text, gekürzt auf 20 Zeichen>“ aufgestellt“:

1. eine Store-Transaktion: die Nachricht an das Archiv, das Objekt (`local_id` die kleinste freie der
   Datei, `script` = Slot, `trainer_type` 0, `data` 0, `y` 0) an die Eventdatei. Das Archiv ist ein
   Teil des Stores (gespeichert wird es mit „Speichern“), kein Dateischritt;
2. das Script angehängt und kompiliert (`append_source`); ein Compile-Fehler stellt Quelle und
   Compile-Zustand wieder her und nimmt die Transaktion zurück, ohne Redo;
3. Dateischritte: die Scriptdatei (Quelle und Member), jedes Textarchiv der Header mit dieser
   Scriptdatei, das der Compile schrieb (JSON und Member), rotoms Compile-Zustand als ganze Datei;
   schrieb der Compile ein Text-Literal in das Archiv der Figur, wird alles zurückgenommen und
   abgelehnt. Die JSON eines Archivs, das DSPRE nicht ausgepackt hat, schreibt der Compile aus dem
   Member (gleicher Inhalt); das Rezept löscht sie wieder, so dass nach „Speichern“ ⌘Z nicht an
   einer Datei scheitert, die das Speichern mit der neuen Nachricht schrieb;
4. alles ab der Marke ein Schritt. ⌘Z stellt Quelle, Member, Compile-Zustand, Archiv und Eventdatei
   byte-gleich her, ⌘⇧Z wieder den Stand danach; eine Quelle, die jemand zwischendurch änderte, lässt
   ⌘Z ablehnen.

**Bericht** (`NewFigureReport`): `event_file`, `object` (Index in der Datei), `local_id`,
`script_file`, `slot`, `archive`, `message`. **Danach:** `edit_dialog` „Dialog bearbeiten“ (frei),
`story_join` „Zu einer Geschichte“ („Kommt mit 3d.7“).

## Tür verbinden (`connect_doors`)

Modul `recipes/connect_doors.rs`, die Tür-Helfer teilt es mit „Raum anlegen“ (`recipes/doors.rs`:
`warp_count`, `warp_at`, `header_event_file`, `outside_map`, `fires`, `set_door`, `local`, `warp` und
`write_warps`, das die Warp-Änderungen eines Türpaars als ein `edit_event` je Eventdatei schreibt).
Verbindet zwei Orte über ein Warp-Paar: Tür A führt zu Tür B und, mit Rückweg, Tür B zu Tür A.

**Eingabe** (`ConnectDoorsInput`): `a` und `b` (`DoorEnd`: `header` und **genau eins** von `warp`, dem
Index einer vorhandenen Tür in der Eventdatei des Headers, und `tile` `{x, z}`, absoluten Kacheln der
Matrix des Headers, auf die eine neue Tür kommt; andere Schlüssel lehnt das Rezept ab), `back`
(Standard `true`: B führt zurück zu A) und `door_tiles` (Standard `true`: die Kachel einer neuen Tür
wird zur Türkachel, wenn das Spiel dort keinen Warp nähme, wie bei „Raum anlegen“).

**Prüfungen** (in `plan` und `apply`, vor dem ersten Schreiben, in dieser Reihenfolge): je Ende, A
vor B, genau eins von `warp` und `tile` („Tür A braucht eine Tür oder eine Kachel“), der Header
existiert, die Eventdatei hat die Tür („Tür A: Header h hat keine Tür k (Eventdatei f hat n)“) bzw.
die Kachel liegt in einer Zelle des Headers (`outside_map`) und kein Warp derselben Datei liegt darauf
(„auf x, z liegt schon Tür k; wähle sie als Tür“), die Karte einer neuen Türkachel ist schreibbar;
dann zwei verschiedene Türen („Tür A und Tür B sind dieselbe Tür“: derselbe Warp oder dieselbe Kachel
in derselben Eventdatei, auch über zwei Header, die sich die Datei teilen); ohne Rückweg muss B eine
vorhandene Tür sein (eine neue Tür B führte nirgendwohin); die Eventdatei von A ist schreibbar und
mit Rückweg die von B.

**Indizes:** eine vorhandene Tür behält ihren Index, eine neue bekommt die bisherige Zahl der Warps
ihrer Datei; zwei neue Türen in derselben Datei bekommen zwei aufeinanderfolgende Indizes, A zuerst.

**Entwurf:** je Ende, das sich ändert (A, mit Rückweg auch B), ein `Warp`-Event auf der Matrix seines
Headers an der Kachel der Tür, `target` das andere Ende (Header und Index); je neuer Türkachel eine
`tile`; `notes`: „Tür A: {Ort} Tür k → Tür B: {Ort} Tür j, und zurück.“ (ohne Rückweg mit Punkt statt
„, und zurück“), je neue Tür „Neue Tür k bei x, z von {Ort}.“, je umgezielte Tür, wohin sie bisher
führte, je Türkachel „Die Kachel bei x, z von {Ort} wird zur Tür.“. Nichts ist unumkehrbar. Der Ort ist
der Ortsname des Headers, sonst sein interner Name, sonst „Header n“ (wie `headerLabel` der App); haben
beide Header denselben Ortsnamen (Haus und Stadt), steht der interne Name in Klammern dahinter.

**Undo-Schritt** „Tür „{Ort A}“ ↔ „{Ort B}“ verbunden“: eine Store-Transaktion mit einem `edit_event`
je Eventdatei (gleiche Datei: ein Aufruf; neue Türen angehängt, vorhandene über `dest_header_id` und
`dest_warp_id` umgezielt; ohne Rückweg nur A) und den Türkacheln (`set_door`). Alles sind Teile des
Stores, kein Dateischritt: ⌘Z stellt beide Eventdateien und die Karte byte-gleich her, auch wenn
zwischen Anwenden und ⌘Z gespeichert wurde. Eine Eingabe, die nichts ändert (die Türen zeigen schon
aufeinander), legt keinen Schritt an.

**Bericht** (`ConnectDoorsReport`): `a`, `b` (`DoorReport`: `header`, `event_file`, `warp`, `created`,
`door_tile` mit `map`, `x`, `y` der Karte oder `null`) und `back`. **Danach:** `play_here` „Hier spielen“
(„Kommt mit 3d.9“).

## Neuer Trainer hier (`new_trainer_here`)

Folgt.
