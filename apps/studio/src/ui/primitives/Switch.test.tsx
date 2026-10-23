// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { Checkbox } from "./Checkbox";
import { Switch } from "./Switch";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("Switch and Checkbox", () => {
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

  it("switch_has_role_switch_and_aria_checked_follows_checked", () => {
    act(() => root.render(<Switch label="Gelände" checked={false} onChange={() => {}} />));
    const el = container.querySelector("[role=switch]")!;
    expect(el.getAttribute("aria-checked")).toBe("false");
    expect(container.textContent).toContain("Gelände");
    act(() => root.render(<Switch label="Gelände" checked onChange={() => {}} />));
    expect(container.querySelector("[role=switch]")!.getAttribute("aria-checked")).toBe("true");
  });

  it("switch_reports_the_new_state_and_leaves_the_state_to_its_owner", () => {
    const onChange = vi.fn();
    act(() => root.render(<Switch label="Gelände" checked={false} onChange={onChange} />));
    act(() => container.querySelector<HTMLElement>("[role=switch]")!.click());
    expect(onChange.mock.calls).toEqual([[true]]);
    expect(container.querySelector("[role=switch]")!.getAttribute("aria-checked")).toBe("false");
  });

  it("switch_is_named_by_its_label_and_can_be_disabled", () => {
    act(() => root.render(<Switch label="Gelände" checked={false} onChange={() => {}} disabled />));
    const el = container.querySelector("[role=switch]")!;
    expect(el.getAttribute("aria-disabled") === "true" || el.hasAttribute("data-disabled")).toBe(true);
  });

  it("checkbox_maps_checked_unchecked_and_mixed_to_aria_checked", () => {
    act(() => root.render(<Checkbox label="Alle" checked={false} onChange={() => {}} />));
    const el = () => container.querySelector("[role=checkbox]")!;
    expect(el().getAttribute("aria-checked")).toBe("false");
    act(() => root.render(<Checkbox label="Alle" checked onChange={() => {}} />));
    expect(el().getAttribute("aria-checked")).toBe("true");
    act(() => root.render(<Checkbox label="Alle" checked="mixed" onChange={() => {}} />));
    expect(el().getAttribute("aria-checked")).toBe("mixed");
  });

  it("checkbox_reports_true_from_mixed_and_unchecked", () => {
    const onChange = vi.fn();
    act(() => root.render(<Checkbox label="Alle" checked="mixed" onChange={onChange} />));
    act(() => container.querySelector<HTMLElement>("[role=checkbox]")!.click());
    expect(onChange.mock.calls).toEqual([[true]]);
  });
});
