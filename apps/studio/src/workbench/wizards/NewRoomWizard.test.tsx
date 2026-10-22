// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createRoom, getEvents, getMatrix } from "../../api";
import type { EventsDto, HeaderSummary, MatrixSummary, RoomReport, WarpLink } from "../../api";
import { buildPlaces } from "../places";
import { NewRoomWizard } from "./NewRoomWizard";

vi.mock("../../api", async (importOriginal) => ({ ...(await importOriginal<typeof import("../../api")>()), createRoom: vi.fn(), getEvents: vi.fn(), getMatrix: vi.fn() }));
// The template picture asks the app-wide cache for a map picture; there is no renderer here.
vi.mock("../../world/thumbnailStore", () => ({
  thumbnails: { get: () => undefined, request: () => {}, cancel: () => {}, subscribe: () => () => {}, failure: () => null },
}));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const flush = () => act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));

const header = (id: number, internal: string, location: string, matrix: number, area: number, event: number): HeaderSummary => ({ id, internal_name: internal, location_name: location, matrix_id: matrix, area_data_id: area, event_file_id: event });
// 0: the village (outside), 2: the template room, 3: the header the new room is given to, 4: another room,
// 5: a second outside header, 6: a room that shares the matrix of header 2 (its first door lies beyond the first map).
const headers = [
  header(0, "T01", "Zweiblattdorf", 0, 1, 5),
  header(2, "T01R0201", "Spielerhaus", 2, 4, 8),
  header(3, "NEWROOM", "", 3, 6, 9),
  header(4, "T01R0202", "Nachbarhaus", 4, 4, 10),
  header(5, "T02", "Blumenau", 5, 1, 11),
  header(6, "T01R0203", "Zwillingshaus", 2, 4, 12),
];
const summary = (index: number, hasHeaders: boolean): MatrixSummary => ({ index, name: `m${index}`, width: hasHeaders ? 2 : 1, height: 1, has_headers: hasHeaders, has_altitudes: false, filled: 1 });
const matrices = [summary(0, true), summary(2, false), summary(3, false), summary(4, false), summary(5, true)];
const places = buildPlaces(headers, matrices);
const links: WarpLink[] = [];
const report: RoomReport = { matrix: 131, map: 571, event_file: 20, outside_warp: 3, inside_warp: 0 };

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
  let lockedFiles: Record<number, string>;
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    lockedFiles = {};
    vi.mocked(getMatrix).mockReset().mockImplementation(async (index) => ({ index, name: `m${index}`, width: 1, height: 1, maps: [index === 2 ? 321 : 400 + index] }));
    vi.mocked(getEvents).mockReset().mockImplementation(async (file) => events(file, lockedFiles[file] ?? null));
    vi.mocked(createRoom).mockReset().mockResolvedValue(report);
    onClose.mockReset();
    onCreated.mockReset().mockResolvedValue(undefined);
    onError.mockReset();
    lastTile.mockReset().mockReturnValue({ x: 12, z: 7 });
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  async function show(prefill: { outsideHeader?: number; template?: number } | undefined = { outsideHeader: 0 }, overview = { loaded: true, failed: false }) {
    await act(async () =>
      root.render(
        <NewRoomWizard headers={headers} links={links} counts={{ Map: 600, AreaData: 20 }} matrices={matrices} matricesLoaded={overview.loaded} matricesFailed={overview.failed} shownMatrix={0} places={places} prefill={prefill} lastTile={lastTile} onClose={onClose} onCreated={onCreated} onError={onError} />,
      ),
    );
    await flush();
  }
  const button = (text: string) => [...container.querySelectorAll("button")].find((b) => b.textContent === text);
  const rowButton = (id: string) => [...container.querySelectorAll("button")].find((b) => b.textContent?.startsWith(id));
  const click = (el: Element | null | undefined) => act(async () => void el?.dispatchEvent(new MouseEvent("click", { bubbles: true })));
  const next = () => click(button("Weiter"));
  const field = (label: string) => container.querySelector<HTMLInputElement>(`input[aria-label="${label}"]`)!;
  const current = () => container.querySelector('[aria-current="step"]')?.textContent;
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
    await click(rowButton("0002"));
    await flush();
    await next();
    await click(rowButton("0003"));
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
    expect(container.textContent).toContain("Anlegen lässt sich nicht rückgängig machen.");
    await click(button("Anlegen"));
    expect(createRoom).toHaveBeenCalledOnce();
    const spec = vi.mocked(createRoom).mock.calls[0][0];
    expect(spec).toEqual({
      name: "Neuer Raum",
      source_map: 321,
      clear_buildings: false,
      header: { id: 3, area_data_id: 4, event_file: "New" },
      entrance: { outside_header: 0, x: 12, z: 7, inside_x: 5, inside_z: 6 },
    });
    expect(container.textContent).toContain("Raum „Neuer Raum“ angelegt");
    expect(container.textContent).toContain("Eingang: Warp 3 in Header 0000");
    await click(button("Im Studio zeigen"));
    expect(onCreated).toHaveBeenCalledWith(report, spec);
  });

  it("keeps_the_default_door_inside_and_leaves_the_door_outside_open_without_a_hovered_tile_or_template_warp", async () => {
    lastTile.mockReturnValue(null);
    await show();
    await click(rowButton("0004"));
    await flush();
    await next();
    await click(rowButton("0003"));
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
    await click(rowButton("0005"));
    expect(field("Außen: Kachel x").value).toBe("");
    expect(field("Außen: Kachel z").value).toBe("");
    expect(container.textContent).not.toContain("zuletzt in der Szene");
    expect(container.textContent).toContain("Raster 0005: 64 × 32 Kacheln");
    expect(button("Weiter")?.disabled).toBe(true);
  });

  it("clamps_a_typed_door_outside_into_the_outside_matrix", async () => {
    await show({ outsideHeader: 0 });
    await toEntrance();
    await type(field("Außen: Kachel x"), "999");
    expect(field("Außen: Kachel x").value).toBe("63");
    await type(field("Außen: Kachel z"), "99");
    expect(field("Außen: Kachel z").value).toBe("31");
  });

  it("can_go_on_after_clicking_the_chosen_template_again_or_taking_one_that_shares_its_matrix", async () => {
    await show();
    await click(rowButton("0002"));
    await flush();
    await click(rowButton("0002"));
    await flush();
    expect(button("Weiter")?.disabled).toBe(false);
    await click(rowButton("0006"));
    await flush();
    expect(button("Weiter")?.disabled).toBe(false);
    await next();
    await click(rowButton("0003"));
    await next();
    await next();
    await click(button("Anlegen"));
    expect(vi.mocked(createRoom).mock.calls[0][0].source_map).toBe(321);
  });

  it("says_where_the_inside_door_comes_from_and_when_it_was_limited_to_the_map", async () => {
    await show();
    await click(rowButton("0002"));
    await flush();
    await next();
    await click(rowButton("0003"));
    await next();
    expect(container.textContent).toContain("aus der Vorlage übernommen");
    expect(container.textContent).not.toContain("begrenzt");
    await click(button("Zurück"));
    await click(button("Zurück"));
    await click(rowButton("0006"));
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
    await click(rowButton("0002"));
    await next();
    await click(rowButton("0003"));
    await next();
    await type(field("Innen: Kachel x"), "9");
    expect(field("Innen: Kachel x").value).toBe("9");
    await flush();
    expect(field("Innen: Kachel x").value).toBe("9");
  });

  it("lists_only_rooms_as_templates_and_narrows_them_by_the_search", async () => {
    await show();
    expect(rowButton("0000")).toBeUndefined();
    expect(["0002", "0003", "0004"].map((id) => rowButton(id) !== undefined)).toEqual([true, true, true]);
    await type(field("Vorlage suchen"), "nachbar");
    expect(rowButton("0002")).toBeUndefined();
    expect(rowButton("0004")).toBeDefined();
  });

  it("gates_weiter_on_the_template_and_on_the_header", async () => {
    await show();
    expect(button("Weiter")?.disabled).toBe(true);
    await click(rowButton("0002"));
    await flush();
    expect(button("Weiter")?.disabled).toBe(false);
    await next();
    expect(current()).toBe("2. Header");
    expect(button("Weiter")?.disabled).toBe(true);
    await click(rowButton("0003"));
    expect(button("Weiter")?.disabled).toBe(false);
  });

  it("copies_the_templates_map_from_the_first_cell_of_its_matrix", async () => {
    await show({ outsideHeader: 0, template: 2 });
    expect(getMatrix).toHaveBeenCalledWith(2);
    expect(button("Weiter")?.disabled).toBe(false);
  });

  it("does_not_offer_the_rooms_own_header_as_the_header_outside", async () => {
    await show({ outsideHeader: 0 });
    await click(rowButton("0002"));
    await flush();
    await next();
    // Header 0000 is the prefilled outside header; making it the room's header drops it from the entrance.
    await click(rowButton("0000"));
    await next();
    expect(rowButton("0000")).toBeUndefined();
    expect(button("Weiter")?.disabled).toBe(true);
    expect(container.textContent).toContain("Außen-Header wählen.");
  });

  it("offers_the_prefilled_outside_header_even_when_it_is_a_room", async () => {
    await show({ outsideHeader: 4 });
    await toEntrance();
    expect(rowButton("0004")).toBeDefined();
    await type(field("Außen: Kachel x"), "3");
    await type(field("Außen: Kachel z"), "3");
    await next();
    expect(container.textContent).toContain("Verbindet 0004 Nachbarhaus");
  });

  it("makes_no_entrance_when_connecting_later", async () => {
    await show();
    await toEntrance();
    await click(container.querySelector('input[aria-label="später verbinden"]'));
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
    const later = container.querySelector<HTMLInputElement>('input[aria-label="später verbinden"]')!;
    expect(later.checked).toBe(true);
    expect(later.disabled).toBe(true);
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
    expect(rowButton("0005")?.disabled).toBe(false);
    await click(rowButton("0005"));
    await flush();
    expect(container.textContent).not.toContain("schreibgeschützt");
    const later = container.querySelector<HTMLInputElement>('input[aria-label="später verbinden"]')!;
    expect(later.checked).toBe(false);
    expect(later.disabled).toBe(false);
    await type(field("Außen: Kachel x"), "4");
    await type(field("Außen: Kachel z"), "5");
    await next();
    await click(button("Anlegen"));
    expect(vi.mocked(createRoom).mock.calls[0][0].entrance).toEqual({ outside_header: 5, x: 4, z: 5, inside_x: 5, inside_z: 6 });
  });

  it("makes_no_entrance_when_a_copy_of_a_locked_template_event_file_is_asked_for", async () => {
    lockedFiles[8] = "boom";
    await show();
    await click(rowButton("0002"));
    await flush();
    await next();
    await click(rowButton("0003"));
    await click([...container.querySelectorAll("input[type=radio]")].find((r) => r.parentElement?.textContent === "Kopie von 0008"));
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
        <NewRoomWizard headers={headers} links={links} counts={{ Map: 600, AreaData: 20 }} matrices={[]} matricesLoaded={false} matricesFailed={false} shownMatrix={0} places={unknown} prefill={{ template: 2 }} lastTile={lastTile} onClose={onClose} onCreated={onCreated} onError={onError} />,
      ),
    );
    await flush();
    expect(container.textContent).toContain("Lade Übersicht …");
    expect(rowButton("0002")).toBeUndefined();
    expect(container.textContent).not.toContain("Kein Raum gefunden");
    // The template the request names is still the chosen one.
    await next();
    await click(rowButton("0003"));
    await next();
    expect(container.textContent).toContain("Lade Übersicht …");
    // The only list items left are the four steps: no outside header is offered.
    expect(container.querySelectorAll("li")).toHaveLength(4);
  });

  it("says_the_overview_is_not_available_when_reading_it_failed", async () => {
    await show({ outsideHeader: 0 }, { loaded: false, failed: true });
    expect(container.textContent).toContain("Übersicht nicht verfügbar");
    expect(container.textContent).not.toContain("Lade Übersicht");
    expect(rowButton("0002")).toBeUndefined();
  });

  it("clamps_an_inside_tile_into_the_map", async () => {
    await show();
    await toEntrance();
    await type(field("Innen: Kachel x"), "40");
    expect(field("Innen: Kachel x").value).toBe("31");
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
    expect(container.textContent).toContain("header edit failed: boom");
    expect(button("Im Studio zeigen")).toBeUndefined();
    await click(button("Schließen"));
    expect(onClose).toHaveBeenCalledWith(true);
  });

  it("closes_with_escape_and_reports_that_nothing_was_written", async () => {
    await show();
    await act(async () => void document.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true })));
    expect(onClose).toHaveBeenCalledWith(false);
    expect(createRoom).not.toHaveBeenCalled();
  });
});
