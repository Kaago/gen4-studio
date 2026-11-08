// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { worldLayers } from "../../api.world";
import type { WorldJson, WorldReport } from "../../api.world";
import { typeInto } from "../../ui/primitives/testUtils";
import { resetSectionFolds } from "./foldState";
import { LayerList } from "./LayerList";

vi.mock("../../api.world", async (importOriginal) => ({ ...(await importOriginal<typeof import("../../api.world")>()), worldLayers: vi.fn() }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const flush = () => act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));

const WORLD: WorldJson = {
  version: 1,
  tileset: 3,
  layers: [
    { name: "Boden", role: "ground", tiles: [] },
    { name: "Büsche", role: "overlay", tiles: [] },
  ],
  permissions: { derive: true },
};
const REPORT: WorldReport = { map: 123, tileset: 3, cells: 0, change: 7, bake: null };

describe("LayerList", () => {
  let container: HTMLElement;
  let root: Root;
  const onActive = vi.fn();
  const onToggle = vi.fn();
  const onError = vi.fn();
  const onChanged = vi.fn();
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    resetSectionFolds();
    for (const f of [onActive, onToggle, onError, onChanged]) f.mockReset();
    vi.mocked(worldLayers).mockReset().mockResolvedValue(REPORT);
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  async function show(hidden: ReadonlySet<number> = new Set(), active = 0) {
    await act(async () =>
      root.render(<LayerList map={123} world={WORLD} active={active} hidden={hidden} onActive={onActive} onToggle={onToggle} onError={onError} onChanged={onChanged} />),
    );
    await flush();
  }
  const row = (i: number) => container.querySelector<HTMLElement>(`[data-paint-layers] [data-layer-row="${i}"]`)!;
  const buttonIn = (scope: Element, label: string) => [...scope.querySelectorAll("button")].find((b) => b.textContent === label || b.getAttribute("aria-label") === label);

  it("adding_calls_world_layers_with_the_name", async () => {
    await show();
    const section = container.querySelector("[data-paint-layers]")!;
    expect(section.querySelector("h3")?.textContent).toBe("Ebenen");
    const field = section.querySelector<HTMLInputElement>('input[aria-label="Name der neuen Ebene"]')!;
    // The name proposed is the next layer's number.
    expect(field.value).toBe("Ebene 2");
    typeInto(field, "Steine");
    await act(async () => buttonIn(section, "Ebene hinzufügen")!.click());
    await flush();
    expect(worldLayers).toHaveBeenCalledWith(123, { add: { name: "Steine" } });
    expect(onChanged).toHaveBeenCalledWith(123);
    expect(onError).not.toHaveBeenCalled();

    // A refusal goes the error's way, nothing is reported as changed.
    vi.mocked(worldLayers).mockRejectedValueOnce({ code: "invalid", message: "Ebene „Büsche“ gibt es schon.", asset: null });
    onChanged.mockReset();
    typeInto(field, "Büsche");
    await act(async () => buttonIn(section, "Ebene hinzufügen")!.click());
    await flush();
    expect(onError).toHaveBeenCalledTimes(1);
    expect(onChanged).not.toHaveBeenCalled();
  });

  it("the_ground_has_no_remove", async () => {
    await show(new Set(), 1);
    expect(row(0).textContent).toContain("Boden · Boden");
    expect(row(1).textContent).toContain("Büsche · Überlagerung");
    expect(buttonIn(row(0), "Entfernen")).toBeUndefined();
    const radios = [0, 1].map((i) => row(i).querySelector<HTMLInputElement>('input[type="radio"]')!);
    expect(radios.map((r) => r.getAttribute("aria-label"))).toEqual(["Aktiv", "Aktiv"]);
    expect(radios.map((r) => r.checked)).toEqual([false, true]);
    act(() => radios[0].click());
    expect(onActive).toHaveBeenCalledWith(0);

    await act(async () => buttonIn(row(1), "Entfernen")!.click());
    await flush();
    expect(worldLayers).toHaveBeenCalledWith(123, { remove: { layer: 1 } });
    expect(onChanged).toHaveBeenCalledWith(123);
  });

  it("hiding_does_not_call_the_kernel", async () => {
    await show(new Set([1]));
    const shown = buttonIn(row(0), "Verbergen")!;
    const hidden = buttonIn(row(1), "Zeigen")!;
    expect(shown.getAttribute("aria-pressed")).toBe("false");
    expect(hidden.getAttribute("aria-pressed")).toBe("true");
    act(() => shown.click());
    act(() => hidden.click());
    expect(onToggle.mock.calls).toEqual([[0], [1]]);
    expect(worldLayers).not.toHaveBeenCalled();
  });
});
