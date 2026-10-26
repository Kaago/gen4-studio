// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { openSelect, pressKey, typeInSearch, typeInto } from "../../ui/primitives/testUtils";
import { Field } from "./Field";
import type { FieldProps } from "./Field";
import type { FieldSpec } from "./fields";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const weather: FieldSpec = { key: "weather_id", label: "Wetter", kind: "u8" };
const music: FieldSpec = { key: "music_day_id", label: "Musik Tag", kind: "u16" };
const kanto: FieldSpec = { key: "kanto_flag", label: "Kanto", kind: "bool" };
const area: FieldSpec = {
  key: "area_type",
  label: "Gebietstyp",
  kind: "select",
  options: [
    { value: 0, label: "Innen (0)" },
    { value: 1, label: "Außen (1)" },
  ],
};
const matrix: FieldSpec = { key: "matrix_id", label: "Matrix", kind: "pick", pick: "matrix" };

describe("Field", () => {
  let container: HTMLElement;
  let root: Root;
  const onCommit = vi.fn();
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    onCommit.mockReset();
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });
  const show = (props: Partial<FieldProps> & { spec: FieldSpec }) => act(() => root.render(<Field value={3} max={undefined} onCommit={onCommit} {...props} />));
  const input = (label: string) => container.querySelector<HTMLInputElement>(`input[aria-label='${label}']`)!;
  const text = () => container.textContent ?? "";
  const button = (name: string) => [...document.querySelectorAll<HTMLButtonElement>("button")].find((b) => (b.getAttribute("aria-label") ?? b.textContent) === name || b.textContent?.startsWith(name));
  const options = () => [...document.querySelectorAll<HTMLElement>("[role=option]")];

  describe("number fields", () => {
    it("commits_on_enter_and_on_blur_and_never_per_keystroke", () => {
      show({ spec: weather });
      act(() => input("Wetter").focus());
      typeInto(input("Wetter"), "1");
      typeInto(input("Wetter"), "12");
      expect(onCommit).not.toHaveBeenCalled();
      act(() => void input("Wetter").dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true, cancelable: true })));
      expect(onCommit.mock.calls).toEqual([[weather, 12]]);
      act(() => input("Wetter").focus());
      typeInto(input("Wetter"), "15");
      act(() => input("Wetter").blur());
      expect(onCommit.mock.calls).toEqual([[weather, 12], [weather, 15]]);
    });

    it("takes_back_the_typed_text_on_escape", () => {
      show({ spec: weather });
      act(() => input("Wetter").focus());
      typeInto(input("Wetter"), "9");
      pressKey(input("Wetter"), "Escape");
      expect(input("Wetter").value).toBe("3");
      act(() => input("Wetter").blur());
      expect(onCommit).not.toHaveBeenCalled();
    });

    it("refuses_a_value_the_storage_cannot_hold_with_the_limit_and_keeps_the_old_one", () => {
      show({ spec: weather });
      act(() => input("Wetter").focus());
      typeInto(input("Wetter"), "300");
      act(() => input("Wetter").blur());
      expect(onCommit).not.toHaveBeenCalled();
      expect(text()).toContain("höchstens 255");
      expect(input("Wetter").value).toBe("3");
    });

    it("takes_a_16_bit_field_up_to_65535_and_refuses_beyond", () => {
      show({ spec: music });
      act(() => input("Musik Tag").focus());
      typeInto(input("Musik Tag"), "65535");
      act(() => input("Musik Tag").blur());
      expect(onCommit.mock.calls).toEqual([[music, 65535]]);
      act(() => input("Musik Tag").focus());
      typeInto(input("Musik Tag"), "70000");
      act(() => input("Musik Tag").blur());
      expect(text()).toContain("höchstens 65535");
      expect(onCommit).toHaveBeenCalledTimes(1);
    });

    it("writes_what_was_typed_when_the_field_goes_away_before_it_blurs", () => {
      // The inspector swaps its content on hover without blurring the field.
      show({ spec: weather });
      act(() => input("Wetter").focus());
      typeInto(input("Wetter"), "7");
      act(() => root.render(<div />));
      expect(onCommit.mock.calls).toEqual([[weather, 7]]);
    });

    it("does_not_write_an_unchanged_or_refused_text_when_the_field_goes_away", () => {
      show({ spec: weather });
      act(() => input("Wetter").focus());
      typeInto(input("Wetter"), "3");
      act(() => root.render(<div />));
      expect(onCommit).not.toHaveBeenCalled();
      show({ spec: weather });
      act(() => input("Wetter").focus());
      typeInto(input("Wetter"), "999");
      act(() => root.render(<div />));
      expect(onCommit).not.toHaveBeenCalled();
    });

    it("can_be_disabled", () => {
      show({ spec: weather, disabled: true });
      expect(input("Wetter").disabled).toBe(true);
    });
  });

  describe("switches and selects", () => {
    it("shows_a_bool_as_a_switch_and_commits_the_change_at_once", () => {
      show({ spec: kanto, value: 0 });
      const toggle = container.querySelector<HTMLElement>("[role=switch]")!;
      expect(toggle.getAttribute("aria-checked")).toBe("false");
      act(() => toggle.click());
      expect(onCommit.mock.calls).toEqual([[kanto, 1]]);
    });

    it("shows_a_set_bool_as_on_and_commits_zero_when_it_is_switched_off", () => {
      show({ spec: kanto, value: 1 });
      const toggle = container.querySelector<HTMLElement>("[role=switch]")!;
      expect(toggle.getAttribute("aria-checked")).toBe("true");
      act(() => toggle.click());
      expect(onCommit.mock.calls).toEqual([[kanto, 0]]);
    });

    it("shows_a_select_with_its_options_and_commits_the_picked_value", () => {
      show({ spec: area, value: 0 });
      const trigger = container.querySelector<HTMLElement>("[role=combobox]")!;
      expect(trigger.textContent).toContain("Innen (0)");
      openSelect(trigger);
      expect(options().map((o) => o.textContent)).toEqual(["Innen (0)", "Außen (1)"]);
      act(() => options()[1]!.click());
      expect(onCommit.mock.calls).toEqual([[area, 1]]);
    });

    it("shows_a_stored_value_without_an_option_as_Wert_7", () => {
      show({ spec: area, value: 7 });
      expect(container.querySelector("[role=combobox]")!.textContent).toContain("Wert 7");
    });
  });

  describe("pick fields", () => {
    const headers = [{ id: 1, internal_name: "T01", location_index: 0, kanto: false, location_name: "Zweiblattdorf", matrix_id: 2, area_data_id: 0, event_file_id: 0, text_archive_id: 0, script_file_id: 0, level_script_id: 0 }];

    it("shows_the_value_as_a_reference_that_leads_to_its_target", () => {
      const goTo = vi.fn();
      show({ spec: matrix, value: 2, max: 9, headers, goTo });
      const chip = button("Matrix0002")!;
      expect(chip).toBeDefined();
      act(() => chip.click());
      expect(goTo).toHaveBeenCalledWith({ kind: "matrixCell", matrix: 2, x: 0, y: 0 });
      expect(text()).toContain("0 bis 9");
    });

    it("lets_the_owner_replace_the_jump_and_disable_it", () => {
      const run = vi.fn();
      show({ spec: matrix, value: 2, max: 9, open: { run, disabled: true } });
      expect(button("Matrix0002")!.disabled).toBe(true);
      show({ spec: matrix, value: 2, max: 9, open: { run } });
      act(() => button("Matrix0002")!.click());
      expect(run).toHaveBeenCalledTimes(1);
    });

    it("changes_the_value_through_the_picker_up_to_the_limit", () => {
      show({ spec: matrix, value: 2, max: 9 });
      expect(document.querySelector("[role=dialog]")).toBeNull();
      act(() => button("Matrix ändern…")!.click());
      expect(document.querySelector("[role=dialog]")).not.toBeNull();
      // 0 to 9: ten choices, found by their number.
      expect(options()).toHaveLength(10);
      typeInSearch(document.querySelector<HTMLInputElement>("input[role=combobox]")!, "0007");
      expect(options().map((o) => o.textContent)).toEqual(["Matrix0007"]);
      act(() => options()[0]!.click());
      expect(onCommit.mock.calls).toEqual([[matrix, 7]]);
      expect(document.querySelector("[role=dialog]")).toBeNull();
    });

    it("cannot_change_a_disabled_pick", () => {
      show({ spec: matrix, value: 2, max: 9, disabled: true });
      expect(button("Matrix ändern…")!.disabled).toBe(true);
    });

    it("is_a_plain_number_field_while_the_size_of_the_archive_is_unknown", () => {
      show({ spec: matrix, value: 2, max: undefined });
      expect(input("Matrix").value).toBe("2");
      expect(button("Matrix ändern…")).toBeUndefined();
    });
  });
});
