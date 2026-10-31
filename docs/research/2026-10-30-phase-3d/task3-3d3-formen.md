# 3d.3 Task 3: Formen der Kampfsprites am Augenschein (Messprotokoll, 2026-10-31)

Gegenstand: die Zuordnung der Member von `otherPokemonBattleSprites` (Platin `files/poketool/pokegra/pl_otherpoke.narc`,
HeartGold/SoulSilver `files/a/1/1/4`) zu Art, Form, Seite und Palette. Gemessen an den Testkopien Platin DE (CPUD,
`_testkopien/3d-3-pt`) und HeartGold DE (IPKD, `3d-3-hgde`); HeartGold US (IPKE, `3d-3-hgus`) ist Member für Member
byte-gleich mit HeartGold DE. Nur gelesen. Dieses Protokoll enthält keine Bilder und keine Bytes, nur Nummern, Magics,
Zuordnungen und Zählungen. Ergebnis: `forms` in `crates/g4-kernel/metadata/battle_sprites.yaml`, Tabelle in
`docs/formats/ncgr_nclr.md` (Abschnitt „Formen“).

## 1. Vorgehen

1. **Inventar.** Eine Kopie von `gutachten-c-sprites-inventar.py` im Scratchpad druckt das Magic jedes Members des
   Ordners `unpacked/otherPokemonBattleSprites` (Aufruf `python3 -I <kopie> <DSPRE-Ordner>`). Die Kopie liegt nicht im
   Repo.
2. **Bilder.** Der ignorierte, gated Test `dump_sprites_for_inspection` (`crates/g4-kernel/tests/battle_sprites.rs`)
   schreibt jedes NCGR entschlüsselt (vorwärts) als ganzes 160 × 80-Bild in einen Ordner außerhalb des Repos, einmal mit
   einer Grauleiter (der Grauwert ist der Palettenindex) und einmal mit jeder NCLR der folgenden vier Member. Weil die
   Paletten hier in einem Block hinter den Bildern liegen, trifft die zweite Art nur den Schwanz des Archivs; die
   Zuordnung der Paletten lief über Kontaktbögen, die ein Skript im Scratchpad aus den Graubildern und den NCLR der
   Kopie neu färbte (Bild und Palette als Paar, mit Member-Nummer beschriftet).
3. **Augenschein.** Art, Form und Seite am Graubild (Rückseiten sind größer und unten angeschnitten), die Palette am
   stimmigen Farbbild: falsche Paletten zeigen Füllfarben (reines Magenta) oder fremde Farben.
4. **Gegenprobe ohne Auge.** Die Farben jeder Form-Palette verglichen mit den Paletten `6·s + 4` (normal) und `6·s + 5`
   (schillernd) von `pokemonBattleSprites`: gleiche oder fast gleiche Paletten verankern die Grundform jeder Art
   (Abschnitt 3). Die entschlüsselten Bilder von Platin und HGSS verglichen (Prüfsumme der Graubilder) und die
   Palettenfarben Platin `k` gegen HGSS `k + 4`.

Zeitaufwand der Augenschein-Arbeit: etwa 45 Minuten von zwei Stunden. Alle Pflicht-Einträge und alle weiteren Formen
sind belegt.

## 2. Member-Folge der Magics

| Member | Platin DE (253) | HeartGold DE (261) |
|---|---|---|
| Bilder | 0–153 `RGCN` (154) | 0–157 `RGCN` (158) |
| Paletten | 154–247 `RLCN` (94) | 158–255 `RLCN` (98) |
| Schwanz | 248 `RGCN`, 249 `RGCN`, 250 `RLCN`, 251 `RGCN`, 252 `RLCN` | 256 `RGCN`, 257 `RGCN`, 258 `RLCN`, 259 `RGCN`, 260 `RLCN` |
| Summe | 157 NCGR, 96 NCLR, kein leeres Member | 161 NCGR, 100 NCLR, kein leeres Member |

Jedes NCGR ist linear (Modus 1), 160 × 80, 4 bpp; jede NCLR hat genau eine Palette zu 16 Farben. Mittel der
zusammenhängenden Flächen (linkes Bild, vorwärts entschlüsselt): 1,1 auf beiden Spielen.

## 3. Zuordnung

`f` ist der Formindex des Spiels. Die Bilder liegen je Art in einer von zwei Ordnungen: **paarweise** (je Form erst
Rückseite, dann Vorderseite) oder **seitenweise** (erst alle Rückseiten, dann alle Vorderseiten). Die Paletten
ebenso: **paarweise** (je Form normal, schillernd) oder **blockweise** (erst alle normalen, dann alle schillernden).

| Platin-Member | Inhalt | Bildordnung | Paletten (Platin) | Palettenordnung | Anker in `pokemonBattleSprites` |
|---|---|---|---|---|---|
| 0–7 | Deoxys, Formen 0–3 (normal, Angriff, Verteidigung, Initiative) | paarweise | 154, 155 für alle Formen | – | 154 = 386 normal, 155 = 386 schillernd (gleich) |
| 8–63 | Icognito A–Z, „!“, „?“ (Formen 0–27) | paarweise | 156, 157 für alle Formen | – | 156 = 201 normal, 157 = 201 schillernd |
| 64–71 | Formeo: Rückseiten 64–67, Vorderseiten 68–71 (normal, Sonne, Regen, Schnee) | seitenweise | 158–165 | blockweise | 158 = 351 normal; 162 weicht in 1 von 16 Farben von 351 schillernd ab |
| 72–77 | Burmy: Pflanzen-, Sand-, Lumpenumhang | paarweise | 166–171 | paarweise | 166 = 412 (dort normal = schillernd) |
| 78–83 | Burmadame: Pflanzen-, Sand-, Lumpenumhang | paarweise | 172–177 | paarweise | keiner (413 weicht in 13 von 16 Farben ab) |
| 84–87 | Schalellos: Rückseiten 84 (West), 85 (Ost), Vorderseiten 86, 87 | seitenweise | 178–181 | paarweise | 178 weicht in 2 Farben von 422 ab (dort normal = schillernd) |
| 88–91 | Gastrodon: Rückseiten 88 (West), 89 (Ost), Vorderseiten 90, 91 | seitenweise | 182–185 | paarweise | 182 = 423 (dort normal = schillernd) |
| 92–95 | Kinoso: Rückseiten 92 (Wolke), 93 (Sonne), Vorderseiten 94, 95 | seitenweise | 186–189 | blockweise | 189 = 421 schillernd, 187 weicht in 1 Farbe von 421 normal ab: die Paletten von 421 sind die der Sonnenform |
| 96–131 | Arceus, Formen 0–17 (Typindex, 9 „???“) | paarweise | 190–225 | paarweise | 190 = 493 normal, 191 = 493 schillernd |
| 132 | Ei (nur Vorderseite) | – | 226 | – | – |
| 133 | Manaphy-Ei (nur Vorderseite) | – | 227 | – | – |
| 134–137 | Shaymin: Land-, Zenitform | paarweise | 228–231 | paarweise | 228 = 492 normal, 229 = 492 schillernd |
| 138–149 | Rotom: normal, Hitze, Wasch, Frost, Wirbel, Schneid | paarweise | 232–243 | paarweise | 232 = 479 normal, 233 = 479 schillernd |
| 150–153 | Giratina: Wandel-, Urform | paarweise | 244–247 | paarweise | 244 = 487 normal, 245 = 487 schillernd; 247 (Urform schillernd) hat dieselben Farben wie 245 |
| 248, 249 | Rückseite, Vorderseite eines kleinen grünen Pokémon (Zweck unbekannt) | paarweise | 250 | – | – |
| 251 | drei flache Ellipsen (wohl Schatten) | – | 252 | – | – |

HeartGold/SoulSilver: die Bilder 0–149 und 152–153 sind nach dem Entschlüsseln gleich denen von Platin (Prüfsumme der
Graubilder), 150 und 151 (Giratina, Wandelform) sind neu gezeichnet. Neu sind 154–157: Pichu, Rückseite und
Vorderseite normal (154, 155) und Strubbelohr (156, 157), paarweise; Paletten 252–255 paarweise (252 weicht in 1
Farbe von 172 normal ab, 253 und 255 = 172 schillernd). Jede Palette Platin `k` (154–247) hat dieselben Farben wie
HGSS `k + 4` (gemessen, keine Ausnahme). Der Schwanz ist 256/257 + 258 und 259 + 260.

Am Augenschein geprüft (Kontaktbögen): Deoxys alle vier Formen beider Seiten; Icognito alle 28 Vorderseiten und
Rückseiten (A bis „?“); Formeo jede Vorderseite gegen jede der acht Paletten (32 Paare; nur mit der Zuordnung
„blockweise“ zeigt jede Form ihre bekannten Farben ohne Füllfarben); Burmy, Burmadame je Form normal und schillernd; Schalellos, Gastrodon, Kinoso je
Vorderseite gegen alle vier Paletten; Arceus alle 18 normalen Paletten (Typfarbe passt zum Typindex) und vier
schillernde; Ei, Manaphy-Ei; Shaymin (Zenitform); Rotom alle fünf Geräte normal und schillernd; Giratina (Urform)
normal und schillernd; in HGSS Pichu (beide Formen, alle vier Paletten), Giratina, Ei, Manaphy-Ei, Rotom, Deoxys,
Arceus, Formeo, Kinoso, Shaymin, Burmadame, Schalellos und der Schwanz als Stichprobe.

## 4. Entscheidungen

- **Je Familie eine Tabelle.** HGSS hat vier Bilder mehr vor dem Palettenblock, daher sind alle Palettennummern um 4
  verschoben; `forms` ist ein Wörterbuch je Familie (`pt`, `hgss`) wie `direction`, jede Liste ausgeschrieben. Eine
  Rechenregel („+ 4“) wäre kürzer, verliert aber den Bezug zur Messung und bräche an einem Hack mit eigener Ordnung.
- **Alle Formen, Form 0 eingeschlossen, und das Ei.** Ursprünglich standen nur die Formen ≠ 0 in der Tabelle, Form 0
  kam aus `pokemonBattleSprites`. Seit dem Nachtrag vom 2026-10-31 (Abschnitt 7) hat jede Art mit Formen auch einen
  Eintrag für Form 0 mit ihren Bildern in diesem Archiv; das Studio zeichnet diese Arten nur noch von hier.
- **Das Ei** (`{ species: 494, form: 0, personal: 494 }`) hat kein `back` und kein `shiny`: `shiny` ist deshalb
  auslassbar (`Option`), schillernd zeigt dann die normale Palette; die Rückseite ist `None`, ohne Rückfall auf eine
  Grundform.
- **Das Manaphy-Ei, 248–252** stehen nicht in der Tabelle (keine Form einer Art).
- **`form_of_personal`** findet auch unbestätigte Einträge: unbestätigt sind die Bilder, nicht die Form eines
  Personal-Members; `form_sprite` gibt unbestätigte Einträge nicht heraus, dann zeigt Form 0 (seit dem Nachtrag aus
  der Tabelle, sonst aus `pokemonBattleSprites`).

## 5. Nicht belegt, unsicher

- Kein Pflicht-Eintrag fehlt, kein Eintrag ist `unverified`.
- Welche der beiden Paletten einer Form normal und welche schillernd ist, folgt bei Formen ohne Anker aus der
  Ordnung der Art und dem Augenschein (die normalen Farben sind die bekannten); bei Formeo und Kinoso stützen die
  Anker die Ordnung „blockweise“.
- Was 248/249 und 251 zeigen, ist nicht bestimmt; das Studio nutzt sie nicht.
- Diamant/Perl: nicht gemessen (kein Projekt), keine Tabelle.

## 6. Zählungen (gated `vanilla_every_form_sprite_decodes`)

Stand Task 3 (nur Formen ≠ 0 und das Ei):

```
forms CPUD images=157 palettes=96 components=1.1 entries=65 rendered=258
forms IPKD images=161 palettes=100 components=1.1 entries=66 rendered=262
forms IPKE images=161 palettes=100 components=1.1 entries=66 rendered=262
```

Stand des Nachtrags vom 2026-10-31 (mit Form 0, Abschnitt 7):

```
forms CPUD images=157 palettes=96 components=1.1 entries=77 rendered=306 form0=12 form0_like_pokegra=[201, 351, 479, 487] female_only=3
forms IPKD images=161 palettes=100 components=1.1 entries=79 rendered=314 form0=13 form0_like_pokegra=[201, 351, 479, 487] female_only=3
forms IPKE images=161 palettes=100 components=1.1 entries=79 rendered=314 form0=13 form0_like_pokegra=[201, 351, 479, 487] female_only=3
```

`rendered` zählt die Bilder, die `battle_sprite` für die Einträge liefert (Vorderseite und Rückseite, normal und
schillernd; das Ei ohne Rückseite). Jede Form ≠ 0 unterscheidet sich von Form 0 ihrer Art (aus der Tabelle), und
jeder Personal-Member liefert dasselbe Bild wie seine Art und Form. Form 0 wird nicht mit sich selbst verglichen;
`form0_like_pokegra` nennt die Arten, deren Form 0 hier (Standbild, normal und schillernd, beide Seiten) genauso
aussieht wie ihr Eintrag in `pokemonBattleSprites`. `female_only` zählt die Einträge mit Plätzen „nur ♀“ (Burmadame,
Form 0 bis 2: Geschlechterverteilung 254).

## 7. Nachtrag 2026-10-31: Form 0 aus diesem Archiv

Anlass: die Abschlussprüfung von 3d.3 Teil A. Für Arten mit Formen ist der Eintrag in `pokemonBattleSprites` nicht
das, was das Spiel zeigt; das Spiel zeichnet diese Arten nur aus `otherPokemonBattleSprites`, auch in Form 0.
Gemessen an den drei Testkopien (nur gelesen; Vergleich der entschlüsselten Indizes je Hälfte und der 16 Farben je
Palette, Platin `k` gegen HGSS `k + 4`; beide Familien gleich, wo nicht anders gesagt):

| Art | Form-0-Member (Rückseite / Vorderseite, Paletten Platin) | Unterschied zu `pokemonBattleSprites` |
|---|---|---|
| Deoxys (386) | 0 / 1, 154 / 155 | Rückseite anders gezeichnet (Standbild 1 040 Pixel), dort ohne Animationsbild |
| Icognito (201) | 8 / 9, 156 / 157 | keiner |
| Formeo (351) | 64 / 68, 158 / 162 | Standbilder gleich; Rückseite dort ohne Animationsbild; schillernd 1 Farbe anders (ungenutzt) |
| Burmy (412) | 72 / 73, 166 / 167 | neu gezeichnet; dort schillernd = normal, hier weicht schillernd in 5 Farben ab |
| Burmadame (413) | 78 / 79, 172 / 173 | neu gezeichnet; dort schillernd = normal; Paletten weichen in 13 von 16 Farben ab |
| Schalellos (422) | 84 / 86, 178 / 179 | neu gezeichnet; dort schillernd = normal; normal 2, schillernd 10 Farben anders |
| Gastrodon (423) | 88 / 90, 182 / 183 | neu gezeichnet; dort schillernd = normal; schillernd 8 Farben anders |
| Kinoso (421) | 92 / 94, 186 / 188 | dort die Sonnenform (Bild und 15 von 16 Farben), hier die Wolkenform |
| Arceus (493) | 96 / 97, 190 / 191 | Vorderseite neu gezeichnet (Standbild 1 285 Pixel) |
| Shaymin (492) | 134 / 135, 228 / 229 | Vorderseite neu gezeichnet (Standbild 413 Pixel) |
| Rotom (479) | 138 / 139, 232 / 233 | Standbilder gleich; Animationsbild der Rückseite anders (329 Pixel) |
| Giratina (487) | 150 / 151, 244 / 245 | Standbilder gleich; Platin: Animationsbild der Vorderseite anders (681 Pixel) |
| Pichu (172), nur HGSS | 154 / 155, HGSS 252 / 253 | Vorderseite neu gezeichnet (Standbild 386 Pixel), normal 1 Farbe anders |

Am Augenschein (Bilder nur im Scratchpad, nicht im Repo): Kinoso Form 0 hier ist die geschlossene violette Knospe
(Wolkenform), `pokemonBattleSprites` zeigt die offene rosa Blüte (Sonnenform, wie Form 1 hier); Burmy schillernd zeigt
hier einen anders gefärbten Kopf, dort dieselben Farben wie normal; Burmadame hier mit dunklem Kopf und
Pflanzenumhang, dort mit braunrosa Kopf in fremden Farben. Deoxys, Formeo, Arceus und Giratina als Stichprobe: dieselbe
Art und Form, nur anders gezeichnet oder mit Animationsbild.

Entscheidung: Jede Art mit Formen hat einen Eintrag `form: 0` je Familie (Platin 12, HGSS 13 mit Pichu); die Member
sind die oben genannten. `battle_sprite` und `sprite_slots` lösen eine Art mit Formen nur über die Tabelle auf; eine
unbekannte oder unbestätigte Form zeigt Form 0 der Tabelle. Weil keine Form eigene weibliche Bilder hat, gelten die
Plätze einer Form als „männlich oder einzig“, bei einer rein weiblichen Art (Geschlechterverteilung 254: Burmadame) als
„nur weiblich“ (`pokemonBattleSprites` legt rein weibliche Arten in die weiblichen Plätze 0 und 2).
