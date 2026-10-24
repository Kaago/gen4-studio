// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ToggleGrid } from "./ToggleGrid";
import type { ToggleCell } from "./ToggleGrid";
import { pressKey } from "./testUtils";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

// Eight machines in rows of three; TM40 is on, TM42 has no name (no machine table) and TM44 is locked.
const cells: ToggleCell[] = [38, 39, 40, 41, 42, 43, 44, 45].map((n) => ({
  id: `tm${n}`,
  label: `TM${n}`,
  name: n === 42 ? undefined : n === 40 ? "Aero-Ass" : `Attacke ${n}`,
  checked: n === 40,
  disabled: n === 44,
}));

describe("ToggleGrid", () => {
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

  const boxes = () => [...container.querySelectorAll<HTMLElement>("[role=checkbox]")];
  const box = (name: string) => boxes().find((b) => accessibleName(b) === name)!;
  // The name a checkbox gets: its own aria-label, else the text of its label element.
  const accessibleName = (el: HTMLElement) => el.getAttribute("aria-label") ?? el.closest("label")?.textContent ?? "";

  it("is_a_named_group_of_checkboxes_named_with_number_and_move", () => {
    act(() => root.render(<ToggleGrid ariaLabel="TM und VM" cells={cells} columns={3} onToggle={() => {}} />));
    expect(container.querySelector("[role=group]")?.getAttribute("aria-label")).toBe("TM und VM");
    expect(boxes().map(accessibleName)).toEqual([
      "TM38 Attacke 38",
      "TM39 Attacke 39",
      "TM40 Aero-Ass",
      "TM41 Attacke 41",
      "TM42",
      "TM43 Attacke 43",
      "TM44 Attacke 44",
      "TM45 Attacke 45",
    ]);
    expect(box("TM40 Aero-Ass").getAttribute("aria-checked")).toBe("true");
    expect(box("TM39 Attacke 39").getAttribute("aria-checked")).toBe("false");
    expect(box("TM44 Attacke 44").hasAttribute("data-disabled") || box("TM44 Attacke 44").getAttribute("aria-disabled") === "true").toBe(true);
  });

  it("a_click_toggles_the_cell", () => {
    const onToggle = vi.fn();
    act(() => root.render(<ToggleGrid ariaLabel="TM und VM" cells={cells} columns={3} onToggle={onToggle} />));
    act(() => box("TM40 Aero-Ass").click());
    expect(onToggle).toHaveBeenLastCalledWith("tm40", false);
    act(() => box("TM38 Attacke 38").click());
    expect(onToggle).toHaveBeenLastCalledWith("tm38", true);
  });

  it("has_one_tab_stop_and_moves_it_with_the_arrow_keys_in_both_directions", () => {
    act(() => root.render(<ToggleGrid ariaLabel="TM und VM" cells={cells} columns={3} onToggle={() => {}} />));
    expect(boxes().filter((b) => b.tabIndex === 0)).toHaveLength(1);
    expect(boxes()[0]!.tabIndex).toBe(0);
    act(() => boxes()[0]!.focus());
    pressKey(boxes()[0]!, "ArrowRight");
    expect(document.activeElement).toBe(boxes()[1]);
    pressKey(boxes()[1]!, "ArrowDown");
    expect(document.activeElement).toBe(boxes()[4]);
    pressKey(boxes()[4]!, "ArrowLeft");
    expect(document.activeElement).toBe(boxes()[3]);
    pressKey(boxes()[3]!, "ArrowUp");
    expect(document.activeElement).toBe(boxes()[0]);
    // Below TM41 is the locked TM44 and nothing below that, so the focus stays.
    act(() => boxes()[3]!.focus());
    pressKey(boxes()[3]!, "ArrowDown");
    expect(document.activeElement).toBe(boxes()[3]);
    expect(boxes().filter((b) => b.tabIndex === 0)).toEqual([boxes()[3]]);
  });

  it("the_arrows_skip_a_locked_cell", () => {
    act(() => root.render(<ToggleGrid ariaLabel="TM und VM" cells={cells} columns={3} onToggle={() => {}} />));
    act(() => boxes()[5]!.focus());
    pressKey(boxes()[5]!, "ArrowRight");
    expect(document.activeElement).toBe(boxes()[7]);
  });
});
