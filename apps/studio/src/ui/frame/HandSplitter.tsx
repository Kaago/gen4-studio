import { useRef } from "react";
import { Splitter } from "../primitives";
import { useShell } from "./ShellContext";
import { HAND_MAX, HAND_MIN } from "./shellState";
import type { AreaId } from "./shellState";

export type HandSplitterProps = {
  area: AreaId;
  /** The id of the dock the splitter sizes (the hand's). */
  controls: string;
  /** While a wizard holds the hand's place the splitter is not there: nothing to size or fold. */
  hidden?: boolean;
};

/**
 * The line between the view and the hand: dragging it sets the height of the hand, a double click (or Enter) folds the hand away and
 * back, the arrow keys, Home and End size it from the keyboard. It holds no state of its own; the height and the fold are the area's, in
 * the shell. A folded hand keeps its height as the value: a drag or an arrow up (or Home) opens it from there, an arrow down (or End) has
 * nothing to lower.
 */
export function HandSplitter({ area, controls, hidden = false }: HandSplitterProps) {
  const { state, setHandHeight, setHandCollapsed, toggleHandCollapsed } = useShell();
  const height = state.handHeight[area];
  const collapsed = state.handCollapsed[area];
  // Whether the hand is still folded for the next event: a move that opens it must not leave the one after it, which arrives before React
  // rendered the opening, to fold the hand again or to be dropped.
  const folded = useRef(collapsed);
  folded.current = collapsed;
  if (hidden) return null;

  function onValue(next: number) {
    if (folded.current) {
      // Higher than the remembered height, or the highest: a lower value is an arrow down, End or a drag below the folded hand.
      if (next <= height && next !== HAND_MAX) return;
      folded.current = false;
      setHandCollapsed(area, false);
    }
    setHandHeight(area, next);
  }
  // The hand grows as the pointer goes up; the splitter only takes differences, so where the deck ends does not matter.
  const measure = (e: PointerEvent) => -e.clientY;

  return (
    <Splitter
      orientation="horizontal"
      grows="up"
      label="Höhe der Hand"
      controls={controls}
      value={height}
      min={HAND_MIN}
      max={HAND_MAX}
      valueText={collapsed ? "eingeklappt" : undefined}
      onValue={onValue}
      onDoubleClick={() => toggleHandCollapsed(area)}
      onEnter={() => toggleHandCollapsed(area)}
      measure={measure}
      title="Ziehen ändert die Höhe, Doppelklick klappt ein"
      dataAttrs={{ "data-splitter": "hand" }}
    />
  );
}
