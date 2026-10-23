import { useEffect } from "react";
import type { MutableRefObject } from "react";
import { inTauri, invoke } from "../platform/transport";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { ask } from "../platform/dialogs";
import { errorText } from "../api";
import type { Message } from "./message";

/** Closing or quitting the app asks before unsaved edits (`dirtyRef`, read when asked) are discarded. */
export function useWindowGuards(dirtyRef: MutableRefObject<number>, setMessage: (message: Message) => void) {
  // Closing the window (button, Cmd+Q) asks before discarding unsaved edits.
  useEffect(() => {
    // The Tauri window API throws synchronously without Tauri (plain browser).
    if (!inTauri()) return;
    let unlisten: (() => void) | undefined;
    let disposed = false;
    getCurrentWindow()
      .onCloseRequested(async (e) => {
        if (dirtyRef.current > 0 && !(await ask("Ungespeicherte Änderungen verwerfen?", { kind: "warning" }))) {
          e.preventDefault();
        }
      })
      .then((fn) => {
        if (disposed) fn();
        else unlisten = fn;
      })
      .catch(() => {
        /* not running inside a Tauri window */
      });
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [dirtyRef]);

  // Cmd+Q does not close the window first: the backend holds the exit back and asks here.
  useEffect(() => {
    if (!inTauri()) return;
    let unlisten: (() => void) | undefined;
    let disposed = false;
    let asking = false;
    listen("confirm-quit", async () => {
      if (asking) return;
      asking = true;
      try {
        if (await ask("Ungespeicherte Änderungen verwerfen und beenden?", { kind: "warning" })) {
          await invoke("force_quit");
        }
      } catch (e) {
        setMessage({ kind: "error", text: errorText(e) });
      } finally {
        asking = false;
      }
    })
      .then((fn) => {
        if (disposed) fn();
        else unlisten = fn;
      })
      .catch(() => {
        /* not running inside a Tauri window */
      });
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [setMessage]);
}
