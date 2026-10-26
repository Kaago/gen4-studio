// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { getMatrix, getTextArchive, placeSlots } from "../../api";
import type { HeaderSummary } from "../../api";
import { visibleWords } from "../../ui/visibleText";
import { PlaceStep } from "./PlaceStep";
import type { PlaceStepValue } from "./PlaceStep";
import { areaPlace, roomPlace } from "./wizardSpecs";

vi.mock("../../api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../../api")>()),
  getMatrix: vi.fn(),
  getTextArchive: vi.fn(),
  placeSlots: vi.fn(),
}));
vi.mock("../../world/thumbnailStore", () => ({
  thumbnails: { get: () => undefined, request: () => {}, cancel: () => {}, subscribe: () => () => {}, failure: () => null },
}));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const flush = () => act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));
const header = (id: number, location_index: number, location_name: string, area = 0): HeaderSummary => ({
  id,
  internal_name: `H${id}`,
  location_name,
  location_index,
  kanto: false,
  matrix_id: 0,
  area_data_id: area,
  event_file_id: 0,
  text_archive_id: 0,
  script_file_id: 0,
  level_script_id: 0,
});
const village = header(411, 1, "Zweiblattdorf", 3);
const town = header(413, 2, "Sandgemme", 4);
const headers = [village, header(412, 1, "Zweiblattdorf", 3), town];
const names = ["Mysteriöser Ort", "Zweiblattdorf", "Sandgemme", "Route 201"];

describe("PlaceStep", () => {
  let container: HTMLElement;
  let root: Root;
  const onChange = vi.fn();
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    vi.mocked(placeSlots).mockReset().mockResolvedValue({ bank: 433, count: 4, free: 252, reusable: [] });
    vi.mocked(getTextArchive)
      .mockReset()
      .mockResolvedValue({ index: 433, key: 0, lang: "de", messages: names.map((text, i) => ({ id: `msg_${i}`, text })), users: [] });
    vi.mocked(getMatrix).mockReset().mockResolvedValue({ index: 0, name: "m0", width: 1, height: 1, maps: [7] });
    onChange.mockReset();
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  async function show(value: PlaceStepValue, template: HeaderSummary | null = village, name = "Neuheim") {
    await act(async () => root.render(<PlaceStep value={value} onChange={onChange} headers={headers} template={template} family="Pt" name={name} />));
    await flush();
  }
  const toggle = (label: string) => [...container.querySelectorAll<HTMLButtonElement>("[aria-label=Ort] button")].find((b) => b.textContent === label);
  const nameField = () => container.querySelector<HTMLInputElement>('input[aria-label="Name des neuen Orts"]');
  const pickerButton = () => container.querySelector<HTMLButtonElement>("button[data-place-picker]");
  const options = () => [...document.querySelectorAll<HTMLElement>("[role=dialog] [role=option]")];

  it("defaults_an_area_to_a_new_place_named_like_it", async () => {
    await show({ place: areaPlace({ place: null, name: "Neuheim" }), areaDataId: 3 });
    expect(toggle("Neuer Ort")?.getAttribute("aria-pressed")).toBe("true");
    expect(nameField()?.value).toBe("Neuheim");
    expect(pickerButton()).toBeNull();
    expect(visibleWords(container)).not.toMatch(/Raster|Matrix|Header/);
  });

  it("defaults_a_room_to_the_place_of_its_entrance", async () => {
    await show({ place: roomPlace(null, town, village)!, areaDataId: 3 });
    expect(toggle("Vorhandener Ort")?.getAttribute("aria-pressed")).toBe("true");
    expect(pickerButton()?.textContent).toBe("Sandgemme");
    expect(nameField()).toBeNull();
    expect(visibleWords(container)).not.toMatch(/Raster|Matrix|Header/);
  });

  it("picks_an_existing_place", async () => {
    await show({ place: { New: "Neuheim" }, areaDataId: 3 });
    // Leaving „Neuer Ort“ starts from the place of the template.
    await act(async () => toggle("Vorhandener Ort")?.click());
    expect(onChange).toHaveBeenLastCalledWith({ place: { Existing: 1 }, areaDataId: 3 });
    await show({ place: { Existing: 1 }, areaDataId: 3 });
    await act(async () => pickerButton()?.click());
    await flush();
    expect(document.querySelector("[role=dialog]")?.textContent).toContain("Ort wählen");
    // Only places that exist: a new one is made with „Neuer Ort“ above, not from the picker.
    expect(document.querySelector("[role=dialog]")?.textContent).not.toContain("Neuer Ort…");
    await act(async () => options().find((o) => o.textContent?.includes("Sandgemme"))?.click());
    await flush();
    expect(onChange).toHaveBeenLastCalledWith({ place: { Existing: 2 }, areaDataId: 3 });
    // Back to „Neuer Ort“: the field takes the wizard's name.
    await act(async () => toggle("Neuer Ort")?.click());
    expect(onChange).toHaveBeenLastCalledWith({ place: { New: "Neuheim" }, areaDataId: 3 });
  });

  it("warns_from_17_characters", async () => {
    await show({ place: { New: "A".repeat(16) }, areaDataId: 3 });
    expect(container.textContent).not.toContain("Länger als 16 Zeichen");
    await show({ place: { New: "A".repeat(17) }, areaDataId: 3 });
    expect(container.textContent).toContain("Länger als 16 Zeichen: im Spiel kann der Name abgeschnitten werden.");
  });

  it("warns_before_reusing_a_sinnoh_name", async () => {
    vi.mocked(placeSlots).mockResolvedValue({ bank: 433, count: 256, free: 0, reusable: [2] });
    await show({ place: { New: "Neuheim" }, areaDataId: 3 });
    expect(container.textContent).toContain("Der neue Ort übernimmt den Platz von „Sandgemme“");
    // An existing place takes no slot.
    await show({ place: { Existing: 1 }, areaDataId: 3 });
    expect(container.textContent).not.toContain("Alle freien Plätze sind belegt");
  });

  it("says_an_empty_name_is_not_allowed_and_locks_a_new_place_in_diamond_and_pearl", async () => {
    await show({ place: { New: "" }, areaDataId: 3 });
    expect(container.textContent).toContain("Name des neuen Orts darf nicht leer sein.");
    await act(async () => root.render(<PlaceStep value={{ place: { Existing: 1 }, areaDataId: 3 }} onChange={onChange} headers={headers} template={village} family="DP" name="Neuheim" />));
    await flush();
    expect(toggle("Neuer Ort")?.disabled).toBe(true);
    expect(container.textContent).toContain("Eigene Orte gibt es nur für Platin und HeartGold/SoulSilver.");
  });

  it("says_the_places_cannot_be_read_instead_of_loading_forever", async () => {
    vi.mocked(getTextArchive).mockRejectedValue({ code: "error", message: "text archive 433 is missing", asset: null });
    await show({ place: { Existing: 1 }, areaDataId: 3 });
    expect(container.textContent).toContain("text archive 433 is missing");
    await act(async () => pickerButton()?.click());
    await flush();
    expect(document.querySelector("[role=dialog]")?.textContent).toContain("Orte nicht lesbar.");
    expect(document.querySelector("[role=dialog]")?.textContent).not.toContain("Lade Orte");
  });
});
