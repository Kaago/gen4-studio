// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { placeSlots, renamePlace } from "../../api";
import type { HeaderSummary } from "../../api";
import { typeInto } from "../../ui/primitives/testUtils";
import { NO_WORLD, WorkbenchProviders } from "../WorkbenchContext";
import type { WorkbenchData } from "../WorkbenchContext";
import type { Area, World } from "../worldModel";
import { RenamePlaceDialog } from "./RenamePlaceDialog";

vi.mock("../../api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../../api")>()),
  placeSlots: vi.fn(),
  renamePlace: vi.fn(),
}));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const flush = () => act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));
const header = (id: number, location_index: number, location_name: string): HeaderSummary => ({
  id,
  internal_name: `H${id}`,
  location_name,
  location_index,
  kanto: false,
  matrix_id: 0,
  area_data_id: 0,
  event_file_id: 0,
  text_archive_id: 0,
  script_file_id: 0,
  level_script_id: 0,
});
const headers = [header(10, 5, "Zweiblattdorf"), header(11, 5, "Zweiblattdorf"), header(20, 5, "Zweiblattdorf"), header(22, 6, "Sandgemme")];

// Zweiblattdorf has two sections (10, 11) and a room (20): two users, not three headers.
const area: Area = { key: "w0:5", name: "Zweiblattdorf", placeKey: "Zweiblattdorf", headers: [headers[0]!, headers[1]!], rooms: [], matrix: 0 };
const world: World = {
  worldMaps: [{ key: "w0", matrix: 0, filter: null, name: "Sinnoh", areas: [area] }],
  moreAreas: [],
  unconnected: [],
  other: [],
  areaOf: new Map([10, 11, 20].map((id) => [id, "w0:5"])),
  roomCount: 1,
  assignedRooms: 1,
};

describe("RenamePlaceDialog", () => {
  let container: HTMLElement;
  let root: Root;
  const onClose = vi.fn();
  const onRenamed = vi.fn();
  const onError = vi.fn();
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    vi.mocked(placeSlots).mockReset().mockResolvedValue({ bank: 433, count: 130, free: 126, reusable: [] });
    vi.mocked(renamePlace).mockReset().mockResolvedValue(undefined);
    for (const fn of [onClose, onRenamed, onError]) fn.mockReset();
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  const dialog = () => document.querySelector<HTMLElement>("[role=dialog]")!;
  const button = (text: string) => [...dialog().querySelectorAll("button")].find((b) => b.textContent === text);

  it("renames_and_says_how_many_it_touches", async () => {
    await act(async () => root.render(<RenamePlaceDialog open index={5} headers={headers} onClose={onClose} onRenamed={onRenamed} onError={onError} />));
    await flush();
    expect(dialog().textContent).toContain("Ort umbenennen");
    // No world model yet: every header is one user.
    expect(dialog().textContent).toContain("Trifft alle 3 Gebiete und Räume mit diesem Ort und den Fundort gefangener Pokémon.");
    const field = dialog().querySelector<HTMLInputElement>("input")!;
    expect(field.value).toBe("Zweiblattdorf");
    typeInto(field, "Zweiblattstadt am Fluss");
    expect(dialog().textContent).toContain("Länger als 16 Zeichen: im Spiel kann der Name abgeschnitten werden.");
    await act(async () => button("Umbenennen")!.click());
    await flush();
    expect(renamePlace).toHaveBeenCalledWith(5, "Zweiblattstadt am Fluss");
    expect(onRenamed).toHaveBeenCalledWith(433);
    expect(onClose).toHaveBeenCalled();
    expect(onError).not.toHaveBeenCalled();
  });

  async function show() {
    await act(async () => root.render(<RenamePlaceDialog open index={5} headers={headers} onClose={onClose} onRenamed={onRenamed} onError={onError} />));
    await flush();
  }

  it("counts_the_sections_of_an_area_once", async () => {
    const data = {} as unknown as WorkbenchData;
    await act(async () =>
      root.render(
        <WorkbenchProviders data={data} live={{ inspector: { kind: "none" }, matrixSelection: null, message: null, world: { ...NO_WORLD, world } }}>
          <RenamePlaceDialog open index={5} headers={headers} onClose={onClose} onRenamed={onRenamed} onError={onError} />
        </WorkbenchProviders>,
      ),
    );
    await flush();
    expect(dialog().textContent).toContain("Trifft alle 2 Gebiete und Räume mit diesem Ort und den Fundort gefangener Pokémon.");
  });

  it("asks_for_the_bank_when_confirmed_before_it_was_known", async () => {
    vi.mocked(placeSlots)
      .mockReset()
      .mockImplementationOnce(() => new Promise(() => {}))
      .mockResolvedValue({ bank: 433, count: 130, free: 126, reusable: [] });
    await show();
    await act(async () => button("Umbenennen")!.click());
    await flush();
    expect(renamePlace).toHaveBeenCalled();
    expect(onRenamed).toHaveBeenCalledWith(433);
  });

  it("cannot_be_confirmed_without_a_name", async () => {
    await show();
    const field = dialog().querySelector<HTMLInputElement>("input")!;
    typeInto(field, " ");
    expect(button("Umbenennen")!.disabled).toBe(true);
    await act(async () => button("Umbenennen")!.click());
    expect(renamePlace).not.toHaveBeenCalled();
    typeInto(field, "Neuheim");
    expect(button("Umbenennen")!.disabled).toBe(false);
  });

  it("sends_the_trimmed_name", async () => {
    await show();
    typeInto(dialog().querySelector<HTMLInputElement>("input")!, "  Neuheim ");
    await act(async () => button("Umbenennen")!.click());
    await flush();
    expect(renamePlace).toHaveBeenCalledWith(5, "Neuheim");
  });
});
