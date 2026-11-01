# Textarchiv (Member und chatot-JSON)

Quelle: chatot 0.8.1 (`decode.rs`, `encode.rs`, `charmap.json`; das Programm, das DSPRE für
Text-JSON mitliefert) und der DSPRE-Report zum Textformat; es sind Tatsachen über das
Dateiformat, keine Übernahme von Programmcode. Nachgeprüft am 22.10.2026: alle Member der
drei Projekte (Platin DE 724, HeartGold DE 829, HeartGold US 829) dekodieren, gehen durch
die JSON und werden mit `write_archive` zu **identischen Bytes** zurückkodiert (gated Test
`text::tests::project_members_round_trip_byte_identical`). Die Implementierung steht in
`crates/g4-kernel/src/text.rs`. Werte sind Little Endian.

Ein Textarchiv liegt im Projekt als Member `unpacked/textArchives/NNNN` (ein Member des Text-NARC)
und, als editierbare Quelle, als `expanded/textArchives/NNNN.json`.
Die Verschlüsselung ist für Diamant/Perl, Platin und HeartGold/SoulSilver gleich.

## Member

```
u16 count                      Anzahl der Nachrichten
u16 key                        Schlüssel dieses Archivs (je Archiv verschieden)
{ u32 offset, u32 length } * count   Eintragstabelle, verschlüsselt
u16 codes[...]                 die Nachrichten, je Nachricht verschlüsselt
```

`offset` ist ein Dateioffset in Bytes (der erste Eintrag liegt bei `4 + 8 * count`, die
folgenden unmittelbar dahinter), `length` zählt `u16`-Codes inklusive des Terminators.

### Eintragstabelle

Eintrag `i` (ab 0) wird mit einem 32-Bit-Schlüssel ver-XOR-t, beide Felder mit demselben:

```
k = (key * 0x2FD * (i + 1)) & 0xFFFF
k = k | (k << 16)
offset = offset_file ^ k
length = length_file ^ k
```

Beispiel Platin DE, Archiv 0000: `count = 1404`, `key = 0x58E6`; Eintrag 0 entschlüsselt
zu `offset = 11236 = 4 + 8 * 1404` und `length = 2`.

### Nachrichten

Nachricht `i` (ab 0) wird code-weise mit einem 16-Bit-Schlüssel ver-XOR-t, der mit dem
Index `n = i + 1` beginnt und je Code wächst:

```
k0 = (n * 0x91BD3) & 0xFFFF
code_plain = code_file ^ k
k = (k + 0x493D) & 0xFFFF         nach jedem Code
```

Jede Nachricht endet mit dem Code `0xFFFF`. Die Nachricht selbst darf Nullcodes enthalten
(`\x0000`), ein Code nach dem Terminator wird beim Lesen ignoriert.

### Codes

| Code | Bedeutung | Textform |
|---|---|---|
| `0xFFFF` | Ende der Nachricht | (nicht im Text) |
| `0xE000` | Zeilenumbruch | `\n` (Backslash und `n`) |
| `0x25BC` | Box leeren, auf Tastendruck | `\r` |
| `0x25BD` | Scrollen | `\f` |
| `0xFFFE` | Steuercode: `0xFFFE, command, count, params[count]` | `{NAME, special, p1, ...}` |
| `0xF100` | Trainername (Zeichenfolge in einem eigenen Format) | `{TRAINER_NAME:...}` |
| übrige Codes der Zeichentabelle | Zeichen (2876 Einträge, ein Satz für alle Spiele und Sprachen; `0x0000` ist `\x0000`) | das Zeichen |
| `[PK]`, `[MN]` und weitere | Sonderglyphen | Alias in eckigen Klammern |
| sonst | unbekannter Code | `\xNNNN` (vier Hex-Ziffern) |

Steuercodes: das Command-Wort ist `0xFF00 | special` oder `<command> | special`; chatot
kennt `STRVAR_1` (`0x0100`), `STRVAR_3`, `STRVAR_4`, `STRVAR_6`, `STRVAR_34`, `YESNO`
(`0x0200`), `PAUSE`, `WAIT`, `CURSOR_X`, `CURSOR_Y`, `ALN_CENTER`, `ALN_RIGHT`, `COLOR`
(`0xFF00`), `SIZE` (`0xFF01`) und einige unbenannte (`UNK_602`, `UNK_603`, `UNK_207`,
`UNK_208`, `UNK_FF02`); das Byte `special` (niedriges Byte des Command-Worts) steht in der
Textform immer als erster Parameter, zum Beispiel `{STRVAR_1, 8, 0, 0}` und `{COLOR, 0, 5}`.
Ein Befehl ohne Eintrag wird als `0xNNNN` geschrieben und beim Kodieren mit Warnung
übernommen. Beim Kodieren erzeugen unbekannte Zeichen, Aliase und Escapes eine Warnung
und den Code `0x0000`; das Kodieren bricht nicht ab.

## JSON (`expanded/textArchives/NNNN.json`)

```json
{
  "key": 22758,
  "messages": [
    { "id": "msg_00000", "de_DE": "Hallo" },
    { "id": "msg_00001", "de_DE": ["Zwei\\n", "Zeilen"], "en_US": "Two lines" }
  ]
}
```

- `key` ist der Schlüssel aus dem Member-Header (`u16`); er wird mitgeführt, damit ein
  Archiv nach dem Speichern byte-identisch bleibt.
- `messages` ist die geordnete Liste, der Index in der Liste ist der Index im Member.
  `id` ist beliebig (uxie schreibt `msg_00012`, chatot `msg_<archiv>_00012`) und wird
  verbatim erhalten.
- Je Sprache ein Schlüssel (`en_US`, `de_DE`, `fr_FR`, `it_IT`, `es_ES`, `ja_JP`,
  `ko_KR`), Wert ein String oder ein Array von Segmenten; die Segmente werden ohne
  Trennzeichen aneinandergehängt. chatot schreibt eine Nachricht mit `\n`, `\r` oder `\f`
  als Array, geteilt **hinter** jeder dieser Sequenzen (ein leeres Segment gibt es nicht;
  eine leere Nachricht wird als `[]` geschrieben). Eine Nachricht ohne Umbruch ist ein String.
- Beim Kodieren nimmt chatot den Schlüssel der Spielsprache und fällt auf `en_US` zurück;
  fehlt beides, ist es ein Fehler. Fremdsprachen werden nicht kodiert.
- Der Kodierer ignoriert Schlüssel auf oberster Ebene außer `key` und `messages`; ein
  UTF-8-BOM am Dateianfang wird abgeschnitten.

Kanonische Form des Studios (`TextArchiveDoc::to_json`): `serde_json::to_vec_pretty`,
`key` vor `messages`, je Nachricht `id` zuerst, dann alle Sprachschlüssel alphabetisch
sortiert (unabhängig davon, welche Sprache der Editor zeigt), Segmente wie chatot.

## Schreiben und Zeitstempel

`write_archive` kodiert zuerst aus einer Temp-Kopie der JSON in eine Temp-Datei (beide unter
`<Projekt>/.g4/tmp`), schreibt dann die JSON atomar und benennt zuletzt die Temp-Datei in
`unpacked/textArchives/NNNN` um. Die JSON wird nach dem Member nicht mehr angefasst.
Weil `rename` den Zeitstempel der Temp-Datei behält, ist die mtime der JSON danach nicht
älter als die des Members (ohne Zeitstempel zu setzen). Das hält DSPREs Regel `--newer` ruhig;
welche Fassung im Studio gilt, entscheidet die Zeit nicht mehr (nächster Abschnitt).

`write_archive` gilt, wo das Studio die Texte verfasst (Speichern im Store). Ist die JSON die
Quelle (der Build kodiert eine neuere JSON, das Auflösen behält die JSON), schreibt
`encode_member` nur den Member: Die JSON behält ihre Bytes, auch eine andere Formatierung oder
ein Zeilenende am Schluss, so bleibt `git status` nach `git switch` und Bauen sauber. Damit die
JSON nicht älter als der Member ist, bekommt der Member dann die mtime der JSON, wenn seine
neuer wäre.

## Welche Fassung gilt

Seit Phase 4.2 entscheiden Fingerabdrücke, nicht die mtime (`text::TextSync`, Implementierung
`crates/g4-kernel/src/text.rs`). Je Archiv steht in `.g4/build/state.json` unter `texts` der
letzte Abgleich: die Fingerabdrücke von JSON und Member, als beide zusammenpassten, immer mit
Inhalts-Hash, dazu der Zeitpunkt der Aufnahme (Format und racy-Regel in
[build_state.md](build_state.md)). `TextSync::side` vergleicht beide Dateien damit:

| Seite (`TextSide`) | wann | es gilt |
|---|---|---|
| `Neither` | keine der beiden Dateien da | – (gepacktes NARC) |
| `OnlyJson` / `OnlyMember` | nur eine Datei da | diese |
| `Unknown` | beide da, kein Abgleich aufgezeichnet (kein Bauzustand) | JSON |
| `InSync` | keine Seite geändert, oder beide geändert und der Member hält schon die Texte der JSON (ein Git-Wechsel, der beide passend tauscht) | JSON |
| `JsonNewer` | nur die JSON geändert | JSON, der Build kodiert sie |
| `MemberNewer` | nur der Member geändert (DSPRE) | Member |
| `Conflict` | beide geändert, die Texte passen nicht | JSON zum Lesen; der Build fragt |

Gleich heißt: gleicher `stat`, oder bei anderem `stat` (Kopie, Checkout, `touch`) gleicher
Hash. Ein kopiertes oder geklontes Projekt mit seinem `.g4/build/` fragt daher nichts. Die
Texte („hält der Member die Texte der JSON“: gleicher Schlüssel, gleiche Nachrichten) werden
nur verglichen, wenn beide Seiten geändert sind.

Aufgezeichnet wird (`text::record_text_sync`) nach jedem Speichern im Store (einmal je `save`
für alle geschriebenen Archive, ebenso beim Anlegen eines Archivs), nach dem Kodieren im Build
oder beim Übersetzen der Scripts, wenn der Build ein Archiv `Unknown` oder beidseitig passend
geändert vorfindet, nach dem Schreiben der JSON aus dem Member (vor rotom, siehe unten) und nach
dem Auflösen eines Konflikts. Ein veralteter Stand kann die falsche Seite lesen (ein seither
aufgezeichneter Abgleich lässt einen neueren Member als `Unknown` oder `Conflict` erscheinen, dann
gälte die JSON). Wer `TextSync` hält, lädt ihn daher neu, sobald `state.json` einen anderen `stat`
hat (`TextSync::is_current`): der Store bei jedem Lesen und Speichern eines Textarchivs, die
Prüfungen (`checks::Snapshot`) einmal je Lauf.

**Speichern im Store:** Ein ungespeichertes Archiv mit `Conflict` wird nicht geschrieben; `save`
gibt `KernelError::TextConflict` zurück, bevor irgendetwas geschrieben ist. Nach dem Auflösen gilt
die übliche Regel: Weicht die geltende Datei von dem ab, was der Store geladen hat, ist es `Stale`.

**Bauen (`chatot`):** kodiert die Archive mit `OnlyJson`, `JsonNewer` und `Unknown`, deren
Member die Texte der JSON noch nicht hält; hält er sie schon, wird nur aufgezeichnet. Hat ein
Archiv `Conflict`, bricht der Schritt mit `KernelError::TextConflict` ab, bevor etwas
geschrieben ist („text archives changed as JSON and as member since the last build: 0012,
0013“). Hat der Build `Unknown`-Archive kodiert oder geprüft, hängt er ein Problem der Schwere
Hinweis an (Datei `.g4/build/state.json`, ohne Position): „N text archives without build state:
the JSON version applies.“

**Auflösen:** `text::resolve_conflict(project, n, TextKeep)`. `Json` kodiert den Member aus
der JSON (die JSON behält ihre Bytes), `Member` schreibt die JSON aus dem Member (nur die Editorsprache, IDs
`msg_NNNNN`). Beides zeichnet auf; der nächste Build läuft durch. Mit `Member` werden in einem
übernommenen Projekt die Scriptdateien der Header, die das Archiv verwenden, als veraltet markiert
(`scripts::mark_stale`): Ein Textliteral, das rotom an die verworfene JSON angehängt hatte, hängt
die nächste Übersetzung wieder an. Markiert wird vor dem Schreiben: Scheitert die Markierung,
bleibt das Archiv im Konflikt, statt als abgeglichen zu gelten.

**Vor rotom** (`scripts::compile`): Eine JSON mit `MemberNewer` wird aus dem Member neu
geschrieben und aufgezeichnet, denn rotom liest die JSON ohne diese Regel und hängte sonst an
eine veraltete Fassung an. Im Build fragt schon der Schritt `rotom`: Hat ein Archiv `Conflict`,
scheitert er mit `KernelError::TextConflict`, bevor rotom etwas schreibt. Außerhalb des Builds
(Übersetzen nach dem Speichern der Header-Tabelle) wird ein Archiv mit `Conflict` nach rotoms
Anhängen nicht kodiert; der nächste Build fragt.

## Abspalten

Zeigen zwei Figuren dieselbe Nachricht (`shared_with > 0` im Dialog, siehe
[script_file.md](script_file.md) „Dialog einer Figur“), gibt
`dialog::split_message(store, header, file, index, archive, message)` (`crates/g4-kernel/src/dialog.rs`)
dem Objekt `index` der Eventdatei `file` eine eigene Kopie: Die Nachricht wird ans Ende des Archivs
kopiert (Text und Fremdsprachen gleich, ID `msg_<n>`, neuer Index = bisherige Zahl der Nachrichten),
und jede Stelle **dieses Dialogs**, die `(archive, message)` nennt (der eigene Slot und die Labels,
die nur er erreicht), bekommt in der Quelle die neue Nummer; die Zahl wird an Zeile und Spalte des
Dialogs ersetzt, von hinten nach vorn. Die Quelle wird ganz geschrieben und übersetzt
(`scripts::write_and_compile`). Kopie, Quelle, Member, berührte Archive und rotoms
`compile-state.json` (als ganze Datei) sind **ein** Undo-Schritt „Dialog von Figur {local_id}
abgespalten“. Die Kopie liegt im Store (gespeichert mit dem nächsten Speichern), die Quelle steht
sofort auf der Platte.

Abgelehnt (`KernelError::Refused`, nichts geschrieben):

| Fall | Satz |
|---|---|
| keine andere Figur zeigt die Nachricht | `NOT_SHARED` „Die Nachricht gehört nur dieser Figur.“ |
| eine Stelle steht in einer Funktion außerhalb des eigenen Slots, die der Lauf eines anderen Slots der Datei erreicht (Sprung oder weiterlaufender Code) | `SHARED_LABEL` „Die Nachricht steht in einem gemeinsamen Sprungziel; in Bauen im Script trennen.“ |
| ein anderes Objekt oder Schild (Eventdateien der Header mit dieser Scriptdatei) führt einen Slot desselben Rumpfs aus | `SHARED_SCRIPT` „Eine andere Figur führt dasselbe Script aus; in Bauen ein eigenes Script anlegen.“ |
| die Nummer der Kopie passt nicht in den Nachrichtenparameter eines umzuschreibenden Befehls (rotoms Datenbank: Platin `Message`/`MessageInstant`, HeartGold `NPCMsg`/`NonNPCMsg` nehmen ein u8, also bis 255; sonst bis 0x3FFF, ab 0x4000 wäre es eine Variable) | `NUMBER_TOO_HIGH` „Die Kopie bekäme eine Nummer, die der Nachrichtenbefehl nicht fasst“ mit Befehl, Höchstnummer und Nummer der Kopie |
| die Nachricht steht nicht im Dialog, das Archiv ist gesperrt | mit Archiv und Nummer |
| rotom hängt beim Übersetzen ein Text-Literal der Datei an dasselbe Archiv | der ganze Schritt wird zurückgenommen |

Ein Übersetzungsfehler in der Datei (auch in einem anderen Script) schreibt die alte Quelle und
rotoms `compile-state.json` zurück (der gescheiterte Lauf schreibt sie: die Quelle als `Dirty`, jeder
andere Eintrag neu gestempelt; den Member des Scripts schreibt er nicht) und nimmt die Kopie zurück
(`Store::rollback_to`): kein halber Schritt, nichts zum Wiederholen.

Das Studio schreibt die Texte des Dialog-Editors über `edit_text_archive` mit dem optionalen
Argument `label` („Dialog geändert“); ohne heißt der Schritt wie bisher `edit text`, `add message`,
`remove message`.

## Prüfungen

`validate` ist ein Trockenlauf des Kodierers (`chatot::validate_message`). Die Probleme sind
Warnungen, sie blockieren das Speichern nicht. Die Zeilenbreite prüft `validate` nicht mehr (früher
200 px, `DIALOG_LINE_MAX_PX`): sie gehört zur Prüfung `text.too-wide` im Modul `checks`, mit
`dialog_line_px` aus [check_limits.md](check_limits.md) (216 px, nur Dialogarchive). Die
Übersetzung von Scripts meldet rotoms eigene Zeilenwarnung (`MessageLineTooLong`) ebenfalls nicht.
