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

  const escape = (target: EventTarget = document) => act(async () => void target.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true, cancelable: true })));

  it("puts_the_focus_on_the_first_control", async () => {
    await show();
    const first = container.querySelector("input");
    expect(first?.type).toBe("radio");
    expect(document.activeElement).toBe(first);
  });

  it("cancels_with_escape_without_calling_the_backend", async () => {
    await show();
    await escape();
    expect(onDone).toHaveBeenCalledOnce();
    expect(onDone).toHaveBeenCalledWith(null);
    expect(extendArea).not.toHaveBeenCalled();
  });

  it("cancels_with_escape_while_a_radio_button_has_the_focus", async () => {
    await show();
    await escape(container.querySelector("input")!);
    expect(onDone).toHaveBeenCalledWith(null);
  });

  it("takes_back_typed_text_with_escape_and_closes_only_from_an_unedited_field", async () => {
    await show();
    const field = container.querySelector<HTMLInputElement>('input[aria-label^="Header-Zelle"]')!;
    const before = field.value;
    await act(async () => field.focus());
    await act(async () => {
      Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")?.set?.call(field, "77");
      field.dispatchEvent(new Event("input", { bubbles: true }));
    });
    await escape(field);
    // The first Escape takes back "77"; the dialog stays.
    expect(onDone).not.toHaveBeenCalled();
    expect(field.value).toBe(before);
    // The field is unedited now: Escape closes.
    await escape(field);
    expect(onDone).toHaveBeenCalledWith(null);
  });

  it("keeps_tab_inside_the_dialog", async () => {
    await show();
    const controls = [...container.querySelectorAll<HTMLElement>("input:not([disabled]), select:not([disabled]), button:not([disabled])")];
    controls[controls.length - 1].focus();
    await act(async () => void document.dispatchEvent(new KeyboardEvent("keydown", { key: "Tab", bubbles: true, cancelable: true })));
    expect(document.activeElement).toBe(controls[0]);
  });

  it("ignores_escape_while_the_request_runs", async () => {
    let finish!: (report: { map: number; width: number; height: number }) => void;
    vi.mocked(extendArea).mockReturnValue(new Promise((resolve) => (finish = resolve)));
    await show();
    await act(async () => button("Anlegen")?.click());
    await escape();
    expect(onDone).not.toHaveBeenCalled();
    await act(async () => finish({ map: 600, width: 3, height: 1 }));
    expect(onDone).toHaveBeenCalledWith({ map: 600, width: 3, height: 1 });
  });
});
