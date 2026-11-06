import { useEffect, useMemo, useRef, useState } from "react";
import { errorText, importMapParts, inspectMapImport } from "../../api";
import type { CheckProblemDto, ImportOptions, ImportSource, MapPart, PartReport } from "../../api";
import type { ChecksApi } from "../../checks/useChecks";
import { openFiles } from "../../platform/dialogs";
import { inTauri } from "../../platform/transport";
import {
  Budget,
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
import type { CheckRow } from "../../ui/primitives";
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
  previewLines,
  sourceMapItems,
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
  /** Tüfteln: the model budget names its limit in hex („0xF000“). */
  tinker?: boolean;
}

/** The findings of the map after an import: the next run is awaited, gave no report, or found these. */
type Found = "waiting" | "unchecked" | readonly CheckProblemDto[];

const MODES = [
  { value: "files", label: "Datei" },
  { value: "map", label: "Karte im Projekt" },
] as const;

const fileName = (path: string) => path.split(/[\\/]/).pop() ?? path;

/** The file types the system dialog offers: map parts, models and images (the extension decides the part). */
const PART_FILES = [
  {
    name: "Kartenteile und Modelle",
    extensions: ["nsbmd", "bmd", "per", "mp", "bld", "bdhc", "bgs", "bin", "obj", "gltf", "glb", "png"],
  },
];

const extension = (path: string) => /\.([^.\\/]+)$/.exec(path)?.[1]?.toLowerCase() ?? "";
const MODEL_EXTENSIONS: ReadonlySet<string> = new Set(["obj", "gltf", "glb"]);

const COLOURS = [
  { value: "16", label: "16" },
  { value: "4", label: "4" },
] as const;
type Colours = (typeof COLOURS)[number]["value"];

/** The default unit: one model unit is one tile (16 world units). */
const UNIT_DEFAULT = 16;
const UNIT_ERROR = "Zwischen 0,0625 und 256.";

/** A unit as typed („16“, „0,5“, „0.5“), or null when it is no number in 0.0625 to 256 (`ImportOptions::check`). */
function parseUnit(text: string): number | null {
  const t = text.trim().replace(",", ".");
  if (!/^(\d+(\.\d*)?|\.\d+)$/.test(t)) return null;
  const n = Number(t);
  return Number.isFinite(n) && n >= 0.0625 && n <= 256 ? n : null;
}

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
  tinker = false,
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
  const [colours, setColours] = useState<Colours>("16");
  const [unit, setUnit] = useState(UNIT_DEFAULT);
  const [unitText, setUnitText] = useState(String(UNIT_DEFAULT));
  const [unitError, setUnitError] = useState<string | undefined>();
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
    setColours("16");
    setUnit(UNIT_DEFAULT);
    setUnitText(String(UNIT_DEFAULT));
    setUnitError(undefined);
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

  // Models and images are converted: the row „Umwandeln“ offers the colours per texture and, for a model, the unit; the check and the
  // import get them as `options` (other parts go without).
  const extensions = source !== null && "files" in source ? source.files.map(extension) : [];
  const hasModel = extensions.some((e) => MODEL_EXTENSIONS.has(e));
  const converting = hasModel || extensions.includes("png");
  const options: ImportOptions | undefined = converting
    ? hasModel
      ? { unit, colors: Number(colours) as 4 | 16 }
      : { colors: Number(colours) as 4 | 16 }
    : undefined;
  const optionsKey = options === undefined ? null : JSON.stringify(options);

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
    (options === undefined ? inspectMapImport(map, source) : inspectMapImport(map, source, options)).then(
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
  }, [open, map, sourceKey, optionsKey]);

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
      const keep = foreignWhole && keepBuildings;
      const changed = await (options === undefined
        ? importMapParts(map, source, parts, keep)
        : importMapParts(map, source, parts, keep, options));
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
          {r.preview !== undefined && (
            <div className={styles.preview} data-import-preview={r.part}>
              {previewLines(r).map((line, i) => (
                <p key={`${i}:${line}`} className={styles.reason}>
                  {line}
                </p>
              ))}
              {r.preview.model !== null && (
                <Budget
                  label="Modell"
                  used={r.preview.model.budget.used}
                  max={r.preview.model.budget.max}
                  state={r.preview.model.budget.state}
                  unit="B"
                  tech={tinker ? `0x${r.preview.model.budget.max.toString(16).toUpperCase()}` : undefined}
                />
              )}
            </div>
          )}
        </div>
      ),
    };
  });

  // Built once per map, not per opening: emptying the list while the picker fades out showed „Nichts gefunden“ (GB7).
  const mapItems = useMemo(() => sourceMapItems(mapCount, map), [mapCount, map]);

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
                  void openFiles({ title: "Teile oder Modelle wählen", filters: PART_FILES }).then((files) => {
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
        {mode === "files" && converting && (
          <div className={styles.convert} data-import-convert="">
            <span className={styles.convertTitle}>Umwandeln</span>
            <div className={styles.convertOptions}>
              <div className={styles.option}>
                <span className={styles.optionLabel}>Farben je Textur</span>
                <Segmented ariaLabel="Farben je Textur" options={COLOURS} value={colours} onChange={setColours} size="s" />
              </div>
              {hasModel && (
                <TextField
                  label="Einheit"
                  showLabel
                  value={unitText}
                  hint="Welteinheiten je Modelleinheit; 16 = eine Kachel"
                  error={unitError}
                  onChange={() => setUnitError(undefined)}
                  onCommit={(text) => {
                    setUnitText(text);
                    const parsed = parseUnit(text);
                    if (parsed === null) setUnitError(UNIT_ERROR);
                    else {
                      setUnitError(undefined);
                      setUnit(parsed);
                    }
                  }}
                />
              )}
            </div>
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
