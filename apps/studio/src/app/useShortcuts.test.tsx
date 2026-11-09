// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { InspectorModel } from "../workbench/Inspector";
import type { AreaId, Tool } from "../ui/frame/shellState";
import type { PaintModel, PaintPatch, PaintState } from "../workbench/paint/PaintContext";
import { useShortcuts } from "./useShortcuts";

vi.mock("../api", () => ({ undo: vi.fn(), redo: vi.fn() }));
const env = vi.hoisted(() => ({ tauri: false }));
vi.mock("../platform/transport", () => ({ inTauri: () => env.tauri }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("useShortcuts", () => {
  let container: HTMLElement;
  let root: Root;
  const handlers = { onSave: vi.fn(async () => {}), step: vi.fn(async () => {}), onPlay: vi.fn(async () => {}), onOpenProject: vi.fn(), onPalette: vi.fn(), onDeleteBuilding: vi.fn(), onDeleteEvent: vi.fn(), unpinSelection: vi.fn(), onTool: vi.fn(), onSub: vi.fn(), onSceneBack: vi.fn(), onSceneForward: vi.fn(), area: "welt" as AreaId, toolLocked: (() => false) as (tool: string) => boolean, toolsShown: (() => true) as () => boolean, onToolsHidden: vi.fn(), placing: false, tool: undefined as Tool | undefined, onEscape: vi.fn((): boolean => false), paint: (() => null) as () => PaintModel | null };
  const pinned = { current: { kind: "building", map: 3, index: 1 } as InspectorModel };

  function Probe() {
    useShortcuts(pinned, handlers);
    return null;
  }
  const press = (init: KeyboardEventInit, target: EventTarget = document.body) => act(async () => void target.dispatchEvent(new KeyboardEvent("keydown", { bubbles: true, cancelable: true, ...init })));

  beforeEach(() => {
    env.tauri = false;
    handlers.area = "welt";
    handlers.toolLocked = () => false;
    handlers.toolsShown = () => true;
    handlers.placing = false;
    handlers.tool = undefined;
    handlers.onEscape.mockImplementation(() => false);
    handlers.paint = () => null;
    pinned.current = { kind: "building", map: 3, index: 1 };
    for (const fn of Object.values(handlers)) if (vi.isMockFunction(fn)) fn.mockClear();
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

  it("escape_drops_an_armed_trainer_binding_before_it_unpins_the_selected_event", async () => {
    pinned.current = { kind: "event", eventFile: 330, eventKind: "object", index: 0 };
    handlers.placing = true;
    await press({ key: "Escape" });
    expect(handlers.onTool).toHaveBeenCalledWith("select");
    expect(handlers.unpinSelection).not.toHaveBeenCalled();
    // Typed into a field, Escape is the field's.
    handlers.onTool.mockClear();
    const input = document.createElement("input");
    container.append(input);
    await press({ key: "Escape" }, input);
    expect(handlers.onTool).not.toHaveBeenCalled();
    // Without a binding the selection goes first, as before.
    handlers.placing = false;
    await press({ key: "Escape" });
    expect(handlers.unpinSelection).toHaveBeenCalledOnce();
    expect(handlers.onTool).not.toHaveBeenCalled();
  });

  it("neither_deletes_nor_unpins_the_selection_of_the_hidden_world_on_another_area", async () => {
    // Texte, Technik, Start: the building or event stays pinned in the hidden world, but Delete and Escape there are not its keys.
    handlers.area = "texte";
    await press({ key: "Delete" });
    await press({ key: "Backspace" });
    await press({ key: "Escape" });
    expect(handlers.onDeleteBuilding).not.toHaveBeenCalled();
    expect(handlers.onDeleteEvent).not.toHaveBeenCalled();
    expect(handlers.unpinSelection).not.toHaveBeenCalled();
  });

  it("neither_deletes_nor_unpins_the_building_of_the_world_in_the_pokemon_area", async () => {
    // The data areas have their own selection: Delete there must not ask about the building pinned in the hidden world.
    for (const area of ["pokemon", "gegenstaende"] as const) {
      handlers.area = area;
      await press({ key: "Delete" });
      await press({ key: "Backspace" });
      await press({ key: "Escape" });
    }
    expect(handlers.onDeleteBuilding).not.toHaveBeenCalled();
    expect(handlers.unpinSelection).not.toHaveBeenCalled();
  });

  it("switches_the_part_of_the_pokemon_area_on_cmd_1_to_4", async () => {
    handlers.area = "pokemon";
    const event = new KeyboardEvent("keydown", { key: "4", metaKey: true, bubbles: true, cancelable: true });
    await act(async () => void document.body.dispatchEvent(event));
    expect(handlers.onSub).toHaveBeenCalledWith("moves");
    expect(event.defaultPrevented).toBe(true);
    await press({ key: "1", ctrlKey: true });
    expect(handlers.onSub).toHaveBeenLastCalledWith("pokedex");
    handlers.onSub.mockClear();
    await press({ key: "2", metaKey: true });
    expect(handlers.onSub).toHaveBeenLastCalledWith("trainer");
    handlers.onSub.mockClear();
    await press({ key: "3", metaKey: true });
    expect(handlers.onSub).toHaveBeenLastCalledWith("encounters");
  });

  it("does_not_switch_parts_outside_the_pokemon_area_or_while_a_wizard_is_open", async () => {
    handlers.area = "welt";
    await press({ key: "4", metaKey: true });
    handlers.area = "gegenstaende";
    await press({ key: "4", metaKey: true });
    handlers.area = "pokemon";
    const wizard = document.createElement("div");
    wizard.setAttribute("data-wizard", "");
    document.body.append(wizard);
    await press({ key: "4", metaKey: true });
    wizard.remove();
    expect(handlers.onSub).not.toHaveBeenCalled();
  });

  it("leaves_a_delete_or_escape_that_something_else_already_handled", async () => {
    // A document tab in "Offene Scripts" closes on Delete (and prevents the default): the pinned event must not be deleted as well.
    for (const key of ["Delete", "Backspace", "Escape"]) {
      const event = new KeyboardEvent("keydown", { key, bubbles: true, cancelable: true });
      event.preventDefault();
      await act(async () => void document.body.dispatchEvent(event));
    }
    expect(handlers.onDeleteBuilding).not.toHaveBeenCalled();
    expect(handlers.onDeleteEvent).not.toHaveBeenCalled();
    expect(handlers.unpinSelection).not.toHaveBeenCalled();
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
    handlers.area = "texte";
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
    handlers.area = "texte";
    await press({ key: "K", ctrlKey: true });
    expect(handlers.onPalette).toHaveBeenCalledTimes(2);
    const field = document.createElement("input");
    container.append(field);
    await press({ key: "k", metaKey: true }, field);
    expect(handlers.onPalette).toHaveBeenCalledTimes(3);
  });

  it("leaves_ctrl_k_and_ctrl_o_that_an_editor_already_used", async () => {
    // On a Mac, CodeMirror's Ctrl+K deletes to the line end and Ctrl+O splits the line: they use the key, the palette and the open dialog stay shut.
    for (const key of ["k", "o"]) {
      const used = new KeyboardEvent("keydown", { key, ctrlKey: true, bubbles: true, cancelable: true });
      used.preventDefault();
      await act(async () => void document.body.dispatchEvent(used));
    }
    expect(handlers.onPalette).not.toHaveBeenCalled();
    expect(handlers.onOpenProject).not.toHaveBeenCalled();
    await press({ key: "k", ctrlKey: true });
    await press({ key: "o", ctrlKey: true });
    expect(handlers.onPalette).toHaveBeenCalledOnce();
    expect(handlers.onOpenProject).toHaveBeenCalledOnce();
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

  it("opens_the_palette_again_while_the_last_dialog_still_closes", async () => {
    // Base UI keeps a closing popup mounted for its exit transition (`data-closed`, still `aria-modal`): a Cmd+K right after Escape
    // must open the palette again instead of being taken for a key behind an open dialog.
    const closing = document.createElement("div");
    closing.setAttribute("role", "dialog");
    closing.setAttribute("aria-modal", "true");
    closing.setAttribute("data-closed", "");
    closing.setAttribute("data-ending-style", "");
    document.body.append(closing);
    await press({ key: "k", metaKey: true });
    closing.remove();
    expect(handlers.onPalette).toHaveBeenCalledOnce();
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

    it("does_nothing_with_the_letter_of_a_locked_tool", async () => {
      // Without a place (or with the event file locked) the event tools are locked: their letters change nothing.
      handlers.toolLocked = (tool) => tool === "figure" || tool === "door";
      await press({ key: "f" });
      await press({ key: "t" });
      expect(handlers.onTool).not.toHaveBeenCalled();
      await press({ key: "b" });
      expect(handlers.onTool).toHaveBeenLastCalledWith("paint");
      expect(handlers.onTool).toHaveBeenCalledOnce();
    });

    it("ignores_a_tool_letter_while_the_3d_view_is_hidden", async () => {
      // Raster or Weltkarte in front: a letter changes nothing and says why.
      handlers.toolsShown = () => false;
      await press({ key: "b" });
      expect(handlers.onTool).not.toHaveBeenCalled();
      expect(handlers.onToolsHidden).toHaveBeenCalledOnce();
    });

    it("switches_the_tool_with_a_letter_while_the_3d_view_shows", async () => {
      handlers.toolsShown = () => true;
      await press({ key: "g" });
      expect(handlers.onTool).toHaveBeenLastCalledWith("building");
      expect(handlers.onToolsHidden).not.toHaveBeenCalled();
    });

    it("still_goes_back_to_select_on_escape_while_the_3d_view_is_hidden", async () => {
      mountWith(none);
      handlers.toolsShown = () => false;
      await press({ key: "Escape" });
      expect(handlers.onTool).toHaveBeenLastCalledWith("select");
      expect(handlers.onToolsHidden).not.toHaveBeenCalled();
    });

    it("keeps_a_locked_tool_silent_while_the_3d_view_is_hidden", async () => {
      handlers.toolsShown = () => false;
      handlers.toolLocked = (tool) => tool === "figure";
      await press({ key: "f" });
      expect(handlers.onTool).not.toHaveBeenCalled();
      expect(handlers.onToolsHidden).not.toHaveBeenCalled();
    });

    it("does_nothing_with_letters_or_escape_while_the_world_is_not_shown_and_works_again_when_it_is", async () => {
      mountWith(none);
      handlers.area = "texte";
      await press({ key: "b" });
      await press({ key: "f" });
      await press({ key: "g" });
      await press({ key: "Escape" });
      expect(handlers.onTool).not.toHaveBeenCalled();
      handlers.area = "welt";
      await press({ key: "b" });
      expect(handlers.onTool).toHaveBeenLastCalledWith("paint");
      await press({ key: "Escape" });
      expect(handlers.onTool).toHaveBeenLastCalledWith("select");
    });

    it("keeps_the_save_and_undo_shortcuts_while_the_world_is_not_shown", async () => {
      handlers.area = "texte";
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

    /** A paint model whose `set` merges into its state, painting `target` (null: a binary map). */
    const painting = (target: number | null, state: Partial<PaintState> = {}) => {
      const model: PaintModel = {
        state: { sub: "brush", layer: 0, tile: "0041", ramp: { dir: "e", steps: 1 }, lower: false, edgeSet: null, cliffRise: 2, ...state },
        set: vi.fn((patch: PaintPatch) => void (model.state = { ...model.state, ...(typeof patch === "function" ? patch(model.state) : patch) })),
        target,
      };
      handlers.paint = () => model;
      return model;
    };

    it("two_fast_turns_build_on_each_other", async () => {
      // The shortcut hands `set` an updater: two keys before a render both see the state the first one left.
      mountWith(none);
      handlers.tool = "paint";
      const model = painting(120);
      const stale = model.state;
      handlers.paint = () => ({ ...model, state: stale });
      await press({ key: "z" });
      await press({ key: "z" });
      expect(model.state.tile).toBe("0041r2");
    });

    it("g_fills_while_painting_a_world_map_and_builds_otherwise", async () => {
      mountWith(none);
      handlers.tool = "paint";
      const model = painting(120);
      await press({ key: "g" });
      expect(model.state.sub).toBe("fill");
      expect(handlers.onTool).not.toHaveBeenCalled();
      // Shift is no obstacle for the paint keys; Z turns the tile, X mirrors it, R and 2 set a ramp of two steps.
      await press({ key: "E", shiftKey: true });
      expect(model.state.sub).toBe("erase");
      await press({ key: "z" });
      expect(model.state.tile).toBe("0041r1");
      await press({ key: "x" });
      expect(model.state.tile).toBe("0041r1f");
      await press({ key: "r" });
      await press({ key: "2" });
      await press({ key: "z" });
      expect(model.state.ramp).toEqual({ dir: "s", steps: 2 });
      // The other tool letters still switch the tool.
      await press({ key: "f" });
      expect(handlers.onTool).toHaveBeenLastCalledWith("figure");
      // On a binary map G is „Gebäude“, as with any other tool.
      handlers.onTool.mockClear();
      const binary = painting(null);
      await press({ key: "g" });
      expect(handlers.onTool).toHaveBeenLastCalledWith("building");
      expect(binary.set).not.toHaveBeenCalled();
      handlers.tool = "select";
      const other = painting(120);
      await press({ key: "g" });
      expect(handlers.onTool).toHaveBeenLastCalledWith("building");
      expect(other.set).not.toHaveBeenCalled();
      // Typed into a field the letter is the field's.
      handlers.tool = "paint";
      const typed = painting(120);
      const input = document.createElement("input");
      container.append(input);
      await press({ key: "g" }, input);
      expect(typed.set).not.toHaveBeenCalled();
    });

    it("t_and_k_pick_the_smart_brushes_while_painting_and_t_is_the_door_otherwise", async () => {
      mountWith(none);
      handlers.tool = "paint";
      const model = painting(120);
      await press({ key: "t" });
      expect(model.state.sub).toBe("terrain");
      await press({ key: "K", shiftKey: true });
      expect(model.state.sub).toBe("cliff");
      expect(handlers.onTool).not.toHaveBeenCalled();
      // On a binary map (or with another tool) T is „Tür“.
      painting(null);
      await press({ key: "t" });
      expect(handlers.onTool).toHaveBeenLastCalledWith("door");
    });

    it("escape_returns_to_the_brush_first", async () => {
      mountWith(none);
      handlers.tool = "paint";
      const model = painting(120, { sub: "ramp" });
      await press({ key: "Escape" });
      expect(model.state.sub).toBe("brush");
      expect(handlers.onTool).not.toHaveBeenCalled();
      await press({ key: "Escape" });
      expect(handlers.onTool).toHaveBeenCalledWith("select");
    });

    it("leaves_an_escape_that_something_else_already_handled", async () => {
      mountWith(none);
      const event = new KeyboardEvent("keydown", { key: "Escape", bubbles: true, cancelable: true });
      event.preventDefault();
      await act(async () => void document.body.dispatchEvent(event));
      expect(handlers.onTool).not.toHaveBeenCalled();
    });
  });

  it("ends_a_maximization_on_escape_when_nothing_else_takes_it", async () => {
    handlers.onEscape.mockImplementation(() => true);
    handlers.tool = "paint";
    const escape = async (target: EventTarget = document.body, init: KeyboardEventInit = {}) => {
      const event = new KeyboardEvent("keydown", { key: "Escape", bubbles: true, cancelable: true, ...init });
      await act(async () => void target.dispatchEvent(event));
      return event;
    };
    // The selection takes Escape first, then the tool goes back to „Auswählen“; only then does Escape end the maximization.
    await escape();
    expect(handlers.unpinSelection).toHaveBeenCalledOnce();
    pinned.current = { kind: "header", id: 3 };
    await escape();
    expect(handlers.onTool).toHaveBeenCalledExactlyOnceWith("select");
    expect(handlers.onEscape).not.toHaveBeenCalled();
    handlers.tool = "select";
    expect((await escape()).defaultPrevented).toBe(true);
    expect(handlers.onEscape).toHaveBeenCalledOnce();
    // A trainer waiting to be placed is let go first.
    handlers.placing = true;
    await escape();
    expect(handlers.onEscape).toHaveBeenCalledOnce();
    handlers.placing = false;
    // A field, a search field, an open menu or list, a modifier, an Escape already used, a dialog: theirs, not the dock's.
    const input = document.createElement("input");
    const search = document.createElement("input");
    search.type = "search";
    const menu = document.createElement("div");
    menu.setAttribute("role", "menu");
    const item = document.createElement("button");
    menu.append(item);
    container.append(input, search, menu);
    await escape(input);
    await escape(search);
    await escape(item);
    await escape(document.body, { shiftKey: true });
    const used = new KeyboardEvent("keydown", { key: "Escape", bubbles: true, cancelable: true });
    used.preventDefault();
    await act(async () => void document.body.dispatchEvent(used));
    const dialog = document.createElement("div");
    dialog.setAttribute("role", "dialog");
    dialog.setAttribute("aria-modal", "true");
    document.body.append(dialog);
    await escape();
    dialog.remove();
    // A dock's tab menu closes on Escape by itself, with the focus left on the tab.
    const dockMenu = document.createElement("div");
    dockMenu.className = "dv-context-menu";
    dockMenu.setAttribute("role", "menu");
    document.body.append(dockMenu);
    await escape();
    dockMenu.remove();
    expect(handlers.onEscape).toHaveBeenCalledOnce();
    // In every area with a dock; nothing maximized: the Escape stays unused.
    handlers.area = "texte";
    await escape();
    expect(handlers.onEscape).toHaveBeenCalledTimes(2);
    handlers.onEscape.mockImplementation(() => false);
    expect((await escape()).defaultPrevented).toBe(false);
  });

  it("goes_back_and_forward_with_option_command_arrows_but_not_in_a_field", async () => {
    const back = new KeyboardEvent("keydown", { key: "ArrowLeft", metaKey: true, altKey: true, bubbles: true, cancelable: true });
    await act(async () => void document.body.dispatchEvent(back));
    expect(handlers.onSceneBack).toHaveBeenCalledOnce();
    expect(back.defaultPrevented).toBe(true);
    await press({ key: "ArrowRight", metaKey: true, altKey: true });
    expect(handlers.onSceneForward).toHaveBeenCalledOnce();
    // Another modifier mix is not ours: plain arrows, Cmd alone (the browser's own back), Shift added.
    await press({ key: "ArrowLeft" });
    await press({ key: "ArrowLeft", metaKey: true });
    await press({ key: "ArrowLeft", metaKey: true, altKey: true, shiftKey: true });
    // In a field the arrows move the caret (Cmd+Alt+Arrow too, in a code field).
    const input = document.createElement("input");
    container.append(input);
    await press({ key: "ArrowLeft", metaKey: true, altKey: true }, input);
    // Not behind a wizard or a dialog, and not outside the world, whose scene the history shows.
    const wizard = document.createElement("div");
    wizard.setAttribute("data-wizard", "");
    document.body.append(wizard);
    await press({ key: "ArrowRight", metaKey: true, altKey: true });
    wizard.remove();
    handlers.area = "pokemon";
    await press({ key: "ArrowLeft", metaKey: true, altKey: true });
    expect(handlers.onSceneBack).toHaveBeenCalledOnce();
    expect(handlers.onSceneForward).toHaveBeenCalledOnce();
  });

  it("leaves_select_all_to_the_text_field", async () => {
    // Cmd+A in "Text der Nachricht" (gate 3b.2, finding 18): no workbench handler takes it, the field selects its text.
    const field = document.createElement("textarea");
    document.body.append(field);
    const event = new KeyboardEvent("keydown", { key: "a", metaKey: true, bubbles: true, cancelable: true });
    await act(async () => void field.dispatchEvent(event));
    expect(event.defaultPrevented).toBe(false);
    for (const fn of Object.values(handlers)) if (vi.isMockFunction(fn)) expect(fn).not.toHaveBeenCalled();
    field.remove();
  });
});
