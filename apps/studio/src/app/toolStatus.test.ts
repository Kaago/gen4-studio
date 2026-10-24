import { describe, expect, it, vi } from "vitest";
import { createToolStatus, readyCount, toolStatus, toolsText } from "./toolStatus";

describe("toolStatus", () => {
  it("starts_without_a_language_server_and_with_apicula_unknown", () => {
    const store = createToolStatus();
    expect(store.get()).toEqual({ lsp: null, apicula: "unbekannt" });
    // The app's one store starts the same way.
    expect(toolStatus.get().lsp).toBeNull();
  });

  it("set_merges_the_partial_and_tells_the_subscribers", () => {
    const store = createToolStatus();
    const seen = vi.fn();
    const stop = store.subscribe(seen);
    const restart = () => {};
    store.set({ lsp: { state: "fehler", note: "rotom-lsp antwortet nicht", restart } });
    expect(seen).toHaveBeenCalledTimes(1);
    expect(store.get()).toEqual({ lsp: { state: "fehler", note: "rotom-lsp antwortet nicht", restart }, apicula: "unbekannt" });
    store.set({ apicula: "fehlt" });
    expect(seen).toHaveBeenCalledTimes(2);
    expect(store.get().lsp?.state).toBe("fehler");
    expect(store.get().apicula).toBe("fehlt");
    stop();
    store.set({ apicula: "bereit" });
    expect(seen).toHaveBeenCalledTimes(2);
  });

  it("keeps_the_snapshot_and_stays_quiet_when_nothing_changed", () => {
    const store = createToolStatus();
    const seen = vi.fn();
    store.subscribe(seen);
    const restart = () => {};
    store.set({ lsp: { state: "bereit", restart } });
    const before = store.get();
    // A new object with the same values (the panel writes on every render of its state) is no change.
    store.set({ lsp: { state: "bereit", restart } });
    store.set({ apicula: "unbekannt" });
    expect(store.get()).toBe(before);
    expect(seen).toHaveBeenCalledTimes(1);
  });

  it("counts_the_tools_that_are_ready", () => {
    expect(readyCount({ lsp: null, apicula: "unbekannt" })).toBe(0);
    expect(readyCount({ lsp: { state: "bereit" }, apicula: "fehlt" })).toBe(1);
    expect(toolsText({ lsp: { state: "bereit" }, apicula: "bereit" })).toBe("2 von 2 bereit");
    expect(toolsText({ lsp: { state: "startet" }, apicula: "bereit" })).toBe("1 von 2 bereit");
  });
});
