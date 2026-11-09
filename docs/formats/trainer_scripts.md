# Kampf-Scriptdatei der Trainer (Platin `1114`, HGSS `0953`)

**In English:** Documents the battle script file that all trainers of a game share (script file 1114
in Platinum, 953 in HeartGold/SoulSilver). The script of a trainer is not a file of its own but a
number in this file: script `3000 + ID - 1` starts the battle against trainer `ID`. The document
gives the binary layout (a jump table ended by `0xFD13`) and how Gen4 Studio extends the file when
it creates a trainer. The general script file format is in `script_file.md`. The code is
`crates/g4-kernel/src/scripts.rs` and `crates/g4-kernel/src/data/trainer_create.rs`, with metadata
in `crates/g4-kernel/metadata/trainer_scripts.yaml`; all values are little endian.

Quelle: Spike [docs/research/2026-10-23-3b3/trainer-script-extension-3b3b.md](../research/2026-10-23-3b3/trainer-script-extension-3b3b.md)
§1 und §2 (pret `pokeplatinum` `be36a9db`, `pokeheartgold` `6988158`; gemessen auf Platin DE und HeartGold DE mit
`g4emu`), dazu [trainer-mechanics-3b3b.md](../research/2026-10-23-3b3/trainer-mechanics-3b3b.md) §1 und §4.
Das allgemeine Format einer Scriptdatei (Sprungtabelle, Terminator `0xFD13`) steht in [script_file.md](script_file.md).
Implementierung: `crates/g4-kernel/src/scripts.rs` (`battle_script_file`, `battle_slots`,
`extend_battle_slots_with`), `crates/g4-kernel/src/data/trainer_create.rs` (`create_trainer`), Metadaten
`crates/g4-kernel/metadata/trainer_scripts.yaml`. Alle Werte Little Endian.

Alle Trainer eines Spiels teilen sich eine Scriptdatei. Das Event-Script eines Trainers ist keine eigene Datei, sondern
eine Nummer in dieser Datei: Script `3000 + ID − 1` startet den Kampf gegen Trainer `ID`.

## Binär

Die Datei beginnt wie jede Scriptdatei mit einer Tabelle aus `i32`-Einträgen, beendet durch `u16 0xFD13`. Eintrag *k*
gehört zur Script-Nummer *k* der Datei; sein Wert zählt ab dem Ende des Eintrags: `Ziel = 4·k + 4 + rel`
(pret `ScriptContext_JumpToOffsetID`). Das Spiel prüft die Nummer **nicht** gegen die Zahl der Einträge: eine Nummer
hinter dem letzten Eintrag liest den Terminator und den Code als Offset, das Ergebnis ist undefiniert (das Spiel friert ein).

| | Platin `1114` (Vanilla) | HGSS `0953` (Vanilla) |
|---|---|---|
| Einträge | 929 | 740 |
| Kampf-Code (Ansprechen) | Einträge 0–927 → `0x0E86` | Einträge 0–738 → `0x0B92` |
| „gesehen“-Code | Eintrag 928 → `0x1054` | Eintrag 739 → `0x0EC4` |
| Ende der Tabelle | `0x0E84` (929 × 4), dann `FD 13` | `0x0B90`, dann `FD 13` |

## Script-Nummer, Eintrag, Trainer

- Der Script-Bereich 3000–4999 lädt diese Datei und zieht 3000 ab: Script `3000 + k` ist Eintrag `k`, rotom-Slot
  `#k+1`, der Kampf gegen Trainer `k + 1`. Der Kampf-Code liest die Trainer-ID aus der Script-Nummer (`GetTrainerID`,
  HGSS `GetTrainerNum`).
- Der Bereich 5000–6999 (Doppelkampf-Partner) lädt dieselbe Datei und zieht 5000 ab; Script `5000 + ID − 1` landet im
  **selben** Eintrag wie `3000 + ID − 1`. Eine Erweiterung deckt beide Bereiche. Der Referenzindex zählt beide Bereiche
  als Verweis auf den Trainer (`refs.rs`, `TRAINER_SCRIPTS`, `PARTNER_SCRIPTS`).
- **Sichtkontakt:** Sieht ein Trainer den Spieler, startet das Spiel immer dasselbe Script, das „gesehen“-Script
  (Platin 3928, HGSS 3739); die Nummer steht fest im arm9 (Pt `trainer_encounter.c`, HGSS `std_trainer_approach`).
  Trainer-ID und Art kommen dabei aus dem Objekt, nicht aus der Script-Nummer. Für die Sichtlinie braucht ein neuer
  Trainer deshalb **keinen** eigenen Eintrag.
- **Ansprechen** (auch nach dem Sieg, für den Text „danach“) läuft über den eigenen Eintrag `ID − 1`. Fehlt er, liest das
  Spiel hinter der Tabelle (siehe oben).
- **Die ID des „gesehen“-Slots** (Platin 929, HGSS 740) ist nie ein echter Trainer: ihr Eintrag ist der „gesehen“-Code.
  Spricht man einen Trainer mit dieser ID an, startet der „gesehen“-Code ohne Objekt und das Spiel friert ein (in beiden
  Spielen gemessen).

`trainer_scripts.yaml` hält je Familie die Kampfdatei (`battle_file`, nur Rückfall: der Kernel nimmt die Datei aus der
Script-Tabelle des Spiels) und die feste Nummer des „gesehen“-Scripts (`seen_script`):

```yaml
pt: { battle_file: 1114, seen_script: 3928 }
hgss: { battle_file: 953, seen_script: 3739 }
```

## rotom-Quelle

Nach „Scripts übernehmen“ (`adopt_scripts`) steht die Datei als rotom-Quelle in `expanded/scripts/1114.rotom` (HGSS
`0953`). Der Kopf des Kampf-Scripts nennt alle Kampf-Slots, das „gesehen“-Script hat genau einen eigenen Slot dahinter:

```
script script_1 #[1-928]:        // HGSS: #[1-739]
    PlaySE 1500
    LockAll
    FacePlayer
    GetTrainerID VAR_0x8004       // HGSS: GetTrainerNum VAR_SPECIAL_x8004
    …
script script_929 #929:           // HGSS: script_740 #740
    LockAll
    GetApproachingTrainerType VAR_RESULT   // HGSS: GetTrainerEyeType
    …
```

- Alle Kampf-Slots teilen einen Code. Ein weiterer Slot im Kopf ergibt beim Kompilieren einen weiteren Eintrag der
  Sprungtabelle, der auf denselben Kampf-Code zeigt; der Code selbst bleibt byte-gleich, die Datei wird je Slot 4 Byte länger.
- Ein Slot ohne Script kompiliert mit der Warnung „Script slot #930 is empty; the next available script's pointer will be
  reused“, die Lücke zeigt dann auf den nächsten Code. Der Kernel schreibt deshalb immer lückenlos.
- `save_script` mit der unveränderten Quelle ergibt die Vanilla-Datei byte-gleich.

## Wie `create_trainer` die Datei erweitert

`create_trainer {class, name}` legt den nächsten Trainer an (Platin ab 928, HGSS ab 738). Ablauf (Spike §2.2):

1. **Vorbedingungen.** Ohne übernommene Scripts lehnt der Befehl ab („Erst Scripts übernehmen (Bereich Welt, Panel
   Script)“); der Binärweg des Spikes (Einträge vor `FD 13` einfügen) ist nicht gebaut. Abgelehnt wird auch über der
   Flag-Grenze (Trainer-ID ≥ 1040, Flags `0x550 + ID` bis `0x95F`), bei einem Namen über 7 Zeichen und bei einer
   Namensbank, die nicht zur Trainerzahl passt.
2. **Kopf lesen** (`battle_slots(project, meta)`): das Script mit Slot 1 ist das Kampf-Script; der Slot direkt hinter dessen erstem
   zusammenhängendem Bereich muss ein anderes Script sein und genau `seen_script − 2999` (Platin 929, HGSS 740). Weicht er
   ab, hat ein Hack die Datei umgebaut, und der Befehl lehnt ab. `max` ist der höchste Kampf-Slot.
3. **ID wählen:** die nächste freie ID. Ist sie der „gesehen“-Slot, bekommt diese ID einen gesperrten **Platzhalter**
   (Eigenschaften und Party aus Nullen, Name „-“; der Store sperrt ihn mit Grund, solange er so aussieht) und der neue
   Trainer die ID danach. Platin: erst 928 (ohne Erweiterung), dann 929 Platzhalter und 930. HGSS: 738, 739, dann 740
   Platzhalter und 741.
4. **Kopf erweitern** (`extend_battle_slots_with`), nur wenn die ID über `max` liegt: alle Slots von `max + 1` bis zur ID,
   außer dem „gesehen“-Slot, kommen in die Slot-Liste des Kampf-Scripts (`#[1-928]` → `#[1-928, 930]`, Bereiche
   zusammengefasst). Vorher lehnt der Befehl ab, wenn ein Textarchiv ungespeichert ist, das der Compile neu lädt
   (außer den Bänken für Trainernamen und Kampftexte): „Textarchiv 0412 erst speichern: das Kampf-Script bekommt neue
   Plätze und wird neu kompiliert“. Die Oberfläche speichert ein offenes, ungespeichertes Dokument der Kampfdatei vorher
   und lädt es danach neu.
5. **Schreiben, kompilieren, prüfen:** Quelle schreiben, kompilieren; dann muss die neue Sprungtabelle genau die
   Erweiterung der alten sein: Code nach der Tabelle byte-gleich, jeder neue Eintrag zeigt auf den Kampf-Code (wie
   Eintrag 0), der „gesehen“-Eintrag woanders hin. Jeder Fehler ab dem Schreiben (Schreibfehler, Compile-Fehler, eine
   Warnung oder ein Fehler des Compiles in dieser Datei, das Member unlesbar oder keine Erweiterung) setzt alte Quelle
   und altes Member zurück (die alte Quelle wird neu kompiliert) und meldet den ursprünglichen Fehler. Lässt sich das
   nicht beides zurücksetzen, meldet der Befehl `partial` mit dem, was bleibt („battle script file 1114 stays changed“).
6. **Daten anlegen,** in einem Changeset: `trainerProperties/NNNN` (20 Byte) und `trainerParty/NNNN` lückenlos bis zur ID
   (mit dem Platzhalter), der Name im Archiv 618 (HGSS 729) an Index = ID, `trtblofs` bis zur ID aufgefüllt (Lücken 0;
   HGSS Vanilla hat nur 735 Einträge für 738 Trainer). Kampftexte schreibt danach `set_trainer_texts` (Einträge ans Ende
   von `trtbl`, Nachrichten ans Ende von 617 bzw. 728).
7. **Antwort:** `{id, placeholder, script_extended}`, zum Beispiel Platin `{id: 930, placeholder: 929, script_extended: true}`.

Anlegen ist **nicht rückgängig**: die Member und die Erweiterung liegen sofort auf der Platte. Der Verlaufsschritt
„Trainer N angelegt“ ist eine Undo-Grenze; ⌘Z davor meldet „Trainer N anlegen lässt sich nicht rückgängig machen“ und
lässt alles stehen, auch die Schritte vor dem Anlegen (sie bleiben bis zum nächsten Öffnen des Projekts hinter der Grenze).

## Event eines Trainers

Ein Objekt-Event wird zum Trainer mit Script `3000 + ID − 1`, `trainer_type = 1` (Sichtlinie an) und der Sichtweite in
`data[0]` (Feld `sight`, Standard 3). „In der Welt platzieren“ und `add_event` mit `patch` setzen alle drei in einem
Verlaufsschritt; die Blickrichtung bestimmt, wohin der Trainer sieht.

## Belegt im Spiel

Die Szenarien `pt-trainer-begegnung.yaml` und `hg-trainer-begegnung.yaml` ([phase-3b3-gate.md](../phase-3b3-gate.md),
Teil B) legen in beiden Spielen Trainer hinter dem Platzhalter an (Platin 930, HGSS 741), sprechen sie an und gewinnen den
Kampf; der erste Trainer (928 bzw. 738) kämpft über die Sichtlinie. Dass der zweite Kampf durch Ansprechen beginnt, zeigt
die Anordnung im Szenario, nicht `expect-battle` (das liest nur Art und Level und unterscheidet Sicht und Ansprechen
nicht): Mia sieht vom Weg weg (Platin nach Süden, HGSS nach Norden), der Spieler kommt auf ihrer Seite an, stellt sich
neben sie, dreht sich zu ihr (`press RICHTUNG:8`) und drückt A.

## Offen

- Der Wächter aus drei Befehlen am Anfang des „gesehen“-Scripts (Spike §2.3), der auch die ID des „gesehen“-Slots nutzbar
  machen würde, ist nicht gebaut.
- Diamant/Perl: keine geprüfte Zeile in `trainer_scripts.yaml`; `create_trainer` lehnt ab.
