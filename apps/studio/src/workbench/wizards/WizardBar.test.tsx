// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { WizardBar } from "./WizardBar";
import type { WizardBarProps } from "./WizardBar";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("WizardBar", () => {
  let container: HTMLElement;
  let root: Root;
  const handlers = { onCancel: vi.fn(), onBack: vi.fn(), onNext: vi.fn() };
  beforeEach(() => {
    for (const fn of Object.values(handlers)) fn.mockReset();
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  const STEPS = ["Quelle", "Anordnung", "Header", "Zusammenfassung"];
  const show = (over: Partial<WizardBarProps> = {}) =>
    act(() =>
      root.render(<WizardBar titleId="t" title="Gebiet anlegen" icon="matrix" steps={STEPS} step={0} done={false} busy={false} canNext submitLabel="Anlegen" {...handlers} {...over} />),
    );
  const button = (name: string) => [...container.querySelectorAll<HTMLButtonElement>("button")].find((b) => b.textContent === name);
  const items = () => [...container.querySelectorAll<HTMLElement>('ol[aria-label="Schritte"] > li')];

  it("names_the_wizard_and_marks_the_current_step", () => {
    show({ step: 1 });
    expect(container.querySelector("#t")?.textContent).toBe("Gebiet anlegen");
    expect(items().map((li) => li.getAttribute("aria-label"))).toEqual(["1. Quelle", "2. Anordnung", "3. Header", "4. Zusammenfassung"]);
    expect(items().map((li) => li.getAttribute("aria-current"))).toEqual([null, "step", null, null]);
    expect(items().map((li) => li.dataset.state)).toEqual(["done", "current", "open", "open"]);
  });

  it("offers_zurueck_from_the_second_step_on", () => {
    show({ step: 0 });
    expect(button("Zurück")).toBeUndefined();
    show({ step: 1 });
    act(() => button("Zurück")!.click());
    expect(handlers.onBack).toHaveBeenCalledOnce();
    expect(button("Zurück")!.getAttribute("aria-keyshortcuts")).toBe("Meta+BracketLeft");
  });

  it("says_weiter_on_its_primary_button_with_the_enter_key_beside_it_and_the_submit_label_last", () => {
    show({ step: 0 });
    const next = button("Weiter")!;
    expect(next.textContent).toBe("Weiter");
    expect(next.parentElement?.querySelector("kbd")?.textContent).toBe("↵");
    act(() => next.click());
    expect(handlers.onNext).toHaveBeenCalledOnce();
    show({ step: 1, canNext: false });
    expect(button("Weiter")!.disabled).toBe(true);
    show({ step: 3 });
    expect(button("Weiter")).toBeUndefined();
    expect(button("Anlegen")).toBeDefined();
    act(() => button("Abbrechen")!.click());
    expect(handlers.onCancel).toHaveBeenCalledOnce();
  });

  it("ticks_every_step_and_leaves_only_the_title_when_done", () => {
    show({ step: 3, done: true });
    expect(items().map((li) => li.dataset.state)).toEqual(["done", "done", "done", "done"]);
    expect(container.querySelector('[aria-current="step"]')).toBeNull();
    expect(container.querySelectorAll("button")).toHaveLength(0);
  });
});
