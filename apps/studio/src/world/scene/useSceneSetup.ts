import { useEffect } from "react";
import type { RefObject } from "react";
import { activeScene } from "./activeScene";
import { createBuildingDrag } from "./buildingDrag";
import { attachCanvasClicks } from "./canvasClicks";
import { createEventDrag } from "./eventDrag";
import { createPicker } from "./picking";
import { createHover } from "./pointerHover";
import { installSceneProbe } from "./sceneProbe";
import { SceneHost } from "./SceneHost";
import type { ReportEvents, SceneRefs } from "./types";

/**
 * Builds the scene host in `el` and wires the pointer interaction to it (hover, drags of the
 * selected building and event, clicks); all of it lives as long as the component. One effect on
 * purpose: on unmount the interaction is detached first, then the host disposes the scene, as it
 * always did.
 */
export function useSceneSetup(el: RefObject<HTMLDivElement | null>, refs: SceneRefs, report: ReportEvents) {
  useEffect(() => {
    const parent = el.current;
    if (!parent) return;
    const h = new SceneHost(parent);
    refs.handle.current = h;
    activeScene.set(h);

    const picker = createPicker(h);
    const surface = { host: parent, canvas: h.canvas, keys: window };
    // `busy` closes over the drags declared below; it is only called from a later animation frame, after both exist.
    const hover = createHover(h, picker, refs, () => buildingDrag.dragging || eventDrag.dragging);
    h.refreshHover = hover.refresh;
    const buildingDrag = createBuildingDrag(h, picker, surface, refs);
    const detachBuildingDrag = buildingDrag.attach();
    const eventDrag = createEventDrag(h, picker, surface, refs, report);
    const detachEventDrag = eventDrag.attach();
    const detachClicks = attachCanvasClicks(h, picker, refs);
    const detachHover = hover.attach();
    // The agent gate's probe exists in the dev build only; the production build drops it with this branch.
    const removeProbe = import.meta.env.DEV ? installSceneProbe(h) : null;

    return () => {
      detachBuildingDrag();
      detachEventDrag();
      detachClicks();
      detachHover();
      removeProbe?.();
      refs.callbacks.current.onInspect({ kind: "none" });
      refs.callbacks.current.onStatus(null);
      refs.handle.current = null;
      activeScene.clear(h);
      h.dispose();
    };
  }, []);
}
