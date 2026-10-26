// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { assignPlace, getTextArchive, placeSlots } from "../../api";
import type { HeaderFamily, HeaderSummary } from "../../api";
import type { Depth } from "../../ui/frame/shellState";
import { visibleWords } from "../../ui/visibleText";
import { placeColor } from "../places";
import { PlacePicker } from "./PlacePicker";

vi.mock("../../api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../../api")>()),
  assignPlace: vi.fn(),
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
const village = header(411, 1, "Zweiblattdorf");
const headers = [village, header(412, 1, "Zweiblattdorf"), header(413, 2, "Sandgemme")];
const names = ["Mysteriöser Ort", "Zweiblattdorf", "Sandgemme", "Route 201"];

describe("PlacePicker", () => {
  let container: HTMLElement;
  let root: Root;
  const onAssigned = vi.fn();
  const onError = vi.fn();
  const commit = vi.fn();
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    vi.mocked(placeSlots).mockReset().mockResolvedValue({ bank: 433, count: 4, free: 252, reusable: [] });
    vi.mocked(getTextArchive)
      .mockReset()
      .mockResolvedValue({ index: 433, key: 0, lang: "de", messages: names.map((text, i) => ({ id: `msg_${i}`, text })), users: [] });
    vi.mocked(assignPlace).mockReset().mockResolvedValue(undefined);
    for (const fn of [onAssigned, onError, commit]) fn.mockReset();
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  async function show(depth: Depth = "bauen", family: HeaderFamily = "Pt") {
    await act(async () =>
      root.render(
        <PlacePicker header={village} depth={depth} onAssigned={onAssigned} headers={headers} family={family} onError={onError} commit={family === "DP" ? commit : undefined} />,
      ),
    );
    await flush();
  }
  const pickerButton = () => container.querySelector<HTMLButtonElement>("button[data-place-picker]")!;
  const options = () => [...document.querySelectorAll<HTMLElement>("[role=dialog] [role=option]")];
  const dialogButton = (text: string) => [...document.querySelectorAll<HTMLButtonElement>("[role=dialog] button")].find((b) => b.textContent === text);
  async function openPicker() {
    await act(async () => pickerButton().click());
    await flush();
  }

  it("shows_the_place_with_its_colour_and_name_in_bauen", async () => {
    await show();
    expect(pickerButton().textContent).toBe("Zweiblattdorf");
    const probe = document.createElement("span");
    probe.style.background = placeColor("Zweiblattdorf");
    expect(pickerButton().querySelector<HTMLElement>("[data-place-dot]")?.style.background).toBe(probe.style.background);
    expect(container.textContent).toContain("Ort");
    expect(visibleWords(container)).not.toMatch(/Raster|Matrix|Header|Nachricht|Archiv/);
  });

  it("shows_message_and_archive_in_tuefteln", async () => {
    await show("tuefteln");
    expect(container.textContent).toContain("Ortsname");
    expect(pickerButton().textContent).toBe("ZweiblattdorfNachricht 1 · Archiv 0433");
  });

  it("assigns_the_picked_place", async () => {
    await show();
    await openPicker();
    expect(document.querySelector("[role=dialog]")?.textContent).toContain("Ort wählen");
    expect(options().map((o) => o.textContent)).toEqual([
      "Kein Ort (Mysteriöser Ort)unbenutzt",
      "Zweiblattdorf2 Gebiete und Räume",
      "Sandgemme1 Gebiet oder Raum",
      "Route 201unbenutzt",
    ]);
    await act(async () => options()[2]!.click());
    await flush();
    expect(assignPlace).toHaveBeenCalledWith([411], 2);
    expect(onAssigned).toHaveBeenCalledWith(null);
    expect(onError).not.toHaveBeenCalled();
  });

  it("opens_the_new_place_dialog_from_the_footer", async () => {
    await show();
    await openPicker();
    await act(async () => dialogButton("Neuer Ort…")!.click());
    await flush();
    const dialog = document.querySelector("[data-new-place]")?.closest("[role=dialog]");
    expect(dialog?.textContent).toContain("Neuer Ort");
    expect(dialog?.textContent).toContain("Name des Orts");
  });

  it("locks_new_place_on_diamond_and_pearl", async () => {
    await show("bauen", "DP");
    await openPicker();
    expect(dialogButton("Neuer Ort…")!.disabled).toBe(true);
    expect(document.querySelector("[role=dialog]")?.textContent).toContain("Eigene Orte gibt es nur für Platin und HeartGold/SoulSilver.");
    // Diamond and Pearl write the field itself: the kernel has no place commands there.
    await act(async () => options()[3]!.click());
    await flush();
    expect(commit).toHaveBeenCalledWith(3);
    expect(assignPlace).not.toHaveBeenCalled();
  });
});
