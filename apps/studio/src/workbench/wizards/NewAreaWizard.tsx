import { useMemo, useState } from "react";
import type { CSSProperties } from "react";
import { createArea, errorText } from "../../api";
import type { AreaReport, AreaSpec, HeaderSummary, MatrixDto, MatrixSummary, MemberCounts, WarpLink } from "../../api";
import { AreaGrid, cellKey } from "../AreaGrid";
import { DraftInput } from "../DraftInput";
import { useMatrix } from "../useMatrix";
import { TERMS, id4 } from "../terms";
import { HeaderChoice } from "./HeaderChoice";
import { DoneResult, PartialResult } from "./WizardResult";
import { WizardShell } from "./WizardShell";
import type { WizardStep } from "./WizardShell";
import { areaLayout, areaResult, areaSpec, areaSummary, emptyAreaForm, isLateFailure, parseMapList, rectSelection, sourceHeader, toggleSelection } from "./wizardSpecs";
import type { AreaForm, HeaderChoiceValue, PickedCell } from "./wizardSpecs";

interface Props {
  headers: HeaderSummary[];
  matrices: MatrixSummary[];
  links: WarpLink[];
  counts: MemberCounts;
  prefill?: { matrix?: number };
  /** The wizard is left; `written` says whether it changed the project (the app then reads it again). */
  onClose: (written: boolean) => void;
  /** "Im Studio zeigen": the app reads the project again and shows the new area. */
  onCreated: (report: AreaReport, spec: AreaSpec) => Promise<void>;
  onError: (e: unknown) => void;
}

type Outcome = { kind: "done"; report: AreaReport; spec: AreaSpec } | { kind: "partial"; message: string };

/** Above this many cells the preview is left out (a 255 × 255 grid would be 65,025 buttons). */
const PREVIEW_CELLS = 1024;

const row: CSSProperties = { display: "flex", alignItems: "center", gap: 8, margin: "6px 0" };
const note: CSSProperties = { color: "#6b7280", margin: "6px 0" };

/** Builds the matrix the preview grid shows: the layout's maps, and a header section filled with the chosen header. */
function previewMatrix(form: AreaForm): MatrixDto | null {
  const { width, height, cells } = areaLayout(form);
  if (cells.length === 0 || width * height > PREVIEW_CELLS) return null;
  const maps: (number | null)[] = Array.from({ length: width * height }, () => null);
  for (const c of cells) maps[c.y * width + c.x] = c.source_map;
  return { index: form.sourceMatrix ?? 0, name: form.name, width, height, maps, headers: form.withHeaders ? maps.map(() => form.header ?? 0) : undefined };
}

/** "Gebiet anlegen": copies maps of an existing matrix into a new one and, if asked, gives it to a header. */
export function NewAreaWizard({ headers, matrices, links, counts, prefill, onClose, onCreated, onError }: Props) {
  const [form, setForm] = useState<AreaForm>(() => ({ ...emptyAreaForm, sourceMatrix: prefill?.matrix ?? null }));
  const [step, setStep] = useState(0);
  const [busy, setBusy] = useState(false);
  const [outcome, setOutcome] = useState<Outcome | null>(null);
  // The cell a Shift-click spans its rectangle from.
  const [anchor, setAnchor] = useState<{ x: number; y: number } | null>(null);
  const [rejected, setRejected] = useState<string[]>([]);
  // Nothing chosen yet: the first matrix, also when the overview arrives after the wizard opened.
  const sourceMatrix = form.sourceMatrix ?? matrices[0]?.index ?? null;
  const matrix = useMatrix(sourceMatrix, 0, onError);

  const mapMax = counts.Map === undefined ? 0xfffe : Math.max(0, counts.Map - 1);
  const source = matrix ? sourceHeader(matrix, headers, form.selected) : null;
  const built = areaSpec(form);
  const preview = useMemo(() => previewMatrix(form), [form]);
  const selectedKeys = useMemo(() => new Set(form.selected.map((c) => cellKey(c.x, c.y))), [form.selected]);

  /** A new selection: area data follows the first source cell's header unless the user changed it, and a copy source that went away is dropped. */
  function pick(selected: PickedCell[]) {
    setForm((f) => {
      const before = matrix ? sourceHeader(matrix, headers, f.selected) : null;
      const after = matrix ? sourceHeader(matrix, headers, selected) : null;
      const untouched = f.areaDataId === (before?.area_data_id ?? null);
      const lostCopy = typeof f.eventFile === "object" && f.eventFile.Copy !== after?.event_file_id;
      return { ...f, selected, areaDataId: untouched ? (after?.area_data_id ?? null) : f.areaDataId, eventFile: lostCopy ? "Keep" : f.eventFile };
    });
  }

  function onCell(x: number, y: number, { shift }: { shift: boolean }) {
    if (!matrix) return;
    pick(shift && anchor ? rectSelection(form.selected, matrix, anchor, { x, y }) : toggleSelection(form.selected, matrix, x, y));
    setAnchor({ x, y });
  }

  function chooseSource(index: number) {
    setForm((f) => ({ ...f, sourceMatrix: index, selected: [], areaDataId: null, eventFile: typeof f.eventFile === "object" ? "Keep" : f.eventFile }));
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
    try {
      setOutcome({ kind: "done", report: await createArea(built.spec), spec: built.spec });
    } catch (e) {
      const text = errorText(e);
      // Something stayed behind: the result step says what, the wizard cannot repair it.
      if (isLateFailure(text)) setOutcome({ kind: "partial", message: text });
      else onError(e);
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

  const headerValue: HeaderChoiceValue = { header: form.header, areaDataId: form.areaDataId, eventFile: form.eventFile };
  const pickedCount = form.selected.length + form.manualMaps.length;

  const steps: WizardStep[] = [
    {
      id: "source",
      title: "Quelle",
      canNext: pickedCount > 0,
      content: (
        <div>
          <label style={row}>
            {TERMS.matrix.label}
            <select aria-label={`${TERMS.matrix.label} der Quelle`} value={sourceMatrix ?? ""} onChange={(e) => chooseSource(Number(e.target.value))}>
              {matrices.map((m) => (
                <option key={m.index} value={m.index}>
                  {TERMS.matrix.label} {id4(m.index)} {m.name} ({m.width} × {m.height})
                </option>
              ))}
            </select>
          </label>
          <div style={note}>{TERMS.map.plural} anklicken; Umschalt-Klick wählt das Rechteck bis zur letzten Zelle.</div>
          <div style={{ maxHeight: 320, overflow: "auto", padding: 2 }}>
            {matrix ? (
              <AreaGrid
                matrix={matrix}
                headers={headers}
                areaFallback={headers.find((h) => h.matrix_id === matrix.index)?.area_data_id ?? 0}
                cellSize={32}
                selected={selectedKeys}
                onCellClick={onCell}
              />
            ) : (
              <div style={note}>{sourceMatrix === null ? `Kein ${TERMS.matrix.label} vorhanden.` : "Lade …"}</div>
            )}
          </div>
          <label style={row}>
            {TERMS.map.plural} per Nummer:
            <DraftInput label={`${TERMS.map.plural} per Nummer`} value={form.manualMaps.join(", ")} onCommit={setManual} placeholder="z. B. 12, 13" style={{ flex: 1 }} />
          </label>
          {rejected.length > 0 && (
            <div style={{ color: "#b91c1c" }}>
              Nicht übernommen (keine {TERMS.map.label} von 0 bis {mapMax}): {rejected.join(", ")}
            </div>
          )}
          <div style={note}>
            {pickedCount} {pickedCount === 1 ? TERMS.map.label : TERMS.map.plural} gewählt.
          </div>
        </div>
      ),
    },
    {
      id: "layout",
      title: "Anordnung",
      canNext: "spec" in built,
      content: (
        <div>
          <label style={row}>
            Name
            <DraftInput label="Name des neuen Gebiets" value={form.name} onCommit={(name) => setForm((f) => ({ ...f, name }))} style={{ flex: 1 }} />
          </label>
          <label style={row}>
            <input type="checkbox" checked={form.clearBuildings} onChange={(e) => setForm((f) => ({ ...f, clearBuildings: e.target.checked }))} />
            ohne Gebäude
          </label>
          <label style={row}>
            <input type="checkbox" checked={form.withHeaders} onChange={(e) => setForm((f) => ({ ...f, withHeaders: e.target.checked }))} />
            {TERMS.header.label}-Abschnitt anlegen
          </label>
          {"error" in built && <div style={{ color: "#b91c1c" }}>{built.error}</div>}
          <div style={{ maxHeight: 320, overflow: "auto", padding: 2, marginTop: 8 }}>
            {preview ? (
              <AreaGrid matrix={preview} headers={headers} areaFallback={0} cellSize={32} />
            ) : (
              "spec" in built && <div style={note}>Die Vorschau entfällt bei so vielen Zellen.</div>
            )}
          </div>
        </div>
      ),
    },
    {
      id: "header",
      title: TERMS.header.label,
      canNext: true,
      content: <HeaderChoice headers={headers} links={links} counts={counts} value={headerValue} copySource={source?.event_file_id ?? null} onChange={(v) => setForm((f) => ({ ...f, ...v }))} />,
    },
    {
      id: "summary",
      title: "Zusammenfassung",
      canNext: "spec" in built,
      content: (
        <div>
          {"spec" in built ? (
            areaSummary(built.spec, headers).map((line) => (
              <p key={line} style={{ margin: "6px 0" }}>
                {line}
              </p>
            ))
          ) : (
            <p style={{ color: "#b91c1c" }}>{built.error}</p>
          )}
        </div>
      ),
    },
  ];

  let result = null;
  if (outcome?.kind === "done") {
    const { report, spec } = outcome;
    result = <DoneResult title="Gebiet angelegt" {...areaResult(report, spec)} busy={busy} onShow={() => void show(report, spec)} />;
  } else if (outcome?.kind === "partial") {
    result = <PartialResult message={outcome.message} />;
  }

  return (
    <WizardShell
      title="Gebiet anlegen"
      steps={steps}
      step={step}
      onStep={setStep}
      submitLabel="Anlegen"
      onSubmit={() => void create()}
      onClose={() => onClose(outcome !== null)}
      busy={busy}
      result={result}
    />
  );
}
