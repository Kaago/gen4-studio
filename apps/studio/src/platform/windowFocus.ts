import { getCurrentWindow } from "@tauri-apps/api/window";
import { inTauri } from "./transport";

/**
 * Calls `onFocus` whenever the window gets the focus back: inside the app when Tauri reports the window focused, in the browser mode on
 * the window's `focus` event and when the tab turns visible again. Returns the unsubscribe; nothing is called after it.
 */
export function subscribeWindowFocus(onFocus: () => void): () => void {
  return inTauri() ? listenToTauri(onFocus) : listenToBrowser(onFocus);
}

function listenToTauri(onFocus: () => void): () => void {
  let disposed = false;
  let unlisten: (() => void) | undefined;
  try {
    getCurrentWindow()
      .onFocusChanged(({ payload: focused }) => {
        if (focused && !disposed) onFocus();
      })
      .then((fn) => {
        if (disposed) fn();
        else unlisten = fn;
      })
      .catch(() => {
        /* not running inside a Tauri window */
      });
  } catch {
    /* the window API throws synchronously outside a Tauri window */
  }
  return () => {
    disposed = true;
    unlisten?.();
  };
}

function listenToBrowser(onFocus: () => void): () => void {
  const onVisibility = () => {
    if (document.visibilityState === "visible") onFocus();
  };
  window.addEventListener("focus", onFocus);
  document.addEventListener("visibilitychange", onVisibility);
  return () => {
    window.removeEventListener("focus", onFocus);
    document.removeEventListener("visibilitychange", onVisibility);
  };
}
