// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { extendArea } from "../api";
import type { MatrixDto } from "../api";
import { ExtendAreaDialog } from "./ExtendAreaDialog";

vi.mock("../api", async (importOriginal) => ({ ...(await importOriginal<typeof import("../api")>()), extendArea: vi.fn() }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const matrix: MatrixDto = { index: 7, name: "m", width: 2, height: 1, maps: [10, 11], headers: [415, 416] };

describe("ExtendAreaDialog", () => {
  let container: HTMLElement;
  let root: Root;
  const onDone = vi.fn();
  const onError = vi.fn();
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    vi.mocked(extendArea).mockReset();
    onDone.mockReset();
    onError.mockReset();
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  const show = (m: MatrixDto = matrix) =>
    act(() => root.render(<ExtendAreaDialog matrix={m} x={2} y={0} headers={[]} counts={{ Map: 600 }} onDone={onDone} onError={onError} />));
  const button = (text: string) => [...container.querySelectorAll("button")].find((b) => b.textContent === text);

  it("proposes_a_copy_of_the_neighbour_with_its_header_and_announces_the_new_size", async () => {
    vi.mocked(extendArea).mockResolvedValue({ map: 600, width: 3, height: 1 });
    show();
    expect(container.textContent).toContain("Kopie von Karte 0011 (Nachbarzelle)");
    expect(container.textContent).toContain("Das Raster wird auf 3 × 1 vergrößert. Anlegen lässt sich nicht rückgängig machen.");
    expect(container.querySelector<HTMLInputElement>('input[aria-label^="Header-Zelle"]')?.value).toBe("416");
    await act(async () => button("Anlegen")?.click());
    expect(extendArea).toHaveBeenCalledWith({ matrix: 7, x: 2, y: 0, source_map: 11, clear_buildings: false, header_cell: 416 });
    expect(onDone).toHaveBeenCalledWith({ map: 600, width: 3, height: 1 });
  });

  it("sends_no_header_cell_for_a_grid_without_header_section", async () => {
    vi.mocked(extendArea).mockResolvedValue({ map: 600, width: 3, height: 1 });
    show({ ...matrix, headers: undefined });
    expect(container.querySelector('input[aria-label^="Header-Zelle"]')).toBeNull();
    await act(async () => button("Anlegen")?.click());
    expect(vi.mocked(extendArea).mock.calls[0][0].header_cell).toBeNull();
  });

  it("reports_a_failure_and_leaves_without_a_result", async () => {
    vi.mocked(extendArea).mockRejectedValue(new Error("boom"));
    show();
    await act(async () => button("Anlegen")?.click());
    expect(onError).toHaveBeenCalledOnce();
    expect(onDone).toHaveBeenCalledWith(null);
  });

  it("cancels_without_calling_the_backend", () => {
    show();
    act(() => button("Abbrechen")?.click());
    expect(extendArea).not.toHaveBeenCalled();
    expect(onDone).toHaveBeenCalledWith(null);
  });
});
