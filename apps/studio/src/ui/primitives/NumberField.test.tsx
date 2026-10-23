// @vitest-environment jsdom
import { act, useState } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { NumberField } from "./NumberField";
import type { NumberFieldProps } from "./NumberField";
import { pressKey, typeInto } from "./testUtils";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("NumberField", () => {
  let container: HTMLElement;
  let root: Root;
  const commits = vi.fn();
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    commits.mockReset();
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  /** An owner that takes every commit, as the inspector does. */
  function Owner(props: Partial<NumberFieldProps> & { start?: number }) {
    const { start = 100, ...rest } = props;
    const [value, setValue] = useState(start);
    return (
      <NumberField
        label="Modell"
        value={value}
        onCommit={(n) => {
          commits(n);
          setValue(n);
        }}
        {...rest}
      />
    );
  }
  const input = () => container.querySelector<HTMLInputElement>("input[aria-label='Modell']")!;
  const text = () => container.textContent ?? "";
  /** Enter blurs the field, as in a browser; the jsdom test has to put the focus there first. */
  const typeAndEnter = (t: string) => {
    act(() => input().focus());
    typeInto(input(), t);
    act(() => {
      input().dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true, cancelable: true }));
    });
  };
  const typeAndBlur = (t: string) => {
    act(() => input().focus());
    typeInto(input(), t);
    act(() => input().blur());
  };

  it("uses_enter_only_while_it_holds_typed_text", () => {
    // A wizard around the field moves on with an Enter the field left alone, never with the one that commits.
    act(() => root.render(<Owner />));
    act(() => input().focus());
    expect(pressKey(input(), "Enter").defaultPrevented).toBe(false);
    act(() => input().focus());
    typeInto(input(), "200");
    expect(pressKey(input(), "Enter").defaultPrevented).toBe(true);
    expect(commits.mock.calls).toEqual([[200]]);
  });

  it("commits_on_enter_and_blur_and_reverts_on_escape", () => {
    act(() => root.render(<Owner />));
    expect(input().value).toBe("100");
    typeAndEnter("200");
    expect(commits.mock.calls).toEqual([[200]]);
    expect(input().value).toBe("200");

    typeAndBlur("300");
    expect(commits.mock.calls).toEqual([[200], [300]]);

    act(() => input().focus());
    typeInto(input(), "400");
    const esc = pressKey(input(), "Escape");
    expect(esc.defaultPrevented).toBe(true);
    expect(input().value).toBe("300");
    act(() => input().blur());
    expect(commits.mock.calls).toEqual([[200], [300]]);
  });

  it("does_not_commit_per_keystroke_or_an_unchanged_value", () => {
    act(() => root.render(<Owner />));
    act(() => input().focus());
    typeInto(input(), "1");
    typeInto(input(), "12");
    expect(commits).not.toHaveBeenCalled();
    typeInto(input(), "100");
    act(() => input().blur());
    expect(commits).not.toHaveBeenCalled();
    expect(input().value).toBe("100");
  });

  it("accepts_the_largest_value_of_the_bit_width", () => {
    act(() => root.render(<Owner bits={16} />));
    typeAndEnter("65535");
    expect(commits.mock.calls).toEqual([[65535]]);
    expect(text()).not.toContain("höchstens");
  });

  it("refuses_a_value_above_the_bit_width_with_the_maximum_in_the_error_and_keeps_the_old_value", () => {
    act(() => root.render(<Owner bits={16} />));
    typeAndEnter("70000");
    expect(commits).not.toHaveBeenCalled();
    expect(text()).toContain("höchstens 65535");
    expect(input().value).toBe("100");
    expect(input().getAttribute("aria-invalid")).toBe("true");
    expect(input().getAttribute("aria-describedby")).toBeTruthy();
  });

  it("refuses_the_same_on_blur", () => {
    act(() => root.render(<Owner bits={16} />));
    typeAndBlur("70000");
    expect(commits).not.toHaveBeenCalled();
    expect(text()).toContain("höchstens 65535");
    expect(input().value).toBe("100");
  });

  it("refuses_a_negative_below_the_minimum_on_enter_and_blur", () => {
    act(() => root.render(<Owner bits={16} />));
    typeAndEnter("-1");
    expect(commits).not.toHaveBeenCalled();
    expect(text()).toContain("mindestens 0");
    expect(input().value).toBe("100");
    typeAndBlur("-5");
    expect(commits).not.toHaveBeenCalled();
    expect(text()).toContain("mindestens 0");
  });

  it("uses_a_custom_minimum_and_never_clamps_to_it", () => {
    act(() => root.render(<Owner min={10} max={20} start={15} />));
    typeAndEnter("9");
    expect(text()).toContain("mindestens 10");
    typeAndBlur("21");
    expect(text()).toContain("höchstens 20");
    expect(commits).not.toHaveBeenCalled();
    expect(input().value).toBe("15");
  });

  it("refuses_unreadable_and_empty_entries_without_committing", () => {
    act(() => root.render(<Owner base="hex" bits={16} start={255} />));
    typeAndEnter("0xZZ");
    expect(commits).not.toHaveBeenCalled();
    expect(text()).toContain("Hex-Zahl");
    expect(input().value).toBe("0xFF");
    typeAndBlur("");
    expect(commits).not.toHaveBeenCalled();
    expect(text()).toContain("Gib eine Zahl ein");
    expect(input().value).toBe("0xFF");
    act(() => root.render(<Owner bits={16} start={5} />));
    typeAndEnter("12.5");
    expect(commits).not.toHaveBeenCalled();
    expect(text()).toContain("ganze Zahl");
  });

  it("takes_the_smaller_of_max_and_the_bit_width_as_the_ceiling", () => {
    act(() => root.render(<Owner bits={8} max={300} />));
    typeAndEnter("256");
    expect(text()).toContain("höchstens 255");
    act(() => root.render(<Owner bits={16} max={100} start={1} />));
    typeAndEnter("101");
    expect(text()).toContain("höchstens 100");
    expect(commits).not.toHaveBeenCalled();
  });

  it("derives_the_ceiling_of_32_bits_without_overflow", () => {
    act(() => root.render(<Owner bits={32} />));
    typeAndEnter("4294967295");
    expect(commits.mock.calls).toEqual([[4294967295]]);
    typeAndEnter("4294967296");
    expect(text()).toContain("höchstens 4294967295");
  });

  it("clears_the_error_when_the_user_types_again_or_takes_the_entry_back", () => {
    act(() => root.render(<Owner bits={16} />));
    typeAndEnter("70000");
    expect(text()).toContain("höchstens 65535");
    act(() => input().focus());
    typeInto(input(), "7");
    expect(text()).not.toContain("höchstens");

    typeAndEnter("70000");
    expect(text()).toContain("höchstens 65535");
    act(() => input().focus());
    pressKey(input(), "Escape");
    expect(text()).not.toContain("höchstens");
  });

  it("switches_to_hex_and_parses_hex_input", () => {
    act(() => root.render(<Owner showBase start={255} bits={16} />));
    expect(input().value).toBe("255");
    const hexButton = [...container.querySelectorAll("button")].find((b) => b.textContent === "Hex")!;
    act(() => hexButton.click());
    expect(input().value).toBe("0xFF");
    typeAndEnter("0x10");
    expect(commits.mock.calls).toEqual([[16]]);
    expect(input().value).toBe("0x10");
    typeAndEnter("1f");
    expect(commits.mock.calls).toEqual([[16], [31]]);
    const decButton = [...container.querySelectorAll("button")].find((b) => b.textContent === "Dez")!;
    act(() => decButton.click());
    expect(input().value).toBe("31");
  });

  it("writes_the_bound_in_hex_when_the_field_shows_hex", () => {
    act(() => root.render(<Owner base="hex" bits={16} start={1} />));
    typeAndEnter("0x10000");
    expect(commits).not.toHaveBeenCalled();
    expect(text()).toContain("höchstens 0xFFFF");
  });

  it("offers_the_base_switch_only_on_request", () => {
    act(() => root.render(<Owner />));
    expect([...container.querySelectorAll("button")].some((b) => b.textContent === "Hex")).toBe(false);
  });

  it("steps_with_the_arrow_keys_and_stops_at_the_bounds_without_an_error", () => {
    act(() => root.render(<Owner min={0} max={102} start={100} step={1} />));
    act(() => input().focus());
    pressKey(input(), "ArrowUp");
    expect(commits.mock.calls).toEqual([[101]]);
    pressKey(input(), "ArrowUp", { shiftKey: true });
    expect(commits.mock.calls).toEqual([[101]]);
    pressKey(input(), "ArrowDown");
    expect(commits.mock.calls).toEqual([[101], [100]]);
    expect(text()).not.toContain("höchstens");
  });

  function Shell() {
    const [shown, setShown] = useState(true);
    return (
      <div>
        {shown && <NumberField label="Modell" value={5} bits={16} onCommit={commits} />}
        <button type="button" onClick={() => setShown(false)}>
          Weiter
        </button>
      </div>
    );
  }
  const leave = () => act(() => container.querySelector("button")!.dispatchEvent(new MouseEvent("click", { bubbles: true })));

  it("commits_a_valid_draft_when_it_unmounts_without_a_blur", () => {
    act(() => root.render(<Shell />));
    typeInto(input(), "7");
    leave();
    expect(commits.mock.calls).toEqual([[7]]);
  });

  it("drops_an_invalid_draft_when_it_unmounts", () => {
    act(() => root.render(<Shell />));
    typeInto(input(), "70000");
    leave();
    expect(commits).not.toHaveBeenCalled();
  });

  it("disables_the_field_and_names_it_by_its_label", () => {
    act(() => root.render(<Owner disabled />));
    expect(input().disabled).toBe(true);
  });

  it("shows_the_label_as_the_scrub_handle_and_a_grip_without_it", () => {
    act(() => root.render(<Owner showLabel />));
    expect(container.querySelector("label")?.textContent).toBe("Modell");
    expect(container.querySelector("[data-scrub]")).not.toBeNull();
    act(() => root.render(<Owner />));
    expect(container.querySelector("label")).toBeNull();
    expect(container.querySelector("[data-scrub]")).not.toBeNull();
  });

  it("scrubs_ten_times_faster_with_shift_and_in_whole_steps", () => {
    act(() => root.render(<Owner showLabel start={100} step={2} />));
    const handle = container.querySelector<HTMLElement>("[data-scrub]")!;
    act(() => {
      handle.dispatchEvent(new MouseEvent("pointerdown", { bubbles: true, cancelable: true, button: 0 }));
    });
    const move = (movementX: number, shiftKey = false) =>
      act(() => {
        const e = new MouseEvent("pointermove", { bubbles: true, cancelable: true, shiftKey });
        Object.defineProperty(e, "movementX", { value: movementX });
        Object.defineProperty(e, "movementY", { value: 0 });
        window.dispatchEvent(e);
      });
    move(8);
    expect(input().value).toBe("104");
    // At the boundary of a step, one pixel back already takes the step back.
    move(-1);
    expect(input().value).toBe("102");
    move(-3);
    expect(input().value).toBe("102");
    move(4, true);
    expect(input().value).toBe("122");
    act(() => {
      window.dispatchEvent(new MouseEvent("pointerup", { bubbles: true }));
    });
    expect(commits.mock.calls).toEqual([[122]]);
  });

  it("scrubs_by_dragging_and_commits_once_on_release_within_the_bounds", () => {
    act(() => root.render(<Owner showLabel start={100} max={102} />));
    const handle = container.querySelector<HTMLElement>("[data-scrub]")!;
    act(() => {
      handle.dispatchEvent(new MouseEvent("pointerdown", { bubbles: true, cancelable: true, button: 0 }));
    });
    const move = (movementX: number) =>
      act(() => {
        const e = new MouseEvent("pointermove", { bubbles: true, cancelable: true });
        Object.defineProperty(e, "movementX", { value: movementX });
        Object.defineProperty(e, "movementY", { value: 0 });
        window.dispatchEvent(e);
      });
    // One step per 4 px, however the movement is split into events.
    move(1);
    move(1);
    move(1);
    expect(input().value).toBe("100");
    move(1);
    expect(input().value).toBe("101");
    expect(commits).not.toHaveBeenCalled();
    move(40);
    expect(input().value).toBe("102");
    act(() => {
      window.dispatchEvent(new MouseEvent("pointerup", { bubbles: true }));
    });
    expect(commits.mock.calls).toEqual([[102]]);
  });
});
