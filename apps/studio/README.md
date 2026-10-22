# Gen4 Studio (App)

Tauri-App (React, TypeScript, three.js) über dem Kernel `g4-kernel`. Entwicklung, Tests und Bundle: [docs/dev-setup.md](../../docs/dev-setup.md) und [docs/release.md](../../docs/release.md). Empfohlene IDE: VS Code mit den Erweiterungen Tauri und rust-analyzer.

## Panels

Die Workbench (Dockview) hat sechs Panels; das Layout wird gespeichert (`g4.layout.v3`).

| Panel | Inhalt |
|---|---|
| Welt | Die Header als Orte: Baum (Ort, Außen, Räume) mit Suche, Umschalter "Baum \| Liste", "+ Neu" für die Assistenten (Gebiet anlegen…, Raum anlegen…) und "Begriffe" (Glossar). |
| 3D | Die Szene des gewählten Headers: Karten mit Texturen, Gebäude, Events, Warps; Kacheln malen, Gebäude und Events ziehen. |
| Raster | Das Raster (Matrix) des Headers mit Kartenbildern; die gestrichelten "+"-Zellen erweitern es um eine Karte. |
| Weltkarte | Die Region mit Ortsfarben und Labels, die Räume je Ort und Linien von den Räumen zu ihren Eingängen; Klick springt zum Header. |
| Inspector | Header, Raster-Zelle, Area-Data, Gebäude und Events bearbeiten; im Header-Editor der Abschnitt "Eingänge / Ausgänge" (Warps, "Raum hier anbinden…"). |
| Status | Meldungen, Stale-Aktionen, Ladefortschritt. |

Welt, Raster und Weltkarte sind aus den Orten und Rastern abgeleitet, die der Kernel liefert (`list_matrices`, `list_warps`); die Assistenten rufen `create_area`, `extend_area` und `create_room` auf. Anlegen neuer Karten, Raster und Eventdateien lässt sich nicht rückgängig machen, die anschließenden Header-, Zellen- und Warp-Änderungen schon. Hintergrund: [Spezifikation Phase 3a](../../docs/superpowers/specs/2026-10-22-world-explorer-design.md).

## Befehle

```sh
pnpm test        # vitest
pnpm build       # tsc und vite
pnpm tauri dev   # mit G4_APICULA, siehe docs/dev-setup.md
```
