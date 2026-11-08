import type { RefObject } from "react";
import type { EventsDto, MapDto } from "../../api";
import type { Message } from "../../app/message";
import type { Tool } from "../../ui/frame/shellState";
import type { InspectorModel } from "../../workbench/Inspector";
import type { BuildingRef } from "../buildingLayer";
import type { EventLock, EventRef } from "../events";
import type { SceneHost } from "./SceneHost";
import type { PaintCallbacks } from "./strokes";

/** A map the scene has fetched (the scene mutates `map` as permissions and buildings change). */
export interface Loaded {
  map: MapDto;
}

/** What reading an event file gave: its events, or why there are none (never both). */
export interface FetchedEvents {
  events: EventsDto | null;
  error: string | null;
}

/** The tile a click with one of the adding tools landed on: the map and the tile in it, and the tile over the whole matrix. */
export interface TileClick {
  mapId: number;
  x: number;
  y: number;
  absX: number;
  absZ: number;
}

/** The buildings of one shown map: the model of each, in the order of the map's building list. */
export interface MapBuildings {
  map: number;
  models: number[];
}

/** What the scene reports upwards; kept in a ref (`SceneRefs.callbacks`) so a new callback never rebuilds the scene. */
export interface SceneCallbacks {
  /** Whether edits (painting, adding, dragging) are allowed at all; false while a build runs. */
  paintEnabled: boolean;
  /** The active tool: what a click on a tile does. Only `paint` paints, only the adding tools call `onTileClick`. */
  tool: Tool;
  /** A click with one of the adding tools (figure, door, trigger, sign, building) landed on this tile. */
  onTileClick: (tile: TileClick) => void;
  /** The buildings of the shown maps, reported whenever they were loaded or changed. */
  onBuildings: (buildings: MapBuildings[]) => void;
  onEdited: () => void;
  onError: (message: string) => void;
  onInspect: (model: InspectorModel) => void;
  onStatus: (message: Message | null) => void;
  onSelectBuilding: (ref: BuildingRef | null) => void;
  onBuildingMoved: (map: number) => void;
  onSelectEvent: (ref: EventRef | null) => void;
  /** A click with the select tool on a loaded map, neither on a marker nor on a building, chose its matrix cell. */
  onSelectCell: (cell: { col: number; row: number }) => void;
  onEventMoved: (eventFile: number) => void;
  onEventsLocked: (lock: EventLock | null) => void;
  /** Painting a world map (6.4a): `sub` null while no paint sub-tool paints (another tool, a binary map); clicks then work as before. */
  paint: PaintCallbacks;
}

/**
 * The scene's mutable inputs, all refs: what the setup effect and the loaders read when they run,
 * so they see the latest props without being re-created (and the scene without being rebuilt).
 */
export interface SceneRefs {
  /** The scene host (null before the setup effect ran and after it was cleaned up). */
  handle: RefObject<SceneHost | null>;
  callbacks: RefObject<SceneCallbacks>;
  /** Whether the permission overlay is shown. */
  showPermissions: RefObject<boolean>;
  /** Whether the layer „Höhen“ is shown. */
  showHeights: RefObject<boolean>;
  /** Whether the event markers are shown. */
  showEvents: RefObject<boolean>;
  /** Like the area data, the event file belongs to the header and is read when a matrix loads. */
  eventFile: RefObject<number | null>;
  /** The name of the header a door leads to, as the world tree calls it (undefined: unknown header). */
  destName: RefObject<(headerId: number) => string | undefined>;
  /** Like the area data, the one the buildings are textured with is read when models are requested. */
  area: RefObject<number>;
}

/** Shows (and reports upwards) what reading an event file gave: the note over the scene and its lock state. */
export type ReportEvents = (fetched: FetchedEvents) => void;
