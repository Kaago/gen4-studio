# Phase 3b.3 "Daten-Editoren": Ziel-UI aus dem Design-System "Doppeldeck"

Stand: 2026-10-23. Quellen: `Editor_DesignSystem.fig` (Seiten "Screens: Pokémon und Kämpfe", "Screens: Gegenstände, Texte, Testen", "Komponenten"), `design-system/README.md`, Spec 3c (`2026-10-23-phase-3c-doppeldeck-design.md`, §1, §5 bis §7), ROADMAP 3b.3.
PNGs (1440 px, alle Screens) liegen neben diesem Bericht: `pokedex.png`, `entwicklung.png`, `trainer.png`, `begegnungen.png`, `attacken.png`, `neuer-trainer.png`, `gegenstaende.png`, `laeden.png`, `komp-spiel-welt.png`, `komp-inhalt-status.png`.
Alle deutschen Texte unten sind wörtlich aus den Frames gelesen (Textknoten), nicht aus dem Gedächtnis.

Kürzel: **L** = Liste (272 px), **S** = Schau, **H** = Hand, **Sch** = Scharnier (44 px). "Neu in 3b.3" = Komponente, die Spec 3c ausdrücklich nicht gebaut hat (TypeBadge, StatBar, Budget, PokemonCard, StoryNode, RecipeButton) oder die in Spec 3c §6 gar nicht vorkommt (dann "fehlt in 3c §6").

---

## 0. Gemeinsamer Rahmen aller 8 Screens

- **Titelleiste** (40 px): Projekt "Sinnoh-Remix", Plakette "Platin, DE", Ungespeichert-Zeile ("1 Änderung nicht gespeichert", "2 Änderungen …", "3 Änderungen …", einmal "Alles gespeichert" mit Haken), Suchfeld "Suchen oder Befehl eingeben" + `⌘K`, Symbole Speichern, Bauen, Veröffentlichen, Thema, Hilfe. (Veröffentlichen-Symbol gibt es in 3c nicht; 3c hat Speichern, Bauen, Thema, Hilfe.)
- **Atlas** (88 px): Start, Welt, Geschichten, Pokémon, Gegenstände, Texte, Technik; unten Hilfe, Einstellungen. In allen Pokémon-Screens ist **Pokémon** aktiv (Bereichsfarbe Koralle), in Gegenstände/Läden ist **Gegenstände** aktiv (Bereichsfarbe Lagune/Türkis). In Spec 3c sind beide Einträge gesperrt ("Kommt mit Phase 3b.3"); 3b.3 muss sie freischalten.
- **Statusleiste** (28 px) lehrt je Screen Kürzel des aktiven Werkzeugs (siehe je Screen), rechts Kontext, "Gebaut vor 4 Minuten" (Haken), Dichte "Komfort".
- **Scharnier**: links Griff (sechs Punkte) + Brotkrume mit **drei Gliedern** (Bereich › Unterbereich › Auswahl, letzte fett), Mitte Tiefe als `Segmented` mit **drei** Segmenten "Erzählen | Bauen | Tüfteln", rechts Prüflicht ("Alles stimmig" grün, "1 Hinweis" ocker), Verlauf (Pfeil + Zahl, daneben Redo), Taste "Hier spielen ⌘R" (Bernstein). Im Assistenten wird es zur Schrittleiste (Screen "Neuer Trainer").
- **Hand-Höhe** schwankt je Screen (aus den Bildern): 286 (Pokédex), ca. 300 (Attacken, Gegenstände), ca. 380 (Trainer, Begegnungen, Läden, Neuer Trainer), ca. 410 (Entwicklung). Alles innerhalb der 200 bis 560 aus 3c §5.5; die Screens setzen also pro Editor eine Wunschhöhe.
- **Tiefe je Screen** (aus dem aktiven Segment): Bauen = Pokédex, Entwicklung, Trainer, Attacken, Gegenstände. **Erzählen = Begegnungen und Läden.** Neuer Trainer = Assistent. 3c baut nur Bauen und Tüfteln (Erzählen ist "später", Spec 3c §3/§13). Siehe §12 Lücken.
- **Tüfteln** erscheint in allen Datenscreens nicht als eigene Ansicht, sondern als **Rohdaten-Spalte/Abschnitt in der Hand** (Pokédex: Reiter "Rohdaten" mit Plakette "Tüfteln"; Attacken: Spalte "Tüfteln · Rohdaten"; Gegenstände: Spalte "Tüfteln · Rohdaten"). Das entspricht 3c §7.7 (Abschnitt "Technisch" im Inspector, hex), geht aber weiter: **Hex-Bytes des Rohdatensatzes mit markiertem Feld, Dateiname und Eintragsnummer**.

---

## 1. Pokédex: Staravia bearbeiten (Frame `0:9497`, Tiefe Bauen)

**Brotkrume:** `Pokémon › Pokédex › Staravia` (Symbole: Pfotenabdruck, Buch, Vogel). Titelleiste: "1 Änderung nicht gespeichert". Prüflicht "Alles stimmig", Verlauf 6.

**Liste ("Pokédex", Zähler "493 Arten", Aktionen: Sortieren-Symbol, `+`)**
- Suchfeld "Name, Nummer oder Kennung".
- **Typ-Filter als Plaketten-Reihe**: Normal, Flug (aktiv, mit Haken), Feuer, Wasser, Pflanze, Käfer, Link "12 weitere". Darunter Filterzeile "Typ Flug: 13 Pokémon" + Link "Filter lösen".
- Zeilen (`ListRow` mit Symbol): Name, Kennung grau mono (`SPECIES_ZUBAT`), Nummer rechts (`041`). Sortierung nach Nummer. Gewählt: Staravia (Kobalt-Strich links, blauer Ungespeichert-Punkt vor der Nummer 397).
- Sichtbare Zeilen: Zubat 041, Golbat 042, Iksbat 169 (`SPECIES_CROBAT`), Kramurx 198, Panzaeron 227, Staralili 396 (`SPECIES_STARLY`), Staravia 397, Staraptor 398, Driftlon 425, Drifzepeli 426, Kramshef 430, Plaudagei 441, Togekiss 468.

**Schau (Pokédex-Profil, "was der Spieler sehen wird")**
- Links ein **DS-Bildschirm-Profil** (nicht Textbox): Kopf "Nr. 397 · Staravia", Pokémon-Bild, Fuß "Star-Pokémon · 0,6 m 15,5 kg" in Pixelify Sans. **Platzhalter:** Vogel-Silhouette.
- Darunter **Größenvergleich**: Silhouetten "Spielfigur" und "Staravia", Block "Größe und Gewicht": "0,6 m" Größe, "15,5 kg" Gewicht, Satz "Halb so groß wie die Spielfigur."
- Rechts Kopfblock: Kleinzeile "Nr. 397, Star-Pokémon", Titel "Staravia" (Zen Maru), Kennung `SPECIES_STARAVIA`, Plakette "Ungespeichert", `TypeBadge` Normal + Flug, Beschriftung "Fähigkeit" + Chip "Bedroher".
- **Basiswerte** (`StatBar` x6): Zeilen KP 55, Angriff 85, Verteidigung 50, Sp.-Angriff 40, Sp.-Verteidigung 40, Initiative 80; rechts je "unter Schnitt"/"über Schnitt"; Legende "Strich: Schnitt aller 493 Pokémon". Die Zeile **Angriff trägt einen Kobalt-Strich links** (Abweichung vom Original). Summe-Zeile: "Summe 350 · vorher 340, Angriff von 75 auf 85 erhöht".
- **Fundorte** "4 Routen, 2 Trainer-Teams": `Reference`-Chips Route 209, Route 210, Route 212, Route 215 (Bereich Welt, grün) und Wanderer Kai, "Rivale Barry, Team 3" (Bereich Pokémon, koralle, Trainer-Symbol).
- **Reagiert auf:** Auswahl in der Liste, und live auf jede Änderung in der Hand (Balken, Summe, "vorher", Typen). Der Strich im Balken ist klickbar: "Klick auf den Strich setzt ihn zurück" (Hinweis in der Hand).

**Hand: Reiter "Werte ●", "Fähigkeiten", "Entwicklung", "Attacken", "Fundorte", "Rohdaten" + Plakette "Tüfteln"; rechts oben leise Taste "Auf Original zurücksetzen"** (Reiter "Werte" trägt Ungespeichert-Punkt; Reiter-Komponente mit "Ungespeichert"-Zustand aus README).
Drei Spalten, getrennt durch 1-px-Linien, Hand scrollt (Inhalt unten abgeschnitten):
1. **"Basiswerte"** (Zusatz "je 0 bis 255"): 3 x 2 `NumberField`: KP 55, Angriff 85 (fokussiert, Kobalt-Strich = `InspectorRow deviates`), Verteidigung 50, Sp.-Angriff 40, Sp.-Verteidigung 40, Initiative 80. Darunter Summenkarte "Summe 350 · vorher 340 · [+10]" und Satz "Angriff war 75. Klick auf den Strich setzt ihn zurück."
2. **"EV-Ertrag"** ("Was der Sieger trainiert"): `NumberField` 2 + `SelectField` "Initiative" + Taste "+ Wert". **"Fang-Rate"** ("1 bis 255, höher ist leichter"): Zahlfeld 120 + dreiteilige Skala schwer / mittel / leicht mit Marker; Satz "Mittel, wie Bidifas: Ein Pokéball fängt Staravia bei voller KP in etwa 16 % der Würfe." **"Basis-EP"** Zahlfeld 113 "für den Sieg".
3. **"Wachstum"**: `SelectField` "Mittellangsam" + Mini-Kurve, "1.059.860 EP bis Level 100". **"Geschlecht"** `SelectField` "50 % weiblich". **"Ei-Gruppe"** `SelectField` "Flug". **"Schlüpfen"** Zahlfeld 15 "Zyklen, 3.840 Schritte".

Statusleiste: "Werte bearbeiten · `↑↓` ändert um 1 · `⇧↑↓` um 10 · `⌘Z` macht rückgängig"; rechts "Staravia, SPECIES_STARAVIA, 397".

**Komponenten:** Atlas-Eintrag, Titelleiste, `PanelHead` (L), `TextField search`, **TypeBadge (neu)** als Filter-Plakette und Anzeige, `ListRow`, DSScreen (**neue Variante "Pokédex-Profil"**, 3c kennt nur textbox/sign/empty), **StatBar (neu)**, `Reference` (Welt, Pokémon), `Badge` (Ungespeichert), **Reiter/Tabs (fehlt in 3c §6)**, `NumberField`, `SelectField`, `InspectorRow` (Abweichung), `Button` (quiet "Auf Original zurücksetzen", "+ Wert"), Scharnier, `Kbd`. Nicht gezeichnet, aber fachlich passend: **Budget** nicht verwendet.

**Daten, die die Felder voraussetzen (Gen 4, Vorschlag zum Abgleich mit ROM):**
| Feld | Quelle (Gen 4) |
|---|---|
| Basiswerte, Typen (2), Fangrate, Basis-EP, EV-Ertrag (2 Bit je Stat), Fähigkeiten (2), Geschlechtsrate, Schlüpfzyklen, Wachstumsrate, Ei-Gruppen (2), TM/VM-Bits (100) | Personal-NARC (`poke_data`/`personal`), 44 Byte je Art |
| Name (DE), Kennung | Textarchiv Artennamen; `SPECIES_*` ist **generiert**, das ROM hat nur Indexe |
| "Star-Pokémon" (Kategorie), Größe, Gewicht | Pokédex-Daten (Zukan-Dateien, Texte), nicht Personal; Ablage prüfen |
| "Halb so groß wie die Spielfigur" | berechnet gegen feste Spielfigur-Höhe |
| Schnitt aller 493 | berechnet |
| "vorher 340", Kobalt-Strich, "Auf Original zurücksetzen" | **Basis-ROM-Vergleich nötig** (Original-Werte vorhalten) |
| Fundorte | **Rückwärtsreferenz** über Begegnungstabellen und Trainerteams |
| "Fang-Rate ... etwa 16 % der Würfe" | berechnete Fangformel (Gen 4), Vergleichs-Pokémon "Bidifas" wählt die App |
| Schritte aus Schlüpfzyklen (15 → 3.840) | berechnet (Zyklen x 256) |

**Design wish / Vorsicht:** nur **eine** Fähigkeit und **eine** Ei-Gruppe gezeichnet, Gen 4 hat je zwei Plätze (Reiter "Fähigkeiten" ist nicht gezeichnet). EV-Ertrag mit mehreren Werten über "+ Wert" ist eine Vereinfachung der sechs 2-Bit-Felder. "Spielfigur" und Größenvergleich brauchen eine Referenzhöhe, die nicht im ROM steht.

---

## 2. Entwicklung und Lernset: Staravia (Frame `0:10104`, Tiefe Bauen)

**Brotkrume:** `Pokémon › Staravia › Entwicklung und Lernset` (Symbole Pfotenabdruck, Vogel, Verzweigung). Titelleiste "2 Änderungen nicht gespeichert". Prüflicht "**1 Hinweis**" (ocker), Verlauf 9. **Liste identisch zum Pokédex** (Staravia gewählt).

**Schau: "Entwicklungskette"** ("Staralili-Familie, 3 Stufen"). Rechts oben `Segmented` "**Kette** | Tabelle" und Taste "+ Entwicklung hinzufügen".
- Drei Karten "Stufe 1 · Nr. 396 Staralili", "Stufe 2 · Nr. 397 Staravia" (gewählt, Kobalt-Rand), "Stufe 3 · Nr. 398 Staraptor"; je Symbol (Platzhalter), Name, Kennung (`SPECIES_STARLY` …), `TypeBadge` Normal + Flug, Fuß "Basiswerte 245 / 350 / 475" und "11 Attacken" / "**Lernset geändert**" (blauer Punkt) / "13 Attacken".
- Pfeile mit Bedingungs-Plaketten "Level 14" und "Level 34", darunter "beim Level-up".
- Hinweis (Info-Symbol): "Das Spiel prüft Entwicklungen nach jedem Kampf. Trägt Staravia einen Ewigstein, bleibt es Staravia."
- Reagiert auf: die gewählte Art; Kartenklick wechselt die Art.

**Hand: zwei Gruppen nebeneinander.**
- Links Reiter "**Level-Attacken ●**", "Ei-Attacken", "Attacken-Lehrer"; rechts Taste (primär, einzige) "+ Attacke hinzufügen". Tabelle mit Spalten **Level, Attacke (Name + Kennung mono), Typ (TypeBadge), Kategorie (Symbol + "physisch" / "Status" / "speziell"), Stärke, Genauigkeit, AP**, am Zeilenende "…"-Menü. Zeilen: 1 Tackle (`MOVE_TACKLE`, Normal, physisch, 35, 95, 35); 1 Heuler (Status, –, 100, 40); 5 Ruckzuckhieb (40, 100, 30); 9 Flügelschlag (Flug, 60, 100, 35); 13 Doppelteam (Status, –, –, 15); 18 Notsituation (–, 100, 5); 23 Wirbelwind (Status, –, 100, 20); **26 Luftschnitt mit Plakette "neu"** (Flug, speziell, 75, 95, 20; gewählt, Level-Feld im Bearbeiten, "x" zum Entfernen); 28 Aero-Ass (60, –, 20); 33 Bodycheck (90, 85, 20). Fußzeile mit Warnung (ocker): "Luftschnitt nutzt Sp.-Angriff, und der ist bei Staravia nur 40." und Zähler "12 Attacken".
- Rechts Reiter "**TM/VM-Kompatibilität**", Zähler "27 von 100". "Technische Maschinen 01 bis 92": Raster 01 bis 92 aus kleinen Umschaltern (aktiv = blau mit Haken, Mono-Ziffern); "VM 01–08": 8 Umschalter. Hover-Tooltip "**TM40 Aero-Ass, Flug**". Legende "01 kann lernen / 02 nicht"; leise Taste "**Wie Staralili**" (Vorlage kopieren).

Statusleiste: "Lernset bearbeiten · `⌘N` neue Attacke · `⌫` entfernt Zeile · `⌥↑↓` verschiebt"; rechts "Staravia, 12 Level-Attacken, 27 TM/VM".

**Komponenten:** Pokédex-Liste wie oben, **Pokémon-Entwicklungskarte** (kein Katalogbaustein; **neu, fehlt in 3c**; ähnelt Story-Knoten/Graph), `Segmented`, `Button`, `Reiter` (fehlt in 3c §6), **Tabelle** (fehlt in 3c §6: Listenzeile reicht nicht, braucht Spalten, Kopf, Inline-Edit), **TypeBadge (neu)**, `NumberField` (Level-Feld), `Notice warning`, `Badge` ("neu", "Lernset geändert"), **TM-Raster** (custom Kästchen-Raster, neu), `Tooltip`, `Menu` ("…").

**Daten:** Entwicklungen = bis zu 7 Einträge je Art (Methode, Parameter, Ziel), im ROM an der **Vorstufe**; die Kette ist abgeleitet. Level-Lernset = Liste (Attacke 9 Bit + Level 7 Bit, abgeschlossen), eine Datei je Art. Ei-Attacken und Attacken-Lehrer liegen in Gen 4 **nicht** in einem NARC (arm9/Overlay-Tabellen, Platin und HGSS verschieden): **design wish, Patch-Layer nötig**. TM-Kennzeichen = Bits im Personal-Datensatz, die TM-Zuordnung (TM40 = Aero-Ass) steht in einer arm9-Tabelle. "Ewigstein"-Satz ist Festtext. Die Warnung "nutzt Sp.-Angriff …" ist App-Logik (Attacken-Kategorie gegen Basiswert).

---

## 3. Trainer: Wanderer Kai (Frame `0:11082`, Tiefe Bauen)

**Brotkrume:** `Pokémon › Trainer › Wanderer Kai`. Titelleiste "3 Änderungen nicht gespeichert". Prüflicht "Alles stimmig", Verlauf 11.

**Liste ("Trainer", "214 in Sinnoh", `+`)**
- Suchfeld "Name, Klasse oder Kennung". `Segmented` "**Nach Ort** | Nach Klasse".
- Baum (`TreeRow`): Orte mit Kennung und Zähler: "Route 202 `0343` · 2" (offen), Kinder (`ListRow`, Symbol je Klasse): "Knirps Tristan `TR_0063` Lv. 5", "Göre Natalie `TR_0064` Lv. 6"; "Route 203 `0344` 5", "Route 204 `0345` 4", "Erzelingen `0005` 3", "Route 207 `0348` 3" (offen): "**Wanderer Kai** `TR_0142` Lv. 18" (gewählt, Ungespeichert-Punkt), "Wanderer Bodo `TR_0143` Lv. 17", "Camper Tim `TR_0144` Lv. 16"; "Route 208 `0349` 6", "Ewigenau `0040` 4", "Route 209 `0350` 7", **"Ohne Ort · 2"**.
- Level rechts je Zeile = höchstes Level im Team (Kai: Lv. 18 = Staravia).

**Schau (zwei Hälften)**
- Links "**Kampfvorschau** · so beginnt der Kampf im Spiel": DS-Bildschirm mit Kampfszene, Textbox "Wanderer Kai möchte kämpfen!", Statuspunkte oben (Team 3 von 6). **Platzhalter:** gezeichneter Kampfhintergrund, Spieler- und Trainerfigur als Blobs.
- Rechts "**Sichtlinie in der Welt** · Kachel 12, 2, Blick nach unten", Link "**In der Welt öffnen**": Draufsicht-Ausschnitt der Karte mit Etikett "Route 207 `0348`", Trainer als Weltmarker (gewählt, Kobalt-Halo), gestrichelte blaue Sichtlinien-Zellen und Etikett "Sieht 4 Kacheln nach unten". **Platzhalter:** gezeichnetes Gras/Bäume; die App rendert hier die three.js-Szene (3a), nicht Bilder.
- Reagiert auf: Trainerwahl (Kampftext, Sprite, Sichtlinie), Änderungen an Klasse/Name/Sichtweite/Team.

**Hand (Einzelspalte Team breit + Spalte Trainer)**
- **"Team"** ("3 von 6 Plätzen, Level 16 bis 18"), rechts `Segmented` "**Einzelkampf** | Doppelkampf". Sechs **PokemonCard** (neu): Kleinstein Lv. 16 (Gestein, Boden; "Trägt nichts"; Attacken Lehmsuhler, Steinpolitur, Steinwurf, Intensität), Onix Lv. 17 (Gestein, Boden; "Trägt nichts"; Kreideschrei, Steinwurf, Wut, Felsgrab), Staravia Lv. 18 (gewählt, Kobalt-Rand; Normal, Flug; "Trägt Sinelbeere" mit Beeren-Symbol; Ruckzuckhieb, Flügelschlag, Heuler, Doppelteam), dann **Leer-Karten** (gestrichelt) "Pokémon hinzufügen · Platz 4 von 6", "… Platz 5 von 6", "… Platz 6 von 6". Attacken-Plaketten tragen einen Typ-Punkt.
- **Spalte "Trainer `TR_0142`"** (Inspector-Zeilen, Bezeichnung 116 px): **Klasse** `SelectField` "Wanderer" (mit Symbol), **Name** `TextField` "Kai", **Belohnung** "**720 ₽**" + "40 ₽ mal Level 18" (berechnet, nicht editierbar), **KI-Stufe** `Segmented` "einfach | **klug** | Experte", **Sichtweite** `NumberField` 4 "Kacheln", **Kampftexte** `Reference`-Chips (Bereich Texte, sandfarben) "Vor dem Kampf", "Niederlage", **Gegenstände** `Reference`-Chips (Bereich Gegenstände, türkis) "Supertrank", "Supertrank" + Hinweis "2 von 4, setzt sie bei wenig KP ein".

Statusleiste: "Team bearbeiten · `1–6` wählt Platz · `⌘D` dupliziert · `⌫` leert Platz"; rechts "Wanderer Kai, Route 207, TR_0142".

**Komponenten:** `PanelHead`, `TextField search`, `Segmented`, `TreeRow`, `ListRow`, **PokemonCard (neu; Zustände Ruhe, Gewählt, Leer)**, **TypeBadge (neu)**, DSScreen (**neue Variante "Kampfvorschau"**), Weltmarker + Szene-Ausschnitt (3c hat Marker, aber keinen Ausschnitt-Viewer mit Sichtlinie), `SelectField`, `TextField`, `NumberField`, `Reference` (Texte, Gegenstände), `Button`, `Kbd`.

**Daten und Annahmen:**
| Feld | Quelle (Gen 4) / Annahme |
|---|---|
| Team (bis 6), Level, Art, Attacken (optional), Beutel-Item (optional), Doppelkampf-Flag, Klasse, 4 Gegenstände, KI-Flags | Trainerdaten-NARC (trdata + Teamdatei je Trainer; Typ-Byte steuert "mit Attacken"/"mit Item") |
| Name | Textarchiv Trainernamen (geteilt, **Längenlimit**!) |
| Klassenname, Klassenicon | Textarchiv Klassennamen, Klassentabelle |
| **Belohnung "40 ₽ mal Level 18"** | Gen 4 speichert **kein Geld je Trainer**, sondern einen Faktor **je Klasse**; "Belohnung" ist deshalb eine Klassen-Eigenschaft und wäre bei Bearbeitung für alle Trainer der Klasse wirksam. Als berechneter Text korrekt, als Feld nicht |
| **KI-Stufe "einfach/klug/Experte"** | in Gen 4 ein **Bitfeld mehrerer KI-Flags**; die drei Stufen sind eine **Vereinfachung** (Abbildung festlegen, "Tüfteln" zeigt die Flags) |
| Kampftexte (Vor dem Kampf, Niederlage) | Trainertext-Tabelle (Trainer-ID → Text-IDs) in einem Textarchiv; Link in die Texte-Fläche |
| Ort, "Nach Ort", "Ohne Ort" | **abgeleitet:** Trainer-Events (Welt) aller Karten scannen (Event trägt Trainer-ID/Script); ID-zu-Ort ist keine ROM-Spalte |
| Sichtweite, Blickrichtung, Kachel 12,2 | **Weltevent**, nicht Trainerdatensatz; Editor schreibt in die Eventdatei (3a/3b.1) |
| Sprite, Kampfhintergrund | ROM-Grafik (Trainer-Kampfsprite), s. §10 |
| `TR_0142`, `0343`, `0348` | Index (Trainer-ID, Header-ID); `TR_*` ist generiert |

**Design wish:** "Wanderer Kai möchte kämpfen!" setzt der Editor aus Klassenname + Name + Kampftext-Vorlage des Spiels zusammen. Die Zahl "214 in Sinnoh" passt nicht zu Platin (mehrere hundert Datensätze, viele unbenutzt/Dubletten); die App muss alle zeigen, ungeortete unter "Ohne Ort".

---

## 4. Begegnungen: Route 201 (Frame `0:11753`, Tiefe **Erzählen**)

**Brotkrume:** `Pokémon › Begegnungen › Route 201` (Pfotenabdruck, Wellen/Gras-Symbol, Karte). Titelleiste "1 Änderung nicht gespeichert". Verlauf 4. **Keine Liste**: die Schau reicht bis an den Atlas (Liste ausgeblendet; der Ort kommt aus der Brotkrume).

**Schau: Weltszene** (Draufsicht) über die ganze Breite.
- Schwebende Leiste oben links: **"Draufsicht | Spielkamera | Begegnungen"** (Draufsicht aktiv, Begegnungen als Ebene aktiv).
- Hohe-Gras-Flecken als Umriss mit Punktmuster, beschriftet "Gras A · 28 Kacheln", "Gras B · 36 Kacheln", "Gras C · 36 Kacheln", "Gras D · 28 Kacheln"; Ausgänge als Etiketten "nach Zweiblattdorf", "nach Sandgemme", "zum See der Wahrheit"; Vorsprung (orange Pfeile).
- **Legendenkarte oben rechts** "**Wer im Gras wartet**" ("tags"): Bidiza "2 von 5", Staralili "1 von 3", Sheinux "1 von 6", Knospi "1 von 11" (Symbol je Art, Zahlplakette).
- Reagiert auf: Tageszeit-Reiter und Methode in der Hand (Legende zeigt "tags"), gewählter Platz (Slot 5 Sheinux ist gewählt).

**Hand (zwei Spalten):**
- **Methoden-Reiter** mit Zählern: "**Gras 12**", "Surfen 0", "Alte Angel 0", "Profiangel 0", "Superangel 0", "Honigbaum 0", "Poké-Radar 4", "Schwarm 1" (leere Methoden gedämpft); rechts Hinweis (Info-Symbol): "Kein Wasser, kein Honigbaum hier".
- `Segmented` "**Morgens | Tags | Nachts**" (Tags aktiv) + Satz "Wer tagsüber im Gras wartet. Morgens und nachts tauschen nur Platz 3 und 4."
- **12 Platzkarten** in zwei Spalten (1 bis 6 | 7 bis 12): Nummer, Symbol, Name, "Level n", Balken, "1 von N Begegnungen", Prozent. Belegung:
 1 Staralili L2 (1 von 5, 20 %), 2 Bidiza L2 (1 von 5, 20 %), 3 Staralili L3 (1 von 10, 10 %), 4 Bidiza L3 (10 %), 5 Sheinux L3 (10 %, **gewählt**), 6 Bidiza L4 (10 %), 7 Staralili L4 (1 von 20, 5 %), 8 Sheinux L4 (5 %), 9 Knospi L3 (1 von 25, 4 %), 10 Knospi L4 (4 %), 11 Sheinux L4 (1 von 100, 1 %), 12 Knospi L4 (1 %).
- **Rechte Spalte:** Meldung (Erfolg, grün) "**100 % verteilt**" / "Die 12 Plätze ergeben zusammen genau eine ganze Begegnung."; Abschnitt "**Wie oft passiert etwas?**" mit Kennung "`rate 30`", Satz "**Jeder 10. Schritt im Gras**", **Slider** (selten ... oft); zwei **Rezept-Tasten (RecipeButton, neu)**: "**Seltenes Pokémon einbauen** · Belegt Platz 11 und 12, je 1 von 100" (Kürzel `S`) und "**Wie Route 202 verteilen** · Holt alle 12 Plätze, danach anpassen" (Kürzel `W`).

Statusleiste: "Begegnungen verteilen · `Klick` wählt Pokémon · `Ziehen` tauscht Plätze"; rechts "Route 201, 0342, Gras tags".

**Komponenten:** Szene (3a) + schwebende `Segmented`/Toggle-Leiste (3c hat "Gelände | Ereignisse"), Legende als Karte (3c: `Popover`), Wassermarker/Ausgangsetiketten (**neue Overlay-Art**, 3c: Muster/Draufsicht = Phase 6), **Reiter mit Zählerplakette (fehlt in 3c §6)**, `Segmented`, **Platzkarte** (neu; mit `TypeBadge`-losem Kopf, Balken = `StatBar`-artig; Drag-Tausch), **RecipeButton (neu)**, `Notice success`, **Slider** (fehlt in 3c §6), `Reference`/Symbol je Art, `Kbd`.

**Daten und Annahmen:**
| Feld | Quelle (Gen 4) / Annahme |
|---|---|
| Gras: 12 Slots (Art + **ein** Level je Slot), Gras-Rate | Encounter-NARC, **ein Datensatz je Encounter-Eintrag des Headers** (nicht je Grasfleck) |
| Slotwahrscheinlichkeiten 20/20/10/10/10/10/5/5/4/4/1/1 | **fest im Spiel**, nicht editierbar |
| Morgens/Nachts: ersetzt Slots 3 und 4 (Platz 3 und 4) | Gen 4: zwei Ersatzarten je Tageszeit ✓ |
| Surfen, Alte/Profi-/Superangel | je 5 Slots mit Level-Spanne (min/max) + Rate; Gras hat nur ein Level |
| Poké-Radar (4), Schwarm | Datensatzfelder ✓; Schwarm in Platin an Ort/Tag gebunden |
| **Honigbaum** | in Gen 4 **nicht** je Route, sondern globale Tabellen im arm9/Overlay: **design wish** |
| Gras A bis D, "28 Kacheln" | **abgeleitet** aus Gelände "hohes Gras" der Karte (zusammenhängende Flächen); Tabelle gilt für alle Flecken des Headers |
| "100 % verteilt" | immer wahr, weil die Slotanteile fest sind (Editor wählt nur Art/Level je Slot); **design wish**, könnte entfallen oder "Zusammenfassung" heißen |
| **"rate 30 = Jeder 10. Schritt"** | Umrechnung Rate → Schritte **prüfen** (Gen 4 Rate ist Prozent-/Zähler-Wert, nicht schlicht "jeder N-te") |
| Name der Arten (DE) | Textarchiv Artennamen |
| "1 von 5", "2 von 5" | berechnet aus Slotanteilen |

**Slot bearbeiten:** Auswahl eines Platzes (Rahmen) zeigt **nirgends** Felder für Art (Picker) und Level; im Screen fehlt dieser Teil (**Designlücke**, s. §12). Gras-Ansichten "Wie Route 202 verteilen" setzt Kopie einer anderen Tabelle voraus.

---

## 5. Attacken: Flügelschlag (Frame `0:12687`, Tiefe Bauen)

**Brotkrume:** `Pokémon › Attacken › Flügelschlag` (Pfotenabdruck, Blitz, Stern/Burst). Titelleiste "1 Änderung nicht gespeichert". Verlauf 3.

**Liste ("Attacken", Zähler "467", Sortieren, `+`)**
- Suchfeld "Name oder Kennung". Typ-Plaketten Flug (aktiv), Normal, Kampf, Psycho, "14 weitere"; `SelectField` "**Alle Kategorien**" (Filtersymbol); Filterzeile "Typ Flug: 19 Attacken" + "Filter lösen".
- Zeilen: **Kategorie-Symbol** (Wirbel = speziell, Strahlen = physisch, Halbkreis = Status), Name, Kennung mono, Nummer. Beispiele: Windstoß 016 `MOVE_GUST`, Flügelschlag 017 `MOVE_WING_ATTACK` (gewählt, Punkt), Fliegen 019, Pikser 064, Bohrschnabel 065, Spiegeltrick 119, Himmelsfeger 143, Luftstoß 177, Federtanz 297, Windschnitt 314, Aero-Ass 332, Sprungfeder 340, Ruheort 355.

**Schau (Karte + DS-Bildschirm)**
- **Karte** (weiße Kachel): `TypeBadge` Flug + Plakette "physisch", Nummer "Nr. 017", Titel "Flügelschlag", Kennung `MOVE_WING_ATTACK`; drei Kennzahlen "60 Stärke", "100 % Genauigkeit", "35 AP"; Merkmal-Plaketten "Macht Kontakt", "Schutz wirkt", "Königsstein", "Priorität 0, normal" (mit Haken); "Sehr effektiv gegen" Pflanze, Käfer, Kampf; "Schwach gegen" Gestein, Elektro, Stahl (`TypeBadge`); Abschnitt "**Verwendet in**" mit Plaketten "14 Lernsets", "3 Trainern", Symbolreihe und "Staralili, Staravia, Staraptor und 11 weitere".
- **DS-Bildschirm "Attacken-Zusammenfassung"** (Pixelify): Kopf "Flügelschlag" + Typfeld "Flug", Zeilen "Kategorie Physisch", "Stärke 60", "Genauigkeit 100", "AP 35/35", Textbox "Trifft den Gegner mit weit / ausgebreiteten Flügeln."; Fuß (grünes Häkchen) "Passt in die Textbox: längste Zeile 26 von 38 Zeichen".
- Reagiert auf: Auswahl, alle Felder der Hand live (Karte und Zusammenfassung).

**Hand (drei Spalten):**
1. **"Grundwerte":** Name `TextField` "Flügelschlag", Typ `SelectField` "Flug" (Typpunkt), Kategorie `Segmented` "physisch | speziell | Status", Stärke `NumberField` 60, Genauigkeit 100 "%", AP 35 "mit AP-Plus höchstens 56", Priorität `SelectField` "0, normal" "von -7 bis +5".
2. **"Verhalten":** Ziel `SelectField` "Ein Pokémon neben dir", Nebeneffekt `SelectField` "Keiner", **Merkmale** als `Checkbox`: Kontakt, Schutz wirkt, Magiespiegel, Königsstein, Spiegeltrick, Übernahme (Kobalt-Strich an "Königsstein" = Abweichung); **Wettbewerb** `SelectField` "Coolness" + "2 Herzen" (Herz-Skala).
3. **"Tüfteln" · Plakette "Rohdaten":** "Effekt-ID" `0x0000` "normaler Treffer", "Flags-Byte" `0x33` "vorher 0x13" (hervorgehoben), Hexblock 16 Byte mit markiertem Byte, Fußzeile "`pl_waza_tbl.narc`, Eintrag 17, 16 Byte".

Statusleiste: "Attacke bearbeiten · `⌘F` sucht · `Leertaste` schaltet Kästchen · `⌘Z` macht rückgängig"; rechts "Flügelschlag, MOVE_WING_ATTACK, 017".

**Komponenten:** wie Pokédex-Liste, **TypeBadge (neu)**, `Segmented`, `Checkbox`, `NumberField`, `SelectField`, `TextField`, `InspectorRow`, `Badge`, `Reference` (Lernsets/Trainer: Verwendet-in-Plaketten), DSScreen (**neue Variante "Attacken-Zusammenfassung"**), `Notice`-artige Prüfzeile (Budget-Gedanke: "26 von 38", **Budget (neu)** hier **implizit**, nicht als Balken gezeichnet), **Rohdaten-Hexblock** (fehlt in 3c; 3c §7.7 beschreibt Dez|Hex-`NumberField` und "Technisch"-Abschnitt, aber keinen Hex-Dump).

**Daten:** `pl_waza_tbl.narc`, **16 Byte je Attacke** (✓ genannt im Screen): Effekt-ID (Kampfskript-Index), Kategorie, Stärke, Typ, Genauigkeit, AP, Effektchance, Ziel, Priorität, Flags-Byte (Kontakt, Schutz, Magiespiegel, Königsstein, Spiegeltrick, Übernahme), Wettbewerbswerte. Name und Beschreibung aus Textarchiven (DE), Kennung `MOVE_*` generiert. "Sehr effektiv/Schwach gegen" kommt aus der **Typentabelle (arm9)**, berechnet. "AP-Plus höchstens 56" berechnet (AP x 1,6). "Verwendet in 14 Lernsets, 3 Trainern" = Rückwärtsreferenz über alle Lernsets und Trainerteams.
**Design wish / Vorsicht:** Nebeneffekt-Auswahl "Keiner" setzt eine **Liste von Effekten** voraus; Gen 4 kennt nur eine Effekt-ID (Skript im Kampf-Overlay), eine freie Namensliste muss die App selbst pflegen. Beschreibungstext-Grenze **38** gilt für Textboxen; Attacken- und Item-Beschreibungen haben eigene Zeilenbreiten (Items: "30 Zeichen" im Gegenstände-Screen), 38 hier ist **zu prüfen**.

---

## 6. Neuer Trainer: Team (Frame `0:13301`, Assistent Schritt 3 von 5)

**Scharnier als Schrittleiste:** Bereichssymbol (Pokémon) + Titel "**Neuer Trainer**" (Zen Maru 15), Schritte "✓ Ort und Figur — ✓ Klasse — **3 Team** — 4 Texte — 5 Zusammenfassung", rechts "Abbrechen", "Zurück" (sekundär), "**Weiter ↵**" (primär). Titelleiste "**Alles gespeichert**" (Assistent baut erst am Ende). Statusleiste: "Assistent: Neuer Trainer · `↵` weiter · `⌘←` zurück · `Esc` bricht ab"; rechts "Schritt 3 von 5, Route 202". **Keine Liste.**

**Schau: Weltszene mit Live-Vorschau** (Draufsicht, ganze Breite).
- Neuer Trainer als Marker mit gestrichelter Sichtlinie und Etikett "**Wanderer Jens** `neu` · sieht 3 Kacheln nach links"; existierender "Rivale Barry · kämpft mit Lv. 9"; Ausgänge "nach Jubelstadt", "nach Sandgemme".
- Karte "**Bisher festgelegt**" (Haken): Ort "Route 202, Kachel 23, 4", Figur "Wanderer, Blick nach links", Name "Jens", Hinweis "Ändern mit Zurück oder Klick auf die Figur".

**Hand (drei Spalten):**
1. "**Pokémon für Platz 2**" (Kennung "Picker"): **Picker inline** (kein Dialog): Suchfeld mit Eingabe "star", Bereichsplakette "Pokémon", "3 Treffer": Staralili (`SPECIES_STARLY, 396`, "passt zu Lv. 7–9", gewählt), Staravia (`SPECIES_STARAVIA, 397`, Warnung "erst ab Lv. 14"), Staraptor (`SPECIES_STARAPTOR, 398`, "erst ab Lv. 34"); Fuß `↑↓` wählen, `↵` übernehmen, `Esc` schließen.
2. "**Team von Wanderer Jens**" ("1 von 6"): Karte Kleinstein Lv. 8 (Gestein, Boden, "Attacken nach Level"); **Platz 2 gewählt** (Kobalt-Rand, gestrichelt, Lupe): "Platz 2 wird gewählt" / "Vorschau: Staralili, Lv. 8"; vier Leer-Karten "Pokémon hinzufügen · Platz 3/4/5/6 von 6".
3. "**Vorschlag**" (Karte): Titel "**Level passend zur Route: 7–9**", Satz "Wilde Pokémon auf Route 202 haben Level 2 bis 4, die anderen Trainer dort 5 bis 7.", Taste (leise/umrandet, Zauberstab) "**Level 8 für alle**"; darunter Meldung (Info): "**Barry kämpft hier mit Level 9; ein Wanderer bleibt darunter**" / "So bleibt der Kampf gegen deinen Rivalen der schwerste auf Route 202. Höchstens Level 8 passt."

**Komponenten:** Scharnier im Assistentenmodus (3c §5.5), **Picker (3c: Dialog; hier inline** mit Vorschau/Warnung je Treffer: Dialog-Variante "inline" fehlt), **PokemonCard (neu)**, `Notice info`, `Button`, `Kbd`, Szene mit Live-Vorschau (3c: "Live-Vorschau der Assistenten in der Schau (gestrichelte neue Zellen)" ist ausdrücklich **nicht in 3c**), `Badge` ("neu"), **TypeBadge (neu)**.

**Daten und Logik:** Vorschläge sind **App-Heuristik** (Wildlevel der Route aus Begegnungen, Level der anderen Trainer auf der Route, Rivalenlevel): **design wish**, keine ROM-Daten. "erst ab Lv. 14" = niedrigstes Level der Entwicklung (Entwicklungsdaten). Platzierung "Route 202, Kachel 23, 4" = Weltevent. Die Schritte "Texte" und "Zusammenfassung" sind nicht gezeichnet.

---

## 7. Gegenstände: Trank (Frame `0:13876`, Tiefe Bauen)

**Brotkrume:** `Gegenstände › Medizin › Trank` (Rucksack, Tasche, Fläschchen). Titelleiste "1 Änderung nicht gespeichert". Verlauf 3.

**Liste ("Gegenstände", "467", Sortieren, `+`)**
- Suchfeld "Name oder ITEM_ suchen". **Taschen-Plaketten mit Zähler**: "Medizin 38" (aktiv), "Bälle 16", "Basis-Items 40", "Beeren 64", "TM/VM 100", "Basis 12" (die letzten beiden Beschriftungen sind **unklar**: "Basis" und "Basis-Items" sind nicht erklärt, vermutlich Kampf-/Basis-Taschen).
- Zeilen (Symbol je Gruppe), Name, Kennung, **Preis rechts**: Trank `ITEM_POTION` 300 ₽ (gewählt), Supertrank `ITEM_SUPER_POTION` 700 ₽, Hypertrank 1.200 ₽, Top-Trank `ITEM_MAX_POTION` 2.500 ₽, Top-Genesung `ITEM_FULL_RESTORE` 3.000 ₽, Gegengift 100 ₽, Para-Heiler 200 ₽, Feuerheiler 250 ₽, Eisheiler 250 ₽, Aufwecker 250 ₽, Hyperheiler 600 ₽, Beleber 1.500 ₽, Tafelwasser 200 ₽.
- Fußleiste: "38 in Medizin" und "● 1 geändert".

**Schau:** oben `Segmented` "**Karte | Im Beutel | Im Laden**", rechts "Vorschau in Spielschrift, 1,5-fach".
- **Karte (links):** Symbol (Platzhalter-Fläschchen), Titel "Trank", Kennung `ITEM_POTION`, "Nr. 0017", Plaketten "Tasche Medizin", "Im Kampf nutzbar", Preis "300 ₽ · Kaufpreis"; **DS-Beschreibungsbox** (3 Zeilen): "Ein Spray-Heilmittel, das die / KP eines Pokémon um 20 Punkte / auffrischt." + Prüfzeile "Beschreibung im Beutel: 3 Zeilen, längste 29 von 30 Zeichen"; drei Kennzahlkarten "Verkauf **150 ₽** halber Kaufpreis", "Wirkung **Heilt 20 KP** ein Pokémon", "Schleuder **30** Wurfstärke".
- **"Verwendet in"** (rechts, "4 Läden, 12 Funde, 2 Trainer", Taste "Alle zeigen"): "**Läden 4**": Pokémarkt Sandgemme, Jubelstadt, Erzelingen, Flori; "**Funde 12**": Route 201 2x, Route 202, Route 203 3x, Erzelingen-Mine, Route 204 2x, Jubelstadt, Flori, Route 205; "**Trainer 2**": "Barry, Route 203", "Arenaleiter Veit"; Hinweis "Klick öffnet die Stelle, Rechtsklick zeigt sie in der Welt." Alles `Reference`-Chips (Welt grün, Pokémon koralle).

**Hand (zwei Spalten):**
- Kopf: Titel "Trank", Untertitel "Gegenstand 17 von 467, Tasche Medizin", Plakette "Ungespeichert", Symboltasten Duplizieren, "…".
- Spalte 1 **links**: **Preis** `NumberField` 300 "₽" + "Verkauf 150 ₽"; **Tasche** `SelectField` "Medizin"; **Wirkung** `SelectField` "Heilt KP" + Zahl 20 "KP"; **Sortierung** Zahl 1 "vorher 3" (Kobalt-Strich = Abweichung). Spalte 2: **Im Kampf** `Switch` "nutzbar", **Im Beutel** `Switch` "nutzbar", **Schleuder** Zahl 30 "Wurfstärke", **Verkaufbar** `Checkbox` "Läden kaufen an". Darunter Meldung (Info): "Der Preis gilt in allen 4 Läden. Läden mit eigenem Preis behalten ihn."
- **Spalte "Tüfteln · Rohdaten"** (rechts): "item_data.narc, Eintrag 17" (Plakette); "Item-ID" `0017` `0x0011` "fest"; "Hold-Effekt" `0x00` `0x00` "kein"; "Feldfunktion" `0x05` "HP_RESTORE"; Hexdump 16 Byte (`00 | 2C 01 | 00 00 00 00 1E 00 | 08 | 00 00 05 01 00 14 00 00`) mit markierten Bytes; Fuß "Byte 0 bis 1: Preis, 0x012C = 300".

Statusleiste: "Gegenstand bearbeiten · `Tab` nächstes Feld · `⌘D` duplizieren · `⌘⇧F` Verwendungen suchen"; rechts "Medizin, Trank, 1 von 38".

**Komponenten:** `PanelHead`, `TextField search`, Taschen-Plaketten (**Filter-Chip mit Zähler**, fehlt in 3c §6), `ListRow` mit Trailing (Preis), `Segmented`, DSScreen (**neue Varianten "Beschreibung/Beutel/Laden"**), `Reference` mit "Verwendet in" (3c: Rechtsklick-Menü; hier **ausgeklappte Gruppe** mit Zähler + "Alle zeigen"), `NumberField`, `SelectField`, `Switch`, `Checkbox`, `Notice info`, Hexblock (fehlt), `Badge`, `IconButton`, `Menu`, `Kbd`. **TypeBadge/StatBar/PokemonCard/Budget** nicht verwendet.

**Daten:** `item_data.narc`, ein Datensatz je Gegenstand (Preis u16, Hold-Effekt + Parameter, Schleuderstärke, Tasche, Kampf-/Feldfunktion, Heil-Parameter wie "20 KP", Bytebelegung stimmt zur Gen-4-Struktur). Beschreibung und Name aus Textarchiven (Beschreibung 3 Zeilen). `ITEM_*` generiert. "Verkauf = halber Preis" abgeleitet. **"Verwendet in" Funde und Trainer:** Funde stecken in **Scripts/Events** (Item-Balls rufen Skripte), also Rückwärtsreferenz über Events + Scripts (3b.2 hat keine Rückwärtsreferenzen!): **design wish, aufwendig**. Läden: aus den Shop-Tabellen. "Sortierung" und "Verkaufbar" (Checkbox) in dieser Form im ROM **prüfen**; "**Läden mit eigenem Preis behalten ihn**" existiert in Gen 4 **nicht** (Preis ist global je Item), also **design wish / zu streichen**.

---

## 8. Läden: Pokémarkt Jubelstadt (Frame `0:14466`, Tiefe **Erzählen**)

**Brotkrume:** `Gegenstände › Läden › Pokémarkt Jubelstadt`. Titelleiste "2 Änderungen nicht gespeichert". Verlauf 2.

**Liste ("Läden", "10", `+`)**
- Suchfeld "Laden oder Ort suchen". Gruppenköpfe "**Pokémärkte**" und "**Fachgeschäfte**" (Schmale Köpfe).
- Zeilen mit Symbol, Name, Kennung, rechts Plakette "+n": Pokémarkt Sandgemme `MART_SANDGEM`; **Pokémarkt Jubelstadt `MART_JUBILIFE` +3** (gewählt); Erzelingen `MART_OREBURGH` +1; Flori `MART_FLOAROMA`; Ewigenau `MART_ETERNA` +2; Herzhofen `MART_HEARTHOME` +1; Weideburg `MART_PASTORIA` +2; Fachgeschäfte: Kaufhaus Schleiede `SHOP_VEILSTONE_DEPT`, Kräuterladen Ewigenau `SHOP_ETERNA_HERB`, Blumenladen Flori `SHOP_FLOAROMA_FLOWER`.
- Fuß: "**+3: Gegenstände, die es nur dort gibt**" (Legende für "+n").

**Schau:** links **DS-Bildschirm "Laden"**: Feld "Geld 3.000", Item-Symbol-Feld, Preisliste mit Cursor (Pokéball 200, Superball 600, Hyperball 1.200, **Trank 300** (Cursor), Supertrank 700, Gegengift 100, Schutz 350), Beschreibungsbox "Ein Spray-Heilmittel. Füllt die KP / eines Pokémon um 20 Punkte auf."; Fuß "Laden mit 3 Orden, 256 × 192, 1,5-fach". Rechts: Titel "Pokémarkt Jubelstadt", Kennung `MART_JUBILIFE, Laden 0003`, Satz "So sieht der Laden aus, wenn der Spieler 3 Orden hat. Wähle einen anderen Stand, um zu sehen, was dann im Regal liegt.", **"Orden-Stand für die Vorschau"** (Segmente 0 bis 8, 3 aktiv), Kennzahlkarten "Im Regal **8** bei 3 Orden", "Nur hier **3** zweite Kasse", "Neu ab 3 Orden **1** Hyperball", Zeile "**Steht in**": `Reference` "Jubelstadt" (Welt) und "Verkäuferin, Kasse 1" (Bereich Geschichten/Figur, violett). Reagiert auf: Ordenstand und alle Haken in der Hand.

**Hand (zwei Spalten, Erzählen):**
- **"Was gibt es ab wie vielen Orden?"** (Zusatz "Ein Haken heißt: ab da im Regal"): **Matrix**: Zeilen = Gegenstände (Symbol + Name + Preis rechts), Spalten = Orden 0 bis 8 mit Orden-Symbol, Spalte 3 hervorgehoben (aktueller Vorschau-Stand). Zeilen: Pokéball 200 ₽ (**Schloss-Symbol, alle Haken gesperrt**), Trank 300, Gegengift 100, Supertrank 700 (Abweichung: Kobalt-Strich; ab Orden 1), Superball 600, Schutz 350, Fluchtseil 550, Hyperball 1.200 (ab 3), Hypertrank 1.200 (ab 5). Meldung unten (Schloss): "Pokébälle gibt es ab 0 Orden in jedem Markt (Spielregel)." + Link "**Warum?**".
- **Spalte "Nur in diesem Laden"** (Zähler 3): Satz "Die zweite Kasse verkauft das immer, egal wie viele Orden."; Zeilen Heilball ("heilt beim Fangen", 300 ₽), Netzball ("gut gegen Wasser und Käfer", 1.000 ₽), Nestball ("gut gegen schwache Pokémon", 1.000 ₽), je mit "x" zum Entfernen; **primäre Taste "+ Gegenstand hinzufügen"**; sekundäre Taste "**Sortiment eines anderen Ladens übernehmen**".

Statusleiste: "Sortiment bearbeiten · `Klick` Haken setzen · `⇧ Klick` ganze Zeile · `⌫` entfernen"; rechts "Pokémarkt Jubelstadt, 8 im Regal, 3 nur hier".

**Komponenten:** `ListRow` mit Gruppen (Spec: Gruppenköpfe wie "Orte"/"Spielweit" in Texte), `TextField search`, DSScreen (**neue Variante "Laden"**), `Segmented` (0 bis 8 als Ordensstand), Kennzahlkarten, `Reference` (Welt, Figur), **Checkbox-Matrix** (fehlt in 3c; `Checkbox` existiert, Matrix/Zeilenklick mit ⇧ nicht), `IconButton`, `Button` primär/sekundär, `Notice`/Schloss-Zeile, `Kbd`. **Kein RecipeButton** (trotz Erzählen: die zwei Tasten sind normale Tasten), kein TypeBadge.

**Daten und Annahmen:**
- Gen 4 kennt **Pokémärkte** mit einem Standardsortiment, das sich mit der **Zahl der Orden** erweitert, plus **eine zweite Kasse** mit festen Sonderartikeln ("Nur hier"): das Datenmodell des Screens (Stand je Orden 0 bis 8, "zweite Kasse") passt grundsätzlich, die Tabellen liegen aber in **arm9/Overlay**, nicht in einem NARC (**design wish: Overlay-Patch**; Platin und HGSS unterscheiden sich).
- **Fachgeschäfte** (Kaufhaus mit Etagen, Kräuterladen, Blumenladen) und Plätze wie "Verkäuferin, Kasse 1" kommen aus **Scripts/NPC-Events**; "Steht in" ist eine **Rückwärtsreferenz** über Scripts.
- `MART_*`/`SHOP_*`-Kennungen und "Laden 0003" sind **erfunden/generiert**; das ROM hat nur Indexe.
- "Pokébälle ab 0 Orden in jedem Markt (Spielregel)" ist Engine-Regel (gesperrte Zeile).
- "Läden mit eigenem Preis" (vom Gegenstände-Screen) kommt hier nicht vor; Preise stehen nur im Item-Datensatz.
- "Sortiment eines anderen Ladens übernehmen" ist Kopie einer Tabelle.

---

## 9. Querschnitt: gemeinsame Muster

**Liste (immer `PanelHead`: Titel + Zähler grau + Sortieren + `+`):**
- Suchfeld (`TextField search`) mit Platzhalter je Bereich: "Name, Nummer oder Kennung" (Pokédex), "Name, Klasse oder Kennung" (Trainer), "Name oder Kennung" (Attacken), "Name oder ITEM_ suchen" (Gegenstände), "Laden oder Ort suchen" (Läden). Suche über Name **und** Kennung (Picker-Regel, 3c §6).
- Filter-Plaketten direkt unter der Suche (Typ für Pokédex/Attacken mit "n weitere" und "Filter lösen"; Taschen mit Zähler für Gegenstände); Filterzeile "Typ Flug: 13 Pokémon" + "Filter lösen".
- Gruppierung: Trainer nach Ort (Baum) oder Klasse (Segmented), Läden nach Pokémärkte/Fachgeschäfte, Gegenstände nach Tasche (Filter), Pokédex/Attacken flach nach Nummer.
- **Zeilen = Symbol + Name + Kennung grau mono + Nummer/Preis/Level rechts**; Gewählt = Kobalt-Strich links + Hintergrund; **Ungespeichert-Punkt** an der Zeile; kleine Zähler/Plaketten rechts ("+3", Level, Anzahl Trainer pro Ort).
- Fußzeile mit Summe und Änderungszähler (Gegenstände: "38 in Medizin · 1 geändert"; Läden: Legende "+3: …").

**Schau = "was der Spieler sieht"** je Editor: DS-Bildschirm (Pokédex-Profil, Kampfvorschau, Attacken-Zusammenfassung, Item-Beschreibung, Laden) oder Weltszene (Trainer: Sichtlinie, Begegnungen: Gras-Overlay, Neuer Trainer: Live-Vorschau). Daneben eine **Kartenansicht der Daten** (Bedeutungen, "Verwendet in", Größenvergleich, Verfügbarkeit).

**Hand:** 2 bis 3 durch 1-px-Linien getrennte Spalten, **Reiter nur wenn viele Gruppen** (Pokédex, Entwicklung, Begegnungen), sonst Spalten; Abschnittsüberschriften fett mit grauem Nebensatz ("Basiswerte — je 0 bis 255"); **Rohdaten ("Tüfteln") als letzte Spalte/Reiter** mit Dateiname, Eintrag, Hexdump und markiertem Feld; **Abweichung vom Original** als Kobalt-Strich und "vorher …" (Pokédex Angriff, Attacken Flags-Byte, Gegenstände Sortierung, Läden Supertrank).

**Verwendet in:** Attacken (Plaketten "14 Lernsets", "3 Trainern" + Symbolzeile), Gegenstände (Gruppen Läden/Funde/Trainer als `Reference`-Chips + "Alle zeigen"), Pokédex ("Fundorte"), Läden ("Steht in"). Immer **Referenz-Chips in Bereichsfarbe**, Klick öffnet, Rechtsklick zeigt in der Welt.

**Zähler:** Listenköpfe (493 Arten, 214 in Sinnoh, 467, 10), Reiter ("Gras 12", "Poké-Radar 4"), Plaketten ("+3", "3 Trainern"), Hand-Abschnitte ("Team · 3 von 6 Plätzen", "TM/VM · 27 von 100", "Nur in diesem Laden · 3"), Statusleiste ("12 Level-Attacken, 27 TM/VM", "8 im Regal, 3 nur hier").

**Bulk-Aktionen:** "Wie Staralili" (TM-Kompatibilität kopieren), "Level 8 für alle", "Wie Route 202 verteilen", "Seltenes Pokémon einbauen", "Sortiment eines anderen Ladens übernehmen", ⇧-Klick setzt ganze Zeile (Läden), ⌘D duplizieren (Trainer, Gegenstände), "Auf Original zurücksetzen", "Filter lösen".

**Rückmeldungs-Sätze mit Rechenergebnis** (App-Logik, nicht ROM): "Mittel, wie Bidifas: … in etwa 16 % der Würfe", "Halb so groß wie die Spielfigur", "Luftschnitt nutzt Sp.-Angriff, und der ist bei Staravia nur 40.", "Level passend zur Route: 7–9", "Beschreibung im Beutel: 3 Zeilen, längste 29 von 30 Zeichen", "Passt in die Textbox: längste Zeile 26 von 38 Zeichen". Alle als kleine Zeile mit Haken/Warnsymbol (Meldung/Budget-Muster, **nie blockierend**).

**Statusleiste lehrt Kürzel je Editor** (Liste oben je Screen): `↑↓`/`⇧↑↓`, `⌘N`, `⌫`, `⌥↑↓`, `1–6`, `⌘D`, `⌘F`, `Leertaste`, `Tab`, `⌘⇧F`, `Klick`/`⇧ Klick`/`Ziehen`, `↵`, `⌘←`, `Esc`.

---

## 10. Platzhalter-Grafiken (keine Nintendo-Assets)

Der Design-Datei (`README.md`: "Pokémon-, Figuren- und Spielgrafiken sind bewusst Platzhalter") liegt nichts Nintendo-eigenes bei. Die App muss **aus dem geöffneten ROM** rendern:
- **Pokémon-Bild** im Pokédex-Profil (Vogel-Silhouette) und im Größenvergleich: Pokémon-Frontsprite aus dem Sprite-NARC (NCGR/NCLR), Spielfigur-Silhouette: Spielersprite.
- **Listen-/Karten-Symbole der Pokémon** (koralle Quadrate mit Linien-Glyphen, auch in Team-Karten und Begegnungs-Plätzen): Menüsymbole (32 x 32) aus dem ROM.
- **Item-Symbole** (Fläschchen, Pillen, Bälle): Item-Icon-NARC (24 x 24).
- **Trainer-Kampfsprite und Kampfhintergrund** in der Kampfvorschau: Trainer-Sprite aus dem ROM, Hintergrund aus den Kampfhintergründen.
- **Weltausschnitt** (gezeichnetes Gras, Bäume, Wege) in Trainer, Begegnungen, Neuer Trainer: die **three.js-Szene** (3a) mit Gelände-Overlay und Markern, kein Bild; Karten-Thumbnails existieren (thumbnailRenderer).
- **DS-Bildschirm-Inhalte** (Pokédex-Profil, Kampf, Zusammenfassung, Laden, Beschreibungsbox): Rahmen/Fläche/Schrift (Pixelify) der App, Hintergründe und Symbole aus dem ROM; die Pixelify-Schrift ist Stimme des Spiels (DS-Original-Font steht nicht zur Verfügung).
- **Behalten** (eigene Gestaltung, kein Nintendo-Asset): Typfarben/-namen, Kategorie-Glyphen (Wirbel, Strahlen, Halbkreis), Bereichsfarben, Weltmarker, Phosphor-Symbole.

---

## 11. Atlas-Bereiche und Zuordnung

- **Pokémon** (aktiv in Pokédex, Entwicklung und Lernset, Trainer, Begegnungen, Attacken, Neuer Trainer): hier leben **auch Trainer, Begegnungen, Attacken**. Brotkrumen-Glied 2 trägt den Unterbereich ("Pokédex", "Trainer", "Begegnungen", "Attacken").
- **Gegenstände** (Gegenstände, **Läden**): Brotkrumen-Glied 2 "Medizin" bzw. "Läden".
- **Geschichten** wird **nur verlinkt**: "Verkäuferin, Kasse 1" als Figur (Bereichsfarbe violett) und Kampftexte (Bereich Texte); **Trainer selbst liegen nicht in Geschichten.** Der Story-Knoten "Kampf" (`Kampf gegen Rivale Barry, Team 1`) verweist auf Trainer, kommt aber in 3b.4 dran.
- **Welt** wird als Referenz benutzt (Orte als Chips: Route 209, Jubelstadt …) und als Schau-Szene (Trainer-Sichtlinie, Begegnungs-Overlay, Assistent). **Texte**: Kampftexte, Namen, Beschreibungen (Referenz-Chips sandfarben, "Nachricht 12").
- **Offen:** Der Atlas hat **keine Unterpunkte**; wie man zwischen Pokédex, Trainer, Begegnungen und Attacken (und zwischen Gegenstände und Läden) wechselt, ist in den Screens **nicht gezeichnet** (Vorschlag: Brotkrumen-Glied 2 als `Menu`, oder Liste mit Kopf-Umschalter). Spec 3c kennt nur einen Bereich je Atlas-Eintrag.

---

## 12. Lücken, Widersprüche zu Spec 3c, was die App neu bauen muss

**Von Spec 3c als "kommen mit ihrem Editor" ausgeschlossen und hier benutzt:**
| Komponente | Wo |
|---|---|
| **TypeBadge** | Pokédex (Filter, Kopf), Entwicklung (Karten, Tabelle), Trainer (Karten), Attacken (Liste, Karte, Wirkung), Neuer Trainer |
| **StatBar** | Pokédex (Basiswerte); evtl. Begegnungs-Platzkarten (Balken je Slot) |
| **PokemonCard** | Trainer (6 Plätze, Zustände Ruhe/Gewählt/Leer), Neuer Trainer |
| **RecipeButton** | Begegnungen ("Seltenes Pokémon einbauen", "Wie Route 202 verteilen") |
| **Budget** | **nicht** als Balken gezeichnet; implizit in "längste Zeile 26 von 38", "29 von 30", Teamgröße "3 von 6 Plätzen" |
| StoryNode | in keinem dieser Screens |

**In Spec 3c §6 gar nicht vorgesehen, aber in den Screens nötig (neu in 3b.3):**
- **Tabs/Reiter** (README-Komponente "Reiter", in §6 nicht aufgeführt; §7.3 erwähnt Base UI Tabs nur für Scripts): Pokédex-Hand, Level-Attacken, Methoden in Begegnungen, TM/VM.
- **Tabelle mit Spaltenkopf** (Lernset, Läden-Matrix).
- **Slider** (Begegnungsrate), **Filter-Chips** (Typen, Taschen mit Zähler, "n weitere"), **Kennzahlkarte** (Stärke/Genauigkeit/AP, Verkauf/Wirkung/Schleuder), **Hexblock** mit Markierung, **Mini-Kurve** (Wachstum), **Entwicklungskarte + Pfeil**, **TM-Raster**, **Checkbox-Matrix mit ⇧-Zeilenklick**, **Orden-Segmente 0 bis 8**.
- **Picker inline** (3c: nur Dialog) mit Warnzeile je Treffer.
- **DSScreen-Varianten** (3c: textbox, sign, empty): Pokédex-Profil, Kampfvorschau, Attacken-Zusammenfassung, Item-Beschreibung, Laden.
- **Tiefe Erzählen** (Begegnungen, Läden) und **Rezepte**: 3c baut nur Bauen/Tüfteln; **Entscheidung nötig**: Erzählen-Segment mit den zwei Rezept-Ansichten in 3b.3 mitbauen, oder Begegnungen/Läden zunächst im Bauen-Layout (Platzkarten + Matrix) ausliefern.
- **Szene:** Overlay "Begegnungen" (Gras-Flecken, Beschriftung), **Draufsicht-Kamera** (3c: Phase 6!), **Sichtlinien-Overlay**, **Live-Vorschau in der Schau** (3c §1: ausdrücklich nicht in 3c).
- **Titelleiste:** Symbol "Veröffentlichen" (3c: Phase 4).
- **Hand-Höhe je Editor** (Wunschwert statt globaler 286; `shellState` merkt Höhe je Bereich, nicht je Unterbereich).

**Design wish: die Screens nehmen an, was ein Gen-4-ROM nicht (so) hergibt**
1. Trainer-**Belohnung** je Trainer (ROM: Faktor je Klasse).
2. **KI-Stufe** einfach/klug/Experte (ROM: Bitfeld mehrerer Flags).
3. **Läden mit eigenem Preis** (ROM: Item-Preis global).
4. **Honigbaum** als Begegnungsmethode je Route (ROM: globale Tabelle in arm9/Overlay).
5. **Ei-Attacken, Attacken-Lehrer, Läden-Sortimente, Orden-Staffelung** liegen in arm9/Overlay-Tabellen (nicht in NARCs): **Patch-Layer** nötig, Platin und HGSS verschieden.
6. **"100 % verteilt"**: Gen-4-Slotanteile sind fest; nur Art und Level je Slot sind frei.
7. **Rate → "Jeder 10. Schritt"**: Umrechnung ungeprüft.
8. **Kennungen** `SPECIES_*`, `MOVE_*`, `ITEM_*`, `TR_*`, `MART_*`, `SHOP_*` sind **generiert**, `Laden 0003`/`Header 0348` sind Indexe.
9. **Fundorte, Funde, Trainer-Verwendungen, "Steht in", Orte der Trainer** sind **Rückwärtsreferenzen** über Begegnungen, Events und Scripts. 3b.2 schloss "Rückwärtsreferenzen" aus; hier werden sie zum Kernstück ("Verwendet in" im README-Prinzip 7). Braucht einen Referenzindex im Kernel.
10. **"vorher …", Kobalt-Strich, "Auf Original zurücksetzen"** brauchen die Originalwerte des Basis-ROMs (Phase 4 hat SHA-1-Prüfung der Basis-ROM; Diff hier **früher**).
11. **Vorschläge** (passende Levels, "wie Bidifas", "erst ab Lv. 14") sind App-Heuristik, keine ROM-Daten.
12. **Zwei Fähigkeiten, zwei Ei-Gruppen, mehrere EV-Stats** werden vereinfacht gezeigt (Tab "Fähigkeiten" nicht gezeichnet).
13. **Textlängen:** Zeichenlimits im Screen (38, 30) müssen je Feld nachgemessen werden (Attacken-Beschreibung, Item-Beschreibung, Trainername).
14. **Anzahlen im Screen** (214 Trainer, 10 Läden, 467 Items) sind Beispielzahlen, nicht die Platin-Zahlen.

**Weitere Designlücken:**
- Slot-Editor in Begegnungen (Art-Picker + Level) fehlt, Gras-Rate-Slider hat keine Zahleneingabe, Methoden Surfen/Angeln haben **Level-Spannen** (Gras nur ein Level).
- Wechsel zwischen den Unterbereichen im Atlas (siehe §11).
- Kein leerer Zustand, kein Fehlerzustand und kein "Nicht rückgängig"-Zustand gezeichnet (z. B. neue Trainer/Läden anlegen via `+`).
- Reiter "Fähigkeiten", "Fundorte", "Rohdaten" (Pokédex) und "Ei-Attacken", "Attacken-Lehrer" (Entwicklung) sind nur als Titel sichtbar.
- Assistent "Neuer Trainer": Schritte 1, 2, 4, 5 nicht gezeichnet.
- 3b.3-Lieferumfang laut ROADMAP: **Encounter, Trainer, Pokémon-Basisdaten, Moves, Items** (uxie); **Läden, Entwicklungen/Lernsets, TM-Kompatibilität, Ei-Attacken** stehen dort **nicht** ausdrücklich (3c nennt "Bereiche Pokémon und Gegenstände mit Läden (3b.3)"). Umfang entscheiden.
