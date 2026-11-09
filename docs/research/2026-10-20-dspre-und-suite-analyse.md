# Von DSPRE zur Pokémon-Gen-4-Engine: Analyse und Architekturvorschlag

> **Einordnung:** Dieses Dokument ist die interne Architekturbewertung vom Oktober 2026, geschrieben als Entscheidungsgrundlage für Gen4 Studio, und keine Kritik am Projekt DSPRE, das seither weitere Releases veröffentlicht hat und dessen heutigen Stand die Analyse nicht abbildet. Gen4 Studio teilt keinen Code mit DSPRE (DSPRE steht unter AGPL-3.0, Gen4 Studio unter MIT) und öffnet DSPRE-Projekte über das gemeinsame Ordnerformat.

Stand: 20. Oktober 2026. Grundlage: Code-Analyse von DSPRE `main` (Release 2.3.4) und `feature/avalonia` (Commit 94455d89), GitHub-Org `DS-Pokemon-Rom-Editor`, Ökosystem-Recherche (pret-Decomps, hg-engine, Pokémon Studio, Plugin-Architekturen). Vier Subagenten haben die Rohdaten geliefert, zentrale Aussagen wurden stichprobenartig im Code nachgeprüft.

---

## 1. Kurzfazit

**Das Domänenwissen ist da, die Architektur nicht.** DSPRE enthält das umfassendste Gen-4-Formatwissen, das es gibt: Lese- und Schreibpfade für praktisch jede ROM-Struktur, inzwischen auch NSBMD/NSBTX-Writer, Tile-zu-Modell-Baking und eine tiefe hg-engine-Quelltextintegration. Der Avalonia-Branch hat die alten Blocker (Tao.OpenGL, WPF, .NET Framework) beseitigt und ist cross-platform.

Aber: Beide Branches sind statische Single-Project-Monolithen. Der Core kennt kein Projektobjekt, nur rund 300 globale statische Felder. Es gibt keinen DI-Container, keine Editor-Registry, keine Contribution Points, kein Plugin-Loading, und die rund 100 Editoren sind in einer 1.790-Zeilen-Launcher-Datei fest verdrahtet. "Everything as a Plugin" lässt sich nicht als Schicht obendrauf setzen. Erst muss der Core ein instanzbasierter, transaktionaler Kernel werden.

**Empfehlung:** Keine Greenfield-Engine und kein bloßer Fork. Stattdessen DSPRE.Core schrittweise zum Kernel umbauen ("Strangler"-Ansatz), die Rust-Werkzeuge (ds-rom, rotom, chatot, uxie) als Out-of-Process-Toolchain behalten, und eine neue Shell mit Docking und Contribution-API bauen, auf der die bestehenden Editoren Stück für Stück als First-Party-Plugins laufen. Der entscheidende "Engine"-Beitrag, den noch niemand hat, ist ein textuelles Quellformat für Karten, Kollision und Gebäude mit Build-Pipeline zu NSBMD.

---

## 2. Ist-Zustand

### 2.1 DSPRE `main` (WinForms, .NET Framework 4.8)

Wartungsmodus. Etwa 121k handgeschriebene Zeilen, davon 17k toter Code (`TextDatabase.cs`). Keine Unit-Tests. CI baut nur.

Kennzahlen zur Kopplung:

| Messgröße | Wert |
|---|---|
| Statische Properties in `RomInfo.cs` | ca. 150 |
| Referenzen auf `RomInfo.` | 1.204 in 102 Dateien |
| `MessageBox`-Aufrufe gesamt | 1.125 in 132 Dateien |
| ROMFiles-Klassen, die `System.Windows.Forms` importieren | 18 von 41 |
| Hartkodierte Backslash-Pfade | 133 |
| Dateien über 2.000 Zeilen | 8 (EventEditor 3.191, NSBMDGlRenderer 3.029, RomInfo 2.718, …) |

Offsets pro Spiel und Sprache liegen als C#-`switch`-Blöcke in `RomInfo` (etwa 238 Sprach-Verzweigungen). Der ROM-Toolbox-Patch-Mechanismus besteht aus Hex-Strings in Dictionaries plus imperativem Code in einem 1.649-Zeilen-Dialog. Die Ekona-`IPlugin`-Schnittstellen sind Tinke-Erbe und werden von nichts geladen.

Wert von `main` heute: Referenz für Formatwissen und als Vergleichsbasis für Round-Trip-Tests. Als Architekturbasis ungeeignet.

### 2.2 DSPRE `feature/avalonia` (die eigentliche Zukunft)

Seit Juni 2026, 710 Commits vor `main`, 2.211 Commits insgesamt, davon 724 seit Januar von einer Person (Mixone-FinallyHere). Alle offenen Issues sind mit "Fixed in avalonia" markiert, d.h. das Team behandelt den Branch als kommendes `main` (Version 3.0, "work in progress").

**Struktur:**

| Projekt | Inhalt | Umfang |
|---|---|---|
| `DSPRE.Core` (net8.0) | Formate, `RomInfo`, `RomLocator`, Scripts, Text, `HgEngine/` | 417 Dateien, ~100k LOC |
| `DSPRE.Avalonia` (net8.0, Avalonia 12.1) | Views, ViewModels, GL-Renderer, `Avalonia/Data` | 534 .cs + 140 .axaml, ~153k LOC |
| `DSPRE.Tests` (xUnit) | Round-Trip-Tests, brauchen echte ROMs | 146 Testmethoden |

**Was gut ist:**

- **Durchgesetzter Hausstil.** `DSPRE-Development.md` (609 Zeilen) plus `BannedSymbols.txt`-Analyzer: Spieldateien dürfen nur über `ArchiveMembers`/`DSUtils.WriteToFile` geschrieben werden, `System.Drawing` ist verboten, `var` bricht den Build. Konventionen: ein Reader pro Format, `WhyNot()`-Gating, `PendingArchiveWrites` als Staging mit Undo, `IEditorWithUnsavedChanges`.
- **Moderner Renderer.** `Gl/NsbmdGlControl.cs` auf `OpenGlControlBase` mit VAO/VBO und GLSL 330/300es. Kein Immediate Mode, kein OpenTK, läuft auf Windows und Linux.
- **Welt-Editing geht über ROM-Bearbeitung hinaus.** `Models/NsbmdWriter.cs`, `NsbtxWriter.cs`, `BdhcBuild`, `TileGrid`/`TilePainter`/`TileBake` (PDSMS-artiges Tile-Malen, das zu NSBMD gebacken wird), OBJ-Import, Geometrie-Editor. Karten können im Editor erzeugt werden, nicht nur extern.
- **hg-engine als Quelltext-Backend.** 88 Dateien in `Core/HgEngine`: C-Tokenizer, Symboltabelle, Konstantenauswerter, `HgEngineSourcePatcher` für Feldpfad-Schreibzugriffe in Designated Initializers, `HgEngineDomains` (8 Domänen mit Quelldatei, make-Target und gespeisten NARCs), `HgEngineSync` (mtime-basiertes Teil-Rebuild, Re-Extraktion in die normalen Ordner, damit Vanilla-Editoren unverändert weiterarbeiten), `HgEngineVerifiedWrite` (jede Änderung wird nach dem Schreiben re-geparst). Das ist bereits das Muster "Quell-Repository plus Build = Wahrheit, ROM = Artefakt".
- **Rust-Toolchain integriert.** rotom (Script-Compiler) inklusive `rotom-lsp` als LSP-Client in AvaloniaEdit, chatot für Text, ds-rom für Extraktion/Build, apicula für Export. Submodule in `Tools/`.
- **Feld-Runtime-Simulation.** `ROMFiles/Field*.cs` plus `AnimatedPreviewViewModel`: NPCs laufen, Wasser animiert, Scripts laufen teilweise. Ein Keim für In-Editor-Playtest.
- **Round-Trip-Tests.** `Core/UnchangedSaveTests*.cs` laden jeden Datensatz und speichern byte-identisch zurück.

**Was fehlt oder stört (für das Engine-Ziel entscheidend):**

| Befund | Beleg | Konsequenz |
|---|---|---|
| Kein Projektobjekt, nur globaler Zustand | `new RomInfo(id, folder)` mutiert ~119 Statics; 223 Dateien mit `using static DSPRE.RomInfo`; ~315 mutable Statics im Core | Ein Projekt pro Prozess. "Projekt in neuem Fenster" startet einen neuen OS-Prozess (`ProjectWindows.cs`). Tests müssen serialisiert laufen. Plugins können nicht isoliert werden. |
| Core blockiert synchron auf UI-Dialoge | `AppMessages` (statische Hooks); 150 Aufrufe in 33 Core-Dateien | Kein headless Betrieb (CLI, CI, Batch), keine Async/Cancellation. |
| UI-Begriffe im Kernel | `Core/EditorAvailability.cs`, `Core/BetaEditors.cs` schlüsseln nach Avalonia-View-Klassennamen | Kernel kennt die Oberfläche. |
| Domänenlogik im UI-Projekt | ~116 Dateien (~33k LOC) in `Avalonia/Data` ohne Avalonia-API: SDAT/SSEQ, SPA-Partikel, Battle-Animationen, `PendingArchiveWrites`, `StagedArchive` | Staging/Dirty-Layer liegt außerhalb des Cores. |
| Keine Verträge, nur Konventionen | 6 Interfaces in der gesamten Solution; `RomFile` nur `ToByteArray()`; neue Formate duck-typed | Nichts ist entdeckbar oder austauschbar. |
| Editor-Registrierung von Hand | `AvaloniaEditorLauncher.cs` 1.790 Zeilen, ~137 `OpenXxx()`-Methoden, hartkodierte Ctrl+P-Liste; `MainWindowView` 1,4k Zeilen Code-Behind mit 39 `Shown[...]`-Bindings | Kein Contribution-Modell. |
| Fette ViewModels mit UI-Typen | 106 von 146 ViewModels referenzieren Avalonia, 74 rufen `DialogHelper`; null `ICommand` | Nicht testbar, nicht wiederverwendbar. |
| Kein Docking | Fester Tab-Satz im `MapsWorkspaceView`, alles andere schwebende Fenster | Plugin-Panels haben keinen Platz. |
| Offsets in C# | `RomLocator.*.cs` ~5,5k Zeilen `switch` über Spiel und Sprache; der `feature/offsetDB`-Branch (YAML) ist nicht gemergt | Neue ROM-Varianten oder Hacks brauchen Code-Änderungen. |
| Dateisystem als Datenbank | NARCs werden in nummerierte Member-Ordner entpackt; kein In-Memory-Projektmodell, keine Transaktion, kein Changeset | Schwer für Undo über Editoren hinweg, Diff, Versionierung. |
| Tests nicht in CI, Busfaktor 1 | Kein `dotnet test`-Workflow; 724 von ~800 Commits von einer Person | Refactoring-Risiko. |
| Lizenz AGPL-3.0 | `LICENSE` | In-Process-Plugins Dritter müssen AGPL-kompatibel sein. |

Plugin-Reife, nüchtern bewertet: etwa 2 von 10.

### 2.3 Die Suite (GitHub-Org)

Die Org ist klein (gegründet Juli 2025, ein öffentliches Mitglied) und besteht aus DSPRE plus Satelliten. Es gibt keine schriftliche Roadmap, keine Issues zu "Plugin", "Roadmap" oder "macOS". Builds gibt es für Windows und Linux, nicht für macOS.

| Repo | Was es ist | Zustand | Rolle für eine Engine |
|---|---|---|---|
| **rotom** (Rust, MIT, Upstream KalaayPT) | Hochsprache für Gen-4-Scripts "inspired by poryscript": `if/else/while/match`, Menü-Builder, `format()`, `#include` von Decomp-Headern; Compiler, Decompiler, Transpiler (DSPRE `.script`, Decomp `.s`), **LSP**; byte-matching Round-Trip gegen Decomps; Ziele: DSPRE-, Decomp- und hg-engine-Projekte | v0.7.0 (Okt 2026), "expect things to break" | Kanonische Script-Repräsentation. Fertiger Sprachserver. |
| **uxie** (Rust, MIT, crates.io 0.9.6) | `Workspace::open(path)` erkennt automatisch DSPRE/ds-rom-Projekt oder pokeplatinum/pokeheartgold-Decomp und liefert Map-Header, Events, Encounters, Personal, Moves, Items, Trainer, Learnsets, Text, Scripts; C-Konstanten-Auswerter; FFI-Layer | Platinum primär, HGSS partiell; von DSPRE nur über rotom genutzt | Prototyp genau der Backend-Abstraktion, die die Engine braucht. Nur lesend. |
| **chatot** (Rust, MIT) | Textarchive ↔ JSON/TXT, byte-identisch, `charmap.json` | stabil | Text-Pipeline, bereits integriert. |
| **ds-rom** (Rust, MIT, Upstream AetiasHax) | Exakte Extraktion und Rebuild von DS-ROMs (`header.yaml`, `config.yaml`, `arm9/`, `arm9_overlays/`, `files/`); Library und CLI | v0.8, aktiv; auch von hg-engine genutzt | Projektformat und Build-Endstufe. |
| **scrcmd-database** (JSON, MIT) | Befehlsdatenbank für DP/Pt/HGSS; v2-Schema mit `meta`, `commands{type,id,params[{name,type}],variants,expansion}`, `flags`, `vars`; Sync-Skripte vom Decomp | aktiv | Vorbild für metadatengetriebene UI-Generierung. |
| hg-engine-research | Staging-Fork von BluRosie/hg-engine für Upstream-PRs | identisch mit Upstream | Signal: das Team baut auf hg-engine. |
| TinkeDSi-DSPRE | Fork ohne eigene Commits | unklar | Keine. |
| vsc-dspre-script-compat | Regex-basierte VS-Code-Extension für `.script` | seit Aug 2025 inaktiv, durch rotom-lsp überholt | Keine. |
| Website | Changelog-Seite | kein Inhalt | Keine. |

Zusammenspiel heute: DSPRE ist Host; die Rust-Tools werden als Prozesse gespawnt (ein `dsrom-ffi`-Branch probiert FFI). `.rotom/` und `rotom.toml` sind die Script-Schicht über dem ds-rom-Projekt, `dspre_hgengine.json` verlinkt ein hg-engine-Checkout. Kein Teil hat eine veröffentlichte, stabile API außer ds-rom und uxie (0.x).

### 2.4 Ökosystem

**Decomps (pret):** pokeplatinum ist praktisch vollständig in C (3 verbleibende asm-Dateien, nur Krypto/Syscalls), baut byte-identisch und hält Spieldaten als Text: `res/pokemon/<species>/data.json`, `res/trainers/data/*.json`, Events als JSON mit symbolischen Konstanten, Scripts als Assembler-Makros. **Kartengeometrie bleibt auch dort Binärdatei** (`map_data_NNN.bin`, 619 NSBMD). pokeheartgold (~300 asm-Dateien) ist laut README nicht shiftbar, pokediamond (~577 asm) weit davon entfernt. Es gibt kein Gen-4-Pendant zu pokeemerald-expansion und keinen Porymap für Gen 4. DSPREs `feature/DecompPartialSupport`-Branch öffnet ein gebautes pokeplatinum über die Linker-xMAP, nur Header/Map/Event, nur Platinum US.

**hg-engine:** Patch-on-ROM (nicht Decomp), nur HeartGold US: `make` auf ein User-ROM, armips-Hooks, GCC für neuen C-Code, Daten als C-Designated-Initializer in `data/*.c`, nutzt inzwischen ds-rom. Starke Battle-Engine (bis Gen 6, Megas, Fairy, 30 Boxen). Karten, Events, 3D kommen weiter aus dem Binär-ROM, also aus DSPRE. Lizenz: eigen, "kein Geld", kein SPDX. Für ein kommerzielles Produkt relevant.

**Fan-Engines:** Pokémon Studio (Electron/React/TypeScript, zod-Schemata, Figma-Designsystem) ist der UX-Maßstab: eine JSON-Datei pro Entität, stabile String-IDs (`dbSymbol`) statt Indizes, versionierte Migrationen, Liste-plus-Detail-"Datenbank"-UI, Karten in Tiled, V3-Plan mit Node-Graph-Event-Editor und erklärtem Ziel, RPG Maker XP loszuwerden. Studio selbst hat keine Plugin-API; PSDK-Plugins sind `.psdkplug`-Dateien. Pokémon Essentials bleibt an das bezahlte, Windows-only RMXP gebunden, Plugins sind Ordner mit `meta.txt` und Monkey-Patching.

**Plugin-Architekturen (Essenz):**

| Vorbild | Übernehmen | Vermeiden |
|---|---|---|
| VS Code / Theia | Deklarative `contributes`-Punkte, Activation Events, LSP, DI-basierte Contributions, Out-of-Process-Host | Fehlende Sandbox wird mit "Workspace Trust" kaschiert |
| Godot | Editor aus denselben Primitiven wie Plugins gebaut (Dogfooding), `EditorInspectorPlugin`, Docks, Import-Plugins | Keine Isolation |
| RPG Maker MZ | `@param`/`@command`-Annotationen erzeugen Parameter-UI automatisch | Globales Monkey-Patching, Ladereihenfolge-Bugs |
| Tiled | `registerMapFormat`, `registerAction`, Custom Property Types, projektlokale Extensions mit Opt-in | — |
| Hex Maniac Advance | **Datenlayouts als Metadaten (TOML) statt Code**; ein generischer Tabellen-/Text-/Sprite-Editor für viele ROM-Versionen | — |
| Obsidian | `onload`/`onunload` mit automatischem Cleanup, `addCommand`, `registerView`, `addSettingTab` | — |
| Porymap | Editiert die Decomp-Quelldateien direkt, löst Konstanten aus C-Headern, JS-Callbacks | Scripting ersetzt kein Panel-/Editor-Plugin |

---

## 3. Bewertung gegen das Ziel

### 3.1 Was trägt

1. Formatwissen und Writer für die komplette Gen-4-Domäne, inklusive 3D.
2. Ein cross-platform .NET-8/Avalonia-Stack mit modernem Renderer.
3. Eine echte Script-Hochsprache mit Compiler und LSP (rotom).
4. Ein Quelltext-Backend-Muster mit verifizierten Schreibzugriffen (hg-engine-Integration).
5. Eine robuste Extraktions- und Build-Endstufe (ds-rom).
6. Metadatengetriebene Befehlsdatenbank (scrcmd v2) als Keim für generierte UIs.
7. Round-Trip-Testmuster und ein erzwungener Hausstil.
8. Eine Nutzer-Community, die UX-Feedback liefert (Issue #279 fordert explizit einen "komponentenbasierten" UI-Ansatz).

### 3.2 Die fünf strukturellen Blocker

1. **Kein Projekt-/Session-Objekt.** Alles ist statisch. Ohne das gibt es keine Plugin-Isolation, kein Multi-Projekt, keine Tests ohne Serialisierung.
2. **Kein Speichermodell im Core.** Die Wahrheit liegt in entpackten Ordnern; Staging und Undo leben im UI-Projekt. Ein Changeset-/Transaktionsmodell fehlt.
3. **Keine Verträge.** Formate, Editoren, Patches, Backends sind Konventionen. Nichts ist registrierbar oder entdeckbar.
4. **Kein Shell-Modell.** Hartkodierte Menüs, Launcher, Tabs, kein Docking, keine Command-/Menu-/View-Contributions.
5. **Keine Textform für Welt-Daten.** Karten, Kollision, Gebäude, Matrizen sind überall (DSPRE, Decomp, hg-engine) Binärdaten. Ohne Textform gibt es keinen "Projekt → Build → ROM"-Workflow für Welten, also keine Engine im eigentlichen Sinn.

### 3.3 Was "Engine" hier realistisch bedeutet

Keine Laufzeit-Engine (die ist das Spiel auf dem DS), sondern eine **Authoring-Engine mit Build-Pipeline**: Quelle ist ein textuelles Projekt (JSON/YAML/rotom/glTF-ähnlich), Ziel ist ein ROM-Patch (xdelta) oder ein ROM, das der Nutzer aus seinem eigenen Basis-ROM baut. Genau so funktionieren pokeemerald-Hacks, und genau diesen Zustand hat Gen 4 noch nirgends.

---

## 4. Zielarchitektur

### 4.1 Leitprinzipien

- **Text-first, Build-oriented.** Eine Datei pro Entität, stabile symbolische IDs (`SPECIES_FOO`, `MAP_TWINLEAF_TOWN`), versioniertes Schema mit Migrationen, ROM nur als Build-Artefakt.
- **Backends statt Spezialfälle.** Vanilla-ROM, hg-engine, pokeplatinum sind drei Provider desselben Domänenmodells. Jedes Backend deklariert, welche Entitätstypen es besitzt. Das ersetzt "Editoren ausblenden, wenn HGE erkannt".
- **Metadaten vor Code.** Binärlayouts, Offsets, Enum-Tabellen, Befehlsparameter liegen in Daten (HMA-Prinzip). Generische Inspector-/Tabellen-UIs werden daraus erzeugt.
- **Dogfooding.** Jeder First-Party-Editor nutzt ausschließlich die Plugin-API. Nur so bleibt die API vollständig.
- **Kernel ohne UI und ohne Blockieren.** Ergebnisse (`Result<T>`, `WhyNot`) statt Dialoge, async mit Cancellation, injizierter `IUserInteraction` nur in der Shell.
- **Rechtlich sauber.** Nie ROM-abgeleitete Assets ausliefern, Basis-ROM per SHA-1 prüfen, Hacks als xdelta/BPS verteilen, Plugin-Registry nur Code und Metadaten.

### 4.2 Schichten

```
┌──────────────────────────────────────────────────────────────────┐
│ Shell (Avalonia 12, Dock.Avalonia)                                │
│  Workbench · Command Palette · Menüs · Docks · Inspector · Tours  │
├──────────────────────────────────────────────────────────────────┤
│ Extension Host                                                    │
│  C#-Plugins (AssemblyLoadContext) · JS-Scripting (Jint, TS-Typen) │
│  Out-of-Process (CLI / JSON-RPC / LSP: rotom, chatot, ds-rom)     │
├──────────────────────────────────────────────────────────────────┤
│ Domain-Module (Plugins, First-Party)                              │
│  World · Pokémon · Trainers · Items · Battle · Graphics · Audio   │
├──────────────────────────────────────────────────────────────────┤
│ Kernel (DSPRE.Core → Engine.Kernel)                               │
│  GameProject · AssetStore (Stage/Commit/Undo/Events)              │
│  Codec-Registry · Schema/Metadata · Build-Graph · Diagnostics     │
├──────────────────────────────────────────────────────────────────┤
│ Backends                                                          │
│  VanillaRom (ds-rom + RomLocator-Metadaten) · HgEngine (Quelltext)│
│  PokePlatinum (Decomp-JSON/.s) · später: eigenes Textprojekt      │
└──────────────────────────────────────────────────────────────────┘
```

### 4.3 Kernel-Verträge (Skizze)

```csharp
public sealed class GameProject            // ersetzt die RomInfo-Statics
{
    public GameIdentity Game { get; }      // Version, Sprache, Revision, Dump-Hash
    public ProjectLayout Layout { get; }   // ds-rom / decomp / hg-engine / textprojekt
    public IAssetStore Assets { get; }
    public IBackendSet Backends { get; }
    public IMetadataCatalog Metadata { get; }   // Offsets, Layouts, Enums (heute RomLocator)
    public IBuildGraph Build { get; }
}

public interface IAssetCodec<T>            // formalisiert die Reader/Writer-Konvention
{
    AssetKind Kind { get; }
    Result<T> Read(ReadOnlySpan<byte> bytes, GameProject p);
    Result<byte[]> Write(T value, GameProject p);
    WhyNot? Supports(GameProject p);       // ersetzt EditorAvailability
}

public interface IAssetStore               // PendingArchiveWrites + ArchiveMembers, in den Kernel gezogen
{
    ValueTask<Asset<T>> GetAsync<T>(AssetId id, CancellationToken ct);
    IChangeSet BeginChanges(string label);  // Stage → Commit → Undo-Schritt
    IObservable<AssetChanged> Changes { get; }
}

public interface IBackend                  // Vanilla / hg-engine / Decomp
{
    IReadOnlySet<AssetKind> Owns { get; }
    ValueTask<Result> Pull(GameProject p, AssetKind kind, CancellationToken ct);  // Quelle → Store
    ValueTask<Result> Push(GameProject p, IChangeSet changes, CancellationToken ct); // Store → Quelle (verified write)
    IBuildStep[] BuildSteps { get; }
}
```

Die bestehende hg-engine-Integration (`HgEngineDomains`, `HgEngineSync`, `HgEngineVerifiedWrite`) ist bereits fast ein `IBackend`. uxie ist bereits fast ein lesender `IBackend` für Decomps.

### 4.4 Contribution Points (das "Everything as a Plugin")

Ein Plugin ist ein Ordner mit `plugin.json` (ID, Semver, `engine`-Bereich, `activationEvents`, `contributes`) und wahlweise einer .NET-Assembly, einem JS-Modul oder einem externen Prozess.

| Contribution | Beispiel | Vorbild |
|---|---|---|
| `backends` | hg-engine-Checkout, pokeplatinum, Vanilla | eigenes Design |
| `assetKinds` + `codecs` | NSBMD, BDHC, EncounterFileHGSS | Tiled `registerMapFormat` |
| `schemas` | JSON-Schema/zod-Äquivalent pro Entität, Migrationen | Pokémon Studio |
| `metadata` | Offset-Tabellen, Enum-Listen, ROM-Varianten, Hack-Overrides (xMAP) | HMA TOML, `feature/offsetDB` |
| `editors` | Vollbild-Editor für eine AssetKind | VS Code `customEditors` |
| `inspectors` | Property-Panel-Erweiterung für einen Typ | Godot `EditorInspectorPlugin` |
| `docks` / `views` | Minimap, Problems, Script-Outline | Dock.Avalonia, Obsidian `registerView` |
| `commands` + `menus` + `keybindings` | Ctrl+P-Einträge, Kontextmenüs | VS Code |
| `tools` (Welt-Editor) | Pinsel, Kollisionsmaler, Event-Platzierer | Tiled tools |
| `importers` / `exporters` | OBJ, glTF, PNG-Spritesheets, Tiled-TMX, PDSMS `.pdsmap` | Godot Import-Plugins |
| `scriptCommands` | scrcmd-v2-Einträge inkl. Parametertypen, UI-Widgets | RPG Maker `@command` |
| `patches` | Toolbox-Patch mit Marker, `IsApplied`, `WhyNot`, `.s`-Quelle | bestehendes DSPRE-Rezept |
| `buildSteps` | rotom compile, chatot encode, make-Target, dsrom build | pokeemerald `make` |
| `languages` | TextMate-Grammatik + LSP-Server | VS Code |
| `settings` | namespaced Settings mit generierter UI | Obsidian `addSettingTab` |
| `tours` / `docs` | Guided Tours (bereits vorhanden) | eigenes |

**Drei Ausführungsstufen:**

1. **C#-Plugins in-process** über `AssemblyLoadContext` (collectible) oder McMaster.NETCore.Plugins, mit einem kleinen, semver-versionierten `Engine.Abstractions`-Paket als geteiltem Vertrag. Für Editoren, Backends, Codecs, Renderer.
2. **JS-Scripting sandboxed** über Jint mit ausgelieferten TypeScript-Typen. Für Nutzer-Makros, Batch-Operationen, Callbacks (`onProjectOpened`, `onAssetChanged`), einfache Actions. Speicher- und Zeitlimits, kein Dateisystemzugriff außer über die API.
3. **Out-of-Process** über CLI/JSON-RPC/LSP. Für die Rust-Toolchain, für Sprach-Server, für alles mit abweichender Lizenz. Das ist auch die AGPL-sichere Grenze für Drittanbieter.

Kein globales Monkey-Patching, keine Ladereihenfolge-Magie: Abhängigkeiten stehen im Manifest, Aktivierung ist lazy.

### 4.5 UI/UX-Konzept ("RPG Maker, nur besser")

- **Eine Workbench, nicht 100 Fenster.** Docking-Shell mit Projektbaum links (Welt, Pokémon, Trainer, Items, Scripts, Grafik, Audio, Patches), Editor-Tabs in der Mitte, Inspector rechts, Problems/Build/Konsole unten. Layouts speicherbar.
- **Welt-Editor als Herzstück.** 3D-Ansicht der ganzen Matrix (hat DSPRE schon) mit Modus-Werkzeugleiste: Tiles malen, Kollision malen, Gebäude platzieren, Events platzieren, Warps per Drag verbinden, Kamera. 2D-Top-Down-Modus für schnelles Arbeiten (`TileGridControl`, `PermissionGridControl` existieren). Alles undo-fähig über das Changeset.
- **Datenbank-Editoren nach Pokémon-Studio-Muster.** Liste plus Detail, Suche, Filter, Bulk-Edit, Referenzzähler ("wird in 12 Trainern verwendet", DSPRE hat "clickable reference counts" schon), Querverlinkung ("Öffnen in …").
- **Events zweigleisig.** Visueller Event-Editor (Liste oder Node-Graph) und rotom-Text im selben Tab, verlustfrei umschaltbar. Befehls-Parameter-UI wird aus der scrcmd-v2-Datenbank generiert (Species-Picker, Item-Picker, Flag-Picker, Variablen mit Namen aus `LabelStore`).
- **Build und Playtest.** "Build"-Button führt den Build-Graph aus (nur geänderte Schritte), zeigt Diagnostics strukturiert (nicht als Dialog), startet optional einen Emulator (melonDS/DeSmuME per CLI) mit dem Ergebnis. Die Feld-Simulation bleibt als schnelle Vorschau im Editor.
- **Projektgesundheit.** "Project Checks" (existiert) als Problems-Panel: kaputte Warps, Scripts ohne Ziel, Trainer ohne Party, Text zu lang für die Box.
- **Konsistenz als Feature.** Das Team hat es in Issue #279 selbst formuliert: eine Toolbar, ein ID-Picker, ein Checkbox-Stil, Sentence Case. Diese Regeln gehören in die Plugin-API als Standard-Controls, damit auch Plugins konsistent aussehen.
- **Versionierung eingebaut.** Textprojekt in git, Diff-Ansicht pro Entität, xdelta-Export für Releases.

### 4.6 Technologieentscheidungen

| Thema | Entscheidung | Begründung |
|---|---|---|
| Sprache/Runtime | C# / .NET 8 (SDK 10), später .NET 10 LTS | DSPRE.Core wiederverwenden, Avalonia 12 braucht es |
| UI | Avalonia 12 + Dock.Avalonia (MIT, 12.1.x, Okt 2026) | vorhanden, cross-platform, Docking fertig |
| MVVM | CommunityToolkit.Mvvm (Source-Generatoren, `RelayCommand`) | ersetzt handgeschriebenes INPC und Click-Handler, macht VMs testbar |
| DI | Microsoft.Extensions.DependencyInjection + Contribution-Discovery per Manifest | Standard, leichtgewichtig |
| 3D | bestehender `OpenGlControlBase`-Renderer, Mesh-Abstraktion in den Kernel | funktioniert bereits, kein OpenTK nötig |
| Scripting | Jint 4.x für Nutzer-Makros | pure .NET, sandboxbar, TS-Typen |
| Text/Scripts | chatot, rotom + rotom-lsp (Out-of-Process) | bereits integriert, byte-matching |
| ROM-I/O | ds-rom (CLI, optional FFI) | Standard auch bei hg-engine |
| Schemata | JSON Schema + generierte C#-Records; Migrationen pro Schema-Version | Pokémon-Studio-Erfahrung |
| 3D-Austausch | glTF (apicula-Export existiert) als Importformat, eigenes JSON für Kollision/Gebäude | Blender-Workflow |
| Tests | xUnit mit synthetischen Fixtures plus optionalen ROM-Round-Trips; `dotnet test` in CI | heute nicht in CI |
| Lizenz | AGPL-Kern beibehalten, `Engine.Abstractions` unter MIT/Apache-2 veröffentlichen, Out-of-Process-Grenze dokumentieren | ermöglicht Drittplugins; mit Maintainern klären |

---

## 5. Strategische Optionen

| Option | Beschreibung | Pro | Contra |
|---|---|---|---|
| **A. Strangler auf `feature/avalonia`** (empfohlen) | Kernel-Refactoring im Core (Projektobjekt, AssetStore, Codecs), neue Shell mit Docking und Contribution-API daneben; Editoren werden nacheinander auf die API migriert; alte Launcher-Pfade bleiben, bis sie leer sind | Erhält 250k LOC Domänenwissen, Tests und Community; sofort nützlich; Upstream-fähig | Erfordert Abstimmung mit dem Maintainer; Refactoring in sehr aktivem Branch (270 Commits/Monat) |
| B. Neue Shell, DSPRE.Core als NuGet | Core unverändert als Paket konsumieren, eigene Engine-App drumherum | Keine Konflikte mit Upstream | Core hat keine öffentliche API, ist statisch, nicht headless; man würde die Statics erben |
| C. Greenfield mit Rust-Kern (uxie/rotom/ds-rom) | Rust-Kernel mit FFI, C#/Avalonia- oder Web-Shell | Sauberste Architektur, Backends von Anfang an | Jahre bis zur Parität mit DSPREs Writern (uxie ist nur lesend, kein 3D); zweites Sprach-Ökosystem |

Entscheidend für A: früh mit Mixone-FinallyHere und KalaayPT sprechen. Das Team hat bereits die passenden Instinkte (Core/UI-Split, Banned-API-Analyzer, hg-engine als Quelle, rotom), aber keine Plugin-Ambition dokumentiert. Ein RFC im DSPRE-Repo mit Schritt 1 (Projektobjekt) wäre der natürliche Einstieg, weil es auch deren eigene Probleme löst (Multi-Window per Prozess, serialisierte Tests).

---

## 6. Roadmap (Phasen, keine Zeitangaben)

**Phase 0: Absichern.** `dotnet test` in CI; synthetische Fixtures für die wichtigsten Codecs; Golden-File-Round-Trips auch gegen `main` als Referenz.

**Phase 1: Kernel-Instanz.** `GameProject` einführen, `RomInfo`-Statics schrittweise migrieren (`GameDirs`, `WorkDir`, `GameFamily` zuerst), `using static RomInfo` abbauen. `AppMessages` durch `Result`/`WhyNot` plus injizierte `IUserInteraction` ersetzen. `EditorAvailability` zu Capability-Abfragen am Projekt.

**Phase 2: Speichermodell.** `PendingArchiveWrites`/`StagedArchive`/`ArchiveMembers` zu `IAssetStore` mit Changesets, Undo und typisierten Events in den Kernel. Avalonia-freie Dateien aus `Avalonia/Data` in Domain-Assemblies verschieben.

**Phase 3: Verträge und Metadaten.** `IAssetCodec`, `IBackend`, `IBuildStep`, `IPatch` formalisieren. `RomLocator` nach YAML (`feature/offsetDB` wiederbeleben). hg-engine-Integration zu `IBackend`, uxie-Reader als Decomp-`IBackend`.

**Phase 4: Shell und Extension Host.** Dock.Avalonia-Workbench, DI, Manifest-Loader, Contribution-Registry, Command-Palette aus Contributions, Settings-Namespaces. Zwei bis drei Editoren (Camera als kleiner, Map als großer, Trainer als Datenbank-Typ) als erste First-Party-Plugins migrieren und die API daran härten.

**Phase 5: Textprojekt und Build-Graph.** JSON-Schemata pro Entität mit `dbSymbol`-IDs, Export/Import vom Vanilla-Backend, inkrementeller Build zu ROM/xdelta, Emulator-Start. rotom als kanonische Script-Form mit visuellem Editor darüber.

**Phase 6: Welt-Textformat.** Kartengeometrie, Kollision, Gebäude, Matrizen als Text/glTF mit Compiler zu NSBMD/BDHC (aufbauend auf `TileBake`, `NsbmdWriter`, `BdhcBuild`). Das ist der Punkt, an dem aus dem ROM-Editor eine Engine wird, und es wäre auch für pokeplatinum ein Geschenk.

**Phase 7: Öffnen.** `Engine.Abstractions` veröffentlichen, Jint-Scripting, Plugin-Registry, Dokumentation, Beispiel-Plugins.

---

## 7. Risiken

- **Busfaktor.** Praktisch ein Entwickler für das C#, einer für Rust. Jede Zusammenarbeit muss deren Tempo respektieren; ein paralleler Fork würde sich sofort entkoppeln.
- **Lizenz.** AGPL-3.0 für den Kern; hg-engine ohne SPDX und mit "kein Geld"-Klausel; Tinke/ndspy/knarc GPL. Für ein kommerzielles Produkt wäre nur Option C mit sorgfältiger Lizenzwahl gangbar. Für ein offenes Projekt reicht ein MIT-lizenziertes Abstractions-Paket plus Out-of-Process-Grenze.
- **Rechtlich.** Nie Assets ausliefern. Nintendo hat Fan-Spiele massenhaft entfernt; ROM-Editoren mit User-ROM sind der etablierte sichere Rahmen.
- **Welt-Textformat ist Neuland.** Niemand hat es; Round-Trip zu byte-identischen NSBMD ist anspruchsvoll (Materialien, Texturen, Display Lists).
- **Decomp-Backend begrenzt.** Nur pokeplatinum ist nutzbar; HGSS bleibt auf hg-engine angewiesen, das wiederum nur HeartGold US patcht.
- **Refactoring im laufenden Betrieb.** 270 Commits pro Monat im Branch; Kernel-Umbauten müssen in kleinen, mergebaren Schritten erfolgen.
- **Performance.** Ein In-Memory-AssetStore mit Changesets darf die heutigen Ladezeiten nicht verschlechtern; Lazy Loading pro AssetKind ist Pflicht.

---

## 8. Nicht verifiziert / offen

- Offizielle Shiftbarkeits-Aussage zu pokeplatinum (decomp.dev war nicht erreichbar).
- Zweck des `TinkeDSi-DSPRE`-Forks (keine eigenen Commits).
- Ob die Maintainer eine Plugin-API wollen (keinerlei Signal in Issues, Docs oder Commits).
- Exakte Zahl der Beta-Editoren (Code: 38 gegated, Changelog: "16 not finished").
- Interna von PSDKs `.psdkplug`.

---

## Anhang: Wichtige Fundstellen

**Avalonia-Branch (`feature/avalonia`):** `DSPRE-Development.md`, `BannedSymbols.txt`, `DSPRE.Core/RomInfo.cs`, `DSPRE.Core/RomLocator.*.cs`, `DSPRE.Core/AppMessages.cs`, `DSPRE.Core/EditorAvailability.cs`, `DSPRE.Core/ArchiveMembers.cs`, `DSPRE.Core/HgEngine/HgEngineDomains.cs`, `HgEngineSync.cs`, `HgEngineVerifiedWrite.cs`, `DSPRE.Core/Models/NsbmdWriter.cs`, `TileBake.cs`, `BdhcBuild.cs`, `DSPRE.Avalonia/Avalonia/AvaloniaEditorLauncher.cs`, `Avalonia/Data/PendingArchiveWrites.cs`, `Avalonia/Gl/NsbmdGlControl.cs`, `Avalonia/RotomLanguageServerClient.cs`, `DSPRE.Tests/Core/UnchangedSaveTests*.cs`.

**Main-Branch:** `DS_Map/RomInfo.cs`, `DS_Map/ROMFiles/ScriptFile.cs`, `DS_Map/Resources/ROMToolboxDB/ToolboxDB.cs`, `DS_Map/Editors/PatchToolboxDialog.cs`.

**Unmergte DSPRE-Branches mit relevanten Ideen:** `feature/offsetDB` (YAML-Offsets), `feature/DecompPartialSupport` (xMAP-Overrides), `dsrom-ffi`, `uxie`.

**Extern:** github.com/DS-Pokemon-Rom-Editor/{rotom,uxie,chatot,scrcmd-database}, github.com/AetiasHax/ds-rom, github.com/pret/pokeplatinum (docs/datafiles/overview.md), github.com/BluRosie/hg-engine, github.com/PokemonWorkshop/PokemonStudio (Wiki "V3 Doc Product Vision"), github.com/huderlem/porymap (manual/scripting-capabilities), github.com/haven1433/HexManiacAdvance (Wiki "TOML and You"), github.com/wieslawsoltes/Dock, github.com/sebastienros/jint, github.com/natemcmaster/DotNetCorePlugins, docs.avaloniaui.net/docs/avalonia12-breaking-changes.
