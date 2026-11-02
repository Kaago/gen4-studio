# Phase 3d.4: Gate prüfen

Stand: 2026-11-02. Branch `phase-3d-4-loeschen` (Worktree `.claude/worktrees/phase-3d-4`, Basis main `79eb730a`, 3d.1 ist darin), Kopf `4e182dcb` bei der Anlage dieses Dokuments, nicht nach main gemergt. Alle Plan-Tasks sind umgesetzt (Kernel, Index, „Nummer freigeben“, Warp-Löschen, Plakette und Diff, MCP und `g4k`, Dialog und Listen, Design-System-Nachtrag) und von der Review-Schleife je Task abgenommen. **Das Gate ist noch nicht gelaufen:** dieses Dokument ist das Gerüst. Der Steuer-Agent arbeitet (1) bis (3) in der UI-Test-Phase nach dem Review des ganzen Branches ab, trägt Ergebnisse, Zahlen und Befunde ein und setzt dann den Stand in [docs/ROADMAP.md](ROADMAP.md); (4) trägt der Nutzer.

Phase 3d.4 bringt das Löschen mit Referenzsperre: im Kernel den **Grabstein** (`deleted: true` als letzter Schlüssel der Entitätsdatei, ein Undo-Schritt im Store, `Tx::set_deleted`, jede Art unter `data/` mit JSON), `schemaVersion` 2 mit der Migration 1 → 2 (Manifest und die Namen `hidden_flag`/`var` in der Event-JSON), den **deklarativen Index** der Welt-Kanten (`metadata/entity_refs.yaml`: Header → Matrix, Eventdatei, Script, Textarchiv, Gebietsdaten, Ort, Begegnungen; Matrix → Karte und Header; Warp → Header und Tür; Gebietsdaten → Texturen; Karte → Gebäudemodelle) und die Script-Ziele `header` (`Warp`) und `warp_index` (`SetWarpEventPos`, `MoveWarp`), `plan_delete`/`delete`/`restore`, die **Referenzsperre** (harte Verweise sperren mit der Liste, `refs::strength` mit `SOFT_SOURCES` ist die Stelle für 3d.6), **Warp löschen ohne stille Umlenkung** (`delete::warps`: eingehende Warps auf spätere Türen rücken nach, ein Script, das eine spätere Tür per Index umsetzt, sperrt), **„Nummer freigeben“** (neue Entitäten am Ende, mit Dateien, unumkehrbar mit Undo-Grenze) und der **Import, der über Git hinweg kürzt** (Fingerabdruck-Regel aus 4.2; `textproject.member-removed`), die Prüfungen, die einen Grabstein als „fehlt“ lesen, „Auf Original zurücksetzen“ und „Wiederherstellen“, den Diff „Gelöscht“; in der Oberfläche den Dialog „Löschen…“, die Plakette `geändert`/`neu`/`gelöscht` im Inspector-Kopf, „Gelöschte zeigen“ in den Listen, die Picker ohne Gelöschte und Technik › Änderungen „Gelöscht“; dazu MCP `plan_delete`, `delete_entity`, `restore_entity` und `g4k delete plan|apply|restore`. Roadmap: [docs/ROADMAP.md](ROADMAP.md), Abschnitt „Phase 3d“, Zeile „3d.4“. Spezifikation: [2026-10-30-phase-3d-erzaehlen-design.md](superpowers/specs/2026-10-30-phase-3d-erzaehlen-design.md) §3.4, §4. Plan: [2026-10-31-phase-3d-4-loeschen.md](superpowers/plans/2026-10-31-phase-3d-4-loeschen.md). Formate: [text_project.md](formats/text_project.md) (Grabstein, Kürz-Regel, `schemaVersion` 2); Werkzeuge: [mcp.md](mcp.md) (`plan_delete`, `delete_entity`, `restore_entity`, `delete_event`).

Gate-Text der Spezifikation (§4, Zeile 3d.4):

> Je löschbarer Art eine neue Entität anlegen, löschen (Grabstein), bauen: ROM baut, `vanilla_checks` 0 Fehler, ⌘Z holt sie byte-gleich zurück; eine verwiesene Entität (Trainer an einer Figur, Begegnungsdatei an einem Header, Karte in einer Matrix, Header als Warp-Ziel und in einem `Warp`-Script) wird mit der Liste der Verweise abgelehnt, in der Oberfläche und als `referenced` über MCP; Original-Header, -Scriptdatei, -Textarchiv abgelehnt mit Grund; „Nummer freigeben“ kürzt das Archiv, `git status` zeigt die entfernte Datei, ein zweiter Branch ohne die Datei baut nach `git switch`; Warp in der Mitte löschen: eingehende Warps zeigen weiter auf dieselbe Tür (`g4emu` geht durch beide Türen); „Auf Original zurücksetzen“ stellt einen gelöschten Original-Trainer her; Diff zeigt „Gelöscht“; Plakette im Inspector-Kopf; `textproject_round_trip` und `export_idempotent` mit Grabsteinen grün; Migration 1 → 2 ändert keine Datei außer Manifest und den Namen in Event-JSON

Wer was prüft: Die Spalte „Wer“ sagt **Agent** (die Test-Umgebung aus [docs/agent-gate.md](agent-gate.md): Browser-Fenster mit `studio-bridge`, `call.sh`, `mcp.sh`, dazu `g4k`, `studio mcp` kopflos, `g4emu` und die gated Tests), **Agent + Blick** (der Agent prüft und beschreibt Ort und Zahlen, der Nutzer bestätigt kurz am Bild), **Hand** (der Nutzer) oder **Blick** (der Nutzer sieht die Bilder an). Befehle: [docs/agent-gate.md](agent-gate.md), Abschnitt „Phase 3d.4“.

Namen in der Oberfläche (Rollen und `data-*` für die Prüfung):

- **Dialog „Löschen…“** `[data-delete-dialog]` (Titel „<Name> löschen“, etwa „Trainer 0930 löschen“; Menüeintrag „Löschen…“ im Menü „Aktionen“ der Köpfe Gebiet, Raum, Ort (Header), Karte, Trainer und Begegnungsdatei; für Matrix, Eventdatei, Gebietsdaten, Scriptdatei, Textarchiv und die Karte in Technik › Textprojekt, Abschnitt „Löschen“ `[data-delete-section]` mit „Art“, „Nummer“ und „Löschen…“, nur in Tüfteln). Zustände: „Prüfe Verweise …“; **abgelehnt** (nur „Schließen“, die Fehlermeldung mit dem Grund); **Verweise** (Fehlermeldung „{n} Verweise: zuerst umhängen.“, bei einem „1 Verweis: zuerst umhängen.“, darunter die Liste `[data-reference]` mit dem Weg zur Quelle, „Löschen“ gesperrt); **Warnungen** (Original: „Original aus der Basis-ROM: der Eintrag bleibt im ROM; „Auf Original zurücksetzen“ holt ihn zurück.“; weiche Verweise `[data-warning-reference]`, in 3d.4 leer); Hinweis „Inhalt und Member bleiben; ⌘Z holt den Eintrag zurück.“; **„Nummer freigeben“** `[data-free-number]`: Kästchen „Nummer freigeben (nicht rückgängig)“ mit Plakette „Nicht rückgängig“ und der Zeile „Entfernt <n> Datei(en): …“, nur wenn `can_free`; sonst steht der Satz aus `cannot_free` (`[data-cannot-free]`, etwa „Nummer {n} bleibt belegt, weil {m} spätere Einträge leben.“). Tasten „Abbrechen“ und die eine primäre „Löschen“ (mit dem Kästchen „Löschen und Nummer freigeben“); Erfolg „Gelöscht: <Name>“ bzw. „Nummer freigegeben: <Name>“.
- **Sätze des Kernels** (wörtlich): „Original-{Art} lassen sich nicht löschen: der Spielcode verweist unsichtbar darauf (ARM9, Overlays).“ (`ORIGINAL_KIND`: Header, Gebietsdaten, Scriptdatei, Textarchiv, Texturen, Modelle der Basis); „Scriptdateien und Textarchive haben keinen Grabstein; neue am Ende lassen sich mit „Nummer freigeben“ entfernen.“; „Texturen und Modelle haben keinen Grabstein; neue am Ende lassen sich mit „Nummer freigeben“ entfernen.“; „Echtes Löschen gibt es im Textprojekt (Technik › Textprojekt).“ (DSPRE-Layout); „Nur der letzte eigene Ort lässt sich entfernen; Orte rücken nicht nach.“; „{Ziel} ist gelöscht.“ (ein neuer Verweis auf einen Grabstein); Agentenfrage „Nummer freigeben“.
- **Plakette im Kopf** `[data-entity-state="changed|new|deleted"]` (`EntityStateBadge`: „geändert“ Warnung, „neu“ Info, „gelöscht“ Fehler) im Inspector-Kopf von Trainer, Begegnungsdatei und Header (Gebiet, Raum, Ort) und im Kopf der Karte; in den Listen die Plakette „gelöscht“ an der Zeile.
- **„Gelöschte zeigen“** `[data-show-deleted="true|false"]` (Standard aus) im Kopf der Listen Trainer, Begegnungen und Welt › Orte; gelöschte Zeilen tragen `data-deleted="true"` an `[data-trainer]`, `[data-encounter]`, `[data-header-id]`, die Plakette „gelöscht“ und im Kontextmenü „Wiederherstellen“ (vor „Auf Original zurücksetzen…“); die Picker (Trainer am Script-Feld, Begegnungsdatei am Header, Header-Picker, Tür-Ziel) lassen Gelöschte aus; am Feld „Tür“ einer Tür mit gelöschtem Ziel ist „Ändern…“ gesperrt („Das Ziel ist gelöscht.“).
- **Technik › Änderungen:** Zeilen mit der Plakette „Gelöscht“ (Fehler); im Detail die Plakette `[data-tombstone=deleted]` statt eines Felds `deleted`, die alte Seite allein „Wiederhergestellt“; eine neue gelöschte Entität hat „Wiederherstellen“, eine gelöschte originale „Auf Original zurücksetzen…“ (der Dialog nennt „Grabstein“: „Gelöscht → wie in der Basis-ROM“).
- **MCP:** `plan_delete {target}` (lesend), `delete_entity {target, free_number?}` (schreibend, mit `free_number` unumkehrbar), `restore_entity {target}` (schreibend); Antwort `referenced` mit `details.references`; `delete_event` einer Tür antwortet `referenced` statt `error`. **`g4k`:** `g4k delete plan <projekt> <art> <index>`, `g4k delete apply <projekt> <art> <index> [--free-number] [--allow-irreversible]`, `g4k delete restore <projekt> <art> <index>`, `g4k text migrate <projekt>`.

## Vorbereitung

Wird beim Lauf ausgefüllt (Zeitpunkt, Stand, Last); die Anleitung dazu:

1. **Frische Kopien** unter `/Users/POLTEP/Projekte/lab/editor/Roms/_testkopien/` per `cp -cR` (Ordner) und `cp -c` (`.nds`) aus den Originalen (`ls /Users/POLTEP/Projekte/lab/editor/Roms`; die Originale werden nie geöffnet oder geändert); Ordner `3d-4-<zweck>_DSPRE_contents` und `.nds` `3d-4-<zweck>.nds` direkt nebeneinander:
   - `3d-4-pt` (Platin DE, CPUD), `3d-4-hgde` (HeartGold DE, IPKD), `3d-4-hgus` (HeartGold US, IPKE) für (1); die Tests schreiben nur auf `vanilla_copy()`;
   - `3d-4-kpt`, `3d-4-khgde`, `3d-4-khgus` für (2), je als Textprojekt mit Basis-ROM eingerichtet (siehe unten, `g4k base assign`, `g4k text adopt`, dann `g4k git init`, Commit „Ausgangsstand“, damit `git status` etwas sagt);
   - `3d-4-web` (Platin DE) und `3d-4-hgweb` (HeartGold DE) für (3) und MCP über die Brücke, ebenso eingerichtet (vor dem Öffnen in der Seite; oder in der App „Textprojekt einrichten“ in Technik › Textprojekt, dann `g4k git init` von außen);
   - `3d-4-tuer-pt` und `3d-4-tuer-hg` für `pt-tuer-loeschen.yaml` und `hg-tuer-loeschen.yaml`;
   - `3d-4-git-pt` und `3d-4-git-hgde` für den Branch-Test in `tests/git.rs` (er schreibt auf `vanilla_copy()`, braucht `git` und die `.nds` neben dem Ordner).

   **Einrichten als Textprojekt** (für (2) und (3), Kopien `3d-4-k*`, `3d-4-web`, `3d-4-hgweb`; ohne das lehnt `delete_entity` jeden Grabstein mit „Echtes Löschen gibt es im Textprojekt (Technik › Textprojekt).“ ab und `plan_delete` antwortet `base_missing`): `N=$K/3d-4-<zweck>.nds; P=$K/3d-4-<zweck>_DSPRE_contents; g4k base assign $P $N; g4k text adopt $P; g4k git init $P`; jeder `studio mcp`-Aufruf bekommt dazu `--base-rom $N`. Die Kopien `3d-4-tuer-*` der Szenarien brauchen das nicht: sie löschen nur Events über `delete_event` im DSPRE-Layout, ohne Grabstein und ohne Basis-ROM.

   Alle `3d-4-*`-Kopien nach dem Gate löschen (`Manual` und `_saves` bleiben).
2. **Werkzeuge:** `export PATH="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$HOME/.cargo/bin:$PATH"`; `cargo build --release -p g4-kernel --bin g4k` und `cargo build --release -p studio --bin studio` (einzeln, nie zwei Bauten gleichzeitig). `tools/` im Worktree ist ein Symlink auf den Haupt-Checkout (nie einchecken). **`G4_ROMS_DIR=/Users/POLTEP/Projekte/lab/editor/Roms` absolut** (der Standard `../Roms` existiert vom Worktree aus nicht).
3. **Test-Umgebung** für (3) nur auf `G4_VITE_PORT=1740 G4_BRIDGE_PORT=1741` (andere Ports nie anfassen): `G4_ROMS_DIR=<absolut> G4_VITE_PORT=1740 G4_BRIDGE_PORT=1741 bash scripts/agent/studio-web.sh > target/agent/studio-web.log 2>&1 & echo $! > target/agent/studio-web.pid`; Seite `http://127.0.0.1:1740/?project=<Roms>/_testkopien/3d-4-web_DSPRE_contents`; Chrome headless 1440 × 900, Hell und Dunkel über `prefers-color-scheme` in **einer** CDP-Verbindung mit dem Bild, Kompakt über die Taste der Statusleiste. Beenden: `kill $(cat target/agent/studio-web.pid)`, danach `lsof -iTCP:1740 -iTCP:1741 -sTCP:LISTEN` leer.
4. **Last:** `uptime` vor und nach jedem Lauf; die Maschine teilen bis zu vier Orchestratoren. Alle Zeiten sind darum „unter Last“ und mit der 1-Minuten-Last zu nennen; Zeitgrenzen gelten nur mit `G4_TIMING_STRICT=1`, sonst druckt der Test.
5. **Ausgaben** unter `target/agent/3d-4/` (und `target/agent/gate-<Datum>/3d-4/` für die Bilder) im Worktree, nicht im Repo; den Ordner vor `git worktree remove` sichern (auch `target/agent/gate-*`).

## (1) Automatik

- [ ] 1. **`scripts/gate.sh` grün:** wird in 3d.4 nicht gestartet. Die Hauptsitzung führt es einmal auf main nach dem Merge der Welle aus (`G4_ROMS_DIR=/Users/POLTEP/Projekte/lab/editor/Roms bash scripts/gate.sh > target/agent/gate-sh-3d-4.log 2>&1`, im Hintergrund, allein; unter paralleler Last scheitern seine Zeitgrenzen); die neuen gated Tests laufen dort im Schritt der gated Tests je Projekt mit. Stattdessen laufen die Schritte einzeln (2 bis 4).
- [ ] 2. **Schritte von `scripts/gate.sh`, die 3d.4 berührt**, einzeln aus dem Worktree:

  ```sh
  cargo fmt --check
  cargo clippy --workspace --all-targets --all-features -- -D warnings
  cargo test -p g4-kernel --all-features                 # ohne G4_*
  cargo test -p studio --all-features --lib              # ohne G4_*
  cargo test -p studio --test mcp_stdio                  # ohne G4_*
  (cd apps/studio && pnpm test && pnpm build)
  bash scripts/check-tokens.sh; bash scripts/check-styles.sh; bash scripts/check-depth.sh
  (cd scripts/g4emu && uv run pytest -m "not integration" -q)
  ```
- [ ] 3. **Gated Tests** je Projekt im Release-Profil, nacheinander (`K=$G4_ROMS_DIR/_testkopien`, `<k>` = `pt`, `hgde`, `hgus`; Last mit `uptime`):

  ```sh
  G4_TEST_PROJECT="$K/3d-4-<k>_DSPRE_contents" \
    cargo test --release -p g4-kernel --all-features \
      --test delete --test refs --test textproject --test textproject_migrate --test original \
      --test diff --test git --test vanilla_checks --test textproject_source \
      -- --nocapture --test-threads 1
  ```

  Zu belegen: `vanilla_world_edges_match_the_measured_counts` (Zählungen Header → Matrix / Matrix → Karte / Warp → Header: Platin 593 / 1 172 / 1 213, HeartGold 540 / 958 / 1 317, ±1 %; Script → Header Platin 67, HeartGold **114**, siehe „Hinweise“), `vanilla_index_builds_under_a_second` und die Index-Zeit (Platin und HeartGold, `G4_TIMING_STRICT=1` nur auf ruhiger Maschine), `textproject_round_trip_with_tombstone` und `textproject_export_idempotent_with_tombstone` (Round-Trip und Idempotenz mit Grabstein), die Migration 1 → 2 (`migration_1_to_2_changes_only_the_manifest_without_own_names`, `migration_writes_own_names_into_event_json_only`: nur Manifest, mit Namen nur Event-JSON; beide laufen auf einem synthetischen Projekt und sind nicht gated, sie kommen im selben Aufruf mit), `vanilla_checks` 0 Fehler (`vanilla_checks_match_the_fixed_counts`, `vanilla_checks_on_the_text_project_have_no_errors`), der Branch-Test `a_branch_without_the_freed_file_builds_after_git_switch` (`tests/git.rs`, nur auf Platin DE und HeartGold DE einzeln, braucht `git`).
- [ ] 4. **Studio-Test** über den Befehlsweg: `G4_TEST_PROJECT="$K/3d-4-<k>_DSPRE_contents" cargo test -p studio --all-features --lib delete_over_the_command_path -- --nocapture --test-threads 1` (`delete_over_the_command_path_on_the_vanilla_copy` in `apps/studio/src-tauri/src/commands/delete.rs`: eine neue Entität löschen, ⌘Z, Nummer freigeben; druckt die Zeit der Trainerliste mit Grabstein-Flaggen).

## (2) Szenario kopflos

Release-`studio mcp --project <Kopie> --base-rom <nds> [--allow-irreversible] [--expert]` über JSON-RPC/stdio (Protokoll 2025-11-25), je Spiel nacheinander auf `3d-4-k<k>` nach `g4k base assign`, `g4k text adopt` und `g4k git init` (siehe „Vorbereitung“); Ablauf und Befehle: [agent-gate.md](agent-gate.md), Abschnitt „Phase 3d.4“. Vor und nach jedem Schritt eine Prüfsumme über die Projektdateien außer `.git` und dem Zwischenspeicher `.g4/`.

- [ ] 1. **Je löschbarer Art eine neue Entität anlegen, löschen, bauen:** Trainer (`create_trainer`), Begegnungsdatei, Eventdatei, Matrix, Karte (über `recipe_apply create_room`); Header, Scriptdatei, Textarchiv, Gebietsdaten in Tüfteln bzw. über `g4_call`; `delete_entity` (Grabstein) → `g4k build` baut, `g4k check` 0 Fehler, `undo` → JSON byte-gleich (Prüfsumme vor und nach). Scriptdatei und Textarchiv haben keinen Grabstein (Satz oben): für sie zählt die Ablehnung und „Nummer freigeben“ (Punkt 4), ebenso für Texturen und Modelle.
- [ ] 2. **Verwiesene Entität → `referenced` mit der Liste** (je Zeile des Gates): Trainer an einer Figur; Begegnungsdatei an einem Header; Karte in einer Matrix; Header als Warp-Ziel und in einem `Warp`-Script (Platin und HeartGold DE); eine Tür, auf die ein Warp zeigt (`delete_event`).
- [ ] 3. **Originale → `refusal`:** Original-Header, -Scriptdatei, -Textarchiv mit dem Satz `ORIGINAL_KIND` bzw. dem Satz zu Scriptdateien und Textarchiven; ein gelöschter Original-Trainer kommt über `reset_to_original` zurück (ein Schritt, Plan nennt „Grabstein“).
- [ ] 4. **„Nummer freigeben“:** `delete_entity` mit `free_number` ohne Erlaubnis → `needs_confirmation` (MCP; `g4k delete apply … --free-number` ohne `--allow-irreversible` druckt auf stderr „error: delete_entity lässt sich nicht rückgängig machen (Nummer freigeben); kopflos nur mit --allow-irreversible“ und endet mit Exit 1, es ändert nichts); mit `--allow-irreversible` → `freed` mit Zahl und Dateien, `git status` zeigt die entfernte Datei (und bei Trainern die gekürzte Namensbank), `undo` hält an der Grenze. Ein Grabstein vor einem lebenden Eintrag bleibt („Nummer {n} bleibt belegt, weil {m} spätere Einträge leben.“).
- [ ] 5. **Branch-Test:** ein zweiter Branch ohne die Datei baut nach `git switch` (gated Test aus (1) 3; von Hand: Trainer anlegen, committen, auf `ohne` löschen und freigeben, committen, `git switch` hin und zurück, `g4k build`, `git status` leer).
- [ ] 6. **Szenarien mit `g4emu`:** `pt-tuer-loeschen.yaml` auf `3d-4-tuer-pt` und `hg-tuer-loeschen.yaml` auf `3d-4-tuer-hg` (`mcp: [--allow-irreversible, --expert]`, die Dateien wörtlich unverändert): Header-Folge 415, 412, 415 bzw. 64, 65, 64, `"ok": true`; B rückt auf A's Index, die Haustür zeigt nachgezogen auf B, beide Türen gehen.
- [ ] 7. **Migration 1 → 2:** `g4k text migrate` auf einer Kopie mit `schemaVersion` 1 (aus einem Commit vor 3d.4 exportiert): `g4k text migrate <Kopie> --json` (ohne `--json` druckt es nur die Zahl: `migrated 1 -> 2 changed=<n> snapshot=…`) nennt `changed == [g4-project.json]` ohne eigene Namen; mit `name_flag` vorher dazu die eine Eventdatei.
- [ ] 8. **`g4k delete`:** `plan`, `apply` (mit `--free-number` ohne `--allow-irreversible`: Exit 1 mit dem Satz aus Punkt 4, nichts geändert; mit beiden: `freed`) und `restore` auf einem neuen Trainer (auf der eingerichteten Kopie, mit der `.nds` daneben oder `--base-rom`); ein abgelehnter Plan endet mit Exit 1.
- [ ] 9. **Zeiten** `plan_delete`, `delete_entity` und `free_number` je Spiel in die Zahlentabelle (ms, unter Last).

## (3) Browser-Teil

Test-Umgebung auf 1740/1741, Kopien `3d-4-web` (Platin DE) und `3d-4-hgweb` (HeartGold DE), Chrome headless 1440 × 900 wie in „Vorbereitung“. Ein neuer Trainer, eine neue Begegnungsdatei und ein neuer Raum dienen als Prüflinge.

- [ ] 1. **Dialog mit Verweisen:** „Löschen…“ am Kopf eines Trainers, der an einer Figur hängt: Fehlermeldung „{n} Verweise: zuerst umhängen.“, die Liste `[data-reference]`, ein Klick führt zur Quelle, „Löschen“ gesperrt; ebenso Begegnungsdatei am Header, Karte in der Matrix (Kopf der Karte), Header als Warp-Ziel.
- [ ] 2. **Löschen und ⌘Z:** ein unverwiesener neuer Trainer: Dialog ohne Verweise, „Löschen“, Meldung „Gelöscht: …“, ein Undo-Schritt, Plakette „gelöscht“; ⌘Z holt ihn zurück (Plakette „neu“).
- [ ] 3. **„Gelöschte zeigen“ und „Wiederherstellen“:** `[data-show-deleted]` aus: Grabsteine fehlen; an: Zeile mit `data-deleted="true"` und Plakette; Kontextmenü „Wiederherstellen“ (ein Schritt „Wiederhergestellt: …“); die Picker zeigen den Grabstein nicht.
- [ ] 4. **Plakette im Inspector-Kopf:** „geändert“ nach einer Änderung und Speichern, „neu“ bei einer angelegten Entität, „gelöscht“ nach dem Löschen; die Plaketten kommen aus dem Hintergrundabgleich (ohne Technik › Änderungen geöffnet zu haben) und nie aus einem veralteten Bericht.
- [ ] 5. **Technik › Änderungen:** Gruppe mit Zeilen „Gelöscht“; Detail mit Plakette statt Feld `deleted`; „Auf Original zurücksetzen…“ eines **gelöschten Original-Trainers** stellt ihn her (der Dialog nennt „Grabstein“); „Wiederherstellen“ bei einer neuen gelöschten Entität.
- [ ] 6. **„Nummer freigeben“:** neue Entität am Ende, Kästchen „Nummer freigeben (nicht rückgängig)“ mit Plakette „Nicht rückgängig“ und der Dateiliste; danach `git -C <Kopie> status --porcelain` nennt die entfernte Datei; ⌘Z geht nicht dahinter zurück. Ein Original: `cannot_free` „Original-Einträge lassen sich nicht freigeben.“; Original-Header: abgelehnt mit `ORIGINAL_KIND`.
- [ ] 7. **Technik › Textprojekt › Löschen (Tüfteln):** Art „Karte“ (nach dem Leeren der Zellen), „Matrix“, „Eventdatei“, „Gebietsdaten“, „Scriptdatei“, „Textarchiv“ mit einer Nummer: der Dialog sagt, was geht; im DSPRE-Layout „Echtes Löschen gibt es im Textprojekt (Technik › Textprojekt).“.
- [ ] 8. **Warp löschen:** Tür löschen, auf die ein anderer Warp zeigt: abgelehnt mit der Zeile „Tür {i} von {Ort} springt hierher“; eine Tür in der Mitte ohne Verweise: spätere Türen rücken nach, die Warps darauf zeigen nachgezogen (`list_warps`).
- [ ] 9. **MCP über die Brücke** (Kopie `3d-4-web` in der Seite geöffnet, Agenten an, Freigabe „Lesen und bearbeiten“; die Kopie ist als Textprojekt mit Basis-ROM eingerichtet): `scripts/agent/mcp.sh call plan_delete '{"target":{"kind":"trainer","index":<n>}}'` (ohne Frage; `references`, `original`, `can_free`); `delete_entity` eines verwiesenen Trainers → `referenced` mit der Liste; `delete_entity` mit `free_number` → Agentenfrage `[data-agent-question]` „Nummer freigeben“, „Ablehnen“ → `needs_confirmation` „… in Gen4 Studio abgelehnt“, „Zulassen“ → `freed`.
- [ ] 10. **Konsole und Brückenlog:** kein `console.error`, keine Ausnahme, im Brückenlog kein Fehler und keine Panik.
- [ ] 11. **Blick** (Bilder, Hell, Dunkel, Kompakt): der Dialog in allen Zuständen (Verweise, Original-Warnung, „Nummer freigeben“, abgelehnt), die Plakette in Kopf und Liste, „Gelöschte zeigen“ (Task 10 Step 5; Bilder unter `target/agent/gate-<Datum>/3d-4/`).

Nicht im Browser prüfbar: die gepackte App, DSPRE.

## (4) Hand und Blick

Beim Nutzer (Abschnitte „Hand“ und „Blick“ unten und die Tabelle „Ergebnis“): der Dialog in Hell, Dunkel und Kompakt (Bilder aus Task 10 Step 5), die gepackte App und die **DSPRE-Hand-Probe „Entfernen“** (offene Frage der Entscheidungen; das Ergebnis trägt der Nutzer in diese Datei ein).

## Ergebnis

| Punkt | Wer | Ergebnis | Datum |
|---|---|---|---|
| (1) 1 `scripts/gate.sh` grün | Agent | offen: läuft einmal auf main nach dem Merge der Welle (Hauptsitzung), nicht aus dem Worktree | |
| (1) 2 Schritte von `gate.sh`, die 3d.4 berührt | Agent | | |
| (1) 3 gated Tests auf Platin DE | Agent | | |
| (1) 3 gated Tests auf HeartGold DE | Agent | | |
| (1) 3 gated Tests auf HeartGold US | Agent | | |
| (1) 4 Studio-Test `delete_over_the_command_path` | Agent | | |
| (2) 1 je Art anlegen, löschen, bauen, ⌘Z byte-gleich | Agent | | |
| (2) 2 verwiesene Entitäten → `referenced` | Agent | | |
| (2) 3 Originale → `refusal` | Agent | | |
| (2) 4 „Nummer freigeben“, `git status` | Agent | | |
| (2) 5 Branch-Test (`git switch`) | Agent | | |
| (2) 6 Szenarien `pt-tuer-loeschen`, `hg-tuer-loeschen` | Agent | | |
| (2) 7 Migration 1 → 2 | Agent | | |
| (2) 8 `g4k delete` | Agent | | |
| (2) 9 Zeiten | Agent | | |
| (3) 1 bis 8 Browser-Teil | Agent | | |
| (3) 9 MCP über die Brücke | Agent | | |
| (3) 10 Konsole und Brückenlog | Agent | | |
| (3) 11 Dialog und Plakette in Hell, Dunkel, Kompakt | Agent + Blick | | |
| (4) Dialog und Plakette in Hell, Dunkel, Kompakt | Blick | | |
| (4) gepackte App | Hand | | |
| (4) DSPRE-Hand-Probe „Entfernen“ | Hand | | |
| `.fig` | – | nicht angefasst, wenn OpenPencil nicht antwortet (Task 11: „die Frames folgen“) | |

Zahlentabelle (Release; Index und Welt-Kanten aus den gated Tests (1), `plan_delete`, `delete_entity` und `free_number` kopflos aus (2); alle Zeiten unter Last):

| Spiel | Index warm (ms) | Welt-Kanten (Header→Matrix / Matrix→Karte / Warp→Header / Script→Header) | `plan_delete` (ms) | `delete_entity` (ms) | `free_number` (ms) | Migration 1 → 2 geänderte Dateien | Last |
|---|---|---|---|---|---|---|---|
| Platin DE | | | | | | | |
| HeartGold DE | | | | | | | |
| HeartGold US | | | | | | | |

Gemessene Soll-Zahlen (Gutachten C §2.4, ±1 %): Platin DE 593 / 1 172 / 1 213 / 67, HeartGold DE 540 / 958 / 1 317 / 114.

## Befunde

### Gate-Lauf

| Nr. | Schwere | Punkt | Befund | Beleg | Vermutete Stelle | Fix |
|---|---|---|---|---|---|---|
| | | | | | | |

### UI-Test

| Nr. | Schwere | Punkt | Befund | Beleg | Vermutete Stelle | Fix |
|---|---|---|---|---|---|---|
| | | | | | | |

### Abschließendes Review des Branches

| Nr. | Schwere | Befund | Fix |
|---|---|---|---|
| | | | |

## Hinweise

Abweichungen der Umsetzung vom Plan, die beim Prüfen richtig vorkommen müssen:

- **H1, Script → Header in HeartGold DE: 114, nicht 154.** Das Gutachten zählte `MoveWarp <n>` als Header; der erste Parameter von HeartGolds `MoveWarp` ist aber `warp_id`, ein Warp-Index der eigenen Karte (`hgss_v2.json`). 154 = 114 `Warp` + 40 `MoveWarp`; `MoveWarp` ist in HGSS als `WarpIndex` indexiert (43 Indizes, drei weitere `MoveWarp` schreiben Warp 1 als `MAPTEMP_FLAG_BASE`, ein `Warp` nimmt eine Variable und zählt nicht). Platin bleibt bei 67 (`SCRIPT_HEADER_EDGES` in `tests/refs.rs`).
- **H2, Texturen und Modelle haben keinen Grabstein.** Karten- und Gebäudetexturen und die Gebäudemodelle (außen und innen) sind Binärarten ohne JSON, es gibt keinen Umschlag; sie sind `Truncate`: nur „Nummer freigeben“ neuer am Ende (Satz „Texturen und Modelle haben keinen Grabstein; …“), Originale abgelehnt. Der Plan sah „Grabstein nur bei neuen“ vor; das geht ohne Umschlag nicht.
- **H3, der Import kürzt über Git hinweg für Entitäten mit JSON und für Header, nicht für Texturen und Modelle.** Ein Member ohne Datei und ohne Basis bleibt bei Binärarten `textproject.member-without-json` (siehe „Offen“).
- **H4, die strenge Grenze des Index (1 000 ms) ließ sich unter Last nicht beurteilen** (siehe „Offen“). Der Zuwachs der Verweise liegt bei etwa 12 300 (Platin) und 11 300 (HeartGold), rund das Doppelte der Schätzung des Plans.
- **H5, die Szenarien `pt-tuer-loeschen` und `hg-tuer-loeschen` laufen mit `mcp: [--allow-irreversible, --expert]`:** die Ausgangsmatte braucht das Verhalten 0x65 über `set_permission`, das es nur über `g4_call` gibt (`--expert`). Warps feuern nur auf Warp-Kacheln.
- **H6, „Karte“ ist aus Tüfteln löschbar.** Der Kopf der Karte hat „Löschen…“, aber die Zelle der Matrix, in der die Karte gezeigt wird, ist ein harter Verweis: erst nach dem Leeren der Zellen geht das Löschen, dann über Technik › Textprojekt › Löschen › „Karte“.
- **H7, die Plaketten in den Köpfen kommen aus einem Hintergrundabgleich** (`changed_from_original`, einmal nach dem Öffnen und nach jedem Speichern, Zurücksetzen oder Löschen mit Dateischreiben), unabhängig davon, ob Technik › Änderungen gezeigt wird; ein Bericht, der älter ist als das letzte Speichern, zeigt keine Plakette „geändert“, bis der neue da ist. Ein Kopf für Matrix, Eventdatei und Gebietsdaten hat keine Plakette; sie zeigen ihren Zustand in Technik › Änderungen.
- **H8, ein neuer Warp mit dem Standardziel (0, 0) zählt als Verweis** und sperrt das Löschen der Tür 0 der Eventdatei von Header 0; ein Warp in einer Eventdatei mit Grabstein sperrt das Löschen der Tür, auf die er zeigt, weiter (ein späteres „Wiederherstellen“ lenkt nicht still um).
- **H9, DSPRE-Layout:** ein Grabstein wird abgelehnt („Echtes Löschen gibt es im Textprojekt (Technik › Textprojekt).“); „Nummer freigeben“ geht für neue Entitäten am Ende (unumkehrbar, wie das Anlegen).
- **H10, nicht in 3d.4:** Nachrücken von Nummern, Wiederverwenden von Grabstein-Nummern, Grabsteine im DSPRE-Layout, Löschen von Spezies, Formen, Attacken, Items, Fähigkeiten, Typen, Trainerklassen und von Original-Headern, -Scriptdateien, -Textarchiven, -Gebietsdaten, -Texturen, -Modellen, `script.trainer-target`, Geschichten als Quelle (3d.6 trägt sie in `SOFT_SOURCES` ein), ein Befehl „Leeren“.
- **H11, Zwischenstände:** `.g4/build/state.json` und Modelle unter `.g4/models/` ändern sich beim Arbeiten (von Git ignoriert); sie zählen nicht zu „alle Dateien“ bei den Prüfsummen.

## Hand

- [ ] **Gepackte App:** `pnpm bundle`, App aus dem Finder starten, ein Textprojekt öffnen, eine neue Entität anlegen, „Löschen…“, ⌘Z, „Gelöschte zeigen“, „Wiederherstellen“; die Plakette im Kopf; die App schließen und öffnen: der Grabstein bleibt (Datei `data/trainers/NNNN.json` trägt `"deleted": true`).
- [ ] **DSPRE-Hand-Probe „Entfernen“** (offene Frage der Entscheidungen): ein Textprojekt mit einer freigegebenen Nummer am Ende und eines mit einem Grabstein in DSPRE öffnen bzw. dort den Eintrag „Entfernen“ benutzen und prüfen, was DSPRE mit einer fehlenden Nummer am Ende, mit dem fehlenden Member und mit einem Eintrag tut, dessen JSON `deleted: true` trägt; das Ergebnis hier eintragen:

  _(vom Nutzer auszufüllen)_

## Blick

Bilder unter `target/agent/gate-<Datum>/3d-4/` im Worktree, 1440 × 900, aus der Test-Umgebung über Chrome headless (Hell und Dunkel über `prefers-color-scheme` in einer CDP-Verbindung mit dem Bild, Kompakt über die Taste der Statusleiste); Dateinamen werden beim Lauf eingetragen.

- [ ] Dialog „Löschen…“ in Hell, Dunkel, Kompakt: Verweise (Fehlermeldung und Liste), Original-Warnung, „Nummer freigeben“ mit der Plakette „Nicht rückgängig“, abgelehnt (nur „Schließen“).
- [ ] Plakette `geändert`, `neu`, `gelöscht` im Inspector-Kopf von Trainer und Begegnungsdatei und im Kopf eines Gebiets, in Hell, Dunkel, Kompakt.
- [ ] „Gelöschte zeigen“ in den Listen Trainer, Begegnungen und Welt › Orte: Zeile mit Plakette „gelöscht“, Kontextmenü „Wiederherstellen“.
- [ ] Technik › Änderungen mit der Gruppe „Gelöscht“ und der Plakette im Detail.

## Offen

- **(1) 1 `scripts/gate.sh`:** einmal auf main nach dem Merge der Welle (Hauptsitzung).
- **Strenge Index-Grenze (1 000 ms):** die Zeit des Index mit den Welt-Kanten ließ sich unter einer Last von 18 bis 36 nicht beurteilen (ein strenger Lauf maß 2,17 s bei Last 33). Auf einer ruhigen Maschine mit `G4_TIMING_STRICT=1` (`vanilla_index_builds_under_a_second`) nachmessen; hält sie nicht, ist es ein eigener Leistungs-Task (zum Beispiel nur den Gebäude-Teil der Karten lesen, siehe „Map-Quelle im Index“ im Plan).
- **Import über Git hinweg für Texturen und Modelle:** die Kürz-Regel deckt Entitäten mit JSON und Header ab, nicht die Binärarten (Karten- und Gebäudetexturen, Gebäudemodelle): ein zweiter Branch ohne ein freigegebenes neues Modell meldet dort `textproject.member-without-json`. Eigener Posten.
- **Hand-Probe „Entfernen“ in DSPRE** (Abschnitt „Hand“).
- **`.fig`:** die Frames für Dialog, Plakette und Grabstein-Zustand sind nicht eingespielt, wenn OpenPencil nicht antwortet (`design-system/README.md`, „Nachtrag 3d.4“: die Frames folgen).
- **Merge-Hinweise** (aus dem Ledger): Konflikte an den Anhängepunkten mit 3d.5 und 3d.6 (`data/refs.rs` `Target` und `MEMBER_SOURCES`, `arts.rs`, `kinds.rs`, `diff/subject.rs`, `InspectorHead.tsx`, `api.ts`, `App.tsx`, `CmdError`-Literale in `src-tauri`, `ROADMAP.md`, `agent-gate.md`, `mcp.md`); `.fig` von main behalten und neu rendern.
