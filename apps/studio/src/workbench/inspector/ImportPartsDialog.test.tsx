// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { importMapParts, inspectMapImport } from "../../api";
import type { PartReport } from "../../api";
import { typeInto } from "../../ui/primitives/testUtils";
import { ImportPartsDialog } from "./ImportPartsDialog";

vi.mock("../../api", async (importOriginal) => ({ ...(await importOriginal<typeof import("../../api")>()), inspectMapImport: vi.fn(), importMapParts: vi.fn() }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const flush = () => act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));
const reports: PartReport[] = [
  { part: "model", file: null, size: 41236, state: { kind: "ok" }, notes: ["terrain (BDHC) stays unchanged; check walking heights", "3 texture or palette names missing in map texture pack 7: a, b, c"] },
  { part: "permissions", file: null, size: 2048, state: { kind: "ok" }, notes: [] },
  { part: "buildings", file: null, size: 50, state: { kind: "error", message: "buildings are 50 bytes, not a multiple of 48" }, notes: [] },
];

describe("ImportPartsDialog", () => {
  let container: HTMLElement;
  let root: Root;
  const onClose = vi.fn();
  const onImported = vi.fn();
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    vi.mocked(inspectMapImport).mockReset().mockResolvedValue(reports);
    vi.mocked(importMapParts).mockReset().mockResolvedValue([{ kind: "Map", index: 5 }]);
    onClose.mockReset();
    onImported.mockReset();
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  const dialog = () => document.querySelector<HTMLElement>("[role=dialog]")!;
  const button = (text: string) => [...dialog().querySelectorAll("button")].find((b) => b.textContent === text) as HTMLButtonElement | undefined;
  const box = (name: string) => dialog().querySelector<HTMLElement>(`[role=checkbox][aria-label='${name}']`)!;
  async function show() {
    await act(async () => root.render(<ImportPartsDialog map={5} open onClose={onClose} mapCount={600} onImported={onImported} />));
    const field = dialog().querySelector<HTMLInputElement>("input[aria-label='Quellkarte']")!;
    await act(async () => void field.focus());
    typeInto(field, "6");
    await act(async () => void field.blur());
    await flush();
  }

  it("lists_the_parts_of_the_source_with_their_state_and_german_hints", async () => {
    await show();
    expect(inspectMapImport).toHaveBeenCalledWith(5, { map: 6 });
    const list = dialog().querySelector("[role=list][aria-label='Teile']")!;
    expect([...list.querySelectorAll("li")].map((r) => (r as HTMLElement).dataset.state)).toEqual(["ok", "ok", "error"]);
    expect(list.textContent).toContain("BDHC bleibt unverändert, Laufhöhen prüfen");
    expect(list.textContent).toContain("3 Texturnamen fehlen im Gebiets-Paket 7");
    expect(list.textContent).toContain("41.236 Bytes");
  });

  it("does_not_preselect_an_empty_part_and_says_it_is_left_out_but_keeps_it_choosable", async () => {
    vi.mocked(inspectMapImport).mockResolvedValue([
      { part: "permissions", file: null, size: 2048, state: { kind: "ok" }, notes: [] },
      { part: "buildings", file: null, size: 0, state: { kind: "ok" }, notes: [] },
      { part: "bgs", file: null, size: 0, state: { kind: "ok" }, notes: [] },
    ]);
    await show();
    const list = dialog().querySelector("[role=list][aria-label='Teile']")!;
    expect([...list.querySelectorAll("li")].map((r) => (r as HTMLElement).dataset.state)).toEqual(["ok", "info", "info"]);
    expect(list.textContent).toContain("Leer, wird nicht übernommen");
    expect(box("Gebäude").getAttribute("aria-checked")).toBe("false");
    expect(box("Hintergrundklang").getAttribute("aria-checked")).toBe("false");
    expect(button("1 Teil übernehmen")).toBeDefined();
    await act(async () => box("Gebäude").click());
    expect(box("Gebäude").getAttribute("aria-checked")).toBe("true");
    expect(button("2 Teile übernehmen")).toBeDefined();
  });

  it("keeps_a_part_with_an_error_from_being_chosen_and_names_the_count_on_the_button", async () => {
    await show();
    expect(box("Gebäude").getAttribute("aria-disabled") === "true" || box("Gebäude").hasAttribute("data-disabled")).toBe(true);
    expect(box("Gebäude").getAttribute("aria-checked")).toBe("false");
    expect(button("2 Teile übernehmen")).toBeDefined();
    await act(async () => box("Kollision").click());
    expect(button("1 Teil übernehmen")).toBeDefined();
  });

  it("imports_the_chosen_parts_and_says_how_many_were_taken_over", async () => {
    await show();
    await act(async () => box("Kollision").click());
    await act(async () => button("1 Teil übernehmen")!.click());
    await flush();
    expect(importMapParts).toHaveBeenCalledWith(5, { map: 6 }, ["model"], false);
    expect(onImported).toHaveBeenCalledWith(5);
    expect(dialog().textContent).toContain("1 Teil übernommen");
    await act(async () => button("Schließen")!.click());
    expect(onClose).toHaveBeenCalled();
  });

  it("offers_to_take_over_buildings_only_for_a_whole_map_of_the_other_family", async () => {
    await show();
    expect(dialog().textContent).not.toContain("Gebäude übernehmen");
    vi.mocked(inspectMapImport).mockResolvedValue([{ part: "whole", file: "/x/m.bin", size: 9000, state: { kind: "ok" }, notes: ["HeartGold/SoulSilver map file; its buildings are left out unless taken over"] }]);
    const field = dialog().querySelector<HTMLInputElement>("input[aria-label='Quellkarte']")!;
    await act(async () => void field.focus());
    typeInto(field, "7");
    await act(async () => void field.blur());
    await flush();
    const toggle = dialog().querySelector<HTMLElement>("[role=switch]")!;
    expect(toggle.getAttribute("aria-checked")).toBe("false");
    await act(async () => toggle.click());
    await act(async () => button("1 Teil übernehmen")!.click());
    await flush();
    expect(importMapParts).toHaveBeenCalledWith(5, { map: 7 }, ["whole"], true);
  });

  it("checks_files_typed_one_per_line", async () => {
    await act(async () => root.render(<ImportPartsDialog map={5} open onClose={onClose} mapCount={600} onImported={onImported} />));
    await act(async () => button("Datei")!.click());
    const area = dialog().querySelector<HTMLTextAreaElement>("textarea")!;
    await act(async () => void area.focus());
    typeInto(area, "/a/m.nsbmd\n\n/a/m.per\n");
    await act(async () => void area.blur());
    await flush();
    expect(inspectMapImport).toHaveBeenLastCalledWith(5, { files: ["/a/m.nsbmd", "/a/m.per"] });
  });

  it("shows_a_refused_import_and_reports_no_change", async () => {
    vi.mocked(importMapParts).mockRejectedValue({ code: "error", message: "model: too big", asset: null });
    await show();
    await act(async () => button("2 Teile übernehmen")!.click());
    await flush();
    expect(dialog().querySelector("[role=alert]")?.textContent).toContain("too big");
    expect(onImported).not.toHaveBeenCalled();
  });
});
