import { useEffect, useRef } from "react";
import type { AreaId } from "../ui/frame/shellState";
import { OPEN_MODAL } from "../ui/primitives/Dialog";

/** What the keyboard can reach by Tab: the first of these in a stop is its target when nothing in it keeps the roving `tabIndex` 0. */
const FOCUSABLE = 'a[href], button:not([disabled]), input:not([disabled]):not([type="hidden"]), select:not([disabled]), textarea:not([disabled]), [contenteditable="true"], [tabindex]:not([tabindex="-1"])';

/** The atlas (`ui/frame/Atlas`) and the hinge (`ui/frame/Hinge`), by their data attributes: their `aria-label` follows the language shown. */
const ATLAS = "nav[data-atlas]";
const HINGE = '[role="toolbar"][data-hinge]';

/** Whether a modal dialog is open (the palette, the help, a question): its focus stays in it, F6 is not the frame's. */
const modalOpen = (root: Document) => root.querySelector(OPEN_MODAL) !== null;

/**
 * Whether `el` shows: not inside a `hidden` part (another area, a folded list), not hidden by an inline style (dockview hides a panel's
 * overlay by `visibility`), and not in a split view dockview collapsed for a maximized group (its `.dv-view` loses the class `visible`).
 */
function shown(el: HTMLElement): boolean {
  for (let node: HTMLElement | null = el; node !== null; node = node.parentElement) {
    if (node.hidden || node.style.display === "none" || node.style.visibility === "hidden") return false;
    if (node.classList.contains("dv-view") && !node.classList.contains("visible")) return false;
  }
  return true;
}

/** The first part of `root` matching `selector` that shows. */
const firstShown = (root: ParentNode, selector: string) => [...root.querySelectorAll<HTMLElement>(selector)].find(shown) ?? null;

/** The content of the active panel of a dock group: dockview renders it over the group (`renderer: "always"`), not inside it. */
function activeContent(group: HTMLElement): HTMLElement | null {
  const id = group.querySelector<HTMLElement>(".dv-active-tab [data-dock-tab]")?.dataset.dockTab;
  const dock = group.closest("[data-dock]");
  if (id === undefined || dock === null) return null;
  return dock.querySelector<HTMLElement>(`[data-dock-panel="${CSS.escape(id)}"]`);
}

/**
 * The stops of the F6 round in the shown area `area`, in their order: the atlas, the area's open list, the hinge, every dock group that
 * shows (the docked ones in the order of the grid, which is the order they read in, then the floating ones), the wizard's step under the
 * dock while a wizard is open, a surface that covers the areas (Start, Technik, Einstellungen), the status line. Parts of hidden areas, a folded list and groups a maximized group hides are left out.
 */
export function f6Targets(root: Document, area: AreaId): HTMLElement[] {
  const stops: (HTMLElement | null)[] = [root.querySelector<HTMLElement>(ATLAS)];
  const list = root.querySelector<HTMLElement>(`[data-area="${area}"][data-part="list"]`);
  // The list's own section (its head and content), not the sections inside it; folded, it is hidden.
  const listBody = list?.querySelector<HTMLElement>("section") ?? null;
  if (listBody !== null && shown(listBody)) stops.push(listBody);
  const deck = root.querySelector<HTMLElement>(`main[data-area="${area}"][data-part="deck"]`);
  if (deck !== null && shown(deck)) {
    stops.push(deck.querySelector<HTMLElement>(HINGE));
    const groups = [...deck.querySelectorAll<HTMLElement>(`#dock-${area} [data-dock-group]`)].filter(shown);
    const floating = (g: HTMLElement) => g.closest(".dv-floating-overlay-host") !== null;
    stops.push(...groups.filter((g) => !floating(g)), ...groups.filter(floating));
    // The strip under the dock with a wizard's step (hidden while none is open).
    stops.push(deck.querySelector<HTMLElement>("[data-wizard-hand]"));
  }
  stops.push(firstShown(root, "main:not([data-area])"));
  const status = root.querySelector<HTMLElement>("[data-status-line]");
  stops.push(status?.closest("footer") ?? status);
  return stops.filter((stop): stop is HTMLElement => stop !== null && shown(stop));
}

/** Whether `el` lies in `stop`; for a dock group also in its active panel's content. */
function within(stop: HTMLElement, el: Element): boolean {
  if (stop.contains(el)) return true;
  return stop.hasAttribute("data-dock-group") && activeContent(stop)?.contains(el) === true;
}

/**
 * Where the focus goes in a stop: the element that keeps the roving `tabIndex` 0 (a tree row, a toolbar's button), else the atlas's entry
 * of the shown area, else the first focusable part; in a dock group all that in the active panel's content first, then its active tab.
 * A stop with nothing to focus takes the focus itself (with `tabIndex` -1, out of the Tab order).
 */
function targetIn(stop: HTMLElement): HTMLElement {
  const current = (part: HTMLElement) => [...part.querySelectorAll<HTMLElement>('[aria-current="page"]')].find((el) => shown(el) && el.matches(FOCUSABLE)) ?? null;
  const pick = (part: HTMLElement) => firstShown(part, '[tabindex="0"]') ?? current(part) ?? firstShown(part, FOCUSABLE);
  if (stop.hasAttribute("data-dock-group")) {
    const content = activeContent(stop);
    const inContent = content === null ? null : pick(content);
    if (inContent !== null) return inContent;
    const tab = stop.querySelector<HTMLElement>(".dv-tab.dv-active-tab");
    if (tab !== null) return tab;
  }
  const target = pick(stop);
  if (target !== null) return target;
  if (!stop.hasAttribute("tabindex")) stop.tabIndex = -1;
  return stop;
}

/** The stop after (or before) the one the focus is in; from outside every stop, the next one in the document (the first, the last). */
function nextStop(stops: HTMLElement[], active: Element | null, backward: boolean): HTMLElement | undefined {
  if (stops.length === 0) return undefined;
  const at = active === null ? -1 : stops.findIndex((stop) => within(stop, active));
  if (at >= 0) return stops[(at + (backward ? -1 : 1) + stops.length) % stops.length];
  const outside = active === null || active === active.ownerDocument.body;
  const after = (stop: HTMLElement) => !outside && (active!.compareDocumentPosition(stop) & Node.DOCUMENT_POSITION_FOLLOWING) !== 0;
  if (backward) return [...stops].reverse().find((stop) => !outside && !after(stop)) ?? stops[stops.length - 1];
  return stops.find(after) ?? stops[0];
}

/** The area the atlas marks as shown. */
const shownArea = (root: Document) => root.querySelector<HTMLElement>(`${ATLAS} [aria-current="page"]`)?.dataset.area as AreaId | undefined;

/**
 * F6 moves the focus to the next part of the frame, Shift+F6 to the previous one, round and round (`f6Targets`): also out of a field,
 * whose characters the key never reaches. Not while a modal dialog is open or `enabled` says no. Registered once (in the capture phase,
 * before a field or an editor sees the key); `enabled` of the latest render is asked.
 */
export function useF6Ring(enabled: () => boolean): void {
  const latest = useRef(enabled);
  latest.current = enabled;
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== "F6" || e.metaKey || e.ctrlKey || e.altKey) return;
      if (!latest.current() || modalOpen(document)) return;
      const stop = nextStop(f6Targets(document, shownArea(document) ?? "start"), document.activeElement, e.shiftKey);
      if (stop === undefined) return;
      e.preventDefault();
      targetIn(stop).focus();
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, []);
}
