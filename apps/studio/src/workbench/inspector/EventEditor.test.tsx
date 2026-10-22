// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { getEvents, resolveScript, updateEvent } from "../../api";
import type { EventsDto, HeaderSummary, ObjectDto } from "../../api";
import { EventEditor } from "./EventEditor";
import type { EventEditorContext } from "./EventEditor";

vi.mock("../../api", async (importOriginal) => ({ ...(await importOriginal<typeof import("../../api")>()), getEvents: vi.fn(), updateEvent: vi.fn(), resolveScript: vi.fn() }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const flush = () => act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));
const header = (id: number, event: number, script: number): HeaderSummary => ({ id, internal_name: `H${id}`, location_name: "", matrix_id: 0, area_data_id: 0, event_file_id: event, text_archive_id: 0, script_file_id: script, level_script_id: 0 });
const npc: ObjectDto = { index: 2, local_id: 1, graphics_id: 5, movement_type: 0, script: 4, dir: 1, x: 3, z: 4, y: 0 };
const events = (): EventsDto => ({ event_file_id: 8, locked: null, bg: [], objects: [{ index: 0, local_id: 0, graphics_id: 0, movement_type: 0, script: 0, dir: 0, x: 0, z: 0, y: 0 }, { ...npc, index: 1 }, npc], coords: [], warps: [] });

describe("EventEditor scripts", () => {
  let container: HTMLElement;
  let root: Root;
  const onOpenScript = vi.fn();
  const onAddScript = vi.fn();
  const onError = vi.fn();
  const onChanged = vi.fn();
  const context = (over: Partial<EventEditorContext> = {}): EventEditorContext => ({
    // Header 3 (script file 40) and 9 (script file 77) share event file 8.
    headers: [header(3, 8, 40), header(9, 8, 77), header(1, 2, 5)],
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
    for (const fn of [onOpenScript, onAddScript, onError, onChanged]) fn.mockReset();
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });
  async function show(ctx = context()) {
    await act(async () => root.render(<EventEditor file={8} kind="object" index={2} context={ctx} />));
    await flush();
  }
  const button = (text: string) => [...container.querySelectorAll("button")].find((b) => b.textContent === text);
  const click = (el: Element | undefined) => act(async () => void (el as HTMLElement).click());

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
});
