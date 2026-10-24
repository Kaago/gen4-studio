import { describe, expect, it, vi } from "vitest";
import { createScriptDirtyStore, scriptDirtyStore } from "./scriptDirtyStore";

describe("scriptDirtyStore", () => {
  it("starts_at_zero_and_keeps_what_was_set", () => {
    const store = createScriptDirtyStore();
    expect(store.get()).toBe(0);
    store.set(2);
    expect(store.get()).toBe(2);
  });

  it("tells_subscribers_about_a_change_and_not_about_the_same_count", () => {
    const store = createScriptDirtyStore();
    const seen = vi.fn();
    const stop = store.subscribe(seen);
    store.set(1);
    store.set(1);
    expect(seen).toHaveBeenCalledTimes(1);
    stop();
    store.set(0);
    expect(seen).toHaveBeenCalledTimes(1);
  });

  it("works_with_its_methods_taken_off_the_object", () => {
    // `useSyncExternalStore(store.subscribe, store.get)` calls them without the object.
    const { subscribe, get, set } = createScriptDirtyStore();
    const seen = vi.fn();
    subscribe(seen);
    set(3);
    expect(get()).toBe(3);
    expect(seen).toHaveBeenCalledTimes(1);
  });

  it("has_one_app_wide_instance", () => {
    expect(scriptDirtyStore.get()).toBe(0);
  });
});
