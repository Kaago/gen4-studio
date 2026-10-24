import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { MutableRefObject } from "react";
import { ask } from "../platform/dialogs";
import { addBuilding, addEvent, deleteBuilding, deleteEvent, getEvents, getMap, setPermission, updateBuilding } from "../api";
import type { EventKind, EventPatch, HeaderSummary, MatrixDto } from "../api";
import type { Message } from "./message";

/** Fields a new event gets in the undo step that adds it, and that step's name ("Trainer Jens platziert"). */
export interface EventPlacement {
  patch: EventPatch;
  label: string;
}
import type { InspectorModel } from "../workbench/Inspector";
import { pinAfterHistory } from "../workbench/pin";
import { DOOR, DOOR_COLLISION, readWarpTile } from "../workbench/warpTile";
import { mapUnderTile } from "../workbench/wizards/wizardSpecs";
import type { BuildingRef } from "../world/buildingLayer";
import { deletePrompt, newBuildingPosition } from "../world/buildings";
import { eventAt, eventDeletePrompt, eventEditGate } from "../world/events";
import type { EventLock, EventRef } from "../world/events";
import type { MapBuildings } from "../world/scene/types";
import type { EditBus } from "../bus/editBus";
import type { InspectorState } from "./useInspectorState";

interface WorldActionsParams {
  bus: EditBus;
  pins: InspectorState;
  matrix: MatrixDto | null;
  matrixRef: MutableRefObject<MatrixDto | null>;
  headersRef: MutableRefObject<HeaderSummary[]>;
  selectedRef: MutableRefObject<number | null>;
  setMessage: (message: Message | null) => void;
  fail: (e: unknown) => void;
  refreshDirty: () => Promise<void>;
}

const sameBuildings = (a: MapBuildings[], b: MapBuildings[]) =>
  a.length === b.length && a.every((x, i) => x.map === b[i]?.map && x.models.length === b[i]?.models.length && x.models.every((m, j) => m === b[i]?.models[j]));

/**
 * Buildings and events of the scene: adding (on the tile the pointer was last over), deleting
 * after asking, pinning the selection in the inspector, and the bus events that make the scene
 * and the editors read their data again after a write.
 */
export function useWorldActions({ bus, pins, matrix, matrixRef, headersRef, selectedRef, setMessage, fail, refreshDirty }: WorldActionsParams) {
  const { pinned, setPinned, setHover } = pins;
  // The tile the pointer was last over: where a building added without a clicked tile (`onAddBuilding(fallbackMap)`) goes.
  const lastTile = useRef<{ map: number; x: number; y: number } | null>(null);
  // The same tile as a position in the whole matrix, where "Hinzufügen" puts an event. Unlike the
  // one above it is forgotten when the scene gets another matrix (the position is only valid in one).
  const matrixTile = useRef<{ x: number; z: number } | null>(null);
  useEffect(() => {
    matrixTile.current = null;
  }, [matrix]);
  /** The tile the pointer was last over as a position in the shown matrix (null: none yet); the room wizard puts its door there. */
  const lastMatrixTile = useCallback(() => matrixTile.current, []);
  const onInspect = useCallback(
    (model: InspectorModel) => {
      if (model.kind === "tile") {
        lastTile.current = { map: model.map, x: model.x, y: model.y };
        matrixTile.current = { x: model.absX, z: model.absZ };
      }
      setHover(model.kind === "none" ? null : model);
    },
    [setHover],
  );

  // Whether the shown event file is locked: reported by the scene whenever it reads events, the one
  // source for every place that disables adding, deleting or dragging.
  const [eventLock, setEventLock] = useState<EventLock | null>(null);
  const eventLockRef = useRef(eventLock);
  eventLockRef.current = eventLock;
  const onEventsLocked = useCallback((lock: EventLock | null) => {
    setEventLock((previous) => (previous?.file === lock?.file && previous?.reason === lock?.reason ? previous : lock));
  }, []);

  // The buildings of the shown maps, as the scene reports them (the list "An diesem Ort" shows). A report that says what is known
  // already keeps the list, so a refresh that changed nothing renders nothing.
  const [sceneBuildings, setSceneBuildings] = useState<MapBuildings[]>([]);
  const onBuildings = useCallback((next: MapBuildings[]) => {
    setSceneBuildings((before) => (sameBuildings(before, next) ? before : next));
  }, []);

  /** Shows the header again in the inspector when a building or event was pinned (it was deselected or deleted). */
  const unpinSelection = useCallback(() => {
    setPinned((p) => pinAfterHistory(p, selectedRef.current));
  }, [setPinned, selectedRef]);
  const onSelectBuilding = useCallback(
    (ref: BuildingRef | null) => (ref ? setPinned({ kind: "building", map: ref.map, index: ref.index }) : unpinSelection()),
    [setPinned, unpinSelection],
  );
  const onSelectEvent = useCallback(
    (ref: EventRef | null) => (ref ? setPinned({ kind: "event", eventFile: ref.file, eventKind: ref.kind, index: ref.index }) : unpinSelection()),
    [setPinned, unpinSelection],
  );
  // Stable while the same event stays pinned (also when it is pinned again), so the memoised scene is not disturbed.
  const pinnedEvent = pinned.kind === "event" ? pinned : null;
  const [eventFile, eventKind, eventIndex] = [pinnedEvent?.eventFile, pinnedEvent?.eventKind, pinnedEvent?.index];
  const selectedEvent = useMemo<EventRef | null>(
    () => (eventFile !== undefined && eventKind !== undefined && eventIndex !== undefined ? { file: eventFile, kind: eventKind, index: eventIndex } : null),
    [eventFile, eventKind, eventIndex],
  );
  // The scene wrote a drag: only the event editor (and the dirty count) has to catch up.
  const onEventMoved = useCallback(async () => {
    bus.emit({ topic: "eventData" });
    await refreshDirty();
  }, [bus, refreshDirty]);
  // The event editor wrote a field: the scene reads that file's events again.
  const onEventChanged = useCallback(
    async (file: number) => {
      bus.emit({ topic: "eventsRefetch", file });
      await refreshDirty();
    },
    [bus, refreshDirty],
  );
  // The scene wrote a drag: only the building editor (and the dirty count) has to catch up.
  const onBuildingMoved = useCallback(async () => {
    bus.emit({ topic: "buildingData" });
    await refreshDirty();
  }, [bus, refreshDirty]);
  // The building editor wrote a field: the scene reads that map's buildings again.
  const onBuildingChanged = useCallback(
    async (map: number, retryModel: number | null) => {
      bus.emit({ topic: "buildingsRefetch", map, retryModel });
      await refreshDirty();
    },
    [bus, refreshDirty],
  );
  // A new building goes on the clicked tile `at` (a click with the building tool), else on the tile the pointer was last over (the map centre of `fallbackMap` when
  // that is unknown); it is selected afterwards. Adding and moving are two undoable changes.
  const onAddBuilding = useCallback(
    async (fallbackMap: number | null, at?: { map: number; x: number; y: number }) => {
      const shown = matrixRef.current;
      if (!shown) return;
      const hovered = at ?? (lastTile.current && shown.maps.includes(lastTile.current.map) ? lastTile.current : null);
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
      bus.emit({ topic: "buildingsRefetch", map, retryModel: null });
      bus.emit({ topic: "buildingData" });
      await refreshDirty();
    },
    [bus, matrixRef, setMessage, setPinned, fail, refreshDirty],
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
      bus.emit({ topic: "buildingsRefetch", map, retryModel: null });
      bus.emit({ topic: "buildingData" });
      await refreshDirty();
    },
    [bus, fail, unpinSelection, refreshDirty],
  );

  // A warp on a plain tile is never taken in the game: "Warp hinzufügen" says so and offers to make the tile a door.
  const offerDoor = useCallback(
    async (shown: MatrixDto, tile: { x: number; z: number }) => {
      const map = mapUnderTile(shown, tile.x, tile.z);
      if (map === null) return;
      const [x, y] = [tile.x % 32, tile.z % 32];
      try {
        const found = await readWarpTile(map, x, y);
        if (found.fires) return;
        const question = `Der neue Warp liegt auf Kachel x ${tile.x}, z ${tile.z}. ${found.text} Die Kachel zur Tür machen (Verhalten 0x69, Kollision 0x80)? Der Spieler läuft dann dagegen, um den Warp zu nehmen.`;
        if (!(await ask(question, { kind: "warning" }))) {
          setMessage({ kind: "info", text: `Warp auf Kachel x ${tile.x}, z ${tile.z}: löst im Spiel nicht aus, solange die Kachel keine Tür- oder Warp-Kachel ist.` });
          return;
        }
        await setPermission(map, x, y, DOOR, DOOR_COLLISION);
        bus.emit({ topic: "mapRefetch", map });
      } catch (e) {
        fail(e);
      }
    },
    [bus, setMessage, fail],
  );

  // A new event goes on the clicked tile `at` (a click with an event tool), else on the tile the pointer was last over, in the event file of the selected header;
  // it is selected afterwards. `placement` sets fields of the new event in the same undo step (a trainer placed from the Pokémon area).
  const onAddEvent = useCallback(
    async (kind: EventKind, at?: { x: number; z: number }, placement?: EventPlacement) => {
      const shown = matrixRef.current;
      const file = headersRef.current.find((h) => h.id === selectedRef.current)?.event_file_id ?? null;
      if (!shown || file === null) return;
      const gate = eventEditGate(eventLockRef.current, file);
      if (!gate.allowed) {
        setMessage({ kind: "error", text: gate.text });
        return;
      }
      const tile = at ?? matrixTile.current;
      if (!tile) {
        setMessage({ kind: "info", text: "Erst mit der Maus über die Kachel fahren, auf der das Event entstehen soll." });
        return;
      }
      let added = false;
      try {
        const index = placement ? await addEvent(file, kind, tile.x, tile.z, placement.patch, placement.label) : await addEvent(file, kind, tile.x, tile.z);
        setPinned({ kind: "event", eventFile: file, eventKind: kind, index });
        added = true;
      } catch (e) {
        fail(e);
      }
      bus.emit({ topic: "eventsRefetch", file });
      bus.emit({ topic: "eventData" });
      if (added && kind === "warp") await offerDoor(shown, tile);
      await refreshDirty();
    },
    [bus, matrixRef, headersRef, selectedRef, setMessage, setPinned, fail, refreshDirty, offerDoor],
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
      bus.emit({ topic: "eventsRefetch", file });
      bus.emit({ topic: "eventData" });
      await refreshDirty();
    },
    [bus, setMessage, fail, unpinSelection, refreshDirty],
  );

  /** Another project was opened: nothing of the old one stays pinned, hovered or locked. */
  function resetForProject() {
    setPinned({ kind: "none" });
    setHover(null);
    lastTile.current = null;
    matrixTile.current = null;
    setEventLock(null);
    setSceneBuildings([]);
  }

  return {
    eventLock,
    sceneBuildings,
    onBuildings,
    selectedEvent,
    onInspect,
    lastMatrixTile,
    onEventsLocked,
    unpinSelection,
    onSelectBuilding,
    onSelectEvent,
    onEventMoved,
    onEventChanged,
    onBuildingMoved,
    onBuildingChanged,
    onAddBuilding,
    onDeleteBuilding,
    onAddEvent,
    onDeleteEvent,
    resetForProject,
  };
}

export type WorldActions = ReturnType<typeof useWorldActions>;
