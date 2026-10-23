// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { getBuildingGlb, getMap, updateBuilding } from "../../api";
import type { BuildingDto, MapDto } from "../../api";
import { typeInto } from "../../ui/primitives/testUtils";
import { BuildingEditor } from "./BuildingEditor";
import type { BuildingEditorContext } from "./BuildingEditor";

vi.mock("../../api", async (importOriginal) => ({ ...(await importOriginal<typeof import("../../api")>()), getMap: vi.fn(), updateBuilding: vi.fn(), getBuildingGlb: vi.fn() }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const flush = () => act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));
const building = (over: Partial<BuildingDto> = {}): BuildingDto => ({ model_id: 7, x: 1, y: 0, z: -2, sx: 1, sy: 1, sz: 1, rotation: ["000000AB", "00000000", "00000000"], ...over });
const map = (): MapDto => ({ index: 12, permissions: [], buildings: [building({ model_id: 1 }), building({ model_id: 2 }), building()] });

describe("BuildingEditor", () => {
  let container: HTMLElement;
  let root: Root;
  const onChanged = vi.fn();
  const onAdd = vi.fn();
  const onDelete = vi.fn();
  const onError = vi.fn();
  const context = (over: Partial<BuildingEditorContext> = {}): BuildingEditorContext => ({ areaDataId: 3, counts: { BuildingModel: 50 }, revision: 0, onChanged, onAdd, onDelete, onError, ...over });
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    vi.mocked(getMap).mockReset().mockImplementation(async () => map());
    vi.mocked(updateBuilding).mockReset().mockResolvedValue(undefined);
    vi.mocked(getBuildingGlb).mockReset().mockResolvedValue(new ArrayBuffer(2048));
    for (const fn of [onChanged, onAdd, onDelete, onError]) fn.mockReset();
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });
  async function show(ctx = context()) {
    await act(async () => root.render(<BuildingEditor map={12} index={2} context={ctx} />));
    await flush();
  }
  const field = (label: string) => container.querySelector<HTMLInputElement>(`input[aria-label='${label}']`)!;
  const button = (text: string) => [...container.querySelectorAll("button")].find((b) => b.textContent === text) as HTMLButtonElement;
  const click = (el: Element | undefined) => act(async () => void (el as HTMLElement).click());
  const typeAndBlur = async (el: HTMLInputElement, text: string) => {
    await act(async () => void el.focus());
    typeInto(el, text);
    await act(async () => void el.blur());
    await flush();
  };

  it("names_the_building_by_its_place_in_the_map", async () => {
    await show();
    expect(container.querySelector("h2")?.textContent).toBe("Gebäude 3 von 3");
    expect(container.textContent).toContain("Karte 0012");
  });

  it("shows_the_fields_and_the_rotation_as_read_only_hex", async () => {
    await show();
    expect(field("Modellnummer").value).toBe("7");
    expect(field("x (Kacheln)").value).toBe("1");
    expect(field("Skalierung y").value).toBe("1");
    expect(container.textContent).toContain("000000AB 00000000 00000000");
    expect(container.textContent).toContain("außen 0 bis 49");
  });

  it("writes_a_changed_fraction_as_one_edit_that_sends_only_that_field", async () => {
    await show();
    await typeAndBlur(field("x (Kacheln)"), "1.5");
    expect(updateBuilding).toHaveBeenCalledWith(12, 2, { x: 1.5 });
    expect(onChanged).toHaveBeenCalledWith(12, null);
  });

  it("refuses_text_that_is_not_a_number_and_says_how_to_write_one", async () => {
    await show();
    await typeAndBlur(field("z (Kacheln)"), "abc");
    expect(updateBuilding).not.toHaveBeenCalled();
    expect(container.textContent).toContain("Das ist keine Zahl. Gib z. B. 1.5 ein.");
  });

  it("writes_a_new_model_number_and_refuses_a_negative_one_without_clamping", async () => {
    await show();
    await typeAndBlur(field("Modellnummer"), "9");
    expect(updateBuilding).toHaveBeenCalledWith(12, 2, { model_id: 9 });
    await typeAndBlur(field("Modellnummer"), "-4");
    expect(container.textContent).toContain("mindestens 0");
    expect(updateBuilding).toHaveBeenCalledTimes(1);
  });

  it("adds_and_deletes_through_the_buttons", async () => {
    await show();
    await click(button("Neues Gebäude"));
    expect(onAdd).toHaveBeenCalledWith(12);
    await click(button("Löschen"));
    expect(onDelete).toHaveBeenCalledWith(12, 2);
  });

  it("says_why_the_model_is_missing_when_it_cannot_be_loaded", async () => {
    vi.mocked(getBuildingGlb).mockRejectedValue(new Error("apicula fehlt"));
    await show();
    expect(container.textContent).toContain("Modell nicht geladen, die 3D-Ansicht zeigt eine Box");
  });
});
