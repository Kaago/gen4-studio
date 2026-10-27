import type { SceneHost } from "./SceneHost";

/**
 * The scene host of the 3D view while it is mounted (there is one 3D view, in the world's dock), for what works on the view from outside
 * it: „Als Vorschaubild“ in its tab menu and in the palette.
 */
let current: Pick<SceneHost, "canvas" | "capture"> | null = null;

export const activeScene = {
  /** The 3D view mounted its host. */
  set(host: Pick<SceneHost, "canvas" | "capture">): void {
    current = host;
  },
  /** The 3D view freed `host` (a host that is no longer the current one changes nothing). */
  clear(host: Pick<SceneHost, "canvas" | "capture">): void {
    if (current === host) current = null;
  },
  get(): Pick<SceneHost, "canvas" | "capture"> | null {
    return current;
  },
  /**
   * The 3D view is mounted and shows: its canvas is in the page and has a size. A view behind another tab or in a hidden area has none (its
   * renderer then draws 1 × 1), so its picture would be no picture.
   */
  shown(): boolean {
    const canvas = current?.canvas;
    return canvas !== undefined && canvas.isConnected && canvas.clientWidth > 1 && canvas.clientHeight > 1;
  },
};
