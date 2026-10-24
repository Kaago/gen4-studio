// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { Segmented } from "./Segmented";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("Segmented", () => {
  let container: HTMLElement;
  let root: Root;
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  const options = [
    { value: "baum", label: "Baum", icon: "tree" as const },
    { value: "liste", label: "Liste", icon: "list" as const },
  ];
  const buttons = () => [...container.querySelectorAll<HTMLButtonElement>("button")];
  const pressed = () => buttons().filter((b) => b.getAttribute("aria-pressed") === "true");

  it("is_a_labelled_group_with_exactly_one_pressed_option", () => {
    act(() => root.render(<Segmented options={options} value="liste" onChange={() => {}} ariaLabel="Ansicht" />));
    expect(container.querySelector("[role=group]")?.getAttribute("aria-label")).toBe("Ansicht");
    expect(buttons().map((b) => b.textContent)).toEqual(["Baum", "Liste"]);
    expect(pressed().map((b) => b.textContent)).toEqual(["Liste"]);
    expect(buttons()[0].querySelector("svg")).not.toBeNull();
  });

  it("calls_onChange_with_the_clicked_value", () => {
    const onChange = vi.fn();
    act(() => root.render(<Segmented options={options} value="liste" onChange={onChange} ariaLabel="Ansicht" />));
    act(() => buttons()[0].click());
    expect(onChange.mock.calls).toEqual([["baum"]]);
  });

  it("keeps_the_active_option_when_it_is_clicked_again", () => {
    const onChange = vi.fn();
    act(() => root.render(<Segmented options={options} value="liste" onChange={onChange} ariaLabel="Ansicht" />));
    act(() => buttons()[1].click());
    expect(onChange).not.toHaveBeenCalled();
    expect(pressed().map((b) => b.textContent)).toEqual(["Liste"]);
  });

  it("follows_the_value_prop_and_does_not_switch_by_itself", () => {
    act(() => root.render(<Segmented options={options} value="liste" onChange={() => {}} ariaLabel="Ansicht" />));
    act(() => buttons()[0].click());
    expect(pressed().map((b) => b.textContent)).toEqual(["Liste"]);
    act(() => root.render(<Segmented options={options} value="baum" onChange={() => {}} ariaLabel="Ansicht" />));
    expect(pressed().map((b) => b.textContent)).toEqual(["Baum"]);
  });

  it("disables_a_single_option_and_never_picks_it", () => {
    const onChange = vi.fn();
    act(() => root.render(<Segmented options={[options[0], { ...options[1], disabled: true }]} value="baum" onChange={onChange} ariaLabel="Ansicht" />));
    expect(buttons().map((b) => b.disabled)).toEqual([false, true]);
    act(() => buttons()[1].click());
    expect(onChange).not.toHaveBeenCalled();
    expect(pressed().map((b) => b.textContent)).toEqual(["Baum"]);
  });

  it("supports_the_small_size_and_icon_only_options_with_a_name", () => {
    act(() =>
      root.render(
        <Segmented
          options={[
            { value: "a", label: "Auswählen", icon: "select" },
            { value: "b", label: "Malen", icon: "paint" },
          ]}
          value="a"
          onChange={() => {}}
          ariaLabel="Werkzeug"
          size="s"
        />,
      ),
    );
    expect(buttons()).toHaveLength(2);
  });

  it("shows_symbols_only_on_request_and_keeps_the_label_as_the_name", () => {
    const onChange = vi.fn();
    act(() => root.render(<Segmented options={options} value="liste" onChange={onChange} ariaLabel="Ansicht" iconOnly />));
    expect(buttons().map((b) => b.textContent)).toEqual(["", ""]);
    expect(buttons().map((b) => b.getAttribute("aria-label"))).toEqual(["Baum", "Liste"]);
    expect(buttons().every((b) => b.querySelector("svg") !== null)).toBe(true);
    expect(pressed().map((b) => b.getAttribute("aria-label"))).toEqual(["Liste"]);
    act(() => buttons()[0].click());
    expect(onChange.mock.calls).toEqual([["baum"]]);
  });

  it("names_an_icon_only_option_in_a_tooltip", async () => {
    vi.useFakeTimers();
    try {
      act(() => root.render(<Segmented options={options} value="liste" onChange={() => {}} ariaLabel="Ansicht" iconOnly />));
      await act(async () => {
        const b = buttons()[0];
        b.dispatchEvent(new MouseEvent("mouseover", { bubbles: true }));
        b.dispatchEvent(new MouseEvent("mouseenter"));
        b.dispatchEvent(new MouseEvent("mousemove", { bubbles: true }));
        await vi.advanceTimersByTimeAsync(500);
      });
      expect(document.querySelector("[role=tooltip]")?.textContent).toBe("Baum");
    } finally {
      vi.useRealTimers();
    }
  });
});
