import { useEffect } from "react";
import type { CSSProperties, RefObject } from "react";

/** The box of a menu or popover under its button (the caller adds `left` or `right`). */
export const popoverStyle: CSSProperties = {
  position: "absolute",
  top: "100%",
  zIndex: 10,
  marginTop: 4,
  background: "#fff",
  border: "1px solid #d1d5db",
  borderRadius: 6,
  boxShadow: "0 4px 12px rgba(0,0,0,0.15)",
};

/** While `open`, Escape and a press outside `ref`'s element call `close` (a popover or menu closes the way users expect). */
export function useDismiss(open: boolean, ref: RefObject<HTMLElement | null>, close: () => void): void {
  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") close();
    };
    const onPress = (e: MouseEvent) => {
      if (!ref.current?.contains(e.target as Node)) close();
    };
    document.addEventListener("keydown", onKey);
    document.addEventListener("mousedown", onPress);
    return () => {
      document.removeEventListener("keydown", onKey);
      document.removeEventListener("mousedown", onPress);
    };
  }, [open, ref, close]);
}
