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
