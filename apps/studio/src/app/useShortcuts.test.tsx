// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { InspectorModel } from "../workbench/Inspector";
import { useShortcuts } from "./useShortcuts";

vi.mock("../api", () => ({ undo: vi.fn(), redo: vi.fn() }));
const env = vi.hoisted(() => ({ tauri: false }));
vi.mock("../platform/transport", () => ({ inTauri: () => env.tauri }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("useShortcuts", () => {
  let container: HTMLElement;
  let root: Root;
  const handlers = { onSave: vi.fn(async () => {}), step: vi.fn(async () => {}), onPlay: vi.fn(async () => {}), onOpenProject: vi.fn(), onPalette: vi.fn(), onDeleteBuilding: vi.fn(), onDeleteEvent: vi.fn(), unpinSelection: vi.fn(), onTool: vi.fn(), toolsActive: true };
  const pinned = { current: { kind: "building", map: 3, index: 1 } as InspectorModel };

  function Probe() {
    useShortcuts(pinned, handlers);
    return null;
  }
  const press = (init: KeyboardEventInit, target: EventTarget = document.body) => act(async () => void target.dispatchEvent(new KeyboardEvent("keydown", { bubbles: true, cancelable: true, ...init })));

  beforeEach(() => {
    env.tauri = false;
    handlers.toolsActive = true;
    for (const fn of Object.values(handlers)) if (typeof fn === "function") fn.mockClear();
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

  it("ignores_delete_undo_save_play_and_the_tool_letters_while_a_wizard_is_open", async () => {
    env.tauri = true;
    // A wizard is a non-modal dialog in the hand: the scene stays usable, the workbench's keys do not.
    const wizard = document.createElement("div");
    wizard.setAttribute("role", "dialog");
    wizard.setAttribute("data-wizard", "");
    document.body.append(wizard);
    for (const target of [document.body, wizard]) {
      await press({ key: "Delete" }, target);
      await press({ key: "Backspace" }, target);
      await press({ key: "Escape" }, target);
      await press({ key: "z", metaKey: true }, target);
      await press({ key: "s", metaKey: true }, target);
      await press({ key: "r", metaKey: true }, target);
      await press({ key: "b" }, target);
    }
    expect(handlers.onDeleteBuilding).not.toHaveBeenCalled();
    expect(handlers.unpinSelection).not.toHaveBeenCalled();
    expect(handlers.step).not.toHaveBeenCalled();
    expect(handlers.onSave).not.toHaveBeenCalled();
    expect(handlers.onPlay).not.toHaveBeenCalled();
    expect(handlers.onTool).not.toHaveBeenCalled();
    wizard.remove();
    await press({ key: "Delete" });
    expect(handlers.onDeleteBuilding).toHaveBeenCalledOnce();
  });

  it("plays_on_cmd_r_inside_the_app_and_leaves_the_reload_to_the_browser", async () => {
    env.tauri = true;
    await press({ key: "r", metaKey: true });
    expect(handlers.onPlay).toHaveBeenCalledOnce();
    // A held key is one build, not one per repeat.
    await press({ key: "r", metaKey: true, repeat: true });
    expect(handlers.onPlay).toHaveBeenCalledOnce();
    // The hard reload stays the webview's.
    await press({ key: "r", metaKey: true, shiftKey: true });
    expect(handlers.onPlay).toHaveBeenCalledOnce();
    handlers.onPlay.mockClear();
    env.tauri = false;
    const reload = new KeyboardEvent("keydown", { key: "r", metaKey: true, bubbles: true, cancelable: true });
    await act(async () => void document.body.dispatchEvent(reload));
    expect(handlers.onPlay).not.toHaveBeenCalled();
    expect(reload.defaultPrevented).toBe(false);
  });

  it("opens_a_project_on_cmd_o_once_per_press_and_keeps_the_browser_from_its_own_open", async () => {
    const event = new KeyboardEvent("keydown", { key: "o", metaKey: true, bubbles: true, cancelable: true });
    await act(async () => void document.body.dispatchEvent(event));
    expect(handlers.onOpenProject).toHaveBeenCalledOnce();
    expect(event.defaultPrevented).toBe(true);
    await press({ key: "o", metaKey: true, repeat: true });
    expect(handlers.onOpenProject).toHaveBeenCalledOnce();
    // Not while the world's tools are away either: Cmd+O works on every area.
    handlers.toolsActive = false;
    await press({ key: "o", ctrlKey: true });
    expect(handlers.onOpenProject).toHaveBeenCalledTimes(2);
  });

  it("does_not_open_a_project_while_a_wizard_or_a_modal_dialog_is_open", async () => {
    for (const attrs of [{ role: "dialog", "data-wizard": "" }, { role: "dialog", "aria-modal": "true" }]) {
      const cover = document.createElement("div");
      for (const [name, value] of Object.entries(attrs)) cover.setAttribute(name, value);
      document.body.append(cover);
      await press({ key: "o", metaKey: true });
      cover.remove();
    }
    expect(handlers.onOpenProject).not.toHaveBeenCalled();
  });

  it("opens_the_palette_on_cmd_k_once_per_press_from_any_area_and_even_while_typing", async () => {
    const event = new KeyboardEvent("keydown", { key: "k", metaKey: true, bubbles: true, cancelable: true });
    await act(async () => void document.body.dispatchEvent(event));
    expect(handlers.onPalette).toHaveBeenCalledOnce();
    expect(event.defaultPrevented).toBe(true);
    await press({ key: "k", metaKey: true, repeat: true });
    expect(handlers.onPalette).toHaveBeenCalledOnce();
    handlers.toolsActive = false;
    await press({ key: "K", ctrlKey: true });
    expect(handlers.onPalette).toHaveBeenCalledTimes(2);
    const field = document.createElement("input");
    container.append(field);
    await press({ key: "k", metaKey: true }, field);
    expect(handlers.onPalette).toHaveBeenCalledTimes(3);
  });

  it("does_not_open_the_palette_while_a_wizard_or_a_modal_dialog_is_open", async () => {
    for (const attrs of [{ role: "dialog", "data-wizard": "" }, { role: "dialog", "aria-modal": "true" }]) {
      const cover = document.createElement("div");
      for (const [name, value] of Object.entries(attrs)) cover.setAttribute(name, value);
      document.body.append(cover);
      await press({ key: "k", metaKey: true });
      cover.remove();
    }
    expect(handlers.onPalette).not.toHaveBeenCalled();
  });

  it("does_not_play_while_a_modal_dialog_is_open", async () => {
    env.tauri = true;
    const dialog = document.createElement("div");
    dialog.setAttribute("role", "dialog");
    dialog.setAttribute("aria-modal", "true");
    document.body.append(dialog);
    await press({ key: "r", metaKey: true });
    expect(handlers.onPlay).not.toHaveBeenCalled();
    dialog.remove();
  });

  describe("tools", () => {
    const none = { current: { kind: "none" } as InspectorModel };
    const withSelection = pinned;
    const mountWith = (model: { current: InspectorModel }) => {
      act(() => root.unmount());
      root = createRoot(container);
      function Other() {
        useShortcuts(model, handlers);
        return null;
      }
      act(() => root.render(<Other />));
    };

    it("switches_to_the_tool_of_a_letter_in_either_case", async () => {
      await press({ key: "b" });
      expect(handlers.onTool).toHaveBeenLastCalledWith("paint");
      await press({ key: "F" });
      expect(handlers.onTool).toHaveBeenLastCalledWith("figure");
      await press({ key: "v" });
      expect(handlers.onTool).toHaveBeenLastCalledWith("select");
      expect(handlers.onTool).toHaveBeenCalledTimes(3);
    });

    it("does_nothing_with_letters_or_escape_while_the_world_is_not_shown_and_works_again_when_it_is", async () => {
      mountWith(none);
      handlers.toolsActive = false;
      await press({ key: "b" });
      await press({ key: "f" });
      await press({ key: "g" });
      await press({ key: "Escape" });
      expect(handlers.onTool).not.toHaveBeenCalled();
      handlers.toolsActive = true;
      await press({ key: "b" });
      expect(handlers.onTool).toHaveBeenLastCalledWith("paint");
      await press({ key: "Escape" });
      expect(handlers.onTool).toHaveBeenLastCalledWith("select");
    });

    it("keeps_the_save_and_undo_shortcuts_while_the_world_is_not_shown", async () => {
      handlers.toolsActive = false;
      await press({ key: "s", metaKey: true });
      await press({ key: "z", metaKey: true });
      expect(handlers.onSave).toHaveBeenCalledOnce();
      expect(handlers.step).toHaveBeenCalledOnce();
    });

    it("ignores_a_letter_typed_in_a_text_field_a_select_or_an_editable_area", async () => {
      const input = document.createElement("input");
      const area = document.createElement("textarea");
      const select = document.createElement("select");
      const editable = document.createElement("div");
      editable.setAttribute("contenteditable", "true");
      // jsdom does not derive isContentEditable from the attribute.
      Object.defineProperty(editable, "isContentEditable", { value: true });
      document.body.append(input, area, select, editable);
      for (const target of [input, area, select, editable]) {
        await press({ key: "b" }, target);
        await press({ key: "f" }, target);
        await press({ key: "Escape" }, target);
      }
      expect(handlers.onTool).not.toHaveBeenCalled();
      input.remove();
      area.remove();
      select.remove();
      editable.remove();
    });

    it("ignores_a_letter_pressed_with_a_modifier_or_held_down", async () => {
      await press({ key: "b", metaKey: true });
      await press({ key: "b", ctrlKey: true });
      await press({ key: "b", altKey: true });
      await press({ key: "B", shiftKey: true });
      await press({ key: "b", repeat: true });
      expect(handlers.onTool).not.toHaveBeenCalled();
    });

    it("ignores_a_letter_in_an_open_menu_or_list_where_it_searches", async () => {
      const menu = document.createElement("div");
      menu.setAttribute("role", "menu");
      const item = document.createElement("div");
      item.setAttribute("role", "menuitem");
      menu.append(item);
      document.body.append(menu);
      await press({ key: "b" }, item);
      await press({ key: "Escape" }, item);
      expect(handlers.onTool).not.toHaveBeenCalled();
      menu.remove();
    });

    it("ignores_letters_and_escape_while_a_modal_dialog_is_open", async () => {
      const dialog = document.createElement("div");
      dialog.setAttribute("role", "dialog");
      dialog.setAttribute("aria-modal", "true");
      document.body.append(dialog);
      await press({ key: "b" });
      await press({ key: "Escape" });
      expect(handlers.onTool).not.toHaveBeenCalled();
      dialog.remove();
    });

    it("resets_to_select_on_escape_when_nothing_is_selected", async () => {
      mountWith(none);
      await press({ key: "Escape" });
      expect(handlers.onTool).toHaveBeenCalledWith("select");
      expect(handlers.unpinSelection).not.toHaveBeenCalled();
    });

    it("lets_escape_deselect_first_and_keeps_the_tool_while_something_is_selected", async () => {
      mountWith(withSelection);
      await press({ key: "Escape" });
      expect(handlers.unpinSelection).toHaveBeenCalledOnce();
      expect(handlers.onTool).not.toHaveBeenCalled();
    });

    it("leaves_an_escape_that_something_else_already_handled", async () => {
      mountWith(none);
      const event = new KeyboardEvent("keydown", { key: "Escape", bubbles: true, cancelable: true });
      event.preventDefault();
      await act(async () => void document.body.dispatchEvent(event));
      expect(handlers.onTool).not.toHaveBeenCalled();
    });
  });

  it("leaves_select_all_to_the_text_field", async () => {
    // Cmd+A in "Text der Nachricht" (gate 3b.2, finding 18): no workbench handler takes it, the field selects its text.
    const field = document.createElement("textarea");
    document.body.append(field);
    const event = new KeyboardEvent("keydown", { key: "a", metaKey: true, bubbles: true, cancelable: true });
    await act(async () => void field.dispatchEvent(event));
    expect(event.defaultPrevented).toBe(false);
    for (const fn of Object.values(handlers)) if (typeof fn === "function") expect(fn).not.toHaveBeenCalled();
    field.remove();
  });
});
