# Phase 3e: Gate prüfen

Stand: 2026-10-25. Phase 3e „Werkbank“ baut die Welt-Bedienung in vier Teilphasen um: 3e.1 Scharnier oben, Teiler, Werkzeugleiste; 3e.2 Weltbaum und Sortierung; 3e.3 Hierarchie und Inspector; 3e.4 ein Dock je Bereich. Jede Teilphase hat ihren Teil unten, mit eigener Ergebnistabelle; am Ende stehen das Gesamtgate aus [docs/ROADMAP.md](ROADMAP.md) („Phase 3e: Werkbank“) und die Tabelle „vorher → jetzt“ der Formulierungen, mit denen die älteren Abnahmen (3a, 3b.1 bis 3b.4, 3c) jetzt laufen.

Grundlagen: [docs/research/2026-10-25-ausbau/entscheidungen.md](research/2026-10-25-ausbau/entscheidungen.md) (Tabellen „Werkbank“ und „Weltmodell“), [gutachten-b-werkbank.md](research/2026-10-25-ausbau/gutachten-b-werkbank.md) (Variante C), [gutachten-a-weltmodell.md](research/2026-10-25-ausbau/gutachten-a-weltmodell.md) (Baum, Sortierung). Pläne: [3e.1](superpowers/plans/2026-10-25-phase-3e1-scharnier-teiler-werkzeugleiste.md), [3e.2](superpowers/plans/2026-10-25-phase-3e2-weltbaum-sortierung.md), [3e.3](superpowers/plans/2026-10-25-phase-3e3-hierarchie-inspector.md), [3e.4](superpowers/plans/2026-10-25-phase-3e4-ein-dock-je-bereich.md).

Wer was prüft: Die Spalte „Wer“ sagt **Agent** (Test-Umgebung aus [docs/agent-gate.md](agent-gate.md): Browser-Fenster mit `studio-bridge`, `call.sh`), **Agent + Blick** (der Agent prüft und beschreibt Ort und Zahlen, der Nutzer bestätigt kurz am Bild) oder **Hand** (der Nutzer).

## Vorbereitung

1. Kopien anlegen (nie ein Original öffnen; das Ziel darf nicht existieren), aus dem Ordner `Roms`:
   - `_testkopien/3e-pt` aus `Pokemon - Platin-Edition (Germany)_DSPRE_contents`,
   - `_testkopien/3e-hgd` aus `Pokemon - Goldene Edition HeartGold (Germany)_DSPRE_contents`,
   - `_testkopien/3e-hgu` aus `Pokemon - HeartGold Version (USA)_DSPRE_contents`.
2. Test-Umgebung aus dem Worktree mit eigenen Ports: `G4_ROMS_DIR=<absoluter Pfad zu Roms> G4_VITE_PORT=1440 G4_BRIDGE_PORT=1441 bash scripts/agent/studio-web.sh > target/agent/studio-web.log 2>&1 &`, warten auf „Brücke läuft“, Seite `http://127.0.0.1:1440/?project=<Kopie>`.
3. Vor jedem Teil `localStorage.clear()` und neu laden (jede Teilphase hebt den Layout-Schlüssel an: `v9`, `v10`, `v11`).
4. Fenster 1440 × 900 (Referenz) und 1100 × 720 (Minimum), Thema Hell, Dichte Komfort, wenn nicht anders gesagt.

## Teil 3e.1: Scharnier oben, Teiler, Werkzeugleiste

Lieferumfang (Roadmap): das Scharnier als Kopfleiste über der Schau (ohne Griff), ein eigener Teiler zwischen Schau und Hand (`role="separator"`, Pfeiltasten, Doppelklick klappt ein, `dock-welt-hand` bleibt), die Werkzeuge als schwebende Leiste in der 3D-Ansicht (Standard senkrecht links, an drei Anker, unter 460 px Höhe in Komfort bzw. 400 px in Kompakt waagrecht, `role="toolbar"` und `data-tool` bleiben; Buchstaben wirken nur bei sichtbarem 3D); Galerie „Rahmen“; Design-System: Prinzipien, Fensterraster, „Werkzeugleiste schwebend“, „Teiler“, Token `schatten/schwebend`.

| Punkt | Prüfung | Wer |
|---|---|---|
| (1.1) Reihenfolge | Von oben: Scharnier (`[role=toolbar][aria-label=Scharnier]`), Reiter der Schau, Schau, Teiler `[role=separator][data-splitter=hand]`, Hand `#dock-welt-hand` (`getBoundingClientRect` aneinander) | Agent |
| (1.2) Werkzeugleiste | In 3D schwebt `[role=toolbar][aria-label=Werkzeugleiste]`; `[data-tool=paint]` per Klick und „B“, „V“ zurück, Esc auf „Auswählen“; mit „Raster“ vorn ändert „B“ nichts und der Toast „Werkzeuge wirken in der 3D-Ansicht“ erscheint | Agent |
| (1.3) Teiler | Pfeil hoch +16 px, Pfeil runter −16 px, Ende 200 px, Pos1 560 px, Enter klappt ein (`aria-valuetext` „eingeklappt“) und wieder aus | Agent |
| (1.4) Script hebt die Hand | Hand auf 200 px, Scriptdatei öffnen → Hand ≥ 400 px | Agent |
| (1.5) Layout-Schlüssel | nach `localStorage.clear()` nur `g4.layout.v9.*`, keine `v8` | Agent |
| (1.6) Fenster 1100 × 720 | Leiste waagrecht rechts neben der Ebenen-Leiste, keine Überdeckung | Agent |
| (1.7) Anker merken | Menü „Lage der Werkzeugleiste“ › „Unten“: Leiste unten mittig, nach Neuladen weiter unten (`g4.shell.v1` `toolbar`) | Agent |
| (1.8) Galerie „Rahmen“ | `?gallery`, Seite „Rahmen“: Scharnier oben mit Teiler, Werkzeugleiste senkrecht, waagrecht, gesperrt | Agent + Blick |

### Ergebnis 3e.1

| Datum | Projekt | Punkt | Ergebnis | Auffälligkeiten |
|---|---|---|---|---|
| 2026-10-25 | 3e-pt | (1.1) | bestanden: Scharnier 40–84, Reiter 84–120, Schau bis 578, Teiler 578–586, Hand 586–872 (1440 × 900) | – |
| 2026-10-25 | 3e-pt | (1.2) | bestanden: Klick „Gelände malen“ → `paint`, V → `select`, B → `paint`, Esc → `select`; Raster vorn, B → bleibt `select`, Toast „Werkzeuge wirken in der 3D-Ansicht“ | Bei 1440 × 900 mit Hand 286 ist die Schau 458 px hoch: die Leiste liegt in Komfort waagrecht (Schwelle 460, siehe Befund 3) |
| 2026-10-25 | 3e-pt | (1.3) | bestanden: 286 → 302 → 286 (zwei Tastendrücke im selben Takt zählten einmal, siehe Befund 4), Enter klappt ein und aus, Ende 200 | – |
| 2026-10-25 | 3e-pt | (1.4) | bestanden: Scripts übernommen (`adopt_scripts`, 1124 Dateien), Scriptdatei 1052 geöffnet, Hand 200 → 400 | – |
| 2026-10-25 | 3e-pt | (1.5) | bestanden: `g4.layout.v9.welt.schau`, `g4.layout.v9.welt.hand` | – |
| 2026-10-25 | 3e-pt | (1.6) | bestanden: Leiste waagrecht ab x 454, Ebenen-Leiste endet bei x 446 | Ein Größenwechsel der Emulation von 1440 × 900 auf 1100 × 720 bei offenem Script ließ das Raster-Panel über die Hand ragen; nach Neuladen korrekt. Wird mit dem Dock aus 3e.4 erneut geprüft |
| 2026-10-25 | 3e-pt | (1.7) | bestanden: `data-anchor="bottom"`, `g4.shell.v1.toolbar = "bottom"`, nach Neuladen unten mittig | – |
| 2026-10-25 | – | (1.8) | Agent: drei Leisten und der Teiler da (Test `FramePage.test.tsx`); Blick beim Nutzer offen | – |

Befunde von 3e.1 (in der Teilphase behoben bzw. weitergegeben):

1. Die Leiste stieß bei 400 bis 455 px Schauhöhe in Komfort an die Legende; Schwelle je Dichte: Komfort 460, Kompakt 400 (Commit `1040d4d`).
2. Griff und Menü teilen sich keine Taste (Base UI öffnet ein Menü schon beim Drücken): der Griff `[data-toolbar-grip]` zieht nur (für Hilfstechnik verborgen), die Taste „Lage der Werkzeugleiste“ öffnet das Menü „Links“, „Oben“, „Unten“.
3. Die Referenzhöhe 900 px ergibt mit offener Hand (286) eine Schau von 458 px, also eine waagrechte Leiste. Mit der Anordnung „Werkbank“ aus 3e.4 (Script und Probleme unten 240 px) wird die Schau höher; dort erneut prüfen.
4. Zwei Tastendrücke am Teiler, die vor dem nächsten Rendern ankommen, rechnen mit derselben alten Höhe; das Öffnen einer eingeklappten Hand beim Ziehen kann doppelt umschalten. Behoben mit dem Baustein `Splitter` in 3e.3 (Task 1).
