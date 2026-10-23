// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { getMatrix } from "../../api";
import type { HeaderPatchDto, HeaderSummary } from "../../api";
import { typeInto } from "../../ui/primitives/testUtils";
import { HeaderChoice } from "./HeaderChoice";
import type { HeaderChoiceValue } from "./wizardSpecs";

vi.mock("../../api", async (importOriginal) => ({ ...(await importOriginal<typeof import("../../api")>()), getMatrix: vi.fn() }));
// The picker rows ask the app-wide cache for map pictures; there is no renderer here.
vi.mock("../../world/thumbnailStore", () => ({
  thumbnails: { get: () => undefined, request: () => {}, cancel: () => {}, subscribe: () => () => {}, failure: () => null },
}));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const flush = () => act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));

const header = (id: number, internal: string, location = ""): HeaderSummary => ({ id, internal_name: internal, location_name: location, matrix_id: 0, area_data_id: 0, event_file_id: id, text_archive_id: 0, script_file_id: 0, level_script_id: 0 });
const headers = [header(0, "ZERO"), header(342, "T20", "R201")];
const existing: HeaderChoiceValue = { header: null, newHeader: null, areaDataId: null, eventFile: "New" };
const creating: HeaderChoiceValue = { ...existing, newHeader: { template: 342, internalName: "ROUTE_230" } };
const patch = (change: Partial<HeaderPatchDto>): HeaderPatchDto => ({ supported: true, applied: true, repairable: false, narc: "unpacked/dynamicHeaders", ...change });

describe("HeaderChoice", () => {
  let container: HTMLElement;
  let root: Root;
  const onChange = vi.fn();
  const onApplyPatch = vi.fn();
  const onRepairPatch = vi.fn();
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    onChange.mockReset();
    onApplyPatch.mockReset().mockResolvedValue(undefined);
    onRepairPatch.mockReset().mockResolvedValue(undefined);
    vi.mocked(getMatrix).mockReset().mockResolvedValue({ index: 0, name: "m0", width: 1, height: 1, maps: [0] });
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  function show(value: HeaderChoiceValue, state: { patch: HeaderPatchDto | null; patchError?: string | null; patchBusy?: boolean }, copySource: number | null = 342) {
    act(() =>
      root.render(
        <HeaderChoice
          headers={headers}
          links={[{ event_file: 9, index: 0, x: 1, z: 1, dest_header_id: 342, dest_warp_id: 0 }]}
          counts={{ AreaData: 10 }}
          value={value}
          copySource={copySource}
          onChange={onChange}
          patch={state.patch}
          patchError={state.patchError ?? null}
          patchBusy={state.patchBusy ?? false}
          onApplyPatch={onApplyPatch}
          onRepairPatch={onRepairPatch}
          template={342}
          defaultName="ROUTE_230"
        />,
      ),
    );
  }
  /** An option of a `Segmented` (or any button) by its text. */
  const button = (text: string) => [...container.querySelectorAll<HTMLButtonElement>("button")].find((b) => b.textContent === text);
  const pressed = (text: string) => button(text)?.getAttribute("aria-pressed") === "true";
  const field = (label: string) => container.querySelector<HTMLInputElement>(`input[aria-label="${label}"]`);

  it("offers_the_existing_header_by_default_and_a_new_one_when_the_patch_is_in", () => {
    show(existing, { patch: patch({}) });
    expect(pressed("Vorhandenen Header umstellen")).toBe(true);
    expect(pressed("Neuer Header")).toBe(false);
    expect(button("Neuer Header")?.disabled).toBe(false);
    expect(button("Header wählen…")).toBeDefined();
    act(() => button("Neuer Header")!.click());
    expect(onChange).toHaveBeenCalledWith({ ...existing, newHeader: { template: 342, internalName: "ROUTE_230" } });
  });

  it("goes_back_to_an_existing_header_and_keeps_the_event_file_choice", () => {
    show({ ...creating, eventFile: "Keep" }, { patch: patch({}) });
    act(() => button("Vorhandenen Header umstellen")!.click());
    expect(onChange).toHaveBeenCalledWith({ ...creating, eventFile: "Keep", newHeader: null });
  });

  it("picks_an_existing_header_in_the_picker_whose_rows_count_the_warps_that_lead_there", async () => {
    show(existing, { patch: patch({}) });
    await act(async () => button("Header wählen…")!.click());
    await flush();
    const options = () => [...document.querySelectorAll<HTMLElement>("[role=option]")];
    expect(options().map((o) => o.textContent)).toEqual(["ZEROWarps hierher: 0, Sonder-Header, nicht frei0000", "R201Warps hierher: 10342 · T20"]);
    await act(async () => options()[1]!.click());
    expect(onChange).toHaveBeenCalledWith({ ...existing, header: 342 });
  });

  it("shows_the_chosen_header_as_a_chip_with_its_warps_and_lets_the_choice_go", () => {
    show({ ...existing, header: 342, areaDataId: 4 }, { patch: patch({}) });
    expect(container.querySelector('[data-header="342"]')?.textContent).toContain("R201");
    expect(container.textContent).toContain("Warps hierher: 1");
    expect(field("Area-Data")?.value).toBe("4");
    expect(field("Area-Data")?.disabled).toBe(false);
    act(() => button("Keinen Header umstellen")!.click());
    expect(onChange).toHaveBeenCalledWith({ ...existing, header: null, areaDataId: 4 });
  });

  it("locks_area_data_and_event_file_while_no_header_is_chosen", () => {
    show(existing, { patch: patch({}) });
    expect(container.textContent).toContain("Kein Header gewählt: nur das Raster wird angelegt.");
    expect(field("Area-Data")?.disabled).toBe(true);
    expect(button("Neu, leer")?.disabled).toBe(true);
  });

  it("takes_a_typed_area_data_and_keeps_the_headers_own_when_the_field_is_emptied", () => {
    show({ ...existing, header: 342, areaDataId: 4 }, { patch: patch({}) });
    const input = field("Area-Data")!;
    act(() => input.focus());
    typeInto(input, "7");
    act(() => input.blur());
    expect(onChange).toHaveBeenLastCalledWith({ ...existing, header: 342, areaDataId: 7 });
    act(() => input.focus());
    typeInto(input, "");
    act(() => input.blur());
    expect(onChange).toHaveBeenLastCalledWith({ ...existing, header: 342, areaDataId: null });
  });

  it("refuses_an_area_data_beyond_the_last_one_instead_of_clamping_it", () => {
    show({ ...existing, header: 342, areaDataId: 4 }, { patch: patch({}) });
    const input = field("Area-Data")!;
    act(() => input.focus());
    typeInto(input, "10");
    act(() => input.blur());
    expect(onChange).not.toHaveBeenCalled();
    expect(container.textContent).toContain("höchstens 9");
    expect(input.value).toBe("4");
  });

  it("chooses_the_event_file_and_offers_no_copy_without_a_source", () => {
    show({ ...existing, header: 342 }, { patch: patch({}) });
    expect(pressed("Neu, leer")).toBe(true);
    act(() => button("Kopie von 0342")!.click());
    expect(onChange).toHaveBeenCalledWith({ ...existing, header: 342, eventFile: { Copy: 342 } });
    show({ ...existing, header: 342 }, { patch: patch({}) }, null);
    expect(button("Kopie")?.disabled).toBe(true);
    expect(container.textContent).toContain("Die Quelle hat keine Eventdatei zum Kopieren.");
  });

  it("does_not_offer_a_new_header_where_there_is_no_patch", () => {
    show(existing, { patch: patch({ supported: false, applied: false, narc: null }) });
    expect(button("Neuer Header")?.disabled).toBe(true);
    expect(container.textContent).toContain("Nur Platin und HGSS");
    expect(container.textContent).toContain("Neue Header gibt es nur für Platin und HeartGold/SoulSilver");
    expect(button("Patch anwenden")).toBeUndefined();
  });

  it("shows_the_error_of_a_status_that_failed_and_treats_it_as_not_supported", () => {
    show(existing, { patch: null, patchError: "unbekannter Spielcode XXXX" });
    expect(button("Neuer Header")?.disabled).toBe(true);
    expect(container.querySelector('[role="alert"]')?.textContent).toContain("unbekannter Spielcode XXXX");
    expect(container.textContent).toContain("Neue Header gibt es nur für Platin und HeartGold/SoulSilver");
  });

  it("waits_for_the_status_before_it_offers_anything", () => {
    show(existing, { patch: null });
    expect(button("Neuer Header")?.disabled).toBe(true);
    expect(container.textContent).toContain("Prüfe, ob neue Header möglich sind");
    expect(container.textContent).not.toContain("Neue Header gibt es nur");
  });

  it("explains_the_patch_and_applies_it_on_the_button", () => {
    show(existing, { patch: patch({ applied: false }) });
    const text = container.textContent ?? "";
    expect(text).toContain("ROM-Patch");
    expect(text).toContain("Bytes im Spielcode");
    expect(text).toContain("einer Datei je Header");
    expect(text).toContain("nicht zurücknehmen");
    expect(text).toContain("arm9.bin.backup");
    expect(text).toContain("DSPRE");
    // The new header can be chosen once the patch is in.
    expect(button("Neuer Header")?.disabled).toBe(true);
    act(() => button("Patch anwenden")!.click());
    expect(onApplyPatch).toHaveBeenCalledOnce();
  });

  it("blocks_the_button_while_the_patch_is_applied_and_shows_a_refusal", () => {
    show(existing, { patch: patch({ applied: false }), patchBusy: true });
    expect(button("Patch anwenden")?.disabled).toBe(true);
    expect(container.textContent).toContain("Wende an …");
    show(existing, { patch: patch({ applied: false }), patchError: "Die Header-Tabelle hat ungespeicherte Änderungen: erst save" });
    expect(container.querySelector('[role="alert"]')?.textContent).toContain("erst save");
    expect(button("Patch anwenden")?.disabled).toBe(false);
  });

  it("offers_to_repair_a_patch_whose_routine_calls_a_wrong_function", () => {
    show(existing, { patch: patch({ repairable: true }) });
    const text = container.textContent ?? "";
    expect(text).toContain("fehlerhaft");
    expect(text).toContain("nach dem Intro");
    expect(text).toContain("4 Bytes");
    // No new header goes into a game that does not start.
    expect(button("Neuer Header")?.disabled).toBe(true);
    expect(button("Patch anwenden")).toBeUndefined();
    act(() => button("Patch reparieren")!.click());
    expect(onRepairPatch).toHaveBeenCalledOnce();
    show(existing, { patch: patch({ repairable: true }), patchBusy: true });
    expect(button("Patch reparieren")?.disabled).toBe(true);
    show(existing, { patch: patch({}) });
    expect(button("Patch reparieren")).toBeUndefined();
    expect(button("Neuer Header")?.disabled).toBe(false);
  });

  it("shows_the_template_and_the_prefilled_internal_name", () => {
    show(creating, { patch: patch({}) });
    expect(container.textContent).toContain("Kopie von Header 0342 „R201“");
    expect(field("Interner Name")?.value).toBe("ROUTE_230");
    expect(container.textContent).toContain("höchstens 16 Zeichen");
    // The choice of an existing header gives way to the new header.
    expect(button("Header wählen…")).toBeUndefined();
    expect(button("Patch anwenden")).toBeUndefined();
  });

  it("commits_a_typed_internal_name_as_upper_case_ascii_of_at_most_16_characters", () => {
    show(creating, { patch: patch({}) });
    const input = field("Interner Name")!;
    act(() => input.focus());
    typeInto(input, "mein weg ß");
    act(() => void input.blur());
    expect(onChange).toHaveBeenCalledWith({ ...creating, newHeader: { template: 342, internalName: "MEIN_WEG" } });
    act(() => input.focus());
    typeInto(input, "ein sehr langer name");
    act(() => void input.blur());
    expect(onChange).toHaveBeenLastCalledWith({ ...creating, newHeader: { template: 342, internalName: "EIN_SEHR_LANGER_" } });
  });

  it("says_that_keeping_the_event_file_shares_the_templates_for_a_new_header", () => {
    show(creating, { patch: patch({}) });
    expect(container.textContent).toContain("Behalten teilt die Eventdatei mit Header 0342.");
    expect(button("Behalten")?.disabled).toBe(false);
    expect(field("Area-Data")?.disabled).toBe(false);
  });
});
