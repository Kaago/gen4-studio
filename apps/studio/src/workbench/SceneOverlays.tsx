import { useCallback, useLayoutEffect, useRef, useState } from "react";
import { FloatingToolbar, isLowView, shownAnchor, toolbarMinHeight } from "../ui/frame/FloatingToolbar";
import { useShell } from "../ui/frame/ShellContext";
import { lockedText } from "../world/events";
import { SceneBar } from "./SceneBar";
import type { SceneBarProps } from "./SceneBar";
import styles from "./SceneOverlays.module.css";
import { TOOLS } from "./tools";
import { useWorkbench } from "./WorkbenchContext";

/**
 * The bars floating over the 3D view, in one corner: the layer bar at the top left, the toolbar under it (anchor left), beside it (top),
 * or at the bottom middle. It covers the view without taking its pointer; only the bars do. A sibling of the scene: reading the shell here
 * renders the bars, never the scene. In a view lower than `toolbarMinHeight` an upright toolbar lies flat at the top; the anchor the
 * shell remembers stays (and is the one the toolbar's menu ticks).
 */
export function SceneOverlays({ layers }: { layers: SceneBarProps }) {
  const { tool, onTool, eventFileId, eventLock } = useWorkbench();
  const { state, setToolbar, density } = useShell();
  const box = useRef<HTMLDivElement>(null);
  const minHeight = toolbarMinHeight(density);
  // Only whether the view is too low is kept, not its height, and the bars are memoised: dragging the splitter re-renders this component
  // with every shell change, but the bars only when this flag, the anchor or their own props change. Measured before the first paint, so
  // a low view never shows the upright bar for a frame; a view that is not measured (jsdom has no layout) is not low.
  const [low, setLow] = useState(false);
  useLayoutEffect(() => {
    const el = box.current;
    if (!el) return;
    const measure = () => setLow(isLowView(el.getBoundingClientRect().height, minHeight));
    measure();
    if (typeof ResizeObserver === "undefined") return;
    const observer = new ResizeObserver(measure);
    observer.observe(el);
    return () => observer.disconnect();
  }, [minHeight]);
  const area = useCallback(() => box.current?.getBoundingClientRect() ?? null, []);
  // The four event tools wait for a shown place and an event file that may be written.
  const eventsBlocked = eventLock ? lockedText(eventLock.reason) : eventFileId === null ? "Erst einen Ort wählen." : null;
  const anchor = shownAnchor(state.toolbar, low);
  return (
    <div ref={box} className={styles.overlays} data-scene-overlays="" data-anchor={anchor}>
      <SceneBar {...layers} />
      <FloatingToolbar
        tools={TOOLS}
        tool={tool}
        onTool={onTool}
        eventsBlocked={eventsBlocked}
        density={density}
        anchor={anchor}
        chosen={state.toolbar}
        onAnchor={setToolbar}
        area={area}
      />
    </div>
  );
}
