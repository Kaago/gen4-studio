// @vitest-environment jsdom
import i18next from "i18next";
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { getEvents, storyList } from "../../api";
import type { EventsDto, HeaderSummary } from "../../api";
import { resetSpy, withReset } from "../../original/resetTesting";
import { storiesOf } from "../../stories/storiesTesting";
import { EventEditor } from "./EventEditor";
import type { EventEditorContext } from "./EventEditor";

vi.mock("../../api", async (importOriginal) => ({ ...(await importOriginal<typeof import("../../api")>()), getEvents: vi.fn(), storyList: vi.fn() }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const flush = () => act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));
const header = (id: number, event: number): HeaderSummary => ({ id, internal_name: `H${id}`, location_index: 0, kanto: false, location_name: "", matrix_id: 0, area_data_id: 0, event_file_id: event, text_archive_id: 0, script_file_id: 0, level_script_id: 0 });
const events = (): EventsDto => ({ event_file_id: 8, locked: null, bg: [], objects: [], coords: [], warps: [{ index: 0, x: 8, z: 4, dest_header_id: 1, dest_warp_id: 0, height: 0 }] });

describe("the inspector of a door in English", () => {
  let container: HTMLElement;
  let root: Root;
  const context = (): EventEditorContext => ({
    headers: [header(3, 8), header(1, 2)],
    dirty: [],
    family: "Pt",
    revision: 0,
    onChanged: vi.fn(),
    onAdd: vi.fn(),
    onDelete: vi.fn(),
    goTo: vi.fn(),
    selected: 3,
    scriptsAdopted: true,
    onOpenScript: vi.fn(),
    onAddScript: vi.fn(),
    onError: vi.fn(),
    openDialogEditor: vi.fn(),
  });
  beforeEach(async () => {
    await i18next.changeLanguage("en");
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    vi.mocked(getEvents).mockReset().mockImplementation(async () => events());
    vi.mocked(storyList).mockReset().mockResolvedValue(storiesOf([]));
  });
  afterEach(async () => {
    act(() => root.unmount());
    container.remove();
    await i18next.changeLanguage("de");
  });

  it("says_door_and_target", async () => {
    await act(async () => root.render(withReset(resetSpy(), <EventEditor file={8} kind="warp" index={0} context={context()} depth="bauen" />)));
    await flush();
    expect(container.querySelector("header[data-inspector-kind='door'] [data-eyebrow]")?.textContent).toBe("Door");
    const labels = [...container.querySelectorAll("span")].map((s) => s.textContent);
    expect(labels).toContain("Target");
    expect(container.querySelector("button[aria-label='Change target…']")).not.toBeNull();
    expect(container.textContent).not.toMatch(/Tür|Ziel|Ändern/);
  });
});
