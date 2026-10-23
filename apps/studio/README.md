# Gen4 Studio (App)

Tauri-App (React, TypeScript, three.js) über dem Kernel `g4-kernel`. Entwicklung, Tests und Bundle: [docs/dev-setup.md](../../docs/dev-setup.md) und [docs/release.md](../../docs/release.md). Empfohlene IDE: VS Code mit den Erweiterungen Tauri und rust-analyzer.

## Aufbau der Oberfläche

Die App folgt dem Design-System „Doppeldeck“ ([design-system/README.md](../../design-system/README.md)): Titelleiste, Atlas (Bereiche), Liste, Deck (Schau oben, Hand unten, dazwischen das Scharnier) und Statusleiste. Das Deck besteht aus zwei Dockview-Instanzen; ihr Layout wird je Deck gespeichert (`g4.layout.v7.welt.schau` und `….hand`). Wer das Panel-Set eines Docks ändert, zählt den Schlüssel hoch (`layoutKey` in `src/workbench/layout.ts`).

| Fläche | Inhalt |
|---|---|
| Atlas | Start und Welt; Geschichten, Pokémon, Gegenstände, Texte, Technik und Einstellungen sind gesperrt (kommen mit 3b.3 und 3c.2); „Hilfe“ öffnet das Glossar. |
| Liste „Orte“ | Die Header als Orte: Baum (Ort, Außen, Räume) mit Suche, Umschalter "Baum \| Liste", Menü "Neu anlegen" für die Assistenten (Gebiet anlegen…, Raum anlegen…), der Knoten "Scripts". |
| Schau | Reiter 3D (Szene des gewählten Headers: Karten mit Texturen, Gebäude, Events, Warps), Raster (Matrix mit Kartenbildern; die gestrichelten "+"-Zellen erweitern sie um eine Karte) und Weltkarte (Region mit Ortsfarben, Räume je Ort, Linien zu ihren Eingängen; Klick springt zum Header); darüber die Szenenleiste mit "Gelände", "Ereignisse" und der Legende. |
| Scharnier | Brotkrume, Tiefe "Bauen" und "Tüfteln", Prüflicht, Verlauf, Wiederholen, "Hier spielen". |
| Hand | Werkzeuge (V B F T A S G) \| Inspector mit den Reitern Text, Script und Probleme \| "An diesem Ort". Der Inspector bearbeitet Header (mit "Eingänge / Ausgänge": Warps, "Raum hier anbinden…"), Raster-Zelle, Area-Data, Gebäude und Events. |
| Statusleiste | Hinweise des Werkzeugs, Kontext, Build-Stand, Dichte. Meldungen und Ladefortschritt erscheinen als Toasts. |

Der Code liegt in `src/ui/` (Tokens, Primitives, Rahmen, Galerie), `src/workbench/` (Panels und Editoren) und `src/world/` (three.js-Szene). Styling nur über CSS Modules und `--g4-*`-Variablen; `scripts/check-styles.sh` im Gate prüft das (Regeln in [docs/dev-setup.md](../../docs/dev-setup.md), Abschnitt "Design-System und Tokens").

Welt, Raster und Weltkarte sind aus den Orten und Rastern abgeleitet, die der Kernel liefert (`list_matrices`, `list_warps`); die Assistenten rufen `create_area`, `extend_area` und `create_room` auf. Anlegen neuer Karten, Raster und Eventdateien lässt sich nicht rückgängig machen, die anschließenden Header-, Zellen- und Warp-Änderungen schon. Hintergrund: [Spezifikation Phase 3a](../../docs/superpowers/specs/2026-10-22-world-explorer-design.md).

Objekt-Events zeigen ihr Overworld-Sprite als Billboard in Blickrichtung (gelbes Quadrat, solange es lädt oder keines bekannt ist), und die Höhen der Matrix heben ihre Zellen samt Modell, Gebäuden, Markern und Sprites an (eine halbe Kachel je Stufe); Hintergrund: [Spezifikation Phase 3b.1](../../docs/superpowers/specs/2026-10-22-phase-3b1-foundation-design.md).

## Befehle

```sh
pnpm test        # vitest
pnpm build       # tsc und vite
pnpm tauri dev   # mit G4_APICULA, siehe docs/dev-setup.md
```
