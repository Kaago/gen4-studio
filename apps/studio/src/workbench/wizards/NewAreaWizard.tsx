import { useEffect, useMemo, useRef, useState } from "react";
import { createArea, errorText } from "../../api";
import type { AreaReport, AreaSpec, HeaderFamily, HeaderSummary, MatrixDto, MatrixSummary, MemberCounts, WarpLink } from "../../api";
import { depthAtLeast } from "../../ui/frame/shellState";
import type { Depth } from "../../ui/frame/shellState";
import { Checkbox, Notice, Segmented, SelectField, TextField } from "../../ui/primitives";
import { AreaGrid, cellKey } from "../AreaGrid";
import { buildPlaces, placeColor } from "../places";
import { useMatrix } from "../useMatrix";
import { TERMS, id4 } from "../terms";
import type { World } from "../worldModel";
import { targetBlock } from "../worldMapView";
import { useStableCallback } from "../../app/useStable";
import { HeaderChoice } from "./HeaderChoice";
import { PatchConsent, patchReady } from "./PatchConsent";
import { PlaceStep } from "./PlaceStep";
import type { PlaceStepValue } from "./PlaceStep";
import { useHeaderPatch } from "./useHeaderPatch";
import { PickGrid } from "./PickGrid";
import { PlaceOnWorldMap } from "./PlaceOnWorldMap";
import type { HeaderPlaces } from "./PickGrid";
import { DoneResult, PartialResult } from "./WizardResult";
import { WizardFrame } from "./WizardFrame";
import type { WizardStep } from "./WizardFrame";
import { areaLayout, areaPlace, areaResult, buildingSources, roomPlace, areaSpec, areaSummary, emptyAreaForm, internalName, isLateFailure, newHeaderForm, PICK_TARGET, placeError, placeForm, newHeaderValue, overviewNote, partialText, previewArea, parseMapList, rectSelection, sourceHeader, toggleSelection, worldMapChoices } from "./wizardSpecs";
import type { AreaForm, HeaderChoiceValue, PickedCell } from "./wizardSpecs";
import parts from "./WizardStep.module.css";
import styles from "./NewAreaWizard.module.css";

interface Props {
  /** Bauen gives the new area a new header with a place and a look (step „Ort“); Tüfteln lets the user choose the header. */
  depth: Depth;
  /** The world model: Bauen offers its world maps and „Weitere Gebiete“ as sources, by name (null: not read yet). */
  world: World | null;
  headers: HeaderSummary[];
  matrices: MatrixSummary[];
  /** Whether the overview (`matrices`) has been read, or its read failed: until then no source can be offered. */
  matricesLoaded: boolean;
  matricesFailed: boolean;
  links: WarpLink[];
  counts: MemberCounts;
  /** The source matrix, and the header whose cells in it are selected when the wizard opens. */
  prefill?: { matrix?: number; header?: number };
  /** The wizard is left; `written` says whether it changed the project (the app then reads it again). */
  onClose: (written: boolean) => void;
  /** "Zeigen": the app reads the project again and shows the new area. */
  onCreated: (report: AreaReport, spec: AreaSpec) => Promise<void>;
  /** The Dynamic Headers patch was applied: the app reads the header table and list again. */
  onHeaderPatch: () => Promise<void>;
  /** A repair of the patch went in (the app closes the toast that offered it). */
  onHeaderPatchRepaired?: () => void;
  onError: (e: unknown) => void;
  /** "Hier spielen" on the result: after it was shown, the app saves, builds and runs. */
  onPlay?: () => void;
}

type Outcome = { kind: "done"; report: AreaReport; spec: AreaSpec } | { kind: "partial"; message: string };

/** Above this many cells the preview is left out (a 255 × 255 grid would be 65,025 buttons). */
const PREVIEW_CELLS = 1024;

/**
 * Builds the matrix the preview grid shows: the layout's maps only. It has no header section on purpose: the grid
 * then draws every cell with `previewArea`, the tileset the new area will be shown with.
 */
function previewMatrix(form: AreaForm, index: number): MatrixDto | null {
  const { width, height, cells } = areaLayout(form);
  if (cells.length === 0 || width * height > PREVIEW_CELLS) return null;
  const maps: (number | null)[] = Array.from({ length: width * height }, () => null);
  for (const c of cells) maps[c.y * width + c.x] = c.source_map;
  return { index, name: form.name, width, height, maps };
}

/** The choice „Eigene Karte“ of „Wo liegt das Gebiet?“; a world map is `w<matrix>`. */
const OWN_MAP = "own";

/** Bauen's accessible name of a wizard grid: „Karten der Quelle, 2 Zeilen, 3 Spalten“ (the grids' own name says „Raster“). */
const gridLabel = (what: string, matrix: { width: number; height: number }) => `${what}, ${matrix.height} Zeilen, ${matrix.width} Spalten`;

/** "Gebiet anlegen": copies maps of an existing matrix into a new one and, if asked, gives it to a header. */
export function NewAreaWizard({ depth, world, headers, matrices, matricesLoaded, matricesFailed, links, counts, prefill, onClose, onCreated, onHeaderPatch, onHeaderPatchRepaired, onError, onPlay }: Props) {
  const [stored, setForm] = useState<AreaForm>(() => ({ ...emptyAreaForm, sourceMatrix: prefill?.matrix ?? null }));
  const building = !depthAtLeast(depth, "tuefteln");
  // Bauen always makes a new header with an empty event file: what Tüfteln chose does not count there.
  const form = useMemo<AreaForm>(() => (building ? { ...stored, header: null, newHeader: null, areaDataId: stored.lookAreaData, eventFile: "New" } : stored), [building, stored]);
  const [step, setStep] = useState(0);
  const [busy, setBusy] = useState(false);
  const [outcome, setOutcome] = useState<Outcome | null>(null);
  // The refusal of the last "Anlegen" (nothing was written), shown inside the dialog.
  const [refusal, setRefusal] = useState<string | null>(null);
  // The cell a Shift-click spans its rectangle from.
  const [anchor, setAnchor] = useState<{ x: number; y: number } | null>(null);
  const [rejected, setRejected] = useState<string[]>([]);
  // Nothing chosen yet: the first matrix (in Bauen the first world map), also when the overview or the world arrives after the wizard opened.
  const firstSource = (building ? buildingSources(world, headers, null)[0]?.value : undefined) ?? matrices[0]?.index ?? null;
  const sourceMatrix = form.sourceMatrix ?? firstSource;
  const matrix = useMatrix(sourceMatrix, 0, onError);

  const mapMax = counts.Map === undefined ? 0xfffe : Math.max(0, counts.Map - 1);
  // The header the wizard was opened for leads; else the first source cell's that is no filler (header 0).
  const preferred = prefill?.header;
  const source = matrix ? sourceHeader(matrix, headers, form.selected, preferred) : null;
  // A new header is a copy of the header the area data is taken from, else of header 0.
  const template = source?.id ?? 0;
  const patch = useHeaderPatch(onHeaderPatch, { repaired: onHeaderPatchRepaired });
  // Only Diamond/Pearl lacks the patch, and only it has no new places: the step asks for nothing else.
  const family: HeaderFamily = patch.patch?.supported === false ? "DP" : "Pt";
  // Bauen: the place and the look follow their defaults (a new place named like the area, the template's look) until chosen; where
  // „Neuer Ort“ is locked, the template's place.
  const placeDefault = family === "DP" ? (roomPlace(null, null, source) ?? { Existing: 0 }) : areaPlace({ ...form, place: null });
  const place = form.place ?? placeDefault;
  const look = form.lookAreaData ?? source?.area_data_id ?? 0;
  // The consent counts only while the patch is missing.
  const patchConsent = form.patchConsent && patch.patch?.applied === false;
  const built = building
    ? areaSpec({ ...form, newHeader: { template, internalName: null }, place, patchConsent }, headers.length, depth)
    : areaSpec({ ...form, newHeader: form.newHeader && { ...form.newHeader, template } }, headers.length, depth);
  const preview = useMemo(() => previewMatrix(form, sourceMatrix ?? 0), [form, sourceMatrix]);
  // „Wo liegt das Gebiet?“: on a world map the block at the chosen cell must fall on empty cells or landscape.
  const worldMaps = useMemo(() => worldMapChoices(world), [world]);
  const targetMatrix = useMatrix(form.worldMap, 0, onError);
  const layout = useMemo(() => areaLayout(form), [form]);
  const target = form.worldMap !== null && form.target?.matrix === form.worldMap ? form.target : null;
  const block = targetMatrix && target ? targetBlock(targetMatrix, target, layout.width, layout.height, layout.cells) : null;
  const placed = form.worldMap === null || block?.ok === true;
  const previewTileset = previewArea(form, headers, source);
  const selectedKeys = useMemo(() => new Set(form.selected.map((c) => cellKey(c.x, c.y))), [form.selected]);
  // The place and colour of every header, built once: a grid asks for each of its cells.
  const headerPlaces = useMemo<HeaderPlaces>(() => {
    const byHeader = new Map<number, { name: string; color: string }>();
    for (const place of buildPlaces(headers, [])) {
      const color = placeColor(place.key);
      for (const { header } of place.headers) byHeader.set(header.id, { name: place.name, color });
    }
    return byHeader;
  }, [headers]);

  /** A new selection: area data follows the first source cell's header unless the user changed it, and a copy source that went away is dropped. */
  function pick(selected: PickedCell[]) {
    setForm((f) => {
      const before = matrix ? sourceHeader(matrix, headers, f.selected, preferred) : null;
      const after = matrix ? sourceHeader(matrix, headers, selected, preferred) : null;
      const untouched = f.areaDataId === (before?.area_data_id ?? null);
      const lostCopy = typeof f.eventFile === "object" && f.eventFile.Copy !== after?.event_file_id;
      return { ...f, selected, areaDataId: untouched ? (after?.area_data_id ?? null) : f.areaDataId, eventFile: lostCopy ? emptyAreaForm.eventFile : f.eventFile };
    });
  }

  // Opened for a header: its cells of the source matrix are selected once, as soon as the matrix is read.
  const preselected = useRef(false);
  useEffect(() => {
    if (preselected.current || !matrix || prefill?.header === undefined || matrix.index !== prefill.matrix || !matrix.headers || form.selected.length > 0) return;
    preselected.current = true;
    const cells: PickedCell[] = [];
    matrix.maps.forEach((map, i) => {
      if (map !== null && matrix.headers?.[i] === prefill.header) cells.push({ x: i % matrix.width, y: Math.floor(i / matrix.width), map });
    });
    if (cells.length > 0) pick(cells);
  }, [matrix]);

  const onCell = useStableCallback((x: number, y: number, { shift }: { shift: boolean }) => {
    if (!matrix) return;
    pick(shift && anchor ? rectSelection(form.selected, matrix, anchor, { x, y }) : toggleSelection(form.selected, matrix, x, y));
    setAnchor({ x, y });
  });

  function chooseSource(index: number) {
    setForm((f) => ({ ...f, sourceMatrix: index, selected: [], areaDataId: null, eventFile: typeof f.eventFile === "object" ? emptyAreaForm.eventFile : f.eventFile }));
    setAnchor(null);
  }

  function setManual(text: string) {
    const { maps, invalid } = parseMapList(text, mapMax);
    setForm((f) => ({ ...f, manualMaps: maps }));
    setRejected(invalid);
  }

  /** The app reads the header table and list again after a create that applied the patch; a failed read goes to `onError`, the result stays. */
  async function rereadPatch() {
    try {
      await onHeaderPatch();
    } catch (e) {
      onError(e);
    }
  }

  async function create() {
    if ("error" in built) return;
    setBusy(true);
    setRefusal(null);
    const patching = built.spec.header?.allow_patch === true;
    try {
      const report = await createArea(built.spec);
      setOutcome({ kind: "done", report, spec: built.spec });
      // The call applied the patch: the app reads the header table and list again, as after „Patch anwenden“.
      if (report.patched) await rereadPatch();
    } catch (e) {
      const text = errorText(e);
      // Something stayed behind: the result step says what, the wizard cannot repair it. A patch asked for may be in by then.
      if (isLateFailure(e)) {
        setOutcome({ kind: "partial", message: partialText(text, depth, "area") });
        if (patching) await rereadPatch();
      } else setRefusal(text);
    } finally {
      setBusy(false);
    }
  }

  async function show(report: AreaReport, spec: AreaSpec) {
    setBusy(true);
    try {
      await onCreated(report, spec);
    } finally {
      setBusy(false);
    }
  }

  /** "Hier spielen": shows what was made, then plays. */
  async function showAndPlay(report: AreaReport, spec: AreaSpec) {
    await show(report, spec);
    onPlay?.();
  }

  const headerValue: HeaderChoiceValue = { header: form.header, newHeader: newHeaderValue(form.newHeader, form.name, template), areaDataId: form.areaDataId, eventFile: form.eventFile };
  const pickedCount = form.selected.length + form.manualMaps.length;

  // Bauen names the sources (world maps, „Weitere Gebiete“); Tüfteln lists every raster by number.
  const matrixOptions = useMemo(
    () => (building ? buildingSources(world, headers, sourceMatrix) : matrices.map((m) => ({ value: m.index, label: `${TERMS.matrix.label} ${id4(m.index)} ${m.name} (${m.width} × ${m.height})` }))),
    [building, world, headers, sourceMatrix, matrices],
  );
  const worldMapName = worldMaps.find((m) => m.matrix === form.worldMap)?.name ?? "";
  const summaryLines = "spec" in built ? areaSummary(built.spec, headers, links, depth, { name: worldMapName, replaced: block?.replaced ?? 0 }) : null;
  const lageOptions = [{ value: OWN_MAP, label: "Eigene Karte (über Türen verbunden)" }, ...worldMaps.map((m) => ({ value: `w${m.matrix}`, label: `Auf der ${TERMS.worldMap.label} ${m.name} (nahtlos begehbar)` }))];

  /** A choice of „Wo liegt das Gebiet?“: the cell chosen on a world map stays for when the user comes back to it. */
  function chooseLage(value: string) {
    const worldMap = worldMaps.find((m) => `w${m.matrix}` === value)?.matrix ?? null;
    setForm((f) => ({ ...f, worldMap }));
  }

  /** A choice of the step „Ort“: a default is kept as null, so it follows the area's name and the template. */
  function choosePlace(value: PlaceStepValue) {
    setForm((f) => ({ ...f, place: placeForm(value.place, family === "DP" ? placeDefault : areaPlace({ ...f, place: null })), lookAreaData: value.areaDataId === (source?.area_data_id ?? 0) ? null : value.areaDataId }));
  }

  // Bauen asks for the place and the look; Tüfteln for the header itself.
  const headerStep: WizardStep = building
    ? {
        id: "place",
        title: "Ort",
        canNext: placeError(place) === null,
        content: <PlaceStep value={{ place, areaDataId: look }} onChange={choosePlace} headers={headers} template={source} family={family} name={form.name} />,
      }
    : {
        id: "header",
        title: TERMS.header.label,
        canNext: true,
        content: (
          <HeaderChoice
            headers={headers}
            links={links}
            counts={counts}
            value={headerValue}
            copySource={source?.event_file_id ?? null}
            onChange={(v) => setForm((f) => ({ ...f, ...v, newHeader: newHeaderForm(v.newHeader, f.name) }))}
            patch={patch.patch}
            patchError={patch.error}
            patchBusy={patch.busy}
            onApplyPatch={patch.apply}
            onRepairPatch={patch.repair}
            template={template}
            defaultName={internalName(form.name)}
          />
        ),
      };

  const steps: WizardStep[] = [
    {
      id: "source",
      title: "Quelle",
      canNext: pickedCount > 0,
      content: (
        <>
          <section className={`${parts.column} ${parts.wide}`}>
            <h3 className={parts.heading}>{building ? `${TERMS.map.plural} aus einer ${TERMS.worldMap.label} oder einem ${TERMS.area.label}` : `${TERMS.map.plural} aus einem ${TERMS.matrix.label}`}</h3>
            {matrixOptions.length > 0 && sourceMatrix !== null && (
              <div className={parts.field}>
                <SelectField label={building ? "Quelle" : `${TERMS.matrix.label} der Quelle`} showLabel value={sourceMatrix} options={matrixOptions} onChange={chooseSource} />
              </div>
            )}
            <p className={parts.note}>{TERMS.map.plural} anklicken; Umschalt-Klick wählt das Rechteck bis zur letzten Zelle.</p>
            {matrix ? (
              <PickGrid
                matrix={matrix}
                headers={headers}
                areaFallback={headers.find((h) => h.matrix_id === matrix.index)?.area_data_id ?? 0}
                selected={selectedKeys}
                places={headerPlaces}
                last={anchor}
                onCellClick={onCell}
                depth={depth}
                label={building ? gridLabel(`${TERMS.map.plural} der Quelle`, matrix) : undefined}
              />
            ) : (
              <p className={parts.note}>{sourceMatrix === null ? (overviewNote(matricesLoaded, matricesFailed) ?? (building ? `Keine ${TERMS.worldMap.label} vorhanden.` : `Kein ${TERMS.matrix.label} vorhanden.`)) : "Lade …"}</p>
            )}
          </section>
          <section className={parts.column}>
            <h3 className={parts.heading}>Dazu per Nummer</h3>
            <div className={parts.field}>
              <TextField
                label={`${TERMS.map.plural} per Nummer`}
                showLabel
                value={form.manualMaps.join(", ")}
                onCommit={setManual}
                placeholder="z. B. 12, 13"
                error={rejected.length > 0 ? `Nicht übernommen (keine ${TERMS.map.label} von 0 bis ${mapMax}): ${rejected.join(", ")}` : undefined}
              />
            </div>
            <p className={parts.text}>
              {pickedCount} {pickedCount === 1 ? TERMS.map.label : TERMS.map.plural} gewählt.
            </p>
          </section>
        </>
      ),
    },
    {
      id: "layout",
      title: "Anordnung",
      canNext: "spec" in built && placed,
      content: (
        <>
          <section className={parts.column}>
            <h3 className={parts.heading}>Das neue Gebiet</h3>
            <div className={parts.field}>
              <TextField label="Name des neuen Gebiets" showLabel value={form.name} onCommit={(name) => setForm((f) => ({ ...f, name }))} />
            </div>
            <Checkbox label="ohne Gebäude" checked={form.clearBuildings} onChange={(clearBuildings) => setForm((f) => ({ ...f, clearBuildings }))} />
            {/* Bauen always gives the new area its cells (`areaSpec`). */}
            {/* An area on a world map takes the world map's cells: the switch does not apply there. */}
            {!building && form.worldMap === null && <Checkbox label={`${TERMS.header.label}-Abschnitt anlegen`} checked={form.withHeaders} onChange={(withHeaders) => setForm((f) => ({ ...f, withHeaders }))} />}
            {worldMaps.length > 0 && (
              <>
                <h3 className={parts.heading}>Wo liegt das {TERMS.area.label}?</h3>
                <Segmented ariaLabel="Lage" options={lageOptions} value={form.worldMap === null ? OWN_MAP : `w${form.worldMap}`} onChange={chooseLage} wrap />
              </>
            )}
            {/* Still to choose a cell of the world map: the grid's hint says so, no error. */}
            {"error" in built && built.error !== PICK_TARGET && <Notice kind="error">{built.error}</Notice>}
          </section>
          <section className={`${parts.column} ${parts.wide}`}>
            <h3 className={parts.heading}>{form.worldMap === null ? "Vorschau" : `${TERMS.worldMap.label} ${worldMapName}`}</h3>
            {form.worldMap !== null ? (
              <PlaceOnWorldMap
                world={world}
                matrix={targetMatrix}
                headers={headers}
                width={layout.width}
                height={layout.height}
                value={target}
                onChange={({ x, y }) => setForm((f) => (f.worldMap === null ? f : { ...f, target: { matrix: f.worldMap, x, y } }))}
                depth={depth}
                focus={source?.id ?? null}
              />
            ) : preview ? (
              <div className={styles.preview}>
                <AreaGrid matrix={preview} headers={headers} areaFallback={previewTileset} cellSize={32} label={building ? gridLabel("Vorschau des neuen Gebiets", preview) : undefined} />
              </div>
            ) : (
              "spec" in built && <p className={parts.note}>Die Vorschau entfällt bei so vielen Zellen.</p>
            )}
          </section>
        </>
      ),
    },
    headerStep,
    {
      id: "summary",
      title: "Zusammenfassung",
      canNext: "spec" in built && placed && (!building || patchReady(patch.patch, form.patchConsent)),
      content: (
        <section className={parts.column}>
          {summaryLines ? (
            <Notice kind="info">
              <div className={parts.lines}>
                {summaryLines.map((line) => (
                  <p key={line}>{line}</p>
                ))}
              </div>
            </Notice>
          ) : (
            "error" in built && <Notice kind="error">{built.error}</Notice>
          )}
          {building && <PatchConsent patch={patch.patch} error={patch.error} checked={form.patchConsent} onChange={(patchConsent) => setForm((f) => ({ ...f, patchConsent }))} />}
        </section>
      ),
    },
  ];

  let result = null;
  let onShow: (() => void) | undefined;
  let play: (() => void) | undefined;
  if (outcome?.kind === "done") {
    const { report, spec } = outcome;
    result = <DoneResult title="Gebiet angelegt" {...areaResult(report, spec, depth)} />;
    onShow = () => void show(report, spec);
    if (onPlay) play = () => void showAndPlay(report, spec);
  } else if (outcome?.kind === "partial") {
    result = <PartialResult message={outcome.message} depth={depth} />;
  }

  return (
    <WizardFrame
      title="Gebiet anlegen"
      icon="matrix"
      steps={steps}
      step={step}
      onStep={(i) => {
        setRefusal(null);
        setStep(i);
      }}
      submitLabel="Anlegen"
      onSubmit={() => void create()}
      onCancel={() => onClose(outcome !== null)}
      onShow={onShow}
      onPlay={play}
      busy={busy || patch.busy}
      result={result}
      error={refusal}
      summary={summaryLines?.[0]}
      irreversible
    />
  );
}
