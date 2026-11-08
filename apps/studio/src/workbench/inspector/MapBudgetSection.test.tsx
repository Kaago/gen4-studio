// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { getMapBudget } from "../../api";
import type { MapBudgetDto } from "../../api";
import type { WorldBakeReport } from "../../api.world";
import type { Depth } from "../../ui/frame/shellState";
import { resetSectionFolds } from "./foldState";
import { MapBudgetSection } from "./MapBudgetSection";

vi.mock("../../api", async (importOriginal) => ({ ...(await importOriginal<typeof import("../../api")>()), getMapBudget: vi.fn() }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const flush = () => act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));

const budget = (over: Partial<MapBudgetDto> = {}): MapBudgetDto => ({
  map: 10,
  model: { used: 21452, max: 61440, state: "calm" },
  bdhc: { used: 300, max: 36864, state: "calm" },
  buildings: { used: 3, max: 32, state: "calm" },
  textures: { area: 3, map_pack: 1, building_pack: 2, textures: 38, vanilla_max: 110640, vram: { used: 37744, max: 262144, state: "calm" } },
  textures_missing: null,
  ...over,
});

/** A promise and the hand that settles it. */
function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((r) => (resolve = r));
  return { promise, resolve };
}

describe("MapBudgetSection", () => {
  let container: HTMLElement;
  let root: Root;
  const onError = vi.fn();
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    resetSectionFolds();
    onError.mockReset();
    vi.mocked(getMapBudget).mockReset().mockImplementation(async () => budget());
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  async function show(map = 10, revision = 0, depth: Depth = "bauen", bake?: WorldBakeReport | null) {
    await act(async () => root.render(<MapBudgetSection map={map} revision={revision} depth={depth} onError={onError} bake={bake} />));
    await flush();
  }
  const row = (key: string) => container.querySelector<HTMLElement>(`[data-map-budget] [data-budget="${key}"]`);

  it("shows_the_four_budgets_with_states", async () => {
    await show();
    const section = container.querySelector("[data-map-budget]")!;
    expect(section.querySelector("h3")?.textContent).toBe("Budget");
    expect(section.textContent).toContain("Grenzen des Spiels; Speichern geht immer.");
    expect(row("model")?.textContent).toContain("Modell");
    expect(row("model")?.textContent).toContain("21.452 von 61.440 B");
    expect(row("bdhc")?.textContent).toContain("Terrain (BDHC)");
    expect(row("bdhc")?.textContent).toContain("300 von 36.864 B");
    expect(row("buildings")?.textContent).toContain("Gebäude");
    expect(row("buildings")?.textContent).toContain("3 von 32");
    expect(row("vram")?.textContent).toContain("Texturen des Gebiets");
    expect(row("vram")?.textContent).toContain("VRAM 37.744 von 262.144 B · 38 Texturen");
    expect(row("vram")?.textContent).toContain("Vanilla höchstens 110.640 B");
    expect(container.querySelectorAll("[data-map-budget] [role=meter]")).toHaveLength(4);
    // All calm: no badge anywhere; Bauen shows no hex.
    expect(section.textContent).not.toMatch(/Knapp|Voll|0x/);
    expect(getMapBudget).toHaveBeenCalledWith(10);

    vi.mocked(getMapBudget).mockImplementation(async () =>
      budget({ model: { used: 58000, max: 61440, state: "tight" }, buildings: { used: 33, max: 32, state: "full" } }),
    );
    await show(10, 1, "tuefteln");
    expect(row("model")?.querySelector("[data-budget-state]")?.getAttribute("data-budget-state")).toBe("tight");
    expect(row("model")?.textContent).toContain("Knapp");
    expect(row("buildings")?.querySelector("[data-budget-state]")?.getAttribute("data-budget-state")).toBe("full");
    expect(row("buildings")?.textContent).toContain("Voll");
    expect(row("bdhc")?.textContent).not.toMatch(/Knapp|Voll/);
    // Tüfteln adds the limits in hex.
    expect(row("model")?.querySelector("[data-tech]")?.textContent).toBe("0xF000");
    expect(row("bdhc")?.querySelector("[data-tech]")?.textContent).toBe("0x9000");
  });

  it("textures_missing_names_the_reason", async () => {
    vi.mocked(getMapBudget).mockImplementation(async () => budget({ textures: null, textures_missing: "Keine Karte zeigt diese Karte in einem Header." }));
    await show();
    expect(row("vram")?.textContent).toContain("Texturen des Gebiets");
    expect(row("vram")?.textContent).toContain("Keine Karte zeigt diese Karte in einem Header.");
    expect(row("vram")?.querySelector("[role=meter]")).toBeNull();
    expect(container.querySelectorAll("[data-map-budget] [role=meter]")).toHaveLength(3);
  });

  it("refetches_after_a_revision", async () => {
    await show();
    expect(getMapBudget).toHaveBeenCalledTimes(1);
    vi.mocked(getMapBudget).mockImplementation(async () => budget({ bdhc: { used: 21452, max: 36864, state: "calm" } }));
    await show(10, 1);
    expect(getMapBudget).toHaveBeenCalledTimes(2);
    expect(row("bdhc")?.textContent).toContain("21.452 von 36.864 B");
    // The same map and revision read nothing again.
    await show(10, 1);
    expect(getMapBudget).toHaveBeenCalledTimes(2);
  });

  it("drops_an_older_answer", async () => {
    const slow = deferred<MapBudgetDto>();
    vi.mocked(getMapBudget).mockImplementationOnce(() => slow.promise);
    await show(10);
    expect(container.querySelector("[data-map-budget]")?.textContent).toContain("Wird gelesen …");
    expect(row("model")).toBeNull();
    // Another map is shown before the first answer is there; its answer comes first.
    vi.mocked(getMapBudget).mockImplementationOnce(async () => budget({ map: 11, model: { used: 1000, max: 61440, state: "calm" } }));
    await show(11);
    expect(row("model")?.textContent).toContain("1.000 von 61.440 B");
    // The late answer of map 10 changes nothing.
    await act(async () => slow.resolve(budget({ model: { used: 9999, max: 61440, state: "calm" } })));
    await flush();
    expect(row("model")?.textContent).toContain("1.000 von 61.440 B");
    expect(onError).not.toHaveBeenCalled();
  });

  it("budget_follows_each_stroke", async () => {
    // Each stroke is a revision: the budget is read after each, the second number is the one shown.
    vi.mocked(getMapBudget).mockImplementationOnce(async () => budget({ model: { used: 30000, max: 61440, state: "calm" } }));
    await show(10, 1);
    expect(row("model")?.textContent).toContain("30.000 von 61.440 B");
    vi.mocked(getMapBudget).mockImplementationOnce(async () => budget({ model: { used: 31500, max: 61440, state: "calm" } }));
    await show(10, 2);
    expect(getMapBudget).toHaveBeenCalledTimes(2);
    expect(row("model")?.textContent).toContain("31.500 von 61.440 B");
  });

  it("a_refused_bake_says_the_game_keeps_the_last_model", async () => {
    const bake = (over: Partial<WorldBakeReport> = {}): WorldBakeReport => ({
      map: 10,
      ok: true,
      style: "outdoor",
      faces: 900,
      model_bytes: 70000,
      model_share: 1.1,
      model_kept: false,
      textures: [],
      vram_bytes: 0,
      bdhc_bytes: 0,
      bdhc_payload: 0,
      plates: 0,
      blocked: 0,
      verbatim_model: false,
      verbatim_terrain: false,
      messages: [],
      ...over,
    });
    await show(10, 0, "bauen", bake());
    expect(container.querySelector("[data-budget-refused]")).toBeNull();
    await show(10, 0, "bauen", null);
    expect(container.querySelector("[data-budget-refused]")).toBeNull();
    await show(
      10,
      1,
      "bauen",
      bake({
        ok: false,
        model_kept: true,
        messages: [
          { code: "w", level: "warning", args: [], message: "Eine Warnung." },
          { code: "model-too-big", level: "error", args: [], message: "Das Modell ist zu groß (70.000 von 61.440 B)." },
        ],
      }),
    );
    const refused = container.querySelector("[data-budget-refused]");
    expect(refused?.textContent).toBe("Nicht gebacken: Das Modell ist zu groß (70.000 von 61.440 B). Das Spiel bekommt das letzte Modell.");
    // The sentence stands under the row „Modell“.
    expect(row("model")?.nextElementSibling).toBe(refused);
  });

  it("reports_a_failed_read", async () => {
    vi.mocked(getMapBudget).mockImplementation(async () => {
      throw new Error("kaputt");
    });
    await show();
    expect(onError).toHaveBeenCalledWith(new Error("kaputt"));
    expect(container.querySelector("[data-map-budget]")?.textContent).not.toContain("Wird gelesen …");
  });
});
