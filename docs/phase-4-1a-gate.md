# Phase 4.1a: Gate prüfen

Stand: 2026-10-27. Branch `phase-4-1a-basis-rom-patch` (Worktree `.claude/worktrees/phase-4-1a`, Basis `d25f61d`), nicht nach main gemergt. Gate vom Agenten bestanden: (1) Automatik (der volle Lauf von `scripts/gate.sh` gegen alle drei Projekte, grün auf `bd383f8` in 4 091 s), (2) Szenario kopflos und (3) Browser-Teil mit MCP auf dem Stand `24bea52` (16 Commits `bf2921f` bis `24bea52`); die drei Befunde der Gate-Runde (B1, B2, U1) sind behoben, je mit eigenem Commit. Unabhängige Abnahme am selben Tag bestanden, vier Befunde (A1 bis A4) behoben in `5a12cd7`, `693c2d4`, `5dce50e` und `c800ce7` (Abschnitt „Abnahme (2026-10-27)“). Beim Nutzer offen: Rom Patcher JS mit der eigenen Basis-ROM (Hand), xdelta UI unter Windows (Hand, kein Windows) und der Blick auf Technik › Veröffentlichen in Hell, Dunkel und Kompakt; Abschnitte „Hand“ und „Blick“.

Phase 4.1a bringt Basis-ROM und Patch: Ein DSPRE-Ordner bekommt über „Basis-ROM zuordnen…“ (App, MCP, `g4k base assign`) die Basismarke v2 mit `origin: "dspre"` und baut danach byte-gleich zu seiner `.nds`. Das Studio findet die Basis-ROM über die SHA-1 (App-Einstellung `base-roms.json`, kopflos `--base-rom` und `G4_BASE_ROM_DIR`), liest sie mit dem Kernel-Modul `base` und schreibt in Technik › Veröffentlichen › „Patch erstellen…“ einen selbstgeprüften xdelta-Patch `build/<Projekt>-<Spielcode>.xdelta` mit Begleitdatei `.xdelta.txt` (xdelta3 3.0.12 im Crate `g4-xdelta`, ohne Zweitkompression, mit Adler32, Basis-SHA-1 im App-Header). `cargo deny check licenses advisories` ist ein Schritt von `scripts/gate.sh`. Roadmap: [docs/ROADMAP.md](ROADMAP.md), Abschnitt „Phase 4“, Punkt „4.1a Basis-ROM und Patch“. Entscheidungen: [entscheidungen.md](research/2026-10-27-phase-4/entscheidungen.md), Abschnitt „Basis-ROM und Patch (Gutachten B, 4.1)“. Plan: [2026-10-27-phase-4-1a-basis-rom-patch.md](superpowers/plans/2026-10-27-phase-4-1a-basis-rom-patch.md).

Gate-Text der Roadmap (Punkt 4.1a):

> Gate: je Projekt `g4k base assign` auf eine frische Kopie des DSPRE-Ordners, dann `g4k build`, SHA-1 gleich der `.nds`; fremder Spielcode wird abgelehnt, unbekannte SHA-1 warnt; Patch nach einer Kartenänderung ≤ 1 KiB, nach Karte, Script und Text ≤ 16 KiB, Erzeugen ≤ 1 s, mit `xdelta3 -d` angewandt gleich (gated, wenn `xdelta3` im PATH ist); fehlende Basis-ROM zeigt den Grund (Test-Umgebung); `cargo deny check licenses advisories` grün; Rom Patcher JS als Hand-Punkt.

Wer was prüft: Die Spalte „Wer“ sagt **Agent** (die Test-Umgebung aus [docs/agent-gate.md](agent-gate.md): Browser-Fenster mit `studio-bridge`, `call.sh`, `mcp.sh`, dazu `g4k` und die gated Tests), **Agent + Blick** (der Agent prüft und beschreibt Ort und Zahlen, der Nutzer bestätigt kurz am Bild) oder **Hand** (der Nutzer). Befehle: [docs/agent-gate.md](agent-gate.md), Abschnitt „Phase 4.1a“.

Namen in der Oberfläche (Rollen und `data-*` für die Prüfung):

- **Technik › Veröffentlichen:** Listeneintrag „Veröffentlichen“ an zweiter Stelle der Liste „Technik“ (Symbol `publish`), Untertitel „Keine Basis-ROM zugeordnet“, „Basis-ROM fehlt“, „Basismarke nicht lesbar“, „Bereit für einen Patch“, „Patch erstellt“; die Fläche `[data-technik=veroeffentlichen]`.
- **Karte „Basis-ROM“** `[data-base-state]` mit `unassigned`, `missing`, `found`, `unreadable` (beim Lesen `loading`); Plakette „Nicht zugeordnet“, „Fehlt“, „Gefunden“, „Nicht lesbar“; Tasten „Zuordnen…“, „Andere Datei wählen…“, „Basis-ROM zuordnen…“, „Basis-ROM suchen…“; Dateidialog „Basis-ROM wählen“.
- **Karte „Patch“** `[data-patch-card]`: Taste „Patch erstellen…“, gesperrt mit dem Grund als Notiz daneben; Ergebnis „Patch erstellt: <Größe>“ mit Pfad und „Selbstprüfung bestanden: …“.
- **Dialoge:** „Basis-ROM zuordnen?“ (Spieltitel, Spielcode, SHA-1, „Bekannte Basis-ROM: …“, Warnungen, Plakette „Nicht rückgängig“, Tasten „Abbrechen“ und „Zuordnen“; bei falschem Spiel nur „Schließen“), „Patch erstellen?“ (Tasten „Abbrechen“ und „Patch erstellen“); über MCP der Dialog „Agent möchte etwas Unumkehrbares tun“ (`[data-agent-question]`) mit „Basis-ROM zuordnen“ bzw. „Patch erstellen“ und der Taste „Zulassen“.

## Vorbereitung

So durchgeführt am 2026-10-27:

1. **Frische Kopien** unter `Roms/_testkopien/` per `cp -cR` (Ordner) und `cp -c` (`.nds`) aus den Originalen, die dabei nur gelesen wurden; Ordner `4-1a-<zweck>_DSPRE_contents` und `.nds` `4-1a-<zweck>.nds` direkt nebeneinander, damit der Vorschlag greift:
   - `4-1a-pt` (Platin DE), `4-1a-hgde` (HeartGold DE), `4-1a-hgus` (HeartGold US) für (2);
   - `4-1a-web` (Platin DE) für den Browser-Teil, `4-1a-mcp` (Platin DE) für MCP, `4-1a-ui` (Platin DE, nur für die Blick-Bilder des Vorschlags);
   - `4-1a-pt-fremd.nds`: Kopie von `4-1a-pt.nds` mit dem letzten Byte auf 0xFF gekippt (unbekannte SHA-1).

   Alle Kopien sind nach dem Gate gelöscht.
2. **Werkzeuge:** `cargo build --release -p g4-kernel --bin g4k` für (2); die Brücke baut `studio-web.sh` selbst. Aus einem Worktree `G4_ROMS_DIR` absolut setzen und `tools/` als Symlink auf den Haupt-Checkout legen (apicula, rotom; den Symlink nie einchecken). `xdelta3` 3.2.2 im PATH für die Gegenprobe.
3. **Test-Umgebung:** `G4_ROMS_DIR=<Roms> G4_VITE_PORT=1500 G4_BRIDGE_PORT=1501 bash scripts/agent/studio-web.sh`; Browser-Fenster der Desktop-App 1440 × 900, Hell, Komfort.
4. **Ausgaben** unter `target/agent/gate-2026-10-27/4-1a/` im Worktree: `kopflos.log` und `kopflos/` (Patches, Begleitdateien, JSON-Berichte von (2)), `gated-base.log` (1), `gate-sh.log` (`scripts/gate.sh`), die 13 Blick-Bilder `blick-veroeffentlichen-*.png`. Das Skript des kopflosen Teils und das kleine Node-Skript für die Bilder (Chrome headless über CDP) lagen im Scratchpad der Sitzung, nicht im Repo. Vor `git worktree remove` den Ordner `target/agent/gate-2026-10-27/` sichern: er liegt im `target/` des Worktrees und geht sonst mit ihm verloren.

## (1) Automatik

- [x] 1. Gated Tests in `crates/g4-kernel/tests/base.rs` auf Platin DE, HeartGold DE und HeartGold US (Originale, nur gelesen; die Basis-ROM ist die Nachbar-`.nds` des Originals über `suggest_base`, ebenfalls nur gelesen; schreibende Schritte auf `vanilla_copy()`), `xdelta3` 3.2.2 im PATH, je 4/4 grün:
   - `base_assign_on_the_vanilla_folder_has_no_warning_and_rebuilds_the_nds`: Zuordnen ohne Warnung, Rebuild-SHA-1 gleich der `.nds`;
   - `base_rom_members_equal_the_vanilla_folders`: die Archive aus der ROM gleich den Ordnern;
   - `base_rom_header_table_equals_the_vanilla_project`: die Header-Tabelle aus dem ARM9 der ROM gleich der des Projekts;
   - `patch_after_map_and_after_map_script_text_is_small_and_applies`: Patch nach einer Kartenänderung ≤ 1 KiB, nach Karte, Script und Text ≤ 16 KiB, Erzeugen ≤ 1 s, angewandt mit dem eigenen Decoder und mit `xdelta3 -d` gleich; dazu die Stresszeile Textarchiv 0 ohne Grenze.
- [x] 2. `cargo deny (licenses, advisories)` als Schritt von `scripts/gate.sh` (nach clippy); `deny.toml` mit sieben Ausnahmen je RUSTSEC-ID, jede mit Grund. Grün im Lauf von `scripts/gate.sh` auf `85cb37f` („advisories ok, licenses ok“, nach `cargo fmt --check` und `cargo clippy -D warnings`; der Lauf wurde danach für die letzte Fix-Runde angehalten); im vollen Lauf auf `bd383f8` grün in 4 s.
- [x] 3. `scripts/gate.sh` grün gegen alle drei Projekte: grün auf `bd383f8` in 4 091 s (Log `target/agent/gate-2026-10-27/4-1a/gate-sh.log`). Lauf 2 auf demselben Stand war rot, nur im gated `tests/build.rs` von HeartGold US (3 Tests: die Kopie im Temp-Ordner hatte `config.yaml` und `header.yaml` verloren, daher `NotAProject` und „dsrom build failed“); einzeln wiederholt 3/3 grün in 103 s, Lauf 3 grün. Der Fehler ließ sich nicht nachstellen; er berührt keinen Code von 4.1a (Log `gate-sh-lauf2-hgus-rot.log`).

Einzeln, je Projekt, aus der Wurzel des Worktrees (so gelaufen am 2026-10-27 auf `24bea52`, Log `gated-base.log`):

```sh
export PATH="$HOME/.cargo/bin:/opt/homebrew/opt/rustup/bin:$PATH"
G4_TEST_PROJECT="<Roms>/Pokemon - Platin-Edition (Germany)_DSPRE_contents" \
  cargo test -p g4-kernel --all-features --test base -- --nocapture --test-threads 1
```

Ebenso mit `"…Goldene Edition HeartGold (Germany)_DSPRE_contents"` und `"…HeartGold Version (USA)_DSPRE_contents"`. Auf stderr stehen die Zeilen `rebuild <Spielcode> sha1=… equal`, `<n> archives equal`, `<n> headers equal`, `patch-sizes <Spielcode> map=… all=… encode_ms=…/… verify_ms=…/…`, `patch-stress <Spielcode> archive0=… encode_ms=…` und `xdelta3 cross-check: map=true all=true stress=true` (ohne `xdelta3` im PATH `xdelta3 cross-check skipped: …`, der Test bleibt grün).

| Spiel | Tests | Rebuild-SHA-1 gleich | Archive gleich | Header gleich | Patch Karte | Patch Karte + Script + Text | Erzeugen ms (Karte / alle) | Selbstprüfung ms (Karte / alle) | `xdelta3 -d` gleich | Stress Textarchiv 0 | Dauer |
|---|---|---|---|---|---|---|---|---|---|---|---|
| Platin DE | 4/4 | ja (`d40f19ac…`) | 22 | 593 | 551 B | 8 227 B (Header 0, Script 401, Textarchiv 23 mit 101 Nachrichten) | 106 / 189 | 132 / 132 | ja | 19 653 B, 183 ms | 132 s |
| HeartGold DE | 4/4 | ja (`13b6853a…`) | 23 | 540 | 552 B | 3 043 B (Header 2, Script 957, Textarchiv 738 mit 220 Nachrichten) | 142 / 197 | 136 / 135 | ja | 7 794 B, 194 ms | 232 s |
| HeartGold US | 4/4 | ja (`4fcded0e…`) | 23 | 540 | 552 B | 3 026 B (Header 2, Script 957, Textarchiv 738 mit 220 Nachrichten) | 140 / 195 | 136 / 135 | ja | 7 737 B, 196 ms | 139 s |

Grenzen: Karte ≤ 1 KiB, Karte + Script + Text ≤ 16 KiB, Erzeugen ≤ 1 s, alle eingehalten. Basis-ROM öffnen und SHA-1 rechnen 102 bis 115 ms; ARM9 entpacken bei HeartGold 6 bis 7 ms.

## (2) Szenario kopflos

Release-`g4k`, je Spiel auf seiner Kopie (`K=<Roms>/_testkopien`, `<k>` = `pt`, `hgde`, `hgus`):

```sh
G=target/release/g4k
$G base check  $K/4-1a-<k>_DSPRE_contents                        # mark=none, suggestion=…/4-1a-<k>.nds
$G base assign $K/4-1a-<k>_DSPRE_contents $K/4-1a-<k>.nds        # assigned sha1=… label=…
$G base check  $K/4-1a-<k>_DSPRE_contents                        # mark=dspre, sha1=…, base=…/4-1a-<k>.nds (sibling)
$G build $K/4-1a-<k>_DSPRE_contents --out $K/4-1a-<k>_DSPRE_contents/build/rebuild.nds
shasum $K/4-1a-<k>_DSPRE_contents/build/rebuild.nds $K/4-1a-<k>.nds
$G base assign $K/4-1a-pt_DSPRE_contents $K/4-1a-hgde.nds         # fremder Spielcode: Exit 1
$G base check  $K/4-1a-pt_DSPRE_contents --rom $K/4-1a-pt-fremd.nds   # unbekannte SHA-1: Warnung, Exit 0
$G map set-perm $K/4-1a-<k>_DSPRE_contents 3 2 0 --kind 0 --collision 0      # Platin; HeartGold mit --collision 128; muss „saved 1 file(s)“ melden
G4_BASE_ROM_DIR=$K $G patch create $K/4-1a-<k>_DSPRE_contents --json
mkdir -p target/agent/4-1a
xdelta3 -d -f -s $K/4-1a-<k>.nds <patch> target/agent/4-1a/out-<k>.nds && shasum target/agent/4-1a/out-<k>.nds $K/4-1a-<k>_DSPRE_contents/build/<Name>.nds
xdelta3 printhdr <patch>
```

- [x] 1. `g4k base check` ohne Marke: `mark=none` und `suggestion=…/4-1a-<k>.nds` (die Nachbar-`.nds`), auf allen drei Kopien.
- [x] 2. `g4k base assign` ohne Warnung: Platin DE `assigned sha1=d40f19ac… label=Platin DE (Testprojekt)`, HeartGold DE `13b6853a…`, HeartGold DE (Testprojekt), HeartGold US `4fcded0e…`, HeartGold US (Testprojekt). Danach `mark=dspre`, `sha1=…`, `base=…/4-1a-<k>.nds (sibling)`.
- [x] 3. `g4k build`, SHA-1 gleich der `.nds`: Platin DE `d40f19ac24d67c721a4ab25e1c013154443d75b4` (74 s, kalt), HeartGold DE `13b6853aa93121069df1d17446d3ecb36872aecf` (48 s), HeartGold US `4fcded0e2713dc03929845de631d0932ea2b5a37` (37 s).
- [x] 4. Fremder Spielcode: `g4k base assign 4-1a-pt_DSPRE_contents 4-1a-hgde.nds` → stderr `error: Falsches Spiel: die ROM ist IPKD, das Projekt CPUD.`, Exit 1.
- [x] 5. Unbekannte SHA-1: `g4k base check 4-1a-pt_DSPRE_contents --rom 4-1a-pt-fremd.nds` → `sha1=483fcaa8… game_code=CPUD title=POKEMON PL`, `warning: Unbekannte oder veränderte ROM; Patches gelten nur für genau diese Datei.`, Exit 0.
- [x] 6. Patch nach einer Kartenänderung (Karte 3, Zelle 2, 0), `G4_BASE_ROM_DIR=<Roms>/_testkopien g4k patch create --json`: Größe ≤ 1 KiB, `encode_ms` ≤ 1000, mit `xdelta3` 3.2.2 `-d -s` angewandt gleich `build/<Name>.nds`.

  | Spiel | Größe | `encode_ms` | `verify_ms` | Ziel-SHA-1 | `xdelta3 -d` gleich |
  |---|---|---|---|---|---|
  | Platin DE | 551 B | 133 | 147 | `e83cdb1e…` | ja |
  | HeartGold DE | 554 B | 236 | 183 | `12ec2645…` | ja |
  | HeartGold US | 554 B | 236 | 158 | `ddafbb81…` | ja |

- [x] 7. `xdelta3 printhdr`: `VCDIFF header indicator: VCD_APPHEADER`, `VCDIFF secondary compressor: none`, `VCDIFF application header: 4-1a-pt-CPUD.nds//Platin DE (Testprojekt) [d40f19ac24d67c721a4ab25e1c013154443d75b4].nds/` (HeartGold entsprechend mit `IPKD` bzw. `IPKE`), Fenster `VCD_SOURCE VCD_ADLER32`.
- [x] 8. Begleitdatei, etwa `4-1a-pt-CPUD.xdelta.txt`, im Wortlaut des Plans: Spiel, Basis- und Ziel-SHA-1, Studio-Version 0.1.0, ein Satz zum Anwenden mit xdelta3 oder Rom Patcher JS.

Patches, Begleitdateien und JSON-Berichte: `target/agent/gate-2026-10-27/4-1a/kopflos/`; Log `kopflos.log`.

## (3) Browser-Teil

Test-Umgebung auf 1500/1501, Kopie `4-1a-web` (Platin DE), Seite `http://127.0.0.1:1500/?project=<Roms>/_testkopien/4-1a-web_DSPRE_contents`.

- [x] 1. Technik › Veröffentlichen zeigt den Vorschlag: Untertitel „Keine Basis-ROM zugeordnet“, Karte „Basis-ROM“ mit Plakette „Nicht zugeordnet“, Info „Neben dem Projekt liegt „4-1a-web.nds“.“, „Zuordnen…“ und „Andere Datei wählen…“; „Patch erstellen…“ gesperrt mit „Erst eine Basis-ROM zuordnen.“
- [x] 2. „Zuordnen…“ → Dialog „Basis-ROM zuordnen?“: „POKEMON PL · CPUD · SHA-1 d40f19ac“, „Bekannte Basis-ROM: Platin DE (Testprojekt)“, keine Warnung, „Schreibt die Basismarke .g4/base.json ins Projekt.“ mit Plakette „Nicht rückgängig“. „Zuordnen“ → Plakette „Gefunden“, „Basis-ROM zugeordnet: Platin DE (Testprojekt).“, Untertitel „Bereit für einen Patch“, Angaben ROM, SHA-1 `d40f19ac` und Pfad.
- [x] 3. „Patch erstellen…“ → Dialog „Patch erstellen?“ mit „…build/4-1a-web-CPUD.xdelta und build/4-1a-web-CPUD.xdelta.txt…“ → „Patch erstellen“: zuerst Befund B2; nach dem Fix „Patch erstellt: 541 Byte“, Pfad `…/build/4-1a-web-CPUD.xdelta`, „Selbstprüfung bestanden: …“, Untertitel „Patch erstellt“, Prüfen und Bauen „gebaut gerade eben“, Statusleiste „Gebaut gerade eben“; 15,6 s vom Klick bis zum Ergebnis (mit Bauen).
- [x] 4. Fehlende Basis-ROM zeigt den Grund: `.nds` umbenannt in `4-1a-web-verschoben.nds`, Technik neu gezeigt → Plakette „Fehlt“, Warnung „Basis-ROM nicht gefunden: Platin DE (Testprojekt) (d40f19ac…).“, „Basis-ROM suchen…“, „Patch erstellen…“ gesperrt mit demselben Grund, Untertitel „Basis-ROM fehlt“.
- [x] 5. „Basis-ROM suchen…“ mit `4-1a-hgde.nds` → Fehler „Das ist eine andere ROM (SHA-1 13b6853a…), gesucht ist Platin DE (Testprojekt) (d40f19ac…).“, der Zustand bleibt „Fehlt“; mit `4-1a-web-verschoben.nds` → „Gefunden“, Pfad der umbenannten Datei, „Patch erstellen…“ frei.
- [x] 6. MCP über die Brücke (Kopie `4-1a-mcp`, Agenten an, Freigabe „Lesen und bearbeiten“ über `call.sh agent_set_grant '{"level":"write"}'`, in `agents.json` mit `base_id: null`): `mcp.sh call assign_base '{"nds":"../4-1a-mcp.nds"}'` im Hintergrund → Dialog „Agent möchte etwas Unumkehrbares tun“ mit „„agent-gate“ möchte: Basis-ROM zuordnen (lässt sich nicht rückgängig machen).“, Klick auf „Zulassen“ → `state: found`, `source: known`, `patch_name: 4-1a-mcp-CPUD`. `agents.json`: dieselbe Freigabe jetzt mit `base_id: d40f19ac…/2026-10-27T01:04:16Z`; der nächste `mcp.sh call base_status` sofort `found`, ohne Frage nach der Freigabe.
- [x] 7. Nach einem Neustart der Brücke `mcp.sh call create_patch '{}'` im Hintergrund → Dialog „… möchte: Patch erstellen (…)“, „Zulassen“ → 541 B, `encode_ms` 38, `verify_ms` 133, Exit 0, keine Frage nach der Freigabe.

## (4) Hand und Blick

Beim Nutzer (Abschnitte „Hand“ und „Blick“ unten): Rom Patcher JS im Browser mit der eigenen Basis-ROM und einem Patch aus (2); xdelta UI unter Windows (kein Windows auf diesem Rechner); der Blick auf Technik › Veröffentlichen in Hell, Dunkel und Kompakt.

## Ergebnis

| Punkt | Wer | Ergebnis | Datum |
|---|---|---|---|
| (1) 1 gated `--test base` | Agent | bestanden: je 4/4 auf Platin DE, HeartGold DE, HeartGold US; Rebuild byte-gleich, Patch Karte 551 bis 552 B, Karte + Script + Text 3 026 bis 8 227 B, Erzeugen ≤ 197 ms, `xdelta3 -d` gleich (Tabelle oben) | 2026-10-27 |
| (1) 2 `cargo deny` | Agent | grün im Lauf von `scripts/gate.sh` auf `85cb37f` („advisories ok, licenses ok“), sieben Ausnahmen mit Grund in `deny.toml`; im vollen Lauf auf `bd383f8` grün | 2026-10-27 |
| (1) 3 `scripts/gate.sh` | Agent | bestanden auf `bd383f8`, 4 091 s; Lauf 2 rot nur in HeartGold US `tests/build.rs` (Temp-Kopie ohne `config.yaml`), einzeln und in Lauf 3 grün | 2026-10-27 |
| (2) 1 bis 3 Zuordnen und Rebuild | Agent | bestanden: Vorschlag, Zuordnen ohne Warnung, Rebuild-SHA-1 gleich der `.nds` auf allen drei Kopien | 2026-10-27 |
| (2) 4 fremder Spielcode | Agent | bestanden: „Falsches Spiel: die ROM ist IPKD, das Projekt CPUD.“, Exit 1 | 2026-10-27 |
| (2) 5 unbekannte SHA-1 | Agent | bestanden: Warnung „Unbekannte oder veränderte ROM …“, Exit 0 | 2026-10-27 |
| (2) 6 bis 8 Patch, `xdelta3`, Kopf, Begleitdatei | Agent | bestanden: 551 / 554 / 554 B, `encode_ms` ≤ 236, `xdelta3 -d` gleich, App-Header mit SHA-1, keine Zweitkompression, Adler32 | 2026-10-27 |
| (3) 1 bis 3 Vorschlag, Zuordnen, Patch | Agent | bestanden nach den Fixes B1, B2 und U1 („Patch erstellt: 541 Byte“, 15,6 s mit Bauen) | 2026-10-27 |
| (3) 4 und 5 Basis-ROM fehlt, suchen | Agent | bestanden: Grund in Karte und Notiz, andere ROM abgelehnt, umbenannte Datei gefunden | 2026-10-27 |
| (3) 6 und 7 MCP | Agent | bestanden: Rückfrage „Basis-ROM zuordnen“ und „Patch erstellen“, Freigabe wandert auf die neue `base_id` | 2026-10-27 |
| Rom Patcher JS | Hand | offen | – |
| xdelta UI (Windows) | Hand | offen, kein Windows | – |
| Abnahme (unabhängig) | Agent | bestanden nach A1 bis A4 (`5a12cd7`, `693c2d4`, `5dce50e`, `c800ce7`): kopflos, Oberfläche und MCP auf frischen Kopien aller drei Spiele, siehe Abschnitt „Abnahme (2026-10-27)“ | 2026-10-27 |
| Blick Technik › Veröffentlichen | Agent + Blick | offen: 13 Bilder des Gates und 34 der Abnahme liegen bereit, vom Agenten angesehen | – |
| `.fig` | Agent | Nicht eingespielt: OpenPencil antwortete weder in der Sitzung des Gates noch in der Abnahme (RPC timeout, Bildschirm gesperrt, `CGSSessionScreenIsLocked = true`); Vorlage `design-system/openpencil/beispiele/veroeffentlichen.body.js`, Anleitung `design-system/openpencil/NACHTRAG-4-1a.md` | 2026-10-27 |

Patch-Größen je Spiel (gated Lauf (1), `patch_after_map_and_after_map_script_text_is_small_and_applies`; Erzeugen und Selbstprüfung als Karte / Karte + Script + Text):

| Spiel | Rebuild-SHA-1 gleich | Patch Karte | Patch Karte + Script + Text | Erzeugen (ms) | Selbstprüfung (ms) | `xdelta3 -d` gleich |
|---|---|---|---|---|---|---|
| Platin DE | ja (`d40f19ac…`) | 551 B | 8 227 B | 106 / 189 | 132 / 132 | ja |
| HeartGold DE | ja (`13b6853a…`) | 552 B | 3 043 B | 142 / 197 | 136 / 135 | ja |
| HeartGold US | ja (`4fcded0e…`) | 552 B | 3 026 B | 140 / 195 | 136 / 135 | ja |

`scripts/gate.sh` auf `bd383f8`: grün in 4 091 s. Schritte in Sekunden: fmt 2, clippy 6, deny 4, `cargo test` ohne ROM 79, mcp 23, `pnpm test` 247, tokens 1, styles 6, `pnpm build` 41, g4emu 17 + 9, gated `cargo test` HeartGold DE 969, HeartGold US 1 450, Platin DE 1 120, gated Projektstart 44 / 37 / 36.

## Befunde

Befunde der Gate-Runde, alle behoben, je eigener Commit, Review ohne offene Punkte:

| Nr. | Befund | Fix |
|---|---|---|
| B1 | Die Brücke sperrte jeden Pfad mit einem Bestandteil `*_DSPRE_contents`, auch Kopien unter `_testkopien`; der Vorschlag braucht aber genau diese Form (`<Name>_DSPRE_contents` neben `<Name>.nds`). | `eeba3c8`: gesperrt bleibt ein solcher Bestandteil, außer ein früherer Bestandteil des Pfads heißt `_testkopien` (dieselbe Teilung wie `is_original`). Einen neuen `*_DSPRE_contents` anlegen bleibt gesperrt (Gate 3g.1 (A4)). |
| B2 | „Patch erstellen“ über die Brücke scheiterte ohne `build/` mit „Pfad nicht lesbar: …/build/4-1a-web-CPUD.nds: No such file or directory (os error 2)“. | `996b583`: die Brücke legt einen fehlenden Ordner `build` eine Ebene über dem Ziel an (nach den Sperrprüfungen), wie die App. |
| U1 | Der Dialog „Basis-ROM zuordnen?“ leerte sich während des Zuordnens bis auf Titel und Tasten. | `24bea52`: der Inhalt bleibt, die Tasten sind gesperrt, Escape wirkt erst danach. |

Vor dem Gate aus den Reviews der Tasks behoben: die Grenze 1 GiB für Eingaben in `g4-xdelta` (`4baff2b`), Originale und `G4_BASE_ROM_DIR` in der Brücke ohne `--app-data` und im Batch (`a9c38fb`), der Patch über den Bauen-Weg der Sitzung mit Sperre von Bauen und Speichern und der Patchname vom Backend (`68bf9bd`).

**Abweichung vom Plan-Szenario (Ruling in Task 5):** Der Plan änderte für „Karte + Script + Text“ das Textarchiv 0 (1404 Nachrichten; ändert eine Nachricht, wird die ganze Offset-Tabelle neu verschlüsselt). Damit lag Platin bei 20 432 B, über 16 KiB. Die Grenze 16 KiB stammt aus dem Szenario des Gutachtens, einer typischen Änderung an einer Stelle; der gated Test nimmt deshalb deterministisch den ersten Karten-Header, dessen Textarchiv höchstens 300 Nachrichten hat, hängt an seine Scriptdatei zwei Byte und an Nachricht 0 seines Textarchivs „ Studio“ (dazu die Kartenänderung). Textarchiv 0 läuft weiter als Stresszeile ohne Grenze (Platin 19 653 B, HeartGold DE 7 794 B, HeartGold US 7 737 B).

**Hinweis zum Skript von (2), kein Befund im Produkt:** Der erste Platin-Lauf setzte `--collision 128` auf eine Zelle, die schon 128 war: `saved 0 file(s)`, der Patch (540 B) hatte als Ziel-SHA-1 die Basis. Wiederholt mit `--collision 0` (`saved 1 file(s)`, 551 B, Ziel `e83cdb1e…`); HeartGold DE und US mit `--collision 128` änderten die Zelle beim ersten Mal.

## Abnahme (2026-10-27)

Unabhängige Abnahme durch eine zweite Sitzung auf dem Stand `cf7df31`, danach die Fixes unten. Test-Umgebung auf 1530/1531 (`G4_ROMS_DIR` absolut, Browser-Fenster und Chrome headless über CDP), Release-`g4k`, `xdelta3` 3.2.2. Frische Kopien unter `Roms/_testkopien/` je Zweck (`cp -cR`, `cp -c`, Ordner und `.nds` nebeneinander): `4-1a-abn-pt`, `-hgde`, `-hgus` (kopflos), `-wpt`, `-whgde`, `-whgus` (Browser, je Spiel), `-mcp` (MCP), `-ui` (Vorschlag und Dialoge, nie zugeordnet) und `4-1a-abn-pt-fremd.nds` (letztes Byte gekippt, SHA-1 `483fcaa8…`). Ausgaben unter `target/agent/gate-2026-10-27/abnahme-4-1a/` im Worktree: `kopflos.log` und `kopflos/` (Patches, Begleitdateien, JSON), `browser/` (Patches aus der Oberfläche), `mcp/` (Antworten und Patch über MCP), `shots.log` und 34 Bilder. Alle Kopien sind nach der Abnahme gelöscht.

| Punkt | Ergebnis |
|---|---|
| Kopflos: `base check` ohne Marke | bestanden auf allen drei: `mark=none`, `suggestion=…/4-1a-abn-<k>.nds` |
| Kopflos: `base assign` mit der Nachbar-`.nds` | bestanden: ohne Warnung, `assigned sha1=d40f19ac… / 13b6853a… / 4fcded0e…`, danach `mark=dspre`, `base=… (sibling)` |
| Byte-gleicher Rebuild (`g4k build`) | bestanden: SHA-1 gleich der `.nds` auf Platin DE (`d40f19ac…`, 16 s), HeartGold DE (`13b6853a…`, 21 s), HeartGold US (`4fcded0e…`, 21 s) |
| Falscher Spielcode | bestanden: kopflos `Falsches Spiel: die ROM ist IPKD, das Projekt CPUD.` und `… CPUD, das Projekt IPKE.`, je Exit 1; in der Oberfläche (Platin mit HeartGold DE, HeartGold US mit Platin) Dialog mit Fehlermeldung und nur „Schließen“ |
| Unbekannte SHA-1 | bestanden: kopflos Warnung „Unbekannte oder veränderte ROM; Patches gelten nur für genau diese Datei.“, Exit 0; in der Oberfläche dieselbe Warnung im Dialog, „Zuordnen“ bleibt möglich; bei „Basis-ROM suchen…“ „Das ist eine andere ROM (SHA-1 483fcaa8…) …“ |
| Fehlende Basis-ROM | bestanden: kopflos `base=missing`, `patch create` Exit 1 „Basis-ROM nicht gefunden: HeartGold US (Testprojekt) (4fcded0e…).“; in der Oberfläche Plakette „Fehlt“, Grund in Karte und Notiz, „Patch erstellen…“ gesperrt (Platin und HeartGold US) |
| Zuordnen in der Oberfläche | bestanden: Platin mit Vorschlag („Zuordnen…“), HeartGold DE über „Andere Datei wählen…“ (Pfad), HeartGold US mit Vorschlag nach einem abgelehnten falschen Spiel; „Basis-ROM suchen…“ mit HeartGold-ROM und mit der veränderten Platin-ROM abgelehnt, mit der umbenannten Datei gefunden |
| Patch erstellen mit Selbstprüfung und Begleitdatei | bestanden: kopflos 555 / 558 / 558 B (`encode_ms` 117 / 157 / 148, `verify_ms` 133 / 136 / 138), Oberfläche 545 B (Platin ohne Änderung, siehe A2) / 559 / 559 B (13 bis 19 s mit Bauen), MCP 556 B; Begleitdatei im Wortlaut des Plans |
| `xdelta3 -d` als Gegenprobe | bestanden: alle sieben Patches angewandt gleich `build/<Name>.nds`; Ziel-SHA-1 `e83cdb1e…` (Platin), `12ec2645…` (HeartGold DE), `ddafbb81…` (HeartGold US), gleich wie im Gate; `printhdr`: `VCD_APPHEADER`, Zweitkompression `none`, `VCD_ADLER32` |
| Umschreiben der Agenten-Freigabe | bestanden: Freigabe „Lesen und bearbeiten“ mit `base_id: null`, nach `assign_base` über MCP `base_id: d40f19ac…/2026-10-27T04:05:30Z`; ein zweites Zuordnen derselben ROM in der Oberfläche lässt Marke und Freigabe gleich |
| MCP mit Rückfrage (`mcp.sh`) | bestanden nach A3: `base_status`, `assign_base` (Rückfrage, „Zulassen“, `state found`, `source known`), `create_patch` (Rückfrage, 556 B, `encode_ms` 137); ohne Antwort endet die Rückfrage mit `needs_confirmation` |
| Technik › Veröffentlichen Hell, Dunkel, Kompakt, 1440 × 900 und 1100 × 720 | bestanden: Bilder unten; Pfade enden gekürzt (Konvention, voller Pfad als Tooltip) |
| Tastatur | bestanden nach A1: Liste mit Pfeiltasten und Enter, Tab durch die Karten, Dialoge mit Tab, Enter, Escape; Fokus auf „Abbrechen“ beim Öffnen |
| Screenreader-Namen | bestanden nach A1: Liste `listbox` „Technik“ mit Optionen (Name und Untertitel über `aria-labelledby`/`-describedby`), Fläche `section` mit `h1`, Karten mit `h2`, Meldungen `status` bzw. `alert`, Dialoge mit Titel und Beschreibung, gesperrte Taste mit Grund über `aria-describedby` |

### Befunde

| Nr. | Schwere | Befund | Fix |
|---|---|---|---|
| A1 | Mittel | Fokus fällt auf `body`: die Tasten von Veröffentlichen sind während Prüfen, Zuordnen, Suchen und Patch `disabled`; nach dem Dialog „Basis-ROM zuordnen?“ landet der Fokus auf `body` oder auf „Andere Datei wählen…“, nach Zuordnen und Suchen verschwindet die Taste, nach „Patch erstellen“ ebenfalls `body`. Der gesperrte „Patch erstellen…“ war nicht mit seinem Grund verbunden. | `5a12cd7`: busy sperrt mit `aria-disabled` statt `disabled` (Button aus `1dfcb70` übernommen, byte-gleich mit main); nach jedem Schritt Fokus auf „Patch erstellen…“, wenn frei, sonst auf die auslösende Taste; `Dialog` bekam `finalFocus`; Notiz neben „Patch erstellen…“ über `aria-describedby`. Nachgeprüft im Browser: Escape → „Zuordnen…“, Zuordnen → „Patch erstellen…“, während des Patches bleibt der Fokus dort. |
| A2 | Gering | Ein Patch ohne Änderung (Ziel-SHA-1 = Basis-SHA-1, 545 B) hieß „Patch erstellt … Selbstprüfung bestanden“, ohne Hinweis, dass er nichts ändert. | `693c2d4`: bei Ziel-SHA-1 = Basis-SHA-1 eine Warnmeldung „Patch ohne Änderungen: 544 Byte“ mit Pfad und „Die gebaute ROM gleicht der Basis-ROM; der Patch ändert nichts.“ Nachgeprüft im Browser (Kopie `ui`). |
| A3 | Mittel | MCP `assign_base`: die Rückfrage nannte die Datei nicht, und Pfad und Spiel wurden erst nach „Zulassen“ geprüft (`../4-1a-abn-hgde.nds`: erst Rückfrage, dann „Pfad liegt außerhalb der erlaubten Ordner“). | `5dce50e`: in der App prüft `assign_base` vor der Rückfrage Pfad, Spiel und Marke (`start::check_assign`, dieselbe Prüfung wie `assign_base`) und gibt eine Ablehnung ohne Rückfrage zurück; die Rückfrage lautet „Basis-ROM zuordnen: 4-1a-abn-mcp2.nds (Platin DE (Testprojekt), SHA-1 d40f19ac)“, die von `create_patch` „Patch erstellen: build/4-1a-abn-mcp2-CPUD.xdelta“; kopflos unverändert. Nachgeprüft über `mcp.sh`: der Pfad außerhalb kam nach 1 s ohne Rückfrage zurück. |
| A4 | Gering | Veröffentlichen folgte keinem Agenten: nach `assign_base` bzw. `create_patch` über MCP blieb die offene Karte auf „Nicht zugeordnet“ bzw. ohne Patch, bis Technik neu gezeigt wurde. | `c800ce7`: nach einem Agenten-Ereignis `assign_base` oder `create_patch` liest die App den Zustand der Basis-ROM neu. Nachgeprüft: die offene Karte sprang nach „Zulassen“ auf „Gefunden“. Einen Agenten-Patch zeigt die Karte nicht (der Patch-Bericht reist nicht mit dem Ereignis), siehe H5. |

Hinweise ohne Fix:

- H1: Nach `create_patch` über MCP zeigt Prüfen und Bauen weiter „Noch nicht gebaut“ (der Agenten-Build setzt `lastRom` der Sitzung nicht); gilt ebenso für `build` über MCP seit 3h, nicht 4.1a.
- H2: Bei 1100 px kürzt die Titelleiste den Projektnamen auf „4…“ (seit 4.0 bekannt, H2 der Abnahme 4.0).
- H3: Fehlertexte des Betriebssystems bleiben englisch im deutschen Satz („Pfad nicht lesbar: …: Not a directory (os error 20)“), wie seit 3g.
- H4: Das Pfadfeld des Browser-Modus merkt sich den letzten Pfad (gewollt); Tippen hängt an, statt zu ersetzen. Nur Test-Umgebung.
- H5: Einen Patch, den ein Agent erstellt hat, zeigt die Karte „Patch“ nicht: `lastPatch` setzt nur der eigene Lauf der Oberfläche, und das Ereignis trägt keinen Patch-Bericht. Der Untertitel bleibt „Bereit für einen Patch“. Dafür müsste das Backend den letzten Patch melden; offen.
- H6: Die Tasten im Dialog „Basis-ROM zuordnen?“ sind während des Zuordnens weiter `disabled`; der Fokus liegt so lange auf `body` und kommt mit dem Schließen zurück (`finalFocus`). Eine Option `aria-disabled` für `DialogAction` wäre der Fix; nicht gemacht, weil das Zuordnen unter einer Sekunde dauert.
- H7: `create_patch` über MCP fragt weiter vor der Suche nach der Basis-ROM; fehlt sie, kommt `base_missing` erst nach „Zulassen“ (die Suche kann eine ROM hashen).
- H8: Der Pfad in Angaben und Erfolgsmeldung wird am Ende gekürzt, der Dateiname des Patches ist bei 1100 px nicht zu sehen (Konvention aus Einstellungen › Agenten, voller Pfad als Tooltip).

### Nach den Fixes

Auf `c800ce7` mit neu gebauter Brücke nachgeprüft (Kopien `ui` und die frische `4-1a-abn-mcp2`): A1 bis A4 wie in der Tabelle beschrieben; die Freigabe von `mcp2` wanderte auf `d40f19ac…/2026-10-27T04:41:24Z`; `create_patch` über MCP ohne Änderung 546 B, Ziel = Basis. Checks auf `c800ce7` (wie `scripts/gate.sh` ohne die gated Schritte, Log `checks.log`), alle grün: `cargo fmt --check` 2 s, `cargo clippy -D warnings` 22 s, `cargo deny` 7 s („advisories ok, licenses ok“), `cargo test --workspace --all-features` ohne ROM 99 s, `mcp_stdio` 26/26 in 26 s, `pnpm test` 102 s, `check-tokens.sh`, `check-styles.sh`, `pnpm build` 51 s. Der volle Lauf von `scripts/gate.sh` mit den gated Tests lief nach den Fixes nicht erneut; die Fixes berühren weder Build noch Patch des Kernels (A3 verschiebt nur die Prüfung von `assign_base` in `start::check_assign`, die Unit-Tests von `start` und `mcp::tools` laufen ohne ROM).

### Bilder

34 Bilder unter `target/agent/gate-2026-10-27/abnahme-4-1a/` im Worktree, Stand `cf7df31` (vor den Fixes; die Fixes ändern das Bild nur bei einem Patch ohne Änderung), Chrome headless über CDP mit der laufenden Test-Umgebung; Hell und Dunkel über `prefers-color-scheme` (Thema „System“), Kompakt über die Taste der Statusleiste:

- `vorschlag-*` (Kopie `ui`): Plakette „Nicht zugeordnet“, Info „Neben dem Projekt liegt „4-1a-abn-ui.nds“.“, „Zuordnen…“ primär, „Andere Datei wählen…“ sekundär, „Patch erstellen…“ gesperrt mit „Erst eine Basis-ROM zuordnen.“
- `zuordnen-dialog-*`: „Basis-ROM zuordnen?“, „POKEMON PL · CPUD · SHA-1 d40f19ac“, „Bekannte Basis-ROM: Platin DE (Testprojekt)“, Plakette „Nicht rückgängig“, „Abbrechen“ und „Zuordnen“.
- `falsches-spiel-hell-1440`, `-dunkel-1440`: derselbe Dialog mit HeartGold DE, Fehlermeldung „Falsches Spiel: die ROM ist IPKD, das Projekt CPUD.“, nur „Schließen“.
- `unbekannte-rom-hell-1440`, `-dunkel-1440`: SHA-1 `483fcaa8`, Warnung „Unbekannte oder veränderte ROM …“, „Zuordnen“ bleibt.
- `gefunden-*` (Kopie `whgde`): Plakette „Gefunden“, ROM „HeartGold DE (Testprojekt)“, SHA-1 `13b6853a`, Pfad in Mono.
- `patch-erstellt-*` (Kopie `whgde`): „Patch erstellt: 559 Byte“, Pfad, Selbstprüfung; Prüfen und Bauen „gebaut gerade eben“.
- `fehlt-*` (Kopie `whgus`, `.nds` umbenannt): Plakette „Fehlt“, Warnung „Basis-ROM nicht gefunden: HeartGold US (Testprojekt) (4fcded0e…).“, „Basis-ROM suchen…“, „Patch erstellen…“ gesperrt mit demselben Grund.

Je Zustand `-hell-1440`, `-dunkel-1440`, `-kompakt-1440`, `-hell-1100`, `-dunkel-1100`, `-kompakt-1100` (1440 × 900 bzw. 1100 × 720). Bei 1100 px passen beide Karten ohne Umbruch der Tasten; im Kompakt zeigt die Leiste „Bereiche“ nur Symbole.

## Hand

- [ ] **Rom Patcher JS** im Browser mit der eigenen Basis-ROM: den Patch aus (2) anwenden, etwa `target/agent/gate-2026-10-27/4-1a/kopflos/4-1a-pt-CPUD.xdelta` auf das eigene Platin DE mit SHA-1 `d40f19ac24d67c721a4ab25e1c013154443d75b4`. Die SHA-1 des Ergebnisses muss gleich der Ziel-SHA-1 in der Begleitdatei `4-1a-pt-CPUD.xdelta.txt` sein: `e83cdb1ecca5e98b077464c24e9c1b530840f76b`. Die Fassung von Rom Patcher JS notieren. (HeartGold DE: `4-1a-hgde-IPKD.xdelta`, Ziel `12ec26454e951e27c808976226974053e3d705c7`; HeartGold US: `4-1a-hgus-IPKE.xdelta`, Ziel `ddafbb8114dbab85208cd2a25cf779f0cde86d77`.)
- [ ] **xdelta UI unter Windows:** derselbe Patch, dieselbe Prüfung; auf diesem Rechner gibt es kein Windows. Der Bau von `g4-xdelta` unter Windows (MSVC) ist ebenfalls ungeprüft.
- Stand der Abnahme (2026-10-27): beide Punkte weiter offen, der Agent hat keinen Zugang zu Rom Patcher JS mit einer eigenen ROM des Nutzers und kein Windows. Die Patches der Abnahme sind dieselben Ziele mit anderem Namen im App-Header und taugen ebenso: `target/agent/gate-2026-10-27/abnahme-4-1a/kopflos/4-1a-abn-pt-CPUD.xdelta` (Ziel `e83cdb1e…`), `4-1a-abn-hgde-IPKD.xdelta` (`12ec2645…`), `4-1a-abn-hgus-IPKE.xdelta` (`ddafbb81…`); dazu die Patches aus der Oberfläche unter `abnahme-4-1a/browser/`. Ein Patch ohne Änderung (`browser/4-1a-abn-wpt-CPUD.xdelta`, Ziel = Basis `d40f19ac…`) prüft, dass Rom Patcher JS auch einen leeren Patch annimmt.

## Blick

Bilder unter `target/agent/gate-2026-10-27/4-1a/` im Worktree, je 1440 × 900, aus der Test-Umgebung über Chrome headless (CDP). Die Werkzeuge des Browser-Fensters liefern Bilder nur an den Agenten und schreiben keine Datei.

**Neu aufgenommen am 2026-10-31.** Die Bilder unter `target/agent/gate-2026-10-27/` gingen mit dem Ordner `target/` verloren. Sie sind neu aufgenommen, auf dem Stand von main `03a5615` (alle Fixes bis 4.4), in der Test-Umgebung mit Chrome headless (Playwright) auf frischen Kopien. Die Namen sind dieselben wie unten. Neuer Ort: `target/agent/gate-2026-10-31/blick-neu/4-1a/` im Worktree `.claude/worktrees/blick-4` (Branch `blick-phase-4`), dazu eine Kopie außerhalb des Repos unter `/Users/POLTEP/Projekte/lab/editor/_blick-phase-4/gate-2026-10-31/blick-neu/4-1a/`. Es sind 18 Bilder: die 13 `blick-veroeffentlichen-*` und ein neues, dazu die vier Bilder der Abnahme, die unten für den Nutzer genannt sind, in `abnahme-4-1a/`. Die übrigen 30 Bilder der Abnahme sind nicht neu aufgenommen; sie zeigten den Stand vor den Fixes. Abweichungen vom alten Inhalt:

- **Alle Bilder:** Die Kopien heißen `blick4-4-1a-<zweck>`, also etwa „Neben dem Projekt liegt „blick4-4-1a-ui.nds“.“
- **Technik:** Die Liste hat seit 4.1b und 4.3b mehr Einträge (Textprojekt, Änderungen). Die Meldung „Dieses Projekt lässt sich als Textprojekt einrichten …“ ist vor jeder Aufnahme geschlossen. Die Zahl der Warnungen in der Statusleiste steht je nach Stand der Hintergrundprüfung da oder nicht.
- **`blick-veroeffentlichen-patch-erstellt-hell.png`:**
  - Seit A2 heißt ein Patch ohne Änderung „Patch ohne Änderungen“. Für „Patch erstellt“ ist deshalb vorher eine Zelle der Bewegungsrechte geändert (`g4k map set-perm`). Das ergibt „Patch erstellt: 559 Byte“ statt 541 Byte.
  - Die Datei der Basis-ROM ist jetzt die eigene `…-web.nds`, nicht `4-1a-mcp.nds`: Es lief keine MCP-Zuordnung einer zweiten Kopie dazwischen.
- **Neu: `blick-veroeffentlichen-patch-ohne-aenderungen-hell.png`.** Es zeigt die Warnung „Patch ohne Änderungen: 549 Byte“ mit dem Satz „Die gebaute ROM gleicht der Basis-ROM; der Patch ändert nichts.“ (A2). Dazu gab es vorher kein Bild.
- **`abnahme-4-1a/patch-erstellt-dunkel-1440.png`:** HeartGold DE mit einer geänderten Zelle, „Patch erstellt: 561 Byte“ (Abnahme: 559 Byte ohne Änderung, vor A2).
- **`abnahme-4-1a/zuordnen-dialog-dunkel-1440.png`:** Dieselbe Aufnahme wie `blick-veroeffentlichen-zuordnen-dialog-dunkel.png`.
- **Test-Umgebung:** Die Instanz lief ohne `G4_BASE_ROM_DIR`. Mit dieser Variable findet die Suche eine andere Kopie mit derselben SHA-1, und „Fehlt“ erscheint nie.

- [ ] `blick-veroeffentlichen-vorschlag-hell.png`, `-dunkel.png`, `-kompakt-hell.png` (Kopie `4-1a-ui`): Untertitel „Keine Basis-ROM zugeordnet“, Plakette „Nicht zugeordnet“, Info-Meldung „Neben dem Projekt liegt „4-1a-ui.nds“.“, primär „Zuordnen…“, sekundär „Andere Datei wählen…“; in „Patch“ die gesperrte Taste mit der Notiz „Erst eine Basis-ROM zuordnen.“ Je Karte höchstens eine primäre Taste.
- [ ] `blick-veroeffentlichen-zuordnen-dialog-hell.png`, `-dunkel.png`, `-kompakt-hell.png`: der Dialog „Basis-ROM zuordnen?“ über der abgedunkelten Fläche, „POKEMON PL · CPUD · SHA-1 d40f19ac“, „Bekannte Basis-ROM: Platin DE (Testprojekt)“, keine Warnung, die Plakette „Nicht rückgängig“, „Abbrechen“ und „Zuordnen“ (primär).
- [ ] `blick-veroeffentlichen-gefunden-hell.png`, `-dunkel.png`, `-kompakt-hell.png`: Plakette „Gefunden“ (Erfolg), Angaben ROM, SHA-1 (acht Stellen in Mono), Datei (Pfad in Mono, gekürzt); „Patch erstellen…“ frei.
- [ ] `blick-veroeffentlichen-fehlt-hell.png`, `-dunkel.png`, `-kompakt-hell.png` (Kopie `4-1a-web`): Untertitel „Basis-ROM fehlt“, Plakette „Fehlt“ (Warnung), Warnmeldung „Basis-ROM nicht gefunden: Platin DE (Testprojekt) (d40f19ac…).“, primär „Basis-ROM suchen…“; „Patch erstellen…“ gesperrt mit demselben Grund als Notiz.
- [ ] `blick-veroeffentlichen-patch-erstellt-hell.png` (Kopie `4-1a-web`): Untertitel „Patch erstellt“, Prüfen und Bauen „gebaut gerade eben“, Erfolgsmeldung „Patch erstellt: 541 Byte“ mit dem Pfad `…/build/4-1a-web-CPUD.xdelta` in Mono (gekürzt) und dem Satz der Selbstprüfung. Die Datei der Basis-ROM zeigt `4-1a-mcp.nds`: `base-roms.json` hat einen Eintrag je SHA-1, und die MCP-Zuordnung von `4-1a-mcp` (gleiche SHA-1) kam danach (so entschieden).

- [ ] **Abnahme (2026-10-27):** 34 Bilder unter `target/agent/gate-2026-10-27/abnahme-4-1a/` in Hell, Dunkel und Kompakt, je bei 1440 × 900 und 1100 × 720 (Liste im Abschnitt „Abnahme“, „Bilder“). Der Agent hat sie angesehen: Texte und Zustände wie oben, je Karte höchstens eine primäre Taste, keine abgeschnittenen Tasten bei 1100 px, Kompakt mit Symbolleiste. Beim Nutzer: ein Blick auf `vorschlag-kompakt-1100.png`, `zuordnen-dialog-dunkel-1440.png`, `patch-erstellt-dunkel-1440.png` und `fehlt-hell-1100.png`; neu seit A2 ist die Warnmeldung „Patch ohne Änderungen: … Byte“ (nur im Browser-Fenster gesehen, kein Bild).

Vergleich mit dem Zielbild „Veröffentlichen: Patch erstellen“ und den Abweichungen: `design-system/README.md`, Abschnitt „Nachtrag 4.1a“.

## Bekannte Grenzen und Folgepunkte

- `base-roms.json` hat einen Eintrag je SHA-1: zwei Projekte mit derselben Basis-ROM teilen den gemerkten Pfad; der zuletzt zugeordnete gilt (siehe das Bild `patch-erstellt`).
- Kopflos (`studio mcp --project`, `g4k`) wird `base-roms.json` nicht gelesen; dort gelten `--base-rom`, die Nachbar-`.nds` und `G4_BASE_ROM_DIR`.
- Die gebaute ROM des Patches bleibt als `build/<Name>.nds` liegen (128 MiB je Projekt); ob sie nach `.g4/tmp/` gehört, ist offen.
- Die bekannten SHA-1 in `crates/g4-kernel/metadata/base_roms.yaml` sind nur die drei Testprojekt-Dumps (keine No-Intro-Prüfsummen); bei echten Nutzer-Dumps warnt „Unbekannte oder veränderte ROM“ öfter als nötig.
- Lizenztext von xdelta3 (Apache-2.0 §4) im App-Bundle: entscheidet der nächste Release-Schritt (`docs/release.md`), nicht 4.1a.
- Einstiege: keine Taste „Veröffentlichen“ in der Titelleiste und kein Hinweis beim Öffnen eines DSPRE-Ordners mit Nachbar-`.nds`; beides nicht entschieden, 4.1a zeigt es nur in Technik.
- Merge mit 4.2: angehängte Einträge in `dispatch.rs`, `lib.rs`, `access.rs`, `catalog.rs`, `g4k.rs` und die Liste in `TechnikArea.tsx`; der Katalog hat 44 MCP-Werkzeuge bei der Grenze 45 von `tool_count_is_about_thirty`.
