# Agent-Testumgebung: App per Browser-Brücke, Spiel per Emulator-Skript

Stand: 2026-10-22. Betrifft die manuellen Gates [phase-3a-gate.md](../../phase-3a-gate.md) und [phase-3b1-gate.md](../../phase-3b1-gate.md) und alle späteren Gates.

## 1. Ziel und Umfang

Die manuellen Gates (Klicks im GUI, Gang in melonDS, Prüfungen auf der Platte) dauern dem Nutzer zu lange. Ein Claude-Agent soll den Großteil selbst abarbeiten:

- **App:** Die echte React-Oberfläche läuft im Browser-Fenster der Claude-Desktop-App. Der Agent klickt dort, liest das DOM und macht Screenshots. Das native Tauri-Fenster kann er nicht bedienen; WebDriver (`tauri-driver`) gibt es für macOS nicht.
- **Spiel:** Ein skriptbarer Emulator ohne Fenster startet das gebaute ROM, spielt mit einem vorbereiteten Spielstand weiter, drückt Tasten, macht Screenshots und liest aus dem RAM die aktuelle Header-ID und Position.
- **Anleitung:** Eine Anleitung sagt dem Agenten Punkt für Punkt, wie er die Gates abarbeitet.

Entscheidungen aus dem Brainstorming:

- Der Agent **klickt selbst** nach Checkliste; es entstehen keine festen Browser-Tests (kein Playwright).
- App-Steuerung über ein **eigenes Dev-Programm `studio-bridge`** (Ansatz A), nicht über einen HTTP-Thread in der laufenden Tauri-App und nicht nur über das CLI `g4k`.
- Spiel über **libretro.py + melonDS DS** (Ansatz A), nicht über eine eigene Rust-Anbindung und nicht über das melonDS-Fenster.
- Die HeartGold-Spielstände legt **der Agent** bei der Umsetzung an, indem er das Intro per Skript durchspielt.

Nicht in dieser Spec: feste Browser-Tests (Playwright), DSPRE-Automatik (Windows), Diamant/Perl, Windows als Testplattform, Änderungen an den Gate-Inhalten selbst (nur die Zuordnung, wer was prüft).

**Fertig heißt:** Mit `.claude/launch.json` startet ein Agent Brücke und Oberfläche im Browser-Fenster, öffnet eine Kopie unter `Roms/_testkopien` und bedient alle Assistenten. `g4emu` bootet Platin DE, HeartGold DE und HeartGold US mit Spielstand bis ins Spielerzimmer und liest die Header-ID. Ein Szenario „Treppe auf neuen Header umbiegen, bauen, hin und zurück“ läuft ohne Klick durch. `docs/agent-gate.md` ordnet jeden Gate-Punkt zu.

## 2. Leitplanken

- Originale (`Roms/*_DSPRE_contents`, `Roms/*.nds`) werden nie geschrieben. Die Brücke lehnt Projekte und Build-Ziele außerhalb der erlaubten Wurzeln ab (§3.4); `g4emu` liest ROMs nur und schreibt Spielstände, Savestates und Screenshots in einen eigenen Ausgabeordner.
- Unter `Roms` wird nichts gelöscht.
- Die Brücke gibt es nur im Dev-Build: eigenes Binary hinter dem Cargo-Feature `dev-bridge`, nicht im App-Bundle (§3.6). Sie lauscht nur auf `127.0.0.1`.
- Der Agent arbeitet in einem eigenen Worktree auf eigenen Ports (Vite 1430, Brücke 1431). Die App des Nutzers (`pnpm tauri dev`, Port 1420) bleibt unberührt und wird nicht neu gestartet.

## 3. Teil 1: `studio-bridge`

### 3.1 Fakten aus dem Code

- Alle 47 Tauri-Befehle (Stand `main` 6e6aa77, inklusive `repair_header_patch`) in `apps/studio/src-tauri/src/commands/*.rs` sind dünne Hüllen um testbare Funktionen `*_impl(&AppState, …)`. `AppState` hält die Sitzung in einem `Mutex` und braucht kein Tauri.
- Vier Befehle brauchen ein `AppHandle`: `get_map_model_glb` und `get_building_glb` liefern rohe Bytes (`tauri::ipc::Response`), `build_rom` läuft auf einem Blocking-Worker, und `force_quit` beendet die App. Die drei ersten haben `*_impl`-Funktionen ohne `AppHandle`.
- Das Frontend ruft alle Befehle über `src/api.ts` auf (`invoke` aus `@tauri-apps/api/core`), dazu `useWindowGuards.ts` (`force_quit`). Tauri übersetzt die camelCase-Schlüssel der Argumente in snake_case-Parameter.
- Native Dialoge stehen an vier Stellen: `ask` (Ja/Nein) in `useWindowGuards.ts`, `useWorldActions.ts`, `useProjectSession.ts` und `MatrixPanel.tsx`; `open` (Ordner und Datei) und `save` in `useProjectSession.ts`. `useWindowGuards.ts` fängt das Fehlen eines Tauri-Fensters schon ab.
- Fehler kommen als `CmdError { code, message, asset }` an. Tauri verwirft das Promise mit genau diesem Objekt.

### 3.2 Aufbau

Ein Modul `apps/studio/src-tauri/src/bridge/` im Crate `studio` (nur mit Feature `dev-bridge` übersetzt) und ein Binary `src/bin/studio-bridge.rs` mit `required-features = ["dev-bridge"]`.

- **`bridge::dispatch(state, name, args: serde_json::Value) -> Result<Reply, CmdError>`** mit `Reply::Json(Value)` oder `Reply::Bytes(Vec<u8>)`. Eine Tabelle, erzeugt von einem kleinen Makro, nennt je Befehl Name, Argumentfelder (camelCase per `#[serde(rename_all = "camelCase")]`, wie Tauri) und den Aufruf der `*_impl`-Funktion. `bridge::COMMANDS` listet die Namen.
- **Abweichungen von Tauri:**
  - `open_project` prüft vorher den Pfad (§3.4).
  - `build_rom` prüft vorher das Ziel (§3.4).
  - `run_rom` antwortet mit dem Fehler „Im Browser-Modus nicht verfügbar, g4emu benutzen“.
  - `force_quit` tut nichts.
  - `get_map_model_glb` und `get_building_glb` liefern `Reply::Bytes`.
- **HTTP-Server:** `tiny_http` (synchron, kleine Abhängigkeit, nur unter dem Feature), vier Worker-Threads, damit ein langes `build_rom` oder apicula das Lesen nicht blockiert.
- **Einmal-Modus ohne Server:** `studio-bridge batch <datei.json> --allow-root <dir>` führt eine Liste `[[befehl, args], …]` in einem Prozess aus und druckt je Befehl eine JSON-Zeile. Gedacht für Szenarien (§4.5) und für Prüfungen ohne Oberfläche. Es geht durch denselben `dispatch`, also auch durch dieselben Pfadprüfungen.

### 3.3 Protokoll

- `POST /invoke/<befehl>` mit einem JSON-Objekt der Argumente (dieselben Schlüssel wie bei `invoke`; ohne Argumente `{}`).
- Antworten:
  - **200** mit `application/json` (Rückgabewert; `null` bei `()`) oder mit `application/octet-stream` (Bytes).
  - **400** mit dem `CmdError` als JSON.
  - **404** bei unbekanntem Befehl.
  - **401, 403 oder 415** bei verletzten Regeln aus §3.4.
- `GET /health` liefert `{"ok":true,"commands":<Anzahl>}` (heute 47) (für Startskripte).

### 3.4 Schutz

- **Bindung:** fest an `127.0.0.1`; der Port kommt aus `--port`, eine Adresse lässt sich nicht einstellen.
- **Host:** der Kopf `Host` muss `127.0.0.1:<port>` oder `localhost:<port>` sein (gegen DNS-Rebinding), sonst 403.
- **Token:** der Kopf `X-G4-Bridge-Token` muss dem Token aus der Umgebungsvariable `G4_BRIDGE_TOKEN` gleichen (vom Startskript zufällig erzeugt, 32 Hexzeichen), sonst 401. Ohne gesetzte Variable startet die Brücke nicht. Die Seite im Browser kennt das Token nicht; der Vite-Proxy setzt den Kopf.
- **Inhaltstyp:** `Content-Type: application/json` ist Pflicht, sonst 415. Damit braucht jede fremde Seite einen CORS-Preflight, den die Brücke nie erlaubt.
- **Origin:** ist der Kopf `Origin` gesetzt, muss er dem Wert von `--origin` gleichen (Standard `http://127.0.0.1:1430`), sonst 403.
- **Erlaubte Wurzeln:** `--allow-root <dir>` ist Pflicht und darf mehrfach stehen.
  - `open_project` nimmt nur Pfade, die nach `canonicalize` unter einer Wurzel liegen.
  - `build_rom` nimmt nur Ziele, deren Elternordner nach `canonicalize` unter einer Wurzel liegt.
  - Zusätzlich lehnen beide jeden Pfad ab, der eine Komponente mit Endung `_DSPRE_contents` enthält, und eine Wurzel mit dieser Endung lässt die Brücke gar nicht zu.
  - Alle anderen schreibenden Befehle schreiben nur ins offene Projekt.

### 3.5 Frontend

- **`src/platform/transport.ts`** exportiert `invoke<T>(cmd, args?)`. In Tauri (`"__TAURI_INTERNALS__" in window`) ruft es das Tauri-`invoke`. Außerhalb von Tauri und nur bei `import.meta.env.DEV` schickt es `fetch("/__bridge/invoke/<cmd>")`:
  - Bei `application/octet-stream` liefert es einen `ArrayBuffer`, sonst das JSON.
  - Bei einem Fehler verwirft es das Promise mit dem `CmdError`-Objekt, genau wie Tauri.
  - Fehlt die Brücke (Netzfehler), verwirft es mit `{code: "error", message: "Brücke nicht erreichbar"}`.
  - Im Produktions-Build ohne Tauri wirft es sofort. Der HTTP-Zweig fällt dort beim Tree-Shaking heraus.
- **`src/api.ts` und `useWindowGuards.ts`** importieren `invoke` aus `transport.ts` statt aus `@tauri-apps/api/core`.
- **`src/platform/dialogs.ts`** exportiert `ask`, `openPath` und `savePath` mit denselben Rückgabewerten wie das Dialog-Plugin. In Tauri leitet es weiter. Im Browser-Modus zeigt ein `WebDialogHost` (in `App` nur außerhalb von Tauri eingehängt) einen Dialog auf der Seite:
  - **Ja/Nein:** Text und zwei Knöpfe „Ja“ und „Nein“.
  - **Pfad wählen:** ein Textfeld „Pfad“ mit „OK“ und „Abbrechen“. Das Feld ist mit dem zuletzt gewählten Pfad vorbelegt (`localStorage`, mit try/catch); bei „Speichern unter“ ist der Vorschlag `<Projekt>/out.nds`.
  - Escape heißt „Nein“ bzw. „Abbrechen“.
  Die vier Aufrufstellen nutzen `dialogs.ts`.
- **Projekt per URL:** Im Browser-Modus öffnet `?project=<pfad>` das Projekt beim Start (über `useProjectSession`, mit derselben Meldung wie beim Öffnen per Dialog).
- **Kennzeichen:** Im Browser-Modus zeigt die Statusleiste „Browser-Modus (Brücke)“, damit Screenshots eindeutig sind.
- **`vite.config.ts`:** Sind `G4_BRIDGE_PORT` und `G4_BRIDGE_TOKEN` gesetzt, leitet ein Proxy `/__bridge/*` an `http://127.0.0.1:<port>/*` weiter und setzt `X-G4-Bridge-Token`. Ohne die Variablen bleibt die Konfiguration wie heute (wichtig für `tauri dev`).

### 3.6 Start und Dev-only

- **`scripts/agent/studio-web.sh`** (bash):
  1. Erzeugt das Token und schreibt es nach `target/agent/bridge.token` (Rechte 600).
  2. Startet `cargo run -p studio --features dev-bridge --bin studio-bridge -- --port 1431 --origin http://127.0.0.1:1430 --allow-root "$G4_ROMS_DIR/_testkopien"` (Standard `G4_ROMS_DIR=../Roms`) und wartet auf `/health`.
  3. Startet dann `pnpm exec vite --host 127.0.0.1 --port 1430 --strictPort` in `apps/studio`.
  4. Beim Beenden stoppt es die Brücke.
  `G4_APICULA` ist standardmäßig `tools/apicula/bin/apicula`, falls die Datei existiert.
- **`.claude/launch.json`** bekommt die Konfiguration `studio-web` (bash `scripts/agent/studio-web.sh`, Port 1430). Der Agent startet sie mit `preview_start` und öffnet `http://127.0.0.1:1430/?project=…`.
- **`scripts/agent/call.sh <befehl> ['<json>']`:** ruft die laufende Brücke per `curl` mit dem Token auf und druckt das JSON (bei Bytes nur die Länge). Das ist der Weg für Prüfungen am Zustand der Oberfläche vorbei, etwa `get_header` nach einem Assistenten.
- **Dev-only:**
  - `tauri build` übersetzt ohne das Feature, das Binary entsteht also nicht. `apps/studio/scripts/check-bundle.sh` prüft zusätzlich, dass im Bundle kein `studio-bridge` liegt.
  - `scripts/gate.sh` prüft nach `pnpm build`, dass `apps/studio/dist` den Text `__bridge` nicht enthält.

## 4. Teil 2: `g4emu`

### 4.1 Fakten aus dem Probelauf (2026-10-22, macOS 26, arm64)

- libretro.py 0.12.0 (PyPI, braucht Python ≥ 3.12) mit dem Kern melonDS DS 1.4.0 läuft ohne Fenster, ohne BIOS (eingebautes FreeBIOS, Direct Boot) und ohne systemweite Installation. Der DeSmuME-Kern stürzte unter libretro.py beim Start ab.
- **Tempo:** etwa 800–1200 Bilder pro Sekunde (JIT, Software-Renderer). Ein Screenshot dauert 25 ms, Savestate speichern 5 ms und laden 18 ms. Ein Savestate ist 19,7 MB groß.
- **Bild:** `ArrayVideoDriver.screenshot()` liefert 256 × 384 Pixel, oberer Bildschirm über dem unteren, Bytefolge BGRX.
- **Touch:** Libretro-Zeigerkoordinaten über das ganze Bild. Ein Tipp muss etwa 6 Bilder gehalten werden und wirkt erst, wenn der Text fertig ausgegeben ist.
- **RAM:** `get_memory(RETRO_MEMORY_SYSTEM_RAM)` liefert die 4 MiB ab `0x02000000`.
- **Spielstand:** Der Kern liest und schreibt keine `.sav`-Datei. Er zeigt den Cartridge-Speicher als `RETRO_MEMORY_SAVE_RAM` (524288 Byte, rohes Flash-Abbild wie `.sav` von melonDS). Das Werkzeug kopiert die Datei vor dem ersten Bild hinein und liest sie bei Bedarf zurück.
- **Platin DE (CPUD):** Die aktuelle Position steht in einem Datensatz `{u32 header; i32 warp; u32 x; u32 y}` an `(u32 @ 0x02101EE0) + 0x1294`. Gemessen: 415/−1/4/6 im Spielerzimmer, 414/1/10/3 unten. Der Zeiger wechselt zwischen zwei Starts, die Kette hält. Feste Adressen ohne Zeiger halten nicht.
- **HeartGold US:** startet bis zum Titelbild. Zeiger und Spielstand fehlen noch.
- Ein Spielstand von Platin DE (Spieler „AAAAA“, gespeichert im Spielerzimmer) liegt aus dem Probelauf vor. Ein frischer Start mit ihm zeigt „WEITER“ und landet im Zimmer.
- Savestates passen nur zum ROM, aus dem sie stammen. Der Patch „Dynamic Headers“ ändert `arm9`, und neue Header ändern Daten. Nach jedem Bauen startet ein Test deshalb **frisch mit dem Spielstand** und wählt „Weiter“.

### 4.2 Ort und Installation

- **Quellcode:** `scripts/g4emu/` (`tools/` steht in `.gitignore`), Python-Paket `g4emu` mit `pyproject.toml`, Abhängigkeiten fest: `libretro.py==0.12.0`, `pillow`, `pyyaml`, dazu `pytest` für die Tests. Verwaltet mit `uv`, Python 3.12.
- **`scripts/g4emu/setup.sh`:**
  1. Ruft `uv sync` auf.
  2. Lädt `melondsds_libretro-macos-arm64-Release.zip` aus dem GitHub-Release v1.4.0 von `JesseTG/melonds-ds` nach `tools/g4emu/core/`.
  3. Prüft die SHA-256-Summe, die im Skript fest steht; weicht sie ab, bricht es ab.
  4. Entpackt den Kern. Der Kern ist danach `tools/g4emu/core/melondsds_libretro.dylib`.
  Andere Quellen gibt es nicht.
- **Spielstände:** `Roms/_testkopien/_saves/<SPIELCODE>-zimmer.sav` (außerhalb des Repos) plus `<SPIELCODE>-zimmer.md` mit Name, Spielzeit und Weg dorthin. `G4EMU_SAVES` überschreibt den Ordner.

### 4.3 Spieltabelle

`scripts/g4emu/g4emu/games.yaml` je Spielcode:

```yaml
CPUD:
  name: Platin DE
  location: { pointer: 0x02101EE0, offset: 0x1294 }  # {u32 header; i32 warp; u32 x; u32 y}
  save: CPUD-zimmer.sav
  start_header: 415
IPKD: …  # bei der Umsetzung ermittelt
IPKE: …
```

Den Spielcode liest `g4emu` aus dem ROM-Kopf (Offset 0x0C). Für Spiele ohne Eintrag gibt es nur Bild und Tasten; `where` und `expect-map` melden dann „Spiel nicht in games.yaml“.

### 4.4 Befehle

`g4emu run --rom <nds> [--save <sav> | --state <state>] --out <ordner> <schritt> …`:

| Schritt | Wirkung |
|---|---|
| `continue` | Titelbild und Menü per A bis „Weiter“, dann warten, bis der Positionszeiger gültig ist und die Header-ID 30 Bilder lang gleich bleibt (Zeitlimit 3000 Bilder). |
| `wait N` | N Bilder laufen lassen |
| `press TASTE[:N]` | Taste N Bilder halten (Standard 6). Tasten: `a b x y l r start select up down left right` |
| `tap X Y` | Tipp auf den unteren Bildschirm (Pixel 0–255, 0–191), 6 Bilder |
| `walk RICHTUNG N` | N Kacheln gehen. Je Kachel die Richtung halten, bis sich x/y um eins ändert (Zeitlimit 64 Bilder). Ein erster Druck, der die Figur nur dreht, zählt nicht als Kachel. Bleibt die Position stehen, ist der Schritt fehlgeschlagen: „blockiert bei x, y“. |
| `walk-to X Y` | Erst x, dann y angleichen, mit `walk`. Ist ein Weg blockiert, wird die andere Achse zuerst versucht. Bleibt die Figur stehen, schlägt der Schritt fehl und nennt die Position. Keine Wegsuche. |
| `expect-map ID [N]` | Bilder laufen lassen, bis die Header-ID gleich ID ist (Zeitlimit N, Standard 600) und danach 30 Bilder stabil bleibt. Sonst Fehlschlag mit der zuletzt gelesenen ID. |
| `where` | druckt `{header, warp, x, y, frame}` |
| `shot NAME` | `NAME.png` im Ausgabeordner |
| `state NAME` | Savestate `NAME.state` im Ausgabeordner |
| `write-save NAME` | Cartridge-Speicher als `NAME.sav` im Ausgabeordner |
| `dump-ram NAME` | 4 MiB RAM als `NAME.bin` (zum Suchen von Adressen) |

Ausgabe und Rückgabe:

- Jeder Schritt druckt eine JSON-Zeile auf stdout (`{"step": …, "ok": …, …}`). Die Konsolenausgabe des Kerns geht in `<out>/core.log`.
- Am Ende schreibt `g4emu` immer `<out>/last.state` und `<out>/last.png`. Ein Folgeaufruf mit `--state <out>/last.state` macht dort weiter; so erkundet der Agent Schritt für Schritt.
- Rückgabe: 0 heißt alles ok, 2 heißt ein `expect-map` oder `walk` ist fehlgeschlagen, 1 heißt ein anderer Fehler (ROM fehlt, Kern fehlt).
- Der Ausgabeordner darf nicht unter `Roms/` außerhalb von `_testkopien` liegen. Die Spielstand-Datei aus `--save` wird nie überschrieben.

Für das Suchen der HG-Adressen gibt es `g4emu find-u16 <wert> <ram.bin> [<wert2> <ram2.bin> …]`. Es druckt alle Adressen, an denen in jedem Abzug der jeweilige Wert steht, dazu je Treffer die Zeiger aus dem festen ARM9-Bereich (`0x02000000–0x02110000`), die auf `treffer − k` mit `k < 0x2000` zeigen. Das ist ein Hilfsmittel und nur leicht getestet.

### 4.5 Szenarien

`g4emu scenario <datei.yaml> --project <kopie> [--set NAME=WERT …] --out <ordner>` verbindet Projekt-Änderung, Bauen und Spielen in einem Lauf:

```yaml
# scripts/agent/scenarios/pt-treppe-warp.yaml
game: CPUD
edits:                       # studio-bridge batch, Platzhalter ${…} aus --set
  - [update_event, {eventFile: 394, kind: warp, index: "${WARP}", patch: {dest_header_id: "${ZIEL}", dest_warp_id: 0}}]
build: out.nds               # build_rom ins Projekt
play:
  - continue
  - expect-map 415
  - walk-to "${TREPPE_X}" "${TREPPE_Y}"
  - expect-map "${ZIEL}"
  - shot angekommen
  - walk-to "${TUER_X}" "${TUER_Y}"
  - expect-map "${ZURUECK}"
```

- Der Ablauf ruft `studio-bridge batch` mit `open_project` und den `edits` auf, dann `save` und `build_rom` (Ziel im Projektordner), danach `g4emu run` mit dem gebauten ROM und dem Spielstand aus `games.yaml`.
- Das Binary der Brücke findet das Szenario über `G4_BRIDGE_BIN`; ohne die Variable baut es `cargo build -p studio --features dev-bridge --bin studio-bridge` und nimmt `target/debug/studio-bridge`. Die erlaubte Wurzel ist der Elternordner von `--project`, sofern er unter `_testkopien` liegt; sonst bricht das Szenario ab.
- Wo die Treppe liegt, ermittelt der Agent aus dem Projekt (Event-Datei des Headers) bzw. mit `where`. Die konkreten Zahlen für Platin trägt die Umsetzung in die Beispieldatei ein.
- Platzhalter: Besteht ein Wert nur aus `${NAME}` und ist der eingesetzte Wert eine ganze Zahl, wird er als Zahl eingesetzt (sonst als Text). Ein fehlender Platzhalter bricht vor jeder Änderung ab.
- Rückgabe wie bei `run`. Am Ende druckt das Szenario eine Zusammenfassung: Header-Folge, Screenshots, Ergebnis.
- Den Typ `update_event` und seine Felder (`EventPatch`) schreibt die Spec nicht neu fest; das Szenario reicht sie unverändert an die Brücke weiter.

## 5. Zuordnung der Gate-Punkte

**Legende:**
- **Agent:** Der Agent erledigt den Punkt allein, im Browser-Fenster, auf der Platte (`ls`, `cmp`, `call.sh`) oder mit `g4emu`.
- **Agent + Blick:** Der Agent prüft und legt Screenshots ab; der Nutzer bestätigt kurz.
- **Hand:** bleibt beim Nutzer.

### Phase 3a ([phase-3a-gate.md](../../phase-3a-gate.md))

| Punkt | Wer | Wie |
|---|---|---|
| (A) 1–10 Assistenten | Agent | Browser-Fenster |
| (A) Stoppuhr und „Zahlen getippt“ | Hand | misst die Bedienung durch den Menschen; der Agent notiert nur seine getippten Zahlen |
| (A) 11 Warps von Hand verbinden | Agent | Browser-Fenster, Warps auf Tür- oder Treppen-Kacheln (siehe Befund unten) |
| (A) 12 Gang im Spiel | Agent | `g4emu`: Haus betreten, verlassen, Route 230 hin und zurück |
| (B) 1–4 Explorer | Agent | Browser-Fenster |
| (C) 1–4 Raster | Agent | Browser-Fenster |
| (D) 1–4 Weltkarte | Agent | Browser-Fenster; (D) 4 auf einer Kopie von HeartGold US |
| (E) 1–2 Automatik | Agent | `scripts/gate.sh` |
| (F) Kartenbilder: Ausrichtung | Agent + Blick | Screenshot Raster neben Screenshot der Draufsicht |
| (F) Kartenbilder ohne apicula | Agent | Brücke mit falschem `G4_APICULA` starten |
| (F) Layout im schmalen Panel | Agent | Panel auf 300 px ziehen, Screenshot |
| (F) Tastatur während eines Assistenten | Agent | Tasten im Browser-Fenster, danach `dirty` über `call.sh` |
| (F) Raum-Assistent, Fehlerfälle | Agent | Browser-Fenster |
| (F) Layout nach Neustart | Agent | Seite neu laden (Layout im `localStorage`) |
| Gebündelte App statt `tauri dev` | Hand | ein kurzer Durchgang im nativen Fenster am Ende |

### Phase 3b.1 ([phase-3b1-gate.md](../../phase-3b1-gate.md))

| Punkt | Wer | Wie |
|---|---|---|
| (A) 1–8 Patch und neuer Header | Agent | Browser-Fenster; Platte mit `ls -l`, `cmp`, `xxd` |
| (A) 9–10 Warp und Gang im Spiel | Agent | Browser-Fenster oder Szenario, dann `g4emu` |
| (A) 11 NARC im ROM | Agent | `g4k`/Platte |
| (A) 12–15 DSPRE-Gegenprobe | Hand | DSPRE läuft nur unter Windows |
| (A) 16 HeartGold DE | Agent | wie (A) 2–10 auf `hg-de-gate`, Spielstand IPKD |
| (A) 17 Diamant/Perl | Hand | entfällt ohne Projekt |
| (A) 18 Patch reparieren | Agent | den fehlerhaften Zustand auf einer Kopie nachstellen: nach „Patch anwenden“ die 4 Byte bei 0x3A0D6 in `arm9/arm9.bin` auf `CC F7 74 FC` setzen (wie DSPRE), dann Meldung, „Patch reparieren“, Bytes und Spielgang prüfen; dass DSPRE selbst so patcht, bleibt Teil der DSPRE-Gegenprobe (Hand) |
| (B) 1–5 Höhen | Agent + Blick | Browser-Fenster (WebGL läuft dort auf der echten GPU), Hover- und Ziehproben im Agent, Optik per Screenshot |
| (C) 1–7 Sprites | Agent + Blick | Agent prüft Erscheinen, Wechsel der Richtung, Platzhalter; aufrecht und Blickrichtung bestätigt der Nutzer am Screenshot |
| (D) 1–3 Kachelnamen HGSS | Agent | Hover im Browser-Fenster |
| (E) 1–2 `gate.sh` | Agent | Skript |
| (F) 1–3 apicula-Zeitlimit | Agent | Brücke mit `G4_APICULA_TIMEOUT_SECS=1` |

Befunde aus dem bisherigen Handdurchlauf, die die Anleitung aufnimmt: Warps feuern nur auf Tür-, Treppen- und Warp-Kacheln. Der Gebiets-Assistent nimmt Header 0 als Vorlage, wenn die erste gewählte Zelle Füllfläche ist. Der von DSPRE übernommene Patch war für Platin DE/FR/IT/ES fehlerhaft; die Korrektur samt „Patch reparieren“ ist seit 6e6aa77 auf `main`. Die Anleitung prüft diese Punkte nicht neu, sondern verweist auf die Problemliste.

## 6. Anleitung `docs/agent-gate.md`

Kurz und auf Deutsch:

1. **Einmalig einrichten:** `scripts/g4emu/setup.sh`; prüfen, dass Spielstände in `_testkopien/_saves` liegen.
2. **Je Lauf:**
   - frische Kopie anlegen (`cp -R` eines Originals nach `_testkopien/<name>`);
   - `preview_start studio-web`;
   - `http://127.0.0.1:1430/?project=<kopie>` öffnen.
3. **Je Gate-Punkt:** was zu klicken ist (Texte der Oberfläche), was auf der Platte oder mit `call.sh` zu prüfen ist, welcher `g4emu`-Aufruf oder welches Szenario passt, und wo Screenshots abgelegt werden (`target/agent/gate-<datum>/`).
4. **Ergebnis:** Der Agent trägt das Ergebnis in den Abschnitt „Ergebnis“ des Gate-Dokuments ein, mit Spalte „Agent“ bzw. „Hand“, und listet die Punkte für den Nutzer (Hand, Agent + Blick) samt Screenshot-Pfaden.

## 7. Tests

**Rust** (Feature `dev-bridge`; läuft in `gate.sh` über `--all-features` mit):

- Die Liste `bridge::COMMANDS` gleicht der Liste in `generate_handler!` (der Test liest `src/lib.rs` per `include_str!` und vergleicht die Namen in beide Richtungen).
- Jeder Befehl geht einmal durch `dispatch` auf dem synthetischen Projekt (`open_synthetic`), mit den camelCase-Schlüsseln aus `api.ts`. Geprüft wird, dass die Argumente ankommen; für die lesenden Befehle wird das Ergebnis mit dem direkten `*_impl`-Aufruf verglichen. GLB-Befehle prüfen nur die Art der Antwort (`Bytes` oder Fehler `tool_missing`).
- Pfade: Projekt außerhalb der Wurzel, Pfad mit `_DSPRE_contents`, Wurzel mit `_DSPRE_contents`, Build-Ziel außerhalb und Symlink aus der Wurzel hinaus werden abgelehnt. `run_rom` meldet „nicht verfügbar“.
- HTTP auf Port 0 mit einem kleinen `TcpStream`-Client: falscher Host 403, fehlendes oder falsches Token 401, falscher Origin 403, falscher Inhaltstyp 415, unbekannter Befehl 404, Fehler 400 mit `code`, Erfolg 200 als JSON und als Bytes, `/health`. Bindung nur an 127.0.0.1.
- `batch`: zwei Befehle in einer Datei, eine JSON-Zeile je Befehl, Abbruch beim ersten Fehler mit Rückgabe ungleich 0.

**Vitest:**

- `transport.ts`: Tauri-Zweig, HTTP-Zweig (gemocktes `fetch`), Fehlerobjekt gleich dem Tauri-Fall, `ArrayBuffer` bei Bytes, Meldung bei fehlender Brücke.
- `dialogs.ts` mit `WebDialogHost`: Ja/Nein, Pfad mit Vorbelegung, Escape heißt Abbrechen.
- `?project=` öffnet das Projekt.
- Bestehende Tests, die `@tauri-apps/api/core` mocken, mocken danach `transport.ts`.

**Python** (`uv run pytest` in `scripts/g4emu`):

- Einheit: Zerlegen der Schritte; Lesen des Positionsdatensatzes über den Zeiger aus künstlichem RAM; `walk` und `walk-to` gegen ein Fake-Emulator-Objekt mit Positionsmodell (Drehen, Blockade, Zeitlimit); `expect-map` mit Zeitlimit; Platzhalter in Szenarien; Pfadregel für den Ausgabeordner; `find-u16`.
- Integration, nur wenn `G4EMU_CORE`, ein ROM und der Spielstand vorhanden sind (sonst `skip`): Platin DE mit Spielstand, `continue`, `expect-map 415`, Weg zur Treppe, `expect-map 414`. Dasselbe für IPKD und IPKE mit ihrem Starthaus.

**`scripts/gate.sh`:**

- Neuer Schritt „g4emu unit“ (`uv run pytest -m "not integration"`).
- Neuer Schritt „g4emu integration“. Er läuft, wenn Kern und Spielstände da sind, sonst Zeile „skipped (kein Kern/Spielstand)“, ohne Abbruch, weil der Teil optional ist.
- Die Prüfung `dist` ohne `__bridge` (§3.6).

## 8. Risiken

- **Zeiger von HeartGold:** Er kann einen anderen Aufbau haben als bei Platin. Die Umsetzung sucht ihn mit `dump-ram` und `find-u16` wie bei Platin und prüft ihn über zwei frische Starts.
- **Zeiger von Platin nach dem Patch:** Ob der Zeiger mit dem Patch „Dynamic Headers“ hält, ist noch nicht geprüft. Der Patch überschreibt Code an fester Stelle und verschiebt keine Daten, deshalb ist das zu erwarten. Die Umsetzung prüft es im Szenario-Test mit neuem Header (Patch-Korrektur ist auf `main`).
- **Kern-Version:** Ein neues Release von melonDS DS kann Optionen umbenennen. Die Version steht fest im Setup-Skript; ein Wechsel ist eine bewusste Änderung mit erneutem Integrationstest.
- **Browser-Fenster:** WebGL und Drag in der 3D-Szene sind dort nicht erprobt. Falls Ziehen per Maus nicht klappt, prüft der Agent Ziehproben über `update_event` per `call.sh` und markiert den Punkt „Agent + Blick“.
- **Tasten:** `Cmd+S` im Browser-Fenster kann die Seite speichern wollen, statt dass die App es abfängt. Die Anleitung nennt das, falls es auftritt.
