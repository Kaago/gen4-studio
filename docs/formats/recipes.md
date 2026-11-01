# Rezepte (`recipes.yaml`, `metadata/recipes/`)

Eine Tatsache über Gen4 Studio selbst, keine Spielformat-Quelle: Rezepte sind Aufgaben in den Worten
des Spiels („Gebiet anlegen“, „Raum anlegen“, „Neue Figur“ …). Implementierung:
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
   abgelehnt;
4. alles ab der Marke ein Schritt. ⌘Z stellt Quelle, Member, Compile-Zustand, Archiv und Eventdatei
   byte-gleich her, ⌘⇧Z wieder den Stand danach; eine Quelle, die jemand zwischendurch änderte, lässt
   ⌘Z ablehnen.

**Bericht** (`NewFigureReport`): `event_file`, `object` (Index in der Datei), `local_id`,
`script_file`, `slot`, `archive`, `message`. **Danach:** `edit_dialog` „Dialog bearbeiten“ (frei),
`story_join` „Zu einer Geschichte“ („Kommt mit 3d.7“).

## Tür verbinden (`connect_doors`)

Folgt.

## Neuer Trainer hier (`new_trainer_here`)

Folgt.
