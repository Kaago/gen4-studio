import { useEffect, useMemo, useRef, useState } from "react";
import { createArea, errorText } from "../../api";
import type { AreaReport, AreaSpec, HeaderSummary, MatrixDto, MatrixSummary, MemberCounts, WarpLink } from "../../api";
import { Checkbox, Notice, SelectField, TextField } from "../../ui/primitives";
import { AreaGrid, cellKey } from "../AreaGrid";
import { buildPlaces, placeColor } from "../places";
import { useMatrix } from "../useMatrix";
import { TERMS, id4 } from "../terms";
import { useStableCallback } from "../../app/useStable";
import { HeaderChoice } from "./HeaderChoice";
import { useHeaderPatch } from "./useHeaderPatch";
import { PickGrid } from "./PickGrid";
import type { HeaderPlaces } from "./PickGrid";
import { DoneResult, PartialResult } from "./WizardResult";
import { WizardFrame } from "./WizardFrame";
import type { WizardStep } from "./WizardFrame";
import { areaLayout, areaResult, areaSpec, areaSummary, emptyAreaForm, internalName, isLateFailure, newHeaderForm, newHeaderValue, overviewNote, partialText, previewArea, parseMapList, rectSelection, sourceHeader, toggleSelection } from "./wizardSpecs";
import type { AreaForm, HeaderChoiceValue, PickedCell } from "./wizardSpecs";
import parts from "./WizardStep.module.css";
import styles from "./NewAreaWizard.module.css";

interface Props {
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

/** "Gebiet anlegen": copies maps of an existing matrix into a new one and, if asked, gives it to a header. */
export function NewAreaWizard({ headers, matrices, matricesLoaded, matricesFailed, links, counts, prefill, onClose, onCreated, onHeaderPatch, onError, onPlay }: Props) {
  const [form, setForm] = useState<AreaForm>(() => ({ ...emptyAreaForm, sourceMatrix: prefill?.matrix ?? null }));
  const [step, setStep] = useState(0);
  const [busy, setBusy] = useState(false);
  const [outcome, setOutcome] = useState<Outcome | null>(null);
  // The refusal of the last "Anlegen" (nothing was written), shown inside the dialog.
  const [refusal, setRefusal] = useState<string | null>(null);
  // The cell a Shift-click spans its rectangle from.
  const [anchor, setAnchor] = useState<{ x: number; y: number } | null>(null);
  const [rejected, setRejected] = useState<string[]>([]);
  // Nothing chosen yet: the first matrix, also when the overview arrives after the wizard opened.
  const sourceMatrix = form.sourceMatrix ?? matrices[0]?.index ?? null;
  const matrix = useMatrix(sourceMatrix, 0, onError);

  const mapMax = counts.Map === undefined ? 0xfffe : Math.max(0, counts.Map - 1);
  // The header the wizard was opened for leads; else the first source cell's that is no filler (header 0).
  const preferred = prefill?.header;
  const source = matrix ? sourceHeader(matrix, headers, form.selected, preferred) : null;
  // A new header is a copy of the header the area data is taken from, else of header 0.
  const template = source?.id ?? 0;
  const patch = useHeaderPatch(onHeaderPatch);
  const built = areaSpec({ ...form, newHeader: form.newHeader && { ...form.newHeader, template } }, headers.length);
  const preview = useMemo(() => previewMatrix(form, sourceMatrix ?? 0), [form, sourceMatrix]);
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

  async function create() {
    if ("error" in built) return;
    setBusy(true);
    setRefusal(null);
    try {
      setOutcome({ kind: "done", report: await createArea(built.spec), spec: built.spec });
    } catch (e) {
      const text = errorText(e);
      // Something stayed behind: the result step says what, the wizard cannot repair it.
      if (isLateFailure(e)) setOutcome({ kind: "partial", message: partialText(text) });
      else setRefusal(text);
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

  const matrixOptions = useMemo(() => matrices.map((m) => ({ value: m.index, label: `${TERMS.matrix.label} ${id4(m.index)} ${m.name} (${m.width} × ${m.height})` })), [matrices]);
  const summaryLines = "spec" in built ? areaSummary(built.spec, headers, links) : null;

  const steps: WizardStep[] = [
    {
      id: "source",
      title: "Quelle",
      canNext: pickedCount > 0,
      content: (
        <>
          <section className={`${parts.column} ${parts.wide}`}>
            <h3 className={parts.heading}>{TERMS.map.plural} aus einem {TERMS.matrix.label}</h3>
            {matrices.length > 0 && sourceMatrix !== null && (
              <div className={parts.field}>
                <SelectField label={`${TERMS.matrix.label} der Quelle`} showLabel value={sourceMatrix} options={matrixOptions} onChange={chooseSource} />
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
              />
            ) : (
              <p className={parts.note}>{sourceMatrix === null ? (overviewNote(matricesLoaded, matricesFailed) ?? `Kein ${TERMS.matrix.label} vorhanden.`) : "Lade …"}</p>
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
      canNext: "spec" in built,
      content: (
        <>
          <section className={parts.column}>
            <h3 className={parts.heading}>Das neue Gebiet</h3>
            <div className={parts.field}>
              <TextField label="Name des neuen Gebiets" showLabel value={form.name} onCommit={(name) => setForm((f) => ({ ...f, name }))} />
            </div>
            <Checkbox label="ohne Gebäude" checked={form.clearBuildings} onChange={(clearBuildings) => setForm((f) => ({ ...f, clearBuildings }))} />
            <Checkbox label={`${TERMS.header.label}-Abschnitt anlegen`} checked={form.withHeaders} onChange={(withHeaders) => setForm((f) => ({ ...f, withHeaders }))} />
            {"error" in built && <Notice kind="error">{built.error}</Notice>}
          </section>
          <section className={`${parts.column} ${parts.wide}`}>
            <h3 className={parts.heading}>Vorschau</h3>
            {preview ? (
              <div className={styles.preview}>
                <AreaGrid matrix={preview} headers={headers} areaFallback={previewTileset} cellSize={32} />
              </div>
            ) : (
              "spec" in built && <p className={parts.note}>Die Vorschau entfällt bei so vielen Zellen.</p>
            )}
          </section>
        </>
      ),
    },
    {
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
    },
    {
      id: "summary",
      title: "Zusammenfassung",
      canNext: "spec" in built,
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
        </section>
      ),
    },
  ];

  let result = null;
  let onShow: (() => void) | undefined;
  let play: (() => void) | undefined;
  if (outcome?.kind === "done") {
    const { report, spec } = outcome;
    result = <DoneResult title="Gebiet angelegt" {...areaResult(report, spec)} />;
    onShow = () => void show(report, spec);
    if (onPlay) play = () => void showAndPlay(report, spec);
  } else if (outcome?.kind === "partial") {
    result = <PartialResult message={outcome.message} />;
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
