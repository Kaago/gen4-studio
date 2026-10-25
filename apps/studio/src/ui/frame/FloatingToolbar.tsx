import { Fragment, memo, useId, useRef } from "react";
import type { PointerEvent as ReactPointerEvent } from "react";
import { isEventTool } from "../../workbench/tools";
import type { ToolSpec } from "../../workbench/tools";
import { Icon, Kbd, Menu, Tooltip, useRovingList } from "../primitives";
import type { Density } from "../tokens/theme";
import type { Tool, ToolbarAnchor } from "./shellState";
import styles from "./FloatingToolbar.module.css";

/**
 * A view lower than this shows a toolbar anchored at the left flat at the top: upright under the layer bar it would reach the legend at
 * the bottom left. The keys of "kompakt" are smaller, so its bar is shorter.
 */
const TOOLBAR_MIN_HEIGHT: Record<Density, number> = { komfort: 460, kompakt: 400 };

export function toolbarMinHeight(density: Density): number {
  return TOOLBAR_MIN_HEIGHT[density];
}

/** Whether a view `viewHeight` high is too low for the upright toolbar; 0 (not measured) is not. */
export function isLowView(viewHeight: number, minHeight: number): boolean {
  return viewHeight > 0 && viewHeight < minHeight;
}

/** A pointer that moved less than this far from where it was pressed did not drag the grip. */
const DRAG_SLOP = 4;

const ANCHOR_LABELS: Record<ToolbarAnchor, string> = { left: "Links", top: "Oben", bottom: "Unten" };
const ANCHORS: readonly ToolbarAnchor[] = ["left", "top", "bottom"];

/** Where the toolbar shows: an upright toolbar lies flat at the top in a low view (`isLowView`), the others stay. */
export function shownAnchor(anchor: ToolbarAnchor, low: boolean): ToolbarAnchor {
  return anchor === "left" && low ? "top" : anchor;
}

/** The anchor nearest to a point `y` px below the top of a view `height` high: the upper quarter is the top, the lower the bottom. */
export function anchorAt(y: number, height: number): ToolbarAnchor {
  if (y < height / 4) return "top";
  if (y > (3 * height) / 4) return "bottom";
  return "left";
}

export interface FloatingToolbarProps {
  tools: readonly ToolSpec[];
  tool: Tool;
  onTool: (tool: Tool) => void;
  /** Why the four event tools cannot be used (the locked file's reason, or that no place is shown); null: they can. */
  eventsBlocked: string | null;
  density: Density;
  /** Where the toolbar shows: upright at the left, flat at the top or at the bottom. */
  anchor: ToolbarAnchor;
  /** The anchor the user chose (the shell remembers it): the menu ticks this one, `anchor` differs from it in a low view. */
  chosen: ToolbarAnchor;
  /** A new anchor, chosen in the grip's menu or by dragging the grip. */
  onAnchor: (anchor: ToolbarAnchor) => void;
  /** The view the toolbar floats in, measured when a drag of the grip ends; without it a drag moves nothing. */
  area?: () => DOMRect | null;
}

/**
 * The tools of the 3D view as a bar floating over it: one key per tool with symbol and letter (the name is the accessible name and the
 * tooltip, the status line says it too), exactly one pressed, the groups apart by a line. A grip in front moves the bar to the nearest
 * anchor when dragged; the caret next to it chooses the anchor from a menu (the keyboard's way). Base UI opens a menu on the press of its
 * trigger, so the grip cannot be the trigger and a drag handle at once.
 */
export const FloatingToolbar = memo(function FloatingToolbar({ tools, tool, onTool, eventsBlocked, density, anchor, chosen, onAnchor, area }: FloatingToolbarProps) {
  const bar = useRef<HTMLDivElement>(null);
  const vertical = anchor === "left";
  useRovingList(bar, { orientation: vertical ? "vertical" : "horizontal" });
  const uid = useId();
  const drag = useRef<{ pointer: number; x: number; y: number; moved: boolean } | null>(null);

  function onPointerDown(e: ReactPointerEvent<HTMLSpanElement>) {
    if (e.button !== 0) return;
    drag.current = { pointer: e.pointerId, x: e.clientX, y: e.clientY, moved: false };
    e.currentTarget.setPointerCapture?.(e.pointerId);
  }
  function onPointerMove(e: ReactPointerEvent<HTMLSpanElement>) {
    const d = drag.current;
    if (d === null || e.pointerId !== d.pointer || d.moved) return;
    d.moved = Math.hypot(e.clientX - d.x, e.clientY - d.y) > DRAG_SLOP;
  }
  function endDrag(e: ReactPointerEvent<HTMLSpanElement>, drop: boolean) {
    const d = drag.current;
    if (d === null || e.pointerId !== d.pointer) return;
    drag.current = null;
    e.currentTarget.releasePointerCapture?.(e.pointerId);
    const rect = drop && d.moved ? (area?.() ?? null) : null;
    if (rect !== null && rect.height > 0) onAnchor(anchorAt(e.clientY - rect.top, rect.height));
  }

  return (
    <div
      ref={bar}
      role="toolbar"
      aria-label="Werkzeugleiste"
      aria-orientation={vertical ? "vertical" : "horizontal"}
      data-anchor={anchor}
      data-dichte={density}
      className={styles.bar}
    >
      <span className={styles.handle}>
        <span
          className={styles.grip}
          data-toolbar-grip=""
          // For the pointer only: the caret's menu next to it is the way of the keyboard and of assistive tech.
          aria-hidden="true"
          title="Ziehen verschiebt die Leiste"
          onPointerDown={onPointerDown}
          onPointerMove={onPointerMove}
          onPointerUp={(e) => endDrag(e, true)}
          onPointerCancel={(e) => endDrag(e, false)}
        >
          <Icon name="grip" />
        </span>
        <Menu
          trigger={<Icon name="caretDown" />}
          label="Lage der Werkzeugleiste"
          // The menu ticks the chosen anchor and always writes the entry; in a low view it says why "Links" shows flat on top.
          items={ANCHORS.map((a) => ({
            label: a === "left" && chosen === "left" && anchor !== "left" ? `${ANCHOR_LABELS[a]} (bei dieser Höhe oben)` : ANCHOR_LABELS[a],
            checked: a === chosen,
            onSelect: () => onAnchor(a),
          }))}
        />
      </span>
      {tools.map((t, at) => {
        // A locked tool stays in the tab order and says why (as tooltip and as description), but does nothing.
        const blocked = isEventTool(t.id) ? eventsBlocked : null;
        const reasonId = `${uid}-${t.id}`;
        const groupStarts = at > 0 && tools[at - 1]!.group !== t.group;
        return (
          <Fragment key={t.id}>
            {groupStarts && <span role="separator" aria-orientation={vertical ? "horizontal" : "vertical"} className={styles.separator} />}
            <Tooltip
              content={
                blocked ?? (
                  <span className={styles.tip}>
                    <span className={styles.tipHead}>
                      {t.label} <Kbd keys={t.key} />
                    </span>
                    <span className={styles.tipHint}>{t.hint}</span>
                  </span>
                )
              }
            >
              <button
                type="button"
                className={styles.tool}
                data-roving=""
                data-tool={t.id}
                aria-label={t.label}
                aria-pressed={tool === t.id}
                aria-disabled={blocked ? true : undefined}
                aria-describedby={blocked ? reasonId : undefined}
                aria-keyshortcuts={t.key}
                onClick={blocked ? undefined : () => onTool(t.id)}
              >
                <Icon name={t.icon} weight={tool === t.id ? "fill" : "regular"} />
                <span className={styles.key} aria-hidden="true">
                  {t.key}
                </span>
              </button>
            </Tooltip>
            {blocked && (
              <span id={reasonId} hidden>
                {blocked}
              </span>
            )}
          </Fragment>
        );
      })}
    </div>
  );
});
