// @vitest-environment jsdom
import { afterEach, describe, expect, it } from "vitest";
import { activeScene } from "./activeScene";

/** A host whose canvas is in the page with the given size. */
function host(width: number, height: number, attached = true) {
  const canvas = document.createElement("canvas");
  Object.defineProperty(canvas, "clientWidth", { value: width });
  Object.defineProperty(canvas, "clientHeight", { value: height });
  if (attached) document.body.append(canvas);
  return { canvas, capture: () => "data:image/png;base64," };
}

afterEach(() => {
  const current = activeScene.get();
  if (current) activeScene.clear(current);
  document.body.replaceChildren();
});

describe("activeScene", () => {
  it("knows_the_mounted_host_and_forgets_it_when_it_goes", () => {
    expect(activeScene.get()).toBeNull();
    const first = host(800, 600);
    activeScene.set(first);
    expect(activeScene.get()).toBe(first);
    // A host freed after a newer one was mounted leaves the newer one.
    const second = host(800, 600);
    activeScene.set(second);
    activeScene.clear(first);
    expect(activeScene.get()).toBe(second);
    activeScene.clear(second);
    expect(activeScene.get()).toBeNull();
  });

  it("shows_only_with_a_canvas_in_the_page_that_has_a_size", () => {
    expect(activeScene.shown()).toBe(false);
    activeScene.set(host(800, 600));
    expect(activeScene.shown()).toBe(true);
    // Behind another tab or in a hidden area the renderer draws 1 × 1, or the canvas has no box at all.
    activeScene.set(host(1, 1));
    expect(activeScene.shown()).toBe(false);
    activeScene.set(host(0, 0));
    expect(activeScene.shown()).toBe(false);
    activeScene.set(host(800, 600, false));
    expect(activeScene.shown()).toBe(false);
  });
});
