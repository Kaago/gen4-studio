// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { InspectorModel } from "../workbench/Inspector";
import { useShortcuts } from "./useShortcuts";

vi.mock("../api", () => ({ undo: vi.fn(), redo: vi.fn() }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("useShortcuts", () => {
  let container: HTMLElement;
  let root: Root;
  const handlers = { onSave: vi.fn(async () => {}), step: vi.fn(async () => {}), onDeleteBuilding: vi.fn(), onDeleteEvent: vi.fn(), unpinSelection: vi.fn() };
  const pinned = { current: { kind: "building", map: 3, index: 1 } as InspectorModel };

  function Probe() {
    useShortcuts(pinned, handlers);
    return null;
  }
  const press = (init: KeyboardEventInit, target: EventTarget = document.body) => act(async () => void target.dispatchEvent(new KeyboardEvent("keydown", { bubbles: true, cancelable: true, ...init })));

  beforeEach(() => {
    for (const fn of Object.values(handlers)) fn.mockClear();
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    act(() => root.render(<Probe />));
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
    document.querySelectorAll('[role="dialog"]').forEach((d) => d.remove());
  });

  it("acts_on_the_pinned_building_and_the_project_while_nothing_covers_the_workbench", async () => {
    await press({ key: "Escape" });
    expect(handlers.unpinSelection).toHaveBeenCalledOnce();
    await press({ key: "Delete" });
    expect(handlers.onDeleteBuilding).toHaveBeenCalledWith(3, 1);
    await press({ key: "z", metaKey: true });
    expect(handlers.step).toHaveBeenCalledOnce();
    await press({ key: "s", metaKey: true });
    expect(handlers.onSave).toHaveBeenCalledOnce();
  });

  it("ignores_every_shortcut_while_a_modal_dialog_is_open_wherever_the_key_goes", async () => {
    const dialog = document.createElement("div");
    dialog.setAttribute("role", "dialog");
    dialog.setAttribute("aria-modal", "true");
    document.body.append(dialog);
    // The key lands on the body when the user clicked the dimmed backdrop, and on the dialog otherwise.
    for (const target of [document.body, dialog]) {
      await press({ key: "Escape" }, target);
      await press({ key: "Delete" }, target);
      await press({ key: "Backspace" }, target);
      await press({ key: "z", metaKey: true }, target);
      await press({ key: "s", metaKey: true }, target);
    }
    expect(handlers.unpinSelection).not.toHaveBeenCalled();
    expect(handlers.onDeleteBuilding).not.toHaveBeenCalled();
    expect(handlers.step).not.toHaveBeenCalled();
    expect(handlers.onSave).not.toHaveBeenCalled();
    dialog.remove();
    await press({ key: "Escape" });
    expect(handlers.unpinSelection).toHaveBeenCalledOnce();
  });

  it("leaves_select_all_to_the_text_field", async () => {
    // Cmd+A in "Text der Nachricht" (gate 3b.2, finding 18): no workbench handler takes it, the field selects its text.
    const field = document.createElement("textarea");
    document.body.append(field);
    const event = new KeyboardEvent("keydown", { key: "a", metaKey: true, bubbles: true, cancelable: true });
    await act(async () => void field.dispatchEvent(event));
    expect(event.defaultPrevented).toBe(false);
    for (const fn of Object.values(handlers)) expect(fn).not.toHaveBeenCalled();
    field.remove();
  });
});
