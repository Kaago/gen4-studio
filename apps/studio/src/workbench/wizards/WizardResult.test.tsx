// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { NextStep } from "../../api";
import { nextStepsFor, startPaintingWith } from "../recipes";
import { DoneResult } from "./WizardResult";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("DoneResult", () => {
  let container: HTMLElement;
  let root: Root;
  const onStep = vi.fn();
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    onStep.mockReset();
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  const STEPS: NextStep[] = [
    { id: "create_area", label: "Gebiet anlegen", available: true, reason: null },
    { id: "play_here", label: "Hier spielen", available: false, reason: "Kommt mit 3d.9" },
  ];
  const keys = () => [...container.querySelectorAll<HTMLButtonElement>("button[data-recipe]")];

  it("steps_render_as_recipe_buttons_locked_with_reason_and_click_calls_on_step", () => {
    act(() => root.render(<DoneResult title="Welt geleert" lines={["Randkarte 0666"]} next={["Mit „Hier spielen“ ausprobieren."]} steps={STEPS} onStep={onStep} />));
    expect(keys().map((b) => [b.getAttribute("data-recipe"), b.textContent, b.disabled])).toEqual([
      ["create_area", "Gebiet anlegen", false],
      ["play_here", "Hier spielen", true],
    ]);
    expect(document.getElementById(keys()[1]!.getAttribute("aria-describedby")!)?.textContent).toBe("Kommt mit 3d.9");
    // Under „Als Nächstes“ the buttons come first, the lines of text after them.
    const next = container.querySelector("[data-next-steps]")!;
    expect(next.previousElementSibling?.textContent).toBe("Als Nächstes");
    expect(container.textContent).toContain("Mit „Hier spielen“ ausprobieren.");
    expect(next.compareDocumentPosition(container.querySelector("ul")!) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    act(() => keys()[0]!.click());
    expect(onStep).toHaveBeenCalledExactlyOnceWith("create_area");
    act(() => keys()[1]!.click());
    expect(onStep).toHaveBeenCalledOnce();
  });

  it("without_steps_shows_the_lines_only", () => {
    act(() => root.render(<DoneResult title="Gebiet angelegt" lines={["A"]} next={["B"]} />));
    expect(keys()).toEqual([]);
    expect(container.querySelector("[data-next-steps]")).toBeNull();
    expect(container.textContent).toContain("Als Nächstes");
  });

  it("adjust_terrain_starts_painting", () => {
    // The step after „Neue Karte malen“ (and „Gebiet erweitern“) is free: it selects the map's cell, turns the top view on and paints
    // with the brush. The app wires the same function (`App` → `startPainting`, `WizardHost.step` for a result with `Created.cell`).
    const app = { goTo: vi.fn(), setTopDown: vi.fn(), onTool: vi.fn(), setPaint: vi.fn() };
    const cell = { matrix: 0, x: 4, y: 27 };
    act(() =>
      root.render(
        <DoneResult title="Karte gemalt" lines={[]} next={[]} steps={nextStepsFor("new_painted_map")} onStep={(id) => id === "adjust_terrain" && startPaintingWith(app, cell)} />,
      ),
    );
    const key = keys()[0]!;
    expect([key.getAttribute("data-recipe"), key.textContent, key.disabled]).toEqual(["adjust_terrain", "Gelände anpassen", false]);
    act(() => key.click());
    expect(app.goTo).toHaveBeenCalledExactlyOnceWith({ kind: "matrixCell", matrix: 0, x: 4, y: 27, show: "map" });
    expect(app.setTopDown).toHaveBeenCalledExactlyOnceWith(true);
    expect(app.onTool).toHaveBeenCalledExactlyOnceWith("paint");
    expect(app.setPaint).toHaveBeenCalledExactlyOnceWith({ sub: "brush" });
  });
});
