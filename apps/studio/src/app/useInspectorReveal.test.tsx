// @vitest-environment jsdom
import { act } from "react";
import type { SetStateAction } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { InspectorModel } from "../workbench/Inspector";
import { useInspectorReveal } from "./useInspectorReveal";
import type { DockApis } from "./useSceneSync";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("useInspectorReveal", () => {
  let container: HTMLElement;
  let root: Root;
  let pin: (next: SetStateAction<InspectorModel>) => void;
  const setPinned = vi.fn();
  const revealHand = vi.fn();
  const focusInspector = vi.fn();
  const getPanel = vi.fn((id: string) => (id === "inspector" ? { focus: focusInspector } : undefined));
  const dockApi = { schau: { current: null }, hand: { current: { getPanel } } } as unknown as DockApis;
  // Whether a wizard holds the hand.
  let paused = false;

  function Probe() {
    pin = useInspectorReveal(setPinned, dockApi, revealHand, () => paused);
    return null;
  }

  beforeEach(() => {
    for (const fn of [setPinned, revealHand, focusInspector, getPanel]) fn.mockClear();
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    act(() => root.render(<Probe />));
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  it.each<InspectorModel>([
    { kind: "header", id: 3 },
    { kind: "event", eventFile: 101, eventKind: "object", index: 0 },
    { kind: "building", map: 2, index: 1 },
    { kind: "area", id: 7 },
    { kind: "matrixCell", matrix: 0, x: 1, y: 2 },
  ])("pins_a_$kind_and_brings_the_inspector_tab_forward_in_an_opened_hand", (model) => {
    pin(model);
    expect(setPinned).toHaveBeenCalledExactlyOnceWith(model);
    expect(revealHand).toHaveBeenCalledOnce();
    expect(focusInspector).toHaveBeenCalledOnce();
  });

  it("leaves_the_tabs_alone_for_no_pin_and_for_an_update_function", () => {
    pin({ kind: "none" });
    pin((p) => p);
    expect(setPinned).toHaveBeenCalledTimes(2);
    expect(focusInspector).not.toHaveBeenCalled();
    expect(revealHand).not.toHaveBeenCalled();
  });

  it("does_not_take_the_keyboard_from_a_text_field", () => {
    const field = document.createElement("input");
    document.body.append(field);
    field.focus();
    pin({ kind: "header", id: 3 });
    expect(setPinned).toHaveBeenCalledOnce();
    expect(focusInspector).not.toHaveBeenCalled();
    field.remove();
    const editable = document.createElement("div");
    editable.contentEditable = "true";
    editable.tabIndex = 0;
    // jsdom does not implement isContentEditable.
    Object.defineProperty(editable, "isContentEditable", { value: true });
    document.body.append(editable);
    editable.focus();
    pin({ kind: "header", id: 4 });
    expect(focusInspector).not.toHaveBeenCalled();
    editable.remove();
  });

  it("pins_but_leaves_the_hand_alone_while_a_wizard_holds_it", () => {
    paused = true;
    pin({ kind: "building", map: 1, index: 0 });
    paused = false;
    expect(setPinned).toHaveBeenCalledOnce();
    expect(revealHand).not.toHaveBeenCalled();
    expect(focusInspector).not.toHaveBeenCalled();
  });

  it("still_pins_when_the_deck_has_no_hand_yet", () => {
    (dockApi.hand as { current: unknown }).current = null;
    pin({ kind: "building", map: 1, index: 0 });
    expect(setPinned).toHaveBeenCalledOnce();
    expect(revealHand).not.toHaveBeenCalled();
    (dockApi.hand as { current: unknown }).current = { getPanel };
  });
});
