// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { Menu } from "./Menu";
import type { MenuEntry } from "./Menu";
import { Popover } from "./Popover";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("Menu and Popover", () => {
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

  const trigger = () => container.querySelector<HTMLElement>("button")!;
  const press = (el: HTMLElement) => {
    act(() => {
      el.dispatchEvent(new MouseEvent("mousedown", { bubbles: true, cancelable: true, button: 0 }));
      el.dispatchEvent(new MouseEvent("mouseup", { bubbles: true, cancelable: true, button: 0 }));
      el.click();
    });
  };
  const items = () => [...document.querySelectorAll<HTMLElement>("[role=menuitem]")];

  it("opens_a_menu_with_menuitems_from_the_trigger", () => {
    const entries: MenuEntry[] = [
      { label: "Rückgängig: Tür verschoben", onSelect: () => {} },
      "separator",
      { label: "Redo", onSelect: () => {} },
    ];
    act(() => root.render(<Menu trigger="Verlauf" items={entries} />));
    expect(document.querySelector("[role=menu]")).toBeNull();
    expect(trigger().getAttribute("aria-haspopup")).toBe("menu");
    press(trigger());
    expect(document.querySelector("[role=menu]")).not.toBeNull();
    expect(items().map((i) => i.textContent)).toEqual(["Rückgängig: Tür verschoben", "Redo"]);
    expect(document.querySelectorAll("[role=separator]")).toHaveLength(1);
  });

  it("selecting_an_item_calls_it_once_and_closes_the_menu", () => {
    const onSelect = vi.fn();
    act(() => root.render(<Menu trigger="Verlauf" items={[{ label: "Eins", onSelect }, { label: "Zwei", onSelect: () => {} }]} />));
    press(trigger());
    act(() => items()[0]!.click());
    expect(onSelect).toHaveBeenCalledTimes(1);
    expect(document.querySelector("[role=menu]")).toBeNull();
  });

  it("a_disabled_item_is_marked_and_not_called", () => {
    const onSelect = vi.fn();
    act(() => root.render(<Menu trigger="Verlauf" items={[{ label: "Gesperrt", onSelect, disabled: true }]} />));
    press(trigger());
    const item = items()[0]!;
    expect(item.getAttribute("aria-disabled") === "true" || item.hasAttribute("data-disabled")).toBe(true);
    act(() => item.click());
    expect(onSelect).not.toHaveBeenCalled();
  });

  it("items_with_checked_are_radio_items_and_show_their_detail", () => {
    const onSelect = vi.fn();
    const entries: MenuEntry[] = [
      { label: "Pokédex", checked: true, detail: "508", onSelect: () => {} },
      { label: "Attacken", checked: false, detail: "471", onSelect },
      "separator",
      { label: "Liste neu laden", onSelect: () => {} },
    ];
    act(() => root.render(<Menu trigger="Pokédex" items={entries} />));
    press(trigger());
    const radios = [...document.querySelectorAll<HTMLElement>("[role=menuitemradio]")];
    expect(radios.map((r) => r.textContent)).toEqual(["Pokédex508", "Attacken471"]);
    expect(radios.map((r) => r.getAttribute("aria-checked"))).toEqual(["true", "false"]);
    expect(items().map((i) => i.textContent)).toEqual(["Liste neu laden"]);
    act(() => radios[1]!.click());
    expect(onSelect).toHaveBeenCalledTimes(1);
    expect(document.querySelector("[role=menu]")).toBeNull();
  });

  it("an_icon_only_trigger_gets_its_name_from_label", () => {
    act(() => root.render(<Menu trigger={<span>3</span>} label="Verlauf" items={[{ label: "Eins", onSelect: () => {} }]} />));
    expect(trigger().getAttribute("aria-label")).toBe("Verlauf");
  });

  it("on_open_change_reports_opening_and_closing", () => {
    const onOpenChange = vi.fn();
    act(() => root.render(<Menu trigger="Verlauf" items={[{ label: "Eins", onSelect: () => {} }]} onOpenChange={onOpenChange} />));
    expect(onOpenChange).not.toHaveBeenCalled();
    press(trigger());
    expect(onOpenChange).toHaveBeenLastCalledWith(true);
    act(() => items()[0]!.click());
    expect(onOpenChange).toHaveBeenLastCalledWith(false);
    expect(onOpenChange.mock.calls.map((c) => c[0])).toEqual([true, false]);
  });

  it("popover_opens_its_content_from_the_trigger", () => {
    act(() =>
      root.render(
        <Popover trigger="Farbe">
          <p>Wähle eine Farbe.</p>
        </Popover>,
      ),
    );
    expect(document.body.textContent).not.toContain("Wähle eine Farbe.");
    press(trigger());
    expect(document.body.textContent).toContain("Wähle eine Farbe.");
    expect(document.querySelector("[role=dialog]")).not.toBeNull();
  });
});
