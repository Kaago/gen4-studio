import { useEffect, useState } from "react";
import type { CSSProperties } from "react";
import { ask } from "@tauri-apps/plugin-dialog";
import { createMatrix, renameMatrix, resizeMatrix, setMatrixSections } from "../api";
import type { MatrixDto, ProjectInfo } from "../api";
import { DraftInput } from "./DraftInput";
import { cellLabel, parseCellField, resizePreview } from "./matrix";
import { useMatrix } from "./useMatrix";

/** What the matrix panel needs from App. */
export interface MatrixPanelContext {
  info: ProjectInfo | null;
  /** Matrix of the scene (the selected header's); null while no header is selected. */
  sceneMatrix: number | null;
  /** Changes whenever matrix data may have changed behind the panel (edits, undo, redo, reload). */
  revision: number;
  /** The cell shown in the inspector, if any. */
  selection: { matrix: number; x: number; y: number } | null;
  onSelectCell: (matrix: number, x: number, y: number) => void;
  /** A matrix was edited: refresh dirty count and, if it is the scene's matrix, the scene. */
  onEdited: (index: number) => void | Promise<void>;
  /** A member was appended to an archive: refresh the member counts. */
  onCountsChanged: () => void | Promise<void>;
  onError: (e: unknown) => void;
}

const bar: CSSProperties = { display: "flex", flexWrap: "wrap", alignItems: "center", gap: 8, padding: "8px 12px" };
const note: CSSProperties = { margin: 0, padding: "0 12px 8px", fontSize: 12, color: "#6b7280" };
const dim: CSSProperties = { width: 56, textAlign: "right", fontFamily: "ui-monospace, monospace" };

const cellStyle = (selected: boolean, empty: boolean): CSSProperties => ({
  all: "unset",
  boxSizing: "border-box",
  minWidth: 84,
  padding: "3px 6px",
  textAlign: "center",
  cursor: "pointer",
  fontFamily: "ui-monospace, monospace",
  fontSize: 12,
  whiteSpace: "nowrap",
  color: empty ? "#9ca3af" : "#111827",
  background: selected ? "#dbeafe" : "transparent",
  outline: selected ? "2px solid #2563eb" : "1px solid #e5e7eb",
  outlineOffset: -1,
});

function sectionHasValues(m: MatrixDto, section: "headers" | "altitudes"): boolean {
  return (m[section] ?? []).some((v) => v !== 0);
}

export function MatrixPanel({ context }: { context: MatrixPanelContext }) {
  const { info, sceneMatrix, revision, selection, onSelectCell, onEdited, onCountsChanged, onError } = context;
  // A freshly created matrix is shown here although no header uses it yet; the choice of header
  // (or project) ends that.
  const [created, setCreated] = useState<number | null>(null);
  useEffect(() => setCreated(null), [sceneMatrix, info?.root]);
  const index = created ?? sceneMatrix;
  const matrix = useMatrix(index, revision, onError);

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
      onSelectCell(made, 0, 0);
    } catch (e) {
      onError(e);
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
      <div style={bar}>
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
          <table style={{ borderCollapse: "collapse" }} aria-label={`Matrix ${matrix.index}, ${matrix.height} Zeilen, ${matrix.width} Spalten`}>
            <tbody>
              {Array.from({ length: matrix.height }, (_, y) => (
                <tr key={y}>
                  {Array.from({ length: matrix.width }, (_, x) => {
                    const i = y * matrix.width + x;
                    const map = matrix.maps[i] ?? null;
                    const selected = selection?.matrix === matrix.index && selection.x === x && selection.y === y;
                    return (
                      <td key={x} style={{ padding: 0 }}>
                        <button
                          onClick={() => onSelectCell(matrix.index, x, y)}
                          title={`x ${x}, y ${y}${matrix.altitudes ? `, Höhe ${matrix.altitudes[i]}` : ""}`}
                          style={cellStyle(selected, map === null)}
                        >
                          {cellLabel(map, matrix.headers ? (matrix.headers[i] ?? null) : null)}
                        </button>
                      </td>
                    );
                  })}
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>
    </div>
  );
}
