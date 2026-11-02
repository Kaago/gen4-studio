import { describe, expect, it, vi } from "vitest";
import { OFF_STATUS } from "../api.live";
import type { LiveStatusDto } from "../api.live";
import { createLiveStore } from "./liveStore";

const live = (revision: number, taken_ms = 12): LiveStatusDto => ({
  ...OFF_STATUS,
  state: "live",
  port: 3333,
  snapshot: { revision, flags: [0, 8, 0], vars: [0, 42], taken_ms },
  breakpoints: [{ id: 1, event: { event_file: 12, kind: "object", index: 3 }, target: { kind: "flag", number: 0x173 }, label: "Figur 3" }],
});

describe("liveStore", () => {
  it("skips_a_status_structurally_equal_to_the_current_one", () => {
    const store = createLiveStore();
    const heard = vi.fn();
    store.subscribe(heard);
    const first = live(7);
    store.set(first);
    expect(heard).toHaveBeenCalledTimes(1);
    // A browser poll answers a fresh object every tick: the same status keeps the object and tells nobody.
    store.set(live(7));
    store.set(structuredClone(first));
    expect(heard).toHaveBeenCalledTimes(1);
    expect(store.get()).toBe(first);
    // Any difference in what is shown, deep inside too, is a new status.
    const deeper = live(7);
    deeper.snapshot!.vars[1] = 43;
    store.set(deeper);
    expect(heard).toHaveBeenCalledTimes(2);
    store.set({ ...deeper, breakpoints: [] });
    expect(heard).toHaveBeenCalledTimes(3);
    store.set({ ...OFF_STATUS });
    expect(heard).toHaveBeenCalledTimes(4);
    expect(store.get().state).toBe("off");
  });

  it("skips_a_refresh_that_only_moved_the_snapshots_revision_and_timing", () => {
    // Like the backend's `same_view`: every refresh while live or halted brings a new revision and time, the values stay.
    const store = createLiveStore();
    const heard = vi.fn();
    store.subscribe(heard);
    const first = live(7, 12);
    store.set(first);
    store.set(live(8, 13));
    store.set(live(9, 4));
    store.set({ ...live(10, 9), state: "halted" });
    expect(heard).toHaveBeenCalledTimes(2);
    expect(store.get().state).toBe("halted");
    // A changed value with the new revision is a new status.
    const changed = live(11, 5);
    changed.snapshot!.flags[1] = 0;
    store.set(changed);
    expect(heard).toHaveBeenCalledTimes(3);
    expect(store.get()).toBe(changed);
    // A snapshot appearing or going away is a change too.
    store.set({ ...changed, snapshot: null });
    expect(heard).toHaveBeenCalledTimes(4);
  });
});
