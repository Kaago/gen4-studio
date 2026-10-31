// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { getEvents, getObjectSprite, resolveScript, updateEvent } from "../../api";
import type { DataRowDto, EventKind, EventsDto, HeaderSummary, ObjectDto, WarpDto } from "../../api";
import { openSelect, typeInSearch, typeInto } from "../../ui/primitives/testUtils";
import { visibleWords } from "../../ui/visibleText";
import { EventEditor } from "./EventEditor";
import type { EventEditorContext } from "./EventEditor";
import { chooseAction, headAction, headActions } from "./headTesting";
import { MISSING, resetSpy, withReset } from "../../original/resetTesting";
import type { Depth } from "../../ui/frame/shellState";
import { resetSprites } from "../../world/sprites";

vi.mock("../../api", async (importOriginal) => ({ ...(await importOriginal<typeof import("../../api")>()), getEvents: vi.fn(), updateEvent: vi.fn(), resolveScript: vi.fn(), getObjectSprite: vi.fn() }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const flush = () => act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));
const header = (id: number, event: number, script: number): HeaderSummary => ({ id, internal_name: `H${id}`, location_index: 0, kanto: false, location_name: "", matrix_id: 0, area_data_id: 0, event_file_id: event, text_archive_id: 0, script_file_id: script, level_script_id: 0 });
const npc: ObjectDto = { index: 2, local_id: 1, graphics_id: 5, movement_type: 0, script: 4, dir: 1, trainer_type: 0, sight: 0, x: 3, z: 4, y: 0 };
const events = (): EventsDto => ({ event_file_id: 8, locked: null, bg: [], objects: [{ index: 0, local_id: 0, graphics_id: 0, movement_type: 0, script: 0, dir: 0, trainer_type: 0, sight: 0, x: 0, z: 0, y: 0 }, { ...npc, index: 1 }, npc], coords: [], warps: [] });

describe("EventEditor scripts", () => {
  let container: HTMLElement;
  let root: Root;
  const onOpenScript = vi.fn();
  const onAddScript = vi.fn();
  const onError = vi.fn();
  const onChanged = vi.fn();
  let reset = resetSpy();
  const context = (over: Partial<EventEditorContext> = {}): EventEditorContext => ({
    // Header 3 (script file 40) and 9 (script file 77) share event file 8.
    headers: [header(3, 8, 40), header(9, 8, 77), header(1, 2, 5)],
    dirty: [],
    family: "Pt",
    revision: 0,
    onChanged,
    onAdd: vi.fn(),
    onDelete: vi.fn(),
    goTo: vi.fn(),
    selected: 9,
    scriptsAdopted: true,
    onOpenScript,
    onAddScript,
    onError,
    ...over,
  });
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    vi.mocked(getEvents).mockReset().mockImplementation(async () => events());
    vi.mocked(updateEvent).mockReset().mockResolvedValue(undefined);
    vi.mocked(resolveScript).mockReset();
    vi.mocked(getObjectSprite).mockReset().mockResolvedValue(null);
    // The sprite cache outlives a test: each one asks the backend anew.
    resetSprites();
    for (const fn of [onOpenScript, onAddScript, onError, onChanged]) fn.mockReset();
    reset = resetSpy();
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });
  async function show(ctx = context(), kind: EventKind = "object", index = 2, depth: Depth = "bauen") {
    await act(async () => root.render(withReset(reset, <EventEditor file={8} kind={kind} index={index} context={ctx} depth={depth} />)));
    await flush();
  }
  const field = (label: string) => container.querySelector<HTMLInputElement>(`input[aria-label='${label}']`)!;
  const typeAndBlur = async (label: string, text: string) => {
    await act(async () => void field(label).focus());
    typeInto(field(label), text);
    await act(async () => void field(label).blur());
    await flush();
  };
  const select = (label: string) => container.querySelector<HTMLElement>(`[role=combobox][aria-label='${label}']`)!;
  const options = () => [...document.querySelectorAll<HTMLElement>("[role=option]")];
  const button = (text: string) => [...container.querySelectorAll("button")].find((b) => b.textContent === text);
  const click = (el: Element | undefined) => act(async () => void (el as HTMLElement).click());
  // The labels of the form's rows (`InspectorRow`: the label, then the control), in order.
  const rowLabels = () => [...container.querySelectorAll("div > span:first-child")].filter((s) => s.nextElementSibling?.tagName === "DIV" && s.textContent !== "").map((s) => s.textContent);

  it("opens_the_script_of_the_event_through_resolve_script_of_the_selected_header", async () => {
    vi.mocked(resolveScript).mockResolvedValue({ kind: "file", file: 0, line: 4 });
    await show();
    await click(button("Öffnen"));
    // Header 9 is the selected one of the two that use event file 8.
    expect(resolveScript).toHaveBeenCalledWith(9, 4);
    expect(onOpenScript).toHaveBeenCalledWith(0, 4);
    expect(onError).not.toHaveBeenCalled();
  });

  it("falls_back_to_the_first_header_of_the_event_file_when_the_selected_one_uses_another", async () => {
    vi.mocked(resolveScript).mockResolvedValue({ kind: "file", file: 40, line: 1 });
    await show(context({ selected: 1 }));
    await click(button("Öffnen"));
    expect(resolveScript).toHaveBeenCalledWith(3, 4);
  });

  it("reports_candidates_for_an_ambiguous_script_and_a_missing_one", async () => {
    vi.mocked(resolveScript).mockResolvedValueOnce({ kind: "ambiguous", candidates: [1, 12] }).mockResolvedValueOnce({ kind: "unknown" });
    await show();
    await click(button("Öffnen"));
    expect(onError).toHaveBeenLastCalledWith("Script 4 liegt in mehreren Dateien: 0001, 0012");
    await click(button("Öffnen"));
    expect(onError).toHaveBeenLastCalledWith("Script 4 ist nicht auffindbar");
    expect(onOpenScript).not.toHaveBeenCalled();
  });

  it("reports_a_refused_resolve", async () => {
    vi.mocked(resolveScript).mockRejectedValue({ code: "error", message: "kaputt", asset: null });
    await show();
    await click(button("Öffnen"));
    expect(onError).toHaveBeenCalledWith({ code: "error", message: "kaputt", asset: null });
  });

  it("adds_a_script_to_the_headers_script_file_and_writes_its_number_into_the_event", async () => {
    onAddScript.mockResolvedValue(12);
    await show();
    await click(button("Neues Script"));
    await flush();
    expect(onAddScript).toHaveBeenCalledWith(77);
    // The number goes through the edit path: an event patch, and the dirty count follows.
    expect(updateEvent).toHaveBeenCalledWith(8, "object", 2, { script: 12 });
    expect(onChanged).toHaveBeenCalledWith(8);
    expect(onOpenScript).toHaveBeenCalledWith(77);
  });

  it("does_not_open_the_script_file_when_the_event_write_was_refused", async () => {
    onAddScript.mockResolvedValue(12);
    vi.mocked(updateEvent).mockRejectedValue({ code: "error", message: "gesperrt", asset: null });
    await show();
    await click(button("Neues Script"));
    await flush();
    expect(updateEvent).toHaveBeenCalledTimes(1);
    expect(onError).toHaveBeenCalledWith({ code: "error", message: "gesperrt", asset: null });
    expect(onOpenScript).not.toHaveBeenCalled();
  });

  it("leaves_the_event_alone_when_adding_the_script_failed", async () => {
    onAddScript.mockResolvedValue(null);
    await show();
    await click(button("Neues Script"));
    await flush();
    expect(updateEvent).not.toHaveBeenCalled();
    expect(onOpenScript).not.toHaveBeenCalled();
  });

  it("shows_a_hint_and_calls_nothing_while_scripts_are_not_adopted", async () => {
    await show(context({ scriptsAdopted: false }));
    expect(container.textContent).toContain("Erst Scripts übernehmen (Panel „Script“)");
    expect((button("Öffnen") as HTMLButtonElement).disabled).toBe(true);
    expect((button("Neues Script") as HTMLButtonElement).disabled).toBe(true);
    await click(button("Öffnen"));
    await click(button("Neues Script"));
    expect(resolveScript).not.toHaveBeenCalled();
    expect(onAddScript).not.toHaveBeenCalled();
  });

  it("has_the_buttons_only_next_to_the_script_field", async () => {
    await show();
    expect([...container.querySelectorAll("button")].filter((b) => b.textContent === "Öffnen")).toHaveLength(1);
  });

  it("does_not_add_scripts_to_a_locked_event_file", async () => {
    vi.mocked(getEvents).mockImplementation(async () => ({ ...events(), locked: "uses common data" }));
    await show();
    expect((button("Neues Script") as HTMLButtonElement).disabled).toBe(true);
    expect((button("Öffnen") as HTMLButtonElement).disabled).toBe(false);
  });

  it("says_Ungespeichert_while_the_event_file_has_unsaved_changes_and_not_for_another_file", async () => {
    await show(context({ dirty: [{ kind: "Event", index: 9 }, { kind: "Map", index: 8 }] }));
    expect(container.textContent).not.toContain("Ungespeichert");
    await show(context({ dirty: [{ kind: "Event", index: 8 }] }));
    expect(container.textContent).toContain("Ungespeichert");
  });

  it("names_the_event_by_its_kind_its_title_and_its_number_in_the_event_file", async () => {
    await show();
    expect(container.querySelector("header[data-inspector-kind='figure'] [data-eyebrow]")?.textContent).toBe("Figur");
    expect(container.querySelector("h2")?.textContent).toBe("Objekt 2 · ID 1");
    expect(container.querySelector("[data-ident]")?.textContent).toBe("Objekt 2 · Eventdatei 0008");
    // The other classes have their own words.
    vi.mocked(getEvents).mockImplementation(async () => ({ ...events(), warps: [{ index: 0, x: 8, z: 4, dest_header_id: 1, dest_warp_id: 0, height: 0 }] }));
    await show(context(), "warp", 0);
    expect(container.querySelector("header[data-inspector-kind='door'] [data-eyebrow]")?.textContent).toBe("Tür");
    expect(container.querySelector("[data-ident]")?.textContent).toBe("Warp 0 · Eventdatei 0008");
  });

  it("deletes_from_the_head_menu", async () => {
    const onDelete = vi.fn();
    await show(context({ onDelete }));
    // „Löschen“ is no button in the form any more: it is the head's action, with its key.
    expect(button("Löschen")).toBeUndefined();
    expect(headActions(container)).toEqual(["LöschenEntf", "Eventdatei auf Original zurücksetzen…"]);
    chooseAction(container, "Löschen");
    expect(onDelete).toHaveBeenCalledWith(8, "object", 2);
  });

  it("head_offers_reset_of_the_event_file", async () => {
    await show();
    // Not the event alone: the whole event file it is in (the reset sets the file's asset).
    chooseAction(container, "Eventdatei auf Original zurücksetzen…");
    expect(reset.open).toHaveBeenCalledWith({ kind: "event_file", index: 8 });
    await act(async () => root.render(null));
    reset = resetSpy(MISSING);
    await show();
    expect(headActions(container)).toEqual(["LöschenEntf", "Eventdatei auf Original zurücksetzen…Basis-ROM fehlt"]);
  });

  it("changes_the_facing_through_a_select_and_writes_only_that_field", async () => {
    await show();
    expect(select("Blickrichtung").textContent).toContain("Unten");
    openSelect(select("Blickrichtung"));
    expect(options().map((o) => o.textContent)).toEqual(["Oben", "Unten", "Links", "Rechts"]);
    await act(async () => void options()[2]!.click());
    await flush();
    expect(updateEvent).toHaveBeenCalledWith(8, "object", 2, { dir: 2 });
    expect(onChanged).toHaveBeenCalledWith(8);
  });

  it("writes_a_typed_number_on_blur_and_refuses_one_outside_the_stored_range_without_clamping", async () => {
    await show();
    await typeAndBlur("x (Kachel)", "9");
    expect(updateEvent).toHaveBeenCalledWith(8, "object", 2, { x: 9 });
    await typeAndBlur("z (Kachel)", "70000");
    expect(container.textContent).toContain("höchstens 65535");
    expect(field("z (Kachel)").value).toBe("4");
    expect(updateEvent).toHaveBeenCalledTimes(1);
  });

  it("shows_a_locked_event_file_with_its_reason_and_changes_nothing", async () => {
    vi.mocked(getEvents).mockImplementation(async () => ({ ...events(), locked: "uses common data" }));
    await show();
    expect(container.textContent).toContain("Gesperrt");
    expect(container.textContent).toContain("schreibgeschützt: uses common data");
    expect(field("x (Kachel)").disabled).toBe(true);
    expect(select("Blickrichtung").hasAttribute("data-disabled") || select("Blickrichtung").getAttribute("aria-disabled") === "true").toBe(true);
    const remove = headAction(container, "Löschen");
    expect(remove?.getAttribute("aria-disabled") === "true" || remove?.hasAttribute("data-disabled")).toBe(true);
  });

  describe("a warp", () => {
    const warp = (dest: number): WarpDto => ({ index: 0, x: 8, z: 4, dest_header_id: dest, dest_warp_id: 0, height: 0 });
    const withWarp = (dest: number) => vi.mocked(getEvents).mockImplementation(async () => ({ ...events(), warps: [warp(dest)] }));

    it("shows_the_destination_as_a_reference_that_leads_to_the_header", async () => {
      const goTo = vi.fn();
      withWarp(1);
      await show(context({ goTo }), "warp", 0);
      expect(container.querySelector("h2")?.textContent).toBe("Warp 0 → Ort 0001");
      await click(button("H10001 · H1"));
      expect(goTo).toHaveBeenCalledWith({ kind: "header", id: 1 });
    });

    it("names_the_destination_in_the_head_as_the_world_tree_does_in_building", async () => {
      // The row in „In dieser Szene“ says „Warp 0 → Haus 1“: the head says the same, not the code of the header.
      withWarp(1);
      const identify = (h: HeaderSummary) => ({ kind: "room" as const, name: h.id === 1 ? "Haus 1" : `Raum ${h.id}`, place: "Zweiblattdorf" });
      await show(context({ identify }), "warp", 0);
      expect(container.querySelector("h2")?.textContent).toBe("Warp 0 → Haus 1");
      await show(context({ identify }), "warp", 0, "tuefteln");
      expect(container.querySelector("h2")?.textContent).toBe("Warp 0 → Header 0001 (H1)");
    });

    it("picks_another_destination_header_in_the_picker_by_number", async () => {
      withWarp(1);
      await show(context(), "warp", 0);
      await click(container.querySelector("button[aria-label='Ziel ändern…']") ?? undefined);
      typeInSearch(document.querySelector<HTMLInputElement>("input[role=combobox]")!, "0009");
      expect(options().map((o) => o.textContent)).toEqual(["H90009"]);
      await click(options()[0]);
      await flush();
      expect(updateEvent).toHaveBeenCalledWith(8, "warp", 0, { dest_header_id: 9 });
    });

    it("says_no_raster_matrix_or_header_in_building", async () => {
      withWarp(1);
      await show(context(), "warp", 0);
      expect(visibleWords(container)).not.toMatch(/Raster|Matrix|Header/);
      // The figure the Gate selects.
      vi.mocked(getEvents).mockImplementation(async () => events());
      await show();
      expect(visibleWords(container)).not.toMatch(/Raster|Matrix|Header/);
    });

    it("keeps_the_technical_words_in_tinkering", async () => {
      withWarp(1);
      await show(context(), "warp", 0, "tuefteln");
      expect(container.querySelector("h2")?.textContent).toBe("Warp 0 → Header 0001 (H1)");
      await click(container.querySelector("button[aria-label='Zielheader ändern…']") ?? undefined);
      expect(document.querySelector("[role=dialog]")?.textContent).toContain("Zielheader wählen");
    });

    it("names_the_destination_ziel_in_building", async () => {
      withWarp(1);
      await show(context(), "warp", 0);
      await click(container.querySelector("button[aria-label='Ziel ändern…']") ?? undefined);
      expect(document.querySelector("[role=dialog]")?.textContent).toContain("Ziel wählen");
      expect(document.querySelector("[role=dialog]")?.textContent).not.toContain("Zielheader");
    });

    it("says_a_door_to_a_header_that_does_not_exist_leads_nowhere_and_how_to_fix_it", async () => {
      withWarp(9999);
      await show(context(), "warp", 0);
      expect(container.textContent).toContain("Die Tür führt ins Leere. Wähle ein Ziel oder lösche die Tür.");
      expect(button("unbekannt (9999)")).toBeUndefined();
    });
  });

  describe("the trainer field of an object", () => {
    const trainerRow = (index: number, name: string, cls: string | null): DataRowDto => ({
      index,
      name,
      tech: String(index).padStart(4, "0"),
      locked: cls === null ? "placeholder for the sight script slot" : null,
      extra: cls === null ? null : { class: 2, class_name: cls, level_max: 5, party_count: 1, places: [] },
    });
    const trainers = [trainerRow(0, "", "Klasse 0"), trainerRow(356, "Sebastian", "Teenager"), trainerRow(928, "Jens", "Wanderer"), trainerRow(929, "", null), trainerRow(930, "Mia", "Göre")];
    const withObject = (over: Partial<ObjectDto>) => vi.mocked(getEvents).mockImplementation(async () => ({ ...events(), objects: [{ ...npc, index: 0, ...over }] }));
    const trainerChip = () => container.querySelector<HTMLButtonElement>("button[data-trainer]");
    const trainerRowText = () => [...container.querySelectorAll("[data-field='trainer']")].map((e) => e.textContent).join("");

    it("shows_trainer_356_for_script_3355_as_a_reference_into_the_pokemon_area", async () => {
      const goTo = vi.fn();
      const needTrainers = vi.fn();
      withObject({ script: 3355, trainer_type: 1, sight: 5 });
      await show(context({ goTo, trainers, needTrainers }), "object", 0);
      expect(needTrainers).toHaveBeenCalled();
      expect(trainerChip()?.textContent).toContain("Teenager Sebastian");
      expect(trainerChip()?.getAttribute("data-trainer")).toBe("356");
      await click(trainerChip()!);
      expect(goTo).toHaveBeenCalledWith({ kind: "trainer", index: 356 });
    });

    it("names_a_trainer_figure_in_the_head_as_the_scene_hierarchy_does", async () => {
      withObject({ script: 3927, trainer_type: 1, sight: 3 });
      await show(context({ trainers }), "object", 0);
      expect(container.querySelector("h2")?.textContent).toBe("Objekt 0 · Wanderer Jens");
    });

    it("says_kein_Trainer_for_a_script_outside_3000_to_4999", async () => {
      withObject({ script: 4 });
      await show(context({ trainers }), "object", 0);
      expect(trainerChip()).toBeNull();
      expect(trainerRowText()).toContain("kein Trainer");
    });

    it("names_a_partner_from_5000_on", async () => {
      withObject({ script: 5355 });
      await show(context({ trainers }), "object", 0);
      expect(trainerChip()?.textContent).toContain("Teenager Sebastian (Partner)");
    });

    it("picking_a_trainer_sets_the_script_and_turns_the_sightline_on_in_one_write_when_type_and_sight_are_0", async () => {
      withObject({ script: 4, trainer_type: 0, sight: 0 });
      await show(context({ trainers }), "object", 0);
      await click(container.querySelector("button[aria-label='Trainer ändern…']") ?? undefined);
      // Neither the dummy 0 nor the seen slot 929 is offered.
      expect(options().map((o) => o.textContent)).toEqual(["Teenager Sebastian0356", "Wanderer Jens0928", "Göre Mia0930"]);
      typeInSearch(document.querySelector<HTMLInputElement>("input[role=combobox]")!, "Mia");
      await click(options()[0]);
      await flush();
      expect(updateEvent).toHaveBeenCalledTimes(1);
      expect(updateEvent).toHaveBeenCalledWith(8, "object", 0, { script: 3929, trainer_type: 1, sight: 3 });
      expect(onChanged).toHaveBeenCalledWith(8);
      expect(trainerChip()?.textContent).toContain("Göre Mia");
    });

    it("picking_a_trainer_keeps_a_sightline_that_is_already_set", async () => {
      withObject({ script: 3355, trainer_type: 1, sight: 5 });
      await show(context({ trainers }), "object", 0);
      await click(container.querySelector("button[aria-label='Trainer ändern…']") ?? undefined);
      await click(options().find((o) => o.textContent?.startsWith("Wanderer Jens")));
      await flush();
      expect(updateEvent).toHaveBeenCalledWith(8, "object", 0, { script: 3927 });
    });

    it("picking_another_trainer_for_a_partner_keeps_the_partner_range", async () => {
      withObject({ script: 5355, trainer_type: 1, sight: 2 });
      await show(context({ trainers }), "object", 0);
      await click(container.querySelector("button[aria-label='Trainer ändern…']") ?? undefined);
      await click(options().find((o) => o.textContent?.startsWith("Göre Mia")));
      await flush();
      expect(updateEvent).toHaveBeenCalledWith(8, "object", 0, { script: 5929 });
      expect(trainerChip()?.textContent).toContain("Göre Mia (Partner)");
    });

    it("shows_the_sightline_fields_of_task_2", async () => {
      withObject({ script: 3355, trainer_type: 1, sight: 5 });
      await show(context({ trainers }), "object", 0);
      expect(select("Sichtlinie").textContent).toContain("Trainer sieht dich");
      expect(field("Sichtweite").value).toBe("5");
    });

    it("has_no_trainer_field_on_other_classes_nor_in_Diamond_and_Pearl", async () => {
      withObject({ script: 3355 });
      await show(context({ trainers, family: "Dp" }), "object", 0);
      expect(trainerRowText()).toBe("");
      vi.mocked(getEvents).mockImplementation(async () => ({ ...events(), coords: [{ index: 0, script: 3355, x: 1, z: 1, width: 1, length: 1 }] }));
      await show(context({ trainers }), "coord", 0);
      expect(trainerRowText()).toBe("");
    });
  });

  it("shows_the_section_Technisch_and_the_Dez_Hex_switch_only_in_tuefteln", async () => {
    const technical = () => [...container.querySelectorAll<HTMLButtonElement>("button[aria-expanded]")].find((b) => b.textContent === "Technisch");
    const hexSwitch = () => [...container.querySelectorAll("button")].some((b) => b.textContent === "Hex");
    await show(context(), "object", 2, "bauen");
    expect(technical()).toBeUndefined();
    expect(hexSwitch()).toBe(false);
    await show(context(), "object", 2, "tuefteln");
    expect(technical()?.getAttribute("aria-expanded")).toBe("false");
    expect(hexSwitch()).toBe(true);
    await act(async () => void technical()?.click());
    expect(container.textContent).toContain("Eventdatei 0008, Index 2");
  });
  describe("in Erzählen", () => {
    const trainers: DataRowDto[] = [{ index: 356, name: "Sebastian", tech: "0356", locked: null, extra: { class: 2, class_name: "Teenager", level_max: 5, party_count: 1, places: [] } }];

    it("erzaehlen_shows_look_movement_facing_and_trainer_only", async () => {
      vi.mocked(getEvents).mockImplementation(async () => ({ ...events(), objects: [{ ...npc, index: 0, script: 3355, trainer_type: 1, sight: 5 }] }));
      await show(context({ trainers }), "object", 0, "erzaehlen");
      expect(rowLabels()).toEqual(["Aussehen", "Bewegung", "Blickrichtung", "Trainer", "y"]);
      expect(field("x (Kachel)")).toBeNull();
      expect(field("Lokale ID")).toBeNull();
      expect(container.textContent).not.toContain("Lokale ID");
      // No number of the script, no script buttons, no Dez | Hex.
      expect(button("Öffnen")).toBeUndefined();
      expect(button("Neues Script")).toBeUndefined();
      expect(button("Hex")).toBeUndefined();
      expect(container.textContent).not.toContain("Erst Scripts übernehmen");
      expect(container.querySelector("button[data-trainer]")?.textContent).toContain("Teenager Sebastian");
      // The movement in words; a stored value without one is „Bewegung N“.
      expect(select("Bewegung").textContent).toContain("Steht still");
      openSelect(select("Bewegung"));
      await click(options().find((o) => o.textContent === "Geht umher"));
      await flush();
      expect(updateEvent).toHaveBeenCalledWith(8, "object", 0, { movement_type: 3 });
      // A new editor for the figure as it is stored now.
      await act(async () => root.render(null));
      vi.mocked(getEvents).mockImplementation(async () => ({ ...events(), objects: [{ ...npc, index: 0, movement_type: 999 }] }));
      await show(context({ trainers }), "object", 0, "erzaehlen");
      expect(select("Bewegung").textContent).toContain("Bewegung 999");
    });

    it("erzaehlen_picks_the_look_of_a_figure_by_its_sprite", async () => {
      await show(context(), "object", 2, "erzaehlen");
      expect(container.querySelector("[data-figure-look='5']")).not.toBeNull();
      await click(container.querySelector("button[aria-label='Aussehen ändern…']") ?? undefined);
      expect(document.querySelector("[role=dialog]")?.textContent).toContain("Aussehen wählen");
      typeInSearch(document.querySelector<HTMLInputElement>("input[role=combobox]")!, "12");
      await click(options()[0]);
      await flush();
      expect(updateEvent).toHaveBeenCalledWith(8, "object", 2, { graphics_id: 12 });
      // The sprites are asked for with the figure's facing.
      expect(getObjectSprite).toHaveBeenCalledWith(5, 1);
    });

    it("erzaehlen_shows_a_door_as_target_and_door", async () => {
      const goTo = vi.fn();
      const destination: EventsDto = { event_file_id: 2, locked: null, bg: [], objects: [], coords: [], warps: [{ index: 0, x: 6, z: 10, dest_header_id: 3, dest_warp_id: 0, height: 0 }, { index: 1, x: 7, z: 11, dest_header_id: 3, dest_warp_id: 0, height: 0 }] };
      vi.mocked(getEvents).mockImplementation(async (file) => (file === 2 ? destination : { ...events(), warps: [{ index: 0, x: 8, z: 4, dest_header_id: 1, dest_warp_id: 1, height: 0 }] }));
      await show(context({ goTo, family: "Hgss" }), "warp", 0, "erzaehlen");
      expect(rowLabels()).toEqual(["Ziel", "Tür"]);
      await click(button("H10001 · H1"));
      expect(goTo).toHaveBeenCalledWith({ kind: "header", id: 1 });
      // The door of the destination's event file, by its number and tile.
      expect(container.querySelector("[data-warp-field]")?.textContent).toContain("Tür 1 · x 7, z 11");
      await click(container.querySelector("button[aria-label='Tür ändern…']") ?? undefined);
      expect(document.querySelector("[role=dialog]")?.textContent).toContain("Tür wählen");
      expect(options().map((o) => o.textContent)).toEqual(["Tür 0 · x 6, z 10", "Tür 1 · x 7, z 11"]);
      await click(options()[0]);
      await flush();
      expect(updateEvent).toHaveBeenCalledWith(8, "warp", 0, { dest_warp_id: 0 });
    });

    it("erzaehlen_tells_sign_and_trigger_what_comes_later", async () => {
      vi.mocked(getEvents).mockImplementation(async () => ({
        ...events(),
        coords: [{ index: 0, script: 9000, x: 1, z: 1, width: 1, length: 1 }],
        bg: [{ index: 0, script: 7, event_type: 0, x: 1, z: 1, y: 0 }],
      }));
      await show(context(), "coord", 0, "erzaehlen");
      expect(container.textContent).toContain("Geschichte und Schritt kommen mit 3d.7.");
      expect(rowLabels()).toEqual([]);
      // Adding events and the hint how to move them stay.
      expect(container.textContent).toContain("Ziehen in der 3D-Ansicht verschiebt das Event");
      await show(context(), "bg", 0, "erzaehlen");
      expect(container.textContent).toContain("Der Text des Schilds kommt mit 3d.5.");
      expect(rowLabels()).toEqual(["y"]);
    });

    it("bauen_and_tuefteln_show_every_field_as_before", async () => {
      for (const depth of ["bauen", "tuefteln"] as const) {
        await show(context({ trainers }), "object", 2, depth);
        expect(rowLabels()).toEqual(["x (Kachel)", "z (Kachel)", "Lokale ID", "Grafik", "Bewegung", "Blickrichtung", "Script", "Trainer", "Sichtlinie", "Sichtweite", "y"]);
        expect(field("Bewegung")).not.toBeNull();
      }
    });
  });
});
