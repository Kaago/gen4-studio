// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { placeSlots, renamePlace } from "../../api";
import type { HeaderSummary } from "../../api";
import { typeInto } from "../../ui/primitives/testUtils";
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
});
