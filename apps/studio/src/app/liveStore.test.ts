import { describe, expect, it, vi } from "vitest";
import { OFF_STATUS } from "../api.live";
import type { LiveStatusDto } from "../api.live";
import { createLiveStore } from "./liveStore";

const live = (revision: number): LiveStatusDto => ({
  ...OFF_STATUS,
  state: "live",
  port: 3333,
  snapshot: { revision, flags: [0, 8, 0], vars: [0, 42], taken_ms: 12 },
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
    // Any difference, deep inside too, is a new status.
    store.set(live(8));
    expect(heard).toHaveBeenCalledTimes(2);
    const deeper = live(8);
    deeper.snapshot!.vars[1] = 43;
    store.set(deeper);
    expect(heard).toHaveBeenCalledTimes(3);
    store.set({ ...deeper, breakpoints: [] });
    expect(heard).toHaveBeenCalledTimes(4);
    store.set({ ...OFF_STATUS });
    expect(heard).toHaveBeenCalledTimes(5);
    expect(store.get().state).toBe("off");
  });
});
