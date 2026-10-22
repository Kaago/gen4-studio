import { describe, expect, it, vi } from "vitest";
import { createEditBus } from "./editBus";
import type { EditEvent } from "./editBus";

function collect(bus: ReturnType<typeof createEditBus>, filter: Parameters<typeof bus.subscribe>[0]) {
  const seen: EditEvent[] = [];
  const unsubscribe = bus.subscribe(filter, (e) => seen.push(e));
  return { seen, unsubscribe };
}

describe("edit bus", () => {
  it("delivers two events for different maps that were emitted before any render, in order", () => {
    const bus = createEditBus();
    const { seen } = collect(bus, "mapRefetch");
    bus.emit({ topic: "mapRefetch", map: 7 });
    bus.emit({ topic: "mapRefetch", map: 9 });
    expect(seen).toEqual([
      { topic: "mapRefetch", map: 7 },
      { topic: "mapRefetch", map: 9 },
    ]);
  });

  it("delivers a repeated event for the same asset every time (nothing is collapsed)", () => {
    const bus = createEditBus();
    const { seen } = collect(bus, "mapRefetch");
    bus.emit({ topic: "mapRefetch", map: 7 });
    bus.emit({ topic: "mapRefetch", map: 7 });
    expect(seen).toHaveLength(2);
  });

  it("filters by one topic, a list of topics or a predicate", () => {
    const bus = createEditBus();
    const one = collect(bus, "headerTable");
    const list = collect(bus, ["matrixData", "areaData"]);
    const predicate = collect(bus, (e) => e.topic === "mapRefetch" && e.map === 2);
    bus.emit({ topic: "headerTable" });
    bus.emit({ topic: "matrixData" });
    bus.emit({ topic: "areaData" });
    bus.emit({ topic: "mapRefetch", map: 1 });
    bus.emit({ topic: "mapRefetch", map: 2 });
    expect(one.seen).toEqual([{ topic: "headerTable" }]);
    expect(list.seen).toEqual([{ topic: "matrixData" }, { topic: "areaData" }]);
    expect(predicate.seen).toEqual([{ topic: "mapRefetch", map: 2 }]);
  });

  it("stops delivering after unsubscribe, and unsubscribing twice is harmless", () => {
    const bus = createEditBus();
    const { seen, unsubscribe } = collect(bus, "headerTable");
    bus.emit({ topic: "headerTable" });
    unsubscribe();
    unsubscribe();
    bus.emit({ topic: "headerTable" });
    expect(seen).toHaveLength(1);
  });

  it("an unsubscribe made while an event is delivered holds for the next one", () => {
    const bus = createEditBus();
    const later = collect(bus, "headerTable");
    bus.subscribe("headerTable", () => later.unsubscribe());
    // `later` subscribed first, so it still sees this one; the unsubscribe holds for the next.
    bus.emit({ topic: "headerTable" });
    bus.emit({ topic: "headerTable" });
    expect(later.seen).toHaveLength(1);
  });

  it("stops the current delivery for a subscriber removed by an earlier subscriber", () => {
    const bus = createEditBus();
    const calls: string[] = [];
    let unsubscribeSecond = () => {};
    bus.subscribe("headerTable", () => {
      calls.push("first");
      unsubscribeSecond();
    });
    unsubscribeSecond = bus.subscribe("headerTable", () => calls.push("second"));
    bus.emit({ topic: "headerTable" });
    expect(calls).toEqual(["first"]);
  });

  it("does not deliver an event to a subscriber added while it is delivered", () => {
    const bus = createEditBus();
    const seen: EditEvent[] = [];
    bus.subscribe("headerTable", () => {
      bus.subscribe("headerTable", (e) => seen.push(e));
    });
    bus.emit({ topic: "headerTable" });
    expect(seen).toEqual([]);
    bus.emit({ topic: "headerTable" });
    expect(seen).toHaveLength(1);
  });

  it("keeps delivering to the others when a subscriber throws, and reports the error", () => {
    const onError = vi.fn();
    const bus = createEditBus(onError);
    const boom = new Error("boom");
    bus.subscribe("headerTable", () => {
      throw boom;
    });
    const { seen } = collect(bus, "headerTable");
    bus.emit({ topic: "headerTable" });
    bus.emit({ topic: "headerTable" });
    expect(seen).toHaveLength(2);
    expect(onError).toHaveBeenCalledTimes(2);
    expect(onError).toHaveBeenCalledWith(boom);
  });

  it("does not strand queued events when the error handler itself throws", () => {
    const bus = createEditBus(() => {
      throw new Error("handler broke");
    });
    bus.subscribe("headerTable", () => {
      // Queued behind the event being delivered, then a subscriber throws.
      bus.emit({ topic: "matrixData" });
      throw new Error("boom");
    });
    const { seen } = collect(bus, ["matrixData"]);
    expect(() => bus.emit({ topic: "headerTable" })).not.toThrow();
    expect(seen).toHaveLength(1);
    // Nothing waits for the next emit: a later event arrives once, not together with a leftover.
    bus.emit({ topic: "matrixData" });
    expect(seen).toHaveLength(2);
  });

  it("keeps the order for every subscriber when a subscriber emits while it is delivered", () => {
    const bus = createEditBus();
    const a: string[] = [];
    const b: string[] = [];
    const label = (e: EditEvent) => (e.topic === "mapRefetch" ? `map${e.map}` : e.topic);
    bus.subscribe("headerTable", (e) => {
      a.push(label(e));
      bus.emit({ topic: "mapRefetch", map: 1 });
    });
    bus.subscribe(["headerTable", "mapRefetch"], (e) => b.push(label(e)));
    bus.subscribe("mapRefetch", (e) => a.push(label(e)));
    bus.emit({ topic: "headerTable" });
    bus.emit({ topic: "mapRefetch", map: 2 });
    // The nested emit is delivered after the event being delivered, never in the middle of it.
    expect(b).toEqual(["headerTable", "map1", "map2"]);
    expect(a).toEqual(["headerTable", "map1", "map2"]);
  });

  it("delivers nothing and does not fail without subscribers", () => {
    const bus = createEditBus();
    expect(() => bus.emit({ topic: "headerTable" })).not.toThrow();
  });
});
