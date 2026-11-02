import { listen } from "@tauri-apps/api/event";
import { OFF_STATUS, liveStatus } from "../api.live";
import type { LiveStatusDto } from "../api.live";
import { inTauri } from "./transport";

export { OFF_STATUS };

/** How often the browser mode asks for the status while play mode is off (the bridge has no pushed events). */
export const LIVE_POLL_MS = 1000;
/** How often it asks while play mode is on (any state but `off`). */
export const LIVE_POLL_ACTIVE_MS = 500;

/**
 * Follows play mode: `onStatus` gets every status, the latest one counts. Inside the app Tauri emits each one as `live-event`, and the
 * status is read once when listening began (an event that arrives before that read answers wins over it). The browser mode polls
 * `liveStatus`, each poll `LIVE_POLL_MS` (`LIVE_POLL_ACTIVE_MS` while not off) after the previous one answered. Errors are silent.
 * Returns the unsubscribe; nothing arrives after it, not even the answer of a read under way.
 */
export function subscribeLive(onStatus: (s: LiveStatusDto) => void): () => void {
  return inTauri() ? listenToTauri(onStatus) : pollTheBridge(onStatus);
}

function listenToTauri(onStatus: (s: LiveStatusDto) => void): () => void {
  let disposed = false;
  let unlisten: (() => void) | undefined;
  // An event newer than the first read: that read's answer is older and must not overwrite it.
  let heard = false;
  listen<LiveStatusDto>("live-event", (e) => {
    if (disposed) return;
    heard = true;
    onStatus(e.payload);
  })
    .then((fn) => {
      if (disposed) {
        fn();
        return;
      }
      unlisten = fn;
      // Listening now: the status as it is, so a connection made before is shown.
      liveStatus().then(
        (status) => {
          if (!disposed && !heard) onStatus(status);
        },
        () => {
          /* silent: the next event brings the status */
        },
      );
    })
    .catch(() => {
      /* not running inside a Tauri window */
    });
  return () => {
    disposed = true;
    unlisten?.();
  };
}

function pollTheBridge(onStatus: (s: LiveStatusDto) => void): () => void {
  let disposed = false;
  let timer: ReturnType<typeof setTimeout> | undefined;
  let active = false;
  const poll = async () => {
    try {
      const status = await liveStatus();
      if (disposed) return;
      active = status.state !== "off";
      onStatus(status);
    } catch {
      // Silent: the bridge may be restarting; the next poll tries again.
    }
    if (!disposed) timer = setTimeout(() => void poll(), active ? LIVE_POLL_ACTIVE_MS : LIVE_POLL_MS);
  };
  void poll();
  return () => {
    disposed = true;
    clearTimeout(timer);
  };
}
