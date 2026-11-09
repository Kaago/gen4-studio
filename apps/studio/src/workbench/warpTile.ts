import { useEffect, useState } from "react";
import { getMap } from "../api";
import type { TileBehavior } from "../api";
import { loadNames } from "../world/scene/tileNames";
import { t } from "../i18n";
import { TERMS } from "./terms";

/** The door behaviour and the collision a door needs (the player bumps into it). */
export const DOOR = 0x69;
export const DOOR_COLLISION = 0x80;

/** What a tile means for a warp on it: whether the game starts the warp there, and the tile in words. */
export interface WarpTile {
  fires: boolean;
  text: string;
}

const hex2 = (n: number) => `0x${n.toString(16).toUpperCase().padStart(2, "0")}`;

/**
 * Whether the game takes a warp on a tile of behaviour `kind` and `collision` (as the kernel's `warp_fires`): warp tiles
 * (stairs, mats, panels) always, doors only when they block the player; any other tile never.
 */
export function warpTile(kind: number, collision: number, names: ReadonlyMap<number, TileBehavior>): WarpTile {
  const behavior = names.get(kind);
  const fires = behavior?.class === "warp" || (behavior?.class === "door" && collision === DOOR_COLLISION);
  const what = behavior
    ? t("workbench:warpTile.whatNamed", { kind: hex2(kind), name: behavior.name, collision: hex2(collision) })
    : t("workbench:warpTile.what", { kind: hex2(kind), collision: hex2(collision) });
  const warp = TERMS.warp.tech;
  if (fires) return { fires, text: t("workbench:warpTile.fires", { what, warp }) };
  const why = behavior?.class === "door" ? t("workbench:warpTile.doorNeedsCollision") : t("workbench:warpTile.noWarpTile");
  return { fires, text: t("workbench:warpTile.doesNotFire", { what, warp, why }) };
}

/** What tile (`x`, `y`) of map `map` means for a warp on it (behaviour names and permissions as the backend holds them). */
export async function readWarpTile(map: number, x: number, y: number): Promise<WarpTile> {
  const [names, data] = await Promise.all([loadNames(), getMap(map)]);
  const [kind, collision] = data.permissions[y * 32 + x] ?? [0, 0];
  return warpTile(kind, collision, names);
}

/**
 * `readWarpTile` for a component: null while it loads, for no map, for a tile off the 32 × 32 map and after a failed
 * read (the hint is left out then; the backend still checks the tile when it makes a door).
 */
export function useWarpTile(map: number | null, x: number | null, y: number | null): WarpTile | null {
  const [tile, setTile] = useState<{ key: string; value: WarpTile } | null>(null);
  const key = `${map}:${x}:${y}`;
  const valid = map !== null && x !== null && y !== null && Number.isInteger(x) && Number.isInteger(y) && x >= 0 && y >= 0 && x < 32 && y < 32;
  useEffect(() => {
    if (!valid || map === null || x === null || y === null) return;
    let cancelled = false;
    readWarpTile(map, x, y).then(
      (value) => {
        if (!cancelled) setTile({ key, value });
      },
      () => {
        if (!cancelled) setTile(null);
      },
    );
    return () => {
      cancelled = true;
    };
  }, [map, x, y, key, valid]);
  return valid && tile?.key === key ? tile.value : null;
}
