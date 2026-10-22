import { useCallback, useEffect, useRef, useState } from "react";
import type { MutableRefObject } from "react";
import type { DockviewApi } from "dockview-react";
import { errorText, getAreaData, getMatrix, listHeaders } from "../api";
import type { HeaderSummary, MatrixDto } from "../api";
import type { Message } from "../Toolbar";
import { fetchCurrent } from "../reload";
import { tilesetKey } from "../workbench/inspector/areaFields";
import { sameLayout } from "../workbench/matrix";
import { nextSelection } from "../workbench/navigation";
import type { NavTarget } from "../workbench/navigation";
import type { InspectorModel } from "../workbench/Inspector";
import type { PanelId } from "../workbench/layout";
import type { EditBus } from "../bus/editBus";
import { initialSync, isCurrentSelection, sceneSync } from "./sceneSync";
import type { SyncAction, SyncOutcome } from "./sceneSync";

interface SceneSyncParams {
  bus: EditBus;
  setMessage: (message: Message | null) => void;
  fail: (e: unknown) => void;
  refreshDirty: () => Promise<void>;
  setPinned: (pinned: InspectorModel) => void;
  /** The dockview api of the workbench: a link focuses the panel that shows its target. */
  dockApi: MutableRefObject<DockviewApi | null>;
  /** Tells the text panel which archive and message to open (a link to a text archive). */
  onOpenText: (index: number, message?: number) => void;
}

/**
 * The header list, the selected header and the matrix the scene shows, kept consistent by the
 * pure decisions of `sceneSync`: selecting, reading the header list again, refetching the matrix
 * and comparing tilesets. Tells the editors and the scene through `bus` when their data moved.
 */
export function useSceneSync({ bus, setMessage, fail, refreshDirty, setPinned, dockApi, onOpenText }: SceneSyncParams) {
  const [headers, setHeaders] = useState<HeaderSummary[]>([]);
  const [selected, setSelected] = useState<number | null>(null);
  const [matrix, setMatrix] = useState<MatrixDto | null>(null);
  // The area data of the shown scene. It changes together with `matrix` (another header) or alone
  // (the header now uses another area data); alone, the scene reloads only its models.
  const [sceneArea, setSceneArea] = useState(0);

  // The decisions' state lives in a ref: they must see what the previous one did at once, not on
  // the next render. Every change goes through the pure `sceneSync`.
  const sync = useRef(initialSync());
  const dispatch = useCallback((action: SyncAction): SyncOutcome => {
    const result = sceneSync(sync.current, action);
    sync.current = result.state;
    return result.outcome;
  }, []);
  // Bumped by every undo and redo the moment it returns: a matrix fetch or scene load that was in
  // flight meanwhile fetches again. Read like a ref, because state would only change on the next render.
  // The object is created once (it is passed on as the scene's identity), not recomputed like a memo.
  const [history] = useState(() => ({
    get current() {
      return sync.current.history;
    },
  }));
  const noteHistoryMoved = useCallback(() => void dispatch({ type: "historyMoved" }), [dispatch]);

  const selectedRef = useRef<number | null>(null);
  selectedRef.current = selected;
  const matrixRef = useRef<MatrixDto | null>(null);
  matrixRef.current = matrix;
  const headersRef = useRef(headers);
  headersRef.current = headers;
  // The area data of the scene as it is shown right now (the area in `sync` may be ahead of it).
  const shownAreaRef = useRef(0);
  shownAreaRef.current = sceneArea;

  /** The area data the scene should show (ahead of `sceneArea` while a matrix is being fetched). */
  const targetArea = useCallback(() => sync.current.area, []);

  /**
   * Compares the shown area data's tilesets with the ones last seen and, with `reload`, reloads the
   * scene's models when they differ (an edit, undo, redo or reload of the area data). Without
   * `reload` it only records them (the scene was just loaded for them).
   */
  const noteSceneTileset = useCallback(
    async (reload: boolean) => {
      const area = sync.current.area;
      try {
        const key = tilesetKey(await getAreaData(area));
        if (dispatch({ type: "tilesetRead", area, key, reload }).kind === "models") bus.emit({ topic: "modelsReload" });
      } catch (e) {
        if (dispatch({ type: "tilesetFailed", area }).kind === "report") setMessage({ kind: "error", text: `Area-Daten ${area} nicht lesbar: ${errorText(e)}` });
      }
    },
    [bus, dispatch, setMessage],
  );

  /** The matrix of `header`, fetched again while undo or redo lands during the fetch. */
  const fetchMatrix = useCallback((header: HeaderSummary) => fetchCurrent(() => getMatrix(header.matrix_id), () => sync.current.history), []);

  /**
   * Reads the header list again (it is backed by the store, so it shows unsaved edits) and
   * reloads the scene when the selected header now points at another matrix, area data or event file.
   */
  const syncHeaders = useCallback(async () => {
    const started = dispatch({ type: "listStart" });
    const seq = started.kind === "list" ? started.seq : 0;
    try {
      const listing = await listHeaders();
      const header = listing.headers.find((h) => h.id === selectedRef.current);
      const outcome = dispatch({ type: "listed", seq, header });
      if (outcome.kind === "stale") return;
      setHeaders(listing.headers);
      if (outcome.kind === "noHeader") return;
      if (outcome.kind === "area") {
        // Only the area data changed: the scene keeps its maps and reloads its models.
        if (outcome.apply) setSceneArea(outcome.area);
        return;
      }
      if (outcome.kind !== "load" || !header) return;
      const sceneSeq = outcome.seq;
      try {
        const next = await fetchMatrix(header);
        const arrived = dispatch({ type: "loaded", seq: sceneSeq });
        if (arrived.kind === "apply") {
          setMatrix(next);
          setSceneArea(arrived.area);
        }
      } catch (e) {
        dispatch({ type: "loadFailed", seq: sceneSeq, shownArea: shownAreaRef.current });
        throw e;
      } finally {
        dispatch({ type: "loadEnded" });
      }
    } catch (e) {
      setMessage({ kind: "error", text: errorText(e) });
    }
  }, [dispatch, fetchMatrix, setMessage]);

  /**
   * Reads the scene's matrix again and replaces it when it now places other maps or other
   * altitudes (`sameLayout` compares both; a new object makes the scene refetch all its maps,
   * so name and header edits leave it alone).
   * A newer selection or scene load that began meanwhile wins.
   */
  const refreshSceneMatrix = useCallback(async (index?: number) => {
    const shown = matrixRef.current;
    if (!shown || (index !== undefined && index !== shown.index)) return;
    const seq = sync.current.selectSeq;
    const next = await fetchCurrent(() => getMatrix(shown.index), () => sync.current.history);
    if (isCurrentSelection(sync.current, seq) && !sameLayout(shown, next)) setMatrix(next);
  }, []);

  const onHeaderChanged = useCallback(async () => {
    await Promise.all([syncHeaders(), refreshDirty()]);
  }, [syncHeaders, refreshDirty]);

  const onMatrixEdited = useCallback(
    async (index: number) => {
      bus.emit({ topic: "matrixData" });
      try {
        await refreshSceneMatrix(index);
      } catch (e) {
        setMessage({ kind: "error", text: errorText(e) });
      }
      await refreshDirty();
    },
    [bus, refreshSceneMatrix, refreshDirty, setMessage],
  );

  const onAreaChanged = useCallback(async () => {
    await Promise.all([noteSceneTileset(true), refreshDirty()]);
  }, [noteSceneTileset, refreshDirty]);

  // The scene was loaded for this area data and matrix: remember its tilesets (no reload).
  useEffect(() => {
    if (matrix) void noteSceneTileset(false);
  }, [matrix, sceneArea, noteSceneTileset]);

  async function onSelect(h: HeaderSummary) {
    const started = dispatch({ type: "select", header: h });
    const seq = started.kind === "load" ? started.seq : 0;
    setSelected(h.id);
    setPinned({ kind: "header", id: h.id });
    try {
      const next = await fetchMatrix(h);
      const arrived = dispatch({ type: "loaded", seq });
      if (arrived.kind !== "apply") return;
      // Together, so the scene sees a new matrix with its area data and loads everything once.
      setMatrix(next);
      setSceneArea(arrived.area);
      setMessage(null);
    } catch (e) {
      // The matrix fetch failed: the scene stays as it was, and a later sync must not pair its maps with the new header's area data.
      if (dispatch({ type: "loadFailed", seq, shownArea: shownAreaRef.current }).kind === "reset") fail(e);
    } finally {
      dispatch({ type: "loadEnded" });
    }
  }

  function focusPanel(panel: PanelId | null) {
    if (panel === null) return;
    try {
      dockApi.current?.getPanel(panel)?.focus();
    } catch {
      /* the workbench is going away */
    }
  }

  /**
   * Follows a link (see `nextSelection`): a header goes through the same path as a click in the
   * list (its matrix loads under the selection guards), the inspector is pinned to the target and
   * the panel showing it is focused. Resolves when the matrix of a newly selected header has loaded (or failed).
   */
  function goTo(target: NavTarget): Promise<void> | void {
    const shown = matrixRef.current;
    const current = headersRef.current.find((h) => h.id === selectedRef.current);
    const next = nextSelection(
      {
        selectedHeader: selectedRef.current,
        matrix: shown?.index ?? null,
        maps: shown ? shown.maps.filter((m): m is number => m !== null) : [],
        eventFile: current?.event_file_id ?? null,
        headers: headersRef.current,
      },
      target,
    );
    if (next.pinned === null) {
      // A text archive changes neither the selection nor the pin: the text panel opens it and takes the focus.
      if (next.text) {
        onOpenText(next.text.index, next.text.message ?? undefined);
        focusPanel(next.panel);
      } else if (target.kind === "header") {
        setMessage({ kind: "error", text: `Header ${String(target.id).padStart(4, "0")} gibt es nicht.` });
      }
      return;
    }
    if (next.panel === "headers") bus.emit({ topic: "headerReveal" });
    // A header already selected has its matrix loaded: it is only pinned again below.
    let loading: Promise<void> | undefined;
    if (next.selectedHeader !== selectedRef.current) {
      const header = headersRef.current.find((h) => h.id === next.selectedHeader);
      if (header) loading = onSelect(header);
    }
    // After a header's own pin, so a building or event the header brought in wins.
    setPinned(next.pinned);
    focusPanel(next.panel);
    return loading;
  }

  /** Another project was opened: nothing of the old one stays selected, shown or in flight. */
  function resetForProject(listed: HeaderSummary[]) {
    dispatch({ type: "opened" });
    setSceneArea(0);
    setHeaders(listed);
    setSelected(null);
    setMatrix(null);
  }

  return {
    headers,
    selected,
    matrix,
    sceneArea,
    history,
    selectedRef,
    matrixRef,
    headersRef,
    targetArea,
    noteHistoryMoved,
    setMatrix,
    syncHeaders,
    noteSceneTileset,
    refreshSceneMatrix,
    onHeaderChanged,
    onMatrixEdited,
    onAreaChanged,
    onSelect,
    goTo,
    resetForProject,
  };
}

export type SceneSync = ReturnType<typeof useSceneSync>;
