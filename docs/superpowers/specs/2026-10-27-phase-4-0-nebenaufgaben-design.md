# Phase 4.0: Nebenaufgaben (App-Symbol, Projektvorschau auf Start)

Stand: 2026-10-27. Teil „4.0 Nebenaufgaben“ der [Roadmap](../../ROADMAP.md), Abschnitt „Phase 4“. Wunsch des Nutzers vom 2026-10-26; unabhängig vom Kernel-Umbau der übrigen Phase 4, läuft vorweg. Branch `phase-4-0-nebenaufgaben` im Worktree `.claude/worktrees/phase-4-0`, kein Merge nach main in dieser Sitzung.

Entscheidungen hat der Agent autonom getroffen (Arbeitsweise des Nutzers: die empfohlene Variante wählen, nicht nachfragen). Wo es eine echte Alternative gab, steht sie mit dem Grund der Wahl in §5.

## 1. Ziel und Umfang

Zwei Posten:

1. **App-Symbol.** Gen4 Studio trägt eine eigene Marke statt des Standardsymbols von create-tauri-app (zwei Ringe in Gelb und Türkis). Die Marke kommt aus dem Design-System „Doppeldeck“: Entwurf in `Editor_DesignSystem.fig`, Quelle als SVG und 1024-px-PNG unter `design-system/marke/`, alle Größen der App aus `pnpm tauri icon`, eine Prüfung im Bundle, Beschreibung in `design-system/README.md` (Abschnitt „Marke“) und `docs/release.md`.
2. **Projektvorschau auf Start.** Jede Projektkarte unter „Willkommen zurück“ zeigt ein Bild des Projekts im DS-Rahmen: ein eigenes Bild (aus der Szene oder aus einer Datei), sonst das Kartenbild der Startstadt, sonst wie heute den Namen.

**Nicht in 4.0:** ein eigenes Dunkel-Symbol für macOS 26 (Tauri 2 kennt nur ein Symbol), ein Symbol für Dokumente oder Projektordner, ein Startbildschirm (Splash), Vorschaubilder in der Liste „Zuletzt“ des Dock-Menüs, Vorschau für Projekte, die nie in 4.0 oder später geöffnet wurden (sie zeigen den Namen, bis sie einmal geöffnet sind, §3.3), Bildbearbeitung (Zuschnitt wählen, Filter), Undo für das Vorschaubild (es liegt neben der Projekt-Historie, §3.2).

**Gate** (Ablauf in `docs/phase-4-0-gate.md`, nach dem Muster von `docs/phase-3b4-gate.md`):

- (G1) Auf einer frischen Platin-DE-Kopie: in der Welt „Als Vorschaubild“ aus der 3D-Ansicht gesetzt; zurück auf Start zeigt die Karte des Projekts dieses Bild.
- (G2) Über „Vorschaubild wählen…“ im Menü der Karte auf Start eine PNG- oder JPEG-Datei gewählt; die Karte zeigt sie auf 256 × 192 gebracht. Dasselbe über Technik.
- (G3) „Vorschaubild entfernen“: die Karte zeigt das Kartenbild der Startstadt (Platin Zweiblattdorf; auf einer HeartGold-Kopie Neuborkia).
- (G4) Ein Projekt ohne Kartenbild (Standardbild noch nicht erzeugt) zeigt den Namen wie bisher, eine verschwundene Projektmappe „nicht gefunden“ wie bisher, „Zuletzt“ bleibt.
- (G5) `scripts/check-bundle.sh` meldet `bundle ok` und erkennt die neue `icon.icns` am Hash.
- **Blick:** Start mit Vorschaubildern in Hell, Dunkel und Kompakt; die Marke in der `.fig` (Brett „Marke“) und in `tauri dev` (Dock).
- **Hand (nur der Nutzer):** Dock, Finder und „Über Gen4 Studio“ der gepackten App (`pnpm bundle`) zeigen die Marke.

## 2. App-Symbol

### 2.1 Konstruktion

Die Marke ist das Doppeldeck selbst, klar und minimalistisch: zwei Bildschirme übereinander mit dem Scharnier dazwischen, der einen Signatur des Systems (Prinzip 2). Oben die Schau (was der Spieler sieht), unten die Hand (was man anfasst). Flache Farben, fünf Formen, keine Kontur, keine Landschaft, kein Raster, keine Textur (Wunsch des Nutzers: „Clean und Minimalistic Vector Design“).

- **Raster:** 1024 × 1024. Körper ein abgerundetes Quadrat 824 × 824, mittig (100 px Rand rundum, das Raster der macOS-App-Symbole), Eckradius 184, Fläche `scharnier/grund` des dunklen Themas (Platin 700; ein Token `scharnier/flaeche` gibt es nicht). Kein Schatten in der Quelle; Finder und Dock zeichnen das Symbol wie es ist.
- **Zwei Bildschirme:** je 584 × 252 in `szene/ds/grund`, 120 px vom Körperrand rundum (der Block aus beiden ist quadratisch), Eckradius 64 = 184 − 120, konzentrisch zum Körper.
- **Scharnier:** im Abstand von 80 px zwischen den Bildschirmen eine Linie 584 × 32 in `akzent/grund` (Kobalt), mittig auf 512, mit runden Enden.
- **Auswahl:** auf dem unteren Bildschirm ein Quadrat 96 × 96 in `akzent/grund`, Radius 24, links, oben und unten gleich weit (78 px) vom Rand. Scharnier und Auswahl sind der einzige Akzent („das bist du gerade“).
- **„4“:** keine Ziffer in der Marke. Name und Generation stehen daneben (Fenstertitel, Über-Dialog), nicht im Symbol; eine Ziffer wäre ab 32 px Brei.
- **Farben:** drei Token-Werte, alle auf dem RGB555-Raster (Prinzip 5). Die Quelle nennt je Farbe das Token im `id` der SVG-Gruppe (`token-scharnier-grund`, `token-szene-ds-grund`, `token-akzent-grund`), damit eine Token-Änderung nachziehbar bleibt.
- **Kleine Größen:** Bei 32 px bleiben Körper, zwei dunkle Flächen, die Kobaltlinie und die Auswahl erkennbar, bei 16 px die Linie als heller Streifen und die Auswahl als Punkt. Kein eigenes Kleinsymbol (YAGNI; `tauri icon` skaliert aus 1024).
- **Schutzraum:** Um den Körper ein Viertel des Scharnier-Abstands frei, 20 px bei 824; in Dokumenten und auf Flächen der App mindestens 1/8 der Körperbreite.
- **Hell und Dunkel:** Ein Symbol für beide. Der Körper in Platin 700 steht auf hellem Dock und Finder als dunkle Form, auf dunklem Dock als hellere; eine Kontur braucht er nicht. Die Ecken außerhalb des Körpers sind durchsichtig, dieselbe Datei dient auf Flächen der App (Galerie, späterer Über-Dialog).

Diese Maße sind der Ausgangspunkt; der Entwurf in OpenPencil darf sie anpassen, die README hält die endgültigen fest.

### 2.2 Dateien

- `design-system/marke/gen4-studio-marke.svg` (Quelle, von Hand lesbar, keine eingebetteten Bilder), `design-system/marke/gen4-studio-marke-1024.png` (1024 × 1024, sRGB, mit Alpha), erzeugt aus dem SVG mit einem kleinen Skript (`design-system/scripts/render_marke.mjs` oder über OpenPencil `export_image`; das Skript, das der Plan wählt, steht in der README).
- `apps/studio/src-tauri/icons/*`: neu aus `pnpm tauri icon ../../design-system/marke/gen4-studio-marke-1024.png` (in `apps/studio`). Die Liste der Dateien bleibt wie heute (`tauri.conf.json` unverändert, außer `tauri icon` legt neue Namen an; dann zieht die Konfiguration nach). Überzählige Windows-Store-Logos, die `tauri icon` mit erzeugt, bleiben eingecheckt wie bisher.
- `.fig`: Brett „Marke“ auf der Seite „Grundlagen“ (Konstruktion mit Maßlinien, die Marke in 1024, 128, 32, 16 px, auf Hell und auf Dunkel, Schutzraum). Erst rendern, dann speichern, dann `fix_fig_text.mjs --rgb555` (Grenzen in `design-system/openpencil/NACHTRAG-3h.md`).

### 2.3 Prüfung im Bundle

`apps/studio/scripts/check-bundle.sh` bekommt zwei Zeilen: (a) `Contents/Resources/icon.icns` im Bundle hat denselben SHA-256 wie `apps/studio/src-tauri/icons/icon.icns`; (b) er ist nicht der Hash des alten Standardsymbols (die Konstante steht im Skript mit Kommentar). Ein Fehlschlag nennt, welche der beiden Bedingungen. `docs/release.md` beschreibt Marke, Quelle und Neuerzeugung (`pnpm tauri icon …`) und die neue Prüfzeile. Der Über-Dialog (`menu.rs`, `AboutMetadata`) zeigt auf macOS das Bundle-Symbol ohne Änderung; unter `tauri dev` nimmt Tauri `icons/icon.png` für das Dock.

## 3. Projektvorschau

### 3.1 Dateien im Projekt

- `.g4/preview.png`: das eigene Vorschaubild, genau 256 × 192, PNG RGB8. Es reist mit dem Projekt (und später mit Git).
- `.g4/preview-standard.png`: das Standardbild (Kartenbild der Startstadt), ebenfalls 256 × 192. Es ist ein Zwischenspeicher: die App erzeugt es beim Öffnen des Projekts neu, wenn es fehlt oder sich unterscheidet (§3.3). Getrennt vom eigenen Bild, damit „entfernen“ sofort ein Standardbild zeigt und die Startliste es ohne geöffnetes Projekt lesen kann.

Reihenfolge beim Lesen: eigenes Bild, sonst Standardbild, sonst keins (die Karte zeigt den Namen). Fehlerhafte Dateien (kein PNG, falsches Maß) zählen als nicht vorhanden; das Lesen schlägt deshalb nie fehl, solange der Projektordner lesbar ist.

### 3.2 Kernel (`crates/g4-kernel/src/preview.rs`)

```rust
pub const PREVIEW_W: u32 = 256;
pub const PREVIEW_H: u32 = 192;
pub enum PreviewSource { Eigen, Standard }       // serde: "eigen", "standard"
pub enum PreviewSlot { Eigen, Standard }
pub struct Preview { pub source: Option<PreviewSource>, pub png: Option<Vec<u8>> }
pub enum PreviewInput<'a> { Bytes(&'a [u8]), File(&'a Path), Remove }

pub fn read_preview(root: &Path) -> Result<Preview, KernelError>;          // nur Dateien, kein Project::open
pub fn write_preview(root: &Path, slot: PreviewSlot, input: PreviewInput) -> Result<Preview, KernelError>;
pub fn fit_preview(bytes: &[u8]) -> Result<Vec<u8>, KernelError>;         // PNG oder JPEG → PNG 256×192
pub fn standard_map(project: &Project) -> Option<StandardMap>;            // { header, map, area_data }
```

- **Einpassen:** decodieren (PNG oder JPEG; erkannt am Inhalt, nicht an der Endung), auf 4 : 3 **beschneiden** (mittig, „cover“) und auf 256 × 192 skalieren (Lanczos3 beim Verkleinern, Nearest beim Vergrößern ganzzahliger Vielfacher, sonst CatmullRom), Alpha auf `szene/ds/grund` legen, als PNG RGB8 schreiben. Ein Bild, das schon 256 × 192 ist, wird nur umkodiert.
- **Grenzen:** Datei höchstens 32 MiB, Bild höchstens 8192 × 8192. Fehler als `KernelError::Refused` mit festen Codes im Text: `preview.format` (weder PNG noch JPEG), `preview.too_large`, `preview.unreadable` (Datei fehlt oder ist kaputt). Die App übersetzt die Codes in deutsche Sätze mit Ausweg.
- **Schreiben** atomar über eine Temp-Datei in `.g4/` und `rename` (`fsutil`), legt `.g4/` bei Bedarf an. `Remove` löscht die Datei des Slots, fehlt sie, ist das kein Fehler.
- **Abhängigkeit:** `image` mit `default-features = false, features = ["png", "jpeg"]` (MIT/Apache-2.0, `cargo deny` prüft). Das bestehende `png` bleibt für die übrigen Stellen.
- **Standardkarte:** aus dem Rezept des Spiels (`templates`, Feld `start_town`, interner Name des Start-Headers in Matrix 0: Platin `T01` Zweiblattdorf, HGSS `T20` Neuborkia, so wie die Rezepte sie heute nennen) der Header, seine erste Zelle in Matrix 0, deren Karte und die AreaData des Headers. Ohne Rezept (Diamant/Perl) oder ohne Zelle `None`.
- Kein Undo: das Vorschaubild ist keine Spieldatei und gehört nicht in die Historie; „entfernen“ und „ersetzen“ sind sofort auf Platte (wie Einstellungen).

### 3.3 Befehle

Zwei Befehle, gleich in Tauri (`commands/preview.rs`), Brücke (`dispatch.rs`) und MCP:

**`project_preview_get { path?: string }`** → `PreviewDto`

```ts
type PreviewDto = {
  source: "eigen" | "standard" | null;
  png: string | null;              // Base64 eines PNG 256×192
  standardMap: { header: number; map: number; areaDataId: number } | null;  // nur für das offene Projekt
};
```

Ohne `path` gilt das offene Projekt (Fehler ohne offenes Projekt). Mit `path` liest der Befehl nur die zwei Dateien unter `<path>/.g4/`, ohne das Projekt zu öffnen und ohne den Zustand der App zu berühren; `standardMap` ist dann `null`, außer `path` ist das offene Projekt. Base64 statt Bytes, weil die Antwort auch `source` trägt und in allen drei Wegen gleich JSON bleibt; ein Bild ist um 50 KB.

**`project_preview_set { path?: string; slot?: "eigen" | "standard"; png?: string; file?: string }`** → `PreviewDto`

Höchstens eins von `png` (Base64, beliebiges Maß, wird eingepasst) und `file` (Pfad zu PNG oder JPEG); keins von beiden entfernt das Bild des Slots. `slot` ist `"eigen"`, wenn er fehlt. Die Antwort ist der Stand nach dem Schreiben.

- **Brücke:** `path` und `file` laufen durch die Wurzelprüfung (`path_text`), wie `inspect_rom`.
- **MCP:** Werkzeuge `project_preview_get` und `project_preview_set` im Katalog (`catalog.rs`, Route `Command`), ohne `path` im Schema (der Server ist an ein Projekt gebunden; ein `path` wird abgelehnt). `file` muss im Projekt liegen oder unter den freigegebenen Wurzeln. Zugriffstabelle `access.rs`: `project_preview_get` `Read`, `project_preview_set` `Irreversible("Vorschaubild ändern")` (es gibt kein Undo; die Freigabe des Nutzers entscheidet wie bei den anderen nicht rückgängigen Befehlen).
- `docs/mcp.md` nennt die zwei Werkzeuge.

### 3.4 Wann das Standardbild entsteht

Nach dem Öffnen eines Projekts, wenn die Welt geladen ist und der Rechner ruhig ist (`requestIdleCallback` oder nach dem ersten Bild der Szene), fragt die App `project_preview_get` ohne `path`. Mit `standardMap` zeichnet sie das Kartenbild der Startstadt mit dem Draufsicht-Renderer der Kartenbilder (`world/thumbnailRenderer.ts`, neu mit Breite und Höhe: 256 × 192, orthografisch, die Kartenbreite füllt das Bild, oben und unten je ein Achtel abgeschnitten) und schreibt es mit `slot: "standard"`, wenn es sich vom gespeicherten unterscheidet. Fehlt apicula oder schlägt das Zeichnen fehl, bleibt alles, wie es ist, ohne Meldung (die Kartenbilder melden apicula schon in Technik). Ein Projekt, das nie geöffnet wurde, hat daher kein Standardbild und zeigt den Namen (G4). Das neue Spiel aus ROM (3g) bekommt sein Standardbild beim ersten Öffnen, das direkt auf das Anlegen folgt.

### 3.5 Oberfläche

**Projektkarte auf Start** (`ui/frame/StartPane.tsx`, neue Komponente `ProjectPreview` daneben): Das Bild liegt im DS-Rahmen an der Stelle des leeren Bildschirms (`DSScreen` mit dem Bild als `background`, `<img alt="">`; den Namen trägt weiter die Zeile unter dem Bildschirm, `image-rendering: pixelated` bei ganzzahligem Maßstab). Bis das Bild da ist oder wenn es keins gibt: der Name auf dem leeren Bildschirm wie heute (Platzhalter). „Zuletzt“ und „nicht gefunden“ (grau, ohne Farbe) bleiben unverändert. Die Startliste liest die Bilder über `project_preview_get { path }` für alle Karten, die nicht als fehlend bekannt sind, jedes Mal, wenn Start sichtbar wird; ein Fehler gilt als „kein Bild“.

Menü der Karte (`Weitere Aktionen für <Name>`): „Vorschaubild wählen…“, „Vorschaubild entfernen“ (nur mit eigenem Bild), Trenner, „Aus der Liste nehmen“. Eine fehlende Karte hat nur „Aus der Liste nehmen“. „Vorschaubild wählen…“ öffnet den Dateidialog „Vorschaubild wählen“ mit dem Filter „Bilder (PNG, JPEG)“ (`openPath` mit `filters`; im Browser-Modus das Pfadfeld der Test-Umgebung). Danach ein Toast „Vorschaubild gesetzt“ oder „Vorschaubild entfernt“; ein Fehler als Toast mit dem Satz des Codes:

| Code | Satz |
|---|---|
| `preview.format` | „Das Bild ist weder PNG noch JPEG. Wähle eine PNG- oder JPEG-Datei.“ |
| `preview.too_large` | „Das Bild ist zu groß (höchstens 32 MB und 8192 × 8192 Pixel). Wähle ein kleineres Bild.“ |
| `preview.unreadable` | „Das Bild lässt sich nicht lesen. Wähle eine andere Datei.“ |

**Technik** (`technik/TechnikArea.tsx`): ein Abschnitt „Vorschaubild“ (oben, vor den Werkzeugen) mit `ProjectPreview` in 256 × 192, darunter „Vorschaubild wählen…“ (sekundär), „Entfernen“ (sekundär, nur mit eigenem Bild) und grau der Satz „Ohne eigenes Bild zeigt Start das Kartenbild der Startstadt.“ (mit Namen, wenn bekannt: „… von Zweiblattdorf.“).

**Szene:** „Als Vorschaubild“ im Reitermenü der Schau-Reiter (`ui/frame/dockMenu.ts`), unter „Maximieren“ und einem Trenner: am Reiter „3D“ setzt es das Bild der 3D-Ansicht, am Reiter „Weltkarte“ das Kartenbild der gewählten Karte (gesperrt ohne gewählte Karte, Tooltip „Erst eine Karte wählen“). In der Palette zwei Befehle: „Als Vorschaubild: 3D-Ansicht“ und „Als Vorschaubild: Kartenbild der gewählten Karte“ (gesperrt mit demselben Hinweis wie die anderen Kartenbefehle). Die 3D-Ansicht zeichnet ein Bild ohne die DOM-Leisten (Werkzeugleiste, Szenenleiste liegen ohnehin außerhalb des Canvas) und liefert es als PNG (`SceneHost.capture()`); der Kernel bringt es auf das Maß. Danach der Toast „Vorschaubild gesetzt“.

### 3.6 Design-System

- `design-system/README.md`: Abschnitt „Marke“ (§2), Komponente „Projektkarte“ in der Tabelle der Komponenten (Zustände: Bild eigen, Standardbild, Platzhalter mit Name, Zuletzt, nicht gefunden, Menü), Abschnitt „Nachtrag 4.0“ mit Verweisen.
- Galerie „Projekt“ (`ui/gallery/pages/ProjectPage.tsx`): die Projektkarte in allen Zuständen mit Beispielbildern, die die Galerie selbst zeichnet (Farbflächen aus den Szenen-Tokens; keine Spielgrafik), und der Abschnitt „Vorschaubild“ von Technik.
- `.fig`: Seite „App: Start und Welt“, der Screen „Start“ mit Bildern auf den Karten, und das Brett „Marke“ (§2.2). Neue Tokens braucht es nicht.
- `scripts/check-styles.sh` und `check-tokens.sh` bleiben grün: nur Bausteine und Tokens, keine Hex-Werte in CSS.

## 4. Tests und Prüfung

- **Kernel:** `fit_preview` (PNG und JPEG, 4 : 3, 1 : 1, 16 : 9, Alpha, schon passend), `read_preview` (eigen vor Standard, kaputte Datei zählt nicht), `write_preview` (atomar, Remove ohne Datei), Grenzen und Codes, `standard_map` auf dem synthetischen Projekt; gated auf frischen Kopien: Platin DE und HeartGold DE geben Zweiblattdorf und Neuborkia.
- **Tauri und Brücke:** `dispatch`-Tests für beide Befehle (offenes Projekt, `path` unter der Wurzel, `path` außerhalb abgelehnt, `file` außerhalb abgelehnt); die bestehenden Vollständigkeitstests der Tabellen (`COMMANDS`, `COMMAND_KINDS`, Katalog) decken die neuen Einträge.
- **vitest:** `ProjectCard` mit Bild, Platzhalter, fehlend; Menü der Karte (Einträge je Zustand, Wählen ruft Dialog und `set`, Entfernen); Technik-Abschnitt; Reitermenü der Schau und Palette (gesperrt ohne Karte); der Renderer in 256 × 192; das Erzeugen des Standardbilds (nur bei Abweichung schreiben, Fehler still).
- **UI-Prüfung über die Test-Umgebung** (`docs/agent-gate.md`, Ports 1490/1491, nur frische Kopien unter `Roms/_testkopien/4-0-<zweck>`): G1 bis G4, Bilder unter `target/agent/gate-2026-10-27/`.
- **Bundle:** `check-bundle.sh` nach `pnpm bundle`, wenn die Maschine es erlaubt; sonst nennt das Gate-Dokument es als offenen Punkt.
- `scripts/gate.sh` im Worktree grün (`G4_ROMS_DIR` absolut, Symlink `tools`).

## 5. Entscheidungen

| Frage | Gewählt | Verworfen, weil |
|---|---|---|
| Wo das Standardbild entsteht | in der App beim Öffnen, gespeichert als `.g4/preview-standard.png` | Kernel kann ohne WebGL kein Kartenbild zeichnen; ein App-Cache außerhalb des Projekts wäre für die Startliste „über den Pfad“ ein zweiter Ort |
| Eigen und Standard in einer Datei | zwei Dateien | mit einer Datei wüsste die App nach „entfernen“ nicht, ob das Bild eigen war, und hätte sofort kein Bild |
| Einpassen | beschneiden (cover) auf 4 : 3 | Einrahmen mit Balken sieht auf dem DS-Bildschirm wie ein Fehler aus |
| Wer einpasst | der Kernel, für alle Quellen | eine Stelle für Maß, Format und Grenzen; MCP und Brücke bekommen dasselbe |
| Antwortformat | JSON mit Base64 | Bytes allein trügen `source` nicht; drei Wege, ein Format |
| Zugriff für Agenten | `Irreversible` | ohne Undo darf ein Agent das Bild nur mit Freigabe ersetzen |
| Marke | Doppeldeck mit Kobalt-Scharnier, ohne Ziffer | eine „4“ oder ein Pokéball-artiges Zeichen wäre klein unleserlich bzw. zu nah an fremden Marken |
