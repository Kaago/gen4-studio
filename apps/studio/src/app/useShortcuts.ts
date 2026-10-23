import { useEffect, useRef } from "react";
import type { MutableRefObject } from "react";
import { redo, undo } from "../api";
import type { EventKind } from "../api";
import type { InspectorModel } from "../workbench/Inspector";
import { inTauri } from "../platform/transport";
import { isTypingTarget, shortcutAction, wantsDelete } from "../world/buildings";

interface ShortcutHandlers {
  onSave: () => Promise<void>;
  /** "Hier spielen": save, build, run. Only asked for inside the app (in a browser Cmd+R reloads the page). */
  onPlay: () => Promise<void>;
  step: (action: typeof undo) => Promise<unknown>;
  onDeleteBuilding: (map: number, index: number) => void | Promise<void>;
  onDeleteEvent: (file: number, kind: EventKind, index: number) => void | Promise<void>;
  unpinSelection: () => void;
}

/** Whether a modal dialog is open: its own keys (Escape, Delete, Cmd+Z in a field) are not the workbench's. */
const modalOpen = () => document.querySelector('[role="dialog"][aria-modal="true"]') !== null;

/**
 * Cmd+S, Cmd+Z, Cmd+Shift+Z and (in the app) Cmd+R (see `shortcutAction`), and Delete / Escape on a pinned building or
 * event, none of them while a modal dialog is open. Registered once; the handlers of the latest render are called through a ref.
 */
export function useShortcuts(pinnedRef: MutableRefObject<InspectorModel>, handlers: ShortcutHandlers) {
  // Shortcuts always call the handlers of the latest render.
  const latest = useRef(handlers);
  latest.current = handlers;
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      // A wizard or dialog covers the workbench: Escape must not unpin, Delete must not delete and undo must not run behind it.
      if (modalOpen()) return;
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
      const action = shortcutAction({ key: e.key, target: e.target as HTMLElement | null, metaKey: e.metaKey, ctrlKey: e.ctrlKey, altKey: e.altKey, shiftKey: e.shiftKey, inApp: inTauri() });
      if (action === "undo" || action === "redo") {
        e.preventDefault();
        void latest.current.step(action === "redo" ? redo : undo);
      } else if (action === "save") {
        e.preventDefault();
        void latest.current.onSave();
      } else if (action === "play") {
        e.preventDefault();
        // A held key must not start a build per repeat.
        if (!e.repeat) void latest.current.onPlay();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [pinnedRef]);
}
