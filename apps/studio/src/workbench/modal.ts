import { useEffect } from "react";
import type { RefObject } from "react";

/**
 * Whether Escape should close a modal dialog. Not when something already used the key (a `DraftInput` taking back
 * what was typed calls `preventDefault`), not in a select or text area, and not in a text field that still holds
 * text (a search field clears itself first). An unedited `DraftInput` (marked `data-draft`) lets the dialog close.
 */
export function escapeCloses(e: KeyboardEvent): boolean {
  if (e.key !== "Escape" || e.defaultPrevented) return false;
  const target = e.target;
  if (!(target instanceof HTMLElement)) return true;
  if (target.matches("[data-draft]")) return true;
  if (target.matches("select, textarea")) return false;
  if (target instanceof HTMLInputElement && !["radio", "checkbox", "button", "submit", "reset"].includes(target.type)) return target.value === "";
  return true;
}

const FOCUSABLE = 'a[href], button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])';

/**
 * Keeps Tab and Shift+Tab inside `ref`'s element while it is mounted (the last control wraps to the first and back),
 * and gives the focus back to what had it before when it goes away.
 */
export function useFocusTrap(ref: RefObject<HTMLElement | null>): void {
  useEffect(() => {
    const before = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    const onKey = (e: KeyboardEvent) => {
      const box = ref.current;
      if (e.key !== "Tab" || !box) return;
      const controls = [...box.querySelectorAll<HTMLElement>(FOCUSABLE)];
      if (controls.length === 0) {
        e.preventDefault();
        return;
      }
      const first = controls[0];
      const last = controls[controls.length - 1];
      const at = document.activeElement;
      const inside = at instanceof Node && box.contains(at);
      if (e.shiftKey && (!inside || at === first || at === box)) {
        e.preventDefault();
        last.focus();
      } else if (!e.shiftKey && (!inside || at === last)) {
        e.preventDefault();
        first.focus();
      }
    };
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("keydown", onKey);
      if (before?.isConnected) before.focus();
    };
  }, [ref]);
}
