# Changelog

Alle nennenswerten Änderungen an Gen4 Studio stehen in dieser Datei.

Das Format folgt [Keep a Changelog](https://keepachangelog.com/de/1.1.0/), die Versionen folgen
[Semantic Versioning](https://semver.org/lang/de/). Die Fassung steht an einer Stelle,
`version` in `[workspace.package]` der `Cargo.toml`; `bash scripts/version.sh set <Fassung>`
hält `apps/studio/package.json` und `Cargo.lock` gleich. Der Release-Workflow nimmt den
Abschnitt der Fassung als Text des Release-Entwurfs.

## [0.1.0] - unveröffentlicht

### Hinzugefügt

- Phase 1: Kernel-Fundament und Shell-Spike: Projekt öffnen, Permissions malen, speichern, ROM bauen und im Emulator laufen lassen (HeartGold, SoulSilver, Platin).
- Phase 2 und 3a: Welt-Editor mit Header, Matrix, Gebäuden, Events, Warps und 3D-Modellen; Welt-Explorer, Assistenten für neue Gebiete und Räume, Weltkarte.
- Phase 3b: Neue Header per Patch, Scripts mit rotom und CodeMirror 6, Text-Editor, Daten-Editoren (Pokédex, Attacken, Gegenstände, Trainer, Begegnungen), Prüfen und Beheben, Import fertiger Kartenteile.
- Phase 3c und 3e: Design-System in der App mit Tokens und Komponentenbibliothek, Rahmen aus Titelleiste, Atlas, Liste und Schau; Werkbank mit Weltbaum und Dock je Bereich.
- Phase 3f und 3g: Weltmodell mit eigenen Orten als Ortsnamen im ROM; „Neues Spiel aus ROM…“ mit Vorlagen.
- Phase 3h: Agenten-Schnittstelle (MCP): kuratierte Tools über `studio mcp`, kopflos und in der laufenden App, Freigabe je Projekt.
- Phase 4: Textprojekt als Quelle, Build-Graph für „Hier spielen“, Auf Original zurücksetzen, minimales Git im Projekt.
- Phase 3d: Erzählen mit Flags, Variablen, Figuren, Dialogen und Türen; Geschichten lesen und erzählen.
- Release: ein Skript (`scripts/version.sh`) hält die Fassung für Cargo, npm und Tauri gleich.
