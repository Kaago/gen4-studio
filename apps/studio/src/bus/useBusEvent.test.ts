// @vitest-environment jsdom
import { act, createElement } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { pendingSince } from "../reload";
import { createEditBus } from "./editBus";
import type { EditBus } from "./editBus";
import { useBusEvent, useBusQueue, useBusRevision } from "./useBusEvent";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

let root: Root;
let host: HTMLElement;
beforeEach(() => {
  host = document.createElement("div");
  root = createRoot(host);
});
afterEach(() => act(() => root.unmount()));

/** Counts the subscriptions a bus gets and ends. */
function spied(): { bus: EditBus; subscribed: () => number; live: () => number } {
  const bus = createEditBus();
  let subscribed = 0;
  let live = 0;
  const subscribe = bus.subscribe.bind(bus);
  bus.subscribe = (filter, fn) => {
    subscribed++;
    live++;
    const end = subscribe(filter, fn);
    return () => {
      live--;
      end();
    };
  };
  return { bus, subscribed: () => subscribed, live: () => live };
}

describe("useBusEvent", () => {
  it("calls the latest callback without resubscribing on re-render", () => {
    const { bus, subscribed } = spied();
    const calls: string[] = [];
    const Probe = ({ tag }: { tag: string }) => {
      useBusEvent(bus, "headerTable", () => calls.push(tag));
      return null;
    };
    act(() => root.render(createElement(Probe, { tag: "a" })));
    act(() => root.render(createElement(Probe, { tag: "b" })));
    act(() => bus.emit({ topic: "headerTable" }));
    expect(calls).toEqual(["b"]);
    expect(subscribed()).toBe(1);
  });

  it("delivers nothing after unmount", () => {
    const { bus, live } = spied();
    const fn = vi.fn();
    const Probe = () => {
      useBusEvent(bus, "headerTable", fn);
      return null;
    };
    act(() => root.render(createElement(Probe)));
    expect(live()).toBe(1);
    act(() => root.render(null));
    expect(live()).toBe(0);
    bus.emit({ topic: "headerTable" });
    expect(fn).not.toHaveBeenCalled();
  });
});

describe("useBusRevision", () => {
  it("changes with every matching event and ignores others", () => {
    const bus = createEditBus();
    const seen: number[] = [];
    const Probe = () => {
      seen.push(useBusRevision(bus, "matrixData"));
      return null;
    };
    act(() => root.render(createElement(Probe)));
    act(() => bus.emit({ topic: "areaData" }));
    act(() => bus.emit({ topic: "matrixData" }));
    act(() => bus.emit({ topic: "matrixData" }));
    expect(seen[seen.length - 1]).toBe(2);
    expect(new Set(seen)).toEqual(new Set([0, 1, 2]));
  });

  it("counts two events of one batch both", () => {
    const bus = createEditBus();
    let revision = -1;
    const Probe = () => {
      revision = useBusRevision(bus, "eventData");
      return null;
    };
    act(() => root.render(createElement(Probe)));
    act(() => {
      bus.emit({ topic: "eventData" });
      bus.emit({ topic: "eventData" });
    });
    expect(revision).toBe(2);
  });
});

describe("useBusQueue", () => {
  it("keeps events for different maps emitted before a render, in order", () => {
    const bus = createEditBus();
    let queue: ReturnType<typeof useBusQueue<"mapRefetch", number>> = null;
    const Probe = () => {
      queue = useBusQueue(bus, "mapRefetch", (e) => e.map);
      return null;
    };
    act(() => root.render(createElement(Probe)));
    expect(queue).toBeNull();
    act(() => {
      bus.emit({ topic: "mapRefetch", map: 4 });
      bus.emit({ topic: "mapRefetch", map: 6 });
      bus.emit({ topic: "headerTable" });
    });
    expect(pendingSince(queue, 0).items).toEqual([4, 6]);
    act(() => bus.emit({ topic: "mapRefetch", map: 4 }));
    const first = pendingSince(queue, 0);
    expect(first.items).toEqual([4, 6, 4]);
    expect(pendingSince(queue, first.last).items).toEqual([]);
  });
});
