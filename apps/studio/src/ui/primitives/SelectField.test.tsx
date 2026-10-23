// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { SelectField } from "./SelectField";
import type { SelectOption } from "./SelectField";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const options: SelectOption<number | string>[] = [
  { value: 0, label: "Nichts" },
  { value: 1, label: "Gras", icon: "pin" },
  { value: "x", label: "Sonder" },
];

describe("SelectField", () => {
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

  const trigger = () => container.querySelector<HTMLElement>("[role=combobox]")!;
  const open = () => {
    act(() => {
      trigger().dispatchEvent(new MouseEvent("mousedown", { bubbles: true, cancelable: true, button: 0 }));
      trigger().dispatchEvent(new MouseEvent("mouseup", { bubbles: true, cancelable: true, button: 0 }));
      trigger().click();
    });
  };
  const optionEls = () => [...document.querySelectorAll<HTMLElement>("[role=option]")];

  it("shows_the_label_of_the_selected_option_and_is_named_by_its_label", () => {
    act(() => root.render(<SelectField label="Gelände" value={1} options={options} onChange={() => {}} />));
    expect(trigger().textContent).toContain("Gras");
    expect(trigger().getAttribute("aria-label") ?? container.textContent).toContain("Gelände");
  });

  it("shows_an_unknown_value_as_an_option_named_Wert_7_and_selects_it", () => {
    act(() => root.render(<SelectField label="Gelände" value={7} options={options} onChange={() => {}} />));
    expect(trigger().textContent).toContain("Wert 7");
    open();
    const names = optionEls().map((o) => o.textContent);
    expect(names).toContain("Wert 7");
    const unknown = optionEls().find((o) => o.textContent?.includes("Wert 7"))!;
    expect(unknown.getAttribute("aria-selected")).toBe("true");
    expect(optionEls().filter((o) => o.getAttribute("aria-selected") === "true")).toHaveLength(1);
  });

  it("does_not_invent_an_option_for_a_known_value", () => {
    act(() => root.render(<SelectField label="Gelände" value={0} options={options} onChange={() => {}} />));
    open();
    expect(optionEls().map((o) => o.textContent)).toEqual(["Nichts", "Gras", "Sonder"]);
  });

  it("calls_onChange_with_the_value_of_the_picked_option_keeping_its_type", () => {
    const onChange = vi.fn();
    act(() => root.render(<SelectField label="Gelände" value={0} options={options} onChange={onChange} />));
    open();
    act(() => optionEls().find((o) => o.textContent === "Gras")!.click());
    expect(onChange.mock.calls).toEqual([[1]]);
  });

  it("can_be_disabled", () => {
    act(() => root.render(<SelectField label="Gelände" value={0} options={options} onChange={() => {}} disabled />));
    expect(trigger().hasAttribute("data-disabled") || trigger().getAttribute("aria-disabled") === "true").toBe(true);
  });
});
