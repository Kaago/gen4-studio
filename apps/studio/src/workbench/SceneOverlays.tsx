import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { FloatingToolbar, isLowView, shownAnchor, toolbarMinHeight } from "../ui/frame/FloatingToolbar";
import { RecipeBar } from "../ui/frame/RecipeBar";
import { useShell } from "../ui/frame/ShellContext";
import { depthAtLeast } from "../ui/frame/shellState";
import { RECIPE_BAR, recipeRequest } from "./recipes";
import type { RecipeEntry } from "./recipes";
import { SceneBar } from "./SceneBar";
import type { SceneBarProps } from "./SceneBar";
import styles from "./SceneOverlays.module.css";
import type { PaintSub } from "../world/paint/ops";
import { usePaint } from "./paint/PaintContext";
import { PAINT_TOOLS } from "./paintTools";
import { eventToolsBlocked, toolsFor } from "./tools";
import { useWorkbench } from "./WorkbenchContext";

/**
 * The bars floating over the 3D view, in one corner: the layer bar (with the top-down switch) at the top left, the toolbar under it (anchor left), beside it (top),
 * or at the bottom middle. It covers the view without taking its pointer; only the bars do. A sibling of the scene: reading the shell here
 * renders the bars, never the scene. In a view lower than `toolbarMinHeight` an upright toolbar lies flat at the top; the anchor the
 * shell remembers stays (and is the one the toolbar's menu ticks). Erzählen shows its recipes there instead of the tools.
 */
export function SceneOverlays({ layers }: { layers: Omit<SceneBarProps, "topDown" | "onTopDown"> }) {
  const { tool, onTool, eventFileId, eventLock, selected, onOpenWizard } = useWorkbench();
  const { state, setToolbar, setTopDown, density, viewDepth } = useShell();
  const box = useRef<HTMLDivElement>(null);
  // On a world map of the text project „Gelände malen“ paints tiles, heights and ramps: its sub-tools follow the tools while it is active.
  const { state: paintState, set: setPaint, target } = usePaint();
  const tools = toolsFor(target !== null);
  const sub = tool === "paint" && target !== null ? paintState.sub : null;
  const paint = useMemo(() => (sub === null ? null : { tools: PAINT_TOOLS, sub, onSub: (s: PaintSub) => setPaint({ sub: s }) }), [sub, setPaint]);
  const minHeight = toolbarMinHeight(density);
  // The sub-tools make the upright bar longer: a view lower than this puts them as a second column beside the tools (the bar stays
  // upright; it lies flat only below `minHeight`, as without them).
  const stackedHeight = toolbarMinHeight(density, paint === null ? 0 : paint.tools.length);
  // Only whether the view is too low is kept, not its height, and the bars are memoised: dragging the splitter re-renders this component
  // with every shell change, but the bars only when these flags, the anchor or their own props change. Measured before the first paint,
  // so a low view never shows the upright bar for a frame; a view that is not measured (jsdom has no layout) is not low.
  const [low, setLow] = useState(false);
  const [beside, setBeside] = useState(false);
  useLayoutEffect(() => {
    const el = box.current;
    if (!el) return;
    const measure = () => {
      const height = el.getBoundingClientRect().height;
      setLow(isLowView(height, minHeight));
      setBeside(isLowView(height, stackedHeight));
    };
    measure();
    if (typeof ResizeObserver === "undefined") return;
    const observer = new ResizeObserver(measure);
    observer.observe(el);
    return () => observer.disconnect();
  }, [minHeight, stackedHeight]);
  const area = useCallback(() => box.current?.getBoundingClientRect() ?? null, []);
  // The four event tools wait for a shown place and an event file that may be written.
  const eventsBlocked = eventToolsBlocked(eventLock, eventFileId);
  const anchor = shownAnchor(state.toolbar, low);
  // Height and ramp paint steps: choosing one shows the layer „Höhen“ (the rule of the layer bar hides „Gelände“ then); leaving them
  // keeps the layer as it is.
  const { onShowHeights } = layers;
  useEffect(() => {
    if (sub === "height" || sub === "ramp") onShowHeights(true);
  }, [sub, onShowHeights]);
  // „Raum anlegen“ puts the door outside into the selected area or room.
  const onRecipe = useCallback(
    (id: RecipeEntry["id"]) => {
      const request = recipeRequest(id, selected);
      if (request) onOpenWizard(request);
    },
    [selected, onOpenWizard],
  );
  return (
    <div ref={box} className={styles.overlays} data-scene-overlays="" data-anchor={anchor}>
      <SceneBar {...layers} topDown={state.topDown} onTopDown={setTopDown} />
      {!depthAtLeast(viewDepth, "bauen") ? (
        <RecipeBar entries={RECIPE_BAR} onRecipe={onRecipe} density={density} anchor={anchor} />
      ) : (
        <FloatingToolbar
          tools={tools}
          tool={tool}
          onTool={onTool}
          eventsBlocked={eventsBlocked}
          density={density}
          anchor={anchor}
          chosen={state.toolbar}
          onAnchor={setToolbar}
          area={area}
          paint={paint}
          paintBeside={beside}
        />
      )}
    </div>
  );
}
