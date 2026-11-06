import * as THREE from "three";
import { MAP_SIZE } from "../layout";
import { OVERLAY_Y } from "./content";
import type { SceneHost } from "./SceneHost";

/** The scene's probe for the agent gate (dev build only, see `useSceneSetup`): where a tile is on the screen. */
export interface SceneProbe {
  /** The client point of the centre of absolute tile (absX, absZ) on its overlay plate (its ground), or null beside the matrix or out of view. */
  tileToClient(absX: number, absZ: number): { clientX: number; clientY: number } | null;
}

declare global {
  interface Window {
    __g4SceneProbe?: SceneProbe;
  }
}

/** Installs `window.__g4SceneProbe` for the scene of `h`; the returned function takes it away again (unless another probe replaced it). */
export function installSceneProbe(h: SceneHost): () => void {
  const probe: SceneProbe = {
    tileToClient: (absX, absZ) => {
      const matrix = h.matrix;
      if (!matrix || absX < 0 || absZ < 0 || absX >= matrix.width * MAP_SIZE || absZ >= matrix.height * MAP_SIZE) return null;
      // The same point pickTile meets: the plate of the tile, on its ground.
      h.camera.updateMatrixWorld();
      const p = new THREE.Vector3(absX + 0.5, OVERLAY_Y + h.ground.y(absX, absZ), absZ + 0.5).project(h.camera);
      if (Math.abs(p.x) > 1 || Math.abs(p.y) > 1 || Math.abs(p.z) > 1) return null;
      const rect = h.canvas.getBoundingClientRect();
      return { clientX: rect.left + ((p.x + 1) / 2) * rect.width, clientY: rect.top + ((1 - p.y) / 2) * rect.height };
    },
  };
  window.__g4SceneProbe = probe;
  return () => {
    if (window.__g4SceneProbe === probe) delete window.__g4SceneProbe;
  };
}
