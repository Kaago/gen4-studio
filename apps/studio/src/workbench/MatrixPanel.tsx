import { memo, useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { ask } from "../platform/dialogs";
import { createMatrix, renameMatrix, resizeMatrix, setMatrixSections } from "../api";
import type { DraftDto, HeaderSummary, MatrixDto, MemberCounts, ProjectInfo } from "../api";
import { AreaGrid, cellKey } from "./AreaGrid";
import { depthAtLeast } from "../ui/frame/shellState";
import type { Depth } from "../ui/frame/shellState";
import { Button, EmptyState, Notice, NumberField, PanelHead, Switch, TextField } from "../ui/primitives";
import { ExtendAreaDialog } from "./ExtendAreaDialog";
import { resizePreview } from "./matrix";
import styles from "./MatrixPanel.module.css";
import type { NavTarget } from "./navigation";
import { buildPlaces, placeColor } from "./places";
import { TERMS, id4, termShort } from "./terms";
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
  /** The cell shown in the inspector, if any: the panel shows its matrix (a link may lead to a cell of another matrix than the scene's). */
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
  /** Sets the draft: the dialog „Karte bei … anlegen?“ plans its new cell while it is open. */
  onDraft: (draft: DraftDto | null) => void;
}

function sectionHasValues(m: MatrixDto, section: "headers" | "altitudes"): boolean {
  return (m[section] ?? []).some((v) => v !== 0);
}

/** Memoised: renders when `context` changes (the owner keeps it, see `MatrixTab`), not for every render of the panel host. */
export const MatrixPanel = memo(function MatrixPanel({ context }: { context: MatrixPanelContext }) {
  const { t } = useTranslation("workbench");
  const { info, headers, areaDataId, counts, sceneMatrix, revision, selection, depth, goTo, onEdited, onCountsChanged, onError, onDraft } = context;
  // A freshly created matrix is shown here although no header uses it yet; the choice of header
  // (or project) ends that.
  const [created, setCreated] = useState<number | null>(null);
  useEffect(() => setCreated(null), [sceneMatrix, info?.root]);
  // A link to a cell of another matrix ends it too (gate 3d.4 B12b); the cell of the created matrix itself (`goTo` after „Neue Matrix“) does not.
  const selectedMatrix = selection?.matrix;
  useEffect(() => {
    if (selectedMatrix !== undefined) setCreated((made) => (made === selectedMatrix ? made : null));
  }, [selectedMatrix]);
  // A cell chosen by a link („Matrix 0290, Zelle 0, 0“) shows its matrix, also when it is not the scene's (gate 3d.4 B12).
  const index = created ?? selection?.matrix ?? sceneMatrix;
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
      const made = await createMatrix(t("matrixPanel.newName"));
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
      const text = t("matrixPanel.lostCells", { count: lostCells, width, height });
      if (!(await ask(text, { kind: "warning" }))) return;
    }
    await edit(m.index, () => resizeMatrix(m.index, width, height));
  }

  async function onSection(m: MatrixDto, section: "headers" | "altitudes", on: boolean) {
    if (!on && sectionHasValues(m, section)) {
      const text = section === "headers" ? t("matrixPanel.dropHeaders") : t("matrixPanel.dropAltitudes");
      if (!(await ask(text, { kind: "warning" }))) return;
    }
    const headers = section === "headers" ? on : m.headers != null;
    const altitudes = section === "altitudes" ? on : m.altitudes != null;
    await edit(m.index, () => setMatrixSections(m.index, headers, altitudes));
  }

  if (!info) {
    return (
      <div className={styles.panel}>
        <EmptyState what={t("matrixPanel.noProject.what")} why={t("matrixPanel.noProject.why")} />
      </div>
    );
  }

  return (
    <div className={styles.panel}>
      {/* The dock tab says "Raster": the head keeps the matrix's number and the action. */}
      <PanelHead
        subtitle={matrix ? `${id4(matrix.index)} · ${TERMS.matrix.tech}` : undefined}
        actions={
          <Button kind="quiet" size="s" icon="plus" onClick={() => void onNew()}>
            {t("matrixPanel.new")}
          </Button>
        }
      />
      {matrix ? (
        <>
          <div className={styles.fields} title={termShort("matrix")}>
            <TextField label={t("matrixPanel.name")} showLabel value={matrix.name} onCommit={(name) => void edit(matrix.index, () => renameMatrix(matrix.index, name))} />
            <NumberField label={t("matrixPanel.width")} showLabel min={1} max={255} hint={t("matrixPanel.sizeHint")} value={matrix.width} onCommit={(w) => void onResize(matrix, w, matrix.height)} />
            <NumberField label={t("matrixPanel.height")} showLabel min={1} max={255} hint={t("matrixPanel.sizeHint")} value={matrix.height} onCommit={(h) => void onResize(matrix, matrix.width, h)} />
            <Switch label={t("matrixPanel.headers")} checked={matrix.headers != null} onChange={(on) => void onSection(matrix, "headers", on)} />
            <Switch label={t("matrixPanel.altitudes")} checked={matrix.altitudes != null} onChange={(on) => void onSection(matrix, "altitudes", on)} />
          </div>
          {created !== null && created !== sceneMatrix && (
            <div className={styles.notes}>
              <Notice kind="warning">{t("matrixPanel.unassigned")}</Notice>
            </div>
          )}
        </>
      ) : (
        index === null && <EmptyState what={t("matrixPanel.noPlace.what")} why={t("matrixPanel.noPlace.why")} />
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
              tinker={depthAtLeast(depth, "tuefteln")}
              onCellClick={onCellClick}
              onCellDoubleClick={onCellDoubleClick}
              onExtend={onExtend}
            />
            {/* After the grid, so the cells get the room. */}
            <div className={styles.note}>
              <Notice kind="info">{t("matrixPanel.note")}</Notice>
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
          onDraft={onDraft}
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
