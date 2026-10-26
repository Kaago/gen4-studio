import { useRef } from "react";
import type { KeyboardEvent as ReactKeyboardEvent, PointerEvent as ReactPointerEvent } from "react";
import styles from "./Splitter.module.css";

/** A pointer that moved less than this far from where it was pressed did not drag (the toolbar's grip counts the same way). */
export const DRAG_SLOP = 4;

/** An arrow key moves the value this far unless the splitter says otherwise. */
const KEY_STEP = 16;

const ARROW = { up: "ArrowUp", down: "ArrowDown", left: "ArrowLeft", right: "ArrowRight" } as const;
const OPPOSITE = { up: "down", down: "up", left: "right", right: "left" } as const;

type SplitterAxis =
  | {
      /** A horizontal line between a part above and one below: the up and down arrows move it. */
      orientation: "horizontal";
      /** Which arrow enlarges the value. */
      grows: "up" | "down";
    }
  | {
      /** A vertical line between a part to its left and one to its right: the left and right arrows move it. */
      orientation: "vertical";
      grows: "left" | "right";
    };

export type SplitterProps = SplitterAxis & {
  label: string;
  /** The id of the part the splitter sizes. */
  controls?: string;
  /** The size in px of the part the splitter sizes; the owner holds it. */
  value: number;
  min: number;
  max: number;
  /** What the value reads as to assistive tech; `${value} px` when left out. */
  valueText?: string;
  step?: number;
  /** A new size, always inside `min..max`: from a drag, an arrow, Home or End. */
  onValue: (value: number) => void;
  onDoubleClick?: () => void;
  onEnter?: () => void;
  /** The value for the pointer's position. Only differences count: a drag moves the value by what `measure` changes from the press. */
  measure: (e: PointerEvent) => number;
  title?: string;
  dataAttrs?: Record<string, string>;
};

/**
 * The line that sizes the part it borders: a drag with the pointer (it begins after `DRAG_SLOP` px), the arrows, Home (`max`) and End
 * (`min`). It holds no value of its own, only the drag: the owner passes the value and takes the new one. Enter and a double click are the
 * owner's too (folding, say).
 */
export function Splitter({ label, controls, value, min, max, valueText, step = KEY_STEP, onValue, onDoubleClick, onEnter, measure, title, dataAttrs, ...axis }: SplitterProps) {
  const drag = useRef<{ pointer: number; x: number; y: number; start: number; from: number; moved: boolean } | null>(null);
  // The value a key press steps from: the newest one it reported. A second press before the owner rendered the first must not start from
  // the old value; once a render (or the end of the task, when the owner took nothing) arrives the owner's value is the truth again.
  const rendered = useRef(value);
  const latest = useRef(value);
  rendered.current = value;
  latest.current = value;

  const clamp = (v: number) => Math.min(max, Math.max(min, v));
  function report(next: number) {
    latest.current = next;
    queueMicrotask(() => (latest.current = rendered.current));
    onValue(next);
  }
  function onPointerDown(e: ReactPointerEvent<HTMLDivElement>) {
    if (e.button !== 0) return;
    drag.current = { pointer: e.pointerId, x: e.clientX, y: e.clientY, start: value, from: measure(e.nativeEvent), moved: false };
    e.currentTarget.setPointerCapture?.(e.pointerId);
  }
  function onPointerMove(e: ReactPointerEvent<HTMLDivElement>) {
    const d = drag.current;
    if (d === null || e.pointerId !== d.pointer) return;
    if (!d.moved) {
      if (Math.hypot(e.clientX - d.x, e.clientY - d.y) <= DRAG_SLOP) return;
      d.moved = true;
    }
    onValue(clamp(d.start + measure(e.nativeEvent) - d.from));
  }
  function endDrag(e: ReactPointerEvent<HTMLDivElement>) {
    const d = drag.current;
    if (d === null || e.pointerId !== d.pointer) return;
    drag.current = null;
    e.currentTarget.releasePointerCapture?.(e.pointerId);
  }
  function onKeyDown(e: ReactKeyboardEvent<HTMLDivElement>) {
    // Only the two arrows of its own axis are the splitter's; `grows` names the one that enlarges.
    if (e.key === ARROW[axis.grows]) report(clamp(latest.current + step));
    else if (e.key === ARROW[OPPOSITE[axis.grows]]) report(clamp(latest.current - step));
    else if (e.key === "Home") report(max);
    else if (e.key === "End") report(min);
    else if (e.key === "Enter" && onEnter !== undefined) onEnter();
    else return;
    e.preventDefault();
  }

  return (
    <div
      className={axis.orientation === "horizontal" ? styles.horizontal : styles.vertical}
      role="separator"
      aria-orientation={axis.orientation}
      aria-label={label}
      aria-controls={controls}
      aria-valuemin={min}
      aria-valuemax={max}
      aria-valuenow={value}
      aria-valuetext={valueText ?? `${value} px`}
      tabIndex={0}
      title={title}
      {...dataAttrs}
      onPointerDown={onPointerDown}
      onPointerMove={onPointerMove}
      onPointerUp={endDrag}
      onPointerCancel={endDrag}
      onDoubleClick={onDoubleClick}
      onKeyDown={onKeyDown}
    />
  );
}
