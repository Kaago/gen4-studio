// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { extendArea } from "../api";
import type { MatrixDto } from "../api";
import { pressKey, typeInto } from "../ui/primitives/testUtils";
import { visibleWords } from "../ui/visibleText";
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
  const dialog = () => document.querySelector<HTMLElement>("[role=dialog]")!;
  const button = (text: string) => [...dialog().querySelectorAll("button")].find((b) => b.textContent === text);
  const field = (name: string) => dialog().querySelector<HTMLInputElement>(`input[aria-label^="${name}"]`);

  it("proposes_a_copy_of_the_neighbour_with_its_header_and_announces_the_new_size", async () => {
    vi.mocked(extendArea).mockResolvedValue({ map: 600, width: 3, height: 1 });
    show();
    expect(dialog().getAttribute("aria-modal")).toBe("true");
    expect(dialog().textContent).toContain("Karte bei x 2, y 0 anlegen?");
    expect(dialog().textContent).toContain("Kopie von Karte 0011 (Nachbarzelle)");
    expect(dialog().textContent).toContain("Das Raster wird auf 3 × 1 vergrößert.");
    expect(dialog().textContent).toContain("Anlegen lässt sich nicht rückgängig machen.");
    expect(field("Header-Zelle")?.value).toBe("416");
    await act(async () => button("Karte anlegen")?.click());
    expect(extendArea).toHaveBeenCalledWith({ matrix: 7, x: 2, y: 0, source_map: 11, clear_buildings: false, header_cell: 416 });
    expect(onDone).toHaveBeenCalledWith({ map: 600, width: 3, height: 1 });
  });

  it("shows_the_not_undoable_badge", () => {
    show();
    expect(dialog().textContent).toContain("Nicht rückgängig");
  });

  it("proposes_the_given_neighbour_with_its_header", () => {
    const m: MatrixDto = { index: 7, name: "m", width: 2, height: 2, maps: [10, 11, 12, null], headers: [0, 416, 0, 0] };
    act(() => root.render(<ExtendAreaDialog matrix={m} x={1} y={1} prefer={{ x: 1, y: 0 }} headers={[]} counts={{ Map: 600 }} onDone={onDone} onError={onError} />));
    expect(dialog().textContent).toContain("Kopie von Karte 0011 (Nachbarzelle)");
    expect(field("Header-Zelle")?.value).toBe("416");
  });

  it("says_no_raster_matrix_or_header_in_building", async () => {
    vi.mocked(extendArea).mockResolvedValue({ map: 600, width: 3, height: 1 });
    act(() => root.render(<ExtendAreaDialog matrix={matrix} x={2} y={0} areaName="Zweiblattdorf" depth="bauen" headers={[]} counts={{ Map: 600 }} onDone={onDone} onError={onError} />));
    expect(visibleWords(dialog())).not.toMatch(/Raster|Matrix|Header/);
    expect(dialog().textContent).toContain("Die neue Karte gehört zu „Zweiblattdorf“.");
    expect(dialog().textContent).toContain("Die Karte wird größer: 3 × 1.");
    expect(field("Header-Zelle")).toBeNull();
    // The header of the neighbour still goes to the backend.
    await act(async () => button("Karte anlegen")?.click());
    expect(extendArea).toHaveBeenCalledWith(expect.objectContaining({ source_map: 11, header_cell: 416 }));
    // Without a neighbour the number is asked for, in the words of Bauen; a cell inside keeps the size.
    act(() => root.render(<ExtendAreaDialog matrix={{ ...matrix, maps: [null, null] }} x={1} y={0} depth="bauen" headers={[]} counts={{ Map: 600 }} onDone={onDone} onError={onError} />));
    expect(visibleWords(dialog())).not.toMatch(/Raster|Matrix|Header/);
    expect(field("Nummer des Gebiets")).not.toBeNull();
    expect(dialog().textContent).toContain("Die Größe bleibt 2 × 1.");
  });

  it("copies_the_chosen_map_without_buildings", async () => {
    vi.mocked(extendArea).mockResolvedValue({ map: 600, width: 3, height: 1 });
    show();
    expect(field("Karte")).toBeNull();
    const choose = [...dialog().querySelectorAll("button")].find((b) => b.textContent === "Karte wählen")!;
    act(() => choose.click());
    await act(async () => field("Karte")!.focus());
    typeInto(field("Karte")!, "42");
    await act(async () => field("Karte")!.blur());
    act(() => [...dialog().querySelectorAll<HTMLElement>("[role=checkbox]")].find((c) => c.closest("label")?.textContent === "ohne Gebäude")!.click());
    await act(async () => button("Karte anlegen")?.click());
    expect(extendArea).toHaveBeenCalledWith({ matrix: 7, x: 2, y: 0, source_map: 42, clear_buildings: true, header_cell: 416 });
  });

  it("refuses_a_map_beyond_the_archive_and_names_the_highest", async () => {
    show({ ...matrix, maps: [null, null], headers: undefined });
    expect(dialog().textContent).not.toContain("Nachbarzelle");
    await act(async () => field("Karte")!.focus());
    typeInto(field("Karte")!, "700");
    await act(async () => field("Karte")!.blur());
    expect(dialog().textContent).toContain("599");
    expect(field("Karte")!.value).toBe("0");
    await act(async () => button("Karte anlegen")?.click());
    // Nothing was taken over: the old value, never the clamped 599, goes to the backend.
    expect(extendArea).toHaveBeenCalledWith(expect.objectContaining({ source_map: 0 }));
  });

  it("sends_no_header_cell_for_a_grid_without_header_section", async () => {
    vi.mocked(extendArea).mockResolvedValue({ map: 600, width: 3, height: 1 });
    show({ ...matrix, headers: undefined });
    expect(field("Header-Zelle")).toBeNull();
    await act(async () => button("Karte anlegen")?.click());
    expect(vi.mocked(extendArea).mock.calls[0][0].header_cell).toBeNull();
  });

  it("reports_a_failure_and_leaves_without_a_result", async () => {
    vi.mocked(extendArea).mockRejectedValue(new Error("boom"));
    show();
    await act(async () => button("Karte anlegen")?.click());
    expect(onError).toHaveBeenCalledOnce();
    expect(onDone).toHaveBeenCalledWith(null);
  });

  it("cancels_without_calling_the_backend", () => {
    show();
    act(() => button("Abbrechen")?.click());
    expect(extendArea).not.toHaveBeenCalled();
    expect(onDone).toHaveBeenCalledWith(null);
  });

  it("puts_the_focus_inside_the_dialog_on_a_control_not_on_the_cross", async () => {
    show();
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 50));
    });
    expect(dialog().contains(document.activeElement)).toBe(true);
    expect(document.activeElement?.getAttribute("aria-label")).not.toBe("Schließen");
  });

  it("cancels_with_escape_without_calling_the_backend", () => {
    show();
    pressKey((document.activeElement as HTMLElement) ?? dialog(), "Escape");
    expect(onDone).toHaveBeenCalledOnce();
    expect(onDone).toHaveBeenCalledWith(null);
    expect(extendArea).not.toHaveBeenCalled();
  });

  it("cancels_with_the_cross", () => {
    show();
    act(() => dialog().querySelector<HTMLElement>("button[aria-label='Schließen']")!.click());
    expect(onDone).toHaveBeenCalledWith(null);
  });

  it("takes_back_typed_text_with_escape_and_closes_only_from_an_unedited_field", async () => {
    show();
    const input = field("Header-Zelle")!;
    const before = input.value;
    await act(async () => input.focus());
    typeInto(input, "77");
    pressKey(input, "Escape");
    // The first Escape takes back "77"; the dialog stays.
    expect(onDone).not.toHaveBeenCalled();
    expect(input.value).toBe(before);
    // The field is unedited now: Escape closes.
    pressKey(input, "Escape");
    expect(onDone).toHaveBeenCalledWith(null);
  });

  it("keeps_the_tab_stops_inside_the_dialog", async () => {
    show();
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 50));
    });
    // Base UI traps the focus with guards around the popup; every control the dialog offers sits inside it.
    const controls = [...dialog().querySelectorAll<HTMLElement>("input:not([disabled]), button:not([disabled])")];
    expect(controls.length).toBeGreaterThan(3);
    expect(controls.every((c) => dialog().contains(c))).toBe(true);
    expect(dialog().contains(document.activeElement)).toBe(true);
  });

  it("ignores_escape_and_a_second_press_while_the_request_runs", async () => {
    let finish!: (report: { map: number; width: number; height: number }) => void;
    vi.mocked(extendArea).mockReturnValue(new Promise((resolve) => (finish = resolve)));
    show();
    await act(async () => button("Karte anlegen")?.click());
    pressKey((document.activeElement as HTMLElement) ?? dialog(), "Escape");
    expect(onDone).not.toHaveBeenCalled();
    await act(async () => button("Karte anlegen")?.click());
    expect(extendArea).toHaveBeenCalledOnce();
    await act(async () => finish({ map: 600, width: 3, height: 1 }));
    expect(onDone).toHaveBeenCalledWith({ map: 600, width: 3, height: 1 });
  });
});
