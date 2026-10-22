import type { BuildingDto } from "../api";
import { snapTo } from "./drag";
import { MAP_SIZE, MODEL_SCALE } from "./layout";

/** A building's position and scale; a whole `BuildingDto` is accepted too. */
export type BuildingPose = Pick<BuildingDto, "x" | "y" | "z" | "sx" | "sy" | "sz"> & Partial<BuildingDto>;

/**
 * Scene transform of a building's model in the matrix cell whose origin is `cell`. Positions are
 * tiles from the map centre; the GLB is in 1/16 tile units like the map models, so its scale is
 * the building's own scale times the map model scale. Rotation is not applied: the game ignores it.
 */
export function buildingTransform(b: BuildingPose, cell: { x: number; z: number }): { position: [number, number, number]; scale: [number, number, number] } {
  return {
    position: [cell.x + MAP_SIZE / 2 + b.x, b.y, cell.z + MAP_SIZE / 2 + b.z],
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

/** Whether a key event asks to delete the selected building: Delete or Backspace, unmodified, not typed into a field. */
export function wantsDelete(e: { key: string; target: { tagName?: string; isContentEditable?: boolean } | null; metaKey: boolean; ctrlKey: boolean; altKey: boolean }): boolean {
  if (e.metaKey || e.ctrlKey || e.altKey) return false;
  return (e.key === "Delete" || e.key === "Backspace") && !isTypingTarget(e.target);
}

/** A request to read one map's buildings again: a new `token` makes it a new request. */
export interface BuildingRefresh {
  map: number;
  token: number;
  /** A model whose earlier failure should be forgotten, so that loading it is tried again. */
  retryModel: number | null;
}

/** Whether two building lists are identical in every field. */
export function sameBuildings(a: readonly BuildingDto[], b: readonly BuildingDto[]): boolean {
  return (
    a.length === b.length &&
    a.every((x, i) => {
      const y = b[i];
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
    })
  );
}
