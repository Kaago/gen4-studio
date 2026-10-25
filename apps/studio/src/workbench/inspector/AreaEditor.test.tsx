// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { getAreaData, setAreaData } from "../../api";
import type { AreaDataDto, AssetId } from "../../api";
import { openSelect, typeInto } from "../../ui/primitives/testUtils";
import { visibleWords } from "../../ui/visibleText";
import { AreaEditor } from "./AreaEditor";

vi.mock("../../api", async (importOriginal) => ({ ...(await importOriginal<typeof import("../../api")>()), getAreaData: vi.fn(), setAreaData: vi.fn() }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const flush = () => act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));
const area: AreaDataDto = { family: "Hgss", buildings_tileset: 1, map_tileset: 10, dynamic_texture_type: 0, area_type: 1, light_type: 4 };

describe("AreaEditor", () => {
  let container: HTMLElement;
  let root: Root;
  const onChanged = vi.fn();
  const onError = vi.fn();
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    vi.mocked(getAreaData).mockReset().mockResolvedValue(structuredClone(area));
    vi.mocked(setAreaData).mockReset().mockResolvedValue(undefined);
    onChanged.mockReset();
    onError.mockReset();
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });
  async function show(counts = {}, dirty: AssetId[] = [], depth: "bauen" | "tuefteln" = "bauen") {
    await act(async () => root.render(<AreaEditor id={7} context={{ counts, dirty, revision: 0, onChanged, onError }} depth={depth} />));
    await flush();
  }
  const input = (label: string) => container.querySelector<HTMLInputElement>(`input[aria-label='${label}']`)!;
  const select = (label: string) => container.querySelector<HTMLElement>(`[role=combobox][aria-label='${label}']`)!;

  it("names_the_record_and_shows_its_fields", async () => {
    await show();
    expect(container.querySelector("header[data-inspector-kind='look'] [data-eyebrow]")?.textContent).toBe("Aussehen");
    expect(container.querySelector("h2")?.textContent).toBe("Area-Data 0007");
    expect(input("Dynamische Textur").value).toBe("0");
    expect(input("Lichttyp").value).toBe("4");
    expect(select("Gebietstyp").textContent).toContain("Außen (1)");
  });

  it("says_no_raster_matrix_or_header_in_building", async () => {
    await show();
    expect(visibleWords(container)).not.toMatch(/Raster|Matrix|Header/);
    expect(container.textContent).toContain("aller Karten, deren Ort diese Area-Data verwendet");
  });

  it("keeps_the_technical_words_in_tinkering", async () => {
    await show({}, [], "tuefteln");
    expect(container.querySelector("header[data-inspector-kind='look'] [data-eyebrow]")?.textContent).toBe("Area-Data");
    expect(container.textContent).toContain("aller Karten, deren Header diese Area-Data verwendet");
  });

  it("says_Ungespeichert_while_this_area_data_has_unsaved_changes_and_not_for_another_one", async () => {
    await show({}, [{ kind: "AreaData", index: 8 }, { kind: "Matrix", index: 7 }]);
    expect(container.textContent).not.toContain("Ungespeichert");
    await show({}, [{ kind: "AreaData", index: 7 }]);
    expect(container.textContent).toContain("Ungespeichert");
  });

  it("writes_a_changed_area_type_chosen_in_the_select_as_one_undoable_edit", async () => {
    await show();
    openSelect(select("Gebietstyp"));
    await act(async () => void [...document.querySelectorAll<HTMLElement>("[role=option]")].find((o) => o.textContent === "Innen (0)")!.click());
    await flush();
    expect(setAreaData).toHaveBeenCalledWith(7, { ...area, area_type: 0 }, "Gebietstyp");
    expect(onChanged).toHaveBeenCalledTimes(1);
  });

  it("writes_a_typed_number_on_blur_and_refuses_one_beyond_the_byte", async () => {
    await show();
    await act(async () => void input("Lichttyp").focus());
    typeInto(input("Lichttyp"), "300");
    await act(async () => void input("Lichttyp").blur());
    expect(container.textContent).toContain("höchstens 255");
    expect(setAreaData).not.toHaveBeenCalled();
  });

  it("changes_a_tileset_through_the_picker_up_to_the_size_of_its_archive", async () => {
    await show({ MapTexture: 20 });
    await act(async () => void container.querySelector<HTMLElement>("button[aria-label='Karten-Tileset ändern…']")!.click());
    const options = [...document.querySelectorAll<HTMLElement>("[role=option]")];
    expect(options).toHaveLength(20);
    await act(async () => void options[3]!.click());
    await flush();
    expect(setAreaData).toHaveBeenCalledWith(7, { ...area, map_tileset: 3 }, "Karten-Tileset");
  });

  it("shows_the_section_Technisch_and_the_Dez_Hex_switch_only_in_tuefteln", async () => {
    const technical = () => [...container.querySelectorAll<HTMLButtonElement>("button[aria-expanded]")].find((b) => b.textContent === "Technisch");
    const hexSwitch = () => [...container.querySelectorAll("button")].some((b) => b.textContent === "Hex");
    await show({}, [], "bauen");
    expect(technical()).toBeUndefined();
    expect(hexSwitch()).toBe(false);
    await show({}, [], "tuefteln");
    expect(technical()?.getAttribute("aria-expanded")).toBe("false");
    expect(hexSwitch()).toBe(true);
    await act(async () => void technical()?.click());
    expect(container.textContent).toContain("Area-Data 0007");
  });
});
