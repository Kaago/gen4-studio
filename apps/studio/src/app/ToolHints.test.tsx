// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { PaintContext } from "../workbench/paint/PaintContext";
import type { PaintModel, PaintState } from "../workbench/paint/PaintContext";
import { ToolHints } from "./ToolHints";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("ToolHints", () => {
  let container: HTMLElement;
  let root: Root;
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  it("names_the_active_tool_and_pairs_each_key_with_what_it_does", () => {
    act(() => root.render(<ToolHints tool="select" />));
    expect(container.textContent).toBe("Auswählen·Klick wählt·Ziehen verschiebt");
    expect([...container.querySelectorAll("kbd")].map((k) => k.textContent)).toEqual(["Klick", "Ziehen"]);
  });

  it("says_which_trainer_a_click_with_the_figure_tool_sets", () => {
    act(() => root.render(<ToolHints tool="figure" placing="Wanderer Jens" />));
    expect(container.textContent).toBe("Figur·Klick setzt Wanderer Jens·Esc hebt die Bindung auf");
    // Only the figure tool places a trainer.
    act(() => root.render(<ToolHints tool="door" placing="Wanderer Jens" />));
    expect(container.textContent).toContain("Klick legt eine Tür an");
  });

  it("follows_the_tool", () => {
    act(() => root.render(<ToolHints tool="paint" />));
    expect(container.textContent).toContain("Gelände malen");
    expect(container.textContent).toContain("Klick setzt Kollision");
    expect(container.textContent).toContain("⇧Klick löscht");
    act(() => root.render(<ToolHints tool="door" />));
    expect(container.textContent).toContain("Klick legt eine Tür an");
    expect(container.textContent).toContain("Esc bricht ab");
  });

  /** The hints while painting `target` (null: a binary map) with `state`. */
  const painting = (target: number | null, state: Partial<PaintState> = {}) => {
    const model: PaintModel = { state: { sub: "brush", layer: 0, tile: "0041", ramp: { dir: "e", steps: 1 }, lower: false, ...state }, set: () => {}, target };
    act(() =>
      root.render(
        <PaintContext.Provider value={model}>
          <ToolHints tool="paint" />
        </PaintContext.Provider>,
      ),
    );
  };

  it("ramp_hint_names_direction_and_steps", () => {
    painting(120, { sub: "ramp" });
    expect(container.textContent).toBe("Gelände malen·Rampe·Klick setzt Rampe nach Osten, 1 Stufe·Z dreht·1/2 Stufen·⇧Klick entfernt");
    painting(120, { sub: "ramp", ramp: { dir: "w", steps: 2 } });
    expect(container.textContent).toContain("Klick setzt Rampe nach Westen, 2 Stufen");
    painting(120, { sub: "brush" });
    expect(container.textContent).toBe("Gelände malen·Pinsel·Ziehen malt·Rechtsklick nimmt auf·Z dreht·X spiegelt");
    // A binary map keeps the collision hints.
    painting(null, { sub: "ramp" });
    expect(container.textContent).toBe("Gelände malen·Klick setzt Kollision·⇧Klick löscht");
  });
});
