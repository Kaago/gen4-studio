// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { assignPlace, dataList, entityIds, getEvents, getHeader, getMatrix, getTextArchive, nameTables, newScriptFile, placeSlots, readScript, setHeader, splitRoom } from "../../api";
import type { EventsDto, HeaderSummary, MapHeader, MapHeaderHGSS, MapHeaderPt, MatrixSummary, NameTablesDto, ProjectInfo } from "../../api";
import { typeInSearch, typeInto } from "../../ui/primitives/testUtils";
import { visibleWords } from "../../ui/visibleText";
import { headerIdentity } from "../headerIdentity";
import { buildPlaces } from "../places";
import type { World } from "../worldModel";
import { HeaderEditor } from "./HeaderEditor";
import type { HeaderEditorContext } from "./HeaderEditor";
import { chooseAction, headAction, headActions } from "./headTesting";
import { MISSING, isDisabled, resetSpy, withReset } from "../../original/resetTesting";
import { textProjectSpy, withTextProject } from "../../textproject/textProjectTesting";
import type { Depth } from "../../ui/frame/shellState";

vi.mock("../../api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../../api")>()),
  getHeader: vi.fn(),
  setHeader: vi.fn(),
  getEvents: vi.fn(),
  newScriptFile: vi.fn(),
  readScript: vi.fn(),
  dataList: vi.fn(),
  placeSlots: vi.fn(),
  getTextArchive: vi.fn(),
  assignPlace: vi.fn(),
  splitRoom: vi.fn(),
  entityIds: vi.fn(),
  getMatrix: vi.fn(),
  nameTables: vi.fn(),
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
// Matrix 2 has a header section: header 411 is outside, an area. Matrix 3 is the one-cell map of a house.
const matrices: MatrixSummary[] = [
  { index: 2, name: "m2", width: 30, height: 30, has_headers: true, has_altitudes: true, filled: 600 },
  { index: 3, name: "m3", width: 1, height: 1, has_headers: false, has_altitudes: false, filled: 1 },
  { index: 4, name: "m4", width: 2, height: 2, has_headers: false, has_altitudes: false, filled: 3 },
];
const house: HeaderSummary = { ...summary, id: 412, internal_name: "T01R0101", matrix_id: 3 };
/** The world with the house as a room that shares its map with `shared` other rooms. */
const houseWorld = (shared: number, matrix_id = 3): World => ({
  worldMaps: [],
  moreAreas: [],
  unconnected: [{ header: { ...house, matrix_id }, name: "Haus 1", suggested: "Haus 1", placeKey: "Zweiblattdorf", placeName: "Zweiblattdorf", manual: false, shared }],
  other: [],
  areaOf: new Map(),
  roomCount: 1,
  assignedRooms: 0,
});

describe("HeaderEditor scripts", () => {
  let container: HTMLElement;
  let root: Root;
  const onOpenScript = vi.fn();
  const onChanged = vi.fn();
  const onPlaceChanged = vi.fn();
  const onCountsChanged = vi.fn();
  const onError = vi.fn();
  const onLevelScriptSaved = vi.fn();
  const onRoomSplit = vi.fn();
  let reset = resetSpy();
  const context = (over: Partial<HeaderEditorContext> = {}): HeaderEditorContext => ({
    headers: [summary],
    identify: (h) => headerIdentity(buildPlaces([summary, house], matrices), h),
    counts: { Script: 800 },
    revision: 0,
    onChanged,
    onPlaceChanged,
    goTo: vi.fn(),
    eventRevision: 0,
    onOpenWizard: vi.fn(),
    info: null,
    scriptsAdopted: true,
    scriptsRevision: 0,
    onOpenScript,
    onLevelScriptSaved,
    isCurrentProject: () => true,
    onCountsChanged,
    onError,
    world: null,
    matrices,
    onRoomSplit,
    ...over,
  });
  async function show(ctx = context(), depth: Depth = "bauen") {
    await act(async () => root.render(withReset(reset, <HeaderEditor id={411} context={ctx} depth={depth} />)));
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
    vi.mocked(placeSlots).mockReset().mockResolvedValue({ bank: 433, count: 3, free: 253, reusable: [] });
    vi.mocked(getTextArchive)
      .mockReset()
      .mockResolvedValue({ index: 433, key: 0, lang: "de", messages: ["Mysteriöser Ort", "Zweiblattdorf", "Sandgemme"].map((text, i) => ({ id: `msg_${i}`, text })), users: [] });
    vi.mocked(assignPlace).mockReset().mockResolvedValue(undefined);
    for (const fn of [onOpenScript, onChanged, onPlaceChanged, onCountsChanged, onError, onLevelScriptSaved, onRoomSplit]) fn.mockReset();
    vi.mocked(splitRoom).mockReset().mockResolvedValue({ matrix: 700, maps: [601] });
    vi.mocked(getMatrix).mockReset().mockRejectedValue({ code: "error", message: "keine Matrix im Test", asset: null });
    reset = resetSpy();
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

  it("shows_the_form_in_the_sections_Karten_Aussehen_Texte_und_Scripts_and_Level_Script", async () => {
    await show();
    expect([...container.querySelectorAll("h3")].map((h) => h.textContent)).toEqual(["Karten", "Aussehen", "Texte und Scripts", "Level-Script"]);
    expect(container.querySelector("h2")?.textContent).toBe("Zweiblattdorf");
  });

  it("calls_an_area_and_a_room_by_their_names_and_the_header_by_its_number_in_tinkering", async () => {
    const eyebrow = () => container.querySelector("header[data-inspector-kind] [data-eyebrow]")?.textContent;
    const kind = () => container.querySelector("header[data-inspector-kind]")?.getAttribute("data-inspector-kind");
    const ident = () => container.querySelector("[data-ident]")?.textContent;
    await show();
    expect([kind(), eyebrow(), container.querySelector("h2")?.textContent, ident()]).toEqual(["area", "Gebiet", "Zweiblattdorf", "Zweiblattdorf · T01"]);
    await show(context(), "tuefteln");
    expect([kind(), eyebrow(), container.querySelector("h2")?.textContent, ident()]).toEqual(["area", "Header", "Zweiblattdorf", "Header 0411 · T01"]);
    // A room has the clear name its code suggests, not "Raum" and its number.
    await act(async () => root.render(withReset(reset, <HeaderEditor id={412} context={context({ headers: [summary, house] })} depth="bauen" />)));
    await flush();
    expect([kind(), eyebrow(), container.querySelector("h2")?.textContent, ident()]).toEqual(["room", "Raum", "Haus 1", "Zweiblattdorf · T01R0101"]);
    expect(visibleWords(container)).not.toMatch(/Raster|Matrix|Header/);
    await act(async () => root.render(withReset(reset, <HeaderEditor id={412} context={context({ headers: [summary, house] })} depth="tuefteln" />)));
    await flush();
    expect([kind(), eyebrow(), container.querySelector("h2")?.textContent, ident()]).toEqual(["room", "Header", "Haus 1", "Header 0412 · T01R0101"]);
  });

  it("calls_the_header_what_the_world_tree_calls_it", async () => {
    const eyebrow = () => container.querySelector("header[data-inspector-kind] [data-eyebrow]")?.textContent;
    // A room renamed in the project file, and a header the world tree lists under „Sonstige“.
    const names = { 412: { kind: "room", name: "Omas Haus", place: "Zweiblattdorf" }, 411: { kind: "other", name: "Zweiblattdorf", place: "Zweiblattdorf" } } as const;
    const identify = (h: HeaderSummary) => names[h.id as 411 | 412];
    await act(async () => root.render(withReset(reset, <HeaderEditor id={412} context={context({ headers: [summary, house], identify })} depth="bauen" />)));
    await flush();
    expect([eyebrow(), container.querySelector("h2")?.textContent]).toEqual(["Raum", "Omas Haus"]);
    await show(context({ identify }));
    expect(container.querySelector("header[data-inspector-kind]")?.getAttribute("data-inspector-kind")).toBe("place");
    expect([eyebrow(), container.querySelector("h2")?.textContent]).toEqual(["Ort", "Zweiblattdorf"]);
    await show(context({ identify }), "tuefteln");
    expect(eyebrow()).toBe("Header");
  });

  it("has_no_exits_list_but_a_line_that_leads_to_the_doors", async () => {
    const goTo = vi.fn();
    const warp = (index: number, dest: number) => ({ index, x: index, z: 2, dest_header_id: dest, dest_warp_id: 0, height: 0 });
    // A door into the house and one whose target the headers do not list.
    vi.mocked(getEvents).mockResolvedValue({ ...noEvents, warps: [warp(0, 412), warp(1, 999)] });
    await show(context({ goTo, headers: [summary, house] }));
    expect(getEvents).toHaveBeenCalledWith(9);
    // No list of the exits, no „Warp hinzufügen“, no hint at the wizards (the „+“ of the places has them).
    expect([...container.querySelectorAll("h3")].map((h) => h.textContent)).not.toContain("Eingänge / Ausgänge2");
    expect(button("Warp hinzufügen")).toBeUndefined();
    expect(button("Bearbeiten")).toBeUndefined();
    expect(container.textContent).not.toContain("Assistenten");
    // The line leads to the doors of this header's scene.
    await click(button("2 Türen, 1 ohne Ziel"));
    expect(goTo).toHaveBeenCalledWith({ kind: "sceneGroup", group: "warp", header: 411 });
    // Counted again when the events moved; one door says so, none is no link.
    vi.mocked(getEvents).mockResolvedValue({ ...noEvents, warps: [warp(0, 412)] });
    await show(context({ goTo, headers: [summary, house], eventRevision: 1 }));
    expect(button("1 Tür")).toBeDefined();
    vi.mocked(getEvents).mockResolvedValue(noEvents);
    await show(context({ goTo, headers: [summary, house], eventRevision: 2 }));
    expect(container.querySelector("[data-doors]")?.textContent).toBe("Keine Türen");
    expect(container.querySelector("[data-doors] button")).toBeNull();
    expect(visibleWords(container)).not.toMatch(/Raster|Matrix|Header/);
  });

  it("binds_a_room_here_from_the_head_menu", async () => {
    const onOpenWizard = vi.fn();
    await show(context({ onOpenWizard }));
    expect(headActions(container)).toEqual(["Raum hier anbinden…", "Ort umbenennen…", "Auf Original zurücksetzen…"]);
    chooseAction(container, "Raum hier anbinden…");
    expect(onOpenWizard).toHaveBeenCalledWith({ kind: "room", prefill: { outsideHeader: 411 } });
  });

  it("text_project_shows_the_id_and_offers_rename", async () => {
    vi.mocked(entityIds).mockReset().mockResolvedValue({ 411: "MAP_T01" });
    const text = textProjectSpy(true);
    await act(async () => root.render(withReset(reset, withTextProject(text, <HeaderEditor id={411} context={context()} depth="bauen" />))));
    await flush();
    expect(entityIds).toHaveBeenCalledWith("Map");
    expect(container.querySelector("header[data-inspector-kind] [data-entity-id]")?.textContent).toBe("MAP_T01");
    expect(container.querySelector('header[data-inspector-kind] button[aria-label="ID kopieren"]')).not.toBeNull();
    expect(headActions(container)).toEqual(["Raum hier anbinden…", "Ort umbenennen…", "Auf Original zurücksetzen…", "ID umbenennen…"]);
    chooseAction(container, "ID umbenennen…");
    const dialog = document.querySelector<HTMLElement>("[role=dialog]");
    expect(dialog?.querySelector("h2")?.textContent).toBe("ID umbenennen");
    expect(dialog?.querySelector<HTMLInputElement>('input[aria-label="Neue ID"]')?.value).toBe("MAP_T01");
  });

  it("dspre_layout_shows_neither", async () => {
    vi.mocked(entityIds).mockReset().mockResolvedValue({});
    await act(async () => root.render(withReset(reset, withTextProject(textProjectSpy(false), <HeaderEditor id={411} context={context()} depth="bauen" />))));
    await flush();
    expect(entityIds).not.toHaveBeenCalled();
    expect(container.querySelector("[data-entity-id]")).toBeNull();
    expect(headActions(container)).toEqual(["Raum hier anbinden…", "Ort umbenennen…", "Auf Original zurücksetzen…"]);
  });

  it("head_offers_reset_of_the_header_and_locks_a_new_one", async () => {
    await show();
    // The header with its fields (the dialog lists the fields that differ from the base).
    chooseAction(container, "Auf Original zurücksetzen…");
    expect(reset.open).toHaveBeenCalledWith({ kind: "header", id: 411 });
    // A header from the count of the base on is new: there is no original to go back to.
    await act(async () => root.render(null));
    await act(async () => root.render(withReset(reset, <HeaderEditor id={500} context={context({ headers: [summary, { ...summary, id: 500 }] })} depth="bauen" />)));
    await flush();
    const item = headAction(container, "Auf Original zurücksetzen…");
    expect(isDisabled(item)).toBe(true);
    expect(item?.textContent).toBe("Auf Original zurücksetzen…Kein Original");
    // Without a base ROM the reason is the missing base, also for a header the head has no other actions for.
    await act(async () => root.render(null));
    reset = resetSpy(MISSING);
    await act(async () => root.render(withReset(reset, <HeaderEditor id={411} context={context({ headers: [] })} depth="bauen" />)));
    await flush();
    expect(headActions(container)).toEqual(["Auf Original zurücksetzen…Basis-ROM fehlt"]);
  });

  describe("a shared room", () => {
    const showHouse = async (world: World | null, depth: "bauen" | "tuefteln" = "bauen") => {
      await act(async () => root.render(withReset(reset, <HeaderEditor id={412} context={context({ headers: [summary, house], world })} depth={depth} />)));
      await flush();
    };
    const badge = () => container.querySelector<HTMLElement>("header[data-inspector-kind] [data-shared]");

    it("shows_the_shared_badge_with_the_count", async () => {
      await showHouse(houseWorld(17));
      expect(badge()?.textContent).toBe("Geteilt mit 17 Räumen");
      vi.useFakeTimers();
      try {
        await act(async () => {
          const b = badge()!;
          b.dispatchEvent(new MouseEvent("mouseover", { bubbles: true }));
          b.dispatchEvent(new MouseEvent("mouseenter"));
          b.dispatchEvent(new MouseEvent("mousemove", { bubbles: true }));
          await vi.advanceTimersByTimeAsync(500);
        });
      } finally {
        vi.useRealTimers();
      }
      expect(document.querySelector("[role=tooltip]")?.textContent).toBe("Diese Karten gehören auch anderen Räumen. Was du hier änderst, ändert sie alle.");
      await showHouse(houseWorld(1), "tuefteln");
      expect(badge()?.textContent).toBe("Geteilt mit 1 Raum");
      // A room of its own, and the world not read yet: no badge.
      await showHouse(houseWorld(0));
      expect(badge()).toBeNull();
      await showHouse(null);
      expect(badge()).toBeNull();
    });

    it("offers_split_only_for_a_shared_room", async () => {
      await showHouse(houseWorld(0));
      expect(headActions(container)).toEqual(["Raum hier anbinden…", "Ort umbenennen…", "Auf Original zurücksetzen…"]);
      // An area is no room, whatever shares its world map.
      await show(context({ world: houseWorld(3) }));
      expect(headActions(container)).not.toContain("Für diesen Raum abspalten…");
      // Another header: a fresh editor, as the inspector mounts one per header (`key={id}`).
      await act(async () => root.render(null));
      await showHouse(houseWorld(3));
      expect(headActions(container)).toEqual(["Raum hier anbinden…", "Ort umbenennen…", "Für diesen Raum abspalten…", "Auf Original zurücksetzen…"]);
      chooseAction(container, "Für diesen Raum abspalten…");
      await flush();
      const dialog = () => document.querySelector<HTMLElement>("[role=dialog]");
      // Matrix 3 holds the house's one map.
      expect(dialog()?.textContent).toContain("Der Raum „Haus 1“ bekommt eine eigene Kopie seiner Karte.");
      const split = [...dialog()!.querySelectorAll("button")].find((b) => b.textContent === "Abspalten")!;
      await act(async () => split.click());
      await flush();
      expect(splitRoom).toHaveBeenCalledWith(412, "Haus 1");
      // The project is read again as after a wizard, the room shown with the message.
      expect(onRoomSplit).toHaveBeenCalledWith({ matrix: 700, header: 412, announcement: "Raum „Haus 1“ abgespalten" });
      expect(dialog()).toBeNull();
    });
  });

  describe("splitting by the matrix", () => {
    const showHouse = async (world: World) => {
      await act(async () => root.render(withReset(reset, <HeaderEditor id={412} context={context({ headers: [summary, house], world })} depth="bauen" />)));
      await flush();
    };
    const dialog = () => document.querySelector<HTMLElement>("[role=dialog]");

    it("offers_split_only_on_a_matrix_without_a_header_section", async () => {
      // A matrix with a header section (a world map), and a matrix the project does not list: the badge stays, the action goes.
      for (const matrixId of [2, 99]) {
        await act(async () => root.render(null));
        await showHouse(houseWorld(3, matrixId));
        expect(container.querySelector("header[data-inspector-kind] [data-shared]")?.textContent).toBe("Geteilt mit 3 Räumen");
        expect(headActions(container)).toEqual(["Raum hier anbinden…", "Ort umbenennen…", "Auf Original zurücksetzen…"]);
      }
    });

    it("names_no_number_of_maps_when_the_matrix_has_several_cells", async () => {
      await showHouse(houseWorld(3, 4));
      chooseAction(container, "Für diesen Raum abspalten…");
      await flush();
      expect(dialog()?.textContent).toContain("Der Raum „Haus 1“ bekommt eigene Kopien seiner Karten. Danach");
      expect(dialog()?.textContent).not.toMatch(/\d Karten|0 Karten/);
    });
  });

  describe("place", () => {
    const options = () => [...document.querySelectorAll<HTMLElement>("[role=dialog] [role=option]")];
    const placed: HeaderSummary = { ...summary, location_index: 1 };

    it("chooses_the_place_by_name_instead_of_the_number", async () => {
      await show(context({ headers: [placed] }));
      expect(container.querySelector("input[aria-label='Ortsname']")).toBeNull();
      await click(container.querySelector("button[data-place-picker]")!);
      await flush();
      await click(options()[2]);
      await flush();
      expect(assignPlace).toHaveBeenCalledWith([411], 2);
      expect(onPlaceChanged).toHaveBeenCalledWith(null);
      expect(setHeader).not.toHaveBeenCalled();
    });

    it("writes_the_field_on_diamond_and_pearl", async () => {
      const { area_icon: _icon, ...fields } = (pt() as { Pt: MapHeaderPt }).Pt;
      vi.mocked(getHeader).mockResolvedValue({ DP: fields });
      await show(context({ headers: [placed] }));
      await click(container.querySelector("button[data-place-picker]")!);
      await flush();
      await click(options()[2]);
      await flush();
      expect(assignPlace).not.toHaveBeenCalled();
      expect(setHeader).toHaveBeenCalledWith(411, { DP: { ...fields, location_name: 2 } }, "Ortsname");
      // No place commands there: renaming is locked.
      expect(headAction(container, "Ort umbenennen…")?.getAttribute("aria-disabled")).toBe("true");
    });

    it("renames_the_place_from_the_head_menu_but_not_no_place", async () => {
      await show(context({ headers: [placed, { ...house, location_index: 1 }] }));
      chooseAction(container, "Ort umbenennen…");
      await flush();
      const dialog = document.querySelector("[data-rename-place]")?.closest("[role=dialog]");
      expect(dialog?.textContent).toContain("Ort umbenennen");
      expect(dialog?.textContent).toContain("Trifft alle 2 Gebiete und Räume mit diesem Ort und den Fundort gefangener Pokémon.");
      await act(async () => root.render(null));
      await show(context({ headers: [summary] }));
      expect(headAction(container, "Ort umbenennen…")?.getAttribute("aria-disabled")).toBe("true");
    });
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
  describe("in Erzählen", () => {
    const titles = () => [...container.querySelectorAll("h3")].map((h) => h.textContent);
    const input = (label: string) => container.querySelector<HTMLInputElement>(`input[aria-label='${label}']`);

    it("erzaehlen_shows_one_section_ort_with_place_look_and_music", async () => {
      await show(context(), "erzaehlen");
      expect(titles()).toEqual(["Ort"]);
      expect(container.textContent).not.toContain("Karten");
      expect(container.querySelector("h2")?.textContent).toBe("Zweiblattdorf");
      // The look by its places, chosen with the header itself as the template.
      expect(container.querySelector("button[data-look-picker]")?.textContent).toBe("Aussehen wie Zweiblattdorf");
      // Music as a picker of the pieces, without a number field and without Dez | Hex.
      expect(button("Musik (Tag) wählen…")).toBeDefined();
      expect(button("Musik (Nacht) wählen…")).toBeDefined();
      expect(input("Musik (Tag)")).toBeNull();
      expect(button("Hex")).toBeUndefined();
      // No script file, no level script, no matrix.
      expect(input("Weltkarte")).toBeNull();
      expect(container.textContent).not.toContain("Scriptdatei");
      // The name of an area stands in its row without a button: there is no rename for areas.
      expect(container.querySelector("[data-name-row]")?.textContent).toContain("Zweiblattdorf");
      expect(button("Umbenennen…")).toBeUndefined();
    });

    it("in erzaehlen music is a picker, in bauen a number with the name", async () => {
      const sounds = [
        { id: 6, name: "SEQ_TOWN01", used_in: "Zweiblattdorf" },
        { id: 7, name: "SEQ_TOWN02", used_in: null },
        { id: 8, name: "SEQ_ROAD_A_D", used_in: "Route 201 (Tag)" },
      ];
      vi.mocked(nameTables).mockReset().mockResolvedValue({ sounds } as unknown as NameTablesDto);
      const info: ProjectInfo = { root: "/projekt-musik", game_code: "CPUD", family: "Pt", language: "de", header_count: 593 };
      await show(context({ info }), "erzaehlen");
      // Erzählen: the piece by its name and where it plays, chosen in a picker; no number.
      const day = container.querySelector<HTMLElement>("[data-music-field='music_day_id']")!;
      expect(day.textContent).toContain("SEQ_TOWN01");
      expect(day.textContent).toContain("Zweiblattdorf");
      expect(container.querySelector("[data-music-field='music_night_id']")?.textContent).toContain("SEQ_TOWN02");
      expect(input("Musik (Tag)")).toBeNull();
      await click(button("Musik (Tag) wählen…"));
      const option = [...document.querySelectorAll<HTMLElement>("[role=dialog] [role=option]")].find((o) => o.textContent?.includes("SEQ_ROAD_A_D"));
      expect(option?.textContent).toContain("Route 201 (Tag)");
      await click(option);
      await flush();
      expect(setHeader).toHaveBeenCalledWith(411, pt({ music_day_id: 8 }), "Musik (Tag)");
      // Bauen (a new editor, as for another header): the number field, with the piece's name beside it.
      act(() => root.unmount());
      root = createRoot(container);
      await show(context({ info }), "bauen");
      expect(input("Musik Tag")?.value).toBe("6");
      expect(input("Musik Nacht")?.value).toBe("7");
      const names = [...container.querySelectorAll("[data-music-name]")].map((n) => n.textContent);
      expect(names).toEqual(["SEQ_TOWN01", "SEQ_TOWN02"]);
      expect(button("Musik Tag wählen…")).toBeUndefined();
      // The names are read once per project, not for every header shown.
      expect(nameTables).toHaveBeenCalledTimes(1);
    });

    it("falls_back_to_the_number_with_the_reason_when_the_music_names_cannot_be_read", async () => {
      vi.mocked(nameTables).mockReset().mockRejectedValue({ code: "error", message: "Namen nicht lesbar", asset: null });
      const info: ProjectInfo = { root: "/projekt-ohne-namen", game_code: "CPUD", family: "Pt", language: "de", header_count: 593 };
      await show(context({ info }), "erzaehlen");
      // No picker that would wait for ever: the number field, which still writes, and why the names are missing.
      expect(button("Musik (Tag) wählen…")).toBeUndefined();
      expect(container.querySelector("[data-music-field]")).toBeNull();
      const day = input("Musik (Tag)")!;
      expect(day.value).toBe("6");
      expect([...container.querySelectorAll("[data-music-error]")].map((n) => n.textContent)).toEqual([expect.stringContaining("Namen nicht lesbar"), expect.stringContaining("Namen nicht lesbar")]);
      await act(async () => void day.focus());
      typeInto(day, "9");
      await act(async () => void day.blur());
      await flush();
      expect(setHeader).toHaveBeenCalledWith(411, pt({ music_day_id: 9 }), "Musik (Tag)");
    });

    it("erzaehlen_writes_the_chosen_look_as_one_header_edit", async () => {
      const other: HeaderSummary = { ...summary, id: 500, internal_name: "R01", location_name: "Route 201", area_data_id: 3 };
      await show(context({ headers: [summary, other] }), "erzaehlen");
      await click(container.querySelector("button[data-look-picker]") ?? undefined);
      const option = [...document.querySelectorAll<HTMLElement>("[role=option]")].find((o) => o.textContent?.includes("Route 201"));
      await click(option);
      await flush();
      expect(setHeader).toHaveBeenCalledWith(411, pt({ area_data_id: 3 }), "Aussehen");
    });

    it("erzaehlen_renames_a_room_from_its_name_row", async () => {
      const onRename = vi.fn();
      await act(async () =>
        root.render(withReset(reset, <HeaderEditor id={412} context={context({ headers: [summary, house], world: houseWorld(0), onRename })} depth="erzaehlen" />)),
      );
      await flush();
      expect(container.querySelector("[data-name-row]")?.textContent).toContain("Haus 1");
      await click(button("Umbenennen…"));
      const field = document.querySelector<HTMLInputElement>("[role=dialog] input")!;
      typeInto(field, "Keller");
      await click([...document.querySelectorAll<HTMLButtonElement>("[role=dialog] button")].find((b) => b.textContent === "Umbenennen"));
      expect(onRename).toHaveBeenCalledWith({ kind: "room", id: 412 }, "Keller");
    });
  });
});
