import { useCallback, useEffect, useRef, useState } from "react";
import { FloatingToolbar, shownAnchor } from "../ui/frame/FloatingToolbar";
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
 * renders the bars, never the scene. In a view lower than `TOOLBAR_MIN_HEIGHT` an upright toolbar lies flat at the top; the anchor the
 * shell remembers stays.
 */
export function SceneOverlays({ layers }: { layers: SceneBarProps }) {
  const { tool, onTool, eventFileId, eventLock } = useWorkbench();
  const { state, setToolbar, density } = useShell();
  const box = useRef<HTMLDivElement>(null);
  // The view's height; 0 while it is not measured (jsdom has no layout), which keeps the anchor.
  const [height, setHeight] = useState(0);
  useEffect(() => {
    const el = box.current;
    if (!el) return;
    const measure = () => setHeight(el.getBoundingClientRect().height);
    measure();
    if (typeof ResizeObserver === "undefined") return;
    const observer = new ResizeObserver(measure);
    observer.observe(el);
    return () => observer.disconnect();
  }, []);
  const area = useCallback(() => box.current?.getBoundingClientRect() ?? null, []);
  // The four event tools wait for a shown place and an event file that may be written.
  const eventsBlocked = eventLock ? lockedText(eventLock.reason) : eventFileId === null ? "Erst einen Ort wählen." : null;
  const anchor = shownAnchor(state.toolbar, height);
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
        onAnchor={setToolbar}
        area={area}
      />
    </div>
  );
}
