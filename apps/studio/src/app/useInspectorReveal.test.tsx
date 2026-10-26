// @vitest-environment jsdom
import { act } from "react";
import type { MutableRefObject, SetStateAction } from "react";
import type { DockviewApi } from "dockview-react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { InspectorModel } from "../workbench/Inspector";
import { useInspectorReveal } from "./useInspectorReveal";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("useInspectorReveal", () => {
  let container: HTMLElement;
  let root: Root;
  let pin: (next: SetStateAction<InspectorModel>) => void;
  const setPinned = vi.fn();
  const focusInspector = vi.fn();
  // Whether a group of the dock is maximized, and whether it is the inspector's.
  let maximized = false;
  let inspectorMaximized = false;
  const exitMaximizedGroup = vi.fn();
  const getPanel = vi.fn((id: string) => (id === "inspector" ? { focus: focusInspector, api: { isMaximized: () => inspectorMaximized } } : undefined));
  const api = { getPanel, hasMaximizedGroup: () => maximized, exitMaximizedGroup };
  const dock = { current: api as unknown } as MutableRefObject<DockviewApi | null>;
  // Whether a wizard works (its step under the dock keeps the keyboard).
  let paused = false;

  function Probe() {
    pin = useInspectorReveal(setPinned, dock, () => paused);
    return null;
  }

  beforeEach(() => {
    for (const fn of [setPinned, focusInspector, getPanel, exitMaximizedGroup]) fn.mockClear();
    maximized = false;
    inspectorMaximized = false;
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
  ])("brings_the_inspector_forward_for_a_pinned_$kind", (model) => {
    pin(model);
    expect(setPinned).toHaveBeenCalledExactlyOnceWith(model);
    expect(focusInspector).toHaveBeenCalledOnce();
  });

  it("brings_the_inspector_forward_in_the_one_dock", () => {
    pin({ kind: "building", map: 1, index: 0 });
    expect(getPanel).toHaveBeenCalledWith("inspector");
    expect(focusInspector).toHaveBeenCalledOnce();
    // Selecting in a maximized view is what it is maximized for: the pin leaves the maximization (and the inspector behind it) alone.
    maximized = true;
    pin({ kind: "building", map: 1, index: 1 });
    expect(setPinned).toHaveBeenCalledTimes(2);
    expect(focusInspector).toHaveBeenCalledOnce();
    expect(exitMaximizedGroup).not.toHaveBeenCalled();
    // The inspector's own group maximized: it is in front already, and is focused as before.
    inspectorMaximized = true;
    pin({ kind: "building", map: 1, index: 2 });
    expect(focusInspector).toHaveBeenCalledTimes(2);
    expect(exitMaximizedGroup).not.toHaveBeenCalled();
  });

  it("leaves_the_tabs_alone_for_no_pin_and_for_an_update_function", () => {
    pin({ kind: "none" });
    pin((p) => p);
    expect(setPinned).toHaveBeenCalledTimes(2);
    expect(focusInspector).not.toHaveBeenCalled();
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

  it("pins_but_leaves_the_inspector_alone_while_a_wizard_works", () => {
    paused = true;
    pin({ kind: "building", map: 1, index: 0 });
    paused = false;
    expect(setPinned).toHaveBeenCalledOnce();
    expect(focusInspector).not.toHaveBeenCalled();
  });

  it("still_pins_when_the_dock_is_not_built_yet", () => {
    dock.current = null;
    pin({ kind: "building", map: 1, index: 0 });
    expect(setPinned).toHaveBeenCalledOnce();
    expect(focusInspector).not.toHaveBeenCalled();
    dock.current = api as unknown as DockviewApi;
  });
});
