# Phase 4.1a: Gate prüfen

Stand: 2026-10-27. Branch `phase-4-1a-basis-rom-patch` (Worktree `.claude/worktrees/phase-4-1a`, Basis `d25f61d`), nicht nach main gemergt. Gate vom Agenten bestanden: (1) Automatik, (2) Szenario kopflos und (3) Browser-Teil mit MCP auf dem Stand `24bea52` (16 Commits `bf2921f` bis `24bea52`); die drei Befunde der Gate-Runde (B1, B2, U1) sind behoben, je mit eigenem Commit. Beim Nutzer offen: Rom Patcher JS mit der eigenen Basis-ROM (Hand), xdelta UI unter Windows (Hand, kein Windows) und der Blick auf Technik › Veröffentlichen in Hell, Dunkel und Kompakt; Abschnitte „Hand“ und „Blick“.

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
- [x] 2. `cargo deny (licenses, advisories)` als Schritt von `scripts/gate.sh` (nach clippy); `deny.toml` mit sieben Ausnahmen je RUSTSEC-ID, jede mit Grund.
- [ ] 3. `scripts/gate.sh` grün gegen alle drei Projekte: **wird nachgetragen** (Ergebnis und Dauer, Log `target/agent/gate-2026-10-27/4-1a/gate-sh.log`).

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
$G map set-perm $K/4-1a-<k>_DSPRE_contents 3 2 0 --kind 0 --collision 0      # Platin; HeartGold mit --collision 128
G4_BASE_ROM_DIR=$K $G patch create $K/4-1a-<k>_DSPRE_contents --json
xdelta3 -d -s $K/4-1a-<k>.nds <patch> out.nds && shasum out.nds $K/4-1a-<k>_DSPRE_contents/build/<Name>.nds
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
| (1) 2 `cargo deny` | Agent | bestanden als Schritt von `scripts/gate.sh`, sieben Ausnahmen mit Grund in `deny.toml` | 2026-10-27 |
| (1) 3 `scripts/gate.sh` | Agent | wird nachgetragen | – |
| (2) 1 bis 3 Zuordnen und Rebuild | Agent | bestanden: Vorschlag, Zuordnen ohne Warnung, Rebuild-SHA-1 gleich der `.nds` auf allen drei Kopien | 2026-10-27 |
| (2) 4 fremder Spielcode | Agent | bestanden: „Falsches Spiel: die ROM ist IPKD, das Projekt CPUD.“, Exit 1 | 2026-10-27 |
| (2) 5 unbekannte SHA-1 | Agent | bestanden: Warnung „Unbekannte oder veränderte ROM …“, Exit 0 | 2026-10-27 |
| (2) 6 bis 8 Patch, `xdelta3`, Kopf, Begleitdatei | Agent | bestanden: 551 / 554 / 554 B, `encode_ms` ≤ 236, `xdelta3 -d` gleich, App-Header mit SHA-1, keine Zweitkompression, Adler32 | 2026-10-27 |
| (3) 1 bis 3 Vorschlag, Zuordnen, Patch | Agent | bestanden nach den Fixes B2 und U1 („Patch erstellt: 541 Byte“, 15,6 s mit Bauen) | 2026-10-27 |
| (3) 4 und 5 Basis-ROM fehlt, suchen | Agent | bestanden: Grund in Karte und Notiz, andere ROM abgelehnt, umbenannte Datei gefunden | 2026-10-27 |
| (3) 6 und 7 MCP | Agent | bestanden: Rückfrage „Basis-ROM zuordnen“ und „Patch erstellen“, Freigabe wandert auf die neue `base_id` | 2026-10-27 |
| Rom Patcher JS | Hand | offen | – |
| xdelta UI (Windows) | Hand | offen, kein Windows | – |
| Blick Technik › Veröffentlichen | Agent + Blick | offen: 13 Bilder liegen bereit | – |
| `.fig` | Agent | Nicht eingespielt: OpenPencil antwortete in dieser Sitzung nicht (RPC timeout); Vorlage `design-system/openpencil/beispiele/veroeffentlichen.body.js`, Anleitung `design-system/openpencil/NACHTRAG-4-1a.md` | 2026-10-27 |

Patch-Größen je Spiel (gated Lauf (1), `patch_after_map_and_after_map_script_text_is_small_and_applies`; Erzeugen und Selbstprüfung als Karte / Karte + Script + Text):

| Spiel | Rebuild-SHA-1 gleich | Patch Karte | Patch Karte + Script + Text | Erzeugen (ms) | Selbstprüfung (ms) | `xdelta3 -d` gleich |
|---|---|---|---|---|---|---|
| Platin DE | ja (`d40f19ac…`) | 551 B | 8 227 B | 106 / 189 | 132 / 132 | ja |
| HeartGold DE | ja (`13b6853a…`) | 552 B | 3 043 B | 142 / 197 | 136 / 135 | ja |
| HeartGold US | ja (`4fcded0e…`) | 552 B | 3 026 B | 140 / 195 | 136 / 135 | ja |

`scripts/gate.sh`: **wird nachgetragen** (Ergebnis und Dauer).

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

## Hand

- [ ] **Rom Patcher JS** im Browser mit der eigenen Basis-ROM: den Patch aus (2) anwenden, etwa `target/agent/gate-2026-10-27/4-1a/kopflos/4-1a-pt-CPUD.xdelta` auf das eigene Platin DE mit SHA-1 `d40f19ac24d67c721a4ab25e1c013154443d75b4`. Die SHA-1 des Ergebnisses muss gleich der Ziel-SHA-1 in der Begleitdatei `4-1a-pt-CPUD.xdelta.txt` sein: `e83cdb1ecca5e98b077464c24e9c1b530840f76b`. Die Fassung von Rom Patcher JS notieren. (HeartGold DE: `4-1a-hgde-IPKD.xdelta`, Ziel `12ec26454e951e27c808976226974053e3d705c7`; HeartGold US: `4-1a-hgus-IPKE.xdelta`, Ziel `ddafbb8114dbab85208cd2a25cf779f0cde86d77`.)
- [ ] **xdelta UI unter Windows:** derselbe Patch, dieselbe Prüfung; auf diesem Rechner gibt es kein Windows. Der Bau von `g4-xdelta` unter Windows (MSVC) ist ebenfalls ungeprüft.

## Blick

Bilder unter `target/agent/gate-2026-10-27/4-1a/` im Worktree, je 1440 × 900, aus der Test-Umgebung über Chrome headless (CDP). Die Werkzeuge des Browser-Fensters liefern Bilder nur an den Agenten und schreiben keine Datei.

- [ ] `blick-veroeffentlichen-vorschlag-hell.png`, `-dunkel.png`, `-kompakt-hell.png` (Kopie `4-1a-ui`): Untertitel „Keine Basis-ROM zugeordnet“, Plakette „Nicht zugeordnet“, Info-Meldung „Neben dem Projekt liegt „4-1a-ui.nds“.“, primär „Zuordnen…“, sekundär „Andere Datei wählen…“; in „Patch“ die gesperrte Taste mit der Notiz „Erst eine Basis-ROM zuordnen.“ Je Karte höchstens eine primäre Taste.
- [ ] `blick-veroeffentlichen-zuordnen-dialog-hell.png`, `-dunkel.png`, `-kompakt-hell.png`: der Dialog „Basis-ROM zuordnen?“ über der abgedunkelten Fläche, „POKEMON PL · CPUD · SHA-1 d40f19ac“, „Bekannte Basis-ROM: Platin DE (Testprojekt)“, keine Warnung, die Plakette „Nicht rückgängig“, „Abbrechen“ und „Zuordnen“ (primär).
- [ ] `blick-veroeffentlichen-gefunden-hell.png`, `-dunkel.png`, `-kompakt-hell.png`: Plakette „Gefunden“ (Erfolg), Angaben ROM, SHA-1 (acht Stellen in Mono), Datei (Pfad in Mono, gekürzt); „Patch erstellen…“ frei.
- [ ] `blick-veroeffentlichen-fehlt-hell.png`, `-dunkel.png`, `-kompakt-hell.png` (Kopie `4-1a-web`): Untertitel „Basis-ROM fehlt“, Plakette „Fehlt“ (Warnung), Warnmeldung „Basis-ROM nicht gefunden: Platin DE (Testprojekt) (d40f19ac…).“, primär „Basis-ROM suchen…“; „Patch erstellen…“ gesperrt mit demselben Grund als Notiz.
- [ ] `blick-veroeffentlichen-patch-erstellt-hell.png` (Kopie `4-1a-web`): Untertitel „Patch erstellt“, Prüfen und Bauen „gebaut gerade eben“, Erfolgsmeldung „Patch erstellt: 541 Byte“ mit dem Pfad `…/build/4-1a-web-CPUD.xdelta` in Mono (gekürzt) und dem Satz der Selbstprüfung. Die Datei der Basis-ROM zeigt `4-1a-mcp.nds`: `base-roms.json` hat einen Eintrag je SHA-1, und die MCP-Zuordnung von `4-1a-mcp` (gleiche SHA-1) kam danach (so entschieden).

Vergleich mit dem Zielbild „Veröffentlichen: Patch erstellen“ und den Abweichungen: `design-system/README.md`, Abschnitt „Nachtrag 4.1a“.

## Bekannte Grenzen und Folgepunkte

- `base-roms.json` hat einen Eintrag je SHA-1: zwei Projekte mit derselben Basis-ROM teilen den gemerkten Pfad; der zuletzt zugeordnete gilt (siehe das Bild `patch-erstellt`).
- Kopflos (`studio mcp --project`, `g4k`) wird `base-roms.json` nicht gelesen; dort gelten `--base-rom`, die Nachbar-`.nds` und `G4_BASE_ROM_DIR`.
- Die gebaute ROM des Patches bleibt als `build/<Name>.nds` liegen (128 MiB je Projekt); ob sie nach `.g4/tmp/` gehört, ist offen.
- Die bekannten SHA-1 in `crates/g4-kernel/metadata/base_roms.yaml` sind nur die drei Testprojekt-Dumps (keine No-Intro-Prüfsummen); bei echten Nutzer-Dumps warnt „Unbekannte oder veränderte ROM“ öfter als nötig.
- Lizenztext von xdelta3 (Apache-2.0 §4) im App-Bundle: entscheidet der nächste Release-Schritt (`docs/release.md`), nicht 4.1a.
- Einstiege: keine Taste „Veröffentlichen“ in der Titelleiste und kein Hinweis beim Öffnen eines DSPRE-Ordners mit Nachbar-`.nds`; beides nicht entschieden, 4.1a zeigt es nur in Technik.
- Merge mit 4.2: angehängte Einträge in `dispatch.rs`, `lib.rs`, `access.rs`, `catalog.rs`, `g4k.rs` und die Liste in `TechnikArea.tsx`; der Katalog hat 44 MCP-Werkzeuge bei der Grenze 45 von `tool_count_is_about_thirty`.
