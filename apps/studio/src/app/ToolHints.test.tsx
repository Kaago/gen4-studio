// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { PaintContext } from "../workbench/paint/PaintContext";
import type { PaintModel, PaintState } from "../workbench/paint/PaintContext";
import type { WorldGetDto } from "../api.world";
import type { Depth } from "../ui/frame/shellState";
import type { SceneHover } from "../workbench/Inspector";
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

  it("tuefteln_names_the_tokens_of_the_hovered_cell", () => {
    const rows = (fill: string, at: [number, number], token: string) =>
      Array.from({ length: 32 }, (_, z) => Array.from({ length: 32 }, (_, x) => (x === at[0] && z === at[1] ? token : fill)).join(" "));
    const world: WorldGetDto = {
      map: 120,
      world: {
        version: 1,
        tileset: 6,
        layers: [{ name: "Boden", role: "ground", tiles: rows("0000", [3, 4], "0041r1"), heights: rows("0", [3, 4], "2") }],
        permissions: { derive: true },
      },
      faces: 0,
      textures: 0,
      estimate: 0,
      share: 0,
      bake: null,
    };
    const hover = { kind: "tile", map: 120, x: 3, y: 4, absX: 3, absZ: 4, info: {} as never, units: 16 } as SceneHover;
    const show = (depth: Depth, over: Partial<PaintModel> = {}) => {
      const model: PaintModel = { state: { sub: "brush", layer: 0, tile: "0041", ramp: { dir: "e", steps: 1 }, lower: false }, set: () => {}, target: 120, world, ...over };
      act(() =>
        root.render(
          <PaintContext.Provider value={model}>
            <ToolHints tool="paint" hover={hover} depth={depth} />
          </PaintContext.Provider>,
        ),
      );
    };
    show("tuefteln");
    expect(container.querySelector("[data-paint-tokens]")?.textContent).toBe("Kachel 0041r1 · Ebene 0 · Höhe 2 (16 E)");
    // Bauen, another map or no world read yet: no tokens.
    show("bauen");
    expect(container.querySelector("[data-paint-tokens]")).toBeNull();
    show("tuefteln", { target: 121 });
    expect(container.querySelector("[data-paint-tokens]")).toBeNull();
    show("tuefteln", { world: null });
    expect(container.querySelector("[data-paint-tokens]")).toBeNull();
  });
});
