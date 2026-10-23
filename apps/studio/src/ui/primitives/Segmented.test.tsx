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
});
