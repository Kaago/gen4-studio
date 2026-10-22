import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { ask, open, save as saveDialog } from "@tauri-apps/plugin-dialog";
import {
  acceptDisk,
  addBuilding,
  addEvent,
  buildRom,
  deleteBuilding,
  deleteEvent,
  dirty,
  errorText,
  getAreaData,
  getEvents,
  getMap,
  getMatrix,
  isCmdError,
  listHeaders,
  memberCounts,
  openProject,
  redo,
  reloadAsset,
  runRom,
  save,
  undo,
  updateBuilding,
} from "./api";
import type { AssetId, EventKind, HeaderListing, HeaderSummary, MatrixDto, MemberCounts, ProjectInfo } from "./api";
import { staleText } from "./assets";
import { Toolbar } from "./Toolbar";
import type { Message } from "./Toolbar";
import Workbench from "./workbench/Workbench";
import type { WorkbenchData } from "./workbench/Workbench";
import type { InspectorModel } from "./workbench/Inspector";
import { tilesetKey } from "./workbench/inspector/areaFields";
import { afterFailedSceneLoad, fetchCurrent, headerSync, reloadPlan, sceneKey, tilesetReload } from "./reload";
import { enqueue } from "./reload";
import type { RefreshQueue, ReloadAction } from "./reload";
import { sameLayout } from "./workbench/matrix";
import { pinAfterHistory, shownInspector } from "./workbench/pin";
import type { BuildingRef } from "./world/buildingLayer";
import { deletePrompt, isTypingTarget, newBuildingPosition, wantsDelete } from "./world/buildings";
import type { BuildingRefresh } from "./world/buildings";
import { eventAt, eventDeletePrompt, eventEditGate, eventReadOnlyReason, lockedText } from "./world/events";
import type { EventLock, EventRef, EventRefresh } from "./world/events";

const EMULATOR_KEY = "g4.emulatorPath";

function storedEmulator(): string | null {
  try {
    return localStorage.getItem(EMULATOR_KEY);
  } catch {
    return null;
  }
}

function storeEmulator(path: string) {
  try {
    localStorage.setItem(EMULATOR_KEY, path);
  } catch {
    /* the choice is simply asked again next time */
  }
}

export default function App() {
  const [info, setInfo] = useState<ProjectInfo | null>(null);
  const [headers, setHeaders] = useState<HeaderSummary[]>([]);
  const [selected, setSelected] = useState<number | null>(null);
  const [matrix, setMatrix] = useState<MatrixDto | null>(null);
  const [showPermissions, setShowPermissions] = useState(true);
  const [showEvents, setShowEvents] = useState(true);
  const [message, setMessage] = useState<Message | null>(null);
  const [dirtyCount, setDirtyCount] = useState(0);
  const [mapRefresh, setMapRefresh] = useState<RefreshQueue<number> | null>(null);
  // Bumped by every undo and redo the moment it returns: a matrix fetch or scene load that was in
  // flight meanwhile fetches again. A ref, because state would only change on the next render.
  const history = useRef(0);
  const [busy, setBusy] = useState(false);
  const [lastRom, setLastRom] = useState<string | null>(null);
  // The selected header is pinned in the inspector; the scene's hover overrides it only while the
  // pointer is over a cell or marker.
  const [pinned, setPinned] = useState<InspectorModel>({ kind: "none" });
  const [hover, setHover] = useState<InspectorModel | null>(null);
  // A selected building or event stays in the inspector while the pointer is over the scene (it is
  // being dragged there); other event markers still take over.
  const inspector = shownInspector(pinned, hover);
  // The tile the pointer was last over: where "Neues Gebäude" puts a building.
  const lastTile = useRef<{ map: number; x: number; y: number } | null>(null);
  // The same tile as a position in the whole matrix, where "Hinzufügen" puts an event. Unlike the
  // one above it is forgotten when the scene gets another matrix (the position is only valid in one).
  const lastMatrixTile = useRef<{ x: number; z: number } | null>(null);
  const onInspect = useCallback((model: InspectorModel) => {
    if (model.kind === "tile") {
      lastTile.current = { map: model.map, x: model.x, y: model.y };
      lastMatrixTile.current = { x: model.absX, z: model.absZ };
    }
    setHover(model.kind === "none" ? null : model);
  }, []);
  const pinnedRef = useRef(pinned);
  pinnedRef.current = pinned;
  const [buildingRevision, setBuildingRevision] = useState(0);
  const [buildingRefresh, setBuildingRefresh] = useState<RefreshQueue<BuildingRefresh> | null>(null);
  const [eventRevision, setEventRevision] = useState(0);
  const [eventRefresh, setEventRefresh] = useState<EventRefresh | null>(null);
  // Whether the shown event file is locked: reported by the scene whenever it reads events, the one
  // source for every place that disables adding, deleting or dragging.
  const [eventLock, setEventLock] = useState<EventLock | null>(null);
  const eventLockRef = useRef(eventLock);
  eventLockRef.current = eventLock;
  const onEventsLocked = useCallback((lock: EventLock | null) => {
    setEventLock((previous) => (previous?.file === lock?.file && previous?.reason === lock?.reason ? previous : lock));
  }, []);
  const [counts, setCounts] = useState<MemberCounts>({});
  const [headerRevision, setHeaderRevision] = useState(0);
  const [matrixRevision, setMatrixRevision] = useState(0);
  const [areaRevision, setAreaRevision] = useState(0);
  // The area data of the shown scene. It changes together with `matrix` (another header) or alone
  // (the header now uses another area data); alone, the scene reloads only its models.
  const [sceneArea, setSceneArea] = useState(0);
  // The area data of the scene as it is shown right now (`sceneAreaRef` may be ahead of it).
  const shownAreaRef = useRef(0);
  shownAreaRef.current = sceneArea;
  // Incremented when the tilesets of the shown area data changed: the scene reloads its models.
  const [modelsToken, setModelsToken] = useState(0);

  // The close guard is registered once, so it reads the dirty count through a ref.
  const dirtyRef = useRef(0);
  dirtyRef.current = dirtyCount;

  const refreshDirty = useCallback(async () => {
    try {
      setDirtyCount((await dirty()).length);
    } catch (e) {
      setMessage({ kind: "error", text: errorText(e) });
    }
  }, []);

  // Stable so the memoised scene is not re-rendered by every hover-driven inspector update.
  const onEdited = useCallback(() => void refreshDirty(), [refreshDirty]);

  // The latest header selection (or project switch) wins: a slow getMatrix of an earlier one is
  // dropped, so the matrix never pairs with another header's event file and area data.
  const selectSeq = useRef(0);
  const selectedRef = useRef<number | null>(null);
  selectedRef.current = selected;
  // The matrix and event file the shown scene was loaded for (see `sceneKey`).
  const sceneKeyRef = useRef<string | null>(null);
  const listSeq = useRef(0);
  // The area data the scene should show (ahead of `sceneArea` while a matrix is being fetched).
  const sceneAreaRef = useRef(0);
  // Scene matrices being fetched; `sceneArea` is only changed alone while none is, so it never pairs with the old matrix.
  const sceneLoads = useRef(0);
  // The tilesets of the area data the scene's models were requested with.
  const tilesetRef = useRef<{ area: number; key: string } | null>(null);

  /**
   * Compares the shown area data's tilesets with the ones last seen and, with `reload`, reloads the
   * scene's models when they differ (an edit, undo, redo or reload of the area data). Without
   * `reload` it only records them (the scene was just loaded for them).
   */
  const noteSceneTileset = useCallback(async (reload: boolean) => {
    const area = sceneAreaRef.current;
    try {
      const key = tilesetKey(await getAreaData(area));
      // Another header was selected meanwhile; its own note follows.
      if (area !== sceneAreaRef.current) return;
      const known = tilesetRef.current;
      tilesetRef.current = { area, key };
      if (reload && tilesetReload(known, area, key)) setModelsToken((t) => t + 1);
    } catch (e) {
      // Nothing to compare: the next successful read reloads the models if it is asked to.
      tilesetRef.current = null;
      if (area === sceneAreaRef.current) setMessage({ kind: "error", text: `Area-Daten ${area} nicht lesbar: ${errorText(e)}` });
    }
  }, []);

  /** The matrix fetch for the selected header failed: the scene stays as it was, and a later sync must not pair its maps with the new header's area data. */
  const resetSceneRefs = useCallback(() => {
    const reset = afterFailedSceneLoad(shownAreaRef.current);
    sceneKeyRef.current = reset.key;
    sceneAreaRef.current = reset.area;
  }, []);

  /**
   * Reads the header list again (it is backed by the store, so it shows unsaved edits) and
   * reloads the scene when the selected header now points at another matrix, area data or event file.
   */
  const syncHeaders = useCallback(async () => {
    const seq = ++listSeq.current;
    try {
      const listing = await listHeaders();
      if (seq !== listSeq.current) return;
      setHeaders(listing.headers);
      const header = listing.headers.find((h) => h.id === selectedRef.current);
      if (!header) return;
      sceneAreaRef.current = header.area_data_id;
      if (headerSync(sceneKeyRef.current, header) === "area") {
        // Only the area data changed: the scene keeps its maps and reloads its models.
        if (sceneLoads.current === 0) setSceneArea(header.area_data_id);
        return;
      }
      sceneKeyRef.current = sceneKey(header);
      const sceneSeq = ++selectSeq.current;
      sceneLoads.current++;
      try {
        const next = await fetchCurrent(() => getMatrix(header.matrix_id), () => history.current);
        if (sceneSeq === selectSeq.current) {
          setMatrix(next);
          setSceneArea(sceneAreaRef.current);
        }
      } catch (e) {
        if (sceneSeq === selectSeq.current) resetSceneRefs();
        throw e;
      } finally {
        sceneLoads.current--;
      }
    } catch (e) {
      setMessage({ kind: "error", text: errorText(e) });
    }
  }, [resetSceneRefs]);

  /** Reloads the shown matrix (a new object makes the scene refetch all its maps), the headers and the dirty count. */
  const refreshAfterStale = useCallback(
    async (matrixIndex: number | null) => {
      try {
        if (matrixIndex !== null) setMatrix(await getMatrix(matrixIndex));
        setHeaderRevision((r) => r + 1);
        setMatrixRevision((r) => r + 1);
        setAreaRevision((r) => r + 1);
        setBuildingRevision((r) => r + 1);
        setEventRevision((r) => r + 1);
        // The reload may have re-ordered the buildings or events: a pinned one is bound by index, so let go of it.
        setPinned((p) => pinAfterHistory(p, selectedRef.current));
        await syncHeaders();
        await noteSceneTileset(true);
      } finally {
        // The dirty count must follow the reload/accept even when refetching the matrix fails.
        await refreshDirty();
      }
    },
    [refreshDirty, syncHeaders, noteSceneTileset],
  );

  const onHeaderChanged = useCallback(async () => {
    await Promise.all([syncHeaders(), refreshDirty()]);
  }, [syncHeaders, refreshDirty]);

  // The scene's progress line ("Modelle 37/900") lives in the message bar, but must not wipe
  // an unrelated message: progress only replaces nothing or its own earlier line, and clearing
  // only removes its own line. A failure summary (kind "error") always shows.
  const loadMessage = useRef<Message | null>(null);
  const onLoadStatus = useCallback((next: Message | null) => {
    const previous = loadMessage.current;
    loadMessage.current = next;
    setMessage((shown) => {
      const own = shown === null || shown === previous;
      if (next === null) return own ? null : shown;
      return own || next.kind === "error" ? next : shown;
    });
  }, []);

  const matrixIndexRef = useRef<number | null>(null);
  matrixIndexRef.current = matrix?.index ?? null;
  const matrixRef = useRef<MatrixDto | null>(null);
  matrixRef.current = matrix;
  const headersRef = useRef(headers);
  headersRef.current = headers;
  useEffect(() => {
    lastMatrixTile.current = null;
  }, [matrix]);

  /**
   * Reads the scene's matrix again and replaces it when it now places other maps (a new object
   * makes the scene refetch all its maps, so name, header and altitude edits leave it alone).
   * A newer selection or scene load that began meanwhile wins.
   */
  const refreshSceneMatrix = useCallback(async (index?: number) => {
    const shown = matrixRef.current;
    if (!shown || (index !== undefined && index !== shown.index)) return;
    const seq = selectSeq.current;
    const next = await fetchCurrent(() => getMatrix(shown.index), () => history.current);
    if (seq === selectSeq.current && !sameLayout(shown, next)) setMatrix(next);
  }, []);

  const fail = useCallback(
    (e: unknown) => {
      // A locked event file says why it cannot be changed, in the words the inspector uses.
      const locked = eventReadOnlyReason(e);
      if (locked !== null) {
        setMessage({ kind: "error", text: lockedText(locked) });
        return;
      }
      if (isCmdError(e) && e.code === "stale" && e.asset) {
        const asset: AssetId = e.asset;
        const resolve = (action: (a: AssetId) => Promise<void>, done: string) => async () => {
          try {
            await action(asset);
            await refreshAfterStale(matrixIndexRef.current);
            setMessage({ kind: "info", text: done });
          } catch (err) {
            setMessage({ kind: "error", text: errorText(err) });
          }
        };
        setMessage({
          kind: "error",
          text: staleText(asset),
          actions: [
            { label: "Neu laden (eigene Änderung verwerfen)", run: () => void resolve(reloadAsset, "Von Platte neu geladen.")() },
            { label: "Meine Version behalten", run: () => void resolve(acceptDisk, "Eigene Version behalten, bitte erneut speichern.")() },
          ],
        });
        return;
      }
      setMessage({ kind: "error", text: errorText(e) });
    },
    [refreshAfterStale],
  );

  /** Shows the header again in the inspector when a building or event was pinned (it was deselected or deleted). */
  const unpinSelection = useCallback(() => {
    setPinned((p) => pinAfterHistory(p, selectedRef.current));
  }, []);
  const onSelectBuilding = useCallback(
    (ref: BuildingRef | null) => (ref ? setPinned({ kind: "building", map: ref.map, index: ref.index }) : unpinSelection()),
    [unpinSelection],
  );
  const onSelectEvent = useCallback(
    (ref: EventRef | null) => (ref ? setPinned({ kind: "event", eventFile: ref.file, eventKind: ref.kind, index: ref.index }) : unpinSelection()),
    [unpinSelection],
  );
  // Stable while the same event stays pinned, so the memoised scene is not disturbed by hover updates.
  const selectedEvent = useMemo<EventRef | null>(
    () => (pinned.kind === "event" ? { file: pinned.eventFile, kind: pinned.eventKind, index: pinned.index } : null),
    [pinned],
  );
  const requestEventRefresh = useCallback((file: number) => {
    setEventRefresh((previous) => ({ file, token: (previous?.token ?? 0) + 1 }));
  }, []);
  // The scene wrote a drag: only the event editor (and the dirty count) has to catch up.
  const onEventMoved = useCallback(async () => {
    setEventRevision((r) => r + 1);
    await refreshDirty();
  }, [refreshDirty]);
  // The event editor wrote a field: the scene reads that file's events again.
  const onEventChanged = useCallback(
    async (file: number) => {
      requestEventRefresh(file);
      await refreshDirty();
    },
    [requestEventRefresh, refreshDirty],
  );
  const requestBuildingRefresh = useCallback((map: number, retryModel: number | null) => {
    setBuildingRefresh((previous) => enqueue(previous, { map, retryModel }));
  }, []);
  // The scene wrote a drag: only the building editor (and the dirty count) has to catch up.
  const onBuildingMoved = useCallback(async () => {
    setBuildingRevision((r) => r + 1);
    await refreshDirty();
  }, [refreshDirty]);
  // The building editor wrote a field: the scene reads that map's buildings again.
  const onBuildingChanged = useCallback(
    async (map: number, retryModel: number | null) => {
      requestBuildingRefresh(map, retryModel);
      await refreshDirty();
    },
    [requestBuildingRefresh, refreshDirty],
  );
  // A new building goes on the tile the pointer was last over (the map centre of `fallbackMap` when
  // that is unknown); it is selected afterwards. Adding and moving are two undoable changes.
  const onAddBuilding = useCallback(
    async (fallbackMap: number | null) => {
      const shown = matrixRef.current;
      if (!shown) return;
      const hovered = lastTile.current && shown.maps.includes(lastTile.current.map) ? lastTile.current : null;
      const map = hovered?.map ?? fallbackMap;
      if (map === null) {
        setMessage({ kind: "info", text: "Erst mit der Maus über die Kachel fahren, auf der das Gebäude entstehen soll." });
        return;
      }
      let added: number | null = null;
      try {
        added = await addBuilding(map);
        if (hovered) {
          const at = newBuildingPosition(hovered.x, hovered.y);
          await updateBuilding(map, added, { x: at.x, z: at.z });
        }
      } catch (e) {
        fail(e);
      }
      if (added !== null) setPinned({ kind: "building", map, index: added });
      requestBuildingRefresh(map, null);
      setBuildingRevision((r) => r + 1);
      await refreshDirty();
    },
    [fail, requestBuildingRefresh, refreshDirty],
  );

  const onDeleteBuilding = useCallback(
    async (map: number, index: number) => {
      try {
        // The building is named by model and position, not only by an index that shifts.
        const building = await getMap(map).then((m) => m.buildings[index], () => undefined);
        if (!(await ask(deletePrompt(map, index, building), { kind: "warning" }))) return;
        await deleteBuilding(map, index);
      } catch (e) {
        fail(e);
      }
      // The ones after a deleted building move up by one: nothing stays selected.
      unpinSelection();
      requestBuildingRefresh(map, null);
      setBuildingRevision((r) => r + 1);
      await refreshDirty();
    },
    [fail, unpinSelection, requestBuildingRefresh, refreshDirty],
  );

  // A new event goes on the tile the pointer was last over, in the event file of the selected header;
  // it is selected afterwards.
  const onAddEvent = useCallback(
    async (kind: EventKind) => {
      const shown = matrixRef.current;
      const file = headersRef.current.find((h) => h.id === selectedRef.current)?.event_file_id ?? null;
      if (!shown || file === null) return;
      const gate = eventEditGate(eventLockRef.current, file);
      if (!gate.allowed) {
        setMessage({ kind: "error", text: gate.text });
        return;
      }
      const tile = lastMatrixTile.current;
      if (!tile) {
        setMessage({ kind: "info", text: "Erst mit der Maus über die Kachel fahren, auf der das Event entstehen soll." });
        return;
      }
      try {
        const index = await addEvent(file, kind, tile.x, tile.z);
        setPinned({ kind: "event", eventFile: file, eventKind: kind, index });
      } catch (e) {
        fail(e);
      }
      requestEventRefresh(file);
      setEventRevision((r) => r + 1);
      await refreshDirty();
    },
    [fail, requestEventRefresh, refreshDirty],
  );

  const onDeleteEvent = useCallback(
    async (file: number, kind: EventKind, index: number) => {
      // A locked file cannot lose events: no question, just the reason.
      const gate = eventEditGate(eventLockRef.current, file);
      if (!gate.allowed) {
        setMessage({ kind: "error", text: gate.text });
        return;
      }
      try {
        // The event is named by class, index and position, not only by an index that shifts.
        const event = await getEvents(file).then((events) => eventAt(events, kind, index), () => undefined);
        if (!(await ask(eventDeletePrompt(kind, index, event), { kind: "warning" }))) return;
        await deleteEvent(file, kind, index);
      } catch (e) {
        fail(e);
      }
      // The ones of its class after a deleted event move up by one: nothing stays selected.
      unpinSelection();
      requestEventRefresh(file);
      setEventRevision((r) => r + 1);
      await refreshDirty();
    },
    [fail, unpinSelection, requestEventRefresh, refreshDirty],
  );

  const onMatrixEdited = useCallback(
    async (index: number) => {
      setMatrixRevision((r) => r + 1);
      try {
        await refreshSceneMatrix(index);
      } catch (e) {
        setMessage({ kind: "error", text: errorText(e) });
      }
      await refreshDirty();
    },
    [refreshSceneMatrix, refreshDirty],
  );

  const onAreaChanged = useCallback(async () => {
    await Promise.all([noteSceneTileset(true), refreshDirty()]);
  }, [noteSceneTileset, refreshDirty]);

  // The scene was loaded for this area data and matrix: remember its tilesets (no reload).
  useEffect(() => {
    if (matrix) void noteSceneTileset(false);
  }, [matrix, sceneArea, noteSceneTileset]);

  // New members change the upper bounds of the header editor's map and matrix fields.
  const onCountsChanged = useCallback(async () => {
    try {
      setCounts(await memberCounts());
    } catch (e) {
      setMessage({ kind: "error", text: errorText(e) });
    }
  }, []);

  async function onOpen() {
    try {
      if (dirtyCount > 0 && !(await ask("Ungespeicherte Änderungen verwerfen?", { kind: "warning" }))) return;
      const path = await open({ directory: true });
      if (typeof path !== "string") return;
      const opened = await openProject(path);
      // The backend now has the new project. Fetch its listing first, then swap the whole UI
      // state at once; if the listing fails the UI shows the new, empty project instead of
      // keeping the old project's headers and matrix.
      let listing: HeaderListing | null = null;
      let listError: unknown = null;
      try {
        listing = await listHeaders();
      } catch (e) {
        listError = e;
      }
      let loadedCounts: MemberCounts = {};
      let countsError: unknown = null;
      try {
        loadedCounts = await memberCounts();
      } catch (e) {
        countsError = e;
      }
      selectSeq.current++;
      listSeq.current++;
      sceneKeyRef.current = null;
      sceneAreaRef.current = 0;
      tilesetRef.current = null;
      setSceneArea(0);
      setHeaders(listing?.headers ?? []);
      setCounts(loadedCounts);
      setInfo(opened);
      setSelected(null);
      setPinned({ kind: "none" });
      setHover(null);
      lastTile.current = null;
      lastMatrixTile.current = null;
      setBuildingRefresh(null);
      setEventRefresh(null);
      setMapRefresh(null);
      setEventLock(null);
      setMatrix(null);
      setLastRom(null);
      if (listError !== null) fail(listError);
      else if (countsError !== null) fail(countsError);
      else setMessage(listing?.names_error ? { kind: "error", text: `Ortsnamen nicht lesbar: ${listing.names_error}` } : null);
      await refreshDirty();
    } catch (e) {
      fail(e);
    }
  }

  async function onSelect(h: HeaderSummary) {
    const seq = ++selectSeq.current;
    setSelected(h.id);
    setPinned({ kind: "header", id: h.id });
    sceneKeyRef.current = sceneKey(h);
    sceneAreaRef.current = h.area_data_id;
    sceneLoads.current++;
    try {
      const next = await fetchCurrent(() => getMatrix(h.matrix_id), () => history.current);
      if (seq !== selectSeq.current) return;
      // Together, so the scene sees a new matrix with its area data and loads everything once.
      setMatrix(next);
      setSceneArea(sceneAreaRef.current);
      setMessage(null);
    } catch (e) {
      if (seq === selectSeq.current) {
        resetSceneRefs();
        fail(e);
      }
    } finally {
      sceneLoads.current--;
    }
  }

  /** Carries out one action of a `reloadPlan`. */
  async function reloadAction(a: ReloadAction) {
    switch (a.kind) {
      case "unpin":
        // A pinned building or event is bound by index, which undo and redo can shift (undoing a
        // delete re-inserts one in front), so it would silently become another one: let go of it.
        setPinned((p) => pinAfterHistory(p, selectedRef.current));
        break;
      case "buildings":
        setBuildingRevision((r) => r + 1);
        break;
      case "map":
        setMapRefresh((previous) => enqueue(previous, a.map));
        break;
      case "matrixPanel":
        setMatrixRevision((r) => r + 1);
        break;
      case "matrix":
        await refreshSceneMatrix(matrixRef.current?.index);
        break;
      case "eventsPanel":
        setEventRevision((r) => r + 1);
        break;
      case "events":
        requestEventRefresh(a.file);
        break;
      case "areaPanel":
        setAreaRevision((r) => r + 1);
        break;
      case "models":
        await noteSceneTileset(true);
        break;
      case "headers":
        setHeaderRevision((r) => r + 1);
        await syncHeaders();
        break;
    }
  }

  /** Reloads what the restored asset affects (see `reloadPlan`), nothing else. */
  async function reloadAfterHistory(asset: AssetId) {
    const shown = matrixRef.current;
    const plan = reloadPlan(asset, {
      matrix: shown?.index ?? null,
      maps: shown?.maps ?? [],
      area: sceneAreaRef.current,
      eventFile: headersRef.current.find((h) => h.id === selectedRef.current)?.event_file_id ?? null,
    });
    // One failing reload must not keep the others from running; the first failure is reported.
    let failure: unknown = null;
    for (const action of plan) {
      try {
        await reloadAction(action);
      } catch (e) {
        failure ??= e;
      }
    }
    if (failure !== null) throw failure;
  }

  async function step(action: typeof undo) {
    if (!info || busy) return;
    try {
      const asset = await action();
      if (asset) {
        history.current++;
        await reloadAfterHistory(asset);
      }
    } catch (e) {
      fail(e);
    }
    await refreshDirty();
  }

  async function onSave() {
    if (!info || busy) return;
    try {
      const report = await save();
      setMessage({ kind: "info", text: `${report.written.length} Datei(en) gespeichert` });
    } catch (e) {
      fail(e);
    }
    await refreshDirty();
  }

  async function onBuild() {
    try {
      const out = await saveDialog({ defaultPath: "out.nds", filters: [{ name: "ROM", extensions: ["nds"] }] });
      if (!out) return;
      setBusy(true);
      setMessage({ kind: "info", text: "Baue ROM ..." });
      const report = await buildRom(out);
      setLastRom(report.rom);
      setMessage({ kind: "info", text: `ROM gebaut: ${report.rom} (${report.saved} Datei(en) vorher gespeichert)` });
    } catch (e) {
      fail(e);
    } finally {
      setBusy(false);
    }
    await refreshDirty();
  }

  async function pickEmulator(): Promise<string | null> {
    const chosen = await open({ directory: false, multiple: false, title: "Emulator wählen (z. B. melonDS)" });
    if (typeof chosen !== "string") return null;
    storeEmulator(chosen);
    return chosen;
  }

  async function onPickEmulator() {
    try {
      const chosen = await pickEmulator();
      if (chosen) setMessage({ kind: "info", text: `Emulator: ${chosen}` });
    } catch (e) {
      fail(e);
    }
  }

  async function onRun() {
    if (!lastRom) return;
    try {
      const emulator = storedEmulator() ?? (await pickEmulator());
      if (!emulator) return;
      await runRom(emulator, lastRom);
      setMessage({ kind: "info", text: `Emulator gestartet: ${lastRom}` });
    } catch (e) {
      fail(e);
    }
  }

  // Closing the window (button, Cmd+Q) asks before discarding unsaved edits.
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let disposed = false;
    getCurrentWindow()
      .onCloseRequested(async (e) => {
        if (dirtyRef.current > 0 && !(await ask("Ungespeicherte Änderungen verwerfen?", { kind: "warning" }))) {
          e.preventDefault();
        }
      })
      .then((fn) => {
        if (disposed) fn();
        else unlisten = fn;
      })
      .catch(() => {
        /* not running inside a Tauri window */
      });
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

  // Cmd+Q does not close the window first: the backend holds the exit back and asks here.
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let disposed = false;
    let asking = false;
    listen("confirm-quit", async () => {
      if (asking) return;
      asking = true;
      try {
        if (await ask("Ungespeicherte Änderungen verwerfen und beenden?", { kind: "warning" })) {
          await invoke("force_quit");
        }
      } catch (e) {
        setMessage({ kind: "error", text: errorText(e) });
      } finally {
        asking = false;
      }
    })
      .then((fn) => {
        if (disposed) fn();
        else unlisten = fn;
      })
      .catch(() => {
        /* not running inside a Tauri window */
      });
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

  // Shortcuts always call the handlers of the latest render.
  const latest = useRef({ onSave, step, onDeleteBuilding, onDeleteEvent, unpinSelection });
  latest.current = { onSave, step, onDeleteBuilding, onDeleteEvent, unpinSelection };
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const selected = pinnedRef.current;
      if ((selected.kind === "building" || selected.kind === "event") && !e.repeat) {
        // Delete (Backspace on a Mac keyboard) removes the selected building or event after asking;
        // neither it nor Escape touches a form field that is being typed in.
        if (wantsDelete({ key: e.key, target: e.target as HTMLElement | null, metaKey: e.metaKey, ctrlKey: e.ctrlKey, altKey: e.altKey })) {
          e.preventDefault();
          if (selected.kind === "building") void latest.current.onDeleteBuilding(selected.map, selected.index);
          else void latest.current.onDeleteEvent(selected.eventFile, selected.eventKind, selected.index);
          return;
        }
        if (e.key === "Escape" && !isTypingTarget(e.target as HTMLElement | null)) {
          latest.current.unpinSelection();
          return;
        }
      }
      if (!(e.metaKey || e.ctrlKey) || e.altKey) return;
      const key = e.key.toLowerCase();
      if (key === "z") {
        e.preventDefault();
        void latest.current.step(e.shiftKey ? redo : undo);
      } else if (key === "s") {
        e.preventDefault();
        void latest.current.onSave();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  const workbench: WorkbenchData = {
    info,
    headers,
    selected,
    onSelect: (h) => void onSelect(h),
    matrix,
    areaDataId: sceneArea,
    modelsToken,
    showPermissions,
    onShowPermissions: setShowPermissions,
    eventFileId: headers.find((h) => h.id === selected)?.event_file_id ?? null,
    showEvents,
    onShowEvents: setShowEvents,
    paintEnabled: !busy,
    history,
    mapRefresh,
    onEdited,
    onError: fail,
    onLoadStatus,
    inspector,
    onInspect,
    memberCounts: counts,
    headerRevision,
    onHeaderChanged,
    areaRevision,
    onAreaChanged,
    matrixRevision,
    onMatrixEdited,
    onCountsChanged,
    onPin: setPinned,
    selectedBuilding: pinned.kind === "building" ? { map: pinned.map, index: pinned.index } : null,
    onSelectBuilding,
    onBuildingMoved,
    buildingRevision,
    buildingRefresh,
    onBuildingChanged,
    onAddBuilding,
    onDeleteBuilding,
    selectedEvent,
    onSelectEvent,
    onEventMoved,
    eventRefresh,
    eventLock,
    onEventsLocked,
    eventRevision,
    onEventChanged,
    onAddEvent,
    onDeleteEvent,
    message,
    onDismissMessage: () => setMessage(null),
  };

  return (
    <main style={{ fontFamily: "system-ui, sans-serif", display: "flex", flexDirection: "column", height: "100vh" }}>
      <Toolbar
        projectOpen={info !== null}
        busy={busy}
        dirtyCount={dirtyCount}
        canRun={lastRom !== null}
        onOpen={onOpen}
        onUndo={() => void step(undo)}
        onRedo={() => void step(redo)}
        onSave={() => void onSave()}
        onBuild={() => void onBuild()}
        onRun={() => void onRun()}
        onPickEmulator={() => void onPickEmulator()}
      />
      <Workbench data={workbench} />
    </main>
  );
}
