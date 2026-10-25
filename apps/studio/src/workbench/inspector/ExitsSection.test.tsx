// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { getEvents } from "../../api";
import type { EventsDto, HeaderSummary, WarpDto } from "../../api";
import { ExitsSection } from "./ExitsSection";

vi.mock("../../api", async (importOriginal) => ({ ...(await importOriginal<typeof import("../../api")>()), getEvents: vi.fn() }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const flush = () => act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));

const header = (id: number, internal: string, location: string, event: number): HeaderSummary => ({ id, internal_name: internal, location_index: 0, kanto: false, location_name: location, matrix_id: 0, area_data_id: 0, event_file_id: event, text_archive_id: 0, script_file_id: 0, level_script_id: 0 });
const headers = [header(411, "T01", "Zweiblattdorf", 5), header(414, "T01R0201", "Zweiblattdorf", 8)];
const warp = (index: number, x: number, z: number, dest: number): WarpDto => ({ index, x, z, dest_header_id: dest, dest_warp_id: 0, height: 0 });
const events = (warps: WarpDto[], locked: string | null = null): EventsDto => ({ event_file_id: 5, locked, bg: [], objects: [], coords: [], warps });

describe("ExitsSection", () => {
  let container: HTMLElement;
  let root: Root;
  const goTo = vi.fn();
  const onOpenWizard = vi.fn();
  const onAddWarp = vi.fn();
  const onError = vi.fn();
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    vi.mocked(getEvents).mockReset().mockResolvedValue(events([warp(0, 8, 4, 414), warp(1, 2, 3, 9999)]));
    for (const fn of [goTo, onOpenWizard, onAddWarp, onError]) fn.mockReset();
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  async function show(revision = 0) {
    await act(async () => root.render(<ExitsSection header={headers[0]} headers={headers} revision={revision} goTo={goTo} onOpenWizard={onOpenWizard} onAddWarp={onAddWarp} onError={onError} depth="bauen" />));
    await flush();
  }
  const button = (text: string, within: ParentNode = container) => [...within.querySelectorAll("button")].find((b) => b.textContent === text);
  const click = (el: Element | null | undefined) => act(async () => void el?.dispatchEvent(new MouseEvent("click", { bubbles: true })));
  const rows = () => [...container.querySelectorAll("li")];

  it("lists_one_row_per_warp_with_its_position_and_destination", async () => {
    await show();
    expect(container.textContent).toContain("Ausgänge");
    expect(getEvents).toHaveBeenCalledWith(5);
    // The destination is a reference: location name, then identifier; a header that does not exist is named as such.
    expect(rows().map((r) => r.textContent?.replace(/Bearbeiten$/, ""))).toEqual(["Warp 0 (x 8, z 4)Zweiblattdorf0414 · T01R0201", "Warp 1 (x 2, z 3)unbekannt (9999)"]);
  });

  it("counts_the_exits_in_the_head_and_folds_the_list_away_without_forgetting_it", async () => {
    await show();
    const head = container.querySelector<HTMLButtonElement>("h3 button")!;
    expect(head.textContent).toBe("Eingänge / Ausgänge2");
    expect(head.getAttribute("aria-expanded")).toBe("true");
    await click(head);
    expect(head.getAttribute("aria-expanded")).toBe("false");
    expect(rows()).toHaveLength(0);
    await click(head);
    expect(rows()).toHaveLength(2);
    expect(getEvents).toHaveBeenCalledTimes(1);
  });

  it("keeps_the_list_folded_when_the_editor_is_mounted_again_as_after_a_hover", async () => {
    await show();
    await click(container.querySelector<HTMLButtonElement>("h3 button"));
    act(() => root.render(null));
    await show();
    expect(container.querySelector<HTMLButtonElement>("h3 button")!.getAttribute("aria-expanded")).toBe("false");
    expect(rows()).toHaveLength(0);
  });

  it("goes_to_the_destination_header_and_not_to_an_unknown_one", async () => {
    await show();
    await click(rows()[0]!.querySelector("button"));
    expect(goTo).toHaveBeenCalledWith({ kind: "header", id: 414 });
    // Nothing to follow for a header that is not there: only "Bearbeiten" is left in the row.
    expect([...rows()[1]!.querySelectorAll("button")].map((b) => b.textContent)).toEqual(["Bearbeiten"]);
  });

  it("opens_the_warp_in_the_event_editor", async () => {
    await show();
    await click(button("Bearbeiten", rows()[1]));
    expect(goTo).toHaveBeenCalledWith({ kind: "event", eventFile: 5, eventKind: "warp", index: 1, header: 411 });
  });

  it("says_so_when_there_are_no_exits", async () => {
    vi.mocked(getEvents).mockResolvedValue(events([]));
    await show();
    expect(container.textContent).toContain("Keine Ausgänge.");
    expect(rows()).toHaveLength(0);
  });

  it("opens_the_room_wizard_with_this_header_as_the_one_outside", async () => {
    await show();
    await click(button("Raum hier anbinden…"));
    expect(onOpenWizard).toHaveBeenCalledWith({ kind: "room", prefill: { outsideHeader: 411 } });
  });

  it("adds_a_warp", async () => {
    await show();
    await click(button("Warp hinzufügen"));
    expect(onAddWarp).toHaveBeenCalledOnce();
  });

  it("disables_adding_a_warp_with_the_reason_when_the_event_file_is_locked", async () => {
    vi.mocked(getEvents).mockResolvedValue(events([], "17 bytes read"));
    await show();
    const add = button("Warp hinzufügen")!;
    expect(add.disabled).toBe(true);
    expect(add.title).toBe("schreibgeschützt: 17 bytes read");
  });

  it("reads_the_event_file_again_when_the_revision_changes", async () => {
    await show(0);
    expect(getEvents).toHaveBeenCalledTimes(1);
    vi.mocked(getEvents).mockResolvedValue(events([warp(0, 8, 4, 414)]));
    await show(1);
    expect(getEvents).toHaveBeenCalledTimes(2);
    expect(rows()).toHaveLength(1);
  });

  it("reports_a_failed_read", async () => {
    vi.mocked(getEvents).mockRejectedValue(new Error("boom"));
    await show();
    expect(onError).toHaveBeenCalledOnce();
    expect(container.textContent).not.toContain("Keine Ausgänge.");
  });
});
