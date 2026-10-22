import { memo, useEffect, useMemo, useState } from "react";
import type { CSSProperties } from "react";
import { ask } from "@tauri-apps/plugin-dialog";
import { createMatrix, renameMatrix, resizeMatrix, setMatrixSections } from "../api";
import type { HeaderSummary, MatrixDto, MemberCounts, ProjectInfo } from "../api";
import { AreaGrid, cellKey } from "./AreaGrid";
import { DraftInput } from "./DraftInput";
import { ExtendAreaDialog } from "./ExtendAreaDialog";
import { parseCellField, resizePreview } from "./matrix";
import type { NavTarget } from "./navigation";
import { buildPlaces, placeColor, placeOf } from "./places";
import { TERMS } from "./terms";
import { useMatrix } from "./useMatrix";

/** What the matrix panel needs from App. */
export interface MatrixPanelContext {
  info: ProjectInfo | null;
  /** The header list: the pictures of cells with a header use that header's tileset, the cells are tinted by its place. */
  headers: HeaderSummary[];
  /** Area data of the scene: the tileset of cells without a header of their own. */
  areaDataId: number;
  counts: MemberCounts;
  /** Matrix of the scene (the selected header's); null while no header is selected. */
  sceneMatrix: number | null;
  /** Changes whenever matrix data may have changed behind the panel (edits, undo, redo, reload). */
  revision: number;
  /** The cell shown in the inspector, if any. */
  selection: { matrix: number; x: number; y: number } | null;
  /** Follows a link: a click shows the cell in the inspector, a double click on a cell with a header opens the header. */
  goTo: (target: NavTarget) => void;
  /** A matrix was edited: refresh dirty count and, if it is the scene's matrix, the scene. */
  onEdited: (index: number) => void | Promise<void>;
  /** A member was appended to an archive: refresh the member counts. */
  onCountsChanged: () => void | Promise<void>;
  onError: (e: unknown) => void;
}

const bar: CSSProperties = { display: "flex", flexWrap: "wrap", alignItems: "center", gap: 8, padding: "8px 12px" };
const note: CSSProperties = { margin: 0, padding: "0 12px 8px", fontSize: 12, color: "#6b7280" };
const dim: CSSProperties = { width: 56, textAlign: "right", fontFamily: "ui-monospace, monospace" };

function sectionHasValues(m: MatrixDto, section: "headers" | "altitudes"): boolean {
  return (m[section] ?? []).some((v) => v !== 0);
}

/** Memoised: renders when `context` changes (the owner keeps it, see `MatrixTab`), not for every render of the panel host. */
export const MatrixPanel = memo(function MatrixPanel({ context }: { context: MatrixPanelContext }) {
  const { info, headers, areaDataId, counts, sceneMatrix, revision, selection, goTo, onEdited, onCountsChanged, onError } = context;
  // A freshly created matrix is shown here although no header uses it yet; the choice of header
  // (or project) ends that.
  const [created, setCreated] = useState<number | null>(null);
  useEffect(() => setCreated(null), [sceneMatrix, info?.root]);
  const index = created ?? sceneMatrix;
  const matrix = useMatrix(index, revision, onError);
  const [extendAt, setExtendAt] = useState<{ x: number; y: number } | null>(null);
  // The dialog belongs to the cell it was opened for: another matrix or project ends it.
  useEffect(() => setExtendAt(null), [index, info?.root]);
  const places = useMemo(() => buildPlaces(headers, []), [headers]);
  const colorOf = useMemo(
    () => (header: number | null) => {
      if (header === null) return null;
      const place = placeOf(places, header);
      return place ? placeColor(place.key) : null;
    },
    [places],
  );
  const selected = useMemo(
    () => (matrix && selection?.matrix === matrix.index ? new Set([cellKey(selection.x, selection.y)]) : undefined),
    [matrix, selection],
  );

  /** Runs one edit of the shown matrix; the panel reloads through the new revision. */
  async function edit(index: number, op: () => Promise<unknown>) {
    try {
      await op();
    } catch (e) {
      onError(e);
    } finally {
      await onEdited(index);
    }
  }

  async function onNew() {
    try {
      const made = await createMatrix("neu");
      setCreated(made);
      await onCountsChanged();
      goTo({ kind: "matrixCell", matrix: made, x: 0, y: 0 });
    } catch (e) {
      onError(e);
    }
  }

  /** The extend dialog is done: a created copy shows in the counts and the matrix; a failed attempt may have left a copy behind, so it refreshes the same. */
  async function afterExtend(m: MatrixDto) {
    try {
      await onCountsChanged();
    } finally {
      await onEdited(m.index);
    }
  }

  async function onResize(m: MatrixDto, width: number, height: number) {
    if (width === m.width && height === m.height) return;
    const { lostCells } = resizePreview(width, height, m);
    if (lostCells > 0) {
      const text = `${lostCells} belegte Zelle(n) liegen außerhalb von ${width} × ${height} und gehen verloren. Fortfahren?`;
      if (!(await ask(text, { kind: "warning" }))) return;
    }
    await edit(m.index, () => resizeMatrix(m.index, width, height));
  }

  async function onSection(m: MatrixDto, section: "headers" | "altitudes", on: boolean) {
    if (!on && sectionHasValues(m, section)) {
      const what = section === "headers" ? "Header-Werte" : "Höhenwerte";
      if (!(await ask(`Der Abschnitt enthält Werte ungleich 0. Beim Entfernen gehen die ${what} verloren. Fortfahren?`, { kind: "warning" }))) return;
    }
    const headers = section === "headers" ? on : m.headers != null;
    const altitudes = section === "altitudes" ? on : m.altitudes != null;
    await edit(m.index, () => setMatrixSections(m.index, headers, altitudes));
  }

  const size = (m: MatrixDto, apply: (width: number, height: number) => void) => (axis: "width" | "height") => (text: string) => {
    const n = parseCellField(text, 255, false);
    if (n === null || n === "empty" || n < 1) return;
    apply(axis === "width" ? n : m.width, axis === "height" ? n : m.height);
  };

  if (!info) return <p style={{ ...note, padding: 16 }}>Kein Projekt geöffnet.</p>;

  const sizer = matrix ? size(matrix, (w, h) => void onResize(matrix, w, h)) : null;
  return (
    <div style={{ height: "100%", display: "flex", flexDirection: "column", fontFamily: "system-ui, sans-serif", fontSize: 13, color: "#111827" }}>
      <div style={bar} title={TERMS.matrix.short}>
        {matrix && sizer ? (
          <>
            <span style={{ fontFamily: "ui-monospace, monospace" }}>{String(matrix.index).padStart(4, "0")}</span>
            <DraftInput
              label="Name der Matrix"
              value={matrix.name}
              onCommit={(name) => void edit(matrix.index, () => renameMatrix(matrix.index, name))}
              style={{ width: 160 }}
            />
            <label>
              Breite{" "}
              <DraftInput label="Breite (1 bis 255)" type="number" min={1} max={255} value={String(matrix.width)} onCommit={sizer("width")} style={dim} />
            </label>
            <label>
              Höhe{" "}
              <DraftInput label="Höhe (1 bis 255)" type="number" min={1} max={255} value={String(matrix.height)} onCommit={sizer("height")} style={dim} />
            </label>
            <label>
              <input type="checkbox" checked={matrix.headers != null} onChange={(e) => void onSection(matrix, "headers", e.target.checked)} /> Header
            </label>
            <label>
              <input type="checkbox" checked={matrix.altitudes != null} onChange={(e) => void onSection(matrix, "altitudes", e.target.checked)} /> Höhen
            </label>
          </>
        ) : (
          <span style={{ color: "#6b7280" }}>Header wählen, um seine Matrix zu bearbeiten.</span>
        )}
        <button onClick={() => void onNew()} style={{ marginLeft: "auto" }}>
          Neue Matrix
        </button>
      </div>
      <p style={note}>
        Neue Matrix und Kartenkopie werden sofort angelegt; Anlegen lässt sich nicht rückgängig machen. Höhen lassen sich bearbeiten, die 3D-Ansicht wertet sie
        noch nicht aus.
      </p>
      {created !== null && created !== sceneMatrix && (
        <p style={{ ...note, color: "#92400e" }}>Diese Matrix gehört noch zu keinem Header. Im Header-Editor einem Header als Matrix zuweisen, dann zeigt die Szene sie.</p>
      )}
      <div style={{ flex: 1, minHeight: 0, overflow: "auto", padding: "0 12px 12px" }}>
        {matrix && (
          <AreaGrid
            matrix={matrix}
            headers={headers}
            areaFallback={areaDataId}
            cellSize={matrix.width > 12 ? 32 : 64}
            selected={selected}
            colorOf={colorOf}
            onCellClick={(x, y) => goTo({ kind: "matrixCell", matrix: matrix.index, x, y })}
            onCellDoubleClick={(x, y) => {
              const header = matrix.headers?.[y * matrix.width + x];
              if (header !== undefined && matrix.maps[y * matrix.width + x] !== null) goTo({ kind: "matrixCell", matrix: matrix.index, x, y, header });
            }}
            onExtend={(x, y) => setExtendAt({ x, y })}
          />
        )}
      </div>
      {matrix && extendAt && (
        <ExtendAreaDialog
          matrix={matrix}
          x={extendAt.x}
          y={extendAt.y}
          headers={headers}
          counts={counts}
          onDone={(report) => {
            setExtendAt(null);
            if (report) void afterExtend(matrix);
          }}
          onError={(e) => {
            onError(e);
            void afterExtend(matrix);
          }}
        />
      )}
    </div>
  );
});
