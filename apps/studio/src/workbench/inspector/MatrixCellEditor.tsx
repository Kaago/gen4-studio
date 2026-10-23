import { useEffect, useState } from "react";
import { MATRIX_EMPTY, duplicateMap, setMatrixCell } from "../../api";
import type { MemberCounts } from "../../api";
import { Button, InspectorRow, NumberField, Reference, TextField, checkNumber } from "../../ui/primitives";
import { cellLabel } from "../matrix";
import type { NavTarget } from "../navigation";
import { TERMS, id4 } from "../terms";
import { useMatrix } from "../useMatrix";
import styles from "./Editor.module.css";
import { InspectorHead } from "./InspectorHead";

/** What the cell editor needs from App. */
export interface MatrixCellContext {
  /** Number of map headers (the largest header id plus one). */
  headerCount: number;
  /** Follows a link (the cell's header). */
  goTo: (target: NavTarget) => void;
  counts: MemberCounts;
  /** Changes whenever matrix data may have changed behind the editor (edits, undo, redo, reload). */
  revision: number;
  onEdited: (index: number) => void | Promise<void>;
  onCountsChanged: () => void | Promise<void>;
  onError: (e: unknown) => void;
}

/** The map of a cell: a number from 0 to `max`, or nothing for a cell without a map. An entry outside that is refused with its limit. */
function MapField({ map, max, onCommit }: { map: number | null; max: number; onCommit: (next: number | "empty") => void }) {
  const [error, setError] = useState<string | undefined>();
  // A new value from the matrix makes an old refusal moot.
  useEffect(() => setError(undefined), [map]);
  return (
    <TextField
      label="Karte"
      placeholder="leer"
      value={map === null ? "" : String(map)}
      error={error}
      onCommit={(text) => {
        if (text.trim() === "") {
          setError(undefined);
          return onCommit("empty");
        }
        const checked = checkNumber(text, "dec", 0, max);
        if (!checked.ok) return setError(checked.error);
        setError(undefined);
        onCommit(checked.value);
      }}
    />
  );
}

/** Editor of one matrix cell: map, header and altitude, each one undoable change. */
export function MatrixCellEditor({ matrix: index, x, y, context }: { matrix: number; x: number; y: number; context: MatrixCellContext }) {
  const { headerCount, goTo, counts, revision, onEdited, onCountsChanged, onError } = context;
  const matrix = useMatrix(index, revision, onError);
  const [working, setWorking] = useState(false);

  const title = `Zelle ${x}, ${y}`;
  const matrixName = `${TERMS.matrix.label} ${id4(index)}`;
  if (!matrix) {
    return (
      <div className={styles.panel}>
        <InspectorHead title={title} subtitle={matrixName} />
      </div>
    );
  }
  if (x >= matrix.width || y >= matrix.height) {
    return (
      <div className={styles.panel}>
        <InspectorHead title={title} subtitle={matrixName} />
        <p className={styles.note}>
          Die Zelle liegt außerhalb des Rasters ({matrix.width} × {matrix.height}).
        </p>
      </div>
    );
  }
  const i = y * matrix.width + x;
  const map = matrix.maps[i] ?? null;
  const header = matrix.headers ? (matrix.headers[i] ?? 0) : null;
  const altitude = matrix.altitudes ? (matrix.altitudes[i] ?? 0) : null;
  const mapMax = Math.max(0, (counts.Map ?? 1) - 1);
  const headerMax = Math.max(0, headerCount - 1);

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

  const commitMap = (v: number | "empty") => {
    const next = v === "empty" ? MATRIX_EMPTY : v;
    if (next !== (map ?? MATRIX_EMPTY)) void run(() => setMatrixCell(index, x, y, next, null, null));
  };
  const commitHeader = (v: number) => void run(() => setMatrixCell(index, x, y, null, v, null));
  const commitAltitude = (v: number) => void run(() => setMatrixCell(index, x, y, null, null, v));

  // The copy is appended first; if placing it fails, the (already written) copy is still reported through the counts.
  const duplicate = (source: number) =>
    run(async () => {
      const copy = await duplicateMap(source);
      await setMatrixCell(index, x, y, copy, null, null);
    }, true);

  return (
    <div className={styles.panel}>
      <InspectorHead title={title} subtitle={`${matrixName} · ${cellLabel(map, header)}`} />
      <div className={styles.body}>
        <InspectorRow label="Karte" hint={`0 bis ${mapMax}, leer = keine Karte`}>
          <MapField map={map} max={mapMax} onCommit={commitMap} />
        </InspectorRow>
        {header !== null ? (
          <InspectorRow label="Header" hint={`0 bis ${headerMax}`}>
            <div className={styles.inline}>
              <div className={styles.grow}>
                <NumberField label="Header" value={header} max={headerMax} bits={16} onCommit={commitHeader} />
              </div>
              {map !== null && header < headerCount && <Reference area="welt" label="Header" tech={id4(header)} onOpen={() => goTo({ kind: "header", id: header })} />}
            </div>
          </InspectorRow>
        ) : (
          <p className={styles.note}>Das Raster hat keinen Header-Abschnitt (in der Schau unter „Raster“ einschaltbar).</p>
        )}
        {altitude !== null ? (
          <InspectorRow label="Höhe" hint="0 bis 255">
            <NumberField label="Höhe" value={altitude} max={255} bits={8} onCommit={commitAltitude} />
          </InspectorRow>
        ) : (
          <p className={styles.note}>Das Raster hat keinen Höhenabschnitt (in der Schau unter „Raster“ einschaltbar).</p>
        )}
        <div className={styles.actions}>
          <Button kind="secondary" disabled={map === null || working} onClick={() => map !== null && void duplicate(map)}>
            Karte duplizieren und hier eintragen
          </Button>
        </div>
        <p className={styles.note}>Anlegen lässt sich nicht rückgängig machen. Die Kopie ist sofort als neue Karte am Ende des Archivs vorhanden.</p>
      </div>
    </div>
  );
}
