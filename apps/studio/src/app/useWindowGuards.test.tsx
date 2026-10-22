// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { describe, expect, it, vi } from "vitest";
import { useWindowGuards } from "./useWindowGuards";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

// Deliberately no mock of @tauri-apps/api/window or /event and no __TAURI_INTERNALS__:
// outside Tauri, getCurrentWindow() and listen() must never be reached.
describe("useWindowGuards outside Tauri", () => {
  it("renders_without_throwing_in_a_plain_browser", () => {
    function Probe() {
      useWindowGuards({ current: 0 }, vi.fn());
      return <p>ok</p>;
    }
    const container = document.createElement("div");
    document.body.append(container);
    const root = createRoot(container);
    expect(() => act(() => root.render(<Probe />))).not.toThrow();
    expect(container.textContent).toBe("ok");
    act(() => root.unmount());
    container.remove();
  });
});
