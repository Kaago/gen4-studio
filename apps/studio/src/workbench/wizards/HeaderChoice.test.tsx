// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { HeaderPatchDto, HeaderSummary } from "../../api";
import { HeaderChoice } from "./HeaderChoice";
import type { HeaderChoiceValue } from "./wizardSpecs";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const header = (id: number, internal: string, location = ""): HeaderSummary => ({ id, internal_name: internal, location_name: location, matrix_id: 0, area_data_id: 0, event_file_id: id });
const headers = [header(0, "ZERO"), header(342, "T20", "R201")];
const existing: HeaderChoiceValue = { header: null, newHeader: null, areaDataId: null, eventFile: "New" };
const creating: HeaderChoiceValue = { ...existing, newHeader: { template: 342, internalName: "ROUTE_230" } };
const patch = (change: Partial<HeaderPatchDto>): HeaderPatchDto => ({ supported: true, applied: true, narc: "unpacked/dynamicHeaders", ...change });

describe("HeaderChoice", () => {
  let container: HTMLElement;
  let root: Root;
  const onChange = vi.fn();
  const onApplyPatch = vi.fn();
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    onChange.mockReset();
    onApplyPatch.mockReset().mockResolvedValue(undefined);
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  function show(value: HeaderChoiceValue, state: { patch: HeaderPatchDto | null; patchError?: string | null; patchBusy?: boolean }) {
    act(() =>
      root.render(
        <HeaderChoice
          headers={headers}
          links={[]}
          counts={{ AreaData: 10 }}
          value={value}
          copySource={342}
          onChange={onChange}
          patch={state.patch}
          patchError={state.patchError ?? null}
          patchBusy={state.patchBusy ?? false}
          onApplyPatch={onApplyPatch}
          template={342}
          defaultName="ROUTE_230"
        />,
      ),
    );
  }
  const radio = (label: string) => [...container.querySelectorAll<HTMLInputElement>('input[type="radio"]')].find((r) => r.parentElement?.textContent?.startsWith(label));
  const button = (text: string) => [...container.querySelectorAll("button")].find((b) => b.textContent === text);
  const field = () => container.querySelector<HTMLInputElement>('input[aria-label="Interner Name"]');

  it("offers_the_existing_header_by_default_and_a_new_one_when_the_patch_is_in", () => {
    show(existing, { patch: patch({}) });
    expect(radio("Vorhandenen Header umstellen")?.checked).toBe(true);
    expect(radio("Neuer Header")?.checked).toBe(false);
    expect(radio("Neuer Header")?.disabled).toBe(false);
    expect(container.textContent).toContain("Header wählen");
    act(() => radio("Neuer Header")!.click());
    expect(onChange).toHaveBeenCalledWith({ ...existing, newHeader: { template: 342, internalName: "ROUTE_230" } });
  });

  it("goes_back_to_an_existing_header_and_keeps_the_event_file_choice", () => {
    show({ ...creating, eventFile: "Keep" }, { patch: patch({}) });
    act(() => radio("Vorhandenen Header umstellen")!.click());
    expect(onChange).toHaveBeenCalledWith({ ...creating, eventFile: "Keep", newHeader: null });
  });

  it("does_not_offer_a_new_header_where_there_is_no_patch", () => {
    show(existing, { patch: patch({ supported: false, applied: false, narc: null }) });
    expect(radio("Neuer Header")?.disabled).toBe(true);
    expect(container.textContent).toContain("Neue Header gibt es nur für Platin und HeartGold/SoulSilver");
    expect(button("Patch anwenden")).toBeUndefined();
  });

  it("shows_the_error_of_a_status_that_failed_and_treats_it_as_not_supported", () => {
    show(existing, { patch: null, patchError: "unbekannter Spielcode XXXX" });
    expect(radio("Neuer Header")?.disabled).toBe(true);
    expect(container.textContent).toContain("unbekannter Spielcode XXXX");
    expect(container.textContent).toContain("Neue Header gibt es nur für Platin und HeartGold/SoulSilver");
  });

  it("waits_for_the_status_before_it_offers_anything", () => {
    show(existing, { patch: null });
    expect(radio("Neuer Header")?.disabled).toBe(true);
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
    expect(radio("Neuer Header")?.disabled).toBe(true);
    act(() => button("Patch anwenden")!.click());
    expect(onApplyPatch).toHaveBeenCalledOnce();
  });

  it("blocks_the_button_while_the_patch_is_applied_and_shows_a_refusal", () => {
    show(existing, { patch: patch({ applied: false }), patchBusy: true });
    expect(button("Patch anwenden")?.disabled).toBe(true);
    show(existing, { patch: patch({ applied: false }), patchError: "Die Header-Tabelle hat ungespeicherte Änderungen: erst save" });
    expect(container.textContent).toContain("erst save");
    expect(button("Patch anwenden")?.disabled).toBe(false);
  });

  it("shows_the_template_and_the_prefilled_internal_name", () => {
    show(creating, { patch: patch({}) });
    expect(container.textContent).toContain("Kopie von Header 0342 „R201“");
    expect(field()?.value).toBe("ROUTE_230");
    expect(field()?.maxLength).toBe(16);
    expect(field()?.pattern).toBe("[A-Z0-9_]*");
    // The list of existing headers gives way to the new header.
    expect(container.querySelector('input[aria-label="Zielheader suchen"]')).toBeNull();
    expect(button("Patch anwenden")).toBeUndefined();
  });

  it("commits_a_typed_internal_name_as_upper_case_ascii", () => {
    show(creating, { patch: patch({}) });
    const input = field()!;
    const setValue = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!;
    act(() => input.focus());
    act(() => {
      setValue.call(input, "mein weg ß");
      input.dispatchEvent(new Event("input", { bubbles: true }));
    });
    act(() => void input.blur());
    expect(onChange).toHaveBeenCalledWith({ ...creating, newHeader: { template: 342, internalName: "MEIN_WEG" } });
  });

  it("says_that_keeping_the_event_file_shares_the_templates_for_a_new_header", () => {
    show(creating, { patch: patch({}) });
    expect(container.textContent).toContain("behalten (geteilt mit Header 0342)");
    expect(container.querySelector<HTMLInputElement>('fieldset')?.disabled).toBe(false);
  });
});
