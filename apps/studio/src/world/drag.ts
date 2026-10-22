/** `v` rounded to a multiple of `step`; never -0. */
export function snapTo(v: number, step: number): number {
  const snapped = Math.round(v / step) * step;
  return snapped === 0 ? 0 : snapped;
}

/**
 * Where an object stands after the pointer travelled from `start` to `current` on the ground
 * plane: the object's `origin` plus that travel, snapped to multiples of `snap` tiles (0.5 for
 * buildings, 1 for events). All points are in the same plane frame (tiles).
 */
export function planeDrag(
  start: { x: number; z: number },
  current: { x: number; z: number },
  origin: { x: number; z: number },
  snap = 0.5,
): { x: number; z: number } {
  return {
    x: snapTo(origin.x + current.x - start.x, snap),
    z: snapTo(origin.z + current.z - start.z, snap),
  };
}
