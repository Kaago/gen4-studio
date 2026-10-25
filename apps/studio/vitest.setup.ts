import { afterEach } from "vitest";
import { resetSectionFolds } from "./src/workbench/inspector/foldState";

// The inspector remembers folded sections across editors (outside React state); a test starts with the defaults.
afterEach(() => resetSectionFolds());

// jsdom lacks these browser APIs; stubs are installed only when the environment has no own version.

if (typeof globalThis.ResizeObserver === "undefined") {
  globalThis.ResizeObserver = class {
    observe(): void {}
    unobserve(): void {}
    disconnect(): void {}
  } as unknown as typeof ResizeObserver;
}

if (typeof window !== "undefined" && typeof window.matchMedia !== "function") {
  window.matchMedia = (query: string): MediaQueryList =>
    ({
      matches: false,
      media: query,
      onchange: null,
      addEventListener: () => {},
      removeEventListener: () => {},
      addListener: () => {},
      removeListener: () => {},
      dispatchEvent: () => false,
    }) as MediaQueryList;
}

// CodeMirror measures text through ranges in an animation frame, which jsdom runs when a test is slow; jsdom's ranges have no geometry.
if (typeof Range !== "undefined" && typeof Range.prototype.getClientRects !== "function") {
  const empty = { x: 0, y: 0, width: 0, height: 0, top: 0, right: 0, bottom: 0, left: 0, toJSON: () => ({}) } as DOMRect;
  Range.prototype.getClientRects = () => Object.assign([], { item: () => null }) as unknown as DOMRectList;
  Range.prototype.getBoundingClientRect = () => empty;
}
