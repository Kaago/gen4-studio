import { useEffect, useState } from "react";
import type { RefObject } from "react";

/** `grid`: items laid out in rows (a CSS grid of cards); Left and Right move by one, Up and Down by one row. */
export type RovingListOptions = { orientation?: "vertical" | "horizontal" | "grid" };

const ITEM = "[data-roving]";

/**
 * The number of items on the first row of a laid-out grid: those as high up on the page as the first one (at least 1). The page position,
 * not `offsetTop`: an item inside a positioned cell measures from that cell and is always at 0.
 */
function columnsOf(list: readonly HTMLElement[]): number {
  const top = (el: HTMLElement) => Math.round(el.getBoundingClientRect().top);
  const firstTop = list[0] === undefined ? 0 : top(list[0]);
  const next = list.findIndex((el) => top(el) !== firstTop);
  return next <= 0 ? Math.max(1, list.length) : next;
}

/**
 * Roving tabindex for a list: of all `[data-roving]` descendants exactly one has `tabindex="0"` (the one last focused,
 * else the selected one, else the first), the arrow keys of the orientation move it, Home and End jump to the ends.
 * In a grid Up and Down go to the item one row above or below; where there is none, the tab stop stays.
 * The host may mount after the first render (a panel that shows an empty state first) or be replaced by another element;
 * the hook picks up whatever element `ref` holds after each render. Rows may come and go.
 */
export function useRovingList(ref: RefObject<HTMLElement | null>, opts: RovingListOptions = {}): void {
  const orientation = opts.orientation ?? "vertical";
  // The element the listeners sit on. A ref change triggers no render, so every render checks whether the ref holds another one.
  const [host, setHost] = useState<HTMLElement | null>(null);
  useEffect(() => {
    if (ref.current !== host) setHost(ref.current);
  });
  useEffect(() => {
    if (!host) return;
    const items = () => [...host.querySelectorAll<HTMLElement>(ITEM)];
    const enabled = () => items().filter((el) => el.getAttribute("aria-disabled") !== "true");
    const makeActive = (active: HTMLElement) => {
      for (const el of items()) el.tabIndex = el === active ? 0 : -1;
    };
    const normalize = () => {
      const all = items();
      const current = all.find((el) => el.getAttribute("tabindex") === "0");
      const active = current ?? all.find((el) => el.getAttribute("aria-selected") === "true") ?? enabled()[0];
      if (active) makeActive(active);
    };
    normalize();

    const previous = orientation === "vertical" ? "ArrowUp" : "ArrowLeft";
    const next = orientation === "vertical" ? "ArrowDown" : "ArrowRight";
    const onKeyDown = (event: KeyboardEvent) => {
      const target = event.target;
      if (!(target instanceof HTMLElement) || !target.matches(ITEM)) return;
      if (event.altKey || event.ctrlKey || event.metaKey || event.shiftKey) return;
      const list = enabled();
      const at = list.indexOf(target);
      let to: HTMLElement | undefined;
      if (event.key === next) to = list[Math.min(at + 1, list.length - 1)];
      else if (event.key === previous) to = list[Math.max(at - 1, 0)];
      else if (orientation === "grid" && (event.key === "ArrowDown" || event.key === "ArrowUp")) {
        const step = columnsOf(list);
        to = list[event.key === "ArrowDown" ? at + step : at - step] ?? target;
      } else if (event.key === "Home") to = list[0];
      else if (event.key === "End") to = list[list.length - 1];
      else return;
      event.preventDefault();
      if (!to) return;
      makeActive(to);
      to.focus();
    };
    const onFocusIn = (event: FocusEvent) => {
      if (event.target instanceof HTMLElement && event.target.matches(ITEM)) makeActive(event.target);
    };
    host.addEventListener("keydown", onKeyDown);
    host.addEventListener("focusin", onFocusIn);
    const observer = new MutationObserver(normalize);
    observer.observe(host, { childList: true, subtree: true });
    return () => {
      host.removeEventListener("keydown", onKeyDown);
      host.removeEventListener("focusin", onFocusIn);
      observer.disconnect();
    };
  }, [host, orientation]);
}
