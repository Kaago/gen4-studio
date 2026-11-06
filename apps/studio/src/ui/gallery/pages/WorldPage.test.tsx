// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { visibleWords } from "../../visibleText";
import { WorldPage } from "./WorldPage";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("WorldPage", () => {
  let container: HTMLElement;
  let root: Root;
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    act(() => root.render(<WorldPage />));
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  const state = (name: string) => container.querySelector<HTMLElement>(`[data-gallery-state="${name}"]`)!;
  const button = (scope: Element, label: string) => [...scope.querySelectorAll("button")].find((b) => b.textContent === label);

  it("shows_the_world_map_heads_and_the_room_plan_with_its_plus_cells", () => {
    const heads = [...state("umschalter").querySelectorAll("[data-world-view]")];
    expect(heads.map((h) => h.getAttribute("data-world-view"))).toEqual(["plan", "w0k"]);
    expect(heads[0].textContent).toContain("Raumplan");
    expect(heads[0].textContent).toContain("Ewigwald, Ebene 1");
    expect(heads[1].textContent).toContain("Kanto");

    const plan = state("raumplan");
    expect(plan.querySelector('[role="group"]')?.getAttribute("aria-label")).toBe("Raumplan, 3 Zeilen, 3 Spalten");
    // Eight maps; the empty ninth cell is a „+“.
    expect(plan.querySelectorAll("[data-cell]")).toHaveLength(8);
    expect([...plan.querySelectorAll("[data-extend]")].map((b) => b.getAttribute("data-extend"))).toEqual(["3,0", "3,1", "2,2", "3,2", "0,3", "1,3", "2,3"]);
    expect(plan.querySelector('[data-extend="3,0"]')?.getAttribute("aria-label")).toBe("Karte bei x 3, y 0 anlegen (nicht rückgängig)");
    expect(plan.querySelector('[data-cell="1,1"]')?.getAttribute("aria-pressed")).toBe("true");
    act(() => plan.querySelector<HTMLButtonElement>('[data-cell="0,0"]')!.click());
    expect(plan.querySelector('[data-cell="0,0"]')?.getAttribute("aria-pressed")).toBe("true");
  });

  it("shows_the_place_picker_the_new_place_form_the_patch_line_and_the_shared_badge", () => {
    const pickers = [...state("ort").querySelectorAll("button[data-place-picker]")].map((b) => b.textContent);
    expect(pickers).toEqual(["Zweiblattdorf", "Kein Ort", "ZweiblattdorfNachricht 1 · Archiv 0433"]);

    const form = state("neuer-ort").querySelector("[data-new-place]")!;
    expect(form.textContent).toContain("Name des Orts");
    expect(form.querySelector('[role="group"][aria-label="Mitnehmen"]')).not.toBeNull();
    expect(button(form, "Ort anlegen")?.disabled).toBe(false);

    // „Anlegen“ waits for the consent.
    const patch = state("patch");
    expect(patch.querySelector("[data-patch-consent]")?.textContent).toContain("Nicht rückgängig");
    expect(patch.textContent).toContain("Einmalig für dieses Projekt: Spielcode für neue Orte anpassen (DSPRE-kompatibel, Sicherung arm9.bin.backup)");
    const create = () => button(patch.querySelector("[data-patch-consent]")!.parentElement!, "Anlegen")!;
    expect(create().disabled).toBe(true);
    act(() => patch.querySelector<HTMLElement>('[data-patch-consent] [role="checkbox"], [data-patch-consent] input[type="checkbox"]')!.click());
    expect(create().disabled).toBe(false);
    expect(patch.textContent).toContain("Reparieren in Technik › Patches.");

    const shared = state("geteilt");
    expect(shared.querySelector("[data-shared]")?.textContent).toBe("Geteilt mit 16 Räumen");
    expect(shared.querySelector("[data-split-room]")?.textContent).toContain("Nicht rückgängig");
  });

  it("shows_the_budget_in_three_states_and_the_seven_height_steps", () => {
    const budgets = state("budget");
    expect([...budgets.querySelectorAll("[data-budget-state]")].map((b) => b.getAttribute("data-budget-state"))).toEqual(["calm", "tight", "full"]);
    expect(budgets.textContent).toContain("Knapp");
    expect(budgets.textContent).toContain("Voll");
    expect(budgets.querySelectorAll("[role=meter]")).toHaveLength(3);

    const steps = [...state("hoehen").querySelectorAll("[data-height-step]")];
    expect(steps.map((s) => s.getAttribute("data-height-step"))).toEqual(["0", "1", "2", "3", "4", "5", "6"]);
    expect(steps[0].textContent).toBe("Stufe 0 · 0 E");
    expect(steps[6].textContent).toBe("Stufe 6 · 48 E");
    expect(state("hoehen").textContent).toContain("Isolinien");
  });

  it("speaks_bauen_without_raster_matrix_or_header", () => {
    const bauen = [...container.querySelectorAll('[data-depth="bauen"]')];
    expect(bauen.length).toBeGreaterThanOrEqual(7);
    for (const part of bauen) expect(visibleWords(part)).not.toMatch(/Raster|Matrix|Header/);
  });
});
