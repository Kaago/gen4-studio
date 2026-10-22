import { useEffect, useRef } from "react";
import type { MutableRefObject } from "react";
import { redo, undo } from "../api";
import type { EventKind } from "../api";
import type { InspectorModel } from "../workbench/Inspector";
import { isTypingTarget, shortcutAction, wantsDelete } from "../world/buildings";

interface ShortcutHandlers {
  onSave: () => Promise<void>;
  step: (action: typeof undo) => Promise<void>;
  onDeleteBuilding: (map: number, index: number) => void | Promise<void>;
  onDeleteEvent: (file: number, kind: EventKind, index: number) => void | Promise<void>;
  unpinSelection: () => void;
}

/**
 * Cmd+S, Cmd+Z and Cmd+Shift+Z (see `shortcutAction`), and Delete / Escape on a pinned building or
 * event. Registered once; the handlers of the latest render are called through a ref.
 */
export function useShortcuts(pinnedRef: MutableRefObject<InspectorModel>, handlers: ShortcutHandlers) {
  // Shortcuts always call the handlers of the latest render.
  const latest = useRef(handlers);
  latest.current = handlers;
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const selected = pinnedRef.current;
      if ((selected.kind === "building" || selected.kind === "event") && !e.repeat) {
        // Delete (Backspace on a Mac keyboard) removes the selected building or event after asking;
        // neither it nor Escape touches a form field that is being typed in.
        if (wantsDelete({ key: e.key, target: e.target as HTMLElement | null, metaKey: e.metaKey, ctrlKey: e.ctrlKey, altKey: e.altKey })) {
          e.preventDefault();
          if (selected.kind === "building") void latest.current.onDeleteBuilding(selected.map, selected.index);
          else void latest.current.onDeleteEvent(selected.eventFile, selected.eventKind, selected.index);
          return;
        }
        if (e.key === "Escape" && !isTypingTarget(e.target as HTMLElement | null)) {
          latest.current.unpinSelection();
          return;
        }
      }
      const action = shortcutAction({ key: e.key, target: e.target as HTMLElement | null, metaKey: e.metaKey, ctrlKey: e.ctrlKey, altKey: e.altKey, shiftKey: e.shiftKey });
      if (action === "undo" || action === "redo") {
        e.preventDefault();
        void latest.current.step(action === "redo" ? redo : undo);
      } else if (action === "save") {
        e.preventDefault();
        void latest.current.onSave();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [pinnedRef]);
}
