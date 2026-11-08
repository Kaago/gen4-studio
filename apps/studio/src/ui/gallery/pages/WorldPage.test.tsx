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

  it("shows_the_paint_tools_the_palette_and_the_layers", async () => {
    const tools = state("malwerkzeuge");
    expect([...tools.querySelectorAll("[data-paint-tool]")].map((b) => b.getAttribute("data-paint-tool"))).toEqual(["brush", "rect", "fill", "erase", "pick", "height", "ramp", "terrain", "cliff"]);
    expect(tools.querySelector('[data-paint-tool="brush"]')?.getAttribute("aria-pressed")).toBe("true");
    act(() => tools.querySelector<HTMLButtonElement>('[data-paint-tool="fill"]')!.click());
    expect(tools.querySelector('[data-paint-tool="fill"]')?.getAttribute("aria-pressed")).toBe("true");

    // Placeholders: no project, every tile in the colour of its role.
    await act(async () => new Promise<void>((resolve) => setTimeout(resolve, 0)));
    const palette = state("palette").querySelector("[data-paint-palette]")!;
    expect([...palette.querySelectorAll("[data-palette-role] h4")].map((h) => h.textContent)).toEqual(["Boden", "Gras", "Wasser", "Wand", "Rampe", "Teilmodell"]);
    expect(palette.querySelector("canvas")).toBeNull();
    const chosen = palette.querySelector('[aria-pressed="true"]')!;
    expect(chosen.querySelector("[data-palette-mark]")).not.toBeNull();
    act(() => palette.querySelector<HTMLButtonElement>('[data-palette-tile="0001"]')!.click());
    expect(palette.querySelector('[data-palette-tile="0001"]')?.getAttribute("aria-pressed")).toBe("true");

    const layers = state("ebenen").querySelector("[data-paint-layers]")!;
    expect(layers.querySelector('[data-layer-row="0"]')?.textContent).toContain("Boden · Boden");
    expect(layers.querySelector('[data-layer-row="1"]')?.textContent).toContain("Büsche · Überlagerung");
    const eye = layers.querySelector<HTMLButtonElement>('[data-layer-row="1"] button[aria-label="Verbergen"]')!;
    act(() => eye.click());
    expect(layers.querySelector('[data-layer-row="1"] button[aria-label="Zeigen"]')?.getAttribute("aria-pressed")).toBe("true");
  });

  it("shows_the_edge_set_editor_with_placeholders", async () => {
    await act(async () => new Promise<void>((resolve) => setTimeout(resolve, 0)));
    const block = state("kantensatz");
    expect(block.querySelector("h2")?.textContent).toBe("Kantensatz-Editor");
    const sets = block.querySelector("[data-edge-sets]")!;
    expect([...sets.querySelectorAll("[data-edge-set]")].map((r) => r.getAttribute("data-edge-set"))).toEqual(["Weg", "Fels"]);
    expect(sets.querySelector('[data-edge-set="Weg"]')?.textContent).toContain("13 von 13 Plätzen · Diagonalen 0 von 2");
    const editor = sets.querySelector("[data-edge-editor]")!;
    expect(editor.querySelectorAll("[data-edge-place]")).toHaveLength(15);
    // Placeholders, no pictures: an empty place, a filled one, the chosen one, one with a counter.
    expect(editor.querySelector("canvas")).toBeNull();
    expect(editor.querySelector('[data-edge-place="diag_nwse"]')?.hasAttribute("data-edge-empty")).toBe(true);
    expect(editor.querySelector('[data-edge-place="center"] [data-edge-count]')?.textContent).toBe("2");
    expect(editor.querySelector('[data-edge-place="outer_nw"]')?.getAttribute("aria-pressed")).toBe("true");
    expect(editor.querySelectorAll("[data-edge-variant]").length).toBeGreaterThan(0);
    act(() => editor.querySelector<HTMLButtonElement>('[data-edge-place="n"]')!.click());
    expect(editor.querySelector('[data-edge-place="n"]')?.getAttribute("aria-pressed")).toBe("true");
    expect(sets.querySelector("[data-edge-block]")?.textContent).toContain("Untere Zelle");
  });

  it("shows_the_terrain_patterns_with_their_names", () => {
    const block = state("gelaende-muster");
    expect(block.querySelector("h2")?.textContent).toBe("Gelände-Muster");
    const swatches = [...block.querySelectorAll("[data-pattern]")];
    expect(swatches.map((s) => s.getAttribute("data-pattern"))).toEqual(["hatch", "waves", "dots", "arrow", "crosshatch"]);
    for (const s of swatches) expect(s.querySelector("svg")).not.toBeNull();
    const labels = [...block.querySelectorAll("li")].map((li) => li.textContent);
    expect(labels).toEqual(["Blockiert · Schraffur", "Wasser · Wellen", "Hohes Gras · Punkte", "Vorsprung · Pfeil in Sprungrichtung", "Eis · Kreuzschraffur"]);
  });

  it("speaks_bauen_without_raster_matrix_or_header", () => {
    const bauen = [...container.querySelectorAll('[data-depth="bauen"]')];
    expect(bauen.length).toBeGreaterThanOrEqual(10);
    for (const part of bauen) expect(visibleWords(part)).not.toMatch(/Raster|Matrix|Header/);
  });
});
