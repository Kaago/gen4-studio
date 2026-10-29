// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from "vitest";

const env = vi.hoisted(() => ({ tauri: false }));
vi.mock("./transport", () => ({ inTauri: () => env.tauri }));
const focusHandlers = vi.hoisted(() => [] as ((e: { payload: boolean }) => void)[]);
const unlisten = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({
    onFocusChanged: async (fn: (e: { payload: boolean }) => void) => {
      focusHandlers.push(fn);
      return unlisten;
    },
  }),
}));

import { subscribeWindowFocus } from "./windowFocus";

/** Sets what `document.visibilityState` answers. */
function setVisibility(state: DocumentVisibilityState) {
  Object.defineProperty(document, "visibilityState", { configurable: true, get: () => state });
}

describe("subscribeWindowFocus", () => {
  afterEach(() => {
    env.tauri = false;
    focusHandlers.length = 0;
    unlisten.mockReset();
    setVisibility("visible");
  });

  it("browser_focus_and_visible_call_back_and_unsubscribe_stops", () => {
    const onFocus = vi.fn();
    const stop = subscribeWindowFocus(onFocus);
    window.dispatchEvent(new Event("focus"));
    expect(onFocus).toHaveBeenCalledTimes(1);
    // A tab that turns visible again counts as a focus; one that hides does not.
    setVisibility("hidden");
    document.dispatchEvent(new Event("visibilitychange"));
    expect(onFocus).toHaveBeenCalledTimes(1);
    setVisibility("visible");
    document.dispatchEvent(new Event("visibilitychange"));
    expect(onFocus).toHaveBeenCalledTimes(2);
    stop();
    window.dispatchEvent(new Event("focus"));
    document.dispatchEvent(new Event("visibilitychange"));
    expect(onFocus).toHaveBeenCalledTimes(2);
  });

  it("tauri_calls_back_when_the_window_gains_focus_and_unlistens", async () => {
    env.tauri = true;
    const onFocus = vi.fn();
    const stop = subscribeWindowFocus(onFocus);
    await Promise.resolve();
    expect(focusHandlers).toHaveLength(1);
    focusHandlers[0]!({ payload: false });
    expect(onFocus).not.toHaveBeenCalled();
    focusHandlers[0]!({ payload: true });
    expect(onFocus).toHaveBeenCalledTimes(1);
    // The browser's own focus event is not listened to inside the app (the window event says it).
    window.dispatchEvent(new Event("focus"));
    expect(onFocus).toHaveBeenCalledTimes(1);
    stop();
    expect(unlisten).toHaveBeenCalledOnce();
    focusHandlers[0]!({ payload: true });
    expect(onFocus).toHaveBeenCalledTimes(1);
  });

  it("tauri_unsubscribed_before_listening_unlistens_once_it_listens", async () => {
    env.tauri = true;
    const stop = subscribeWindowFocus(vi.fn());
    stop();
    await Promise.resolve();
    await Promise.resolve();
    expect(unlisten).toHaveBeenCalledOnce();
  });
});
