// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createPlace, getTextArchive, placeSlots } from "../../api";
import type { HeaderSummary, PlaceSlots } from "../../api";
import { listToasts, resetToasts } from "../../ui/frame/toasts";
import { typeInto } from "../../ui/primitives/testUtils";
import { NO_WORLD, WorkbenchProviders } from "../WorkbenchContext";
import type { WorkbenchData } from "../WorkbenchContext";
import type { Area, Room, World } from "../worldModel";
import { NewPlaceDialog } from "./NewPlaceDialog";

vi.mock("../../api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../../api")>()),
  createPlace: vi.fn(),
  getTextArchive: vi.fn(),
  placeSlots: vi.fn(),
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
const room = (h: HeaderSummary, name: string): Room => ({ header: h, name, suggested: name, placeKey: h.location_name, placeName: h.location_name, manual: false, shared: 0 });
const north = header(10, 5, "Zweiblattdorf");
const south = header(11, 5, "Zweiblattdorf");
const house = header(20, 5, "Zweiblattdorf");
const lab = header(22, 6, "Sandgemme");
const area: Area = { key: "w0:5", name: "Zweiblattdorf", placeKey: "Zweiblattdorf", headers: [north, south], rooms: [room(house, "Haus 1"), room(lab, "Labor")], matrix: 0 };
const world: World = {
  worldMaps: [{ key: "w0", matrix: 0, filter: null, name: "Sinnoh", areas: [area] }],
  moreAreas: [],
  unconnected: [],
  other: [],
  areaOf: new Map([10, 11, 20, 22].map((id) => [id, "w0:5"])),
  roomCount: 2,
  assignedRooms: 2,
};
const roomy: PlaceSlots = { bank: 433, count: 130, free: 126, reusable: [] };

describe("NewPlaceDialog", () => {
  let container: HTMLElement;
  let root: Root;
  const onClose = vi.fn();
  const onCreated = vi.fn();
  const onError = vi.fn();
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    resetToasts();
    vi.mocked(placeSlots).mockReset().mockResolvedValue(roomy);
    vi.mocked(getTextArchive)
      .mockReset()
      .mockResolvedValue({ index: 279, key: 0, lang: "de", messages: ["Mysteriöser Ort", "Zweiblattdorf", "Sandgemme"].map((text, i) => ({ id: `msg_${i}`, text })), users: [] });
    vi.mocked(createPlace).mockReset().mockResolvedValue({ index: 130, reused: null });
    for (const fn of [onClose, onCreated, onError]) fn.mockReset();
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  async function show(from: HeaderSummary = south) {
    const data = {} as unknown as WorkbenchData;
    await act(async () =>
      root.render(
        <WorkbenchProviders data={data} live={{ inspector: { kind: "none" }, matrixSelection: null, message: null, world: { ...NO_WORLD, world } }}>
          <NewPlaceDialog open header={from} onClose={onClose} onCreated={onCreated} onError={onError} />
        </WorkbenchProviders>,
      ),
    );
    await flush();
  }
  const dialog = () => document.querySelector<HTMLElement>("[data-new-place]")!.closest<HTMLElement>("[role=dialog]")!;
  const button = (text: string) => [...dialog().querySelectorAll("button")].find((b) => b.textContent === text);
  const nameField = () => dialog().querySelector<HTMLInputElement>('input[aria-label="Name des Orts"]')!;
  const box = (label: string) => [...dialog().querySelectorAll("label")].find((l) => l.textContent === label)!.querySelector<HTMLElement>("[role=checkbox]")!;

  it("creates_the_place_with_the_checked_headers", async () => {
    await show();
    expect(dialog().textContent).toContain("Neuer Ort");
    expect(dialog().textContent).toContain("Mitnehmen");
    expect(box("Zweiblattdorf").getAttribute("aria-checked")).toBe("true");
    expect(box("Haus 1").getAttribute("aria-checked")).toBe("true");
    expect(box("Labor").getAttribute("aria-checked")).toBe("false");
    typeInto(nameField(), "Neuheim");
    // Ticking the lab takes it along as well.
    await act(async () => box("Labor").click());
    await act(async () => button("Ort anlegen")!.click());
    await flush();
    expect(createPlace).toHaveBeenCalledWith("Neuheim", [10, 11, 20, 22]);
    expect(onCreated).toHaveBeenCalledWith({ index: 130, reused: null }, 433);
    expect(onClose).toHaveBeenCalled();
    expect(listToasts().map((t) => t.text)).toContain("Ort „Neuheim“ angelegt");
    expect(onError).not.toHaveBeenCalled();
  });

  it("says_the_long_name_note", async () => {
    await show();
    expect(dialog().textContent).not.toContain("Länger als 16 Zeichen");
    typeInto(nameField(), "Neuheim am Kraterberg");
    expect(dialog().textContent).toContain("Länger als 16 Zeichen: im Spiel kann der Name abgeschnitten werden.");
    // A note, no refusal: the place can still be created.
    await act(async () => button("Ort anlegen")!.click());
    await flush();
    expect(createPlace).toHaveBeenCalledWith("Neuheim am Kraterberg", [10, 11, 20]);
  });

  it("says_the_reuse_warning_on_a_full_heartgold_bank", async () => {
    vi.mocked(placeSlots).mockResolvedValue({ bank: 279, count: 256, free: 0, reusable: [2] });
    await show();
    expect(getTextArchive).toHaveBeenCalledWith(279);
    expect(dialog().textContent).toContain(
      "Alle freien Plätze sind belegt. Der neue Ort übernimmt den Platz von „Sandgemme“, einem Sinnoh-Namen, den kein Gebiet nutzt. Pokémon, die über den Pal-Park kommen und dort gefangen wurden, zeigen dann den neuen Namen als Fundort.",
    );
  });

  it("keeps_the_dialog_open_when_the_kernel_refuses", async () => {
    const refusal = { code: "refused", message: "Der Name enthält Zeichen, die das Spiel nicht kennt." };
    vi.mocked(createPlace).mockRejectedValue(refusal);
    await show();
    typeInto(nameField(), "Neu#heim");
    await act(async () => button("Ort anlegen")!.click());
    await flush();
    expect(onError).toHaveBeenCalledWith(refusal);
    expect(onClose).not.toHaveBeenCalled();
    expect(onCreated).not.toHaveBeenCalled();
  });

  it("asks_for_the_bank_when_confirmed_before_it_was_known", async () => {
    // The first answer of `placeSlots` (the dialog's own) never comes; the one of `create` does.
    vi.mocked(placeSlots)
      .mockReset()
      .mockImplementationOnce(() => new Promise(() => {}))
      .mockResolvedValue(roomy);
    await show();
    typeInto(nameField(), "Neuheim");
    await act(async () => button("Ort anlegen")!.click());
    await flush();
    expect(onCreated).toHaveBeenCalledWith({ index: 130, reused: null }, 433);
  });

  it("cannot_be_confirmed_without_a_name", async () => {
    await show();
    expect(button("Ort anlegen")!.disabled).toBe(true);
    typeInto(nameField(), "   ");
    expect(button("Ort anlegen")!.disabled).toBe(true);
    await act(async () => button("Ort anlegen")!.click());
    expect(createPlace).not.toHaveBeenCalled();
    typeInto(nameField(), "Neuheim");
    expect(button("Ort anlegen")!.disabled).toBe(false);
  });

  it("sends_and_names_the_trimmed_name", async () => {
    await show();
    typeInto(nameField(), "  Neuheim ");
    await act(async () => button("Ort anlegen")!.click());
    await flush();
    expect(createPlace).toHaveBeenCalledWith("Neuheim", [10, 11, 20]);
    expect(listToasts().map((t) => t.text)).toContain("Ort „Neuheim“ angelegt");
  });
});
