// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { getEvents } from "../../api";
import type { EventsDto, HeaderSummary } from "../../api";
import { WarpField } from "./WarpField";

vi.mock("../../api", async (importOriginal) => ({ ...(await importOriginal<typeof import("../../api")>()), getEvents: vi.fn() }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const flush = () => act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));
const header = (id: number, event_file_id: number, deleted = false): HeaderSummary => ({
  id,
  internal_name: `T01R0${id}`,
  location_name: "Zweiblattdorf",
  location_index: 2,
  kanto: false,
  matrix_id: 0,
  area_data_id: 0,
  event_file_id,
  text_archive_id: 0,
  script_file_id: 0,
  level_script_id: 0,
  deleted,
});
const events = (event_file_id: number, deleted = false): EventsDto => ({
  event_file_id,
  locked: null,
  bg: [],
  objects: [],
  coords: [],
  warps: [0, 1].map((index) => ({ index, x: 3 + index, z: 7, dest_header_id: 411, dest_warp_id: 0, height: 0 })),
  deleted,
});

describe("WarpField", () => {
  let container: HTMLElement;
  let root: Root;
  const onCommit = vi.fn();
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    onCommit.mockReset();
    vi.mocked(getEvents).mockReset().mockImplementation(async (file: number) => events(file, file === 91));
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  const show = async (dest: HeaderSummary) => {
    await act(async () => root.render(<WarpField destHeader={dest} value={0} onCommit={onCommit} onError={() => {}} />));
    await flush();
  };
  const change = () => container.querySelector<HTMLButtonElement>('button[aria-label="Tür ändern…"]')!;
  const hint = () => container.textContent;

  it("deleted_headers_are_not_offered", async () => {
    // A living destination offers its doors.
    await show(header(414, 90));
    expect(change().disabled).toBe(false);
    await act(async () => change().click());
    expect([...document.querySelectorAll("[role=dialog] [role=option]")].map((o) => o.textContent)).toEqual(["Tür 0 · x 3, z 7", "Tür 1 · x 4, z 7"]);
    await act(async () => document.querySelector<HTMLElement>("[role=dialog]")!.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true })));
    // A deleted destination (the header or its event file): none of its doors is offered, a new reference to it would be refused.
    await show(header(415, 90, true));
    expect(change().disabled).toBe(true);
    expect(hint()).toContain("Das Ziel ist gelöscht.");
    await show(header(416, 91));
    expect(change().disabled).toBe(true);
    expect(hint()).toContain("Das Ziel ist gelöscht.");
  });
});
