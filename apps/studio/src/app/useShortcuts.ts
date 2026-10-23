import { useEffect, useRef } from "react";
import type { MutableRefObject } from "react";
import { redo, undo } from "../api";
import type { EventKind } from "../api";
import type { InspectorModel } from "../workbench/Inspector";
import { inTauri } from "../platform/transport";
import type { Tool } from "../ui/frame/shellState";
import { toolForKey } from "../workbench/tools";
import { isTypingTarget, shortcutAction, wantsDelete } from "../world/buildings";

interface ShortcutHandlers {
  onSave: () => Promise<void>;
  /** "Hier spielen": save, build, run. Only asked for inside the app (in a browser Cmd+R reloads the page). */
  onPlay: () => Promise<void>;
  /** Cmd+O: asks for a project folder and opens it (asking first about unsaved edits, like the title bar's button). */
  onOpenProject: () => void;
  /** Cmd+K: opens the command palette. */
  onPalette: () => void;
  step: (action: typeof undo) => Promise<unknown>;
  onDeleteBuilding: (map: number, index: number) => void | Promise<void>;
  onDeleteEvent: (file: number, kind: EventKind, index: number) => void | Promise<void>;
  unpinSelection: () => void;
  /** Switches the tool of the scene (a letter, or "select" on Escape). */
  onTool: (tool: Tool) => void;
  /** Whether the world (the deck with the scene) is shown: only then do the tool letters and Escape-to-"Auswählen" act. */
  toolsActive: boolean;
}

/**
 * Whether a modal dialog or a wizard is open: their own keys (Escape, Delete, Cmd+Z in a field) are not the workbench's. A wizard is not
 * modal (the view stays usable), but while it works on the project nothing else may change it.
 */
const modalOpen = () => document.querySelector('[role="dialog"][aria-modal="true"], [data-wizard]') !== null;

/** Whether the key goes to something that takes letters for itself: a text field, or an open menu or list (type-ahead, Escape closes it). */
function inField(target: HTMLElement | null): boolean {
  return isTypingTarget(target) || target?.closest?.('[role="menu"], [role="listbox"], [role="combobox"]') != null;
}

/**
 * Cmd+O, Cmd+K, Cmd+S, Cmd+Z, Cmd+Shift+Z and (in the app) Cmd+R (see `shortcutAction`), the tool letters (V B F T A S G) and Escape back to
 * "Auswählen", and Delete / Escape on a pinned building or
 * event, none of them while a modal dialog or a wizard is open. Registered once; the handlers of the latest render are called through a ref.
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
      // The tools: a letter, or Escape back to "Auswählen" when nothing is selected (a selection consumed Escape above). Never typed into
      // a field, never with a modifier, and not in an open menu or list, where a letter searches and Escape closes. Only while the world
      // is shown: on another area a letter must not change the tool the world comes back with.
      if (latest.current.toolsActive && !inField(e.target as HTMLElement | null) && !e.metaKey && !e.ctrlKey && !e.altKey && !e.shiftKey && !e.repeat) {
        const tool = toolForKey(e.key);
        if (tool !== null) {
          latest.current.onTool(tool);
          return;
        }
        if (e.key === "Escape" && !e.defaultPrevented) {
          latest.current.onTool("select");
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
      } else if (action === "open") {
        // The browser's own "open file" stays away; a held key asks once.
        e.preventDefault();
        if (!e.repeat) latest.current.onOpenProject();
      } else if (action === "palette") {
        // The browser's own Cmd+K (search bar) stays away; a held key opens once.
        e.preventDefault();
        if (!e.repeat) latest.current.onPalette();
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
