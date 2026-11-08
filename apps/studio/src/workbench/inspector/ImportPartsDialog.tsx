import { useEffect, useMemo, useRef, useState } from "react";
import type { ReactNode } from "react";
import { errorText, importMapParts, inspectMapImport, worldFreeformAdd, worldFreeformInspect, worldGet, worldImportTmj, worldInspectTmj } from "../../api";
import type { CheckProblemDto, FreeformInspect, ImportOptions, ImportSource, MapPart, PartReport, TmjImportReport } from "../../api";
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
  modelBudget,
  partText,
  partView,
  partsWord,
  preselected,
  previewLines,
  sourceMapItems,
  textureLines,
  tmjChangeText,
  tmjErrorLines,
  worldText,
} from "./importParts";
import styles from "./ImportPartsDialog.module.css";

type Mode = "files" | "map";

/** What a world map takes from files instead of map parts (6.5b): a free-form model and a Tiled map. */
type WorldPart = "freeform" | "tmj";
type Choice = MapPart | WorldPart;

/** The check of a world part's file: running, refused by the command, two files for the part, or the answer. */
type WorldRead<T> = "reading" | "duplicate" | { failure: string } | { report: T };

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

/** A world map also takes Tiled maps. */
const WORLD_FILES = [{ ...PART_FILES[0]!, extensions: [...PART_FILES[0]!.extensions, "tmj"] }];

const WORLD_NAME: Record<WorldPart, string> = { freeform: "Freiform (glTF)", tmj: "Tiled (TMJ)" };
const WORLD_ONLY_BAKES = "Bei Welt-Karten entstehen Modell, Terrain und Permissions beim Backen.";
const AT_AXES = ["x", "y", "z"] as const;
/** The bounds of a coordinate of „Lage“ in world units. */
const AT_MIN = -32768;
const AT_MAX = 32767;

const extension = (path: string) => /\.([^.\\/]+)$/.exec(path)?.[1]?.toLowerCase() ?? "";
const MODEL_EXTENSIONS: ReadonlySet<string> = new Set(["obj", "gltf", "glb"]);

const COLOURS = [
  { value: "16", label: "16" },
  { value: "4", label: "4" },
] as const;
type Colours = (typeof COLOURS)[number]["value"];

/** The files of `paths` with the given extensions. */
const withExtension = (paths: readonly string[], set: ReadonlySet<string>) => paths.filter((p) => set.has(extension(p)));
const TMJ_EXTENSIONS: ReadonlySet<string> = new Set(["tmj"]);

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
 *
 * A world map (6.5b; the dialog asks `world_get` when it opens) takes no map parts, they come from the bake: a model file (`.obj`, `.gltf`,
 * `.glb`) becomes the row „Freiform (glTF)“ with its place „Lage“ (`world_freeform_inspect`, `world_freeform_add`), a `.tmj` the row
 * „Tiled (TMJ)“ (`world_inspect_tmj`, `world_import_tmj`); other parts show, but cannot be chosen.
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
  const [chosen, setChosen] = useState<ReadonlySet<Choice>>(new Set());
  // Whether the map is a world map, for the map it belongs to (null: still asking).
  const [worldOf, setWorldOf] = useState<{ map: number; world: boolean } | null>(null);
  const [freeform, setFreeform] = useState<WorldRead<FreeformInspect> | null>(null);
  const [tmj, setTmj] = useState<WorldRead<TmjImportReport> | null>(null);
  const [at, setAt] = useState<[number, number, number]>([0, 0, 0]);
  const [atText, setAtText] = useState<[string, string, string]>(["0", "0", "0"]);
  const [atError, setAtError] = useState<[string?, string?, string?]>([]);
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
    setAt([0, 0, 0]);
    setAtText(["0", "0", "0"]);
    setAtError([]);
  }, [open, map]);

  useEffect(() => {
    if (!open) return;
    let current = true;
    worldGet(map).then(
      (dto) => current && setWorldOf({ map, world: dto.world !== null }),
      // Outside a text project (or when the map does not read) there is no world: the parts as ever.
      () => current && setWorldOf({ map, world: false }),
    );
    return () => {
      current = false;
    };
  }, [open, map]);
  const world = worldOf?.map === map ? worldOf.world : null;

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

  // A world map: model files and TMJ go to their own rows, the other files (or another map) are listed as parts that cannot be chosen.
  const files = source !== null && "files" in source ? source.files : [];
  const freeformFiles = world === true ? withExtension(files, MODEL_EXTENSIONS) : [];
  const tmjFiles = world === true ? withExtension(files, TMJ_EXTENSIONS) : [];
  const partSource: ImportSource | null =
    world !== true || source === null || !("files" in source)
      ? source
      : (() => {
          const rest = files.filter((f) => !MODEL_EXTENSIONS.has(extension(f)) && !TMJ_EXTENSIONS.has(extension(f)));
          return rest.length > 0 ? { files: rest } : null;
        })();
  const sourceKey = world === null || partSource === null ? null : JSON.stringify(partSource);

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
  // The parts of a world map cannot be chosen: they are checked without converting.
  const partOptions = world === true ? undefined : options;
  const partOptionsKey = partOptions === undefined ? null : JSON.stringify(partOptions);
  const freeformPath = freeformFiles.length === 1 ? freeformFiles[0]! : null;
  const tmjPath = tmjFiles.length === 1 ? tmjFiles[0]! : null;
  const freeformKey = freeformFiles.length === 0 ? null : JSON.stringify([freeformFiles, at, options]);
  const tmjKey = tmjFiles.length === 0 ? null : JSON.stringify(tmjFiles);

  // The check runs whenever the source changes; an answer for an older source is dropped, and so are the findings of an import from it.
  useEffect(() => {
    opening.current++;
    setReports(null);
    setFailure(null);
    setChosen((prev) => keepWorld(prev));
    setDone(null);
    setFound(null);
    if (!open || partSource === null || world === null) return;
    let current = true;
    (partOptions === undefined ? inspectMapImport(map, partSource) : inspectMapImport(map, partSource, partOptions)).then(
      (list) => {
        if (!current) return;
        setReports(list);
        const parts = world ? [] : list.filter(preselected).map((r) => r.part);
        setChosen((prev) => new Set([...keepWorld(prev), ...parts]));
      },
      (e) => current && setFailure(errorText(e)),
    );
    return () => {
      current = false;
    };
  }, [open, map, sourceKey, partOptionsKey, world]);

  // The free-form model of a world map is read again whenever its file, its place or the options change.
  useEffect(
    () =>
      readWorld("freeform", freeformKey, freeformFiles.length, setFreeform, () => worldFreeformInspect({ map, path: freeformPath!, at, options }), (r) => r.ok && (r.report?.bake?.ok ?? true)),
    [open, map, freeformKey],
  );
  useEffect(
    () => readWorld("tmj", tmjKey, tmjFiles.length, setTmj, () => worldInspectTmj(map, tmjPath!), (r) => r.errors.length === 0 && r.bake !== null),
    [open, map, tmjKey],
  );

  /**
   * Checks the file of world part `part` (`key` null: none): two files are one too many, else `inspect` asks the command; an answer that
   * `ready` accepts chooses the part. An answer for an older key is dropped.
   */
  function readWorld<T>(
    part: WorldPart,
    key: string | null,
    count: number,
    set: (read: WorldRead<T> | null) => void,
    inspect: () => Promise<T>,
    ready: (report: T) => boolean,
  ): (() => void) | undefined {
    opening.current++;
    setDone(null);
    setFound(null);
    setChosen((prev) => without(prev, part));
    if (!open || key === null) {
      set(null);
      return;
    }
    if (count > 1) {
      set("duplicate");
      return;
    }
    set("reading");
    let current = true;
    inspect().then(
      (report) => {
        if (!current) return;
        set({ report });
        if (ready(report)) setChosen((prev) => new Set([...prev, part]));
      },
      (e) => current && set({ failure: errorText(e) }),
    );
    return () => {
      current = false;
    };
  }

  const foreignWhole =
    reports?.some((r) => isForeignWhole(r) && chosen.has(r.part)) ?? false;
  const count = chosen.size;

  const toggle = (part: Choice, on: boolean) =>
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
      let taken = 0;
      if (parts.length > 0 && partSource !== null) {
        const changed = await (partOptions === undefined
          ? importMapParts(map, partSource, parts, keep)
          : importMapParts(map, partSource, parts, keep, partOptions));
        if (changed.length > 0) taken += parts.length;
      }
      if (chosen.has("freeform") && freeformPath !== null) {
        await worldFreeformAdd({ map, path: freeformPath, at, options });
        taken++;
      }
      if (chosen.has("tmj") && tmjPath !== null) {
        const imported = await worldImportTmj(map, tmjPath);
        if (imported.change !== null) taken++;
      }
      setDone(taken);
      if (taken > 0) {
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

  const partRows: CheckRow[] = (reports ?? []).map((r) => {
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
            disabled={r.state.kind === "error" || done !== null || world === true}
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

  /** The row of world part `part`: its name and file, its state with the sentences under it and, for a model, its preview. */
  function worldRow<T>(part: WorldPart, read: WorldRead<T>, file: string | null, view: (report: T) => WorldView): CheckRow {
    const shown: WorldView =
      read === "reading"
        ? { state: "info", lines: ["Wird geprüft …"], ready: false }
        : read === "duplicate"
          ? { state: "error", lines: [partText({ code: "part.duplicate", args: [], message: "" })], ready: false }
          : "failure" in read
            ? { state: "error", lines: [read.failure], ready: false }
            : view(read.report);
    return {
      id: part,
      state: shown.state,
      text: (
        <div className={styles.part} data-import-part={part}>
          <Checkbox
            label={`${WORLD_NAME[part]}${file !== null ? ` · ${fileName(file)}` : ""}`}
            ariaLabel={WORLD_NAME[part]}
            checked={chosen.has(part)}
            disabled={!shown.ready || done !== null}
            onChange={(on) => toggle(part, on)}
          />
          {shown.lines.map((line, i) => (
            <p key={`${i}:${line}`} className={styles.reason}>
              {line}
            </p>
          ))}
          {shown.preview}
        </div>
      ),
    };
  }

  const freeformView = (r: FreeformInspect): WorldView => {
    if (!r.ok || r.report === null) {
      const lines = r.notes.map(worldText);
      return { state: "error", lines: r.refusal !== null ? [worldText(r.refusal), ...lines] : lines, ready: false };
    }
    const { report } = r;
    const refused = report.bake?.messages.find((m) => m.level === "error");
    const budget = report.bake !== null ? modelBudget(report.bake.model_bytes) : null;
    return {
      state: refused !== undefined || report.bake?.ok === false ? "error" : "ok",
      lines: [...(refused !== undefined ? [worldText(refused)] : []), ...report.notes.map(worldText)],
      ready: refused === undefined && report.bake?.ok !== false,
      preview: (
        <div className={styles.preview} data-import-preview="freeform">
          {textureLines(report.textures).map((line, i) => (
            <p key={`${i}:${line}`} className={styles.reason}>
              {line}
            </p>
          ))}
          {budget !== null && (
            <Budget label="Modell" used={budget.used} max={budget.max} state={budget.state} unit="B" tech={tinker ? `0x${budget.max.toString(16).toUpperCase()}` : undefined} />
          )}
        </div>
      ),
    };
  };

  const tmjView = (r: TmjImportReport): WorldView => {
    const notes = r.notes.map((code) => worldText({ code, args: [], message: code }));
    if (r.errors.length > 0) return { state: "error", lines: [...tmjErrorLines(r.errors), ...notes], ready: false };
    const refused = r.bake?.messages.find((m) => m.level === "error");
    return {
      state: r.bake === null ? "info" : refused !== undefined ? "warning" : "ok",
      lines: [tmjChangeText(r), ...(refused !== undefined ? [worldText(refused)] : []), ...notes],
      ready: r.bake !== null,
    };
  };

  const rows: CheckRow[] = [
    ...(freeform !== null ? [worldRow("freeform", freeform, freeformPath, freeformView)] : []),
    ...(tmj !== null ? [worldRow("tmj", tmj, tmjPath, tmjView)] : []),
    ...partRows,
  ];
  const checking =
    (partSource !== null && reports === null && failure === null) || freeform === "reading" || tmj === "reading" || (source !== null && world === null);

  // Built once per map, not per opening: emptying the list while the picker fades out showed „Nichts gefunden“ (GB7).
  const mapItems = useMemo(() => sourceMapItems(mapCount, map), [mapCount, map]);

  return (
    <Dialog
      open={open}
      onOpenChange={(next) => !next && onClose()}
      title={`Teile in Karte ${id4(map)} importieren`}
    >
      <div className={styles.body} data-import-dialog={map} data-world={world === true ? "true" : "false"}>
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
                  void openFiles({ title: "Teile oder Modelle wählen", filters: world === true ? WORLD_FILES : PART_FILES }).then((files) => {
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
        {freeformFiles.length > 0 && (
          <div className={styles.convert} data-import-at="">
            <span className={styles.convertTitle}>Lage</span>
            <div className={styles.convertOptions}>
              {AT_AXES.map((axis, i) => (
                <div key={axis} className={styles.coordinate}>
                  <TextField
                    label={axis}
                    showLabel
                    value={atText[i]!}
                    error={atError[i]}
                    onChange={() => setAtError((prev) => withAt(prev, i, undefined))}
                    onCommit={(text) => {
                      setAtText((prev) => withAt(prev, i, text));
                      const checked = checkNumber(text, "dec", AT_MIN, AT_MAX);
                      if (!checked.ok) setAtError((prev) => withAt(prev, i, checked.error));
                      else {
                        setAtError((prev) => withAt(prev, i, undefined));
                        if (checked.value !== at[i]) setAt((prev) => withAt(prev, i, checked.value));
                      }
                    }}
                  />
                </div>
              ))}
            </div>
            <p className={styles.reason}>Welteinheiten; 16 = eine Kachel; 0, 0, 0 = Kartenmitte</p>
          </div>
        )}
        {source !== null && checking && failure === null && (
          <p className={styles.reason}>Teile werden geprüft…</p>
        )}
        {reports !== null && reports.length === 0 && freeform === null && tmj === null && (
          <p className={styles.reason}>Die Quelle enthält keine Teile.</p>
        )}
        {world === true && reports !== null && reports.length > 0 && (
          <p className={styles.reason} data-import-world-note="">
            {WORLD_ONLY_BAKES}
          </p>
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
              disabled={count === 0 || working || source === null || checking}
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

/** How a world part's row shows: its state, the sentences under the name, whether it can be chosen and its preview. */
interface WorldView {
  state: CheckRow["state"];
  lines: string[];
  ready: boolean;
  preview?: ReactNode;
}

/** `set` without the world parts (a new source of map parts keeps the choice of the world rows, which check themselves). */
function keepWorld(set: ReadonlySet<Choice>): Set<Choice> {
  return new Set([...set].filter((c) => c === "freeform" || c === "tmj"));
}

function without(set: ReadonlySet<Choice>, part: Choice): Set<Choice> {
  const next = new Set(set);
  next.delete(part);
  return next;
}

/** The triple `values` with entry `i` replaced. */
function withAt<T extends readonly unknown[]>(values: T, i: number, value: T[number]): T {
  const next = [...values] as unknown as T & unknown[];
  next[i] = value;
  return next;
}
