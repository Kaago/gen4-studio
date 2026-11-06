# 6.3a Task 0: wie eindeutig nennen die Vanilla-Permissions das Gelände einer Textur (Messprotokoll, 2026-11-05)

Gegenstand: die Vorgaben je Kachel eines abgeleiteten Tilesets (`kind`, `collision`, HGSS-Geräusch), die Task 5 aus
den Vanilla-Permissions der Karten eines Kartentextur-Pakets ableitet, und die Palette je Kachel. Gemessen an den
Testkopien Platin DE (`_testkopien/6-3a-task0-pt`), HeartGold DE (`6-3a-task0-hgde`) und HeartGold US
(`6-3a-task0-hgus`) mit `task0-6-3a-vorgaben.py` (liest nur, druckt nur Zahlen). Dieses Protokoll enthält keine
Namen, keine Pixel und keine Bytes, nur Zählungen. Regeln K1 und K2 wie im Plan 6.3a, Zeile „Vorgaben je Textur“.

**Ergebnis:** `DEFAULT_RULE = VoteRule::K1`, `UNIQUE_SHARE = 0.6`; Palettenregel **ja**: Task 5 ergänzt „Palette mit
dem Index der Textur“ (Abschnitt 8). Die Wahl ist knapp und hängt an der Lesart der Übereinstimmung und an der
Zuordnung der Karten zu Paketen (Abschnitte 8, 9).

## 1. Vorgehen

Aufruf je Spiel, im Hintergrund, Ausgabe nach `target/agent/` (nicht im Repo):

```sh
python3 -I docs/research/2026-11-05-phase-6/task0-6-3a-vorgaben.py /Users/POLTEP/Projekte/lab/editor/Roms/_testkopien/6-3a-task0-pt --json > target/agent/6-3a-task0-pt.json 2> target/agent/6-3a-task0-pt.log
```

ebenso `hgde` und `hgus`; Laufzeit je Spiel 22 bis 32 Sekunden, Ausgang 0.

1. **Pakete.** Je `unpacked/mapTextures/NNNN` die Texturen (`read_btx`, wörtlich aus Gutachten C) und die Namen des
   Paletten-Wörterbuchs (neu, `palette_names`). Kontrolle: das Textur-Wörterbuch über `names_of_dict` liest dieselben
   Namen wie `read_btx` (in jedem Paket).
2. **(a) Karten je Paket.** Je Karte die Texturnamen, die ihr Modell über das Material-Wörterbuch bindet
   (`read_mdl`, wörtlich); die Karte gehört zu dem Paket mit den meisten Treffern dieser Namen, bei Gleichstand zum
   ersten (wie Gutachten C), ohne Treffer zu keinem.
3. **(b) Wahrheit.** Die Display-Listen ausgeführt (`run_dl`, `split`, `sbc_shape_materials`, wörtlich); je
   waagerechtem Polygon (alle Ecken auf einer Höhe), dessen Ecken alle auf dem 16er-Raster liegen, die überdeckten
   Zellen: Zellmitte im Polygon, Rand zählt (ein Dreieck an der Diagonale zählt die Zelle mit); Zelle (Spalte, Zeile) =
   (x / 16 + 16, z / 16 + 16), Index `Zeile · 32 + Spalte`. Je Textur des Pakets die `kind`-Werte dieser Zellen über
   alle Karten des Pakets; Wahrheit ist die Mehrheit, wenn ihr Anteil ≥ 0,6 ist, sonst „keine“. Wahrheit der
   Kollision: `0x80`, wenn mindestens die Hälfte der überdeckten Zellen Bit 7 trägt.
4. **(c) K1, K2** genau wie im Plan, aus der Karten-weiten Zählung: `votes(k) = Σ Zellen_m(k) / |Namen_m|` über die
   Karten des Pakets, die die Textur binden; K1 und K2 je `UNIQUE_SHARE` ∈ {0,6, 0,7, 0,8, 0,9} (K2 nutzt die Schwelle
   nur im Rückfall „Kind 0“). Kollision und Geräusch der entschiedenen wie im Plan (gewichtete Zellen mit `k*`). K2
   löst gleiche Quotienten `p_T(k) / base(k)` zum kleineren `k` auf (wie K1 gleiche `votes`).
5. **(d) Übereinstimmung** über die Texturen mit Wahrheit: das Ergebnis-`kind` der Regel gleich der Wahrheit, eine
   unklare Textur zählt mit `kind 0` (so steht sie im Tileset). Daneben zwei andere Lesarten (Abschnitt 8): „streng“
   (entschieden und richtig, gleicher Nenner) und „Trefferquote“ (richtig unter den entschiedenen mit Wahrheit).
6. **Gegenproben.** Ausrichtung der Zellen (x gespiegelt, z gespiegelt, Achsen vertauscht) und Zuordnung bei
   Gleichstand (Karte zu allen gleich guten Paketen), Abschnitte 3 und 9.

## 2. Pakete und Texturen

| Größe | Platin DE | HeartGold DE | HeartGold US |
|---|---|---|---|
| Pakete (lesbar) | 74 | 106 | 106 |
| Texturen gesamt | 3 130 | 3 659 | 3 659 |
| Paletten gesamt | 3 000 | 3 628 | 3 628 |
| Textur-Wörterbuch weicht von `read_btx` ab | 0 | 0 | 0 |
| doppelte Texturnamen in einem Paket | 0 | 0 | 0 |
| Karten gelesen / nicht lesbar | 666 / 0 | 676 / 0 | 676 / 0 |

Die Kontrollzahlen 3 130 / 74 und 3 659 / 106 treffen. HeartGold US ist in jeder Zahl gleich HeartGold DE bis auf die
Zahl der nicht waagerechten Polygone (265 766 statt 265 781); alle abgeleiteten Zahlen sind gleich. Die Tabellen unten
nennen darum nur Platin DE und HeartGold DE.

## 3. Karten je Paket und Wahrheit

| Größe | Platin DE | HeartGold DE |
|---|---|---|
| Karten mit Paket / ohne Treffer | 661 / 5 | 674 / 2 |
| Karten mit Gleichstand mehrerer Pakete (erstes genommen) | 317 | 284 |
| Pakete mit Karte / ohne Karte | 65 / 9 | 89 / 17 |
| Karten je Paket min / Median / p90 / max | 0 / 3 / 18 / 146 | 0 / 3 / 11 / 120 |
| Texturen mit mindestens einer Karte / ohne Karte | 1 669 / 1 461 | 2 861 / 798 |
| Karten je Textur (mit Karte) min / Median / p90 / max | 1 / 2 / 7 / 118 | 1 / 1 / 6 / 115 |
| Polygone: waagerecht auf dem Raster | 44 359 | 67 846 |
| davon ohne Zelle im 32 × 32 | 313 | 1 110 |
| Polygone: waagerecht, nicht auf dem Raster | 55 781 | 87 496 |
| Polygone: nicht waagerecht | 225 168 | 265 781 |
| Polygone: Textur nicht im Paket der Karte | 1 774 | 1 743 |
| überdeckte Zellen im Raster / außerhalb 32 × 32 (je Polygon gezählt) | 304 977 / 3 160 | 461 701 / 29 465 |
| Texturen mit überdeckten Zellen | 580 | 825 |
| **Texturen mit Wahrheit** (Mehrheit ≥ 0,6) / „keine“ | **555** / 25 | **780** / 45 |
| Wahrheit `kind 0` (% der Texturen mit Wahrheit) | 71,9 % (399) | 74,1 % (578) |
| Wahrheit `kind ≠ 0` | 156 | 202 |
| Mehrheitsanteil der Wahrheit min / Median / p90 / max | 0,345 / 1 / 1 / 1 | 0,333 / 1 / 1 / 1 |
| verschiedene `kind`-Werte der Wahrheit | 27 | 23 |
| Wahrheit Kollision `0x80` (Texturen) | 118 | 178 |

**Gegenprobe Ausrichtung** (Texturen mit Wahrheit / mittlerer Mehrheitsanteil): wie gelesen 555 / 0,943 und
780 / 0,937; x gespiegelt 512 / 0,870 und 640 / 0,795; z gespiegelt 506 / 0,867 und 659 / 0,810; Achsen vertauscht
498 / 0,859 und 658 / 0,799. Die Ausrichtung „wie gelesen“ ist deutlich die beste: die Zellzuordnung stimmt.

Unter einer Textur ist das Gelände sehr eindeutig (Median des Mehrheitsanteils 1): die Wahrheit taugt als Maßstab. Sie
deckt aber nur 555 von 3 130 und 780 von 3 659 Texturen.

## 4. K1 (Mehrheit) je Schwelle

| `UNIQUE_SHARE` | entschieden | davon `k* ≠ 0` | unklar | Übereinstimmung | streng | Trefferquote | Wahrheit `≠ 0` richtig | Wahrheit `0` richtig |
|---|---|---|---|---|---|---|---|---|
| Platin 0,6 | 1 619 | 3 | 1 511 | **72,1 %** (400) | 70,5 % | 74,1 % | 1 von 156 | 399 von 399 |
| Platin 0,7 | 1 494 | 2 | 1 636 | 72,1 % (400) | 67,0 % | 80,3 % | 1 von 156 | 399 von 399 |
| Platin 0,8 | 1 312 | 1 | 1 818 | 72,1 % (400) | 58,2 % | 84,3 % | 1 von 156 | 399 von 399 |
| Platin 0,9 | 1 073 | 0 | 2 057 | 71,9 % (399) | 45,2 % | 88,7 % | 0 von 156 | 399 von 399 |
| HeartGold 0,6 | 2 420 | 108 | 1 239 | **72,1 %** (562) | 56,7 % | 74,7 % | 4 von 202 | 558 von 578 |
| HeartGold 0,7 | 2 239 | 71 | 1 420 | 72,7 % (567) | 51,0 % | 75,7 % | 2 von 202 | 565 von 578 |
| HeartGold 0,8 | 1 836 | 15 | 1 823 | 74,2 % (579) | 38,1 % | 86,1 % | 1 von 202 | 578 von 578 |
| HeartGold 0,9 | 1 627 | 0 | 2 032 | 74,1 % (578) | 30,4 % | 92,9 % | 0 von 202 | 578 von 578 |

Anteil `votes(k*) / Σ votes` min / Median / p90 / max: Platin 0,452 / 0,968 / 0,999 / 1 (Texturen mit Karte),
0,452 / 0,902 / 0,997 / 1 (Texturen mit Wahrheit); HeartGold 0,327 / 0,966 / 0,999 / 1 und 0,327 / 0,776 / 0,998 / 1.
`k*` der entschiedenen bei 0,6: Platin `0x00` 1 616, `0x15` 3; HeartGold `0x00` 2 312, `0x06` 73, `0x15` 35.

## 5. K2 (Abhebung) je Schwelle

| `UNIQUE_SHARE` | entschieden | davon `k* ≠ 0` | unklar | Übereinstimmung | streng | Trefferquote | Wahrheit `≠ 0` richtig | Wahrheit `0` richtig |
|---|---|---|---|---|---|---|---|---|
| Platin 0,6 | 1 644 | 200 | 1 486 | **74,1 %** (411) | 72,4 % | 73,8 % | 45 von 156 | 366 von 399 |
| Platin 0,7 | 1 585 | 200 | 1 545 | 74,1 % (411) | 71,2 % | 75,2 % | 45 von 156 | 366 von 399 |
| Platin 0,8 | 1 437 | 200 | 1 693 | 74,1 % (411) | 63,2 % | 76,3 % | 45 von 156 | 366 von 399 |
| Platin 0,9 | 1 248 | 200 | 1 882 | 74,1 % (411) | 52,8 % | 76,5 % | 45 von 156 | 366 von 399 |
| HeartGold 0,6 | 2 498 | 250 | 1 161 | **69,4 %** (541) | 56,8 % | 70,5 % | 27 von 202 | 514 von 578 |
| HeartGold 0,7 | 2 386 | 250 | 1 273 | 69,4 % (541) | 53,3 % | 70,0 % | 27 von 202 | 514 von 578 |
| HeartGold 0,8 | 2 061 | 250 | 1 598 | 69,4 % (541) | 41,4 % | 73,6 % | 27 von 202 | 514 von 578 |
| HeartGold 0,9 | 1 876 | 250 | 1 783 | 69,4 % (541) | 33,8 % | 74,4 % | 27 von 202 | 514 von 578 |

Größter Quotient `p_T(k) / base(k)` min / Median / p90 / max: Platin 0,14 / 1,25 / 4,27 / 30, HeartGold
0,28 / 1,19 / 2,92 / 97; ohne Kandidat `k ≠ 0` mit `p ≥ 0,05`: 1 012 und 1 532 Texturen mit Karte. Die Schwelle
wirkt bei K2 nur auf den Rückfall „Kind 0“ (Übereinstimmung gleich je Schwelle). `k*` der entschiedenen bei 0,6:
Platin `0x00` 1 444, `0x15` 53, `0x10` 33, `0x02` 28, `0x0b` 19, `0xa7` 13, `0x21` 12, `0xa4` 11; HeartGold `0x00`
2 248, `0x15` 73, `0x06` 68, `0x02` 35, `0x10` 23, `0xa4` 14, `0x71` 11, `0x2d` 8.

**Ohne Regel (`None`, alles unklar):** Übereinstimmung Platin 71,9 % (399), HeartGold 74,1 % (578): die Grundlinie
„jede Textur `kind 0`“.

## 6. Kollision und HGSS-Geräusch

| Regel | entschieden mit `0x80` (Platin / HeartGold) | Kollision gleich der Wahrheit (Platin / HeartGold) |
|---|---|---|
| K1 0,6 | 745 / 620 | 44,5 % / 64,0 % |
| K1 0,7 | 630 / 503 | 53,0 % / 66,5 % |
| K1 0,8 | 469 / 286 | 59,6 % / 71,7 % |
| K1 0,9 | 312 / 172 | 65,0 % / 74,0 % |
| K2 0,6 | 615 / 572 | 55,1 % / 62,4 % |
| K2 0,9 | 305 / 239 | 65,2 % / 69,4 % |
| überall `0` | 0 / 0 | 78,7 % / 77,2 % |

Die Kollision nach der Planregel (Hälfte der gewichteten Zellen mit `k*` blockiert) trifft seltener als „überall 0“:
mit `k* = 0` zählen die blockierten Zellen der ganzen Karte (Bäume, Gebäude, Felsen tragen meist `kind 0`) mit.
HGSS-Geräusch der entschiedenen bei K1 0,6: `0x00` 2 383, `0x06` 30, `0x04` 7.

## 7. Paletten

| Fall | Platin DE | HeartGold DE |
|---|---|---|
| `<Textur>_pl` | 664 (21,2 %) | 2 781 (76,0 %) |
| `<Textur>_pl`, nur über den auf 16 Byte abgeschnittenen Namen | 8 (0,3 %) | 0 |
| gleichnamig | 1 820 (58,1 %) | 546 (14,9 %) |
| keine, eine Palette mit dem Index der Textur gibt es | 582 (18,6 %) | 324 (8,9 %) |
| keine, auch kein Index | 56 (1,8 %) | 8 (0,2 %) |
| **keine gesamt** | **20,4 %** | **9,1 %** |
| Format 7 (direkt) | 0 | 0 |

## 8. Entscheidung

Regel aus Step 4: der Kandidat mit der höheren Übereinstimmung (Gleichstand K1); bei K1 der kleinste `UNIQUE_SHARE`
mit ≥ 70 %, sonst 0,9; unter 50 % ist die Wahl `None`.

- **Je Spiel:** K1 erreicht bei 0,6 schon 72,1 % (Platin) und 72,1 % (HeartGold), also `UNIQUE_SHARE = 0,6` in beiden.
  Gegen K2 (bei 0,6): Platin 72,1 % gegen 74,1 % → K2; HeartGold 72,1 % gegen 69,4 % → K1. Die Spiele widersprechen
  sich.
- **Zusammen** (Task 5 hat eine Konstante; Platin DE und HeartGold DE gezählt, HeartGold US ist gleich HeartGold DE und
  zählte sonst doppelt): K1 0,6 trifft 400 + 562 = 962 von 555 + 780 = 1 335 Texturen (72,1 %), K2 411 + 541 = 952
  (71,3 %). Das Mittel der beiden Prozentwerte gibt dasselbe (72,1 % gegen 71,8 %). K1 ≥ 50 %. Die Grundlinie
  „alles `kind 0`“ trifft zusammen 399 + 578 = 977 von 1 335 (73,2 %), mehr als K1 0,6 (72,1 %) und K2 (71,3 %).
- **Wahl: `DEFAULT_RULE = VoteRule::K1`, `UNIQUE_SHARE = 0.6`.**
- **Palettenregel: ja.** „keine“ 20,4 % (Platin) und 9,1 % (HeartGold), beide ≥ 5 %: Task 5 ergänzt nach „`_pl`, sonst
  gleichnamig“ die „Palette mit dem Index der Textur“; das deckt 582 von 638 und 324 von 332 der Fälle ohne Palette,
  der Rest bleibt ohne Palette. Den Namen `<Textur>_pl` auf 16 Byte abschneiden, bevor verglichen wird (8 Fälle in
  Platin).

**Wie knapp die Wahl ist.** Der Abstand beträgt 10 von 1 335 Texturen. Die beiden anderen Lesarten der
Übereinstimmung: „streng“ (entschieden und richtig) wählt K2 in beiden Spielen (Platin 70,5 % gegen 72,4 %; HeartGold
erreicht K1 nie 70 %, also 0,9 mit 30,4 % gegen K2 56,8 %); „Trefferquote“ wählt K1 0,6 in beiden (74,1 % gegen
73,8 %, 74,7 % gegen 70,5 %). Gewählt ist die Lesart „unklar zählt als `kind 0`“, weil das Tileset eine unklare Textur
mit `kind 0` trägt; sie stand vor dem Lauf fest.

**Was die Wahl bedeutet.** K1 0,6 nennt fast nur `kind 0`: von den Texturen, deren Wahrheit nicht 0 ist (Gras,
Wasser, Sand und andere), trifft K1 1 von 156 (Platin) und 4 von 202 (HeartGold); K1 liegt damit auf der Grundlinie
„alles 0“ (71,9 % und 74,1 %), in HeartGold knapp darunter. K2 trifft 45 von 156 und 27 von 202 dieser Texturen und
verliert dafür 33 von 399 und 64 von 578 Texturen mit Wahrheit 0. Abgeleitete Rollen `grass` und `water` gibt es mit K1
also kaum.

**Festlegung** (was Task 5 nimmt):

- **Regel:** `DEFAULT_RULE = K1`, `UNIQUE_SHARE = 0,6`: Step 4 auf die zusammengezählten Zahlen angewandt; `None` gilt
  nur unter 50 %. K1 ist die vorsichtige Wahl: K2 gäbe 33 von 399 (Platin) und 64 von 578 (HeartGold) Texturen mit
  Wahrheit `kind 0` ein anderes `kind`.
- **Kollision:** jede abgeleitete Kachel bekommt `collision 0`, abweichend von der Planregel (`0x80` bei blockierter
  Mehrheit der gewichteten Zellen mit `k*`): die Planregel stimmt in 44,5 % / 64,0 % der Texturen mit Wahrheit, „überall
  0“ in 78,7 % / 77,2 % (Abschnitt 6), und eine blockierte Vorgabe machte gemalten Boden unbegehbar. Das HGSS-Geräusch
  bleibt wie im Plan.
- **Palette:** `<Textur>_pl` (Name auf 16 Byte abgeschnitten), sonst die gleichnamige, sonst die Palette mit dem Index
  der Textur, sonst keine.

## 9. Gegenprobe: Zuordnung bei Gleichstand

Fast die Hälfte der Karten hat mehrere gleich gute Pakete (317 von 661, 284 von 674), weil Pakete Texturnamen teilen.
Ordnet man eine Karte allen gleich guten Paketen zu (Platin 4 522, HeartGold 2 557 Zuordnungen), ändern sich die
Zahlen stark:

| Größe | Platin DE | HeartGold DE |
|---|---|---|
| Texturen mit Karte / mit Wahrheit | 2 624 / 1 115 | 3 506 / 1 142 |
| Wahrheit `kind 0` | 59,4 % | 71,4 % |
| K1 Übereinstimmung 0,6 / 0,7 / 0,8 / 0,9 | 59,4 / 59,4 / 59,4 / 59,4 % | 75,0 / 70,8 / 71,4 / 71,4 % |
| K2 Übereinstimmung (jede Schwelle) | 64,3 % | 71,0 % |
| Wahl je Spiel nach Step 4 | K2, 0,6 | K1, 0,6 |

Zusammen (662 + 856 = 1 518 gegen 717 + 811 = 1 528 von 2 257) wählte diese Zuordnung K2: K1 erreicht zusammen bei
keiner Schwelle 70 %, also 0,9 mit 65,4 % gegen K2 67,7 %. Die Wahl in Abschnitt 8 gilt für die Zuordnung des Plans
(bester Treffer, Gleichstand: erstes Paket).

## 10. Grenzen

- **Karten-weite Zählung.** K1 und K2 wissen nur, welche Texturen eine Karte bindet, nicht, wo sie liegen: jede Karte
  verteilt ihre 1 024 Zellen gleich auf alle ihre Texturen. Weil auf den meisten Karten `kind 0` überwiegt, gewinnt
  `kind 0` die Mehrheit fast immer; das polygon-genaue Zählen (wie hier die Wahrheit) braucht den Display-List-Codec
  von 6.1.
- **Polygone außerhalb des Rasters.** Nur waagerechte Polygone mit allen Ecken auf dem 16er-Raster zählen zur Wahrheit:
  44 359 von 100 140 waagerechten Polygonen (Platin), 67 846 von 155 342 (HeartGold). Polygone auf halben Kacheln,
  freie Polygone, Schrägen und Wände fehlen; Zellen außerhalb 32 × 32 (3 160 und 29 465) fallen weg.
- **Wenig Wahrheit.** Nur 555 von 3 130 (18 %) und 780 von 3 659 (21 %) Texturen haben eine Wahrheit, davon 156 und 202
  mit `kind ≠ 0`; die Entscheidung steht auf diesen Texturen. 1 461 und 798 Texturen haben keine Karte und werden immer
  unklar.
- **Zuordnung Karte → Paket über Texturnamen**, nicht wie der Kernel über Header, Gebietsdaten und Matrizen; bei
  Gleichstand das erste Paket (Abschnitt 9); eine Karte ohne jeden Namenstreffer gehört zu keinem Paket (anders als in
  Gutachten C; 5 Karten in Platin, 2 in HeartGold). Die Zahlen des Kernels können abweichen; Task 5 trägt sie unten
  nach.
- **Kollision** nach der Planregel trifft seltener als „überall 0“ (Abschnitt 6); die Wahl der Regel betrifft nur
  `kind`.
- **Paletten:** gezählt ist nur, ob eine Palette des Namens oder Index da ist, nicht, ob sie die richtige ist (keine
  Pixel verglichen).

## 11. Vergleichszahlen für den gated Test von Task 5

Mit K1 0,6 und der Zuordnung über Texturnamen: Platin DE 74 Pakete, 3 130 Kacheln, 1 619 entschieden (`vanilla`),
1 511 unklar, 3 mit `k* ≠ 0`; HeartGold DE und US 106 Pakete, 3 659 Kacheln, 2 420 entschieden, 1 239 unklar, 108 mit
`k* ≠ 0`. Verteilung der Anteile `votes(k*) / Σ votes` (Texturen mit Karte) min / Median / p90 / max: Platin
0,452 / 0,968 / 0,999 / 1, HeartGold 0,327 / 0,966 / 0,999 / 1.

## 12. Gegenprobe im Kernel

Nachgetragen von Task 5 (`vanilla_tilesets_have_one_ground_tile_per_texture`, `vanilla_vote_shares_match_task0`,
Release-Build, je Testkopie, 2026-11-06). Der Kernel ordnet die Karten über Header, Gebietsdaten und Matrizen zu
(eine Karte kann zu mehreren Paketen gehören), nicht über Texturnamen wie Abschnitt 1; Regel K1, `UNIQUE_SHARE` 0,6,
Kollision überall 0.

| Größe | Platin DE | HeartGold DE | HeartGold US |
|---|---|---|---|
| Pakete | 74 | 106 | 106 |
| Kacheln (= Texturen) | 3 130 | 3 659 | 3 659 |
| entschieden (`vanilla`) | 1 765 | 2 612 | 2 612 |
| unklar | 1 365 | 1 047 | 1 047 |
| Pakete mit Karte | 71 | 98 | 98 |
| Texturen mit Karte / ohne Karte | 1 837 / 1 293 | 3 238 / 421 | 3 238 / 421 |
| Anteil `votes(k*) / Σ votes` min / Median / p90 / max | 0,392 / 0,951 / 0,998 / 1 | 0,309 / 0,918 / 0,998 / 1 | 0,309 / 0,918 / 0,998 / 1 |
| Ableitung aller Pakete (`Deriver::new` und `derive`), ms | 6 470 | 7 751 | 5 988 |

Gegen Abschnitt 11: mehr Texturen haben eine Karte (1 837 statt 1 669, 3 238 statt 2 861), darum mehr entschiedene
(1 765 statt 1 619, 2 612 statt 2 420) und eine etwas breitere Verteilung der Anteile (Median 0,951 statt 0,968 und
0,918 statt 0,966). Die Kontrollzahlen 74 / 3 130 und 106 / 3 659 treffen.
