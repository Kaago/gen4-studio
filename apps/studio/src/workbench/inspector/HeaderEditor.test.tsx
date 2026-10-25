// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { dataList, getEvents, getHeader, newScriptFile, readScript, setHeader } from "../../api";
import type { EventsDto, HeaderSummary, MapHeader, MapHeaderHGSS, MapHeaderPt } from "../../api";
import { typeInSearch, typeInto } from "../../ui/primitives/testUtils";
import { visibleWords } from "../../ui/visibleText";
import { HeaderEditor } from "./HeaderEditor";
import type { HeaderEditorContext } from "./HeaderEditor";

vi.mock("../../api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../../api")>()),
  getHeader: vi.fn(),
  setHeader: vi.fn(),
  getEvents: vi.fn(),
  newScriptFile: vi.fn(),
  readScript: vi.fn(),
  dataList: vi.fn(),
}));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const flush = () => act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));
const pt = (over: Partial<MapHeaderPt> = {}): MapHeader => ({
  Pt: {
    area_data_id: 1,
    unknown1: 0,
    matrix_id: 2,
    script_file_id: 40,
    level_script_id: 41,
    text_archive_id: 5,
    music_day_id: 6,
    music_night_id: 7,
    wild_pokemon: 8,
    event_file_id: 9,
    location_name: 10,
    area_icon: 11,
    weather_id: 12,
    camera_angle_id: 13,
    location_specifier: 14,
    battle_background: 15,
    flags: 1,
    ...over,
  },
});
const summary: HeaderSummary = { id: 411, internal_name: "T01", location_index: 0, kanto: false, location_name: "Zweiblattdorf", matrix_id: 2, area_data_id: 1, event_file_id: 9, text_archive_id: 5, script_file_id: 40, level_script_id: 41 };
const noEvents: EventsDto = { event_file_id: 9, locked: null, bg: [], objects: [], warps: [], coords: [] };

describe("HeaderEditor scripts", () => {
  let container: HTMLElement;
  let root: Root;
  const onOpenScript = vi.fn();
  const onChanged = vi.fn();
  const onCountsChanged = vi.fn();
  const onError = vi.fn();
  const onLevelScriptSaved = vi.fn();
  const context = (over: Partial<HeaderEditorContext> = {}): HeaderEditorContext => ({
    headers: [summary],
    counts: { Script: 800 },
    revision: 0,
    onChanged,
    goTo: vi.fn(),
    eventRevision: 0,
    onOpenWizard: vi.fn(),
    onAddWarp: vi.fn(),
    info: null,
    scriptsAdopted: true,
    scriptsRevision: 0,
    onOpenScript,
    onLevelScriptSaved,
    isCurrentProject: () => true,
    onCountsChanged,
    onError,
    ...over,
  });
  async function show(ctx = context(), depth: "bauen" | "tuefteln" = "bauen") {
    await act(async () => root.render(<HeaderEditor id={411} context={ctx} depth={depth} />));
    await flush();
  }
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    vi.mocked(getHeader).mockReset().mockResolvedValue(pt());
    vi.mocked(setHeader).mockReset().mockResolvedValue(undefined);
    vi.mocked(getEvents).mockReset().mockResolvedValue(noEvents);
    vi.mocked(newScriptFile).mockReset();
    vi.mocked(readScript).mockReset().mockResolvedValue({ kind: "levelScript", doc: { header_entries: [], var_conditions: [] } });
    for (const fn of [onOpenScript, onChanged, onCountsChanged, onError, onLevelScriptSaved]) fn.mockReset();
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });
  const button = (label: string) => [...container.querySelectorAll("button")].find((b) => b.getAttribute("aria-label") === label || b.textContent === label) as HTMLButtonElement | undefined;
  const click = (el: Element | undefined) => act(async () => void (el as HTMLElement).click());

  it("limits_both_pickers_by_the_member_count_of_the_script_archive", async () => {
    await show();
    const row = (label: string) => [...container.querySelectorAll<HTMLElement>("div")].find((d) => d.firstElementChild?.textContent === label && d.children.length >= 2)!;
    expect(row("Scriptdatei").textContent).toContain("0 bis 799");
    expect(row("Level-Script").textContent).toContain("0 bis 799");
    // A picker whose archive size is not known yet is a plain number field.
    expect(container.querySelector("input[aria-label='Weltkarte']")).not.toBeNull();
  });

  it("shows_the_form_in_the_sections_Karten_Aussehen_Texte_und_Scripts_Ausgänge_and_Level_Script", async () => {
    await show();
    expect([...container.querySelectorAll("h3")].map((h) => h.textContent)).toEqual(["Karten", "Aussehen", "Texte und Scripts", "Eingänge / Ausgänge0", "Level-Script"]);
    expect(container.querySelector("h2")?.textContent).toBe("Zweiblattdorf");
    expect(container.textContent).toContain("0411 · T01");
  });

  it("opens_the_script_file_and_the_level_script_of_the_header", async () => {
    await show();
    // The references read "Scriptdatei" and the number.
    await click(button("Scriptdatei0040"));
    expect(onOpenScript).toHaveBeenLastCalledWith(40);
    await click(button("Level-Script0041"));
    expect(onOpenScript).toHaveBeenLastCalledWith(41);
  });

  it("has_nothing_to_open_for_a_header_without_a_level_script", async () => {
    vi.mocked(getHeader).mockResolvedValue(pt({ level_script_id: 0 }));
    await show();
    expect(button("Level-Scriptkeins")!.disabled).toBe(true);
    expect(button("Scriptdatei0040")!.disabled).toBe(false);
  });

  it("writes_the_number_of_a_new_script_file_into_the_header_and_opens_the_file", async () => {
    vi.mocked(newScriptFile).mockResolvedValue(812);
    await show();
    await click(button("Neue Scriptdatei anlegen"));
    await flush();
    expect(newScriptFile).toHaveBeenCalledWith("script");
    // Through the edit path (an undoable header write with the field's name), then list, scene and counts follow.
    expect(setHeader).toHaveBeenCalledTimes(1);
    expect(setHeader).toHaveBeenCalledWith(411, pt({ script_file_id: 812 }), "Scriptdatei");
    expect(onChanged).toHaveBeenCalledTimes(1);
    expect(onCountsChanged).toHaveBeenCalledTimes(1);
    expect(onOpenScript).toHaveBeenCalledWith(812);
  });

  it("neither_opens_the_new_file_nor_refreshes_the_counts_when_the_header_write_was_refused", async () => {
    vi.mocked(newScriptFile).mockResolvedValue(812);
    vi.mocked(setHeader).mockRejectedValue({ code: "error", message: "Wert passt nicht", asset: null });
    await show();
    await click(button("Neue Scriptdatei anlegen"));
    await flush();
    expect(setHeader).toHaveBeenCalledTimes(1);
    expect(onError).toHaveBeenCalledWith({ code: "error", message: "Wert passt nicht", asset: null });
    expect(onOpenScript).not.toHaveBeenCalled();
    expect(onCountsChanged).not.toHaveBeenCalled();
  });

  it("writes_the_number_of_a_new_level_script_into_the_header_without_opening_it", async () => {
    vi.mocked(newScriptFile).mockResolvedValue(813);
    await show();
    await click(button("Neues Level-Script anlegen"));
    await flush();
    expect(newScriptFile).toHaveBeenCalledWith("levelScript");
    expect(setHeader).toHaveBeenCalledWith(411, pt({ level_script_id: 813 }), "Level-Script");
    expect(onOpenScript).not.toHaveBeenCalled();
  });

  it("creates_the_level_script_from_the_form_when_the_header_has_none", async () => {
    vi.mocked(getHeader).mockResolvedValue(pt({ level_script_id: 0 }));
    vi.mocked(newScriptFile).mockResolvedValue(30);
    await show();
    expect(container.textContent).toContain("Kein Level-Script — Anlegen");
    await click([...container.querySelectorAll("section button")].find((b) => b.textContent === "Anlegen"));
    await flush();
    expect(newScriptFile).toHaveBeenCalledWith("levelScript");
    expect(setHeader).toHaveBeenCalledWith(411, pt({ level_script_id: 30 }), "Level-Script");
    // The header now names the file: the form reads it.
    expect(readScript).toHaveBeenCalledWith(30);
  });

  it("reports_a_refused_new_file_and_writes_nothing", async () => {
    vi.mocked(newScriptFile).mockRejectedValue({ code: "error", message: "Textarchiv 0199 erst speichern", asset: null });
    await show();
    await click(button("Neue Scriptdatei anlegen"));
    await flush();
    expect(onError).toHaveBeenCalledWith({ code: "error", message: "Textarchiv 0199 erst speichern", asset: null });
    expect(setHeader).not.toHaveBeenCalled();
  });

  it("shows_the_form_of_the_level_script", async () => {
    vi.mocked(readScript).mockResolvedValue({ kind: "levelScript", doc: { header_entries: [{ type: "on_load", script_id: 3 }], var_conditions: [] } });
    await show();
    expect(readScript).toHaveBeenCalledWith(41);
    expect(container.querySelector('table[aria-label="Kopfeinträge"]')).not.toBeNull();
  });

  it("asks_for_the_adoption_and_calls_no_command_while_scripts_are_not_adopted", async () => {
    await show(context({ scriptsAdopted: false }));
    expect(container.textContent).toContain("Erst Scripts übernehmen (Panel „Script“)");
    for (const label of ["Scriptdatei0040", "Level-Script0041", "Neue Scriptdatei anlegen", "Neues Level-Script anlegen"]) {
      expect(button(label)!.disabled).toBe(true);
      await click(button(label));
    }
    expect(onOpenScript).not.toHaveBeenCalled();
    expect(newScriptFile).not.toHaveBeenCalled();
    expect(readScript).not.toHaveBeenCalled();
  });

  describe("fields", () => {
    const field = (label: string) => container.querySelector<HTMLInputElement>(`input[aria-label='${label}']`)!;
    const sectionHead = (title: string) => [...container.querySelectorAll<HTMLElement>("h3 button")].find((b) => b.textContent === title)!;
    const options = () => [...document.querySelectorAll<HTMLElement>("[role=option]")];

    it("writes_a_typed_number_on_blur_as_one_undoable_header_edit", async () => {
      await show();
      await act(async () => void field("Wetter").focus());
      typeInto(field("Wetter"), "20");
      expect(setHeader).not.toHaveBeenCalled();
      await act(async () => void field("Wetter").blur());
      await flush();
      expect(setHeader).toHaveBeenCalledWith(411, pt({ weather_id: 20 }), "Wetter");
      expect(onChanged).toHaveBeenCalledTimes(1);
    });

    it("refuses_a_number_the_field_cannot_hold_and_writes_nothing", async () => {
      await show();
      await act(async () => void field("Wetter").focus());
      typeInto(field("Wetter"), "300");
      await act(async () => void field("Wetter").blur());
      await flush();
      expect(container.textContent).toContain("höchstens 255");
      expect(setHeader).not.toHaveBeenCalled();
    });

    it("writes_a_number_that_is_still_being_typed_when_its_section_is_folded", async () => {
      await show();
      await act(async () => void field("Wetter").focus());
      typeInto(field("Wetter"), "21");
      // The field goes away without a blur; the draft is written, not lost.
      await click(sectionHead("Aussehen"));
      await flush();
      expect(field("Wetter")).toBeNull();
      expect(setHeader).toHaveBeenCalledWith(411, pt({ weather_id: 21 }), "Wetter");
      await click(sectionHead("Aussehen"));
      expect(field("Wetter").value).toBe("21");
    });

    it("keeps_a_folded_section_folded_when_the_editor_is_replaced_and_mounted_again_as_a_hover_does", async () => {
      await show();
      await click(sectionHead("Aussehen"));
      expect(field("Wetter")).toBeNull();
      await act(async () => root.render(null));
      await show();
      await flush();
      expect(field("Wetter")).toBeNull();
      await click(sectionHead("Aussehen"));
      expect(field("Wetter")).not.toBeNull();
    });

    it("leads_the_matrix_and_the_area_data_to_their_targets_and_changes_them_through_the_picker", async () => {
      const goTo = vi.fn();
      await show(context({ counts: { Script: 800, Matrix: 10, AreaData: 5 }, goTo }));
      await click(button("Weltkarte0002"));
      expect(goTo).toHaveBeenLastCalledWith({ kind: "matrixCell", matrix: 2, x: 0, y: 0 });
      await click(button("Area-Data0001"));
      expect(goTo).toHaveBeenLastCalledWith({ kind: "area", id: 1 });
      await click(button("Weltkarte ändern…"));
      typeInSearch(document.querySelector<HTMLInputElement>("input[role=combobox]")!, "0005");
      expect(options().map((o) => o.textContent)).toEqual(["Weltkarte0005"]);
      await click(options()[0]);
      await flush();
      expect(setHeader).toHaveBeenCalledWith(411, pt({ matrix_id: 5 }), "Weltkarte");
    });
  });

  describe("encounter file", () => {
    const options = () => [...document.querySelectorAll<HTMLElement>("[role=dialog] [role=option]")];
    const files = [
      { index: 8, name: "", tech: "0008", locked: null, extra: { users: [411, 412] } },
      { index: 9, name: "", tech: "0009", locked: null, extra: { users: [] } },
    ];
    const headers: HeaderSummary[] = [summary, { ...summary, id: 412, internal_name: "R201", location_name: "Route 201" }];
    beforeEach(() => vi.mocked(dataList).mockReset().mockResolvedValue(files));

    it("shows_the_file_as_a_reference_into_the_pokemon_area", async () => {
      const goTo = vi.fn();
      await show(context({ goTo, headers }));
      // Nothing is read before the picker opens.
      expect(dataList).not.toHaveBeenCalled();
      await click(button("Begegnungen0008"));
      expect(goTo).toHaveBeenLastCalledWith({ kind: "encounter", index: 8 });
    });

    it("picks_another_file_by_its_places_as_one_undoable_header_edit", async () => {
      await show(context({ headers }));
      await click(button("Begegnungen ändern…"));
      await flush();
      expect(dataList).toHaveBeenCalledWith("encounter");
      expect(options().map((o) => o.textContent)).toEqual(["Keine", expect.stringContaining("Zweiblattdorf, Route 201"), expect.stringContaining("Begegnungen 0009")]);
      expect(options()[1]?.textContent).toContain("0008");
      await click(options()[2]);
      await flush();
      expect(setHeader).toHaveBeenCalledWith(411, pt({ wild_pokemon: 9 }), "Begegnungen");
      expect(onChanged).toHaveBeenCalledTimes(1);
    });

    it("writes_0xFFFF_for_keine_in_platinum_and_0xFF_in_heartgold", async () => {
      await show(context({ headers }));
      await click(button("Begegnungen ändern…"));
      await flush();
      await click(options()[0]);
      await flush();
      expect(setHeader).toHaveBeenLastCalledWith(411, pt({ wild_pokemon: 0xffff }), "Begegnungen");
      const fields: MapHeaderHGSS = {
        wild_pokemon: 8,
        area_data_id: 1,
        unknown0: 0,
        worldmap_x: 0,
        worldmap_y: 0,
        matrix_id: 2,
        script_file_id: 40,
        level_script_id: 41,
        text_archive_id: 5,
        music_day_id: 6,
        music_night_id: 7,
        event_file_id: 9,
        location_name: 10,
        area_icon: 11,
        unknown1: 0,
        kanto_flag: false,
        weather_id: 12,
        location_type: 0,
        camera_angle_id: 13,
        follow_mode: 0,
        battle_background: 15,
        flags: 1,
      };
      const hg: MapHeader = { HGSS: fields };
      vi.mocked(getHeader).mockResolvedValue(hg);
      await act(async () => root.render(null));
      await show(context({ headers }));
      await click(button("Begegnungen ändern…"));
      await flush();
      await click(options()[0]);
      await flush();
      expect(setHeader).toHaveBeenLastCalledWith(411, { HGSS: { ...fields, wild_pokemon: 0xff } }, "Begegnungen");
    });

    it("says_keine_for_a_header_without_a_file", async () => {
      vi.mocked(getHeader).mockResolvedValue(pt({ wild_pokemon: 0xffff }));
      await show(context({ headers }));
      expect(button("Begegnungen0008")).toBeUndefined();
      expect(container.textContent).toContain("Keine");
    });
  });

  it("says_no_raster_matrix_or_header_in_building", async () => {
    vi.mocked(getEvents).mockResolvedValue({ ...noEvents, warps: [{ index: 0, x: 1, z: 2, dest_header_id: 411, dest_warp_id: 0, height: 0 }] });
    await show();
    expect(visibleWords(container)).not.toMatch(/Raster|Matrix|Header/);
    expect(container.textContent).toContain("Neue Gebiete und Räume legen die Assistenten „Gebiet anlegen“ und „Raum anlegen“ an.");
    // While the header is read, and for a header the list does not know.
    vi.mocked(getHeader).mockReturnValue(new Promise(() => {}));
    await show(context({ headers: [] }));
    expect(visibleWords(container)).not.toMatch(/Raster|Matrix|Header/);
    expect(container.querySelector("h2")?.textContent).toBe("Ort 0411");
  });

  it("keeps_the_technical_words_in_tinkering", async () => {
    await show(context(), "tuefteln");
    expect([...container.querySelectorAll("h3")].map((h) => h.textContent)).toContain("Karten und Raster");
    expect(container.querySelector("input[aria-label='Matrix']")).not.toBeNull();
    expect(container.textContent).toContain("(Schritt „Header“, Option „Neuer Header“)");
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
    expect(container.textContent).toContain("Header 0411, Eintrag der Header-Tabelle");
  });
});
