# Probe: GDB-Stub von melonDS 1.1 für 3d.9 „Spielmodus live“ (Weg 3)

Stand 2026-10-31. Messskript: `probe-gdb-stub.py` (selber Ordner, nur Standardbibliothek).
Gemessen auf diesem Mac mit `/Applications/melonDS.app` (Info.plist `CFBundleShortVersionString` = **1.1**),
Platin DE (CPUD) und HeartGold DE (IPKD), je mit Kopie von `_saves/{CPUD,IPKD}-zimmer.sav`, am **Titelbildschirm**
(siehe „Nicht getestet“). Quelltext gelesen: `melonDS-emu/melonDS` Tag `1.1`, `src/debug/GdbStub.cpp`, `GdbProto.cpp`,
`GdbCmds.cpp`, `src/ARM.cpp`, `src/NDS.cpp`, `src/frontend/qt_sdl/EmuInstance.cpp`.

## Ergebnis in einem Satz

Der Stub liest und schreibt RAM **bei laufender Emulation ohne Halt**; ein Halt ist nur beim Verbinden nötig
(einmal `c` senden). Weg 3 trägt 3d.9. **Empfehlung (recommend): 3d.9 über Weg 3 bauen**, Weg 2 bleibt Rückfall.

## Version, Konfiguration, Schalter

- Konfigurationsdatei: **`~/Library/Preferences/melonDS/melonDS.toml`** (nicht `Application Support`).
- Schlüssel (pro Instanz, hier `Instance0`):

```toml
[Instance0.Gdb]
Enabled = true          # einziger nötiger Schalter
[Instance0.Gdb.ARM9]
BreakOnStartup = false  # muss false bleiben (Standard)
Port = 3333
[Instance0.Gdb.ARM7]
BreakOnStartup = false
Port = 3334             # wird mit geöffnet, für 3d.9 unnötig
[JIT]
Enable = false          # Pflicht: mit JIT gibt es keinen Stub (ARM-Konstruktor: SetGdbArgs(jit ? nullopt : gdb))
```

- GUI: Einstellungen „Emu settings“ → Reiter **„Devtools“** → **„Enable GDB stub“** (Port, „Break on startup“). Der Wert
  wird beim Start eines Spiels gelesen (`EmuInstance::updateConsole`), also nach dem Umschalten ROM neu starten.
- **Datei selbst setzen geht, aber nur bei beendetem melonDS:** melonDS schreibt beim Beenden seinen Speicherstand der
  Konfiguration zurück. Gemessen: `Enabled` während des Laufs auf `false` geändert → nach dem Beenden wieder `true`.
- Mit `Enabled = false` lauscht nichts auf 3333 (gemessen mit `lsof`).

## Messungen

Je ein Lauf pro Spiel (`measure`), Bildzähler `u32 @ 0x023FFC3C` (NitroSDK `HW_VBLANK_COUNT_BUF`, +60/s, gefunden mit
`findcounter`). Zeiten sind Median aus 20 Wiederholungen, Wanduhr auf Client-Seite, 127.0.0.1.

| Messung | Platin DE | HeartGold DE |
|---|---|---|
| TCP-Verbindung + Handshake | 1,4 ms | 1,4 ms |
| Emulation nach dem Verbinden | **hält an** (Zähler 0,5 s unverändert) | hält an |
| `qSupported` | `PacketSize=47F;qXfer:features:read+;swbreak-;hwbreak+;QStartNoAckMode+` | gleich |
| `?` nach Verbinden / Stopp nach `\x03` | `S02` / `S05` | gleich |
| nach `c`: `m` bei laufender Emulation | **ja**, 8,7 ms (max 16,8) | ja, 10,4 ms (max 20,7) |
| Emulation läuft dabei weiter | ja, +73 Bilder in 20 Lesungen, streng steigend | ja, +75 |
| Bildrate mit verbundenem Client | 60,0/s | 60,1/s |
| Zeiger + 288 Vars + 364 B Flags, laufend (4 Pakete) | 51 ms | 57 ms |
| Zeiger (Spielstand-Block) | `0x0227E354` | `0x0227C2A8` |
| Halt-Lesen-Weiter (Zeiger, Vars, Flags) | 10,4 ms (max 20,9) | 13,0 ms (max 16,3) |
| davon Warten auf die Stopp-Antwort | 9,95 ms | 12,5 ms |
| davon reine Haltdauer (Stopp bis `c`) | ≈ 0,5 ms | 0,43 ms (max 1,25) |
| Emulation zwischen zwei Zyklen (100 ms Pause) | +7 Bilder | +6…7 Bilder |
| No-Ack-Modus (`QStartNoAckMode`) | Zyklus 10,7 ms, Lesen 7,5 ms | 13,0 / 11,1 ms |
| `M` Variable 287 = `0xBEEF`, laufend, + Rücklesen | 36,5 ms; gelesen `0xBEEF`, nach 1 s noch `0xBEEF` | 33,3 ms; gleich |
| `M` 2 Byte, laufend (20×) | – | 13,5 ms (max 20,8) |
| `M` Flag-Bit (letztes Flag-Byte, Bit 7) | landet, nach 1 s noch gesetzt | gleich |
| Werte wiederhergestellt | ja | ja |
| `D` (Detach) | `OK`, Emulation läuft weiter | gleich |
| Ohne Client ab Start | läuft (Zähler 1171 beim ersten Lesen ≈ 19,5 s Laufzeit) | läuft |
| Neu verbinden nach `D` / nach hartem Schließen / nach Schließen im Halt | geht jedes Mal; Emulation lief in der Zwischenzeit mit 60/s weiter (371 Bilder in 6,2 s usw.) | – |
| zweiter Client gleichzeitig | TCP nimmt an, aber **keine Antwort**; erster Client arbeitet weiter | – |
| Client ohne anfängliches `+` | Stub trennt nach 1 s (`b''`) | – |
| CPU-Last melonDS am Titel, Stub an / aus | – | 10,3 % / 12,8 % (kein messbarer Aufpreis) |

Deutung: Der Stub wird **einmal pro Bild** abgefragt (`NDS::RunFrame` → `ARM9.CheckGdbIncoming()` am Bildanfang) und
bearbeitet dann **ein** Paket. Lesen und Schreiben bei laufender Emulation kosten also keinen Halt, aber bis zu einem
Bild (≈ 8–17 ms) Latenz pro Paket. Ein Halt (`\x03`) greift ebenfalls erst am nächsten Bildanfang; danach werden
beliebig viele Pakete ohne Wartezeit bedient, und `c` lässt weiterlaufen. Die Pause für den Spieler ist ≈ 0,5 ms, weit
unter einem Bild; ein Ruckeln ist nicht zu erwarten (Bildrate blieb bei 60/s, sichtbar geprüft wurde es nicht).

## Eigenheiten von 1.1 (Quelltext + Messung)

1. **Verbinden hält an.** Nach `accept()` bleibt der Stub in einer Schleife (`Enter`, `stay = true`), bis `c`, `vCont;c`
   oder Trennen kommt. Der Client muss sofort `c` senden. Ohne Client und nach jeder Trennung läuft die Emulation.
2. **Handshake:** innerhalb 1 s nach dem Verbinden ein `+` senden, sonst trennt der Stub.
3. **Acks:** Der Stub wartet nach jeder Antwort bis zu 2 s blockierend (im Emulations-Thread!) auf das `+` des Clients.
   Sofort quittieren oder `QStartNoAckMode` (geht, bringt lokal kaum etwas). `NoAck` wird beim Trennen nicht
   zurückgesetzt: ein neuer Client muss erneut `QStartNoAckMode` senden oder damit rechnen.
4. **Paketgrößen (`PacketSize=47F` = 1151):** `m` bis 575 Byte; **576 Byte → gar keine Antwort**, ab 577 `E02`.
   **Ein `M`-Paket über ≈ 1150 Zeichen legt den Stub bis zum Neustart von melonDS lahm** (Empfangspuffer läuft voll,
   `RecvBufferFilled` wird nie zurückgesetzt; danach antwortet er keinem Client mehr). Gemessen: `M` mit 575 Byte →
   Hänger; `M` 512 und 560 Byte → `OK`. Die App muss in Stücken ≤ 512 Byte lesen und schreiben.
5. **Nur ein Client.** Ein zweiter wird erst bedient, wenn der erste trennt. Ein offener eigener Debugger des Nutzers
   blockiert die App (und umgekehrt).
6. `vCont?` → `vCont;c;s;t`; `?` braucht man nicht. Stopp-Antworten sind `S02`/`S05`, keine `T`-Pakete.
7. **Lauscht auf allen Schnittstellen** (`INADDR_ANY`, `lsof`: `*:3333`, `*:3334`): aus dem LAN erreichbar, die
   macOS-Firewall kann fragen. Hier keine Abfrage beobachtet.
8. **JIT aus ist Pflicht** (siehe oben); `BreakOnStartup = true` würde den Emulations-Thread blockieren, bis ein Client
   kommt (Quelltext `Poll(wait)`; nicht gemessen).
9. melonDS loggt **jedes GDB-Paket auf stdout** (Debug-Ausgabe auch im Release); während eines Halts im Takt der
   Warteschleife. Unser mitgeschnittenes Log wuchs in 5 min auf 250 MB. Startet die App melonDS selbst, stdout verwerfen.
10. Lesen/Schreiben läuft über den ARM9-Bus inkl. ITCM/DTCM; 1.1 emuliert keinen Daten-Cache, Schreibzugriffe sind für das
    Spiel sofort sichtbar.

### Nachtrag 3d.9 (Befund B2, 2026-11-02, Quelltext `GdbProto.cpp` + Mitschnitt)

11. **Ein Paket je Ankunft.** `MsgRecv` liest alles Angekommene in `RecvBuffer`, behandelt aber nur das erste Paket;
    der Rest wird erst behandelt, wenn wieder Bytes am Socket ankommen. `c` (ohne Antwort) und das gleich danach
    geschickte `\x03` oder `D` kommen oft zusammen an; das zweite bleibt dann unbeantwortet. Ein einzelnes `+` danach
    (wird übersprungen) löst es aus.
12. **Der Puffer überlebt das Trennen** (`RecvBufferFilled` wird nie zurückgesetzt): der nächste Client bekommt zuerst
    die Antworten auf das, was der vorige liegen ließ (`S02`, `OK` + `W00`).
13. **Ein fremdes Zeichen am Pufferanfang** („unknown character“) schließt die Verbindung, ohne das Zeichen zu
    verwerfen: danach schließt der Stub jeden Client sofort, bis melonDS neu startet. Im Ack-Modus entsteht so ein
    Zeichen, wenn eine Antwort (`WaitAckBlocking`) Bytes des nächsten Pakets als „Ack“ verschluckt.
14. **`qSupported` braucht den Doppelpunkt** (`qSupported:…`); ohne ihn antwortet der Stub leer („unknown subcommand“).
15. Nach `D` antwortet der Stub `OK`, behält die Verbindung und schickt beim nächsten Blick `W00`.

## Urteil (recommend)

**Weg 3 trägt 3d.9.** Kriterien:
- Flags und Variablen lesen und schreiben: **ja**, beide Spiele, mit den Adressen aus Gutachten B (Zeiger + Kopf).
- ≤ 100 ms sichtbare Pause: **ja.** Lesen und Schreiben bei laufender Emulation pausieren gar nicht (51–57 ms Latenz
  für den ganzen Zustand, aber ohne Halt). Halt-Lesen-Weiter pausiert ≈ 0,5 ms.
- Keine GUI-Schritte außer einem Schalter: **ja**, „Enable GDB stub“ (oder die App setzt `Gdb.Enabled` bei beendetem
  melonDS). JIT muss aus sein; das ist ein zweiter Haken, falls der Nutzer JIT eingeschaltet hat.

Empfohlene Arbeitsweise der App:
- Verbinden → `+` → `qSupported` → `c`; dann laufend lesen (Abfrage z. B. alle 250 ms, 4 Pakete).
- **Für einen konsistenten Schnappschuss** (alle Flags+Vars aus demselben Bild) und für Schreibvorgänge: `\x03` → lesen/
  schreiben → `c` (≈ 10–13 ms Wartezeit, 0,5 ms Pause). Bei laufendem Lesen kann ein Mehr-Paket-Block über 3–4 Bilder
  verteilt sein.
- Nach dem Schreiben nach einigen Bildern zurücklesen; ein Schreiben kann theoretisch verloren gehen, wenn das Spiel das
  Byte gerade in einem Register hält (Halt am Bildanfang, nicht an einem sicheren Punkt im Spielcode).
- Spielcode-Prüfung über den Zeiger: vor dem Laden des Spielstands ungültig; am Titel ist er hier bereits gültig.

**Weg 2** (libretro-Kern im Prozess) ist nur Rückfall, falls melonDS-Versionen den Stub ändern oder der Nutzer kein
melonDS nutzt; für die Lese-/Schreibfrage ist er nicht nötig.

## Was die App dem Nutzer sagen muss / selbst tun kann

Sagen: „melonDS ≥ 1.1 → Emu settings → Devtools → *Enable GDB stub* einschalten (Port 3333), *Break on startup* aus,
JIT aus; danach das Spiel neu starten.“ Und: Nur ein Programm kann sich gleichzeitig verbinden; der Port ist im LAN offen.

Selbst tun:
- Version aus `/Applications/melonDS.app/Contents/Info.plist` lesen; `~/Library/Preferences/melonDS/melonDS.toml` lesen
  und den Zustand der drei Schlüssel anzeigen.
- Schlüssel selbst setzen **nur wenn melonDS nicht läuft** (sonst überschreibt melonDS beim Beenden); vorher sichern.
- melonDS mit ROM starten (`/Applications/melonDS.app/Contents/MacOS/melonDS <rom>`), stdout verwerfen.
- Verbinden, sofort `c`, lesen/schreiben in Stücken ≤ 512 Byte, sauber mit `D` trennen; Wiederverbinden funktioniert.
- Erkennen, dass der Stub hängt (Verbindung geht, keine Antwort in 1 s) → Hinweis „melonDS neu starten“.

## Nicht getestet

- **Overworld und sichtbarer Effekt:** Ohne Bedienungshilfen-Berechtigung für `osascript` konnte ich keine Tasten an
  melonDS senden und keinen Bildschirm aufnehmen; alle Messungen am Titelbildschirm (Spielstand ist dort schon geladen:
  Zeiger gültig, 11 bzw. 4 Variablen ≠ 0, 114 bzw. 148 Flags gesetzt). Dass ein Flag-Schreiben ein NPC sichtbar macht,
  ist nur über Gutachten B (libretro-Kern, dieselben Adressen) belegt. Nächster Schritt: einmal mit der Hand ins Zimmer,
  `probe-gdb-stub.py --game pt poke --flag N --value 0`, Kartenwechsel.
- Ob das Spiel ein geschriebenes Flag beim Kartenwechsel oder Speichern zurückschreibt (Gutachten B sagt: nein).
- HeartGold US, DSi-Modus, Versionen ≠ 1.1, Windows/Linux, `BreakOnStartup = true`, der ARM7-Port.
- Bildrate unter Last (Kampf, 3D-Overworld) mit Stub; CPU-Vergleich nur am Titel und nicht streng.
- Ob ein ROM-Reset (ohne Neustart von melonDS) den gelähmten Stub wiederbelebt (Quelltext: eher nein).
- Spielcode im RAM-Header (`0x023FFE0C`) zur Prüfung, welches Spiel läuft: naheliegend, nicht gemessen.

## Aufräumen

Konfiguration aus der Sicherung wiederhergestellt (byte-gleich geprüft), Testkopien `Roms/_testkopien/3d-9-probe/`
gelöscht, melonDS beendet. Im Repo nur `probe-gdb-stub.py` und diese Datei.
