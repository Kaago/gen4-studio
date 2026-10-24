import { memo, useCallback, useEffect, useMemo, useState } from "react";
import { ask } from "../platform/dialogs";
import { createMatrix, renameMatrix, resizeMatrix, setMatrixSections } from "../api";
import type { HeaderSummary, MatrixDto, MemberCounts, ProjectInfo } from "../api";
import { AreaGrid, cellKey } from "./AreaGrid";
import type { Depth } from "../ui/frame/shellState";
import { Button, EmptyState, Notice, NumberField, PanelHead, Switch, TextField } from "../ui/primitives";
import { ExtendAreaDialog } from "./ExtendAreaDialog";
import { resizePreview } from "./matrix";
import styles from "./MatrixPanel.module.css";
import type { NavTarget } from "./navigation";
import { buildPlaces, placeColor } from "./places";
import { TERMS, id4 } from "./terms";
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
  /** The depth of the app: Tüfteln writes the raw values into every cell. */
  depth: Depth;
  /** Follows a link: a click shows the cell in the inspector, a double click on a cell with a header opens the header. */
  goTo: (target: NavTarget) => void;
  /** A matrix was edited: refresh dirty count and, if it is the scene's matrix, the scene. */
  onEdited: (index: number) => void | Promise<void>;
  /** A member was appended to an archive: refresh the member counts. */
  onCountsChanged: () => void | Promise<void>;
  onError: (e: unknown) => void;
}

function sectionHasValues(m: MatrixDto, section: "headers" | "altitudes"): boolean {
  return (m[section] ?? []).some((v) => v !== 0);
}

/** Memoised: renders when `context` changes (the owner keeps it, see `MatrixTab`), not for every render of the panel host. */
export const MatrixPanel = memo(function MatrixPanel({ context }: { context: MatrixPanelContext }) {
  const { info, headers, areaDataId, counts, sceneMatrix, revision, selection, depth, goTo, onEdited, onCountsChanged, onError } = context;
  // A freshly created matrix is shown here although no header uses it yet; the choice of header
  // (or project) ends that.
  const [created, setCreated] = useState<number | null>(null);
  useEffect(() => setCreated(null), [sceneMatrix, info?.root]);
  const index = created ?? sceneMatrix;
  const matrix = useMatrix(index, revision, onError);
  const [extendAt, setExtendAt] = useState<{ x: number; y: number } | null>(null);
  // The dialog belongs to the cell it was opened for: another matrix or project ends it.
  useEffect(() => setExtendAt(null), [index, info?.root]);
  // The place colour of every header, built once per header list (a grid asks for each of its cells).
  const colorByHeader = useMemo(
    () => new Map(buildPlaces(headers, []).flatMap((place) => place.headers.map(({ header }) => [header.id, placeColor(place.key)] as const))),
    [headers],
  );
  const colorOf = useCallback((header: number | null) => (header === null ? null : (colorByHeader.get(header) ?? null)), [colorByHeader]);
  // Stable handlers, so the memoised cells render again only when their own data changes.
  const onCellClick = useCallback(
    (x: number, y: number) => {
      if (matrix) goTo({ kind: "matrixCell", matrix: matrix.index, x, y });
    },
    [matrix, goTo],
  );
  const onCellDoubleClick = useCallback(
    (x: number, y: number) => {
      if (!matrix) return;
      const header = matrix.headers?.[y * matrix.width + x];
      if (header !== undefined && matrix.maps[y * matrix.width + x] !== null) goTo({ kind: "matrixCell", matrix: matrix.index, x, y, header });
    },
    [matrix, goTo],
  );
  const onExtend = useCallback((x: number, y: number) => setExtendAt({ x, y }), []);
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

  if (!info) {
    return (
      <div className={styles.panel}>
        <EmptyState what="Kein Projekt geöffnet." why="Öffne ein Projekt, dann zeigt das Raster seine Karten." />
      </div>
    );
  }

  return (
    <div className={styles.panel}>
      <PanelHead
        title={TERMS.matrix.label}
        subtitle={matrix ? `${id4(matrix.index)} · ${TERMS.matrix.tech}` : undefined}
        actions={
          <Button kind="quiet" size="s" icon="plus" onClick={() => void onNew()}>
            Neue Matrix
          </Button>
        }
      />
      {matrix ? (
        <>
          <div className={styles.fields} title={TERMS.matrix.short}>
            <TextField label="Name der Matrix" showLabel value={matrix.name} onCommit={(name) => void edit(matrix.index, () => renameMatrix(matrix.index, name))} />
            <NumberField label="Breite" showLabel min={1} max={255} hint="1 bis 255" value={matrix.width} onCommit={(w) => void onResize(matrix, w, matrix.height)} />
            <NumberField label="Höhe" showLabel min={1} max={255} hint="1 bis 255" value={matrix.height} onCommit={(h) => void onResize(matrix, matrix.width, h)} />
            <Switch label="Header" checked={matrix.headers != null} onChange={(on) => void onSection(matrix, "headers", on)} />
            <Switch label="Höhen" checked={matrix.altitudes != null} onChange={(on) => void onSection(matrix, "altitudes", on)} />
          </div>
          {created !== null && created !== sceneMatrix && (
            <div className={styles.notes}>
              <Notice kind="warning">Diese Matrix gehört noch zu keinem Header. Im Header-Editor einem Header als Matrix zuweisen, dann zeigt die Schau sie.</Notice>
            </div>
          )}
        </>
      ) : (
        index === null && <EmptyState what="Noch kein Ort gewählt." why="Wähle links einen Ort, dann zeigt das Raster seine Karten." />
      )}
      <div className={styles.grid}>
        {matrix && (
          <>
            <AreaGrid
              matrix={matrix}
              headers={headers}
              areaFallback={areaDataId}
              cellSize={64}
              selected={selected}
              colorOf={colorOf}
              tinker={depth === "tuefteln"}
              onCellClick={onCellClick}
              onCellDoubleClick={onCellDoubleClick}
              onExtend={onExtend}
            />
            {/* After the grid, so the cells get the room. */}
            <div className={styles.note}>
              <Notice kind="info">
                Neue Matrix und Kartenkopie werden sofort angelegt; Anlegen lässt sich nicht rückgängig machen. Höhen lassen sich bearbeiten; eine Stufe hebt die Zelle
                in der 3D-Ansicht um eine halbe Kachel.
              </Notice>
            </div>
          </>
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
