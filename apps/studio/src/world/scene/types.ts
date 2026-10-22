import type { RefObject } from "react";
import type { EventsDto, HeaderSummary, MapDto } from "../../api";
import type { Message } from "../../Toolbar";
import type { InspectorModel } from "../../workbench/Inspector";
import type { BuildingRef } from "../buildingLayer";
import type { EventLock, EventRef } from "../events";
import type { SceneHost } from "./SceneHost";

/** A map the scene has fetched (the scene mutates `map` as permissions and buildings change). */
export interface Loaded {
  map: MapDto;
}

/** What reading an event file gave: its events, or why there are none (never both). */
export interface FetchedEvents {
  events: EventsDto | null;
  error: string | null;
}

/** What the scene reports upwards; kept in a ref (`SceneRefs.callbacks`) so a new callback never rebuilds the scene. */
export interface SceneCallbacks {
  paintEnabled: boolean;
  onEdited: () => void;
  onError: (message: string) => void;
  onInspect: (model: InspectorModel) => void;
  onStatus: (message: Message | null) => void;
  onSelectBuilding: (ref: BuildingRef | null) => void;
  onBuildingMoved: (map: number) => void;
  onSelectEvent: (ref: EventRef | null) => void;
  onEventMoved: (eventFile: number) => void;
  onEventsLocked: (lock: EventLock | null) => void;
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
  /** Whether the event markers are shown. */
  showEvents: RefObject<boolean>;
  /** Like the area data, the event file belongs to the header and is read when a matrix loads. */
  eventFile: RefObject<number | null>;
  headers: RefObject<HeaderSummary[]>;
  /** Like the area data, the one the buildings are textured with is read when models are requested. */
  area: RefObject<number>;
}

/** Shows (and reports upwards) what reading an event file gave: the note over the scene and its lock state. */
export type ReportEvents = (fetched: FetchedEvents) => void;
