import type { BuildingDto } from "../api";
import { snapTo } from "./drag";
import { MAP_SIZE, MODEL_SCALE } from "./layout";

/** A building's position and scale; a whole `BuildingDto` is accepted too. */
export type BuildingPose = Pick<BuildingDto, "x" | "y" | "z" | "sx" | "sy" | "sz"> & Partial<BuildingDto>;

/**
 * Scene transform of a building's model in the matrix cell whose origin is `cell` (`y`: the height of the cell). Positions are
 * tiles from the map centre; the GLB is in 1/16 tile units like the map models, so its scale is
 * the building's own scale times the map model scale. Rotation is not applied: the game ignores it.
 */
export function buildingTransform(b: BuildingPose, cell: { x: number; y?: number; z: number }): { position: [number, number, number]; scale: [number, number, number] } {
  return {
    position: [cell.x + MAP_SIZE / 2 + b.x, (cell.y ?? 0) + b.y, cell.z + MAP_SIZE / 2 + b.z],
    scale: [b.sx * MODEL_SCALE, b.sy * MODEL_SCALE, b.sz * MODEL_SCALE],
  };
}

/** Placeholder box for a building without a model: its scale factors as size, standing on its position. */
export function buildingFallbackBox(b: BuildingPose): { offsetY: number; size: [number, number, number] } {
  return { offsetY: b.sy / 2, size: [b.sx, b.sy, b.sz] };
}

/** Buildings snap to half tiles. */
export function snapHalfTile(v: number): number {
  return snapTo(v, 0.5);
}

/** Identity of a building in the scene: its map and index in that map. */
export function buildingKey(map: number, index: number): string {
  return `${map}:${index}`;
}

/** The distinct model ids of `buildings`, in order of first use. */
export function buildingModelIds(buildings: readonly Pick<BuildingDto, "model_id">[]): number[] {
  return [...new Set(buildings.map((b) => b.model_id))];
}

/** Position (tiles from the map centre) of the centre of tile (`tx`, `ty`) of a map. */
export function newBuildingPosition(tx: number, ty: number): { x: number; z: number } {
  return { x: tx + 0.5 - MAP_SIZE / 2, z: ty + 0.5 - MAP_SIZE / 2 };
}

/** Whether keys typed at `target` belong to a form field (so scene shortcuts must stay out of the way). */
export function isTypingTarget(target: { tagName?: string; isContentEditable?: boolean } | null): boolean {
  if (!target) return false;
  return target.isContentEditable === true || target.tagName === "INPUT" || target.tagName === "TEXTAREA" || target.tagName === "SELECT";
}

export type ShortcutAction = "open" | "palette" | "save" | "undo" | "redo" | "play";

/**
 * Every app-level shortcut as the settings list it (read only): its keys as shown and what it does. A record over `ShortcutAction`, so no
 * action can be left out; a test presses each entry's keys through `shortcutAction`, so the keys cannot drift either.
 */
export const SHORTCUTS: Readonly<Record<ShortcutAction, { keys: string; label: string }>> = {
  open: { keys: "⌘O", label: "Projekt öffnen" },
  palette: { keys: "⌘K", label: "Befehlspalette öffnen" },
  save: { keys: "⌘S", label: "Speichern" },
  undo: { keys: "⌘Z", label: "Rückgängig" },
  redo: { keys: "⇧⌘Z", label: "Wiederholen" },
  play: { keys: "⌘R", label: "Hier spielen (nur in der App)" },
};

/**
 * The app-level shortcut a key event asks for (Cmd/Ctrl+O, +K, +S, +Z, +Shift+Z, and +R for "Hier spielen" inside the app), or null; none
 * while typing in a field, where Z must stay the text field's own undo. `inApp` is whether the page runs in the Tauri window: in a browser
 * Cmd+R reloads the page and is not ours.
 */
export function shortcutAction(e: {
  key: string;
  target: { tagName?: string; isContentEditable?: boolean } | null;
  metaKey: boolean;
  ctrlKey: boolean;
  altKey: boolean;
  shiftKey: boolean;
  inApp?: boolean;
}): ShortcutAction | null {
  if (!(e.metaKey || e.ctrlKey) || e.altKey) return null;
  const key = e.key.toLowerCase();
  if (key === "s") return "save";
  if (key === "o" && !e.shiftKey) return "open";
  if (key === "k" && !e.shiftKey) return "palette";
  // Shift+R is the hard reload of a webview.
  if (key === "r" && e.inApp === true && !e.shiftKey) return "play";
  if (key === "z" && !isTypingTarget(e.target)) return e.shiftKey ? "redo" : "undo";
  return null;
}

/** Whether a key event asks to delete the selected building: Delete or Backspace, unmodified, not typed into a field. */
export function wantsDelete(e: { key: string; target: { tagName?: string; isContentEditable?: boolean } | null; metaKey: boolean; ctrlKey: boolean; altKey: boolean }): boolean {
  if (e.metaKey || e.ctrlKey || e.altKey) return false;
  return (e.key === "Delete" || e.key === "Backspace") && !isTypingTarget(e.target);
}

/** A request to read one map's buildings again (queued by the scene, see `RefreshQueue`). */
export interface BuildingRefresh {
  map: number;
  /** A model whose earlier failure should be forgotten, so that loading it is tried again. */
  retryModel: number | null;
}

/** Whether two buildings are identical in every field. */
export function sameBuilding(x: BuildingDto, y: BuildingDto): boolean {
  return (
    x.model_id === y.model_id &&
    x.x === y.x &&
    x.y === y.y &&
    x.z === y.z &&
    x.sx === y.sx &&
    x.sy === y.sy &&
    x.sz === y.sz &&
    x.rotation.every((r, k) => r === y.rotation[k])
  );
}

/** Whether two building lists are identical in every field. */
export function sameBuildings(a: readonly BuildingDto[], b: readonly BuildingDto[]): boolean {
  return a.length === b.length && a.every((x, i) => sameBuilding(x, b[i]));
}

/**
 * What a map's buildings are after a reload: when `fresh` holds the same values as `current`, the
 * current list itself (same objects, so the scene's slots, which hold those objects, stay in step
 * with the map data); otherwise `fresh`, which the scene must be rebuilt from (`changed`).
 */
export function reconcileBuildings(current: BuildingDto[], fresh: BuildingDto[]): { buildings: BuildingDto[]; changed: boolean } {
  return sameBuildings(current, fresh) ? { buildings: current, changed: false } : { buildings: fresh, changed: true };
}

/**
 * Records a confirmed move: puts `next` at `index` of `list` when the building there still has the
 * values `expected` (compared by value, not identity) and says whether it did. A list that no
 * longer has that building was replaced by a reload, which the scene already follows.
 */
export function settleBuilding(list: BuildingDto[], index: number, expected: BuildingDto, next: BuildingDto): boolean {
  const current = list[index];
  if (!current || !sameBuilding(current, expected)) return false;
  list[index] = next;
  return true;
}

/** Text of the confirmation before a building is deleted: its number (1-based, like every building label), map, model and position. */
export function deletePrompt(map: number, index: number, b: Pick<BuildingDto, "model_id" | "x" | "y" | "z"> | undefined): string {
  const what = `Gebäude ${index + 1} von Karte ${String(map).padStart(4, "0")}`;
  return b ? `${what} (Modell ${b.model_id}, x ${b.x}, y ${b.y}, z ${b.z}) löschen?` : `${what} löschen?`;
}
