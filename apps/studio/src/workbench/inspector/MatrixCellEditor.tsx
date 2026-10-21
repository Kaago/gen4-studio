import { useState } from "react";
import type { CSSProperties } from "react";
import { MATRIX_EMPTY, duplicateMap, setMatrixCell } from "../../api";
import type { MemberCounts } from "../../api";
import { DraftInput } from "../DraftInput";
import { cellLabel, parseCellField } from "../matrix";
import { useMatrix } from "../useMatrix";

/** What the cell editor needs from App. */
export interface MatrixCellContext {
  /** Number of map headers (the largest header id plus one). */
  headerCount: number;
  counts: MemberCounts;
  /** Changes whenever matrix data may have changed behind the editor (edits, undo, redo, reload). */
  revision: number;
  onEdited: (index: number) => void | Promise<void>;
  onCountsChanged: () => void | Promise<void>;
  onError: (e: unknown) => void;
}

const row: CSSProperties = { display: "grid", gridTemplateColumns: "minmax(0, 1fr) 96px", alignItems: "center", columnGap: 8, padding: "2px 0" };
const input: CSSProperties = { width: "100%", fontFamily: "ui-monospace, monospace" };

/** Editor of one matrix cell: map, header and altitude, each one undoable change. */
export function MatrixCellEditor({ matrix: index, x, y, context }: { matrix: number; x: number; y: number; context: MatrixCellContext }) {
  const { headerCount, counts, revision, onEdited, onCountsChanged, onError } = context;
  const matrix = useMatrix(index, revision, onError);
  const [working, setWorking] = useState(false);

  const title = `Matrix ${String(index).padStart(4, "0")} · Zelle (${x}, ${y})`;
  if (!matrix) return <div style={{ padding: 16, fontSize: 13, color: "#6b7280" }}>{title}</div>;
  if (x >= matrix.width || y >= matrix.height) {
    return (
      <div style={{ padding: 16, fontSize: 13, color: "#6b7280" }}>
        {title}
        <p>Die Zelle liegt außerhalb der Matrix ({matrix.width} × {matrix.height}).</p>
      </div>
    );
  }
  const i = y * matrix.width + x;
  const map = matrix.maps[i] ?? null;
  const header = matrix.headers ? (matrix.headers[i] ?? 0) : null;
  const altitude = matrix.altitudes ? (matrix.altitudes[i] ?? 0) : null;
  const mapMax = Math.max(0, (counts.Map ?? 1) - 1);

  /** Writes the changed field of this cell; always refreshes afterwards, also after a refusal. */
  async function run(op: () => Promise<unknown>, alsoCounts = false) {
    setWorking(true);
    try {
      await op();
    } catch (e) {
      onError(e);
    } finally {
      try {
        if (alsoCounts) await onCountsChanged();
        await onEdited(index);
      } finally {
        setWorking(false);
      }
    }
  }

  const commitMap = (text: string) => {
    const v = parseCellField(text, mapMax, true);
    if (v === null) return;
    const next = v === "empty" ? MATRIX_EMPTY : v;
    if (next !== (map ?? MATRIX_EMPTY)) void run(() => setMatrixCell(index, x, y, next, null, null));
  };
  const commitHeader = (text: string) => {
    const v = parseCellField(text, Math.max(0, headerCount - 1), false);
    if (typeof v === "number" && v !== header) void run(() => setMatrixCell(index, x, y, null, v, null));
  };
  const commitAltitude = (text: string) => {
    const v = parseCellField(text, 255, false);
    if (typeof v === "number" && v !== altitude) void run(() => setMatrixCell(index, x, y, null, null, v));
  };

  // The copy is appended first; if placing it fails, the (already written) copy is still reported through the counts.
  const duplicate = (source: number) =>
    run(async () => {
      const copy = await duplicateMap(source);
      await setMatrixCell(index, x, y, copy, null, null);
    }, true);

  return (
    <div style={{ padding: 16, fontFamily: "system-ui, sans-serif", fontSize: 13, color: "#111827" }}>
      <div style={{ fontSize: 14, fontWeight: 600 }}>{title}</div>
      <div style={{ fontSize: 12, color: "#6b7280" }}>{cellLabel(map, header)}</div>
      <div style={{ marginTop: 8 }}>
        <label style={row}>
          <span>Karte</span>
          <DraftInput
            label={`Karte (0 bis ${mapMax}, leer = keine Karte)`}
            type="number"
            min={0}
            max={mapMax}
            placeholder="leer"
            value={map === null ? "" : String(map)}
            onCommit={commitMap}
            style={input}
          />
        </label>
        {header !== null ? (
          <label style={row}>
            <span>Header</span>
            <DraftInput
              label={`Header (0 bis ${Math.max(0, headerCount - 1)})`}
              type="number"
              min={0}
              max={Math.max(0, headerCount - 1)}
              value={String(header)}
              onCommit={commitHeader}
              style={input}
            />
          </label>
        ) : (
          <p style={{ margin: "4px 0", fontSize: 12, color: "#6b7280" }}>Die Matrix hat keinen Header-Abschnitt (im Matrix-Panel einschaltbar).</p>
        )}
        {altitude !== null ? (
          <label style={row}>
            <span>Höhe</span>
            <DraftInput label="Höhe (0 bis 255)" type="number" min={0} max={255} value={String(altitude)} onCommit={commitAltitude} style={input} />
          </label>
        ) : (
          <p style={{ margin: "4px 0", fontSize: 12, color: "#6b7280" }}>Die Matrix hat keinen Höhenabschnitt (im Matrix-Panel einschaltbar).</p>
        )}
      </div>
      <button disabled={map === null || working} onClick={() => map !== null && void duplicate(map)} style={{ marginTop: 8 }}>
        Karte duplizieren und hier eintragen
      </button>
      <p style={{ marginTop: 8, fontSize: 12, color: "#6b7280" }}>Anlegen lässt sich nicht rückgängig machen. Die Kopie ist sofort als neue Karte am Ende des Archivs vorhanden.</p>
    </div>
  );
}
