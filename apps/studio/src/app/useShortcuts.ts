import { useEffect, useRef } from "react";
import type { MutableRefObject } from "react";
import { redo, undo } from "../api";
import type { EventKind } from "../api";
import type { InspectorModel } from "../workbench/Inspector";
import { inTauri } from "../platform/transport";
import type { AreaId, PokemonSub, Tool } from "../ui/frame/shellState";
import { isSubLocked } from "../data/dataModel";
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
  /** Option+Cmd+Left and Right in the world: shows the previous or the next scene of the history („Zurück“ and „Vor“ of „In dieser Szene“). */
  onSceneBack: () => void;
  onSceneForward: () => void;
  /** Cmd+1 to 4 in the Pokémon area: shows that part (never a locked one). */
  onSub: (sub: PokemonSub) => void;
  /**
   * The area that shows. Only in the world (the deck with the scene) do the tool letters, Escape-to-"Auswählen" and Delete or Escape on
   * the pinned building or event act; only in Pokémon do Cmd+1 to 4.
   */
  area: AreaId;
  /** Whether a tool is locked (the event tools without a place or with a locked event file): its letter does nothing. */
  toolLocked: (tool: Tool) => boolean;
  /** Whether the 3D view of the world is the one in front: only then do the tool letters act (not with Raster or Weltkarte shown). */
  toolsShown: () => boolean;
  /** A tool letter was pressed while the 3D view is hidden: the letter did nothing (the caller says so, once). */
  onToolsHidden: () => void;
  /** A trainer waits to be placed with the figure tool: Escape drops that binding first (back to "Auswählen"), before it unpins anything. */
  placing?: boolean;
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
 * Cmd+O, Cmd+K, Cmd+S, Cmd+Z, Cmd+Shift+Z and (in the app) Cmd+R (see `shortcutAction`), in the world the tool letters (V B F T A S G),
 * Escape back to "Auswählen", Delete / Escape on a pinned building or event and Option+Cmd+Left / Right through the scenes shown, in the Pokémon area Cmd+1 to 4 for its parts, none of them
 * while a modal dialog or a wizard is open. Registered once; the handlers of the latest render are called through a ref.
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
      const { area } = latest.current;
      const inWorld = area === "welt";
      // An armed trainer binding is what Escape ends first; the selection stays pinned.
      const plainKey = !e.metaKey && !e.ctrlKey && !e.altKey && !e.shiftKey && !e.repeat;
      if (latest.current.placing && inWorld && e.key === "Escape" && plainKey && !e.defaultPrevented && !inField(e.target as HTMLElement | null)) {
        latest.current.onTool("select");
        return;
      }
      // Only while the world is shown (on another area the selection stays pinned in the hidden world, the keys are not its), and not
      // when something else already handled the key (a document tab in "Offene Scripts" closes on Delete).
      if ((selected.kind === "building" || selected.kind === "event") && !e.repeat && inWorld && !e.defaultPrevented) {
        // Delete (Backspace on a Mac keyboard) removes the selected building or event after asking;
        // neither it nor Escape touches a form field that is being typed in.
        if (wantsDelete({ key: e.key, target: e.target as HTMLElement | null, metaKey: e.metaKey, ctrlKey: e.ctrlKey, altKey: e.altKey, area })) {
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
      if (inWorld && !inField(e.target as HTMLElement | null) && !e.metaKey && !e.ctrlKey && !e.altKey && !e.shiftKey && !e.repeat) {
        const tool = toolForKey(e.key);
        if (tool !== null) {
          // The palette's button of a locked tool cannot be pressed; its letter cannot switch to it either.
          if (!latest.current.toolLocked(tool)) {
            // The tools belong to the 3D view; with Raster or Weltkarte in front the letter is consumed, but changes nothing.
            if (latest.current.toolsShown()) latest.current.onTool(tool);
            else latest.current.onToolsHidden();
          }
          return;
        }
        if (e.key === "Escape" && !e.defaultPrevented) {
          latest.current.onTool("select");
          return;
        }
      }
      // „Zurück“ and „Vor“ of the scenes shown: only in the world (the scene they show), never in a field, where the arrows move the caret.
      if (inWorld && (e.metaKey || e.ctrlKey) && e.altKey && !e.shiftKey && (e.key === "ArrowLeft" || e.key === "ArrowRight") && !inField(e.target as HTMLElement | null)) {
        e.preventDefault();
        if (!e.repeat) (e.key === "ArrowLeft" ? latest.current.onSceneBack : latest.current.onSceneForward)();
        return;
      }
      const action = shortcutAction({ key: e.key, target: e.target as HTMLElement | null, metaKey: e.metaKey, ctrlKey: e.ctrlKey, altKey: e.altKey, shiftKey: e.shiftKey, inApp: inTauri(), area });
      if (action === "pokedex" || action === "trainer" || action === "encounters" || action === "moves") {
        // The browser's own Cmd+digit (switching tabs) stays away; a locked part's key does nothing, like its menu entry.
        e.preventDefault();
        if (!e.repeat && !isSubLocked(action)) latest.current.onSub(action);
      } else if (action === "undo" || action === "redo") {
        e.preventDefault();
        void latest.current.step(action === "redo" ? redo : undo);
      } else if (action === "save") {
        e.preventDefault();
        void latest.current.onSave();
      } else if (action === "open" || action === "palette") {
        // A key an editor already used is not ours: on a Mac CodeMirror's Ctrl+K deletes to the line end, Ctrl+O splits the line.
        if (e.defaultPrevented) return;
        // The browser's own "open file" and Cmd+K (search bar) stay away; a held key asks or opens once.
        e.preventDefault();
        if (e.repeat) return;
        if (action === "open") latest.current.onOpenProject();
        else latest.current.onPalette();
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
