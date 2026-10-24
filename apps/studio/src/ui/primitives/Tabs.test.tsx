// @vitest-environment jsdom
import { act, useState } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { Tabs } from "./Tabs";
import { pressKey } from "./testUtils";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("Tabs", () => {
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

  const items = [
    { id: "a", label: "Alpha", count: 3, closeLabel: "Alpha schließen" },
    { id: "b", label: "Beta", closeLabel: "Beta schließen" },
    { id: "c", label: "Gamma", unsaved: true, closeLabel: "Gamma schließen" },
  ];
  const tabs = () => [...container.querySelectorAll<HTMLElement>("[role=tab]")];

  function Harness({ onClose, onChange }: { onClose?: (id: string) => void; onChange?: (id: string) => void }) {
    const [value, setValue] = useState("a");
    return (
      <Tabs
        items={items}
        value={value}
        onChange={(id) => {
          onChange?.(id);
          setValue(id);
        }}
        onClose={onClose}
        ariaLabel="Offene Dateien"
      >
        <p>Inhalt von {value}</p>
      </Tabs>
    );
  }

  it("is_a_labelled_tablist_with_the_active_tab_selected_and_its_content_shown", () => {
    act(() => root.render(<Harness />));
    expect(container.querySelector("[role=tablist]")?.getAttribute("aria-label")).toBe("Offene Dateien");
    expect(tabs().map((t) => t.getAttribute("aria-selected"))).toEqual(["true", "false", "false"]);
    expect(container.textContent).toContain("Inhalt von a");
  });

  it("puts_the_data_attributes_of_an_item_on_its_tab", () => {
    act(() => root.render(<Tabs items={[{ id: "7", label: "Datei 0007", dataAttrs: { "data-script": 7 } }]} value="7" onChange={() => {}} ariaLabel="Offene Dateien" />));
    expect(container.querySelector('[data-script="7"]')?.getAttribute("role")).toBe("tab");
  });

  it("switches_on_click", () => {
    const onChange = vi.fn();
    act(() => root.render(<Harness onChange={onChange} />));
    act(() => tabs()[1].click());
    expect(onChange).toHaveBeenCalledWith("b");
    expect(tabs()[1].getAttribute("aria-selected")).toBe("true");
    expect(container.textContent).toContain("Inhalt von b");
  });

  it("switches_with_the_arrow_key_because_focus_activates", async () => {
    const onChange = vi.fn();
    act(() => root.render(<Harness onChange={onChange} />));
    act(() => tabs()[0].focus());
    // Base UI moves focus in the key handler and activates the focused tab afterwards.
    await act(async () => {
      tabs()[0].dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowRight", bubbles: true, cancelable: true }));
      await new Promise((resolve) => setTimeout(resolve, 20));
    });
    expect(document.activeElement).toBe(tabs()[1]);
    expect(onChange).toHaveBeenCalledWith("b");
    expect(tabs()[1].getAttribute("aria-selected")).toBe("true");
  });

  it("shows_the_count", () => {
    act(() => root.render(<Harness />));
    expect(tabs()[0].textContent).toContain("3");
    expect(tabs()[1].textContent).toBe("Beta");
  });

  it("marks_an_unsaved_tab_with_a_dot_and_keeps_it_closable_by_mouse", () => {
    const onClose = vi.fn();
    act(() => root.render(<Harness onClose={onClose} />));
    expect(container.querySelectorAll("[aria-label=ungespeichert]")).toHaveLength(1);
    const close = container.querySelector<HTMLElement>("[aria-label='Gamma schließen']");
    expect(close?.tagName).toBe("BUTTON");
    act(() => close?.click());
    expect(onClose).toHaveBeenCalledExactlyOnceWith("c");
  });

  it("shows_only_the_dot_for_an_unsaved_tab_without_a_close_handler", () => {
    act(() => root.render(<Harness />));
    expect(container.querySelector("[aria-label=ungespeichert]")).not.toBeNull();
    expect(container.querySelector("[aria-label='Gamma schließen']")).toBeNull();
  });

  it("keeps_the_close_button_beside_the_tab_not_inside_it", () => {
    act(() => root.render(<Harness onClose={() => {}} />));
    const close = container.querySelector<HTMLElement>("[aria-label='Alpha schließen']");
    expect(close).not.toBeNull();
    expect(tabs()[0].contains(close)).toBe(false);
  });

  it("closes_with_the_close_button", () => {
    const onClose = vi.fn();
    act(() => root.render(<Harness onClose={onClose} />));
    act(() => container.querySelector<HTMLElement>("[aria-label='Beta schließen']")?.click());
    expect(onClose).toHaveBeenCalledExactlyOnceWith("b");
  });

  it("closes_with_delete_on_the_focused_tab", () => {
    const onClose = vi.fn();
    act(() => root.render(<Harness onClose={onClose} />));
    act(() => tabs()[1].focus());
    pressKey(tabs()[1], "Delete");
    expect(onClose).toHaveBeenCalledExactlyOnceWith("b");
  });

  it("closes_with_cmd_w_and_ctrl_w_on_the_focused_tab_even_when_unsaved", () => {
    const onClose = vi.fn();
    act(() => root.render(<Harness onClose={onClose} />));
    pressKey(tabs()[2], "w", { metaKey: true });
    pressKey(tabs()[0], "w", { ctrlKey: true });
    expect(onClose.mock.calls).toEqual([["c"], ["a"]]);
  });

  it("does_not_close_without_a_handler_and_shows_no_close_button", () => {
    act(() => root.render(<Harness />));
    expect(container.querySelector("[aria-label='Alpha schließen']")).toBeNull();
    pressKey(tabs()[0], "Delete");
    expect(tabs()).toHaveLength(3);
  });
});
