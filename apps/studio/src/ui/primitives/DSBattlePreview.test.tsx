// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { DSBattlePreview } from "./DSBattlePreview";
import type { BattleSide } from "./DSBattlePreview";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("DSBattlePreview", () => {
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

  const group = () => container.querySelector<HTMLElement>("[role=group]");
  const boxes = () => [...container.querySelectorAll<HTMLElement>("[data-ds-status]")];
  const meters = () => [...container.querySelectorAll<HTMLElement>("[role=meter]")];
  const staravia: BattleSide = { name: "Staravia", level: 12, spriteUrl: null };
  const panflam: BattleSide = { name: "Panflam", level: 7, spriteUrl: "blob:battle-390", hp: 0.3 };

  it("shows_two_status_boxes_with_name_and_level", () => {
    act(() => root.render(<DSBattlePreview foe={staravia} own={panflam} scale={1} />));
    expect(container.querySelector("[data-ds-battle]")).not.toBeNull();
    const [foe, own] = boxes();
    expect(boxes()).toHaveLength(2);
    expect(foe!.getAttribute("data-ds-status")).toBe("foe");
    expect(foe!.textContent).toContain("Staravia");
    expect(foe!.textContent).toContain("Lv. 12");
    expect(own!.getAttribute("data-ds-status")).toBe("own");
    expect(own!.textContent).toContain("Panflam");
    expect(own!.textContent).toContain("Lv. 7");
  });

  it("colours_the_hp_bar_by_the_thresholds", () => {
    act(() => root.render(<DSBattlePreview foe={staravia} own={panflam} scale={1} />));
    const [full, low] = meters();
    expect(full!.getAttribute("data-hp")).toBe("gruen");
    expect(full!.getAttribute("aria-valuenow")).toBe("100");
    expect(low!.getAttribute("data-hp")).toBe("gelb");
    expect(low!.getAttribute("aria-valuenow")).toBe("30");
    expect(low!.getAttribute("aria-valuemin")).toBe("0");
    expect(low!.getAttribute("aria-valuemax")).toBe("100");
    const at = (hp: number) => {
      act(() => root.render(<DSBattlePreview foe={{ ...staravia, hp }} own={null} scale={1} />));
      return [meters()[0]!.getAttribute("data-hp"), meters()[0]!.getAttribute("aria-valuenow")];
    };
    expect(at(0.51)).toEqual(["gruen", "51"]);
    expect(at(0.5)).toEqual(["gelb", "50"]);
    expect(at(0.21)).toEqual(["gelb", "21"]);
    expect(at(0.2)).toEqual(["rot", "20"]);
    expect(at(0)).toEqual(["rot", "0"]);
    // Out of range is clamped.
    expect(at(1.7)).toEqual(["gruen", "100"]);
    expect(at(-1)).toEqual(["rot", "0"]);
  });

  it("without_an_own_side_shows_only_the_foe_box", () => {
    act(() => root.render(<DSBattlePreview foe={staravia} own={null} scale={1} />));
    expect(boxes()).toHaveLength(1);
    expect(boxes()[0]!.getAttribute("data-ds-status")).toBe("foe");
    expect(container.querySelectorAll("[data-sprite]")).toHaveLength(1);
    // Both grounds stay.
    expect(container.querySelectorAll("[data-ds-ground]")).toHaveLength(2);
  });

  it("a_side_without_a_level_shows_only_the_name", () => {
    // A trainer before the battle: no level, no HP bar.
    act(() => root.render(<DSBattlePreview foe={{ name: "Ass-Trainer Lutz", level: 0, spriteUrl: null }} own={null} scale={1} />));
    expect(boxes()[0]!.textContent).toBe("Ass-Trainer Lutz");
    expect(meters()).toHaveLength(0);
    expect(group()?.getAttribute("aria-label")).toBe("Kampfvorschau: Ass-Trainer Lutz");
  });

  it("names_both_sides_in_the_label", () => {
    act(() => root.render(<DSBattlePreview foe={staravia} own={panflam} scale={1} />));
    expect(group()?.getAttribute("aria-label")).toBe("Kampfvorschau: Staravia Lv. 12 gegen Panflam Lv. 7");
    act(() => root.render(<DSBattlePreview foe={staravia} own={null} scale={1} />));
    expect(group()?.getAttribute("aria-label")).toBe("Kampfvorschau: Staravia Lv. 12");
  });

  it("shows_the_pictures_and_placeholders", () => {
    act(() => root.render(<DSBattlePreview foe={staravia} own={panflam} scale={1} />));
    expect(container.querySelector("[data-sprite=placeholder]")?.textContent).toBe("S");
    const img = container.querySelector("img")!;
    expect(img.getAttribute("src")).toBe("blob:battle-390");
    expect(img.getAttribute("width")).toBe("80");
  });

  it("draws_at_the_given_scale_and_hides_when_decorative", () => {
    act(() => root.render(<DSBattlePreview foe={staravia} own={panflam} scale={3} />));
    expect(group()?.style.getPropertyValue("--ds-scale")).toBe("3");
    act(() => root.render(<DSBattlePreview foe={staravia} own={panflam} scale={1} decorative />));
    expect(group()).toBeNull();
    expect(container.firstElementChild?.getAttribute("aria-hidden")).toBe("true");
    expect(container.querySelector("[data-ds-battle]")).not.toBeNull();
  });
});
