# Ausbau nach Phase 3: Entscheidungen (2026-10-25)

Anlass: Die Wunschliste des Nutzers nach dem Abschluss von Phase 3 (sortierbare Orte und eigene Orte, leichterer Umgang mit Dynamischen Headern und Patches, Projektstart aus einer `.nds` mit Vorlagen je Edition, die lokale Test-API als MCP-Server für alle Nutzer, eine Unity-artige Werkbank mit schwebender Werkzeugleiste, getrennter Hierarchie und Inspector, Scharnier über den Reitern, frei anordenbaren Panels, die Abstraktion von Raster und Header hinter Weltkarte › Gebiete › Räume). Drei Gutachten (nur lesend, mit Messungen an den drei Vanilla-Projekten) haben die schwierigen Fragen bewertet:

- [gutachten-a-weltmodell.md](gutachten-a-weltmodell.md): Begriffs- und Datenmodell, eigene Orte, Sortierung, Raster-Reiter, Patch, mehrere Weltkarten.
- [gutachten-b-werkbank.md](gutachten-b-werkbank.md): Dockview-Fähigkeiten, drei Werkbank-Varianten, Design-System-Änderungen, Umbaupfad.
- [gutachten-c-projektstart-mcp.md](gutachten-c-projektstart-mcp.md): Entpacken aus der ROM, Vorlagen, MCP-Server auf der Befehlsschicht der Brücke.

Die Entscheidungen folgen jeweils der Empfehlung („recommend“) des Gutachtens; wo die Gutachten Fragen an den Nutzer offen ließen, steht hier die gewählte Vorgabe. Der Nutzer kann jede einzeln umstoßen; die Roadmap ([docs/ROADMAP.md](../../ROADMAP.md), Phasen 3e bis 3h) setzt sie voraus.

## Phasenschnitt und Reihenfolge

| Phase | Inhalt | Warum an dieser Stelle |
|---|---|---|
| 3e Werkbank | Scharnier als Kopfleiste über der Schau, schwebende Werkzeugleiste, Weltbaum Weltkarte › Gebiet › Räume mit Ortsetikett und Sortierung, Hierarchie „In dieser Szene“ statt „An diesem Ort“, Inspector nur fürs Gewählte, ein Dock je Bereich mit freier Anordnung | Der sichtbarste Wunsch; jede Teilphase ist für sich ein Gewinn, und 3f setzt auf dem neuen Baum auf |
| 3f Weltmodell | Reiter „Raster“ nur noch in Tüfteln (Weltkarte und 3D übernehmen Zellen, Auswahl, Raumplan, Kamera aufs Gebiet, Teil-Laden), eigene Orte als echte Ortsnamen im ROM, Patch mit einer Zustimmung im Assistenten, optional nahtlose Gebiete und Abspalten geteilter Räume | Braucht den Baum aus 3e.2; größter Backend-Anteil der UI-Wünsche |
| 3g Projektstart | „Neues Spiel aus ROM…“ als Assistent (Original, Nur Daten), dann „Leere Region mit Startstadt“ und „Welt leeren…“ | Unabhängig, klein im Kernel (Entpacken gibt es); legt `.g4/base.json` an, das Phase 4 für xdelta braucht |
| 3h Agenten-Schnittstelle (MCP) | Befehlsschicht aus dem Dev-Feature lösen, kuratierte Tools, `studio mcp` kopflos und in der laufenden App, Freigabe je Projekt, Agenten-Schritte im Verlauf | Baut auf der fertigen Welt-Bedienung auf (Tool-Namen in Nutzerworten) und macht das Gate von Phase 4 kopflos fahrbar |
| 3d Erzählen | wie bisher „später“ (Spec 3c §13): Erzählen-Tiefe, Flags und Variablen, Geschichten, Löschen mit Referenzsperre | nicht Teil der Wunschliste; Einordnung nach 3h oder nach Phase 4 bleibt offen |

Phase 4 (Textprojekt und Build-Graph), 5 (Plugin-Host) und 6 (Kartenerstellung) folgen unverändert. Begründung der Reihenfolge 3e → 3f → 3g → 3h: 3e und 3f ändern die Welt-Bedienung, auf der der Assistent „Welt leeren…“ (3g) und die Tool-Beschreibungen (3h) aufsetzen; 3g.1 ist klein und kann bei Bedarf parallel zu 3f laufen (eigene Dateien).

## Werkbank (Gutachten B)

| Frage | Entscheidung | Grund |
|---|---|---|
| Gesamtvariante | **C, Hybrid**: Scharnier oben, ein Dock je Bereich, Hierarchie in der Liste, Werkzeuge schweben; in der Reihenfolge W1 → W2 → W3, W4 optional | erfüllt alle Wünsche, Atlas und Liste bleiben Rahmen aller Bereiche, Aufwand M; W1 allein ist schon ein Gewinn ohne Rückbau |
| Name „Scharnier“ | bleibt (Tokens `scharnier/*`, `aria-label`, Gate-Texte); Prinzip 2 lautet „über der Schau“ statt „zwischen Schau und Hand“ | Signatur des Systems, keine Umbenennung in Tests und Doku |
| Prinzip 1 | wird „Zeigen und anfassen“: in der Welt steht die Hand an den Rändern (Hierarchie links, Inspector rechts, Script und Probleme unten), in Texten und Daten unter der Schau | Unity-Bild für die Welt, DS-Bild für Daten |
| Werkzeugleiste | schwebend nur in der 3D-Ansicht, Standard senkrecht links, ziehbar an drei Anker (links, oben, unten), merkt die Lage; unter rund 400 px Höhe waagrecht; `role="toolbar"`, `data-tool`, Roving bleiben | gehört zur Ansicht, nicht zum Dock; Raster und Weltkarte bekommen in W4 eine eigene kleine Leiste |
| Buchstaben-Kürzel | wirken nur, wenn 3D sichtbar ist | heute wechseln sie das Werkzeug auch unsichtbar |
| Hierarchie | fest in der Liste „Welt“: oben „Orte“ (einklappbar zu einer Ortszeile mit Picker), unten „In dieser Szene“ (Header, Karten mit Gebäuden, Figuren, Türen mit Ziel, Auslöser, Schilder, Scripts), Teiler dazwischen; frei andockbar erst als Spike in W4 | ein Panel, Szenenwechsel bleibt ein Klick; Godot-Muster |
| Inspector | nur das Gewählte, mit Kopf (Typ-Wort, Titel, Kennung, Plaketten, Aktionsmenü); „An diesem Ort“ und die Ausgangsliste wandern in die Hierarchie, „Teile importieren…“ ins Kopfmenü der Karte; die Hover-Ansichten gehen in die Statusleiste | Unity-Trennung Hierarchy/Inspector |
| Schau-Panels (3D, Raster, Weltkarte) | bleiben in der Mitte, schweben nicht, kein Ausklappen in eigene Fenster | Canvas-Zustand (three.js), Tauri ohne Fenster-Handler |
| Datenbereiche (Texte, Pokémon, Gegenstände) | in W3 mit einem Dock je Bereich, aber Vorlage „Doppeldeck“ (Schau oben, Bearbeiten unten) | sehen aus wie heute, Lernkurve klein |
| Tastatur | eigene F6-Runde durch die Gruppen und Palette-Befehle („Inspector schweben lassen“, „andocken“, „Anordnung: Werkbank / Doppeldeck“, „Schau maximieren“) | Dockview-Tastatur ist Enterprise |
| Design-System | je Teilphase README (Prinzipien, Fensterraster, Komponenten „Werkzeugleiste schwebend“, „Hierarchie“, „Inspector-Kopf“, „Dock-Reiter“, „Schwebende Gruppe“, „Teiler“), Token `schatten/schwebend`, Seite „App: Werkbank“ in `Editor_DesignSystem.fig` per OpenPencil, danach `fix_fig_text.mjs --rgb555` | Wunsch des Nutzers: Design-System erweitern und .fig nachziehen, mit dem Frontend-Design-Skill |

## Weltmodell (Gutachten A)

| Frage | Entscheidung | Grund |
|---|---|---|
| Modell | **Variante B**: Weltkarte › Gebiet › Räume physisch (Räume über den Warp-Graphen dem Gebiet ihres Eingangs zugeordnet), der Ort ist ein Etikett mit Farbe an Gebiet und Raum; `g4-world.json` hält nur, was das ROM nicht weiß (Namen der Weltkarten, Klarnamen der Räume, manuelle Zuordnungen) | entspricht dem Bild „das Haus steht in Zweiblattdorf“; 62 (Pt) bzw. 44 (HGSS) Orte bestehen nur aus Räumen und hätten nach Namen kein Gebiet |
| Weltkarten | Matrizen mit Header-Abschnitt und mindestens zwei Ortsnamen; HGSS zeigt Johto und Kanto als **zwei Weltkarten** (Ansichten derselben Matrix 0 über `kanto_flag`); Großmoor, Safari, Kampfzone als „Weitere Gebiete“ | Nutzerwunsch „mehrere Weltkarten“, Datenlage (nur je zwei Matrizen mit Header-Abschnitt) |
| Begriffe in Bauen | Weltkarte, Gebiet, Raum, Ort, Karte, Eingang/Ausgang, Aussehen, Landschaft (Zellen mit Header 0); **Header und Raster nur in Tüfteln** | Wunsch des Nutzers; Glossar `terms.ts` bekommt die neuen Schlüssel |
| Eigene Orte | **echte Ortsnamen im ROM** (Nachricht an die Ortsnamen-Bank anhängen, `location_name` der Header umhängen, ein Undo-Schritt); keine Editor-Ordner oder Tags; Warnung ab 17 Zeichen; HGSS mit nur 21 freien Plätzen plus Wiederverwendung ungenutzter Sinnoh-Namen mit Warnsatz | sichtbar im Spiel (Einblendung, Fundort); eine dritte Hierarchie würde verwirren; Tags später ergänzbar |
| Stadtkarte und Flugziele neuer Orte | nicht in 3f; eigener Recherche-Posten (Overlay-Tabellen, Patch) | nicht belegt, wo sie liegen |
| Sortierung | Baum: Spielreihenfolge (Standard), Name A–Z (`localeCompare('de', {numeric: true})`), Zuletzt gewählt; Liste: Nummer, Name, Kürzel; gemerkt als `worldSort` in `g4.shell.v1`, „Zuletzt gewählt“ je Projekt im `localStorage` | Wunsch „alphabetisch sortierbar“; „zuletzt bearbeitet“ je Header geht nicht (eine Header-Tabelle ist ein Asset) |
| Klarnamen für Räume | aus einer Heuristik vorgeschlagen, umbenennbar, in `g4-world.json` | Audit-Punkt „Klarnamen für Räume“, zwei Sprachebenen |
| Raster-Reiter | in Bauen weg: Zellen, Auswahl, „+“ und Kartenbilder in der Weltkarte (mehrzellige Räume als Raumplan), 3D wählt die Karte per Klick, Kamera rahmt das Gebiet, nur Gebiet plus Nachbarn werden geladen, **Draufsicht-Kamera schon hier** (Phase 6 behält Gelände malen); Name, Größe, Abschnitte, „Neue Matrix“ bleiben im Reiter unter Tüfteln | heute lädt die Szene alle 468 bzw. 492 Karten von Matrix 0 |
| Neue Gebiete | standardmäßig wie heute als eigene Matrix mit Türen; nahtlos auf der Weltkarte (`create_area` mit Ziel „bestehende Weltkarte“) als optionales 3f.4 | kleiner Schritt zuerst; Nahtlos braucht freie Zellen und einen Ortswechsel ohne Warp |
| Geteilte Räume (bis zu 33 Räume auf einer Karte) | Plakette „Geteilt“ in Bauen; „Abspalten“ (`split_room`) als optionales 3f.4 | sonst ändert „dieser Raum“ alle |
| Freie Vanilla-Header wiederverwenden | nur noch in Tüfteln | Bauen legt immer neu an |
| Patch | Schritt „Ort“ statt „Header“ im Assistenten, der Header wird immer neu angelegt (interner Name automatisch), der Patch wird einmal in der Zusammenfassung bestätigt (Plakette „Nicht rückgängig“); der Kernel wendet ihn auch bei ungespeicherter Header-Tabelle an und legt vorher einen Schnappschuss von `unpacked/dynamicHeaders` an | heute: Assistent abbrechen, speichern, Technik, neu öffnen |

## Projektstart (Gutachten C, Teil A)

| Frage | Entscheidung | Grund |
|---|---|---|
| Fluss | **S1 plus S3**: Assistent „Neues Spiel aus ROM…“ im Scharnier (ROM wählen → `inspect_rom` zeigt Spiel, Sprache, Freigabe → Vorlage → Name und Zielordner → Zusammenfassung → Anlegen mit Fortschritt → Öffnen), dazu Drag-and-drop einer `.nds` auf den Start-Screen | gleiche Muster wie die vorhandenen Assistenten |
| Vorlagen | Original, Leere Region mit Startstadt, Nur Daten (eine Voreinstellung der Oberfläche); Vorlagen sind Rezepte aus IDs, keine Dateien | nie ROM-Daten im Repo |
| „Leere Welt“ | **Leere Region mit Startstadt**, nicht „ganz leer“: alle Header-IDs bleiben (Code verweist fest darauf), Matrix 0 behält Startstadt plus gesperrte Rand-Karte, Außen-Header bekommen leere Eventdateien und keine Level-Scripts, Scripts und Texte werden nur nicht mehr verwiesen, Start-Header und Spielerhaus bleiben, ein Start-Script als rotom-Vorlage gibt den Starter und setzt Pokédex- und Laufschuh-Flags; eigener Startort und übersprungenes Intro erst nach einem Spike (ARM9-Patch) | Intro und „zu Hause aufwachen“ laufen ohne ARM9-Patch |
| „Welt leeren…“ | derselbe Schritt als Assistent im offenen Projekt (Technik › Werkzeuge) | Vorlage ist kein Einmal-Pfad, das Gate kann sie auf jeder Testkopie fahren |
| Projektordner | Standardname `<Name>_DSPRE_contents` (änderbar), Marke `.g4/base.json` mit SHA-1 der Basis-ROM und `origin: "extracted"`; die Brücke bekommt für Agenten-Tests einen Weg ohne die Suffix-Sperre | DSPRE-Kompatibilität; Phase 4 braucht die Basis-SHA-1 für xdelta |
| Kompatibilität | arm9, Overlays, `files`, `header.yaml` sind byte-gleich zu DSPRE-Ordnern; einzig `padding:` in `config.yaml` fehlt beim gepinnten ds-rom; ob DSPRE den Ordner öffnet, ist ein Handpunkt im Gate | gemessen am 2026-10-25 (Platin DE 0,49 s, HeartGold DE 0,37 s) |

## Agenten-Schnittstelle (Gutachten C, Teil B)

| Frage | Entscheidung | Grund |
|---|---|---|
| Architektur | **Hybrid aus „in der App“ und „kopflos“** über eine gemeinsame Werkzeugschicht: `dispatch` aus dem Dev-Feature nach `commands/dispatch.rs`, `Roots` wird `Policy` (`DevPolicy` mit `_testkopien`, `UserPolicy` nur das offene Projekt); darüber `mcp::tools` mit rund 30 kuratierten englischen Tools, `g4_batch` (ein Undo-Schritt) und `g4_call` (alle Befehle, Expertenmodus, Standard aus); Wirte: die App mit Streamable HTTP auf 127.0.0.1 (Token, Entdeckungsdatei 0600) und `studio mcp` (stdio), das an die laufende App durchreicht oder kopflos mit `--project` arbeitet | 91 Befehle 1:1 sind zu UI-nah; ein Sidecar, der die Brücke anspricht, wären zwei Protokolle ohne gemeinsame Sitzung |
| SDK | `rmcp` (offizielles Rust-SDK, Apache-2.0, tokio), Stand prüfen beim Start der Phase | Spezifikation ändert sich schnell |
| Sicherheit | Standard aus; Freigabe je Projekt in den App-Einstellungen („nicht / lesen / lesen und bearbeiten“, nach kanonischem Pfad und `.g4/base.json`-ID, nie im Projektordner); Bestätigung für Unumkehrbares (Patch, Trainer anlegen, Script anlegen, Bauen außerhalb `build/`, Emulator, Ausgangslage); kopflos `--allow-irreversible` | ein fremdes Projekt darf sich nicht selbst freischalten |
| Sichtbarkeit | Statusleiste „Agent verbunden: <Client>“ mit „Trennen“ und „Nur lesen“; `Store::Change` bekommt `origin: User | Agent { client }`, der Verlauf zeigt es; nach jedem schreibenden Aufruf `emit("agent-changed", { assets })` und der `reloadPlan` wie nach Undo; ⌘Z nimmt Agenten-Schritte zurück | Mensch und Agent im selben Projekt |
| Tests | die Brücke bekommt `/mcp` auf derselben Sitzung; die Agenten-Testumgebung wird Kunde des MCP-Servers; Gate-Schritt `mcp` | eine Schnittstelle statt zwei |
| Einordnung | eigene Phase 3h nach 3g und vor Phase 4 (M1 und M2 Pflicht, M3 Gate der Phase, M4 optional); braucht den Plugin-Host nicht, Phase 5 baut später auf derselben Befehls-API auf | das Gate von Phase 4 wird kopflos fahrbar |

## Was bewusst nicht entschieden ist

- Stadtkarte und Flugziele für neue Orte (Recherche-Posten).
- Eigener Startort und übersprungenes Intro für die leere Region (Spike zu ARM9-Patches).
- Die Hierarchie frei andockbar (Spike in 3e.4).
- Einordnung von 3d (Erzählen) relativ zu Phase 4.
