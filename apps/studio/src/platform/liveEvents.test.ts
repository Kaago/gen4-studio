import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const invoke = vi.hoisted(() => vi.fn());
const tauri = vi.hoisted(() => ({ on: false }));
const listen = vi.hoisted(() => vi.fn());
vi.mock("./transport", () => ({ invoke, inTauri: () => tauri.on }));
vi.mock("@tauri-apps/api/event", () => ({ listen }));

import type { LiveState, LiveStatusDto } from "../api";
import { LIVE_POLL_ACTIVE_MS, LIVE_POLL_MS, OFF_STATUS, subscribeLive } from "./liveEvents";

const status = (state: LiveState, reason: string | null = null): LiveStatusDto => ({ ...OFF_STATUS, state, reason });

/** Lets the pending promises settle without moving the clock. */
const settle = () => vi.advanceTimersByTimeAsync(0);

describe("subscribeLive", () => {
  let handler: (e: { payload: LiveStatusDto }) => void = () => {};
  const unlisten = vi.fn();

  beforeEach(() => {
    vi.useFakeTimers();
    invoke.mockReset();
    listen.mockReset();
    unlisten.mockReset();
    listen.mockImplementation(async (_name: string, fn: typeof handler) => {
      handler = fn;
      return unlisten;
    });
  });
  afterEach(() => vi.useRealTimers());

  it("listens_in_the_app_and_polls_in_the_browser_faster_while_active", async () => {
    // The app: one read once listening, then every status as Tauri emits it; no polling.
    tauri.on = true;
    invoke.mockResolvedValue(status("off"));
    const inApp = vi.fn();
    const stopApp = subscribeLive(inApp);
    await settle();
    expect(listen).toHaveBeenCalledWith("live-event", expect.any(Function));
    expect(invoke).toHaveBeenCalledTimes(1);
    expect(invoke).toHaveBeenLastCalledWith("live_status", {});
    expect(inApp).toHaveBeenLastCalledWith(status("off"));
    handler({ payload: status("connecting") });
    expect(inApp).toHaveBeenLastCalledWith(status("connecting"));
    await vi.advanceTimersByTimeAsync(LIVE_POLL_MS * 4);
    expect(invoke).toHaveBeenCalledTimes(1);
    stopApp();
    expect(unlisten).toHaveBeenCalledTimes(1);

    // The browser: a poll every 1000 ms while off, every 500 ms while not.
    expect(LIVE_POLL_MS).toBe(1000);
    expect(LIVE_POLL_ACTIVE_MS).toBe(500);
    tauri.on = false;
    invoke.mockReset();
    listen.mockClear();
    const answers = [status("off"), status("live"), status("live"), status("off")];
    invoke.mockImplementation(async () => answers.shift() ?? status("off"));
    const inBrowser = vi.fn();
    const stop = subscribeLive(inBrowser);
    await settle();
    expect(listen).not.toHaveBeenCalled();
    expect(invoke).toHaveBeenCalledTimes(1);
    expect(inBrowser).toHaveBeenLastCalledWith(status("off"));
    await vi.advanceTimersByTimeAsync(LIVE_POLL_MS - 1);
    expect(invoke).toHaveBeenCalledTimes(1);
    await vi.advanceTimersByTimeAsync(1);
    expect(invoke).toHaveBeenCalledTimes(2);
    expect(inBrowser).toHaveBeenLastCalledWith(status("live"));
    // Live: the next poll comes after 500 ms.
    await vi.advanceTimersByTimeAsync(LIVE_POLL_ACTIVE_MS);
    expect(invoke).toHaveBeenCalledTimes(3);
    await vi.advanceTimersByTimeAsync(LIVE_POLL_ACTIVE_MS);
    expect(invoke).toHaveBeenCalledTimes(4);
    expect(inBrowser).toHaveBeenLastCalledWith(status("off"));
    // Off again: back to 1000 ms.
    await vi.advanceTimersByTimeAsync(LIVE_POLL_ACTIVE_MS);
    expect(invoke).toHaveBeenCalledTimes(4);
    await vi.advanceTimersByTimeAsync(LIVE_POLL_ACTIVE_MS);
    expect(invoke).toHaveBeenCalledTimes(5);
    stop();
    await vi.advanceTimersByTimeAsync(LIVE_POLL_MS * 4);
    expect(invoke).toHaveBeenCalledTimes(5);
  });

  it("an_event_wins_over_the_first_read_answering_late_in_the_app", async () => {
    tauri.on = true;
    let resolve: (s: LiveStatusDto) => void = () => {};
    invoke.mockImplementation(() => new Promise<LiveStatusDto>((r) => (resolve = r)));
    const onStatus = vi.fn();
    const stop = subscribeLive(onStatus);
    await settle();
    handler({ payload: status("live") });
    resolve(status("off"));
    await settle();
    expect(onStatus.mock.calls).toEqual([[status("live")]]);
    stop();
  });

  it("nothing_arrives_after_unsubscribe", async () => {
    // The app: stopped before listening began, and a read answering after the stop.
    tauri.on = true;
    invoke.mockResolvedValue(status("live"));
    const early = vi.fn();
    subscribeLive(early)();
    await settle();
    expect(unlisten).toHaveBeenCalledTimes(1);
    expect(early).not.toHaveBeenCalled();
    // The browser: a poll under way when stopped.
    tauri.on = false;
    let resolve: (s: LiveStatusDto) => void = () => {};
    invoke.mockReset();
    invoke.mockImplementation(() => new Promise<LiveStatusDto>((r) => (resolve = r)));
    const late = vi.fn();
    const stop = subscribeLive(late);
    await settle();
    stop();
    resolve(status("live"));
    await vi.advanceTimersByTimeAsync(LIVE_POLL_MS * 4);
    expect(late).not.toHaveBeenCalled();
    expect(invoke).toHaveBeenCalledTimes(1);
  });

  it("keeps_polling_after_an_error_silently", async () => {
    tauri.on = false;
    invoke.mockRejectedValueOnce({ code: "error", message: "Brücke nicht erreichbar", asset: null });
    invoke.mockResolvedValue(status("waiting_for_save"));
    const onStatus = vi.fn();
    const stop = subscribeLive(onStatus);
    await settle();
    expect(onStatus).not.toHaveBeenCalled();
    await vi.advanceTimersByTimeAsync(LIVE_POLL_MS);
    expect(onStatus).toHaveBeenLastCalledWith(status("waiting_for_save"));
    stop();
  });
});
