// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createRoom, getEvents, getMap, getMatrix, getTextArchive, headerPatchStatus, placeSlots, recipePlan, tileBehaviors } from "../../api";
import type { DraftDto, EventsDto, HeaderPatchDto, HeaderSummary, MatrixSummary, RoomReport, RoomSpec, WarpLink } from "../../api";
import type { Depth } from "../../ui/frame/shellState";
import { visibleWords } from "../../ui/visibleText";
import { resetTileNames } from "../../world/scene/tileNames";
import { buildPlaces } from "../places";
import { NewRoomWizard } from "./NewRoomWizard";
import { DRAFT_DEBOUNCE_MS } from "./useDraftPlan";

vi.mock("../../api", async (importOriginal) => ({ ...(await importOriginal<typeof import("../../api")>()), createRoom: vi.fn(), getEvents: vi.fn(), getMap: vi.fn(), getMatrix: vi.fn(), tileBehaviors: vi.fn(), headerPatchStatus: vi.fn(), applyHeaderPatch: vi.fn(), dirty: vi.fn(async () => []), placeSlots: vi.fn(), getTextArchive: vi.fn(), recipePlan: vi.fn() }));
// The template picture asks the app-wide cache for a map picture; there is no renderer here.
vi.mock("../../world/thumbnailStore", () => ({
  thumbnails: { get: () => undefined, request: () => {}, cancel: () => {}, subscribe: () => () => {}, failure: () => null },
}));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const flush = () => act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));

const header = (id: number, internal: string, location: string, matrix: number, area: number, event: number, place = 0): HeaderSummary => ({ id, internal_name: internal, location_index: place, kanto: false, location_name: location, matrix_id: matrix, area_data_id: area, event_file_id: event, text_archive_id: 0, script_file_id: 0, level_script_id: 0 });
// 0: the village (outside), 2: the template room, 3: the header the new room is given to, 4: another room,
// 5: a second outside header, 6: a room that shares the matrix of header 2 (its first door lies beyond the first map).
const headers = [
  header(0, "T01", "Zweiblattdorf", 0, 1, 5, 1),
  header(2, "T01R0201", "Spielerhaus", 2, 4, 8, 3),
  header(3, "NEWROOM", "", 3, 6, 9),
  header(4, "T01R0202", "Nachbarhaus", 4, 4, 10),
  header(5, "T02", "Blumenau", 5, 1, 11, 2),
  header(6, "T01R0203", "Zwillingshaus", 2, 4, 12),
];
const summary = (index: number, hasHeaders: boolean): MatrixSummary => ({ index, name: `m${index}`, width: hasHeaders ? 2 : 1, height: 1, has_headers: hasHeaders, has_altitudes: false, filled: 1 });
const matrices = [summary(0, true), summary(2, false), summary(3, false), summary(4, false), summary(5, true)];
const places = buildPlaces(headers, matrices);
const links: WarpLink[] = [];
const patchState = (change: Partial<HeaderPatchDto> = {}): HeaderPatchDto => ({ supported: true, applied: true, repairable: false, narc: "unpacked/dynamicHeaders", ...change });
// The plan of the room as the kernel answers it: the new matrix (null) and, on the village, the door and its tile.
const planned: DraftDto = {
  cells: [{ matrix: null, x: 0, y: 0, source_map: 321 }],
  tiles: [{ matrix: 0, x: 12, z: 7 }],
  events: [{ kind: "warp", matrix: 0, x: 12, z: 7, look: null, target: { header: 3, warp: 0 } }],
  messages: [],
  notes: ["Legt Raum „Neuer Raum“ als Kopie von Karte 0321 an.", "Der Raum entsteht auf einer neuen Weltkarte."],
  irreversible: ["Karte 0321 wird kopiert", "Eine neue Weltkarte wird angelegt"],
};
// The same plan in the kernel's own words, as `recipes/area.rs` and `room.rs` write them (Tüfteln's words).
const kernelWords: DraftDto = {
  ...planned,
  notes: ["Legt Raum „Neuer Raum“ als Kopie von Karte 0321 an.", "Header 3 zeigt auf die neue Weltkarte.", "Header 3 bekommt eine Kopie von Eventdatei 12."],
  irreversible: ["Karte 0321 wird kopiert", "Eventdatei 12 wird kopiert"],
};
const report: RoomReport = { matrix: 131, map: 571, event_file: 20, outside_warp: 3, inside_warp: 0, inside_warp_reused: false, door_tiles: [], patched: false, snapshot: null, place: null };

const events = (file: number, locked: string | null = null): EventsDto => ({
  event_file_id: file,
  locked,
  bg: [],
  objects: [],
  coords: [],
  // The template room's door is the first warp of file 8.
  warps: file === 8 ? [{ index: 0, x: 5, z: 6, dest_header_id: 0, dest_warp_id: 1, height: 0 }] : file === 12 ? [{ index: 0, x: 40, z: 6, dest_header_id: 0, dest_warp_id: 1, height: 0 }] : [],
});

describe("NewRoomWizard", () => {
  let container: HTMLElement;
  let root: Root;
  const onClose = vi.fn();
  const onCreated = vi.fn();
  const onError = vi.fn();
  const lastTile = vi.fn();
  const onHeaderPatch = vi.fn();
  const onDraft = vi.fn<(draft: DraftDto | null) => void>();
  let lockedFiles: Record<number, string>;
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    lockedFiles = {};
    // The village (matrix 0) is two maps wide; its second cell belongs to header 5.
    vi.mocked(getMatrix)
      .mockReset()
      .mockImplementation(async (index) => (index === 0 ? { index, name: "m0", width: 2, height: 1, maps: [400, 401], headers: [0, 5] } : { index, name: `m${index}`, width: 1, height: 1, maps: [index === 2 ? 321 : 400 + index] }));
    // Plain tiles everywhere, except the template's door mat at (5, 6) of map 321.
    resetTileNames();
    vi.mocked(tileBehaviors).mockReset().mockResolvedValue([
      { value: 0x65, name: "WARP_ENTRANCE_SOUTH", class: "warp" },
      { value: 0x69, name: "DOOR", class: "door" },
    ]);
    vi.mocked(getMap)
      .mockReset()
      .mockImplementation(async (index) => ({ index, buildings: [], permissions: Array.from({ length: 1024 }, (_, i): [number, number] => (index === 321 && i === 6 * 32 + 5 ? [0x65, 0] : [0, 0])) }));
    vi.mocked(getEvents).mockReset().mockImplementation(async (file) => events(file, lockedFiles[file] ?? null));
    vi.mocked(createRoom).mockReset().mockResolvedValue(report);
    vi.mocked(recipePlan).mockReset().mockResolvedValue(planned);
    onDraft.mockReset();
    onClose.mockReset();
    onCreated.mockReset().mockResolvedValue(undefined);
    onError.mockReset();
    lastTile.mockReset().mockReturnValue({ x: 12, z: 7 });
    onHeaderPatch.mockReset().mockResolvedValue(undefined);
    vi.mocked(headerPatchStatus).mockReset().mockResolvedValue(patchState());
    vi.mocked(placeSlots).mockReset().mockResolvedValue({ bank: 433, count: 4, free: 252, reusable: [] });
    vi.mocked(getTextArchive)
      .mockReset()
      .mockResolvedValue({ index: 433, key: 0, lang: "de", messages: ["Mysteriöser Ort", "Zweiblattdorf", "Blumenau", "Spielerhaus"].map((text, i) => ({ id: `msg_${i}`, text })), users: [] });
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
    vi.useRealTimers();
  });

  async function show(prefill: { outsideHeader?: number; template?: number } | undefined = { outsideHeader: 0 }, overview = { loaded: true, failed: false }, depth: Depth = "tuefteln") {
    await act(async () =>
      root.render(
        <NewRoomWizard depth={depth} headers={headers} links={links} counts={{ Map: 600, AreaData: 20 }} matrices={matrices} matricesLoaded={overview.loaded} matricesFailed={overview.failed} shownMatrix={0} places={places} prefill={prefill} lastTile={lastTile} onClose={onClose} onCreated={onCreated} onHeaderPatch={onHeaderPatch} onError={onError} onDraft={onDraft} />,
      ),
    );
    await flush();
  }
  const button = (name: string) => [...container.querySelectorAll("button")].find((b) => (b.getAttribute("aria-label") ?? b.textContent) === name);
  /** A template room in the list of the first step, by its header number. */
  const templateRow = (id: number) => container.querySelector<HTMLElement>(`[role="listbox"] [data-header="${id}"]`);
  /** The rows of the open header picker (a dialog outside the container). */
  const options = () => [...document.querySelectorAll<HTMLElement>("[role=option]")];
  const option = (id: string) => options().find((o) => o.textContent?.includes(id));
  async function openPicker(name: string) {
    await click(button(name));
    await flush();
  }
  /** Picks the header with this number: in step "Header" the room's own, in step "Eingang" the one outside. */
  async function pickHeader(id: string, picker = "Header wählen…") {
    await openPicker(picker);
    await click(option(id));
    await flush();
  }
  const pickOutside = (id: string) => pickHeader(id, "Header außen wählen…");
  /** A checkbox by the start of its label. */
  const checkbox = (label: string) => [...container.querySelectorAll<HTMLElement>("[role=checkbox]")].find((c) => c.closest("label")?.textContent?.startsWith(label));
  const checked = (el: Element | undefined) => el?.getAttribute("aria-checked") === "true";
  const disabled = (el: Element | undefined) => el?.getAttribute("aria-disabled") === "true" || el?.hasAttribute("data-disabled") === true;
  const click = (el: Element | null | undefined) => act(async () => void el?.dispatchEvent(new MouseEvent("click", { bubbles: true })));
  const next = () => click(button("Weiter"));
  const field = (label: string) => container.querySelector<HTMLInputElement>(`input[aria-label="${label}"]`)!;
  const current = () => container.querySelector('[aria-current="step"]')?.getAttribute("aria-label");
  async function type(input: HTMLInputElement, text: string) {
    await act(async () => input.focus());
    await act(async () => {
      Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")?.set?.call(input, text);
      input.dispatchEvent(new Event("input", { bubbles: true }));
    });
    await act(async () => input.blur());
  }
  /** Template room 0002, new header 0003, then on to the step "Eingang". */
  async function toEntrance() {
    await click(templateRow(2));
    await flush();
    await next();
    await pickHeader("0003");
    await next();
  }

  it("creates_the_room_with_the_entrance_at_the_last_hovered_tile_and_the_templates_door_inside", async () => {
    await show();
    await toEntrance();
    expect(current()).toBe("3. Eingang");
    expect(field("Außen: Kachel x").value).toBe("12");
    expect(field("Außen: Kachel z").value).toBe("7");
    expect(field("Innen: Kachel x").value).toBe("5");
    expect(field("Innen: Kachel z").value).toBe("6");
    await next();
    expect(container.textContent).toContain("Verbindet 0000 Zweiblattdorf (Kachel x 12, z 7) mit dem neuen Raum");
    // Only the tile outside is a plain one: the template's door mat inside starts a warp.
    expect(container.textContent).toContain("Macht die Kachel außen zur Tür (Verhalten 0x69, Kollision 0x80)");
    expect(container.textContent).toContain("Anlegen lässt sich nicht rückgängig machen.");
    await click(button("Anlegen"));
    expect(createRoom).toHaveBeenCalledOnce();
    const spec = vi.mocked(createRoom).mock.calls[0][0];
    expect(spec).toEqual({
      name: "Neuer Raum",
      source_map: 321,
      clear_buildings: false,
      header: { id: 3, area_data_id: 4, event_file: "New" },
      entrance: { outside_header: 0, x: 12, z: 7, inside_x: 5, inside_z: 6, door_tiles: true },
    });
    expect(container.textContent).toContain("Raum „Neuer Raum“ angelegt");
    expect(container.textContent).toContain("Eingang: Warp 3 in Header 0000");
    await click(button("Zeigen"));
    expect(onCreated).toHaveBeenCalledWith(report, spec);
  });

  it("the_result_offers_neue_figur_hier_locked", async () => {
    await show();
    await toEntrance();
    await next();
    await click(button("Anlegen"));
    // The kernel's next step after a room, as a recipe button that waits for 3d.5.
    const keys = [...container.querySelectorAll<HTMLButtonElement>("button[data-recipe]")];
    expect(keys.map((b) => [b.getAttribute("data-recipe"), b.textContent, b.disabled])).toEqual([["new_figure", "Neue Figur hier", true]]);
    expect(document.getElementById(keys[0]!.getAttribute("aria-describedby")!)?.textContent).toBe("Kommt mit 3d.5");
  });

  it("keeps_the_default_door_inside_and_leaves_the_door_outside_open_without_a_hovered_tile_or_template_warp", async () => {
    lastTile.mockReturnValue(null);
    await show();
    await click(templateRow(4));
    await flush();
    await next();
    await pickHeader("0003");
    await next();
    expect(field("Außen: Kachel x").value).toBe("");
    expect(field("Innen: Kachel x").value).toBe("16");
    expect(field("Innen: Kachel z").value).toBe("30");
    expect(container.textContent).toContain("Standardwert: die Vorlage hat keinen Warp.");
    // Without a tile the entrance cannot be made: x and z have to be entered.
    expect(button("Weiter")?.disabled).toBe(true);
    await type(field("Außen: Kachel x"), "3");
    await type(field("Außen: Kachel z"), "4");
    expect(button("Weiter")?.disabled).toBe(false);
  });

  it("takes_the_hovered_tile_only_for_an_outside_header_that_uses_the_shown_matrix", async () => {
    await show({ outsideHeader: 0 });
    await toEntrance();
    expect(container.textContent).toContain("zuletzt in der Szene");
    expect(container.textContent).toContain("Raster 0000: 64 × 32 Kacheln");
    await click(button("Zurück"));
    await click(button("Zurück"));
    await click(button("Zurück"));
    expect(current()).toBe("1. Vorlage");
    await next();
    await next();
    // Header 0005 uses matrix 5, not the shown matrix 0: the tile stays open.
    await pickOutside("0005");
    expect(field("Außen: Kachel x").value).toBe("");
    expect(field("Außen: Kachel z").value).toBe("");
    expect(container.textContent).not.toContain("zuletzt in der Szene");
    expect(container.textContent).toContain("Raster 0005: 64 × 32 Kacheln");
    expect(button("Weiter")?.disabled).toBe(true);
  });

  it("refuses_a_typed_door_outside_the_outside_matrix_instead_of_clamping_it", async () => {
    // Gate 3a (F): x 999 became 959 without a word, in a cell without a map. The field refuses it, names the bound and keeps the tile.
    await show({ outsideHeader: 0 });
    await toEntrance();
    await type(field("Außen: Kachel x"), "999");
    expect(field("Außen: Kachel x").value).toBe("12");
    expect(container.textContent).toContain("höchstens 63");
    expect(container.textContent).toContain("0 bis 63");
    expect(container.textContent).not.toContain("959");
    // The second cell has a map, but it belongs to header 0005: the village's warps are not loaded there.
    await type(field("Außen: Kachel x"), "40");
    expect(container.textContent).toContain("Kachel x 40, z 7 liegt in Zelle (1, 0) von Header 0005, nicht von 0000");
    expect(button("Weiter")?.disabled).toBe(true);
    await type(field("Außen: Kachel x"), "12");
    expect(button("Weiter")?.disabled).toBe(false);
  });

  it("refuses_a_door_outside_on_a_tile_that_already_has_a_warp", async () => {
    vi.mocked(getEvents).mockImplementation(async (file) => (file === 5 ? { ...events(5), warps: [{ index: 2, x: 12, z: 7, dest_header_id: 4, dest_warp_id: 0, height: 0 }] } : events(file)));
    await show();
    await toEntrance();
    expect(container.textContent).toContain("Auf Kachel x 12, z 7 außen liegt schon Warp 2 (Ziel Header 0004)");
    expect(button("Weiter")?.disabled).toBe(true);
  });

  it("makes_the_copied_door_warp_the_way_back_instead_of_adding_a_second_one", async () => {
    // Gate 3a (A) 12: with a copy of the template's event file, its exit warp stays on the inside door tile.
    await show();
    await click(templateRow(2));
    await flush();
    await next();
    await pickHeader("0003");
    await click(button("Kopie von 0008"));
    await next();
    await flush();
    expect(container.textContent).toContain("Die kopierte Eventdatei hat hier schon Warp 0 (Ziel Header 0000): er wird zum Rückweg");
    await next();
    expect(container.textContent).toContain("Der kopierte Warp 0 auf dieser Kachel (bisher Ziel Header 0000) wird zum Rückweg; es kommt kein zweiter dazu.");
  });

  it("names_the_door_tiles_and_leaves_them_alone_when_asked_to", async () => {
    await show();
    await toEntrance();
    await flush();
    expect(container.textContent).toContain("Außen: Verhalten 0x00, Kollision 0x00: ein Warp löst hier im Spiel nicht aus (keine Tür- oder Warp-Kachel).");
    expect(container.textContent).toContain("Innen: Verhalten 0x65 WARP_ENTRANCE_SOUTH, Kollision 0x00: ein Warp löst hier aus.");
    expect(checked(checkbox("Kacheln, auf denen"))).toBe(true);
    await click(checkbox("Kacheln, auf denen"));
    expect(checked(checkbox("Kacheln, auf denen"))).toBe(false);
    await next();
    expect(container.textContent).toContain("Achtung: auf der Kachel außen löst ein Warp im Spiel nicht aus");
    await click(button("Anlegen"));
    expect(vi.mocked(createRoom).mock.calls[0][0].entrance?.door_tiles).toBe(false);
  });

  it("can_go_on_after_clicking_the_chosen_template_again_or_taking_one_that_shares_its_matrix", async () => {
    await show();
    await click(templateRow(2));
    await flush();
    await click(templateRow(2));
    await flush();
    expect(button("Weiter")?.disabled).toBe(false);
    await click(templateRow(6));
    await flush();
    expect(button("Weiter")?.disabled).toBe(false);
    await next();
    await pickHeader("0003");
    await next();
    await next();
    await click(button("Anlegen"));
    expect(vi.mocked(createRoom).mock.calls[0][0].source_map).toBe(321);
  });

  it("says_where_the_inside_door_comes_from_and_when_it_was_limited_to_the_map", async () => {
    await show();
    await click(templateRow(2));
    await flush();
    await next();
    await pickHeader("0003");
    await next();
    expect(container.textContent).toContain("aus der Vorlage übernommen");
    expect(container.textContent).not.toContain("begrenzt");
    await click(button("Zurück"));
    await click(button("Zurück"));
    await click(templateRow(6));
    await flush();
    await next();
    await next();
    expect(field("Innen: Kachel x").value).toBe("31");
    expect(container.textContent).toContain("Kachel x 40, z 6");
    expect(container.textContent).toContain("auf Kachel x 31, z 6 begrenzt");
    await type(field("Innen: Kachel x"), "7");
    expect(container.textContent).not.toContain("begrenzt");
    expect(container.textContent).not.toContain("aus der Vorlage übernommen");
  });

  it("keeps_an_inside_tile_the_user_typed_when_the_templates_door_arrives", async () => {
    await show();
    await click(templateRow(2));
    await next();
    await pickHeader("0003");
    await next();
    await type(field("Innen: Kachel x"), "9");
    expect(field("Innen: Kachel x").value).toBe("9");
    await flush();
    expect(field("Innen: Kachel x").value).toBe("9");
  });

  it("lists_only_rooms_as_templates_and_narrows_them_by_the_search", async () => {
    await show();
    expect(templateRow(0)).toBeNull();
    expect([2, 3, 4].map((id) => templateRow(id) !== null)).toEqual([true, true, true]);
    // Each row names the room and its header; the place is the group above.
    expect(templateRow(2)?.textContent).toBe("Spielerhaus0002 T01R0201");
    await type(field("Vorlage suchen"), "nachbar");
    expect(templateRow(2)).toBeNull();
    expect(templateRow(4)).not.toBeNull();
  });

  it("clears_a_filled_template_search_with_the_first_escape_and_cancels_with_the_second", async () => {
    await show();
    const search = field("Vorlage suchen");
    const escape = () => act(async () => void search.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true, cancelable: true })));
    await act(async () => search.focus());
    await act(async () => {
      Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")?.set?.call(search, "nachbar");
      search.dispatchEvent(new Event("input", { bubbles: true }));
    });
    expect(templateRow(2)).toBeNull();
    await escape();
    expect(onClose).not.toHaveBeenCalled();
    expect(search.value).toBe("");
    expect(templateRow(2)).not.toBeNull();
    await escape();
    expect(onClose).toHaveBeenCalledWith(false);
  });

  it("keeps_the_notes_of_the_template_list_outside_the_listbox", async () => {
    await show();
    await type(field("Vorlage suchen"), "gibtsnicht");
    expect(container.textContent).toContain("Kein Raum gefunden");
    expect(container.querySelector('[role="listbox"]')?.textContent).not.toContain("Kein Raum gefunden");
    expect([...(container.querySelector('[role="listbox"]')?.children ?? [])].every((c) => c.getAttribute("role") === "group")).toBe(true);
  });

  it("gates_weiter_on_the_template_and_on_the_header", async () => {
    await show();
    expect(button("Weiter")?.disabled).toBe(true);
    await click(templateRow(2));
    await flush();
    expect(templateRow(2)?.getAttribute("aria-selected")).toBe("true");
    expect(button("Weiter")?.disabled).toBe(false);
    await next();
    expect(current()).toBe("2. Header");
    expect(button("Weiter")?.disabled).toBe(true);
    await pickHeader("0003");
    expect(button("Weiter")?.disabled).toBe(false);
  });

  it("copies_the_templates_map_from_the_first_cell_of_its_matrix", async () => {
    await show({ outsideHeader: 0, template: 2 });
    expect(getMatrix).toHaveBeenCalledWith(2);
    expect(button("Weiter")?.disabled).toBe(false);
  });

  it("does_not_offer_the_rooms_own_header_as_the_header_outside", async () => {
    await show({ outsideHeader: 0 });
    await click(templateRow(2));
    await flush();
    await next();
    // Header 0000 is the prefilled outside header; making it the room's header drops it from the entrance.
    await pickHeader("0000");
    await next();
    expect(button("Weiter")?.disabled).toBe(true);
    expect(container.textContent).toContain("Außen-Header wählen.");
    await openPicker("Header außen wählen…");
    expect(option("0000")).toBeUndefined();
    expect(option("0005")).toBeDefined();
  });

  it("offers_the_prefilled_outside_header_even_when_it_is_a_room", async () => {
    await show({ outsideHeader: 4 });
    await toEntrance();
    expect(container.querySelector('[data-header="4"]')?.textContent).toContain("Nachbarhaus");
    await type(field("Außen: Kachel x"), "3");
    await type(field("Außen: Kachel z"), "3");
    await next();
    expect(container.textContent).toContain("Verbindet 0004 Nachbarhaus");
  });

  it("makes_no_entrance_when_connecting_later", async () => {
    await show();
    await toEntrance();
    await click(checkbox("später verbinden"));
    expect(checked(checkbox("später verbinden"))).toBe(true);
    expect(field("Außen: Kachel x").disabled).toBe(true);
    expect(button("Header außen wählen…")?.disabled).toBe(true);
    await next();
    expect(container.textContent).toContain("Legt keinen Eingang an");
    await click(button("Anlegen"));
    expect(vi.mocked(createRoom).mock.calls[0][0].entrance).toBeNull();
  });

  it("makes_no_entrance_and_says_why_when_the_outside_event_file_is_locked", async () => {
    lockedFiles[5] = "17 bytes read";
    await show();
    await toEntrance();
    expect(container.textContent).toContain("schreibgeschützt");
    expect(container.textContent).toContain("17 bytes read");
    expect(checked(checkbox("später verbinden"))).toBe(true);
    expect(disabled(checkbox("später verbinden"))).toBe(true);
    await next();
    expect(container.textContent).toContain("Legt keinen Eingang an (Eventdatei 0005 (Außen) ist schreibgeschützt: 17 bytes read)");
    await click(button("Anlegen"));
    expect(vi.mocked(createRoom).mock.calls[0][0].entrance).toBeNull();
  });

  it("follows_the_lock_to_the_chosen_outside_header_so_another_one_can_still_be_picked", async () => {
    lockedFiles[5] = "17 bytes read";
    await show();
    await toEntrance();
    expect(container.textContent).toContain("schreibgeschützt");
    // The picker stays usable: header 0005 has an editable event file.
    expect(button("Header außen wählen…")?.disabled).toBe(false);
    await pickOutside("0005");
    expect(container.textContent).not.toContain("schreibgeschützt");
    expect(checked(checkbox("später verbinden"))).toBe(false);
    expect(disabled(checkbox("später verbinden"))).toBe(false);
    await type(field("Außen: Kachel x"), "4");
    await type(field("Außen: Kachel z"), "5");
    await next();
    await click(button("Anlegen"));
    expect(vi.mocked(createRoom).mock.calls[0][0].entrance).toEqual({ outside_header: 5, x: 4, z: 5, inside_x: 5, inside_z: 6, door_tiles: true });
  });

  it("makes_no_entrance_when_a_copy_of_a_locked_template_event_file_is_asked_for", async () => {
    lockedFiles[8] = "boom";
    await show();
    await click(templateRow(2));
    await flush();
    await next();
    await pickHeader("0003");
    await click(button("Kopie von 0008"));
    await next();
    await flush();
    expect(container.textContent).toContain("schreibgeschützt");
    await next();
    await click(button("Anlegen"));
    const spec = vi.mocked(createRoom).mock.calls[0][0];
    expect(spec.header.event_file).toEqual({ Copy: 8 });
    expect(spec.entrance).toBeNull();
  });

  it("says_the_overview_is_loading_instead_of_listing_no_rooms_and_no_outside_headers", async () => {
    // Before the overview arrived every header counts as a room and no places are known.
    const unknown = buildPlaces(headers, []);
    await act(async () =>
      root.render(
        <NewRoomWizard depth="tuefteln" headers={headers} links={links} counts={{ Map: 600, AreaData: 20 }} matrices={[]} matricesLoaded={false} matricesFailed={false} shownMatrix={0} places={unknown} prefill={{ template: 2 }} lastTile={lastTile} onClose={onClose} onCreated={onCreated} onHeaderPatch={onHeaderPatch} onError={onError} onDraft={onDraft} />,
      ),
    );
    await flush();
    expect(container.textContent).toContain("Lade Übersicht …");
    expect(templateRow(2)).toBeNull();
    expect(container.textContent).not.toContain("Kein Raum gefunden");
    // The template the request names is still the chosen one.
    await next();
    await pickHeader("0003");
    await next();
    expect(container.textContent).toContain("Lade Übersicht …");
    // No outside header is offered.
    expect(button("Header außen wählen…")).toBeUndefined();
  });

  it("says_the_overview_is_not_available_when_reading_it_failed", async () => {
    await show({ outsideHeader: 0 }, { loaded: false, failed: true });
    expect(container.textContent).toContain("Übersicht nicht verfügbar");
    expect(container.textContent).not.toContain("Lade Übersicht");
    expect(templateRow(2)).toBeNull();
  });

  it("refuses_an_inside_tile_off_the_map_instead_of_clamping_it", async () => {
    await show();
    await toEntrance();
    await type(field("Innen: Kachel x"), "40");
    expect(field("Innen: Kachel x").value).toBe("5");
    expect(container.textContent).toContain("höchstens 31");
    expect(button("Weiter")?.disabled).toBe(false);
  });

  it("stays_on_the_summary_and_shows_a_refusal_that_created_nothing_inside_the_dialog", async () => {
    vi.mocked(createRoom).mockRejectedValue({ code: "error", message: "the room's header cannot also be the header outside", asset: null });
    await show();
    await toEntrance();
    await next();
    await click(button("Anlegen"));
    // The refusal is shown inside the dialog (the status strip is hidden behind its backdrop).
    expect(onError).not.toHaveBeenCalled();
    expect(container.querySelector('[role="dialog"] [role="alert"]')?.textContent).toBe("the room's header cannot also be the header outside");
    expect(current()).toBe("4. Zusammenfassung");
    expect(button("Anlegen")?.disabled).toBe(false);
  });

  it("shows_a_late_failure_in_the_result_step", async () => {
    vi.mocked(createRoom).mockRejectedValue({ code: "partial", message: "map 0571 and matrix 0131 were created; header edit failed: boom", asset: null });
    await show();
    await toEntrance();
    await next();
    await click(button("Anlegen"));
    expect(onError).not.toHaveBeenCalled();
    // In German, with the system's reason as it came.
    expect(container.textContent).toContain("Angelegt: Karte 0571, Raster 0131. Danach fehlgeschlagen: Header umstellen (boom).");
    expect(button("Zeigen")).toBeUndefined();
    await click(button("Schließen"));
    expect(onClose).toHaveBeenCalledWith(true);
  });

  describe("in Bauen", () => {
    const consent = () => container.querySelector<HTMLElement>("[data-patch-consent] [role=checkbox]");
    const placeButton = () => container.querySelector<HTMLButtonElement>("button[data-place-picker]");
    /** Template room 0002, then the step „Ort“ (the entrance leads out to 0000 Zweiblattdorf). */
    async function toPlace(patch: Partial<HeaderPatchDto> = {}, depth: Depth = "bauen") {
      vi.mocked(headerPatchStatus).mockResolvedValue(patchState(patch));
      await show({ outsideHeader: 0 }, { loaded: true, failed: false }, depth);
      await click(templateRow(2));
      await flush();
      await next();
      await flush();
    }
    async function toSummary(patch: Partial<HeaderPatchDto> = {}) {
      await toPlace(patch);
      await next();
      await next();
    }

    it("shows_the_step_place_in_bauen_and_header_in_tuefteln", async () => {
      await toPlace();
      expect(current()).toBe("2. Ort");
      // The room takes the place of the area at its entrance.
      expect(placeButton()?.textContent).toBe("Zweiblattdorf");
      expect(container.textContent).toContain("Aussehen wie Spielerhaus");
      expect(button("Weiter")?.disabled).toBe(false);
      act(() => root.unmount());
      root = createRoot(container);
      await toPlace({}, "tuefteln");
      expect(current()).toBe("2. Header");
      expect(placeButton()).toBeNull();
    });

    it("cannot_create_without_the_consent_when_the_patch_is_missing", async () => {
      await toSummary({ applied: false });
      expect(current()).toBe("4. Zusammenfassung");
      expect(container.querySelector("[data-patch-consent]")?.textContent).toContain("Nicht rückgängig");
      expect(button("Anlegen")?.disabled).toBe(true);
      await click(consent());
      expect(button("Anlegen")?.disabled).toBe(false);
    });

    it("sends_allow_patch_a_new_place_and_no_internal_name", async () => {
      await toPlace({ applied: false });
      await click(button("Neuer Ort"));
      expect(field("Name des neuen Orts").value).toBe("Neuer Raum");
      await next();
      await next();
      await click(consent());
      await click(button("Anlegen"));
      expect(vi.mocked(createRoom).mock.calls[0][0].header).toEqual({
        id: 6,
        area_data_id: null,
        event_file: "New",
        new_header: { template: 2, internal_name: null },
        place: { New: "Neuer Raum" },
        allow_patch: true,
      });
    });

    it("calls_on_header_patch_after_a_patching_create", async () => {
      vi.mocked(createRoom).mockResolvedValue({ ...report, patched: true });
      await toSummary({ applied: false });
      await click(consent());
      await click(button("Anlegen"));
      await flush();
      expect(vi.mocked(createRoom).mock.calls[0][0].header).toMatchObject({ place: { Existing: 1 } });
      expect(onHeaderPatch).toHaveBeenCalledOnce();
    });

    it("reads_the_patch_again_after_a_late_failure_of_a_patching_create", async () => {
      vi.mocked(createRoom).mockRejectedValue({ code: "partial", message: "the Dynamic Headers patch was applied and stays, header 0006 and matrix 0131 were created; warp edit failed: boom", asset: null });
      await toSummary({ applied: false });
      await click(consent());
      await click(button("Anlegen"));
      await flush();
      expect(onHeaderPatch).toHaveBeenCalledOnce();
      expect(container.textContent).toContain("Spielcode für neue Orte angepasst (bleibt). Angelegt: der Raum, die Anordnung. Danach fehlgeschlagen: Warp setzen (boom).");
      expect(visibleWords(document.body)).not.toMatch(/Raster|Matrix|Header/);
      act(() => root.unmount());
      root = createRoot(container);
      // Without the patch in the call there is nothing to read again.
      onHeaderPatch.mockClear();
      await toSummary({ applied: true });
      await click(button("Anlegen"));
      await flush();
      expect(onHeaderPatch).not.toHaveBeenCalled();
    });

    it("names_the_snapshot_and_hands_a_failed_read_to_on_error", async () => {
      const failure = { code: "error", message: "read failed", asset: null };
      onHeaderPatch.mockRejectedValue(failure);
      vi.mocked(createRoom).mockResolvedValue({ ...report, patched: true, snapshot: "/p/.g4/backup/dynamicHeaders-20261026-120000" });
      await toSummary({ applied: false });
      await click(consent());
      await click(button("Anlegen"));
      await flush();
      expect(onError).toHaveBeenCalledWith(failure);
      expect(container.textContent).toContain("Raum „Neuer Raum“ angelegt");
      expect(container.textContent).toContain("Die alten Dateien sind unter .g4/backup gesichert.");
      expect(visibleWords(document.body)).not.toMatch(/Raster|Matrix|Header/);
      expect(container.querySelector('[role="dialog"] [role="alert"]')).toBeNull();
    });

    it("takes_the_templates_place_when_the_entrance_cannot_be_made", async () => {
      // The village's event file is locked: no entrance, so the room does not take the village's place.
      lockedFiles[5] = "DSPRE";
      await toPlace();
      expect(placeButton()?.textContent).toBe("Spielerhaus");
    });

    it("plans_with_the_patch_assumed_while_it_is_missing_and_creates_with_the_users_consent", async () => {
      vi.useFakeTimers({ shouldAdvanceTime: true });
      // As the kernel: a new header without the patch and without allow_patch is refused.
      vi.mocked(recipePlan).mockImplementation(async (_recipe, input) =>
        (input as RoomSpec).header.allow_patch === true ? planned : Promise.reject({ code: "refused", message: "new headers need the dynamic headers patch, apply the patch first", asset: null }),
      );
      await toPlace({ applied: false });
      await act(async () => void (await vi.advanceTimersByTimeAsync(DRAFT_DEBOUNCE_MS)));
      // The draft shows from the step „Ort“ on, before any consent: the plan writes nothing.
      expect(vi.mocked(recipePlan).mock.lastCall?.[0]).toBe("create_room");
      expect((vi.mocked(recipePlan).mock.lastCall?.[1] as RoomSpec).header.allow_patch).toBe(true);
      expect(onDraft).toHaveBeenLastCalledWith(planned);
      await next();
      await next();
      expect(current()).toBe("4. Zusammenfassung");
      expect(container.textContent).not.toContain("Vorschau nicht möglich");
      expect(container.textContent).not.toContain("dynamic headers");
      // „Anlegen“ still waits for the user's consent and sends it.
      expect(button("Anlegen")?.disabled).toBe(true);
      await click(consent());
      await click(button("Anlegen"));
      expect(vi.mocked(createRoom).mock.calls[0][0].header.allow_patch).toBe(true);
      // With the patch in the game nothing is assumed: plan and create send no consent.
      act(() => root.unmount());
      root = createRoot(container);
      vi.mocked(recipePlan).mockClear();
      vi.mocked(createRoom).mockClear();
      await toSummary({ applied: true });
      await act(async () => void (await vi.advanceTimersByTimeAsync(DRAFT_DEBOUNCE_MS)));
      expect((vi.mocked(recipePlan).mock.lastCall?.[1] as RoomSpec).header.allow_patch).toBe(false);
      await click(button("Anlegen"));
      expect(vi.mocked(createRoom).mock.calls[0][0].header.allow_patch).toBe(false);
    });

    it("keeps_the_kernels_words_out_of_the_summary_in_bauen", async () => {
      vi.useFakeTimers({ shouldAdvanceTime: true });
      vi.mocked(recipePlan).mockResolvedValue(kernelWords);
      await toSummary();
      await act(async () => void (await vi.advanceTimersByTimeAsync(DRAFT_DEBOUNCE_MS)));
      expect(current()).toBe("4. Zusammenfassung");
      // The draft still reaches the scene; only the sentences stay out of the wizard.
      expect(onDraft).toHaveBeenLastCalledWith(kernelWords);
      expect(visibleWords(document.body)).not.toMatch(/Raster|Matrix|Header/);
      expect(container.textContent).not.toContain("zeigt auf die neue Weltkarte");
      expect(container.textContent).not.toContain("Eventdatei 12 wird kopiert");
      expect(container.querySelector("[data-draft-notes]")).toBeNull();
    });

    it("says_no_raster_matrix_or_header_in_building", async () => {
      const FORBIDDEN = /Raster|Matrix|Header/;
      const words = () => visibleWords(document.body);
      vi.mocked(headerPatchStatus).mockResolvedValue(patchState({ applied: false }));
      await show({ outsideHeader: 0 }, { loaded: true, failed: false }, "bauen");
      expect(words()).not.toMatch(FORBIDDEN);
      await click(templateRow(2));
      await flush();
      expect(container.textContent).toContain("„Spielerhaus“");
      expect(words()).not.toMatch(FORBIDDEN);
      await next();
      await flush();
      expect(current()).toBe("2. Ort");
      expect(words()).not.toMatch(FORBIDDEN);
      await next();
      expect(current()).toBe("3. Eingang");
      expect(container.textContent).toContain("Gebiet am Eingang");
      expect(words()).not.toMatch(FORBIDDEN);
      // A door in a cell of another area is refused in the same words.
      await type(field("Außen: Kachel x"), "40");
      expect(container.textContent).toContain("eines anderen Gebiets");
      expect(words()).not.toMatch(FORBIDDEN);
      await type(field("Außen: Kachel x"), "12");
      await openPicker("Gebiet am Eingang wählen…");
      expect(document.querySelector("[role=dialog]")?.textContent).toContain("Gebiet am Eingang wählen");
      expect(words()).not.toMatch(FORBIDDEN);
      await click(option("Zweiblattdorf"));
      await flush();
      await next();
      expect(current()).toBe("4. Zusammenfassung");
      expect(container.textContent).toContain("Legt den Raum „Neuer Raum“ mit einer Kopie der Karte 0321 an, mit Gebäuden.");
      expect(container.textContent).toContain("Verbindet Zweiblattdorf (Kachel x 12, z 7) mit dem neuen Raum.");
      expect(words()).not.toMatch(FORBIDDEN);
      await click(consent());
      await click(button("Anlegen"));
      await flush();
      expect(container.textContent).toContain("Raum „Neuer Raum“ angelegt: Karte 0571.");
      expect(words()).not.toMatch(FORBIDDEN);
    });

    it("says_no_raster_matrix_or_header_without_an_entrance_in_building", async () => {
      // No pointer over the scene: the door outside is asked for, then the room is connected later.
      lastTile.mockReturnValue(null);
      await toPlace();
      await next();
      expect(container.textContent).toContain("Noch keine Kachel");
      expect(visibleWords(document.body)).not.toMatch(/Raster|Matrix|Header/);
      await click(checkbox("später verbinden"));
      await next();
      expect(container.textContent).toContain("Legt keinen Eingang an");
      expect(visibleWords(document.body)).not.toMatch(/Raster|Matrix|Header/);
    });
  });

  describe("a new header", () => {
    /** An option of a `Segmented` by its text. */
    const radio = (label: string) => button(label) as HTMLButtonElement | undefined;
    const pressed = (label: string) => radio(label)?.getAttribute("aria-pressed") === "true";
    async function toHeaderStep() {
      await show();
      await click(templateRow(2));
      await flush();
      await next();
    }

    it("gates_weiter_on_a_choice_and_sends_the_new_header_as_a_copy_of_the_template_headers_next_number", async () => {
      await toHeaderStep();
      expect(button("Weiter")?.disabled).toBe(true);
      await click(radio("Neuer Header"));
      expect(button("Weiter")?.disabled).toBe(false);
      expect(container.textContent).toContain("Kopie von Header 0002 „Spielerhaus“");
      await next();
      await next();
      expect(container.textContent).toContain("legt Header 0006 „NEUER_RAUM“ neu an (Kopie von 0002) und stellt ihn auf das neue Raster um, Area-Data 0004, Eventdatei: neu, leer.");
      await click(button("Anlegen"));
      const spec = vi.mocked(createRoom).mock.calls[0][0];
      expect(spec.header).toEqual({ id: 6, area_data_id: 4, event_file: "New", new_header: { template: 2, internal_name: "NEUER_RAUM" } });
    });

    it("starts_with_a_new_event_file_and_says_that_keeping_it_shares_the_templates", async () => {
      await toHeaderStep();
      await click(radio("Neuer Header"));
      expect(pressed("Neu, leer")).toBe(true);
      await click(radio("Behalten"));
      await next();
      await flush();
      // The shared file is the template's: its door warp stands on the inside tile, and is not the wizard's to repoint.
      expect(container.textContent).toContain("Auf Kachel x 5, z 6 innen liegt schon Warp 0 der Eventdatei 0008, die der Raum behält");
      expect(button("Weiter")?.disabled).toBe(true);
      await click(checkbox("später verbinden"));
      await next();
      expect(container.textContent).toContain("Eventdatei: behalten (geteilt mit Header 0002)");
    });

    it("does_not_offer_a_new_header_without_the_patch", async () => {
      vi.mocked(headerPatchStatus).mockResolvedValue(patchState({ supported: false, applied: false, narc: null }));
      await toHeaderStep();
      expect(radio("Neuer Header")?.disabled).toBe(true);
      expect(container.textContent).toContain("Neue Header gibt es nur für Platin und HeartGold/SoulSilver");
    });
  });

  it("closes_with_escape_and_reports_that_nothing_was_written", async () => {
    await show();
    await act(async () => void container.querySelector('[role="dialog"]')?.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true })));
    expect(onClose).toHaveBeenCalledWith(false);
    expect(createRoom).not.toHaveBeenCalled();
  });

  it("shows_the_draft_while_the_entrance_is_complete_and_clears_it_on_cancel", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    const settle = () => act(async () => void (await vi.advanceTimersByTimeAsync(DRAFT_DEBOUNCE_MS)));
    // The host takes the wizard away when it closes.
    onClose.mockImplementation(() => root.render(null));
    await show();
    // No template yet: nothing to plan.
    await settle();
    expect(recipePlan).not.toHaveBeenCalled();
    await toEntrance();
    await settle();
    expect(recipePlan).toHaveBeenLastCalledWith("create_room", {
      name: "Neuer Raum",
      source_map: 321,
      clear_buildings: false,
      header: { id: 3, area_data_id: 4, event_file: "New" },
      entrance: { outside_header: 0, x: 12, z: 7, inside_x: 5, inside_z: 6, door_tiles: true },
    });
    expect(onDraft).toHaveBeenLastCalledWith(planned);
    await next();
    expect(current()).toBe("4. Zusammenfassung");
    const notes = container.querySelector("[data-draft-notes]");
    expect(notes?.textContent).toContain("Der Raum entsteht auf einer neuen Weltkarte.");
    expect(notes?.textContent).toContain("Nicht rückgängig");
    expect(notes?.textContent).toContain("Karte 0321 wird kopiert, Eine neue Weltkarte wird angelegt");
    await act(async () => void container.querySelector('[role="dialog"]')?.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true })));
    expect(onClose).toHaveBeenCalledWith(false);
    expect(onDraft).toHaveBeenLastCalledWith(null);
  });

  it("warns_when_the_plan_fails_and_still_creates", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    vi.mocked(recipePlan).mockRejectedValue({ code: "error", message: "Karte 0321 fehlt", asset: null });
    await show();
    await toEntrance();
    await act(async () => void (await vi.advanceTimersByTimeAsync(DRAFT_DEBOUNCE_MS)));
    await next();
    expect(container.textContent).toContain("Vorschau nicht möglich: Karte 0321 fehlt");
    expect(onDraft).not.toHaveBeenCalledWith(expect.objectContaining({ notes: expect.anything() }));
    await click(button("Anlegen"));
    expect(createRoom).toHaveBeenCalledOnce();
    // The result is shown: no draft any more.
    expect(container.querySelector("[data-draft-notes]")).toBeNull();
  });

  it("clears_the_draft_once_the_room_is_created", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    await show();
    await toEntrance();
    await act(async () => void (await vi.advanceTimersByTimeAsync(DRAFT_DEBOUNCE_MS)));
    expect(onDraft).toHaveBeenLastCalledWith(planned);
    await next();
    await click(button("Anlegen"));
    expect(container.textContent).toContain("Raum „Neuer Raum“ angelegt");
    expect(onDraft).toHaveBeenLastCalledWith(null);
  });

  it("shows_the_kernels_words_in_the_summary_in_tuefteln", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    vi.mocked(recipePlan).mockResolvedValue(kernelWords);
    await show();
    await toEntrance();
    await act(async () => void (await vi.advanceTimersByTimeAsync(DRAFT_DEBOUNCE_MS)));
    await next();
    const notes = container.querySelector("[data-draft-notes]");
    expect(notes?.textContent).toContain("Header 3 zeigt auf die neue Weltkarte.");
    expect(notes?.textContent).toContain("Nicht rückgängig: Karte 0321 wird kopiert, Eventdatei 12 wird kopiert");
    // The list has no badge of its own: the frame's foot carries it.
    expect(notes?.querySelector("[class*=badge]")).toBeNull();
  });
});
