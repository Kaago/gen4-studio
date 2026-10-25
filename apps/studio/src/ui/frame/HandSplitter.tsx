import { useRef } from "react";
import type { KeyboardEvent as ReactKeyboardEvent, PointerEvent as ReactPointerEvent } from "react";
import { useShell } from "./ShellContext";
import { HAND_MAX, HAND_MIN, clampHand } from "./shellState";
import type { AreaId } from "./shellState";
import styles from "./HandSplitter.module.css";

/** An arrow key moves the edge of the hand this far. */
const KEY_STEP = 16;

export type HandSplitterProps = {
  area: AreaId;
  /** The id of the dock the splitter sizes (the hand's). */
  controls: string;
  /** While a wizard holds the hand's place the splitter is not there: nothing to size or fold. */
  hidden?: boolean;
};

/**
 * The line between the view and the hand: dragging it sets the height of the hand, a double click (or Enter) folds the hand away and
 * back, the arrow keys, Home and End size it from the keyboard. It holds no state of its own beyond the drag; the height and the fold
 * are the area's, in the shell.
 */
export function HandSplitter({ area, controls, hidden = false }: HandSplitterProps) {
  const { state, setHandHeight, toggleHandCollapsed } = useShell();
  const drag = useRef<{ pointer: number; startY: number; startHeight: number } | null>(null);
  const height = state.handHeight[area];
  const collapsed = state.handCollapsed[area];
  if (hidden) return null;

  /** Sizes the hand; a folded hand opens to where it is sized. */
  function resize(next: number) {
    if (collapsed) toggleHandCollapsed(area);
    setHandHeight(area, clampHand(next));
  }
  function onPointerDown(e: ReactPointerEvent<HTMLDivElement>) {
    if (e.button !== 0) return;
    const bar = e.currentTarget;
    // The hand reaches from the splitter's lower edge to the bottom of the deck that holds both.
    const deckBottom = bar.parentElement?.getBoundingClientRect().bottom ?? 0;
    drag.current = { pointer: e.pointerId, startY: e.clientY, startHeight: deckBottom - bar.getBoundingClientRect().bottom };
    bar.setPointerCapture?.(e.pointerId);
  }
  function onPointerMove(e: ReactPointerEvent<HTMLDivElement>) {
    const d = drag.current;
    if (d === null || e.pointerId !== d.pointer) return;
    resize(d.startHeight + d.startY - e.clientY);
  }
  function endDrag(e: ReactPointerEvent<HTMLDivElement>) {
    const d = drag.current;
    if (d === null || e.pointerId !== d.pointer) return;
    drag.current = null;
    e.currentTarget.releasePointerCapture?.(e.pointerId);
  }
  function onKeyDown(e: ReactKeyboardEvent<HTMLDivElement>) {
    switch (e.key) {
      case "ArrowUp":
        resize(height + KEY_STEP);
        break;
      case "ArrowDown":
        resize(height - KEY_STEP);
        break;
      case "Home":
        resize(HAND_MAX);
        break;
      case "End":
        resize(HAND_MIN);
        break;
      case "Enter":
        toggleHandCollapsed(area);
        break;
      default:
        return;
    }
    e.preventDefault();
  }

  return (
    <div
      className={styles.splitter}
      role="separator"
      aria-orientation="horizontal"
      aria-label="Höhe der Hand"
      aria-controls={controls}
      aria-valuemin={HAND_MIN}
      aria-valuemax={HAND_MAX}
      aria-valuenow={collapsed ? 0 : height}
      aria-valuetext={collapsed ? "eingeklappt" : `${height} px`}
      tabIndex={0}
      title="Ziehen ändert die Höhe, Doppelklick klappt ein"
      data-splitter="hand"
      onPointerDown={onPointerDown}
      onPointerMove={onPointerMove}
      onPointerUp={endDrag}
      onPointerCancel={endDrag}
      onDoubleClick={() => toggleHandCollapsed(area)}
      onKeyDown={onKeyDown}
    />
  );
}
