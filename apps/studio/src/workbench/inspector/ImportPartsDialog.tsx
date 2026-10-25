import { useEffect, useMemo, useRef, useState } from "react";
import { errorText, importMapParts, inspectMapImport } from "../../api";
import type { CheckProblemDto, ImportSource, MapPart, PartReport } from "../../api";
import type { ChecksApi } from "../../checks/useChecks";
import { openFiles } from "../../platform/dialogs";
import { inTauri } from "../../platform/transport";
import {
  Button,
  CheckList,
  Checkbox,
  Dialog,
  Notice,
  Picker,
  Segmented,
  Switch,
  TextField,
  checkNumber,
} from "../../ui/primitives";
import type { CheckRow, PickerItem } from "../../ui/primitives";
import type { NavTarget } from "../navigation";
import { findingRow } from "../problems/ProblemsPanel";
import { id4 } from "../terms";
import {
  PART_NAME,
  isForeignWhole,
  partText,
  partView,
  partsWord,
  preselected,
} from "./importParts";
import styles from "./ImportPartsDialog.module.css";

type Mode = "files" | "map";

export interface ImportPartsDialogProps {
  /** The map the parts go into. */
  map: number;
  open: boolean;
  onClose: () => void;
  /** Number of maps in the project (the maps the picker offers). */
  mapCount: number;
  /** An import changed the map: the app refreshes pictures, scene and the dirty count (and announces the edit, so the checks run). */
  onImported: (map: number) => void | Promise<void>;
  /** The checks of the project: after an import the dialog lists what their next run finds at the map. None: it lists nothing. */
  checks?: ChecksApi | null;
  /** „Zeigen“ of such a finding (the dialog closes first). */
  goTo?: (target: NavTarget) => void;
}

/** The findings of the map after an import: the next run is awaited, gave no report, or found these. */
type Found = "waiting" | "unchecked" | readonly CheckProblemDto[];

const MODES = [
  { value: "files", label: "Datei" },
  { value: "map", label: "Karte im Projekt" },
] as const;

const fileName = (path: string) => path.split(/[\\/]/).pop() ?? path;

/**
 * „Teile importieren…“: parts from files or from another map of the project are checked first (`inspect_map_import`), each one with its
 * size and state in German; the chosen parts are written as one undo step (`import_map_parts`). A part with an error cannot be chosen.
 * After an import the dialog waits for the next report of the checks (the import is an edit; the full run follows its pause) and lists
 * the findings about the map, each with „Zeigen“ where it leads somewhere (spec 3b.4 §9).
 */
export function ImportPartsDialog({
  map,
  open,
  onClose,
  mapCount,
  onImported,
  checks = null,
  goTo,
}: ImportPartsDialogProps) {
  const [mode, setMode] = useState<Mode>("map");
  const [paths, setPaths] = useState("");
  const [sourceText, setSourceText] = useState("");
  const [sourceError, setSourceError] = useState<string | undefined>();
  const [pickerOpen, setPickerOpen] = useState(false);
  const [reports, setReports] = useState<PartReport[] | null>(null);
  const [failure, setFailure] = useState<string | null>(null);
  const [chosen, setChosen] = useState<ReadonlySet<MapPart>>(new Set());
  const [keepBuildings, setKeepBuildings] = useState(false);
  const [working, setWorking] = useState(false);
  const [done, setDone] = useState<number | null>(null);
  const [found, setFound] = useState<Found | null>(null);
  // Counts the openings and the sources: a report that arrives after the dialog was opened again, or after the source changed, belongs
  // to the earlier import.
  const opening = useRef(0);

  // Every opening starts afresh.
  useEffect(() => {
    if (!open) return;
    opening.current++;
    setFound(null);
    setMode("map");
    setPaths("");
    setSourceText("");
    setSourceError(undefined);
    setReports(null);
    setFailure(null);
    setChosen(new Set());
    setKeepBuildings(false);
    setDone(null);
  }, [open, map]);

  const source = useMemo<ImportSource | null>(() => {
    if (mode === "files") {
      const files = paths
        .split("\n")
        .map((l) => l.trim())
        .filter((l) => l !== "");
      return files.length > 0 ? { files } : null;
    }
    const checked = checkNumber(
      sourceText,
      "dec",
      0,
      Math.max(0, mapCount - 1),
    );
    return sourceText.trim() !== "" && checked.ok
      ? { map: checked.value }
      : null;
  }, [mode, paths, sourceText, mapCount]);
  const sourceKey = source === null ? null : JSON.stringify(source);

  // The check runs whenever the source changes; an answer for an older source is dropped, and so are the findings of an import from it.
  useEffect(() => {
    opening.current++;
    setReports(null);
    setFailure(null);
    setChosen(new Set());
    setDone(null);
    setFound(null);
    if (!open || source === null) return;
    let current = true;
    inspectMapImport(map, source).then(
      (list) => {
        if (!current) return;
        setReports(list);
        setChosen(new Set(list.filter(preselected).map((r) => r.part)));
      },
      (e) => current && setFailure(errorText(e)),
    );
    return () => {
      current = false;
    };
  }, [open, map, sourceKey]);

  const foreignWhole =
    reports?.some((r) => isForeignWhole(r) && chosen.has(r.part)) ?? false;
  const count = chosen.size;

  const toggle = (part: MapPart, on: boolean) =>
    setChosen((prev) => {
      const next = new Set(prev);
      if (on) next.add(part);
      else next.delete(part);
      return next;
    });

  async function run() {
    if (source === null || count === 0) return;
    setWorking(true);
    setFailure(null);
    try {
      const parts = (reports ?? [])
        .map((r) => r.part)
        .filter((p) => chosen.has(p));
      const changed = await importMapParts(
        map,
        source,
        parts,
        foreignWhole && keepBuildings,
      );
      setDone(changed.length > 0 ? parts.length : 0);
      if (changed.length > 0) {
        // Asked before the import's edit is announced (`onImported`): the run after its pause is the one that counts.
        const next = checks?.nextReport();
        if (next) {
          const at = opening.current;
          setFound("waiting");
          void next.then((findings) => {
            if (opening.current !== at) return;
            setFound(findings === null ? "unchecked" : findings.filter((f) => f.subject.kind === "map" && f.subject.index === map));
          });
        }
        await onImported(map);
      }
    } catch (e) {
      setFailure(errorText(e));
    } finally {
      setWorking(false);
    }
  }

  const rows: CheckRow[] = (reports ?? []).map((r) => {
    const view = partView(r);
    return {
      id: r.part,
      state: view.state,
      text: (
        <div className={styles.part}>
          <Checkbox
            label={`${PART_NAME[r.part]} · ${r.size.toLocaleString("de-DE")} Bytes${r.file ? ` · ${fileName(r.file)}` : ""}`}
            ariaLabel={PART_NAME[r.part]}
            checked={chosen.has(r.part)}
            disabled={r.state.kind === "error" || done !== null}
            onChange={(on) => toggle(r.part, on)}
          />
          {view.message !== null && (
            <p className={styles.reason}>{view.message}</p>
          )}
          {r.notes.map((n) => (
            <p key={`${n.code}:${n.args.join(",")}`} className={styles.reason}>
              {partText(n)}
            </p>
          ))}
        </div>
      ),
    };
  });

  const mapItems = useMemo<PickerItem[]>(
    () =>
      pickerOpen
        ? Array.from({ length: mapCount }, (_, i) => ({
            id: i,
            label: `Karte ${id4(i)}`,
            tech: String(i),
            area: "welt" as const,
            disabled: i === map,
          }))
        : [],
    [pickerOpen, mapCount, map],
  );

  return (
    <Dialog
      open={open}
      onOpenChange={(next) => !next && onClose()}
      title={`Teile in Karte ${id4(map)} importieren`}
    >
      <div className={styles.body} data-import-dialog={map}>
        <Segmented
          ariaLabel="Quelle"
          options={MODES}
          value={mode}
          onChange={setMode}
          size="s"
        />
        {mode === "files" ? (
          <div className={styles.source}>
            <TextField
              label="Pfade, eine je Zeile"
              showLabel
              multiline
              value={paths}
              onCommit={setPaths}
              placeholder="/Pfad/zur/karte.nsbmd"
            />
            {inTauri() && (
              <Button
                kind="secondary"
                size="s"
                onClick={() =>
                  void openFiles({ title: "Teile wählen" }).then((files) => {
                    if (files && files.length > 0) setPaths(files.join("\n"));
                  })
                }
              >
                Dateien wählen…
              </Button>
            )}
          </div>
        ) : (
          <div className={styles.source}>
            <TextField
              label="Quellkarte"
              showLabel
              placeholder="Nummer"
              value={sourceText}
              error={sourceError}
              onChange={() => setSourceError(undefined)}
              onCommit={(text) => {
                const checked = checkNumber(
                  text,
                  "dec",
                  0,
                  Math.max(0, mapCount - 1),
                );
                setSourceError(
                  text.trim() === "" || checked.ok ? undefined : checked.error,
                );
                setSourceText(text);
              }}
            />
            <Button
              kind="secondary"
              size="s"
              onClick={() => setPickerOpen(true)}
            >
              Karte wählen…
            </Button>
            <Picker
              open={pickerOpen}
              onOpenChange={setPickerOpen}
              title="Quellkarte wählen"
              items={mapItems}
              placeholder="Nummer suchen"
              onPick={(item) => setSourceText(String(item.id))}
            />
          </div>
        )}
        {source !== null && reports === null && failure === null && (
          <p className={styles.reason}>Teile werden geprüft…</p>
        )}
        {reports !== null && reports.length === 0 && (
          <p className={styles.reason}>Die Quelle enthält keine Teile.</p>
        )}
        {rows.length > 0 && <CheckList ariaLabel="Teile" rows={rows} />}
        {foreignWhole && done === null && (
          <Switch
            label="Gebäude übernehmen"
            checked={keepBuildings}
            onChange={setKeepBuildings}
          />
        )}
        {failure !== null && <Notice kind="error">{failure}</Notice>}
        {done !== null && (
          <Notice
            kind={done > 0 ? "success" : "info"}
            actions={[{ label: "Schließen", onClick: onClose }]}
          >
            {done > 0
              ? `${partsWord(done)} übernommen`
              : "Nichts geändert: die Teile waren schon so."}
          </Notice>
        )}
        {found === "waiting" && (
          <p role="status" className={styles.reason}>
            Prüft die Karte …
          </p>
        )}
        {found === "unchecked" && <p className={styles.reason}>Die Prüfungen haben kein Ergebnis geliefert. Starte sie unter Probleme mit „Erneut prüfen“.</p>}
        {Array.isArray(found) && found.length === 0 && <p className={styles.reason}>Keine neuen Befunde an dieser Karte.</p>}
        {Array.isArray(found) && found.length > 0 && (
          <CheckList
            ariaLabel={`Befunde an Karte ${id4(map)}`}
            rows={found.map((f) => {
              const target = f.target;
              return findingRow(
                f,
                "bauen",
                target !== undefined && goTo !== undefined ? (
                  <span className={styles.findingActions}>
                    <Button
                      kind="quiet"
                      size="s"
                      onClick={() => {
                        onClose();
                        goTo(target);
                      }}
                    >
                      Zeigen
                    </Button>
                  </span>
                ) : undefined,
              );
            })}
          />
        )}
        {done === null && (
          <div className={styles.actions}>
            <Button kind="secondary" onClick={onClose}>
              Abbrechen
            </Button>
            <Button
              kind="primary"
              disabled={count === 0 || working || source === null}
              onClick={() => void run()}
            >
              {`${partsWord(count)} übernehmen`}
            </Button>
          </div>
        )}
      </div>
    </Dialog>
  );
}
