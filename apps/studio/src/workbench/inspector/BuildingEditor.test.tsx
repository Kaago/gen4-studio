// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { getAreaData, getBuildingGlb, getMap, updateBuilding } from "../../api";
import type { AreaDataDto, BuildingDto, MapDto } from "../../api";
import { typeInto } from "../../ui/primitives/testUtils";
import { BuildingEditor } from "./BuildingEditor";
import type { BuildingEditorContext } from "./BuildingEditor";
import { chooseAction, headAction, headActions } from "./headTesting";
import { MISSING, isDisabled, resetSpy, withReset } from "../../original/resetTesting";

vi.mock("../../api", async (importOriginal) => ({ ...(await importOriginal<typeof import("../../api")>()), getMap: vi.fn(), updateBuilding: vi.fn(), getBuildingGlb: vi.fn(), getAreaData: vi.fn() }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const flush = () => act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));
const building = (over: Partial<BuildingDto> = {}): BuildingDto => ({ model_id: 7, x: 1, y: 0, z: -2, sx: 1, sy: 1, sz: 1, rotation: ["000000AB", "00000000", "00000000"], ...over });
const outside: AreaDataDto = { family: "Dppt", buildings_tileset: 1, map_tileset: 2, unknown: 0, light_type: 0 };
const map = (): MapDto => ({ index: 12, permissions: [], buildings: [building({ model_id: 1 }), building({ model_id: 2 }), building()] });

describe("BuildingEditor", () => {
  let container: HTMLElement;
  let root: Root;
  const onChanged = vi.fn();
  const onAdd = vi.fn();
  const onDelete = vi.fn();
  const onError = vi.fn();
  let reset = resetSpy();
  const context = (over: Partial<BuildingEditorContext> = {}): BuildingEditorContext => ({ areaDataId: 3, counts: { BuildingModel: 50 }, dirty: [], revision: 0, onChanged, onAdd, onDelete, onError, ...over });
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    vi.mocked(getMap).mockReset().mockImplementation(async () => map());
    vi.mocked(updateBuilding).mockReset().mockResolvedValue(undefined);
    vi.mocked(getBuildingGlb).mockReset().mockResolvedValue(new ArrayBuffer(2048));
    vi.mocked(getAreaData).mockReset().mockResolvedValue(outside);
    for (const fn of [onChanged, onAdd, onDelete, onError]) fn.mockReset();
    reset = resetSpy();
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });
  async function show(ctx = context(), depth: "bauen" | "tuefteln" = "bauen") {
    await act(async () => root.render(withReset(reset, <BuildingEditor map={12} index={2} context={ctx} depth={depth} />)));
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
    expect(container.querySelector("header[data-inspector-kind='building'] [data-eyebrow]")?.textContent).toBe("Gebäude");
    expect(container.querySelector("h2")?.textContent).toBe("Gebäude 3 von 3");
    expect(container.querySelector("[data-ident]")?.textContent).toBe("Karte 0012");
  });

  it("says_Ungespeichert_while_the_map_has_unsaved_changes_and_not_for_another_map", async () => {
    await show(context({ dirty: [{ kind: "Map", index: 13 }, { kind: "Event", index: 12 }] }));
    expect(container.textContent).not.toContain("Ungespeichert");
    await show(context({ dirty: [{ kind: "Map", index: 12 }] }));
    expect(container.textContent).toContain("Ungespeichert");
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

  it("drops_the_refusal_as_soon_as_the_user_types_again", async () => {
    await show();
    await typeAndBlur(field("z (Kacheln)"), "abc");
    expect(container.textContent).toContain("keine Zahl");
    await act(async () => void field("z (Kacheln)").focus());
    typeInto(field("z (Kacheln)"), "1");
    expect(container.textContent).not.toContain("keine Zahl");
  });

  it("writes_a_new_model_number_and_refuses_a_negative_one_without_clamping", async () => {
    await show();
    await typeAndBlur(field("Modellnummer"), "9");
    expect(updateBuilding).toHaveBeenCalledWith(12, 2, { model_id: 9 });
    await typeAndBlur(field("Modellnummer"), "-4");
    expect(container.textContent).toContain("mindestens 0");
    expect(updateBuilding).toHaveBeenCalledTimes(1);
  });

  it("adds_through_the_button", async () => {
    await show();
    await click(button("Neues Gebäude"));
    expect(onAdd).toHaveBeenCalledWith(12);
  });

  it("deletes_from_the_head_menu", async () => {
    await show();
    // „Löschen“ is no button in the form any more: it is the head's action, with its key.
    expect(button("Löschen")).toBeUndefined();
    expect(headActions(container)).toEqual(["LöschenEntf", "Gebäudemodell auf Original zurücksetzen…"]);
    chooseAction(container, "Löschen");
    expect(onDelete).toHaveBeenCalledWith(12, 2);
  });

  it("head_offers_the_model", async () => {
    await show();
    // The building's model (7), from the archive the scene loads it from: outside, the exterior models.
    chooseAction(container, "Gebäudemodell auf Original zurücksetzen…");
    expect(getAreaData).toHaveBeenCalledWith(3);
    expect(reset.open).toHaveBeenLastCalledWith({ kind: "building_model", index: 7 });
    // HeartGold and SoulSilver take the models of an interior area (area type 0) from the interior archive, so does the reset.
    vi.mocked(getAreaData).mockResolvedValue({ family: "Hgss", buildings_tileset: 1, map_tileset: 2, dynamic_texture_type: 0, area_type: 0, light_type: 0 });
    await act(async () => root.render(null));
    await show();
    chooseAction(container, "Gebäudemodell auf Original zurücksetzen…");
    expect(reset.open).toHaveBeenLastCalledWith({ kind: "interior_building_model", index: 7 });
    // An outside HGSS area uses the exterior archive again.
    vi.mocked(getAreaData).mockResolvedValue({ family: "Hgss", buildings_tileset: 1, map_tileset: 2, dynamic_texture_type: 0, area_type: 1, light_type: 0 });
    await act(async () => root.render(null));
    await show();
    chooseAction(container, "Gebäudemodell auf Original zurücksetzen…");
    expect(reset.open).toHaveBeenLastCalledWith({ kind: "building_model", index: 7 });
    // Without a base ROM: locked with the reason.
    await act(async () => root.render(null));
    reset = resetSpy(MISSING);
    await show();
    const item = headAction(container, "Gebäudemodell auf Original zurücksetzen…");
    expect(isDisabled(item)).toBe(true);
    expect(item?.textContent).toBe("Gebäudemodell auf Original zurücksetzen…Basis-ROM fehlt");
  });

  it("says_why_the_model_is_missing_when_it_cannot_be_loaded", async () => {
    vi.mocked(getBuildingGlb).mockRejectedValue(new Error("apicula fehlt"));
    await show();
    expect(container.textContent).toContain("Modell nicht geladen, die 3D-Ansicht zeigt eine Box");
  });

  it("shows_the_section_Technisch_and_the_Dez_Hex_switch_only_in_tuefteln", async () => {
    const technical = () => [...container.querySelectorAll<HTMLButtonElement>("button[aria-expanded]")].find((b) => b.textContent === "Technisch");
    const hexSwitch = () => [...container.querySelectorAll("button")].some((b) => b.textContent === "Hex");
    await show(context(), "bauen");
    expect(technical()).toBeUndefined();
    expect(hexSwitch()).toBe(false);
    await show(context(), "tuefteln");
    expect(technical()?.getAttribute("aria-expanded")).toBe("false");
    expect(hexSwitch()).toBe(true);
    await act(async () => void technical()?.click());
    expect(container.textContent).toContain("Karte 0012, Index 2");
  });
});
