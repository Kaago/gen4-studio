// @vitest-environment jsdom
import { act, createElement } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { shallowEqual, useShallowStable, useStableCallback } from "./useStable";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("shallowEqual", () => {
  it("compares the fields by identity", () => {
    const f = () => {};
    expect(shallowEqual({ a: 1, f }, { a: 1, f })).toBe(true);
    expect(shallowEqual({ a: 1 }, { a: 2 })).toBe(false);
    expect(shallowEqual({ a: {} }, { a: {} })).toBe(false);
    expect(shallowEqual({ a: 1 }, { a: 1, b: 2 })).toBe(false);
    expect(shallowEqual({ a: undefined }, { b: undefined })).toBe(false);
    expect(shallowEqual({ a: NaN }, { a: NaN })).toBe(true);
  });
});

describe("stable hooks", () => {
  let container: HTMLElement;
  let root: ReturnType<typeof createRoot>;
  beforeEach(() => {
    container = document.createElement("div");
    root = createRoot(container);
  });
  afterEach(() => act(() => root.unmount()));

  it("useShallowStable keeps the identity until a field changes", () => {
    const seen: object[] = [];
    const Probe = ({ a, b }: { a: number; b: number }) => {
      seen.push(useShallowStable({ a, b }));
      return null;
    };
    act(() => root.render(createElement(Probe, { a: 1, b: 1 })));
    act(() => root.render(createElement(Probe, { a: 1, b: 1 })));
    expect(seen[1]).toBe(seen[0]);
    act(() => root.render(createElement(Probe, { a: 1, b: 2 })));
    expect(seen[2]).not.toBe(seen[1]);
    expect(seen[2]).toEqual({ a: 1, b: 2 });
  });

  it("useStableCallback keeps its identity and calls the latest function", () => {
    const seen: Array<() => number> = [];
    const Probe = ({ n }: { n: number }) => {
      seen.push(useStableCallback(() => n));
      return null;
    };
    act(() => root.render(createElement(Probe, { n: 1 })));
    act(() => root.render(createElement(Probe, { n: 2 })));
    expect(seen[1]).toBe(seen[0]);
    expect(seen[0]()).toBe(2);
  });
});
