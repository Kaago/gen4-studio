# Nachtrag 6.5a für die OpenPencil-Datei

Stand: 06.11.2026. Diese Anleitung sagt, was zu Phase 6.5a „PNG und Modelle“ neu in `Editor_DesignSystem.fig` gehört: der Dialog „Teile importieren…“ mit der Zeile „Umwandeln“, den Vorschauzeilen unter einem umgewandelten Teil und dem Baustein `Budget` im Dialog. **Die Datei wurde in diesem Branch nicht angefasst:** 6.5a läuft parallel zu anderen Phasen, die alle in `main` zusammenkommen, und die binäre `.fig` lässt sich nicht mergen. Der Frame ist darum noch nicht gerendert; er wird **auf `main` nach dem Merge** in die Datei gerendert (wie bei 6.2, als OpenPencil nicht erreichbar war).

Was die App zeigt, beschreibt `design-system/README.md`, Abschnitt „Nachtrag 6.5a“. Texte und Zustände stammen von dort und aus dem Code: `apps/studio/src/workbench/inspector/{ImportPartsDialog.tsx,ImportPartsDialog.module.css,importParts.ts}`, `ui/primitives/{Dialog,Segmented,TextField,CheckList,Checkbox,Budget,Button}.tsx`, `platform/dialogs.ts` (`openFiles` mit Dateifilter). Projekt „Sinnoh-Remix“, Platin DE, Karte 0005 wie im Gate von 3b.4b. Beispielwerte, keine Spieldaten:
- das Testmodell `g4_imp_cube.obj` mit seiner Textur ist selbst erzeugt (`g4k import-samples`, `model_import::samples`), keine Spielgrafik;
- die Größen (Würfel 21.740 B, „stadt.obj“ 139.264 B), die Zahlen der Flächen und das VRAM sind Beispiele; die Test-Umgebung zeigt für den Würfel andere, kleinere Werte.

Vergleichsquelle sind die Screenshots der Test-Umgebung aus dem Gate von 6.5a (Task 11, Ports 1500/1501; Ablage nach `docs/phase-6-5a-gate.md`, nicht im Repo). Verglichen werden Aufbau, Maße und Texte, nicht die Zahlen.

Neue Tokens gibt es nicht, neue Symbole auch nicht. Der Dialog nutzt `flaeche/blatt`, `linie/standard`, `text/primaer`, `text/sekundaer`, `flaeche/deck` und für das Budget `akzent/grund` und `status/fehler`; die Zustandssymbole der Prüfliste sind die vorhandenen (`success`, `error`, in der Vorlage `ph:check-circle-fill`, `ph:x-circle-fill`).

**Die Vorlage ist nicht gerendert.** `beispiele/import-modell.body.js` ist gegen `jsx-lib-app.js` geschrieben und lief mit einer Attrappe der Bausteine (`Frame`, `Text`, `Icon`, `Ellipse`, `designVar`), hell und dunkel, ohne Fehler durch (126 Knoten); ein Bild davon gibt es nicht. Beim ersten Rendern Maße und Umbrüche prüfen. Wahrscheinliche Stellen:
- die Prüfzeile: Symbol, Kästchen und die Zeilen darunter stehen in einer Spalte von 414 px; lange Sätze müssen dort umbrechen, nicht über den Dialog laufen;
- das Feld „Einheit“ (220 breit) und „Farben je Textur“ in einer Reihe: bricht die Reihe in der App um (sie ist `flex-wrap`), die Vorlage anpassen;
- der Balken des Budgets: die Füllung ist ein absoluter Rahmen in der Spur (mindestens 4 breit), über der Grenze voll.

## Was neu zu zeichnen ist

Ein Frame, gebaut aus `beispiele/import-modell.body.js`. Diese Teile gibt es schon: `Dialog`, `Segmented` (klein), `Field`, `Check`, `Btn`, `Badge` und das Budget aus der Vorlage von 6.2 (`hoehen-budget.body.js`, `BudgetX`; hier als `BudgetModell` nachgebaut, damit die Vorlage für sich steht).

1. **Frame „Teile importieren: Modell aus OBJ“** (Hell und Dunkel, je ein Frame; zwei Dialoge zu 480 px nebeneinander auf der Abdunklung `flaeche/ueberlagerung`, 1080 breit).
   - **Links, der Würfel:** Titel „Teile in Karte 0005 importieren“; Segmente „Quelle“ mit „Datei“ gewählt, Feld „Pfade, eine je Zeile“ mit `~/Modelle/g4_imp_cube.obj`, „Dateien wählen…“; die Zeile **„Umwandeln“** mit „Farben je Textur“ (Segmente „16“ gewählt, „4“) und dem Feld „Einheit“ (16, Hilfe „Welteinheiten je Modelleinheit; 16 = eine Kachel“); die Prüfliste mit einer Zeile „Modell · 21.740 Bytes · g4_imp_cube.obj“ (ok, angekreuzt), darunter die Hinweise „OBJ-Modell umgewandelt: 6 Flächen, 6 Vierecke, 0 Dreiecke, 2 Streifen.“, „Neue Textur g4_imp_cube: 16 × 16, 16 Farben, mittlerer Farbfehler 7,6.“, „BDHC bleibt unverändert, Laufhöhen prüfen.“ und die **Vorschau**: „OBJ · 6 Vierecke · 0 Dreiecke · 2 Streifen“, „g4_imp_cube · 16 × 16 · 16 Farben · neu“, „VRAM des Gebiets 23.544 B · Vanilla höchstens 110.640 B“ und das Budget „Modell“ Ruhig „21.740 von 61.440 B“. Tasten „Abbrechen“ und „1 Teil übernehmen“ (primär).
   - **Rechts, die Fehlerzeile `model.too-big`:** dieselbe Quelle mit `~/Modelle/stadt.obj`, die Zeile „Modell · 139.264 Bytes · stadt.obj“ mit Fehlersymbol und gesperrtem Kästchen, darunter „Das Modell ist 139.264 Byte groß, erlaubt sind 61.440. Wähle ein kleineres Modell.“, der Hinweis der Umwandlung, die Vorschau mit einer Textur des Pakets („stadt · 64 × 64 · im Paket“) und das Budget „Modell“ **Voll** („139.264 von 61.440 B“, Plakette „Voll“, Balken voll in `status/fehler`). Die Taste „0 Teile übernehmen“ ist gesperrt.

## Lage und Einspielen (auf `main`, nach dem Merge)

- **Lage:** Seite „App: Werkbank“ (dort stehen die Dialoge der Welt), nächste freie Reihe nach allen Phasen, die bis dahin eingespielt sind; x = Spalte × 1520, y = Reihe × 1000, Hell in Spalte 0, Dunkel in Spalte 1. Vorher mit `get_page_tree` (depth 1) die belegten Positionen ablesen und nur eigene Frames anfassen. An der Grenze von 10.000 Knoten gehen neue Frames nur über `render`, Korrekturen über `render` mit `replace_id`.
- **Bauen:**
  1. Aus `openpencil/beispiele/` `bash ../build-screen.sh import-modell` (Hell) und `bash ../build-screen.sh import-modell --dunkel` (Dunkel) aufrufen. Das Skript schreibt nur die `.jsx` neben die Vorlage.
  2. `node ../../scripts/op.mjs render '{"document_id":"tab-1","page_id":"<SEITE>","x":<x>,"y":<y>}' --jsx import-modell.jsx` (`<SEITE>` aus `list_pages`).
  3. Per `export_image` mit den Screenshots des Gates vergleichen (Dialog mit dem Würfel, Zeile „Umwandeln“, Vorschau, Budget).
  4. Korrigieren: Body ändern, neu bauen, `render` mit `"replace_id":"<ID>"` (höchstens drei Läufe je Frame). Ein halber Frame nach einem abgebrochenen `render` wird gelöscht. Die erzeugten `.jsx` und `.png` nicht einchecken.
- **Danach:**
  1. Speichern (`save_file`), schließen; bei gesperrtem Bildschirm warten, nicht wiederholen.
  2. Aus der Repo-Wurzel `node design-system/scripts/fix_fig_text.mjs design-system/Editor_DesignSystem.fig --rgb555`.
  3. Im README den Satz „Der Frame ist noch nicht in der `.fig`“ im Abschnitt „Nachtrag 6.5a“ durch Seite und Lage ersetzen, hier „nicht gerendert“ und „nicht angefasst“ ebenso.
  4. Im Brett „Stand der App“ (Seite „Titel“, Vorlage `beispiele/stand-der-app.body.js`, `render` mit `replace_id`) die Zeile „Kartenteile importieren“ um „OBJ, glTF und PNG (6.5a)“ ergänzen.

  `check-tokens.sh` ist nicht betroffen (keine neuen Tokens).

## Was nicht gezeichnet werden muss

- **Nur PNG:** dieselbe Zeile „Umwandeln“ ohne „Einheit“, der Teil heißt „Texturen“, die Vorschau ohne Modellzeile und ohne Budget.
- **glTF/GLB:** wie OBJ, die Modellzeile beginnt mit „glTF“.
- **„Einheit“ ungültig:** das Feld mit Fehlerkante und dem Satz „Zwischen 0,0625 und 256.“ (wie jedes `TextField` mit Fehler).
- **Tüfteln:** das Budget nennt die Grenze hex dahinter („0xF000“), wie im Abschnitt „Budget“ des Inspectors (Nachtrag 6.2).
- **Die übrigen Sätze der Prüfcodes** (`model.*`, `texture.*`, `part.no-area`): dieselbe Zeile mit anderem Satz; die Liste steht in `importParts.ts`.
- **Nach dem Import:** Erfolgsmeldung und Befunde wie in 3b.4b; die Szene zeigt das neue Modell (keine 3D-Vorschau im Dialog).
