# Schnelles Gate: Design

Stand: 2026-11-13. Status: vom Auftraggeber freigegeben.

## Absicht

Ein Release hängt heute an `scripts/gate.sh`. Ein grüner Lauf dauert 6,1 h, jeder Fehlschlag
startet ihn von vorn; für 1.0.2 waren es zusammen 10 h. Das Ziel ist ein blockierendes Gate
unter 15 Minuten, ohne die Abdeckung der vollen ROM-Matrix aufzugeben.

## Befund (gemessen)

- Gate 1.0.2, grüner Lauf: 22.078 s. Davon vier volle Durchläufe der Workspace-Testsuite:
  2.689 s ohne ROM-Daten, 6.406 s / 6.186 s / 5.903 s je ROM-Projekt. Kompilieren etwa 195 s.
- Ein Test mit dem Kunstprojekt (`synthetic_project()`, `TextFixture`) legt in der Spitze
  9.127 bis 18.557 Dateien an und löscht sie wieder. Auch ein reiner Ablehnungstest
  (`world::tileset::tests::tileset_edit_refuses_without_file_and_past_the_tiles`) braucht so
  einzeln 13,9 s.
- Zeit pro Test (Kernel-Lib, 1.618 Tests, unter Last gemessen): 1.185 Tests unter 1 s
  (zusammen 84 s). Rund 430 Tests tragen fast die ganze Zeit; `textproject` allein 309 Tests
  mit im Schnitt 27,8 s.
- Optimiertes Testprofil bringt nur etwa 11 %: die Tests warten in `open()` und `unlink()`,
  nicht im eigenen Code.
- `vanilla_copy()` kopiert rund 20.000 Dateien einzeln: 3,8 s Kopieren, 2,0 s Löschen. Ein
  `clonefile(2)` auf das ganze Verzeichnis braucht 0,1 s; das Löschen bleibt bei rund 2 s.
- Die ROM-Durchläufe wiederholen alle Tests, die keine ROM lesen (`gate.sh:209`), etwa 2,1 h
  pro Gate.
- Das Gate bricht beim ersten Fehler ab (`gate.sh:106`) und kann nicht wiederaufsetzen. Drei
  der vier Fehlschläge bei 1.0.2 waren Umgebung oder Wackler.

## Ziele

1. Das blockierende Gate (`scripts/gate.sh` ohne Argumente, lokal und in `ci.yml`) läuft auf
   dem Entwicklungs-Mac bei ruhiger Maschine in höchstens 15 Minuten, gemessen an der
   Sekunden-Tabelle des Gates.
2. Das schnelle Gate führt weiterhin alle Tests aus, die keine ROM brauchen. Nichts wird
   weggelassen, um die Zeit zu erreichen.
3. Die volle ROM-Matrix läuft weiter vollständig, aber entkoppelt: nachts und auf Abruf.
4. Beim Release zeigt ein Skript, auf welchem Stand der letzte grüne Volllauf liegt.
5. Ein Fehlschlag durch Umgebung kostet Minuten, keinen Neustart von Stunden.

## Nicht-Ziele

- Umbau der GitHub-Workflows (doppelte Dispatch-Läufe, Dependabot-Schübe, Cache-Kontingent,
  parallele Jobs). Das ist ein eigenes Vorhaben. `ci.yml` profitiert hier nur dadurch, dass
  dasselbe Gate schneller wird.
- Ausnahmen in Microsoft Defender. Das entscheidet der Besitzer der Maschine, nicht das Repo.
- Wechsel auf cargo-nextest.
- Ein Dateisystem im Speicher oder eine Abstraktion über Dateizugriffe im Produktcode.

## Anti-Ziele

- Tests teilen sich veränderlichen Zustand. Jeder schreibende Test arbeitet weiter auf
  seiner eigenen Kopie.
- Wiederholungsversuche verdecken echte Fehler im Volllauf.
- Ein Release geht ohne grünen Volllauf durch, ohne dass das Skript es sichtbar gesagt hat.
- Ein neuer ROM-Test fällt still aus beiden Läufen heraus.

## Wege

1. **Testprojekte einmal bauen, pro Test klonen (gewählt).** Die Tests bleiben wie sie sind,
   nur der Aufbau ihrer Projekte ändert sich. Risiko: Der Anteil des Aufbaus an den 28 s pro
   `textproject`-Test ist noch nicht gemessen; deshalb beginnt die Umsetzung mit einem
   Probeumbau (Teil A, Schritt 1).
2. **Dateisystem im Speicher.** Am schnellsten, greift aber in den ganzen Kernel ein und
   prüft nicht mehr das echte Dateisystem. Verworfen.
3. **Langsame Tests aus dem schnellen Gate nehmen.** Billig, lässt aber gerade `textproject`
   ungeprüft. Verworfen (Ziel 2).

Zur Zweiteilung selbst hat der Auftraggeber entschieden: Blockierend läuft nur das schnelle
Gate, die volle ROM-Matrix läuft nachts. Der Preis ist bekannt und angenommen: Ein
ROM-spezifischer Fehler fällt erst im Nachtlauf auf statt vor dem Merge.

## Teil A: Testprojekte

Betroffen ist `crates/g4-kernel/src/test_support.rs` und was darauf aufbaut (728
Aufrufstellen von `synthetic_project()`, `synthetic_hgss_project(..)` und `TextFixture::*`,
107 von `vanilla_copy()`), dazu gleichartige Helfer in `apps/studio/src-tauri`.

1. **Probeumbau zuerst.** `TextFixture::exported()` und `TextFixture::dspre()` auf „einmal
   bauen, klonen“ umstellen und das Modul `textproject` der Kernel-Lib vorher und nachher
   messen (`RUSTC_BOOTSTRAP=1 cargo test -p g4-kernel --all-features --lib -- -Z
   unstable-options --report-time textproject`, ruhige Maschine). Sinkt die Summe der
   Testzeiten nicht mindestens auf ein Fünftel, anhalten und den Befund zurückmelden; dann
   trägt dieser Weg Ziel 1 nicht allein.
2. **Vorlage pro Variante.** Jede Variante ohne Parameter wird einmal pro Testprozess gebaut
   (`OnceLock`) und danach nicht mehr verändert. Varianten mit Parametern
   (`synthetic_hgss_project(names, headers)`) werden nach ihren Parametern abgelegt.
   Varianten mit Closure (`TextFixture::with(prepare)`, `unmarked(prepare)`) lassen sich
   nicht ablegen; ob `prepare` nach dem Klon laufen kann oder die Variante beim Neuaufbau
   bleibt, entscheidet der Umsetzer je Fall.
3. **Klon pro Test.** Auf macOS ein `clonefile(2)` auf das Vorlagenverzeichnis, sonst
   (Windows-Lauf in CI) das heutige `copy_dir`. Apples Handbuch rät vom Klonen ganzer
   Verzeichnisse ab und empfiehlt `copyfile(3)`; das klont aber pro Datei und ist der
   langsame Weg von heute. Für Testdaten ist der Verzeichnis-Klon vertretbar (gemessen:
   18.485 Dateien vollständig, 0,1 s). Schlägt er fehl, fällt der Helfer auf `copy_dir`
   zurück. `vanilla_copy()` nutzt denselben Helfer.
4. **Löschen.** Das Aufräumen eines Klons kostet weiter rund 2 s pro 18.000 Dateien. Der
   Umsetzer misst nach Schritt 3, ob das Ziel 1 gefährdet, und wählt dann: Tests, die nur
   lesen, bekommen die Vorlage direkt (schreibgeschützt), oder das Kunstprojekt wird
   kleiner, oder beides. Ein kleineres Kunstprojekt darf keine Zusicherung eines Tests
   ändern.
5. Die öffentlichen Signaturen der Helfer bleiben, damit die Aufrufstellen nicht angefasst
   werden müssen.

## Teil B: Zwei Läufe

**Schnelles Gate** (`scripts/gate.sh`, auch in `ci.yml` mit `G4_GATE_HERMETIC=1`):

- Reihenfolge: zuerst alles Billige und Störanfällige (fmt, clippy, deny, Lizenzen,
  Skript-Tests, Prüfskripte, `pnpm test`, `pnpm build`, `g4emu unit`), dann `cargo test`
  ohne ROM-Daten und `mcp`.
- Lokal zusätzlich eine ROM-Stichprobe an einem Projekt (Platin DE, das schnellste):
  Projektstart und die Zeitmessungen des Build-Graphen. Heute zusammen etwa 150 s.
- `UV_NATIVE_TLS=1` fest für die `uv`-Schritte (PyPI-TLS-Fehler am 10.11. und 12.11.).

**Volllauf** (`scripts/gate.sh --full`):

- Alles aus dem schnellen Gate, dann die bedingten Tests, dann Zeitmessungen und
  Projektstart wie heute.
- **Bedingter Test** heißt jeder Test, der ohne eine der Variablen `G4_TEST_PROJECT`,
  `G4_TEST_ROM`, `G4_APICULA`, `G4_DSROM_CLI`, `G4_ROTOM_LSP` überspringt, ob er sie direkt
  liest oder über einen Helfer (`vanilla_*`, `set_up`, `open_copy`, `exported_copy`, ...).
  Tests, die ein Projekt oder eine ROM brauchen, laufen einmal pro ROM-Projekt. Tests, die
  nur ein Werkzeug brauchen (etwa `world_bake`, `world_freeform`, `form2_tests` mit
  `G4_APICULA`), laufen einmal pro Volllauf.
- Wie die bedingten Tests ausgewählt werden (Namensschema, eigenes Test-Target, erzeugte
  Liste), entscheidet der Umsetzer. Bedingung: Eine Prüfung im Volllauf stellt sicher, dass
  jeder Test, der im Lauf ohne ROM-Daten überspringt, im Volllauf tatsächlich ausgeführt
  wird. Fällt einer heraus, ist der Volllauf rot.
- `cargo test` läuft mit `--no-fail-fast`; das Gate läuft nach einem Fehlschlag in einem
  ROM-Projekt mit den übrigen weiter und meldet am Ende alles.
- `--from <Schritt>` setzt bei einem Schritt wieder auf. Auf demselben Commit mit sauberem
  Arbeitsbaum übernimmt der Lauf die grünen Zeilen des früheren Ergebnisses und zählt als
  Volllauf; die Ergebnisdatei vermerkt das Wiederaufsetzen. Hat sich der Commit oder der
  Arbeitsbaum geändert, lehnt `--from` ab.
- Kein Wiederholungsversuch für Tests im Volllauf, außer dem schon vorhandenen bei den
  Zeitmessungen.
- Ergebnis: eine Datei `<sha>.json` (Commit, Ergebnis, Dauer, Tabelle, ob wiederaufgesetzt)
  mit dem Log daneben im Ordner `gate-artefakte/full/` neben dem Haupt-Checkout. Der Pfad
  wird am Haupt-Checkout verankert (über das gemeinsame Git-Verzeichnis), nicht am
  Worktree, und lässt sich per Variable überschreiben. Dasselbe gilt für `G4_ROMS_DIR`.
- **Grün** ist ein Ergebnis nur, wenn kein Schritt fehlschlug und keiner als „skipped“
  verzeichnet ist. Fehlt ein Kern, ein Spielstand, eine ROM oder ein Werkzeug, ist das
  Ergebnis „unvollständig“ und zählt für ein Release nicht.

**Nachtlauf:** Ein launchd-Job auf dem Entwicklungs-Mac startet den Volllauf nachts auf
`origin/main` in einem eigenen Worktree mit dem gemeinsamen `target/`, sofern für diesen
Commit noch kein grünes Ergebnis vorliegt; ein rotes oder unvollständiges Ergebnis sperrt
den nächsten Versuch nicht. Der Job richtet den Worktree so ein, wie `docs/agent-gate.md`
es beschreibt (`tools/`, `node_modules`, absolute Pfade). Die Einrichtung ist ein Skript im
Repo plus ein Abschnitt in `docs/dev-setup.md`; installiert wird der Job von Hand.

**Release-Regel** (`scripts/release-local.sh`, vom Auftraggeber am 2026-11-13 so
entschieden: die lockere Fassung): Das Skript sucht das jüngste grüne Volllauf-Ergebnis für
den Release-Commit oder einen Vorfahren. Liegt es nicht auf dem Release-Commit selbst,
warnt das Skript und listet die Commits und geänderten Pfade seitdem, läuft aber weiter.
Gibt es gar kein grünes Ergebnis auf einem Vorfahren, warnt es ebenso deutlich und läuft
weiter. Die Prüfung blockiert nie. Der Preis ist bekannt und angenommen: Code, den kein
Volllauf gesehen hat, kann in ein Release gelangen.

## Reihenfolge und Haltepunkte

- Teil B hängt nicht an Teil A und kann zuerst oder parallel gebaut werden. Er bringt für
  sich die Zweiteilung, das Wiederaufsetzen und den kürzeren Volllauf, aber nicht Ziel 1.
- Der Probeumbau (Teil A, Schritt 1) ist der einzige Haltepunkt: Verfehlt er das Fünftel,
  ruht Teil A, Teil B läuft weiter, und der Befund geht an den Auftraggeber.
- Ist Teil A fertig und Ziel 1 trotzdem verfehlt: unabhängige Schritte des Gates dürfen
  parallel laufen (etwa `pnpm test` neben `cargo test`). Tests weglassen ist kein Mittel
  (Ziel 2). Reicht auch das nicht, geht der Befund mit Zahlen an den Auftraggeber.

## Zeitbudget schnelles Gate (Ziel, nicht gemessen)

| Block | Heute | Ziel |
|---|---|---|
| Prüfungen, Skript-Tests, Builds, `g4emu unit` | ca. 90 s | ca. 90 s |
| `pnpm test` | 136 s | 136 s |
| `cargo test` ohne ROM-Daten und `mcp` | 2.717 s | höchstens 480 s |
| ROM-Stichprobe (nur lokal) | – | ca. 150 s |
| Summe | ca. 49 min | höchstens 15 min |

## Dem Umsetzer überlassen

- Ablage und Schlüssel der Vorlagen, Umgang mit den Closure-Varianten, Wahl beim Löschen.
- Auswahlmechanik der ROM-Tests.
- Form der Ergebnisdatei und des launchd-Jobs.
- Anpassung von `docs/release.md` und `docs/dev-setup.md` an die neuen Abläufe und Zeiten.

## Erfolg

- Die Sekunden-Tabelle eines schnellen Gates auf ruhiger Maschine zeigt höchstens 900 s.
- Ein Volllauf auf demselben Commit ist grün und führt jeden ROM-Test für jedes Projekt aus.
- `release-local.sh` nennt das jüngste grüne Volllauf-Ergebnis und warnt, wenn es nicht auf
  dem Release-Commit liegt oder fehlt.

## Nachtrag 2026-11-13: schwere Tests und ROM-Stichprobe im Volllauf

Gemessen: Das schnelle Gate mit allen Tests ohne ROM-Daten brauchte 1.342 s statt der
geforderten 900 s; `cargo test` ohne ROM-Daten allein 809 s, neben den parallelen Schritten
1.181 s. Ein Virenscanner macht jede Datei einer Testvorlage teuer, und der Testcode ist
ausgereizt (Teil A, Parallelität). Der Auftraggeber hat zwei Dinge entschieden: Das schnelle
Gate hat keinen ROM-Schritt, auch keine Stichprobe; die Zeile „ROM-Stichprobe“ in Teil B und im
Zeitbudget entfällt. Und die schwersten Tests, die ab 8 s (238 von rund 3.900 Tests, zusammen
4.072 von 6.080 s Testzeit), stehen in `scripts/gate-heavy-tests.txt` und laufen nur im
Volllauf. Das ersetzt in Ziel 2 „alle Tests ohne ROM“ durch „alle Tests ohne ROM, außer den
gelisteten schweren“; Weg 3 gilt damit für diese Tests. Der Preis: Ein Fehler, den nur ein
gelisteter Test findet, fällt erst im Nachtlauf auf. Die Liste erzeugt
`scripts/gate-heavy-update.sh` aus einem Zeitlog neu, wenn das Gate warnt (Summe über 900 s).
Auf CI (`CI=true`, wie GitHub Actions es setzt) liest das schnelle Gate die Liste nicht und führt
jeden Test ohne ROM-Daten aus, denn die Liste gibt es nur wegen des Virenscanners auf dem Mac des
Entwicklers; eine ausdrücklich gesetzte `G4_GATE_HEAVY_LIST` gilt weiter (der Testzugang der
Skript-Tests).
