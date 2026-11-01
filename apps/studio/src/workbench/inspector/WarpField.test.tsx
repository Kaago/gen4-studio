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

const header = (id: number, internal: string, location: string, event: number): HeaderSummary => ({ id, internal_name: internal, location_index: 0, kanto: false, location_name: location, matrix_id: id, area_data_id: 0, event_file_id: event, text_archive_id: 0, script_file_id: 0, level_script_id: 0 });
// The door of header 0 (Zweiblattdorf, warp 2) leads into the house (header 4, event file 9).
const house = header(4, "T01R0201", "Spielerhaus", 9);
const houseEvents = (back: { header: number; warp: number }): EventsDto => ({
  event_file_id: 9,
  locked: null,
  bg: [],
  objects: [],
  coords: [],
  warps: [
    { index: 0, x: 3, z: 8, dest_header_id: back.header, dest_warp_id: back.warp, height: 0 },
    { index: 1, x: 9, z: 2, dest_header_id: 7, dest_warp_id: 0, height: 0 },
  ],
});

describe("WarpField", () => {
  let container: HTMLElement;
  let root: Root;
  const onCommit = vi.fn();
  const onError = vi.fn();
  const onConnect = vi.fn();
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    onCommit.mockReset();
    onError.mockReset();
    onConnect.mockReset();
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  async function show(value: number, connect = true) {
    await act(async () =>
      root.render(<WarpField destHeader={house} value={value} self={{ header: 0, warp: 2 }} onCommit={onCommit} onError={onError} onConnect={connect ? onConnect : undefined} />),
    );
    await flush();
  }
  const counterpart = () => container.querySelector("[data-warp-counterpart]")?.textContent;
  const connectButton = () => container.querySelector<HTMLButtonElement>("button[data-connect-doors]");

  it("names_the_counterpart_and_offers_connect", async () => {
    // Door 0 of the house leads back to this door (header 0, warp 2).
    vi.mocked(getEvents).mockResolvedValue(houseEvents({ header: 0, warp: 2 }));
    await show(0);
    expect(container.querySelector("[data-warp-field]")?.textContent).toContain("Tür 0 · x 3, z 8");
    expect(counterpart()).toBe("Tür 0 in Spielerhaus · führt zurück");
    // „Tür verbinden…“ hands the wish to the editor, which opens the wizard with this door as door A.
    expect(connectButton()?.textContent).toBe("Tür verbinden…");
    await act(async () => connectButton()!.click());
    expect(onConnect).toHaveBeenCalledOnce();

    // A door there that leads elsewhere does not lead back.
    vi.mocked(getEvents).mockResolvedValue(houseEvents({ header: 0, warp: 1 }));
    await act(async () => root.unmount());
    root = createRoot(container);
    await show(0);
    expect(counterpart()).toBe("Tür 0 in Spielerhaus · führt nicht zurück");

    // A door number the destination does not have.
    await act(async () => root.unmount());
    root = createRoot(container);
    await show(5, false);
    expect(counterpart()).toBe("Ziel hat keine Tür 5");
    // Without `onConnect` there is no button.
    expect(connectButton()).toBeNull();
  });
});
